//! 工作表表格：inspect 名单、insert / set / delete。

use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::json;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-xlsx-table");
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
                        { "ref": "A1", "value": "产品" },
                        { "ref": "B1", "value": "数量" },
                        { "ref": "C1", "value": "金额" },
                        { "ref": "A2", "value": "甲" },
                        { "ref": "B2", "value": 1 },
                        { "ref": "C2", "value": 10 },
                        { "ref": "A3", "value": "乙" },
                        { "ref": "B3", "value": 2 },
                        { "ref": "C3", "value": 20 }
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

fn inspect_sheet(path: &Path) -> serde_json::Value {
    let RunOutput::Inspect(tree) = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": path.to_string_lossy(),
            "scope": { "sheet": "Sheet1", "range": "A1:C4" }
        })
        .to_string(),
        None,
    )
    .unwrap() else {
        panic!("inspect 应返回树");
    };
    tree["sheets"][0].clone()
}

fn inspect_noscope(path: &Path) -> serde_json::Value {
    let RunOutput::Inspect(tree) = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": path.to_string_lossy(),
        })
        .to_string(),
        None,
    )
    .unwrap() else {
        panic!("inspect 应返回树");
    };
    tree["sheets"][0].clone()
}

fn part_text(path: &Path, name: &str) -> String {
    let f = std::fs::File::open(path).unwrap();
    let mut zip = zip::ZipArchive::new(f).unwrap();
    let mut zf = zip.by_name(name).unwrap();
    let mut s = String::new();
    zf.read_to_string(&mut s).unwrap();
    s
}

fn part_exists(path: &Path, name: &str) -> bool {
    let f = std::fs::File::open(path).unwrap();
    let mut zip = zip::ZipArchive::new(f).unwrap();
    let ok = zip.by_name(name).is_ok();
    ok
}

fn object_keys(v: &serde_json::Value) -> Vec<&str> {
    v.as_object().unwrap().keys().map(|k| k.as_str()).collect()
}

#[test]
fn inspect_empty_on_create() {
    let path = create_book("empty");
    let no = inspect_noscope(&path);
    assert_eq!(no["tableCount"], json!(0));
    assert!(no.get("tables").is_none());
    let sh = inspect_sheet(&path);
    assert_eq!(sh["tableCount"], json!(0));
    assert_eq!(sh["tables"], json!([]));
}

#[test]
fn insert_named_table_and_inspect() {
    let src = create_book("ins");
    let out = tmp("ins-out");
    apply(
        &src,
        &out,
        json!([{
            "op": "insertSheetTable",
            "id": "sheet:Sheet1",
            "ref": "A1:C3",
            "name": "Sales",
            "columns": ["产品", "数量", "金额"]
        }]),
    )
    .unwrap();
    let sh = inspect_sheet(&out);
    assert_eq!(sh["tableCount"], json!(1));
    let t = &sh["tables"][0];
    assert_eq!(t["id"], json!("sheet:Sheet1/table:1"));
    assert_eq!(t["name"], json!("Sales"));
    assert_eq!(t["ref"], json!("A1:C3"));
    assert_eq!(t["columns"], json!(["产品", "数量", "金额"]));
    assert_eq!(object_keys(t), ["id", "name", "ref", "columns"]);
    let sheet = part_text(&out, "xl/worksheets/sheet1.xml");
    assert!(sheet.contains("tableParts"), "{sheet}");
    assert!(sheet.contains("tablePart"), "{sheet}");
    let table = part_text(&out, "xl/tables/table1.xml");
    assert!(table.contains(r#"name="Sales""#), "{table}");
    assert!(table.contains(r#"displayName="Sales""#), "{table}");
    assert!(table.contains(r#"ref="A1:C3""#), "{table}");
    assert!(table.contains("autoFilter"), "{table}");
    assert!(table.contains("TableStyleMedium2"), "{table}");
    assert!(table.contains(r#"totalsRowShown="0""#), "{table}");
    let cts = part_text(&out, "[Content_Types].xml");
    assert!(cts.contains("spreadsheetml.table+xml"), "{cts}");
    let a1 = sh["cells"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["ref"] == "A1")
        .unwrap();
    assert_eq!(a1["value"], json!("产品"));
}

#[test]
fn set_ref_then_delete_leaves_cells() {
    let src = create_book("setdel");
    let mid = tmp("setdel-mid");
    apply(
        &src,
        &mid,
        json!([{
            "op": "insertSheetTable",
            "id": "sheet:Sheet1",
            "ref": "A1:C3",
            "name": "Sales",
            "columns": ["产品", "数量", "金额"]
        }]),
    )
    .unwrap();
    let expanded = tmp("setdel-exp");
    apply(
        &mid,
        &expanded,
        json!([{
            "op": "setSheetTable",
            "id": "sheet:Sheet1/table:1",
            "ref": "A1:C4"
        }]),
    )
    .unwrap();
    let sh = inspect_sheet(&expanded);
    assert_eq!(sh["tables"][0]["ref"], json!("A1:C4"));
    assert_eq!(sh["tables"][0]["columns"], json!(["产品", "数量", "金额"]));
    let table = part_text(&expanded, "xl/tables/table1.xml");
    assert!(table.contains(r#"ref="A1:C4""#), "{table}");
    assert!(
        table.contains(r#"autoFilter ref="A1:C4""#)
            || table.contains(r#"<autoFilter ref="A1:C4"/>"#),
        "{table}"
    );
    let gone = tmp("setdel-gone");
    apply(
        &expanded,
        &gone,
        json!([{ "op": "deleteSheetTable", "id": "sheet:Sheet1/table:1" }]),
    )
    .unwrap();
    let sh2 = inspect_sheet(&gone);
    assert_eq!(sh2["tableCount"], json!(0));
    assert_eq!(sh2["tables"], json!([]));
    let xml = part_text(&gone, "xl/worksheets/sheet1.xml");
    assert!(!xml.contains("tableParts"), "{xml}");
    assert!(!part_exists(&gone, "xl/tables/table1.xml"));
    let a1 = sh2["cells"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["ref"] == "A1")
        .unwrap();
    assert_eq!(a1["value"], json!("产品"));
}

#[test]
fn insert_default_name_from_headers() {
    let src = create_book("hdr");
    let out = tmp("hdr-out");
    apply(
        &src,
        &out,
        json!([{
            "op": "insertSheetTable",
            "id": "sheet:Sheet1",
            "ref": "A1:C3"
        }]),
    )
    .unwrap();
    let sh = inspect_sheet(&out);
    assert_eq!(sh["tables"][0]["name"], json!("Table1"));
    assert_eq!(sh["tables"][0]["columns"], json!(["产品", "数量", "金额"]));
}

#[test]
fn overlap_and_one_row_are_invalid() {
    let src = create_book("bad");
    let first = tmp("bad-first");
    apply(
        &src,
        &first,
        json!([{
            "op": "insertSheetTable",
            "id": "sheet:Sheet1",
            "ref": "A1:C3",
            "name": "Sales"
        }]),
    )
    .unwrap();
    let out = tmp("bad-overlap");
    let err = apply(
        &first,
        &out,
        json!([{
            "op": "insertSheetTable",
            "id": "sheet:Sheet1",
            "ref": "B2:D4",
            "name": "Other"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());
    let row = tmp("bad-row");
    let err2 = apply(
        &src,
        &row,
        json!([{
            "op": "insertSheetTable",
            "id": "sheet:Sheet1",
            "ref": "A1:C1",
            "name": "Row"
        }]),
    )
    .unwrap_err();
    assert_eq!(err2.code(), ErrorCode::InvalidField);
    assert!(!row.exists());
}

#[test]
fn missing_fields_are_invalid() {
    let src = create_book("miss");
    let out = tmp("miss-out");
    let err = apply(
        &src,
        &out,
        json!([{ "op": "insertSheetTable", "id": "sheet:Sheet1" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());
}

#[test]
fn set_without_fields_is_invalid() {
    let src = create_book("setmiss");
    let mid = tmp("setmiss-mid");
    apply(
        &src,
        &mid,
        json!([{
            "op": "insertSheetTable",
            "id": "sheet:Sheet1",
            "ref": "A1:C3"
        }]),
    )
    .unwrap();
    let out = tmp("setmiss-out");
    let err = apply(
        &mid,
        &out,
        json!([{ "op": "setSheetTable", "id": "sheet:Sheet1/table:1" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());
}

#[test]
fn wrong_id_is_invalid() {
    let src = create_book("id");
    let out = tmp("id-out");
    let err = apply(
        &src,
        &out,
        json!([{ "op": "deleteSheetTable", "id": "sheet:Sheet1" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    assert!(!out.exists());
}

#[test]
fn missing_table_is_not_found() {
    let src = create_book("nf");
    let out = tmp("nf-out");
    let err = apply(
        &src,
        &out,
        json!([{
            "op": "deleteSheetTable",
            "id": "sheet:Sheet1/table:1"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::IdNotFound);
    assert!(!out.exists());
}
