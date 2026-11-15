//! 隐藏行和隐藏列不占像素。可见格子接上，整边隐藏时只留 1 像素白。

use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::json;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

static SEQ: AtomicU32 = AtomicU32::new(0);

const GRID: [u8; 3] = [224, 224, 224];

fn tmp(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-xlsx-sheet-hidden");
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

fn pixel(rgb: &[u8], width: u32, x: u32, y: u32) -> [u8; 3] {
    let i = ((y * width + x) * 3) as usize;
    [rgb[i], rgb[i + 1], rgb[i + 2]]
}

fn book(tag: &str, cells: serde_json::Value) -> PathBuf {
    let path = tmp(tag);
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "workbook": { "sheets": [{ "name": "Sheet1", "cells": cells }] }
    }));
    path
}

fn apply(src: &Path, ops: serde_json::Value) -> PathBuf {
    let out = tmp("applied");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": out.to_string_lossy(),
        "ops": ops
    }));
    out
}

fn fill(id_ref: &str, rgb: &str) -> serde_json::Value {
    json!({
        "op": "setCellStyle",
        "id": format!("cell:Sheet1!{id_ref}"),
        "fill": { "pattern": "solid", "fg": { "rgb": rgb } }
    })
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

fn rewrite_sheet(src: &Path, edit: impl FnOnce(String) -> String) -> PathBuf {
    let dst = tmp("patched");
    let bytes = std::fs::read(src).unwrap();
    let mut zip_in = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut out = Cursor::new(Vec::new());
    let mut edit = Some(edit);
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
                data = edit.take().unwrap()(xml).into_bytes();
            }
            zip_out.start_file(&name, opt).unwrap();
            zip_out.write_all(&data).unwrap();
        }
        zip_out.finish().unwrap();
    }
    std::fs::write(&dst, out.into_inner()).unwrap();
    dst
}

fn three_rows() -> PathBuf {
    book(
        "rows",
        json!([
            { "ref": "A1", "value": "" },
            { "ref": "A2", "value": "" },
            { "ref": "A3", "value": "" }
        ]),
    )
}

#[test]
fn hidden_middle_row_packs_the_fills() {
    let src = apply(
        &three_rows(),
        json!([
            fill("A1", "FFEE4455"),
            fill("A2", "FF112233"),
            fill("A3", "FF00AA66"),
            { "op": "setRowHidden", "id": "sheet:Sheet1", "rows": "2", "hidden": true }
        ]),
    );
    let (w, h, px) = decode_png(&export(&src).unwrap());
    assert_eq!((w, h), (65, 41));
    assert_eq!(pixel(&px, w, 0, 0), GRID);
    assert_eq!(pixel(&px, w, 1, 1), [0xEE, 0x44, 0x55]);
    assert_eq!(pixel(&px, w, 1, 19), [0xEE, 0x44, 0x55]);
    assert_eq!(pixel(&px, w, 1, 20), [0x00, 0xAA, 0x66]);
    assert_eq!(pixel(&px, w, 1, 40), [0x00, 0xAA, 0x66]);
    assert!(!px.chunks(3).any(|p| p == [0x11, 0x22, 0x33]));
}

#[test]
fn hidden_middle_column_packs_the_fills() {
    let src = book(
        "cols",
        json!([
            { "ref": "A1", "value": "" },
            { "ref": "B1", "value": "" },
            { "ref": "C1", "value": "" }
        ]),
    );
    let src = apply(
        &src,
        json!([
            fill("A1", "FFEE4455"),
            fill("B1", "FF112233"),
            fill("C1", "FF00AA66"),
            { "op": "setColHidden", "id": "sheet:Sheet1", "col": "B", "hidden": true }
        ]),
    );
    let (w, h, px) = decode_png(&export(&src).unwrap());
    assert_eq!((w, h), (129, 21));
    assert_eq!(pixel(&px, w, 1, 1), [0xEE, 0x44, 0x55]);
    assert_eq!(pixel(&px, w, 63, 1), [0xEE, 0x44, 0x55]);
    assert_eq!(pixel(&px, w, 64, 1), [0x00, 0xAA, 0x66]);
    assert_eq!(pixel(&px, w, 128, 1), [0x00, 0xAA, 0x66]);
    assert!(!px.chunks(3).any(|p| p == [0x11, 0x22, 0x33]));
}

#[test]
fn fully_hidden_row_is_a_white_strip() {
    let src = book("one", json!([{ "ref": "A1", "value": "" }]));
    let src = apply(
        &src,
        json!([
            fill("A1", "FFEE4455"),
            { "op": "setRowHidden", "id": "sheet:Sheet1", "rows": "1", "hidden": true }
        ]),
    );
    let (w, h, px) = decode_png(&export(&src).unwrap());
    assert_eq!((w, h), (65, 1));
    assert!(px.chunks(3).all(|p| p == [255, 255, 255]));
}

#[test]
fn hidden_merge_anchor_still_paints_the_visible_rows() {
    let src = apply(
        &three_rows(),
        json!([
            fill("A1", "FFEE4455"),
            { "op": "mergeRange", "id": "sheet:Sheet1", "range": "A1:A3" },
            { "op": "setRowHidden", "id": "sheet:Sheet1", "rows": "1", "hidden": true }
        ]),
    );
    let (w, h, px) = decode_png(&export(&src).unwrap());
    assert_eq!((w, h), (65, 41));
    assert_eq!(pixel(&px, w, 0, 0), GRID);
    assert_eq!(pixel(&px, w, 1, 1), [0xEE, 0x44, 0x55]);
    assert_eq!(pixel(&px, w, 1, 40), [0xEE, 0x44, 0x55]);
}

#[test]
fn hidden_column_font_still_grows_the_row() {
    let src = book(
        "font",
        json!([
            { "ref": "A1", "value": "" },
            { "ref": "B1", "value": "" },
            { "ref": "C1", "value": "" }
        ]),
    );
    let src = apply(
        &src,
        json!([
            {
                "op": "setCellStyle",
                "id": "cell:Sheet1!B1",
                "font": { "name": "Calibri", "sz": 20 }
            },
            { "op": "setColHidden", "id": "sheet:Sheet1", "col": "B", "hidden": true }
        ]),
    );
    let (w, h, _) = decode_png(&export(&src).unwrap());
    assert_eq!((w, h), (129, 37));
}

#[test]
fn grid_off_and_hidden_row_leave_a_white_frame() {
    let src = apply(
        &three_rows(),
        json!([
            fill("A1", "FFEE4455"),
            fill("A3", "FF00AA66"),
            { "op": "setRowHidden", "id": "sheet:Sheet1", "rows": "2", "hidden": true }
        ]),
    );
    let off = rewrite_sheet(&src, |xml| {
        xml.replace(
            "<sheetData>",
            "<sheetViews><sheetView showGridLines=\"0\" workbookViewId=\"0\"/></sheetViews><sheetData>",
        )
    });
    let (w, h, px) = decode_png(&export(&off).unwrap());
    assert_eq!((w, h), (65, 41));
    assert_eq!(pixel(&px, w, 0, 0), [255, 255, 255]);
    assert_eq!(pixel(&px, w, 1, 1), [0xEE, 0x44, 0x55]);
    assert_eq!(pixel(&px, w, 1, 20), [0x00, 0xAA, 0x66]);
}

#[test]
fn conditional_format_on_a_hidden_row_is_rejected() {
    let src = apply(
        &three_rows(),
        json!([{ "op": "setRowHidden", "id": "sheet:Sheet1", "rows": "2", "hidden": true }]),
    );
    let patched = rewrite_sheet(&src, |xml| {
        xml.replace(
            "</worksheet>",
            "<conditionalFormatting sqref=\"A1:A3\"><cfRule type=\"cellIs\" priority=\"1\"/></conditionalFormatting></worksheet>",
        )
    });
    let err = export(&patched).unwrap_err();
    assert_eq!(err.code(), ErrorCode::NotImplemented);
}
