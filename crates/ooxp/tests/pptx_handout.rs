//! 讲义母版：inspect `hasHandout` / `scope.handout`，以及 `setHandoutText`。

use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::ZipArchive;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-pptx-handout");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn tmp(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}"))
}

fn create_deck(tag: &str) -> PathBuf {
    let path = tmp(tag).with_extension("pptx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "presentation": {
                "slides": [
                    { "shapes": [{ "text": "标题" }] },
                    { "shapes": [{ "text": "第二页" }] }
                ]
            }
        })
        .to_string(),
        None,
    )
    .unwrap();
    path
}

fn apply(src: &Path, out: &Path, ops: Value) -> Result<RunOutput, ooxp::Error> {
    run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": src.to_string_lossy(),
            "output": out.to_string_lossy(),
            "ops": ops,
        })
        .to_string(),
        None,
    )
}

fn inspect_scope(path: &Path, scope: Option<Value>) -> Value {
    let mut req = json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": path.to_string_lossy()
    });
    if let Some(scope) = scope {
        req["scope"] = scope;
    }
    let RunOutput::Inspect(tree) = run(&req.to_string(), None).unwrap() else {
        panic!("inspect 应返回树");
    };
    tree
}

fn inspect_err(path: &Path, scope: Value) -> ooxp::Error {
    run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": path.to_string_lossy(),
            "scope": scope
        })
        .to_string(),
        None,
    )
    .unwrap_err()
}

fn object_keys(v: &Value) -> Vec<&str> {
    v.as_object().unwrap().keys().map(String::as_str).collect()
}

fn part_names(path: &Path) -> Vec<String> {
    let bytes = std::fs::read(path).unwrap();
    let zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    zip.file_names().map(str::to_string).collect()
}

fn part_text(path: &Path, name: &str) -> String {
    let bytes = std::fs::read(path).unwrap();
    let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut file = zip.by_name(name).unwrap();
    let mut out = Vec::new();
    file.read_to_end(&mut out).unwrap();
    String::from_utf8(out).unwrap()
}

#[test]
fn create_has_no_handout_without_scope() {
    let path = create_deck("bare");
    let tree = inspect_scope(&path, None);
    assert_eq!(tree["hasHandout"], false);
    assert!(tree.get("handout").is_none());
    assert_eq!(
        object_keys(&tree),
        [
            "schemaVersion",
            "format",
            "id",
            "slideCount",
            "pictureCount",
            "mathCount",
            "videoCount",
            "audioCount",
            "chartCount",
            "smartArtCount",
            "commentCount",
            "hyperlinkCount",
            "fieldCount",
            "tableCount",
            "oleObjectCount",
            "model3dCount",
            "zoomCount",
            "connectorCount",
            "animCount",
            "masterCount",
            "hasHandout",
            "sectionCount",
            "slideSize",
            "customShowCount",
            "showPr",
            "modifyProtection",
            "props",
            "slides"
        ]
    );
}

#[test]
fn scope_handout_is_null_on_create() {
    let path = create_deck("scope");
    let tree = inspect_scope(&path, Some(json!({ "handout": true })));
    assert_eq!(tree["hasHandout"], false);
    assert!(tree["handout"].is_null());
}

#[test]
fn set_handout_text_creates_and_sets_slots() {
    let src = create_deck("set");
    let out = tmp("set-out").with_extension("pptx");
    apply(
        &src,
        &out,
        json!([{
            "op": "setHandoutText",
            "id": "handout",
            "header": "讲义页眉",
            "footer": "讲义页脚"
        }]),
    )
    .unwrap();
    let tree = inspect_scope(&out, Some(json!({ "handout": true })));
    assert_eq!(tree["hasHandout"], true);
    assert_eq!(tree["handout"]["id"], "handout");
    assert_eq!(tree["handout"]["header"], "讲义页眉");
    assert_eq!(tree["handout"]["footer"], "讲义页脚");
    assert_eq!(
        object_keys(&tree["handout"]),
        [
            "id",
            "header",
            "footer",
            "date",
            "sldNum",
            "animCount",
            "placeholders"
        ]
    );
    let names = part_names(&out);
    assert!(names.iter().any(|n| n.contains("handoutMaster")));
    let xml = part_text(&out, "ppt/handoutMasters/handoutMaster1.xml");
    assert!(xml.contains("讲义页脚"));
    assert!(!xml.contains("sldImg"));
    assert!(!xml.contains("creationId"));
    let pres = part_text(&out, "ppt/presentation.xml");
    assert!(pres.contains("handoutMasterIdLst"));
    let rels = part_text(&out, "ppt/handoutMasters/_rels/handoutMaster1.xml.rels");
    assert!(rels.contains("/theme"));
    assert!(
        names.iter().any(|n| n == "ppt/theme/theme2.xml"),
        "讲义母版必须有独立 theme 部件，不能和 slideMaster 共用 theme1"
    );
    assert!(rels.contains("theme2.xml"));
    assert_eq!(tree["handout"]["sldNum"], "‹#›");
}

#[test]
fn set_handout_text_keeps_unmentioned_slot() {
    let src = create_deck("keep");
    let mid = tmp("keep-mid").with_extension("pptx");
    apply(
        &src,
        &mid,
        json!([{ "op": "setHandoutText", "id": "handout", "header": "甲", "footer": "乙" }]),
    )
    .unwrap();
    let out = tmp("keep-out").with_extension("pptx");
    apply(
        &mid,
        &out,
        json!([{ "op": "setHandoutText", "id": "handout", "footer": "丙" }]),
    )
    .unwrap();
    let tree = inspect_scope(&out, Some(json!({ "handout": true })));
    assert_eq!(tree["handout"]["header"], "甲");
    assert_eq!(tree["handout"]["footer"], "丙");
}

#[test]
fn date_slot_includes_field_cache_then_set_replaces() {
    let src = create_deck("date");
    let out = tmp("date-out").with_extension("pptx");
    apply(
        &src,
        &out,
        json!([{ "op": "setHandoutText", "id": "handout", "date": "2026-09-02" }]),
    )
    .unwrap();
    let tree = inspect_scope(&out, Some(json!({ "handout": true })));
    assert_eq!(tree["handout"]["date"], "2026-09-02");
}

#[test]
fn bad_id_and_missing_slot_are_invalid() {
    let src = create_deck("bad");
    let out = tmp("bad-id").with_extension("pptx");
    let err = apply(
        &src,
        &out,
        json!([{ "op": "setHandoutText", "id": "slide:1", "footer": "甲" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    assert!(!out.exists());
    let err = apply(
        &src,
        &out,
        json!([{ "op": "setHandoutText", "id": "handout" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());
    let err = apply(
        &src,
        &out,
        json!([{ "op": "setShapeText", "id": "handout", "text": "甲" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    assert!(!out.exists());
}

#[test]
fn create_rejects_presentation_handout() {
    let bad = tmp("create-handout").with_extension("pptx");
    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": bad.to_string_lossy(),
            "presentation": {
                "handout": {},
                "slides": [{ "shapes": [{ "text": "甲" }] }]
            }
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!bad.exists());
}

#[test]
fn xlsx_and_docx_reject_handout() {
    let xlsx = tmp_dir().join("scope.xlsx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": xlsx.to_string_lossy(),
            "workbook": { "sheets": [{ "name": "Sheet1", "cells": [{ "ref": "A1", "value": 1 }] }] }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let err = inspect_err(&xlsx, json!({ "handout": true }));
    assert_eq!(err.code(), ErrorCode::InvalidField);
    let out = tmp_dir().join("xlsx-handout.xlsx");
    let err = apply(
        &xlsx,
        &out,
        json!([{ "op": "setHandoutText", "id": "handout", "footer": "甲" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());

    let docx = tmp_dir().join("scope.docx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": docx.to_string_lossy(),
            "document": { "paragraphs": [{ "text": "甲" }] }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let err = inspect_err(&docx, json!({ "handout": true }));
    assert_eq!(err.code(), ErrorCode::InvalidField);
    let out = tmp_dir().join("docx-handout.docx");
    let err = apply(
        &docx,
        &out,
        json!([{ "op": "setHandoutText", "id": "handout", "footer": "甲" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());
}
