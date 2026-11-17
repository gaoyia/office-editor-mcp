//! 条件格式图标集：选择语义（cfvo / reverse / showValue）。
//!
//! 真源：[MS-XLSX] CT_IconSet（首个 cfvo 忽略；3–5 个阈值值）、ECMA-376 iconSet 例、
//! 以及公开文档化的三档比较顺序（从高阈值向下，gte 缺省 true）。
//!
//! **本模块不绘制 Excel 内置字形。** 无公开可合法复现的标准图标像素/矢量资产前，
//! `paint_rules` 对 `iconSet` 仍返回 NotImplemented；不得用三角形等近似冒充整图 PASS。

#![allow(dead_code)]

/// 本刀锁定的最小真实子集（仅选择语义；出图仍 NI）。
pub const MINIMAL_THREE_ICON_SETS: &[&str] = &["3Arrows", "3ArrowsGray", "3TrafficLights1"];

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
    }

    #[test]
    fn percent_threshold_midpoint() {
        let vals = [0.0, 50.0, 100.0];
        assert_eq!(percent_threshold(0.0, &vals), Some(0.0));
        assert_eq!(percent_threshold(50.0, &vals), Some(50.0));
        assert_eq!(percent_threshold(100.0, &vals), Some(100.0));
    }
}
