//! docx 表格宽高与对齐：首选宽度、列宽、行高、行内对齐。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-docx-table-width");
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
fn inspect_reports_width_align_height() {
    let src = create_table("insp");
    let tree = inspect_tables(&src);
    let t = &tree["tables"][0];
    assert!(t["align"].is_null());
    assert!(t["indent"].is_null());
    assert_eq!(t["width"]["type"], "auto");
    assert_eq!(t["width"]["w"], 0);
    assert!(t["layout"].is_null());
    assert_eq!(t["colWidths"], json!([1440, 1440]));
    assert!(t["rows"][0]["height"].is_null());
}

#[test]
fn set_table_width_align_and_row_height() {
    let src = create_table("wh");
    let out = tmp_dir().join("wh-out.docx");
    apply(
        &src,
        &out,
        json!([
            {
                "op": "setTableWidth",
                "id": "table:1",
                "type": "dxa",
                "w": 3600,
                "cols": [1800, 1800]
            },
            {
                "op": "setTableAlign",
                "id": "table:1",
                "align": "center"
            },
            {
                "op": "setTableRowHeight",
                "id": "table:1/row:1",
                "h": 400,
                "rule": "atLeast"
            }
        ]),
    );
    let tree = inspect_tables(&out);
    let t = &tree["tables"][0];
    assert_eq!(t["align"], "center");
    assert_eq!(t["width"]["type"], "dxa");
    assert_eq!(t["width"]["w"], 3600);
    assert_eq!(t["layout"], "fixed");
    assert_eq!(t["colWidths"], json!([1800, 1800]));
    assert_eq!(t["rows"][0]["height"]["h"], 400);
    assert_eq!(t["rows"][0]["height"]["rule"], "atLeast");
    assert!(t["rows"][1]["height"].is_null());
    let xml = unzip_document(&out);
    assert!(xml.contains(r#"w:type="fixed""#) || xml.contains(r#"type="fixed""#));
    assert!(xml.contains(r#"w:val="center""#) || xml.contains(r#"val="center""#));
    assert!(xml.contains("1800"));
}

#[test]
fn set_table_width_scales_existing_cols() {
    let src = create_table("scale");
    let out = tmp_dir().join("scale-out.docx");
    apply(
        &src,
        &out,
        json!([{ "op": "setTableWidth", "id": "table:1", "type": "dxa", "w": 3600 }]),
    );
    let t = &inspect_tables(&out)["tables"][0];
    assert_eq!(t["width"]["w"], 3600);
    assert_eq!(t["colWidths"], json!([1800, 1800]));
}

#[test]
fn set_table_align_indent_and_clear() {
    let src = create_table("ind");
    let mid = tmp_dir().join("ind-mid.docx");
    apply(
        &src,
        &mid,
        json!([{ "op": "setTableAlign", "id": "table:1", "align": "right", "indent": 240 }]),
    );
    let t = &inspect_tables(&mid)["tables"][0];
    assert_eq!(t["align"], "right");
    assert_eq!(t["indent"], 240);
    let out = tmp_dir().join("ind-clear.docx");
    apply(
        &mid,
        &out,
        json!([{ "op": "setTableAlign", "id": "table:1", "align": null, "indent": null }]),
    );
    let t2 = &inspect_tables(&out)["tables"][0];
    assert!(t2["align"].is_null());
    assert!(t2["indent"].is_null());
}

#[test]
fn set_table_row_height_null_removes() {
    let src = create_table("rh");
    let mid = tmp_dir().join("rh-mid.docx");
    apply(
        &src,
        &mid,
        json!([{ "op": "setTableRowHeight", "id": "table:1/row:1", "h": 400 }]),
    );
    assert_eq!(
        inspect_tables(&mid)["tables"][0]["rows"][0]["height"]["rule"],
        "atLeast"
    );
    let out = tmp_dir().join("rh-clear.docx");
    apply(
        &mid,
        &out,
        json!([{ "op": "setTableRowHeight", "id": "table:1/row:1", "h": null }]),
    );
    assert!(inspect_tables(&out)["tables"][0]["rows"][0]["height"].is_null());
}

#[test]
fn set_table_width_pct_and_auto() {
    let src = create_table("pct");
    let pct = tmp_dir().join("pct-out.docx");
    apply(
        &src,
        &pct,
        json!([{ "op": "setTableWidth", "id": "table:1", "type": "pct", "w": 5000 }]),
    );
    let t = &inspect_tables(&pct)["tables"][0];
    assert_eq!(t["width"]["type"], "pct");
    assert_eq!(t["width"]["w"], 5000);
    assert_eq!(t["layout"], "autofit");
    let auto = tmp_dir().join("auto-out.docx");
    apply(
        &pct,
        &auto,
        json!([{ "op": "setTableWidth", "id": "table:1", "type": "auto" }]),
    );
    let t2 = &inspect_tables(&auto)["tables"][0];
    assert_eq!(t2["width"]["type"], "auto");
    assert_eq!(t2["width"]["w"], 0);
    assert_eq!(t2["layout"], "autofit");
}

#[test]
fn create_table_width_align_height() {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let path = tmp_dir().join(format!("create-{n}.docx"));
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "format": "docx",
            "document": {
                "paragraphs": [{ "text": "前" }],
                "tables": [{
                    "cells": [["A1", "B1"], ["A2", "B2"]],
                    "align": "center",
                    "indent": 120,
                    "width": { "type": "dxa", "w": 3600 },
                    "cols": [1800, 1800],
                    "rowHeights": [
                        { "h": 400, "rule": "exact" },
                        { "h": null }
                    ]
                }]
            }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let t = &inspect_tables(&path)["tables"][0];
    assert_eq!(t["align"], "center");
    assert_eq!(t["indent"], 120);
    assert_eq!(t["width"]["w"], 3600);
    assert_eq!(t["colWidths"], json!([1800, 1800]));
    assert_eq!(t["rows"][0]["height"]["h"], 400);
    assert_eq!(t["rows"][0]["height"]["rule"], "exact");
    assert!(t["rows"][1]["height"].is_null());
}

#[test]
fn set_table_width_rejects() {
    let src = create_table("bad");
    let err = apply_err(
        &src,
        json!([{ "op": "setTableWidth", "id": "para:nope", "w": 1440 }]),
    );
    assert_eq!(err.code(), ErrorCode::InvalidId);
    let err = apply_err(&src, json!([{ "op": "setTableWidth", "id": "table:1" }]));
    assert_eq!(err.code(), ErrorCode::InvalidField);
    let err = apply_err(
        &src,
        json!([{ "op": "setTableWidth", "id": "table:1", "type": "auto", "w": 100 }]),
    );
    assert_eq!(err.code(), ErrorCode::InvalidField);
    let err = apply_err(
        &src,
        json!([{ "op": "setTableWidth", "id": "table:1", "cols": [1, 2, 3] }]),
    );
    assert_eq!(err.code(), ErrorCode::InvalidField);
    let err = apply_err(
        &src,
        json!([{ "op": "setTableAlign", "id": "table:1", "align": "middle" }]),
    );
    assert_eq!(err.code(), ErrorCode::InvalidField);
    let err = apply_err(
        &src,
        json!([{ "op": "setTableRowHeight", "id": "table:1", "h": 400 }]),
    );
    assert_eq!(err.code(), ErrorCode::InvalidId);
    let err = apply_err(
        &src,
        json!([{ "op": "setTableRowHeight", "id": "table:1/row:1", "h": null, "rule": "exact" }]),
    );
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

#[test]
fn set_table_width_not_for_xlsx() {
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
            "ops": [{ "op": "setTableWidth", "id": "table:1", "w": 1440 }]
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}
