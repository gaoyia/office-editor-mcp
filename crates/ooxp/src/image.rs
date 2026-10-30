//! 图文件头：EMF / WMF / TIFF，以及 PNG / JPEG / GIF / BMP 的尺寸。
//! 不认 SVGZ / WebP。SVG 由 `svg` 认。

use crate::error::{Error, ErrorKind};
use crate::opc::Relationships;

/// `[Content_Types].xml` 的 Default `emf`。
pub const CT_EMF: &str = "image/x-emf";
/// `[Content_Types].xml` 的 Default `wmf`。
pub const CT_WMF: &str = "image/x-wmf";
/// `[Content_Types].xml` 的 Default `tif`。
pub const CT_TIFF: &str = "image/tiff";

/// 嗅探到的图种类。写出扩展名用这里的 `ext`。
pub struct Metafile {
    pub ext: &'static str,
    pub content_type: &'static str,
    pub cx: i64,
    pub cy: i64,
}

/// 按文件头认 EMF / WMF / TIFF。读不到尺寸则 1 英寸见方。
pub fn classify(bytes: &[u8]) -> Option<Metafile> {
    if looks_like_emf(bytes) {
        let (cx, cy) = emf_size(bytes).unwrap_or((914400, 914400));
        return Some(Metafile {
            ext: "emf",
            content_type: CT_EMF,
            cx,
            cy,
        });
    }
    if looks_like_wmf(bytes) {
        let (cx, cy) = wmf_size(bytes).unwrap_or((914400, 914400));
        return Some(Metafile {
            ext: "wmf",
            content_type: CT_WMF,
            cx,
            cy,
        });
    }
    if looks_like_tiff(bytes) {
        let (cx, cy) = tiff_size(bytes).unwrap_or((914400, 914400));
        return Some(Metafile {
            ext: "tif",
            content_type: CT_TIFF,
            cx,
            cy,
        });
    }
    None
}

/// inspect：有 `svgBlip` 则 SVG；否则看 image 关系 Target 扩展名。
pub fn inspect_content_type(
    has_svg_blip: bool,
    embed_rid: Option<&str>,
    rels: Option<&Relationships>,
) -> Option<String> {
    if has_svg_blip {
        return Some(crate::svg::CT.to_string());
    }
    let rid = embed_rid?;
    let target = rels?.find_by_id(rid)?.target.as_str();
    content_type_from_name(target).map(str::to_string)
}

/// `.emf` / `.wmf` / `.tif` / `.tiff` 对应 Content Type。其它扩展名为空。
pub fn content_type_from_name(name: &str) -> Option<&'static str> {
    let base = name.rsplit(['/', '\\']).next().unwrap_or(name);
    let ext = base.rsplit('.').next()?.to_ascii_lowercase();
    match ext.as_str() {
        "emf" => Some(CT_EMF),
        "wmf" => Some(CT_WMF),
        "tif" | "tiff" => Some(CT_TIFF),
        _ => None,
    }
}

fn looks_like_emf(bytes: &[u8]) -> bool {
    if bytes.len() < 44 {
        return false;
    }
    u32_le(bytes, 0) == Some(1) && u32_le(bytes, 40) == Some(0x464D4520)
}

fn looks_like_wmf(bytes: &[u8]) -> bool {
    if bytes.len() >= 4 && bytes.starts_with(&[0xD7, 0xCD, 0xC6, 0x9A]) {
        return true;
    }
    let Some(mt_type) = u16_le(bytes, 0) else {
        return false;
    };
    let Some(header_size) = u16_le(bytes, 2) else {
        return false;
    };
    let Some(version) = u16_le(bytes, 4) else {
        return false;
    };
    (mt_type == 1 || mt_type == 2) && header_size == 9 && (version == 0x0100 || version == 0x0300)
}

fn looks_like_tiff(bytes: &[u8]) -> bool {
    bytes.len() >= 8 && (bytes.starts_with(b"II*\0") || bytes.starts_with(b"MM\0*"))
}

fn emf_size(bytes: &[u8]) -> Option<(i64, i64)> {
    if bytes.len() < 88 {
        return None;
    }
    let fl = i32_le(bytes, 24)?;
    let ft = i32_le(bytes, 28)?;
    let fr = i32_le(bytes, 32)?;
    let fb = i32_le(bytes, 36)?;
    let fw = (fr as i64).saturating_sub(fl as i64);
    let fh = (fb as i64).saturating_sub(ft as i64);
    if fw > 0 && fh > 0 {
        return Some((himetric_to_emu(fw), himetric_to_emu(fh)));
    }
    let bl = i32_le(bytes, 8)?;
    let bt = i32_le(bytes, 12)?;
    let br = i32_le(bytes, 16)?;
    let bb = i32_le(bytes, 20)?;
    let bw = (br as i64).saturating_sub(bl as i64).saturating_add(1);
    let bh = (bb as i64).saturating_sub(bt as i64).saturating_add(1);
    if bw > 0 && bh > 0 {
        return Some((px_to_emu(bw as u32), px_to_emu(bh as u32)));
    }
    None
}

fn wmf_size(bytes: &[u8]) -> Option<(i64, i64)> {
    if bytes.len() < 16 || !bytes.starts_with(&[0xD7, 0xCD, 0xC6, 0x9A]) {
        return None;
    }
    let left = i16_le(bytes, 6)? as i64;
    let top = i16_le(bytes, 8)? as i64;
    let right = i16_le(bytes, 10)? as i64;
    let bottom = i16_le(bytes, 12)? as i64;
    let inch = u16_le(bytes, 14)? as i64;
    if inch <= 0 {
        return None;
    }
    let w = (right - left).saturating_mul(914400) / inch;
    let h = (bottom - top).saturating_mul(914400) / inch;
    if w <= 0 || h <= 0 {
        return None;
    }
    Some((w, h))
}

fn tiff_size(bytes: &[u8]) -> Option<(i64, i64)> {
    let le = bytes.starts_with(b"II*\0");
    let be = bytes.starts_with(b"MM\0*");
    if !le && !be {
        return None;
    }
    let ifd = u32_at(bytes, 4, le)? as usize;
    let count = u16_at(bytes, ifd, le)? as usize;
    let mut w = None;
    let mut h = None;
    for i in 0..count {
        let off = ifd.checked_add(2)?.checked_add(i.checked_mul(12)?)?;
        let tag = u16_at(bytes, off, le)?;
        let typ = u16_at(bytes, off + 2, le)?;
        let val = match typ {
            3 => u16_at(bytes, off + 8, le)? as u32,
            4 => u32_at(bytes, off + 8, le)?,
            _ => continue,
        };
        match tag {
            256 => w = Some(val),
            257 => h = Some(val),
            _ => {}
        }
    }
    Some((px_to_emu(w?), px_to_emu(h?)))
}

fn himetric_to_emu(h: i64) -> i64 {
    h.saturating_mul(360).max(1)
}

pub(crate) fn px_to_emu(px: u32) -> i64 {
    (px as i64).saturating_mul(914400) / 96
}

/// 栅格图或 EMF / WMF / TIFF。认不出则 `None`，调用方再试 SVG。
pub(crate) struct Raster {
    pub ext: &'static str,
    pub content_type: &'static str,
    pub cx: i64,
    pub cy: i64,
}

pub(crate) fn raster(bytes: &[u8]) -> Option<Raster> {
    if bytes.len() >= 8 && bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) {
        let (cx, cy) = png_size(bytes).unwrap_or((914400, 914400));
        return Some(Raster {
            ext: "png",
            content_type: "image/png",
            cx,
            cy,
        });
    }
    if bytes.len() >= 3 && bytes[0] == 0xFF && bytes[1] == 0xD8 && bytes[2] == 0xFF {
        let (cx, cy) = jpeg_size(bytes).unwrap_or((914400, 914400));
        return Some(Raster {
            ext: "jpeg",
            content_type: "image/jpeg",
            cx,
            cy,
        });
    }
    if bytes.len() >= 6 && (bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a")) {
        let (cx, cy) = gif_size(bytes).unwrap_or((914400, 914400));
        return Some(Raster {
            ext: "gif",
            content_type: "image/gif",
            cx,
            cy,
        });
    }
    if bytes.len() >= 26 && bytes.starts_with(b"BM") {
        let (cx, cy) = bmp_size(bytes).unwrap_or((914400, 914400));
        return Some(Raster {
            ext: "bmp",
            content_type: "image/bmp",
            cx,
            cy,
        });
    }
    classify(bytes).map(|m| Raster {
        ext: m.ext,
        content_type: m.content_type,
        cx: m.cx,
        cy: m.cy,
    })
}

/// 调用方给的一张图。
pub struct ImageFile {
    pub bytes: Vec<u8>,
    pub ext: String,
    pub content_type: String,
    pub cx: i64,
    pub cy: i64,
    /// 有值表示这是 SVG：`bytes` 是 PNG 占位，原件在这里。
    pub svg: Option<Vec<u8>>,
}

pub fn read_image(path: &str) -> Result<ImageFile, Error> {
    image_file(crate::host_files::read_user_file(path)?)
}

fn image_file(bytes: Vec<u8>) -> Result<ImageFile, Error> {
    if let Some(r) = raster(&bytes) {
        return Ok(ImageFile {
            bytes,
            ext: r.ext.into(),
            content_type: r.content_type.into(),
            cx: r.cx,
            cy: r.cy,
            svg: None,
        });
    }
    if crate::svg::looks_like(&bytes) {
        let (cx, cy) = crate::svg::size(&bytes).unwrap_or((914400, 914400));
        return Ok(ImageFile {
            bytes: crate::ole::PREVIEW_PNG.to_vec(),
            ext: "png".into(),
            content_type: "image/png".into(),
            cx,
            cy,
            svg: Some(bytes),
        });
    }
    Err(Error::new(ErrorKind::InvalidImageFile))
}

pub(crate) fn png_size(bytes: &[u8]) -> Option<(i64, i64)> {
    if bytes.len() < 24 {
        return None;
    }
    let w = u32::from_be_bytes(bytes[16..20].try_into().ok()?);
    let h = u32::from_be_bytes(bytes[20..24].try_into().ok()?);
    Some((px_to_emu(w), px_to_emu(h)))
}

pub(crate) fn gif_size(bytes: &[u8]) -> Option<(i64, i64)> {
    if bytes.len() < 10 {
        return None;
    }
    let w = u16::from_le_bytes(bytes[6..8].try_into().ok()?) as u32;
    let h = u16::from_le_bytes(bytes[8..10].try_into().ok()?) as u32;
    Some((px_to_emu(w), px_to_emu(h)))
}

pub(crate) fn bmp_size(bytes: &[u8]) -> Option<(i64, i64)> {
    if bytes.len() < 26 {
        return None;
    }
    let w = i32::from_le_bytes(bytes[18..22].try_into().ok()?).unsigned_abs();
    let h = i32::from_le_bytes(bytes[22..26].try_into().ok()?).unsigned_abs();
    Some((px_to_emu(w), px_to_emu(h)))
}

pub(crate) fn jpeg_size(bytes: &[u8]) -> Option<(i64, i64)> {
    let mut i = 2usize;
    while i + 9 < bytes.len() {
        if bytes[i] != 0xFF {
            i += 1;
            continue;
        }
        let marker = bytes[i + 1];
        if marker == 0xD8 || marker == 0xD9 || (0xD0..=0xD7).contains(&marker) {
            i += 2;
            continue;
        }
        if i + 3 >= bytes.len() {
            break;
        }
        let len = u16::from_be_bytes([bytes[i + 2], bytes[i + 3]]) as usize;
        if matches!(marker, 0xC0..=0xC2) && i + 8 < bytes.len() {
            let h = u16::from_be_bytes([bytes[i + 5], bytes[i + 6]]) as u32;
            let w = u16::from_be_bytes([bytes[i + 7], bytes[i + 8]]) as u32;
            return Some((px_to_emu(w), px_to_emu(h)));
        }
        i += 2 + len;
    }
    None
}

fn u16_le(bytes: &[u8], off: usize) -> Option<u16> {
    Some(u16::from_le_bytes(
        bytes.get(off..off + 2)?.try_into().ok()?,
    ))
}

fn u32_le(bytes: &[u8], off: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        bytes.get(off..off + 4)?.try_into().ok()?,
    ))
}

fn i32_le(bytes: &[u8], off: usize) -> Option<i32> {
    Some(i32::from_le_bytes(
        bytes.get(off..off + 4)?.try_into().ok()?,
    ))
}

fn i16_le(bytes: &[u8], off: usize) -> Option<i16> {
    Some(i16::from_le_bytes(
        bytes.get(off..off + 2)?.try_into().ok()?,
    ))
}

fn u16_at(bytes: &[u8], off: usize, le: bool) -> Option<u16> {
    let b: [u8; 2] = bytes.get(off..off + 2)?.try_into().ok()?;
    Some(if le {
        u16::from_le_bytes(b)
    } else {
        u16::from_be_bytes(b)
    })
}

fn u32_at(bytes: &[u8], off: usize, le: bool) -> Option<u32> {
    let b: [u8; 4] = bytes.get(off..off + 4)?.try_into().ok()?;
    Some(if le {
        u32::from_le_bytes(b)
    } else {
        u32::from_be_bytes(b)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emf_header_sniff_and_frame() {
        let mut b = vec![0u8; 88];
        b[0..4].copy_from_slice(&1u32.to_le_bytes());
        b[40..44].copy_from_slice(&0x464D4520u32.to_le_bytes());
        b[32..36].copy_from_slice(&847i32.to_le_bytes());
        b[36..40].copy_from_slice(&423i32.to_le_bytes());
        let m = classify(&b).unwrap();
        assert_eq!(m.ext, "emf");
        assert_eq!(m.content_type, CT_EMF);
        assert_eq!(m.cx, 847 * 360);
        assert_eq!(m.cy, 423 * 360);
        assert_eq!(content_type_from_name("../media/image1.emf"), Some(CT_EMF));
        assert_eq!(content_type_from_name("image1.tif"), Some(CT_TIFF));
        assert_eq!(content_type_from_name("image1.tiff"), Some(CT_TIFF));
        assert_eq!(content_type_from_name("image1.wmf"), Some(CT_WMF));
    }

    #[test]
    fn placeable_wmf_size() {
        let mut aldus = Vec::new();
        aldus.extend_from_slice(&0x9AC6CDD7u32.to_le_bytes());
        aldus.extend_from_slice(&0u16.to_le_bytes());
        aldus.extend_from_slice(&0i16.to_le_bytes());
        aldus.extend_from_slice(&0i16.to_le_bytes());
        aldus.extend_from_slice(&32i16.to_le_bytes());
        aldus.extend_from_slice(&16i16.to_le_bytes());
        aldus.extend_from_slice(&96u16.to_le_bytes());
        aldus.extend_from_slice(&0u16.to_le_bytes());
        aldus.extend_from_slice(&0u16.to_le_bytes());
        let m = classify(&aldus).unwrap();
        assert_eq!(m.ext, "wmf");
        assert_eq!(m.cx, 304800);
        assert_eq!(m.cy, 152400);
    }

    #[test]
    fn tiff_ifd_size() {
        let mut b = vec![0u8; 26];
        b[0..4].copy_from_slice(b"II*\0");
        b[4..8].copy_from_slice(&8u32.to_le_bytes());
        b[8..10].copy_from_slice(&1u16.to_le_bytes());
        b[10..12].copy_from_slice(&256u16.to_le_bytes());
        b[12..14].copy_from_slice(&3u16.to_le_bytes());
        b[14..18].copy_from_slice(&1u32.to_le_bytes());
        b[18..20].copy_from_slice(&32u16.to_le_bytes());
        let m = classify(&b);
        assert!(m.is_some());
        assert_eq!(m.unwrap().ext, "tif");
    }

    #[test]
    fn png_and_webp_not_metafile() {
        assert!(classify(&[0x89, 0x50, 0x4E, 0x47]).is_none());
        assert!(classify(b"RIFF....WEBP").is_none());
        assert!(content_type_from_name("image1.png").is_none());
    }
}
