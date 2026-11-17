//! Shared read-only worksheet column geometry. UI character widths are not a layout unit.
use crate::error::{Error, ErrorKind};
use crate::opc::{Package, Relationships};
use crate::xml::Element;

use super::render_dpi::RenderDpi;

#[derive(Clone, Copy)]
pub(crate) struct GeometryContext {
    mdw: f64,
    dpi: RenderDpi,
}

impl GeometryContext {
    /// Canonical OOXML Calibri 11 / 96 dpi example; also the platform-independent fallback.
    pub(crate) const fn calibri11() -> Self {
        Self {
            mdw: 7.0,
            dpi: RenderDpi::DEFAULT,
        }
    }

    pub(crate) fn with_dpi(self, dpi: RenderDpi) -> Self {
        Self { mdw: self.mdw, dpi }
    }

    pub(crate) fn dpi(self) -> RenderDpi {
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
        let font = fonts
            .get(normal_font_index(styles))
            .ok_or_else(unsupported)?;
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
            mdw: normal_mdw(name, pt, bold, italic)?,
            dpi: RenderDpi::DEFAULT,
        })
    }

    pub(crate) fn column_px(&self, sheet: &Element, col: u32) -> Result<i32, Error> {
        Ok(self.dpi.scale_px(self.column_px_96(sheet, col)?))
    }

    fn column_px_96(&self, sheet: &Element, col: u32) -> Result<i32, Error> {
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
        Ok((8.43 * self.mdw + 5.0).floor() as i32)
    }

    pub(crate) fn xml_width_px(&self, width: f64) -> Result<i32, Error> {
        xml_width_px(width, self.mdw)
    }
}

/// `width` is the stored OOXML value, already including its padding.
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
fn normal_mdw(name: &str, pt: f64, bold: bool, italic: bool) -> Result<f64, Error> {
    let face = super::chart_gdi::Face::open_cell((pt * 96.0 / 72.0) as f32, bold, italic, name)
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
fn normal_mdw(name: &str, pt: f64, bold: bool, italic: bool) -> Result<f64, Error> {
    if name.eq_ignore_ascii_case("Calibri") && (pt - 11.0).abs() < 0.0001 && !bold && !italic {
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
    fn default_and_hidden_columns_use_the_same_units() {
        let doc = crate::xml::parse(br#"<worksheet><sheetFormatPr defaultColWidth="10"/><cols><col min="2" max="2" width="10.125"/><col min="3" max="3" width="10.125" hidden="1"/></cols></worksheet>"#).unwrap();
        for (mdw, default, explicit) in [(7.0, 70, 71), (8.0, 80, 81)] {
            let g = GeometryContext {
                mdw,
                dpi: RenderDpi::DEFAULT,
            };
            assert_eq!(g.column_px(&doc.root, 1).unwrap(), default);
            assert_eq!(g.column_px(&doc.root, 2).unwrap(), explicit);
            assert_eq!(g.column_px(&doc.root, 3).unwrap(), 0);
            assert_eq!(g.column_px(&doc.root, 100).unwrap(), default);
        }
    }
    #[test]
    fn localized_builtin_normal_precedes_english_name() {
        let doc = crate::xml::parse("<styleSheet><cellStyles><cellStyle name=\"Normal\" xfId=\"0\"/><cellStyle name=\"常规重命名\" builtinId=\"0\" xfId=\"1\"/></cellStyles><cellStyleXfs><xf fontId=\"0\"/><xf fontId=\"2\"/></cellStyleXfs></styleSheet>".as_bytes()).unwrap();
        assert_eq!(normal_font_index(&doc.root), 2);
    }
    #[cfg(windows)]
    #[test]
    fn fixed96_mdw_uses_actual_normal_face_and_size() {
        for (name, pt, mdw) in [
            ("Calibri", 11, 7),
            ("Calibri", 14, 10),
            ("Arial", 11, 8),
            ("Consolas", 11, 8),
            ("DengXian", 11, 8),
        ] {
            let doc = crate::xml::parse(format!("<styleSheet><fonts><font><name val=\"{name}\"/><sz val=\"{pt}\"/></font></fonts></styleSheet>").as_bytes()).unwrap();
            let g = GeometryContext::from_styles(&doc.root).unwrap();
            assert_eq!(g.mdw, f64::from(mdw), "{name} {pt}");
        }
        assert!(normal_mdw("DefinitelyMissingFont_ooxp_20261117", 11.0, false, false).is_err());
    }
}
