//! `styles.xml`：inspect 名单、增删自定义样式、改定义、给段落换 `pStyle`。

use serde_json::{json, Value};

use crate::error::{Error, ErrorKind, IdReason};
use crate::opc::{self, ContentTypes, Package, Relationship, Relationships};
use crate::xml::{self, Document, Element, Node};

use super::{CT_STYLES, REL_STYLES, WML_NS};

pub fn styles_part(pkg: &Package, doc_part: &str) -> Result<Option<String>, Error> {
    let rels = Relationships::from_package(pkg, doc_part)?;
    let Some(rel) = rels
        .rels
        .iter()
        .find(|r| r.rel_type == REL_STYLES && !r.is_external())
    else {
        return Ok(None);
    };
    Ok(Some(opc::resolve_rel_target(doc_part, &rel.target)?))
}

pub fn count_styles(pkg: &Package, doc_part: &str) -> Result<usize, Error> {
    let Some(part) = styles_part(pkg, doc_part)? else {
        return Ok(0);
    };
    let doc = crate::xml::parse(pkg.get(&part)?)?;
    Ok(collect_style_els(&doc.root).len())
}

pub fn styles_json(pkg: &Package, doc_part: &str) -> Result<Vec<Value>, Error> {
    let Some(part) = styles_part(pkg, doc_part)? else {
        return Ok(Vec::new());
    };
    let doc = crate::xml::parse(pkg.get(&part)?)?;
    let mut out = Vec::new();
    for el in collect_style_els(&doc.root) {
        let Some(style_id) = el.attr_local("styleId").filter(|s| !s.is_empty()) else {
            continue;
        };
        let type_val = el.attr_local("type").map(str::to_string);
        let name = el
            .child_by_local("name")
            .and_then(|e| e.attr_local("val"))
            .map(str::to_string);
        let based_on = el
            .child_by_local("basedOn")
            .and_then(|e| e.attr_local("val"))
            .map(str::to_string);
        let default = matches!(el.attr_local("default").unwrap_or(""), "1" | "true" | "on");
        out.push(json!({
            "id": format!("style:{style_id}"),
            "styleId": style_id,
            "type": type_val,
            "name": name,
            "basedOn": based_on,
            "default": default,
        }));
    }
    Ok(out)
}

pub fn parse_style_ref(id: &str, path: &str) -> Result<String, Error> {
    let rest = id.strip_prefix("style:").ok_or_else(|| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::StylePrefix,
            },
            path.to_string(),
        )
    })?;
    if rest.is_empty() {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::StylePrefix,
            },
            path.to_string(),
        ));
    }
    Ok(rest.to_string())
}

pub fn is_paragraph_style(el: &Element) -> bool {
    matches!(el.attr_local("type"), None | Some("paragraph"))
}

pub fn is_character_style(el: &Element) -> bool {
    el.attr_local("type") == Some("character")
}

pub fn is_table_style(el: &Element) -> bool {
    el.attr_local("type") == Some("table")
}

pub fn is_numbering_style(el: &Element) -> bool {
    el.attr_local("type") == Some("numbering")
}

pub fn is_custom_style(el: &Element) -> bool {
    matches!(
        el.attr_local("customStyle").unwrap_or(""),
        "1" | "true" | "on"
    )
}

pub fn is_default_style(el: &Element) -> bool {
    matches!(el.attr_local("default").unwrap_or(""), "1" | "true" | "on")
}

pub fn validate_style_id_token(raw: &str, path: &str) -> Result<(), Error> {
    let mut chars = raw.chars();
    let Some(first) = chars.next() else {
        return Err(Error::with_path(
            ErrorKind::InvalidStyleIdToken,
            path.to_string(),
        ));
    };
    if !first.is_ascii_alphabetic() && first != '_' {
        return Err(Error::with_path(
            ErrorKind::InvalidStyleIdToken,
            path.to_string(),
        ));
    }
    if chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
        Ok(())
    } else {
        Err(Error::with_path(
            ErrorKind::InvalidStyleIdToken,
            path.to_string(),
        ))
    }
}

fn default_style_id_of_type(root: &Element, ty: &str) -> Option<String> {
    collect_style_els(root).into_iter().find_map(|el| {
        let matches_type = if ty == "paragraph" {
            is_paragraph_style(el)
        } else {
            el.attr_local("type") == Some(ty)
        };
        if matches_type && is_default_style(el) {
            el.attr_local("styleId").map(str::to_string)
        } else {
            None
        }
    })
}

pub fn find_style<'a>(root: &'a Element, style_id: &str) -> Option<&'a Element> {
    collect_style_els(root)
        .into_iter()
        .find(|s| s.attr_local("styleId") == Some(style_id))
}

pub fn find_style_mut<'a>(root: &'a mut Element, style_id: &str) -> Option<&'a mut Element> {
    root.children.iter_mut().find_map(|n| match n {
        Node::Element(e)
            if e.local_name() == "style" && e.attr_local("styleId") == Some(style_id) =>
        {
            Some(e)
        }
        _ => None,
    })
}

pub fn missing_style(id: &str, i: usize) -> Error {
    Error::with_path(
        ErrorKind::StyleNotFound { id: id.to_string() },
        format!("ops[{i}].id"),
    )
}

pub fn missing_para_style(id: &str, i: usize) -> Error {
    Error::with_path(
        ErrorKind::StyleNotFound { id: id.to_string() },
        format!("ops[{i}].style"),
    )
}

pub fn bind_para(p: &mut Element, style_id: &str) {
    ensure_ppr(p);
    let ppr = p.child_by_local_mut("pPr").expect("刚写入 pPr");
    if let Some(ps) = ppr.child_by_local_mut("pStyle") {
        let key = ps.prefixed("val");
        ps.set_attr(&key, style_id);
        return;
    }
    let mut ps = Element::new(ppr.prefixed("pStyle"));
    let key = ps.prefixed("val");
    ps.set_attr(&key, style_id);
    ppr.children.insert(0, Node::Element(ps));
}

pub fn unbind_para(p: &mut Element) {
    let Some(ppr) = p.child_by_local_mut("pPr") else {
        return;
    };
    ppr.children
        .retain(|n| !matches!(n, Node::Element(e) if e.local_name() == "pStyle"));
}

pub struct StylePatch<'a> {
    pub font: Option<&'a str>,
    pub east_asia_font: Option<&'a str>,
    pub sz: Option<u32>,
    pub bold: Option<bool>,
    pub color: Option<&'a str>,
    pub before: Option<u32>,
    pub after: Option<u32>,
}

pub fn set_style_props(
    root: &mut Element,
    style_id: &str,
    patch: StylePatch<'_>,
    id: &str,
    i: usize,
) -> Result<(), Error> {
    let Some(style) = find_style_mut(root, style_id) else {
        return Err(missing_style(id, i));
    };
    if is_numbering_style(style) {
        return Err(Error::with_path(
            ErrorKind::InvalidStyleType,
            format!("ops[{i}].id"),
        ));
    }
    let character = is_character_style(style);
    if character && (patch.before.is_some() || patch.after.is_some()) {
        return Err(Error::with_path(
            ErrorKind::SetStyleSpacingNeedsParagraph,
            format!("ops[{i}]"),
        ));
    }
    if patch.font.is_some()
        || patch.east_asia_font.is_some()
        || patch.sz.is_some()
        || patch.bold.is_some()
        || patch.color.is_some()
    {
        ensure_style_rpr(style);
        let rpr = style.child_by_local_mut("rPr").expect("刚写入 rPr");
        if patch.font.is_some() || patch.east_asia_font.is_some() {
            apply_fonts(rpr, patch.font, patch.east_asia_font);
        }
        if let Some(sz) = patch.sz {
            set_sz(rpr, sz);
        }
        if let Some(bold) = patch.bold {
            set_bold(rpr, bold);
        }
        if let Some(color) = patch.color {
            set_color(rpr, color);
        }
    }
    if !character && (patch.before.is_some() || patch.after.is_some()) {
        ensure_style_ppr(style);
        let ppr = style.child_by_local_mut("pPr").expect("刚写入 pPr");
        ensure_child(ppr, "spacing");
        let spacing = ppr.child_by_local_mut("spacing").expect("刚写入 spacing");
        if let Some(n) = patch.before {
            let key = spacing.prefixed("before");
            spacing.set_attr(&key, n.to_string());
        }
        if let Some(n) = patch.after {
            let key = spacing.prefixed("after");
            spacing.set_attr(&key, n.to_string());
        }
    }
    Ok(())
}

pub fn ensure_part(
    pkg: &mut Package,
    docs: &mut std::collections::HashMap<String, Document>,
    dirty: &mut std::collections::HashMap<String, ()>,
    doc_part: &str,
) -> Result<String, Error> {
    if let Some(existing) = styles_part(pkg, doc_part)? {
        return Ok(existing);
    }
    let target = "styles.xml";
    let part = opc::resolve_rel_target(doc_part, target)?;
    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:styles xmlns:w="{WML_NS}"/>"#
    );
    pkg.set(&part, xml.into_bytes())?;
    let parsed = xml::parse(pkg.get(&part)?)?;
    docs.insert(part.clone(), parsed);
    dirty.insert(part.clone(), ());

    let mut rels = Relationships::from_package(pkg, doc_part)?;
    let rid = rels.next_rid();
    rels.push(Relationship {
        id: rid,
        rel_type: REL_STYLES.to_string(),
        target: target.into(),
        target_mode: None,
        extra: Vec::new(),
    });
    rels.write_to(pkg, doc_part)?;

    let mut cts = ContentTypes::from_package(pkg)?;
    cts.set_override(&part, CT_STYLES)?;
    cts.write_to(pkg)?;
    Ok(part)
}

pub fn insert_style(
    root: &mut Element,
    style_id: &str,
    ty: &str,
    name: &str,
    based_on: Option<&str>,
    i: usize,
) -> Result<(), Error> {
    if find_style(root, style_id).is_some() {
        return Err(Error::with_path(
            ErrorKind::StyleIdTaken,
            format!("ops[{i}].styleId"),
        ));
    }
    let inherit = based_on
        .map(str::to_string)
        .or_else(|| default_style_id_of_type(root, ty));
    let mut style = Element::new(root.prefixed("style"));
    let type_key = style.prefixed("type");
    let id_key = style.prefixed("styleId");
    let custom_key = style.prefixed("customStyle");
    style.set_attr(&type_key, ty);
    style.set_attr(&id_key, style_id);
    style.set_attr(&custom_key, "1");
    let mut name_el = Element::new(style.prefixed("name"));
    let name_val = name_el.prefixed("val");
    name_el.set_attr(&name_val, name);
    style.push_element(name_el);
    if let Some(base) = inherit {
        if base != style_id {
            let mut based = Element::new(style.prefixed("basedOn"));
            let based_val = based.prefixed("val");
            based.set_attr(&based_val, base);
            style.push_element(based);
        }
    }
    if ty == "table" {
        style.push_element(Element::new(style.prefixed("tblPr")));
    }
    root.push_element(style);
    Ok(())
}

pub fn delete_style(root: &mut Element, style_id: &str, id: &str, i: usize) -> Result<(), Error> {
    let Some(idx) = root.children.iter().position(|n| {
        matches!(
            n,
            Node::Element(e)
                if e.local_name() == "style" && e.attr_local("styleId") == Some(style_id)
        )
    }) else {
        return Err(missing_style(id, i));
    };
    let Node::Element(el) = &root.children[idx] else {
        return Err(missing_style(id, i));
    };
    if is_default_style(el) || !is_custom_style(el) {
        return Err(Error::with_path(
            ErrorKind::StyleNotCustom,
            format!("ops[{i}].id"),
        ));
    }
    root.children.remove(idx);
    Ok(())
}

fn collect_style_els(root: &Element) -> Vec<&Element> {
    root.children_by_local("style")
        .into_iter()
        .filter(|s| s.attr_local("styleId").is_some_and(|id| !id.is_empty()))
        .collect()
}

fn ensure_ppr(p: &mut Element) {
    if p.child_by_local("pPr").is_some() {
        return;
    }
    let ppr = Element::new(p.prefixed("pPr"));
    p.children.insert(0, Node::Element(ppr));
}

fn ensure_style_ppr(style: &mut Element) {
    if style.child_by_local("pPr").is_some() {
        return;
    }
    let ppr = Element::new(style.prefixed("pPr"));
    let at = style
        .children
        .iter()
        .position(|n| matches!(n, Node::Element(e) if e.local_name() == "rPr"));
    if let Some(i) = at {
        style.children.insert(i, Node::Element(ppr));
    } else {
        style.push_element(ppr);
    }
}

fn ensure_style_rpr(style: &mut Element) {
    if style.child_by_local("rPr").is_some() {
        return;
    }
    let rpr = Element::new(style.prefixed("rPr"));
    let at = style
        .children
        .iter()
        .position(|n| matches!(n, Node::Element(e) if e.local_name() == "pPr"));
    if let Some(i) = at {
        style.children.insert(i + 1, Node::Element(rpr));
    } else {
        style.push_element(rpr);
    }
}

fn ensure_child(parent: &mut Element, local: &str) {
    if parent.child_by_local(local).is_some() {
        return;
    }
    parent.push_element(Element::new(parent.prefixed(local)));
}

fn apply_fonts(rpr: &mut Element, font: Option<&str>, east_asia: Option<&str>) {
    ensure_child(rpr, "rFonts");
    let fonts = rpr.child_by_local_mut("rFonts").expect("刚写入 rFonts");
    if let Some(name) = font {
        let ascii = fonts.prefixed("ascii");
        let h_ansi = fonts.prefixed("hAnsi");
        let cs = fonts.prefixed("cs");
        fonts.set_attr(&ascii, name);
        fonts.set_attr(&h_ansi, name);
        fonts.set_attr(&cs, name);
    }
    if let Some(name) = east_asia {
        let key = fonts.prefixed("eastAsia");
        fonts.set_attr(&key, name);
    }
}

fn set_sz(rpr: &mut Element, sz: u32) {
    for local in ["sz", "szCs"] {
        ensure_child(rpr, local);
        let el = rpr.child_by_local_mut(local).expect("刚写入 sz");
        let key = el.prefixed("val");
        el.set_attr(&key, sz.to_string());
    }
}

fn set_bold(rpr: &mut Element, bold: bool) {
    for local in ["b", "bCs"] {
        ensure_child(rpr, local);
        let el = rpr.child_by_local_mut(local).expect("刚写入 b");
        let key = el.prefixed("val");
        if bold {
            el.remove_attr(&key);
        } else {
            el.set_attr(&key, "0");
        }
    }
}

fn set_color(rpr: &mut Element, hex: &str) {
    ensure_child(rpr, "color");
    let el = rpr.child_by_local_mut("color").expect("刚写入 color");
    el.attrs.retain(|(k, _)| {
        let local = k.rsplit(':').next().unwrap_or(k);
        !matches!(local, "themeColor" | "themeTint" | "themeShade" | "theme")
    });
    let key = el.prefixed("val");
    el.set_attr(&key, hex.to_ascii_uppercase());
}

use super::format;
use super::hf;
use super::request::DeleteStyleOp;
use super::request::InsertStyleOp;
use super::request::SetParaStyleOp;
use super::request::SetRunStyleOp;
use super::request::SetStyleOp;
use super::request::SetTableStyleOp;
use super::tbl;
use std::collections::HashMap;
// 从 apply 搬来的操作体。
pub(crate) fn apply_set_style(
    pkg: &Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    op: &SetStyleOp,
    i: usize,
) -> Result<(), Error> {
    let Some(part) = super::styles::styles_part(pkg, doc_part)? else {
        return Err(super::styles::missing_style(&op.id, i));
    };
    let doc = super::apply::load_part(pkg, docs, &part)?;
    super::styles::set_style_props(
        &mut doc.root,
        &op.style_id,
        super::styles::StylePatch {
            font: op.font.as_deref(),
            east_asia_font: op.east_asia_font.as_deref(),
            sz: op.sz,
            bold: op.bold,
            color: op.color.as_deref(),
            before: op.before,
            after: op.after,
        },
        &op.id,
        i,
    )?;
    dirty.insert(part, ());
    Ok(())
}
pub(crate) fn apply_set_para_style(
    pkg: &Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &SetParaStyleOp,
    i: usize,
) -> Result<(), Error> {
    if let Some(style_id) = op.style.as_deref() {
        let style_part = super::styles::styles_part(pkg, doc_part)?
            .ok_or_else(|| super::styles::missing_para_style(&format!("style:{style_id}"), i))?;
        {
            let style_doc = super::apply::load_part(pkg, docs, &style_part)?;
            let Some(el) = super::styles::find_style(&style_doc.root, style_id) else {
                return Err(super::styles::missing_para_style(
                    &format!("style:{style_id}"),
                    i,
                ));
            };
            if !super::styles::is_paragraph_style(el) {
                return Err(Error::with_path(
                    ErrorKind::SetStyleNeedsParagraph,
                    format!("ops[{i}].style"),
                ));
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
    match op.style.as_deref() {
        Some(s) => super::styles::bind_para(el, s),
        None => super::styles::unbind_para(el),
    }
    dirty.insert(part_name, ());
    Ok(())
}
pub(crate) fn apply_insert_style(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    op: &InsertStyleOp,
    i: usize,
) -> Result<(), Error> {
    let part = super::styles::ensure_part(pkg, docs, dirty, doc_part)?;
    let doc = super::apply::load_part(pkg, docs, &part)?;
    super::styles::insert_style(
        &mut doc.root,
        &op.style_id,
        &op.ty,
        &op.name,
        op.based_on.as_deref(),
        i,
    )?;
    dirty.insert(part, ());
    Ok(())
}
pub(crate) fn apply_delete_style(
    pkg: &Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    op: &DeleteStyleOp,
    i: usize,
) -> Result<(), Error> {
    let Some(part) = super::styles::styles_part(pkg, doc_part)? else {
        return Err(super::styles::missing_style(&op.id, i));
    };
    let doc = super::apply::load_part(pkg, docs, &part)?;
    super::styles::delete_style(&mut doc.root, &op.style_id, &op.id, i)?;
    dirty.insert(part, ());
    Ok(())
}
pub(crate) fn apply_set_run_style(
    pkg: &Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &SetRunStyleOp,
    i: usize,
) -> Result<(), Error> {
    if let Some(style_id) = op.style.as_deref() {
        let style_part = super::styles::styles_part(pkg, doc_part)?
            .ok_or_else(|| super::styles::missing_para_style(&format!("style:{style_id}"), i))?;
        {
            let style_doc = super::apply::load_part(pkg, docs, &style_part)?;
            let Some(el) = super::styles::find_style(&style_doc.root, style_id) else {
                return Err(super::styles::missing_para_style(
                    &format!("style:{style_id}"),
                    i,
                ));
            };
            if !super::styles::is_character_style(el) {
                return Err(Error::with_path(
                    ErrorKind::SetRunStyleNeedsCharacter,
                    format!("ops[{i}].style"),
                ));
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
    let run = format::find_run_mut(el, op.run).ok_or_else(|| format::missing_run(&op.id, i))?;
    match op.style.as_deref() {
        Some(s) => format::bind_run_style(run, s),
        None => format::unbind_run_style(run),
    }
    dirty.insert(part_name, ());
    Ok(())
}
pub(crate) fn apply_set_table_style(
    pkg: &Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &SetTableStyleOp,
    i: usize,
) -> Result<(), Error> {
    if let Some(Some(style_id)) = op.style.as_ref() {
        let style_part = super::styles::styles_part(pkg, doc_part)?
            .ok_or_else(|| super::styles::missing_para_style(&format!("style:{style_id}"), i))?;
        {
            let style_doc = super::apply::load_part(pkg, docs, &style_part)?;
            let Some(el) = super::styles::find_style(&style_doc.root, style_id) else {
                return Err(super::styles::missing_para_style(
                    &format!("style:{style_id}"),
                    i,
                ));
            };
            if !super::styles::is_table_style(el) {
                return Err(Error::with_path(
                    ErrorKind::SetTableStyleNeedsTable,
                    format!("ops[{i}].style"),
                ));
            }
        }
    }
    let part_name = tbl::part_for_story(catalog, doc_part, op.target.story(), &op.id, i)?;
    let doc = super::apply::load_part(pkg, docs, &part_name)?;
    tbl::apply_set_table_style(
        &mut doc.root,
        &op.target,
        op.style.as_ref().map(|s| s.as_deref()),
        op.look.as_ref().map(|l| l.as_ref()),
        &op.id,
        i,
    )?;
    dirty.insert(part_name, ());
    Ok(())
}
