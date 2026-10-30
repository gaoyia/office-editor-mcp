//! 工作表文本框 / 形状的 DrawingML 文字格式。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-xlsx-shape-format");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn tmp(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}.xlsx"))
}

fn create_book(tag: &str) -> PathBuf {
    let path = tmp(tag);
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "workbook": {
            "sheets": [{
                "name": "Sheet1",
                "cells": [{ "ref": "A1", "value": 1 }]
            }]
        }
    })
    .to_string())
    .unwrap();
    path
}

fn apply(src: &Path, out: &Path, ops: Value) -> Result<RunOutput, ooxp::Error> {
    run(&json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": out.to_string_lossy(),
        "ops": ops,
    })
    .to_string())
}

fn inspect_boxes(path: &Path) -> Value {
    let RunOutput::Inspect(tree) = run(&json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": path.to_string_lossy(),
        "scope": { "textBoxes": true, "shapes": true }
    })
    .to_string())
    .unwrap() else {
        panic!("inspect 应返回树");
    };
    tree
}

#[test]
fn insert_textbox_then_set_format() {
    let src = create_book("box");
    let inserted = tmp("box-ins");
    apply(
        &src,
        &inserted,
        json!([{
            "op": "insertTextBox",
            "id": "sheet:Sheet1",
            "from": "B2",
            "text": "你好",
            "sz": 2400,
            "color": "0000FF"
        }]),
    )
    .unwrap();
    let tree = inspect_boxes(&inserted);
    let box0 = &tree["sheets"][0]["textBoxes"][0];
    assert_eq!(box0["text"], "你好");
    assert_eq!(box0["sz"], 2400);
    assert_eq!(box0["color"], "0000FF");
    let id = box0["id"].as_str().unwrap().to_string();

    let out = tmp("box-fmt");
    apply(
        &inserted,
        &out,
        json!([{
            "op": "setShapeFormat",
            "id": id,
            "sz": 1800,
            "color": "C41E3A",
            "bold": true,
            "align": "ctr"
        }]),
    )
    .unwrap();
    let after = &inspect_boxes(&out)["sheets"][0]["textBoxes"][0];
    assert_eq!(after["sz"], 1800);
    assert_eq!(after["color"], "C41E3A");
    assert_eq!(after["bold"], true);
    assert_eq!(after["align"], "ctr");
}

#[test]
fn rejects_empty_format_and_anchor_name() {
    let src = create_book("bad");
    let inserted = tmp("bad-ins");
    apply(
        &src,
        &inserted,
        json!([{
            "op": "insertTextBox",
            "id": "sheet:Sheet1",
            "from": "B2",
            "text": "x"
        }]),
    )
    .unwrap();
    let id = inspect_boxes(&inserted)["sheets"][0]["textBoxes"][0]["id"].clone();
    let dest = tmp("no-field");
    let err = apply(
        &inserted,
        &dest,
        json!([{ "op": "setShapeFormat", "id": id }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!dest.exists());
}

#[test]
fn set_textbox_spacing_and_kern() {
    let src = create_book("spc");
    let inserted = tmp("spc-ins");
    apply(
        &src,
        &inserted,
        json!([{
            "op": "insertTextBox",
            "id": "sheet:Sheet1",
            "from": "B2",
            "text": "你好",
            "lnSpc": { "type": "pts", "val": 2400 },
            "spc": 200
        }]),
    )
    .unwrap();
    let box0 = &inspect_boxes(&inserted)["sheets"][0]["textBoxes"][0];
    assert_eq!(box0["lnSpc"]["type"], "pts");
    assert_eq!(box0["lnSpc"]["val"], 2400);
    assert_eq!(box0["runs"][0]["spc"], 200);
    let id = box0["id"].clone();
    let out = tmp("spc-out");
    apply(
        &inserted,
        &out,
        json!([{
            "op": "setShapeFormat",
            "id": id,
            "spcBef": { "type": "pct", "val": 100000 },
            "kern": 800,
            "textAnchor": "b"
        }]),
    )
    .unwrap();
    let after = &inspect_boxes(&out)["sheets"][0]["textBoxes"][0];
    assert_eq!(after["textAnchor"], "b");
    assert_eq!(after["spcBef"]["type"], "pct");
    assert_eq!(after["spcBef"]["val"], 100000);
    assert_eq!(after["lnSpc"]["val"], 2400);
    assert_eq!(after["runs"][0]["kern"], 800);
}

#[test]
fn rejects_bad_spacing_type() {
    let src = create_book("bad-spc");
    let inserted = tmp("bad-spc-ins");
    apply(
        &src,
        &inserted,
        json!([{
            "op": "insertTextBox",
            "id": "sheet:Sheet1",
            "from": "B2",
            "text": "x"
        }]),
    )
    .unwrap();
    let id = inspect_boxes(&inserted)["sheets"][0]["textBoxes"][0]["id"].clone();
    let dest = tmp("bad-type");
    let err = apply(
        &inserted,
        &dest,
        json!([{
            "op": "setShapeFormat",
            "id": id,
            "lnSpc": { "type": "auto", "val": 0 }
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!dest.exists());
}
