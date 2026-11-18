//! 簇状柱形图、簇状条形图、普通折线图、带标记的散点图、普通面积图和普通饼图只读栅格化。优先读图部件里已有的缓存。没有缓存时，读公式所指格子里已有的值。不写回包，也不算公式。
//! 像素差要改这里的布局、颜色和字形，不要靠放宽对照容差。
//! 折线本切片只画 `c:lineChart` 且 grouping 为 standard 或省略的图：实线、缺省/auto 按系列循环标记（diamond、square、triangle、x、star、circle、plus、dot、dash，9 个一圈）、显式符号、空值按缺口断线。省略 `c:marker` 时尺寸为 9 磅；写出的 `c:size` 优先，写出标记但省略尺寸时按 OOXML 缺省 5 磅。堆积、百分比堆积、平滑线、虚线、空值按零或连线、三维折线、组合图及其余类型仍是 NotImplemented，不用缺一块的图充数。
//! 散点本切片只画 `c:scatterChart` 且 scatterStyle 为 marker 或 lineMarker（缺省按 lineMarker）的图：X/Y 都是数值轴，点按系列顺序用数值坐标定位；lineMarker 画实线，缺值断开；标记规则与折线相同。仅连线、平滑、虚线、空值按零或连线、非数值 X/Y、次轴及其余类型仍是 NotImplemented。
//! 条形本切片只画 `c:barChart` 且 `barDir` 为 `bar` 或省略、grouping 为 clustered 或省略的簇状条形图：数值轴在下、类目轴在左；类目 minMax（或缺省）时第一类在下，maxMin 时第一类在上；系列 0 在类目轴方向的槽起点（maxMin 时槽顶，minMax 时槽底）；正值向右、负值向左，缺值不画该条。堆积、百分比、三维、可见数据标签、空值按零或连线、类目轴网格、非 l/b 轴位及其余类型仍是 NotImplemented。
//! 面积本切片只画 `c:areaChart` 且 grouping 为 standard 或省略的图：按与折线相同的类目槽公式定位，用不透明纯色填到数值轴交叉（autoZero 为零，零在轴外时取较近端）；正值在交叉之上、负值在下，相邻异号在交叉处剖开以免自交；系列按 order 先画后盖；缺值按缺口断开填充，不跨空点相连。有写出的实线描边才画上沿线，不默认画标记。堆积、百分比堆积、三维、可见数据标签、空值按零或连线、半透明填充、组合图、标记及其余类型仍是 NotImplemented，不用缺一块的图充数。
//! 饼图本切片只画单系列 `c:pieChart`：扇区从 12 点方向按 firstSliceAng（缺省 0，顺时针，单位度）起算，只把有限正值按总和比例切开；空值跳过该扇区，零值不占角度但仍可出现在图例；负值、多系列、三维、环形、复合饼、爆炸、可见数据标签、半透明或非纯色扇区、非右侧缺省图例及其余类型仍是 NotImplemented，不用缺一块的图充数。

use fontdue::{Font, FontSettings};

#[path = "chart_axis.rs"]
mod axis;
#[path = "chart_legend.rs"]
mod legend;

use super::render_dpi::RenderDpi;
use crate::error::{Error, ErrorKind};
use crate::xml::Element;

const MAX_PX: u32 = 4096;
/// 图里没写字号时，标题、轴、类目、图例都是 10 磅。标题另用粗体。
const LABEL_PT: f64 = 10.0;

const INK: [u8; 3] = [0, 0, 0];
const BORDER: [u8; 3] = [0x89, 0x89, 0x89];
/// 下列边距、刻度臂长和图例间距是 96 dpi 下的像素。出图时经 `RenderDpi::scale_px`，
/// 96 dpi 保持原值。圆角 `CORNER_R` 仍固定，圆角框另案处理。
const INSET_TOP: i32 = 14;
const INSET_RIGHT: i32 = 60;
const INSET_BOTTOM: i32 = 34;
const TICK_OUT: i32 = 3;
/// 刻度字前进宽度的左缘。加上字宽和 `TICK_GAP` 就是轴线。
/// 缩放 100 的金样：字宽 7 → 27，字宽 17 → 37，字宽 28 → 48。
const LABEL_EDGE: i32 = 9;
const TICK_GAP: i32 = 11;
/// 色块、色块到字、绘图区到色块、字到右缘。短标签用不满 `INSET_RIGHT`。
const LEGEND_SWATCH: i32 = 8;
const LEGEND_GAP: i32 = 6;
const LEGEND_PLOT_GAP: i32 = 22;
const LEGEND_PAD: i32 = 4;
/// 有标题时，图例在这 30 像素标题带之下垂直居中。无标题的图不加这截。
const LEGEND_TITLE_BAND: i32 = 30;
/// 标题墨迹比布局顶低 7 像素。Excel 的绘图区仍从布局顶往下排。
/// PowerPoint 幻灯片把这 7 像素算进绘图区，网格才和导出的页对齐。
const TITLE_DROP: f64 = 7.0;
/// Excel 16.0 图表外框圆角。576、384、647 三档都是 14 像素。
const CORNER_R: i32 = 14;
const CAT_MAJOR: i32 = 4;
const CAT_MAJOR_IN: i32 = 3;
const CAT_MINOR: i32 = 3;
const CAT_MINOR_IN: i32 = 2;
/// 数值轴一个主刻度分成 5 段，中间 4 道次刻度。实心部分向两侧各 2 像素。
#[cfg(test)]
const VAL_MINOR_PARTS: i32 = 5;
const VAL_MINOR_ARM: i32 = 2;

/// 包里没有 theme 时，Excel 16.0 用当前 Office 主题给自动系列上色。这套是 2023 的 accent。
const FALLBACK_ACCENTS: [[u8; 3]; 6] = [
    [0x15, 0x60, 0x82],
    [0xE9, 0x71, 0x32],
    [0x19, 0x6B, 0x24],
    [0x0F, 0x9E, 0xD5],
    [0xA0, 0x2B, 0x93],
    [0x4E, 0xA7, 0x2E],
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum LegendPos {
    Right,
    Left,
    Top,
    Bottom,
    TopRight,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PlotKind {
    ClusteredColumn,
    ClusteredBar,
    StandardLine,
    StandardArea,
    Pie,
    Scatter { connect: bool },
}

/// Excel default auto-marker cycle when `c:marker` is omitted or `symbol=auto`.
/// Locked from Excel 16.0 Chart.Export of a dedicated 10-series Temp workbook
/// (omitted `c:marker`) plus Series.MarkerSize=9: S1 diamond … S9 dash, S10 wraps
/// to diamond. Official 2-series line golds show the same S1 diamond / S2 square.
const DEFAULT_MARKER_CYCLE: [MarkerSymbol; 9] = [
    MarkerSymbol::Diamond,
    MarkerSymbol::Square,
    MarkerSymbol::Triangle,
    MarkerSymbol::X,
    MarkerSymbol::Star,
    MarkerSymbol::Circle,
    MarkerSymbol::Plus,
    MarkerSymbol::Dot,
    MarkerSymbol::Dash,
];
/// Point size used when the series has no `c:marker` element. Unique lock:
/// Excel COM MarkerSize=9 and 13x13 px boxes at 106 dpi (9 * 106 / 72 = 13.25).
const OMITTED_MARKER_SIZE_PT: f64 = 9.0;
/// OOXML default for `c:size` when a marker element is present but the size is omitted.
const EXPLICIT_MARKER_DEFAULT_SIZE_PT: f64 = 5.0;

#[derive(Clone, Copy, PartialEq)]
enum MarkerSymbol {
    Diamond,
    Square,
    Triangle,
    X,
    Star,
    Circle,
    Plus,
    Dot,
    Dash,
}

#[derive(Clone, Copy, PartialEq)]
enum MarkerKind {
    None,
    Shaped { symbol: MarkerSymbol, size: f64 },
}

fn default_marker_symbol(slot: usize) -> MarkerSymbol {
    DEFAULT_MARKER_CYCLE[slot % DEFAULT_MARKER_CYCLE.len()]
}

fn shaped_marker(symbol: MarkerSymbol, size_pt: f64, dpi: RenderDpi) -> MarkerKind {
    MarkerKind::Shaped {
        symbol,
        size: dpi.pt_to_px_font(size_pt),
    }
}

fn marker_symbol_from_name(name: &str) -> Option<MarkerSymbol> {
    match name {
        "diamond" => Some(MarkerSymbol::Diamond),
        "square" => Some(MarkerSymbol::Square),
        "triangle" => Some(MarkerSymbol::Triangle),
        "x" => Some(MarkerSymbol::X),
        "star" => Some(MarkerSymbol::Star),
        "circle" => Some(MarkerSymbol::Circle),
        "plus" => Some(MarkerSymbol::Plus),
        "dot" => Some(MarkerSymbol::Dot),
        "dash" => Some(MarkerSymbol::Dash),
        _ => None,
    }
}

fn with_marker_size(marker: MarkerKind, size: f64) -> MarkerKind {
    match marker {
        MarkerKind::None => MarkerKind::None,
        MarkerKind::Shaped { symbol, .. } => MarkerKind::Shaped { symbol, size },
    }
}

struct LineSeriesStyle {
    color: Option<[u8; 3]>,
    visible: bool,
    width: f64,
    marker: MarkerKind,
    marker_color: Option<[u8; 3]>,
}

struct Series {
    name: String,
    values: Vec<Option<f64>>,
    color: Option<[u8; 3]>,
    slot: usize,
    line_visible: bool,
    line_width: f64,
    marker: MarkerKind,
    marker_color: Option<[u8; 3]>,
    /// Area stroke color when a solid outline is present. Lines keep this unset.
    line_color: Option<[u8; 3]>,
    x_values: Vec<Option<f64>>,
}

impl Default for Series {
    fn default() -> Self {
        Self {
            name: String::new(),
            values: Vec::new(),
            color: None,
            slot: 0,
            line_visible: false,
            line_width: 0.0,
            marker: MarkerKind::None,
            marker_color: None,
            line_color: None,
            x_values: Vec::new(),
        }
    }
}

struct ColumnChart {
    categories: Vec<String>,
    series: Vec<Series>,
    gap_width: f64,
    overlap: f64,
    vary_points: bool,
    grid: bool,
    legend: Option<LegendPos>,
    legend_spec: legend::Spec,
    axis: axis::Spec,
    axis_x: Option<axis::Spec>,
    grid_x: bool,
    /// 条形图类目轴：minMax 时第一类在下，maxMin 时第一类在上。系列 0 落在该类目轴方向的槽起点。柱形/折线不用。
    cats_from_bottom: bool,
    title: Option<String>,
    accents: [[u8; 3]; 6],
    kind: PlotKind,
    /// Pie first slice angle in degrees, clockwise from up. Unused for other kinds.
    first_slice_ang: f64,
    /// Pie per-point fill overrides (dPt). Empty for other kinds.
    point_colors: Vec<Option<[u8; 3]>>,
}

struct Rect {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

struct Canvas {
    w: u32,
    h: u32,
    px: Vec<u8>,
    /// Straight alpha generated by the existing rounded-frame coverage, never a white key.
    alpha: Option<Vec<u8>>,
    dpi: RenderDpi,
}

/// 图表 PNG 的像素。`twoCell` 按格子累加，其它锚点用 `a:ext`。缩放来自工作表。
pub(crate) fn bitmap_px(
    sheet: &Element,
    anchor: &Element,
    kind: super::picture::AnchorKind,
    geometry: &super::geometry::GeometryContext,
) -> Result<(u32, u32), Error> {
    let zoom = sheet_zoom(sheet);
    let (w, h) = match kind {
        super::picture::AnchorKind::TwoCell => two_cell_px(sheet, anchor, zoom, geometry)?,
        _ => ext_px(anchor, zoom, geometry.dpi())?,
    };
    Ok((fit_px(w)?, fit_px(h)?))
}

fn fit_px(px: i64) -> Result<u32, Error> {
    if !(1..=i64::from(MAX_PX)).contains(&px) {
        return Err(Error::with_path(ErrorKind::ChartImageUnsupported, "id"));
    }
    Ok(px as u32)
}

fn sheet_zoom(sheet: &Element) -> u32 {
    sheet
        .child_by_local("sheetViews")
        .and_then(|views| views.child_by_local("sheetView"))
        .and_then(|view| view.attr("zoomScale"))
        .and_then(|s| s.parse::<u32>().ok())
        .filter(|n| (10..=400).contains(n))
        .unwrap_or(100)
}

fn zoomed(px: i64, zoom: u32) -> i64 {
    (px as f64 * f64::from(zoom) / 100.0).round() as i64
}

fn emu_px(emu: i64, dpi: RenderDpi) -> i64 {
    dpi.emu_to_px_round(emu)
}

fn pt_to_px(pt: f64, dpi: RenderDpi) -> i64 {
    let emu = (pt * 12_700.0).round() as i64;
    dpi.emu_to_px_round(emu)
}

fn row_px(sheet: &Element, row: u32, dpi: RenderDpi) -> i64 {
    if let Some(sheet_data) = sheet.child_by_local("sheetData") {
        for el in sheet_data.children_by_local("row") {
            let Some(r) = el.attr("r").and_then(|s| s.parse::<u32>().ok()) else {
                continue;
            };
            if r != row {
                continue;
            }
            if let Some(ht) = el.attr("ht").and_then(|s| s.parse::<f64>().ok()) {
                return pt_to_px(ht, dpi).max(1);
            }
        }
    }
    let def = sheet
        .child_by_local("sheetFormatPr")
        .and_then(|el| el.attr_local("defaultRowHeight"))
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(14.25);
    pt_to_px(def, dpi).max(1)
}

fn marker_x(
    sheet: &Element,
    col: u32,
    off: i64,
    zoom: u32,
    geometry: &super::geometry::GeometryContext,
) -> Result<i64, Error> {
    let mut px = 0i64;
    for c in 1..col {
        px += zoomed(
            i64::from(
                geometry
                    .column_px(sheet, c)
                    .map_err(|_| Error::with_path(ErrorKind::ChartImageUnsupported, "id"))?,
            ),
            zoom,
        );
    }
    Ok(px + zoomed(emu_px(off, geometry.dpi()), zoom))
}

fn marker_y(sheet: &Element, row: u32, off: i64, zoom: u32, dpi: RenderDpi) -> i64 {
    let mut px = 0i64;
    for r in 1..row {
        px += zoomed(row_px(sheet, r, dpi), zoom);
    }
    px + zoomed(emu_px(off, dpi), zoom)
}

fn two_cell_px(
    sheet: &Element,
    anchor: &Element,
    zoom: u32,
    geometry: &super::geometry::GeometryContext,
) -> Result<(i64, i64), Error> {
    let (from, from_x, from_y) = super::picture::read_marker(anchor.child_by_local("from"))?;
    let (to, to_x, to_y) = super::picture::read_marker(anchor.child_by_local("to"))?;
    let (Some(from), Some(to)) = (from, to) else {
        return ext_px(anchor, zoom, geometry.dpi());
    };
    let w = marker_x(sheet, to.col, to_x, zoom, geometry)?
        - marker_x(sheet, from.col, from_x, zoom, geometry)?;
    let h = marker_y(sheet, to.row, to_y, zoom, geometry.dpi())
        - marker_y(sheet, from.row, from_y, zoom, geometry.dpi());
    Ok((w, h))
}

fn ext_px(anchor: &Element, zoom: u32, dpi: RenderDpi) -> Result<(i64, i64), Error> {
    let ext = anchor
        .child_by_local("graphicFrame")
        .and_then(|frame| frame.child_by_local("xfrm"))
        .and_then(|xfrm| xfrm.child_by_local("ext"))
        .or_else(|| anchor.child_by_local("ext"));
    let Some(ext) = ext else {
        return Err(Error::with_path(ErrorKind::ChartImageUnsupported, "id"));
    };
    let cx: i64 = ext
        .attr_local("cx")
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| Error::with_path(ErrorKind::ChartImageUnsupported, "id"))?;
    let cy: i64 = ext
        .attr_local("cy")
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| Error::with_path(ErrorKind::ChartImageUnsupported, "id"))?;
    Ok((zoomed(emu_px(cx, dpi), zoom), zoomed(emu_px(cy, dpi), zoom)))
}

pub(crate) fn theme_accents(theme_xml: Option<&[u8]>) -> [[u8; 3]; 6] {
    let mut accents = FALLBACK_ACCENTS;
    let Some(bytes) = theme_xml else {
        return accents;
    };
    let Ok(doc) = crate::xml::parse(bytes) else {
        return accents;
    };
    let Some(scheme) = find_deep(&doc.root, "clrScheme") else {
        return accents;
    };
    for (i, name) in [
        "accent1", "accent2", "accent3", "accent4", "accent5", "accent6",
    ]
    .into_iter()
    .enumerate()
    {
        if let Some(color) = scheme
            .child_by_local(name)
            .and_then(|slot| color_of_slot(slot, &accents))
        {
            accents[i] = color;
        }
    }
    accents
}

/// Excel 导出带圆角外框，绘图区从标题布局顶起算。
/// 幻灯片导出没有这圈外框，有标题时绘图区再低一截。
#[derive(Clone, Copy)]
pub(crate) enum ChartHost {
    Excel,
    Slide,
}

pub(crate) fn render_chart(
    root: &Element,
    width: u32,
    height: u32,
    accents: [[u8; 3]; 6],
    font_bytes: Option<&[u8]>,
    host: ChartHost,
    cells: Option<&super::sparkline::CellCache>,
    sheet: &str,
) -> Result<Vec<u8>, Error> {
    render_chart_at(
        root,
        width,
        height,
        accents,
        font_bytes,
        host,
        cells,
        sheet,
        RenderDpi::DEFAULT,
    )
}

pub(crate) fn render_chart_at(
    root: &Element,
    width: u32,
    height: u32,
    accents: [[u8; 3]; 6],
    font_bytes: Option<&[u8]>,
    host: ChartHost,
    cells: Option<&super::sparkline::CellCache>,
    sheet: &str,
    dpi: RenderDpi,
) -> Result<Vec<u8>, Error> {
    if width == 0 || height == 0 || width > MAX_PX || height > MAX_PX {
        return Err(Error::with_path(ErrorKind::ChartImageUnsupported, "id"));
    }
    if matches!(host, ChartHost::Excel) {
        validate_default_frame(root)?;
    }
    let chart = read_chart(root, accents, cells, sheet, dpi)?;
    let font = font_bytes.and_then(|bytes| Font::from_bytes(bytes, FontSettings::default()).ok());
    let mut canvas = Canvas::with_dpi(width, height, dpi);
    paint(&mut canvas, &chart, font.as_ref(), host)?;
    canvas.encode_png()
}

/// This painter only has evidence for the existing opaque-white rounded frame.
/// Valid but unsupported backgrounds/borders are NI, not malformed defaults.
fn validate_default_frame(root: &Element) -> Result<(), Error> {
    let unsupported = || Error::with_path(ErrorKind::ChartImageUnsupported, "id");
    if flag(root, "roundedCorners") == Some(false) {
        return Err(unsupported());
    }
    let plot = root
        .child_by_local("chart")
        .and_then(|c| c.child_by_local("plotArea"));
    for node in [Some(root), plot].into_iter().flatten() {
        if let Some(props) = node.child_by_local("spPr") {
            for prop in props.element_children() {
                let supported = match prop.local_name() {
                    "solidFill" => {
                        let mut colors = prop.element_children();
                        let color = colors.next();
                        colors.next().is_none()
                            && color.is_some_and(|c| {
                                c.local_name() == "srgbClr"
                                    && c.attr_local("val")
                                        .is_some_and(|v| v.eq_ignore_ascii_case("FFFFFF"))
                                    && c.element_children().next().is_none()
                            })
                    }
                    "effectLst" => prop.element_children().next().is_none(),
                    _ => false,
                };
                if !supported {
                    return Err(unsupported());
                }
            }
        }
    }
    Ok(())
}

fn read_chart(
    root: &Element,
    accents: [[u8; 3]; 6],
    cells: Option<&super::sparkline::CellCache>,
    sheet: &str,
    dpi: RenderDpi,
) -> Result<ColumnChart, Error> {
    let unsupported = || Error::with_path(ErrorKind::ChartImageUnsupported, "id");
    let chart = root.child_by_local("chart").ok_or_else(unsupported)?;
    let plot = chart.child_by_local("plotArea").ok_or_else(unsupported)?;
    let mut wrapper = None;
    for child in plot.element_children() {
        if is_chart_wrapper(child.local_name()) {
            if wrapper.is_some() {
                return Err(unsupported());
            }
            wrapper = Some(child);
        }
    }
    let wrapper = wrapper.ok_or_else(unsupported)?;
    let kind = match wrapper.local_name() {
        "barChart" => {
            let dir = wrapper
                .child_by_local("barDir")
                .and_then(|el| el.attr_local("val"))
                .unwrap_or("bar");
            let grouping = wrapper
                .child_by_local("grouping")
                .and_then(|el| el.attr_local("val"))
                .unwrap_or("clustered");
            if grouping != "clustered" {
                return Err(unsupported());
            }
            if dir == "bar" {
                data_labels_hidden(wrapper.child_by_local("dLbls"))?;
                let blanks = chart
                    .child_by_local("dispBlanksAs")
                    .and_then(|el| el.attr_local("val"))
                    .unwrap_or("gap");
                if blanks != "gap" {
                    return Err(unsupported());
                }
                if flag(wrapper, "varyColors").unwrap_or(false)
                    && wrapper
                        .element_children()
                        .filter(|el| el.local_name() == "ser")
                        .count()
                        != 1
                {
                    return Err(unsupported());
                }
            }
            match dir {
                "col" => PlotKind::ClusteredColumn,
                "bar" => PlotKind::ClusteredBar,
                _ => return Err(unsupported()),
            }
        }
        "lineChart" => {
            let grouping = wrapper
                .child_by_local("grouping")
                .and_then(|el| el.attr_local("val"))
                .unwrap_or("standard");
            if grouping != "standard" {
                return Err(unsupported());
            }
            if flag(wrapper, "varyColors").unwrap_or(false) {
                return Err(unsupported());
            }
            if flag(wrapper, "smooth").unwrap_or(false) {
                return Err(unsupported());
            }
            for child in wrapper.element_children() {
                if matches!(
                    child.local_name(),
                    "dropLines" | "hiLowLines" | "upDownBars"
                ) {
                    return Err(unsupported());
                }
            }
            data_labels_hidden(wrapper.child_by_local("dLbls"))?;
            let blanks = chart
                .child_by_local("dispBlanksAs")
                .and_then(|el| el.attr_local("val"))
                .unwrap_or("gap");
            if blanks != "gap" {
                return Err(unsupported());
            }
            PlotKind::StandardLine
        }
        "scatterChart" => {
            if flag(wrapper, "varyColors").unwrap_or(false) {
                return Err(unsupported());
            }
            data_labels_hidden(wrapper.child_by_local("dLbls"))?;
            let blanks = chart
                .child_by_local("dispBlanksAs")
                .and_then(|el| el.attr_local("val"))
                .unwrap_or("gap");
            if blanks != "gap" {
                return Err(unsupported());
            }
            for child in wrapper.element_children() {
                if matches!(
                    child.local_name(),
                    "dropLines" | "hiLowLines" | "upDownBars"
                ) {
                    return Err(unsupported());
                }
            }
            let style = wrapper
                .child_by_local("scatterStyle")
                .and_then(|el| el.attr_local("val"))
                .unwrap_or("lineMarker");
            let connect = match style {
                "marker" => false,
                "lineMarker" => true,
                _ => return Err(unsupported()),
            };
            PlotKind::Scatter { connect }
        }
        "areaChart" => {
            let grouping = wrapper
                .child_by_local("grouping")
                .and_then(|el| el.attr_local("val"))
                .unwrap_or("standard");
            if grouping != "standard" {
                return Err(unsupported());
            }
            if flag(wrapper, "varyColors").unwrap_or(false) {
                return Err(unsupported());
            }
            for child in wrapper.element_children() {
                if matches!(child.local_name(), "dropLines") {
                    return Err(unsupported());
                }
            }
            data_labels_hidden(wrapper.child_by_local("dLbls"))?;
            let blanks = chart
                .child_by_local("dispBlanksAs")
                .and_then(|el| el.attr_local("val"))
                .unwrap_or("gap");
            if blanks != "gap" {
                return Err(unsupported());
            }
            PlotKind::StandardArea
        }
        "pieChart" => {
            if wrapper
                .element_children()
                .filter(|el| el.local_name() == "ser")
                .count()
                != 1
            {
                return Err(unsupported());
            }
            data_labels_hidden(wrapper.child_by_local("dLbls"))?;
            let blanks = chart
                .child_by_local("dispBlanksAs")
                .and_then(|el| el.attr_local("val"))
                .unwrap_or("gap");
            if blanks != "gap" {
                return Err(unsupported());
            }
            if plot
                .element_children()
                .any(|el| matches!(el.local_name(), "valAx" | "catAx" | "dateAx" | "serAx"))
            {
                return Err(unsupported());
            }
            PlotKind::Pie
        }
        _ => return Err(unsupported()),
    };
    let vary = if kind == PlotKind::Pie {
        flag(wrapper, "varyColors").unwrap_or(true)
    } else {
        flag(wrapper, "varyColors").unwrap_or(false)
    };
    let gap_width = number_val(wrapper, "gapWidth").unwrap_or(150.0);
    let overlap = number_val(wrapper, "overlap").unwrap_or(0.0);
    let mut series = Vec::new();
    for ser in wrapper
        .element_children()
        .filter(|el| el.local_name() == "ser")
    {
        let order = number_val(ser, "order").unwrap_or(series.len() as f64) as usize;
        let (values, x_values) = if matches!(kind, PlotKind::Scatter { .. }) {
            let ys = series_numbers(ser.child_by_local("yVal"), sheet, cells);
            let xs = series_numbers(ser.child_by_local("xVal"), sheet, cells);
            if xs.is_empty() && ys.is_empty() {
                continue;
            }
            if !xs.iter().any(|v| v.is_some_and(|n| n.is_finite()))
                || !ys.iter().any(|v| v.is_some_and(|n| n.is_finite()))
            {
                return Err(unsupported());
            }
            (ys, xs)
        } else {
            let values = series_numbers(ser.child_by_local("val"), sheet, cells);
            if values.is_empty() {
                continue;
            }
            (values, Vec::new())
        };
        let name = series_name(ser, sheet, cells).unwrap_or_else(|| format!("系列{}", order + 1));
        let mut line_color = None;
        let style = match kind {
            PlotKind::ClusteredColumn | PlotKind::ClusteredBar => LineSeriesStyle {
                color: series_color(ser, accents),
                visible: false,
                width: 0.0,
                marker: MarkerKind::None,
                marker_color: None,
            },
            PlotKind::StandardLine => read_line_series(ser, accents, dpi, order)?,
            PlotKind::StandardArea => {
                let area = read_area_series(ser, accents, dpi)?;
                line_color = area.line_color;
                LineSeriesStyle {
                    color: area.fill,
                    visible: area.line_visible,
                    width: area.line_width,
                    marker: MarkerKind::None,
                    marker_color: None,
                }
            }
            PlotKind::Pie => read_pie_series_style(ser, accents, dpi)?,
            PlotKind::Scatter { connect } => {
                let mut style = read_line_series(ser, accents, dpi, order)?;
                if !connect {
                    style.visible = false;
                }
                style
            }
        };
        if series.len() <= order {
            series.resize_with(order + 1, Series::default);
        }
        series[order] = Series {
            name,
            values,
            color: style.color,
            slot: order,
            line_visible: style.visible,
            line_width: style.width,
            marker: style.marker,
            marker_color: style.marker_color,
            line_color,
            x_values,
        };
    }
    series.retain(|ser| !ser.values.is_empty() || !ser.x_values.is_empty());
    if series.is_empty() {
        return Err(unsupported());
    }
    let n = series
        .iter()
        .map(|ser| ser.values.len().max(ser.x_values.len()))
        .max()
        .unwrap_or(0);
    if n == 0 {
        return Err(unsupported());
    }
    let mut categories = if matches!(kind, PlotKind::Scatter { .. }) {
        Vec::new()
    } else {
        cache_labels(series_cat(wrapper))
    };
    if categories.is_empty() && !matches!(kind, PlotKind::Scatter { .. }) {
        if let Some(text) = formula_text(series_cat(wrapper)) {
            if let Some(labels) = labels_from_cells(&text, sheet, cells) {
                categories = labels;
            }
        }
    }
    if !matches!(kind, PlotKind::Scatter { .. }) {
        if categories.len() < n {
            let start = categories.len();
            categories.extend((start..n).map(|i| (i + 1).to_string()));
        }
        categories.truncate(n);
    }
    for ser in &mut series {
        ser.values.resize(n, None);
        if matches!(kind, PlotKind::Scatter { .. }) {
            ser.x_values.resize(n, None);
        }
    }
    let grid = plot.element_children().any(|el| {
        matches!(el.local_name(), "valAx" | "dateAx")
            && el.child_by_local("majorGridlines").is_some()
    });
    let between = plot
        .element_children()
        .filter(|el| el.local_name() == "valAx")
        .all(|el| {
            el.child_by_local("crossBetween")
                .and_then(|c| c.attr_local("val"))
                .unwrap_or("between")
                == "between"
        });
    if !between {
        return Err(unsupported());
    }
    let vary_points = (matches!(kind, PlotKind::ClusteredColumn | PlotKind::ClusteredBar)
        && vary
        && series.len() == 1)
        || (kind == PlotKind::Pie && vary);
    let first_slice_ang = if kind == PlotKind::Pie {
        pie_first_slice_ang(wrapper)?
    } else {
        0.0
    };
    let point_colors = if kind == PlotKind::Pie {
        if series.len() != 1 {
            return Err(unsupported());
        }
        let ser = wrapper
            .element_children()
            .find(|el| el.local_name() == "ser")
            .ok_or_else(unsupported)?;
        read_pie_point_colors(ser, accents, n, dpi)?
    } else {
        Vec::new()
    };
    if kind == PlotKind::Pie {
        let values = &series[0].values;
        if values
            .iter()
            .any(|v| v.is_some_and(|n| n.is_finite() && n < 0.0))
        {
            return Err(unsupported());
        }
        if !values
            .iter()
            .any(|v| v.is_some_and(|n| n.is_finite() && n > 0.0))
        {
            return Err(unsupported());
        }
        if legend_pos(chart).is_some() && !legend::Spec::read(root, accents)?.default_right_ink() {
            return Err(unsupported());
        }
    }
    let (axis, axis_x, grid_x, cats_from_bottom) = if matches!(kind, PlotKind::Pie) {
        (axis::Spec::default(), None, false, true)
    } else if matches!(kind, PlotKind::Scatter { .. }) {
        let (x, y) = axis::scatter_axes(plot)?;
        let grid_x = plot.element_children().any(|el| {
            el.local_name() == "valAx"
                && matches!(
                    el.child_by_local("axPos")
                        .and_then(|p| p.attr_local("val"))
                        .unwrap_or(""),
                    "b" | "t"
                )
                && el.child_by_local("majorGridlines").is_some()
        });
        (y, Some(x), grid_x, true)
    } else if kind == PlotKind::ClusteredBar {
        let (spec, from_bottom) = axis::bar_axes(plot)?;
        (spec, None, false, from_bottom)
    } else {
        (axis::Spec::read(plot)?, None, false, true)
    };
    Ok(ColumnChart {
        categories,
        series,
        gap_width,
        overlap,
        vary_points,
        grid,
        grid_x,
        legend: legend_pos(chart),
        legend_spec: legend::Spec::read(root, accents)?,
        axis,
        axis_x,
        cats_from_bottom,
        title: title_text(chart),
        accents,
        kind,
        first_slice_ang,
        point_colors,
    })
}

fn series_cat(bar: &Element) -> Option<&Element> {
    bar.element_children()
        .find(|el| el.local_name() == "ser")
        .and_then(|ser| ser.child_by_local("cat"))
}

fn is_chart_wrapper(name: &str) -> bool {
    matches!(
        name,
        "barChart"
            | "bar3DChart"
            | "lineChart"
            | "line3DChart"
            | "pieChart"
            | "pie3DChart"
            | "doughnutChart"
            | "ofPieChart"
            | "areaChart"
            | "area3DChart"
            | "scatterChart"
            | "radarChart"
            | "bubbleChart"
            | "stockChart"
            | "surfaceChart"
            | "surface3DChart"
    )
}

fn paint_scatter(
    canvas: &mut Canvas,
    chart: &ColumnChart,
    font: Option<&Font>,
    host: ChartHost,
) -> Result<(), Error> {
    let unsupported = || Error::with_path(ErrorKind::ChartImageUnsupported, "id");
    let legend_layout = legend::Layout::new(canvas, chart, font)?;
    let label_px = canvas.dpi.pt_to_px_font(LABEL_PT) as f32;
    let gdi = super::chart_gdi::Face::open(label_px, false);
    let title_face = chart
        .title
        .as_ref()
        .and_then(|_| super::chart_gdi::Face::open(label_px, true));
    let em = text_height(font, label_px).max(gdi.as_ref().map_or(0.0, |f| f.em_box()));
    let y_scale = axis::scale(
        &chart.series,
        chart.axis,
        axis::interval_budget_dpi(canvas.h, em, canvas.dpi),
    )?;
    let y_ticks = tick_values(y_scale.min, y_scale.max, y_scale.major);
    let y_labels: Vec<String> = y_ticks.iter().copied().map(fmt_tick).collect();
    let y_intervals = y_ticks.len().saturating_sub(1).max(1) as i32;
    let widest_tick = y_labels
        .iter()
        .map(|label| text_width(font, gdi.as_ref(), label, label_px).ceil() as i32)
        .max()
        .unwrap_or(0);
    let mut axis_x = axis_x_of(widest_tick, canvas.dpi);
    let mut plot_x = axis_x + 1;
    let title_top = canvas.dpi.scale_f(4.0);
    let mut plot_y = if chart.title.is_some() {
        let band = title_face
            .as_ref()
            .map(|face| face.em_box().ceil() as i32)
            .unwrap_or_else(|| text_height(font, label_px).ceil() as i32);
        let half = gdi
            .as_ref()
            .map(|face| face.line_height().ceil() as i32 / 2)
            .unwrap_or(0);
        let mut y = title_top as i32 + 2 * (band + half);
        if matches!(host, ChartHost::Slide) {
            y += canvas.dpi.scale_px(TITLE_DROP as i32);
        }
        y
    } else {
        canvas.dpi.scale_px(INSET_TOP)
    };
    let inset_right = legend_right_inset(chart, font, gdi.as_ref(), label_px, canvas.dpi);
    let mut plot_r = canvas.w as i32 - inset_right;
    let mut plot_b = canvas.h as i32 - canvas.dpi.scale_px(INSET_BOTTOM);
    if !chart.legend_spec.legacy {
        let reserve = legend_layout.reserve();
        axis_x += reserve[0];
        plot_x = axis_x + 1;
        plot_y += reserve[1];
        plot_r = canvas.w as i32 - canvas.dpi.scale_px(INSET_TOP) - reserve[2];
        plot_b -= reserve[3];
        if plot_r <= plot_x || plot_b <= plot_y {
            return Err(legend::unsupported());
        }
    }
    let x_spec = chart.axis_x.ok_or_else(unsupported)?;
    let x_budget = axis::interval_budget_span(f64::from((plot_r - plot_x).max(1)), em, 3.0);
    let x_scale = axis::scale_values(
        chart.series.iter().flat_map(|s| s.x_values.iter().copied()),
        x_spec,
        x_budget,
    )?;
    let x_ticks = tick_values(x_scale.min, x_scale.max, x_scale.major);
    let x_labels: Vec<String> = x_ticks.iter().copied().map(fmt_tick).collect();
    let plot = Rect {
        x: f64::from(plot_x),
        y: f64::from(plot_y),
        w: f64::from(plot_r - plot_x),
        h: f64::from(plot_b - plot_y),
    };
    let y_regular = (((y_scale.max - y_scale.min) / y_scale.major).round() * y_scale.major
        - (y_scale.max - y_scale.min))
        .abs()
        <= y_scale.major * 1e-8;
    if chart.grid && y_regular {
        for i in 0..=y_intervals {
            let y = plot_y + (i * (plot_b - plot_y) + y_intervals - 1) / y_intervals;
            canvas.hline_i(axis_x - canvas.dpi.scale_px(TICK_OUT), plot_r, y, INK);
        }
    }
    if chart.grid && !y_regular {
        for value in &y_ticks {
            let y = y_of(*value, y_scale.min, y_scale.max, &plot).ceil() as i32;
            canvas.hline_i(axis_x - canvas.dpi.scale_px(TICK_OUT), plot_r, y, INK);
        }
    }
    if chart.grid_x {
        for value in &x_ticks {
            let x = x_of(*value, x_scale.min, x_scale.max, &plot).round() as i32;
            canvas.vline_i(x, plot_y, plot_b, INK);
        }
    }
    let y_parts = y_scale.major / y_scale.minor;
    if y_regular && (y_parts - y_parts.round()).abs() < 1e-8 && (1.0..=100.0).contains(&y_parts) {
        draw_value_minor_ticks(
            canvas,
            axis_x,
            plot_y,
            plot_b,
            y_intervals,
            y_parts.round() as i32,
        );
    } else {
        for value in tick_values(y_scale.min + y_scale.minor, y_scale.max, y_scale.minor) {
            let at_major = (value - y_scale.min) / y_scale.major;
            if (at_major - at_major.round()).abs() < 1e-8 {
                continue;
            }
            let y = y_of(value, y_scale.min, y_scale.max, &plot).round() as i32;
            canvas.hline_i(
                axis_x - canvas.dpi.scale_px(VAL_MINOR_ARM),
                axis_x + canvas.dpi.scale_px(VAL_MINOR_ARM),
                y,
                INK,
            );
        }
    }
    draw_scatter(canvas, chart, &plot, &x_scale, &y_scale);
    let axis_line_x = x_of(x_scale.crossing(), x_scale.min, x_scale.max, &plot).round() as i32;
    let axis_line_y = y_of(y_scale.crossing(), y_scale.min, y_scale.max, &plot).round() as i32;
    canvas.vline_i(axis_line_x, plot_y, plot_b, INK);
    canvas.hline_i(axis_x, plot_r, axis_line_y, INK);
    for value in &x_ticks {
        let x = x_of(*value, x_scale.min, x_scale.max, &plot).round() as i32;
        canvas.vline_i(
            x,
            axis_line_y - canvas.dpi.scale_px(CAT_MAJOR_IN),
            axis_line_y + canvas.dpi.scale_px(CAT_MAJOR),
            INK,
        );
    }
    for value in tick_values(x_scale.min + x_scale.minor, x_scale.max, x_scale.minor) {
        let at_major = (value - x_scale.min) / x_scale.major;
        if (at_major - at_major.round()).abs() < 1e-8 {
            continue;
        }
        let x = x_of(value, x_scale.min, x_scale.max, &plot).round() as i32;
        canvas.vline_i(
            x,
            axis_line_y - canvas.dpi.scale_px(VAL_MINOR_ARM),
            axis_line_y + canvas.dpi.scale_px(VAL_MINOR_ARM),
            INK,
        );
    }
    for (tick, label) in y_ticks.iter().zip(&y_labels) {
        let y = y_of(*tick, y_scale.min, y_scale.max, &plot);
        let w = text_width(font, gdi.as_ref(), label, label_px);
        let h = text_height(font, label_px);
        draw_text(
            canvas,
            font,
            gdi.as_ref(),
            label,
            f64::from(axis_x) - w - f64::from(canvas.dpi.scale_px(TICK_GAP)),
            y - h / 2.0,
            label_px,
            INK,
        );
    }
    let baseline =
        f64::from(canvas.h as i32 - canvas.dpi.scale_px(INSET_BOTTOM)) + canvas.dpi.scale_f(10.0);
    for (tick, label) in x_ticks.iter().zip(&x_labels) {
        let x = x_of(*tick, x_scale.min, x_scale.max, &plot);
        let w = text_width(font, gdi.as_ref(), label, label_px);
        draw_text(
            canvas,
            font,
            gdi.as_ref(),
            label,
            x - w / 2.0,
            baseline,
            label_px,
            INK,
        );
    }
    if let Some(title) = &chart.title {
        let face = title_face.as_ref().or(gdi.as_ref());
        let width = text_width(font, face, title, label_px);
        let x = (f64::from(canvas.w) - width) / 2.0;
        draw_text(
            canvas,
            font,
            face,
            title,
            x,
            title_top + canvas.dpi.scale_f(TITLE_DROP),
            label_px,
            INK,
        );
    }
    if chart.legend_spec.default_right_ink() {
        let legend_plot_r = canvas.w as i32 - inset_right;
        draw_legend(canvas, chart, font, gdi.as_ref(), label_px, legend_plot_r);
    } else {
        legend_layout.draw(canvas, chart, font);
    }
    if matches!(host, ChartHost::Excel) {
        canvas.stroke_border(BORDER);
    }
    Ok(())
}

fn x_of(value: f64, axis_min: f64, axis_max: f64, plot: &Rect) -> f64 {
    let span = (axis_max - axis_min).max(1e-12);
    let t = (value - axis_min) / span;
    plot.x + t * plot.w
}

fn draw_scatter(
    canvas: &mut Canvas,
    chart: &ColumnChart,
    plot: &Rect,
    x_scale: &axis::Scale,
    y_scale: &axis::Scale,
) {
    for ser in &chart.series {
        let color = ser.color.unwrap_or(chart.accents[ser.slot % 6]);
        let n = ser.values.len().max(ser.x_values.len());
        let mut run = Vec::new();
        let mut points = Vec::new();
        for i in 0..n {
            let x = ser.x_values.get(i).and_then(|v| *v);
            let y = ser.values.get(i).and_then(|v| *v);
            match (x, y) {
                (Some(x), Some(y)) if x.is_finite() && y.is_finite() => {
                    let px = x_of(
                        x.clamp(x_scale.min, x_scale.max),
                        x_scale.min,
                        x_scale.max,
                        plot,
                    );
                    let py = y_of(
                        y.clamp(y_scale.min, y_scale.max),
                        y_scale.min,
                        y_scale.max,
                        plot,
                    );
                    run.push((px, py));
                    points.push((px, py));
                }
                _ => {
                    if ser.line_visible && ser.line_width > 0.0 {
                        stroke_run(canvas, &run, ser.line_width, color);
                    }
                    run.clear();
                }
            }
        }
        if ser.line_visible && ser.line_width > 0.0 {
            stroke_run(canvas, &run, ser.line_width, color);
        }
        if !matches!(ser.marker, MarkerKind::None) {
            let marker_color = ser.marker_color.unwrap_or(color);
            for (px, py) in points {
                paint_marker(canvas, ser.marker, px, py, marker_color);
            }
        }
    }
}

fn paint_bar(
    canvas: &mut Canvas,
    chart: &ColumnChart,
    font: Option<&Font>,
    host: ChartHost,
) -> Result<(), Error> {
    let legend_layout = legend::Layout::new(canvas, chart, font)?;
    let label_px = canvas.dpi.pt_to_px_font(LABEL_PT) as f32;
    let gdi = super::chart_gdi::Face::open(label_px, false);
    let title_face = chart
        .title
        .as_ref()
        .and_then(|_| super::chart_gdi::Face::open(label_px, true));
    let em = text_height(font, label_px).max(gdi.as_ref().map_or(0.0, |f| f.em_box()));
    let widest_cat = chart
        .categories
        .iter()
        .map(|label| text_width(font, gdi.as_ref(), label, label_px).ceil() as i32)
        .max()
        .unwrap_or(0);
    let mut axis_x = axis_x_of(widest_cat, canvas.dpi);
    let mut plot_x = axis_x + 1;
    let title_top = canvas.dpi.scale_f(4.0);
    let mut plot_y = if chart.title.is_some() {
        let band = title_face
            .as_ref()
            .map(|face| face.em_box().ceil() as i32)
            .unwrap_or_else(|| text_height(font, label_px).ceil() as i32);
        let half = gdi
            .as_ref()
            .map(|face| face.line_height().ceil() as i32 / 2)
            .unwrap_or(0);
        let mut y = title_top as i32 + 2 * (band + half);
        if matches!(host, ChartHost::Slide) {
            y += canvas.dpi.scale_px(TITLE_DROP as i32);
        }
        y
    } else {
        canvas.dpi.scale_px(INSET_TOP)
    };
    let inset_right = legend_right_inset(chart, font, gdi.as_ref(), label_px, canvas.dpi);
    let mut plot_r = canvas.w as i32 - inset_right;
    let mut plot_b = canvas.h as i32 - canvas.dpi.scale_px(INSET_BOTTOM);
    if !chart.legend_spec.legacy {
        let reserve = legend_layout.reserve();
        axis_x += reserve[0];
        plot_x = axis_x + 1;
        plot_y += reserve[1];
        plot_r = canvas.w as i32 - canvas.dpi.scale_px(INSET_TOP) - reserve[2];
        plot_b -= reserve[3];
        if plot_r <= plot_x || plot_b <= plot_y {
            return Err(legend::unsupported());
        }
    }
    let plot = Rect {
        x: f64::from(plot_x),
        y: f64::from(plot_y),
        w: f64::from(plot_r - plot_x),
        h: f64::from(plot_b - plot_y),
    };
    // 数值轴在水平方向，间隔预算按绘图区宽度，occupancy=3，与散点 X 轴同一套公式。
    let scale = axis::scale(
        &chart.series,
        chart.axis,
        axis::interval_budget_span(plot.w, em, 3.0),
    )?;
    let (axis_min, axis_max, step) = (scale.min, scale.max, scale.major);
    let ticks = tick_values(axis_min, axis_max, step);
    let tick_labels: Vec<String> = ticks.iter().copied().map(fmt_tick).collect();
    let intervals = ticks.len().saturating_sub(1).max(1) as i32;
    let regular = (((axis_max - axis_min) / step).round() * step - (axis_max - axis_min)).abs()
        <= step * 1e-8;
    if chart.grid && regular {
        for i in 0..=intervals {
            let x = plot_x + (i * (plot_r - plot_x) + intervals - 1) / intervals;
            canvas.vline_i(x, plot_y, plot_b, INK);
        }
    }
    if chart.grid && !regular {
        for value in &ticks {
            let x = x_of(*value, axis_min, axis_max, &plot).round() as i32;
            canvas.vline_i(x, plot_y, plot_b, INK);
        }
    }
    let parts = step / scale.minor;
    if regular && (parts - parts.round()).abs() < 1e-8 && (1.0..=100.0).contains(&parts) {
        draw_value_minor_ticks_x(
            canvas,
            plot_x,
            plot_r,
            plot_b,
            intervals,
            parts.round() as i32,
        );
    } else {
        for value in tick_values(axis_min + scale.minor, axis_max, scale.minor) {
            let at_major = (value - axis_min) / step;
            if (at_major - at_major.round()).abs() < 1e-8 {
                continue;
            }
            let x = x_of(value, axis_min, axis_max, &plot).round() as i32;
            canvas.vline_i(
                x,
                plot_b - canvas.dpi.scale_px(VAL_MINOR_ARM),
                plot_b + canvas.dpi.scale_px(VAL_MINOR_ARM),
                INK,
            );
        }
    }
    draw_clustered_bars(canvas, chart, &plot, axis_min, axis_max);
    let zero_x = x_of(scale.crossing(), axis_min, axis_max, &plot).round() as i32;
    canvas.vline_i(zero_x, plot_y, plot_b, INK);
    canvas.hline_i(plot_x, plot_r, plot_b, INK);
    draw_cat_ticks_y(
        canvas,
        chart.categories.len(),
        plot.y,
        plot.y + plot.h + 1.0,
        zero_x,
        chart.cats_from_bottom,
    );
    // 类目标签在左侧 inset，数值标签在底轴下方。
    for (i, label) in chart.categories.iter().enumerate() {
        let mid = category_mid_y(&plot, i, chart.categories.len(), chart.cats_from_bottom);
        let w = text_width(font, gdi.as_ref(), label, label_px);
        let h = text_height(font, label_px);
        draw_text(
            canvas,
            font,
            gdi.as_ref(),
            label,
            f64::from(axis_x) - w - f64::from(canvas.dpi.scale_px(TICK_GAP)),
            mid - h / 2.0,
            label_px,
            INK,
        );
    }
    let baseline = f64::from(plot_b) + canvas.dpi.scale_f(10.0);
    for (tick, label) in ticks.iter().zip(&tick_labels) {
        let w = text_width(font, gdi.as_ref(), label, label_px);
        let x = x_of(*tick, axis_min, axis_max, &plot) - w / 2.0;
        draw_text(
            canvas,
            font,
            gdi.as_ref(),
            label,
            x,
            baseline,
            label_px,
            INK,
        );
    }
    if let Some(title) = &chart.title {
        let face = title_face.as_ref().or(gdi.as_ref());
        let width = text_width(font, face, title, label_px);
        let x = (f64::from(canvas.w) - width) / 2.0;
        draw_text(
            canvas,
            font,
            face,
            title,
            x,
            title_top + canvas.dpi.scale_f(TITLE_DROP),
            label_px,
            INK,
        );
    }
    if chart.legend_spec.default_right_ink() {
        let legend_plot_r = canvas.w as i32 - inset_right;
        draw_legend(canvas, chart, font, gdi.as_ref(), label_px, legend_plot_r);
    } else {
        legend_layout.draw(canvas, chart, font);
    }
    if matches!(host, ChartHost::Excel) {
        canvas.stroke_border(BORDER);
    }
    Ok(())
}

fn paint_pie(
    canvas: &mut Canvas,
    chart: &ColumnChart,
    font: Option<&Font>,
    host: ChartHost,
) -> Result<(), Error> {
    let unsupported = || Error::with_path(ErrorKind::ChartImageUnsupported, "id");
    if chart.series.len() != 1 {
        return Err(unsupported());
    }
    if chart.legend.is_some() && !chart.legend_spec.default_right_ink() {
        return Err(unsupported());
    }
    let label_px = canvas.dpi.pt_to_px_font(LABEL_PT) as f32;
    let gdi = super::chart_gdi::Face::open(label_px, false);
    let title_face = chart
        .title
        .as_ref()
        .and_then(|_| super::chart_gdi::Face::open(label_px, true));
    let title_top = canvas.dpi.scale_f(4.0);
    let plot_y = if chart.title.is_some() {
        let band = title_face
            .as_ref()
            .map(|face| face.em_box().ceil() as i32)
            .unwrap_or_else(|| text_height(font, label_px).ceil() as i32);
        let half = gdi
            .as_ref()
            .map(|face| face.line_height().ceil() as i32 / 2)
            .unwrap_or(0);
        let mut y = title_top as i32 + 2 * (band + half);
        if matches!(host, ChartHost::Slide) {
            y += canvas.dpi.scale_px(TITLE_DROP as i32);
        }
        y
    } else {
        canvas.dpi.scale_px(INSET_TOP)
    };
    let inset_right = if chart.legend == Some(LegendPos::Right) {
        pie_legend_right_inset(chart, font, gdi.as_ref(), label_px, canvas.dpi)
    } else {
        canvas.dpi.scale_px(INSET_TOP)
    };
    let plot_x = canvas.dpi.scale_px(INSET_TOP);
    let plot_r = canvas.w as i32 - inset_right;
    let plot_b = canvas.h as i32 - canvas.dpi.scale_px(INSET_TOP);
    if plot_r <= plot_x || plot_b <= plot_y {
        return Err(unsupported());
    }
    let plot = Rect {
        x: f64::from(plot_x),
        y: f64::from(plot_y),
        w: f64::from(plot_r - plot_x),
        h: f64::from(plot_b - plot_y),
    };
    draw_pie_slices(canvas, chart, &plot);
    if let Some(title) = &chart.title {
        let face = title_face.as_ref().or(gdi.as_ref());
        let width = text_width(font, face, title, label_px);
        let x = (f64::from(canvas.w) - width) / 2.0;
        draw_text(
            canvas,
            font,
            face,
            title,
            x,
            title_top + canvas.dpi.scale_f(TITLE_DROP),
            label_px,
            INK,
        );
    }
    if chart.legend == Some(LegendPos::Right) {
        let legend_plot_r = canvas.w as i32 - inset_right;
        draw_pie_legend(canvas, chart, font, gdi.as_ref(), label_px, legend_plot_r);
    }
    if matches!(host, ChartHost::Excel) {
        canvas.stroke_border(BORDER);
    }
    Ok(())
}

fn pie_slice_color(chart: &ColumnChart, index: usize) -> [u8; 3] {
    if let Some(Some(color)) = chart.point_colors.get(index) {
        return *color;
    }
    if chart.vary_points {
        chart.accents[index % 6]
    } else {
        chart.series[0]
            .color
            .unwrap_or(chart.accents[chart.series[0].slot % 6])
    }
}

fn pie_legend_items(chart: &ColumnChart) -> Vec<(String, [u8; 3])> {
    let ser = &chart.series[0];
    chart
        .categories
        .iter()
        .enumerate()
        .filter_map(
            |(index, name)| match ser.values.get(index).copied().flatten() {
                Some(value) if value.is_finite() && value >= 0.0 => {
                    Some((name.clone(), pie_slice_color(chart, index)))
                }
                _ => None,
            },
        )
        .collect()
}

fn pie_legend_right_inset(
    chart: &ColumnChart,
    font: Option<&Font>,
    gdi: Option<&super::chart_gdi::Face>,
    px: f32,
    dpi: RenderDpi,
) -> i32 {
    let from_layout = gdi.is_some();
    let widest = pie_legend_items(chart)
        .into_iter()
        .map(|(name, _)| match gdi {
            Some(face) => face.layout_width(&name),
            None => text_width(font, None, &name, px),
        })
        .fold(0.0, f64::max);
    let name_px = if from_layout {
        widest as i32
    } else {
        widest.ceil() as i32
    };
    let need = dpi.scale_px(LEGEND_PLOT_GAP)
        + dpi.scale_px(LEGEND_SWATCH)
        + dpi.scale_px(LEGEND_GAP)
        + name_px
        + dpi.scale_px(LEGEND_PAD);
    need.max(dpi.scale_px(INSET_RIGHT))
}

fn draw_pie_legend(
    canvas: &mut Canvas,
    chart: &ColumnChart,
    font: Option<&Font>,
    gdi: Option<&super::chart_gdi::Face>,
    px: f32,
    plot_r: i32,
) {
    let items = pie_legend_items(chart);
    let n = items.len() as i32;
    if n == 0 {
        return;
    }
    let pitch = canvas.dpi.scale_px(24);
    let block = pitch * (n - 1) + canvas.dpi.scale_px(LEGEND_SWATCH);
    let title_band = if chart.title.is_some() {
        canvas.dpi.scale_px(LEGEND_TITLE_BAND)
    } else {
        0
    };
    let mut y = (title_band + canvas.h as i32 - block) / 2;
    let x = plot_r + canvas.dpi.scale_px(LEGEND_PLOT_GAP);
    for (name, color) in items {
        draw_plot_key(
            canvas,
            PlotKind::Pie,
            x,
            y,
            x + canvas.dpi.scale_px(LEGEND_SWATCH),
            y + canvas.dpi.scale_px(LEGEND_SWATCH),
            color,
            MarkerKind::None,
        );
        draw_text(
            canvas,
            font,
            gdi,
            &name,
            f64::from(x + canvas.dpi.scale_px(LEGEND_SWATCH) + canvas.dpi.scale_px(LEGEND_GAP)),
            f64::from(y - canvas.dpi.scale_px(2)),
            px,
            INK,
        );
        y += pitch;
    }
}

fn draw_pie_slices(canvas: &mut Canvas, chart: &ColumnChart, plot: &Rect) {
    let ser = &chart.series[0];
    let sum: f64 = ser
        .values
        .iter()
        .flatten()
        .filter(|value| value.is_finite() && **value > 0.0)
        .sum();
    if !sum.is_finite() || sum <= 0.0 {
        return;
    }
    let cx = plot.x + plot.w / 2.0;
    let cy = plot.y + plot.h / 2.0;
    let radius = plot.w.min(plot.h) / 2.0;
    if radius <= 0.5 {
        return;
    }
    let mut start = chart.first_slice_ang.rem_euclid(360.0);
    for (index, value) in ser.values.iter().enumerate() {
        let Some(value) = *value else {
            continue;
        };
        if !value.is_finite() || value <= 0.0 {
            continue;
        }
        let end = start + 360.0 * (value / sum);
        canvas.fill_wedge(cx, cy, radius, start, end, pie_slice_color(chart, index));
        start = end;
    }
}

fn paint(
    canvas: &mut Canvas,
    chart: &ColumnChart,
    font: Option<&Font>,
    host: ChartHost,
) -> Result<(), Error> {
    if matches!(chart.kind, PlotKind::Scatter { .. }) {
        return paint_scatter(canvas, chart, font, host);
    }
    if chart.kind == PlotKind::ClusteredBar {
        return paint_bar(canvas, chart, font, host);
    }
    if chart.kind == PlotKind::Pie {
        return paint_pie(canvas, chart, font, host);
    }
    let legend_layout = legend::Layout::new(canvas, chart, font)?;
    let label_px = canvas.dpi.pt_to_px_font(LABEL_PT) as f32;
    let gdi = super::chart_gdi::Face::open(label_px, false);
    let title_face = chart
        .title
        .as_ref()
        .and_then(|_| super::chart_gdi::Face::open(label_px, true));
    // Use the full supplied font line box, not just the GDI raster ascent.
    // Glyph drawing can use the existing GDI fallback; label packing must reserve
    // the larger declared line box so the two paths cannot over-pack the axis.
    let em = text_height(font, label_px).max(gdi.as_ref().map_or(0.0, |f| f.em_box()));
    let scale = axis::scale(
        &chart.series,
        chart.axis,
        axis::interval_budget_dpi(canvas.h, em, canvas.dpi),
    )?;
    let (axis_min, axis_max, step) = (scale.min, scale.max, scale.major);
    let ticks = tick_values(axis_min, axis_max, step);
    let tick_labels: Vec<String> = ticks.iter().copied().map(fmt_tick).collect();
    let intervals = ticks.len().saturating_sub(1).max(1) as i32;
    let widest_tick = tick_labels
        .iter()
        .map(|label| text_width(font, gdi.as_ref(), label, label_px).ceil() as i32)
        .max()
        .unwrap_or(0);
    let mut axis_x = axis_x_of(widest_tick, canvas.dpi);
    let mut plot_x = axis_x + 1;
    let title_top = canvas.dpi.scale_f(4.0);
    let mut plot_y = if chart.title.is_some() {
        let band = title_face
            .as_ref()
            .map(|face| face.em_box().ceil() as i32)
            .unwrap_or_else(|| text_height(font, label_px).ceil() as i32);
        let half = gdi
            .as_ref()
            .map(|face| face.line_height().ceil() as i32 / 2)
            .unwrap_or(0);
        // 标题占一行，下面再空一行，顶上的刻度才不压住标题。
        let mut y = title_top as i32 + 2 * (band + half);
        if matches!(host, ChartHost::Slide) {
            y += canvas.dpi.scale_px(TITLE_DROP as i32);
        }
        y
    } else {
        canvas.dpi.scale_px(INSET_TOP)
    };
    let inset_right = legend_right_inset(chart, font, gdi.as_ref(), label_px, canvas.dpi);
    let mut plot_r = canvas.w as i32 - inset_right;
    let mut plot_b = canvas.h as i32 - canvas.dpi.scale_px(INSET_BOTTOM);
    if !chart.legend_spec.legacy {
        let reserve = legend_layout.reserve();
        axis_x += reserve[0];
        plot_x = axis_x + 1;
        plot_y += reserve[1];
        plot_r = canvas.w as i32 - canvas.dpi.scale_px(INSET_TOP) - reserve[2];
        plot_b -= reserve[3];
        if plot_r <= plot_x || plot_b <= plot_y {
            return Err(legend::unsupported());
        }
    }
    let plot = Rect {
        x: f64::from(plot_x),
        y: f64::from(plot_y),
        w: f64::from(plot_r - plot_x),
        h: f64::from(plot_b - plot_y),
    };
    let regular = (((axis_max - axis_min) / step).round() * step - (axis_max - axis_min)).abs()
        <= step * 1e-8;
    if chart.grid && regular {
        for i in 0..=intervals {
            let y = plot_y + (i * (plot_b - plot_y) + intervals - 1) / intervals;
            canvas.hline_i(axis_x - canvas.dpi.scale_px(TICK_OUT), plot_r, y, INK);
        }
    }
    if chart.grid && !regular {
        for value in &ticks {
            let y = y_of(*value, axis_min, axis_max, &plot).ceil() as i32;
            canvas.hline_i(axis_x - canvas.dpi.scale_px(TICK_OUT), plot_r, y, INK);
        }
    }
    let parts = step / scale.minor;
    if regular && (parts - parts.round()).abs() < 1e-8 && (1.0..=100.0).contains(&parts) {
        draw_value_minor_ticks(
            canvas,
            axis_x,
            plot_y,
            plot_b,
            intervals,
            parts.round() as i32,
        );
    } else {
        for value in tick_values(axis_min + scale.minor, axis_max, scale.minor) {
            let at_major = (value - axis_min) / step;
            if (at_major - at_major.round()).abs() < 1e-8 {
                continue;
            }
            let y = y_of(value, axis_min, axis_max, &plot).round() as i32;
            canvas.hline_i(
                axis_x - canvas.dpi.scale_px(VAL_MINOR_ARM),
                axis_x + canvas.dpi.scale_px(VAL_MINOR_ARM),
                y,
                INK,
            );
        }
    }
    match chart.kind {
        PlotKind::ClusteredColumn => draw_bars(canvas, chart, &plot, axis_min, axis_max),
        PlotKind::StandardLine => draw_lines(canvas, chart, &plot, axis_min, axis_max),
        PlotKind::StandardArea => draw_areas(canvas, chart, &plot, axis_min, axis_max),
        PlotKind::ClusteredBar | PlotKind::Pie | PlotKind::Scatter { .. } => {
            unreachable!("bar/pie/scatter charts use dedicated paint")
        }
    }
    canvas.vline_i(axis_x, plot_y, plot_b, INK);
    // 次刻度在类目正中，会穿过柱底。先画柱再画刻度，向上的那一截才留得住。
    let category_y = y_of(scale.crossing(), axis_min, axis_max, &plot).round() as i32;
    canvas.hline_i(axis_x, plot_r, category_y, INK);
    draw_cat_ticks(
        canvas,
        chart.categories.len(),
        f64::from(axis_x),
        f64::from(plot_r + 1),
        category_y,
    );
    draw_tick_labels(
        canvas,
        font,
        gdi.as_ref(),
        label_px,
        &plot,
        axis_x,
        &ticks,
        &tick_labels,
        axis_min,
        axis_max,
        &chart.categories,
        category_y,
    );
    if let Some(title) = &chart.title {
        let face = title_face.as_ref().or(gdi.as_ref());
        let width = text_width(font, face, title, label_px);
        let x = (f64::from(canvas.w) - width) / 2.0;
        draw_text(
            canvas,
            font,
            face,
            title,
            x,
            title_top + canvas.dpi.scale_f(TITLE_DROP),
            label_px,
            INK,
        );
    }
    if chart.legend_spec.default_right_ink() {
        // An overlay must release plot space without changing the default legend.
        // Anchor its ink at the same right edge, not at the expanded plot edge.
        let legend_plot_r = canvas.w as i32 - inset_right;
        draw_legend(canvas, chart, font, gdi.as_ref(), label_px, legend_plot_r);
    } else {
        legend_layout.draw(canvas, chart, font);
    }
    if matches!(host, ChartHost::Excel) {
        canvas.stroke_border(BORDER);
    }
    Ok(())
}

fn draw_cat_ticks(canvas: &mut Canvas, n: usize, x0: f64, x1: f64, axis_y: i32) {
    let n = n.max(1);
    let slot = (x1 - x0) / n as f64;
    if slot <= 0.0 {
        return;
    }
    for i in 0..=n {
        let x = round_half_even(x0 + slot * i as f64);
        canvas.vline_i(
            x,
            axis_y - canvas.dpi.scale_px(CAT_MAJOR_IN),
            axis_y + canvas.dpi.scale_px(CAT_MAJOR),
            INK,
        );
    }
    for i in 0..n {
        let x = round_half_even(x0 + slot * (i as f64 + 0.5));
        canvas.vline_i(
            x,
            axis_y - canvas.dpi.scale_px(CAT_MINOR_IN),
            axis_y + canvas.dpi.scale_px(CAT_MINOR),
            INK,
        );
    }
}

/// 正好在两个整数中间时取偶数。Excel 16.0 的类目分界是这样取的。
fn round_half_even(value: f64) -> i32 {
    let base = value.floor();
    let frac = value - base;
    if frac < 0.5 - 1e-8 {
        return base as i32;
    }
    if frac > 0.5 + 1e-8 {
        return base as i32 + 1;
    }
    let down = base as i32;
    if down % 2 == 0 {
        down
    } else {
        down + 1
    }
}

fn draw_value_minor_ticks(
    canvas: &mut Canvas,
    axis_x: i32,
    plot_y: i32,
    plot_b: i32,
    intervals: i32,
    parts: i32,
) {
    let span = plot_b - plot_y;
    if intervals < 1 || span <= 0 {
        return;
    }
    for i in 0..intervals {
        let y0 = plot_y + (i * span + intervals - 1) / intervals;
        let y1 = plot_y + ((i + 1) * span + intervals - 1) / intervals;
        let gap = y1 - y0;
        for k in 1..parts {
            let y = y0 + (k * gap + parts / 2) / parts;
            if y <= y0 || y >= y1 {
                continue;
            }
            canvas.hline_i(
                axis_x - canvas.dpi.scale_px(VAL_MINOR_ARM),
                axis_x + canvas.dpi.scale_px(VAL_MINOR_ARM),
                y,
                INK,
            );
        }
    }
}

fn draw_lines(canvas: &mut Canvas, chart: &ColumnChart, plot: &Rect, axis_min: f64, axis_max: f64) {
    let cats = chart.categories.len().max(1) as f64;
    let slot = (plot.w + 1.0) / cats;
    if slot <= 0.0 {
        return;
    }
    for ser in &chart.series {
        let color = ser.color.unwrap_or(chart.accents[ser.slot % 6]);
        if ser.line_visible && ser.line_width > 0.0 {
            let mut run = Vec::new();
            for (ci, value) in ser.values.iter().enumerate() {
                match value {
                    Some(v) if v.is_finite() => {
                        run.push(line_point(*v, ci, slot, plot, axis_min, axis_max));
                    }
                    _ => {
                        stroke_run(canvas, &run, ser.line_width, color);
                        run.clear();
                    }
                }
            }
            stroke_run(canvas, &run, ser.line_width, color);
        }
        if !matches!(ser.marker, MarkerKind::None) {
            let marker_color = ser.marker_color.unwrap_or(color);
            for (ci, value) in ser.values.iter().enumerate() {
                let Some(v) = value else {
                    continue;
                };
                if !v.is_finite() {
                    continue;
                }
                let (x, y) = line_point(*v, ci, slot, plot, axis_min, axis_max);
                paint_marker(canvas, ser.marker, x, y, marker_color);
            }
        }
    }
}

fn line_point(
    value: f64,
    ci: usize,
    slot: f64,
    plot: &Rect,
    axis_min: f64,
    axis_max: f64,
) -> (f64, f64) {
    let x = plot.x + slot * (ci as f64 + 0.5);
    let y = y_of(value.clamp(axis_min, axis_max), axis_min, axis_max, plot);
    (x, y)
}

fn stroke_run(canvas: &mut Canvas, run: &[(f64, f64)], width: f64, color: [u8; 3]) {
    for pair in run.windows(2) {
        canvas.stroke_segment(pair[0].0, pair[0].1, pair[1].0, pair[1].1, width, color);
    }
}

fn draw_areas(canvas: &mut Canvas, chart: &ColumnChart, plot: &Rect, axis_min: f64, axis_max: f64) {
    let cats = chart.categories.len().max(1) as f64;
    let slot = (plot.w + 1.0) / cats;
    if slot <= 0.0 {
        return;
    }
    let zero_y = y_of(0.0_f64.clamp(axis_min, axis_max), axis_min, axis_max, plot);
    for ser in &chart.series {
        let fill = ser.color.unwrap_or(chart.accents[ser.slot % 6]);
        let stroke = ser.line_color.unwrap_or(fill);
        let mut run = Vec::new();
        for (ci, value) in ser.values.iter().enumerate() {
            match value {
                Some(v) if v.is_finite() => {
                    run.push(line_point(*v, ci, slot, plot, axis_min, axis_max));
                }
                _ => {
                    fill_area_run(canvas, &run, zero_y, fill);
                    if ser.line_visible && ser.line_width > 0.0 {
                        stroke_run(canvas, &run, ser.line_width, stroke);
                    }
                    run.clear();
                }
            }
        }
        fill_area_run(canvas, &run, zero_y, fill);
        if ser.line_visible && ser.line_width > 0.0 {
            stroke_run(canvas, &run, ser.line_width, stroke);
        }
    }
}

fn fill_area_run(canvas: &mut Canvas, run: &[(f64, f64)], zero_y: f64, color: [u8; 3]) {
    for pair in run.windows(2) {
        fill_area_span(canvas, pair[0], pair[1], zero_y, color);
    }
}

fn fill_area_span(
    canvas: &mut Canvas,
    start: (f64, f64),
    end: (f64, f64),
    zero_y: f64,
    color: [u8; 3],
) {
    let (x0, y0) = start;
    let (x1, y1) = end;
    if ![x0, y0, x1, y1, zero_y].iter().all(|v| v.is_finite()) {
        return;
    }
    let s0 = y0 - zero_y;
    let s1 = y1 - zero_y;
    if s0 * s1 < 0.0 {
        let t = s0 / (s0 - s1);
        let xc = x0 + t * (x1 - x0);
        canvas.fill_polygon(&[(x0, zero_y), (x0, y0), (xc, zero_y)], color);
        canvas.fill_polygon(&[(xc, zero_y), (x1, y1), (x1, zero_y)], color);
    } else {
        canvas.fill_polygon(&[(x0, zero_y), (x0, y0), (x1, y1), (x1, zero_y)], color);
    }
}

fn paint_marker(canvas: &mut Canvas, marker: MarkerKind, x: f64, y: f64, color: [u8; 3]) {
    let MarkerKind::Shaped { symbol, size } = marker else {
        return;
    };
    if size <= 0.0 || !x.is_finite() || !y.is_finite() {
        return;
    }
    let half = size / 2.0;
    match symbol {
        MarkerSymbol::Circle => canvas.fill_circle(x, y, half, color),
        MarkerSymbol::Square => canvas.fill_polygon(
            &[
                (x - half, y - half),
                (x + half, y - half),
                (x + half, y + half),
                (x - half, y + half),
            ],
            color,
        ),
        MarkerSymbol::Diamond => canvas.fill_polygon(
            &[(x, y - half), (x + half, y), (x, y + half), (x - half, y)],
            color,
        ),
        MarkerSymbol::Triangle => canvas.fill_polygon(
            &[(x, y - half), (x + half, y + half), (x - half, y + half)],
            color,
        ),
        MarkerSymbol::X => {
            let w = canvas.dpi.scale_f(1.0);
            canvas.stroke_segment(x - half, y - half, x + half, y + half, w, color);
            canvas.stroke_segment(x + half, y - half, x - half, y + half, w, color);
        }
        MarkerSymbol::Plus => {
            let w = canvas.dpi.scale_f(1.0);
            canvas.stroke_segment(x - half, y, x + half, y, w, color);
            canvas.stroke_segment(x, y - half, x, y + half, w, color);
        }
        MarkerSymbol::Star => {
            // Official auto star is an asterisk: X plus a vertical bar (no horizontal).
            let w = canvas.dpi.scale_f(1.0);
            canvas.stroke_segment(x - half, y - half, x + half, y + half, w, color);
            canvas.stroke_segment(x + half, y - half, x - half, y + half, w, color);
            canvas.stroke_segment(x, y - half, x, y + half, w, color);
        }
        MarkerSymbol::Dash => {
            let h = canvas.dpi.pt_to_px_font(2.0);
            let hh = h / 2.0;
            canvas.fill_polygon(
                &[
                    (x - half, y - hh),
                    (x + half, y - hh),
                    (x + half, y + hh),
                    (x - half, y + hh),
                ],
                color,
            );
        }
        MarkerSymbol::Dot => {
            let h = canvas.dpi.pt_to_px_font(2.0);
            let hh = h / 2.0;
            let hw = size / 4.0;
            canvas.fill_polygon(
                &[
                    (x - hw, y - hh),
                    (x + hw, y - hh),
                    (x + hw, y + hh),
                    (x - hw, y + hh),
                ],
                color,
            );
        }
    }
}

fn draw_plot_key(
    canvas: &mut Canvas,
    kind: PlotKind,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    color: [u8; 3],
    marker: MarkerKind,
) {
    match kind {
        PlotKind::ClusteredColumn | PlotKind::StandardArea | PlotKind::Pie => {
            canvas.fill_rect(x0, y0, x1, y1, color)
        }
        PlotKind::ClusteredBar => {
            let mid = (y0 + y1) / 2;
            canvas.fill_rect(x0, mid - 1, x1, mid + 2, color);
        }
        PlotKind::StandardLine | PlotKind::Scatter { connect: true } => {
            let mid_y = (f64::from(y0) + f64::from(y1)) / 2.0;
            let mid_x = (f64::from(x0) + f64::from(x1)) / 2.0;
            canvas.stroke_segment(f64::from(x0), mid_y, f64::from(x1) - 1.0, mid_y, 2.0, color);
            let size = f64::from((x1 - x0).abs().min((y1 - y0).abs()));
            paint_marker(canvas, with_marker_size(marker, size), mid_x, mid_y, color);
        }
        PlotKind::Scatter { connect: false } => {
            let mid_y = (f64::from(y0) + f64::from(y1)) / 2.0;
            let mid_x = (f64::from(x0) + f64::from(x1)) / 2.0;
            let size = f64::from((x1 - x0).abs().min((y1 - y0).abs()));
            paint_marker(canvas, with_marker_size(marker, size), mid_x, mid_y, color);
        }
    }
}

fn category_slot_origin(plot: &Rect, index: usize, cats: usize, from_bottom: bool) -> f64 {
    let cats = cats.max(1) as f64;
    let slot = (plot.h + 1.0) / cats;
    if from_bottom {
        plot.y + plot.h + 1.0 - slot * (index as f64 + 1.0)
    } else {
        plot.y + slot * index as f64
    }
}

fn category_mid_y(plot: &Rect, index: usize, cats: usize, from_bottom: bool) -> f64 {
    let cats = cats.max(1) as f64;
    let slot = (plot.h + 1.0) / cats;
    category_slot_origin(plot, index, cats as usize, from_bottom) + slot / 2.0
}

fn draw_clustered_bars(
    canvas: &mut Canvas,
    chart: &ColumnChart,
    plot: &Rect,
    axis_min: f64,
    axis_max: f64,
) {
    let cats = chart.categories.len().max(1) as f64;
    let n = chart.series.len().max(1) as f64;
    let overlap = chart.overlap / 100.0;
    let gap = chart.gap_width / 100.0;
    let slot = (plot.h + 1.0) / cats;
    let denom = n - (n - 1.0) * overlap + gap;
    if denom <= 0.0 || slot <= 0.0 {
        return;
    }
    let bar_h = slot / denom;
    let cluster = bar_h * (n - (n - 1.0) * overlap);
    let side = (slot - cluster) / 2.0;
    let x_zero = x_of(0.0_f64.clamp(axis_min, axis_max), axis_min, axis_max, plot);
    let n_ser = chart.series.len();
    for (ci, _) in chart.categories.iter().enumerate() {
        let origin =
            category_slot_origin(plot, ci, chart.categories.len(), chart.cats_from_bottom) + side;
        for (si, ser) in chart.series.iter().enumerate() {
            let Some(value) = ser.values[ci] else {
                continue;
            };
            if !value.is_finite() {
                continue;
            }
            // 系列 0 在类目轴方向的槽起点：maxMin（第一类在上）时靠槽顶，minMax 时靠槽底。
            let place = if chart.cats_from_bottom {
                n_ser.saturating_sub(1).saturating_sub(si)
            } else {
                si
            };
            let y = origin + place as f64 * bar_h * (1.0 - overlap);
            let x_val = x_of(value.clamp(axis_min, axis_max), axis_min, axis_max, plot);
            let left = x_zero.min(x_val);
            let w = (x_zero - x_val).abs();
            if w < 0.2 || bar_h < 0.2 {
                continue;
            }
            let color = if chart.vary_points {
                chart.accents[ci % 6]
            } else {
                ser.color.unwrap_or(chart.accents[ser.slot % 6])
            };
            let x0 = left.round() as i32;
            let x1 = (left + w).round() as i32;
            let y0 = round_half_even(y);
            let y1 = round_half_even(y + bar_h);
            canvas.fill_rect(x0, y0, x1, y1, color);
        }
    }
}

fn draw_cat_ticks_y(
    canvas: &mut Canvas,
    n: usize,
    y0: f64,
    y1: f64,
    axis_x: i32,
    from_bottom: bool,
) {
    let n = n.max(1);
    let slot = (y1 - y0) / n as f64;
    if slot <= 0.0 {
        return;
    }
    for i in 0..=n {
        let t = if from_bottom { n - i } else { i };
        let y = round_half_even(y0 + slot * t as f64);
        canvas.hline_i(
            axis_x - canvas.dpi.scale_px(CAT_MAJOR),
            axis_x + canvas.dpi.scale_px(CAT_MAJOR_IN),
            y,
            INK,
        );
    }
    for i in 0..n {
        let t = if from_bottom { n - 1 - i } else { i };
        let y = round_half_even(y0 + slot * (t as f64 + 0.5));
        canvas.hline_i(
            axis_x - canvas.dpi.scale_px(CAT_MINOR),
            axis_x + canvas.dpi.scale_px(CAT_MINOR_IN),
            y,
            INK,
        );
    }
}

fn draw_value_minor_ticks_x(
    canvas: &mut Canvas,
    plot_x: i32,
    plot_r: i32,
    axis_y: i32,
    intervals: i32,
    parts: i32,
) {
    let span = plot_r - plot_x;
    if intervals < 1 || span <= 0 {
        return;
    }
    for i in 0..intervals {
        let x0 = plot_x + (i * span + intervals - 1) / intervals;
        let x1 = plot_x + ((i + 1) * span + intervals - 1) / intervals;
        let gap = x1 - x0;
        for k in 1..parts {
            let x = x0 + (k * gap + parts / 2) / parts;
            if x <= x0 || x >= x1 {
                continue;
            }
            canvas.vline_i(
                x,
                axis_y - canvas.dpi.scale_px(VAL_MINOR_ARM),
                axis_y + canvas.dpi.scale_px(VAL_MINOR_ARM),
                INK,
            );
        }
    }
}

fn draw_bars(canvas: &mut Canvas, chart: &ColumnChart, plot: &Rect, axis_min: f64, axis_max: f64) {
    let cats = chart.categories.len().max(1) as f64;
    let n = chart.series.len().max(1) as f64;
    let overlap = chart.overlap / 100.0;
    let gap = chart.gap_width / 100.0;
    // 右端那一列算进槽宽。左缘仍是轴线右侧的第一列。
    let slot = (plot.w + 1.0) / cats;
    let denom = n - (n - 1.0) * overlap + gap;
    if denom <= 0.0 || slot <= 0.0 {
        return;
    }
    let bar_w = slot / denom;
    let cluster = bar_w * (n - (n - 1.0) * overlap);
    let side = (slot - cluster) / 2.0;
    let y_zero = y_of(0.0_f64.clamp(axis_min, axis_max), axis_min, axis_max, plot);
    for (ci, _) in chart.categories.iter().enumerate() {
        let origin = plot.x + slot * ci as f64 + side;
        for (si, ser) in chart.series.iter().enumerate() {
            let Some(value) = ser.values[ci] else {
                continue;
            };
            if !value.is_finite() {
                continue;
            }
            let x = origin + si as f64 * bar_w * (1.0 - overlap);
            let y_val = y_of(value.clamp(axis_min, axis_max), axis_min, axis_max, plot);
            let top = y_zero.min(y_val);
            let h = (y_zero - y_val).abs();
            if h < 0.2 || bar_w < 0.2 {
                continue;
            }
            let color = if chart.vary_points {
                chart.accents[ci % 6]
            } else {
                ser.color.unwrap_or(chart.accents[ser.slot % 6])
            };
            let x0 = round_half_even(x);
            let x1 = round_half_even(x + bar_w);
            let y0 = top.round() as i32;
            let y1 = (top + h).round() as i32;
            canvas.fill_rect(x0, y0, x1, y1, color);
        }
    }
}

fn axis_x_of(widest_tick: i32, dpi: RenderDpi) -> i32 {
    dpi.scale_px(LABEL_EDGE) + widest_tick + dpi.scale_px(TICK_GAP)
}

fn draw_tick_labels(
    canvas: &mut Canvas,
    font: Option<&Font>,
    gdi: Option<&super::chart_gdi::Face>,
    px: f32,
    plot: &Rect,
    axis_x: i32,
    ticks: &[f64],
    labels: &[String],
    axis_min: f64,
    axis_max: f64,
    categories: &[String],
    category_y: i32,
) {
    for (tick, label) in ticks.iter().zip(labels) {
        let y = y_of(*tick, axis_min, axis_max, plot);
        let w = text_width(font, gdi, label, px);
        let h = text_height(font, px);
        draw_text(
            canvas,
            font,
            gdi,
            label,
            f64::from(axis_x) - w - f64::from(canvas.dpi.scale_px(TICK_GAP)),
            y - h / 2.0,
            px,
            INK,
        );
    }
    let cats = categories.len().max(1) as f64;
    let slot = (plot.w + 1.0) / cats;
    let baseline = f64::from(category_y) + canvas.dpi.scale_f(10.0);
    for (i, label) in categories.iter().enumerate() {
        let w = text_width(font, gdi, label, px);
        let x = plot.x + slot * (i as f64 + 0.5) - w / 2.0;
        draw_text(canvas, font, gdi, label, x, baseline, px, INK);
    }
}

fn draw_legend(
    canvas: &mut Canvas,
    chart: &ColumnChart,
    font: Option<&Font>,
    gdi: Option<&super::chart_gdi::Face>,
    px: f32,
    plot_r: i32,
) {
    if chart.legend != Some(LegendPos::Right) {
        return;
    }
    let pitch = canvas.dpi.scale_px(24);
    let n = chart.series.len() as i32;
    if n == 0 {
        return;
    }
    let block = pitch * (n - 1) + canvas.dpi.scale_px(LEGEND_SWATCH);
    let title_band = if chart.title.is_some() {
        canvas.dpi.scale_px(LEGEND_TITLE_BAND)
    } else {
        0
    };
    let mut y = (title_band + canvas.h as i32 - block) / 2;
    let x = plot_r + canvas.dpi.scale_px(LEGEND_PLOT_GAP);
    for ser in &chart.series {
        let color = ser.color.unwrap_or(chart.accents[ser.slot % 6]);
        draw_plot_key(
            canvas,
            chart.kind,
            x,
            y,
            x + canvas.dpi.scale_px(LEGEND_SWATCH),
            y + canvas.dpi.scale_px(LEGEND_SWATCH),
            color,
            ser.marker,
        );
        draw_text(
            canvas,
            font,
            gdi,
            &ser.name,
            f64::from(x + canvas.dpi.scale_px(LEGEND_SWATCH) + canvas.dpi.scale_px(LEGEND_GAP)),
            f64::from(y - canvas.dpi.scale_px(2)),
            px,
            INK,
        );
        y += pitch;
    }
}

/// 短名字维持测过的 60 像素。长名字把绘图区右缘向左推，避免字被画布切掉。
fn legend_right_inset(
    chart: &ColumnChart,
    font: Option<&Font>,
    gdi: Option<&super::chart_gdi::Face>,
    px: f32,
    dpi: RenderDpi,
) -> i32 {
    if chart.legend != Some(LegendPos::Right) {
        return dpi.scale_px(INSET_RIGHT);
    }
    // Excel 和幻灯片都按 GDI+ 默认排版宽度把绘图区向左推。
    // 字形前进宽度更窄，类目槽会被撑开，右边的柱跟着偏出去。
    let from_layout = gdi.is_some();
    let widest = chart
        .series
        .iter()
        .map(|ser| match gdi {
            Some(face) => face.layout_width(&ser.name),
            None => text_width(font, None, &ser.name, px),
        })
        .fold(0.0, f64::max);
    // 默认排版宽度已经含防裁切的边，再向上取整会把绘图区多让出 1 像素。
    let name_px = if from_layout {
        widest as i32
    } else {
        widest.ceil() as i32
    };
    let need = dpi.scale_px(LEGEND_PLOT_GAP)
        + dpi.scale_px(LEGEND_SWATCH)
        + dpi.scale_px(LEGEND_GAP)
        + name_px
        + dpi.scale_px(LEGEND_PAD);
    need.max(dpi.scale_px(INSET_RIGHT))
}

#[cfg(test)]
fn axis_range(series: &[Series]) -> (f64, f64, f64) {
    let s = axis::scale(series, axis::Spec::default(), 10).unwrap();
    (s.min, s.max, s.major)
}

fn tick_values(min: f64, max: f64, step: f64) -> Vec<f64> {
    if step <= 0.0 {
        return vec![min, max];
    }
    let mut out = Vec::new();
    let mut v = min;
    let guard = ((max - min) / step).ceil() as i32 + 2;
    for _ in 0..guard.max(0) {
        if v > max + step * 0.01 {
            break;
        }
        out.push(v);
        v += step;
    }
    if out.is_empty() {
        out.push(min);
    }
    out
}

fn y_of(value: f64, axis_min: f64, axis_max: f64, plot: &Rect) -> f64 {
    let span = (axis_max - axis_min).max(1e-12);
    let t = (value - axis_min) / span;
    plot.y + plot.h - t * plot.h
}

fn fmt_tick(value: f64) -> String {
    if !value.is_finite() {
        return String::new();
    }
    let abs = value.abs();
    if abs != 0.0 && !(0.0001..10_000_000.0).contains(&abs) {
        return format!("{value:.2e}");
    }
    if (value - value.round()).abs() < 1e-8 * abs.max(1.0) {
        return format!("{}", value.round() as i64);
    }
    let mut text = format!("{value:.4}");
    while text.contains('.') && (text.ends_with('0') || text.ends_with('.')) {
        text.pop();
    }
    text
}

fn legend_pos(chart: &Element) -> Option<LegendPos> {
    let legend = chart.child_by_local("legend")?;
    if flag(legend, "delete").unwrap_or(false) {
        return None;
    }
    let pos = legend
        .child_by_local("legendPos")
        .and_then(|el| el.attr_local("val"))
        .unwrap_or("r");
    Some(match pos {
        "l" => LegendPos::Left,
        "t" => LegendPos::Top,
        "b" => LegendPos::Bottom,
        "tr" => LegendPos::TopRight,
        _ => LegendPos::Right,
    })
}

fn title_text(chart: &Element) -> Option<String> {
    if flag(chart, "autoTitleDeleted").unwrap_or(false) {
        return None;
    }
    let title = chart.child_by_local("title")?;
    let tx = title.child_by_local("tx")?;
    if let Some(text) = tx.child_by_local("v").map(|el| el.text_content()) {
        if !text.is_empty() {
            return Some(text);
        }
    }
    if let Some(rich) = find_deep(tx, "rich") {
        let text = join_text(rich);
        if !text.is_empty() {
            return Some(text);
        }
    }
    None
}

fn series_numbers(
    parent: Option<&Element>,
    sheet: &str,
    cells: Option<&super::sparkline::CellCache>,
) -> Vec<Option<f64>> {
    let cached = cache_numbers(parent);
    if !cached.is_empty() {
        return cached;
    }
    let Some(text) = formula_text(parent) else {
        return Vec::new();
    };
    numbers_from_cells(&text, sheet, cells).unwrap_or_default()
}

fn formula_text(parent: Option<&Element>) -> Option<String> {
    let text = find_deep(parent?, "f")?.text_content();
    let text = text.trim();
    if text.is_empty() {
        None
    } else {
        Some(text.to_string())
    }
}

fn numbers_from_cells(
    formula: &str,
    sheet: &str,
    cells: Option<&super::sparkline::CellCache>,
) -> Option<Vec<Option<f64>>> {
    let values = super::sparkline::range_values(formula, sheet, cells?)?;
    Some(values.into_iter().map(json_number).collect())
}

fn labels_from_cells(
    formula: &str,
    sheet: &str,
    cells: Option<&super::sparkline::CellCache>,
) -> Option<Vec<String>> {
    let values = super::sparkline::range_values(formula, sheet, cells?)?;
    Some(values.into_iter().map(json_label).collect())
}

fn json_number(value: serde_json::Value) -> Option<f64> {
    match value {
        serde_json::Value::Number(n) => n.as_f64().filter(|n| n.is_finite()),
        serde_json::Value::Bool(b) => Some(if b { 1.0 } else { 0.0 }),
        serde_json::Value::String(s) => s.parse().ok(),
        _ => None,
    }
}

fn json_label(value: serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s,
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(true) => "TRUE".into(),
        serde_json::Value::Bool(false) => "FALSE".into(),
        _ => String::new(),
    }
}

fn series_name(
    ser: &Element,
    sheet: &str,
    cells: Option<&super::sparkline::CellCache>,
) -> Option<String> {
    let tx = ser.child_by_local("tx")?;
    if let Some(v) = tx.child_by_local("v") {
        let text = v.text_content();
        if !text.is_empty() {
            return Some(text);
        }
    }
    if let Some(cache) = find_deep(tx, "strCache").or_else(|| find_deep(tx, "numCache")) {
        let text = cache_labels_of(cache).into_iter().next()?;
        if !text.is_empty() {
            return Some(text);
        }
    }
    let text = formula_text(Some(tx))?;
    labels_from_cells(&text, sheet, cells)?
        .into_iter()
        .find(|label| !label.is_empty())
}

fn read_line_series(
    ser: &Element,
    accents: [[u8; 3]; 6],
    dpi: RenderDpi,
    slot: usize,
) -> Result<LineSeriesStyle, Error> {
    let unsupported = || Error::with_path(ErrorKind::ChartImageUnsupported, "id");
    if flag(ser, "smooth").unwrap_or(false) {
        return Err(unsupported());
    }
    data_labels_hidden(ser.child_by_local("dLbls"))?;
    for name in ["dPt", "trendline", "errBars"] {
        if ser.child_by_local(name).is_some() {
            return Err(unsupported());
        }
    }
    let mut visible = true;
    let mut width = dpi.pt_to_px_font(2.25);
    let mut color = None;
    if let Some(sp) = ser.child_by_local("spPr") {
        let style = read_line_sp_pr(sp, accents, dpi)?;
        visible = style.visible;
        width = style.width;
        color = style.color;
    }
    if color.is_none() {
        color = series_color(ser, accents);
    }
    let (marker, marker_color) = read_line_marker(ser, accents, dpi, slot)?;
    Ok(LineSeriesStyle {
        color,
        visible,
        width,
        marker,
        marker_color,
    })
}

fn read_line_sp_pr(
    sp: &Element,
    accents: [[u8; 3]; 6],
    dpi: RenderDpi,
) -> Result<LineSeriesStyle, Error> {
    let unsupported = || Error::with_path(ErrorKind::ChartImageUnsupported, "id");
    let mut visible = true;
    let mut width = dpi.pt_to_px_font(2.25);
    let mut color = None;
    for child in sp.element_children() {
        match child.local_name() {
            "noFill" | "solidFill" => {}
            "effectLst" => {
                if child.element_children().next().is_some() {
                    return Err(unsupported());
                }
            }
            "ln" => {
                let line = read_solid_ln(child, accents, width, dpi)?;
                visible = line.visible;
                width = line.width;
                if line.color.is_some() {
                    color = line.color;
                }
            }
            _ => return Err(unsupported()),
        }
    }
    if color.is_none() {
        if let Some(fill) = sp.child_by_local("solidFill") {
            color = color_of_slot(fill, &accents);
        }
    }
    Ok(LineSeriesStyle {
        color,
        visible,
        width,
        marker: MarkerKind::None,
        marker_color: None,
    })
}

fn read_solid_ln(
    ln: &Element,
    accents: [[u8; 3]; 6],
    default_width: f64,
    dpi: RenderDpi,
) -> Result<LineSeriesStyle, Error> {
    let unsupported = || Error::with_path(ErrorKind::ChartImageUnsupported, "id");
    for (attr, allowed) in [
        ("cap", &["rnd"][..]),
        ("cmpd", &["sng"][..]),
        ("algn", &["ctr"][..]),
    ] {
        if let Some(val) = ln.attr_local(attr) {
            if !allowed.contains(&val) {
                return Err(unsupported());
            }
        }
    }
    for (name, _) in &ln.attrs {
        let local = name.rsplit(':').next().unwrap_or(name.as_str());
        if local == "xmlns" || name.starts_with("xmlns:") {
            continue;
        }
        if !matches!(local, "w" | "cap" | "cmpd" | "algn") {
            return Err(unsupported());
        }
    }
    let width = ln
        .attr_local("w")
        .and_then(|raw| raw.parse::<f64>().ok())
        .map(|emu| dpi.emu_to_px_f(emu))
        .unwrap_or(default_width);
    if !width.is_finite() || width < 0.0 {
        return Err(unsupported());
    }
    let mut visible = true;
    let mut color = None;
    for child in ln.element_children() {
        match child.local_name() {
            "noFill" => visible = false,
            "solidFill" => {
                color = Some(color_of_slot(child, &accents).ok_or_else(unsupported)?);
            }
            "prstDash" => {
                let dash = child.attr_local("val").unwrap_or("solid");
                if dash != "solid" {
                    return Err(unsupported());
                }
            }
            "miter" | "round" | "bevel" => {}
            "effectLst" => {
                if child.element_children().next().is_some() {
                    return Err(unsupported());
                }
            }
            _ => return Err(unsupported()),
        }
    }
    if !visible {
        color = None;
    }
    Ok(LineSeriesStyle {
        color,
        visible,
        width,
        marker: MarkerKind::None,
        marker_color: None,
    })
}

fn read_line_marker(
    ser: &Element,
    accents: [[u8; 3]; 6],
    dpi: RenderDpi,
    slot: usize,
) -> Result<(MarkerKind, Option<[u8; 3]>), Error> {
    let unsupported = || Error::with_path(ErrorKind::ChartImageUnsupported, "id");
    let Some(marker) = ser.child_by_local("marker") else {
        return Ok((
            shaped_marker(default_marker_symbol(slot), OMITTED_MARKER_SIZE_PT, dpi),
            None,
        ));
    };
    let symbol = marker
        .child_by_local("symbol")
        .and_then(|el| el.attr_local("val"))
        .unwrap_or("auto");
    if symbol == "none" {
        return Ok((MarkerKind::None, None));
    }
    let symbol = if symbol == "auto" {
        default_marker_symbol(slot)
    } else {
        marker_symbol_from_name(symbol).ok_or_else(unsupported)?
    };
    let size_pt = number_val(marker, "size").unwrap_or(EXPLICIT_MARKER_DEFAULT_SIZE_PT);
    if !size_pt.is_finite() || size_pt <= 0.0 {
        return Err(unsupported());
    }
    let mut marker_color = None;
    if let Some(sp) = marker.child_by_local("spPr") {
        for child in sp.element_children() {
            match child.local_name() {
                "solidFill" => {
                    marker_color = Some(color_of_slot(child, &accents).ok_or_else(unsupported)?);
                }
                "noFill" => return Err(unsupported()),
                "ln" => {
                    let _ = read_solid_ln(child, accents, 1.0, dpi)?;
                }
                "effectLst" => {
                    if child.element_children().next().is_some() {
                        return Err(unsupported());
                    }
                }
                _ => return Err(unsupported()),
            }
        }
    }
    Ok((shaped_marker(symbol, size_pt, dpi), marker_color))
}

fn pie_first_slice_ang(wrapper: &Element) -> Result<f64, Error> {
    let unsupported = || Error::with_path(ErrorKind::ChartImageUnsupported, "id");
    let Some(el) = wrapper.child_by_local("firstSliceAng") else {
        return Ok(0.0);
    };
    let angle = el
        .attr_local("val")
        .unwrap_or("0")
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite())
        .ok_or_else(unsupported)?;
    if !(0.0..=360.0).contains(&angle) {
        return Err(unsupported());
    }
    if (angle - 360.0).abs() < 1e-9 {
        Ok(0.0)
    } else {
        Ok(angle)
    }
}

fn read_pie_series_style(
    ser: &Element,
    accents: [[u8; 3]; 6],
    dpi: RenderDpi,
) -> Result<LineSeriesStyle, Error> {
    let unsupported = || Error::with_path(ErrorKind::ChartImageUnsupported, "id");
    data_labels_hidden(ser.child_by_local("dLbls"))?;
    for name in ["trendline", "errBars"] {
        if ser.child_by_local(name).is_some() {
            return Err(unsupported());
        }
    }
    if let Some(explosion) = number_val(ser, "explosion") {
        if explosion != 0.0 {
            return Err(unsupported());
        }
    }
    if let Some(sp) = ser.child_by_local("spPr") {
        reject_pie_sp_pr(sp, accents, dpi)?;
    }
    Ok(LineSeriesStyle {
        color: series_color(ser, accents),
        visible: false,
        width: 0.0,
        marker: MarkerKind::None,
        marker_color: None,
    })
}

fn reject_pie_sp_pr(sp: &Element, accents: [[u8; 3]; 6], dpi: RenderDpi) -> Result<(), Error> {
    let unsupported = || Error::with_path(ErrorKind::ChartImageUnsupported, "id");
    for child in sp.element_children() {
        match child.local_name() {
            "solidFill" => {
                if color_has_non_opaque_alpha(child) {
                    return Err(unsupported());
                }
                let _ = color_of_slot(child, &accents).ok_or_else(unsupported)?;
            }
            "noFill" | "gradFill" | "pattFill" | "blipFill" => return Err(unsupported()),
            "ln" => {
                let line = read_solid_ln(child, accents, 0.0, dpi)?;
                if line.visible {
                    return Err(unsupported());
                }
            }
            "effectLst" => {
                if child.element_children().next().is_some() {
                    return Err(unsupported());
                }
            }
            _ => return Err(unsupported()),
        }
    }
    Ok(())
}

fn read_pie_point_colors(
    ser: &Element,
    accents: [[u8; 3]; 6],
    n: usize,
    dpi: RenderDpi,
) -> Result<Vec<Option<[u8; 3]>>, Error> {
    let unsupported = || Error::with_path(ErrorKind::ChartImageUnsupported, "id");
    if let Some(explosion) = number_val(ser, "explosion") {
        if explosion != 0.0 {
            return Err(unsupported());
        }
    }
    let mut colors = vec![None; n];
    for dpt in ser.element_children().filter(|el| el.local_name() == "dPt") {
        let idx = number_val(dpt, "idx").ok_or_else(unsupported)?;
        if !idx.is_finite() || idx < 0.0 || idx.fract() != 0.0 {
            return Err(unsupported());
        }
        let idx = idx as usize;
        if idx >= n || colors[idx].is_some() {
            return Err(unsupported());
        }
        if let Some(explosion) = number_val(dpt, "explosion") {
            if explosion != 0.0 {
                return Err(unsupported());
            }
        }
        for name in ["invertIfNegative", "marker", "bubble3D", "pictureOptions"] {
            if dpt.child_by_local(name).is_some() {
                return Err(unsupported());
            }
        }
        if let Some(sp) = dpt.child_by_local("spPr") {
            reject_pie_sp_pr(sp, accents, dpi)?;
            if let Some(fill) = sp.child_by_local("solidFill") {
                colors[idx] = Some(color_of_slot(fill, &accents).ok_or_else(unsupported)?);
            }
        }
    }
    Ok(colors)
}

struct AreaSeriesStyle {
    fill: Option<[u8; 3]>,
    line_visible: bool,
    line_width: f64,
    line_color: Option<[u8; 3]>,
}

fn read_area_series(
    ser: &Element,
    accents: [[u8; 3]; 6],
    dpi: RenderDpi,
) -> Result<AreaSeriesStyle, Error> {
    let unsupported = || Error::with_path(ErrorKind::ChartImageUnsupported, "id");
    if flag(ser, "smooth").unwrap_or(false) {
        return Err(unsupported());
    }
    data_labels_hidden(ser.child_by_local("dLbls"))?;
    for name in ["dPt", "trendline", "errBars"] {
        if ser.child_by_local(name).is_some() {
            return Err(unsupported());
        }
    }
    if let Some(marker) = ser.child_by_local("marker") {
        let symbol = marker
            .child_by_local("symbol")
            .and_then(|el| el.attr_local("val"))
            .unwrap_or("auto");
        if symbol != "none" {
            return Err(unsupported());
        }
    }
    let mut fill = None;
    let mut line_visible = false;
    let mut line_width = dpi.pt_to_px_font(2.25);
    let mut line_color = None;
    if let Some(sp) = ser.child_by_local("spPr") {
        for child in sp.element_children() {
            match child.local_name() {
                "solidFill" => {
                    if color_has_non_opaque_alpha(child) {
                        return Err(unsupported());
                    }
                    fill = Some(color_of_slot(child, &accents).ok_or_else(unsupported)?);
                }
                "noFill" => return Err(unsupported()),
                "ln" => {
                    if let Some(solid) = child.child_by_local("solidFill") {
                        if color_has_non_opaque_alpha(solid) {
                            return Err(unsupported());
                        }
                    }
                    let line = read_solid_ln(child, accents, line_width, dpi)?;
                    line_visible = line.visible;
                    line_width = line.width;
                    line_color = line.color;
                }
                "effectLst" => {
                    if child.element_children().next().is_some() {
                        return Err(unsupported());
                    }
                }
                _ => return Err(unsupported()),
            }
        }
    }
    Ok(AreaSeriesStyle {
        fill,
        line_visible,
        line_width,
        line_color,
    })
}

fn color_has_non_opaque_alpha(slot: &Element) -> bool {
    let Some(node) = slot
        .child_by_local("srgbClr")
        .or_else(|| slot.child_by_local("sysClr"))
        .or_else(|| slot.child_by_local("schemeClr"))
        .or_else(|| {
            if matches!(slot.local_name(), "srgbClr" | "sysClr" | "schemeClr") {
                Some(slot)
            } else {
                None
            }
        })
    else {
        return false;
    };
    node.element_children()
        .any(|child| match child.local_name() {
            "alpha" => child
                .attr_local("val")
                .and_then(|raw| raw.parse::<f64>().ok())
                .is_none_or(|val| (val - 100_000.0).abs() > 1e-6),
            "alphaMod" | "alphaOff" => true,
            _ => false,
        })
}

fn series_color(ser: &Element, accents: [[u8; 3]; 6]) -> Option<[u8; 3]> {
    let fill = ser.child_by_local("spPr")?.child_by_local("solidFill")?;
    color_of_slot(fill, &accents)
}

fn cache_numbers(parent: Option<&Element>) -> Vec<Option<f64>> {
    let Some(parent) = parent else {
        return Vec::new();
    };
    let Some(cache) = find_deep(parent, "numCache").or_else(|| find_deep(parent, "numLit")) else {
        return Vec::new();
    };
    points_to_values(&read_pts(cache), |text| text.parse().ok())
}

fn cache_labels(parent: Option<&Element>) -> Vec<String> {
    let Some(parent) = parent else {
        return Vec::new();
    };
    let Some(cache) = find_cache(parent) else {
        return Vec::new();
    };
    cache_labels_of(cache)
}

fn cache_labels_of(cache: &Element) -> Vec<String> {
    points_to_values(&read_pts(cache), |text| Some(text.to_string()))
        .into_iter()
        .map(|item| item.unwrap_or_default())
        .collect()
}

fn find_cache(el: &Element) -> Option<&Element> {
    find_deep(el, "strCache")
        .or_else(|| find_deep(el, "numCache"))
        .or_else(|| find_deep(el, "strLit"))
        .or_else(|| find_deep(el, "numLit"))
}

fn read_pts(cache: &Element) -> Vec<(usize, String)> {
    let mut pts = Vec::new();
    for pt in cache.element_children() {
        if pt.local_name() != "pt" {
            continue;
        }
        let Some(idx) = pt.attr_local("idx").and_then(|s| s.parse().ok()) else {
            continue;
        };
        let text = pt
            .child_by_local("v")
            .map(|el| el.text_content())
            .unwrap_or_default();
        pts.push((idx, text));
    }
    pts
}

fn points_to_values<T>(pts: &[(usize, String)], map: impl Fn(&str) -> Option<T>) -> Vec<Option<T>> {
    let len = pts.iter().map(|(idx, _)| *idx + 1).max().unwrap_or(0);
    let mut out = Vec::new();
    out.resize_with(len, || None);
    for (idx, text) in pts {
        if let Some(value) = map(text.trim()) {
            out[*idx] = Some(value);
        }
    }
    out
}

fn color_of_slot(slot: &Element, accents: &[[u8; 3]; 6]) -> Option<[u8; 3]> {
    let node = slot
        .child_by_local("srgbClr")
        .or_else(|| slot.child_by_local("sysClr"))
        .or_else(|| slot.child_by_local("schemeClr"))
        .or_else(|| {
            if matches!(slot.local_name(), "srgbClr" | "sysClr" | "schemeClr") {
                Some(slot)
            } else {
                None
            }
        })?;
    let mut rgb = if node.local_name() == "schemeClr" {
        scheme_rgb(node.attr_local("val")?, accents)?
    } else {
        parse_hex(
            node.attr_local("val")
                .or_else(|| node.attr_local("lastClr"))?,
        )?
    };
    for child in node.element_children() {
        let Some(raw) = child.attr_local("val") else {
            continue;
        };
        let Some(num) = raw.parse::<f64>().ok() else {
            continue;
        };
        let frac = num / 100_000.0;
        match child.local_name() {
            "lumMod" => rgb = lum_mod(rgb, frac),
            "lumOff" => rgb = lum_off(rgb, frac),
            "shade" => rgb = scale_rgb(rgb, frac),
            "tint" => rgb = tint_rgb(rgb, frac),
            _ => {}
        }
    }
    Some(rgb)
}

fn scheme_rgb(name: &str, accents: &[[u8; 3]; 6]) -> Option<[u8; 3]> {
    Some(match name {
        "accent1" => accents[0],
        "accent2" => accents[1],
        "accent3" => accents[2],
        "accent4" => accents[3],
        "accent5" => accents[4],
        "accent6" => accents[5],
        "dk1" | "tx1" => [0, 0, 0],
        "lt1" | "bg1" => [255, 255, 255],
        "dk2" | "tx2" => [0x1F, 0x49, 0x7D],
        "lt2" | "bg2" => [0xEE, 0xEC, 0xE1],
        _ => return None,
    })
}

fn parse_hex(text: &str) -> Option<[u8; 3]> {
    let text = text.trim();
    if text.len() < 6 {
        return None;
    }
    let text = &text[text.len() - 6..];
    Some([
        u8::from_str_radix(&text[0..2], 16).ok()?,
        u8::from_str_radix(&text[2..4], 16).ok()?,
        u8::from_str_radix(&text[4..6], 16).ok()?,
    ])
}

fn lum_mod(rgb: [u8; 3], factor: f64) -> [u8; 3] {
    let (h, s, l) = rgb_hsl(rgb);
    hsl_rgb(h, s, (l * factor).clamp(0.0, 1.0))
}

fn lum_off(rgb: [u8; 3], offset: f64) -> [u8; 3] {
    let (h, s, l) = rgb_hsl(rgb);
    hsl_rgb(h, s, (l + offset).clamp(0.0, 1.0))
}

fn scale_rgb(rgb: [u8; 3], factor: f64) -> [u8; 3] {
    rgb.map(|c| ((c as f64) * factor).round().clamp(0.0, 255.0) as u8)
}

fn tint_rgb(rgb: [u8; 3], factor: f64) -> [u8; 3] {
    rgb.map(|c| {
        let v = c as f64;
        (v + (255.0 - v) * factor).round().clamp(0.0, 255.0) as u8
    })
}

fn rgb_hsl(rgb: [u8; 3]) -> (f64, f64, f64) {
    let r = rgb[0] as f64 / 255.0;
    let g = rgb[1] as f64 / 255.0;
    let b = rgb[2] as f64 / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    if (max - min).abs() < 1e-8 {
        return (0.0, 0.0, l);
    }
    let d = max - min;
    let s = if l > 0.5 {
        d / (2.0 - max - min)
    } else {
        d / (max + min)
    };
    let h = if (max - r).abs() < 1e-8 {
        (g - b) / d + if g < b { 6.0 } else { 0.0 }
    } else if (max - g).abs() < 1e-8 {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };
    (h / 6.0, s, l)
}

fn hsl_rgb(h: f64, s: f64, l: f64) -> [u8; 3] {
    if s <= 1e-8 {
        let v = (l * 255.0).round().clamp(0.0, 255.0) as u8;
        return [v, v, v];
    }
    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;
    [
        hue(p, q, h + 1.0 / 3.0),
        hue(p, q, h),
        hue(p, q, h - 1.0 / 3.0),
    ]
}

fn hue(p: f64, q: f64, mut t: f64) -> u8 {
    if t < 0.0 {
        t += 1.0;
    }
    if t > 1.0 {
        t -= 1.0;
    }
    let v = if t < 1.0 / 6.0 {
        p + (q - p) * 6.0 * t
    } else if t < 0.5 {
        q
    } else if t < 2.0 / 3.0 {
        p + (q - p) * (2.0 / 3.0 - t) * 6.0
    } else {
        p
    };
    (v * 255.0).round().clamp(0.0, 255.0) as u8
}

fn data_labels_hidden(dlbls: Option<&Element>) -> Result<(), Error> {
    let Some(dlbls) = dlbls else {
        return Ok(());
    };
    let unsupported = || Error::with_path(ErrorKind::ChartImageUnsupported, "id");
    if dlbls
        .element_children()
        .any(|child| child.local_name() == "dLbl")
    {
        return Err(unsupported());
    }
    for name in [
        "showLegendKey",
        "showVal",
        "showCatName",
        "showSerName",
        "showPercent",
        "showBubbleSize",
        "showLeaderLines",
    ] {
        if flag(dlbls, name).unwrap_or(false) {
            return Err(unsupported());
        }
    }
    Ok(())
}

fn flag(el: &Element, name: &str) -> Option<bool> {
    let child = el.child_by_local(name)?;
    Some(!matches!(
        child.attr_local("val"),
        Some("0" | "false" | "off")
    ))
}

fn number_val(el: &Element, name: &str) -> Option<f64> {
    el.child_by_local(name)?.attr_local("val")?.parse().ok()
}

fn find_deep<'a>(el: &'a Element, name: &str) -> Option<&'a Element> {
    if el.local_name() == name {
        return Some(el);
    }
    for child in el.element_children() {
        if let Some(found) = find_deep(child, name) {
            return Some(found);
        }
    }
    None
}

fn join_text(el: &Element) -> String {
    let mut text = String::new();
    if el.local_name() == "t" {
        text.push_str(&el.text_content());
    }
    for child in el.element_children() {
        text.push_str(&join_text(child));
    }
    text
}

fn text_width(
    font: Option<&Font>,
    gdi: Option<&super::chart_gdi::Face>,
    text: &str,
    px: f32,
) -> f64 {
    if let Some(face) = gdi {
        return face.width(text);
    }
    let Some(font) = font else {
        return text.chars().count() as f64 * f64::from(px) * 0.5;
    };
    text.chars()
        .map(|ch| f64::from(font.metrics(ch, px).advance_width))
        .sum()
}

fn text_height(font: Option<&Font>, px: f32) -> f64 {
    font.and_then(|font| font.horizontal_line_metrics(px))
        .map(|metrics| f64::from(metrics.new_line_size))
        .unwrap_or(f64::from(px))
}

fn draw_text(
    canvas: &mut Canvas,
    font: Option<&Font>,
    gdi: Option<&super::chart_gdi::Face>,
    text: &str,
    x: f64,
    top: f64,
    px: f32,
    color: [u8; 3],
) {
    if let Some(face) = gdi {
        face.draw(text, x, top, color, |px, py, rgb, cov| {
            canvas.blend(px, py, rgb, cov);
        });
        return;
    }
    let Some(font) = font else {
        return;
    };
    let metrics = font.horizontal_line_metrics(px);
    let ascent = metrics
        .map(|item| f64::from(item.ascent))
        .unwrap_or(f64::from(px));
    let baseline = top + ascent;
    let mut cursor = x;
    for ch in text.chars() {
        let (glyph, bitmap) = font.rasterize(ch, px);
        let left = cursor + f64::from(glyph.xmin);
        let glyph_top = baseline - f64::from(glyph.height as i32 + glyph.ymin);
        for row in 0..glyph.height {
            for col in 0..glyph.width {
                let cov = bitmap[row * glyph.width + col];
                canvas.blend(
                    left.round() as i32 + col as i32,
                    glyph_top.round() as i32 + row as i32,
                    color,
                    cov,
                );
            }
        }
        cursor += f64::from(glyph.advance_width);
    }
}

fn angle_in_wedge(dx: f64, dy: f64, start_deg: f64, end_deg: f64) -> bool {
    let span = end_deg - start_deg;
    if span >= 360.0 - 1e-9 {
        return true;
    }
    if span <= 1e-9 {
        return false;
    }
    let mut deg = dx.atan2(-dy).to_degrees();
    if deg < 0.0 {
        deg += 360.0;
    }
    let start = start_deg.rem_euclid(360.0);
    let end = start + span;
    if end <= 360.0 {
        deg >= start && deg < end
    } else {
        deg >= start || deg < (end - 360.0)
    }
}

fn point_in_polygon(px: f64, py: f64, pts: &[(f64, f64)]) -> bool {
    let mut inside = false;
    let mut j = pts.len() - 1;
    for i in 0..pts.len() {
        let (xi, yi) = pts[i];
        let (xj, yj) = pts[j];
        if (yi > py) != (yj > py) {
            let x_int = (xj - xi) * (py - yi) / (yj - yi) + xi;
            if px < x_int {
                inside = !inside;
            }
        }
        j = i;
    }
    inside
}

fn coverage(dist: f64, radius: f64) -> u8 {
    let cov = (radius + 0.5 - dist).clamp(0.0, 1.0);
    (cov * 255.0 + 0.5) as u8
}

impl Canvas {
    fn encode_png(&self) -> Result<Vec<u8>, Error> {
        if let Some(alpha) = &self.alpha {
            let mut rgba = Vec::with_capacity(alpha.len() * 4);
            for (rgb, a) in self.px.as_chunks::<3>().0.iter().zip(alpha) {
                rgba.extend_from_slice(rgb);
                rgba.push(*a);
            }
            encode_png(self.w, self.h, &rgba, png::ColorType::Rgba, self.dpi)
        } else {
            encode_png(self.w, self.h, &self.px, png::ColorType::Rgb, self.dpi)
        }
    }

    #[allow(dead_code)]
    fn new(w: u32, h: u32) -> Self {
        Self::with_dpi(w, h, RenderDpi::DEFAULT)
    }

    fn with_dpi(w: u32, h: u32, dpi: RenderDpi) -> Self {
        Self {
            w,
            h,
            px: vec![255; (w as usize) * (h as usize) * 3],
            alpha: None,
            dpi,
        }
    }

    fn sx(&self, px96: i32) -> i32 {
        self.dpi.scale_px(px96)
    }

    fn blend(&mut self, x: i32, y: i32, rgb: [u8; 3], cov: u8) {
        if cov == 0 || x < 0 || y < 0 || x >= self.w as i32 || y >= self.h as i32 {
            return;
        }
        let i = (y as usize * self.w as usize + x as usize) * 3;
        if cov == 255 {
            self.px[i..i + 3].copy_from_slice(&rgb);
            if let Some(alpha) = &mut self.alpha {
                alpha[i / 3] = 255;
            }
            return;
        }
        let c = u16::from(cov);
        for (k, src) in rgb.into_iter().enumerate() {
            let dst = u16::from(self.px[i + k]);
            self.px[i + k] = ((u16::from(src) * c + dst * (255 - c) + 127) / 255) as u8;
        }
    }

    fn fill_rect(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, color: [u8; 3]) {
        let x0 = x0.max(0);
        let y0 = y0.max(0);
        let x1 = x1.min(self.w as i32);
        let y1 = y1.min(self.h as i32);
        for y in y0..y1 {
            for x in x0..x1 {
                self.blend(x, y, color, 255);
            }
        }
    }

    /// Opaque even-odd fill using pixel centers. Shared edges of split
    /// trapezoids are not double-counted, and later series overwrite earlier ones.
    fn fill_polygon(&mut self, pts: &[(f64, f64)], color: [u8; 3]) {
        if pts.len() < 3 || pts.iter().any(|(x, y)| !x.is_finite() || !y.is_finite()) {
            return;
        }
        let min_x = pts
            .iter()
            .map(|p| p.0)
            .fold(f64::INFINITY, f64::min)
            .floor() as i32;
        let max_x = pts
            .iter()
            .map(|p| p.0)
            .fold(f64::NEG_INFINITY, f64::max)
            .ceil() as i32;
        let min_y = pts
            .iter()
            .map(|p| p.1)
            .fold(f64::INFINITY, f64::min)
            .floor() as i32;
        let max_y = pts
            .iter()
            .map(|p| p.1)
            .fold(f64::NEG_INFINITY, f64::max)
            .ceil() as i32;
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let px = f64::from(x) + 0.5;
                let py = f64::from(y) + 0.5;
                if point_in_polygon(px, py, pts) {
                    self.blend(x, y, color, 255);
                }
            }
        }
    }

    /// Clockwise wedge from `start_deg` (from up) to `end_deg`. The outer
    /// rim uses the same coverage as filled circles; the angular range is half-open.
    fn fill_wedge(
        &mut self,
        cx: f64,
        cy: f64,
        radius: f64,
        start_deg: f64,
        end_deg: f64,
        color: [u8; 3],
    ) {
        if radius <= 0.0
            || !cx.is_finite()
            || !cy.is_finite()
            || !radius.is_finite()
            || !start_deg.is_finite()
            || !end_deg.is_finite()
        {
            return;
        }
        let span = end_deg - start_deg;
        if span <= 1e-9 {
            return;
        }
        let pad = radius + 1.0;
        let min_x = (cx - pad).floor() as i32;
        let min_y = (cy - pad).floor() as i32;
        let max_x = (cx + pad).ceil() as i32;
        let max_y = (cy + pad).ceil() as i32;
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let px = f64::from(x) + 0.5;
                let py = f64::from(y) + 0.5;
                let dx = px - cx;
                let dy = py - cy;
                let dist = dx.hypot(dy);
                let cov = coverage(dist, radius);
                if cov == 0 {
                    continue;
                }
                if span >= 360.0 - 1e-9 || angle_in_wedge(dx, dy, start_deg, end_deg) {
                    self.blend(x, y, color, cov);
                }
            }
        }
    }

    fn hline_i(&mut self, x0: i32, x1: i32, y: i32, color: [u8; 3]) {
        for x in x0..=x1 {
            self.blend(x, y, color, 255);
        }
    }

    fn vline_i(&mut self, x: i32, y0: i32, y1: i32, color: [u8; 3]) {
        for y in y0..=y1 {
            self.blend(x, y, color, 255);
        }
    }

    fn stroke_segment(&mut self, x0: f64, y0: f64, x1: f64, y1: f64, width: f64, color: [u8; 3]) {
        if width <= 0.0 || !x0.is_finite() || !y0.is_finite() || !x1.is_finite() || !y1.is_finite()
        {
            return;
        }
        let radius = width / 2.0;
        let pad = radius + 1.0;
        let min_x = (x0.min(x1) - pad).floor() as i32;
        let min_y = (y0.min(y1) - pad).floor() as i32;
        let max_x = (x0.max(x1) + pad).ceil() as i32;
        let max_y = (y0.max(y1) + pad).ceil() as i32;
        let dx = x1 - x0;
        let dy = y1 - y0;
        let len2 = dx * dx + dy * dy;
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let px = f64::from(x) + 0.5;
                let py = f64::from(y) + 0.5;
                let dist = if len2 <= 1e-12 {
                    let vx = px - x0;
                    let vy = py - y0;
                    (vx * vx + vy * vy).sqrt()
                } else {
                    let t = ((px - x0) * dx + (py - y0) * dy) / len2;
                    let t = t.clamp(0.0, 1.0);
                    let vx = px - (x0 + t * dx);
                    let vy = py - (y0 + t * dy);
                    (vx * vx + vy * vy).sqrt()
                };
                let cov = coverage(dist, radius);
                if cov > 0 {
                    self.blend(x, y, color, cov);
                }
            }
        }
    }

    fn fill_circle(&mut self, cx: f64, cy: f64, radius: f64, color: [u8; 3]) {
        if radius <= 0.0 || !cx.is_finite() || !cy.is_finite() {
            return;
        }
        let pad = radius + 1.0;
        let min_x = (cx - pad).floor() as i32;
        let min_y = (cy - pad).floor() as i32;
        let max_x = (cx + pad).ceil() as i32;
        let max_y = (cy + pad).ceil() as i32;
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let px = f64::from(x) + 0.5;
                let py = f64::from(y) + 0.5;
                let dist = (px - cx).hypot(py - cy);
                let cov = coverage(dist, radius);
                if cov > 0 {
                    self.blend(x, y, color, cov);
                }
            }
        }
    }

    fn stroke_border(&mut self, color: [u8; 3]) {
        let w = self.w as i32;
        let h = self.h as i32;
        if w < 2 || h < 2 {
            return;
        }
        let r = CORNER_R.min(w / 2).min(h / 2);
        if r < 2 {
            self.hline_i(0, w - 1, 0, color);
            self.hline_i(0, w - 1, h - 1, color);
            self.vline_i(0, 0, h - 1, color);
            self.vline_i(w - 1, 0, h - 1, color);
            return;
        }
        self.alpha = Some(vec![255; (self.w as usize) * (self.h as usize)]);
        self.stroke_quarter(r, r, 0, r, 0, r, color);
        self.stroke_quarter(w - r, r, w - r, w - 1, 0, r, color);
        self.stroke_quarter(r, h - r, 0, r, h - r, h - 1, color);
        self.stroke_quarter(w - r, h - r, w - r, w - 1, h - r, h - 1, color);
        self.hline_i(r, w - 1 - r, 0, color);
        self.hline_i(r, w - 1 - r, h - 1, color);
        self.vline_i(0, r, h - 1 - r, color);
        self.vline_i(w - 1, r, h - 1 - r, color);
    }

    /// 一个圆角。圆心用像素坐标，半径是外框圆角。只涂靠近圆弧的那一圈。
    fn stroke_quarter(
        &mut self,
        cx: i32,
        cy: i32,
        x0: i32,
        x1: i32,
        y0: i32,
        y1: i32,
        color: [u8; 3],
    ) {
        // 大图圆心离边缘远，半径仍是 14。小图圆心就在圆角半径上。
        let rad = f64::from(cx.min(cy).min(CORNER_R)) - 0.5;
        let cx = f64::from(cx);
        let cy = f64::from(cy);
        for y in y0..=y1 {
            for x in x0..=x1 {
                let dx = f64::from(x) + 0.5 - cx;
                let dy = f64::from(y) + 0.5 - cy;
                let dist = (dx * dx + dy * dy).sqrt();
                let cov = (1.0 - (dist - rad).abs()).clamp(0.0, 1.0);
                let coverage = (cov * 255.0).round() as u8;
                self.blend(x, y, color, coverage);
                // Outside the stroke centre the same geometric coverage is also
                // the silhouette coverage. Store unassociated border RGB rather
                // than the old border-over-white RGB (which creates a white halo).
                // Inside it, the opaque white fill and original RGB stay intact.
                if dist >= rad {
                    let i = y as usize * self.w as usize + x as usize;
                    if let Some(alpha) = &mut self.alpha {
                        alpha[i] = coverage;
                    }
                    self.px[i * 3..i * 3 + 3].copy_from_slice(&if coverage == 0 {
                        [0; 3]
                    } else {
                        color
                    });
                }
            }
        }
    }
}

fn encode_png(
    w: u32,
    h: u32,
    rgb: &[u8],
    color: png::ColorType,
    dpi: RenderDpi,
) -> Result<Vec<u8>, Error> {
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, w, h);
    encoder.set_color(color);
    encoder.set_depth(png::BitDepth::Eight);
    if !dpi.is_default() {
        let ppm = (dpi.get() / 0.0254).round() as u32;
        encoder.set_pixel_dims(Some(png::PixelDimensions {
            xppu: ppm,
            yppu: ppm,
            unit: png::Unit::Meter,
        }));
    }
    let mut writer = encoder
        .write_header()
        .map_err(|_| Error::with_path(ErrorKind::ChartImageUnsupported, "id"))?;
    writer
        .write_image_data(rgb)
        .map_err(|_| Error::with_path(ErrorKind::ChartImageUnsupported, "id"))?;
    drop(writer);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Existing colour/layout assertions inspect the white-composited export.
    /// Raw RGBA and coverage have separate assertions below.
    fn render_rgb_chart(
        root: &Element,
        width: u32,
        height: u32,
        accents: [[u8; 3]; 6],
        font_bytes: Option<&[u8]>,
        host: ChartHost,
        cells: Option<&super::super::sparkline::CellCache>,
        sheet: &str,
    ) -> Result<Vec<u8>, Error> {
        let data =
            super::render_chart(root, width, height, accents, font_bytes, host, cells, sheet)?;
        let mut reader = png::Decoder::new(std::io::Cursor::new(data))
            .read_info()
            .unwrap();
        let mut buf = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).unwrap();
        let data = &buf[..info.buffer_size()];
        if info.color_type == png::ColorType::Rgb {
            return encode_png(width, height, data, png::ColorType::Rgb, RenderDpi::DEFAULT);
        }
        let rgb: Vec<_> = data
            .as_chunks::<4>()
            .0
            .iter()
            .flat_map(|p| {
                p[..3].iter().map(move |c| {
                    ((u16::from(*c) * u16::from(p[3]) + 255 * (255 - u16::from(p[3])) + 127) / 255)
                        as u8
                })
            })
            .collect();
        encode_png(width, height, &rgb, png::ColorType::Rgb, RenderDpi::DEFAULT)
    }

    #[test]
    fn valid_unsupported_backgrounds_are_not_silently_defaulted() {
        for config in [
            "<c:roundedCorners val='0'/>",
            "<c:spPr><a:noFill/></c:spPr>",
            "<c:spPr><a:solidFill><a:srgbClr val='123456'/></a:solidFill></c:spPr>",
            "<c:spPr><a:ln w='25400'/></c:spPr>",
        ] {
            let xml = LONG_LEGEND.replace("<c:chart>", &format!("{config}<c:chart>"));
            let doc = crate::xml::parse(xml.as_bytes()).unwrap();
            assert!(
                super::render_chart(
                    &doc.root,
                    576,
                    336,
                    FALLBACK_ACCENTS,
                    None,
                    ChartHost::Excel,
                    None,
                    ""
                )
                .is_err(),
                "unsupported valid config: {config}"
            );
        }
        for config in [
            "<c:roundedCorners val='1'/>",
            "<c:spPr><a:solidFill><a:srgbClr val='FFFFFF'/></a:solidFill></c:spPr>",
        ] {
            let xml = LONG_LEGEND.replace("<c:chart>", &format!("{config}<c:chart>"));
            let doc = crate::xml::parse(xml.as_bytes()).unwrap();
            assert!(super::render_chart(
                &doc.root,
                576,
                336,
                FALLBACK_ACCENTS,
                None,
                ChartHost::Excel,
                None,
                ""
            )
            .is_ok());
        }
    }

    #[test]
    fn rounded_frame_has_straight_alpha_not_a_white_key() {
        let mut canvas = Canvas::new(80, 60);
        canvas.fill_rect(20, 20, 30, 30, [255; 3]);
        canvas.stroke_border(BORDER);
        let alpha = canvas.alpha.as_ref().unwrap();
        assert_eq!(alpha[0], 0);
        assert_eq!(&canvas.px[..3], &[0; 3]);
        assert_eq!(alpha[79], 0);
        assert_eq!(alpha[59 * 80], 0);
        assert_eq!(alpha[60 * 80 - 1], 0);
        assert_eq!(alpha[25 * 80 + 25], 255, "internal white stays opaque");
        assert_eq!(
            &canvas.px[(25 * 80 + 25) * 3..(25 * 80 + 25) * 3 + 3],
            &[255; 3]
        );
        let mut partial = 0;
        for (i, a) in alpha.iter().enumerate() {
            if *a > 0 && *a < 255 {
                partial += 1;
                assert_eq!(
                    &canvas.px[i * 3..i * 3 + 3],
                    &BORDER,
                    "unassociated edge RGB prevents white halos"
                );
            }
        }
        assert!(partial > 0);
        let png = canvas.encode_png().unwrap();
        let reader = png::Decoder::new(std::io::Cursor::new(png))
            .read_info()
            .unwrap();
        assert_eq!(reader.info().color_type, png::ColorType::Rgba);
    }

    #[test]
    fn rounded_white_composite_preserves_original_border_pixels() {
        for (w, h) in [(80, 60), (384, 240), (576, 336)] {
            let mut actual = Canvas::new(w, h);
            actual.stroke_border(BORDER);
            let mut old = Canvas::new(w, h);
            let (w, h) = (w as i32, h as i32);
            let r = CORNER_R.min(w / 2).min(h / 2);
            for (cx, cy, x0, x1, y0, y1) in [
                (r, r, 0, r, 0, r),
                (w - r, r, w - r, w - 1, 0, r),
                (r, h - r, 0, r, h - r, h - 1),
                (w - r, h - r, w - r, w - 1, h - r, h - 1),
            ] {
                let rad = f64::from(cx.min(cy).min(CORNER_R)) - 0.5;
                for y in y0..=y1 {
                    for x in x0..=x1 {
                        let dx = f64::from(x) + 0.5 - f64::from(cx);
                        let dy = f64::from(y) + 0.5 - f64::from(cy);
                        let cov = (1.0 - ((dx * dx + dy * dy).sqrt() - rad).abs()).clamp(0.0, 1.0);
                        old.blend(x, y, BORDER, (cov * 255.0).round() as u8);
                    }
                }
            }
            old.hline_i(r, w - 1 - r, 0, BORDER);
            old.hline_i(r, w - 1 - r, h - 1, BORDER);
            old.vline_i(0, r, h - 1 - r, BORDER);
            old.vline_i(w - 1, r, h - 1 - r, BORDER);
            for (i, a) in actual.alpha.as_ref().unwrap().iter().enumerate() {
                for k in 0..3 {
                    let composited = ((u16::from(actual.px[i * 3 + k]) * u16::from(*a)
                        + 255 * (255 - u16::from(*a))
                        + 127)
                        / 255) as u8;
                    assert_eq!(
                        composited,
                        old.px[i * 3 + k],
                        "white composite changed at {i}/{k}"
                    );
                }
            }
        }
    }

    #[test]
    fn unmasked_canvas_keeps_rgb_bytes_and_slide_export() {
        let canvas = Canvas::new(30, 20);
        assert_eq!(
            canvas.encode_png().unwrap(),
            encode_png(30, 20, &canvas.px, png::ColorType::Rgb, RenderDpi::DEFAULT).unwrap()
        );
        let doc = crate::xml::parse(LONG_LEGEND.as_bytes()).unwrap();
        for (host, expected) in [
            (ChartHost::Slide, png::ColorType::Rgb),
            (ChartHost::Excel, png::ColorType::Rgba),
        ] {
            let data =
                super::render_chart(&doc.root, 576, 336, FALLBACK_ACCENTS, None, host, None, "")
                    .unwrap();
            let reader = png::Decoder::new(std::io::Cursor::new(data))
                .read_info()
                .unwrap();
            assert_eq!(reader.info().color_type, expected);
        }
    }

    const LONG_LEGEND: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart">
  <c:chart>
    <c:plotArea>
      <c:barChart>
        <c:barDir val="col"/>
        <c:grouping val="clustered"/>
        <c:ser>
          <c:order val="0"/>
          <c:tx><c:v>2025 年</c:v></c:tx>
          <c:val><c:numLit><c:pt idx="0"><c:v>850</c:v></c:pt></c:numLit></c:val>
        </c:ser>
        <c:ser>
          <c:order val="1"/>
          <c:tx><c:v>2026 年</c:v></c:tx>
          <c:val><c:numLit><c:pt idx="0"><c:v>1160</c:v></c:pt></c:numLit></c:val>
        </c:ser>
      </c:barChart>
    </c:plotArea>
    <c:legend><c:legendPos val="r"/></c:legend>
  </c:chart>
</c:chartSpace>"#;

    #[cfg(windows)]
    fn grid_right(xml: &str) -> i32 {
        let doc = crate::xml::parse(xml.as_bytes()).unwrap();
        let png = render_rgb_chart(
            &doc.root,
            576,
            336,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .unwrap();
        let decoder = png::Decoder::new(std::io::Cursor::new(png));
        let mut reader = decoder.read_info().unwrap();
        let mut buf = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).unwrap();
        let w = info.width as i32;
        let bytes = &buf[..info.buffer_size()];
        let y = INSET_TOP;
        let mut right = None;
        for x in (0..w).rev() {
            let i = (y as usize * w as usize + x as usize) * 3;
            if bytes[i] == 0 && bytes[i + 1] == 0 && bytes[i + 2] == 0 {
                right = Some(x);
                break;
            }
        }
        right.expect("顶上网格不存在")
    }

    #[cfg(windows)]
    fn column_chart(name: &str) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart">
  <c:chart>
    <c:plotArea>
      <c:barChart>
        <c:barDir val="col"/>
        <c:grouping val="clustered"/>
        <c:ser>
          <c:order val="0"/>
          <c:tx><c:v>{name}</c:v></c:tx>
          <c:val><c:numLit><c:pt idx="0"><c:v>1</c:v></c:pt></c:numLit></c:val>
        </c:ser>
      </c:barChart>
      <c:valAx><c:majorGridlines/></c:valAx>
    </c:plotArea>
    <c:legend><c:legendPos val="r"/></c:legend>
  </c:chart>
</c:chartSpace>"#
        )
    }

    #[cfg(windows)]
    #[test]
    fn long_legend_uses_layout_width_and_short_names_keep_the_inset() {
        assert!(
            super::super::chart_gdi::Face::open((10.0_f64 * 96.0 / 72.0) as f32, false).is_some(),
            "本机应有图表字体"
        );
        assert_eq!(grid_right(&column_chart("S1")), 576 - INSET_RIGHT);
        let px = (10.0_f64 * 96.0 / 72.0) as f32;
        let face = super::super::chart_gdi::Face::open(px, false).expect("本机应有图表字体");
        let widest = face.layout_width("South") as i32;
        let need = LEGEND_PLOT_GAP + LEGEND_SWATCH + LEGEND_GAP + widest + LEGEND_PAD;
        let edge = 576 - need.max(INSET_RIGHT);
        assert!(
            edge < 576 - INSET_RIGHT,
            "长名字应把绘图区向左推 edge={edge}"
        );
        assert_eq!(grid_right(&column_chart("South")), edge);
    }

    #[test]
    fn category_ticks_paint_over_bars() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart">
  <c:chart>
    <c:plotArea>
      <c:barChart>
        <c:barDir val="col"/>
        <c:grouping val="clustered"/>
        <c:ser>
          <c:order val="0"/>
          <c:tx><c:v>S1</c:v></c:tx>
          <c:cat><c:strLit><c:pt idx="0"><c:v>A</c:v></c:pt></c:strLit></c:cat>
          <c:val><c:numLit><c:pt idx="0"><c:v>4</c:v></c:pt></c:numLit></c:val>
        </c:ser>
      </c:barChart>
      <c:valAx><c:majorGridlines/></c:valAx>
    </c:plotArea>
    <c:legend><c:legendPos val="r"/></c:legend>
  </c:chart>
</c:chartSpace>"#;
        let doc = crate::xml::parse(xml.as_bytes()).unwrap();
        let png = render_rgb_chart(
            &doc.root,
            576,
            336,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .unwrap();
        let decoder = png::Decoder::new(std::io::Cursor::new(png));
        let mut reader = decoder.read_info().unwrap();
        let mut buf = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).unwrap();
        let w = info.width as i32;
        let bytes = &buf[..info.buffer_size()];
        let at = |x: i32, y: i32| {
            let i = (y as usize * w as usize + x as usize) * 3;
            (bytes[i], bytes[i + 1], bytes[i + 2])
        };
        let axis_y = 336 - INSET_BOTTOM;
        let tick_y = axis_y - 1;
        let bar = |x: i32, y: i32| {
            let (r, g, b) = at(x, y);
            let hi = r.max(g).max(b);
            let lo = r.min(g).min(b);
            hi > 80 && hi - lo > 40
        };
        let ink = |x: i32, y: i32| at(x, y) == (0, 0, 0);
        let kept = (0..w).any(|x| ink(x, tick_y) && bar(x, tick_y - 12));
        assert!(kept, "柱底把类目刻度盖住了");
    }

    #[test]
    fn long_legend_label_stays_off_the_right_edge() {
        let doc = crate::xml::parse(LONG_LEGEND.as_bytes()).unwrap();
        let png = render_rgb_chart(
            &doc.root,
            576,
            336,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .unwrap();
        let decoder = png::Decoder::new(std::io::Cursor::new(png));
        let mut reader = decoder.read_info().unwrap();
        let mut buf = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).unwrap();
        let w = info.width as i32;
        let h = info.height as i32;
        let bytes = &buf[..info.buffer_size()];
        let px = |x: i32, y: i32| {
            let i = (y as usize * w as usize + x as usize) * 3;
            (bytes[i], bytes[i + 1], bytes[i + 2])
        };
        for x in (w - LEGEND_PAD)..(w - 1) {
            for y in CORNER_R..(h - CORNER_R) {
                let (r, g, b) = px(x, y);
                assert!(
                    r > 250 && g > 250 && b > 250,
                    "右缘被写上了像素 x={x} y={y} {r},{g},{b}"
                );
            }
        }
        let mut dark = false;
        for y in 0..h {
            for x in (w / 2)..(w - LEGEND_PAD) {
                let (r, g, b) = px(x, y);
                if r < 40 && g < 40 && b < 40 {
                    dark = true;
                }
            }
        }
        assert!(dark, "图例文字没有画到右侧");
    }

    #[test]
    fn frame_is_rounded_and_category_ticks_drop() {
        let doc = crate::xml::parse(LONG_LEGEND.as_bytes()).unwrap();
        let png = render_rgb_chart(
            &doc.root,
            576,
            336,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .unwrap();
        let decoder = png::Decoder::new(std::io::Cursor::new(png));
        let mut reader = decoder.read_info().unwrap();
        let mut buf = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).unwrap();
        let w = info.width as i32;
        let bytes = &buf[..info.buffer_size()];
        let px = |x: i32, y: i32| {
            let i = (y as usize * w as usize + x as usize) * 3;
            (bytes[i], bytes[i + 1], bytes[i + 2])
        };
        let (r, g, b) = px(0, 0);
        assert!(r > 250 && g > 250 && b > 250, "圆角外被涂了 {r},{g},{b}");
        assert_eq!(px(0, 20), (0x89, 0x89, 0x89));
        assert_eq!(px(20, 0), (0x89, 0x89, 0x89));
        let mut canvas = Canvas::new(200, 40);
        draw_cat_ticks(&mut canvas, 2, 10.0, 190.0, 8);
        draw_value_minor_ticks(&mut canvas, 20, 0, 25, 1, VAL_MINOR_PARTS);
        let ink = |x: i32, y: i32| {
            let i = (y as usize * canvas.w as usize + x as usize) * 3;
            canvas.px[i] < 40
        };
        for x in [10, 100, 190] {
            assert!(ink(x, 5), "主刻度没有向上 x={x}");
            assert!(ink(x, 12), "主刻度没画到 x={x}");
            assert!(!ink(x, 4), "主刻度向上超过 3 像素 x={x}");
        }
        for x in [55, 145] {
            assert!(ink(x, 6), "次刻度没有向上 x={x}");
            assert!(!ink(x, 5), "次刻度向上超过 2 像素 x={x}");
            assert!(ink(x, 11), "次刻度没画到 x={x}");
            assert!(!ink(x, 12), "次刻度画到了第 4 像素 x={x}");
        }
        for y in [5, 10, 15, 20] {
            assert!(ink(18, y), "数值轴次刻度太短 y={y}");
            assert!(ink(22, y), "数值轴次刻度没有向右 y={y}");
            assert!(!ink(17, y), "数值轴次刻度向左超过 2 像素 y={y}");
        }
    }

    const TITLED: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart">
  <c:chart>
    <c:title><c:tx><c:v>标题</c:v></c:tx></c:title>
    <c:plotArea>
      <c:barChart>
        <c:barDir val="col"/>
        <c:grouping val="clustered"/>
        <c:ser>
          <c:order val="0"/>
          <c:tx><c:v>2025 年</c:v></c:tx>
          <c:val><c:numLit><c:pt idx="0"><c:v>850</c:v></c:pt></c:numLit></c:val>
        </c:ser>
      </c:barChart>
      <c:valAx><c:majorGridlines/></c:valAx>
    </c:plotArea>
  </c:chart>
</c:chartSpace>"#;

    #[test]
    fn slide_host_drops_the_plot_and_skips_the_frame() {
        let doc = crate::xml::parse(TITLED.as_bytes()).unwrap();
        let excel = render_rgb_chart(
            &doc.root,
            576,
            336,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .unwrap();
        let slide = render_rgb_chart(
            &doc.root,
            576,
            336,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Slide,
            None,
            "",
        )
        .unwrap();
        let decode = |png: Vec<u8>| {
            let decoder = png::Decoder::new(std::io::Cursor::new(png));
            let mut reader = decoder.read_info().unwrap();
            let mut buf = vec![0; reader.output_buffer_size()];
            let info = reader.next_frame(&mut buf).unwrap();
            buf.truncate(info.buffer_size());
            (info.width as i32, info.height as i32, buf)
        };
        let (w, h, excel_px) = decode(excel);
        let (_, _, slide_px) = decode(slide);
        let at = |bytes: &[u8], x: i32, y: i32| {
            let i = (y as usize * w as usize + x as usize) * 3;
            (bytes[i], bytes[i + 1], bytes[i + 2])
        };
        assert_eq!(at(&excel_px, 0, 20), (0x89, 0x89, 0x89));
        assert_eq!(at(&slide_px, 0, 20), (255, 255, 255));
        let first_grid = |bytes: &[u8]| {
            (0..h)
                .find(|y| at(bytes, 80, *y) == (0, 0, 0))
                .expect("没有网格")
        };
        assert_eq!(
            first_grid(&slide_px),
            first_grid(&excel_px) + TITLE_DROP as i32
        );
    }

    #[test]
    fn slide_legend_keeps_the_category_span_from_growing() {
        let doc = crate::xml::parse(LONG_LEGEND.as_bytes()).unwrap();
        let render = |host| {
            let png = render_rgb_chart(&doc.root, 576, 336, FALLBACK_ACCENTS, None, host, None, "")
                .unwrap();
            let decoder = png::Decoder::new(std::io::Cursor::new(png));
            let mut reader = decoder.read_info().unwrap();
            let mut buf = vec![0; reader.output_buffer_size()];
            let info = reader.next_frame(&mut buf).unwrap();
            buf.truncate(info.buffer_size());
            (info.width as i32, buf)
        };
        let (w, excel) = render(ChartHost::Excel);
        let (_, slide) = render(ChartHost::Slide);
        let last_dark = |bytes: &[u8], y: i32| {
            (0..w).rev().find(|x| {
                let i = (y as usize * w as usize + *x as usize) * 3;
                bytes[i] < 40 && bytes[i + 1] < 40 && bytes[i + 2] < 40
            })
        };
        let y = 336 - INSET_BOTTOM;
        let excel_r = last_dark(&excel, y).expect("excel 没有横轴");
        let slide_r = last_dark(&slide, y).expect("slide 没有横轴");
        assert_eq!(excel_r, slide_r, "Excel 和幻灯片应按同一段排版宽度留白");
        let face = super::super::chart_gdi::Face::open((10.0_f64 * 96.0 / 72.0) as f32, false);
        if let Some(face) = face {
            let adv = face.width("2025 年").max(face.width("2026 年")).ceil() as i32;
            let advance_edge = 576
                - (LEGEND_PLOT_GAP + LEGEND_SWATCH + LEGEND_GAP + adv + LEGEND_PAD)
                    .max(INSET_RIGHT)
                + 1;
            assert!(
                excel_r + 4 <= advance_edge,
                "排版宽度应把绘图区向左推 excel={excel_r} advance={advance_edge}"
            );
        }
    }

    fn one_series(max: f64) -> Series {
        Series {
            name: "S".to_string(),
            values: vec![Some(max)],
            color: None,
            slot: 0,
            ..Series::default()
        }
    }

    #[test]
    fn axis_headroom_matches_excel_samples() {
        let cases = [
            (4.0, 4.5, 0.5),
            (3.9, 4.5, 0.5),
            (95.0, 100.0, 10.0),
            (96.0, 120.0, 20.0),
            (1100.0, 1200.0, 200.0),
            (1150.0, 1400.0, 200.0),
            (1160.0, 1400.0, 200.0),
        ];
        for (max, axis, step) in cases {
            let (lo, hi, got) = axis_range(&[one_series(max)]);
            assert!(
                (lo - 0.0).abs() < 1e-6 && (hi - axis).abs() < 1e-6 && (got - step).abs() < 1e-6,
                "max={max} got {lo}..{hi} step {got}"
            );
        }
    }

    #[test]
    fn axis_follows_tick_width() {
        assert_eq!(axis_x_of(7, RenderDpi::DEFAULT), 27);
        assert_eq!(axis_x_of(17, RenderDpi::DEFAULT), 37);
        assert_eq!(axis_x_of(28, RenderDpi::DEFAULT), 48);
        let mut dpi_obj = serde_json::Map::new();
        dpi_obj.insert("dpi".into(), serde_json::json!(106));
        let dpi106 = RenderDpi::from_request(&dpi_obj).unwrap();
        assert_eq!(dpi106.scale_px(INSET_TOP), 15);
        assert_eq!(dpi106.scale_px(INSET_BOTTOM), 38);
        assert_eq!(dpi106.scale_px(LABEL_EDGE), 10);
        assert_eq!(dpi106.scale_px(TICK_GAP), 12);
        assert_eq!(axis_x_of(7, dpi106), 29);
        assert_eq!(axis_x_of(19, dpi106), 41);
    }

    #[test]
    fn half_pixel_lands_on_even() {
        assert_eq!(round_half_even(149.5), 150);
        assert_eq!(round_half_even(165.5), 166);
        assert_eq!(round_half_even(250.5), 250);
        assert_eq!(round_half_even(394.5), 394);
        assert_eq!(round_half_even(88.25), 88);
        assert_eq!(round_half_even(54.2), 54);
    }

    #[test]
    fn two_cell_size_follows_columns_and_zoom() {
        let sheet = crate::xml::parse(
            br#"<worksheet>
              <sheetViews><sheetView zoomScale="95" workbookViewId="0"/></sheetViews>
              <cols>
                <col min="1" max="1" width="16.7109375"/>
                <col min="2" max="3" width="14.7109375"/>
                <col min="4" max="7" width="12.7109375"/>
                <col min="8" max="8" width="22.7109375"/>
              </cols>
            </worksheet>"#,
        )
        .unwrap();
        let anchor = crate::xml::parse(
            br#"<twoCellAnchor>
              <from><col>0</col><colOff>0</colOff><row>18</row><rowOff>0</rowOff></from>
              <to><col>7</col><colOff>0</colOff><row>32</row><rowOff>0</rowOff></to>
            </twoCellAnchor>"#,
        )
        .unwrap();
        let (w, h) = bitmap_px(
            &sheet.root,
            &anchor.root,
            crate::xlsx::picture::AnchorKind::TwoCell,
            &super::super::geometry::GeometryContext::calibri11(),
        )
        .unwrap();
        assert_eq!((w, h), (647, 252));
    }

    #[test]
    fn one_cell_at_full_zoom_keeps_ext() {
        let sheet = crate::xml::parse(br#"<worksheet/>"#).unwrap();
        let anchor = crate::xml::parse(
            br#"<oneCellAnchor><ext cx="5486400" cy="3200400"/></oneCellAnchor>"#,
        )
        .unwrap();
        let (w, h) = bitmap_px(
            &sheet.root,
            &anchor.root,
            crate::xlsx::picture::AnchorKind::OneCell,
            &super::super::geometry::GeometryContext::calibri11(),
        )
        .unwrap();
        assert_eq!((w, h), (576, 336));
    }

    #[test]
    fn missing_num_cache_reads_cell_values() {
        let xml =
            br#"<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart">
          <c:chart><c:plotArea><c:barChart>
            <c:barDir val="col"/><c:grouping val="clustered"/>
            <c:ser>
              <c:order val="0"/>
              <c:tx><c:strRef><c:f>Data!B1</c:f></c:strRef></c:tx>
              <c:cat><c:strRef><c:f>Data!A2:A3</c:f></c:strRef></c:cat>
              <c:val><c:numRef><c:f>Data!B2:B3</c:f></c:numRef></c:val>
            </c:ser>
          </c:barChart></c:plotArea></c:chart>
        </c:chartSpace>"#;
        let doc = crate::xml::parse(xml).unwrap();
        let missed = render_rgb_chart(
            &doc.root,
            320,
            200,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "Data",
        );
        assert!(missed.is_err());
        let cells = super::super::sparkline::CellCache::from_pairs(
            "Data",
            &[
                ((2, 1), serde_json::json!("系列")),
                ((1, 2), serde_json::json!("甲")),
                ((1, 3), serde_json::json!("乙")),
                ((2, 2), serde_json::json!(10)),
                ((2, 3), serde_json::json!(30)),
            ],
        );
        let png = render_rgb_chart(
            &doc.root,
            320,
            200,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            Some(&cells),
            "Data",
        )
        .unwrap();
        assert!(png.len() > 100);
    }

    fn line_space(plot_body: &str, chart_extra: &str) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <c:chart>
    {chart_extra}
    <c:plotArea>
      {plot_body}
      <c:valAx><c:majorGridlines/><c:crossBetween val="between"/></c:valAx>
      <c:catAx/>
    </c:plotArea>
    <c:legend><c:legendPos val="r"/></c:legend>
  </c:chart>
</c:chartSpace>"#
        )
    }

    fn standard_line_xml(series: &str, chart_extra: &str) -> String {
        line_space(
            &format!(
                r#"<c:lineChart>
        <c:grouping val="standard"/>
        <c:varyColors val="0"/>
        {series}
        <c:axId val="1"/><c:axId val="2"/>
      </c:lineChart>"#
            ),
            chart_extra,
        )
    }

    fn lit_pts(values: &[Option<f64>]) -> String {
        values
            .iter()
            .enumerate()
            .filter_map(|(idx, value)| {
                value.map(|v| format!(r#"<c:pt idx="{idx}"><c:v>{v}</c:v></c:pt>"#))
            })
            .collect()
    }

    fn line_ser(order: usize, name: &str, values: &[Option<f64>], extra: &str) -> String {
        let cats: String = (0..values.len())
            .map(|i| {
                format!(
                    r#"<c:pt idx="{i}"><c:v>{}</c:v></c:pt>"#,
                    (b'A' + i as u8) as char
                )
            })
            .collect();
        format!(
            r#"<c:ser>
          <c:order val="{order}"/>
          <c:tx><c:v>{name}</c:v></c:tx>
          {extra}
          <c:cat><c:strLit>{cats}</c:strLit></c:cat>
          <c:val><c:numLit>{pts}</c:numLit></c:val>
        </c:ser>"#,
            pts = lit_pts(values)
        )
    }

    fn render_line(xml: &str) -> Result<Vec<u8>, Error> {
        let doc = crate::xml::parse(xml.as_bytes()).unwrap();
        render_rgb_chart(
            &doc.root,
            576,
            336,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
    }

    fn png_rgb(png: &[u8]) -> (u32, u32, Vec<u8>) {
        let mut reader = png::Decoder::new(std::io::Cursor::new(png))
            .read_info()
            .unwrap();
        let mut buf = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).unwrap();
        let data = buf[..info.buffer_size()].to_vec();
        assert_eq!(info.color_type, png::ColorType::Rgb);
        (info.width, info.height, data)
    }

    fn near(a: [u8; 3], b: [u8; 3], tol: i16) -> bool {
        (0..3).all(|i| (i16::from(a[i]) - i16::from(b[i])).abs() <= tol)
    }

    fn count_in(
        rgb: &[u8],
        w: u32,
        x0: u32,
        x1: u32,
        y0: u32,
        y1: u32,
        target: [u8; 3],
        tol: i16,
    ) -> usize {
        let mut n = 0;
        for y in y0..y1 {
            for x in x0..x1 {
                let i = ((y * w + x) * 3) as usize;
                if near([rgb[i], rgb[i + 1], rgb[i + 2]], target, tol) {
                    n += 1;
                }
            }
        }
        n
    }

    #[test]
    fn standard_line_paints_series_colors_and_gap_breaks_the_stroke() {
        let continuous = standard_line_xml(
            &format!(
                "{}{}",
                line_ser(0, "S1", &[Some(20.0), Some(20.0), Some(20.0)], ""),
                line_ser(1, "S2", &[Some(5.0), Some(15.0), Some(8.0)], "")
            ),
            "",
        );
        let png = render_line(&continuous).unwrap();
        let (w, h, rgb) = png_rgb(&png);
        assert_eq!((w, h), (576, 336));
        let a1 = FALLBACK_ACCENTS[0];
        let a2 = FALLBACK_ACCENTS[1];
        assert!(
            count_in(&rgb, w, 0, w, 0, h, a1, 18) >= 20,
            "first series color should appear on the default line and markers"
        );
        assert!(
            count_in(&rgb, w, 0, w, 0, h, a2, 18) >= 20,
            "second series color should appear on the default line and markers"
        );

        let none = r#"<c:marker><c:symbol val="none"/></c:marker>"#;
        let solid = standard_line_xml(
            &line_ser(0, "S1", &[Some(20.0), Some(20.0), Some(20.0)], none),
            "",
        );
        let gapped = standard_line_xml(
            &line_ser(0, "S1", &[Some(20.0), None, Some(20.0)], none),
            "",
        );
        let (_, _, solid_rgb) = png_rgb(&render_line(&solid).unwrap());
        let (_, _, gap_rgb) = png_rgb(&render_line(&gapped).unwrap());
        let mid0 = w / 3;
        let mid1 = (w * 2) / 3;
        let solid_mid = count_in(&solid_rgb, w, mid0, mid1, 0, h, a1, 24);
        let gap_mid = count_in(&gap_rgb, w, mid0, mid1, 0, h, a1, 24);
        assert!(
            solid_mid >= 10,
            "a continuous line should cross the middle category, got {solid_mid}"
        );
        assert_eq!(
            gap_mid, 0,
            "a missing value must break the stroke, not span the gap; got {gap_mid}"
        );
        let gap_sides = count_in(&gap_rgb, w, 0, mid0, 0, h, a1, 24)
            + count_in(&gap_rgb, w, mid1, w, 0, h, a1, 24);
        assert!(
            gap_sides >= 8,
            "the remaining points of a gapped series should still be drawn"
        );
    }

    #[test]
    fn line_marker_none_is_not_the_same_as_default_marker() {
        let values = [Some(8.0), Some(16.0), Some(12.0)];
        let default_xml = standard_line_xml(&line_ser(0, "S1", &values, ""), "");
        let none_xml = standard_line_xml(
            &line_ser(
                0,
                "S1",
                &values,
                r#"<c:marker><c:symbol val="none"/></c:marker>"#,
            ),
            "",
        );
        let (_, _, default_rgb) = png_rgb(&render_line(&default_xml).unwrap());
        let (_, _, none_rgb) = png_rgb(&render_line(&none_xml).unwrap());
        let a1 = FALLBACK_ACCENTS[0];
        let default_n = count_in(&default_rgb, 576, 0, 576, 0, 336, a1, 18);
        let none_n = count_in(&none_rgb, 576, 0, 576, 0, 336, a1, 18);
        assert!(
            default_n > none_n,
            "omitted markers should paint the default symbol, so they must add ink; default={default_n} none={none_n}"
        );
        assert!(none_n >= 8, "a visible solid line should still leave ink");
    }

    fn marker_el(symbol: &str, size: f64) -> String {
        format!(r#"<c:marker><c:symbol val="{symbol}"/><c:size val="{size}"/></c:marker>"#)
    }

    const AUTO_MARKER_SYMBOLS: [&str; 9] = [
        "diamond", "square", "triangle", "x", "star", "circle", "plus", "dot", "dash",
    ];

    fn line_series_with_markers(
        count: usize,
        values: &[Option<f64>],
        extra: impl Fn(usize) -> String,
    ) -> String {
        (0..count)
            .map(|i| line_ser(i, &format!("S{}", i + 1), values, &extra(i)))
            .collect::<String>()
    }

    fn scatter_series_with_markers(
        count: usize,
        xs: &[Option<f64>],
        ys: &[Option<f64>],
        extra: impl Fn(usize) -> String,
    ) -> String {
        (0..count)
            .map(|i| scatter_ser(i, &format!("S{}", i + 1), xs, ys, &extra(i)))
            .collect::<String>()
    }

    #[test]
    fn omitted_line_markers_follow_excel_auto_cycle_at_size_9() {
        let values = [Some(8.0), Some(16.0), Some(12.0)];
        let omitted = standard_line_xml(
            &line_series_with_markers(10, &values, |_| String::new()),
            "",
        );
        let explicit = standard_line_xml(
            &line_series_with_markers(10, &values, |i| {
                marker_el(AUTO_MARKER_SYMBOLS[i % AUTO_MARKER_SYMBOLS.len()], 9.0)
            }),
            "",
        );
        assert_eq!(
            render_line(&omitted).unwrap(),
            render_line(&explicit).unwrap(),
            "omitted c:marker must match the 9-symbol auto cycle at 9pt, wrapping at series 10"
        );
        let size5 = standard_line_xml(
            &line_series_with_markers(10, &values, |i| {
                marker_el(AUTO_MARKER_SYMBOLS[i % AUTO_MARKER_SYMBOLS.len()], 5.0)
            }),
            "",
        );
        assert_ne!(
            render_line(&omitted).unwrap(),
            render_line(&size5).unwrap(),
            "omitted markers use 9pt, not the OOXML explicit-marker default of 5pt"
        );
    }

    #[test]
    fn explicit_line_marker_symbol_wins_over_the_auto_cycle() {
        let values = [Some(8.0), Some(16.0), Some(12.0)];
        let omitted = standard_line_xml(&line_ser(0, "S1", &values, ""), "");
        let circle = standard_line_xml(&line_ser(0, "S1", &values, &marker_el("circle", 9.0)), "");
        let square = standard_line_xml(&line_ser(0, "S1", &values, &marker_el("square", 9.0)), "");
        let omitted_png = render_line(&omitted).unwrap();
        let circle_png = render_line(&circle).unwrap();
        let square_png = render_line(&square).unwrap();
        assert_ne!(
            omitted_png, circle_png,
            "series 0 default is diamond, so an explicit 9pt circle must differ"
        );
        assert_ne!(
            omitted_png, square_png,
            "an explicit square on series 0 must not be replaced by the default diamond"
        );
        render_line(&standard_line_xml(
            &line_ser(0, "S1", &values, &marker_el("diamond", 5.0)),
            "",
        ))
        .unwrap();
    }

    #[test]
    fn omitted_scatter_markers_follow_the_same_auto_cycle() {
        let xs = [Some(0.0), Some(4.0), Some(8.0)];
        let ys = [Some(1.0), Some(3.0), Some(2.0)];
        let omitted = scatter_xml(
            "lineMarker",
            &scatter_series_with_markers(10, &xs, &ys, |_| String::new()),
            "",
        );
        let explicit = scatter_xml(
            "lineMarker",
            &scatter_series_with_markers(10, &xs, &ys, |i| {
                marker_el(AUTO_MARKER_SYMBOLS[i % AUTO_MARKER_SYMBOLS.len()], 9.0)
            }),
            "",
        );
        assert_eq!(
            render_scatter(&omitted).unwrap(),
            render_scatter(&explicit).unwrap(),
            "omitted scatter markers share the line auto cycle at 9pt"
        );
        let circle = scatter_xml(
            "lineMarker",
            &scatter_ser(0, "S1", &xs, &ys, &marker_el("circle", 9.0)),
            "",
        );
        let omitted_s1 = scatter_xml("lineMarker", &scatter_ser(0, "S1", &xs, &ys, ""), "");
        assert_ne!(
            render_scatter(&omitted_s1).unwrap(),
            render_scatter(&circle).unwrap(),
            "explicit circle must win over the default diamond on scatter series 0"
        );
    }

    #[test]
    fn unsupported_line_inputs_stay_not_implemented() {
        let base = line_ser(0, "S1", &[Some(1.0), Some(2.0), Some(3.0)], "");
        let cases = [
            standard_line_xml(&base, "")
                .replace(r#"grouping val="standard""#, r#"grouping val="stacked""#),
            standard_line_xml(&base, "").replace(
                r#"grouping val="standard""#,
                r#"grouping val="percentStacked""#,
            ),
            standard_line_xml(
                &line_ser(
                    0,
                    "S1",
                    &[Some(1.0), Some(2.0), Some(3.0)],
                    r#"<c:smooth val="1"/>"#,
                ),
                "",
            ),
            standard_line_xml(&base, r#"<c:dispBlanksAs val="zero"/>"#),
            standard_line_xml(&base, r#"<c:dispBlanksAs val="span"/>"#),
            standard_line_xml(
                &line_ser(
                    0,
                    "S1",
                    &[Some(1.0), Some(2.0), Some(3.0)],
                    r#"<c:spPr><a:ln w="28575"><a:prstDash val="dash"/><a:solidFill><a:srgbClr val="156082"/></a:solidFill></a:ln></c:spPr>"#,
                ),
                "",
            ),
            standard_line_xml(&base, "").replace(r#"varyColors val="0""#, r#"varyColors val="1""#),
            line_space(
                r#"<c:line3DChart><c:grouping val="standard"/><c:ser>
          <c:order val="0"/><c:val><c:numLit><c:pt idx="0"><c:v>1</c:v></c:pt></c:numLit></c:val>
        </c:ser></c:line3DChart>"#,
                "",
            ),
            line_space(
                r#"<c:barChart><c:barDir val="col"/><c:grouping val="clustered"/><c:ser>
          <c:order val="0"/><c:val><c:numLit><c:pt idx="0"><c:v>1</c:v></c:pt></c:numLit></c:val>
        </c:ser></c:barChart><c:lineChart><c:grouping val="standard"/><c:ser>
          <c:order val="1"/><c:val><c:numLit><c:pt idx="0"><c:v>2</c:v></c:pt></c:numLit></c:val>
        </c:ser></c:lineChart>"#,
                "",
            ),
            line_space(
                r#"<c:pieChart><c:ser>
          <c:order val="0"/><c:val><c:numLit><c:pt idx="0"><c:v>1</c:v></c:pt></c:numLit></c:val>
        </c:ser></c:pieChart>"#,
                "",
            ),
            standard_line_xml(&base, "").replace(
                r#"crossBetween val="between""#,
                r#"crossBetween val="midCat""#,
            ),
        ];
        for (i, xml) in cases.iter().enumerate() {
            let err = render_line(xml).expect_err(&format!("case {i} should stay NotImplemented"));
            assert_eq!(
                err.code(),
                crate::error::ErrorCode::NotImplemented,
                "case {i} should stay NotImplemented"
            );
        }

        // A supported standard line still renders, so the counterexamples are not a broken harness.
        render_line(&standard_line_xml(&base, "")).unwrap();

        render_line(&standard_line_xml(
            &line_ser(
                0,
                "S1",
                &[Some(1.0), Some(2.0), Some(3.0)],
                r#"<c:marker><c:symbol val="circle"/><c:size val="5"/></c:marker>"#,
            ),
            r#"<c:dispBlanksAs val="gap"/>"#,
        ))
        .unwrap();
    }

    fn scatter_space(plot_body: &str, chart_extra: &str) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <c:chart>
    {chart_extra}
    <c:plotArea>
      {plot_body}
      <c:valAx><c:axId val="1"/><c:scaling><c:orientation val="minMax"/></c:scaling><c:axPos val="b"/><c:crosses val="autoZero"/></c:valAx>
      <c:valAx><c:axId val="2"/><c:scaling><c:orientation val="minMax"/></c:scaling><c:axPos val="l"/><c:majorGridlines/><c:crosses val="autoZero"/></c:valAx>
    </c:plotArea>
    <c:legend><c:legendPos val="r"/></c:legend>
  </c:chart>
</c:chartSpace>"#
        )
    }

    fn scatter_xml(style: &str, series: &str, chart_extra: &str) -> String {
        scatter_space(
            &format!(
                r#"<c:scatterChart>
        <c:scatterStyle val="{style}"/>
        <c:varyColors val="0"/>
        {series}
        <c:axId val="1"/><c:axId val="2"/>
      </c:scatterChart>"#
            ),
            chart_extra,
        )
    }

    fn scatter_ser(
        order: usize,
        name: &str,
        xs: &[Option<f64>],
        ys: &[Option<f64>],
        extra: &str,
    ) -> String {
        format!(
            r#"<c:ser>
          <c:order val="{order}"/>
          <c:tx><c:v>{name}</c:v></c:tx>
          {extra}
          <c:xVal><c:numLit>{xs}</c:numLit></c:xVal>
          <c:yVal><c:numLit>{ys}</c:numLit></c:val>
        </c:ser>"#,
            xs = lit_pts(xs),
            ys = lit_pts(ys)
        )
        .replace("</c:val>", "</c:yVal>")
    }

    fn render_scatter(xml: &str) -> Result<Vec<u8>, Error> {
        render_line(xml)
    }

    #[test]
    fn marked_scatter_uses_numeric_xy_and_paints_series_colors() {
        let xml = scatter_xml(
            "lineMarker",
            &format!(
                "{}{}",
                scatter_ser(
                    0,
                    "S1",
                    &[Some(0.0), Some(8.0)],
                    &[Some(1.0), Some(1.0)],
                    ""
                ),
                scatter_ser(
                    1,
                    "S2",
                    &[Some(1.0), Some(7.0)],
                    &[Some(4.0), Some(6.0)],
                    ""
                )
            ),
            "",
        );
        let png = render_scatter(&xml).unwrap();
        let (w, h, rgb) = png_rgb(&png);
        assert_eq!((w, h), (576, 336));
        let a1 = FALLBACK_ACCENTS[0];
        let a2 = FALLBACK_ACCENTS[1];
        assert!(
            count_in(&rgb, w, 0, w, 0, h, a1, 18) >= 20,
            "first scatter series color should appear"
        );
        assert!(
            count_in(&rgb, w, 0, w, 0, h, a2, 18) >= 20,
            "second scatter series color should appear"
        );

        // A lone point at the right of the X scale must not sit on a category midpoint.
        let right = scatter_xml(
            "marker",
            &scatter_ser(0, "S1", &[Some(8.0)], &[Some(2.0)], ""),
            "",
        );
        let (_, _, right_rgb) = png_rgb(&render_scatter(&right).unwrap());
        let left_count = count_in(&right_rgb, w, 0, w / 2, 0, h, a1, 18);
        let right_count = count_in(&right_rgb, w, w / 2, w, 0, h, a1, 18);
        assert!(
            right_count > left_count,
            "x=8 on a 0..=8-like scale should land in the right half, not a single category mid-slot; left={left_count} right={right_count}"
        );
    }

    #[test]
    fn scatter_gap_breaks_the_stroke_and_marker_style_drops_the_line() {
        let none = r#"<c:marker><c:symbol val="none"/></c:marker>"#;
        let solid = scatter_xml(
            "lineMarker",
            &scatter_ser(
                0,
                "S1",
                &[Some(0.0), Some(5.0), Some(10.0)],
                &[Some(8.0), Some(8.0), Some(8.0)],
                none,
            ),
            "",
        );
        let gapped = scatter_xml(
            "lineMarker",
            &scatter_ser(
                0,
                "S1",
                &[Some(0.0), None, Some(10.0)],
                &[Some(8.0), Some(8.0), Some(8.0)],
                none,
            ),
            "",
        );
        let (_, _, solid_rgb) = png_rgb(&render_scatter(&solid).unwrap());
        let (_, _, gap_rgb) = png_rgb(&render_scatter(&gapped).unwrap());
        let a1 = FALLBACK_ACCENTS[0];
        let mid0 = 576 / 3;
        let mid1 = (576 * 2) / 3;
        let solid_mid = count_in(&solid_rgb, 576, mid0, mid1, 0, 336, a1, 24);
        let gap_mid = count_in(&gap_rgb, 576, mid0, mid1, 0, 336, a1, 24);
        assert!(
            solid_mid >= 10,
            "a lineMarker series should cross the middle x when every point is present, got {solid_mid}"
        );
        assert_eq!(
            gap_mid, 0,
            "a missing x or y must break the scatter stroke; got {gap_mid}"
        );

        let markers = scatter_xml(
            "marker",
            &scatter_ser(
                0,
                "S1",
                &[Some(0.0), Some(5.0), Some(10.0)],
                &[Some(8.0), Some(8.0), Some(8.0)],
                "",
            ),
            "",
        );
        let (_, _, marker_rgb) = png_rgb(&render_scatter(&markers).unwrap());
        let marker_mid = count_in(&marker_rgb, 576, mid0 + 20, mid1 - 20, 0, 336, a1, 18);
        // The middle category of a marker-only series may contain a marker at x=5.
        // A band between the first and middle marker should stay empty of series ink
        // if we are not stroking a line.
        let between = count_in(&marker_rgb, 576, 576 / 6, mid0 - 10, 0, 336, a1, 18);
        assert_eq!(
            between, 0,
            "scatterStyle=marker must not draw a connecting stroke; got {between} (mid markers {marker_mid})"
        );
    }

    #[test]
    fn unsupported_scatter_inputs_stay_not_implemented() {
        let ser = scatter_ser(
            0,
            "S1",
            &[Some(1.0), Some(2.0), Some(3.0)],
            &[Some(4.0), Some(5.0), Some(6.0)],
            "",
        );
        let cases = [
            scatter_xml("line", &ser, ""),
            scatter_xml("smoothMarker", &ser, ""),
            scatter_xml("smooth", &ser, ""),
            scatter_xml(
                "lineMarker",
                &scatter_ser(
                    0,
                    "S1",
                    &[Some(1.0), Some(2.0)],
                    &[Some(3.0), Some(4.0)],
                    r#"<c:spPr><a:ln w="28575"><a:prstDash val="dash"/><a:solidFill><a:srgbClr val="156082"/></a:solidFill></a:ln></c:spPr>"#,
                ),
                "",
            ),
            scatter_xml("lineMarker", &ser, r#"<c:dispBlanksAs val="zero"/>"#),
            scatter_xml("lineMarker", &ser, r#"<c:dispBlanksAs val="span"/>"#),
            scatter_xml("lineMarker", &ser, "")
                .replace(r#"varyColors val="0""#, r#"varyColors val="1""#),
            scatter_space(
                r#"<c:bubbleChart><c:ser>
          <c:order val="0"/><c:xVal><c:numLit><c:pt idx="0"><c:v>1</c:v></c:pt></c:numLit></c:xVal>
          <c:yVal><c:numLit><c:pt idx="0"><c:v>2</c:v></c:pt></c:numLit></c:yVal>
        </c:ser></c:bubbleChart>"#,
                "",
            ),
            scatter_xml("lineMarker", &ser, "").replace(
                "</c:plotArea>",
                r#"<c:catAx><c:axId val="3"/><c:axPos val="b"/></c:catAx></c:plotArea>"#,
            ),
            scatter_xml(
                "lineMarker",
                &scatter_ser(0, "S1", &[None, None], &[Some(1.0), Some(2.0)], ""),
                "",
            ),
        ];
        for (i, xml) in cases.iter().enumerate() {
            let err = render_scatter(xml)
                .expect_err(&format!("scatter case {i} should stay NotImplemented"));
            assert_eq!(
                err.code(),
                crate::error::ErrorCode::NotImplemented,
                "scatter case {i} should stay NotImplemented"
            );
        }
        render_scatter(&scatter_xml("marker", &ser, "")).unwrap();
        render_scatter(&scatter_xml("lineMarker", &ser, "")).unwrap();
        // Omitted scatterStyle follows Excel's lineMarker default.
        render_scatter(
            &scatter_xml("lineMarker", &ser, "")
                .replace(r#"<c:scatterStyle val="lineMarker"/>"#, ""),
        )
        .unwrap();
    }

    fn bar_space(plot_body: &str, chart_extra: &str, cat_orient: &str) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <c:chart>
    {chart_extra}
    <c:plotArea>
      {plot_body}
      <c:catAx><c:axId val="1"/><c:scaling><c:orientation val="{cat_orient}"/></c:scaling><c:delete val="0"/><c:axPos val="l"/><c:crossAx val="2"/><c:crosses val="autoZero"/></c:catAx>
      <c:valAx><c:axId val="2"/><c:scaling><c:orientation val="minMax"/></c:scaling><c:delete val="0"/><c:axPos val="b"/><c:majorGridlines/><c:crossAx val="1"/><c:crosses val="autoZero"/><c:crossBetween val="between"/></c:valAx>
    </c:plotArea>
    <c:legend><c:legendPos val="r"/></c:legend>
  </c:chart>
</c:chartSpace>"#
        )
    }

    fn bar_xml(series: &str, chart_extra: &str, cat_orient: &str, dir: Option<&str>) -> String {
        let dir_el = match dir {
            Some(dir) => format!(r#"<c:barDir val="{dir}"/>"#),
            None => String::new(),
        };
        bar_space(
            &format!(
                r#"<c:barChart>
        {dir_el}
        <c:grouping val="clustered"/>
        <c:varyColors val="0"/>
        {series}
        <c:axId val="1"/><c:axId val="2"/>
      </c:barChart>"#
            ),
            chart_extra,
            cat_orient,
        )
    }

    fn render_bar(xml: &str) -> Result<Vec<u8>, Error> {
        render_line(xml)
    }

    fn centroid_y(
        rgb: &[u8],
        w: u32,
        x0: u32,
        x1: u32,
        y0: u32,
        y1: u32,
        target: [u8; 3],
        tol: i16,
    ) -> Option<f64> {
        let mut sum = 0u64;
        let mut n = 0u64;
        for y in y0..y1 {
            for x in x0..x1 {
                let i = ((y * w + x) * 3) as usize;
                if near([rgb[i], rgb[i + 1], rgb[i + 2]], target, tol) {
                    sum += u64::from(y);
                    n += 1;
                }
            }
        }
        (n > 0).then(|| sum as f64 / n as f64)
    }

    #[test]
    fn clustered_bar_places_first_category_by_orientation() {
        let series = line_ser(0, "S1", &[Some(8.0), Some(1.0)], "");
        let a1 = FALLBACK_ACCENTS[0];
        let (w, h) = (576u32, 336u32);
        // 长条在右半边：maxMin 时第一类在上，minMax 时第一类在下。
        let top_xml = bar_xml(&series, "", "maxMin", Some("bar"));
        let (_, _, top_rgb) = png_rgb(&render_bar(&top_xml).unwrap());
        let top_right = count_in(&top_rgb, w, w / 2, w * 3 / 4, 20, h / 2, a1, 18);
        let bot_right = count_in(&top_rgb, w, w / 2, w * 3 / 4, h / 2, h - 20, a1, 18);
        assert!(
            top_right > bot_right,
            "maxMin should put the long first category in the top half; top={top_right} bottom={bot_right}"
        );

        let bot_xml = bar_xml(&series, "", "minMax", Some("bar"));
        let (_, _, bot_rgb) = png_rgb(&render_bar(&bot_xml).unwrap());
        let top_right = count_in(&bot_rgb, w, w / 2, w * 3 / 4, 20, h / 2, a1, 18);
        let bot_right = count_in(&bot_rgb, w, w / 2, w * 3 / 4, h / 2, h - 20, a1, 18);
        assert!(
            bot_right > top_right,
            "minMax should put the long first category in the bottom half; top={top_right} bottom={bot_right}"
        );
    }

    #[test]
    fn clustered_bar_keeps_series_order_and_skips_missing_values() {
        let a1 = FALLBACK_ACCENTS[0];
        let a2 = FALLBACK_ACCENTS[1];
        let (w, h) = (576u32, 336u32);
        // 单类两系列：系列 0 在类目轴方向的槽起点（第一类在上时是槽顶）。
        let ordered = bar_xml(
            &format!(
                "{}{}",
                line_ser(0, "S1", &[Some(8.0)], ""),
                line_ser(1, "S2", &[Some(8.0)], "")
            ),
            "",
            "maxMin",
            Some("bar"),
        );
        let (_, _, rgb) = png_rgb(&render_bar(&ordered).unwrap());
        let y1 = centroid_y(&rgb, w, 80, 450, 20, 300, a1, 18).expect("series 0 color");
        let y2 = centroid_y(&rgb, w, 80, 450, 20, 300, a2, 18).expect("series 1 color");
        assert!(
            y1 < y2,
            "series 0 should sit above series 1 when the category starts at the top; y1={y1} y2={y2}"
        );

        // minMax：类目轴从下往上，槽起点在底部，系列 0 应在系列 1 下方。
        let ordered_minmax = bar_xml(
            &format!(
                "{}{}",
                line_ser(0, "S1", &[Some(8.0)], ""),
                line_ser(1, "S2", &[Some(8.0)], "")
            ),
            "",
            "minMax",
            Some("bar"),
        );
        let (_, _, rgb_min) = png_rgb(&render_bar(&ordered_minmax).unwrap());
        let y1_min =
            centroid_y(&rgb_min, w, 80, 450, 20, 300, a1, 18).expect("series 0 color minMax");
        let y2_min =
            centroid_y(&rgb_min, w, 80, 450, 20, 300, a2, 18).expect("series 1 color minMax");
        assert!(
            y1_min > y2_min,
            "series 0 should sit below series 1 when the category starts at the bottom; y1={y1_min} y2={y2_min}"
        );

        // 缺值不把后续条挤进空位：S1 只在第一类，S2 只在第二类。
        let missing = bar_xml(
            &format!(
                "{}{}",
                line_ser(0, "S1", &[Some(8.0), None], ""),
                line_ser(1, "S2", &[None, Some(8.0)], "")
            ),
            "",
            "maxMin",
            Some("bar"),
        );
        let (_, _, miss) = png_rgb(&render_bar(&missing).unwrap());
        let s1_top = count_in(&miss, w, 80, 450, 20, h / 2, a1, 18);
        let s1_bot = count_in(&miss, w, 80, 450, h / 2, 300, a1, 18);
        let s2_top = count_in(&miss, w, 80, 450, 20, h / 2, a2, 18);
        let s2_bot = count_in(&miss, w, 80, 450, h / 2, 300, a2, 18);
        assert!(
            s1_top > s1_bot && s2_bot > s2_top,
            "missing values should leave the other series in its own category; s1={s1_top}/{s1_bot} s2={s2_top}/{s2_bot}"
        );
    }

    #[test]
    fn clustered_bar_grows_from_zero_and_differs_from_columns() {
        let a1 = FALLBACK_ACCENTS[0];
        let (w, h) = (576u32, 336u32);
        let signed = bar_xml(
            &line_ser(0, "S1", &[Some(-8.0), Some(8.0)], ""),
            "",
            "maxMin",
            Some("bar"),
        );
        let png = render_bar(&signed).unwrap();
        let (_, _, rgb) = png_rgb(&png);
        let top_left = count_in(&rgb, w, 40, w / 2, 20, h / 2, a1, 18);
        let top_right = count_in(&rgb, w, w / 2, 500, 20, h / 2, a1, 18);
        let bot_left = count_in(&rgb, w, 40, w / 2, h / 2, 300, a1, 18);
        let bot_right = count_in(&rgb, w, w / 2, 500, h / 2, 300, a1, 18);
        assert!(
            top_left > top_right,
            "negative first category should grow left in the top half; left={top_left} right={top_right}"
        );
        assert!(
            bot_right > bot_left,
            "positive second category should grow right in the bottom half; left={bot_left} right={bot_right}"
        );

        let maxed = bar_xml(
            &line_ser(0, "S1", &[Some(8.0)], ""),
            "",
            "maxMin",
            Some("bar"),
        );
        let (_, _, max_rgb) = png_rgb(&render_bar(&maxed).unwrap());
        let near_right = count_in(&max_rgb, w, 400, 520, 20, 300, a1, 18);
        assert!(
            near_right > 0,
            "a maximum positive bar should reach the right of the value axis; count={near_right}"
        );

        // 同一组数据的柱形图是竖向的，像素不应与条形图相同。
        let col_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart">
  <c:chart>
    <c:plotArea>
      <c:barChart>
        <c:barDir val="col"/>
        <c:grouping val="clustered"/>
        <c:varyColors val="0"/>
"#
        .to_string()
            + &line_ser(0, "S1", &[Some(-8.0), Some(8.0)], "")
            + r#"
        <c:axId val="1"/><c:axId val="2"/>
      </c:barChart>
      <c:catAx><c:axId val="1"/><c:scaling><c:orientation val="minMax"/></c:scaling><c:axPos val="b"/></c:catAx>
      <c:valAx><c:axId val="2"/><c:scaling><c:orientation val="minMax"/></c:scaling><c:axPos val="l"/><c:majorGridlines/></c:valAx>
    </c:plotArea>
    <c:legend><c:legendPos val="r"/></c:legend>
  </c:chart>
</c:chartSpace>"#;
        let col_png = render_bar(&col_xml).unwrap();
        assert_ne!(
            png, col_png,
            "clustered bars and clustered columns should not paint the same pixels"
        );
        let (_, _, col_rgb) = png_rgb(&col_png);
        let col_near_right = count_in(&col_rgb, w, 400, 520, 20, 300, a1, 18);
        assert!(
            near_right > col_near_right,
            "a horizontal max bar should occupy the right value edge more than a vertical column; bar={near_right} col={col_near_right}"
        );

        // 省略 barDir 时 OOXML 缺省为 bar。
        render_bar(&bar_xml(
            &line_ser(0, "S1", &[Some(4.0)], ""),
            "",
            "maxMin",
            None,
        ))
        .unwrap();
    }

    #[test]
    fn clustered_bar_unsupported_inputs_stay_not_implemented() {
        let ser = line_ser(0, "S1", &[Some(1.0), Some(2.0)], "");
        let two = format!(
            "{}{}",
            line_ser(0, "S1", &[Some(1.0), Some(2.0)], ""),
            line_ser(1, "S2", &[Some(3.0), Some(4.0)], "")
        );
        let base = bar_xml(&ser, "", "maxMin", Some("bar"));
        let cases = [
            bar_xml(&ser, "", "maxMin", Some("bar")).replacen(
                r#"grouping val="clustered""#,
                r#"grouping val="stacked""#,
                1,
            ),
            bar_xml(&ser, "", "maxMin", Some("bar")).replacen(
                r#"grouping val="clustered""#,
                r#"grouping val="percentStacked""#,
                1,
            ),
            bar_xml(
                &ser,
                r#"<c:dispBlanksAs val="zero"/>"#,
                "maxMin",
                Some("bar"),
            ),
            bar_xml(
                &ser,
                r#"<c:dispBlanksAs val="span"/>"#,
                "maxMin",
                Some("bar"),
            ),
            bar_xml(&ser, "", "maxMin", Some("bar")).replacen(
                r#"</c:barChart>"#,
                r#"<c:dLbls><c:showVal val="1"/></c:dLbls></c:barChart>"#,
                1,
            ),
            bar_xml(&two, "", "maxMin", Some("bar")).replacen(
                r#"varyColors val="0""#,
                r#"varyColors val="1""#,
                1,
            ),
            // 柱形轴位不能 silently 当条形画。
            bar_xml(&ser, "", "maxMin", Some("bar"))
                .replace(r#"<c:axPos val="l"/>"#, r#"<c:axPos val="b"/>"#),
            bar_xml(&ser, "", "maxMax", Some("bar")),
            base.replace("<c:catAx>", r#"<c:catAx><c:majorGridlines/>"#),
            base.replacen(r#"<c:delete val="0"/>"#, r#"<c:delete val="1"/>"#, 1),
            base.replace(
                "<c:catAx>",
                r#"<c:catAx><c:scaling><c:logBase val="10"/></c:scaling>"#,
            )
            .replacen(
                r#"<c:scaling><c:orientation val="maxMin"/></c:scaling>"#,
                "",
                1,
            ),
            base.replace("</c:plotArea>", "<c:dateAx/></c:plotArea>"),
            base.replace("<c:barChart>", "<c:bar3DChart>")
                .replace("</c:barChart>", "</c:bar3DChart>"),
        ];
        for (i, xml) in cases.iter().enumerate() {
            let err =
                render_bar(xml).expect_err(&format!("bar case {i} should stay NotImplemented"));
            assert_eq!(
                err.code(),
                crate::error::ErrorCode::NotImplemented,
                "bar case {i} should stay NotImplemented"
            );
        }
        // 隐藏数据标签与空值缺口仍可画；gap 是缺省。
        render_bar(&bar_xml(
            &ser,
            r#"<c:dispBlanksAs val="gap"/>"#,
            "maxMin",
            Some("bar"),
        )
        .replacen(
            r#"</c:barChart>"#,
            r#"<c:dLbls><c:showVal val="0"/><c:showCatName val="0"/><c:showSerName val="0"/><c:showPercent val="0"/><c:showLegendKey val="0"/><c:showBubbleSize val="0"/></c:dLbls></c:barChart>"#,
            1,
        ))
        .unwrap();
        // 已支持的簇状柱仍可画，不因条形检查被误伤。
        let col_ok = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart">
  <c:chart>
    <c:plotArea>
      <c:barChart>
        <c:barDir val="col"/>
        <c:grouping val="clustered"/>
"#
        .to_string()
            + &ser
            + r#"
      </c:barChart>
      <c:valAx><c:majorGridlines/></c:valAx>
    </c:plotArea>
  </c:chart>
</c:chartSpace>"#;
        render_bar(&col_ok).unwrap();
    }

    fn area_space(plot_body: &str, chart_extra: &str) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <c:chart>
    {chart_extra}
    <c:plotArea>
      {plot_body}
      <c:valAx><c:majorGridlines/><c:crossBetween val="between"/></c:valAx>
      <c:catAx/>
    </c:plotArea>
    <c:legend><c:legendPos val="r"/></c:legend>
  </c:chart>
</c:chartSpace>"#
        )
    }

    fn standard_area_xml(series: &str, chart_extra: &str) -> String {
        area_space(
            &format!(
                r#"<c:areaChart>
        <c:grouping val="standard"/>
        <c:varyColors val="0"/>
        {series}
        <c:axId val="1"/><c:axId val="2"/>
      </c:areaChart>"#
            ),
            chart_extra,
        )
    }

    fn area_fill(hex: &str) -> String {
        format!(r#"<c:spPr><a:solidFill><a:srgbClr val="{hex}"/></a:solidFill></c:spPr>"#)
    }

    fn render_area(xml: &str) -> Result<Vec<u8>, Error> {
        let doc = crate::xml::parse(xml.as_bytes()).unwrap();
        render_rgb_chart(
            &doc.root,
            576,
            336,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
    }

    fn pixel_at(rgb: &[u8], w: u32, x: u32, y: u32) -> [u8; 3] {
        let i = ((y * w + x) * 3) as usize;
        [rgb[i], rgb[i + 1], rgb[i + 2]]
    }

    #[test]
    fn standard_area_fills_to_zero_covers_later_series_and_breaks_on_gaps() {
        let s1 = line_ser(
            0,
            "S1",
            &[Some(2.0), Some(2.0), Some(2.0), Some(2.0)],
            &area_fill("156082"),
        );
        let s2 = line_ser(
            1,
            "S2",
            &[Some(1.0), Some(1.0), Some(1.0), Some(1.0)],
            &area_fill("E97132"),
        );
        let png = render_area(&standard_area_xml(&format!("{s1}{s2}"), "")).unwrap();
        let (w, h, rgb) = png_rgb(&png);
        assert_eq!((w, h), (576, 336));
        let a1 = [0x15, 0x60, 0x82];
        let a2 = [0xE9, 0x71, 0x32];
        assert!(
            count_in(&rgb, w, 80, w - 80, 40, h - 40, a1, 2) >= 80,
            "the lower series should remain visible above the covering series"
        );
        assert!(
            count_in(&rgb, w, 80, w - 80, 40, h - 40, a2, 2) >= 80,
            "the later series should cover the first near the baseline"
        );

        // Gap in the middle must not be filled across.
        let gapped = line_ser(
            0,
            "S1",
            &[Some(2.0), Some(2.0), None, Some(2.0), Some(2.0)],
            &area_fill("156082"),
        );
        let png = render_area(&standard_area_xml(
            &gapped,
            r#"<c:dispBlanksAs val="gap"/>"#,
        ))
        .unwrap();
        let (w, h, rgb) = png_rgb(&png);
        assert!(count_in(&rgb, w, 80, w - 80, 40, h - 40, a1, 2) >= 40);
        // Mid-category of the missing point should stay near the plot background.
        let cx = w / 2;
        let mut mid_hits = 0usize;
        for y in 40..h - 50 {
            if near(pixel_at(&rgb, w, cx, y), a1, 8) {
                mid_hits += 1;
            }
        }
        assert!(
            mid_hits < 8,
            "a gap must not leave a filled band down the missing category, got {mid_hits}"
        );
    }

    fn count_fill_band(rgb: &[u8], w: u32, x: u32, y0: u32, y1: u32, color: [u8; 3]) -> usize {
        (y0..y1)
            .filter(|&y| near(pixel_at(rgb, w, x, y), color, 8))
            .count()
    }

    #[test]
    fn standard_area_crosses_zero_and_slide_host_paints_fill() {
        let ser = line_ser(
            0,
            "S1",
            &[Some(2.0), Some(-2.0), Some(2.0)],
            &area_fill("156082"),
        );
        let png = render_area(&standard_area_xml(&ser, "")).unwrap();
        let (w, h, rgb) = png_rgb(&png);
        let a1 = [0x15, 0x60, 0x82];
        // With +/- values the auto axis includes zero near mid-height.
        // First category (near w/4) is positive: fill sits in the upper half.
        // Second category (near w/2) is negative: fill sits in the lower half.
        let left = w / 4;
        let mid = w / 2;
        let pos_above = count_fill_band(&rgb, w, left, 40, h / 2, a1);
        let pos_below = count_fill_band(&rgb, w, left, h / 2, h - 40, a1);
        let neg_above = count_fill_band(&rgb, w, mid, 40, h / 2, a1);
        let neg_below = count_fill_band(&rgb, w, mid, h / 2, h - 40, a1);
        assert!(
            pos_above > 20 && pos_above > pos_below * 3,
            "positive values should fill above the zero crossing; above={pos_above} below={pos_below}"
        );
        assert!(
            neg_below > 20 && neg_below > neg_above * 3,
            "negative values should fill below zero, not above; below={neg_below} above={neg_above}"
        );

        let doc = crate::xml::parse(standard_area_xml(&ser, "").as_bytes()).unwrap();
        let slide = render_rgb_chart(
            &doc.root,
            576,
            336,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Slide,
            None,
            "",
        )
        .unwrap();
        let (sw, _, srgb) = png_rgb(&slide);
        assert_eq!(sw, 576);
        assert!(
            count_in(&srgb, sw, 80, sw - 80, 40, 300, a1, 2) >= 80,
            "slide host shares the area painter"
        );
    }

    #[test]
    fn unsupported_area_inputs_stay_not_implemented() {
        let ser = line_ser(
            0,
            "S1",
            &[Some(1.0), Some(2.0), Some(3.0)],
            &area_fill("156082"),
        );
        let base = standard_area_xml(&ser, "");
        let cases = [
            base.replace(r#"grouping val="standard""#, r#"grouping val="stacked""#),
            base.replace(
                r#"grouping val="standard""#,
                r#"grouping val="percentStacked""#,
            ),
            base.replace("<c:areaChart>", "<c:area3DChart>")
                .replace("</c:areaChart>", "</c:area3DChart>"),
            base.replace(
                r#"crossBetween val="between""#,
                r#"crossBetween val="midCat""#,
            ),
            standard_area_xml(&ser, r#"<c:dispBlanksAs val="zero"/>"#),
            standard_area_xml(&ser, r#"<c:dispBlanksAs val="span"/>"#),
            base.replacen(r#"varyColors val="0""#, r#"varyColors val="1""#, 1),
            base.replacen(
                r#"</c:areaChart>"#,
                r#"<c:dLbls><c:showVal val="1"/></c:dLbls></c:areaChart>"#,
                1,
            ),
            base.replacen(r#"</c:areaChart>"#, r#"<c:dropLines/></c:areaChart>"#, 1),
            standard_area_xml(
                &line_ser(
                    0,
                    "S1",
                    &[Some(1.0), Some(2.0)],
                    r#"<c:spPr><a:solidFill><a:srgbClr val="156082"><a:alpha val="50000"/></a:srgbClr></a:solidFill></c:spPr>"#,
                ),
                "",
            ),
            standard_area_xml(
                &line_ser(
                    0,
                    "S1",
                    &[Some(1.0), Some(2.0)],
                    r#"<c:marker><c:symbol val="circle"/><c:size val="5"/></c:marker>"#,
                ),
                "",
            ),
            area_space(
                r#"<c:barChart><c:barDir val="col"/><c:grouping val="clustered"/><c:ser>
          <c:order val="0"/><c:val><c:numLit><c:pt idx="0"><c:v>1</c:v></c:pt></c:numLit></c:val>
        </c:ser></c:barChart><c:areaChart><c:grouping val="standard"/><c:ser>
          <c:order val="1"/><c:val><c:numLit><c:pt idx="0"><c:v>2</c:v></c:pt></c:numLit></c:val>
        </c:ser></c:areaChart>"#,
                "",
            ),
        ];
        for (i, xml) in cases.iter().enumerate() {
            let err =
                render_area(xml).expect_err(&format!("area case {i} should stay NotImplemented"));
            assert_eq!(
                err.code(),
                crate::error::ErrorCode::NotImplemented,
                "area case {i} should stay NotImplemented"
            );
        }
        // Omitted grouping, gap blanks, hidden labels, and marker none stay supported.
        render_area(
            &area_space(
                &format!(
                    r#"<c:areaChart>
        <c:varyColors val="0"/>
        {ser}
        <c:dLbls><c:showVal val="0"/><c:showCatName val="0"/><c:showSerName val="0"/><c:showPercent val="0"/><c:showLegendKey val="0"/><c:showBubbleSize val="0"/></c:dLbls>
        <c:axId val="1"/><c:axId val="2"/>
      </c:areaChart>"#
                ),
                r#"<c:dispBlanksAs val="gap"/>"#,
            )
            .replacen(
                &area_fill("156082"),
                &format!(
                    r#"{}<c:marker><c:symbol val="none"/></c:marker>"#,
                    area_fill("156082")
                ),
                1,
            ),
        )
        .unwrap();
        // A supported line is not broken by the new arm.
        render_line(&standard_line_xml(
            &line_ser(0, "S1", &[Some(1.0), Some(2.0), Some(3.0)], ""),
            "",
        ))
        .unwrap();
    }

    fn pie_space(plot_body: &str, chart_extra: &str) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <c:chart>
    {chart_extra}
    <c:plotArea>
      {plot_body}
    </c:plotArea>
    <c:legend><c:legendPos val="r"/></c:legend>
  </c:chart>
</c:chartSpace>"#
        )
    }

    fn standard_pie_xml(series: &str, chart_extra: &str, extra: &str) -> String {
        pie_space(
            &format!(
                r#"<c:pieChart>
        <c:varyColors val="1"/>
        {series}
        {extra}
      </c:pieChart>"#
            ),
            chart_extra,
        )
    }

    fn render_pie(xml: &str) -> Result<Vec<u8>, Error> {
        let doc = crate::xml::parse(xml.as_bytes()).unwrap();
        render_rgb_chart(
            &doc.root,
            576,
            336,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
    }

    fn color_centroid(
        rgb: &[u8],
        w: u32,
        h: u32,
        target: [u8; 3],
        tol: i16,
    ) -> Option<(f64, f64, usize)> {
        let mut sx = 0.0;
        let mut sy = 0.0;
        let mut n = 0usize;
        for y in 0..h {
            for x in 0..w {
                let i = ((y * w + x) * 3) as usize;
                if near([rgb[i], rgb[i + 1], rgb[i + 2]], target, tol) {
                    sx += f64::from(x) + 0.5;
                    sy += f64::from(y) + 0.5;
                    n += 1;
                }
            }
        }
        (n > 0).then_some((sx / n as f64, sy / n as f64, n))
    }

    #[test]
    fn standard_pie_assigns_point_colors_clockwise_from_up() {
        let ser = line_ser(0, "S1", &[Some(1.0), Some(1.0), Some(1.0), Some(1.0)], "");
        let png = render_pie(&standard_pie_xml(&ser, "", "")).unwrap();
        let (w, h, rgb) = png_rgb(&png);
        assert_eq!((w, h), (576, 336));
        let a1 = FALLBACK_ACCENTS[0];
        let a2 = FALLBACK_ACCENTS[1];
        let a3 = FALLBACK_ACCENTS[2];
        let a4 = FALLBACK_ACCENTS[3];
        let c1 = color_centroid(&rgb, w, h, a1, 2).expect("first slice color");
        let c2 = color_centroid(&rgb, w, h, a2, 2).expect("second slice color");
        let c3 = color_centroid(&rgb, w, h, a3, 2).expect("third slice color");
        let c4 = color_centroid(&rgb, w, h, a4, 2).expect("fourth slice color");
        let cx = (c1.0 + c2.0 + c3.0 + c4.0) / 4.0;
        let cy = (c1.1 + c2.1 + c3.1 + c4.1) / 4.0;
        // Clockwise from 12 o'clock: accent1 upper-right, 2 lower-right, 3 lower-left, 4 upper-left.
        assert!(
            c1.0 > cx && c1.1 < cy,
            "first slice should sit in the upper-right; c1={c1:?} center=({cx},{cy})"
        );
        assert!(
            c2.0 > cx && c2.1 > cy,
            "second slice should sit in the lower-right; c2={c2:?}"
        );
        assert!(
            c3.0 < cx && c3.1 > cy,
            "third slice should sit in the lower-left; c3={c3:?}"
        );
        assert!(
            c4.0 < cx && c4.1 < cy,
            "fourth slice should sit in the upper-left; c4={c4:?}"
        );
        for (_, _, n) in [c1, c2, c3, c4] {
            assert!(
                n >= 80,
                "each equal slice should cover a real area, got {n}"
            );
        }
    }

    #[test]
    fn pie_first_slice_angle_rotates_and_dpt_overrides_color() {
        let ser = line_ser(0, "S1", &[Some(1.0), Some(1.0), Some(1.0), Some(1.0)], "");
        let png = render_pie(&standard_pie_xml(
            &ser,
            "",
            r#"<c:firstSliceAng val="90"/>"#,
        ))
        .unwrap();
        let (w, h, rgb) = png_rgb(&png);
        let a1 = FALLBACK_ACCENTS[0];
        let a2 = FALLBACK_ACCENTS[1];
        let a3 = FALLBACK_ACCENTS[2];
        let a4 = FALLBACK_ACCENTS[3];
        let c1 = color_centroid(&rgb, w, h, a1, 2).unwrap();
        let c2 = color_centroid(&rgb, w, h, a2, 2).unwrap();
        let c3 = color_centroid(&rgb, w, h, a3, 2).unwrap();
        let c4 = color_centroid(&rgb, w, h, a4, 2).unwrap();
        let cx = (c1.0 + c2.0 + c3.0 + c4.0) / 4.0;
        let cy = (c1.1 + c2.1 + c3.1 + c4.1) / 4.0;
        // Starting at 3 o'clock, first slice fills the lower-right quadrant.
        assert!(
            c1.0 > cx && c1.1 > cy,
            "firstSliceAng=90 should move the first slice into the lower-right; c1={c1:?} center=({cx},{cy})"
        );
        assert!(
            c2.0 < cx && c2.1 > cy,
            "second slice should sit in the lower-left; c2={c2:?}"
        );

        let override_ser = line_ser(
            0,
            "S1",
            &[Some(1.0), Some(1.0)],
            r#"<c:dPt><c:idx val="0"/><c:spPr><a:solidFill><a:srgbClr val="CC0000"/></a:solidFill></c:spPr></c:dPt>"#,
        );
        let png = render_pie(&standard_pie_xml(&override_ser, "", "")).unwrap();
        let (w, h, rgb) = png_rgb(&png);
        let red = [0xCC, 0x00, 0x00];
        let hits = color_centroid(&rgb, w, h, red, 2).expect("dPt solid fill");
        assert!(
            hits.2 >= 80,
            "dPt should paint the first slice, got {}",
            hits.2
        );
        // The second point still uses accent2, not accent1.
        let a2 = FALLBACK_ACCENTS[1];
        assert!(color_centroid(&rgb, w, h, a2, 2).is_some());
    }

    #[test]
    fn pie_skips_blanks_keeps_zero_in_legend_and_slide_host_paints() {
        let gapped = line_ser(0, "S1", &[Some(1.0), None, Some(1.0)], "");
        let png = render_pie(&standard_pie_xml(
            &gapped,
            r#"<c:dispBlanksAs val="gap"/>"#,
            "",
        ))
        .unwrap();
        let (w, h, rgb) = png_rgb(&png);
        let a1 = FALLBACK_ACCENTS[0];
        let a2 = FALLBACK_ACCENTS[1];
        let a3 = FALLBACK_ACCENTS[2];
        let c1 = color_centroid(&rgb, w, h, a1, 2).unwrap();
        let c3 = color_centroid(&rgb, w, h, a3, 2).unwrap();
        // Blank point is skipped: no accent2 wedge, first and third share the pie (right vs left).
        assert!(
            color_centroid(&rgb, w, h, a2, 2).is_none_or(|(_, _, n)| n < 8),
            "a blank should not leave a filled slice in the missing category color"
        );
        assert!(c1.2 >= 80 && c3.2 >= 80);
        assert!(
            (c1.0 - c3.0).abs() > 20.0,
            "the two remaining slices should sit on opposite sides"
        );

        let with_zero = line_ser(0, "S1", &[Some(1.0), Some(0.0), Some(1.0)], "");
        let png = render_pie(&standard_pie_xml(&with_zero, "", "")).unwrap();
        let (w, h, rgb) = png_rgb(&png);
        // Zero has no wedge, but the category remains in the legend swatch.
        let zero_hits = color_centroid(&rgb, w, h, a2, 2).map(|c| c.2).unwrap_or(0);
        assert!(
            zero_hits > 0 && zero_hits < 80,
            "a zero should appear in the legend, not as a slice; hits={zero_hits}"
        );

        let doc = crate::xml::parse(standard_pie_xml(&gapped, "", "").as_bytes()).unwrap();
        let slide = render_rgb_chart(
            &doc.root,
            576,
            336,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Slide,
            None,
            "",
        )
        .unwrap();
        let (sw, _, srgb) = png_rgb(&slide);
        assert_eq!(sw, 576);
        assert!(color_centroid(&srgb, sw, 336, a1, 2).is_some());
    }

    #[test]
    fn unsupported_pie_inputs_stay_not_implemented() {
        let ser = line_ser(0, "S1", &[Some(1.0), Some(2.0), Some(3.0)], "");
        let base = standard_pie_xml(&ser, "", "");
        let two = format!(
            "{}{}",
            ser,
            line_ser(1, "S2", &[Some(1.0), Some(2.0), Some(3.0)], "")
        );
        let cases = [
            standard_pie_xml(
                &line_ser(0, "S1", &[Some(1.0), Some(-2.0), Some(3.0)], ""),
                "",
                "",
            ),
            pie_space(
                &format!(r#"<c:pieChart><c:varyColors val="1"/>{two}</c:pieChart>"#),
                "",
            ),
            base.replace("<c:pieChart>", "<c:pie3DChart>")
                .replace("</c:pieChart>", "</c:pie3DChart>"),
            base.replace("<c:pieChart>", "<c:doughnutChart>")
                .replace("</c:pieChart>", "</c:doughnutChart>"),
            pie_space(
                r#"<c:ofPieChart><c:ofPieType val="pie"/><c:varyColors val="1"/><c:ser>
          <c:order val="0"/><c:val><c:numLit><c:pt idx="0"><c:v>1</c:v></c:pt></c:numLit></c:val>
        </c:ser></c:ofPieChart>"#,
                "",
            ),
            standard_pie_xml(&ser, "", r#"<c:firstSliceAng val="361"/>"#),
            standard_pie_xml(
                &line_ser(
                    0,
                    "S1",
                    &[Some(1.0), Some(2.0)],
                    r#"<c:explosion val="10"/>"#,
                ),
                "",
                "",
            ),
            standard_pie_xml(
                &line_ser(
                    0,
                    "S1",
                    &[Some(1.0), Some(2.0)],
                    r#"<c:dPt><c:idx val="0"/><c:explosion val="8"/></c:dPt>"#,
                ),
                "",
                "",
            ),
            standard_pie_xml(
                &line_ser(
                    0,
                    "S1",
                    &[Some(1.0), Some(2.0)],
                    r#"<c:spPr><a:solidFill><a:srgbClr val="156082"><a:alpha val="50000"/></a:srgbClr></a:solidFill></c:spPr>"#,
                ),
                "",
                "",
            ),
            standard_pie_xml(&ser, r#"<c:dispBlanksAs val="zero"/>"#, ""),
            standard_pie_xml(&ser, r#"<c:dispBlanksAs val="span"/>"#, ""),
            base.replacen(
                r#"</c:pieChart>"#,
                r#"<c:dLbls><c:showVal val="1"/></c:dLbls></c:pieChart>"#,
                1,
            ),
            pie_space(
                &format!(
                    r#"<c:pieChart><c:varyColors val="1"/>{ser}</c:pieChart>
       <c:valAx><c:majorGridlines/><c:crossBetween val="between"/></c:valAx>"#
                ),
                "",
            ),
            standard_pie_xml(&ser, "", "")
                .replace(r#"<c:legendPos val="r"/>"#, r#"<c:legendPos val="l"/>"#),
        ];
        for (i, xml) in cases.iter().enumerate() {
            let err =
                render_pie(xml).expect_err(&format!("pie case {i} should stay NotImplemented"));
            assert_eq!(
                err.code(),
                crate::error::ErrorCode::NotImplemented,
                "pie case {i} should stay NotImplemented"
            );
        }
        // Omitted firstSliceAng and hidden labels stay supported; a column chart is unharmed.
        render_pie(
            &standard_pie_xml(&ser, r#"<c:dispBlanksAs val="gap"/>"#, "").replacen(
                r#"</c:pieChart>"#,
                r#"<c:dLbls><c:showVal val="0"/><c:showCatName val="0"/><c:showSerName val="0"/><c:showPercent val="0"/><c:showLegendKey val="0"/><c:showBubbleSize val="0"/></c:dLbls></c:pieChart>"#,
                1,
            ),
        )
        .unwrap();
        render_line(&standard_line_xml(
            &line_ser(0, "S1", &[Some(1.0), Some(2.0), Some(3.0)], ""),
            "",
        ))
        .unwrap();
    }
}
