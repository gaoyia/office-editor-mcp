//! 工作表已用区域出图。像素对齐 Excel 16.0 的 `Range.CopyPicture`（xlScreen、xlBitmap）。
//! 不乘工作表 `zoomScale`。缩放 95 的销售看板金样仍是 100% 的 1095×418。
//! `showGridLines` 为假时不画网格，上边和左边的外框留白，宽高不变。
//! 隐藏行和隐藏列不占像素。这一边全部隐藏时只留 1 像素白。

use std::collections::{HashMap, HashSet};

use crate::error::{Error, ErrorKind, IdReason};
use crate::opc::{Package, Relationships};
use crate::xml::Element;

use super::a1::{self, CellRef};
use super::chart_gdi::Face;
use super::eval::{self, Displayed, FormulaSession};
use super::inspect;
use super::picture::{self, AnchorKind};
use super::ExportedMedia;

const MAX_PX: i32 = 4096;
const GRID: [u8; 3] = [224, 224, 224];
const DEFAULT_COL_PX: i32 = 64;
const DEFAULT_ROW_PT: f64 = 15.0;

pub fn export_sheet_image(bytes: &[u8], id: &str) -> Result<ExportedMedia, Error> {
    let name = parse_sheet_id(id)?;
    let pkg = Package::open_bytes(bytes.to_vec())?;
    let sheet_part = picture::lookup_sheet_part(&pkg, &name, "id")?;
    let sheet_doc = super::list_pin::parse_part(&pkg, &sheet_part)?;
    let root = &sheet_doc.root;
    reject_view(root)?;
    reject_hidden_rules(root)?;
    let grid_lines = shows_grid(root);
    let wb_part = crate::opc::office_document_part(&pkg)?;
    let rels = Relationships::from_package(&pkg, &wb_part)?;
    let sst = inspect::load_shared_strings(&pkg, &wb_part, &rels)?;
    let styles = load_styles(&pkg, &wb_part, &rels)?;
    let (c0, r0, c1, r1) = used_range(root)?;
    reject_drawings(&pkg, &sheet_part, c0, r0, c1, r1)?;
    let session = eval::FormulaSession::open(&pkg, &name)?;
    let cells = read_cells(root, &sst, session)?;
    let cols = column_sizes(root, c0, c1)?;
    let rows = row_sizes(root, &styles, &cells, r0, r1)?;
    let canvas = paint(root, &styles, &cells, c0, r0, &cols, &rows, grid_lines)?;
    let png = encode_png(canvas.w, canvas.h, &canvas.px)?;
    Ok(ExportedMedia {
        bytes: png,
        content_type: Some("image/png".to_string()),
    })
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

fn reject_view(sheet: &Element) -> Result<(), Error> {
    let Some(view) = sheet
        .child_by_local("sheetViews")
        .and_then(|views| views.child_by_local("sheetView"))
    else {
        return Ok(());
    };
    // 从右到左时列从右边起排，外框像素会变。这一期不画。
    if truthy(view.attr("rightToLeft")) {
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

fn reject_drawings(
    pkg: &Package,
    sheet_part: &str,
    c0: u32,
    r0: u32,
    c1: u32,
    r1: u32,
) -> Result<(), Error> {
    for part in picture::drawing_parts(pkg, sheet_part)? {
        let doc = super::list_pin::parse_part(pkg, &part)?;
        if has_local(&doc.root, "grpSp") {
            return Err(unsupported());
        }
        picture::walk_anchors(&doc.root, &mut |anchor, kind| match kind {
            AnchorKind::TwoCell => {
                let (from, _, _) = picture::read_marker(anchor.child_by_local("from"))?;
                let (to, _, _) = picture::read_marker(anchor.child_by_local("to"))?;
                if let (Some(a), Some(b)) = (from, to) {
                    if overlaps(a.col, b.col, c0, c1) && overlaps(a.row, b.row, r0, r1) {
                        return Err(unsupported());
                    }
                }
                Ok(())
            }
            AnchorKind::OneCell => {
                let (from, _, _) = picture::read_marker(anchor.child_by_local("from"))?;
                if let Some(a) = from {
                    if a.col <= c1 && a.row <= r1 {
                        return Err(unsupported());
                    }
                }
                Ok(())
            }
            AnchorKind::Absolute | AnchorKind::RelSize | AnchorKind::AbsSize => Err(unsupported()),
        })?;
    }
    Ok(())
}

fn overlaps(a: u32, b: u32, lo: u32, hi: u32) -> bool {
    let a0 = a.min(b);
    let a1 = a.max(b);
    a0 <= hi && lo <= a1
}

fn has_local(el: &Element, name: &str) -> bool {
    el.local_name() == name || el.element_children().any(|child| has_local(child, name))
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
}

#[derive(Clone, Copy, Debug)]
enum Fill {
    None,
    Solid([u8; 3]),
    Nope,
}

struct FontInfo {
    name: String,
    pt: f64,
    bold: bool,
    italic: bool,
    color: Option<[u8; 3]>,
}

#[derive(Clone, Copy, Debug)]
struct Edge {
    px: i32,
    color: [u8; 3],
}

#[derive(Clone, Copy, Debug, Default)]
struct Edges {
    left: Option<Edge>,
    right: Option<Edge>,
    top: Option<Edge>,
    bottom: Option<Edge>,
}

struct Xf {
    font: usize,
    fill: usize,
    edges: Edges,
    num_fmt: u32,
    h_align: HAlign,
    v_align: VAlign,
}

struct Dxf {
    fill: Option<[u8; 3]>,
    color: Option<[u8; 3]>,
}

struct Styles {
    fonts: Vec<FontInfo>,
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

/// `tint` 按 ECMA-376 §18.8.3：负值把通道乘 `(1 + tint)`，正值向白靠拢。
fn apply_sheet_tint(rgb: [u8; 3], tint: f64) -> [u8; 3] {
    if tint.abs() < 1e-9 {
        return rgb;
    }
    rgb.map(|channel| {
        let v = f64::from(channel) / 255.0;
        let out = if tint < 0.0 {
            v * (1.0 + tint)
        } else {
            v * (1.0 - tint) + tint
        };
        (out * 255.0).round().clamp(0.0, 255.0) as u8
    })
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
            fills: vec![Fill::None, Fill::Nope],
            xfs: vec![Xf {
                font: 0,
                fill: 0,
                edges: Edges::default(),
                num_fmt: 0,
                h_align: HAlign::General,
                v_align: VAlign::Bottom,
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
    if fill.child_by_local("gradientFill").is_some() {
        return Fill::Nope;
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
        _ => Fill::Nope,
    }
}

fn parse_edges(border: &Element, palette: &Palette) -> Result<Edges, Error> {
    if truthy(border.attr("diagonalUp")) || truthy(border.attr("diagonalDown")) {
        return Err(unsupported());
    }
    if let Some(el) = border.child_by_local("diagonal") {
        if el.attr("style").is_some_and(|style| style != "none") {
            return Err(unsupported());
        }
    }
    Ok(Edges {
        left: parse_edge(border, "left", palette)?,
        right: parse_edge(border, "right", palette)?,
        top: parse_edge(border, "top", palette)?,
        bottom: parse_edge(border, "bottom", palette)?,
    })
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
    let px = match style {
        "thin" => 1,
        "medium" => 2,
        "thick" => 3,
        _ => return Err(unsupported()),
    };
    let color = match el.child_by_local("color") {
        None => [0, 0, 0],
        Some(color) => palette.resolve(color).ok_or_else(unsupported)?,
    };
    Ok(Some(Edge { px, color }))
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
    let (h_align, v_align) = match xf.child_by_local("alignment") {
        None => (HAlign::General, VAlign::Bottom),
        Some(al) => {
            if truthy(al.attr("wrapText"))
                || al.attr("textRotation").is_some_and(|v| v != "0")
                || al.attr("indent").is_some_and(|v| v != "0")
            {
                return Err(unsupported());
            }
            (read_h(al.attr("horizontal"))?, read_v(al.attr("vertical"))?)
        }
    };
    Ok(Xf {
        font,
        fill,
        edges,
        num_fmt,
        h_align,
        v_align,
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
        Some(fill) => match read_fill(fill, palette) {
            Fill::None => None,
            Fill::Solid(rgb) => Some(rgb),
            Fill::Nope => return Err(unsupported()),
        },
    };
    let color = dxf
        .child_by_local("font")
        .and_then(|font| font.child_by_local("color"))
        .and_then(|el| palette.resolve(el));
    if dxf.child_by_local("border").is_some() {
        return Err(unsupported());
    }
    Ok(Dxf { fill, color })
}

fn builtin_fmt(id: u32) -> Option<NumFmt> {
    match id {
        0 | 49 => Some(NumFmt::General),
        1 => Some(NumFmt::Fixed(0)),
        2 => Some(NumFmt::Fixed(2)),
        3 => Some(NumFmt::IntComma),
        9 => Some(NumFmt::Percent(0)),
        10 => Some(NumFmt::Percent(2)),
        _ => None,
    }
}

fn parse_format(code: &str) -> Result<NumFmt, Error> {
    let code = code.trim();
    if code.contains(';') || code.contains('[') {
        return Err(unsupported());
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
    Ok(NumFmt::Fixed(zeros(code)?))
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
}

struct CellRec {
    style: usize,
    shown: Shown,
}

fn read_cells(
    sheet: &Element,
    sst: &[String],
    mut session: Option<FormulaSession>,
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
            let shown = shown_of(cell, sst, CellRef { col, row: row_n }, &mut session)?;
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

/// Excel 把列宽换成 96 dpi 像素。`16.7109375` 是 117。
fn excel_col_px(width: f64) -> i32 {
    let mdw = 7.0f64;
    let px = ((256.0 * width + (128.0 / mdw).floor()) / 256.0) * mdw;
    px.floor() as i32
}

fn row_sizes(
    sheet: &Element,
    styles: &Styles,
    cells: &HashMap<(u32, u32), CellRec>,
    r0: u32,
    r1: u32,
) -> Result<Vec<i32>, Error> {
    let default_pt = sheet
        .child_by_local("sheetFormatPr")
        .and_then(|el| el.attr("defaultRowHeight"))
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_ROW_PT);
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
            row_auto_pt(styles, cells, row, default_pt)?
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
    styles: &Styles,
    cells: &HashMap<(u32, u32), CellRec>,
    row: u32,
    default_pt: f64,
) -> Result<f64, Error> {
    let mut pt = default_pt;
    for ((_, r), cell) in cells {
        if *r != row {
            continue;
        }
        let font = styles.font(cell.style)?;
        pt = pt.max(auto_row_pt(&font.name, font.pt));
    }
    Ok(pt)
}

/// 未写行高时的磅值。Calibri 11 是 15。10 磅微软雅黑是 16.5。
fn auto_row_pt(name: &str, pt: f64) -> f64 {
    const CJK: &[&str] = &[
        "微软雅黑",
        "宋体",
        "黑体",
        "等线",
        "楷体",
        "仿宋",
        "DengXian",
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
    px: Vec<u8>,
}

impl Canvas {
    fn new(w: i32, h: i32) -> Result<Self, Error> {
        if !(1..=MAX_PX).contains(&w) || !(1..=MAX_PX).contains(&h) {
            return Err(unsupported());
        }
        let n = (w as usize)
            .checked_mul(h as usize)
            .and_then(|n| n.checked_mul(3))
            .ok_or_else(unsupported)?;
        Ok(Self {
            w,
            h,
            px: vec![255; n],
        })
    }

    fn put(&mut self, x: i32, y: i32, color: [u8; 3]) {
        if x < 0 || y < 0 || x >= self.w || y >= self.h {
            return;
        }
        let i = (y * self.w + x) as usize * 3;
        self.px[i] = color[0];
        self.px[i + 1] = color[1];
        self.px[i + 2] = color[2];
    }

    fn blend(&mut self, x: i32, y: i32, color: [u8; 3], cov: u8) {
        if cov == 0 || x < 0 || y < 0 || x >= self.w || y >= self.h {
            return;
        }
        if cov == 255 {
            self.put(x, y, color);
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

    fn grid(&mut self, xs: &Axis, ys: &Axis) {
        for x in &xs.origin {
            for y in 0..self.h {
                self.put(*x, y, GRID);
            }
        }
        for y in &ys.origin {
            for x in 0..self.w {
                self.put(x, *y, GRID);
            }
        }
    }

    /// 上边和左边的外框盖住第一格的填充。网格开着时是网格色，关掉时留白。
    fn leading_frame(&mut self, color: [u8; 3]) {
        for x in 0..self.w {
            self.put(x, 0, color);
        }
        for y in 0..self.h {
            self.put(0, y, color);
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
) -> Result<Canvas, Error> {
    let xs = Axis::new(cols);
    let ys = Axis::new(rows);
    let mut canvas = Canvas::new(xs.bitmap(), ys.bitmap())?;
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
        )?;
    }
    paint_rules(&mut canvas, sheet, cells, &xs, &ys, c0, r0)?;
    let mut fonts = FontBook::default();
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
    )?;
    if draw_frame {
        canvas.leading_frame(if grid_lines { GRID } else { [255, 255, 255] });
    }
    paint_borders(&mut canvas, styles, cells, &xs, &ys, c0, r0, &rects)?;
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
) -> Result<(), Error> {
    let x0 = xs.origin[(col - c0) as usize];
    let x1 = xs.origin[(col_end - c0) as usize + 1];
    let y0 = ys.origin[(row - r0) as usize];
    let y1 = ys.origin[(row_end - r0) as usize + 1];
    // 隐藏行、隐藏列的跨度是 0。不能往交界上涂出 1 像素。
    if x1 == x0 || y1 == y0 {
        return Ok(());
    }
    let Some(cell) = cells.get(&(col, row)) else {
        return Ok(());
    };
    let xf = styles.xf(cell.style)?;
    let fill = styles.fills.get(xf.fill).copied().unwrap_or(Fill::Nope);
    if matches!(fill, Fill::Nope) {
        return Err(unsupported());
    }
    let base = match fill {
        Fill::Solid(rgb) => Some(rgb),
        Fill::None => None,
        Fill::Nope => None,
    };
    let Some(rgb) = cf_fill(sheet, styles, cells, col, row)?.or(base) else {
        return Ok(());
    };
    canvas.fill_cell(x0, y0, x1, y1, rgb);
    Ok(())
}

fn paint_borders(
    canvas: &mut Canvas,
    styles: &Styles,
    cells: &HashMap<(u32, u32), CellRec>,
    xs: &Axis,
    ys: &Axis,
    c0: u32,
    r0: u32,
    rects: &[(u32, u32, u32, u32)],
) -> Result<(), Error> {
    let mut strokes = Vec::new();
    let mut order = 0u32;
    for &(row, col, col_end, row_end) in rects {
        let Some(cell) = cells.get(&(col, row)) else {
            continue;
        };
        let edges = styles.xf(cell.style)?.edges;
        if edges.left.is_none()
            && edges.right.is_none()
            && edges.top.is_none()
            && edges.bottom.is_none()
        {
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
        let mut edges = edges;
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
        );
    }
    // 更粗的后画，盖住同一条边界上的细线。同样粗时保持格子的先后。
    strokes.sort_by_key(|stroke| (stroke.px, stroke.order));
    for stroke in strokes {
        canvas.fill_rect(stroke.x0, stroke.y0, stroke.x1, stroke.y1, stroke.color);
    }
    Ok(())
}

struct Stroke {
    px: i32,
    order: u32,
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

/// 邻边实际存在时，线才向那一端伸出它的外侧像素。
fn end_pad(edge: Option<Edge>) -> i32 {
    edge.map(|edge| edge.px / 2).unwrap_or(0)
}

/// 线以边界像素为中线。偶数宽度多出的一像素朝格外。
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
) {
    let Some(edge) = edge else {
        return;
    };
    let outside = edge.px / 2;
    let inside = edge.px - outside - 1;
    let (along0, along1, boundary, vertical, outward_negative) = match side {
        Side::Left => (y0, y1, x0, true, true),
        Side::Right => (y0, y1, x1, true, false),
        Side::Top => (x0, x1, y0, false, true),
        Side::Bottom => (x0, x1, y1, false, false),
    };
    let (across0, across1) = if outward_negative {
        (boundary - outside, boundary + inside + 1)
    } else {
        (boundary - inside, boundary + outside + 1)
    };
    let (x0, x1, y0, y1) = if vertical {
        (across0, across1, along0 - pad0, along1 + pad1 + 1)
    } else {
        (along0 - pad0, along1 + pad1 + 1, across0, across1)
    };
    out.push(Stroke {
        px: edge.px,
        order: *order,
        x0,
        y0,
        x1,
        y1,
        color: edge.color,
    });
    *order += 1;
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
            "dataBar" => {}
            _ => return Err(unsupported()),
        }
    }
    for (_, areas, rule) in &rules {
        if rule.attr("type") == Some("dataBar") {
            paint_bar(canvas, cells, xs, ys, c0, r0, areas, rule)?;
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

fn cf_fill(
    sheet: &Element,
    styles: &Styles,
    cells: &HashMap<(u32, u32), CellRec>,
    col: u32,
    row: u32,
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
    let cfvos = bar.children_by_local("cfvo");
    if cfvos.len() != 2 {
        return Err(unsupported());
    }
    let values = area_numbers(cells, areas);
    if values.iter().any(|n| *n < 0.0) {
        return Err(unsupported());
    }
    let min = cfvo_value(cfvos[0], &values)?;
    let max = cfvo_value(cfvos[1], &values)?;
    let color = bar
        .child_by_local("color")
        .and_then(|el| el.attr("rgb"))
        .and_then(parse_rgb)
        .ok_or_else(unsupported)?;
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
        let text = label_text(&cell.shown, &fmt)?;
        if text.is_empty() {
            continue;
        }
        let Some(color) = font.color else {
            return Err(unsupported());
        };
        let color = dxf_color(sheet, styles, cells, col, row).unwrap_or(color);
        let mut clip_r = x1;
        if filter_hit(filter, col, row, col_end) {
            clip_r = clip_r.min(x1 - 17);
        }
        let face = fonts.face(&font.name, font.pt, font.bold, font.italic)?;
        let width = face.width(&text);
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
        let x = match h_align {
            HAlign::Left | HAlign::General => f64::from(x0) + face.cell_pad(),
            HAlign::Center => f64::from(x0) + (f64::from(x1 - x0) - width) / 2.0,
            HAlign::Right => f64::from(x1) - width - 2.0,
        };
        let em = face.em_box();
        let top = match xf.v_align {
            VAlign::Top => f64::from(y0) + 2.0,
            VAlign::Center => f64::from(y0) + (f64::from(y1 - y0) - em) / 2.0,
            VAlign::Bottom => f64::from(y1) - em - 1.0,
        };
        face.draw_cells(&text, x, top, color, |px, py, rgb, cov| {
            if px < x0 || py < y0 || px >= clip_r || py >= y1 {
                return;
            }
            canvas.blend(px, py, rgb, cov);
        });
    }
    paint_filters(canvas, xs, ys, c0, r0, filter);
    Ok(())
}

fn label_text(shown: &Shown, fmt: &NumFmt) -> Result<String, Error> {
    match shown {
        Shown::Empty => Ok(String::new()),
        Shown::Text(text) => Ok(text.clone()),
        Shown::Bool(true) => Ok("TRUE".into()),
        Shown::Bool(false) => Ok("FALSE".into()),
        Shown::Num(n) => format_num(*n, fmt),
    }
}

fn format_num(n: f64, fmt: &NumFmt) -> Result<String, Error> {
    if !n.is_finite() {
        return Err(unsupported());
    }
    Ok(match fmt {
        NumFmt::General => format_general(n),
        NumFmt::IntComma => format_comma(n),
        NumFmt::Fixed(digits) => format_fixed(n, *digits),
        NumFmt::Percent(digits) => format!("{}%", format_fixed(n * 100.0, *digits)),
    })
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

fn dxf_color(
    sheet: &Element,
    styles: &Styles,
    cells: &HashMap<(u32, u32), CellRec>,
    col: u32,
    row: u32,
) -> Option<[u8; 3]> {
    let Shown::Num(n) = cells.get(&(col, row))?.shown else {
        return None;
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
            if rule.attr("type") != Some("cellIs") {
                continue;
            }
            let Ok(args) = rule_numbers(rule) else {
                continue;
            };
            let Ok(true) = cell_is(rule.attr("operator").unwrap_or(""), n, &args) else {
                continue;
            };
            let Some(id) = rule.attr("dxfId").and_then(|s| s.parse::<usize>().ok()) else {
                continue;
            };
            if let Some(color) = styles.dxfs.get(id).and_then(|dxf| dxf.color) {
                hit = Some(color);
            }
        }
    }
    hit
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
        height += data * 7;
        assert_eq!(height, 418);
        assert_eq!(excel_row_px(20.0), 26);
        assert_eq!(excel_row_px(22.0), 29);
        assert_eq!(excel_row_px(32.0), 42);
        assert_eq!(excel_row_px(26.0), 34);
    }

    #[test]
    fn theme_one_is_black_and_tint_moves_toward_white() {
        let slots = default_theme();
        assert_eq!(slots[0], [255, 255, 255]);
        assert_eq!(slots[1], [0, 0, 0]);
        assert_eq!(apply_sheet_tint([0, 0, 0], 0.5), [128, 128, 128]);
        assert_eq!(apply_sheet_tint([255, 255, 255], -0.5), [128, 128, 128]);
        let palette = Palette {
            theme: slots,
            indexed: DEFAULT_INDEXED.iter().copied().map(rgb_of).collect(),
        };
        assert_eq!(palette.indexed_at(2), Some([255, 0, 0]));
        assert_eq!(palette.indexed_at(64), Some([0, 0, 0]));
        assert_eq!(palette.indexed_at(80), None);
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
    }
}
