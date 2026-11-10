//! 单元格、公式和 sheetData 的读写。

use std::cell::Cell;

use super::a1;
use super::formula;
use super::request::{SetCellOp, SetCellValue};
use super::REL_CALC_CHAIN;
use crate::error::{Error, ErrorKind, IdReason};
use crate::opc::{self, ContentTypes, Package, Relationships};
use crate::xml::{Document, Element, Node};

pub(crate) fn apply_hot(
    pkg: &Package,
    sheet_index: &[(String, String, &'static str)],
    dirty_sheets: &mut Vec<(String, Document)>,
    hot: &mut Option<SheetHot>,
    op: &SetCellOp,
    i: usize,
) -> Result<(), Error> {
    let (sheet_name, a1ref) = parse_cell_id(&op.id, i)?;
    let part = super::apply::worksheet_part(sheet_index, &sheet_name, i)?;
    let cell = a1::parse_cell(&a1ref)?;
    let reuse = hot.as_ref().is_some_and(|h| h.part == part);
    super::apply::load_dirty(pkg, dirty_sheets, &part, !reuse)?;
    let doc = super::apply::dirty_of(dirty_sheets, &part);
    if !reuse {
        let sheet_data = doc.root.child_by_local("sheetData");
        *hot = Some(SheetHot {
            part: part.clone(),
            index: sheet_data.map(formula::index_sheet).unwrap_or_default(),
            binary: sheet_data.is_none_or(grid_is_sorted),
            fresh: true,
        });
    }
    let hot = hot.as_mut().unwrap();
    if !hot.fresh {
        hot.index = doc
            .root
            .child_by_local("sheetData")
            .map(formula::index_sheet)
            .unwrap_or_default();
        hot.fresh = true;
    }
    let _binary = BinaryGrid::enter(hot.binary);
    let keep = apply_set_cell_with(&mut doc.root, cell, op, i, &hot.index)?;
    hot.fresh = keep;
    Ok(())
}

pub(crate) fn parse_cell_id(id: &str, op_index: usize) -> Result<(String, String), Error> {
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
/// 一次 apply 里连续 `setCell` 打在同一张表上时复用。换表、或中间插入别的 op，就丢掉。
pub(crate) struct SheetHot {
    part: String,
    index: formula::FormulaIndex,
    /// 行号、行内列号都严格递增，且没有缺 `r` 的行或格。定位用二分。
    binary: bool,
    /// 上一条没有改共享公式、数组公式或数据表的布局。
    fresh: bool,
}

thread_local! {
    /// `setCell` 进入期间为真，让 `ensure_cell` 走二分。离开时恢复，别的 op 仍线性扫描。
    static BINARY_GRID: Cell<bool> = const { Cell::new(false) };
}

struct BinaryGrid(bool);

impl BinaryGrid {
    fn enter(on: bool) -> Self {
        let prev = BINARY_GRID.with(|c| c.replace(on));
        Self(prev)
    }
}

impl Drop for BinaryGrid {
    fn drop(&mut self) {
        let prev = self.0;
        BINARY_GRID.with(|c| c.set(prev));
    }
}

fn binary_grid() -> bool {
    BINARY_GRID.with(|c| c.get())
}

pub(crate) fn make_refs_explicit(worksheet: &mut Element) {
    let Some(sheet_data) = worksheet.child_by_local_mut("sheetData") else {
        return;
    };
    let mut prev_row = 0u32;
    for row in sheet_data.element_children_mut() {
        if row.local_name() != "row" {
            continue;
        }
        let row_num = a1::row_after(prev_row, row.attr("r"));
        if row.attr("r").and_then(|s| s.parse::<u32>().ok()).is_none() {
            row.set_attr("r", row_num.to_string());
        }
        prev_row = row_num;

        let mut prev_col = 0u32;
        for c in row.element_children_mut() {
            if c.local_name() != "c" {
                continue;
            }
            let place = a1::place_cell(row_num, prev_col, c.attr("r"));
            let cell = place.cell();
            if matches!(place, a1::CellPlace::Omitted { .. }) {
                c.set_attr("r", a1::format_cell(cell.col, cell.row));
            }
            prev_col = cell.col;
        }
    }
}

pub(crate) fn apply_set_cell(
    worksheet: &mut Element,
    cell: a1::CellRef,
    op: &SetCellOp,
    op_index: usize,
) -> Result<(), Error> {
    let index = worksheet
        .child_by_local("sheetData")
        .map(formula::index_sheet)
        .unwrap_or_default();
    let binary = worksheet
        .child_by_local("sheetData")
        .is_none_or(grid_is_sorted);
    let _binary = BinaryGrid::enter(binary);
    apply_set_cell_with(worksheet, cell, op, op_index, &index).map(|_| ())
}

/// 返回值：公式索引还能不能留给下一条 `setCell`。
fn apply_set_cell_with(
    worksheet: &mut Element,
    cell: a1::CellRef,
    op: &SetCellOp,
    op_index: usize,
    index: &formula::FormulaIndex,
) -> Result<bool, Error> {
    if let Some(formula::Occupied::DataTable { master }) = formula::occupy_at(index, cell) {
        let extras = op.array.is_some()
            || op.array_ref.is_some()
            || op.shared.is_some()
            || op.shared_ref.is_some()
            || op.value.is_some();
        if master != cell || op.formula.is_none() || extras {
            return Err(Error::with_path(
                ErrorKind::DataTableFormula {
                    cell: a1::format_cell(cell.col, cell.row),
                },
                format!("ops[{op_index}].id"),
            ));
        }
        if let Some((start, end)) = op.data_table_ref {
            let same = index
                .tables
                .iter()
                .find(|g| g.master == cell)
                .is_some_and(|g| g.start == start && g.end == end);
            if !same {
                return Err(Error::with_path(
                    ErrorKind::DataTableResize,
                    format!("ops[{op_index}].dataTableRef"),
                ));
            }
        }
        return write_data_table_inputs(worksheet, cell, op, op_index).map(|()| false);
    }

    if op.data_table_ref.is_some() || formula_is_table(op.formula.as_deref()) {
        return write_new_data_table(worksheet, cell, op, index, op_index).map(|()| false);
    }

    let array_master = match formula::occupy_at(index, cell) {
        Some(formula::Occupied::Array { master }) => Some(master),
        _ => None,
    };
    let shared_here = match formula::occupy_at(index, cell) {
        Some(formula::Occupied::Shared { si, master }) => Some((si, master)),
        _ => None,
    };

    let wants_array = op.array == Some(true) || op.array_ref.is_some();
    let wants_drop_array = op.array == Some(false);
    let wants_explode = op.shared == Some(false);
    let wants_shared_keep = op.shared == Some(true) || op.shared_ref.is_some();
    let is_array_master = array_master == Some(cell);
    let is_shared_master = shared_here
        .as_ref()
        .map(|(_, master)| *master == cell)
        .unwrap_or(false);

    if let Some(master) = array_master {
        if master != cell {
            return Err(Error::with_path(
                ErrorKind::ArrayFormulaPart {
                    cell: a1::format_cell(cell.col, cell.row),
                },
                format!("ops[{op_index}].id"),
            ));
        }
    }

    if wants_array {
        let (start, end) = op
            .array_ref
            .or_else(|| {
                index
                    .arrays
                    .iter()
                    .find(|g| g.master == cell)
                    .map(|g| (g.start, g.end))
            })
            .unwrap_or((cell, cell));
        return write_array_formula(worksheet, cell, start, end, op, index, op_index)
            .map(|()| false);
    }

    if is_array_master && op.formula.is_some() && !wants_drop_array {
        let g = index.arrays.iter().find(|g| g.master == cell).unwrap();
        return write_array_formula(worksheet, cell, g.start, g.end, op, index, op_index)
            .map(|()| false);
    }

    if is_array_master {
        apply_plain_or_clear(worksheet, cell, op);
        return Ok(false);
    }

    if let Some((_, master)) = &shared_here {
        if *master != cell && !wants_shared_keep {
            apply_plain_or_clear(worksheet, cell, op);
            return Ok(false);
        }
    }

    let explode_then_write =
        wants_explode || (is_shared_master && op.value.is_some() && !wants_shared_keep);

    if explode_then_write {
        if let Some((si, master)) = shared_here {
            let body = index
                .shared
                .get(&si)
                .map(|g| g.formula.clone())
                .unwrap_or_default();
            explode_shared_group(worksheet, &si, master, &body);
        }
        apply_plain_or_clear(worksheet, cell, op);
        return Ok(false);
    }

    if wants_shared_keep || (is_shared_master && op.formula.is_some()) {
        return write_shared_formula(worksheet, cell, op, index, is_shared_master, op_index)
            .map(|()| false);
    }

    apply_plain_or_clear(worksheet, cell, op);
    Ok(true)
}

fn formula_is_table(formula: Option<&str>) -> bool {
    formula.is_some_and(|f| formula::parse_table_formula(f).is_some())
}

fn write_new_data_table(
    worksheet: &mut Element,
    cell: a1::CellRef,
    op: &SetCellOp,
    index: &formula::FormulaIndex,
    op_index: usize,
) -> Result<(), Error> {
    let Some((start, end)) = op.data_table_ref else {
        return Err(Error::with_path(
            ErrorKind::MissingField {
                field: "dataTableRef".into(),
            },
            format!("ops[{op_index}].dataTableRef"),
        ));
    };
    let formula = op.formula.as_deref().unwrap_or("");
    let Some((first, second)) = formula::parse_table_formula(formula) else {
        return Err(Error::with_path(
            ErrorKind::InvalidDataTableFormula,
            format!("ops[{op_index}].formula"),
        ));
    };
    if cell != start {
        return Err(Error::with_path(
            ErrorKind::FormulaRangeNotTopLeft,
            format!("ops[{op_index}].id"),
        ));
    }
    let (dt2d, dtr, r1, r2) = match (first, second) {
        (Some(a), Some(b)) => (
            true,
            true,
            a1::format_cell(a.col, a.row),
            Some(a1::format_cell(b.col, b.row)),
        ),
        (Some(a), None) => (false, true, a1::format_cell(a.col, a.row), None),
        (None, Some(b)) => (false, false, a1::format_cell(b.col, b.row), None),
        (None, None) => {
            return Err(Error::with_path(
                ErrorKind::InvalidDataTableFormula,
                format!("ops[{op_index}].formula"),
            ));
        }
    };
    reject_overlap(index, start, end, SkipGroup::Fresh, op_index)?;
    let refer = a1::format_range(start, end);
    {
        let sheet_data = ensure_sheet_data(worksheet);
        for row in start.row..=end.row {
            for col in start.col..=end.col {
                let here = a1::CellRef { col, row };
                let c = ensure_cell(sheet_data, here);
                if here == cell {
                    write_formula_elem(
                        c,
                        "",
                        FormulaWrite::DataTable {
                            refer: &refer,
                            dt2d,
                            dtr,
                            r1: &r1,
                            r2: r2.as_deref(),
                        },
                    );
                } else {
                    strip_formula_and_value(c);
                }
            }
        }
    }
    expand_dimension(worksheet, start);
    expand_dimension(worksheet, end);
    Ok(())
}

fn write_data_table_inputs(
    worksheet: &mut Element,
    cell: a1::CellRef,
    op: &SetCellOp,
    op_index: usize,
) -> Result<(), Error> {
    let formula = op.formula.as_deref().unwrap_or("");
    let Some((first, second)) = formula::parse_table_formula(formula) else {
        return Err(Error::with_path(
            ErrorKind::InvalidDataTableFormula,
            format!("ops[{op_index}].formula"),
        ));
    };
    let sheet_data = ensure_sheet_data(worksheet);
    let c = ensure_cell(sheet_data, cell);
    let Some(f) = c.child_by_local_mut("f") else {
        return Err(Error::new(ErrorKind::XmlMissing {
            parent: "c".into(),
            child: "f".into(),
        }));
    };
    let shape = formula::table_shape(
        formula::xml_true(f.attr("dt2D")),
        formula::xml_true(f.attr("dtr")),
    );
    if !formula::args_match_shape(first, second, shape) {
        return Err(Error::with_path(
            ErrorKind::InvalidDataTableFormula,
            format!("ops[{op_index}].formula"),
        ));
    }
    match (shape, first, second) {
        (formula::TableShape::TwoD, Some(r1), Some(r2)) => {
            f.set_attr("r1", a1::format_cell(r1.col, r1.row));
            f.set_attr("r2", a1::format_cell(r2.col, r2.row));
        }
        (formula::TableShape::Row, Some(r1), None) => {
            f.set_attr("r1", a1::format_cell(r1.col, r1.row));
        }
        (formula::TableShape::Column, None, Some(r1)) => {
            f.set_attr("r1", a1::format_cell(r1.col, r1.row));
        }
        _ => {
            return Err(Error::with_path(
                ErrorKind::InvalidDataTableFormula,
                format!("ops[{op_index}].formula"),
            ));
        }
    }
    f.children.clear();
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
            shared: None,
            shared_ref: None,
            array: None,
            array_ref: None,
            data_table_ref: None,
        },
        op_index,
    )
}

fn apply_plain_or_clear(worksheet: &mut Element, cell: a1::CellRef, op: &SetCellOp) {
    let sheet_data = ensure_sheet_data(worksheet);
    if matches!(&op.value, Some(SetCellValue::Null)) {
        clear_cell(sheet_data, cell);
        expand_dimension(worksheet, cell);
        return;
    }
    let c = ensure_cell(sheet_data, cell);
    fill_cell(c, op);
    expand_dimension(worksheet, cell);
}

fn write_array_formula(
    worksheet: &mut Element,
    cell: a1::CellRef,
    start: a1::CellRef,
    end: a1::CellRef,
    op: &SetCellOp,
    index: &formula::FormulaIndex,
    op_index: usize,
) -> Result<(), Error> {
    if cell != start {
        return Err(Error::with_path(
            ErrorKind::FormulaRangeNotTopLeft,
            format!("ops[{op_index}].id"),
        ));
    }
    let body = formula::formula_body(op.formula.as_deref().unwrap_or("")).to_string();
    reject_overlap(index, start, end, SkipGroup::Array(cell), op_index)?;
    let refer = a1::format_range(start, end);
    {
        let sheet_data = ensure_sheet_data(worksheet);
        for row in start.row..=end.row {
            for col in start.col..=end.col {
                let here = a1::CellRef { col, row };
                let c = ensure_cell(sheet_data, here);
                if here == cell {
                    write_formula_elem(c, &body, FormulaWrite::Array { refer: &refer });
                } else {
                    strip_formula_and_value(c);
                }
            }
        }
    }
    expand_dimension(worksheet, start);
    expand_dimension(worksheet, end);
    Ok(())
}

fn write_shared_formula(
    worksheet: &mut Element,
    cell: a1::CellRef,
    op: &SetCellOp,
    index: &formula::FormulaIndex,
    is_shared_master: bool,
    op_index: usize,
) -> Result<(), Error> {
    if !is_shared_master && op.shared_ref.is_none() {
        return Err(Error::with_path(
            ErrorKind::SharedNeedsRef,
            format!("ops[{op_index}]"),
        ));
    }
    let existing_by_master = index.shared.values().find(|g| g.master == cell);
    let (start, end) = op
        .shared_ref
        .or_else(|| existing_by_master.map(|g| (g.start, g.end)))
        .unwrap_or((cell, cell));
    if cell != start {
        return Err(Error::with_path(
            ErrorKind::FormulaRangeNotTopLeft,
            format!("ops[{op_index}].id"),
        ));
    }
    let existing = existing_by_master.or_else(|| {
        let hits: Vec<_> = index
            .shared
            .values()
            .filter(|g| a1::ranges_overlap(g.start, g.end, start, end))
            .collect();
        if hits.len() == 1 {
            Some(hits[0])
        } else {
            None
        }
    });
    let body = op
        .formula
        .as_deref()
        .map(formula::formula_body)
        .map(str::to_string)
        .or_else(|| existing.map(|g| g.formula.clone()))
        .unwrap_or_default();
    let si = existing.map(|g| g.si.clone()).unwrap_or_else(|| {
        worksheet
            .child_by_local("sheetData")
            .map(formula::next_si)
            .unwrap_or(0)
            .to_string()
    });
    let same_range = existing
        .map(|g| g.start == start && g.end == end && g.master == cell)
        .unwrap_or(false);
    if same_range {
        let refer = a1::format_range(start, end);
        let sheet_data = ensure_sheet_data(worksheet);
        let c = ensure_cell(sheet_data, cell);
        if let Some(f) = c.child_by_local_mut("f") {
            f.set_text(&body);
            if f.attr("t") != Some("shared") {
                f.set_attr("t", "shared");
            }
            f.set_attr("si", &si);
            f.set_attr("ref", refer);
        } else {
            write_formula_elem(
                c,
                &body,
                FormulaWrite::Shared {
                    si: &si,
                    refer: Some(&refer),
                },
            );
        }
        strip_value_only(c);
        expand_dimension(worksheet, cell);
        return Ok(());
    }
    reject_overlap(index, start, end, SkipGroup::Shared(&si), op_index)?;
    if let Some(old) = existing {
        let members = worksheet
            .child_by_local("sheetData")
            .map(|sd| formula::shared_members(sd, &old.si))
            .unwrap_or_default();
        for m in members {
            if m != cell && !a1::in_range(m, start, end) {
                let shifted = formula::shift_a1_formula(&old.formula, old.master, m);
                let sheet_data = ensure_sheet_data(worksheet);
                let c = ensure_cell(sheet_data, m);
                write_formula_elem(c, &shifted, FormulaWrite::Normal);
            }
        }
    }
    if start == end {
        apply_plain_or_clear(worksheet, cell, op);
        return Ok(());
    }
    let refer = a1::format_range(start, end);
    {
        let sheet_data = ensure_sheet_data(worksheet);
        for row in start.row..=end.row {
            for col in start.col..=end.col {
                let here = a1::CellRef { col, row };
                let c = ensure_cell(sheet_data, here);
                if here == cell {
                    write_formula_elem(
                        c,
                        &body,
                        FormulaWrite::Shared {
                            si: &si,
                            refer: Some(&refer),
                        },
                    );
                } else {
                    write_formula_elem(
                        c,
                        "",
                        FormulaWrite::Shared {
                            si: &si,
                            refer: None,
                        },
                    );
                }
            }
        }
    }
    expand_dimension(worksheet, start);
    expand_dimension(worksheet, end);
    Ok(())
}

fn explode_shared_group(worksheet: &mut Element, si: &str, master: a1::CellRef, body: &str) {
    let members = worksheet
        .child_by_local("sheetData")
        .map(|sd| formula::shared_members(sd, si))
        .unwrap_or_default();
    let sheet_data = ensure_sheet_data(worksheet);
    for m in members {
        if m == master {
            continue;
        }
        let shifted = formula::shift_a1_formula(body, master, m);
        let c = ensure_cell(sheet_data, m);
        write_formula_elem(c, &shifted, FormulaWrite::Normal);
    }
}

enum SkipGroup<'a> {
    Shared(&'a str),
    Array(a1::CellRef),
    Fresh,
}

fn reject_overlap(
    index: &formula::FormulaIndex,
    start: a1::CellRef,
    end: a1::CellRef,
    skip: SkipGroup<'_>,
    op_index: usize,
) -> Result<(), Error> {
    for row in start.row..=end.row {
        for col in start.col..=end.col {
            let here = a1::CellRef { col, row };
            match formula::occupy_at(index, here) {
                Some(formula::Occupied::DataTable { .. }) => {
                    return Err(Error::with_path(
                        ErrorKind::FormulaGroupOverlap {
                            cell: a1::format_cell(col, row),
                        },
                        format!("ops[{op_index}].id"),
                    ));
                }
                Some(formula::Occupied::Array { master }) => {
                    let ok = matches!(skip, SkipGroup::Array(m) if m == master);
                    if !ok {
                        return Err(Error::with_path(
                            ErrorKind::FormulaGroupOverlap {
                                cell: a1::format_cell(col, row),
                            },
                            format!("ops[{op_index}].id"),
                        ));
                    }
                }
                Some(formula::Occupied::Shared { si, .. }) => {
                    let ok = matches!(skip, SkipGroup::Shared(s) if s == si);
                    if !ok {
                        return Err(Error::with_path(
                            ErrorKind::FormulaGroupOverlap {
                                cell: a1::format_cell(col, row),
                            },
                            format!("ops[{op_index}].id"),
                        ));
                    }
                }
                None => {}
            }
        }
    }
    Ok(())
}

enum FormulaWrite<'a> {
    Normal,
    Shared {
        si: &'a str,
        refer: Option<&'a str>,
    },
    Array {
        refer: &'a str,
    },
    DataTable {
        refer: &'a str,
        dt2d: bool,
        dtr: bool,
        r1: &'a str,
        r2: Option<&'a str>,
    },
}

fn strip_formula_and_value(c: &mut Element) {
    c.children.retain(|n| match n {
        Node::Element(e) => !matches!(e.local_name(), "f" | "v" | "is"),
        Node::Text(_) | Node::Raw(_) => true,
    });
    c.remove_attr("t");
}

fn strip_value_only(c: &mut Element) {
    c.children.retain(|n| match n {
        Node::Element(e) => !matches!(e.local_name(), "v" | "is"),
        Node::Text(_) | Node::Raw(_) => true,
    });
}

fn write_formula_elem(c: &mut Element, body: &str, kind: FormulaWrite<'_>) {
    strip_formula_and_value(c);
    let mut f = Element::new("f");
    match kind {
        FormulaWrite::Normal => {}
        FormulaWrite::Shared { si, refer } => {
            f.set_attr("t", "shared");
            f.set_attr("si", si);
            if let Some(r) = refer {
                f.set_attr("ref", r);
            }
        }
        FormulaWrite::Array { refer } => {
            f.set_attr("t", "array");
            f.set_attr("ref", refer);
        }
        FormulaWrite::DataTable {
            refer,
            dt2d,
            dtr,
            r1,
            r2,
        } => {
            f.set_attr("t", "dataTable");
            f.set_attr("ref", refer);
            f.set_attr("dt2D", if dt2d { "1" } else { "0" });
            f.set_attr("dtr", if dtr { "1" } else { "0" });
            f.set_attr("r1", r1);
            if let Some(r2) = r2 {
                f.set_attr("r2", r2);
            }
        }
    }
    if !body.is_empty() {
        f.set_text(body);
    }
    c.children.insert(0, Node::Element(f));
}

/// 合并区域碰到共享主格、数组或数据表时拒绝，避免 Excel 修复。
pub(crate) fn reject_merge_formula_conflicts(
    worksheet: &Element,
    start: a1::CellRef,
    end: a1::CellRef,
    op_index: usize,
) -> Result<(), Error> {
    let Some(sheet_data) = worksheet.child_by_local("sheetData") else {
        return Ok(());
    };
    let index = formula::index_sheet(sheet_data);
    for row in start.row..=end.row {
        for col in start.col..=end.col {
            let here = a1::CellRef { col, row };
            match formula::occupy_at(&index, here) {
                Some(formula::Occupied::DataTable { .. }) => {
                    return Err(Error::with_path(
                        ErrorKind::DataTableFormula {
                            cell: a1::format_cell(col, row),
                        },
                        format!("ops[{op_index}].id"),
                    ));
                }
                Some(formula::Occupied::Array { master }) => {
                    return Err(Error::with_path(
                        ErrorKind::ArrayFormulaPart {
                            cell: a1::format_cell(master.col, master.row),
                        },
                        format!("ops[{op_index}].id"),
                    ));
                }
                Some(formula::Occupied::Shared { si, master })
                    if master == here
                        && formula::has_shared_dependents(sheet_data, &si, master) =>
                {
                    return Err(Error::with_path(
                        ErrorKind::SharedFormulaMaster {
                            cell: a1::format_cell(master.col, master.row),
                        },
                        format!("ops[{op_index}].id"),
                    ));
                }
                Some(formula::Occupied::Shared { .. }) | None => {}
            }
        }
    }
    Ok(())
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

pub(crate) fn ensure_sheet_data(worksheet: &mut Element) -> &mut Element {
    if worksheet.child_by_local("sheetData").is_none() {
        worksheet.push_element(Element::new("sheetData"));
    }
    worksheet.child_by_local_mut("sheetData").unwrap()
}

pub(crate) fn ensure_cell(sheet_data: &mut Element, cell: a1::CellRef) -> &mut Element {
    let (row_idx, row_found) = find_row_insert_pos(sheet_data, cell.row);
    if !row_found {
        let mut el = Element::new("row");
        el.set_attr("r", cell.row.to_string());
        sheet_data.children.insert(row_idx, Node::Element(el));
    }
    let Node::Element(row) = &mut sheet_data.children[row_idx] else {
        unreachable!();
    };
    let (col_idx, col_found) = find_cell_insert_pos(row, cell.col);
    if !col_found {
        let mut c = Element::new("c");
        c.set_attr("r", a1::format_cell(cell.col, cell.row));
        row.children.insert(col_idx, Node::Element(c));
    }
    let Node::Element(c) = &mut row.children[col_idx] else {
        unreachable!();
    };
    c
}

pub(crate) fn ensure_row(sheet_data: &mut Element, row: u32) -> &mut Element {
    let (row_idx, found) = find_row_insert_pos(sheet_data, row);
    if !found {
        let mut el = Element::new("row");
        el.set_attr("r", row.to_string());
        sheet_data.children.insert(row_idx, Node::Element(el));
    }
    let Node::Element(row_el) = &mut sheet_data.children[row_idx] else {
        unreachable!();
    };
    row_el
}

pub(crate) fn find_row_index(sheet_data: &Element, row: u32) -> Option<usize> {
    find_row_insert_pos(sheet_data, row).found_then()
}

fn grid_is_sorted(sheet_data: &Element) -> bool {
    let mut prev_row = 0u32;
    for n in &sheet_data.children {
        let Some(r) = row_key(n) else {
            return false;
        };
        if r <= prev_row {
            return false;
        }
        prev_row = r;
        let Node::Element(row) = n else {
            return false;
        };
        let mut prev_col = 0u32;
        for cn in &row.children {
            let Some(col) = cell_col(cn) else {
                return false;
            };
            if col <= prev_col {
                return false;
            }
            prev_col = col;
        }
    }
    true
}

fn row_key(n: &Node) -> Option<u32> {
    let Node::Element(e) = n else {
        return None;
    };
    if e.local_name() != "row" {
        return None;
    }
    e.attr("r")?.parse().ok()
}

fn cell_col(n: &Node) -> Option<u32> {
    let Node::Element(e) = n else {
        return None;
    };
    if e.local_name() != "c" {
        return None;
    }
    a1::parse_cell(e.attr("r")?).ok().map(|c| c.col)
}

fn find_row_insert_pos(sheet_data: &Element, row: u32) -> (usize, bool) {
    if binary_grid() {
        if let Some(pos) = find_row_binary(sheet_data, row) {
            return pos;
        }
    }
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

fn find_row_binary(sheet_data: &Element, row: u32) -> Option<(usize, bool)> {
    let children = &sheet_data.children;
    let mut lo = 0;
    let mut hi = children.len();
    while lo < hi {
        let mid = (lo + hi) / 2;
        let r = row_key(&children[mid])?;
        if r < row {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    if lo < children.len() && row_key(&children[lo])? == row {
        Some((lo, true))
    } else {
        Some((lo, false))
    }
}

fn find_cell_insert_pos(row: &Element, col: u32) -> (usize, bool) {
    if binary_grid() {
        if let Some(pos) = find_cell_binary(row, col) {
            return pos;
        }
    }
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

fn find_cell_binary(row: &Element, col: u32) -> Option<(usize, bool)> {
    let children = &row.children;
    let mut lo = 0;
    let mut hi = children.len();
    while lo < hi {
        let mid = (lo + hi) / 2;
        let c = cell_col(&children[mid])?;
        if c < col {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    if lo < children.len() && cell_col(&children[lo])? == col {
        Some((lo, true))
    } else {
        Some((lo, false))
    }
}

pub(crate) fn clear_cell(sheet_data: &mut Element, cell: a1::CellRef) {
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
        Node::Text(_) | Node::Raw(_) => false,
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
        Node::Text(_) | Node::Raw(_) => true,
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

pub(crate) fn expand_dimension(worksheet: &mut Element, cell: a1::CellRef) {
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
