//! 工作表上没有描边的纯色矩形，按 Excel 16.0 量过的像素来画。

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
    let dir = std::env::temp_dir().join("ooxp-xlsx-sheet-shape");
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

fn export(src: &Path) -> Vec<u8> {
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
    .to_string())
    .unwrap();
    let RunOutput::Written { path, .. } = written else {
        panic!("应写出文件");
    };
    std::fs::read(path).unwrap()
}

fn rewrite(src: &Path, dst: &Path, edit: impl Fn(&str, &mut String)) {
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
            if name.ends_with(".xml") {
                let mut xml = String::from_utf8(data).unwrap();
                edit(&name, &mut xml);
                data = xml.into_bytes();
            }
            zip_out.start_file(&name, opt).unwrap();
            zip_out.write_all(&data).unwrap();
        }
        zip_out.finish().unwrap();
    }
    std::fs::write(dst, out.into_inner()).unwrap();
}

fn paint_red(src: &Path) -> PathBuf {
    let dst = tmp("red");
    rewrite(src, &dst, |name, xml| {
        if name == "xl/worksheets/sheet1.xml" && !xml.contains("showGridLines") {
            *xml = xml.replace(
                "<sheetData>",
                "<sheetViews><sheetView showGridLines=\"0\" workbookViewId=\"0\"/></sheetViews><sheetData>",
            );
        }
        if name.starts_with("xl/drawings/drawing") {
            *xml = xml.replace("val=\"FFFFFF\"", "val=\"EE4455\"").replace(
                "<a:ln w=\"9525\"><a:solidFill><a:srgbClr val=\"000000\"/></a:solidFill></a:ln>",
                "<a:ln><a:noFill/></a:ln>",
            );
        }
    });
    dst
}

fn assert_block(rgb: &[u8], width: u32, x0: u32, x1: u32, y0: u32, y1: u32) {
    for y in y0..y1 {
        for x in x0..x1 {
            assert_eq!(pixel(rgb, width, x, y), RED, "({x},{y})");
        }
    }
}

#[test]
fn plain_rectangles_follow_the_measured_edges() {
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
    let shaped = tmp("shaped");
    let px = 9525;
    run_ok(json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": shaped.to_string_lossy(),
        "ops": [
            { "op": "setRowHeight", "id": "sheet:Sheet1", "row": 1, "height": 15 },
            { "op": "setRowHeight", "id": "sheet:Sheet1", "row": 2, "height": 15 },
            { "op": "setRowHeight", "id": "sheet:Sheet1", "row": 3, "height": 15 },
            { "op": "insertShape", "id": "sheet:Sheet1", "prst": "rect", "anchor": "oneCell",
              "from": "A1", "cx": 8 * px, "cy": 8 * px },
            { "op": "insertShape", "id": "sheet:Sheet1", "prst": "rect", "anchor": "oneCell",
              "from": "A2", "fromOffX": 5 * px, "fromOffY": 5 * px, "cx": 20 * px, "cy": 8 * px },
            { "op": "insertShape", "id": "sheet:Sheet1", "prst": "rect", "anchor": "oneCell",
              "from": "A3", "fromOffX": 4763, "cx": 16 * px, "cy": 8 * px }
        ]
    }));
    let book = paint_red(&shaped);
    let (w, h, rgb) = decode_png(&export(&book));
    assert_eq!((w, h), (65, 61));
    assert_eq!(pixel(&rgb, w, 0, 1), [255, 255, 255]);
    assert_eq!(pixel(&rgb, w, 1, 0), [255, 255, 255]);
    assert_block(&rgb, w, 1, 8, 1, 8);
    assert_eq!(pixel(&rgb, w, 8, 1), [255, 255, 255]);
    assert_block(&rgb, w, 5, 25, 25, 33);
    assert_eq!(pixel(&rgb, w, 4, 25), [255, 255, 255]);
    assert_eq!(pixel(&rgb, w, 25, 25), [255, 255, 255]);
    assert_block(&rgb, w, 1, 17, 40, 48);
    assert_eq!(pixel(&rgb, w, 16, 40), RED);
    assert_eq!(pixel(&rgb, w, 17, 40), [255, 255, 255]);
}

#[test]
fn rectangle_covers_an_interior_border_and_leaves_the_frame() {
    let src = tmp("src");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": src.to_string_lossy(),
        "workbook": { "sheets": [{ "name": "Sheet1", "cells": [
            { "ref": "A1", "value": "" },
            { "ref": "B1", "value": "" }
        ] }] }
    }));
    let shaped = tmp("shaped");
    let px = 9525;
    run_ok(json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": shaped.to_string_lossy(),
        "ops": [
            { "op": "setRowHeight", "id": "sheet:Sheet1", "row": 1, "height": 15 },
            { "op": "setCellStyle", "id": "cell:Sheet1!A1", "border": {
                "right": { "style": "thin", "color": { "rgb": "FF000000" } }
            } },
            { "op": "insertShape", "id": "sheet:Sheet1", "prst": "rect", "anchor": "oneCell",
              "from": "A1", "fromOffX": 60 * px, "cx": 16 * px, "cy": 10 * px }
        ]
    }));
    let book = paint_red(&shaped);
    let (w, h, rgb) = decode_png(&export(&book));
    assert_eq!((w, h), (129, 21));
    // 格子没有上边，上面也没有底边，竖线从上边界涂起。
    assert_eq!(pixel(&rgb, w, 64, 0), [0, 0, 0]);
    assert_block(&rgb, w, 60, 76, 1, 10);
    assert_eq!(pixel(&rgb, w, 64, 5), RED);
    assert_eq!(pixel(&rgb, w, 64, 10), [0, 0, 0]);
}

#[test]
fn a_rectangle_with_a_line_still_fails_the_sheet() {
    let src = tmp("src");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": src.to_string_lossy(),
        "workbook": { "sheets": [{ "name": "Sheet1", "cells": [
            { "ref": "A1", "value": "" }
        ] }] }
    }));
    let shaped = tmp("shaped");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": shaped.to_string_lossy(),
        "ops": [{
            "op": "insertShape", "id": "sheet:Sheet1", "prst": "rect",
            "anchor": "oneCell", "from": "A1", "cx": 95250, "cy": 95250
        }]
    }));
    let err = run(&json!({
        "schemaVersion": 1,
        "action": "exportSheetImage",
        "path": shaped.to_string_lossy(),
        "id": "sheet:Sheet1",
        "output": tmp("out").with_extension("png").to_string_lossy(),
    })
    .to_string())
    .unwrap_err();
    let text = err.to_string();
    assert!(
        text.contains("工作表") || text.contains("worksheet"),
        "{text}"
    );
}
