use super::axis;
use super::gradient;
use super::legend;
use super::stroke;

pub(super) const MAX_PX: u32 = 4096;

/// 图里没写字号时，标题、轴、类目、图例都是 10 磅。标题另用粗体。
pub(super) const LABEL_PT: f64 = 10.0;


pub(super) const INK: [u8; 3] = [0, 0, 0];

pub(super) const BORDER: [u8; 3] = [0x89, 0x89, 0x89];

/// 下列边距、刻度臂长和图例间距是 96 dpi 下的像素。出图时经 `RenderDpi::scale_px`，
/// 96 dpi 保持原值。圆角 `CORNER_R` 仍固定，圆角框另案处理。
/// 绘图区顶/底边距不用各自四舍五入：106 dpi 下 14→15.458、34→37.542，
/// 四舍五入得到 15+38，官方 Chart.Export 的网格顶/底却是 16 与 37（总边距同为 53）。
/// 起点边距向上取整、对边向下取整，96/144 等整倍数不变。
pub(super) const INSET_TOP: i32 = 14;

pub(super) const INSET_RIGHT: i32 = 60;

pub(super) const INSET_BOTTOM: i32 = 34;

pub(super) const TICK_OUT: i32 = 3;

/// 刻度字前进宽度的左缘。加上字宽和 `TICK_GAP` 就是轴线。
/// 缩放 100 的金样：字宽 7 → 27，字宽 17 → 37，字宽 28 → 48。
pub(super) const LABEL_EDGE: i32 = 9;

pub(super) const TICK_GAP: i32 = 11;

/// 色块、色块到字、绘图区到色块、字到右缘。短标签用不满 `INSET_RIGHT`。
pub(super) const LEGEND_SWATCH: i32 = 8;

pub(super) const LEGEND_GAP: i32 = 6;

pub(super) const LEGEND_PLOT_GAP: i32 = 22;

pub(super) const LEGEND_PAD: i32 = 4;

/// 有标题时，图例在这 30 像素标题带之下垂直居中。无标题的图不加这截。
pub(super) const LEGEND_TITLE_BAND: i32 = 30;

/// 标题墨迹比布局顶低 7 像素。Excel 的绘图区仍从布局顶往下排。
/// PowerPoint 幻灯片把这 7 像素算进绘图区，网格才和导出的页对齐。
pub(super) const TITLE_DROP: f64 = 7.0;

/// Excel 16.0 图表外框圆角。576、384、647 三档都是 14 像素。
pub(super) const CORNER_R: i32 = 14;

pub(super) const CAT_MAJOR: i32 = 4;

pub(super) const CAT_MAJOR_IN: i32 = 3;

pub(super) const CAT_MINOR: i32 = 3;

pub(super) const CAT_MINOR_IN: i32 = 2;

/// 数值轴一个主刻度分成 5 段，中间 4 道次刻度。实心部分向两侧各 2 像素。
#[cfg(test)]
pub(super) const VAL_MINOR_PARTS: i32 = 5;

pub(super) const VAL_MINOR_ARM: i32 = 2;


/// 包里没有 theme 时，Excel 16.0 用当前 Office 主题给自动系列上色。这套是 2023 的 accent。
pub(super) const FALLBACK_ACCENTS: [[u8; 3]; 6] = [
    [0x15, 0x60, 0x82],
    [0xE9, 0x71, 0x32],
    [0x19, 0x6B, 0x24],
    [0x0F, 0x9E, 0xD5],
    [0xA0, 0x2B, 0x93],
    [0x4E, 0xA7, 0x2E],
];


#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum LegendPos {
    Right,
    Left,
    Top,
    Bottom,
    TopRight,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum PlotKind {
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
pub(super) enum Stacking {
    Clustered,
    Stacked,
    PercentStacked,
}

/// Excel default auto-marker cycle when `c:marker` is omitted or `symbol=auto`.
/// Locked from Excel 16.0 Chart.Export of a dedicated 10-series Temp workbook
/// (omitted `c:marker`) plus Series.MarkerSize=9: S1 diamond … S9 dash, S10 wraps
/// to diamond. Official 2-series line golds show the same S1 diamond / S2 square.
pub(super) const DEFAULT_MARKER_CYCLE: [MarkerSymbol; 9] = [
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
pub(super) const OMITTED_MARKER_SIZE_PT: f64 = 9.0;

/// OOXML default for `c:size` when a marker element is present but the size is omitted.
pub(super) const EXPLICIT_MARKER_DEFAULT_SIZE_PT: f64 = 5.0;


#[derive(Clone, Copy, PartialEq)]
pub(super) enum MarkerSymbol {
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
pub(super) enum MarkerKind {
    None,
    Shaped { symbol: MarkerSymbol, size: f64 },
}

pub(super) struct LineSeriesStyle {
    pub(super) dash: Option<std::sync::Arc<stroke::DashPattern>>,
    pub(super) cap: stroke::Cap,
    pub(super) join: stroke::Join,
    pub(super) color: Option<[u8; 3]>,
    pub(super) visible: bool,
    pub(super) width: f64,
    pub(super) marker: MarkerKind,
    pub(super) marker_color: Option<[u8; 3]>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ValueLabel {
    OutsideEnd,
    InsideEnd,
    Right,
}

pub(super) struct Series {
    pub(super) name: String,
    pub(super) values: Vec<Option<f64>>,
    pub(super) color: Option<[u8; 3]>,
    pub(super) slot: usize,
    pub(super) line_visible: bool,
    pub(super) line_width: f64,
    pub(super) line_cap: stroke::Cap,
    pub(super) line_join: stroke::Join,
    pub(super) line_dash: Option<std::sync::Arc<stroke::DashPattern>>,
    pub(super) marker: MarkerKind,
    pub(super) marker_color: Option<[u8; 3]>,
    /// Area stroke color when a solid outline is present. Lines keep this unset.
    pub(super) line_color: Option<[u8; 3]>,
    pub(super) x_values: Vec<Option<f64>>,
    /// c:barSer defaults to inversion; dPt can override individual values.
    pub(super) invert_if_negative: bool,
    pub(super) point_inversion: Vec<Option<bool>>,
    pub(super) negative_outline: Option<LineSeriesStyle>,
    pub(super) value_label: Option<ValueLabel>,
    pub(super) deleted_label_points: Vec<usize>,
    /// Distinguish explicit errors from omitted points for dispBlanksAs.
    pub(super) error_points: Vec<bool>,
    pub(super) unavailable_points: Vec<bool>,
    pub(super) span_blanks: bool,
    /// Official F21 secondary valAx: this series is in the second same-type
    /// wrapper and is mapped with the secondary scale.
    pub(super) on_secondary: bool,
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

pub(super) struct ColumnChart {
    pub(super) categories: Vec<String>,
    pub(super) series: Vec<Series>,
    pub(super) gap_width: f64,
    pub(super) overlap: f64,
    pub(super) stacking: Stacking,
    pub(super) vary_points: bool,
    pub(super) grid: bool,
    pub(super) legend: Option<LegendPos>,
    pub(super) legend_spec: legend::Spec,
    pub(super) axis: axis::Spec,
    pub(super) axis_x: Option<axis::Spec>,
    /// Official F21 secondary valAx (axPos=r, crosses=max). None for one axis.
    pub(super) axis_secondary: Option<axis::Spec>,
    pub(super) grid_x: bool,
    /// 条形图类目轴：minMax 时第一类在下，maxMin 时第一类在上。系列 0 落在该类目轴方向的槽起点。柱形/折线不用。
    pub(super) cats_from_bottom: bool,
    pub(super) title: Option<String>,
    /// Explicit title typography (sz in points plus optional bold/italic/typeface).
    /// None means omitted defRPr/rPr, which stays on the legacy 10pt Face::open path
    /// because Excel Font.Size for that pattern is not a unique constant (10 vs 18).
    pub(super) title_typo: Option<TitleTypo>,
    pub(super) accents: [[u8; 3]; 6],
    pub(super) kind: PlotKind,
    /// Pie first slice angle in degrees, clockwise from up. Unused for other kinds.
    pub(super) first_slice_ang: f64,
    /// Pie per-point fill overrides (dPt). Empty for other kinds.
    pub(super) point_colors: Vec<Option<[u8; 3]>>,
    /// Official series-level pie fill. None when the series has no explicit fill.
    /// When present, it applies to every slice even if varyColors=1, and a dPt
    /// fill override still wins for that point.
    pub(super) series_fill: Option<SliceFill>,
    /// Pie per-point fill overrides, including noFill (alpha 0) and straight alpha.
    pub(super) point_fills: Vec<Option<SliceFill>>,
    /// Series-level pie explosion percent. Zero when omitted or explicitly 0.
    pub(super) pie_explosion: f64,
    /// Per-point pie explosion percent (dPt). Empty or all-zero when none.
    pub(super) point_explosions: Vec<f64>,
    /// Official Excel 16.0 area charts with omitted c:dispBlanksAs plot
    /// missing cells / omitted cache points as 0 (a V to the baseline).
    /// Explicit dispBlanksAs=gap still leaves a hole. Unused for other kinds.
    pub(super) area_missing_as_zero: bool,
}

/// Two integer-thousandth opacity operands retain their exact product until
/// final 8-bit coverage conversion. Rounding to an intermediate percentage
/// loses the distinction between 50000*33333 and 50001*33333 at the 42/43 edge.
pub(super) const ALPHA_PRODUCT_DENOM: u64 = 100_000 * 100_000;


/// Supported frame styles are semantic fills and a round/rectangle switch.
/// Pattern, blip, path/non-axis-aligned/multi-stop gradients, theme/system
/// colors, transforms and custom strokes remain NI.
/// Default (missing spPr) is white; explicit noFill is transparent; a solid
/// srgbClr with alpha other than 100000 is straight translucency, not a
/// flatten onto white.
/// `Linear` uses an opaque sRGB stop table at the existing 0/90 degree geometry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum FrameFill {
    Default,
    Solid([u8; 3]),
    /// Straight sRGB fill with DrawingML alpha in 0..=100000, excluding 100000
    /// (that case is `Solid`).
    Translucent([u8; 3], u32),
    /// Exact product of two percentage integers, denominator 100000^2.
    /// Only nonintegral-percent, nonopaque new alphaMod inputs use this path.
    AlphaProduct([u8; 3], u64),
    Linear(LinearFrameGrad),
    NoFill,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct LinearFrameGrad {
    pub(super) stops: gradient::StopTable,
    /// DrawingML `a:lin/@ang` in 60000ths of a degree. Only 0 (left to right)
    /// and 5_400_000 (top to bottom in y-down space) are locked.
    pub(super) ang: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum PlotPaint {
    Solid([u8; 3], u8),
    Linear(LinearFrameGrad),
}

pub(super) const LINEAR_ANG_HORIZONTAL: i64 = 0;

pub(super) const LINEAR_ANG_VERTICAL: i64 = 5_400_000;


pub(super) struct FrameStyle {
    pub(super) rounded: bool,
    pub(super) border: bool,
    pub(super) fill: FrameFill,
    pub(super) plot_fill: FrameFill,
}

pub(super) const OFFICE_GRAD_WEIGHT: [f64; 17] = [
    0.0, 0.0180, 0.0444, 0.0856, 0.1414, 0.2132, 0.2982, 0.3969, 0.5, 0.6031, 0.7018, 0.7868,
    0.8586, 0.9144, 0.9556, 0.9820, 1.0,
];


pub(super) const MAX_CUSTOM_DASH_PIECES: usize = 65_536; // Resource bound, not a visual period.


/// Locked only from explicit DrawingML `sz` (hundredths of a point). Omitted
/// typography is not filled in with a guessed default; Excel Font.Size on
/// empty defRPr / style 2 was 10pt on Chinese titles and 18pt on English ones.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct TitleTypo {
    pub(super) pt: f64,
    pub(super) bold: bool,
    pub(super) italic: bool,
    pub(super) name: Option<String>,
}

/// Official Excel 16 hidden-fill memory on a pie noFill. The remembered
/// solid is not painted; the visible fill is noFill.
pub(super) const PIE_HIDDEN_FILL_URI: &str = "{909E8E84-426E-40DD-AFC4-6F175D3DCCD1}";


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct SliceFill {
    pub(super) color: [u8; 3],
    pub(super) alpha: u8,
}

pub(super) struct AreaSeriesStyle {
    pub(super) fill: Option<[u8; 3]>,
    pub(super) line_visible: bool,
    pub(super) line_width: f64,
    pub(super) line_color: Option<[u8; 3]>,
}
