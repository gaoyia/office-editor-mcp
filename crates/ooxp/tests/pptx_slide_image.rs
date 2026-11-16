//! exportSlideImage：页底、PNG、实心矩形、形状图片填充、渐变、多行文本，以及水平或垂直的直线。没有本页页底时铺白。

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
fn rect_stroke_corners_cover_one_and_two_pixel_widths() {
    let src = tmp("stroke-src");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": src.to_string_lossy(),
        "presentation": {
            "slides": [
                { "shapes": [{ "text": "", "prst": "rect", "fill": "FFFFFF", "line": { "color": "000000", "width": 9525 }, "x": 914400, "y": 914400, "cx": 1828800, "cy": 1828800 }] },
                { "shapes": [{ "text": "", "prst": "rect", "fill": null, "line": { "color": "000000", "width": 19050 }, "x": 914400, "y": 914400, "cx": 1828800, "cy": 1828800 }] }
            ]
        }
    }));
    let out = tmp("stroke-1-png").with_extension("png");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "exportSlideImage",
        "path": src.to_string_lossy(),
        "id": "slide:1",
        "output": out.to_string_lossy()
    }));
    let (w, h, rgb) = decode_png(&std::fs::read(&out).unwrap());
    assert_eq!((w, h), (1280, 720));
    assert_eq!(pixel(&rgb, w, 96, 96), [48, 48, 48]);
    assert_eq!(pixel(&rgb, w, 288, 96), [48, 48, 48]);
    assert_eq!(pixel(&rgb, w, 96, 288), [16, 16, 16]);
    assert_eq!(pixel(&rgb, w, 288, 288), [16, 16, 16]);
    assert_eq!(pixel(&rgb, w, 192, 96), [0, 0, 0]);
    assert_eq!(pixel(&rgb, w, 120, 120), [255, 255, 255]);
    let out = tmp("stroke-2-png").with_extension("png");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "exportSlideImage",
        "path": src.to_string_lossy(),
        "id": "slide:2",
        "output": out.to_string_lossy()
    }));
    let (w, h, rgb) = decode_png(&std::fs::read(&out).unwrap());
    assert_eq!((w, h), (1280, 720));
    assert_eq!(pixel(&rgb, w, 95, 95), [120, 120, 120]);
    assert_eq!(pixel(&rgb, w, 288, 95), [96, 96, 96]);
    assert_eq!(pixel(&rgb, w, 288, 288), [32, 32, 32]);
    assert_eq!(pixel(&rgb, w, 96, 96), [0, 0, 0]);
    assert_eq!(pixel(&rgb, w, 192, 95), [0, 0, 0]);
}

#[test]
fn scheme_rect_uses_the_theme_slot_and_a_transform_fails() {
    let src = tmp("scheme-src");
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
    let deck = tmp("scheme-deck");
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
                "fill": "FFFFFF",
                "x": 1828800,
                "y": 914400,
                "cx": 914400,
                "cy": 457200
            },
            {
                "op": "insertShape",
                "id": "slide:1",
                "fill": "FFFFFF",
                "x": 3657600,
                "y": 914400,
                "cx": 457200,
                "cy": 457200
            },
            {
                "op": "insertShape",
                "id": "slide:1",
                "fill": "FFFFFF",
                "x": 4572000,
                "y": 1828800,
                "cx": 457200,
                "cy": 457200
            }
        ]
    }));
    let white = "<a:solidFill><a:srgbClr val=\"FFFFFF\"/></a:solidFill>";
    let painted = tmp("scheme-painted");
    rewrite_named(&deck, &painted, |name, xml| match name {
        "ppt/slides/slide1.xml" => {
            let xml = swap(
                xml,
                white,
                "<a:solidFill><a:schemeClr val=\"accent1\"/></a:solidFill>",
            );
            let xml = swap(
                xml,
                white,
                "<a:solidFill><a:schemeClr val=\"dk1\"/></a:solidFill>",
            );
            swap(
                xml,
                white,
                "<a:solidFill><a:schemeClr val=\"bg1\"/></a:solidFill>",
            )
        }
        "ppt/theme/theme1.xml" => swap(
            xml,
            "<a:accent1><a:srgbClr val=\"4F81BD\"/></a:accent1>",
            "<a:accent1><a:srgbClr val=\"EE4455\"/></a:accent1>",
        ),
        _ => xml,
    });
    let out = tmp("scheme-png").with_extension("png");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "exportSlideImage",
        "path": painted.to_string_lossy(),
        "id": "slide:1",
        "output": out.to_string_lossy()
    }));
    let (w, _h, rgb) = decode_png(&std::fs::read(&out).unwrap());
    assert_eq!(pixel(&rgb, w, 200, 100), [0xEE, 0x44, 0x55]);
    assert_eq!(pixel(&rgb, w, 192, 96), [0xEE, 0x44, 0x55]);
    assert_eq!(pixel(&rgb, w, 191, 100), [0xAA, 0xBB, 0xCC]);
    assert_eq!(pixel(&rgb, w, 400, 120), [0, 0, 0]);
    assert_eq!(pixel(&rgb, w, 500, 200), [255, 255, 255]);

    let mapped = tmp("scheme-mapped");
    rewrite_slide(&painted, &mapped, |xml| {
        swap(
            xml,
            "<a:masterClrMapping/>",
            "<a:overrideClrMapping bg1=\"lt1\" tx1=\"dk1\" bg2=\"lt2\" tx2=\"dk2\" accent1=\"accent2\" accent2=\"accent2\" accent3=\"accent3\" accent4=\"accent4\" accent5=\"accent5\" accent6=\"accent6\" hlink=\"hlink\" folHlink=\"folHlink\"/>",
        )
    });
    let mapped_png = tmp("scheme-mapped-png").with_extension("png");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "exportSlideImage",
        "path": mapped.to_string_lossy(),
        "id": "slide:1",
        "output": mapped_png.to_string_lossy()
    }));
    let (w, _h, rgb) = decode_png(&std::fs::read(&mapped_png).unwrap());
    assert_eq!(pixel(&rgb, w, 200, 100), [0xC0, 0x50, 0x4D]);
    assert_eq!(pixel(&rgb, w, 400, 120), [0, 0, 0]);

    let shifted = tmp("scheme-lum");
    rewrite_slide(&painted, &shifted, |xml| {
        swap(
            xml,
            "<a:schemeClr val=\"accent1\"/>",
            "<a:schemeClr val=\"accent1\"><a:lumMod val=\"50000\"/></a:schemeClr>",
        )
    });
    let err = run(&json!({
        "schemaVersion": 1,
        "action": "exportSlideImage",
        "path": shifted.to_string_lossy(),
        "id": "slide:1",
        "output": tmp("scheme-lum-png").with_extension("png").to_string_lossy()
    })
    .to_string())
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::NotImplemented);
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
    // 50% `EE4455` 叠在页底 `AABBCC` 上，与 PowerPoint 16.0 一致。
    assert_eq!(pixel(&rgb, w, 384, 96), [0xCC, 0x7F, 0x91]);
}

#[test]
fn translucent_solid_rects_blend_over_the_page() {
    let src = tmp("alpha-src");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": src.to_string_lossy(),
        "presentation": { "slides": [
            { "shapes": [{ "text": "", "prst": "rect", "fill": { "type": "solid", "color": "EE4455", "alpha": 50000 }, "x": 1828800, "y": 914400, "cx": 914400, "cy": 457200 }] },
            { "shapes": [{ "text": "", "prst": "rect", "fill": { "type": "solid", "color": "EE4455", "alpha": 50000 }, "x": 1828800, "y": 914400, "cx": 914400, "cy": 457200 }] },
            { "shapes": [{ "text": "", "prst": "rect", "fill": { "type": "solid", "color": "EE4455", "alpha": 50000 }, "x": 1828800, "y": 914400, "cx": 914400, "cy": 457200 }] },
            { "shapes": [{ "text": "", "prst": "rect", "fill": { "type": "solid", "color": "0000FF", "alpha": 50000 }, "x": 1828800, "y": 914400, "cx": 914400, "cy": 457200 }] },
            { "shapes": [{ "text": "", "prst": "rect", "fill": { "type": "solid", "color": "FFFFFF", "alpha": 50000 }, "x": 1828800, "y": 914400, "cx": 914400, "cy": 457200 }] },
            { "shapes": [{ "text": "", "prst": "rect", "fill": { "type": "solid", "color": "000000", "alpha": 50000 }, "x": 1828800, "y": 914400, "cx": 914400, "cy": 457200 }] },
            { "shapes": [{ "text": "", "prst": "rect", "fill": { "type": "solid", "color": "00FF00", "alpha": 50000 }, "x": 1828800, "y": 914400, "cx": 914400, "cy": 457200 }] },
            { "shapes": [{ "text": "", "prst": "rect", "fill": { "type": "solid", "color": "EE4455", "alpha": 50000 }, "x": 1828800, "y": 914400, "cx": 914400, "cy": 457200 }] },
            { "shapes": [{ "text": "", "prst": "rect", "fill": { "type": "solid", "color": "EE4455", "alpha": 25000 }, "x": 1828800, "y": 914400, "cx": 914400, "cy": 457200 }] },
            { "shapes": [{ "text": "", "prst": "rect", "fill": { "type": "solid", "color": "EE4455", "alpha": 75000 }, "x": 1828800, "y": 914400, "cx": 914400, "cy": 457200 }] },
            { "shapes": [{ "text": "", "prst": "rect", "fill": { "type": "solid", "color": "EE4455", "alpha": 10000 }, "x": 1828800, "y": 914400, "cx": 914400, "cy": 457200 }] },
            { "shapes": [
                { "text": "", "prst": "rect", "fill": "EE4455", "x": 1828800, "y": 914400, "cx": 914400, "cy": 457200 },
                { "text": "", "prst": "rect", "fill": { "type": "solid", "color": "0000FF", "alpha": 50000 }, "x": 2057400, "y": 914400, "cx": 914400, "cy": 457200 }
            ] },
            { "shapes": [
                { "text": "", "prst": "rect", "fill": { "type": "solid", "color": "EE4455", "alpha": 50000 }, "x": 1828800, "y": 914400, "cx": 914400, "cy": 457200 },
                { "text": "", "prst": "rect", "fill": { "type": "solid", "color": "0000FF", "alpha": 50000 }, "x": 2057400, "y": 914400, "cx": 914400, "cy": 457200 }
            ] },
            { "shapes": [{ "text": "", "prst": "rect", "fill": { "type": "solid", "color": "EE4455", "alpha": 50000 }, "x": 1828800, "y": 914400, "cx": 914400, "cy": 457200 }] },
            { "shapes": [{ "text": "", "prst": "rect", "fill": { "type": "solid", "color": "EE4455", "alpha": 50000 }, "x": -9525, "y": 1828800, "cx": 28575, "cy": 457200 }] }
        ] }
    }));
    let deck = tmp("alpha-deck");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": deck.to_string_lossy(),
        "ops": [
            { "op": "setPageBackground", "id": "slide:1", "color": "AABBCC" },
            { "op": "setPageBackground", "id": "slide:2", "color": "FFFFFF" },
            { "op": "setPageBackground", "id": "slide:3", "color": "000000" },
            { "op": "setPageBackground", "id": "slide:4", "color": "AABBCC" },
            { "op": "setPageBackground", "id": "slide:5", "color": "AABBCC" },
            { "op": "setPageBackground", "id": "slide:6", "color": "AABBCC" },
            { "op": "setPageBackground", "id": "slide:7", "color": "AABBCC" },
            { "op": "setPageBackground", "id": "slide:8", "color": "112233" },
            { "op": "setPageBackground", "id": "slide:9", "color": "AABBCC" },
            { "op": "setPageBackground", "id": "slide:10", "color": "AABBCC" },
            { "op": "setPageBackground", "id": "slide:11", "color": "AABBCC" },
            { "op": "setPageBackground", "id": "slide:12", "color": "AABBCC" },
            { "op": "setPageBackground", "id": "slide:13", "color": "AABBCC" },
            { "op": "setPageBackground", "id": "slide:15", "color": "AABBCC" }
        ]
    }));
    let sample = |id: &str, x: u32, y: u32| -> [u8; 3] {
        let tag = id.replace(":", "-");
        let out = tmp(&format!("alpha-{tag}")).with_extension("png");
        run_ok(json!({
            "schemaVersion": 1,
            "action": "exportSlideImage",
            "path": deck.to_string_lossy(),
            "id": id,
            "output": out.to_string_lossy()
        }));
        let (w, _h, rgb) = decode_png(&std::fs::read(&out).unwrap());
        pixel(&rgb, w, x, y)
    };
    assert_eq!(sample("slide:1", 220, 110), [0xCC, 0x7F, 0x91]);
    assert_eq!(sample("slide:1", 192, 96), [0xCC, 0x7F, 0x91]);
    assert_eq!(sample("slide:1", 287, 143), [0xCC, 0x7F, 0x91]);
    assert_eq!(sample("slide:1", 10, 10), [0xAA, 0xBB, 0xCC]);
    assert_eq!(sample("slide:2", 220, 110), [0xF6, 0xA1, 0xAA]);
    assert_eq!(sample("slide:3", 220, 110), [0x77, 0x22, 0x2B]);
    assert_eq!(sample("slide:4", 220, 110), [0x55, 0x5D, 0xE6]);
    assert_eq!(sample("slide:5", 220, 110), [0xD5, 0xDD, 0xE6]);
    assert_eq!(sample("slide:6", 220, 110), [0x55, 0x5D, 0x66]);
    assert_eq!(sample("slide:7", 220, 110), [0x55, 0xDD, 0x66]);
    assert_eq!(sample("slide:8", 220, 110), [0x7F, 0x33, 0x44]);
    assert_eq!(sample("slide:9", 220, 110), [0xBB, 0x9D, 0xAE]);
    assert_eq!(sample("slide:10", 220, 110), [0xDD, 0x62, 0x73]);
    assert_eq!(sample("slide:11", 220, 110), [0xB1, 0xAF, 0xC0]);
    // 不透明红 (192..287) 与 50% 蓝 (216..311) 重叠。
    assert_eq!(sample("slide:12", 200, 110), [0xEE, 0x44, 0x55]);
    assert_eq!(sample("slide:12", 260, 110), [0x77, 0x22, 0xAA]);
    assert_eq!(sample("slide:12", 310, 110), [0x55, 0x5D, 0xE6]);
    assert_eq!(sample("slide:13", 200, 110), [0xCC, 0x7F, 0x91]);
    assert_eq!(sample("slide:13", 260, 110), [0x66, 0x3F, 0xC8]);
    assert_eq!(sample("slide:13", 310, 110), [0x55, 0x5D, 0xE6]);
    assert_eq!(sample("slide:14", 220, 110), [0xF6, 0xA1, 0xAA]);
    assert_eq!(sample("slide:14", 10, 10), [0xFF, 0xFF, 0xFF]);
    assert_eq!(sample("slide:15", 0, 192), [0xCC, 0x7F, 0x91]);
    assert_eq!(sample("slide:15", 1, 192), [0xCC, 0x7F, 0x91]);
    assert_eq!(sample("slide:15", 2, 192), [0xAA, 0xBB, 0xCC]);
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
    // rot=100（约 0.00167 度）原先整块跳过。Windows 上已画非直角不透明纯色矩形，取中心。
    #[cfg(windows)]
    assert_eq!(pixel(&rgb, w, 432, 120), [0x11, 0x22, 0x33]);
    #[cfg(not(windows))]
    assert_eq!(pixel(&rgb, w, 384, 96), [0xAA, 0xBB, 0xCC]);
    assert_eq!(pixel(&rgb, w, 576, 96), [0x00, 0xAA, 0x66]);
    assert_eq!(pixel(&rgb, w, 671, 143), [0x00, 0xAA, 0x66]);
    assert_eq!(pixel(&rgb, w, 672, 96), [0xAA, 0xBB, 0xCC]);
}

#[cfg(windows)]
#[test]
fn non_right_angle_opaque_rects_keep_center_and_outside_on_windows() {
    // 不透明无描边纯色矩形：多角度、方向、尺寸、原点。中心必须是填色，明显在外接盒外仍是页底。
    let cases: &[(&str, i64, i64, i64, i64, i64)] = &[
        ("rot45", 2_700_000, 1_828_800, 914_400, 609_600, 609_600),
        ("rot30", 1_800_000, 1_828_800, 914_400, 609_600, 609_600),
        ("rot15", 900_000, 1_828_800, 914_400, 609_600, 609_600),
        ("rot60", 3_600_000, 1_828_800, 914_400, 609_600, 609_600),
        ("rot120", 7_200_000, 1_828_800, 914_400, 609_600, 609_600),
        ("rot315", 18_900_000, 1_828_800, 914_400, 609_600, 609_600),
        ("rot225", 1_350_000, 1_828_800, 914_400, 609_600, 609_600),
        ("wide45", 2_700_000, 1_828_800, 914_400, 914_400, 457_200),
        ("tall30", 1_800_000, 1_828_800, 914_400, 304_800, 914_400),
        (
            "origin45", 2_700_000, 3_810_000, 1_905_000, 609_600, 609_600,
        ),
        ("near45", 2_700_000, 95_250, 1_905_000, 914_400, 457_200),
    ];
    let bg = [0xAA, 0xBB, 0xCC];
    let fill = [0xEE, 0x44, 0x55];
    for (tag, rot, x, y, cx, cy) in cases {
        let src = tmp(&format!("{tag}-src"));
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
        let deck = tmp(&format!("{tag}-deck"));
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
                    "x": x,
                    "y": y,
                    "cx": cx,
                    "cy": cy
                }
            ]
        }));
        let turned = tmp(&format!("{tag}-rot"));
        let needle =
            format!("<a:xfrm><a:off x=\"{x}\" y=\"{y}\"/><a:ext cx=\"{cx}\" cy=\"{cy}\"/>");
        let repl = format!(
            "<a:xfrm rot=\"{rot}\"><a:off x=\"{x}\" y=\"{y}\"/><a:ext cx=\"{cx}\" cy=\"{cy}\"/>"
        );
        rewrite_slide(&deck, &turned, |xml| swap(xml, &needle, &repl));
        let out = tmp(&format!("{tag}-png")).with_extension("png");
        run_ok(json!({
            "schemaVersion": 1,
            "action": "exportSlideImage",
            "path": turned.to_string_lossy(),
            "id": "slide:1",
            "output": out.to_string_lossy()
        }));
        let (w, h, rgb) = decode_png(&std::fs::read(&out).unwrap());
        assert_eq!((w, h), (1280, 720));
        let cx_px = ((*x as f64 + *cx as f64 / 2.0) * 96.0 / 914400.0).round() as u32;
        let cy_px = ((*y as f64 + *cy as f64 / 2.0) * 96.0 / 914400.0).round() as u32;
        assert_eq!(
            pixel(&rgb, w, cx_px, cy_px),
            fill,
            "{tag} center ({cx_px},{cy_px})"
        );
        assert_eq!(pixel(&rgb, w, 10, 10), bg, "{tag} far background");
    }
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
    let turned_png = tmp("grp-rot-png").with_extension("png");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "exportSlideImage",
        "path": turned.to_string_lossy(),
        "id": "slide:1",
        "output": turned_png.to_string_lossy()
    }));
    let (w, _h, rgb) = decode_png(&std::fs::read(&turned_png).unwrap());
    // 组合绕中心顺时针转 90 度后，左块在上，右块在下，外接框是 48×192。
    assert_eq!(pixel(&rgb, w, 264, 24), [0xEE, 0x44, 0x55]);
    assert_eq!(pixel(&rgb, w, 311, 119), [0xEE, 0x44, 0x55]);
    assert_eq!(pixel(&rgb, w, 264, 120), [0x11, 0x22, 0x33]);
    assert_eq!(pixel(&rgb, w, 311, 215), [0x11, 0x22, 0x33]);
    assert_eq!(pixel(&rgb, w, 263, 24), [0xAA, 0xBB, 0xCC]);
    assert_eq!(pixel(&rgb, w, 264, 23), [0xAA, 0xBB, 0xCC]);
    assert_eq!(pixel(&rgb, w, 312, 24), [0xAA, 0xBB, 0xCC]);
    assert_eq!(pixel(&rgb, w, 264, 216), [0xAA, 0xBB, 0xCC]);
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

#[test]
fn theme_page_background_follows_the_measured_color() {
    let src = tmp("theme-src");
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
    let base = tmp("theme-base");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": base.to_string_lossy(),
        "ops": [{ "op": "deleteShape", "id": shape_id }]
    }));

    let accent = "<p:bg><p:bgPr><a:solidFill><a:schemeClr val=\"accent1\"/></a:solidFill><a:effectLst/></p:bgPr></p:bg>";
    let custom = tmp("theme-custom");
    rewrite_named(&base, &custom, |name, xml| match name {
        "ppt/slides/slide1.xml" => swap(xml, "<p:cSld>", &format!("<p:cSld>{accent}")),
        "ppt/theme/theme1.xml" => swap(
            xml,
            "<a:accent1><a:srgbClr val=\"4F81BD\"/></a:accent1>",
            "<a:accent1><a:srgbClr val=\"EE4455\"/></a:accent1>",
        ),
        _ => xml,
    });
    assert_page(&custom, [0xEE, 0x44, 0x55]);

    let bg1 = tmp("theme-bg1");
    rewrite_named(&base, &bg1, |name, xml| {
        match name {
        "ppt/slides/slide1.xml" => swap(
            xml,
            "<p:cSld>",
            "<p:cSld><p:bg><p:bgPr><a:solidFill><a:schemeClr val=\"bg1\"/></a:solidFill><a:effectLst/></p:bgPr></p:bg>",
        ),
        _ => xml,
    }
    });
    assert_page(&bg1, [255, 255, 255]);

    let reference = tmp("theme-ref");
    rewrite_named(&base, &reference, |name, xml| match name {
        "ppt/slides/slide1.xml" => swap(
            xml,
            "<p:cSld>",
            "<p:cSld><p:bg><p:bgRef idx=\"1001\"><a:schemeClr val=\"accent1\"/></p:bgRef></p:bg>",
        ),
        _ => xml,
    });
    assert_page(&reference, [0x4F, 0x81, 0xBD]);

    let reference_rgb = tmp("theme-ref-rgb");
    rewrite_named(&base, &reference_rgb, |name, xml| match name {
        "ppt/slides/slide1.xml" => swap(
            xml,
            "<p:cSld>",
            "<p:cSld><p:bg><p:bgRef idx=\"1001\"><a:srgbClr val=\"112233\"/></p:bgRef></p:bg>",
        ),
        _ => xml,
    });
    assert_page(&reference_rgb, [0x11, 0x22, 0x33]);

    let inherited = tmp("theme-inherit");
    rewrite_named(&base, &inherited, |name, xml| match name {
        "ppt/slideMasters/slideMaster1.xml" => swap(
            xml,
            "<p:bgRef idx=\"1001\"><a:schemeClr val=\"bg1\"/></p:bgRef>",
            "<p:bgRef idx=\"1001\"><a:schemeClr val=\"accent1\"/></p:bgRef>",
        ),
        _ => xml,
    });
    assert_page(&inherited, [0x4F, 0x81, 0xBD]);

    let hidden = tmp("theme-hidden");
    rewrite_named(&base, &hidden, |name, xml| match name {
        "ppt/slides/slide1.xml" => swap(xml, "<p:cSld>", "<p:cSld showMasterSp=\"0\">"),
        "ppt/slideMasters/slideMaster1.xml" => swap(
            xml,
            "<p:bgRef idx=\"1001\"><a:schemeClr val=\"bg1\"/></p:bgRef>",
            "<p:bgRef idx=\"1001\"><a:schemeClr val=\"accent1\"/></p:bgRef>",
        ),
        _ => xml,
    });
    assert_page(&hidden, [0x4F, 0x81, 0xBD]);

    let layout = tmp("theme-layout");
    rewrite_named(&base, &layout, |name, xml| {
        match name {
        "ppt/slideLayouts/slideLayout1.xml" => swap(
            xml,
            "<p:cSld name=\"空白\">",
            "<p:cSld name=\"空白\"><p:bg><p:bgPr><a:solidFill><a:schemeClr val=\"accent2\"/></a:solidFill><a:effectLst/></p:bgPr></p:bg>",
        ),
        _ => xml,
    }
    });
    assert_page(&layout, [0xC0, 0x50, 0x4D]);

    let remap = "<p:clrMapOvr><a:overrideClrMapping bg1=\"accent2\" tx1=\"dk1\" bg2=\"lt2\" tx2=\"dk2\" accent1=\"accent1\" accent2=\"accent2\" accent3=\"accent3\" accent4=\"accent4\" accent5=\"accent5\" accent6=\"accent6\" hlink=\"hlink\" folHlink=\"folHlink\"/></p:clrMapOvr>";
    let slide_map = tmp("theme-remap");
    rewrite_named(&base, &slide_map, |name, xml| match name {
        "ppt/slides/slide1.xml" => {
            let xml = swap(
                xml,
                "<p:cSld>",
                "<p:cSld><p:bg><p:bgPr><a:solidFill><a:schemeClr val=\"bg1\"/></a:solidFill><a:effectLst/></p:bgPr></p:bg>",
            );
            swap(
                xml,
                "<p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr>",
                remap,
            )
        }
        _ => xml,
    });
    assert_page(&slide_map, [0xC0, 0x50, 0x4D]);

    let layout_map = tmp("theme-layout-map");
    rewrite_named(&base, &layout_map, |name, xml| match name {
        "ppt/slideLayouts/slideLayout1.xml" => {
            let xml = swap(
                xml,
                "<p:cSld name=\"空白\">",
                "<p:cSld name=\"空白\"><p:bg><p:bgPr><a:solidFill><a:schemeClr val=\"bg1\"/></a:solidFill><a:effectLst/></p:bgPr></p:bg>",
            );
            swap(
                xml,
                "<p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr>",
                remap,
            )
        }
        _ => xml,
    });
    assert_page(&layout_map, [0xC0, 0x50, 0x4D]);

    let inherit_map = remap.replace("bg1=\"accent2\"", "bg1=\"accent1\"");
    let inherited_map = tmp("theme-inherit-map");
    rewrite_named(&base, &inherited_map, |name, xml| match name {
        "ppt/slides/slide1.xml" => swap(
            xml,
            "<p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr>",
            &inherit_map,
        ),
        _ => xml,
    });
    assert_page(&inherited_map, [0x4F, 0x81, 0xBD]);

    let empty_ref = tmp("theme-idx0");
    rewrite_named(&base, &empty_ref, |name, xml| match name {
        "ppt/slides/slide1.xml" => swap(
            xml,
            "<p:cSld>",
            "<p:cSld><p:bg><p:bgRef idx=\"0\"><a:schemeClr val=\"accent1\"/></p:bgRef></p:bg>",
        ),
        "ppt/slideMasters/slideMaster1.xml" => swap(
            xml,
            "<p:bgRef idx=\"1001\"><a:schemeClr val=\"bg1\"/></p:bgRef>",
            "<p:bgRef idx=\"1001\"><a:schemeClr val=\"accent1\"/></p:bgRef>",
        ),
        _ => xml,
    });
    assert_page(&empty_ref, [255, 255, 255]);

    let gradient = tmp("theme-grad");
    rewrite_named(&base, &gradient, |name, xml| match name {
        "ppt/slides/slide1.xml" => swap(
            xml,
            "<p:cSld>",
            "<p:cSld><p:bg><p:bgRef idx=\"1002\"><a:schemeClr val=\"accent1\"/></p:bgRef></p:bg>",
        ),
        _ => xml,
    });
    assert_page(&gradient, [255, 255, 255]);

    let tinted = tmp("theme-tint");
    rewrite_named(&base, &tinted, |name, xml| {
        match name {
        "ppt/slides/slide1.xml" => swap(
            xml,
            "<p:cSld>",
            "<p:cSld><p:bg><p:bgPr><a:solidFill><a:schemeClr val=\"accent1\"><a:lumMod val=\"50000\"/></a:schemeClr></a:solidFill><a:effectLst/></p:bgPr></p:bg>",
        ),
        _ => xml,
    }
    });
    assert_page(&tinted, [255, 255, 255]);

    let opaque = tmp("theme-alpha");
    rewrite_named(&base, &opaque, |name, xml| {
        match name {
        "ppt/slides/slide1.xml" => swap(
            xml,
            "<p:cSld>",
            "<p:cSld><p:bg><p:bgPr><a:solidFill><a:schemeClr val=\"accent1\"><a:alpha val=\"100000\"/></a:schemeClr></a:solidFill><a:effectLst/></p:bgPr></p:bg>",
        ),
        _ => xml,
    }
    });
    assert_page(&opaque, [0x4F, 0x81, 0xBD]);

    let wins = tmp("theme-wins");
    rewrite_named(&base, &wins, |name, xml| {
        match name {
        "ppt/slides/slide1.xml" => swap(
            xml,
            "<p:cSld>",
            "<p:cSld><p:bg><p:bgPr><a:solidFill><a:srgbClr val=\"FF0000\"/></a:solidFill><a:effectLst/></p:bgPr></p:bg>",
        ),
        "ppt/slideMasters/slideMaster1.xml" => swap(
            xml,
            "<p:bgRef idx=\"1001\"><a:schemeClr val=\"bg1\"/></p:bgRef>",
            "<p:bgRef idx=\"1001\"><a:schemeClr val=\"accent1\"/></p:bgRef>",
        ),
        _ => xml,
    }
    });
    assert_page(&wins, [255, 0, 0]);
}

#[test]
fn shape_blip_fill_is_painted_and_a_tile_fails() {
    let src = tmp("blip-src");
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
    std::fs::write(&png_path, solid_png(8, 8, [0x22, 0x88, 0x44])).unwrap();
    let deck = tmp("blip-deck");
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
                "fill": { "type": "blip", "file": png_path.to_string_lossy() },
                "x": 1828800,
                "y": 914400,
                "cx": 914400,
                "cy": 914400
            }
        ]
    }));
    let out = tmp("blip-png").with_extension("png");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "exportSlideImage",
        "path": deck.to_string_lossy(),
        "id": "slide:1",
        "output": out.to_string_lossy()
    }));
    let (w, _h, rgb) = decode_png(&std::fs::read(&out).unwrap());
    assert_eq!(pixel(&rgb, w, 192, 96), [0x22, 0x88, 0x44]);
    assert_eq!(pixel(&rgb, w, 239, 143), [0x22, 0x88, 0x44]);
    assert_eq!(pixel(&rgb, w, 191, 96), [0xAA, 0xBB, 0xCC]);
    assert_eq!(pixel(&rgb, w, 288, 96), [0xAA, 0xBB, 0xCC]);

    let tiled = tmp("blip-tile");
    rewrite_slide(&deck, &tiled, |xml| {
        swap(xml, "<a:stretch><a:fillRect/></a:stretch>", "<a:tile/>")
    });
    let err = run(&json!({
        "schemaVersion": 1,
        "action": "exportSlideImage",
        "path": tiled.to_string_lossy(),
        "id": "slide:1",
        "output": tmp("blip-tile-png").with_extension("png").to_string_lossy()
    })
    .to_string())
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::NotImplemented);
}

#[test]
fn translucent_two_stop_gradient_blends_over_the_page() {
    let src = tmp("alpha-grad-src");
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
    let deck = tmp("alpha-grad-deck");
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
                "fill": {
                    "type": "grad",
                    "ang": 0,
                    "alpha": 50000,
                    "stops": [
                        { "pos": 0, "color": "FF0000" },
                        { "pos": 100000, "color": "0000FF" }
                    ]
                },
                "x": 1828800,
                "y": 914400,
                "cx": 914400,
                "cy": 457200
            }
        ]
    }));
    let out = tmp("alpha-grad-png").with_extension("png");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "exportSlideImage",
        "path": deck.to_string_lossy(),
        "id": "slide:1",
        "output": out.to_string_lossy()
    }));
    let (w, _h, rgb) = decode_png(&std::fs::read(&out).unwrap());
    let left = pixel(&rgb, w, 192, 120);
    let right = pixel(&rgb, w, 287, 120);
    let mid = pixel(&rgb, w, 240, 120);
    // 与 Office 16.0 金样端点、中点一致：sRGB 线性插值后再按 128/255 浮点叠在页底上。
    assert_eq!(left, [212, 93, 102], "左端 {left:?}");
    assert_eq!(right, [85, 93, 229], "右端 {right:?}");
    assert_eq!(mid, [148, 93, 166], "中点 {mid:?}");
    assert_ne!(left, [0xAA, 0xBB, 0xCC]);
    assert_ne!(right, [0xAA, 0xBB, 0xCC]);
    assert_ne!(left, [255, 0, 0]);
    assert_ne!(right, [0, 0, 255]);
    assert!(left[0] > left[2], "左端应更红 {left:?}");
    assert!(right[2] > right[0], "右端应更蓝 {right:?}");
}

#[test]
fn round_rect_and_ellipse_gradients_are_painted() {
    let src = tmp("shape-grad-src");
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
    let deck = tmp("shape-grad-deck");
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
                "prst": "ellipse",
                "line": { "color": null },
                "fill": {
                    "type": "grad",
                    "ang": 0,
                    "stops": [
                        { "pos": 0, "color": "FF0000" },
                        { "pos": 100000, "color": "0000FF" }
                    ]
                },
                "x": 914400,
                "y": 914400,
                "cx": 1828800,
                "cy": 1828800
            },
            {
                "op": "insertShape",
                "id": "slide:1",
                "prst": "roundRect",
                "line": { "color": null },
                "fill": {
                    "type": "grad",
                    "ang": 0,
                    "stops": [
                        { "pos": 0, "color": "00AA00" },
                        { "pos": 100000, "color": "AA00AA" }
                    ]
                },
                "x": 3657600,
                "y": 914400,
                "cx": 1828800,
                "cy": 914400
            }
        ]
    }));
    let out = tmp("shape-grad-png").with_extension("png");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "exportSlideImage",
        "path": deck.to_string_lossy(),
        "id": "slide:1",
        "output": out.to_string_lossy()
    }));
    let (w, _h, rgb) = decode_png(&std::fs::read(&out).unwrap());
    // 椭圆外接矩形 96,96 到 288,288。圆心约 192,192。四角在椭圆外。
    let oval_center = pixel(&rgb, w, 192, 192);
    let oval_left = pixel(&rgb, w, 110, 192);
    let oval_right = pixel(&rgb, w, 274, 192);
    assert_ne!(oval_center, [0xAA, 0xBB, 0xCC]);
    assert_ne!(oval_left, oval_right);
    assert_ne!(oval_left, [255, 0, 0]);
    assert!(oval_left[0] > oval_left[2], "椭圆左端应更红 {oval_left:?}");
    assert!(
        oval_right[2] > oval_right[0],
        "椭圆右端应更蓝 {oval_right:?}"
    );
    assert_eq!(pixel(&rgb, w, 96, 96), [0xAA, 0xBB, 0xCC]);
    // 圆角矩形 384,96 到 576,192。中心 480,144。左上圆角外侧仍是页底。
    let round_center = pixel(&rgb, w, 480, 144);
    let round_left = pixel(&rgb, w, 400, 144);
    let round_right = pixel(&rgb, w, 560, 144);
    assert_ne!(round_center, [0xAA, 0xBB, 0xCC]);
    assert_ne!(round_left, round_right);
    assert_ne!(round_center, [0x00, 0xAA, 0x00]);
    assert_eq!(pixel(&rgb, w, 384, 96), [0xAA, 0xBB, 0xCC]);
}

#[test]
fn shape_blip_fill_is_written_before_line() {
    let png_path = tmp("blip-order").with_extension("png");
    std::fs::write(&png_path, solid_png(8, 8, [0x22, 0x88, 0x44])).unwrap();

    let created = tmp("blip-order-create");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": created.to_string_lossy(),
        "presentation": { "slides": [{ "shapes": [{
            "text": "",
            "fill": { "type": "blip", "file": png_path.to_string_lossy() },
            "x": 1828800,
            "y": 914400,
            "cx": 914400,
            "cy": 914400
        }] }] }
    }));
    let created_xml = slide1_xml(&created);
    assert!(
        created_xml.contains("</a:blipFill><a:ln>"),
        "create 的 blipFill 必须写在 ln 之前，否则 Office 16.0 不画图片填充：{created_xml}"
    );
    assert!(
        !created_xml.contains("</a:ln><a:blipFill>"),
        "{created_xml}"
    );

    let src = tmp("blip-order-src");
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
    let inserted = tmp("blip-order-insert");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": inserted.to_string_lossy(),
        "ops": [
            { "op": "deleteShape", "id": shape_id },
            {
                "op": "insertShape",
                "id": "slide:1",
                "fill": { "type": "blip", "file": png_path.to_string_lossy() },
                "x": 1828800,
                "y": 914400,
                "cx": 914400,
                "cy": 914400
            }
        ]
    }));
    let inserted_xml = slide1_xml(&inserted);
    assert!(
        inserted_xml.contains("</a:blipFill><a:ln>"),
        "insertShape 的 blipFill 必须写在 ln 之前：{inserted_xml}"
    );
    assert!(
        !inserted_xml.contains("</a:ln><a:blipFill>"),
        "{inserted_xml}"
    );
}

#[test]
fn axis_aligned_connectors_paint_and_a_diagonal_paints() {
    let src = tmp("line-src");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": src.to_string_lossy(),
        "presentation": {
            "slides": [
                { "connectors": [{ "prst": "straightConnector1", "x": 914400, "y": 914400, "cx": 1828800, "cy": 762000 }] },
                { "connectors": [{ "prst": "line", "x": 914400, "y": 914400, "cx": 1828800, "cy": 762000 }] }
            ]
        }
    }));

    let two_px_h = tmp("line-2h");
    rewrite_slide(&src, &two_px_h, |xml| {
        let xml = swap(xml, "cy=\"762000\"", "cy=\"0\"");
        swap(xml, "w=\"12700\"", "w=\"19050\"")
    });
    let out = tmp("line-2h-png").with_extension("png");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "exportSlideImage",
        "path": two_px_h.to_string_lossy(),
        "id": "slide:1",
        "output": out.to_string_lossy()
    }));
    let (w, h, rgb) = decode_png(&std::fs::read(&out).unwrap());
    assert_eq!((w, h), (1280, 720));
    assert_eq!(pixel(&rgb, w, 96, 95), [0, 0, 0]);
    assert_eq!(pixel(&rgb, w, 96, 96), [0, 0, 0]);
    assert_eq!(pixel(&rgb, w, 287, 96), [0, 0, 0]);
    assert_eq!(pixel(&rgb, w, 288, 96), [255, 255, 255]);
    assert_eq!(pixel(&rgb, w, 96, 97), [255, 255, 255]);
    assert_eq!(pixel(&rgb, w, 95, 96), [255, 255, 255]);

    let two_px_v = tmp("line-2v");
    rewrite_named(&src, &two_px_v, |name, xml| {
        if name != "ppt/slides/slide1.xml" {
            return xml;
        }
        let xml = swap(xml, "cx=\"1828800\"", "cx=\"0\"");
        let xml = swap(xml, "cy=\"762000\"", "cy=\"1828800\"");
        swap(xml, "w=\"12700\"", "w=\"19050\"")
    });
    let out = tmp("line-2v-png").with_extension("png");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "exportSlideImage",
        "path": two_px_v.to_string_lossy(),
        "id": "slide:1",
        "output": out.to_string_lossy()
    }));
    let (w, h, rgb) = decode_png(&std::fs::read(&out).unwrap());
    assert_eq!((w, h), (1280, 720));
    assert_eq!(pixel(&rgb, w, 95, 96), [0, 0, 0]);
    assert_eq!(pixel(&rgb, w, 96, 96), [0, 0, 0]);
    assert_eq!(pixel(&rgb, w, 96, 287), [0, 0, 0]);
    assert_eq!(pixel(&rgb, w, 96, 288), [255, 255, 255]);
    assert_eq!(pixel(&rgb, w, 97, 96), [255, 255, 255]);

    let one_px_h = tmp("line-1h");
    rewrite_slide(&src, &one_px_h, |xml| {
        let xml = swap(xml, "cy=\"762000\"", "cy=\"0\"");
        swap(xml, "w=\"12700\"", "w=\"9525\"")
    });
    let out = tmp("line-1h-png").with_extension("png");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "exportSlideImage",
        "path": one_px_h.to_string_lossy(),
        "id": "slide:1",
        "output": out.to_string_lossy()
    }));
    let (w, h, rgb) = decode_png(&std::fs::read(&out).unwrap());
    assert_eq!((w, h), (1280, 720));
    assert_eq!(pixel(&rgb, w, 96, 96), [64, 64, 64]);
    assert_eq!(pixel(&rgb, w, 100, 96), [0, 0, 0]);
    assert_eq!(pixel(&rgb, w, 288, 96), [64, 64, 64]);
    assert_eq!(pixel(&rgb, w, 96, 95), [255, 255, 255]);

    let one_px_v = tmp("line-1v");
    rewrite_named(&src, &one_px_v, |name, xml| {
        if name != "ppt/slides/slide1.xml" {
            return xml;
        }
        let xml = swap(xml, "cx=\"1828800\"", "cx=\"0\"");
        let xml = swap(xml, "cy=\"762000\"", "cy=\"1828800\"");
        swap(xml, "w=\"12700\"", "w=\"9525\"")
    });
    let out = tmp("line-1v-png").with_extension("png");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "exportSlideImage",
        "path": one_px_v.to_string_lossy(),
        "id": "slide:1",
        "output": out.to_string_lossy()
    }));
    let (w, h, rgb) = decode_png(&std::fs::read(&out).unwrap());
    assert_eq!((w, h), (1280, 720));
    assert_eq!(pixel(&rgb, w, 96, 96), [96, 96, 96]);
    assert_eq!(pixel(&rgb, w, 96, 100), [0, 0, 0]);
    assert_eq!(pixel(&rgb, w, 96, 288), [32, 32, 32]);
    assert_eq!(pixel(&rgb, w, 95, 96), [255, 255, 255]);
    assert_eq!(pixel(&rgb, w, 97, 96), [255, 255, 255]);

    let out = tmp("line-diag-png").with_extension("png");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "exportSlideImage",
        "path": src.to_string_lossy(),
        "id": "slide:1",
        "output": out.to_string_lossy()
    }));
    let (w, h, rgb) = decode_png(&std::fs::read(&out).unwrap());
    assert_eq!((w, h), (1280, 720));
    assert_eq!(pixel(&rgb, w, 96, 96), [96, 96, 96]);
    assert_eq!(pixel(&rgb, w, 288, 176), [64, 64, 64]);
    assert_eq!(pixel(&rgb, w, 95, 96), [255, 255, 255]);
}

#[test]
fn bent_connectors_paint_l_and_z() {
    let src = tmp("elbow-src");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": src.to_string_lossy(),
        "presentation": {
            "slides": [
                { "connectors": [{ "prst": "bentConnector2", "x": 914400, "y": 914400, "cx": 1828800, "cy": 1828800 }] },
                { "connectors": [{ "prst": "bentConnector3", "x": 914400, "y": 914400, "cx": 1828800, "cy": 1828800 }] }
            ]
        }
    }));

    let b2 = tmp("elbow-b2");
    rewrite_named(&src, &b2, |name, xml| {
        if name != "ppt/slides/slide1.xml" {
            return xml;
        }
        swap(xml, "w=\"12700\"", "w=\"9525\"")
    });
    let out = tmp("elbow-b2-png").with_extension("png");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "exportSlideImage",
        "path": b2.to_string_lossy(),
        "id": "slide:1",
        "output": out.to_string_lossy()
    }));
    let (w, h, rgb) = decode_png(&std::fs::read(&out).unwrap());
    assert_eq!((w, h), (1280, 720));
    assert_eq!(pixel(&rgb, w, 96, 96), [64, 64, 64]);
    assert_eq!(pixel(&rgb, w, 288, 96), [48, 48, 48]);
    assert_eq!(pixel(&rgb, w, 288, 288), [32, 32, 32]);
    assert_eq!(pixel(&rgb, w, 200, 96), [0, 0, 0]);

    let b2v = tmp("elbow-b2v");
    rewrite_named(&src, &b2v, |name, xml| {
        if name != "ppt/slides/slide1.xml" {
            return xml;
        }
        let xml = swap(
            xml,
            "<a:xfrm><a:off x=\"914400\"",
            "<a:xfrm flipV=\"1\"><a:off x=\"914400\"",
        );
        swap(xml, "w=\"12700\"", "w=\"9525\"")
    });
    let out = tmp("elbow-b2v-png").with_extension("png");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "exportSlideImage",
        "path": b2v.to_string_lossy(),
        "id": "slide:1",
        "output": out.to_string_lossy()
    }));
    let (w, h, rgb) = decode_png(&std::fs::read(&out).unwrap());
    assert_eq!((w, h), (1280, 720));
    assert_eq!(pixel(&rgb, w, 96, 288), [64, 64, 64]);
    assert_eq!(pixel(&rgb, w, 288, 288), [16, 16, 16]);
    assert_eq!(pixel(&rgb, w, 288, 96), [96, 96, 96]);

    let b3 = tmp("elbow-b3");
    rewrite_named(&src, &b3, |name, xml| {
        if name != "ppt/slides/slide2.xml" {
            return xml;
        }
        let xml = swap(
            xml,
            "prst=\"bentConnector3\"><a:avLst/>",
            "prst=\"bentConnector3\"><a:avLst><a:gd name=\"adj1\" fmla=\"val 25000\"/></a:avLst>",
        );
        swap(xml, "w=\"12700\"", "w=\"9525\"")
    });
    let out = tmp("elbow-b3-png").with_extension("png");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "exportSlideImage",
        "path": b3.to_string_lossy(),
        "id": "slide:2",
        "output": out.to_string_lossy()
    }));
    let (w, h, rgb) = decode_png(&std::fs::read(&out).unwrap());
    assert_eq!((w, h), (1280, 720));
    assert_eq!(pixel(&rgb, w, 96, 96), [64, 64, 64]);
    assert_eq!(pixel(&rgb, w, 144, 96), [48, 48, 48]);
    assert_eq!(pixel(&rgb, w, 144, 288), [16, 16, 16]);
    assert_eq!(pixel(&rgb, w, 288, 288), [64, 64, 64]);

    let b32 = tmp("elbow-b32");
    rewrite_named(&src, &b32, |name, xml| {
        if name != "ppt/slides/slide1.xml" {
            return xml;
        }
        swap(xml, "w=\"12700\"", "w=\"19050\"")
    });
    let out = tmp("elbow-b32-png").with_extension("png");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "exportSlideImage",
        "path": b32.to_string_lossy(),
        "id": "slide:1",
        "output": out.to_string_lossy()
    }));
    let (w, h, rgb) = decode_png(&std::fs::read(&out).unwrap());
    assert_eq!((w, h), (1280, 720));
    assert_eq!(pixel(&rgb, w, 96, 96), [0, 0, 0]);
    assert_eq!(pixel(&rgb, w, 96, 95), [0, 0, 0]);
    assert_eq!(pixel(&rgb, w, 288, 95), [96, 96, 96]);
    assert_eq!(pixel(&rgb, w, 288, 288), [255, 255, 255]);
}

#[test]
fn curved_connectors_paint_c2_and_c3() {
    let src = tmp("curve-src");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": src.to_string_lossy(),
        "presentation": {
            "slides": [
                { "connectors": [{ "prst": "curvedConnector2", "x": 914400, "y": 914400, "cx": 1828800, "cy": 1828800 }] },
                { "connectors": [{ "prst": "curvedConnector3", "x": 914400, "y": 914400, "cx": 1828800, "cy": 1828800 }] }
            ]
        }
    }));

    let c2 = tmp("curve-c2");
    rewrite_named(&src, &c2, |name, xml| {
        if name != "ppt/slides/slide1.xml" {
            return xml;
        }
        swap(xml, "w=\"12700\"", "w=\"9525\"")
    });
    let out = tmp("curve-c2-png").with_extension("png");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "exportSlideImage",
        "path": c2.to_string_lossy(),
        "id": "slide:1",
        "output": out.to_string_lossy()
    }));
    let (w, h, rgb) = decode_png(&std::fs::read(&out).unwrap());
    assert_eq!((w, h), (1280, 720));
    assert_eq!(pixel(&rgb, w, 96, 96), [96, 96, 96]);
    assert_eq!(pixel(&rgb, w, 97, 96), [64, 64, 64]);
    assert_eq!(pixel(&rgb, w, 192, 96), [255, 255, 255]);
    assert_eq!(pixel(&rgb, w, 205, 136), [64, 64, 64]);
    assert_eq!(pixel(&rgb, w, 288, 288), [32, 32, 32]);

    let c2v = tmp("curve-c2v");
    rewrite_named(&src, &c2v, |name, xml| {
        if name != "ppt/slides/slide1.xml" {
            return xml;
        }
        let xml = swap(
            xml,
            "<a:xfrm><a:off x=\"914400\"",
            "<a:xfrm flipV=\"1\"><a:off x=\"914400\"",
        );
        swap(xml, "w=\"12700\"", "w=\"9525\"")
    });
    let out = tmp("curve-c2v-png").with_extension("png");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "exportSlideImage",
        "path": c2v.to_string_lossy(),
        "id": "slide:1",
        "output": out.to_string_lossy()
    }));
    let (w, h, rgb) = decode_png(&std::fs::read(&out).unwrap());
    assert_eq!((w, h), (1280, 720));
    assert_eq!(pixel(&rgb, w, 96, 288), [64, 64, 64]);
    assert_eq!(pixel(&rgb, w, 288, 96), [96, 96, 96]);
    assert_eq!(pixel(&rgb, w, 288, 97), [0, 0, 0]);

    let c3 = tmp("curve-c3");
    rewrite_named(&src, &c3, |name, xml| {
        if name != "ppt/slides/slide2.xml" {
            return xml;
        }
        let xml = swap(
            xml,
            "prst=\"curvedConnector3\"><a:avLst/>",
            "prst=\"curvedConnector3\"><a:avLst><a:gd name=\"adj1\" fmla=\"val 30000\"/></a:avLst>",
        );
        swap(xml, "w=\"12700\"", "w=\"9525\"")
    });
    let out = tmp("curve-c3-png").with_extension("png");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "exportSlideImage",
        "path": c3.to_string_lossy(),
        "id": "slide:2",
        "output": out.to_string_lossy()
    }));
    let (w, h, rgb) = decode_png(&std::fs::read(&out).unwrap());
    assert_eq!((w, h), (1280, 720));
    assert_eq!(pixel(&rgb, w, 96, 96), [96, 96, 96]);
    assert_eq!(pixel(&rgb, w, 153, 192), [167, 167, 167]);
    assert_eq!(pixel(&rgb, w, 154, 192), [88, 88, 88]);
    assert_eq!(pixel(&rgb, w, 288, 288), [64, 64, 64]);

    let c32 = tmp("curve-c32");
    rewrite_named(&src, &c32, |name, xml| {
        if name != "ppt/slides/slide1.xml" {
            return xml;
        }
        swap(xml, "w=\"12700\"", "w=\"19050\"")
    });
    let out = tmp("curve-c32-png").with_extension("png");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "exportSlideImage",
        "path": c32.to_string_lossy(),
        "id": "slide:1",
        "output": out.to_string_lossy()
    }));
    let (w, h, rgb) = decode_png(&std::fs::read(&out).unwrap());
    assert_eq!((w, h), (1280, 720));
    assert_eq!(pixel(&rgb, w, 96, 96), [32, 32, 32]);
    assert_eq!(pixel(&rgb, w, 96, 95), [88, 88, 88]);
    assert_eq!(pixel(&rgb, w, 234, 163), [0, 0, 0]);
    assert_eq!(pixel(&rgb, w, 288, 288), [191, 191, 191]);
}

fn slide1_xml(path: &Path) -> String {
    let bytes = std::fs::read(path).unwrap();
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut file = zip.by_name("ppt/slides/slide1.xml").unwrap();
    let mut xml = String::new();
    file.read_to_string(&mut xml).unwrap();
    xml
}

#[test]
fn shape_text_newline_keeps_the_second_line() {
    let src = tmp("multiline-src");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": src.to_string_lossy(),
        "presentation": {
            "slides": [{
                "shapes": [{
                    "text": "AAAA\nBBBB",
                    "x": 914400,
                    "y": 914400,
                    "cx": 3657600,
                    "cy": 2743200
                }]
            }]
        }
    }));
    let bytes = std::fs::read(&src).unwrap();
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut file = zip.by_name("ppt/slides/slide1.xml").unwrap();
    let mut xml = String::new();
    file.read_to_string(&mut xml).unwrap();
    assert!(xml.contains("<a:t>AAAA</a:t>"), "{xml}");
    assert!(xml.contains("<a:br/>"), "{xml}");
    assert!(xml.contains("<a:t>BBBB</a:t>"), "{xml}");
    let out = tmp("multiline-png").with_extension("png");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "exportSlideImage",
        "path": src.to_string_lossy(),
        "id": "slide:1",
        "output": out.to_string_lossy()
    }));
    let (_w, _h, rgb) = decode_png(&std::fs::read(&out).unwrap());
    let _ = rgb;
}

fn assert_page(path: &Path, color: [u8; 3]) {
    let out = tmp("theme-png").with_extension("png");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "exportSlideImage",
        "path": path.to_string_lossy(),
        "id": "slide:1",
        "output": out.to_string_lossy()
    }));
    let (w, h, rgb) = decode_png(&std::fs::read(&out).unwrap());
    assert_eq!((w, h), (1280, 720));
    assert_eq!(pixel(&rgb, w, 0, 0), color);
    assert_eq!(pixel(&rgb, w, 640, 360), color);
    assert_eq!(pixel(&rgb, w, w - 1, h - 1), color);
}

fn swap(xml: String, from: &str, to: &str) -> String {
    assert!(xml.contains(from), "缺少 {from}");
    xml.replacen(from, to, 1)
}

fn rewrite_slide(src: &Path, dst: &Path, edit: impl FnOnce(String) -> String) {
    let mut edit = Some(edit);
    rewrite_named(src, dst, |name, xml| {
        if name == "ppt/slides/slide1.xml" {
            edit.take().unwrap()(xml)
        } else {
            xml
        }
    });
}

fn rewrite_named(src: &Path, dst: &Path, mut edit: impl FnMut(&str, String) -> String) {
    let bytes = std::fs::read(src).unwrap();
    let mut zip_in = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut out = Cursor::new(Vec::new());
    {
        let mut zip_out = zip::ZipWriter::new(&mut out);
        let opt = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        for i in 0..zip_in.len() {
            let mut file = zip_in.by_index(i).unwrap();
            let name = file.name().to_string();
            let mut data = Vec::new();
            file.read_to_end(&mut data).unwrap();
            if name.ends_with(".xml") {
                let xml = String::from_utf8(data).unwrap();
                data = edit(&name, xml).into_bytes();
            }
            zip_out.start_file(&name, opt).unwrap();
            zip_out.write_all(&data).unwrap();
        }
        zip_out.finish().unwrap();
    }
    std::fs::write(dst, out.into_inner()).unwrap();
}

fn rgba_png(w: u32, h: u32, px: &[[u8; 4]]) -> Vec<u8> {
    assert_eq!(px.len(), (w as usize) * (h as usize));
    let mut raw = Vec::with_capacity(px.len() * 4);
    for p in px {
        raw.extend_from_slice(p);
    }
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, w, h);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(&raw)
        .unwrap();
    out
}

fn pattern_2x2() -> Vec<[u8; 4]> {
    vec![
        [255, 0, 0, 255],
        [0, 255, 0, 255],
        [0, 0, 255, 255],
        [255, 255, 0, 255],
    ]
}

fn export_picture_slide(
    png: &[u8],
    x: i64,
    y: i64,
    cx: i64,
    cy: i64,
    bg: &str,
    tag: &str,
) -> (u32, u32, Vec<u8>) {
    let src = tmp(tag);
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
    std::fs::write(&png_path, png).unwrap();
    let deck = tmp(&format!("{tag}-deck"));
    run_ok(json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": deck.to_string_lossy(),
        "ops": [
            { "op": "setPageBackground", "id": "slide:1", "color": bg },
            { "op": "deleteShape", "id": title },
            {
                "op": "insertPicture",
                "id": "slide:1",
                "file": png_path.to_string_lossy(),
                "x": x,
                "y": y,
                "cx": cx,
                "cy": cy
            }
        ]
    }));
    let out = tmp(&format!("{tag}-out")).with_extension("png");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "exportSlideImage",
        "path": deck.to_string_lossy(),
        "id": "slide:1",
        "output": out.to_string_lossy()
    }));
    decode_png(&std::fs::read(&out).unwrap())
}

fn rgb_at(hex: u32) -> [u8; 3] {
    [
        ((hex >> 16) & 0xff) as u8,
        ((hex >> 8) & 0xff) as u8,
        (hex & 0xff) as u8,
    ]
}

fn assert_grid(rgb: &[u8], w: u32, x0: u32, y0: u32, grid: &[&[u32]]) {
    for (row, line) in grid.iter().enumerate() {
        for (col, color) in line.iter().enumerate() {
            assert_eq!(
                pixel(rgb, w, x0 + col as u32, y0 + row as u32),
                rgb_at(*color),
                "px ({}, {})",
                x0 + col as u32,
                y0 + row as u32
            );
        }
    }
}

fn assert_box_outside(rgb: &[u8], w: u32, x0: u32, y0: u32, bw: u32, bh: u32, bg: [u8; 3]) {
    if x0 > 0 {
        for row in 0..bh {
            assert_eq!(pixel(rgb, w, x0 - 1, y0 + row), bg, "left of box");
        }
    }
    if y0 > 0 {
        for col in 0..bw {
            assert_eq!(pixel(rgb, w, x0 + col, y0 - 1), bg, "above box");
        }
    }
    for row in 0..bh {
        assert_eq!(pixel(rgb, w, x0 + bw, y0 + row), bg, "right of box");
    }
    for col in 0..bw {
        assert_eq!(pixel(rgb, w, x0 + col, y0 + bh), bg, "below box");
    }
}

#[cfg(windows)]
#[test]
fn patterned_png_scale_up_matches_office_on_windows() {
    let png = rgba_png(2, 2, &pattern_2x2());
    let bg = [0xAA, 0xBB, 0xCC];
    let x = 1828800i64;
    let y = 914400i64;
    // 2x2 -> 4x4
    let (w, _, rgb) = export_picture_slide(&png, x, y, 38100, 38100, "AABBCC", "scale-2to4");
    assert_eq!(w, 1280);
    assert_grid(
        &rgb,
        w,
        192,
        96,
        &[
            &[0xFF0000, 0xCD4000, 0x34BD00, 0x00FF00],
            &[0xBF0040, 0x9F4030, 0x61BD11, 0x40FF00],
            &[0x4200BD, 0x61408E, 0x9DBD31, 0xBDFF00],
            &[0x0000FF, 0x3240D4, 0xCABD49, 0xFFFF00],
        ],
    );
    assert_box_outside(&rgb, w, 192, 96, 4, 4, bg);

    // 2x2 -> 5x5 非整数倍率
    let (w, _, rgb) = export_picture_slide(&png, x, y, 47625, 47625, "AABBCC", "scale-2to5");
    assert_grid(
        &rgb,
        w,
        192,
        96,
        &[
            &[0xFF0000, 0xFA1E00, 0x807F00, 0x05E100, 0x00FF00],
            &[0xE1001E, 0xCA1E1B, 0x807F0F, 0x35E104, 0x1EFF00],
            &[0x80007F, 0x801E70, 0x807F40, 0x7FE10F, 0x7FFF00],
            &[0x1E00E1, 0x351EC6, 0x7F7F71, 0xCAE11A, 0xE1FF00],
            &[0x0000FF, 0x051EFE, 0x7F7F91, 0xFAE122, 0xFFFF00],
        ],
    );
    assert_box_outside(&rgb, w, 192, 96, 5, 5, bg);

    // 2x2 -> 4x2 只放宽
    let (w, _, rgb) = export_picture_slide(&png, x, y, 38100, 19050, "AABBCC", "scale-2to4x2");
    assert_grid(
        &rgb,
        w,
        192,
        96,
        &[
            &[0xF5000A, 0xBA4007, 0x47BD03, 0x0AFF00],
            &[0x0B00F4, 0x4640B6, 0xB8BD3F, 0xF4FF00],
        ],
    );
    assert_box_outside(&rgb, w, 192, 96, 4, 2, bg);
}

#[cfg(windows)]
#[test]
fn shape_blip_scale_up_matches_picture_on_windows() {
    let png = rgba_png(2, 2, &pattern_2x2());
    let src = tmp("scale-shape-src");
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
    std::fs::write(&png_path, png).unwrap();
    let deck = tmp("scale-shape-deck");
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
                "fill": { "type": "blip", "file": png_path.to_string_lossy() },
                "x": 1828800,
                "y": 914400,
                "cx": 38100,
                "cy": 38100
            }
        ]
    }));
    let out = tmp("scale-shape-out").with_extension("png");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "exportSlideImage",
        "path": deck.to_string_lossy(),
        "id": "slide:1",
        "output": out.to_string_lossy()
    }));
    let (w, _, rgb) = decode_png(&std::fs::read(&out).unwrap());
    assert_grid(
        &rgb,
        w,
        192,
        96,
        &[
            &[0xFF0000, 0xCD4000, 0x34BD00, 0x00FF00],
            &[0xBF0040, 0x9F4030, 0x61BD11, 0x40FF00],
            &[0x4200BD, 0x61408E, 0x9DBD31, 0xBDFF00],
            &[0x0000FF, 0x3240D4, 0xCABD49, 0xFFFF00],
        ],
    );
    assert_box_outside(&rgb, w, 192, 96, 4, 4, [0xAA, 0xBB, 0xCC]);
}

#[cfg(windows)]
#[test]
fn left_clipped_scale_up_keeps_geometry_on_windows() {
    let png = rgba_png(2, 2, &pattern_2x2());
    let src = tmp("scale-clip-src");
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
    std::fs::write(&png_path, png).unwrap();
    let deck = tmp("scale-clip-deck");
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
                "fill": { "type": "blip", "file": png_path.to_string_lossy() },
                "x": -9525,
                "y": 1828800,
                "cx": 38100,
                "cy": 38100
            }
        ]
    }));
    let out = tmp("scale-clip-out").with_extension("png");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "exportSlideImage",
        "path": deck.to_string_lossy(),
        "id": "slide:1",
        "output": out.to_string_lossy()
    }));
    let (w, _, rgb) = decode_png(&std::fs::read(&out).unwrap());
    // 4x4 的右 3 列落在 x=0..2，y=192..195
    assert_grid(
        &rgb,
        w,
        0,
        192,
        &[
            &[0xCD4000, 0x34BD00, 0x00FF00],
            &[0x9F4030, 0x61BD11, 0x40FF00],
            &[0x61408E, 0x9DBD31, 0xBDFF00],
            &[0x3240D4, 0xCABD49, 0xFFFF00],
        ],
    );
    assert_eq!(pixel(&rgb, w, 3, 192), [0xAA, 0xBB, 0xCC]);
    assert_eq!(pixel(&rgb, w, 0, 191), [0xAA, 0xBB, 0xCC]);
    assert_eq!(pixel(&rgb, w, 0, 196), [0xAA, 0xBB, 0xCC]);
}

#[cfg(windows)]
#[test]
fn transparent_scale_up_keeps_geometry_on_windows() {
    // t93u：透明混合可以有残差，但位置、目标尺寸和边界不得偏移。
    let png = rgba_png(
        2,
        2,
        &[
            [255, 0, 0, 255],
            [0, 255, 0, 128],
            [0, 0, 255, 0],
            [255, 255, 0, 64],
        ],
    );
    let (w, _, rgb) =
        export_picture_slide(&png, 1828800, 914400, 38100, 38100, "AABBCC", "scale-a");
    let bg = [0xAA, 0xBB, 0xCC];
    assert_box_outside(&rgb, w, 192, 96, 4, 4, bg);
    assert_eq!(pixel(&rgb, w, 192, 96), [0xFF, 0x00, 0x00]);
    assert_eq!(pixel(&rgb, w, 192, 99), bg);
    assert_eq!(pixel(&rgb, w, 193, 99), bg);
    // 对照官方混合，只记录残差，不用颜色容差掩盖位移。
    let official: [[u32; 4]; 4] = [
        [0xFF0000, 0xD82907, 0x7A9F41, 0x5BDB6D],
        [0xEA2F33, 0xCB5A43, 0x8FAD62, 0x77D67A],
        [0xC08B97, 0xB89C94, 0xABBE8E, 0xA8D18A],
        [0xAABBCC, 0xAABBCC, 0xB6C4B0, 0xC0CD97],
    ];
    let mut max_delta = 0u32;
    let mut differ = 0u32;
    for y in 0..4 {
        for x in 0..4 {
            let got = pixel(&rgb, w, 192 + x, 96 + y);
            let want = rgb_at(official[y as usize][x as usize]);
            let d = got
                .iter()
                .zip(want.iter())
                .map(|(a, b)| u32::from(a.abs_diff(*b)))
                .max()
                .unwrap();
            if d > 0 {
                differ += 1;
            }
            max_delta = max_delta.max(d);
        }
    }
    // 几何已在上面锁死。残差照实记录，不要求 0，也不允许整块错成最近邻。
    eprintln!("transparent 2x2->4x4 residual differ={differ} maxDelta={max_delta}");
    assert!(
        max_delta < 40,
        "transparent residual too large maxDelta={max_delta}, likely not HQ bicubic"
    );
}

#[test]
fn shrinking_a_picture_is_not_implemented() {
    let png = rgba_png(4, 4, &[[0x22, 0x88, 0x44, 255]; 16]);
    let src = tmp("scale-shrink-src");
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
    std::fs::write(&png_path, png).unwrap();
    let deck = tmp("scale-shrink-deck");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": deck.to_string_lossy(),
        "ops": [
            { "op": "setPageBackground", "id": "slide:1", "color": "AABBCC" },
            { "op": "deleteShape", "id": title },
            {
                "op": "insertPicture",
                "id": "slide:1",
                "file": png_path.to_string_lossy(),
                "x": 1828800,
                "y": 914400,
                "cx": 19050,
                "cy": 19050
            }
        ]
    }));
    let err = run(&json!({
        "schemaVersion": 1,
        "action": "exportSlideImage",
        "path": deck.to_string_lossy(),
        "id": "slide:1",
        "output": tmp("scale-shrink-out").with_extension("png").to_string_lossy()
    })
    .to_string())
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::NotImplemented);
}

fn unique_4x4_png() -> Vec<u8> {
    let mut px = Vec::with_capacity(16);
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
        px.push([r, g, b, 255]);
    }
    rgba_png(4, 4, &px)
}

fn unique_8x8_png() -> Vec<u8> {
    let mut px = Vec::with_capacity(64);
    for i in 0..64u32 {
        px.push([
            ((i * 41) % 256) as u8,
            ((i * 73 + 20) % 256) as u8,
            ((i * 17 + 90) % 256) as u8,
            255,
        ]);
    }
    rgba_png(8, 8, &px)
}

#[allow(clippy::too_many_arguments)]
fn export_cropped_picture_slide(
    png: &[u8],
    x: i64,
    y: i64,
    cx: i64,
    cy: i64,
    l: i32,
    t: i32,
    r: i32,
    b: i32,
    bg: &str,
    tag: &str,
) -> (u32, u32, Vec<u8>) {
    let src = tmp(tag);
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
    std::fs::write(&png_path, png).unwrap();
    let deck = tmp(&format!("{tag}-deck"));
    run_ok(json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": deck.to_string_lossy(),
        "ops": [
            { "op": "setPageBackground", "id": "slide:1", "color": bg },
            { "op": "deleteShape", "id": title },
            {
                "op": "insertPicture",
                "id": "slide:1",
                "file": png_path.to_string_lossy(),
                "x": x,
                "y": y,
                "cx": cx,
                "cy": cy
            }
        ]
    }));
    let cropped = tmp(&format!("{tag}-crop"));
    let rect =
        format!("<a:blip r:embed=\"rId2\"/><a:srcRect l=\"{l}\" t=\"{t}\" r=\"{r}\" b=\"{b}\"/>");
    rewrite_slide(&deck, &cropped, |xml| {
        xml.replace("<a:blip r:embed=\"rId2\"/>", &rect)
    });
    let out = tmp(&format!("{tag}-out")).with_extension("png");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "exportSlideImage",
        "path": cropped.to_string_lossy(),
        "id": "slide:1",
        "output": out.to_string_lossy()
    }));
    decode_png(&std::fs::read(&out).unwrap())
}

#[allow(clippy::too_many_arguments)]
fn export_cropped_should_fail(
    png: &[u8],
    cx: i64,
    cy: i64,
    l: i32,
    t: i32,
    r: i32,
    b: i32,
    tag: &str,
) {
    let src = tmp(tag);
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
    std::fs::write(&png_path, png).unwrap();
    let deck = tmp(&format!("{tag}-deck"));
    run_ok(json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": deck.to_string_lossy(),
        "ops": [
            { "op": "setPageBackground", "id": "slide:1", "color": "AABBCC" },
            { "op": "deleteShape", "id": title },
            {
                "op": "insertPicture",
                "id": "slide:1",
                "file": png_path.to_string_lossy(),
                "x": 1828800,
                "y": 914400,
                "cx": cx,
                "cy": cy
            }
        ]
    }));
    let cropped = tmp(&format!("{tag}-crop"));
    let rect =
        format!("<a:blip r:embed=\"rId2\"/><a:srcRect l=\"{l}\" t=\"{t}\" r=\"{r}\" b=\"{b}\"/>");
    rewrite_slide(&deck, &cropped, |xml| {
        xml.replace("<a:blip r:embed=\"rId2\"/>", &rect)
    });
    let err = run(&json!({
        "schemaVersion": 1,
        "action": "exportSlideImage",
        "path": cropped.to_string_lossy(),
        "id": "slide:1",
        "output": tmp(&format!("{tag}-out")).with_extension("png").to_string_lossy()
    })
    .to_string())
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::NotImplemented);
}

#[cfg(windows)]
#[test]
fn crop_then_scale_up_matches_office_on_windows() {
    let png4 = unique_4x4_png();
    let png8 = unique_8x8_png();
    let bg = [0xAA, 0xBB, 0xCC];
    let x = 1828800i64;
    let y = 914400i64;

    let (w, _, rgb) = export_cropped_picture_slide(
        &png4, x, y, 38100, 38100, 25000, 25000, 25000, 25000, "AABBCC", "cs-c2to4",
    );
    assert_eq!(w, 1280);
    assert_grid(
        &rgb,
        w,
        192,
        96,
        &[
            &[0x3EC9C0, 0x00CAA8, 0x005A79, 0x212458],
            &[0x62CAFF, 0x3FDFE4, 0x469B97, 0x49545E],
            &[0x8BBBE7, 0x9CE2F4, 0xAED9DD, 0x89969C],
            &[0x79818D, 0x9FA4B4, 0xB2BBC7, 0x859099],
        ],
    );
    assert_box_outside(&rgb, w, 192, 96, 4, 4, bg);

    let (w, _, rgb) = export_cropped_picture_slide(
        &png4, x, y, 57150, 76200, 25000, 0, 0, 0, "AABBCC", "cs-left",
    );
    assert_grid(
        &rgb,
        w,
        192,
        96,
        &[
            &[0x42C700, 0x00B844, 0x0042D2, 0x4337DA, 0xCCBA53, 0xFFFF00],
            &[0x3DC944, 0x00BF6D, 0x0047A6, 0x2F299D, 0x9B9051, 0xD6DB21],
            &[0x3EC9C0, 0x00CAA8, 0x005A79, 0x212458, 0x5B584E, 0x7D8A54],
            &[0x62CAFF, 0x3FDFE4, 0x469B97, 0x49545E, 0x363A4E, 0x2E3A54],
            &[0x8BBBE7, 0x9CE2F4, 0xAED9DD, 0x89969C, 0x363B4B, 0x080D22],
            &[0x79818D, 0x9FA4B4, 0xB2BBC7, 0x859099, 0x303438, 0x000000],
            &[0x323033, 0x463F46, 0x4E4E54, 0x3C4044, 0x191B1C, 0x050504],
            &[0x000000, 0x000000, 0x000000, 0x000000, 0x070708, 0x0E0F10],
        ],
    );
    assert_box_outside(&rgb, w, 192, 96, 6, 8, bg);

    let (w, _, rgb) = export_cropped_picture_slide(
        &png4, x, y, 76200, 57150, 0, 25000, 0, 0, "AABBCC", "cs-top96",
    );
    assert_grid(
        &rgb,
        w,
        192,
        96,
        &[
            &[
                0xFF00C3, 0xC24CC3, 0x3FC9C2, 0x00CAA9, 0x005B79, 0x212457, 0x5A574E, 0x7C8855,
            ],
            &[
                0xE02AF8, 0xB668FD, 0x63CAFF, 0x40E0E5, 0x489C99, 0x4A555F, 0x363A4E, 0x2D3953,
            ],
            &[
                0x9A67C5, 0x9687D1, 0x8BBBE6, 0x9DE1F4, 0xAFDADD, 0x89979D, 0x363B4B, 0x070C21,
            ],
            &[
                0x4D686A, 0x5D7177, 0x787F8B, 0x9FA3B2, 0xB1B9C6, 0x858F98, 0x303438, 0x000000,
            ],
            &[
                0x172C23, 0x202D29, 0x312F32, 0x443E45, 0x4C4C52, 0x3B3F43, 0x191B1C, 0x050505,
            ],
            &[
                0x000000, 0x000000, 0x000000, 0x000000, 0x000000, 0x000000, 0x070708, 0x0E0F10,
            ],
        ],
    );
    assert_box_outside(&rgb, w, 192, 96, 8, 6, bg);

    let (w, _, rgb) = export_cropped_picture_slide(
        &png4, x, 0, 76200, 57150, 0, 25000, 0, 0, "AABBCC", "cs-top0",
    );
    assert_grid(
        &rgb,
        w,
        192,
        0,
        &[
            &[
                0xFF00C2, 0xC24CC1, 0x3EC9C0, 0x00CAA8, 0x005A79, 0x212458, 0x5B584E, 0x7D8A54,
            ],
            &[
                0xE129F8, 0xB668FD, 0x62CAFF, 0x3FDFE4, 0x469B97, 0x49545E, 0x363A4E, 0x2E3A54,
            ],
            &[
                0x9C66C6, 0x9787D3, 0x8BBBE7, 0x9CE2F4, 0xAED9DD, 0x89969C, 0x363B4B, 0x080D22,
            ],
            &[
                0x4E686B, 0x5E7178, 0x79818D, 0x9FA4B4, 0xB2BBC7, 0x859099, 0x303438, 0x000000,
            ],
            &[
                0x172D24, 0x212E2A, 0x323033, 0x463F46, 0x4E4E54, 0x3C4044, 0x191B1C, 0x050504,
            ],
            &[
                0x000000, 0x000000, 0x000000, 0x000000, 0x000000, 0x000000, 0x070708, 0x0E0F10,
            ],
        ],
    );
    assert_box_outside(&rgb, w, 192, 0, 8, 6, bg);

    let (w, _, rgb) = export_cropped_picture_slide(
        &png4, x, y, 47625, 47625, 25000, 25000, 25000, 25000, "AABBCC", "cs-c2to5",
    );
    assert_grid(
        &rgb,
        w,
        192,
        96,
        &[
            &[0x49C0B6, 0x00DAAB, 0x009490, 0x00396F, 0x272559],
            &[0x5DC2F9, 0x23E1E7, 0x26B4AE, 0x296771, 0x383D4F],
            &[0x7EC1FF, 0x6DE4FF, 0x7EDCDF, 0x7EACAA, 0x677177],
            &[0x8CAACB, 0x9FCAE1, 0xBAE0EE, 0xB7CFD8, 0x89949C],
            &[0x707680, 0x8E8E9C, 0xA7ABBB, 0xA4ADB9, 0x788189],
        ],
    );
    assert_box_outside(&rgb, w, 192, 96, 5, 5, bg);

    let (w, _, rgb) = export_cropped_picture_slide(
        &png4,
        x,
        y,
        57150,
        38100,
        25000,
        25000,
        25000,
        25000,
        "AABBCC",
        "cs-c2to6x4",
    );
    assert_grid(
        &rgb,
        w,
        192,
        96,
        &[
            &[0x52BBC0, 0x09DDB9, 0x00BBA0, 0x006D81, 0x033065, 0x2B2754],
            &[0x6EBEFF, 0x45E1FF, 0x41D9D8, 0x46A9A3, 0x497376, 0x474C58],
            &[0x8CB3E3, 0x8ED2F1, 0xA2E5F4, 0xAFE0E2, 0xA3BDC2, 0x7C868E],
            &[0x747D89, 0x8B909E, 0xA5ABBB, 0xB3BCC7, 0xA3AEBA, 0x78818A],
        ],
    );
    assert_box_outside(&rgb, w, 192, 96, 6, 4, bg);

    let (w, _, rgb) = export_cropped_picture_slide(
        &png8,
        x,
        y,
        133350,
        152400,
        12500,
        0,
        0,
        0,
        "AABBCC",
        "cs-c8left",
    );
    assert_grid(
        &rgb,
        w,
        192,
        96,
        &[
            &[
                0x143E56, 0x2B6567, 0x408C7E, 0x54CA8E, 0x69F797, 0x7CCD9E, 0x926AA7, 0xAE33AF,
                0xD156B8, 0xFEA2C0, 0xFFD2C9, 0xCEADD2, 0x5848DB, 0x0000E0,
            ],
            &[
                0x2A5393, 0x408584, 0x55B266, 0x6ECD57, 0x8BC95E, 0xA8986A, 0xBC6074, 0xB7577C,
                0xA68185, 0xADA78D, 0xBDA496, 0x9E799F, 0x563FA8, 0x1D14AD,
            ],
            &[
                0x4E78DA, 0x65B0A7, 0x7AD94E, 0x91C71B, 0xAD8A1E, 0xD05A32, 0xE25D3B, 0xB98743,
                0x6CAF4C, 0x46A154, 0x57655D, 0x653A66, 0x5A3B6F, 0x4E4474,
            ],
            &[
                0x81B0E2, 0xA2D0B3, 0xB6CB5A, 0xB09626, 0x9F5929, 0xA6513D, 0xB68047, 0x92AA4F,
                0x45A758, 0x1A7260, 0x2C3569, 0x522772, 0x6D4F7B, 0x7F7580,
            ],
            &[
                0xA4D7A4, 0xCACB98, 0xDC8E79, 0xB35968, 0x67546E, 0x427E7B, 0x52AE84, 0x59A68C,
                0x466A95, 0x40349D, 0x552EA6, 0x754DAF, 0x927AB8, 0xA59BBD,
            ],
            &[
                0x92C55F, 0x9C9654, 0xAD595F, 0x884F6F, 0x3C7E78, 0x11A980, 0x23A689, 0x457291,
                0x60349A, 0x7C21A2, 0x9948AB, 0xB485B4, 0xC9B3BD, 0xD6CEC2,
            ],
            &[
                0x5A8744, 0x395817, 0x485321, 0x4F7D3A, 0x3CAD44, 0x36A54B, 0x4B6954, 0x68335C,
                0x852D65, 0xA1486D, 0xBC7576, 0xD6AA7F, 0xEAD588, 0xF6EF8D,
            ],
            &[
                0x2F5857, 0x074F26, 0x197D28, 0x3BA83C, 0x56A546, 0x72714E, 0x8F3357, 0xAC205F,
                0xC14868, 0xBC8570, 0xA9B179, 0xA7C682, 0xBAC38B, 0xCCBB90,
            ],
            &[
                0x2F5A76, 0x2D7D67, 0x41AC6D, 0x5EA47A, 0x7B6883, 0x97328B, 0xB32C94, 0xD5479C,
                0xE774A5, 0xBEB2AD, 0x72DAB6, 0x4BC8BF, 0x5B8BC9, 0x7357CE,
            ],
            &[
                0x498462, 0x68A86E, 0x85A477, 0xA2707F, 0xB73288, 0xB21F90, 0xA04799, 0xA784A1,
                0xB8B1AA, 0x93CDB1, 0x47C8BA, 0x1F99C8, 0x305AD8, 0x482BE2,
            ],
            &[
                0x6EAB2C, 0x8DA439, 0xA96743, 0xCB314A, 0xDD2B53, 0xB4465B, 0x687364, 0x43B16C,
                0x53D975, 0x5AC77C, 0x488A85, 0x455B92, 0x5854A0, 0x6A58A8,
            ],
            &[
                0xA59D33, 0xA86F3C, 0x973145, 0x9D1E4D, 0xAE4656, 0x89835E, 0x3DB067, 0x12CC6F,
                0x24C778, 0x469285, 0x61548D, 0x804B7E, 0x9C7A5F, 0xAEA54E,
            ],
            &[
                0xCD6071, 0xAA317A, 0x5F2B83, 0x39458B, 0x4A7294, 0x50AF9C, 0x3ED8A5, 0x37C7AD,
                0x4B8AB6, 0x6854CA, 0x854ED0, 0xA5719D, 0xBFA343, 0xD0C812,
            ],
            &[
                0xBC357D, 0x802286, 0x334A8F, 0x0C8197, 0x1EA7A0, 0x41C3A8, 0x5CC5B1, 0x7397B9,
                0x895AC3, 0xA04AD7, 0xB571DD, 0xB894A9, 0xAD9B4F, 0xA19B1D,
            ],
            &[
                0x7E3546, 0x41494F, 0x2C7558, 0x30AE60, 0x45D169, 0x63BF71, 0x80887A, 0x995782,
                0xAF538B, 0xCD7898, 0xDFA19F, 0xBC9A90, 0x746371, 0x3B395F,
            ],
            &[
                0x493E12, 0x116F1B, 0x2E9724, 0x5BCA2C, 0x72EA35, 0x84B63D, 0x9A5346, 0xAE284E,
                0xC45657, 0xE9A35D, 0xF9C766, 0xBA9877, 0x45338D, 0x000099,
            ],
        ],
    );
    assert_box_outside(&rgb, w, 192, 96, 14, 16, bg);
}

#[cfg(windows)]
#[test]
fn crop_then_scale_up_dest_y_phase_matches_office_on_windows() {
    // 顶裁 4x3→8x6：官方与 GDI+ 在 y=0/64 走低相位，y=66/96 走高相位，都差 0。
    // y=65 官方已切到高相位、GDI+ 仍是低相位。这是未接受的不透明插值相位残差，
    // 不在 t93u 透明度放宽范围；测试只锁几何，并防止残差变大，不把本切片标完成。
    let png4 = unique_4x4_png();
    let bg = [0xAA, 0xBB, 0xCC];
    let x = 1828800i64;
    let low: &[&[u32]] = &[
        &[
            0xFF00C2, 0xC24CC1, 0x3EC9C0, 0x00CAA8, 0x005A79, 0x212458, 0x5B584E, 0x7D8A54,
        ],
        &[
            0xE129F8, 0xB668FD, 0x62CAFF, 0x3FDFE4, 0x469B97, 0x49545E, 0x363A4E, 0x2E3A54,
        ],
        &[
            0x9C66C6, 0x9787D3, 0x8BBBE7, 0x9CE2F4, 0xAED9DD, 0x89969C, 0x363B4B, 0x080D22,
        ],
        &[
            0x4E686B, 0x5E7178, 0x79818D, 0x9FA4B4, 0xB2BBC7, 0x859099, 0x303438, 0x000000,
        ],
        &[
            0x172D24, 0x212E2A, 0x323033, 0x463F46, 0x4E4E54, 0x3C4044, 0x191B1C, 0x050504,
        ],
        &[
            0x000000, 0x000000, 0x000000, 0x000000, 0x000000, 0x000000, 0x070708, 0x0E0F10,
        ],
    ];
    let high: &[&[u32]] = &[
        &[
            0xFF00C3, 0xC24CC3, 0x3FC9C2, 0x00CAA9, 0x005B79, 0x212457, 0x5A574E, 0x7C8855,
        ],
        &[
            0xE02AF8, 0xB668FD, 0x63CAFF, 0x40E0E5, 0x489C99, 0x4A555F, 0x363A4E, 0x2D3953,
        ],
        &[
            0x9A67C5, 0x9687D1, 0x8BBBE6, 0x9DE1F4, 0xAFDADD, 0x89979D, 0x363B4B, 0x070C21,
        ],
        &[
            0x4D686A, 0x5D7177, 0x787F8B, 0x9FA3B2, 0xB1B9C6, 0x858F98, 0x303438, 0x000000,
        ],
        &[
            0x172C23, 0x202D29, 0x312F32, 0x443E45, 0x4C4C52, 0x3B3F43, 0x191B1C, 0x050505,
        ],
        &[
            0x000000, 0x000000, 0x000000, 0x000000, 0x000000, 0x000000, 0x070708, 0x0E0F10,
        ],
    ];

    for (y_emu, y_px, grid, tag) in [
        (609600i64, 64u32, low, "cs-y64"),
        (628650, 66, high, "cs-y66"),
    ] {
        let (w, _, rgb) = export_cropped_picture_slide(
            &png4, x, y_emu, 76200, 57150, 0, 25000, 0, 0, "AABBCC", tag,
        );
        assert_grid(&rgb, w, 192, y_px, grid);
        assert_box_outside(&rgb, w, 192, y_px, 8, 6, bg);
    }

    let (w, _, rgb) = export_cropped_picture_slide(
        &png4, x, 619125, 76200, 57150, 0, 25000, 0, 0, "AABBCC", "cs-y65",
    );
    assert_box_outside(&rgb, w, 192, 65, 8, 6, bg);
    let official = high;
    let mut max_delta = 0u32;
    let mut differ = 0u32;
    for row in 0..6 {
        for col in 0..8 {
            let got = pixel(&rgb, w, 192 + col, 65 + row);
            let want = rgb_at(official[row as usize][col as usize]);
            let d = got
                .iter()
                .zip(want.iter())
                .map(|(a, b)| u32::from(a.abs_diff(*b)))
                .max()
                .unwrap();
            if d > 0 {
                differ += 1;
            }
            max_delta = max_delta.max(d);
        }
    }
    eprintln!("crop-scale dest_y=65 vs official high-phase differ={differ} maxDelta={max_delta}");
    assert!(
        max_delta <= 2,
        "unaccepted y=65 opaque phase residual grew past known official maxDelta={max_delta}"
    );
}

#[cfg(windows)]
#[test]
fn transparent_crop_then_scale_up_keeps_geometry_on_windows() {
    // 沿用已接受的放大插值后再 over。裁左上 2x2 再放到 4x4，不把 t93u 自动扩成零差声明。
    let mut px = vec![[0x20u8, 0x20, 0x20, 255]; 16];
    px[0] = [255, 0, 0, 255];
    px[1] = [0, 255, 0, 128];
    px[4] = [0, 0, 255, 0];
    px[5] = [255, 255, 0, 64];
    let png = rgba_png(4, 4, &px);
    let (w, _, rgb) = export_cropped_picture_slide(
        &png, 1828800, 914400, 38100, 38100, 0, 0, 50000, 50000, "AABBCC", "cs-alpha",
    );
    let bg = [0xAA, 0xBB, 0xCC];
    assert_box_outside(&rgb, w, 192, 96, 4, 4, bg);
    assert_eq!(pixel(&rgb, w, 192, 96), [0xFF, 0x00, 0x00]);
}

#[test]
fn shrinking_or_mixed_crop_scale_is_not_implemented() {
    let png = unique_4x4_png();
    // 中心 2x2 缩到 1x1
    export_cropped_should_fail(&png, 9525, 9525, 25000, 25000, 25000, 25000, "cs-shrink");
    // 一边放大一边缩小
    export_cropped_should_fail(&png, 38100, 9525, 25000, 25000, 25000, 25000, "cs-mix-wh");
    export_cropped_should_fail(&png, 9525, 38100, 25000, 25000, 25000, 25000, "cs-mix-hw");
}
