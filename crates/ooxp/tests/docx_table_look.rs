//! docx 表格外观：浮动、底纹、边距、斜线、表格样式。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-docx-table-look");
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

#[test]
fn table_float_and_cell_format() {
    let src = create_table("float");
    let out = tmp_dir().join("float-out.docx");
    apply(
        &src,
        &out,
        json!([
            {
                "op": "setTableFloat",
                "id": "table:1",
                "pos": { "horzAnchor": "margin", "vertAnchor": "text", "x": 1440, "y": 720 }
            },
            {
                "op": "setTableCellMar",
                "id": "table:1",
                "top": 120,
                "left": 160,
                "bottom": 120,
                "right": 160
            },
            {
                "op": "setCellFormat",
                "id": "table:1/r1c1",
                "fill": "FF0000",
                "top": 240,
                "tl2br": { "val": "single", "sz": 12, "color": "0000FF" }
            }
        ]),
    );
    let tree = inspect_tables(&out);
    let t = &tree["tables"][0];
    assert_eq!(t["pos"]["x"], 1440);
    assert_eq!(t["pos"]["y"], 720);
    assert_eq!(t["pos"]["horzAnchor"], "margin");
    assert_eq!(t["cellMar"]["top"], 120);
    assert_eq!(t["cellMar"]["left"], 160);
    let cell = &t["rows"][0]["cells"][0];
    assert_eq!(cell["fill"], "FF0000");
    assert_eq!(cell["cellMar"]["top"], 240);
    assert_eq!(cell["tl2br"]["val"], "single");
    assert_eq!(cell["tl2br"]["color"], "0000FF");
    let xml = unzip_document(&out);
    assert!(xml.contains("tblpPr"));
    assert!(xml.contains("tblOverlap"));
    assert!(xml.contains("tl2br"));
    assert!(xml.contains("FF0000"));
}

#[test]
fn table_style_and_look() {
    let src = create_table("style");
    let mid = tmp_dir().join("tbl-style-ins.docx");
    apply(
        &src,
        &mid,
        json!([{
            "op": "insertStyle",
            "id": "docx:document",
            "styleId": "ooxpTbl",
            "type": "table"
        }]),
    );
    let out = tmp_dir().join("tbl-style.docx");
    apply(
        &mid,
        &out,
        json!([{
            "op": "setTableStyle",
            "id": "table:1",
            "style": "style:ooxpTbl",
            "look": { "firstRow": true, "bandedRows": true, "bandedColumns": false }
        }]),
    );
    let tree = inspect_tables(&out);
    let t = &tree["tables"][0];
    assert_eq!(t["style"], "ooxpTbl");
    assert_eq!(t["look"]["firstRow"], true);
    assert_eq!(t["look"]["bandedRows"], true);
    assert_eq!(t["look"]["bandedColumns"], false);
    let err = apply_err(
        &src,
        json!([{ "op": "setTableStyle", "id": "table:1", "style": "style:Normal" }]),
    );
    assert_eq!(err.code(), ErrorCode::InvalidField);
    let err_pos = apply_err(
        &src,
        json!([{ "op": "setTableFloat", "id": "para:nope", "pos": { "x": 0 } }]),
    );
    assert_eq!(err_pos.code(), ErrorCode::InvalidId);
}

#[test]
fn table_look_not_for_xlsx() {
    let xlsx = tmp_dir().join("t.xlsx");
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
    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": xlsx.to_string_lossy(),
            "output": tmp_dir().join("t-out.xlsx").to_string_lossy(),
            "ops": [{ "op": "setTableFloat", "id": "table:1", "pos": null }]
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

#[test]
fn cell_valign_inspect_apply_create_and_clear() {
    let src = create_table("valign");
    let tree0 = inspect_tables(&src);
    let cell0 = &tree0["tables"][0]["rows"][0]["cells"][0];
    assert!(cell0["vAlign"].is_null());
    assert_eq!(cell0["vMerge"], Value::Null);

    let out = tmp_dir().join("valign-out.docx");
    apply(
        &src,
        &out,
        json!([{
            "op": "setCellFormat",
            "id": "table:1/r1c1",
            "vAlign": "center"
        }]),
    );
    let tree = inspect_tables(&out);
    assert_eq!(tree["tables"][0]["rows"][0]["cells"][0]["vAlign"], "center");
    let xml = unzip_document(&out);
    assert!(xml.contains("w:vAlign") || xml.contains("vAlign"));
    assert!(xml.contains("center"));

    let cleared = tmp_dir().join("valign-clear.docx");
    apply(
        &out,
        &cleared,
        json!([{
            "op": "setCellFormat",
            "id": "table:1/r1c1",
            "vAlign": null
        }]),
    );
    let tree_c = inspect_tables(&cleared);
    assert!(tree_c["tables"][0]["rows"][0]["cells"][0]["vAlign"].is_null());
    let xml_c = unzip_document(&cleared);
    assert!(!xml_c.contains("vAlign"));

    let created = tmp_dir().join("valign-create.docx");
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
                    "cellFormats": [{ "r": 1, "c": 1, "vAlign": "bottom" }]
                }]
            }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let tree_cr = inspect_tables(&created);
    assert_eq!(
        tree_cr["tables"][0]["rows"][0]["cells"][0]["vAlign"],
        "bottom"
    );

    let err = apply_err(
        &src,
        json!([{ "op": "setCellFormat", "id": "table:1/r1c1", "vAlign": "middle" }]),
    );
    assert_eq!(err.code(), ErrorCode::InvalidField);
    let err_empty = apply_err(
        &src,
        json!([{ "op": "setCellFormat", "id": "table:1/r1c1" }]),
    );
    assert_eq!(err_empty.code(), ErrorCode::InvalidField);
    let err_id = apply_err(
        &src,
        json!([{ "op": "setCellFormat", "id": "para:nope", "vAlign": "center" }]),
    );
    assert_eq!(err_id.code(), ErrorCode::InvalidId);

    let xlsx = tmp_dir().join("valign.xlsx");
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
            "output": tmp_dir().join("valign-out.xlsx").to_string_lossy(),
            "ops": [{ "op": "setCellFormat", "id": "table:1/r1c1", "vAlign": "center" }]
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err_x.code(), ErrorCode::InvalidField);
}
