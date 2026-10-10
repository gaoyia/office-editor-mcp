//! 工作表级排序：inspect 摘要、setSortState。不重排格子。

use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::json;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-xlsx-sort");
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
                        { "ref": "A1", "value": 3 },
                        { "ref": "A2", "value": 1 },
                        { "ref": "A3", "value": 2 }
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
    inspect_tree_scope(path, json!({ "sheet": "Sheet1", "range": "A1:A3" }))
}

fn inspect_tree_scope(path: &Path, scope: serde_json::Value) -> serde_json::Value {
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

fn part_text(path: &Path, name: &str) -> String {
    let f = std::fs::File::open(path).unwrap();
    let mut zip = zip::ZipArchive::new(f).unwrap();
    let mut zf = zip.by_name(name).unwrap();
    let mut s = String::new();
    zf.read_to_string(&mut s).unwrap();
    s
}

#[test]
fn create_sort_is_null() {
    let path = create_book("def");
    let sheet = &inspect_tree(&path)["sheets"][0];
    assert_eq!(sheet["sortState"], serde_json::Value::Null);
}

#[test]
fn set_sort_state_writes_xml() {
    let src = create_book("set");
    let out = tmp("set-out");
    apply(
        &src,
        &out,
        json!([{
            "op": "setSortState",
            "id": "sheet:Sheet1",
            "ref": "A1:A3",
            "caseSensitive": true,
            "columns": [{ "ref": "A1:A3", "descending": true }]
        }]),
    )
    .unwrap();
    let st = &inspect_tree(&out)["sheets"][0]["sortState"];
    assert_eq!(st["ref"], json!("A1:A3"));
    assert_eq!(st["caseSensitive"], json!(true));
    assert_eq!(st["columnSort"], json!(false));
    assert_eq!(st["columns"][0]["ref"], json!("A1:A3"));
    assert_eq!(st["columns"][0]["descending"], json!(true));
    let xml = part_text(&out, "xl/worksheets/sheet1.xml");
    assert!(xml.contains(r#"<sortState"#), "{xml}");
    assert!(xml.contains(r#"ref="A1:A3""#), "{xml}");
    assert!(xml.contains(r#"caseSensitive="1""#), "{xml}");
    assert!(xml.contains(r#"descending="1""#), "{xml}");
    assert!(!xml.contains("columnSort="), "{xml}");
    // 不重排格子：A1 仍是 3。
    let cells = &inspect_tree(&out)["sheets"][0]["cells"];
    let a1 = cells
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["ref"] == "A1")
        .expect("A1");
    assert_eq!(a1["value"], json!(3), "{cells:?}");
}

#[test]
fn clear_sort_state() {
    let src = create_book("clr");
    let mid = tmp("clr-mid");
    apply(
        &src,
        &mid,
        json!([{
            "op": "setSortState",
            "id": "sheet:Sheet1",
            "ref": "A1:A3",
            "columns": [{ "ref": "A1:A3" }]
        }]),
    )
    .unwrap();
    let out = tmp("clr-out");
    apply(
        &mid,
        &out,
        json!([{ "op": "setSortState", "id": "sheet:Sheet1", "ref": null }]),
    )
    .unwrap();
    assert_eq!(
        inspect_tree(&out)["sheets"][0]["sortState"],
        serde_json::Value::Null
    );
    let xml = part_text(&out, "xl/worksheets/sheet1.xml");
    assert!(!xml.contains("<sortState"), "{xml}");
}

#[test]
fn replace_does_not_merge_conditions() {
    let src = create_book("rep");
    let mid = tmp("rep-mid");
    apply(
        &src,
        &mid,
        json!([{
            "op": "setSortState",
            "id": "sheet:Sheet1",
            "ref": "A1:B3",
            "columns": [
                { "ref": "A1:A3", "descending": true },
                { "ref": "B1:B3" }
            ]
        }]),
    )
    .unwrap();
    let out = tmp("rep-out");
    apply(
        &mid,
        &out,
        json!([{
            "op": "setSortState",
            "id": "sheet:Sheet1",
            "ref": "A1:A3",
            "columns": [{ "ref": "A1:A3" }]
        }]),
    )
    .unwrap();
    let cols = &inspect_tree(&out)["sheets"][0]["sortState"]["columns"];
    assert_eq!(cols.as_array().map(|a| a.len()), Some(1));
    assert_eq!(cols[0]["descending"], json!(false));
}

#[test]
fn rejects_missing_ref_and_out_of_range() {
    let src = create_book("bad");
    let miss = tmp("miss");
    let err = apply(
        &src,
        &miss,
        json!([{ "op": "setSortState", "id": "sheet:Sheet1" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!miss.exists());

    let oob = tmp("oob");
    let err = apply(
        &src,
        &oob,
        json!([{
            "op": "setSortState",
            "id": "sheet:Sheet1",
            "ref": "A1:A3",
            "columns": [{ "ref": "B1:B3" }]
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!oob.exists());

    let empty = tmp("empty");
    let err = apply(
        &src,
        &empty,
        json!([{
            "op": "setSortState",
            "id": "sheet:Sheet1",
            "ref": "A1:A3",
            "columns": []
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!empty.exists());

    let extra = tmp("null-cols");
    let err = apply(
        &src,
        &extra,
        json!([{
            "op": "setSortState",
            "id": "sheet:Sheet1",
            "ref": null,
            "columns": [{ "ref": "A1:A3" }]
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!extra.exists());

    let slide = tmp("slide");
    let err = apply(
        &src,
        &slide,
        json!([{
            "op": "setSortState",
            "id": "slide:1",
            "ref": "A1:A3",
            "columns": [{ "ref": "A1:A3" }]
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
}

#[test]
fn create_writes_sort_state() {
    let path = tmp("create-sort");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "workbook": {
                "sheets": [{
                    "name": "Sheet1",
                    "cells": [
                        { "ref": "A1", "value": 3 },
                        { "ref": "A2", "value": 1 },
                        { "ref": "A3", "value": 2 }
                    ],
                    "sortState": {
                        "ref": "A1:A3",
                        "columns": [{ "ref": "A1:A3", "descending": true }]
                    }
                }]
            }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let st = &inspect_tree(&path)["sheets"][0]["sortState"];
    assert_eq!(st["ref"], json!("A1:A3"));
    assert_eq!(st["columns"][0]["descending"], json!(true));
}

#[test]
fn custom_list_and_color_sort_fields() {
    let src = create_book("fields");
    let list_out = tmp("list-out");
    apply(
        &src,
        &list_out,
        json!([{
            "op": "setSortState",
            "id": "sheet:Sheet1",
            "ref": "A1:A3",
            "columns": [{ "ref": "A1:A3", "customList": "Low,Medium,High" }]
        }]),
    )
    .unwrap();
    let st = &inspect_tree(&list_out)["sheets"][0]["sortState"];
    assert_eq!(st["columns"][0]["customList"], json!("Low,Medium,High"));
    assert!(st["columns"][0].get("sortBy").is_none());
    let cells = &inspect_tree(&list_out)["sheets"][0]["cells"];
    let a1 = cells
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["ref"] == "A1")
        .expect("A1");
    assert_eq!(a1["value"], json!(3));

    let color_out = tmp("color-out");
    apply(
        &src,
        &color_out,
        json!([{
            "op": "setSortState",
            "id": "sheet:Sheet1",
            "ref": "A1:A3",
            "sortMethod": "stroke",
            "columns": [{
                "ref": "A1:A3",
                "sortBy": "cellColor",
                "color": { "rgb": "FFFF0000" }
            }]
        }]),
    )
    .unwrap();
    let st = &inspect_tree(&color_out)["sheets"][0]["sortState"];
    assert_eq!(st["sortMethod"], json!("stroke"));
    assert_eq!(st["columns"][0]["sortBy"], json!("cellColor"));
    assert_eq!(st["columns"][0]["color"]["rgb"], json!("FFFF0000"));
    assert!(st["columns"][0]["dxfId"].as_u64().is_some());
    let xml = part_text(&color_out, "xl/worksheets/sheet1.xml");
    assert!(xml.contains(r#"sortBy="cellColor""#), "{xml}");
    assert!(xml.contains(r#"sortMethod="stroke""#), "{xml}");
    assert!(!xml.contains("xmlns:xlrd2"), "{xml}");
}

#[test]
fn rejects_color_sort_without_color_and_icon_custom_list() {
    let src = create_book("bad-fields");
    let no_color = tmp("no-color");
    let err = apply(
        &src,
        &no_color,
        json!([{
            "op": "setSortState",
            "id": "sheet:Sheet1",
            "ref": "A1:A3",
            "columns": [{ "ref": "A1:A3", "sortBy": "cellColor" }]
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!no_color.exists());

    let icon_list = tmp("icon-list");
    let err = apply(
        &src,
        &icon_list,
        json!([{
            "op": "setSortState",
            "id": "sheet:Sheet1",
            "ref": "A1:A3",
            "columns": [{
                "ref": "A1:A3",
                "sortBy": "icon",
                "customList": "Low,Medium,High"
            }]
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!icon_list.exists());
}
