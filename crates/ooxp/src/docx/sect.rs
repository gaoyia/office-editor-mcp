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
        hf::strip_hf_refs(&mut clone);
        set_section_type(&mut clone, ty);
        insert_empty_after(root, &path, clone, err_id, op_i)?;
    } else {
        let dest = para::element_at(root, &dest_path).expect("dest path");
        let mut clone = dest.clone();
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
    if n < 1 || n > paths.len() {
        return Err(Error::with_path(
            ErrorKind::SectionNotFound {
                id: err_id.to_string(),
            },
            format!("ops[{op_i}].id"),
        ));
    }
    if n == paths.len() {
        return Err(Error::with_path(
            ErrorKind::CannotDeleteLastSection,
            format!("ops[{op_i}].id"),
        ));
    }
    let dest_path = &paths[n - 1];
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
    let next_path = paths[n].clone();
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
