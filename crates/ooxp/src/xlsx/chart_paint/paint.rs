use fontdue::Font;

use crate::error::{Error, ErrorKind};
use crate::xml::Element;

use super::*;


/// Excel Chart.Export keeps a 96px gutter's leading edge by ceiling the
/// scaled length, and the trailing edge by flooring it. Independently
/// rounding both sides at 106dpi yields 15 and 38, but official untitled
/// 636x371 / 424x265 plots start at y=16 and end at 334 / 228 (insets 16
/// and 37). The total chrome is the same 53px. Exact at 96dpi.
pub(super) fn scale_plot_leading(dpi: RenderDpi, px96: i32) -> i32 {
    if dpi.is_default() {
        px96
    } else {
        (f64::from(px96) * dpi.get() / RenderDpi::DEFAULT.get()).ceil() as i32
    }
}

pub(super) fn scale_plot_trailing(dpi: RenderDpi, px96: i32) -> i32 {
    if dpi.is_default() {
        px96
    } else {
        (f64::from(px96) * dpi.get() / RenderDpi::DEFAULT.get()).floor() as i32
    }
}

/// Regular major-grid rows: truncate from the official plot top,
/// `y = plot_y + i * span / intervals`. The old `+ intervals - 1` bias
/// pushed several 106dpi ticks one pixel past unstretched official grids.
pub(super) fn regular_grid_y(plot_y: i32, plot_b: i32, i: i32, intervals: i32) -> i32 {
    plot_y + i * (plot_b - plot_y) / intervals
}

// Keep an on-major zero crossing on the very same raster row as its grid.
// Rounding y_of(0) separately puts mixed-sign axes one row below the 0 grid
// whenever the exact position is a half pixel (e.g. [-3,5], 106dpi).
pub(super) fn crossing_axis_y(plot: &Rect, min: f64, max: f64, major: f64, crossing: f64) -> i32 {
    crossing_axis_y_mode(plot, min, max, major, crossing, None, false)
}

pub(super) fn crossing_axis_y_mode(
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

pub(super) fn default_marker_symbol(slot: usize) -> MarkerSymbol {
    DEFAULT_MARKER_CYCLE[slot % DEFAULT_MARKER_CYCLE.len()]
}

pub(super) fn shaped_marker(symbol: MarkerSymbol, size_pt: f64, dpi: RenderDpi) -> MarkerKind {
    MarkerKind::Shaped {
        symbol,
        size: dpi.pt_to_px_font(size_pt),
    }
}

pub(super) fn marker_symbol_from_name(name: &str) -> Option<MarkerSymbol> {
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

pub(super) fn with_marker_size(marker: MarkerKind, size: f64) -> MarkerKind {
    match marker {
        MarkerKind::None => MarkerKind::None,
        MarkerKind::Shaped { symbol, .. } => MarkerKind::Shaped { symbol, size },
    }
}

pub(super) fn paint_scatter(
    canvas: &mut Canvas,
    chart: &ColumnChart,
    font: Option<&Font>,
    host: ChartHost,
) -> Result<(), Error> {
    let unsupported = || Error::with_path(ErrorKind::ChartImageUnsupported, "id");
    let legend_layout = legend::Layout::new(canvas, chart, font)?;
    let label_px = canvas.dpi.pt_to_px_font(LABEL_PT) as f32;
    let gdi = super::super::chart_gdi::Face::open(label_px, false);
    let title_px = title_face_px(chart.title_typo.as_ref(), canvas.dpi, label_px);
    let title_face = chart
        .title
        .as_ref()
        .and_then(|_| open_title_face(chart.title_typo.as_ref(), title_px, canvas.dpi));
    let title_lines = title_lines_for(
        chart.title.as_deref(),
        font,
        title_face.as_ref(),
        gdi.as_ref(),
        title_px,
        canvas.w,
        canvas.dpi,
    );
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
        titled_plot_y(
            canvas.dpi,
            host,
            title_face.as_ref(),
            gdi.as_ref(),
            font,
            label_px,
            title_lines.len(),
        )
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
        )?;
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
    if !title_lines.is_empty() {
        let face = title_face.as_ref().or(gdi.as_ref());
        draw_chart_title(canvas, font, face, &title_lines, title_px, title_top);
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

pub(super) fn x_of(value: f64, axis_min: f64, axis_max: f64, plot: &Rect) -> f64 {
    x_of_dir(value, axis_min, axis_max, plot, false)
}

pub(super) fn x_of_dir(value: f64, axis_min: f64, axis_max: f64, plot: &Rect, reversed: bool) -> f64 {
    let span = (axis_max - axis_min).max(1e-12);
    let mut t = (value - axis_min) / span;
    if reversed {
        t = 1.0 - t;
    }
    plot.x + t * plot.w
}

pub(super) fn draw_scatter(
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

pub(super) fn paint_bar(
    canvas: &mut Canvas,
    chart: &ColumnChart,
    font: Option<&Font>,
    host: ChartHost,
) -> Result<(), Error> {
    let legend_layout = legend::Layout::new(canvas, chart, font)?;
    let label_px = canvas.dpi.pt_to_px_font(LABEL_PT) as f32;
    let gdi = super::super::chart_gdi::Face::open(label_px, false);
    let title_px = title_face_px(chart.title_typo.as_ref(), canvas.dpi, label_px);
    let title_face = chart
        .title
        .as_ref()
        .and_then(|_| open_title_face(chart.title_typo.as_ref(), title_px, canvas.dpi));
    let title_lines = title_lines_for(
        chart.title.as_deref(),
        font,
        title_face.as_ref(),
        gdi.as_ref(),
        title_px,
        canvas.w,
        canvas.dpi,
    );
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
        let mut y = titled_plot_y(
            canvas.dpi,
            host,
            title_face.as_ref(),
            gdi.as_ref(),
            font,
            label_px,
            title_lines.len(),
        );
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
        )?;
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
    if !title_lines.is_empty() {
        let face = title_face.as_ref().or(gdi.as_ref());
        draw_chart_title(canvas, font, face, &title_lines, title_px, title_top);
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

pub(super) fn paint_pie(
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
    let gdi = super::super::chart_gdi::Face::open(label_px, false);
    let title_px = title_face_px(chart.title_typo.as_ref(), canvas.dpi, label_px);
    let title_face = chart
        .title
        .as_ref()
        .and_then(|_| open_title_face(chart.title_typo.as_ref(), title_px, canvas.dpi));
    let title_lines = title_lines_for(
        chart.title.as_deref(),
        font,
        title_face.as_ref(),
        gdi.as_ref(),
        title_px,
        canvas.w,
        canvas.dpi,
    );
    let title_top = canvas.dpi.scale_f(4.0);
    // Public Office PlotArea probes: the pie is not an inscribed circle of
    // the whole host. Title and right legend bands are removed first; Office
    // then contracts the shorter side by 7/8 and reserves 11 pt on each side.
    // These are point-space layout values, not export-pixel corrections.
    // ChartTitle TextFrame2 has 3.6 pt top/bottom margins; reserve the
    // additional 6 pt title/plot separation. Font fallback can still differ
    // from the host theme and remains a separately reported layout residual.
    let title_band = pie_title_band(
        canvas.dpi,
        title_face.as_ref(),
        font,
        label_px,
        title_lines.len(),
    );
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
    if let Some(fill) = canvas.plot_fill.clone() {
        canvas.paint_fractional_plot(&plot, fill)?;
    }
    draw_pie_slices(canvas, chart, &plot);
    if !title_lines.is_empty() {
        let face = title_face.as_ref().or(gdi.as_ref());
        draw_chart_title(canvas, font, face, &title_lines, title_px, title_top);
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
pub(super) fn pie_plot_bounds(
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

pub(super) fn pie_slice_fill(chart: &ColumnChart, index: usize) -> SliceFill {
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

pub(super) fn pie_legend_items(chart: &ColumnChart) -> Vec<(String, SliceFill)> {
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

pub(super) fn pie_legend_right_inset(
    chart: &ColumnChart,
    font: Option<&Font>,
    gdi: Option<&super::super::chart_gdi::Face>,
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
        + legend_filled_key_reserve_gap(dpi)
        + name_px
        + dpi.scale_px(LEGEND_PAD);
    need.max(dpi.scale_px(INSET_RIGHT))
}


/// Typographic top for a legend label that shares a bottom edge with the
/// filled key. `draw_text` takes typographic top; the GDI baseline is
/// `top + line_height()` (TEXTMETRIC ascent). Official 106 dpi Chart.Export
/// year ink for lining figures sits on that baseline, and the baseline
/// should sit on the last swatch pixel (`y + swatch_h - 1`). Not a unique
/// Office lock and not a typography rewrite: this only changes where the
/// already-drawn cell is placed. Fontdue fallback uses the same ascent rule.
pub(super) fn legend_filled_label_top(
    swatch_y: i32,
    swatch_h: i32,
    gdi: Option<&super::super::chart_gdi::Face>,
    font: Option<&Font>,
    px: f32,
) -> f64 {
    let baseline = f64::from(swatch_y + swatch_h - 1);
    let ascent = if let Some(face) = gdi {
        face.line_height()
    } else if let Some(font) = font {
        font.horizontal_line_metrics(px)
            .map(|item| f64::from(item.ascent))
            .unwrap_or(f64::from(px))
    } else {
        return f64::from(swatch_y);
    };
    baseline - ascent
}

pub(super) fn draw_pie_legend(
    canvas: &mut Canvas,
    chart: &ColumnChart,
    font: Option<&Font>,
    gdi: Option<&super::super::chart_gdi::Face>,
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
        let swatch_h = canvas.dpi.scale_px(LEGEND_SWATCH);
        draw_text(
            canvas,
            font,
            gdi,
            &name,
            f64::from(x + swatch_h + legend_filled_key_gap(canvas.dpi)),
            legend_filled_label_top(y, swatch_h, gdi, font, px),
            px,
            INK,
        );
        y += pitch;
    }
}

pub(super) fn draw_pie_slices(canvas: &mut Canvas, chart: &ColumnChart, plot: &Rect) {
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

pub(super) fn paint(
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
    let gdi = super::super::chart_gdi::Face::open(label_px, false);
    let title_px = title_face_px(chart.title_typo.as_ref(), canvas.dpi, label_px);
    let title_face = chart
        .title
        .as_ref()
        .and_then(|_| open_title_face(chart.title_typo.as_ref(), title_px, canvas.dpi));
    let title_lines = title_lines_for(
        chart.title.as_deref(),
        font,
        title_face.as_ref(),
        gdi.as_ref(),
        title_px,
        canvas.w,
        canvas.dpi,
    );
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
        )?;
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
    if !title_lines.is_empty() {
        let face = title_face.as_ref().or(gdi.as_ref());
        draw_chart_title(canvas, font, face, &title_lines, title_px, title_top);
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

pub(super) fn draw_cat_ticks(canvas: &mut Canvas, n: usize, x0: f64, x1: f64, axis_y: i32) {
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
pub(super) fn round_half_even(value: f64) -> i32 {
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

pub(super) fn draw_value_minor_ticks(
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

pub(super) fn draw_lines(
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

pub(super) fn line_point(
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

pub(super) fn stroke_run(canvas: &mut Canvas, run: &[(f64, f64)], width: f64, color: [u8; 3]) {
    if let Some(style) = stroke::Style::new(width, stroke::Cap::Round) {
        stroke_run_style(canvas, run, style, color);
    }
}
pub(super) fn stroke_run_pattern(canvas: &mut Canvas, points: &[(f64,f64)], style: stroke::Style,
    pattern: Option<&stroke::DashPattern>, color: [u8;3]) -> Result<(),Error> {
    stroke_run_pattern_join(canvas,points,style,pattern,stroke::Join::Round,color)
}
pub(super) fn stroke_run_pattern_join(canvas: &mut Canvas, points: &[(f64,f64)], style: stroke::Style,
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
    } else if join!=stroke::Join::Round {
        // One solid run, not a fabricated enormous dash. Gaps are split by the
        // caller, so both true ends get their cap and no dash budget is spent.
        let mut run=points.to_vec();
        stroke::simplify_collinear(&mut run);
        if run.len()<=2 {stroke_run_style(canvas,&run,style,color);}
        else {canvas.stroke_custom_dash_union_join(&run,style.width,style.cap,style.cap,join,color);}
    } else { stroke_run_style(canvas,points,style,color); }
    Ok(())
}

// Only the new custom-dash path uses independently resolved dash/run caps.
// The accepted solid rasterizer and default cap behavior stay byte-for-byte.
pub(super) fn stroke_run_dash_caps(canvas: &mut Canvas, points: &[(f64,f64)], width: f64,
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

pub(super) fn stroke_run_style(canvas: &mut Canvas, points: &[(f64, f64)], style: stroke::Style, color: [u8; 3]) {
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

pub(super) fn draw_areas(
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

pub(super) fn draw_stacked_areas(
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

pub(super) fn category_x_midcat(plot: &Rect, ci: usize, cats: f64) -> f64 {
    if cats <= 1.0 {
        plot.x + plot.w / 2.0
    } else {
        plot.x + plot.w * (ci as f64) / (cats - 1.0)
    }
}

pub(super) fn fill_area_run(canvas: &mut Canvas, run: &[(f64, f64)], zero_y: f64, color: [u8; 3]) {
    for pair in run.windows(2) {
        fill_area_span(canvas, pair[0], pair[1], zero_y, color);
    }
}

pub(super) fn fill_area_span(
    canvas: &mut Canvas,
    start: (f64, f64),
    end: (f64, f64),
    zero_y: f64,
    color: [u8; 3],
) {
    fill_area_band(canvas, start, (start.0, zero_y), end, (end.0, zero_y), color);
}

pub(super) fn fill_area_bands(
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
pub(super) fn fill_area_band(
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

pub(super) fn paint_marker(canvas: &mut Canvas, marker: MarkerKind, x: f64, y: f64, color: [u8; 3]) {
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
pub(super) fn legend_key_width(kind: PlotKind, square: i32, dpi: RenderDpi) -> i32 {
    if matches!(kind, PlotKind::StandardLine | PlotKind::Scatter { .. }) {
        dpi.pt_to_px_font(20.0).round() as i32
    } else {
        square
    }
}

/// Drawn swatch-to-series-name gap for filled square keys (column, bar, area,
/// pie). 96 dpi keeps the locked `LEGEND_GAP` (6). At other export DPIs,
/// `scale_px(LEGEND_GAP)` places official-export year labels too far from the
/// key. t200u authorized a close-enough move: draw with `LEGEND_GAP_SCALED_DPI`
/// (2 device px) so first ink sits near the measured official columns, while
/// plot reservation still uses `scale_px(LEGEND_GAP)` so the swatch does not
/// shift. Not a unique Office lock. Line/scatter keys keep 1.5pt.
pub(super) fn legend_filled_key_gap(dpi: RenderDpi) -> i32 {
    if dpi.is_default() {
        LEGEND_GAP
    } else {
        LEGEND_GAP_SCALED_DPI
    }
}

/// Plot-area reservation for a filled-key gap. Always the 96-dpi-scaled
/// LEGEND_GAP so 106 dpi swatches stay at the already-aligned left edge.
pub(super) fn legend_filled_key_reserve_gap(dpi: RenderDpi) -> i32 {
    dpi.scale_px(LEGEND_GAP)
}

pub(super) fn legend_key_gap(kind: PlotKind, dpi: RenderDpi) -> i32 {
    if matches!(kind, PlotKind::StandardLine | PlotKind::Scatter { .. }) {
        dpi.pt_to_px_font(1.5).round() as i32
    } else {
        legend_filled_key_gap(dpi)
    }
}

#[cfg(test)]
pub(super) fn draw_plot_key(
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

pub(super) fn draw_plot_key_styled(canvas: &mut Canvas, kind: PlotKind, x0:i32,y0:i32,x1:i32,y1:i32,color:[u8;3],marker:MarkerKind,style:Option<stroke::Style>,pattern:Option<&stroke::DashPattern>) -> Result<(),Error> {
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

pub(super) fn category_slot_origin(plot: &Rect, index: usize, cats: usize, from_bottom: bool) -> f64 {
    let cats = cats.max(1) as f64;
    let slot = (plot.h + 1.0) / cats;
    if from_bottom {
        plot.y + plot.h + 1.0 - slot * (index as f64 + 1.0)
    } else {
        plot.y + slot * index as f64
    }
}

pub(super) fn category_mid_y(plot: &Rect, index: usize, cats: usize, from_bottom: bool) -> f64 {
    let cats = cats.max(1) as f64;
    let slot = (plot.h + 1.0) / cats;
    category_slot_origin(plot, index, cats as usize, from_bottom) + slot / 2.0
}

/// Office inversion is a white negative fill with an automatic black 0.75 pt
/// outline, NOT a solid accent and NOT transparent. Public Point.Fill/Line
/// properties and toggled Series.InvertIfNegative exports lock this default.
/// The 0.75 pt hairline snaps to a 1 device-pixel solid stroke on the integer
/// edges; it is not a centered anti-aliased strip of that width.
pub(super) fn draw_bar_value(
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
pub(super) fn stack_segment(group: &[&Series], ci: usize, si: usize, percent: bool) -> Option<(f64, f64)> {
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
pub(super) fn stack_cumulative(
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

pub(super) fn stacked_edge_y(
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

pub(super) fn draw_clustered_bars(
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
pub(super) fn draw_horizontal_value_labels(
    canvas: &mut Canvas,
    chart: &ColumnChart,
    plot: &Rect,
    axis_min: f64,
    axis_max: f64,
    font: Option<&Font>,
    gdi: Option<&super::super::chart_gdi::Face>,
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

pub(super) fn draw_cat_ticks_y(
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
pub(super) fn draw_value_labels(
    canvas: &mut Canvas,
    chart: &ColumnChart,
    plot: &Rect,
    axis_min: f64,
    axis_max: f64,
    font: Option<&Font>,
    gdi: Option<&super::super::chart_gdi::Face>,
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

pub(super) fn column_bar_bounds(
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

pub(super) fn draw_bars(
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

pub(super) fn draw_stacked_columns(
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

pub(super) fn draw_stacked_horizontal_bars(
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

pub(super) fn axis_x_of(widest_tick: i32, dpi: RenderDpi) -> i32 {
    dpi.scale_px(LABEL_EDGE) + widest_tick + dpi.scale_px(TICK_GAP)
}

pub(super) fn draw_tick_labels(
    canvas: &mut Canvas,
    font: Option<&Font>,
    gdi: Option<&super::super::chart_gdi::Face>,
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

pub(super) fn draw_right_tick_labels(
    canvas: &mut Canvas,
    font: Option<&Font>,
    gdi: Option<&super::super::chart_gdi::Face>,
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

pub(super) fn draw_legend(
    canvas: &mut Canvas,
    chart: &ColumnChart,
    font: Option<&Font>,
    gdi: Option<&super::super::chart_gdi::Face>,
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
            legend_filled_label_top(y, canvas.dpi.scale_px(LEGEND_SWATCH), gdi, font, px),
            px,
            chart.legend_spec.entry_text_color(index),
        );
        y += pitch;
    }
    Ok(())
}

/// 短名字维持测过的 60 像素。长名字把绘图区右缘向左推，避免字被画布切掉。
pub(super) fn legend_right_inset(
    chart: &ColumnChart,
    font: Option<&Font>,
    gdi: Option<&super::super::chart_gdi::Face>,
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
    // Reservation keeps the 96-dpi-scaled filled-key gap so the plot edge
    // (and therefore the swatch) does not shift at 106 dpi. Drawn text uses
    // the tighter t200u `legend_key_gap` and sits closer to the swatch.
    let reserved_gap = if matches!(chart.kind, PlotKind::StandardLine | PlotKind::Scatter { .. }) {
        legend_key_gap(chart.kind, dpi)
    } else {
        legend_filled_key_reserve_gap(dpi)
    };
    let need = dpi.scale_px(LEGEND_PLOT_GAP)
        + key_width
        + reserved_gap
        + name_px
        + dpi.scale_px(LEGEND_PAD);
    let reference = gdi
        .map(|f| f.layout_width("S1"))
        .unwrap_or_else(|| text_width(font, None, "S1", px));
    let short_reduction = (reference - widest).max(0.0) as i32;
    need.max(dpi.scale_px(INSET_RIGHT) + key_width - square - short_reduction)
}

#[cfg(test)]
pub(super) fn axis_range(series: &[Series]) -> (f64, f64, f64) {
    let s = axis::scale(series, axis::Spec::default(), 10).unwrap();
    (s.min, s.max, s.major)
}

pub(super) fn tick_values(min: f64, max: f64, step: f64) -> Vec<f64> {
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

pub(super) fn y_of(value: f64, axis_min: f64, axis_max: f64, plot: &Rect) -> f64 {
    y_map(value, axis_min, axis_max, plot, None, false)
}

pub(super) fn y_map(
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

pub(super) fn tick_values_log(min: f64, max: f64, base: f64) -> Vec<f64> {
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
pub(super) fn fmt_tick(value: f64) -> String {
    fmt_tick_with(value, axis::TickFormat::General)
}

pub(super) fn fmt_tick_with(value: f64, format: axis::TickFormat) -> String {
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

pub(super) fn fmt_tick_general(value: f64) -> String {
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

pub(super) fn fmt_fixed_int(value: f64, percent: bool) -> String {
    let n = value.round();
    let n = if n == 0.0 { 0_i64 } else { n as i64 };
    if percent {
        format!("{n}%")
    } else {
        format!("{n}")
    }
}

pub(super) fn fmt_fixed(value: f64, places: u32, percent: bool) -> String {
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

pub(super) fn fmt_thousands(value: f64) -> String {
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

pub(super) fn fmt_scientific2(value: f64) -> String {
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

pub(super) fn legend_pos(chart: &Element) -> Option<LegendPos> {
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
