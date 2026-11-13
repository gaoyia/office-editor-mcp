//! exportSlideImage：纯色页底和等像素 PNG。形状不画。

use std::io::Cursor;
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
