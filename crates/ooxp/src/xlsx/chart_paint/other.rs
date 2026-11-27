// Close-enough painters for official-like F25 chart types that Office can
// Chart.Export: doughnutChart, ofPieChart (pie of pie), radarChart
// (radarStyle=marker), bubbleChart (bubble3D=0), stockChart (HLC + hiLowLines),
// and surfaceChart viewed from above (wireframe=0, rotX=90).
//
// These are close-enough approximations, not unique official-matrix locks.
// Residual pixels stay on the review page. Do not auto-mark pass.

use fontdue::Font;

use crate::error::{Error, ErrorKind};

use super::*;

fn unsupported() -> Error {
    Error::with_path(ErrorKind::ChartImageUnsupported, "id")
}

fn x_map(value: f64, axis_min: f64, axis_max: f64, plot: &Rect, reversed: bool) -> f64 {
    let span = (axis_max - axis_min).max(1e-12);
    let mut t = (value - axis_min) / span;
    if reversed {
        t = 1.0 - t;
    }
    plot.x + t * plot.w
}

pub(super) fn paint_of_pie(
    canvas: &mut Canvas,
    chart: &ColumnChart,
    font: Option<&Font>,
    host: ChartHost,
) -> Result<(), Error> {
    let Some(spec) = chart.other.as_ref() else {
        return Err(unsupported());
    };
    // Reuse the pie frame (title, legend, plot fill) then draw two pies.
    // Layout is close-enough: main pie left, second pie right, gap from
    // gapWidth, second pie scaled by secondPieSize percent of the main.
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
    if let Some(fill) = canvas.plot_fill.clone() {
        canvas.paint_fractional_plot(&plot, fill)?;
    }
    draw_of_pie_slices(canvas, chart, spec, &plot);
    if !title_lines.is_empty() {
        let face = title_face.as_ref().or(gdi.as_ref());
        draw_chart_title(canvas, font, face, &title_lines, title_px, title_top);
    }
    if chart.legend == Some(LegendPos::Right) {
        let inset_right = pie_legend_right_inset(chart, font, gdi.as_ref(), label_px, canvas.dpi);
        let legend_plot_r = canvas.w as i32 - inset_right;
        draw_pie_legend(canvas, chart, font, gdi.as_ref(), label_px, legend_plot_r);
    }
    if matches!(host, ChartHost::Excel) {
        canvas.finish_excel_frame(Some(&plot));
    }
    Ok(())
}

fn draw_of_pie_slices(canvas: &mut Canvas, chart: &ColumnChart, spec: &OtherSpec, plot: &Rect) {
    let ser = &chart.series[0];
    let mut pts: Vec<(usize, f64)> = ser
        .values
        .iter()
        .enumerate()
        .filter_map(|(i, v)| {
            v.filter(|n| n.is_finite() && n.abs() > 0.0)
                .map(|n| (i, n.abs()))
        })
        .collect();
    if pts.is_empty() {
        return;
    }
    let n = pts.len();
    let split = spec.split_last.min(n.saturating_sub(1));
    let second: Vec<(usize, f64)> = if split == 0 {
        Vec::new()
    } else {
        pts.split_off(n - split)
    };
    let other_sum: f64 = second.iter().map(|p| p.1).sum();
    let mut main = pts;
    let _other_index = if other_sum > 0.0 {
        // "Other" is a composite slice; color is close-enough (next accent).
        Some(main.len())
    } else {
        None
    };
    if other_sum > 0.0 {
        main.push((usize::MAX, other_sum));
    }
    let main_sum: f64 = main.iter().map(|p| p.1).sum();
    if main_sum <= 0.0 {
        return;
    }
    // Close-enough two-pie layout: main on the left 58%, second on the right,
    // gap from gapWidth (100 ~= a sliver, 150 official).
    let gap_frac = (spec.of_pie_gap / 150.0 * 0.08).clamp(0.04, 0.16);
    let second_scale = (spec.second_pie_size / 100.0).clamp(0.3, 1.2);
    let work_w = plot.w;
    let main_d = (work_w * (0.50 - gap_frac / 2.0)).min(plot.h) * 0.95;
    let sec_d = main_d * second_scale;
    let main_cx = plot.x + main_d * 0.52;
    let main_cy = plot.y + plot.h * 0.52;
    let sec_cx = plot.x + plot.w - sec_d * 0.52;
    let sec_cy = main_cy;
    let main_r = main_d / 2.0;
    let sec_r = sec_d / 2.0;

    let mut start = chart.first_slice_ang.rem_euclid(360.0);
    let mut other_start = 0.0;
    let mut other_end = 0.0;
    for (k, (idx, value)) in main.iter().enumerate() {
        let end = start + 360.0 * (value / main_sum);
        let fill = if *idx == usize::MAX {
            other_start = start;
            other_end = end;
            SliceFill {
                color: chart.accents[(chart.categories.len().saturating_sub(1) + 1) % 6],
                alpha: 255,
            }
        } else {
            pie_slice_fill(chart, *idx)
        };
        if fill.alpha > 0 {
            canvas.fill_wedge(main_cx, main_cy, main_r, start, end, fill.color, fill.alpha);
        }
        let _ = k;
        start = end;
    }

    let sec_sum: f64 = second.iter().map(|p| p.1).sum();
    if sec_sum > 0.0 {
        let mut s0 = 0.0;
        for (idx, value) in &second {
            let end = s0 + 360.0 * (value / sec_sum);
            let fill = pie_slice_fill(chart, *idx);
            if fill.alpha > 0 {
                canvas.fill_wedge(sec_cx, sec_cy, sec_r, s0, end, fill.color, fill.alpha);
            }
            s0 = end;
        }
    }

    if spec.ser_lines && other_sum > 0.0 && sec_r > 0.0 {
        // Close-enough connectors from the Other slice chord to the second pie.
        let mid_a = ((other_start + other_end) / 2.0).to_radians();
        let a0 = other_start.to_radians();
        let a1 = other_end.to_radians();
        let p0 = (
            main_cx + a0.sin() * main_r,
            main_cy - a0.cos() * main_r,
        );
        let p1 = (
            main_cx + a1.sin() * main_r,
            main_cy - a1.cos() * main_r,
        );
        let left = (sec_cx - sec_r, sec_cy);
        let left_top = (sec_cx - sec_r * 0.3, sec_cy - sec_r * 0.85);
        let left_bot = (sec_cx - sec_r * 0.3, sec_cy + sec_r * 0.85);
        let _ = mid_a;
        canvas.stroke_segment(p0.0, p0.1, left_top.0, left_top.1, 1.0, [90, 90, 90]);
        canvas.stroke_segment(p1.0, p1.1, left_bot.0, left_bot.1, 1.0, [90, 90, 90]);
        let _ = left;
    }
}

pub(super) fn paint_radar(
    canvas: &mut Canvas,
    chart: &ColumnChart,
    font: Option<&Font>,
    host: ChartHost,
) -> Result<(), Error> {
    let (plot, font_bits) = cartesian_frame(canvas, chart, font, host)?;
    let (font, gdi, label_px, title_lines, title_px, title_top, title_face) = font_bits;
    let n_cat = chart
        .categories
        .len()
        .max(
            chart
                .series
                .iter()
                .map(|s| s.values.len())
                .max()
                .unwrap_or(0),
        )
        .max(3);
    let budget = axis::interval_budget_dpi(canvas.h, 12.0, canvas.dpi);
    let scale = axis::scale(&chart.series, chart.axis, budget)?;
    let cx = plot.x + plot.w / 2.0;
    let cy = plot.y + plot.h / 2.0;
    let r_max = plot.w.min(plot.h) * 0.38;
    let ticks = tick_values(scale.min, scale.max, scale.major);
    let grid = [137, 137, 137];
    for t in &ticks {
        let r = ((*t - scale.min) / (scale.max - scale.min).max(1e-12) * r_max).max(0.0);
        let mut ring = Vec::with_capacity(n_cat + 1);
        for i in 0..n_cat {
            ring.push(radar_point(cx, cy, r, i, n_cat));
        }
        if ring.len() >= 2 {
            for w in 0..n_cat {
                let a = ring[w];
                let b = ring[(w + 1) % n_cat];
                canvas.stroke_segment(a.0, a.1, b.0, b.1, 1.0, grid);
            }
        }
    }
    for i in 0..n_cat {
        let outer = radar_point(cx, cy, r_max, i, n_cat);
        canvas.stroke_segment(cx, cy, outer.0, outer.1, 1.0, grid);
        if let Some(name) = chart.categories.get(i) {
            let lab = radar_point(cx, cy, r_max + canvas.dpi.scale_f(14.0), i, n_cat);
            let w = text_width(font, gdi.as_ref(), name, label_px);
            draw_text(
                canvas,
                font,
                gdi.as_ref(),
                name,
                lab.0 - w / 2.0,
                lab.1 - text_height(font, label_px) / 2.0,
                label_px,
                [0, 0, 0],
            );
        }
    }
    // Value labels along the first (top) spoke.
    for t in &ticks {
        let r = ((*t - scale.min) / (scale.max - scale.min).max(1e-12) * r_max).max(0.0);
        let p = radar_point(cx, cy, r, 0, n_cat);
        let s = fmt_tick_with(*t, scale.tick_format);
        let w = text_width(font, gdi.as_ref(), &s, label_px);
        draw_text(
            canvas,
            font,
            gdi.as_ref(),
            &s,
            p.0 - w - canvas.dpi.scale_f(4.0),
            p.1 - text_height(font, label_px) / 2.0,
            label_px,
            [0, 0, 0],
        );
    }
    for ser in &chart.series {
        let color = ser.color.unwrap_or(chart.accents[ser.slot % 6]);
        let mut pts = Vec::new();
        for i in 0..n_cat {
            let v = ser.values.get(i).copied().flatten().unwrap_or(scale.min);
            let r = ((v - scale.min) / (scale.max - scale.min).max(1e-12) * r_max).max(0.0);
            pts.push(radar_point(cx, cy, r, i, n_cat));
        }
        if pts.len() >= 2 {
            for w in 0..pts.len() {
                let a = pts[w];
                let b = pts[(w + 1) % pts.len()];
                canvas.stroke_segment(a.0, a.1, b.0, b.1, 1.5, color);
            }
        }
        for p in &pts {
            paint_marker(canvas, ser.marker, p.0, p.1, ser.marker_color.unwrap_or(color));
        }
    }
    finish_frame(
        canvas,
        chart,
        font,
        gdi.as_ref(),
        label_px,
        &title_lines,
        title_px,
        title_top,
        title_face.as_ref(),
        &plot,
        host,
        true,
    )?;
    Ok(())
}

fn radar_point(cx: f64, cy: f64, r: f64, i: usize, n: usize) -> (f64, f64) {
    // Close-enough: first category at 12 o'clock, clockwise (same as pie).
    let deg = 360.0 * (i as f64) / (n as f64);
    let a = deg.to_radians();
    (cx + a.sin() * r, cy - a.cos() * r)
}

pub(super) fn paint_bubble(
    canvas: &mut Canvas,
    chart: &ColumnChart,
    font: Option<&Font>,
    host: ChartHost,
) -> Result<(), Error> {
    let (plot, font_bits) = cartesian_frame(canvas, chart, font, host)?;
    let (font, gdi, label_px, title_lines, title_px, title_top, title_face) = font_bits;
    let y_budget = axis::interval_budget_dpi(canvas.h, 12.0, canvas.dpi);
    let y_scale = axis::scale(&chart.series, chart.axis, y_budget)?;
    let x_spec = chart.axis_x.unwrap_or(chart.axis);
    let x_budget = axis::interval_budget_span(plot.w.max(1.0), 12.0, 3.0);
    let x_scale = axis::scale_values(
        chart.series.iter().flat_map(|s| s.x_values.iter().copied()),
        x_spec,
        x_budget,
    )?;
    let scale_pct = chart
        .other
        .as_ref()
        .map(|o| o.bubble_scale / 100.0)
        .unwrap_or(1.0)
        .clamp(0.2, 3.0);
    let max_size = chart
        .series
        .iter()
        .flat_map(|s| s.bubble_sizes.iter().copied())
        .flatten()
        .filter(|v| v.is_finite() && *v > 0.0)
        .fold(0.0_f64, f64::max)
        .max(1.0);
    // Close-enough: area scales with bubbleSize; max diameter is a fraction
    // of the shorter plot side times bubbleScale/100. Not a unique lock.
    let max_d = plot.w.min(plot.h) * 0.28 * scale_pct;
    draw_val_grid(canvas, chart, &plot, &x_scale, &y_scale, font, gdi.as_ref(), label_px);
    for ser in &chart.series {
        let color = ser.color.unwrap_or(chart.accents[ser.slot % 6]);
        let n = ser
            .values
            .len()
            .max(ser.x_values.len())
            .max(ser.bubble_sizes.len());
        for i in 0..n {
            let Some(x) = ser.x_values.get(i).copied().flatten() else {
                continue;
            };
            let Some(y) = ser.values.get(i).copied().flatten() else {
                continue;
            };
            let Some(sz) = ser.bubble_sizes.get(i).copied().flatten() else {
                continue;
            };
            if !x.is_finite() || !y.is_finite() || !sz.is_finite() || sz <= 0.0 {
                continue;
            }
            let px = x_map(x, x_scale.min, x_scale.max, &plot, x_scale.reversed);
            let py = y_map(y, y_scale.min, y_scale.max, &plot, y_scale.log_base, y_scale.reversed);
            let r = (sz / max_size).sqrt() * (max_d / 2.0);
            if r > 0.5 {
                canvas.fill_circle(px, py, r, color);
            }
        }
    }
    finish_frame(
        canvas,
        chart,
        font,
        gdi.as_ref(),
        label_px,
        &title_lines,
        title_px,
        title_top,
        title_face.as_ref(),
        &plot,
        host,
        true,
    )?;
    Ok(())
}

pub(super) fn paint_stock(
    canvas: &mut Canvas,
    chart: &ColumnChart,
    font: Option<&Font>,
    host: ChartHost,
) -> Result<(), Error> {
    if chart.series.len() != 3 {
        return Err(unsupported());
    }
    let (plot, font_bits) = cartesian_frame(canvas, chart, font, host)?;
    let (font, gdi, label_px, title_lines, title_px, title_top, title_face) = font_bits;
    let budget = axis::interval_budget_dpi(canvas.h, 12.0, canvas.dpi);
    let scale = axis::scale(&chart.series, chart.axis, budget)?;
    let n = chart
        .series
        .iter()
        .map(|s| s.values.len())
        .max()
        .unwrap_or(0)
        .max(chart.categories.len())
        .max(1);
    draw_val_grid_y_only(canvas, chart, &plot, &scale, font, gdi.as_ref(), label_px, n);
    let high = &chart.series[0];
    let low = &chart.series[1];
    let close = &chart.series[2];
    let stem = [60, 60, 60];
    for i in 0..n {
        let x = cat_center(i, n, &plot);
        let Some(h) = high.values.get(i).copied().flatten() else {
            continue;
        };
        let Some(l) = low.values.get(i).copied().flatten() else {
            continue;
        };
        if !h.is_finite() || !l.is_finite() {
            continue;
        }
        let y_h = y_map(h, scale.min, scale.max, &plot, scale.log_base, scale.reversed);
        let y_l = y_map(l, scale.min, scale.max, &plot, scale.log_base, scale.reversed);
        canvas.stroke_segment(x, y_h, x, y_l, 1.25, stem);
        if let Some(c) = close.values.get(i).copied().flatten() {
            if c.is_finite() {
                let y_c = y_map(c, scale.min, scale.max, &plot, scale.log_base, scale.reversed);
                let color = close.color.unwrap_or(chart.accents[close.slot % 6]);
                paint_marker(
                    canvas,
                    if matches!(close.marker, MarkerKind::None) {
                        MarkerKind::Shaped {
                            symbol: MarkerSymbol::Circle,
                            size: canvas.dpi.pt_to_px_font(5.0),
                        }
                    } else {
                        close.marker
                    },
                    x,
                    y_c,
                    close.marker_color.unwrap_or(color),
                );
            }
        }
    }
    finish_frame(
        canvas,
        chart,
        font,
        gdi.as_ref(),
        label_px,
        &title_lines,
        title_px,
        title_top,
        title_face.as_ref(),
        &plot,
        host,
        true,
    )?;
    Ok(())
}

fn cat_center(i: usize, n: usize, plot: &Rect) -> f64 {
    let slot = plot.w / n.max(1) as f64;
    plot.x + slot * (i as f64 + 0.5)
}

pub(super) fn paint_surface_top(
    canvas: &mut Canvas,
    chart: &ColumnChart,
    font: Option<&Font>,
    host: ChartHost,
) -> Result<(), Error> {
    let (plot, font_bits) = cartesian_frame(canvas, chart, font, host)?;
    let (font, gdi, label_px, title_lines, title_px, title_top, title_face) = font_bits;
    let budget = axis::interval_budget_dpi(canvas.h, 12.0, canvas.dpi);
    let scale = axis::scale(&chart.series, chart.axis, budget)?;
    let n_cat = chart
        .categories
        .len()
        .max(
            chart
                .series
                .iter()
                .map(|s| s.values.len())
                .max()
                .unwrap_or(0),
        )
        .max(1);
    let n_ser = chart.series.len().max(1);
    // Vertex grid: categories along X, series along Y (series 0 at the bottom).
    let vx: Vec<f64> = (0..n_cat)
        .map(|i| {
            if n_cat == 1 {
                plot.x + plot.w / 2.0
            } else {
                plot.x + plot.w * (i as f64) / (n_cat as f64 - 1.0)
            }
        })
        .collect();
    let vy: Vec<f64> = (0..n_ser)
        .map(|j| {
            if n_ser == 1 {
                plot.y + plot.h / 2.0
            } else {
                plot.y + plot.h - plot.h * (j as f64) / (n_ser as f64 - 1.0)
            }
        })
        .collect();
    let val = |j: usize, i: usize| -> f64 {
        chart
            .series
            .get(j)
            .and_then(|s| s.values.get(i).copied().flatten())
            .unwrap_or(scale.min)
    };
    if n_cat >= 2 && n_ser >= 2 {
        for j in 0..n_ser - 1 {
            for i in 0..n_cat - 1 {
                let a = (vx[i], vy[j], val(j, i));
                let b = (vx[i + 1], vy[j], val(j, i + 1));
                let c = (vx[i + 1], vy[j + 1], val(j + 1, i + 1));
                let d = (vx[i], vy[j + 1], val(j + 1, i));
                // Close-enough: two triangles, each a solid major-unit band
                // of the average of its vertices.
                fill_band_tri(canvas, a, b, c, &scale);
                fill_band_tri(canvas, a, c, d, &scale);
            }
        }
    }
    // Outline.
    canvas.stroke_segment(plot.x, plot.y, plot.x + plot.w, plot.y, 1.0, [80, 80, 80]);
    canvas.stroke_segment(
        plot.x + plot.w,
        plot.y,
        plot.x + plot.w,
        plot.y + plot.h,
        1.0,
        [80, 80, 80],
    );
    canvas.stroke_segment(
        plot.x + plot.w,
        plot.y + plot.h,
        plot.x,
        plot.y + plot.h,
        1.0,
        [80, 80, 80],
    );
    canvas.stroke_segment(plot.x, plot.y + plot.h, plot.x, plot.y, 1.0, [80, 80, 80]);
    for (i, name) in chart.categories.iter().enumerate().take(n_cat) {
        let w = text_width(font, gdi.as_ref(), name, label_px);
        draw_text(
            canvas,
            font,
            gdi.as_ref(),
            name,
            vx[i] - w / 2.0,
            plot.y + plot.h + canvas.dpi.scale_f(4.0),
            label_px,
            [0, 0, 0],
        );
    }
    for (j, ser) in chart.series.iter().enumerate().take(n_ser) {
        draw_text(
            canvas,
            font,
            gdi.as_ref(),
            &ser.name,
            plot.x + plot.w + canvas.dpi.scale_f(4.0),
            vy[j] - text_height(font, label_px) / 2.0,
            label_px,
            [0, 0, 0],
        );
    }
    draw_surface_band_legend(canvas, chart, font, gdi.as_ref(), label_px, &scale);
    finish_frame(
        canvas,
        chart,
        font,
        gdi.as_ref(),
        label_px,
        &title_lines,
        title_px,
        title_top,
        title_face.as_ref(),
        &plot,
        host,
        false,
    )?;
    Ok(())
}

fn fill_band_tri(
    canvas: &mut Canvas,
    a: (f64, f64, f64),
    b: (f64, f64, f64),
    c: (f64, f64, f64),
    scale: &axis::Scale,
) {
    let avg = (a.2 + b.2 + c.2) / 3.0;
    canvas.fill_polygon(&[(a.0, a.1), (b.0, b.1), (c.0, c.1)], band_color(avg, scale));
}

fn band_color(v: f64, scale: &axis::Scale) -> [u8; 3] {
    // Close-enough Excel-like surface bands; not a unique official palette.
    let step = if scale.major.abs() > 1e-12 {
        scale.major.abs()
    } else {
        ((scale.max - scale.min).abs() / 4.0).max(1.0)
    };
    let idx = ((v - scale.min) / step).floor().max(0.0) as usize;
    const BANDS: [[u8; 3]; 4] = [
        [47, 84, 150],
        [237, 125, 49],
        [84, 130, 53],
        [68, 158, 191],
    ];
    BANDS[idx.min(BANDS.len() - 1)]
}

fn draw_surface_band_legend(
    canvas: &mut Canvas,
    chart: &ColumnChart,
    font: Option<&Font>,
    gdi: Option<&super::super::chart_gdi::Face>,
    label_px: f32,
    scale: &axis::Scale,
) {
    let step = if scale.major.abs() > 1e-12 {
        scale.major.abs()
    } else {
        ((scale.max - scale.min).abs() / 4.0).max(1.0)
    };
    let mut bands = Vec::new();
    let mut v = scale.max;
    while v - step > scale.min - 1e-9 && bands.len() < 8 {
        let lo = (v - step).max(scale.min);
        bands.push((lo, v));
        v -= step;
    }
    if bands.is_empty() {
        bands.push((scale.min, scale.max));
    }
    let x = canvas.w as f64 - canvas.dpi.scale_f(70.0);
    let mut y = canvas.dpi.scale_f(36.0);
    let sw = canvas.dpi.scale_f(10.0);
    for (lo, hi) in bands {
        let mid = (lo + hi) * 0.5;
        canvas.fill_rect(
            x as i32,
            y as i32,
            (x + sw) as i32,
            (y + sw) as i32,
            band_color(mid, scale),
        );
        let label = format!("{:.0}-{:.0}", lo, hi);
        draw_text(
            canvas,
            font,
            gdi,
            &label,
            x + sw + canvas.dpi.scale_f(4.0),
            y,
            label_px,
            [0, 0, 0],
        );
        y += canvas.dpi.scale_f(18.0);
    }
    let _ = chart;
}

type FontBits<'a> = (
    Option<&'a Font>,
    Option<super::super::chart_gdi::Face>,
    f32,
    Vec<String>,
    f32,
    f64,
    Option<super::super::chart_gdi::Face>,
);

fn cartesian_frame<'a>(
    canvas: &mut Canvas,
    chart: &ColumnChart,
    font: Option<&'a Font>,
    host: ChartHost,
) -> Result<(Rect, FontBits<'a>), Error> {
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
    let inset_right = legend_right_inset(chart, font, gdi.as_ref(), label_px, canvas.dpi)
        .max(canvas.dpi.scale_px(48));
    let mut plot_x = canvas.dpi.scale_px(36);
    let mut plot_r = canvas.w as i32 - inset_right;
    let mut plot_b = canvas.h as i32 - scale_plot_trailing(canvas.dpi, INSET_BOTTOM + 8);
    if !chart.legend_spec.legacy {
        let reserve = legend_layout.reserve();
        plot_x += reserve[0];
        plot_y += reserve[1];
        plot_r = canvas.w as i32 - 1 - scale_plot_leading(canvas.dpi, INSET_TOP) - reserve[2];
        plot_b -= reserve[3];
    }
    if plot_r - plot_x < 16 || plot_b - plot_y < 16 {
        return Err(unsupported());
    }
    let plot = Rect {
        x: f64::from(plot_x),
        y: f64::from(plot_y),
        w: f64::from(plot_r - plot_x),
        h: f64::from(plot_b - plot_y),
    };
    if let Some(fill) = canvas.plot_fill.clone() {
        canvas.paint_fractional_plot(&plot, fill)?;
    }
    Ok((
        plot,
        (
            font,
            gdi,
            label_px,
            title_lines,
            title_px,
            title_top,
            title_face,
        ),
    ))
}

fn finish_frame(
    canvas: &mut Canvas,
    chart: &ColumnChart,
    font: Option<&Font>,
    gdi: Option<&super::super::chart_gdi::Face>,
    label_px: f32,
    title_lines: &[String],
    title_px: f32,
    title_top: f64,
    title_face: Option<&super::super::chart_gdi::Face>,
    plot: &Rect,
    host: ChartHost,
    draw_right_legend: bool,
) -> Result<(), Error> {
    if !title_lines.is_empty() {
        let face = title_face.or(gdi);
        draw_chart_title(canvas, font, face, title_lines, title_px, title_top);
    }
    if draw_right_legend && chart.legend.is_some() {
        let inset_right = legend_right_inset(chart, font, gdi, label_px, canvas.dpi);
        let legend_plot_r = canvas.w as i32 - inset_right;
        draw_legend(canvas, chart, font, gdi, label_px, legend_plot_r)?;
    }
    if matches!(host, ChartHost::Excel) {
        canvas.finish_excel_frame(Some(plot));
    }
    Ok(())
}

fn draw_val_grid(
    canvas: &mut Canvas,
    chart: &ColumnChart,
    plot: &Rect,
    x_scale: &axis::Scale,
    y_scale: &axis::Scale,
    font: Option<&Font>,
    gdi: Option<&super::super::chart_gdi::Face>,
    label_px: f32,
) {
    draw_val_grid_y_only(canvas, chart, plot, y_scale, font, gdi, label_px, 0);
    let xt = tick_values(x_scale.min, x_scale.max, x_scale.major);
    for t in &xt {
        let x = x_map(*t, x_scale.min, x_scale.max, plot, x_scale.reversed);
        canvas.stroke_segment(x, plot.y, x, plot.y + plot.h, 1.0, [220, 220, 220]);
        let s = fmt_tick_with(*t, x_scale.tick_format);
        let w = text_width(font, gdi, &s, label_px);
        draw_text(
            canvas,
            font,
            gdi,
            &s,
            x - w / 2.0,
            plot.y + plot.h + canvas.dpi.scale_f(3.0),
            label_px,
            [0, 0, 0],
        );
    }
}

fn draw_val_grid_y_only(
    canvas: &mut Canvas,
    chart: &ColumnChart,
    plot: &Rect,
    scale: &axis::Scale,
    font: Option<&Font>,
    gdi: Option<&super::super::chart_gdi::Face>,
    label_px: f32,
    n_cat: usize,
) {
    let hide_val = scale.hide_tick_labels;
    let ticks = tick_values(scale.min, scale.max, scale.major);
    for t in &ticks {
        let y = y_map(*t, scale.min, scale.max, plot, scale.log_base, scale.reversed);
        if chart.grid {
            canvas.stroke_segment(plot.x, y, plot.x + plot.w, y, 1.0, [200, 200, 200]);
        }
        if !hide_val {
            let s = fmt_tick_with(*t, scale.tick_format);
            let w = text_width(font, gdi, &s, label_px);
            draw_text(
                canvas,
                font,
                gdi,
                &s,
                plot.x - w - canvas.dpi.scale_f(6.0),
                y - text_height(font, label_px) / 2.0,
                label_px,
                [0, 0, 0],
            );
        }
    }
    canvas.stroke_segment(plot.x, plot.y, plot.x, plot.y + plot.h, 1.0, [0, 0, 0]);
    canvas.stroke_segment(
        plot.x,
        plot.y + plot.h,
        plot.x + plot.w,
        plot.y + plot.h,
        1.0,
        [0, 0, 0],
    );
    if n_cat > 0 {
        for i in 0..n_cat {
            let x = cat_center(i, n_cat, plot);
            if let Some(name) = chart.categories.get(i) {
                let w = text_width(font, gdi, name, label_px);
                draw_text(
                    canvas,
                    font,
                    gdi,
                    name,
                    x - w / 2.0,
                    plot.y + plot.h + canvas.dpi.scale_f(4.0),
                    label_px,
                    [0, 0, 0],
                );
            }
        }
    }
}

pub(super) fn excel_serial_ymd(serial: f64) -> Option<(i32, u32, u32)> {
    if !serial.is_finite() || serial < 1.0 {
        return None;
    }
    // 1900 date system. Excel serial 25569 is 1970-01-01 (Unix day 0).
    // The formula holds for serials >= 61 (after Excel's fake 1900-02-29).
    // Close-enough for daily stock labels; not a unique official date-axis lock.
    let days = serial.floor() as i64;
    civil_from_days(days - 25569)
}

fn civil_from_days(z: i64) -> Option<(i32, u32, u32)> {
    // Howard Hinnant civil_from_days (Unix day 0 = 1970-01-01).
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = y + if m <= 2 { 1 } else { 0 };
    if y < 0 || m < 1 || m > 12 || d < 1 || d > 31 {
        return None;
    }
    Some((y as i32, m as u32, d as u32))
}

pub(super) fn format_excel_date(serial: f64) -> String {
    match excel_serial_ymd(serial) {
        Some((y, m, d)) => format!("{y:04}-{m:02}-{d:02}"),
        None => format!("{serial:.0}"),
    }
}
