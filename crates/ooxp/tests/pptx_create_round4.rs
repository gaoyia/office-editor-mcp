//! create 第四轮：OLE、Zoom、域、连接线、自定义放映、讲义、备注动画、放映 / 修改保护。

use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::ZipArchive;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-pptx-create-round4");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn tmp(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}"))
}

fn create_xlsx(tag: &str) -> PathBuf {
    let path = tmp(tag).with_extension("xlsx");
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "workbook": {
            "sheets": [{
                "name": "Sheet1",
                "cells": [{ "ref": "A1", "value": "嵌入" }]
            }]
        }
    })
    .to_string())
    .unwrap();
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

fn sample_paths() -> Value {
    json!([{
        "w": 1905001,
        "h": 1270001,
        "ops": [
            { "type": "moveTo", "x": 0, "y": 0 },
            { "type": "lnTo", "x": 1905000, "y": 0 },
            { "type": "lnTo", "x": 1905000, "y": 1270000 },
            { "type": "lnTo", "x": 0, "y": 1270000 },
            { "type": "close" }
        ]
    }])
}

fn round4_presentation(xlsx: &Path) -> Value {
    json!({
        "sections": [
            { "name": "开场", "start": 1 },
            { "name": "内容", "start": 2 }
        ],
        "customShows": [{ "name": "预演", "slides": [1, 2] }],
        "handout": { "header": "讲义头", "footer": "讲义尾" },
        "showPr": { "type": "kiosk", "loop": true },
        "slides": [
            {
                "shapes": [
                    {
                        "text": "甲",
                        "fields": [{ "type": "slidenum" }],
                        "geom": { "paths": sample_paths() }
                    },
                    { "text": "乙" }
                ],
                "connectors": [{
                    "prst": "straightConnector1",
                    "start": { "shape": 1, "site": 1 },
                    "end": { "shape": 2, "site": 1 }
                }],
                "oleObjects": [{ "file": xlsx.to_string_lossy() }],
                "notes": {
                    "text": "备注",
                    "anims": [{ "type": "fade", "shape": 1 }]
                }
            },
            {
                "shapes": [{ "text": "第二页" }],
                "zooms": [{ "type": "slide", "target": 1 }]
            }
        ]
    })
}

#[test]
fn create_round4_writes_ole_show_handout_and_notes_string_still_ok() {
    let xlsx = create_xlsx("embed");
    let path = tmp("all").with_extension("pptx");
    create(&path, round4_presentation(&xlsx));

    let tree = inspect(
        &path,
        json!({
            "oleObjects": true,
            "customShows": true,
            "handout": true,
            "zooms": true,
            "fields": true,
            "connectors": true,
            "notes": true,
            "timing": true,
            "slide": 1
        }),
    );
    assert_eq!(tree["oleObjectCount"], 1);
    assert_eq!(tree["customShowCount"], 1);
    assert_eq!(tree["customShows"][0]["name"], "预演");
    assert_eq!(
        tree["customShows"][0]["slides"],
        json!(["slide:1", "slide:2"])
    );
    assert_eq!(tree["handout"]["header"], "讲义头");
    assert_eq!(tree["handout"]["footer"], "讲义尾");
    assert_eq!(tree["showPr"]["type"], "kiosk");
    assert_eq!(tree["showPr"]["loop"], true);
    assert_eq!(tree["slides"][0]["fieldCount"], 1);
    assert_eq!(tree["slides"][0]["fields"][0]["type"], "slidenum");
    assert_eq!(
        tree["slides"][0]["shapes"][0]["geom"]["paths"][0]["ops"][0]["type"],
        "moveTo"
    );
    assert_eq!(tree["slides"][0]["connectors"][0]["start"]["site"], 1);
    assert_eq!(tree["slides"][0]["notes"]["id"], "slide:1/notes");
    let notes_text = tree["slides"][0]["notes"]["shapes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["text"] == "备注")
        .is_some();
    assert!(notes_text, "备注页应有可见文字「备注」");
    assert_eq!(tree["slides"][0]["notes"]["animCount"], 1);
    assert_eq!(tree["slides"][0]["notes"]["timing"][0]["type"], "fade");

    let whole = inspect(&path, json!({ "zooms": true, "slide": 2 }));
    assert_eq!(whole["slides"][1]["zoomCount"], 1);
    assert_eq!(whole["slides"][1]["zooms"][0]["type"], "slide");
    assert_eq!(whole["slides"][1]["zooms"][0]["target"], "slide:1");

    let slide = part_text(&path, "ppt/slides/slide1.xml");
    assert!(slide.contains("<a:custGeom>"));
    assert!(slide.contains(r#"type="slidenum""#));
    assert!(slide.contains("<a:stCxn"));
    assert!(slide.contains("oleObj") || slide.contains("ole"));
    let notes = part_text(&path, "ppt/notesSlides/notesSlide1.xml");
    assert!(notes.contains("<p:timing>"));
    let pres = part_text(&path, "ppt/presentation.xml");
    assert!(pres.contains("custShowLst"));
    let props = part_text(&path, "ppt/presProps.xml");
    assert!(props.contains("<p:showPr>") || props.contains("showPr"));

    let notes_only = tmp("notes-str").with_extension("pptx");
    create(
        &notes_only,
        json!({
            "slides": [{ "shapes": [{ "text": "甲" }], "notes": "纯文本备注" }]
        }),
    );
    let ntree = inspect(&notes_only, json!({ "notes": true, "slide": 1 }));
    let got = ntree["slides"][0]["notes"]["shapes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|s| s["text"] == "纯文本备注");
    assert!(got, "字符串 notes 仍应写成备注页文字");
}

#[test]
fn create_round4_password_writes_hash_without_com() {
    let path = tmp("hash").with_extension("pptx");
    create(
        &path,
        json!({
            "modifyProtection": { "enforced": true, "password": "secret" },
            "slides": [{ "shapes": [{ "text": "甲" }] }]
        }),
    );
    let tree = inspect(&path, json!({}));
    assert_eq!(tree["modifyProtection"]["hasPassword"], true);
    let xml = part_text(&path, "ppt/presentation.xml");
    assert!(xml.contains("modifyVerifier"));
    assert!(xml.contains("hashData="), "{xml}");
}

#[test]
fn create_round4_rejects_page_fields_timing_and_xlsx_custom_shows() {
    let path = tmp("bad").with_extension("pptx");
    let err = create_err(
        &path,
        json!({ "slides": [{ "shapes": [{ "text": "a" }], "timing": [] }] }),
    );
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!path.exists());

    let err = create_err(
        &path,
        json!({ "slides": [{ "shapes": [{ "text": "a" }], "fields": [] }] }),
    );
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let err = create_err(
        &path,
        json!({ "slides": [{ "shapes": [{ "text": "a" }], "hyperlinks": [] }] }),
    );
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let err = create_err(
        &path,
        json!({
            "oleObjects": [{ "file": "a.xlsx" }],
            "slides": [{ "shapes": [{ "text": "a" }] }]
        }),
    );
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let xlsx = tmp("xlsx-cs").with_extension("xlsx");
    let err = run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": xlsx.to_string_lossy(),
        "workbook": {
            "customShows": [{ "name": "A", "slides": [1] }],
            "sheets": [{ "name": "Sheet1", "cells": [{ "ref": "A1", "value": 1 }] }]
        }
    })
    .to_string())
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!xlsx.exists());
}
