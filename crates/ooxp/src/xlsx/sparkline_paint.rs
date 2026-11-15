//! 折线迷你图和柱形迷你图。几何按 Excel 16.0 空单元格量出来的位置。
//! 折线左右各留 2 像素，点落在等分格的正中。纵轴上沿 5 像素，下沿是高度减 4。
//! 柱形槽从第 3 列起、右端留 2，柱占槽的三分之二。柱是纯色，没有抗锯齿。

use crate::error::{Error, ErrorKind};

const PAD_X: f64 = 2.0;
const TOP: f64 = 5.0;
const BOTTOM: f64 = 4.0;
const MAX_PX: u32 = 4096;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum EmptyCells {
    Gap,
    Span,
    Zero,
}

pub(crate) struct LineSpec {
    pub values: Vec<Option<f64>>,
    pub empty: EmptyCells,
    pub axis_min: Option<f64>,
    pub axis_max: Option<f64>,
    pub color: [u8; 3],
    pub weight_px: f64,
}

pub(crate) fn render(width: u32, height: u32, spec: &LineSpec) -> Result<Vec<u8>, Error> {
    if !(1..=MAX_PX).contains(&width) || !(1..=MAX_PX).contains(&height) {
        return Err(unsupported());
    }
    if !(spec.weight_px.is_finite() && spec.weight_px > 0.0) {
        return Err(unsupported());
    }
    let mut px = vec![255u8; (width as usize) * (height as usize) * 3];
    let n = spec.values.len();
    if n >= 2 {
        let plotted = plotted(spec);
        if !plotted.is_empty() {
            let (lo, hi) = axis_ends(spec, &plotted)?;
            let runs = runs(spec.empty, &plotted);
            let mut cover = vec![0f32; (width as usize) * (height as usize)];
            for run in runs {
                let mut prev: Option<(f64, f64)> = None;
                for (index, value) in run {
                    let here = point_xy(index, n, value, lo, hi, width, height);
                    if let Some(from) = prev {
                        stamp(&mut cover, width, height, from, here, spec.weight_px);
                    }
                    prev = Some(here);
                }
            }
            for (i, cov) in cover.iter().enumerate() {
                if *cov <= 0.0 {
                    continue;
                }
                let x = (i % width as usize) as u32;
                let y = (i / width as usize) as u32;
                blend(&mut px, width, x, y, f64::from(*cov), spec.color);
            }
        }
    }
    encode_png(width, height, &px)
}

/// 非负柱形。宽至少 6、高至少 9，否则上下左右的留白叠在一起。
pub(crate) fn render_columns(
    width: u32,
    height: u32,
    values: &[f64],
    colors: &[[u8; 3]],
) -> Result<Vec<u8>, Error> {
    if !(6..=MAX_PX).contains(&width) || !(9..=MAX_PX).contains(&height) {
        return Err(unsupported());
    }
    if values.is_empty()
        || colors.len() != values.len()
        || values.iter().any(|n| !n.is_finite() || *n < 0.0)
    {
        return Err(unsupported());
    }
    let mut px = vec![255u8; (width as usize) * (height as usize) * 3];
    let lo = values.iter().copied().fold(f64::INFINITY, f64::min);
    let hi = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let plot_h = height as i32 - 8;
    let bottom = height as i32 - 4;
    for (index, value) in values.iter().enumerate() {
        let bar = bar_px(*value, lo, hi, plot_h);
        if bar <= 0 {
            continue;
        }
        let Some((x0, x1)) = column_x(width, values.len(), index) else {
            return Err(unsupported());
        };
        if x1 < x0 {
            continue;
        }
        let y0 = bottom - bar + 1;
        for y in y0..=bottom {
            for x in x0..=x1 {
                if x >= 0 && y >= 0 && (x as u32) < width && (y as u32) < height {
                    put(&mut px, width, x as u32, y as u32, colors[index]);
                }
            }
        }
    }
    encode_png(width, height, &px)
}

/// 槽宽是 `(宽 - 5) / 点数`。第 i 根柱的左右缘是 `3 + (i + 1/6) * 槽宽` 与 `3 + (i + 5/6) * 槽宽`。
/// 左缘取 floor，右缘是不含端点的 ceil 再减 1。整数除法与这个取整相同。
fn column_x(width: u32, count: usize, index: usize) -> Option<(i32, i32)> {
    let n = i64::try_from(count).ok()?;
    let i = i64::try_from(index).ok()?;
    let span = i64::from(width) - 5;
    if n <= 0 || span <= 0 {
        return None;
    }
    let den = 6_i64.checked_mul(n)?;
    let x0 = 3 + (6 * i + 1).checked_mul(span)? / den;
    let k = (6 * i + 5).checked_mul(span)?;
    let q = k / den;
    let x1 = 3 + if k % den == 0 { q - 1 } else { q };
    Some((i32::try_from(x0).ok()?, i32::try_from(x1).ok()?))
}

fn bar_px(value: f64, lo: f64, hi: f64, plot_h: i32) -> i32 {
    if hi == lo {
        return if hi > 0.0 { plot_h } else { 0 };
    }
    if value == 0.0 && lo == 0.0 {
        return 0;
    }
    let t = (value - lo) / (hi - lo);
    let h = (t * f64::from(plot_h - 1)).floor() as i32 + 1;
    h.clamp(0, plot_h)
}

fn put(px: &mut [u8], width: u32, x: u32, y: u32, color: [u8; 3]) {
    let i = ((y as usize) * (width as usize) + x as usize) * 3;
    px[i..i + 3].copy_from_slice(&color);
}

fn plotted(spec: &LineSpec) -> Vec<(usize, f64)> {
    let mut out = Vec::new();
    for (i, value) in spec.values.iter().enumerate() {
        match (value, spec.empty) {
            (Some(n), _) if n.is_finite() => out.push((i, *n)),
            (None, EmptyCells::Zero) => out.push((i, 0.0)),
            (Some(_), _) => {}
            (None, EmptyCells::Gap | EmptyCells::Span) => {}
        }
    }
    out
}

fn axis_ends(spec: &LineSpec, plotted: &[(usize, f64)]) -> Result<(f64, f64), Error> {
    let data_min = plotted
        .iter()
        .map(|(_, v)| *v)
        .fold(f64::INFINITY, f64::min);
    let data_max = plotted
        .iter()
        .map(|(_, v)| *v)
        .fold(f64::NEG_INFINITY, f64::max);
    let lo = spec.axis_min.unwrap_or(data_min);
    let hi = spec.axis_max.unwrap_or(data_max);
    if !lo.is_finite() || !hi.is_finite() || lo > hi {
        return Err(unsupported());
    }
    Ok((lo, hi))
}

fn runs(empty: EmptyCells, plotted: &[(usize, f64)]) -> Vec<Vec<(usize, f64)>> {
    if plotted.len() < 2 {
        return Vec::new();
    }
    if empty != EmptyCells::Gap {
        return vec![plotted.to_vec()];
    }
    let mut runs = Vec::new();
    let mut cur = Vec::new();
    let mut prev: Option<usize> = None;
    for (index, value) in plotted {
        if prev.is_some_and(|p| *index > p + 1) {
            if cur.len() >= 2 {
                runs.push(std::mem::take(&mut cur));
            } else {
                cur.clear();
            }
        }
        cur.push((*index, *value));
        prev = Some(*index);
    }
    if cur.len() >= 2 {
        runs.push(cur);
    }
    runs
}

/// 公式给出的是像素序号。画的时候落到该像素中心。
fn point_xy(
    index: usize,
    n: usize,
    value: f64,
    lo: f64,
    hi: f64,
    width: u32,
    height: u32,
) -> (f64, f64) {
    let w = f64::from(width);
    let h = f64::from(height);
    let x = (PAD_X + (index as f64 + 0.5) * (w - PAD_X * 2.0) / n as f64).round();
    let t = if (hi - lo).abs() < 1e-9 {
        1.0
    } else {
        ((value - lo) / (hi - lo)).clamp(0.0, 1.0)
    };
    let y = (TOP + (1.0 - t) * (h - TOP - BOTTOM)).round();
    (x + 0.5, y + 0.5)
}

fn stamp(
    cover: &mut [f32],
    width: u32,
    height: u32,
    from: (f64, f64),
    to: (f64, f64),
    weight: f64,
) {
    let half = weight / 2.0;
    let pad = half + 1.0;
    let min_x = (from.0.min(to.0) - pad).floor().max(0.0) as i32;
    let max_x = (from.0.max(to.0) + pad).ceil().min(f64::from(width) - 1.0) as i32;
    let min_y = (from.1.min(to.1) - pad).floor().max(0.0) as i32;
    let max_y = (from.1.max(to.1) + pad).ceil().min(f64::from(height) - 1.0) as i32;
    if max_x < min_x || max_y < min_y {
        return;
    }
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let dist = dist_to_seg(f64::from(x) + 0.5, f64::from(y) + 0.5, from, to);
            // 1 像素宽的线，中心覆盖大约 0.75。Excel 16.0 的最暗像素不是纯色。
            let cov = (half + 0.25 - dist).clamp(0.0, 1.0) as f32;
            if cov <= 0.0 {
                continue;
            }
            let i = (y as usize) * (width as usize) + x as usize;
            if cov > cover[i] {
                cover[i] = cov;
            }
        }
    }
}

fn dist_to_seg(px: f64, py: f64, from: (f64, f64), to: (f64, f64)) -> f64 {
    let dx = to.0 - from.0;
    let dy = to.1 - from.1;
    let len2 = dx * dx + dy * dy;
    if len2 < 1e-12 {
        return (px - from.0).hypot(py - from.1);
    }
    let t = (((px - from.0) * dx + (py - from.1) * dy) / len2).clamp(0.0, 1.0);
    (px - (from.0 + t * dx)).hypot(py - (from.1 + t * dy))
}

fn blend(px: &mut [u8], width: u32, x: u32, y: u32, cov: f64, color: [u8; 3]) {
    let i = ((y as usize) * (width as usize) + x as usize) * 3;
    for (channel, src) in color.iter().enumerate() {
        let dst = f64::from(px[i + channel]);
        px[i + channel] = (dst + (f64::from(*src) - dst) * cov).round() as u8;
    }
}

fn encode_png(w: u32, h: u32, rgb: &[u8]) -> Result<Vec<u8>, Error> {
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, w, h);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(|_| unsupported())?;
    writer.write_image_data(rgb).map_err(|_| unsupported())?;
    drop(writer);
    Ok(out)
}

fn unsupported() -> Error {
    Error::with_path(ErrorKind::SparklineImageUnsupported, "id")
}
