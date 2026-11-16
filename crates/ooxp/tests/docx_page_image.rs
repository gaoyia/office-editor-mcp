//! exportPageImage：没有字的页画成白 PNG。单节单段一行的不透明 Calibri 正文可画。

use std::io::Cursor;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::json;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-docx-page-image");
    std::fs::create_dir_all(&dir).unwrap();
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    dir.join(format!("{tag}-{n}.docx"))
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

fn paragraphs(n: usize) -> Vec<serde_json::Value> {
    (0..n).map(|_| json!({ "text": "" })).collect()
}

fn png_path(src: &std::path::Path, id: &str) -> PathBuf {
    let stem = src.file_stem().unwrap().to_string_lossy();
    src.with_file_name(format!("{stem}-{}.png", id.replace(':', "-")))
}

fn export(src: &std::path::Path, id: &str) -> Result<Vec<u8>, ooxp::Error> {
    let out = png_path(src, id);
    if out.exists() {
        std::fs::remove_file(&out).unwrap();
    }
    let result = run(&json!({
        "schemaVersion": 1,
        "action": "exportPageImage",
        "path": src.to_string_lossy(),
        "id": id,
        "output": out.to_string_lossy(),
    })
    .to_string());
    let bytes = std::fs::read(&out).ok();
    match result {
        Ok(RunOutput::Written { content_type, .. }) => {
            assert_eq!(content_type.as_deref(), Some("image/png"));
            Ok(bytes.expect("png"))
        }
        Ok(other) => panic!("应写出文件，得到 {other:?}"),
        Err(err) => Err(err),
    }
}

#[test]
fn blank_page_is_white_a4_at_96_dpi() {
    let src = tmp("blank");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": src.to_string_lossy(),
        "document": { "paragraphs": [{ "text": "" }] }
    }));
    let png = export(&src, "page:1").unwrap();
    let (w, h, rgb) = decode_png(&png);
    assert_eq!((w, h), (794, 1123));
    assert!(rgb.iter().all(|b| *b == 255));
    let missing = export(&src, "page:2").unwrap_err();
    assert_eq!(missing.code(), ErrorCode::IdNotFound);
}

#[test]
fn fifty_two_empty_paragraphs_make_two_white_pages() {
    let src = tmp("pages");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": src.to_string_lossy(),
        "document": { "paragraphs": paragraphs(52) }
    }));
    let first = export(&src, "page:1").unwrap();
    let second = export(&src, "page:2").unwrap();
    let (w1, h1, rgb1) = decode_png(&first);
    let (w2, h2, rgb2) = decode_png(&second);
    assert_eq!((w1, h1), (794, 1123));
    assert_eq!((w2, h2), (w1, h1));
    assert!(rgb1.iter().all(|b| *b == 255));
    assert!(rgb2.iter().all(|b| *b == 255));
    let missing = export(&src, "page:3").unwrap_err();
    assert_eq!(missing.code(), ErrorCode::IdNotFound);

    let one = tmp("one-page");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": one.to_string_lossy(),
        "document": { "paragraphs": paragraphs(51) }
    }));
    let err = export(&one, "page:2").unwrap_err();
    assert_eq!(err.code(), ErrorCode::IdNotFound);
}

fn ink_box(rgb: &[u8], w: u32, h: u32) -> (u32, u32, u32, u32) {
    let mut min_x = w;
    let mut min_y = h;
    let mut max_x = 0u32;
    let mut max_y = 0u32;
    let mut any = false;
    for y in 0..h {
        for x in 0..w {
            let i = ((y * w + x) * 3) as usize;
            if rgb[i] != 255 || rgb[i + 1] != 255 || rgb[i + 2] != 255 {
                any = true;
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }
    }
    assert!(any, "expected ink");
    (min_x, min_y, max_x - min_x + 1, max_y - min_y + 1)
}

fn first_para_id(src: &std::path::Path) -> String {
    let RunOutput::Inspect(tree) = run_ok(json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": src.to_string_lossy(),
        "scope": { "from": 1, "maxParas": 4 }
    })) else {
        panic!("inspect");
    };
    tree["paragraphs"][0]["id"].as_str().unwrap().to_string()
}

fn apply_ops(src: &std::path::Path, ops: serde_json::Value) -> PathBuf {
    let out = src.with_file_name(format!(
        "{}-applied.docx",
        src.file_stem().unwrap().to_string_lossy()
    ));
    if out.exists() {
        std::fs::remove_file(&out).unwrap();
    }
    run_ok(json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": out.to_string_lossy(),
        "ops": ops
    }));
    out
}

fn create_text(tag: &str, text: &str) -> PathBuf {
    let src = tmp(tag);
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": src.to_string_lossy(),
        "document": { "paragraphs": [{ "text": text }] }
    }));
    src
}

#[cfg(windows)]
#[test]
fn default_letter_a_matches_measured_ink_box() {
    let src = create_text("letter-a", "A");
    let png = export(&src, "page:1").unwrap();
    let (w, h, rgb) = decode_png(&png);
    assert_eq!((w, h), (794, 1123));
    // C25 第 0 段实测：Word 16.0 PDF→96dpi 金样里，Calibri 11 磅「A」墨迹在 (96, 101)，9×9。
    assert_eq!(ink_box(&rgb, w, h), (96, 101, 9, 9));
}

#[cfg(windows)]
#[test]
fn text_and_position_samples_shift_with_size_and_origin() {
    let base = create_text("hello", "Hello");
    let (bw, bh, brgb) = decode_png(&export(&base, "page:1").unwrap());
    assert_eq!((bw, bh), (794, 1123));
    let (bx, _by, bw_ink, bh_ink) = ink_box(&brgb, bw, bh);
    assert!(
        (96..100).contains(&bx),
        "Hello ink should start near the left margin, got x={bx}"
    );
    assert!(bw_ink > 9 && bh_ink > 0);

    let letter = create_text("sz24-src", "A");
    let pid = first_para_id(&letter);
    let sized = apply_ops(
        &letter,
        json!([{ "op": "setRunFormat", "id": format!("{pid}/run:1"), "sz": 48 }]),
    );
    let (_, _, srgb) = decode_png(&export(&sized, "page:1").unwrap());
    let (sx, _sy, sw, sh) = ink_box(&srgb, 794, 1123);
    assert_eq!(sx, 96);
    assert!(sh > 9 && sw > 9);

    let indented = create_text("indent-src", "A");
    let pid = first_para_id(&indented);
    let indented = apply_ops(
        &indented,
        json!([{ "op": "setParaFormat", "id": pid, "left": 720 }]),
    );
    let (_, _, irgb) = decode_png(&export(&indented, "page:1").unwrap());
    let (ix, iy, iw, ih) = ink_box(&irgb, 794, 1123);
    assert_eq!((ix, iy, iw, ih), (144, 101, 9, 9));

    let spaced = create_text("before-src", "A");
    let pid = first_para_id(&spaced);
    let spaced = apply_ops(
        &spaced,
        json!([{ "op": "setParaFormat", "id": pid, "before": 240 }]),
    );
    let (_, _, prgb) = decode_png(&export(&spaced, "page:1").unwrap());
    let (px, py, pw, ph) = ink_box(&prgb, 794, 1123);
    assert_eq!((px, py, pw, ph), (96, 117, 9, 9));

    let margin = create_text("margin-src", "A");
    let margined = apply_ops(
        &margin,
        json!([{
            "op": "setPageMargin",
            "id": "section:1",
            "top": 2160,
            "left": 2160,
            "right": 1440,
            "bottom": 1440
        }]),
    );
    let (_, _, mrgb) = decode_png(&export(&margined, "page:1").unwrap());
    let (mx, my, mw, mh) = ink_box(&mrgb, 794, 1123);
    assert_eq!((mx, my, mw, mh), (144, 149, 9, 9));
}

#[cfg(windows)]
#[test]
fn wrapping_or_second_paragraph_is_not_drawn() {
    let wrap = create_text(
        "wrap",
        "This line is intentionally long enough that it should not fit on one A4 content width at Calibri eleven point without wrapping in Word.",
    );
    let err = export(&wrap, "page:1").unwrap_err();
    assert_eq!(err.code(), ErrorCode::NotImplemented);

    let src = tmp("two-para");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": src.to_string_lossy(),
        "document": { "paragraphs": [{ "text": "A" }, { "text": "B" }] }
    }));
    let err = export(&src, "page:1").unwrap_err();
    assert_eq!(err.code(), ErrorCode::NotImplemented);
}

#[cfg(not(windows))]
#[test]
fn text_is_not_drawn_off_windows() {
    let src = create_text("letter", "A");
    let err = export(&src, "page:1").unwrap_err();
    assert_eq!(err.code(), ErrorCode::NotImplemented);
    assert!(!png_path(&src, "page:1").exists());
}

#[test]
fn other_formats_are_rejected() {
    let src = tmp("book").with_extension("xlsx");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": src.to_string_lossy(),
        "workbook": { "sheets": [{ "name": "Sheet1" }] }
    }));
    let err = run(&json!({
        "schemaVersion": 1,
        "action": "exportPageImage",
        "path": src.to_string_lossy(),
        "id": "page:1",
        "output": src.with_extension("png").to_string_lossy(),
    })
    .to_string())
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}
