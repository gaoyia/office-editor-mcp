//! 备注页尺寸：inspect `notesSize`，以及 `setNotesSize` / create `presentation.notesSize`。

use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::ZipArchive;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-pptx-notes-size");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn tmp(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}.pptx"))
}

fn create_deck(tag: &str) -> PathBuf {
    let path = tmp(tag);
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "presentation": {
            "slides": [{ "shapes": [{ "text": "标题" }], "notes": "备注" }]
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

fn inspect_scope(path: &Path, scope: Value) -> Value {
    let req = json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": path.to_string_lossy(),
        "scope": scope
    });
    let RunOutput::Inspect(tree) = run(&req.to_string()).unwrap() else {
        panic!("inspect 应返回树");
    };
    tree
}

fn object_keys(v: &Value) -> Vec<&str> {
    v.as_object().unwrap().keys().map(|s| s.as_str()).collect()
}

fn part(path: &Path, name: &str) -> String {
    let bytes = std::fs::read(path).unwrap();
    let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut file = zip.by_name(name).unwrap();
    let mut out = String::new();
    file.read_to_string(&mut out).unwrap();
    out
}

#[test]
fn inspect_notes_size_after_slide_size() {
    let path = create_deck("keys");
    let tree = inspect_scope(&path, json!({ "notesMaster": true }));
    let keys = object_keys(&tree);
    let slide = keys.iter().position(|k| *k == "slideSize").unwrap();
    let notes = keys.iter().position(|k| *k == "notesSize").unwrap();
    assert_eq!(notes, slide + 1);
    assert_eq!(tree["notesSize"]["cx"], 6858000);
    assert_eq!(tree["notesSize"]["cy"], 9144000);
    assert_eq!(object_keys(&tree["notesSize"]), ["cx", "cy"]);
}

#[test]
fn set_notes_size_writes_notes_sz_and_leaves_sld_sz() {
    let src = create_deck("set");
    let out = tmp("set-out");
    let before = inspect_scope(&src, json!({ "notesMaster": true }));
    let ph = &before["notesMaster"]["placeholders"][0];
    let x0 = ph["x"].clone();
    let y0 = ph["y"].clone();
    let cx0 = ph["cx"].clone();
    let cy0 = ph["cy"].clone();
    let sld_cx = before["slideSize"]["cx"].clone();
    apply(
        &src,
        &out,
        json!([{
            "op": "setNotesSize",
            "id": "pptx:presentation",
            "cx": 9144000,
            "cy": 12192000
        }]),
    )
    .unwrap();
    let tree = inspect_scope(&out, json!({ "notesMaster": true }));
    assert_eq!(tree["notesSize"]["cx"], 9144000);
    assert_eq!(tree["notesSize"]["cy"], 12192000);
    assert_eq!(tree["slideSize"]["cx"], sld_cx);
    let ph = &tree["notesMaster"]["placeholders"][0];
    assert_eq!(ph["x"], x0);
    assert_eq!(ph["y"], y0);
    assert_eq!(ph["cx"], cx0);
    assert_eq!(ph["cy"], cy0);
    let xml = part(&out, "ppt/presentation.xml");
    assert!(xml.contains(r#"<p:notesSz cx="9144000" cy="12192000"/>"#));
    assert!(xml.contains(r#"<p:sldSz cx="12192000" cy="6858000"/>"#));
}

#[test]
fn create_notes_size_writes_presentation() {
    let path = tmp("create");
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "presentation": {
            "notesSize": { "cx": 9144000, "cy": 12192000 },
            "slides": [{ "shapes": [{ "text": "标题" }] }]
        }
    })
    .to_string())
    .unwrap();
    let tree = inspect_scope(&path, json!({}));
    assert_eq!(tree["notesSize"]["cx"], 9144000);
    assert_eq!(tree["notesSize"]["cy"], 12192000);
}

#[test]
fn reject_notes_size_ids_and_other_formats() {
    let src = create_deck("bad");
    let out = tmp("page");
    let err = apply(
        &src,
        &out,
        json!([{ "op": "setNotesSize", "id": "slide:1", "cx": 9144000, "cy": 12192000 }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    assert!(!out.exists());

    let out = tmp("missing-cy");
    let err = apply(
        &src,
        &out,
        json!([{ "op": "setNotesSize", "id": "pptx:presentation", "cx": 9144000 }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());

    let out = tmp("range");
    let err = apply(
        &src,
        &out,
        json!([{ "op": "setNotesSize", "id": "pptx:presentation", "cx": 1, "cy": 12192000 }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());

    let xlsx = tmp_dir().join("nsz.xlsx");
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": xlsx.to_string_lossy(),
        "workbook": { "sheets": [{ "name": "Sheet1", "cells": [{ "ref": "A1", "value": 1 }] }] }
    })
    .to_string())
    .unwrap();
    let out = tmp_dir().join("xlsx-nsz.xlsx");
    let err = apply(
        &xlsx,
        &out,
        json!([{ "op": "setNotesSize", "id": "pptx:presentation", "cx": 9144000, "cy": 12192000 }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());

    let docx = tmp_dir().join("nsz.docx");
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": docx.to_string_lossy(),
        "document": { "paragraphs": [{ "text": "甲" }] }
    })
    .to_string())
    .unwrap();
    let out = tmp_dir().join("docx-nsz.docx");
    let err = apply(
        &docx,
        &out,
        json!([{ "op": "setNotesSize", "id": "pptx:presentation", "cx": 9144000, "cy": 12192000 }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());
}
