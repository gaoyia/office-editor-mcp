//! 簇状柱形图只读栅格化。读图部件里已有的缓存，不写回包，也不算公式。
//! 像素差要改这里的布局、颜色和字形，不要靠放宽对照容差。

use fontdue::{Font, FontSettings};

use crate::error::{Error, ErrorKind};
use crate::xml::Element;

const DPI: f64 = 96.0;
const EMU_PER_INCH: f64 = 914_400.0;
const MAX_PX: u32 = 4096;
/// 图里没写字号时，标题、轴、类目、图例都是 10 磅。标题另用粗体。
const LABEL_PT: f64 = 10.0;

const INK: [u8; 3] = [0, 0, 0];
const BORDER: [u8; 3] = [0x89, 0x89, 0x89];
/// 图例在右、没有标题时，576×336 和 384×240 两张图的轴边距相同。
const INSET_LEFT: i32 = 37;
const INSET_TOP: i32 = 14;
const INSET_RIGHT: i32 = 60;
const INSET_BOTTOM: i32 = 34;
const TICK_OUT: i32 = 3;
/// 刻度字到左缘、刻度字到轴线。短标签仍停在 `INSET_LEFT`，不挪已对过的轴。
const LABEL_EDGE: i32 = 6;
const TICK_GAP: i32 = 11;
/// 色块、色块到字、绘图区到色块、字到右缘。短标签用不满 `INSET_RIGHT`。
const LEGEND_SWATCH: i32 = 8;
const LEGEND_GAP: i32 = 6;
const LEGEND_PLOT_GAP: i32 = 22;
const LEGEND_PAD: i32 = 4;
/// Excel 16.0 图表外框圆角。576、384、647 三档都是 14 像素。
const CORNER_R: i32 = 14;
const CAT_MAJOR: i32 = 4;
const CAT_MAJOR_IN: i32 = 3;
const CAT_MINOR: i32 = 3;
const CAT_MINOR_IN: i32 = 2;
/// 数值轴一个主刻度分成 5 段，中间 4 道次刻度。实心部分向两侧各 2 像素。
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
}

struct Series {
    name: String,
    values: Vec<Option<f64>>,
    color: Option<[u8; 3]>,
    slot: usize,
}

struct ColumnChart {
    categories: Vec<String>,
    series: Vec<Series>,
    gap_width: f64,
    overlap: f64,
    vary_points: bool,
    grid: bool,
    legend: Option<LegendPos>,
    title: Option<String>,
    accents: [[u8; 3]; 6],
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
}

/// 图表 PNG 的像素。`twoCell` 按格子累加，其它锚点用 `a:ext`。缩放来自工作表。
pub(crate) fn bitmap_px(
    sheet: &Element,
    anchor: &Element,
    kind: super::picture::AnchorKind,
) -> Result<(u32, u32), Error> {
    let zoom = sheet_zoom(sheet);
    let (w, h) = match kind {
        super::picture::AnchorKind::TwoCell => two_cell_px(sheet, anchor, zoom)?,
        _ => ext_px(anchor, zoom)?,
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

fn emu_px(emu: i64) -> i64 {
    (emu as f64 * DPI / EMU_PER_INCH).round() as i64
}

fn pt_to_px(pt: f64) -> i64 {
    let emu = (pt * 12_700.0).round();
    (emu / 9_525.0).round() as i64
}

fn row_px(sheet: &Element, row: u32) -> i64 {
    if let Some(sheet_data) = sheet.child_by_local("sheetData") {
        for el in sheet_data.children_by_local("row") {
            let Some(r) = el.attr("r").and_then(|s| s.parse::<u32>().ok()) else {
                continue;
            };
            if r != row {
                continue;
            }
            if let Some(ht) = el.attr("ht").and_then(|s| s.parse::<f64>().ok()) {
                return pt_to_px(ht).max(1);
            }
        }
    }
    let def = sheet
        .child_by_local("sheetFormatPr")
        .and_then(|el| el.attr_local("defaultRowHeight"))
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(14.25);
    pt_to_px(def).max(1)
}

fn marker_x(sheet: &Element, col: u32, off: i64, zoom: u32) -> i64 {
    let mut px = 0i64;
    for c in 1..col {
        px += zoomed(i64::from(super::layout::col_width_px(sheet, c)), zoom);
    }
    px + zoomed(emu_px(off), zoom)
}

fn marker_y(sheet: &Element, row: u32, off: i64, zoom: u32) -> i64 {
    let mut px = 0i64;
    for r in 1..row {
        px += zoomed(row_px(sheet, r), zoom);
    }
    px + zoomed(emu_px(off), zoom)
}

fn two_cell_px(sheet: &Element, anchor: &Element, zoom: u32) -> Result<(i64, i64), Error> {
    let (from, from_x, from_y) = super::picture::read_marker(anchor.child_by_local("from"))?;
    let (to, to_x, to_y) = super::picture::read_marker(anchor.child_by_local("to"))?;
    let (Some(from), Some(to)) = (from, to) else {
        return ext_px(anchor, zoom);
    };
    let w = marker_x(sheet, to.col, to_x, zoom) - marker_x(sheet, from.col, from_x, zoom);
    let h = marker_y(sheet, to.row, to_y, zoom) - marker_y(sheet, from.row, from_y, zoom);
    Ok((w, h))
}

fn ext_px(anchor: &Element, zoom: u32) -> Result<(i64, i64), Error> {
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
    Ok((zoomed(emu_px(cx), zoom), zoomed(emu_px(cy), zoom)))
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

pub(crate) fn render_chart(
    root: &Element,
    width: u32,
    height: u32,
    accents: [[u8; 3]; 6],
    font_bytes: Option<&[u8]>,
) -> Result<Vec<u8>, Error> {
    if width == 0 || height == 0 || width > MAX_PX || height > MAX_PX {
        return Err(Error::with_path(ErrorKind::ChartImageUnsupported, "id"));
    }
    let chart = read_clustered_column(root, accents)?;
    let font = font_bytes.and_then(|bytes| Font::from_bytes(bytes, FontSettings::default()).ok());
    let mut canvas = Canvas::new(width, height);
    paint(&mut canvas, &chart, font.as_ref());
    encode_png(width, height, &canvas.px)
}

fn read_clustered_column(root: &Element, accents: [[u8; 3]; 6]) -> Result<ColumnChart, Error> {
    let chart = root
        .child_by_local("chart")
        .ok_or_else(|| Error::with_path(ErrorKind::ChartImageUnsupported, "id"))?;
    let plot = chart
        .child_by_local("plotArea")
        .ok_or_else(|| Error::with_path(ErrorKind::ChartImageUnsupported, "id"))?;
    let mut bar = None;
    for child in plot.element_children() {
        if is_chart_wrapper(child.local_name()) {
            if bar.is_some() || child.local_name() != "barChart" {
                return Err(Error::with_path(ErrorKind::ChartImageUnsupported, "id"));
            }
            bar = Some(child);
        }
    }
    let bar = bar.ok_or_else(|| Error::with_path(ErrorKind::ChartImageUnsupported, "id"))?;
    let dir = bar
        .child_by_local("barDir")
        .and_then(|el| el.attr_local("val"))
        .unwrap_or("bar");
    let grouping = bar
        .child_by_local("grouping")
        .and_then(|el| el.attr_local("val"))
        .unwrap_or("clustered");
    if dir != "col" || grouping != "clustered" {
        return Err(Error::with_path(ErrorKind::ChartImageUnsupported, "id"));
    }
    let vary = flag(bar, "varyColors").unwrap_or(false);
    let gap_width = number_val(bar, "gapWidth").unwrap_or(150.0);
    let overlap = number_val(bar, "overlap").unwrap_or(0.0);
    let mut series = Vec::new();
    for ser in bar.element_children().filter(|el| el.local_name() == "ser") {
        let order = number_val(ser, "order").unwrap_or(series.len() as f64) as usize;
        let values = cache_numbers(ser.child_by_local("val"));
        if values.is_empty() {
            continue;
        }
        let name = series_name(ser).unwrap_or_else(|| format!("系列{}", order + 1));
        let color = series_color(ser, accents);
        if series.len() <= order {
            series.resize_with(order + 1, || Series {
                name: String::new(),
                values: Vec::new(),
                color: None,
                slot: 0,
            });
        }
        series[order] = Series {
            name,
            values,
            color,
            slot: order,
        };
    }
    series.retain(|ser| !ser.values.is_empty());
    if series.is_empty() {
        return Err(Error::with_path(ErrorKind::ChartImageUnsupported, "id"));
    }
    let n = series.iter().map(|ser| ser.values.len()).max().unwrap_or(0);
    if n == 0 {
        return Err(Error::with_path(ErrorKind::ChartImageUnsupported, "id"));
    }
    let mut categories = cache_labels(series_cat(bar));
    if categories.len() < n {
        let start = categories.len();
        categories.extend((start..n).map(|i| (i + 1).to_string()));
    }
    categories.truncate(n);
    for ser in &mut series {
        ser.values.resize(n, None);
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
        return Err(Error::with_path(ErrorKind::ChartImageUnsupported, "id"));
    }
    let vary_points = vary && series.len() == 1;
    Ok(ColumnChart {
        categories,
        series,
        gap_width,
        overlap,
        vary_points,
        grid,
        legend: legend_pos(chart),
        title: title_text(chart),
        accents,
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

fn paint(canvas: &mut Canvas, chart: &ColumnChart, font: Option<&Font>) {
    let label_px = (LABEL_PT * DPI / 72.0) as f32;
    let gdi = super::chart_gdi::Face::open(label_px, false);
    let title_face = chart
        .title
        .as_ref()
        .and_then(|_| super::chart_gdi::Face::open(label_px, true));
    let (axis_min, axis_max, step) = axis_range(&chart.series);
    let ticks = tick_values(axis_min, axis_max, step);
    let tick_labels: Vec<String> = ticks.iter().copied().map(fmt_tick).collect();
    let intervals = ticks.len().saturating_sub(1).max(1) as i32;
    let widest_tick = tick_labels
        .iter()
        .map(|label| text_width(font, gdi.as_ref(), label, label_px).ceil() as i32)
        .max()
        .unwrap_or(0);
    let axis_x = axis_x_of(widest_tick);
    let plot_x = axis_x + 1;
    let title_top = 4.0;
    let plot_y = if chart.title.is_some() {
        let band = title_face
            .as_ref()
            .map(|face| face.em_box().ceil() as i32)
            .unwrap_or_else(|| text_height(font, label_px).ceil() as i32);
        let half = gdi
            .as_ref()
            .map(|face| face.line_height().ceil() as i32 / 2)
            .unwrap_or(0);
        // 标题占一行，下面再空一行，顶上的刻度才不压住标题。
        title_top as i32 + 2 * (band + half)
    } else {
        INSET_TOP
    };
    let inset_right = legend_right_inset(chart, font, gdi.as_ref(), label_px);
    let plot_r = canvas.w as i32 - inset_right;
    let plot_b = canvas.h as i32 - INSET_BOTTOM;
    let plot = Rect {
        x: f64::from(plot_x),
        y: f64::from(plot_y),
        w: f64::from(plot_r - plot_x),
        h: f64::from(plot_b - plot_y),
    };
    if chart.grid {
        for i in 0..=intervals {
            let y = plot_y + (i * (plot_b - plot_y) + intervals - 1) / intervals;
            canvas.hline_i(axis_x - TICK_OUT, plot_r, y, INK);
        }
    }
    draw_value_minor_ticks(canvas, axis_x, plot_y, plot_b, intervals);
    draw_cat_ticks(
        canvas,
        chart.categories.len(),
        f64::from(axis_x),
        f64::from(plot_r),
        plot_b,
    );
    draw_bars(canvas, chart, &plot, axis_min, axis_max);
    canvas.vline_i(axis_x, plot_y, plot_b, INK);
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
    );
    if let Some(title) = &chart.title {
        let face = title_face.as_ref().or(gdi.as_ref());
        let width = text_width(font, face, title, label_px);
        let x = (f64::from(canvas.w) - width) / 2.0;
        draw_text(canvas, font, face, title, x, title_top, label_px, INK);
    }
    draw_legend(canvas, chart, font, gdi.as_ref(), label_px, plot_r);
    canvas.stroke_border(BORDER);
}

fn draw_cat_ticks(canvas: &mut Canvas, n: usize, x0: f64, x1: f64, axis_y: i32) {
    let n = n.max(1);
    let slot = (x1 - x0) / n as f64;
    if slot <= 0.0 {
        return;
    }
    for i in 0..=n {
        let x = (x0 + slot * i as f64).round() as i32;
        canvas.vline_i(x, axis_y - CAT_MAJOR_IN, axis_y + CAT_MAJOR, INK);
    }
    for i in 0..n {
        let x = (x0 + slot * (i as f64 + 0.5)).round() as i32;
        canvas.vline_i(x, axis_y - CAT_MINOR_IN, axis_y + CAT_MINOR, INK);
    }
}

fn draw_value_minor_ticks(
    canvas: &mut Canvas,
    axis_x: i32,
    plot_y: i32,
    plot_b: i32,
    intervals: i32,
) {
    let span = plot_b - plot_y;
    if intervals < 1 || span <= 0 {
        return;
    }
    for i in 0..intervals {
        let y0 = plot_y + (i * span + intervals - 1) / intervals;
        let y1 = plot_y + ((i + 1) * span + intervals - 1) / intervals;
        let gap = y1 - y0;
        for k in 1..VAL_MINOR_PARTS {
            let y = y0 + (k * gap + VAL_MINOR_PARTS / 2) / VAL_MINOR_PARTS;
            if y <= y0 || y >= y1 {
                continue;
            }
            canvas.hline_i(axis_x - VAL_MINOR_ARM, axis_x + VAL_MINOR_ARM, y, INK);
        }
    }
}

fn draw_bars(canvas: &mut Canvas, chart: &ColumnChart, plot: &Rect, axis_min: f64, axis_max: f64) {
    let cats = chart.categories.len().max(1) as f64;
    let n = chart.series.len().max(1) as f64;
    let overlap = chart.overlap / 100.0;
    let gap = chart.gap_width / 100.0;
    let slot = plot.w / cats;
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
            let x0 = x.round() as i32;
            let x1 = (x + bar_w).round() as i32;
            let y0 = top.round() as i32;
            let y1 = (top + h).round() as i32;
            canvas.fill_rect(x0, y0, x1, y1, color);
        }
    }
}

fn axis_x_of(widest_tick: i32) -> i32 {
    INSET_LEFT.max(LABEL_EDGE + widest_tick + TICK_GAP)
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
            f64::from(axis_x) - w - f64::from(TICK_GAP),
            y - h / 2.0 + 4.0,
            px,
            INK,
        );
    }
    let cats = categories.len().max(1) as f64;
    let slot = plot.w / cats;
    let baseline = plot.y + plot.h + 13.0;
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
    let pitch = 24;
    let n = chart.series.len() as i32;
    if n == 0 {
        return;
    }
    let block = pitch * (n - 1) + LEGEND_SWATCH;
    let mut y = (canvas.h as i32 - block) / 2;
    let x = plot_r + LEGEND_PLOT_GAP;
    for ser in &chart.series {
        let color = ser.color.unwrap_or(chart.accents[ser.slot % 6]);
        canvas.fill_rect(x, y, x + LEGEND_SWATCH, y + LEGEND_SWATCH, color);
        draw_text(
            canvas,
            font,
            gdi,
            &ser.name,
            f64::from(x + LEGEND_SWATCH + LEGEND_GAP),
            f64::from(y - 2),
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
) -> i32 {
    if chart.legend != Some(LegendPos::Right) {
        return INSET_RIGHT;
    }
    let widest = chart
        .series
        .iter()
        .map(|ser| text_width(font, gdi, &ser.name, px))
        .fold(0.0, f64::max);
    let need = LEGEND_PLOT_GAP + LEGEND_SWATCH + LEGEND_GAP + widest.ceil() as i32 + LEGEND_PAD;
    need.max(INSET_RIGHT)
}

fn axis_range(series: &[Series]) -> (f64, f64, f64) {
    let mut min_v = 0.0_f64;
    let mut max_v = 0.0_f64;
    for ser in series {
        for value in ser.values.iter().flatten() {
            if !value.is_finite() {
                continue;
            }
            min_v = min_v.min(*value);
            max_v = max_v.max(*value);
        }
    }
    if max_v.abs() < 1e-12 && min_v.abs() < 1e-12 {
        return (0.0, 1.0, 0.2);
    }
    let pin_min = min_v >= 0.0;
    let pin_max = max_v <= 0.0;
    let lo = if pin_min { 0.0 } else { min_v };
    let hi = if pin_max { 0.0 } else { max_v };
    let span = (hi - lo).abs().max(1e-9);
    let exp = span.log10().floor() as i32;
    let mut best: Option<(f64, f64, f64)> = None;
    for mag in (exp - 2)..=(exp + 1) {
        for nice in [1.0_f64, 2.0, 5.0] {
            let step = nice * 10f64.powi(mag);
            let (axis_min, axis_max) = axis_ends(lo, hi, step, pin_min, pin_max);
            if axis_max <= axis_min {
                continue;
            }
            let intervals = ((axis_max - axis_min) / step).round() as i32;
            if !(5..=10).contains(&intervals) {
                continue;
            }
            let width = axis_max - axis_min;
            let replace = match best {
                None => true,
                Some((old_w, old_step, _)) => {
                    width < old_w - 1e-9 || ((width - old_w).abs() < 1e-9 && step < old_step)
                }
            };
            if replace {
                best = Some((width, step, axis_max));
            }
        }
    }
    if let Some((_, step, _)) = best {
        let (axis_min, axis_max) = axis_ends(lo, hi, step, pin_min, pin_max);
        return (axis_min, axis_max, step);
    }
    (0.0, 1.0, 0.2)
}

fn axis_ends(lo: f64, hi: f64, step: f64, pin_min: bool, pin_max: bool) -> (f64, f64) {
    let mut axis_min = if pin_min {
        0.0
    } else {
        (lo / step).floor() * step
    };
    let mut axis_max = if pin_max {
        0.0
    } else {
        (hi / step).ceil() * step
    };
    // 数据贴到自动上限的 95% 以上时，Excel 16.0 再加一个步长。正好 95% 不加。
    if !pin_max && past_headroom(hi, axis_max) {
        axis_max += step;
    }
    if !pin_min && past_headroom_low(lo, axis_min) {
        axis_min -= step;
    }
    (axis_min, axis_max)
}

fn past_headroom(value: f64, edge: f64) -> bool {
    edge > 1e-12 && value > edge * 0.95 + edge * 1e-9
}

fn past_headroom_low(value: f64, edge: f64) -> bool {
    edge < -1e-12 && value < edge * 0.95 - edge.abs() * 1e-9
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

fn series_name(ser: &Element) -> Option<String> {
    let tx = ser.child_by_local("tx")?;
    if let Some(v) = tx.child_by_local("v") {
        let text = v.text_content();
        if !text.is_empty() {
            return Some(text);
        }
    }
    let cache = find_deep(tx, "strCache").or_else(|| find_deep(tx, "numCache"))?;
    let text = cache_labels_of(cache).into_iter().next()?;
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
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

impl Canvas {
    fn new(w: u32, h: u32) -> Self {
        Self {
            w,
            h,
            px: vec![255; (w as usize) * (h as usize) * 3],
        }
    }

    fn blend(&mut self, x: i32, y: i32, rgb: [u8; 3], cov: u8) {
        if cov == 0 || x < 0 || y < 0 || x >= self.w as i32 || y >= self.h as i32 {
            return;
        }
        let i = (y as usize * self.w as usize + x as usize) * 3;
        if cov == 255 {
            self.px[i..i + 3].copy_from_slice(&rgb);
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
                if cov <= 0.0 {
                    continue;
                }
                self.blend(x, y, color, (cov * 255.0).round() as u8);
            }
        }
    }
}

fn encode_png(w: u32, h: u32, rgb: &[u8]) -> Result<Vec<u8>, Error> {
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, w, h);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
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

    #[test]
    fn long_legend_label_stays_off_the_right_edge() {
        let doc = crate::xml::parse(LONG_LEGEND.as_bytes()).unwrap();
        let png = render_chart(&doc.root, 576, 336, FALLBACK_ACCENTS, None).unwrap();
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
        let png = render_chart(&doc.root, 576, 336, FALLBACK_ACCENTS, None).unwrap();
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
        draw_value_minor_ticks(&mut canvas, 20, 0, 25, 1);
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

    fn one_series(max: f64) -> Series {
        Series {
            name: "S".to_string(),
            values: vec![Some(max)],
            color: None,
            slot: 0,
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
    fn wide_tick_pushes_axis_right() {
        assert_eq!(axis_x_of(18), 37);
        assert_eq!(axis_x_of(28), 45);
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
        )
        .unwrap();
        assert_eq!((w, h), (576, 336));
    }
}
