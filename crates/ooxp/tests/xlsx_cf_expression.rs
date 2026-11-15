//! 公式条件格式的纯色，铺在和单元格填充同一块矩形上。

use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, RunOutput};
use serde_json::json;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

static SEQ: AtomicU32 = AtomicU32::new(0);

const RED: [u8; 3] = [0xEE, 0x44, 0x55];

fn tmp(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-xlsx-cf-expression");
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

fn hide_grid(src: &Path) -> PathBuf {
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
                let mut xml = String::from_utf8(data).unwrap();
                if !xml.contains("showGridLines") {
                    xml = xml.replace(
                        "<sheetData>",
                        "<sheetViews><sheetView showGridLines=\"0\" workbookViewId=\"0\"/></sheetViews><sheetData>",
                    );
                }
                data = xml.into_bytes();
            }
            zip_out.start_file(&name, opt).unwrap();
            zip_out.write_all(&data).unwrap();
        }
        zip_out.finish().unwrap();
    }
    let dst = tmp("grid");
    std::fs::write(&dst, out.into_inner()).unwrap();
    dst
}

fn assert_red_span(rgb: &[u8], width: u32, y0: u32, y1: u32) {
    for y in y0..y1 {
        for x in 1..width {
            assert_eq!(pixel(rgb, width, x, y), RED, "({x},{y})");
        }
        assert_eq!(pixel(rgb, width, 0, y), [255, 255, 255], "外框 ({y})");
    }
}

#[test]
fn true_formula_fills_like_a_cell_and_false_stays_blank() {
    let src = tmp("src");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": src.to_string_lossy(),
        "workbook": { "sheets": [{ "name": "Sheet1", "cells": [
            { "ref": "A1", "value": "" },
            { "ref": "A2", "value": "" },
            { "ref": "A3", "value": "" }
        ] }] }
    }));
    let shaped = tmp("cf");
    let fill = json!({ "pattern": "solid", "fg": { "rgb": "FFEE4455" } });
    run_ok(json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": shaped.to_string_lossy(),
        "ops": [
            { "op": "setRowHeight", "id": "sheet:Sheet1", "row": 1, "height": 15 },
            { "op": "setRowHeight", "id": "sheet:Sheet1", "row": 2, "height": 15 },
            { "op": "setRowHeight", "id": "sheet:Sheet1", "row": 3, "height": 15 },
            { "op": "insertConditionalFormat", "id": "sheet:Sheet1", "sqref": "A1:A2",
              "type": "expression", "formulas": ["TRUE"], "dxf": { "fill": fill } },
            { "op": "insertConditionalFormat", "id": "sheet:Sheet1", "sqref": "A3",
              "type": "expression", "formulas": ["FALSE"], "dxf": { "fill": fill } }
        ]
    }));
    let (w, h, rgb) = decode_png(&export(&hide_grid(&shaped)).unwrap());
    assert_eq!((w, h), (65, 61));
    assert_eq!(pixel(&rgb, w, 1, 0), [255, 255, 255]);
    assert_red_span(&rgb, w, 1, 41);
    for y in 41..h {
        for x in 0..w {
            assert_eq!(pixel(&rgb, w, x, y), [255, 255, 255], "({x},{y})");
        }
    }
}

#[test]
fn a_missing_cell_inside_the_rule_still_fills() {
    let src = tmp("src");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": src.to_string_lossy(),
        "workbook": { "sheets": [{ "name": "Sheet1", "cells": [
            { "ref": "A1", "value": "" },
            { "ref": "A3", "value": "" }
        ] }] }
    }));
    let shaped = tmp("gap");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": shaped.to_string_lossy(),
        "ops": [
            { "op": "setRowHeight", "id": "sheet:Sheet1", "row": 1, "height": 15 },
            { "op": "setRowHeight", "id": "sheet:Sheet1", "row": 2, "height": 15 },
            { "op": "setRowHeight", "id": "sheet:Sheet1", "row": 3, "height": 15 },
            { "op": "insertConditionalFormat", "id": "sheet:Sheet1", "sqref": "A1:A3",
              "type": "expression", "formulas": ["TRUE"],
              "dxf": { "fill": { "pattern": "solid", "fg": { "rgb": "FFEE4455" } } } }
        ]
    }));
    let (w, h, rgb) = decode_png(&export(&hide_grid(&shaped)).unwrap());
    assert_eq!((w, h), (65, 61));
    assert_red_span(&rgb, w, 1, h);
}

#[test]
fn an_unknown_formula_fails_the_sheet() {
    let src = tmp("src");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": src.to_string_lossy(),
        "workbook": { "sheets": [{ "name": "Sheet1", "cells": [
            { "ref": "A1", "value": "" }
        ] }] }
    }));
    let shaped = tmp("bad");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": shaped.to_string_lossy(),
        "ops": [{
            "op": "insertConditionalFormat", "id": "sheet:Sheet1", "sqref": "A1",
            "type": "expression", "formulas": ["FOOBAR()"],
            "dxf": { "fill": { "pattern": "solid", "fg": { "rgb": "FFEE4455" } } }
        }]
    }));
    let err = export(&shaped).unwrap_err();
    let text = err.to_string();
    assert!(
        text.contains("工作表") || text.contains("worksheet"),
        "{text}"
    );
}
