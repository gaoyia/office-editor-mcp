//! 母版写入剩余：插删空白版式、占位符与第二份母版。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::ZipArchive;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-pptx-master-write");
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
            "slides": [
                { "shapes": [{ "text": "标题" }] }
            ]
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

fn inspect_masters(path: &Path) -> Value {
    let RunOutput::Inspect(tree) = run(&json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": path.to_string_lossy(),
        "scope": { "masters": true }
    })
    .to_string())
    .unwrap() else {
        panic!("inspect 应返回树");
    };
    tree
}

fn part(path: &Path, name: &str) -> String {
    let bytes = std::fs::read(path).unwrap();
    let mut zip = ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    let mut file = zip.by_name(name).unwrap();
    let mut out = String::new();
    std::io::Read::read_to_string(&mut file, &mut out).unwrap();
    out
}

#[test]
fn insert_layout_adds_blank() {
    let src = create_deck("layout");
    let out = tmp("layout-out");
    apply(
        &src,
        &out,
        json!([{ "op": "insertLayout", "id": "master:1", "name": "空白 2" }]),
    )
    .unwrap();
    let tree = inspect_masters(&out);
    assert_eq!(tree["masterCount"], 1);
    let layouts = tree["masters"][0]["layouts"].as_array().unwrap();
    assert_eq!(layouts.len(), 2);
    assert_eq!(layouts[1]["id"], "master:1/layout:2");
    assert_eq!(layouts[1]["kind"], "blank");
    assert_eq!(layouts[1]["name"], "空白 2");
    let xml = part(&out, "ppt/slideLayouts/slideLayout2.xml");
    assert!(xml.contains(r#"type="blank""#));
    assert!(xml.contains(r#"preserve="1""#));
    assert!(xml.contains("masterClrMapping"));
}

#[test]
fn insert_and_delete_placeholder_body() {
    let src = create_deck("ph");
    let laid = tmp("ph-layout");
    apply(
        &src,
        &laid,
        json!([{ "op": "insertLayout", "id": "master:1" }]),
    )
    .unwrap();
    let with_ph = tmp("ph-ins");
    apply(
        &laid,
        &with_ph,
        json!([{
            "op": "insertPlaceholder",
            "id": "master:1/layout:2",
            "type": "body",
            "text": "提示"
        }]),
    )
    .unwrap();
    let tree = inspect_masters(&with_ph);
    let phs = tree["masters"][0]["layouts"][1]["placeholders"]
        .as_array()
        .unwrap();
    assert_eq!(phs.len(), 1);
    assert_eq!(phs[0]["type"], "body");
    assert_eq!(phs[0]["idx"], "10");
    assert_eq!(phs[0]["text"], "提示");
    assert!(phs[0]["id"]
        .as_str()
        .unwrap()
        .starts_with("master:1/layout:2/shape:"));
    let xml = part(&with_ph, "ppt/slideLayouts/slideLayout2.xml");
    assert!(xml.contains(r#"type="body""#));
    assert!(xml.contains(r#"sz="quarter""#));
    assert!(xml.contains(r#"idx="10""#));
    assert!(xml.contains("noGrp"));
    assert!(!xml.contains("txBox"));
    assert!(!xml.contains("creationId"));

    let gone = tmp("ph-del");
    apply(
        &with_ph,
        &gone,
        json!([{ "op": "deletePlaceholder", "id": phs[0]["id"] }]),
    )
    .unwrap();
    let tree = inspect_masters(&gone);
    let phs = tree["masters"][0]["layouts"][1]["placeholders"]
        .as_array()
        .unwrap();
    assert!(phs.is_empty());
}

#[test]
fn insert_master_clones_and_occupied_delete_fails() {
    let src = create_deck("master");
    let cloned = tmp("master-clone");
    apply(
        &src,
        &cloned,
        json!([{ "op": "insertMaster", "id": "pptx:presentation" }]),
    )
    .unwrap();
    let tree = inspect_masters(&cloned);
    assert_eq!(tree["masterCount"], 2);
    assert_eq!(tree["masters"][1]["id"], "master:2");
    assert_eq!(tree["masters"][1]["layouts"][0]["kind"], "blank");
    assert_eq!(tree["slides"][0]["layoutId"], "master:1/layout:1");
    let xml = part(&cloned, "ppt/slideMasters/slideMaster2.xml");
    assert!(xml.contains("sldLayoutId"));

    let blocked = tmp("del-layout-used");
    let err = apply(
        &cloned,
        &blocked,
        json!([{ "op": "deleteLayout", "id": "master:1/layout:1" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!blocked.exists());

    let blocked_m = tmp("del-master-used");
    let err = apply(
        &cloned,
        &blocked_m,
        json!([{ "op": "deleteMaster", "id": "master:1" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!blocked_m.exists());

    let dropped = tmp("del-master-unused");
    apply(
        &cloned,
        &dropped,
        json!([{ "op": "deleteMaster", "id": "master:2" }]),
    )
    .unwrap();
    let tree = inspect_masters(&dropped);
    assert_eq!(tree["masterCount"], 1);
}

#[test]
fn last_master_and_last_layout_are_invalid_field() {
    let src = create_deck("last");
    let out = tmp("last-master");
    let err = apply(
        &src,
        &out,
        json!([{ "op": "deleteMaster", "id": "master:1" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());

    let out = tmp("last-layout");
    let err = apply(
        &src,
        &out,
        json!([{ "op": "deleteLayout", "id": "master:1/layout:1" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());
}

#[test]
fn reject_bad_ids_and_types() {
    let src = create_deck("bad");
    let out = tmp("slide-id");
    let err = apply(
        &src,
        &out,
        json!([{ "op": "insertLayout", "id": "slide:1" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    assert!(!out.exists());

    let out = tmp("bad-type");
    let err = apply(
        &src,
        &out,
        json!([{
            "op": "insertPlaceholder",
            "id": "master:1/layout:1",
            "type": "clipArt"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());

    let tree = inspect_masters(&src);
    let ph = tree["masters"][0]["placeholders"].as_array().unwrap();
    assert!(ph.is_empty(), "create 空白母版没有占位符");
    let laid = tmp("ph-for-delshape");
    apply(
        &src,
        &laid,
        json!([
            { "op": "insertLayout", "id": "master:1" },
            { "op": "insertPlaceholder", "id": "master:1/layout:2", "type": "title" }
        ]),
    )
    .unwrap();
    let tree = inspect_masters(&laid);
    let id = tree["masters"][0]["layouts"][1]["placeholders"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let out = tmp("del-shape-ph");
    let err = apply(&laid, &out, json!([{ "op": "deleteShape", "id": id }])).unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    assert!(!out.exists());
}

#[test]
fn delete_unused_layout_ok() {
    let src = create_deck("unused-ly");
    let laid = tmp("two-layouts");
    apply(
        &src,
        &laid,
        json!([{ "op": "insertLayout", "id": "master:1" }]),
    )
    .unwrap();
    let out = tmp("del-unused-ly");
    apply(
        &laid,
        &out,
        json!([{ "op": "deleteLayout", "id": "master:1/layout:2" }]),
    )
    .unwrap();
    let tree = inspect_masters(&out);
    assert_eq!(tree["masters"][0]["layouts"].as_array().unwrap().len(), 1);
    assert_eq!(tree["slides"][0]["layoutId"], "master:1/layout:1");
}

#[test]
fn xlsx_and_docx_reject_master_write_ops() {
    let xlsx = tmp_dir().join("mw.xlsx");
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": xlsx.to_string_lossy(),
        "workbook": { "sheets": [{ "name": "Sheet1", "cells": [{ "ref": "A1", "value": 1 }] }] }
    })
    .to_string())
    .unwrap();
    let out = tmp_dir().join("xlsx-mw.xlsx");
    let err = apply(
        &xlsx,
        &out,
        json!([{ "op": "insertLayout", "id": "master:1" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());

    let docx = tmp_dir().join("mw.docx");
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": docx.to_string_lossy(),
        "document": { "paragraphs": [{ "text": "甲" }] }
    })
    .to_string())
    .unwrap();
    let out = tmp_dir().join("docx-mw.docx");
    let err = apply(
        &docx,
        &out,
        json!([{ "op": "insertMaster", "id": "pptx:presentation" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());
}

#[test]
fn content_chart_picture_and_slide_image_placeholders() {
    let src = create_deck("ph-more");
    let out = tmp("ph-more-out");
    apply(
        &src,
        &out,
        json!([
            { "op": "insertPlaceholder", "id": "master:1/layout:1", "type": "obj" },
            { "op": "insertPlaceholder", "id": "master:1/layout:1", "type": "chart" },
            { "op": "insertPlaceholder", "id": "master:1/layout:1", "type": "tbl" },
            { "op": "insertPlaceholder", "id": "master:1/layout:1", "type": "pic" },
            { "op": "insertPlaceholder", "id": "master:1/layout:1", "type": "media" },
            { "op": "insertPlaceholder", "id": "notesMaster", "type": "sldImg" }
        ]),
    )
    .unwrap();
    let tree = inspect_masters(&out);
    let types: Vec<_> = tree["masters"][0]["layouts"][0]["placeholders"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["type"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(types, ["obj", "chart", "tbl", "pic", "media"]);
    let xml = part(&out, "ppt/slideLayouts/slideLayout1.xml");
    assert!(xml.contains(r#"type="obj""#), "{xml}");
    assert!(xml.contains("单击此处编辑母版文本样式"));
    assert!(xml.contains(r#"type="chart""#));
    assert!(xml.contains(r#"sz="quarter""#));
    assert!(xml.contains(r#"type="pic""#));
    assert!(xml.contains(r#"noChangeAspect="1""#));
    assert!(!xml.contains("graphicFrame"));
    let err = apply(
        &out,
        &tmp("sldimg-layout"),
        json!([{ "op": "insertPlaceholder", "id": "master:1/layout:1", "type": "sldImg" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    let notes = run(&json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": out.to_string_lossy(),
        "scope": { "notesMaster": true }
    })
    .to_string())
    .unwrap();
    let RunOutput::Inspect(notes) = notes else {
        panic!("inspect");
    };
    let ph = notes["notesMaster"]["placeholders"].as_array().unwrap();
    assert!(ph.iter().any(|p| p["type"] == "sldImg"));
    let notes_xml = part(&out, "ppt/notesMasters/notesMaster1.xml");
    assert!(notes_xml.contains(r#"type="sldImg""#));
    assert!(notes_xml.contains(r#"noRot="1""#));
}
