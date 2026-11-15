//! exportSlideImage：页底、PNG、实心矩形。没有本页页底时铺白。

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
fn missing_page_background_is_white() {
    let src = tmp("plain");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": src.to_string_lossy(),
        "presentation": { "slides": [{ "shapes": [{ "text": "标题" }] }] }
    }));
    let out = tmp("plain-png").with_extension("png");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "exportSlideImage",
        "path": src.to_string_lossy(),
        "id": "slide:1",
        "output": out.to_string_lossy()
    }));
    let (w, h, rgb) = decode_png(&std::fs::read(&out).unwrap());
    assert_eq!((w, h), (1280, 720));
    assert_eq!(pixel(&rgb, w, 0, 0), [255, 255, 255]);

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

#[test]
fn grouped_rects_keep_their_boxes_and_scaled_groups_map_first() {
    let src = tmp("grp-src");
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
    let title = tree["slides"][0]["shapes"][0]["id"].as_str().unwrap();
    let deck = tmp("grp-deck");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": deck.to_string_lossy(),
        "ops": [
            { "op": "setPageBackground", "id": "slide:1", "color": "AABBCC" },
            { "op": "deleteShape", "id": title },
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
                "x": 2743200,
                "y": 914400,
                "cx": 914400,
                "cy": 457200
            }
        ]
    }));
    let placed = {
        let RunOutput::Inspect(tree) = run_ok(json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": deck.to_string_lossy(),
            "scope": { "slide": 1 }
        })) else {
            panic!("inspect");
        };
        tree
    };
    let shapes = placed["slides"][0]["shapes"].as_array().unwrap();
    assert_eq!(shapes.len(), 2);
    let ids: Vec<&str> = shapes
        .iter()
        .map(|shape| shape["id"].as_str().unwrap())
        .collect();
    let grouped = tmp("grp-grouped");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": deck.to_string_lossy(),
        "output": grouped.to_string_lossy(),
        "ops": [{
            "op": "groupShapes",
            "id": "slide:1",
            "ids": ids
        }]
    }));
    let out = tmp("grp-png").with_extension("png");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "exportSlideImage",
        "path": grouped.to_string_lossy(),
        "id": "slide:1",
        "output": out.to_string_lossy()
    }));
    let (w, _h, rgb) = decode_png(&std::fs::read(&out).unwrap());
    assert_eq!(pixel(&rgb, w, 192, 96), [0xEE, 0x44, 0x55]);
    assert_eq!(pixel(&rgb, w, 287, 143), [0xEE, 0x44, 0x55]);
    assert_eq!(pixel(&rgb, w, 288, 96), [0x11, 0x22, 0x33]);
    assert_eq!(pixel(&rgb, w, 383, 143), [0x11, 0x22, 0x33]);
    assert_eq!(pixel(&rgb, w, 191, 96), [0xAA, 0xBB, 0xCC]);
    assert_eq!(pixel(&rgb, w, 384, 96), [0xAA, 0xBB, 0xCC]);

    let scaled = tmp("grp-scaled");
    rewrite_slide(&grouped, &scaled, |xml| {
        xml.replace(
            "<a:chExt cx=\"1828800\" cy=\"457200\"/>",
            "<a:chExt cx=\"914400\" cy=\"228600\"/>",
        )
    });
    let scaled_png = tmp("grp-scaled-png").with_extension("png");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "exportSlideImage",
        "path": scaled.to_string_lossy(),
        "id": "slide:1",
        "output": scaled_png.to_string_lossy()
    }));
    let (w, _h, rgb) = decode_png(&std::fs::read(&scaled_png).unwrap());
    assert_eq!(pixel(&rgb, w, 192, 96), [0xEE, 0x44, 0x55]);
    assert_eq!(pixel(&rgb, w, 383, 191), [0xEE, 0x44, 0x55]);
    assert_eq!(pixel(&rgb, w, 384, 96), [0x11, 0x22, 0x33]);
    assert_eq!(pixel(&rgb, w, 575, 191), [0x11, 0x22, 0x33]);
    assert_eq!(pixel(&rgb, w, 191, 96), [0xAA, 0xBB, 0xCC]);
    assert_eq!(pixel(&rgb, w, 576, 96), [0xAA, 0xBB, 0xCC]);

    let turned = tmp("grp-rot");
    rewrite_slide(&grouped, &turned, |xml| {
        xml.replace(
            "<a:xfrm><a:off x=\"1828800\" y=\"914400\"/><a:ext cx=\"1828800\"",
            "<a:xfrm rot=\"5400000\"><a:off x=\"1828800\" y=\"914400\"/><a:ext cx=\"1828800\"",
        )
    });
    let err = run(&json!({
        "schemaVersion": 1,
        "action": "exportSlideImage",
        "path": turned.to_string_lossy(),
        "id": "slide:1",
        "output": tmp("grp-rot-png").with_extension("png").to_string_lossy()
    })
    .to_string())
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::NotImplemented);
}

#[test]
fn whole_pixel_crop_copies_the_source_and_a_fractional_crop_fails() {
    let src = tmp("crop-src");
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
    let title = tree["slides"][0]["shapes"][0]["id"].as_str().unwrap();
    let png_path = src.with_extension("png");
    let mut raw = Vec::new();
    for (r, g, b) in [
        (0xFFu8, 0x00, 0x00),
        (0x00, 0xFF, 0x00),
        (0x00, 0x00, 0xFF),
        (0xFF, 0xFF, 0x00),
        (0xFF, 0x00, 0xFF),
        (0x00, 0xFF, 0xFF),
        (0x11, 0x22, 0x33),
        (0x44, 0x55, 0x66),
        (0x77, 0x88, 0x99),
        (0xAA, 0xBB, 0xCC),
        (0xDD, 0xEE, 0xFF),
        (0x01, 0x02, 0x03),
        (0x04, 0x05, 0x06),
        (0x07, 0x08, 0x09),
        (0x0A, 0x0B, 0x0C),
        (0x0D, 0x0E, 0x0F),
    ] {
        raw.extend_from_slice(&[r, g, b]);
    }
    let mut png = Vec::new();
    let mut encoder = png::Encoder::new(&mut png, 4, 4);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(&raw)
        .unwrap();
    std::fs::write(&png_path, png).unwrap();

    let deck = tmp("crop-deck");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": deck.to_string_lossy(),
        "ops": [
            { "op": "setPageBackground", "id": "slide:1", "color": "F0F0F0" },
            { "op": "deleteShape", "id": title },
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
    let cropped = tmp("crop-whole");
    rewrite_slide(&deck, &cropped, |xml| {
        xml.replace(
            "<a:blip r:embed=\"rId2\"/>",
            "<a:blip r:embed=\"rId2\"/><a:srcRect l=\"25000\" t=\"25000\" r=\"25000\" b=\"25000\"/>",
        )
    });
    let out = tmp("crop-png").with_extension("png");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "exportSlideImage",
        "path": cropped.to_string_lossy(),
        "id": "slide:1",
        "output": out.to_string_lossy()
    }));
    let (w, _h, rgb) = decode_png(&std::fs::read(&out).unwrap());
    assert_eq!(pixel(&rgb, w, 96, 96), [0x00, 0xFF, 0xFF]);
    assert_eq!(pixel(&rgb, w, 97, 96), [0x11, 0x22, 0x33]);
    assert_eq!(pixel(&rgb, w, 96, 97), [0xAA, 0xBB, 0xCC]);
    assert_eq!(pixel(&rgb, w, 97, 97), [0xDD, 0xEE, 0xFF]);
    assert_eq!(pixel(&rgb, w, 95, 96), [0xF0, 0xF0, 0xF0]);

    let fractional = tmp("crop-frac");
    rewrite_slide(&deck, &fractional, |xml| {
        xml.replace(
            "<a:blip r:embed=\"rId2\"/>",
            "<a:blip r:embed=\"rId2\"/><a:srcRect l=\"12500\"/>",
        )
    });
    let err = run(&json!({
        "schemaVersion": 1,
        "action": "exportSlideImage",
        "path": fractional.to_string_lossy(),
        "id": "slide:1",
        "output": tmp("crop-frac-png").with_extension("png").to_string_lossy()
    })
    .to_string())
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::NotImplemented);
}

#[test]
fn layout_and_master_shapes_paint_under_the_slide() {
    let src = tmp("master-src");
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
    let title = tree["slides"][0]["shapes"][0]["id"].as_str().unwrap();
    let deck = tmp("master-deck");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": deck.to_string_lossy(),
        "ops": [
            { "op": "setPageBackground", "id": "slide:1", "color": "F0F0F0" },
            { "op": "deleteShape", "id": title },
            {
                "op": "insertShape",
                "id": "master:1/layout:1",
                "fill": "EE4455",
                "x": 914400,
                "y": 914400,
                "cx": 914400,
                "cy": 457200
            },
            {
                "op": "insertShape",
                "id": "master:1",
                "fill": "112233",
                "x": 914400,
                "y": 1828800,
                "cx": 914400,
                "cy": 457200
            },
            {
                "op": "insertShape",
                "id": "slide:1",
                "fill": "00AA66",
                "x": 1524000,
                "y": 914400,
                "cx": 304800,
                "cy": 457200
            }
        ]
    }));
    let out = tmp("master-png").with_extension("png");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "exportSlideImage",
        "path": deck.to_string_lossy(),
        "id": "slide:1",
        "output": out.to_string_lossy()
    }));
    let (w, _h, rgb) = decode_png(&std::fs::read(&out).unwrap());
    assert_eq!(pixel(&rgb, w, 96, 96), [0xEE, 0x44, 0x55]);
    assert_eq!(pixel(&rgb, w, 159, 143), [0xEE, 0x44, 0x55]);
    assert_eq!(pixel(&rgb, w, 160, 96), [0x00, 0xAA, 0x66]);
    assert_eq!(pixel(&rgb, w, 191, 143), [0x00, 0xAA, 0x66]);
    assert_eq!(pixel(&rgb, w, 96, 192), [0x11, 0x22, 0x33]);
    assert_eq!(pixel(&rgb, w, 191, 239), [0x11, 0x22, 0x33]);
    assert_eq!(pixel(&rgb, w, 95, 96), [0xF0, 0xF0, 0xF0]);
    assert_eq!(pixel(&rgb, w, 96, 191), [0xF0, 0xF0, 0xF0]);

    let hidden = tmp("master-hidden");
    rewrite_slide(&deck, &hidden, |xml| {
        xml.replace("<p:sld ", "<p:sld showMasterSp=\"0\" ")
    });
    let hidden_png = tmp("master-hidden-png").with_extension("png");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "exportSlideImage",
        "path": hidden.to_string_lossy(),
        "id": "slide:1",
        "output": hidden_png.to_string_lossy()
    }));
    let (w, _h, rgb) = decode_png(&std::fs::read(&hidden_png).unwrap());
    assert_eq!(pixel(&rgb, w, 96, 96), [0xF0, 0xF0, 0xF0]);
    assert_eq!(pixel(&rgb, w, 96, 192), [0xF0, 0xF0, 0xF0]);
    assert_eq!(pixel(&rgb, w, 160, 96), [0x00, 0xAA, 0x66]);
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
