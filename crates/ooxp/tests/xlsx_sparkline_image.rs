//! exportSparklineImage：一组里的一条折线画成 PNG。柱形和标记不画。

use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::json;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-xlsx-sparkline-image");
    std::fs::create_dir_all(&dir).unwrap();
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    dir.join(format!("{tag}-{n}.xlsx"))
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

fn pixel(rgb: &[u8], width: u32, x: u32, y: u32) -> [u8; 3] {
    let i = ((y * width + x) * 3) as usize;
    [rgb[i], rgb[i + 1], rgb[i + 2]]
}

fn book(tag: &str) -> PathBuf {
    let path = tmp(tag);
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "workbook": {
            "sheets": [{
                "name": "Sheet1",
                "cells": [
                    { "ref": "B1", "value": 1 },
                    { "ref": "C1", "value": 4 },
                    { "ref": "D1", "value": 2 }
                ]
            }]
        }
    }));
    path
}

fn insert_line(src: &Path) -> PathBuf {
    let out = tmp("line");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": out.to_string_lossy(),
        "ops": [{
            "op": "insertSparkline",
            "id": "sheet:Sheet1",
            "ref": "A1",
            "formula": "B1:D1"
        }]
    }));
    out
}

fn export(src: &Path, id: &str) -> Result<Vec<u8>, ooxp::Error> {
    let png = src.with_extension("png");
    if png.exists() {
        std::fs::remove_file(&png).unwrap();
    }
    let written = run(&json!({
        "schemaVersion": 1,
        "action": "exportSparklineImage",
        "path": src.to_string_lossy(),
        "id": id,
        "output": png.to_string_lossy(),
    })
    .to_string())?;
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

#[test]
fn line_sparkline_is_a_png_of_the_cell() {
    let src = insert_line(&book("cell"));
    let before = std::fs::read(&src).unwrap();
    let bytes = export(&src, "sheet:Sheet1/spark:1").unwrap();
    assert_eq!(std::fs::read(&src).unwrap(), before);
    assert_eq!(&bytes[..8], &[137, 80, 78, 71, 13, 10, 26, 10]);
    let (w, h, rgb) = decode_png(&bytes);
    assert_eq!((w, h), (64, 20));
    assert_eq!(pixel(&rgb, w, 0, 0), [255, 255, 255]);
    let peak = pixel(&rgb, w, 32, 5);
    let start = pixel(&rgb, w, 12, 16);
    assert!(peak[2] > peak[0], "峰顶像素 {peak:?}");
    assert!(start[2] > 140, "起点像素 {start:?}");
}

#[test]
fn column_markers_and_missing_ids_are_rejected() {
    let src = insert_line(&book("reject"));
    let column = tmp("column");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": column.to_string_lossy(),
        "ops": [{
            "op": "setSparkline",
            "id": "sheet:Sheet1/spark:1",
            "type": "column"
        }]
    }));
    let err = export(&column, "sheet:Sheet1/spark:1").unwrap_err();
    assert_eq!(err.code(), ErrorCode::NotImplemented);

    let marked = tmp("marked");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": marked.to_string_lossy(),
        "ops": [{
            "op": "setSparkline",
            "id": "sheet:Sheet1/spark:1",
            "markers": true
        }]
    }));
    let err = export(&marked, "sheet:Sheet1/spark:1").unwrap_err();
    assert_eq!(err.code(), ErrorCode::NotImplemented);

    let missing = export(&src, "sheet:Sheet1/spark:2").unwrap_err();
    assert_eq!(missing.code(), ErrorCode::IdNotFound);
    let bad = export(&src, "sheet:Sheet1/chart:1").unwrap_err();
    assert_eq!(bad.code(), ErrorCode::InvalidId);
}

#[test]
fn other_formats_are_rejected() {
    let src = tmp("page").with_extension("docx");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": src.to_string_lossy(),
        "document": { "paragraphs": [{ "text": "" }] }
    }));
    let err = run(&json!({
        "schemaVersion": 1,
        "action": "exportSparklineImage",
        "path": src.to_string_lossy(),
        "id": "sheet:Sheet1/spark:1",
        "output": src.with_extension("png").to_string_lossy(),
    })
    .to_string())
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}
