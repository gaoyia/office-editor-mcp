//! 工作表上没有描边的纯色矩形，按 Excel 16.0 量过的像素来画。

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
fn quarter_turn_uses_the_anchor_box() {
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
        "ops": [
            { "op": "setRowHeight", "id": "sheet:Sheet1", "row": 1, "height": 15 },
            { "op": "insertShape", "id": "sheet:Sheet1", "prst": "rect", "anchor": "oneCell",
              "from": "A1", "cx": 8 * 9525, "cy": 8 * 9525 }
        ]
    }));
    let turned = tmp("turn");
    rewrite(&paint_red(&shaped), &turned, |name, xml| {
        if name.starts_with("xl/drawings/drawing") {
            *xml = xml
                .replace("<a:xfrm>", "<a:xfrm rot=\"5400000\">")
                .replace(
                    "<a:ln><a:noFill/></a:ln>",
                    "<a:ln w=\"19050\"><a:noFill/><a:prstDash val=\"solid\"/><a:miter lim=\"800000\"/></a:ln>",
                );
        }
    });
    let (w, h, rgb) = decode_png(&export(&turned));
    assert_eq!((w, h), (65, 21));
    assert_block(&rgb, w, 1, 8, 1, 8);
    assert_eq!(pixel(&rgb, w, 8, 1), [255, 255, 255]);

    for raw in [
        "rot=\"16200000\"",
        "flipH=\"1\"",
        "flipV=\"1\"",
        "rot=\"10800000\" flipH=\"1\" flipV=\"1\"",
    ] {
        let book = tmp("flip");
        rewrite(&paint_red(&shaped), &book, |name, xml| {
            if name.starts_with("xl/drawings/drawing") {
                *xml = xml.replace("<a:xfrm>", &format!("<a:xfrm {raw}>"));
            }
        });
        let (w, h, rgb) = decode_png(&export(&book));
        assert_eq!((w, h), (65, 21), "{raw}");
        assert_block(&rgb, w, 1, 8, 1, 8);
    }

    let odd = tmp("odd");
    rewrite(&turned, &odd, |name, xml| {
        if name.starts_with("xl/drawings/drawing") {
            *xml = xml.replace("rot=\"5400000\"", "rot=\"100\"");
        }
    });
    let err = run(&json!({
        "schemaVersion": 1,
        "action": "exportSheetImage",
        "path": odd.to_string_lossy(),
        "id": "sheet:Sheet1",
        "output": odd.with_extension("png").to_string_lossy(),
    })
    .to_string())
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::NotImplemented);
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

#[test]
fn an_identity_group_paints_the_same_rectangles() {
    let book = grouped_book(GROUP_XML);
    let (w, h, rgb) = decode_png(&export(&book));
    assert_eq!((w, h), (65, 41));
    assert_block(&rgb, w, 1, 9, 1, 9);
    assert_eq!(pixel(&rgb, w, 9, 1), [255, 255, 255]);
    assert_block(&rgb, w, 20, 26, 1, 5);
    assert_eq!(pixel(&rgb, w, 19, 1), [255, 255, 255]);
    assert_eq!(pixel(&rgb, w, 26, 1), [255, 255, 255]);
}

#[test]
fn an_identity_group_can_cross_a_column() {
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
    run_ok(json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": shaped.to_string_lossy(),
        "ops": [
            { "op": "setRowHeight", "id": "sheet:Sheet1", "row": 1, "height": 30 },
            { "op": "insertShape", "id": "sheet:Sheet1", "prst": "rect", "anchor": "oneCell",
              "from": "A1", "cx": 9525, "cy": 9525 }
        ]
    }));
    let book = tmp("cross");
    rewrite(&shaped, &book, |name, xml| {
        if name == "xl/worksheets/sheet1.xml" && !xml.contains("showGridLines") {
            *xml = xml.replace(
                "<sheetData>",
                "<sheetViews><sheetView showGridLines=\"0\" workbookViewId=\"0\"/></sheetViews><sheetData>",
            );
        }
        if name.starts_with("xl/drawings/drawing") {
            *xml = CROSS_XML.to_string();
        }
    });
    let (w, h, rgb) = decode_png(&export(&book));
    assert_eq!((w, h), (129, 41));
    assert_block(&rgb, w, 8, 24, 8, 20);
    assert_eq!(pixel(&rgb, w, 24, 8), [255, 255, 255]);
    assert_eq!(pixel(&rgb, w, 71, 8), [255, 255, 255]);
    assert_block(&rgb, w, 72, 82, 8, 20);
    assert_eq!(pixel(&rgb, w, 82, 8), [255, 255, 255]);
}

#[test]
fn an_identity_group_can_cross_a_row() {
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
    run_ok(json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": shaped.to_string_lossy(),
        "ops": [
            { "op": "setRowHeight", "id": "sheet:Sheet1", "row": 1, "height": 30 },
            { "op": "setRowHeight", "id": "sheet:Sheet1", "row": 2, "height": 30 },
            { "op": "setRowHeight", "id": "sheet:Sheet1", "row": 3, "height": 30 },
            { "op": "insertShape", "id": "sheet:Sheet1", "prst": "rect", "anchor": "oneCell",
              "from": "A1", "cx": 9525, "cy": 9525 }
        ]
    }));
    let book = tmp("down");
    rewrite(&shaped, &book, |name, xml| {
        if name == "xl/worksheets/sheet1.xml" && !xml.contains("showGridLines") {
            *xml = xml.replace(
                "<sheetData>",
                "<sheetViews><sheetView showGridLines=\"0\" workbookViewId=\"0\"/></sheetViews><sheetData>",
            );
        }
        if name.starts_with("xl/drawings/drawing") {
            *xml = ROW_XML.to_string();
        }
    });
    let (w, h, rgb) = decode_png(&export(&book));
    assert_eq!((w, h), (65, 121));
    assert_block(&rgb, w, 8, 20, 48, 58);
    assert_eq!(pixel(&rgb, w, 8, 47), [255, 255, 255]);
    assert_eq!(pixel(&rgb, w, 8, 58), [255, 255, 255]);
    assert_block(&rgb, w, 8, 20, 88, 98);
    assert_eq!(pixel(&rgb, w, 8, 87), [255, 255, 255]);
    assert_eq!(pixel(&rgb, w, 8, 98), [255, 255, 255]);
}

#[test]
fn a_scaled_group_fails_the_sheet() {
    let xml = GROUP_XML.replace(
        r#"<a:chExt cx="238125" cy="76200"/>"#,
        r#"<a:chExt cx="100000" cy="76200"/>"#,
    );
    let book = grouped_book(&xml);
    let err = run(&json!({
        "schemaVersion": 1,
        "action": "exportSheetImage",
        "path": book.to_string_lossy(),
        "id": "sheet:Sheet1",
        "output": tmp("scaled-out").with_extension("png").to_string_lossy(),
    })
    .to_string())
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::NotImplemented);
}

fn grouped_book(drawing: &str) -> PathBuf {
    let src = tmp("group-src");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": src.to_string_lossy(),
        "workbook": { "sheets": [{ "name": "Sheet1", "cells": [{ "ref": "A1", "value": "" }] }] }
    }));
    let shaped = tmp("group-shaped");
    let px = 9525;
    run_ok(json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": shaped.to_string_lossy(),
        "ops": [
            { "op": "setRowHeight", "id": "sheet:Sheet1", "row": 1, "height": 30 },
            { "op": "insertShape", "id": "sheet:Sheet1", "prst": "rect", "anchor": "oneCell",
              "from": "A1", "cx": 8 * px, "cy": 8 * px }
        ]
    }));
    let book = tmp("group");
    rewrite(&shaped, &book, |name, xml| {
        if name == "xl/worksheets/sheet1.xml" && !xml.contains("showGridLines") {
            *xml = xml.replace(
                "<sheetData>",
                "<sheetViews><sheetView showGridLines=\"0\" workbookViewId=\"0\"/></sheetViews><sheetData>",
            );
        }
        if name.starts_with("xl/drawings/drawing") {
            *xml = drawing.to_string();
        }
    });
    book
}

#[test]
fn a_theme_color_with_no_transform_is_flat() {
    let shaped = one_rect();
    let painted = tmp("scheme");
    rewrite(&shaped, &painted, |name, xml| {
        if name == "xl/worksheets/sheet1.xml" && !xml.contains("showGridLines") {
            *xml = xml.replace(
                "<sheetData>",
                "<sheetViews><sheetView showGridLines=\"0\" workbookViewId=\"0\"/></sheetViews><sheetData>",
            );
        }
        if name.starts_with("xl/drawings/drawing") {
            *xml = xml.replace(
                "<a:solidFill><a:srgbClr val=\"FFFFFF\"/></a:solidFill>",
                "<a:solidFill><a:schemeClr val=\"accent1\"/></a:solidFill>",
            );
            *xml = xml.replace(
                "<a:ln w=\"9525\"><a:solidFill><a:srgbClr val=\"000000\"/></a:solidFill></a:ln>",
                "<a:ln><a:noFill/></a:ln>",
            );
        }
    });
    let book = with_theme(&painted);
    let (w, h, rgb) = decode_png(&export(&book));
    assert_eq!((w, h), (65, 41));
    assert_block(&rgb, w, 1, 9, 1, 9);
    assert_eq!(pixel(&rgb, w, 9, 1), [255, 255, 255]);
}

#[test]
fn a_theme_color_transform_fails_the_sheet() {
    let shaped = one_rect();
    let painted = tmp("lum");
    rewrite(&shaped, &painted, |name, xml| {
        if name.starts_with("xl/drawings/drawing") {
            *xml = xml.replace(
                "<a:srgbClr val=\"FFFFFF\"/>",
                "<a:schemeClr val=\"accent1\"><a:lumMod val=\"50000\"/></a:schemeClr>",
            );
        }
    });
    let book = with_theme(&painted);
    let err = run(&json!({
        "schemaVersion": 1,
        "action": "exportSheetImage",
        "path": book.to_string_lossy(),
        "id": "sheet:Sheet1",
        "output": tmp("lum-out").with_extension("png").to_string_lossy(),
    })
    .to_string())
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::NotImplemented);
}

#[test]
fn a_theme_color_without_a_theme_part_fails_the_sheet() {
    let shaped = one_rect();
    let painted = tmp("no-theme");
    rewrite(&shaped, &painted, |name, xml| {
        if name.starts_with("xl/drawings/drawing") {
            *xml = xml.replace(
                "<a:srgbClr val=\"FFFFFF\"/>",
                "<a:schemeClr val=\"accent1\"/>",
            );
        }
    });
    let err = run(&json!({
        "schemaVersion": 1,
        "action": "exportSheetImage",
        "path": painted.to_string_lossy(),
        "id": "sheet:Sheet1",
        "output": tmp("no-theme-out").with_extension("png").to_string_lossy(),
    })
    .to_string())
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::NotImplemented);
}

fn one_rect() -> PathBuf {
    let src = tmp("scheme-src");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": src.to_string_lossy(),
        "workbook": { "sheets": [{ "name": "Sheet1", "cells": [{ "ref": "A1", "value": "" }] }] }
    }));
    let shaped = tmp("scheme-shaped");
    let px = 9525;
    run_ok(json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": shaped.to_string_lossy(),
        "ops": [
            { "op": "setRowHeight", "id": "sheet:Sheet1", "row": 1, "height": 30 },
            { "op": "insertShape", "id": "sheet:Sheet1", "prst": "rect", "anchor": "oneCell",
              "from": "A1", "fromOffX": px, "fromOffY": px, "cx": 8 * px, "cy": 8 * px }
        ]
    }));
    shaped
}

fn with_theme(src: &Path) -> PathBuf {
    let dest = tmp("themed");
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
            if name == "xl/_rels/workbook.xml.rels" {
                let text = String::from_utf8(data).unwrap().replace(
                    "</Relationships>",
                    r#"<Relationship Id="rIdTheme" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="theme/theme1.xml"/></Relationships>"#,
                );
                data = text.into_bytes();
            } else if name == "[Content_Types].xml" {
                let text = String::from_utf8(data).unwrap().replace(
                    "</Types>",
                    r#"<Override PartName="/xl/theme/theme1.xml" ContentType="application/vnd.openxmlformats-officedocument.theme+xml"/></Types>"#,
                );
                data = text.into_bytes();
            }
            zip_out.start_file(&name, opt).unwrap();
            zip_out.write_all(&data).unwrap();
        }
        zip_out.start_file("xl/theme/theme1.xml", opt).unwrap();
        zip_out.write_all(SCHEME_THEME.as_bytes()).unwrap();
        zip_out.finish().unwrap();
    }
    std::fs::write(&dest, out.into_inner()).unwrap();
    dest
}

const SCHEME_THEME: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="Probe"><a:themeElements><a:clrScheme name="Probe"><a:dk1><a:sysClr val="windowText" lastClr="010203"/></a:dk1><a:lt1><a:sysClr val="window" lastClr="FFFFFF"/></a:lt1><a:dk2><a:srgbClr val="1F497D"/></a:dk2><a:lt2><a:srgbClr val="EEECE1"/></a:lt2><a:accent1><a:srgbClr val="EE4455"/></a:accent1><a:accent2><a:srgbClr val="C0504D"/></a:accent2><a:accent3><a:srgbClr val="9BBB59"/></a:accent3><a:accent4><a:srgbClr val="8064A2"/></a:accent4><a:accent5><a:srgbClr val="4BACC6"/></a:accent5><a:accent6><a:srgbClr val="F79646"/></a:accent6><a:hlink><a:srgbClr val="0000FF"/></a:hlink><a:folHlink><a:srgbClr val="800080"/></a:folHlink></a:clrScheme></a:themeElements></a:theme>"#;

const CROSS_XML: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<xdr:wsDr xmlns:xdr="http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><xdr:twoCellAnchor><xdr:from><xdr:col>0</xdr:col><xdr:colOff>76200</xdr:colOff><xdr:row>0</xdr:row><xdr:rowOff>76200</xdr:rowOff></xdr:from><xdr:to><xdr:col>1</xdr:col><xdr:colOff>171450</xdr:colOff><xdr:row>0</xdr:row><xdr:rowOff>190500</xdr:rowOff></xdr:to><xdr:grpSp><xdr:nvGrpSpPr><xdr:cNvPr id="4" name="组合"/><xdr:cNvGrpSpPr/></xdr:nvGrpSpPr><xdr:grpSpPr><a:xfrm><a:off x="76200" y="76200"/><a:ext cx="704850" cy="114300"/><a:chOff x="76200" y="76200"/><a:chExt cx="704850" cy="114300"/></a:xfrm></xdr:grpSpPr><xdr:sp><xdr:nvSpPr><xdr:cNvPr id="2" name="形状 1"/><xdr:cNvSpPr/></xdr:nvSpPr><xdr:spPr><a:xfrm><a:off x="76200" y="76200"/><a:ext cx="152400" cy="114300"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:solidFill><a:srgbClr val="EE4455"/></a:solidFill><a:ln><a:noFill/></a:ln></xdr:spPr><xdr:txBody><a:bodyPr/><a:lstStyle/><a:p/></xdr:txBody></xdr:sp><xdr:sp><xdr:nvSpPr><xdr:cNvPr id="3" name="形状 2"/><xdr:cNvSpPr/></xdr:nvSpPr><xdr:spPr><a:xfrm><a:off x="685800" y="76200"/><a:ext cx="95250" cy="114300"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:solidFill><a:srgbClr val="EE4455"/></a:solidFill><a:ln><a:noFill/></a:ln></xdr:spPr><xdr:txBody><a:bodyPr/><a:lstStyle/><a:p/></xdr:txBody></xdr:sp></xdr:grpSp><xdr:clientData/></xdr:twoCellAnchor></xdr:wsDr>"#;

const ROW_XML: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<xdr:wsDr xmlns:xdr="http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><xdr:twoCellAnchor><xdr:from><xdr:col>0</xdr:col><xdr:colOff>76200</xdr:colOff><xdr:row>1</xdr:row><xdr:rowOff>76200</xdr:rowOff></xdr:from><xdr:to><xdr:col>0</xdr:col><xdr:colOff>190500</xdr:colOff><xdr:row>2</xdr:row><xdr:rowOff>171450</xdr:rowOff></xdr:to><xdr:grpSp><xdr:nvGrpSpPr><xdr:cNvPr id="7" name="组合"/><xdr:cNvGrpSpPr/></xdr:nvGrpSpPr><xdr:grpSpPr><a:xfrm><a:off x="76200" y="457200"/><a:ext cx="114300" cy="476250"/><a:chOff x="76200" y="457200"/><a:chExt cx="114300" cy="476250"/></a:xfrm></xdr:grpSpPr><xdr:sp><xdr:nvSpPr><xdr:cNvPr id="4" name="形状 3"/><xdr:cNvSpPr/></xdr:nvSpPr><xdr:spPr><a:xfrm><a:off x="76200" y="457200"/><a:ext cx="114300" cy="95250"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:solidFill><a:srgbClr val="EE4455"/></a:solidFill><a:ln><a:noFill/></a:ln></xdr:spPr><xdr:txBody><a:bodyPr/><a:lstStyle/><a:p/></xdr:txBody></xdr:sp><xdr:sp><xdr:nvSpPr><xdr:cNvPr id="5" name="形状 4"/><xdr:cNvSpPr/></xdr:nvSpPr><xdr:spPr><a:xfrm><a:off x="76200" y="838200"/><a:ext cx="114300" cy="95250"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:solidFill><a:srgbClr val="EE4455"/></a:solidFill><a:ln><a:noFill/></a:ln></xdr:spPr><xdr:txBody><a:bodyPr/><a:lstStyle/><a:p/></xdr:txBody></xdr:sp></xdr:grpSp><xdr:clientData/></xdr:twoCellAnchor></xdr:wsDr>"#;

const GROUP_XML: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<xdr:wsDr xmlns:xdr="http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><xdr:twoCellAnchor><xdr:from><xdr:col>0</xdr:col><xdr:colOff>9525</xdr:colOff><xdr:row>0</xdr:row><xdr:rowOff>9525</xdr:rowOff></xdr:from><xdr:to><xdr:col>0</xdr:col><xdr:colOff>247650</xdr:colOff><xdr:row>0</xdr:row><xdr:rowOff>85725</xdr:rowOff></xdr:to><xdr:grpSp><xdr:nvGrpSpPr><xdr:cNvPr id="4" name="组合 3"/><xdr:cNvGrpSpPr/></xdr:nvGrpSpPr><xdr:grpSpPr><a:xfrm><a:off x="9525" y="9525"/><a:ext cx="238125" cy="76200"/><a:chOff x="9525" y="9525"/><a:chExt cx="238125" cy="76200"/></a:xfrm></xdr:grpSpPr><xdr:sp><xdr:nvSpPr><xdr:cNvPr id="2" name="形状 1"/><xdr:cNvSpPr/></xdr:nvSpPr><xdr:spPr><a:xfrm><a:off x="9525" y="9525"/><a:ext cx="76200" cy="76200"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:solidFill><a:srgbClr val="EE4455"/></a:solidFill><a:ln><a:noFill/></a:ln></xdr:spPr><xdr:txBody><a:bodyPr/><a:lstStyle/><a:p/></xdr:txBody></xdr:sp><xdr:sp><xdr:nvSpPr><xdr:cNvPr id="3" name="形状 2"/><xdr:cNvSpPr/></xdr:nvSpPr><xdr:spPr><a:xfrm><a:off x="190500" y="9525"/><a:ext cx="57150" cy="38100"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:solidFill><a:srgbClr val="EE4455"/></a:solidFill><a:ln><a:noFill/></a:ln></xdr:spPr><xdr:txBody><a:bodyPr/><a:lstStyle/><a:p/></xdr:txBody></xdr:sp></xdr:grpSp><xdr:clientData/></xdr:twoCellAnchor></xdr:wsDr>"#;
