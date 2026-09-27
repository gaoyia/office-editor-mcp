//! 列宽、行高与合并：inspect 摘要，apply 写 `cols` / `ht` / `mergeCells`。

use serde_json::{json, Map, Value};

use super::a1::{self, CellRef};
use super::apply;
use super::request::{MergeRangeOp, SetColWidthOp, SetRowHeightOp, UnmergeRangeOp};
use crate::error::{Error, ErrorKind};
use crate::xml::{Element, Node};

use super::request::Patch;

/// `CT_Worksheet` 子元素顺序（§A.2）。未知元素（如 `legacyDrawing`）插在 `extLst` 之前。
const WORKSHEET_CHILD_ORDER: &[&str] = &[
    "sheetPr",
    "dimension",
    "sheetViews",
    "sheetFormatPr",
    "cols",
    "sheetData",
    "sheetCalcPr",
    "sheetProtection",
    "protectedRanges",
    "scenarios",
    "autoFilter",
    "sortState",
    "dataConsolidate",
    "customSheetViews",
    "mergeCells",
    "phoneticPr",
    "conditionalFormatting",
    "dataValidations",
    "hyperlinks",
    "printOptions",
    "pageMargins",
    "pageSetup",
    "headerFooter",
    "rowBreaks",
    "colBreaks",
    "customProperties",
    "cellWatches",
    "ignoredErrors",
    "smartTags",
    "drawing",
    "legacyDrawing",
    "drawingHF",
    "legacyDrawingHF",
    "picture",
    "oleObjects",
    "controls",
    "webPublishItems",
    "tableParts",
    "extLst",
];

pub(crate) fn inspect(root: &Element, start: CellRef, end: CellRef) -> (Value, Value, Value) {
    (
        inspect_cols(root, start, end),
        inspect_rows(root, start, end),
        inspect_merges(root, start, end),
    )
}

fn inspect_cols(root: &Element, start: CellRef, end: CellRef) -> Value {
    let mut out = Vec::new();
    for cols in root.children_by_local("cols") {
        for col in cols.children_by_local("col") {
            let Some((min, max)) = col_span(col) else {
                continue;
            };
            let cmin = min.max(start.col);
            let cmax = max.min(end.col);
            if cmin > cmax {
                continue;
            }
            let width = col
                .attr("width")
                .and_then(parse_xml_f64)
                .map(xml_width_to_chars);
            let best_fit = attr_true(col, "bestFit");
            if width.is_none() && !best_fit {
                continue;
            }
            let mut obj = Map::new();
            obj.insert("min".into(), json!(a1::col_to_letters(cmin)));
            obj.insert("max".into(), json!(a1::col_to_letters(cmax)));
            if let Some(w) = width {
                obj.insert("width".into(), json_num(w));
            }
            if best_fit {
                obj.insert("bestFit".into(), json!(true));
            }
            out.push(Value::Object(obj));
        }
    }
    Value::Array(out)
}

fn inspect_rows(root: &Element, start: CellRef, end: CellRef) -> Value {
    let mut out = Vec::new();
    let Some(sheet_data) = root.child_by_local("sheetData") else {
        return Value::Array(out);
    };
    for row in sheet_data.children_by_local("row") {
        let Some(r) = row.attr("r").and_then(|s| s.parse::<u32>().ok()) else {
            continue;
        };
        if r < start.row || r > end.row {
            continue;
        }
        let Some(ht) = row.attr("ht").and_then(parse_xml_f64) else {
            continue;
        };
        out.push(json!({
            "r": r,
            "height": json_num(ht),
        }));
    }
    Value::Array(out)
}

fn inspect_merges(root: &Element, start: CellRef, end: CellRef) -> Value {
    let mut items: Vec<(CellRef, String)> = Vec::new();
    let Some(merges) = root.child_by_local("mergeCells") else {
        return json!([]);
    };
    for m in merges.children_by_local("mergeCell") {
        let Some(pref) = m.attr("ref") else {
            continue;
        };
        let Ok((a, b)) = a1::parse_range(pref) else {
            continue;
        };
        if !ranges_intersect(a, b, start, end) {
            continue;
        }
        let norm = format!(
            "{}:{}",
            a1::format_cell(a.col, a.row),
            a1::format_cell(b.col, b.row)
        );
        items.push((a, norm));
    }
    items.sort_by_key(|(c, _)| (c.row, c.col));
    Value::Array(items.into_iter().map(|(_, s)| json!(s)).collect())
}

pub(crate) fn apply_set_col_width(
    worksheet: &mut Element,
    op: &SetColWidthOp,
) -> Result<(), Error> {
    let mut pieces = collect_col_pieces(worksheet);
    let (a, b) = (op.min_col, op.max_col);
    let mut next = Vec::new();
    let mut covered = Vec::new();
    for piece in pieces.drain(..) {
        if piece.max < a || piece.min > b {
            next.push(piece);
            continue;
        }
        if piece.min < a {
            next.push(ColPiece {
                min: piece.min,
                max: a - 1,
                attrs: piece.attrs.clone(),
            });
        }
        let mid_min = piece.min.max(a);
        let mid_max = piece.max.min(b);
        let mut mid = ColPiece {
            min: mid_min,
            max: mid_max,
            attrs: piece.attrs.clone(),
        };
        apply_col_patch(&mut mid, op);
        if mid.keep() {
            next.push(mid);
        }
        covered.push((mid_min, mid_max));
        if piece.max > b {
            next.push(ColPiece {
                min: b + 1,
                max: piece.max,
                attrs: piece.attrs,
            });
        }
    }
    for (gap_min, gap_max) in uncovered_spans(a, b, &covered) {
        let mut mid = ColPiece {
            min: gap_min,
            max: gap_max,
            attrs: Vec::new(),
        };
        apply_col_patch(&mut mid, op);
        if mid.keep() {
            next.push(mid);
        }
    }
    next.sort_by_key(|p| p.min);
    write_cols(worksheet, next);
    Ok(())
}

pub(crate) fn apply_set_row_height(
    worksheet: &mut Element,
    op: &SetRowHeightOp,
) -> Result<(), Error> {
    let sheet_data = apply::ensure_sheet_data(worksheet);
    for r in op.row..=op.to_row {
        match &op.height {
            Patch::Set(ht) => {
                let row = apply::ensure_row(sheet_data, r);
                row.set_attr("ht", fmt_num(*ht));
                row.set_attr("customHeight", "1");
            }
            Patch::Clear => {
                clear_row_height(sheet_data, r);
            }
            Patch::Absent => {}
        }
    }
    Ok(())
}

pub(crate) fn apply_merge_range(
    worksheet: &mut Element,
    op: &MergeRangeOp,
    op_index: usize,
) -> Result<(), Error> {
    let start = op.start;
    let end = op.end;
    if start == end {
        return Err(Error::with_path(
            ErrorKind::MergeNeedsMultipleCells,
            format!("ops[{op_index}].range"),
        ));
    }
    for row in start.row..=end.row {
        for col in start.col..=end.col {
            apply::reject_shared_formula_master(worksheet, CellRef { col, row }, op_index)?;
        }
    }
    {
        let sheet_data = apply::ensure_sheet_data(worksheet);
        for row in start.row..=end.row {
            for col in start.col..=end.col {
                if col == start.col && row == start.row {
                    continue;
                }
                apply::clear_cell(sheet_data, CellRef { col, row });
            }
        }
    }
    apply::expand_dimension(worksheet, start);
    apply::expand_dimension(worksheet, end);
    remove_intersecting_merges(worksheet, start, end);
    let ref_s = format!(
        "{}:{}",
        a1::format_cell(start.col, start.row),
        a1::format_cell(end.col, end.row)
    );
    let el = Element::new("mergeCell").with_attr("ref", ref_s);
    let parent = ensure_merge_cells(worksheet);
    parent.push_element(el);
    set_merge_count(parent);
    Ok(())
}

pub(crate) fn apply_unmerge_range(worksheet: &mut Element, op: &UnmergeRangeOp) {
    remove_intersecting_merges(worksheet, op.start, op.end);
}

fn apply_col_patch(piece: &mut ColPiece, op: &SetColWidthOp) {
    match &op.width {
        Patch::Set(w) => {
            set_attr(&mut piece.attrs, "width", fmt_num(chars_to_xml_width(*w)));
            set_attr(&mut piece.attrs, "customWidth", "1");
            if op.best_fit != Some(true) {
                remove_attr(&mut piece.attrs, "bestFit");
            }
        }
        Patch::Clear => {
            remove_attr(&mut piece.attrs, "width");
            remove_attr(&mut piece.attrs, "customWidth");
        }
        Patch::Absent => {}
    }
    match op.best_fit {
        Some(true) => set_attr(&mut piece.attrs, "bestFit", "1"),
        Some(false) => remove_attr(&mut piece.attrs, "bestFit"),
        None => {}
    }
}

#[derive(Clone)]
struct ColPiece {
    min: u32,
    max: u32,
    attrs: Vec<(String, String)>,
}

impl ColPiece {
    fn keep(&self) -> bool {
        self.attrs.iter().any(|(k, _)| k != "min" && k != "max")
    }

    fn to_element(&self) -> Element {
        let mut el = Element::new("col");
        el.set_attr("min", self.min.to_string());
        el.set_attr("max", self.max.to_string());
        for (k, v) in &self.attrs {
            if k == "min" || k == "max" {
                continue;
            }
            el.set_attr(k, v.clone());
        }
        el
    }
}

fn collect_col_pieces(worksheet: &Element) -> Vec<ColPiece> {
    let mut out = Vec::new();
    for cols in worksheet.children_by_local("cols") {
        for col in cols.children_by_local("col") {
            let Some((min, max)) = col_span(col) else {
                continue;
            };
            let attrs = col
                .attrs
                .iter()
                .filter(|(k, _)| k != "min" && k != "max")
                .cloned()
                .collect();
            out.push(ColPiece { min, max, attrs });
        }
    }
    out
}

fn write_cols(worksheet: &mut Element, pieces: Vec<ColPiece>) {
    worksheet.children.retain(|n| match n {
        Node::Element(e) => e.local_name() != "cols",
        Node::Text(_) => true,
    });
    if pieces.is_empty() {
        return;
    }
    let mut cols = Element::new("cols");
    for p in pieces {
        cols.push_element(p.to_element());
    }
    insert_child_ordered(worksheet, cols, "cols");
}

fn uncovered_spans(a: u32, b: u32, covered: &[(u32, u32)]) -> Vec<(u32, u32)> {
    let mut cov = covered.to_vec();
    cov.sort_by_key(|x| x.0);
    let mut out = Vec::new();
    let mut cursor = a;
    for (lo, hi) in cov {
        if lo > cursor {
            out.push((cursor, lo - 1));
        }
        if hi + 1 > cursor {
            cursor = hi + 1;
        }
    }
    if cursor <= b {
        out.push((cursor, b));
    }
    out
}

fn clear_row_height(sheet_data: &mut Element, r: u32) {
    let Some(idx) = apply::find_row_index(sheet_data, r) else {
        return;
    };
    let Node::Element(row) = &mut sheet_data.children[idx] else {
        return;
    };
    row.remove_attr("ht");
    row.remove_attr("customHeight");
    let only_r = row.attrs.iter().all(|(k, _)| k == "r")
        && row.element_children().all(|e| e.local_name() != "c");
    if only_r {
        sheet_data.children.remove(idx);
    }
}

fn remove_intersecting_merges(worksheet: &mut Element, start: CellRef, end: CellRef) {
    let Some(pos) = worksheet
        .children
        .iter()
        .position(|n| matches!(n, Node::Element(e) if e.local_name() == "mergeCells"))
    else {
        return;
    };
    let Node::Element(parent) = &mut worksheet.children[pos] else {
        return;
    };
    parent.children.retain(|n| match n {
        Node::Element(e) if e.local_name() == "mergeCell" => {
            let Some(pref) = e.attr("ref") else {
                return true;
            };
            let Ok((a, b)) = a1::parse_range(pref) else {
                return true;
            };
            !ranges_intersect(a, b, start, end)
        }
        _ => true,
    });
    if parent.children_by_local("mergeCell").is_empty() {
        worksheet.children.remove(pos);
        return;
    }
    set_merge_count(parent);
}

fn ensure_merge_cells(worksheet: &mut Element) -> &mut Element {
    if worksheet.child_by_local("mergeCells").is_none() {
        insert_child_ordered(worksheet, Element::new("mergeCells"), "mergeCells");
    }
    worksheet.child_by_local_mut("mergeCells").unwrap()
}

fn set_merge_count(parent: &mut Element) {
    let n = parent.children_by_local("mergeCell").len();
    parent.set_attr("count", n.to_string());
}

fn insert_child_ordered(root: &mut Element, el: Element, local: &str) {
    let want = WORKSHEET_CHILD_ORDER
        .iter()
        .position(|n| *n == local)
        .unwrap_or(WORKSHEET_CHILD_ORDER.len());
    let mut at = root.children.len();
    for (i, n) in root.children.iter().enumerate() {
        let Node::Element(e) = n else { continue };
        let pos = WORKSHEET_CHILD_ORDER
            .iter()
            .position(|n| *n == e.local_name())
            .unwrap_or(WORKSHEET_CHILD_ORDER.len());
        if pos > want {
            at = i;
            break;
        }
    }
    root.children.insert(at, Node::Element(el));
}

fn col_span(col: &Element) -> Option<(u32, u32)> {
    let min = col.attr("min")?.parse().ok()?;
    let max = col.attr("max")?.parse().ok()?;
    if min == 0 || max == 0 || min > max {
        return None;
    }
    Some((min, max))
}

fn attr_true(el: &Element, name: &str) -> bool {
    matches!(
        el.attr(name).map(str::trim),
        Some("1" | "true" | "TRUE" | "on")
    )
}

fn parse_xml_f64(s: &str) -> Option<f64> {
    s.trim().parse().ok().filter(|n: &f64| n.is_finite())
}

/// §18.3.1.13 示例：Calibri 11 点在 96 dpi 下最大数字宽度为 7 像素。
const MAX_DIGIT_WIDTH: f64 = 7.0;

fn chars_to_xml_width(chars: f64) -> f64 {
    let raw = (chars * MAX_DIGIT_WIDTH + 5.0) / MAX_DIGIT_WIDTH * 256.0;
    raw.trunc() / 256.0
}

fn xml_width_to_chars(xml: f64) -> f64 {
    let chars = xml - 5.0 / MAX_DIGIT_WIDTH;
    (chars * 100.0).round() / 100.0
}

fn json_num(n: f64) -> Value {
    if n.fract() == 0.0 && n.abs() <= i64::MAX as f64 {
        json!(n as i64)
    } else {
        json!(n)
    }
}

fn fmt_num(n: f64) -> String {
    if n.fract() == 0.0 && n.abs() < 1e15 {
        format!("{}", n as i64)
    } else {
        let s = format!("{n}");
        s
    }
}

fn ranges_intersect(a1: CellRef, b1: CellRef, a2: CellRef, b2: CellRef) -> bool {
    !(b1.col < a2.col || b2.col < a1.col || b1.row < a2.row || b2.row < a1.row)
}

fn set_attr(attrs: &mut Vec<(String, String)>, key: &str, value: impl Into<String>) {
    let value = value.into();
    if let Some((_, v)) = attrs.iter_mut().find(|(k, _)| k == key) {
        *v = value;
        return;
    }
    attrs.push((key.to_string(), value));
}

fn remove_attr(attrs: &mut Vec<(String, String)>, key: &str) {
    attrs.retain(|(k, _)| k != key);
}

#[cfg(test)]
mod tests {
    use super::{chars_to_xml_width, xml_width_to_chars};

    #[test]
    fn width_roundtrip_twenty() {
        let xml = chars_to_xml_width(20.0);
        assert!((xml - 20.7109375).abs() < 1e-9);
        assert_eq!(xml_width_to_chars(xml), 20.0);
    }

    #[test]
    fn xml_twenty_is_excel_nineteen_point_two_nine() {
        assert_eq!(xml_width_to_chars(20.0), 19.29);
    }
}
