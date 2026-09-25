//! 按 id 改单元格，并标记打开时重算。

use crate::error::{Error, ErrorCode};
use crate::json::{SetCellOp, SetCellValue};
use crate::opc::{self, ContentTypes, Package, Relationships};
use crate::xml::{self, Document, Element, Node};

use super::a1;
use super::REL_CALC_CHAIN;

pub fn apply(bytes: &[u8], ops: &[SetCellOp]) -> Result<Vec<u8>, Error> {
    let mut pkg = Package::open_bytes(bytes.to_vec())?;
    let wb_part = opc::office_document_part(&pkg)?;
    let mut wb_doc = xml::parse(pkg.get(&wb_part)?)?;
    let wb_rels = Relationships::from_package(&pkg, &wb_part)?;

    let sheet_index = sheet_index(&wb_doc.root, &wb_rels, &wb_part)?;

    let mut dirty_sheets: Vec<(String, Document)> = Vec::new();

    for (i, op) in ops.iter().enumerate() {
        let (sheet_name, a1) = parse_cell_id(&op.id, i)?;
        let meta = sheet_index
            .iter()
            .find(|(n, _, k)| n.eq_ignore_ascii_case(&sheet_name) && *k == "worksheet")
            .ok_or_else(|| {
                if sheet_index
                    .iter()
                    .any(|(n, _, _)| n.eq_ignore_ascii_case(&sheet_name))
                {
                    Error::with_path(
                        ErrorCode::InvalidField,
                        "只能对工作表 setCell",
                        format!("ops[{i}].id"),
                    )
                } else {
                    Error::with_path(
                        ErrorCode::IdNotFound,
                        format!("找不到工作表：{sheet_name}"),
                        format!("ops[{i}].id"),
                    )
                }
            })?;
        let original_name = meta.0.clone();
        let part = meta.1.clone();
        let cell = a1::parse_cell(&a1)?;

        let doc = if let Some((_, doc)) = dirty_sheets.iter_mut().find(|(p, _)| p == &part) {
            doc
        } else {
            let parsed = xml::parse(pkg.get(&part)?)?;
            dirty_sheets.push((part.clone(), parsed));
            &mut dirty_sheets.last_mut().unwrap().1
        };
        apply_set_cell(&mut doc.root, cell, op, &original_name)?;
        let _ = original_name;
    }

    for (part, doc) in dirty_sheets {
        pkg.set(&part, doc.to_bytes()?)?;
    }

    mark_full_calc(&mut wb_doc.root);
    pkg.set(&wb_part, wb_doc.to_bytes()?)?;
    remove_calc_chain(&mut pkg, &wb_part)?;

    pkg.save_to_vec()
}

fn sheet_index(
    workbook: &Element,
    rels: &Relationships,
    wb_part: &str,
) -> Result<Vec<(String, String, &'static str)>, Error> {
    let sheets = workbook
        .child_by_local("sheets")
        .ok_or_else(|| Error::new(ErrorCode::XmlBroken, "workbook.xml 缺少 sheets"))?;
    let mut out = Vec::new();
    for sh in sheets.children_by_local("sheet") {
        let name = sh.attr("name").unwrap_or("").to_string();
        let rid = sh
            .attr("id")
            .ok_or_else(|| Error::new(ErrorCode::XmlBroken, "sheet 缺少 r:id"))?;
        let rel = rels
            .find_by_id(rid)
            .ok_or_else(|| Error::new(ErrorCode::OpcPartMissing, format!("没有关系 {rid}")))?;
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
            ErrorCode::InvalidId,
            format!("不是单元格 id：{id}"),
            format!("ops[{op_index}].id"),
        )
    })?;
    let bang = rest.rfind('!').ok_or_else(|| {
        Error::with_path(
            ErrorCode::InvalidId,
            format!("不是单元格 id：{id}"),
            format!("ops[{op_index}].id"),
        )
    })?;
    let name = rest[..bang].to_string();
    if name.is_empty() {
        return Err(Error::with_path(
            ErrorCode::InvalidId,
            format!("不是单元格 id：{id}"),
            format!("ops[{op_index}].id"),
        ));
    }
    let a1 = a1::normalize_a1(&rest[bang + 1..]).map_err(|_| {
        Error::with_path(
            ErrorCode::InvalidId,
            format!("不是单元格 id：{id}"),
            format!("ops[{op_index}].id"),
        )
    })?;
    Ok((name, a1))
}

fn apply_set_cell(
    worksheet: &mut Element,
    cell: a1::CellRef,
    op: &SetCellOp,
    _sheet_name: &str,
) -> Result<(), Error> {
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

fn mark_full_calc(workbook: &mut Element) {
    if let Some(calc) = workbook.child_by_local_mut("calcPr") {
        calc.set_attr("fullCalcOnLoad", "1");
        return;
    }
    workbook.push_element(Element::new("calcPr").with_attr("fullCalcOnLoad", "1"));
}

fn remove_calc_chain(pkg: &mut Package, wb_part: &str) -> Result<(), Error> {
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
