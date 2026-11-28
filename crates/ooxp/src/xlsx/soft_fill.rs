//! 非 Windows / WASM 的 `fill_device_*` 软件光栅。抗锯齿只求接近可读，不锁 GDI+ 逐像素。
#![cfg_attr(windows, allow(dead_code))]

const SS: i32 = 4;
const MAX_BUF: i32 = 8192;

fn coverage_from_hits(hits: u32) -> u8 {
    ((hits * 255 + (SS * SS / 2) as u32) / (SS * SS) as u32) as u8
}

fn sample_inside(x: i32, y: i32, inside: &mut impl FnMut(f64, f64) -> bool) -> u8 {
    let mut hits = 0u32;
    for sy in 0..SS {
        for sx in 0..SS {
            let px = f64::from(x) + (f64::from(sx) + 0.5) / f64::from(SS);
            let py = f64::from(y) + (f64::from(sy) + 0.5) / f64::from(SS);
            if inside(px, py) {
                hits += 1;
            }
        }
    }
    coverage_from_hits(hits)
}

fn emit_coverage(
    min_x: i32,
    min_y: i32,
    max_x: i32,
    max_y: i32,
    mut inside: impl FnMut(f64, f64) -> bool,
    mut blend: impl FnMut(i32, i32, u8),
) -> bool {
    if min_x >= max_x || min_y >= max_y {
        return false;
    }
    if max_x.saturating_sub(min_x) > MAX_BUF || max_y.saturating_sub(min_y) > MAX_BUF {
        return false;
    }
    for y in min_y..max_y {
        for x in min_x..max_x {
            let cov = sample_inside(x, y, &mut inside);
            if cov != 0 {
                blend(x, y, cov);
            }
        }
    }
    true
}

fn page_ok(rgb: &[u8], page_w: u32, page_h: u32) -> bool {
    page_w > 0
        && page_h > 0
        && rgb.len() == (page_w as usize) * (page_h as usize) * 3
}

fn page_pixel(rgb: &[u8], page_w: u32, page_h: u32, x: i32, y: i32) -> [u8; 3] {
    if x < 0 || y < 0 || x >= page_w as i32 || y >= page_h as i32 {
        return [255, 255, 255];
    }
    let i = (y as usize * page_w as usize + x as usize) * 3;
    if i + 2 >= rgb.len() {
        return [255, 255, 255];
    }
    [rgb[i], rgb[i + 1], rgb[i + 2]]
}

fn set_page_pixel(rgb: &mut [u8], page_w: u32, page_h: u32, x: i32, y: i32, color: [u8; 3]) {
    if x < 0 || y < 0 || x >= page_w as i32 || y >= page_h as i32 {
        return;
    }
    let i = (y as usize * page_w as usize + x as usize) * 3;
    if i + 2 >= rgb.len() {
        return;
    }
    rgb[i..i + 3].copy_from_slice(&color);
}

fn over(dst: [u8; 3], src: [u8; 3], cov: u8) -> [u8; 3] {
    if cov == 0 {
        return dst;
    }
    if cov == 255 {
        return src;
    }
    let a = u16::from(cov);
    let ia = 255 - a;
    [
        ((u16::from(src[0]) * a + u16::from(dst[0]) * ia + 127) / 255) as u8,
        ((u16::from(src[1]) * a + u16::from(dst[1]) * ia + 127) / 255) as u8,
        ((u16::from(src[2]) * a + u16::from(dst[2]) * ia + 127) / 255) as u8,
    ]
}

fn dist_to_segment(px: f64, py: f64, x0: f64, y0: f64, x1: f64, y1: f64) -> f64 {
    let dx = x1 - x0;
    let dy = y1 - y0;
    let len2 = dx * dx + dy * dy;
    if len2 <= 1e-12 {
        let ex = px - x0;
        let ey = py - y0;
        return (ex * ex + ey * ey).sqrt();
    }
    let t = ((px - x0) * dx + (py - y0) * dy) / len2;
    let t = t.clamp(0.0, 1.0);
    let ex = px - (x0 + t * dx);
    let ey = py - (y0 + t * dy);
    (ex * ex + ey * ey).sqrt()
}

fn dist_to_polyline(px: f64, py: f64, pts: &[(f32, f32)]) -> f64 {
    let mut best = f64::INFINITY;
    for pair in pts.windows(2) {
        let (x0, y0) = (f64::from(pair[0].0), f64::from(pair[0].1));
        let (x1, y1) = (f64::from(pair[1].0), f64::from(pair[1].1));
        best = best.min(dist_to_segment(px, py, x0, y0, x1, y1));
    }
    best
}

fn unique_points(points: &[(f32, f32)]) -> Option<Vec<(f32, f32)>> {
    let mut pts = Vec::new();
    for &p in points {
        if !p.0.is_finite() || !p.1.is_finite() {
            return None;
        }
        if pts.last().is_none_or(|q| *q != p) {
            pts.push(p);
        }
    }
    Some(pts)
}

fn stroke_points(
    points: &[(f32, f32)],
    width_px: i32,
    blend: impl FnMut(i32, i32, u8),
) -> bool {
    if width_px < 1 {
        return false;
    }
    let Some(pts) = unique_points(points) else {
        return false;
    };
    if pts.len() < 2 {
        return false;
    }
    let half = f64::from(width_px) * 0.5;
    let pad = width_px + 3;
    let mut min_x = i32::MAX;
    let mut min_y = i32::MAX;
    let mut max_x = i32::MIN;
    let mut max_y = i32::MIN;
    for &(x, y) in &pts {
        min_x = min_x.min(x.floor() as i32 - pad);
        min_y = min_y.min(y.floor() as i32 - pad);
        max_x = max_x.max(x.ceil() as i32 + pad);
        max_y = max_y.max(y.ceil() as i32 + pad);
    }
    emit_coverage(
        min_x,
        min_y,
        max_x + 1,
        max_y + 1,
        |px, py| dist_to_polyline(px, py, &pts) <= half,
        blend,
    )
}

fn flatten_cubic(p0: (f32, f32), p1: (f32, f32), p2: (f32, f32), p3: (f32, f32), out: &mut Vec<(f32, f32)>) {
    const STEPS: i32 = 24;
    if out.is_empty() {
        out.push(p0);
    }
    for i in 1..=STEPS {
        let t = i as f32 / STEPS as f32;
        let u = 1.0 - t;
        let a = u * u * u;
        let b = 3.0 * u * u * t;
        let c = 3.0 * u * t * t;
        let d = t * t * t;
        let x = a * p0.0 + b * p1.0 + c * p2.0 + d * p3.0;
        let y = a * p0.1 + b * p1.1 + c * p2.1 + d * p3.1;
        if out.last().is_none_or(|q| *q != (x, y)) {
            out.push((x, y));
        }
    }
}

fn in_ellipse(px: f64, py: f64, x: f64, y: f64, w: f64, h: f64) -> bool {
    if w <= 0.0 || h <= 0.0 {
        return false;
    }
    let cx = x + w * 0.5;
    let cy = y + h * 0.5;
    let rx = w * 0.5;
    let ry = h * 0.5;
    let dx = (px - cx) / rx;
    let dy = (py - cy) / ry;
    dx * dx + dy * dy <= 1.0
}

fn in_round_rect(px: f64, py: f64, x: f64, y: f64, w: f64, h: f64, r: f64) -> bool {
    if px < x || py < y || px > x + w || py > y + h {
        return false;
    }
    if r <= 0.0 {
        return true;
    }
    let r = r.min(w * 0.5).min(h * 0.5);
    let cx = if px < x + r {
        x + r
    } else if px > x + w - r {
        x + w - r
    } else {
        return true;
    };
    let cy = if py < y + r {
        y + r
    } else if py > y + h - r {
        y + h - r
    } else {
        return true;
    };
    let dx = px - cx;
    let dy = py - cy;
    dx * dx + dy * dy <= r * r
}

pub fn fill_device_ellipse(
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    blend: impl FnMut(i32, i32, u8),
) -> bool {
    if w <= 0 || h <= 0 {
        return false;
    }
    let pad = 2;
    let fx = f64::from(x);
    let fy = f64::from(y);
    let fw = f64::from(w);
    let fh = f64::from(h);
    emit_coverage(
        x.saturating_sub(pad),
        y.saturating_sub(pad),
        x.saturating_add(w).saturating_add(pad),
        y.saturating_add(h).saturating_add(pad),
        |px, py| in_ellipse(px, py, fx, fy, fw, fh),
        blend,
    )
}

pub fn fill_device_round_rect(
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    radius: f64,
    mut blend: impl FnMut(i32, i32, u8),
) -> bool {
    if w <= 0 || h <= 0 {
        return false;
    }
    let pad = 2;
    let fx = f64::from(x);
    let fy = f64::from(y);
    let fw = f64::from(w);
    let fh = f64::from(h);
    let max_r = f64::from(w.min(h)) / 2.0;
    let r = radius.clamp(0.0, max_r);
    let corner = r.floor() as i32;
    emit_coverage(
        x.saturating_sub(pad),
        y.saturating_sub(pad),
        x.saturating_add(w).saturating_add(pad),
        y.saturating_add(h).saturating_add(pad),
        |px, py| in_round_rect(px, py, fx, fy, fw, fh, r),
        |wx, wy, cov| {
            let in_box = wx >= x && wx < x + w && wy >= y && wy < y + h;
            let in_corner = corner > 0
                && in_box
                && ((wx < x + corner && wy < y + corner)
                    || (wx >= x + w - corner && wy < y + corner)
                    || (wx < x + corner && wy >= y + h - corner)
                    || (wx >= x + w - corner && wy >= y + h - corner));
            let cov = if in_box && !in_corner { 255 } else { cov };
            if cov != 0 {
                blend(wx, wy, cov);
            }
        },
    )
}

pub fn fill_device_opaque_round_rect(
    rgb: &mut [u8],
    page_w: u32,
    page_h: u32,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    radius: f64,
    color: [u8; 3],
) -> bool {
    if !page_ok(rgb, page_w, page_h) || w <= 0 || h <= 0 {
        return false;
    }
    fill_device_round_rect(x, y, w, h, radius, |px, py, cov| {
        let dst = page_pixel(rgb, page_w, page_h, px, py);
        set_page_pixel(rgb, page_w, page_h, px, py, over(dst, color, cov));
    })
}

pub fn fill_device_line(
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    width_px: i32,
    blend: impl FnMut(i32, i32, u8),
) -> bool {
    stroke_points(&[(x0 as f32, y0 as f32), (x1 as f32, y1 as f32)], width_px, blend)
}

pub fn fill_device_polyline(
    points: &[(f32, f32)],
    width_px: i32,
    blend: impl FnMut(i32, i32, u8),
) -> bool {
    stroke_points(points, width_px, blend)
}

pub fn fill_device_beziers(
    points: &[(f32, f32)],
    width_px: i32,
    blend: impl FnMut(i32, i32, u8),
) -> bool {
    if width_px < 1 || points.len() < 4 || !(points.len() - 1).is_multiple_of(3) {
        return false;
    }
    if points.iter().any(|p| !p.0.is_finite() || !p.1.is_finite()) {
        return false;
    }
    let mut flat = Vec::new();
    let mut i = 0;
    while i + 3 < points.len() {
        flatten_cubic(points[i], points[i + 1], points[i + 2], points[i + 3], &mut flat);
        i += 3;
    }
    stroke_points(&flat, width_px, blend)
}

pub fn fill_device_stroked_rect(
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    fill: Option<[u8; 3]>,
    stroke: [u8; 3],
    width_px: i32,
    rgb: &mut [u8],
    page_w: u32,
    page_h: u32,
) -> bool {
    if !page_ok(rgb, page_w, page_h) || w <= 0 || h <= 0 || width_px < 1 {
        return false;
    }
    if let Some(fill) = fill {
        let pad = 1;
        let fx = f64::from(x);
        let fy = f64::from(y);
        let fw = f64::from(w);
        let fh = f64::from(h);
        let _ = emit_coverage(
            x.saturating_sub(pad),
            y.saturating_sub(pad),
            x.saturating_add(w).saturating_add(pad),
            y.saturating_add(h).saturating_add(pad),
            |px, py| px >= fx && px < fx + fw && py >= fy && py < fy + fh,
            |px, py, cov| {
                let dst = page_pixel(rgb, page_w, page_h, px, py);
                set_page_pixel(rgb, page_w, page_h, px, py, over(dst, fill, cov));
            },
        );
    }
    let pts = [
        (x as f32, y as f32),
        ((x + w) as f32, y as f32),
        ((x + w) as f32, (y + h) as f32),
        (x as f32, (y + h) as f32),
        (x as f32, y as f32),
    ];
    stroke_points(&pts, width_px, |px, py, cov| {
        let dst = page_pixel(rgb, page_w, page_h, px, py);
        set_page_pixel(rgb, page_w, page_h, px, py, over(dst, stroke, cov));
    })
}

pub fn fill_device_rotated_rect(
    rgb: &mut [u8],
    page_w: u32,
    page_h: u32,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    deg: f32,
    color: [u8; 3],
) -> bool {
    if !page_ok(rgb, page_w, page_h)
        || !x.is_finite()
        || !y.is_finite()
        || !w.is_finite()
        || !h.is_finite()
        || !deg.is_finite()
        || w <= 0.0
        || h <= 0.0
    {
        return false;
    }
    let (min_x, min_y, max_x, max_y) = rotated_rect_bounds(x, y, w, h, deg);
    let pad = 4.0;
    let origin_x = (min_x - pad).floor() as i32;
    let origin_y = (min_y - pad).floor() as i32;
    let far_x = (max_x + pad).ceil() as i32;
    let far_y = (max_y + pad).ceil() as i32;
    let cx = f64::from(x + w / 2.0);
    let cy = f64::from(y + h / 2.0);
    let rad = f64::from(deg).to_radians();
    let (c, s) = (rad.cos(), rad.sin());
    let fx = f64::from(x);
    let fy = f64::from(y);
    let fw = f64::from(w);
    let fh = f64::from(h);
    emit_coverage(
        origin_x,
        origin_y,
        far_x,
        far_y,
        |px, py| {
            let dx = px - cx;
            let dy = py - cy;
            // 与 GDI+ 路径同一套顺时针旋转的逆变换。
            let qx = cx + dx * c - dy * s;
            let qy = cy + dx * s + dy * c;
            qx >= fx && qx < fx + fw && qy >= fy && qy < fy + fh
        },
        |px, py, cov| {
            let dst = page_pixel(rgb, page_w, page_h, px, py);
            set_page_pixel(rgb, page_w, page_h, px, py, over(dst, color, cov));
        },
    )
}

fn rotated_rect_bounds(x: f32, y: f32, w: f32, h: f32, deg: f32) -> (f32, f32, f32, f32) {
    let cx = x + w / 2.0;
    let cy = y + h / 2.0;
    let rad = (deg as f64).to_radians();
    let (c, s) = (rad.cos() as f32, rad.sin() as f32);
    let corners = [(x, y), (x + w, y), (x + w, y + h), (x, y + h)];
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    for (px, py) in corners {
        let dx = px - cx;
        let dy = py - cy;
        let nx = cx + dx * c + dy * s;
        let ny = cy - dx * s + dy * c;
        min_x = min_x.min(nx);
        min_y = min_y.min(ny);
        max_x = max_x.max(nx);
        max_y = max_y.max(ny);
    }
    (min_x, min_y, max_x, max_y)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn collect(blend_fn: impl FnOnce(&mut dyn FnMut(i32, i32, u8))) -> Vec<(i32, i32, u8)> {
        let mut out = Vec::new();
        let mut blend = |x, y, cov| out.push((x, y, cov));
        blend_fn(&mut blend);
        out
    }

    #[test]
    fn software_ellipse_covers_center_and_leaves_corners() {
        let hits = collect(|b| {
            assert!(fill_device_ellipse(10, 20, 40, 20, |x, y, cov| b(x, y, cov)));
        });
        assert!(hits.iter().any(|&(x, y, cov)| {
            x == 30 && y == 30 && cov >= 200
        }));
        assert!(!hits.iter().any(|&(x, y, _)| x == 10 && y == 20));
        assert!(hits.len() > 80);
    }

    #[test]
    fn software_round_rect_fills_body() {
        let hits = collect(|b| {
            assert!(fill_device_round_rect(4, 4, 30, 20, 6.0, |x, y, cov| b(x, y, cov)));
        });
        assert!(hits.iter().any(|&(x, y, cov)| x == 19 && y == 14 && cov == 255));
        assert!(hits.len() > 100);
    }

    #[test]
    fn software_stroked_rect_paints_fill_and_stroke() {
        let mut rgb = vec![255u8; 80 * 80 * 3];
        assert!(fill_device_stroked_rect(
            10,
            10,
            40,
            30,
            Some([0, 80, 0]),
            [200, 0, 0],
            2,
            &mut rgb,
            80,
            80,
        ));
        let mid = ((25 * 80 + 30) * 3) as usize;
        assert!(rgb[mid + 1] < 120, "fill green should appear at center");
        let mut edge = false;
        for x in 8..14 {
            let i = (10 * 80 + x) * 3;
            if rgb[i] > 120 && rgb[i + 1] < 80 {
                edge = true;
            }
        }
        assert!(edge, "stroke red should appear on the top edge");
    }

    #[test]
    fn software_rotated_rect_is_not_axis_aligned() {
        let mut rgb = vec![255u8; 80 * 80 * 3];
        assert!(fill_device_rotated_rect(
            &mut rgb, 80, 80, 25.0, 25.0, 30.0, 12.0, 35.0, [0, 0, 180],
        ));
        let mut ink = 0usize;
        let mut outside_aabb = 0usize;
        for y in 0..80i32 {
            for x in 0..80i32 {
                let i = (y as usize * 80 + x as usize) * 3;
                if rgb[i + 2] < 200 {
                    ink += 1;
                    if x < 25 || x >= 55 || y < 25 || y >= 37 {
                        outside_aabb += 1;
                    }
                }
            }
        }
        assert!(ink > 80, "rotated rect should paint, ink={ink}");
        assert!(
            outside_aabb > 10,
            "rotation must spill outside the unrotated box, outside={outside_aabb}"
        );
        assert!(!fill_device_rotated_rect(
            &mut rgb, 80, 80, 25.0, 25.0, 0.0, 12.0, 35.0, [0, 0, 180],
        ));
    }

    #[test]
    fn software_line_and_bezier_leave_coverage() {
        let line = collect(|b| {
            assert!(fill_device_line(2, 2, 40, 20, 2, |x, y, cov| b(x, y, cov)));
        });
        assert!(line.len() > 20);
        let bez = collect(|b| {
            assert!(fill_device_beziers(
                &[(4.0, 30.0), (16.0, 4.0), (28.0, 4.0), (40.0, 30.0)],
                2,
                |x, y, cov| b(x, y, cov),
            ));
        });
        assert!(bez.len() > 20);
        assert!(!fill_device_beziers(&[(0.0, 0.0)], 2, |_, _, _| {}));
        assert!(!fill_device_ellipse(0, 0, 0, 10, |_, _, _| {}));
    }

    #[test]
    fn software_polyline_and_opaque_round_rect_paint() {
        let line = collect(|b| {
            assert!(fill_device_polyline(
                &[(4.0, 4.0), (30.0, 8.0), (40.0, 28.0)],
                2,
                |x, y, cov| b(x, y, cov),
            ));
        });
        assert!(line.len() > 20);
        let mut rgb = vec![255u8; 60 * 60 * 3];
        assert!(fill_device_opaque_round_rect(
            &mut rgb, 60, 60, 8, 8, 30, 24, 6.0, [20, 40, 180],
        ));
        let mid = ((20 * 60 + 22) * 3) as usize;
        assert!(rgb[mid + 2] < 200, "opaque round rect should paint blue");
    }
}
