//! exportSlideImage / exportPageImage 的可选 dpi：省略或 96 与现有像素一致；
//! 其它整数 DPI 按请求值原生换算（twip/EMU/磅），不是先画 96 再拉伸，也不套用 Excel 的 96 先量化。

use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::json;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp(tag: &str, ext: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-docx-pptx-export-dpi");
    std::fs::create_dir_all(&dir).unwrap();
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    dir.join(format!("{tag}-{n}.{ext}"))
}

fn run_ok(value: serde_json::Value) -> RunOutput {
    run(&value.to_string()).unwrap()
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

fn png_pixel_dims(bytes: &[u8]) -> Option<png::PixelDimensions> {
    let decoder = png::Decoder::new(Cursor::new(bytes));
    let reader = decoder.read_info().unwrap();
    reader.info().pixel_dims
}

fn assert_no_phys(bytes: &[u8]) {
    assert!(png_pixel_dims(bytes).is_none(), "默认 96 DPI 不得写入 pHYs");
}

fn assert_phys_dpi(bytes: &[u8], dpi: u32) {
    let ppm = (f64::from(dpi) / 0.0254).round() as u32;
    let dims = png_pixel_dims(bytes).expect("非默认 DPI 应写入 pHYs");
    assert_eq!(dims.xppu, ppm);
    assert_eq!(dims.yppu, ppm);
    assert_eq!(dims.unit, png::Unit::Meter);
}

fn ink_bbox(rgb: &[u8], w: u32, h: u32, pred: impl Fn([u8; 3]) -> bool) -> (i32, i32, i32, i32) {
    let mut min_x = w as i32;
    let mut min_y = h as i32;
    let mut max_x = -1i32;
    let mut max_y = -1i32;
    for y in 0..h as i32 {
        for x in 0..w as i32 {
            let i = ((y as u32 * w + x as u32) * 3) as usize;
            if pred([rgb[i], rgb[i + 1], rgb[i + 2]]) {
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }
    }
    assert!(max_x >= min_x && max_y >= min_y, "应画出内容");
    (min_x, min_y, max_x + 1, max_y + 1)
}

fn export_bytes(action: &str, src: &Path, id: &str, dpi: Option<i64>) -> Result<Vec<u8>, ooxp::Error> {
    let png = tmp(action, "png");
    if png.exists() {
        std::fs::remove_file(&png).unwrap();
    }
    let mut req = json!({
        "schemaVersion": 1,
        "action": action,
        "path": src.to_string_lossy(),
        "id": id,
        "output": png.to_string_lossy(),
    });
    if let Some(dpi) = dpi {
        req["dpi"] = json!(dpi);
    }
    let written = run(&req.to_string())?;
    let RunOutput::Written {
        path, content_type, ..
    } = written
    else {
        panic!("应写出文件");
    };
    assert_eq!(content_type.as_deref(), Some("image/png"));
    assert_eq!(path, png);
    Ok(std::fs::read(png).unwrap())
}

fn blank_docx(tag: &str) -> PathBuf {
    let path = tmp(tag, "docx");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "document": { "paragraphs": [{ "text": "" }] }
    }));
    path
}

fn solid_rect_pptx(tag: &str, cx: i64, cy: i64) -> PathBuf {
    let src = tmp(tag, "pptx");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": src.to_string_lossy(),
        "presentation": { "slides": [{ "shapes": [{ "text": "标题" }] }] }
    }));
    let tree = {
        let RunOutput::Inspect(tree) = run_ok(json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": src.to_string_lossy(),
            "scope": { "slide": 1 }
        })) else {
            panic!("inspect");
        };
        tree
    };
    let shape_id = tree["slides"][0]["shapes"][0]["id"].as_str().unwrap();
    let deck = tmp(&format!("{tag}-deck"), "pptx");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": deck.to_string_lossy(),
        "ops": [
            { "op": "setPageBackground", "id": "slide:1", "color": "FFFFFF" },
            { "op": "deleteShape", "id": shape_id },
            {
                "op": "insertShape",
                "id": "slide:1",
                "prst": "rect",
                "text": "",
                "x": 0,
                "y": 0,
                "cx": cx,
                "cy": cy,
                "fill": "EE4455",
                "line": { "color": null }
            }
        ]
    }));
    deck
}

#[test]
fn page_omitted_and_explicit_96_match_historical_a4() {
    let src = blank_docx("page-default");
    let before = std::fs::read(&src).unwrap();
    let omitted = export_bytes("exportPageImage", &src, "page:1", None).unwrap();
    let explicit = export_bytes("exportPageImage", &src, "page:1", Some(96)).unwrap();
    assert_eq!(std::fs::read(&src).unwrap(), before, "导出不得改源包");
    assert_eq!(omitted, explicit);
    let (w, h, rgb) = decode_png(&omitted);
    assert_eq!((w, h), (794, 1123));
    assert!(rgb.iter().all(|b| *b == 255));
    assert_no_phys(&omitted);
    assert_no_phys(&explicit);
}

#[test]
fn page_106_is_native_twip_not_96_scaled() {
    let src = blank_docx("page-106");
    let before = std::fs::read(&src).unwrap();
    let at96 = export_bytes("exportPageImage", &src, "page:1", Some(96)).unwrap();
    let at106 = export_bytes("exportPageImage", &src, "page:1", Some(106)).unwrap();
    let at192 = export_bytes("exportPageImage", &src, "page:1", Some(192)).unwrap();
    assert_eq!(std::fs::read(&src).unwrap(), before);
    let (w96, h96, _) = decode_png(&at96);
    let (w106, h106, _) = decode_png(&at106);
    let (w192, h192, _) = decode_png(&at192);
    assert_eq!((w96, h96), (794, 1123));
    // A4 11906×16838 twip：原生 106 是 876×1239，不是先得到 794/1123 再 scale_px（877×1240）。
    assert_eq!((w106, h106), (876, 1239));
    assert_ne!(w106, (794.0f64 * 106.0 / 96.0).round() as u32);
    assert_eq!((w192, h192), (1587, 2245));
    assert_no_phys(&at96);
    assert_phys_dpi(&at106, 106);
    assert_phys_dpi(&at192, 192);
}

#[test]
fn slide_omitted_and_explicit_96_match_historical_16x9() {
    let src = solid_rect_pptx("slide-default", 90488, 90488);
    let before = std::fs::read(&src).unwrap();
    let omitted = export_bytes("exportSlideImage", &src, "slide:1", None).unwrap();
    let explicit = export_bytes("exportSlideImage", &src, "slide:1", Some(96)).unwrap();
    assert_eq!(std::fs::read(&src).unwrap(), before, "导出不得改源包");
    assert_eq!(omitted, explicit);
    let (w, h, rgb) = decode_png(&omitted);
    assert_eq!((w, h), (1280, 720));
    let (x0, y0, x1, y1) = ink_bbox(&rgb, w, h, |c| c == [0xEE, 0x44, 0x55]);
    assert_eq!((x0, y0, x1, y1), (0, 0, 10, 10));
    assert_no_phys(&omitted);
    assert_no_phys(&explicit);
}

#[test]
fn slide_106_uses_native_emu_not_96_scaled_px() {
    // 90488 EMU 在 96 dpi 是 10 px；原生 106 仍是 10，按 10 再放大则是 11。
    let src = solid_rect_pptx("slide-106", 90488, 90488);
    let before = std::fs::read(&src).unwrap();
    let at96 = export_bytes("exportSlideImage", &src, "slide:1", Some(96)).unwrap();
    let at106 = export_bytes("exportSlideImage", &src, "slide:1", Some(106)).unwrap();
    let at192 = export_bytes("exportSlideImage", &src, "slide:1", Some(192)).unwrap();
    assert_eq!(std::fs::read(&src).unwrap(), before);
    let (w96, h96, rgb96) = decode_png(&at96);
    let (w106, h106, rgb106) = decode_png(&at106);
    let (w192, h192, rgb192) = decode_png(&at192);
    assert_eq!((w96, h96), (1280, 720));
    assert_eq!((w106, h106), (1413, 795));
    assert_eq!((w192, h192), (2560, 1440));
    let (_, _, x1_96, y1_96) = ink_bbox(&rgb96, w96, h96, |c| c == [0xEE, 0x44, 0x55]);
    let (_, _, x1_106, y1_106) = ink_bbox(&rgb106, w106, h106, |c| c == [0xEE, 0x44, 0x55]);
    let (_, _, x1_192, y1_192) = ink_bbox(&rgb192, w192, h192, |c| c == [0xEE, 0x44, 0x55]);
    assert_eq!((x1_96, y1_96), (10, 10));
    assert_eq!((x1_106, y1_106), (10, 10), "不得把 96 dpi 的 10 px 放大成 11");
    assert_eq!((x1_192, y1_192), (19, 19));
    assert_no_phys(&at96);
    assert_phys_dpi(&at106, 106);
    assert_phys_dpi(&at192, 192);
}

#[test]
fn illegal_dpi_is_rejected_and_source_stays() {
    let docx = blank_docx("bad-page");
    let pptx = solid_rect_pptx("bad-slide", 90488, 90488);
    let before_docx = std::fs::read(&docx).unwrap();
    let before_pptx = std::fs::read(&pptx).unwrap();
    for (action, src, id) in [
        ("exportPageImage", docx.as_path(), "page:1"),
        ("exportSlideImage", pptx.as_path(), "slide:1"),
    ] {
        for (dpi, code) in [
            (serde_json::json!(0), ErrorCode::TruncatedLimit),
            (serde_json::json!(-1), ErrorCode::TruncatedLimit),
            (serde_json::json!(23), ErrorCode::TruncatedLimit),
            (serde_json::json!(601), ErrorCode::TruncatedLimit),
            (serde_json::json!(24.5), ErrorCode::InvalidField),
            (serde_json::json!("96"), ErrorCode::InvalidField),
            (serde_json::json!(true), ErrorCode::InvalidField),
            (serde_json::json!(null), ErrorCode::InvalidField),
            (serde_json::json!([]), ErrorCode::InvalidField),
            (serde_json::json!({}), ErrorCode::InvalidField),
        ] {
            let png = tmp("bad", "png");
            let err = run(&json!({
                "schemaVersion": 1,
                "action": action,
                "path": src.to_string_lossy(),
                "id": id,
                "output": png.to_string_lossy(),
                "dpi": dpi,
            })
            .to_string())
            .unwrap_err();
            assert_eq!(err.code(), code, "{action} dpi={dpi}");
        }
    }
    assert_eq!(std::fs::read(&docx).unwrap(), before_docx);
    assert_eq!(std::fs::read(&pptx).unwrap(), before_pptx);
    let lo_page = export_bytes("exportPageImage", &docx, "page:1", Some(24)).unwrap();
    let (w, h, _) = decode_png(&lo_page);
    assert_eq!((w, h), (198, 281));
}

#[test]
fn wide_16x9_slide_at_600_dpi_is_not_implemented() {
    let src = solid_rect_pptx("slide-600", 90488, 90488);
    let before = std::fs::read(&src).unwrap();
    let err = export_bytes("exportSlideImage", &src, "slide:1", Some(600)).unwrap_err();
    assert_eq!(err.code(), ErrorCode::NotImplemented);
    assert_eq!(std::fs::read(&src).unwrap(), before);
}

#[cfg(windows)]
fn text_docx(tag: &str) -> PathBuf {
    let path = tmp(tag, "docx");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "document": { "paragraphs": [{ "text": "Hg" }] }
    }));
    path
}

/// 正文必须按请求 DPI 重算字号，不能先画 96 再拉伸。
#[cfg(windows)]
#[test]
fn page_text_at_192_dpi_is_native_not_96_glyphs() {
    let src = text_docx("page-text");
    let before = std::fs::read(&src).unwrap();
    let at96 = export_bytes("exportPageImage", &src, "page:1", Some(96)).unwrap();
    let omitted = export_bytes("exportPageImage", &src, "page:1", None).unwrap();
    let at192 = export_bytes("exportPageImage", &src, "page:1", Some(192)).unwrap();
    assert_eq!(std::fs::read(&src).unwrap(), before, "导出不得改源包");
    assert_eq!(at96, omitted);
    assert_no_phys(&at96);
    assert_phys_dpi(&at192, 192);
    let (w96, h96, rgb96) = decode_png(&at96);
    let (w192, h192, rgb192) = decode_png(&at192);
    assert_eq!((w96, h96), (794, 1123));
    assert_eq!((w192, h192), (1587, 2245));
    let ink = |c: [u8; 3]| c[0] < 180 && c[1] < 180 && c[2] < 180;
    let (_, y0_96, _, y1_96) = ink_bbox(&rgb96, w96, h96, ink);
    let (_, y0_192, _, y1_192) = ink_bbox(&rgb192, w192, h192, ink);
    let ink96 = y1_96 - y0_96;
    let ink192 = y1_192 - y0_192;
    assert!(
        ink192 > ink96,
        "192 dpi 字形应高于 96：96={ink96}px 192={ink192}px"
    );
}
