//! 备注页 DrawingML 表：名单、插表、改格、删表。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-pptx-notes-tables");
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
                "slides": [{ "shapes": [{ "text": "标题" }] }]
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

#[test]
fn insert_on_notes_lists_and_skips_slide_count() {
    let src = create_deck("ins");
    let out = tmp("ins").with_extension("pptx");
    apply(
        &src,
        &out,
        json!([{
            "op": "insertTable",
            "id": "slide:1/notes",
            "rows": 2,
            "cols": 2,
            "cells": [["甲", "乙"], ["丙", "丁"]]
        }]),
    )
    .unwrap();
    let tree = inspect(&out, json!({ "notes": true, "tables": true }));
    assert_eq!(tree["tableCount"], 0);
    assert_eq!(tree["slides"][0]["tableCount"], 0);
    let notes = &tree["slides"][0]["notes"];
    assert_eq!(notes["tableCount"], 1);
    let id = notes["tables"][0]["id"].as_str().unwrap();
    assert!(id.starts_with("slide:1/notes/table:"), "{id}");
    assert_eq!(notes["tables"][0]["rows"][0]["cells"][0]["text"], "甲");
    let bare = inspect(&out, json!({ "notes": true }));
    assert_eq!(bare["slides"][0]["notes"]["tableCount"], 1);
    assert!(bare["slides"][0]["notes"].get("tables").is_none());
}

#[test]
fn set_cell_insert_row_then_delete() {
    let src = create_deck("mut");
    let with = tmp("with").with_extension("pptx");
    apply(
        &src,
        &with,
        json!([{
            "op": "insertTable",
            "id": "slide:1/notes",
            "rows": 2,
            "cols": 2,
            "cells": [["甲", "乙"], ["丙", "丁"]]
        }]),
    )
    .unwrap();
    let id = inspect(&with, json!({ "notes": true, "tables": true }))["slides"][0]["notes"]
        ["tables"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let cell = format!("{id}/r:1/c:1");
    let edited = tmp("cell").with_extension("pptx");
    apply(
        &with,
        &edited,
        json!([{ "op": "setTableCellText", "id": cell, "text": "改过的正文" }]),
    )
    .unwrap();
    let tree = inspect(&edited, json!({ "notes": true, "tables": true }));
    assert_eq!(
        tree["slides"][0]["notes"]["tables"][0]["rows"][0]["cells"][0]["text"],
        "改过的正文"
    );
    let rowed = tmp("row").with_extension("pptx");
    apply(&edited, &rowed, json!([{ "op": "insertRow", "id": id }])).unwrap();
    assert_eq!(
        inspect(&rowed, json!({ "notes": true, "tables": true }))["slides"][0]["notes"]["tables"]
            [0]["rowCount"],
        3
    );
    let gone = tmp("del").with_extension("pptx");
    apply(&rowed, &gone, json!([{ "op": "deleteTable", "id": id }])).unwrap();
    let after = inspect(&gone, json!({ "notes": true, "tables": true }));
    assert_eq!(after["slides"][0]["notes"]["tableCount"], 0);
    assert_eq!(after["slides"][0]["notes"]["tables"], json!([]));
}

#[test]
fn rejects_wrong_ids_and_xlsx() {
    let src = create_deck("bad");
    let err = apply(
        &src,
        &tmp("shape").with_extension("pptx"),
        json!([{ "op": "insertTable", "id": "slide:1/shape:2", "rows": 1, "cols": 1 }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);

    let with = tmp("tbl").with_extension("pptx");
    apply(
        &src,
        &with,
        json!([{ "op": "insertTable", "id": "slide:1/notes", "rows": 1, "cols": 1 }]),
    )
    .unwrap();
    let tid = inspect(&with, json!({ "notes": true, "tables": true }))["slides"][0]["notes"]
        ["tables"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let err = apply(
        &with,
        &tmp("tid").with_extension("pptx"),
        json!([{ "op": "insertTable", "id": tid, "rows": 1, "cols": 1 }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);

    let xlsx = tmp("book").with_extension("xlsx");
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
    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": xlsx.to_string_lossy(),
            "output": tmp("xlsx").with_extension("xlsx").to_string_lossy(),
            "ops": [{ "op": "insertTable", "id": "slide:1/notes", "rows": 1, "cols": 1 }]
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

#[test]
fn notes_table_grid_widths() {
    let src = create_deck("grid");
    let with = tmp("grid").with_extension("pptx");
    apply(
        &src,
        &with,
        json!([{
            "op": "insertTable",
            "id": "slide:1/notes",
            "rows": 1,
            "cols": 2,
            "cx": 2540000,
            "cy": 635000,
            "cells": [["甲", "乙"]]
        }]),
    )
    .unwrap();
    let tree = inspect(&with, json!({ "notes": true, "tables": true }));
    let t = &tree["slides"][0]["notes"]["tables"][0];
    let id = t["id"].as_str().unwrap().to_string();
    assert_eq!(t["colWidths"], json!([1270000, 1270000]));
    let out = tmp("grid-set").with_extension("pptx");
    apply(
        &with,
        &out,
        json!([
            {
                "op": "setTableColWidths",
                "id": id,
                "cols": [800000, 1740000]
            },
            {
                "op": "setTableTrHeight",
                "id": format!("{id}/row:1"),
                "h": 400000
            }
        ]),
    )
    .unwrap();
    let tree = inspect(&out, json!({ "notes": true, "tables": true }));
    let t = &tree["slides"][0]["notes"]["tables"][0];
    assert_eq!(t["colWidths"], json!([800000, 1740000]));
    assert_eq!(t["rows"][0]["height"], 400000);
    assert_eq!(t["cx"], 2540000);
}
