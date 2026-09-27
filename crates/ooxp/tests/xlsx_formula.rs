//! 共享公式与数组公式：inspect 展开、主格保住或拆组、数组读写。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::json;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-xlsx-formula");
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
                        { "ref": "A1", "value": 10 },
                        { "ref": "A2", "value": 11 }
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

fn inspect_cells(path: &Path) -> serde_json::Value {
    let RunOutput::Inspect(tree) = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": path.to_string_lossy(),
            "scope": { "sheet": "Sheet1", "range": "A1:C3" }
        })
        .to_string(),
        None,
    )
    .unwrap() else {
        panic!("inspect 应返回树");
    };
    tree["sheets"][0]["cells"].clone()
}

fn cell_of<'a>(cells: &'a serde_json::Value, id: &str) -> &'a serde_json::Value {
    cells
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == id)
        .unwrap_or_else(|| panic!("树里没有 {id}"))
}

fn with_shared(tag: &str) -> PathBuf {
    let src = create_book(tag);
    let out = tmp(&format!("{tag}-shared"));
    apply(
        &src,
        &out,
        json!([{
            "op": "setCell",
            "id": "cell:Sheet1!B1",
            "formula": "=A1*2",
            "sharedRef": "B1:B2"
        }]),
    )
    .unwrap();
    out
}

#[test]
fn inspect_expands_shared_dependent() {
    let path = with_shared("insp");
    let cells = inspect_cells(&path);
    let b1 = cell_of(&cells, "cell:Sheet1!B1");
    let b2 = cell_of(&cells, "cell:Sheet1!B2");
    assert_eq!(b1["formula"], json!("=A1*2"));
    assert_eq!(b1["shared"], json!(true));
    assert_eq!(b1["sharedRef"], json!("B1:B2"));
    assert_eq!(b2["formula"], json!("=A2*2"));
    assert_eq!(b2["shared"], json!(true));
    assert!(b2.get("sharedRef").is_none());
}

#[test]
fn keep_shared_master_formula() {
    let src = with_shared("keep");
    let out = tmp("keep-out");
    apply(
        &src,
        &out,
        json!([{ "op": "setCell", "id": "cell:Sheet1!B1", "formula": "=A1*3" }]),
    )
    .unwrap();
    let cells = inspect_cells(&out);
    let b1 = cell_of(&cells, "cell:Sheet1!B1");
    let b2 = cell_of(&cells, "cell:Sheet1!B2");
    assert_eq!(b1["formula"], json!("=A1*3"));
    assert_eq!(b1["shared"], json!(true));
    assert_eq!(b1["sharedRef"], json!("B1:B2"));
    assert_eq!(b2["formula"], json!("=A2*3"));
    assert_eq!(b2["shared"], json!(true));
}

#[test]
fn explode_shared_group() {
    let src = with_shared("exp");
    let out = tmp("exp-out");
    apply(
        &src,
        &out,
        json!([{
            "op": "setCell",
            "id": "cell:Sheet1!B1",
            "formula": "=A1*4",
            "shared": false
        }]),
    )
    .unwrap();
    let cells = inspect_cells(&out);
    let b1 = cell_of(&cells, "cell:Sheet1!B1");
    let b2 = cell_of(&cells, "cell:Sheet1!B2");
    assert_eq!(b1["formula"], json!("=A1*4"));
    assert!(b1.get("shared").is_none());
    assert_eq!(b2["formula"], json!("=A2*2"));
    assert!(b2.get("shared").is_none());
}

#[test]
fn dependent_still_becomes_independent() {
    let src = with_shared("dep");
    let out = tmp("dep-out");
    apply(
        &src,
        &out,
        json!([{ "op": "setCell", "id": "cell:Sheet1!B2", "formula": "=A2*9" }]),
    )
    .unwrap();
    let cells = inspect_cells(&out);
    let b1 = cell_of(&cells, "cell:Sheet1!B1");
    let b2 = cell_of(&cells, "cell:Sheet1!B2");
    assert_eq!(b1["formula"], json!("=A1*2"));
    assert_eq!(b1["shared"], json!(true));
    assert_eq!(b2["formula"], json!("=A2*9"));
    assert!(b2.get("shared").is_none());
}

#[test]
fn write_and_inspect_array() {
    let src = create_book("arr");
    let out = tmp("arr-out");
    apply(
        &src,
        &out,
        json!([{
            "op": "setCell",
            "id": "cell:Sheet1!C1",
            "formula": "=A1:A2",
            "arrayRef": "C1:C2"
        }]),
    )
    .unwrap();
    let cells = inspect_cells(&out);
    let c1 = cell_of(&cells, "cell:Sheet1!C1");
    let c2 = cell_of(&cells, "cell:Sheet1!C2");
    assert_eq!(c1["formula"], json!("=A1:A2"));
    assert_eq!(c1["array"], json!(true));
    assert_eq!(c1["arrayRef"], json!("C1:C2"));
    assert_eq!(c2["array"], json!(true));
    assert!(c2["formula"].is_null());
}

#[test]
fn array_part_is_not_implemented() {
    let src = create_book("arrp");
    let mid = tmp("arrp-mid");
    apply(
        &src,
        &mid,
        json!([{
            "op": "setCell",
            "id": "cell:Sheet1!C1",
            "formula": "=A1:A2",
            "arrayRef": "C1:C2"
        }]),
    )
    .unwrap();
    let out = tmp("arrp-out");
    let err = apply(
        &mid,
        &out,
        json!([{ "op": "setCell", "id": "cell:Sheet1!C2", "value": 1 }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::NotImplemented);
    assert!(!out.exists());
}

#[test]
fn shared_true_without_ref_on_plain_cell() {
    let src = create_book("needref");
    let out = tmp("needref-out");
    let err = apply(
        &src,
        &out,
        json!([{
            "op": "setCell",
            "id": "cell:Sheet1!B1",
            "formula": "=A1",
            "shared": true
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());
}

#[test]
fn array_ref_not_top_left() {
    let src = create_book("nottl");
    let out = tmp("nottl-out");
    let err = apply(
        &src,
        &out,
        json!([{
            "op": "setCell",
            "id": "cell:Sheet1!C2",
            "formula": "=A1:A2",
            "arrayRef": "C1:C2"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());
}

#[test]
fn expand_shared_ref() {
    let src = with_shared("grow");
    let out = tmp("grow-out");
    apply(
        &src,
        &out,
        json!([{
            "op": "setCell",
            "id": "cell:Sheet1!B1",
            "formula": "=A1*2",
            "sharedRef": "B1:B3"
        }]),
    )
    .unwrap();
    let cells = inspect_cells(&out);
    let b1 = cell_of(&cells, "cell:Sheet1!B1");
    let b3 = cell_of(&cells, "cell:Sheet1!B3");
    assert_eq!(b1["sharedRef"], json!("B1:B3"));
    assert_eq!(b3["formula"], json!("=A3*2"));
    assert_eq!(b3["shared"], json!(true));
}
