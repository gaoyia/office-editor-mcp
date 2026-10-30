//! 表单元格 tcPr 剩余：vert / anchorCtr / horzOverflow / cell3D / headers。

use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::ZipArchive;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-pptx-table-cell-pr");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn tmp(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}.pptx"))
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

fn inspect_tables(path: &Path) -> Value {
    let RunOutput::Inspect(tree) = run(&json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": path.to_string_lossy(),
        "scope": { "tables": true }
    })
    .to_string())
    .unwrap() else {
        panic!("inspect 应返回树");
    };
    tree
}

fn object_keys(v: &Value) -> Vec<&str> {
    v.as_object().unwrap().keys().map(|s| s.as_str()).collect()
}

fn slide_xml(path: &Path) -> String {
    let f = std::fs::File::open(path).unwrap();
    let mut zip = ZipArchive::new(f).unwrap();
    let mut s = String::new();
    zip.by_name("ppt/slides/slide1.xml")
        .unwrap()
        .read_to_string(&mut s)
        .unwrap();
    s
}

fn create_table_deck(tag: &str, first_cell: Value) -> PathBuf {
    let path = tmp(tag);
    run(&json!({
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
                    "cells": [
                        [first_cell, "乙"],
                        ["丙", "丁"]
                    ]
                }]
            }]
        }
    })
    .to_string())
    .unwrap();
    path
}

fn cell0(tree: &Value) -> &Value {
    &tree["slides"][0]["tables"][0]["rows"][0]["cells"][0]
}

#[test]
fn create_vert_and_default_keys() {
    let path = create_table_deck("vert", json!({ "text": "甲", "vert": "eaVert" }));
    let tree = inspect_tables(&path);
    let c = cell0(&tree);
    assert_eq!(c["vert"], "eaVert");
    assert_eq!(c["anchorCtr"], Value::Null);
    assert_eq!(c["horzOverflow"], Value::Null);
    assert_eq!(c["cell3D"], Value::Null);
    assert_eq!(c["headers"], json!([]));
    let xml = slide_xml(&path);
    assert!(xml.contains("vert=\"eaVert\""));
    assert!(!xml.contains("<a:tcPr vert=\"eaVert\" anchor"));
}

#[test]
fn set_table_cell_pr_attrs_3d_headers_and_clear() {
    let src = create_table_deck("src", json!("甲"));
    let out = tmp("pr");
    apply(
        &src,
        &out,
        json!([
            {
                "op": "setTableCellPr",
                "id": "slide:1/table:3/r:1/c:1",
                "vert": "vert",
                "anchorCtr": true,
                "horzOverflow": "overflow"
            }
        ]),
    )
    .unwrap();
    let tree = inspect_tables(&out);
    let c = cell0(&tree);
    assert_eq!(c["vert"], "vert");
    assert_eq!(c["anchorCtr"], true);
    assert_eq!(c["horzOverflow"], "overflow");

    let three = tmp("3d");
    apply(
        &out,
        &three,
        json!([{
            "op": "setTableCellPr",
            "id": "slide:1/table:3/r:1/c:1",
            "cell3D": {
                "prstMaterial": "plastic",
                "bevel": { "prst": "circle", "w": 76200, "h": 76200 }
            }
        }]),
    )
    .unwrap();
    let tree = inspect_tables(&three);
    let c = cell0(&tree);
    assert_eq!(c["cell3D"]["prstMaterial"], "plastic");
    assert_eq!(c["cell3D"]["bevel"]["prst"], "circle");
    assert_eq!(c["cell3D"]["bevel"]["w"], 76200);
    assert_eq!(c["cell3D"]["bevel"]["h"], 76200);
    assert_eq!(object_keys(&c["cell3D"]), ["prstMaterial", "bevel"]);
    assert_eq!(object_keys(&c["cell3D"]["bevel"]), ["prst", "w", "h"]);
    let xml = slide_xml(&three);
    assert!(xml.contains("<a:cell3D"));
    assert!(xml.contains("<a:bevel"));
    assert!(!xml.contains("lightRig"));

    let headed = tmp("headers");
    apply(
        &three,
        &headed,
        json!([{
            "op": "setTableCellPr",
            "id": "slide:1/table:3/r:1/c:1",
            "headers": ["列甲"]
        }]),
    )
    .unwrap();
    let tree = inspect_tables(&headed);
    let c = cell0(&tree);
    assert_eq!(c["headers"], json!(["列甲"]));
    assert!(slide_xml(&headed).contains("<a:header>列甲</a:header>"));

    let cleared = tmp("clear");
    apply(
        &headed,
        &cleared,
        json!([{
            "op": "setTableCellPr",
            "id": "slide:1/table:3/r:1/c:1",
            "vert": null,
            "anchorCtr": null,
            "horzOverflow": null,
            "cell3D": null,
            "headers": null
        }]),
    )
    .unwrap();
    let tree = inspect_tables(&cleared);
    let c = cell0(&tree);
    assert_eq!(c["vert"], Value::Null);
    assert_eq!(c["anchorCtr"], Value::Null);
    assert_eq!(c["horzOverflow"], Value::Null);
    assert_eq!(c["cell3D"], Value::Null);
    assert_eq!(c["headers"], json!([]));
    let xml = slide_xml(&cleared);
    assert!(!xml.contains("vert="));
    assert!(!xml.contains("anchorCtr="));
    assert!(!xml.contains("horzOverflow="));
    assert!(!xml.contains("<a:cell3D"));
    assert!(!xml.contains("<a:headers"));
}

#[test]
fn cell3d_omitted_bevel_writes_xsd_defaults() {
    let src = create_table_deck("def", json!("甲"));
    let out = tmp("def-out");
    apply(
        &src,
        &out,
        json!([{
            "op": "setTableCellPr",
            "id": "slide:1/table:3/r:1/c:1",
            "cell3D": { "prstMaterial": "metal" }
        }]),
    )
    .unwrap();
    let tree = inspect_tables(&out);
    let c = cell0(&tree);
    assert_eq!(c["cell3D"]["prstMaterial"], "metal");
    assert_eq!(c["cell3D"]["bevel"]["prst"], "circle");
    assert_eq!(c["cell3D"]["bevel"]["w"], 76200);
    assert_eq!(c["cell3D"]["bevel"]["h"], 76200);
}

#[test]
fn set_table_cell_pr_rejects() {
    let src = create_table_deck("rej", json!("甲"));
    let id = "slide:1/table:3/r:1/c:1";

    let err = apply(
        &src,
        &tmp("empty"),
        json!([{ "op": "setTableCellPr", "id": id }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let err = apply(
        &src,
        &tmp("vert"),
        json!([{ "op": "setTableCellPr", "id": id, "vert": "sideways" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let err = apply(
        &src,
        &tmp("table-id"),
        json!([{ "op": "setTableCellPr", "id": "slide:1/table:3", "vert": "horz" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);

    let xlsx = tmp("xlsx").with_extension("xlsx");
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": xlsx.to_string_lossy(),
        "workbook": { "sheets": [{ "name": "Sheet1" }] }
    })
    .to_string())
    .unwrap();
    let err = apply(
        &xlsx,
        &tmp("xlsx-out").with_extension("xlsx"),
        json!([{ "op": "setTableCellPr", "id": id, "vert": "horz" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}
