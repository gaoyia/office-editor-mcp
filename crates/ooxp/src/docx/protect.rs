//! 文档保护：`settings.xml` 的 `documentProtection` 与 `writeProtection`。
//! 可写 Word 16.0 的限制编辑 SHA-512 哈希，不破解密码，不写 `writeProtection` 修改密码。

use std::collections::HashMap;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde_json::{json, Value};

use crate::error::{Error, ErrorKind};
use crate::opc::Package;
use crate::password::word_sha512_spin;
use crate::xml::{self, Document, Element, Node};

use super::hf;
use super::notes;
use super::sect;

const EDIT_VALUES: &[&str] = &["readOnly", "comments", "trackedChanges", "forms"];

const HASH_ATTRS: &[&str] = &[
    "hash",
    "hashValue",
    "salt",
    "saltValue",
    "reservationPassword",
];

const SPIN_COUNT: u32 = 100_000;

#[derive(Debug, Clone)]
pub struct SetProtectionOp {
    #[allow(dead_code)]
    pub id: String,
    pub enforced: Option<bool>,
    pub edit: Option<String>,
    pub formatting: Option<bool>,
    pub read_only_recommended: Option<bool>,
    pub password: Option<String>,
}

#[derive(Debug, Clone)]
struct Snapshot {
    enforced: bool,
    edit: Option<String>,
    formatting: bool,
    doc_has_password: bool,
    read_only_recommended: bool,
    write_has_password: bool,
}

pub fn inspect(pkg: &Package, doc_part: &str) -> Result<Value, Error> {
    let snap = read_snapshot(pkg, doc_part)?;
    Ok(to_json(&snap))
}

fn to_json(snap: &Snapshot) -> Value {
    json!({
        "enforced": snap.enforced,
        "edit": snap.edit,
        "formatting": snap.formatting,
        "hasPassword": snap.doc_has_password || snap.write_has_password,
        "readOnlyRecommended": snap.read_only_recommended,
    })
}

fn empty_snapshot() -> Snapshot {
    Snapshot {
        enforced: false,
        edit: None,
        formatting: false,
        doc_has_password: false,
        read_only_recommended: false,
        write_has_password: false,
    }
}

fn read_snapshot(pkg: &Package, doc_part: &str) -> Result<Snapshot, Error> {
    let Some(settings) = notes::settings_part_name(pkg, doc_part)? else {
        return Ok(empty_snapshot());
    };
    let Ok(bytes) = pkg.get(&settings) else {
        return Ok(empty_snapshot());
    };
    let doc = xml::parse(bytes)?;
    snapshot_from_root(&doc.root)
}

fn snapshot_from_root(root: &Element) -> Result<Snapshot, Error> {
    let mut snap = empty_snapshot();
    if let Some(el) = root.child_by_local("documentProtection") {
        snap.enforced =
            sect::on_off_attr(el, "enforcement", "documentProtection")?.unwrap_or(false);
        snap.formatting =
            sect::on_off_attr(el, "formatting", "documentProtection")?.unwrap_or(false);
        snap.edit = parse_edit_attr(el)?;
        snap.doc_has_password = elem_has_password(el);
    }
    if let Some(el) = root.child_by_local("writeProtection") {
        snap.write_has_password = elem_has_password(el);
        let recommended = sect::on_off_attr(el, "recommended", "writeProtection")?.unwrap_or(false);
        snap.read_only_recommended = recommended && !snap.write_has_password;
    }
    Ok(snap)
}

fn parse_edit_attr(el: &Element) -> Result<Option<String>, Error> {
    let Some(raw) = el.attr_local("edit") else {
        return Ok(None);
    };
    let t = raw.trim();
    if t.is_empty() || t == "none" {
        return Ok(None);
    }
    if EDIT_VALUES.contains(&t) {
        return Ok(Some(t.to_string()));
    }
    Err(Error::xml_detail(
        crate::error::XmlIssue::Parse,
        format!("documentProtection/@edit is not a known protection type: {raw}"),
    ))
}

fn elem_has_password(el: &Element) -> bool {
    el.attrs.iter().any(|(k, v)| {
        let local = k.rsplit(':').next().unwrap_or(k);
        HASH_ATTRS.contains(&local) && !v.trim().is_empty()
    })
}

pub fn apply(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    op: &SetProtectionOp,
    i: usize,
) -> Result<(), Error> {
    let settings_name = notes::settings_part_name(pkg, doc_part)?.ok_or_else(|| {
        Error::with_path(
            ErrorKind::PartMissing {
                name: "/word/settings.xml".into(),
            },
            format!("ops[{i}].id"),
        )
    })?;
    let settings_doc = load_cached(pkg, docs, &settings_name)?;
    let snap = snapshot_from_root(&settings_doc.root)?;
    let touch_doc = op.enforced.is_some() || op.edit.is_some() || op.formatting.is_some();
    let touch_ror = op.read_only_recommended.is_some();
    if op.password.is_some() && !touch_doc && touch_ror {
        return Err(Error::with_path(
            ErrorKind::ProtectionPasswordNeedsDoc,
            format!("ops[{i}].password"),
        ));
    }
    if touch_doc && snap.doc_has_password {
        let Some(el) = settings_doc.root.child_by_local("documentProtection") else {
            return Err(Error::xml_detail(
                crate::error::XmlIssue::Parse,
                "documentProtection hash attributes present but the element is missing",
            ));
        };
        gate_existing_hash(el, op.password.as_deref(), i)?;
    }
    if touch_ror && snap.write_has_password {
        return Err(Error::with_path(
            ErrorKind::ProtectionHasPassword,
            format!("ops[{i}].readOnlyRecommended"),
        ));
    }

    let mut next_enforced = op.enforced.unwrap_or(snap.enforced);
    if op.edit.is_some() && op.enforced.is_none() && !snap.enforced {
        next_enforced = true;
    }
    let next_edit = op.edit.clone().or(snap.edit.clone());
    let next_formatting = op.formatting.unwrap_or(snap.formatting);
    let next_ror = op
        .read_only_recommended
        .unwrap_or(snap.read_only_recommended);

    if touch_doc {
        let settings_doc = load_cached(pkg, docs, &settings_name)?;
        if next_enforced {
            let edit = next_edit.unwrap_or_else(|| "readOnly".into());
            write_document_protection(
                &mut settings_doc.root,
                &edit,
                next_formatting,
                op.password.as_deref(),
            );
            if edit == "trackedChanges" {
                hf::set_on_off(
                    &mut settings_doc.root,
                    "trackRevisions",
                    true,
                    hf::SETTINGS_ORDER,
                );
            }
            dirty.insert(settings_name.clone(), ());
            set_all_form_prot(pkg, docs, dirty, doc_part, edit == "forms")?;
        } else {
            hf::remove_local(&mut settings_doc.root, "documentProtection");
            dirty.insert(settings_name.clone(), ());
            set_all_form_prot(pkg, docs, dirty, doc_part, false)?;
        }
    }

    if touch_ror {
        let settings_doc = load_cached(pkg, docs, &settings_name)?;
        write_read_only_recommended(&mut settings_doc.root, next_ror);
        dirty.insert(settings_name, ());
    }
    Ok(())
}

fn write_document_protection(
    root: &mut Element,
    edit: &str,
    formatting: bool,
    password: Option<&str>,
) {
    hf::remove_local(root, "documentProtection");
    let mut el = Element::new(root.prefixed("documentProtection"));
    let edit_attr = root.prefixed("edit");
    let enf_attr = root.prefixed("enforcement");
    el.set_attr(&edit_attr, edit);
    el.set_attr(&enf_attr, "1");
    if formatting {
        let fmt_attr = root.prefixed("formatting");
        el.set_attr(&fmt_attr, "1");
    }
    if let Some(pw) = password {
        write_word_hash(root, &mut el, pw);
    }
    insert_child(root, el, "documentProtection");
}

fn write_word_hash(root: &Element, el: &mut Element, password: &str) {
    let mut salt = [0u8; 16];
    getrandom::getrandom(&mut salt).expect("系统随机数不可用");
    let hash = word_sha512_spin(password, &salt, SPIN_COUNT);
    el.set_attr(&root.prefixed("cryptProviderType"), "rsaAES");
    el.set_attr(&root.prefixed("cryptAlgorithmClass"), "hash");
    el.set_attr(&root.prefixed("cryptAlgorithmType"), "typeAny");
    el.set_attr(&root.prefixed("cryptAlgorithmSid"), "14");
    el.set_attr(&root.prefixed("cryptSpinCount"), SPIN_COUNT.to_string());
    el.set_attr(&root.prefixed("hash"), STANDARD.encode(&hash));
    el.set_attr(&root.prefixed("salt"), STANDARD.encode(salt));
}

struct WordHashSpec {
    salt: Vec<u8>,
    hash: Vec<u8>,
    spin: u32,
}

fn gate_existing_hash(el: &Element, password: Option<&str>, i: usize) -> Result<(), Error> {
    let Some(pw) = password else {
        return Err(Error::with_path(
            ErrorKind::ProtectionHasPassword,
            format!("ops[{i}]"),
        ));
    };
    let Some(spec) = word_hash_spec(el) else {
        return Err(Error::with_path(
            ErrorKind::ProtectionHasPassword,
            format!("ops[{i}]"),
        ));
    };
    if word_sha512_spin(pw, &spec.salt, spec.spin) != spec.hash {
        return Err(Error::with_path(
            ErrorKind::ProtectionPasswordWrong,
            format!("ops[{i}].password"),
        ));
    }
    Ok(())
}

fn word_hash_spec(el: &Element) -> Option<WordHashSpec> {
    let sid = el.attr_local("cryptAlgorithmSid")?;
    if sid != "14" {
        return None;
    }
    let hash = STANDARD.decode(el.attr_local("hash")?).ok()?;
    let salt = STANDARD.decode(el.attr_local("salt")?).ok()?;
    let spin: u32 = el.attr_local("cryptSpinCount")?.parse().ok()?;
    if hash.is_empty() || salt.is_empty() || spin == 0 {
        return None;
    }
    Some(WordHashSpec { salt, hash, spin })
}

fn write_read_only_recommended(root: &mut Element, on: bool) {
    hf::remove_local(root, "writeProtection");
    if !on {
        return;
    }
    let mut el = Element::new(root.prefixed("writeProtection"));
    let rec_attr = root.prefixed("recommended");
    el.set_attr(&rec_attr, "true");
    insert_child(root, el, "writeProtection");
}

fn insert_child(parent: &mut Element, el: Element, local: &str) {
    let dest = insert_index(parent, local, hf::SETTINGS_ORDER);
    parent.children.insert(dest, Node::Element(el));
}

fn insert_index(parent: &Element, local: &str, order: &[&str]) -> usize {
    let my = order
        .iter()
        .position(|&x| x == local)
        .unwrap_or(order.len());
    let mut after_lower = 0usize;
    for (i, n) in parent.children.iter().enumerate() {
        let Node::Element(e) = n else {
            continue;
        };
        let Some(rank) = order.iter().position(|&x| x == e.local_name()) else {
            continue;
        };
        if rank < my {
            after_lower = i + 1;
        } else if rank > my {
            return i;
        }
    }
    after_lower
}

fn set_all_form_prot(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    on: bool,
) -> Result<(), Error> {
    let n = {
        let doc = load_cached(pkg, docs, doc_part)?;
        hf::collect_sect_prs(&doc.root).len()
    };
    if n == 0 {
        return Ok(());
    }
    let mut changed = false;
    for idx in 1..=n {
        let doc = load_cached(pkg, docs, doc_part)?;
        let Some(sect) = hf::find_sect_pr_mut(&mut doc.root, idx) else {
            continue;
        };
        let had = hf::on_off_opt(sect, "formProt").unwrap_or(false);
        if had == on {
            continue;
        }
        hf::set_on_off(sect, "formProt", on, hf::TITLE_PG_ORDER);
        changed = true;
    }
    if changed {
        dirty.insert(doc_part.to_string(), ());
    }
    Ok(())
}

fn load_cached<'a>(
    pkg: &Package,
    docs: &'a mut HashMap<String, Document>,
    name: &str,
) -> Result<&'a mut Document, Error> {
    if !docs.contains_key(name) {
        let bytes = pkg.get(name)?;
        docs.insert(name.to_string(), xml::parse(bytes)?);
    }
    Ok(docs.get_mut(name).unwrap())
}

pub fn apply_create(pkg: &mut Package, v: Option<&Value>) -> Result<(), Error> {
    let Some(v) = v else {
        return Ok(());
    };
    let obj = v.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "document.protection".into(),
                expected: crate::error::TypeName::Object,
            },
            "document.protection",
        )
    })?;
    if obj.contains_key("password") {
        return Err(Error::with_path(
            ErrorKind::ProtectionPassword,
            "document.protection.password",
        ));
    }
    for k in obj.keys() {
        if !matches!(
            k.as_str(),
            "enforced" | "edit" | "formatting" | "readOnlyRecommended"
        ) {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("document.protection.{k}"),
            ));
        }
    }
    let op = parse_fields(obj, "document.protection")?;
    let doc_part = crate::opc::office_document_part(pkg)?;
    let mut docs = HashMap::new();
    let mut dirty = HashMap::new();
    apply(pkg, &mut docs, &mut dirty, &doc_part, &op, 0)?;
    for name in dirty.keys() {
        let doc = docs.get(name).unwrap();
        pkg.set(name, doc.to_bytes()?)?;
    }
    Ok(())
}

pub fn parse_op(obj: &serde_json::Map<String, Value>, i: usize) -> Result<SetProtectionOp, Error> {
    for k in obj.keys() {
        if !matches!(
            k.as_str(),
            "op" | "id" | "enforced" | "edit" | "formatting" | "readOnlyRecommended" | "password"
        ) {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("ops[{i}].{k}"),
            ));
        }
    }
    let id = match obj.get("id") {
        Some(Value::String(s)) if !s.is_empty() => s.clone(),
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "id".into(),
                    expected: crate::error::TypeName::NonEmptyString,
                },
                format!("ops[{i}].id"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "id".into(),
                    expected: crate::error::TypeName::String,
                },
                format!("ops[{i}].id"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::MissingField { field: "id".into() },
                format!("ops[{i}].id"),
            ));
        }
    };
    if id != "docx:document" {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: crate::error::IdReason::SettingsNeedDocument,
            },
            format!("ops[{i}].id"),
        ));
    }
    parse_fields(obj, &format!("ops[{i}]")).map(|mut op| {
        op.id = id;
        op
    })
}

fn parse_fields(
    obj: &serde_json::Map<String, Value>,
    path: &str,
) -> Result<SetProtectionOp, Error> {
    let enforced = parse_opt_bool(obj.get("enforced"), path, "enforced")?;
    let formatting = parse_opt_bool(obj.get("formatting"), path, "formatting")?;
    let read_only_recommended =
        parse_opt_bool(obj.get("readOnlyRecommended"), path, "readOnlyRecommended")?;
    let password = parse_opt_password(obj.get("password"), path)?;
    let edit = match obj.get("edit") {
        None => None,
        Some(Value::String(s)) => {
            if EDIT_VALUES.contains(&s.as_str()) {
                Some(s.clone())
            } else {
                return Err(Error::with_path(
                    ErrorKind::InvalidProtectionEdit,
                    format!("{path}.edit"),
                ));
            }
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "edit".into(),
                    expected: crate::error::TypeName::String,
                },
                format!("{path}.edit"),
            ));
        }
    };
    if enforced.is_none()
        && edit.is_none()
        && formatting.is_none()
        && read_only_recommended.is_none()
    {
        return Err(Error::with_path(ErrorKind::NeedProtectionField, path));
    }
    Ok(SetProtectionOp {
        id: String::new(),
        enforced,
        edit,
        formatting,
        read_only_recommended,
        password,
    })
}

fn parse_opt_password(v: Option<&Value>, path: &str) -> Result<Option<String>, Error> {
    match v {
        None => Ok(None),
        Some(Value::String(s)) if !s.is_empty() => Ok(Some(s.clone())),
        Some(Value::String(_)) => Err(Error::with_path(
            ErrorKind::ProtectionPasswordBlank,
            format!("{path}.password"),
        )),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "password".into(),
                expected: crate::error::TypeName::String,
            },
            format!("{path}.password"),
        )),
    }
}

fn parse_opt_bool(v: Option<&Value>, path: &str, field: &str) -> Result<Option<bool>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Bool(b)) => Ok(Some(*b)),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: crate::error::TypeName::Boolean,
            },
            format!("{path}.{field}"),
        )),
    }
}
