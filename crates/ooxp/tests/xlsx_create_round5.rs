//! xlsx create 第五轮：命名单元格样式与一层批注回复。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-xlsx-create-round5");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn tmp(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}.xlsx"))
}

fn create(path: &Path, workbook: Value) {
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "workbook": workbook
        })
        .to_string(),
        None,
    )
    .unwrap();
}

fn create_err(path: &Path, workbook: Value) -> ooxp::Error {
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "workbook": workbook
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

fn sample_book() -> Value {
    json!({
        "cellStyles": [{
            "name": "OoxpNamed",
            "font": { "bold": true },
            "fill": { "pattern": "solid", "fg": { "rgb": "FFFFFF00" } }
        }],
        "sheets": [{
            "name": "Sheet1",
            "cells": [
                { "ref": "A1", "value": 1.2, "namedStyle": "OoxpNamed" },
                { "ref": "B1", "value": 3 }
            ],
            "comments": [{
                "cell": "A1",
                "text": "顶层批注",
                "replies": [{ "text": "一层回复", "author": "Bob" }]
            }]
        }]
    })
}

#[test]
fn create_named_style_and_reply() {
    let path = tmp("ok");
    create(&path, sample_book());
    let tree = inspect(
        &path,
        json!({
            "sheet": "Sheet1",
            "range": "A1:B1",
            "cellStyles": true,
            "comments": true
        }),
    );
    assert_eq!(tree["cellStyleCount"], 2);
    assert_eq!(tree["cellStyles"][1]["name"], "OoxpNamed");
    assert_eq!(tree["cellStyles"][1]["builtinId"], Value::Null);
    let a1 = tree["sheets"][0]["cells"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "cell:Sheet1!A1")
        .unwrap();
    assert_eq!(a1["font"]["bold"], true);
    assert_eq!(a1["fill"]["pattern"], "solid");
    let comments = tree["sheets"][0]["comments"].as_array().unwrap();
    assert_eq!(comments.len(), 1);
    assert_eq!(comments[0]["text"], "顶层批注");
    assert_eq!(comments[0]["replies"][0]["text"], "一层回复");
    assert_eq!(comments[0]["replies"][0]["author"], "Bob");
}

#[test]
fn named_style_conflicts_with_inline() {
    let path = tmp("conflict");
    let e = create_err(
        &path,
        json!({
            "cellStyles": [{
                "name": "OoxpNamed",
                "font": { "bold": true }
            }],
            "sheets": [{
                "name": "Sheet1",
                "cells": [{
                    "ref": "A1",
                    "value": 1,
                    "namedStyle": "OoxpNamed",
                    "font": { "italic": true }
                }]
            }]
        }),
    );
    assert_eq!(e.code(), ErrorCode::InvalidField);
    assert_eq!(e.field_path(), Some("workbook.sheets[0].cells[0]"));
}

#[test]
fn unknown_named_style_and_nested_replies() {
    let path = tmp("bad");
    let e = create_err(
        &path,
        json!({
            "sheets": [{
                "name": "Sheet1",
                "cells": [{ "ref": "A1", "value": 1, "namedStyle": "Missing" }]
            }]
        }),
    );
    assert_eq!(e.code(), ErrorCode::InvalidField);
    assert_eq!(
        e.field_path(),
        Some("workbook.sheets[0].cells[0].namedStyle")
    );

    let e = create_err(
        &path,
        json!({
            "sheets": [{
                "name": "Sheet1",
                "comments": [{
                    "cell": "A1",
                    "text": "甲",
                    "replies": [{
                        "text": "乙",
                        "replies": [{ "text": "丙" }]
                    }]
                }]
            }]
        }),
    );
    assert_eq!(e.code(), ErrorCode::InvalidField);
    assert_eq!(
        e.field_path(),
        Some("workbook.sheets[0].comments[0].replies[0].replies")
    );
}

#[test]
fn empty_replies_ok_and_datatable_still_rejected() {
    let path = tmp("empty-r");
    create(
        &path,
        json!({
            "sheets": [{
                "name": "Sheet1",
                "comments": [{ "cell": "A1", "text": "甲", "replies": [] }]
            }]
        }),
    );
    let tree = inspect(&path, json!({ "comments": true }));
    assert_eq!(tree["sheets"][0]["comments"][0]["replies"], json!([]));

    let e = create_err(
        &tmp("dt"),
        json!({
            "sheets": [{
                "name": "Sheet1",
                "cells": [{ "ref": "A1", "formula": "=A2", "dataTable": true }]
            }]
        }),
    );
    assert_eq!(e.code(), ErrorCode::InvalidField);
}
