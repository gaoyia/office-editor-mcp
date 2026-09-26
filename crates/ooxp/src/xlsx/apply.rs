//! 按 id 改单元格或工作表图，改单元格时标记打开时重算。

use super::request::{SetCellOp, SetCellValue, XlsxOp};
use crate::error::{Error, ErrorKind, IdReason, RelSource};
use crate::opc::{self, ContentTypes, Package, Relationships};
use crate::xml::{self, Document, Element, Node};

use super::a1;
use super::chart;
use super::picture;
use super::REL_CALC_CHAIN;

pub fn apply(bytes: &[u8], ops: &[XlsxOp]) -> Result<Vec<u8>, Error> {
    let mut pkg = Package::open_bytes(bytes.to_vec())?;
    let wb_part = opc::office_document_part(&pkg)?;
    let mut wb_doc = xml::parse(pkg.get(&wb_part)?)?;
    let wb_rels = Relationships::from_package(&pkg, &wb_part)?;

    let sheet_index = sheet_index(&wb_doc.root, &wb_rels, &wb_part)?;

    let mut dirty_sheets: Vec<(String, Document)> = Vec::new();
    let mut touched_cells = false;

    for (i, op) in ops.iter().enumerate() {
        match op {
            XlsxOp::SetCell(op) => {
                touched_cells = true;
                let (sheet_name, a1) = parse_cell_id(&op.id, i)?;
                let meta = sheet_index
                    .iter()
                    .find(|(n, _, k)| n.eq_ignore_ascii_case(&sheet_name) && *k == "worksheet")
                    .ok_or_else(|| {
                        if sheet_index
                            .iter()
                            .any(|(n, _, _)| n.eq_ignore_ascii_case(&sheet_name))
                        {
                            Error::with_path(ErrorKind::SetCellWorksheetOnly, format!("ops[{i}].id"))
                        } else {
                            Error::with_path(
                                ErrorKind::SheetNotFound {
                                    name: sheet_name.clone(),
                                },
                                format!("ops[{i}].id"),
                            )
                        }
                    })?;
                let part = meta.1.clone();
                let cell = a1::parse_cell(&a1)?;
                load_dirty(&pkg, &mut dirty_sheets, &part, true)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                apply_set_cell(&mut doc.root, cell, op, i)?;
            }
            XlsxOp::ReplacePicture(op) => {
                let part = require_sheet_part(&sheet_index, &op.target.sheet, i)?;
                picture::replace_picture(&mut pkg, &part, op, i)?;
            }
            XlsxOp::InsertPicture(op) => {
                let (part, kind) = require_named_sheet(&sheet_index, &op.sheet, i)?;
                if kind != "worksheet" {
                    return Err(Error::with_path(
                        ErrorKind::InsertPictureWorksheetOnly,
                        format!("ops[{i}].id"),
                    ));
                }
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                picture::insert_picture(&mut pkg, doc, &part, op, i)?;
            }
            XlsxOp::DeletePicture(op) => {
                let part = require_sheet_part(&sheet_index, &op.target.sheet, i)?;
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                picture::delete_picture(&mut pkg, doc, &part, op, i)?;
            }
            XlsxOp::SetPictureSize(op) => {
                let part = require_sheet_part(&sheet_index, &op.target.sheet, i)?;
                picture::set_picture_size(&mut pkg, &part, op, i)?;
            }
            XlsxOp::SetPictureAnchor(op) => {
                let part = require_sheet_part(&sheet_index, &op.target.sheet, i)?;
                picture::set_picture_anchor(&mut pkg, &part, op, i)?;
            }
            XlsxOp::InsertChart(op) => {
                let (part, kind) = require_named_sheet(&sheet_index, &op.sheet, i)?;
                if kind != "worksheet" {
                    return Err(Error::with_path(
                        ErrorKind::InsertChartWorksheetOnly,
                        format!("ops[{i}].id"),
                    ));
                }
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                chart::insert_chart(&mut pkg, &part, &op.sheet, &mut dirty_sheets, op, i)?;
                touched_cells = true;
            }
            XlsxOp::DeleteChart(op) => {
                let (part, kind) = require_named_sheet(&sheet_index, &op.target.sheet, i)?;
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                chart::delete_chart(&mut pkg, doc, &part, kind, op, i)?;
            }
            XlsxOp::SetChartTitle(op) => {
                let part = require_sheet_part(&sheet_index, &op.target.sheet, i)?;
                chart::set_chart_title(&mut pkg, &part, op, i)?;
            }
            XlsxOp::SetChartSeries(op) => {
                let part = require_sheet_part(&sheet_index, &op.target.sheet, i)?;
                if chart::set_chart_series(
                    &mut pkg,
                    &part,
                    &op.target.sheet,
                    &sheet_index,
                    &mut dirty_sheets,
                    op,
                    i,
                )? {
                    touched_cells = true;
                }
            }
            XlsxOp::AddChartSeries(op) => {
                let part = require_sheet_part(&sheet_index, &op.target.sheet, i)?;
                if chart::add_chart_series(
                    &mut pkg,
                    &part,
                    &op.target.sheet,
                    &sheet_index,
                    &mut dirty_sheets,
                    op,
                    i,
                )? {
                    touched_cells = true;
                }
            }
            XlsxOp::DeleteChartSeries(op) => {
                let part = require_sheet_part(&sheet_index, &op.target.sheet, i)?;
                chart::delete_chart_series(&mut pkg, &part, op, i)?;
            }
            XlsxOp::SetChartType(op) => {
                let part = require_sheet_part(&sheet_index, &op.target.sheet, i)?;
                chart::set_chart_type(&mut pkg, &part, op, i)?;
            }
            XlsxOp::SetChartAxisTitle(op) => {
                let part = require_sheet_part(&sheet_index, &op.target.sheet, i)?;
                chart::set_chart_axis_title(&mut pkg, &part, op, i)?;
            }
            XlsxOp::SetChartLegend(op) => {
                let part = require_sheet_part(&sheet_index, &op.target.sheet, i)?;
                chart::set_chart_legend(&mut pkg, &part, op, i)?;
            }
            XlsxOp::SetChartDataLabels(op) => {
                let part = require_sheet_part(&sheet_index, &op.target.sheet, i)?;
                chart::set_chart_data_labels(&mut pkg, &part, op, i)?;
            }
            XlsxOp::SetChartSize(op) => {
                let part = require_sheet_part(&sheet_index, &op.target.sheet, i)?;
                chart::set_chart_size(&mut pkg, &part, op, i)?;
            }
            XlsxOp::SetChartAnchor(op) => {
                let part = require_sheet_part(&sheet_index, &op.target.sheet, i)?;
                chart::set_chart_anchor(&mut pkg, &part, op, i)?;
            }
        }
    }

    for (part, doc) in dirty_sheets {
        pkg.set(&part, doc.to_bytes()?)?;
    }

    if touched_cells {
        mark_full_calc(&mut wb_doc.root);
        pkg.set(&wb_part, wb_doc.to_bytes()?)?;
        remove_calc_chain(&mut pkg, &wb_part)?;
    }

    pkg.save_to_vec()
}

fn require_named_sheet(
    index: &[(String, String, &'static str)],
    name: &str,
    op_index: usize,
) -> Result<(String, &'static str), Error> {
    index
        .iter()
        .find(|(n, _, _)| n.eq_ignore_ascii_case(name))
        .map(|(_, part, kind)| (part.clone(), *kind))
        .ok_or_else(|| {
            Error::with_path(
                ErrorKind::SheetNotFound {
                    name: name.to_string(),
                },
                format!("ops[{op_index}].id"),
            )
        })
}

fn require_sheet_part(
    index: &[(String, String, &'static str)],
    name: &str,
    op_index: usize,
) -> Result<String, Error> {
    Ok(require_named_sheet(index, name, op_index)?.0)
}

pub(crate) fn load_dirty(
    pkg: &Package,
    dirty: &mut Vec<(String, Document)>,
    part: &str,
    make_explicit: bool,
) -> Result<(), Error> {
    if let Some((_, doc)) = dirty.iter_mut().find(|(p, _)| p == part) {
        if make_explicit {
            make_refs_explicit(&mut doc.root);
        }
        return Ok(());
    }
    let mut parsed = xml::parse(pkg.get(part)?)?;
    if make_explicit {
        // 省略 r 的行和单元格按隐式编号补成显式，否则后面插新行会让它们的编号漂移。
        make_refs_explicit(&mut parsed.root);
    }
    dirty.push((part.to_string(), parsed));
    Ok(())
}

pub(crate) fn dirty_of<'a>(dirty: &'a mut [(String, Document)], part: &str) -> &'a mut Document {
    &mut dirty.iter_mut().find(|(p, _)| p == part).unwrap().1
}

fn sheet_index(
    workbook: &Element,
    rels: &Relationships,
    wb_part: &str,
) -> Result<Vec<(String, String, &'static str)>, Error> {
    let sheets = workbook.child_by_local("sheets").ok_or_else(|| {
        Error::new(ErrorKind::XmlMissing {
            parent: "workbook.xml".into(),
            child: "sheets".into(),
        })
    })?;
    let mut out = Vec::new();
    for sh in sheets.children_by_local("sheet") {
        let name = sh.attr("name").unwrap_or("").to_string();
        // `r:id`：前缀只是习惯，任何映射到 relationships 命名空间的前缀都合法。
        let rid = sh.attr_local("id").ok_or_else(|| {
            Error::new(ErrorKind::XmlMissing {
                parent: "sheet".into(),
                child: "r:id".into(),
            })
        })?;
        let rel = rels.find_by_id(rid).ok_or_else(|| {
            Error::new(ErrorKind::RelNotFound {
                source: RelSource::Unspecified,
                rid: rid.to_string(),
            })
        })?;
        let kind = if rel.rel_type.ends_with("/chartsheet") {
            "chartsheet"
        } else if rel.rel_type.ends_with("/dialogsheet") {
            "dialogsheet"
        } else {
            "worksheet"
        };
        let part = opc::resolve_rel_target(wb_part, &rel.target)?;
        out.push((name, part, kind));
    }
    Ok(out)
}

fn parse_cell_id(id: &str, op_index: usize) -> Result<(String, String), Error> {
    let rest = id.strip_prefix("cell:").ok_or_else(|| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::NotCell { id: id.to_string() },
            },
            format!("ops[{op_index}].id"),
        )
    })?;
    let bang = rest.rfind('!').ok_or_else(|| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::NotCell { id: id.to_string() },
            },
            format!("ops[{op_index}].id"),
        )
    })?;
    let name = rest[..bang].to_string();
    if name.is_empty() {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::NotCell { id: id.to_string() },
            },
            format!("ops[{op_index}].id"),
        ));
    }
    let a1 = a1::normalize_a1(&rest[bang + 1..]).map_err(|_| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::NotCell { id: id.to_string() },
            },
            format!("ops[{op_index}].id"),
        )
    })?;
    Ok((name, a1))
}

/// SpreadsheetML 允许 `<row>` 和 `<c>` 省略 `r`，此时编号由出现顺序推出（§18.3.1.73、§18.3.1.4）。
/// 这里把隐式编号写成显式，语义不变，但后续插入新行就不会改变别的行的编号。
fn make_refs_explicit(worksheet: &mut Element) {
    let Some(sheet_data) = worksheet.child_by_local_mut("sheetData") else {
        return;
    };
    let mut prev_row = 0u32;
    for row in sheet_data.element_children_mut() {
        if row.local_name() != "row" {
            continue;
        }
        let declared = row.attr("r").and_then(|s| s.parse::<u32>().ok());
        let row_num = declared.unwrap_or(prev_row + 1);
        if declared.is_none() {
            row.set_attr("r", row_num.to_string());
        }
        prev_row = row_num;

        let mut prev_col = 0u32;
        for c in row.element_children_mut() {
            if c.local_name() != "c" {
                continue;
            }
            let declared_col = c
                .attr("r")
                .and_then(|s| a1::parse_cell(s).ok())
                .map(|p| p.col);
            let col = declared_col.unwrap_or(prev_col + 1);
            if declared_col.is_none() {
                c.set_attr("r", a1::format_cell(col, row_num));
            }
            prev_col = col;
        }
    }
}

pub(crate) fn apply_set_cell(
    worksheet: &mut Element,
    cell: a1::CellRef,
    op: &SetCellOp,
    op_index: usize,
) -> Result<(), Error> {
    reject_shared_formula_master(worksheet, cell, op_index)?;
    let sheet_data = ensure_sheet_data(worksheet);
    if matches!(&op.value, Some(SetCellValue::Null)) {
        clear_cell(sheet_data, cell);
        expand_dimension(worksheet, cell);
        return Ok(());
    }
    let c = ensure_cell(sheet_data, cell);
    fill_cell(c, op);
    expand_dimension(worksheet, cell);
    Ok(())
}

pub(crate) fn write_json_cell(
    worksheet: &mut Element,
    cell: a1::CellRef,
    value: &serde_json::Value,
    op_index: usize,
) -> Result<(), Error> {
    let parsed = super::request::parse_cell_value(value, &format!("ops[{op_index}]"))?;
    apply_set_cell(
        worksheet,
        cell,
        &SetCellOp {
            id: String::new(),
            value: Some(parsed),
            formula: None,
        },
        op_index,
    )
}

/// 共享公式的主格带 `ref` 和 `si`，从属格只带 `si`（§18.3.1.40）。改写或清空主格会让从属格
/// 的 `si` 失去定义，Excel 打开时报需要修复。第一期不做公式引用平移，因此直接拒绝这条 op。
fn reject_shared_formula_master(
    worksheet: &Element,
    cell: a1::CellRef,
    op_index: usize,
) -> Result<(), Error> {
    let Some(sheet_data) = worksheet.child_by_local("sheetData") else {
        return Ok(());
    };
    let Some(target) = find_cell(sheet_data, cell) else {
        return Ok(());
    };
    let Some(f) = target.child_by_local("f") else {
        return Ok(());
    };
    if f.attr("t") != Some("shared") || f.attr("ref").is_none() {
        return Ok(());
    }
    let Some(si) = f.attr("si") else {
        return Ok(());
    };
    if !has_shared_dependents(sheet_data, si, cell) {
        return Ok(());
    }
    Err(Error::with_path(
        ErrorKind::SharedFormulaMaster {
            cell: a1::format_cell(cell.col, cell.row),
        },
        format!("ops[{op_index}].id"),
    ))
}

pub(crate) fn find_cell(sheet_data: &Element, cell: a1::CellRef) -> Option<&Element> {
    for row in sheet_data.children_by_local("row") {
        if row.attr("r").and_then(|s| s.parse::<u32>().ok()) != Some(cell.row) {
            continue;
        }
        for c in row.children_by_local("c") {
            if c.attr("r").and_then(|r| a1::parse_cell(r).ok()) == Some(cell) {
                return Some(c);
            }
        }
    }
    None
}

fn has_shared_dependents(sheet_data: &Element, si: &str, master: a1::CellRef) -> bool {
    for row in sheet_data.children_by_local("row") {
        for c in row.children_by_local("c") {
            if c.attr("r").and_then(|r| a1::parse_cell(r).ok()) == Some(master) {
                continue;
            }
            let Some(f) = c.child_by_local("f") else {
                continue;
            };
            if f.attr("t") == Some("shared") && f.attr("si") == Some(si) {
                return true;
            }
        }
    }
    false
}

fn ensure_sheet_data(worksheet: &mut Element) -> &mut Element {
    if worksheet.child_by_local("sheetData").is_none() {
        worksheet.push_element(Element::new("sheetData"));
    }
    worksheet.child_by_local_mut("sheetData").unwrap()
}

fn ensure_cell(sheet_data: &mut Element, cell: a1::CellRef) -> &mut Element {
    let row_pos = find_row_insert_pos(sheet_data, cell.row);
    if row_pos.1 {
        // 已有该行，row_pos.0 是索引
    } else {
        let mut row = Element::new("row");
        row.set_attr("r", cell.row.to_string());
        sheet_data.children.insert(row_pos.0, Node::Element(row));
    }
    let row_idx = find_row_insert_pos(sheet_data, cell.row).0;
    let Node::Element(row) = &mut sheet_data.children[row_idx] else {
        unreachable!();
    };
    let col_pos = find_cell_insert_pos(row, cell.col);
    if !col_pos.1 {
        let mut c = Element::new("c");
        c.set_attr("r", a1::format_cell(cell.col, cell.row));
        row.children.insert(col_pos.0, Node::Element(c));
    }
    let col_idx = find_cell_insert_pos(row, cell.col).0;
    let Node::Element(c) = &mut row.children[col_idx] else {
        unreachable!();
    };
    c
}

fn find_row_insert_pos(sheet_data: &Element, row: u32) -> (usize, bool) {
    for (i, n) in sheet_data.children.iter().enumerate() {
        let Node::Element(e) = n else { continue };
        if e.local_name() != "row" {
            continue;
        }
        let r = e.attr("r").and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
        if r == row {
            return (i, true);
        }
        if r > row {
            return (i, false);
        }
    }
    (sheet_data.children.len(), false)
}

fn find_cell_insert_pos(row: &Element, col: u32) -> (usize, bool) {
    for (i, n) in row.children.iter().enumerate() {
        let Node::Element(e) = n else { continue };
        if e.local_name() != "c" {
            continue;
        }
        let Some(r) = e.attr("r") else { continue };
        let Ok(pref) = a1::parse_cell(r) else {
            continue;
        };
        if pref.col == col {
            return (i, true);
        }
        if pref.col > col {
            return (i, false);
        }
    }
    (row.children.len(), false)
}

fn clear_cell(sheet_data: &mut Element, cell: a1::CellRef) {
    let Some(row_idx) = find_row_insert_pos(sheet_data, cell.row).found_then() else {
        return;
    };
    let Node::Element(row) = &mut sheet_data.children[row_idx] else {
        return;
    };
    let Some(cell_idx) = find_cell_insert_pos(row, cell.col).found_then() else {
        return;
    };
    let Node::Element(c) = &mut row.children[cell_idx] else {
        return;
    };
    c.children.retain(|n| match n {
        Node::Element(e) => !matches!(e.local_name(), "f" | "v" | "is"),
        Node::Text(_) => false,
    });
    c.remove_attr("t");
    let only_r = c
        .attrs
        .iter()
        .all(|(k, _)| k.rsplit(':').next().unwrap_or(k) == "r");
    if only_r {
        row.children.remove(cell_idx);
    }
    let row_empty = row.element_children().all(|e| e.local_name() != "c");
    if row_empty && row.attrs.iter().all(|(k, _)| k == "r") {
        sheet_data.children.remove(row_idx);
    }
}

trait FoundThen {
    fn found_then(self) -> Option<usize>;
}

impl FoundThen for (usize, bool) {
    fn found_then(self) -> Option<usize> {
        if self.1 {
            Some(self.0)
        } else {
            None
        }
    }
}

fn fill_cell(c: &mut Element, op: &SetCellOp) {
    c.children.retain(|n| match n {
        Node::Element(e) => !matches!(e.local_name(), "f" | "v" | "is"),
        Node::Text(_) => true,
    });
    if let Some(formula) = &op.formula {
        c.remove_attr("t");
        let body = formula.strip_prefix('=').unwrap_or(formula);
        let mut f = Element::new("f");
        f.set_text(body);
        c.children.insert(0, Node::Element(f));
        return;
    }
    match op.value.as_ref() {
        Some(SetCellValue::Number(n)) => {
            c.remove_attr("t");
            let mut v = Element::new("v");
            v.set_text(number_xml(n));
            c.push_element(v);
        }
        Some(SetCellValue::Bool(b)) => {
            c.set_attr("t", "b");
            let mut v = Element::new("v");
            v.set_text(if *b { "1" } else { "0" });
            c.push_element(v);
        }
        Some(SetCellValue::Text(s)) => {
            c.set_attr("t", "inlineStr");
            let mut t = Element::new("t");
            if s.starts_with(' ') || s.ends_with(' ') || s.contains('\n') {
                t.set_attr("xml:space", "preserve");
            }
            t.set_text(s.clone());
            let mut is = Element::new("is");
            is.push_element(t);
            c.push_element(is);
        }
        Some(SetCellValue::Null) | None => {}
    }
}

fn number_xml(n: &serde_json::Number) -> String {
    if let Some(i) = n.as_i64() {
        return i.to_string();
    }
    if let Some(u) = n.as_u64() {
        return u.to_string();
    }
    n.to_string()
}

fn expand_dimension(worksheet: &mut Element, cell: a1::CellRef) {
    let Some(dim) = worksheet.child_by_local_mut("dimension") else {
        return;
    };
    let Some(pref) = dim.attr("ref") else {
        dim.set_attr("ref", a1::format_cell(cell.col, cell.row));
        return;
    };
    let Ok((mut start, mut end)) = a1::parse_range(pref) else {
        return;
    };
    if cell.col < start.col {
        start.col = cell.col;
    }
    if cell.row < start.row {
        start.row = cell.row;
    }
    if cell.col > end.col {
        end.col = cell.col;
    }
    if cell.row > end.row {
        end.row = cell.row;
    }
    dim.set_attr(
        "ref",
        format!(
            "{}:{}",
            a1::format_cell(start.col, start.row),
            a1::format_cell(end.col, end.row)
        ),
    );
}

pub(crate) fn mark_full_calc(workbook: &mut Element) {
    if let Some(calc) = workbook.child_by_local_mut("calcPr") {
        calc.set_attr("fullCalcOnLoad", "1");
        return;
    }
    workbook.push_element(Element::new("calcPr").with_attr("fullCalcOnLoad", "1"));
}

pub(crate) fn remove_calc_chain(pkg: &mut Package, wb_part: &str) -> Result<(), Error> {
    let mut rels = Relationships::from_package(pkg, wb_part)?;
    let targets: Vec<String> = rels
        .rels
        .iter()
        .filter(|r| r.rel_type.ends_with("/calcChain") || r.rel_type == REL_CALC_CHAIN)
        .filter(|r| !r.is_external())
        .filter_map(|r| opc::resolve_rel_target(wb_part, &r.target).ok())
        .collect();

    let mut extra = Vec::new();
    if pkg.contains("/xl/calcChain.xml") {
        extra.push("/xl/calcChain.xml".to_string());
    }

    let mut types = ContentTypes::from_package(pkg)?;
    let mut changed_rels = false;
    for t in targets.iter().chain(extra.iter()) {
        if pkg.contains(t) {
            pkg.remove(t);
            types.remove_override(t);
            rels.remove_by_target_part(wb_part, t);
            changed_rels = true;
        }
    }
    if changed_rels {
        types.write_to(pkg)?;
        rels.write_to(pkg, wb_part)?;
    }
    Ok(())
}
