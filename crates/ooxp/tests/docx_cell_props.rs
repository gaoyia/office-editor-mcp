//! docx 格属性剩余：noWrap、textDirection、fitText、hideMark、headers。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-docx-cell-props");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn create_table(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let path = tmp_dir().join(format!("{tag}-{n}.docx"));
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "format": "docx",
        "document": {
            "paragraphs": [{ "text": "前" }],
            "tables": [{ "cells": [["A1", "B1"], ["A2", "B2"]] }]
        }
    })
    .to_string())
    .unwrap();
    path
}

fn inspect_tables(path: &Path) -> Value {
    let RunOutput::Inspect(tree) = run(&json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": path.to_string_lossy(),
        "scope": { "tables": true, "from": 1, "maxParas": 20 }
    })
    .to_string())
    .unwrap() else {
        panic!("inspect");
    };
    tree
}

fn apply(path: &Path, out: &Path, ops: Value) {
    run(&json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": path.to_string_lossy(),
        "output": out.to_string_lossy(),
        "ops": ops
    })
    .to_string())
    .unwrap();
}

fn apply_err(path: &Path, ops: Value) -> ooxp::Error {
    let out = tmp_dir().join("should-not.docx");
    run(&json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": path.to_string_lossy(),
        "output": out.to_string_lossy(),
        "ops": ops
    })
    .to_string())
    .unwrap_err()
}

fn unzip_document(path: &Path) -> String {
    let file = std::fs::File::open(path).unwrap();
    let mut zip = zip::ZipArchive::new(file).unwrap();
    let mut part = zip.by_name("word/document.xml").unwrap();
    let mut s = String::new();
    std::io::Read::read_to_string(&mut part, &mut s).unwrap();
    s
}

fn cell0(tree: &Value) -> &Value {
    &tree["tables"][0]["rows"][0]["cells"][0]
}

#[test]
fn cell_remain_inspect_apply_create_and_clear() {
    let src = create_table("remain");
    let tree0 = inspect_tables(&src);
    let cell = cell0(&tree0);
    let keys: Vec<&str> = cell
        .as_object()
        .unwrap()
        .keys()
        .map(|k| k.as_str())
        .collect();
    assert_eq!(
        keys,
        [
            "id",
            "row",
            "col",
            "gridSpan",
            "vMerge",
            "vAlign",
            "noWrap",
            "textDirection",
            "fitText",
            "hideMark",
            "headers",
            "fill",
            "cellMar",
            "borders",
            "tl2br",
            "tr2bl",
            "paragraphs"
        ]
    );
    assert!(cell["noWrap"].is_null());
    assert!(cell["textDirection"].is_null());
    assert!(cell["fitText"].is_null());
    assert!(cell["hideMark"].is_null());
    assert_eq!(cell["headers"], json!([]));

    let out = tmp_dir().join("remain-out.docx");
    apply(
        &src,
        &out,
        json!([{
            "op": "setCellFormat",
            "id": "table:1/r1c1",
            "noWrap": true,
            "textDirection": "tbRl",
            "fitText": true,
            "hideMark": true,
            "headers": ["A1"]
        }]),
    );
    let tree = inspect_tables(&out);
    let cell = cell0(&tree);
    assert_eq!(cell["noWrap"], true);
    assert_eq!(cell["textDirection"], "tbRl");
    assert_eq!(cell["fitText"], true);
    assert_eq!(cell["hideMark"], true);
    assert_eq!(cell["headers"], json!(["A1"]));
    let xml = unzip_document(&out);
    assert!(xml.contains("<w:noWrap"));
    assert!(xml.contains(r#"w:val="tbRl""#));
    assert!(xml.contains("<w:tcFitText"));
    assert!(xml.contains("<w:hideMark"));
    assert!(xml.contains("<w:header>A1</w:header>"));

    let only_dir = tmp_dir().join("remain-dir.docx");
    apply(
        &src,
        &only_dir,
        json!([{
            "op": "setCellFormat",
            "id": "table:1/r1c1",
            "textDirection": "btLr"
        }]),
    );
    assert_eq!(cell0(&inspect_tables(&only_dir))["textDirection"], "btLr");

    let cleared = tmp_dir().join("remain-clear.docx");
    apply(
        &out,
        &cleared,
        json!([{
            "op": "setCellFormat",
            "id": "table:1/r1c1",
            "noWrap": null,
            "textDirection": null,
            "fitText": null,
            "hideMark": null,
            "headers": []
        }]),
    );
    let tree_c = inspect_tables(&cleared);
    let cell_c = cell0(&tree_c);
    assert!(cell_c["noWrap"].is_null());
    assert!(cell_c["textDirection"].is_null());
    assert!(cell_c["fitText"].is_null());
    assert!(cell_c["hideMark"].is_null());
    assert_eq!(cell_c["headers"], json!([]));
    let xml_c = unzip_document(&cleared);
    assert!(!xml_c.contains("<w:noWrap"));
    assert!(!xml_c.contains("textDirection"));
    assert!(!xml_c.contains("tcFitText"));
    assert!(!xml_c.contains("hideMark"));
    assert!(!xml_c.contains("<w:headers"));

    let created = tmp_dir().join("remain-create.docx");
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": created.to_string_lossy(),
        "format": "docx",
        "document": {
            "paragraphs": [{ "text": "前" }],
            "tables": [{
                "cells": [["A1", "B1"], ["A2", "B2"]],
                "cellFormats": [{
                    "r": 1,
                    "c": 1,
                    "noWrap": true,
                    "textDirection": "tbRl",
                    "headers": ["A1"]
                }]
            }]
        }
    })
    .to_string())
    .unwrap();
    let tree_cr = inspect_tables(&created);
    let cell_cr = cell0(&tree_cr);
    assert_eq!(cell_cr["noWrap"], true);
    assert_eq!(cell_cr["textDirection"], "tbRl");
    assert_eq!(cell_cr["headers"], json!(["A1"]));

    let err_dir = apply_err(
        &src,
        json!([{ "op": "setCellFormat", "id": "table:1/r1c1", "textDirection": "sideways" }]),
    );
    assert_eq!(err_dir.code(), ErrorCode::InvalidField);
    let err_empty = apply_err(
        &src,
        json!([{ "op": "setCellFormat", "id": "table:1/r1c1" }]),
    );
    assert_eq!(err_empty.code(), ErrorCode::InvalidField);
    let err_hdr = apply_err(
        &src,
        json!([{ "op": "setCellFormat", "id": "table:1/r1c1", "headers": [""] }]),
    );
    assert_eq!(err_hdr.code(), ErrorCode::InvalidField);
    let err_id = apply_err(
        &src,
        json!([{ "op": "setCellFormat", "id": "table:1", "noWrap": true }]),
    );
    assert_eq!(err_id.code(), ErrorCode::InvalidId);

    let xlsx = tmp_dir().join("remain.xlsx");
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": xlsx.to_string_lossy(),
        "workbook": { "sheets": [{ "name": "Sheet1" }] }
    })
    .to_string())
    .unwrap();
    let err_x = run(&json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": xlsx.to_string_lossy(),
        "output": tmp_dir().join("remain-out.xlsx").to_string_lossy(),
        "ops": [{ "op": "setCellFormat", "id": "table:1/r1c1", "noWrap": true }]
    })
    .to_string())
    .unwrap_err();
    assert_eq!(err_x.code(), ErrorCode::InvalidField);
}
