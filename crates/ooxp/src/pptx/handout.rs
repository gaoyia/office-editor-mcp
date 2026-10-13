//! 讲义母版 `handoutMaster`：inspect 槽位文字、`setHandoutText`、按需新建最小部件。

use serde_json::{json, Map, Value};

use crate::error::{Error, ErrorKind};
use crate::opc::{self, ContentTypes, Package, Relationship, Relationships};
use crate::xml::{self, Document, Element, Node};

use super::notes;
use super::picture;
use super::request::SetHandoutTextOp;
use super::shape;
use super::{CT_THEME, DML_NS, PML_NS};

pub const REL_HANDOUT_MASTER: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/handoutMaster";
pub const CT_HANDOUT_MASTER: &str =
    "application/vnd.openxmlformats-officedocument.presentationml.handoutMaster+xml";

/// 对照备注母版的页眉页脚框，去掉 `sldImg` / `body` 与 `p14:creationId`。
const HANDOUT_MASTER_XML: &str = include_str!("handout_master.xml");

const SLOTS: [(&str, &str); 4] = [
    ("header", "hdr"),
    ("footer", "ftr"),
    ("date", "dt"),
    ("sldNum", "sldNum"),
];

pub fn has_handout(pkg: &Package, pres_part: &str) -> Result<bool, Error> {
    Ok(handout_part(pkg, pres_part)?.is_some())
}

pub fn handout_json(pkg: &Package, pres_part: &str) -> Result<Value, Error> {
    let Some(part) = handout_part(pkg, pres_part)? else {
        return Ok(Value::Null);
    };
    let doc = xml::parse(pkg.get(&part)?)?;
    let mut obj = Map::new();
    obj.insert("id".into(), json!("handout"));
    obj.insert("clrMap".into(), super::theme_map::clr_map_json(&doc.root));
    for (field, ph) in SLOTS {
        obj.insert(field.into(), slot_json(&doc.root, ph));
    }
    Ok(Value::Object(obj))
}

pub fn set_text(
    pkg: &mut Package,
    pres_part: &str,
    pres: &mut Document,
    op: &SetHandoutTextOp,
    i: usize,
) -> Result<(), Error> {
    let part = ensure_handout_master(pkg, pres_part, pres)?;
    let mut doc = xml::parse(pkg.get(&part)?)?;
    for (field, ph, text) in [
        ("header", "hdr", op.header.as_deref()),
        ("footer", "ftr", op.footer.as_deref()),
        ("date", "dt", op.date.as_deref()),
        ("sldNum", "sldNum", op.sld_num.as_deref()),
    ] {
        let Some(text) = text else {
            continue;
        };
        if find_sp_ph_mut(&mut doc.root, ph).is_none() {
            append_placeholder(&mut doc.root, ph)?;
        }
        let sp = find_sp_ph_mut(&mut doc.root, ph).ok_or_else(|| {
            Error::with_path(
                ErrorKind::ShapeNotFound {
                    id: format!("handout/{field}"),
                },
                format!("ops[{i}].{field}"),
            )
        })?;
        let tx = sp.child_by_local_mut("txBody").ok_or_else(|| {
            Error::with_path(
                ErrorKind::ShapeHasNoText {
                    id: format!("handout/{field}"),
                },
                format!("ops[{i}].{field}"),
            )
        })?;
        // 契约：日期 / 页码原先若是 a:fld，改完变成普通文字。
        strip_fields(tx);
        shape::set_txbody_text(tx, text);
        enable_hf(&mut doc.root, ph);
    }
    pkg.set(&part, doc.to_bytes()?)?;
    Ok(())
}

pub(crate) fn handout_part(pkg: &Package, pres_part: &str) -> Result<Option<String>, Error> {
    let rels = Relationships::from_package(pkg, pres_part)?;
    let Some(rel) = rels
        .rels
        .iter()
        .find(|r| r.rel_type.ends_with("/handoutMaster"))
    else {
        return Ok(None);
    };
    Ok(Some(opc::resolve_id(pkg, pres_part, &rel.id)?))
}

pub(crate) fn ensure_handout_master(
    pkg: &mut Package,
    pres_part: &str,
    pres: &mut Document,
) -> Result<String, Error> {
    if let Some(part) = handout_part(pkg, pres_part)? {
        return Ok(part);
    }
    let mut pres_rels = Relationships::from_package(pkg, pres_part)?;
    let master = next_part(pkg, "/ppt/handoutMasters", "handoutMaster");
    pkg.set(&master, HANDOUT_MASTER_XML.as_bytes().to_vec())?;
    let cloned_theme = notes::attach_cloned_theme(pkg, pres_part, &pres_rels, &master)?;
    let rid = pres_rels.next_rid();
    pres_rels.push(Relationship {
        id: rid.clone(),
        rel_type: REL_HANDOUT_MASTER.into(),
        target: picture::rel_target_from(pres_part, &master),
        target_mode: None,
        extra: Vec::new(),
    });
    pres_rels.write_to(pkg, pres_part)?;
    insert_handout_master_id(&mut pres.root, &rid);
    pkg.set(pres_part, pres.to_bytes()?)?;
    let mut cts = ContentTypes::from_package(pkg)?;
    cts.set_override(&master, CT_HANDOUT_MASTER)?;
    if let Some(theme) = cloned_theme.as_deref() {
        cts.set_override(theme, CT_THEME)?;
    }
    cts.write_to(pkg)?;
    Ok(master)
}

fn insert_handout_master_id(pres_root: &mut Element, rid: &str) {
    if pres_root.child_by_local("handoutMasterIdLst").is_some() {
        return;
    }
    let mut item = Element::new(pres_root.prefixed("handoutMasterId"));
    item.attrs.push(("r:id".into(), rid.to_string()));
    let mut lst = Element::new(pres_root.prefixed("handoutMasterIdLst"));
    lst.push_element(item);
    let after = ["notesMasterIdLst", "sldMasterIdLst"]
        .into_iter()
        .find_map(|name| {
            pres_root
                .children
                .iter()
                .position(|n| matches!(n, Node::Element(e) if e.local_name() == name))
        });
    if let Some(i) = after {
        pres_root.children.insert(i + 1, Node::Element(lst));
        return;
    }
    let before = pres_root
        .children
        .iter()
        .position(|n| matches!(n, Node::Element(e) if e.local_name() == "sldIdLst"));
    if let Some(i) = before {
        pres_root.children.insert(i, Node::Element(lst));
    } else {
        pres_root.push_element(lst);
    }
}

fn next_part(pkg: &Package, dir: &str, stem: &str) -> String {
    let mut n = 1u32;
    loop {
        let name = format!("{dir}/{stem}{n}.xml");
        if !pkg.contains(&name) {
            return name;
        }
        n += 1;
    }
}

fn slot_json(root: &Element, ph: &str) -> Value {
    match find_sp_ph(root, ph) {
        None => Value::Null,
        Some(sp) => json!(slot_text(sp)),
    }
}

/// 讲义槽位把 `a:fld` 缓存也算进去，与幻灯片形状 `text` 跳过域不同。
fn slot_text(sp: &Element) -> String {
    let Some(tx) = sp.child_by_local("txBody") else {
        return String::new();
    };
    let mut out = String::new();
    collect_slot_text(tx, &mut out);
    out
}

fn collect_slot_text(el: &Element, out: &mut String) {
    if el.local_name() == "AlternateContent" {
        if let Some(i) = shape::first_alt_child_index(el) {
            if let Node::Element(child) = &el.children[i] {
                collect_slot_text(child, out);
            }
        }
        return;
    }
    match el.local_name() {
        "t" => out.push_str(&el.text_content()),
        "br" => out.push('\n'),
        _ => {}
    }
    for n in &el.children {
        if let Node::Element(e) = n {
            collect_slot_text(e, out);
        }
    }
}

fn find_sp_ph<'a>(el: &'a Element, ph: &str) -> Option<&'a Element> {
    if el.local_name() == "AlternateContent" {
        if let Some(i) = shape::first_alt_child_index(el) {
            if let Node::Element(child) = &el.children[i] {
                return find_sp_ph(child, ph);
            }
        }
        return None;
    }
    if el.local_name() == "sp" && notes::placeholder_type(el) == Some(ph) {
        return Some(el);
    }
    for child in &el.children {
        if let Node::Element(e) = child {
            if let Some(found) = find_sp_ph(e, ph) {
                return Some(found);
            }
        }
    }
    None
}

fn find_sp_ph_mut<'a>(el: &'a mut Element, ph: &str) -> Option<&'a mut Element> {
    if el.local_name() == "AlternateContent" {
        if let Some(i) = shape::first_alt_child_index(el) {
            if let Node::Element(child) = &mut el.children[i] {
                return find_sp_ph_mut(child, ph);
            }
        }
        return None;
    }
    if el.local_name() == "sp" && notes::placeholder_type(el) == Some(ph) {
        return Some(el);
    }
    for child in el.children.as_mut_slice() {
        if let Node::Element(e) = child {
            if let Some(found) = find_sp_ph_mut(e, ph) {
                return Some(found);
            }
        }
    }
    None
}

fn append_placeholder(root: &mut Element, ph: &str) -> Result<(), Error> {
    let cnv = picture::next_cnv_id(root);
    let name = match ph {
        "hdr" => "页眉占位符",
        "ftr" => "页脚占位符",
        "dt" => "日期占位符",
        _ => "幻灯片编号占位符",
    };
    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?><p:sp xmlns:a="{DML_NS}" xmlns:p="{PML_NS}"><p:nvSpPr><p:cNvPr id="{cnv}" name="{name}"/><p:cNvSpPr><a:spLocks noGrp="1"/></p:cNvSpPr><p:nvPr><p:ph type="{ph}"/></p:nvPr></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:endParaRPr lang="zh-CN"/></a:p></p:txBody></p:sp>"#
    );
    let frag = xml::parse(xml.as_bytes())?;
    picture::insert_into_sp_tree(root, frag.root)
}

fn enable_hf(root: &mut Element, ph: &str) {
    let Some(hf) = root.child_by_local_mut("hf") else {
        return;
    };
    let attr = match ph {
        "hdr" => "hdr",
        "ftr" => "ftr",
        "dt" => "dt",
        "sldNum" => "sldNum",
        _ => return,
    };
    hf.set_attr(attr, "1");
}

fn strip_fields(el: &mut Element) {
    let mut i = 0;
    while i < el.children.len() {
        match &mut el.children[i] {
            Node::Element(e) if e.local_name() == "fld" => {
                el.children.remove(i);
            }
            Node::Element(e) => {
                strip_fields(e);
                i += 1;
            }
            _ => i += 1,
        }
    }
}
