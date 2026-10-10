//! 行 / 列大纲：inspect 始终给出 outline，apply 分组、取消与折叠。

use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::json;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-xlsx-outline");
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
                        { "ref": "A1", "value": 1 },
                        { "ref": "A2", "value": 2 },
                        { "ref": "A3", "value": 3 },
                        { "ref": "A4", "value": 4 },
                        { "ref": "A5", "value": 5 },
                        { "ref": "B1", "value": "B" },
                        { "ref": "C1", "value": "C" }
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

fn inspect_tree(path: &Path, with_cells: bool) -> serde_json::Value {
    let mut req = json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": path.to_string_lossy(),
    });
    if with_cells {
        req["scope"] = json!({ "sheet": "Sheet1", "range": "A1:C8" });
    }
    let RunOutput::Inspect(tree) = run(&req.to_string(), None).unwrap() else {
        panic!("inspect 应返回树");
    };
    tree
}

fn object_keys(v: &serde_json::Value) -> Vec<&str> {
    v.as_object().unwrap().keys().map(|s| s.as_str()).collect()
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
fn inspect_default_outline_without_cells() {
    let path = create_book("empty");
    let tree = inspect_tree(&path, false);
    let o = &tree["sheets"][0]["outline"];
    assert_eq!(
        object_keys(o),
        ["summaryBelow", "summaryRight", "rows", "cols"]
    );
    assert_eq!(o["summaryBelow"], json!(true));
    assert_eq!(o["summaryRight"], json!(true));
    assert_eq!(o["rows"], json!([]));
    assert_eq!(o["cols"], json!([]));
}

#[test]
fn group_rows_and_cols() {
    let src = create_book("group");
    let out = tmp("group-out");
    apply(
        &src,
        &out,
        json!([
            { "op": "groupRows", "id": "sheet:Sheet1", "rows": "2:4" },
            { "op": "groupCols", "id": "sheet:Sheet1", "col": "B:C" }
        ]),
    )
    .unwrap();
    let tree = inspect_tree(&out, true);
    let o = &tree["sheets"][0]["outline"];
    assert_eq!(
        o["rows"],
        json!([
            { "r": 2, "level": 1, "hidden": false, "collapsed": false },
            { "r": 3, "level": 1, "hidden": false, "collapsed": false },
            { "r": 4, "level": 1, "hidden": false, "collapsed": false }
        ])
    );
    assert_eq!(
        o["cols"],
        json!([{
            "min": "B",
            "max": "C",
            "level": 1,
            "hidden": false,
            "collapsed": false
        }])
    );
    let xml = sheet_xml(&out);
    assert!(xml.contains("outlineLevel=\"1\""), "{xml}");
    assert!(xml.contains("outlineLevelRow=\"1\""), "{xml}");
    assert!(xml.contains("outlineLevelCol=\"1\""), "{xml}");
    assert!(xml.contains("defaultRowHeight=\"15\""), "{xml}");
    assert!(xml.contains("width="), "{xml}");
    assert!(!xml.contains("summaryBelow"), "{xml}");
    let layout_b = tree["sheets"][0]["cols"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c.get("min") == Some(&json!("B")))
        .expect("groupCols 新建列应写入缺省列宽");
    assert_eq!(layout_b["max"], json!("C"));
    assert!(layout_b["width"].as_f64().unwrap() > 8.0);
}

#[test]
fn nested_group_increments_level() {
    let src = create_book("nest");
    let out = tmp("nest-out");
    apply(
        &src,
        &out,
        json!([
            { "op": "groupRows", "id": "sheet:Sheet1", "rows": "2:5" },
            { "op": "groupRows", "id": "sheet:Sheet1", "rows": "2:3" }
        ]),
    )
    .unwrap();
    let o = &inspect_tree(&out, true)["sheets"][0]["outline"]["rows"];
    assert_eq!(o[0]["r"], json!(2));
    assert_eq!(o[0]["level"], json!(2));
    assert_eq!(o[1]["level"], json!(2));
    assert_eq!(o[2]["r"], json!(4));
    assert_eq!(o[2]["level"], json!(1));
    assert_eq!(o[3]["r"], json!(5));
    assert_eq!(o[3]["level"], json!(1));
    let xml = sheet_xml(&out);
    assert!(xml.contains("outlineLevelRow=\"2\""), "{xml}");
}

#[test]
fn collapse_hides_detail_and_marks_summary() {
    let src = create_book("collapse");
    let grouped = tmp("collapse-g");
    apply(
        &src,
        &grouped,
        json!([{ "op": "groupRows", "id": "sheet:Sheet1", "rows": "2:4" }]),
    )
    .unwrap();
    let out = tmp("collapse-out");
    apply(
        &grouped,
        &out,
        json!([{
            "op": "setOutline",
            "id": "sheet:Sheet1",
            "collapseRows": "2:4",
            "collapsed": true
        }]),
    )
    .unwrap();
    let rows = &inspect_tree(&out, true)["sheets"][0]["outline"]["rows"];
    assert_eq!(rows[0]["hidden"], json!(true));
    assert_eq!(rows[1]["hidden"], json!(true));
    assert_eq!(rows[2]["hidden"], json!(true));
    let summary = rows
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["r"] == json!(5))
        .unwrap();
    assert_eq!(summary["collapsed"], json!(true));
    assert_eq!(summary["level"], json!(0));
}

#[test]
fn ungroup_clears_levels() {
    let src = create_book("ungroup");
    let grouped = tmp("ungroup-g");
    apply(
        &src,
        &grouped,
        json!([
            { "op": "groupRows", "id": "sheet:Sheet1", "rows": "2:4" },
            { "op": "groupCols", "id": "sheet:Sheet1", "col": "B:C" }
        ]),
    )
    .unwrap();
    let out = tmp("ungroup-out");
    apply(
        &grouped,
        &out,
        json!([
            { "op": "ungroupRows", "id": "sheet:Sheet1", "rows": "2:4" },
            { "op": "ungroupCols", "id": "sheet:Sheet1", "col": "B:C" }
        ]),
    )
    .unwrap();
    let o = &inspect_tree(&out, true)["sheets"][0]["outline"];
    assert_eq!(o["rows"], json!([]));
    assert_eq!(o["cols"], json!([]));
    let xml = sheet_xml(&out);
    assert!(!xml.contains("outlineLevelRow"), "{xml}");
    assert!(!xml.contains("outlineLevelCol"), "{xml}");
}

#[test]
fn set_outline_summary_above() {
    let src = create_book("above");
    let out = tmp("above-out");
    apply(
        &src,
        &out,
        json!([{
            "op": "setOutline",
            "id": "sheet:Sheet1",
            "summaryBelow": false,
            "summaryRight": false
        }]),
    )
    .unwrap();
    let o = &inspect_tree(&out, false)["sheets"][0]["outline"];
    assert_eq!(o["summaryBelow"], json!(false));
    assert_eq!(o["summaryRight"], json!(false));
    let xml = sheet_xml(&out);
    assert!(xml.contains("summaryBelow=\"0\""), "{xml}");
    assert!(xml.contains("summaryRight=\"0\""), "{xml}");
}

#[test]
fn outline_does_not_mark_recalc() {
    let src = create_book("recalc");
    let out = tmp("recalc-out");
    apply(
        &src,
        &out,
        json!([{ "op": "groupRows", "id": "sheet:Sheet1", "rows": "2:4" }]),
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
fn missing_set_outline_field_is_invalid() {
    let src = create_book("empty-op");
    let out = tmp("empty-op-out");
    let err = apply(
        &src,
        &out,
        json!([{ "op": "setOutline", "id": "sheet:Sheet1" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());
}

#[test]
fn outline_level_eight_rejected() {
    let src = create_book("max");
    let mut cur = src;
    for n in 0..7 {
        let next = tmp(&format!("max-{n}"));
        apply(
            &cur,
            &next,
            json!([{ "op": "groupRows", "id": "sheet:Sheet1", "rows": "2" }]),
        )
        .unwrap();
        cur = next;
    }
    let out = tmp("max-fail");
    let err = apply(
        &cur,
        &out,
        json!([{ "op": "groupRows", "id": "sheet:Sheet1", "rows": "2" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());
}

#[test]
fn create_writes_outline() {
    let path = tmp("create-outline");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "workbook": {
                "sheets": [{
                    "name": "Sheet1",
                    "outline": { "summaryBelow": false }
                }]
            }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let tree = inspect_tree(&path, false);
    assert_eq!(tree["sheets"][0]["outline"]["summaryBelow"], false);
}

#[test]
fn docx_rejects_group_rows() {
    let path = tmp_dir().join(format!(
        "docx-host-{}.docx",
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "document": { "paragraphs": [{ "text": "x" }] }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let out = tmp_dir().join(format!(
        "docx-out-{}.docx",
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": path.to_string_lossy(),
            "output": out.to_string_lossy(),
            "ops": [{ "op": "groupRows", "id": "sheet:Sheet1", "rows": "2:4" }]
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());
}
