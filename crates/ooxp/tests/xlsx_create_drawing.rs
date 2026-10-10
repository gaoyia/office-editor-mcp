//! xlsx create：批注、超链接、文本框、形状、连接线、SmartArt。

use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};

const UID_LIST: &str = "urn:microsoft.com/office/officeart/2005/8/layout/default";

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-xlsx-create-drawing");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn tmp(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}.xlsx"))
}

fn create_book(path: &Path, sheets: Value) {
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "workbook": { "sheets": sheets }
        })
        .to_string(),
        None,
    )
    .unwrap();
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

fn part_names(path: &Path) -> Vec<String> {
    let f = std::fs::File::open(path).unwrap();
    let zip = zip::ZipArchive::new(f).unwrap();
    zip.file_names().map(str::to_string).collect()
}

fn part_text(path: &Path, name: &str) -> String {
    let f = std::fs::File::open(path).unwrap();
    let mut zip = zip::ZipArchive::new(f).unwrap();
    let mut zf = zip.by_name(name).unwrap();
    let mut out = String::new();
    zf.read_to_string(&mut out).unwrap();
    out
}

#[test]
fn create_writes_comments_hyperlinks_and_drawing() {
    let path = tmp("full");
    create_book(
        &path,
        json!([{
            "name": "Sheet1",
            "cells": [{ "ref": "A1", "value": "链字" }],
            "comments": [{
                "cell": "A1",
                "text": "C17批注",
                "author": "Alice",
                "date": "2026-09-13T00:00:00.000"
            }],
            "hyperlinks": [{
                "ref": "A1",
                "url": "https://example.com/",
                "tooltip": "示例"
            }],
            "textBoxes": [{ "from": "B2", "text": "框" }],
            "shapes": [{ "from": "B4", "prst": "ellipse" }],
            "connectors": [{ "from": "B5", "prst": "straightConnector1" }],
            "smartArts": [{ "from": "D6", "layout": "list" }]
        }]),
    );

    let names = part_names(&path);
    assert!(names.iter().any(|n| n.starts_with("xl/threadedComments/")));
    assert!(names.iter().any(|n| n == "xl/persons/person.xml"));
    assert!(names.iter().any(|n| n.starts_with("xl/comments")));
    assert!(names.iter().any(|n| n.contains("vmlDrawing")));
    assert!(names.iter().any(|n| n == "xl/drawings/drawing1.xml"));
    assert!(names.iter().any(|n| n.starts_with("xl/diagrams/")));

    let sheet = part_text(&path, "xl/worksheets/sheet1.xml");
    assert!(sheet.contains("legacyDrawing"));
    assert!(sheet.contains("<hyperlinks>"));

    let tree = inspect(
        &path,
        json!({
            "comments": true,
            "hyperlinks": true,
            "textBoxes": true,
            "shapes": true,
            "connectors": true,
            "smartArt": true
        }),
    );
    assert_eq!(tree["commentCount"], 1);
    let comments = tree["sheets"][0]["comments"].as_array().unwrap();
    assert_eq!(comments[0]["ref"], "A1");
    assert_eq!(comments[0]["text"], "C17批注");
    assert_eq!(comments[0]["author"], "Alice");
    assert!(comments[0]["id"]
        .as_str()
        .unwrap()
        .starts_with("cell:Sheet1!A1/comment:{"));

    let links = tree["sheets"][0]["hyperlinks"].as_array().unwrap();
    assert_eq!(links[0]["id"], "sheet:Sheet1/link:1");
    assert_eq!(links[0]["url"], "https://example.com/");
    assert_eq!(links[0]["tooltip"], "示例");

    let boxes = tree["sheets"][0]["textBoxes"].as_array().unwrap();
    assert_eq!(boxes[0]["from"], "B2");
    assert_eq!(boxes[0]["text"], "框");

    let shapes = tree["sheets"][0]["shapes"].as_array().unwrap();
    assert_eq!(shapes[0]["from"], "B4");
    assert_eq!(shapes[0]["prst"], "ellipse");

    let cxns = tree["sheets"][0]["connectors"].as_array().unwrap();
    assert_eq!(cxns[0]["from"], "B5");

    let arts = tree["sheets"][0]["smartArts"].as_array().unwrap();
    assert_eq!(arts[0]["from"], "D6");
    assert_eq!(arts[0]["layout"], UID_LIST);
    assert!(arts[0]["id"]
        .as_str()
        .unwrap()
        .starts_with("sheet:Sheet1/smartdiagram:"));
}

#[test]
fn create_empty_arrays_omit_parts() {
    let path = tmp("empty");
    create_book(
        &path,
        json!([{
            "name": "Sheet1",
            "comments": [],
            "hyperlinks": [],
            "textBoxes": [],
            "shapes": [],
            "connectors": [],
            "smartArts": []
        }]),
    );
    let names = part_names(&path);
    assert!(!names.iter().any(|n| n.contains("threadedComments")));
    assert!(!names.iter().any(|n| n == "xl/drawings/drawing1.xml"));
    let tree = inspect(&path, json!({ "comments": true, "hyperlinks": true }));
    assert_eq!(tree["commentCount"], 0);
    assert_eq!(tree["hyperlinkCount"], 0);
}

#[test]
fn create_rejects_root_comments() {
    let path = tmp("bad");
    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "workbook": {
                "comments": [],
                "sheets": [{ "name": "Sheet1" }]
            }
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert_eq!(err.field_path(), Some("workbook.comments"));
}

#[test]
fn create_rejects_smartart_kind_and_missing_from() {
    let path = tmp("kind");
    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "workbook": {
                "sheets": [{
                    "name": "Sheet1",
                    "smartArts": [{ "layout": "list", "from": "A1", "kind": "anchor" }]
                }]
            }
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert_eq!(
        err.field_path(),
        Some("workbook.sheets[0].smartArts[0].kind")
    );

    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "workbook": {
                "sheets": [{
                    "name": "Sheet1",
                    "textBoxes": [{ "text": "框" }]
                }]
            }
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert_eq!(
        err.field_path(),
        Some("workbook.sheets[0].textBoxes[0].from")
    );
}

#[test]
fn create_rejects_hyperlink_overlap() {
    let path = tmp("overlap");
    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "workbook": {
                "sheets": [{
                    "name": "Sheet1",
                    "hyperlinks": [
                        { "ref": "A1:B1", "url": "https://a.example/" },
                        { "ref": "B1", "url": "https://b.example/" }
                    ]
                }]
            }
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert_eq!(
        err.field_path(),
        Some("workbook.sheets[0].hyperlinks[1].ref")
    );
}
