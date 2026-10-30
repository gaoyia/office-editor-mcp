//! 拆分窗格与自定义视图：计数、名单、插删与按快照写回。

use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::json;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-xlsx-custom-view");
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
            "sheets": [{ "name": "Sheet1", "cells": [{ "ref": "A1", "value": 1 }] }]
        }
    })
    .to_string())
    .unwrap();
    path
}

fn apply(src: &Path, out: &Path, ops: serde_json::Value) -> Result<RunOutput, ooxp::Error> {
    run(&json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": out.to_string_lossy(),
        "ops": ops,
    })
    .to_string())
}

fn inspect(path: &Path, scope: serde_json::Value) -> serde_json::Value {
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

fn object_keys(v: &serde_json::Value) -> Vec<&str> {
    v.as_object().unwrap().keys().map(|k| k.as_str()).collect()
}

fn part_text(path: &Path, name: &str) -> String {
    let f = std::fs::File::open(path).unwrap();
    let mut zip = zip::ZipArchive::new(f).unwrap();
    let mut zf = zip.by_name(name).unwrap();
    let mut s = String::new();
    zf.read_to_string(&mut s).unwrap();
    s
}

#[test]
fn empty_create_has_count_without_list() {
    let path = create_book("empty");
    let tree = inspect(&path, json!({}));
    assert_eq!(tree["customViewCount"], 0);
    assert!(tree.get("customViews").is_none());
    let keys = object_keys(&tree);
    assert!(keys
        .windows(2)
        .any(|w| w == ["externalLinkCount", "customViewCount"]));
    assert!(keys
        .windows(2)
        .any(|w| w == ["customViewCount", "definedNameCount"]));
    let listed = inspect(&path, json!({ "customViews": true }));
    assert_eq!(listed["customViews"], json!([]));
}

#[test]
fn insert_lists_and_writes_xml() {
    let src = create_book("ins");
    let out = tmp("ins-out");
    apply(
        &src,
        &out,
        json!([{
            "op": "insertCustomView",
            "id": "xlsx:workbook",
            "name": "DemoView"
        }]),
    )
    .unwrap();
    let tree = inspect(&out, json!({ "customViews": true }));
    assert_eq!(tree["customViewCount"], 1);
    let item = &tree["customViews"][0];
    assert_eq!(item["id"], "customView:1");
    assert_eq!(item["name"], "DemoView");
    assert_eq!(item["includePrintSettings"], true);
    assert_eq!(item["includeHiddenRowCol"], true);
    assert_eq!(item["activeSheet"], "Sheet1");
    assert_eq!(
        object_keys(item),
        [
            "id",
            "name",
            "includePrintSettings",
            "includeHiddenRowCol",
            "activeSheet"
        ]
    );
    let wb = part_text(&out, "xl/workbook.xml");
    assert!(wb.contains("customWorkbookViews"), "{wb}");
    assert!(wb.contains(r#"name="DemoView""#), "{wb}");
    let sheet = part_text(&out, "xl/worksheets/sheet1.xml");
    assert!(sheet.contains("customSheetViews"), "{sheet}");
    assert!(sheet.contains("pageMargins"), "{sheet}");
}

#[test]
fn delete_removes_view() {
    let src = create_book("del");
    let mid = tmp("del-mid");
    apply(
        &src,
        &mid,
        json!([{
            "op": "insertCustomView",
            "id": "xlsx:workbook",
            "name": "DemoView"
        }]),
    )
    .unwrap();
    let out = tmp("del-out");
    apply(
        &mid,
        &out,
        json!([{ "op": "deleteCustomView", "id": "customView:1" }]),
    )
    .unwrap();
    let tree = inspect(&out, json!({ "customViews": true }));
    assert_eq!(tree["customViewCount"], 0);
    assert_eq!(tree["customViews"], json!([]));
    let wb = part_text(&out, "xl/workbook.xml");
    assert!(!wb.contains("customWorkbookView"), "{wb}");
    let sheet = part_text(&out, "xl/worksheets/sheet1.xml");
    assert!(!sheet.contains("customSheetView"), "{sheet}");
}

#[test]
fn show_restores_freeze() {
    let src = create_book("show");
    let frozen = tmp("show-frozen");
    apply(
        &src,
        &frozen,
        json!([{ "op": "setSheetView", "id": "sheet:Sheet1", "frozenRows": 1 }]),
    )
    .unwrap();
    let with_view = tmp("show-view");
    apply(
        &frozen,
        &with_view,
        json!([{
            "op": "insertCustomView",
            "id": "xlsx:workbook",
            "name": "FrozenView"
        }]),
    )
    .unwrap();
    let unfrozen = tmp("show-unfrozen");
    apply(
        &with_view,
        &unfrozen,
        json!([{ "op": "setSheetView", "id": "sheet:Sheet1", "frozenRows": 0 }]),
    )
    .unwrap();
    assert_eq!(
        inspect(&unfrozen, json!({}))["sheets"][0]["view"]["frozenRows"],
        0
    );
    let shown = tmp("show-shown");
    apply(
        &unfrozen,
        &shown,
        json!([{ "op": "showCustomView", "id": "customView:1" }]),
    )
    .unwrap();
    let v = &inspect(&shown, json!({}))["sheets"][0]["view"];
    assert_eq!(v["frozenRows"], 1);
    assert_eq!(v["splitRows"], 0);
}

#[test]
fn duplicate_and_bad_ids() {
    let src = create_book("bad");
    let first = tmp("bad-first");
    apply(
        &src,
        &first,
        json!([{
            "op": "insertCustomView",
            "id": "xlsx:workbook",
            "name": "DemoView"
        }]),
    )
    .unwrap();
    let dup = tmp("bad-dup");
    let err = apply(
        &first,
        &dup,
        json!([{
            "op": "insertCustomView",
            "id": "xlsx:workbook",
            "name": "demoview"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!dup.exists());

    let empty = tmp("bad-empty");
    let err = apply(
        &src,
        &empty,
        json!([{
            "op": "insertCustomView",
            "id": "xlsx:workbook",
            "name": ""
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!empty.exists());

    let slide = tmp("bad-slide");
    let err = apply(
        &src,
        &slide,
        json!([{
            "op": "insertCustomView",
            "id": "slide:1",
            "name": "DemoView"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    assert!(!slide.exists());

    let miss = tmp("bad-miss");
    let err = apply(
        &src,
        &miss,
        json!([{ "op": "deleteCustomView", "id": "customView:9" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::IdNotFound);
    assert!(!miss.exists());
}

#[test]
fn create_empty_custom_views_is_ok() {
    let path = tmp("create-cv");
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "workbook": {
            "customViews": [],
            "sheets": [{ "name": "Sheet1" }]
        }
    })
    .to_string())
    .unwrap();
    let tree = inspect(&path, json!({ "customViews": true }));
    assert_eq!(tree["customViewCount"], 0);
    assert_eq!(tree["customViews"], json!([]));
}

#[test]
fn pptx_rejects_scope_and_op() {
    let pptx = tmp_dir().join("pptx-cv.pptx");
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": pptx.to_string_lossy(),
        "presentation": { "slides": [{ "shapes": [{ "text": "a" }] }] }
    })
    .to_string())
    .unwrap();
    let err = run(&json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": pptx.to_string_lossy(),
        "scope": { "customViews": true }
    })
    .to_string())
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let out = tmp_dir().join("pptx-cv-out.pptx");
    let err = run(&json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": pptx.to_string_lossy(),
        "output": out.to_string_lossy(),
        "ops": [{ "op": "insertCustomView", "id": "xlsx:workbook", "name": "DemoView" }]
    })
    .to_string())
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());
}
