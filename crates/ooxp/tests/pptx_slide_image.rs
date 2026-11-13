//! exportSlideImage：纯色页底、等像素 PNG，以及没有描边的实心矩形。文字不画。

use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::json;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-pptx-slide-image");
    std::fs::create_dir_all(&dir).unwrap();
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    dir.join(format!("{tag}-{n}.pptx"))
}

fn run_ok(value: serde_json::Value) -> RunOutput {
    run(&value.to_string()).unwrap()
}

fn solid_png(w: u32, h: u32, rgb: [u8; 3]) -> Vec<u8> {
    let mut raw = Vec::with_capacity((w as usize) * (h as usize) * 3);
    for _ in 0..(w * h) {
        raw.extend_from_slice(&rgb);
    }
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, w, h);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(&raw)
        .unwrap();
    out
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

fn pixel(rgb: &[u8], w: u32, x: u32, y: u32) -> [u8; 3] {
    let i = ((y as usize) * (w as usize) + x as usize) * 3;
    [rgb[i], rgb[i + 1], rgb[i + 2]]
}

#[test]
fn solid_background_and_png_are_copied_at_96_dpi() {
    let src = tmp("src");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": src.to_string_lossy(),
        "presentation": { "slides": [{ "shapes": [{ "text": "标题" }] }] }
    }));
    let tree = {
        let RunOutput::Inspect(tree) = run_ok(json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": src.to_string_lossy(),
            "scope": { "slide": 1 }
        })) else {
            panic!("inspect");
        };
        tree
    };
    let shape_id = tree["slides"][0]["shapes"][0]["id"].as_str().unwrap();
    let png_path = src.with_extension("png");
    std::fs::write(&png_path, solid_png(2, 2, [0x11, 0x22, 0x33])).unwrap();
    let deck = tmp("deck");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": deck.to_string_lossy(),
        "ops": [
            { "op": "setPageBackground", "id": "slide:1", "color": "AABBCC" },
            { "op": "deleteShape", "id": shape_id },
            {
                "op": "insertPicture",
                "id": "slide:1",
                "file": png_path.to_string_lossy(),
                "x": 914400,
                "y": 914400,
                "cx": 19050,
                "cy": 19050
            }
        ]
    }));
    let out = tmp("slide").with_extension("png");
    let written = run_ok(json!({
        "schemaVersion": 1,
        "action": "exportSlideImage",
        "path": deck.to_string_lossy(),
        "id": "slide:1",
        "output": out.to_string_lossy()
    }));
    let RunOutput::Written { content_type, .. } = written else {
        panic!("应写出文件");
    };
    assert_eq!(content_type.as_deref(), Some("image/png"));
    let (w, h, rgb) = decode_png(&std::fs::read(&out).unwrap());
    assert_eq!((w, h), (1280, 720));
    assert_eq!(pixel(&rgb, w, 0, 0), [0xAA, 0xBB, 0xCC]);
    assert_eq!(pixel(&rgb, w, 96, 96), [0x11, 0x22, 0x33]);
    assert_eq!(pixel(&rgb, w, 97, 97), [0x11, 0x22, 0x33]);
    assert_eq!(pixel(&rgb, w, 98, 96), [0xAA, 0xBB, 0xCC]);
    let _ = Path::new(&out);
}

#[test]
fn missing_solid_background_and_other_formats_are_rejected() {
    let src = tmp("plain");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": src.to_string_lossy(),
        "presentation": { "slides": [{ "shapes": [{ "text": "标题" }] }] }
    }));
    let out = tmp("plain-png").with_extension("png");
    let err = run(&json!({
        "schemaVersion": 1,
        "action": "exportSlideImage",
        "path": src.to_string_lossy(),
        "id": "slide:1",
        "output": out.to_string_lossy()
    })
    .to_string())
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::NotImplemented);
    assert!(!out.exists());

    let err = run(&json!({
        "schemaVersion": 1,
        "action": "exportSlideImage",
        "path": src.to_string_lossy(),
        "id": "slide:1/shape:2",
        "output": out.to_string_lossy()
    })
    .to_string())
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);

    let book = tmp("book").with_extension("xlsx");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": book.to_string_lossy(),
        "workbook": { "sheets": [{ "name": "Sheet1", "cells": [] }] }
    }));
    let err = run(&json!({
        "schemaVersion": 1,
        "action": "exportSlideImage",
        "path": book.to_string_lossy(),
        "id": "slide:1",
        "output": out.to_string_lossy()
    })
    .to_string())
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

#[test]
fn solid_rect_is_painted_and_stroke_or_text_is_not() {
    let src = tmp("rect-src");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": src.to_string_lossy(),
        "presentation": { "slides": [{ "shapes": [{ "text": "标题" }] }] }
    }));
    let deck = tmp("rect-deck");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": deck.to_string_lossy(),
        "ops": [
            { "op": "setPageBackground", "id": "slide:1", "color": "AABBCC" },
            {
                "op": "insertShape",
                "id": "slide:1",
                "fill": "EE4455",
                "x": 1828800,
                "y": 914400,
                "cx": 914400,
                "cy": 457200
            },
            {
                "op": "insertShape",
                "id": "slide:1",
                "fill": "112233",
                "line": { "color": "000000" },
                "x": 2743200,
                "y": 914400,
                "cx": 914400,
                "cy": 457200
            }
        ]
    }));
    let out = tmp("rect-png").with_extension("png");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "exportSlideImage",
        "path": deck.to_string_lossy(),
        "id": "slide:1",
        "output": out.to_string_lossy()
    }));
    let (w, _h, rgb) = decode_png(&std::fs::read(&out).unwrap());
    assert_eq!(pixel(&rgb, w, 192, 96), [0xEE, 0x44, 0x55]);
    assert_eq!(pixel(&rgb, w, 287, 143), [0xEE, 0x44, 0x55]);
    assert_eq!(pixel(&rgb, w, 191, 96), [0xAA, 0xBB, 0xCC]);
    assert_eq!(pixel(&rgb, w, 288, 96), [0xAA, 0xBB, 0xCC]);
    assert_eq!(pixel(&rgb, w, 300, 100), [0xAA, 0xBB, 0xCC]);
    assert_eq!(pixel(&rgb, w, 90, 80), [0xAA, 0xBB, 0xCC]);
}

#[test]
fn later_rect_covers_earlier_and_the_page_clips() {
    let src = tmp("stack-src");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": src.to_string_lossy(),
        "presentation": { "slides": [{ "shapes": [{ "text": "标题" }] }] }
    }));
    let tree = {
        let RunOutput::Inspect(tree) = run_ok(json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": src.to_string_lossy(),
            "scope": { "slide": 1 }
        })) else {
            panic!("inspect");
        };
        tree
    };
    let shape_id = tree["slides"][0]["shapes"][0]["id"].as_str().unwrap();
    let deck = tmp("stack-deck");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": deck.to_string_lossy(),
        "ops": [
            { "op": "setPageBackground", "id": "slide:1", "color": "AABBCC" },
            { "op": "deleteShape", "id": shape_id },
            {
                "op": "insertShape",
                "id": "slide:1",
                "fill": "EE4455",
                "x": 1828800,
                "y": 914400,
                "cx": 914400,
                "cy": 457200
            },
            {
                "op": "insertShape",
                "id": "slide:1",
                "fill": "112233",
                "x": 2286000,
                "y": 914400,
                "cx": 914400,
                "cy": 457200
            },
            {
                "op": "insertShape",
                "id": "slide:1",
                "fill": "00FF00",
                "x": -9525,
                "y": 1828800,
                "cx": 28575,
                "cy": 457200
            },
            {
                "op": "insertShape",
                "id": "slide:1",
                "fill": { "type": "solid", "color": "EE4455", "alpha": 50000 },
                "x": 3657600,
                "y": 914400,
                "cx": 914400,
                "cy": 457200
            }
        ]
    }));
    let out = tmp("stack-png").with_extension("png");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "exportSlideImage",
        "path": deck.to_string_lossy(),
        "id": "slide:1",
        "output": out.to_string_lossy()
    }));
    let (w, _h, rgb) = decode_png(&std::fs::read(&out).unwrap());
    assert_eq!(pixel(&rgb, w, 239, 96), [0xEE, 0x44, 0x55]);
    assert_eq!(pixel(&rgb, w, 240, 96), [0x11, 0x22, 0x33]);
    assert_eq!(pixel(&rgb, w, 335, 96), [0x11, 0x22, 0x33]);
    assert_eq!(pixel(&rgb, w, 336, 96), [0xAA, 0xBB, 0xCC]);
    assert_eq!(pixel(&rgb, w, 0, 192), [0x00, 0xFF, 0x00]);
    assert_eq!(pixel(&rgb, w, 1, 239), [0x00, 0xFF, 0x00]);
    assert_eq!(pixel(&rgb, w, 2, 192), [0xAA, 0xBB, 0xCC]);
    assert_eq!(pixel(&rgb, w, 384, 96), [0xAA, 0xBB, 0xCC]);
}

#[test]
fn quarter_turn_swaps_the_box_around_its_center() {
    let src = tmp("turn-src");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": src.to_string_lossy(),
        "presentation": { "slides": [{ "shapes": [{ "text": "标题" }] }] }
    }));
    let tree = {
        let RunOutput::Inspect(tree) = run_ok(json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": src.to_string_lossy(),
            "scope": { "slide": 1 }
        })) else {
            panic!("inspect");
        };
        tree
    };
    let shape_id = tree["slides"][0]["shapes"][0]["id"].as_str().unwrap();
    let deck = tmp("turn-deck");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": deck.to_string_lossy(),
        "ops": [
            { "op": "setPageBackground", "id": "slide:1", "color": "AABBCC" },
            { "op": "deleteShape", "id": shape_id },
            {
                "op": "insertShape",
                "id": "slide:1",
                "fill": "EE4455",
                "x": 1828800,
                "y": 914400,
                "cx": 914400,
                "cy": 457200
            },
            {
                "op": "insertShape",
                "id": "slide:1",
                "fill": "112233",
                "x": 3657600,
                "y": 914400,
                "cx": 914400,
                "cy": 457200
            },
            {
                "op": "insertShape",
                "id": "slide:1",
                "fill": "00AA66",
                "x": 5486400,
                "y": 914400,
                "cx": 914400,
                "cy": 457200
            }
        ]
    }));
    let turned = tmp("turn-rot");
    rewrite_slide(&deck, &turned, |xml| {
        xml.replace(
            "<a:xfrm><a:off x=\"1828800\"",
            "<a:xfrm rot=\"5400000\"><a:off x=\"1828800\"",
        )
        .replace(
            "<a:xfrm><a:off x=\"3657600\"",
            "<a:xfrm rot=\"100\"><a:off x=\"3657600\"",
        )
        .replace(
            "<a:xfrm><a:off x=\"5486400\"",
            "<a:xfrm rot=\"10800000\"><a:off x=\"5486400\"",
        )
    });
    let out = tmp("turn-png").with_extension("png");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "exportSlideImage",
        "path": turned.to_string_lossy(),
        "id": "slide:1",
        "output": out.to_string_lossy()
    }));
    let (w, _h, rgb) = decode_png(&std::fs::read(&out).unwrap());
    assert_eq!(pixel(&rgb, w, 216, 72), [0xEE, 0x44, 0x55]);
    assert_eq!(pixel(&rgb, w, 263, 167), [0xEE, 0x44, 0x55]);
    assert_eq!(pixel(&rgb, w, 215, 72), [0xAA, 0xBB, 0xCC]);
    assert_eq!(pixel(&rgb, w, 216, 71), [0xAA, 0xBB, 0xCC]);
    assert_eq!(pixel(&rgb, w, 264, 120), [0xAA, 0xBB, 0xCC]);
    assert_eq!(pixel(&rgb, w, 216, 168), [0xAA, 0xBB, 0xCC]);
    assert_eq!(pixel(&rgb, w, 384, 96), [0xAA, 0xBB, 0xCC]);
    assert_eq!(pixel(&rgb, w, 576, 96), [0x00, 0xAA, 0x66]);
    assert_eq!(pixel(&rgb, w, 671, 143), [0x00, 0xAA, 0x66]);
    assert_eq!(pixel(&rgb, w, 672, 96), [0xAA, 0xBB, 0xCC]);
}

fn rewrite_slide(src: &Path, dst: &Path, edit: impl FnOnce(String) -> String) {
    let bytes = std::fs::read(src).unwrap();
    let mut zip_in = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut out = Cursor::new(Vec::new());
    let mut edit = Some(edit);
    {
        let mut zip_out = zip::ZipWriter::new(&mut out);
        let opt = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        for i in 0..zip_in.len() {
            let mut file = zip_in.by_index(i).unwrap();
            let name = file.name().to_string();
            let mut data = Vec::new();
            file.read_to_end(&mut data).unwrap();
            if name == "ppt/slides/slide1.xml" {
                let xml = String::from_utf8(data).unwrap();
                data = edit.take().unwrap()(xml).into_bytes();
            }
            zip_out.start_file(&name, opt).unwrap();
            zip_out.write_all(&data).unwrap();
        }
        zip_out.finish().unwrap();
    }
    std::fs::write(dst, out.into_inner()).unwrap();
}
