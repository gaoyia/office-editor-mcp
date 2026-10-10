//! docx create 第五轮：框内表与链、段边框底纹、runs、组合、writePassword。

use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::ZipArchive;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-docx-create-round5");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn seq_path(tag: &str, ext: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}.{ext}"))
}

fn create_docx(tag: &str, document: Value) -> PathBuf {
    let path = seq_path(tag, "docx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "format": "docx",
            "document": document
        })
        .to_string(),
        None,
    )
    .unwrap();
    path
}

fn create_err(document: Value) -> ooxp::Error {
    let path = seq_path("err", "docx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "format": "docx",
            "document": document
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
    let mut file = zip.by_name(name).unwrap();
    let mut s = String::new();
    std::io::Read::read_to_string(&mut file, &mut s).unwrap();
    s
}

fn round5_document() -> Value {
    json!({
        "paragraphs": [
            {
                "borders": {
                    "top": { "val": "single", "sz": 12, "space": 1, "color": "000000" }
                },
                "shd": { "fill": "ffff00" },
                "tabs": [{ "val": "left", "pos": 720 }],
                "runs": [
                    { "text": "黄", "highlight": "yellow", "strike": true },
                    { "text": "常" }
                ]
            },
            {
                "text": "框",
                "textBoxes": [{
                    "text": "框内",
                    "tables": [{ "cells": [["A"]] }],
                    "hyperlinks": [{ "text": "链", "url": "https://example.com" }]
                }],
                "shapes": [{ "prst": "ellipse" }]
            }
        ],
        "groups": [{
            "ids": ["para:00000002/txbx:1", "para:00000002/shape:2"]
        }]
    })
}

#[test]
fn create_round5_box_table_link_para_run_and_group() {
    let path = create_docx("round5", round5_document());
    let tree = inspect(
        &path,
        json!({
            "from": 1,
            "maxParas": 20,
            "textBoxes": true,
            "tables": true,
            "hyperlinks": true,
            "shapes": true,
            "runs": true
        }),
    );

    assert_eq!(tree["tableCount"], 0, "{tree}");
    assert_eq!(tree["hyperlinkCount"], 0, "{tree}");
    let bx = &tree["paragraphs"][1]["textBoxes"][0];
    assert_eq!(bx["tableCount"], 1, "{bx}");
    assert_eq!(bx["hyperlinkCount"], 1, "{bx}");
    assert_eq!(bx["tables"][0]["rowCount"], 1);
    assert_eq!(bx["tables"][0]["colCount"], 1);
    assert_eq!(bx["hyperlinks"][0]["url"], "https://example.com");

    let p0 = &tree["paragraphs"][0];
    assert_eq!(p0["borders"]["top"]["val"], "single");
    assert_eq!(p0["shd"]["fill"], "FFFF00");
    assert_eq!(p0["tabs"][0]["val"], "left");
    assert_eq!(p0["tabs"][0]["pos"], 720);
    assert_eq!(p0["runs"][0]["text"], "黄");
    assert_eq!(p0["runs"][0]["highlight"], "yellow");
    assert_eq!(p0["runs"][0]["strike"], true);
    assert_eq!(p0["runs"][1]["text"], "常");

    let xml = part_text(&path, "word/document.xml");
    assert!(xml.contains("wpg:wgp"), "{xml}");
}

#[test]
fn create_round5_write_password() {
    let path = create_docx(
        "wp",
        json!({
            "paragraphs": [{ "text": "a" }],
            "protection": { "writePassword": "secret" }
        }),
    );
    let tree = inspect(&path, json!({ "from": 1, "maxParas": 5 }));
    assert_eq!(tree["protection"]["hasPassword"], true, "{tree}");
    let dumped = tree.to_string();
    assert!(!dumped.contains("secret"), "{dumped}");
    let settings = part_text(&path, "word/settings.xml");
    assert!(settings.contains("writeProtection"), "{settings}");
    assert!(settings.contains("cryptAlgorithmSid"), "{settings}");
}

#[test]
fn create_round5_rejects_empty_runs_box_after_zero_and_foreign_fields() {
    let err = create_err(json!({
        "paragraphs": [{ "runs": [] }]
    }));
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let err = create_err(json!({
        "paragraphs": [{
            "text": "a",
            "textBoxes": [{
                "text": "b",
                "tables": [{ "cells": [["A"]], "after": 0 }]
            }]
        }]
    }));
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let xlsx = seq_path("xlsx", "xlsx");
    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": xlsx.to_string_lossy(),
            "format": "xlsx",
            "workbook": {
                "sheets": [{ "name": "Sheet1" }],
                "groups": [{ "ids": ["a", "b"] }]
            }
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}
