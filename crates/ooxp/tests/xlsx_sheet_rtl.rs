//! 从右到左且没有文字时，整张图是从左到右那张的水平翻转。有文字仍不画。

use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode};
use serde_json::json;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-xlsx-sheet-rtl");
    std::fs::create_dir_all(&dir).unwrap();
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    dir.join(format!("{tag}-{n}.xlsx"))
}

fn run_ok(value: serde_json::Value) {
    run(&value.to_string()).unwrap();
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

fn book() -> PathBuf {
    let path = tmp("book");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "workbook": {
            "sheets": [{
                "name": "Sheet1",
                "cells": [
                    { "ref": "A1", "value": "" },
                    { "ref": "B1", "value": "" },
                    { "ref": "A2", "value": "" }
                ]
            }]
        }
    }));
    let out = tmp("filled");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": path.to_string_lossy(),
        "output": out.to_string_lossy(),
        "ops": [
            {
                "op": "setCellStyle",
                "id": "cell:Sheet1!A1",
                "fill": { "pattern": "solid", "fg": { "rgb": "FFEE4455" } }
            },
            {
                "op": "setCellStyle",
                "id": "cell:Sheet1!B1",
                "fill": { "pattern": "solid", "fg": { "rgb": "FF00AA66" } }
            },
            {
                "op": "setCellStyle",
                "id": "cell:Sheet1!A2",
                "fill": { "pattern": "solid", "fg": { "rgb": "FF112233" } }
            }
        ]
    }));
    out
}

fn export(src: &Path) -> Result<Vec<u8>, ooxp::Error> {
    let png = src.with_extension("png");
    run(&json!({
        "schemaVersion": 1,
        "action": "exportSheetImage",
        "path": src.to_string_lossy(),
        "id": "sheet:Sheet1",
        "output": png.to_string_lossy(),
    })
    .to_string())?;
    Ok(std::fs::read(png).unwrap())
}

fn with_view(src: &Path, view: &str) -> PathBuf {
    let dst = tmp("view");
    let bytes = std::fs::read(src).unwrap();
    let mut zip_in = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut out = Cursor::new(Vec::new());
    {
        let mut zip_out = ZipWriter::new(&mut out);
        let opt = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        for i in 0..zip_in.len() {
            let mut file = zip_in.by_index(i).unwrap();
            let name = file.name().to_string();
            let mut data = Vec::new();
            file.read_to_end(&mut data).unwrap();
            if name == "xl/worksheets/sheet1.xml" {
                let xml = String::from_utf8(data).unwrap();
                data = xml
                    .replace(
                        "<sheetData>",
                        &format!("<sheetViews><sheetView {view} workbookViewId=\"0\"/></sheetViews><sheetData>"),
                    )
                    .into_bytes();
            }
            zip_out.start_file(&name, opt).unwrap();
            zip_out.write_all(&data).unwrap();
        }
        zip_out.finish().unwrap();
    }
    std::fs::write(&dst, out.into_inner()).unwrap();
    dst
}

fn flip(px: &[u8], w: u32, h: u32) -> Vec<u8> {
    let mut out = vec![0; px.len()];
    for y in 0..h {
        for x in 0..w {
            let i = ((y * w + x) * 3) as usize;
            let j = ((y * w + (w - 1 - x)) * 3) as usize;
            out[j..j + 3].copy_from_slice(&px[i..i + 3]);
        }
    }
    out
}

#[test]
fn right_to_left_flips_a_sheet_without_text() {
    let src = book();
    let (w, h, ltr) = decode_png(&export(&src).unwrap());
    let rtl_book = with_view(&src, "rightToLeft=\"1\"");
    let (w2, h2, rtl) = decode_png(&export(&rtl_book).unwrap());
    assert_eq!((w, h), (129, 41));
    assert_eq!((w2, h2), (w, h));
    assert_eq!(rtl, flip(&ltr, w, h));
}

#[test]
fn right_to_left_with_text_is_rejected() {
    let src = book();
    let written = tmp("text");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": written.to_string_lossy(),
        "ops": [{ "op": "setCell", "id": "cell:Sheet1!A1", "value": "AB" }]
    }));
    let rtl_book = with_view(&written, "rightToLeft=\"1\"");
    let err = export(&rtl_book).unwrap_err();
    assert_eq!(err.code(), ErrorCode::NotImplemented);
}
