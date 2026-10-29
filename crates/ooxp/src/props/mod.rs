//! 包根 core / app / custom 文件属性。三格式共用，不依赖 docx / xlsx / pptx。

use serde_json::{json, Map, Number, Value};

use crate::error::{Error, ErrorKind, IdReason, TypeName};
use crate::opc::{self, ContentTypes, Package, Relationship, Relationships};
use crate::xml::{self, Document, Element, Node};

pub const REL_CORE: &str =
    "http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties";
pub const REL_APP: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/extended-properties";
pub const REL_CUSTOM: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/custom-properties";

pub const CT_CORE: &str = "application/vnd.openxmlformats-package.core-properties+xml";
pub const CT_APP: &str = "application/vnd.openxmlformats-officedocument.extended-properties+xml";
pub const CT_CUSTOM: &str = "application/vnd.openxmlformats-officedocument.custom-properties+xml";

const CP_NS: &str = "http://schemas.openxmlformats.org/package/2006/metadata/core-properties";
const DC_NS: &str = "http://purl.org/dc/elements/1.1/";
const DCTERMS_NS: &str = "http://purl.org/dc/terms/";
const DCMITYPE_NS: &str = "http://purl.org/dc/dcmitype/";
const XSI_NS: &str = "http://www.w3.org/2001/XMLSchema-instance";
const EP_NS: &str = "http://schemas.openxmlformats.org/officeDocument/2006/extended-properties";
const CUST_NS: &str = "http://schemas.openxmlformats.org/officeDocument/2006/custom-properties";
const VT_NS: &str = "http://schemas.openxmlformats.org/officeDocument/2006/docPropsVTypes";

const FMTID: &str = "{D5CDD505-2E9C-101B-9397-08002B2CF9AE}";

const SET_PROPS_FIELDS: &[&str] = &[
    "title",
    "subject",
    "creator",
    "description",
    "keywords",
    "category",
    "company",
    "manager",
];

#[derive(Debug, Clone)]
pub struct SetPropsOp {
    pub title: Option<Option<String>>,
    pub subject: Option<Option<String>>,
    pub creator: Option<Option<String>>,
    pub description: Option<Option<String>>,
    pub keywords: Option<Option<String>>,
    pub category: Option<Option<String>>,
    pub company: Option<Option<String>>,
    pub manager: Option<Option<String>>,
}

impl SetPropsOp {
    fn touches_core(&self) -> bool {
        self.title.is_some()
            || self.subject.is_some()
            || self.creator.is_some()
            || self.description.is_some()
            || self.keywords.is_some()
            || self.category.is_some()
    }

    fn touches_app(&self) -> bool {
        self.company.is_some() || self.manager.is_some()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CustomType {
    Lpwstr,
    I4,
    R8,
    Bool,
}

impl CustomType {
    fn parse(s: &str) -> Option<Self> {
        match s {
            "lpwstr" => Some(Self::Lpwstr),
            "i4" => Some(Self::I4),
            "r8" => Some(Self::R8),
            "bool" => Some(Self::Bool),
            _ => None,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Lpwstr => "lpwstr",
            Self::I4 => "i4",
            Self::R8 => "r8",
            Self::Bool => "bool",
        }
    }
}

#[derive(Debug, Clone)]
pub enum CustomScalar {
    Text(String),
    Int(i32),
    Float(Number),
    Bool(bool),
}

#[derive(Debug, Clone)]
pub struct SetCustomPropOp {
    pub name: String,
    pub ty: CustomType,
    pub value: CustomScalar,
}

#[derive(Debug, Clone)]
pub struct DeleteCustomPropOp {
    pub name: String,
}

pub fn inspect(pkg: &Package) -> Result<Value, Error> {
    let mut obj = Map::new();
    let core = load_part(pkg, "core-properties")?;
    obj.insert("title".into(), core_field(core.as_ref(), "title"));
    obj.insert("subject".into(), core_field(core.as_ref(), "subject"));
    obj.insert("creator".into(), core_field(core.as_ref(), "creator"));
    obj.insert(
        "description".into(),
        core_field(core.as_ref(), "description"),
    );
    obj.insert("keywords".into(), core_field(core.as_ref(), "keywords"));
    obj.insert("category".into(), core_field(core.as_ref(), "category"));
    obj.insert(
        "lastModifiedBy".into(),
        core_field(core.as_ref(), "lastModifiedBy"),
    );
    obj.insert("created".into(), core_field(core.as_ref(), "created"));
    obj.insert("modified".into(), core_field(core.as_ref(), "modified"));

    let app = load_part(pkg, "extended-properties")?;
    obj.insert("company".into(), app_field(app.as_ref(), "Company"));
    obj.insert("manager".into(), app_field(app.as_ref(), "Manager"));
    obj.insert("custom".into(), Value::Array(inspect_custom(pkg)?));
    Ok(Value::Object(obj))
}

pub fn parse_set_props(
    obj: &Map<String, Value>,
    i: usize,
    root_id: &str,
) -> Result<SetPropsOp, Error> {
    deny_op_keys(
        obj,
        i,
        &[
            "op",
            "id",
            "title",
            "subject",
            "creator",
            "description",
            "keywords",
            "category",
            "company",
            "manager",
        ],
    )?;
    require_root_id(obj, i, root_id)?;
    let op = SetPropsOp {
        title: parse_str_or_null(obj.get("title"), i, "title")?,
        subject: parse_str_or_null(obj.get("subject"), i, "subject")?,
        creator: parse_str_or_null(obj.get("creator"), i, "creator")?,
        description: parse_str_or_null(obj.get("description"), i, "description")?,
        keywords: parse_str_or_null(obj.get("keywords"), i, "keywords")?,
        category: parse_str_or_null(obj.get("category"), i, "category")?,
        company: parse_str_or_null(obj.get("company"), i, "company")?,
        manager: parse_str_or_null(obj.get("manager"), i, "manager")?,
    };
    if SET_PROPS_FIELDS.iter().all(|f| !obj.contains_key(*f)) {
        return Err(Error::with_path(
            ErrorKind::NeedPropsField,
            format!("ops[{i}]"),
        ));
    }
    Ok(op)
}

pub fn parse_set_custom_prop(
    obj: &Map<String, Value>,
    i: usize,
    root_id: &str,
) -> Result<SetCustomPropOp, Error> {
    deny_op_keys(obj, i, &["op", "id", "name", "value", "type"])?;
    require_root_id(obj, i, root_id)?;
    parse_custom_item(obj, &format!("ops[{i}]"))
}

pub fn parse_delete_custom_prop(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<DeleteCustomPropOp, Error> {
    deny_op_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let Some(name) = id.strip_prefix("prop:") else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::CustomPropPrefix,
            },
            format!("ops[{i}].id"),
        ));
    };
    if name.is_empty() {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::CustomPropPrefix,
            },
            format!("ops[{i}].id"),
        ));
    }
    Ok(DeleteCustomPropOp {
        name: name.to_string(),
    })
}

pub fn apply_set_props(pkg: &mut Package, op: &SetPropsOp, _i: usize) -> Result<(), Error> {
    if op.touches_core() {
        let (part, mut doc) = ensure_core(pkg)?;
        apply_core_fields(&mut doc.root, op);
        pkg.set(&part, doc.to_bytes()?)?;
    }
    if op.touches_app() {
        let (part, mut doc) = ensure_app(pkg)?;
        apply_app_fields(&mut doc.root, op);
        pkg.set(&part, doc.to_bytes()?)?;
    }
    Ok(())
}

pub fn apply_set_custom_prop(
    pkg: &mut Package,
    op: &SetCustomPropOp,
    _i: usize,
) -> Result<(), Error> {
    let (part, mut doc) = ensure_custom(pkg)?;
    upsert_custom(&mut doc.root, op);
    pkg.set(&part, doc.to_bytes()?)?;
    Ok(())
}

pub fn apply_delete_custom_prop(
    pkg: &mut Package,
    op: &DeleteCustomPropOp,
    i: usize,
) -> Result<(), Error> {
    let Some(part) = part_path(pkg, "custom-properties")? else {
        return Err(Error::with_path(
            ErrorKind::CustomPropNotFound {
                id: format!("prop:{}", op.name),
            },
            format!("ops[{i}].id"),
        ));
    };
    if !pkg.contains(&part) {
        return Err(Error::with_path(
            ErrorKind::CustomPropNotFound {
                id: format!("prop:{}", op.name),
            },
            format!("ops[{i}].id"),
        ));
    }
    let mut doc = xml::parse(pkg.get(&part)?)?;
    let mut found = false;
    doc.root.children.retain(|n| match n {
        Node::Element(e) if e.local_name() == "property" => {
            let name = e.attr_local("name").unwrap_or("");
            if !found && name == op.name {
                found = true;
                false
            } else {
                true
            }
        }
        _ => true,
    });
    if !found {
        return Err(Error::with_path(
            ErrorKind::CustomPropNotFound {
                id: format!("prop:{}", op.name),
            },
            format!("ops[{i}].id"),
        ));
    }
    if doc.root.children_by_local("property").is_empty() {
        remove_custom_part(pkg, &part)?;
        return Ok(());
    }
    pkg.set(&part, doc.to_bytes()?)?;
    Ok(())
}

/// create 已写好默认 core / app 之后再覆盖点名的字段。
pub fn apply_create(pkg: &mut Package, props: Option<&Value>, path: &str) -> Result<(), Error> {
    let Some(v) = props else {
        return Ok(());
    };
    let obj = v.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: path.to_string(),
                expected: TypeName::Object,
            },
            path,
        )
    })?;
    deny_prefixed(
        obj,
        path,
        &[
            "title",
            "subject",
            "creator",
            "description",
            "keywords",
            "category",
            "company",
            "manager",
            "custom",
        ],
    )?;
    let set = SetPropsOp {
        title: parse_str_or_null_at(obj.get("title"), &format!("{path}.title"))?,
        subject: parse_str_or_null_at(obj.get("subject"), &format!("{path}.subject"))?,
        creator: parse_str_or_null_at(obj.get("creator"), &format!("{path}.creator"))?,
        description: parse_str_or_null_at(obj.get("description"), &format!("{path}.description"))?,
        keywords: parse_str_or_null_at(obj.get("keywords"), &format!("{path}.keywords"))?,
        category: parse_str_or_null_at(obj.get("category"), &format!("{path}.category"))?,
        company: parse_str_or_null_at(obj.get("company"), &format!("{path}.company"))?,
        manager: parse_str_or_null_at(obj.get("manager"), &format!("{path}.manager"))?,
    };
    if set.touches_core() {
        let (part, mut doc) = ensure_core(pkg)?;
        apply_core_fields(&mut doc.root, &set);
        pkg.set(&part, doc.to_bytes()?)?;
    }
    if set.touches_app() {
        let (part, mut doc) = ensure_app(pkg)?;
        apply_app_fields(&mut doc.root, &set);
        pkg.set(&part, doc.to_bytes()?)?;
    }
    if let Some(custom_v) = obj.get("custom") {
        let arr = custom_v.as_array().ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: format!("{path}.custom"),
                    expected: TypeName::Array,
                },
                format!("{path}.custom"),
            )
        })?;
        if !arr.is_empty() {
            let (part, mut doc) = ensure_custom(pkg)?;
            for (j, item) in arr.iter().enumerate() {
                let item_obj = item.as_object().ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::WrongType {
                            field: format!("{path}.custom[{j}]"),
                            expected: TypeName::Object,
                        },
                        format!("{path}.custom[{j}]"),
                    )
                })?;
                deny_prefixed(
                    item_obj,
                    &format!("{path}.custom[{j}]"),
                    &["name", "value", "type"],
                )?;
                let op = parse_custom_item(item_obj, &format!("{path}.custom[{j}]"))?;
                upsert_custom(&mut doc.root, &op);
            }
            pkg.set(&part, doc.to_bytes()?)?;
        }
    }
    Ok(())
}

fn deny_prefixed(obj: &Map<String, Value>, prefix: &str, allowed: &[&str]) -> Result<(), Error> {
    for k in obj.keys() {
        if !allowed.contains(&k.as_str()) {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("{prefix}.{k}"),
            ));
        }
    }
    Ok(())
}

fn inspect_custom(pkg: &Package) -> Result<Vec<Value>, Error> {
    let Some(part) = part_path(pkg, "custom-properties")? else {
        return Ok(Vec::new());
    };
    if !pkg.contains(&part) {
        return Ok(Vec::new());
    }
    let doc = xml::parse(pkg.get(&part)?)?;
    let mut out = Vec::new();
    for prop in doc.root.children_by_local("property") {
        let name = prop.attr_local("name").unwrap_or("");
        if name.is_empty() {
            continue;
        }
        let (ty, value) = variant_value(prop);
        let mut item = Map::new();
        item.insert("id".into(), json!(format!("prop:{name}")));
        item.insert("name".into(), json!(name));
        item.insert("type".into(), json!(ty));
        item.insert("value".into(), value);
        out.push(Value::Object(item));
    }
    Ok(out)
}

fn variant_value(prop: &Element) -> (String, Value) {
    let Some(child) = prop.element_children().next() else {
        return ("".into(), json!(""));
    };
    let ty = child.local_name().to_string();
    let text = child.text_content();
    let value = match ty.as_str() {
        "lpwstr" | "lpstr" | "bstr" => json!(text),
        "i4" | "int" | "i2" | "ui4" => match text.trim().parse::<i64>() {
            Ok(n) => json!(n),
            Err(_) => json!(text),
        },
        "r8" | "r4" => match text.trim().parse::<f64>() {
            Ok(n) if n.is_finite() => json!(n),
            _ => json!(text),
        },
        "bool" => json!(is_true(&text)),
        _ => json!(text),
    };
    (ty, value)
}

fn is_true(s: &str) -> bool {
    matches!(s.trim().to_ascii_lowercase().as_str(), "true" | "1" | "on")
}

fn core_field(doc: Option<&Document>, local: &str) -> Value {
    match doc.and_then(|d| d.root.child_by_local(local)) {
        None => Value::Null,
        Some(el) => json!(el.text_content()),
    }
}

fn app_field(doc: Option<&Document>, local: &str) -> Value {
    match doc.and_then(|d| d.root.child_by_local(local)) {
        None => Value::Null,
        Some(el) => json!(el.text_content()),
    }
}

fn load_part(pkg: &Package, suffix: &str) -> Result<Option<Document>, Error> {
    let Some(part) = part_path(pkg, suffix)? else {
        return Ok(None);
    };
    if !pkg.contains(&part) {
        return Ok(None);
    }
    Ok(Some(xml::parse(pkg.get(&part)?)?))
}

fn part_path(pkg: &Package, suffix: &str) -> Result<Option<String>, Error> {
    let rels = Relationships::from_package(pkg, "/")?;
    let Some(rel) = rels.find_by_type_suffix(suffix).into_iter().next() else {
        return Ok(None);
    };
    if rel.is_external() {
        return Ok(None);
    }
    Ok(Some(opc::resolve_rel_target("/", &rel.target)?))
}

fn ensure_core(pkg: &mut Package) -> Result<(String, Document), Error> {
    if let Some(part) = part_path(pkg, "core-properties")? {
        if pkg.contains(&part) {
            let doc = xml::parse(pkg.get(&part)?)?;
            return Ok((part, doc));
        }
    }
    let xml = empty_core_xml();
    add_root_part(
        pkg,
        "/docProps/core.xml",
        REL_CORE,
        CT_CORE,
        xml.into_bytes(),
    )?;
    let doc = xml::parse(pkg.get("/docProps/core.xml")?)?;
    Ok(("/docProps/core.xml".into(), doc))
}

fn ensure_app(pkg: &mut Package) -> Result<(String, Document), Error> {
    if let Some(part) = part_path(pkg, "extended-properties")? {
        if pkg.contains(&part) {
            let doc = xml::parse(pkg.get(&part)?)?;
            return Ok((part, doc));
        }
    }
    let xml = empty_app_xml();
    add_root_part(pkg, "/docProps/app.xml", REL_APP, CT_APP, xml.into_bytes())?;
    let doc = xml::parse(pkg.get("/docProps/app.xml")?)?;
    Ok(("/docProps/app.xml".into(), doc))
}

fn ensure_custom(pkg: &mut Package) -> Result<(String, Document), Error> {
    if let Some(part) = part_path(pkg, "custom-properties")? {
        if pkg.contains(&part) {
            let doc = xml::parse(pkg.get(&part)?)?;
            return Ok((part, doc));
        }
    }
    let xml = empty_custom_xml();
    add_root_part(
        pkg,
        "/docProps/custom.xml",
        REL_CUSTOM,
        CT_CUSTOM,
        xml.into_bytes(),
    )?;
    let doc = xml::parse(pkg.get("/docProps/custom.xml")?)?;
    Ok(("/docProps/custom.xml".into(), doc))
}

fn add_root_part(
    pkg: &mut Package,
    path: &str,
    rel_type: &str,
    ct: &str,
    bytes: Vec<u8>,
) -> Result<(), Error> {
    pkg.set(path, bytes)?;
    let mut types = ContentTypes::from_package(pkg)?;
    types.set_override(path, ct)?;
    types.write_to(pkg)?;
    let mut rels = Relationships::from_package(pkg, "/")?;
    let already = rels
        .find_by_type_suffix(rel_type.rsplit('/').next().unwrap_or(rel_type))
        .into_iter()
        .any(|r| {
            !r.is_external()
                && opc::resolve_rel_target("/", &r.target)
                    .ok()
                    .is_some_and(|found| opc::same_part_name(&found, path))
        });
    if !already {
        let target = path.trim_start_matches('/');
        rels.push(Relationship {
            id: rels.next_rid(),
            rel_type: rel_type.to_string(),
            target: target.to_string(),
            target_mode: None,
            extra: Vec::new(),
        });
        rels.write_to(pkg, "/")?;
    }
    Ok(())
}

fn remove_custom_part(pkg: &mut Package, part: &str) -> Result<(), Error> {
    pkg.remove(part);
    let mut types = ContentTypes::from_package(pkg)?;
    types.remove_override(part);
    types.write_to(pkg)?;
    let mut rels = Relationships::from_package(pkg, "/")?;
    rels.remove_by_target_part("/", part);
    rels.write_to(pkg, "/")?;
    Ok(())
}

fn apply_core_fields(root: &mut Element, op: &SetPropsOp) {
    ensure_xmlns(root, "cp", CP_NS);
    ensure_xmlns(root, "dc", DC_NS);
    ensure_xmlns(root, "dcterms", DCTERMS_NS);
    ensure_xmlns(root, "dcmitype", DCMITYPE_NS);
    ensure_xmlns(root, "xsi", XSI_NS);
    set_named(root, DC_NS, "dc", "title", op.title.as_ref());
    set_named(root, DC_NS, "dc", "subject", op.subject.as_ref());
    set_named(root, DC_NS, "dc", "creator", op.creator.as_ref());
    set_named(root, DC_NS, "dc", "description", op.description.as_ref());
    set_named(root, CP_NS, "cp", "keywords", op.keywords.as_ref());
    set_named(root, CP_NS, "cp", "category", op.category.as_ref());
}

fn apply_app_fields(root: &mut Element, op: &SetPropsOp) {
    set_local(root, EP_NS, "Company", op.company.as_ref());
    set_local(root, EP_NS, "Manager", op.manager.as_ref());
}

fn set_named(
    root: &mut Element,
    ns: &str,
    default_prefix: &str,
    local: &str,
    value: Option<&Option<String>>,
) {
    let Some(v) = value else {
        return;
    };
    let qname = qname_for(root, ns, default_prefix, local);
    set_child(root, &qname, local, v.as_deref());
}

fn set_local(root: &mut Element, ns: &str, local: &str, value: Option<&Option<String>>) {
    let Some(v) = value else {
        return;
    };
    let qname = match xmlns_prefix(root, ns) {
        Some(p) if p.is_empty() => local.to_string(),
        Some(p) => format!("{p}:{local}"),
        None => local.to_string(),
    };
    set_child(root, &qname, local, v.as_deref());
}

fn set_child(root: &mut Element, qname: &str, local: &str, value: Option<&str>) {
    match value {
        None => {
            root.children.retain(|n| match n {
                Node::Element(e) => e.local_name() != local,
                _ => true,
            });
        }
        Some(text) => {
            if let Some(el) = root.child_by_local_mut(local) {
                if text.is_empty() {
                    el.children.clear();
                } else {
                    el.set_text(text.to_string());
                }
            } else {
                let mut el = Element::new(qname);
                if !text.is_empty() {
                    el.set_text(text.to_string());
                }
                root.push_element(el);
            }
        }
    }
}

fn upsert_custom(root: &mut Element, op: &SetCustomPropOp) {
    ensure_xmlns(root, "vt", VT_NS);
    if let Some(existing) = root
        .element_children_mut()
        .find(|e| e.local_name() == "property" && e.attr_local("name") == Some(op.name.as_str()))
    {
        write_variant(existing, op);
        return;
    }
    let pid = next_pid(root);
    let mut prop = Element::new(root.prefixed("property"))
        .with_attr("fmtid", FMTID)
        .with_attr("pid", pid.to_string())
        .with_attr("name", op.name.clone());
    write_variant(&mut prop, op);
    root.push_element(prop);
}

fn next_pid(root: &Element) -> u32 {
    let mut max = 1u32;
    for p in root.children_by_local("property") {
        if let Some(n) = p.attr_local("pid").and_then(|s| s.parse::<u32>().ok()) {
            max = max.max(n);
        }
    }
    max + 1
}

fn write_variant(prop: &mut Element, op: &SetCustomPropOp) {
    prop.children.retain(|n| !matches!(n, Node::Element(_)));
    let vt_name = match xmlns_prefix(prop, VT_NS) {
        Some(p) if p.is_empty() => op.ty.as_str().to_string(),
        Some(p) => format!("{}:{}", p, op.ty.as_str()),
        None => format!("vt:{}", op.ty.as_str()),
    };
    let text = match &op.value {
        CustomScalar::Text(s) => s.clone(),
        CustomScalar::Int(n) => n.to_string(),
        CustomScalar::Float(n) => n.to_string(),
        CustomScalar::Bool(true) => "true".into(),
        CustomScalar::Bool(false) => "false".into(),
    };
    let mut child = Element::new(vt_name);
    child.set_text(text);
    prop.push_element(child);
}

fn empty_core_xml() -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><cp:coreProperties xmlns:cp="{CP_NS}" xmlns:dc="{DC_NS}" xmlns:dcterms="{DCTERMS_NS}" xmlns:dcmitype="{DCMITYPE_NS}" xmlns:xsi="{XSI_NS}"/>"#
    )
}

fn empty_app_xml() -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Properties xmlns="{EP_NS}" xmlns:vt="{VT_NS}"><Application>ooxp</Application></Properties>"#
    )
}

fn empty_custom_xml() -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Properties xmlns="{CUST_NS}" xmlns:vt="{VT_NS}"/>"#
    )
}

fn qname_for(root: &Element, ns: &str, default_prefix: &str, local: &str) -> String {
    match xmlns_prefix(root, ns) {
        Some(p) if p.is_empty() => local.to_string(),
        Some(p) => format!("{p}:{local}"),
        None => format!("{default_prefix}:{local}"),
    }
}

fn xmlns_prefix(el: &Element, ns: &str) -> Option<String> {
    for (k, v) in &el.attrs {
        if v != ns {
            continue;
        }
        if k == "xmlns" {
            return Some(String::new());
        }
        if let Some(pfx) = k.strip_prefix("xmlns:") {
            return Some(pfx.to_string());
        }
    }
    None
}

fn ensure_xmlns(el: &mut Element, prefix: &str, ns: &str) {
    if xmlns_prefix(el, ns).is_some() {
        return;
    }
    el.set_attr(&format!("xmlns:{prefix}"), ns);
}

fn deny_op_keys(obj: &Map<String, Value>, i: usize, allowed: &[&str]) -> Result<(), Error> {
    for k in obj.keys() {
        if !allowed.contains(&k.as_str()) {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("ops[{i}].{k}"),
            ));
        }
    }
    Ok(())
}

fn require_id(obj: &Map<String, Value>, i: usize) -> Result<String, Error> {
    obj.get("id")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .ok_or_else(|| {
            Error::with_path(
                ErrorKind::MissingField { field: "id".into() },
                format!("ops[{i}].id"),
            )
        })
}

fn require_root_id(obj: &Map<String, Value>, i: usize, root_id: &str) -> Result<(), Error> {
    let id = require_id(obj, i)?;
    if id == root_id {
        return Ok(());
    }
    Err(Error::with_path(
        ErrorKind::InvalidId {
            reason: IdReason::SetPropsNeedsRoot,
        },
        format!("ops[{i}].id"),
    ))
}

fn parse_str_or_null(
    v: Option<&Value>,
    i: usize,
    field: &str,
) -> Result<Option<Option<String>>, Error> {
    parse_str_or_null_at(v, &format!("ops[{i}].{field}"))
}

fn parse_str_or_null_at(v: Option<&Value>, path: &str) -> Result<Option<Option<String>>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(None)),
        Some(Value::String(s)) => Ok(Some(Some(s.clone()))),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: path.rsplit('.').next().unwrap_or("field").into(),
                expected: TypeName::String,
            },
            path,
        )),
    }
}

fn parse_custom_item(obj: &Map<String, Value>, path: &str) -> Result<SetCustomPropOp, Error> {
    let name = match obj.get("name") {
        Some(Value::String(s)) if !s.is_empty() => s.clone(),
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::EmptyCustomPropName,
                format!("{path}.name"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "name".into(),
                    expected: TypeName::NonEmptyString,
                },
                format!("{path}.name"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "name".into(),
                },
                format!("{path}.name"),
            ));
        }
    };
    let value_v = obj.get("value").ok_or_else(|| {
        Error::with_path(
            ErrorKind::MissingField {
                field: "value".into(),
            },
            format!("{path}.value"),
        )
    })?;
    let ty = match obj.get("type") {
        None => infer_type(value_v, path)?,
        Some(Value::String(s)) => CustomType::parse(s).ok_or_else(|| {
            Error::with_path(ErrorKind::InvalidCustomPropType, format!("{path}.type"))
        })?,
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "type".into(),
                    expected: TypeName::String,
                },
                format!("{path}.type"),
            ));
        }
    };
    let value = coerce_value(ty, value_v, path)?;
    Ok(SetCustomPropOp { name, ty, value })
}

fn infer_type(v: &Value, path: &str) -> Result<CustomType, Error> {
    match v {
        Value::String(_) => Ok(CustomType::Lpwstr),
        Value::Bool(_) => Ok(CustomType::Bool),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                if i >= i32::MIN as i64 && i <= i32::MAX as i64 {
                    return Ok(CustomType::I4);
                }
            }
            Ok(CustomType::R8)
        }
        _ => Err(Error::with_path(
            ErrorKind::InvalidCustomPropType,
            format!("{path}.value"),
        )),
    }
}

fn coerce_value(ty: CustomType, v: &Value, path: &str) -> Result<CustomScalar, Error> {
    let bad = || Error::with_path(ErrorKind::InvalidCustomPropType, format!("{path}.value"));
    match ty {
        CustomType::Lpwstr => match v {
            Value::String(s) => Ok(CustomScalar::Text(s.clone())),
            Value::Number(n) => Ok(CustomScalar::Text(n.to_string())),
            Value::Bool(b) => Ok(CustomScalar::Text(b.to_string())),
            _ => Err(bad()),
        },
        CustomType::I4 => match v {
            Value::Number(n) => {
                let i = n.as_i64().ok_or_else(bad)?;
                if i < i32::MIN as i64 || i > i32::MAX as i64 {
                    return Err(bad());
                }
                Ok(CustomScalar::Int(i as i32))
            }
            _ => Err(bad()),
        },
        CustomType::R8 => match v {
            Value::Number(n) => Ok(CustomScalar::Float(n.clone())),
            _ => Err(bad()),
        },
        CustomType::Bool => match v {
            Value::Bool(b) => Ok(CustomScalar::Bool(*b)),
            _ => Err(bad()),
        },
    }
}
