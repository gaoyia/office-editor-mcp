//! 簇状柱形图只读栅格化。读图部件里已有的缓存，不写回包，也不算公式。
//! 像素差要改这里的布局、颜色和字形，不要靠放宽对照容差。

use fontdue::{Font, FontSettings};

use crate::error::{Error, ErrorKind};
use crate::xml::Element;

const DPI: f64 = 96.0;
const EMU_PER_INCH: f64 = 914_400.0;
const MAX_PX: u32 = 4096;
/// 轴、图例用 10 磅。Excel 16.0 画出来的数字比 9 磅更宽。
const LABEL_PT: f64 = 10.0;
/// 标题用 14 磅。
const TITLE_PT: f64 = 14.0;

const INK: [u8; 3] = [0, 0, 0];
const BORDER: [u8; 3] = [0x89, 0x89, 0x89];
/// 图例在右、没有标题时，576×336 和 384×240 两张图的轴边距相同。
const INSET_LEFT: i32 = 37;
const INSET_TOP: i32 = 14;
const INSET_RIGHT: i32 = 60;
const INSET_BOTTOM: i32 = 34;
const TICK_OUT: i32 = 3;

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

/// 图形 `a:ext` 的 EMU 按 96 dpi 四舍五入。超出 1..=4096 则不能画。
pub(crate) fn emu_to_px(emu: i64) -> Result<u32, Error> {
    if emu <= 0 {
        return Err(Error::with_path(ErrorKind::ChartImageUnsupported, "id"));
    }
    let px = (emu as f64) * DPI / EMU_PER_INCH;
    let rounded = px.round();
    if !(1.0..=f64::from(MAX_PX)).contains(&rounded) {
        return Err(Error::with_path(ErrorKind::ChartImageUnsupported, "id"));
    }
    Ok(rounded as u32)
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
    let gdi = super::chart_gdi::Face::new(label_px);
    let title_px = (TITLE_PT * DPI / 72.0) as f32;
    let (axis_min, axis_max, step) = axis_range(&chart.series);
    let ticks = tick_values(axis_min, axis_max, step);
    let tick_labels: Vec<String> = ticks.iter().copied().map(fmt_tick).collect();
    let intervals = ticks.len().saturating_sub(1).max(1) as i32;
    let plot_x = INSET_LEFT + 1;
    let plot_y = INSET_TOP
        + if chart.title.is_some() {
            text_height(font, title_px).ceil() as i32
        } else {
            0
        };
    let plot_r = canvas.w as i32 - INSET_RIGHT;
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
            canvas.hline_i(INSET_LEFT - TICK_OUT, plot_r, y, INK);
        }
    }
    draw_bars(canvas, chart, &plot, axis_min, axis_max);
    canvas.vline_i(INSET_LEFT, plot_y, plot_b, INK);
    draw_tick_labels(
        canvas,
        font,
        gdi.as_ref(),
        label_px,
        &plot,
        &ticks,
        &tick_labels,
        axis_min,
        axis_max,
        &chart.categories,
    );
    if let Some(title) = &chart.title {
        let width = text_width(font, gdi.as_ref(), title, title_px);
        let x = (f64::from(canvas.w) - width) / 2.0;
        draw_text(canvas, font, gdi.as_ref(), title, x, 4.0, title_px, INK);
    }
    draw_legend(canvas, chart, font, gdi.as_ref(), label_px, plot_r);
    canvas.stroke_border(BORDER);
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

fn draw_tick_labels(
    canvas: &mut Canvas,
    font: Option<&Font>,
    gdi: Option<&super::chart_gdi::Face>,
    px: f32,
    plot: &Rect,
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
            f64::from(INSET_LEFT) - w - 11.0,
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
    let swatch = 8;
    let pitch = 24;
    let n = chart.series.len() as i32;
    if n == 0 {
        return;
    }
    let block = pitch * (n - 1) + swatch;
    let mut y = (canvas.h as i32 - block) / 2;
    let x = plot_r + 22;
    for ser in &chart.series {
        let color = ser.color.unwrap_or(chart.accents[ser.slot % 6]);
        canvas.fill_rect(x, y, x + swatch, y + swatch, color);
        draw_text(
            canvas,
            font,
            gdi,
            &ser.name,
            f64::from(x + swatch + 6),
            f64::from(y - 2),
            px,
            INK,
        );
        y += pitch;
    }
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
            let (axis_min, axis_max) = axis_ends(min_v, max_v, lo, hi, step, pin_min, pin_max);
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
                    width < old_w - 1e-9 || ((width - old_w).abs() < 1e-9 && step > old_step)
                }
            };
            if replace {
                best = Some((width, step, axis_max));
            }
        }
    }
    if let Some((_, step, _)) = best {
        let (axis_min, axis_max) = axis_ends(min_v, max_v, lo, hi, step, pin_min, pin_max);
        return (axis_min, axis_max, step);
    }
    (0.0, 1.0, 0.2)
}

fn axis_ends(
    min_v: f64,
    max_v: f64,
    lo: f64,
    hi: f64,
    step: f64,
    pin_min: bool,
    pin_max: bool,
) -> (f64, f64) {
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
    if !pin_min && axis_min >= min_v - step * 1e-8 {
        axis_min -= step;
    }
    if !pin_max && axis_max <= max_v + step * 1e-8 {
        axis_max += step;
    }
    (axis_min, axis_max)
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
        if w == 0 || h == 0 {
            return;
        }
        self.hline_i(0, w - 1, 0, color);
        self.hline_i(0, w - 1, h - 1, color);
        self.vline_i(0, 0, h - 1, color);
        self.vline_i(w - 1, 0, h - 1, color);
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
