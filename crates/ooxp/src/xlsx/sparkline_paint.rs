//! 折线迷你图、柱形迷你图和盈亏（`type=stacked`）迷你图。几何按 Excel 16.0 空单元格量出来的位置。
//! 折线左右各留 2 像素，点落在等分格的正中。纵轴上沿 5 像素，下沿是高度减 4。
//! 柱形槽从第 3 列起、右端留 2，柱占槽的三分之二。柱是纯色，没有抗锯齿。
//! 盈亏复用同一套横向 `column_x`；正半区上沿→中线上一像素，负半区中线下一像素→下沿；0/空不画。
//! 日期轴：用组上 `xm:f` 的数值序列（Excel 日期序列）把点放到时间比例位置，而不是按下标等距。
//! 等距日期与原分类轴同一套公式，使连续等差日期与关闭日期轴的像素一致；不等距日期按 (date-min)/(max-min)*(n-1) 映射到该公式的槽坐标。不排序、不把缺日期或非数值日期当成等距轴。

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
    /// 与 `values` 等长的有限日期序列。`None` 表示分类轴（按下标等距）。
    pub dates: Option<Vec<f64>>,
}

/// Unflattened host-cell raster. Keep the original f32 coverage so sheet fills
/// and tile windows use exactly the standalone painter's sampling/rounding.
pub(crate) struct Raster {
    pub width: u32,
    pub height: u32,
    rgb: Vec<u8>,
    coverage: Vec<f32>,
}
impl Raster {
    fn new(width: u32, height: u32) -> Self {
        let n = width as usize * height as usize;
        Self {
            width,
            height,
            rgb: vec![255; n * 3],
            coverage: vec![0.0; n],
        }
    }
    pub fn composite_pixel(&self, x: u32, y: u32, dst: [u8; 3]) -> [u8; 3] {
        let i = (y * self.width + x) as usize;
        let cov = f64::from(self.coverage[i]);
        let mut out = dst;
        for (k, channel) in out.iter_mut().enumerate() {
            let base = f64::from(*channel);
            *channel = (base + (f64::from(self.rgb[i * 3 + k]) - base) * cov).round() as u8;
        }
        out
    }
    pub fn png(&self) -> Result<Vec<u8>, Error> {
        let mut rgb = Vec::with_capacity(self.width as usize * self.height as usize * 3);
        for y in 0..self.height {
            for x in 0..self.width {
                rgb.extend_from_slice(&self.composite_pixel(x, y, [255; 3]));
            }
        }
        encode_png(self.width, self.height, &rgb)
    }
}

pub(crate) fn raster_line(width: u32, height: u32, spec: &LineSpec) -> Result<Raster, Error> {
    if !(1..=MAX_PX).contains(&width) || !(1..=MAX_PX).contains(&height) {
        return Err(unsupported());
    }
    if !(spec.weight_px.is_finite() && spec.weight_px > 0.0) {
        return Err(unsupported());
    }
    let mut px = Raster::new(width, height);
    let n = spec.values.len();
    if let Some(dates) = spec.dates.as_ref() {
        if dates.len() != n {
            return Err(unsupported());
        }
    }
    if n >= 2 {
        let plotted = plotted(spec);
        if !plotted.is_empty() {
            let (lo, hi) = axis_ends(spec, &plotted)?;
            let runs = runs(spec.empty, &plotted);
            let mut cover = vec![0f32; (width as usize) * (height as usize)];
            for run in runs {
                let mut prev: Option<(f64, f64)> = None;
                for (index, value) in run {
                    let u = slot_of(index, n, spec.dates.as_deref())?;
                    let here = point_xy(u, n, value, lo, hi, width, height);
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
    Ok(px)
}

/// 非负柱形。宽至少 6、高至少 9，否则上下左右的留白叠在一起。
pub(crate) fn raster_columns(
    width: u32,
    height: u32,
    values: &[f64],
    colors: &[[u8; 3]],
    dates: Option<&[f64]>,
) -> Result<Raster, Error> {
    if !(6..=MAX_PX).contains(&width) || !(9..=MAX_PX).contains(&height) {
        return Err(unsupported());
    }
    if values.is_empty()
        || colors.len() != values.len()
        || values.iter().any(|n| !n.is_finite() || *n < 0.0)
    {
        return Err(unsupported());
    }
    let mut px = Raster::new(width, height);
    let lo = values.iter().copied().fold(f64::INFINITY, f64::min);
    let hi = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let plot_h = height as i32 - 8;
    let bottom = height as i32 - 4;
    for (index, value) in values.iter().enumerate() {
        let bar = bar_px(*value, lo, hi, plot_h);
        if bar <= 0 {
            continue;
        }
        let u = slot_of(index, values.len(), dates)?;
        let Some((x0, x1)) = column_x_at(width, values.len(), u) else {
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
    Ok(px)
}

/// 盈亏（SpreadsheetML `type=stacked`）。横向与柱形同一套 `column_x`。
/// 纵轴上沿 5、下沿 `height-4`（含）；绘图高 `height-8` 对半：正值铺上半、负值铺下半；`0` 与空槽不画。
/// 颜色参数来自调用方（OOXML `colorSeries`/`colorNegative` 解析结果）；本函数不写死样本色。
/// 缺元素时的缺省 rgb（`376092`/`D00000`）只在 `sparkline.rs` 解析侧对齐 Excel 新建写入；
/// 有元素但无 `rgb`（theme/tint）→ 上游 NI。上沿/下沿是 CopyPicture 几何常数，不是单样本硬编码色。
/// 证据：Excel 16.0 CopyPicture（2026-11-17 Asia/Shanghai）G2 `+/-` 缺省色实心与 20px 行 5..=10/11..=16；
/// 全 0 无柱；另有自定义色/高宽单元。
pub(crate) fn raster_win_loss(
    width: u32,
    height: u32,
    values: &[Option<f64>],
    series: [u8; 3],
    negative: [u8; 3],
    color_negatives: bool,
    dates: Option<&[f64]>,
) -> Result<Raster, Error> {
    if !(6..=MAX_PX).contains(&width) || !(9..=MAX_PX).contains(&height) {
        return Err(unsupported());
    }
    if values.is_empty() {
        return Err(unsupported());
    }
    let mut px = Raster::new(width, height);
    let top = 5_i32;
    let bottom = height as i32 - 4;
    let plot_h = bottom - top + 1;
    if plot_h < 2 {
        return Err(unsupported());
    }
    let half = plot_h / 2;
    let pos_y0 = top;
    let pos_y1 = top + half - 1;
    let neg_y0 = top + half;
    let neg_y1 = bottom;
    for (index, value) in values.iter().enumerate() {
        let Some(n) = value else {
            continue;
        };
        if !n.is_finite() || *n == 0.0 {
            continue;
        }
        let u = slot_of(index, values.len(), dates)?;
        let Some((x0, x1)) = column_x_at(width, values.len(), u) else {
            return Err(unsupported());
        };
        if x1 < x0 {
            continue;
        }
        let (y0, y1, color) = if *n > 0.0 {
            (pos_y0, pos_y1, series)
        } else if color_negatives {
            (neg_y0, neg_y1, negative)
        } else {
            (neg_y0, neg_y1, series)
        };
        for y in y0..=y1 {
            for x in x0..=x1 {
                if x >= 0 && y >= 0 && (x as u32) < width && (y as u32) < height {
                    put(&mut px, width, x as u32, y as u32, color);
                }
            }
        }
    }
    Ok(px)
}

/// 槽宽是 `(宽 - 5) / 点数`。 `(宽 - 5) / 点数`。第 i 根柱的左右缘是 `3 + (i + 1/6) * 槽宽` 与 `3 + (i + 5/6) * 槽宽`。
/// 左缘取 floor，右缘是不含端点的 ceil 再减 1。整数除法与这个取整相同。
#[cfg(test)]
fn column_x(width: u32, count: usize, index: usize) -> Option<(i32, i32)> {
    column_x_at(width, count, index as f64)
}

fn column_x_at(width: u32, count: usize, u: f64) -> Option<(i32, i32)> {
    if !u.is_finite() || count == 0 {
        return None;
    }
    let n = i64::try_from(count).ok()?;
    let span = i64::from(width) - 5;
    if n <= 0 || span <= 0 {
        return None;
    }
    let den = 6.0 * n as f64;
    let start = (6.0 * u + 1.0) * span as f64 / den;
    let end = (6.0 * u + 5.0) * span as f64 / den;
    if !start.is_finite() || !end.is_finite() {
        return None;
    }
    let x0 = 3 + start.floor() as i64;
    let x1 = 3 + end.ceil() as i64 - 1;
    Some((i32::try_from(x0).ok()?, i32::try_from(x1).ok()?))
}

/// 日期轴把序列映射到分类槽坐标：等距日期时 `slot == index`，与关闭日期轴一致。
/// `n == 1` 时落在唯一槽；`n >= 2` 且最大最小日期相同则无法定比例，报错。
pub(crate) fn date_slot(dates: &[f64], index: usize) -> Result<f64, Error> {
    let n = dates.len();
    let date = *dates.get(index).ok_or_else(unsupported)?;
    if !date.is_finite() {
        return Err(unsupported());
    }
    if n == 1 {
        return Ok(0.0);
    }
    let min = dates.iter().copied().fold(f64::INFINITY, f64::min);
    let max = dates.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if !min.is_finite() || !max.is_finite() || (max - min).abs() < 1e-12 {
        return Err(unsupported());
    }
    Ok((date - min) / (max - min) * (n as f64 - 1.0))
}

fn slot_of(index: usize, n: usize, dates: Option<&[f64]>) -> Result<f64, Error> {
    match dates {
        None => {
            if index < n {
                Ok(index as f64)
            } else {
                Err(unsupported())
            }
        }
        Some(dates) => {
            if dates.len() != n {
                return Err(unsupported());
            }
            date_slot(dates, index)
        }
    }
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

fn put(px: &mut Raster, width: u32, x: u32, y: u32, color: [u8; 3]) {
    let i = (y * width + x) as usize;
    px.rgb[i * 3..i * 3 + 3].copy_from_slice(&color);
    px.coverage[i] = 1.0;
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

/// 公式给出的是像素序号。画的时候落到该像素中心。`u` 是分类下标或日期映射后的槽坐标。
fn point_xy(u: f64, n: usize, value: f64, lo: f64, hi: f64, width: u32, height: u32) -> (f64, f64) {
    let w = f64::from(width);
    let h = f64::from(height);
    let x = (PAD_X + (u + 0.5) * (w - PAD_X * 2.0) / n as f64).round();
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
            let cov = normal_box_coverage(dist, half);
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

/// Unit-width box filter along the stroke normal. Intersect the stroke interval
/// [-radius, radius] with the pixel interval [distance-0.5, distance+0.5].
/// Their overlap has length clamp(radius+0.5-distance, 0, min(1, 2*radius)).
/// Thus a centered one-pixel strip is opaque, and a subpixel strip conserves
/// its width instead of gaining opacity. For diagonal edges / circular caps
/// this is a distance-filter approximation, not exact square-pixel area.
/// Segment coverages are unioned with max by stamp, never repeatedly blended.
fn normal_box_coverage(distance: f64, radius: f64) -> f32 {
    (radius + 0.5 - distance).clamp(0.0, (2.0 * radius).min(1.0)) as f32
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

fn blend(px: &mut Raster, width: u32, x: u32, y: u32, cov: f64, color: [u8; 3]) {
    let i = (y * width + x) as usize;
    px.rgb[i * 3..i * 3 + 3].copy_from_slice(&color);
    px.coverage[i] = cov as f32;
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

#[cfg(test)]
fn render(w: u32, h: u32, spec: &LineSpec) -> Result<Vec<u8>, Error> {
    raster_line(w, h, spec)?.png()
}
#[cfg(test)]
fn render_columns(w: u32, h: u32, values: &[f64], colors: &[[u8; 3]]) -> Result<Vec<u8>, Error> {
    raster_columns(w, h, values, colors, None)?.png()
}
#[cfg(test)]
fn render_win_loss(
    w: u32,
    h: u32,
    values: &[Option<f64>],
    series: [u8; 3],
    negative: [u8; 3],
    flags: bool,
) -> Result<Vec<u8>, Error> {
    raster_win_loss(w, h, values, series, negative, flags, None)?.png()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode_rgb(png: &[u8]) -> (u32, u32, Vec<u8>) {
        let decoder = png::Decoder::new(std::io::Cursor::new(png));
        let mut reader = decoder.read_info().unwrap();
        let mut buf = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).unwrap();
        buf.truncate(info.buffer_size());
        // expand to rgb8 if needed
        (info.width, info.height, buf)
    }

    #[test]
    fn normal_box_filter_conserves_thin_and_thick_axis_aligned_widths() {
        for weight in [0.25, 0.5, 1.0, 1.5, 2.0, 4.0] {
            for phase in [0.0, 0.25, 0.5, 0.75] {
                let mass: f64 = (-12..=12)
                    .map(|row| {
                        f64::from(normal_box_coverage(
                            (f64::from(row) - phase).abs(),
                            weight / 2.0,
                        ))
                    })
                    .sum();
                assert!(
                    (mass - weight).abs() < 1e-6,
                    "weight={weight}, phase={phase}, mass={mass}"
                );
            }
        }
        assert_eq!(normal_box_coverage(0.0, 0.5), 1.0);
        assert_eq!(normal_box_coverage(0.0, 0.125), 0.25);
        assert_eq!(normal_box_coverage(3.0, 0.5), 0.0);
    }

    #[test]
    fn default_flat_line_has_source_color_and_composites_white_on_color() {
        let mut spec = LineSpec {
            values: vec![Some(2.0); 3],
            empty: EmptyCells::Gap,
            axis_min: None,
            axis_max: None,
            color: [55, 96, 146],
            weight_px: 1.0,
            dates: None,
        };
        let raster = raster_line(72, 20, &spec).unwrap();
        assert_eq!(raster.composite_pixel(36, 5, [255; 3]), spec.color);
        assert_eq!(raster.composite_pixel(0, 0, [40, 80, 120]), [40, 80, 120]);
        spec.color = [255; 3];
        let white = raster_line(72, 20, &spec).unwrap();
        assert_eq!(white.composite_pixel(36, 5, [40, 80, 120]), [255; 3]);
        assert_eq!(white.composite_pixel(0, 0, [40, 80, 120]), [40, 80, 120]);
    }

    #[test]
    fn stroke_coverage_is_axis_symmetric_and_direction_independent() {
        for weight in [0.5, 1.0, 2.0, 4.0] {
            let mut horizontal = vec![0.0; 32 * 32];
            let mut vertical = horizontal.clone();
            let mut reversed = horizontal.clone();
            stamp(&mut horizontal, 32, 32, (8.5, 16.5), (24.5, 16.5), weight);
            stamp(&mut vertical, 32, 32, (16.5, 8.5), (16.5, 24.5), weight);
            stamp(&mut reversed, 32, 32, (24.5, 16.5), (8.5, 16.5), weight);
            assert_eq!(horizontal, reversed);
            for y in 0..32 {
                for x in 0..32 {
                    assert_eq!(horizontal[y * 32 + x], vertical[x * 32 + y]);
                }
            }
            let mut diagonal = vec![0.0; 32 * 32];
            let mut back = diagonal.clone();
            stamp(&mut diagonal, 32, 32, (8.5, 8.5), (24.5, 24.5), weight);
            stamp(&mut back, 32, 32, (24.5, 24.5), (8.5, 8.5), weight);
            assert_eq!(diagonal, back);
            assert!(diagonal.iter().all(|v| (0.0..=1.0).contains(v)));
        }
    }

    #[test]
    fn overlapping_segments_union_coverage_once_at_bends_and_repeated_paths() {
        let mut cover = vec![0.0; 32 * 32];
        stamp(&mut cover, 32, 32, (5.5, 20.5), (16.5, 5.5), 1.0);
        let once = cover.clone();
        stamp(&mut cover, 32, 32, (5.5, 20.5), (16.5, 5.5), 1.0);
        assert_eq!(once, cover);
        stamp(&mut cover, 32, 32, (16.5, 5.5), (26.5, 20.5), 1.0);
        assert_eq!(cover[5 * 32 + 16], 1.0);
        assert!(cover.iter().all(|v| (0.0..=1.0).contains(v)));
    }

    #[test]
    fn degenerate_round_segment_is_bounded_and_single_data_point_stays_empty() {
        let mut cover = vec![0.0; 16 * 16];
        stamp(&mut cover, 16, 16, (8.5, 8.5), (8.5, 8.5), 1.0);
        assert_eq!(cover[8 * 16 + 8], 1.0);
        assert_eq!(cover[0], 0.0);
        let spec = LineSpec {
            values: vec![Some(2.0)],
            empty: EmptyCells::Gap,
            axis_min: None,
            axis_max: None,
            color: [55, 96, 146],
            weight_px: 1.0,
            dates: None,
        };
        let raster = raster_line(16, 16, &spec).unwrap();
        assert!(raster.coverage.iter().all(|v| *v == 0.0));
    }

    #[test]
    fn line_float_coverage_composites_without_quantizing_or_restarting_sampling() {
        let spec = LineSpec {
            values: vec![Some(1.0), Some(4.0), Some(2.0)],
            empty: EmptyCells::Gap,
            axis_min: None,
            axis_max: None,
            color: [55, 96, 146],
            weight_px: 1.0,
            dates: None,
        };
        let raster = raster_line(64, 20, &spec).unwrap();
        let (_, _, rgb) = decode_rgb(&render(64, 20, &spec).unwrap());
        let mut fractional = 0;
        for y in 0..20 {
            for x in 0..64 {
                let i = (y * 64 + x) as usize;
                assert_eq!(
                    raster.composite_pixel(x, y, [255; 3]),
                    rgb[i * 3..i * 3 + 3]
                );
                let cov = f64::from(raster.coverage[i]);
                if cov > 0.0 && cov < 1.0 {
                    fractional += 1;
                    let base = [40, 80, 120];
                    let mut expected = base;
                    for k in 0..3 {
                        expected[k] = (f64::from(base[k])
                            + (f64::from(spec.color[k]) - f64::from(base[k])) * cov)
                            .round() as u8;
                    }
                    assert_eq!(raster.composite_pixel(x, y, base), expected);
                }
            }
        }
        assert!(fractional > 0);
    }

    #[test]
    fn white_columns_retain_coverage_on_a_nonwhite_background() {
        let raster = raster_columns(64, 20, &[1.0, 4.0, 2.0], &[[255; 3]; 3], None).unwrap();
        assert_eq!(raster.composite_pixel(30, 6, [40, 80, 120]), [255; 3]);
        assert_eq!(raster.composite_pixel(0, 0, [40, 80, 120]), [40, 80, 120]);
        let (_, _, px) =
            decode_rgb(&render_columns(64, 20, &[1.0, 4.0, 2.0], &[[255; 3]; 3]).unwrap());
        assert!(px.iter().all(|v| *v == 255));
    }

    #[test]
    fn win_loss_half_bars_match_excel_copypicture_geometry() {
        let series = [0x37, 0x60, 0x92];
        let neg = [0xd0, 0x00, 0x00];
        let values = [Some(1.0), Some(-1.0), Some(1.0), Some(-1.0), Some(1.0)];
        let png = render_win_loss(64, 20, &values, series, neg, true).unwrap();
        let (w, h, px) = decode_rgb(&png);
        assert_eq!((w, h), (64, 20));
        let at = |x: u32, y: u32| {
            let i = ((y * w + x) * 3) as usize;
            [px[i], px[i + 1], px[i + 2]]
        };
        // column_x(64,5,0)=(4,12) positive y 5..=10
        assert_eq!(at(4, 5), series);
        assert_eq!(at(4, 10), series);
        assert_eq!(at(4, 11), [255, 255, 255]);
        assert_eq!(at(12, 5), series);
        // index 1 negative y 11..=16; column_x(64,5,1)=(16,24)
        assert_eq!(at(16, 11), neg);
        assert_eq!(at(16, 16), neg);
        assert_eq!(at(16, 10), [255, 255, 255]);
        // zeros / empty
        let zeros = [Some(0.0); 5];
        let png0 = render_win_loss(64, 20, &zeros, series, neg, true).unwrap();
        let (_, _, px0) = decode_rgb(&png0);
        assert!(px0.iter().all(|&b| b == 255));
        let mixed = [Some(1.0), None, Some(0.0), Some(-2.0), None];
        let png_m = render_win_loss(64, 20, &mixed, series, neg, true).unwrap();
        let (_, _, pxm) = decode_rgb(&png_m);
        let at_m = |x: u32, y: u32| {
            let i = ((y * 64 + x) * 3) as usize;
            [pxm[i], pxm[i + 1], pxm[i + 2]]
        };
        assert_eq!(at_m(4, 5), series);
        assert_eq!(at_m(16, 5), [255, 255, 255]); // empty slot 1 would be (16,24)
        assert_eq!(at_m(28, 5), [255, 255, 255]); // zero slot 2 (28,36)
        assert_eq!(at_m(40, 11), neg); // slot 3 column_x(64,5,3)=(40,48)
    }

    #[test]
    fn win_loss_respects_custom_colors_and_height() {
        // 颜色与高宽来自调用方（OOXML colorSeries/colorNegative / 格子像素），不是探针写死。
        let series = [0x11, 0x22, 0x33];
        let neg = [0xaa, 0xbb, 0xcc];
        let values = [Some(2.0), Some(-3.0)];
        let png = render_win_loss(80, 24, &values, series, neg, true).unwrap();
        let (w, h, px) = decode_rgb(&png);
        assert_eq!((w, h), (80, 24));
        let at = |x: u32, y: u32| {
            let i = ((y * w + x) * 3) as usize;
            [px[i], px[i + 1], px[i + 2]]
        };
        // h=24 → bottom=20, plot=16, half=8 → pos 5..=12, neg 13..=20
        let (px0, _) = column_x(80, 2, 0).unwrap();
        let (nx0, _) = column_x(80, 2, 1).unwrap();
        assert_eq!(at(px0 as u32, 5), series);
        assert_eq!(at(px0 as u32, 12), series);
        assert_eq!(at(px0 as u32, 13), [255, 255, 255]);
        assert_eq!(at(nx0 as u32, 13), neg);
        assert_eq!(at(nx0 as u32, 20), neg);
    }

    #[test]
    fn win_loss_without_negative_flag_uses_series_color() {
        let series = [0x37, 0x60, 0x92];
        let neg = [0xd0, 0x00, 0x00];
        let values = [Some(-1.0)];
        let png = render_win_loss(64, 20, &values, series, neg, false).unwrap();
        let (w, _, px) = decode_rgb(&png);
        let (x0, _) = column_x(64, 1, 0).unwrap();
        let i = ((11u32 * w + x0 as u32) * 3) as usize;
        assert_eq!([px[i], px[i + 1], px[i + 2]], series);
    }

    fn line_spec(values: Vec<Option<f64>>, dates: Option<Vec<f64>>) -> LineSpec {
        LineSpec {
            values,
            empty: EmptyCells::Gap,
            axis_min: None,
            axis_max: None,
            color: [55, 96, 146],
            weight_px: 1.0,
            dates,
        }
    }

    fn peak_x(rgb: &[u8], w: u32, h: u32, color: [u8; 3]) -> u32 {
        // The highest plotted value sits nearest the top of the plot (y around TOP=5).
        let mut best_y = h;
        let mut sum = 0u64;
        let mut n = 0u64;
        for y in 0..h {
            for x in 0..w {
                let i = ((y * w + x) * 3) as usize;
                if [rgb[i], rgb[i + 1], rgb[i + 2]] == color {
                    if y < best_y {
                        best_y = y;
                        sum = u64::from(x);
                        n = 1;
                    } else if y == best_y {
                        sum += u64::from(x);
                        n += 1;
                    }
                }
            }
        }
        assert!(n > 0, "expected ink");
        (sum / n) as u32
    }

    #[test]
    fn column_x_at_matches_integer_slots() {
        for width in [6u32, 16, 64, 72, 80, 128] {
            for n in 1..=12 {
                for i in 0..n {
                    let via_int = column_x(width, n, i);
                    let via_at = column_x_at(width, n, i as f64);
                    assert_eq!(via_int, via_at, "width={width} n={n} i={i}");
                }
            }
        }
    }

    #[test]
    fn equal_dates_match_category_axis_for_line_column_and_win_loss() {
        let values = vec![Some(1.0), Some(4.0), Some(2.0)];
        let equal = Some(vec![10.0, 11.0, 12.0]);
        let cat = render(64, 20, &line_spec(values.clone(), None)).unwrap();
        let dated = render(64, 20, &line_spec(values, equal.clone())).unwrap();
        assert_eq!(cat, dated, "equal dates must not change a line");

        let cols = [1.0, 4.0, 2.0];
        let colors = [[55, 96, 146]; 3];
        let cat = render_columns(64, 20, &cols, &colors).unwrap();
        let dated = raster_columns(64, 20, &cols, &colors, equal.as_deref())
            .unwrap()
            .png()
            .unwrap();
        assert_eq!(cat, dated, "equal dates must not change columns");

        let wl = [Some(1.0), Some(-1.0), Some(1.0)];
        let series = [0x37, 0x60, 0x92];
        let neg = [0xd0, 0x00, 0x00];
        let cat = render_win_loss(64, 20, &wl, series, neg, true).unwrap();
        let dated = raster_win_loss(64, 20, &wl, series, neg, true, equal.as_deref())
            .unwrap()
            .png()
            .unwrap();
        assert_eq!(cat, dated, "equal dates must not change win/loss");
    }

    #[test]
    fn uneven_dates_move_the_peak_off_the_category_midpoint() {
        let color = [55, 96, 146];
        let values = vec![Some(1.0), Some(8.0), Some(1.0)];
        let cat = decode_rgb(&render(64, 20, &line_spec(values.clone(), None)).unwrap());
        let early = decode_rgb(
            &render(
                64,
                20,
                &line_spec(values.clone(), Some(vec![0.0, 1.0, 10.0])),
            )
            .unwrap(),
        );
        let late =
            decode_rgb(&render(64, 20, &line_spec(values, Some(vec![0.0, 9.0, 10.0]))).unwrap());
        assert_ne!(
            cat.2, early.2,
            "clustered early dates must not look equally spaced"
        );
        assert_ne!(
            cat.2, late.2,
            "clustered late dates must not look equally spaced"
        );
        let mid = peak_x(&cat.2, 64, 20, color);
        let left = peak_x(&early.2, 64, 20, color);
        let right = peak_x(&late.2, 64, 20, color);
        assert!(
            left < mid && mid < right,
            "peak should follow the date, not the category mid; left={left} mid={mid} right={right}"
        );
    }

    #[test]
    fn reverse_dates_place_the_first_point_on_the_right() {
        let color = [55, 96, 146];
        // First value is the peak; with reversed dates it belongs on the right.
        let values = vec![Some(8.0), Some(1.0), Some(1.0)];
        let (_, _, rgb) =
            decode_rgb(&render(64, 20, &line_spec(values, Some(vec![10.0, 5.0, 0.0]))).unwrap());
        let peak = peak_x(&rgb, 64, 20, color);
        assert!(
            peak > 32,
            "first point with the latest date should sit in the right half; peak={peak}"
        );
    }

    #[test]
    fn missing_value_keeps_date_positions_and_does_not_collapse() {
        let color = [55, 96, 146];
        // 等距日期 + gap：与分类轴相同（中间空点把线段拆开，两端单点不画）。
        let cat = render(64, 20, &line_spec(vec![Some(1.0), None, Some(8.0)], None)).unwrap();
        let dated = render(
            64,
            20,
            &line_spec(
                vec![Some(1.0), None, Some(8.0)],
                Some(vec![10.0, 11.0, 12.0]),
            ),
        )
        .unwrap();
        assert_eq!(cat, dated);

        // span 会跨过空值；末点在最后的日期上，不能被挤进中间空槽。
        let mut spec = line_spec(vec![Some(1.0), None, Some(8.0)], Some(vec![0.0, 1.0, 10.0]));
        spec.empty = EmptyCells::Span;
        let (_, _, rgb) = decode_rgb(&render(64, 20, &spec).unwrap());
        let peak = peak_x(&rgb, 64, 20, color);
        assert!(
            peak > 40,
            "a high value on the last date should stay at the right; peak={peak}"
        );
    }

    #[test]
    fn duplicate_dates_share_an_x_and_uneven_columns_differ_from_category() {
        let dates = [0.0, 0.0, 10.0];
        assert!((date_slot(&dates, 0).unwrap() - date_slot(&dates, 1).unwrap()).abs() < 1e-12);
        assert!((date_slot(&dates, 2).unwrap() - 2.0).abs() < 1e-12);
        let cols = [1.0, 4.0, 2.0];
        let colors = [[55, 96, 146], [200, 0, 0], [55, 96, 146]];
        let cat = render_columns(64, 20, &cols, &colors).unwrap();
        let dated = raster_columns(64, 20, &cols, &colors, Some(&dates))
            .unwrap()
            .png()
            .unwrap();
        assert_ne!(cat, dated);
        let (x0_a, x1_a) = column_x_at(64, 3, date_slot(&dates, 0).unwrap()).unwrap();
        let (x0_b, x1_b) = column_x_at(64, 3, date_slot(&dates, 1).unwrap()).unwrap();
        assert_eq!((x0_a, x1_a), (x0_b, x1_b));
    }

    #[test]
    fn unsupported_date_inputs_stay_not_implemented() {
        // n>=2 且所有日期相同，无法定比例。
        assert!(date_slot(&[5.0, 5.0, 5.0], 0).is_err());
        let err = render(
            64,
            20,
            &line_spec(
                vec![Some(1.0), Some(2.0), Some(3.0)],
                Some(vec![7.0, 7.0, 7.0]),
            ),
        )
        .unwrap_err();
        assert_eq!(err.code(), crate::error::ErrorCode::NotImplemented);
        let err = render(
            64,
            20,
            &line_spec(vec![Some(1.0), Some(2.0)], Some(vec![1.0])),
        )
        .unwrap_err();
        assert_eq!(err.code(), crate::error::ErrorCode::NotImplemented);
        let err = match raster_columns(
            64,
            20,
            &[1.0, 2.0],
            &[[55, 96, 146]; 2],
            Some(&[f64::NAN, 1.0]),
        ) {
            Err(err) => err,
            Ok(_) => panic!("non-finite dates should stay NotImplemented"),
        };
        assert_eq!(err.code(), crate::error::ErrorCode::NotImplemented);
    }
}
