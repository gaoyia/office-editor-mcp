//! 列宽、行高与合并：inspect 稀疏名单、apply 拆列、合并清值。

use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::json;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-xlsx-layout");
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
            "sheets": [{
                "name": "Sheet1",
                "cells": [
                    { "ref": "A1", "value": 1 },
                    { "ref": "B1", "value": 2 },
                    { "ref": "A2", "value": 3 },
                    { "ref": "B2", "value": 4 }
                ]
            }]
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

fn inspect_sheet(path: &Path) -> serde_json::Value {
    let RunOutput::Inspect(tree) = run(&json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": path.to_string_lossy(),
        "scope": { "sheet": "Sheet1", "range": "A1:C3" }
    })
    .to_string())
    .unwrap() else {
        panic!("inspect 应返回树");
    };
    tree["sheets"][0].clone()
}

#[test]
fn inspect_empty_layout_on_create() {
    let path = create_book("empty");
    let sh = inspect_sheet(&path);
    assert_eq!(sh["cols"], json!([]));
    assert_eq!(sh["rows"], json!([]));
    assert_eq!(sh["merges"], json!([]));
    assert_eq!(sh["dataValidations"], json!([]));
    assert_eq!(sh["conditionalFormats"], json!([]));
}

#[test]
fn set_col_width_and_row_height() {
    let src = create_book("wh");
    let out = tmp("wh-out");
    apply(
        &src,
        &out,
        json!([
            { "op": "setColWidth", "id": "sheet:Sheet1", "col": "A", "width": 20 },
            { "op": "setRowHeight", "id": "sheet:Sheet1", "row": 1, "height": 30 }
        ]),
    )
    .unwrap();
    let sh = inspect_sheet(&out);
    assert_eq!(sh["cols"][0]["min"], json!("A"));
    assert_eq!(sh["cols"][0]["max"], json!("A"));
    assert_eq!(sh["cols"][0]["width"], json!(20));
    assert_eq!(sh["rows"][0]["r"], json!(1));
    assert_eq!(sh["rows"][0]["height"], json!(30));
    assert!(sh["cols"]
        .as_array()
        .unwrap()
        .iter()
        .all(|c| c["min"] != json!("B")));
}

#[test]
fn split_col_span_keeps_neighbors() {
    let src = create_book("split");
    let mid = tmp("split-ac");
    apply(
        &src,
        &mid,
        json!([{ "op": "setColWidth", "id": "sheet:Sheet1", "col": "A:C", "width": 15 }]),
    )
    .unwrap();
    let out = tmp("split-b");
    apply(
        &mid,
        &out,
        json!([{ "op": "setColWidth", "id": "sheet:Sheet1", "col": "B", "width": 20 }]),
    )
    .unwrap();
    let cols = inspect_sheet(&out)["cols"].clone();
    let a = cols
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["min"] == json!("A"))
        .unwrap();
    let b = cols
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["min"] == json!("B"))
        .unwrap();
    let c = cols
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["min"] == json!("C"))
        .unwrap();
    assert_eq!(a["width"], json!(15));
    assert_eq!(b["width"], json!(20));
    assert_eq!(c["width"], json!(15));
}

#[test]
fn merge_keeps_top_left_clears_rest() {
    let src = create_book("merge");
    let out = tmp("merge-out");
    apply(
        &src,
        &out,
        json!([{ "op": "mergeRange", "id": "sheet:Sheet1", "range": "A1:B2" }]),
    )
    .unwrap();
    let sh = inspect_sheet(&out);
    assert_eq!(sh["merges"], json!(["A1:B2"]));
    let cells = sh["cells"].as_array().unwrap();
    let a1 = cells.iter().find(|c| c["ref"] == json!("A1")).unwrap();
    assert_eq!(a1["value"], json!(1));
    let others: Vec<_> = cells.iter().filter(|c| c["ref"] != json!("A1")).collect();
    for c in others {
        assert!(c["value"].is_null(), "{}", c);
        assert!(c["formula"].is_null());
    }
}

#[test]
fn unmerge_removes_entry() {
    let src = create_book("unmerge");
    let merged = tmp("unmerge-m");
    apply(
        &src,
        &merged,
        json!([{ "op": "mergeRange", "id": "sheet:Sheet1", "range": "A1:B2" }]),
    )
    .unwrap();
    let out = tmp("unmerge-out");
    apply(
        &merged,
        &out,
        json!([{ "op": "unmergeRange", "id": "sheet:Sheet1", "range": "A1" }]),
    )
    .unwrap();
    let sh = inspect_sheet(&out);
    assert_eq!(sh["merges"], json!([]));
}

#[test]
fn layout_ops_do_not_mark_recalc() {
    let src = create_book("recalc");
    let out = tmp("recalc-out");
    apply(
        &src,
        &out,
        json!([{ "op": "setColWidth", "id": "sheet:Sheet1", "col": "A", "width": 20 }]),
    )
    .unwrap();
    let bytes = std::fs::read(&out).unwrap();
    let cursor = std::io::Cursor::new(bytes);
    let mut zip = zip::ZipArchive::new(cursor).unwrap();
    let mut wb = String::new();
    zip.by_name("xl/workbook.xml")
        .unwrap()
        .read_to_string(&mut wb)
        .unwrap();
    assert!(!wb.contains("fullCalcOnLoad"), "只改列宽不应标重算：{wb}");
}

#[test]
fn missing_width_is_invalid_field() {
    let src = create_book("empty-op");
    let out = tmp("empty-op-out");
    let err = apply(
        &src,
        &out,
        json!([{ "op": "setColWidth", "id": "sheet:Sheet1", "col": "A" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());
}

#[test]
fn single_cell_merge_rejected() {
    let src = create_book("single");
    let out = tmp("single-out");
    let err = apply(
        &src,
        &out,
        json!([{ "op": "mergeRange", "id": "sheet:Sheet1", "range": "A1" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());
}

#[test]
fn docx_rejects_set_col_width() {
    let path = tmp_dir().join(format!(
        "docx-host-{}.docx",
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "format": "docx",
        "document": { "paragraphs": [{ "text": "hi" }] }
    })
    .to_string())
    .unwrap();
    let out = tmp_dir().join(format!(
        "docx-layout-out-{}.docx",
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    let err = apply(
        &path,
        &out,
        json!([{ "op": "setColWidth", "id": "sheet:Sheet1", "col": "A", "width": 20 }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}
