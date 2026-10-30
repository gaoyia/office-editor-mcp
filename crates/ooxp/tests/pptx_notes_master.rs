//! 备注母版与讲义形状：inspect `scope.notesMaster` / 讲义 `placeholders`，以及插删占位符与预设动画。

use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::ZipArchive;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-pptx-notes-master");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn tmp(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}"))
}

fn create_deck(tag: &str) -> PathBuf {
    let path = tmp(tag).with_extension("pptx");
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "presentation": {
            "slides": [{ "shapes": [{ "text": "标题" }] }]
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
    let RunOutput::Inspect(tree) = run(&json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": path.to_string_lossy(),
        "scope": scope
    })
    .to_string())
    .unwrap() else {
        panic!("inspect 应返回树");
    };
    tree
}

fn inspect_err(path: &Path, scope: Value) -> ooxp::Error {
    run(&json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": path.to_string_lossy(),
        "scope": scope
    })
    .to_string())
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

fn ph_by_text(tree: &Value, host: &str, text: &str) -> Value {
    tree[host]["placeholders"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["text"] == text)
        .cloned()
        .unwrap_or_else(|| panic!("{host} 上找不到 text={text}"))
}

#[test]
fn create_has_no_notes_master_without_scope() {
    let path = create_deck("bare");
    let tree = inspect_scope(&path, json!({}));
    assert!(tree.get("notesMaster").is_none());
}

#[test]
fn scope_notes_master_is_null_on_create() {
    let path = create_deck("scope");
    let tree = inspect_scope(&path, json!({ "notesMaster": true }));
    assert!(tree["notesMaster"].is_null());
    let keys = object_keys(&tree);
    assert!(keys.contains(&"notesMaster"));
    assert!(
        keys.iter().position(|k| *k == "notesMaster").unwrap()
            < keys.iter().position(|k| *k == "slides").unwrap()
    );
}

#[test]
fn insert_placeholder_on_notes_master() {
    let src = create_deck("ins");
    let out = tmp("ins-out").with_extension("pptx");
    apply(
        &src,
        &out,
        json!([{
            "op": "insertPlaceholder",
            "id": "notesMaster",
            "type": "body",
            "text": "母版备注"
        }]),
    )
    .unwrap();
    let tree = inspect_scope(&out, json!({ "notesMaster": true }));
    assert_eq!(tree["notesMaster"]["id"], "notesMaster");
    assert_eq!(
        object_keys(&tree["notesMaster"]),
        ["id", "theme", "clrMap", "animCount", "placeholders"]
    );
    assert_eq!(tree["notesMaster"]["animCount"], 0);
    let ph = ph_by_text(&tree, "notesMaster", "母版备注");
    assert!(ph["id"].as_str().unwrap().starts_with("notesMaster/shape:"));
    assert_eq!(ph["type"], "body");
    let xml = part_text(&out, "ppt/notesMasters/notesMaster1.xml");
    assert!(xml.contains("母版备注"));
    assert!(xml.contains(r#"type="body""#));
}

#[test]
fn set_text_and_delete_notes_master_placeholder() {
    let src = create_deck("edit");
    let with = tmp("ins").with_extension("pptx");
    apply(
        &src,
        &with,
        json!([{
            "op": "insertPlaceholder",
            "id": "notesMaster",
            "type": "body",
            "text": "母版备注"
        }]),
    )
    .unwrap();
    let tree = inspect_scope(&with, json!({ "notesMaster": true }));
    let id = ph_by_text(&tree, "notesMaster", "母版备注")["id"]
        .as_str()
        .unwrap()
        .to_string();
    let edited = tmp("set").with_extension("pptx");
    apply(
        &with,
        &edited,
        json!([{ "op": "setShapeText", "id": id, "text": "改过" }]),
    )
    .unwrap();
    let tree = inspect_scope(&edited, json!({ "notesMaster": true }));
    assert_eq!(ph_by_text(&tree, "notesMaster", "改过")["id"], id);
    let gone = tmp("del").with_extension("pptx");
    apply(
        &edited,
        &gone,
        json!([{ "op": "deletePlaceholder", "id": id }]),
    )
    .unwrap();
    let tree = inspect_scope(&gone, json!({ "notesMaster": true }));
    assert!(tree["notesMaster"]["placeholders"]
        .as_array()
        .unwrap()
        .iter()
        .all(|p| p["id"] != id));
}

#[test]
fn insert_anim_on_notes_master() {
    let src = create_deck("anim");
    let with = tmp("ph").with_extension("pptx");
    apply(
        &src,
        &with,
        json!([{
            "op": "insertPlaceholder",
            "id": "notesMaster",
            "type": "body",
            "text": "母版备注"
        }]),
    )
    .unwrap();
    let tree = inspect_scope(&with, json!({ "notesMaster": true }));
    let id = ph_by_text(&tree, "notesMaster", "母版备注")["id"]
        .as_str()
        .unwrap()
        .to_string();
    let out = tmp("anim-out").with_extension("pptx");
    apply(
        &with,
        &out,
        json!([{ "op": "insertAnim", "id": id, "type": "fade" }]),
    )
    .unwrap();
    let tree = inspect_scope(&out, json!({ "notesMaster": true, "timing": true }));
    assert_eq!(tree["animCount"], 0);
    assert_eq!(tree["notesMaster"]["animCount"], 1);
    assert_eq!(tree["notesMaster"]["timing"][0]["id"], "notesMaster/anim:1");
    assert_eq!(tree["notesMaster"]["timing"][0]["target"], id);
    let xml = part_text(&out, "ppt/notesMasters/notesMaster1.xml");
    assert!(xml.contains("<p:timing"));
    assert!(xml.contains(r#"folHlink="folHlink"/><p:timing"#));
    assert!(xml.contains("</p:timing><p:notesStyle"));
}

#[test]
fn insert_placeholder_on_handout_and_anim() {
    let src = create_deck("ho");
    let with = tmp("hdr").with_extension("pptx");
    apply(
        &src,
        &with,
        json!([{
            "op": "insertPlaceholder",
            "id": "handout",
            "type": "hdr",
            "text": "讲义形"
        }]),
    )
    .unwrap();
    let tree = inspect_scope(&with, json!({ "handout": true }));
    assert_eq!(
        object_keys(&tree["handout"]),
        [
            "id",
            "theme",
            "clrMap",
            "header",
            "footer",
            "date",
            "sldNum",
            "animCount",
            "placeholders"
        ]
    );
    let ph = ph_by_text(&tree, "handout", "讲义形");
    assert!(ph["id"].as_str().unwrap().starts_with("handout/shape:"));
    assert_eq!(ph["type"], "hdr");
    let first_hdr = tree["handout"]["placeholders"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["type"] == "hdr")
        .unwrap();
    let first_id = first_hdr["id"].as_str().unwrap().to_string();
    let id = ph["id"].as_str().unwrap().to_string();
    let out = tmp("ho-out").with_extension("pptx");
    apply(
        &with,
        &out,
        json!([
            { "op": "setShapeText", "id": first_id, "text": "讲义形" },
            { "op": "insertAnim", "id": id, "type": "fade" },
            { "op": "setHandoutText", "id": "handout", "footer": "槽位页脚" }
        ]),
    )
    .unwrap();
    let tree = inspect_scope(&out, json!({ "handout": true, "timing": true }));
    assert_eq!(tree["handout"]["header"], "讲义形");
    assert_eq!(tree["handout"]["footer"], "槽位页脚");
    assert_eq!(tree["animCount"], 0);
    assert_eq!(tree["handout"]["animCount"], 1);
    assert_eq!(tree["handout"]["timing"][0]["id"], "handout/anim:1");
    assert_eq!(tree["handout"]["timing"][0]["target"], id);
    let xml = part_text(&out, "ppt/handoutMasters/handoutMaster1.xml");
    assert!(xml.contains("<p:timing"));
    assert!(xml.contains(r#"folHlink="folHlink"/><p:timing"#));
    assert!(xml.contains("</p:timing><p:hf"));
}

#[test]
fn sld_img_on_notes_master_is_invalid_field() {
    let src = create_deck("sldimg");
    let with = tmp("ins").with_extension("pptx");
    apply(
        &src,
        &with,
        json!([{
            "op": "insertPlaceholder",
            "id": "notesMaster",
            "type": "body",
            "text": "母版备注"
        }]),
    )
    .unwrap();
    let tree = inspect_scope(&with, json!({ "notesMaster": true }));
    let sld = tree["notesMaster"]["placeholders"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["type"] == "sldImg")
        .unwrap();
    let id = sld["id"].as_str().unwrap();
    let out = tmp("sldimg-out").with_extension("pptx");
    let err = apply(
        &with,
        &out,
        json!([{ "op": "setShapeText", "id": id, "text": "图" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());
}

#[test]
fn reject_bad_hosts() {
    let src = create_deck("bad");
    let with = tmp("ho").with_extension("pptx");
    apply(
        &src,
        &with,
        json!([{
            "op": "insertPlaceholder",
            "id": "handout",
            "type": "hdr",
            "text": "讲义形"
        }]),
    )
    .unwrap();
    let tree = inspect_scope(&with, json!({ "handout": true }));
    let ph = ph_by_text(&tree, "handout", "讲义形")["id"]
        .as_str()
        .unwrap()
        .to_string();

    let out = tmp("ins-shape").with_extension("pptx");
    let err = apply(
        &src,
        &out,
        json!([{ "op": "insertShape", "id": "handout", "text": "甲" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    assert!(!out.exists());

    let err = apply(&with, &out, json!([{ "op": "deleteShape", "id": ph }])).unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    assert!(!out.exists());

    let err = apply(
        &src,
        &out,
        json!([{ "op": "insertPlaceholder", "id": "pptx:presentation", "type": "body" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    assert!(!out.exists());

    let xlsx = tmp("scope").with_extension("xlsx");
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": xlsx.to_string_lossy(),
        "workbook": { "sheets": [{ "name": "Sheet1", "cells": [{ "ref": "A1", "value": 1 }] }] }
    })
    .to_string())
    .unwrap();
    let err = inspect_err(&xlsx, json!({ "notesMaster": true }));
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let docx = tmp("scope").with_extension("docx");
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": docx.to_string_lossy(),
        "document": { "paragraphs": [{ "text": "甲" }] }
    })
    .to_string())
    .unwrap();
    let err = inspect_err(&docx, json!({ "notesMaster": true }));
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let bad = tmp("create-nm").with_extension("pptx");
    let err = run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": bad.to_string_lossy(),
        "presentation": {
            "notesMaster": {},
            "slides": [{ "shapes": [{ "text": "甲" }] }]
        }
    })
    .to_string())
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!bad.exists());
}
