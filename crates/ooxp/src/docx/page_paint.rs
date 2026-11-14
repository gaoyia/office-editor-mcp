//! 把 docx 里没有可见内容的一页画成白 PNG。
//! 宽高按节的 `w:pgSz`，96 dpi，twip×96/1440 四舍五入。
//! 有字、表、图、页眉页脚或还没对齐的行距时不画，避免拿近似图冒充。

use crate::error::{Error, ErrorKind, IdReason};
use crate::opc::{self, Package};
use crate::xml::{self, Element, Node};

use super::export::ExportedMedia;
use super::styles;

/// Calibri 11 磅单倍行距。hhea、typo、win 的行距单位都是 2500/2048。
/// 分子是 11 磅 × 20 twip × 2500。
const LINE_NUM: u64 = 550_000;
const LINE_DEN: u64 = 2048;
const MAX_PX: u64 = 4096;

pub fn export_page_image(bytes: &[u8], id: &str) -> Result<ExportedMedia, Error> {
    let page = parse_page_id(id)?;
    let pkg = Package::open_bytes(bytes.to_vec())?;
    let doc_part = opc::office_document_part(&pkg)?;
    let doc = xml::parse(pkg.get(&doc_part)?)?;
    let layout = layout_of(&pkg, &doc_part, &doc.root)?;
    if page > layout.pages {
        return Err(Error::with_path(ErrorKind::PageNotFound { n: page }, "id"));
    }
    let png = encode_png(layout.width, layout.height)?;
    Ok(ExportedMedia {
        bytes: png,
        content_type: Some("image/png".to_string()),
    })
}

struct Layout {
    width: u32,
    height: u32,
    pages: u32,
}

fn layout_of(pkg: &Package, doc_part: &str, root: &Element) -> Result<Layout, Error> {
    if root.element_children().any(|el| el.local_name() != "body") {
        return Err(unsupported());
    }
    let body = root.child_by_local("body").ok_or_else(unsupported)?;
    let (paras, sect) = blank_body(body)?;
    if !sect_is_plain(sect) || !uses_calibri_11(pkg, doc_part)? {
        return Err(unsupported());
    }
    let page_w = page_twip(sect, "w")?;
    let page_h = page_twip(sect, "h")?;
    let top = margin(sect, "top")?;
    let bottom = margin(sect, "bottom")?;
    let body_h = page_h - top - bottom;
    if body_h <= 0 {
        return Err(unsupported());
    }
    let width = twip_px(page_w)?;
    let height = twip_px(page_h)?;
    let pages = page_count(paras, body_h as u64)?;
    Ok(Layout {
        width,
        height,
        pages,
    })
}

fn blank_body(body: &Element) -> Result<(u32, &Element), Error> {
    let mut paras = 0u32;
    let mut sect: Option<&Element> = None;
    let children: Vec<&Element> = body.element_children().collect();
    for (index, child) in children.iter().copied().enumerate() {
        match child.local_name() {
            "p" => {
                if sect.is_some() || !paragraph_empty(child) {
                    return Err(unsupported());
                }
                paras = paras.saturating_add(1);
            }
            "sectPr" => {
                if sect.is_some() || index + 1 != children.len() {
                    return Err(unsupported());
                }
                sect = Some(child);
            }
            _ => return Err(unsupported()),
        }
    }
    let Some(sect) = sect else {
        return Err(unsupported());
    };
    Ok((paras, sect))
}

fn paragraph_empty(p: &Element) -> bool {
    p.children.iter().all(|node| match node {
        Node::Element(_) => false,
        Node::Text(text) => text.trim().is_empty(),
        Node::Raw(_) => true,
    })
}

fn sect_is_plain(sect: &Element) -> bool {
    sect.element_children()
        .all(|child| match child.local_name() {
            "pgSz" | "pgMar" | "type" | "titlePg" | "formProt" | "vAlign" | "paperSrc"
            | "textDirection" | "bidi" | "rtlGutter" | "pgNumType" => true,
            "cols" => {
                child.attr_local("num").unwrap_or("1") == "1"
                    && child.element_children().next().is_none()
            }
            _ => false,
        })
}

fn uses_calibri_11(pkg: &Package, doc_part: &str) -> Result<bool, Error> {
    let Some(part) = styles::styles_part(pkg, doc_part)? else {
        return Ok(false);
    };
    let styles = xml::parse(pkg.get(&part)?)?;
    let root = &styles.root;
    let Some(defaults) = root.child_by_local("docDefaults") else {
        return Ok(false);
    };
    if let Some(ppr_default) = defaults.child_by_local("pPrDefault") {
        if let Some(ppr) = ppr_default.child_by_local("pPr") {
            if ppr.element_children().next().is_some() {
                return Ok(false);
            }
        }
        if ppr_default
            .element_children()
            .any(|el| el.local_name() != "pPr")
        {
            return Ok(false);
        }
    }
    let Some(rpr) = defaults
        .child_by_local("rPrDefault")
        .and_then(|el| el.child_by_local("rPr"))
    else {
        return Ok(false);
    };
    if rpr.child_by_local("sz").and_then(|el| el.attr_local("val")) != Some("22") {
        return Ok(false);
    }
    if rpr
        .child_by_local("rFonts")
        .and_then(|el| el.attr_local("ascii"))
        != Some("Calibri")
    {
        return Ok(false);
    }
    let Some(normal) = root.element_children().find(|el| {
        el.local_name() == "style"
            && el.attr_local("type") == Some("paragraph")
            && el.attr_local("default") == Some("1")
    }) else {
        return Ok(false);
    };
    if let Some(ppr) = normal.child_by_local("pPr") {
        if ppr.element_children().next().is_some() {
            return Ok(false);
        }
    }
    Ok(true)
}

fn page_twip(sect: &Element, local: &str) -> Result<i64, Error> {
    let raw = sect
        .child_by_local("pgSz")
        .and_then(|el| el.attr_local(local))
        .ok_or_else(unsupported)?;
    let n: i64 = raw.parse().map_err(|_| unsupported())?;
    if n <= 0 {
        return Err(unsupported());
    }
    Ok(n)
}

fn margin(sect: &Element, local: &str) -> Result<i64, Error> {
    let Some(mar) = sect.child_by_local("pgMar") else {
        return Ok(1440);
    };
    match mar.attr_local(local) {
        None => Ok(1440),
        Some(raw) => raw.parse().map_err(|_| unsupported()),
    }
}

fn page_count(paras: u32, body_twips: u64) -> Result<u32, Error> {
    let per = (body_twips.saturating_mul(LINE_DEN) / LINE_NUM) as u32;
    if per == 0 {
        return Err(unsupported());
    }
    let paras = paras.max(1);
    Ok(paras.div_ceil(per))
}

/// `twip × 96 / 1440` 就是 `twip / 15`。正数四舍五入用 `(twip + 7) / 15`。
fn twip_px(twip: i64) -> Result<u32, Error> {
    let n = u64::try_from(twip).map_err(|_| unsupported())?;
    let px = (n + 7) / 15;
    if !(1..=MAX_PX).contains(&px) {
        return Err(unsupported());
    }
    Ok(px as u32)
}

fn encode_png(w: u32, h: u32) -> Result<Vec<u8>, Error> {
    let rgb = vec![255u8; (w as usize) * (h as usize) * 3];
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, w, h);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(|_| unsupported())?;
    writer.write_image_data(&rgb).map_err(|_| unsupported())?;
    drop(writer);
    Ok(out)
}

fn parse_page_id(id: &str) -> Result<u32, Error> {
    let Some(rest) = id.strip_prefix("page:") else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::ExportPageNeedsPage,
            },
            "id",
        ));
    };
    if rest.contains('/') || rest.is_empty() {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::ExportPageNeedsPage,
            },
            "id",
        ));
    }
    let n: u32 = rest.parse().map_err(|_| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::ExportPageNeedsPage,
            },
            "id",
        )
    })?;
    if n == 0 {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::ExportPageNeedsPage,
            },
            "id",
        ));
    }
    Ok(n)
}

fn unsupported() -> Error {
    Error::with_path(ErrorKind::PageImageUnsupported, "id")
}
