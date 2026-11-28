//! Close-enough 3D chart export so Office-exportable F24 charts produce an image.
//!
//! Official Excel 16 Chart.Export gold exists for bar3DChart (col/clustered/
//! shape=box), line3DChart, area3DChart, pie3DChart, and surface3DChart
//! (wireframe=0). NotImplemented is not acceptable for those cases.
//!
//! Projection uses explicit `c:view3D` `rotX` / `rotY` / `rAngAx` when present.
//! Omitted `perspective`, `depthPercent`, and `hPercent`, and omitted view3D
//! children, use ECMA-376 schema defaults (perspective 30, depth/h 100,
//! rotX/rotY 0, rAngAx true). Those defaults, the perspective division,
//! box/ribbon/pie thickness, wall layout, face shading, and surface value
//! bands are close-enough approximations, not unique official-matrix locks.
//! Residual pixels stay on the review page. Do not auto-mark pass.

use fontdue::Font;

use crate::error::Error;

use super::*;

const WALL_EDGE: [u8; 3] = [137, 137, 137];
const LABEL_RGB: [u8; 3] = [0, 0, 0];

pub(super) fn paint_3d(
    canvas: &mut Canvas,
    chart: &ColumnChart,
    font: Option<&Font>,
    host: ChartHost,
) -> Result<(), Error> {
    let kind = chart.chart3d.ok_or_else(|| {
        Error::with_path(crate::error::ErrorKind::ChartImageUnsupported, "id")
    })?;
    let view = chart.view3d.as_ref().ok_or_else(|| {
        Error::with_path(crate::error::ErrorKind::ChartImageUnsupported, "id")
    })?;

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
    let budget = axis::interval_budget_dpi(canvas.h, em, canvas.dpi);
    let scale = axis::scale(&chart.series, chart.axis, budget)?;

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
    let inset_right = if matches!(kind, Chart3DKind::Pie) {
        pie_legend_right_inset(chart, font, gdi.as_ref(), label_px, canvas.dpi)
    } else if matches!(kind, Chart3DKind::Surface) {
        legend_right_inset(chart, font, gdi.as_ref(), label_px, canvas.dpi).max(
            (text_width(font, gdi.as_ref(), "60-80", label_px) as i32)
                + canvas.dpi.scale_px(28),
        )
    } else {
        legend_right_inset(chart, font, gdi.as_ref(), label_px, canvas.dpi)
    };
    let mut plot_x = canvas.dpi.scale_px(12);
    let mut plot_r = canvas.w as i32 - inset_right;
    let mut plot_b = canvas.h as i32 - scale_plot_trailing(canvas.dpi, INSET_BOTTOM);
    if !chart.legend_spec.legacy {
        let reserve = legend_layout.reserve();
        plot_x += reserve[0];
        plot_y += reserve[1];
        plot_r = canvas.w as i32 - 1 - scale_plot_leading(canvas.dpi, INSET_TOP) - reserve[2];
        plot_b -= reserve[3];
    }
    if plot_r - plot_x < 16 || plot_b - plot_y < 16 {
        return Ok(());
    }
    let viewport = Rect {
        x: f64::from(plot_x),
        y: f64::from(plot_y),
        w: f64::from(plot_r - plot_x),
        h: f64::from(plot_b - plot_y),
    };

    match kind {
        Chart3DKind::Pie => draw_pie_3d(canvas, chart, view, &viewport, &scale)?,
        Chart3DKind::Surface => {
            draw_cartesian_3d(canvas, chart, view, kind, &viewport, &scale, font, gdi.as_ref(), label_px)?
        }
        _ => draw_cartesian_3d(
            canvas,
            chart,
            view,
            kind,
            &viewport,
            &scale,
            font,
            gdi.as_ref(),
            label_px,
        )?,
    }

    if !title_lines.is_empty() {
        let face = title_face.as_ref().or(gdi.as_ref());
        draw_chart_title(canvas, font, face, &title_lines, title_px, title_top);
    }
    let legend_plot_r = plot_r;
    if matches!(kind, Chart3DKind::Surface) {
        draw_surface_legend(canvas, chart, font, gdi.as_ref(), label_px, &scale, legend_plot_r);
    } else if matches!(kind, Chart3DKind::Pie) {
        draw_pie_legend(canvas, chart, font, gdi.as_ref(), label_px, legend_plot_r);
    } else if chart.legend.is_some() {
        draw_legend(canvas, chart, font, gdi.as_ref(), label_px, legend_plot_r)?;
    }
    let _ = em;
    Ok(())
}

fn draw_cartesian_3d(
    canvas: &mut Canvas,
    chart: &ColumnChart,
    view: &View3D,
    kind: Chart3DKind,
    viewport: &Rect,
    scale: &axis::Scale,
    font: Option<&Font>,
    gdi: Option<&super::super::chart_gdi::Face>,
    label_px: f32,
) -> Result<(), Error> {
    let n_cat = chart
        .categories
        .len()
        .max(chart.series.iter().map(|s| s.values.len()).max().unwrap_or(0))
        .max(1);
    let n_ser = chart.series.len().max(1);
    let x_max = n_cat as f64;
    let y_min = scale.min;
    let y_max = scale.max;
    let z_max = (x_max * (view.depth_percent / 100.0)).max(1.0);
    let y_span = (y_max - y_min).abs().max(1e-9);
    // hPercent close-enough: stretch the world Y relative to X.
    let y_world_h = x_max * (view.h_percent / 100.0).clamp(0.2, 4.0);
    let proj = Projector::fit(
        view,
        WorldBox {
            x0: 0.0,
            x1: x_max,
            y0: 0.0,
            y1: y_world_h,
            z0: 0.0,
            z1: z_max,
        },
        viewport,
    );
    let map_y = |v: f64| -> f64 { (v - y_min) / y_span * y_world_h };

    draw_walls(canvas, &proj, x_max, y_world_h, z_max, scale, map_y);

    match kind {
        Chart3DKind::ColumnClustered => {
            draw_column_boxes(canvas, chart, &proj, x_max, z_max, n_cat, n_ser, map_y, y_min)
        }
        Chart3DKind::Line => draw_line_ribbons(canvas, chart, &proj, x_max, z_max, n_cat, n_ser, map_y),
        Chart3DKind::Area => draw_area_prisms(canvas, chart, &proj, x_max, z_max, n_cat, n_ser, map_y),
        Chart3DKind::Surface => draw_surface_mesh(canvas, chart, &proj, x_max, z_max, n_cat, n_ser, map_y, scale),
        Chart3DKind::Pie => {}
    }

    draw_3d_labels(
        canvas,
        chart,
        kind,
        &proj,
        x_max,
        y_world_h,
        z_max,
        n_cat,
        n_ser,
        scale,
        map_y,
        font,
        gdi,
        label_px,
    );
    Ok(())
}

struct WorldBox {
    x0: f64,
    x1: f64,
    y0: f64,
    y1: f64,
    z0: f64,
    z1: f64,
}

struct Projector {
    rot_x: f64,
    rot_y: f64,
    perspective: Option<f64>,
    ox: f64,
    oy: f64,
    scale: f64,
}

impl Projector {
    fn raw(view: &View3D, x: f64, y: f64, z: f64) -> (f64, f64, f64) {
        let rx = view.rot_x.to_radians();
        let ry = view.rot_y.to_radians();
        // Yaw around Y, then pitch around X (elevation). Close-enough composition.
        let x1 = x * ry.cos() + z * ry.sin();
        let z1 = -x * ry.sin() + z * ry.cos();
        let y1 = y * rx.cos() - z1 * rx.sin();
        let z2 = y * rx.sin() + z1 * rx.cos();
        let (x2, y2) = if view.right_angle_axes {
            (x1, y1)
        } else {
            let p = view.perspective.max(0.0);
            let f = 1.0 / (1.0 + (z2.max(-0.5) + 0.5) * p / 2000.0);
            (x1 * f, y1 * f)
        };
        (x2, y2, z2)
    }

    fn fit(view: &View3D, world: WorldBox, screen: &Rect) -> Self {
        let corners = [
            (world.x0, world.y0, world.z0),
            (world.x1, world.y0, world.z0),
            (world.x0, world.y1, world.z0),
            (world.x1, world.y1, world.z0),
            (world.x0, world.y0, world.z1),
            (world.x1, world.y0, world.z1),
            (world.x0, world.y1, world.z1),
            (world.x1, world.y1, world.z1),
        ];
        let mut min_x = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        for &(x, y, z) in &corners {
            let (px, py, _) = Self::raw(view, x, y, z);
            min_x = min_x.min(px);
            max_x = max_x.max(px);
            min_y = min_y.min(py);
            max_y = max_y.max(py);
        }
        let w = (max_x - min_x).max(1e-9);
        let h = (max_y - min_y).max(1e-9);
        let pad = 0.08;
        let avail_w = screen.w * (1.0 - 2.0 * pad);
        let avail_h = screen.h * (1.0 - 2.0 * pad);
        let scale = (avail_w / w).min(avail_h / h);
        let ox = screen.x + screen.w * 0.5 - scale * (min_x + max_x) * 0.5;
        // Screen Y grows downward; world Y grows up.
        let oy = screen.y + screen.h * 0.5 + scale * (min_y + max_y) * 0.5;
        Self {
            rot_x: view.rot_x,
            rot_y: view.rot_y,
            perspective: if view.right_angle_axes {
                None
            } else {
                Some(view.perspective)
            },
            ox,
            oy,
            scale,
        }
    }

    fn project(&self, x: f64, y: f64, z: f64) -> (f64, f64) {
        let view = View3D {
            rot_x: self.rot_x,
            rot_y: self.rot_y,
            right_angle_axes: self.perspective.is_none(),
            perspective: self.perspective.unwrap_or(0.0),
            depth_percent: 100.0,
            h_percent: 100.0,
        };
        let (px, py, _) = Self::raw(&view, x, y, z);
        (self.ox + self.scale * px, self.oy - self.scale * py)
    }

    fn depth(&self, x: f64, y: f64, z: f64) -> f64 {
        let view = View3D {
            rot_x: self.rot_x,
            rot_y: self.rot_y,
            right_angle_axes: self.perspective.is_none(),
            perspective: self.perspective.unwrap_or(0.0),
            depth_percent: 100.0,
            h_percent: 100.0,
        };
        Self::raw(&view, x, y, z).2
    }
}

fn draw_walls(
    canvas: &mut Canvas,
    proj: &Projector,
    x_max: f64,
    y_h: f64,
    z_max: f64,
    scale: &axis::Scale,
    map_y: impl Fn(f64) -> f64,
) {
    let floor = [
        proj.project(0.0, 0.0, 0.0),
        proj.project(x_max, 0.0, 0.0),
        proj.project(x_max, 0.0, z_max),
        proj.project(0.0, 0.0, z_max),
    ];
    let back = [
        proj.project(0.0, 0.0, z_max),
        proj.project(x_max, 0.0, z_max),
        proj.project(x_max, y_h, z_max),
        proj.project(0.0, y_h, z_max),
    ];
    let side = [
        proj.project(0.0, 0.0, 0.0),
        proj.project(0.0, 0.0, z_max),
        proj.project(0.0, y_h, z_max),
        proj.project(0.0, y_h, 0.0),
    ];
    canvas.fill_polygon(&back, [228, 240, 240]);
    canvas.fill_polygon(&side, [222, 234, 234]);
    canvas.fill_polygon(&floor, [216, 228, 228]);
    stroke_loop(canvas, &back);
    stroke_loop(canvas, &side);
    stroke_loop(canvas, &floor);

    let ticks = tick_values(scale.min, scale.max, scale.major);
    for v in &ticks {
        let y = map_y(*v);
        let a = proj.project(0.0, y, 0.0);
        let b = proj.project(0.0, y, z_max);
        let c = proj.project(x_max, y, z_max);
        canvas.stroke_segment(a.0, a.1, b.0, b.1, 1.0, WALL_EDGE);
        canvas.stroke_segment(b.0, b.1, c.0, c.1, 1.0, WALL_EDGE);
    }
}

fn stroke_loop(canvas: &mut Canvas, pts: &[(f64, f64)]) {
    for i in 0..pts.len() {
        let a = pts[i];
        let b = pts[(i + 1) % pts.len()];
        canvas.stroke_segment(a.0, a.1, b.0, b.1, 1.0, WALL_EDGE);
    }
}

fn shade(rgb: [u8; 3], k: f64) -> [u8; 3] {
    let t = |c: u8| -> u8 {
        let v = f64::from(c) * k;
        v.round().clamp(0.0, 255.0) as u8
    };
    [t(rgb[0]), t(rgb[1]), t(rgb[2])]
}

fn series_color(chart: &ColumnChart, si: usize) -> [u8; 3] {
    chart.series.get(si).and_then(|s| s.color).unwrap_or(chart.accents[si % 6])
}

fn draw_box(canvas: &mut Canvas, proj: &Projector, x0: f64, x1: f64, y0: f64, y1: f64, z0: f64, z1: f64, rgb: [u8; 3]) {
    let faces = [
        // front z=z0
        ([ (x0, y0, z0), (x1, y0, z0), (x1, y1, z0), (x0, y1, z0) ], 1.0),
        // back z=z1
        ([ (x0, y0, z1), (x1, y0, z1), (x1, y1, z1), (x0, y1, z1) ], 0.7),
        // top y=y1
        ([ (x0, y1, z0), (x1, y1, z0), (x1, y1, z1), (x0, y1, z1) ], 1.12),
        // right x=x1
        ([ (x1, y0, z0), (x1, y0, z1), (x1, y1, z1), (x1, y1, z0) ], 0.75),
        // left x=x0
        ([ (x0, y0, z0), (x0, y0, z1), (x0, y1, z1), (x0, y1, z0) ], 0.82),
    ];
    let mut ordered: Vec<(f64, [(f64, f64, f64); 4], f64)> = faces
        .iter()
        .map(|&(pts, k)| {
            let mid = (
                (pts[0].0 + pts[2].0) * 0.5,
                (pts[0].1 + pts[2].1) * 0.5,
                (pts[0].2 + pts[2].2) * 0.5,
            );
            (proj.depth(mid.0, mid.1, mid.2), pts, k)
        })
        .collect();
    ordered.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    for (_, pts, k) in ordered {
        let poly: Vec<(f64, f64)> = pts.iter().map(|&(x, y, z)| proj.project(x, y, z)).collect();
        canvas.fill_polygon(&poly, shade(rgb, k));
        stroke_loop(canvas, &poly);
    }
}

fn draw_column_boxes(
    canvas: &mut Canvas,
    chart: &ColumnChart,
    proj: &Projector,
    x_max: f64,
    z_max: f64,
    n_cat: usize,
    n_ser: usize,
    map_y: impl Fn(f64) -> f64,
    y_min: f64,
) {
    let gap = (chart.gap_width / 100.0).clamp(0.0, 5.0);
    let slot = x_max / n_cat as f64;
    let inner = slot / (1.0 + gap / 1.5);
    let bar_w = inner / n_ser as f64;
    let pad = (slot - inner) * 0.5;
    // Close-enough box depth: a large fraction of floor depth, not a unique lock.
    let z0 = z_max * 0.18;
    let z1 = z_max * 0.82;
    let mut items: Vec<(f64, usize, usize)> = Vec::new();
    for ci in 0..n_cat {
        for si in 0..n_ser {
            let x_mid = ci as f64 * slot + pad + (si as f64 + 0.5) * bar_w;
            items.push((proj.depth(x_mid, map_y(y_min), (z0 + z1) * 0.5), ci, si));
        }
    }
    items.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    for (_, ci, si) in items {
        let Some(ser) = chart.series.get(si) else { continue };
        let Some(Some(v)) = ser.values.get(ci).copied() else { continue };
        if !v.is_finite() || v == 0.0 {
            continue;
        }
        let (y0, y1) = if v >= 0.0 {
            (map_y(y_min.max(0.0)), map_y(v))
        } else {
            (map_y(v), map_y(y_min.min(0.0)))
        };
        let x0 = ci as f64 * slot + pad + si as f64 * bar_w;
        let x1 = x0 + bar_w * 0.92;
        draw_box(canvas, proj, x0, x1, y0.min(y1), y0.max(y1), z0, z1, series_color(chart, si));
    }
}

fn series_z(si: usize, n_ser: usize, z_max: f64) -> (f64, f64) {
    let slot = z_max / n_ser as f64;
    let z0 = (si as f64 + 0.15) * slot;
    let z1 = (si as f64 + 0.85) * slot;
    (z0, z1)
}

fn cat_x(ci: usize, n_cat: usize, x_max: f64, midcat: bool) -> f64 {
    if n_cat <= 1 {
        return x_max * 0.5;
    }
    if midcat {
        ci as f64 / (n_cat as f64 - 1.0) * x_max
    } else {
        (ci as f64 + 0.5) / n_cat as f64 * x_max
    }
}

fn draw_line_ribbons(
    canvas: &mut Canvas,
    chart: &ColumnChart,
    proj: &Projector,
    x_max: f64,
    z_max: f64,
    n_cat: usize,
    n_ser: usize,
    map_y: impl Fn(f64) -> f64,
) {
    let midcat = false;
    let mut order: Vec<usize> = (0..n_ser).collect();
    order.sort_by(|a, b| {
        let za = series_z(*a, n_ser, z_max);
        let zb = series_z(*b, n_ser, z_max);
        proj.depth(x_max * 0.5, 0.0, (za.0 + za.1) * 0.5)
            .partial_cmp(&proj.depth(x_max * 0.5, 0.0, (zb.0 + zb.1) * 0.5))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    for si in order {
        let Some(ser) = chart.series.get(si) else { continue };
        let (z0, z1) = series_z(si, n_ser, z_max);
        let rgb = series_color(chart, si);
        for ci in 0..n_cat.saturating_sub(1) {
            let Some(Some(v0)) = ser.values.get(ci).copied() else { continue };
            let Some(Some(v1)) = ser.values.get(ci + 1).copied() else { continue };
            if !v0.is_finite() || !v1.is_finite() {
                continue;
            }
            let x0 = cat_x(ci, n_cat, x_max, midcat);
            let x1 = cat_x(ci + 1, n_cat, x_max, midcat);
            let y0 = map_y(v0);
            let y1 = map_y(v1);
            draw_box(
                canvas,
                proj,
                x0,
                x1,
                y0.min(y1),
                y0.max(y1).max(y0.min(y1) + 1e-3),
                z0,
                z1,
                rgb,
            );
            // Ribbon along the line (top face already from box is axis-aligned;
            // also fill the slanted top).
            let top = [
                proj.project(x0, y0, z0),
                proj.project(x1, y1, z0),
                proj.project(x1, y1, z1),
                proj.project(x0, y0, z1),
            ];
            canvas.fill_polygon(&top, shade(rgb, 1.1));
            stroke_loop(canvas, &top);
        }
    }
}

fn draw_area_prisms(
    canvas: &mut Canvas,
    chart: &ColumnChart,
    proj: &Projector,
    x_max: f64,
    z_max: f64,
    n_cat: usize,
    n_ser: usize,
    map_y: impl Fn(f64) -> f64,
) {
    let midcat = chart.area_midcat;
    let mut order: Vec<usize> = (0..n_ser).collect();
    order.sort_by(|a, b| {
        let za = series_z(*a, n_ser, z_max);
        let zb = series_z(*b, n_ser, z_max);
        proj.depth(x_max * 0.5, 0.0, (za.0 + za.1) * 0.5)
            .partial_cmp(&proj.depth(x_max * 0.5, 0.0, (zb.0 + zb.1) * 0.5))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    for si in order {
        let Some(ser) = chart.series.get(si) else { continue };
        let (z0, z1) = series_z(si, n_ser, z_max);
        let rgb = series_color(chart, si);
        for ci in 0..n_cat.saturating_sub(1) {
            let Some(Some(v0)) = ser.values.get(ci).copied() else { continue };
            let Some(Some(v1)) = ser.values.get(ci + 1).copied() else { continue };
            if !v0.is_finite() || !v1.is_finite() {
                continue;
            }
            let x0 = cat_x(ci, n_cat, x_max, midcat);
            let x1 = cat_x(ci + 1, n_cat, x_max, midcat);
            let y0 = map_y(v0);
            let y1 = map_y(v1);
            let yb = map_y(0.0);
            let front = [
                proj.project(x0, yb, z0),
                proj.project(x1, yb, z0),
                proj.project(x1, y1, z0),
                proj.project(x0, y0, z0),
            ];
            let top = [
                proj.project(x0, y0, z0),
                proj.project(x1, y1, z0),
                proj.project(x1, y1, z1),
                proj.project(x0, y0, z1),
            ];
            let side = [
                proj.project(x1, yb, z0),
                proj.project(x1, yb, z1),
                proj.project(x1, y1, z1),
                proj.project(x1, y1, z0),
            ];
            let mut faces = vec![
                (proj.depth((x0 + x1) * 0.5, (y0 + y1) * 0.25, z0), front, 1.0),
                (proj.depth((x0 + x1) * 0.5, (y0 + y1) * 0.5, (z0 + z1) * 0.5), top, 1.1),
                (proj.depth(x1, (yb + y1) * 0.5, (z0 + z1) * 0.5), side, 0.75),
            ];
            faces.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
            for (_, poly, k) in faces {
                canvas.fill_polygon(&poly, shade(rgb, k));
                stroke_loop(canvas, &poly);
            }
        }
    }
}

fn draw_surface_mesh(
    canvas: &mut Canvas,
    chart: &ColumnChart,
    proj: &Projector,
    x_max: f64,
    z_max: f64,
    n_cat: usize,
    n_ser: usize,
    map_y: impl Fn(f64) -> f64,
    scale: &axis::Scale,
) {
    let midcat = true;
    let mut quads = Vec::new();
    for si in 0..n_ser.saturating_sub(1) {
        for ci in 0..n_cat.saturating_sub(1) {
            let pts = [(ci, si), (ci + 1, si), (ci + 1, si + 1), (ci, si + 1)];
            let mut world = [(0.0, 0.0, 0.0); 4];
            let mut vals = [0.0; 4];
            let mut ok = true;
            for (k, (c, s)) in pts.iter().enumerate() {
                let Some(ser) = chart.series.get(*s) else {
                    ok = false;
                    break;
                };
                let Some(Some(v)) = ser.values.get(*c).copied() else {
                    ok = false;
                    break;
                };
                if !v.is_finite() {
                    ok = false;
                    break;
                }
                let x = cat_x(*c, n_cat, x_max, midcat);
                let z = if n_ser <= 1 {
                    z_max * 0.5
                } else {
                    *s as f64 / (n_ser as f64 - 1.0) * z_max
                };
                world[k] = (x, map_y(v), z);
                vals[k] = v;
            }
            if !ok {
                continue;
            }
            let mid = (
                (world[0].0 + world[2].0) * 0.5,
                (world[0].1 + world[2].1) * 0.5,
                (world[0].2 + world[2].2) * 0.5,
            );
            let mean = (vals[0] + vals[1] + vals[2] + vals[3]) * 0.25;
            quads.push((proj.depth(mid.0, mid.1, mid.2), world, mean));
        }
    }
    quads.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    for (_, world, mean) in quads {
        let poly: Vec<(f64, f64)> = world.iter().map(|&(x, y, z)| proj.project(x, y, z)).collect();
        canvas.fill_polygon(&poly, band_color(chart, scale, mean));
        stroke_loop(canvas, &poly);
    }
}

fn band_color(chart: &ColumnChart, scale: &axis::Scale, value: f64) -> [u8; 3] {
    let step = if scale.major.abs() > 1e-12 {
        scale.major.abs()
    } else {
        ((scale.max - scale.min).abs() / 4.0).max(1.0)
    };
    let idx = ((value - scale.min) / step).floor() as i32;
    let n = ((scale.max - scale.min) / step).round().max(1.0) as i32;
    // High band uses early accents to match official legend order 60-80 first.
    let band_from_top = (n - 1 - idx).clamp(0, 5) as usize;
    chart.accents[band_from_top % 6]
}

fn draw_3d_labels(
    canvas: &mut Canvas,
    chart: &ColumnChart,
    kind: Chart3DKind,
    proj: &Projector,
    x_max: f64,
    y_h: f64,
    z_max: f64,
    n_cat: usize,
    n_ser: usize,
    scale: &axis::Scale,
    map_y: impl Fn(f64) -> f64,
    font: Option<&Font>,
    gdi: Option<&super::super::chart_gdi::Face>,
    label_px: f32,
) {
    let ticks = tick_values(scale.min, scale.max, scale.major);
    for v in &ticks {
        let (x, y) = proj.project(-x_max * 0.06, map_y(*v), 0.0);
        let label = fmt_tick_with(*v, scale.tick_format);
        let w = text_width(font, gdi, &label, label_px);
        draw_text(canvas, font, gdi, &label, x - w - 2.0, y - f64::from(label_px) * 0.5, label_px, LABEL_RGB);
        let _ = y_h;
    }
    for (ci, name) in chart.categories.iter().enumerate() {
        if ci >= n_cat {
            break;
        }
        let xw = cat_x(ci, n_cat, x_max, matches!(kind, Chart3DKind::Area | Chart3DKind::Surface));
        let (x, y) = proj.project(xw, 0.0, -z_max * 0.06);
        let w = text_width(font, gdi, name, label_px);
        draw_text(canvas, font, gdi, name, x - w * 0.5, y + 2.0, label_px, LABEL_RGB);
    }
    if matches!(kind, Chart3DKind::Line | Chart3DKind::Area) {
        for (si, ser) in chart.series.iter().enumerate() {
            let (z0, z1) = series_z(si, n_ser, z_max);
            let z = (z0 + z1) * 0.5;
            let (x, y) = proj.project(x_max + x_max * 0.04, 0.0, z);
            draw_text(canvas, font, gdi, &ser.name, x, y - f64::from(label_px) * 0.4, label_px, LABEL_RGB);
        }
    }
}

fn draw_pie_3d(
    canvas: &mut Canvas,
    chart: &ColumnChart,
    view: &View3D,
    viewport: &Rect,
    _scale: &axis::Scale,
) -> Result<(), Error> {
    let Some(ser) = chart.series.first() else {
        return Ok(());
    };
    let mut slices: Vec<(usize, f64, [u8; 3])> = Vec::new();
    let mut total = 0.0;
    for (i, v) in ser.values.iter().enumerate() {
        let Some(n) = *v else { continue };
        if !n.is_finite() || n <= 0.0 {
            continue;
        }
        total += n;
        let color = chart
            .point_colors
            .get(i)
            .and_then(|c| *c)
            .unwrap_or(chart.accents[i % 6]);
        slices.push((i, n, color));
    }
    if total <= 0.0 || slices.is_empty() {
        return Ok(());
    }
    let cx = viewport.x + viewport.w * 0.46;
    let cy = viewport.y + viewport.h * 0.52;
    let rx = viewport.w.min(viewport.h * 1.4) * 0.38;
    // Close-enough tilt from rotX; not a unique official ellipse lock.
    let ry = (rx * view.rot_x.to_radians().cos().abs()).max(rx * 0.25);
    let thick = rx * 0.22; // close-enough pie thickness
    let start0 = chart.first_slice_ang;
    // Side wall (front half) first.
    let mut ang = start0;
    for (_, n, color) in &slices {
        let sweep = *n / total * 360.0;
        let a0 = ang;
        let _a1 = ang + sweep;
        ang += sweep;
        // Sample the front-facing rim (degrees clockwise from up, matching 2D pie).
        let steps = ((sweep.abs() / 6.0).ceil() as usize).max(2);
        for s in 0..steps {
            let t0 = a0 + sweep * (s as f64 / steps as f64);
            let t1 = a0 + sweep * ((s + 1) as f64 / steps as f64);
            let (x0, y0) = pie_ellipse(cx, cy, rx, ry, t0);
            let (x1, y1) = pie_ellipse(cx, cy, rx, ry, t1);
            // Only draw sides that face the viewer (lower half of the ellipse).
            if (y0 + y1) * 0.5 < cy {
                continue;
            }
            let poly = [
                (x0, y0),
                (x1, y1),
                (x1, y1 + thick),
                (x0, y0 + thick),
            ];
            canvas.fill_polygon(&poly, shade(*color, 0.68));
        }
    }
    // Top ellipse slices.
    ang = start0;
    for (_, n, color) in &slices {
        let sweep = *n / total * 360.0;
        let a0 = ang;
        let a1 = ang + sweep;
        ang += sweep;
        let mut poly = vec![(cx, cy)];
        let steps = ((sweep.abs() / 4.0).ceil() as usize).max(3);
        for s in 0..=steps {
            let t = a0 + (a1 - a0) * (s as f64 / steps as f64);
            poly.push(pie_ellipse(cx, cy, rx, ry, t));
        }
        canvas.fill_polygon(&poly, *color);
        canvas.stroke_segment(cx, cy, poly[1].0, poly[1].1, 1.0, shade(*color, 0.6));
        let last = *poly.last().unwrap();
        canvas.stroke_segment(cx, cy, last.0, last.1, 1.0, shade(*color, 0.6));
    }
    Ok(())
}

fn pie_ellipse(cx: f64, cy: f64, rx: f64, ry: f64, deg_cw_from_up: f64) -> (f64, f64) {
    // Convert clockwise-from-up to standard math angle: 0 at +X, CCW.
    // 2D pie starts at 12 o'clock going clockwise: angle from +X is 90° - deg.
    let a = (90.0 - deg_cw_from_up).to_radians();
    (cx + rx * a.cos(), cy - ry * a.sin())
}

fn draw_surface_legend(
    canvas: &mut Canvas,
    chart: &ColumnChart,
    font: Option<&Font>,
    gdi: Option<&super::super::chart_gdi::Face>,
    label_px: f32,
    scale: &axis::Scale,
    plot_r: i32,
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
    let box_s = canvas.dpi.scale_px(10) as f64;
    let x = f64::from(plot_r) + canvas.dpi.scale_f(8.0);
    let mut y = canvas.dpi.scale_f(36.0);
    for (i, (lo, hi)) in bands.iter().enumerate() {
        let mean = (*lo + *hi) * 0.5;
        let color = band_color(chart, scale, mean);
        canvas.fill_rect(x as i32, y as i32, (x + box_s) as i32, (y + box_s) as i32, color);
        let label = format!("{}-{}", fmt_tick_with(*lo, scale.tick_format), fmt_tick_with(*hi, scale.tick_format));
        // Prefer compact integers for the official 0-20 style.
        let label = if step >= 1.0 && (*lo - lo.floor()).abs() < 1e-9 && (*hi - hi.floor()).abs() < 1e-9 {
            format!("{}-{}", *lo as i64, *hi as i64)
        } else {
            label
        };
        draw_text(
            canvas,
            font,
            gdi,
            &label,
            x + box_s + 4.0,
            y - 1.0,
            label_px,
            LABEL_RGB,
        );
        y += box_s + 6.0;
        let _ = i;
    }
}
