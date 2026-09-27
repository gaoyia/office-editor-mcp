//! 节上的页面边距、纸张、分栏、对齐、行号、网格与装订线。

use serde_json::{json, Value};

use crate::error::{Error, XmlIssue};
use crate::opc::Package;
use crate::xml::{Element, Node};

use super::hf;
use super::notes;

/// `w:cols/@w:equalWidth` 一类 OnOff 属性。
pub fn on_off_attr(el: &Element, local: &str, elem: &str) -> Result<Option<bool>, Error> {
    let Some(raw) = el.attr_local(local) else {
        return Ok(None);
    };
    Ok(Some(parse_on_off(raw, elem, local)?))
}

fn parse_on_off(raw: &str, elem: &str, local: &str) -> Result<bool, Error> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "on" => Ok(true),
        "0" | "false" | "off" => Ok(false),
        _ => Err(Error::xml_detail(
            XmlIssue::Parse,
            format!("{elem}/@{local} is not OnOff: {raw}"),
        )),
    }
}

fn enum_attr<'a>(
    el: &'a Element,
    local: &str,
    elem: &str,
    allowed: &[&str],
) -> Result<Option<&'a str>, Error> {
    let Some(raw) = el.attr_local(local) else {
        return Ok(None);
    };
    let t = raw.trim();
    if allowed.contains(&t) {
        return Ok(Some(t));
    }
    Err(Error::xml_detail(
        XmlIssue::Parse,
        format!("{elem}/@{local} not recognized: {raw}"),
    ))
}

fn write_on_off_attr(el: &mut Element, local: &str, on: bool) {
    hf::set_local_attr(el, local, if on { "1" } else { "0" });
}

/// 根上的整篇镜像边距、装订线在上。缺 settings 或缺元素为 `None`。
pub fn document_flags(
    pkg: &Package,
    doc_part: &str,
) -> Result<(Option<bool>, Option<bool>), Error> {
    let Some(name) = notes::settings_part_name(pkg, doc_part)? else {
        return Ok((None, None));
    };
    let Ok(bytes) = pkg.get(&name) else {
        return Ok((None, None));
    };
    let doc = crate::xml::parse(bytes)?;
    Ok((
        hf::on_off_opt(&doc.root, "mirrorMargins"),
        hf::on_off_opt(&doc.root, "gutterAtTop"),
    ))
}

pub fn orient_value(sz: Option<&Element>) -> Result<Value, Error> {
    let Some(el) = sz else {
        return Ok(Value::Null);
    };
    match enum_attr(el, "orient", "pgSz", &["portrait", "landscape"])? {
        Some(v) => Ok(json!(v)),
        None => Ok(Value::Null),
    }
}

pub fn columns_value(cols: Option<&Element>) -> Result<Value, Error> {
    let Some(el) = cols else {
        return Ok(Value::Null);
    };
    let mut col_items = Vec::new();
    for c in el.children_by_local("col") {
        col_items.push(json!({
            "width": hf::measure_attr(Some(c), "w", "col", false)?,
            "space": hf::measure_attr(Some(c), "space", "col", false)?,
        }));
    }
    Ok(json!({
        "num": hf::measure_attr(Some(el), "num", "cols", false)?,
        "space": hf::measure_attr(Some(el), "space", "cols", false)?,
        "equalWidth": on_off_attr(el, "equalWidth", "cols")?,
        "sep": on_off_attr(el, "sep", "cols")?,
        "cols": col_items,
    }))
}

pub fn valign_value(el: Option<&Element>) -> Result<Value, Error> {
    let Some(el) = el else {
        return Ok(Value::Null);
    };
    let Some(v) = enum_attr(el, "val", "vAlign", &["top", "center", "both", "bottom"])? else {
        return Err(Error::xml_detail(XmlIssue::Parse, "vAlign missing @val"));
    };
    Ok(json!(v))
}

pub fn line_numbering_value(el: Option<&Element>) -> Result<Value, Error> {
    let Some(el) = el else {
        return Ok(Value::Null);
    };
    Ok(json!({
        "countBy": hf::measure_attr(Some(el), "countBy", "lnNumType", true)?,
        "start": hf::measure_attr(Some(el), "start", "lnNumType", true)?,
        "distance": hf::measure_attr(Some(el), "distance", "lnNumType", false)?,
        "restart": enum_attr(el, "restart", "lnNumType", &["newPage", "newSection", "continuous"])?,
    }))
}

pub fn doc_grid_value(el: Option<&Element>) -> Result<Value, Error> {
    let Some(el) = el else {
        return Ok(Value::Null);
    };
    Ok(json!({
        "type": enum_attr(
            el,
            "type",
            "docGrid",
            &["default", "lines", "linesAndChars", "snapToChars"],
        )?,
        "linePitch": hf::measure_attr(Some(el), "linePitch", "docGrid", true)?,
        "charSpace": hf::measure_attr(Some(el), "charSpace", "docGrid", true)?,
    }))
}

pub fn rtl_gutter_value(sect: &Element) -> Value {
    match hf::on_off_opt(sect, "rtlGutter") {
        Some(b) => json!(b),
        None => Value::Null,
    }
}

pub fn apply_page_margin(
    sect: &mut Element,
    top: Option<i64>,
    bottom: Option<i64>,
    left: Option<u32>,
    right: Option<u32>,
    gutter: Option<u32>,
) {
    let pg = hf::ensure_child_in_order(sect, "pgMar", hf::TITLE_PG_ORDER);
    if let Some(v) = top {
        hf::set_signed_twip_attr(pg, "top", v);
    }
    if let Some(v) = bottom {
        hf::set_signed_twip_attr(pg, "bottom", v);
    }
    if let Some(v) = left {
        hf::set_twip_attr(pg, "left", v);
    }
    if let Some(v) = right {
        hf::set_twip_attr(pg, "right", v);
    }
    if let Some(v) = gutter {
        hf::set_twip_attr(pg, "gutter", v);
    }
}

pub fn apply_page_size(
    sect: &mut Element,
    width: Option<u32>,
    height: Option<u32>,
    orient: Option<&str>,
) {
    let sz = hf::ensure_child_in_order(sect, "pgSz", hf::TITLE_PG_ORDER);
    if let Some(v) = width {
        hf::set_twip_attr(sz, "w", v);
    }
    if let Some(v) = height {
        hf::set_twip_attr(sz, "h", v);
    }
    if let Some(v) = orient {
        hf::set_local_attr(sz, "orient", v);
    }
}

pub struct ColItem {
    pub width: Option<u32>,
    pub space: Option<u32>,
}

pub fn apply_columns(
    sect: &mut Element,
    num: Option<u32>,
    space: Option<u32>,
    equal_width: Option<bool>,
    sep: Option<bool>,
    cols: Option<&[ColItem]>,
    enabled: Option<bool>,
) {
    if enabled == Some(false) {
        hf::remove_local(sect, "cols");
        return;
    }
    let el = hf::ensure_child_in_order(sect, "cols", hf::TITLE_PG_ORDER);
    if let Some(v) = num {
        hf::set_local_attr(el, "num", v.to_string());
    }
    if let Some(v) = space {
        hf::set_twip_attr(el, "space", v);
    }
    if let Some(v) = equal_width {
        write_on_off_attr(el, "equalWidth", v);
    }
    if let Some(v) = sep {
        write_on_off_attr(el, "sep", v);
    }
    if let Some(items) = cols {
        el.children.retain(|n| match n {
            Node::Element(e) => e.local_name() != "col",
            _ => true,
        });
        for item in items {
            let mut col = Element::new(el.prefixed("col"));
            if let Some(w) = item.width {
                hf::set_twip_attr(&mut col, "w", w);
            }
            if let Some(s) = item.space {
                hf::set_twip_attr(&mut col, "space", s);
            }
            el.push_element(col);
        }
    }
}

pub fn apply_valign(sect: &mut Element, v_align: Option<&str>) {
    match v_align {
        None => hf::remove_local(sect, "vAlign"),
        Some(v) => {
            let el = hf::ensure_child_in_order(sect, "vAlign", hf::TITLE_PG_ORDER);
            hf::set_local_attr(el, "val", v);
        }
    }
}

pub fn apply_line_numbering(
    sect: &mut Element,
    count_by: Option<u32>,
    start: Option<u32>,
    distance: Option<u32>,
    restart: Option<&str>,
    enabled: Option<bool>,
) {
    if enabled == Some(false) {
        hf::remove_local(sect, "lnNumType");
        return;
    }
    let el = hf::ensure_child_in_order(sect, "lnNumType", hf::TITLE_PG_ORDER);
    if let Some(v) = count_by {
        hf::set_local_attr(el, "countBy", v.to_string());
    }
    if let Some(v) = start {
        hf::set_local_attr(el, "start", v.to_string());
    }
    if let Some(v) = distance {
        hf::set_twip_attr(el, "distance", v);
    }
    if let Some(v) = restart {
        hf::set_local_attr(el, "restart", v);
    }
}

pub fn apply_doc_grid(
    sect: &mut Element,
    grid_type: Option<&str>,
    line_pitch: Option<i64>,
    char_space: Option<i64>,
    enabled: Option<bool>,
) {
    if enabled == Some(false) {
        hf::remove_local(sect, "docGrid");
        return;
    }
    let el = hf::ensure_child_in_order(sect, "docGrid", hf::TITLE_PG_ORDER);
    if let Some(v) = grid_type {
        hf::set_local_attr(el, "type", v);
    }
    if let Some(v) = line_pitch {
        hf::set_signed_twip_attr(el, "linePitch", v);
    }
    if let Some(v) = char_space {
        hf::set_signed_twip_attr(el, "charSpace", v);
    }
}

pub fn apply_rtl_gutter(sect: &mut Element, on: bool) {
    hf::set_on_off(sect, "rtlGutter", on, hf::TITLE_PG_ORDER);
}

pub fn apply_settings_flag(root: &mut Element, local: &str, on: bool) {
    hf::set_on_off(root, local, on, hf::SETTINGS_ORDER);
}
