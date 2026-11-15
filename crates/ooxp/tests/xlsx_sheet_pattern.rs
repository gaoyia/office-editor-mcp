//! 两种纯色的图案按整张图的像素铺。点不从格子左上角重算。

use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-xlsx-sheet-pattern");
    std::fs::create_dir_all(&dir).unwrap();
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    dir.join(format!("{tag}-{n}.xlsx"))
}

fn run_ok(value: Value) -> RunOutput {
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

fn rewrite_part(src: &Path, part: &str, edit: impl FnOnce(String) -> String) -> PathBuf {
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
            if name == part {
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

fn cells() -> Vec<Value> {
    let mut out = Vec::new();
    for row in 1..=4 {
        for col in ["A", "B", "C", "D", "E"] {
            out.push(json!({ "ref": format!("{col}{row}"), "value": "" }));
        }
    }
    out
}

fn style(reference: &str, pattern: &str, fg: Option<&str>, bg: Option<&str>) -> Value {
    let mut fill = serde_json::Map::new();
    fill.insert("pattern".into(), json!(pattern));
    if let Some(fg) = fg {
        fill.insert("fg".into(), json!({ "rgb": fg }));
    }
    if let Some(bg) = bg {
        fill.insert("bg".into(), json!({ "rgb": bg }));
    }
    json!({
        "op": "setCellStyle",
        "id": format!("cell:Sheet1!{reference}"),
        "fill": fill
    })
}

/// 列 A 宽 10（75 像素），第 1 行高 16 磅（21 像素）。其余列 64，其余行 20。
fn patterned() -> PathBuf {
    let src = tmp("src");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": src.to_string_lossy(),
        "workbook": { "sheets": [{ "name": "Sheet1", "cells": cells() }] }
    }));
    let styled = tmp("styled");
    let mut ops = vec![
        json!({ "op": "setColWidth", "id": "sheet:Sheet1", "col": "A", "width": 10 }),
        json!({ "op": "setRowHeight", "id": "sheet:Sheet1", "row": 1, "height": 16 }),
    ];
    let specs = [
        ("A1", "gray125", Some("FF000000"), Some("FFFFFFFF")),
        ("B1", "gray0625", Some("FF000000"), Some("FFFFFFFF")),
        ("C1", "lightHorizontal", Some("FF000000"), Some("FFFFFFFF")),
        ("D1", "darkHorizontal", Some("FF000000"), Some("FFFFFFFF")),
        ("E1", "mediumGray", Some("FF000000"), Some("FFFFFFFF")),
        ("A2", "lightVertical", Some("FF000000"), Some("FFFFFFFF")),
        ("B2", "darkVertical", Some("FF000000"), Some("FFFFFFFF")),
        ("C2", "lightGrid", Some("FF000000"), Some("FFFFFFFF")),
        ("D2", "darkGrid", Some("FF000000"), Some("FFFFFFFF")),
        ("E2", "lightGray", Some("FF000000"), Some("FFFFFFFF")),
        ("A3", "darkGray", Some("FF000000"), Some("FFFFFFFF")),
        ("B3", "darkDown", Some("FF000000"), Some("FFFFFFFF")),
        ("C3", "lightDown", Some("FF000000"), Some("FFFFFFFF")),
        ("D3", "darkUp", Some("FF000000"), Some("FFFFFFFF")),
        ("E3", "lightUp", Some("FF000000"), Some("FFFFFFFF")),
        ("A4", "darkTrellis", Some("FF000000"), Some("FFFFFFFF")),
        ("B4", "lightTrellis", Some("FF000000"), Some("FFFFFFFF")),
        ("C4", "gray125", Some("FFEE4455"), Some("FF112233")),
        ("D4", "gray125", None, None),
        ("E4", "gray125", Some("FFEE4455"), None),
    ];
    for (reference, pattern, fg, bg) in specs {
        ops.push(style(reference, pattern, fg, bg));
    }
    run_ok(json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": styled.to_string_lossy(),
        "ops": ops
    }));
    rewrite_part(&styled, "xl/worksheets/sheet1.xml", |xml| {
        let start = xml.find("<dimension ").expect("dimension");
        let end = xml[start..].find("/>").expect("dimension end") + start + 2;
        let views = r#"<sheetViews><sheetView showGridLines="0" workbookViewId="0"/></sheetViews>"#;
        format!("{}{views}{}", &xml[..end], &xml[end..])
    })
}

fn dot(kind: &str, x: i32, y: i32) -> bool {
    let x4 = x.rem_euclid(4);
    let y4 = y.rem_euclid(4);
    match kind {
        "gray125" => (y4 == 2 && x4 == 2) || (y4 == 0 && x4 == 0),
        "gray0625" => {
            let x8 = x.rem_euclid(8);
            let y8 = y.rem_euclid(8);
            ((y8 == 2 || y8 == 6) && x8 == 4) || ((y8 == 0 || y8 == 4) && x8 == 0)
        }
        "lightHorizontal" => y4 == 0,
        "darkHorizontal" => y4 == 0 || y4 == 1,
        "lightVertical" => x4 == 0,
        "darkVertical" => x4 == 0 || x4 == 1,
        "lightGrid" => x4 == 0 || y4 == 0,
        "darkGrid" => {
            if y4 == 1 || y4 == 2 {
                x4 == 1 || x4 == 2
            } else {
                x4 == 0 || x4 == 3
            }
        }
        "mediumGray" => (x + y).rem_euclid(2) == 0,
        "lightGray" => {
            if y.rem_euclid(2) == 1 {
                x4 == 2
            } else {
                x4 == 0
            }
        }
        "darkGray" => {
            if y.rem_euclid(2) == 1 {
                x4 != 1
            } else {
                x4 != 3
            }
        }
        "darkDown" => (x - y).rem_euclid(4) <= 1,
        "lightDown" => x4 == y4,
        "darkUp" => {
            let sum = (x + y).rem_euclid(4);
            sum == 2 || sum == 3
        }
        "lightUp" => (x + y).rem_euclid(4) == 3,
        "darkTrellis" => {
            y.rem_euclid(2) == 0 || {
                let sum = (x + y).rem_euclid(4);
                sum == 2 || sum == 3
            }
        }
        "lightTrellis" => {
            if y.rem_euclid(2) == 0 {
                x4 == y4
            } else {
                x.rem_euclid(2) == 1
            }
        }
        _ => panic!("未知图案 {kind}"),
    }
}

fn owner(origins: &[i32], at: i32) -> usize {
    let mut index = 0;
    for i in 0..origins.len() - 1 {
        if at >= origins[i] && at <= origins[i + 1] {
            index = i;
        }
    }
    index
}

#[test]
fn patterns_follow_the_picture_pixel() {
    let (w, h, px) = decode_png(&export(&patterned()).unwrap());
    assert_eq!((w, h), (332, 82));
    assert_eq!(pixel(&px, w, 0, 0), [255, 255, 255]);
    // (2,2) 是 gray125 的点。若从格子可见左缘重算，这里会变成底。
    assert_eq!(pixel(&px, w, 2, 2), [0, 0, 0]);
    // B 列起点是 75。76 在整张图上是 gray0625 的点，按格子原点则是底。
    assert_eq!(pixel(&px, w, 76, 2), [0, 0, 0]);
    // C4 的前景是点，背景是底。
    assert_eq!(pixel(&px, w, 140, 64), [0xEE, 0x44, 0x55]);
    assert_eq!(pixel(&px, w, 139, 64), [0x11, 0x22, 0x33]);
    // 没写颜色时点是黑、底是白。只写前景时底仍是白。
    assert_eq!(pixel(&px, w, 204, 64), [0, 0, 0]);
    assert_eq!(pixel(&px, w, 203, 64), [255, 255, 255]);
    assert_eq!(pixel(&px, w, 268, 64), [0xEE, 0x44, 0x55]);
    assert_eq!(pixel(&px, w, 267, 64), [255, 255, 255]);

    let xs = [0, 75, 139, 203, 267, 331];
    let ys = [0, 21, 41, 61, 81];
    let kinds = [
        [
            "gray125",
            "gray0625",
            "lightHorizontal",
            "darkHorizontal",
            "mediumGray",
        ],
        [
            "lightVertical",
            "darkVertical",
            "lightGrid",
            "darkGrid",
            "lightGray",
        ],
        ["darkGray", "darkDown", "lightDown", "darkUp", "lightUp"],
        [
            "darkTrellis",
            "lightTrellis",
            "gray125",
            "gray125",
            "gray125",
        ],
    ];
    for y in 0..h {
        for x in 0..w {
            let expect = if x == 0 || y == 0 {
                [255, 255, 255]
            } else {
                let col = owner(&xs, x as i32);
                let row = owner(&ys, y as i32);
                let (fg, bg) = match (col, row) {
                    (2, 3) => ([0xEE, 0x44, 0x55], [0x11, 0x22, 0x33]),
                    (4, 3) => ([0xEE, 0x44, 0x55], [255, 255, 255]),
                    _ => ([0, 0, 0], [255, 255, 255]),
                };
                if dot(kinds[row][col], x as i32, y as i32) {
                    fg
                } else {
                    bg
                }
            };
            assert_eq!(pixel(&px, w, x, y), expect, "像素 {x},{y}");
        }
    }
}

#[test]
fn unknown_pattern_gradient_and_translucent_fail() {
    let unknown = rewrite_part(&patterned(), "xl/styles.xml", |xml| {
        xml.replace("patternType=\"lightUp\"", "patternType=\"notAPattern\"")
    });
    assert_eq!(
        export(&unknown).unwrap_err().code(),
        ErrorCode::NotImplemented
    );

    let gradient = rewrite_part(&patterned(), "xl/styles.xml", |xml| {
        xml.replace("patternFill", "gradientFill")
    });
    assert_eq!(
        export(&gradient).unwrap_err().code(),
        ErrorCode::NotImplemented
    );

    let translucent = rewrite_part(&patterned(), "xl/styles.xml", |xml| {
        xml.replace("FFEE4455", "80EE4455")
    });
    assert_eq!(
        export(&translucent).unwrap_err().code(),
        ErrorCode::NotImplemented
    );

    let dxf = rewrite_part(&patterned(), "xl/styles.xml", |xml| {
        xml.replace(
            "</styleSheet>",
            r#"<dxfs count="1"><dxf><fill><patternFill patternType="gray125"><fgColor rgb="FF000000"/><bgColor rgb="FFFFFFFF"/></patternFill></fill></dxf></dxfs></styleSheet>"#,
        )
    });
    assert_eq!(export(&dxf).unwrap_err().code(), ErrorCode::NotImplemented);
}
