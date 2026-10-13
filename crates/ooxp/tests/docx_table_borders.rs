//! docx 表级边框：inspect `borders` 与 `setTableBorders`。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-docx-table-borders");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn create_table(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let path = tmp_dir().join(format!("{tag}-{n}.docx"));
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "format": "docx",
            "document": {
                "paragraphs": [{ "text": "前" }],
                "tables": [{ "cells": [["A1", "B1"], ["A2", "B2"]] }]
            }
        })
        .to_string(),
        None,
    )
    .unwrap();
    path
}

fn inspect_tables(path: &Path) -> Value {
    let RunOutput::Inspect(tree) = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": path.to_string_lossy(),
            "scope": { "tables": true, "from": 1, "maxParas": 20 }
        })
        .to_string(),
        None,
    )
    .unwrap() else {
        panic!("inspect");
    };
    tree
}

fn apply(path: &Path, out: &Path, ops: Value) {
    run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": path.to_string_lossy(),
            "output": out.to_string_lossy(),
            "ops": ops
        })
        .to_string(),
        None,
    )
    .unwrap();
}

fn apply_err(path: &Path, ops: Value) -> ooxp::Error {
    let out = tmp_dir().join("should-not.docx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": path.to_string_lossy(),
            "output": out.to_string_lossy(),
            "ops": ops
        })
        .to_string(),
        None,
    )
    .unwrap_err()
}

fn unzip_document(path: &Path) -> String {
    let file = std::fs::File::open(path).unwrap();
    let mut zip = zip::ZipArchive::new(file).unwrap();
    let mut part = zip.by_name("word/document.xml").unwrap();
    let mut s = String::new();
    std::io::Read::read_to_string(&mut part, &mut s).unwrap();
    s
}

#[test]
fn default_create_has_single_borders() {
    let src = create_table("default");
    let tree = inspect_tables(&src);
    let t = &tree["tables"][0];
    let keys: Vec<&str> = t.as_object().unwrap().keys().map(String::as_str).collect();
    assert_eq!(
        keys,
        [
            "id",
            "index",
            "rowCount",
            "colCount",
            "nested",
            "style",
            "look",
            "pos",
            "align",
            "indent",
            "width",
            "layout",
            "colWidths",
            "cellMar",
            "borders",
            "rows"
        ]
    );
    assert!(t.get("borders").is_some());
    assert_eq!(t["borders"]["top"]["val"], "single");
    assert_eq!(t["borders"]["left"]["val"], "single");
    assert_eq!(t["borders"]["insideH"]["val"], "single");
    assert_eq!(t["borders"]["insideV"]["val"], "single");
}

#[test]
fn set_table_borders_patch_and_null() {
    let src = create_table("patch");
    let out = tmp_dir().join("top-double.docx");
    apply(
        &src,
        &out,
        json!([{
            "op": "setTableBorders",
            "id": "table:1",
            "borders": { "top": { "val": "double", "sz": 12, "space": 0, "color": "000000" } }
        }]),
    );
    let tree = inspect_tables(&out);
    let t = &tree["tables"][0];
    assert_eq!(t["borders"]["top"]["val"], "double");
    assert_eq!(t["borders"]["top"]["sz"], 12);
    assert_eq!(t["borders"]["top"]["color"], "000000");
    assert_eq!(t["borders"]["left"]["val"], "single");

    let inside = tmp_dir().join("insideh.docx");
    apply(
        &out,
        &inside,
        json!([{
            "op": "setTableBorders",
            "id": "table:1",
            "borders": { "insideH": { "val": "dashed", "sz": 4, "space": 0, "color": "auto" } }
        }]),
    );
    let tree_i = inspect_tables(&inside);
    assert_eq!(tree_i["tables"][0]["borders"]["top"]["val"], "double");
    assert_eq!(tree_i["tables"][0]["borders"]["insideH"]["val"], "dashed");

    let cleared = tmp_dir().join("cleared.docx");
    apply(
        &inside,
        &cleared,
        json!([{ "op": "setTableBorders", "id": "table:1", "borders": null }]),
    );
    let tree_c = inspect_tables(&cleared);
    assert!(tree_c["tables"][0]["borders"].is_null());
    let xml = unzip_document(&cleared);
    assert!(!xml.contains("tblBorders"));
}

#[test]
fn create_table_borders() {
    let created = tmp_dir().join("create-borders.docx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": created.to_string_lossy(),
            "format": "docx",
            "document": {
                "paragraphs": [{ "text": "前" }],
                "tables": [{
                    "cells": [["A1", "B1"], ["A2", "B2"]],
                    "borders": {
                        "top": { "val": "double", "sz": 8, "space": 0, "color": "FF0000" }
                    }
                }]
            }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let tree = inspect_tables(&created);
    assert_eq!(tree["tables"][0]["borders"]["top"]["val"], "double");
    assert_eq!(tree["tables"][0]["borders"]["top"]["color"], "FF0000");
    assert_eq!(tree["tables"][0]["borders"]["left"]["val"], "single");
}

#[test]
fn create_table_borders_null_drops_defaults() {
    let created = tmp_dir().join("create-null.docx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": created.to_string_lossy(),
            "format": "docx",
            "document": {
                "paragraphs": [{ "text": "前" }],
                "tables": [{
                    "cells": [["A1"]],
                    "borders": null
                }]
            }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let tree = inspect_tables(&created);
    assert!(tree["tables"][0]["borders"].is_null());
    let xml = unzip_document(&created);
    assert!(!xml.contains("tblBorders"));
}

#[test]
fn set_table_borders_rejects() {
    let src = create_table("reject");
    let err_missing = apply_err(&src, json!([{ "op": "setTableBorders", "id": "table:1" }]));
    assert_eq!(err_missing.code(), ErrorCode::InvalidField);
    let err_empty = apply_err(
        &src,
        json!([{ "op": "setTableBorders", "id": "table:1", "borders": {} }]),
    );
    assert_eq!(err_empty.code(), ErrorCode::InvalidField);
    let err_val = apply_err(
        &src,
        json!([{
            "op": "setTableBorders",
            "id": "table:1",
            "borders": { "top": { "val": "not-a-border" } }
        }]),
    );
    assert_eq!(err_val.code(), ErrorCode::InvalidField);
    let err_id = apply_err(
        &src,
        json!([{
            "op": "setTableBorders",
            "id": "para:nope",
            "borders": { "top": { "val": "single" } }
        }]),
    );
    assert_eq!(err_id.code(), ErrorCode::InvalidId);

    let xlsx = tmp_dir().join("borders.xlsx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": xlsx.to_string_lossy(),
            "workbook": { "sheets": [{ "name": "Sheet1" }] }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let err_x = run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": xlsx.to_string_lossy(),
            "output": tmp_dir().join("borders-out.xlsx").to_string_lossy(),
            "ops": [{
                "op": "setTableBorders",
                "id": "table:1",
                "borders": { "top": { "val": "double" } }
            }]
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err_x.code(), ErrorCode::InvalidField);
}
