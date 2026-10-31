//! pptx 位置序号：先删或先插，再改原来的第 n 个。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, RunOutput};
use serde_json::{json, Value};

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-pptx-id-pin");
    std::fs::create_dir_all(&dir).unwrap();
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    dir.join(format!("{tag}-{n}.pptx"))
}

fn create_deck(tag: &str) -> PathBuf {
    let path = tmp(tag);
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "presentation": {
            "slides": [
                { "shapes": [{ "text": "第一页" }] },
                { "shapes": [{ "text": "第二页" }] }
            ]
        }
    })
    .to_string())
    .unwrap();
    path
}

fn apply(src: &Path, out: &Path, ops: Value) {
    run(&json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": out.to_string_lossy(),
        "ops": ops,
    })
    .to_string())
    .unwrap();
}

fn inspect(path: &Path, scope: Value) -> Value {
    let RunOutput::Inspect(tree) = run(&json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": path.to_string_lossy(),
        "scope": scope
    })
    .to_string())
    .unwrap() else {
        panic!("inspect");
    };
    tree
}

#[test]
fn insert_row_then_write_original_third() {
    let src = create_deck("row");
    let mid = tmp("row-mid");
    apply(
        &src,
        &mid,
        json!([{
            "op": "insertTable",
            "id": "slide:1",
            "rows": 3,
            "cols": 3,
            "x": 914400,
            "y": 1828800,
            "cx": 5486400,
            "cy": 3200400,
            "cells": [["a1", "a2", "a3"], ["b1", "b2", "b3"], ["c1", "c2", "c3"]]
        }]),
    );
    let tree = inspect(&mid, json!({ "tables": true }));
    let table_id = tree["slides"][0]["tables"][0]["id"].as_str().unwrap();
    let out = tmp("row-out");
    apply(
        &mid,
        &out,
        json!([
            { "op": "insertRow", "id": format!("{table_id}/row:1") },
            { "op": "setTableCellText", "id": format!("{table_id}/r:3/c:1"), "text": "原三" }
        ]),
    );
    let tree = inspect(&out, json!({ "tables": true }));
    let rows = tree["slides"][0]["tables"][0]["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 4);
    let text = |row: usize, col: usize| rows[row]["cells"][col]["text"].as_str().unwrap();
    assert_eq!(text(0, 0), "a1");
    assert_eq!(text(2, 0), "b1");
    assert_eq!(text(3, 0), "原三");
    assert_eq!(text(3, 1), "c2");
}

#[test]
fn delete_col_then_write_original_third() {
    let src = create_deck("col");
    let mid = tmp("col-mid");
    apply(
        &src,
        &mid,
        json!([{
            "op": "insertTable",
            "id": "slide:1",
            "rows": 1,
            "cols": 3,
            "x": 914400,
            "y": 1828800,
            "cx": 5486400,
            "cy": 800000,
            "cells": [["a1", "a2", "a3"]]
        }]),
    );
    let tree = inspect(&mid, json!({ "tables": true }));
    let table_id = tree["slides"][0]["tables"][0]["id"].as_str().unwrap();
    let out = tmp("col-out");
    apply(
        &mid,
        &out,
        json!([
            { "op": "deleteCol", "id": table_id, "col": 1 },
            { "op": "setTableCellText", "id": format!("{table_id}/r:1/c:3"), "text": "原三列" }
        ]),
    );
    let tree = inspect(&out, json!({ "tables": true }));
    let cells = &tree["slides"][0]["tables"][0]["rows"][0]["cells"];
    assert_eq!(cells.as_array().unwrap().len(), 2);
    assert_eq!(cells[0]["text"], "a2");
    assert_eq!(cells[1]["text"], "原三列");
}

#[test]
fn delete_section_then_rename_original_third() {
    let src = create_deck("sec");
    let mid = tmp("sec-mid");
    apply(
        &src,
        &mid,
        json!([
            { "op": "insertSlideSection", "id": "pptx:presentation", "name": "甲" },
            { "op": "insertSlideSection", "id": "pptx:presentation", "name": "乙" },
            { "op": "insertSlideSection", "id": "pptx:presentation", "name": "丙" }
        ]),
    );
    let out = tmp("sec-out");
    apply(
        &mid,
        &out,
        json!([
            { "op": "deleteSlideSection", "id": "section:1" },
            { "op": "setSlideSectionName", "id": "section:3", "name": "丙改" }
        ]),
    );
    let tree = inspect(&out, json!({ "sections": true }));
    let secs = tree["sections"].as_array().unwrap();
    assert_eq!(secs.len(), 2);
    let names: Vec<_> = secs.iter().map(|s| s["name"].as_str().unwrap()).collect();
    assert!(names.contains(&"乙"), "{names:?}");
    assert!(names.contains(&"丙改"), "{names:?}");
    assert!(!names.contains(&"甲"), "{names:?}");
}

#[test]
fn delete_show_then_rename_original_second() {
    let src = create_deck("show");
    let mid = tmp("show-mid");
    apply(
        &src,
        &mid,
        json!([
            {
                "op": "insertCustomShow",
                "id": "pptx:presentation",
                "name": "甲",
                "slides": ["slide:1"]
            },
            {
                "op": "insertCustomShow",
                "id": "pptx:presentation",
                "name": "乙",
                "slides": ["slide:2"]
            }
        ]),
    );
    let out = tmp("show-out");
    apply(
        &mid,
        &out,
        json!([
            { "op": "deleteCustomShow", "id": "show:1" },
            { "op": "setCustomShow", "id": "show:2", "name": "乙改" }
        ]),
    );
    let tree = inspect(&out, json!({ "customShows": true }));
    let shows = tree["customShows"].as_array().unwrap();
    assert_eq!(shows.len(), 1);
    assert_eq!(shows[0]["name"], "乙改");
}

#[test]
fn delete_master_then_edit_original_second() {
    let src = create_deck("master");
    let cloned = tmp("master-clone");
    apply(
        &src,
        &cloned,
        json!([{ "op": "insertMaster", "id": "pptx:presentation" }]),
    );
    let shaped = tmp("master-shaped");
    apply(
        &cloned,
        &shaped,
        json!([{
            "op": "insertShape",
            "id": "master:2",
            "text": "母版二",
            "x": 914400,
            "y": 914400,
            "cx": 1828800,
            "cy": 914400
        }]),
    );
    let before = inspect(&shaped, json!({ "masters": true }));
    let shape_id = before["masters"][1]["shapes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|shape| shape["text"] == "母版二")
        .map(|shape| shape["id"].as_str().unwrap().to_string())
        .unwrap_or_else(|| panic!("没有母版二的形状: {}", before["masters"][1]["shapes"]));
    let moved = tmp("master-moved");
    apply(
        &shaped,
        &moved,
        json!([
            { "op": "setSlideLayout", "id": "slide:1", "layout": "master:2/layout:1" },
            { "op": "setSlideLayout", "id": "slide:2", "layout": "master:2/layout:1" }
        ]),
    );
    let out = tmp("master-out");
    apply(
        &moved,
        &out,
        json!([
            { "op": "deleteMaster", "id": "master:1" },
            { "op": "setShapeText", "id": shape_id, "text": "原二母版" }
        ]),
    );
    let tree = inspect(&out, json!({ "masters": true }));
    assert_eq!(tree["masterCount"], 1);
    let text = tree["masters"][0]["shapes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|shape| shape["text"] == "原二母版");
    assert!(text.is_some(), "{:?}", tree["masters"][0]["shapes"]);
}
