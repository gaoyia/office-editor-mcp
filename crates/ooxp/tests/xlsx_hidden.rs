//! 没有大纲级的隐藏行 / 列：inspect 进 outline，setRowHidden / setColHidden。

use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::json;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-xlsx-hidden");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn tmp(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}.xlsx"))
}

fn create_book(tag: &str) -> PathBuf {
    let path = tmp(tag);
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "workbook": {
            "sheets": [{
                "name": "Sheet1",
                "cells": [
                    { "ref": "A1", "value": 1 },
                    { "ref": "A2", "value": 2 },
                    { "ref": "A3", "value": 3 },
                    { "ref": "B1", "value": "B" }
                ]
            }]
        }
    })
    .to_string())
    .unwrap();
    path
}

fn apply(src: &Path, out: &Path, ops: serde_json::Value) -> Result<RunOutput, ooxp::Error> {
    run(&json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": out.to_string_lossy(),
        "ops": ops,
    })
    .to_string())
}

fn inspect_tree(path: &Path) -> serde_json::Value {
    let RunOutput::Inspect(tree) = run(&json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": path.to_string_lossy(),
        "scope": { "sheet": "Sheet1", "range": "A1:C8" }
    })
    .to_string())
    .unwrap() else {
        panic!("inspect 应返回树");
    };
    tree
}

fn sheet_xml(path: &Path) -> String {
    let bytes = std::fs::read(path).unwrap();
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    let mut xml = String::new();
    zip.by_name("xl/worksheets/sheet1.xml")
        .unwrap()
        .read_to_string(&mut xml)
        .unwrap();
    xml
}

#[test]
fn hide_row_and_col_enter_outline() {
    let src = create_book("hide");
    let out = tmp("hide-out");
    apply(
        &src,
        &out,
        json!([
            { "op": "setRowHidden", "id": "sheet:Sheet1", "rows": "3", "hidden": true },
            { "op": "setColHidden", "id": "sheet:Sheet1", "col": "B", "hidden": true }
        ]),
    )
    .unwrap();
    let o = &inspect_tree(&out)["sheets"][0]["outline"];
    assert_eq!(
        o["rows"],
        json!([{ "r": 3, "level": 0, "hidden": true, "collapsed": false }])
    );
    assert_eq!(
        o["cols"],
        json!([{
            "min": "B",
            "max": "B",
            "level": 0,
            "hidden": true,
            "collapsed": false
        }])
    );
    let xml = sheet_xml(&out);
    assert!(xml.contains("hidden=\"1\""), "{xml}");
    assert!(!xml.contains("width=\"0\""), "{xml}");
}

#[test]
fn unhide_drops_level_zero_from_outline() {
    let src = create_book("unhide");
    let hidden = tmp("unhide-mid");
    apply(
        &src,
        &hidden,
        json!([
            { "op": "setRowHidden", "id": "sheet:Sheet1", "rows": "3", "hidden": true },
            { "op": "setColHidden", "id": "sheet:Sheet1", "col": "B", "hidden": true }
        ]),
    )
    .unwrap();
    let out = tmp("unhide-out");
    apply(
        &hidden,
        &out,
        json!([
            { "op": "setRowHidden", "id": "sheet:Sheet1", "rows": "3", "hidden": false },
            { "op": "setColHidden", "id": "sheet:Sheet1", "col": "B", "hidden": false }
        ]),
    )
    .unwrap();
    let o = &inspect_tree(&out)["sheets"][0]["outline"];
    assert_eq!(o["rows"], json!([]));
    assert_eq!(o["cols"], json!([]));
}

#[test]
fn hide_does_not_mark_recalc() {
    let src = create_book("recalc");
    let out = tmp("recalc-out");
    apply(
        &src,
        &out,
        json!([{ "op": "setRowHidden", "id": "sheet:Sheet1", "rows": "3", "hidden": true }]),
    )
    .unwrap();
    let bytes = std::fs::read(&out).unwrap();
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    let mut wb = String::new();
    zip.by_name("xl/workbook.xml")
        .unwrap()
        .read_to_string(&mut wb)
        .unwrap();
    assert!(!wb.contains("fullCalcOnLoad"), "{wb}");
}

#[test]
fn chartsheet_rejects_hidden() {
    let src = create_book("cs");
    let with_cs = tmp("cs-mid");
    apply(
        &src,
        &with_cs,
        json!([{
            "op": "insertChartSheet",
            "id": "xlsx:workbook",
            "name": "Chart1",
            "dataFrom": "A1",
            "type": "col",
            "title": "销量",
            "series": [{
                "name": "Q1",
                "categories": ["甲", "乙", "丙"],
                "values": [1, 2, 3]
            }]
        }]),
    )
    .unwrap();
    let out = tmp("cs-fail");
    let err = apply(
        &with_cs,
        &out,
        json!([{ "op": "setRowHidden", "id": "sheet:Chart1", "rows": "1", "hidden": true }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());
}

#[test]
fn rejects_missing_hidden_and_slide_id() {
    let src = create_book("bad");
    let miss = tmp("miss");
    let err = apply(
        &src,
        &miss,
        json!([{ "op": "setRowHidden", "id": "sheet:Sheet1", "rows": "3" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!miss.exists());

    let slide = tmp("slide");
    let err = apply(
        &src,
        &slide,
        json!([{
            "op": "setColHidden",
            "id": "slide:1",
            "col": "B",
            "hidden": true
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    assert!(!slide.exists());
}

#[test]
fn docx_rejects_set_row_hidden() {
    let path = tmp_dir().join(format!(
        "docx-host-{}.docx",
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "document": { "paragraphs": [{ "text": "x" }] }
    })
    .to_string())
    .unwrap();
    let out = tmp_dir().join(format!(
        "docx-out-{}.docx",
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    let err = run(&json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": path.to_string_lossy(),
        "output": out.to_string_lossy(),
        "ops": [{ "op": "setRowHidden", "id": "sheet:Sheet1", "rows": "3", "hidden": true }]
    })
    .to_string())
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());
}
