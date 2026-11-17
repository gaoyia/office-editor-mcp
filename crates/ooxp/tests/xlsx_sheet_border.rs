//! 虚线、双线和点划线按整张图的坐标取余。偶数宽度多出的一像素朝坐标较小的一侧。

use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

static SEQ: AtomicU32 = AtomicU32::new(0);

const RED: [u8; 3] = [0xEE, 0x44, 0x55];
const BLUE: [u8; 3] = [0x00, 0x00, 0xFF];

fn tmp(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-xlsx-sheet-border");
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
    let RunOutput::Written { path, .. } = written else {
        panic!("应写出文件");
    };
    Ok(std::fs::read(path).unwrap())
}

fn rewrite_part(src: &Path, name: &str, edit: impl FnOnce(String) -> String) -> PathBuf {
    let dst = tmp("rewrite");
    let bytes = std::fs::read(src).unwrap();
    let mut zip_in = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut out = Cursor::new(Vec::new());
    let mut edit = Some(edit);
    {
        let mut zip_out = ZipWriter::new(&mut out);
        let opt = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        for i in 0..zip_in.len() {
            let mut file = zip_in.by_index(i).unwrap();
            let entry = file.name().to_string();
            let mut data = Vec::new();
            file.read_to_end(&mut data).unwrap();
            if entry == name {
                let xml = String::from_utf8(data).unwrap();
                data = edit.take().unwrap()(xml).into_bytes();
            }
            zip_out.start_file(&entry, opt).unwrap();
            zip_out.write_all(&data).unwrap();
        }
        zip_out.finish().unwrap();
    }
    std::fs::write(&dst, out.into_inner()).unwrap();
    dst
}

fn bordered() -> PathBuf {
    let src = tmp("src");
    let mut cells = Vec::new();
    for col in ["A", "B", "C"] {
        for row in 1..=2 {
            cells.push(json!({ "ref": format!("{col}{row}"), "value": "" }));
        }
    }
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": src.to_string_lossy(),
        "workbook": { "sheets": [{ "name": "Sheet1", "cells": cells }] }
    }));
    let out = tmp("borders");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": out.to_string_lossy(),
        "ops": [
            { "op": "setColWidth", "id": "sheet:Sheet1", "col": "B", "width": 10 },
            { "op": "setRowHeight", "id": "sheet:Sheet1", "row": 1, "height": 15 },
            { "op": "setRowHeight", "id": "sheet:Sheet1", "row": 2, "height": 15 },
            {
                "op": "setCellStyle", "id": "cell:Sheet1!A1",
                "border": { "bottom": { "style": "double", "color": { "rgb": "FFEE4455" } } }
            },
            {
                "op": "setCellStyle", "id": "cell:Sheet1!C1",
                "border": { "bottom": { "style": "medium", "color": { "rgb": "FF0000FF" } } }
            },
            {
                "op": "setCellStyle", "id": "cell:Sheet1!A2",
                "border": { "bottom": { "style": "dashed", "color": { "rgb": "FFEE4455" } } }
            },
            {
                "op": "setCellStyle", "id": "cell:Sheet1!C2",
                "border": { "bottom": { "style": "slantDashDot", "color": { "rgb": "FFEE4455" } } }
            },
            {
                "op": "setCellStyle", "id": "cell:Sheet1!B2",
                "border": { "left": { "style": "dotted", "color": { "rgb": "FFEE4455" } } }
            }
        ]
    }));
    rewrite_part(&out, "xl/worksheets/sheet1.xml", |xml| {
        let views =
            "<sheetViews><sheetView showGridLines=\"0\" workbookViewId=\"0\"/></sheetViews>";
        let at = xml.find("<dimension ").expect("dimension");
        let end = xml[at..].find("/>").expect("dimension end") + at + 2;
        let xml = format!("{}{}{}", &xml[..end], views, &xml[end..]);
        assert!(xml.find("sheetViews").unwrap() < xml.find("<cols>").unwrap());
        xml
    })
}

fn dash_on(style: &str, along: i32, across: i32, boundary: i32) -> bool {
    let r4 = along.rem_euclid(4);
    let r18 = along.rem_euclid(18);
    match style {
        "solid" => true,
        "double" => across != boundary,
        "dashed" => r4 != 3,
        "dotted" => r4 == 0 || r4 == 3,
        "slantDashDot" => {
            if across == boundary {
                (0..=9).contains(&r18) || (12..=15).contains(&r18)
            } else {
                (1..=11).contains(&r18) || (13..=17).contains(&r18)
            }
        }
        _ => panic!("未知线型 {style}"),
    }
}

struct Expect {
    px: i32,
    order: u32,
    style: &'static str,
    vertical: bool,
    boundary: i32,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    color: [u8; 3],
}

struct LineSpec {
    style: &'static str,
    px: i32,
    vertical: bool,
    boundary: i32,
    along0: i32,
    along1: i32,
    color: [u8; 3],
}

fn push_expect(out: &mut Vec<Expect>, order: &mut u32, spec: LineSpec) {
    let neg = spec.px / 2;
    let pos = spec.px - neg - 1;
    let across0 = spec.boundary - neg;
    let across1 = spec.boundary + pos + 1;
    let (x0, y0, x1, y1) = if spec.vertical {
        (across0, spec.along0, across1, spec.along1 + 1)
    } else {
        (spec.along0, across0, spec.along1 + 1, across1)
    };
    out.push(Expect {
        px: spec.px,
        order: *order,
        style: spec.style,
        vertical: spec.vertical,
        boundary: spec.boundary,
        x0,
        y0,
        x1,
        y1,
        color: spec.color,
    });
    *order += 1;
}

#[test]
fn dashes_and_double_follow_the_picture_pixel() {
    let (w, h, px) = decode_png(&export(&bordered()).unwrap());
    assert_eq!((w, h), (204, 41));

    let mut strokes = Vec::new();
    let mut order = 0u32;
    // 没有边的格子不占顺序。A1 底、C1 底、A2 底、B2 左、C2 底。
    push_expect(
        &mut strokes,
        &mut order,
        LineSpec {
            style: "double",
            px: 3,
            vertical: false,
            boundary: 20,
            along0: 0,
            along1: 64,
            color: RED,
        },
    );
    push_expect(
        &mut strokes,
        &mut order,
        LineSpec {
            style: "solid",
            px: 2,
            vertical: false,
            boundary: 20,
            along0: 139,
            along1: 203,
            color: BLUE,
        },
    );
    push_expect(
        &mut strokes,
        &mut order,
        LineSpec {
            style: "dashed",
            px: 1,
            vertical: false,
            boundary: 40,
            along0: 0,
            along1: 64,
            color: RED,
        },
    );
    // B2 没有上边，点线从这一行的上边界涂起。
    push_expect(
        &mut strokes,
        &mut order,
        LineSpec {
            style: "dotted",
            px: 1,
            vertical: true,
            boundary: 64,
            along0: 20,
            along1: 40,
            color: RED,
        },
    );
    push_expect(
        &mut strokes,
        &mut order,
        LineSpec {
            style: "slantDashDot",
            px: 2,
            vertical: false,
            boundary: 40,
            along0: 139,
            along1: 203,
            color: RED,
        },
    );
    strokes.sort_by_key(|stroke| (stroke.px, stroke.order));

    let mut expect = vec![[255u8, 255, 255]; (w * h) as usize];
    for stroke in &strokes {
        for y in stroke.y0..stroke.y1 {
            for x in stroke.x0..stroke.x1 {
                if x < 0 || y < 0 || x >= w as i32 || y >= h as i32 {
                    continue;
                }
                let (along, across) = if stroke.vertical { (y, x) } else { (x, y) };
                if dash_on(stroke.style, along, across, stroke.boundary) {
                    let i = (y as u32 * w + x as u32) as usize;
                    expect[i] = stroke.color;
                }
            }
        }
    }
    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) as usize;
            assert_eq!(
                pixel(&px, w, x, y),
                expect[i],
                "像素 {x},{y} 期望 {:02X}{:02X}{:02X}",
                expect[i][0],
                expect[i][1],
                expect[i][2]
            );
        }
    }
}

#[test]
fn diagonal_flag_without_style_still_not_implemented() {
    let src = bordered();
    let diagonal = rewrite_part(&src, "xl/styles.xml", |xml| {
        xml.replacen("<border>", "<border diagonalUp=\"1\">", 1)
    });
    assert_eq!(
        export(&diagonal).unwrap_err().code(),
        ErrorCode::NotImplemented
    );
}

/// ddc5a85d 起两停点不透明 linear 可画；本测试原期待 NI，属过期合同。
#[test]
fn linear_two_stop_gradient_fill_exports_png() {
    let src = bordered();
    let gradient = rewrite_part(&src, "xl/styles.xml", |xml| {
        xml.replacen(
            "<patternFill patternType=\"none\"/>",
            "<gradientFill degree=\"0\"><stop position=\"0\"><color rgb=\"FFEE4455\"/></stop><stop position=\"1\"><color rgb=\"FF112233\"/></stop></gradientFill>",
            1,
        )
    });
    let bytes = export(&gradient).unwrap();
    assert_eq!(&bytes[..8], &[137, 80, 78, 71, 13, 10, 26, 10]);
}

/// `type=path` 等非本刀子集仍整表 NI。
#[test]
fn path_gradient_fill_still_not_implemented() {
    let src = bordered();
    let gradient = rewrite_part(&src, "xl/styles.xml", |xml| {
        xml.replacen(
            "<patternFill patternType=\"none\"/>",
            "<gradientFill type=\"path\"><stop position=\"0\"><color rgb=\"FFEE4455\"/></stop><stop position=\"1\"><color rgb=\"FF112233\"/></stop></gradientFill>",
            1,
        )
    });
    assert_eq!(
        export(&gradient).unwrap_err().code(),
        ErrorCode::NotImplemented
    );
}

#[test]
fn solid_diagonal_thin_and_medium_export_png() {
    let src = tmp("diag-src");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": src.to_string_lossy(),
        "workbook": { "sheets": [{ "name": "Sheet1", "cells": [
            { "ref": "A1", "value": "" },
            { "ref": "B1", "value": "" },
            { "ref": "A2", "value": "" },
            { "ref": "B2", "value": "" }
        ] }] }
    }));
    let book = tmp("diag");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": book.to_string_lossy(),
        "ops": [
            {
                "op": "setCellStyle",
                "id": "cell:Sheet1!A1",
                "border": {
                    "diagonal": { "style": "thin", "color": { "rgb": "FFEE4455" } },
                    "diagonalDown": true
                }
            },
            {
                "op": "setCellStyle",
                "id": "cell:Sheet1!B1",
                "border": {
                    "diagonal": { "style": "thin", "color": { "rgb": "FF0000FF" } },
                    "diagonalUp": true
                }
            },
            {
                "op": "setCellStyle",
                "id": "cell:Sheet1!A2",
                "border": {
                    "diagonal": { "style": "medium", "color": { "rgb": "FF112233" } },
                    "diagonalDown": true,
                    "diagonalUp": true
                }
            },
            { "op": "setColWidth", "id": "sheet:Sheet1", "col": "A", "width": 12.0 },
            { "op": "setColWidth", "id": "sheet:Sheet1", "col": "B", "width": 8.0 },
            { "op": "setRowHeight", "id": "sheet:Sheet1", "row": 1, "height": 30.0 },
            { "op": "setRowHeight", "id": "sheet:Sheet1", "row": 2, "height": 20.0 }
        ]
    }));
    let png = export(&book).unwrap();
    assert!(png.starts_with(&[0x89, b'P', b'N', b'G']));
    let (w, h, px) = decode_png(&png);
    assert!(w > 10 && h > 10, "{w}x{h}");
    let ink = px.chunks(3).filter(|c| *c != [255, 255, 255]).count();
    assert!(ink > 10, "ink={ink}");
}

#[test]
fn dashed_diagonal_still_not_implemented() {
    let src = tmp("diag-dash-src");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": src.to_string_lossy(),
        "workbook": { "sheets": [{ "name": "Sheet1", "cells": [
            { "ref": "A1", "value": "" }
        ] }] }
    }));
    let book = tmp("diag-dash");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": book.to_string_lossy(),
        "ops": [{
            "op": "setCellStyle",
            "id": "cell:Sheet1!A1",
            "border": {
                "diagonal": { "style": "dashed", "color": { "rgb": "FFEE4455" } },
                "diagonalDown": true
            }
        }]
    }));
    assert_eq!(export(&book).unwrap_err().code(), ErrorCode::NotImplemented);
}

/// 第二行只有细右边框时，竖线含上边界。同一行另有细上边时，上边界留给上边。
#[test]
fn vertical_line_paints_the_top_boundary_without_a_top_edge() {
    let src = tmp("cap-src");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": src.to_string_lossy(),
        "workbook": { "sheets": [{ "name": "Sheet1", "cells": [
            { "ref": "A1", "value": "" },
            { "ref": "A2", "value": "" },
            { "ref": "C1", "value": "" },
            { "ref": "C2", "value": "" }
        ] }] }
    }));
    let book = tmp("cap");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": book.to_string_lossy(),
        "ops": [
            { "op": "setRowHeight", "id": "sheet:Sheet1", "row": 1, "height": 15 },
            { "op": "setRowHeight", "id": "sheet:Sheet1", "row": 2, "height": 15 },
            {
                "op": "setCellStyle", "id": "cell:Sheet1!A2",
                "border": { "right": { "style": "thin", "color": { "rgb": "FF000000" } } }
            },
            {
                "op": "setCellStyle", "id": "cell:Sheet1!C2",
                "border": {
                    "top": { "style": "thin", "color": { "rgb": "FFEE4455" } },
                    "right": { "style": "thin", "color": { "rgb": "FF0000FF" } }
                }
            }
        ]
    }));
    let book = rewrite_part(&book, "xl/worksheets/sheet1.xml", |xml| {
        let views =
            "<sheetViews><sheetView showGridLines=\"0\" workbookViewId=\"0\"/></sheetViews>";
        let at = xml.find("<dimension ").expect("dimension");
        let end = xml[at..].find("/>").expect("dimension end") + at + 2;
        format!("{}{}{}", &xml[..end], views, &xml[end..])
    });
    let (w, h, px) = decode_png(&export(&book).unwrap());
    assert_eq!((w, h), (193, 41));
    assert_eq!(pixel(&px, w, 64, 19), [255, 255, 255]);
    assert_eq!(pixel(&px, w, 64, 20), [0, 0, 0]);
    assert_eq!(pixel(&px, w, 64, 40), [0, 0, 0]);
    assert_eq!(pixel(&px, w, 64, 21), [0, 0, 0]);
    // C 列右缘在 x=192。有上边时，y=20 是红上边，蓝竖线从 y=21 开始。
    assert_eq!(pixel(&px, w, 192, 20), RED);
    assert_eq!(pixel(&px, w, 192, 21), BLUE);
    assert_eq!(pixel(&px, w, 191, 20), RED);
}

/// 上面那一格有底边时，双线的内侧也不涂上边界。底边让开后露出的那一像素保持空白。
#[test]
fn vertical_line_yields_its_top_when_the_cell_above_has_a_bottom() {
    let src = tmp("yield-src");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": src.to_string_lossy(),
        "workbook": { "sheets": [{ "name": "Sheet1", "cells": [
            { "ref": "A1", "value": "" },
            { "ref": "B1", "value": "" },
            { "ref": "B2", "value": "" }
        ] }] }
    }));
    let book = tmp("yield");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": book.to_string_lossy(),
        "ops": [
            { "op": "setRowHeight", "id": "sheet:Sheet1", "row": 1, "height": 15 },
            { "op": "setRowHeight", "id": "sheet:Sheet1", "row": 2, "height": 15 },
            {
                "op": "setCellStyle", "id": "cell:Sheet1!A1",
                "border": { "bottom": { "style": "mediumDashDot", "color": { "rgb": "FFEE4455" } } }
            },
            {
                "op": "setCellStyle", "id": "cell:Sheet1!B1",
                "border": { "bottom": { "style": "dashDotDot", "color": { "rgb": "FFEE4455" } } }
            },
            {
                "op": "setCellStyle", "id": "cell:Sheet1!B2",
                "border": { "left": { "style": "double", "color": { "rgb": "FF0000FF" } } }
            }
        ]
    }));
    let book = rewrite_part(&book, "xl/worksheets/sheet1.xml", |xml| {
        let views =
            "<sheetViews><sheetView showGridLines=\"0\" workbookViewId=\"0\"/></sheetViews>";
        let at = xml.find("<dimension ").expect("dimension");
        let end = xml[at..].find("/>").expect("dimension end") + at + 2;
        format!("{}{}{}", &xml[..end], views, &xml[end..])
    });
    let (w, h, px) = decode_png(&export(&book).unwrap());
    assert_eq!((w, h), (129, 41));
    // 双线中线在 x=64。内侧 x=65 的上边界被上面的底边占住，底边本身从 x=66 才开始。
    assert_eq!(pixel(&px, w, 65, 20), [255, 255, 255]);
    assert_eq!(pixel(&px, w, 65, 21), BLUE);
    assert_eq!(pixel(&px, w, 63, 21), BLUE);
    assert_eq!(pixel(&px, w, 64, 21), [255, 255, 255]);
}

fn formula_right(formula: &str) -> PathBuf {
    let src = tmp("cf-src");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": src.to_string_lossy(),
        "workbook": { "sheets": [{ "name": "Sheet1", "cells": [
            { "ref": "A1", "value": "" },
            { "ref": "A2", "value": "" }
        ] }] }
    }));
    let book = tmp("cf-border");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": book.to_string_lossy(),
        "ops": [
            { "op": "setRowHeight", "id": "sheet:Sheet1", "row": 1, "height": 15 },
            { "op": "setRowHeight", "id": "sheet:Sheet1", "row": 2, "height": 15 },
            {
                "op": "insertConditionalFormat", "id": "sheet:Sheet1", "sqref": "A2",
                "type": "expression", "formulas": [formula],
                "dxf": { "border": { "right": { "style": "thin", "color": { "rgb": "FF000000" } } } }
            }
        ]
    }));
    rewrite_part(&book, "xl/worksheets/sheet1.xml", |xml| {
        let views =
            "<sheetViews><sheetView showGridLines=\"0\" workbookViewId=\"0\"/></sheetViews>";
        let at = xml.find("<dimension ").expect("dimension");
        let end = xml[at..].find("/>").expect("dimension end") + at + 2;
        format!("{}{}{}", &xml[..end], views, &xml[end..])
    })
}

/// 公式差分的细右边框和格子边框同一套像素，含上边界。
#[test]
fn formula_thin_right_paints_the_top_boundary() {
    let (w, h, px) = decode_png(&export(&formula_right("TRUE")).unwrap());
    assert_eq!((w, h), (65, 41));
    assert_eq!(pixel(&px, w, 64, 19), [255, 255, 255]);
    assert_eq!(pixel(&px, w, 64, 20), [0, 0, 0]);
    assert_eq!(pixel(&px, w, 64, 40), [0, 0, 0]);
}

#[test]
fn formula_false_does_not_paint_a_border() {
    let (w, h, px) = decode_png(&export(&formula_right("FALSE")).unwrap());
    assert_eq!((w, h), (65, 41));
    assert_eq!(pixel(&px, w, 64, 20), [255, 255, 255]);
    assert_eq!(pixel(&px, w, 64, 30), [255, 255, 255]);
}

#[test]
fn formula_dotted_right_paints_like_a_cell_border() {
    let src = tmp("dot-src");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": src.to_string_lossy(),
        "workbook": { "sheets": [{ "name": "Sheet1", "cells": [
            { "ref": "A1", "value": "" },
            { "ref": "A2", "value": "" }
        ] }] }
    }));
    let book = tmp("dot");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": book.to_string_lossy(),
        "ops": [
            { "op": "setRowHeight", "id": "sheet:Sheet1", "row": 1, "height": 15 },
            { "op": "setRowHeight", "id": "sheet:Sheet1", "row": 2, "height": 15 },
            {
                "op": "insertConditionalFormat", "id": "sheet:Sheet1", "sqref": "A2",
                "type": "expression", "formulas": ["TRUE"],
                "dxf": { "border": { "right": { "style": "dotted", "color": { "rgb": "FF000000" } } } }
            }
        ]
    }));
    let book = rewrite_part(&book, "xl/worksheets/sheet1.xml", |xml| {
        let views =
            "<sheetViews><sheetView showGridLines=\"0\" workbookViewId=\"0\"/></sheetViews>";
        let at = xml.find("<dimension ").expect("dimension");
        let end = xml[at..].find("/>").expect("dimension end") + at + 2;
        format!("{}{}{}", &xml[..end], views, &xml[end..])
    });
    let (w, h, px) = decode_png(&export(&book).unwrap());
    assert_eq!((w, h), (65, 41));
    assert_eq!(pixel(&px, w, 64, 20), [0, 0, 0]);
    assert_eq!(pixel(&px, w, 64, 21), [255, 255, 255]);
    assert_eq!(pixel(&px, w, 64, 23), [0, 0, 0]);
}

#[test]
fn formula_dashed_and_slant_export_ok_cell_is_still_ni() {
    let src = tmp("dash-src");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": src.to_string_lossy(),
        "workbook": { "sheets": [{ "name": "Sheet1", "cells": [
            { "ref": "A1", "value": 1 }
        ] }] }
    }));
    let dashed = tmp("dash");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": dashed.to_string_lossy(),
        "ops": [{
            "op": "insertConditionalFormat", "id": "sheet:Sheet1", "sqref": "A1",
            "type": "expression", "formulas": ["TRUE"],
            "dxf": { "border": { "right": { "style": "dashed", "color": { "rgb": "FF000000" } } } }
        }]
    }));
    let _ = export(&dashed).expect("公式差分 dashed 应可出图");

    let slant = tmp("slant");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": slant.to_string_lossy(),
        "ops": [{
            "op": "insertConditionalFormat", "id": "sheet:Sheet1", "sqref": "A1",
            "type": "expression", "formulas": ["TRUE"],
            "dxf": { "border": { "right": { "style": "slantDashDot", "color": { "rgb": "FF000000" } } } }
        }]
    }));
    let _ = export(&slant).expect("公式差分 slantDashDot 应可出图");

    let cell_is = tmp("cellis");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": cell_is.to_string_lossy(),
        "ops": [{
            "op": "insertConditionalFormat", "id": "sheet:Sheet1", "sqref": "A1",
            "type": "cellIs", "operator": "greaterThan", "formulas": ["0"],
            "dxf": { "border": { "right": { "style": "thin", "color": { "rgb": "FF000000" } } } }
        }]
    }));
    assert_eq!(
        export(&cell_is).unwrap_err().code(),
        ErrorCode::NotImplemented
    );
}

#[test]
fn formula_dashed_right_rhythm_matches_line_on() {
    // 与格子 dashed 同一套 paint_line：竖线 along=y，相位 along-1 后 r4!=3。
    let src = tmp("fdash-src");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": src.to_string_lossy(),
        "workbook": { "sheets": [{ "name": "Sheet1", "cells": [
            { "ref": "A1", "value": "" },
            { "ref": "A2", "value": "" },
            { "ref": "A3", "value": "" },
            { "ref": "A4", "value": "" }
        ] }] }
    }));
    let book = tmp("fdash");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": book.to_string_lossy(),
        "ops": [
            { "op": "setRowHeight", "id": "sheet:Sheet1", "row": 1, "height": 15 },
            { "op": "setRowHeight", "id": "sheet:Sheet1", "row": 2, "height": 15 },
            { "op": "setRowHeight", "id": "sheet:Sheet1", "row": 3, "height": 15 },
            { "op": "setRowHeight", "id": "sheet:Sheet1", "row": 4, "height": 15 },
            {
                "op": "insertConditionalFormat", "id": "sheet:Sheet1", "sqref": "A2:A4",
                "type": "expression", "formulas": ["TRUE"],
                "dxf": { "border": { "right": { "style": "dashed", "color": { "rgb": "FF000000" } } } }
            }
        ]
    }));
    let book = rewrite_part(&book, "xl/worksheets/sheet1.xml", |xml| {
        let views =
            "<sheetViews><sheetView showGridLines=\"0\" workbookViewId=\"0\"/></sheetViews>";
        let at = xml.find("<dimension ").expect("dimension");
        let end = xml[at..].find("/>").expect("dimension end") + at + 2;
        format!("{}{}{}", &xml[..end], views, &xml[end..])
    });
    let (w, h, px) = decode_png(&export(&book).unwrap());
    let x = w - 1;
    let mut ink = Vec::new();
    for y in 0..h {
        ink.push(pixel(&px, w, x, y) == [0, 0, 0]);
    }
    let first = ink.iter().position(|&b| b).expect("dashed ink");
    for (y, &bit) in ink.iter().enumerate().take((h as usize).saturating_sub(1)).skip(first) {
        let along = y as i32 - 1;
        let expect = along.rem_euclid(4) != 3;
        assert_eq!(bit, expect, "y={y} phase");
    }
}

#[test]
fn formula_slant_bottom_two_row_stagger() {
    let src = tmp("fslant-src");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": src.to_string_lossy(),
        "workbook": { "sheets": [{ "name": "Sheet1", "cells": [
            { "ref": "A1", "value": "" }, { "ref": "B1", "value": "" }, { "ref": "C1", "value": "" },
            { "ref": "A2", "value": "" }, { "ref": "B2", "value": "" }, { "ref": "C2", "value": "" }
        ] }] }
    }));
    let book = tmp("fslant");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": book.to_string_lossy(),
        "ops": [
            { "op": "setRowHeight", "id": "sheet:Sheet1", "row": 1, "height": 15 },
            { "op": "setRowHeight", "id": "sheet:Sheet1", "row": 2, "height": 15 },
            {
                "op": "insertConditionalFormat", "id": "sheet:Sheet1", "sqref": "A1:C1",
                "type": "expression", "formulas": ["TRUE"],
                "dxf": { "border": { "bottom": { "style": "slantDashDot", "color": { "rgb": "FF000000" } } } }
            }
        ]
    }));
    let book = rewrite_part(&book, "xl/worksheets/sheet1.xml", |xml| {
        let views =
            "<sheetViews><sheetView showGridLines=\"0\" workbookViewId=\"0\"/></sheetViews>";
        let at = xml.find("<dimension ").expect("dimension");
        let end = xml[at..].find("/>").expect("dimension end") + at + 2;
        format!("{}{}{}", &xml[..end], views, &xml[end..])
    });
    let (w, h, px) = decode_png(&export(&book).unwrap());
    // 找墨水最多的两行（2px slant）
    let mut scores: Vec<(u32, u32)> = (0..h)
        .map(|y| {
            let n = (0..w).filter(|&x| pixel(&px, w, x, y) == [0, 0, 0]).count() as u32;
            (n, y)
        })
        .collect();
    scores.sort_by(|a, b| b.cmp(a));
    let y0 = scores[0].1.min(scores[1].1);
    let y1 = scores[0].1.max(scores[1].1);
    assert_eq!(y1, y0 + 1, "slant should be 2px");
    // boundary 行与邻行错位：同 x 不应整段同亮（允许部分重叠）
    let mut same = 0u32;
    let mut both = 0u32;
    for x in 0..w {
        let a = pixel(&px, w, x, y0) == [0, 0, 0];
        let b = pixel(&px, w, x, y1) == [0, 0, 0];
        if a || b {
            both += 1;
            if a && b {
                same += 1;
            }
        }
    }
    assert!(both > 20, "slant ink present");
    assert!(
        same < both,
        "two rows should stagger, same={same} both={both}"
    );
}
