//! 表样式引用：tableStyleId 与 tblLook。

use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::ZipArchive;

const DEFAULT_STYLE: &str = "{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}";
const SPEC_STYLE: &str = "{5940675A-B579-460E-94D1-54222C63F5DA}";

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-pptx-table-style");
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

fn null_look() -> Value {
    json!({
        "firstRow": null,
        "firstCol": null,
        "lastRow": null,
        "lastCol": null,
        "bandRow": null,
        "bandCol": null
    })
}

fn create_table_deck(tag: &str, extra: Value) -> PathBuf {
    let path = tmp(tag);
    let mut table = json!({
        "rows": 2,
        "cols": 2,
        "x": 914400,
        "y": 1828800,
        "cells": [["甲", "乙"], ["丙", "丁"]]
    });
    if let Some(obj) = extra.as_object() {
        for (k, v) in obj {
            table[k] = v.clone();
        }
    }
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "presentation": {
            "slides": [{
                "shapes": [{ "text": "标题" }],
                "tables": [table]
            }]
        }
    })
    .to_string())
    .unwrap();
    path
}

fn table0(tree: &Value) -> &Value {
    &tree["slides"][0]["tables"][0]
}

#[test]
fn create_default_style_and_look() {
    let path = create_table_deck("def", json!({}));
    let tree = inspect_tables(&path);
    let t = table0(&tree);
    assert_eq!(t["tableStyleId"], DEFAULT_STYLE);
    assert_eq!(t["tblLook"]["firstRow"], true);
    assert_eq!(t["tblLook"]["bandRow"], true);
    assert_eq!(t["tblLook"]["firstCol"], Value::Null);
    assert_eq!(
        object_keys(&t["tblLook"]),
        ["firstRow", "firstCol", "lastRow", "lastCol", "bandRow", "bandCol"]
    );
    let xml = slide_xml(&path);
    assert!(xml.contains(DEFAULT_STYLE));
    assert!(xml.contains("firstRow=\"1\""));
    assert!(xml.contains("bandRow=\"1\""));
}

#[test]
fn set_table_style_id_and_patch_look() {
    let src = create_table_deck("guid", json!({}));
    let out = tmp("guid-out");
    apply(
        &src,
        &out,
        json!([{
            "op": "setTableStyleRef",
            "id": "slide:1/table:3",
            "tableStyleId": SPEC_STYLE
        }]),
    )
    .unwrap();
    let tree = inspect_tables(&out);
    let t = table0(&tree);
    assert_eq!(t["tableStyleId"], SPEC_STYLE);
    assert_eq!(t["tblLook"]["firstRow"], true);
    assert_eq!(t["tblLook"]["bandRow"], true);
    assert!(slide_xml(&out).contains(SPEC_STYLE));
    assert!(!slide_xml(&out).contains(DEFAULT_STYLE));

    let patched = tmp("look");
    apply(
        &out,
        &patched,
        json!([{
            "op": "setTableStyleRef",
            "id": "slide:1/table:3",
            "tblLook": { "firstCol": true }
        }]),
    )
    .unwrap();
    let tree = inspect_tables(&patched);
    let t = table0(&tree);
    assert_eq!(t["tableStyleId"], SPEC_STYLE);
    assert_eq!(t["tblLook"]["firstRow"], true);
    assert_eq!(t["tblLook"]["firstCol"], true);
    assert_eq!(t["tblLook"]["bandRow"], true);
}

#[test]
fn clear_style_and_look() {
    let src = create_table_deck("clr", json!({}));
    let out = tmp("clr-out");
    apply(
        &src,
        &out,
        json!([{
            "op": "setTableStyleRef",
            "id": "slide:1/table:3",
            "tableStyleId": null,
            "tblLook": null
        }]),
    )
    .unwrap();
    let tree = inspect_tables(&out);
    let t = table0(&tree);
    assert_eq!(t["tableStyleId"], Value::Null);
    assert_eq!(t["tblLook"], null_look());
    let xml = slide_xml(&out);
    assert!(!xml.contains("tableStyleId"));
    assert!(!xml.contains("firstRow="));
    assert!(!xml.contains("bandRow="));
}

#[test]
fn create_explicit_null_and_named_look() {
    let path = create_table_deck(
        "named",
        json!({
            "tableStyleId": null,
            "tblLook": { "lastRow": true, "bandCol": false }
        }),
    );
    let tree = inspect_tables(&path);
    let t = table0(&tree);
    assert_eq!(t["tableStyleId"], Value::Null);
    assert_eq!(t["tblLook"]["firstRow"], Value::Null);
    assert_eq!(t["tblLook"]["lastRow"], true);
    assert_eq!(t["tblLook"]["bandRow"], Value::Null);
    assert_eq!(t["tblLook"]["bandCol"], false);
    let xml = slide_xml(&path);
    assert!(!xml.contains("tableStyleId"));
    assert!(!xml.contains("firstRow="));
    assert!(xml.contains("lastRow=\"1\""));
    assert!(xml.contains("bandCol=\"0\""));
}

#[test]
fn empty_tbl_look_object_is_noop() {
    let src = create_table_deck("noop", json!({}));
    let out = tmp("noop-out");
    apply(
        &src,
        &out,
        json!([{
            "op": "setTableStyleRef",
            "id": "slide:1/table:3",
            "tblLook": {}
        }]),
    )
    .unwrap();
    let tree = inspect_tables(&out);
    let t = table0(&tree);
    assert_eq!(t["tableStyleId"], DEFAULT_STYLE);
    assert_eq!(t["tblLook"]["firstRow"], true);
    assert_eq!(t["tblLook"]["bandRow"], true);
    assert_eq!(t["tblLook"]["firstCol"], Value::Null);
}

#[test]
fn set_table_style_ref_rejects() {
    let src = create_table_deck("rej", json!({}));
    let id = "slide:1/table:3";

    let err = apply(
        &src,
        &tmp("empty"),
        json!([{ "op": "setTableStyleRef", "id": id }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let err = apply(
        &src,
        &tmp("guid"),
        json!([{ "op": "setTableStyleRef", "id": id, "tableStyleId": "not-a-guid" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let err = apply(
        &src,
        &tmp("cell-id"),
        json!([{
            "op": "setTableStyleRef",
            "id": "slide:1/table:3/r:1/c:1",
            "tableStyleId": SPEC_STYLE
        }]),
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
        json!([{ "op": "setTableStyleRef", "id": id, "tableStyleId": SPEC_STYLE }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}
