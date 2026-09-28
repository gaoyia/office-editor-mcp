//! 幻灯片 DrawingML 表：名单、改格、插删行列、插表删表。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-pptx-tables");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn tmp(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}.pptx"))
}

fn create_deck(tag: &str) -> PathBuf {
    let path = tmp(tag);
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

fn inspect_bare(path: &Path) -> Value {
    inspect_scope(path, json!({}))
}

fn inspect_tables(path: &Path) -> Value {
    inspect_scope(path, json!({ "tables": true }))
}

fn inspect_scope(path: &Path, scope: Value) -> Value {
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

fn object_keys(v: &Value) -> Vec<&str> {
    v.as_object().unwrap().keys().map(|s| s.as_str()).collect()
}

fn insert_two_by_two(src: &Path, out: &Path) {
    apply(
        src,
        out,
        json!([{
            "op": "insertTable",
            "id": "slide:1",
            "rows": 2,
            "cols": 2,
            "x": 914400,
            "y": 1828800,
            "cx": 5486400,
            "cy": 3200400,
            "cells": [["甲", "乙"], ["丙", "丁"]]
        }]),
    )
    .unwrap();
}

#[test]
fn table_count_without_list() {
    let path = create_deck("count");
    let tree = inspect_bare(&path);
    assert_eq!(tree["tableCount"], 0);
    assert_eq!(tree["slides"][0]["tableCount"], 0);
    assert!(tree["slides"][0].get("tables").is_none());
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
            "slideSize",
            "customShowCount",
            "props",
            "slides"
        ]
    );
}

#[test]
fn tables_scope_does_not_need_slide() {
    let src = create_deck("scope");
    let out = tmp("scope");
    insert_two_by_two(&src, &out);
    let tree = inspect_tables(&out);
    assert!(tree["slides"][0].get("shapes").is_none());
    assert_eq!(tree["tableCount"], 1);
    assert_eq!(tree["slides"][0]["tables"].as_array().unwrap().len(), 1);
}

#[test]
fn insert_inspect_cell_text_row_col_pos_delete() {
    let src = create_deck("round");
    let inserted = tmp("ins");
    insert_two_by_two(&src, &inserted);

    let tree = inspect_tables(&inserted);
    assert_eq!(tree["tableCount"], 1);
    assert_eq!(tree["slides"][0]["tableCount"], 1);
    assert_eq!(tree["slides"][1]["tableCount"], 0);
    let tables = tree["slides"][0]["tables"].as_array().unwrap();
    assert_eq!(tables.len(), 1);
    let t = &tables[0];
    assert_eq!(t["id"], "slide:1/table:3");
    assert_eq!(t["cNvPrId"], "3");
    assert_eq!(t["name"], "表格 3");
    assert_eq!(t["x"], 914400);
    assert_eq!(t["y"], 1828800);
    assert_eq!(t["cx"], 5486400);
    assert_eq!(t["cy"], 3200400);
    assert_eq!(t["rowCount"], 2);
    assert_eq!(t["colCount"], 2);
    assert_eq!(t["stable"], true);
    assert_eq!(
        object_keys(t),
        ["id", "cNvPrId", "name", "x", "y", "cx", "cy", "rowCount", "colCount", "rows", "stable"]
    );
    assert_eq!(t["rows"][0]["id"], "slide:1/table:3/row:1");
    assert_eq!(t["rows"][0]["index"], 1);
    let c = &t["rows"][0]["cells"][0];
    assert_eq!(c["id"], "slide:1/table:3/r:1/c:1");
    assert_eq!(c["row"], 1);
    assert_eq!(c["col"], 1);
    assert_eq!(c["gridSpan"], 1);
    assert_eq!(c["rowSpan"], 1);
    assert_eq!(c["hMerge"], false);
    assert_eq!(c["vMerge"], false);
    assert_eq!(c["text"], "甲");
    assert_eq!(
        object_keys(c),
        ["id", "row", "col", "gridSpan", "rowSpan", "hMerge", "vMerge", "text"]
    );
    assert_eq!(t["rows"][0]["cells"][1]["text"], "乙");
    assert_eq!(t["rows"][1]["cells"][0]["text"], "丙");
    assert_eq!(t["rows"][1]["cells"][1]["id"], "slide:1/table:3/r:2/c:2");

    let patched = tmp("cell");
    apply(
        &inserted,
        &patched,
        json!([{
            "op": "setTableCellText",
            "id": "slide:1/table:3/r:1/c:1",
            "text": "改过"
        }]),
    )
    .unwrap();
    let tree = inspect_tables(&patched);
    assert_eq!(
        tree["slides"][0]["tables"][0]["rows"][0]["cells"][0]["text"],
        "改过"
    );
    assert_eq!(
        tree["slides"][0]["tables"][0]["rows"][0]["cells"][1]["text"],
        "乙"
    );

    let rowed = tmp("row");
    apply(
        &patched,
        &rowed,
        json!([{ "op": "insertRow", "id": "slide:1/table:3" }]),
    )
    .unwrap();
    let tree = inspect_tables(&rowed);
    assert_eq!(tree["slides"][0]["tables"][0]["rowCount"], 3);
    assert_eq!(
        tree["slides"][0]["tables"][0]["rows"][2]["cells"][0]["text"],
        ""
    );

    let after_row = tmp("after-row");
    apply(
        &rowed,
        &after_row,
        json!([{ "op": "insertRow", "id": "slide:1/table:3/row:1" }]),
    )
    .unwrap();
    let tree = inspect_tables(&after_row);
    assert_eq!(tree["slides"][0]["tables"][0]["rowCount"], 4);

    let cold = tmp("col");
    apply(
        &after_row,
        &cold,
        json!([{ "op": "insertCol", "id": "slide:1/table:3", "after": 0 }]),
    )
    .unwrap();
    let tree = inspect_tables(&cold);
    assert_eq!(tree["slides"][0]["tables"][0]["colCount"], 3);
    assert_eq!(
        tree["slides"][0]["tables"][0]["rows"][0]["cells"]
            .as_array()
            .unwrap()
            .len(),
        3
    );

    let pos = tmp("pos");
    apply(
        &cold,
        &pos,
        json!([
            { "op": "setTablePos", "id": "slide:1/table:3", "x": 100, "y": 200 },
            { "op": "setTableSize", "id": "slide:1/table:3", "cx": 4000000, "cy": 2000000 }
        ]),
    )
    .unwrap();
    let tree = inspect_tables(&pos);
    assert_eq!(tree["slides"][0]["tables"][0]["x"], 100);
    assert_eq!(tree["slides"][0]["tables"][0]["y"], 200);
    assert_eq!(tree["slides"][0]["tables"][0]["cx"], 4000000);
    assert_eq!(tree["slides"][0]["tables"][0]["cy"], 2000000);

    let trimmed = tmp("del-row-col");
    apply(
        &pos,
        &trimmed,
        json!([
            { "op": "deleteCol", "id": "slide:1/table:3", "col": 1 },
            { "op": "deleteRow", "id": "slide:1/table:3/row:4" }
        ]),
    )
    .unwrap();
    let tree = inspect_tables(&trimmed);
    assert_eq!(tree["slides"][0]["tables"][0]["colCount"], 2);
    assert_eq!(tree["slides"][0]["tables"][0]["rowCount"], 3);

    let gone = tmp("del");
    apply(
        &trimmed,
        &gone,
        json!([{ "op": "deleteTable", "id": "slide:1/table:3" }]),
    )
    .unwrap();
    let tree = inspect_tables(&gone);
    assert_eq!(tree["tableCount"], 0);
    assert_eq!(tree["slides"][0]["tables"], json!([]));
    let shapes = inspect_scope(&gone, json!({ "slide": 1 }));
    assert_eq!(shapes["slides"][0]["shapes"][0]["text"], "标题");
}

#[test]
fn create_with_tables() {
    let path = tmp("create");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "presentation": {
                "slides": [{
                    "shapes": [{ "text": "标题" }],
                    "tables": [{
                        "rows": 2,
                        "cols": 2,
                        "x": 914400,
                        "y": 1828800,
                        "cx": 5486400,
                        "cy": 3200400,
                        "cells": [["甲", "乙"], ["丙", "丁"]]
                    }]
                }]
            }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let tree = inspect_tables(&path);
    assert_eq!(tree["tableCount"], 1);
    let t = &tree["slides"][0]["tables"][0];
    assert_eq!(t["id"], "slide:1/table:3");
    assert_eq!(t["stable"], true);
    assert_eq!(t["rows"][0]["cells"][0]["text"], "甲");
    assert_eq!(t["rows"][1]["cells"][1]["text"], "丁");
}

#[test]
fn charts_are_not_tables() {
    let src = create_deck("not-chart");
    let out = tmp("chart");
    apply(
        &src,
        &out,
        json!([{
            "op": "insertChart",
            "id": "slide:1",
            "type": "col",
            "series": [{ "values": [1, 2, 3] }]
        }]),
    )
    .unwrap();
    let tree = inspect_tables(&out);
    assert_eq!(tree["tableCount"], 0);
    assert_eq!(tree["slides"][0]["tables"], json!([]));
    assert_eq!(tree["chartCount"], 1);
}

#[test]
fn wrong_ids_and_missing_fields() {
    let src = create_deck("bad");
    let dest = tmp("bad-id");
    let err = apply(
        &src,
        &dest,
        json!([{ "op": "insertTable", "id": "slide:1/shape:2", "rows": 2, "cols": 2 }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    assert!(!dest.exists());

    let dest = tmp("bad-rows");
    let err = apply(
        &src,
        &dest,
        json!([{ "op": "insertTable", "id": "slide:1", "cols": 2 }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!dest.exists());

    let with = tmp("with");
    insert_two_by_two(&src, &with);
    let dest = tmp("last-row");
    apply(
        &with,
        &dest,
        json!([{ "op": "deleteRow", "id": "slide:1/table:3/row:2" }]),
    )
    .unwrap();
    let dest2 = tmp("last-row-fail");
    let err = apply(
        &dest,
        &dest2,
        json!([{ "op": "deleteRow", "id": "slide:1/table:3/row:1" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!dest2.exists());

    let err = apply(
        &with,
        &tmp("merge"),
        json!([{
            "op": "mergeCells",
            "id": "slide:1/table:3",
            "startRow": 1,
            "startCol": 1,
            "endRow": 1,
            "endCol": 2
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let err = apply(
        &with,
        &tmp("cell-on-table"),
        json!([{ "op": "setTableCellText", "id": "slide:1/table:3", "text": "x" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
}

#[test]
fn docx_and_xlsx_reject_pptx_table_ops() {
    let docx = tmp("d").with_extension("docx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": docx.to_string_lossy(),
            "document": { "paragraphs": [{ "text": "a" }] }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let err = apply(
        &docx,
        &tmp("d-out").with_extension("docx"),
        json!([{ "op": "setTableCellText", "id": "slide:1/table:3/r:1/c:1", "text": "x" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let xlsx = tmp("x").with_extension("xlsx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": xlsx.to_string_lossy(),
            "workbook": { "sheets": [{ "name": "Sheet1" }] }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let err = apply(
        &xlsx,
        &tmp("x-out").with_extension("xlsx"),
        json!([{ "op": "setTablePos", "id": "slide:1/table:3", "x": 1 }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}
