//! 工作表锚点组合的非直角旋转：按组缩放/翻转后再绕中心旋转，不拿外接框铺色。
//! 不把官方 CopyPicture 对照算作通过。不涉及单元格 textRotation。

use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::json;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

static SEQ: AtomicU32 = AtomicU32::new(0);

const RED: [u8; 3] = [0xEE, 0x44, 0x55];
const EMU: i64 = 9_525;
const COL_PX: i32 = 64;
const ROW_PX: i32 = 40; // 30 磅

fn tmp(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-xlsx-sheet-group-angle");
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

fn drawing_book(drawing: &str) -> PathBuf {
    let src = tmp("src");
    let mut cells = Vec::new();
    for col in ['A', 'B', 'C'] {
        for row in 1..=8 {
            cells.push(json!({ "ref": format!("{col}{row}"), "value": "" }));
        }
    }
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": src.to_string_lossy(),
        "workbook": { "sheets": [{ "name": "Sheet1", "cells": cells }] }
    }));
    let shaped = tmp("shaped");
    let ops: Vec<_> = (1..=8)
        .map(|row| json!({ "op": "setRowHeight", "id": "sheet:Sheet1", "row": row, "height": 30 }))
        .chain(std::iter::once(json!({
            "op": "insertShape", "id": "sheet:Sheet1", "prst": "rect", "anchor": "oneCell",
            "from": "A1", "cx": 9525, "cy": 9525
        })))
        .collect();
    run_ok(json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": shaped.to_string_lossy(),
        "ops": ops
    }));
    let book = tmp("drawn");
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

fn rotate_cw(px: f64, py: f64, cx: f64, cy: f64, deg: f64) -> (f64, f64) {
    let rad = deg.to_radians();
    let (c, s) = (rad.cos(), rad.sin());
    let dx = px - cx;
    let dy = py - cy;
    (cx + dx * c - dy * s, cy + dx * s + dy * c)
}

fn rot_deg(rot: i64) -> f64 {
    (rot as f64) / 60_000.0
}

fn group_box_px(off_x: i64, off_y: i64, cx: i64, cy: i64, rot: i64) -> (i32, i32, i32, i32) {
    let deg = rot_deg(rot);
    let gcx = off_x as f64 + cx as f64 / 2.0;
    let gcy = off_y as f64 + cy as f64 / 2.0;
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for (sx, sy) in [(0, 0), (cx, 0), (cx, cy), (0, cy)] {
        let (rx, ry) = rotate_cw(
            off_x as f64 + sx as f64,
            off_y as f64 + sy as f64,
            gcx,
            gcy,
            deg,
        );
        min_x = min_x.min(rx);
        min_y = min_y.min(ry);
        max_x = max_x.max(rx);
        max_y = max_y.max(ry);
    }
    (
        (min_x / EMU as f64).round() as i32,
        (min_y / EMU as f64).round() as i32,
        (max_x / EMU as f64).round() as i32,
        (max_y / EMU as f64).round() as i32,
    )
}

fn cell_part(px: i32, span: i32) -> (u32, i64) {
    assert!(px >= 0);
    ((px / span) as u32, i64::from(px % span) * EMU)
}

fn two_cell(x0: i32, y0: i32, x1: i32, y1: i32) -> String {
    let (c0, ox0) = cell_part(x0, COL_PX);
    let (r0, oy0) = cell_part(y0, ROW_PX);
    let (c1, ox1) = cell_part(x1, COL_PX);
    let (r1, oy1) = cell_part(y1, ROW_PX);
    format!(
        "<xdr:from><xdr:col>{c0}</xdr:col><xdr:colOff>{ox0}</xdr:colOff>\
         <xdr:row>{r0}</xdr:row><xdr:rowOff>{oy0}</xdr:rowOff></xdr:from>\
         <xdr:to><xdr:col>{c1}</xdr:col><xdr:colOff>{ox1}</xdr:colOff>\
         <xdr:row>{r1}</xdr:row><xdr:rowOff>{oy1}</xdr:rowOff></xdr:to>"
    )
}

fn xfrm_attrs(rot: i64, flip_h: bool, flip_v: bool) -> String {
    let mut s = String::new();
    if rot != 0 {
        s.push_str(&format!(" rot=\"{rot}\""));
    }
    if flip_h {
        s.push_str(" flipH=\"1\"");
    }
    if flip_v {
        s.push_str(" flipV=\"1\"");
    }
    s
}

fn rect_sp(id: u32, x: i64, y: i64, cx: i64, cy: i64, rot: i64) -> String {
    format!(
        "<xdr:sp><xdr:nvSpPr><xdr:cNvPr id=\"{id}\" name=\"形状 {id}\"/><xdr:cNvSpPr/></xdr:nvSpPr>\
         <xdr:spPr><a:xfrm{attrs}><a:off x=\"{x}\" y=\"{y}\"/><a:ext cx=\"{cx}\" cy=\"{cy}\"/></a:xfrm>\
         <a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom>\
         <a:solidFill><a:srgbClr val=\"EE4455\"/></a:solidFill><a:ln><a:noFill/></a:ln></xdr:spPr>\
         <xdr:txBody><a:bodyPr/><a:lstStyle/><a:p/></xdr:txBody></xdr:sp>",
        attrs = xfrm_attrs(rot, false, false)
    )
}

#[allow(clippy::too_many_arguments)]
fn group_drawing(
    off_x: i64,
    off_y: i64,
    cx: i64,
    cy: i64,
    ch_x: i64,
    ch_y: i64,
    ch_cx: i64,
    ch_cy: i64,
    rot: i64,
    flip_h: bool,
    flip_v: bool,
    body: &str,
) -> String {
    let (x0, y0, x1, y1) = group_box_px(off_x, off_y, cx, cy, rot);
    let anchor = two_cell(x0, y0, x1, y1);
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<xdr:wsDr xmlns:xdr="http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
<xdr:twoCellAnchor>{anchor}<xdr:grpSp>
<xdr:nvGrpSpPr><xdr:cNvPr id="20" name="组合"/><xdr:cNvGrpSpPr/></xdr:nvGrpSpPr>
<xdr:grpSpPr><a:xfrm{attrs}><a:off x="{off_x}" y="{off_y}"/><a:ext cx="{cx}" cy="{cy}"/>
<a:chOff x="{ch_x}" y="{ch_y}"/><a:chExt cx="{ch_cx}" cy="{ch_cy}"/></a:xfrm></xdr:grpSpPr>
{body}
</xdr:grpSp><xdr:clientData/></xdr:twoCellAnchor></xdr:wsDr>"#,
        attrs = xfrm_attrs(rot, flip_h, flip_v)
    )
}

#[allow(clippy::too_many_arguments)]
fn child_sheet_box(
    off_x: i64,
    off_y: i64,
    cx: i64,
    cy: i64,
    ch_x: i64,
    ch_y: i64,
    ch_cx: i64,
    ch_cy: i64,
    rot: i64,
    flip_h: bool,
    flip_v: bool,
    child: (i64, i64, i64, i64),
) -> (f64, f64, f64, f64, f64) {
    let (x, y, w, h) = child;
    let sx0 = (x - ch_x) * cx / ch_cx;
    let sy0 = (y - ch_y) * cy / ch_cy;
    let sx1 = (x + w - ch_x) * cx / ch_cx;
    let sy1 = (y + h - ch_y) * cy / ch_cy;
    let mut fsx0 = sx0;
    let mut fsy0 = sy0;
    let mut fsx1 = sx1;
    let mut fsy1 = sy1;
    if flip_h {
        let a = cx - fsx0;
        let b = cx - fsx1;
        fsx0 = a.min(b);
        fsx1 = a.max(b);
    }
    if flip_v {
        let a = cy - fsy0;
        let b = cy - fsy1;
        fsy0 = a.min(b);
        fsy1 = a.max(b);
    }
    let x0 = (off_x + fsx0) as f64 / EMU as f64;
    let y0 = (off_y + fsy0) as f64 / EMU as f64;
    let pw = (fsx1 - fsx0) as f64 / EMU as f64;
    let ph = (fsy1 - fsy0) as f64 / EMU as f64;
    let gcx = (off_x as f64 + cx as f64 / 2.0) / EMU as f64;
    let gcy = (off_y as f64 + cy as f64 / 2.0) / EMU as f64;
    let deg = rot_deg(rot);
    let (ncx, ncy) = rotate_cw(x0 + pw / 2.0, y0 + ph / 2.0, gcx, gcy, deg);
    (ncx - pw / 2.0, ncy - ph / 2.0, pw, ph, deg)
}

#[allow(clippy::too_many_arguments)]
fn inside(px: f64, py: f64, x: f64, y: f64, w: f64, h: f64, deg: f64, margin: f64) -> bool {
    let (ux, uy) = rotate_cw(px, py, x + w / 2.0, y + h / 2.0, -deg);
    ux >= x + margin && ux <= x + w - margin && uy >= y + margin && uy <= y + h - margin
}

#[allow(clippy::too_many_arguments)]
fn clearly_outside(
    px: f64,
    py: f64,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    deg: f64,
    margin: f64,
) -> bool {
    let (ux, uy) = rotate_cw(px, py, x + w / 2.0, y + h / 2.0, -deg);
    ux < x - margin || ux > x + w + margin || uy < y - margin || uy > y + h + margin
}

fn aabb(x: f64, y: f64, w: f64, h: f64, deg: f64) -> (f64, f64, f64, f64) {
    let cx = x + w / 2.0;
    let cy = y + h / 2.0;
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for (px, py) in [(x, y), (x + w, y), (x + w, y + h), (x, y + h)] {
        let (nx, ny) = rotate_cw(px, py, cx, cy, deg);
        min_x = min_x.min(nx);
        min_y = min_y.min(ny);
        max_x = max_x.max(nx);
        max_y = max_y.max(ny);
    }
    (min_x, min_y, max_x, max_y)
}

fn assert_geometry(rgb: &[u8], width: u32, height: u32, boxes: &[(f64, f64, f64, f64, f64)]) {
    assert!(
        rgb.chunks(3).any(|p| p == RED),
        "expected some red ink from the rotated group"
    );
    for y in 1..height {
        for x in 1..width {
            let px = f64::from(x) + 0.5;
            let py = f64::from(y) + 0.5;
            let color = pixel(rgb, width, x, y);
            if boxes
                .iter()
                .any(|&(bx, by, bw, bh, deg)| inside(px, py, bx, by, bw, bh, deg, 1.5))
            {
                assert_eq!(color, RED, "interior ({x},{y})");
            } else if boxes
                .iter()
                .all(|&(bx, by, bw, bh, deg)| clearly_outside(px, py, bx, by, bw, bh, deg, 2.0))
            {
                assert_ne!(color, RED, "exterior ({x},{y}) should not be a bbox fill");
            }
        }
    }
    // 外接框四角附近、但落在旋转矩形之外的像素不得被当成整块 AABB 填红。
    for &(bx, by, bw, bh, deg) in boxes {
        let (min_x, min_y, max_x, max_y) = aabb(bx, by, bw, bh, deg);
        let corners = [
            (min_x + 0.6, min_y + 0.6),
            (max_x - 0.6, min_y + 0.6),
            (max_x - 0.6, max_y - 0.6),
            (min_x + 0.6, max_y - 0.6),
        ];
        for (cx, cy) in corners {
            if cx < 1.0 || cy < 1.0 {
                continue;
            }
            if !clearly_outside(cx, cy, bx, by, bw, bh, deg, 1.2) {
                continue;
            }
            let x = cx.floor() as u32;
            let y = cy.floor() as u32;
            if x >= width || y >= height {
                continue;
            }
            assert_ne!(
                pixel(rgb, width, x, y),
                RED,
                "AABB corner ({x},{y}) is outside the rotated rect"
            );
        }
    }
}

const OFF: i64 = 95_250; // 10px
const WIDE: i64 = 228_600; // 24px
const TALL: i64 = 114_300; // 12px
const DEG15: i64 = 900_000;
const DEG30: i64 = 1_800_000;

fn full_child_group(rot: i64, flip_h: bool, flip_v: bool) -> (String, (f64, f64, f64, f64, f64)) {
    let xml = group_drawing(
        OFF,
        OFF,
        WIDE,
        TALL,
        OFF,
        OFF,
        WIDE,
        TALL,
        rot,
        flip_h,
        flip_v,
        &rect_sp(2, OFF, OFF, WIDE, TALL, 0),
    );
    let box_ = child_sheet_box(
        OFF,
        OFF,
        WIDE,
        TALL,
        OFF,
        OFF,
        WIDE,
        TALL,
        rot,
        flip_h,
        flip_v,
        (OFF, OFF, WIDE, TALL),
    );
    (xml, box_)
}

#[cfg(windows)]
#[test]
fn a_15_degree_group_paints_the_rotated_rectangle_not_its_bbox() {
    let (xml, box_) = full_child_group(DEG15, false, false);
    let book = drawing_book(&xml);
    let before = std::fs::read(&book).unwrap();
    let (w, h, rgb) = decode_png(&export(&book).unwrap());
    assert_eq!(
        std::fs::read(&book).unwrap(),
        before,
        "export must not rewrite the package"
    );
    assert!(w >= 65 && h >= 41, "{w}x{h}");
    assert_geometry(&rgb, w, h, &[box_]);
}

#[cfg(windows)]
#[test]
fn fifteen_and_thirty_degrees_are_not_the_same_as_zero() {
    let (xml0, _) = full_child_group(0, false, false);
    let (xml15, _) = full_child_group(DEG15, false, false);
    let (xml30, _) = full_child_group(DEG30, false, false);
    let png0 = export(&drawing_book(&xml0)).unwrap();
    let png15 = export(&drawing_book(&xml15)).unwrap();
    let png30 = export(&drawing_book(&xml30)).unwrap();
    assert_ne!(png15, png0, "15° must not match the unrotated group");
    assert_ne!(png30, png0, "30° must not match the unrotated group");
    assert_ne!(png15, png30, "15° and 30° must differ");
}

#[cfg(windows)]
#[test]
fn a_flipped_15_degree_group_moves_the_child_before_rotating() {
    // 子矩形只占左半，水平翻转后应到右半，再转 15°。
    let child = (OFF, OFF, WIDE / 2, TALL);
    let xml = group_drawing(
        OFF,
        OFF,
        WIDE,
        TALL,
        OFF,
        OFF,
        WIDE,
        TALL,
        DEG15,
        true,
        false,
        &rect_sp(2, child.0, child.1, child.2, child.3, 0),
    );
    let box_ = child_sheet_box(
        OFF, OFF, WIDE, TALL, OFF, OFF, WIDE, TALL, DEG15, true, false, child,
    );
    let unflipped = child_sheet_box(
        OFF, OFF, WIDE, TALL, OFF, OFF, WIDE, TALL, DEG15, false, false, child,
    );
    let (w, h, rgb) = decode_png(&export(&drawing_book(&xml)).unwrap());
    assert_geometry(&rgb, w, h, &[box_]);
    assert_ne!(
        (box_.0, box_.1),
        (unflipped.0, unflipped.1),
        "flip must move the child before rotation"
    );
}

#[cfg(windows)]
#[test]
fn a_non_uniform_scale_then_30_degree_turn_keeps_a_rectangle() {
    // 子坐标 12×12 的正方形，外框宽乘 2，再转 30°。
    let ch = 114_300;
    let child = (OFF, OFF, ch, ch);
    let xml = group_drawing(
        OFF,
        OFF,
        WIDE,
        TALL,
        OFF,
        OFF,
        ch,
        ch,
        DEG30,
        false,
        false,
        &rect_sp(2, child.0, child.1, child.2, child.3, 0),
    );
    let box_ = child_sheet_box(
        OFF, OFF, WIDE, TALL, OFF, OFF, ch, ch, DEG30, false, false, child,
    );
    assert!((box_.2 - 24.0).abs() < 1e-6, "scaled width {}", box_.2);
    assert!((box_.3 - 12.0).abs() < 1e-6, "scaled height {}", box_.3);
    let (w, h, rgb) = decode_png(&export(&drawing_book(&xml)).unwrap());
    assert_geometry(&rgb, w, h, &[box_]);
}

#[cfg(windows)]
#[test]
fn an_inner_quarter_turn_then_outer_15_degrees_composes() {
    let inner_off_x = OFF + 76_200;
    let inner_off_y = OFF - 19_050;
    let inner_cx = 76_200; // 8px
    let inner_cy = 152_400; // 16px
                            // 里层转 90° 后外接框仍落在外层子坐标里：8×16 → 16×8，中心不变。
    let inner = format!(
        "<xdr:grpSp><xdr:nvGrpSpPr><xdr:cNvPr id=\"8\" name=\"里层\"/><xdr:cNvGrpSpPr/></xdr:nvGrpSpPr>\
         <xdr:grpSpPr><a:xfrm rot=\"5400000\"><a:off x=\"{inner_off_x}\" y=\"{inner_off_y}\"/>\
         <a:ext cx=\"{inner_cx}\" cy=\"{inner_cy}\"/>\
         <a:chOff x=\"{inner_off_x}\" y=\"{inner_off_y}\"/>\
         <a:chExt cx=\"{inner_cx}\" cy=\"{inner_cy}\"/></a:xfrm></xdr:grpSpPr>\
         {rect}</xdr:grpSp>",
        rect = rect_sp(2, inner_off_x, inner_off_y, inner_cx, inner_cy, 0)
    );
    let xml = group_drawing(
        OFF, OFF, WIDE, TALL, OFF, OFF, WIDE, TALL, DEG15, false, false, &inner,
    );
    // 里层 90° 后的轴对齐框：宽高对调，中心不变。
    let child_after = (
        inner_off_x + (inner_cx - inner_cy) / 2,
        inner_off_y + (inner_cy - inner_cx) / 2,
        inner_cy,
        inner_cx,
    );
    let box_ = child_sheet_box(
        OFF,
        OFF,
        WIDE,
        TALL,
        OFF,
        OFF,
        WIDE,
        TALL,
        DEG15,
        false,
        false,
        child_after,
    );
    let (w, h, rgb) = decode_png(&export(&drawing_book(&xml)).unwrap());
    assert_geometry(&rgb, w, h, &[box_]);
}

fn export_err(drawing: &str) -> ErrorCode {
    let book = drawing_book(drawing);
    export(&book).unwrap_err().code()
}

#[test]
fn an_inner_non_right_angle_stays_not_implemented() {
    let inner = format!(
        "<xdr:grpSp><xdr:nvGrpSpPr><xdr:cNvPr id=\"8\" name=\"里层\"/><xdr:cNvGrpSpPr/></xdr:nvGrpSpPr>\
         <xdr:grpSpPr><a:xfrm rot=\"{DEG15}\"><a:off x=\"{OFF}\" y=\"{OFF}\"/>\
         <a:ext cx=\"{TALL}\" cy=\"{TALL}\"/>\
         <a:chOff x=\"{OFF}\" y=\"{OFF}\"/>\
         <a:chExt cx=\"{TALL}\" cy=\"{TALL}\"/></a:xfrm></xdr:grpSpPr>\
         {rect}</xdr:grpSp>",
        rect = rect_sp(2, OFF, OFF, TALL, TALL, 0)
    );
    let xml = group_drawing(
        OFF, OFF, WIDE, TALL, OFF, OFF, WIDE, TALL, DEG15, false, false, &inner,
    );
    assert_eq!(export_err(&xml), ErrorCode::NotImplemented);
}

#[test]
fn a_child_non_right_angle_in_an_angled_group_stays_not_implemented() {
    let xml = group_drawing(
        OFF,
        OFF,
        WIDE,
        TALL,
        OFF,
        OFF,
        WIDE,
        TALL,
        DEG15,
        false,
        false,
        &rect_sp(2, OFF, OFF, WIDE, TALL, 100),
    );
    assert_eq!(export_err(&xml), ErrorCode::NotImplemented);
}

#[test]
fn a_stroked_child_in_an_angled_group_stays_not_implemented() {
    let body = rect_sp(2, OFF, OFF, WIDE, TALL, 0).replace(
        "<a:ln><a:noFill/></a:ln>",
        "<a:ln w=\"9525\"><a:solidFill><a:srgbClr val=\"000000\"/></a:solidFill></a:ln>",
    );
    let xml = group_drawing(
        OFF, OFF, WIDE, TALL, OFF, OFF, WIDE, TALL, DEG15, false, false, &body,
    );
    assert_eq!(export_err(&xml), ErrorCode::NotImplemented);
}

#[test]
fn a_mismatched_anchor_for_an_angled_group_stays_not_implemented() {
    let (xml, _) = full_child_group(DEG15, false, false);
    // 仍用未旋转的外框当锚点，与转完后的外接框对不上。
    let wrong = xml.replace(&two_cell_from_rot(DEG15), &two_cell_from_rot(0));
    if wrong == xml {
        panic!("failed to swap the anchor");
    }
    assert_eq!(export_err(&wrong), ErrorCode::NotImplemented);
}

fn two_cell_from_rot(rot: i64) -> String {
    let (x0, y0, x1, y1) = group_box_px(OFF, OFF, WIDE, TALL, rot);
    two_cell(x0, y0, x1, y1)
}
