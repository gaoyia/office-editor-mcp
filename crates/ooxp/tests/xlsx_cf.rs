//! 条件格式：inspect 名单、insert / set / delete、dxf 复用。

use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::json;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-xlsx-cf");
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
                    { "ref": "A2", "value": -1 },
                    { "ref": "A3", "value": 5 }
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

fn inspect_sheet(path: &Path) -> serde_json::Value {
    let RunOutput::Inspect(tree) = run(&json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": path.to_string_lossy(),
        "scope": { "sheet": "Sheet1", "range": "A1:C3" }
    })
    .to_string())
    .unwrap() else {
        panic!("inspect 应返回树");
    };
    tree["sheets"][0].clone()
}

fn inspect_noscope(path: &Path) -> serde_json::Value {
    let RunOutput::Inspect(tree) = run(&json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": path.to_string_lossy(),
    })
    .to_string())
    .unwrap() else {
        panic!("inspect 应返回树");
    };
    tree["sheets"][0].clone()
}

fn part_text(path: &Path, name: &str) -> String {
    let f = std::fs::File::open(path).unwrap();
    let mut zip = zip::ZipArchive::new(f).unwrap();
    let mut zf = zip.by_name(name).unwrap();
    let mut s = String::new();
    zf.read_to_string(&mut s).unwrap();
    s
}

fn object_keys(v: &serde_json::Value) -> Vec<&str> {
    v.as_object().unwrap().keys().map(|k| k.as_str()).collect()
}

#[test]
fn inspect_empty_on_create() {
    let path = create_book("empty");
    let no = inspect_noscope(&path);
    assert_eq!(no["conditionalFormatCount"], json!(0));
    assert!(no.get("conditionalFormats").is_none());
    let sh = inspect_sheet(&path);
    assert_eq!(sh["conditionalFormatCount"], json!(0));
    assert_eq!(sh["conditionalFormats"], json!([]));
}

#[test]
fn insert_cell_is_green_and_inspect() {
    let src = create_book("cellis");
    let out = tmp("cellis-out");
    apply(
        &src,
        &out,
        json!([{
            "op": "insertConditionalFormat",
            "id": "sheet:Sheet1",
            "sqref": "A1:A10",
            "type": "cellIs",
            "operator": "greaterThan",
            "formulas": ["0"],
            "dxf": { "fill": { "pattern": "solid", "fg": { "rgb": "FFC6EFCE" } } }
        }]),
    )
    .unwrap();
    let sh = inspect_sheet(&out);
    assert_eq!(sh["conditionalFormatCount"], json!(1));
    let rule = &sh["conditionalFormats"][0];
    assert_eq!(rule["id"], json!("sheet:Sheet1/cf:1"));
    assert_eq!(rule["sqref"], json!(["A1:A10"]));
    assert_eq!(rule["type"], json!("cellIs"));
    assert_eq!(rule["operator"], json!("greaterThan"));
    assert_eq!(rule["formulas"], json!(["0"]));
    assert_eq!(rule["priority"], json!(1));
    assert_eq!(rule["dxf"]["fill"]["pattern"], json!("solid"));
    assert_eq!(object_keys(rule)[0..4], ["id", "sqref", "type", "priority"]);
    let xml = part_text(&out, "xl/worksheets/sheet1.xml");
    assert!(xml.contains("conditionalFormatting"), "{xml}");
    assert!(xml.contains(r#"type="cellIs""#), "{xml}");
    assert!(xml.contains("greaterThan"), "{xml}");
    let styles = part_text(&out, "xl/styles.xml");
    assert!(styles.contains("<dxfs"), "{styles}");
    assert!(
        styles.contains("FFC6EFCE") || styles.contains("C6EFCE"),
        "{styles}"
    );
}

#[test]
fn set_sqref_and_delete() {
    let src = create_book("setdel");
    let mid = tmp("setdel-mid");
    apply(
        &src,
        &mid,
        json!([{
            "op": "insertConditionalFormat",
            "id": "sheet:Sheet1",
            "sqref": "A1:A10",
            "type": "cellIs",
            "operator": "greaterThan",
            "formulas": ["0"],
            "dxf": { "fill": { "fg": { "rgb": "FF00B050" } } }
        }]),
    )
    .unwrap();
    let expanded = tmp("setdel-exp");
    apply(
        &mid,
        &expanded,
        json!([{
            "op": "setConditionalFormat",
            "id": "sheet:Sheet1/cf:1",
            "sqref": "A1:B10"
        }]),
    )
    .unwrap();
    let sh = inspect_sheet(&expanded);
    assert_eq!(sh["conditionalFormats"][0]["sqref"], json!(["A1:B10"]));
    let gone = tmp("setdel-gone");
    apply(
        &expanded,
        &gone,
        json!([{ "op": "deleteConditionalFormat", "id": "sheet:Sheet1/cf:1" }]),
    )
    .unwrap();
    let sh2 = inspect_sheet(&gone);
    assert_eq!(sh2["conditionalFormatCount"], json!(0));
    assert_eq!(sh2["conditionalFormats"], json!([]));
    let xml = part_text(&gone, "xl/worksheets/sheet1.xml");
    assert!(!xml.contains("conditionalFormatting"), "{xml}");
}

#[test]
fn insert_color_scale() {
    let src = create_book("cs");
    let out = tmp("cs-out");
    apply(
        &src,
        &out,
        json!([{
            "op": "insertConditionalFormat",
            "id": "sheet:Sheet1",
            "sqref": "A1:A10",
            "type": "colorScale",
            "colorScale": {
                "colors": [
                    { "rgb": "FFFF0000" },
                    { "rgb": "FF00B050" }
                ]
            }
        }]),
    )
    .unwrap();
    let sh = inspect_sheet(&out);
    let rule = &sh["conditionalFormats"][0];
    assert_eq!(rule["type"], json!("colorScale"));
    assert_eq!(rule["colorScale"]["colors"].as_array().unwrap().len(), 2);
    assert_eq!(rule["colorScale"]["cfvo"][0]["type"], json!("min"));
    assert_eq!(rule["colorScale"]["cfvo"][1]["type"], json!("max"));
    let xml = part_text(&out, "xl/worksheets/sheet1.xml");
    assert!(xml.contains("colorScale"), "{xml}");
}

#[test]
fn insert_data_bar_and_icon_set() {
    let src = create_book("db");
    let out = tmp("db-out");
    apply(
        &src,
        &out,
        json!([
            {
                "op": "insertConditionalFormat",
                "id": "sheet:Sheet1",
                "sqref": "A1:A3",
                "type": "dataBar"
            },
            {
                "op": "insertConditionalFormat",
                "id": "sheet:Sheet1",
                "sqref": "A1:A3",
                "type": "iconSet",
                "iconSet": { "set": "3Arrows" }
            }
        ]),
    )
    .unwrap();
    let sh = inspect_sheet(&out);
    assert_eq!(sh["conditionalFormatCount"], json!(2));
    assert_eq!(sh["conditionalFormats"][0]["type"], json!("dataBar"));
    assert_eq!(sh["conditionalFormats"][0]["priority"], json!(1));
    assert_eq!(sh["conditionalFormats"][1]["type"], json!("iconSet"));
    assert_eq!(
        sh["conditionalFormats"][1]["iconSet"]["set"],
        json!("3Arrows")
    );
    assert_eq!(sh["conditionalFormats"][1]["priority"], json!(2));
    let xml = part_text(&out, "xl/worksheets/sheet1.xml");
    assert!(xml.contains("<dataBar"), "{xml}");
    assert!(xml.contains(r#"iconSet="3Arrows""#), "{xml}");
}

#[test]
fn missing_fields_are_invalid() {
    let src = create_book("miss");
    let out = tmp("miss-out");
    let err = apply(
        &src,
        &out,
        json!([{ "op": "insertConditionalFormat", "id": "sheet:Sheet1" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());
}

#[test]
fn set_without_fields_is_invalid() {
    let src = create_book("setmiss");
    let mid = tmp("setmiss-mid");
    apply(
        &src,
        &mid,
        json!([{
            "op": "insertConditionalFormat",
            "id": "sheet:Sheet1",
            "sqref": "A1",
            "type": "cellIs",
            "operator": "greaterThan",
            "formulas": ["0"]
        }]),
    )
    .unwrap();
    let out = tmp("setmiss-out");
    let err = apply(
        &mid,
        &out,
        json!([{ "op": "setConditionalFormat", "id": "sheet:Sheet1/cf:1" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());
}

#[test]
fn wrong_id_is_invalid() {
    let src = create_book("id");
    let out = tmp("id-out");
    let err = apply(
        &src,
        &out,
        json!([{
            "op": "deleteConditionalFormat",
            "id": "sheet:Sheet1"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
}

#[test]
fn formula_strips_equals() {
    let src = create_book("eq");
    let out = tmp("eq-out");
    apply(
        &src,
        &out,
        json!([{
            "op": "insertConditionalFormat",
            "id": "sheet:Sheet1",
            "sqref": "A1",
            "type": "expression",
            "formulas": ["=A1>0"],
            "dxf": { "font": { "bold": true } }
        }]),
    )
    .unwrap();
    let sh = inspect_sheet(&out);
    assert_eq!(sh["conditionalFormats"][0]["formulas"], json!(["A1>0"]));
    assert_eq!(
        sh["conditionalFormats"][0]["dxf"]["font"]["bold"],
        json!(true)
    );
}

#[test]
fn insert_top10_above_average_and_text() {
    let src = create_book("xfs");
    let out = tmp("xfs-out");
    apply(
        &src,
        &out,
        json!([
            {
                "op": "insertConditionalFormat",
                "id": "sheet:Sheet1",
                "sqref": "A1:A10",
                "type": "top10",
                "rank": 3,
                "dxf": { "font": { "bold": true } }
            },
            {
                "op": "insertConditionalFormat",
                "id": "sheet:Sheet1",
                "sqref": "A1:A10",
                "type": "aboveAverage",
                "aboveAverage": false
            },
            {
                "op": "insertConditionalFormat",
                "id": "sheet:Sheet1",
                "sqref": "A1:A10",
                "type": "containsText",
                "text": "foo"
            }
        ]),
    )
    .unwrap();
    let sh = inspect_sheet(&out);
    let top = &sh["conditionalFormats"][0];
    assert_eq!(top["type"], json!("top10"));
    assert_eq!(top["rank"], json!(3));
    assert!(top.get("percent").is_none());
    let avg = &sh["conditionalFormats"][1];
    assert_eq!(avg["type"], json!("aboveAverage"));
    assert_eq!(avg["aboveAverage"], json!(false));
    let text = &sh["conditionalFormats"][2];
    assert_eq!(text["type"], json!("containsText"));
    assert_eq!(text["text"], json!("foo"));
    assert_eq!(text["operator"], json!("containsText"));
    assert_eq!(
        text["formulas"],
        json!(["NOT(ISERROR(SEARCH(\"foo\",A1)))"])
    );
    let xml = part_text(&out, "xl/worksheets/sheet1.xml");
    assert!(xml.contains(r#"type="top10""#), "{xml}");
    assert!(xml.contains(r#"rank="3""#), "{xml}");
    assert!(xml.contains(r#"aboveAverage="0""#), "{xml}");
    assert!(xml.contains(r#"text="foo""#), "{xml}");
}

#[test]
fn text_without_text_and_top10_operator_rejected() {
    let src = create_book("xfs-bad");
    let out = tmp("xfs-bad-out");
    let err = apply(
        &src,
        &out,
        json!([{
            "op": "insertConditionalFormat",
            "id": "sheet:Sheet1",
            "sqref": "A1:A10",
            "type": "containsText"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());
    let err2 = apply(
        &src,
        &out,
        json!([{
            "op": "insertConditionalFormat",
            "id": "sheet:Sheet1",
            "sqref": "A1:A10",
            "type": "top10",
            "operator": "greaterThan"
        }]),
    )
    .unwrap_err();
    assert_eq!(err2.code(), ErrorCode::InvalidField);
    assert!(!out.exists());
}
