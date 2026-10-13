//! docx 格正交边框：inspect `borders` 与 `setCellBorders`。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-docx-cell-borders");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn create_table(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let path = tmp_dir().join(format!("{tag}-{n}.docx"));
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "format": "docx",
            "document": {
                "paragraphs": [{ "text": "前" }],
                "tables": [{ "cells": [["A1", "B1"], ["A2", "B2"]] }]
            }
        })
        .to_string(),
        None,
    )
    .unwrap();
    path
}

fn inspect_tables(path: &Path) -> Value {
    let RunOutput::Inspect(tree) = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": path.to_string_lossy(),
            "scope": { "tables": true, "from": 1, "maxParas": 20 }
        })
        .to_string(),
        None,
    )
    .unwrap() else {
        panic!("inspect");
    };
    tree
}

fn apply(path: &Path, out: &Path, ops: Value) {
    run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": path.to_string_lossy(),
            "output": out.to_string_lossy(),
            "ops": ops
        })
        .to_string(),
        None,
    )
    .unwrap();
}

fn apply_err(path: &Path, ops: Value) -> ooxp::Error {
    let out = tmp_dir().join("should-not.docx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": path.to_string_lossy(),
            "output": out.to_string_lossy(),
            "ops": ops
        })
        .to_string(),
        None,
    )
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
fn default_create_cell_borders_null() {
    let src = create_table("default");
    let tree = inspect_tables(&src);
    let cell = cell0(&tree);
    let keys: Vec<&str> = cell
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
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
    assert!(cell["borders"].is_null());
    assert!(cell["tl2br"].is_null());
}

#[test]
fn set_cell_borders_patch_keeps_unnamed_and_null_keeps_diagonal() {
    let src = create_table("patch");
    let out = tmp_dir().join("top-double.docx");
    apply(
        &src,
        &out,
        json!([{
            "op": "setCellBorders",
            "id": "table:1/r1c1",
            "borders": { "top": { "val": "double", "sz": 12, "space": 0, "color": "000000" } }
        }]),
    );
    let tree = inspect_tables(&out);
    let cell = cell0(&tree);
    assert_eq!(cell["borders"]["top"]["val"], "double");
    assert_eq!(cell["borders"]["top"]["sz"], 12);
    assert_eq!(cell["borders"]["top"]["color"], "000000");
    assert!(cell["borders"]["left"].is_null());
    assert!(cell["borders"]["insideH"].is_null());

    let inside = tmp_dir().join("insideh.docx");
    apply(
        &out,
        &inside,
        json!([{
            "op": "setCellBorders",
            "id": "table:1/r1c1",
            "borders": { "insideH": { "val": "dashed", "sz": 4, "space": 0, "color": "auto" } }
        }]),
    );
    let tree_i = inspect_tables(&inside);
    let cell_i = cell0(&tree_i);
    assert_eq!(cell_i["borders"]["top"]["val"], "double");
    assert_eq!(cell_i["borders"]["insideH"]["val"], "dashed");

    let with_diag = tmp_dir().join("with-diag.docx");
    apply(
        &inside,
        &with_diag,
        json!([
            {
                "op": "setCellFormat",
                "id": "table:1/r1c1",
                "tl2br": { "val": "single", "sz": 12, "color": "0000FF" }
            },
            { "op": "setCellBorders", "id": "table:1/r1c1", "borders": null }
        ]),
    );
    let tree_c = inspect_tables(&with_diag);
    let cell_c = cell0(&tree_c);
    assert!(cell_c["borders"].is_null());
    assert_eq!(cell_c["tl2br"]["val"], "single");
    assert_eq!(cell_c["tl2br"]["color"], "0000FF");
    let xml = unzip_document(&with_diag);
    assert!(xml.contains("tl2br"));
}

#[test]
fn create_cell_formats_borders() {
    let created = tmp_dir().join("create-borders.docx");
    run(
        &json!({
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
                        "borders": {
                            "top": { "val": "double", "sz": 8, "space": 0, "color": "FF0000" }
                        }
                    }]
                }]
            }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let tree = inspect_tables(&created);
    let cell = cell0(&tree);
    assert_eq!(cell["borders"]["top"]["val"], "double");
    assert_eq!(cell["borders"]["top"]["color"], "FF0000");
    assert!(cell["borders"]["left"].is_null());
    assert!(cell["tl2br"].is_null());
}

#[test]
fn set_cell_borders_rejects() {
    let src = create_table("reject");
    let err_missing = apply_err(
        &src,
        json!([{ "op": "setCellBorders", "id": "table:1/r1c1" }]),
    );
    assert_eq!(err_missing.code(), ErrorCode::InvalidField);
    let err_empty = apply_err(
        &src,
        json!([{ "op": "setCellBorders", "id": "table:1/r1c1", "borders": {} }]),
    );
    assert_eq!(err_empty.code(), ErrorCode::InvalidField);
    let err_val = apply_err(
        &src,
        json!([{
            "op": "setCellBorders",
            "id": "table:1/r1c1",
            "borders": { "top": { "val": "not-a-border" } }
        }]),
    );
    assert_eq!(err_val.code(), ErrorCode::InvalidField);
    let err_table = apply_err(
        &src,
        json!([{
            "op": "setCellBorders",
            "id": "table:1",
            "borders": { "top": { "val": "single" } }
        }]),
    );
    assert_eq!(err_table.code(), ErrorCode::InvalidId);
    let err_para = apply_err(
        &src,
        json!([{
            "op": "setCellBorders",
            "id": "para:nope",
            "borders": { "top": { "val": "single" } }
        }]),
    );
    assert_eq!(err_para.code(), ErrorCode::InvalidId);

    let xlsx = tmp_dir().join("borders.xlsx");
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
    let err_x = run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": xlsx.to_string_lossy(),
            "output": tmp_dir().join("borders-out.xlsx").to_string_lossy(),
            "ops": [{
                "op": "setCellBorders",
                "id": "table:1/r1c1",
                "borders": { "top": { "val": "double" } }
            }]
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err_x.code(), ErrorCode::InvalidField);

    let pptx = tmp_dir().join("borders.pptx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": pptx.to_string_lossy(),
            "presentation": { "slides": [{ "shapes": [{ "text": "a" }] }] }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let err_p = run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": pptx.to_string_lossy(),
            "output": tmp_dir().join("borders-out.pptx").to_string_lossy(),
            "ops": [{
                "op": "setCellBorders",
                "id": "table:1/r1c1",
                "borders": { "top": { "val": "double" } }
            }]
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err_p.code(), ErrorCode::InvalidField);
}
