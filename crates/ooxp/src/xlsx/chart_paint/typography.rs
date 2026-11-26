use fontdue::Font;

use crate::error::{Error, ErrorKind};
use crate::xml::Element;

use super::*;


pub(super) fn title_text(chart: &Element) -> Option<String> {
    if flag(chart, "autoTitleDeleted").unwrap_or(false) {
        return None;
    }
    let title = chart.child_by_local("title")?;
    let tx = title.child_by_local("tx")?;
    if let Some(text) = tx.child_by_local("v").map(|el| el.text_content()) {
        if !text.is_empty() {
            return Some(text);
        }
    }
    if let Some(rich) = find_deep(tx, "rich") {
        let text = join_text(rich);
        if !text.is_empty() {
            return Some(text);
        }
    }
    None
}

pub(super) fn title_typography(chart: &Element) -> Result<Option<TitleTypo>, Error> {
    if title_text(chart).is_none() {
        return Ok(None);
    }
    let Some(title) = chart.child_by_local("title") else {
        return Ok(None);
    };
    let unsupported = || Error::with_path(ErrorKind::ChartImageUnsupported, "id");
    let mut props: Vec<&Element> = Vec::new();
    if let Some(txpr) = title.child_by_local("txPr") {
        collect_title_props(txpr, &mut props)?;
    }
    if let Some(tx) = title.child_by_local("tx") {
        if let Some(rich) = find_deep(tx, "rich") {
            collect_title_props(rich, &mut props)?;
        }
    }
    let mut typo: Option<TitleTypo> = None;
    for prop in props {
        if let Some(parsed) = parse_title_rpr(prop)? {
            if let Some(existing) = typo.as_ref() {
                if existing != &parsed {
                    return Err(unsupported());
                }
            } else {
                typo = Some(parsed);
            }
        }
    }
    Ok(typo)
}

pub(super) fn collect_title_props<'a>(root: &'a Element, out: &mut Vec<&'a Element>) -> Result<(), Error> {
    let unsupported = || Error::with_path(ErrorKind::ChartImageUnsupported, "id");
    if root
        .child_by_local("lstStyle")
        .is_some_and(|n| n.element_children().next().is_some())
    {
        return Err(unsupported());
    }
    let paragraphs: Vec<&Element> = if root.local_name() == "p" {
        vec![root]
    } else {
        root.children_by_local("p")
    };
    if paragraphs.len() > 1 {
        return Err(unsupported());
    }
    for p in paragraphs {
        if let Some(def) = p
            .child_by_local("pPr")
            .and_then(|n| n.child_by_local("defRPr"))
        {
            out.push(def);
        }
        for r in p.children_by_local("r") {
            if let Some(rpr) = r.child_by_local("rPr") {
                out.push(rpr);
            }
        }
    }
    Ok(())
}

pub(super) fn parse_title_rpr(p: &Element) -> Result<Option<TitleTypo>, Error> {
    let unsupported = || Error::with_path(ErrorKind::ChartImageUnsupported, "id");
    let allowed = [
        "sz", "b", "i", "u", "strike", "baseline", "spc", "cap", "normalizeH",
        "kumimoji", "lang", "altLang", "dirty", "err", "smtClean", "smtId",
        "noProof", "bmk",
    ];
    for (name, _) in p.attrs.iter() {
        if name.starts_with("xmlns") {
            continue;
        }
        let local = name.rsplit(':').next().unwrap_or(name.as_str());
        if !allowed.contains(&local) {
            return Err(unsupported());
        }
    }
    if p.attr_local("cap").is_some_and(|v| v != "none")
        || p.attr_local("spc")
            .and_then(|v| v.parse::<i32>().ok())
            .is_some_and(|v| v != 0)
        || matches!(p.attr_local("normalizeH"), Some("1" | "true" | "on"))
        || matches!(p.attr_local("kumimoji"), Some("1" | "true" | "on"))
        || p.attr_local("u").is_some_and(|v| v != "none")
        || p.attr_local("strike").is_some_and(|v| v != "noStrike")
        || p.attr_local("baseline")
            .and_then(|v| v.parse::<i32>().ok())
            .is_some_and(|v| v != 0)
    {
        return Err(unsupported());
    }
    let Some(sz_raw) = p.attr_local("sz") else {
        // lang-only / empty defRPr: not an explicit typography contract.
        if p.attr_local("b").is_some() || p.attr_local("i").is_some() {
            // Bold/italic without sz is not uniquely sized; do not invent 10pt.
            // Also do not silently drop the explicit attribute.
            return Err(unsupported());
        }
        if p.child_by_local("latin").is_some() || p.child_by_local("ea").is_some() {
            return Err(unsupported());
        }
        return Ok(None);
    };
    let sz = sz_raw.parse::<f64>().map_err(|_| unsupported())?;
    if !sz.is_finite() || !(100.0..=400000.0).contains(&sz) || sz > 40900.0 {
        return Err(unsupported());
    }
    let bold = match p.attr_local("b") {
        None => false,
        Some("1" | "true" | "on") => true,
        Some("0" | "false" | "off") => false,
        Some(_) => return Err(unsupported()),
    };
    let italic = match p.attr_local("i") {
        None => false,
        Some("1" | "true" | "on") => true,
        Some("0" | "false" | "off") => false,
        Some(_) => return Err(unsupported()),
    };
    let latin = p
        .child_by_local("latin")
        .and_then(|n| n.attr_local("typeface"))
        .filter(|n| !n.trim().is_empty());
    let ea = p
        .child_by_local("ea")
        .and_then(|n| n.attr_local("typeface"))
        .filter(|n| !n.trim().is_empty());
    if matches!((latin, ea), (Some(a), Some(b)) if a != b) {
        return Err(unsupported());
    }
    if let Some(name) = latin.or(ea) {
        if name.starts_with('+') {
            return Err(unsupported());
        }
    }
    for child in p.element_children() {
        match child.local_name() {
            "latin" | "ea" | "extLst" => {}
            "cs" => {
                let cs = child.attr_local("typeface").filter(|n| !n.trim().is_empty());
                if cs.is_some() && cs != latin.or(ea) {
                    return Err(unsupported());
                }
            }
            "effectLst" | "effectDag" if child.element_children().next().is_none() => {}
            _ => return Err(unsupported()),
        }
    }
    Ok(Some(TitleTypo {
        pt: sz / 100.0,
        bold,
        italic,
        name: latin.or(ea).map(|s| s.to_string()),
    }))
}

pub(super) fn title_face_px(typo: Option<&TitleTypo>, dpi: RenderDpi, label_px: f32) -> f32 {
    typo.map(|t| dpi.pt_to_px_font(t.pt) as f32).unwrap_or(label_px)
}

pub(super) fn open_title_face(
    typo: Option<&TitleTypo>,
    px: f32,
    dpi: RenderDpi,
) -> Option<super::super::chart_gdi::Face> {
    let Some(typo) = typo else {
        return super::super::chart_gdi::Face::open(px, true);
    };
    if let Some(name) = typo.name.as_deref() {
        return super::super::chart_gdi::Face::open_cell_dpi(px, typo.bold, typo.italic, name, dpi.get());
    }
    let face = super::super::chart_gdi::Face::open_dpi(px, typo.bold, dpi.get());
    if typo.italic {
        #[cfg(windows)]
        {
            face.and_then(|f| {
                super::super::chart_gdi::Face::open_cell_dpi(
                    px,
                    typo.bold,
                    true,
                    f.family_name(),
                    dpi.get(),
                )
            })
        }
        #[cfg(not(windows))]
        {
            face
        }
    } else {
        face
    }
}

pub(super) fn text_width(
    font: Option<&Font>,
    gdi: Option<&super::super::chart_gdi::Face>,
    text: &str,
    px: f32,
) -> f64 {
    if let Some(face) = gdi {
        return face.width(text);
    }
    let Some(font) = font else {
        return text.chars().count() as f64 * f64::from(px) * 0.5;
    };
    text.chars()
        .map(|ch| f64::from(font.metrics(ch, px).advance_width))
        .sum()
}

pub(super) fn text_height(font: Option<&Font>, px: f32) -> f64 {
    font.and_then(|font| font.horizontal_line_metrics(px))
        .map(|metrics| f64::from(metrics.new_line_size))
        .unwrap_or(f64::from(px))
}

pub(super) fn draw_text(
    canvas: &mut Canvas,
    font: Option<&Font>,
    gdi: Option<&super::super::chart_gdi::Face>,
    text: &str,
    x: f64,
    top: f64,
    px: f32,
    color: [u8; 3],
) {
    if let Some(face) = gdi {
        face.draw(text, x, top, color, |px, py, rgb, cov| {
            canvas.blend(px, py, rgb, cov);
        });
        return;
    }
    let Some(font) = font else {
        return;
    };
    let metrics = font.horizontal_line_metrics(px);
    let ascent = metrics
        .map(|item| f64::from(item.ascent))
        .unwrap_or(f64::from(px));
    let baseline = top + ascent;
    let mut cursor = x;
    for ch in text.chars() {
        let (glyph, bitmap) = font.rasterize(ch, px);
        let left = cursor + f64::from(glyph.xmin);
        let glyph_top = baseline - f64::from(glyph.height as i32 + glyph.ymin);
        for row in 0..glyph.height {
            for col in 0..glyph.width {
                let cov = bitmap[row * glyph.width + col];
                canvas.blend(
                    left.round() as i32 + col as i32,
                    glyph_top.round() as i32 + row as i32,
                    color,
                    cov,
                );
            }
        }
        cursor += f64::from(glyph.advance_width);
    }
}
