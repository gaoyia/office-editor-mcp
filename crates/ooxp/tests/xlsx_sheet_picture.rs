//! 工作表上不缩放的不透明 PNG，按原图像素贴上。

use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::json;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-xlsx-sheet-picture");
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

fn export(src: &Path) -> Result<Vec<u8>, ooxp::Error> {
    let png = src.with_extension("png");
    if png.exists() {
        std::fs::remove_file(&png).unwrap();
    }
    let written = run(&json!({
        "schemaVersion": 1,
        "action": "exportSheetImage",
        "path": src.to_string_lossy(),
        "id": "sheet:Sheet1",
        "output": png.to_string_lossy(),
    })
    .to_string())?;
    let RunOutput::Written { path, .. } = written else {
        panic!("应写出文件");
    };
    Ok(std::fs::read(path).unwrap())
}

fn checker(path: &Path) {
    let mut data = Vec::new();
    for y in 0..4 {
        for x in 0..4 {
            let px: [u8; 3] = if (x + y) % 2 == 0 {
                [0, 0, 0]
            } else {
                [255, 255, 255]
            };
            data.extend_from_slice(&px);
        }
    }
    let file = std::fs::File::create(path).unwrap();
    let mut encoder = png::Encoder::new(file, 4, 4);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(&data)
        .unwrap();
}

fn book_with_picture(cx: i64) -> PathBuf {
    let src = tmp("src");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": src.to_string_lossy(),
        "workbook": { "sheets": [{ "name": "Sheet1", "cells": [{ "ref": "A1", "value": "" }] }] }
    }));
    let png = tmp("checker").with_extension("png");
    checker(&png);
    let shaped = tmp("shaped");
    let px = 9525;
    run_ok(json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": shaped.to_string_lossy(),
        "ops": [
            { "op": "setRowHeight", "id": "sheet:Sheet1", "row": 1, "height": 15 },
            { "op": "insertPicture", "id": "sheet:Sheet1", "file": png.to_string_lossy(),
              "from": "A1", "fromOffX": px, "fromOffY": px, "cx": cx, "cy": 4 * px }
        ]
    }));
    shaped
}

#[test]
fn an_unscaled_png_is_copied_pixel_for_pixel() {
    let book = book_with_picture(4 * 9525);
    let (w, h, rgb) = decode_png(&export(&book).unwrap());
    assert_eq!((w, h), (65, 21));
    assert_eq!(pixel(&rgb, w, 1, 1), [0, 0, 0]);
    assert_eq!(pixel(&rgb, w, 2, 1), [255, 255, 255]);
    assert_eq!(pixel(&rgb, w, 1, 2), [255, 255, 255]);
    assert_eq!(pixel(&rgb, w, 4, 4), [0, 0, 0]);
    assert_eq!(pixel(&rgb, w, 5, 1), [255, 255, 255]);
}

#[test]
fn a_scaled_png_fails_the_sheet() {
    let book = book_with_picture(8 * 9525);
    assert_eq!(export(&book).unwrap_err().code(), ErrorCode::NotImplemented);
}
