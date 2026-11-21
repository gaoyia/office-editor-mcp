//! 簇状柱形图、簇状条形图、普通折线图、带标记的散点图、普通面积图和普通饼图只读栅格化。优先读图部件里已有的缓存。没有缓存时，读公式所指格子里已有的值。不写回包，也不算公式。
//! 像素差要改这里的布局、颜色和字形，不要靠放宽对照容差。
//! 折线本切片只画 `c:lineChart` 且 grouping 为 standard 或省略的图：实线、缺省/auto 按系列循环标记（diamond、square、triangle、x、star、circle、plus、dot、dash，9 个一圈）、显式符号、缺格子或省略的 cache 点按缺口断线、空 inlineStr 与空 cache/lit <c:v></c:v> 按 0 连线。省略 `c:marker` 时尺寸为 9 磅；写出的 `c:size` 优先，写出标记但省略尺寸时按 OOXML 缺省 5 磅。平滑线、预设虚线、复杂自定义虚线、三维折线、组合图及其余类型仍是 NotImplemented，不用缺一块的图充数。已锁定的 lineChart grouping=stacked/percentStacked（线性显式 min/max autoZero、marker=none、crossBetween=between）按系列顺序把各类目的代数累加端点连成折线，见 F23；省略 min/max、可见标记、空点、percentStacked 异号或缺值仍是 NotImplemented。
//! 散点本切片只画 `c:scatterChart` 且 scatterStyle 为 marker 或 lineMarker（缺省按 lineMarker）的图：X/Y 都是数值轴，点按系列顺序用数值坐标定位；autoZero 在 0 落在轴范围内时把坐标轴画在原点并在轴旁标刻度，0 在轴外时轴仍在较近端；marker 与 lineMarker 都画实线（官方 Excel 把 scatterStyle=marker 且系列未隐藏线条的图报为 xlXYScatterLines=74），缺格子或省略的 cache 点断开，空 inlineStr 与空 cache/lit <c:v></c:v> 按 0 连线；系列 `a:ln`/`a:noFill` 仍可关掉连线只留标记；标记规则与折线相同。官方 Excel 16 公开 API 的散点 valAx 写 crossBetween=midCat（不是类目槽公式，数值仍按线性 min/max 定位）；xlXYScatterLinesNoMarkers 写 scatterStyle=lineMarker 且 marker symbol=none，不是 scatterStyle=line；xlXYScatter 写 lineMarker 加系列 a:ln/a:noFill。这些已锁定路径在 midCat 下同样出图。scatterStyle=none/line/smooth，以及 scatterStyle=smoothMarker 或系列 smooth=1 的平滑曲线，仍是 NotImplemented。
//! 条形本切片只画 `c:barChart` 且 `barDir` 为 `bar` 或省略、grouping 为 clustered 或省略的簇状条形图：类目轴在左；类目 maxMin 时数值轴在上、minMax（或缺省）时数值轴在下；类目 minMax（或缺省）时第一类在下，maxMin 时第一类在上；系列 0 在类目轴方向的槽起点（maxMin 时槽顶，minMax 时槽底）；正值向右、负值向左，缺值不画该条。已锁定的 barChart grouping=stacked/percentStacked（overlap=100，线性显式 min/max autoZero）见 F22；三维、可见数据标签、空值按零或连线、类目轴网格、非 l/b 轴位及其余类型仍是 NotImplemented。
//! 面积本切片只画 `c:areaChart` 且 grouping 为 standard 或省略的图：按与折线相同的类目槽公式定位，用不透明纯色填到数值轴交叉（autoZero 为零，零在轴外时取较近端）；正值在交叉之上、负值在下，相邻异号在交叉处剖开以免自交；系列按 order 先画后盖；缺格子或省略的 cache 点：未写 dispBlanksAs 时按官方画到零基线（连续 V 形谷），显式 dispBlanksAs=gap 时仍按缺口断开、不跨空点相连；空 inlineStr 与空 cache/lit <c:v></c:v> 按 0 连接填充。有写出的实线描边才画上沿线，不默认画标记。已锁定的 areaChart grouping=stacked/percentStacked（线性显式 min/max autoZero、valAx crossBetween=midCat）按系列顺序代数累加，并填充相邻累加线之间的带（后画的系列盖住先画的），见 F23；省略 min/max、空点、between 类目位、percentStacked 异号或缺值、三维、可见数据标签、显式 dispBlanksAs=zero 与显式 dispBlanksAs=span 对真实空点/省略 cache 点按与省略项相同的零基线 V 形填充（官方面积 span 不是 4-到-9 弦）；span 下的 #N/A 亦按该 V 形填充，其余错误点仍 NI；半透明填充、组合图、标记及其余类型仍是 NotImplemented，不用缺一块的图充数。
//! 饼图本切片只画单系列 `c:pieChart`：扇区从 12 点方向按 firstSliceAng（缺省 0，顺时针，单位度）起算，只把有限正值按总和比例切开；空值跳过该扇区但仍出现在图例并占用 accent 槽，零值不占角度但仍可出现在图例；负值、多系列、三维、环形、复合饼、爆炸、可见数据标签、半透明或非纯色扇区、非右侧缺省图例及其余类型仍是 NotImplemented，不用缺一块的图充数。

use fontdue::{Font, FontSettings};

#[path = "chart_axis.rs"]
mod axis;
#[path = "chart_legend.rs"]
mod legend;
#[path = "chart_gradient.rs"]
mod gradient;
#[path = "chart_stroke.rs"]
mod stroke;

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
/// 绘图区顶/底边距不用各自四舍五入：106 dpi 下 14→15.458、34→37.542，
/// 四舍五入得到 15+38，官方 Chart.Export 的网格顶/底却是 16 与 37（总边距同为 53）。
/// 起点边距向上取整、对边向下取整，96/144 等整倍数不变。
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

/// Official Excel 16 grouping for barChart clustered / stacked / percentStacked,
/// and for lineChart / areaChart stacked / percentStacked (F23). `Clustered` also
/// means a standard (non-stacked) line or area.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stacking {
    Clustered,
    Stacked,
    PercentStacked,
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

/// Excel Chart.Export keeps a 96px gutter's leading edge by ceiling the
/// scaled length, and the trailing edge by flooring it. Independently
/// rounding both sides at 106dpi yields 15 and 38, but official untitled
/// 636x371 / 424x265 plots start at y=16 and end at 334 / 228 (insets 16
/// and 37). The total chrome is the same 53px. Exact at 96dpi.
fn scale_plot_leading(dpi: RenderDpi, px96: i32) -> i32 {
    if dpi.is_default() {
        px96
    } else {
        (f64::from(px96) * dpi.get() / RenderDpi::DEFAULT.get()).ceil() as i32
    }
}

fn scale_plot_trailing(dpi: RenderDpi, px96: i32) -> i32 {
    if dpi.is_default() {
        px96
    } else {
        (f64::from(px96) * dpi.get() / RenderDpi::DEFAULT.get()).floor() as i32
    }
}

/// Regular major-grid rows: truncate from the official plot top,
/// `y = plot_y + i * span / intervals`. The old `+ intervals - 1` bias
/// pushed several 106dpi ticks one pixel past unstretched official grids.
fn regular_grid_y(plot_y: i32, plot_b: i32, i: i32, intervals: i32) -> i32 {
    plot_y + i * (plot_b - plot_y) / intervals
}

// Keep an on-major zero crossing on the very same raster row as its grid.
// Rounding y_of(0) separately puts mixed-sign axes one row below the 0 grid
// whenever the exact position is a half pixel (e.g. [-3,5], 106dpi).
fn crossing_axis_y(plot: &Rect, min: f64, max: f64, major: f64, crossing: f64) -> i32 {
    crossing_axis_y_mode(plot, min, max, major, crossing, None, false)
}

fn crossing_axis_y_mode(
    plot: &Rect,
    min: f64,
    max: f64,
    major: f64,
    crossing: f64,
    log_base: Option<f64>,
    reversed: bool,
) -> i32 {
    if log_base.is_some() {
        return y_map(crossing, min, max, plot, log_base, reversed).round() as i32;
    }
    let intervals = (max - min) / major;
    let index = if reversed {
        (crossing - min) / major
    } else {
        (max - crossing) / major
    };
    if major > 0.0
        && (intervals - intervals.round()).abs() < 1e-8
        && (index - index.round()).abs() < 1e-8
        && intervals >= 1.0
    {
        regular_grid_y(
            plot.y as i32,
            (plot.y + plot.h) as i32,
            index.round() as i32,
            intervals.round() as i32,
        )
    } else {
        y_map(crossing, min, max, plot, None, reversed).round() as i32
    }
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
    dash: Option<std::sync::Arc<stroke::DashPattern>>,
    cap: stroke::Cap,
    join: stroke::Join,
    color: Option<[u8; 3]>,
    visible: bool,
    width: f64,
    marker: MarkerKind,
    marker_color: Option<[u8; 3]>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ValueLabel {
    OutsideEnd,
    InsideEnd,
    Right,
}

struct Series {
    name: String,
    values: Vec<Option<f64>>,
    color: Option<[u8; 3]>,
    slot: usize,
    line_visible: bool,
    line_width: f64,
    line_cap: stroke::Cap,
    line_join: stroke::Join,
    line_dash: Option<std::sync::Arc<stroke::DashPattern>>,
    marker: MarkerKind,
    marker_color: Option<[u8; 3]>,
    /// Area stroke color when a solid outline is present. Lines keep this unset.
    line_color: Option<[u8; 3]>,
    x_values: Vec<Option<f64>>,
    /// c:barSer defaults to inversion; dPt can override individual values.
    invert_if_negative: bool,
    point_inversion: Vec<Option<bool>>,
    negative_outline: Option<LineSeriesStyle>,
    value_label: Option<ValueLabel>,
    deleted_label_points: Vec<usize>,
    /// Distinguish explicit errors from omitted points for dispBlanksAs.
    error_points: Vec<bool>,
    unavailable_points: Vec<bool>,
    span_blanks: bool,
    /// Official F21 secondary valAx: this series is in the second same-type
    /// wrapper and is mapped with the secondary scale.
    on_secondary: bool,
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
            line_cap: stroke::Cap::Round,
            line_join: stroke::Join::Round,
            line_dash: None,
            marker: MarkerKind::None,
            marker_color: None,
            line_color: None,
            x_values: Vec::new(),
            invert_if_negative: true,
            point_inversion: Vec::new(),
            negative_outline: None,
            value_label: None,
            deleted_label_points: Vec::new(),
            error_points: Vec::new(),
            unavailable_points: Vec::new(),
            span_blanks: false,
            on_secondary: false,
        }
    }
}

struct ColumnChart {
    categories: Vec<String>,
    series: Vec<Series>,
    gap_width: f64,
    overlap: f64,
    stacking: Stacking,
    vary_points: bool,
    grid: bool,
    legend: Option<LegendPos>,
    legend_spec: legend::Spec,
    axis: axis::Spec,
    axis_x: Option<axis::Spec>,
    /// Official F21 secondary valAx (axPos=r, crosses=max). None for one axis.
    axis_secondary: Option<axis::Spec>,
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
    /// Official series-level pie fill. None when the series has no explicit fill.
    /// When present, it applies to every slice even if varyColors=1, and a dPt
    /// fill override still wins for that point.
    series_fill: Option<SliceFill>,
    /// Pie per-point fill overrides, including noFill (alpha 0) and straight alpha.
    point_fills: Vec<Option<SliceFill>>,
    /// Series-level pie explosion percent. Zero when omitted or explicitly 0.
    pie_explosion: f64,
    /// Per-point pie explosion percent (dPt). Empty or all-zero when none.
    point_explosions: Vec<f64>,
    /// Official Excel 16.0 area charts with omitted c:dispBlanksAs plot
    /// missing cells / omitted cache points as 0 (a V to the baseline).
    /// Explicit dispBlanksAs=gap still leaves a hole. Unused for other kinds.
    area_missing_as_zero: bool,
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
    rounded_frame: bool,
    chart_border: bool,
    custom_dash_budget: usize,
    plot_fill: Option<PlotPaint>,
    dpi: RenderDpi,
}

/// 图表 PNG 的像素。`twoCell` 按格子累加，`oneCell`/`absolute` 用锚点上的 `xdr:ext`。
/// Excel 16.0 保存后 `graphicFrame/xfrm/ext` 常被写成 0×0，不能拿来当尺寸。缩放来自工作表。
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
    // oneCell/absolute 的合同尺寸在锚点 `xdr:ext`。内层 graphicFrame xfrm 只是占位，
    // Office 保存后经常是 0×0；若优先读它，fit_px 会把整张已会画的图误判为 NI。
    let frame_ext = anchor
        .child_by_local("graphicFrame")
        .and_then(|frame| frame.child_by_local("xfrm"))
        .and_then(|xfrm| xfrm.child_by_local("ext"));
    let ext = first_positive_ext([anchor.child_by_local("ext"), frame_ext])
        .ok_or_else(|| Error::with_path(ErrorKind::ChartImageUnsupported, "id"))?;
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

fn first_positive_ext(cands: [Option<&Element>; 2]) -> Option<&Element> {
    cands.into_iter().flatten().find(|ext| {
        matches!(
            (
                ext.attr_local("cx").and_then(|s| s.parse::<i64>().ok()),
                ext.attr_local("cy").and_then(|s| s.parse::<i64>().ok()),
            ),
            (Some(cx), Some(cy)) if cx > 0 && cy > 0
        )
    })
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

#[derive(Clone, Copy)]
struct ThemeScheme {
    accents: [[u8; 3]; 6],
    dk1: Option<[u8; 3]>,
    lt1: Option<[u8; 3]>,
    dk2: Option<[u8; 3]>,
    lt2: Option<[u8; 3]>,
    hlink: Option<[u8; 3]>,
    fol_hlink: Option<[u8; 3]>,
}

fn theme_slot_rgb(slot: &Element, accents: &[[u8; 3]; 6]) -> Option<[u8; 3]> {
    if let Some(sys) = slot.child_by_local("sysClr") {
        // Theme clrScheme records the slot in lastClr. Chart-frame sysClr
        // fills are a different path: Excel paints the live system color
        // and ignores lastClr, so those fills stay NI.
        return parse_hex(sys.attr_local("lastClr")?);
    }
    color_of_slot(slot, accents)
}

fn theme_scheme(theme_xml: Option<&[u8]>, accents: [[u8; 3]; 6]) -> ThemeScheme {
    let mut scheme = ThemeScheme {
        accents,
        dk1: None,
        lt1: None,
        dk2: None,
        lt2: None,
        hlink: None,
        fol_hlink: None,
    };
    let Some(bytes) = theme_xml else {
        return scheme;
    };
    let Ok(doc) = crate::xml::parse(bytes) else {
        return scheme;
    };
    let Some(slots) = find_deep(&doc.root, "clrScheme") else {
        return scheme;
    };
    let color = |name: &str| {
        slots
            .child_by_local(name)
            .and_then(|slot| theme_slot_rgb(slot, &accents))
    };
    scheme.dk1 = color("dk1");
    scheme.lt1 = color("lt1");
    scheme.dk2 = color("dk2");
    scheme.lt2 = color("lt2");
    scheme.hlink = color("hlink");
    scheme.fol_hlink = color("folHlink");
    scheme
}

fn scheme_slot(name: &str, scheme: &ThemeScheme) -> Option<[u8; 3]> {
    Some(match name {
        "accent1" => scheme.accents[0],
        "accent2" => scheme.accents[1],
        "accent3" => scheme.accents[2],
        "accent4" => scheme.accents[3],
        "accent5" => scheme.accents[4],
        "accent6" => scheme.accents[5],
        "dk1" | "tx1" => scheme.dk1?,
        "lt1" | "bg1" => scheme.lt1?,
        "dk2" | "tx2" => scheme.dk2?,
        "lt2" | "bg2" => scheme.lt2?,
        "hlink" => scheme.hlink?,
        "folHlink" => scheme.fol_hlink?,
        _ => return None,
    })
}

/// IEC 61966-2-1 sRGB. Excel chart-frame tint/shade interpolate in this space,
/// not in gamma-encoded bytes and not with a pure 2.2 gamma.
fn srgb_to_linear(channel: u8) -> f64 {
    let s = f64::from(channel) / 255.0;
    if s <= 0.04045 {
        s / 12.92
    } else {
        ((s + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb(value: f64) -> u8 {
    let value = value.clamp(0.0, 1.0);
    let s = if value <= 0.0031308 {
        12.92 * value
    } else {
        1.055 * value.powf(1.0 / 2.4) - 0.055
    };
    (s * 255.0).round().clamp(0.0, 255.0) as u8
}

fn frame_tint(rgb: [u8; 3], val: u32) -> [u8; 3] {
    let share = f64::from(val) / 100_000.0;
    rgb.map(|channel| {
        let lin = srgb_to_linear(channel);
        linear_to_srgb(lin * share + (1.0 - share))
    })
}

fn frame_shade(rgb: [u8; 3], val: u32) -> [u8; 3] {
    let share = f64::from(val) / 100_000.0;
    rgb.map(|channel| linear_to_srgb(srgb_to_linear(channel) * share))
}

fn frame_lum(rgb: [u8; 3], lum_mod: Option<u32>, lum_off: Option<i32>) -> [u8; 3] {
    let (hue, sat, light) = rgb_hsl(rgb);
    let mut light = light;
    if let Some(factor) = lum_mod {
        light *= f64::from(factor) / 100_000.0;
    }
    if let Some(offset) = lum_off {
        light += f64::from(offset) / 100_000.0;
    }
    hsl_rgb(hue, sat, light.clamp(0.0, 1.0))
}

/// Baseline signed DrawingML luminance offset, in thousandths of a percent.
/// Alpha/modulation percentages remain unsigned. Unsupported ranges are not
/// silently clamped as input; only the resulting HSL luminance is clamped.
fn parse_luminance_offset(node: &Element) -> Option<i32> {
    if node.attrs.len() != 1 || node.element_children().next().is_some() {
        return None;
    }
    let raw = node.attr_local("val")?;
    let digits = raw.strip_prefix('-').unwrap_or(raw);
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let value: i32 = raw.parse().ok()?;
    (-100_000..=100_000).contains(&value).then_some(value)
}

fn frame_alpha_fill(rgb: [u8; 3], alpha: &Element) -> Result<FrameFill, Error> {
    let unsupported = || Error::with_path(ErrorKind::ChartImageUnsupported, "id");
    let alpha = parse_ooxml_alpha(alpha.attr_local("val")).ok_or_else(unsupported)?;
    if alpha == 100_000 {
        Ok(FrameFill::Solid(rgb))
    } else {
        Ok(FrameFill::Translucent(rgb, alpha))
    }
}

fn resolve_frame_solid(color: &Element, scheme: &ThemeScheme) -> Result<FrameFill, Error> {
    let unsupported = || Error::with_path(ErrorKind::ChartImageUnsupported, "id");
    let rgb = match color.local_name() {
        "srgbClr" => color
            .attr_local("val")
            .and_then(parse_hex)
            .ok_or_else(unsupported)?,
        "schemeClr" => {
            let name = color.attr_local("val").ok_or_else(unsupported)?;
            scheme_slot(name, scheme).ok_or_else(unsupported)?
        }
        _ => return Err(unsupported()),
    };
    let kids: Vec<&Element> = color.element_children().collect();
    let names: Vec<&str> = kids.iter().map(|child| child.local_name()).collect();
    let pct = |node: &Element| parse_ooxml_alpha(node.attr_local("val")).ok_or_else(unsupported);
    match names.as_slice() {
        [] => Ok(FrameFill::Solid(rgb)),
        ["alpha"] => frame_alpha_fill(rgb, kids[0]),
        ["tint"] => Ok(FrameFill::Solid(frame_tint(rgb, pct(kids[0])?))),
        ["tint", "alpha"] => frame_alpha_fill(frame_tint(rgb, pct(kids[0])?), kids[1]),
        ["shade"] => Ok(FrameFill::Solid(frame_shade(rgb, pct(kids[0])?))),
        ["shade", "alpha"] => frame_alpha_fill(frame_shade(rgb, pct(kids[0])?), kids[1]),
        ["lumMod"] => Ok(FrameFill::Solid(frame_lum(rgb, Some(pct(kids[0])?), None))),
        ["lumMod", "alpha"] => frame_alpha_fill(frame_lum(rgb, Some(pct(kids[0])?), None), kids[1]),
        ["lumOff"] => Ok(FrameFill::Solid(frame_lum(
            rgb,
            None,
            Some(parse_luminance_offset(kids[0]).ok_or_else(unsupported)?),
        ))),
        ["lumOff", "alpha"] => frame_alpha_fill(
            frame_lum(rgb, None, Some(parse_luminance_offset(kids[0]).ok_or_else(unsupported)?)),
            kids[1],
        ),
        ["lumMod", "lumOff"] => Ok(FrameFill::Solid(frame_lum(
            rgb,
            Some(pct(kids[0])?),
            Some(parse_luminance_offset(kids[1]).ok_or_else(unsupported)?),
        ))),
        ["lumMod", "lumOff", "alpha"] => frame_alpha_fill(
            frame_lum(rgb, Some(pct(kids[0])?), Some(parse_luminance_offset(kids[1]).ok_or_else(unsupported)?)),
            kids[2],
        ),
        _ => Err(unsupported()),
    }
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
    render_chart_with_theme(
        root, width, height, accents, font_bytes, host, cells, sheet, dpi, None,
    )
}

pub(crate) fn render_chart_with_theme(
    root: &Element,
    width: u32,
    height: u32,
    accents: [[u8; 3]; 6],
    font_bytes: Option<&[u8]>,
    host: ChartHost,
    cells: Option<&super::sparkline::CellCache>,
    sheet: &str,
    dpi: RenderDpi,
    theme_xml: Option<&[u8]>,
) -> Result<Vec<u8>, Error> {
    if width == 0 || height == 0 || width > MAX_PX || height > MAX_PX {
        return Err(Error::with_path(ErrorKind::ChartImageUnsupported, "id"));
    }
    let scheme = theme_scheme(theme_xml, accents);
    let frame = if matches!(host, ChartHost::Excel) {
        Some(read_frame_with(root, &scheme)?)
    } else {
        None
    };
    let chart = read_chart(root, accents, cells, sheet, dpi)?;
    let font = font_bytes.and_then(|bytes| Font::from_bytes(bytes, FontSettings::default()).ok());
    let mut canvas = Canvas::with_dpi(width, height, dpi);
    if let Some(frame) = frame {
        // Chart area and plot area have independent fills. The default Cartesian
        // plot is white, not the explicitly coloured chart-area background.
        canvas.rounded_frame = frame.rounded;
        canvas.chart_border = frame.border;
        match frame.fill {
            FrameFill::NoFill => canvas.clear_to_transparent(),
            FrameFill::Default => canvas.fill_rect(0, 0, width as i32, height as i32, [255; 3]),
            FrameFill::Solid(color) => canvas.fill_rect(0, 0, width as i32, height as i32, color),
            FrameFill::Translucent(color, alpha) => {
                // Translucent chart fill is straight alpha, not flattened onto white.
                canvas.clear_to_transparent();
                let cov = ooxml_alpha_to_u8(alpha);
                if cov > 0 {
                    canvas.fill_rect_cov(0, 0, width as i32, height as i32, color, cov);
                }
            }
            FrameFill::Linear(grad) => {
                canvas.fill_linear_rect(0, 0, width as i32, height as i32, &grad)
            }
        }
        canvas.plot_fill = if chart.kind == PlotKind::Pie {
            // A pie's automatic plot area is transparent. Explicit opaque solid
            // fill uses the same title/legend-aware plot rectangle as the pie,
            // before slices, never the full available host rectangle.
            match frame.plot_fill {
                FrameFill::NoFill | FrameFill::Default => None,
                FrameFill::Solid(color) => Some(PlotPaint::Solid(color, 255)),
                // Translucent and gradient pie plot fills need their own
                // fractional-edge compositing locks, not Cartesian assumptions.
                _ => return Err(Error::with_path(ErrorKind::ChartImageUnsupported, "id")),
            }
        } else {
            match frame.plot_fill {
                FrameFill::NoFill => None,
                FrameFill::Default => Some(PlotPaint::Solid([255; 3], 255)),
                FrameFill::Solid(color) => Some(PlotPaint::Solid(color, 255)),
                FrameFill::Translucent(color, alpha) => {
                    Some(PlotPaint::Solid(color, ooxml_alpha_to_u8(alpha)))
                }
                FrameFill::Linear(grad) => Some(PlotPaint::Linear(grad)),
            }
        };
    }
    paint(&mut canvas, &chart, font.as_ref(), host)?;
    canvas.encode_png()
}

/// Supported frame styles are semantic fills and a round/rectangle switch.
/// Pattern, blip, path/non-axis-aligned/multi-stop gradients, theme/system
/// colors, transforms and custom strokes remain NI.
/// Default (missing spPr) is white; explicit noFill is transparent; a solid
/// srgbClr with alpha other than 100000 is straight translucency, not a
/// flatten onto white.
/// `Linear` uses an opaque sRGB stop table at the existing 0/90 degree geometry.
#[derive(Debug, Clone, PartialEq, Eq)]
enum FrameFill {
    Default,
    Solid([u8; 3]),
    /// Straight sRGB fill with DrawingML alpha in 0..=100000, excluding 100000
    /// (that case is `Solid`).
    Translucent([u8; 3], u32),
    Linear(LinearFrameGrad),
    NoFill,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LinearFrameGrad {
    stops: gradient::StopTable,
    /// DrawingML `a:lin/@ang` in 60000ths of a degree. Only 0 (left to right)
    /// and 5_400_000 (top to bottom in y-down space) are locked.
    ang: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PlotPaint {
    Solid([u8; 3], u8),
    Linear(LinearFrameGrad),
}

impl LinearFrameGrad {
    fn sample(&self, t: f64) -> [u8; 3] {
        let segment = self.stops.segment(t).expect("finite axis-aligned fill coordinate");
        if self.stops.len() == 2 {
            // Preserve the locked two-stop policy exactly. Fresh three/four-stop
            // Excel controls instead use encoded-sRGB piecewise interpolation.
            let share = office_grad_weight(segment.fraction);
            std::array::from_fn(|i| office_gamma_mix(segment.start[i], segment.end[i], share))
        } else {
            segment.encoded_srgb()
        }
    }
}

const LINEAR_ANG_HORIZONTAL: i64 = 0;
const LINEAR_ANG_VERTICAL: i64 = 5_400_000;

struct FrameStyle {
    rounded: bool,
    border: bool,
    fill: FrameFill,
    plot_fill: FrameFill,
}

#[cfg(test)]
fn read_frame(root: &Element) -> Result<FrameStyle, Error> {
    read_frame_with(root, &theme_scheme(None, FALLBACK_ACCENTS))
}

fn read_frame_with(root: &Element, scheme: &ThemeScheme) -> Result<FrameStyle, Error> {
    let plot = root
        .child_by_local("chart")
        .and_then(|c| c.child_by_local("plotArea"));
    let rounded = flag(root, "roundedCorners").unwrap_or(true);
    let border = root
        .child_by_local("spPr")
        .and_then(|p| p.child_by_local("ln"))
        .is_none();
    // Only the rectangular no-line silhouette is locked. Rounded no-line
    // needs fill clipping independent of the old rounded stroke coverage.
    if (!border && rounded)
        || plot
            .and_then(|p| p.child_by_local("spPr"))
            .and_then(|p| p.child_by_local("ln"))
            .is_some()
    {
        return Err(Error::with_path(ErrorKind::ChartImageUnsupported, "id"));
    }
    Ok(FrameStyle {
        rounded,
        border,
        fill: read_frame_fill(root, scheme)?,
        plot_fill: plot
            .map(|node| read_frame_fill(node, scheme))
            .transpose()?
            .unwrap_or(FrameFill::Default),
    })
}

fn read_frame_fill(node: &Element, scheme: &ThemeScheme) -> Result<FrameFill, Error> {
    let unsupported = || Error::with_path(ErrorKind::ChartImageUnsupported, "id");
    let Some(props) = node.child_by_local("spPr") else {
        return Ok(FrameFill::Default);
    };
    let mut fill = FrameFill::Default;
    let mut saw_fill = false;
    let mut saw_hidden_ext = false;
    let mut saw_line = false;
    let mut saw_hidden_line = false;
    for prop in props.element_children() {
        match prop.local_name() {
            "noFill" => {
                if saw_fill {
                    return Err(unsupported());
                }
                saw_fill = true;
                fill = FrameFill::NoFill;
            }
            "gradFill" => {
                if saw_fill {
                    return Err(unsupported());
                }
                saw_fill = true;
                fill = read_linear_frame_grad(prop)?;
            }
            "solidFill" => {
                if saw_fill {
                    return Err(unsupported());
                }
                saw_fill = true;
                let mut colors = prop.element_children();
                let color = colors.next().ok_or_else(unsupported)?;
                if colors.next().is_some() {
                    return Err(unsupported());
                }
                fill = resolve_frame_solid(color, scheme)?;
            }
            "ln" if !saw_line && is_invisible_frame_line(prop) => saw_line = true,
            "effectLst" if prop.element_children().next().is_none() => (),
            // Excel 16 writes a:noFill plus an Office 2010 hiddenFill ext that
            // only remembers the previous fill; it is not a visible fill.
            "extLst" if is_office_hidden_fill_ext(prop) => saw_hidden_ext = true,
            "extLst" if is_office_hidden_line_ext(prop) => saw_hidden_line = true,
            _ => return Err(unsupported()),
        }
    }
    if (saw_hidden_ext && fill != FrameFill::NoFill) || (saw_hidden_line && !saw_line) {
        return Err(unsupported());
    }
    Ok(fill)
}

/// Excel's invisible line retains the old line in a14:hiddenLine. That
/// extension is editing memory, not a visible stroke or effect.
fn is_office_hidden_line_ext(ext_lst: &Element) -> bool {
    let mut exts = ext_lst.element_children();
    let Some(ext) = exts.next() else {
        return false;
    };
    if exts.next().is_some()
        || ext.local_name() != "ext"
        || ext.attr_local("uri") != Some("{91240B29-F687-4F45-9708-019B960494DF}")
    {
        return false;
    }
    let mut kids = ext.element_children();
    matches!(kids.next(), Some(node) if node.local_name() == "hiddenLine") && kids.next().is_none()
}

fn is_invisible_frame_line(line: &Element) -> bool {
    for (name, value) in &line.attrs {
        match name.as_str() {
            "w" if value.parse::<u32>().is_ok() => (),
            "cap" if value == "flat" => (),
            "cmpd" if value == "sng" => (),
            "algn" if value == "ctr" => (),
            _ => return false,
        }
    }
    let mut no_fill = false;
    let mut dash = false;
    let mut join = false;
    for child in line.element_children() {
        if child.element_children().next().is_some() {
            return false;
        }
        match child.local_name() {
            "noFill" if !no_fill && child.attrs.is_empty() => no_fill = true,
            "prstDash" if !dash && child.attr_local("val") == Some("solid") => dash = true,
            "round" if !join && child.attrs.is_empty() => join = true,
            _ => return false,
        }
    }
    no_fill
}

fn is_office_hidden_fill_ext(ext_lst: &Element) -> bool {
    let mut exts = ext_lst.element_children();
    let Some(ext) = exts.next() else {
        return false;
    };
    if exts.next().is_some() || ext.local_name() != "ext" {
        return false;
    }
    if ext.attr_local("uri") != Some("{909E8E84-426E-40DD-AFC4-6F175D3DCCD1}") {
        return false;
    }
    let mut kids = ext.element_children();
    let Some(hidden) = kids.next() else {
        return false;
    };
    if kids.next().is_some() || hidden.local_name() != "hiddenFill" {
        return false;
    }
    let mut fills = hidden.element_children();
    let Some(remembered) = fills.next() else {
        return false;
    };
    fills.next().is_none() && matches!(remembered.local_name(), "solidFill" | "noFill")
}

/// DrawingML `a:alpha` is opacity in thousandths of a percent, 100000 = opaque.
fn parse_ooxml_alpha(val: Option<&str>) -> Option<u32> {
    let raw = val?;
    if raw.is_empty() || !raw.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let alpha: u32 = raw.parse().ok()?;
    (alpha <= 100_000).then_some(alpha)
}

fn ooxml_alpha_to_u8(alpha: u32) -> u8 {
    ((u64::from(alpha) * 255 + 50_000) / 100_000) as u8
}

/// Opaque sRGB stop-table linear gradFill at the existing 0/90 geometry. Path fills,
/// other angles, pattern, blip, and color transforms stay NI.
fn read_linear_frame_grad(grad: &Element) -> Result<FrameFill, Error> {
    let unsupported = || Error::with_path(ErrorKind::ChartImageUnsupported, "id");
    let flip = grad.attr_local("flip").unwrap_or("none");
    if flip != "none" {
        return Err(unsupported());
    }
    match grad.attr_local("rotWithShape") {
        None | Some("1") => {}
        _ => return Err(unsupported()),
    }
    let mut gs_lst = None;
    let mut lin = None;
    let mut saw_tile = false;
    for child in grad.element_children() {
        match child.local_name() {
            "gsLst" if gs_lst.is_none() => gs_lst = Some(child),
            "lin" if lin.is_none() => lin = Some(child),
            "tileRect" if !saw_tile => {
                if child.element_children().next().is_some()
                    || child.attrs.iter().any(|(_, v)| !v.is_empty() && v != "0")
                {
                    return Err(unsupported());
                }
                saw_tile = true;
            }
            _ => return Err(unsupported()),
        }
    }
    let gs_lst = gs_lst.ok_or_else(unsupported)?;
    let lin = lin.ok_or_else(unsupported)?;
    let scaled = lin.attr_local("scaled").unwrap_or("1");
    if scaled != "0" && scaled != "1" {
        return Err(unsupported());
    }
    let ang: i64 = lin
        .attr_local("ang")
        .ok_or_else(unsupported)?
        .parse()
        .map_err(|_| unsupported())?;
    if ang != LINEAR_ANG_HORIZONTAL && ang != LINEAR_ANG_VERTICAL {
        return Err(unsupported());
    }
    if lin.element_children().next().is_some() {
        return Err(unsupported());
    }
    let mut stops = Vec::new();
    for gs in gs_lst.element_children() {
        if gs.local_name() != "gs" {
            return Err(unsupported());
        }
        let pos: u32 = gs
            .attr_local("pos")
            .ok_or_else(unsupported)?
            .parse()
            .map_err(|_| unsupported())?;
        let mut colors = gs.element_children();
        let color = colors.next().ok_or_else(unsupported)?;
        if colors.next().is_some() || color.local_name() != "srgbClr" {
            return Err(unsupported());
        }
        if color.element_children().next().is_some() {
            return Err(unsupported());
        }
        let rgb = color
            .attr_local("val")
            .and_then(parse_hex)
            .ok_or_else(unsupported)?;
        stops.push((pos, rgb));
    }
    let stops = gradient::StopTable::new(stops).ok_or_else(unsupported)?;
    Ok(FrameFill::Linear(LinearFrameGrad { stops, ang }))
}

const OFFICE_GRAD_WEIGHT: [f64; 17] = [
    0.0, 0.0180, 0.0444, 0.0856, 0.1414, 0.2132, 0.2982, 0.3969, 0.5, 0.6031, 0.7018, 0.7868,
    0.8586, 0.9144, 0.9556, 0.9820, 1.0,
];

fn office_grad_weight(t: f64) -> f64 {
    let x = t.clamp(0.0, 1.0) * 16.0;
    let i = (x.floor() as usize).min(OFFICE_GRAD_WEIGHT.len() - 2);
    let f = x - i as f64;
    OFFICE_GRAD_WEIGHT[i] * (1.0 - f) + OFFICE_GRAD_WEIGHT[i + 1] * f
}

fn office_gamma_mix(start: u8, end: u8, share: f64) -> u8 {
    let from = (f64::from(start) / 255.0).powf(2.2);
    let to = (f64::from(end) / 255.0).powf(2.2);
    let mixed = (from + (to - from) * share).clamp(0.0, 1.0);
    (mixed.powf(1.0 / 2.2) * 255.0).round().clamp(0.0, 255.0) as u8
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
    let wrappers: Vec<&Element> = plot
        .element_children()
        .filter(|child| is_chart_wrapper(child.local_name()))
        .collect();
    if wrappers.is_empty() || wrappers.len() > 2 {
        return Err(unsupported());
    }
    let wrapper = wrappers[0];
    let locked_secondary = if wrappers.len() == 2 {
        Some(axis::read_locked_secondary_axes(plot)?)
    } else {
        None
    };
    let mut area_missing_as_zero = false;
    let mut stacking = Stacking::Clustered;
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
            stacking = match grouping {
                "clustered" => Stacking::Clustered,
                "stacked" => Stacking::Stacked,
                "percentStacked" => Stacking::PercentStacked,
                _ => return Err(unsupported()),
            };
            if dir == "bar" {
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
            stacking = match grouping {
                "standard" => Stacking::Clustered,
                "stacked" => Stacking::Stacked,
                "percentStacked" => Stacking::PercentStacked,
                _ => return Err(unsupported()),
            };
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
            let blanks = chart
                .child_by_local("dispBlanksAs")
                .and_then(|el| el.attr_local("val"))
                .unwrap_or("gap");
            if !matches!(blanks, "gap" | "zero" | "span") {
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
            // Official Excel 16.0 maps scatterStyle=marker (no series line hide,
            // smooth=0) to xlXYScatterLines (74) and draws connecting strokes.
            // Markers-only remains available via an explicit series a:ln/a:noFill.
            let connect = match style {
                "marker" | "lineMarker" => true,
                _ => return Err(unsupported()),
            };
            PlotKind::Scatter { connect }
        }
        "areaChart" => {
            let grouping = wrapper
                .child_by_local("grouping")
                .and_then(|el| el.attr_local("val"))
                .unwrap_or("standard");
            stacking = match grouping {
                "standard" => Stacking::Clustered,
                "stacked" => Stacking::Stacked,
                "percentStacked" => Stacking::PercentStacked,
                _ => return Err(unsupported()),
            };
            if flag(wrapper, "varyColors").unwrap_or(false) {
                return Err(unsupported());
            }
            for child in wrapper.element_children() {
                if matches!(child.local_name(), "dropLines") {
                    return Err(unsupported());
                }
            }
            data_labels_hidden(wrapper.child_by_local("dLbls"))?;
            let blanks_el = chart.child_by_local("dispBlanksAs");
            let blanks = blanks_el
                .and_then(|el| el.attr_local("val"))
                .unwrap_or("gap");
            // Omitted dispBlanksAs is official area default (missing as 0).
            // Excel 16 public saves of that default write explicit val=zero,
            // which fills the same V-to-baseline. Explicit gap stays a hole.
            // Official Excel 16 area span paints the same V-to-baseline as
            // zero, including #N/A; it is not a 4-to-9 chord. Other errors
            // and non-area types remain NI.
            area_missing_as_zero = blanks_el.is_none() || blanks == "zero" || blanks == "span";
            if blanks != "gap" && !area_missing_as_zero {
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
    if let Some(sec) = &locked_secondary {
        if !matches!(kind, PlotKind::ClusteredColumn | PlotKind::StandardLine) {
            return Err(unsupported());
        }
        if wrappers[1].local_name() != wrapper.local_name() {
            return Err(unsupported());
        }
        if kind == PlotKind::ClusteredColumn {
            let dir = wrappers[1]
                .child_by_local("barDir")
                .and_then(|el| el.attr_local("val"))
                .unwrap_or("bar");
            let grouping = wrappers[1]
                .child_by_local("grouping")
                .and_then(|el| el.attr_local("val"))
                .unwrap_or("clustered");
            if dir != "col" || grouping != "clustered" {
                return Err(unsupported());
            }
        }
        if kind == PlotKind::StandardLine {
            let grouping = wrappers[1]
                .child_by_local("grouping")
                .and_then(|el| el.attr_local("val"))
                .unwrap_or("standard");
            if grouping != "standard"
                || flag(wrappers[1], "varyColors").unwrap_or(false)
                || flag(wrappers[1], "smooth").unwrap_or(false)
            {
                return Err(unsupported());
            }
        }
        let gap1 = number_val(wrappers[1], "gapWidth").unwrap_or(150.0);
        let over1 = number_val(wrappers[1], "overlap").unwrap_or(0.0);
        if (gap1 - number_val(wrapper, "gapWidth").unwrap_or(150.0)).abs() > 1e-9
            || (over1 - number_val(wrapper, "overlap").unwrap_or(0.0)).abs() > 1e-9
        {
            return Err(unsupported());
        }
        for wrap in &wrappers {
            let nser = wrap
                .element_children()
                .filter(|el| el.local_name() == "ser")
                .count();
            if nser != 1 {
                return Err(unsupported());
            }
            let ids: Vec<_> = wrap
                .element_children()
                .filter(|el| el.local_name() == "axId")
                .filter_map(|el| el.attr_local("val"))
                .collect();
            let uses_primary = ids.iter().any(|id| *id == sec.primary_val_id.as_str());
            let uses_secondary = ids.iter().any(|id| *id == sec.secondary_val_id.as_str());
            if uses_primary == uses_secondary {
                return Err(unsupported());
            }
        }
        let _ = sec;
    }
    let vary = if kind == PlotKind::Pie {
        flag(wrapper, "varyColors").unwrap_or(true)
    } else {
        flag(wrapper, "varyColors").unwrap_or(false)
    };
    let gap_width = number_val(wrapper, "gapWidth").unwrap_or(150.0);
    let overlap = number_val(wrapper, "overlap").unwrap_or(0.0);
    // Official Excel 16 stacked / percentStacked barChart writes overlap=100.
    // lineChart / areaChart have no overlap element.
    if stacking != Stacking::Clustered
        && matches!(kind, PlotKind::ClusteredColumn | PlotKind::ClusteredBar)
    {
        match number_val(wrapper, "overlap") {
            Some(v) if (v - 100.0).abs() <= 1e-9 => {}
            _ => return Err(unsupported()),
        }
    }
    let mut series = Vec::new();
    let ser_wrappers: Vec<&Element> = if locked_secondary.is_some() {
        wrappers.clone()
    } else {
        vec![wrapper]
    };
    for wrap in &ser_wrappers {
    let on_secondary = locked_secondary.as_ref().is_some_and(|sec| {
        wrap
            .element_children()
            .filter(|el| el.local_name() == "axId")
            .filter_map(|el| el.attr_local("val"))
            .any(|id| id == sec.secondary_val_id)
    });
    for ser in wrap
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
            // Official all-empty pies omit every val cache pt while keeping
            // category pts. Do not drop that series here; the category domain
            // is applied below. Do not size the domain from untrusted ptCount
            // alone.
            if values.is_empty() && kind != PlotKind::Pie {
                continue;
            }
            (values, Vec::new())
        };
        let value_label = if matches!(
            kind,
            PlotKind::ClusteredColumn | PlotKind::ClusteredBar | PlotKind::StandardLine
        ) {
            read_value_label(wrap, ser, kind)?
        } else {
            data_labels_hidden(ser.child_by_local("dLbls"))?;
            None
        };
        if on_secondary && value_label.is_some() {
            return Err(unsupported());
        }
        if value_label == Some(ValueLabel::InsideEnd) && values.iter().flatten().any(|v| *v == 0.0)
        {
            return Err(unsupported());
        }
        if value_label.is_some()
            && values
                .iter()
                .flatten()
                .any(|v| value_label_text(*v).is_none())
        {
            return Err(unsupported());
        }
        let (error_points, unavailable_points) = if kind == PlotKind::StandardLine {
            series_error_points(ser.child_by_local("val"), sheet, cells, values.len())
        } else {
            (Vec::new(), Vec::new())
        };
        // Explicit area zero is locked only for true blanks / omitted cache
        // points. Official area span also locks #N/A as a baseline V, but
        // other errors stay NI under both modes.
        if kind == PlotKind::StandardArea
            && area_missing_as_zero
            && chart.child_by_local("dispBlanksAs").is_some()
        {
            let (errors, unavailable) =
                series_error_points(ser.child_by_local("val"), sheet, cells, values.len());
            let blanks = chart
                .child_by_local("dispBlanksAs")
                .and_then(|el| el.attr_local("val"))
                .unwrap_or("gap");
            let unverified = errors
                .iter()
                .zip(&unavailable)
                .any(|(error, na)| *error && (blanks != "span" || !*na));
            if unverified {
                return Err(unsupported());
            }
        }
        // Like the bubble-only flag, NA-as-blank is inert without NA points.
        // Do not reject a fully numeric ordinary line for an inactive option.
        if kind == PlotKind::StandardLine
            && unavailable_points.iter().any(|na| *na)
            && find_deep(root, "dispNaAsBlank")
                .is_some_and(|e| !matches!(e.attr_local("val"), Some("0" | "false" | "off")))
        {
            return Err(unsupported());
        }
        if value_label.is_some()
            && error_points
                .iter()
                .zip(&unavailable_points)
                .any(|(error, na)| *error && !na)
        {
            return Err(unsupported());
        }
        let name = series_name(ser, sheet, cells).unwrap_or_else(|| format!("系列{}", order + 1));
        let mut line_color = None;
        let style = match kind {
            PlotKind::ClusteredColumn | PlotKind::ClusteredBar => LineSeriesStyle {
        join: stroke::Join::Round,
        dash: None,
                cap: stroke::Cap::Round,
                color: series_color(ser, accents),
                visible: false,
                width: 0.0,
                marker: MarkerKind::None,
                marker_color: None,
            },
            PlotKind::StandardLine => read_line_series(ser, accents, dpi, order, true)?,
            PlotKind::StandardArea => {
                let area = read_area_series(ser, accents, dpi)?;
                line_color = area.line_color;
                LineSeriesStyle {
        join: stroke::Join::Round,
        dash: None,
                cap: stroke::Cap::Round,
                    color: area.fill,
                    visible: area.line_visible,
                    width: area.line_width,
                    marker: MarkerKind::None,
                    marker_color: None,
                }
            }
            PlotKind::Pie => read_pie_series_style(ser, accents, dpi)?,
            PlotKind::Scatter { connect } => {
                let mut style = read_line_series(ser, accents, dpi, order, false)?;
                if style.cap != stroke::Cap::Round || style.dash.is_some() { return Err(unsupported()); }
                if !connect {
                    style.visible = false;
                }
                style
            }
        };
        if series.len() <= order {
            series.resize_with(order + 1, Series::default);
        }
        let is_bar = matches!(kind, PlotKind::ClusteredBar | PlotKind::ClusteredColumn);
        let mut point_inversion = vec![None; values.len()];
        if is_bar {
            for point in ser.element_children().filter(|el| el.local_name() == "dPt") {
                if let Some(invert) = flag(point, "invertIfNegative") {
                    let index = number_val(point, "idx").ok_or_else(unsupported)?;
                    if !index.is_finite()
                        || index < 0.0
                        || index.fract() != 0.0
                        || index as usize >= point_inversion.len()
                        || point_inversion[index as usize].is_some()
                    {
                        return Err(unsupported());
                    }
                    point_inversion[index as usize] = Some(invert);
                }
            }
        }
        let negative_outline = if is_bar {
            ser.child_by_local("spPr")
                .and_then(|sp| sp.child_by_local("ln"))
                .map(|ln| read_solid_ln(ln, accents, dpi.pt_to_px_font(0.75), dpi))
                .transpose()?
        } else {
            None
        };
        let values_len = values.len();
        series[order] = Series {
            name,
            values,
            color: style.color,
            slot: order,
            line_visible: style.visible,
            line_width: style.width,
            line_cap: style.cap,
            line_join: style.join,
            line_dash: style.dash,
            marker: style.marker,
            marker_color: style.marker_color,
            line_color,
            x_values,
            invert_if_negative: is_bar && flag(ser, "invertIfNegative").unwrap_or(true),
            point_inversion,
            negative_outline,
            value_label,
            deleted_label_points: if matches!(
                kind,
                PlotKind::ClusteredColumn | PlotKind::ClusteredBar | PlotKind::StandardLine
            ) {
                read_deleted_label_points(ser.child_by_local("dLbls"), values_len)?
            } else {
                Vec::new()
            },
            error_points,
            unavailable_points,
            span_blanks: kind == PlotKind::StandardLine
                && chart
                    .child_by_local("dispBlanksAs")
                    .and_then(|el| el.attr_local("val"))
                    == Some("span"),
            on_secondary,
        };
    }
    }
    // Official all-empty pies keep category cache points but omit every val
    // pt. Do not drop that series before the category domain is applied.
    if kind != PlotKind::Pie {
        series.retain(|ser| !ser.values.is_empty() || !ser.x_values.is_empty());
    }
    if series.is_empty() {
        return Err(unsupported());
    }
    let mut n = series
        .iter()
        .map(|ser| ser.values.len().max(ser.x_values.len()))
        .max()
        .unwrap_or(0);
    if n == 0 && kind != PlotKind::Pie {
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
    if kind == PlotKind::StandardLine || kind == PlotKind::Pie {
        n = n.max(categories.len());
    }
    if n == 0 {
        return Err(unsupported());
    }
    if !matches!(kind, PlotKind::Scatter { .. }) {
        if categories.len() < n {
            let start = categories.len();
            categories.extend((start..n).map(|i| (i + 1).to_string()));
        }
        categories.truncate(n);
    }
    if kind == PlotKind::StandardLine || series.iter().any(|s| s.value_label.is_some()) {
        // Do not silently shrink the category domain when both caches omit
        // trailing points. Sparse-domain/resource-budget support is separate;
        // this batch requires a complete domain from parsed values/categories.
        for wrap in &ser_wrappers {
        for ser in wrap
            .element_children()
            .filter(|e| e.local_name() == "ser")
        {
            if let Some(count) = ser
                .child_by_local("val")
                .and_then(|e| find_deep(e, "numCache").or_else(|| find_deep(e, "numLit")))
                .and_then(|e| e.child_by_local("ptCount"))
            {
                let count = count
                    .attr_local("val")
                    .and_then(|v| v.parse::<u64>().ok())
                    .ok_or_else(unsupported)?;
                if count > n as u64 {
                    return Err(unsupported());
                }
            }
        }
        }
    }
    for ser in &mut series {
        ser.values.resize(n, None);
        // Column blank/error label text has not yet been locked by a public
        // probe. Never report successful showVal while silently omitting it.
        if matches!(kind, PlotKind::ClusteredColumn | PlotKind::ClusteredBar)
            && ser.value_label.is_some()
            && ser.values.iter().any(Option::is_none)
        {
            return Err(unsupported());
        }
        if kind == PlotKind::StandardLine {
            ser.error_points.resize(n, false);
            ser.unavailable_points.resize(n, false);
            let mode = chart
                .child_by_local("dispBlanksAs")
                .and_then(|el| el.attr_local("val"))
                .unwrap_or("gap");
            if mode != "gap"
                && ser
                    .error_points
                    .iter()
                    .zip(&ser.unavailable_points)
                    .any(|(error, na)| *error && !na)
            {
                return Err(unsupported());
            }
            if chart
                .child_by_local("dispBlanksAs")
                .and_then(|el| el.attr_local("val"))
                == Some("zero")
            {
                for (value, error) in ser.values.iter_mut().zip(&ser.error_points) {
                    if value.is_none() && !error {
                        *value = Some(0.0);
                    }
                }
            }
        }
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
    // Official Excel 16 areaChart stacked / percentStacked writes valAx
    // crossBetween=midCat (first/last categories on the plot edges). Standard
    // area and all line charts stay on the existing between path.
    let area_midcat = matches!(kind, PlotKind::StandardArea)
        && stacking != Stacking::Clustered
        && plot.element_children().filter(|el| el.local_name() == "valAx").all(|el| {
            el.child_by_local("crossBetween")
                .and_then(|c| c.attr_local("val"))
                == Some("midCat")
        });
    // Official Excel 16 scatter ChartTypes write crossBetween=midCat on both
    // numeric valAx. This is not the category-slot midCat rule; scatter x/y
    // still map linearly over the axis min/max.
    let scatter_midcat = matches!(kind, PlotKind::Scatter { .. })
        && plot.element_children().filter(|el| el.local_name() == "valAx").all(|el| {
            el.child_by_local("crossBetween")
                .and_then(|c| c.attr_local("val"))
                == Some("midCat")
        });
    if !between && !area_midcat && !scatter_midcat {
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
    let (point_colors, pie_explosion, point_explosions, series_fill, point_fills) =
        if kind == PlotKind::Pie {
            if series.len() != 1 {
                return Err(unsupported());
            }
            let ser = wrapper
                .element_children()
                .find(|el| el.local_name() == "ser")
                .ok_or_else(unsupported)?;
            read_pie_point_colors(ser, accents, n, dpi)?
        } else {
            (Vec::new(), 0.0, Vec::new(), None, Vec::new())
        };
    if kind == PlotKind::Pie {
        // Official Excel 16 public pies paint finite non-zero values at |v|
        // even when every value is negative. All-zero and all-empty pies keep
        // the frame and legend but take no slice. The previous "at least one
        // finite positive" gate is therefore not required.
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
        let (spec, from_bottom) = if stacking == Stacking::PercentStacked {
            axis::bar_axes_percent_stacked(plot)?
        } else {
            axis::bar_axes(plot)?
        };
        (spec, None, false, from_bottom)
    } else if locked_secondary.is_some() {
        let sec = locked_secondary.as_ref().unwrap();
        (sec.primary, None, false, true)
    } else if stacking == Stacking::PercentStacked {
        (axis::Spec::read_percent_stacked(plot)?, None, false, true)
    } else {
        (axis::Spec::read(plot)?, None, false, true)
    };
    let axis_secondary = locked_secondary.as_ref().map(|sec| sec.secondary);
    if axis_secondary.is_some() {
        if series.iter().any(|ser| ser.value_label.is_some())
            || !series.iter().any(|ser| ser.on_secondary)
            || !series.iter().any(|ser| !ser.on_secondary)
        {
            return Err(unsupported());
        }
    }
    if axis.log_base.is_some()
        && !matches!(kind, PlotKind::ClusteredColumn | PlotKind::StandardLine)
    {
        return Err(unsupported());
    }
    if axis_x.as_ref().is_some_and(|spec| spec.log_base.is_some()) {
        return Err(unsupported());
    }
    if axis.log_base.is_some() && series.iter().any(|ser| ser.value_label.is_some()) {
        return Err(unsupported());
    }
    if (axis.crosses_at.is_some() || !matches!(axis.cat_crosses, axis::CatCrosses::AutoZero))
        && !matches!(
            kind,
            PlotKind::ClusteredColumn | PlotKind::StandardLine | PlotKind::StandardArea
        )
    {
        return Err(unsupported());
    }
    // Official F20 evidence is clustered column, standard line, and clustered bar.
    if axis.deleted
        && !matches!(
            kind,
            PlotKind::ClusteredColumn | PlotKind::StandardLine | PlotKind::ClusteredBar
        )
    {
        return Err(unsupported());
    }
    if stacking != Stacking::Clustered {
        // Official F22 evidence is linear autoZero minMax, explicit min/max,
        // no value labels, no inversion, no log / maxMin / delete / custom crosses.
        if axis.min.is_none()
            || axis.max.is_none()
            || axis.log_base.is_some()
            || axis.reversed
            || axis.deleted
            || axis.crosses_at.is_some()
            || !matches!(axis.cat_crosses, axis::CatCrosses::AutoZero)
            || series.iter().any(|ser| ser.value_label.is_some())
            || series.iter().any(|ser| {
                ser.invert_if_negative || ser.point_inversion.iter().any(|p| p.is_some())
            })
        {
            return Err(unsupported());
        }
        if stacking == Stacking::PercentStacked {
            if axis.min != Some(0.0)
                || axis.max != Some(1.0)
                || axis.tick_format != axis::TickFormat::Percent
            {
                return Err(unsupported());
            }
            for ser in &series {
                if ser.values.iter().any(|v| {
                    !v.is_some_and(|n| n.is_finite() && n > 0.0)
                }) {
                    return Err(unsupported());
                }
            }
        }
        // Official F23 line/area evidence has every cache point finite.
        // Blanks, packing, and percentStacked mixed/missing stay NI.
        if matches!(kind, PlotKind::StandardLine | PlotKind::StandardArea) {
            if series.iter().any(|ser| {
                ser.values.iter().any(|v| !v.is_some_and(|n| n.is_finite()))
            }) {
                return Err(unsupported());
            }
        }
        // Official xlLineStacked writes c:marker/symbol=none. Auto or shaped
        // markers on stacked lines are not uniquely locked.
        if kind == PlotKind::StandardLine
            && series
                .iter()
                .any(|ser| !matches!(ser.marker, MarkerKind::None))
        {
            return Err(unsupported());
        }
        if kind == PlotKind::StandardArea && !area_midcat {
            return Err(unsupported());
        }
        if !matches!(
            kind,
            PlotKind::ClusteredColumn
                | PlotKind::ClusteredBar
                | PlotKind::StandardLine
                | PlotKind::StandardArea
        ) {
            return Err(unsupported());
        }
    }
    Ok(ColumnChart {
        categories,
        series,
        gap_width,
        overlap,
        stacking,
        vary_points,
        grid,
        grid_x,
        legend: legend_pos(chart),
        legend_spec: legend::Spec::read(root, accents)?,
        axis,
        axis_x,
        axis_secondary,
        cats_from_bottom,
        title: title_text(chart),
        accents,
        kind,
        first_slice_ang,
        point_colors,
        series_fill,
        point_fills,
        pie_explosion,
        point_explosions,
        area_missing_as_zero,
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
    if y_scale.reversed || y_scale.deleted {
        return Err(unsupported());
    }
    let y_ticks = tick_values(y_scale.min, y_scale.max, y_scale.major);
    let y_labels: Vec<String> = y_ticks.iter().copied().map(|v| fmt_tick_with(v, y_scale.tick_format)).collect();
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
        scale_plot_leading(canvas.dpi, INSET_TOP)
    };
    let inset_right = legend_right_inset(chart, font, gdi.as_ref(), label_px, canvas.dpi);
    let mut plot_r = canvas.w as i32 - inset_right;
    let mut plot_b = canvas.h as i32 - scale_plot_trailing(canvas.dpi, INSET_BOTTOM);
    if !chart.legend_spec.legacy {
        let reserve = legend_layout.reserve();
        axis_x += reserve[0];
        plot_x = axis_x + 1;
        plot_y += reserve[1];
        plot_r = canvas.w as i32 - 1 - scale_plot_leading(canvas.dpi, INSET_TOP) - reserve[2];
        plot_b -= reserve[3];
        if plot_r <= plot_x || plot_b <= plot_y {
            return Err(legend::unsupported());
        }
    }
    // autoZero: when 0 is inside the Y scale, the horizontal axis sits at y=0
    // inside the plot, so the bottom no longer reserves a label gutter.
    let y_cross_interior = y_scale.min < -1e-12 && y_scale.max > 1e-12;
    if y_cross_interior {
        plot_b = canvas.h as i32 - scale_plot_trailing(canvas.dpi, INSET_TOP);
        if !chart.legend_spec.legacy {
            plot_b -= legend_layout.reserve()[3];
        }
        if plot_r <= plot_x || plot_b <= plot_y {
            return Err(legend::unsupported());
        }
    }
    let x_spec = chart.axis_x.ok_or_else(unsupported)?;
    let x_budget = axis::interval_budget_span(f64::from((plot_r - plot_x).max(1)), em, 3.0);
    let mut x_scale = axis::scale_values(
        chart.series.iter().flat_map(|s| s.x_values.iter().copied()),
        x_spec,
        x_budget,
    )?;
    if x_scale.reversed || x_scale.deleted {
        return Err(unsupported());
    }
    // When 0 is inside the X scale, the vertical axis sits at x=0 inside the
    // plot; drop the left tick-label gutter and use the same frame inset as the top.
    let x_cross_interior = x_scale.min < -1e-12 && x_scale.max > 1e-12;
    if x_cross_interior {
        let mut slim = canvas.dpi.scale_px(INSET_TOP);
        if !chart.legend_spec.legacy {
            slim += legend_layout.reserve()[0];
        }
        if slim != plot_x {
            axis_x = slim;
            plot_x = slim;
            let x_budget = axis::interval_budget_span(f64::from((plot_r - plot_x).max(1)), em, 3.0);
            x_scale = axis::scale_values(
                chart.series.iter().flat_map(|s| s.x_values.iter().copied()),
                x_spec,
                x_budget,
            )?;
        }
    }
    let x_ticks = tick_values(x_scale.min, x_scale.max, x_scale.major);
    let x_labels: Vec<String> = x_ticks.iter().copied().map(|v| fmt_tick_with(v, x_scale.tick_format)).collect();
    let plot = Rect {
        x: f64::from(plot_x),
        y: f64::from(plot_y),
        w: f64::from(plot_r - plot_x),
        h: f64::from(plot_b - plot_y),
    };
    if let Some(fill) = canvas.plot_fill.clone() {
        canvas.paint_plot(
            plot.x as i32,
            plot.y as i32,
            (plot.x + plot.w) as i32,
            (plot.y + plot.h) as i32,
            fill,
        );
    }
    let y_regular = (((y_scale.max - y_scale.min) / y_scale.major).round() * y_scale.major
        - (y_scale.max - y_scale.min))
        .abs()
        <= y_scale.major * 1e-8;
    let grid_left = if x_cross_interior {
        plot_x
    } else {
        axis_x - canvas.dpi.scale_px(TICK_OUT)
    };
    if chart.grid && y_regular {
        for i in 0..=y_intervals {
            let y = regular_grid_y(plot_y, plot_b, i, y_intervals);
            canvas.hline_i(grid_left, plot_r, y, INK);
        }
    }
    if chart.grid && !y_regular {
        for value in &y_ticks {
            let y = y_of(*value, y_scale.min, y_scale.max, &plot).ceil() as i32;
            canvas.hline_i(grid_left, plot_r, y, INK);
        }
    }
    if chart.grid_x {
        for value in &x_ticks {
            let x = x_of(*value, x_scale.min, x_scale.max, &plot).round() as i32;
            canvas.vline_i(x, plot_y, plot_b, INK);
        }
    }
    let y_parts = y_scale.major / y_scale.minor;
    let y_tick_x = if x_cross_interior {
        x_of(x_scale.crossing(), x_scale.min, x_scale.max, &plot).round() as i32
    } else {
        axis_x
    };
    if y_regular && (y_parts - y_parts.round()).abs() < 1e-8 && (1.0..=100.0).contains(&y_parts) {
        draw_value_minor_ticks(
            canvas,
            y_tick_x,
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
                y_tick_x - canvas.dpi.scale_px(VAL_MINOR_ARM),
                y_tick_x + canvas.dpi.scale_px(VAL_MINOR_ARM),
                y,
                INK,
            );
        }
    }
    draw_scatter(canvas, chart, &plot, &x_scale, &y_scale);
    let axis_line_x = x_of(x_scale.crossing(), x_scale.min, x_scale.max, &plot).round() as i32;
    let axis_line_y = y_of(y_scale.crossing(), y_scale.min, y_scale.max, &plot).round() as i32;
    canvas.vline_i(axis_line_x, plot_y, plot_b, INK);
    let axis_hline_left = if x_cross_interior { plot_x } else { axis_x };
    canvas.hline_i(axis_hline_left, plot_r, axis_line_y, INK);
    if x_cross_interior {
        for value in &y_ticks {
            let y = y_of(*value, y_scale.min, y_scale.max, &plot).round() as i32;
            canvas.hline_i(
                axis_line_x - canvas.dpi.scale_px(CAT_MAJOR),
                axis_line_x + canvas.dpi.scale_px(CAT_MAJOR_IN),
                y,
                INK,
            );
        }
    }
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
    let y_label_axis = if x_cross_interior {
        axis_line_x
    } else {
        axis_x
    };
    for (tick, label) in y_ticks.iter().zip(&y_labels) {
        let y = y_of(*tick, y_scale.min, y_scale.max, &plot);
        let w = text_width(font, gdi.as_ref(), label, label_px);
        let h = text_height(font, label_px);
        draw_text(
            canvas,
            font,
            gdi.as_ref(),
            label,
            f64::from(y_label_axis) - w - f64::from(canvas.dpi.scale_px(TICK_GAP)),
            y - h / 2.0,
            label_px,
            INK,
        );
    }
    let baseline = if y_cross_interior {
        f64::from(axis_line_y) + canvas.dpi.scale_f(10.0)
    } else {
        f64::from(canvas.h as i32 - scale_plot_trailing(canvas.dpi, INSET_BOTTOM))
            + canvas.dpi.scale_f(10.0)
    };
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
    if chart.legend_spec.default_right_geometry() {
        let legend_plot_r = canvas.w as i32 - inset_right;
        draw_legend(canvas, chart, font, gdi.as_ref(), label_px, legend_plot_r)?;
    } else {
        legend_layout.draw(canvas, chart, font)?;
    }
    if matches!(host, ChartHost::Excel) {
        if canvas.chart_border {
            canvas.stroke_border(BORDER);
        }
    }
    Ok(())
}

fn x_of(value: f64, axis_min: f64, axis_max: f64, plot: &Rect) -> f64 {
    x_of_dir(value, axis_min, axis_max, plot, false)
}

fn x_of_dir(value: f64, axis_min: f64, axis_max: f64, plot: &Rect, reversed: bool) -> f64 {
    let span = (axis_max - axis_min).max(1e-12);
    let mut t = (value - axis_min) / span;
    if reversed {
        t = 1.0 - t;
    }
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
                        stroke_run_style(canvas, &run, stroke::Style { width: ser.line_width, cap: ser.line_cap }, color);
                    }
                    run.clear();
                }
            }
        }
        if ser.line_visible && ser.line_width > 0.0 {
            stroke_run_style(canvas, &run, stroke::Style { width: ser.line_width, cap: ser.line_cap }, color);
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
    // Official Excel 16.0 draws the value axis at the top when catAx is maxMin
    // (first category on top), and at the bottom when minMax, even if valAx
    // axPos is still `b`. Unused gutters reuse INSET_TOP; the label side reuses
    // INSET_BOTTOM. This is not a guessed 1px shift.
    let val_axis_at_top = !chart.cats_from_bottom;
    let val_deleted = chart.axis.deleted;
    let val_label_top = scale_plot_leading(canvas.dpi, INSET_BOTTOM);
    let val_label_bottom = scale_plot_trailing(canvas.dpi, INSET_BOTTOM);
    let unused_top = scale_plot_leading(canvas.dpi, INSET_TOP);
    let unused_bottom = scale_plot_trailing(canvas.dpi, INSET_TOP);
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
        if val_axis_at_top && !val_deleted {
            y += val_label_top;
        }
        y
    } else if val_axis_at_top && !val_deleted {
        val_label_top
    } else {
        unused_top
    };
    let inset_right = legend_right_inset(chart, font, gdi.as_ref(), label_px, canvas.dpi);
    let mut plot_r = canvas.w as i32 - inset_right;
    let mut plot_b = canvas.h as i32
        - if val_axis_at_top || val_deleted {
            unused_bottom
        } else {
            val_label_bottom
        };
    if !chart.legend_spec.legacy {
        let reserve = legend_layout.reserve();
        axis_x += reserve[0];
        plot_x = axis_x + 1;
        plot_y += reserve[1];
        plot_r = canvas.w as i32 - 1 - scale_plot_leading(canvas.dpi, INSET_TOP) - reserve[2];
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
    if let Some(fill) = canvas.plot_fill.clone() {
        canvas.paint_plot(
            plot.x as i32,
            plot.y as i32,
            (plot.x + plot.w) as i32,
            (plot.y + plot.h) as i32,
            fill,
        );
    }
    // 数值轴在水平方向，间隔预算按绘图区宽度。占用系数用 1，与竖向数值轴同一套
    // 行盒口径，上限仍是 10。occupancy=3 是散点 X 轴为较长标签留的余量，在典型
    // 条形宽度下会把预算压到 9，从而把 Office 对 -8..8 的自动主间隔 2（-10..10
    // 共 10 格）拒掉，改成间隔 5。短数字标签在 10 格下仍放得下。
    let scale = axis::scale(
        &chart.series,
        chart.axis,
        axis::interval_budget_span(plot.w, em, 1.0),
    )?;
    let (axis_min, axis_max, step) = (scale.min, scale.max, scale.major);
    let ticks = tick_values(axis_min, axis_max, step);
    let tick_labels: Vec<String> = ticks.iter().copied().map(|v| fmt_tick_with(v, scale.tick_format)).collect();
    let axis_y = if val_axis_at_top { plot_y } else { plot_b };
    // 主刻度、网格与条右缘共用 x_of 四舍五入，避免整数 ceil 网格与条端错开 1px。
    if chart.grid {
        for value in &ticks {
            let x = x_of_dir(*value, axis_min, axis_max, &plot, scale.reversed).round() as i32;
            canvas.vline_i(x, plot_y, plot_b, INK);
        }
    }
    if !scale.deleted {
        for value in tick_values(axis_min + scale.minor, axis_max, scale.minor) {
            let at_major = (value - axis_min) / step;
            if (at_major - at_major.round()).abs() < 1e-8 {
                continue;
            }
            let x = x_of_dir(value, axis_min, axis_max, &plot, scale.reversed).round() as i32;
            canvas.vline_i(
                x,
                axis_y - canvas.dpi.scale_px(VAL_MINOR_ARM),
                axis_y + canvas.dpi.scale_px(VAL_MINOR_ARM),
                INK,
            );
        }
    }
    if scale.reversed && chart.series.iter().any(|s| s.value_label.is_some()) {
        return Err(Error::with_path(ErrorKind::ChartImageUnsupported, "id"));
    }
    draw_clustered_bars(canvas, chart, &plot, axis_min, axis_max, scale.reversed);
    draw_horizontal_value_labels(
        canvas,
        chart,
        &plot,
        axis_min,
        axis_max,
        font,
        gdi.as_ref(),
        label_px,
        scale.reversed,
    )?;
    let zero_x = x_of_dir(scale.crossing(), axis_min, axis_max, &plot, scale.reversed).round() as i32;
    canvas.vline_i(zero_x, plot_y, plot_b, INK);
    if !scale.deleted {
        canvas.hline_i(plot_x, plot_r, axis_y, INK);
    }
    draw_cat_ticks_y(
        canvas,
        chart.categories.len(),
        plot.y,
        plot.y + plot.h + 1.0,
        zero_x,
        chart.cats_from_bottom,
    );
    // With an interior autoZero crossing, official category labels sit next
    // to that axis rather than remaining in the unused left label gutter.
    // Preserve edge-axis geometry and all plot gutters; this only moves ink.
    let cat_label_axis_x = if axis_min < scale.crossing() && scale.crossing() < axis_max {
        zero_x
    } else {
        axis_x
    };
    // 数值标签在顶轴上方或底轴下方，跟随官方 maxMin/minMax。
    for (i, label) in chart.categories.iter().enumerate() {
        let mid = category_mid_y(&plot, i, chart.categories.len(), chart.cats_from_bottom);
        let w = text_width(font, gdi.as_ref(), label, label_px);
        let h = text_height(font, label_px);
        draw_text(
            canvas,
            font,
            gdi.as_ref(),
            label,
            f64::from(cat_label_axis_x) - w - f64::from(canvas.dpi.scale_px(TICK_GAP)),
            mid - h / 2.0,
            label_px,
            INK,
        );
    }
    if !scale.deleted {
        let tick_h = text_height(font, label_px);
        let label_top = if val_axis_at_top {
            f64::from(axis_y) - canvas.dpi.scale_f(10.0) - tick_h
        } else {
            f64::from(axis_y) + canvas.dpi.scale_f(10.0)
        };
        for (tick, label) in ticks.iter().zip(&tick_labels) {
            let w = text_width(font, gdi.as_ref(), label, label_px);
            let x = x_of_dir(*tick, axis_min, axis_max, &plot, scale.reversed) - w / 2.0;
            draw_text(
                canvas,
                font,
                gdi.as_ref(),
                label,
                x,
                label_top,
                label_px,
                INK,
            );
        }
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
    if chart.legend_spec.default_right_geometry() {
        let legend_plot_r = canvas.w as i32 - inset_right;
        draw_legend(canvas, chart, font, gdi.as_ref(), label_px, legend_plot_r)?;
    } else {
        legend_layout.draw(canvas, chart, font)?;
    }
    if matches!(host, ChartHost::Excel) {
        if canvas.chart_border {
            canvas.stroke_border(BORDER);
        }
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
    // Public Office PlotArea probes: the pie is not an inscribed circle of
    // the whole host. Title and right legend bands are removed first; Office
    // then contracts the shorter side by 7/8 and reserves 11 pt on each side.
    // These are point-space layout values, not export-pixel corrections.
    // ChartTitle TextFrame2 has 3.6 pt top/bottom margins; reserve the
    // additional 6 pt title/plot separation. Font fallback can still differ
    // from the host theme and remains a separately reported layout residual.
    let title_band = if chart.title.is_some() {
        title_face
            .as_ref()
            .map_or_else(|| text_height(font, label_px), |face| face.em_box())
            + canvas.dpi.pt_to_px_font(3.6 * 2.0 + 6.0)
    } else {
        0.0
    };
    let legend_band = if chart.legend == Some(LegendPos::Right) {
        let line = gdi
            .as_ref()
            .map_or_else(|| text_height(font, label_px), |face| face.em_box());
        let widest = pie_legend_items(chart)
            .iter()
            .map(|(name, _)| {
                gdi.as_ref().map_or_else(
                    || text_width(font, None, name, label_px),
                    |face| face.typo_width(name),
                )
            })
            .fold(0.0, f64::max);
        // LegendEntry has 1.5 pt padding on either side. Its square key
        // measures 45% of the font line box; the entry reserves 2.5 keys
        // including its side spaces. The plot/legend separation is 6 pt.
        widest + line * 0.45 * 2.5 + canvas.dpi.pt_to_px_font(9.0)
    } else {
        0.0
    };
    let plot = pie_plot_bounds(
        f64::from(canvas.w),
        f64::from(canvas.h),
        title_band,
        legend_band,
        canvas.dpi,
    )
    .ok_or_else(unsupported)?;
    let inset_right = if chart.legend == Some(LegendPos::Right) {
        pie_legend_right_inset(chart, font, gdi.as_ref(), label_px, canvas.dpi)
    } else {
        canvas.dpi.scale_px(INSET_TOP)
    };
    if let Some(PlotPaint::Solid(color, 255)) = canvas.plot_fill {
        canvas.fill_fractional_rect(&plot, color);
    }
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
        if canvas.chart_border {
            canvas.stroke_border(BORDER);
        }
    }
    Ok(())
}

/// Auto-layout of the supported flat pie, in host-local pixels. The bands
/// include the title/legend padding. Public Excel/PowerPoint PlotArea values
/// have a 4 pt chart-local origin; host coordinates already include that origin.
fn pie_plot_bounds(
    width: f64,
    height: f64,
    title_band: f64,
    legend_band: f64,
    dpi: RenderDpi,
) -> Option<Rect> {
    let available_width = width - legend_band;
    let available_height = height - title_band;
    let diameter = available_width.min(available_height) * 0.875 - dpi.pt_to_px_font(22.0);
    (diameter > 1.0).then_some(Rect {
        x: (available_width - diameter) / 2.0,
        y: title_band + (available_height - diameter) / 2.0,
        w: diameter,
        h: diameter,
    })
}

fn pie_slice_fill(chart: &ColumnChart, index: usize) -> SliceFill {
    if let Some(Some(fill)) = chart.point_fills.get(index) {
        return *fill;
    }
    // RGB cache of a non-noFill dPt override; point_fills already won above
    // when both are populated together.
    if let Some(Some(color)) = chart.point_colors.get(index) {
        return SliceFill {
            color: *color,
            alpha: 255,
        };
    }
    if let Some(fill) = chart.series_fill {
        return fill;
    }
    let color = if chart.vary_points {
        chart.accents[index % 6]
    } else {
        chart.series[0]
            .color
            .unwrap_or(chart.accents[chart.series[0].slot % 6])
    };
    SliceFill { color, alpha: 255 }
}

fn pie_legend_items(chart: &ColumnChart) -> Vec<(String, SliceFill)> {
    let ser = &chart.series[0];
    chart
        .categories
        .iter()
        .enumerate()
        .filter_map(|(index, name)| {
            // Official Excel still lists omitted/blank categories in the pie
            // legend and keeps their accent slot, even though they take no slice.
            match ser.values.get(index).copied().flatten() {
                // Official Excel still lists omitted/blank/zero categories in
                // the pie legend and keeps their accent slot. Finite negatives
                // stay in the legend as well, because they take a |v| slice.
                Some(value) if !value.is_finite() => None,
                _ => Some((name.clone(), pie_slice_fill(chart, index))),
            }
        })
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
    for (name, fill) in items {
        if fill.alpha > 0 {
            canvas.fill_rect_cov(
                x,
                y,
                x + canvas.dpi.scale_px(LEGEND_SWATCH),
                y + canvas.dpi.scale_px(LEGEND_SWATCH),
                fill.color,
                fill.alpha,
            );
        }
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
    // Official Chart.Export of mixed-sign pies matches the same categories
    // at |v|, not a skip-zero or omitted-category layout.
    let sum: f64 = ser
        .values
        .iter()
        .flatten()
        .filter(|value| value.is_finite() && value.abs() > 0.0)
        .map(|value| value.abs())
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
    // Official Excel 16 series explosion shrinks the pie about the same
    // center by 1/(1+e/100) and offsets every slice along its bisector by
    // (e/100)*R' so the outer radius stays R. A single dPt explosion leaves
    // the other slices and R unchanged and offsets only that slice by
    // (e/100)*R along its bisector. Series and dPt explosion together are
    // not uniquely locked and stay NI at read time.
    let series_exp = chart.pie_explosion;
    let scale = if series_exp > 0.0 {
        1.0 / (1.0 + series_exp / 100.0)
    } else {
        1.0
    };
    let r_draw = radius * scale;
    let mut start = chart.first_slice_ang.rem_euclid(360.0);
    for (index, value) in ser.values.iter().enumerate() {
        let Some(value) = *value else {
            continue;
        };
        if !value.is_finite() || value.abs() <= 0.0 {
            continue;
        }
        let end = start + 360.0 * (value.abs() / sum);
        let mid = (start + end) / 2.0;
        let point_exp = chart
            .point_explosions
            .get(index)
            .copied()
            .unwrap_or(0.0);
        let offset = if series_exp > 0.0 {
            (series_exp / 100.0) * r_draw
        } else {
            (point_exp / 100.0) * radius
        };
        let mid_rad = mid.to_radians();
        let ox = cx + mid_rad.sin() * offset;
        let oy = cy - mid_rad.cos() * offset;
        let fill = pie_slice_fill(chart, index);
        if fill.alpha > 0 {
            canvas.fill_wedge(ox, oy, r_draw, start, end, fill.color, fill.alpha);
        }
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
    let budget = axis::interval_budget_dpi(canvas.h, em, canvas.dpi);
    let scale = if chart.axis_secondary.is_some() {
        axis::scale_values(
            chart
                .series
                .iter()
                .filter(|ser| !ser.on_secondary)
                .flat_map(|ser| ser.values.iter().copied()),
            chart.axis,
            budget,
        )?
    } else {
        axis::scale(&chart.series, chart.axis, budget)?
    };
    let secondary_scale = if let Some(spec) = chart.axis_secondary {
        Some(axis::scale_values(
            chart
                .series
                .iter()
                .filter(|ser| ser.on_secondary)
                .flat_map(|ser| ser.values.iter().copied()),
            spec,
            budget,
        )?)
    } else {
        None
    };
    let (axis_min, axis_max, step) = (scale.min, scale.max, scale.major);
    let log_base = scale.log_base;
    let ticks = if let Some(base) = log_base {
        tick_values_log(axis_min, axis_max, base)
    } else {
        tick_values(axis_min, axis_max, step)
    };
    let tick_labels: Vec<String> = ticks.iter().copied().map(|v| fmt_tick_with(v, scale.tick_format)).collect();
    let intervals = ticks.len().saturating_sub(1).max(1) as i32;
    let widest_tick = if scale.deleted {
        // Official valAx delete=true releases the value-label gutter; the
        // axis line and tick labels are omitted. This is not a 1px fudge.
        0
    } else {
        tick_labels
            .iter()
            .map(|label| {
                let width = text_width(font, gdi.as_ref(), label, label_px);
                // Excel hangs half the minus advance outside the numeric label
                // column. Counting it as a full digit widens all signed plots.
                let hanging = if label.starts_with('-') {
                    text_width(font, gdi.as_ref(), "-", label_px) / 2.0
                } else {
                    0.0
                };
                (width - hanging).ceil() as i32
            })
            .max()
            .unwrap_or(0)
    };
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
        scale_plot_leading(canvas.dpi, INSET_TOP)
    };
    let inset_right = legend_right_inset(chart, font, gdi.as_ref(), label_px, canvas.dpi);
    let mut plot_r = canvas.w as i32 - inset_right;
    let mut plot_b = canvas.h as i32 - scale_plot_trailing(canvas.dpi, INSET_BOTTOM);
    if !chart.legend_spec.legacy {
        let reserve = legend_layout.reserve();
        axis_x += reserve[0];
        plot_x = axis_x + 1;
        plot_y += reserve[1];
        plot_r = canvas.w as i32 - 1 - scale_plot_leading(canvas.dpi, INSET_TOP) - reserve[2];
        plot_b -= reserve[3];
        if plot_r <= plot_x || plot_b <= plot_y {
            return Err(legend::unsupported());
        }
    }
    // autoZero category labels follow the crossing axis. When that axis is
    // inside the plot (or at its top for negative-only values), keeping the
    // bottom category-label gutter shortens every tick interval and moves 0.
    if !scale.category_labels_below() {
        plot_b = canvas.h as i32 - 1 - scale_plot_leading(canvas.dpi, INSET_TOP);
        if !chart.legend_spec.legacy {
            plot_b -= legend_layout.reserve()[3];
        }
    }
    // Official crosses=max keeps the category-label gutter on the top edge and
    // releases the unused bottom gutter. A crossesAt that merely clamps to max
    // (F16-6) does not get this top gutter. Official valAx maxMin with the
    // crossing at min likewise keeps the category labels in the top gutter.
    if scale.category_labels_above() {
        plot_y += scale_plot_trailing(canvas.dpi, INSET_BOTTOM)
            - scale_plot_leading(canvas.dpi, INSET_TOP);
    }
    let (secondary_ticks, secondary_tick_labels, right_axis_x) =
        if let Some(sec) = secondary_scale.as_ref() {
            let ticks = tick_values(sec.min, sec.max, sec.major);
            let labels: Vec<String> = ticks
                .iter()
                .copied()
                .map(|v| fmt_tick_with(v, sec.tick_format))
                .collect();
            let widest = labels
                .iter()
                .map(|label| {
                    let width = text_width(font, gdi.as_ref(), label, label_px);
                    let hanging = if label.starts_with('-') {
                        text_width(font, gdi.as_ref(), "-", label_px) / 2.0
                    } else {
                        0.0
                    };
                    (width - hanging).ceil() as i32
                })
                .max()
                .unwrap_or(0);
            // Reserve the mirrored label column between the plot and the
            // already-reserved legend. The right axis sits on the new plot
            // edge; labels start TICK_GAP to its right. Not a 1px fudge.
            let gutter = widest + canvas.dpi.scale_px(TICK_GAP);
            plot_r -= gutter;
            if plot_r <= plot_x {
                return Err(Error::with_path(ErrorKind::ChartImageUnsupported, "id"));
            }
            (ticks, labels, plot_r)
        } else {
            (Vec::new(), Vec::new(), plot_r)
        };
    let plot = Rect {
        x: f64::from(plot_x),
        y: f64::from(plot_y),
        w: f64::from(plot_r - plot_x),
        h: f64::from(plot_b - plot_y),
    };
    if let Some(fill) = canvas.plot_fill.clone() {
        canvas.paint_plot(
            plot.x as i32,
            plot.y as i32,
            (plot.x + plot.w) as i32,
            (plot.y + plot.h) as i32,
            fill,
        );
    }
    let regular = log_base.is_some()
        || (((axis_max - axis_min) / step).round() * step - (axis_max - axis_min)).abs()
            <= step * 1e-8;
    let grid_x0 = if scale.deleted {
        plot_x
    } else {
        axis_x - canvas.dpi.scale_px(TICK_OUT)
    };
    let grid_x1 = if secondary_scale.is_some() {
        right_axis_x + canvas.dpi.scale_px(TICK_OUT)
    } else {
        plot_r
    };
    if chart.grid && regular {
        for i in 0..=intervals {
            let y = regular_grid_y(plot_y, plot_b, i, intervals);
            canvas.hline_i(grid_x0, grid_x1, y, INK);
        }
    }
    if chart.grid && !regular {
        for value in &ticks {
            let y = y_map(*value, axis_min, axis_max, &plot, log_base, scale.reversed).ceil() as i32;
            canvas.hline_i(grid_x0, grid_x1, y, INK);
        }
    }
    let parts = step / scale.minor;
    if scale.deleted {
        // Official F20 omits value-axis tick marks along with the axis line.
    } else if log_base.is_some() {
        // Official Excel 16 log axes in this subset have minorTickMark=none.
    } else if regular && (parts - parts.round()).abs() < 1e-8 && (1.0..=100.0).contains(&parts) {
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
            let y = y_map(value, axis_min, axis_max, &plot, None, scale.reversed).round() as i32;
            canvas.hline_i(
                axis_x - canvas.dpi.scale_px(VAL_MINOR_ARM),
                axis_x + canvas.dpi.scale_px(VAL_MINOR_ARM),
                y,
                INK,
            );
        }
    }
    match chart.kind {
        PlotKind::ClusteredColumn => {
            if secondary_scale.is_some() {
                // Official same-type secondary columns overlap: each wrapper
                // is a one-series cluster. Paint primary first, secondary on top.
                draw_bars(
                    canvas,
                    chart,
                    &plot,
                    axis_min,
                    axis_max,
                    step,
                    log_base,
                    scale.crossing(),
                    scale.reversed,
                    Some(false),
                );
                let sec = secondary_scale.as_ref().unwrap();
                draw_bars(
                    canvas,
                    chart,
                    &plot,
                    sec.min,
                    sec.max,
                    sec.major,
                    sec.log_base,
                    sec.crossing(),
                    sec.reversed,
                    Some(true),
                );
            } else {
                draw_bars(
                    canvas,
                    chart,
                    &plot,
                    axis_min,
                    axis_max,
                    step,
                    log_base,
                    scale.crossing(),
                    scale.reversed,
                    None,
                );
            }
        }
        PlotKind::StandardLine => {
            if let Some(sec) = secondary_scale.as_ref() {
                draw_lines(
                    canvas,
                    chart,
                    &plot,
                    axis_min,
                    axis_max,
                    log_base,
                    scale.reversed,
                    Some(false),
                )?;
                draw_lines(
                    canvas,
                    chart,
                    &plot,
                    sec.min,
                    sec.max,
                    sec.log_base,
                    sec.reversed,
                    Some(true),
                )?;
            } else {
                draw_lines(canvas, chart, &plot, axis_min, axis_max, log_base, scale.reversed, None)?;
            }
        }
        PlotKind::StandardArea => draw_areas(
            canvas,
            chart,
            &plot,
            axis_min,
            axis_max,
            scale.crossing(),
            scale.reversed,
        ),
        PlotKind::ClusteredBar | PlotKind::Pie | PlotKind::Scatter { .. } => {
            unreachable!("bar/pie/scatter charts use dedicated paint")
        }
    }
    if scale.reversed && chart.series.iter().any(|s| s.value_label.is_some()) {
        return Err(Error::with_path(ErrorKind::ChartImageUnsupported, "id"));
    }
    draw_value_labels(
        canvas,
        chart,
        &plot,
        axis_min,
        axis_max,
        font,
        gdi.as_ref(),
        label_px,
    )?;
    if !scale.deleted {
        canvas.vline_i(axis_x, plot_y, plot_b, INK);
    }
    // 次刻度在类目正中，会穿过柱底。先画柱再画刻度，向上的那一截才留得住。
    let category_y =
        crossing_axis_y_mode(&plot, axis_min, axis_max, step, scale.crossing(), log_base, scale.reversed);
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
        log_base,
        scale.category_labels_above(),
        scale.reversed,
        !scale.deleted,
    );
    if let Some(sec) = secondary_scale.as_ref() {
        canvas.vline_i(right_axis_x, plot_y, plot_b, INK);
        draw_right_tick_labels(
            canvas,
            font,
            gdi.as_ref(),
            label_px,
            &plot,
            right_axis_x,
            &secondary_ticks,
            &secondary_tick_labels,
            sec.min,
            sec.max,
            sec.log_base,
            sec.reversed,
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
    if chart.legend_spec.default_right_geometry() {
        // An overlay must release plot space without changing the default legend.
        // Anchor its ink at the same right edge, not at the expanded plot edge.
        let legend_plot_r = canvas.w as i32 - inset_right;
        draw_legend(canvas, chart, font, gdi.as_ref(), label_px, legend_plot_r)?;
    } else {
        legend_layout.draw(canvas, chart, font)?;
    }
    if matches!(host, ChartHost::Excel) {
        if canvas.chart_border {
            canvas.stroke_border(BORDER);
        }
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
        let y0 = regular_grid_y(plot_y, plot_b, i, intervals);
        let y1 = regular_grid_y(plot_y, plot_b, i + 1, intervals);
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

fn draw_lines(
    canvas: &mut Canvas,
    chart: &ColumnChart,
    plot: &Rect,
    axis_min: f64,
    axis_max: f64,
    log_base: Option<f64>,
    reversed: bool,
    group_secondary: Option<bool>,
) -> Result<(),Error> {
    let cats = chart.categories.len().max(1) as f64;
    let slot = (plot.w + 1.0) / cats;
    if slot <= 0.0 {
        return Ok(());
    }
    let stacked = chart.stacking != Stacking::Clustered;
    let percent = chart.stacking == Stacking::PercentStacked;
    let group: Vec<&Series> = chart
        .series
        .iter()
        .filter(|ser| !ser.values.is_empty())
        .collect();
    for (si, ser) in group.iter().enumerate() {
        if !group_secondary
            .map(|flag| ser.on_secondary == flag)
            .unwrap_or(true)
        {
            continue;
        }
        let color = ser.color.unwrap_or(chart.accents[ser.slot % 6]);
        if ser.line_visible && ser.line_width > 0.0 {
            let mut run = Vec::new();
            for (ci, value) in ser.values.iter().enumerate() {
                let plotted = if stacked {
                    stack_cumulative(&group, ci, si, percent).map(|(_, end)| end)
                } else if let Some(v) = value {
                    if v.is_finite() {
                        Some(*v)
                    } else {
                        None
                    }
                } else if ser.unavailable_points.get(ci).copied().unwrap_or(false)
                    || (ser.span_blanks
                        && !ser.error_points.get(ci).copied().unwrap_or(false))
                {
                    None
                } else {
                    stroke_run_pattern_join(canvas, &run, stroke::Style { width: ser.line_width, cap: ser.line_cap }, ser.line_dash.as_deref(), ser.line_join, color)?;
                    run.clear();
                    None
                };
                if let Some(v) = plotted {
                    run.push(line_point(v, ci, slot, plot, axis_min, axis_max, log_base, reversed));
                }
            }
            stroke_run_pattern_join(canvas, &run, stroke::Style { width: ser.line_width, cap: ser.line_cap }, ser.line_dash.as_deref(), ser.line_join, color)?;
        }
        if !matches!(ser.marker, MarkerKind::None) {
            let marker_color = ser.marker_color.unwrap_or(color);
            for (ci, value) in ser.values.iter().enumerate() {
                let plotted = if stacked {
                    stack_cumulative(&group, ci, si, percent).map(|(_, end)| end)
                } else {
                    value.filter(|n| n.is_finite())
                };
                let Some(v) = plotted else {
                    continue;
                };
                let (x, y) = line_point(v, ci, slot, plot, axis_min, axis_max, log_base, reversed);
                paint_marker(canvas, ser.marker, x, y, marker_color);
            }
        }
    }
    Ok(())
}

fn line_point(
    value: f64,
    ci: usize,
    slot: f64,
    plot: &Rect,
    axis_min: f64,
    axis_max: f64,
    log_base: Option<f64>,
    reversed: bool,
) -> (f64, f64) {
    let x = plot.x + slot * (ci as f64 + 0.5);
    let y = y_map(
        value.clamp(axis_min, axis_max),
        axis_min,
        axis_max,
        plot,
        log_base,
        reversed,
    );
    (x, y)
}

fn stroke_run(canvas: &mut Canvas, run: &[(f64, f64)], width: f64, color: [u8; 3]) {
    if let Some(style) = stroke::Style::new(width, stroke::Cap::Round) {
        stroke_run_style(canvas, run, style, color);
    }
}

const MAX_CUSTOM_DASH_PIECES: usize = 65_536; // Resource bound, not a visual period.
fn stroke_run_pattern(canvas: &mut Canvas, points: &[(f64,f64)], style: stroke::Style,
    pattern: Option<&stroke::DashPattern>, color: [u8;3]) -> Result<(),Error> {
    stroke_run_pattern_join(canvas,points,style,pattern,stroke::Join::Round,color)
}
fn stroke_run_pattern_join(canvas: &mut Canvas, points: &[(f64,f64)], style: stroke::Style,
    pattern: Option<&stroke::DashPattern>, join: stroke::Join, color: [u8;3]) -> Result<(),Error> {
    if let Some(pattern) = pattern {
        if let Some(run) = stroke::Run::new(points) {
            let paths=run.dashed_capped(pattern,style,canvas.custom_dash_budget)
                .ok_or_else(|| Error::with_path(ErrorKind::ChartImageUnsupported,"id"))?;
            canvas.custom_dash_budget -= paths.len();
            for mut piece in paths {
                // Do not brighten an otherwise identical straight dash merely
                // because a category/cache point split it into two edges.
                stroke::simplify_collinear(&mut piece.points);
                // No internal join exists on one edge or a zero-body dot. Keep
                // the locked endpoint caps and single-circle primitive.
                if join==stroke::Join::Round || piece.points.len()<=2 {stroke_run_dash_caps(canvas,&piece.points,style.width,piece.first,piece.last,color);}
                else {canvas.stroke_custom_dash_union_join(&piece.points,style.width,piece.first,piece.last,join,color);}
            }
        }
    } else { stroke_run_style(canvas,points,style,color); }
    Ok(())
}

// Only the new custom-dash path uses independently resolved dash/run caps.
// The accepted solid rasterizer and default cap behavior stay byte-for-byte.
fn stroke_run_dash_caps(canvas: &mut Canvas, points: &[(f64,f64)], width: f64,
    first: stroke::Cap, last: stroke::Cap, color: [u8;3]) {
    // Only an explicit zero-body ROUND dash is a point. Do not change legacy
    // solid degenerate runs or turn a closed polyline into a dot.
    if points.len()==2 && points[0]==points[1]
        && first==stroke::Cap::Round && last==stroke::Cap::Round {
        canvas.stroke_segment(points[0].0,points[0].1,points[0].0,points[0].1,width,color);
        return;
    }
    if points.len()==2 {
        canvas.stroke_segment_capped(points[0],points[1],width,first,last,color);
    } else {
        canvas.stroke_custom_dash_union(points,width,first,last,color);
    }
}

fn stroke_run_style(canvas: &mut Canvas, points: &[(f64, f64)], style: stroke::Style, color: [u8; 3]) {
    if let Some(run) = stroke::Run::new(points) {
        let endpoints = if style.cap == stroke::Cap::Round { Some((0,run.last_segment())) } else {
            let mut nonzero=run.segments().filter(|segment| segment.length > 0.0);
            nonzero.next().map(|first| (first.index,nonzero.last().map_or(first.index,|last| last.index)))
        };
        let Some((first_index,last_index))=endpoints else { return; };
        for segment in run.segments() {
            if style.cap != stroke::Cap::Round && segment.length == 0.0 { continue; }
            // Only true run endpoints receive caps; internal joins retain the
            // existing round-join baseline. Arc coordinates never reset here.
            let first = if segment.index == first_index { style.cap } else { stroke::Cap::Round };
            let last = if segment.index == last_index { style.cap } else { stroke::Cap::Round };
            canvas.stroke_segment_capped(segment.from, segment.to, style.width, first, last, color);
        }
    } else if style.cap == stroke::Cap::Round {
        // Preserve the old valid-segment behavior for a nonfinite legacy run.
        for pair in points.windows(2) {
            canvas.stroke_segment(pair[0].0, pair[0].1, pair[1].0, pair[1].1, style.width, color);
        }
    }
}

fn draw_areas(
    canvas: &mut Canvas,
    chart: &ColumnChart,
    plot: &Rect,
    axis_min: f64,
    axis_max: f64,
    origin: f64,
    reversed: bool,
) {
    let cats = chart.categories.len().max(1) as f64;
    let slot = (plot.w + 1.0) / cats;
    if slot <= 0.0 {
        return;
    }
    let zero_y = y_map(origin.clamp(axis_min, axis_max), axis_min, axis_max, plot, None, reversed);
    if chart.stacking != Stacking::Clustered {
        draw_stacked_areas(canvas, chart, plot, axis_min, axis_max, reversed);
        return;
    }
    for ser in &chart.series {
        let fill = ser.color.unwrap_or(chart.accents[ser.slot % 6]);
        let stroke = ser.line_color.unwrap_or(fill);
        let mut run = Vec::new();
        for (ci, value) in ser.values.iter().enumerate() {
            match value {
                Some(v) if v.is_finite() => {
                    run.push(line_point(*v, ci, slot, plot, axis_min, axis_max, None, reversed));
                }
                None if chart.area_missing_as_zero => {
                    run.push(line_point(0.0, ci, slot, plot, axis_min, axis_max, None, reversed));
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

fn draw_stacked_areas(
    canvas: &mut Canvas,
    chart: &ColumnChart,
    plot: &Rect,
    axis_min: f64,
    axis_max: f64,
    reversed: bool,
) {
    let cats = chart.categories.len().max(1) as f64;
    let percent = chart.stacking == Stacking::PercentStacked;
    let group: Vec<&Series> = chart
        .series
        .iter()
        .filter(|ser| !ser.values.is_empty())
        .collect();
    for (si, ser) in group.iter().enumerate() {
        let fill = ser.color.unwrap_or(chart.accents[ser.slot % 6]);
        let stroke = ser.line_color.unwrap_or(fill);
        let mut tops = Vec::new();
        let mut bases = Vec::new();
        for ci in 0..ser.values.len() {
            let Some((base, end)) = stack_cumulative(&group, ci, si, percent) else {
                fill_area_bands(canvas, &tops, &bases, fill);
                if ser.line_visible && ser.line_width > 0.0 {
                    stroke_run(canvas, &tops, ser.line_width, stroke);
                }
                tops.clear();
                bases.clear();
                continue;
            };
            let x = category_x_midcat(plot, ci, cats);
            let y_end = y_map(
                end.clamp(axis_min, axis_max),
                axis_min,
                axis_max,
                plot,
                None,
                reversed,
            );
            let y_base = y_map(
                base.clamp(axis_min, axis_max),
                axis_min,
                axis_max,
                plot,
                None,
                reversed,
            );
            tops.push((x, y_end));
            bases.push((x, y_base));
        }
        fill_area_bands(canvas, &tops, &bases, fill);
        if ser.line_visible && ser.line_width > 0.0 {
            stroke_run(canvas, &tops, ser.line_width, stroke);
        }
    }
}

fn category_x_midcat(plot: &Rect, ci: usize, cats: f64) -> f64 {
    if cats <= 1.0 {
        plot.x + plot.w / 2.0
    } else {
        plot.x + plot.w * (ci as f64) / (cats - 1.0)
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
    fill_area_band(canvas, start, (start.0, zero_y), end, (end.0, zero_y), color);
}

fn fill_area_bands(
    canvas: &mut Canvas,
    tops: &[(f64, f64)],
    bases: &[(f64, f64)],
    color: [u8; 3],
) {
    let n = tops.len().min(bases.len());
    for i in 1..n {
        fill_area_band(canvas, tops[i - 1], bases[i - 1], tops[i], bases[i], color);
    }
}

/// Fill the band between the previous cumulative polyline and the current one.
/// Official mixed stacked areas can have those polylines cross; split there
/// so the fill does not depend on a self-intersecting quad.
fn fill_area_band(
    canvas: &mut Canvas,
    top0: (f64, f64),
    base0: (f64, f64),
    top1: (f64, f64),
    base1: (f64, f64),
    color: [u8; 3],
) {
    let (x0, y0) = top0;
    let (_, b0) = base0;
    let (x1, y1) = top1;
    let (_, b1) = base1;
    if ![x0, y0, b0, x1, y1, b1].iter().all(|v| v.is_finite()) {
        return;
    }
    if (y0 - b0).abs() < 1e-12 && (y1 - b1).abs() < 1e-12 {
        return;
    }
    let s0 = y0 - b0;
    let s1 = y1 - b1;
    if s0 * s1 < 0.0 {
        let t = s0 / (s0 - s1);
        let xc = x0 + t * (x1 - x0);
        let yc = y0 + t * (y1 - y0);
        canvas.fill_polygon(&[(x0, b0), (x0, y0), (xc, yc)], color);
        canvas.fill_polygon(&[(xc, yc), (x1, y1), (x1, b1)], color);
    } else {
        canvas.fill_polygon(&[(x0, b0), (x0, y0), (x1, y1), (x1, b1)], color);
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

// Excel's line sample is a 20pt segment, not the square marker box.
// Public 10/14/20pt legend-font probes at 432/600pt chart widths keep the
// same 32px ink span at 106dpi (20pt centreline plus the 2.25pt round caps).
// LegendKey.Width reports the marker box and is not the segment extent.
fn legend_key_width(kind: PlotKind, square: i32, dpi: RenderDpi) -> i32 {
    if matches!(kind, PlotKind::StandardLine | PlotKind::Scatter { .. }) {
        dpi.pt_to_px_font(20.0).round() as i32
    } else {
        square
    }
}

fn legend_key_gap(kind: PlotKind, dpi: RenderDpi) -> i32 {
    if matches!(kind, PlotKind::StandardLine | PlotKind::Scatter { .. }) {
        dpi.pt_to_px_font(1.5).round() as i32
    } else {
        dpi.scale_px(LEGEND_GAP)
    }
}

#[cfg(test)]
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
    draw_plot_key_styled(canvas,kind,x0,y0,x1,y1,color,marker,None,None).unwrap();
}

fn draw_plot_key_styled(canvas: &mut Canvas, kind: PlotKind, x0:i32,y0:i32,x1:i32,y1:i32,color:[u8;3],marker:MarkerKind,style:Option<stroke::Style>,pattern:Option<&stroke::DashPattern>) -> Result<(),Error> {
    match kind {
        PlotKind::ClusteredColumn
        | PlotKind::ClusteredBar
        | PlotKind::StandardArea
        | PlotKind::Pie => canvas.fill_rect(x0, y0, x1, y1, color),
        PlotKind::StandardLine | PlotKind::Scatter { connect: true } => {
            let mid_y = (f64::from(y0) + f64::from(y1)) / 2.0;
            let line_end = f64::from(x0) + canvas.dpi.pt_to_px_font(20.0);
            let mid_x = (f64::from(x0) + line_end) / 2.0;
            if let Some(style)=style {
                stroke_run_pattern(canvas,&[(f64::from(x0),mid_y),(line_end,mid_y)],style,pattern,color)?;
            } else {
            canvas.stroke_segment(
                f64::from(x0),
                mid_y,
                line_end,
                mid_y,
                excel_chart_line_width_px(canvas.dpi.pt_to_px_font(2.25)),
                color,
            );
            }
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
    Ok(())
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

/// Office inversion is a white negative fill with an automatic black 0.75 pt
/// outline, NOT a solid accent and NOT transparent. Public Point.Fill/Line
/// properties and toggled Series.InvertIfNegative exports lock this default.
/// The 0.75 pt hairline snaps to a 1 device-pixel solid stroke on the integer
/// edges; it is not a centered anti-aliased strip of that width.
fn draw_bar_value(
    canvas: &mut Canvas,
    series: &Series,
    index: usize,
    value: f64,
    bounds: [i32; 4],
    color: [u8; 3],
) {
    let [x0, y0, x1, y1] = bounds;
    let invert = series
        .point_inversion
        .get(index)
        .copied()
        .flatten()
        .unwrap_or(series.invert_if_negative);
    if value >= 0.0 || !invert {
        canvas.fill_rect(x0, y0, x1, y1, color);
        return;
    }
    canvas.fill_rect(x0, y0, x1, y1, [255; 3]);
    let (visible, width, stroke) = series
        .negative_outline
        .as_ref()
        .map_or((true, canvas.dpi.pt_to_px_font(0.75), INK), |line| {
            (line.visible, line.width, line.color.unwrap_or(INK))
        });
    if visible && width > 0.0 {
        // Official automatic 0.75 pt outline is a 1 device-pixel solid black
        // hairline on the integer edges: left on the value tick, right on the
        // zero axis for a negative bar. Centered stroke_segment along those
        // same integers at ~1.10 px (0.75 pt at 106 dpi) covers both adjacent
        // pixel centers at ~55% and paints a 2 px #727272 double line with no
        // black core; overlapping end caps also dirty the corners. Do not thin
        // the 0.75 pt source width to fake a 1 px AA stroke; snap to the grid.
        canvas.hline_i(x0, x1, y0, stroke);
        canvas.hline_i(x0, x1, y1, stroke);
        canvas.vline_i(x0, y0, y1, stroke);
        canvas.vline_i(x1, y0, y1, stroke);
    }
}


/// Official Excel 16 stacked / percentStacked: positives accumulate upward
/// (or right) from zero in series order; negatives accumulate the other way
/// from zero, independently. Omitted or non-finite points are skipped and
/// later series pack onto the running total. percentStacked uses value/sum
/// of all (required positive finite) points in the category.
fn stack_segment(group: &[&Series], ci: usize, si: usize, percent: bool) -> Option<(f64, f64)> {
    if si >= group.len() {
        return None;
    }
    let mut denom = 0.0;
    if percent {
        for ser in group {
            let v = ser.values.get(ci).copied().flatten()?;
            if !v.is_finite() || v <= 0.0 {
                return None;
            }
            denom += v;
        }
        if denom <= 0.0 {
            return None;
        }
    }
    let mut pos = 0.0;
    let mut neg = 0.0;
    for (i, ser) in group.iter().enumerate() {
        let Some(v) = ser.values.get(ci).copied().flatten() else {
            continue;
        };
        if !v.is_finite() {
            continue;
        }
        let v = if percent { v / denom } else { v };
        let (base, end) = if v >= 0.0 {
            let b = pos;
            pos += v;
            (b, pos)
        } else {
            let b = neg;
            neg += v;
            (b, neg)
        };
        if i == si {
            if (end - base).abs() < 1e-12 {
                return None;
            }
            return Some((base, end));
        }
    }
    None
}

/// Official Excel 16 lineChart / areaChart stacked and percentStacked use an
/// algebraic running total in series order (F23). This is not the F22 bar
/// rule that stacks positives and negatives independently from zero.
fn stack_cumulative(
    group: &[&Series],
    ci: usize,
    si: usize,
    percent: bool,
) -> Option<(f64, f64)> {
    if si >= group.len() {
        return None;
    }
    let mut denom = 0.0;
    if percent {
        for ser in group {
            let v = ser.values.get(ci).copied().flatten()?;
            if !v.is_finite() || v <= 0.0 {
                return None;
            }
            denom += v;
        }
        if denom <= 0.0 {
            return None;
        }
    }
    let mut acc = 0.0;
    for (i, ser) in group.iter().enumerate() {
        let v = ser.values.get(ci).copied().flatten()?;
        if !v.is_finite() {
            return None;
        }
        let v = if percent { v / denom } else { v };
        let base = acc;
        acc += v;
        if i == si {
            return Some((base, acc));
        }
    }
    None
}

fn stacked_edge_y(
    plot: &Rect,
    axis_min: f64,
    axis_max: f64,
    step: f64,
    edge: f64,
    log_base: Option<f64>,
    reversed: bool,
    axis_origin: f64,
) -> i32 {
    let clipped = edge.clamp(axis_min, axis_max);
    if (edge - axis_origin).abs() <= 1e-12 || (edge < axis_origin && log_base.is_none()) {
        crossing_axis_y_mode(
            plot,
            axis_min,
            axis_max,
            step,
            clipped,
            if (edge - axis_origin).abs() <= 1e-12 {
                log_base
            } else {
                None
            },
            reversed,
        )
    } else {
        y_map(clipped, axis_min, axis_max, plot, log_base, reversed).round() as i32
    }
}

fn draw_clustered_bars(
    canvas: &mut Canvas,
    chart: &ColumnChart,
    plot: &Rect,
    axis_min: f64,
    axis_max: f64,
    reversed: bool,
) {
    let cats = chart.categories.len().max(1) as f64;
    let stacked = !matches!(chart.stacking, Stacking::Clustered);
    let n = if stacked { 1.0 } else { chart.series.len().max(1) as f64 };
    let overlap = if stacked { 0.0 } else { chart.overlap / 100.0 };
    let gap = chart.gap_width / 100.0;
    let slot = (plot.h + 1.0) / cats;
    let denom = n - (n - 1.0) * overlap + gap;
    if denom <= 0.0 || slot <= 0.0 {
        return;
    }
    let bar_h = slot / denom;
    let cluster = bar_h * (n - (n - 1.0) * overlap);
    let side = (slot - cluster) / 2.0;
    let x_zero = x_of_dir(0.0_f64.clamp(axis_min, axis_max), axis_min, axis_max, plot, reversed);
    if !matches!(chart.stacking, Stacking::Clustered) {
        draw_stacked_horizontal_bars(canvas, chart, plot, axis_min, axis_max, reversed, side, slot);
        return;
    }
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
            let x_val = x_of_dir(value.clamp(axis_min, axis_max), axis_min, axis_max, plot, reversed);
            let w = (x_zero - x_val).abs();
            if w < 0.2 || bar_h < 0.2 {
                continue;
            }
            let color = if chart.vary_points {
                chart.accents[ci % 6]
            } else {
                ser.color.unwrap_or(chart.accents[ser.slot % 6])
            };
            // fill_rect 是左闭右开。数值端包含对应刻度像素，零端不越过零轴：
            // 正条为 [zero, tick]，负条为 [tick, zero)。值 8 的右缘因此落在自己的
            // 8 刻度上，不再在刻度前空出 1px 白缝。不是把宽度盲目 +1。
            let x_zero_i = x_zero.round() as i32;
            let x_val_i = x_val.round() as i32;
            let (x0, x1) = if x_val_i >= x_zero_i {
                (x_zero_i, x_val_i + 1)
            } else {
                (x_val_i, x_zero_i)
            };
            let y0 = round_half_even(y);
            let y1 = round_half_even(y + bar_h);
            draw_bar_value(canvas, ser, ci, value, [x0, y0, x1, y1], color);
        }
    }
}

/// OutsideEnd on horizontal bars follows the same category/cluster order as
/// the marks. General integer formatting, plain font and complete values only.
fn draw_horizontal_value_labels(
    canvas: &mut Canvas,
    chart: &ColumnChart,
    plot: &Rect,
    axis_min: f64,
    axis_max: f64,
    font: Option<&Font>,
    gdi: Option<&super::chart_gdi::Face>,
    px: f32,
    reversed: bool,
) -> Result<(), Error> {
    let n = chart.series.len().max(1);
    let overlap = chart.overlap / 100.0;
    let slot = (plot.h + 1.0) / chart.categories.len().max(1) as f64;
    let bar_h = slot / (n as f64 - (n - 1) as f64 * overlap + chart.gap_width / 100.0);
    let side = (slot - bar_h * (n as f64 - (n - 1) as f64 * overlap)) / 2.0;
    let pad = canvas.dpi.pt_to_px_font(6.0);
    for (si, ser) in chart.series.iter().enumerate() {
        if ser.value_label != Some(ValueLabel::OutsideEnd) {
            continue;
        }
        let place = if chart.cats_from_bottom {
            n - 1 - si
        } else {
            si
        };
        for (ci, value) in ser.values.iter().enumerate() {
            if ser.deleted_label_points.binary_search(&ci).is_ok() {
                continue;
            }
            let Some(value) = value else {
                continue;
            };
            let Some(text) = value_label_text(*value) else {
                continue;
            };
            let end = x_of_dir(value.clamp(axis_min, axis_max), axis_min, axis_max, plot, reversed);
            let width = text_width(font, gdi, &text, px);
            let height = value_label_height(font, gdi, px);
            let x = if *value < 0.0 {
                end - pad - width
            } else {
                end + pad
            };
            let top =
                category_slot_origin(plot, ci, chart.categories.len(), chart.cats_from_bottom)
                    + side
                    + place as f64 * bar_h * (1.0 - overlap)
                    + (bar_h - height) / 2.0;
            if !x.is_finite()
                || !top.is_finite()
                || x < 0.0
                || top < 0.0
                || x + width > f64::from(canvas.w)
                || top + height > f64::from(canvas.h)
            {
                return Err(Error::with_path(ErrorKind::ChartImageUnsupported, "id"));
            }
            draw_text(canvas, font, gdi, &text, x, top, px, INK);
        }
    }
    Ok(())
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

/// Labels share the exact category slot / clustered-bar geometry with the marks.
/// This deliberately supports only plain General showVal: unsupported label
/// semantics are rejected during parsing, never reported as successful omission.
fn draw_value_labels(
    canvas: &mut Canvas,
    chart: &ColumnChart,
    plot: &Rect,
    axis_min: f64,
    axis_max: f64,
    font: Option<&Font>,
    gdi: Option<&super::chart_gdi::Face>,
    px: f32,
) -> Result<(), Error> {
    let slot = (plot.w + 1.0) / chart.categories.len().max(1) as f64;
    let n = chart.series.len().max(1) as f64;
    let overlap = chart.overlap / 100.0;
    let width = slot / (n - (n - 1.0) * overlap + chart.gap_width / 100.0);
    let side = (slot - width * (n - (n - 1.0) * overlap)) / 2.0;
    // Public Excel label boxes have 3pt mark-to-box gap and 3pt text inset.
    // The marker's actual requested size, not a fixed pixel offset, is excluded.
    let pad = canvas.dpi.pt_to_px_font(6.0);
    for (si, ser) in chart.series.iter().enumerate() {
        let Some(position) = ser.value_label else {
            continue;
        };
        for (ci, value) in ser.values.iter().enumerate() {
            if ser.deleted_label_points.binary_search(&ci).is_ok() {
                continue;
            }
            let Some(value) = value.filter(|n| n.is_finite()) else {
                continue;
            };
            let Some(text) = value_label_text(value) else {
                continue;
            };
            let w = text_width(font, gdi, &text, px);
            let h = value_label_height(font, gdi, px);
            let y = y_of(value.clamp(axis_min, axis_max), axis_min, axis_max, plot);
            let (x, top) = match position {
                ValueLabel::OutsideEnd | ValueLabel::InsideEnd => {
                    let x = plot.x
                        + slot * ci as f64
                        + side
                        + si as f64 * width * (1.0 - overlap)
                        + width / 2.0;
                    let top = if position == ValueLabel::InsideEnd {
                        // Public inEnd box is 3pt from the mark end; its text
                        // has the same 3pt inset as the locked outEnd labels.
                        let zero =
                            y_of(0.0_f64.clamp(axis_min, axis_max), axis_min, axis_max, plot);
                        // Short/zero marks and box overflow are not auto-fitted
                        // or silently clipped; their placement remains NI.
                        if value == 0.0 || (zero - y).abs() < h + 2.0 * pad || width < w + pad {
                            return Err(Error::with_path(ErrorKind::ChartImageUnsupported, "id"));
                        }
                        if value < 0.0 {
                            y - h - pad
                        } else {
                            y + pad
                        }
                    } else if value < 0.0 {
                        y + pad
                    } else {
                        y - h - pad
                    };
                    (x - w / 2.0, top)
                }
                ValueLabel::Right => (
                    plot.x
                        + slot * (ci as f64 + 0.5)
                        + pad
                        + match ser.marker {
                            MarkerKind::None => 0.0,
                            MarkerKind::Shaped { size, .. } => size / 2.0,
                        },
                    y - h / 2.0,
                ),
            };
            if position == ValueLabel::InsideEnd
                && (!x.is_finite()
                    || !top.is_finite()
                    || x < 0.0
                    || top < 0.0
                    || x + w > f64::from(canvas.w)
                    || top + h > f64::from(canvas.h))
            {
                return Err(Error::with_path(ErrorKind::ChartImageUnsupported, "id"));
            }
            draw_text(canvas, font, gdi, &text, x, top, px, INK);
        }
    }
    Ok(())
}

fn column_bar_bounds(
    x0: f64,
    x1: f64,
    y_zero: i32,
    y_val: i32,
    value: f64,
    origin: f64,
    invert: bool,
) -> [i32; 4] {
    if value < 0.0 && invert {
        // A 1px sharp outline chooses the pixel center nearest each continuous
        // path edge (ties toward the positive side): floor(x) + 0.5. Both path
        // edges are inclusive; a fill's rounded half-open end minus 1 is not
        // an outline edge and can inset both sides of the same narrow column.
        [
            x0.floor() as i32,
            y_zero.min(y_val),
            x1.floor() as i32,
            y_zero.max(y_val),
        ]
    } else {
        let x0 = round_half_even(x0);
        let x1_open = round_half_even(x1);
        // Inclusive edge at the crossing belongs to the visually upper
        // bar; the lower bar starts on the next row. Using y_val vs y_zero
        // (not value vs origin) keeps this true when orientation=maxMin
        // flips the growth direction.
        let _ = origin;
        if y_val <= y_zero {
            [x0, y_val, x1_open, y_zero]
        } else {
            [x0, y_zero + 1, x1_open, y_val]
        }
    }
}

fn draw_bars(
    canvas: &mut Canvas,
    chart: &ColumnChart,
    plot: &Rect,
    axis_min: f64,
    axis_max: f64,
    step: f64,
    log_base: Option<f64>,
    axis_origin: f64,
    reversed: bool,
    group_secondary: Option<bool>,
) {
    let group: Vec<&Series> = chart
        .series
        .iter()
        .filter(|ser| group_secondary.map(|flag| ser.on_secondary == flag).unwrap_or(true))
        .collect();
    if group.is_empty() {
        return;
    }
    let cats = chart.categories.len().max(1) as f64;
    let stacked = !matches!(chart.stacking, Stacking::Clustered);
    let n = if stacked { 1.0 } else { group.len().max(1) as f64 };
    let overlap = if stacked { 0.0 } else { chart.overlap / 100.0 };
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
    // Use the same major-grid raster as the axis. Independent y_of(0).round()
    // lands one row below the 0-grid on mixed-sign half-pixel spans (e.g.
    // [-3, 5] at 106dpi), so the inverted "A" box missed the axis by 1px.
    let y_zero = crossing_axis_y_mode(
        plot,
        axis_min,
        axis_max,
        step,
        axis_origin.clamp(axis_min, axis_max),
        log_base,
        reversed,
    );
    if !matches!(chart.stacking, Stacking::Clustered) {
        draw_stacked_columns(
            canvas,
            chart,
            &group,
            plot,
            axis_min,
            axis_max,
            step,
            log_base,
            axis_origin,
            reversed,
            side,
            slot,
            y_zero,
        );
        return;
    }
    for (ci, _) in chart.categories.iter().enumerate() {
        let origin = plot.x + slot * ci as f64 + side;
        for (si, ser) in group.iter().enumerate() {
            let Some(value) = ser.values[ci] else {
                continue;
            };
            if !value.is_finite() {
                continue;
            }
            let x = origin + si as f64 * bar_w * (1.0 - overlap);
            let clipped = value.clamp(axis_min, axis_max);
            let y_val = if value < axis_origin && log_base.is_none() {
                crossing_axis_y_mode(plot, axis_min, axis_max, step, clipped, None, reversed)
            } else {
                y_map(clipped, axis_min, axis_max, plot, log_base, reversed).round() as i32
            };
            if y_val == y_zero || bar_w < 0.2 {
                continue;
            }
            let color = if chart.vary_points {
                chart.accents[ci % 6]
            } else {
                ser.color.unwrap_or(chart.accents[ser.slot % 6])
            };
            let invert = ser
                .point_inversion
                .get(ci)
                .copied()
                .flatten()
                .unwrap_or(ser.invert_if_negative);
            let bounds = column_bar_bounds(x, x + bar_w, y_zero, y_val, value, axis_origin, invert);
            // Inclusive outline edges may coincide for a visible 1px column;
            // coincident half-open fill edges are empty.
            if bounds[2] < bounds[0]
                || bounds[3] <= bounds[1]
                || (bounds[2] == bounds[0] && !(value < 0.0 && invert))
            {
                continue;
            }
            draw_bar_value(canvas, ser, ci, value, bounds, color);
        }
    }
}

fn draw_stacked_columns(
    canvas: &mut Canvas,
    chart: &ColumnChart,
    group: &[&Series],
    plot: &Rect,
    axis_min: f64,
    axis_max: f64,
    step: f64,
    log_base: Option<f64>,
    axis_origin: f64,
    reversed: bool,
    side: f64,
    slot: f64,
    y_zero: i32,
) {
    let percent = matches!(chart.stacking, Stacking::PercentStacked);
    let bar_w = slot / (1.0 + chart.gap_width / 100.0);
    let _ = y_zero;
    for (ci, _) in chart.categories.iter().enumerate() {
        let x = plot.x + slot * ci as f64 + side;
        for (si, ser) in group.iter().enumerate() {
            let Some((base, end)) = stack_segment(group, ci, si, percent) else {
                continue;
            };
            if bar_w < 0.2 {
                continue;
            }
            let y_base = stacked_edge_y(
                plot, axis_min, axis_max, step, base, log_base, reversed, axis_origin,
            );
            let y_end = stacked_edge_y(
                plot, axis_min, axis_max, step, end, log_base, reversed, axis_origin,
            );
            if y_base == y_end {
                continue;
            }
            let color = if chart.vary_points {
                chart.accents[ci % 6]
            } else {
                ser.color.unwrap_or(chart.accents[ser.slot % 6])
            };
            let value = end - base;
            let bounds = column_bar_bounds(x, x + bar_w, y_base, y_end, value, axis_origin, false);
            if bounds[2] < bounds[0]
                || bounds[3] <= bounds[1]
                || (bounds[2] == bounds[0] && !(value < 0.0))
            {
                continue;
            }
            let raw = ser.values[ci].unwrap_or(value);
            draw_bar_value(canvas, ser, ci, raw, bounds, color);
        }
    }
}

fn draw_stacked_horizontal_bars(
    canvas: &mut Canvas,
    chart: &ColumnChart,
    plot: &Rect,
    axis_min: f64,
    axis_max: f64,
    reversed: bool,
    side: f64,
    slot: f64,
) {
    let percent = matches!(chart.stacking, Stacking::PercentStacked);
    let bar_h = slot / (1.0 + chart.gap_width / 100.0);
    let group: Vec<&Series> = chart.series.iter().collect();
    for (ci, _) in chart.categories.iter().enumerate() {
        let origin =
            category_slot_origin(plot, ci, chart.categories.len(), chart.cats_from_bottom) + side;
        for (si, ser) in group.iter().enumerate() {
            let Some((base, end)) = stack_segment(&group, ci, si, percent) else {
                continue;
            };
            if bar_h < 0.2 {
                continue;
            }
            let x_base = x_of_dir(base.clamp(axis_min, axis_max), axis_min, axis_max, plot, reversed);
            let x_end = x_of_dir(end.clamp(axis_min, axis_max), axis_min, axis_max, plot, reversed);
            let w = (x_end - x_base).abs();
            if w < 0.2 {
                continue;
            }
            let color = if chart.vary_points {
                chart.accents[ci % 6]
            } else {
                ser.color.unwrap_or(chart.accents[ser.slot % 6])
            };
            let x_base_i = x_base.round() as i32;
            let x_end_i = x_end.round() as i32;
            let (x0, x1) = if x_end_i >= x_base_i {
                (x_base_i, x_end_i + 1)
            } else {
                (x_end_i, x_base_i)
            };
            let y0 = round_half_even(origin);
            let y1 = round_half_even(origin + bar_h);
            let raw = ser.values[ci].unwrap_or(end - base);
            draw_bar_value(canvas, ser, ci, raw, [x0, y0, x1, y1], color);
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
    log_base: Option<f64>,
    labels_above: bool,
    reversed: bool,
    draw_value: bool,
) {
    if draw_value {
    for (tick, label) in ticks.iter().zip(labels) {
        let y = y_map(*tick, axis_min, axis_max, plot, log_base, reversed);
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
    }
    let cats = categories.len().max(1) as f64;
    let slot = (plot.w + 1.0) / cats;
    let gap = canvas.dpi.scale_f(10.0);
    let baseline = if labels_above {
        f64::from(category_y) - gap - text_height(font, px)
    } else {
        f64::from(category_y) + gap
    };
    for (i, label) in categories.iter().enumerate() {
        let w = text_width(font, gdi, label, px);
        let x = plot.x + slot * (i as f64 + 0.5) - w / 2.0;
        draw_text(canvas, font, gdi, label, x, baseline, px, INK);
    }
}

fn draw_right_tick_labels(
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
    log_base: Option<f64>,
    reversed: bool,
) {
    for (tick, label) in ticks.iter().zip(labels) {
        let y = y_map(*tick, axis_min, axis_max, plot, log_base, reversed);
        let h = text_height(font, px);
        draw_text(
            canvas,
            font,
            gdi,
            label,
            f64::from(axis_x) + f64::from(canvas.dpi.scale_px(TICK_GAP)),
            y - h / 2.0,
            px,
            INK,
        );
    }
}

fn draw_legend(
    canvas: &mut Canvas,
    chart: &ColumnChart,
    font: Option<&Font>,
    gdi: Option<&super::chart_gdi::Face>,
    px: f32,
    plot_r: i32,
) -> Result<(),Error> {
    if chart.legend != Some(LegendPos::Right) {
        return Ok(());
    }
    let key_width = legend_key_width(chart.kind, canvas.dpi.scale_px(LEGEND_SWATCH), canvas.dpi);
    let key_gap = legend_key_gap(chart.kind, canvas.dpi);
    let pitch = canvas.dpi.scale_px(24);
    let n = chart.series.len() as i32;
    if n == 0 {
        return Ok(());
    }
    let block = pitch * (n - 1) + canvas.dpi.scale_px(LEGEND_SWATCH);
    let title_band = if chart.title.is_some() {
        canvas.dpi.scale_px(LEGEND_TITLE_BAND)
    } else {
        0
    };
    let mut y = (title_band + canvas.h as i32 - block) / 2;
    let x = plot_r + canvas.dpi.scale_px(LEGEND_PLOT_GAP);
    // Official Excel 16.0 keeps series-index order for maxMin bars (S1 above
    // S2) but flips the default right legend when catAx is minMax, so the
    // keys follow the visual stack (S2 above S1). Package legend/series XML
    // is otherwise identical. Other chart kinds keep series-index order.
    let legend_series: Vec<(usize, &Series)> =
        if chart.kind == PlotKind::ClusteredBar && chart.cats_from_bottom {
            chart.series.iter().enumerate().rev().collect()
        } else {
            chart.series.iter().enumerate().collect()
        };
    for (index, ser) in legend_series {
        let color = ser.color.unwrap_or(chart.accents[ser.slot % 6]);
        draw_plot_key_styled(
            canvas,
            chart.kind,
            x,
            y,
            x + key_width,
            y + canvas.dpi.scale_px(LEGEND_SWATCH),
            color,
            ser.marker,
            if ser.line_visible { stroke::Style::new(ser.line_width, ser.line_cap) } else { None },
            ser.line_dash.as_deref(),
        )?;
        draw_text(
            canvas,
            font,
            gdi,
            &ser.name,
            f64::from(x + key_width + key_gap),
            f64::from(y - canvas.dpi.scale_px(2)),
            px,
            chart.legend_spec.entry_text_color(index),
        );
        y += pitch;
    }
    Ok(())
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
    let square = dpi.scale_px(LEGEND_SWATCH);
    let key_width = legend_key_width(chart.kind, square, dpi);
    let need = dpi.scale_px(LEGEND_PLOT_GAP)
        + key_width
        + legend_key_gap(chart.kind, dpi)
        + name_px
        + dpi.scale_px(LEGEND_PAD);
    let reference = gdi
        .map(|f| f.layout_width("S1"))
        .unwrap_or_else(|| text_width(font, None, "S1", px));
    let short_reduction = (reference - widest).max(0.0) as i32;
    need.max(dpi.scale_px(INSET_RIGHT) + key_width - square - short_reduction)
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
        // Decimal stepping can leave a tiny nonzero residue at 0. Rendering
        // that as scientific notation creates an enormous, spurious label
        // gutter (e.g. -0.3..0.5 by 0.1). This is scale-relative, so genuine
        // small axes still retain their scientific labels.
        out.push(if v.abs() <= step.abs() * 1e-10 {
            0.0
        } else {
            v
        });
        v += step;
    }
    if out.is_empty() {
        out.push(min);
    }
    out
}

fn y_of(value: f64, axis_min: f64, axis_max: f64, plot: &Rect) -> f64 {
    y_map(value, axis_min, axis_max, plot, None, false)
}

fn y_map(
    value: f64,
    axis_min: f64,
    axis_max: f64,
    plot: &Rect,
    log_base: Option<f64>,
    reversed: bool,
) -> f64 {
    let mut t = if let Some(base) = log_base {
        let lo = axis_min.log(base);
        let hi = axis_max.log(base);
        let v = value.max(axis_min).log(base);
        (v - lo) / (hi - lo).max(1e-12)
    } else {
        let span = (axis_max - axis_min).max(1e-12);
        (value - axis_min) / span
    };
    if reversed {
        t = 1.0 - t;
    }
    plot.y + plot.h - t * plot.h
}

fn tick_values_log(min: f64, max: f64, base: f64) -> Vec<f64> {
    let mut out = Vec::new();
    let mut v = min;
    for _ in 0..64 {
        if v > max * (1.0 + 1e-9) {
            break;
        }
        out.push(v);
        v *= base;
        if !v.is_finite() {
            break;
        }
    }
    if out.is_empty() {
        out.push(min);
    }
    out
}

#[cfg(test)]
fn fmt_tick(value: f64) -> String {
    fmt_tick_with(value, axis::TickFormat::General)
}

fn fmt_tick_with(value: f64, format: axis::TickFormat) -> String {
    if !value.is_finite() {
        return String::new();
    }
    match format {
        axis::TickFormat::General => fmt_tick_general(value),
        axis::TickFormat::Integer => fmt_fixed_int(value, false),
        axis::TickFormat::Fixed2 => fmt_fixed(value, 2, false),
        axis::TickFormat::Percent => fmt_fixed_int(value * 100.0, true),
        axis::TickFormat::PercentFixed2 => fmt_fixed(value * 100.0, 2, true),
        axis::TickFormat::Thousands => fmt_thousands(value),
        axis::TickFormat::Scientific2 => fmt_scientific2(value),
    }
}

fn fmt_tick_general(value: f64) -> String {
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

fn fmt_fixed_int(value: f64, percent: bool) -> String {
    let n = value.round();
    let n = if n == 0.0 { 0_i64 } else { n as i64 };
    if percent {
        format!("{n}%")
    } else {
        format!("{n}")
    }
}

fn fmt_fixed(value: f64, places: u32, percent: bool) -> String {
    let factor = 10f64.powi(places as i32);
    let mut n = (value * factor).round();
    if n == 0.0 {
        n = 0.0;
    }
    let sign = if n < 0.0 { "-" } else { "" };
    let abs = n.abs() as u64;
    let div = 10u64.pow(places);
    let whole = abs / div;
    let frac = abs % div;
    let width = places as usize;
    let body = format!("{sign}{whole}.{frac:0width$}");
    if percent {
        format!("{body}%")
    } else {
        body
    }
}

fn fmt_thousands(value: f64) -> String {
    let n = value.round();
    let n = if n == 0.0 { 0_i64 } else { n as i64 };
    let sign = if n < 0 { "-" } else { "" };
    let digits = n.unsigned_abs().to_string();
    let mut grouped = String::new();
    for (i, ch) in digits.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(ch);
    }
    let grouped: String = grouped.chars().rev().collect();
    format!("{sign}{grouped}")
}

fn fmt_scientific2(value: f64) -> String {
    if value == 0.0 {
        return "0.00E+00".to_owned();
    }
    let sign = if value < 0.0 { "-" } else { "" };
    let abs = value.abs();
    let mut exp = abs.log10().floor() as i32;
    let mant = abs / 10f64.powi(exp);
    let mut scaled = (mant * 100.0).round();
    if scaled >= 1000.0 {
        scaled = 100.0;
        exp += 1;
    }
    if scaled == 0.0 {
        return format!("{sign}0.00E+00");
    }
    let whole = (scaled as u64) / 100;
    let frac = (scaled as u64) % 100;
    format!("{sign}{whole}.{frac:02}E{exp:+03}")
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
        serde_json::Value::String(s) => parse_chart_number(&s),
        _ => None,
    }
}

/// Excel charts treat an empty string (empty `inlineStr`, empty cached `<c:v>`) as 0
/// and keep the stroke connected. A missing cell, an omitted cache point, or an
/// unparsable value such as `#N/A` stays `None` and breaks the stroke.
fn parse_chart_number(text: &str) -> Option<f64> {
    let text = text.trim();
    if text.is_empty() {
        Some(0.0)
    } else {
        text.parse().ok().filter(|n: &f64| n.is_finite())
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

/// Preserved inherited chart pen / legacy scatter normalization. Native
/// explicit width controls have a distinct nearest-pixel rule (chart_stroke).
/// In particular omitted w remains the accepted 2.25pt/default 4px at 106dpi;
/// an explicit 2.25pt resolves to 3px. Do not globally replace this old rule.
fn excel_chart_line_width_px(px: f64) -> f64 {
    if !px.is_finite() || px <= 0.0 {
        return px;
    }
    let rounded = px.round();
    if (px - rounded).abs() <= 1e-6 {
        rounded
    } else {
        px.ceil()
    }
}

fn read_line_series(
    ser: &Element,
    accents: [[u8; 3]; 6],
    dpi: RenderDpi,
    slot: usize,
    explicit_pens: bool,
) -> Result<LineSeriesStyle, Error> {
    let unsupported = || Error::with_path(ErrorKind::ChartImageUnsupported, "id");
    if flag(ser, "smooth").unwrap_or(false) {
        return Err(unsupported());
    }
    for name in ["dPt", "trendline", "errBars"] {
        if ser.child_by_local(name).is_some() {
            return Err(unsupported());
        }
    }
    let mut dash = None;
    let mut cap = stroke::Cap::Round;
    let mut join = stroke::Join::Round;
    let mut visible = true;
    let mut width = excel_chart_line_width_px(dpi.pt_to_px_font(2.25));
    let mut color = None;
    if let Some(sp) = ser.child_by_local("spPr") {
        let style = read_line_sp_pr(sp, accents, dpi, explicit_pens)?;
        dash = style.dash;
        cap = style.cap;
        join = style.join;
        visible = style.visible;
        width = excel_chart_line_width_px(style.width);
        color = style.color;
    }
    if color.is_none() {
        color = series_color(ser, accents);
    }
    let (marker, marker_color) = read_line_marker(ser, accents, dpi, slot)?;
    Ok(LineSeriesStyle {
        dash,
        cap,
        join,
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
    explicit_pens: bool,
) -> Result<LineSeriesStyle, Error> {
    let unsupported = || Error::with_path(ErrorKind::ChartImageUnsupported, "id");
    let mut dash = None;
    let mut cap = stroke::Cap::Round;
    let mut join = stroke::Join::Round;
    let mut visible = true;
    let mut width = excel_chart_line_width_px(dpi.pt_to_px_font(2.25));
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
                let line = read_ln_with_caps(child, accents, width, dpi, explicit_pens)?;
                dash = line.dash;
                cap = line.cap;
                join = line.join;
                visible = line.visible;
                width = excel_chart_line_width_px(line.width);
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
        dash,
        cap,
        join,
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
    read_ln_with_caps(ln, accents, default_width, dpi, false)
}

fn read_ln_with_caps(ln: &Element, accents: [[u8;3];6], default_width: f64, dpi: RenderDpi, explicit_caps: bool) -> Result<LineSeriesStyle, Error> {
    let unsupported = || Error::with_path(ErrorKind::ChartImageUnsupported, "id");
    // Omitted cap deliberately retains the accepted chart baseline, not a new
    // guessed global default. Only explicit cap values are expanded here.
    let cap = match ln.attr_local("cap") { Some(v) => stroke::Cap::from_ooxml(v).ok_or_else(unsupported)?, None => stroke::Cap::Round };
    if !explicit_caps && cap != stroke::Cap::Round { return Err(unsupported()); }
    for (attr, allowed) in [
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
    let mut width = ln
        .attr_local("w")
        .and_then(|raw| raw.parse::<f64>().ok())
        .map(|emu| dpi.emu_to_px_f(emu))
        .unwrap_or(default_width);
    if !width.is_finite() || width < 0.0 {
        return Err(unsupported());
    }
    if explicit_caps && ln.attr_local("w").is_some() {
        width = stroke::explicit_pen_width_px(width).ok_or_else(unsupported)?;
    }
    let mut join = stroke::Join::Round;
    let mut dash = None;
    let mut dash_declared = false;
    let mut visible = true;
    let mut color = None;
    for child in ln.element_children() {
        match child.local_name() {
            "noFill" => visible = false,
            "solidFill" => {
                if cap != stroke::Cap::Round && child.element_children().any(|node| node.element_children().next().is_some()) {
                    // New cap coverage is an opaque-color baseline, not permission
                    // to flatten alpha or invent additional color transforms.
                    return Err(unsupported());
                }
                color = Some(color_of_slot(child, &accents).ok_or_else(unsupported)?);
            }
            "custDash" => {
                // Explicit opaque custom dashes: flat and square positive stops;
                // round includes its caps and clamps short positive d to a dot.
                // Omitted caps and zero dash/space remain separately gated.
                if dash_declared || !explicit_caps || ln.attr_local("cap").is_none()
                    || child.attrs.iter().any(|(key,_)| !key.starts_with("xmlns"))
                    || ln.attr_local("w").and_then(|v| v.parse::<f64>().ok())
                        .is_none_or(|w| !w.is_finite() || w <= 0.0)
                { return Err(unsupported()); }
                let mut stops = Vec::new();
                for stop in child.element_children() {
                    if stop.local_name() != "ds" || stop.element_children().next().is_some()
                        || stop.attrs.iter().any(|(k,_)| !matches!(k.as_str(), "d"|"sp") && !k.starts_with("xmlns"))
                    { return Err(unsupported()); }
                    let percentage = |key| stop.attr_local(key).and_then(|v| v.parse::<i32>().ok())
                        .filter(|v| *v > 0).map(f64::from).ok_or_else(unsupported);
                    let d=percentage("d")?;
                    stops.push((d,percentage("sp")?));
                }
                dash = Some(std::sync::Arc::new(stroke::DashPattern::from_width_percent(&stops,width).ok_or_else(unsupported)?));
                dash_declared = true;
            }
            "prstDash" => {
                if dash_declared { return Err(unsupported()); }
                dash_declared = true;
                let dash = child.attr_local("val").unwrap_or("solid");
                if dash != "solid" {
                    return Err(unsupported());
                }
            }
            "round" => {},
            "bevel" if explicit_caps => {},
            "miter" | "bevel" if cap == stroke::Cap::Round => {},
            "effectLst" => {
                if child.element_children().next().is_some() {
                    return Err(unsupported());
                }
            }
            _ => return Err(unsupported()),
        }
    }
    if dash.is_some() && (!visible || ln.element_children().any(|child|
        matches!(child.local_name(),"miter") ||
        (child.local_name()=="solidFill" && child.element_children()
            .any(|color| color.element_children().next().is_some())))) {
        return Err(unsupported());
    }
    let joins:Vec<_>=ln.element_children().filter(|c|matches!(c.local_name(),"round"|"bevel"|"miter")).collect();
    if dash.is_some() {
        if joins.len()>1 || joins.iter().any(|c|c.local_name()=="miter" || c.element_children().next().is_some() || c.attrs.iter().any(|(k,_)|!k.starts_with("xmlns"))) {return Err(unsupported());}
        if joins.first().is_some_and(|c|c.local_name()=="bevel") {join=stroke::Join::Bevel;}
    } else if cap!=stroke::Cap::Round && joins.iter().any(|c|c.local_name()=="bevel") {
        // Do not silently widen legacy solid-cap handling.
        return Err(unsupported());
    }
    if !visible {
        color = None;
    }
    Ok(LineSeriesStyle {
        dash,
        cap,
        join,
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
    // Series explosion is applied in draw_pie_slices after the category
    // domain is known. A non-zero value is accepted here and validated
    // together with dPt explosion in read_pie_point_colors.
    if let Some(explosion) = number_val(ser, "explosion") {
        if !explosion.is_finite() || explosion < 0.0 {
            return Err(unsupported());
        }
    }
    if let Some(sp) = ser.child_by_local("spPr") {
        let _ = read_pie_sp_pr(sp, accents, dpi)?;
    }
    Ok(LineSeriesStyle {
        join: stroke::Join::Round,
        dash: None,
                cap: stroke::Cap::Round,
        color: series_color(ser, accents),
        visible: false,
        width: 0.0,
        marker: MarkerKind::None,
        marker_color: None,
    })
}

/// Official Excel 16 hidden-fill memory on a pie noFill. The remembered
/// solid is not painted; the visible fill is noFill.
const PIE_HIDDEN_FILL_URI: &str = "{909E8E84-426E-40DD-AFC4-6F175D3DCCD1}";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SliceFill {
    color: [u8; 3],
    alpha: u8,
}

fn pie_color_node(slot: &Element) -> Option<&Element> {
    slot.child_by_local("srgbClr")
        .or_else(|| slot.child_by_local("sysClr"))
        .or_else(|| slot.child_by_local("schemeClr"))
        .or_else(|| {
            if matches!(slot.local_name(), "srgbClr" | "sysClr" | "schemeClr") {
                Some(slot)
            } else {
                None
            }
        })
}

fn pie_solid_fill(slot: &Element, accents: &[[u8; 3]; 6]) -> Result<SliceFill, Error> {
    let unsupported = || Error::with_path(ErrorKind::ChartImageUnsupported, "id");
    let color = color_of_slot(slot, accents).ok_or_else(unsupported)?;
    let Some(node) = pie_color_node(slot) else {
        return Err(unsupported());
    };
    let mut alpha = 255u8;
    let mut saw_alpha = false;
    for child in node.element_children() {
        match child.local_name() {
            "alpha" => {
                if saw_alpha {
                    return Err(unsupported());
                }
                saw_alpha = true;
                let raw = child.attr_local("val").ok_or_else(unsupported)?;
                let val = raw
                    .parse::<f64>()
                    .ok()
                    .filter(|v| v.is_finite() && *v >= 0.0 && *v <= 100_000.0)
                    .ok_or_else(unsupported)?;
                alpha = ooxml_alpha_to_u8(val.round() as u32);
            }
            "alphaMod" | "alphaOff" => return Err(unsupported()),
            "lumMod" | "lumOff" | "shade" | "tint" => {
                // Opaque F07 transforms already resolve through color_of_slot.
                // Official pie evidence did not lock those transforms with a
                // non-opaque alpha, so that combination stays NI.
                if saw_alpha || color_has_non_opaque_alpha(slot) {
                    return Err(unsupported());
                }
            }
            _ => {}
        }
    }
    if alpha < 255 && node.local_name() != "srgbClr" {
        return Err(unsupported());
    }
    Ok(SliceFill { color, alpha })
}

fn pie_hidden_fill_memory(ext_lst: &Element) -> bool {
    let mut count = 0;
    for ext in ext_lst.element_children() {
        if ext.local_name() != "ext" {
            return false;
        }
        count += 1;
        let uri = ext.attr_local("uri").unwrap_or("");
        if uri != PIE_HIDDEN_FILL_URI {
            return false;
        }
        let mut kids = ext.element_children();
        let Some(hidden) = kids.next() else {
            return false;
        };
        if kids.next().is_some() || hidden.local_name() != "hiddenFill" {
            return false;
        }
    }
    count == 1
}

fn read_pie_sp_pr(
    sp: &Element,
    accents: [[u8; 3]; 6],
    dpi: RenderDpi,
) -> Result<Option<SliceFill>, Error> {
    let unsupported = || Error::with_path(ErrorKind::ChartImageUnsupported, "id");
    let mut fill = None;
    let mut no_fill = false;
    for child in sp.element_children() {
        match child.local_name() {
            "solidFill" => {
                if no_fill || fill.is_some() {
                    return Err(unsupported());
                }
                fill = Some(pie_solid_fill(child, &accents)?);
            }
            "noFill" => {
                if no_fill || fill.is_some() {
                    return Err(unsupported());
                }
                no_fill = true;
                fill = Some(SliceFill {
                    color: [0, 0, 0],
                    alpha: 0,
                });
            }
            "gradFill" | "pattFill" | "blipFill" => return Err(unsupported()),
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
            "extLst" => {
                if !no_fill || !pie_hidden_fill_memory(child) {
                    return Err(unsupported());
                }
            }
            _ => return Err(unsupported()),
        }
    }
    Ok(fill)
}

fn read_pie_point_colors(
    ser: &Element,
    accents: [[u8; 3]; 6],
    n: usize,
    dpi: RenderDpi,
) -> Result<
    (
        Vec<Option<[u8; 3]>>,
        f64,
        Vec<f64>,
        Option<SliceFill>,
        Vec<Option<SliceFill>>,
    ),
    Error,
> {
    let unsupported = || Error::with_path(ErrorKind::ChartImageUnsupported, "id");
    let pie_explosion = match number_val(ser, "explosion") {
        Some(v) if v.is_finite() && v >= 0.0 => v,
        None => 0.0,
        _ => return Err(unsupported()),
    };
    let series_fill = match ser.child_by_local("spPr") {
        Some(sp) => read_pie_sp_pr(sp, accents, dpi)?,
        None => None,
    };
    let mut colors = vec![None; n];
    let mut point_fills = vec![None; n];
    let mut point_explosions = vec![0.0; n];
    for dpt in ser.element_children().filter(|el| el.local_name() == "dPt") {
        let idx = number_val(dpt, "idx").ok_or_else(unsupported)?;
        if !idx.is_finite() || idx < 0.0 || idx.fract() != 0.0 {
            return Err(unsupported());
        }
        let idx = idx as usize;
        if idx >= n
            || colors[idx].is_some()
            || point_fills[idx].is_some()
            || point_explosions[idx] != 0.0
        {
            return Err(unsupported());
        }
        if let Some(explosion) = number_val(dpt, "explosion") {
            if !explosion.is_finite() || explosion < 0.0 {
                return Err(unsupported());
            }
            // Official point explosion and series explosion were not written
            // together; that combination stays NI.
            if pie_explosion != 0.0 && explosion != 0.0 {
                return Err(unsupported());
            }
            point_explosions[idx] = explosion;
        }
        if let Some(bubble) = dpt.child_by_local("bubble3D") {
            // Official exploded dPt writes bubble3D=0 on a 2D pie; treat that
            // as a no-op. Other values stay NI.
            let val = bubble
                .attr_local("val")
                .unwrap_or("1")
                .parse::<f64>()
                .ok()
                .filter(|v| v.is_finite())
                .ok_or_else(unsupported)?;
            if val != 0.0 {
                return Err(unsupported());
            }
        }
        for name in ["invertIfNegative", "marker", "pictureOptions"] {
            if dpt.child_by_local(name).is_some() {
                return Err(unsupported());
            }
        }
        if let Some(sp) = dpt.child_by_local("spPr") {
            if let Some(fill) = read_pie_sp_pr(sp, accents, dpi)? {
                point_fills[idx] = Some(fill);
                if fill.alpha > 0 {
                    colors[idx] = Some(fill.color);
                }
            }
        }
    }
    Ok((colors, pie_explosion, point_explosions, series_fill, point_fills))
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
    points_to_values(&read_pts(cache), parse_chart_number)
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

/// Use the renderer that will actually draw the label. On Windows chart text
/// is GDI; fontdue's fallback line leading is not its line box and would shift
/// outside-end labels upward, even when the visible glyph itself is correct.
fn value_label_height(font: Option<&Font>, gdi: Option<&super::chart_gdi::Face>, px: f32) -> f64 {
    gdi.map_or_else(|| text_height(font, px), |face| face.em_box())
}

/// Minimum locked General subset: ordinary integers (no scientific notation,
/// locale-dependent separators or decimal precision approximation).
fn value_label_text(value: f64) -> Option<String> {
    if !value.is_finite() || value.fract() != 0.0 || value.abs() >= 100_000_000_000.0 {
        return None;
    }
    Some(if value == 0.0 {
        "0".to_owned()
    } else {
        format!("{value:.0}")
    })
}

/// Only deletion-only series point labels are locked. Known Excel editing
/// identity extensions carry no paint semantics; unknown extensions stay NI.
fn read_deleted_label_points(labels: Option<&Element>, len: usize) -> Result<Vec<usize>, Error> {
    let unsupported = || Error::with_path(ErrorKind::ChartImageUnsupported, "id");
    let Some(labels) = labels else {
        return Ok(Vec::new());
    };
    let scalar = |e: &Element| {
        e.element_children().next().is_none()
            && e.attrs
                .iter()
                .filter(|(k, _)| !k.starts_with("xmlns"))
                .all(|(k, _)| k == "val")
            && e.attrs.iter().filter(|(k, _)| k == "val").count() <= 1
    };
    let mut deleted = Vec::new();
    for point in labels
        .element_children()
        .filter(|e| e.local_name() == "dLbl")
    {
        if point.attrs.iter().any(|(k, _)| !k.starts_with("xmlns")) {
            return Err(unsupported());
        }
        let mut idx = None;
        let mut delete_seen = false;
        let mut ext_seen = false;
        for child in point.element_children() {
            match child.local_name() {
                "idx" if idx.is_none() && scalar(child) => {
                    let raw = child.attr_local("val").ok_or_else(unsupported)?;
                    if raw.is_empty() || !raw.bytes().all(|b| b.is_ascii_digit()) {
                        return Err(unsupported());
                    }
                    idx = Some(raw.parse::<usize>().map_err(|_| unsupported())?);
                }
                "delete"
                    if !delete_seen
                        && scalar(child)
                        && matches!(child.attr_local("val"), None | Some("1" | "true")) =>
                {
                    delete_seen = true;
                }
                "extLst" if !ext_seen => {
                    ext_seen = true;
                    for ext in child.element_children() {
                        if ext.local_name() != "ext" {
                            return Err(unsupported());
                        }
                        let mut settings = ext.element_children();
                        let inert = match ext.attr_local("uri") {
                            Some("{CE6537A1-D6FC-4f65-9D91-7224C49458BB}") => {
                                settings.next().is_none()
                            }
                            Some("{C3380CC4-5D6E-409C-BE32-E72D297353CC}") => {
                                settings.next().is_some_and(|e| {
                                    e.local_name() == "uniqueId"
                                        && scalar(e)
                                        && e.attr_local("val").is_some()
                                }) && settings.next().is_none()
                            }
                            _ => false,
                        };
                        if !inert {
                            return Err(unsupported());
                        }
                    }
                }
                _ => return Err(unsupported()),
            }
        }
        let idx = idx.ok_or_else(unsupported)?;
        if !delete_seen || idx >= len {
            return Err(unsupported());
        }
        deleted.push(idx);
    }
    // Keep input-sized work O(n log n), including duplicate validation and
    // point lookup during painting; do not add a quadratic resource path.
    deleted.sort_unstable();
    if deleted.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(unsupported());
    }
    Ok(deleted)
}

/// Only these non-bubble chart kinds reach this parser. Excel writes
/// showBubbleSize=1 on ordinary line/column wrappers; it has no visible meaning
/// there. Real label flags, custom formatting and visible point overrides remain guarded.
fn read_value_label(
    wrapper: &Element,
    ser: &Element,
    kind: PlotKind,
) -> Result<Option<ValueLabel>, Error> {
    let unsupported = || Error::with_path(ErrorKind::ChartImageUnsupported, "id");
    let mut visible = false;
    let mut position = if kind == PlotKind::StandardLine {
        "r"
    } else {
        "outEnd"
    };
    let configs: Vec<_> = [
        (false, wrapper.child_by_local("dLbls")),
        (true, ser.child_by_local("dLbls")),
    ]
    .into_iter()
    .filter_map(|(series_level, labels)| labels.map(|labels| (series_level, labels)))
    .collect();
    for (series_level, labels) in &configs {
        if flag(labels, "delete").unwrap_or(false) {
            visible = false;
            continue;
        }
        if labels.element_children().any(|e| e.local_name() == "dLbl") {
            if !series_level {
                return Err(unsupported());
            }
            // Shape is checked here even for this parser's direct callers;
            // read_chart additionally checks indices against real values.
            read_deleted_label_points(Some(labels), usize::MAX)?;
        }
        for name in [
            "showLegendKey",
            "showCatName",
            "showSerName",
            "showPercent",
            "showLeaderLines",
        ] {
            if flag(labels, name).unwrap_or(false) {
                return Err(unsupported());
            }
        }
        if let Some(v) = flag(labels, "showVal") {
            visible = v;
        }
        if let Some(p) = labels
            .child_by_local("dLblPos")
            .and_then(|e| e.attr_local("val"))
        {
            position = p;
        }
    }
    if !visible {
        return Ok(None);
    }
    for (_, labels) in configs {
        // Hidden label styling cannot affect output. Visible custom styling is
        // not approximated by the default font/background.

        for child in labels.element_children() {
            match child.local_name() {
                "spPr" => {
                    for style in child.element_children() {
                        match style.local_name() {
                            "noFill" => {}
                            "ln" if style
                                .element_children()
                                .all(|e| e.local_name() == "noFill") => {}
                            "effectLst" if style.element_children().next().is_none() => {}
                            _ => return Err(unsupported()),
                        }
                    }
                }
                "numFmt" if child.attr_local("formatCode").unwrap_or("General") != "General" => {
                    return Err(unsupported())
                }
                "txPr" | "layout" => return Err(unsupported()),
                "extLst" => {
                    for ext in child.element_children() {
                        for setting in ext.element_children() {
                            if setting.local_name() != "showLeaderLines"
                                || !matches!(setting.attr_local("val"), Some("0" | "false"))
                            {
                                return Err(unsupported());
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
    if let Some(cache) = ser
        .child_by_local("val")
        .and_then(|v| find_deep(v, "numCache").or_else(|| find_deep(v, "numLit")))
    {
        if cache
            .child_by_local("formatCode")
            .is_some_and(|e| e.text_content() != "General")
        {
            return Err(unsupported());
        }
    }
    match (kind, position) {
        (PlotKind::ClusteredColumn | PlotKind::ClusteredBar, "outEnd") => {
            Ok(Some(ValueLabel::OutsideEnd))
        }
        (PlotKind::ClusteredColumn, "inEnd") => Ok(Some(ValueLabel::InsideEnd)),
        (PlotKind::StandardLine, "r") => Ok(Some(ValueLabel::Right)),
        _ => Err(unsupported()),
    }
}

/// Excel16 public probes show #N/A is not a zero point and the line connects
/// across it (including zero/span); an actual blank follows dispBlanksAs.
/// Other explicit non-numeric errors remain distinguishable and are not silently
/// treated as blanks when opting into the newly supported zero/span modes.
fn series_error_points(
    parent: Option<&Element>,
    sheet: &str,
    cells: Option<&super::sparkline::CellCache>,
    len: usize,
) -> (Vec<bool>, Vec<bool>) {
    let mut errors = vec![false; len];
    let mut unavailable = vec![false; len];
    let cache = parent
        .and_then(|p| find_deep(p, "numCache").or_else(|| find_deep(p, "numLit")))
        .filter(|_| !cache_numbers(parent).is_empty());
    if let Some(cache) = cache {
        for (idx, text) in read_pts(cache) {
            if let Some(error) = errors.get_mut(idx) {
                *error = parse_chart_number(text.trim()).is_none();
                unavailable[idx] = text.trim() == "#N/A";
            }
        }
    } else if let Some(values) = cells.and_then(|cells| {
        formula_text(parent).and_then(|f| super::sparkline::range_values(&f, sheet, cells))
    }) {
        for (idx, value) in values.into_iter().take(len).enumerate() {
            unavailable[idx] = value.as_str() == Some("#N/A");
            errors[idx] = !value.is_null() && json_number(value).is_none();
        }
    }
    (errors, unavailable)
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
            rounded_frame: true,
            chart_border: true,
            custom_dash_budget: MAX_CUSTOM_DASH_PIECES,
            plot_fill: None,
            dpi,
        }
    }

    fn clear_to_transparent(&mut self) {
        self.px.fill(0);
        self.alpha = Some(vec![0; (self.w as usize) * (self.h as usize)]);
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
        let src_a = u16::from(cov);
        let dst_a = self
            .alpha
            .as_ref()
            .map(|alpha| u16::from(alpha[i / 3]))
            .unwrap_or(255);
        if dst_a == 255 {
            // Opaque destination keeps the existing coverage blend; the
            // rounded-corner path relies on this and then overwrites outside alpha.
            for (k, src) in rgb.into_iter().enumerate() {
                let dst = u16::from(self.px[i + k]);
                self.px[i + k] = ((u16::from(src) * src_a + dst * (255 - src_a) + 127) / 255) as u8;
            }
            return;
        }
        if dst_a == 0 {
            // Unassociated source over a transparent pixel, same silhouette
            // convention as the rounded-frame coverage mask.
            self.px[i..i + 3].copy_from_slice(&rgb);
            if let Some(alpha) = &mut self.alpha {
                alpha[i / 3] = cov;
            }
            return;
        }
        let src_a32 = u32::from(src_a);
        let dst_a32 = u32::from(dst_a);
        let out_a = src_a32 + (dst_a32 * (255 - src_a32) + 127) / 255;
        if out_a == 0 {
            self.px[i..i + 3].copy_from_slice(&[0; 3]);
        } else {
            for (k, src) in rgb.into_iter().enumerate() {
                let dst = u32::from(self.px[i + k]);
                let premul =
                    u32::from(src) * src_a32 + (dst * dst_a32 * (255 - src_a32) + 127) / 255;
                self.px[i + k] = ((premul + out_a / 2) / out_a) as u8;
            }
        }
        if let Some(alpha) = &mut self.alpha {
            alpha[i / 3] = out_a as u8;
        }
    }

    fn fill_rect(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, color: [u8; 3]) {
        self.fill_rect_cov(x0, y0, x1, y1, color, 255);
    }

    fn fill_rect_cov(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, color: [u8; 3], cov: u8) {
        let x0 = x0.max(0);
        let y0 = y0.max(0);
        let x1 = x1.min(self.w as i32);
        let y1 = y1.min(self.h as i32);
        for y in y0..y1 {
            for x in x0..x1 {
                self.blend(x, y, color, cov);
            }
        }
    }

    /// Area coverage of an axis-aligned floating rectangle. Pie plot bounds
    /// are point-derived and must not be shifted/truncated to match one gold.
    fn fill_fractional_rect(&mut self, rect: &Rect, color: [u8; 3]) {
        let right = rect.x + rect.w;
        let bottom = rect.y + rect.h;
        for y in (rect.y.floor() as i32).max(0)..(bottom.ceil() as i32).min(self.h as i32) {
            let cy = (bottom.min(f64::from(y) + 1.0) - rect.y.max(f64::from(y))).clamp(0.0, 1.0);
            for x in (rect.x.floor() as i32).max(0)..(right.ceil() as i32).min(self.w as i32) {
                let cx = (right.min(f64::from(x) + 1.0) - rect.x.max(f64::from(x))).clamp(0.0, 1.0);
                self.blend(x, y, color, (cx * cy * 255.0).round() as u8);
            }
        }
    }

    fn paint_plot(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, fill: PlotPaint) {
        match fill {
            PlotPaint::Solid(color, cov) => self.fill_rect_cov(x0, y0, x1, y1, color, cov),
            PlotPaint::Linear(grad) => self.fill_linear_rect(x0, y0, x1, y1, &grad),
        }
    }

    /// Existing axis-aligned pixel-center geometry. Cache at most one clipped
    /// axis (<= canvas edge) so many-stop interval searches are not per pixel.
    /// Neither the two-stop interpolation nor border/plot geometry is changed.
    fn fill_linear_rect(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, grad: &LinearFrameGrad) {
        let x0c = x0.max(0);
        let y0c = y0.max(0);
        let x1c = x1.min(self.w as i32);
        let y1c = y1.min(self.h as i32);
        let w = f64::from(x1 - x0);
        let h = f64::from(y1 - y0);
        if w <= 0.0 || h <= 0.0 || x1c <= x0c || y1c <= y0c {
            return;
        }
        let vertical = grad.ang == LINEAR_ANG_VERTICAL;
        let colors: Vec<[u8; 3]> = if vertical {
            (y0c..y1c).map(|y| grad.sample((f64::from(y - y0) + 0.5) / h)).collect()
        } else {
            (x0c..x1c).map(|x| grad.sample((f64::from(x - x0) + 0.5) / w)).collect()
        };
        for y in y0c..y1c {
            for x in x0c..x1c {
                let index = if vertical { y - y0c } else { x - x0c } as usize;
                self.blend(x, y, colors[index], 255);
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
        alpha: u8,
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
                let cov = ((u16::from(cov) * u16::from(alpha) + 127) / 255) as u8;
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

    /// One custom dash is one painted shape, including its round internal joins.
    /// Max coverage is composited once: overlapping edges/retracing must not
    /// darken antialiased boundaries. Solid/one-edge legacy rasterizers are
    /// intentionally unchanged. The explicit custom-bevel branch below adds the
    /// offset-edge hull; miter and exact reversal remain separate boundaries.
    fn stroke_custom_dash_union(&mut self, points: &[stroke::Point], width: f64,
        first: stroke::Cap, last: stroke::Cap, color: [u8;3]) {
        self.stroke_custom_dash_union_join(points,width,first,last,stroke::Join::Round,color);
    }
    fn stroke_custom_dash_union_join(&mut self, points: &[stroke::Point], width: f64,
        first: stroke::Cap, last: stroke::Cap, join: stroke::Join, color: [u8;3]) {
        let Some(run)=stroke::Run::new(points) else {return;};
        if !width.is_finite() || width<=0.0 {return;}
        let edges:Vec<_>=run.segments().filter(|e| e.length>0.0).collect();
        let (Some(start),Some(end))=(edges.first(),edges.last()) else {return;};
        let pad=width/2.0+1.0;
        let x0=((points.iter().map(|p|p.0).fold(f64::INFINITY,f64::min)-pad).floor() as i32).max(0);
        let x1=((points.iter().map(|p|p.0).fold(f64::NEG_INFINITY,f64::max)+pad).ceil() as i32).min(self.w as i32-1);
        let y0=((points.iter().map(|p|p.1).fold(f64::INFINITY,f64::min)-pad).floor() as i32).max(0);
        let y1=((points.iter().map(|p|p.1).fold(f64::NEG_INFINITY,f64::max)+pad).ceil() as i32).min(self.h as i32-1);
        if x1<x0 || y1<y0 {return;}
        // Allocation is clipped to this piece's canvas rectangle, never to
        // off-canvas coordinates or a new unbounded world-space surface.
        let mw=(x1-x0+1) as usize;
        let mut mask=vec![0u8;mw*(y1-y0+1) as usize];
        for edge in &edges {
            let Some(g)=stroke::CapSegment::new(edge.from,edge.to,width,
                if edge.index==start.index {first}else if join==stroke::Join::Round {stroke::Cap::Round}else{stroke::Cap::Flat},
                if edge.index==end.index {last}else if join==stroke::Join::Round {stroke::Cap::Round}else{stroke::Cap::Flat}) else {continue;};
            let ax=((edge.from.0.min(edge.to.0)-pad).floor() as i32).max(x0);
            let bx=((edge.from.0.max(edge.to.0)+pad).ceil() as i32).min(x1);
            let ay=((edge.from.1.min(edge.to.1)-pad).floor() as i32).max(y0);
            let by=((edge.from.1.max(edge.to.1)+pad).ceil() as i32).min(y1);
            for y in ay..=by {for x in ax..=bx {
                let alpha=coverage(g.signed_distance((f64::from(x)+0.5,f64::from(y)+0.5)),0.0);
                let slot=&mut mask[(y-y0) as usize*mw+(x-x0) as usize];
                *slot=(*slot).max(alpha);
            }}
        }
        if join==stroke::Join::Bevel {
            for pair in edges.windows(2) {
                let Some(g)=stroke::BevelJoin::new(pair[0].from,pair[0].to,pair[1].to,width) else {continue;};
                let (low,high)=g.bounds();let ax=((low.0-1.0).floor() as i32).max(x0);let bx=((high.0+1.0).ceil() as i32).min(x1);
                let ay=((low.1-1.0).floor() as i32).max(y0);let by=((high.1+1.0).ceil() as i32).min(y1);
                for y in ay..=by {for x in ax..=bx {
                    let alpha=coverage(g.signed_distance((f64::from(x)+0.5,f64::from(y)+0.5)),0.0);
                    let slot=&mut mask[(y-y0) as usize*mw+(x-x0) as usize];*slot=(*slot).max(alpha);
                }}
            }
        }
        for y in y0..=y1 {for x in x0..=x1 {
            let alpha=mask[(y-y0) as usize*mw+(x-x0) as usize];
            if alpha>0 {self.blend(x,y,color,alpha);}
        }}
    }

    fn stroke_segment_capped(&mut self, from: stroke::Point, to: stroke::Point, width: f64, first: stroke::Cap, last: stroke::Cap, color: [u8; 3]) {
        if first == stroke::Cap::Round && last == stroke::Cap::Round {
            // Accepted round pixels go through the untouched legacy rasterizer.
            self.stroke_segment(from.0, from.1, to.0, to.1, width, color);
            return;
        }
        let Some(geometry) = stroke::CapSegment::new(from, to, width, first, last) else { return; };
        let pad = width / 2.0 + 1.0;
        let x0 = ((from.0.min(to.0)-pad).floor() as i32).max(0);
        let x1 = ((from.0.max(to.0)+pad).ceil() as i32).min(self.w as i32-1);
        let y0 = ((from.1.min(to.1)-pad).floor() as i32).max(0);
        let y1 = ((from.1.max(to.1)+pad).ceil() as i32).min(self.h as i32-1);
        for y in y0..=y1 { for x in x0..=x1 {
            let distance = geometry.signed_distance((f64::from(x)+0.5,f64::from(y)+0.5));
            let alpha = coverage(distance,0.0);
            if alpha > 0 { self.blend(x,y,color,alpha); }
        }}
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
        let r = if self.rounded_frame {
            CORNER_R.min(w / 2).min(h / 2)
        } else {
            0
        };
        if r < 2 {
            self.hline_i(0, w - 1, 0, color);
            self.hline_i(0, w - 1, h - 1, color);
            self.vline_i(0, 0, h - 1, color);
            self.vline_i(w - 1, 0, h - 1, color);
            return;
        }
        if self.alpha.is_none() {
            // Do not reset an existing noFill alpha mask to opaque white.
            self.alpha = Some(vec![255; (self.w as usize) * (self.h as usize)]);
        }
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
    use super::super::geometry::GeometryContext;
    use super::super::picture::AnchorKind;
    use super::*;

    #[test]
    fn decimal_axis_ticks_do_not_invent_a_scientific_label_at_zero() {
        let ticks = tick_values(-0.3, 0.5, 0.1);
        assert_eq!(ticks.len(), 9);
        assert_eq!(ticks[3], 0.0);
        assert_eq!(fmt_tick(ticks[3]), "0");
        assert!(ticks.iter().all(|v| !fmt_tick(*v).contains('e')));
        assert!(
            fmt_tick(1e-8).contains('e'),
            "genuinely small data is not zero"
        );
    }

    #[test]
    fn signed_crossing_uses_major_grid_rasterization_not_independent_rounding() {
        let p = Rect {
            x: 35.0,
            y: 16.0,
            w: 220.0,
            h: 188.0,
        };
        assert_eq!(crossing_axis_y(&p, -3.0, 5.0, 1.0, 0.0), 133);
        assert_eq!(crossing_axis_y(&p, -3.0, 5.0, 1.0, -1.0), 157);
        assert_eq!(crossing_axis_y(&p, -3.0, 5.0, 1.0, -2.0), 180);
        assert_eq!(crossing_axis_y(&p, -3.0, 5.0, 1.0, 4.0), 39);
        assert_eq!(crossing_axis_y(&p, -2.0, 8.0, 2.0, 0.0), 166);
        assert_eq!(crossing_axis_y(&p, 0.0, 5.0, 1.0, 0.0), 204);
        assert_eq!(crossing_axis_y(&p, -5.0, 0.0, 1.0, 0.0), 16);
        // Invert frames use inclusive edges; positive fills retain the
        // supplied shape edge rather than unconditionally clearing a grid.
        assert_eq!(
            column_bar_bounds(57.1, 86.56666666666666, 133, 180, -2.0, 0.0, true),
            [57, 133, 86, 180]
        );
        assert_eq!(
            column_bar_bounds(57.1, 86.56666666666666, 16, 53, -1.0, 0.0, true),
            [57, 16, 86, 53]
        );
        assert_eq!(
            column_bar_bounds(70.0, 107.0, 334, 51, 1.0, 0.0, true),
            [70, 51, 107, 334]
        );
    }

    #[test]
    fn one_cell_size_uses_anchor_ext_when_frame_xfrm_is_zero() {
        // Excel 16.0 Save writes graphicFrame/xfrm/ext as 0×0 while keeping the
        // real oneCell size on xdr:ext. Prefer the anchor so export still works.
        let geometry = GeometryContext::calibri11();
        let sheet = crate::xml::parse(br#"<worksheet/>"#).unwrap();
        let cases = [
            (5486400, 3200400, 0, 0, true, Ok((576, 336))),
            (5486400, 3200400, 5486400, 3200400, true, Ok((576, 336))),
            (0, 0, 5486400, 3200400, true, Ok((576, 336))),
            (5486400, 3200400, 5486400, 3200400, false, Ok((576, 336))),
            (0, 0, 0, 0, true, Err(())),
        ];
        for (anchor_cx, anchor_cy, frame_cx, frame_cy, include_anchor, expected) in cases {
            let anchor_ext = if include_anchor {
                format!(r#"<xdr:ext cx="{anchor_cx}" cy="{anchor_cy}"/>"#)
            } else {
                String::new()
            };
            let xml = format!(
                r#"<xdr:wsDr xmlns:xdr="http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><xdr:oneCellAnchor><xdr:from><xdr:col>4</xdr:col><xdr:colOff>0</xdr:colOff><xdr:row>1</xdr:row><xdr:rowOff>0</xdr:rowOff></xdr:from>{anchor_ext}<xdr:graphicFrame><xdr:nvGraphicFramePr><xdr:cNvPr id="2" name="Chart 1"/><xdr:cNvGraphicFramePr/></xdr:nvGraphicFramePr><xdr:xfrm><a:off x="0" y="0"/><a:ext cx="{frame_cx}" cy="{frame_cy}"/></xdr:xfrm></xdr:graphicFrame></xdr:oneCellAnchor></xdr:wsDr>"#
            );
            let doc = crate::xml::parse(xml.as_bytes()).unwrap();
            let anchor = doc.root.child_by_local("oneCellAnchor").unwrap();
            let got = bitmap_px(&sheet.root, anchor, AnchorKind::OneCell, &geometry);
            match expected {
                Ok(size) => assert_eq!(got.unwrap(), size, "anchor={anchor_cx}x{anchor_cy} frame={frame_cx}x{frame_cy} include={include_anchor}"),
                Err(()) => assert!(got.is_err(), "expected NI for zero extents"),
            }
        }
    }

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
    fn bar_negative_default_inversion_and_explicit_point_overrides() {
        for (extra, expected) in [
            ("", true),
            ("<c:invertIfNegative/>", true),
            ("<c:invertIfNegative val=\"1\"/>", true),
            ("<c:invertIfNegative val=\"0\"/>", false),
            ("<c:invertIfNegative val=\"false\"/>", false),
            ("<c:invertIfNegative val=\"0\"/><c:dPt><c:idx val=\"0\"/><c:invertIfNegative/></c:dPt>", true),
            ("<c:dPt><c:idx val=\"0\"/><c:invertIfNegative val=\"0\"/></c:dPt>", false),
        ] {
            let xml = bar_xml(&line_ser(0, "S1", &[Some(-8.0), Some(8.0)], extra), "", "maxMin", Some("bar"));
            let doc = crate::xml::parse(xml.as_bytes()).unwrap();
            let chart = read_chart(&doc.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT).unwrap();
            let series = &chart.series[0];
            assert_eq!(series.point_inversion.first().copied().flatten().unwrap_or(series.invert_if_negative), expected, "{extra}");
            for dpi_value in [96, 106, 144] {
                let dpi = RenderDpi::from_request(serde_json::json!({"dpi":dpi_value}).as_object().unwrap()).unwrap();
                let mut canvas = Canvas::new(80, 60); canvas.dpi = dpi;
                canvas.fill_rect(0, 0, 80, 60, [200, 100, 50]);
                draw_bar_value(&mut canvas, series, 0, -8.0, [10, 10, 60, 40], FALLBACK_ACCENTS[0]);
                let pixel = |x: usize, y: usize| {let i=(y*80+x)*3; &canvas.px[i..i+3]};
                assert_eq!(pixel(30, 25), if expected { &[255,255,255] } else { &FALLBACK_ACCENTS[0] });
                if expected {assert!(pixel(10, 25).iter().all(|v| *v < 160), "negative outline must be black");}
                // Positive values must never be inverted.
                draw_bar_value(&mut canvas, series, 0, 8.0, [10, 10, 60, 40], FALLBACK_ACCENTS[0]);
                assert_eq!(&canvas.px[(25*80+30)*3..(25*80+30)*3+3], &FALLBACK_ACCENTS[0]);
            }
        }
    }

    #[test]
    fn bar_negative_outline_is_one_pixel_solid_and_pixel_aligned() {
        // Official signed bar-2 (chart:3, A=-8) uses a 1 px solid #000000 box.
        // Mid-edge neighbors are pure fill or background, not a 2 px gray AA
        // band. Bounds are half-open fill [x0, x1) x [y0, y1); the outline sits
        // on those integer edges, so the right/bottom lines occupy the exclusive
        // fill sides and coincide with the zero axis on the value end.
        let xml = bar_xml(
            &line_ser(0, "S1", &[Some(-8.0), Some(8.0)], ""),
            "",
            "maxMin",
            Some("bar"),
        );
        let doc = crate::xml::parse(xml.as_bytes()).unwrap();
        let chart = read_chart(&doc.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT).unwrap();
        let series = &chart.series[0];
        let bg = [200, 100, 50];
        let [x0, y0, x1, y1] = [10, 10, 60, 40];
        for dpi_value in [96, 106, 144] {
            let dpi =
                RenderDpi::from_request(serde_json::json!({"dpi": dpi_value}).as_object().unwrap())
                    .unwrap();
            let mut canvas = Canvas::new(80, 60);
            canvas.dpi = dpi;
            canvas.fill_rect(0, 0, 80, 60, bg);
            draw_bar_value(
                &mut canvas,
                series,
                0,
                -8.0,
                [x0, y0, x1, y1],
                FALLBACK_ACCENTS[0],
            );
            let pixel = |x: i32, y: i32| {
                let i = (y as usize * 80 + x as usize) * 3;
                [canvas.px[i], canvas.px[i + 1], canvas.px[i + 2]]
            };
            assert_eq!(pixel(x0, 25), INK, "left mid @ {dpi_value}");
            assert_eq!(pixel(x1, 25), INK, "right mid @ {dpi_value}");
            assert_eq!(pixel(35, y0), INK, "top mid @ {dpi_value}");
            assert_eq!(pixel(35, y1), INK, "bottom mid @ {dpi_value}");
            assert_eq!(pixel(x0 - 1, 25), bg, "outside left @ {dpi_value}");
            assert_eq!(
                pixel(x0 + 1, 25),
                [255, 255, 255],
                "inside left @ {dpi_value}"
            );
            assert_eq!(
                pixel(x1 - 1, 25),
                [255, 255, 255],
                "inside right @ {dpi_value}"
            );
            assert_eq!(pixel(x1 + 1, 25), bg, "outside right @ {dpi_value}");
            assert_eq!(pixel(35, y0 - 1), bg, "outside top @ {dpi_value}");
            assert_eq!(
                pixel(35, y0 + 1),
                [255, 255, 255],
                "inside top @ {dpi_value}"
            );
            assert_eq!(
                pixel(35, y1 - 1),
                [255, 255, 255],
                "inside bottom @ {dpi_value}"
            );
            assert_eq!(pixel(35, y1 + 1), bg, "outside bottom @ {dpi_value}");
            for (x, y) in [(x0, y0), (x1, y0), (x1, y1), (x0, y1)] {
                assert_eq!(pixel(x, y), INK, "corner {x},{y} @ {dpi_value}");
            }
            assert_eq!(pixel(x0 - 1, y0 - 1), bg, "outside corner @ {dpi_value}");
        }
    }

    fn mixed_column_xml(values: &[f64]) -> String {
        let pts = values
            .iter()
            .enumerate()
            .map(|(i, _v)| {
                let name = (b'A' + i as u8) as char;
                format!("<c:pt idx=\"{i}\"><c:v>{name}</c:v></c:pt>",)
            })
            .collect::<String>();
        let nums = values
            .iter()
            .enumerate()
            .map(|(i, v)| format!("<c:pt idx=\"{i}\"><c:v>{v}</c:v></c:pt>"))
            .collect::<String>();
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
          <c:tx><c:v>S</c:v></c:tx>
          <c:cat><c:strLit><c:ptCount val="{n}"/>{pts}</c:strLit></c:cat>
          <c:val><c:numLit><c:ptCount val="{n}"/>{nums}</c:numLit></c:val>
        </c:ser>
      </c:barChart>
      <c:valAx>
        <c:scaling>
          <c:min val="-3"/>
          <c:max val="5"/>
        </c:scaling>
        <c:majorUnit val="1"/>
        <c:majorGridlines/>
        <c:crosses val="autoZero"/>
      </c:valAx>
      <c:catAx>
        <c:scaling><c:orientation val="minMax"/></c:scaling>
        <c:crosses val="autoZero"/>
      </c:catAx>
    </c:plotArea>
    <c:legend><c:legendPos val="r"/><c:overlay val="0"/></c:legend>
  </c:chart>
</c:chartSpace>"#,
            n = values.len(),
            pts = pts,
            nums = nums
        )
    }

    fn decode_rgb(png: &[u8]) -> (u32, u32, Vec<u8>) {
        let decoder = png::Decoder::new(std::io::Cursor::new(png));
        let mut reader = decoder.read_info().unwrap();
        let mut buf = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).unwrap();
        let data = &buf[..info.buffer_size()];
        if info.color_type == png::ColorType::Rgb {
            return (info.width, info.height, data.to_vec());
        }
        let rgb: Vec<u8> = data
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
        (info.width, info.height, rgb)
    }

    fn row_ink_runs(rgb: &[u8], w: u32, y: i32) -> Vec<(i32, i32)> {
        let mut runs = Vec::new();
        let mut start = None;
        for x in 0..=w as i32 {
            let ink = if x < w as i32 {
                let i = (y as usize * w as usize + x as usize) * 3;
                rgb[i] < 40 && rgb[i + 1] < 40 && rgb[i + 2] < 40
            } else {
                false
            };
            match (ink, start) {
                (true, None) => start = Some(x),
                (false, Some(s)) => {
                    runs.push((s, x - 1));
                    start = None;
                }
                _ => {}
            }
        }
        runs
    }

    fn long_ink_rows(rgb: &[u8], w: u32, h: u32, min_dark: i32) -> Vec<i32> {
        (0..h as i32)
            .filter(|&y| {
                row_ink_runs(rgb, w, y)
                    .iter()
                    .map(|(a, b)| b - a + 1)
                    .sum::<i32>()
                    >= min_dark
            })
            .collect()
    }

    #[test]
    fn inverted_columns_rasterize_both_path_edges_without_a_fill_inset() {
        let xml = mixed_column_xml(&[-1.0, -4.0, -2.0]);
        let doc = crate::xml::parse(xml.as_bytes()).unwrap();
        let mut chart =
            read_chart(&doc.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT).unwrap();
        chart.series[0].invert_if_negative = true;
        let mut canvas = Canvas::new(300, 220);
        let plot = Rect {
            x: 35.0,
            y: 16.0,
            w: 220.0,
            h: 188.0,
        };
        draw_bars(&mut canvas, &chart, &plot, -5.0, 0.0, 1.0, None, 0.0, false, None);
        // Three equal path widths are 221 / 7.5 pixels. The middle path
        // crosses columns 130..160: rounding two fill edges and subtracting
        // one instead loses both outline sides (131..159).
        assert_eq!(
            row_ink_runs(&canvas.px, canvas.w, 30),
            vec![
                (57, 57),
                (86, 86),
                (130, 130),
                (160, 160),
                (204, 204),
                (233, 233)
            ]
        );
    }

    #[test]
    fn coincident_inverted_path_edges_remain_visible_vertical_hairlines() {
        let xml = mixed_column_xml(&[-1.0, -4.0, -2.0]);
        let doc = crate::xml::parse(xml.as_bytes()).unwrap();
        let chart = read_chart(&doc.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT).unwrap();
        let mut canvas = Canvas::new(80, 220);
        // Each .4px path has both edges in one pixel. All three must remain
        // visible, even though inclusive x0 == x1 for every outline.
        let plot = Rect {
            x: 35.0,
            y: 16.0,
            w: 2.0,
            h: 188.0,
        };
        draw_bars(&mut canvas, &chart, &plot, -5.0, 0.0, 1.0, None, 0.0, false, None);
        assert_eq!(row_ink_runs(&canvas.px, canvas.w, 30), vec![(35, 37)]);
    }

    #[test]
    fn inverted_path_edges_keep_fractional_phase_and_integer_translation() {
        for x in [-20.75, -0.25, 0.0, 0.5, 3.9, 130.76666666666665] {
            for width in [0.21, 1.0, 2.5, 29.466666666666665] {
                let bounds = column_bar_bounds(x, x + width, 10, 40, -1.0, 0.0, true);
                assert_eq!(
                    bounds,
                    [x.floor() as i32, 10, (x + width).floor() as i32, 40]
                );
                for offset in [-7, -1, 1, 11] {
                    let shifted = column_bar_bounds(
                        x + f64::from(offset),
                        x + width + f64::from(offset),
                        10,
                        40,
                        -1.0,
                        0.0,
                        true,
                    );
                    assert_eq!(shifted, [bounds[0] + offset, 10, bounds[2] + offset, 40]);
                }
            }
        }
    }

    #[test]
    fn positive_column_bounds_preserve_the_supplied_shape_edge() {
        // A rasterized shape edge is not the same thing as a grid row.
        // Moving every positive endpoint by 1 also shifts non-major values.
        assert_eq!(
            column_bar_bounds(10.0, 20.0, 100, 40, 2.3, 0.0, false),
            [10, 40, 20, 100]
        );
    }

    #[test]
    fn thin_inverted_columns_keep_their_single_pixel_outline() {
        // Thin path edges can fall in the same pixel or in separate pixels.
        // Neither case is empty; a small outline need not be a solid ink run.
        let xml = mixed_column_xml(&[-2.0; 60]);
        let doc = crate::xml::parse(xml.as_bytes()).unwrap();
        let dpi =
            RenderDpi::from_request(serde_json::json!({"dpi": 106}).as_object().unwrap()).unwrap();
        // Supply the plot explicitly: this is an outline-coverage regression,
        // not a dependency on installed fonts or automatic axis label layout.
        let chart = read_chart(&doc.root, FALLBACK_ACCENTS, None, "", dpi).unwrap();
        let mut canvas = Canvas::new(314, 221);
        canvas.dpi = dpi;
        let plot = Rect {
            x: 35.0,
            y: 16.0,
            w: 220.0,
            h: 188.0,
        };
        draw_bars(&mut canvas, &chart, &plot, -3.0, 5.0, 1.0, None, 0.0, false, None);
        // Verify both path edges of all 60 columns, not a run count: an
        // unfilled 1.47px-wide path can have two separated 1px edges.
        let slot = 221.0 / 60.0;
        let width = slot / 2.5;
        let mut expected = Vec::new();
        for i in 0..60 {
            let x = 35.0 + slot * f64::from(i) + (slot - width) / 2.0;
            expected.push(x.floor() as i32);
            expected.push((x + width).floor() as i32);
        }
        expected.sort_unstable();
        expected.dedup();
        let actual: Vec<i32> = row_ink_runs(&canvas.px, canvas.w, 167)
            .into_iter()
            .flat_map(|(a, b)| a..=b)
            .collect();
        assert_eq!(actual, expected, "all 60 thin path outlines");
    }

    #[test]
    fn inverted_column_outline_sits_on_zero_axis_and_value_grid() {
        // 106dpi mixed-sign columns (official A=-2 on [-3,5]) must press the
        // zero axis and land the bottom hairline on the -2 grid, not hang a
        // 1px framed box below / beside those rows.
        let xml = mixed_column_xml(&[-2.0, 4.0, 1.0]);
        let doc = crate::xml::parse(xml.as_bytes()).unwrap();
        let dpi =
            RenderDpi::from_request(serde_json::json!({"dpi": 106}).as_object().unwrap()).unwrap();
        let png = render_chart_at(
            &doc.root,
            314,
            221,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
            dpi,
        )
        .unwrap();
        let (w, h, rgb) = decode_rgb(&png);
        assert_eq!((w, h), (314, 221));
        let grids = long_ink_rows(&rgb, w, h, 160);
        assert!(
            grids.len() >= 5,
            "expected signed major grids, got {grids:?}"
        );
        // Explicit -3..5 / major 1 yields 9 grids; 0 is the sixth, -2 the eighth, 4 the second.
        assert_eq!(grids.len(), 9, "signed major grids {grids:?}");
        let zero = grids[5];
        let neg2 = grids[7];
        let pos4 = grids[1];
        let ink = |x: i32, y: i32| {
            let i = (y as usize * w as usize + x as usize) * 3;
            rgb[i] < 40 && rgb[i + 1] < 40 && rgb[i + 2] < 40
        };
        // Side strokes are 1px verticals under the 0 axis; skip the value axis.
        let probe = zero + ((neg2 - zero) / 3).max(8);
        let sides: Vec<i32> = (50..w as i32 - 20)
            .filter(|&x| ink(x, probe) && ink(x, probe + 8))
            .collect();
        assert!(
            sides.len() >= 2,
            "expected inverted A sides, got {sides:?} grids={grids:?} zero={zero} probe={probe}"
        );
        let x0 = sides[0];
        let x1 = *sides
            .iter()
            .find(|&&x| x >= x0 + 16)
            .unwrap_or_else(|| panic!("A right edge after {x0} in {sides:?} grids={grids:?}"));
        assert!(
            x1 - x0 >= 16,
            "A box too narrow {x0}..{x1} sides={sides:?} grids={grids:?}"
        );
        let span_ink = |y: i32| (x0..=x1).filter(|&x| ink(x, y)).count();
        assert!(
            span_ink(zero) >= (x1 - x0) as usize,
            "zero axis must press the A box top; zero={zero} box={x0}..{x1} ink={}",
            span_ink(zero)
        );
        assert!(
            span_ink(zero + 1) <= 4,
            "no separate top stroke 1px below the axis; ink={} box={x0}..{x1}",
            span_ink(zero + 1)
        );
        assert!(
            span_ink(neg2) >= (x1 - x0) as usize,
            "A bottom must sit on the -2 grid; y={neg2} ink={}",
            span_ink(neg2)
        );
        assert!(
            neg2 + 1 >= h as i32 || span_ink(neg2 + 1) <= 4,
            "A box must not hang 1px past the value grid; ink={}",
            span_ink(neg2 + 1)
        );
        // Positive 4 must start 1px below its grid, not cover the line.
        let is_teal = |x: i32, y: i32| {
            let i = (y as usize * w as usize + x as usize) * 3;
            let (r, g, b) = (rgb[i], rgb[i + 1], rgb[i + 2]);
            b > 80 && u16::from(b) > u16::from(r) + 20 && g > r
        };
        let teal_on = |y: i32| (0..w as i32).filter(|&x| is_teal(x, y)).count();
        assert_eq!(
            teal_on(pos4),
            0,
            "value grid must not be covered by the bar"
        );
        assert!(
            teal_on(pos4 + 1) >= 10,
            "positive bar must start immediately under the value grid; n={}",
            teal_on(pos4 + 1)
        );
    }

    #[test]
    fn pie_auto_layout_matches_public_office_aspect_and_host_probes() {
        // Excel public PlotArea relative to its 4 pt origin, then PowerPoint.
        // Includes height-limited, width-limited, no legend and title hosts.
        let cases = [
            (
                432.0,
                252.0,
                0.0,
                29.314488188976377,
                98.09275590551181,
                22.75,
                198.5,
            ),
            (
                432.0,
                180.0,
                0.0,
                29.314488188976377,
                129.5927559055118,
                18.25,
                135.5,
            ),
            (
                300.0,
                360.0,
                0.0,
                29.314488188976377,
                23.91779527559055,
                68.57503937007874,
                214.84992125984252,
            ),
            (
                600.0,
                252.0,
                0.0,
                29.314488188976377,
                182.0927559055118,
                22.75,
                198.5,
            ),
            (
                300.0,
                300.0,
                0.0,
                29.314488188976377,
                23.91779527559055,
                38.57503937007874,
                214.84992125984252,
            ),
            (432.0, 252.0, 0.0, 0.0, 112.75, 22.75, 198.5),
            (240.0, 180.0, 0.0, 0.0, 48.25, 18.25, 135.5),
            (
                432.0,
                252.0,
                23.65,
                29.314488188976377,
                108.4396062992126,
                44.92181102362205,
                177.80637795,
            ),
            (
                432.0,
                252.0,
                26.15503937007874,
                29.134488188976377,
                109.62559055,
                47.27031496062992,
                175.61440945,
            ),
        ];
        for dpi_value in [96, 106, 144] {
            let dpi =
                RenderDpi::from_request(serde_json::json!({"dpi": dpi_value}).as_object().unwrap())
                    .unwrap();
            let px = |pt| dpi.pt_to_px_font(pt);
            for (w, h, title, legend, left, top, diameter) in cases {
                let plot = pie_plot_bounds(px(w), px(h), px(title), px(legend), dpi).unwrap();
                // COM reports geometry to 1/12700 pt. No image-specific offset.
                assert!((plot.x - px(left + 4.0)).abs() < px(0.001));
                assert!((plot.y - px(top + 4.0)).abs() < px(0.001));
                assert!((plot.w - px(diameter)).abs() < px(0.001));
                assert_eq!(plot.w, plot.h);
            }
            assert!(pie_plot_bounds(px(20.0), px(20.0), 0.0, 0.0, dpi).is_none());
        }
    }

    #[test]
    fn valid_unsupported_backgrounds_are_not_silently_defaulted() {
        for config in ["<c:spPr><a:ln w='25400'/></c:spPr>"] {
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
            "<c:roundedCorners val='0'/>",
            "<c:spPr><a:solidFill><a:srgbClr val='123456'/></a:solidFill></c:spPr>",
            "<c:roundedCorners val='1'/>",
            "<c:spPr><a:solidFill><a:srgbClr val='FFFFFF'/></a:solidFill></c:spPr>",
            "<c:spPr><a:noFill/></c:spPr>",
            "<c:spPr><a:solidFill><a:srgbClr val='EAF7F7'><a:alpha val='50000'/></a:srgbClr></a:solidFill></c:spPr>",
            "<c:spPr><a:solidFill><a:srgbClr val='EAF7F7'><a:alpha val='25000'/></a:srgbClr></a:solidFill></c:spPr>",
            "<c:spPr><a:solidFill><a:srgbClr val='EAF7F7'><a:alpha val='100000'/></a:srgbClr></a:solidFill></c:spPr>",
            "<c:spPr><a:gradFill flip='none' rotWithShape='1'><a:gsLst><a:gs pos='0'><a:srgbClr val='EAF7F7'/></a:gs><a:gs pos='100000'><a:srgbClr val='0F6082'/></a:gs></a:gsLst><a:lin ang='0' scaled='1'/><a:tileRect/></a:gradFill></c:spPr>",
            "<c:spPr><a:gradFill flip='none' rotWithShape='1'><a:gsLst><a:gs pos='0'><a:srgbClr val='EAF7F7'/></a:gs><a:gs pos='100000'><a:srgbClr val='0F6082'/></a:gs></a:gsLst><a:lin ang='5400000' scaled='1'/><a:tileRect/></a:gradFill></c:spPr>",
            "<c:spPr><a:solidFill><a:schemeClr val='accent1'/></a:solidFill></c:spPr>",
            "<c:spPr><a:solidFill><a:srgbClr val='156082'><a:tint val='60000'/></a:srgbClr></a:solidFill></c:spPr>",
            "<c:spPr><a:solidFill><a:srgbClr val='156082'><a:shade val='75000'/></a:srgbClr></a:solidFill></c:spPr>",
            "<c:spPr><a:solidFill><a:srgbClr val='156082'><a:lumMod val='75000'/></a:srgbClr></a:solidFill></c:spPr>",
            "<c:spPr><a:solidFill><a:srgbClr val='156082'><a:lumMod val='60000'/><a:lumOff val='40000'/></a:srgbClr></a:solidFill></c:spPr>",
            "<c:spPr><a:solidFill><a:schemeClr val='accent1'><a:tint val='60000'/></a:schemeClr></a:solidFill></c:spPr>",
            "<c:spPr><a:solidFill><a:schemeClr val='accent1'><a:alpha val='50000'/></a:schemeClr></a:solidFill></c:spPr>",
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
    fn rectangular_frame_and_chart_plot_fills_have_real_pixels() {
        for dpi_value in [96, 106, 144] {
            let dpi =
                RenderDpi::from_request(serde_json::json!({"dpi": dpi_value}).as_object().unwrap())
                    .unwrap();
            let xml = LONG_LEGEND.replace("<c:chart>", "<c:roundedCorners val='0'/><c:spPr><a:solidFill><a:srgbClr val='EAF7F7'/></a:solidFill></c:spPr><c:chart>")
                .replace("<c:plotArea>", "<c:plotArea><c:spPr><a:solidFill><a:srgbClr val='FFF0DD'/></a:solidFill></c:spPr>");
            let doc = crate::xml::parse(xml.as_bytes()).unwrap();
            let frame = read_frame(&doc.root).unwrap();
            assert!(!frame.rounded);
            assert_eq!(frame.fill, FrameFill::Solid([234, 247, 247]));
            assert_eq!(frame.plot_fill, FrameFill::Solid([255, 240, 221]));
            let png = render_chart_at(
                &doc.root,
                576,
                336,
                FALLBACK_ACCENTS,
                None,
                ChartHost::Excel,
                None,
                "",
                dpi,
            )
            .unwrap();
            let mut reader = png::Decoder::new(std::io::Cursor::new(png))
                .read_info()
                .unwrap();
            let mut buf = vec![0; reader.output_buffer_size()];
            let info = reader.next_frame(&mut buf).unwrap();
            assert_eq!(
                info.color_type,
                png::ColorType::Rgb,
                "rectangle is opaque; no rounded-corner mask"
            );
            let px = |x: usize, y: usize| &buf[(y * 576 + x) * 3..(y * 576 + x) * 3 + 3];
            assert_eq!(
                px(5, 5),
                &[234, 247, 247],
                "chart-area fill must not stay white"
            );
            assert_eq!(
                px(0, 5),
                &BORDER,
                "straight edge must reach the corner band"
            );
            assert!(
                buf[..info.buffer_size()]
                    .as_chunks::<3>()
                    .0
                    .contains(&[255, 240, 221]),
                "plot-area fill must be independently painted"
            );
            assert!(
                buf[..info.buffer_size()]
                    .as_chunks::<3>()
                    .0
                    .contains(&FALLBACK_ACCENTS[0]),
                "data must still be drawn"
            );
        }
    }

    #[test]
    fn chart_and_plot_nofill_are_not_silently_white() {
        let hidden = "<c:spPr><a:noFill/><a:extLst><a:ext uri=\"{909E8E84-426E-40DD-AFC4-6F175D3DCCD1}\"><a14:hiddenFill xmlns:a14=\"http://schemas.microsoft.com/office/drawing/2010/main\"><a:solidFill><a:sysClr val=\"window\" lastClr=\"FFFFFF\"/></a:solidFill></a14:hiddenFill></a:ext></a:extLst></c:spPr>";
        for chart_sppr in ["<c:spPr><a:noFill/></c:spPr>", hidden] {
            let xml = LONG_LEGEND.replace(
                "<c:chart>",
                &format!("<c:roundedCorners val='0'/>{chart_sppr}<c:chart>"),
            );
            let doc = crate::xml::parse(xml.as_bytes()).unwrap();
            let frame = read_frame(&doc.root).unwrap();
            assert_eq!(frame.fill, FrameFill::NoFill, "{chart_sppr}");
            assert_eq!(frame.plot_fill, FrameFill::Default);
            let png = super::render_chart(
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
            let mut reader = png::Decoder::new(std::io::Cursor::new(png))
                .read_info()
                .unwrap();
            let mut buf = vec![0; reader.output_buffer_size()];
            let info = reader.next_frame(&mut buf).unwrap();
            assert_eq!(info.color_type, png::ColorType::Rgba);
            let i = (5 * 576 + 5) * 4;
            assert_eq!(
                buf[i + 3],
                0,
                "chart-area noFill must be transparent, not white"
            );
            assert_ne!(&buf[i..i + 3], &[255, 255, 255]);
        }

        let xml = LONG_LEGEND
            .replace(
                "<c:chart>",
                "<c:roundedCorners val='0'/><c:spPr><a:solidFill><a:srgbClr val='EAF7F7'/></a:solidFill></c:spPr><c:chart>",
            )
            .replace(
                "<c:plotArea>",
                "<c:plotArea><c:spPr><a:noFill/></c:spPr>",
            );
        let doc = crate::xml::parse(xml.as_bytes()).unwrap();
        let frame = read_frame(&doc.root).unwrap();
        assert_eq!(frame.fill, FrameFill::Solid([234, 247, 247]));
        assert_eq!(frame.plot_fill, FrameFill::NoFill);
        let png = super::render_chart(
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
        let mut reader = png::Decoder::new(std::io::Cursor::new(png))
            .read_info()
            .unwrap();
        let mut buf = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).unwrap();
        assert_eq!(
            info.color_type,
            png::ColorType::Rgb,
            "opaque chart fill with plot noFill stays RGB"
        );
        let px = |x: usize, y: usize| &buf[(y * 576 + x) * 3..(y * 576 + x) * 3 + 3];
        assert_eq!(
            px(5, 5),
            &[234, 247, 247],
            "plot noFill must not paint a white plot over the chart fill"
        );
        assert!(
            buf[..info.buffer_size()]
                .as_chunks::<3>()
                .0
                .contains(&[234, 247, 247]),
            "chart-area fill must remain visible in the plot"
        );
        assert!(
            !buf[..info.buffer_size()]
                .as_chunks::<3>()
                .0
                .contains(&[255, 255, 255]),
            "plot noFill over a non-white chart fill must not introduce a white fallback"
        );

        let xml = LONG_LEGEND
            .replace(
                "<c:chart>",
                "<c:roundedCorners val='0'/><c:spPr><a:noFill/></c:spPr><c:chart>",
            )
            .replace("<c:plotArea>", "<c:plotArea><c:spPr><a:noFill/></c:spPr>");
        let doc = crate::xml::parse(xml.as_bytes()).unwrap();
        let frame = read_frame(&doc.root).unwrap();
        assert_eq!(frame.fill, FrameFill::NoFill);
        assert_eq!(frame.plot_fill, FrameFill::NoFill);
        let png = super::render_chart(
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
        let mut reader = png::Decoder::new(std::io::Cursor::new(png))
            .read_info()
            .unwrap();
        let mut buf = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).unwrap();
        assert_eq!(info.color_type, png::ColorType::Rgba);
        let margin = (5 * 576 + 5) * 4;
        assert_eq!(
            buf[margin + 3],
            0,
            "both noFill must keep the chart margin transparent"
        );
        let mut transparent = 0usize;
        for px in buf[..info.buffer_size()].chunks_exact(4) {
            if px[3] == 0 {
                transparent += 1;
                assert_ne!(
                    &px[..3],
                    &[255, 255, 255],
                    "transparent noFill must not use a white key"
                );
            }
        }
        assert!(
            transparent > 1000,
            "both noFill must leave a transparent field, got {transparent}"
        );
    }

    #[test]
    fn chart_and_plot_translucent_fills_keep_straight_alpha() {
        assert_eq!(ooxml_alpha_to_u8(0), 0);
        assert_eq!(ooxml_alpha_to_u8(25000), 64);
        assert_eq!(ooxml_alpha_to_u8(50000), 128);
        assert_eq!(ooxml_alpha_to_u8(100000), 255);

        let xml = LONG_LEGEND.replace(
            "<c:chart>",
            "<c:roundedCorners val='0'/><c:spPr><a:solidFill><a:srgbClr val='EAF7F7'><a:alpha val='50000'/></a:srgbClr></a:solidFill></c:spPr><c:chart>",
        );
        let doc = crate::xml::parse(xml.as_bytes()).unwrap();
        let frame = read_frame(&doc.root).unwrap();
        assert_eq!(frame.fill, FrameFill::Translucent([234, 247, 247], 50000));
        assert_eq!(frame.plot_fill, FrameFill::Default);
        let png = super::render_chart(
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
        let mut reader = png::Decoder::new(std::io::Cursor::new(png))
            .read_info()
            .unwrap();
        let mut buf = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).unwrap();
        assert_eq!(info.color_type, png::ColorType::Rgba);
        let i = (5 * 576 + 5) * 4;
        assert_eq!(
            &buf[i..i + 3],
            &[234, 247, 247],
            "chart fill must stay unassociated, not flattened onto white"
        );
        assert_eq!(buf[i + 3], 128, "alpha 50000 maps to coverage 128");

        let xml = LONG_LEGEND
            .replace(
                "<c:chart>",
                "<c:roundedCorners val='0'/><c:spPr><a:solidFill><a:srgbClr val='EAF7F7'/></a:solidFill></c:spPr><c:chart>",
            )
            .replace(
                "<c:plotArea>",
                "<c:plotArea><c:spPr><a:solidFill><a:srgbClr val='0F6082'><a:alpha val='50000'/></a:srgbClr></a:solidFill></c:spPr>",
            );
        let doc = crate::xml::parse(xml.as_bytes()).unwrap();
        let frame = read_frame(&doc.root).unwrap();
        assert_eq!(frame.fill, FrameFill::Solid([234, 247, 247]));
        assert_eq!(
            frame.plot_fill,
            FrameFill::Translucent([15, 96, 130], 50000)
        );
        let png = super::render_chart(
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
        let mut reader = png::Decoder::new(std::io::Cursor::new(png))
            .read_info()
            .unwrap();
        let mut buf = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).unwrap();
        assert_eq!(
            info.color_type,
            png::ColorType::Rgb,
            "translucent plot over an opaque chart stays opaque"
        );
        let px = |x: usize, y: usize| &buf[(y * 576 + x) * 3..(y * 576 + x) * 3 + 3];
        assert_eq!(px(5, 5), &[234, 247, 247]);
        assert!(
            buf[..info.buffer_size()]
                .as_chunks::<3>()
                .0
                .contains(&[124, 171, 188]),
            "plot alpha 50000 must composite 0F6082 over the chart fill, not fall back to white or an opaque plot color"
        );
        assert!(
            !buf[..info.buffer_size()]
                .as_chunks::<3>()
                .0
                .contains(&[255, 255, 255]),
            "translucent plot over a non-white chart fill must not introduce a white fallback"
        );
        assert!(
            !buf[..info.buffer_size()]
                .as_chunks::<3>()
                .0
                .contains(&[15, 96, 130]),
            "translucent plot must not paint the source color as opaque"
        );

        let xml = LONG_LEGEND.replace(
            "<c:chart>",
            "<c:roundedCorners val='0'/><c:spPr><a:solidFill><a:srgbClr val='EAF7F7'><a:alpha val='25000'/></a:srgbClr></a:solidFill></c:spPr><c:chart>",
        );
        let doc = crate::xml::parse(xml.as_bytes()).unwrap();
        assert_eq!(
            read_frame(&doc.root).unwrap().fill,
            FrameFill::Translucent([234, 247, 247], 25000)
        );
        let png = super::render_chart(
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
        let mut reader = png::Decoder::new(std::io::Cursor::new(png))
            .read_info()
            .unwrap();
        let mut buf = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).unwrap();
        assert_eq!(info.color_type, png::ColorType::Rgba);
        let i = (5 * 576 + 5) * 4;
        assert_eq!(&buf[i..i + 3], &[234, 247, 247]);
        assert_eq!(buf[i + 3], 64, "alpha 25000 maps to coverage 64");
    }


    #[test]
    fn explicit_width_and_inherited_chart_pen_are_distinct_and_scatter_is_preserved() {
        let dpi=RenderDpi::from_request(serde_json::json!({"dpi":106}).as_object().unwrap()).unwrap();
        for (attribute,ordinary,scatter) in [("",4.0,4.0),("w='28575'",3.0,4.0),("w='12700'",1.0,2.0)] {
            let doc=crate::xml::parse(format!("<spPr><ln {attribute}/></spPr>").as_bytes()).unwrap();
            assert_eq!(read_line_sp_pr(&doc.root,FALLBACK_ACCENTS,dpi,true).unwrap().width,ordinary);
            assert_eq!(read_line_sp_pr(&doc.root,FALLBACK_ACCENTS,dpi,false).unwrap().width,scatter);
        }
    }
    #[test]
    fn positive_flat_custom_dash_parses_percentages_of_the_explicit_device_pen() {
        for (width,device) in [(12700,1.0),(25400,3.0),(50800,6.0)] {
            let doc=crate::xml::parse(format!("<ln w='{width}' cap='flat'><custDash><ds d='200000' sp='100000'/></custDash><round/></ln>").as_bytes()).unwrap();
            let style=read_ln_with_caps(&doc.root,FALLBACK_ACCENTS,4.0,RenderDpi::from_request(serde_json::json!({"dpi":106}).as_object().unwrap()).unwrap(),true).unwrap();
            assert_eq!(style.width,device);
            assert_eq!(style.dash.unwrap().painted_intervals(device*5.0,16).unwrap(),[(0.0,device*2.0),(device*3.0,device*5.0)]);
            assert!(read_solid_ln(&doc.root,FALLBACK_ACCENTS,4.0,RenderDpi::from_request(serde_json::json!({"dpi":106}).as_object().unwrap()).unwrap()).is_err());
        }
    }
    #[test]
    fn custom_dash_rejects_unimplemented_caps_missing_width_and_ambiguous_stops() {
        for attrs in ["w='50800'","cap='flat'","w='0' cap='flat'"] {
            let doc=crate::xml::parse(format!("<ln {attrs}><custDash><ds d='200000' sp='100000'/></custDash></ln>").as_bytes()).unwrap();
            assert!(read_ln_with_caps(&doc.root,FALLBACK_ACCENTS,4.0,RenderDpi::default(),true).is_err());
        }
        for body in ["<custDash/>","<custDash offset='1'><ds d='1' sp='1'/></custDash>","<custDash><ds d='0' sp='1'/></custDash>","<custDash><ds d='1.5' sp='1'/></custDash>","<custDash><ds d='1'/></custDash>","<custDash><ds d='2147483648' sp='1'/></custDash>","<custDash><ds d='1' sp='1' other='1'/></custDash>","<prstDash val='solid'/><custDash><ds d='1' sp='1'/></custDash>","<custDash><ds d='1' sp='1'/></custDash><noFill/>","<custDash><ds d='1' sp='1'/></custDash><solidFill><srgbClr val='175DB5'><alpha val='50000'/></srgbClr></solidFill>"] {
            let doc=crate::xml::parse(format!("<ln w='50800' cap='flat'>{body}</ln>").as_bytes()).unwrap();
            assert!(read_ln_with_caps(&doc.root,FALLBACK_ACCENTS,4.0,RenderDpi::default(),true).is_err(),"{body}");
        }
    }
    #[test]
    fn bevel_is_irrelevant_for_a_single_edge_or_round_dot() {
        for cap in [stroke::Cap::Round,stroke::Cap::Flat,stroke::Cap::Square] {
            for d in [25000.0,100000.0,10000000.0] {
                let pattern=stroke::DashPattern::from_width_percent(&[(d,100000.0)],6.0).unwrap();let style=stroke::Style::new(6.0,cap).unwrap();
                let mut a=Canvas::new(100,50);let mut b=Canvas::new(100,50);
                for (canvas,join) in [(&mut a,stroke::Join::Round),(&mut b,stroke::Join::Bevel)] {
                    stroke_run_pattern_join(canvas,&[(20.25,20.25),(70.25,20.25)],style,Some(&pattern),join,[23,93,181]).unwrap();
                }
                assert_eq!(a.px,b.px);assert_eq!(a.alpha,b.alpha);assert_eq!(a.custom_dash_budget,b.custom_dash_budget);
            }
        }
    }
    #[test]
    fn bevel_custom_parser_is_explicit_and_does_not_expand_miter_or_legacy_solid() {
        for cap in ["flat","sq","rnd"] {
            let xml=format!("<ln w='50800' cap='{cap}'><solidFill><srgbClr val='175DB5'/></solidFill><custDash><ds d='10000000' sp='100000'/></custDash><bevel/></ln>");
            let doc=crate::xml::parse(xml.as_bytes()).unwrap();let style=read_ln_with_caps(&doc.root,FALLBACK_ACCENTS,6.0,RenderDpi::default(),true).unwrap();assert_eq!(style.join,stroke::Join::Bevel);
            for bad in ["<miter lim='800000'/>","<bevel lim='1'/>","<bevel><x/></bevel>","<round/><bevel/>","<bevel/><bevel/>"] {
                let doc=crate::xml::parse(xml.replace("<bevel/>",bad).as_bytes()).unwrap();assert!(read_ln_with_caps(&doc.root,FALLBACK_ACCENTS,6.0,RenderDpi::default(),true).is_err());
            }
        }
        let doc=crate::xml::parse(b"<ln w='50800' cap='rnd'><solidFill><srgbClr val='175DB5'/></solidFill><bevel/></ln>").unwrap();assert_eq!(read_solid_ln(&doc.root,FALLBACK_ACCENTS,6.0,RenderDpi::default()).unwrap().join,stroke::Join::Round);
    }
    #[test]
    fn bevel_custom_joins_fill_hull_without_round_bulge_or_butt_gap() {
        let pts=[(15.0,40.0),(40.0,15.0),(65.0,40.0)];let mut round=Canvas::new(80,60);let mut bevel=Canvas::new(80,60);
        for (canvas,join) in [(&mut round,stroke::Join::Round),(&mut bevel,stroke::Join::Bevel)] {canvas.stroke_custom_dash_union_join(&pts,14.0,stroke::Cap::Flat,stroke::Cap::Flat,join,[23,93,181]);}
        assert_ne!(round.px,bevel.px);let at=|c:&Canvas,x:usize,y:usize|c.px[(y*80+x)*3..(y*80+x+1)*3].to_vec();
        assert_eq!(at(&bevel,40,15),vec![23,93,181]);assert_eq!(at(&bevel,40,9),vec![255,255,255]);assert_ne!(at(&round,40,9),vec![255,255,255]);
    }
    #[test]
    fn bevel_custom_round_trip_reaches_line_renderer_and_respects_budget() {
        let make=|join:&str| {
            let extra=format!("<c:spPr><a:ln w='101600' cap='flat'><a:solidFill><a:srgbClr val='175DB5'/></a:solidFill><a:custDash><a:ds d='10000000' sp='100000'/></a:custDash><a:{join}/></a:ln></c:spPr><c:marker><c:symbol val='none'/></c:marker>");
            standard_line_xml(&line_ser(0,"H",&[Some(10.0),Some(10.0),Some(90.0),Some(10.0)],&extra),"")
        };
        assert_ne!(render_line(&make("round")).unwrap(),render_line(&make("bevel")).unwrap());assert!(render_line(&make("miter")).is_err());
        let mut c=Canvas::new(80,60);c.custom_dash_budget=1;let p=stroke::DashPattern::from_width_percent(&[(10000000.0,100000.0)],6.0).unwrap();
        stroke_run_pattern_join(&mut c,&[(10.0,40.0),(40.0,10.0),(70.0,40.0)],stroke::Style::new(6.0,stroke::Cap::Flat).unwrap(),Some(&p),stroke::Join::Bevel,[23,93,181]).unwrap();assert_eq!(c.custom_dash_budget,0);
    }
    #[test]
    fn custom_dash_union_retrace_does_not_reblend_boundary() {
        let mut once=Canvas::new(80,60);let mut retrace=Canvas::new(80,60);
        let a=(18.25,20.25);let b=(58.25,20.25);let color=[23,93,181];
        stroke_run_dash_caps(&mut once,&[a,b],6.0,stroke::Cap::Round,stroke::Cap::Round,color);
        stroke_run_dash_caps(&mut retrace,&[a,b,a],6.0,stroke::Cap::Round,stroke::Cap::Round,color);
        assert_eq!(once.px,retrace.px);assert_eq!(once.alpha,retrace.alpha);
    }
    #[test]
    fn custom_dash_union_matches_one_blend_of_max_coverage_at_turn() {
        for cap in [stroke::Cap::Round,stroke::Cap::Flat,stroke::Cap::Square] {
            let points=[(18.25,40.25),(38.25,12.25),(58.25,40.25)];let width=6.0;let color=[23,93,181];
            let mut actual=Canvas::new(80,60);let mut expected=Canvas::new(80,60);
            stroke_run_dash_caps(&mut actual,&points,width,cap,cap,color);
            let a=stroke::CapSegment::new(points[0],points[1],width,cap,stroke::Cap::Round).unwrap();
            let b=stroke::CapSegment::new(points[1],points[2],width,stroke::Cap::Round,cap).unwrap();
            for y in 0..60 {for x in 0..80 {
                let p=(f64::from(x)+0.5,f64::from(y)+0.5);
                let alpha=coverage(a.signed_distance(p),0.0).max(coverage(b.signed_distance(p),0.0));
                if alpha>0 {expected.blend(x,y,color,alpha);}
            }}
            assert_eq!(actual.px,expected.px);assert_eq!(actual.alpha,expected.alpha);
        }
    }
    #[test]
    fn custom_dash_union_zero_edges_reverse_and_offcanvas_are_stable() {
        let p=[(-30.25,20.25),(30.25,10.25),(90.25,20.25)];let mut reversed=p;reversed.reverse();
        let mut a=Canvas::new(60,40);let mut b=Canvas::new(60,40);let mut c=Canvas::new(60,40);
        for (canvas,points) in [(&mut a,p.as_slice()),(&mut b,reversed.as_slice()),(&mut c,&[p[0],p[0],p[1],p[1],p[2]])] {
            stroke_run_dash_caps(canvas,points,6.0,stroke::Cap::Flat,stroke::Cap::Flat,[23,93,181]);
        }
        assert_eq!(a.px,b.px);assert_eq!(a.px,c.px);
        let before=a.px.clone();a.stroke_custom_dash_union(&[(-100.0,-100.0),(-80.0,-60.0),(-50.0,-100.0)],6.0,stroke::Cap::Round,stroke::Cap::Round,[23,93,181]);assert_eq!(a.px,before);
    }
    #[test]
    fn custom_dash_union_keeps_phase_and_piece_budget_across_true_turns() {
        let mut c=Canvas::new(80,60);c.custom_dash_budget=1;
        let p=stroke::DashPattern::from_width_percent(&[(10000000.0,100000.0)],6.0).unwrap();
        stroke_run_pattern(&mut c,&[(10.0,40.0),(40.0,10.0),(70.0,40.0)],stroke::Style::new(6.0,stroke::Cap::Flat).unwrap(),Some(&p),[23,93,181]).unwrap();
        assert_eq!(c.custom_dash_budget,0);let before=c.px.clone();
        assert!(stroke_run_pattern(&mut c,&[(10.0,50.0),(30.0,50.0)],stroke::Style::new(6.0,stroke::Cap::Flat).unwrap(),Some(&p),[23,93,181]).is_err());assert_eq!(c.px,before);
    }

    #[test]
    fn round_dot_raster_reuses_one_existing_circle_and_is_invariant_below_one_pen() {
        let mut expected=None;
        for d in [25000.0,50000.0,100000.0] {
            let p=stroke::DashPattern::from_width_percent(&[(d,100000.0)],6.0).unwrap();let mut c=Canvas::new(100,40);
            stroke_run_pattern(&mut c,&[(20.0,20.0),(70.0,20.0)],stroke::Style::new(6.0,stroke::Cap::Round).unwrap(),Some(&p),[23,93,181]).unwrap();
            if let Some(ref pixels)=expected {assert_eq!(&c.px,pixels);}else{expected=Some(c.px.clone());}
            let mut primitive=Canvas::new(100,40);
            for x in [20.0,32.0,44.0,56.0,68.0] {primitive.stroke_segment(x,20.0,x,20.0,6.0,[23,93,181]);}
            assert_eq!(c.px,primitive.px);assert_eq!(c.alpha,primitive.alpha);
        }
    }
    #[test]
    fn round_dot_is_not_a_closed_polyline_and_uses_the_shared_budget() {
        let p=stroke::DashPattern::from_width_percent(&[(50000.0,100000.0)],2.0).unwrap();let mut c=Canvas::new(100,50);c.custom_dash_budget=2;
        let style=stroke::Style::new(2.0,stroke::Cap::Round).unwrap();
        stroke_run_pattern(&mut c,&[(20.0,20.0),(25.0,20.0)],style,Some(&p),[23,93,181]).unwrap();assert_eq!(c.custom_dash_budget,0);
        let before=c.px.clone();assert!(stroke_run_pattern(&mut c,&[(30.0,20.0),(35.0,20.0)],style,Some(&p),[23,93,181]).is_err());assert_eq!(c.px,before);
        let mut looped=Canvas::new(100,50);stroke_run_dash_caps(&mut looped,&[(20.0,20.0),(40.0,20.0),(20.0,20.0)],6.0,stroke::Cap::Round,stroke::Cap::Round,[23,93,181]);
        assert_eq!(&looped.px[(20*100+35)*3..(20*100+36)*3],&[23,93,181]);
    }
    #[test]
    fn positive_round_dots_reach_legacy_and_advanced_legend_keys_without_enabling_zero_stops() {
        for pos in ["r","b"] {
            let make=|d| {
                let extra=format!("<c:spPr><a:ln w='50800' cap='rnd'><a:solidFill><a:srgbClr val='175DB5'/></a:solidFill><a:custDash><a:ds d='{d}' sp='100000'/></a:custDash><a:round/></a:ln></c:spPr><c:marker><c:symbol val='none'/></c:marker>");
                standard_line_xml(&line_ser(0,"H",&[Some(40.0);8],&extra),"").replace("<c:legendPos val=\"r\"/>",&format!("<c:legendPos val=\"{pos}\"/>"))
            };
            assert_eq!(render_line(&make(25000)).unwrap(),render_line(&make(100000)).unwrap());assert!(render_line(&make(0)).is_err());
        }
    }

    #[test]
    fn explicit_round_square_custom_dash_parser_supports_only_evidenced_baseline() {
        for cap in ["rnd","sq","flat"] {
            let make=|extra:&str,d:i32| format!("<ln w='50800' cap='{cap}'><solidFill><srgbClr val='175DB5'/></solidFill><custDash><ds d='{d}' sp='100000'/></custDash>{extra}</ln>");
            let parse=|xml:String| {let doc=crate::xml::parse(xml.as_bytes()).unwrap();read_ln_with_caps(&doc.root,FALLBACK_ACCENTS,4.0,RenderDpi::default(),true)};
            assert!(parse(make("<round/>",200000)).is_ok());
            assert!(parse(make("<bevel/>",200000)).is_ok());
            assert!(parse(make("<miter/>",200000)).is_err());
            let alpha=make("",200000).replace("val='175DB5'/>","val='175DB5'><alpha val='50000'/></srgbClr>");
            assert!(parse(alpha).is_err());
            for d in [1,25000,50000,100000] {assert!(parse(make("",d)).is_ok());}
        }
    }
    #[test]
    fn round_square_custom_raster_does_not_depend_on_collinear_tessellation() {
        for cap in [stroke::Cap::Round,stroke::Cap::Square] {
            let p=stroke::DashPattern::from_width_percent(&[(200000.0,100000.0)],6.0).unwrap();
            let style=stroke::Style::new(6.0,cap).unwrap();let mut a=Canvas::new(120,40);let mut b=Canvas::new(120,40);
            stroke_run_pattern(&mut a,&[(20.0,20.0),(100.0,20.0)],style,Some(&p),[23,93,181]).unwrap();
            stroke_run_pattern(&mut b,&[(20.0,20.0),(23.0,20.0),(54.0,20.0),(100.0,20.0)],style,Some(&p),[23,93,181]).unwrap();
            assert_eq!(a.px,b.px);assert_eq!(a.alpha,b.alpha);
        }
    }
    #[test]
    fn square_internal_dash_is_flat_while_true_run_ends_extend_half_a_pen() {
        let p=stroke::DashPattern::from_width_percent(&[(200000.0,100000.0)],6.0).unwrap();
        let mut flat=Canvas::new(120,40);let mut square=Canvas::new(120,40);
        for (c,cap) in [(&mut flat,stroke::Cap::Flat),(&mut square,stroke::Cap::Square)] {
            stroke_run_pattern(c,&[(20.0,20.0),(100.0,20.0)],stroke::Style::new(6.0,cap).unwrap(),Some(&p),[23,93,181]).unwrap();
        }
        assert_ne!(flat.px,square.px);
        // Exclude BOTH actual run endpoints: each legitimately gets a cap.
        for y in 0..40 {assert_eq!(&flat.px[(y*120+25)*3..(y*120+95)*3],&square.px[(y*120+25)*3..(y*120+95)*3]);}
        assert_ne!(&flat.px[(20*120+101)*3..(20*120+102)*3],&square.px[(20*120+101)*3..(20*120+102)*3]);
    }
    #[test]
    fn round_square_custom_keys_render_in_both_legacy_and_advanced_legends() {
        for pos in ["r","b"] {
            let mut images=Vec::new();
            for cap in ["flat","sq","rnd"] {
                let extra=format!("<c:spPr><a:ln w='50800' cap='{cap}'><a:solidFill><a:srgbClr val='175DB5'/></a:solidFill><a:custDash><a:ds d='200000' sp='100000'/></a:custDash><a:round/></a:ln></c:spPr><c:marker><c:symbol val='none'/></c:marker>");
                let xml=standard_line_xml(&line_ser(0,"H",&[Some(40.0);8],&extra),"").replace("<c:legendPos val=\"r\"/>",&format!("<c:legendPos val=\"{pos}\"/>"));
                images.push(render_line(&xml).unwrap());
            }
            assert_ne!(images[0],images[1]);assert_ne!(images[0],images[2]);assert_ne!(images[1],images[2]);
        }
    }

    #[test]
    fn patterned_strokes_share_an_image_budget_and_never_silently_truncate() {
        let pattern=stroke::DashPattern::from_width_percent(&[(200000.0,100000.0)],2.0).unwrap();
        let style=stroke::Style::new(2.0,stroke::Cap::Flat).unwrap();
        let mut c=Canvas::new(80,40);c.custom_dash_budget=2;
        stroke_run_pattern(&mut c,&[(20.0,20.0),(30.0,20.0)],style,Some(&pattern),[23,93,181]).unwrap();
        assert_eq!(c.custom_dash_budget,0);
        let pixel=|c:&Canvas,x:usize| &c.px[(20*80+x)*3..(20*80+x)*3+3]==[23,93,181];
        assert!(pixel(&c,21));assert!(!pixel(&c,25));assert!(pixel(&c,27));
        let before=c.px.clone();
        assert!(stroke_run_pattern(&mut c,&[(40.0,20.0),(50.0,20.0)],style,Some(&pattern),[23,93,181]).is_err());
        assert_eq!(before,c.px);
    }

    #[test]
    fn custom_flat_raster_is_invariant_to_collinear_subdivision_without_touching_legacy_solids() {
        let pattern=stroke::DashPattern::from_width_percent(&[(200000.0,100000.0)],1.0).unwrap();
        let style=stroke::Style::new(1.0,stroke::Cap::Flat).unwrap();
        let mut a=Canvas::new(80,40);let mut b=Canvas::new(80,40);
        stroke_run_pattern(&mut a,&[(20.0,20.0),(60.0,20.0)],style,Some(&pattern),[23,93,181]).unwrap();
        stroke_run_pattern(&mut b,&[(20.0,20.0),(23.5,20.0),(41.5,20.0),(60.0,20.0)],style,Some(&pattern),[23,93,181]).unwrap();
        assert_eq!(a.px,b.px);
    }
    #[test]
    fn flat_custom_dash_and_actual_pen_are_painted_in_advanced_legend_too() {
        let make=|dash:&str| standard_line_xml(&line_ser(0,"H",&[Some(40.0);8],&format!("<c:spPr><a:ln w='50800' cap='flat'><a:solidFill><a:srgbClr val='175DB5'/></a:solidFill>{dash}<a:round/></a:ln></c:spPr><c:marker><c:symbol val='none'/></c:marker>")),"")
            .replace("<c:legendPos val=\"r\"/>","<c:legendPos val=\"b\"/>");
        let (_,_,solid)=png_rgb(&render_line(&make("<a:prstDash val='solid'/>")).unwrap());
        let (_,_,dashed)=png_rgb(&render_line(&make("<a:custDash><a:ds d='200000' sp='100000'/></a:custDash>")).unwrap());
        let a=count_in(&solid,576,0,576,300,336,[23,93,181],0);
        let b=count_in(&dashed,576,0,576,300,336,[23,93,181],0);
        assert!(b>0 && b<a,"the advanced legend must show a real dashed key: solid={a},dash={b}");
    }

    #[test]
    fn explicit_line_caps_and_key_width_use_shared_style_but_other_strokes_stay_gated() {
        let mut images=Vec::new();
        for cap in ["rnd","flat","sq"] {
            let extra=format!("<c:spPr><a:ln w='50800' cap='{cap}' cmpd='sng' algn='ctr'><a:solidFill><a:srgbClr val='175DB5'/></a:solidFill><a:prstDash val='solid'/><a:round/></a:ln></c:spPr><c:marker><c:symbol val='none'/></c:marker>");
            let xml=standard_line_xml(&line_ser(0,"H",&[Some(40.0),Some(40.0),Some(40.0)],&extra),"");images.push(render_line(&xml).unwrap());
            let node=crate::xml::parse(format!("<a:ln xmlns:a='a' cap='{cap}' w='50800'><a:prstDash val='solid'/><a:round/></a:ln>").as_bytes()).unwrap();
            let style=read_ln_with_caps(&node.root,FALLBACK_ACCENTS,2.0,RenderDpi::default(),true).unwrap();assert_eq!(style.cap,stroke::Cap::from_ooxml(cap).unwrap());
            if cap!="rnd" {assert!(read_solid_ln(&node.root,FALLBACK_ACCENTS,2.0,RenderDpi::default()).is_err());}
        }
        assert_ne!(images[0],images[1]);assert_ne!(images[0],images[2]);assert_ne!(images[1],images[2]);
        for body in ["<a:prstDash val='dash'/>","<a:custDash><a:ds d='200000' sp='100000'/></a:custDash>","<a:headEnd type='triangle'/>","<a:miter/>","<a:solidFill><a:srgbClr val='175DB5'><a:alpha val='50000'/></a:srgbClr></a:solidFill>"] {
            let doc=crate::xml::parse(format!("<a:ln xmlns:a='a' cap='flat'>{body}</a:ln>").as_bytes()).unwrap();assert!(read_ln_with_caps(&doc.root,FALLBACK_ACCENTS,2.0,RenderDpi::default(),true).is_err());
        }
        let mut thin=Canvas::new(90,40);let mut thick=Canvas::new(90,40);
        for (canvas,width) in [(&mut thin,1.0),(&mut thick,6.0)] {draw_plot_key_styled(canvas,PlotKind::StandardLine,20,10,49,19,[0,0,0],MarkerKind::None,stroke::Style::new(width,stroke::Cap::Flat),None).unwrap();}
        assert_ne!(thin.px,thick.px);
    }

    #[test]
    fn duplicate_endpoint_edges_do_not_steal_nonround_run_caps() {
        for cap in [stroke::Cap::Flat,stroke::Cap::Square] {
            let mut simple=Canvas::new(80,40);let mut repeated=Canvas::new(80,40);let style=stroke::Style::new(8.0,cap).unwrap();
            stroke_run_style(&mut simple,&[(20.0,20.0),(60.0,20.0)],style,[0,0,0]);
            stroke_run_style(&mut repeated,&[(20.0,20.0),(20.0,20.0),(60.0,20.0),(60.0,20.0)],style,[0,0,0]);assert_eq!(simple.px,repeated.px);
        }
    }

    #[test]
    fn shared_stroke_round_runs_preserve_legacy_pixels_exactly() {
        for width in [1.0, 2.25, 4.0, 7.5] {
            for points in [vec![(10.0,20.0),(40.0,20.0),(40.0,50.0),(90.0,50.0)], vec![(10.0,20.0),(10.0,20.0),(90.0,50.0)], vec![(-5.0,10.0),(50.5,40.5),(130.0,10.0)]] {
                let mut before=Canvas::new(120,70);let mut after=Canvas::new(120,70);
                for pair in points.windows(2) { before.stroke_segment(pair[0].0,pair[0].1,pair[1].0,pair[1].1,width,[23,93,181]); }
                stroke_run(&mut after,&points,width,[23,93,181]);assert_eq!(before.px,after.px);assert_eq!(before.alpha,after.alpha);
            }
        }
    }

    #[test]
    fn explicit_cap_prerequisites_render_document_contract_not_native_gold() {
        let mut image=Canvas::new(280,180);
        for (i,cap) in [stroke::Cap::Round,stroke::Cap::Flat,stroke::Cap::Square].into_iter().enumerate() {
            let y=25.0+i as f64*55.0;
            stroke_run_style(&mut image,&[(30.0,y),(130.0,y),(180.0,y+20.0),(240.0,y+20.0)],stroke::Style::new(12.0,cap).unwrap(),[23,93,181]);
        }
        let mut round=Canvas::new(80,30);let mut flat=Canvas::new(80,30);let mut square=Canvas::new(80,30);
        for (canvas,cap) in [(&mut round,stroke::Cap::Round),(&mut flat,stroke::Cap::Flat),(&mut square,stroke::Cap::Square)] {stroke_run_style(canvas,&[(20.0,15.0),(60.0,15.0)],stroke::Style::new(8.0,cap).unwrap(),[0,0,0]);}
        assert_ne!(round.px,flat.px);assert_ne!(round.px,square.px);assert_ne!(flat.px,square.px);
        if let Some(dir)=std::env::var_os("OOXP_STROKE_FOUNDATION_REVIEW_DIR") {
            let dir=std::path::PathBuf::from(dir);assert!(dir.is_dir());let path=dir.join("document-contract-cap-primitives.png");assert!(!path.exists());std::fs::write(path,image.encode_png().unwrap()).unwrap();
        }
    }

    #[test]
    fn two_stop_linear_frame_fills_use_office_gamma_weight() {
        let xml = LONG_LEGEND.replace(
            "<c:chart>",
            "<c:roundedCorners val='0'/><c:spPr><a:gradFill flip='none' rotWithShape='1'><a:gsLst><a:gs pos='0'><a:srgbClr val='EAF7F7'/></a:gs><a:gs pos='100000'><a:srgbClr val='0F6082'/></a:gs></a:gsLst><a:lin ang='0' scaled='1'/><a:tileRect/></a:gradFill></c:spPr><c:chart>",
        );
        let doc = crate::xml::parse(xml.as_bytes()).unwrap();
        let frame = read_frame(&doc.root).unwrap();
        assert_eq!(
            frame.fill,
            FrameFill::Linear(LinearFrameGrad {
                stops: gradient::StopTable::new(vec![(0, [234, 247, 247]), (100_000, [15, 96, 130])]).unwrap(),
                ang: LINEAR_ANG_HORIZONTAL,
            })
        );
        let png = super::render_chart(
            &doc.root,
            424,
            265,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .unwrap();
        let mut reader = png::Decoder::new(std::io::Cursor::new(png))
            .read_info()
            .unwrap();
        let mut buf = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).unwrap();
        let stride = if info.color_type == png::ColorType::Rgba {
            4
        } else {
            3
        };
        let mid = (8 * 424 + 212) * stride;
        assert_eq!(
            &buf[mid..mid + 3],
            &[171, 190, 199],
            "geometric mid must follow gamma 2.2 plus the Office weight curve, not an sRGB lerp"
        );
        let left = (8 * 424 + 5) * stride;
        assert_eq!(&buf[left..left + 3], &[234, 247, 247]);

        let xml = LONG_LEGEND
            .replace(
                "<c:chart>",
                "<c:roundedCorners val='0'/><c:spPr><a:solidFill><a:srgbClr val='EAF7F7'/></a:solidFill></c:spPr><c:chart>",
            )
            .replace(
                "<c:plotArea>",
                "<c:plotArea><c:spPr><a:gradFill flip='none' rotWithShape='1'><a:gsLst><a:gs pos='0'><a:srgbClr val='0F6082'/></a:gs><a:gs pos='100000'><a:srgbClr val='EAF7F7'/></a:gs></a:gsLst><a:lin ang='5400000' scaled='1'/><a:tileRect/></a:gradFill></c:spPr>",
            );
        let doc = crate::xml::parse(xml.as_bytes()).unwrap();
        assert_eq!(
            read_frame(&doc.root).unwrap().plot_fill,
            FrameFill::Linear(LinearFrameGrad {
                stops: gradient::StopTable::new(vec![(0, [15, 96, 130]), (100_000, [234, 247, 247])]).unwrap(),
                ang: LINEAR_ANG_VERTICAL,
            })
        );
        assert!(super::render_chart(
            &doc.root,
            424,
            265,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .is_ok());
    }

    const OFFICE16_THEME: &str = r#"<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <a:themeElements>
    <a:clrScheme name="Office">
      <a:dk1><a:sysClr val="windowText" lastClr="000000"/></a:dk1>
      <a:lt1><a:sysClr val="window" lastClr="FFFFFF"/></a:lt1>
      <a:dk2><a:srgbClr val="0E2841"/></a:dk2>
      <a:lt2><a:srgbClr val="E8E8E8"/></a:lt2>
      <a:accent1><a:srgbClr val="156082"/></a:accent1>
      <a:accent2><a:srgbClr val="E97132"/></a:accent2>
      <a:accent3><a:srgbClr val="196B24"/></a:accent3>
      <a:accent4><a:srgbClr val="0F9ED5"/></a:accent4>
      <a:accent5><a:srgbClr val="A02B93"/></a:accent5>
      <a:accent6><a:srgbClr val="4EA72E"/></a:accent6>
      <a:hlink><a:srgbClr val="467886"/></a:hlink>
      <a:folHlink><a:srgbClr val="96607D"/></a:folHlink>
    </a:clrScheme>
  </a:themeElements>
</a:theme>"#;

    #[test]
    fn stop_table_axis_cache_preserves_all_legacy_pixels_exactly() {
        let start = [234, 247, 247];
        let end = [15, 96, 130];
        for (width, height) in [(2, 3), (17, 9), (424, 265), (4096, 3)] {
            for ang in [LINEAR_ANG_HORIZONTAL, LINEAR_ANG_VERTICAL] {
                for clip in [false, true] {
                    let (x0, y0, x1, y1) = if clip { (-5, -2, width + 7, height + 3) } else { (0, 0, width, height) };
                    let grad = LinearFrameGrad { stops: gradient::StopTable::new(vec![(0, start), (100_000, end)]).unwrap(), ang };
                    let mut canvas = Canvas::new(width as u32, height as u32);
                    canvas.fill_linear_rect(x0, y0, x1, y1, &grad);
                    for y in 0..height {
                        for x in 0..width {
                            let along = if ang == LINEAR_ANG_VERTICAL { (f64::from(y - y0) + 0.5) / f64::from(y1 - y0) } else { (f64::from(x - x0) + 0.5) / f64::from(x1 - x0) };
                            let share = office_grad_weight(along);
                            let expected = std::array::from_fn::<_, 3, _>(|i| office_gamma_mix(start[i], end[i], share));
                            let i = ((y * width + x) * 3) as usize;
                            assert_eq!(&canvas.px[i..i + 3], &expected, "{width}x{height} angle={ang} clip={clip} x={x} y={y}");
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn multistop_frame_and_plot_use_segmented_srgb_not_two_stop_gamma() {
        let stops = "<a:gs pos='0'><a:srgbClr val='EAF7F7'/></a:gs><a:gs pos='50000'><a:srgbClr val='0F6082'/></a:gs><a:gs pos='100000'><a:srgbClr val='F04020'/></a:gs>";
        for plot in [false, true] {
            for ang in [0, 5_400_000] {
                let fill = format!("<c:spPr><a:gradFill><a:gsLst>{stops}</a:gsLst><a:lin ang='{ang}' scaled='1'/></a:gradFill></c:spPr>");
                let xml = if plot { LONG_LEGEND.replace("<c:plotArea>", &format!("<c:plotArea>{fill}")) } else { LONG_LEGEND.replace("<c:chart>", &format!("{fill}<c:chart>")) };
                let doc = crate::xml::parse(xml.as_bytes()).unwrap();
                let frame = read_frame(&doc.root).unwrap();
                let FrameFill::Linear(grad) = (if plot { frame.plot_fill } else { frame.fill }) else { panic!("expected gradient table"); };
                assert_eq!(grad.stops.len(), 3);
                assert_eq!(grad.sample(0.25), [125, 172, 189]);
                assert_eq!(grad.sample(0.5), [15, 96, 130]);
                assert_eq!(grad.sample(0.75), [128, 80, 81]);
                for value in [96, 106, 144] {
                    let dpi = RenderDpi::from_request(serde_json::json!({"dpi":value}).as_object().unwrap()).unwrap();
                    assert!(render_chart_at(&doc.root, 424, 265, FALLBACK_ACCENTS, None, ChartHost::Excel, None, "", dpi).is_ok());
                }
            }
        }
        for stops in [
            "<a:gs pos='0'><a:srgbClr val='FFFFFF'/></a:gs><a:gs pos='0'><a:srgbClr val='000000'/></a:gs><a:gs pos='100000'><a:srgbClr val='FF0000'/></a:gs>",
            "<a:gs pos='10000'><a:srgbClr val='FFFFFF'/></a:gs><a:gs pos='50000'><a:srgbClr val='000000'/></a:gs><a:gs pos='100000'><a:srgbClr val='FF0000'/></a:gs>",
            "<a:gs pos='0'><a:srgbClr val='FFFFFF'/></a:gs><a:gs pos='50000'><a:srgbClr val='000000'><a:alpha val='50000'/></a:srgbClr></a:gs><a:gs pos='100000'><a:srgbClr val='FF0000'/></a:gs>",
        ] {
            let fill = format!("<c:spPr><a:gradFill><a:gsLst>{stops}</a:gsLst><a:lin ang='0'/></a:gradFill></c:spPr>");
            let doc = crate::xml::parse(LONG_LEGEND.replace("<c:chart>", &format!("{fill}<c:chart>")).as_bytes()).unwrap();
            assert!(read_frame(&doc.root).is_err());
        }
    }

    #[test]
    fn frame_scheme_tint_shade_and_lum_match_official_excel_pixels() {
        let accent1 = [0x15, 0x60, 0x82];
        assert_eq!(frame_tint(accent1, 60_000), [170, 182, 193]);
        assert_eq!(frame_shade(accent1, 75_000), [17, 84, 114]);
        assert_eq!(frame_lum(accent1, Some(75_000), None), [16, 72, 98]);
        assert_eq!(
            frame_lum(accent1, Some(60_000), Some(40_000)),
            [70, 177, 225]
        );
        // Pure 2.2 or gamma-encoded lerp must not be used: they miss official by 1+.
        assert_ne!(
            accent1.map(|c| {
                let lin = (f64::from(c) / 255.0).powf(2.2);
                let mixed = lin * 0.6 + 0.4;
                (mixed.powf(1.0 / 2.2) * 255.0).round().clamp(0.0, 255.0) as u8
            }),
            [170, 182, 193]
        );
        assert_ne!(tint_rgb(accent1, 0.6), [170, 182, 193]);
        assert_ne!(scale_rgb(accent1, 0.75), [17, 84, 114]);

        let scheme = theme_scheme(Some(OFFICE16_THEME.as_bytes()), FALLBACK_ACCENTS);
        assert_eq!(scheme_slot("accent1", &scheme), Some(accent1));
        assert_eq!(scheme_slot("dk2", &scheme), Some([0x0E, 0x28, 0x41]));
        assert_eq!(scheme_slot("lt2", &scheme), Some([0xE8, 0xE8, 0xE8]));
        assert_eq!(scheme_slot("dk1", &scheme), Some([0, 0, 0]));
        assert_eq!(scheme_slot("lt1", &scheme), Some([255, 255, 255]));
        // Without the theme, dk2 is not the stale hardcoded 1F497D.
        let fallback = theme_scheme(None, FALLBACK_ACCENTS);
        assert_eq!(scheme_slot("dk2", &fallback), None);
        assert_eq!(scheme_slot("accent1", &fallback), Some(accent1));

        let cases = [
            (
                "<c:spPr><a:solidFill><a:schemeClr val='accent1'/></a:solidFill></c:spPr>",
                FrameFill::Solid(accent1),
            ),
            (
                "<c:spPr><a:solidFill><a:schemeClr val='dk2'/></a:solidFill></c:spPr>",
                FrameFill::Solid([0x0E, 0x28, 0x41]),
            ),
            (
                "<c:spPr><a:solidFill><a:srgbClr val='156082'><a:tint val='60000'/></a:srgbClr></a:solidFill></c:spPr>",
                FrameFill::Solid([170, 182, 193]),
            ),
            (
                "<c:spPr><a:solidFill><a:schemeClr val='accent1'><a:tint val='60000'/></a:schemeClr></a:solidFill></c:spPr>",
                FrameFill::Solid([170, 182, 193]),
            ),
            (
                "<c:spPr><a:solidFill><a:srgbClr val='156082'><a:shade val='75000'/></a:srgbClr></a:solidFill></c:spPr>",
                FrameFill::Solid([17, 84, 114]),
            ),
            (
                "<c:spPr><a:solidFill><a:schemeClr val='accent1'><a:shade val='75000'/></a:schemeClr></a:solidFill></c:spPr>",
                FrameFill::Solid([17, 84, 114]),
            ),
            (
                "<c:spPr><a:solidFill><a:srgbClr val='156082'><a:lumMod val='75000'/></a:srgbClr></a:solidFill></c:spPr>",
                FrameFill::Solid([16, 72, 98]),
            ),
            (
                "<c:spPr><a:solidFill><a:srgbClr val='156082'><a:lumMod val='60000'/><a:lumOff val='40000'/></a:srgbClr></a:solidFill></c:spPr>",
                FrameFill::Solid([70, 177, 225]),
            ),
        ];
        for (config, expected) in cases {
            let xml = LONG_LEGEND.replace("<c:chart>", &format!("{config}<c:chart>"));
            let doc = crate::xml::parse(xml.as_bytes()).unwrap();
            let frame = read_frame_with(&doc.root, &scheme).unwrap();
            assert_eq!(frame.fill, expected, "parse {config}");
            assert!(super::render_chart_with_theme(
                &doc.root,
                424,
                265,
                FALLBACK_ACCENTS,
                None,
                ChartHost::Excel,
                None,
                "",
                RenderDpi::DEFAULT,
                Some(OFFICE16_THEME.as_bytes()),
            )
            .is_ok());
        }

        let plot_xml = LONG_LEGEND
            .replace(
                "<c:chart>",
                "<c:roundedCorners val='0'/><c:spPr><a:solidFill><a:srgbClr val='EAF7F7'/></a:solidFill></c:spPr><c:chart>",
            )
            .replace(
                "<c:plotArea>",
                "<c:plotArea><c:spPr><a:solidFill><a:schemeClr val='accent1'/></a:solidFill></c:spPr>",
            );
        let doc = crate::xml::parse(plot_xml.as_bytes()).unwrap();
        let frame = read_frame_with(&doc.root, &scheme).unwrap();
        assert_eq!(frame.fill, FrameFill::Solid([234, 247, 247]));
        assert_eq!(frame.plot_fill, FrameFill::Solid(accent1));
    }

    #[test]
    fn signed_luminance_offset_is_distinct_from_alpha_percentage() {
        for (raw, expected) in [("0", 0), ("-0", 0), ("40000", 40_000), ("-20000", -20_000), ("100000", 100_000), ("-100000", -100_000)] {
            let xml = crate::xml::parse(format!(r#"<lumOff val="{raw}"/>"#).as_bytes()).unwrap();
            assert_eq!(parse_luminance_offset(&xml.root), Some(expected));
        }
        assert_eq!(parse_ooxml_alpha(Some("-20000")), None);
        assert_eq!(frame_lum([21, 96, 130], None, Some(-100_000)), [0, 0, 0]);
        assert_eq!(frame_lum([21, 96, 130], None, Some(100_000)), [255, 255, 255]);
    }

    #[test]
    fn standalone_luminance_offset_matches_controlled_native_colors() {
        // Six same-title/data/288x180pt fixtures were re-exported readonly by
        // Excel16/DPI106. These are native color probes, not full-image passes.
        let scheme = theme_scheme(Some(OFFICE16_THEME.as_bytes()), FALLBACK_ACCENTS);
        for (kind, base, offset, alpha, expected) in [
            ("srgbClr", "156082", "40000", "", FrameFill::Solid([122, 199, 233])),
            ("srgbClr", "156082", "-20000", "", FrameFill::Solid([7, 31, 42])),
            ("srgbClr", "156082", "100000", "", FrameFill::Solid([255, 255, 255])),
            ("schemeClr", "accent1", "40000", "", FrameFill::Solid([122, 199, 233])),
            ("srgbClr", "156082", "40000", "<a:alpha val='50000'/>", FrameFill::Translucent([122, 199, 233], 50_000)),
        ] {
            let xml = format!("<c:chartSpace xmlns:c='http://schemas.openxmlformats.org/drawingml/2006/chart' xmlns:a='http://schemas.openxmlformats.org/drawingml/2006/main'><c:spPr><a:solidFill><a:{kind} val='{base}'><a:lumOff val='{offset}'/>{alpha}</a:{kind}></a:solidFill></c:spPr></c:chartSpace>");
            let doc = crate::xml::parse(xml.as_bytes()).unwrap();
            assert_eq!(read_frame_with(&doc.root, &scheme).unwrap().fill, expected);
        }
        // Derived algebra regression; preserves existing positive mod+off and
        // extends its offset operand without inventing a transform-order rule.
        assert_eq!(frame_lum([21, 96, 130], Some(100_000), Some(-20_000)), [7, 31, 42]);
    }

    #[test]
    fn luminance_offset_paints_chart_and_plot_pixels_at_three_dpis() {
        for plot in [false, true] {
            for (offset, rgb, opacity, byte_alpha) in [(40_000, [122, 199, 233], 100_000, 255u8), (-20_000, [7, 31, 42], 100_000, 255), (40_000, [122, 199, 233], 50_000, 128)] {
                let fill = format!("<c:spPr><a:solidFill><a:srgbClr val='156082'><a:lumOff val='{offset}'/><a:alpha val='{opacity}'/></a:srgbClr></a:solidFill></c:spPr>");
                let xml = if plot { LONG_LEGEND.replace("<c:plotArea>", &format!("<c:plotArea>{fill}")) } else { LONG_LEGEND.replace("<c:chart>", &format!("{fill}<c:chart>")) };
                let doc = crate::xml::parse(xml.as_bytes()).unwrap();
                for value in [96, 106, 144] {
                    let dpi = RenderDpi::from_request(serde_json::json!({"dpi":value}).as_object().unwrap()).unwrap();
                    let image = render_chart_at(&doc.root, 424, 265, FALLBACK_ACCENTS, None, ChartHost::Excel, None, "", dpi).unwrap();
                    let mut decoder = png::Decoder::new(std::io::Cursor::new(image)).read_info().unwrap();
                    let mut buffer = vec![0; decoder.output_buffer_size()];
                    let info = decoder.next_frame(&mut buffer).unwrap();
                    let stride = if info.color_type == png::ColorType::Rgba { 4 } else { 3 };
                    // Plot paint composites over the existing opaque white
                    // chart frame; chart alpha itself stays straight RGBA.
                    let (expected_rgb, expected_alpha) = if plot {
                        (rgb.map(|channel| ((u32::from(channel) * u32::from(byte_alpha) + 255 * (255 - u32::from(byte_alpha)) + 127) / 255) as u8), 255)
                    } else {
                        (rgb, byte_alpha)
                    };
                    let count = buffer[..info.buffer_size()].chunks_exact(stride).filter(|px| px[..3] == expected_rgb && (stride == 3 || px[3] == expected_alpha)).count();
                    assert!(count > 100, "plot={plot} offset={offset} alpha={opacity} dpi={value}");
                }
            }
        }
    }

    #[test]
    fn unsupported_luminance_offset_inputs_do_not_lose_semantics() {
        for transform in ["<a:lumOff/>", "<a:lumOff val='-100001'/>", "<a:lumOff val='100001'/>", "<a:lumOff val='1.5'/>", "<a:lumOff val='+20000'/>", "<a:lumOff val='40000'><a:alpha val='50000'/></a:lumOff>", "<a:lumOff val='40000'/><a:lumOff val='40000'/>", "<a:lumOff val='40000'/><a:satMod val='50000'/>"] {
            let config = format!("<c:spPr><a:solidFill><a:srgbClr val='156082'>{transform}</a:srgbClr></a:solidFill></c:spPr>");
            let doc = crate::xml::parse(LONG_LEGEND.replace("<c:chart>", &format!("{config}<c:chart>")).as_bytes()).unwrap();
            assert!(read_frame(&doc.root).is_err(), "{transform}");
        }
    }

    #[test]
    fn rectangular_chart_no_line_preserves_fill_and_alpha_to_the_edge() {
        for (fill, rgb, alpha) in [
            (
                "<a:solidFill><a:srgbClr val='EAF7F7'/></a:solidFill>",
                [234, 247, 247],
                255,
            ),
            ("<a:noFill/>", [0, 0, 0], 0),
        ] {
            let config = format!("<c:roundedCorners val='0'/><c:spPr>{fill}<a:ln w='12700' cap='flat' cmpd='sng' algn='ctr'><a:noFill/><a:prstDash val='solid'/><a:round/></a:ln><a:effectLst/><a:extLst><a:ext uri='{{91240B29-F687-4F45-9708-019B960494DF}}'><a14:hiddenLine><a:solidFill><a:srgbClr val='FF0000'/></a:solidFill></a14:hiddenLine></a:ext></a:extLst></c:spPr>");
            let xml = LONG_LEGEND.replace("<c:chart>", &format!("{config}<c:chart>"));
            let doc = crate::xml::parse(xml.as_bytes()).unwrap();
            let frame = read_frame(&doc.root).unwrap();
            assert!(!frame.border);
            for dpi_value in [96, 106, 144] {
                let dpi = RenderDpi::from_request(
                    serde_json::json!({"dpi":dpi_value}).as_object().unwrap(),
                )
                .unwrap();
                let png = render_chart_at(
                    &doc.root,
                    424,
                    265,
                    FALLBACK_ACCENTS,
                    None,
                    ChartHost::Excel,
                    None,
                    "",
                    dpi,
                )
                .unwrap();
                let mut decoder = png::Decoder::new(std::io::Cursor::new(png))
                    .read_info()
                    .unwrap();
                let mut buf = vec![0; decoder.output_buffer_size()];
                let info = decoder.next_frame(&mut buf).unwrap();
                let stride = if info.color_type == png::ColorType::Rgba {
                    4
                } else {
                    3
                };
                for (x, y) in [(0, 0), (100, 0), (423, 100), (100, 264)] {
                    let i = (y * 424 + x) * stride;
                    assert_eq!(&buf[i..i + 3], &rgb);
                    if stride == 4 {
                        assert_eq!(buf[i + 3], alpha);
                    }
                }
            }
        }
        for config in [
            "<c:roundedCorners val='1'/><c:spPr><a:ln><a:noFill/></a:ln></c:spPr>",
            "<c:roundedCorners val='0'/><c:spPr><a:ln><a:solidFill><a:srgbClr val='FF0000'/></a:solidFill></a:ln></c:spPr>",
            "<c:roundedCorners val='0'/><c:spPr><a:effectLst><a:outerShdw/></a:effectLst></c:spPr>",
        ] {
            let xml=LONG_LEGEND.replace("<c:chart>",&format!("{config}<c:chart>"));
            assert!(read_frame(&crate::xml::parse(xml.as_bytes()).unwrap().root).is_err());
        }
    }

    #[test]
    fn unsupported_frame_semantics_do_not_fall_back_to_white() {
        for config in [
            "<c:spPr><a:gradFill/></c:spPr>",
            "<c:spPr><a:gradFill flip='none' rotWithShape='1'><a:gsLst><a:gs pos='0'><a:srgbClr val='EAF7F7'/></a:gs><a:gs pos='100000'><a:srgbClr val='0F6082'/></a:gs></a:gsLst><a:lin ang='2700000' scaled='1'/><a:tileRect/></a:gradFill></c:spPr>",
            "<c:spPr><a:pattFill prst='ltHorz'><a:fgClr><a:srgbClr val='0F6082'/></a:fgClr><a:bgClr><a:srgbClr val='EAF7F7'/></a:bgClr></a:pattFill></c:spPr>",
            "<c:spPr><a:blipFill rotWithShape='1'><a:blip r:embed='rId1'/><a:stretch><a:fillRect/></a:stretch></a:blipFill></c:spPr>",
            "<c:spPr><a:solidFill><a:srgbClr val='EAF7F7'><a:alpha val='50000'/><a:alphaMod val='50000'/></a:srgbClr></a:solidFill></c:spPr>",
            "<c:spPr><a:solidFill><a:sysClr val='windowText' lastClr='FF0000'/></a:solidFill></c:spPr>",
            "<c:spPr><a:solidFill><a:sysClr val='window' lastClr='00FF00'/></a:solidFill></c:spPr>",
            "<c:spPr><a:solidFill><a:schemeClr val='dk2'/></a:solidFill></c:spPr>",
            "<c:spPr><a:solidFill><a:srgbClr val='156082'><a:satMod val='50000'/></a:srgbClr></a:solidFill></c:spPr>",
            "<c:spPr><a:solidFill><a:srgbClr val='156082'><a:hueMod val='50000'/></a:srgbClr></a:solidFill></c:spPr>",
            "<c:spPr><a:solidFill><a:srgbClr val='156082'><a:lumOff val='40000'/><a:lumMod val='60000'/></a:srgbClr></a:solidFill></c:spPr>",
            "<c:spPr><a:solidFill><a:srgbClr val='156082'><a:tint val='60000'/><a:shade val='75000'/></a:srgbClr></a:solidFill></c:spPr>",
            "<c:spPr><a:ln w='25400'/></c:spPr>",
        ] {
            let xml = LONG_LEGEND.replace("<c:chart>", &format!("{config}<c:chart>"));
            let doc = crate::xml::parse(xml.as_bytes()).unwrap();
            assert!(read_frame(&doc.root).is_err(), "unsupported frame must not silently lose semantics: {config}");
        }
    }

    #[test]
    fn rounded_frame_has_straight_alpha_not_a_white_key() {
        let mut canvas = Canvas::new(80, 60);
        canvas.fill_rect(20, 20, 30, 30, [255; 3]);
        if canvas.chart_border {
            canvas.stroke_border(BORDER);
        }
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
    <c:legend><c:legendPos val="r"/><c:overlay val="0"/></c:legend>
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
    <c:legend><c:legendPos val="r"/><c:overlay val="0"/></c:legend>
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
    fn plot_insets_and_regular_grids_match_official_106dpi_edges() {
        let dpi106 =
            RenderDpi::from_request(serde_json::json!({"dpi": 106}).as_object().unwrap()).unwrap();
        assert_eq!(scale_plot_leading(dpi106, INSET_TOP), 16);
        assert_eq!(scale_plot_trailing(dpi106, INSET_BOTTOM), 37);
        assert_eq!(scale_plot_leading(RenderDpi::DEFAULT, INSET_TOP), 14);
        assert_eq!(scale_plot_trailing(RenderDpi::DEFAULT, INSET_BOTTOM), 34);
        let dpi144 =
            RenderDpi::from_request(serde_json::json!({"dpi": 144}).as_object().unwrap()).unwrap();
        assert_eq!(scale_plot_leading(dpi144, INSET_TOP), 21);
        assert_eq!(scale_plot_trailing(dpi144, INSET_BOTTOM), 51);
        // Official untitled 636x371 / 424x265 grids at 106dpi.
        assert_eq!(
            (0..=9)
                .map(|i| regular_grid_y(16, 334, i, 9))
                .collect::<Vec<_>>(),
            vec![16, 51, 86, 122, 157, 192, 228, 263, 298, 334]
        );
        assert_eq!(regular_grid_y(16, 228, 0, 9), 16);
        assert_eq!(regular_grid_y(16, 228, 9, 9), 228);

        let series = line_ser(
            0,
            "S1",
            &[Some(4.0), Some(2.0), Some(3.0), Some(1.0)],
            r#"<c:marker><c:symbol val="none"/></c:marker>"#,
        );
        let doc = crate::xml::parse(standard_line_xml(&series, "").as_bytes()).unwrap();
        let png = render_chart_at(
            &doc.root,
            636,
            371,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
            dpi106,
        )
        .unwrap();
        let mut reader = png::Decoder::new(std::io::Cursor::new(png))
            .read_info()
            .unwrap();
        let mut buf = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).unwrap();
        let bpp = match info.color_type {
            png::ColorType::Rgb => 3usize,
            png::ColorType::Rgba => 4usize,
            other => panic!("unexpected color type {other:?}"),
        };
        let w = info.width;
        let h = info.height;
        assert_eq!((w, h), (636, 371));
        let mut rows = Vec::new();
        for y in 1..h - 1 {
            let mut ink = 0u32;
            for x in 80..400 {
                let i = ((y * w + x) * bpp as u32) as usize;
                let r = buf[i];
                let g = buf[i + 1];
                let b = buf[i + 2];
                let mx = r.max(g).max(b);
                let mn = r.min(g).min(b);
                if mx < 230 && mx - mn < 20 {
                    ink += 1;
                }
            }
            if ink >= 160 {
                rows.push(y as i32);
            }
        }
        assert_eq!(
            rows,
            vec![16, 51, 86, 122, 157, 192, 228, 263, 298, 334],
            "106dpi 636x371 major grids must sit on official rows, got {rows:?}"
        );
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
    fn empty_cache_v_plots_as_zero_and_connects_the_line() {
        let none = r#"<c:marker><c:symbol val="none"/></c:marker>"#;
        let zero = standard_line_xml(
            &line_ser(0, "S1", &[Some(20.0), Some(0.0), Some(20.0)], none),
            "",
        );
        let empty_v = standard_line_xml(
            &line_ser(0, "S1", &[Some(20.0), Some(0.0), Some(20.0)], none).replace(
                r#"<c:pt idx="1"><c:v>0</c:v></c:pt>"#,
                r#"<c:pt idx="1"><c:v></c:v></c:pt>"#,
            ),
            "",
        );
        let omitted = standard_line_xml(
            &line_ser(0, "S1", &[Some(20.0), None, Some(20.0)], none),
            "",
        );
        let zero_png = render_line(&zero).unwrap();
        let empty_png = render_line(&empty_v).unwrap();
        let omitted_png = render_line(&omitted).unwrap();
        assert_eq!(
            empty_png, zero_png,
            "an empty cache <c:v></c:v> must plot as 0 and keep the stroke connected"
        );
        assert_ne!(
            empty_png, omitted_png,
            "an omitted cache point must stay a gap, not the same as an empty <c:v>"
        );
        let (w, h, empty_rgb) = png_rgb(&empty_png);
        let (_, _, omit_rgb) = png_rgb(&omitted_png);
        let a1 = FALLBACK_ACCENTS[0];
        let mid0 = w / 3;
        let mid1 = (w * 2) / 3;
        let empty_mid = count_in(&empty_rgb, w, mid0, mid1, 0, h, a1, 24);
        let omit_mid = count_in(&omit_rgb, w, mid0, mid1, 0, h, a1, 24);
        assert!(
            empty_mid >= 10,
            "empty cache v should connect through the middle category like 0, got {empty_mid}"
        );
        assert_eq!(
            omit_mid, 0,
            "an omitted cache point must still break the stroke; got {omit_mid}"
        );
    }

    #[test]
    fn parse_chart_number_locks_empty_text_zero_and_keeps_blanks_and_errors_as_gaps() {
        assert_eq!(parse_chart_number(""), Some(0.0));
        assert_eq!(parse_chart_number("   "), Some(0.0));
        assert_eq!(parse_chart_number("20"), Some(20.0));
        assert_eq!(parse_chart_number("-1.5"), Some(-1.5));
        assert_eq!(parse_chart_number("#N/A"), None);
        assert_eq!(json_number(serde_json::json!("")), Some(0.0));
        assert_eq!(json_number(serde_json::json!(null)), None);
        assert_eq!(json_number(serde_json::Value::String("#N/A".into())), None);
        assert_eq!(json_number(serde_json::json!(20)), Some(20.0));
    }

    #[test]
    fn empty_scatter_cache_v_plots_as_zero_and_connects_the_line() {
        let none = r#"<c:marker><c:symbol val="none"/></c:marker>"#;
        let zero = scatter_xml(
            "lineMarker",
            &scatter_ser(
                0,
                "S1",
                &[Some(0.0), Some(5.0), Some(10.0)],
                &[Some(8.0), Some(0.0), Some(8.0)],
                none,
            ),
            "",
        );
        let empty_v = scatter_xml(
            "lineMarker",
            &scatter_ser(
                0,
                "S1",
                &[Some(0.0), Some(5.0), Some(10.0)],
                &[Some(8.0), Some(0.0), Some(8.0)],
                none,
            )
            .replace(
                r#"<c:pt idx="1"><c:v>0</c:v></c:pt>"#,
                r#"<c:pt idx="1"><c:v></c:v></c:pt>"#,
            ),
            "",
        );
        let omitted = scatter_xml(
            "lineMarker",
            &scatter_ser(
                0,
                "S1",
                &[Some(0.0), Some(5.0), Some(10.0)],
                &[Some(8.0), None, Some(8.0)],
                none,
            ),
            "",
        );
        let zero_png = render_scatter(&zero).unwrap();
        let empty_png = render_scatter(&empty_v).unwrap();
        assert_eq!(
            empty_png, zero_png,
            "an empty scatter cache <c:v></c:v> must plot as 0 and keep the stroke connected"
        );
        assert_ne!(
            empty_png,
            render_scatter(&omitted).unwrap(),
            "an omitted scatter cache point must stay a gap"
        );
    }

    #[test]
    fn signed_scatter_auto_zero_axes_and_labels_meet_at_origin() {
        let none = r#"<c:marker><c:symbol val="none"/></c:marker>"#;
        let signed = scatter_xml(
            "lineMarker",
            &scatter_ser(
                0,
                "S1",
                &[Some(-2.0), Some(-1.0), Some(1.0), Some(3.0)],
                &[Some(-1.0), Some(2.0), Some(-2.0), Some(3.0)],
                none,
            ),
            "",
        );
        let (w, h, rgb) = png_rgb(&render_scatter(&signed).unwrap());
        assert_eq!((w, h), (576, 336));
        let dark = [0_u8, 0, 0];
        let mut col = vec![0_u32; w as usize];
        let mut row = vec![0_u32; h as usize];
        for y in 0..h {
            for x in 0..w {
                if near(
                    [
                        rgb[((y * w + x) * 3) as usize],
                        rgb[((y * w + x) * 3 + 1) as usize],
                        rgb[((y * w + x) * 3 + 2) as usize],
                    ],
                    dark,
                    40,
                ) {
                    col[x as usize] += 1;
                    row[y as usize] += 1;
                }
            }
        }
        let axis_x = col
            .iter()
            .enumerate()
            .max_by_key(|(_, n)| *n)
            .map(|(x, _)| x as u32)
            .unwrap();
        assert!(
            axis_x > w / 4 && axis_x < (w * 3) / 4,
            "vertical autoZero axis should sit at the origin, not the left edge; x={axis_x}"
        );
        let mid_axis_y = (h / 4..(h * 3) / 4)
            .max_by_key(|&y| row[y as usize])
            .unwrap();
        assert!(
            row[mid_axis_y as usize] >= w / 2,
            "a full-width horizontal autoZero axis should sit at the origin, not only at the bottom; y={mid_axis_y} count={}",
            row[mid_axis_y as usize]
        );
        let leftover_gutter = count_in(&rgb, w, 0, 36, 0, h / 3, dark, 40);
        let origin_y_labels = count_in(&rgb, w, axis_x - 36, axis_x, 0, h / 3, dark, 40);
        assert!(
            origin_y_labels > leftover_gutter,
            "Y tick labels should sit left of the origin axis, not in the left gutter; origin={origin_y_labels} gutter={leftover_gutter}"
        );

        let positive = scatter_xml(
            "lineMarker",
            &scatter_ser(
                0,
                "S1",
                &[Some(0.0), Some(4.0), Some(8.0)],
                &[Some(1.0), Some(3.0), Some(2.0)],
                none,
            ),
            "",
        );
        let (_, _, pos_rgb) = png_rgb(&render_scatter(&positive).unwrap());
        let mut pos_col = vec![0_u32; w as usize];
        for y in 0..h {
            for x in 0..w {
                if near(
                    [
                        pos_rgb[((y * w + x) * 3) as usize],
                        pos_rgb[((y * w + x) * 3 + 1) as usize],
                        pos_rgb[((y * w + x) * 3 + 2) as usize],
                    ],
                    dark,
                    40,
                ) {
                    pos_col[x as usize] += 1;
                }
            }
        }
        let pos_axis_x = pos_col
            .iter()
            .enumerate()
            .max_by_key(|(_, n)| *n)
            .map(|(x, _)| x as u32)
            .unwrap();
        assert!(
            pos_axis_x < w / 4,
            "when 0 is on the left bound, the vertical axis should stay on the left edge; x={pos_axis_x}"
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
    fn non_bubble_flag_is_inert_but_real_label_semantics_are_not() {
        let base = standard_line_xml(&line_ser(0, "S1", &[Some(1.0), Some(2.0)], ""), "");
        let bubble = base.replace(
            "<c:varyColors",
            "<c:dLbls><c:showBubbleSize val='1'/></c:dLbls><c:varyColors",
        );
        assert_eq!(render_line(&base).unwrap(), render_line(&bubble).unwrap());
        for unsupported in [
            "<c:showCatName val='1'/>",
            "<c:showPercent val='1'/>",
            "<c:showLegendKey val='1'/>",
            "<c:dLbl><c:idx val='0'/><c:delete val='1'/></c:dLbl>",
        ] {
            let xml = base.replace(
                "<c:varyColors",
                &format!("<c:dLbls>{unsupported}</c:dLbls><c:varyColors"),
            );
            assert!(
                render_line(&xml).is_err(),
                "must not silently omit {unsupported}"
            );
        }
    }

    #[test]
    fn line_zero_span_and_na_are_distinct_in_values_and_actual_strokes() {
        let ser = line_ser(
            0,
            "S1",
            &[Some(1.0), None, Some(3.0)],
            "<c:marker><c:symbol val='none'/></c:marker>",
        );
        let plot = Rect {
            x: 0.0,
            y: 0.0,
            w: 299.0,
            h: 100.0,
        };
        let parse = |mode: &str, ser: &str| {
            let xml = standard_line_xml(ser, &format!("<c:dispBlanksAs val='{mode}'/>"));
            let doc = crate::xml::parse(xml.as_bytes()).unwrap();
            read_chart(&doc.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT).unwrap()
        };
        let mut canvases = Vec::new();
        for mode in ["gap", "zero", "span"] {
            let chart = parse(mode, &ser);
            assert_eq!(
                chart.series[0].values[1],
                if mode == "zero" { Some(0.0) } else { None }
            );
            let mut canvas = Canvas::new(300, 110);
            draw_lines(&mut canvas, &chart, &plot, 0.0, 4.0, None, false, None).unwrap();
            canvases.push(canvas);
        }
        let ink = |canvas: &Canvas, x: usize, y: usize| {
            canvas.px[(y * 300 + x) * 3..(y * 300 + x) * 3 + 3] != [255; 3]
        };
        assert!(!ink(&canvases[0], 150, 50), "gap must not connect");
        assert!(
            ink(&canvases[1], 150, 99),
            "zero must draw down to baseline"
        );
        assert!(
            ink(&canvases[2], 150, 50),
            "span must actually connect across missing point"
        );
        let na = ser.replace(
            "</c:numLit>",
            "<c:pt idx='1'><c:v>#N/A</c:v></c:pt></c:numLit>",
        );
        for mode in ["gap", "zero", "span"] {
            let chart = parse(mode, &na);
            assert_eq!(chart.series[0].values[1], None, "NA is never a zero point");
            assert!(chart.series[0].unavailable_points[1]);
            let mut canvas = Canvas::new(300, 110);
            draw_lines(&mut canvas, &chart, &plot, 0.0, 4.0, None, false, None).unwrap();
            assert!(
                ink(&canvas, 150, 50),
                "Excel16 NA probe connects across NA: {mode}"
            );
        }
        let error = na.replace("#N/A", "#DIV/0!");
        for mode in ["zero", "span"] {
            let xml = standard_line_xml(&error, &format!("<c:dispBlanksAs val='{mode}'/>"));
            assert!(
                render_line(&xml).is_err(),
                "unverified error semantics must not silently be a blank/zero"
            );
        }
    }

    #[test]
    fn na_as_blank_extension_is_not_silently_treated_as_ordinary_na() {
        let base = standard_line_xml(
            &line_ser(0, "S1", &[Some(1.0), Some(2.0), Some(3.0)], ""),
            "<c:dispBlanksAs val='zero'/>",
        );
        let finite=base.replace("</c:chart>","<c:extLst><c:ext uri='dataDisplayOptions16'><dispNaAsBlank xmlns='http://schemas.microsoft.com/office/drawing/2017/03/chart' val='1'/></c:ext></c:extLst></c:chart>");
        assert!(
            render_line(&finite).is_ok(),
            "inactive option must not be a false export failure"
        );
        let with_na = finite.replace(
            "<c:pt idx=\"1\"><c:v>2</c:v></c:pt>",
            "<c:pt idx=\"1\"><c:v>#N/A</c:v></c:pt>",
        );
        assert_ne!(with_na, finite, "counterexample must really contain NA");
        assert!(
            render_line(&with_na).is_err(),
            "active NA-as-blank has different valid semantics still needing implementation"
        );
        assert!(render_line(&with_na.replace("val='1'", "val='0'")).is_ok());
    }

    #[test]
    fn sparse_unresolved_cache_domain_is_not_silently_shortened_or_allocated_unbounded() {
        let ser = line_ser(0, "S1", &[Some(1.0), None, Some(3.0)], "");
        for count in [5_u64, 4_000_000_000_u64] {
            let ser = ser.replace(
                "<c:numLit>",
                &format!("<c:numLit><c:ptCount val='{count}'/>"),
            );
            for mode in ["gap", "zero", "span"] {
                let xml = standard_line_xml(&ser, &format!("<c:dispBlanksAs val='{mode}'/>"));
                assert!(render_line(&xml).is_err(),"unresolved domain must not lose trailing slots or allocate by untrusted ptCount");
            }
        }
    }

    #[test]
    fn line_blank_modes_keep_trailing_categories_and_cell_error_identity() {
        let ser = line_ser(0, "S1", &[Some(1.0), None, Some(3.0), None], "");
        let xml = standard_line_xml(&ser, "<c:dispBlanksAs val='zero'/>");
        let doc = crate::xml::parse(xml.as_bytes()).unwrap();
        let chart = read_chart(&doc.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT).unwrap();
        assert_eq!(
            chart.series[0].values,
            vec![Some(1.0), Some(0.0), Some(3.0), Some(0.0)]
        );
        let cells = super::super::sparkline::CellCache::from_pairs(
            "Data",
            &[
                ((2, 1), serde_json::json!(1)),
                ((2, 3), serde_json::json!("#N/A")),
                ((2, 4), serde_json::json!(4)),
            ],
        );
        let parent =
            crate::xml::parse(b"<val><numRef><f>Data!B1:B4</f><numCache/></numRef></val>").unwrap();
        let (errors, na) = series_error_points(Some(&parent.root), "Data", Some(&cells), 4);
        assert_eq!(errors, vec![false, false, true, false]);
        assert_eq!(na, vec![false, false, true, false]);
    }

    #[test]
    fn point_label_deletion_only_validates_indices_and_preserves_ni() {
        let valid = "<dLbl><idx val='1'/><delete val='1'/></dLbl>";
        let read = |body: &str, len| {
            let doc = crate::xml::parse(format!("<dLbls>{body}</dLbls>").as_bytes()).unwrap();
            read_deleted_label_points(Some(&doc.root), len)
        };
        assert_eq!(read(valid, 3).unwrap(), vec![1]);
        assert_eq!(
            read(&valid.replace("val='1'/></dLbl>", "/></dLbl>"), 3).unwrap(),
            vec![1]
        );
        let metadata = valid.replace("</dLbl>", "<extLst><ext uri='{CE6537A1-D6FC-4f65-9D91-7224C49458BB}'/><ext uri='{C3380CC4-5D6E-409C-BE32-E72D297353CC}'><uniqueId val='{00000001-2765-4173-A018-3A6CF43989F2}'/></ext></extLst></dLbl>");
        assert_eq!(read(&metadata, 3).unwrap(), vec![1]);
        for bad in [
            valid.replace("idx val='1'", "idx val='-1'"),
            valid.replace("idx val='1'", "idx val='3'"),
            valid.replace("idx val='1'", "idx val='1.5'"),
            valid.replace("idx val='1'", "idx val=''"),
            valid.replace("idx val='1'", "idx val='184467440737095516160'"),
            valid.replace("<idx val='1'/>", ""),
            valid.replace("<delete val='1'/>", ""),
            valid.replace("delete val='1'", "delete val='0'"),
            valid.replace("</dLbl>", "<showVal val='1'/></dLbl>"),
            valid.replace("</dLbl>", "<spPr><solidFill/></spPr></dLbl>"),
            valid.replace("</dLbl>", "<dLblPos val='outEnd'/></dLbl>"),
            valid.replace("</dLbl>", "<tx><rich/></tx></dLbl>"),
            metadata.replace("uniqueId", "showLeaderLines"),
            metadata.replace("{CE6537A1-D6FC-4f65-9D91-7224C49458BB}", "unknown"),
            format!("{valid}{valid}"),
        ] {
            assert_eq!(
                read(&bad, 3).unwrap_err().code(),
                crate::error::ErrorCode::NotImplemented,
                "{bad}"
            );
        }
        let wrapper = crate::xml::parse(b"<wrapper/>").unwrap();
        let ser = crate::xml::parse(
            format!("<ser><dLbls>{valid}<showVal val='1'/></dLbls></ser>").as_bytes(),
        )
        .unwrap();
        assert_eq!(
            read_value_label(&wrapper.root, &ser.root, PlotKind::ClusteredColumn).unwrap(),
            Some(ValueLabel::OutsideEnd)
        );
        assert!(
            read_value_label(&ser.root, &wrapper.root, PlotKind::ClusteredColumn).is_err(),
            "wrapper-level per-point overrides remain NI"
        );
        let other_kind = crate::xml::parse(format!("<dLbls>{valid}</dLbls>").as_bytes()).unwrap();
        assert!(
            data_labels_hidden(Some(&other_kind.root)).is_err(),
            "other chart kinds remain NI"
        );
    }

    #[cfg(windows)]
    #[test]
    fn deleted_point_labels_remove_only_the_requested_glyphs() {
        for dpi_value in [96, 106, 144] {
            let dpi =
                RenderDpi::from_request(serde_json::json!({"dpi":dpi_value}).as_object().unwrap())
                    .unwrap();
            for kind in [
                PlotKind::ClusteredColumn,
                PlotKind::ClusteredBar,
                PlotKind::StandardLine,
            ] {
                let pos = if kind == PlotKind::StandardLine {
                    "r"
                } else {
                    "outEnd"
                };
                let labels =
                    format!("<c:dLbls><c:showVal val='1'/><c:dLblPos val='{pos}'/></c:dLbls>");
                let ser = line_ser(0, "S1", &[Some(4.0), Some(6.0), Some(9.0)], &labels);
                let base = match kind {
                    PlotKind::StandardLine => {
                        standard_line_xml(&ser, "<c:dispBlanksAs val='gap'/>")
                    }
                    PlotKind::ClusteredColumn => bar_xml(&ser, "", "minMax", Some("col")),
                    PlotKind::ClusteredBar => bar_xml(&ser, "", "minMax", Some("bar")),
                    _ => unreachable!(),
                };
                let deleted = base.replace(
                    "<c:dLbls>",
                    "<c:dLbls><c:dLbl><c:idx val='1'/><c:delete val='1'/></c:dLbl>",
                );
                let control_doc = crate::xml::parse(base.as_bytes()).unwrap();
                let deleted_doc = crate::xml::parse(deleted.as_bytes()).unwrap();
                let control_chart =
                    read_chart(&control_doc.root, FALLBACK_ACCENTS, None, "", dpi).unwrap();
                let deleted_chart =
                    read_chart(&deleted_doc.root, FALLBACK_ACCENTS, None, "", dpi).unwrap();
                assert_eq!(deleted_chart.series[0].deleted_label_points, vec![1]);
                assert_eq!(
                    deleted_chart.series[0].values, control_chart.series[0].values,
                    "deleting labels must not delete chart marks"
                );
                let face = super::super::chart_gdi::Face::open(14.0, false).unwrap();
                let plot = Rect {
                    x: 100.0,
                    y: 50.0,
                    w: 300.0,
                    h: 220.0,
                };
                let mut control = Canvas::with_dpi(576, 336, dpi);
                let mut actual = Canvas::with_dpi(576, 336, dpi);
                for (canvas, chart) in [
                    (&mut control, &control_chart),
                    (&mut actual, &deleted_chart),
                ] {
                    if kind == PlotKind::ClusteredBar {
                        draw_horizontal_value_labels(
                            canvas,
                            chart,
                            &plot,
                            0.0,
                            10.0,
                            None,
                            Some(&face),
                            14.0,
                            false,
                        )
                        .unwrap();
                    } else {
                        draw_value_labels(canvas, chart, &plot, 0.0, 10.0, None, Some(&face), 14.0)
                            .unwrap();
                    }
                }
                let mut removed = 0;
                let mut kept = 0;
                for y in 0..336 {
                    for x in 0..576 {
                        let i = (y * 576 + x) * 3;
                        let before = &control.px[i..i + 3];
                        let after = &actual.px[i..i + 3];
                        let middle = if kind == PlotKind::ClusteredBar {
                            (270..330).contains(&x) && (140..180).contains(&y)
                        } else {
                            (225..285).contains(&x) && (100..155).contains(&y)
                        };
                        if middle {
                            assert_eq!(after, [255; 3], "deleted glyph must disappear");
                            removed += usize::from(before != after);
                        } else {
                            assert_eq!(before, after, "unrelated point labels must not move");
                            kept += usize::from(after != [255; 3]);
                        }
                    }
                }
                assert!(
                    removed > 0 && kept > 0,
                    "both real deleted and retained glyphs required at {dpi_value}"
                );
            }
        }
    }

    #[test]
    fn inside_end_is_column_only_and_zero_is_explicit_ni() {
        let wrapper = crate::xml::parse(b"<wrapper/>").unwrap();
        let ser = crate::xml::parse(
            b"<ser><dLbls><showVal val='1'/><dLblPos val='inEnd'/></dLbls></ser>",
        )
        .unwrap();
        assert_eq!(
            read_value_label(&wrapper.root, &ser.root, PlotKind::ClusteredColumn).unwrap(),
            Some(ValueLabel::InsideEnd)
        );
        for kind in [PlotKind::ClusteredBar, PlotKind::StandardLine] {
            assert!(read_value_label(&wrapper.root, &ser.root, kind).is_err());
        }
        let ser = line_ser(
            0,
            "S1",
            &[Some(0.0), Some(6.0), Some(9.0)],
            "<c:dLbls><c:showVal val='1'/><c:dLblPos val='inEnd'/></c:dLbls>",
        );
        let xml = bar_xml(&ser, "", "minMax", Some("col"));
        let doc = crate::xml::parse(xml.as_bytes()).unwrap();
        assert!(
            read_chart(&doc.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT).is_err(),
            "zero fallback placement is not silently approximated"
        );
    }

    #[cfg(windows)]
    #[test]
    fn column_inside_end_labels_paint_inside_marks_and_reject_nonfitting_boxes() {
        for dpi_value in [96, 106, 144] {
            let dpi =
                RenderDpi::from_request(serde_json::json!({"dpi":dpi_value}).as_object().unwrap())
                    .unwrap();
            for sign in [-1.0, 1.0] {
                let values = [Some(4.0 * sign), Some(6.0 * sign), Some(9.0 * sign)];
                let ser = line_ser(
                    0,
                    "S1",
                    &values,
                    "<c:dLbls><c:showVal val='1'/><c:dLblPos val='inEnd'/></c:dLbls>",
                );
                let xml = bar_xml(&ser, "", "minMax", Some("col"));
                let doc = crate::xml::parse(xml.as_bytes()).unwrap();
                let chart = read_chart(&doc.root, FALLBACK_ACCENTS, None, "", dpi).unwrap();
                let face = super::super::chart_gdi::Face::open(14.0, false).unwrap();
                let plot = Rect {
                    x: 80.0,
                    y: 30.0,
                    w: 400.0,
                    h: 280.0,
                };
                let (lo, hi) = if sign < 0.0 {
                    (-10.0, 0.0)
                } else {
                    (0.0, 10.0)
                };
                let mut canvas = Canvas::with_dpi(576, 336, dpi);
                draw_value_labels(&mut canvas, &chart, &plot, lo, hi, None, Some(&face), 14.0)
                    .unwrap();
                let slot = (plot.w + 1.0) / 3.0;
                let mark_width = slot / (1.0 + chart.gap_width / 100.0);
                let baseline = y_of(0.0, lo, hi, &plot);
                let mut ink = [0; 3];
                for y in 0..336 {
                    for x in 0..576 {
                        let i = (y * 576 + x) * 3;
                        if canvas.px[i..i + 3] == [255; 3] {
                            continue;
                        }
                        let mut found = false;
                        for ci in 0..3 {
                            let center = plot.x + slot * (ci as f64 + 0.5);
                            let end = y_of(values[ci].unwrap(), lo, hi, &plot);
                            if (x as f64 - center).abs() <= mark_width / 2.0
                                && (y as f64) >= baseline.min(end)
                                && (y as f64) <= baseline.max(end)
                            {
                                ink[ci] += 1;
                                found = true;
                            }
                        }
                        assert!(found, "inside-end glyph escaped every mark");
                    }
                }
                assert!(
                    ink.into_iter().all(|n| n > 0),
                    "every value must paint real glyph ink"
                );
                for small in [
                    Rect {
                        x: 80.0,
                        y: 30.0,
                        w: 20.0,
                        h: 280.0,
                    },
                    Rect {
                        x: 80.0,
                        y: 30.0,
                        w: 400.0,
                        h: 20.0,
                    },
                ] {
                    let mut canvas = Canvas::with_dpi(576, 336, dpi);
                    assert!(
                        draw_value_labels(
                            &mut canvas,
                            &chart,
                            &small,
                            lo,
                            hi,
                            None,
                            Some(&face),
                            14.0
                        )
                        .is_err(),
                        "short/narrow boxes remain NI instead of clipped/fitted"
                    );
                }
            }
        }
    }

    #[test]
    fn plain_value_labels_are_parsed_and_unsupported_styles_are_never_omitted() {
        for kind in [
            PlotKind::ClusteredColumn,
            PlotKind::ClusteredBar,
            PlotKind::StandardLine,
        ] {
            let wrapper = crate::xml::parse(
                b"<wrapper><dLbls><showVal val='1'/><showBubbleSize val='1'/></dLbls></wrapper>",
            )
            .unwrap();
            let ser = crate::xml::parse(b"<ser/>").unwrap();
            assert_eq!(
                read_value_label(&wrapper.root, &ser.root, kind).unwrap(),
                Some(if kind == PlotKind::StandardLine {
                    ValueLabel::Right
                } else {
                    ValueLabel::OutsideEnd
                })
            );
            for config in [
                "<showCatName val='1'/>",
                "<numFmt formatCode='0.00'/>",
                "<dLblPos val='bestFit'/>",
                "<txPr/>",
                "<dLbl><idx val='0'/></dLbl>",
                "<spPr><solidFill/></spPr>",
            ] {
                let ser = crate::xml::parse(
                    format!("<ser><dLbls><showVal val='1'/>{config}</dLbls></ser>").as_bytes(),
                )
                .unwrap();
                assert!(
                    read_value_label(&wrapper.root, &ser.root, kind).is_err(),
                    "must not ignore {config}"
                );
            }
        }
    }

    #[cfg(windows)]
    #[test]
    fn horizontal_outside_end_labels_paint_and_keep_other_subranges_ni() {
        let base = standard_line_xml(
            &line_ser(
                0,
                "S1",
                &[Some(-4.0), Some(0.0), Some(9.0)],
                "<c:dLbls><c:showVal val='1'/><c:dLblPos val='outEnd'/></c:dLbls>",
            ),
            "<c:dispBlanksAs val='gap'/>",
        )
        .replace("lineChart", "barChart")
        .replace(
            "<c:grouping val=\"standard\"/>",
            "<c:barDir val='bar'/><c:grouping val='clustered'/>",
        );
        for dpi_value in [96, 106, 144] {
            let dpi =
                RenderDpi::from_request(serde_json::json!({"dpi":dpi_value}).as_object().unwrap())
                    .unwrap();
            for orient in ["minMax", "maxMin"] {
                let xml = base.replace(
                    "<c:catAx/>",
                    &format!("<c:catAx><c:axPos val='l'/><c:scaling><c:orientation val='{orient}'/></c:scaling></c:catAx>"),
                );
                let xml = xml.replace("<c:valAx>", "<c:valAx><c:axPos val='b'/>");
                let doc = crate::xml::parse(xml.as_bytes()).unwrap();
                let chart = read_chart(&doc.root, FALLBACK_ACCENTS, None, "", dpi).unwrap();
                assert_eq!(chart.series[0].value_label, Some(ValueLabel::OutsideEnd));
                let mut actual = Canvas::with_dpi(576, 336, dpi);
                let face = super::super::chart_gdi::Face::open(14.0, false).unwrap();
                let plot = Rect {
                    x: 100.0,
                    y: 50.0,
                    w: 300.0,
                    h: 220.0,
                };
                draw_horizontal_value_labels(
                    &mut actual,
                    &chart,
                    &plot,
                    -10.0,
                    10.0,
                    None,
                    Some(&face),
                    14.0,
                    false,
                )
                .unwrap();
                // Left of the negative end, right of zero, and right of positive
                // end must each contain actual glyph ink (not success + omission).
                let pad = dpi.pt_to_px_font(6.0);
                for (ci, v) in [-4.0, 0.0, 9.0].into_iter().enumerate() {
                    let end = x_of(v, -10.0, 10.0, &plot);
                    let y = category_mid_y(&plot, ci, 3, chart.cats_from_bottom);
                    let xr = if v < 0.0 {
                        (end - pad - 25.0) as i32..(end - pad + 1.0) as i32
                    } else {
                        (end + pad) as i32..(end + pad + 20.0) as i32
                    };
                    let mut ink = 0;
                    for yy in (y - 16.0) as i32..(y + 16.0) as i32 {
                        for xx in xr.clone() {
                            if xx >= 0 && yy >= 0 && xx < 576 && yy < 336 {
                                let i = (yy as usize * 576 + xx as usize) * 3;
                                ink += usize::from(actual.px[i..i + 3] != [255; 3]);
                            }
                        }
                    }
                    assert!(ink > 0, "missing value {v} at {dpi_value}/{orient}");
                }
            }
        }
        for xml in [
            base.replace("val='outEnd'", "val='inEnd'"),
            base.replace("<c:v>9</c:v>", "<c:v>9.5</c:v>"),
            base.replace("<c:v>9</c:v>", "<c:v>#N/A</c:v>"),
        ] {
            let xml = xml
                .replace("<c:valAx>", "<c:valAx><c:axPos val='b'/>")
                .replace("<c:catAx/>", "<c:catAx><c:axPos val='l'/></c:catAx>");
            assert!(
                render_line(&xml).is_err(),
                "unlocked bar labels must remain NI"
            );
        }
    }

    #[test]
    fn value_label_missing_and_error_semantics_are_never_silently_omitted() {
        let base = standard_line_xml(
            &line_ser(
                0,
                "S1",
                &[Some(1.0), None, Some(3.0)],
                "<c:dLbls><c:showVal val='1'/></c:dLbls>",
            ),
            "<c:dispBlanksAs val='zero'/>",
        );
        let column = base.replace("lineChart", "barChart").replace(
            "<c:grouping val=\"standard\"/>",
            "<c:barDir val='col'/><c:grouping val='clustered'/>",
        );
        assert!(
            render_line(&column).is_err(),
            "unlocked column blank labels must not silently disappear"
        );
        let error = base
            .replace(
                "</c:numLit>",
                "<c:pt idx='1'><c:v>#DIV/0!</c:v></c:pt></c:numLit>",
            )
            .replace("val='zero'", "val='gap'");
        assert!(
            render_line(&error).is_err(),
            "unsupported error label semantics are not a successful omission"
        );
    }

    #[test]
    fn label_inheritance_and_integer_format_boundary_are_explicit() {
        let root =
            crate::xml::parse(b"<wrapper><dLbls><showVal val='0'/><txPr/></dLbls></wrapper>")
                .unwrap();
        let ser = crate::xml::parse(b"<ser><dLbls><showVal val='1'/></dLbls></ser>").unwrap();
        assert!(
            read_value_label(&root.root, &ser.root, PlotKind::StandardLine).is_err(),
            "must not silently drop inherited custom text styling"
        );
        assert_eq!(value_label_text(-0.0), Some("0".to_owned()));
        assert_eq!(value_label_text(-42.0), Some("-42".to_owned()));
        for value in [1.25, f64::INFINITY, 100_000_000_000.0] {
            assert!(value_label_text(value).is_none());
        }
        let xml = standard_line_xml(
            &line_ser(
                0,
                "S1",
                &[Some(1.25), Some(2.0)],
                "<c:dLbls><c:showVal val='1'/></c:dLbls>",
            ),
            "",
        );
        assert!(
            render_line(&xml).is_err(),
            "decimal General labels need an actual formatter, not an approximate integer"
        );
    }

    #[cfg(windows)]
    #[test]
    fn value_labels_use_actual_gdi_line_box_not_fallback_font_leading() {
        let face = super::super::chart_gdi::Face::open_dpi(15.0, false, 106.0).unwrap();
        assert_eq!(value_label_height(None, Some(&face), 50.0), face.em_box());
        assert_ne!(
            value_label_height(None, Some(&face), 50.0),
            text_height(None, 50.0)
        );
    }

    #[cfg(windows)]
    #[test]
    fn real_value_label_glyph_pixels_are_added_for_column_and_line_at_each_dpi() {
        let base = standard_line_xml(
            &line_ser(0, "S1", &[Some(1.0), Some(2.0), Some(3.0)], ""),
            "",
        );
        for kind in [PlotKind::ClusteredColumn, PlotKind::StandardLine] {
            let hidden = if kind == PlotKind::ClusteredColumn {
                base.replace("lineChart", "barChart").replace(
                    "<c:grouping val=\"standard\"/>",
                    "<c:barDir val='col'/><c:grouping val='clustered'/>",
                )
            } else {
                base.clone()
            };
            let visible = hidden.replace(
                "<c:order",
                "<c:dLbls><c:showVal val='1'/></c:dLbls><c:order",
            );
            for dpi_value in [96, 106, 144] {
                let dpi = RenderDpi::from_request(
                    serde_json::json!({"dpi":dpi_value}).as_object().unwrap(),
                )
                .unwrap();
                let mut images = Vec::new();
                for xml in [&hidden, &visible] {
                    let doc = crate::xml::parse(xml.as_bytes()).unwrap();
                    let png = render_chart_at(
                        &doc.root,
                        636,
                        371,
                        FALLBACK_ACCENTS,
                        None,
                        ChartHost::Excel,
                        None,
                        "",
                        dpi,
                    )
                    .unwrap();
                    let mut reader = png::Decoder::new(std::io::Cursor::new(png))
                        .read_info()
                        .unwrap();
                    let mut bytes = vec![0; reader.output_buffer_size()];
                    reader.next_frame(&mut bytes).unwrap();
                    images.push(bytes);
                }
                assert!(
                    images[0]
                        .iter()
                        .zip(&images[1])
                        .filter(|(a, b)| a != b)
                        .count()
                        > 30,
                    "labels must add actual glyph pixels, not just remove NI"
                );
            }
        }
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
    fn scatter_gap_breaks_the_stroke_and_hidden_line_drops_the_stroke() {
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
                none,
            ),
            "",
        );
        let marker_png = render_scatter(&markers).unwrap();
        assert_eq!(
            marker_png,
            render_scatter(&solid).unwrap(),
            "scatterStyle=marker with no series line hide must connect like lineMarker"
        );

        let no_line = scatter_xml(
            "marker",
            &scatter_ser(
                0,
                "S1",
                &[Some(0.0), Some(5.0), Some(10.0)],
                &[Some(8.0), Some(8.0), Some(8.0)],
                r#"<c:spPr><a:ln><a:noFill/></a:ln></c:spPr><c:marker><c:symbol val="circle"/><c:size val="9"/></c:marker>"#,
            ),
            "",
        );
        let (_, _, hidden_rgb) = png_rgb(&render_scatter(&no_line).unwrap());
        let between = count_in(&hidden_rgb, 576, 576 / 6, mid0 - 10, 0, 336, a1, 18);
        let marker_mid = count_in(&hidden_rgb, 576, mid0 + 20, mid1 - 20, 0, 336, a1, 18);
        assert_eq!(
            between, 0,
            "an explicit series a:ln/a:noFill must drop the connecting stroke; got {between} (mid markers {marker_mid})"
        );
        assert!(
            marker_mid >= 1,
            "hiding the scatter line should still leave the middle marker, got {marker_mid}"
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
            scatter_xml("none", &ser, ""),
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

        // Official Excel 16 scatter valAx writes midCat. Locked styles still
        // paint; this is not the category-slot midCat rule.
        let midcat = scatter_xml("lineMarker", &ser, "").replace(
            "</c:valAx>",
            r#"<c:crossBetween val="midCat"/></c:valAx>"#,
        );
        assert_eq!(
            midcat.matches(r#"<c:crossBetween val="midCat"/>"#).count(),
            2,
            "both official scatter valAx should carry midCat",
        );
        render_scatter(&midcat).unwrap();
        let line_no_markers = scatter_xml(
            "lineMarker",
            &scatter_ser(
                0,
                "S1",
                &[Some(1.0), Some(2.0), Some(3.0)],
                &[Some(4.0), Some(5.0), Some(6.0)],
                r#"<c:marker><c:symbol val="none"/></c:marker><c:smooth val="0"/>"#,
            ),
            "",
        )
        .replace(
            "</c:valAx>",
            r#"<c:crossBetween val="midCat"/></c:valAx>"#,
        );
        render_scatter(&line_no_markers).unwrap();
        let markers_only = scatter_xml(
            "lineMarker",
            &scatter_ser(
                0,
                "S1",
                &[Some(1.0), Some(2.0), Some(3.0)],
                &[Some(4.0), Some(5.0), Some(6.0)],
                r#"<c:spPr><a:ln w="38100"><a:noFill/></a:ln></c:spPr><c:smooth val="0"/>"#,
            ),
            "",
        )
        .replace(
            "</c:valAx>",
            r#"<c:crossBetween val="midCat"/></c:valAx>"#,
        );
        render_scatter(&markers_only).unwrap();
        let mixed = scatter_xml("lineMarker", &ser, "").replacen(
            "</c:valAx>",
            r#"<c:crossBetween val="midCat"/></c:valAx>"#,
            1,
        );
        assert!(
            render_scatter(&mixed).is_err(),
            "mixed midCat/between scatter valAx stays NI"
        );
        let smooth_midcat = scatter_xml("smoothMarker", &ser, "").replace(
            "</c:valAx>",
            r#"<c:crossBetween val="midCat"/></c:valAx>"#,
        );
        assert!(
            render_scatter(&smooth_midcat).is_err(),
            "smoothMarker stays NI even with official midCat"
        );
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
    <c:legend><c:legendPos val="r"/><c:overlay val="0"/></c:legend>
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
    fn clustered_bar_legend_key_is_a_filled_swatch_not_a_short_line() {
        // Excel 16.0 Chart.Export and PowerPoint Slide.Export both use a
        // filled swatch for clustered bars, just as for columns and areas.
        let color = [0x15, 0x60, 0x82];
        let mut actual = Canvas::new(24, 24);
        draw_plot_key(
            &mut actual,
            PlotKind::ClusteredBar,
            5,
            5,
            13,
            13,
            color,
            MarkerKind::None,
        );
        let mut expected = Canvas::new(24, 24);
        expected.fill_rect(5, 5, 13, 13, color);
        assert!(
            actual.px == expected.px,
            "bar legend keys must fill the swatch"
        );

        let ser = line_ser(0, "S1", &[Some(8.0), Some(1.0)], "");
        for orientation in ["maxMin", "minMax"] {
            let png = render_bar(&bar_xml(&ser, "", orientation, Some("bar"))).unwrap();
            let (w, h, rgb) = png_rgb(&png);
            let mut xs = Vec::new();
            let mut ys = Vec::new();
            for y in 0..h {
                for x in w - 55..w - 5 {
                    if pixel_at(&rgb, w, x, y) == color {
                        xs.push(x);
                        ys.push(y);
                    }
                }
            }
            assert!(!xs.is_empty(), "legend swatch must exist");
            let width = xs.iter().max().unwrap() - xs.iter().min().unwrap() + 1;
            let height = ys.iter().max().unwrap() - ys.iter().min().unwrap() + 1;
            assert_eq!((width, height), (8, 8), "filled bar swatch {orientation}");
            assert_eq!(xs.len(), 64, "no hollow or line-shaped key");
        }
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

    fn bar_dark_band(rgb: &[u8], w: u32, x0: u32, x1: u32, y0: u32, y1: u32) -> u32 {
        let mut n = 0u32;
        for y in y0..y1 {
            for x in x0..x1 {
                let i = ((y * w + x) * 3) as usize;
                if rgb[i] < 40 && rgb[i + 1] < 40 && rgb[i + 2] < 40 {
                    n += 1;
                }
            }
        }
        n
    }

    #[test]
    fn signed_bar_category_labels_follow_the_zero_crossing_axis() {
        let series = line_ser(0, "S1", &[Some(-8.0), Some(0.0), Some(8.0)], "");
        let (w, h, rgb) =
            png_rgb(&render_bar(&bar_xml(&series, "", "maxMin", Some("bar"))).unwrap());
        // Official signed-bar chart3 puts A/B/C immediately left of the
        // vertical zero axis. B must not remain stranded in the left inset.
        let left = bar_dark_band(&rgb, w, 5, 25, h / 2 - 10, h / 2 + 30);
        assert_eq!(
            left, 0,
            "category B must follow the zero axis, not the left gutter"
        );
        let at_origin = bar_dark_band(&rgb, w, w / 2 - 45, w / 2 - 15, h / 2 - 10, h / 2 + 30);
        assert!(
            at_origin > 0,
            "category label ink must exist by the zero axis"
        );
    }

    #[test]
    fn excel_chart_line_width_snaps_to_the_official_device_pen() {
        let px96 = RenderDpi::DEFAULT.pt_to_px_font(2.25);
        assert_eq!(px96, 3.0);
        assert_eq!(excel_chart_line_width_px(px96), 3.0);

        let dpi106 =
            RenderDpi::from_request(serde_json::json!({"dpi": 106}).as_object().unwrap()).unwrap();
        assert!((dpi106.pt_to_px_font(2.25) - 3.3125).abs() < 1e-9);
        assert_eq!(excel_chart_line_width_px(dpi106.pt_to_px_font(2.25)), 4.0);
        assert_eq!(excel_chart_line_width_px(dpi106.emu_to_px_f(28_575.0)), 4.0);

        let dpi144 =
            RenderDpi::from_request(serde_json::json!({"dpi": 144}).as_object().unwrap()).unwrap();
        assert!((dpi144.pt_to_px_font(2.25) - 4.5).abs() < 1e-9);
        assert_eq!(excel_chart_line_width_px(dpi144.pt_to_px_font(2.25)), 5.0);
    }

    #[test]
    fn snapped_device_pen_paints_an_aligned_opaque_core() {
        let color = [21, 96, 130];
        for (dpi_value, expect) in [(96u32, 3i32), (106, 4), (144, 5)] {
            let dpi = RenderDpi::from_request(
                serde_json::json!({ "dpi": dpi_value }).as_object().unwrap(),
            )
            .unwrap();
            let width = excel_chart_line_width_px(dpi.pt_to_px_font(2.25));
            assert_eq!(width, f64::from(expect));
            // Odd widths sit on a pixel centre; even widths sit on a pixel edge
            // so the opaque core is exactly `expect` device pixels.
            let y = if expect % 2 == 0 { 20.0 } else { 20.5 };
            let mut canvas = Canvas::with_dpi(80, 40, dpi);
            canvas.stroke_segment(10.0, y, 70.0, y, width, color);
            let x = 40u32;
            let mut run = 0i32;
            let mut best = 0i32;
            for row in 0..canvas.h {
                let i = ((row * canvas.w + x) * 3) as usize;
                let px = [canvas.px[i], canvas.px[i + 1], canvas.px[i + 2]];
                if px == color {
                    run += 1;
                    best = best.max(run);
                } else {
                    run = 0;
                }
            }
            assert_eq!(
                best, expect,
                "aligned 2.25pt core at dpi={dpi_value} should be {expect}px, got {best}"
            );
        }
    }

    #[test]
    fn line_legend_key_preserves_fractional_point_endpoints() {
        let mut canvas = Canvas::new(100, 40);
        canvas.dpi =
            RenderDpi::from_request(serde_json::json!({"dpi":106}).as_object().unwrap()).unwrap();
        let color = [21, 96, 130];
        for kind in [PlotKind::StandardLine, PlotKind::Scatter { connect: true }] {
            let mut actual = Canvas::new(100, 40);
            actual.dpi = canvas.dpi;
            draw_plot_key(&mut actual, kind, 20, 10, 49, 19, color, MarkerKind::None);
            // A 20pt centreline at 106dpi is 29.444px, not (rounded slot - 1).
            // The cap support must include the rightmost official sample pixel.
            let offset = ((14 * actual.w + 50) * 3) as usize;
            assert_ne!(&actual.px[offset..offset + 3], &[255, 255, 255]);
        }
    }

    #[test]
    fn line_and_scatter_keys_use_point_width_not_square_marker_width() {
        for dpi_value in [96, 106, 192] {
            let dpi =
                RenderDpi::from_request(serde_json::json!({"dpi": dpi_value}).as_object().unwrap())
                    .unwrap();
            for marker_box in [7, 9, 14] {
                for kind in [
                    PlotKind::StandardLine,
                    PlotKind::Scatter { connect: true },
                    PlotKind::Scatter { connect: false },
                ] {
                    assert_eq!(
                        legend_key_width(kind, marker_box, dpi),
                        dpi.pt_to_px_font(20.0).round() as i32
                    );
                    assert_eq!(
                        legend_key_gap(kind, dpi),
                        dpi.pt_to_px_font(1.5).round() as i32
                    );
                }
                for kind in [
                    PlotKind::ClusteredBar,
                    PlotKind::ClusteredColumn,
                    PlotKind::StandardArea,
                    PlotKind::Pie,
                ] {
                    assert_eq!(legend_key_width(kind, marker_box, dpi), marker_box);
                }
            }
        }
    }

    #[test]
    fn clustered_bar_legend_follows_visual_series_order() {
        let series = format!(
            "{}{}",
            line_ser(0, "S1", &[Some(8.0), Some(1.0)], ""),
            line_ser(1, "S2", &[Some(2.0), Some(3.0)], "")
        );
        let a1 = FALLBACK_ACCENTS[0];
        let a2 = FALLBACK_ACCENTS[1];
        let (w, h) = (576u32, 336u32);
        // Restrict to the default right legend so plot bars cannot move the centroid.
        let x0 = 516;
        let x1 = 571;

        let (_, _, maxmin) =
            png_rgb(&render_bar(&bar_xml(&series, "", "maxMin", Some("bar"))).unwrap());
        let y1 = centroid_y(&maxmin, w, x0, x1, 0, h, a1, 2).expect("maxMin S1 key");
        let y2 = centroid_y(&maxmin, w, x0, x1, 0, h, a2, 2).expect("maxMin S2 key");
        assert!(
            y1 < y2,
            "maxMin must keep official S1-above-S2 legend order; y1={y1} y2={y2}"
        );

        let (_, _, minmax) =
            png_rgb(&render_bar(&bar_xml(&series, "", "minMax", Some("bar"))).unwrap());
        let y1 = centroid_y(&minmax, w, x0, x1, 0, h, a1, 2).expect("minMax S1 key");
        let y2 = centroid_y(&minmax, w, x0, x1, 0, h, a2, 2).expect("minMax S2 key");
        assert!(
            y2 < y1,
            "minMax must put official S2 above S1 in the legend; y1={y1} y2={y2}"
        );
    }

    #[test]
    fn clustered_bar_value_axis_follows_category_orientation() {
        let series = line_ser(0, "S1", &[Some(8.0), Some(1.0)], "");
        let (w, h) = (576u32, 336u32);
        let band = 36u32;
        let x0 = w / 5;
        let x1 = w * 3 / 4;

        let (_, _, top_rgb) =
            png_rgb(&render_bar(&bar_xml(&series, "", "maxMin", Some("bar"))).unwrap());
        let top = bar_dark_band(&top_rgb, w, x0, x1, 0, band);
        let bot = bar_dark_band(&top_rgb, w, x0, x1, h - band, h);
        assert!(
            top > bot * 3 && top > 80,
            "maxMin should put the value axis and tick labels at the top; top={top} bottom={bot}"
        );

        let (_, _, bot_rgb) =
            png_rgb(&render_bar(&bar_xml(&series, "", "minMax", Some("bar"))).unwrap());
        let top = bar_dark_band(&bot_rgb, w, x0, x1, 0, band);
        let bot = bar_dark_band(&bot_rgb, w, x0, x1, h - band, h);
        assert!(
            bot > top * 3 && bot > 80,
            "minMax should keep the value axis and tick labels at the bottom; top={top} bottom={bot}"
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
            &line_ser(
                0,
                "S1",
                &[Some(-8.0), Some(8.0)],
                "<c:invertIfNegative val=\"0\"/>",
            ),
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
            + &line_ser(0, "S1", &[Some(8.0)], "")
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

    fn near_ink(rgb: [u8; 3]) -> bool {
        rgb[0] < 40 && rgb[1] < 40 && rgb[2] < 40
    }

    fn pixel_rgb(rgb: &[u8], w: u32, x: u32, y: u32) -> [u8; 3] {
        let i = ((y * w + x) * 3) as usize;
        [rgb[i], rgb[i + 1], rgb[i + 2]]
    }

    fn fill_x_span_limit(
        rgb: &[u8],
        w: u32,
        h: u32,
        color: [u8; 3],
        tol: i16,
        x_limit: u32,
    ) -> Option<(u32, u32)> {
        let mut min_x = u32::MAX;
        let mut max_x = 0u32;
        let mut found = false;
        let x_end = x_limit.min(w);
        for y in 0..h {
            for x in 0..x_end {
                if near(pixel_rgb(rgb, w, x, y), color, tol) {
                    min_x = min_x.min(x);
                    max_x = max_x.max(x);
                    found = true;
                }
            }
        }
        found.then_some((min_x, max_x))
    }

    /// Long vertical ink columns in the plot, i.e. major grid / zero / max ticks.
    fn major_grid_xs(rgb: &[u8], w: u32, y0: u32, y1: u32, min_count: u32) -> Vec<u32> {
        let mut xs = Vec::new();
        for x in 0..w {
            let mut n = 0u32;
            for y in y0..y1 {
                if near_ink(pixel_rgb(rgb, w, x, y)) {
                    n += 1;
                }
            }
            if n >= min_count {
                xs.push(x);
            }
        }
        xs
    }

    #[test]
    fn clustered_bar_value_edge_lands_on_its_tick() {
        let color = FALLBACK_ACCENTS[0];
        let png = render_bar(&bar_xml(
            &line_ser(0, "S1", &[Some(8.0)], ""),
            "",
            "maxMin",
            Some("bar"),
        ))
        .unwrap();
        let (w, h, rgb) = png_rgb(&png);
        assert_eq!((w, h), (576, 336));
        // Skip the top value-axis band and the bottom gutter; grids run through the plot.
        let grids = major_grid_xs(&rgb, w, 50, 300, 80);
        assert!(
            grids.len() >= 10,
            "0..9 auto scale should expose the major grid; grids={grids:?}"
        );
        let zero = *grids.first().expect("zero / min tick");
        let max_tick = *grids.last().expect("max tick");
        // Auto 0..9, value 8 is 8/9 of the axis. Identify that tick from the grid list.
        let expected = zero + ((max_tick - zero) * 8 + 4) / 9;
        let tick8 = grids
            .iter()
            .copied()
            .min_by_key(|x| x.abs_diff(expected))
            .unwrap();
        // Exclude the right-side legend swatch, which uses the same accent color.
        let (x0, x1) =
            fill_x_span_limit(&rgb, w, h, color, 8, max_tick + 1).expect("value-8 bar fill");
        assert_eq!(
            x1, tick8,
            "value-8 bar right edge must land on the 8 tick (no 1px gap); fill={x0}..{x1} zero={zero} max={max_tick} tick8={tick8} grids={grids:?}"
        );
        assert!(
            x0 == zero || x0 == zero + 1,
            "positive bar should start at the zero axis (axis ink may cover the first pixel); fill={x0}..{x1} zero={zero}"
        );
    }

    #[test]
    fn clustered_bar_signed_auto_major_uses_step_two() {
        // Office auto-scales -8..8 to -10..10 with majorUnit=2. This must not be
        // hardcoded for one fixture; the same path serves any clustered bar.
        let color = FALLBACK_ACCENTS[0];
        let png = render_bar(&bar_xml(
            &line_ser(
                0,
                "S1",
                &[Some(-8.0), Some(0.0), Some(8.0)],
                r#"<c:invertIfNegative val="0"/>"#,
            ),
            "",
            "maxMin",
            Some("bar"),
        ))
        .unwrap();
        let (w, h, rgb) = png_rgb(&png);
        assert_eq!((w, h), (576, 336));
        let grids = major_grid_xs(&rgb, w, 50, 300, 80);
        assert_eq!(
            grids.len(),
            11,
            "auto major 2 on -10..10 should draw 11 vertical grid lines; grids={grids:?}"
        );
        let min_tick = grids[0];
        let max_tick = grids[10];
        let span = max_tick - min_tick;
        for (i, x) in grids.iter().copied().enumerate() {
            let expect = min_tick + ((span * i as u32) + 5) / 10;
            assert!(
                x.abs_diff(expect) <= 1,
                "grid {i} should be evenly spaced for step 2; x={x} expect={expect} grids={grids:?}"
            );
        }
        // Value 8 is the 9th tick (index 9: -10,-8,...,8,10). The bottom category
        // (C=+8, maxMin so first category on top) must meet that tick.
        let tick8 = grids[9];
        let tick0 = grids[5];
        // Restrict the fill hunt to the lower half so A=-8 does not mix in,
        // and stop before the legend swatch.
        let mut right = None;
        let mut left_pos = None;
        for y in (h / 2)..(h.saturating_sub(20)) {
            for x in 0..(max_tick + 1) {
                if near(pixel_rgb(&rgb, w, x, y), color, 8) {
                    left_pos = Some(left_pos.map_or(x, |cur: u32| cur.min(x)));
                    right = Some(right.map_or(x, |cur: u32| cur.max(x)));
                }
            }
        }
        let right = right.expect("positive C bar fill");
        let left_pos = left_pos.expect("positive C bar left");
        assert_eq!(
            right, tick8,
            "C=+8 bar right edge must land on the 8 tick; fill={left_pos}..{right} tick8={tick8} zero={tick0} grids={grids:?}"
        );
        assert!(
            left_pos.abs_diff(tick0) <= 1,
            "C bar should grow from the zero axis; left={left_pos} zero={tick0}"
        );
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
                r#"<c:dLbls><c:dLblPos val="inEnd"/><c:showVal val="1"/></c:dLbls></c:barChart>"#,
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

        // Official Excel 16.0: omitted dispBlanksAs plots the missing area
        // point as 0 (a V to the baseline), not a hole. Explicit 0 matches.
        let as_zero = line_ser(
            0,
            "S1",
            &[Some(2.0), Some(2.0), Some(0.0), Some(2.0), Some(2.0)],
            &area_fill("156082"),
        );
        let omitted_png = render_area(&standard_area_xml(&gapped, "")).unwrap();
        let zero_png = render_area(&standard_area_xml(&as_zero, "")).unwrap();
        assert_eq!(
            omitted_png, zero_png,
            "omitted dispBlanksAs plus a missing area point must fill like an explicit 0"
        );
        assert_ne!(
            omitted_png, png,
            "explicit dispBlanksAs=gap must still leave a hole, unlike the omitted default"
        );
        let explicit_zero_png = render_area(&standard_area_xml(
            &gapped,
            r#"<c:dispBlanksAs val="zero"/>"#,
        ))
        .unwrap();
        assert_eq!(
            explicit_zero_png, omitted_png,
            "explicit area zero must fill the same V-to-baseline as omitted default"
        );
        let na = gapped.replace("<c:v>2</c:v>", "<c:v>#N/A</c:v>");
        let div = gapped.replace("<c:v>2</c:v>", "<c:v>#DIV/0!</c:v>");
        for body in [&na, &div] {
            assert!(
                render_area(&standard_area_xml(body, r#"<c:dispBlanksAs val="zero"/>"#)).is_err(),
                "unverified area error points must not silently become baseline zero"
            );
        }
        let explicit_span_png = render_area(&standard_area_xml(
            &gapped,
            r#"<c:dispBlanksAs val="span"/>"#,
        ))
        .unwrap();
        assert_eq!(
            explicit_span_png, omitted_png,
            "explicit area span must fill the same V-to-baseline as omitted default"
        );
        // Mid-only #N/A / #DIV/0! — do not reuse the all-points error XML
        // used for the explicit-zero NI asserts above.
        let na_mid = gapped.replace(
            r#"<c:pt idx="1"><c:v>2</c:v></c:pt>"#,
            r#"<c:pt idx="1"><c:v>2</c:v></c:pt><c:pt idx="2"><c:v>#N/A</c:v></c:pt>"#,
        );
        let div_mid = gapped.replace(
            r#"<c:pt idx="1"><c:v>2</c:v></c:pt>"#,
            r#"<c:pt idx="1"><c:v>2</c:v></c:pt><c:pt idx="2"><c:v>#DIV/0!</c:v></c:pt>"#,
        );
        assert!(
            render_area(&standard_area_xml(
                &na_mid,
                r#"<c:dispBlanksAs val="span"/>"#
            ))
            .is_ok(),
            "official area span paints #N/A as the same baseline V"
        );
        let span_na_png = render_area(&standard_area_xml(
            &na_mid,
            r#"<c:dispBlanksAs val="span"/>"#,
        ))
        .unwrap();
        assert_eq!(
            span_na_png, explicit_span_png,
            "area span + #N/A must match the blank-mid span fill"
        );
        assert!(
            render_area(&standard_area_xml(
                &div_mid,
                r#"<c:dispBlanksAs val="span"/>"#
            ))
            .is_err(),
            "unverified area #DIV/0! under span must stay not implemented"
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
    fn pie_opaque_plot_fill_paints_bounding_square_before_slices() {
        let ser = line_ser(0, "S1", &[Some(1.0), Some(1.0), Some(1.0), Some(1.0)], "");
        for dpi_value in [96, 106, 144] {
            let dpi =
                RenderDpi::from_request(serde_json::json!({"dpi":dpi_value}).as_object().unwrap())
                    .unwrap();
            let xml = standard_pie_xml(&ser, "", "").replace(r#"<c:legend><c:legendPos val="r"/></c:legend>"#, "").replace(
                "<c:plotArea>",
                "<c:plotArea><c:spPr><a:solidFill><a:srgbClr val='FFFF00'/></a:solidFill></c:spPr>",
            );
            let doc = crate::xml::parse(xml.as_bytes()).unwrap();
            let png = render_chart_at(
                &doc.root,
                576,
                336,
                FALLBACK_ACCENTS,
                None,
                ChartHost::Excel,
                None,
                "",
                dpi,
            )
            .unwrap();
            let (w, h, rgb) = decode_rgb(&png);
            let rect = pie_plot_bounds(576.0, 336.0, 0.0, 0.0, dpi).unwrap();
            let x = rect.x.ceil() as u32 + 2;
            let y = rect.y.ceil() as u32 + 2;
            let i = ((y * w + x) * 3) as usize;
            assert_eq!(
                &rgb[i..i + 3],
                &[255, 255, 0],
                "bounding-square corner, not just wedge"
            );
            let outside = ((y * w + x - 5) * 3) as usize;
            assert_eq!(
                &rgb[outside..outside + 3],
                &[255, 255, 255],
                "not full host fill"
            );
            for accent in &FALLBACK_ACCENTS[..4] {
                assert!(
                    color_centroid(&rgb, w, h, *accent, 0).unwrap().2 > 100,
                    "fill must stay below slices"
                );
            }
        }
        for fill in [
            "<a:solidFill><a:srgbClr val='FFFF00'><a:alpha val='50000'/></a:srgbClr></a:solidFill>",
            "<a:gradFill><a:gsLst><a:gs pos='0'><a:srgbClr val='FFFF00'/></a:gs><a:gs pos='100000'><a:srgbClr val='0000FF'/></a:gs></a:gsLst><a:lin ang='0'/></a:gradFill>",
        ] {
            let xml=standard_pie_xml(&ser,"","").replace("<c:plotArea>",&format!("<c:plotArea><c:spPr>{fill}</c:spPr>"));
            assert!(render_pie(&xml).is_err(),"unlocked pie plot fill must remain NI");
        }
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
    fn pie_skips_blank_and_zero_slices_but_keeps_both_in_the_legend() {
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
        // Blank point is skipped as a slice, but official Excel still shows it in the legend.
        assert!(c1.2 >= 80 && c3.2 >= 80);
        assert!(
            (c1.0 - c3.0).abs() > 20.0,
            "the two remaining slices should sit on opposite sides"
        );
        let blank_hits = color_centroid(&rgb, w, h, a2, 2).map(|c| c.2).unwrap_or(0);
        assert!(
            blank_hits > 0 && blank_hits < 80,
            "a blank should appear in the legend, not as a slice; hits={blank_hits}"
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
    fn pie_all_zero_all_empty_and_all_negative_follow_official_empty_or_abs_plot() {
        let all_neg = render_pie(&standard_pie_xml(
            &line_ser(0, "S1", &[Some(-10.0), Some(-20.0), Some(-30.0), Some(-40.0)], ""),
            "",
            "",
        ))
        .unwrap();
        let all_abs = render_pie(&standard_pie_xml(
            &line_ser(0, "S1", &[Some(10.0), Some(20.0), Some(30.0), Some(40.0)], ""),
            "",
            "",
        ))
        .unwrap();
        assert_eq!(all_neg, all_abs, "all-negative pies should paint as |v|");

        let zero_neg = render_pie(&standard_pie_xml(
            &line_ser(0, "S1", &[Some(0.0), Some(-20.0), Some(0.0), Some(0.0)], ""),
            "",
            "",
        ))
        .unwrap();
        let zero_abs = render_pie(&standard_pie_xml(
            &line_ser(0, "S1", &[Some(0.0), Some(20.0), Some(0.0), Some(0.0)], ""),
            "",
            "",
        ))
        .unwrap();
        assert_eq!(
            zero_neg, zero_abs,
            "a lone finite negative among zeros should paint as |v|"
        );
        assert_ne!(zero_neg, all_abs, "zero-and-negative is not a four-slice abs pie");

        let all_zero = render_pie(&standard_pie_xml(
            &line_ser(0, "S1", &[Some(0.0), Some(0.0), Some(0.0), Some(0.0)], ""),
            "",
            "",
        ))
        .unwrap();
        let all_empty_ser = r#"<c:ser>
          <c:idx val="0"/>
          <c:order val="0"/>
          <c:tx><c:v>S1</c:v></c:tx>
          <c:cat><c:strLit><c:ptCount val="4"/><c:pt idx="0"><c:v>A</c:v></c:pt><c:pt idx="1"><c:v>B</c:v></c:pt><c:pt idx="2"><c:v>C</c:v></c:pt><c:pt idx="3"><c:v>D</c:v></c:pt></c:strLit></c:cat>
          <c:val><c:numLit><c:ptCount val="4"/></c:numLit></c:val>
        </c:ser>"#;
        let all_empty = render_pie(&standard_pie_xml(
            all_empty_ser,
            r#"<c:dispBlanksAs val="gap"/>"#,
            "",
        ))
        .unwrap();
        assert_eq!(
            all_zero, all_empty,
            "official-like all-empty (ptCount, no val pts) should match all-zero"
        );
        // line_ser omits val pts and ptCount, but still writes category pts.
        // That is the same locked domain rule, not a second painter.
        let omitted_pts = render_pie(&standard_pie_xml(
            &line_ser(0, "S1", &[None, None, None, None], ""),
            r#"<c:dispBlanksAs val="gap"/>"#,
            "",
        ))
        .unwrap();
        assert_eq!(
            all_zero, omitted_pts,
            "omitted val pts with category pts should match all-zero"
        );
        let (w, h, rgb) = png_rgb(&all_zero);
        assert_eq!((w, h), (576, 336));
        let a1 = FALLBACK_ACCENTS[0];
        let a2 = FALLBACK_ACCENTS[1];
        let a3 = FALLBACK_ACCENTS[2];
        let a4 = FALLBACK_ACCENTS[3];
        for (i, accent) in [a1, a2, a3, a4].iter().enumerate() {
            let hits = color_centroid(&rgb, w, h, *accent, 2).map(|c| c.2).unwrap_or(0);
            assert!(
                hits > 0 && hits < 80,
                "all-zero/empty should keep legend swatch {i}, not a slice; hits={hits}"
            );
        }
    }

    #[test]
    fn pie_finite_negatives_use_absolute_slice_weight_and_stay_in_legend() {
        let cases: &[(&[Option<f64>], &[Option<f64>])] = &[
            (
                &[Some(10.0), Some(-20.0), Some(30.0), Some(40.0)],
                &[Some(10.0), Some(20.0), Some(30.0), Some(40.0)],
            ),
            (
                &[Some(-10.0), Some(20.0), Some(30.0), Some(40.0)],
                &[Some(10.0), Some(20.0), Some(30.0), Some(40.0)],
            ),
            (
                &[Some(10.0), Some(-20.0), Some(-30.0), Some(40.0)],
                &[Some(10.0), Some(20.0), Some(30.0), Some(40.0)],
            ),
            (
                &[Some(1.0), Some(-2.0), Some(3.0)],
                &[Some(1.0), Some(2.0), Some(3.0)],
            ),
        ];
        for (signed, absed) in cases {
            let signed_png = render_pie(&standard_pie_xml(
                &line_ser(0, "S1", signed, ""),
                "",
                "",
            ))
            .unwrap();
            let abs_png = render_pie(&standard_pie_xml(
                &line_ser(0, "S1", absed, ""),
                "",
                "",
            ))
            .unwrap();
            assert_eq!(
                signed_png, abs_png,
                "finite negatives should paint as |v|, signed={signed:?}"
            );
        }
        let mixed = render_pie(&standard_pie_xml(
            &line_ser(
                0,
                "S1",
                &[Some(10.0), Some(-20.0), Some(30.0), Some(40.0)],
                "",
            ),
            "",
            "",
        ))
        .unwrap();
        let skip_zero = render_pie(&standard_pie_xml(
            &line_ser(
                0,
                "S1",
                &[Some(10.0), Some(0.0), Some(30.0), Some(40.0)],
                "",
            ),
            "",
            "",
        ))
        .unwrap();
        assert_ne!(
            mixed, skip_zero,
            "finite negatives are not skipped as zero"
        );
    }

    #[test]
    fn pie_explosion_follows_official_series_shrink_and_point_bisector() {
        let vals = [Some(10.0), Some(20.0), Some(30.0), Some(40.0)];
        let ctrl = render_pie(&standard_pie_xml(
            &line_ser(0, "S1", &vals, ""),
            "",
            "",
        ))
        .unwrap();
        let exp0 = render_pie(&standard_pie_xml(
            &line_ser(0, "S1", &vals, r#"<c:explosion val="0"/>"#),
            "",
            "",
        ))
        .unwrap();
        assert_eq!(ctrl, exp0, "explicit explosion 0 should match an omitted explosion");

        let ser10 = render_pie(&standard_pie_xml(
            &line_ser(0, "S1", &vals, r#"<c:explosion val="10"/>"#),
            "",
            "",
        ))
        .unwrap();
        let ser25 = render_pie(&standard_pie_xml(
            &line_ser(0, "S1", &vals, r#"<c:explosion val="25"/>"#),
            "",
            "",
        ))
        .unwrap();
        assert_ne!(ser10, ctrl, "series explosion 10 should move the slices");
        assert_ne!(ser25, ser10, "series explosion 25 is not the same as 10");

        let (w, h, ctrl_rgb) = png_rgb(&ctrl);
        let (_, _, s10_rgb) = png_rgb(&ser10);
        let (_, _, s25_rgb) = png_rgb(&ser25);
        let accents = [
            FALLBACK_ACCENTS[0],
            FALLBACK_ACCENTS[1],
            FALLBACK_ACCENTS[2],
            FALLBACK_ACCENTS[3],
        ];
        let ctrl_c: Vec<_> = accents
            .iter()
            .map(|c| color_centroid(&ctrl_rgb, w, h, *c, 2).expect("ctrl slice"))
            .collect();
        let s10_c: Vec<_> = accents
            .iter()
            .map(|c| color_centroid(&s10_rgb, w, h, *c, 2).expect("ser10 slice"))
            .collect();
        let s25_c: Vec<_> = accents
            .iter()
            .map(|c| color_centroid(&s25_rgb, w, h, *c, 2).expect("ser25 slice"))
            .collect();
        let cx: f64 = ctrl_c.iter().map(|c| c.0).sum::<f64>() / 4.0;
        let cy: f64 = ctrl_c.iter().map(|c| c.1).sum::<f64>() / 4.0;
        for i in 0..4 {
            let r0 = ((ctrl_c[i].0 - cx).hypot(ctrl_c[i].1 - cy)).max(1.0);
            let r10 = (s10_c[i].0 - cx).hypot(s10_c[i].1 - cy);
            let r25 = (s25_c[i].0 - cx).hypot(s25_c[i].1 - cy);
            assert!(
                r10 > r0 && r25 > r10,
                "series explosion should move slice {i} outward; r0={r0:.2} r10={r10:.2} r25={r25:.2}"
            );
            assert!(
                s10_c[i].2 < ctrl_c[i].2 && s25_c[i].2 < s10_c[i].2,
                "series explosion should shrink slice {i} area; n0={} n10={} n25={}",
                ctrl_c[i].2,
                s10_c[i].2,
                s25_c[i].2
            );
        }

        let pt_a = render_pie(&standard_pie_xml(
            &line_ser(
                0,
                "S1",
                &vals,
                r#"<c:dPt><c:idx val="0"/><c:bubble3D val="0"/><c:explosion val="20"/></c:dPt>"#,
            ),
            "",
            "",
        ))
        .unwrap();
        let pt_b = render_pie(&standard_pie_xml(
            &line_ser(
                0,
                "S1",
                &vals,
                r#"<c:dPt><c:idx val="1"/><c:bubble3D val="0"/><c:explosion val="20"/></c:dPt>"#,
            ),
            "",
            "",
        ))
        .unwrap();
        assert_ne!(pt_a, ctrl);
        assert_ne!(pt_b, ctrl);
        assert_ne!(pt_a, pt_b);
        let (_, _, a_rgb) = png_rgb(&pt_a);
        let (_, _, b_rgb) = png_rgb(&pt_b);
        let a_c: Vec<_> = accents
            .iter()
            .map(|c| color_centroid(&a_rgb, w, h, *c, 2).expect("ptA slice"))
            .collect();
        let b_c: Vec<_> = accents
            .iter()
            .map(|c| color_centroid(&b_rgb, w, h, *c, 2).expect("ptB slice"))
            .collect();
        let dx_a = a_c[0].0 - ctrl_c[0].0;
        let dy_a = a_c[0].1 - ctrl_c[0].1;
        let ang_a = dx_a.atan2(-dy_a).to_degrees();
        assert!(
            dy_a < -8.0 && ang_a > 10.0 && ang_a < 26.0,
            "point A explosion should move the first slice up along its ~18 deg bisector; d=({dx_a:.2},{dy_a:.2}) ang={ang_a:.1}"
        );
        for i in 1..4 {
            let d = (a_c[i].0 - ctrl_c[i].0).hypot(a_c[i].1 - ctrl_c[i].1);
            assert!(
                d < 2.0,
                "unexploded slices should stay put on a point-A explosion; slice {i} moved {d:.2}"
            );
        }
        let dx_b = b_c[1].0 - ctrl_c[1].0;
        let dy_b = b_c[1].1 - ctrl_c[1].1;
        let ang_b = dx_b.atan2(-dy_b).to_degrees();
        assert!(
            dx_b > 8.0 && ang_b > 64.0 && ang_b < 80.0,
            "point B explosion should move the second slice along its ~72 deg bisector; d=({dx_b:.2},{dy_b:.2}) ang={ang_b:.1}"
        );
        for i in [0, 2, 3] {
            let d = (b_c[i].0 - ctrl_c[i].0).hypot(b_c[i].1 - ctrl_c[i].1);
            assert!(
                d < 2.0,
                "unexploded slices should stay put on a point-B explosion; slice {i} moved {d:.2}"
            );
        }
    }

    #[test]
    fn pie_slice_fills_reuse_locked_solid_alpha_and_nofill() {
        let vals = [Some(10.0), Some(20.0), Some(30.0), Some(40.0)];
        let with_plot = |xml: String| {
            xml.replace(
                "<c:plotArea>",
                "<c:plotArea><c:spPr><a:solidFill><a:srgbClr val='FFFFCC'/></a:solidFill></c:spPr>",
            )
        };
        let src_over = |src: [u8; 3], dst: [u8; 3], cov: u8| -> [u8; 3] {
            let src_a = u16::from(cov);
            std::array::from_fn(|k| {
                ((u16::from(src[k]) * src_a + u16::from(dst[k]) * (255 - src_a) + 127) / 255) as u8
            })
        };
        let exact = |rgb: &[u8], w: u32, h: u32, color: [u8; 3]| {
            let mut n = 0usize;
            for i in 0..(w * h) as usize {
                if rgb[i * 3..i * 3 + 3] == color {
                    n += 1;
                }
            }
            n
        };

        let ser_solid = render_pie(&with_plot(standard_pie_xml(
            &line_ser(
                0,
                "S1",
                &vals,
                r#"<c:spPr><a:solidFill><a:srgbClr val="FF0000"/></a:solidFill></c:spPr>"#,
            ),
            "",
            "",
        )))
        .unwrap();
        let (w, h, rgb) = png_rgb(&ser_solid);
        assert!(
            exact(&rgb, w, h, [255, 0, 0]) >= 200,
            "series solidFill should paint every slice even when varyColors=1"
        );
        for accent in &FALLBACK_ACCENTS[..4] {
            assert!(
                exact(&rgb, w, h, *accent) < 40,
                "varyColors must not keep accent slices once the series has an explicit solid"
            );
        }

        let cov = ooxml_alpha_to_u8(50_000);
        assert_eq!(cov, 128);
        let plot = [255, 255, 204];
        let red_over_plot = src_over([255, 0, 0], plot, cov);
        assert_eq!(red_over_plot, [255, 127, 102]);
        let ser_alpha = render_pie(&with_plot(standard_pie_xml(
            &line_ser(
                0,
                "S1",
                &vals,
                r#"<c:spPr><a:solidFill><a:srgbClr val="FF0000"><a:alpha val="50000"/></a:srgbClr></a:solidFill></c:spPr>"#,
            ),
            "",
            "",
        )))
        .unwrap();
        let (w, h, rgb) = png_rgb(&ser_alpha);
        assert!(
            exact(&rgb, w, h, red_over_plot) >= 200,
            "series alpha 50000 should use locked straight alpha over the plot fill"
        );
        assert_eq!(exact(&rgb, w, h, [255, 0, 0]), 0);

        let hidden = r#"<c:spPr><a:noFill/><a:extLst><a:ext uri="{909E8E84-426E-40DD-AFC4-6F175D3DCCD1}"><a14:hiddenFill xmlns:a14="http://schemas.microsoft.com/office/drawing/2010/main"><a:solidFill><a:srgbClr val="156082"/></a:solidFill></a14:hiddenFill></a:ext></a:extLst></c:spPr>"#;
        let ser_nofill = render_pie(&with_plot(standard_pie_xml(
            &line_ser(0, "S1", &vals, hidden),
            "",
            "",
        )))
        .unwrap();
        let (w, h, rgb) = png_rgb(&ser_nofill);
        assert!(exact(&rgb, w, h, plot) >= 200, "series noFill shows the plot");
        assert_eq!(exact(&rgb, w, h, [255, 0, 0]), 0);
        assert_eq!(exact(&rgb, w, h, [0x15, 0x60, 0x82]), 0);
        for accent in &FALLBACK_ACCENTS[..4] {
            assert_eq!(
                exact(&rgb, w, h, *accent),
                0,
                "series noFill should not paint accent wedges or keys"
            );
        }

        let pt_nofill = render_pie(&with_plot(standard_pie_xml(
            &line_ser(
                0,
                "S1",
                &vals,
                &format!(r#"<c:dPt><c:idx val="0"/><c:bubble3D val="0"/>{hidden}</c:dPt>"#),
            ),
            "",
            "",
        )))
        .unwrap();
        let (w, h, rgb) = png_rgb(&pt_nofill);
        assert_eq!(
            exact(&rgb, w, h, FALLBACK_ACCENTS[0]),
            0,
            "dPt noFill removes that slice and its legend key"
        );
        for accent in &FALLBACK_ACCENTS[1..4] {
            assert!(
                exact(&rgb, w, h, *accent) >= 80,
                "other slices stay on the default accent"
            );
        }

        let lime_over_plot = src_over([0, 255, 0], plot, cov);
        assert_eq!(lime_over_plot, [127, 255, 102]);
        let mixed = render_pie(&with_plot(standard_pie_xml(
            &line_ser(
                0,
                "S1",
                &vals,
                r#"<c:spPr><a:solidFill><a:srgbClr val="FF0000"/></a:solidFill></c:spPr>
          <c:dPt><c:idx val="1"/><c:bubble3D val="0"/><c:spPr><a:solidFill><a:srgbClr val="0000FF"/></a:solidFill></c:spPr></c:dPt>
          <c:dPt><c:idx val="2"/><c:bubble3D val="0"/><c:spPr><a:solidFill><a:srgbClr val="00FF00"><a:alpha val="50000"/></a:srgbClr></a:solidFill></c:spPr></c:dPt>"#,
            ),
            "",
            "",
        )))
        .unwrap();
        let (w, h, rgb) = png_rgb(&mixed);
        assert!(exact(&rgb, w, h, [255, 0, 0]) >= 80, "unoverridden slices use the series solid");
        assert!(exact(&rgb, w, h, [0, 0, 255]) >= 80, "dPt solid overrides that slice");
        assert!(
            exact(&rgb, w, h, lime_over_plot) >= 80,
            "dPt alpha overrides that slice with locked straight alpha"
        );
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
            // Leftover: a val cache ptCount with no val pts and no category
            // pts is not sized from untrusted ptCount.
            standard_pie_xml(
                r#"<c:ser>
          <c:idx val="0"/>
          <c:order val="0"/>
          <c:tx><c:v>S1</c:v></c:tx>
          <c:val><c:numLit><c:ptCount val="4"/></c:numLit></c:val>
        </c:ser>"#,
                r#"<c:dispBlanksAs val="gap"/>"#,
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
            // Leftover: series explosion plus a dPt explosion is not uniquely locked.
            standard_pie_xml(
                &line_ser(
                    0,
                    "S1",
                    &[Some(1.0), Some(2.0)],
                    r#"<c:explosion val="10"/><c:dPt><c:idx val="0"/><c:explosion val="20"/></c:dPt>"#,
                ),
                "",
                "",
            ),
            standard_pie_xml(
                &line_ser(
                    0,
                    "S1",
                    &[Some(1.0), Some(2.0)],
                    r#"<c:dPt><c:idx val="0"/><c:bubble3D val="1"/><c:explosion val="20"/></c:dPt>"#,
                ),
                "",
                "",
            ),
            standard_pie_xml(
                &line_ser(
                    0,
                    "S1",
                    &[Some(1.0), Some(2.0)],
                    r#"<c:spPr><a:gradFill flip="none" rotWithShape="1"><a:gsLst><a:gs pos="0"><a:srgbClr val="FF0000"/></a:gs><a:gs pos="100000"><a:srgbClr val="0000FF"/></a:gs></a:gsLst><a:lin ang="0" scaled="1"/><a:tileRect/></a:gradFill></c:spPr>"#,
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

    fn inject_explicit_log10(xml: &str) -> String {
        let scale =
            r#"<c:scaling><c:logBase val="10"/><c:min val="1"/><c:max val="1000"/></c:scaling>"#;
        if xml.contains(r#"<c:min val="-3"/>"#) {
            return xml.replace(
                "<c:scaling>\n          <c:min val=\"-3\"/>\n          <c:max val=\"5\"/>\n        </c:scaling>\n        <c:majorUnit val=\"1\"/>",
                scale,
            );
        }
        if xml.contains(
            r#"<c:valAx><c:axId val="2"/><c:scaling><c:orientation val="minMax"/></c:scaling>"#,
        ) {
            return xml.replace(
                r#"<c:valAx><c:axId val="2"/><c:scaling><c:orientation val="minMax"/></c:scaling>"#,
                &format!(r#"<c:valAx><c:axId val="2"/>{scale}"#),
            );
        }
        xml.replace(
            r#"<c:valAx><c:majorGridlines/><c:crossBetween val="between"/></c:valAx>"#,
            &format!(
                r#"<c:valAx>{scale}<c:majorGridlines/><c:crossBetween val="between"/></c:valAx>"#
            ),
        )
    }

    #[test]
    fn explicit_log_base_maps_equal_decades_for_column_and_line() {
        let plot = Rect {
            x: 0.0,
            y: 10.0,
            w: 100.0,
            h: 300.0,
        };
        let ticks = super::tick_values_log(1.0, 1000.0, 10.0);
        assert_eq!(ticks, vec![1.0, 10.0, 100.0, 1000.0]);
        let ys: Vec<i32> = ticks
            .iter()
            .map(|v| super::y_map(*v, 1.0, 1000.0, &plot, Some(10.0), false).round() as i32)
            .collect();
        assert_eq!(ys, vec![310, 210, 110, 10]);
        assert_eq!(
            super::tick_values_log(1.0, 32.0, 2.0),
            vec![1.0, 2.0, 4.0, 8.0, 16.0, 32.0]
        );

        let values = [1.0, 10.0, 100.0, 1000.0];
        let col = inject_explicit_log10(&mixed_column_xml(&values));
        let doc = crate::xml::parse(col.as_bytes()).unwrap();
        render_rgb_chart(
            &doc.root,
            424,
            265,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .unwrap();
        let chart = read_chart(&doc.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT).unwrap();
        let mut canvas = Canvas::new(300, 220);
        let bar_plot = Rect {
            x: 35.0,
            y: 16.0,
            w: 220.0,
            h: 188.0,
        };
        draw_bars(
            &mut canvas,
            &chart,
            &bar_plot,
            1.0,
            1000.0,
            10.0,
            Some(10.0),
            1.0,
            false,
            None,
        );
        let accent = FALLBACK_ACCENTS[0];
        let near = |px: &[u8]| {
            (px[0] as i16 - accent[0] as i16).abs() < 8
                && (px[1] as i16 - accent[1] as i16).abs() < 8
                && (px[2] as i16 - accent[2] as i16).abs() < 8
        };
        let mut counts = [0u32; 4];
        let slot = bar_plot.w / 4.0;
        for y in 0..canvas.h {
            for x in 0..canvas.w {
                let i = (y as usize * canvas.w as usize + x as usize) * 3;
                if !near(&canvas.px[i..i + 3]) {
                    continue;
                }
                let ci = ((f64::from(x) - bar_plot.x) / slot).floor();
                if (0.0..4.0).contains(&ci) {
                    counts[ci as usize] += 1;
                }
            }
        }
        assert_eq!(counts[0], 0, "value==min must paint a zero-height bar");
        assert!(counts[1] > 0 && counts[2] > counts[1] && counts[3] > counts[2]);

        let line = inject_explicit_log10(&standard_line_xml(
            &line_ser(
                0,
                "S",
                &[Some(1.0), Some(10.0), Some(100.0), Some(1000.0)],
                "",
            ),
            "",
        ));
        render_line(&line).unwrap();

        let auto = col.replace(r#"<c:min val="1"/><c:max val="1000"/>"#, "");
        let auto_doc = crate::xml::parse(auto.as_bytes()).unwrap();
        assert!(
            read_chart(
                &auto_doc.root,
                FALLBACK_ACCENTS,
                None,
                "",
                RenderDpi::DEFAULT
            )
            .is_err(),
            "omitted log min/max must stay NI"
        );

        let below = inject_explicit_log10(&mixed_column_xml(&[0.5, 10.0, 100.0, 1000.0]));
        let below_doc = crate::xml::parse(below.as_bytes()).unwrap();
        assert!(
            render_rgb_chart(
                &below_doc.root,
                424,
                265,
                FALLBACK_ACCENTS,
                None,
                ChartHost::Excel,
                None,
                "",
            )
            .is_err(),
            "values below the explicit log min must stay NI"
        );

        let area = inject_explicit_log10(&standard_area_xml(
            &line_ser(
                0,
                "S",
                &[Some(1.0), Some(10.0), Some(100.0), Some(1000.0)],
                "",
            ),
            "",
        ));
        let err = render_area(&area).expect_err("area log scale stays NI");
        assert_eq!(err.code(), crate::error::ErrorCode::NotImplemented);

        let bar = inject_explicit_log10(&bar_xml(
            &line_ser(
                0,
                "S",
                &[Some(1.0), Some(10.0), Some(100.0), Some(1000.0)],
                "",
            ),
            "",
            "maxMin",
            Some("bar"),
        ));
        let err = render_bar(&bar).expect_err("bar log scale stays NI");
        assert_eq!(err.code(), crate::error::ErrorCode::NotImplemented);
    }

    fn with_cat_crosses_at(xml: &str, value: &str) -> String {
        let xml = xml.replace(
            r#"<c:crosses val="autoZero"/>
      </c:catAx>"#,
            &format!(
                r#"<c:crossesAt val="{value}"/>
      </c:catAx>"#
            ),
        );
        let xml = if xml.contains(r#"<c:catAx/>"#) {
            xml.replace(
                r#"<c:valAx><c:majorGridlines/><c:crossBetween val="between"/></c:valAx>
      <c:catAx/>"#,
                &format!(
                    r#"<c:valAx><c:scaling><c:min val="-8"/><c:max val="8"/></c:scaling><c:majorUnit val="2"/><c:majorGridlines/><c:crossBetween val="between"/></c:valAx>
      <c:catAx><c:crossesAt val="{value}"/></c:catAx>"#
                ),
            )
        } else {
            xml
        };
        // Compact bar fixtures keep autoZero on the same line as </c:catAx>.
        xml.replace(
            r#"<c:crosses val="autoZero"/></c:catAx>"#,
            &format!(r#"<c:crossesAt val="{value}"/></c:catAx>"#),
        )
    }

    fn mixed_column_explicit_pm8(values: &[f64]) -> String {
        mixed_column_xml(values)
            .replace(r#"<c:min val="-3"/>"#, r#"<c:min val="-8"/>"#)
            .replace(r#"<c:max val="5"/>"#, r#"<c:max val="8"/>"#)
            .replace(r#"<c:majorUnit val="1"/>"#, r#"<c:majorUnit val="2"/>"#)
    }

    #[test]
    fn cat_axis_crosses_at_grows_columns_and_areas_from_the_crossing() {
        let values = [-4.0, -1.0, 3.0, 6.0];
        let xml = with_cat_crosses_at(&mixed_column_explicit_pm8(&values), "2");
        assert!(
            xml.contains(r#"<c:crossesAt val="2"/>"#),
            "fixture must carry catAx crossesAt"
        );
        let doc = crate::xml::parse(xml.as_bytes()).unwrap();
        let chart = read_chart(&doc.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT).unwrap();
        assert_eq!(chart.axis.crosses_at, Some(2.0));
        let mut canvas = Canvas::new(300, 220);
        let plot = Rect {
            x: 35.0,
            y: 16.0,
            w: 220.0,
            h: 188.0,
        };
        draw_bars(&mut canvas, &chart, &plot, -8.0, 8.0, 2.0, None, 2.0, false, None);
        let origin_y = crossing_axis_y(&plot, -8.0, 8.0, 2.0, 2.0);
        let zero_y = crossing_axis_y(&plot, -8.0, 8.0, 2.0, 0.0);
        assert_ne!(origin_y, zero_y);
        let accent = FALLBACK_ACCENTS[0];
        let near = |px: &[u8]| {
            (px[0] as i16 - accent[0] as i16).abs() < 8
                && (px[1] as i16 - accent[1] as i16).abs() < 8
                && (px[2] as i16 - accent[2] as i16).abs() < 8
        };
        let mut above_origin = 0u32;
        let mut between_origin_and_zero = 0u32;
        for y in 0..canvas.h {
            for x in 0..canvas.w {
                let i = (y as usize * canvas.w as usize + x as usize) * 3;
                if !near(&canvas.px[i..i + 3]) {
                    continue;
                }
                if (y as i32) < origin_y {
                    above_origin += 1;
                }
                if origin_y < y as i32 && (y as i32) < zero_y {
                    between_origin_and_zero += 1;
                }
            }
        }
        assert!(
            above_origin > 0,
            "values above the crossing must fill toward it"
        );
        assert_eq!(
            between_origin_and_zero, 0,
            "columns must not keep growing to zero once crossesAt is set"
        );

        render_rgb_chart(
            &doc.root,
            424,
            265,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .unwrap();
        render_line(&with_cat_crosses_at(
            &standard_line_xml(
                &line_ser(0, "S", &[Some(-4.0), Some(-1.0), Some(3.0), Some(6.0)], ""),
                "",
            ),
            "2",
        ))
        .unwrap();
        render_area(&with_cat_crosses_at(
            &standard_area_xml(
                &line_ser(0, "S", &[Some(-4.0), Some(-1.0), Some(3.0), Some(6.0)], ""),
                "",
            ),
            "2",
        ))
        .unwrap();
        let bar = with_cat_crosses_at(
            &bar_xml(
                &line_ser(0, "S", &[Some(-4.0), Some(-1.0), Some(3.0), Some(6.0)], ""),
                "",
                "maxMin",
                Some("bar"),
            ),
            "2",
        );
        assert!(
            bar.contains(r#"<c:crossesAt val="2"/>"#),
            "bar fixture must carry catAx crossesAt"
        );
        let err = render_bar(&bar).expect_err("bar crossesAt stays NI");
        assert_eq!(err.code(), crate::error::ErrorCode::NotImplemented);
    }

    fn with_cat_crosses(xml: &str, mode: &str) -> String {
        let xml = xml.replace(
            r#"<c:crosses val="autoZero"/>
      </c:catAx>"#,
            &format!(
                r#"<c:crosses val="{mode}"/>
      </c:catAx>"#
            ),
        );
        let xml = if xml.contains(r#"<c:catAx/>"#) {
            xml.replace(
                r#"<c:valAx><c:majorGridlines/><c:crossBetween val="between"/></c:valAx>
      <c:catAx/>"#,
                &format!(
                    r#"<c:valAx><c:scaling><c:min val="-8"/><c:max val="8"/></c:scaling><c:majorUnit val="2"/><c:majorGridlines/><c:crossBetween val="between"/></c:valAx>
      <c:catAx><c:crosses val="{mode}"/></c:catAx>"#
                ),
            )
        } else {
            xml
        };
        xml.replace(
            r#"<c:crosses val="autoZero"/></c:catAx>"#,
            &format!(r#"<c:crosses val="{mode}"/></c:catAx>"#),
        )
    }

    fn first_accent_row(png: &[u8]) -> i32 {
        let (w, h, rgb) = png_rgb(png);
        let accent = FALLBACK_ACCENTS[0];
        for y in 0..h as usize {
            for x in 0..w as usize {
                let i = (y * w as usize + x) * 3;
                if (rgb[i] as i16 - accent[0] as i16).abs() < 8
                    && (rgb[i + 1] as i16 - accent[1] as i16).abs() < 8
                    && (rgb[i + 2] as i16 - accent[2] as i16).abs() < 8
                {
                    return y as i32;
                }
            }
        }
        panic!("expected accent fill");
    }

    #[test]
    fn cat_axis_crosses_min_max_grows_from_the_bounds() {
        let values = [-4.0, -1.0, 3.0, 6.0];
        let min_xml = with_cat_crosses(&mixed_column_explicit_pm8(&values), "min");
        let max_xml = with_cat_crosses(&mixed_column_explicit_pm8(&values), "max");
        assert!(min_xml.contains(r#"<c:crosses val="min"/>"#));
        assert!(max_xml.contains(r#"<c:crosses val="max"/>"#));
        let min_doc = crate::xml::parse(min_xml.as_bytes()).unwrap();
        let max_doc = crate::xml::parse(max_xml.as_bytes()).unwrap();
        let min_chart = read_chart(
            &min_doc.root,
            FALLBACK_ACCENTS,
            None,
            "",
            RenderDpi::DEFAULT,
        )
        .unwrap();
        let max_chart = read_chart(
            &max_doc.root,
            FALLBACK_ACCENTS,
            None,
            "",
            RenderDpi::DEFAULT,
        )
        .unwrap();
        assert_eq!(min_chart.axis.cat_crosses, super::axis::CatCrosses::Min);
        assert_eq!(max_chart.axis.cat_crosses, super::axis::CatCrosses::Max);

        let plot = Rect {
            x: 35.0,
            y: 16.0,
            w: 220.0,
            h: 188.0,
        };
        let accent = FALLBACK_ACCENTS[0];
        let near = |px: &[u8]| {
            (px[0] as i16 - accent[0] as i16).abs() < 8
                && (px[1] as i16 - accent[1] as i16).abs() < 8
                && (px[2] as i16 - accent[2] as i16).abs() < 8
        };
        let min_y = crossing_axis_y(&plot, -8.0, 8.0, 2.0, -8.0);
        let max_y = crossing_axis_y(&plot, -8.0, 8.0, 2.0, 8.0);
        let mut min_canvas = Canvas::new(300, 220);
        draw_bars(
            &mut min_canvas,
            &min_chart,
            &plot,
            -8.0,
            8.0,
            2.0,
            None,
            -8.0,
            false,
            None,
        );
        let mut below_min = 0u32;
        let mut min_fill = 0u32;
        for y in 0..min_canvas.h {
            for x in 0..min_canvas.w {
                let i = (y as usize * min_canvas.w as usize + x as usize) * 3;
                if !near(&min_canvas.px[i..i + 3]) {
                    continue;
                }
                min_fill += 1;
                if (y as i32) > min_y {
                    below_min += 1;
                }
            }
        }
        assert!(min_fill > 0, "crosses=min columns must fill from the min");
        assert_eq!(below_min, 0, "crosses=min columns must not grow past min");

        let mut max_canvas = Canvas::new(300, 220);
        draw_bars(
            &mut max_canvas,
            &max_chart,
            &plot,
            -8.0,
            8.0,
            2.0,
            None,
            8.0,
            false,
            None,
        );
        let mut above_max = 0u32;
        let mut max_fill = 0u32;
        for y in 0..max_canvas.h {
            for x in 0..max_canvas.w {
                let i = (y as usize * max_canvas.w as usize + x as usize) * 3;
                if !near(&max_canvas.px[i..i + 3]) {
                    continue;
                }
                max_fill += 1;
                if (y as i32) < max_y {
                    above_max += 1;
                }
            }
        }
        assert!(max_fill > 0, "crosses=max columns must fill from the max");
        assert_eq!(above_max, 0, "crosses=max columns must not grow past max");

        let min_png = render_rgb_chart(
            &min_doc.root,
            424,
            265,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .unwrap();
        let max_png = render_rgb_chart(
            &max_doc.root,
            424,
            265,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .unwrap();
        let at_max = with_cat_crosses_at(&mixed_column_explicit_pm8(&values), "8");
        let at_doc = crate::xml::parse(at_max.as_bytes()).unwrap();
        let at_png = render_rgb_chart(
            &at_doc.root,
            424,
            265,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .unwrap();
        let max_top = first_accent_row(&max_png);
        let at_top = first_accent_row(&at_png);
        assert!(
            max_top > at_top + 8,
            "official crosses=max keeps a top category-label gutter, unlike crossesAt=max; max={max_top} at={at_top}"
        );
        let _ = min_png;

        render_line(&with_cat_crosses(
            &standard_line_xml(
                &line_ser(0, "S", &[Some(-4.0), Some(-1.0), Some(3.0), Some(6.0)], ""),
                "",
            ),
            "min",
        ))
        .unwrap();
        render_line(&with_cat_crosses(
            &standard_line_xml(
                &line_ser(0, "S", &[Some(-4.0), Some(-1.0), Some(3.0), Some(6.0)], ""),
                "",
            ),
            "max",
        ))
        .unwrap();
        render_area(&with_cat_crosses(
            &standard_area_xml(
                &line_ser(0, "S", &[Some(-4.0), Some(-1.0), Some(3.0), Some(6.0)], ""),
                "",
            ),
            "min",
        ))
        .unwrap();
        let bar = with_cat_crosses(
            &bar_xml(
                &line_ser(0, "S", &[Some(-4.0), Some(-1.0), Some(3.0), Some(6.0)], ""),
                "",
                "maxMin",
                Some("bar"),
            ),
            "min",
        );
        assert!(bar.contains(r#"<c:crosses val="min"/>"#));
        let err = render_bar(&bar).expect_err("bar crosses=min stays NI");
        assert_eq!(err.code(), crate::error::ErrorCode::NotImplemented);
    }

    fn with_val_num_fmt(xml: &str, code: &str) -> String {
        let inject = format!(r#"<c:numFmt formatCode="{code}" sourceLinked="0"/>"#);
        let replaced = xml.replacen("</c:valAx>", &format!("{inject}</c:valAx>"), 1);
        assert!(
            replaced.contains(&inject),
            "fixture must carry valAx numFmt {code}"
        );
        replaced
    }

    fn mixed_column_thousands(values: &[f64]) -> String {
        mixed_column_xml(values)
            .replace(r#"<c:min val="-3"/>"#, r#"<c:min val="-8000"/>"#)
            .replace(r#"<c:max val="5"/>"#, r#"<c:max val="8000"/>"#)
            .replace(r#"<c:majorUnit val="1"/>"#, r#"<c:majorUnit val="2000"/>"#)
    }

    fn first_axis_x(png: &[u8]) -> u32 {
        let (w, h, rgb) = png_rgb(png);
        let y0 = h / 4;
        let y1 = (h * 3) / 4;
        let mut best_x = 0u32;
        let mut best_n = 0u32;
        for x in 0..w / 2 {
            let mut n = 0u32;
            for y in y0..y1 {
                let i = (y as usize * w as usize + x as usize) * 3;
                if rgb[i] < 40 && rgb[i + 1] < 40 && rgb[i + 2] < 40 {
                    n += 1;
                }
            }
            if n > best_n {
                best_n = n;
                best_x = x;
            }
        }
        assert!(
            best_n > (y1 - y0) / 2,
            "expected a dark vertical value axis in the left half"
        );
        best_x
    }

    #[test]
    fn val_axis_locked_format_codes_match_official_tick_text() {
        use super::axis::TickFormat;
        let ticks = [-8.0, -6.0, -4.0, -2.0, 0.0, 2.0, 4.0, 6.0, 8.0];
        assert_eq!(
            ticks.map(|v| fmt_tick_with(v, TickFormat::General)),
            ["-8", "-6", "-4", "-2", "0", "2", "4", "6", "8"]
        );
        assert_eq!(
            ticks.map(|v| fmt_tick_with(v, TickFormat::Integer)),
            ["-8", "-6", "-4", "-2", "0", "2", "4", "6", "8"]
        );
        assert_eq!(
            ticks.map(|v| fmt_tick_with(v, TickFormat::Fixed2)),
            [
                "-8.00", "-6.00", "-4.00", "-2.00", "0.00", "2.00", "4.00", "6.00",
                "8.00"
            ]
        );
        assert_eq!(
            ticks.map(|v| fmt_tick_with(v, TickFormat::Percent)),
            [
                "-800%", "-600%", "-400%", "-200%", "0%", "200%", "400%", "600%",
                "800%"
            ]
        );
        assert_eq!(
            ticks.map(|v| fmt_tick_with(v, TickFormat::PercentFixed2)),
            [
                "-800.00%",
                "-600.00%",
                "-400.00%",
                "-200.00%",
                "0.00%",
                "200.00%",
                "400.00%",
                "600.00%",
                "800.00%"
            ]
        );
        assert_eq!(
            ticks.map(|v| fmt_tick_with(v, TickFormat::Scientific2)),
            [
                "-8.00E+00",
                "-6.00E+00",
                "-4.00E+00",
                "-2.00E+00",
                "0.00E+00",
                "2.00E+00",
                "4.00E+00",
                "6.00E+00",
                "8.00E+00"
            ]
        );
        let thousands = [-8000.0, -6000.0, -2000.0, 0.0, 2000.0, 8000.0];
        assert_eq!(
            thousands.map(|v| fmt_tick_with(v, TickFormat::Thousands)),
            ["-8,000", "-6,000", "-2,000", "0", "2,000", "8,000"]
        );
        assert_eq!(fmt_tick_with(-0.0, TickFormat::Fixed2), "0.00");
        assert_eq!(fmt_tick_with(-0.0, TickFormat::Percent), "0%");
        assert_eq!(fmt_tick_with(-0.0, TickFormat::Scientific2), "0.00E+00");
    }

    #[test]
    fn val_axis_non_general_format_codes_widen_labels_and_keep_unlocked_ni() {
        let values = [-4.0, -1.0, 3.0, 6.0];
        let general = mixed_column_explicit_pm8(&values);
        let general_doc = crate::xml::parse(general.as_bytes()).unwrap();
        let general_chart = read_chart(
            &general_doc.root,
            FALLBACK_ACCENTS,
            None,
            "",
            RenderDpi::DEFAULT,
        )
        .unwrap();
        assert_eq!(general_chart.axis.tick_format, super::axis::TickFormat::General);

        let fixed = with_val_num_fmt(&general, "0.00");
        let fixed_doc = crate::xml::parse(fixed.as_bytes()).unwrap();
        let fixed_chart = read_chart(
            &fixed_doc.root,
            FALLBACK_ACCENTS,
            None,
            "",
            RenderDpi::DEFAULT,
        )
        .unwrap();
        assert_eq!(fixed_chart.axis.tick_format, super::axis::TickFormat::Fixed2);

        let general_png = render_rgb_chart(
            &general_doc.root,
            424,
            265,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .unwrap();
        let fixed_png = render_rgb_chart(
            &fixed_doc.root,
            424,
            265,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .unwrap();
        let general_x = first_axis_x(&general_png);
        let fixed_x = first_axis_x(&fixed_png);
        assert!(
            fixed_x > general_x + 8,
            "0.00 tick labels must widen the left gutter; general={general_x} fixed={fixed_x}"
        );

        let integer = with_val_num_fmt(&general, "0");
        let integer_doc = crate::xml::parse(integer.as_bytes()).unwrap();
        let integer_png = render_rgb_chart(
            &integer_doc.root,
            424,
            265,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .unwrap();
        let integer_x = first_axis_x(&integer_png);
        assert!(
            (integer_x as i32 - general_x as i32).abs() <= 1,
            "format 0 on integer ticks matches General gutter; general={general_x} integer={integer_x}"
        );

        for code in ["0%", "0.00%", "0.00E+00"] {
            render_rgb_chart(
                &crate::xml::parse(with_val_num_fmt(&general, code).as_bytes())
                    .unwrap()
                    .root,
                424,
                265,
                FALLBACK_ACCENTS,
                None,
                ChartHost::Excel,
                None,
                "",
            )
            .unwrap();
        }
        let thousands = with_val_num_fmt(
            &mixed_column_thousands(&[-4000.0, -1000.0, 3000.0, 6000.0]),
            "#,##0",
        );
        render_rgb_chart(
            &crate::xml::parse(thousands.as_bytes()).unwrap().root,
            424,
            265,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .unwrap();
        render_line(&with_val_num_fmt(
            &standard_line_xml(
                &line_ser(0, "S", &[Some(-4.0), Some(-1.0), Some(3.0), Some(6.0)], ""),
                "",
            ),
            "0.00",
        ))
        .unwrap();

        let currency = with_val_num_fmt(&general, "$#,##0.00");
        let err = render_rgb_chart(
            &crate::xml::parse(currency.as_bytes()).unwrap().root,
            424,
            265,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .expect_err("currency formatCode stays NI");
        assert_eq!(err.code(), crate::error::ErrorCode::NotImplemented);
    }

    fn with_val_orientation(xml: &str, mode: &str) -> String {
        let start = xml.find("<c:valAx").expect("fixture must have valAx");
        let rel_end = xml[start..]
            .find("</c:valAx>")
            .expect("fixture must close valAx");
        let end = start + rel_end;
        let val = &xml[start..end];
        let inject = format!(r#"<c:orientation val="{mode}"/>"#);
        let new_val = if let Some((head, tail)) = val.split_once("<c:orientation val=\"") {
            let close = tail.find('"').expect("orientation close");
            format!("{}<c:orientation val=\"{}{}", head, mode, &tail[close..])
        } else if val.contains("<c:scaling>") {
            val.replacen("<c:scaling>", &format!("<c:scaling>{inject}"), 1)
        } else {
            let gt = val.find('>').expect("valAx open tag");
            format!(
                "{}<c:scaling>{inject}</c:scaling>{}",
                &val[..=gt],
                &val[gt + 1..]
            )
        };
        let out = format!("{}{}{}", &xml[..start], new_val, &xml[end..]);
        assert!(
            out[start..].contains(&inject),
            "fixture must carry valAx orientation {mode}"
        );
        out
    }

    #[test]
    fn val_axis_orientation_max_min_flips_columns_lines_and_bars() {
        let values = [-4.0, -1.0, 3.0, 6.0];
        let mixed = mixed_column_explicit_pm8(&values).replacen(
            "<c:ser>",
            r#"<c:ser><c:invertIfNegative val="0"/>"#,
            1,
        );
        let xml = with_val_orientation(&mixed, "maxMin");
        let doc = crate::xml::parse(xml.as_bytes()).unwrap();
        let chart = read_chart(
            &doc.root,
            FALLBACK_ACCENTS,
            None,
            "",
            RenderDpi::DEFAULT,
        )
        .unwrap();
        assert!(chart.axis.reversed);
        let mixed_scale = super::axis::scale(&chart.series, chart.axis, 10).unwrap();
        assert!(!mixed_scale.category_labels_above());
        assert!(!mixed_scale.category_labels_below());

        let png = render_rgb_chart(
            &doc.root,
            424,
            265,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .unwrap();
        let minmax_doc = crate::xml::parse(mixed.as_bytes()).unwrap();
        let minmax_png = render_rgb_chart(
            &minmax_doc.root,
            424,
            265,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .unwrap();
        let (w, h, rgb) = png_rgb(&png);
        let (_, _, minmax_rgb) = png_rgb(&minmax_png);
        let accent = FALLBACK_ACCENTS[0];
        let near = |px: &[u8]| {
            (px[0] as i16 - accent[0] as i16).abs() < 8
                && (px[1] as i16 - accent[1] as i16).abs() < 8
                && (px[2] as i16 - accent[2] as i16).abs() < 8
        };
        let centroid_y = |rgb: &[u8], x0: u32, x1: u32| {
            let mut sum = 0u64;
            let mut n = 0u64;
            for y in 0..h {
                for x in x0..x1 {
                    let i = (y as usize * w as usize + x as usize) * 3;
                    if near(&rgb[i..i + 3]) {
                        sum += u64::from(y);
                        n += 1;
                    }
                }
            }
            assert!(n > 0, "expected accent fill in x={x0}..{x1}");
            sum as f64 / n as f64
        };
        // Official F19-2: negative A/B grow upward (toward -8 at the top).
        // The first category (A=-4) occupies the left of the plot. Do not use
        // the longest dark row as the crossing: major grids are the same length
        // as the category axis and the top grid wins first.
        let mid = w / 2;
        let left_maxmin = centroid_y(&rgb, 0, mid);
        let right_maxmin = centroid_y(&rgb, mid, w);
        assert!(
            left_maxmin + 8.0 < right_maxmin,
            "official maxMin grows negative columns toward the top; left={left_maxmin} right={right_maxmin}"
        );
        let left_minmax = centroid_y(&minmax_rgb, 0, mid);
        let right_minmax = centroid_y(&minmax_rgb, mid, w);
        assert!(
            left_minmax > right_minmax + 8.0,
            "minMax control grows negative columns downward; left={left_minmax} right={right_minmax}"
        );

        let pos = with_val_orientation(
            &mixed_column_xml(&[2.0, 4.0, 6.0, 8.0])
                .replace(r#"<c:min val="-3"/>"#, r#"<c:min val="0"/>"#)
                .replace(r#"<c:max val="5"/>"#, r#"<c:max val="10"/>"#)
                .replace(r#"<c:majorUnit val="1"/>"#, r#"<c:majorUnit val="2"/>"#),
            "maxMin",
        );
        let pos_doc = crate::xml::parse(pos.as_bytes()).unwrap();
        let pos_chart = read_chart(
            &pos_doc.root,
            FALLBACK_ACCENTS,
            None,
            "",
            RenderDpi::DEFAULT,
        )
        .unwrap();
        assert!(pos_chart.axis.reversed);
        let pos_scale = super::axis::scale(&pos_chart.series, pos_chart.axis, 10).unwrap();
        assert!(pos_scale.category_labels_above());
        assert!(!pos_scale.category_labels_below());
        let pos_png = render_rgb_chart(
            &pos_doc.root,
            424,
            265,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .unwrap();
        let pos_minmax_xml = mixed_column_xml(&[2.0, 4.0, 6.0, 8.0])
            .replace(r#"<c:min val="-3"/>"#, r#"<c:min val="0"/>"#)
            .replace(r#"<c:max val="5"/>"#, r#"<c:max val="10"/>"#)
            .replace(r#"<c:majorUnit val="1"/>"#, r#"<c:majorUnit val="2"/>"#);
        let pos_minmax_png = render_rgb_chart(
            &crate::xml::parse(pos_minmax_xml.as_bytes()).unwrap().root,
            424,
            265,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .unwrap();
        let dark_band = |png: &[u8], y0: u32, y1: u32| {
            let (w, _, rgb) = png_rgb(png);
            let mut n = 0u32;
            for y in y0..y1 {
                for x in 0..w {
                    let i = (y as usize * w as usize + x as usize) * 3;
                    if rgb[i] < 40 && rgb[i + 1] < 40 && rgb[i + 2] < 40 {
                        n += 1;
                    }
                }
            }
            n
        };
        let (_, ph, _) = png_rgb(&pos_png);
        let top = dark_band(&pos_png, 0, ph / 6);
        let bottom = dark_band(&pos_png, (ph * 5) / 6, ph);
        let top_ctrl = dark_band(&pos_minmax_png, 0, ph / 6);
        let bottom_ctrl = dark_band(&pos_minmax_png, (ph * 5) / 6, ph);
        assert!(
            top > bottom && top > top_ctrl,
            "positive maxMin keeps category labels in the top gutter; top={top} bottom={bottom} ctrl_top={top_ctrl} ctrl_bottom={bottom_ctrl}"
        );
        assert!(
            bottom_ctrl > bottom,
            "positive minMax keeps category labels in the bottom gutter; bottom={bottom} ctrl_bottom={bottom_ctrl}"
        );

        let neg = with_val_orientation(
            &mixed_column_xml(&[-8.0, -5.0, -3.0, -1.0])
                .replace(r#"<c:min val="-3"/>"#, r#"<c:min val="-10"/>"#)
                .replace(r#"<c:max val="5"/>"#, r#"<c:max val="0"/>"#)
                .replace(r#"<c:majorUnit val="1"/>"#, r#"<c:majorUnit val="2"/>"#),
            "maxMin",
        );
        let neg_doc = crate::xml::parse(neg.as_bytes()).unwrap();
        let neg_chart = read_chart(
            &neg_doc.root,
            FALLBACK_ACCENTS,
            None,
            "",
            RenderDpi::DEFAULT,
        )
        .unwrap();
        let neg_scale = super::axis::scale(&neg_chart.series, neg_chart.axis, 10).unwrap();
        assert!(neg_scale.category_labels_below());
        assert!(!neg_scale.category_labels_above());
        render_rgb_chart(
            &neg_doc.root,
            424,
            265,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .unwrap();

        render_line(&with_val_orientation(
            &standard_line_xml(
                &line_ser(0, "S", &[Some(-4.0), Some(-1.0), Some(3.0), Some(6.0)], ""),
                "",
            ),
            "maxMin",
        ))
        .unwrap();

        let bar_vals = [Some(2.0), Some(4.0), Some(6.0), Some(8.0)];
        let bar_base = bar_xml(&line_ser(0, "S", &bar_vals, ""), "", "minMax", Some("bar"));
        let bar_png = render_bar(&with_val_orientation(&bar_base, "maxMin")).unwrap();
        let minmax_bar = render_bar(&bar_base).unwrap();
        let (bw, bh, brgb) = png_rgb(&bar_png);
        let (_, _, mrgb) = png_rgb(&minmax_bar);
        let accent = FALLBACK_ACCENTS[0];
        let near = |px: &[u8]| {
            (px[0] as i16 - accent[0] as i16).abs() < 8
                && (px[1] as i16 - accent[1] as i16).abs() < 8
                && (px[2] as i16 - accent[2] as i16).abs() < 8
        };
        let centroid_x = |rgb: &[u8]| {
            let mut sum = 0u64;
            let mut n = 0u64;
            for y in 0..bh {
                for x in 0..bw {
                    let i = (y as usize * bw as usize + x as usize) * 3;
                    if near(&rgb[i..i + 3]) {
                        sum += u64::from(x);
                        n += 1;
                    }
                }
            }
            assert!(n > 0, "expected horizontal-bar accent fill");
            sum as f64 / n as f64
        };
        let maxmin_x = centroid_x(&brgb);
        let minmax_x = centroid_x(&mrgb);
        // Positive bars grow from the visual zero. Official F19-7 puts max
        // on the left, so autoZero sits on the right and the fill centroid
        // stays right of a minMax control (whose zero is on the left).
        assert!(
            maxmin_x > minmax_x + 8.0,
            "official bar maxMin places max on the left so positives grow left from a right-hand zero; maxmin={maxmin_x} minmax={minmax_x}"
        );
        // Official F19-7 writes catAx axPos=r with valAx orientation=maxMin.
        render_bar(&with_val_orientation(
            &bar_base.replace(r#"<c:axPos val="l"/>"#, r#"<c:axPos val="r"/>"#),
            "maxMin",
        ))
        .unwrap();

        let log = with_val_orientation(
            &inject_explicit_log10(&mixed_column_xml(&[1.0, 10.0, 100.0, 1000.0])),
            "maxMin",
        );
        let log_doc = crate::xml::parse(log.as_bytes()).unwrap();
        assert!(
            read_chart(&log_doc.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT).is_err(),
            "log plus orientation=maxMin stays NI"
        );

        let labeled = xml.replacen(
            "</c:ser>",
            r#"<c:dLbls><c:showVal val="1"/></c:dLbls></c:ser>"#,
            1,
        );
        let labeled_doc = crate::xml::parse(labeled.as_bytes()).unwrap();
        let err = render_rgb_chart(
            &labeled_doc.root,
            424,
            265,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .expect_err("visible value labels plus orientation=maxMin stay NI");
        assert_eq!(err.code(), crate::error::ErrorCode::NotImplemented);
    }

    fn with_val_delete(xml: &str) -> String {
        let start = xml.find("<c:valAx").expect("fixture must have valAx");
        let rel_end = xml[start..]
            .find("</c:valAx>")
            .expect("fixture must close valAx");
        let end = start + rel_end;
        let val = &xml[start..end];
        let new_val = if let Some((head, tail)) = val.split_once(r#"<c:delete val=""#) {
            let close = tail.find('"').expect("delete close");
            format!(r#"{}<c:delete val="1{}"#, head, &tail[close..])
        } else {
            let gt = val.find('>').expect("valAx open tag");
            format!(r#"{}<c:delete val="1"/>{}"#, &val[..=gt], &val[gt + 1..])
        };
        let out = format!("{}{}{}", &xml[..start], new_val, &xml[end..]);
        assert!(
            out[start..].contains(r#"<c:delete val="1"/>"#),
            "fixture must carry valAx delete=true"
        );
        out
    }

    #[test]
    fn val_axis_delete_hides_axis_line_and_releases_gutter() {
        let values = [-4.0, -1.0, 3.0, 6.0];
        let mixed = mixed_column_explicit_pm8(&values).replacen(
            "<c:ser>",
            r#"<c:ser><c:invertIfNegative val="0"/>"#,
            1,
        );
        let control_doc = crate::xml::parse(mixed.as_bytes()).unwrap();
        let delete_doc = crate::xml::parse(with_val_delete(&mixed).as_bytes()).unwrap();
        let control_chart = read_chart(
            &control_doc.root,
            FALLBACK_ACCENTS,
            None,
            "",
            RenderDpi::DEFAULT,
        )
        .unwrap();
        let delete_chart = read_chart(
            &delete_doc.root,
            FALLBACK_ACCENTS,
            None,
            "",
            RenderDpi::DEFAULT,
        )
        .unwrap();
        assert!(!control_chart.axis.deleted);
        assert!(delete_chart.axis.deleted);

        let render = |root: &crate::xml::Element| {
            render_rgb_chart(
                root,
                424,
                265,
                FALLBACK_ACCENTS,
                None,
                ChartHost::Excel,
                None,
                "",
            )
            .unwrap()
        };
        let control_png = render(&control_doc.root);
        let delete_png = render(&delete_doc.root);
        let (w, h, crgb) = png_rgb(&control_png);
        let (_, _, drgb) = png_rgb(&delete_png);
        let accent = FALLBACK_ACCENTS[0];
        let near = |px: &[u8]| {
            (px[0] as i16 - accent[0] as i16).abs() < 8
                && (px[1] as i16 - accent[1] as i16).abs() < 8
                && (px[2] as i16 - accent[2] as i16).abs() < 8
        };
        let first_accent_x = |rgb: &[u8]| {
            for x in 0..w {
                for y in 0..h {
                    let i = (y as usize * w as usize + x as usize) * 3;
                    if near(&rgb[i..i + 3]) {
                        return x;
                    }
                }
            }
            panic!("expected accent fill");
        };
        let left_x = first_accent_x(&crgb);
        let del_x = first_accent_x(&drgb);
        assert!(
            del_x + 8 < left_x,
            "official delete=true releases the value-label gutter; delete={del_x} control={left_x}"
        );
        let dark_left = |rgb: &[u8]| {
            let mut n = 0u32;
            for y in h / 6..(h * 5) / 6 {
                for x in 0..w / 8 {
                    let i = (y as usize * w as usize + x as usize) * 3;
                    if rgb[i] < 40 && rgb[i + 1] < 40 && rgb[i + 2] < 40 {
                        n += 1;
                    }
                }
            }
            n
        };
        let control_left = dark_left(&crgb);
        let delete_left = dark_left(&drgb);
        assert!(
            control_left > 20 && delete_left + 10 < control_left,
            "official delete=true omits value tick labels; control_left={control_left} delete_left={delete_left}"
        );

        render_line(&with_val_delete(&standard_line_xml(
            &line_ser(0, "S", &[Some(-4.0), Some(-1.0), Some(3.0), Some(6.0)], ""),
            "",
        )))
        .unwrap();

        let bar_base = bar_xml(
            &line_ser(0, "S", &[Some(-4.0), Some(-1.0), Some(3.0), Some(6.0)], ""),
            "",
            "minMax",
            Some("bar"),
        );
        let bar_ctrl = render_bar(&bar_base).unwrap();
        let bar_del = render_bar(&with_val_delete(&bar_base)).unwrap();
        let (bw, bh, brgb) = png_rgb(&bar_ctrl);
        let (_, _, bdrgb) = png_rgb(&bar_del);
        let bottom_dark = |rgb: &[u8]| {
            let mut n = 0u32;
            for y in (bh * 5) / 6..bh {
                for x in 0..bw {
                    let i = (y as usize * bw as usize + x as usize) * 3;
                    if rgb[i] < 40 && rgb[i + 1] < 40 && rgb[i + 2] < 40 {
                        n += 1;
                    }
                }
            }
            n
        };
        let bar_bottom = bottom_dark(&brgb);
        let bar_del_bottom = bottom_dark(&bdrgb);
        assert!(
            bar_bottom > bar_del_bottom + 10,
            "official bar delete=true releases the bottom value-label gutter; control={bar_bottom} delete={bar_del_bottom}"
        );

        let log = with_val_delete(&inject_explicit_log10(&mixed_column_xml(&[
            1.0, 10.0, 100.0, 1000.0,
        ])));
        let log_doc = crate::xml::parse(log.as_bytes()).unwrap();
        assert!(
            read_chart(&log_doc.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT).is_err(),
            "log plus valAx delete=true stays NI"
        );
    }

    fn secondary_pair_xml(kind: &str, s1: &[f64], s2: &[f64], auto: bool) -> String {
        let n = s1.len();
        let cats = (0..n)
            .map(|i| format!(r#"<c:pt idx="{i}"><c:v>{}</c:v></c:pt>"#, (b'A' + i as u8) as char))
            .collect::<String>();
        let nums = |vals: &[f64]| {
            vals.iter()
                .enumerate()
                .map(|(i, v)| format!(r#"<c:pt idx="{i}"><c:v>{v}</c:v></c:pt>"#))
                .collect::<String>()
        };
        let scale_p = if auto {
            String::new()
        } else if s1.iter().any(|v| *v < 0.0) {
            r#"<c:min val="-8"/><c:max val="8"/>"#.into()
        } else {
            r#"<c:min val="0"/><c:max val="10"/>"#.into()
        };
        let major_p = if auto {
            String::new()
        } else if s1.iter().any(|v| *v < 0.0) {
            r#"<c:majorUnit val="2"/>"#.into()
        } else {
            r#"<c:majorUnit val="2"/>"#.into()
        };
        let scale_s = if auto {
            String::new()
        } else {
            r#"<c:min val="0"/><c:max val="100"/>"#.into()
        };
        let major_s = if auto {
            String::new()
        } else {
            r#"<c:majorUnit val="20"/>"#.into()
        };
        let (w1, w2) = match kind {
            "line" => (
                format!(
                    r#"<c:lineChart><c:grouping val="standard"/><c:varyColors val="0"/><c:ser><c:order val="0"/><c:tx><c:v>S1</c:v></c:tx><c:cat><c:strLit><c:ptCount val="{n}"/>{cats}</c:strLit></c:cat><c:val><c:numLit><c:ptCount val="{n}"/>{n1}</c:numLit></c:val></c:ser><c:axId val="1"/><c:axId val="2"/></c:lineChart>"#,
                    n = n,
                    cats = cats,
                    n1 = nums(s1),
                ),
                format!(
                    r#"<c:lineChart><c:grouping val="standard"/><c:varyColors val="0"/><c:ser><c:order val="1"/><c:tx><c:v>S2</c:v></c:tx><c:cat><c:strLit><c:ptCount val="{n}"/>{cats}</c:strLit></c:cat><c:val><c:numLit><c:ptCount val="{n}"/>{n2}</c:numLit></c:val></c:ser><c:axId val="3"/><c:axId val="4"/></c:lineChart>"#,
                    n = n,
                    cats = cats,
                    n2 = nums(s2),
                ),
            ),
            "bar" => (
                format!(
                    r#"<c:barChart><c:barDir val="bar"/><c:grouping val="clustered"/><c:ser><c:order val="0"/><c:tx><c:v>S1</c:v></c:tx><c:cat><c:strLit><c:ptCount val="{n}"/>{cats}</c:strLit></c:cat><c:val><c:numLit><c:ptCount val="{n}"/>{n1}</c:numLit></c:val></c:ser><c:gapWidth val="150"/><c:axId val="1"/><c:axId val="2"/></c:barChart>"#,
                    n = n,
                    cats = cats,
                    n1 = nums(s1),
                ),
                format!(
                    r#"<c:barChart><c:barDir val="bar"/><c:grouping val="clustered"/><c:ser><c:order val="1"/><c:tx><c:v>S2</c:v></c:tx><c:cat><c:strLit><c:ptCount val="{n}"/>{cats}</c:strLit></c:cat><c:val><c:numLit><c:ptCount val="{n}"/>{n2}</c:numLit></c:val></c:ser><c:gapWidth val="150"/><c:axId val="3"/><c:axId val="4"/></c:barChart>"#,
                    n = n,
                    cats = cats,
                    n2 = nums(s2),
                ),
            ),
            "combo" => (
                format!(
                    r#"<c:barChart><c:barDir val="col"/><c:grouping val="clustered"/><c:ser><c:order val="0"/><c:tx><c:v>S1</c:v></c:tx><c:cat><c:strLit><c:ptCount val="{n}"/>{cats}</c:strLit></c:cat><c:val><c:numLit><c:ptCount val="{n}"/>{n1}</c:numLit></c:val></c:ser><c:gapWidth val="150"/><c:axId val="1"/><c:axId val="2"/></c:barChart>"#,
                    n = n,
                    cats = cats,
                    n1 = nums(s1),
                ),
                format!(
                    r#"<c:lineChart><c:grouping val="standard"/><c:varyColors val="0"/><c:ser><c:order val="1"/><c:tx><c:v>S2</c:v></c:tx><c:cat><c:strLit><c:ptCount val="{n}"/>{cats}</c:strLit></c:cat><c:val><c:numLit><c:ptCount val="{n}"/>{n2}</c:numLit></c:val></c:ser><c:axId val="3"/><c:axId val="4"/></c:lineChart>"#,
                    n = n,
                    cats = cats,
                    n2 = nums(s2),
                ),
            ),
            _ => (
                format!(
                    r#"<c:barChart><c:barDir val="col"/><c:grouping val="clustered"/><c:ser><c:order val="0"/><c:tx><c:v>S1</c:v></c:tx><c:cat><c:strLit><c:ptCount val="{n}"/>{cats}</c:strLit></c:cat><c:val><c:numLit><c:ptCount val="{n}"/>{n1}</c:numLit></c:val></c:ser><c:gapWidth val="150"/><c:axId val="1"/><c:axId val="2"/></c:barChart>"#,
                    n = n,
                    cats = cats,
                    n1 = nums(s1),
                ),
                format!(
                    r#"<c:barChart><c:barDir val="col"/><c:grouping val="clustered"/><c:ser><c:order val="1"/><c:tx><c:v>S2</c:v></c:tx><c:cat><c:strLit><c:ptCount val="{n}"/>{cats}</c:strLit></c:cat><c:val><c:numLit><c:ptCount val="{n}"/>{n2}</c:numLit></c:val></c:ser><c:gapWidth val="150"/><c:axId val="3"/><c:axId val="4"/></c:barChart>"#,
                    n = n,
                    cats = cats,
                    n2 = nums(s2),
                ),
            ),
        };
        format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart">
  <c:chart>
    <c:plotArea>
      {w1}{w2}
      <c:catAx><c:axId val="1"/><c:scaling><c:orientation val="minMax"/></c:scaling><c:delete val="0"/><c:axPos val="b"/><c:crossAx val="2"/><c:crosses val="autoZero"/></c:catAx>
      <c:valAx><c:axId val="2"/><c:scaling><c:orientation val="minMax"/>{scale_p}</c:scaling>{major_p}<c:delete val="0"/><c:axPos val="l"/><c:majorGridlines/><c:crossAx val="1"/><c:crosses val="autoZero"/><c:crossBetween val="between"/></c:valAx>
      <c:valAx><c:axId val="4"/><c:scaling><c:orientation val="minMax"/>{scale_s}</c:scaling>{major_s}<c:delete val="0"/><c:axPos val="r"/><c:crossAx val="3"/><c:crosses val="max"/><c:crossBetween val="between"/></c:valAx>
      <c:catAx><c:axId val="3"/><c:scaling><c:orientation val="minMax"/></c:scaling><c:delete val="1"/><c:axPos val="b"/><c:crossAx val="4"/></c:catAx>
    </c:plotArea>
    <c:legend><c:legendPos val="r"/><c:overlay val="0"/></c:legend>
  </c:chart>
</c:chartSpace>"#
        )
    }

    fn two_series_primary_xml(s1: &[f64], s2: &[f64]) -> String {
        let n = s1.len();
        let cats = (0..n)
            .map(|i| format!(r#"<c:pt idx="{i}"><c:v>{}</c:v></c:pt>"#, (b'A' + i as u8) as char))
            .collect::<String>();
        let nums = |vals: &[f64]| {
            vals.iter()
                .enumerate()
                .map(|(i, v)| format!(r#"<c:pt idx="{i}"><c:v>{v}</c:v></c:pt>"#))
                .collect::<String>()
        };
        format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart">
  <c:chart>
    <c:plotArea>
      <c:barChart>
        <c:barDir val="col"/><c:grouping val="clustered"/>
        <c:ser><c:order val="0"/><c:tx><c:v>S1</c:v></c:tx><c:cat><c:strLit><c:ptCount val="{n}"/>{cats}</c:strLit></c:cat><c:val><c:numLit><c:ptCount val="{n}"/>{n1}</c:numLit></c:val></c:ser>
        <c:ser><c:order val="1"/><c:tx><c:v>S2</c:v></c:tx><c:cat><c:strLit><c:ptCount val="{n}"/>{cats}</c:strLit></c:cat><c:val><c:numLit><c:ptCount val="{n}"/>{n2}</c:numLit></c:val></c:ser>
        <c:gapWidth val="150"/><c:axId val="1"/><c:axId val="2"/>
      </c:barChart>
      <c:catAx><c:axId val="1"/><c:scaling><c:orientation val="minMax"/></c:scaling><c:delete val="0"/><c:axPos val="b"/><c:crossAx val="2"/><c:crosses val="autoZero"/></c:catAx>
      <c:valAx><c:axId val="2"/><c:scaling><c:orientation val="minMax"/><c:min val="0"/><c:max val="100"/></c:scaling><c:majorUnit val="20"/><c:delete val="0"/><c:axPos val="l"/><c:majorGridlines/><c:crossAx val="1"/><c:crosses val="autoZero"/><c:crossBetween val="between"/></c:valAx>
    </c:plotArea>
    <c:legend><c:legendPos val="r"/><c:overlay val="0"/></c:legend>
  </c:chart>
</c:chartSpace>"#,
            n = n,
            cats = cats,
            n1 = nums(s1),
            n2 = nums(s2),
        )
    }

    fn accent_x_runs(rgb: &[u8], w: u32, h: u32, accent: [u8; 3]) -> Vec<(u32, u32)> {
        let mut xs = Vec::new();
        for x in 0..w {
            let mut n = 0u32;
            for y in 0..h {
                let i = (y as usize * w as usize + x as usize) * 3;
                if near([rgb[i], rgb[i + 1], rgb[i + 2]], accent, 16) {
                    n += 1;
                }
            }
            if n >= 3 {
                xs.push(x);
            }
        }
        let mut runs = Vec::new();
        if xs.is_empty() {
            return runs;
        }
        let mut start = xs[0];
        let mut prev = xs[0];
        for x in xs.into_iter().skip(1) {
            if x <= prev + 2 {
                prev = x;
            } else {
                // drop slim legend swatches
                if prev - start >= 8 {
                    runs.push((start, prev));
                }
                start = x;
                prev = x;
            }
        }
        if prev - start >= 8 {
            runs.push((start, prev));
        }
        runs
    }

    #[test]
    fn secondary_val_axis_maps_own_scale_and_overlaps_same_type_columns() {
        let s1 = [2.0, 4.0, 6.0, 8.0];
        let s2 = [20.0, 40.0, 80.0, 60.0];
        let xml = secondary_pair_xml("col", &s1, &s2, false);
        let doc = crate::xml::parse(xml.as_bytes()).unwrap();
        let chart = read_chart(&doc.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT).unwrap();
        assert!(chart.axis_secondary.is_some());
        assert_eq!(chart.axis.min, Some(0.0));
        assert_eq!(chart.axis.max, Some(10.0));
        let sec = chart.axis_secondary.unwrap();
        assert_eq!(sec.min, Some(0.0));
        assert_eq!(sec.max, Some(100.0));
        assert!(!chart.series[0].on_secondary);
        assert!(chart.series[1].on_secondary);

        let png = render_rgb_chart(
            &doc.root,
            424,
            265,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .unwrap();
        let (w, h, rgb) = png_rgb(&png);
        let a0 = FALLBACK_ACCENTS[0];
        let a1 = FALLBACK_ACCENTS[1];
        let r0 = accent_x_runs(&rgb, w, h, a0);
        let r1 = accent_x_runs(&rgb, w, h, a1);
        // Official same-type secondary columns overlap in the same slots.
        // Secondary is painted on top, so S1 is only visible in D (8/10 vs 60/100).
        assert_eq!(r1.len(), 4, "secondary series must occupy four category slots, got {r1:?}");
        assert_eq!(
            r0.len(),
            1,
            "primary S1 should only stick out above S2 in category D, got {r0:?}"
        );
        assert!(
            r1.iter().any(|(s, e)| r0[0].0 <= *e && r0[0].1 >= *s),
            "official secondary columns overlap the primary slot; primary={r0:?} secondary={r1:?}"
        );
        let ctrl_doc = crate::xml::parse(two_series_primary_xml(&s1, &s2).as_bytes()).unwrap();
        let ctrl = read_chart(&ctrl_doc.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT).unwrap();
        assert!(ctrl.axis_secondary.is_none());
        let ctrl_png = render_rgb_chart(
            &ctrl_doc.root,
            424,
            265,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .unwrap();
        let (cw, ch, crgb) = png_rgb(&ctrl_png);
        let c0 = accent_x_runs(&crgb, cw, ch, a0);
        let c1 = accent_x_runs(&crgb, cw, ch, a1);
        assert_eq!(c0.len(), 4);
        assert_eq!(c1.len(), 4);
        // Control clusters sit side by side, so matching-index runs should not fully coincide.
        for ((a0s, a0e), (a1s, a1e)) in c0.iter().zip(&c1) {
            assert!(
                *a0e < *a1s || *a1e < *a0s,
                "one-axis two-series columns stay clustered side by side; {c0:?} {c1:?}"
            );
        }
        let secondary_w: i32 = r1.iter().map(|(s, e)| (*e - *s) as i32 + 1).sum();
        let cluster_w: i32 = c0.iter().map(|(s, e)| (*e - *s) as i32 + 1).sum();
        assert!(
            secondary_w > cluster_w,
            "each secondary-axis wrapper is a one-series cluster, so bars are wider than a two-series cluster; secondary_w={secondary_w} control_w={cluster_w}"
        );
        // Right-side tick labels: more ink in the right gutter than the one-axis control.
        let ink = |rgb: &[u8], w: u32, h: u32| {
            let mut n = 0u32;
            for y in h / 6..(h * 5) / 6 {
                for x in (w * 5) / 8..(w * 7) / 8 {
                    let i = (y as usize * w as usize + x as usize) * 3;
                    if rgb[i] < 40 && rgb[i + 1] < 40 && rgb[i + 2] < 40 {
                        n += 1;
                    }
                }
            }
            n
        };
        let right = ink(&rgb, w, h);
        let ctrl_right = ink(&crgb, cw, ch);
        assert!(
            right > ctrl_right + 10,
            "official secondary valAx places tick labels on the right; secondary={right} control={ctrl_right}"
        );

        render_line(&secondary_pair_xml(
            "line",
            &s1,
            &s2,
            false,
        ))
        .unwrap();

        let bar = crate::xml::parse(secondary_pair_xml("bar", &s1, &s2, false).as_bytes()).unwrap();
        assert!(
            read_chart(&bar.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT).is_err(),
            "clustered bar secondary valAx stays NI"
        );
        let combo = crate::xml::parse(secondary_pair_xml("combo", &s1, &s2, false).as_bytes()).unwrap();
        assert!(
            read_chart(&combo.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT).is_err(),
            "combo (different wrappers) stays NI for F26"
        );
        let auto = crate::xml::parse(secondary_pair_xml("col", &s1, &s2, true).as_bytes()).unwrap();
        assert!(
            read_chart(&auto.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT).is_err(),
            "omitted min/max on a secondary pair stays NI"
        );
    }

    fn stacked_xml(
        dir: &str,
        grouping: &str,
        series: &[&[Option<f64>]],
        auto: bool,
        invert: bool,
        labels: bool,
    ) -> String {
        let n = series[0].len();
        let cats = (0..n)
            .map(|i| format!(r#"<c:pt idx="{i}"><c:v>{}</c:v></c:pt>"#, (b'A' + i as u8) as char))
            .collect::<String>();
        let mut sers = String::new();
        for (si, vals) in series.iter().enumerate() {
            let pts = vals
                .iter()
                .enumerate()
                .filter_map(|(i, v)| {
                    v.map(|x| format!(r#"<c:pt idx="{i}"><c:v>{x}</c:v></c:pt>"#))
                })
                .collect::<String>();
            let present = vals.iter().filter(|v| v.is_some()).count();
            let inv = if invert { "1" } else { "0" };
            let d_lbls = if labels {
                r#"<c:dLbls><c:dLblPos val="outEnd"/><c:showVal val="1"/></c:dLbls>"#
            } else {
                ""
            };
            sers.push_str(&format!(
                r#"<c:ser><c:idx val="{si}"/><c:order val="{si}"/><c:tx><c:v>S{si}</c:v></c:tx><c:invertIfNegative val="{inv}"/><c:cat><c:strLit><c:ptCount val="{n}"/>{cats}</c:strLit></c:cat><c:val><c:numLit><c:ptCount val="{present}"/>{pts}</c:numLit></c:val>{d_lbls}</c:ser>"#
            ));
        }
        let (vmin, vmax, major, numfmt) = if grouping == "percentStacked" {
            (
                "0",
                "1",
                "0.2",
                r#"<c:numFmt formatCode="0%" sourceLinked="1"/>"#,
            )
        } else {
            ("0", "80", "20", "")
        };
        let scaling = if auto {
            r#"<c:scaling><c:orientation val="minMax"/></c:scaling>"#.to_string()
        } else {
            format!(
                r#"<c:scaling><c:orientation val="minMax"/><c:min val="{vmin}"/><c:max val="{vmax}"/></c:scaling>"#
            )
        };
        let (cat_pos, val_pos) = if dir == "bar" { ("l", "b") } else { ("b", "l") };
        format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart">
  <c:chart>
    <c:plotArea>
      <c:barChart>
        <c:barDir val="{dir}"/><c:grouping val="{grouping}"/><c:varyColors val="0"/>
        {sers}
        <c:gapWidth val="150"/><c:overlap val="100"/><c:axId val="1"/><c:axId val="2"/>
      </c:barChart>
      <c:catAx><c:axId val="1"/><c:scaling><c:orientation val="minMax"/></c:scaling><c:delete val="0"/><c:axPos val="{cat_pos}"/><c:crossAx val="2"/><c:crosses val="autoZero"/></c:catAx>
      <c:valAx><c:axId val="2"/>{scaling}<c:majorUnit val="{major}"/><c:delete val="0"/><c:axPos val="{val_pos}"/><c:majorGridlines/>{numfmt}<c:crossAx val="1"/><c:crosses val="autoZero"/><c:crossBetween val="between"/></c:valAx>
    </c:plotArea>
    <c:legend><c:legendPos val="r"/><c:overlay val="0"/></c:legend>
  </c:chart>
</c:chartSpace>"#
        )
    }

    fn clustered_three_xml(series: &[&[Option<f64>]]) -> String {
        stacked_xml("col", "stacked", series, false, false, false).replacen(
            r#"grouping val="stacked""#,
            r#"grouping val="clustered""#,
            1,
        ).replacen(r#"<c:overlap val="100"/>"#, "", 1)
    }

    #[test]
    fn stacked_and_percent_stacked_bars_accumulate_in_series_order() {
        let s1 = [Some(10.0), Some(20.0), Some(30.0), Some(40.0)];
        let s2 = [Some(20.0), Some(10.0), Some(20.0), Some(10.0)];
        let s3 = [Some(20.0), Some(30.0), Some(10.0), Some(20.0)];
        let series: [&[Option<f64>]; 3] = [&s1, &s2, &s3];
        let xml = stacked_xml("col", "stacked", &series, false, false, false);
        let doc = crate::xml::parse(xml.as_bytes()).unwrap();
        let chart = read_chart(&doc.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT).unwrap();
        assert_eq!(chart.stacking, Stacking::Stacked);
        assert_eq!(chart.overlap, 100.0);

        let png = render_rgb_chart(
            &doc.root, 424, 265, FALLBACK_ACCENTS, None, ChartHost::Excel, None, "",
        )
        .unwrap();
        let (w, h, rgb) = png_rgb(&png);
        let a0 = FALLBACK_ACCENTS[0];
        let a1 = FALLBACK_ACCENTS[1];
        let a2 = FALLBACK_ACCENTS[2];
        let r0 = accent_x_runs(&rgb, w, h, a0);
        let r1 = accent_x_runs(&rgb, w, h, a1);
        let r2 = accent_x_runs(&rgb, w, h, a2);
        assert_eq!(r0.len(), 4, "S1 four stacked slots, got {r0:?}");
        assert_eq!(r1.len(), 4, "S2 four stacked slots, got {r1:?}");
        assert_eq!(r2.len(), 4, "S3 four stacked slots, got {r2:?}");
        for i in 0..4 {
            let overlap01 = r0[i].0 <= r1[i].1 && r1[i].0 <= r0[i].1;
            let overlap12 = r1[i].0 <= r2[i].1 && r2[i].0 <= r1[i].1;
            assert!(
                overlap01 && overlap12,
                "official stacked columns share category slots; {r0:?} {r1:?} {r2:?}"
            );
        }

        let ctrl_doc = crate::xml::parse(clustered_three_xml(&series).as_bytes()).unwrap();
        let ctrl = read_chart(&ctrl_doc.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT).unwrap();
        assert_eq!(ctrl.stacking, Stacking::Clustered);
        let ctrl_png = render_rgb_chart(
            &ctrl_doc.root, 424, 265, FALLBACK_ACCENTS, None, ChartHost::Excel, None, "",
        )
        .unwrap();
        let (cw, ch, crgb) = png_rgb(&ctrl_png);
        let c0 = accent_x_runs(&crgb, cw, ch, a0);
        assert_eq!(c0.len(), 4);
        let stacked_w: i32 = r0.iter().map(|(s, e)| (*e - *s) as i32 + 1).sum();
        let cluster_w: i32 = c0.iter().map(|(s, e)| (*e - *s) as i32 + 1).sum();
        assert!(
            stacked_w > cluster_w,
            "overlap=100 stacked bars are one-series wide; stacked={stacked_w} clustered={cluster_w}"
        );

        // S1 sits on the zero baseline; S3 is farther from it (on top of the stack).
        let mid_x = (r0[0].0 + r0[0].1) / 2;
        let ys = |accent: [u8; 3]| {
            (0..h)
                .filter(|&y| {
                    let i = (y as usize * w as usize + mid_x as usize) * 3;
                    near([rgb[i], rgb[i + 1], rgb[i + 2]], accent, 16)
                })
                .collect::<Vec<_>>()
        };
        let y1 = ys(a0);
        let y3 = ys(a2);
        assert!(!y1.is_empty() && !y3.is_empty());
        let mean = |v: &Vec<u32>| v.iter().sum::<u32>() as f64 / v.len() as f64;
        assert!(
            mean(&y1) > mean(&y3),
            "S1 is at the base (higher y), S3 on top; S1={:?} S3={:?}",
            y1,
            y3
        );

        let pct_xml = stacked_xml("col", "percentStacked", &series, false, false, false);
        let pct_doc = crate::xml::parse(pct_xml.as_bytes()).unwrap();
        let pct = read_chart(&pct_doc.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT).unwrap();
        assert_eq!(pct.stacking, Stacking::PercentStacked);
        assert_eq!(pct.axis.min, Some(0.0));
        assert_eq!(pct.axis.max, Some(1.0));
        assert_eq!(pct.axis.tick_format, super::axis::TickFormat::Percent);
        let pct_png = render_rgb_chart(
            &pct_doc.root, 424, 265, FALLBACK_ACCENTS, None, ChartHost::Excel, None, "",
        )
        .unwrap();
        let (pw, ph, prgb) = png_rgb(&pct_png);
        let tops: Vec<u32> = (0..4)
            .map(|i| {
                let pr = accent_x_runs(&prgb, pw, ph, a0);
                let mid = (pr[i].0 + pr[i].1) / 2;
                (0..ph)
                    .filter(|&y| {
                        let idx = (y as usize * pw as usize + mid as usize) * 3;
                        near([prgb[idx], prgb[idx + 1], prgb[idx + 2]], a0, 16)
                            || near([prgb[idx], prgb[idx + 1], prgb[idx + 2]], a1, 16)
                            || near([prgb[idx], prgb[idx + 1], prgb[idx + 2]], a2, 16)
                    })
                    .min()
                    .unwrap()
            })
            .collect();
        let span = tops.iter().max().unwrap() - tops.iter().min().unwrap();
        assert!(
            span <= 3,
            "percentStacked all-positive stacks reach the same 100% top; tops={tops:?}"
        );

        render_rgb_chart(
            &crate::xml::parse(stacked_xml("bar", "stacked", &series, false, false, false).as_bytes())
                .unwrap()
                .root,
            424,
            265,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .unwrap();
        render_rgb_chart(
            &crate::xml::parse(
                stacked_xml("bar", "percentStacked", &series, false, false, false).as_bytes(),
            )
            .unwrap()
            .root,
            424,
            265,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .unwrap();

        // Mixed signs: positives stack up from zero, negatives down, independently.
        let m1 = [Some(10.0), Some(20.0), Some(-30.0), Some(40.0)];
        let m2 = [Some(20.0), Some(-10.0), Some(20.0), Some(-10.0)];
        let m3 = [Some(-20.0), Some(30.0), Some(10.0), Some(20.0)];
        let mixed: [&[Option<f64>]; 3] = [&m1, &m2, &m3];
        let mix_xml = stacked_xml("col", "stacked", &mixed, false, false, false)
            .replace(r#"min val="0""#, r#"min val="-40""#);
        let mix_doc = crate::xml::parse(mix_xml.as_bytes()).unwrap();
        read_chart(&mix_doc.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT).unwrap();
        let mix_png = render_rgb_chart(
            &mix_doc.root, 424, 265, FALLBACK_ACCENTS, None, ChartHost::Excel, None, "",
        )
        .unwrap();
        let (mw, mh, mrgb) = png_rgb(&mix_png);
        let mr0 = accent_x_runs(&mrgb, mw, mh, a0);
        assert_eq!(mr0.len(), 4, "mixed S1 still occupies four categories, {mr0:?}");

        // Blank S2 in category B: S3 packs onto S1 (no reserved hole).
        let b2 = [Some(20.0), None, Some(20.0), Some(10.0)];
        let blank: [&[Option<f64>]; 3] = [&s1, &b2, &s3];
        let blank_png = render_rgb_chart(
            &crate::xml::parse(stacked_xml("col", "stacked", &blank, false, false, false).as_bytes())
                .unwrap()
                .root,
            424,
            265,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .unwrap();
        let (bw, bh, brgb) = png_rgb(&blank_png);
        let br1 = accent_x_runs(&brgb, bw, bh, a1);
        assert_eq!(br1.len(), 3, "missing S2 in B is not drawn, got {br1:?}");

        let auto = crate::xml::parse(stacked_xml("col", "stacked", &series, true, false, false).as_bytes())
            .unwrap();
        assert!(
            read_chart(&auto.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT).is_err(),
            "omitted min/max on stacked stays NI"
        );
        let inv = crate::xml::parse(stacked_xml("col", "stacked", &series, false, true, false).as_bytes())
            .unwrap();
        assert!(
            read_chart(&inv.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT).is_err(),
            "invertIfNegative=1 on stacked stays NI"
        );
        let labs = crate::xml::parse(stacked_xml("col", "stacked", &series, false, false, true).as_bytes())
            .unwrap();
        assert!(
            read_chart(&labs.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT).is_err(),
            "value labels plus stacked stay NI"
        );
        let pct_mixed = crate::xml::parse(stacked_xml("col", "percentStacked", &mixed, false, false, false).as_bytes())
            .unwrap();
        assert!(
            read_chart(&pct_mixed.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT).is_err(),
            "percentStacked mixed signs stay NI"
        );
        let no_overlap = stacked_xml("col", "stacked", &series, false, false, false)
            .replacen(r#"<c:overlap val="100"/>"#, "", 1);
        let no_over_doc = crate::xml::parse(no_overlap.as_bytes()).unwrap();
        assert!(
            read_chart(&no_over_doc.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT).is_err(),
            "stacked without official overlap=100 stays NI"
        );
    }

    fn line_area_stacked_xml(
        kind: &str,
        grouping: &str,
        series: &[&[Option<f64>]],
        auto: bool,
        min: &str,
        max: &str,
        midcat: bool,
        marker_none: bool,
    ) -> String {
        let n = series[0].len();
        let cats = (0..n)
            .map(|i| format!(r#"<c:pt idx="{i}"><c:v>{}</c:v></c:pt>"#, (b'A' + i as u8) as char))
            .collect::<String>();
        let mut sers = String::new();
        for (si, vals) in series.iter().enumerate() {
            let pts = vals
                .iter()
                .enumerate()
                .filter_map(|(i, v)| {
                    v.map(|x| format!(r#"<c:pt idx="{i}"><c:v>{x}</c:v></c:pt>"#))
                })
                .collect::<String>();
            let present = vals.iter().filter(|v| v.is_some()).count();
            let marker = if kind == "lineChart" && marker_none {
                r#"<c:marker><c:symbol val="none"/></c:marker>"#
            } else {
                ""
            };
            let smooth = if kind == "lineChart" {
                r#"<c:smooth val="0"/>"#
            } else {
                ""
            };
            sers.push_str(&format!(
                r#"<c:ser><c:idx val="{si}"/><c:order val="{si}"/><c:tx><c:v>S{si}</c:v></c:tx>{marker}{smooth}<c:cat><c:strLit><c:ptCount val="{n}"/>{cats}</c:strLit></c:cat><c:val><c:numLit><c:ptCount val="{present}"/>{pts}</c:numLit></c:val></c:ser>"#
            ));
        }
        let (major, numfmt) = if grouping == "percentStacked" {
            ("0.2", r#"<c:numFmt formatCode="0%" sourceLinked="1"/>"#)
        } else {
            ("20", "")
        };
        let scaling = if auto {
            r#"<c:scaling><c:orientation val="minMax"/></c:scaling>"#.to_string()
        } else {
            format!(
                r#"<c:scaling><c:orientation val="minMax"/><c:min val="{min}"/><c:max val="{max}"/></c:scaling>"#
            )
        };
        let cross = if midcat { "midCat" } else { "between" };
        format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart">
  <c:chart>
    <c:plotArea>
      <c:{kind}>
        <c:grouping val="{grouping}"/><c:varyColors val="0"/>
        {sers}
        <c:axId val="1"/><c:axId val="2"/>
      </c:{kind}>
      <c:catAx><c:axId val="1"/><c:scaling><c:orientation val="minMax"/></c:scaling><c:delete val="0"/><c:axPos val="b"/><c:crossAx val="2"/><c:crosses val="autoZero"/></c:catAx>
      <c:valAx><c:axId val="2"/>{scaling}<c:majorUnit val="{major}"/><c:delete val="0"/><c:axPos val="l"/><c:majorGridlines/>{numfmt}<c:crossAx val="1"/><c:crosses val="autoZero"/><c:crossBetween val="{cross}"/></c:valAx>
    </c:plotArea>
    <c:legend><c:legendPos val="r"/><c:overlay val="0"/></c:legend>
  </c:chart>
</c:chartSpace>"#
        )
    }

    fn accent_pixels(rgb: &[u8], w: u32, h: u32, accent: [u8; 3]) -> Vec<(u32, u32)> {
        let mut out = Vec::new();
        for y in 0..h {
            for x in 0..w {
                let i = (y as usize * w as usize + x as usize) * 3;
                if near([rgb[i], rgb[i + 1], rgb[i + 2]], accent, 20) {
                    out.push((x, y));
                }
            }
        }
        out
    }

    fn polylines_match(a: &[(u32, u32)], b: &[(u32, u32)]) -> bool {
        if a.len() < 40 || b.len() < 40 {
            return false;
        }
        let close = |pts: &[(u32, u32)], other: &[(u32, u32)]| {
            let mut hit = 0usize;
            for &(x, y) in pts {
                if other.iter().any(|&(ox, oy)| x.abs_diff(ox) <= 2 && y.abs_diff(oy) <= 2) {
                    hit += 1;
                }
            }
            hit * 100 / pts.len() >= 80
        };
        close(a, b) && close(b, a)
    }

    fn area_column_bands(rgb: &[u8], w: u32, h: u32, x: u32, accents: [[u8; 3]; 3]) -> Vec<(usize, u32, u32)> {
        let mut bands = Vec::new();
        let mut cur: Option<usize> = None;
        let mut y0 = 0u32;
        for y in 0..h {
            let i = (y as usize * w as usize + x as usize) * 3;
            let px = [rgb[i], rgb[i + 1], rgb[i + 2]];
            let hit = accents.iter().position(|a| near(px, *a, 24));
            if hit != cur {
                if let Some(idx) = cur {
                    bands.push((idx, y0, y.saturating_sub(1)));
                }
                cur = hit;
                y0 = y;
            }
        }
        if let Some(idx) = cur {
            bands.push((idx, y0, h.saturating_sub(1)));
        }
        bands
    }

    fn leftmost_area_column(rgb: &[u8], w: u32, h: u32, accents: [[u8; 3]; 3]) -> u32 {
        for x in 0..w {
            let mut seen = [false; 3];
            for y in 0..h {
                let i = (y as usize * w as usize + x as usize) * 3;
                let px = [rgb[i], rgb[i + 1], rgb[i + 2]];
                if let Some(idx) = accents.iter().position(|a| near(px, *a, 24)) {
                    seen[idx] = true;
                }
            }
            if seen.iter().filter(|v| **v).count() >= 2 {
                return x;
            }
        }
        panic!("no stacked area column with at least two series");
    }

    #[test]
    fn stacked_and_percent_stacked_lines_and_areas_use_algebraic_cumulative() {
        let s1 = [Some(10.0), Some(20.0), Some(30.0), Some(40.0)];
        let s2 = [Some(20.0), Some(10.0), Some(20.0), Some(10.0)];
        let s3 = [Some(20.0), Some(30.0), Some(10.0), Some(20.0)];
        let series: [&[Option<f64>]; 3] = [&s1, &s2, &s3];
        let a0 = FALLBACK_ACCENTS[0];
        let a1 = FALLBACK_ACCENTS[1];
        let a2 = FALLBACK_ACCENTS[2];

        let dummy = Series {
            values: s1.to_vec(),
            ..Series::default()
        };
        let dummy2 = Series {
            values: s2.to_vec(),
            ..Series::default()
        };
        let dummy3 = Series {
            values: s3.to_vec(),
            ..Series::default()
        };
        let group = [&dummy, &dummy2, &dummy3];
        assert_eq!(stack_cumulative(&group, 0, 0, false), Some((0.0, 10.0)));
        assert_eq!(stack_cumulative(&group, 0, 1, false), Some((10.0, 30.0)));
        assert_eq!(stack_cumulative(&group, 0, 2, false), Some((30.0, 50.0)));
        assert_eq!(stack_cumulative(&group, 3, 2, false), Some((50.0, 70.0)));
        let p0 = stack_cumulative(&group, 0, 0, true).unwrap();
        let p2 = stack_cumulative(&group, 0, 2, true).unwrap();
        assert!((p0.0 - 0.0).abs() < 1e-12 && (p0.1 - 0.2).abs() < 1e-12);
        assert!((p2.0 - 0.6).abs() < 1e-12 && (p2.1 - 1.0).abs() < 1e-12);

        let m1v = [Some(10.0), Some(20.0), Some(-30.0), Some(40.0)];
        let m2v = [Some(20.0), Some(-10.0), Some(20.0), Some(-10.0)];
        let m3v = [Some(-20.0), Some(30.0), Some(10.0), Some(20.0)];
        let mix1 = Series {
            values: m1v.to_vec(),
            ..Series::default()
        };
        let mix2 = Series {
            values: m2v.to_vec(),
            ..Series::default()
        };
        let mix3 = Series {
            values: m3v.to_vec(),
            ..Series::default()
        };
        let mixed_group = [&mix1, &mix2, &mix3];
        // Algebraic, not F22 pos/neg packing: cat A S3 ends at 10, not -20.
        assert_eq!(stack_cumulative(&mixed_group, 0, 2, false), Some((30.0, 10.0)));
        assert_eq!(stack_cumulative(&mixed_group, 2, 2, false), Some((-10.0, 0.0)));
        assert_eq!(stack_cumulative(&mixed_group, 3, 2, false), Some((30.0, 50.0)));

        let line_xml = line_area_stacked_xml("lineChart", "stacked", &series, false, "0", "80", false, true);
        let line_doc = crate::xml::parse(line_xml.as_bytes()).unwrap();
        let line = read_chart(&line_doc.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT).unwrap();
        assert_eq!(line.stacking, Stacking::Stacked);
        assert!(matches!(line.kind, PlotKind::StandardLine));
        let line_png = render_rgb_chart(
            &line_doc.root, 424, 265, FALLBACK_ACCENTS, None, ChartHost::Excel, None, "",
        )
        .unwrap();
        let (lw, lh, lrgb) = png_rgb(&line_png);

        let s2_end = [Some(30.0), Some(30.0), Some(50.0), Some(50.0)];
        let s3_end = [Some(50.0), Some(60.0), Some(60.0), Some(70.0)];
        let ends: [&[Option<f64>]; 3] = [&s1, &s2_end, &s3_end];
        let ctrl_xml = line_area_stacked_xml("lineChart", "standard", &ends, false, "0", "80", false, true);
        let ctrl_doc = crate::xml::parse(ctrl_xml.as_bytes()).unwrap();
        let ctrl = read_chart(&ctrl_doc.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT).unwrap();
        assert_eq!(ctrl.stacking, Stacking::Clustered);
        let ctrl_png = render_rgb_chart(
            &ctrl_doc.root, 424, 265, FALLBACK_ACCENTS, None, ChartHost::Excel, None, "",
        )
        .unwrap();
        let (_, _, crgb) = png_rgb(&ctrl_png);
        for (i, accent) in [a0, a1, a2].into_iter().enumerate() {
            let got = accent_pixels(&lrgb, lw, lh, accent);
            let exp = accent_pixels(&crgb, lw, lh, accent);
            assert!(
                polylines_match(&got, &exp),
                "stacked line S{i} should match a standard line of algebraic ends; stacked={} control={}",
                got.len(),
                exp.len()
            );
        }

        let pct_xml = line_area_stacked_xml(
            "lineChart",
            "percentStacked",
            &series,
            false,
            "0",
            "1",
            false,
            true,
        );
        let pct_doc = crate::xml::parse(pct_xml.as_bytes()).unwrap();
        let pct = read_chart(&pct_doc.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT).unwrap();
        assert_eq!(pct.stacking, Stacking::PercentStacked);
        assert_eq!(pct.axis.tick_format, super::axis::TickFormat::Percent);
        let pct_png = render_rgb_chart(
            &pct_doc.root, 424, 265, FALLBACK_ACCENTS, None, ChartHost::Excel, None, "",
        )
        .unwrap();
        let (pw, ph, prgb) = png_rgb(&pct_png);
        let p1 = [Some(0.2), Some(20.0 / 60.0), Some(0.5), Some(40.0 / 70.0)];
        let p2 = [Some(0.6), Some(0.5), Some(50.0 / 60.0), Some(50.0 / 70.0)];
        let p3 = [Some(1.0), Some(1.0), Some(1.0), Some(1.0)];
        let pct_ends: [&[Option<f64>]; 3] = [&p1, &p2, &p3];
        let pct_ctrl_xml = line_area_stacked_xml(
            "lineChart",
            "standard",
            &pct_ends,
            false,
            "0",
            "1",
            false,
            true,
        );
        let pct_ctrl_png = render_rgb_chart(
            &crate::xml::parse(pct_ctrl_xml.as_bytes()).unwrap().root,
            424,
            265,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .unwrap();
        let (_, _, pcrgb) = png_rgb(&pct_ctrl_png);
        let got3 = accent_pixels(&prgb, pw, ph, a2);
        let exp3 = accent_pixels(&pcrgb, pw, ph, a2);
        assert!(
            polylines_match(&got3, &exp3),
            "percentStacked line S3 should sit on the 100% cumulative, stacked={} control={}",
            got3.len(),
            exp3.len()
        );

        let area_xml = line_area_stacked_xml("areaChart", "stacked", &series, false, "0", "80", true, false);
        let area_doc = crate::xml::parse(area_xml.as_bytes()).unwrap();
        let area = read_chart(&area_doc.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT).unwrap();
        assert_eq!(area.stacking, Stacking::Stacked);
        assert!(matches!(area.kind, PlotKind::StandardArea));
        let area_png = render_rgb_chart(
            &area_doc.root, 424, 265, FALLBACK_ACCENTS, None, ChartHost::Excel, None, "",
        )
        .unwrap();
        let (aw, ah, argb) = png_rgb(&area_png);
        let x0 = leftmost_area_column(&argb, aw, ah, [a0, a1, a2]);
        let bands = area_column_bands(&argb, aw, ah, x0, [a0, a1, a2]);
        let stacked_bands: Vec<_> = bands.into_iter().filter(|(_idx, y0, y1)| *y1 > *y0 + 1).collect();
        assert!(
            stacked_bands.len() >= 3,
            "stacked area cat A should show S3/S2/S1 bands, got {stacked_bands:?} x={x0}"
        );
        assert_eq!(stacked_bands[0].0, 2, "top band is S3, {stacked_bands:?}");
        assert_eq!(stacked_bands[1].0, 1, "middle band is S2, {stacked_bands:?}");
        assert_eq!(stacked_bands[2].0, 0, "bottom band is S1, {stacked_bands:?}");
        let h1 = stacked_bands[2].2 - stacked_bands[2].1 + 1;
        let h2 = stacked_bands[1].2 - stacked_bands[1].1 + 1;
        let h3 = stacked_bands[0].2 - stacked_bands[0].1 + 1;
        assert!(
            (h2 as i32 - h3 as i32).abs() <= 3 && (h1 * 2 + 3) >= h2 && h1 + 3 >= h2 / 2,
            "area cat A heights should be ~10/20/20, got {h1},{h2},{h3}"
        );

        let area_pct_xml = line_area_stacked_xml(
            "areaChart",
            "percentStacked",
            &series,
            false,
            "0",
            "1",
            true,
            false,
        );
        let area_pct_doc = crate::xml::parse(area_pct_xml.as_bytes()).unwrap();
        let area_pct =
            read_chart(&area_pct_doc.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT).unwrap();
        assert_eq!(area_pct.stacking, Stacking::PercentStacked);
        let area_pct_png = render_rgb_chart(
            &area_pct_doc.root, 424, 265, FALLBACK_ACCENTS, None, ChartHost::Excel, None, "",
        )
        .unwrap();
        let (apw, aph, aprgb) = png_rgb(&area_pct_png);
        let apx = leftmost_area_column(&aprgb, apw, aph, [a0, a1, a2]);
        let ap_bands = area_column_bands(&aprgb, apw, aph, apx, [a0, a1, a2]);
        let ap_stack: Vec<_> = ap_bands.into_iter().filter(|(_, y0, y1)| *y1 > *y0 + 1).collect();
        assert!(ap_stack.len() >= 3, "percent area bands {ap_stack:?}");
        assert_eq!(ap_stack[0].0, 2);
        assert_eq!(ap_stack[ap_stack.len() - 1].0, 0);

        let mixed: [&[Option<f64>]; 3] = [&m1v, &m2v, &m3v];
        let mix_xml = line_area_stacked_xml("lineChart", "stacked", &mixed, false, "-40", "80", false, true);
        let mix_doc = crate::xml::parse(mix_xml.as_bytes()).unwrap();
        read_chart(&mix_doc.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT).unwrap();
        let mix_png = render_rgb_chart(
            &mix_doc.root, 424, 265, FALLBACK_ACCENTS, None, ChartHost::Excel, None, "",
        )
        .unwrap();
        let (mw, mh, mrgb) = png_rgb(&mix_png);
        let mix_s1 = [Some(10.0), Some(20.0), Some(-30.0), Some(40.0)];
        let mix_s2 = [Some(30.0), Some(10.0), Some(-10.0), Some(30.0)];
        let mix_s3 = [Some(10.0), Some(40.0), Some(0.0), Some(50.0)];
        let mix_ends: [&[Option<f64>]; 3] = [&mix_s1, &mix_s2, &mix_s3];
        let mix_ctrl_png = render_rgb_chart(
            &crate::xml::parse(
                line_area_stacked_xml("lineChart", "standard", &mix_ends, false, "-40", "80", false, true)
                    .as_bytes(),
            )
            .unwrap()
            .root,
            424,
            265,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .unwrap();
        let (_, _, mcrgb) = png_rgb(&mix_ctrl_png);
        let got_m3 = accent_pixels(&mrgb, mw, mh, a2);
        let exp_m3 = accent_pixels(&mcrgb, mw, mh, a2);
        assert!(
            polylines_match(&got_m3, &exp_m3),
            "mixed stacked line is algebraic (not F22 pos/neg pack); stacked={} control={}",
            got_m3.len(),
            exp_m3.len()
        );

        let mix_area_xml =
            line_area_stacked_xml("areaChart", "stacked", &mixed, false, "-40", "80", true, false);
        let mix_area_doc = crate::xml::parse(mix_area_xml.as_bytes()).unwrap();
        read_chart(
            &mix_area_doc.root,
            FALLBACK_ACCENTS,
            None,
            "",
            RenderDpi::DEFAULT,
        )
        .unwrap();
        let mix_area_png = render_rgb_chart(
            &mix_area_doc.root, 424, 265, FALLBACK_ACCENTS, None, ChartHost::Excel, None, "",
        )
        .unwrap();
        let (maw, mah, margb) = png_rgb(&mix_area_png);
        let xa = leftmost_area_column(&margb, maw, mah, [a0, a1, a2]);
        let mix_bands = area_column_bands(&margb, maw, mah, xa, [a0, a1, a2]);
        let mix_stack: Vec<_> = mix_bands.into_iter().filter(|(_, y0, y1)| *y1 > *y0 + 1).collect();
        assert!(
            mix_stack.iter().any(|(idx, _, _)| *idx == 0)
                && mix_stack.iter().any(|(idx, _, _)| *idx == 2),
            "mixed area cat A shows S1 and S3, got {mix_stack:?}"
        );
        assert!(
            !mix_stack.iter().any(|(idx, _, _)| *idx == 1),
            "mixed area cat A S3 covers the S2 band, got {mix_stack:?}"
        );

        let auto = crate::xml::parse(
            line_area_stacked_xml("lineChart", "stacked", &series, true, "0", "80", false, true)
                .as_bytes(),
        )
        .unwrap();
        assert!(
            read_chart(&auto.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT).is_err(),
            "omitted min/max on stacked line stays NI"
        );
        let between_area = crate::xml::parse(
            line_area_stacked_xml("areaChart", "stacked", &series, false, "0", "80", false, false)
                .as_bytes(),
        )
        .unwrap();
        assert!(
            read_chart(&between_area.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT).is_err(),
            "stacked area with crossBetween=between stays NI"
        );
        let marked = crate::xml::parse(
            line_area_stacked_xml("lineChart", "stacked", &series, false, "0", "80", false, false)
                .as_bytes(),
        )
        .unwrap();
        assert!(
            read_chart(&marked.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT).is_err(),
            "stacked line without official marker=none stays NI"
        );
        let blank_s2 = [Some(20.0), None, Some(20.0), Some(10.0)];
        let blank: [&[Option<f64>]; 3] = [&s1, &blank_s2, &s3];
        let blank_doc = crate::xml::parse(
            line_area_stacked_xml("lineChart", "stacked", &blank, false, "0", "80", false, true)
                .as_bytes(),
        )
        .unwrap();
        assert!(
            read_chart(&blank_doc.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT).is_err(),
            "stacked line with a missing cache point stays NI"
        );
        let pct_mixed = crate::xml::parse(
            line_area_stacked_xml("lineChart", "percentStacked", &mixed, false, "0", "1", false, true)
                .as_bytes(),
        )
        .unwrap();
        assert!(
            read_chart(&pct_mixed.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT).is_err(),
            "percentStacked mixed signs on a line stay NI"
        );
    }
}
