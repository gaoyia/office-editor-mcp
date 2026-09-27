//! 视图与筛选：inspect 摘要、setSheetView / setAutoFilter。

use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::json;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-xlsx-view");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn tmp(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}.xlsx"))
}

fn create_book(tag: &str) -> PathBuf {
    let path = tmp(tag);
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "workbook": {
                "sheets": [{
                    "name": "Sheet1",
                    "cells": [
                        { "ref": "A1", "value": 1 },
                        { "ref": "A10", "value": 10 }
                    ]
                }]
            }
        })
        .to_string(),
        None,
    )
    .unwrap();
    path
}

fn apply(src: &Path, out: &Path, ops: serde_json::Value) -> Result<RunOutput, ooxp::Error> {
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

fn inspect_tree(path: &Path) -> serde_json::Value {
    let RunOutput::Inspect(tree) = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": path.to_string_lossy()
        })
        .to_string(),
        None,
    )
    .unwrap() else {
        panic!("inspect 应返回树");
    };
    tree
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
fn inspect_defaults() {
    let path = create_book("def");
    let v = &inspect_tree(&path)["sheets"][0]["view"];
    assert_eq!(v["zoom"], json!(100));
    assert_eq!(v["frozenRows"], json!(0));
    assert_eq!(v["frozenCols"], json!(0));
    assert_eq!(v["activeCell"], json!("A1"));
    assert_eq!(
        inspect_tree(&path)["sheets"][0]["autoFilter"],
        serde_json::Value::Null
    );
}

#[test]
fn freeze_zoom_and_filter() {
    let src = create_book("fz");
    let out = tmp("fz-out");
    apply(
        &src,
        &out,
        json!([
            {
                "op": "setSheetView",
                "id": "sheet:Sheet1",
                "zoom": 80,
                "frozenRows": 1,
                "frozenCols": 0,
                "activeCell": "A2"
            },
            {
                "op": "setAutoFilter",
                "id": "sheet:Sheet1",
                "ref": "A1:A10"
            }
        ]),
    )
    .unwrap();
    let tree = inspect_tree(&out);
    let v = &tree["sheets"][0]["view"];
    assert_eq!(v["zoom"], json!(80));
    assert_eq!(v["frozenRows"], json!(1));
    assert_eq!(v["frozenCols"], json!(0));
    assert_eq!(v["activeCell"], json!("A2"));
    assert_eq!(tree["sheets"][0]["autoFilter"], json!("A1:A10"));
    let xml = part_text(&out, "xl/worksheets/sheet1.xml");
    assert!(xml.contains(r#"zoomScale="80""#), "{xml}");
    assert!(xml.contains(r#"ySplit="1""#), "{xml}");
    assert!(xml.contains(r#"state="frozen""#), "{xml}");
    assert!(xml.contains(r#"autoFilter ref="A1:A10""#), "{xml}");
}

#[test]
fn unfreeze_and_clear_filter() {
    let src = create_book("clr");
    let mid = tmp("clr-mid");
    apply(
        &src,
        &mid,
        json!([
            { "op": "setSheetView", "id": "sheet:Sheet1", "frozenRows": 1, "frozenCols": 1 },
            { "op": "setAutoFilter", "id": "sheet:Sheet1", "ref": "A1:A10" }
        ]),
    )
    .unwrap();
    let out = tmp("clr-out");
    apply(
        &mid,
        &out,
        json!([
            { "op": "setSheetView", "id": "sheet:Sheet1", "frozenRows": 0, "frozenCols": 0 },
            { "op": "setAutoFilter", "id": "sheet:Sheet1", "ref": null }
        ]),
    )
    .unwrap();
    let tree = inspect_tree(&out);
    assert_eq!(tree["sheets"][0]["view"]["frozenRows"], json!(0));
    assert_eq!(tree["sheets"][0]["view"]["frozenCols"], json!(0));
    assert_eq!(tree["sheets"][0]["autoFilter"], serde_json::Value::Null);
    let xml = part_text(&out, "xl/worksheets/sheet1.xml");
    assert!(!xml.contains("<pane"), "{xml}");
    assert!(!xml.contains("<autoFilter"), "{xml}");
}

#[test]
fn missing_field_and_cell_id() {
    let src = create_book("bad");
    let fail_out = tmp("bad-wrote");
    let err = apply(
        &src,
        &fail_out,
        json!([{ "op": "setSheetView", "id": "sheet:Sheet1" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!fail_out.exists());

    let err = apply(
        &src,
        &tmp("cell-id"),
        json!([{ "op": "setSheetView", "id": "cell:Sheet1!A1", "zoom": 80 }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);

    let miss = tmp("af-miss");
    let err = apply(
        &src,
        &miss,
        json!([{ "op": "setAutoFilter", "id": "sheet:Sheet1" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!miss.exists());
}

#[test]
fn filter_rejects_table_overlap() {
    let src = create_book("tbl");
    let with_table = tmp("tbl-mid");
    apply(
        &src,
        &with_table,
        json!([{
            "op": "insertSheetTable",
            "id": "sheet:Sheet1",
            "ref": "A1:A10",
            "name": "Sales"
        }]),
    )
    .unwrap();
    let err = apply(
        &with_table,
        &tmp("tbl-fail"),
        json!([{ "op": "setAutoFilter", "id": "sheet:Sheet1", "ref": "A1:A10" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}
