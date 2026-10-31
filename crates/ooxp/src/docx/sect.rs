//! 节上的页面边距、纸张、分栏、对齐、行号、网格与装订线；插删分节符。

use serde_json::{json, Value};

use crate::error::{Error, ErrorKind, IdReason, XmlIssue};
use crate::opc::Package;
use crate::xml::{Element, Node};

use super::hf;
use super::notes;
use super::para::{self, BodyPara};

/// `ST_SectionMark`（ECMA-376 §17.18.77）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SectionBreakType {
    NextPage,
    Continuous,
    EvenPage,
    OddPage,
    NextColumn,
}

impl SectionBreakType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NextPage => "nextPage",
            Self::Continuous => "continuous",
            Self::EvenPage => "evenPage",
            Self::OddPage => "oddPage",
            Self::NextColumn => "nextColumn",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "nextPage" => Some(Self::NextPage),
            "continuous" => Some(Self::Continuous),
            "evenPage" => Some(Self::EvenPage),
            "oddPage" => Some(Self::OddPage),
            "nextColumn" => Some(Self::NextColumn),
            _ => None,
        }
    }
}

pub fn parse_section_break_type(raw: &str) -> Result<SectionBreakType, ErrorKind> {
    SectionBreakType::parse(raw).ok_or(ErrorKind::InvalidSectionType)
}

/// 在点名段之后插入分节符；`after` 为 `None` 表示文档末尾。
pub fn insert_section_break(
    root: &mut Element,
    after: Option<&BodyPara>,
    ty: SectionBreakType,
    err_id: &str,
    op_i: usize,
) -> Result<(), Error> {
    match after {
        Some(para) => insert_after_para(root, para, ty, err_id, op_i),
        None => insert_at_end(root, ty, err_id, op_i),
    }
}

fn insert_after_para(
    root: &mut Element,
    para: &BodyPara,
    ty: SectionBreakType,
    err_id: &str,
    op_i: usize,
) -> Result<(), Error> {
    let path = para::find_paragraph_path(root, para).ok_or_else(|| {
        Error::with_path(
            ErrorKind::ParaNotFound {
                id: err_id.to_string(),
            },
            format!("ops[{op_i}].id"),
        )
    })?;
    let parent =
        para::element_at(root, &path[..path.len().saturating_sub(1)]).ok_or_else(|| {
            Error::with_path(
                ErrorKind::ParaNotFound {
                    id: err_id.to_string(),
                },
                format!("ops[{op_i}].id"),
            )
        })?;
    if parent.local_name() != "body" {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::InsertSectionNeedsBodyPara,
            },
            format!("ops[{op_i}].id"),
        ));
    }

    let dest_path = dest_sect_path(root, &path)
        .ok_or_else(|| Error::xml_detail(XmlIssue::Parse, "document body has no sectPr"))?;
    let dest_on_this = dest_path.starts_with(&path) && dest_path.len() > path.len();

    if dest_on_this {
        let dest = para::element_at(root, &dest_path).expect("dest path");
        let mut clone = dest.clone();
        crate::xml::mark_new(&mut clone, "section");
        hf::strip_hf_refs(&mut clone);
        set_section_type(&mut clone, ty);
        insert_empty_after(root, &path, clone, err_id, op_i)?;
    } else {
        let dest = para::element_at(root, &dest_path).expect("dest path");
        let mut clone = dest.clone();
        crate::xml::mark_new(&mut clone, "section");
        set_section_type(&mut clone, ty);
        let p = para::element_at_mut(root, &path).expect("para path");
        para::put_ppr_sect_pr(p, clone);
        let dest = para::element_at_mut(root, &dest_path).expect("dest path");
        hf::strip_hf_refs(dest);
        if is_body_level_sect(root, &dest_path) && !has_block_between(root, &path, &dest_path) {
            insert_empty_before_dest(root, &dest_path)?;
        }
    }
    Ok(())
}

fn insert_at_end(
    root: &mut Element,
    ty: SectionBreakType,
    _err_id: &str,
    op_i: usize,
) -> Result<(), Error> {
    let (sect_idx, mut clone, prev_plain) = {
        let body = para::story_body_mut(root).ok_or_else(|| {
            Error::with_path(
                ErrorKind::XmlMissing {
                    parent: "document".into(),
                    child: "body".into(),
                },
                format!("ops[{op_i}].id"),
            )
        })?;
        let sect_idx = body
            .children
            .iter()
            .position(|n| matches!(n, Node::Element(e) if e.local_name() == "sectPr"))
            .ok_or_else(|| Error::xml_detail(XmlIssue::Parse, "document body has no sectPr"))?;
        let dest = match &body.children[sect_idx] {
            Node::Element(e) => e.clone(),
            _ => unreachable!(),
        };
        let prev_plain = sect_idx >= 1
            && matches!(
                &body.children[sect_idx - 1],
                Node::Element(e) if e.local_name() == "p" && !para::para_has_sect_pr(e)
            );
        (sect_idx, dest, prev_plain)
    };
    crate::xml::mark_new(&mut clone, "section");
    set_section_type(&mut clone, ty);

    let empty_id = para::next_para_hex(root);
    {
        let body = para::story_body_mut(root).unwrap();
        let empty = para::new_paragraph(body, &empty_id, "");
        body.children.insert(sect_idx, Node::Element(empty));
    }

    if prev_plain {
        let body = para::story_body_mut(root).unwrap();
        let p = match &mut body.children[sect_idx - 1] {
            Node::Element(e) => e,
            _ => unreachable!(),
        };
        para::put_ppr_sect_pr(p, clone);
    } else {
        let mid_id = para::next_para_hex(root);
        let body = para::story_body_mut(root).unwrap();
        let mut mid = para::new_paragraph(body, &mid_id, "");
        para::put_ppr_sect_pr(&mut mid, clone);
        let cur_sect = body
            .children
            .iter()
            .position(|n| matches!(n, Node::Element(e) if e.local_name() == "sectPr"))
            .unwrap();
        body.children.insert(cur_sect - 1, Node::Element(mid));
    }

    let body = para::story_body_mut(root).unwrap();
    let cur_sect = body
        .children
        .iter()
        .position(|n| matches!(n, Node::Element(e) if e.local_name() == "sectPr"))
        .unwrap();
    if let Node::Element(sect) = &mut body.children[cur_sect] {
        hf::strip_hf_refs(sect);
    }
    Ok(())
}

fn dest_sect_path(root: &Element, para_path: &[usize]) -> Option<Vec<usize>> {
    hf::collect_sect_pr_paths(root)
        .into_iter()
        .find(|p| p.as_slice() >= para_path)
}

fn is_body_level_sect(root: &Element, dest_path: &[usize]) -> bool {
    if dest_path.is_empty() {
        return false;
    }
    para::element_at(root, &dest_path[..dest_path.len() - 1])
        .is_some_and(|p| p.local_name() == "body")
}

fn has_block_between(root: &Element, para_path: &[usize], dest_path: &[usize]) -> bool {
    if para_path.len() != dest_path.len() {
        return true;
    }
    if para_path.len() < 2 {
        return true;
    }
    if para_path[..para_path.len() - 1] != dest_path[..dest_path.len() - 1] {
        return true;
    }
    let parent = match para::element_at(root, &para_path[..para_path.len() - 1]) {
        Some(p) => p,
        None => return true,
    };
    let i = *para_path.last().unwrap();
    let j = *dest_path.last().unwrap();
    if j <= i + 1 {
        return false;
    }
    parent.children[i + 1..j]
        .iter()
        .any(|n| matches!(n, Node::Element(e) if e.local_name() == "p" || e.local_name() == "tbl"))
}

fn insert_empty_after(
    root: &mut Element,
    para_path: &[usize],
    sect: Element,
    err_id: &str,
    op_i: usize,
) -> Result<(), Error> {
    let new_id = para::next_para_hex(root);
    let host = para::element_at(root, para_path).ok_or_else(|| {
        Error::with_path(
            ErrorKind::ParaNotFound {
                id: err_id.to_string(),
            },
            format!("ops[{op_i}].id"),
        )
    })?;
    let mut new_p = para::new_paragraph(host, &new_id, "");
    para::put_ppr_sect_pr(&mut new_p, sect);
    let (parent, idx) = para::parent_and_index_mut(root, para_path).ok_or_else(|| {
        Error::with_path(
            ErrorKind::ParaNotFound {
                id: err_id.to_string(),
            },
            format!("ops[{op_i}].id"),
        )
    })?;
    parent.children.insert(idx + 1, Node::Element(new_p));
    Ok(())
}

fn insert_empty_before_dest(root: &mut Element, dest_path: &[usize]) -> Result<(), Error> {
    let new_id = para::next_para_hex(root);
    let parent_path = &dest_path[..dest_path.len() - 1];
    let parent = para::element_at(root, parent_path)
        .ok_or_else(|| Error::xml_detail(XmlIssue::Parse, "missing parent of body sectPr"))?;
    let new_p = para::new_paragraph(parent, &new_id, "");
    let (parent, idx) = para::parent_and_index_mut(root, dest_path)
        .ok_or_else(|| Error::xml_detail(XmlIssue::Parse, "missing body sectPr"))?;
    parent.children.insert(idx, Node::Element(new_p));
    Ok(())
}

fn set_section_type(sect: &mut Element, ty: SectionBreakType) {
    let el = hf::ensure_child_in_order(sect, "type", hf::TITLE_PG_ORDER);
    hf::set_local_attr(el, "val", ty.as_str());
}

/// 删掉结束第 `n` 节（从 1 计、不能是最后一节）的段上 `sectPr`。
pub fn delete_section_break(
    root: &mut Element,
    n: usize,
    err_id: &str,
    op_i: usize,
) -> Result<(), Error> {
    let paths = hf::collect_sect_pr_paths(root);
    if paths.len() < 2 {
        return Err(Error::with_path(
            ErrorKind::CannotDeleteLastSection,
            format!("ops[{op_i}].id"),
        ));
    }
    let Some(dest_path) = para::pick_collected(root, paths.clone(), n, "section", |p| p.as_slice())
    else {
        return Err(Error::with_path(
            ErrorKind::SectionNotFound {
                id: err_id.to_string(),
            },
            format!("ops[{op_i}].id"),
        ));
    };
    let Some(pos) = paths.iter().position(|p| p == &dest_path) else {
        return Err(Error::with_path(
            ErrorKind::SectionNotFound {
                id: err_id.to_string(),
            },
            format!("ops[{op_i}].id"),
        ));
    };
    if pos + 1 >= paths.len() {
        return Err(Error::with_path(
            ErrorKind::CannotDeleteLastSection,
            format!("ops[{op_i}].id"),
        ));
    }
    let dest_path = &paths[pos];
    if is_body_level_sect(root, dest_path) {
        return Err(Error::with_path(
            ErrorKind::CannotDeleteLastSection,
            format!("ops[{op_i}].id"),
        ));
    }
    let taken = {
        let dest = para::element_at(root, dest_path).ok_or_else(|| {
            Error::with_path(
                ErrorKind::SectionNotFound {
                    id: err_id.to_string(),
                },
                format!("ops[{op_i}].id"),
            )
        })?;
        dest.clone()
    };
    let next_path = paths[pos + 1].clone();
    let next = para::element_at_mut(root, &next_path).ok_or_else(|| {
        Error::with_path(
            ErrorKind::SectionNotFound {
                id: err_id.to_string(),
            },
            format!("ops[{op_i}].id"),
        )
    })?;
    hf::copy_missing_hf_refs(&taken, next);

    let p_path = para_path_of_sect(dest_path).ok_or_else(|| {
        Error::with_path(
            ErrorKind::CannotDeleteLastSection,
            format!("ops[{op_i}].id"),
        )
    })?;
    let p = para::element_at_mut(root, &p_path).ok_or_else(|| {
        Error::with_path(
            ErrorKind::SectionNotFound {
                id: err_id.to_string(),
            },
            format!("ops[{op_i}].id"),
        )
    })?;
    para::take_ppr_sect_pr(p);
    Ok(())
}

fn para_path_of_sect(sect_path: &[usize]) -> Option<Vec<usize>> {
    if sect_path.len() < 2 {
        return None;
    }
    Some(sect_path[..sect_path.len() - 2].to_vec())
}

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

pub fn page_margin(
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

pub fn page_size(
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

pub fn columns(
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

pub fn valign(sect: &mut Element, v_align: Option<&str>) {
    match v_align {
        None => hf::remove_local(sect, "vAlign"),
        Some(v) => {
            let el = hf::ensure_child_in_order(sect, "vAlign", hf::TITLE_PG_ORDER);
            hf::set_local_attr(el, "val", v);
        }
    }
}

pub fn line_numbering(
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

pub fn doc_grid(
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

pub fn rtl_gutter(sect: &mut Element, on: bool) {
    hf::set_on_off(sect, "rtlGutter", on, hf::TITLE_PG_ORDER);
}

pub fn settings_flag(root: &mut Element, local: &str, on: bool) {
    hf::set_on_off(root, local, on, hf::SETTINGS_ORDER);
}

use super::hf::HfKind;
use super::page_bg;
use super::picture;
use super::request::DeleteSectionOp;
use super::request::InsertSectionOp;
use super::request::InsertSectionPlace;
use super::request::SetColumnsOp;
use super::request::SetDocGridOp;
use super::request::SetFormProtOp;
use super::request::SetGutterAtTopOp;
use super::request::SetHfLinkOp;
use super::request::SetLineNumberingOp;
use super::request::SetMirrorMarginsOp;
use super::request::SetPageBordersOp;
use super::request::SetPageMarginOp;
use super::request::SetPageSizeOp;
use super::request::SetRtlGutterOp;
use super::request::SetVerticalAlignOp;
use super::CT_FOOTER;
use super::CT_HEADER;
use super::REL_FOOTER;
use super::REL_HEADER;
use crate::opc::rels_part_name;
use crate::opc::ContentTypes;
use crate::opc::{Relationship, Relationships};
use crate::xml::Document;
use std::collections::HashMap;
use std::collections::HashSet;
// 从 apply 搬来的操作体。
pub(crate) fn find_sect_for_apply<'a>(
    docs: &'a mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    n: usize,
    id: &str,
    i: usize,
) -> Result<&'a mut Element, Error> {
    let doc = docs.get_mut(doc_part).unwrap();
    let sect = hf::find_sect_pr_mut(&mut doc.root, n).ok_or_else(|| {
        Error::with_path(
            ErrorKind::SectionNotFound { id: id.to_string() },
            format!("ops[{i}].id"),
        )
    })?;
    dirty.insert(doc_part.to_string(), ());
    Ok(sect)
}
pub(crate) fn apply_page_borders(
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    op: &SetPageBordersOp,
    i: usize,
) -> Result<(), Error> {
    let sect = find_sect_for_apply(docs, dirty, doc_part, op.n, &op.id, i)?;
    page_bg::set_borders(
        sect,
        op.enabled,
        op.offset_from.as_deref(),
        op.display.as_deref(),
        op.z_order.as_deref(),
        op.top.as_ref().map(|s| s.as_ref()),
        op.left.as_ref().map(|s| s.as_ref()),
        op.bottom.as_ref().map(|s| s.as_ref()),
        op.right.as_ref().map(|s| s.as_ref()),
    );
    Ok(())
}
pub(crate) fn apply_set_form_prot(
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    op: &SetFormProtOp,
    i: usize,
) -> Result<(), Error> {
    let sect = find_sect_for_apply(docs, dirty, doc_part, op.n, &op.id, i)?;
    hf::set_form_prot(sect, op.form_prot);
    Ok(())
}
pub(crate) fn apply_page_margin(
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    op: &SetPageMarginOp,
    i: usize,
) -> Result<(), Error> {
    let sect = find_sect_for_apply(docs, dirty, doc_part, op.n, &op.id, i)?;
    super::sect::page_margin(sect, op.top, op.bottom, op.left, op.right, op.gutter);
    Ok(())
}
pub(crate) fn apply_page_size(
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    op: &SetPageSizeOp,
    i: usize,
) -> Result<(), Error> {
    let sect = find_sect_for_apply(docs, dirty, doc_part, op.n, &op.id, i)?;
    super::sect::page_size(sect, op.page_width, op.page_height, op.orient.as_deref());
    Ok(())
}
pub(crate) fn apply_columns(
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    op: &SetColumnsOp,
    i: usize,
) -> Result<(), Error> {
    let items: Option<Vec<super::sect::ColItem>> = op.cols.as_ref().map(|v| {
        v.iter()
            .map(|c| super::sect::ColItem {
                width: c.width,
                space: c.space,
            })
            .collect()
    });
    let sect = find_sect_for_apply(docs, dirty, doc_part, op.n, &op.id, i)?;
    super::sect::columns(
        sect,
        op.num,
        op.space,
        op.equal_width,
        op.sep,
        items.as_deref(),
        op.enabled,
    );
    Ok(())
}
pub(crate) fn apply_valign(
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    op: &SetVerticalAlignOp,
    i: usize,
) -> Result<(), Error> {
    let sect = find_sect_for_apply(docs, dirty, doc_part, op.n, &op.id, i)?;
    super::sect::valign(sect, op.v_align.as_deref());
    Ok(())
}
pub(crate) fn apply_line_numbering(
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    op: &SetLineNumberingOp,
    i: usize,
) -> Result<(), Error> {
    let sect = find_sect_for_apply(docs, dirty, doc_part, op.n, &op.id, i)?;
    super::sect::line_numbering(
        sect,
        op.count_by,
        op.start,
        op.distance,
        op.restart.as_deref(),
        op.enabled,
    );
    Ok(())
}
pub(crate) fn apply_doc_grid(
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    op: &SetDocGridOp,
    i: usize,
) -> Result<(), Error> {
    let sect = find_sect_for_apply(docs, dirty, doc_part, op.n, &op.id, i)?;
    super::sect::doc_grid(
        sect,
        op.grid_type.as_deref(),
        op.line_pitch,
        op.char_space,
        op.enabled,
    );
    Ok(())
}
pub(crate) fn apply_rtl_gutter(
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    op: &SetRtlGutterOp,
    i: usize,
) -> Result<(), Error> {
    let sect = find_sect_for_apply(docs, dirty, doc_part, op.n, &op.id, i)?;
    super::sect::rtl_gutter(sect, op.rtl_gutter);
    Ok(())
}
pub(crate) fn apply_mirror_margins(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    op: &SetMirrorMarginsOp,
    i: usize,
) -> Result<(), Error> {
    apply_settings_flag(
        pkg,
        docs,
        dirty,
        doc_part,
        "mirrorMargins",
        op.mirror_margins,
        i,
    )
}
pub(crate) fn apply_gutter_at_top(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    op: &SetGutterAtTopOp,
    i: usize,
) -> Result<(), Error> {
    apply_settings_flag(
        pkg,
        docs,
        dirty,
        doc_part,
        "gutterAtTop",
        op.gutter_at_top,
        i,
    )
}
pub(crate) fn apply_settings_flag(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    local: &str,
    on: bool,
    i: usize,
) -> Result<(), Error> {
    let settings = notes::settings_part_name(pkg, doc_part)?.ok_or_else(|| {
        Error::with_path(
            ErrorKind::PartMissing {
                name: "/word/settings.xml".into(),
            },
            format!("ops[{i}].id"),
        )
    })?;
    let doc = super::apply::load_part(pkg, docs, &settings)?;
    super::sect::settings_flag(&mut doc.root, local, on);
    dirty.insert(settings, ());
    Ok(())
}
pub(crate) fn unlink_section(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    op: &SetHfLinkOp,
) -> Result<(), Error> {
    let inherited = hf::inherited_part_path(
        pkg,
        doc_part,
        &docs.get(doc_part).unwrap().root,
        op.n,
        op.kind,
        op.hf_type,
    )?;
    let bytes = match inherited {
        Some(ref src) => super::apply::current_part_bytes(pkg, docs, src)?,
        None => hf::minimal_hf_xml(op.kind),
    };
    let new_part = hf::next_unused_hf_part(pkg, op.kind);
    pkg.set(&new_part, bytes)?;
    if let Some(src) = &inherited {
        picture::copy_part_rels(pkg, src, &new_part)?;
    }

    let mut rels = Relationships::from_package(pkg, doc_part)?;
    let rid = rels.next_rid();
    rels.push(Relationship {
        id: rid.clone(),
        rel_type: match op.kind {
            HfKind::Header => REL_HEADER,
            HfKind::Footer => REL_FOOTER,
        }
        .into(),
        target: hf::same_dir_rel_target(doc_part, &new_part),
        target_mode: None,
        extra: Vec::new(),
    });
    rels.write_to(pkg, doc_part)?;

    let mut cts = ContentTypes::from_package(pkg)?;
    let ct = match op.kind {
        HfKind::Header => CT_HEADER,
        HfKind::Footer => CT_FOOTER,
    };
    cts.set_override(&new_part, ct)?;
    cts.write_to(pkg)?;

    let doc = docs.get_mut(doc_part).unwrap();
    let sect = hf::find_sect_pr_mut(&mut doc.root, op.n).unwrap();
    let rid_attr = hf::rid_attr_name(sect);
    hf::insert_hf_ref(sect, op.kind, op.hf_type, &rid_attr, &rid);
    dirty.insert(doc_part.to_string(), ());
    Ok(())
}
pub(crate) fn relink_section(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    removed: &mut HashSet<String>,
    doc_part: &str,
    op: &SetHfLinkOp,
) -> Result<(), Error> {
    let old_part = hf::explicit_part_path(
        pkg,
        doc_part,
        &docs.get(doc_part).unwrap().root,
        op.n,
        op.kind,
        op.hf_type,
    )?;

    let doc = docs.get_mut(doc_part).unwrap();
    let sect = hf::find_sect_pr_mut(&mut doc.root, op.n).unwrap();
    hf::remove_hf_ref(sect, op.kind, op.hf_type);
    dirty.insert(doc_part.to_string(), ());

    let Some(old) = old_part else {
        return Ok(());
    };
    let still = hf::catalog(pkg, doc_part, &docs.get(doc_part).unwrap().root)?;
    if still.iter().any(|h| h.part == old) {
        return Ok(());
    }

    let mut rels = Relationships::from_package(pkg, doc_part)?;
    rels.remove_by_target_part(doc_part, &old);
    rels.write_to(pkg, doc_part)?;

    let mut cts = ContentTypes::from_package(pkg)?;
    cts.remove_override(&old);
    cts.write_to(pkg)?;

    pkg.remove(&old);
    pkg.remove(&rels_part_name(&old));
    removed.insert(old.clone());
    docs.remove(&old);
    dirty.remove(&old);
    Ok(())
}
pub(crate) fn apply_insert_section(
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    op: &InsertSectionOp,
    i: usize,
) -> Result<(), Error> {
    let doc = docs.get_mut(doc_part).unwrap();
    let after = match &op.place {
        InsertSectionPlace::After(p) => Some(p),
        InsertSectionPlace::EndOfDocument => None,
    };
    super::sect::insert_section_break(&mut doc.root, after, op.break_type, &op.id, i)?;
    dirty.insert(doc_part.to_string(), ());
    Ok(())
}
pub(crate) fn apply_delete_section(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    removed: &mut HashSet<String>,
    doc_part: &str,
    op: &DeleteSectionOp,
    i: usize,
) -> Result<(), Error> {
    let old_parts: Vec<String> = {
        let catalog = hf::catalog(pkg, doc_part, &docs.get(doc_part).unwrap().root)?;
        catalog.into_iter().map(|h| h.part).collect()
    };
    {
        let doc = docs.get_mut(doc_part).unwrap();
        super::sect::delete_section_break(&mut doc.root, op.n, &op.id, i)?;
    }
    dirty.insert(doc_part.to_string(), ());
    super::hf::drop_hf_parts_not_in_catalog(pkg, docs, dirty, removed, doc_part, &old_parts)
}
