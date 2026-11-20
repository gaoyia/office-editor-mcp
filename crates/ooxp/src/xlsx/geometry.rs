//! Shared read-only worksheet column geometry. UI character widths are not a layout unit.
use crate::error::{Error, ErrorKind};
use crate::opc::{Package, Relationships};
use crate::xml::Element;

use super::render_dpi::RenderDpi;

#[derive(Clone)]
struct FaceSpec {
    name: String,
    pt: f64,
    bold: bool,
    italic: bool,
}

#[derive(Clone)]
pub(crate) struct GeometryContext {
    mdw: f64,
    dpi: RenderDpi,
    face: Option<FaceSpec>,
    /// Non-Windows cannot remasure MDW at a non-96 export dpi; fall back to the
    /// 96-pixel formula and scale_px. Windows always remasures and never takes
    /// this path, and must not hard-code a host-106 MDW.
    scale_from_96: bool,
}

impl GeometryContext {
    /// Canonical OOXML Calibri 11 / 96 dpi example; also the platform-independent fallback.
    pub(crate) fn calibri11() -> Self {
        Self {
            mdw: 7.0,
            dpi: RenderDpi::DEFAULT,
            face: Some(FaceSpec {
                name: "Calibri".into(),
                pt: 11.0,
                bold: false,
                italic: false,
            }),
            scale_from_96: false,
        }
    }

    pub(crate) fn with_dpi(self, dpi: RenderDpi) -> Result<Self, Error> {
        if dpi == self.dpi {
            return Ok(self);
        }
        if let Some(face) = self.face.clone() {
            match normal_mdw(&face.name, face.pt, face.bold, face.italic, dpi.get()) {
                Ok(mdw) => Ok(Self {
                    mdw,
                    dpi,
                    face: Some(face),
                    scale_from_96: false,
                }),
                Err(_) if !cfg!(windows) => Ok(Self {
                    mdw: self.mdw,
                    dpi,
                    face: Some(face),
                    scale_from_96: true,
                }),
                Err(err) => Err(err),
            }
        } else if self.dpi.is_default() && !dpi.is_default() {
            Ok(Self {
                mdw: self.mdw,
                dpi,
                face: None,
                scale_from_96: true,
            })
        } else {
            Ok(Self {
                mdw: self.mdw,
                dpi,
                face: None,
                scale_from_96: false,
            })
        }
    }

    pub(crate) fn dpi(&self) -> RenderDpi {
        self.dpi
    }

    pub(crate) fn from_package(pkg: &Package) -> Result<Self, Error> {
        let wb = crate::opc::office_document_part(pkg)?;
        let rels = Relationships::from_package(pkg, &wb)?;
        let Some(rel) = rels
            .rels
            .iter()
            .find(|r| r.rel_type.ends_with("/styles") && !r.is_external())
        else {
            return Ok(Self::calibri11());
        };
        let part = crate::opc::resolve_rel_target(&wb, &rel.target)?;
        let doc = crate::xml::parse(pkg.get(&part)?)?;
        Self::from_styles(&doc.root)
    }

    pub(crate) fn from_styles(styles: &Element) -> Result<Self, Error> {
        let Some(fonts) = styles.child_by_local("fonts") else {
            return Ok(Self::calibri11());
        };
        let fonts = fonts.children_by_local("font");
        if fonts.is_empty() {
            return Ok(Self::calibri11());
        }
        // Excel 16.0 列宽 MDW 取 stylesheet 的第一个 font，而不是 Normal
        // cellStyle。两者不一致时（人工把 Normal 指到 Arial、fonts[0] 仍是
        // Calibri）官方 106 CopyPicture / Range.Width 仍按 fonts[0]。
        // 2026-11-20 五组探针锁定：P1-P5 见证据 pr12-t162-width-height。
        let font = &fonts[0];
        let name = font
            .child_by_local("name")
            .and_then(|n| n.attr("val"))
            .unwrap_or("Calibri");
        let pt = font
            .child_by_local("sz")
            .and_then(|n| n.attr("val"))
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or(11.0);
        let bold = font
            .child_by_local("b")
            .is_some_and(|v| !matches!(v.attr("val"), Some("0" | "false" | "off")));
        let italic = font
            .child_by_local("i")
            .is_some_and(|v| !matches!(v.attr("val"), Some("0" | "false" | "off")));
        if !pt.is_finite() || pt <= 0.0 || pt > 409.0 {
            return Err(unsupported());
        }
        Ok(Self {
            mdw: normal_mdw(name, pt, bold, italic, RenderDpi::DEFAULT.get())?,
            dpi: RenderDpi::DEFAULT,
            face: Some(FaceSpec {
                name: name.to_string(),
                pt,
                bold,
                italic,
            }),
            scale_from_96: false,
        })
    }

    pub(crate) fn column_px(&self, sheet: &Element, col: u32) -> Result<i32, Error> {
        let px = self.column_px_at_mdw(sheet, col)?;
        if self.scale_from_96 {
            Ok(self.dpi.scale_px(px))
        } else {
            Ok(px)
        }
    }

    fn column_px_at_mdw(&self, sheet: &Element, col: u32) -> Result<i32, Error> {
        for list in sheet.children_by_local("cols") {
            for node in list.children_by_local("col") {
                let min = node.attr("min").and_then(|v| v.parse::<u32>().ok());
                let max = node.attr("max").and_then(|v| v.parse::<u32>().ok());
                if !matches!((min, max), (Some(a), Some(b)) if a <= col && col <= b) {
                    continue;
                }
                if on(node.attr("hidden")) {
                    return Ok(0);
                }
                if let Some(width) = node.attr("width") {
                    return self.xml_width_px(parse_width(width)?);
                }
            }
        }
        if let Some(width) = sheet
            .child_by_local("sheetFormatPr")
            .and_then(|n| n.attr("defaultColWidth"))
        {
            return self.xml_width_px(parse_width(width)?);
        }
        default_col_px(self.mdw)
    }

    pub(crate) fn xml_width_px(&self, width: f64) -> Result<i32, Error> {
        xml_width_px(width, self.mdw)
    }
}

/// Default column with no width / defaultColWidth: 8 MDW characters plus 8 px.
/// This equals the historical loor(8.43 * MDW + 5) at MDW 7 (64) and 8 (72),
/// and matches official Excel 16.0 CopyPicture at MDW 11 (96, not 97).
pub(crate) fn default_col_px(mdw: f64) -> Result<i32, Error> {
    if !mdw.is_finite() || mdw <= 0.0 {
        return Err(unsupported());
    }
    let px = (8.0 * mdw + 8.0).floor();
    if px > f64::from(i32::MAX) {
        return Err(unsupported());
    }
    Ok((px as i32).max(1))
}

/// width is the stored OOXML value, already including its padding.
pub(crate) fn xml_width_px(width: f64, mdw: f64) -> Result<i32, Error> {
    if !width.is_finite() || width < 0.0 || !mdw.is_finite() || mdw <= 0.0 {
        return Err(unsupported());
    }
    let px = (((256.0 * width + (128.0 / mdw).floor()) / 256.0) * mdw).floor();
    if px > f64::from(i32::MAX) {
        return Err(unsupported());
    }
    Ok((px as i32).max(1))
}

fn parse_width(v: &str) -> Result<f64, Error> {
    v.parse().map_err(|_| unsupported())
}
fn on(v: Option<&str>) -> bool {
    matches!(v, Some("1" | "true" | "on"))
}
fn unsupported() -> Error {
    Error::with_path(ErrorKind::SheetImageUnsupported, "id")
}

pub(crate) fn normal_font_index(styles: &Element) -> usize {
    let Some(list) = styles.child_by_local("cellStyles") else {
        return 0;
    };
    let entries = list.children_by_local("cellStyle");
    let style = entries
        .iter()
        .find(|s| s.attr("builtinId") == Some("0"))
        .or_else(|| {
            entries.iter().find(|s| {
                s.attr("name")
                    .is_some_and(|n| n.eq_ignore_ascii_case("Normal"))
            })
        });
    let Some(style) = style else {
        return 0;
    };
    let index = style
        .attr("xfId")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(0);
    styles
        .child_by_local("cellStyleXfs")
        .and_then(|xfs| xfs.children_by_local("xf").into_iter().nth(index))
        .and_then(|xf| xf.attr("fontId"))
        .and_then(|v| v.parse().ok())
        .unwrap_or(0)
}

#[cfg(windows)]
fn normal_mdw(name: &str, pt: f64, bold: bool, italic: bool, dpi: f64) -> Result<f64, Error> {
    if !dpi.is_finite() || dpi <= 0.0 {
        return Err(unsupported());
    }
    let face = if (dpi - 96.0).abs() < 0.0001 {
        super::chart_gdi::Face::open_cell((pt * 96.0 / 72.0) as f32, bold, italic, name)
    } else {
        super::chart_gdi::Face::open_cell_dpi((pt * dpi / 72.0) as f32, bold, italic, name, dpi)
    }
    .ok_or_else(unsupported)?;
    let alias = |n: &str| {
        if n.eq_ignore_ascii_case("DengXian") || n == "等线" {
            "dengxian".to_string()
        } else {
            n.to_lowercase()
        }
    };
    if alias(face.family_name()) != alias(name) {
        return Err(unsupported());
    }
    let mut mdw = 0.0_f64;
    for digit in '0'..='9' {
        let w = face.width(&digit.to_string());
        if !w.is_finite() || w <= 0.0 {
            return Err(unsupported());
        }
        mdw = mdw.max(w);
    }
    Ok(mdw)
}

#[cfg(not(windows))]
fn normal_mdw(name: &str, pt: f64, bold: bool, italic: bool, dpi: f64) -> Result<f64, Error> {
    if name.eq_ignore_ascii_case("Calibri")
        && (pt - 11.0).abs() < 0.0001
        && !bold
        && !italic
        && (dpi - 96.0).abs() < 0.0001
    {
        Ok(7.0)
    } else {
        Err(unsupported())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn direct_width_does_not_roundtrip_through_ui() {
        for (w, mdw, expected) in [
            (10.125, 7.0, 71),
            (10.125, 8.0, 81),
            (16.7109375, 7.0, 117),
            // Excel UI 255 characters is stored with padding, not as width=255.
            (255.7109375, 7.0, 1790),
            (8.7109375, 7.0, 61),
        ] {
            assert_eq!(xml_width_px(w, mdw).unwrap(), expected);
        }
        for w in [f64::NAN, f64::INFINITY, -1.0, f64::MAX] {
            assert!(xml_width_px(w, 7.0).is_err());
        }
    }
    #[test]
    fn default_column_is_eight_mdw_plus_eight_px() {
        // Official 106 CopyPicture 2026-11-18: MDW 8 default 72, MDW 11 default 96
        // (not floor(8.43*11+5)=97). MDW 7 stays 64; MDW 10 is 88, not 89.
        for (mdw, expected) in [(7.0, 64), (8.0, 72), (10.0, 88), (11.0, 96)] {
            assert_eq!(default_col_px(mdw).unwrap(), expected, "mdw={mdw}");
        }
        assert!(default_col_px(0.0).is_err());
        assert!(default_col_px(f64::NAN).is_err());
    }
    #[test]
    fn default_and_hidden_columns_use_the_same_units() {
        let doc = crate::xml::parse(br#"<worksheet><sheetFormatPr defaultColWidth="10"/><cols><col min="2" max="2" width="10.125"/><col min="3" max="3" width="10.125" hidden="1"/></cols></worksheet>"#).unwrap();
        for (mdw, default, explicit) in [(7.0, 70, 71), (8.0, 80, 81)] {
            let g = GeometryContext {
                mdw,
                dpi: RenderDpi::DEFAULT,
                face: None,
                scale_from_96: false,
            };
            assert_eq!(g.column_px(&doc.root, 1).unwrap(), default);
            assert_eq!(g.column_px(&doc.root, 2).unwrap(), explicit);
            assert_eq!(g.column_px(&doc.root, 3).unwrap(), 0);
            assert_eq!(g.column_px(&doc.root, 100).unwrap(), default);
        }
    }
    #[test]
    fn explicit_widths_use_mdw_at_the_context_dpi() {
        // Stored XML from setColWidth (canonical MDW=7) evaluated at request-dpi MDW.
        // Official 106 CopyPicture 2026-11-18, Calibri 11 / 等线 11 / Calibri 14.
        let xml = [
            ("1.2109375", 8, 10, 11, 13),
            ("1.7109375", 8, 14, 11, 19),
            ("3.2109375", 8, 26, 11, 35),
            ("9.140625", 8, 73, 11, 101),
            ("10.7109375", 8, 86, 11, 118),
            ("10.125", 8, 81, 11, 111),
            ("20.7109375", 8, 166, 11, 228),
            ("30.7109375", 8, 246, 11, 338),
        ];
        for (width, mdw_a, px_a, mdw_b, px_b) in xml {
            assert_eq!(
                xml_width_px(width.parse().unwrap(), f64::from(mdw_a)).unwrap(),
                px_a
            );
            assert_eq!(
                xml_width_px(width.parse().unwrap(), f64::from(mdw_b)).unwrap(),
                px_b
            );
        }
        let doc = crate::xml::parse(
            br#"<worksheet><cols><col min="1" max="1" width="10.125"/><col min="2" max="2" hidden="1"/></cols></worksheet>"#,
        )
        .unwrap();
        let g = GeometryContext {
            mdw: 8.0,
            dpi: RenderDpi::from_request(&{
                let mut m = serde_json::Map::new();
                m.insert("dpi".into(), serde_json::json!(106));
                m
            })
            .unwrap(),
            face: None,
            scale_from_96: false,
        };
        assert_eq!(g.column_px(&doc.root, 1).unwrap(), 81);
        assert_eq!(g.column_px(&doc.root, 2).unwrap(), 0);
        assert_eq!(g.column_px(&doc.root, 3).unwrap(), 72);
    }
    #[test]
    fn localized_builtin_normal_precedes_english_name() {
        let doc = crate::xml::parse("<styleSheet><cellStyles><cellStyle name=\"Normal\" xfId=\"0\"/><cellStyle name=\"常规重命名\" builtinId=\"0\" xfId=\"1\"/></cellStyles><cellStyleXfs><xf fontId=\"0\"/><xf fontId=\"2\"/></cellStyleXfs></styleSheet>".as_bytes()).unwrap();
        assert_eq!(normal_font_index(&doc.root), 2);
    }

    #[cfg(windows)]
    #[test]
    fn column_mdw_follows_first_font_not_normal_style() {
        // Excel 16.0 window 106, 2026-11-20: fonts[0]=Calibri, Normal=Arial
        // still uses Calibri MDW (8 at 106, width 10.125 -> 81), not Arial 9->91.
        let doc = crate::xml::parse(
            r#"<styleSheet><fonts><font><name val="Calibri"/><sz val="11"/></font><font><name val="Arial"/><sz val="11"/></font></fonts><cellStyleXfs><xf fontId="0"/><xf fontId="1"/></cellStyleXfs><cellXfs><xf fontId="0" xfId="0"/></cellXfs><cellStyles><cellStyle name="常规重命名" builtinId="0" xfId="1"/></cellStyles></styleSheet>"#.as_bytes(),
        )
        .unwrap();
        let g = GeometryContext::from_styles(&doc.root).unwrap();
        assert_eq!(g.mdw, 7.0, "fonts[0] Calibri @96");
        let dpi106 = RenderDpi::from_request(&{
            let mut m = serde_json::Map::new();
            m.insert("dpi".into(), serde_json::json!(106));
            m
        })
        .unwrap();
        let g106 = g.clone().with_dpi(dpi106).unwrap();
        assert_eq!(g106.mdw, 8.0, "fonts[0] Calibri @106");
        let sheet = crate::xml::parse(
            br#"<worksheet><cols><col min="1" max="1" width="10.125"/></cols></worksheet>"#,
        )
        .unwrap();
        assert_eq!(g.column_px(&sheet.root, 1).unwrap(), 71);
        assert_eq!(g106.column_px(&sheet.root, 1).unwrap(), 81);
    }
    #[cfg(windows)]
    #[test]
    fn mdw_is_measured_at_the_request_dpi() {
        {
            let doc = crate::xml::parse(br#"<styleSheet><fonts><font><name val="Consolas"/><sz val="11"/></font></fonts></styleSheet>"#).unwrap();
            assert_eq!(
                GeometryContext::from_styles(&doc.root).unwrap().mdw,
                8.0,
                "Consolas 11 @96"
            );
        }
        for (name, pt, mdw96, mdw106) in [
            // 106 values locked by GDI + official 2026-11-18 CopyPicture.
            ("Calibri", 11, 7, 8),
            ("Calibri", 14, 10, 11),
            ("Arial", 11, 8, 9),
            ("DengXian", 11, 8, 8),
        ] {
            let doc = crate::xml::parse(format!("<styleSheet><fonts><font><name val=\"{name}\"/><sz val=\"{pt}\"/></font></fonts></styleSheet>").as_bytes()).unwrap();
            let g = GeometryContext::from_styles(&doc.root).unwrap();
            assert_eq!(g.mdw, f64::from(mdw96), "{name} {pt} @96");
            let dpi106 = RenderDpi::from_request(&{
                let mut m = serde_json::Map::new();
                m.insert("dpi".into(), serde_json::json!(106));
                m
            })
            .unwrap();
            let g106 = g.clone().with_dpi(dpi106).unwrap();
            assert_eq!(g106.mdw, f64::from(mdw106), "{name} {pt} @106");
            assert!(!g106.scale_from_96);
            let blank = crate::xml::parse(br#"<worksheet/>"#).unwrap();
            assert_eq!(
                g.column_px(&blank.root, 1).unwrap(),
                default_col_px(f64::from(mdw96)).unwrap()
            );
            assert_eq!(
                g106.column_px(&blank.root, 1).unwrap(),
                default_col_px(f64::from(mdw106)).unwrap()
            );
        }
        assert!(normal_mdw(
            "DefinitelyMissingFont_ooxp_20261117",
            11.0,
            false,
            false,
            96.0
        )
        .is_err());
        // Calibri 11 at 106 is 72, not scale_px(64)=71.
        let c11 = GeometryContext::calibri11()
            .with_dpi({
                let mut m = serde_json::Map::new();
                m.insert("dpi".into(), serde_json::json!(106));
                RenderDpi::from_request(&m).unwrap()
            })
            .unwrap();
        let blank = crate::xml::parse(br#"<worksheet/>"#).unwrap();
        assert_eq!(c11.column_px(&blank.root, 1).unwrap(), 72);
        assert_eq!(c11.mdw, 8.0);
    }
}
