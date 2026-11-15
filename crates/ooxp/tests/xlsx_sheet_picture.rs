//! 工作表上不缩放的不透明 PNG，按原图像素贴上。整像素裁剪也贴这一块。

use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::json;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

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

#[test]
fn an_integer_crop_copies_that_window() {
    let book = cropped_book();
    let (w, h, rgb) = decode_png(&export(&book).unwrap());
    assert_eq!((w, h), (65, 41));
    // 左边裁掉 1 像素，留下第 2 到第 4 列。
    assert_eq!(pixel(&rgb, w, 1, 1), [0x00, 0xFF, 0x00]);
    assert_eq!(pixel(&rgb, w, 3, 1), [0xFF, 0xFF, 0x00]);
    assert_eq!(pixel(&rgb, w, 1, 4), [0x07, 0x08, 0x09]);
    assert_eq!(pixel(&rgb, w, 3, 4), [0x0D, 0x0E, 0x0F]);
    assert_eq!(pixel(&rgb, w, 4, 1), [255, 255, 255]);
    // 四边各 25%，中心 2×2。
    assert_eq!(pixel(&rgb, w, 20, 1), [0x00, 0xFF, 0xFF]);
    assert_eq!(pixel(&rgb, w, 21, 1), [0x11, 0x22, 0x33]);
    assert_eq!(pixel(&rgb, w, 20, 2), [0xAA, 0xBB, 0xCC]);
    assert_eq!(pixel(&rgb, w, 21, 2), [0xDD, 0xEE, 0xFF]);
    // 3 像素宽上 l="33333" 正好 1 像素，留下后两格。
    assert_eq!(pixel(&rgb, w, 40, 1), [0x40, 0x50, 0x60]);
    assert_eq!(pixel(&rgb, w, 41, 1), [0x70, 0x80, 0x90]);
}

#[test]
fn a_fractional_crop_fails_the_sheet() {
    let book = book_with_picture(4 * 9525);
    let cropped = tmp("frac");
    rewrite(&book, &cropped, |name, xml| {
        if name.starts_with("xl/drawings/drawing") {
            inject_src_rects(xml, &[r#"<a:srcRect l="12500"/>"#]);
        }
    });
    assert_eq!(
        export(&cropped).unwrap_err().code(),
        ErrorCode::NotImplemented
    );
}

#[test]
fn a_crop_that_still_scales_fails_the_sheet() {
    let book = book_with_picture(4 * 9525);
    let cropped = tmp("scale-crop");
    rewrite(&book, &cropped, |name, xml| {
        if name.starts_with("xl/drawings/drawing") {
            inject_src_rects(xml, &[r#"<a:srcRect l="25000"/>"#]);
        }
    });
    assert_eq!(
        export(&cropped).unwrap_err().code(),
        ErrorCode::NotImplemented
    );
}

fn cropped_book() -> PathBuf {
    let src = tmp("crop-src");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": src.to_string_lossy(),
        "workbook": { "sheets": [{ "name": "Sheet1", "cells": [{ "ref": "A1", "value": "" }] }] }
    }));
    let dots = tmp("dots").with_extension("png");
    let strip = tmp("strip").with_extension("png");
    write_png(&dots, 4, 4, &DOTS);
    write_png(
        &strip,
        3,
        1,
        &[[0x10, 0x20, 0x30], [0x40, 0x50, 0x60], [0x70, 0x80, 0x90]],
    );
    let shaped = tmp("crop-shaped");
    let px = 9525;
    run_ok(json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": shaped.to_string_lossy(),
        "ops": [
            { "op": "setRowHeight", "id": "sheet:Sheet1", "row": 1, "height": 30 },
            { "op": "setCellStyle", "id": "cell:Sheet1!A1", "font": { "color": { "rgb": "FFFFFFFF" } } },
            { "op": "insertPicture", "id": "sheet:Sheet1", "file": dots.to_string_lossy(),
              "from": "A1", "fromOffX": px, "fromOffY": px, "cx": 3 * px, "cy": 4 * px },
            { "op": "insertPicture", "id": "sheet:Sheet1", "file": dots.to_string_lossy(),
              "from": "A1", "fromOffX": 20 * px, "fromOffY": px, "cx": 2 * px, "cy": 2 * px },
            { "op": "insertPicture", "id": "sheet:Sheet1", "file": strip.to_string_lossy(),
              "from": "A1", "fromOffX": 40 * px, "fromOffY": px, "cx": 2 * px, "cy": px }
        ]
    }));
    let cropped = tmp("crop");
    rewrite(&shaped, &cropped, |name, xml| {
        if name.starts_with("xl/drawings/drawing") {
            inject_src_rects(
                xml,
                &[
                    r#"<a:srcRect l="25000"/>"#,
                    r#"<a:srcRect l="25000" t="25000" r="25000" b="25000"/>"#,
                    r#"<a:srcRect l="33333"/>"#,
                ],
            );
        }
    });
    cropped
}

const DOTS: [[u8; 3]; 16] = [
    [0xFF, 0x00, 0x00],
    [0x00, 0xFF, 0x00],
    [0x00, 0x00, 0xFF],
    [0xFF, 0xFF, 0x00],
    [0xFF, 0x00, 0xFF],
    [0x00, 0xFF, 0xFF],
    [0x11, 0x22, 0x33],
    [0x44, 0x55, 0x66],
    [0x77, 0x88, 0x99],
    [0xAA, 0xBB, 0xCC],
    [0xDD, 0xEE, 0xFF],
    [0x01, 0x02, 0x03],
    [0x04, 0x05, 0x06],
    [0x07, 0x08, 0x09],
    [0x0A, 0x0B, 0x0C],
    [0x0D, 0x0E, 0x0F],
];

fn write_png(path: &Path, width: u32, height: u32, pixels: &[[u8; 3]]) {
    let mut data = Vec::new();
    for px in pixels {
        data.extend_from_slice(px);
    }
    let file = std::fs::File::create(path).unwrap();
    let mut encoder = png::Encoder::new(file, width, height);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(&data)
        .unwrap();
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

fn inject_src_rects(xml: &mut String, rects: &[&str]) {
    let mut rest = 0;
    for rect in rects {
        let start = xml[rest..].find("<a:blip ").expect("blip") + rest;
        let end = xml[start..].find("/>").expect("blip end") + start + 2;
        xml.insert_str(end, rect);
        rest = end + rect.len();
    }
}
