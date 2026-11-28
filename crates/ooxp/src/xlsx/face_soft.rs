//! 非 Windows（WASM / macOS / Linux）共用的软件字体。
//! 用 fontdue 画出中文和西文，位置和大小接近可读，不与 Windows GDI 逐像素锁定。
//! 内嵌回退字体是 SIL OFL 1.1 下江城黑体 400W 的字形子集，名称表沿用官方源文件。

use std::sync::{Arc, OnceLock};

#[cfg(not(target_arch = "wasm32"))]
use std::path::{Path, PathBuf};
#[cfg(not(target_arch = "wasm32"))]
use std::sync::Mutex;

use fontdue::{Font, FontSettings};

const EMBEDDED_FONT: &[u8] = include_bytes!("../../fonts/JiangChengHeiTi-400W-subset.ttf");

const DEFAULT_FACES: &[&str] = &["等线", "DengXian", "微软雅黑", "Microsoft YaHei", "Calibri"];

pub struct Face {
    font: Arc<Font>,
    #[allow(dead_code)]
    name: String,
    px: f32,
    fake_bold: bool,
    fake_italic: bool,
    ascent: f64,
    descent: f64,
    em_box: f64,
    pad: f64,
}

impl Face {
    pub fn open(px: f32, bold: bool) -> Option<Self> {
        Self::open_dpi(px, bold, 96.0)
    }

    pub fn open_dpi(px: f32, bold: bool, dpi: f64) -> Option<Self> {
        Self::open_cell_dpi(px, bold, false, "", dpi)
    }

    pub fn open_named(px: f32, bold: bool, name: &str) -> Option<Self> {
        Self::open_cell(px, bold, false, name)
    }

    pub fn open_cell(px: f32, bold: bool, italic: bool, name: &str) -> Option<Self> {
        Self::open_cell_dpi(px, bold, italic, name, 96.0)
    }

    pub fn open_cell_dpi(px: f32, bold: bool, italic: bool, name: &str, dpi: f64) -> Option<Self> {
        if !px.is_finite() || px <= 0.0 || px.round() as i32 == 0 {
            return None;
        }
        if !dpi.is_finite() || dpi <= 0.0 {
            return None;
        }
        if !name.is_empty() && !name.starts_with('+') {
            if let Some(face) = try_named(px, bold, italic, name) {
                return Some(face);
            }
            return from_embedded(px, bold, italic, name);
        }
        for fallback in DEFAULT_FACES {
            if let Some(face) = try_named(px, bold, italic, fallback) {
                return Some(face);
            }
        }
        from_embedded(px, bold, italic, "江城黑体 400W")
    }

    #[allow(dead_code)]
    pub fn family_name(&self) -> &str {
        &self.name
    }

    pub fn width(&self, text: &str) -> f64 {
        text.chars().map(|ch| self.advance(ch)).sum()
    }

    pub fn layout_width(&self, text: &str) -> f64 {
        self.width(text)
    }

    pub fn typo_width(&self, text: &str) -> f64 {
        self.width(text)
    }

    pub fn cell_pad(&self) -> f64 {
        self.pad
    }

    pub fn line_height(&self) -> f64 {
        self.ascent
    }

    pub fn descent(&self) -> f64 {
        self.descent
    }

    pub fn cell_height(&self) -> Option<f64> {
        Some(self.ascent + self.descent)
    }

    pub fn em_box(&self) -> f64 {
        self.em_box
    }

    fn advance(&self, ch: char) -> f64 {
        f64::from(self.font.metrics(ch, self.px).advance_width)
    }

    pub fn draw(
        &self,
        text: &str,
        x: f64,
        top: f64,
        color: [u8; 3],
        blend: impl FnMut(i32, i32, [u8; 3], u8),
    ) {
        self.paint(text, x, top, color, blend);
    }

    pub fn draw_slide(
        &self,
        text: &str,
        x: f64,
        top: f64,
        color: [u8; 3],
        blend: impl FnMut(i32, i32, [u8; 3], u8),
    ) {
        self.paint(text, x, top, color, blend);
    }

    pub fn draw_cells(
        &self,
        text: &str,
        x: f64,
        top: f64,
        color: [u8; 3],
        blend: impl FnMut(i32, i32, [u8; 3], u8),
    ) {
        self.paint(text, x, top, color, blend);
    }

    pub fn draw_cells_rotated(
        &self,
        text: &str,
        origin_x: f64,
        origin_y: f64,
        deg: f64,
        color: [u8; 3],
        mut blend: impl FnMut(i32, i32, [u8; 3], u8),
    ) {
        if text.is_empty() {
            return;
        }
        if deg.abs() < 1e-9 {
            self.paint(text, origin_x, origin_y, color, blend);
            return;
        }
        self.paint(text, 0.0, 0.0, color, |px, py, rgb, cov| {
            let (rx, ry) = rotate_escapement(f64::from(px), f64::from(py), deg);
            blend(
                (origin_x + rx).round() as i32,
                (origin_y + ry).round() as i32,
                rgb,
                cov,
            );
        });
    }

    fn paint(
        &self,
        text: &str,
        x: f64,
        top: f64,
        color: [u8; 3],
        mut blend: impl FnMut(i32, i32, [u8; 3], u8),
    ) {
        if text.is_empty() {
            return;
        }
        let baseline = top + self.ascent;
        let mut cursor = x;
        let offsets = if self.fake_bold {
            &[0.0, 0.7][..]
        } else {
            &[0.0][..]
        };
        for ch in text.chars() {
            let (metrics, bitmap) = self.font.rasterize(ch, self.px);
            for &dx in offsets {
                for row in 0..metrics.height {
                    for col in 0..metrics.width {
                        let cov = bitmap[row * metrics.width + col];
                        if cov == 0 {
                            continue;
                        }
                        let mut gx = cursor + f64::from(metrics.xmin) + dx + f64::from(col as u32);
                        let gy = baseline - f64::from(metrics.height as i32 + metrics.ymin)
                            + f64::from(row as u32);
                        if self.fake_italic {
                            gx += (baseline - gy) * 0.2126;
                        }
                        blend(gx.round() as i32, gy.round() as i32, color, cov);
                    }
                }
            }
            cursor += f64::from(metrics.advance_width);
        }
    }
}

fn rotate_escapement(x: f64, y: f64, deg: f64) -> (f64, f64) {
    let rad = deg.to_radians();
    let (s, c) = (rad.sin(), rad.cos());
    (x * c + y * s, -x * s + y * c)
}

fn from_embedded(px: f32, bold: bool, italic: bool, name: &str) -> Option<Face> {
    from_font(embedded_font(), px, name, bold, italic)
}

fn from_font(
    font: Arc<Font>,
    px: f32,
    name: &str,
    fake_bold: bool,
    fake_italic: bool,
) -> Option<Face> {
    let line = font.horizontal_line_metrics(px)?;
    let ascent = f64::from(line.ascent);
    let descent = f64::from(line.descent).abs();
    let em_box = f64::from(line.new_line_size).max(ascent + descent);
    let zero = f64::from(font.metrics('0', px).advance_width);
    let pad = (zero * 0.5).max(1.0);
    Some(Face {
        font,
        name: name.to_string(),
        px,
        fake_bold,
        fake_italic,
        ascent,
        descent,
        em_box,
        pad,
    })
}

fn embedded_font() -> Arc<Font> {
    static FONT: OnceLock<Arc<Font>> = OnceLock::new();
    FONT.get_or_init(|| {
        let font = Font::from_bytes(EMBEDDED_FONT, FontSettings::default())
            .expect("embedded 江城黑体 400W should parse");
        Arc::new(font)
    })
    .clone()
}

fn try_named(px: f32, bold: bool, italic: bool, name: &str) -> Option<Face> {
    let (font, file_is_bold, file_is_italic) = load_system_font(name, bold, italic)?;
    from_font(
        font,
        px,
        name,
        bold && !file_is_bold,
        italic && !file_is_italic,
    )
}

#[cfg(not(target_arch = "wasm32"))]
struct FontKey {
    path: PathBuf,
    index: u32,
}

#[cfg(not(target_arch = "wasm32"))]
static SYSTEM_FONTS: OnceLock<Mutex<Vec<(FontKey, Arc<Font>)>>> = OnceLock::new();

fn load_system_font(name: &str, bold: bool, italic: bool) -> Option<(Arc<Font>, bool, bool)> {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (name, bold, italic);
        return None;
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let (path, index, file_is_bold, file_is_italic) = resolve_system_font(name, bold, italic)?;
        let cache = SYSTEM_FONTS.get_or_init(|| Mutex::new(Vec::new()));
        let mut list = cache.lock().unwrap_or_else(|err| err.into_inner());
        if let Some((_, font)) = list
            .iter()
            .find(|(key, _)| key.path == path && key.index == index)
        {
            return Some((font.clone(), file_is_bold, file_is_italic));
        }
        let bytes = std::fs::read(&path).ok()?;
        let settings = FontSettings {
            collection_index: index,
            ..FontSettings::default()
        };
        let font = Arc::new(Font::from_bytes(bytes, settings).ok()?);
        list.push((FontKey { path, index }, font.clone()));
        Some((font, file_is_bold, file_is_italic))
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn resolve_system_font(name: &str, bold: bool, italic: bool) -> Option<(PathBuf, u32, bool, bool)> {
    let stems = filename_stems(name, bold, italic);
    let files = system_font_index();
    for (stem, want_bold, want_italic) in stems {
        for (file_stem, path) in files {
            if file_stem == &stem || file_stem.starts_with(&(stem.clone() + ".")) {
                return Some((path.clone(), 0, want_bold, want_italic));
            }
        }
        for (file_stem, path) in files {
            if file_stem.contains(&stem) {
                return Some((path.clone(), 0, want_bold, want_italic));
            }
        }
    }
    None
}

#[cfg(not(target_arch = "wasm32"))]
fn filename_stems(name: &str, bold: bool, italic: bool) -> Vec<(String, bool, bool)> {
    let mut out = Vec::new();
    let push = |out: &mut Vec<(String, bool, bool)>, stem: &str, b: bool, i: bool| {
        let key = stem.to_ascii_lowercase();
        if !out.iter().any(|(s, _, _)| s == &key) {
            out.push((key, b, i));
        }
    };
    let mapped = map_family_files(name, bold, italic);
    for (stem, file_bold, file_italic) in mapped {
        push(&mut out, stem, file_bold, file_italic);
    }
    out
}

#[cfg(not(target_arch = "wasm32"))]
fn map_family_files(name: &str, bold: bool, italic: bool) -> Vec<(&'static str, bool, bool)> {
    let n = name.trim();
    let key = n.to_ascii_lowercase();
    let mut rows: Vec<(&'static str, bool, bool)> = Vec::new();
    let add = |rows: &mut Vec<(&'static str, bool, bool)>, stem: &'static str, b: bool, i: bool| {
        rows.push((stem, b, i));
    };
    if n == "等线" || key == "dengxian" {
        if bold {
            add(&mut rows, "dengb", true, false);
        }
        add(&mut rows, "deng", false, false);
        add(&mut rows, "dengxian", bold, italic);
    } else if n == "微软雅黑" || key == "microsoft yahei" {
        if bold {
            add(&mut rows, "msyhbd", true, false);
            add(&mut rows, "msyh_bold", true, false);
        }
        add(&mut rows, "msyh", false, false);
        add(&mut rows, "microsoftyahei", bold, italic);
        add(&mut rows, "msyhl", false, false);
    } else if key == "calibri" {
        if bold && italic {
            add(&mut rows, "calibriz", true, true);
        }
        if bold {
            add(&mut rows, "calibrib", true, false);
        }
        if italic {
            add(&mut rows, "calibrii", false, true);
        }
        add(&mut rows, "calibri", false, false);
    } else if key == "arial" {
        if bold && italic {
            add(&mut rows, "arialbi", true, true);
        }
        if bold {
            add(&mut rows, "arialbd", true, false);
        }
        if italic {
            add(&mut rows, "ariali", false, true);
        }
        add(&mut rows, "arial", false, false);
        add(&mut rows, "liberationsans-regular", false, false);
        add(&mut rows, "nimbussans-regular", false, false);
        add(&mut rows, "dejavusans", false, false);
    } else if key == "times new roman" || key == "times" {
        if bold {
            add(&mut rows, "timesbd", true, false);
        }
        add(&mut rows, "times", false, false);
        add(&mut rows, "liberationserif-regular", false, false);
    } else if n.contains("雅黑") {
        add(&mut rows, "msyh", false, false);
    } else if key.contains("pingfang") {
        add(&mut rows, "pingfang", false, false);
        add(&mut rows, "pingfangsc", false, false);
    } else if key.contains("noto sans sc")
        || key.contains("notosanssc")
        || key.contains("noto sans cjk sc")
    {
        add(&mut rows, "notosanssc-regular", false, false);
        add(&mut rows, "notosanscjksc-regular", false, false);
        add(&mut rows, "noto sans sc (truetype)", false, false);
        add(&mut rows, "notosanscjk-regular", false, false);
    } else if key.contains("source han sans") || n.contains("思源黑体") {
        add(&mut rows, "sourcehansanssc-regular", false, false);
        add(&mut rows, "sourcehansanscn-regular", false, false);
    } else if key.contains("wqy") || n.contains("文泉驿") {
        add(&mut rows, "wqy-microhei", false, false);
        add(&mut rows, "wqy-zenhei", false, false);
    } else if key == "stheiti" || n.contains("华文黑体") {
        add(&mut rows, "stheiti", false, false);
        add(&mut rows, "stheiti light", false, false);
        add(&mut rows, "stheiti medium", false, false);
    }
    rows
}

#[cfg(not(target_arch = "wasm32"))]
fn system_font_index() -> &'static [(String, PathBuf)] {
    static INDEX: OnceLock<Vec<(String, PathBuf)>> = OnceLock::new();
    INDEX.get_or_init(|| {
        let mut out = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for dir in font_dirs() {
            collect_fonts(&dir, &mut out, &mut seen);
        }
        out
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn font_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    let push = |dirs: &mut Vec<PathBuf>, path: PathBuf| {
        if path.is_dir() && !dirs.iter().any(|d| d == &path) {
            dirs.push(path);
        }
    };
    if let Some(windir) = std::env::var_os("WINDIR") {
        push(&mut dirs, PathBuf::from(windir).join("Fonts"));
    }
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        push(
            &mut dirs,
            PathBuf::from(local)
                .join("Microsoft")
                .join("Windows")
                .join("Fonts"),
        );
    }
    if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
        push(&mut dirs, home.join("Library").join("Fonts"));
        push(&mut dirs, home.join(".local").join("share").join("fonts"));
        push(&mut dirs, home.join(".fonts"));
    }
    for path in [
        "/System/Library/Fonts",
        "/System/Library/Fonts/Supplemental",
        "/Library/Fonts",
        "/usr/share/fonts",
        "/usr/local/share/fonts",
        "/usr/share/fonts/truetype",
        "/usr/share/fonts/opentype",
        "/usr/share/fonts/truetype/noto",
        "/usr/share/fonts/opentype/noto",
        "/usr/share/fonts/truetype/liberation",
        "/usr/share/fonts/truetype/wqy",
        "/usr/share/fonts/opentype/source-han-sans",
    ] {
        push(&mut dirs, PathBuf::from(path));
    }
    dirs
}

#[cfg(not(target_arch = "wasm32"))]
fn collect_fonts(
    dir: &Path,
    out: &mut Vec<(String, PathBuf)>,
    seen: &mut std::collections::HashSet<PathBuf>,
) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            // 只往下走一层常见发行版目录，避免扫遍整个家目录。
            if dir.ends_with("fonts") || dir.ends_with("Fonts") || dir.ends_with("share") {
                collect_fonts(&path, out, seen);
            }
            continue;
        }
        let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
            continue;
        };
        if !matches!(
            ext.to_ascii_lowercase().as_str(),
            "ttf" | "otf" | "ttc" | "otc"
        ) {
            continue;
        }
        if !seen.insert(path.clone()) {
            continue;
        }
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if !stem.is_empty() {
            out.push((stem, path));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn ink_count(px: f32, name: &str, text: &str) -> usize {
        let face = Face::open_cell(px, false, false, name).expect("soft face should open");
        let mut canvas = vec![255u8; 240 * 80 * 3];
        face.draw_cells(text, 8.0, 12.0, [0, 0, 0], |x, y, rgb, cov| {
            if x < 0 || y < 0 || x >= 240 || y >= 80 || cov == 0 {
                return;
            }
            let i = (y as usize * 240 + x as usize) * 3;
            let a = u16::from(cov);
            for c in 0..3 {
                let dst = u16::from(canvas[i + c]);
                let src = u16::from(rgb[c]);
                canvas[i + c] = (((src * a) + dst * (255 - a) + 127) / 255) as u8;
            }
        });
        canvas
            .chunks(3)
            .filter(|p| p.iter().any(|c| *c < 200))
            .count()
    }

    #[test]
    fn soft_face_opens_and_paints_cjk_and_latin() {
        for name in ["微软雅黑", "等线", "Calibri", "DoesNotExistFont"] {
            let zh = ink_count(16.0, name, "销售经营看板");
            let en = ink_count(16.0, name, "Hello 2026");
            assert!(zh > 40, "{name} chinese ink={zh}");
            assert!(en > 20, "{name} latin ink={en}");
            let face = Face::open_cell(16.0, true, true, name).unwrap();
            assert!(face.width("销售A") > 0.0);
            assert!(face.line_height() > 0.0);
            assert!(face.cell_pad() >= 1.0);
            assert_eq!(
                face.family_name(),
                if name.starts_with('+') {
                    "江城黑体 400W"
                } else {
                    name
                }
            );
        }
        assert!(Face::open_cell(0.0, false, false, "Calibri").is_none());
    }

    fn decode_png(bytes: &[u8]) -> (u32, u32, Vec<u8>) {
        let decoder = png::Decoder::new(Cursor::new(bytes));
        let mut reader = decoder.read_info().unwrap();
        let mut buf = vec![0; reader.output_buffer_size()];
        let frame = reader.next_frame(&mut buf).unwrap();
        (
            frame.width,
            frame.height,
            buf[..frame.buffer_size()].to_vec(),
        )
    }

    fn dark_pixels(rgb: &[u8]) -> usize {
        rgb.chunks(3).filter(|p| p.iter().all(|c| *c < 80)).count()
    }

    fn lock_soft() -> Option<crate::xlsx::chart_gdi::SoftFontGuard> {
        #[cfg(windows)]
        {
            Some(crate::xlsx::chart_gdi::force_soft_fonts())
        }
        #[cfg(not(windows))]
        {
            None
        }
    }

    #[test]
    fn soft_fonts_export_minisheet_with_visible_text() {
        let _guard = lock_soft();
        let dir = std::env::temp_dir().join("ooxp-sheet-font-soft");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("mini-zh.xlsx");
        let req = serde_json::json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "workbook": {
                "sheets": [{
                    "name": "Sheet1",
                    "cells": [
                        { "ref": "A1", "value": "销售经营看板", "font": { "name": "微软雅黑", "sz": 16, "bold": true } },
                        { "ref": "A2", "value": "Hello 2026", "font": { "name": "Calibri", "sz": 12 } }
                    ]
                }]
            }
        });
        crate::run(&req.to_string()).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        let media = crate::xlsx::export_sheet_image(&bytes, "sheet:Sheet1").unwrap();
        let (w, h, rgb) = decode_png(&media.bytes);
        assert!(
            (1..=4096).contains(&w) && (1..=4096).contains(&h),
            "{w}x{h}"
        );
        let ink = dark_pixels(&rgb);
        assert!(ink > 40, "expected visible text ink, got {ink} in {w}x{h}");
    }

    #[test]
    fn soft_fonts_export_official_sales_sheets() {
        let _guard = lock_soft();
        let dir = std::env::temp_dir().join("ooxp-sheet-font-soft");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("sales.xlsx");
        let mut req: serde_json::Value =
            serde_json::from_str(include_str!("../../../../demo/web/samples/xlsx.json")).unwrap();
        req["path"] = serde_json::Value::String(path.to_string_lossy().into_owned());
        crate::run(&req.to_string()).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        for id in ["sheet:总览", "sheet:区域明细", "sheet:说明"] {
            let media = crate::xlsx::export_sheet_image(&bytes, id)
                .unwrap_or_else(|err| panic!("{id} failed: {err}"));
            let (w, h, rgb) = decode_png(&media.bytes);
            assert!(
                (1..=4096).contains(&w) && (1..=4096).contains(&h),
                "{id} size {w}x{h}"
            );
            assert!(!rgb.is_empty());
            let ink = dark_pixels(&rgb);
            assert!(
                ink > 20,
                "{id} should show text, dark pixels={ink} size={w}x{h}"
            );
            if id == "sheet:总览" {
                let chart = crate::xlsx::export_chart_image(&bytes, "sheet:总览/chart:index:1")
                    .unwrap_or_else(|err| panic!("总览 chart export failed: {err}"));
                let (cw, ch, crgb) = decode_png(&chart.bytes);
                assert!(cw > 0 && ch > 0);
                let chart_colors = signature_colors(&crgb);
                let sheet_colors = signature_colors(&rgb);
                let overlap = chart_colors.iter().filter(|c| sheet_colors.contains(*c)).count();
                assert!(
                    overlap >= 3,
                    "总览 sheet PNG should contain the embedded chart colors, overlap={overlap} chart={cw}x{ch} sheet={w}x{h}"
                );
                let colorful = colorful_pixels(&rgb);
                assert!(
                    colorful > 80,
                    "总览 sheet should include chart fills, colorful={colorful}"
                );
            }
        }
    }

    fn signature_colors(rgb: &[u8]) -> std::collections::HashSet<[u8; 3]> {
        let mut out = std::collections::HashSet::new();
        for p in rgb.chunks(3) {
            let chroma = (p[0] as i16 - p[1] as i16)
                .abs()
                .max((p[1] as i16 - p[2] as i16).abs())
                .max((p[0] as i16 - p[2] as i16).abs());
            if chroma > 30 && p.iter().any(|c| *c < 200) {
                // quantize a little so close antialiased shades still match
                out.insert([p[0] & 0xF0, p[1] & 0xF0, p[2] & 0xF0]);
            }
        }
        out
    }

    fn colorful_pixels(rgb: &[u8]) -> usize {
        rgb.chunks(3)
            .filter(|p| {
                let chroma = (p[0] as i16 - p[1] as i16)
                    .abs()
                    .max((p[1] as i16 - p[2] as i16).abs())
                    .max((p[0] as i16 - p[2] as i16).abs());
                chroma > 30 && p.iter().any(|c| *c < 200)
            })
            .count()
    }

}
