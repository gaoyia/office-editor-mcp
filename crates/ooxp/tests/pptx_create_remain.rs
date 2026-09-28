//! create 补齐：节、形状级超链接、媒体、切换、预设进入淡入。

use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::ZipArchive;

/// 只含 `ftyp` 的最小 MP4，库按文件头认，不解码。
const MP4_FTYP: &[u8] = &[
    0x00, 0x00, 0x00, 0x18, b'f', b't', b'y', b'p', b'i', b's', b'o', b'm', 0x00, 0x00, 0x00, 0x00,
    b'i', b's', b'o', b'm', b'm', b'p', b'4', b'1',
];

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-pptx-create-remain");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn tmp(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}"))
}

fn tiny_wav() -> Vec<u8> {
    let data_len = 8u32;
    let mut v = Vec::new();
    v.extend_from_slice(b"RIFF");
    v.extend_from_slice(&(36u32 + data_len).to_le_bytes());
    v.extend_from_slice(b"WAVE");
    v.extend_from_slice(b"fmt ");
    v.extend_from_slice(&16u32.to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes());
    v.extend_from_slice(&8000u32.to_le_bytes());
    v.extend_from_slice(&16000u32.to_le_bytes());
    v.extend_from_slice(&2u16.to_le_bytes());
    v.extend_from_slice(&16u16.to_le_bytes());
    v.extend_from_slice(b"data");
    v.extend_from_slice(&data_len.to_le_bytes());
    v.extend_from_slice(&[0u8; 8]);
    v
}

fn write_mp4(tag: &str) -> PathBuf {
    let path = tmp(tag).with_extension("mp4");
    std::fs::write(&path, MP4_FTYP).unwrap();
    path
}

fn write_wav(tag: &str) -> PathBuf {
    let path = tmp(tag).with_extension("wav");
    std::fs::write(&path, tiny_wav()).unwrap();
    path
}

fn create(path: &Path, presentation: Value) {
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "presentation": presentation
        })
        .to_string(),
        None,
    )
    .unwrap();
}

fn create_err(path: &Path, presentation: Value) -> ooxp::Error {
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "presentation": presentation
        })
        .to_string(),
        None,
    )
    .unwrap_err()
}

fn inspect(path: &Path, scope: Value) -> Value {
    let RunOutput::Inspect(tree) = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": path.to_string_lossy(),
            "scope": scope
        })
        .to_string(),
        None,
    )
    .unwrap() else {
        panic!("inspect 应返回树");
    };
    tree
}

fn part_text(path: &Path, name: &str) -> String {
    let bytes = std::fs::read(path).unwrap();
    let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut f = zip.by_name(name).unwrap();
    let mut s = String::new();
    f.read_to_string(&mut s).unwrap();
    s
}

#[test]
fn create_remain_writes_sections_link_transition_anim() {
    let path = tmp("remain").with_extension("pptx");
    create(
        &path,
        json!({
            "sections": [
                { "name": "开场", "start": 1 },
                { "name": "内容", "start": 2 }
            ],
            "slides": [
                {
                    "shapes": [{ "text": "标题", "url": "https://example.com/" }],
                    "transition": { "type": "fade" },
                    "anims": [{ "type": "fade", "shape": 1 }]
                },
                { "shapes": [{ "text": "第二页" }] }
            ]
        }),
    );
    let tree = inspect(
        &path,
        json!({
            "sections": true,
            "hyperlinks": true,
            "timing": true,
            "slide": 1
        }),
    );
    assert_eq!(tree["sectionCount"], 2);
    assert_eq!(tree["sections"][0]["name"], "开场");
    assert_eq!(tree["sections"][1]["name"], "内容");
    assert_eq!(tree["sections"][1]["slides"], json!(["slide:2"]));
    assert_eq!(tree["slides"][0]["hyperlinkCount"], 1);
    assert_eq!(
        tree["slides"][0]["hyperlinks"][0]["url"],
        "https://example.com/"
    );
    assert_eq!(tree["slides"][0]["transition"]["type"], "fade");
    assert_eq!(tree["slides"][0]["animCount"], 1);
    assert_eq!(tree["slides"][0]["timing"][0]["type"], "fade");
    assert_eq!(tree["slides"][0]["timing"][0]["target"], "slide:1/shape:2");
    let xml = part_text(&path, "ppt/slides/slide1.xml");
    assert!(xml.contains("hlinkClick"));
    assert!(xml.contains("<p:transition"));
    assert!(xml.contains("<p:timing"));
    assert!(xml.contains("filter=\"fade\""));
}

#[test]
fn create_remain_writes_video_and_audio() {
    let mp4 = write_mp4("clip");
    let wav = write_wav("beep");
    let path = tmp("media").with_extension("pptx");
    create(
        &path,
        json!({
            "slides": [{
                "shapes": [{ "text": "标题" }],
                "videos": [{ "file": mp4.to_string_lossy() }],
                "audios": [{ "file": wav.to_string_lossy() }]
            }]
        }),
    );
    let tree = inspect(&path, json!({ "media": true }));
    assert_eq!(tree["slides"][0]["videoCount"], 1);
    assert_eq!(tree["slides"][0]["audioCount"], 1);
    let videos = tree["slides"][0]["videos"].as_array().unwrap();
    let audios = tree["slides"][0]["audios"].as_array().unwrap();
    assert_eq!(videos.len(), 1);
    assert_eq!(audios.len(), 1);
    assert!(videos[0]["id"]
        .as_str()
        .unwrap()
        .starts_with("slide:1/video:"));
    assert!(audios[0]["id"]
        .as_str()
        .unwrap()
        .starts_with("slide:1/audio:"));
    let xml = part_text(&path, "ppt/slides/slide1.xml");
    assert!(xml.contains("videoFile"));
    assert!(xml.contains("audioFile"));
    assert!(!xml.contains("<p:timing"));
}

#[test]
fn create_remain_shape_jump_and_anim_on_picture() {
    let png = tmp("pic").with_extension("png");
    std::fs::write(
        &png,
        [
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48,
            0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00,
            0x00, 0x90, 0x77, 0x53, 0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08,
            0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00, 0x00, 0x03, 0x01, 0x01, 0x00, 0x18, 0xDD, 0x8D,
            0xB4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
        ],
    )
    .unwrap();
    let path = tmp("jump-pic").with_extension("pptx");
    create(
        &path,
        json!({
            "slides": [
                {
                    "shapes": [{ "text": "去第二页", "slide": 2 }],
                    "pictures": [{ "file": png.to_string_lossy() }],
                    "anims": [{ "type": "fade", "picture": 1 }]
                },
                { "shapes": [{ "text": "落地" }] }
            ]
        }),
    );
    let tree = inspect(
        &path,
        json!({ "hyperlinks": true, "timing": true, "pictures": true, "slide": 1 }),
    );
    assert_eq!(tree["slides"][0]["hyperlinks"][0]["kind"], "slide");
    assert_eq!(tree["slides"][0]["hyperlinks"][0]["slide"], "slide:2");
    assert_eq!(
        tree["slides"][0]["timing"][0]["target"],
        tree["slides"][0]["pictures"][0]["id"]
    );
}

#[test]
fn create_remain_rejects_unknown_and_bad_fields() {
    let path = tmp("bad").with_extension("pptx");
    let err = create_err(
        &path,
        json!({
            "slides": [{ "shapes": [{ "text": "a" }], "hyperlinks": [] }]
        }),
    );
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!path.exists());

    let err = create_err(
        &path,
        json!({
            "slides": [{ "shapes": [{ "text": "a" }], "oleObjects": [{ "file": "a.xlsx" }] }]
        }),
    );
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let err = create_err(
        &path,
        json!({
            "slides": [{ "shapes": [{ "text": "a" }], "timing": [] }]
        }),
    );
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let err = create_err(
        &path,
        json!({
            "slides": [{ "shapes": [{ "text": "a" }], "transition": { "type": null } }]
        }),
    );
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let err = create_err(
        &path,
        json!({
            "slides": [{
                "shapes": [{ "text": "a" }],
                "anims": [{ "type": "fade", "shape": 1, "picture": 1 }]
            }]
        }),
    );
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let err = create_err(
        &path,
        json!({
            "slides": [{
                "shapes": [{ "text": "a", "slide": 1 }]
            }]
        }),
    );
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let err = create_err(
        &path,
        json!({
            "sections": [{ "name": "后", "start": 2 }],
            "slides": [{ "shapes": [{ "text": "a" }] }]
        }),
    );
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let xlsx = tmp("xlsx-sec").with_extension("xlsx");
    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": xlsx.to_string_lossy(),
            "workbook": {
                "sections": [{ "name": "A", "start": 1 }],
                "sheets": [{ "name": "Sheet1", "cells": [{ "ref": "A1", "value": 1 }] }]
            }
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!xlsx.exists());
}
