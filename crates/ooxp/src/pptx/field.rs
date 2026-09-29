//! 幻灯片文本形状里的 `a:fld`（ECMA-376 §21.1.2.2.4）。不算放映引擎，不刷新缓存。

use serde_json::{json, Map, Value};

use crate::error::{Error, ErrorKind, IdReason};
use crate::xml::{Element, Node};

use super::picture;
use super::section;
use super::shape::{self, ShapeTarget};

const DML_NS: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";

#[derive(Debug, Clone)]
pub enum FieldKey {
    Guid(String),
    Index(usize),
}

#[derive(Debug, Clone)]
pub struct InsertFieldOp {
    pub id: String,
    pub shape: ShapeTarget,
    pub type_name: String,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct SetFieldOp {
    pub id: String,
    pub slide: usize,
    pub key: FieldKey,
    pub type_name: Option<String>,
    pub text: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DeleteFieldOp {
    pub id: String,
    pub slide: usize,
    pub key: FieldKey,
    pub keep: FieldKeep,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldKeep {
    Result,
    Instr,
}

pub fn is_reserved_type(s: &str) -> bool {
    matches!(
        s,
        "slidenum"
            | "datetime"
            | "datetime1"
            | "datetime2"
            | "datetime3"
            | "datetime4"
            | "datetime5"
            | "datetime6"
            | "datetime7"
            | "datetime8"
            | "datetime9"
            | "datetime10"
            | "datetime11"
            | "datetime12"
            | "datetime13"
    )
}

pub fn count(root: &Element) -> usize {
    let mut n = 0usize;
    for sp in shape::collect_text_shapes(root) {
        n += flds_in(sp).len();
    }
    n
}

pub fn list_json(root: &Element, slide: usize) -> Value {
    let host_prefix = format!("slide:{slide}");
    let mut out = Vec::new();
    let mut index = 0usize;
    for (shape_index, sp) in shape::collect_text_shapes(root).into_iter().enumerate() {
        let (host, _, _) = shape::shape_stable_id(sp, &host_prefix, shape_index + 1);
        for el in flds_in(sp) {
            index += 1;
            out.push(item_json(el, slide, &host, index));
        }
    }
    Value::Array(out)
}

fn item_json(el: &Element, slide: usize, host: &str, index: usize) -> Value {
    let mut obj = Map::new();
    obj.insert("id".into(), json!(field_id(el, slide, index)));
    obj.insert(
        "type".into(),
        json!(el
            .attr("type")
            .or_else(|| el.attr_local("type"))
            .unwrap_or("")),
    );
    obj.insert("host".into(), json!(host));
    obj.insert("text".into(), json!(cache_text(el)));
    Value::Object(obj)
}

fn field_id(el: &Element, slide: usize, index: usize) -> String {
    match el.attr("id").or_else(|| el.attr_local("id")) {
        Some(g) if !g.is_empty() => format!("slide:{slide}/fld:{g}"),
        _ => format!("slide:{slide}/fld:index:{index}"),
    }
}

fn cache_text(el: &Element) -> String {
    el.child_by_local("t")
        .map(|t| t.text_content())
        .unwrap_or_default()
}

fn flds_in(sp: &Element) -> Vec<&Element> {
    let mut out = Vec::new();
    if let Some(tx) = sp.child_by_local("txBody") {
        collect_flds(tx, &mut out);
    }
    out
}

fn collect_flds<'a>(el: &'a Element, out: &mut Vec<&'a Element>) {
    if el.local_name() == "AlternateContent" {
        if let Some(i) = shape::first_alt_child_index(el) {
            if let Node::Element(child) = &el.children[i] {
                collect_flds(child, out);
            }
        }
        return;
    }
    if el.local_name() == "fld" {
        out.push(el);
        return;
    }
    for n in &el.children {
        if let Node::Element(e) = n {
            collect_flds(e, out);
        }
    }
}

pub fn parse_field_id(id: &str, op_index: usize) -> Result<(usize, FieldKey), Error> {
    let path = format!("ops[{op_index}].id");
    let rest = id
        .strip_prefix("slide:")
        .ok_or_else(|| invalid_form(&path))?;
    let Some((n, after)) = rest.split_once("/fld:") else {
        return Err(invalid_form(&path));
    };
    if after.starts_with("notes/") || after.contains("/shape:") {
        return Err(invalid_form(&path));
    }
    let slide: usize = n.parse().map_err(|_| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SlideIndexNotInt,
            },
            path.clone(),
        )
    })?;
    if slide < 1 {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SlideIndexFromOne,
            },
            path,
        ));
    }
    if let Some(idx) = after.strip_prefix("index:") {
        let k: usize = idx.parse().map_err(|_| {
            Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::FieldIndexNotInt,
                },
                path.clone(),
            )
        })?;
        if k < 1 {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::FieldIndexFromOne,
                },
                path,
            ));
        }
        return Ok((slide, FieldKey::Index(k)));
    }
    if after.starts_with('{') && after.ends_with('}') && after.len() > 2 {
        return Ok((slide, FieldKey::Guid(after.to_string())));
    }
    Err(invalid_form(&path))
}

fn invalid_form(path: &str) -> Error {
    Error::with_path(
        ErrorKind::InvalidId {
            reason: IdReason::PptxFieldForm,
        },
        path,
    )
}

pub fn apply_insert(root: &mut Element, op: &InsertFieldOp, i: usize) -> Result<(), Error> {
    picture::ensure_xmlns(root, "a", DML_NS);
    let sp = shape::find_shape_mut(root, &op.shape.key).ok_or_else(|| {
        Error::with_path(
            ErrorKind::ShapeNotFound { id: op.id.clone() },
            format!("ops[{i}].id"),
        )
    })?;
    if sp.child_by_local("txBody").is_none() {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::InsertFieldNeedsShape,
            },
            format!("ops[{i}].id"),
        ));
    }
    let tx = sp.child_by_local_mut("txBody").ok_or_else(|| {
        Error::with_path(
            ErrorKind::ShapeNotFound { id: op.id.clone() },
            format!("ops[{i}].id"),
        )
    })?;
    let used = existing_guids(tx);
    let guid = fresh_guid(&used);
    if last_para_index(tx).is_none() {
        tx.push_element(Element::new(tx.prefixed("p")));
    }
    let para_i = last_para_index(tx).ok_or_else(|| {
        Error::with_path(
            ErrorKind::ShapeNotFound { id: op.id.clone() },
            format!("ops[{i}].id"),
        )
    })?;
    let Node::Element(p) = &mut tx.children[para_i] else {
        return Err(Error::with_path(
            ErrorKind::ShapeNotFound { id: op.id.clone() },
            format!("ops[{i}].id"),
        ));
    };
    let rpr = last_rpr(p);
    let mut fld = Element::new(p.prefixed("fld"))
        .with_attr("id", guid)
        .with_attr("type", op.type_name.clone());
    if let Some(mut copied) = rpr {
        copied.name = p.prefixed("rPr");
        copied.set_attr("smtClean", "0");
        fld.push_element(copied);
    }
    let mut t = Element::new(fld.prefixed("t"));
    if !op.text.is_empty() {
        t.set_text(&op.text);
        if needs_xml_space(&op.text) {
            t.set_attr("xml:space", "preserve");
        }
    }
    fld.push_element(t);
    let at = p
        .children
        .iter()
        .position(|n| matches!(n, Node::Element(e) if e.local_name() == "endParaRPr"))
        .unwrap_or(p.children.len());
    p.children.insert(at, Node::Element(fld));
    Ok(())
}

fn last_para_index(tx: &Element) -> Option<usize> {
    tx.children
        .iter()
        .rposition(|n| matches!(n, Node::Element(e) if e.local_name() == "p"))
}

fn last_rpr(p: &Element) -> Option<Element> {
    for n in p.children.iter().rev() {
        let Node::Element(e) = n else {
            continue;
        };
        if matches!(e.local_name(), "r" | "fld") {
            if let Some(rpr) = e.child_by_local("rPr") {
                return Some(rpr.clone());
            }
        }
    }
    p.child_by_local("endParaRPr").cloned()
}

fn existing_guids(tx: &Element) -> Vec<String> {
    let mut flds = Vec::new();
    collect_flds(tx, &mut flds);
    flds.into_iter()
        .filter_map(|f| el_id(f).map(str::to_string))
        .collect()
}

fn el_id(el: &Element) -> Option<&str> {
    el.attr("id").or_else(|| el.attr_local("id"))
}

fn fresh_guid(used: &[String]) -> String {
    for _ in 0..8 {
        let g = section::new_guid();
        if !used.iter().any(|u| u == &g) {
            return g;
        }
    }
    section::new_guid()
}

fn needs_xml_space(text: &str) -> bool {
    text.starts_with(' ') || text.starts_with('\t') || text.ends_with(' ') || text.ends_with('\t')
}

pub fn apply_set(root: &mut Element, op: &SetFieldOp, i: usize) -> Result<(), Error> {
    let el = find_fld_mut(root, &op.key).ok_or_else(|| {
        Error::with_path(
            ErrorKind::FieldNotFound { id: op.id.clone() },
            format!("ops[{i}].id"),
        )
    })?;
    if let Some(ty) = &op.type_name {
        el.set_attr("type", ty.clone());
    }
    if let Some(text) = &op.text {
        let tname = el.prefixed("t");
        if let Some(t) = el.child_by_local_mut("t") {
            t.set_text(text);
            if needs_xml_space(text) {
                t.set_attr("xml:space", "preserve");
            } else {
                t.remove_attr("xml:space");
            }
        } else {
            let mut t = Element::new(tname);
            if !text.is_empty() {
                t.set_text(text);
                if needs_xml_space(text) {
                    t.set_attr("xml:space", "preserve");
                }
            }
            el.push_element(t);
        }
    }
    Ok(())
}

pub fn apply_delete(root: &mut Element, op: &DeleteFieldOp, i: usize) -> Result<(), Error> {
    if take_fld(root, &op.key, op.keep) {
        Ok(())
    } else {
        Err(Error::with_path(
            ErrorKind::FieldNotFound { id: op.id.clone() },
            format!("ops[{i}].id"),
        ))
    }
}

fn find_fld_mut<'a>(root: &'a mut Element, key: &FieldKey) -> Option<&'a mut Element> {
    let mut idx = 0usize;
    find_fld_in_shapes(root, key, &mut idx)
}

fn find_fld_in_shapes<'a>(
    el: &'a mut Element,
    key: &FieldKey,
    idx: &mut usize,
) -> Option<&'a mut Element> {
    if el.local_name() == "AlternateContent" {
        if let Some(i) = shape::first_alt_child_index(el) {
            if let Node::Element(child) = &mut el.children[i] {
                return find_fld_in_shapes(child, key, idx);
            }
        }
        return None;
    }
    if el.local_name() == "sp" && el.child_by_local("txBody").is_some() {
        if let Some(tx) = el.child_by_local_mut("txBody") {
            return find_fld_in_tx(tx, key, idx);
        }
        return None;
    }
    if el.local_name() == "oleObj" {
        return None;
    }
    for child in el.children.as_mut_slice() {
        if let Node::Element(e) = child {
            if let Some(found) = find_fld_in_shapes(e, key, idx) {
                return Some(found);
            }
        }
    }
    None
}

fn find_fld_in_tx<'a>(
    el: &'a mut Element,
    key: &FieldKey,
    idx: &mut usize,
) -> Option<&'a mut Element> {
    if el.local_name() == "AlternateContent" {
        if let Some(i) = shape::first_alt_child_index(el) {
            if let Node::Element(child) = &mut el.children[i] {
                return find_fld_in_tx(child, key, idx);
            }
        }
        return None;
    }
    if el.local_name() == "fld" {
        *idx += 1;
        if matches_key(el, key, *idx) {
            return Some(el);
        }
        return None;
    }
    for child in el.children.as_mut_slice() {
        if let Node::Element(e) = child {
            if let Some(found) = find_fld_in_tx(e, key, idx) {
                return Some(found);
            }
        }
    }
    None
}

fn matches_key(el: &Element, key: &FieldKey, index: usize) -> bool {
    match key {
        FieldKey::Index(n) => *n == index,
        FieldKey::Guid(g) => el_id(el) == Some(g.as_str()),
    }
}

fn take_fld(root: &mut Element, key: &FieldKey, keep: FieldKeep) -> bool {
    let mut idx = 0usize;
    replace_in_shapes(root, key, &mut idx, keep)
}

fn replace_in_shapes(el: &mut Element, key: &FieldKey, idx: &mut usize, keep: FieldKeep) -> bool {
    if el.local_name() == "AlternateContent" {
        if let Some(i) = shape::first_alt_child_index(el) {
            if let Node::Element(child) = &mut el.children[i] {
                return replace_in_shapes(child, key, idx, keep);
            }
        }
        return false;
    }
    if el.local_name() == "sp" && el.child_by_local("txBody").is_some() {
        if let Some(tx) = el.child_by_local_mut("txBody") {
            return replace_in_tx(tx, key, idx, keep);
        }
        return false;
    }
    if el.local_name() == "oleObj" {
        return false;
    }
    for child in el.children.as_mut_slice() {
        if let Node::Element(e) = child {
            if replace_in_shapes(e, key, idx, keep) {
                return true;
            }
        }
    }
    false
}

fn replace_in_tx(el: &mut Element, key: &FieldKey, idx: &mut usize, keep: FieldKeep) -> bool {
    if el.local_name() == "AlternateContent" {
        if let Some(i) = shape::first_alt_child_index(el) {
            if let Node::Element(child) = &mut el.children[i] {
                return replace_in_tx(child, key, idx, keep);
            }
        }
        return false;
    }
    if el.local_name() == "p" {
        return replace_in_para(el, key, idx, keep);
    }
    for child in el.children.as_mut_slice() {
        if let Node::Element(e) = child {
            if replace_in_tx(e, key, idx, keep) {
                return true;
            }
        }
    }
    false
}

fn replace_in_para(p: &mut Element, key: &FieldKey, idx: &mut usize, keep: FieldKeep) -> bool {
    let mut i = 0usize;
    while i < p.children.len() {
        let hit = match &p.children[i] {
            Node::Element(e) if e.local_name() == "fld" => {
                *idx += 1;
                matches_key(e, key, *idx)
            }
            _ => false,
        };
        if hit {
            match keep {
                FieldKeep::Result => {
                    let run = fld_to_run(match &p.children[i] {
                        Node::Element(e) => e,
                        _ => unreachable!(),
                    });
                    p.children[i] = Node::Element(run);
                }
                FieldKeep::Instr => {
                    let run = fld_type_to_run(match &p.children[i] {
                        Node::Element(e) => e,
                        _ => unreachable!(),
                    });
                    p.children[i] = Node::Element(run);
                }
            }
            return true;
        }
        i += 1;
    }
    false
}

fn fld_to_run(fld: &Element) -> Element {
    let mut run = Element::new(fld.prefixed("r"));
    if let Some(rpr) = fld.child_by_local("rPr") {
        run.push_element(rpr.clone());
    }
    if let Some(t) = fld.child_by_local("t") {
        run.push_element(t.clone());
    } else {
        run.push_element(Element::new(run.prefixed("t")));
    }
    run
}

fn fld_type_to_run(fld: &Element) -> Element {
    let mut run = Element::new(fld.prefixed("r"));
    if let Some(rpr) = fld.child_by_local("rPr") {
        run.push_element(rpr.clone());
    }
    let ty = fld
        .attr("type")
        .or_else(|| fld.attr_local("type"))
        .unwrap_or("");
    let mut t = Element::new(run.prefixed("t"));
    t.set_text(ty);
    run.push_element(t);
    run
}
