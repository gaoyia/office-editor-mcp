//! 图表工作表导出画布：按纸张、页边距和视图缩放计算，而不是包里绘图锚点的缓存框。
//!
//! 官方 Excel 16.0 对无 `pageSetup` 的库建图表工作表，在 A4 默认打印机上激活后
//! `Chart.Export` 使用横向 A4 可打印区，而不是 drawing `a:ext` 的 432×252 磅。
//! 下列纸张边长的内边界（EMU）来自公开 `PageSetup` / `ChartArea` / `Chart.Export`
//! 有界探针，不是 1078 或 −9pt 的单图修补。页边距按包里的英寸值减去，页眉页脚
//! 不进入画布。省略 `pageSetup` 时，本实现把默认纸上下文明确记为横向 A4（与该
//! 官方对照主机一致）；不是工作表 inspect 的 Letter/portrait 缺省，也不得改系统
//! 打印机。未探测过的 `paperSize` 或自定义 `paperWidth`/`paperHeight` 报
//! `ChartImageUnsupported`，不猜测内边界。
//!
//! 仍保留的残差（不把本项标用户验收，也不改写官方金样）：同一 A4 默认页边距在
//! 106 dpi 下，公式为 1077×704，首次激活或改过窗口缩放后的官方金样是 1078×704，
//! 差 1 像素宽，不为此写死 1078；Letter 默认 0.7/0.75 英寸公式宽是 1005，官方为
//! 1004；A4 纵向默认或四边均为 0.7 英寸时，短边可能再差 1 像素；零页边距时某些
//! 纸张的宽度还会再小 1–2 像素；200% 视图缩放不是精确的 100% 的两倍。

use crate::error::{Error, ErrorKind};
use crate::xml::Element;

use super::RenderDpi;

const EMU_PER_INCH: i64 = 914_400;
const EMU_PER_MM: i64 = 36_000;
const MAX_PX: i64 = 4096;
const DEF_MARGIN_LR_IN: f64 = 0.7;
const DEF_MARGIN_TB_IN: f64 = 0.75;
const DEF_ZOOM: u32 = 100;

/// 受支持的 ST_PaperSize 竖向尺寸（宽×高，EMU）。
const PAPERS: &[(u32, i64, i64)] = &[
    (1, inch_tenths(85), inch_tenths(110)), // Letter
    (5, inch_tenths(85), inch_tenths(140)), // Legal
    (8, mm(297), mm(420)),                  // A3
    (9, mm(210), mm(297)),                  // A4
    (11, mm(148), mm(210)),                 // A5
    (13, mm(182), mm(257)),                 // JIS B5 (not ISO B5 176×250mm)
];

const fn inch_tenths(tenths: i64) -> i64 {
    tenths * EMU_PER_INCH / 10
}

const fn mm(value: i64) -> i64 {
    value * EMU_PER_MM
}

/// 与纸张一边对应的稳定内边界。值来自 36/54/72 磅等边距下的公开 ChartArea。
fn side_inset_emu(side: i64) -> Option<i64> {
    match side {
        5_328_000 => Some(126_272), // 148mm
        // F44 native Excel16/DPI106: both orientations, 36/54/72pt, repeat exports.
        // These side insets are independently constant in ChartArea and native PNG;
        // they extend the existing physical-side rule, not per-case pixel patches.
        6_552_000 => Some(116_694),  // 182mm JIS B5
        7_560_000 => Some(115_404),  // 210mm
        7_772_400 => Some(112_143),  // 8.5in
        9_252_000 => Some(108_000),  // 257mm JIS B5
        10_058_400 => Some(112_143), // 11in
        10_692_000 => Some(124_642), // 297mm
        12_801_600 => Some(112_143), // 14in
        15_120_000 => Some(127_291), // 420mm
        _ => None,
    }
}

fn unsupported() -> Error {
    Error::with_path(ErrorKind::ChartImageUnsupported, "id")
}

fn inch_attr(el: Option<&Element>, name: &str, default: f64) -> i64 {
    let inches = el
        .and_then(|e| e.attr_local(name))
        .and_then(|s| s.trim().parse::<f64>().ok())
        .filter(|n| n.is_finite() && *n >= 0.0)
        .unwrap_or(default);
    (inches * EMU_PER_INCH as f64).round() as i64
}

fn paper_portrait_emu(setup: Option<&Element>) -> Result<(i64, i64), Error> {
    if let Some(setup) = setup {
        if let (Some(w), Some(h)) = (
            setup.attr_local("paperWidth").and_then(parse_measure),
            setup.attr_local("paperHeight").and_then(parse_measure),
        ) {
            if w > 0 && h > 0 {
                return Ok((w, h));
            }
            return Err(unsupported());
        }
        let size = setup
            .attr_local("paperSize")
            .and_then(|s| s.trim().parse::<u32>().ok())
            .unwrap_or(9);
        return paper_size_emu(size);
    }
    // 省略整个 pageSetup：锁定为官方对照主机上的横向 A4 来源，不是 Letter。
    paper_size_emu(9)
}

fn paper_size_emu(size: u32) -> Result<(i64, i64), Error> {
    PAPERS
        .iter()
        .find(|(id, _, _)| *id == size)
        .map(|(_, w, h)| (*w, *h))
        .ok_or_else(unsupported)
}

fn parse_measure(raw: &str) -> Option<i64> {
    let t = raw.trim();
    let split = t.find(|c: char| c.is_ascii_alphabetic())?;
    let (num, unit) = t.split_at(split);
    let value: f64 = num.trim().parse().ok()?;
    if !value.is_finite() || value <= 0.0 {
        return None;
    }
    let emu = match unit.trim().to_ascii_lowercase().as_str() {
        "mm" => value * EMU_PER_MM as f64,
        "cm" => value * (EMU_PER_MM as f64 * 10.0),
        "in" => value * EMU_PER_INCH as f64,
        "pt" => value * 12_700.0,
        "pc" | "pi" => value * 152_400.0,
        _ => return None,
    };
    Some(emu.round() as i64)
}

fn landscape(setup: Option<&Element>) -> bool {
    match setup
        .and_then(|e| e.attr_local("orientation"))
        .map(str::trim)
    {
        Some("portrait") => false,
        // 省略方向或整个 pageSetup 时，Excel 16.0 在本官方主机上用横向。
        _ => true,
    }
}

fn zoom_scale(sheet: &Element) -> Result<u32, Error> {
    let Some(view) = sheet
        .child_by_local("sheetViews")
        .and_then(|v| v.child_by_local("sheetView"))
    else {
        return Ok(DEF_ZOOM);
    };
    let Some(raw) = view.attr_local("zoomScale") else {
        return Ok(DEF_ZOOM);
    };
    let zoom: u32 = raw.trim().parse().map_err(|_| unsupported())?;
    if (10..=400).contains(&zoom) {
        Ok(zoom)
    } else {
        Err(unsupported())
    }
}

fn to_px(emu: i64, zoom: u32, dpi: RenderDpi) -> Result<u32, Error> {
    let mut px = dpi.emu_to_px_round(emu);
    if zoom != 100 {
        px = ((px as f64) * f64::from(zoom) / 100.0).round() as i64;
    }
    if (1..=MAX_PX).contains(&px) {
        Ok(px as u32)
    } else {
        Err(unsupported())
    }
}

/// 图表工作表宿主画布，单位为导出像素。
pub(crate) fn canvas_px(sheet: &Element, dpi: RenderDpi) -> Result<(u32, u32), Error> {
    if sheet.local_name() != "chartsheet" {
        return Err(unsupported());
    }
    let setup = sheet.child_by_local("pageSetup");
    let (mut width, mut height) = paper_portrait_emu(setup)?;
    if landscape(setup) {
        std::mem::swap(&mut width, &mut height);
    }
    let margins = sheet.child_by_local("pageMargins");
    let left = inch_attr(margins, "left", DEF_MARGIN_LR_IN);
    let right = inch_attr(margins, "right", DEF_MARGIN_LR_IN);
    let top = inch_attr(margins, "top", DEF_MARGIN_TB_IN);
    let bottom = inch_attr(margins, "bottom", DEF_MARGIN_TB_IN);
    let inset_w = side_inset_emu(width).ok_or_else(unsupported)?;
    let inset_h = side_inset_emu(height).ok_or_else(unsupported)?;
    let ca_w = width.saturating_sub(left.saturating_add(right).saturating_add(inset_w));
    let ca_h = height.saturating_sub(top.saturating_add(bottom).saturating_add(inset_h));
    if ca_w <= 0 || ca_h <= 0 {
        return Err(unsupported());
    }
    let zoom = zoom_scale(sheet)?;
    Ok((to_px(ca_w, zoom, dpi)?, to_px(ca_h, zoom, dpi)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::xml;

    fn dpi(value: i64) -> RenderDpi {
        RenderDpi::from_request(serde_json::json!({ "dpi": value }).as_object().unwrap()).unwrap()
    }

    fn parse(body: &str) -> xml::Document {
        xml::parse(
            format!(
                r#"<chartsheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">{body}</chartsheet>"#
            )
            .as_bytes(),
        )
        .unwrap()
    }

    fn default_margins() -> &'static str {
        r#"<pageMargins left="0.7" right="0.7" top="0.75" bottom="0.75" header="0.3" footer="0.3"/>"#
    }

    #[test]
    fn omitted_pagesetup_uses_documented_a4_landscape_context() {
        let sheet = parse(&format!(
            r#"<sheetViews><sheetView workbookViewId="0" zoomToFit="1"/></sheetViews>{}"#,
            default_margins()
        ));
        let explicit = parse(&format!(
            r#"<sheetViews><sheetView workbookViewId="0" zoomToFit="1"/></sheetViews>{}<pageSetup paperSize="9" orientation="landscape"/>"#,
            default_margins()
        ));
        assert_eq!(
            canvas_px(&sheet.root, RenderDpi::DEFAULT).unwrap(),
            (975, 638)
        );
        assert_eq!(canvas_px(&sheet.root, dpi(106)).unwrap(), (1077, 704));
        // 官方首次激活金样是 1078×704；显式 A4 页设置与公式均为 1077×704，不写死 1078。
        assert_eq!(
            canvas_px(&sheet.root, RenderDpi::DEFAULT).unwrap(),
            canvas_px(&explicit.root, RenderDpi::DEFAULT).unwrap()
        );
        assert_eq!(
            canvas_px(&sheet.root, dpi(106)).unwrap(),
            canvas_px(&explicit.root, dpi(106)).unwrap()
        );
    }

    #[test]
    fn public_probe_page_sizes_at_106_dpi() {
        // 与 2026-11-18/19 公开探针同尺寸的稳定用例。零边距宽度的 1–2px 特例不写进通过期望。
        let cases = [
            ("9", "landscape", "0", "0", 1225, 863),
            ("9", "landscape", "0.5", "0.5", 1119, 757),
            ("9", "landscape", "0.75", "0.75", 1066, 704),
            ("9", "landscape", "1", "1", 1013, 651),
            ("9", "landscape", "0.7", "0.75", 1077, 704),
            ("9", "portrait", "0.5", "0.5", 757, 1119),
            ("1", "landscape", "0.5", "0.5", 1047, 782),
            ("1", "landscape", "1", "1", 941, 676),
            ("1", "portrait", "0.5", "0.5", 782, 1047),
            ("5", "landscape", "0.5", "0.5", 1365, 782),
            ("11", "landscape", "0.5", "0.5", 757, 497),
            ("8", "landscape", "0.5", "0.5", 1632, 1119),
        ];
        for (paper, orient, lr, tb, w, h) in cases {
            let xml = format!(
                r#"<sheetViews><sheetView workbookViewId="0"/></sheetViews><pageMargins left="{lr}" right="{lr}" top="{tb}" bottom="{tb}" header="0.3" footer="0.3"/><pageSetup paperSize="{paper}" orientation="{orient}"/>"#
            );
            assert_eq!(
                canvas_px(&parse(&xml).root, dpi(106)).unwrap(),
                (w, h),
                "paper={paper} {orient} margins={lr}/{tb}"
            );
        }
    }

    /// Native Chart.Export, not CopyPicture bitmap or printed-page gold. All six
    /// pairs were measured twice in the same readonly Excel16/DPI106 source.
    #[test]
    fn jis_b5_native_paper_margin_orientation_matrix() {
        for (orientation, cases) in [
            (
                "portrait",
                [("0.5", 640, 954), ("0.75", 587, 901), ("1", 534, 848)],
            ),
            (
                "landscape",
                [("0.5", 954, 640), ("0.75", 901, 587), ("1", 848, 534)],
            ),
        ] {
            for (margin, width, height) in cases {
                let sheet = parse(&format!(
                    r#"<pageMargins left="{margin}" right="{margin}" top="{margin}" bottom="{margin}"/><pageSetup paperSize="13" orientation="{orientation}"/>"#
                ));
                assert_eq!(canvas_px(&sheet.root, dpi(106)).unwrap(), (width, height));
            }
        }
    }

    #[test]
    fn jis_b5_reuses_axis_margin_sum_and_dpi_contract() {
        // Derived regression: asymmetric margin sums, not another official gold.
        let sheet = parse(
            r#"<pageMargins left="0.25" right="0.75" top="0.5" bottom="1"/><pageSetup paperSize="13" orientation="portrait"/>"#,
        );
        assert_eq!(canvas_px(&sheet.root, dpi(106)).unwrap(), (640, 901));
        let symmetric = parse(
            r#"<pageMargins left="0.5" right="0.5" top="0.5" bottom="0.5"/><pageSetup paperSize="13" orientation="portrait"/>"#,
        );
        assert_eq!(canvas_px(&symmetric.root, dpi(96)).unwrap(), (580, 864));
    }

    #[test]
    fn jis_b5_does_not_allow_nonpositive_canvas_or_guess_other_papers() {
        let sheet = parse(
            r#"<pageMargins left="4" right="4" top="4" bottom="4"/><pageSetup paperSize="13" orientation="portrait"/>"#,
        );
        assert!(canvas_px(&sheet.root, dpi(106)).is_err());
        assert!(canvas_px(&parse(r#"<pageSetup paperSize="12"/>"#).root, dpi(106)).is_err());
        assert!(canvas_px(
            &parse(r#"<pageSetup paperWidth="182mm" paperHeight="258mm"/>"#).root,
            dpi(106)
        )
        .is_err());
    }

    #[test]
    fn zoom_scale_halves_default_a4_canvas() {
        let sheet = parse(&format!(
            r#"<sheetViews><sheetView workbookViewId="0" zoomScale="50"/></sheetViews>{}<pageSetup paperSize="9" orientation="landscape"/>"#,
            default_margins()
        ));
        assert_eq!(canvas_px(&sheet.root, dpi(106)).unwrap(), (539, 352));
    }

    #[test]
    fn unknown_paper_is_not_guessed() {
        let sheet = parse(
            r#"<pageMargins left="0.7" right="0.7" top="0.75" bottom="0.75"/><pageSetup paperSize="66" orientation="landscape"/>"#,
        );
        assert!(canvas_px(&sheet.root, RenderDpi::DEFAULT).is_err());
    }

    #[test]
    fn worksheet_root_is_rejected() {
        let doc = xml::parse(b"<worksheet/>").unwrap();
        assert!(canvas_px(&doc.root, RenderDpi::DEFAULT).is_err());
    }
}
