//! 幻灯片只读栅格化。先画本页纯色页底，再按文档顺序画 PNG 和没有描边的实心矩形。
//! 不写回包，像素不进 inspect。文字、其它形状和图表留到后面。

use std::io::Cursor;

use crate::error::{Error, ErrorKind, IdReason};
use crate::opc::{self, Package};
use crate::xml::Element;

use super::picture;
use super::shape;
use super::ExportedMedia;

const EMU_PER_INCH: f64 = 914_400.0;
const DPI: f64 = 96.0;
const MAX_PX: u32 = 4096;

pub fn export_slide_image(bytes: &[u8], id: &str) -> Result<ExportedMedia, Error> {
    let n = parse_slide_id(id)?;
    let pkg = Package::open_bytes(bytes.to_vec())?;
    let pres_part = opc::office_document_part(&pkg)?;
    let pres = super::list_pin::parse_part(&pkg, &pres_part)?;
    let slides = shape::list_slides(&pkg, &pres_part, &pres.root)?;
    let slide = slides
        .iter()
        .find(|s| s.index == n)
        .ok_or_else(|| Error::with_path(ErrorKind::SlideNotFound { n: n as u32 }, "id"))?;
    let doc = super::list_pin::parse_part(&pkg, &slide.part)?;
    let (cx, cy) = picture::canvas_size(&pres.root);
    let width = emu_span(cx)?;
    let height = emu_span(cy)?;
    let bg = page_color(&doc.root).ok_or_else(unsupported)?;
    let mut rgb = vec![0u8; (width as usize) * (height as usize) * 3];
    fill(&mut rgb, bg);
    paint_tree(&pkg, &slide.part, &doc.root, &mut rgb, width, height)?;
    let png = encode_png(width, height, &rgb)?;
    Ok(ExportedMedia {
        bytes: png,
        content_type: Some("image/png".to_string()),
    })
}

fn parse_slide_id(id: &str) -> Result<usize, Error> {
    let Some(rest) = id.strip_prefix("slide:") else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::ExportSlideNeedsSlide,
            },
            "id",
        ));
    };
    if rest.contains('/') || rest.is_empty() {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::ExportSlideNeedsSlide,
            },
            "id",
        ));
    }
    let n: u32 = rest.parse().map_err(|_| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SlideIndexNotInt,
            },
            "id",
        )
    })?;
    if n == 0 {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SlideIndexFromOne,
            },
            "id",
        ));
    }
    Ok(n as usize)
}

fn page_color(root: &Element) -> Option<[u8; 3]> {
    let clr = root
        .child_by_local("cSld")?
        .child_by_local("bg")?
        .child_by_local("bgPr")?
        .child_by_local("solidFill")?
        .child_by_local("srgbClr")?;
    for child in clr.element_children() {
        if child.local_name() != "alpha" {
            return None;
        }
        let val = child.attr("val").unwrap_or("");
        if val != "100000" {
            return None;
        }
    }
    parse_rgb(clr.attr("val")?)
}

fn paint_tree(
    pkg: &Package,
    slide_part: &str,
    root: &Element,
    rgb: &mut [u8],
    width: u32,
    height: u32,
) -> Result<(), Error> {
    let Some(tree) = root
        .child_by_local("cSld")
        .and_then(|c| c.child_by_local("spTree"))
    else {
        return Ok(());
    };
    visit(pkg, slide_part, tree, false, rgb, width, height)
}

fn visit(
    pkg: &Package,
    slide_part: &str,
    el: &Element,
    grouped: bool,
    rgb: &mut [u8],
    width: u32,
    height: u32,
) -> Result<(), Error> {
    for child in el.element_children() {
        match child.local_name() {
            "pic" => {
                if grouped || transformed(child) || is_media(child) {
                    return Err(unsupported());
                }
                blit_pic(pkg, slide_part, child, rgb, width, height)?;
            }
            "sp" if !grouped => paint_solid_rect(child, rgb, width, height),
            "grpSp" => visit(pkg, slide_part, child, true, rgb, width, height)?,
            "AlternateContent" => {
                if let Some(choice) = child.child_by_local("Choice") {
                    visit(pkg, slide_part, choice, grouped, rgb, width, height)?;
                }
            }
            _ => {}
        }
    }
    Ok(())
}

/// 只画轴对齐、不透明、没有描边的矩形。对不上的形状留空，留给后面的绘制。
fn paint_solid_rect(sp: &Element, rgb: &mut [u8], width: u32, height: u32) {
    let Some((color, x, y, w, h)) = plain_rect(sp) else {
        return;
    };
    fill_rect(rgb, width, height, x, y, w, h, color);
}

fn plain_rect(sp: &Element) -> Option<([u8; 3], i32, i32, i32, i32)> {
    let sp_pr = sp.child_by_local("spPr")?;
    if sp_pr.child_by_local("effectLst").is_some()
        || sp_pr.child_by_local("scene3d").is_some()
        || sp_pr.child_by_local("sp3d").is_some()
    {
        return None;
    }
    let xfrm = sp_pr.child_by_local("xfrm")?;
    if crate::xml::attr_is_true(xfrm, "flipH") || crate::xml::attr_is_true(xfrm, "flipV") {
        return None;
    }
    if xfrm
        .attr("rot")
        .is_some_and(|raw| raw.parse::<i64>().ok().unwrap_or(1) != 0)
    {
        return None;
    }
    let prst = sp_pr.child_by_local("prstGeom")?.attr("prst")?;
    if prst != "rect" {
        return None;
    }
    if !line_is_clear(sp_pr) {
        return None;
    }
    let color = opaque_rgb(sp_pr.child_by_local("solidFill")?)?;
    let (x, y, cx, cy) = xfrm_box(xfrm);
    let w = emu_px(cx);
    let h = emu_px(cy);
    if w <= 0 || h <= 0 {
        return None;
    }
    Some((color, emu_px(x), emu_px(y), w, h))
}

fn line_is_clear(sp_pr: &Element) -> bool {
    let Some(ln) = sp_pr.child_by_local("ln") else {
        return true;
    };
    ln.element_children()
        .all(|child| child.local_name() == "noFill")
        && ln.child_by_local("noFill").is_some()
}

fn opaque_rgb(fill: &Element) -> Option<[u8; 3]> {
    let clr = fill.child_by_local("srgbClr")?;
    for child in clr.element_children() {
        if child.local_name() != "alpha" {
            return None;
        }
        if child.attr("val").unwrap_or("") != "100000" {
            return None;
        }
    }
    parse_rgb(clr.attr("val")?)
}

fn xfrm_box(xfrm: &Element) -> (i64, i64, i64, i64) {
    let off = xfrm.child_by_local("off");
    let ext = xfrm.child_by_local("ext");
    let num = |el: Option<&Element>, name: &str| {
        el.and_then(|node| node.attr(name))
            .and_then(|raw| raw.parse().ok())
            .unwrap_or(0)
    };
    (num(off, "x"), num(off, "y"), num(ext, "cx"), num(ext, "cy"))
}

fn transformed(pic: &Element) -> bool {
    if pic
        .child_by_local("blipFill")
        .and_then(|fill| fill.child_by_local("srcRect"))
        .is_some()
    {
        return true;
    }
    let Some(xfrm) = pic
        .child_by_local("spPr")
        .and_then(|sp| sp.child_by_local("xfrm"))
    else {
        return false;
    };
    if crate::xml::attr_is_true(xfrm, "flipH") || crate::xml::attr_is_true(xfrm, "flipV") {
        return true;
    }
    match xfrm.attr("rot") {
        Some(raw) => raw.parse::<i64>().ok().unwrap_or(1) != 0,
        None => false,
    }
}

fn is_media(pic: &Element) -> bool {
    pic.child_by_local("nvPicPr")
        .and_then(|nv| nv.child_by_local("nvPr"))
        .is_some_and(|pr| {
            pr.child_by_local("videoFile").is_some() || pr.child_by_local("audioFile").is_some()
        })
}

fn blit_pic(
    pkg: &Package,
    slide_part: &str,
    pic: &Element,
    rgb: &mut [u8],
    width: u32,
    height: u32,
) -> Result<(), Error> {
    let Some(rid) = pic
        .child_by_local("blipFill")
        .and_then(|fill| fill.child_by_local("blip"))
        .and_then(|blip| blip.attr_local("embed"))
        .filter(|s| !s.is_empty())
    else {
        return Err(unsupported());
    };
    let part = opc::resolve_id(pkg, slide_part, rid)?;
    let bytes = pkg.get(&part)?;
    if !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Err(unsupported());
    }
    let image = decode_png(bytes)?;
    let (x, y, cx, cy) = pic_box(pic);
    let dest_w = emu_px(cx);
    let dest_h = emu_px(cy);
    if dest_w <= 0 || dest_h <= 0 {
        return Err(unsupported());
    }
    let dest_x = emu_px(x);
    let dest_y = emu_px(y);
    paint_image(
        rgb,
        width,
        height,
        dest_x,
        dest_y,
        dest_w as u32,
        dest_h as u32,
        &image,
    );
    Ok(())
}

fn pic_box(pic: &Element) -> (i64, i64, i64, i64) {
    pic.child_by_local("spPr")
        .and_then(|sp| sp.child_by_local("xfrm"))
        .map(xfrm_box)
        .unwrap_or((0, 0, 0, 0))
}

struct Rgba {
    w: u32,
    h: u32,
    px: Vec<u8>,
}

fn decode_png(bytes: &[u8]) -> Result<Rgba, Error> {
    let decoder = png::Decoder::new(Cursor::new(bytes));
    let mut reader = decoder.read_info().map_err(|_| unsupported())?;
    if reader.info().bit_depth != png::BitDepth::Eight {
        return Err(unsupported());
    }
    let mut buf = vec![0; reader.output_buffer_size()];
    let frame = reader.next_frame(&mut buf).map_err(|_| unsupported())?;
    if frame.width == 0 || frame.height == 0 || frame.width > MAX_PX || frame.height > MAX_PX {
        return Err(unsupported());
    }
    let raw = &buf[..frame.buffer_size()];
    let px = match frame.color_type {
        png::ColorType::Rgb => {
            let mut out = Vec::with_capacity(raw.len() / 3 * 4);
            for chunk in raw.chunks_exact(3) {
                out.extend_from_slice(&[chunk[0], chunk[1], chunk[2], 255]);
            }
            out
        }
        png::ColorType::Rgba => raw.to_vec(),
        _ => return Err(unsupported()),
    };
    if px.len() != (frame.width as usize) * (frame.height as usize) * 4 {
        return Err(unsupported());
    }
    Ok(Rgba {
        w: frame.width,
        h: frame.height,
        px,
    })
}

fn paint_image(
    rgb: &mut [u8],
    width: u32,
    height: u32,
    dest_x: i32,
    dest_y: i32,
    dest_w: u32,
    dest_h: u32,
    image: &Rgba,
) {
    for row in 0..dest_h {
        let y = dest_y + row as i32;
        if y < 0 || y >= height as i32 {
            continue;
        }
        let sy = ((row as u64 * image.h as u64) / dest_h as u64).min(image.h as u64 - 1) as u32;
        for col in 0..dest_w {
            let x = dest_x + col as i32;
            if x < 0 || x >= width as i32 {
                continue;
            }
            let sx = ((col as u64 * image.w as u64) / dest_w as u64).min(image.w as u64 - 1) as u32;
            let si = ((sy as usize) * (image.w as usize) + sx as usize) * 4;
            let src = [
                image.px[si],
                image.px[si + 1],
                image.px[si + 2],
                image.px[si + 3],
            ];
            let di = ((y as usize) * (width as usize) + x as usize) * 3;
            let dst = [rgb[di], rgb[di + 1], rgb[di + 2]];
            let mixed = over(dst, src);
            rgb[di..di + 3].copy_from_slice(&mixed);
        }
    }
}

fn over(dst: [u8; 3], src: [u8; 4]) -> [u8; 3] {
    let a = u16::from(src[3]);
    if a == 255 {
        return [src[0], src[1], src[2]];
    }
    if a == 0 {
        return dst;
    }
    let inv = 255 - a;
    [
        ((u16::from(src[0]) * a + u16::from(dst[0]) * inv) / 255) as u8,
        ((u16::from(src[1]) * a + u16::from(dst[1]) * inv) / 255) as u8,
        ((u16::from(src[2]) * a + u16::from(dst[2]) * inv) / 255) as u8,
    ]
}

fn fill(rgb: &mut [u8], color: [u8; 3]) {
    for px in rgb.chunks_exact_mut(3) {
        px.copy_from_slice(&color);
    }
}

fn fill_rect(
    rgb: &mut [u8],
    width: u32,
    height: u32,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    color: [u8; 3],
) {
    for row in 0..h {
        let py = y + row;
        if py < 0 || py >= height as i32 {
            continue;
        }
        for col in 0..w {
            let px = x + col;
            if px < 0 || px >= width as i32 {
                continue;
            }
            let i = (py as usize * width as usize + px as usize) * 3;
            rgb[i..i + 3].copy_from_slice(&color);
        }
    }
}

fn emu_span(emu: i64) -> Result<u32, Error> {
    let px = emu_px(emu);
    if px < 1 || px > MAX_PX as i32 {
        return Err(unsupported());
    }
    Ok(px as u32)
}

fn emu_px(emu: i64) -> i32 {
    let px = (emu as f64) * DPI / EMU_PER_INCH;
    if !px.is_finite() {
        return 0;
    }
    px.round().clamp(-100_000.0, 100_000.0) as i32
}

fn parse_rgb(raw: &str) -> Option<[u8; 3]> {
    if raw.len() != 6 {
        return None;
    }
    let n = u32::from_str_radix(raw, 16).ok()?;
    Some([
        ((n >> 16) & 0xff) as u8,
        ((n >> 8) & 0xff) as u8,
        (n & 0xff) as u8,
    ])
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
    Error::with_path(ErrorKind::SlideImageUnsupported, "id")
}
