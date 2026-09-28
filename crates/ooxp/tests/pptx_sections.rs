//! 幻灯片节：inspect `sectionCount` / `scope.sections`，以及四条节 op。

use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::ZipArchive;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-pptx-sections");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn tmp(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}"))
}

fn create_deck(tag: &str, n: usize) -> PathBuf {
    let path = tmp(tag).with_extension("pptx");
    let slides: Vec<Value> = (1..=n)
        .map(|i| json!({ "shapes": [{ "text": format!("页{i}") }] }))
        .collect();
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "presentation": { "slides": slides }
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

fn part_text(path: &Path, name: &str) -> String {
    let bytes = std::fs::read(path).unwrap();
    let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut file = zip.by_name(name).unwrap();
    let mut out = Vec::new();
    file.read_to_end(&mut out).unwrap();
    String::from_utf8(out).unwrap()
}

fn guid_ok(g: &str) -> bool {
    g.len() == 38 && g.starts_with('{') && g.ends_with('}') && g.matches('-').count() == 4
}

#[test]
fn create_has_zero_section_count_without_list() {
    let path = create_deck("bare", 2);
    let tree = inspect_scope(&path, None);
    assert_eq!(tree["sectionCount"], 0);
    assert!(tree.get("sections").is_none());
    assert_eq!(
        object_keys(&tree),
        [
            "schemaVersion",
            "format",
            "id",
            "slideCount",
            "pictureCount",
            "videoCount",
            "audioCount",
            "chartCount",
            "smartArtCount",
            "commentCount",
            "hyperlinkCount",
            "tableCount",
            "connectorCount",
            "animCount",
            "masterCount",
            "hasHandout",
            "sectionCount",
            "props",
            "slides"
        ]
    );
    let listed = inspect_scope(&path, Some(json!({ "sections": true })));
    assert_eq!(listed["sectionCount"], 0);
    assert_eq!(listed["sections"], json!([]));
    assert!(listed["slides"][0].get("shapes").is_none());
}

#[test]
fn insert_two_sections_then_move_second_slide() {
    let src = create_deck("psc2", 2);
    let out = tmp("psc2").with_extension("pptx");
    apply(
        &src,
        &out,
        json!([
            { "op": "insertSlideSection", "id": "pptx:presentation", "name": "开场" },
            { "op": "insertSlideSection", "id": "pptx:presentation", "name": "内容" },
            { "op": "setSlideSection", "id": "slide:2", "section": "section:2" }
        ]),
    )
    .unwrap();
    let tree = inspect_scope(&out, Some(json!({ "sections": true })));
    assert_eq!(tree["sectionCount"], 2);
    let secs = tree["sections"].as_array().unwrap();
    assert_eq!(object_keys(&secs[0]), ["id", "guid", "name", "slides"]);
    assert_eq!(secs[0]["id"], "section:1");
    assert_eq!(secs[0]["name"], "开场");
    assert_eq!(secs[0]["slides"], json!(["slide:1"]));
    assert_eq!(secs[1]["id"], "section:2");
    assert_eq!(secs[1]["name"], "内容");
    assert_eq!(secs[1]["slides"], json!(["slide:2"]));
    assert!(guid_ok(secs[0]["guid"].as_str().unwrap()));
    assert!(guid_ok(secs[1]["guid"].as_str().unwrap()));
    let xml = part_text(&out, "ppt/presentation.xml");
    assert!(xml.contains("{521415D9-36F7-43E2-AB2F-B90AF26B5E84}"));
    assert!(xml.contains("p14:sectionLst"));
    assert!(xml.contains("xmlns:p14=\"http://schemas.microsoft.com/office/powerpoint/2010/main\""));
}

#[test]
fn start_without_list_splits_leading_unnamed() {
    let src = create_deck("start", 2);
    let out = tmp("start").with_extension("pptx");
    apply(
        &src,
        &out,
        json!([{
            "op": "insertSlideSection",
            "id": "pptx:presentation",
            "name": "内容",
            "start": "slide:2"
        }]),
    )
    .unwrap();
    let tree = inspect_scope(&out, Some(json!({ "sections": true })));
    assert_eq!(tree["sectionCount"], 2);
    assert_eq!(tree["sections"][0]["name"], "");
    assert_eq!(tree["sections"][0]["slides"], json!(["slide:1"]));
    assert_eq!(tree["sections"][1]["name"], "内容");
    assert_eq!(tree["sections"][1]["slides"], json!(["slide:2"]));
}

#[test]
fn rename_and_delete_keep_slides() {
    let src = create_deck("rename", 2);
    let two = tmp("two").with_extension("pptx");
    apply(
        &src,
        &two,
        json!([
            { "op": "insertSlideSection", "id": "pptx:presentation", "name": "开场" },
            { "op": "insertSlideSection", "id": "pptx:presentation", "name": "内容" },
            { "op": "setSlideSection", "id": "slide:2", "section": "section:2" }
        ]),
    )
    .unwrap();
    let renamed = tmp("renamed").with_extension("pptx");
    apply(
        &two,
        &renamed,
        json!([{ "op": "setSlideSectionName", "id": "section:2", "name": "正文" }]),
    )
    .unwrap();
    let tree = inspect_scope(&renamed, Some(json!({ "sections": true })));
    assert_eq!(tree["sections"][1]["name"], "正文");
    assert_eq!(tree["slideCount"], 2);

    let deleted = tmp("deleted").with_extension("pptx");
    apply(
        &renamed,
        &deleted,
        json!([{ "op": "deleteSlideSection", "id": "section:2" }]),
    )
    .unwrap();
    let tree = inspect_scope(&deleted, Some(json!({ "sections": true })));
    assert_eq!(tree["sectionCount"], 1);
    assert_eq!(tree["slideCount"], 2);
    assert_eq!(tree["sections"][0]["name"], "开场");
    assert_eq!(tree["sections"][0]["slides"], json!(["slide:1", "slide:2"]));

    let gone = tmp("gone").with_extension("pptx");
    apply(
        &deleted,
        &gone,
        json!([{ "op": "deleteSlideSection", "id": "section:1" }]),
    )
    .unwrap();
    let tree = inspect_scope(&gone, Some(json!({ "sections": true })));
    assert_eq!(tree["sectionCount"], 0);
    assert_eq!(tree["sections"], json!([]));
    assert_eq!(tree["slideCount"], 2);
    let xml = part_text(&gone, "ppt/presentation.xml");
    assert!(!xml.contains("sectionLst"));
}

#[test]
fn insert_slide_appends_to_last_section() {
    let src = create_deck("ins-sld", 2);
    let with_sec = tmp("with-sec").with_extension("pptx");
    apply(
        &src,
        &with_sec,
        json!([
            { "op": "insertSlideSection", "id": "pptx:presentation", "name": "开场" },
            { "op": "insertSlideSection", "id": "pptx:presentation", "name": "内容" }
        ]),
    )
    .unwrap();
    let out = tmp("ins-sld-out").with_extension("pptx");
    apply(
        &with_sec,
        &out,
        json!([{ "op": "insertSlide", "id": "pptx:presentation" }]),
    )
    .unwrap();
    let tree = inspect_scope(&out, Some(json!({ "sections": true })));
    assert_eq!(tree["slideCount"], 3);
    assert_eq!(tree["sections"][0]["slides"], json!(["slide:1", "slide:2"]));
    assert_eq!(tree["sections"][1]["slides"], json!(["slide:3"]));
}

#[test]
fn empty_name_is_allowed() {
    let src = create_deck("empty-name", 1);
    let out = tmp("empty-name").with_extension("pptx");
    apply(
        &src,
        &out,
        json!([{ "op": "insertSlideSection", "id": "pptx:presentation", "name": "" }]),
    )
    .unwrap();
    let tree = inspect_scope(&out, Some(json!({ "sections": true })));
    assert_eq!(tree["sections"][0]["name"], "");
}

#[test]
fn rejects_bad_id_and_start_with_after() {
    let src = create_deck("bad", 2);
    let out = tmp("bad-id").with_extension("pptx");
    let err = apply(
        &src,
        &out,
        json!([{ "op": "insertSlideSection", "id": "slide:1", "name": "内容" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    assert_eq!(err.field_path(), Some("ops[0].id"));
    assert!(!out.exists());

    let err = apply(
        &src,
        &out,
        json!([{
            "op": "insertSlideSection",
            "id": "pptx:presentation",
            "name": "内容",
            "start": "slide:2",
            "after": "section:1"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());

    let err = apply(
        &src,
        &out,
        json!([{
            "op": "insertSlideSection",
            "id": "pptx:presentation",
            "name": "内容",
            "after": "section:1"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::IdNotFound);
    assert!(!out.exists());

    let err = apply(
        &src,
        &out,
        json!([{ "op": "setSlideSection", "id": "slide:1", "section": "section:1" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());

    let err = apply(
        &src,
        &out,
        json!([{ "op": "insertSection", "id": "pptx:presentation" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

#[test]
fn create_rejects_presentation_sections() {
    let bad = tmp("create-sec").with_extension("pptx");
    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": bad.to_string_lossy(),
            "presentation": {
                "sections": [],
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
fn xlsx_and_docx_reject_sections() {
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
    let err = inspect_err(&xlsx, json!({ "sections": true }));
    assert_eq!(err.code(), ErrorCode::InvalidField);
    let out = tmp_dir().join("xlsx-sec.xlsx");
    let err = apply(
        &xlsx,
        &out,
        json!([{ "op": "insertSlideSection", "id": "pptx:presentation", "name": "内容" }]),
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
    let err = inspect_err(&docx, json!({ "sections": true }));
    assert_eq!(err.code(), ErrorCode::InvalidField);
    let out = tmp_dir().join("docx-sec.docx");
    let err = apply(
        &docx,
        &out,
        json!([{ "op": "insertSlideSection", "id": "pptx:presentation", "name": "内容" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());
}
