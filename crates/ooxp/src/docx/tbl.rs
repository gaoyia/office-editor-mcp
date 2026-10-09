//! 正文与页眉页脚里的 `w:tbl`：inspect 名单与结构 apply。

use serde_json::{json, Value};

use crate::error::{Error, ErrorKind, IdReason, XmlIssue};
use crate::xml::{Element, Node};

use super::format;
use super::hf::{self, HfKind, HfPart, HfType};
use super::para::{self, ParaTarget};

const COL_W: u32 = 1440;
const MAX_ROWS: u64 = 500;
const MAX_COLS: u64 = 63;

#[derive(Debug, Clone)]
pub enum TableStory {
    Body,
    HeaderFooter {
        kind: HfKind,
        hf_type: HfType,
        n: u32,
    },
}

#[derive(Debug, Clone)]
pub enum TableRef {
    Table {
        story: TableStory,
        n: usize,
    },
    Row {
        story: TableStory,
        n: usize,
        row: usize,
    },
    Cell {
        story: TableStory,
        n: usize,
        row: usize,
        col: usize,
    },
}

impl TableRef {
    pub fn story(&self) -> &TableStory {
        match self {
            Self::Table { story, .. } | Self::Row { story, .. } | Self::Cell { story, .. } => story,
        }
    }

    fn table_n(&self) -> usize {
        match self {
            Self::Table { n, .. } | Self::Row { n, .. } | Self::Cell { n, .. } => *n,
        }
    }
}

#[derive(Debug, Clone)]
pub enum InsertAnchor {
    Document,
    HeaderFooterEnd {
        kind: HfKind,
        hf_type: HfType,
        n: u32,
    },
    AfterPara(ParaTarget),
    AfterTable(TableRef),
    InCell(TableRef),
}

#[derive(Clone, Copy)]
enum VMerge {
    Restart,
    Continue,
}

pub fn count_tables(root: &Element) -> usize {
    collect_tbl_paths(root).len()
}

pub fn tables_json(
    root: &Element,
    container: Option<&str>,
    expand_pics: bool,
    expand_runs: bool,
) -> Result<Vec<Value>, Error> {
    let paras = para::collect_paragraphs(root);
    let mut index_of: Vec<(*const Element, usize)> = Vec::new();
    for (i, p) in paras.iter().enumerate() {
        index_of.push((*p as *const Element, i + 1));
    }
    let paths = collect_tbl_paths(root);
    let mut out = Vec::new();
    for (i, (path, nested)) in paths.iter().enumerate() {
        let tbl = para::element_at(root, path).ok_or_else(|| xml_broken("tbl"))?;
        out.push(table_node(
            tbl,
            i + 1,
            *nested,
            container,
            &index_of,
            expand_pics,
            expand_runs,
        )?);
    }
    Ok(out)
}

pub fn parse_table_ref(id: &str, path: &str) -> Result<TableRef, Error> {
    let (story, rest) = split_story(id, path)?;
    parse_table_tail(story, rest, path)
}

pub fn parse_insert_anchor(id: &str, i: usize) -> Result<InsertAnchor, Error> {
    let path = format!("ops[{i}].id");
    if id == "docx:document" {
        return Ok(InsertAnchor::Document);
    }
    if let Some((kind, hf_type, n)) = para::parse_hf_container_id(id) {
        return Ok(InsertAnchor::HeaderFooterEnd { kind, hf_type, n });
    }
    if looks_like_table(id) {
        let r = parse_table_ref(id, &path)?;
        return match r {
            TableRef::Table { .. } => Ok(InsertAnchor::AfterTable(r)),
            TableRef::Cell { .. } => Ok(InsertAnchor::InCell(r)),
            TableRef::Row { .. } => Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::InsertTableAnchor,
                },
                path,
            )),
        };
    }
    let para = para::parse_para_id_at(id, &path)?;
    match para {
        ParaTarget::Body(_) | ParaTarget::HeaderFooter { .. } => Ok(InsertAnchor::AfterPara(para)),
        _ => Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::InsertTableAnchor,
            },
            path,
        )),
    }
}

pub fn parse_table_only(id: &str, i: usize, reason: IdReason) -> Result<TableRef, Error> {
    let path = format!("ops[{i}].id");
    let r = parse_table_ref(id, &path)?;
    match r {
        TableRef::Table { .. } => Ok(r),
        _ => Err(Error::with_path(ErrorKind::InvalidId { reason }, path)),
    }
}

pub fn parse_row_ref(id: &str, i: usize) -> Result<TableRef, Error> {
    let path = format!("ops[{i}].id");
    let r = parse_table_ref(id, &path)?;
    match r {
        TableRef::Row { .. } => Ok(r),
        _ => Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::DeleteRowNeedsRow,
            },
            path,
        )),
    }
}

pub fn parse_table_or_row(id: &str, i: usize) -> Result<TableRef, Error> {
    let path = format!("ops[{i}].id");
    let r = parse_table_ref(id, &path)?;
    match r {
        TableRef::Table { .. } | TableRef::Row { .. } => Ok(r),
        _ => Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::InsertRowNeedsTableOrRow,
            },
            path,
        )),
    }
}

pub fn parse_cell_ref(id: &str, i: usize) -> Result<TableRef, Error> {
    let path = format!("ops[{i}].id");
    let r = parse_table_ref(id, &path)?;
    match r {
        TableRef::Cell { .. } => Ok(r),
        _ => Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SplitNeedsCell,
            },
            path,
        )),
    }
}

pub fn part_for_story(
    catalog: &[HfPart],
    doc_part: &str,
    story: &TableStory,
    id: &str,
    i: usize,
) -> Result<String, Error> {
    match story {
        TableStory::Body => Ok(doc_part.to_string()),
        TableStory::HeaderFooter { kind, hf_type, n } => {
            let cid = format!("{}:{}:{n}", kind.as_str(), hf_type.as_str());
            let item = hf::find_by_id(catalog, &cid).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::TableNotFound { id: id.to_string() },
                    format!("ops[{i}].id"),
                )
            })?;
            Ok(item.part.clone())
        }
    }
}

pub fn part_for_anchor(
    catalog: &[HfPart],
    doc_part: &str,
    anchor: &InsertAnchor,
    id: &str,
    i: usize,
) -> Result<String, Error> {
    match anchor {
        InsertAnchor::Document => Ok(doc_part.to_string()),
        InsertAnchor::HeaderFooterEnd { kind, hf_type, n } => part_for_story(
            catalog,
            doc_part,
            &TableStory::HeaderFooter {
                kind: *kind,
                hf_type: *hf_type,
                n: *n,
            },
            id,
            i,
        ),
        InsertAnchor::AfterPara(p) => match p {
            ParaTarget::Body(_) => Ok(doc_part.to_string()),
            ParaTarget::HeaderFooter {
                kind, hf_type, n, ..
            } => part_for_story(
                catalog,
                doc_part,
                &TableStory::HeaderFooter {
                    kind: *kind,
                    hf_type: *hf_type,
                    n: *n,
                },
                id,
                i,
            ),
            _ => Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::InsertTableAnchor,
                },
                format!("ops[{i}].id"),
            )),
        },
        InsertAnchor::AfterTable(r) | InsertAnchor::InCell(r) => {
            part_for_story(catalog, doc_part, r.story(), id, i)
        }
    }
}

pub fn apply_insert_table(
    root: &mut Element,
    anchor: &InsertAnchor,
    cells: &[Vec<String>],
    id: &str,
    i: usize,
) -> Result<(), Error> {
    let ids = alloc_para_ids(root, cells.iter().map(|r| r.len()).sum());
    let tbl = build_table_el(root, cells, &ids);
    match anchor {
        InsertAnchor::Document => {
            let body = para::story_body_mut(root).ok_or_else(|| xml_missing("body", i))?;
            insert_before_sect_pr(body, tbl);
        }
        InsertAnchor::HeaderFooterEnd { .. } => {
            insert_before_sect_pr(root, tbl);
        }
        InsertAnchor::AfterPara(p) => {
            let body_para = match p {
                ParaTarget::Body(b) | ParaTarget::HeaderFooter { para: b, .. } => b,
                _ => {
                    return Err(Error::with_path(
                        ErrorKind::InvalidId {
                            reason: IdReason::InsertTableAnchor,
                        },
                        format!("ops[{i}].id"),
                    ));
                }
            };
            let path = para::find_paragraph_path(root, body_para).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::ParaNotFound { id: id.to_string() },
                    format!("ops[{i}].id"),
                )
            })?;
            let (parent, idx) = para::parent_and_index_mut(root, &path).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::ParaNotFound { id: id.to_string() },
                    format!("ops[{i}].id"),
                )
            })?;
            parent.children.insert(idx + 1, Node::Element(tbl));
        }
        InsertAnchor::AfterTable(r) => {
            let path = tbl_path(root, r.table_n(), id, i)?;
            let (parent, idx) = para::parent_and_index_mut(root, &path).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::TableNotFound { id: id.to_string() },
                    format!("ops[{i}].id"),
                )
            })?;
            parent.children.insert(idx + 1, Node::Element(tbl));
        }
        InsertAnchor::InCell(r) => {
            let TableRef::Cell { n, row, col, .. } = r else {
                unreachable!();
            };
            let pid = para::next_para_hex(root);
            let path = tbl_path(root, *n, id, i)?;
            let tbl_el = para::element_at_mut(root, &path).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::TableNotFound { id: id.to_string() },
                    format!("ops[{i}].id"),
                )
            })?;
            reject_complex(tbl_el, i)?;
            let tc = find_tc_mut(tbl_el, *row, *col, id, i)?;
            tc.children.push(Node::Element(tbl));
            ensure_trailing_p(tc, &pid);
        }
    }
    Ok(())
}

pub fn apply_delete_table(
    root: &mut Element,
    r: &TableRef,
    id: &str,
    i: usize,
) -> Result<(), Error> {
    let path = tbl_path(root, r.table_n(), id, i)?;
    {
        let tbl = para::element_at(root, &path).ok_or_else(|| {
            Error::with_path(
                ErrorKind::TableNotFound { id: id.to_string() },
                format!("ops[{i}].id"),
            )
        })?;
        reject_complex(tbl, i)?;
    }
    let (parent, idx) = para::parent_and_index_mut(root, &path).ok_or_else(|| {
        Error::with_path(
            ErrorKind::TableNotFound { id: id.to_string() },
            format!("ops[{i}].id"),
        )
    })?;
    parent.children.remove(idx);
    if parent.local_name() == "tc" {
        ensure_trailing_p(parent, "00000001");
    } else if !has_block(parent) {
        let empty = para::new_paragraph(parent, &para::next_para_hex(parent), "");
        insert_before_sect_pr(parent, empty);
    }
    Ok(())
}

pub fn apply_insert_row(root: &mut Element, r: &TableRef, id: &str, i: usize) -> Result<(), Error> {
    let n = r.table_n();
    let path = tbl_path(root, n, id, i)?;
    let (layout, src_idx) = {
        let tbl = para::element_at(root, &path).ok_or_else(|| missing_table(id, i))?;
        reject_complex(tbl, i)?;
        reject_grid_mismatch(tbl, i)?;
        let trs = tr_indices(tbl);
        if trs.is_empty() {
            return Err(missing_table(id, i));
        }
        let after = match r {
            TableRef::Table { .. } => trs.len() - 1,
            TableRef::Row { row, .. } => {
                if *row < 1 || *row > trs.len() {
                    return Err(missing_table(id, i));
                }
                *row - 1
            }
            _ => unreachable!(),
        };
        let src_idx = trs[after];
        let layout = row_spans(child_el(tbl, src_idx))?;
        (layout, src_idx)
    };
    let mut ids = alloc_para_ids(root, layout.len());
    let tbl = para::element_at_mut(root, &path).ok_or_else(|| missing_table(id, i))?;
    let new_tr = build_row(tbl, &layout, &mut ids);
    tbl.children.insert(src_idx + 1, Node::Element(new_tr));
    Ok(())
}

pub fn apply_delete_row(root: &mut Element, r: &TableRef, id: &str, i: usize) -> Result<(), Error> {
    let TableRef::Row { n, row, .. } = r else {
        unreachable!();
    };
    let path = tbl_path(root, *n, id, i)?;
    let tbl = para::element_at_mut(root, &path).ok_or_else(|| missing_table(id, i))?;
    reject_complex(tbl, i)?;
    let trs = tr_indices(tbl);
    if trs.len() <= 1 {
        return Err(Error::with_path(
            ErrorKind::LastTableRow,
            format!("ops[{i}].id"),
        ));
    }
    if *row < 1 || *row > trs.len() {
        return Err(missing_table(id, i));
    }
    tbl.children.remove(trs[*row - 1]);
    Ok(())
}

pub fn apply_insert_col(
    root: &mut Element,
    r: &TableRef,
    after: u32,
    id: &str,
    i: usize,
) -> Result<(), Error> {
    let path = tbl_path(root, r.table_n(), id, i)?;
    let (cols, n_rows, pos) = {
        let tbl = para::element_at_mut(root, &path).ok_or_else(|| missing_table(id, i))?;
        reject_complex(tbl, i)?;
        reject_grid_mismatch(tbl, i)?;
        let cols = col_count(tbl)?;
        if after as usize > cols {
            return Err(Error::with_path(
                ErrorKind::OutOfRange {
                    field: "after".into(),
                    min: 0,
                    max: cols as i64,
                },
                format!("ops[{i}].after"),
            ));
        }
        (cols, tr_indices(tbl).len(), after as usize)
    };
    let id_pool = alloc_para_ids(root, n_rows.max(1));
    let tbl = para::element_at_mut(root, &path).ok_or_else(|| missing_table(id, i))?;
    let w = grid_width_at(tbl, if pos == 0 { 0 } else { pos - 1 }).unwrap_or(COL_W);
    ensure_tbl_grid(tbl, cols, w);
    insert_grid_col(tbl, pos, w);
    let trs = tr_indices(tbl);
    for (k, tr_i) in trs.into_iter().enumerate() {
        let pid = id_pool.get(k).cloned().unwrap_or_else(|| "00000001".into());
        let tr = child_el_mut(tbl, tr_i);
        insert_tc_at(tr, pos, w, &pid)?;
    }
    Ok(())
}

pub fn apply_delete_col(
    root: &mut Element,
    r: &TableRef,
    col: u32,
    id: &str,
    i: usize,
) -> Result<(), Error> {
    let path = tbl_path(root, r.table_n(), id, i)?;
    let tbl = para::element_at_mut(root, &path).ok_or_else(|| missing_table(id, i))?;
    reject_complex(tbl, i)?;
    reject_grid_mismatch(tbl, i)?;
    let cols = col_count(tbl)?;
    if cols <= 1 {
        return Err(Error::with_path(
            ErrorKind::LastTableCol,
            format!("ops[{i}].col"),
        ));
    }
    let pos = col as usize;
    if pos < 1 || pos > cols {
        return Err(missing_table(id, i));
    }
    let trs = tr_indices(tbl);
    for tr_i in trs {
        let tr = child_el_mut(tbl, tr_i);
        delete_tc_at(tr, pos - 1)?;
    }
    remove_grid_col(tbl, pos - 1);
    Ok(())
}

pub fn apply_merge(
    root: &mut Element,
    r: &TableRef,
    r1: usize,
    c1: usize,
    r2: usize,
    c2: usize,
    id: &str,
    i: usize,
) -> Result<(), Error> {
    if r1 > r2 || c1 > c2 {
        return Err(Error::with_path(
            ErrorKind::MinValue {
                field: "startRow".into(),
                min: 1,
            },
            format!("ops[{i}]"),
        ));
    }
    let path = tbl_path(root, r.table_n(), id, i)?;
    let vertical = r2 > r1;
    {
        let tbl = para::element_at_mut(root, &path).ok_or_else(|| missing_table(id, i))?;
        reject_complex(tbl, i)?;
        reject_grid_mismatch(tbl, i)?;
        let rows = tr_indices(tbl).len();
        let cols = col_count(tbl)?;
        if r1 < 1 || r2 > rows || c1 < 1 || c2 > cols {
            return Err(missing_table(id, i));
        }
        if !merge_is_simple(tbl, r1, c1, r2, c2)? {
            return Err(Error::with_path(
                ErrorKind::MergeNotSimple,
                format!("ops[{i}]"),
            ));
        }
        for row in r1..=r2 {
            merge_row_fixed(tbl, row, c1, c2)?;
        }
        if vertical {
            absorb_vertical(tbl, r1, c1, r2)?;
            set_vmerge(tbl, r1, c1, VMerge::Restart)?;
            for row in r1 + 1..=r2 {
                set_vmerge(tbl, row, c1, VMerge::Continue)?;
            }
        }
    }
    if vertical {
        let ids = alloc_para_ids(root, r2 - r1);
        let tbl = para::element_at_mut(root, &path).ok_or_else(|| missing_table(id, i))?;
        for (k, row) in (r1 + 1..=r2).enumerate() {
            let pid = ids.get(k).map(String::as_str).unwrap_or("00000001");
            leave_empty_p(tbl, row, c1, pid)?;
        }
    }
    Ok(())
}

pub fn apply_split(root: &mut Element, r: &TableRef, id: &str, i: usize) -> Result<(), Error> {
    let TableRef::Cell { n, row, col, .. } = r else {
        unreachable!();
    };
    let path = tbl_path(root, *n, id, i)?;
    let tbl = para::element_at_mut(root, &path).ok_or_else(|| missing_table(id, i))?;
    reject_complex(tbl, i)?;
    reject_grid_mismatch(tbl, i)?;
    let info = cell_info(tbl, *row, *col).ok_or_else(|| missing_table(id, i))?;
    if matches!(info.v_merge, Some(VMerge::Continue)) {
        return Err(Error::with_path(
            ErrorKind::SplitContinueCell,
            format!("ops[{i}].id"),
        ));
    }
    if info.span == 1 && info.v_merge.is_none() {
        return Err(Error::with_path(
            ErrorKind::NothingToSplit,
            format!("ops[{i}].id"),
        ));
    }
    let mut last = *row;
    if matches!(info.v_merge, Some(VMerge::Restart)) {
        last = vertical_end(tbl, *row, *col, info.span);
    }
    for rr in *row..=last {
        split_row_cell(tbl, rr, *col, info.span)?;
        clear_vmerge(tbl, rr, *col)?;
    }
    Ok(())
}

pub fn table_xml(cells: &[Vec<String>], para_ids: &[String]) -> String {
    let cols = cells.first().map(|r| r.len()).unwrap_or(1);
    let mut grid = String::new();
    for _ in 0..cols {
        grid.push_str(&format!(r#"<w:gridCol w:w="{COL_W}"/>"#));
    }
    let mut rows = String::new();
    let mut k = 0usize;
    for row in cells {
        let mut tcs = String::new();
        for text in row {
            let pid = para_ids.get(k).map(String::as_str).unwrap_or("00000001");
            k += 1;
            let t = if text.is_empty() {
                String::new()
            } else {
                let space = if needs_space(text) {
                    r#" xml:space="preserve""#
                } else {
                    ""
                };
                format!(
                    "<w:r><w:t{space}>{}</w:t></w:r>",
                    crate::xml::escape_text(text)
                )
            };
            tcs.push_str(&format!(
                r#"<w:tc><w:tcPr><w:tcW w:w="{COL_W}" w:type="dxa"/></w:tcPr><w:p w14:paraId="{pid}" w14:textId="{pid}">{t}</w:p></w:tc>"#
            ));
        }
        rows.push_str(&format!("<w:tr>{tcs}</w:tr>"));
    }
    format!(
        r#"<w:tbl><w:tblPr><w:tblW w:w="0" w:type="auto"/>{borders}<w:tblLook w:val="04A0" w:firstRow="1" w:lastRow="0" w:firstColumn="1" w:lastColumn="0" w:noHBand="0" w:noVBand="1"/></w:tblPr><w:tblGrid>{grid}</w:tblGrid>{rows}</w:tbl>"#,
        borders = tbl_borders_xml()
    )
}

pub fn parse_cells_field(
    v: Option<&Value>,
    i: usize,
    rows: usize,
    cols: usize,
) -> Result<Vec<Vec<String>>, Error> {
    match v {
        None => Ok(vec![vec![String::new(); cols]; rows]),
        Some(Value::Array(arr)) => {
            if arr.len() != rows {
                return Err(Error::with_path(
                    ErrorKind::NeedTableCellsMatch,
                    format!("ops[{i}].cells"),
                ));
            }
            let mut out = Vec::new();
            for (ri, row) in arr.iter().enumerate() {
                let Some(cells) = row.as_array() else {
                    return Err(Error::with_path(
                        ErrorKind::WrongType {
                            field: "cells".into(),
                            expected: crate::error::TypeName::Array,
                        },
                        format!("ops[{i}].cells[{ri}]"),
                    ));
                };
                if cells.len() != cols {
                    return Err(Error::with_path(
                        ErrorKind::NeedTableCellsMatch,
                        format!("ops[{i}].cells[{ri}]"),
                    ));
                }
                let mut line = Vec::new();
                for (ci, c) in cells.iter().enumerate() {
                    match c {
                        Value::String(s) => line.push(s.clone()),
                        _ => {
                            return Err(Error::with_path(
                                ErrorKind::WrongType {
                                    field: "cells".into(),
                                    expected: crate::error::TypeName::String,
                                },
                                format!("ops[{i}].cells[{ri}][{ci}]"),
                            ));
                        }
                    }
                }
                out.push(line);
            }
            Ok(out)
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "cells".into(),
                expected: crate::error::TypeName::Array,
            },
            format!("ops[{i}].cells"),
        )),
    }
}

/// create 用的二维字符串表，路径由调用方给出。
pub fn parse_create_cells(v: &Value, path: &str) -> Result<Vec<Vec<String>>, Error> {
    let arr = v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "cells".into(),
                expected: crate::error::TypeName::Array,
            },
            path,
        )
    })?;
    if arr.is_empty() {
        return Err(Error::with_path(
            ErrorKind::EmptyArray {
                field: "cells".into(),
            },
            path,
        ));
    }
    if arr.len() as u64 > MAX_ROWS {
        return Err(Error::with_path(
            ErrorKind::OutOfRange {
                field: "cells".into(),
                min: 1,
                max: MAX_ROWS as i64,
            },
            path,
        ));
    }
    let mut out = Vec::new();
    let mut width = None;
    for (ri, row) in arr.iter().enumerate() {
        let cells = row.as_array().ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: "cells".into(),
                    expected: crate::error::TypeName::Array,
                },
                format!("{path}[{ri}]"),
            )
        })?;
        if cells.is_empty() {
            return Err(Error::with_path(
                ErrorKind::EmptyArray {
                    field: "cells".into(),
                },
                format!("{path}[{ri}]"),
            ));
        }
        if cells.len() as u64 > MAX_COLS {
            return Err(Error::with_path(
                ErrorKind::OutOfRange {
                    field: "cells".into(),
                    min: 1,
                    max: MAX_COLS as i64,
                },
                format!("{path}[{ri}]"),
            ));
        }
        match width {
            None => width = Some(cells.len()),
            Some(w) if w != cells.len() => {
                return Err(Error::with_path(
                    ErrorKind::NeedTableCellsMatch,
                    format!("{path}[{ri}]"),
                ));
            }
            Some(_) => {}
        }
        let mut line = Vec::new();
        for (ci, c) in cells.iter().enumerate() {
            match c {
                Value::String(s) => line.push(s.clone()),
                _ => {
                    return Err(Error::with_path(
                        ErrorKind::WrongType {
                            field: "cells".into(),
                            expected: crate::error::TypeName::String,
                        },
                        format!("{path}[{ri}][{ci}]"),
                    ));
                }
            }
        }
        out.push(line);
    }
    Ok(out)
}

pub fn parse_dim(v: Option<&Value>, i: usize, field: &str, max: u64) -> Result<usize, Error> {
    let path = format!("ops[{i}].{field}");
    match v {
        Some(Value::Number(n)) => {
            let Some(u) = n.as_u64() else {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: field.into(),
                        expected: crate::error::TypeName::Integer,
                    },
                    path,
                ));
            };
            if u < 1 || u > max {
                return Err(Error::with_path(
                    ErrorKind::OutOfRange {
                        field: field.into(),
                        min: 1,
                        max: max as i64,
                    },
                    path,
                ));
            }
            Ok(u as usize)
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: crate::error::TypeName::Integer,
            },
            path,
        )),
        None => Err(Error::with_path(
            ErrorKind::MissingField {
                field: field.into(),
            },
            path,
        )),
    }
}

pub fn max_rows() -> u64 {
    MAX_ROWS
}
pub fn max_cols() -> u64 {
    MAX_COLS
}

struct CellInfo {
    span: usize,
    v_merge: Option<VMerge>,
    start_col: usize,
}

fn table_node(
    tbl: &Element,
    index: usize,
    nested: bool,
    container: Option<&str>,
    index_of: &[(*const Element, usize)],
    expand_pics: bool,
    expand_runs: bool,
) -> Result<Value, Error> {
    let id = match container {
        Some(c) => format!("{c}/table:{index}"),
        None => format!("table:{index}"),
    };
    let trs: Vec<&Element> = tbl.children_by_local("tr");
    let col_count = col_count(tbl)?;
    let mut rows = Vec::new();
    for (ri, tr) in trs.iter().enumerate() {
        let row_n = ri + 1;
        let mut cells = Vec::new();
        let mut col = 1usize;
        for tc in tr.children_by_local("tc") {
            let span = grid_span(tc)?;
            let vm = v_merge_of(tc)?;
            let cell_id = format!("{id}/r{row_n}c{col}");
            let paras = cell_paragraphs(tc, container, index_of, expand_pics, expand_runs)?;
            cells.push(json!({
                "id": cell_id,
                "row": row_n,
                "col": col,
                "gridSpan": span as i64,
                "vMerge": match vm {
                    Some(VMerge::Restart) => Value::String("restart".into()),
                    Some(VMerge::Continue) => Value::String("continue".into()),
                    None => Value::Null,
                },
                "paragraphs": paras,
            }));
            col += span;
        }
        rows.push(json!({
            "id": format!("{id}/row:{row_n}"),
            "index": row_n,
            "cells": cells,
        }));
    }
    Ok(json!({
        "id": id,
        "index": index,
        "rowCount": trs.len(),
        "colCount": col_count,
        "nested": nested,
        "rows": rows,
    }))
}

fn cell_paragraphs(
    tc: &Element,
    container: Option<&str>,
    index_of: &[(*const Element, usize)],
    expand_pics: bool,
    expand_runs: bool,
) -> Result<Vec<Value>, Error> {
    let mut ps = Vec::new();
    collect_cell_ps(tc, &mut ps);
    let mut out = Vec::new();
    for p in ps {
        let idx = index_of
            .iter()
            .find(|(ptr, _)| std::ptr::eq(*ptr, p))
            .map(|(_, n)| *n)
            .unwrap_or(1);
        let (id, stable) = match container {
            Some(c) => para::para_stable_id_in(c, p, idx),
            None => para::para_stable_id(p, idx),
        };
        let pics = super::picture::collect_pictures(p, &id)?;
        let boxes = super::textbox::collect_textboxes(p, &id)?;
        out.push(rebuild_para_node(
            &id,
            idx,
            p,
            stable,
            pics.len(),
            boxes.len(),
            expand_pics,
            expand_runs,
            &pics,
        )?);
    }
    Ok(out)
}

fn rebuild_para_node(
    id: &str,
    idx: usize,
    p: &Element,
    stable: bool,
    pic_n: usize,
    box_n: usize,
    expand_pics: bool,
    expand_runs: bool,
    pics: &[super::picture::PicInfo],
) -> Result<Value, Error> {
    let mut node = format::para_node(p, idx, None, Some(pic_n), Some(box_n));
    // 单元格段落 id 已按故事序号算好，覆盖 format 按该 p 自己算出来的 id。
    node["id"] = json!(id);
    node["stable"] = json!(stable);
    if expand_pics {
        node["pictures"] = Value::Array(super::picture::pictures_json(pics));
    }
    format::maybe_runs(&mut node, p, expand_runs);
    Ok(node)
}

fn collect_cell_ps<'a>(el: &'a Element, out: &mut Vec<&'a Element>) {
    if el.local_name() == "tbl" {
        return;
    }
    if el.local_name() == "p" {
        out.push(el);
        return;
    }
    if el.local_name() == "AlternateContent" {
        if let Some(i) = first_alt(el) {
            if let Node::Element(c) = &el.children[i] {
                collect_cell_ps(c, out);
            }
        }
        return;
    }
    for n in &el.children {
        if let Node::Element(e) = n {
            collect_cell_ps(e, out);
        }
    }
}

fn collect_tbl_paths(root: &Element) -> Vec<(Vec<usize>, bool)> {
    let mut path = Vec::new();
    let mut out = Vec::new();
    walk_tbl(root, &mut path, false, &mut out);
    out
}

fn walk_tbl(el: &Element, path: &mut Vec<usize>, in_tc: bool, out: &mut Vec<(Vec<usize>, bool)>) {
    if el.local_name() == "txbxContent" {
        return;
    }
    if el.local_name() == "AlternateContent" {
        if let Some(i) = first_alt(el) {
            path.push(i);
            if let Node::Element(c) = &el.children[i] {
                walk_tbl(c, path, in_tc, out);
            }
            path.pop();
        }
        return;
    }
    if el.local_name() == "tbl" {
        out.push((path.clone(), in_tc));
        for (i, n) in el.children.iter().enumerate() {
            if let Node::Element(e) = n {
                path.push(i);
                walk_tbl(e, path, false, out);
                path.pop();
            }
        }
        return;
    }
    let next = in_tc || el.local_name() == "tc";
    for (i, n) in el.children.iter().enumerate() {
        if let Node::Element(e) = n {
            path.push(i);
            walk_tbl(e, path, next, out);
            path.pop();
        }
    }
}

fn looks_like_table(id: &str) -> bool {
    id.starts_with("table:") || id.contains("/table:")
}

fn split_story<'a>(id: &'a str, path: &str) -> Result<(TableStory, &'a str), Error> {
    if let Some(rest) = id.strip_prefix("table:") {
        return Ok((TableStory::Body, rest));
    }
    if let Some((pre, rest)) = id.split_once("/table:") {
        if let Some((kind, hf_type, n)) = para::parse_hf_container_id(pre) {
            return Ok((TableStory::HeaderFooter { kind, hf_type, n }, rest));
        }
    }
    Err(Error::with_path(
        ErrorKind::InvalidId {
            reason: IdReason::TablePrefix,
        },
        path.to_string(),
    ))
}

fn parse_table_tail(story: TableStory, rest: &str, path: &str) -> Result<TableRef, Error> {
    if let Some((n_s, tail)) = rest.split_once('/') {
        let n = parse_n(n_s, path)?;
        if let Some(row_s) = tail.strip_prefix("row:") {
            let row = parse_n(row_s, path)?;
            return Ok(TableRef::Row { story, n, row });
        }
        if let Some(rc) = parse_rc(tail) {
            return Ok(TableRef::Cell {
                story,
                n,
                row: rc.0,
                col: rc.1,
            });
        }
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::TableCellForm,
            },
            path.to_string(),
        ));
    }
    let n = parse_n(rest, path)?;
    Ok(TableRef::Table { story, n })
}

fn parse_rc(s: &str) -> Option<(usize, usize)> {
    let rest = s.strip_prefix('r')?;
    let (rs, cs) = rest.split_once('c')?;
    let row = rs.parse().ok()?;
    let col = cs.parse().ok()?;
    if row < 1 || col < 1 {
        return None;
    }
    Some((row, col))
}

fn parse_n(s: &str, path: &str) -> Result<usize, Error> {
    let n: usize = s.parse().map_err(|_| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::TableIndexNotInt,
            },
            path.to_string(),
        )
    })?;
    if n < 1 {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::TableIndexFromOne,
            },
            path.to_string(),
        ));
    }
    Ok(n)
}

fn tbl_path(root: &Element, n: usize, id: &str, i: usize) -> Result<Vec<usize>, Error> {
    let paths = collect_tbl_paths(root);
    paths
        .get(n - 1)
        .map(|(p, _)| p.clone())
        .ok_or_else(|| missing_table(id, i))
}

fn col_count(tbl: &Element) -> Result<usize, Error> {
    if let Some(grid) = tbl.child_by_local("tblGrid") {
        let n = grid
            .element_children()
            .filter(|e| e.local_name() == "gridCol")
            .count();
        if n > 0 {
            return Ok(n);
        }
    }
    if let Some(tr) = tbl.children_by_local("tr").into_iter().next() {
        let mut n = 0usize;
        for tc in tr.children_by_local("tc") {
            n += grid_span(tc)?;
        }
        return Ok(n.max(1));
    }
    Ok(1)
}

fn grid_span(tc: &Element) -> Result<usize, Error> {
    let Some(pr) = tc.child_by_local("tcPr") else {
        return Ok(1);
    };
    let Some(gs) = pr.child_by_local("gridSpan") else {
        return Ok(1);
    };
    let Some(v) = gs.attr_local("val") else {
        return Ok(1);
    };
    let n: usize = v.parse().map_err(|_| xml_broken("gridSpan"))?;
    if n < 1 {
        return Err(xml_broken("gridSpan"));
    }
    Ok(n)
}

fn v_merge_of(tc: &Element) -> Result<Option<VMerge>, Error> {
    let Some(pr) = tc.child_by_local("tcPr") else {
        return Ok(None);
    };
    let Some(vm) = pr.child_by_local("vMerge") else {
        return Ok(None);
    };
    match vm.attr_local("val") {
        None => Ok(Some(VMerge::Continue)),
        Some("restart") => Ok(Some(VMerge::Restart)),
        Some("continue") => Ok(Some(VMerge::Continue)),
        Some(_) => Err(xml_broken("vMerge")),
    }
}

fn row_spans(tr: &Element) -> Result<Vec<usize>, Error> {
    let mut out = Vec::new();
    for tc in tr.children_by_local("tc") {
        out.push(grid_span(tc)?);
    }
    Ok(out)
}

fn reject_complex(tbl: &Element, i: usize) -> Result<(), Error> {
    if structural_revision(tbl) {
        return Err(Error::with_path(
            ErrorKind::TableHasRevision,
            format!("ops[{i}]"),
        ));
    }
    Ok(())
}

fn reject_grid_mismatch(tbl: &Element, i: usize) -> Result<(), Error> {
    let cols = col_count(tbl)?;
    for tr in tbl.children_by_local("tr") {
        let mut n = 0usize;
        for tc in tr.children_by_local("tc") {
            n += grid_span(tc)?;
        }
        if n != cols {
            return Err(Error::with_path(
                ErrorKind::TableGridMismatch,
                format!("ops[{i}]"),
            ));
        }
    }
    Ok(())
}

fn structural_revision(tbl: &Element) -> bool {
    scan_struct(tbl, true)
}

fn scan_struct(el: &Element, top: bool) -> bool {
    let name = el.local_name();
    if !top && name == "tbl" {
        return false;
    }
    if name == "p" {
        return false;
    }
    if matches!(
        name,
        "ins"
            | "del"
            | "moveFrom"
            | "moveTo"
            | "tblPrChange"
            | "trPrChange"
            | "tcPrChange"
            | "tblGridChange"
            | "tblPrExChange"
    ) {
        return true;
    }
    el.element_children().any(|c| scan_struct(c, false))
}

fn tr_indices(tbl: &Element) -> Vec<usize> {
    tbl.children
        .iter()
        .enumerate()
        .filter_map(|(i, n)| match n {
            Node::Element(e) if e.local_name() == "tr" => Some(i),
            _ => None,
        })
        .collect()
}

fn child_el(parent: &Element, i: usize) -> &Element {
    match &parent.children[i] {
        Node::Element(e) => e,
        _ => unreachable!(),
    }
}

fn child_el_mut(parent: &mut Element, i: usize) -> &mut Element {
    match &mut parent.children[i] {
        Node::Element(e) => e,
        _ => unreachable!(),
    }
}

fn build_table_el(hint: &Element, cells: &[Vec<String>], ids: &[String]) -> Element {
    let pfx = hint.prefixed("tbl");
    let mut tbl = Element::new(pfx);
    let mut pr = Element::new(tbl.prefixed("tblPr"));
    let mut w = Element::new(pr.prefixed("tblW"));
    w.set_attr(&w.prefixed("w"), "0");
    w.set_attr(&w.prefixed("type"), "auto");
    pr.push_element(w);
    push_borders(&mut pr);
    let mut look = Element::new(pr.prefixed("tblLook"));
    look.set_attr(&look.prefixed("val"), "04A0");
    look.set_attr(&look.prefixed("firstRow"), "1");
    look.set_attr(&look.prefixed("lastRow"), "0");
    look.set_attr(&look.prefixed("firstColumn"), "1");
    look.set_attr(&look.prefixed("lastColumn"), "0");
    look.set_attr(&look.prefixed("noHBand"), "0");
    look.set_attr(&look.prefixed("noVBand"), "1");
    pr.push_element(look);
    tbl.push_element(pr);
    let cols = cells.first().map(|r| r.len()).unwrap_or(1);
    let mut grid = Element::new(tbl.prefixed("tblGrid"));
    for _ in 0..cols {
        let mut gc = Element::new(grid.prefixed("gridCol"));
        gc.set_attr(&gc.prefixed("w"), COL_W.to_string());
        grid.push_element(gc);
    }
    tbl.push_element(grid);
    let mut k = 0usize;
    for row in cells {
        let spans = vec![1usize; row.len()];
        let mut id_slice: Vec<String> = Vec::new();
        for _ in row {
            id_slice.push(ids.get(k).cloned().unwrap_or_else(|| "00000001".into()));
            k += 1;
        }
        tbl.push_element(build_row_texts(&tbl, row, &spans, &id_slice));
    }
    tbl
}

fn build_row(hint: &Element, spans: &[usize], ids: &mut Vec<String>) -> Element {
    let texts = vec![String::new(); spans.len()];
    let mut used = Vec::new();
    for _ in spans {
        if ids.is_empty() {
            used.push("00000001".into());
        } else {
            used.push(ids.remove(0));
        }
    }
    build_row_texts(hint, &texts, spans, &used)
}

fn build_row_texts(hint: &Element, texts: &[String], spans: &[usize], ids: &[String]) -> Element {
    let mut tr = Element::new(hint.prefixed("tr"));
    for (i, text) in texts.iter().enumerate() {
        let span = spans.get(i).copied().unwrap_or(1);
        let pid = ids.get(i).map(String::as_str).unwrap_or("00000001");
        tr.push_element(build_tc(&tr, text, span, COL_W, pid));
    }
    tr
}

fn build_tc(hint: &Element, text: &str, span: usize, width: u32, para_id: &str) -> Element {
    let mut tc = Element::new(hint.prefixed("tc"));
    let mut pr = Element::new(tc.prefixed("tcPr"));
    let mut w = Element::new(pr.prefixed("tcW"));
    w.set_attr(&w.prefixed("w"), (width * span as u32).to_string());
    w.set_attr(&w.prefixed("type"), "dxa");
    pr.push_element(w);
    if span > 1 {
        let mut gs = Element::new(pr.prefixed("gridSpan"));
        gs.set_attr(&gs.prefixed("val"), span.to_string());
        pr.push_element(gs);
    }
    tc.push_element(pr);
    tc.push_element(para::new_paragraph(&tc, para_id, text));
    tc
}

fn push_borders(pr: &mut Element) {
    let mut b = Element::new(pr.prefixed("tblBorders"));
    for name in ["top", "left", "bottom", "right", "insideH", "insideV"] {
        let mut e = Element::new(b.prefixed(name));
        e.set_attr(&e.prefixed("val"), "single");
        e.set_attr(&e.prefixed("sz"), "4");
        e.set_attr(&e.prefixed("space"), "0");
        e.set_attr(&e.prefixed("color"), "auto");
        b.push_element(e);
    }
    pr.push_element(b);
}

fn tbl_borders_xml() -> String {
    let mut s = String::from("<w:tblBorders>");
    for name in ["top", "left", "bottom", "right", "insideH", "insideV"] {
        s.push_str(&format!(
            r#"<w:{name} w:val="single" w:sz="4" w:space="0" w:color="auto"/>"#
        ));
    }
    s.push_str("</w:tblBorders>");
    s
}

fn alloc_para_ids(root: &Element, n: usize) -> Vec<String> {
    let start = u32::from_str_radix(&para::next_para_hex(root), 16).unwrap_or(1);
    (0..n as u32)
        .map(|i| format!("{:08X}", start.saturating_add(i).max(1)))
        .collect()
}

fn insert_before_sect_pr(parent: &mut Element, el: Element) {
    let pos = parent
        .children
        .iter()
        .position(|n| matches!(n, Node::Element(e) if e.local_name() == "sectPr"));
    match pos {
        Some(i) => parent.children.insert(i, Node::Element(el)),
        None => parent.children.push(Node::Element(el)),
    }
}

fn has_block(parent: &Element) -> bool {
    parent
        .element_children()
        .any(|e| matches!(e.local_name(), "p" | "tbl"))
}

fn ensure_trailing_p(tc: &mut Element, fallback: &str) {
    let has_p = tc.element_children().any(|e| e.local_name() == "p");
    if !has_p {
        let id = para::next_para_hex(tc);
        let pid = if id == "00000001" {
            fallback.to_string()
        } else {
            id
        };
        tc.push_element(para::new_paragraph(tc, &pid, ""));
    }
}

fn find_tc_mut<'a>(
    tbl: &'a mut Element,
    row: usize,
    col: usize,
    id: &str,
    i: usize,
) -> Result<&'a mut Element, Error> {
    let trs = tr_indices(tbl);
    if row < 1 || row > trs.len() {
        return Err(missing_table(id, i));
    }
    let tr_i = trs[row - 1];
    let tr = child_el_mut(tbl, tr_i);
    let mut cursor = 1usize;
    let mut hit = None;
    for (j, n) in tr.children.iter().enumerate() {
        if let Node::Element(tc) = n {
            if tc.local_name() == "tc" {
                let span = grid_span(tc).unwrap_or(1);
                if cursor == col {
                    hit = Some(j);
                    break;
                }
                cursor += span;
            }
        }
    }
    let j = hit.ok_or_else(|| missing_table(id, i))?;
    match &mut tr.children[j] {
        Node::Element(tc) => Ok(tc),
        _ => Err(missing_table(id, i)),
    }
}

fn ensure_tbl_grid(tbl: &mut Element, cols: usize, w: u32) {
    if tbl.child_by_local("tblGrid").is_some() {
        return;
    }
    let mut grid = Element::new(tbl.prefixed("tblGrid"));
    for _ in 0..cols {
        let mut gc = Element::new(grid.prefixed("gridCol"));
        gc.set_attr(&gc.prefixed("w"), w.to_string());
        grid.push_element(gc);
    }
    let pos = tbl
        .children
        .iter()
        .position(|n| matches!(n, Node::Element(e) if e.local_name() == "tblPr"))
        .map(|i| i + 1)
        .unwrap_or(0);
    tbl.children.insert(pos, Node::Element(grid));
}

fn grid_width_at(tbl: &Element, idx: usize) -> Option<u32> {
    let grid = tbl.child_by_local("tblGrid")?;
    let cols: Vec<&Element> = grid
        .element_children()
        .filter(|e| e.local_name() == "gridCol")
        .collect();
    cols.get(idx)
        .and_then(|c| c.attr_local("w"))
        .and_then(|s| s.parse().ok())
}

fn insert_grid_col(tbl: &mut Element, pos: usize, w: u32) {
    let Some(grid) = tbl.child_by_local_mut("tblGrid") else {
        return;
    };
    let mut gc = Element::new(grid.prefixed("gridCol"));
    gc.set_attr(&gc.prefixed("w"), w.to_string());
    let mut seen = 0usize;
    let mut at = grid.children.len();
    for (i, n) in grid.children.iter().enumerate() {
        if let Node::Element(e) = n {
            if e.local_name() == "gridCol" {
                if seen == pos {
                    at = i;
                    break;
                }
                seen += 1;
            }
        }
    }
    grid.children.insert(at, Node::Element(gc));
}

fn remove_grid_col(tbl: &mut Element, pos: usize) {
    let Some(grid) = tbl.child_by_local_mut("tblGrid") else {
        return;
    };
    let mut seen = 0usize;
    let mut hit = None;
    for (i, n) in grid.children.iter().enumerate() {
        if let Node::Element(e) = n {
            if e.local_name() == "gridCol" {
                if seen == pos {
                    hit = Some(i);
                    break;
                }
                seen += 1;
            }
        }
    }
    if let Some(i) = hit {
        grid.children.remove(i);
    }
}

fn insert_tc_at(tr: &mut Element, pos: usize, w: u32, para_id: &str) -> Result<(), Error> {
    let mut cursor = 0usize;
    let mut insert_before = None;
    let mut bump = None;
    for (i, n) in tr.children.iter().enumerate() {
        if let Node::Element(tc) = n {
            if tc.local_name() == "tc" {
                let span = grid_span(tc)?;
                if cursor == pos {
                    insert_before = Some(i);
                    break;
                }
                if cursor < pos && pos < cursor + span {
                    bump = Some(i);
                    break;
                }
                cursor += span;
            }
        }
    }
    if let Some(i) = bump {
        if let Node::Element(tc) = &mut tr.children[i] {
            bump_span(tc, 1)?;
        }
        return Ok(());
    }
    let tc = build_tc(tr, "", 1, w, para_id);
    match insert_before {
        Some(i) => tr.children.insert(i, Node::Element(tc)),
        None => tr.children.push(Node::Element(tc)),
    }
    Ok(())
}

fn delete_tc_at(tr: &mut Element, pos: usize) -> Result<(), Error> {
    let mut cursor = 0usize;
    let mut hit = None;
    for (i, n) in tr.children.iter().enumerate() {
        if let Node::Element(tc) = n {
            if tc.local_name() == "tc" {
                let span = grid_span(tc)?;
                if pos >= cursor && pos < cursor + span {
                    hit = Some((i, span));
                    break;
                }
                cursor += span;
            }
        }
    }
    let Some((i, span)) = hit else {
        return Ok(());
    };
    if span == 1 {
        tr.children.remove(i);
    } else if let Node::Element(tc) = &mut tr.children[i] {
        bump_span(tc, -1)?;
    }
    Ok(())
}

fn bump_span(tc: &mut Element, delta: i32) -> Result<(), Error> {
    let new_span = {
        let cur = tc
            .child_by_local("tcPr")
            .and_then(|pr| pr.child_by_local("gridSpan"))
            .and_then(|gs| gs.attr_local("val"))
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or(1);
        (cur + delta).max(1) as usize
    };
    set_span(tc, new_span);
    Ok(())
}

fn merge_is_simple(
    tbl: &Element,
    r1: usize,
    c1: usize,
    r2: usize,
    c2: usize,
) -> Result<bool, Error> {
    for row in r1..=r2 {
        let info = row_cell_starts(tbl, row)?;
        let mut cover = 0usize;
        for c in &info {
            if c.start_col >= c1 && c.start_col <= c2 {
                if c.start_col + c.span - 1 > c2 {
                    return Ok(false);
                }
                if c.span != 1 || c.v_merge.is_some() {
                    return Ok(false);
                }
                cover += c.span;
            } else if c.start_col < c1 && c.start_col + c.span > c1 {
                return Ok(false);
            }
        }
        if cover != c2 - c1 + 1 {
            return Ok(false);
        }
    }
    Ok(true)
}

fn row_cell_starts(tbl: &Element, row: usize) -> Result<Vec<CellInfo>, Error> {
    let trs = tbl.children_by_local("tr");
    let tr = trs.get(row - 1).ok_or_else(|| xml_broken("tr"))?;
    let mut cursor = 1usize;
    let mut out = Vec::new();
    for tc in tr.children_by_local("tc") {
        let span = grid_span(tc)?;
        out.push(CellInfo {
            span,
            v_merge: v_merge_of(tc)?,
            start_col: cursor,
        });
        cursor += span;
    }
    Ok(out)
}

fn set_span(tc: &mut Element, span: usize) {
    let pr = if tc.child_by_local("tcPr").is_some() {
        tc.child_by_local_mut("tcPr").unwrap()
    } else {
        let pr = Element::new(tc.prefixed("tcPr"));
        tc.children.insert(0, Node::Element(pr));
        tc.child_by_local_mut("tcPr").unwrap()
    };
    if span <= 1 {
        pr.children
            .retain(|n| !matches!(n, Node::Element(e) if e.local_name() == "gridSpan"));
        return;
    }
    if let Some(gs) = pr.child_by_local_mut("gridSpan") {
        hf::set_local_attr(gs, "val", span.to_string());
    } else {
        let mut gs = Element::new(pr.prefixed("gridSpan"));
        gs.set_attr(&gs.prefixed("val"), span.to_string());
        pr.push_element(gs);
    }
}

fn set_vmerge(tbl: &mut Element, row: usize, col: usize, kind: VMerge) -> Result<(), Error> {
    let tc = tc_at_mut(tbl, row, col).ok_or_else(|| xml_broken("tc"))?;
    let pr = ensure_tc_pr(tc);
    if let Some(vm) = pr.child_by_local_mut("vMerge") {
        hf::set_local_attr(
            vm,
            "val",
            match kind {
                VMerge::Restart => "restart",
                VMerge::Continue => "continue",
            },
        );
    } else {
        let mut vm = Element::new(pr.prefixed("vMerge"));
        vm.set_attr(
            &vm.prefixed("val"),
            match kind {
                VMerge::Restart => "restart",
                VMerge::Continue => "continue",
            },
        );
        pr.push_element(vm);
    }
    Ok(())
}

fn clear_vmerge(tbl: &mut Element, row: usize, col: usize) -> Result<(), Error> {
    let Some(tc) = tc_at_mut(tbl, row, col) else {
        return Ok(());
    };
    if let Some(pr) = tc.child_by_local_mut("tcPr") {
        pr.children
            .retain(|n| !matches!(n, Node::Element(e) if e.local_name() == "vMerge"));
    }
    Ok(())
}

fn ensure_tc_pr(tc: &mut Element) -> &mut Element {
    if tc.child_by_local("tcPr").is_none() {
        let pr = Element::new(tc.prefixed("tcPr"));
        tc.children.insert(0, Node::Element(pr));
    }
    tc.child_by_local_mut("tcPr").unwrap()
}

fn tc_at_mut(tbl: &mut Element, row: usize, col: usize) -> Option<&mut Element> {
    let trs = tr_indices(tbl);
    let tr = child_el_mut(tbl, *trs.get(row - 1)?);
    let mut cursor = 1usize;
    let mut hit = None;
    for (j, n) in tr.children.iter().enumerate() {
        if let Node::Element(tc) = n {
            if tc.local_name() == "tc" {
                let span = grid_span(tc).ok()?;
                if cursor == col {
                    hit = Some(j);
                    break;
                }
                cursor += span;
            }
        }
    }
    hit.and_then(move |j| match &mut tr.children[j] {
        Node::Element(tc) => Some(tc),
        _ => None,
    })
}

fn cell_info(tbl: &Element, row: usize, col: usize) -> Option<CellInfo> {
    let starts = row_cell_starts(tbl, row).ok()?;
    starts.into_iter().find(|c| c.start_col == col)
}

fn vertical_end(tbl: &Element, row: usize, col: usize, span: usize) -> usize {
    let nrows = tbl.children_by_local("tr").len();
    let mut last = row;
    for r in row + 1..=nrows {
        match cell_info(tbl, r, col) {
            Some(info) if info.span == span && matches!(info.v_merge, Some(VMerge::Continue)) => {
                last = r;
            }
            _ => break,
        }
    }
    last
}

fn split_row_cell(tbl: &mut Element, row: usize, col: usize, span: usize) -> Result<(), Error> {
    if span <= 1 {
        return Ok(());
    }
    let trs = tr_indices(tbl);
    let tr_i = trs[row - 1];
    let extra = span - 1;
    let mut ids = alloc_para_ids(child_el(tbl, tr_i), extra);
    let tr = child_el_mut(tbl, tr_i);
    let mut cursor = 1usize;
    let mut hit = None;
    for (j, n) in tr.children.iter().enumerate() {
        if let Node::Element(tc) = n {
            if tc.local_name() == "tc" {
                let s = grid_span(tc)?;
                if cursor == col {
                    hit = Some(j);
                    break;
                }
                cursor += s;
            }
        }
    }
    let j = hit.ok_or_else(|| xml_broken("tc"))?;
    if let Node::Element(tc) = &mut tr.children[j] {
        set_span(tc, 1);
    }
    for k in 0..extra {
        let pid = ids
            .get_mut(k)
            .map(std::mem::take)
            .unwrap_or_else(|| "00000001".into());
        let tc = build_tc(tr, "", 1, COL_W, &pid);
        tr.children.insert(j + 1 + k, Node::Element(tc));
    }
    Ok(())
}

fn leave_empty_p(tbl: &mut Element, row: usize, col: usize, pid: &str) -> Result<(), Error> {
    let tc = tc_at_mut(tbl, row, col).ok_or_else(|| xml_broken("tc"))?;
    tc.children
        .retain(|n| matches!(n, Node::Element(e) if e.local_name() == "tcPr"));
    let p = para::new_paragraph(tc, pid, "");
    tc.push_element(p);
    Ok(())
}

fn absorb_vertical(tbl: &mut Element, r1: usize, c1: usize, r2: usize) -> Result<(), Error> {
    let mut extra = Vec::new();
    for row in r1 + 1..=r2 {
        if let Some(tc) = tc_at_mut(tbl, row, c1) {
            extra.extend(
                tc.children
                    .iter()
                    .filter(|n| !matches!(n, Node::Element(e) if e.local_name() == "tcPr"))
                    .cloned(),
            );
        }
    }
    if let Some(top) = tc_at_mut(tbl, r1, c1) {
        top.children.extend(extra);
    }
    Ok(())
}

fn merge_row_fixed(tbl: &mut Element, row: usize, c1: usize, c2: usize) -> Result<(), Error> {
    let trs = tr_indices(tbl);
    let tr_i = trs[row - 1];
    let tr = child_el_mut(tbl, tr_i);
    let mut cursor = 1usize;
    let mut idxs = Vec::new();
    for (j, n) in tr.children.iter().enumerate() {
        if let Node::Element(tc) = n {
            if tc.local_name() == "tc" {
                let span = grid_span(tc)?;
                if cursor >= c1 && cursor <= c2 {
                    idxs.push(j);
                }
                cursor += span;
            }
        }
    }
    if idxs.is_empty() {
        return Err(xml_broken("tc"));
    }
    let keep = idxs[0];
    let drop: Vec<usize> = idxs[1..].to_vec();
    let mut extra = Vec::new();
    for &j in drop.iter().rev() {
        if let Node::Element(mut dead) = tr.children.remove(j) {
            extra.extend(
                dead.children
                    .drain(..)
                    .filter(|n| !matches!(n, Node::Element(e) if e.local_name() == "tcPr")),
            );
        }
    }
    if let Node::Element(keep_tc) = &mut tr.children[keep] {
        set_span(keep_tc, c2 - c1 + 1);
        keep_tc.children.extend(extra);
    }
    Ok(())
}

fn first_alt(el: &Element) -> Option<usize> {
    let mut fallback = None;
    for (i, n) in el.children.iter().enumerate() {
        if let Node::Element(e) = n {
            if e.local_name() == "Choice" {
                return Some(i);
            }
            if e.local_name() == "Fallback" && fallback.is_none() {
                fallback = Some(i);
            }
        }
    }
    fallback
}

fn needs_space(text: &str) -> bool {
    text.starts_with(' ') || text.starts_with('\t') || text.ends_with(' ') || text.ends_with('\t')
}

fn xml_broken(detail: &str) -> Error {
    Error::with_path(
        ErrorKind::XmlBroken {
            issue: XmlIssue::Parse,
            detail: Some(detail.into()),
        },
        "document",
    )
}

fn xml_missing(child: &str, i: usize) -> Error {
    Error::with_path(
        ErrorKind::XmlMissing {
            parent: "document.xml".into(),
            child: child.into(),
        },
        format!("ops[{i}].id"),
    )
}

fn missing_table(id: &str, i: usize) -> Error {
    Error::with_path(
        ErrorKind::TableNotFound { id: id.to_string() },
        format!("ops[{i}].id"),
    )
}
