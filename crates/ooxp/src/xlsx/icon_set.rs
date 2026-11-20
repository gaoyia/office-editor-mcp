//! 条件格式图标集：选择语义（cfvo / reverse / showValue）与最小子集出图。
//!
//! 真源：[MS-XLSX] CT_IconSet（首个 cfvo 忽略；3–5 个阈值值）、ECMA-376 iconSet 例、
//! 以及公开文档化的三档比较顺序（从高阈值向下，gte 缺省 true）。
//!
//! 出图只覆盖 `3Arrows` / `3ArrowsGray` / `3TrafficLights1`。字形是程序绘制的带简单
//! 高光近似，不是 Office 私有资产，也不是官方像素描摹；不得声称整图 differ=0。
//! 其它集合、`custom` / `cfIcon` 仍由调用方返回 NotImplemented。

#![allow(dead_code)]

/// 本刀锁定的最小真实子集。
pub const MINIMAL_THREE_ICON_SETS: &[&str] = &["3Arrows", "3ArrowsGray", "3TrafficLights1"];

/// 未写 `ht` 的自动行，若该行落在已实现图标集区域内，至少抬到 18 磅。
///
/// 官方 Excel 16.0 CopyPicture（窗口 106 DPI）：Calibri 11 缺省行本为 14.25 磅→21px，
/// 有 3Arrows / 3ArrowsGray / 3TrafficLights1 时变为 27px，与 `excel_row_px(18)` @106
/// 一致。Arial 14 同行未再抬高。写了 `ht` 的自定义行不因图标抬高（30 磅行仍按自定义）。
/// 这是对照证据上的最佳努力，不是 ECMA 公开公式。
pub const ICON_AUTO_ROW_MIN_PT: f64 = 18.0;

/// 图标槽相对格子左缘（含左侧网格像素）的起点。官方 11pt / 14pt 均为 x=cell_x0+2。
pub const ICON_LEFT_PAD: i32 = 2;

/// 最小三色集合。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThreeIconSet {
    Arrows,
    ArrowsGray,
    TrafficLights1,
}

/// 解析 `iconSet/@iconSet`；缺省与 Excel/写入路径一致为 `3TrafficLights1`。
pub fn icon_set_name(attr: Option<&str>) -> &str {
    match attr {
        None | Some("") => "3TrafficLights1",
        Some(s) => s,
    }
}

/// 是否为本刀承认的三色子集名称（不含 4/5 档、Flags、custom）。
pub fn is_minimal_three_set(name: &str) -> bool {
    MINIMAL_THREE_ICON_SETS.contains(&name)
}

pub fn parse_minimal_set(name: &str) -> Option<ThreeIconSet> {
    match name {
        "3Arrows" => Some(ThreeIconSet::Arrows),
        "3ArrowsGray" => Some(ThreeIconSet::ArrowsGray),
        "3TrafficLights1" => Some(ThreeIconSet::TrafficLights1),
        _ => None,
    }
}

/// 三档集合的图标个数；非本刀子集返回 None。
pub fn icon_count_for_set(name: &str) -> Option<usize> {
    if is_minimal_three_set(name) {
        Some(3)
    } else {
        None
    }
}

/// `showValue` 缺省 true（ECMA / MS-XLSX / 写入路径）。
pub fn show_value(attr: Option<&str>) -> bool {
    match attr {
        None => true,
        Some("0") | Some("false") | Some("False") => false,
        Some(_) => true,
    }
}

/// `reverse` 缺省 false。
pub fn reverse_icons(attr: Option<&str>) -> bool {
    matches!(attr, Some("1") | Some("true") | Some("True"))
}

/// `cfvo/@gte` 缺省 true（inclusive）。
pub fn cfvo_gte(attr: Option<&str>) -> bool {
    !matches!(attr, Some("0") | Some("false") | Some("False"))
}

/// 按已解析的阈值选图标下标。
///
/// - `thresholds` 与 `gte` 等长，文档顺序，长度 3..=5
/// - **第一个阈值忽略**（[MS-XLSX] CT_IconSet）
/// - 从最高档向下比：命中则取该档下标；全未命中 → 0
/// - `reverse` 时映射为 `n - 1 - icon`
///
/// 返回的下标在反转前：0=最低档，n-1=最高档。
pub fn select_icon_index(
    value: f64,
    thresholds: &[f64],
    gte: &[bool],
    reverse: bool,
) -> Result<usize, ()> {
    let n = thresholds.len();
    if !(3..=5).contains(&n) || gte.len() != n {
        return Err(());
    }
    let mut icon = 0usize;
    for i in (1..n).rev() {
        let thr = thresholds[i];
        let hit = if gte[i] { value >= thr } else { value > thr };
        if hit {
            icon = i;
            break;
        }
    }
    if reverse {
        icon = n - 1 - icon;
    }
    Ok(icon)
}

/// `percent` 型 cfvo：相对区域 min..max 的比例（0–100）。
/// 父级 `iconSet/@percent` 按 MS-XLSX 忽略；以单个 cfvo/@type 为准。
pub fn percent_threshold(pct: f64, values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let min = values.iter().copied().fold(f64::INFINITY, f64::min);
    let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let t = (pct / 100.0).clamp(0.0, 1.0);
    Some(min + (max - min) * t)
}

/// 图标槽边长（像素）。公开公式：`round(pt * dpi / 72)`，至少 1。
///
/// 官方 106 DPI：Calibri 11 → 16（11×106/72≈16.19）；Arial 14 → 21（≈20.61），
/// 官方长边实测 22，允许 1px 残差。不跟随行高或列宽。
pub fn icon_slot_px(font_pt: f64, dpi: f64) -> i32 {
    if !font_pt.is_finite() || font_pt <= 0.0 || !dpi.is_finite() || dpi <= 0.0 {
        return 1;
    }
    (font_pt * dpi / 72.0).round().clamp(1.0, 4096.0) as i32
}

/// 图标占用的左侧文字避让宽度：槽起点 + 边长 + 1px 间隙。
pub fn icon_text_reserve(font_pt: f64, dpi: f64) -> i32 {
    ICON_LEFT_PAD + icon_slot_px(font_pt, dpi) + 1
}

/// 在格子 `(x0,y0,x1,y1)` 内放图标槽（左上角与边长）。
/// 水平：`x0 + ICON_LEFT_PAD`。垂直：槽中心对准 `typo_center_y`（与底对齐文字行盒中心同一套），
/// 再夹进格子；上下若放得下则各留 1px 给网格。
pub fn icon_slot_rect(
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    slot: i32,
    typo_center_y: f64,
) -> Option<(i32, i32, i32)> {
    let avail_w = x1 - x0 - ICON_LEFT_PAD - 1;
    let avail_h = y1 - y0;
    if avail_w < 1 || avail_h < 1 {
        return None;
    }
    let size = slot.min(avail_w).min(avail_h).max(1);
    let x = x0 + ICON_LEFT_PAD;
    let mut y = (typo_center_y - f64::from(size) / 2.0).round() as i32;
    let min_y = y0 + 1;
    let max_y = y1 - size - 1;
    if max_y >= min_y {
        y = y.clamp(min_y, max_y);
    } else {
        y = y.clamp(y0, (y1 - size).max(y0));
    }
    Some((x, y, size))
}

/// 单行文字行盒中心。`pitch` 为自动行高像素，`top0` 为该行盒顶（与 `paint_labels` 相同）。
pub fn typo_line_center(top0: f64, pitch: f64) -> f64 {
    top0 + pitch * 0.5
}

#[derive(Clone, Copy)]
struct IconPalette {
    fill: [u8; 3],
    dark: [u8; 3],
    light: [u8; 3],
}

fn palette(set: ThreeIconSet, index: usize) -> IconPalette {
    const RED: IconPalette = IconPalette {
        fill: [214, 85, 50],
        dark: [158, 56, 28],
        light: [252, 220, 200],
    };
    const YELLOW: IconPalette = IconPalette {
        fill: [234, 194, 130],
        dark: [164, 128, 43],
        light: [255, 245, 210],
    };
    const GREEN: IconPalette = IconPalette {
        fill: [104, 164, 144],
        dark: [50, 111, 91],
        light: [200, 230, 220],
    };
    const GRAY: IconPalette = IconPalette {
        fill: [168, 166, 166],
        dark: [100, 98, 98],
        light: [230, 230, 230],
    };
    let color = index.min(2);
    match set {
        ThreeIconSet::TrafficLights1 | ThreeIconSet::Arrows => match color {
            0 => RED,
            1 => YELLOW,
            _ => GREEN,
        },
        ThreeIconSet::ArrowsGray => GRAY,
    }
}

fn mix(a: [u8; 3], b: [u8; 3], t: f64) -> [u8; 3] {
    let t = t.clamp(0.0, 1.0);
    [
        (f64::from(a[0]) * (1.0 - t) + f64::from(b[0]) * t).round() as u8,
        (f64::from(a[1]) * (1.0 - t) + f64::from(b[1]) * t).round() as u8,
        (f64::from(a[2]) * (1.0 - t) + f64::from(b[2]) * t).round() as u8,
    ]
}

fn point_in_poly(x: f64, y: f64, pts: &[(f64, f64)]) -> bool {
    let n = pts.len();
    if n < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = n - 1;
    for i in 0..n {
        let (xi, yi) = pts[i];
        let (xj, yj) = pts[j];
        if ((yi > y) != (yj > y)) && (x < (xj - xi) * (y - yi) / (yj - yi + 0.0) + xi) {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// 下箭头（单位方框，y 向下）：短柄在上，三角头朝下。
fn arrow_down_unit() -> Vec<(f64, f64)> {
    let sw = 0.30;
    let sh = 0.40;
    let hw = 0.92;
    let top = 0.06;
    let mid = top + sh;
    let tip = 0.94;
    vec![
        (0.5 - sw / 2.0, top),
        (0.5 + sw / 2.0, top),
        (0.5 + sw / 2.0, mid),
        (0.5 + hw / 2.0, mid),
        (0.5, tip),
        (0.5 - hw / 2.0, mid),
        (0.5 - sw / 2.0, mid),
    ]
}

fn map_unit(pts: &[(f64, f64)], map: impl Fn(f64, f64) -> (f64, f64)) -> Vec<(f64, f64)> {
    pts.iter().copied().map(|(x, y)| map(x, y)).collect()
}

fn arrow_poly(index: usize, x: f64, y: f64, size: f64) -> Vec<(f64, f64)> {
    let down = arrow_down_unit();
    let unit = match index {
        0 => down,                                 // 低档：红/灰下箭头
        1 => map_unit(&down, |u, v| (v, u)),       // 右箭头：由下箭头逆时针 90°（y 向下）
        _ => map_unit(&down, |u, v| (u, 1.0 - v)), // 高档：上箭头
    };
    map_unit(&unit, |u, v| (x + u * size, y + v * size))
}

fn shade_at(px: f64, py: f64, cx: f64, cy: f64, pal: IconPalette) -> [u8; 3] {
    let dx = (cx - px) / (cx.abs() + 8.0);
    let dy = (cy - py) / (cy.abs() + 8.0);
    let t = (0.55 + 0.55 * (dx * 0.45 + dy * 0.70)).clamp(0.0, 1.0);
    if t < 0.45 {
        mix(pal.dark, pal.fill, t / 0.45)
    } else {
        mix(pal.fill, pal.light, (t - 0.45) / 0.55)
    }
}

fn paint_coverage(
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    pal: IconPalette,
    inside: impl Fn(f64, f64) -> bool,
    mut blend: impl FnMut(i32, i32, [u8; 3], u8),
) {
    let cx = f64::from(x0 + x1) / 2.0;
    let cy = f64::from(y0 + y1) / 2.0;
    const OFF: [f64; 2] = [0.25, 0.75];
    for y in y0..y1 {
        for x in x0..x1 {
            let mut hit = 0u8;
            let mut edge = 0u8;
            for &oy in &OFF {
                for &ox in &OFF {
                    let px = f64::from(x) + ox;
                    let py = f64::from(y) + oy;
                    if inside(px, py) {
                        hit += 1;
                    } else {
                        // 邻域样本用于描边
                        let ring = [
                            (px + 0.35, py),
                            (px - 0.35, py),
                            (px, py + 0.35),
                            (px, py - 0.35),
                        ];
                        if ring.iter().any(|&(qx, qy)| inside(qx, qy)) {
                            edge += 1;
                        }
                    }
                }
            }
            if hit == 0 && edge == 0 {
                continue;
            }
            let cov = ((u16::from(hit) * 255 + u16::from(edge) * 90) / 4).min(255) as u8;
            let color = if hit == 0 {
                pal.dark
            } else {
                shade_at(f64::from(x) + 0.5, f64::from(y) + 0.5, cx, cy, pal)
            };
            blend(x, y, color, cov);
        }
    }
}

/// 在已算好的槽里画程序字形。`index`：0 低档、1 中档、2 高档（反转后）。
pub fn paint_three_icon(
    set: ThreeIconSet,
    index: usize,
    slot_x: i32,
    slot_y: i32,
    slot: i32,
    blend: impl FnMut(i32, i32, [u8; 3], u8),
) {
    if slot < 1 {
        return;
    }
    let pal = palette(set, index);
    match set {
        ThreeIconSet::TrafficLights1 => paint_light(pal, slot_x, slot_y, slot, blend),
        ThreeIconSet::Arrows | ThreeIconSet::ArrowsGray => {
            paint_arrow(pal, index, slot_x, slot_y, slot, blend)
        }
    }
}

fn paint_light(
    pal: IconPalette,
    slot_x: i32,
    slot_y: i32,
    slot: i32,
    blend: impl FnMut(i32, i32, [u8; 3], u8),
) {
    let inset = 0.15;
    let cx = f64::from(slot_x) + f64::from(slot) / 2.0;
    let cy = f64::from(slot_y) + f64::from(slot) / 2.0;
    let r = (f64::from(slot) / 2.0 - inset).max(0.6);
    paint_coverage(
        slot_x,
        slot_y,
        slot_x + slot,
        slot_y + slot,
        pal,
        |px, py| {
            let dx = px - cx;
            let dy = py - cy;
            dx * dx + dy * dy <= r * r
        },
        blend,
    );
}

fn paint_arrow(
    pal: IconPalette,
    index: usize,
    slot_x: i32,
    slot_y: i32,
    slot: i32,
    blend: impl FnMut(i32, i32, [u8; 3], u8),
) {
    let poly = arrow_poly(
        index.min(2),
        f64::from(slot_x),
        f64::from(slot_y),
        f64::from(slot),
    );
    paint_coverage(
        slot_x,
        slot_y,
        slot_x + slot,
        slot_y + slot,
        pal,
        |px, py| point_in_poly(px, py, &poly),
        blend,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ecma_3arrows_percentile_bands() {
        // ECMA 例：0 / 33 / 67 → <33 低档、[33,67) 中档、>=67 高档
        let thr = [0.0, 33.0, 67.0];
        let gte = [true, true, true];
        assert_eq!(select_icon_index(10.0, &thr, &gte, false), Ok(0));
        assert_eq!(select_icon_index(33.0, &thr, &gte, false), Ok(1));
        assert_eq!(select_icon_index(50.0, &thr, &gte, false), Ok(1));
        assert_eq!(select_icon_index(67.0, &thr, &gte, false), Ok(2));
        assert_eq!(select_icon_index(100.0, &thr, &gte, false), Ok(2));
    }

    #[test]
    fn reverse_flips_index() {
        let thr = [0.0, 33.0, 67.0];
        let gte = [true, true, true];
        assert_eq!(select_icon_index(10.0, &thr, &gte, true), Ok(2));
        assert_eq!(select_icon_index(50.0, &thr, &gte, true), Ok(1));
        assert_eq!(select_icon_index(80.0, &thr, &gte, true), Ok(0));
    }

    #[test]
    fn gte_false_is_exclusive() {
        let thr = [0.0, 33.0, 67.0];
        let gte = [true, false, false];
        // 33 不入中档（要 >33）；67 不入高档
        assert_eq!(select_icon_index(33.0, &thr, &gte, false), Ok(0));
        assert_eq!(select_icon_index(33.1, &thr, &gte, false), Ok(1));
        assert_eq!(select_icon_index(67.0, &thr, &gte, false), Ok(1));
        assert_eq!(select_icon_index(67.1, &thr, &gte, false), Ok(2));
    }

    #[test]
    fn first_threshold_ignored() {
        // 即使把首阈值设得很高，低值仍落在 0 档（只看第 2/3）
        let thr = [999.0, 33.0, 67.0];
        let gte = [true, true, true];
        assert_eq!(select_icon_index(10.0, &thr, &gte, false), Ok(0));
    }

    #[test]
    fn show_value_and_reverse_defaults() {
        assert!(show_value(None));
        assert!(!show_value(Some("0")));
        assert!(!reverse_icons(None));
        assert!(reverse_icons(Some("1")));
        assert!(cfvo_gte(None));
        assert!(!cfvo_gte(Some("0")));
    }

    #[test]
    fn minimal_set_names() {
        assert!(is_minimal_three_set(icon_set_name(None)));
        assert!(is_minimal_three_set("3Arrows"));
        assert_eq!(icon_count_for_set("3TrafficLights1"), Some(3));
        assert_eq!(icon_count_for_set("5Rating"), None);
        assert_eq!(icon_count_for_set("3Flags"), None);
        assert_eq!(
            parse_minimal_set("3ArrowsGray"),
            Some(ThreeIconSet::ArrowsGray)
        );
        assert_eq!(parse_minimal_set("5Rating"), None);
    }

    #[test]
    fn percent_threshold_midpoint() {
        let vals = [0.0, 50.0, 100.0];
        assert_eq!(percent_threshold(0.0, &vals), Some(0.0));
        assert_eq!(percent_threshold(50.0, &vals), Some(50.0));
        assert_eq!(percent_threshold(100.0, &vals), Some(100.0));
    }

    #[test]
    fn slot_px_follows_font_and_dpi() {
        assert_eq!(icon_slot_px(11.0, 106.0), 16);
        assert_eq!(icon_slot_px(11.0, 96.0), 15);
        assert_eq!(icon_slot_px(14.0, 106.0), 21);
        assert_eq!(icon_text_reserve(11.0, 106.0), 19);
    }

    #[test]
    fn slot_rect_centers_on_typo_line_at_official_11pt() {
        // 27px 行、底对齐 21px 行距：top0=6，中心 16.5；16px 槽 → y=9。
        let typo = typo_line_center(6.0, 21.0);
        assert!((typo - 16.5).abs() < 1e-9);
        assert_eq!(icon_slot_rect(0, 0, 73, 27, 16, typo), Some((2, 9, 16)));
    }
}
