//! 请求里的非法 XML 字符在写文件之前被拒绝。

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode};
use serde_json::json;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp(ext: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join("ooxp-xml-char");
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(format!("f-{n}.{ext}"))
}

fn assert_rejected(err: ooxp::Error, path: &str, scalar: &str) {
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert_eq!(err.field_path(), Some(path));
    let message = err.to_json(ooxp::Lang::ZhCn)["error"]["message"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(message.contains(scalar), "{message}");
}

#[test]
fn xlsx_create_rejects_a_control_char_and_writes_nothing() {
    let path = tmp("xlsx");
    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path,
            "workbook": { "sheets": [{ "name": "Sheet1", "cells": [{ "ref": "A1", "value": "a\u{0001}b" }] }] }
        })
        .to_string())
    .unwrap_err();
    assert_rejected(err, "workbook.sheets[0].cells[0].value", "U+0001");
    assert!(!path.exists());
}

#[test]
fn docx_create_rejects_vertical_tab() {
    let path = tmp("docx");
    let err = run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path,
        "document": { "paragraphs": [{ "text": "x\u{000B}y" }] }
    })
    .to_string())
    .unwrap_err();
    assert_rejected(err, "document.paragraphs[0].text", "U+000B");
    assert!(!path.exists());
}

#[test]
fn pptx_apply_rejects_a_control_char_without_touching_the_file() {
    let path = tmp("pptx");
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path,
        "presentation": { "slides": [{ "shapes": [{ "text": "标题" }] }] }
    })
    .to_string())
    .unwrap();
    let before = std::fs::read(&path).unwrap();
    let err = run(&json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": path,
        "ops": [{ "op": "setShapeText", "id": "slide:1/shape:2", "text": "a\u{0001}b" }]
    })
    .to_string())
    .unwrap_err();
    assert_rejected(err, "ops[0].text", "U+0001");
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

#[test]
fn illegal_key_is_rejected() {
    let err = run(&json!({
        "schemaVersion": 1,
        "action": "help",
        "\u{0001}": 1
    })
    .to_string())
    .unwrap_err();
    assert_rejected(err, "", "U+0001");
}

#[test]
fn tab_lf_cr_cjk_and_emoji_are_allowed() {
    let path = tmp("xlsx");
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path,
        "workbook": {
            "sheets": [{
                "name": "Sheet1",
                "cells": [{ "ref": "A1", "value": "a\tb\nc\r中文😀" }]
            }]
        }
    })
    .to_string())
    .unwrap();
    assert!(path.exists());
    let _ = std::fs::remove_file(path);
}
