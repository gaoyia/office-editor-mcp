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
            let best_fit = crate::xml::attr_is_true_trimmed(col, "bestFit");
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

const MAX_OUTLINE: u32 = 7;

fn col_outline_level(piece: &ColPiece) -> u32 {
    piece
        .attrs
        .iter()
        .find(|(k, _)| k == "outlineLevel")
        .and_then(|(_, v)| v.parse().ok())
        .unwrap_or(0)
}

fn col_level_at(pieces: &[ColPiece], col: u32) -> u32 {
    pieces
        .iter()
        .find(|p| p.min <= col && col <= p.max)
        .map(col_outline_level)
        .unwrap_or(0)
}

/// 给闭区间内每列的 `outlineLevel` 加 / 减 1。`delta` 只能是 `1` 或 `-1`。
pub(crate) fn apply_col_outline_delta(
    worksheet: &mut Element,
    min_col: u32,
    max_col: u32,
    delta: i32,
    op_index: usize,
) -> Result<(), Error> {
    if delta > 0 {
        let pieces = collect_col_pieces(worksheet);
        for c in min_col..=max_col {
            let lvl = col_level_at(&pieces, c);
            if lvl.saturating_add(delta as u32) > MAX_OUTLINE {
                return Err(Error::with_path(
                    ErrorKind::OutlineLevelMax,
                    format!("ops[{op_index}].col"),
                ));
            }
        }
    }
    map_col_span(worksheet, min_col, max_col, delta > 0, |piece| {
        let cur = col_outline_level(piece);
        let next = if delta > 0 {
            cur.saturating_add(delta as u32)
        } else {
            cur.saturating_sub((-delta) as u32)
        };
        if next == 0 {
            remove_attr(&mut piece.attrs, "outlineLevel");
        } else {
            ensure_default_col_width(piece);
            set_attr(&mut piece.attrs, "outlineLevel", next.to_string());
        }
    });
    Ok(())
}

/// 给闭区间内每列写或去掉布尔属性（`hidden` / `collapsed`）。
pub(crate) fn apply_col_flag(
    worksheet: &mut Element,
    min_col: u32,
    max_col: u32,
    attr: &str,
    on: bool,
) {
    map_col_span(worksheet, min_col, max_col, on, |piece| {
        if on {
            ensure_default_col_width(piece);
            set_attr(&mut piece.attrs, attr, "1");
        } else {
            remove_attr(&mut piece.attrs, attr);
        }
    });
}

/// 写或去掉列的 `@hidden`。已有 `width` 不改成 0；取消隐藏时若宽度为 0 则改回缺省列宽。
pub(crate) fn apply_col_hidden(worksheet: &mut Element, min_col: u32, max_col: u32, hidden: bool) {
    map_col_span(worksheet, min_col, max_col, true, |piece| {
        if hidden {
            ensure_default_col_width(piece);
            set_attr(&mut piece.attrs, "hidden", "1");
        } else {
            remove_attr(&mut piece.attrs, "hidden");
            restore_zero_col_width(piece);
        }
    });
}

/// 新建 `<col>` 时补缺省列宽，避免 Excel 修复。已有 `width` 不改。
fn ensure_default_col_width(piece: &mut ColPiece) {
    if piece.attrs.iter().any(|(k, _)| k == "width") {
        return;
    }
    set_attr(
        &mut piece.attrs,
        "width",
        fmt_num(chars_to_xml_width(DEFAULT_COL_CHARS)),
    );
}

/// Excel 另存隐藏列常见 `width="0"`；去掉 hidden 后若不改宽，看起来仍是隐藏。
fn restore_zero_col_width(piece: &mut ColPiece) {
    let Some(raw) = piece
        .attrs
        .iter()
        .find(|(k, _)| k == "width")
        .map(|(_, v)| v.clone())
    else {
        return;
    };
    let Some(n) = parse_xml_f64(&raw) else {
        return;
    };
    if n != 0.0 {
        return;
    }
    set_attr(
        &mut piece.attrs,
        "width",
        fmt_num(chars_to_xml_width(DEFAULT_COL_CHARS)),
    );
}

fn map_col_span(
    worksheet: &mut Element,
    a: u32,
    b: u32,
    create_missing: bool,
    mut patch: impl FnMut(&mut ColPiece),
) {
    let mut pieces = collect_col_pieces(worksheet);
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
        patch(&mut mid);
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
    if create_missing {
        for (gap_min, gap_max) in uncovered_spans(a, b, &covered) {
            let mut mid = ColPiece {
                min: gap_min,
                max: gap_max,
                attrs: Vec::new(),
            };
            patch(&mut mid);
            if mid.keep() {
                next.push(mid);
            }
        }
    }
    next.sort_by_key(|p| p.min);
    write_cols(worksheet, next);
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
    apply::reject_merge_formula_conflicts(worksheet, start, end, op_index)?;
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
        Node::Text(_) | Node::Raw(_) => true,
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

/// 保证有 `sheetFormatPr`，且带规范必填的 `defaultRowHeight`。
/// 已有该元素时不覆盖已有行高。
pub(crate) fn ensure_sheet_format_pr(sheet: &mut Element) -> &mut Element {
    if sheet.child_by_local("sheetFormatPr").is_none() {
        let mut pr = Element::new("sheetFormatPr");
        pr.set_attr("defaultRowHeight", fmt_num(DEFAULT_ROW_PT));
        insert_child_ordered(sheet, pr, "sheetFormatPr");
    } else if sheet
        .child_by_local("sheetFormatPr")
        .unwrap()
        .attr("defaultRowHeight")
        .is_none()
    {
        sheet
            .child_by_local_mut("sheetFormatPr")
            .unwrap()
            .attrs
            .insert(0, ("defaultRowHeight".into(), fmt_num(DEFAULT_ROW_PT)));
    }
    sheet.child_by_local_mut("sheetFormatPr").unwrap()
}

pub(crate) fn insert_child_ordered(root: &mut Element, el: Element, local: &str) {
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

/// 拆分窗格 `ySplit`：Excel 16.0 金样 `SplitRow=4` 对应 `ySplit=1425`，不是 15 磅 × 20。
pub(crate) fn default_row_split_twentieths(rows: u32) -> String {
    fmt_num(f64::from(rows) * 1425.0 / 4.0)
}

/// 拆分窗格 `xSplit`：缺省列宽 8.43 字符换成像素再换成 1/20 磅。
pub(crate) fn default_col_split_twentieths(cols: u32) -> String {
    let px = (DEFAULT_COL_CHARS * MAX_DIGIT_WIDTH + 5.0).floor().max(0.0);
    fmt_num(px * 15.0 * f64::from(cols))
}

/// 组合写入用：列宽字符 → 像素时的缺省（Calibri 11 / 96dpi 的 8.43 → 64 像素）。
const DEFAULT_COL_CHARS: f64 = 8.43;
const DEFAULT_ROW_PT: f64 = 15.0;
pub(crate) const EMU_PER_PIXEL: i64 = 9525;
pub(crate) const EMU_PER_POINT: i64 = 12_700;
pub(crate) const MAX_COL: u32 = 16_384;
pub(crate) const MAX_ROW: u32 = 1_048_576;

fn chars_to_emu(chars: f64) -> i64 {
    let px = (chars * MAX_DIGIT_WIDTH + 5.0).floor().max(0.0) as i64;
    px.saturating_mul(EMU_PER_PIXEL)
}

fn points_to_emu(pt: f64) -> i64 {
    (pt * EMU_PER_POINT as f64).round() as i64
}

fn default_col_emu(sheet: &Element) -> i64 {
    sheet
        .child_by_local("sheetFormatPr")
        .and_then(|e| e.attr_local("defaultColWidth"))
        .and_then(parse_xml_f64)
        .map(chars_to_emu)
        .unwrap_or_else(|| chars_to_emu(DEFAULT_COL_CHARS))
}

fn default_row_emu(sheet: &Element) -> i64 {
    let pt = sheet
        .child_by_local("sheetFormatPr")
        .and_then(|e| e.attr_local("defaultRowHeight"))
        .and_then(parse_xml_f64)
        .unwrap_or(DEFAULT_ROW_PT);
    points_to_emu(pt).max(1)
}

fn col_width_emu(sheet: &Element, col: u32) -> i64 {
    for cols in sheet.children_by_local("cols") {
        for el in cols.children_by_local("col") {
            let Some((min, max)) = col_span(el) else {
                continue;
            };
            if col >= min && col <= max {
                if let Some(w) = el.attr("width").and_then(parse_xml_f64) {
                    return chars_to_emu(xml_width_to_chars(w)).max(1);
                }
            }
        }
    }
    default_col_emu(sheet).max(1)
}

pub(crate) fn col_origin_emu(sheet: &Element, col: u32) -> i64 {
    let end = col.min(MAX_COL.saturating_add(1));
    let mut x = 0i64;
    for c in 1..end {
        x = x.saturating_add(col_width_emu(sheet, c));
    }
    x
}

pub(crate) fn row_origin_emu(sheet: &Element, row: u32) -> i64 {
    if row <= 1 {
        return 0;
    }
    let def = default_row_emu(sheet).max(1);
    let mut y = 0i64;
    let mut cursor = 1u32;
    for (cr, ch) in custom_row_heights(sheet) {
        if cr >= row {
            break;
        }
        if cr > cursor {
            y = y.saturating_add(def.saturating_mul((cr - cursor) as i64));
        }
        y = y.saturating_add(ch);
        cursor = cr.saturating_add(1);
    }
    if row > cursor {
        y = y.saturating_add(def.saturating_mul((row - cursor) as i64));
    }
    y
}

fn custom_row_heights(sheet: &Element) -> Vec<(u32, i64)> {
    let mut out = Vec::new();
    let Some(sheet_data) = sheet.child_by_local("sheetData") else {
        return out;
    };
    for el in sheet_data.children_by_local("row") {
        let Some(r) = el.attr("r").and_then(|s| s.parse::<u32>().ok()) else {
            continue;
        };
        if let Some(ht) = el.attr("ht").and_then(parse_xml_f64) {
            out.push((r, points_to_emu(ht).max(1)));
        }
    }
    out.sort_by_key(|(r, _)| *r);
    out
}

pub(crate) fn emu_to_marker(sheet: &Element, x: i64, y: i64) -> (CellRef, i64, i64) {
    let (col, col_off) = emu_to_col(sheet, x.max(0));
    let (row, row_off) = emu_to_row(sheet, y.max(0));
    (CellRef { col, row }, col_off, row_off)
}

fn emu_to_col(sheet: &Element, x: i64) -> (u32, i64) {
    let mut acc = 0i64;
    for c in 1..=MAX_COL {
        let w = col_width_emu(sheet, c);
        if acc.saturating_add(w) > x || c == MAX_COL {
            return (c, x.saturating_sub(acc));
        }
        acc = acc.saturating_add(w);
    }
    (MAX_COL, 0)
}

fn emu_to_row(sheet: &Element, y: i64) -> (u32, i64) {
    let def = default_row_emu(sheet).max(1);
    let mut acc = 0i64;
    let mut r = 1u32;
    for (cr, ch) in custom_row_heights(sheet) {
        if cr > r {
            let gap = (cr - r) as i64;
            let gap_emu = def.saturating_mul(gap);
            if acc.saturating_add(gap_emu) > y {
                let remain = y.saturating_sub(acc);
                let extra = (remain / def) as u32;
                return ((r.saturating_add(extra)).clamp(1, MAX_ROW), remain % def);
            }
            acc = acc.saturating_add(gap_emu);
            r = cr;
        }
        if cr == r {
            if acc.saturating_add(ch) > y {
                return (r, y.saturating_sub(acc));
            }
            acc = acc.saturating_add(ch);
            r = r.saturating_add(1);
        }
    }
    let remain = y.saturating_sub(acc);
    let extra = (remain / def) as u32;
    (r.saturating_add(extra).clamp(1, MAX_ROW), remain % def)
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

    #[test]
    fn huge_column_does_not_walk_every_index() {
        let sheet = crate::xml::Element::new("worksheet");
        let origin = super::col_origin_emu(&sheet, u32::MAX);
        let capped = super::col_origin_emu(&sheet, super::MAX_COL.saturating_add(1));
        assert_eq!(origin, capped);
    }
}
