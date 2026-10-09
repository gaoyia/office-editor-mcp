//! 幻灯片 `p:graphicFrame` 里的 DrawingML 表（`a:tbl`）。

use serde_json::{json, Map, Value};

use crate::error::{Error, ErrorKind, IdReason, TypeName};
use crate::xml::{Document, Element, Node};

use super::picture;
use super::request::{
    DeleteColOp, DeleteRowOp, DeleteTableOp, InsertColOp, InsertRowOp, InsertTableOp,
    SetTableCellTextOp, SetTablePosOp, SetTableSizeOp,
};
use super::shape;
use super::DML_NS;

pub const TABLE_URI: &str = "http://schemas.openxmlformats.org/drawingml/2006/table";
const STYLE_ID: &str = "{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}";
const DEFAULT_CX: i64 = 5_486_400;
const DEFAULT_CY: i64 = 3_200_400;
const MAX_ROWS: u64 = 500;
const MAX_COLS: u64 = 75;

#[derive(Debug, Clone)]
pub enum TableKey {
    Id(String),
    Index(usize),
}

#[derive(Debug, Clone)]
pub struct TableTarget {
    pub slide: usize,
    pub on_notes: bool,
    pub master: Option<(usize, Option<usize>)>,
    pub key: TableKey,
}

#[derive(Debug, Clone)]
pub enum TableOrRow {
    Table(TableTarget),
    Row { table: TableTarget, row: usize },
}

#[derive(Debug, Clone)]
pub struct CellTarget {
    pub table: TableTarget,
    pub row: usize,
    pub col: usize,
}

pub fn parse_insert_slide_id(id: &str, op_index: usize) -> Result<picture::TreeHost, Error> {
    picture::parse_insert_host(id, op_index, IdReason::InsertTableNeedsSlide)
}

pub fn parse_table_id(id: &str, json_path: &str, wrong: IdReason) -> Result<TableTarget, Error> {
    parse_table_rest(id, json_path, wrong)
}

pub fn parse_table_or_row(id: &str, json_path: &str) -> Result<TableOrRow, Error> {
    if let Some((left, row)) = split_suffix(id, "/row:") {
        let table = parse_table_id(left, json_path, IdReason::InsertRowNeedsTableOrRow)?;
        let row = parse_from_one(row, json_path, IdReason::SlideTableRowForm)?;
        return Ok(TableOrRow::Row { table, row });
    }
    if id.contains("/r:") {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::InsertRowNeedsTableOrRow,
            },
            json_path.to_string(),
        ));
    }
    Ok(TableOrRow::Table(parse_table_id(
        id,
        json_path,
        IdReason::InsertRowNeedsTableOrRow,
    )?))
}

pub fn parse_row_id(id: &str, json_path: &str) -> Result<(TableTarget, usize), Error> {
    let (left, row) = split_suffix(id, "/row:").ok_or_else(|| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::DeleteRowNeedsRow,
            },
            json_path.to_string(),
        )
    })?;
    let table = parse_table_id(left, json_path, IdReason::DeleteRowNeedsRow)?;
    let row = parse_from_one(row, json_path, IdReason::SlideTableRowForm)?;
    Ok((table, row))
}

pub fn parse_cell_id(id: &str, json_path: &str) -> Result<CellTarget, Error> {
    let path = json_path.to_string();
    let (left, after_r) = split_suffix(id, "/r:").ok_or_else(|| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SetTableCellNeedsCell,
            },
            path.clone(),
        )
    })?;
    let (row_s, col_s) = after_r.split_once("/c:").ok_or_else(|| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SlideTableCellForm,
            },
            path.clone(),
        )
    })?;
    let table = parse_table_id(left, json_path, IdReason::SetTableCellNeedsCell)?;
    let row = parse_from_one(row_s, json_path, IdReason::SlideTableCellForm)?;
    let col = parse_from_one(col_s, json_path, IdReason::SlideTableCellForm)?;
    Ok(CellTarget { table, row, col })
}

fn parse_table_rest(id: &str, json_path: &str, wrong: IdReason) -> Result<TableTarget, Error> {
    let path = json_path.to_string();
    let (host, after) =
        picture::parse_object_id(id, json_path, "table", IdReason::SlideTablePrefix)?;
    if after.is_empty() {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SlideTableContainer,
            },
            path,
        ));
    }
    let (slide, on_notes, master) = host.parts();
    if let Some(rest) = after.strip_prefix("index:") {
        if rest.contains('/') {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::SlideTablePrefix,
                },
                path,
            ));
        }
        let k: usize = rest.parse().map_err(|_| {
            Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::SlideTableIndexNotInt,
                },
                path.clone(),
            )
        })?;
        if k < 1 {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::SlideTableIndexFromOne,
                },
                path,
            ));
        }
        return Ok(TableTarget {
            slide,
            on_notes,
            master,
            key: TableKey::Index(k),
        });
    }
    if after.contains('/') {
        return Err(Error::with_path(
            ErrorKind::InvalidId { reason: wrong },
            path,
        ));
    }
    Ok(TableTarget {
        slide,
        on_notes,
        master,
        key: TableKey::Id(after.to_string()),
    })
}

fn parse_from_one(s: &str, json_path: &str, reason: IdReason) -> Result<usize, Error> {
    let n: usize = s.parse().map_err(|_| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: reason.clone(),
            },
            json_path.to_string(),
        )
    })?;
    if n < 1 {
        return Err(Error::with_path(
            ErrorKind::InvalidId { reason },
            json_path.to_string(),
        ));
    }
    Ok(n)
}

fn split_suffix<'a>(id: &'a str, sep: &str) -> Option<(&'a str, &'a str)> {
    id.rsplit_once(sep)
}

pub fn parse_dim(
    v: Option<&Value>,
    path: &str,
    field: &str,
    min: u64,
    max: u64,
) -> Result<usize, Error> {
    match v {
        Some(Value::Number(n)) => {
            let Some(u) = n.as_u64() else {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: field.into(),
                        expected: TypeName::Integer,
                    },
                    path.to_string(),
                ));
            };
            if u < min || u > max {
                return Err(Error::with_path(
                    ErrorKind::OutOfRange {
                        field: field.into(),
                        min: min as i64,
                        max: max as i64,
                    },
                    path.to_string(),
                ));
            }
            Ok(u as usize)
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Integer,
            },
            path.to_string(),
        )),
        None => Err(Error::with_path(
            ErrorKind::MissingField {
                field: field.into(),
            },
            path.to_string(),
        )),
    }
}

pub fn parse_after(v: Option<&Value>, i: usize) -> Result<u32, Error> {
    let path = format!("ops[{i}].after");
    match v {
        Some(Value::Number(n)) => {
            let Some(u) = n.as_u64() else {
                if n.as_i64().is_some_and(|x| x < 0) {
                    return Err(Error::with_path(
                        ErrorKind::MinValue {
                            field: "after".into(),
                            min: 0,
                        },
                        path,
                    ));
                }
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: "after".into(),
                        expected: TypeName::Integer,
                    },
                    path,
                ));
            };
            Ok(u as u32)
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "after".into(),
                expected: TypeName::Integer,
            },
            path,
        )),
        None => Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "insertCol".into(),
                field: "after".into(),
            },
            path,
        )),
    }
}

pub fn parse_cells(
    v: Option<&Value>,
    path: &str,
    rows: usize,
    cols: usize,
) -> Result<Vec<Vec<String>>, Error> {
    match v {
        None => Ok(vec![vec![String::new(); cols]; rows]),
        Some(Value::Array(arr)) => {
            if arr.len() != rows {
                return Err(Error::with_path(
                    ErrorKind::NeedTableCellsMatch,
                    path.to_string(),
                ));
            }
            let mut out = Vec::new();
            for (ri, row) in arr.iter().enumerate() {
                let Some(cells) = row.as_array() else {
                    return Err(Error::with_path(
                        ErrorKind::WrongType {
                            field: "cells".into(),
                            expected: TypeName::Array,
                        },
                        format!("{path}[{ri}]"),
                    ));
                };
                if cells.len() != cols {
                    return Err(Error::with_path(
                        ErrorKind::NeedTableCellsMatch,
                        format!("{path}[{ri}]"),
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
                                    expected: TypeName::String,
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
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "cells".into(),
                expected: TypeName::Array,
            },
            path.to_string(),
        )),
    }
}

pub fn max_rows() -> u64 {
    MAX_ROWS
}
pub fn max_cols() -> u64 {
    MAX_COLS
}

pub fn count_tables(root: &Element) -> usize {
    let mut n = 0usize;
    count_in(root, &mut n);
    n
}

fn count_in(el: &Element, n: &mut usize) {
    if el.local_name() == "AlternateContent" {
        if let Some(i) = shape::first_alt_child_index(el) {
            if let Node::Element(child) = &el.children[i] {
                count_in(child, n);
            }
        }
        return;
    }
    if is_table_frame(el) {
        *n += 1;
        return;
    }
    for child in &el.children {
        if let Node::Element(e) = child {
            count_in(e, n);
        }
    }
}

pub fn collect_tables(root: &Element, host: &str) -> Vec<Value> {
    let mut out = Vec::new();
    let mut index = 0usize;
    collect_in(root, host, &mut index, &mut out);
    out
}

fn collect_in(el: &Element, host: &str, index: &mut usize, out: &mut Vec<Value>) {
    if el.local_name() == "AlternateContent" {
        if let Some(i) = shape::first_alt_child_index(el) {
            if let Node::Element(child) = &el.children[i] {
                collect_in(child, host, index, out);
            }
        }
        return;
    }
    if is_table_frame(el) {
        *index += 1;
        out.push(table_json(el, host, *index));
        return;
    }
    for n in &el.children {
        if let Node::Element(e) = n {
            collect_in(e, host, index, out);
        }
    }
}

fn is_table_frame(el: &Element) -> bool {
    el.local_name() == "graphicFrame" && graphic_uri(el).is_some_and(|u| is_table_uri(&u))
}

fn is_table_uri(u: &str) -> bool {
    u.trim_end_matches('/') == TABLE_URI
}

fn graphic_uri(el: &Element) -> Option<String> {
    let mut found = None;
    shape::walk_choice(el, &mut |e| {
        if e.local_name() == "graphicData" && found.is_none() {
            found = e.attr_local("uri").map(str::to_string);
        }
    });
    found
}

fn cnv_of_frame(frame: &Element) -> (Option<String>, String) {
    let mut raw = None;
    let mut name = String::new();
    if let Some(nv) = frame.child_by_local("nvGraphicFramePr") {
        if let Some(pr) = nv.child_by_local("cNvPr") {
            raw = pr.attr_local("id").map(str::to_string);
            name = pr.attr_local("name").unwrap_or("").to_string();
        }
    }
    (raw, name)
}

fn parse_i64_attr(el: &Element, name: &str) -> Option<i64> {
    el.attr_local(name).and_then(|s| s.parse().ok())
}

fn frame_xfrm(frame: &Element) -> Option<&Element> {
    frame.child_by_local("xfrm")
}

fn find_tbl(frame: &Element) -> Option<&Element> {
    find_tbl_in(frame)
}

fn find_tbl_in(el: &Element) -> Option<&Element> {
    if el.local_name() == "AlternateContent" {
        if let Some(i) = shape::first_alt_child_index(el) {
            if let Node::Element(child) = &el.children[i] {
                return find_tbl_in(child);
            }
        }
        return None;
    }
    if el.local_name() == "tbl" {
        return Some(el);
    }
    for n in &el.children {
        if let Node::Element(e) = n {
            if let Some(t) = find_tbl_in(e) {
                return Some(t);
            }
        }
    }
    None
}

fn find_tbl_mut(frame: &mut Element) -> Option<&mut Element> {
    find_tbl_mut_in(frame)
}

fn find_tbl_mut_in(el: &mut Element) -> Option<&mut Element> {
    if el.local_name() == "AlternateContent" {
        if let Some(i) = shape::first_alt_child_index(el) {
            if let Node::Element(child) = &mut el.children[i] {
                return find_tbl_mut_in(child);
            }
        }
        return None;
    }
    if el.local_name() == "tbl" {
        return Some(el);
    }
    for child in el.children.as_mut_slice() {
        if let Node::Element(e) = child {
            if let Some(t) = find_tbl_mut_in(e) {
                return Some(t);
            }
        }
    }
    None
}

fn col_count(tbl: &Element) -> usize {
    if let Some(grid) = tbl.child_by_local("tblGrid") {
        let n = grid.children_by_local("gridCol").len();
        if n > 0 {
            return n;
        }
    }
    tbl.children_by_local("tr")
        .first()
        .map(|tr| tr.children_by_local("tc").len())
        .unwrap_or(0)
}

fn is_on(el: &Element, name: &str) -> bool {
    matches!(
        el.attr_local(name).map(|s| s.to_ascii_lowercase()),
        Some(s) if s == "1" || s == "true" || s == "on"
    )
}

fn grid_span(tc: &Element) -> usize {
    tc.attr_local("gridSpan")
        .and_then(|s| s.parse().ok())
        .filter(|&n| n >= 1)
        .unwrap_or(1)
}

fn row_span(tc: &Element) -> usize {
    tc.attr_local("rowSpan")
        .and_then(|s| s.parse().ok())
        .filter(|&n| n >= 1)
        .unwrap_or(1)
}

fn table_json(frame: &Element, host: &str, index: usize) -> Value {
    let (raw_id, name) = cnv_of_frame(frame);
    let stable = raw_id.is_some();
    let id = match &raw_id {
        Some(v) => format!("{host}/table:{v}"),
        None => format!("{host}/table:index:{index}"),
    };
    let xfrm = frame_xfrm(frame);
    let (x, y) = match xfrm.and_then(|x| x.child_by_local("off")) {
        Some(off) => (parse_i64_attr(off, "x"), parse_i64_attr(off, "y")),
        None => (None, None),
    };
    let (cx, cy) = match xfrm.and_then(|x| x.child_by_local("ext")) {
        Some(ext) => (parse_i64_attr(ext, "cx"), parse_i64_attr(ext, "cy")),
        None => (None, None),
    };
    let tbl = find_tbl(frame);
    let trs = tbl.map(|t| t.children_by_local("tr")).unwrap_or_default();
    let cols = tbl.map(col_count).unwrap_or(0);
    let mut rows = Vec::new();
    for (ri, tr) in trs.iter().enumerate() {
        let row_n = ri + 1;
        let mut cells = Vec::new();
        for (col, tc) in (1usize..).zip(tr.children_by_local("tc")) {
            let span = grid_span(tc);
            let rspan = row_span(tc);
            let h_merge = is_on(tc, "hMerge");
            let v_merge = is_on(tc, "vMerge");
            let text = tc
                .child_by_local("txBody")
                .map(shape::txbody_visible_text)
                .unwrap_or_default();
            let mut cell = Map::new();
            cell.insert("id".into(), json!(format!("{id}/r:{row_n}/c:{col}")));
            cell.insert("row".into(), json!(row_n));
            cell.insert("col".into(), json!(col));
            cell.insert("gridSpan".into(), json!(span as i64));
            cell.insert("rowSpan".into(), json!(rspan as i64));
            cell.insert("hMerge".into(), json!(h_merge));
            cell.insert("vMerge".into(), json!(v_merge));
            cell.insert("text".into(), json!(text));
            crate::drawing_text::insert_inspect(&mut cell, tc.child_by_local("txBody"));
            cells.push(Value::Object(cell));
        }
        rows.push(json!({
            "id": format!("{id}/row:{row_n}"),
            "index": row_n,
            "cells": cells,
        }));
    }
    let mut m = Map::new();
    m.insert("id".into(), json!(id));
    m.insert(
        "cNvPrId".into(),
        match raw_id {
            Some(v) => json!(v),
            None => Value::Null,
        },
    );
    m.insert("name".into(), json!(name));
    m.insert("x".into(), json!(x));
    m.insert("y".into(), json!(y));
    m.insert("cx".into(), json!(cx));
    m.insert("cy".into(), json!(cy));
    m.insert("rowCount".into(), json!(trs.len()));
    m.insert("colCount".into(), json!(cols));
    m.insert("rows".into(), json!(rows));
    m.insert("stable".into(), json!(stable));
    Value::Object(m)
}

fn key_matches(frame: &Element, index: usize, key: &TableKey) -> bool {
    match key {
        TableKey::Id(want) => cnv_of_frame(frame)
            .0
            .as_deref()
            .is_some_and(|id| ids_match(id, want)),
        TableKey::Index(k) => index == *k,
    }
}

fn ids_match(file: &str, req: &str) -> bool {
    match (file.parse::<u64>(), req.parse::<u64>()) {
        (Ok(a), Ok(b)) => a == b,
        _ => file == req,
    }
}

fn find_frame_mut<'a>(root: &'a mut Element, key: &TableKey) -> Option<&'a mut Element> {
    let mut idx = 0usize;
    find_frame_in(root, &mut idx, key)
}

fn find_frame_in<'a>(
    el: &'a mut Element,
    idx: &mut usize,
    key: &TableKey,
) -> Option<&'a mut Element> {
    if el.local_name() == "AlternateContent" {
        if let Some(i) = shape::first_alt_child_index(el) {
            if let Node::Element(child) = &mut el.children[i] {
                return find_frame_in(child, idx, key);
            }
        }
        return None;
    }
    if is_table_frame(el) {
        *idx += 1;
        if key_matches(el, *idx, key) {
            return Some(el);
        }
        return None;
    }
    for child in el.children.as_mut_slice() {
        if let Node::Element(e) = child {
            if let Some(found) = find_frame_in(e, idx, key) {
                return Some(found);
            }
        }
    }
    None
}

fn missing_table(id: &str, i: usize) -> Error {
    Error::with_path(
        ErrorKind::TableNotFound { id: id.to_string() },
        format!("ops[{i}].id"),
    )
}

fn reject_grid(tbl: &Element, i: usize) -> Result<(), Error> {
    let want = col_count(tbl);
    for tr in tbl.children_by_local("tr") {
        if tr.children_by_local("tc").len() != want {
            return Err(Error::with_path(
                ErrorKind::TableGridMismatch,
                format!("ops[{i}].id"),
            ));
        }
    }
    Ok(())
}

pub fn insert_table(
    slide_doc: &mut Document,
    pres_root: &Element,
    op: &InsertTableOp,
    i: usize,
) -> Result<(), Error> {
    let cx = op.cx.unwrap_or(DEFAULT_CX);
    let cy = op.cy.unwrap_or(DEFAULT_CY);
    let (x, y) = match (op.x, op.y) {
        (Some(px), Some(py)) => (px, py),
        _ => {
            if op.on_notes {
                picture::vacant_pos_canvas(
                    super::notes::notes_size(pres_root),
                    &slide_doc.root,
                    cx,
                    cy,
                )
            } else {
                picture::vacant_pos(pres_root, &slide_doc.root, cx, cy)
            }
        }
    };
    picture::ensure_xmlns(&mut slide_doc.root, "a", DML_NS);
    let cnv = picture::next_cnv_id(&slide_doc.root);
    let frame = build_table_frame(cnv, x, y, cx, cy, op.rows, op.cols, &op.cells);
    picture::insert_into_sp_tree(&mut slide_doc.root, frame)?;
    let _ = i;
    Ok(())
}

fn build_table_frame(
    cnv: u32,
    x: i64,
    y: i64,
    cx: i64,
    cy: i64,
    rows: usize,
    cols: usize,
    cells: &[Vec<String>],
) -> Element {
    let mut nv = Element::new("p:nvGraphicFramePr");
    nv.push_element(
        Element::new("p:cNvPr")
            .with_attr("id", cnv.to_string())
            .with_attr("name", format!("表格 {cnv}")),
    );
    let mut locks = Element::new("p:cNvGraphicFramePr");
    locks.push_element(Element::new("a:graphicFrameLocks").with_attr("noGrp", "1"));
    nv.push_element(locks);
    nv.push_element(Element::new("p:nvPr"));
    let mut xfrm = Element::new("p:xfrm");
    xfrm.push_element(
        Element::new("a:off")
            .with_attr("x", x.to_string())
            .with_attr("y", y.to_string()),
    );
    xfrm.push_element(
        Element::new("a:ext")
            .with_attr("cx", cx.to_string())
            .with_attr("cy", cy.to_string()),
    );
    let col_ws = split_emu(cx, cols);
    let row_hs = split_emu(cy, rows);
    let mut grid = Element::new("a:tblGrid");
    for w in &col_ws {
        grid.push_element(Element::new("a:gridCol").with_attr("w", w.to_string()));
    }
    let mut tbl = Element::new("a:tbl");
    let mut tbl_pr = Element::new("a:tblPr")
        .with_attr("firstRow", "1")
        .with_attr("bandRow", "1");
    tbl_pr.push_element(Element::new("a:tableStyleId").with_text(STYLE_ID));
    tbl.push_element(tbl_pr);
    tbl.push_element(grid);
    for (r, h) in row_hs.iter().enumerate() {
        let mut tr = Element::new("a:tr").with_attr("h", h.to_string());
        for c in 0..cols {
            let text = cells
                .get(r)
                .and_then(|row| row.get(c))
                .map(|s| s.as_str())
                .unwrap_or("");
            tr.push_element(build_tc(text));
        }
        tbl.push_element(tr);
    }
    let mut data = Element::new("a:graphicData").with_attr("uri", TABLE_URI);
    data.push_element(tbl);
    let mut graphic = Element::new("a:graphic");
    graphic.push_element(data);
    let mut frame = Element::new("p:graphicFrame");
    frame.push_element(nv);
    frame.push_element(xfrm);
    frame.push_element(graphic);
    frame
}

fn split_emu(total: i64, n: usize) -> Vec<i64> {
    if n == 0 {
        return Vec::new();
    }
    let n_i = n as i64;
    let base = total / n_i;
    let rem = total - base * n_i;
    let mut out = vec![base; n];
    if let Some(last) = out.last_mut() {
        *last += rem;
    }
    out
}

fn build_tc(text: &str) -> Element {
    let mut tc = Element::new("a:tc");
    let mut tx = empty_tx_body();
    if !text.is_empty() {
        shape::set_txbody_text(&mut tx, text);
    }
    tc.push_element(tx);
    tc.push_element(Element::new("a:tcPr"));
    tc
}

fn empty_tx_body() -> Element {
    let mut tx = Element::new("a:txBody");
    tx.push_element(Element::new("a:bodyPr"));
    tx.push_element(Element::new("a:lstStyle"));
    let mut p = Element::new("a:p");
    p.push_element(Element::new("a:endParaRPr").with_attr("dirty", "0"));
    tx.push_element(p);
    tx
}

fn empty_hmerge_tc() -> Element {
    let mut tc = Element::new("a:tc").with_attr("hMerge", "1");
    tc.push_element(empty_tx_body());
    tc.push_element(Element::new("a:tcPr"));
    tc
}

pub fn delete_table(slide_doc: &mut Document, op: &DeleteTableOp, i: usize) -> Result<(), Error> {
    let mut idx = 0usize;
    let mut spid = None;
    if !remove_in(&mut slide_doc.root, &mut idx, &op.target.key, &mut spid) {
        return Err(missing_table(&op.id, i));
    }
    prune_empty_groups(&mut slide_doc.root);
    if let Some(id) = spid {
        shape::strip_timing_for_spid(&mut slide_doc.root, &id);
    }
    Ok(())
}

fn remove_in(el: &mut Element, idx: &mut usize, key: &TableKey, spid: &mut Option<String>) -> bool {
    let mut i = 0;
    while i < el.children.len() {
        let is_ac =
            matches!(&el.children[i], Node::Element(e) if e.local_name() == "AlternateContent");
        if is_ac {
            let mut found = false;
            let mut hit_id = None;
            if let Node::Element(ac) = &el.children[i] {
                peek_choice(ac, idx, key, &mut found, &mut hit_id);
            }
            if found {
                *spid = hit_id;
                el.children.remove(i);
                return true;
            }
            i += 1;
            continue;
        }
        let is_hit = match &el.children[i] {
            Node::Element(e) if is_table_frame(e) => {
                *idx += 1;
                if key_matches(e, *idx, key) {
                    *spid = cnv_of_frame(e).0;
                    true
                } else {
                    false
                }
            }
            _ => false,
        };
        if is_hit {
            el.children.remove(i);
            return true;
        }
        if let Node::Element(e) = &mut el.children[i] {
            if !is_table_frame(e) && remove_in(e, idx, key, spid) {
                return true;
            }
        }
        i += 1;
    }
    false
}

fn peek_choice(
    ac: &Element,
    idx: &mut usize,
    key: &TableKey,
    found: &mut bool,
    hit_id: &mut Option<String>,
) {
    if let Some(i) = shape::first_alt_child_index(ac) {
        if let Node::Element(child) = &ac.children[i] {
            mark_if_match(child, idx, key, found, hit_id);
        }
    }
}

fn mark_if_match(
    el: &Element,
    idx: &mut usize,
    key: &TableKey,
    found: &mut bool,
    hit_id: &mut Option<String>,
) {
    if el.local_name() == "AlternateContent" {
        peek_choice(el, idx, key, found, hit_id);
        return;
    }
    if is_table_frame(el) {
        *idx += 1;
        if key_matches(el, *idx, key) {
            *found = true;
            *hit_id = cnv_of_frame(el).0;
        }
        return;
    }
    for n in &el.children {
        if let Node::Element(e) = n {
            mark_if_match(e, idx, key, found, hit_id);
        }
    }
}

fn prune_empty_groups(el: &mut Element) {
    for n in &mut el.children {
        if let Node::Element(e) = n {
            prune_empty_groups(e);
        }
    }
    el.children.retain(
        |n| !matches!(n, Node::Element(e) if e.local_name() == "grpSp" && !grp_has_member(e)),
    );
}

fn grp_has_member(grp: &Element) -> bool {
    grp.element_children().any(|c| {
        matches!(
            c.local_name(),
            "sp" | "pic" | "cxnSp" | "graphicFrame" | "grpSp" | "AlternateContent"
        )
    })
}

fn require_tbl<'a>(
    slide_doc: &'a mut Document,
    key: &TableKey,
    id: &str,
    i: usize,
) -> Result<&'a mut Element, Error> {
    let frame = find_frame_mut(&mut slide_doc.root, key).ok_or_else(|| missing_table(id, i))?;
    find_tbl_mut(frame).ok_or_else(|| missing_table(id, i))
}

pub fn insert_row(slide_doc: &mut Document, op: &InsertRowOp, i: usize) -> Result<(), Error> {
    let (key, after_row) = match &op.target {
        TableOrRow::Table(t) => (&t.key, None),
        TableOrRow::Row { table, row } => (&table.key, Some(*row)),
    };
    let tbl = require_tbl(slide_doc, key, &op.id, i)?;
    reject_grid(tbl, i)?;
    let tr_pos = tr_child_indices(tbl);
    if tr_pos.is_empty() {
        return Err(missing_table(&op.id, i));
    }
    let ref_i = match after_row {
        None => tr_pos.len() - 1,
        Some(r) => {
            if r < 1 || r > tr_pos.len() {
                return Err(missing_table(&op.id, i));
            }
            r - 1
        }
    };
    let ref_child = tr_pos[ref_i];
    let proto = match &tbl.children[ref_child] {
        Node::Element(e) => e.clone(),
        _ => return Err(missing_table(&op.id, i)),
    };
    let neu = clone_row_layout(&proto);
    let insert_at = ref_child + 1;
    tbl.children.insert(insert_at, Node::Element(neu));
    Ok(())
}

fn tr_child_indices(tbl: &Element) -> Vec<usize> {
    tbl.children
        .iter()
        .enumerate()
        .filter_map(|(i, n)| match n {
            Node::Element(e) if e.local_name() == "tr" => Some(i),
            _ => None,
        })
        .collect()
}

fn clone_row_layout(src: &Element) -> Element {
    let mut tr = Element::new(src.name.clone());
    if let Some(h) = src.attr_local("h") {
        tr.set_attr("h", h);
    }
    for n in &src.children {
        match n {
            Node::Element(e) if e.local_name() == "tc" => {
                tr.push_element(clone_tc_layout(e));
            }
            Node::Element(e) => tr.push_element(e.clone()),
            Node::Text(t) => tr.children.push(Node::Text(t.clone())),
        }
    }
    tr
}

fn clone_tc_layout(src: &Element) -> Element {
    let mut tc = Element::new(src.name.clone());
    for (k, v) in &src.attrs {
        let local = k.rsplit(':').next().unwrap_or(k);
        if local == "vMerge" || local == "rowSpan" {
            continue;
        }
        tc.attrs.push((k.clone(), v.clone()));
    }
    tc.push_element(empty_tx_body());
    if let Some(pr) = src.child_by_local("tcPr") {
        tc.push_element(pr.clone());
    } else {
        tc.push_element(Element::new("a:tcPr"));
    }
    tc
}

pub fn delete_row(slide_doc: &mut Document, op: &DeleteRowOp, i: usize) -> Result<(), Error> {
    let tbl = require_tbl(slide_doc, &op.table.key, &op.id, i)?;
    let tr_pos = tr_child_indices(tbl);
    if op.row < 1 || op.row > tr_pos.len() {
        return Err(missing_table(&op.id, i));
    }
    if tr_pos.len() <= 1 {
        return Err(Error::with_path(
            ErrorKind::LastTableRow,
            format!("ops[{i}].id"),
        ));
    }
    let child_i = tr_pos[op.row - 1];
    let tr = match &tbl.children[child_i] {
        Node::Element(e) => e,
        _ => return Err(missing_table(&op.id, i)),
    };
    for tc in tr.children_by_local("tc") {
        if row_span(tc) > 1 || is_on(tc, "vMerge") {
            return Err(Error::with_path(
                ErrorKind::TableGridMismatch,
                format!("ops[{i}].id"),
            ));
        }
    }
    tbl.children.remove(child_i);
    Ok(())
}

pub fn insert_col(slide_doc: &mut Document, op: &InsertColOp, i: usize) -> Result<(), Error> {
    let tbl = require_tbl(slide_doc, &op.target.key, &op.id, i)?;
    reject_grid(tbl, i)?;
    let n = col_count(tbl);
    let after = op.after as usize;
    if after > n {
        return Err(Error::with_path(
            ErrorKind::OutOfRange {
                field: "after".into(),
                min: 0,
                max: n as i64,
            },
            format!("ops[{i}].after"),
        ));
    }
    let w = grid_width_at(tbl, if after == 0 { 0 } else { after - 1 }).unwrap_or(914_400);
    insert_grid_col(tbl, after, w);
    let tr_pos = tr_child_indices(tbl);
    for ti in tr_pos {
        let tr = match &mut tbl.children[ti] {
            Node::Element(e) => e,
            _ => continue,
        };
        insert_tc_in_row(tr, after);
    }
    Ok(())
}

fn grid_width_at(tbl: &Element, idx: usize) -> Option<i64> {
    let grid = tbl.child_by_local("tblGrid")?;
    let cols = grid.children_by_local("gridCol");
    cols.get(idx)
        .and_then(|c| c.attr_local("w"))
        .and_then(|s| s.parse().ok())
}

fn insert_grid_col(tbl: &mut Element, at: usize, w: i64) {
    if tbl.child_by_local("tblGrid").is_none() {
        let mut i = 0;
        while i < tbl.children.len() {
            if let Node::Element(e) = &tbl.children[i] {
                if e.local_name() == "tblPr" {
                    i += 1;
                    continue;
                }
            }
            break;
        }
        tbl.children
            .insert(i, Node::Element(Element::new("a:tblGrid")));
    }
    let grid = tbl.child_by_local_mut("tblGrid").expect("刚补的 tblGrid");
    let mut col_i = 0usize;
    let mut insert_at = grid.children.len();
    for (i, n) in grid.children.iter().enumerate() {
        if let Node::Element(e) = n {
            if e.local_name() == "gridCol" {
                if col_i == at {
                    insert_at = i;
                    break;
                }
                col_i += 1;
            }
        }
    }
    if col_i < at {
        insert_at = grid.children.len();
    }
    grid.children.insert(
        insert_at,
        Node::Element(Element::new("a:gridCol").with_attr("w", w.to_string())),
    );
}

fn insert_tc_in_row(tr: &mut Element, after: usize) {
    let tcs = tc_child_indices(tr);
    let mut c = 1usize;
    for &child_i in &tcs {
        let Node::Element(tc) = &tr.children[child_i] else {
            continue;
        };
        if !is_on(tc, "hMerge") {
            let span = grid_span(tc);
            if c < after + 1 && after + 1 < c + span {
                let name = tc.name.clone();
                if let Node::Element(tc) = &mut tr.children[child_i] {
                    picture::set_attr_local(tc, "gridSpan", &(span + 1).to_string());
                }
                let insert_at = tc_insert_index(tr, after);
                let mut neu = empty_hmerge_tc();
                neu.name = name;
                tr.children.insert(insert_at, Node::Element(neu));
                return;
            }
        }
        c += 1;
    }
    let insert_at = tc_insert_index(tr, after);
    tr.children.insert(insert_at, Node::Element(build_tc("")));
}

fn tc_child_indices(tr: &Element) -> Vec<usize> {
    tr.children
        .iter()
        .enumerate()
        .filter_map(|(i, n)| match n {
            Node::Element(e) if e.local_name() == "tc" => Some(i),
            _ => None,
        })
        .collect()
}

fn tc_insert_index(tr: &Element, after: usize) -> usize {
    let tcs = tc_child_indices(tr);
    if after >= tcs.len() {
        tcs.last().map(|i| i + 1).unwrap_or(tr.children.len())
    } else {
        tcs[after]
    }
}

pub fn delete_col(slide_doc: &mut Document, op: &DeleteColOp, i: usize) -> Result<(), Error> {
    let tbl = require_tbl(slide_doc, &op.target.key, &op.id, i)?;
    reject_grid(tbl, i)?;
    let n = col_count(tbl);
    let col = op.col as usize;
    if col < 1 || col > n {
        return Err(Error::with_path(
            ErrorKind::OutOfRange {
                field: "col".into(),
                min: 1,
                max: n as i64,
            },
            format!("ops[{i}].col"),
        ));
    }
    if n <= 1 {
        return Err(Error::with_path(
            ErrorKind::LastTableCol,
            format!("ops[{i}].id"),
        ));
    }
    remove_grid_col(tbl, col - 1);
    let tr_pos = tr_child_indices(tbl);
    for ti in tr_pos {
        let tr = match &mut tbl.children[ti] {
            Node::Element(e) => e,
            _ => continue,
        };
        delete_tc_in_row(tr, col);
    }
    Ok(())
}

fn remove_grid_col(tbl: &mut Element, idx: usize) {
    let Some(grid) = tbl.child_by_local_mut("tblGrid") else {
        return;
    };
    let mut seen = 0usize;
    let mut drop = None;
    for (i, n) in grid.children.iter().enumerate() {
        if let Node::Element(e) = n {
            if e.local_name() == "gridCol" {
                if seen == idx {
                    drop = Some(i);
                    break;
                }
                seen += 1;
            }
        }
    }
    if let Some(i) = drop {
        grid.children.remove(i);
    }
}

fn delete_tc_in_row(tr: &mut Element, col: usize) {
    let tcs = tc_child_indices(tr);
    let Some(&child_i) = tcs.get(col - 1) else {
        return;
    };
    let (h_merge, span) = match &tr.children[child_i] {
        Node::Element(tc) => (is_on(tc, "hMerge"), grid_span(tc)),
        _ => return,
    };
    if h_merge {
        if let Some(owner) = owner_start_index(&tcs, tr, col - 1) {
            if let Node::Element(tc) = &mut tr.children[tcs[owner]] {
                let s = grid_span(tc).saturating_sub(1).max(1);
                if s == 1 {
                    remove_attr_local(tc, "gridSpan");
                } else {
                    picture::set_attr_local(tc, "gridSpan", &s.to_string());
                }
            }
        }
        tr.children.remove(child_i);
        return;
    }
    if span > 1 {
        if let Node::Element(tc) = &mut tr.children[child_i] {
            let s = span - 1;
            if s == 1 {
                remove_attr_local(tc, "gridSpan");
            } else {
                picture::set_attr_local(tc, "gridSpan", &s.to_string());
            }
        }
        if let Some(&next_i) = tcs.get(col) {
            if let Node::Element(next) = &tr.children[next_i] {
                if is_on(next, "hMerge") {
                    tr.children.remove(next_i);
                    return;
                }
            }
        }
        return;
    }
    tr.children.remove(child_i);
}

fn owner_start_index(tcs: &[usize], tr: &Element, hmerge_k: usize) -> Option<usize> {
    let mut k = hmerge_k;
    while k > 0 {
        k -= 1;
        let Node::Element(tc) = &tr.children[tcs[k]] else {
            continue;
        };
        if !is_on(tc, "hMerge") {
            return Some(k);
        }
    }
    None
}

fn remove_attr_local(el: &mut Element, local: &str) {
    el.attrs
        .retain(|(k, _)| k.rsplit(':').next().unwrap_or(k) != local);
}

pub fn set_cell_text(
    slide_doc: &mut Document,
    op: &SetTableCellTextOp,
    i: usize,
) -> Result<(), Error> {
    let tbl = require_tbl(slide_doc, &op.target.table.key, &op.id, i)?;
    let tr_pos = tr_child_indices(tbl);
    if op.target.row < 1 || op.target.row > tr_pos.len() {
        return Err(missing_table(&op.id, i));
    }
    let tr = match &mut tbl.children[tr_pos[op.target.row - 1]] {
        Node::Element(e) => e,
        _ => return Err(missing_table(&op.id, i)),
    };
    let tcs = tc_child_indices(tr);
    if op.target.col < 1 || op.target.col > tcs.len() {
        return Err(missing_table(&op.id, i));
    }
    let tc = match &mut tr.children[tcs[op.target.col - 1]] {
        Node::Element(e) => e,
        _ => return Err(missing_table(&op.id, i)),
    };
    let tx = tc
        .child_by_local_mut("txBody")
        .ok_or_else(|| missing_table(&op.id, i))?;
    shape::set_txbody_text(tx, &op.text);
    Ok(())
}

pub fn set_cell_format(
    slide_doc: &mut Document,
    id: &str,
    target: &CellTarget,
    fmt: &crate::drawing_text::TextFormat,
    i: usize,
) -> Result<(), Error> {
    let tbl = require_tbl(slide_doc, &target.table.key, id, i)?;
    let tr_pos = tr_child_indices(tbl);
    if target.row < 1 || target.row > tr_pos.len() {
        return Err(missing_table(id, i));
    }
    let tr = match &mut tbl.children[tr_pos[target.row - 1]] {
        Node::Element(e) => e,
        _ => return Err(missing_table(id, i)),
    };
    let tcs = tc_child_indices(tr);
    if target.col < 1 || target.col > tcs.len() {
        return Err(missing_table(id, i));
    }
    let tc = match &mut tr.children[tcs[target.col - 1]] {
        Node::Element(e) => e,
        _ => return Err(missing_table(id, i)),
    };
    let tx = tc
        .child_by_local_mut("txBody")
        .ok_or_else(|| missing_table(id, i))?;
    crate::drawing_text::apply(tx, fmt);
    Ok(())
}

pub fn apply_pos(slide_doc: &mut Document, op: &SetTablePosOp, i: usize) -> Result<(), Error> {
    let frame = find_frame_mut(&mut slide_doc.root, &op.target.key)
        .ok_or_else(|| missing_table(&op.id, i))?;
    let xfrm = ensure_frame_xfrm(frame);
    let off = ensure_a(xfrm, "off");
    if let Some(x) = op.x {
        picture::set_attr_local(off, "x", &x.to_string());
    }
    if let Some(y) = op.y {
        picture::set_attr_local(off, "y", &y.to_string());
    }
    Ok(())
}

pub fn apply_size(slide_doc: &mut Document, op: &SetTableSizeOp, i: usize) -> Result<(), Error> {
    let frame = find_frame_mut(&mut slide_doc.root, &op.target.key)
        .ok_or_else(|| missing_table(&op.id, i))?;
    let xfrm = ensure_frame_xfrm(frame);
    let ext = ensure_a(xfrm, "ext");
    if let Some(cx) = op.cx {
        picture::set_attr_local(ext, "cx", &cx.to_string());
    }
    if let Some(cy) = op.cy {
        picture::set_attr_local(ext, "cy", &cy.to_string());
    }
    Ok(())
}

fn ensure_frame_xfrm(frame: &mut Element) -> &mut Element {
    if frame.child_by_local("xfrm").is_none() {
        frame.push_element(Element::new("p:xfrm"));
    }
    frame.child_by_local_mut("xfrm").expect("刚插入的 p:xfrm")
}

fn ensure_a<'a>(parent: &'a mut Element, local: &str) -> &'a mut Element {
    if parent.child_by_local(local).is_none() {
        parent.push_element(Element::new(format!("a:{local}")));
    }
    parent.child_by_local_mut(local).expect("刚插入的 a 子元素")
}
