//! 工作表已用区域出图。像素对齐 Excel 16.0 的 `Range.CopyPicture`（xlScreen、xlBitmap）。
//! 不乘工作表 `zoomScale`。缩放 95 的销售看板金样仍是 100% 的 1095×418。
//! `showGridLines` 为假时不画网格，上边和左边的外框留白，宽高不变。
//! 隐藏行和隐藏列不占像素。这一边全部隐藏时只留 1 像素白。
//! 从右到左且没有文字时，把整张图水平翻转。有文字时不翻。
//! 两种纯色的图案铺在和纯色同一块矩形上。点的位置用整张图的像素，不从格子左上角重算。
//! 虚线、点线、发丝、双线和点划线按整图坐标取余。偶数宽度多出的一像素朝坐标较小的一侧。
//! 没写行高时，没写 `defaultRowHeight` 的空行是 14.25 磅。
//! 写了这个属性但 `customHeight` 不是真时，不采用这个数字。
//! `customHeight` 为真时，空行和常规字体的行用写下的磅值。
//! 没有描边、没有字的纯色矩形画在边框上面。锚点 EMU 除以 9525 后四舍五入。
//! 上边和左边的外框盖住矩形。

use std::collections::{HashMap, HashSet};
use std::io::Cursor;

use crate::error::{Error, ErrorKind, IdReason};
use crate::opc::{Package, Relationships};
use crate::xml::{Element, Node};

use super::a1::{self, CellRef};
use super::chart_gdi::Face;
use super::eval::{self, Displayed, FormulaOutcome, FormulaSession};
use super::inspect;
use super::picture::{self, AnchorKind};
use super::ExportedMedia;
use crate::xlsx::fill_device_stroked_rect;

const MAX_PX: i32 = 4096;
/// 切块张数上限：`tilePx=1` 时防海量文件/编码爆炸；超限在分配任何块之前 NI。
const MAX_TILE_COUNT: u32 = 1024;
/// 全部切块像素合计上限（≈全图 RGB 体量）；与 MAX_TILE_COUNT 双闸。
const MAX_TILE_TOTAL_PX: u64 = 64 * 1024 * 1024;
const GRID: [u8; 3] = [224, 224, 224];
const DEFAULT_COL_PX: i32 = 64;
/// 没写 `defaultRowHeight` 时的空行。Excel 的标准行高是 14.25 磅，19 像素。
const DEFAULT_ROW_PT: f64 = 14.25;

pub fn export_sheet_image(bytes: &[u8], id: &str) -> Result<ExportedMedia, Error> {
    let name = parse_sheet_id(id)?;
    let pkg = Package::open_bytes(bytes.to_vec())?;
    let sheet_part = picture::lookup_sheet_part(&pkg, &name, "id")?;
    let sheet_doc = super::list_pin::parse_part(&pkg, &sheet_part)?;
    let root = &sheet_doc.root;
    reject_rtl_extras(root)?;
    reject_hidden_rules(root)?;
    let grid_lines = shows_grid(root);
    let rtl = right_to_left(root);
    let wb_part = crate::opc::office_document_part(&pkg)?;
    let rels = Relationships::from_package(&pkg, &wb_part)?;
    let sst = inspect::load_shared_strings(&pkg, &wb_part, &rels)?;
    let styles = load_styles(&pkg, &wb_part, &rels)?;
    let (c0, r0, c1, r1) = used_range(root)?;
    let mut session = eval::FormulaSession::open(&pkg, &name)?;
    let cells = read_cells(root, &sst, &mut session)?;
    let cols = column_sizes(root, c0, c1)?;
    let rows = row_sizes(root, &styles, &cells, r0, r1)?;
    let theme = loaded_theme(&pkg, &wb_part, &rels);
    let shapes = plain_rects(
        &pkg,
        &sheet_part,
        root,
        &styles,
        &cells,
        c0,
        r0,
        c1,
        r1,
        &cols,
        &rows,
        theme.as_ref(),
    )?;
    let canvas = paint(
        root,
        &styles,
        &cells,
        c0,
        r0,
        &cols,
        &rows,
        grid_lines,
        rtl,
        &shapes,
        &mut session,
    )?;
    let png = encode_png(canvas.w, canvas.h, &canvas.px)?;
    Ok(ExportedMedia {
        bytes: png,
        content_type: Some("image/png".to_string()),
    })
}

/// 切块导出：每块边长 ≤ `tile_px`（且 ≤ `MAX_PX`），不分配整表画布。
/// 返回顺序行主序 `(row_idx, col_idx, png_bytes)`。单块未超限时仍走整图路径（与旧行为同像素）。
pub fn export_sheet_image_tiles(
    bytes: &[u8],
    id: &str,
    tile_px: i32,
) -> Result<Vec<SheetImageTile>, Error> {
    if !(1..=MAX_PX).contains(&tile_px) {
        return Err(unsupported());
    }
    let name = parse_sheet_id(id)?;
    let pkg = Package::open_bytes(bytes.to_vec())?;
    let sheet_part = picture::lookup_sheet_part(&pkg, &name, "id")?;
    let sheet_doc = super::list_pin::parse_part(&pkg, &sheet_part)?;
    let root = &sheet_doc.root;
    reject_rtl_extras(root)?;
    reject_hidden_rules(root)?;
    let grid_lines = shows_grid(root);
    let rtl = right_to_left(root);
    if rtl {
        // 切块 + RTL 翻转语义未定，明确 NI（默认单图 API 仍可画 RTL）。
        return Err(unsupported());
    }
    let wb_part = crate::opc::office_document_part(&pkg)?;
    let rels = Relationships::from_package(&pkg, &wb_part)?;
    let sst = inspect::load_shared_strings(&pkg, &wb_part, &rels)?;
    let styles = load_styles(&pkg, &wb_part, &rels)?;
    let (c0, r0, c1, r1) = used_range(root)?;
    let mut session = eval::FormulaSession::open(&pkg, &name)?;
    let cells = read_cells(root, &sst, &mut session)?;
    let cols = column_sizes(root, c0, c1)?;
    let rows = row_sizes(root, &styles, &cells, r0, r1)?;
    let theme = loaded_theme(&pkg, &wb_part, &rels);
    let shapes = plain_rects(
        &pkg,
        &sheet_part,
        root,
        &styles,
        &cells,
        c0,
        r0,
        c1,
        r1,
        &cols,
        &rows,
        theme.as_ref(),
    )?;
    let xs = Axis::new(&cols);
    let ys = Axis::new(&rows);
    let full_w = xs.bitmap();
    let full_h = ys.bitmap();
    if full_w < 1 || full_h < 1 {
        return Err(unsupported());
    }
    // 未超 tile：整图一次，保证与 export_sheet_image 同像素。
    if full_w <= tile_px && full_h <= tile_px {
        let canvas = paint(
            root,
            &styles,
            &cells,
            c0,
            r0,
            &cols,
            &rows,
            grid_lines,
            rtl,
            &shapes,
            &mut session,
        )?;
        let png = encode_png(canvas.w, canvas.h, &canvas.px)?;
        return Ok(vec![SheetImageTile {
            row: 0,
            col: 0,
            bytes: png,
        }]);
    }
    // 超限切块：先算格网与总像素，失败不分配/不编码任何块（CLI 亦不会落盘）。
    let tiles_x = ((full_w + tile_px - 1) / tile_px) as u32;
    let tiles_y = ((full_h + tile_px - 1) / tile_px) as u32;
    let tile_count = tiles_x.checked_mul(tiles_y).ok_or_else(unsupported)?;
    if tile_count == 0 || tile_count > MAX_TILE_COUNT {
        return Err(unsupported());
    }
    let total_px = (full_w as u64)
        .checked_mul(full_h as u64)
        .ok_or_else(unsupported)?;
    if total_px > MAX_TILE_TOTAL_PX {
        return Err(unsupported());
    }
    let mut out = Vec::with_capacity(tile_count as usize);
    let mut oy = 0;
    let mut row = 0u32;
    while oy < full_h {
        let th = (full_h - oy).min(tile_px);
        let mut ox = 0;
        let mut col = 0u32;
        while ox < full_w {
            let tw = (full_w - ox).min(tile_px);
            let canvas = paint_window(
                root,
                &styles,
                &cells,
                c0,
                r0,
                &cols,
                &rows,
                grid_lines,
                rtl,
                &shapes,
                &mut session,
                ox,
                oy,
                tw,
                th,
            )?;
            let png = encode_png(canvas.w, canvas.h, &canvas.px)?;
            out.push(SheetImageTile {
                row,
                col,
                bytes: png,
            });
            ox += tw;
            col += 1;
        }
        oy += th;
        row += 1;
    }
    Ok(out)
}

/// 一块切块 PNG。
#[derive(Debug, Clone)]
pub struct SheetImageTile {
    pub row: u32,
    pub col: u32,
    pub bytes: Vec<u8>,
}

fn unsupported() -> Error {
    Error::with_path(ErrorKind::SheetImageUnsupported, "id")
}

fn parse_sheet_id(id: &str) -> Result<String, Error> {
    let Some(name) = id.strip_prefix("sheet:") else {
        return Err(bad_id());
    };
    if name.is_empty() || name.contains('/') {
        return Err(bad_id());
    }
    Ok(name.to_string())
}

fn bad_id() -> Error {
    Error::with_path(
        ErrorKind::InvalidId {
            reason: IdReason::ExportSheetNeedsSheet,
        },
        "id",
    )
}

fn right_to_left(sheet: &Element) -> bool {
    let Some(view) = sheet
        .child_by_local("sheetViews")
        .and_then(|views| views.child_by_local("sheetView"))
    else {
        return false;
    };
    truthy(view.attr("rightToLeft"))
}

/// 没有文字时，Excel 的从右到左就是整图水平翻转。有文字时字仍正写，不能翻。
/// 条件格式和筛选按钮的方向还没对过。
fn reject_rtl_extras(sheet: &Element) -> Result<(), Error> {
    if !right_to_left(sheet) {
        return Ok(());
    }
    if sheet.child_by_local("conditionalFormatting").is_some()
        || sheet.child_by_local("autoFilter").is_some()
    {
        return Err(unsupported());
    }
    Ok(())
}

/// 没写 `showGridLines` 时默认开着。`0` 和 `false` 关掉。
fn shows_grid(sheet: &Element) -> bool {
    let Some(view) = sheet
        .child_by_local("sheetViews")
        .and_then(|views| views.child_by_local("sheetView"))
    else {
        return true;
    };
    !view
        .attr("showGridLines")
        .is_some_and(|v| v == "0" || v.eq_ignore_ascii_case("false"))
}

/// 条件格式把隐藏格子算进最小、最大时，可见格子的颜色会变。这一期不猜，整表不画。
fn reject_hidden_rules(sheet: &Element) -> Result<(), Error> {
    let mut hidden_rows = HashSet::new();
    if let Some(data) = sheet.child_by_local("sheetData") {
        let mut prev = 0u32;
        for row in data.children_by_local("row") {
            let r = a1::row_after(prev, row.attr("r"));
            prev = r;
            if truthy(row.attr("hidden")) {
                hidden_rows.insert(r);
            }
        }
    }
    let mut hidden_cols = Vec::new();
    for cols in sheet.children_by_local("cols") {
        for el in cols.children_by_local("col") {
            if !truthy(el.attr("hidden")) {
                continue;
            }
            let min = el.attr("min").and_then(|s| s.parse::<u32>().ok());
            let max = el.attr("max").and_then(|s| s.parse::<u32>().ok());
            if let (Some(min), Some(max)) = (min, max) {
                hidden_cols.push((min, max));
            }
        }
    }
    if hidden_rows.is_empty() && hidden_cols.is_empty() {
        return Ok(());
    }
    for block in sheet.children_by_local("conditionalFormatting") {
        let areas = parse_areas(block.attr("sqref").unwrap_or(""))?;
        for (a, b) in areas {
            let r0 = a.row.min(b.row);
            let r1 = a.row.max(b.row);
            let c0 = a.col.min(b.col);
            let c1 = a.col.max(b.col);
            let row_hit = hidden_rows.iter().any(|row| *row >= r0 && *row <= r1);
            let col_hit = hidden_cols
                .iter()
                .any(|(min, max)| c0 <= *max && c1 >= *min);
            if row_hit || col_hit {
                return Err(unsupported());
            }
        }
    }
    Ok(())
}

fn truthy(value: Option<&str>) -> bool {
    match value {
        Some(v) if v == "0" || v.eq_ignore_ascii_case("false") => false,
        Some(_) => true,
        None => false,
    }
}

fn used_range(sheet: &Element) -> Result<(u32, u32, u32, u32), Error> {
    // dimension 经常比格子小。两边都在时取并集，避免只画左上角，也避免画到界外时越界。
    let dim = dimension_box(sheet)?;
    let cells = cell_bounds(sheet)?;
    match (dim, cells) {
        (Some(dim), Some(cells)) => Ok(union_box(dim, cells)),
        (Some(only), None) | (None, Some(only)) => Ok(only),
        (None, None) => Err(unsupported()),
    }
}

fn dimension_box(sheet: &Element) -> Result<Option<(u32, u32, u32, u32)>, Error> {
    let Some(dim) = sheet
        .child_by_local("dimension")
        .and_then(|el| el.attr("ref"))
        .filter(|dim| !dim.is_empty())
    else {
        return Ok(None);
    };
    let (a, b) = a1::parse_range(dim).map_err(|_| unsupported())?;
    Ok(Some(box_of(a, b)))
}

fn cell_bounds(sheet: &Element) -> Result<Option<(u32, u32, u32, u32)>, Error> {
    let Some(data) = sheet.child_by_local("sheetData") else {
        return Ok(None);
    };
    let mut c0 = u32::MAX;
    let mut r0 = u32::MAX;
    let mut c1 = 0u32;
    let mut r1 = 0u32;
    let mut any = false;
    let mut prev_row = 0u32;
    for row in data.children_by_local("row") {
        let r = a1::row_after(prev_row, row.attr("r"));
        prev_row = r;
        let mut prev_col = 0u32;
        for cell in row.children_by_local("c") {
            let (col, row_n) = cell_addr(prev_col, r, cell.attr("r"))?;
            prev_col = col;
            any = true;
            c0 = c0.min(col);
            r0 = r0.min(row_n);
            c1 = c1.max(col);
            r1 = r1.max(row_n);
        }
    }
    if !any {
        return Ok(None);
    }
    Ok(Some((c0, r0, c1, r1)))
}

fn box_of(a: CellRef, b: CellRef) -> (u32, u32, u32, u32) {
    (
        a.col.min(b.col),
        a.row.min(b.row),
        a.col.max(b.col),
        a.row.max(b.row),
    )
}

fn union_box(a: (u32, u32, u32, u32), b: (u32, u32, u32, u32)) -> (u32, u32, u32, u32) {
    (a.0.min(b.0), a.1.min(b.1), a.2.max(b.2), a.3.max(b.3))
}

fn cell_addr(prev_col: u32, row: u32, attr: Option<&str>) -> Result<(u32, u32), Error> {
    if let Some(r) = attr {
        let cell = a1::parse_cell(r).map_err(|_| unsupported())?;
        return Ok((cell.col, cell.row));
    }
    Ok((prev_col + 1, row))
}

#[derive(Clone, Copy)]
struct ShapeLine {
    color: [u8; 3],
    /// 仅 1 或 2（9525 / 19050 EMU）。
    px: i32,
}

struct PlainRect {
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    /// `None` = `a:noFill`（空心，仅描边时合法）。
    fill: Option<[u8; 3]>,
    /// `None` = 线 `noFill`（现有无描边纯色路径）。
    stroke: Option<ShapeLine>,
}

struct SheetPng {
    x0: i32,
    y0: i32,
    w: u32,
    h: u32,
    rgb: Vec<u8>,
}

enum Overlay {
    Fill(PlainRect),
    Png(SheetPng),
}

/// 压到区域上的图形：纯色矩形留下，其余仍让整表失败。
fn plain_rects(
    pkg: &Package,
    sheet_part: &str,
    sheet: &Element,
    styles: &Styles,
    cells: &HashMap<(u32, u32), CellRec>,
    c0: u32,
    r0: u32,
    c1: u32,
    r1: u32,
    cols: &[i32],
    rows: &[i32],
    theme: Option<&LoadedTheme>,
) -> Result<Vec<Overlay>, Error> {
    let mut out = Vec::new();
    for part in picture::drawing_parts(pkg, sheet_part)? {
        let doc = super::list_pin::parse_part(pkg, &part)?;
        walk_anchors_forward(&doc.root, &mut |anchor, kind| {
            if anchor.child_by_local("grpSp").is_some() {
                if anchor.child_by_local("pic").is_some() || anchor.child_by_local("sp").is_some() {
                    return Err(unsupported());
                }
                for rect in identity_group(
                    anchor, kind, sheet, styles, cells, c0, r0, cols, rows, theme,
                )? {
                    out.push(Overlay::Fill(rect));
                }
                return Ok(());
            }
            if !anchor_hits(anchor, kind, c0, r0, c1, r1)? {
                return Ok(());
            }
            let (x0, y0, x1, y1) =
                rect_edges(anchor, kind, sheet, styles, cells, c0, r0, cols, rows)?;
            if crosses_hidden(x0, y0, x1, y1, cols, rows) {
                return Err(unsupported());
            }
            if let Some(pic) = anchor.child_by_local("pic") {
                if right_to_left(sheet) {
                    return Err(unsupported());
                }
                out.push(Overlay::Png(sheet_png(pkg, &part, pic, x0, y0, x1, y1)?));
            } else if let Some(sp) = anchor.child_by_local("sp") {
                out.push(Overlay::Fill(plain_rect_at(
                    sp, theme, true, x0, y0, x1, y1,
                )?));
            } else {
                return Err(unsupported());
            }
            Ok(())
        })?;
    }
    Ok(out)
}

fn walk_anchors_forward(
    el: &Element,
    f: &mut impl FnMut(&Element, AnchorKind) -> Result<(), Error>,
) -> Result<(), Error> {
    if el.local_name() == "AlternateContent" {
        if let Some(index) = crate::xml::alternate_child(el)? {
            if let crate::xml::Node::Element(child) = &el.children[index] {
                return walk_anchors_forward(child, f);
            }
        }
        return Ok(());
    }
    if let Some(kind) = AnchorKind::from_local(el.local_name()) {
        return f(el, kind);
    }
    if el.local_name() == "grpSp" {
        return Err(unsupported());
    }
    for child in el.element_children() {
        walk_anchors_forward(child, f)?;
    }
    Ok(())
}

fn anchor_hits(
    anchor: &Element,
    kind: AnchorKind,
    c0: u32,
    r0: u32,
    c1: u32,
    r1: u32,
) -> Result<bool, Error> {
    match kind {
        AnchorKind::TwoCell => {
            let (from, _, _) = picture::read_marker(anchor.child_by_local("from"))?;
            let (to, _, _) = picture::read_marker(anchor.child_by_local("to"))?;
            Ok(match (from, to) {
                (Some(a), Some(b)) => {
                    overlaps(a.col, b.col, c0, c1) && overlaps(a.row, b.row, r0, r1)
                }
                _ => false,
            })
        }
        AnchorKind::OneCell => {
            let (from, _, _) = picture::read_marker(anchor.child_by_local("from"))?;
            Ok(from.is_some_and(|a| a.col <= c1 && a.row <= r1))
        }
        AnchorKind::Absolute => Ok(true),
        AnchorKind::RelSize | AnchorKind::AbsSize => Err(unsupported()),
    }
}

fn rect_edges(
    anchor: &Element,
    kind: AnchorKind,
    sheet: &Element,
    styles: &Styles,
    cells: &HashMap<(u32, u32), CellRec>,
    c0: u32,
    r0: u32,
    cols: &[i32],
    rows: &[i32],
) -> Result<(i32, i32, i32, i32), Error> {
    if kind == AnchorKind::Absolute {
        let pos = anchor.child_by_local("pos").ok_or_else(unsupported)?;
        let ext = anchor.child_by_local("ext").ok_or_else(unsupported)?;
        let x0 = round_px(emu_attr(pos, "x")?)?;
        let y0 = round_px(emu_attr(pos, "y")?)?;
        let cx = emu_attr(ext, "cx")?;
        let cy = emu_attr(ext, "cy")?;
        if cx <= 0 || cy <= 0 {
            return Err(unsupported());
        }
        let x1 = x0 + round_px(cx)?;
        let y1 = y0 + round_px(cy)?;
        return Ok((x0, y0, x1, y1));
    }
    let (from, col_off, row_off) = picture::read_marker(anchor.child_by_local("from"))?;
    let from = from.ok_or_else(unsupported)?;
    let x_from = axis_origin(from.col, c0, cols, &mut |col| column_px_of(sheet, col))?;
    let y_from = axis_origin(from.row, r0, rows, &mut |row| {
        row_px_one(sheet, styles, cells, row)
    })?;
    let (x1, y1) = match kind {
        AnchorKind::OneCell => {
            let ext = anchor.child_by_local("ext").ok_or_else(unsupported)?;
            let cx = emu_attr(ext, "cx")?;
            let cy = emu_attr(ext, "cy")?;
            if cx <= 0 || cy <= 0 {
                return Err(unsupported());
            }
            let x_end = col_off.checked_add(cx).ok_or_else(unsupported)?;
            let y_end = row_off.checked_add(cy).ok_or_else(unsupported)?;
            (x_from + round_px(x_end)?, y_from + round_px(y_end)?)
        }
        AnchorKind::TwoCell => {
            let (to, to_x, to_y) = picture::read_marker(anchor.child_by_local("to"))?;
            let to = to.ok_or_else(unsupported)?;
            let x_to = axis_origin(to.col, c0, cols, &mut |col| column_px_of(sheet, col))?;
            let y_to = axis_origin(to.row, r0, rows, &mut |row| {
                row_px_one(sheet, styles, cells, row)
            })?;
            (x_to + round_px(to_x)?, y_to + round_px(to_y)?)
        }
        AnchorKind::Absolute => unreachable!("absolute handled above"),
        AnchorKind::RelSize | AnchorKind::AbsSize => {
            return Err(unsupported());
        }
    };
    let x0 = x_from + round_px(col_off)?;
    let y0 = y_from + round_px(row_off)?;
    if x1 < x0 || y1 < y0 {
        return Err(unsupported());
    }
    Ok((x0, y0, x1, y1))
}

fn axis_origin(
    index: u32,
    start: u32,
    sizes: &[i32],
    extra: &mut impl FnMut(u32) -> Result<i32, Error>,
) -> Result<i32, Error> {
    if index >= start {
        let mut at = 0i32;
        for i in start..index {
            let rel = (i - start) as usize;
            let px = if rel < sizes.len() {
                sizes[rel]
            } else {
                extra(i)?
            };
            at = at.saturating_add(px);
        }
        Ok(at)
    } else {
        let mut at = 0i32;
        for i in index..start {
            at = at.saturating_sub(extra(i)?);
        }
        Ok(at)
    }
}

fn column_px_of(sheet: &Element, col: u32) -> Result<i32, Error> {
    let default = sheet
        .child_by_local("sheetFormatPr")
        .and_then(|el| el.attr("defaultColWidth"))
        .and_then(|s| s.parse().ok());
    column_px(sheet, col, default)
}

fn row_px_one(
    sheet: &Element,
    styles: &Styles,
    cells: &HashMap<(u32, u32), CellRec>,
    row: u32,
) -> Result<i32, Error> {
    let base = row_base(sheet, styles);
    if let Some(data) = sheet.child_by_local("sheetData") {
        let mut prev = 0u32;
        for el in data.children_by_local("row") {
            let r = a1::row_after(prev, el.attr("r"));
            prev = r;
            if r != row {
                continue;
            }
            if truthy(el.attr("hidden")) {
                return Ok(0);
            }
            if let Some(ht) = el.attr("ht").and_then(|s| s.parse::<f64>().ok()) {
                let px = excel_row_px(ht);
                if px < 1 {
                    return Err(unsupported());
                }
                return Ok(px);
            }
            break;
        }
    }
    let pt = row_auto_pt(sheet, styles, cells, row, &base)?;
    let px = excel_row_px(pt);
    if px < 1 {
        return Err(unsupported());
    }
    Ok(px)
}

fn emu_attr(el: &Element, name: &str) -> Result<i64, Error> {
    el.attr(name)
        .ok_or_else(unsupported)?
        .parse()
        .map_err(|_| unsupported())
}

/// 9525 是奇数，EMU 除出来不会正好落在半像素上。
fn round_px(emu: i64) -> Result<i32, Error> {
    if emu < 0 {
        return Err(unsupported());
    }
    let px = (emu as f64 / 9_525.0).round();
    if px > i32::MAX as f64 {
        return Err(unsupported());
    }
    Ok(px as i32)
}

fn plain_rect_at(
    sp: &Element,
    theme: Option<&LoadedTheme>,
    quarter: bool,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
) -> Result<PlainRect, Error> {
    let (fill, stroke) = plain_style(sp, theme, quarter)?;
    Ok(PlainRect {
        x0,
        y0,
        x1,
        y1,
        fill,
        stroke,
    })
}

/// 解析工作表 `xdr:sp` 的填充与描边。有描边时仅轴对齐、1/2px 实线；半透明填充明确 NI。
fn plain_style(
    sp: &Element,
    theme: Option<&LoadedTheme>,
    quarter: bool,
) -> Result<(Option<[u8; 3]>, Option<ShapeLine>), Error> {
    let sp_pr = sp.child_by_local("spPr").ok_or_else(unsupported)?;
    let geom = sp_pr.child_by_local("prstGeom").ok_or_else(unsupported)?;
    if geom.attr("prst") != Some("rect") || sp_pr.child_by_local("custGeom").is_some() {
        return Err(unsupported());
    }
    let stroked = sp_pr.child_by_local("ln").is_some_and(|ln| !clear_line(ln));
    if let Some(xfrm) = sp_pr.child_by_local("xfrm") {
        if stroked {
            // 描边子集：不旋转、不翻转（与 PPT 描边外角同口径，Excel 另验）。
            if xfrm.attr("rot").is_some_and(|rot| {
                rot.parse::<i64>()
                    .ok()
                    .is_some_and(|n| n.rem_euclid(21_600_000) != 0)
            }) {
                return Err(unsupported());
            }
            if truthy(xfrm.attr("flipH")) || truthy(xfrm.attr("flipV")) {
                return Err(unsupported());
            }
        } else if xfrm
            .attr("rot")
            .is_some_and(|rot| !quarter_turn(rot, quarter))
            || (!quarter && (truthy(xfrm.attr("flipH")) || truthy(xfrm.attr("flipV"))))
        {
            return Err(unsupported());
        }
    }
    if sp_pr
        .element_children()
        .any(|child| matches!(child.local_name(), "gradFill" | "blipFill" | "pattFill"))
    {
        return Err(unsupported());
    }
    let fill = if sp_pr.child_by_local("noFill").is_some() {
        if !stroked {
            return Err(unsupported());
        }
        None
    } else {
        let solid = sp_pr.child_by_local("solidFill").ok_or_else(unsupported)?;
        Some(shape_fill(solid, theme)?)
    };
    let stroke = match sp_pr.child_by_local("ln") {
        None => return Err(unsupported()),
        Some(ln) if clear_line(ln) => None,
        Some(ln) => Some(shape_line(ln, theme)?),
    };
    if fill.is_none() && stroke.is_none() {
        return Err(unsupported());
    }
    if visible_text(sp) {
        return Err(unsupported());
    }
    Ok((fill, stroke))
}

fn shape_line(ln: &Element, theme: Option<&LoadedTheme>) -> Result<ShapeLine, Error> {
    // 未知线型 / 虚线 / 箭头 / 显式接缝 / 非 ctr：NI，不静默当 solid。
    if ln.attr("algn").is_some_and(|a| a != "ctr") {
        return Err(unsupported());
    }
    if ln.child_by_local("prstDash").is_some()
        || ln.child_by_local("custDash").is_some()
        || ln.child_by_local("headEnd").is_some()
        || ln.child_by_local("tailEnd").is_some()
        || ln.child_by_local("miter").is_some()
        || ln.child_by_local("bevel").is_some()
        || ln.child_by_local("round").is_some()
        || ln.child_by_local("gradFill").is_some()
        || ln.child_by_local("pattFill").is_some()
        || ln.child_by_local("blipFill").is_some()
    {
        return Err(unsupported());
    }
    let solid = ln.child_by_local("solidFill").ok_or_else(unsupported)?;
    let color = shape_fill(solid, theme)?;
    let w_emu = ln
        .attr("w")
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(0);
    // 缺省或非 9525/19050（含 insertShape 曾用的默认外其它宽度）→ NI。
    let px = match w_emu {
        9525 => 1,
        19050 => 2,
        _ => return Err(unsupported()),
    };
    Ok(ShapeLine { color, px })
}

/// 纯色矩形的组合。可以缩放，也可以绕中心转 90 度的倍数，或翻转。坐标从 A1 量起。
fn identity_group(
    anchor: &Element,
    kind: AnchorKind,
    sheet: &Element,
    styles: &Styles,
    cells: &HashMap<(u32, u32), CellRec>,
    c0: u32,
    r0: u32,
    cols: &[i32],
    rows: &[i32],
    theme: Option<&LoadedTheme>,
) -> Result<Vec<PlainRect>, Error> {
    if kind != AnchorKind::TwoCell {
        return Err(unsupported());
    }
    let anchor_box = rect_edges(anchor, kind, sheet, styles, cells, c0, r0, cols, rows)?;
    let group = anchor.child_by_local("grpSp").ok_or_else(unsupported)?;
    let map = read_group_map(group)?;
    // 组里的 EMU 从 A1 左上角量起。格子下标从 1 起，不是从锚点那一格量起。
    let origin_x = axis_origin(1, c0, cols, &mut |col| column_px_of(sheet, col))?;
    let origin_y = axis_origin(1, r0, rows, &mut |row| {
        row_px_one(sheet, styles, cells, row)
    })?;
    if anchor_box != turned_box(&map, origin_x, origin_y)? {
        return Err(unsupported());
    }
    let rects = group_rects(group, &[map], origin_x, origin_y, cols, rows, theme)?;
    if rects.is_empty() {
        return Err(unsupported());
    }
    Ok(rects)
}

fn read_group_map(group: &Element) -> Result<GroupMap, Error> {
    let pr = group.child_by_local("grpSpPr").ok_or_else(unsupported)?;
    if pr
        .element_children()
        .any(|child| child.local_name() != "xfrm")
    {
        return Err(unsupported());
    }
    let xfrm = pr.child_by_local("xfrm").ok_or_else(unsupported)?;
    if xfrm
        .element_children()
        .any(|child| !matches!(child.local_name(), "off" | "ext" | "chOff" | "chExt"))
    {
        return Err(unsupported());
    }
    let (off_x, off_y, cx, cy) = box_emu(xfrm, "off", "ext")?;
    let (ch_x, ch_y, ch_cx, ch_cy) = box_emu(xfrm, "chOff", "chExt")?;
    if cx <= 0 || cy <= 0 || ch_cx <= 0 || ch_cy <= 0 {
        return Err(unsupported());
    }
    Ok(GroupMap {
        off_x,
        off_y,
        cx,
        cy,
        ch_x,
        ch_y,
        ch_cx,
        ch_cy,
        quarter: group_quarter(xfrm)?,
        flip_h: truthy(xfrm.attr("flipH")),
        flip_v: truthy(xfrm.attr("flipV")),
    })
}

/// 链从外到内。子矩形先落到里层，再逐层换到外层。
fn group_rects(
    group: &Element,
    chain: &[GroupMap],
    x_from: i32,
    y_from: i32,
    cols: &[i32],
    rows: &[i32],
    theme: Option<&LoadedTheme>,
) -> Result<Vec<PlainRect>, Error> {
    let mut rects = Vec::new();
    for child in group.element_children() {
        match child.local_name() {
            "nvGrpSpPr" | "grpSpPr" => {}
            "sp" => {
                let (x0, y0, x1, y1) = child_rect(child, x_from, y_from, chain)?;
                if crosses_hidden(x0, y0, x1, y1, cols, rows) {
                    return Err(unsupported());
                }
                let rect = plain_rect_at(child, theme, true, x0, y0, x1, y1)?;
                // 组合内描边未单独官方锁；不默画错。
                if rect.stroke.is_some() {
                    return Err(unsupported());
                }
                rects.push(rect);
            }
            "grpSp" => {
                // 里层转 90 度后，存下来的外框可以伸出父组合的子坐标。像素按换算结果取。
                let inner = read_group_map(child)?;
                let mut next = chain.to_vec();
                next.push(inner);
                rects.extend(group_rects(
                    child, &next, x_from, y_from, cols, rows, theme,
                )?);
            }
            _ => return Err(unsupported()),
        }
    }
    Ok(rects)
}

/// 子矩形先绕自己的中心转 90 度的倍数，得到外接矩形，再交给组去缩放和旋转。
fn local_box(xfrm: &Element) -> Result<(i64, i64, i64, i64), Error> {
    let (x, y, w, h) = box_emu(xfrm, "off", "ext")?;
    if w <= 0 || h <= 0 {
        return Err(unsupported());
    }
    let quarter = group_quarter(xfrm)?;
    if quarter == 0 {
        return Ok((x, y, w, h));
    }
    let cx2 = x
        .checked_mul(2)
        .ok_or_else(unsupported)?
        .checked_add(w)
        .ok_or_else(unsupported)?;
    let cy2 = y
        .checked_mul(2)
        .ok_or_else(unsupported)?
        .checked_add(h)
        .ok_or_else(unsupported)?;
    let x1 = x.checked_add(w).ok_or_else(unsupported)?;
    let y1 = y.checked_add(h).ok_or_else(unsupported)?;
    let mut min_x = i64::MAX;
    let mut min_y = i64::MAX;
    let mut max_x = i64::MIN;
    let mut max_y = i64::MIN;
    for (px, py) in [(x, y), (x1, y), (x1, y1), (x, y1)] {
        let (nx2, ny2) = spin_twice(cx2, cy2, px, py, quarter)?;
        min_x = min_x.min(nx2);
        min_y = min_y.min(ny2);
        max_x = max_x.max(nx2);
        max_y = max_y.max(ny2);
    }
    if min_x % 2 != 0 || min_y % 2 != 0 || max_x % 2 != 0 || max_y % 2 != 0 {
        return Err(unsupported());
    }
    let left = min_x / 2;
    let top = min_y / 2;
    Ok((left, top, max_x / 2 - left, max_y / 2 - top))
}

fn spin_twice(cx2: i64, cy2: i64, px: i64, py: i64, quarter: i64) -> Result<(i64, i64), Error> {
    let mut dx2 = px
        .checked_mul(2)
        .ok_or_else(unsupported)?
        .checked_sub(cx2)
        .ok_or_else(unsupported)?;
    let mut dy2 = py
        .checked_mul(2)
        .ok_or_else(unsupported)?
        .checked_sub(cy2)
        .ok_or_else(unsupported)?;
    match quarter {
        1 => {
            let next = dx2;
            dx2 = dy2.checked_neg().ok_or_else(unsupported)?;
            dy2 = next;
        }
        2 => {
            dx2 = dx2.checked_neg().ok_or_else(unsupported)?;
            dy2 = dy2.checked_neg().ok_or_else(unsupported)?;
        }
        3 => {
            let next = dx2;
            dx2 = dy2;
            dy2 = next.checked_neg().ok_or_else(unsupported)?;
        }
        _ => return Err(unsupported()),
    }
    Ok((
        cx2.checked_add(dx2).ok_or_else(unsupported)?,
        cy2.checked_add(dy2).ok_or_else(unsupported)?,
    ))
}

fn box_emu(xfrm: &Element, off_name: &str, ext_name: &str) -> Result<(i64, i64, i64, i64), Error> {
    let off = xfrm.child_by_local(off_name).ok_or_else(unsupported)?;
    let ext = xfrm.child_by_local(ext_name).ok_or_else(unsupported)?;
    Ok((
        emu_attr(off, "x")?,
        emu_attr(off, "y")?,
        emu_attr(ext, "cx")?,
        emu_attr(ext, "cy")?,
    ))
}

#[derive(Clone, Copy)]
struct GroupMap {
    off_x: i64,
    off_y: i64,
    cx: i64,
    cy: i64,
    ch_x: i64,
    ch_y: i64,
    ch_cx: i64,
    ch_cy: i64,
    /// 顺时针 90 度的次数，0 到 3。屏幕坐标向下为正。
    quarter: i64,
    flip_h: bool,
    flip_v: bool,
}

fn child_rect(
    sp: &Element,
    x_from: i32,
    y_from: i32,
    chain: &[GroupMap],
) -> Result<(i32, i32, i32, i32), Error> {
    let map = chain.last().ok_or_else(unsupported)?;
    let xfrm = sp
        .child_by_local("spPr")
        .and_then(|pr| pr.child_by_local("xfrm"))
        .ok_or_else(unsupported)?;
    // 纯色矩形绕自己的中心翻转后，外接框不变，也可以和直角叠在一起。
    let (x, y, w, h) = local_box(xfrm)?;
    let x1_emu = x.checked_add(w).ok_or_else(unsupported)?;
    let y1_emu = y.checked_add(h).ok_or_else(unsupported)?;
    let ch_x1 = map.ch_x.checked_add(map.ch_cx).ok_or_else(unsupported)?;
    let ch_y1 = map.ch_y.checked_add(map.ch_cy).ok_or_else(unsupported)?;
    if x < map.ch_x || y < map.ch_y || x1_emu > ch_x1 || y1_emu > ch_y1 {
        return Err(unsupported());
    }
    let mut corners = [(x, y), (x1_emu, y), (x1_emu, y1_emu), (x, y1_emu)];
    for (index, map) in chain.iter().enumerate().rev() {
        let mut next = [(0_i64, 0_i64); 4];
        for (i, (px, py)) in corners.into_iter().enumerate() {
            let sx = into_ext(px, map.ch_x, map.cx, map.ch_cx)?;
            let sy = into_ext(py, map.ch_y, map.cy, map.ch_cy)?;
            if index == 0 {
                next[i] = (sx, sy);
            } else {
                let (x2, y2) = place_twice(map, sx, sy)?;
                if x2 % 2 != 0 || y2 % 2 != 0 {
                    return Err(unsupported());
                }
                next[i] = (x2 / 2, y2 / 2);
            }
        }
        corners = next;
    }
    let (x0, y0, x1, y1) = turned_edges(&chain[0], x_from, y_from, corners)?;
    if x1 < x0 || y1 < y0 {
        return Err(unsupported());
    }
    Ok((x0, y0, x1, y1))
}

fn turned_box(map: &GroupMap, x_from: i32, y_from: i32) -> Result<(i32, i32, i32, i32), Error> {
    turned_edges(
        map,
        x_from,
        y_from,
        [(0, 0), (map.cx, 0), (map.cx, map.cy), (0, map.cy)],
    )
}

fn turned_edges(
    map: &GroupMap,
    x_from: i32,
    y_from: i32,
    corners: [(i64, i64); 4],
) -> Result<(i32, i32, i32, i32), Error> {
    let mut min_x = i64::MAX;
    let mut min_y = i64::MAX;
    let mut max_x = i64::MIN;
    let mut max_y = i64::MIN;
    for (sx, sy) in corners {
        let (x2, y2) = place_twice(map, sx, sy)?;
        min_x = min_x.min(x2);
        min_y = min_y.min(y2);
        max_x = max_x.max(x2);
        max_y = max_y.max(y2);
    }
    Ok((
        x_from + round_twice(min_x)?,
        y_from + round_twice(min_y)?,
        x_from + round_twice(max_x)?,
        y_from + round_twice(max_y)?,
    ))
}

/// 子坐标先按整数比例放进组的外框。除法去掉余数。
fn into_ext(value: i64, ch_origin: i64, span: i64, ch_span: i64) -> Result<i64, Error> {
    if span <= 0 || ch_span <= 0 {
        return Err(unsupported());
    }
    let delta = value.checked_sub(ch_origin).ok_or_else(unsupported)?;
    if delta < 0 {
        return Err(unsupported());
    }
    let prod = delta.checked_mul(span).ok_or_else(unsupported)?;
    Ok(prod / ch_span)
}

/// 先翻转，再绕外框中心顺时针转。返回的数是 EMU 的两倍，避免中心落在半个 EMU 上。
fn place_twice(map: &GroupMap, mut sx: i64, mut sy: i64) -> Result<(i64, i64), Error> {
    if map.flip_h {
        sx = map.cx.checked_sub(sx).ok_or_else(unsupported)?;
    }
    if map.flip_v {
        sy = map.cy.checked_sub(sy).ok_or_else(unsupported)?;
    }
    let mut dx2 = sx
        .checked_mul(2)
        .ok_or_else(unsupported)?
        .checked_sub(map.cx)
        .ok_or_else(unsupported)?;
    let mut dy2 = sy
        .checked_mul(2)
        .ok_or_else(unsupported)?
        .checked_sub(map.cy)
        .ok_or_else(unsupported)?;
    match map.quarter {
        0 => {}
        1 => {
            let next = dx2;
            dx2 = dy2.checked_neg().ok_or_else(unsupported)?;
            dy2 = next;
        }
        2 => {
            dx2 = dx2.checked_neg().ok_or_else(unsupported)?;
            dy2 = dy2.checked_neg().ok_or_else(unsupported)?;
        }
        3 => {
            let next = dx2;
            dx2 = dy2;
            dy2 = next.checked_neg().ok_or_else(unsupported)?;
        }
        _ => return Err(unsupported()),
    }
    let x2 = map
        .off_x
        .checked_mul(2)
        .ok_or_else(unsupported)?
        .checked_add(map.cx)
        .ok_or_else(unsupported)?
        .checked_add(dx2)
        .ok_or_else(unsupported)?;
    let y2 = map
        .off_y
        .checked_mul(2)
        .ok_or_else(unsupported)?
        .checked_add(map.cy)
        .ok_or_else(unsupported)?
        .checked_add(dy2)
        .ok_or_else(unsupported)?;
    Ok((x2, y2))
}

fn group_quarter(xfrm: &Element) -> Result<i64, Error> {
    let Some(raw) = xfrm.attr("rot") else {
        return Ok(0);
    };
    let n = raw.parse::<i64>().map_err(|_| unsupported())?;
    if n.rem_euclid(5_400_000) != 0 {
        return Err(unsupported());
    }
    Ok(n.rem_euclid(21_600_000) / 5_400_000)
}

/// `num` 是 EMU 的两倍。
fn round_twice(num: i64) -> Result<i32, Error> {
    if num < 0 {
        return Err(unsupported());
    }
    let px = (num as f64 / 19_050.0).round();
    if px > i32::MAX as f64 {
        return Err(unsupported());
    }
    Ok(px as i32)
}

fn shape_fill(fill: &Element, theme: Option<&LoadedTheme>) -> Result<[u8; 3], Error> {
    let mut children = fill.element_children();
    let clr = children.next().ok_or_else(unsupported)?;
    if children.next().is_some() {
        return Err(unsupported());
    }
    match clr.local_name() {
        "srgbClr" => srgb_bytes(clr),
        "schemeClr" => scheme_rgb(clr, theme),
        _ => Err(unsupported()),
    }
}

fn scheme_rgb(clr: &Element, theme: Option<&LoadedTheme>) -> Result<[u8; 3], Error> {
    if clr.element_children().next().is_some() {
        return Err(unsupported());
    }
    let theme = theme.ok_or_else(unsupported)?;
    let name = clr.attr("val").ok_or_else(unsupported)?;
    let index = THEME_NAMES
        .iter()
        .position(|slot| *slot == name)
        .ok_or_else(unsupported)?;
    if !theme.present[index] {
        return Err(unsupported());
    }
    Ok(theme.slots[index])
}

fn srgb_bytes(clr: &Element) -> Result<[u8; 3], Error> {
    for child in clr.element_children() {
        if child.local_name() != "alpha" || child.attr("val") != Some("100000") {
            return Err(unsupported());
        }
    }
    let hex = clr.attr("val").ok_or_else(unsupported)?;
    if hex.len() != 6 || !hex.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return Err(unsupported());
    }
    let mut color = [0u8; 3];
    for (i, byte) in color.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).map_err(|_| unsupported())?;
    }
    Ok(color)
}

/// 直角倍数不改变纯色矩形的外接框。组合里的子矩形不走这条。
fn quarter_turn(raw: &str, allow: bool) -> bool {
    if !allow {
        return raw == "0";
    }
    raw.parse::<i64>()
        .is_ok_and(|n| n.rem_euclid(5_400_000) == 0)
}

/// `noFill` 时虚线、斜接和隐藏线都不画。旁边再有填充才算描边。
fn clear_line(ln: &Element) -> bool {
    ln.child_by_local("noFill").is_some()
        && ln.element_children().all(|child| {
            !matches!(
                child.local_name(),
                "solidFill" | "gradFill" | "pattFill" | "blipFill"
            )
        })
}

fn visible_text(el: &Element) -> bool {
    if el.local_name() == "t" && !el.text_content().trim().is_empty() {
        return true;
    }
    el.element_children().any(visible_text)
}

fn fill_plain_rgb(canvas: &mut Canvas, x0: i32, y0: i32, x1: i32, y1: i32, color: [u8; 3]) {
    for y in y0..y1 {
        // 外框盖住第一行和第一列，边框留在外框上。
        if y <= 0 {
            continue;
        }
        for x in x0..x1 {
            if x <= 0 {
                continue;
            }
            canvas.put(x, y, color);
        }
    }
}

fn paint_plain_rects(canvas: &mut Canvas, shapes: &[Overlay]) {
    for shape in shapes {
        match shape {
            Overlay::Fill(shape) => {
                let w = shape.x1 - shape.x0;
                let h = shape.y1 - shape.y0;
                if w <= 0 || h <= 0 {
                    continue;
                }
                if let Some(ref line) = shape.stroke {
                    let ok = fill_device_stroked_rect(
                        shape.x0 - canvas.ox,
                        shape.y0 - canvas.oy,
                        w,
                        h,
                        shape.fill,
                        line.color,
                        line.px,
                        &mut canvas.px,
                        canvas.w as u32,
                        canvas.h as u32,
                    );
                    if !ok {
                        // GDI+ 不可用时不静默空心；上层已约定 Windows。留白会导致假绿，改由调用方失败更安全——此处无法返回 Err。
                        // 回退：至少铺不透明填充，避免整页空洞；描边缺失会在 oracle 暴露。
                        if let Some(fill) = shape.fill {
                            fill_plain_rgb(canvas, shape.x0, shape.y0, shape.x1, shape.y1, fill);
                        }
                    }
                } else if let Some(fill) = shape.fill {
                    fill_plain_rgb(canvas, shape.x0, shape.y0, shape.x1, shape.y1, fill);
                }
            }
            Overlay::Png(shape) => {
                for row in 0..shape.h {
                    let y = shape.y0 + row as i32;
                    if y <= 0 {
                        continue;
                    }
                    for col in 0..shape.w {
                        let x = shape.x0 + col as i32;
                        if x <= 0 {
                            continue;
                        }
                        let i = (row * shape.w + col) as usize * 3;
                        canvas.put(x, y, [shape.rgb[i], shape.rgb[i + 1], shape.rgb[i + 2]]);
                    }
                }
            }
        }
    }
}

/// 目标框正好等于要贴的像素时，按原图贴上。整像素裁剪且不缩放时贴这一块。
fn sheet_png(
    pkg: &Package,
    drawing_part: &str,
    pic: &Element,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
) -> Result<SheetPng, Error> {
    accept_sheet_pic(pic)?;
    let Some(w) = (x1 - x0).try_into().ok().filter(|w: &u32| *w > 0) else {
        return Err(unsupported());
    };
    let Some(h) = (y1 - y0).try_into().ok().filter(|h: &u32| *h > 0) else {
        return Err(unsupported());
    };
    let rid = pic
        .child_by_local("blipFill")
        .and_then(|fill| fill.child_by_local("blip"))
        .and_then(|blip| blip.attr_local("embed"))
        .filter(|rid| !rid.is_empty())
        .ok_or_else(unsupported)?;
    let media = crate::opc::resolve_id(pkg, drawing_part, rid).map_err(|_| unsupported())?;
    let bytes = pkg.get(&media).map_err(|_| unsupported())?;
    let (image_w, image_h, rgb) = decode_opaque_png(bytes)?;
    let rgb = match source_window(pic, image_w, image_h, w, h)? {
        Some((left, top, _, _)) => crop_rgb(&rgb, image_w, left, top, w, h),
        None => {
            if image_w != w || image_h != h {
                return Err(unsupported());
            }
            rgb
        }
    };
    Ok(SheetPng { x0, y0, w, h, rgb })
}

/// 裁剪四边落到整像素，且裁完尺寸等于目标时，返回原图上的这块。没有裁剪时返回 `None`。
fn source_window(
    pic: &Element,
    image_w: u32,
    image_h: u32,
    dest_w: u32,
    dest_h: u32,
) -> Result<Option<(u32, u32, u32, u32)>, Error> {
    let Some(rect) = pic
        .child_by_local("blipFill")
        .and_then(|fill| fill.child_by_local("srcRect"))
    else {
        return Ok(None);
    };
    let (l, t, r, b) = (
        percent_attr(rect, "l")?,
        percent_attr(rect, "t")?,
        percent_attr(rect, "r")?,
        percent_attr(rect, "b")?,
    );
    if l == 0 && t == 0 && r == 0 && b == 0 {
        return Ok(None);
    }
    let left = percent_px(l, image_w).ok_or_else(unsupported)?;
    let top = percent_px(t, image_h).ok_or_else(unsupported)?;
    let right = percent_px(r, image_w).ok_or_else(unsupported)?;
    let bottom = percent_px(b, image_h).ok_or_else(unsupported)?;
    let Some(src_w) = image_w.checked_sub(left).and_then(|w| w.checked_sub(right)) else {
        return Err(unsupported());
    };
    let Some(src_h) = image_h.checked_sub(top).and_then(|h| h.checked_sub(bottom)) else {
        return Err(unsupported());
    };
    if src_w == 0 || src_h == 0 || src_w != dest_w || src_h != dest_h {
        return Err(unsupported());
    }
    Ok(Some((left, top, src_w, src_h)))
}

fn percent_attr(el: &Element, name: &str) -> Result<i64, Error> {
    match el.attr(name) {
        None => Ok(0),
        Some(raw) => raw.parse().map_err(|_| unsupported()),
    }
}

/// 百分比换成像素。离整像素超过半个百分比量化步长时不认。
fn percent_px(pct: i64, extent: u32) -> Option<u32> {
    if pct < 0 {
        return None;
    }
    let prod = i128::from(pct) * i128::from(extent);
    let n = (prod + 50_000) / 100_000;
    let err = (prod - n * 100_000).abs();
    if err * 2 > i128::from(extent) || n > i128::from(extent) {
        return None;
    }
    Some(n as u32)
}

fn crop_rgb(rgb: &[u8], image_w: u32, left: u32, top: u32, w: u32, h: u32) -> Vec<u8> {
    let mut out = Vec::with_capacity((w * h * 3) as usize);
    for row in 0..h {
        let start = (((top + row) * image_w + left) * 3) as usize;
        let end = start + (w as usize) * 3;
        out.extend_from_slice(&rgb[start..end]);
    }
    out
}

fn accept_sheet_pic(pic: &Element) -> Result<(), Error> {
    if let Some(sp_pr) = pic.child_by_local("spPr") {
        for child in sp_pr.element_children() {
            match child.local_name() {
                "xfrm" | "prstGeom" | "ln" => {}
                _ => return Err(unsupported()),
            }
        }
        if let Some(xfrm) = sp_pr.child_by_local("xfrm") {
            if xfrm.attr("rot").is_some_and(|rot| rot != "0")
                || truthy(xfrm.attr("flipH"))
                || truthy(xfrm.attr("flipV"))
            {
                return Err(unsupported());
            }
        }
        if let Some(geom) = sp_pr.child_by_local("prstGeom") {
            if geom.attr("prst") != Some("rect") {
                return Err(unsupported());
            }
        }
        if let Some(ln) = sp_pr.child_by_local("ln") {
            if ln.child_by_local("noFill").is_none()
                || ln
                    .element_children()
                    .any(|child| child.local_name() != "noFill")
            {
                return Err(unsupported());
            }
        }
    }
    let fill = pic.child_by_local("blipFill").ok_or_else(unsupported)?;
    for child in fill.element_children() {
        match child.local_name() {
            "blip" | "stretch" | "srcRect" => {}
            _ => return Err(unsupported()),
        }
    }
    if let Some(rect) = fill
        .child_by_local("stretch")
        .and_then(|stretch| stretch.child_by_local("fillRect"))
    {
        for name in ["l", "t", "r", "b"] {
            if rect.attr(name).is_some_and(|raw| raw != "0") {
                return Err(unsupported());
            }
        }
    }
    let blip = fill.child_by_local("blip").ok_or_else(unsupported)?;
    if blip
        .element_children()
        .any(|child| child.local_name() != "extLst")
    {
        return Err(unsupported());
    }
    Ok(())
}

fn decode_opaque_png(bytes: &[u8]) -> Result<(u32, u32, Vec<u8>), Error> {
    if !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Err(unsupported());
    }
    let decoder = png::Decoder::new(Cursor::new(bytes));
    let mut reader = decoder.read_info().map_err(|_| unsupported())?;
    if reader.info().bit_depth != png::BitDepth::Eight {
        return Err(unsupported());
    }
    let mut buf = vec![0; reader.output_buffer_size()];
    let frame = reader.next_frame(&mut buf).map_err(|_| unsupported())?;
    let raw = &buf[..frame.buffer_size()];
    let rgb = match frame.color_type {
        png::ColorType::Rgb => raw.to_vec(),
        png::ColorType::Rgba => {
            let mut out = Vec::with_capacity(raw.len() / 4 * 3);
            for chunk in raw.as_chunks::<4>().0 {
                if chunk[3] != 255 {
                    return Err(unsupported());
                }
                out.extend_from_slice(&chunk[..3]);
            }
            out
        }
        _ => return Err(unsupported()),
    };
    if rgb.len() != (frame.width as usize) * (frame.height as usize) * 3 {
        return Err(unsupported());
    }
    Ok((frame.width, frame.height, rgb))
}

fn crosses_hidden(x0: i32, y0: i32, x1: i32, y1: i32, cols: &[i32], rows: &[i32]) -> bool {
    let mut at = 0;
    for size in cols {
        if *size == 0 && x0 <= at && x1 > at {
            return true;
        }
        at += size;
    }
    at = 0;
    for size in rows {
        if *size == 0 && y0 <= at && y1 > at {
            return true;
        }
        at += size;
    }
    false
}

fn overlaps(a: u32, b: u32, lo: u32, hi: u32) -> bool {
    let a0 = a.min(b);
    let a1 = a.max(b);
    a0 <= hi && lo <= a1
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HAlign {
    General,
    Left,
    Center,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum VAlign {
    Bottom,
    Center,
    Top,
}

#[derive(Clone, Debug)]
enum NumFmt {
    General,
    IntComma,
    Fixed(u32),
    Percent(u32),
    /// 段数是 2、3 或 4，每段都空。4 段时文字和布尔也不画。
    Blank(u8),
    /// 货币/多段（无 `[颜色]`/`[条件]`）。按正/负/零选段；字面量段；`_` 跳宽按空格进显示串。
    Sections(Vec<NumSection>),
    /// 内置 14–22 与点名自定义 `yyyy/mm/dd`。方括号/其它自定义日期码仍 NI。
    DateTime(DateTimePat),
}

/// 本切片锁定的日期时间图案（英文月缩写；时间段里的 `m` 为分钟）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DateTimePat {
    MdYy,
    DMmmYy,
    DMmm,
    MmmYy,
    HmmAmpm,
    HmmssAmpm,
    Hmm,
    Hmmss,
    MdYyHmm,
    YyyyMmDd,
}

/// 一段数字格式（公开 OOXML 分号分段 + Excel 选段规则）。
#[derive(Clone, Debug, PartialEq, Eq)]
struct NumSection {
    /// 仅字面量（如 `"-"`），忽略数值本身。
    literal: Option<String>,
    prefix: String,
    suffix: String,
    digits: u32,
    thousands: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PatternKind {
    Gray125,
    Gray0625,
    LightHorizontal,
    DarkHorizontal,
    LightVertical,
    DarkVertical,
    LightGrid,
    DarkGrid,
    MediumGray,
    LightGray,
    DarkGray,
    DarkDown,
    LightDown,
    DarkUp,
    LightUp,
    DarkTrellis,
    LightTrellis,
}

#[derive(Clone, Copy, Debug)]
enum Fill {
    None,
    Solid([u8; 3]),
    Pattern(PatternKind, [u8; 3], [u8; 3]),
    /// SpreadsheetML `gradientFill`：不透明两停点线性。与 DrawingML/PPT `gamma_mix` 无关。
    Linear {
        degree: f64,
        start: [u8; 3],
        end: [u8; 3],
    },
    Nope,
}

struct FontInfo {
    name: String,
    pt: f64,
    bold: bool,
    italic: bool,
    color: Option<[u8; 3]>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LineKind {
    Solid,
    Double,
    Dashed,
    Dotted,
    Hair,
    DashDot,
    DashDotDot,
    MediumDashed,
    MediumDashDot,
    MediumDashDotDot,
    SlantDashDot,
}

#[derive(Clone, Copy, Debug)]
struct Edge {
    kind: LineKind,
    px: i32,
    color: [u8; 3],
}

#[derive(Clone, Copy, Debug, Default)]
struct Edges {
    left: Option<Edge>,
    right: Option<Edge>,
    top: Option<Edge>,
    bottom: Option<Edge>,
    /// `border/@diagonalDown`：左上→右下。
    diagonal_down: Option<Edge>,
    /// `border/@diagonalUp`：左下→右上。
    diagonal_up: Option<Edge>,
}

struct Xf {
    font: usize,
    fill: usize,
    edges: Edges,
    num_fmt: u32,
    h_align: HAlign,
    v_align: VAlign,
    /// `alignment/@wrapText`。
    wrap: bool,
    /// `alignment/@indent`，0–15；每级约 3 个 `0` 字宽（与绘制同一套 `Face::width`）。
    indent: u32,
    /// `alignment/@readingOrder`：0 上下文 / 1 LTR / 2 RTL。2 与强 RTL 文种出图 NI。
    reading_order: u8,
}

struct Dxf {
    fill: Option<[u8; 3]>,
    color: Option<[u8; 3]>,
    edges: Edges,
}

struct Styles {
    fonts: Vec<FontInfo>,
    normal_font: usize,
    fills: Vec<Fill>,
    xfs: Vec<Xf>,
    formats: HashMap<u32, String>,
    dxfs: Vec<Dxf>,
}

fn load_styles(pkg: &Package, wb_part: &str, rels: &Relationships) -> Result<Styles, Error> {
    let Some(rel) = rels
        .rels
        .iter()
        .find(|rel| rel.rel_type.ends_with("/styles") && !rel.is_external())
    else {
        return Ok(Styles::normal());
    };
    let part = crate::opc::resolve_rel_target(wb_part, &rel.target)?;
    let bytes = pkg.get(&part).map_err(|_| unsupported())?;
    let doc = crate::xml::parse(bytes)?;
    let palette = Palette {
        theme: theme_slots(pkg, wb_part, rels),
        indexed: indexed_palette(&doc.root),
    };
    Styles::parse(&doc.root, &palette)
}

/// 工作表 `color/@theme` 的下标。0 是 `lt1`，1 是 `dk1`。默认正文字色写的就是 1。
const THEME_NAMES: [&str; 12] = [
    "lt1", "dk1", "lt2", "dk2", "accent1", "accent2", "accent3", "accent4", "accent5", "accent6",
    "hlink", "folHlink",
];

struct LoadedTheme {
    slots: [[u8; 3]; 12],
    present: [bool; 12],
}

/// 主题部件里实际写下的档。没有部件时返回 `None`，形状上的 `schemeClr` 整表不做。
fn loaded_theme(pkg: &Package, wb_part: &str, rels: &Relationships) -> Option<LoadedTheme> {
    let rel = rels
        .rels
        .iter()
        .find(|rel| rel.rel_type.ends_with("/theme") && !rel.is_external())?;
    let part = crate::opc::resolve_rel_target(wb_part, &rel.target).ok()?;
    let bytes = pkg.get(&part).ok()?;
    let doc = crate::xml::parse(bytes).ok()?;
    let scheme = find_local(&doc.root, "clrScheme")?;
    let mut theme = LoadedTheme {
        slots: default_theme(),
        present: [false; 12],
    };
    for (i, name) in THEME_NAMES.iter().enumerate() {
        if let Some(rgb) = scheme.child_by_local(name).and_then(slot_rgb) {
            theme.slots[i] = rgb;
            theme.present[i] = true;
        }
    }
    Some(theme)
}

struct Palette {
    theme: [[u8; 3]; 12],
    indexed: Vec<[u8; 3]>,
}

fn theme_slots(pkg: &Package, wb_part: &str, rels: &Relationships) -> [[u8; 3]; 12] {
    let mut slots = default_theme();
    let Some(rel) = rels
        .rels
        .iter()
        .find(|rel| rel.rel_type.ends_with("/theme") && !rel.is_external())
    else {
        return slots;
    };
    let Ok(part) = crate::opc::resolve_rel_target(wb_part, &rel.target) else {
        return slots;
    };
    let Ok(bytes) = pkg.get(&part) else {
        return slots;
    };
    let Ok(doc) = crate::xml::parse(bytes) else {
        return slots;
    };
    let Some(scheme) = find_local(&doc.root, "clrScheme") else {
        return slots;
    };
    for (i, name) in THEME_NAMES.iter().enumerate() {
        if let Some(rgb) = scheme.child_by_local(name).and_then(slot_rgb) {
            slots[i] = rgb;
        }
    }
    slots
}

fn default_theme() -> [[u8; 3]; 12] {
    [
        [255, 255, 255],
        [0, 0, 0],
        [0xEE, 0xEC, 0xE1],
        [0x1F, 0x49, 0x7D],
        [0x4F, 0x81, 0xBD],
        [0xC0, 0x50, 0x4D],
        [0x9B, 0xBB, 0x59],
        [0x80, 0x64, 0xA2],
        [0x4B, 0xAC, 0xC6],
        [0xF7, 0x96, 0x46],
        [0x00, 0x00, 0xFF],
        [0x80, 0x00, 0x80],
    ]
}

fn find_local<'a>(el: &'a Element, name: &str) -> Option<&'a Element> {
    if el.local_name() == name {
        return Some(el);
    }
    for child in el.element_children() {
        if let Some(found) = find_local(child, name) {
            return Some(found);
        }
    }
    None
}

fn slot_rgb(slot: &Element) -> Option<[u8; 3]> {
    if let Some(el) = slot.child_by_local("srgbClr") {
        return el.attr("val").and_then(parse_rgb);
    }
    let el = slot.child_by_local("sysClr")?;
    el.attr("lastClr")
        .and_then(parse_rgb)
        .or_else(|| match el.attr("val") {
            Some("window") => Some([255, 255, 255]),
            Some("windowText") => Some([0, 0, 0]),
            _ => None,
        })
}

fn indexed_palette(root: &Element) -> Vec<[u8; 3]> {
    let mut colors: Vec<[u8; 3]> = DEFAULT_INDEXED.iter().copied().map(rgb_of).collect();
    let Some(list) = root
        .child_by_local("colors")
        .and_then(|el| el.child_by_local("indexedColors"))
    else {
        return colors;
    };
    for (i, el) in list.children_by_local("rgbColor").into_iter().enumerate() {
        let Some(rgb) = el.attr("rgb").and_then(parse_rgb) else {
            continue;
        };
        if i < colors.len() {
            colors[i] = rgb;
        } else {
            colors.push(rgb);
        }
    }
    colors
}

fn rgb_of(n: u32) -> [u8; 3] {
    [
        ((n >> 16) & 0xff) as u8,
        ((n >> 8) & 0xff) as u8,
        (n & 0xff) as u8,
    ]
}

/// ECMA-376 默认索引色，从 0 起。64 是窗口文字，65 是窗口底。
const DEFAULT_INDEXED: [u32; 64] = [
    0x000000, 0xFFFFFF, 0xFF0000, 0x00FF00, 0x0000FF, 0xFFFF00, 0xFF00FF, 0x00FFFF, 0x000000,
    0xFFFFFF, 0xFF0000, 0x00FF00, 0x0000FF, 0xFFFF00, 0xFF00FF, 0x00FFFF, 0x800000, 0x008000,
    0x000080, 0x808000, 0x800080, 0x008080, 0xC0C0C0, 0x808080, 0x9999FF, 0x993366, 0xFFFFCC,
    0xCCFFFF, 0x660066, 0xFF8080, 0x0066CC, 0xCCCCFF, 0x000080, 0xFF00FF, 0xFFFF00, 0x00FFFF,
    0x800080, 0x800000, 0x008080, 0x0000FF, 0x00CCFF, 0xCCFFFF, 0xCCFFCC, 0xFFFF99, 0x99CCFF,
    0xFF99CC, 0xCC99FF, 0xFFCC99, 0x3366FF, 0x33CCCC, 0x99CC00, 0xFFCC00, 0xFF9900, 0xFF6600,
    0x666699, 0x969696, 0x003366, 0x339966, 0x003300, 0x333300, 0x993300, 0x993366, 0x333399,
    0x333333,
];

impl Palette {
    fn resolve(&self, el: &Element) -> Option<[u8; 3]> {
        let base = if let Some(rgb) = el.attr("rgb").and_then(parse_rgb) {
            rgb
        } else if let Some(index) = el.attr("theme").and_then(|s| s.parse::<usize>().ok()) {
            *self.theme.get(index)?
        } else if let Some(index) = el.attr("indexed").and_then(|s| s.parse::<usize>().ok()) {
            self.indexed_at(index)?
        } else if truthy(el.attr("auto")) {
            [0, 0, 0]
        } else {
            return None;
        };
        let Some(tint) = el.attr("tint").and_then(|s| s.parse::<f64>().ok()) else {
            return Some(base);
        };
        Some(apply_sheet_tint(base, tint))
    }

    fn indexed_at(&self, index: usize) -> Option<[u8; 3]> {
        if let Some(rgb) = self.indexed.get(index) {
            return Some(*rgb);
        }
        match index {
            64 => Some([0, 0, 0]),
            65 => Some([255, 255, 255]),
            _ => None,
        }
    }
}

const EXCEL_HLS_MAX: i32 = 240;
const EXCEL_RGB_MAX: i32 = 255;

/// Excel 把 `tint` 打在 240 级 HLS 的亮度上。亮度先向 0 截成整数，再转回 RGB。
/// 离整百分比不超过 `1/32767` 的值按那个百分比算，这是 Excel 存进样式的那一档。
fn apply_sheet_tint(rgb: [u8; 3], tint: f64) -> [u8; 3] {
    if tint.abs() < 1e-9 {
        return rgb;
    }
    let tint = normalize_excel_tint(tint);
    let (hue, lum, sat) = rgb_to_excel_hls(rgb);
    let moved = if tint < 0.0 {
        f64::from(lum) * (1.0 + tint)
    } else {
        f64::from(lum) * (1.0 - tint) + f64::from(EXCEL_HLS_MAX) * tint
    };
    let lum = (moved as i32).clamp(0, EXCEL_HLS_MAX);
    excel_hls_to_rgb(hue, lum, sat)
}

fn normalize_excel_tint(tint: f64) -> f64 {
    let whole = (tint * 100.0).round() / 100.0;
    if (tint - whole).abs() <= 1.0 / 32_767.0 {
        whole
    } else {
        tint
    }
}

fn rgb_to_excel_hls(rgb: [u8; 3]) -> (i32, i32, i32) {
    let (red, green, blue) = (i32::from(rgb[0]), i32::from(rgb[1]), i32::from(rgb[2]));
    let brightest = red.max(green).max(blue);
    let darkest = red.min(green).min(blue);
    let sum = brightest + darkest;
    let span = brightest - darkest;
    let lum = (sum * EXCEL_HLS_MAX + EXCEL_RGB_MAX) / (2 * EXCEL_RGB_MAX);
    if span == 0 {
        return (0, lum, 0);
    }
    let sat = if lum <= EXCEL_HLS_MAX / 2 {
        (span * EXCEL_HLS_MAX + sum / 2) / sum
    } else {
        (span * EXCEL_HLS_MAX + (2 * EXCEL_RGB_MAX - sum) / 2) / (2 * EXCEL_RGB_MAX - sum)
    };
    let distance = |channel: i32| ((brightest - channel) * (EXCEL_HLS_MAX / 6) + span / 2) / span;
    let hue = if red == brightest {
        distance(blue) - distance(green)
    } else if green == brightest {
        EXCEL_HLS_MAX / 3 + distance(red) - distance(blue)
    } else {
        2 * EXCEL_HLS_MAX / 3 + distance(green) - distance(red)
    };
    (hue.rem_euclid(EXCEL_HLS_MAX), lum, sat)
}

fn excel_hls_to_rgb(hue: i32, lum: i32, sat: i32) -> [u8; 3] {
    let upper = if lum <= EXCEL_HLS_MAX / 2 {
        (lum * (EXCEL_HLS_MAX + sat) + EXCEL_HLS_MAX / 2) / EXCEL_HLS_MAX
    } else {
        lum + sat - (lum * sat + EXCEL_HLS_MAX / 2) / EXCEL_HLS_MAX
    };
    let lower = 2 * lum - upper;
    let channel = |offset: i32| {
        let level = excel_hue_level(lower, upper, hue + offset);
        ((level * EXCEL_RGB_MAX + EXCEL_HLS_MAX / 2) / EXCEL_HLS_MAX).clamp(0, EXCEL_RGB_MAX) as u8
    };
    [
        channel(EXCEL_HLS_MAX / 3),
        channel(0),
        channel(-EXCEL_HLS_MAX / 3),
    ]
}

fn excel_hue_level(lower: i32, upper: i32, hue: i32) -> i32 {
    let hue = hue.rem_euclid(EXCEL_HLS_MAX);
    let sixth = EXCEL_HLS_MAX / 6;
    if hue < sixth {
        lower + ((upper - lower) * hue + EXCEL_HLS_MAX / 12) / sixth
    } else if hue < EXCEL_HLS_MAX / 2 {
        upper
    } else if hue < 2 * EXCEL_HLS_MAX / 3 {
        lower + ((upper - lower) * (2 * EXCEL_HLS_MAX / 3 - hue) + EXCEL_HLS_MAX / 12) / sixth
    } else {
        lower
    }
}

impl Styles {
    fn normal() -> Self {
        Self {
            fonts: vec![FontInfo {
                name: "Calibri".into(),
                pt: 11.0,
                bold: false,
                italic: false,
                color: Some([0, 0, 0]),
            }],
            normal_font: 0,
            fills: vec![Fill::None, Fill::Nope],
            xfs: vec![Xf {
                font: 0,
                fill: 0,
                edges: Edges::default(),
                num_fmt: 0,
                h_align: HAlign::General,
                v_align: VAlign::Bottom,
                wrap: false,
                indent: 0,
                reading_order: 0,
            }],
            formats: HashMap::new(),
            dxfs: Vec::new(),
        }
    }

    fn parse(root: &Element, palette: &Palette) -> Result<Self, Error> {
        let mut formats = HashMap::new();
        if let Some(list) = root.child_by_local("numFmts") {
            for item in list.children_by_local("numFmt") {
                let Some(id) = item.attr("numFmtId").and_then(|s| s.parse().ok()) else {
                    continue;
                };
                if let Some(code) = item.attr("formatCode") {
                    formats.insert(id, code.to_string());
                }
            }
        }
        let mut fonts = Vec::new();
        if let Some(list) = root.child_by_local("fonts") {
            for font in list.children_by_local("font") {
                fonts.push(read_font(font, palette));
            }
        }
        if fonts.is_empty() {
            fonts.push(FontInfo {
                name: "Calibri".into(),
                pt: 11.0,
                bold: false,
                italic: false,
                color: Some([0, 0, 0]),
            });
        }
        let normal_font = normal_font_index(root).min(fonts.len() - 1);
        let mut fills = Vec::new();
        if let Some(list) = root.child_by_local("fills") {
            for fill in list.children_by_local("fill") {
                fills.push(read_fill(fill, palette));
            }
        }
        let mut borders = Vec::new();
        if let Some(list) = root.child_by_local("borders") {
            for border in list.children_by_local("border") {
                borders.push(parse_edges(border, palette)?);
            }
        }
        let mut xfs = Vec::new();
        if let Some(list) = root.child_by_local("cellXfs") {
            for xf in list.children_by_local("xf") {
                xfs.push(read_xf(xf, &borders)?);
            }
        }
        if xfs.is_empty() {
            xfs.push(Xf {
                font: 0,
                fill: 0,
                edges: Edges::default(),
                num_fmt: 0,
                h_align: HAlign::General,
                v_align: VAlign::Bottom,
                wrap: false,
                indent: 0,
                reading_order: 0,
            });
        }
        let mut dxfs = Vec::new();
        if let Some(list) = root.child_by_local("dxfs") {
            for dxf in list.children_by_local("dxf") {
                dxfs.push(read_dxf(dxf, palette)?);
            }
        }
        Ok(Self {
            fonts,
            normal_font,
            fills,
            xfs,
            formats,
            dxfs,
        })
    }

    fn xf(&self, index: usize) -> Result<&Xf, Error> {
        self.xfs.get(index).ok_or_else(unsupported)
    }

    fn font(&self, index: usize) -> Result<&FontInfo, Error> {
        let xf = self.xf(index)?;
        self.fonts.get(xf.font).ok_or_else(unsupported)
    }

    fn normal_font(&self) -> &FontInfo {
        &self.fonts[self.normal_font]
    }

    fn normal_row_pt(&self) -> f64 {
        let font = self.normal_font();
        auto_row_pt(&font.name, font.pt)
    }

    fn fmt_of(&self, index: usize) -> Result<NumFmt, Error> {
        let id = self.xf(index)?.num_fmt;
        if let Some(fmt) = builtin_fmt(id) {
            return Ok(fmt);
        }
        let Some(code) = self.formats.get(&id) else {
            return Err(unsupported());
        };
        parse_format(code)
    }
}

fn read_font(font: &Element, palette: &Palette) -> FontInfo {
    let name = font
        .child_by_local("name")
        .and_then(|el| el.attr("val"))
        .unwrap_or("Calibri")
        .to_string();
    let pt = font
        .child_by_local("sz")
        .and_then(|el| el.attr("val"))
        .and_then(|s| s.parse().ok())
        .unwrap_or(11.0);
    let color = match font.child_by_local("color") {
        None => Some([0, 0, 0]),
        Some(el) => palette.resolve(el),
    };
    FontInfo {
        name,
        pt,
        bold: child_on(font, "b"),
        italic: child_on(font, "i"),
        color,
    }
}

fn child_on(el: &Element, name: &str) -> bool {
    match el.child_by_local(name) {
        None => false,
        Some(child) => {
            !matches!(child.attr("val"), Some(v) if v == "0" || v.eq_ignore_ascii_case("false"))
        }
    }
}

fn read_fill(fill: &Element, palette: &Palette) -> Fill {
    if let Some(grad) = fill.child_by_local("gradientFill") {
        return read_linear_fill(grad);
    }
    let Some(pat) = fill.child_by_local("patternFill") else {
        return Fill::None;
    };
    match pat.attr("patternType").unwrap_or("none") {
        "none" => Fill::None,
        "solid" => pat
            .child_by_local("fgColor")
            .and_then(|el| palette.resolve(el))
            .map(Fill::Solid)
            .unwrap_or(Fill::Nope),
        other => match pattern_kind(other) {
            Some(kind) => {
                let Some(fg) = pattern_channel(pat, "fgColor", [0, 0, 0], palette) else {
                    return Fill::Nope;
                };
                let Some(bg) = pattern_channel(pat, "bgColor", [255, 255, 255], palette) else {
                    return Fill::Nope;
                };
                Fill::Pattern(kind, fg, bg)
            }
            None => Fill::Nope,
        },
    }
}

/// SpreadsheetML `CT_GradientFill`：`type` 缺省/`linear`，两停点不透明 `rgb`（A=FF）。
/// `degree` 缺省 0（左→右）；90 为上→下。插值是通道直线（与 colorScale 同 `mix`），
/// **不是** PPT/DrawingML 的 `gamma_mix` / S 曲线。`path`、theme、indexed、alpha、多于两停 → Nope。
fn read_linear_fill(grad: &Element) -> Fill {
    let kind = grad.attr("type").unwrap_or("linear");
    if !kind.eq_ignore_ascii_case("linear") {
        return Fill::Nope;
    }
    let degree = match grad.attr("degree") {
        Some(s) => match s.parse::<f64>() {
            Ok(v) if v.is_finite() => v,
            _ => return Fill::Nope,
        },
        None => 0.0,
    };
    let stops = grad.children_by_local("stop");
    if stops.len() != 2 {
        return Fill::Nope;
    }
    let mut parsed = Vec::with_capacity(2);
    for stop in stops {
        let pos = match stop.attr("position").and_then(|s| s.parse::<f64>().ok()) {
            Some(v) if v.is_finite() => v,
            _ => return Fill::Nope,
        };
        let Some(color) = stop.child_by_local("color") else {
            return Fill::Nope;
        };
        // theme / indexed / auto / tint：本刀 NI。只要不透明 rgb。
        if color.attr("theme").is_some()
            || color.attr("indexed").is_some()
            || color.attr("tint").is_some()
            || truthy(color.attr("auto"))
        {
            return Fill::Nope;
        }
        let Some(hex) = color.attr("rgb") else {
            return Fill::Nope;
        };
        let hex = hex.trim();
        if hex.len() == 8 && !hex[..2].eq_ignore_ascii_case("ff") {
            return Fill::Nope;
        }
        let Some(rgb) = parse_rgb(hex) else {
            return Fill::Nope;
        };
        parsed.push((pos, rgb));
    }
    parsed.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    Fill::Linear {
        degree,
        start: parsed[0].1,
        end: parsed[1].1,
    }
}

/// 差分填充没写 `patternType`、只有 `bgColor` 时，Excel 把它画成纯色。
fn dxf_fill_color(fill: &Element, palette: &Palette) -> Result<Option<[u8; 3]>, Error> {
    if fill.child_by_local("gradientFill").is_some() {
        return Err(unsupported());
    }
    let Some(pat) = fill.child_by_local("patternFill") else {
        return Ok(None);
    };
    if pat.attr("patternType").is_none() {
        if pat.child_by_local("fgColor").is_some() {
            return Err(unsupported());
        }
        let Some(color) = pat.child_by_local("bgColor") else {
            return Ok(None);
        };
        return palette.resolve(color).map(Some).ok_or_else(unsupported);
    }
    match read_fill(fill, palette) {
        Fill::None => Ok(None),
        Fill::Solid(rgb) => Ok(Some(rgb)),
        // 条件格式里的图案 / 单元格渐变还没和 Excel 对过；渐变也不走 dxf 纯色捷径。
        Fill::Pattern(_, _, _) | Fill::Linear { .. } | Fill::Nope => Err(unsupported()),
    }
}

fn pattern_kind(name: &str) -> Option<PatternKind> {
    Some(match name {
        "gray125" => PatternKind::Gray125,
        "gray0625" => PatternKind::Gray0625,
        "lightHorizontal" => PatternKind::LightHorizontal,
        "darkHorizontal" => PatternKind::DarkHorizontal,
        "lightVertical" => PatternKind::LightVertical,
        "darkVertical" => PatternKind::DarkVertical,
        "lightGrid" => PatternKind::LightGrid,
        "darkGrid" => PatternKind::DarkGrid,
        "mediumGray" => PatternKind::MediumGray,
        "lightGray" => PatternKind::LightGray,
        "darkGray" => PatternKind::DarkGray,
        "darkDown" => PatternKind::DarkDown,
        "lightDown" => PatternKind::LightDown,
        "darkUp" => PatternKind::DarkUp,
        "lightUp" => PatternKind::LightUp,
        "darkTrellis" => PatternKind::DarkTrellis,
        "lightTrellis" => PatternKind::LightTrellis,
        _ => return None,
    })
}

/// 没写颜色时，前景是黑，背景是白。`rgb` 带了不是 `FF` 的透明度时不认。
fn pattern_channel(
    pat: &Element,
    name: &str,
    fallback: [u8; 3],
    palette: &Palette,
) -> Option<[u8; 3]> {
    let Some(el) = pat.child_by_local(name) else {
        return Some(fallback);
    };
    if let Some(rgb) = el.attr("rgb") {
        let hex = rgb.trim();
        if hex.len() == 8 && !hex[..2].eq_ignore_ascii_case("ff") {
            return None;
        }
    }
    palette.resolve(el)
}

fn parse_edges(border: &Element, palette: &Palette) -> Result<Edges, Error> {
    let diag = parse_diagonal_edge(border, palette)?;
    let down = if truthy(border.attr("diagonalDown")) {
        diag
    } else {
        None
    };
    let up = if truthy(border.attr("diagonalUp")) {
        diag
    } else {
        None
    };
    // 有对角标志但无可用 style，或 style 非本刀实线 1/2px → NI（不静默忽略）。
    if (truthy(border.attr("diagonalDown")) || truthy(border.attr("diagonalUp"))) && diag.is_none()
    {
        return Err(unsupported());
    }
    Ok(Edges {
        left: parse_edge(border, "left", palette)?,
        right: parse_edge(border, "right", palette)?,
        top: parse_edge(border, "top", palette)?,
        bottom: parse_edge(border, "bottom", palette)?,
        diagonal_down: down,
        diagonal_up: up,
    })
}

/// 斜线只认 thin(1) / medium(2) 实线；色取 `<diagonal><color/>`。其它线型明确 NI。
fn parse_diagonal_edge(border: &Element, palette: &Palette) -> Result<Option<Edge>, Error> {
    let Some(el) = border.child_by_local("diagonal") else {
        return Ok(None);
    };
    let Some(style) = el.attr("style") else {
        return Ok(None);
    };
    if style == "none" {
        return Ok(None);
    }
    let (kind, px) = match style {
        "thin" => (LineKind::Solid, 1),
        "medium" => (LineKind::Solid, 2),
        _ => return Err(unsupported()),
    };
    let color = el
        .child_by_local("color")
        .and_then(|c| palette.resolve(c))
        .unwrap_or([0, 0, 0]);
    Ok(Some(Edge { kind, px, color }))
}

fn parse_edge(border: &Element, name: &str, palette: &Palette) -> Result<Option<Edge>, Error> {
    let Some(el) = border.child_by_local(name) else {
        return Ok(None);
    };
    let Some(style) = el.attr("style") else {
        return Ok(None);
    };
    if style == "none" {
        return Ok(None);
    }
    // 双线占边界两侧，排序时按 3 像素，绘制时跳过正中那一像素。
    let (kind, px) = match style {
        "thin" => (LineKind::Solid, 1),
        "medium" => (LineKind::Solid, 2),
        "thick" => (LineKind::Solid, 3),
        "dashed" => (LineKind::Dashed, 1),
        "dotted" => (LineKind::Dotted, 1),
        "hair" => (LineKind::Hair, 1),
        "double" => (LineKind::Double, 3),
        "dashDot" => (LineKind::DashDot, 1),
        "dashDotDot" => (LineKind::DashDotDot, 1),
        "mediumDashed" => (LineKind::MediumDashed, 2),
        "mediumDashDot" => (LineKind::MediumDashDot, 2),
        "mediumDashDotDot" => (LineKind::MediumDashDotDot, 2),
        "slantDashDot" => (LineKind::SlantDashDot, 2),
        _ => return Err(unsupported()),
    };
    let color = match el.child_by_local("color") {
        None => [0, 0, 0],
        Some(color) => palette.resolve(color).ok_or_else(unsupported)?,
    };
    Ok(Some(Edge { kind, px, color }))
}

fn read_xf(xf: &Element, borders: &[Edges]) -> Result<Xf, Error> {
    let font = xf.attr("fontId").and_then(|s| s.parse().ok()).unwrap_or(0);
    let fill = xf.attr("fillId").and_then(|s| s.parse().ok()).unwrap_or(0);
    let border = xf
        .attr("borderId")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(0);
    let edges = if let Some(edges) = borders.get(border) {
        *edges
    } else if border == 0 {
        Edges::default()
    } else {
        return Err(unsupported());
    };
    let num_fmt = xf
        .attr("numFmtId")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let (h_align, v_align, wrap, indent, reading_order) = match xf.child_by_local("alignment") {
        None => (HAlign::General, VAlign::Bottom, false, 0u32, 0u8),
        Some(al) => {
            // 旋转 / 缩小字体仍 NI（旋转与两端对齐另见计划）。
            if al.attr("textRotation").is_some_and(|v| v != "0") || truthy(al.attr("shrinkToFit")) {
                return Err(unsupported());
            }
            let wrap = truthy(al.attr("wrapText"));
            let indent = match al.attr("indent") {
                None | Some("0") => 0u32,
                Some(raw) => {
                    let n: u32 = raw.parse().map_err(|_| unsupported())?;
                    if n > 15 {
                        return Err(unsupported());
                    }
                    n
                }
            };
            // readingOrder 与 sheetView.rightToLeft 不同：前者是单元格双向，后者是列镜像。
            let reading_order = match al.attr("readingOrder") {
                None | Some("0") => 0u8,
                Some("1") => 1u8,
                Some("2") => 2u8,
                Some(_) => return Err(unsupported()),
            };
            (
                read_h(al.attr("horizontal"))?,
                read_v(al.attr("vertical"))?,
                wrap,
                indent,
                reading_order,
            )
        }
    };
    Ok(Xf {
        font,
        fill,
        edges,
        num_fmt,
        h_align,
        v_align,
        wrap,
        indent,
        reading_order,
    })
}

fn read_h(value: Option<&str>) -> Result<HAlign, Error> {
    match value {
        None | Some("general") => Ok(HAlign::General),
        Some("left") => Ok(HAlign::Left),
        Some("center") => Ok(HAlign::Center),
        Some("right") => Ok(HAlign::Right),
        Some(_) => Err(unsupported()),
    }
}

fn read_v(value: Option<&str>) -> Result<VAlign, Error> {
    match value {
        None | Some("bottom") => Ok(VAlign::Bottom),
        Some("center") => Ok(VAlign::Center),
        Some("top") => Ok(VAlign::Top),
        Some(_) => Err(unsupported()),
    }
}

fn read_dxf(dxf: &Element, palette: &Palette) -> Result<Dxf, Error> {
    let fill = match dxf.child_by_local("fill") {
        None => None,
        Some(fill) => dxf_fill_color(fill, palette)?,
    };
    let color = dxf
        .child_by_local("font")
        .and_then(|font| font.child_by_local("color"))
        .and_then(|el| palette.resolve(el));
    // 竖向 dashed 的相位和 slantDashDot 的末端还没对齐。这两种和斜线仍让整表失败。
    let edges = match dxf.child_by_local("border") {
        None => Edges::default(),
        Some(border) => accepted_dxf_edges(parse_edges(border, palette)?)?,
    };
    Ok(Dxf { fill, color, edges })
}

fn accepted_dxf_edges(edges: Edges) -> Result<Edges, Error> {
    // 条件格式斜线仍 NI。dashed / slantDashDot 与格子共用 paint_line（相位/末端以 oracle 为准）。
    if edges.diagonal_down.is_some() || edges.diagonal_up.is_some() {
        return Err(unsupported());
    }
    Ok(edges)
}

fn strip_format_escapes(code: &str) -> String {
    let mut out = String::with_capacity(code.len());
    let mut chars = code.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            if let Some(next) = chars.next() {
                out.push(next);
            }
            continue;
        }
        out.push(ch);
    }
    out
}

fn named_datetime_pat(code: &str) -> Option<DateTimePat> {
    match code {
        "yyyy/mm/dd" => Some(DateTimePat::YyyyMmDd),
        "m/d/yy" => Some(DateTimePat::MdYy),
        "d-mmm-yy" => Some(DateTimePat::DMmmYy),
        "d-mmm" => Some(DateTimePat::DMmm),
        "mmm-yy" => Some(DateTimePat::MmmYy),
        "h:mm AM/PM" | "h:mm am/pm" => Some(DateTimePat::HmmAmpm),
        "h:mm:ss AM/PM" | "h:mm:ss am/pm" => Some(DateTimePat::HmmssAmpm),
        "h:mm" => Some(DateTimePat::Hmm),
        "h:mm:ss" => Some(DateTimePat::Hmmss),
        "m/d/yy h:mm" => Some(DateTimePat::MdYyHmm),
        _ => None,
    }
}

fn builtin_fmt(id: u32) -> Option<NumFmt> {
    match id {
        0 | 49 => Some(NumFmt::General),
        1 => Some(NumFmt::Fixed(0)),
        2 => Some(NumFmt::Fixed(2)),
        3 => Some(NumFmt::IntComma),
        9 => Some(NumFmt::Percent(0)),
        10 => Some(NumFmt::Percent(2)),
        14 => Some(NumFmt::DateTime(DateTimePat::MdYy)),
        15 => Some(NumFmt::DateTime(DateTimePat::DMmmYy)),
        16 => Some(NumFmt::DateTime(DateTimePat::DMmm)),
        17 => Some(NumFmt::DateTime(DateTimePat::MmmYy)),
        18 => Some(NumFmt::DateTime(DateTimePat::HmmAmpm)),
        19 => Some(NumFmt::DateTime(DateTimePat::HmmssAmpm)),
        20 => Some(NumFmt::DateTime(DateTimePat::Hmm)),
        21 => Some(NumFmt::DateTime(DateTimePat::Hmmss)),
        22 => Some(NumFmt::DateTime(DateTimePat::MdYyHmm)),
        _ => None,
    }
}

fn parse_format(code: &str) -> Result<NumFmt, Error> {
    if code.contains('[') {
        // 方括号颜色/条件：本切片继续 NI（计划：对不上的方括号条件继续失败）。
        return Err(unsupported());
    }
    if code.contains(';') {
        let parts = split_format_sections(code)?;
        if (2..=4).contains(&parts.len()) && parts.iter().all(|part| part.is_empty()) {
            return Ok(NumFmt::Blank(parts.len() as u8));
        }
        if !(1..=4).contains(&parts.len()) {
            return Err(unsupported());
        }
        let mut sections = Vec::with_capacity(parts.len());
        for part in parts {
            sections.push(parse_num_section(part)?);
        }
        return Ok(NumFmt::Sections(sections));
    }
    let code = code.trim();
    // 去掉格式码里的 `\` 转义后再匹配点名日期/时间图案（Excel 常写 `m/d/yy\ h:mm`）。
    let unlocked = strip_format_escapes(code);
    if let Some(pat) = named_datetime_pat(&unlocked) {
        return Ok(NumFmt::DateTime(pat));
    }
    if code.eq_ignore_ascii_case("general") {
        return Ok(NumFmt::General);
    }
    if code == "#,##0" {
        return Ok(NumFmt::IntComma);
    }
    if let Some(body) = code.strip_suffix('%') {
        return Ok(NumFmt::Percent(zeros(body)?));
    }
    // 单段货币等（含 `$` / `#,##0`）走 Sections(1)。
    if code.contains('$') || code.contains('#') {
        return Ok(NumFmt::Sections(vec![parse_num_section(code)?]));
    }
    Ok(NumFmt::Fixed(zeros(code)?))
}

fn split_format_sections(code: &str) -> Result<Vec<&str>, Error> {
    let mut parts = Vec::new();
    let mut start = 0usize;
    let mut in_quote = false;
    let bytes = code.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'"' {
            in_quote = !in_quote;
            i += 1;
            continue;
        }
        if b == b'\\' && i + 1 < bytes.len() {
            i += 2;
            continue;
        }
        if b == b';' && !in_quote {
            parts.push(&code[start..i]);
            start = i + 1;
        }
        i += 1;
    }
    if in_quote {
        return Err(unsupported());
    }
    parts.push(&code[start..]);
    Ok(parts)
}

fn parse_num_section(raw: &str) -> Result<NumSection, Error> {
    // 不 trim：尾空格段（如 `;;; `）在 Excel 里是字面空格，不能静默吃掉。
    if raw.is_empty() {
        return Ok(NumSection {
            literal: Some(String::new()),
            prefix: String::new(),
            suffix: String::new(),
            digits: 0,
            thousands: false,
        });
    }
    let s = raw;
    // 整段只是引号字面量。
    if let Some(inner) = s.strip_prefix('"').and_then(|rest| rest.strip_suffix('"')) {
        if !inner.contains('"') {
            return Ok(NumSection {
                literal: Some(inner.to_string()),
                prefix: String::new(),
                suffix: String::new(),
                digits: 0,
                thousands: false,
            });
        }
    }

    let mut prefix = String::new();
    let mut suffix = String::new();
    let mut digits = 0u32;
    let mut thousands = false;
    let mut saw_num = false;
    let mut chars = s.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '"' => {
                let mut lit = String::new();
                loop {
                    let Some(c) = chars.next() else {
                        return Err(unsupported());
                    };
                    if c == '"' {
                        break;
                    }
                    lit.push(c);
                }
                if saw_num {
                    suffix.push_str(&lit);
                } else {
                    prefix.push_str(&lit);
                }
            }
            '_' => {
                let Some(_) = chars.next() else {
                    return Err(unsupported());
                };
                // 跳宽：显示串与 Excel Range.Text 对齐为空格（非其它字体宽度泛化）。
                if saw_num {
                    suffix.push(' ');
                } else {
                    prefix.push(' ');
                }
            }
            '\\' => {
                // OOXML 转义：`\$` `\(` `\)` 等 → 字面量（Excel 存盘常见）。
                let Some(next) = chars.next() else {
                    return Err(unsupported());
                };
                if saw_num {
                    suffix.push(next);
                } else {
                    prefix.push(next);
                }
            }
            '(' | ')' | '$' => {
                if saw_num {
                    suffix.push(ch);
                } else {
                    prefix.push(ch);
                }
            }
            '#' | '0' | ',' | '.' => {
                // 把连续数字骨架收成 thousands + digits。
                let mut body = String::new();
                body.push(ch);
                while let Some(c) = chars.peek().copied() {
                    if matches!(c, '#' | '0' | ',' | '.') {
                        body.push(c);
                        chars.next();
                    } else {
                        break;
                    }
                }
                if body.contains(',') {
                    thousands = true;
                }
                if let Some((_, frac)) = body.split_once('.') {
                    if !frac.chars().all(|c| c == '0') {
                        return Err(unsupported());
                    }
                    digits = frac.len() as u32;
                } else {
                    digits = 0;
                }
                // 必须是 #,##0 / 0 / 0.00 一类。
                let int_part = body.split('.').next().unwrap_or("");
                let ok = int_part == "0"
                    || int_part == "#,##0"
                    || int_part == "#,###"
                    || int_part.chars().all(|c| c == '#' || c == '0' || c == ',');
                if !ok && int_part != "#,##0" {
                    // 再收紧：只接受常见货币骨架。
                    if !(int_part.contains('0') && int_part.chars().all(|c| "#,0".contains(c))) {
                        return Err(unsupported());
                    }
                }
                saw_num = true;
            }
            ' ' => {
                if saw_num {
                    suffix.push(' ');
                } else {
                    prefix.push(' ');
                }
            }
            _ => return Err(unsupported()),
        }
    }
    if !saw_num {
        // 无数字骨架：整段当字面量（含仅空格的文本段）。
        return Ok(NumSection {
            literal: Some(s.to_string()),
            prefix: String::new(),
            suffix: String::new(),
            digits: 0,
            thousands: false,
        });
    }
    Ok(NumSection {
        literal: None,
        prefix,
        suffix,
        digits,
        thousands,
    })
}

fn zeros(body: &str) -> Result<u32, Error> {
    if body == "0" {
        return Ok(0);
    }
    let Some(rest) = body.strip_prefix("0.") else {
        return Err(unsupported());
    };
    if rest.is_empty() || !rest.chars().all(|c| c == '0') {
        return Err(unsupported());
    }
    Ok(rest.len() as u32)
}

fn parse_rgb(s: &str) -> Option<[u8; 3]> {
    let hex = s.trim();
    let hex = if hex.len() == 8 {
        &hex[2..]
    } else if hex.len() == 6 {
        hex
    } else {
        return None;
    };
    let n = u32::from_str_radix(hex, 16).ok()?;
    Some([
        ((n >> 16) & 0xff) as u8,
        ((n >> 8) & 0xff) as u8,
        (n & 0xff) as u8,
    ])
}

#[derive(Clone, Debug)]
enum Shown {
    Empty,
    Num(f64),
    Text(String),
    Bool(bool),
    /// `t="e"` 且 `<v>` 为点名错误码（如 `#DIV/0!`）。显示串用缓存原文。
    Err(String),
}

struct CellRec {
    style: usize,
    shown: Shown,
}

fn read_cells(
    sheet: &Element,
    sst: &[String],
    session: &mut Option<FormulaSession>,
) -> Result<HashMap<(u32, u32), CellRec>, Error> {
    let mut out = HashMap::new();
    let Some(data) = sheet.child_by_local("sheetData") else {
        return Ok(out);
    };
    let mut prev_row = 0u32;
    for row in data.children_by_local("row") {
        // 隐藏行也读。合并的锚点可能藏在这里，填充仍要铺到可见格子上。
        let r = a1::row_after(prev_row, row.attr("r"));
        prev_row = r;
        let mut prev_col = 0u32;
        for cell in row.children_by_local("c") {
            let (col, row_n) = cell_addr(prev_col, r, cell.attr("r"))?;
            prev_col = col;
            let style = cell
                .attr("s")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0usize);
            let shown = shown_of(cell, sst, CellRef { col, row: row_n }, session)?;
            out.insert((col, row_n), CellRec { style, shown });
        }
    }
    Ok(out)
}

fn shown_of(
    cell: &Element,
    sst: &[String],
    here: CellRef,
    session: &mut Option<FormulaSession>,
) -> Result<Shown, Error> {
    // 有 `<v>` 或内联结果就用缓存。共享公式的从属格公式文本是空的，值在缓存里。
    if cell.child_by_local("f").is_some() && !has_cached_value(cell) {
        let text = cell
            .child_by_local("f")
            .map(|el| el.text_content())
            .unwrap_or_default();
        if text.trim().is_empty() {
            return Err(unsupported());
        }
        let Some(session) = session.as_mut() else {
            return Err(unsupported());
        };
        return match session.displayed(&text, here) {
            Displayed::Num(n) => Ok(Shown::Num(n)),
            Displayed::Text(s) => Ok(Shown::Text(s)),
            Displayed::Bool(b) => Ok(Shown::Bool(b)),
            Displayed::Empty => Ok(Shown::Empty),
            Displayed::Bad => Err(unsupported()),
        };
    }
    value_shown(cell, sst)
}

fn has_cached_value(cell: &Element) -> bool {
    cell.child_by_local("v").is_some() || cell.child_by_local("is").is_some()
}

fn known_error_code(code: &str) -> bool {
    matches!(
        code,
        "#NULL!" | "#DIV/0!" | "#VALUE!" | "#REF!" | "#NAME?" | "#NUM!" | "#N/A"
    )
}

fn value_shown(cell: &Element, sst: &[String]) -> Result<Shown, Error> {
    let kind = cell.attr("t").unwrap_or("n");
    let value = cell
        .child_by_local("v")
        .map(|el| el.text_content())
        .unwrap_or_default();
    match kind {
        "s" => {
            let idx = value.parse::<usize>().map_err(|_| unsupported())?;
            Ok(Shown::Text(sst.get(idx).cloned().unwrap_or_default()))
        }
        "inlineStr" => Ok(Shown::Text(
            cell.child_by_local("is")
                .map(|el| el.text_content())
                .unwrap_or_default(),
        )),
        "str" => Ok(Shown::Text(value)),
        "b" => Ok(Shown::Bool(
            value == "1" || value.eq_ignore_ascii_case("true"),
        )),
        "n" | "d" if value.is_empty() => Ok(Shown::Empty),
        "n" => Ok(Shown::Num(value.parse().map_err(|_| unsupported())?)),
        "e" => {
            if value.is_empty() || !known_error_code(&value) {
                return Err(unsupported());
            }
            Ok(Shown::Err(value))
        }
        _ => Err(unsupported()),
    }
}

fn column_sizes(sheet: &Element, c0: u32, c1: u32) -> Result<Vec<i32>, Error> {
    let default = sheet
        .child_by_local("sheetFormatPr")
        .and_then(|el| el.attr("defaultColWidth"))
        .and_then(|s| s.parse().ok());
    let mut sizes = Vec::new();
    for col in c0..=c1 {
        sizes.push(column_px(sheet, col, default)?);
    }
    Ok(sizes)
}

fn column_px(sheet: &Element, col: u32, default: Option<f64>) -> Result<i32, Error> {
    for cols in sheet.children_by_local("cols") {
        for el in cols.children_by_local("col") {
            if truthy(el.attr("hidden")) {
                let min = el.attr("min").and_then(|s| s.parse::<u32>().ok());
                let max = el.attr("max").and_then(|s| s.parse::<u32>().ok());
                if let (Some(min), Some(max)) = (min, max) {
                    if col >= min && col <= max {
                        // 隐藏列不占像素。XML 里的宽度留着，出图不用。
                        return Ok(0);
                    }
                }
            }
            let Some(min) = el.attr("min").and_then(|s| s.parse::<u32>().ok()) else {
                continue;
            };
            let Some(max) = el.attr("max").and_then(|s| s.parse::<u32>().ok()) else {
                continue;
            };
            if col < min || col > max {
                continue;
            }
            if let Some(width) = el.attr("width").and_then(|s| s.parse::<f64>().ok()) {
                return Ok(excel_col_px(width).max(1));
            }
        }
    }
    Ok(match default {
        Some(width) => excel_col_px(width).max(1),
        None => DEFAULT_COL_PX,
    })
}

/// Excel 把 OOXML `col/@width`（已含 padding 的字符宽单位）换成 **96 dpi** 像素。
/// MDW=7：Calibri 11（Normal）在 96dpi 下 digit width 的规范取值；与销售看板金样 `excel_col_px(16.7109375)=117` 锁定。
/// 不把本机 106 屏 MDW≈8 硬写进公式。高 DPI 存盘 width 相对 COM 点宽推导的 96dpi 目标可能偏短——记残差，不改 MDW。
fn excel_col_px(width: f64) -> i32 {
    let mdw = 7.0f64;
    let px = ((256.0 * width + (128.0 / mdw).floor()) / 256.0) * mdw;
    px.floor() as i32
}

struct RowBase {
    pt: f64,
    /// `customHeight` 为真时，常规字体不再把行顶高。
    lock_normal: bool,
}

fn row_base(sheet: &Element, styles: &Styles) -> RowBase {
    let pr = sheet.child_by_local("sheetFormatPr");
    let written = pr
        .and_then(|el| el.attr("defaultRowHeight"))
        .and_then(|s| s.parse().ok());
    let custom = pr.is_some_and(|el| truthy(el.attr("customHeight")));
    if custom {
        if let Some(pt) = written {
            return RowBase {
                pt,
                lock_normal: true,
            };
        }
    }
    // 写了磅值但没有 customHeight 时，Excel 不用这个数字，改用常规样式。
    if written.is_some() {
        return RowBase {
            pt: styles.normal_row_pt(),
            lock_normal: false,
        };
    }
    RowBase {
        pt: DEFAULT_ROW_PT,
        lock_normal: false,
    }
}

fn normal_font_index(root: &Element) -> usize {
    let Some(styles) = root.child_by_local("cellStyles") else {
        return 0;
    };
    let Some(style) = styles
        .children_by_local("cellStyle")
        .into_iter()
        .find(|el| {
            el.attr("name")
                .is_some_and(|name| name.eq_ignore_ascii_case("Normal"))
        })
    else {
        return 0;
    };
    let xf_id = style
        .attr("xfId")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(0);
    let Some(xfs) = root.child_by_local("cellStyleXfs") else {
        return 0;
    };
    xfs.children_by_local("xf")
        .into_iter()
        .nth(xf_id)
        .and_then(|xf| xf.attr("fontId"))
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}

fn row_sizes(
    sheet: &Element,
    styles: &Styles,
    cells: &HashMap<(u32, u32), CellRec>,
    r0: u32,
    r1: u32,
) -> Result<Vec<i32>, Error> {
    let base = row_base(sheet, styles);
    let mut custom = HashMap::new();
    let mut hidden = HashSet::new();
    if let Some(data) = sheet.child_by_local("sheetData") {
        let mut prev = 0u32;
        for row in data.children_by_local("row") {
            let r = a1::row_after(prev, row.attr("r"));
            prev = r;
            if truthy(row.attr("hidden")) {
                hidden.insert(r);
            }
            if let Some(ht) = row.attr("ht").and_then(|s| s.parse::<f64>().ok()) {
                custom.insert(r, ht);
            }
        }
    }
    let mut sizes = Vec::new();
    for row in r0..=r1 {
        if hidden.contains(&row) {
            sizes.push(0);
            continue;
        }
        let pt = if let Some(ht) = custom.get(&row) {
            *ht
        } else {
            row_auto_pt(sheet, styles, cells, row, &base)?
        };
        let px = excel_row_px(pt);
        if px < 1 {
            return Err(unsupported());
        }
        sizes.push(px);
    }
    Ok(sizes)
}

fn row_auto_pt(
    sheet: &Element,
    styles: &Styles,
    cells: &HashMap<(u32, u32), CellRec>,
    row: u32,
    base: &RowBase,
) -> Result<f64, Error> {
    let normal = styles.normal_font();
    let mut pt = base.pt;
    let mut raise = |font: &FontInfo| {
        if base.lock_normal && same_face(font, normal) {
            return;
        }
        pt = pt.max(auto_row_pt(&font.name, font.pt));
    };
    for ((_, r), cell) in cells {
        if *r != row {
            continue;
        }
        raise(styles.font(cell.style)?);
    }
    // 合并盖住的行没有自己的格子。锚点的字仍把这些行加高，隐藏的锚点也算。
    if let Some(list) = sheet.child_by_local("mergeCells") {
        for el in list.children_by_local("mergeCell") {
            let Some(raw) = el.attr("ref") else {
                continue;
            };
            let (a, b) = a1::parse_range(raw).map_err(|_| unsupported())?;
            let r0 = a.row.min(b.row);
            let r1 = a.row.max(b.row);
            if row < r0 || row > r1 || row == r0 {
                continue;
            }
            let anchor = cells.get(&(a.col.min(b.col), r0));
            let Some(cell) = anchor else {
                continue;
            };
            raise(styles.font(cell.style)?);
        }
    }
    Ok(pt)
}

fn same_face(font: &FontInfo, normal: &FontInfo) -> bool {
    font.name.eq_ignore_ascii_case(&normal.name) && (font.pt - normal.pt).abs() < 0.05
}

/// 未写行高时的磅值。Calibri 11 是 15。10 磅微软雅黑是 16.5。
fn auto_row_pt(name: &str, pt: f64) -> f64 {
    // 等线/DengXian：COM Excel 16.0 StandardHeight / AutoFit 矩阵（2026-11-17 Asia/Shanghai）：
    // 字号 10/11/空/StandardHeight → 14.25；12 → 15.75；18 → 23。
    // 本刀只锁 11→14.25（与 sheetFormatPr 缺省 14.25 一致）；其它字号暂 pt*15/11，不拟合单例 delta。
    // 不回归：Calibri 11 仍 15（COM AutoFit 亦见 14.25，契约保持 15）；雅黑仍 1.65×。
    // RTL 探针曾误走雅黑 1.65→18.15→高 49，COM 两行≈28.5pt→≈38px，残差非纯 DPI。
    if name.eq_ignore_ascii_case("等线") || name.eq_ignore_ascii_case("DengXian") {
        if (pt - 11.0).abs() < 0.05 {
            return 14.25;
        }
        return pt * 15.0 / 11.0;
    }
    const CJK: &[&str] = &[
        "微软雅黑",
        "宋体",
        "黑体",
        "楷体",
        "仿宋",
        "Microsoft YaHei",
        "SimSun",
        "SimHei",
    ];
    if CJK.iter().any(|item| item.eq_ignore_ascii_case(name)) {
        pt * 165.0 / 100.0
    } else if name.eq_ignore_ascii_case("Calibri") && (pt - 11.0).abs() < 0.05 {
        15.0
    } else {
        pt * 15.0 / 11.0
    }
}

/// 自定义行高截断成像素。20 磅是 26，不是四舍五入的 27。
fn excel_row_px(pt: f64) -> i32 {
    let emu = (pt * 12_700.0).round();
    (emu / 9_525.0).floor() as i32
}

struct Axis {
    origin: Vec<i32>,
}

impl Axis {
    fn new(sizes: &[i32]) -> Self {
        let mut origin = Vec::with_capacity(sizes.len() + 1);
        let mut at = 0;
        origin.push(0);
        for size in sizes {
            at += size;
            origin.push(at);
        }
        Self { origin }
    }

    fn span(&self) -> i32 {
        self.origin.last().copied().unwrap_or(0)
    }

    fn bitmap(&self) -> i32 {
        self.span() + 1
    }

    fn has_span(&self) -> bool {
        self.origin.windows(2).any(|pair| pair[1] > pair[0])
    }
}

struct Canvas {
    w: i32,
    h: i32,
    /// 窗口左上角在整表位图中的绝对像素坐标。`put`/`blend` 吃绝对坐标。
    ox: i32,
    oy: i32,
    px: Vec<u8>,
}

impl Canvas {
    #[allow(dead_code)]
    fn new(w: i32, h: i32) -> Result<Self, Error> {
        Self::window(0, 0, w, h)
    }

    /// 只分配 `tw×th`；不分配整表。边长仍受 `MAX_PX` 约束。
    fn window(ox: i32, oy: i32, tw: i32, th: i32) -> Result<Self, Error> {
        if !(1..=MAX_PX).contains(&tw) || !(1..=MAX_PX).contains(&th) {
            return Err(unsupported());
        }
        if ox < 0 || oy < 0 {
            return Err(unsupported());
        }
        let n = (tw as usize)
            .checked_mul(th as usize)
            .and_then(|n| n.checked_mul(3))
            .ok_or_else(unsupported)?;
        Ok(Self {
            w: tw,
            h: th,
            ox,
            oy,
            px: vec![255; n],
        })
    }

    fn put(&mut self, x: i32, y: i32, color: [u8; 3]) {
        let x = x - self.ox;
        let y = y - self.oy;
        if x < 0 || y < 0 || x >= self.w || y >= self.h {
            return;
        }
        let i = (y * self.w + x) as usize * 3;
        self.px[i] = color[0];
        self.px[i + 1] = color[1];
        self.px[i + 2] = color[2];
    }

    fn blend(&mut self, x: i32, y: i32, color: [u8; 3], cov: u8) {
        let x = x - self.ox;
        let y = y - self.oy;
        if cov == 0 || x < 0 || y < 0 || x >= self.w || y >= self.h {
            return;
        }
        if cov == 255 {
            self.put(x + self.ox, y + self.oy, color);
            return;
        }
        let i = (y * self.w + x) as usize * 3;
        for (k, src) in color.iter().enumerate() {
            let dst = u16::from(self.px[i + k]);
            let src = u16::from(*src);
            self.px[i + k] =
                ((src * u16::from(cov) + dst * u16::from(255 - cov) + 127) / 255) as u8;
        }
    }

    fn fill_rect(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, color: [u8; 3]) {
        for y in y0..y1 {
            for x in x0..x1 {
                self.put(x, y, color);
            }
        }
    }

    /// 填到右缘和下缘的网格像素。后画的格子盖住交界。
    fn fill_cell(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, color: [u8; 3]) {
        self.fill_rect(x0, y0, x1 + 1, y1 + 1, color);
    }

    /// 图案和纯色铺同一块矩形。点用整张图的像素坐标。
    fn fill_pattern(
        &mut self,
        x0: i32,
        y0: i32,
        x1: i32,
        y1: i32,
        kind: PatternKind,
        fg: [u8; 3],
        bg: [u8; 3],
    ) {
        for y in y0..=y1 {
            for x in x0..=x1 {
                let color = if pattern_on(kind, x, y) { fg } else { bg };
                self.put(x, y, color);
            }
        }
    }

    /// SpreadsheetML 线性渐变。`degree` 与 ECMA-376 / MS Open XML：0 左→右，90 上→下。
    /// 四角投影归一化后通道 `mix`（sRGB 直线）。不调用 PPT `gamma_mix`。
    fn fill_linear(
        &mut self,
        x0: i32,
        y0: i32,
        x1: i32,
        y1: i32,
        degree: f64,
        start: [u8; 3],
        end: [u8; 3],
    ) {
        let w = f64::from(x1 - x0);
        let h = f64::from(y1 - y0);
        let rad = degree.to_radians();
        let (dx, dy) = (rad.cos(), rad.sin());
        let corners = [(0.0, 0.0), (w, 0.0), (0.0, h), (w, h)];
        let mut pmin = f64::INFINITY;
        let mut pmax = f64::NEG_INFINITY;
        for (cx, cy) in corners {
            let p = cx * dx + cy * dy;
            pmin = pmin.min(p);
            pmax = pmax.max(p);
        }
        let span = pmax - pmin;
        for y in y0..=y1 {
            for x in x0..=x1 {
                let px = f64::from(x - x0);
                let py = f64::from(y - y0);
                let p = px * dx + py * dy;
                let t = if span.abs() < 1e-9 {
                    0.0
                } else {
                    (p - pmin) / span
                };
                self.put(x, y, mix(start, end, t));
            }
        }
    }

    fn grid(&mut self, xs: &Axis, ys: &Axis) {
        let y0 = self.oy;
        let y1 = self.oy + self.h;
        let x0 = self.ox;
        let x1 = self.ox + self.w;
        for x in &xs.origin {
            if *x < x0 || *x >= x1 {
                continue;
            }
            for y in y0..y1 {
                self.put(*x, y, GRID);
            }
        }
        for y in &ys.origin {
            if *y < y0 || *y >= y1 {
                continue;
            }
            for x in x0..x1 {
                self.put(x, *y, GRID);
            }
        }
    }

    /// 上边和左边的外框盖住第一格的填充。网格开着时是网格色，关掉时留白。
    /// 切块时只在整表绝对边 `x=0`/`y=0` 落入本窗时画。
    fn leading_frame(&mut self, color: [u8; 3]) {
        let x0 = self.ox;
        let x1 = self.ox + self.w;
        let y0 = self.oy;
        let y1 = self.oy + self.h;
        if y0 == 0 {
            for x in x0..x1 {
                self.put(x, 0, color);
            }
        }
        if x0 == 0 {
            for y in y0..y1 {
                self.put(0, y, color);
            }
        }
    }

    fn flip_horizontal(&mut self) {
        let w = self.w as usize;
        let h = self.h as usize;
        for y in 0..h {
            for x in 0..w / 2 {
                let i = (y * w + x) * 3;
                let j = (y * w + (w - 1 - x)) * 3;
                for k in 0..3 {
                    self.px.swap(i + k, j + k);
                }
            }
        }
    }
}

struct Merge {
    c0: u32,
    r0: u32,
    c1: u32,
    r1: u32,
}

fn paint(
    sheet: &Element,
    styles: &Styles,
    cells: &HashMap<(u32, u32), CellRec>,
    c0: u32,
    r0: u32,
    cols: &[i32],
    rows: &[i32],
    grid_lines: bool,
    rtl: bool,
    shapes: &[Overlay],
    session: &mut Option<FormulaSession>,
) -> Result<Canvas, Error> {
    let xs = Axis::new(cols);
    let ys = Axis::new(rows);
    paint_window(
        sheet,
        styles,
        cells,
        c0,
        r0,
        cols,
        rows,
        grid_lines,
        rtl,
        shapes,
        session,
        0,
        0,
        xs.bitmap(),
        ys.bitmap(),
    )
}

fn paint_window(
    sheet: &Element,
    styles: &Styles,
    cells: &HashMap<(u32, u32), CellRec>,
    c0: u32,
    r0: u32,
    cols: &[i32],
    rows: &[i32],
    grid_lines: bool,
    rtl: bool,
    shapes: &[Overlay],
    session: &mut Option<FormulaSession>,
    ox: i32,
    oy: i32,
    tw: i32,
    th: i32,
) -> Result<Canvas, Error> {
    let xs = Axis::new(cols);
    let ys = Axis::new(rows);
    let full_w = xs.bitmap();
    let full_h = ys.bitmap();
    if ox < 0
        || oy < 0
        || tw < 1
        || th < 1
        || ox.saturating_add(tw) > full_w
        || oy.saturating_add(th) > full_h
    {
        return Err(unsupported());
    }
    // RTL 切块会改翻转语义，本刀不切 RTL。
    if rtl && (ox != 0 || oy != 0 || tw != full_w || th != full_h) {
        return Err(unsupported());
    }
    let mut canvas = Canvas::window(ox, oy, tw, th)?;
    // 一整边都隐藏时，Excel 只留 1 像素白，不画网格。
    let draw_frame = xs.has_span() && ys.has_span();
    if draw_frame && grid_lines {
        canvas.grid(&xs, &ys);
    }
    let merges = merges_of(
        sheet,
        c0,
        r0,
        c0 + cols.len() as u32 - 1,
        r0 + rows.len() as u32 - 1,
    )?;
    let covered = covered_of(&merges);
    let mut rects = Vec::new();
    for merge in &merges {
        rects.push((merge.r0, merge.c0, merge.c1, merge.r1));
    }
    for &(col, row) in cells.keys() {
        if col < c0 || row < r0 || col >= c0 + cols.len() as u32 || row >= r0 + rows.len() as u32 {
            continue;
        }
        if covered.contains(&(col, row)) || merges.iter().any(|m| m.c0 == col && m.r0 == row) {
            continue;
        }
        rects.push((row, col, col, row));
    }
    if !cols.is_empty() && !rows.is_empty() {
        let c1 = c0 + cols.len() as u32 - 1;
        let r1 = r0 + rows.len() as u32 - 1;
        for (col, row) in expression_gaps(sheet, c0, r0, c1, r1)? {
            if cells.contains_key(&(col, row))
                || covered.contains(&(col, row))
                || merges.iter().any(|m| m.c0 == col && m.r0 == row)
            {
                continue;
            }
            rects.push((row, col, col, row));
        }
    }
    rects.sort_unstable();
    for &(row, col, col_end, row_end) in &rects {
        paint_cell(
            &mut canvas,
            sheet,
            styles,
            cells,
            &xs,
            &ys,
            c0,
            r0,
            col,
            row,
            col_end,
            row_end,
            session,
        )?;
    }
    paint_rules(&mut canvas, sheet, cells, &xs, &ys, c0, r0)?;
    let mut fonts = FontBook::default();
    // sheetView RTL：无文字整图翻转已差 0。有文字时 Excel 只换列边、字仍正写——
    // 先画非文字层再翻转，最后在翻转坐标系画字（不 Unicode 反序、不镜像字形）。
    if !rtl {
        paint_labels(
            &mut canvas,
            &mut fonts,
            styles,
            cells,
            &merges,
            &covered,
            &xs,
            &ys,
            c0,
            r0,
            sheet,
            false,
            session,
        )?;
    }
    if draw_frame {
        canvas.leading_frame(if grid_lines { GRID } else { [255, 255, 255] });
    }
    paint_borders(
        &mut canvas,
        sheet,
        styles,
        cells,
        &xs,
        &ys,
        c0,
        r0,
        &rects,
        session,
    )?;
    paint_text_number_marks(&mut canvas, cells, &merges, &covered, &xs, &ys, c0, r0)?;
    paint_plain_rects(&mut canvas, shapes);
    if rtl {
        canvas.flip_horizontal();
        paint_labels(
            &mut canvas,
            &mut fonts,
            styles,
            cells,
            &merges,
            &covered,
            &xs,
            &ys,
            c0,
            r0,
            sheet,
            true,
            session,
        )?;
    }
    Ok(canvas)
}

fn merges_of(sheet: &Element, c0: u32, r0: u32, c1: u32, r1: u32) -> Result<Vec<Merge>, Error> {
    let mut out = Vec::new();
    let Some(list) = sheet.child_by_local("mergeCells") else {
        return Ok(out);
    };
    for el in list.children_by_local("mergeCell") {
        let Some(r) = el.attr("ref") else {
            continue;
        };
        let (a, b) = a1::parse_range(r).map_err(|_| unsupported())?;
        let merge = Merge {
            c0: a.col.min(b.col),
            r0: a.row.min(b.row),
            c1: a.col.max(b.col),
            r1: a.row.max(b.row),
        };
        let hits = overlaps(merge.c0, merge.c1, c0, c1) && overlaps(merge.r0, merge.r1, r0, r1);
        if !hits {
            continue;
        }
        if merge.c0 < c0 || merge.c1 > c1 || merge.r0 < r0 || merge.r1 > r1 {
            return Err(unsupported());
        }
        out.push(merge);
    }
    Ok(out)
}

fn covered_of(merges: &[Merge]) -> HashSet<(u32, u32)> {
    let mut out = HashSet::new();
    for merge in merges {
        for row in merge.r0..=merge.r1 {
            for col in merge.c0..=merge.c1 {
                if col == merge.c0 && row == merge.r0 {
                    continue;
                }
                out.insert((col, row));
            }
        }
    }
    out
}

fn paint_cell(
    canvas: &mut Canvas,
    sheet: &Element,
    styles: &Styles,
    cells: &HashMap<(u32, u32), CellRec>,
    xs: &Axis,
    ys: &Axis,
    c0: u32,
    r0: u32,
    col: u32,
    row: u32,
    col_end: u32,
    row_end: u32,
    session: &mut Option<FormulaSession>,
) -> Result<(), Error> {
    let x0 = xs.origin[(col - c0) as usize];
    let x1 = xs.origin[(col_end - c0) as usize + 1];
    let y0 = ys.origin[(row - r0) as usize];
    let y1 = ys.origin[(row_end - r0) as usize + 1];
    // 隐藏行、隐藏列的跨度是 0。不能往交界上涂出 1 像素。
    if x1 == x0 || y1 == y0 {
        return Ok(());
    }
    let fill = if let Some(cell) = cells.get(&(col, row)) {
        let xf = styles.xf(cell.style)?;
        let fill = styles.fills.get(xf.fill).copied().unwrap_or(Fill::Nope);
        if matches!(fill, Fill::Nope) {
            return Err(unsupported());
        }
        Some(fill)
    } else {
        None
    };
    if let Some(rgb) = cf_fill(sheet, styles, cells, col, row, session)? {
        canvas.fill_cell(x0, y0, x1, y1, rgb);
        return Ok(());
    }
    let Some(fill) = fill else {
        return Ok(());
    };
    match fill {
        Fill::Solid(rgb) => canvas.fill_cell(x0, y0, x1, y1, rgb),
        Fill::Pattern(kind, fg, bg) => canvas.fill_pattern(x0, y0, x1, y1, kind, fg, bg),
        Fill::Linear { degree, start, end } => {
            canvas.fill_linear(x0, y0, x1, y1, degree, start, end)
        }
        Fill::None | Fill::Nope => {}
    }
    Ok(())
}

/// 点阵按 Excel 16.0 的复制图片量过。坐标含外框，不从格子原点重算。
fn pattern_on(kind: PatternKind, x: i32, y: i32) -> bool {
    let x4 = x.rem_euclid(4);
    let y4 = y.rem_euclid(4);
    match kind {
        PatternKind::Gray125 => (y4 == 2 && x4 == 2) || (y4 == 0 && x4 == 0),
        PatternKind::Gray0625 => {
            let x8 = x.rem_euclid(8);
            let y8 = y.rem_euclid(8);
            ((y8 == 2 || y8 == 6) && x8 == 4) || ((y8 == 0 || y8 == 4) && x8 == 0)
        }
        PatternKind::LightHorizontal => y4 == 0,
        PatternKind::DarkHorizontal => y4 == 0 || y4 == 1,
        PatternKind::LightVertical => x4 == 0,
        PatternKind::DarkVertical => x4 == 0 || x4 == 1,
        PatternKind::LightGrid => x4 == 0 || y4 == 0,
        PatternKind::DarkGrid => {
            if y4 == 1 || y4 == 2 {
                x4 == 1 || x4 == 2
            } else {
                x4 == 0 || x4 == 3
            }
        }
        PatternKind::MediumGray => (x + y).rem_euclid(2) == 0,
        PatternKind::LightGray => {
            if y.rem_euclid(2) == 1 {
                x4 == 2
            } else {
                x4 == 0
            }
        }
        PatternKind::DarkGray => {
            if y.rem_euclid(2) == 1 {
                x4 != 1
            } else {
                x4 != 3
            }
        }
        PatternKind::DarkDown => (x - y).rem_euclid(4) <= 1,
        PatternKind::LightDown => x4 == y4,
        PatternKind::DarkUp => {
            let sum = (x + y).rem_euclid(4);
            sum == 2 || sum == 3
        }
        PatternKind::LightUp => (x + y).rem_euclid(4) == 3,
        PatternKind::DarkTrellis => {
            y.rem_euclid(2) == 0 || {
                let sum = (x + y).rem_euclid(4);
                sum == 2 || sum == 3
            }
        }
        PatternKind::LightTrellis => {
            if y.rem_euclid(2) == 0 {
                x4 == y4
            } else {
                x.rem_euclid(2) == 1
            }
        }
    }
}

fn paint_borders(
    canvas: &mut Canvas,
    sheet: &Element,
    styles: &Styles,
    cells: &HashMap<(u32, u32), CellRec>,
    xs: &Axis,
    ys: &Axis,
    c0: u32,
    r0: u32,
    rects: &[(u32, u32, u32, u32)],
    session: &mut Option<FormulaSession>,
) -> Result<(), Error> {
    reject_other_dxf_borders(sheet, styles)?;
    let mut strokes = Vec::new();
    let mut order = 0u32;
    for &(row, col, col_end, row_end) in rects {
        let mut edges = resolved_edges(sheet, styles, cells, session, col, row)?;
        if !edges_present(edges) {
            continue;
        }
        // 边框属于写下它的那一格。合并矩形只用来丢掉伸进合并区内部的边。
        let x0 = xs.origin[(col - c0) as usize];
        let x1 = xs.origin[(col - c0) as usize + 1];
        let y0 = ys.origin[(row - r0) as usize];
        let y1 = ys.origin[(row - r0) as usize + 1];
        if x1 == x0 || y1 == y0 {
            continue;
        }
        if col_end > col {
            edges.right = None;
        }
        if row_end > row {
            edges.bottom = None;
        }
        let left_pad = end_pad(edges.left);
        let right_pad = end_pad(edges.right);
        let top_pad = end_pad(edges.top);
        let bottom_pad = end_pad(edges.bottom);
        // 比左边更细时，底边左端才让开。左边是虚线让开全部厚度，实线让开一半。
        let bottom_inset = inset_for(
            edges.bottom,
            neighbor_bottom(sheet, styles, cells, session, col, row)?,
        );
        // 自己有上边，或上面那一格有底边时，上边界留给横线。两边都没有才从边界涂起。
        let yield_top =
            edges.top.is_some() || above_has_bottom(sheet, styles, cells, session, col, row)?;
        push_stroke(
            &mut strokes,
            &mut order,
            edges.left,
            Side::Left,
            x0,
            y0,
            x1,
            y1,
            top_pad,
            bottom_pad,
            0,
            yield_top,
        );
        push_stroke(
            &mut strokes,
            &mut order,
            edges.right,
            Side::Right,
            x0,
            y0,
            x1,
            y1,
            top_pad,
            bottom_pad,
            0,
            yield_top,
        );
        push_stroke(
            &mut strokes,
            &mut order,
            edges.top,
            Side::Top,
            x0,
            y0,
            x1,
            y1,
            left_pad,
            right_pad,
            0,
            false,
        );
        push_stroke(
            &mut strokes,
            &mut order,
            edges.bottom,
            Side::Bottom,
            x0,
            y0,
            x1,
            y1,
            left_pad,
            right_pad,
            bottom_inset,
            false,
        );
        // 斜线：格子像素矩形真实边（[x0,x1)×[y0,y1)），不搬 PPT 端点偏移假设。
        if let Some(edge) = edges.diagonal_down {
            paint_cell_diagonal(canvas, x0, y0, x1, y1, true, edge);
        }
        if let Some(edge) = edges.diagonal_up {
            paint_cell_diagonal(canvas, x0, y0, x1, y1, false, edge);
        }
    }
    // 更粗的后画。同样粗时，右边缘盖住左边缘，其余保持格子的先后。
    strokes.sort_by_key(|stroke| (stroke.px, stroke.rank, stroke.order));
    for stroke in strokes {
        paint_line(canvas, &stroke);
    }
    Ok(())
}

/// `down=true`：左上→右下；`false`：左下→右上。仅 Solid 1/2px（parse 已挡其它）。
fn paint_cell_diagonal(
    canvas: &mut Canvas,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    down: bool,
    edge: Edge,
) {
    if x1 <= x0 + 1 || y1 <= y0 + 1 {
        return;
    }
    let (x_a, y_a, x_b, y_b) = if down {
        (x0, y0, x1 - 1, y1 - 1)
    } else {
        (x0, y1 - 1, x1 - 1, y0)
    };
    paint_diag_segment(canvas, x_a, y_a, x_b, y_b, edge.color, edge.px);
}

fn paint_diag_segment(
    canvas: &mut Canvas,
    mut x0: i32,
    mut y0: i32,
    x1: i32,
    y1: i32,
    color: [u8; 3],
    thickness: i32,
) {
    // Bresenham；厚度 2 时在主线两侧各补半像素朝较小坐标（与正交偶数宽约定一致）。
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    loop {
        put_thick(canvas, x0, y0, color, thickness);
        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }
}

fn put_thick(canvas: &mut Canvas, x: i32, y: i32, color: [u8; 3], thickness: i32) {
    canvas.put(x, y, color);
    if thickness >= 2 {
        // 多出的一像素朝坐标较小侧（左/上），对齐正交 medium 约定。
        canvas.put(x - 1, y, color);
        canvas.put(x, y - 1, color);
    }
}

struct Stroke {
    px: i32,
    rank: i32,
    order: u32,
    kind: LineKind,
    vertical: bool,
    boundary: i32,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    color: [u8; 3],
}

enum Side {
    Left,
    Right,
    Top,
    Bottom,
}

/// 正上方那一格有底边时，竖线不占这一行的上边界。底边可以来自格子，也可以来自命中的公式差分。
fn above_has_bottom(
    sheet: &Element,
    styles: &Styles,
    cells: &HashMap<(u32, u32), CellRec>,
    session: &mut Option<FormulaSession>,
    col: u32,
    row: u32,
) -> Result<bool, Error> {
    if row == 0 {
        return Ok(false);
    }
    Ok(resolved_edges(sheet, styles, cells, session, col, row - 1)?
        .bottom
        .is_some())
}

/// 邻边实际存在时，线才向那一端伸出它的外侧像素。
fn end_pad(edge: Option<Edge>) -> i32 {
    edge.map(|edge| edge.px / 2).unwrap_or(0)
}

/// 线以边界像素为中线。偶数宽度多出的一像素朝坐标较小的一侧。
fn neighbor_bottom(
    sheet: &Element,
    styles: &Styles,
    cells: &HashMap<(u32, u32), CellRec>,
    session: &mut Option<FormulaSession>,
    col: u32,
    row: u32,
) -> Result<Option<Edge>, Error> {
    if col == 0 {
        return Ok(None);
    }
    Ok(resolved_edges(sheet, styles, cells, session, col - 1, row)?.bottom)
}

fn inset_for(edge: Option<Edge>, neighbor: Option<Edge>) -> i32 {
    let (Some(edge), Some(neighbor)) = (edge, neighbor) else {
        return 0;
    };
    // 双线的两条线各是 1 像素，让开时不按 3 像素算。
    let ours = if edge.kind == LineKind::Double {
        1
    } else {
        edge.px
    };
    if neighbor.px < ours {
        return 0;
    }
    let dashed = matches!(
        neighbor.kind,
        LineKind::Dashed
            | LineKind::Dotted
            | LineKind::Hair
            | LineKind::DashDot
            | LineKind::DashDotDot
            | LineKind::MediumDashed
            | LineKind::MediumDashDot
            | LineKind::MediumDashDotDot
            | LineKind::SlantDashDot
    );
    if neighbor.px == ours {
        if dashed {
            neighbor.px
        } else {
            0
        }
    } else if dashed {
        neighbor.px
    } else {
        neighbor.px / 2
    }
}

fn push_stroke(
    out: &mut Vec<Stroke>,
    order: &mut u32,
    edge: Option<Edge>,
    side: Side,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    pad0: i32,
    pad1: i32,
    inset0: i32,
    yield_start: bool,
) {
    let Some(edge) = edge else {
        return;
    };
    let neg = edge.px / 2;
    let pos = edge.px - neg - 1;
    let (along0, along1, boundary, vertical) = match side {
        Side::Left => (y0, y1, x0, true),
        Side::Right => (y0, y1, x1, true),
        Side::Top => (x0, x1, y0, false),
        Side::Bottom => (x0, x1, y1, false),
    };
    let across0 = boundary - neg;
    let across1 = boundary + pos + 1;
    // 1 像素上边，或上面那一格的底边，占住上边界。更粗的上边仍按 pad 向上伸出。
    let along_start = if vertical && yield_start && pad0 == 0 {
        along0 - pad0 + inset0 + 1
    } else {
        along0 - pad0 + inset0
    };
    let (x0, y0, x1, y1) = if vertical {
        (across0, along_start, across1, along1 + pad1 + 1)
    } else {
        (along_start, across0, along1 + pad1 + 1, across1)
    };
    // 右边缘后画，盖住相邻格子的左边缘。
    let rank = match side {
        Side::Right => 1,
        Side::Left | Side::Top | Side::Bottom => 0,
    };
    out.push(Stroke {
        px: edge.px,
        rank,
        order: *order,
        kind: edge.kind,
        vertical,
        boundary,
        x0,
        y0,
        x1,
        y1,
        color: edge.color,
    });
    *order += 1;
}

fn paint_line(canvas: &mut Canvas, stroke: &Stroke) {
    for y in stroke.y0..stroke.y1 {
        for x in stroke.x0..stroke.x1 {
            let (along, across) = if stroke.vertical { (y, x) } else { (x, y) };
            // 竖向 dashed 的空档比横线晚一个像素。
            let along = if stroke.vertical && stroke.kind == LineKind::Dashed {
                along - 1
            } else {
                along
            };
            if line_on(stroke.kind, along, across, stroke.boundary) {
                canvas.put(x, y, stroke.color);
            }
        }
    }
    // 2 像素虚线的边界那一行向终点再涂 1 像素。
    if !stroke.vertical && two_pixel_dash(stroke.kind) {
        let x = stroke.x1;
        let y = stroke.boundary;
        if (stroke.y0..stroke.y1).contains(&y) && line_on(stroke.kind, x, y, stroke.boundary) {
            canvas.put(x, y, stroke.color);
        }
    }
}

fn two_pixel_dash(kind: LineKind) -> bool {
    matches!(
        kind,
        LineKind::MediumDashed
            | LineKind::MediumDashDot
            | LineKind::MediumDashDotDot
            | LineKind::SlantDashDot
    )
}

/// 余数用整张图的坐标。横线看 x，竖线看 y。
fn line_on(kind: LineKind, along: i32, across: i32, boundary: i32) -> bool {
    let r4 = along.rem_euclid(4);
    let r12 = along.rem_euclid(12);
    let r18 = along.rem_euclid(18);
    let r24 = along.rem_euclid(24);
    match kind {
        LineKind::Solid => true,
        LineKind::Double => across != boundary,
        LineKind::Dashed => r4 != 3,
        LineKind::Dotted => r4 == 0 || r4 == 3,
        LineKind::Hair => along.rem_euclid(2) == 0,
        LineKind::DashDot | LineKind::MediumDashDot => {
            (0..=8).contains(&r18) || (12..=14).contains(&r18)
        }
        LineKind::DashDotDot | LineKind::MediumDashDotDot => {
            (0..=8).contains(&r24) || (12..=14).contains(&r24) || (18..=20).contains(&r24)
        }
        LineKind::MediumDashed => (0..=8).contains(&r12),
        LineKind::SlantDashDot => {
            if across == boundary {
                (0..=9).contains(&r18) || (12..=15).contains(&r18)
            } else {
                (1..=11).contains(&r18) || (13..=17).contains(&r18)
            }
        }
    }
}

fn paint_rules(
    canvas: &mut Canvas,
    sheet: &Element,
    cells: &HashMap<(u32, u32), CellRec>,
    xs: &Axis,
    ys: &Axis,
    c0: u32,
    r0: u32,
) -> Result<(), Error> {
    let mut rules = Vec::new();
    for block in sheet.children_by_local("conditionalFormatting") {
        let sqref = block.attr("sqref").unwrap_or("");
        let areas = parse_areas(sqref)?;
        for rule in block.children_by_local("cfRule") {
            let priority = rule
                .attr("priority")
                .and_then(|s| s.parse::<i32>().ok())
                .unwrap_or(i32::MAX);
            rules.push((priority, areas.clone(), rule));
        }
    }
    rules.sort_by_key(|(priority, _, _)| std::cmp::Reverse(*priority));
    for (_, areas, rule) in &rules {
        match rule.attr("type").unwrap_or("") {
            "colorScale" => {
                let _ = scale_stops(cells, areas, rule)?;
            }
            "cellIs" => {
                let _ = rule_numbers(rule)?;
            }
            "dataBar" | "expression" | "timePeriod" | "aboveAverage" | "top10" | "containsText"
            | "notContainsText" | "beginsWith" | "endsWith" | "duplicateValues"
            | "uniqueValues" => {}
            "containsBlanks" | "notContainsBlanks" => {
                let _ = blank_mode(rule, areas)?;
            }
            "containsErrors" | "notContainsErrors" => {
                let _ = error_mode(rule, areas)?;
            }
            // iconSet：选择语义见 `icon_set` 模块；无合法字形资产前整表仍 NI（不冒充出图）。
            "iconSet" => return Err(unsupported()),
            _ => return Err(unsupported()),
        }
    }
    for (_, areas, rule) in &rules {
        if rule.attr("type") == Some("dataBar") {
            paint_bar(canvas, sheet, cells, xs, ys, c0, r0, areas, rule)?;
        }
    }
    Ok(())
}

fn parse_areas(sqref: &str) -> Result<Vec<(CellRef, CellRef)>, Error> {
    let mut out = Vec::new();
    for token in sqref.split_whitespace() {
        if token.is_empty() {
            continue;
        }
        if let Ok((a, b)) = a1::parse_range(token) {
            out.push((a, b));
            continue;
        }
        let cell = a1::parse_cell(token).map_err(|_| unsupported())?;
        out.push((cell, cell));
    }
    Ok(out)
}

fn in_areas(col: u32, row: u32, areas: &[(CellRef, CellRef)]) -> bool {
    areas.iter().any(|(a, b)| {
        col >= a.col.min(b.col)
            && col <= a.col.max(b.col)
            && row >= a.row.min(b.row)
            && row <= a.row.max(b.row)
    })
}

fn area_numbers(cells: &HashMap<(u32, u32), CellRec>, areas: &[(CellRef, CellRef)]) -> Vec<f64> {
    let mut out = Vec::new();
    for ((col, row), cell) in cells {
        if in_areas(*col, *row, areas) {
            if let Shown::Num(n) = cell.shown {
                out.push(n);
            }
        }
    }
    out
}

fn expression_gaps(
    sheet: &Element,
    c0: u32,
    r0: u32,
    c1: u32,
    r1: u32,
) -> Result<Vec<(u32, u32)>, Error> {
    let mut out = Vec::new();
    for block in sheet.children_by_local("conditionalFormatting") {
        let areas = parse_areas(block.attr("sqref").unwrap_or(""))?;
        let mut paints_blank = false;
        for rule in block.children_by_local("cfRule") {
            match rule.attr("type") {
                Some("expression") | Some("timePeriod") | Some("notContainsText") => {
                    paints_blank = true
                }
                Some("containsBlanks") | Some("notContainsBlanks") => {
                    paints_blank |= blank_mode(rule, &areas)? == Some(true);
                }
                Some("notContainsErrors") => {
                    paints_blank |= error_mode(rule, &areas)? == Some(ErrorPaint::ExceptErrors);
                }
                Some("containsErrors") => {
                    let _ = error_mode(rule, &areas)?;
                }
                _ => {}
            }
        }
        if !paints_blank {
            continue;
        }
        for row in r0..=r1 {
            for col in c0..=c1 {
                if in_areas(col, row, &areas) {
                    out.push((col, row));
                }
            }
        }
    }
    Ok(out)
}

fn edges_present(edges: Edges) -> bool {
    edges.left.is_some()
        || edges.right.is_some()
        || edges.top.is_some()
        || edges.bottom.is_some()
        || edges.diagonal_down.is_some()
        || edges.diagonal_up.is_some()
}

fn overlay_edges(base: Edges, over: Edges) -> Edges {
    Edges {
        left: over.left.or(base.left),
        right: over.right.or(base.right),
        top: over.top.or(base.top),
        bottom: over.bottom.or(base.bottom),
        diagonal_down: over.diagonal_down.or(base.diagonal_down),
        diagonal_up: over.diagonal_up.or(base.diagonal_up),
    }
}

/// 格子自己的边，再盖上命中的公式差分。没写的一侧留着。
fn resolved_edges(
    sheet: &Element,
    styles: &Styles,
    cells: &HashMap<(u32, u32), CellRec>,
    session: &mut Option<FormulaSession>,
    col: u32,
    row: u32,
) -> Result<Edges, Error> {
    let mut edges = match cells.get(&(col, row)) {
        Some(cell) => styles.xf(cell.style)?.edges,
        None => Edges::default(),
    };
    if let Some(over) = expression_border(sheet, styles, session, col, row)? {
        edges = overlay_edges(edges, over);
    }
    Ok(edges)
}

/// 公式以外的规则如果带了边框，整表仍不做。
fn reject_other_dxf_borders(sheet: &Element, styles: &Styles) -> Result<(), Error> {
    for block in sheet.children_by_local("conditionalFormatting") {
        for rule in block.children_by_local("cfRule") {
            let Some(id) = rule.attr("dxfId").and_then(|s| s.parse::<usize>().ok()) else {
                continue;
            };
            let Some(dxf) = styles.dxfs.get(id) else {
                return Err(unsupported());
            };
            if !edges_present(dxf.edges) {
                continue;
            }
            if rule.attr("type") != Some("expression") {
                return Err(unsupported());
            }
        }
    }
    Ok(())
}

/// 优先级数字最小的那条命中规则提供整份差分边框，不和另一条拼边。
fn expression_border(
    sheet: &Element,
    styles: &Styles,
    session: &mut Option<FormulaSession>,
    col: u32,
    row: u32,
) -> Result<Option<Edges>, Error> {
    let mut best: Option<(i32, Edges)> = None;
    for block in sheet.children_by_local("conditionalFormatting") {
        let areas = parse_areas(block.attr("sqref").unwrap_or(""))?;
        if !in_areas(col, row, &areas) {
            continue;
        }
        for rule in block.children_by_local("cfRule") {
            if rule.attr("type") != Some("expression") {
                continue;
            }
            let Some(id) = rule.attr("dxfId").and_then(|s| s.parse::<usize>().ok()) else {
                continue;
            };
            let Some(dxf) = styles.dxfs.get(id) else {
                return Err(unsupported());
            };
            if !edges_present(dxf.edges) {
                continue;
            }
            if !expression_hit(session, rule, &areas, col, row)? {
                continue;
            }
            let priority = rule
                .attr("priority")
                .and_then(|s| s.parse().ok())
                .unwrap_or(i32::MAX);
            if best.is_none_or(|(p, _)| priority < p) {
                best = Some((priority, dxf.edges));
            }
        }
    }
    Ok(best.map(|(_, edges)| edges))
}

fn expression_fill(
    styles: &Styles,
    session: &mut Option<FormulaSession>,
    rule: &Element,
    areas: &[(CellRef, CellRef)],
    col: u32,
    row: u32,
) -> Result<Option<[u8; 3]>, Error> {
    if !expression_hit(session, rule, areas, col, row)? {
        return Ok(None);
    }
    let Some(id) = rule.attr("dxfId").and_then(|s| s.parse::<usize>().ok()) else {
        return Ok(None);
    };
    let Some(dxf) = styles.dxfs.get(id) else {
        return Err(unsupported());
    };
    Ok(dxf.fill)
}

fn expression_hit(
    session: &mut Option<FormulaSession>,
    rule: &Element,
    areas: &[(CellRef, CellRef)],
    col: u32,
    row: u32,
) -> Result<bool, Error> {
    let text = rule
        .children_by_local("formula")
        .into_iter()
        .next()
        .ok_or_else(unsupported)?
        .text_content();
    let (a, b) = areas.first().copied().ok_or_else(unsupported)?;
    let anchor = CellRef {
        col: a.col.min(b.col),
        row: a.row.min(b.row),
    };
    let here = CellRef { col, row };
    let shifted = super::formula::shift_a1_formula(&text, anchor, here);
    let Some(session) = session.as_mut() else {
        return Err(unsupported());
    };
    match session.eval(&shifted, here) {
        FormulaOutcome::Bool(value) => Ok(value),
        FormulaOutcome::Num(n) if n.is_finite() => Ok(n != 0.0),
        FormulaOutcome::Num(_) | FormulaOutcome::Text | FormulaOutcome::Err => Ok(false),
        FormulaOutcome::Unknown => Err(unsupported()),
    }
}

fn cf_fill(
    sheet: &Element,
    styles: &Styles,
    cells: &HashMap<(u32, u32), CellRec>,
    col: u32,
    row: u32,
    session: &mut Option<FormulaSession>,
) -> Result<Option<[u8; 3]>, Error> {
    let mut best: Option<(i32, [u8; 3])> = None;
    for block in sheet.children_by_local("conditionalFormatting") {
        let areas = parse_areas(block.attr("sqref").unwrap_or(""))?;
        if !in_areas(col, row, &areas) {
            continue;
        }
        for rule in block.children_by_local("cfRule") {
            let priority = rule
                .attr("priority")
                .and_then(|s| s.parse().ok())
                .unwrap_or(i32::MAX);
            let color = match rule.attr("type").unwrap_or("") {
                "dataBar" => None,
                "colorScale" => scale_of(cells, &areas, rule, col, row)?,
                "cellIs" => cell_is_fill(styles, cells, rule, col, row)?,
                "expression" | "timePeriod" => {
                    expression_fill(styles, session, rule, &areas, col, row)?
                }
                "aboveAverage" | "top10" | "containsText" | "notContainsText" | "beginsWith"
                | "endsWith" => {
                    if simple_hit(cells, rule, &areas, col, row)? {
                        dxf_fill(styles, rule)?
                    } else {
                        None
                    }
                }
                "duplicateValues" | "uniqueValues" => {
                    if value_hit(
                        cells,
                        &areas,
                        col,
                        row,
                        rule.attr("type") == Some("uniqueValues"),
                    )? {
                        dxf_fill(styles, rule)?
                    } else {
                        None
                    }
                }
                "containsBlanks" | "notContainsBlanks" => match blank_mode(rule, &areas)? {
                    Some(want_blank) if blank_cell_hit(cells, col, row, want_blank) => {
                        dxf_fill(styles, rule)?
                    }
                    Some(_) | None => None,
                },
                "containsErrors" | "notContainsErrors" => match error_mode(rule, &areas)? {
                    Some(paint) if error_cell_hit(cells, col, row, paint) => {
                        dxf_fill(styles, rule)?
                    }
                    Some(_) | None => None,
                },
                _ => return Err(unsupported()),
            };
            if let Some(color) = color {
                if best.is_none_or(|(p, _)| priority < p) {
                    best = Some((priority, color));
                }
            }
        }
    }
    Ok(best.map(|(_, color)| color))
}

fn scale_stops(
    cells: &HashMap<(u32, u32), CellRec>,
    areas: &[(CellRef, CellRef)],
    rule: &Element,
) -> Result<Vec<(f64, [u8; 3])>, Error> {
    let Some(scale) = rule.child_by_local("colorScale") else {
        return Err(unsupported());
    };
    let cfvos = scale.children_by_local("cfvo");
    let colors = scale.children_by_local("color");
    if cfvos.len() != colors.len() || !(2..=3).contains(&cfvos.len()) {
        return Err(unsupported());
    }
    let values = area_numbers(cells, areas);
    if values.is_empty() {
        return Ok(Vec::new());
    }
    let mut stops = Vec::new();
    for (cfvo, color) in cfvos.iter().zip(colors.iter()) {
        let at = cfvo_value(cfvo, &values)?;
        let rgb = color
            .attr("rgb")
            .and_then(parse_rgb)
            .ok_or_else(unsupported)?;
        stops.push((at, rgb));
    }
    stops.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    Ok(stops)
}

fn scale_of(
    cells: &HashMap<(u32, u32), CellRec>,
    areas: &[(CellRef, CellRef)],
    rule: &Element,
    col: u32,
    row: u32,
) -> Result<Option<[u8; 3]>, Error> {
    let n = match cells.get(&(col, row)).map(|cell| &cell.shown) {
        Some(Shown::Num(n)) => *n,
        _ => return Ok(None),
    };
    let stops = scale_stops(cells, areas, rule)?;
    if stops.is_empty() {
        return Ok(None);
    }
    Ok(Some(scale_color(n, &stops)))
}

fn cfvo_value(el: &Element, values: &[f64]) -> Result<f64, Error> {
    match el.attr("type").unwrap_or("") {
        "min" => values
            .iter()
            .copied()
            .reduce(f64::min)
            .ok_or_else(unsupported),
        "max" => values
            .iter()
            .copied()
            .reduce(f64::max)
            .ok_or_else(unsupported),
        "num" => el
            .attr("val")
            .and_then(|s| s.parse().ok())
            .ok_or_else(unsupported),
        "percentile" => {
            let pct = el
                .attr("val")
                .and_then(|s| s.parse().ok())
                .ok_or_else(unsupported)?;
            Ok(percentile_inc(values, pct))
        }
        "percent" => {
            let pct = el
                .attr("val")
                .and_then(|s| s.parse().ok())
                .ok_or_else(unsupported)?;
            super::icon_set::percent_threshold(pct, values).ok_or_else(unsupported)
        }
        _ => Err(unsupported()),
    }
}

fn percentile_inc(values: &[f64], pct: f64) -> f64 {
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    if sorted.len() <= 1 {
        return sorted.first().copied().unwrap_or(0.0);
    }
    let p = (pct / 100.0).clamp(0.0, 1.0);
    let k = p * (sorted.len() - 1) as f64;
    let i = k.floor() as usize;
    let frac = k - i as f64;
    let j = (i + 1).min(sorted.len() - 1);
    sorted[i] * (1.0 - frac) + sorted[j] * frac
}

fn scale_color(value: f64, stops: &[(f64, [u8; 3])]) -> [u8; 3] {
    if stops.len() == 1 || value <= stops[0].0 {
        return stops[0].1;
    }
    let last = stops.len() - 1;
    if value >= stops[last].0 {
        return stops[last].1;
    }
    for pair in stops.windows(2) {
        if value <= pair[1].0 {
            let span = pair[1].0 - pair[0].0;
            let t = if span.abs() < 1e-12 {
                0.0
            } else {
                (value - pair[0].0) / span
            };
            return mix(pair[0].1, pair[1].1, t);
        }
    }
    stops[last].1
}

fn mix(a: [u8; 3], b: [u8; 3], t: f64) -> [u8; 3] {
    let t = t.clamp(0.0, 1.0);
    let mut out = [0u8; 3];
    for i in 0..3 {
        let v = f64::from(a[i]) + (f64::from(b[i]) - f64::from(a[i])) * t;
        out[i] = v.round().clamp(0.0, 255.0) as u8;
    }
    out
}

fn cell_is_fill(
    styles: &Styles,
    cells: &HashMap<(u32, u32), CellRec>,
    rule: &Element,
    col: u32,
    row: u32,
) -> Result<Option<[u8; 3]>, Error> {
    let Some(id) = rule.attr("dxfId").and_then(|s| s.parse::<usize>().ok()) else {
        return Ok(None);
    };
    let Some(dxf) = styles.dxfs.get(id) else {
        return Err(unsupported());
    };
    let Some(fill) = dxf.fill else {
        return Ok(None);
    };
    let n = match cells.get(&(col, row)).map(|cell| &cell.shown) {
        Some(Shown::Num(n)) => *n,
        _ => return Ok(None),
    };
    let args = rule_numbers(rule)?;
    if cell_is(rule.attr("operator").unwrap_or(""), n, &args)? {
        Ok(Some(fill))
    } else {
        Ok(None)
    }
}

fn rule_numbers(rule: &Element) -> Result<Vec<f64>, Error> {
    let mut out = Vec::new();
    for formula in rule.children_by_local("formula") {
        let text = formula.text_content();
        let n = text.trim().parse::<f64>().map_err(|_| unsupported())?;
        out.push(n);
    }
    Ok(out)
}

fn cell_is(op: &str, value: f64, args: &[f64]) -> Result<bool, Error> {
    match op {
        "lessThan" if args.len() == 1 => Ok(value < args[0]),
        "lessThanOrEqual" if args.len() == 1 => Ok(value <= args[0]),
        "greaterThan" if args.len() == 1 => Ok(value > args[0]),
        "greaterThanOrEqual" if args.len() == 1 => Ok(value >= args[0]),
        "equal" if args.len() == 1 => Ok((value - args[0]).abs() < 1e-9),
        "notEqual" if args.len() == 1 => Ok((value - args[0]).abs() >= 1e-9),
        "between" if args.len() == 2 => {
            Ok(value >= args[0].min(args[1]) && value <= args[0].max(args[1]))
        }
        "notBetween" if args.len() == 2 => {
            Ok(value < args[0].min(args[1]) || value > args[0].max(args[1]))
        }
        _ => Err(unsupported()),
    }
}

fn paint_bar(
    canvas: &mut Canvas,
    sheet: &Element,
    cells: &HashMap<(u32, u32), CellRec>,
    xs: &Axis,
    ys: &Axis,
    c0: u32,
    r0: u32,
    areas: &[(CellRef, CellRef)],
    rule: &Element,
) -> Result<(), Error> {
    let Some(bar) = rule.child_by_local("dataBar") else {
        return Err(unsupported());
    };
    let values = area_numbers(cells, areas);
    let color = bar
        .child_by_local("color")
        .and_then(|el| el.attr("rgb"))
        .and_then(parse_rgb)
        .ok_or_else(unsupported)?;
    // 主题色-only / 无 rgb：仍 NI（本刀不扩 theme 解析进数据条）。
    if let Some(x14) = x14_data_bar(sheet, rule) {
        return paint_bar_x14(
            canvas, cells, xs, ys, c0, r0, areas, bar, x14, &values, color,
        );
    }
    // 无 x14：只画非负 ECMA 渐变条（历史行为）。
    if values.iter().any(|n| *n < 0.0) {
        return Err(unsupported());
    }
    let cfvos = bar.children_by_local("cfvo");
    if cfvos.len() != 2 {
        return Err(unsupported());
    }
    let min = cfvo_value(cfvos[0], &values)?;
    let max = cfvo_value(cfvos[1], &values)?;
    for ((col, row), cell) in cells {
        let Shown::Num(n) = cell.shown else {
            continue;
        };
        if !in_areas(*col, *row, areas) {
            continue;
        }
        let Some((x0, y0, x1, y1)) = cell_rect(xs, ys, c0, r0, *col, *row) else {
            continue;
        };
        let width = x1 - x0;
        let height = y1 - y0;
        let usable = (width - 3).max(1);
        let t = if (max - min).abs() < 1e-12 {
            1.0
        } else {
            ((n - min) / (max - min)).clamp(0.0, 1.0)
        };
        // Excel 缺省最短 10%、最长 90%。列宽 103 时内宽是 100。
        let len = ((10.0 + 80.0 * t) / 100.0 * f64::from(usable)).round() as i32;
        let len = len.clamp(1, width - 2);
        let bx = x0 + 2;
        let by = y0 + 2;
        let bh = (height - 4).max(1);
        for dx in 0..len {
            let fade = if len <= 1 {
                0.0
            } else {
                f64::from(dx) / f64::from(len - 1) * 0.81
            };
            let rgb = mix(color, [255, 255, 255], fade);
            for dy in 0..bh {
                canvas.put(bx + dx, by + dy, rgb);
            }
        }
    }
    Ok(())
}

/// 工作表 `extLst` / `{78C0D931-…}` 里按 cfRule `x14:id` 找 `x14:dataBar`。
fn x14_data_bar<'a>(sheet: &'a Element, rule: &Element) -> Option<&'a Element> {
    let id = cf_rule_x14_id(rule)?;
    let ext = sheet.child_by_local("extLst")?;
    for node in &ext.children {
        let Node::Element(el) = node else { continue };
        if el.local_name() != "ext" {
            continue;
        }
        let uri = el.attr("uri").unwrap_or("");
        if !uri.eq_ignore_ascii_case("{78C0D931-6437-407D-A8EE-F0AAD7539E65}") {
            continue;
        }
        let Some(list) = el.child_by_local("conditionalFormattings") else {
            continue;
        };
        for block in list.children_by_local("conditionalFormatting") {
            for cf in block.children_by_local("cfRule") {
                if cf.attr("type") != Some("dataBar") {
                    continue;
                }
                if cf.attr("id").map(str::trim) != Some(id.as_str()) {
                    continue;
                }
                if let Some(bar) = cf.child_by_local("dataBar") {
                    return Some(bar);
                }
            }
        }
    }
    None
}

fn cf_rule_x14_id(rule: &Element) -> Option<String> {
    let ext_lst = rule.child_by_local("extLst")?;
    for node in &ext_lst.children {
        let Node::Element(ext) = node else { continue };
        if ext.local_name() != "ext" {
            continue;
        }
        let uri = ext.attr("uri").unwrap_or("");
        if !uri.eq_ignore_ascii_case("{B025F937-C7B1-47D3-B67F-A62EFF666E3E}") {
            continue;
        }
        let id_el = ext.child_by_local("id")?;
        let text = id_el.text_content();
        let text = text.trim();
        if text.is_empty() {
            return None;
        }
        return Some(text.to_string());
    }
    None
}

fn attr_is_zero_or_false(el: &Element, name: &str) -> bool {
    match el.attr(name).map(str::trim) {
        Some("0") => true,
        Some(s) if s.eq_ignore_ascii_case("false") => true,
        _ => false,
    }
}

fn attr_is_one_or_true(el: &Element, name: &str) -> bool {
    match el.attr(name).map(str::trim) {
        Some("1") => true,
        Some(s) if s.eq_ignore_ascii_case("true") => true,
        _ => false,
    }
}

/// x14 实心 + `axisPosition=middle`：正右负左；`gradient=0`；`minLength=0`/`maxLength=100`。
/// 负色：`negativeBarColorSameAsPositive=0` 时用 `negativeFillColor` rgb，否则用正色。
/// 轴色：`axisColor` rgb。其它 axis/cfvo/border/direction/theme NI。
fn paint_bar_x14(
    canvas: &mut Canvas,
    cells: &HashMap<(u32, u32), CellRec>,
    xs: &Axis,
    ys: &Axis,
    c0: u32,
    r0: u32,
    areas: &[(CellRef, CellRef)],
    ecma_bar: &Element,
    x14: &Element,
    values: &[f64],
    pos_color: [u8; 3],
) -> Result<(), Error> {
    if !attr_is_zero_or_false(x14, "gradient") {
        // 渐变 x14：有负数则 NI；全非负时退回 ECMA 渐变路径由调用方…此处直接 NI 以免混 PPT/单元格渐变。
        // 全非负且无特殊轴时，仍可用 ECMA 子树重画：
        if values.iter().any(|n| *n < 0.0) {
            return Err(unsupported());
        }
        let cfvos = ecma_bar.children_by_local("cfvo");
        if cfvos.len() != 2 {
            return Err(unsupported());
        }
        let min = cfvo_value(cfvos[0], values)?;
        let max = cfvo_value(cfvos[1], values)?;
        return paint_bar_ecma_gradient(canvas, cells, xs, ys, c0, r0, areas, min, max, pos_color);
    }
    match x14.attr("axisPosition").map(str::trim) {
        Some("middle") => {}
        _ => return Err(unsupported()),
    }
    let min_len = x14.attr("minLength").unwrap_or("0").trim();
    let max_len = x14.attr("maxLength").unwrap_or("100").trim();
    if min_len != "0" || max_len != "100" {
        return Err(unsupported());
    }
    if attr_is_one_or_true(x14, "border") {
        return Err(unsupported());
    }
    if let Some(dir) = x14.attr("direction").map(str::trim) {
        if !dir.is_empty() && !dir.eq_ignore_ascii_case("leftToRight") {
            return Err(unsupported());
        }
    }
    // MS：有 x14 cfvo 时应忽略 ECMA cfvo。本刀只认 min/max。
    let cfvos = {
        let x = x14.children_by_local("cfvo");
        if x.len() == 2 {
            x
        } else {
            let e = ecma_bar.children_by_local("cfvo");
            if e.len() != 2 {
                return Err(unsupported());
            }
            e
        }
    };
    for cfvo in &cfvos {
        match cfvo.attr("type").map(str::trim) {
            Some("min") | Some("max") => {}
            _ => return Err(unsupported()),
        }
    }
    let _min = cfvo_value(cfvos[0], values)?;
    let _max = cfvo_value(cfvos[1], values)?;
    let extent = values.iter().map(|n| n.abs()).fold(0.0_f64, f64::max);
    // MS：negativeFillColor 仅当 negativeBarColorSameAsPositive=false 时存在。
    // Excel 有时省略该属性但仍写 negativeFillColor——有该元素则用负色。
    let neg_same = match x14.attr("negativeBarColorSameAsPositive").map(str::trim) {
        Some("0") => false,
        Some(s) if s.eq_ignore_ascii_case("false") => false,
        Some("1") => true,
        Some(s) if s.eq_ignore_ascii_case("true") => true,
        None => x14.child_by_local("negativeFillColor").is_none(),
        Some(_) => return Err(unsupported()),
    };
    let neg_color = if neg_same {
        pos_color
    } else {
        x14.child_by_local("negativeFillColor")
            .and_then(|el| el.attr("rgb"))
            .and_then(parse_rgb)
            .ok_or_else(unsupported)?
    };
    let axis_color = x14
        .child_by_local("axisColor")
        .and_then(|el| el.attr("rgb"))
        .and_then(parse_rgb)
        .unwrap_or([0, 0, 0]);

    for ((col, row), cell) in cells {
        let Shown::Num(n) = cell.shown else {
            continue;
        };
        if !in_areas(*col, *row, areas) {
            continue;
        }
        let Some((x0, y0, x1, y1)) = cell_rect(xs, ys, c0, r0, *col, *row) else {
            continue;
        };
        let width = x1 - x0;
        let height = y1 - y0;
        if width < 6 || height < 5 {
            continue;
        }
        let by = y0 + 2;
        let bh = (height - 4).max(1);
        let axis = x0 + width / 2;
        let left_max = (axis - (x0 + 2)).max(0);
        let right_max = ((x1 - 2) - axis).max(0);
        let half = left_max.max(right_max);
        // 轴线
        for dy in 0..bh {
            canvas.put(axis, by + dy, axis_color);
        }
        if n == 0.0 || extent < 1e-12 || half <= 0 {
            continue;
        }
        let len = ((n.abs() / extent) * f64::from(half)).round() as i32;
        let len = len.clamp(0, half);
        if len <= 0 {
            continue;
        }
        if n > 0.0 {
            let max_len = right_max.min(half);
            let len = len.min(max_len);
            for dx in 1..=len {
                for dy in 0..bh {
                    canvas.put(axis + dx, by + dy, pos_color);
                }
            }
        } else {
            let max_len = left_max.min(half);
            let len = len.min(max_len);
            for dx in 1..=len {
                for dy in 0..bh {
                    canvas.put(axis - dx, by + dy, neg_color);
                }
            }
        }
    }
    Ok(())
}

fn paint_bar_ecma_gradient(
    canvas: &mut Canvas,
    cells: &HashMap<(u32, u32), CellRec>,
    xs: &Axis,
    ys: &Axis,
    c0: u32,
    r0: u32,
    areas: &[(CellRef, CellRef)],
    min: f64,
    max: f64,
    color: [u8; 3],
) -> Result<(), Error> {
    for ((col, row), cell) in cells {
        let Shown::Num(n) = cell.shown else {
            continue;
        };
        if !in_areas(*col, *row, areas) {
            continue;
        }
        let Some((x0, y0, x1, y1)) = cell_rect(xs, ys, c0, r0, *col, *row) else {
            continue;
        };
        let width = x1 - x0;
        let height = y1 - y0;
        let usable = (width - 3).max(1);
        let t = if (max - min).abs() < 1e-12 {
            1.0
        } else {
            ((n - min) / (max - min)).clamp(0.0, 1.0)
        };
        let len = ((10.0 + 80.0 * t) / 100.0 * f64::from(usable)).round() as i32;
        let len = len.clamp(1, width - 2);
        let bx = x0 + 2;
        let by = y0 + 2;
        let bh = (height - 4).max(1);
        for dx in 0..len {
            let fade = if len <= 1 {
                0.0
            } else {
                f64::from(dx) / f64::from(len - 1) * 0.81
            };
            let rgb = mix(color, [255, 255, 255], fade);
            for dy in 0..bh {
                canvas.put(bx + dx, by + dy, rgb);
            }
        }
    }
    Ok(())
}

fn cell_rect(
    xs: &Axis,
    ys: &Axis,
    c0: u32,
    r0: u32,
    col: u32,
    row: u32,
) -> Option<(i32, i32, i32, i32)> {
    let cx = col.checked_sub(c0)? as usize;
    let ry = row.checked_sub(r0)? as usize;
    if cx + 1 >= xs.origin.len() || ry + 1 >= ys.origin.len() {
        return None;
    }
    let x0 = xs.origin[cx];
    let x1 = xs.origin[cx + 1];
    let y0 = ys.origin[ry];
    let y1 = ys.origin[ry + 1];
    if x1 == x0 || y1 == y0 {
        return None;
    }
    Some((x0, y0, x1, y1))
}

#[derive(Default)]
struct FontBook {
    faces: Vec<(String, i32, bool, bool, Face)>,
}

impl FontBook {
    fn face(&mut self, name: &str, pt: f64, bold: bool, italic: bool) -> Result<&Face, Error> {
        let key = (pt * 100.0).round() as i32;
        if let Some(index) = self
            .faces
            .iter()
            .position(|(n, k, b, i, _)| n == name && *k == key && *b == bold && *i == italic)
        {
            return Ok(&self.faces[index].4);
        }
        let px = (pt * 96.0 / 72.0) as f32;
        let face = Face::open_cell(px, bold, italic, name).ok_or_else(unsupported)?;
        self.faces.push((name.to_string(), key, bold, italic, face));
        Ok(&self.faces.last().expect("刚放进去").4)
    }
}

fn paint_labels(
    canvas: &mut Canvas,
    fonts: &mut FontBook,
    styles: &Styles,
    cells: &HashMap<(u32, u32), CellRec>,
    merges: &[Merge],
    covered: &HashSet<(u32, u32)>,
    xs: &Axis,
    ys: &Axis,
    c0: u32,
    r0: u32,
    sheet: &Element,
    rtl: bool,
    session: &mut Option<FormulaSession>,
) -> Result<(), Error> {
    let filter = filter_header(sheet)?;
    let mut jobs = Vec::new();
    for merge in merges {
        jobs.push((merge.c0, merge.r0, merge.c1, merge.r1));
    }
    let c1 = c0 + xs.origin.len() as u32 - 2;
    let r1 = r0 + ys.origin.len() as u32 - 2;
    for (col, row) in cells.keys() {
        if *col < c0 || *row < r0 || *col > c1 || *row > r1 {
            continue;
        }
        if covered.contains(&(*col, *row)) {
            continue;
        }
        if merges.iter().any(|m| m.c0 == *col && m.r0 == *row) {
            continue;
        }
        jobs.push((*col, *row, *col, *row));
    }
    for (col, row, col_end, row_end) in jobs {
        let Some(cell) = cells.get(&(col, row)) else {
            continue;
        };
        let x0 = xs.origin[(col - c0) as usize];
        let x1 = xs.origin[(col_end - c0) as usize + 1];
        let y0 = ys.origin[(row - r0) as usize];
        let y1 = ys.origin[(row_end - r0) as usize + 1];
        if x1 == x0 || y1 == y0 {
            continue;
        }
        let xf = styles.xf(cell.style)?;
        let font = styles.font(cell.style)?;
        let fmt = styles.fmt_of(cell.style)?;
        let date1904 = session.as_ref().map(|s| s.date1904()).unwrap_or(false);
        let text = label_text(&cell.shown, &fmt, date1904)?;
        if text.is_empty() {
            continue;
        }
        // readingOrder=2 / 强 RTL 文种需要整段 bidi+shaping。
        // Face::width 与 draw_cells 是逐字 glyph advance，接 DirectionRightToLeft 会测画错配——NI。
        // 不把 Unicode 反序当 RTL；不与 sheetView.rightToLeft（列镜像）混同。
        if xf.reading_order == 2 || has_strong_rtl(&text) {
            return Err(unsupported());
        }
        let Some(color) = font.color else {
            return Err(unsupported());
        };
        let color = dxf_color(sheet, styles, cells, col, row, session)?.unwrap_or(color);
        // sheet RTL：调用方已翻转画布；格子 x 映到翻转后坐标，字仍 draw_cells 正写。
        let (x0, x1) = if rtl {
            (canvas.w - x1, canvas.w - x0)
        } else {
            (x0, x1)
        };
        let mut clip_r = x1;
        if filter_hit(filter, col, row, col_end) {
            clip_r = if rtl {
                (x0 + 17).min(x1)
            } else {
                clip_r.min(x1 - 17)
            };
        }
        let face = fonts.face(&font.name, font.pt, font.bold, font.italic)?;
        let h_align = match xf.h_align {
            HAlign::Left => HAlign::Left,
            HAlign::Center => HAlign::Center,
            HAlign::Right => HAlign::Right,
            HAlign::General => match cell.shown {
                Shown::Num(_) => HAlign::Right,
                Shown::Bool(_) => HAlign::Center,
                _ => HAlign::Left,
            },
        };
        // 缩进：每级 3×`0` 字宽；与 draw_cells 同一套 Face::width，不用 Word typo_width。
        let indent_px = if xf.indent == 0 {
            0.0
        } else {
            face.width("0") * 3.0 * f64::from(xf.indent)
        };
        let pad = face.cell_pad();
        let inner_left = f64::from(x0)
            + pad
            + match h_align {
                HAlign::Right => 0.0,
                _ => indent_px,
            };
        let inner_right = f64::from(clip_r)
            - 2.0
            - match h_align {
                HAlign::Right => indent_px,
                _ => 0.0,
            };
        let max_w = (inner_right - inner_left).max(1.0);
        let lines = if xf.wrap || text.contains('\n') {
            wrap_cell_lines(face, &text, max_w)
        } else {
            vec![text]
        };
        let em = face.em_box();
        let block_h = em * lines.len() as f64;
        let top0 = match xf.v_align {
            VAlign::Top => f64::from(y0) + 2.0,
            VAlign::Center => f64::from(y0) + (f64::from(y1 - y0) - block_h) / 2.0,
            VAlign::Bottom => f64::from(y1) - block_h - 1.0,
        };
        for (i, line) in lines.iter().enumerate() {
            if line.is_empty() {
                continue;
            }
            let width = face.width(line);
            let x = match h_align {
                HAlign::Left | HAlign::General => inner_left,
                HAlign::Center => {
                    let span = (inner_right - inner_left).max(width);
                    inner_left + (span - width) / 2.0
                }
                HAlign::Right => inner_right - width,
            };
            let top = top0 + em * i as f64;
            face.draw_cells(line, x, top, color, |px, py, rgb, cov| {
                if px < x0 || py < y0 || px >= clip_r || py >= y1 {
                    return;
                }
                canvas.blend(px, py, rgb, cov);
            });
        }
    }
    paint_filters(canvas, xs, ys, c0, r0, filter);
    Ok(())
}

/// Hebrew/Arabic strong RTL. sheetView RTL this slice: Latin/CJK upright only.
fn has_strong_rtl(text: &str) -> bool {
    text.chars().any(|ch| {
        let n = ch as u32;
        (0x0590..=0x05FF).contains(&n)
            || (0x0600..=0x06FF).contains(&n)
            || (0x0700..=0x074F).contains(&n)
            || (0x0750..=0x077F).contains(&n)
            || (0x08A0..=0x08FF).contains(&n)
            || (0xFB1D..=0xFDFF).contains(&n)
            || (0xFE70..=0xFEFF).contains(&n)
    })
}

/// 单元格换行：硬换行 `\n` 分段，再按空格贪心软断；超长无空格词中断。
/// 宽度一律 `Face::width`（与 `draw_cells` 同前进宽），不用 Word `typo_width`。
fn wrap_cell_lines(face: &Face, text: &str, max_width: f64) -> Vec<String> {
    let mut out = Vec::new();
    for para in text.split('\n') {
        if para.is_empty() {
            out.push(String::new());
            continue;
        }
        let chars: Vec<char> = para.chars().collect();
        let mut start = 0usize;
        let mut i = 0usize;
        let mut last_space: Option<usize> = None;
        while i < chars.len() {
            let line: String = chars[start..=i].iter().collect();
            if face.width(&line) <= max_width {
                if chars[i] == ' ' {
                    last_space = Some(i);
                }
                i += 1;
                continue;
            }
            if i == start {
                out.push(chars[i].to_string());
                i += 1;
                start = i;
                last_space = None;
                continue;
            }
            let break_at = match last_space {
                Some(s) => s + 1,
                None => i,
            };
            let piece: String = chars[start..break_at].iter().collect();
            out.push(piece);
            start = break_at;
            last_space = None;
            i = start;
        }
        if start < chars.len() {
            out.push(chars[start..].iter().collect());
        }
    }
    if out.is_empty() {
        out.push(String::new());
    }
    out
}

fn label_text(shown: &Shown, fmt: &NumFmt, date1904: bool) -> Result<String, Error> {
    let hide_text = matches!(fmt, NumFmt::Blank(4));
    match shown {
        Shown::Empty => Ok(String::new()),
        Shown::Text(text) if hide_text => Ok(String::new()),
        Shown::Text(text) => match fmt {
            NumFmt::Sections(sections) if sections.len() >= 4 => {
                let sec = &sections[3];
                if let Some(lit) = &sec.literal {
                    Ok(lit.clone())
                } else {
                    Ok(format!("{}{}{}", sec.prefix, text, sec.suffix))
                }
            }
            _ => Ok(text.clone()),
        },
        Shown::Bool(_) if hide_text => Ok(String::new()),
        Shown::Bool(true) => Ok("TRUE".into()),
        Shown::Bool(false) => Ok("FALSE".into()),
        Shown::Num(_) if matches!(fmt, NumFmt::Blank(_)) => Ok(String::new()),
        Shown::Num(n) => format_num(*n, fmt, date1904),
        Shown::Err(code) => Ok(code.clone()),
    }
}

fn format_num(n: f64, fmt: &NumFmt, date1904: bool) -> Result<String, Error> {
    if !n.is_finite() {
        return Err(unsupported());
    }
    Ok(match fmt {
        NumFmt::General => format_general(n),
        NumFmt::IntComma => format_comma(n),
        NumFmt::Fixed(digits) => format_fixed(n, *digits),
        NumFmt::Percent(digits) => format!("{}%", format_fixed(n * 100.0, *digits)),
        NumFmt::Blank(_) => String::new(),
        NumFmt::Sections(sections) => format_sections(n, sections)?,
        NumFmt::DateTime(pat) => format_datetime(n, *pat, date1904)?,
    })
}

const MONTH_MMM: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

fn excel_hms(serial: f64) -> Result<(u32, u32, u32), Error> {
    // 日小数部分 → 时分秒；与 Excel 显示一致按秒四舍五入。
    let frac = serial - serial.floor();
    let mut secs = (frac * 86_400.0).round() as i64;
    if secs < 0 {
        return Err(unsupported());
    }
    if secs >= 86_400 {
        secs = 0;
    }
    let h = (secs / 3600) as u32;
    let m = ((secs % 3600) / 60) as u32;
    let s = (secs % 60) as u32;
    Ok((h, m, s))
}

fn hour_12(h24: u32) -> (u32, &'static str) {
    let ampm = if h24 >= 12 { "PM" } else { "AM" };
    let h = match h24 % 12 {
        0 => 12,
        x => x,
    };
    (h, ampm)
}

fn format_datetime(n: f64, pat: DateTimePat, date1904: bool) -> Result<String, Error> {
    let (y, month, day) = super::eval::excel_ymd(n, date1904).ok_or_else(unsupported)?;
    if !(1..=12).contains(&month) {
        return Err(unsupported());
    }
    let (hh, mm, ss) = excel_hms(n)?;
    let yy = y.rem_euclid(100) as u32;
    let mmm = MONTH_MMM[(month as usize) - 1];
    Ok(match pat {
        DateTimePat::MdYy => format!("{month}/{day}/{yy:02}"),
        DateTimePat::DMmmYy => format!("{day}-{mmm}-{yy:02}"),
        DateTimePat::DMmm => format!("{day}-{mmm}"),
        DateTimePat::MmmYy => format!("{mmm}-{yy:02}"),
        DateTimePat::HmmAmpm => {
            let (h, ap) = hour_12(hh);
            format!("{h}:{mm:02} {ap}")
        }
        DateTimePat::HmmssAmpm => {
            let (h, ap) = hour_12(hh);
            format!("{h}:{mm:02}:{ss:02} {ap}")
        }
        DateTimePat::Hmm => format!("{hh}:{mm:02}"),
        DateTimePat::Hmmss => format!("{hh}:{mm:02}:{ss:02}"),
        DateTimePat::MdYyHmm => format!("{month}/{day}/{yy:02} {hh}:{mm:02}"),
        DateTimePat::YyyyMmDd => format!("{y:04}/{month:02}/{day:02}"),
    })
}

fn pick_section_index(n: f64, n_sections: usize) -> usize {
    // Excel：1 段全用；2 段正+零 / 负；3+ 段正 / 负 / 零。
    if n_sections >= 3 && n == 0.0 {
        2
    } else if n < 0.0 && n_sections >= 2 {
        1
    } else {
        0
    }
}

fn format_sections(n: f64, sections: &[NumSection]) -> Result<String, Error> {
    if sections.is_empty() || sections.len() > 4 {
        return Err(unsupported());
    }
    let idx = pick_section_index(n, sections.len());
    let sec = &sections[idx];
    if let Some(lit) = &sec.literal {
        return Ok(lit.clone());
    }
    let abs = n.abs();
    let body = if sec.thousands {
        format_comma_abs(abs, sec.digits)
    } else {
        format_fixed(abs, sec.digits)
    };
    let core = format!("{}{}{}", sec.prefix, body, sec.suffix);
    if n < 0.0 && idx == 0 {
        // 单段（负值仍走正段）时 Excel 在货币前加 `-`（如 `-$1,234.50`）。
        Ok(format!("-{core}"))
    } else {
        Ok(core)
    }
}

fn format_comma_abs(n: f64, digits: u32) -> String {
    let scale = 10f64.powi(digits as i32);
    let v = (n * scale).round() / scale;
    let v = if v == 0.0 { 0.0 } else { v };
    let int_part = v.trunc().abs() as u64;
    let mut rest = int_part;
    let mut parts = Vec::new();
    if rest == 0 {
        parts.push("0".to_string());
    }
    while rest > 0 {
        let chunk = rest % 1000;
        rest /= 1000;
        if rest > 0 {
            parts.push(format!("{chunk:03}"));
        } else {
            parts.push(chunk.to_string());
        }
    }
    parts.reverse();
    let head = parts.join(",");
    if digits == 0 {
        head
    } else {
        let frac = ((v.fract().abs() * scale).round() as u64) % (10u64.pow(digits));
        format!("{head}.{frac:0width$}", width = digits as usize)
    }
}

fn format_general(n: f64) -> String {
    if n.abs() < 1e15 && (n - n.round()).abs() < 1e-8 {
        let v = n.round() as i64;
        return v.to_string();
    }
    let raw = format!("{n:.10}");
    let trimmed = raw.trim_end_matches('0').trim_end_matches('.');
    if trimmed.is_empty() {
        "0".into()
    } else {
        trimmed.to_string()
    }
}

fn format_comma(n: f64) -> String {
    let v = n.round();
    let neg = v < 0.0;
    let mut rest = v.abs() as u64;
    if !v.is_finite() || v.abs() > u64::MAX as f64 {
        return format_general(n);
    }
    let mut parts = Vec::new();
    if rest == 0 {
        parts.push("0".to_string());
    }
    while rest > 0 {
        let chunk = rest % 1000;
        rest /= 1000;
        if rest > 0 {
            parts.push(format!("{chunk:03}"));
        } else {
            parts.push(chunk.to_string());
        }
    }
    parts.reverse();
    let body = parts.join(",");
    if neg {
        format!("-{body}")
    } else {
        body
    }
}

fn format_fixed(n: f64, digits: u32) -> String {
    let scale = 10f64.powi(digits as i32);
    let v = (n * scale).round() / scale;
    let v = if v == 0.0 { 0.0 } else { v };
    format!("{v:.prec$}", prec = digits as usize)
}

fn dxf_fill(styles: &Styles, rule: &Element) -> Result<Option<[u8; 3]>, Error> {
    let Some(id) = rule.attr("dxfId").and_then(|s| s.parse::<usize>().ok()) else {
        return Ok(None);
    };
    let Some(dxf) = styles.dxfs.get(id) else {
        return Err(unsupported());
    };
    Ok(dxf.fill)
}

fn area_nums(cells: &HashMap<(u32, u32), CellRec>, areas: &[(CellRef, CellRef)]) -> Vec<f64> {
    let mut out = Vec::new();
    for ((col, row), cell) in cells {
        if in_areas(*col, *row, areas) {
            if let Shown::Num(n) = cell.shown {
                out.push(n);
            }
        }
    }
    out
}

fn shown_text(shown: &Shown) -> String {
    match shown {
        Shown::Empty => String::new(),
        Shown::Text(text) => text.to_lowercase(),
        Shown::Num(n) => format_general(*n).to_lowercase(),
        Shown::Bool(true) => "true".into(),
        Shown::Bool(false) => "false".into(),
        Shown::Err(code) => code.clone(),
    }
}

fn cell_text(cells: &HashMap<(u32, u32), CellRec>, col: u32, row: u32) -> String {
    cells
        .get(&(col, row))
        .map(|cell| shown_text(&cell.shown))
        .unwrap_or_default()
}

#[derive(Clone, PartialEq, Eq, Hash)]
enum DupKey {
    Num(u64),
    Text(String),
    Bool(bool),
}

fn num_bits(n: f64) -> u64 {
    let n = if n == 0.0 { 0.0 } else { n };
    n.to_bits()
}

/// 重复值。没有值的格子不进比较。文字能看成数字时并进数字。
const TEXT_NUMBER_MARK: [u8; 3] = [0x00, 0x80, 0x00];

/// Excel 把存成文字的数字标上左上角绿三角。复制图片里这块是 6×6，盖住填充。
fn paint_text_number_marks(
    canvas: &mut Canvas,
    cells: &HashMap<(u32, u32), CellRec>,
    merges: &[Merge],
    covered: &HashSet<(u32, u32)>,
    xs: &Axis,
    ys: &Axis,
    c0: u32,
    r0: u32,
) -> Result<(), Error> {
    let mut jobs = Vec::new();
    for merge in merges {
        jobs.push((merge.c0, merge.r0, merge.c1, merge.r1));
    }
    let c1 = c0 + xs.origin.len() as u32 - 2;
    let r1 = r0 + ys.origin.len() as u32 - 2;
    for (col, row) in cells.keys() {
        if *col < c0 || *row < r0 || *col > c1 || *row > r1 || covered.contains(&(*col, *row)) {
            continue;
        }
        if merges
            .iter()
            .any(|merge| merge.c0 == *col && merge.r0 == *row)
        {
            continue;
        }
        jobs.push((*col, *row, *col, *row));
    }
    for (col, row, col_end, row_end) in jobs {
        let Some(cell) = cells.get(&(col, row)) else {
            continue;
        };
        let Shown::Text(text) = &cell.shown else {
            continue;
        };
        if !matches!(parse_dup_number(text), Ok(Some(_))) {
            continue;
        }
        let x0 = xs.origin[(col - c0) as usize];
        let y0 = ys.origin[(row - r0) as usize];
        let x1 = xs.origin[(col_end - c0) as usize + 1];
        let y1 = ys.origin[(row_end - r0) as usize + 1];
        for i in 0..6 {
            for k in 0..(6 - i) {
                let x = x0 + 1 + k;
                let y = y0 + 1 + i;
                if x < x1 && y < y1 {
                    canvas.put(x, y, TEXT_NUMBER_MARK);
                }
            }
        }
    }
    Ok(())
}

/// `Some(true)` 铺空白，`Some(false)` 铺非空白。没有公式时不铺。
fn blank_mode(rule: &Element, areas: &[(CellRef, CellRef)]) -> Result<Option<bool>, Error> {
    let formulas = rule.children_by_local("formula");
    if formulas.is_empty() {
        return Ok(None);
    }
    if formulas.len() != 1 {
        return Err(unsupported());
    }
    let (a, b) = areas.first().copied().ok_or_else(unsupported)?;
    let anchor = CellRef {
        col: a.col.min(b.col),
        row: a.row.min(b.row),
    };
    let want_blank = match rule.attr("type") {
        Some("containsBlanks") => true,
        Some("notContainsBlanks") => false,
        _ => return Err(unsupported()),
    };
    let text = formulas[0].text_content();
    if !canonical_blank_formula(text.trim(), anchor, want_blank) {
        return Err(unsupported());
    }
    Ok(Some(want_blank))
}

/// Excel 16.0 把空白规则存成锚点上的 `LEN(TRIM(锚点))`。
fn canonical_blank_formula(text: &str, anchor: CellRef, want_blank: bool) -> bool {
    let text = text.strip_prefix('=').unwrap_or(text);
    let Some(rest) = strip_ascii_prefix(text, "LEN(") else {
        return false;
    };
    let Some(rest) = strip_ascii_prefix(rest, "TRIM(") else {
        return false;
    };
    let Some((cell, rest)) = rest.split_once("))") else {
        return false;
    };
    if !cell.eq_ignore_ascii_case(&a1::format_cell(anchor.col, anchor.row)) {
        return false;
    }
    if want_blank {
        rest == "=0"
    } else {
        rest == ">0"
    }
}

fn strip_ascii_prefix<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
    if text.len() >= prefix.len() && text[..prefix.len()].eq_ignore_ascii_case(prefix) {
        Some(&text[prefix.len()..])
    } else {
        None
    }
}

/// 空白含没有格子、没有值，以及去掉首尾普通空格后为空的文字。错误值另判。
fn cell_is_blank(cells: &HashMap<(u32, u32), CellRec>, col: u32, row: u32) -> bool {
    match cells.get(&(col, row)).map(|cell| &cell.shown) {
        None | Some(Shown::Empty) => true,
        Some(Shown::Text(text)) => text.trim_matches(' ').is_empty(),
        Some(Shown::Num(_) | Shown::Bool(_) | Shown::Err(_)) => false,
    }
}

/// 错误值上 `LEN(TRIM)` 得到错误，空白和非空白都不铺。
fn blank_cell_hit(
    cells: &HashMap<(u32, u32), CellRec>,
    col: u32,
    row: u32,
    want_blank: bool,
) -> bool {
    if matches!(
        cells.get(&(col, row)).map(|cell| &cell.shown),
        Some(Shown::Err(_))
    ) {
        return false;
    }
    cell_is_blank(cells, col, row) == want_blank
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ErrorPaint {
    OnlyErrors,
    ExceptErrors,
}

/// `Some` 表示公式能铺。没有公式时不铺。
fn error_mode(rule: &Element, areas: &[(CellRef, CellRef)]) -> Result<Option<ErrorPaint>, Error> {
    let formulas = rule.children_by_local("formula");
    if formulas.is_empty() {
        return Ok(None);
    }
    if formulas.len() != 1 {
        return Err(unsupported());
    }
    let (a, b) = areas.first().copied().ok_or_else(unsupported)?;
    let anchor = CellRef {
        col: a.col.min(b.col),
        row: a.row.min(b.row),
    };
    let paint = match rule.attr("type") {
        Some("containsErrors") => ErrorPaint::OnlyErrors,
        Some("notContainsErrors") => ErrorPaint::ExceptErrors,
        _ => return Err(unsupported()),
    };
    if !canonical_error_formula(formulas[0].text_content().trim(), anchor, paint) {
        return Err(unsupported());
    }
    Ok(Some(paint))
}

/// Excel 16.0 把错误规则存成锚点上的 `ISERROR` 或 `NOT(ISERROR)`。
fn canonical_error_formula(text: &str, anchor: CellRef, paint: ErrorPaint) -> bool {
    let text = text.strip_prefix('=').unwrap_or(text);
    let anchor_text = a1::format_cell(anchor.col, anchor.row);
    let (body, closing) = match paint {
        ErrorPaint::OnlyErrors => ("ISERROR(", ")"),
        ErrorPaint::ExceptErrors => ("NOT(ISERROR(", "))"),
    };
    let Some(rest) = strip_ascii_prefix(text, body) else {
        return false;
    };
    let Some((cell, rest)) = rest.split_once(closing) else {
        return false;
    };
    cell.eq_ignore_ascii_case(&anchor_text) && rest.is_empty()
}

fn cell_is_error(cells: &HashMap<(u32, u32), CellRec>, col: u32, row: u32) -> bool {
    matches!(
        cells.get(&(col, row)).map(|cell| &cell.shown),
        Some(Shown::Err(_))
    )
}

fn error_cell_hit(
    cells: &HashMap<(u32, u32), CellRec>,
    col: u32,
    row: u32,
    paint: ErrorPaint,
) -> bool {
    let error = cell_is_error(cells, col, row);
    match paint {
        ErrorPaint::OnlyErrors => error,
        ErrorPaint::ExceptErrors => !error,
    }
}

fn value_hit(
    cells: &HashMap<(u32, u32), CellRec>,
    areas: &[(CellRef, CellRef)],
    col: u32,
    row: u32,
    unique: bool,
) -> Result<bool, Error> {
    let mut counts: HashMap<DupKey, usize> = HashMap::new();
    let mut here = None;
    for ((c, r), cell) in cells {
        if !in_areas(*c, *r, areas) {
            continue;
        }
        let Some(key) = duplicate_key(cell)? else {
            continue;
        };
        if *c == col && *r == row {
            here = Some(key.clone());
        }
        *counts.entry(key).or_default() += 1;
    }
    Ok(here.is_some_and(|key| {
        let n = counts.get(&key).copied().unwrap_or(0);
        if unique {
            n == 1
        } else {
            n >= 2
        }
    }))
}

fn duplicate_key(cell: &CellRec) -> Result<Option<DupKey>, Error> {
    match &cell.shown {
        Shown::Empty => Ok(None),
        Shown::Bool(value) => Ok(Some(DupKey::Bool(*value))),
        Shown::Num(n) => Ok(Some(DupKey::Num(num_bits(*n)))),
        Shown::Text(text) => {
            if let Some(n) = parse_dup_number(text)? {
                Ok(Some(DupKey::Num(num_bits(n))))
            } else {
                Ok(Some(DupKey::Text(text.to_lowercase())))
            }
        }
        Shown::Err(_) => Err(unsupported()),
    }
}

/// 没有数字时返回 `None`，交给文字比较。有数字但不是这套写法时整表失败。
fn parse_dup_number(text: &str) -> Result<Option<f64>, Error> {
    if !text.chars().any(|ch| ch.is_ascii_digit()) {
        return Ok(None);
    }
    let mut body = text.trim();
    let mut neg = false;
    if let Some(inner) = body
        .strip_prefix('(')
        .and_then(|rest| rest.strip_suffix(')'))
    {
        neg = true;
        body = inner.trim();
    } else if let Some(rest) = body.strip_prefix('+') {
        body = rest.trim();
    } else if let Some(rest) = body.strip_prefix('-') {
        neg = true;
        body = rest.trim();
    }
    let mut percent = false;
    if let Some(rest) = body.strip_suffix('%') {
        percent = true;
        body = rest.trim_end();
    }
    let (mantissa, exp) = split_dup_exp(body)?;
    let mut value = parse_dup_mantissa(mantissa)? * 10f64.powi(exp);
    if percent {
        value /= 100.0;
    }
    if neg {
        value = -value;
    }
    if !value.is_finite() {
        return Err(unsupported());
    }
    Ok(Some(value))
}

fn split_dup_exp(text: &str) -> Result<(&str, i32), Error> {
    let Some(pos) = text.find(['e', 'E']) else {
        return Ok((text, 0));
    };
    let exp = &text[pos + 1..];
    if exp.is_empty() || exp.contains(['e', 'E']) {
        return Err(unsupported());
    }
    let exp = exp.parse::<i32>().map_err(|_| unsupported())?;
    Ok((&text[..pos], exp))
}

fn parse_dup_mantissa(text: &str) -> Result<f64, Error> {
    if text.is_empty() {
        return Err(unsupported());
    }
    let mut parts = text.split('.');
    let int_part = parts.next().unwrap_or("");
    let frac = parts.next();
    if parts.next().is_some() || int_part.is_empty() || frac.is_some_and(str::is_empty) {
        return Err(unsupported());
    }
    let digits = grouped_digits(int_part)?;
    if let Some(frac) = frac {
        if !frac.chars().all(|ch| ch.is_ascii_digit()) {
            return Err(unsupported());
        }
        format!("{digits}.{frac}")
            .parse()
            .map_err(|_| unsupported())
    } else {
        digits.parse().map_err(|_| unsupported())
    }
}

fn grouped_digits(text: &str) -> Result<String, Error> {
    if !text.contains(',') {
        if text.chars().all(|ch| ch.is_ascii_digit()) {
            return Ok(text.to_string());
        }
        return Err(unsupported());
    }
    let groups: Vec<&str> = text.split(',').collect();
    if groups.len() < 2
        || groups[0].is_empty()
        || groups[0].len() > 3
        || groups[1..]
            .iter()
            .any(|group| group.len() != 3 || !group.chars().all(|ch| ch.is_ascii_digit()))
        || !groups[0].chars().all(|ch| ch.is_ascii_digit())
    {
        return Err(unsupported());
    }
    Ok(groups.concat())
}

/// 高于平均、前 N 和文本规则。命中判断和检查结果用同一套。
fn simple_hit(
    cells: &HashMap<(u32, u32), CellRec>,
    rule: &Element,
    areas: &[(CellRef, CellRef)],
    col: u32,
    row: u32,
) -> Result<bool, Error> {
    Ok(match rule.attr("type").unwrap_or("") {
        "aboveAverage" => average_hit(cells, rule, areas, col, row)?,
        "top10" => top_hit(cells, rule, areas, col, row),
        "containsText" | "notContainsText" | "beginsWith" | "endsWith" => {
            text_hit(cells, rule, col, row)?
        }
        _ => false,
    })
}

fn average_hit(
    cells: &HashMap<(u32, u32), CellRec>,
    rule: &Element,
    areas: &[(CellRef, CellRef)],
    col: u32,
    row: u32,
) -> Result<bool, Error> {
    let nums = area_nums(cells, areas);
    let Some(Shown::Num(n)) = cells.get(&(col, row)).map(|cell| &cell.shown) else {
        return Ok(false);
    };
    if nums.is_empty() {
        return Ok(false);
    }
    let mean = nums.iter().sum::<f64>() / nums.len() as f64;
    let spread = match rule.attr("stdDev") {
        None => 0.0,
        Some(raw) => {
            if nums.len() < 2 {
                return Ok(false);
            }
            let k: f64 = raw.parse().map_err(|_| unsupported())?;
            let var = nums.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / nums.len() as f64;
            k * var.sqrt()
        }
    };
    let above = !matches!(
        rule.attr("aboveAverage"),
        Some("0") | Some("false") | Some("FALSE")
    );
    let eq = truthy(rule.attr("equalAverage"));
    let hit = if above {
        if eq {
            *n >= mean + spread
        } else {
            *n > mean + spread
        }
    } else if eq {
        *n <= mean - spread
    } else {
        *n < mean - spread
    };
    Ok(hit)
}

fn top_hit(
    cells: &HashMap<(u32, u32), CellRec>,
    rule: &Element,
    areas: &[(CellRef, CellRef)],
    col: u32,
    row: u32,
) -> bool {
    let nums = area_nums(cells, areas);
    let Some(Shown::Num(n)) = cells.get(&(col, row)).map(|cell| &cell.shown) else {
        return false;
    };
    if nums.is_empty() {
        return false;
    }
    let mut sorted = nums;
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let rank = rule
        .attr("rank")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(10)
        .max(1);
    let count = if truthy(rule.attr("percent")) {
        (sorted.len() * rank).div_ceil(100).max(1)
    } else {
        rank
    };
    let bottom = truthy(rule.attr("bottom"));
    let threshold = if bottom {
        sorted[count.saturating_sub(1).min(sorted.len() - 1)]
    } else {
        sorted[sorted.len().saturating_sub(count)]
    };
    if bottom {
        *n <= threshold
    } else {
        *n >= threshold
    }
}

fn text_hit(
    cells: &HashMap<(u32, u32), CellRec>,
    rule: &Element,
    col: u32,
    row: u32,
) -> Result<bool, Error> {
    if matches!(
        cells.get(&(col, row)).map(|cell| &cell.shown),
        Some(Shown::Err(_))
    ) {
        return Err(unsupported());
    }
    let needle = rule.attr("text").unwrap_or("").to_lowercase();
    let text = cell_text(cells, col, row);
    match rule.attr("type").unwrap_or("") {
        "containsText" => Ok(!needle.is_empty() && text.contains(&needle)),
        "notContainsText" => Ok(!text.contains(&needle)),
        "beginsWith" => Ok(text.starts_with(&needle)),
        "endsWith" => Ok(text.ends_with(&needle)),
        _ => Ok(false),
    }
}

fn dxf_color(
    sheet: &Element,
    styles: &Styles,
    cells: &HashMap<(u32, u32), CellRec>,
    col: u32,
    row: u32,
    session: &mut Option<FormulaSession>,
) -> Result<Option<[u8; 3]>, Error> {
    let n = match cells.get(&(col, row)).map(|cell| &cell.shown) {
        Some(Shown::Num(n)) => Some(*n),
        _ => None,
    };
    let mut hit = None;
    for block in sheet.children_by_local("conditionalFormatting") {
        let Ok(areas) = parse_areas(block.attr("sqref").unwrap_or("")) else {
            continue;
        };
        if !in_areas(col, row, &areas) {
            continue;
        }
        for rule in block.children_by_local("cfRule") {
            let applies = match rule.attr("type").unwrap_or("") {
                "cellIs" => {
                    let Some(n) = n else { continue };
                    let Ok(args) = rule_numbers(rule) else {
                        continue;
                    };
                    cell_is(rule.attr("operator").unwrap_or(""), n, &args).unwrap_or(false)
                }
                "expression" | "timePeriod" => expression_hit(session, rule, &areas, col, row)?,
                "aboveAverage" | "top10" | "containsText" | "notContainsText" | "beginsWith"
                | "endsWith" => simple_hit(cells, rule, &areas, col, row)?,
                "duplicateValues" | "uniqueValues" => value_hit(
                    cells,
                    &areas,
                    col,
                    row,
                    rule.attr("type") == Some("uniqueValues"),
                )?,
                "containsBlanks" | "notContainsBlanks" => match blank_mode(rule, &areas)? {
                    Some(want_blank) => blank_cell_hit(cells, col, row, want_blank),
                    None => false,
                },
                "containsErrors" | "notContainsErrors" => match error_mode(rule, &areas)? {
                    Some(paint) => error_cell_hit(cells, col, row, paint),
                    None => false,
                },
                _ => false,
            };
            if !applies {
                continue;
            }
            let Some(id) = rule.attr("dxfId").and_then(|s| s.parse::<usize>().ok()) else {
                continue;
            };
            if let Some(color) = styles.dxfs.get(id).and_then(|dxf| dxf.color) {
                hit = Some(color);
            }
        }
    }
    Ok(hit)
}

fn filter_header(sheet: &Element) -> Result<Option<(u32, u32, u32, u32)>, Error> {
    let Some(filter) = sheet.child_by_local("autoFilter") else {
        return Ok(None);
    };
    let Some(r) = filter.attr("ref") else {
        return Err(unsupported());
    };
    let (a, b) = a1::parse_range(r).map_err(|_| unsupported())?;
    Ok(Some((
        a.col.min(b.col),
        a.row.min(b.row),
        a.col.max(b.col),
        a.row.max(b.row),
    )))
}

fn filter_hit(filter: Option<(u32, u32, u32, u32)>, col: u32, row: u32, col_end: u32) -> bool {
    let Some((c0, r0, c1, _)) = filter else {
        return false;
    };
    row == r0 && col <= c1 && col_end >= c0
}

fn paint_filters(
    canvas: &mut Canvas,
    xs: &Axis,
    ys: &Axis,
    c0: u32,
    r0: u32,
    filter: Option<(u32, u32, u32, u32)>,
) {
    let Some((fc0, fr, fc1, _)) = filter else {
        return;
    };
    let cols = xs.origin.len() - 1;
    let rows = ys.origin.len() - 1;
    for col in fc0..=fc1 {
        if col < c0 || col >= c0 + cols as u32 || fr < r0 || fr >= r0 + rows as u32 {
            continue;
        }
        let x0 = xs.origin[(col - c0) as usize];
        let x1 = xs.origin[(col - c0) as usize + 1];
        let y0 = ys.origin[(fr - r0) as usize];
        let y1 = ys.origin[(fr - r0) as usize + 1];
        if x1 == x0 || y1 == y0 {
            continue;
        }
        paint_button(canvas, x1 - 17, y1 - 18);
    }
}

fn paint_button(canvas: &mut Canvas, x: i32, y: i32) {
    let border = [166, 172, 179];
    let arrow = [120, 122, 125];
    for dy in 0..17 {
        let t = f64::from(dy) / 16.0;
        let fill = mix([254, 255, 255], [239, 242, 247], t);
        for dx in 0..17 {
            let edge = dx == 0 || dy == 0 || dx == 16 || dy == 16;
            canvas.put(x + dx, y + dy, if edge { border } else { fill });
        }
    }
    // 17 像素按钮的正中是第 8 列。箭头尖端落在这一列。
    for (dy, span) in [(7, 5..=11), (8, 6..=10), (9, 7..=9), (10, 8..=8)] {
        for dx in span {
            canvas.put(x + dx, y + dy, arrow);
        }
    }
}

fn encode_png(w: i32, h: i32, rgb: &[u8]) -> Result<Vec<u8>, Error> {
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, w as u32, h as u32);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(|_| unsupported())?;
    writer.write_image_data(rgb).map_err(|_| unsupported())?;
    drop(writer);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sales_dashboard_canvas_matches_excel_copypicture() {
        let cols = [
            16.7109375, 14.7109375, 14.7109375, 12.7109375, 12.7109375, 12.7109375, 12.7109375,
            22.7109375,
        ];
        let mut width = 1;
        for col in cols {
            width += excel_col_px(col);
        }
        width += DEFAULT_COL_PX * 4;
        assert_eq!(width, 1095);
        assert_eq!(excel_col_px(16.7109375), 117);
        assert_eq!(excel_col_px(8.43), 59);
        let rows = [30.0, 15.0, 22.0, 15.0, 20.0, 32.0, 15.0, 24.0, 26.0];
        let mut height = 1;
        for pt in rows {
            height += excel_row_px(pt);
        }
        let data = excel_row_px(auto_row_pt("微软雅黑", 10.0));
        assert_eq!(data, 22);
        assert_eq!(excel_row_px(auto_row_pt("Calibri", 11.0)), 20);
        // 等线 11 = Normal 14.25pt → 19px；不得走雅黑 1.65×（否则 18.15→24，双行+框≈49）。
        assert_eq!(auto_row_pt("等线", 11.0), 14.25);
        assert_eq!(excel_row_px(auto_row_pt("等线", 11.0)), 19);
        assert_eq!(auto_row_pt("DengXian", 11.0), 14.25);
        // 非 11：不锁 COM 12→15.75 / 18→23，保持 pt*15/11，避免单例拟合。
        assert!((auto_row_pt("等线", 12.0) - 12.0 * 15.0 / 11.0).abs() < 1e-9);
        assert_eq!(auto_row_pt("Calibri", 11.0), 15.0);
        assert!((auto_row_pt("微软雅黑", 10.0) - 16.5).abs() < 1e-9);
        height += data * 7;
        assert_eq!(height, 418);
        assert_eq!(excel_row_px(20.0), 26);
        assert_eq!(excel_row_px(22.0), 29);
        assert_eq!(excel_row_px(32.0), 42);
        assert_eq!(excel_row_px(26.0), 34);
    }

    #[test]
    fn sheet_linear_fill_is_srgb_mix_not_ppt_gamma() {
        // OOXML 示例色 FF92D050→FF0070C0；Excel ExportAsFixedFormat PDF 条带 RMSE(sRGB)≈0.68 vs gamma2.2≈18.5。
        let c0 = [0x92, 0xd0, 0x50];
        let c1 = [0x00, 0x70, 0xc0];
        assert_eq!(mix(c0, c1, 0.0), c0);
        assert_eq!(mix(c0, c1, 1.0), c1);
        assert_eq!(mix(c0, c1, 0.5), [73, 160, 136]);
        let mut canvas = Canvas::new(5, 3).unwrap();
        canvas.fill_linear(0, 0, 4, 2, 0.0, c0, c1);
        let at = |c: &Canvas, x: i32, y: i32| {
            let i = (y * c.w + x) as usize * 3;
            [c.px[i], c.px[i + 1], c.px[i + 2]]
        };
        assert_eq!(at(&canvas, 0, 1), c0);
        assert_eq!(at(&canvas, 4, 1), c1);
        assert_eq!(at(&canvas, 2, 1), mix(c0, c1, 0.5));
        let mut canvas90 = Canvas::new(3, 5).unwrap();
        canvas90.fill_linear(0, 0, 2, 4, 90.0, c0, c1);
        assert_eq!(at(&canvas90, 1, 0), c0);
        assert_eq!(at(&canvas90, 1, 4), c1);
    }

    #[test]
    fn theme_one_is_black_and_tint_moves_toward_white() {
        let slots = default_theme();
        assert_eq!(slots[0], [255, 255, 255]);
        assert_eq!(slots[1], [0, 0, 0]);
        assert_eq!(apply_sheet_tint([0, 0, 0], 0.5), [128, 128, 128]);
        assert_eq!(apply_sheet_tint([255, 255, 255], -0.5), [128, 128, 128]);
        assert_eq!(
            apply_sheet_tint([0xEE, 0x44, 0x55], 0.399_975_585_192_419_2),
            [0xF4, 0x8E, 0x99]
        );
        assert_eq!(
            apply_sheet_tint([0xEE, 0x44, 0x55], -0.249_977_111_117_893),
            [0xD2, 0x13, 0x26]
        );
        assert_eq!(
            apply_sheet_tint([0xFF, 0xFF, 0xFF], -0.15),
            [0xD9, 0xD9, 0xD9]
        );
        assert_eq!(
            apply_sheet_tint([0xDA, 0xB6, 0xBA], 0.7999),
            [0xF8, 0xEF, 0xF0]
        );
        let palette = Palette {
            theme: slots,
            indexed: DEFAULT_INDEXED.iter().copied().map(rgb_of).collect(),
        };
        assert_eq!(palette.indexed_at(2), Some([255, 0, 0]));
        assert_eq!(palette.indexed_at(64), Some([0, 0, 0]));
        assert_eq!(palette.indexed_at(80), None);
    }

    #[test]
    fn currency_and_multi_section_match_excel_range_text() {
        // Excel 16 Range.Text oracle（本机 COM 实测）：
        // $#,##0.00 → $1,234.50 / -$1,234.50 / $0.00
        // $#,##0.00;($#,##0.00) → $1,234.50 / ($1,234.50)
        // $#,##0.00;($#,##0.00);"-" → 零段 "-"
        // 人读码与 OOXML 存盘转义码（styles.xml 实测）都要对上 Excel Range.Text。
        for code in ["$#,##0.00", r"\$#,##0.00"] {
            let currency = parse_format(code).unwrap();
            assert_eq!(
                label_text(&Shown::Num(1234.5), &currency, false).unwrap(),
                "$1,234.50"
            );
            assert_eq!(
                label_text(&Shown::Num(-1234.5), &currency, false).unwrap(),
                "-$1,234.50"
            );
            assert_eq!(
                label_text(&Shown::Num(0.0), &currency, false).unwrap(),
                "$0.00"
            );
        }

        for code in ["$#,##0.00;($#,##0.00)", r"\$#,##0.00;\(\$#,##0.00\)"] {
            let paren = parse_format(code).unwrap();
            assert_eq!(
                label_text(&Shown::Num(1234.5), &paren, false).unwrap(),
                "$1,234.50"
            );
            assert_eq!(
                label_text(&Shown::Num(-1234.5), &paren, false).unwrap(),
                "($1,234.50)"
            );
            assert_eq!(
                label_text(&Shown::Num(0.0), &paren, false).unwrap(),
                "$0.00"
            );
        }

        let zero_dash = parse_format("\\$#,##0.00;\\(\\$#,##0.00\\);\"-\"").unwrap();
        assert_eq!(
            label_text(&Shown::Num(0.0), &zero_dash, false).unwrap(),
            "-"
        );
        assert_eq!(
            label_text(&Shown::Num(-42.0), &zero_dash, false).unwrap(),
            "($42.00)"
        );

        // 会计跳宽 `_`：Excel Text 正数尾空格。
        let acct = parse_format(r"\$#,##0.00_);\(\$#,##0.00\)").unwrap();
        assert_eq!(
            label_text(&Shown::Num(1234.5), &acct, false).unwrap(),
            "$1,234.50 "
        );
        assert_eq!(
            label_text(&Shown::Num(-1234.5), &acct, false).unwrap(),
            "($1,234.50)"
        );

        // 未知方括号条件/颜色仍 NI；未知不静默当 General。
        assert!(parse_format("$#,##0.00;[Red]($#,##0.00)").is_err());
        assert!(parse_format("[>=100]0;0").is_err());
        assert!(parse_format("¥#,##0.00").is_err());
    }

    #[test]
    fn datetime_builtin_and_yyyy_mm_dd_match_excel_range_text() {
        // Excel 16 Range.Text oracle：serial 44941.6046875 = 2023-01-15 14:30:45（1900 系）
        let n = 44941.6046875;
        let cases = [
            (DateTimePat::MdYy, "1/15/23"),
            (DateTimePat::DMmmYy, "15-Jan-23"),
            (DateTimePat::DMmm, "15-Jan"),
            (DateTimePat::MmmYy, "Jan-23"),
            (DateTimePat::HmmAmpm, "2:30 PM"),
            (DateTimePat::HmmssAmpm, "2:30:45 PM"),
            (DateTimePat::Hmm, "14:30"),
            (DateTimePat::Hmmss, "14:30:45"),
            (DateTimePat::MdYyHmm, "1/15/23 14:30"),
            (DateTimePat::YyyyMmDd, "2023/01/15"),
        ];
        for (pat, want) in cases {
            let fmt = NumFmt::DateTime(pat);
            assert_eq!(
                label_text(&Shown::Num(n), &fmt, false).unwrap(),
                want,
                "{pat:?}"
            );
        }
        assert!(matches!(
            parse_format("yyyy/mm/dd").unwrap(),
            NumFmt::DateTime(DateTimePat::YyyyMmDd)
        ));
        assert!(matches!(
            builtin_fmt(14),
            Some(NumFmt::DateTime(DateTimePat::MdYy))
        ));
        assert!(matches!(
            builtin_fmt(22),
            Some(NumFmt::DateTime(DateTimePat::MdYyHmm))
        ));
        // 1900 假闰日
        assert_eq!(
            label_text(
                &Shown::Num(60.0),
                &NumFmt::DateTime(DateTimePat::YyyyMmDd),
                false
            )
            .unwrap(),
            "1900/02/29"
        );
        // 歧义/未点名自定义 NI；方括号仍 NI
        assert!(matches!(
            parse_format("m/d/yy").unwrap(),
            NumFmt::DateTime(DateTimePat::MdYy)
        ));
        assert!(matches!(
            parse_format(r"m/d/yy\ h:mm").unwrap(),
            NumFmt::DateTime(DateTimePat::MdYyHmm)
        ));
        assert!(parse_format("m/d/yyyy").is_err());
        assert!(parse_format("yyyy-mm-dd").is_err());
        assert!(parse_format("[$-409]yyyy/mm/dd").is_err());
        assert!(parse_format("$#,##0.00;[Red]($#,##0.00)").is_err());
    }

    #[test]
    fn cached_error_codes_match_excel_range_text() {
        for code in [
            "#DIV/0!", "#N/A", "#VALUE!", "#REF!", "#NAME?", "#NUM!", "#NULL!",
        ] {
            assert!(known_error_code(code));
            assert_eq!(
                label_text(&Shown::Err(code.into()), &NumFmt::General, false).unwrap(),
                code
            );
        }
        assert!(!known_error_code("#SPILL!"));
        assert!(!known_error_code("#CALC!"));
        assert!(!known_error_code("#DIV/0"));
        // 文字字面量走 Text，与 t=e 区分
        assert_eq!(
            label_text(&Shown::Text("#DIV/0!".into()), &NumFmt::General, false).unwrap(),
            "#DIV/0!"
        );
    }

    #[test]
    fn wrap_cell_lines_uses_face_width_breakpoints() {
        let Some(face) = Face::open_cell(14.6667, false, false, "Calibri") else {
            return;
        };
        let w0 = face.width("0");
        assert!(w0 > 0.0);
        // 窄到大约两个字宽，"one two" 应断成至少两行。
        let max_w = face.width("one ") + 0.5;
        let lines = wrap_cell_lines(&face, "one two three", max_w);
        assert!(lines.len() >= 2, "lines={lines:?}");
        assert!(lines.iter().all(|l| face.width(l) <= max_w + w0));
        // 硬换行
        let hard = wrap_cell_lines(&face, "a\nb", 1000.0);
        assert_eq!(hard, vec!["a".to_string(), "b".to_string()]);
        // 缩进像素公式：2 级 = 6 个 0 宽
        let indent2 = face.width("0") * 3.0 * 2.0;
        assert!((indent2 - face.width("000000")).abs() < 0.01);
    }

    #[test]
    fn short_dimension_still_covers_later_cells() {
        let xml = br#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><dimension ref="A1"/><sheetData><row r="1"><c r="A1"/><c r="C2"><v>1</v></c></row></sheetData></worksheet>"#;
        let doc = crate::xml::parse(xml).unwrap();
        assert_eq!(used_range(&doc.root).unwrap(), (1, 1, 3, 2));
    }

    #[test]
    fn general_format_code_ignores_case() {
        assert!(matches!(parse_format("GENERAL").unwrap(), NumFmt::General));
        assert!(matches!(parse_format("General").unwrap(), NumFmt::General));
        assert!(matches!(parse_format(";;;").unwrap(), NumFmt::Blank(4)));
        assert!(matches!(parse_format(";;").unwrap(), NumFmt::Blank(3)));
        assert!(matches!(parse_format(";").unwrap(), NumFmt::Blank(2)));
        // `0;` 现为合法两段（正显 / 负隐），不再 NI。
        assert!(matches!(parse_format("0;").unwrap(), NumFmt::Sections(_)));
        // `;;; ` 第四段为空格字面量（Excel 文本段），合法 Sections。
        assert!(matches!(parse_format(";;; ").unwrap(), NumFmt::Sections(_)));
        assert_eq!(
            label_text(&Shown::Num(123.0), &NumFmt::Blank(3), false).unwrap(),
            ""
        );
        assert_eq!(
            label_text(&Shown::Text("字".into()), &NumFmt::Blank(3), false).unwrap(),
            "字"
        );
        assert_eq!(
            label_text(&Shown::Text("字".into()), &NumFmt::Blank(4), false).unwrap(),
            ""
        );
        assert_eq!(
            label_text(&Shown::Bool(true), &NumFmt::Blank(4), false).unwrap(),
            ""
        );
        assert_eq!(
            label_text(&Shown::Bool(true), &NumFmt::Blank(2), false).unwrap(),
            "TRUE"
        );
    }
}
