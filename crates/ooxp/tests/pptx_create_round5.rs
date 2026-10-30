//! create 第五轮：备注页公式 / 3D，图 / 表 / 图表 / SmartArt 链，表单元格链。

use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, sample_glb, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::ZipArchive;

const PNG_1X1: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
    0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08, 0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00,
    0x00, 0x03, 0x01, 0x01, 0x00, 0x18, 0xDD, 0x8D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E,
    0x44, 0xAE, 0x42, 0x60, 0x82,
];

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-pptx-create-round5");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn tmp(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}"))
}

fn write_png(tag: &str) -> PathBuf {
    let path = tmp(tag).with_extension("png");
    std::fs::write(&path, PNG_1X1).unwrap();
    path
}

fn write_glb(tag: &str) -> PathBuf {
    let path = tmp(tag).with_extension("glb");
    std::fs::write(&path, sample_glb()).unwrap();
    path
}

fn create(path: &Path, presentation: Value) {
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "presentation": presentation
    })
    .to_string())
    .unwrap();
}

fn create_err(path: &Path, presentation: Value) -> ooxp::Error {
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "presentation": presentation
    })
    .to_string())
    .unwrap_err()
}

fn inspect(path: &Path, scope: Value) -> Value {
    let RunOutput::Inspect(tree) = run(&json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": path.to_string_lossy(),
        "scope": scope
    })
    .to_string())
    .unwrap() else {
        panic!("inspect 应返回树");
    };
    tree
}

fn part_text(path: &Path, name: &str) -> String {
    let bytes = std::fs::read(path).unwrap();
    let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut file = zip.by_name(name).unwrap();
    let mut out = Vec::new();
    file.read_to_end(&mut out).unwrap();
    String::from_utf8(out).unwrap()
}

fn by_on<'a>(links: &'a [Value], on: &str) -> &'a Value {
    links
        .iter()
        .find(|l| l["on"] == on)
        .unwrap_or_else(|| panic!("缺少 on={on}"))
}

#[test]
fn create_round5_notes_math_and_model3d() {
    let glb = write_glb("notes");
    let path = tmp("notes").with_extension("pptx");
    create(
        &path,
        json!({
            "slides": [{
                "shapes": [{ "text": "标题" }],
                "notes": {
                    "text": "备注",
                    "maths": [{ "text": "E=mc^2" }],
                    "model3ds": [{ "file": glb }]
                }
            }]
        }),
    );
    let tree = inspect(
        &path,
        json!({ "maths": true, "model3ds": true, "notes": true, "slide": 1 }),
    );
    assert_eq!(tree["mathCount"], 0);
    assert_eq!(tree["model3dCount"], 0);
    let notes = &tree["slides"][0]["notes"];
    assert_eq!(notes["mathCount"], 1);
    assert_eq!(notes["maths"][0]["text"], "E=mc^2");
    assert_eq!(notes["model3dCount"], 1);
    let notes_xml = part_text(&path, "ppt/notesSlides/notesSlide1.xml");
    assert!(notes_xml.contains("a14:m"));
    assert!(notes_xml.contains("am3d") || notes_xml.contains("model3d"));
}

#[test]
fn create_round5_notes_string_still_text_only() {
    let path = tmp("notes-str").with_extension("pptx");
    create(
        &path,
        json!({
            "slides": [{
                "shapes": [{ "text": "标题" }],
                "notes": "只要文本"
            }]
        }),
    );
    let tree = inspect(&path, json!({ "notes": true, "maths": true, "slide": 1 }));
    assert_eq!(tree["slides"][0]["notes"]["mathCount"], 0);
    let body = tree["slides"][0]["notes"]["shapes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["text"] == "只要文本")
        .expect("备注正文");
    assert_eq!(body["text"], "只要文本");
}

#[test]
fn create_round5_object_hyperlinks() {
    let png = write_png("pic");
    let path = tmp("objs").with_extension("pptx");
    create(
        &path,
        json!({
            "slides": [{
                "shapes": [{ "text": "标题" }],
                "pictures": [{
                    "file": png,
                    "hyperlink": { "url": "https://example.com/pic", "event": "hover" }
                }],
                "tables": [{
                    "rows": 1,
                    "cols": 2,
                    "hyperlink": { "url": "https://example.com/table" },
                    "cells": [[
                        "甲",
                        { "text": "乙", "url": "https://example.com/cell" }
                    ]]
                }],
                "charts": [{
                    "type": "col",
                    "series": [{ "values": [1, 2, 3] }],
                    "hyperlink": { "url": "https://example.com/chart" }
                }],
                "smartArts": [{
                    "layout": "list",
                    "hyperlink": { "url": "https://example.com/sa" }
                }]
            }]
        }),
    );
    let tree = inspect(
        &path,
        json!({
            "hyperlinks": true,
            "pictures": true,
            "tables": true,
            "charts": true,
            "smartArt": true,
            "slide": 1
        }),
    );
    let links = tree["slides"][0]["hyperlinks"].as_array().unwrap();
    assert_eq!(by_on(links, "pic")["url"], "https://example.com/pic");
    assert_eq!(by_on(links, "pic")["event"], "hover");
    assert_eq!(by_on(links, "table")["url"], "https://example.com/table");
    assert_eq!(by_on(links, "cell")["url"], "https://example.com/cell");
    assert_eq!(by_on(links, "cell")["text"], "乙");
    assert_eq!(by_on(links, "chart")["url"], "https://example.com/chart");
    assert_eq!(by_on(links, "smartArt")["url"], "https://example.com/sa");
    let xml = part_text(&path, "ppt/slides/slide1.xml");
    assert!(xml.contains("hlinkHover"));
    assert!(xml.contains("hlinkClick"));
}

#[test]
fn create_round5_rejects_hyperlink_text_and_timing() {
    let png = write_png("bad");
    let path = tmp("bad").with_extension("pptx");
    let err = create_err(
        &path,
        json!({
            "slides": [{
                "shapes": [{ "text": "a" }],
                "pictures": [{
                    "file": png,
                    "hyperlink": { "url": "https://example.com/", "text": "x" }
                }]
            }]
        }),
    );
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!path.exists());

    let err = create_err(
        &path,
        json!({ "slides": [{ "shapes": [{ "text": "a" }], "timing": [] }] }),
    );
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

#[test]
fn create_round5_cell_string_still_ok() {
    let path = tmp("cells").with_extension("pptx");
    create(
        &path,
        json!({
            "slides": [{
                "shapes": [{ "text": "标题" }],
                "tables": [{
                    "rows": 2,
                    "cols": 2,
                    "cells": [["甲", "乙"], ["丙", "丁"]]
                }]
            }]
        }),
    );
    let tree = inspect(
        &path,
        json!({ "tables": true, "hyperlinks": true, "slide": 1 }),
    );
    assert_eq!(tree["slides"][0]["tableCount"], 1);
    assert_eq!(tree["slides"][0]["hyperlinkCount"], 0);
}
