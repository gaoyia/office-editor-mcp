//! exportPageImage：没有字的页画成白 PNG。有字的页不画。

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

#[test]
fn text_is_not_drawn() {
    let src = tmp("letter");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": src.to_string_lossy(),
        "document": { "paragraphs": [{ "text": "A" }] }
    }));
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
