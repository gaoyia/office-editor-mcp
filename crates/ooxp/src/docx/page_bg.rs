//! 整篇页面背景与节上页面边框。
//! 不实现排版引擎，不写 DrawingML 渐变 / 艺术型边框。

use std::collections::HashMap;

use serde_json::{json, Value};

use crate::error::Error;
use crate::opc::{ContentTypes, Package, Relationship, Relationships};
use crate::xml::{self, Document, Element, Node};

use super::hf;
use super::notes;
use super::picture;
use super::{CT_SETTINGS, REL_SETTINGS, WML_NS};

const BORDER_VALS: [&str; 11] = [
    "nil",
    "none",
    "single",
    "dashed",
    "dotted",
    "double",
    "thick",
    "wave",
    "dashSmallGap",
    "dotDash",
    "dotDotDash",
];

#[derive(Debug, Clone)]
pub struct BorderSide {
    pub val: String,
    pub sz: Option<u32>,
    pub space: Option<u32>,
    pub color: Option<String>,
}

pub fn background_json(pkg: &Package, doc_part: &str, root: &Element) -> Result<Value, Error> {
    let color = root
        .child_by_local("background")
        .and_then(|e| e.attr_local("color"))
        .map_or(Value::Null, |c| json!(c));
    Ok(json!({
        "color": color,
        "display": read_display(pkg, doc_part)?,
    }))
}

pub fn borders_json(sect: &Element) -> Value {
    let Some(el) = sect.child_by_local("pgBorders") else {
        return Value::Null;
    };
    json!({
        "offsetFrom": attr_or_null(el, "offsetFrom"),
        "display": attr_or_null(el, "display"),
        "zOrder": attr_or_null(el, "zOrder"),
        "top": side_json(el, "top"),
        "left": side_json(el, "left"),
        "bottom": side_json(el, "bottom"),
        "right": side_json(el, "right"),
    })
}

pub fn set_background(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    color: Option<&str>,
) -> Result<(), Error> {
    {
        let doc = docs.get_mut(doc_part).expect("apply 已载入主文档");
        match color {
            None => hf::remove_local(&mut doc.root, "background"),
            Some(c) => {
                let bg = ensure_background(&mut doc.root);
                hf::set_local_attr(bg, "color", c);
            }
        }
        dirty.insert(doc_part.to_string(), ());
    }
    set_display(pkg, docs, dirty, doc_part, color.is_some())?;
    Ok(())
}

pub fn set_borders(
    sect: &mut Element,
    enabled: Option<bool>,
    offset_from: Option<&str>,
    display: Option<&str>,
    z_order: Option<&str>,
    top: Option<Option<&BorderSide>>,
    left: Option<Option<&BorderSide>>,
    bottom: Option<Option<&BorderSide>>,
    right: Option<Option<&BorderSide>>,
) {
    if enabled == Some(false) {
        hf::remove_local(sect, "pgBorders");
        return;
    }
    let el = hf::ensure_child_in_order(sect, "pgBorders", hf::TITLE_PG_ORDER);
    if let Some(v) = offset_from {
        hf::set_local_attr(el, "offsetFrom", v);
    }
    if let Some(v) = display {
        hf::set_local_attr(el, "display", v);
    }
    if let Some(v) = z_order {
        hf::set_local_attr(el, "zOrder", v);
    }
    write_side(el, "top", top);
    write_side(el, "left", left);
    write_side(el, "bottom", bottom);
    write_side(el, "right", right);
}

fn attr_or_null(el: &Element, local: &str) -> Value {
    el.attr_local(local).map_or(Value::Null, |s| json!(s))
}

fn side_json(parent: &Element, local: &str) -> Value {
    let Some(el) = parent.child_by_local(local) else {
        return Value::Null;
    };
    json!({
        "val": el.attr_local("val").unwrap_or(""),
        "sz": parse_u32_attr(el, "sz"),
        "space": parse_u32_attr(el, "space"),
        "color": el.attr_local("color").map_or(Value::Null, |c| json!(c)),
    })
}

fn parse_u32_attr(el: &Element, local: &str) -> Value {
    el.attr_local(local)
        .and_then(|s| s.parse::<u32>().ok())
        .map_or(Value::Null, |n| json!(n))
}

fn write_side(parent: &mut Element, local: &str, side: Option<Option<&BorderSide>>) {
    let Some(slot) = side else {
        return;
    };
    match slot {
        None => {
            parent.children.retain(|n| match n {
                Node::Element(e) => e.local_name() != local,
                _ => true,
            });
        }
        Some(b) => {
            let has = parent.child_by_local(local).is_some();
            if !has {
                let el = Element::new(parent.prefixed(local));
                parent.push_element(el);
            }
            let el = parent
                .element_children_mut()
                .find(|e| e.local_name() == local)
                .expect("边框子元素");
            hf::set_local_attr(el, "val", &b.val);
            if let Some(sz) = b.sz {
                hf::set_local_attr(el, "sz", sz.to_string());
            }
            if let Some(space) = b.space {
                hf::set_local_attr(el, "space", space.to_string());
            }
            if let Some(color) = &b.color {
                hf::set_local_attr(el, "color", color);
            }
        }
    }
}

fn ensure_background(root: &mut Element) -> &mut Element {
    if root.child_by_local("background").is_some() {
        return root.child_by_local_mut("background").unwrap();
    }
    let el = Element::new(root.prefixed("background"));
    let dest = root
        .children
        .iter()
        .position(|n| matches!(n, Node::Element(e) if e.local_name() == "body"))
        .unwrap_or(root.children.len());
    root.children.insert(dest, Node::Element(el));
    root.child_by_local_mut("background").unwrap()
}

fn read_display(pkg: &Package, doc_part: &str) -> Result<bool, Error> {
    let Some(settings) = notes::settings_part_name(pkg, doc_part)? else {
        return Ok(false);
    };
    let Ok(bytes) = pkg.get(&settings) else {
        return Ok(false);
    };
    let doc = xml::parse(bytes)?;
    Ok(hf::bool_prop(&doc.root, "displayBackgroundShape"))
}

fn set_display(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    on: bool,
) -> Result<(), Error> {
    let settings = match notes::settings_part_name(pkg, doc_part)? {
        Some(p) => p,
        None if on => ensure_settings(pkg, docs, dirty, doc_part)?,
        None => return Ok(()),
    };
    if !docs.contains_key(&settings) {
        let parsed = xml::parse(pkg.get(&settings)?)?;
        docs.insert(settings.clone(), parsed);
    }
    let doc = docs.get_mut(&settings).expect("已载入 settings");
    hf::set_on_off(
        &mut doc.root,
        "displayBackgroundShape",
        on,
        hf::SETTINGS_ORDER,
    );
    dirty.insert(settings, ());
    Ok(())
}

pub(crate) fn ensure_settings(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
) -> Result<String, Error> {
    let part = "/word/settings.xml".to_string();
    if !pkg.contains(&part) {
        pkg.set(
            &part,
            format!(
                r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:settings xmlns:w="{WML_NS}"><w:compat><w:compatSetting w:name="compatibilityMode" w:uri="http://schemas.microsoft.com/office/word" w:val="15"/></w:compat></w:settings>"#
            )
            .into_bytes(),
        )?;
    }
    if !docs.contains_key(&part) {
        let parsed = xml::parse(pkg.get(&part)?)?;
        docs.insert(part.clone(), parsed);
    }
    dirty.insert(part.clone(), ());
    let mut doc_rels = Relationships::from_package(pkg, doc_part)?;
    if doc_rels.find_by_type_suffix("/settings").is_empty() {
        doc_rels.push(Relationship {
            id: doc_rels.next_rid(),
            rel_type: REL_SETTINGS.into(),
            target: picture::rel_target_from(doc_part, &part),
            target_mode: None,
            extra: Vec::new(),
        });
        doc_rels.write_to(pkg, doc_part)?;
    }
    let mut cts = ContentTypes::from_package(pkg)?;
    cts.set_override(&part, CT_SETTINGS)?;
    cts.write_to(pkg)?;
    Ok(part)
}

pub fn parse_color(raw: &str) -> Option<String> {
    if raw.len() != 6 || !raw.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    Some(raw.to_ascii_uppercase())
}

pub fn parse_border_val(raw: &str) -> Option<&str> {
    BORDER_VALS.iter().copied().find(|&v| v == raw)
}

pub fn parse_side_color(raw: &str) -> Option<String> {
    if raw.eq_ignore_ascii_case("auto") {
        return Some("auto".into());
    }
    parse_color(raw)
}
