//! 公式条件格式的纯色，铺在和单元格填充同一块矩形上。

use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
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

fn quiet_row(rgb: &[u8], width: u32, y: u32, color: [u8; 3]) {
    for x in 1..width {
        assert_eq!(pixel(rgb, width, x, y), color, "({x},{y})");
    }
    assert_eq!(pixel(rgb, width, 0, y), [255, 255, 255], "外框 ({y})");
}

fn rank_book(cells: serde_json::Value, rule: serde_json::Value) -> PathBuf {
    let src = tmp("src");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": src.to_string_lossy(),
        "workbook": { "sheets": [{ "name": "Sheet1", "cells": cells.clone() }] }
    }));
    let book = tmp("rank");
    let mut ops = vec![
        json!({ "op": "setRowHeight", "id": "sheet:Sheet1", "row": 1, "height": 15 }),
        json!({ "op": "setRowHeight", "id": "sheet:Sheet1", "row": 2, "height": 15 }),
        json!({ "op": "setRowHeight", "id": "sheet:Sheet1", "row": 3, "height": 15 }),
    ];
    let white = json!({ "color": { "rgb": "FFFFFFFF" } });
    if let Some(list) = cells.as_array() {
        for cell in list {
            let r = cell["ref"].as_str().unwrap();
            ops.push(json!({
                "op": "setCellStyle",
                "id": format!("cell:Sheet1!{r}"),
                "font": white.clone()
            }));
        }
    }
    ops.push(rule);
    run_ok(json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": book.to_string_lossy(),
        "ops": ops
    }));
    hide_grid(&book)
}

#[test]
fn above_average_paints_only_the_strictly_greater_cell() {
    let dxf = json!({
        "fill": { "pattern": "solid", "fg": { "rgb": "FFEE4455" } },
        "font": { "color": { "rgb": "FFEE4455" } }
    });
    let book = rank_book(
        json!([
            { "ref": "A1", "value": 1 },
            { "ref": "A2", "value": 2 },
            { "ref": "A3", "value": 3 }
        ]),
        json!({
            "op": "insertConditionalFormat", "id": "sheet:Sheet1", "sqref": "A1:A3",
            "type": "aboveAverage", "dxf": dxf
        }),
    );
    let (w, h, rgb) = decode_png(&export(&book).unwrap());
    assert_eq!((w, h), (65, 61));
    quiet_row(&rgb, w, 10, [255, 255, 255]);
    quiet_row(&rgb, w, 30, [255, 255, 255]);
    quiet_row(&rgb, w, 50, RED);
}

#[test]
fn population_stddev_top_rank_and_text_use_the_same_fill() {
    let dxf = json!({
        "fill": { "pattern": "solid", "fg": { "rgb": "FFEE4455" } },
        "font": { "color": { "rgb": "FFEE4455" } }
    });
    let sd = rank_book(
        json!([
            { "ref": "A1", "value": 0 },
            { "ref": "A2", "value": 2 },
            { "ref": "A3", "value": 3 }
        ]),
        json!({
            "op": "insertConditionalFormat", "id": "sheet:Sheet1", "sqref": "A1:A3",
            "type": "aboveAverage", "stdDev": 1, "dxf": dxf
        }),
    );
    let (w, _, rgb) = decode_png(&export(&sd).unwrap());
    quiet_row(&rgb, w, 10, [255, 255, 255]);
    quiet_row(&rgb, w, 50, RED);

    let top = rank_book(
        json!([
            { "ref": "A1", "value": 5 },
            { "ref": "A2", "value": 1 },
            { "ref": "A3", "value": 5 }
        ]),
        json!({
            "op": "insertConditionalFormat", "id": "sheet:Sheet1", "sqref": "A1:A3",
            "type": "top10", "rank": 1, "dxf": dxf
        }),
    );
    let (w, _, rgb) = decode_png(&export(&top).unwrap());
    quiet_row(&rgb, w, 10, RED);
    quiet_row(&rgb, w, 30, [255, 255, 255]);
    quiet_row(&rgb, w, 50, RED);

    let text = rank_book(
        json!([
            { "ref": "A1", "value": "abc" },
            { "ref": "A2", "value": "zz" },
            { "ref": "A3", "value": "Ab" }
        ]),
        json!({
            "op": "insertConditionalFormat", "id": "sheet:Sheet1", "sqref": "A1:A3",
            "type": "containsText", "text": "ab", "dxf": dxf
        }),
    );
    let (w, _, rgb) = decode_png(&export(&text).unwrap());
    quiet_row(&rgb, w, 10, RED);
    quiet_row(&rgb, w, 30, [255, 255, 255]);
    quiet_row(&rgb, w, 50, RED);
}

#[test]
fn not_contains_paints_a_missing_cell() {
    let book = rank_book(
        json!([
            { "ref": "A1", "value": "abc" },
            { "ref": "A3", "value": "zz" }
        ]),
        json!({
            "op": "insertConditionalFormat", "id": "sheet:Sheet1", "sqref": "A1:A3",
            "type": "notContainsText", "text": "ab",
            "dxf": {
                "fill": { "pattern": "solid", "fg": { "rgb": "FFEE4455" } },
                "font": { "color": { "rgb": "FFEE4455" } }
            }
        }),
    );
    let (w, h, rgb) = decode_png(&export(&book).unwrap());
    assert_eq!((w, h), (65, 61));
    quiet_row(&rgb, w, 10, [255, 255, 255]);
    quiet_row(&rgb, w, 30, RED);
    quiet_row(&rgb, w, 50, RED);
}

fn duplicate_book(
    cells: serde_json::Value,
    kind: &str,
    rows: u32,
    formula: Option<&str>,
) -> PathBuf {
    let src = tmp("dup-src");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": src.to_string_lossy(),
        "workbook": { "sheets": [{ "name": "Sheet1", "cells": cells.clone() }] }
    }));
    let book = tmp("dup");
    let mut ops = Vec::new();
    let list = cells.as_array().unwrap();
    let white = json!({ "color": { "rgb": "FFFFFFFF" } });
    for row in 1..=rows {
        ops.push(json!({
            "op": "setRowHeight",
            "id": "sheet:Sheet1",
            "row": row,
            "height": 15
        }));
    }
    for cell in list {
        let r = cell["ref"].as_str().unwrap();
        ops.push(json!({
            "op": "setCellStyle",
            "id": format!("cell:Sheet1!{r}"),
            "font": white.clone()
        }));
    }
    run_ok(json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": book.to_string_lossy(),
        "ops": ops
    }));
    let bytes = std::fs::read(&book).unwrap();
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
            if name == "xl/styles.xml" {
                let mut xml = String::from_utf8(data).unwrap();
                let dxf = "<dxfs count=\"1\"><dxf><font><color rgb=\"FFEE4455\"/></font><fill><patternFill><bgColor rgb=\"FFEE4455\"/></patternFill></fill></dxf></dxfs>";
                xml = xml.replace("</styleSheet>", &format!("{dxf}</styleSheet>"));
                data = xml.into_bytes();
            }
            if name == "xl/worksheets/sheet1.xml" {
                let mut xml = String::from_utf8(data).unwrap();
                if !xml.contains("showGridLines") {
                    xml = xml.replace(
                        "<sheetData>",
                        "<sheetViews><sheetView showGridLines=\"0\" workbookViewId=\"0\"/></sheetViews><sheetData>",
                    );
                }
                let last = list.last().unwrap()["ref"].as_str().unwrap();
                let formula_xml = match formula {
                    Some(text) => format!(
                        "<formula>{}</formula>",
                        text.replace('&', "&amp;")
                            .replace('<', "&lt;")
                            .replace('>', "&gt;")
                    ),
                    None => String::new(),
                };
                let rule = format!(
                    "<conditionalFormatting sqref=\"A1:{last}\"><cfRule type=\"{kind}\" dxfId=\"0\" priority=\"1\">{formula_xml}</cfRule></conditionalFormatting>"
                );
                xml = xml.replace("</worksheet>", &format!("{rule}</worksheet>"));
                data = xml.into_bytes();
            }
            zip_out.start_file(&name, opt).unwrap();
            zip_out.write_all(&data).unwrap();
        }
        zip_out.finish().unwrap();
    }
    let dst = tmp("dup-grid");
    std::fs::write(&dst, out.into_inner()).unwrap();
    dst
}

#[test]
fn duplicate_values_paint_numeric_text_and_ignore_case() {
    let book = duplicate_book(
        json!([
            { "ref": "A1", "value": 1 },
            { "ref": "A2", "value": 2 },
            { "ref": "A3", "value": "1.00" },
            { "ref": "A4", "value": "" },
            { "ref": "A5", "value": "Ab" },
            { "ref": "A6", "value": "ab" }
        ]),
        "duplicateValues",
        6,
        None,
    );
    let (w, h, rgb) = decode_png(&export(&book).unwrap());
    assert_eq!((w, h), (65, 121));
    quiet_row(&rgb, w, 10, RED);
    quiet_row(&rgb, w, 30, [255, 255, 255]);
    quiet_row(&rgb, w, 50, RED);
    quiet_row(&rgb, w, 70, [255, 255, 255]);
    quiet_row(&rgb, w, 90, RED);
    quiet_row(&rgb, w, 110, RED);
    assert_eq!(pixel(&rgb, w, 1, 1), RED);
    assert_eq!(pixel(&rgb, w, 1, 41), [0x00, 0x80, 0x00]);
    assert_eq!(pixel(&rgb, w, 6, 41), [0x00, 0x80, 0x00]);
    assert_eq!(pixel(&rgb, w, 7, 41), RED);
    assert_eq!(pixel(&rgb, w, 1, 46), [0x00, 0x80, 0x00]);
    assert_eq!(pixel(&rgb, w, 2, 46), RED);
}

#[test]
fn unparsed_numeric_text_fails_the_sheet() {
    let book = duplicate_book(
        json!([
            { "ref": "A1", "value": "$1" },
            { "ref": "A2", "value": 1 }
        ]),
        "duplicateValues",
        2,
        None,
    );
    assert_eq!(export(&book).unwrap_err().code(), ErrorCode::NotImplemented);
}

#[test]
fn unique_values_paint_only_the_single_occurrence() {
    let book = duplicate_book(
        json!([
            { "ref": "A1", "value": 1 },
            { "ref": "A2", "value": 2 },
            { "ref": "A3", "value": 1 },
            { "ref": "A4", "value": "1.00" }
        ]),
        "uniqueValues",
        4,
        None,
    );
    let (w, h, rgb) = decode_png(&export(&book).unwrap());
    assert_eq!((w, h), (65, 81));
    quiet_row(&rgb, w, 10, [255, 255, 255]);
    quiet_row(&rgb, w, 30, RED);
    quiet_row(&rgb, w, 50, [255, 255, 255]);
    quiet_row(&rgb, w, 70, [255, 255, 255]);
    assert_eq!(pixel(&rgb, w, 1, 61), [0x00, 0x80, 0x00]);
    assert_eq!(pixel(&rgb, w, 7, 61), [255, 255, 255]);
}

#[test]
fn blanks_fill_a_missing_cell_and_spaces() {
    let book = duplicate_book(
        json!([
            { "ref": "A1", "value": 1 },
            { "ref": "A3", "value": "  " },
            { "ref": "A4", "value": "a" }
        ]),
        "containsBlanks",
        4,
        Some("LEN(TRIM(A1))=0"),
    );
    let (w, h, rgb) = decode_png(&export(&book).unwrap());
    assert_eq!((w, h), (65, 81));
    quiet_row(&rgb, w, 10, [255, 255, 255]);
    quiet_row(&rgb, w, 30, RED);
    quiet_row(&rgb, w, 50, RED);
    quiet_row(&rgb, w, 70, [255, 255, 255]);
}

#[test]
fn not_contains_blanks_skips_empty_cells() {
    let book = duplicate_book(
        json!([
            { "ref": "A1", "value": 1 },
            { "ref": "A3", "value": "  " },
            { "ref": "A4", "value": "a" }
        ]),
        "notContainsBlanks",
        4,
        Some("len(trim(a1))>0"),
    );
    let (w, h, rgb) = decode_png(&export(&book).unwrap());
    assert_eq!((w, h), (65, 81));
    quiet_row(&rgb, w, 10, RED);
    quiet_row(&rgb, w, 30, [255, 255, 255]);
    quiet_row(&rgb, w, 50, [255, 255, 255]);
    quiet_row(&rgb, w, 70, RED);
}

#[test]
fn a_blank_rule_without_formula_does_not_fill() {
    let book = duplicate_book(
        json!([
            { "ref": "A1", "value": 1 },
            { "ref": "A3", "value": "  " },
            { "ref": "A4", "value": "a" }
        ]),
        "containsBlanks",
        4,
        None,
    );
    let (w, h, rgb) = decode_png(&export(&book).unwrap());
    assert_eq!((w, h), (65, 81));
    quiet_row(&rgb, w, 10, [255, 255, 255]);
    quiet_row(&rgb, w, 30, [255, 255, 255]);
    quiet_row(&rgb, w, 50, [255, 255, 255]);
    quiet_row(&rgb, w, 70, [255, 255, 255]);
}

#[test]
fn a_foreign_blank_formula_fails_the_sheet() {
    let book = duplicate_book(
        json!([
            { "ref": "A1", "value": 1 },
            { "ref": "A2", "value": "" }
        ]),
        "containsBlanks",
        2,
        Some("A1=1"),
    );
    assert_eq!(export(&book).unwrap_err().code(), ErrorCode::NotImplemented);
}
