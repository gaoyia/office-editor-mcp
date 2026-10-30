//! DrawingML 形状文字格式：字号、字色、加粗、对齐。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-pptx-shape-format");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn tmp(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}.pptx"))
}

fn create_deck(tag: &str) -> PathBuf {
    let path = tmp(tag);
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "presentation": {
            "slides": [
                { "shapes": [{ "text": "标题" }] }
            ]
        }
    })
    .to_string())
    .unwrap();
    path
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

fn inspect_slide(path: &Path) -> Value {
    let RunOutput::Inspect(tree) = run(&json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": path.to_string_lossy(),
        "scope": { "slide": 1, "tables": true }
    })
    .to_string())
    .unwrap() else {
        panic!("inspect 应返回树");
    };
    tree
}

#[test]
fn create_reports_default_run_format() {
    let path = create_deck("def");
    let sh = &inspect_slide(&path)["slides"][0]["shapes"][0];
    assert_eq!(sh["sz"], 1800);
    assert_eq!(sh["color"], "000000");
    assert_eq!(sh["bold"], Value::Null);
    assert_eq!(sh["font"], "Calibri");
    assert_eq!(sh["eastAsiaFont"], "宋体");
    assert_eq!(sh["align"], Value::Null);
    assert_eq!(sh["textAnchor"], Value::Null);
    assert_eq!(sh["spcBef"], Value::Null);
    assert_eq!(sh["spcAft"], Value::Null);
    assert_eq!(sh["lnSpc"], Value::Null);
    assert_eq!(sh["runs"][0]["spc"], Value::Null);
    assert_eq!(sh["runs"][0]["kern"], Value::Null);
}

#[test]
fn set_shape_format_size_color_bold_align() {
    let src = create_deck("fmt");
    let out = tmp("fmt-out");
    apply(
        &src,
        &out,
        json!([{
            "op": "setShapeFormat",
            "id": "slide:1/shape:2",
            "sz": 3600,
            "color": "c41e3a",
            "bold": true,
            "align": "ctr",
            "textAnchor": "ctr"
        }]),
    )
    .unwrap();
    let sh = &inspect_slide(&out)["slides"][0]["shapes"][0];
    assert_eq!(sh["text"], "标题");
    assert_eq!(sh["sz"], 3600);
    assert_eq!(sh["color"], "C41E3A");
    assert_eq!(sh["bold"], true);
    assert_eq!(sh["align"], "ctr");
    assert_eq!(sh["textAnchor"], "ctr");
}

#[test]
fn insert_shape_accepts_format() {
    let src = create_deck("ins");
    let out = tmp("ins-out");
    apply(
        &src,
        &out,
        json!([{
            "op": "insertShape",
            "id": "slide:1",
            "text": "新框",
            "sz": 2800,
            "color": "0000FF",
            "italic": true,
            "font": "Arial",
            "eastAsiaFont": "黑体"
        }]),
    )
    .unwrap();
    let tree = inspect_slide(&out);
    let shapes = tree["slides"][0]["shapes"].as_array().unwrap();
    let neu = shapes.iter().find(|s| s["text"] == "新框").unwrap();
    assert_eq!(neu["sz"], 2800);
    assert_eq!(neu["color"], "0000FF");
    assert_eq!(neu["italic"], true);
    assert_eq!(neu["font"], "Arial");
    assert_eq!(neu["eastAsiaFont"], "黑体");
}

#[test]
fn create_shape_format_fields() {
    let path = tmp("create-fmt");
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "presentation": {
            "slides": [{
                "shapes": [{
                    "text": "大标题",
                    "sz": 4400,
                    "color": "1F4E79",
                    "bold": true,
                    "align": "ctr"
                }]
            }]
        }
    })
    .to_string())
    .unwrap();
    let sh = &inspect_slide(&path)["slides"][0]["shapes"][0];
    assert_eq!(sh["sz"], 4400);
    assert_eq!(sh["color"], "1F4E79");
    assert_eq!(sh["bold"], true);
    assert_eq!(sh["align"], "ctr");
}

#[test]
fn table_cell_format() {
    let src = create_deck("tbl");
    let with_tbl = tmp("tbl-ins");
    apply(
        &src,
        &with_tbl,
        json!([{
            "op": "insertTable",
            "id": "slide:1",
            "rows": 1,
            "cols": 1,
            "cells": [["格"]]
        }]),
    )
    .unwrap();
    let tree = inspect_slide(&with_tbl);
    let cell_id = tree["slides"][0]["tables"][0]["rows"][0]["cells"][0]["id"].clone();
    let out = tmp("tbl-fmt");
    apply(
        &with_tbl,
        &out,
        json!([{
            "op": "setShapeFormat",
            "id": cell_id,
            "sz": 2000,
            "color": "FF0000"
        }]),
    )
    .unwrap();
    let cell = &inspect_slide(&out)["slides"][0]["tables"][0]["rows"][0]["cells"][0];
    assert_eq!(cell["text"], "格");
    assert_eq!(cell["sz"], 2000);
    assert_eq!(cell["color"], "FF0000");
}

#[test]
fn rejects_missing_field_and_bad_sz_and_docx() {
    let src = create_deck("bad");
    let dest = tmp("no-field");
    let err = apply(
        &src,
        &dest,
        json!([{ "op": "setShapeFormat", "id": "slide:1/shape:2" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!dest.exists());

    let dest = tmp("bad-sz");
    let err = apply(
        &src,
        &dest,
        json!([{ "op": "setShapeFormat", "id": "slide:1/shape:2", "sz": 1 }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!dest.exists());

    let dest = tmp("bad-align");
    let err = apply(
        &src,
        &dest,
        json!([{ "op": "setShapeFormat", "id": "slide:1/shape:2", "align": "center" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!dest.exists());

    let docx = tmp("doc").with_extension("docx");
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": docx.to_string_lossy(),
        "document": { "paragraphs": [{ "text": "hi" }] }
    })
    .to_string())
    .unwrap();
    let err = run(&json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": docx.to_string_lossy(),
        "output": tmp("docx-out").with_extension("docx").to_string_lossy(),
        "ops": [{ "op": "setShapeFormat", "id": "slide:1/shape:2", "sz": 1800 }]
    })
    .to_string())
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

#[test]
fn set_shape_format_spacing_kern_and_anchor() {
    let src = create_deck("spc");
    let out = tmp("spc-out");
    apply(
        &src,
        &out,
        json!([{
            "op": "setShapeFormat",
            "id": "slide:1/shape:2",
            "spcBef": { "type": "pts", "val": 1200 },
            "spcAft": { "type": "pts", "val": 600 },
            "lnSpc": { "type": "pct", "val": 200000 },
            "spc": 200,
            "kern": 1200,
            "textAnchor": "just"
        }]),
    )
    .unwrap();
    let sh = &inspect_slide(&out)["slides"][0]["shapes"][0];
    assert_eq!(sh["text"], "标题");
    assert_eq!(sh["textAnchor"], "just");
    assert_eq!(sh["spcBef"]["type"], "pts");
    assert_eq!(sh["spcBef"]["val"], 1200);
    assert_eq!(sh["spcAft"]["type"], "pts");
    assert_eq!(sh["spcAft"]["val"], 600);
    assert_eq!(sh["lnSpc"]["type"], "pct");
    assert_eq!(sh["lnSpc"]["val"], 200000);
    assert_eq!(sh["runs"][0]["spc"], 200);
    assert_eq!(sh["runs"][0]["kern"], 1200);

    let cleared = tmp("spc-clear");
    apply(
        &out,
        &cleared,
        json!([{
            "op": "setShapeFormat",
            "id": "slide:1/shape:2",
            "spcBef": null,
            "lnSpc": null,
            "spc": null,
            "textAnchor": "dist"
        }]),
    )
    .unwrap();
    let sh2 = &inspect_slide(&cleared)["slides"][0]["shapes"][0];
    assert_eq!(sh2["textAnchor"], "dist");
    assert_eq!(sh2["spcBef"], Value::Null);
    assert_eq!(sh2["spcAft"]["val"], 600);
    assert_eq!(sh2["lnSpc"], Value::Null);
    assert_eq!(sh2["runs"][0]["spc"], Value::Null);
    assert_eq!(sh2["runs"][0]["kern"], 1200);
}

#[test]
fn table_cell_spacing() {
    let src = create_deck("tbl-spc");
    let with_tbl = tmp("tbl-spc-ins");
    apply(
        &src,
        &with_tbl,
        json!([{
            "op": "insertTable",
            "id": "slide:1",
            "rows": 1,
            "cols": 1,
            "cells": [["格"]]
        }]),
    )
    .unwrap();
    let cell_id =
        inspect_slide(&with_tbl)["slides"][0]["tables"][0]["rows"][0]["cells"][0]["id"].clone();
    let out = tmp("tbl-spc-fmt");
    apply(
        &with_tbl,
        &out,
        json!([{
            "op": "setShapeFormat",
            "id": cell_id,
            "lnSpc": { "type": "pts", "val": 2400 },
            "spc": -100
        }]),
    )
    .unwrap();
    let cell = &inspect_slide(&out)["slides"][0]["tables"][0]["rows"][0]["cells"][0];
    assert_eq!(cell["lnSpc"]["type"], "pts");
    assert_eq!(cell["lnSpc"]["val"], 2400);
    assert_eq!(cell["runs"][0]["spc"], -100);
}

#[test]
fn rejects_bad_spacing_and_kern() {
    let src = create_deck("bad-spc");
    let dest = tmp("bad-type");
    let err = apply(
        &src,
        &dest,
        json!([{
            "op": "setShapeFormat",
            "id": "slide:1/shape:2",
            "lnSpc": { "type": "lines", "val": 1 }
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!dest.exists());

    let dest = tmp("bad-kern");
    let err = apply(
        &src,
        &dest,
        json!([{
            "op": "setShapeFormat",
            "id": "slide:1/shape:2",
            "kern": -1
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!dest.exists());

    let dest = tmp("big-pts");
    let err = apply(
        &src,
        &dest,
        json!([{
            "op": "setShapeFormat",
            "id": "slide:1/shape:2",
            "spcBef": { "type": "pts", "val": 158401 }
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::TruncatedLimit);
    assert!(!dest.exists());
}
