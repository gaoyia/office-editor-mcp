//! 段落 `w:numPr` 与 `numbering.xml`：inspect 名单与插改绑。

use serde_json::{json, Map, Value};

use crate::error::{Error, ErrorKind, IdReason};
use crate::opc::{self, ContentTypes, Package, Relationship, Relationships};
use crate::xml::{self, Document, Element, Node};

use super::{CT_NUMBERING, REL_NUMBERING, WML_NS};

pub const NUM_FMT_SET: &[&str] = &[
    "decimal",
    "bullet",
    "lowerLetter",
    "upperLetter",
    "lowerRoman",
    "upperRoman",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumberingKind {
    Decimal,
    Bullet,
}

pub fn parse_kind(s: &str) -> Option<NumberingKind> {
    match s {
        "decimal" => Some(NumberingKind::Decimal),
        "bullet" => Some(NumberingKind::Bullet),
        _ => None,
    }
}

pub fn numbering_part(pkg: &Package, doc_part: &str) -> Result<Option<String>, Error> {
    let rels = Relationships::from_package(pkg, doc_part)?;
    let Some(rel) = rels
        .rels
        .iter()
        .find(|r| r.rel_type == REL_NUMBERING && !r.is_external())
    else {
        return Ok(None);
    };
    Ok(Some(opc::resolve_rel_target(doc_part, &rel.target)?))
}

pub fn count_nums(pkg: &Package, doc_part: &str) -> Result<usize, Error> {
    let Some(part) = numbering_part(pkg, doc_part)? else {
        return Ok(0);
    };
    let doc = xml::parse(pkg.get(&part)?)?;
    Ok(collect_num_els(&doc.root).len())
}

pub fn numberings_json(pkg: &Package, doc_part: &str) -> Result<Vec<Value>, Error> {
    let Some(part) = numbering_part(pkg, doc_part)? else {
        return Ok(Vec::new());
    };
    let doc = xml::parse(pkg.get(&part)?)?;
    let mut out = Vec::new();
    for num in collect_num_els(&doc.root) {
        let Some(num_id) = parse_id_attr(num, "numId").filter(|&n| n > 0) else {
            continue;
        };
        let abstract_id = num
            .child_by_local("abstractNumId")
            .and_then(|e| parse_id_attr(e, "val"))
            .unwrap_or(0);
        let abs = find_abstract(&doc.root, abstract_id);
        let multi = abs
            .and_then(|a| a.child_by_local("multiLevelType"))
            .and_then(|e| e.attr_local("val"))
            .map(str::to_string);
        let levels = merged_levels(abs, num);
        out.push(json!({
            "id": format!("num:{num_id}"),
            "numId": num_id,
            "abstractId": format!("abstractNum:{abstract_id}"),
            "abstractNumId": abstract_id,
            "multiLevelType": multi,
            "levels": levels,
        }));
    }
    Ok(out)
}

pub fn para_num(p: &Element) -> Option<(String, u32)> {
    let num_pr = p.child_by_local("pPr")?.child_by_local("numPr")?;
    let num_id = num_pr
        .child_by_local("numId")
        .and_then(|e| parse_id_attr(e, "val"))
        .filter(|&n| n > 0)?;
    let ilvl = num_pr
        .child_by_local("ilvl")
        .and_then(|e| parse_id_attr(e, "val"))
        .unwrap_or(0);
    Some((format!("num:{num_id}"), ilvl))
}

pub fn parse_num_ref(id: &str, path: &str) -> Result<u32, Error> {
    let rest = id.strip_prefix("num:").ok_or_else(|| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::NumPrefix,
            },
            path.to_string(),
        )
    })?;
    parse_xml_id(
        rest,
        path,
        IdReason::NumIndexNotInt,
        IdReason::NumIndexFromOne,
    )
}

pub fn parse_abstract_ref(id: &str, path: &str) -> Result<u32, Error> {
    let rest = id.strip_prefix("abstractNum:").ok_or_else(|| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::AbstractNumPrefix,
            },
            path.to_string(),
        )
    })?;
    rest.parse().map_err(|_| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::AbstractNumIndexNotInt,
            },
            path.to_string(),
        )
    })
}

pub fn ensure_part(
    pkg: &mut Package,
    docs: &mut std::collections::HashMap<String, Document>,
    dirty: &mut std::collections::HashMap<String, ()>,
    doc_part: &str,
) -> Result<String, Error> {
    if let Some(existing) = numbering_part(pkg, doc_part)? {
        return Ok(existing);
    }
    let target = "numbering.xml";
    let part = opc::resolve_rel_target(doc_part, target)?;
    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:numbering xmlns:w="{WML_NS}"/>"#
    );
    pkg.set(&part, xml.into_bytes())?;
    let parsed = xml::parse(pkg.get(&part)?)?;
    docs.insert(part.clone(), parsed);
    dirty.insert(part.clone(), ());

    let mut rels = Relationships::from_package(pkg, doc_part)?;
    let rid = rels.next_rid();
    rels.push(Relationship {
        id: rid,
        rel_type: REL_NUMBERING.to_string(),
        target: target.into(),
        target_mode: None,
        extra: Vec::new(),
    });
    rels.write_to(pkg, doc_part)?;

    let mut cts = ContentTypes::from_package(pkg)?;
    cts.set_override(&part, CT_NUMBERING)?;
    cts.write_to(pkg)?;
    Ok(part)
}

pub fn insert_definition(root: &mut Element, kind: NumberingKind) -> (u32, u32) {
    let abstract_id = next_abstract_id(root);
    let num_id = next_num_id(root);
    let nsid = format!(
        "{:08X}",
        abstract_id.wrapping_mul(0x1357_9BDF).wrapping_add(1)
    );
    let abs = build_abstract(root, abstract_id, &nsid, kind);
    let num = build_num(root, num_id, abstract_id);
    insert_abstract(root, abs);
    root.push_element(num);
    (num_id, abstract_id)
}

pub fn set_level(
    root: &mut Element,
    abstract_id: u32,
    ilvl: u32,
    num_fmt: Option<&str>,
    start: Option<u32>,
    lvl_text: Option<&str>,
    id: &str,
    i: usize,
) -> Result<(), Error> {
    let abs = find_abstract_mut(root, abstract_id).ok_or_else(|| {
        Error::with_path(
            ErrorKind::AbstractNumNotFound { id: id.to_string() },
            format!("ops[{i}].id"),
        )
    })?;
    let Some(lvl) = find_lvl_mut(abs, ilvl) else {
        return Err(Error::with_path(
            ErrorKind::InvalidIlvl,
            format!("ops[{i}].ilvl"),
        ));
    };
    if let Some(fmt) = num_fmt {
        set_child_val(lvl, "numFmt", fmt);
    }
    if let Some(n) = start {
        set_child_val(lvl, "start", &n.to_string());
    }
    if let Some(t) = lvl_text {
        set_child_val(lvl, "lvlText", t);
    }
    Ok(())
}

pub fn bind_para(p: &mut Element, num_id: u32, ilvl: u32) {
    ensure_ppr(p);
    let ppr = p.child_by_local_mut("pPr").expect("刚写入 pPr");
    if let Some(existing) = ppr
        .children
        .iter()
        .position(|n| matches!(n, Node::Element(e) if e.local_name() == "numPr"))
    {
        ppr.children.remove(existing);
    }
    let mut num_pr = Element::new(ppr.prefixed("numPr"));
    let mut ilvl_el = Element::new(num_pr.prefixed("ilvl"));
    ilvl_el.set_attr(&ilvl_el.prefixed("val"), ilvl.to_string());
    let mut nid = Element::new(num_pr.prefixed("numId"));
    nid.set_attr(&nid.prefixed("val"), num_id.to_string());
    num_pr.push_element(ilvl_el);
    num_pr.push_element(nid);
    let after_style = ppr
        .children
        .iter()
        .position(|n| matches!(n, Node::Element(e) if e.local_name() == "pStyle"))
        .map(|i| i + 1)
        .unwrap_or(0);
    ppr.children.insert(after_style, Node::Element(num_pr));
}

pub fn unbind_para(p: &mut Element) {
    let Some(ppr) = p.child_by_local_mut("pPr") else {
        return;
    };
    ppr.children
        .retain(|n| !matches!(n, Node::Element(e) if e.local_name() == "numPr"));
}

pub fn num_exists(root: &Element, num_id: u32) -> bool {
    collect_num_els(root)
        .iter()
        .any(|n| parse_id_attr(n, "numId") == Some(num_id))
}

fn parse_xml_id(s: &str, path: &str, not_int: IdReason, from_one: IdReason) -> Result<u32, Error> {
    let n: u32 = s.parse().map_err(|_| {
        Error::with_path(ErrorKind::InvalidId { reason: not_int }, path.to_string())
    })?;
    if n < 1 {
        return Err(Error::with_path(
            ErrorKind::InvalidId { reason: from_one },
            path.to_string(),
        ));
    }
    Ok(n)
}

fn parse_id_attr(el: &Element, local: &str) -> Option<u32> {
    el.attr_local(local)?.parse().ok()
}

fn collect_num_els(root: &Element) -> Vec<&Element> {
    root.children_by_local("num")
        .into_iter()
        .filter(|n| parse_id_attr(n, "numId").is_some_and(|id| id > 0))
        .collect()
}

fn find_abstract(root: &Element, id: u32) -> Option<&Element> {
    root.children_by_local("abstractNum")
        .into_iter()
        .find(|a| parse_id_attr(a, "abstractNumId") == Some(id))
}

fn find_abstract_mut(root: &mut Element, id: u32) -> Option<&mut Element> {
    root.element_children_mut()
        .find(|a| a.local_name() == "abstractNum" && parse_id_attr(a, "abstractNumId") == Some(id))
}

fn find_lvl_mut(abs: &mut Element, ilvl: u32) -> Option<&mut Element> {
    abs.element_children_mut()
        .find(|e| e.local_name() == "lvl" && parse_id_attr(e, "ilvl") == Some(ilvl))
}

fn merged_levels(abs: Option<&Element>, num: &Element) -> Vec<Value> {
    let mut by_ilvl: std::collections::BTreeMap<u32, Element> = std::collections::BTreeMap::new();
    if let Some(a) = abs {
        for lvl in a.children_by_local("lvl") {
            if let Some(ilvl) = parse_id_attr(lvl, "ilvl") {
                by_ilvl.insert(ilvl, lvl.clone());
            }
        }
    }
    for ov in num.children_by_local("lvlOverride") {
        let Some(ilvl) = parse_id_attr(ov, "ilvl") else {
            continue;
        };
        if let Some(lvl) = ov.child_by_local("lvl") {
            by_ilvl.insert(ilvl, lvl.clone());
        }
    }
    by_ilvl
        .into_iter()
        .map(|(ilvl, lvl)| {
            json!({
                "ilvl": ilvl,
                "numFmt": child_val(&lvl, "numFmt"),
                "lvlText": child_val(&lvl, "lvlText"),
                "start": child_val(&lvl, "start").and_then(|s| s.parse::<u32>().ok()),
                "lvlJc": child_val(&lvl, "lvlJc"),
            })
        })
        .collect()
}

fn child_val(el: &Element, local: &str) -> Option<String> {
    el.child_by_local(local)
        .and_then(|e| e.attr_local("val"))
        .map(str::to_string)
}

fn set_child_val(parent: &mut Element, local: &str, val: &str) {
    if let Some(el) = parent.child_by_local_mut(local) {
        let key = el.prefixed("val");
        el.set_attr(&key, val);
        return;
    }
    let mut el = Element::new(parent.prefixed(local));
    let key = el.prefixed("val");
    el.set_attr(&key, val);
    parent.push_element(el);
}

fn next_abstract_id(root: &Element) -> u32 {
    root.children_by_local("abstractNum")
        .into_iter()
        .filter_map(|a| parse_id_attr(a, "abstractNumId"))
        .max()
        .unwrap_or(0)
        .saturating_add(1)
}

fn next_num_id(root: &Element) -> u32 {
    root.children_by_local("num")
        .into_iter()
        .filter_map(|n| parse_id_attr(n, "numId"))
        .max()
        .unwrap_or(0)
        .saturating_add(1)
        .max(1)
}

fn insert_abstract(root: &mut Element, abs: Element) {
    let pos = root
        .children
        .iter()
        .position(|n| matches!(n, Node::Element(e) if e.local_name() == "num"))
        .unwrap_or(root.children.len());
    root.children.insert(pos, Node::Element(abs));
}

fn build_num(parent: &Element, num_id: u32, abstract_id: u32) -> Element {
    let mut num = Element::new(parent.prefixed("num"));
    num.set_attr(&num.prefixed("numId"), num_id.to_string());
    let mut aid = Element::new(num.prefixed("abstractNumId"));
    aid.set_attr(&aid.prefixed("val"), abstract_id.to_string());
    num.push_element(aid);
    num
}

fn build_abstract(parent: &Element, id: u32, nsid: &str, kind: NumberingKind) -> Element {
    let mut abs = Element::new(parent.prefixed("abstractNum"));
    abs.set_attr(&abs.prefixed("abstractNumId"), id.to_string());
    let mut ns = Element::new(abs.prefixed("nsid"));
    ns.set_attr(&ns.prefixed("val"), nsid);
    abs.push_element(ns);
    let mut mt = Element::new(abs.prefixed("multiLevelType"));
    mt.set_attr(&mt.prefixed("val"), "hybridMultilevel");
    abs.push_element(mt);
    for ilvl in 0..9u32 {
        abs.push_element(build_lvl(&abs, ilvl, kind));
    }
    abs
}

fn build_lvl(parent: &Element, ilvl: u32, kind: NumberingKind) -> Element {
    let mut lvl = Element::new(parent.prefixed("lvl"));
    lvl.set_attr(&lvl.prefixed("ilvl"), ilvl.to_string());
    let mut start = Element::new(lvl.prefixed("start"));
    start.set_attr(&start.prefixed("val"), "1");
    lvl.push_element(start);
    let (fmt, text) = match kind {
        NumberingKind::Decimal => ("decimal", level_decimal_text(ilvl)),
        NumberingKind::Bullet => ("bullet", bullet_text(ilvl).to_string()),
    };
    let mut num_fmt = Element::new(lvl.prefixed("numFmt"));
    num_fmt.set_attr(&num_fmt.prefixed("val"), fmt);
    lvl.push_element(num_fmt);
    let mut lvl_text = Element::new(lvl.prefixed("lvlText"));
    lvl_text.set_attr(&lvl_text.prefixed("val"), text);
    lvl.push_element(lvl_text);
    let mut jc = Element::new(lvl.prefixed("lvlJc"));
    jc.set_attr(&jc.prefixed("val"), "left");
    lvl.push_element(jc);
    let left = 720 * (ilvl + 1);
    let mut ppr = Element::new(lvl.prefixed("pPr"));
    let mut ind = Element::new(ppr.prefixed("ind"));
    ind.set_attr(&ind.prefixed("left"), left.to_string());
    ind.set_attr(&ind.prefixed("hanging"), "360");
    ppr.push_element(ind);
    lvl.push_element(ppr);
    if kind == NumberingKind::Bullet {
        let mut rpr = Element::new(lvl.prefixed("rPr"));
        let mut fonts = Element::new(rpr.prefixed("rFonts"));
        fonts.set_attr(&fonts.prefixed("ascii"), "Symbol");
        fonts.set_attr(&fonts.prefixed("hAnsi"), "Symbol");
        fonts.set_attr(&fonts.prefixed("hint"), "default");
        rpr.push_element(fonts);
        lvl.push_element(rpr);
    }
    lvl
}

fn level_decimal_text(ilvl: u32) -> String {
    let mut s = String::new();
    for i in 1..=ilvl + 1 {
        if i > 1 {
            s.push('.');
        }
        s.push('%');
        s.push(char::from(b'0' + i as u8));
    }
    s.push('.');
    s
}

fn bullet_text(ilvl: u32) -> &'static str {
    match ilvl % 3 {
        0 => "\u{F0B7}",
        1 => "o",
        _ => "\u{F0A7}",
    }
}

fn ensure_ppr(p: &mut Element) {
    if p.child_by_local("pPr").is_some() {
        return;
    }
    let ppr = Element::new(p.prefixed("pPr"));
    p.children.insert(0, Node::Element(ppr));
}

pub fn missing_num(id: &str, i: usize) -> Error {
    Error::with_path(
        ErrorKind::NumberingNotFound { id: id.to_string() },
        format!("ops[{i}].num"),
    )
}

pub fn missing_abstract(id: &str, i: usize) -> Error {
    Error::with_path(
        ErrorKind::AbstractNumNotFound { id: id.to_string() },
        format!("ops[{i}].id"),
    )
}

/// 给段落 JSON 插入 `num` / `ilvl`，保持字段顺序。
pub fn stamp_para(node: &mut Map<String, Value>, p: &Element) {
    let Some((num, ilvl)) = para_num(p) else {
        return;
    };
    let mut out = Map::new();
    for (k, v) in node.iter() {
        if k == "text" {
            out.insert("num".into(), json!(num.clone()));
            out.insert("ilvl".into(), json!(ilvl));
        }
        out.insert(k.clone(), v.clone());
    }
    if !out.contains_key("num") {
        out.insert("num".into(), json!(num));
        out.insert("ilvl".into(), json!(ilvl));
    }
    *node = out;
}

use super::hf;
use super::request::InsertNumberingOp;
use super::request::SetNumberingOp;
use super::request::SetParaNumberingOp;
use std::collections::HashMap;
// 从 apply 搬来的操作体。
pub(crate) fn apply_insert_numbering(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    op: &InsertNumberingOp,
) -> Result<(), Error> {
    let part = super::numbering::ensure_part(pkg, docs, dirty, doc_part)?;
    let doc = super::apply::load_part(pkg, docs, &part)?;
    super::numbering::insert_definition(&mut doc.root, op.kind);
    dirty.insert(part, ());
    Ok(())
}
pub(crate) fn apply_set_numbering(
    pkg: &Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    op: &SetNumberingOp,
    i: usize,
) -> Result<(), Error> {
    let Some(part) = super::numbering::numbering_part(pkg, doc_part)? else {
        return Err(super::numbering::missing_abstract(&op.id, i));
    };
    let doc = super::apply::load_part(pkg, docs, &part)?;
    super::numbering::set_level(
        &mut doc.root,
        op.abstract_id,
        op.ilvl,
        op.num_fmt.as_deref(),
        op.start,
        op.lvl_text.as_deref(),
        &op.id,
        i,
    )?;
    dirty.insert(part, ());
    Ok(())
}
pub(crate) fn apply_set_para_numbering(
    pkg: &Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &SetParaNumberingOp,
    i: usize,
) -> Result<(), Error> {
    if let Some(num_id) = op.num {
        let num_part = super::numbering::numbering_part(pkg, doc_part)?
            .ok_or_else(|| super::numbering::missing_num(&format!("num:{num_id}"), i))?;
        {
            let num_doc = super::apply::load_part(pkg, docs, &num_part)?;
            if !super::numbering::num_exists(&num_doc.root, num_id) {
                return Err(super::numbering::missing_num(&format!("num:{num_id}"), i));
            }
        }
    }
    let part_name = super::apply::part_for_para(pkg, catalog, doc_part, &op.para, &op.id, i)?;
    let doc = super::apply::load_part(pkg, docs, &part_name)?;
    let el = super::apply::find_insert_para_mut(&mut doc.root, &op.para).ok_or_else(|| {
        Error::with_path(
            ErrorKind::ParaNotFound { id: op.id.clone() },
            format!("ops[{i}].id"),
        )
    })?;
    match op.num {
        Some(n) => super::numbering::bind_para(el, n, op.ilvl),
        None => super::numbering::unbind_para(el),
    }
    dirty.insert(part_name, ());
    Ok(())
}
