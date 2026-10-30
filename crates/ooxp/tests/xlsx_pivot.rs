//! 工作表透视表：inspect 名单、insert / set / delete。

use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::json;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-xlsx-pivot");
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
                    { "ref": "A1", "value": "产品" },
                    { "ref": "B1", "value": "数量" },
                    { "ref": "C1", "value": "金额" },
                    { "ref": "A2", "value": "甲" },
                    { "ref": "B2", "value": 1 },
                    { "ref": "C2", "value": 10 },
                    { "ref": "A3", "value": "乙" },
                    { "ref": "B3", "value": 2 },
                    { "ref": "C3", "value": 20 },
                    { "ref": "A4", "value": "甲" },
                    { "ref": "B4", "value": 3 },
                    { "ref": "C4", "value": 30 }
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
        "scope": { "sheet": "Sheet1", "range": "A1:C5" }
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

fn part_exists(path: &Path, name: &str) -> bool {
    let f = std::fs::File::open(path).unwrap();
    let mut zip = zip::ZipArchive::new(f).unwrap();
    let ok = zip.by_name(name).is_ok();
    ok
}

fn object_keys(v: &serde_json::Value) -> Vec<&str> {
    v.as_object().unwrap().keys().map(|k| k.as_str()).collect()
}

#[test]
fn inspect_empty_on_create() {
    let path = create_book("empty");
    let no = inspect_noscope(&path);
    assert_eq!(no["pivotCount"], json!(0));
    assert!(no.get("pivots").is_none());
    let sh = inspect_sheet(&path);
    assert_eq!(sh["pivotCount"], json!(0));
    assert_eq!(sh["pivots"], json!([]));
}

#[test]
fn insert_named_pivot_and_inspect() {
    let src = create_book("ins");
    let out = tmp("ins-out");
    apply(
        &src,
        &out,
        json!([{
            "op": "insertPivotTable",
            "id": "sheet:Sheet1",
            "source": { "sheet": "Sheet1", "ref": "A1:C4" },
            "location": "E3",
            "name": "SalesPivot",
            "rows": ["产品"],
            "values": ["金额"]
        }]),
    )
    .unwrap();
    let sh = inspect_sheet(&out);
    assert_eq!(sh["pivotCount"], json!(1));
    let p = &sh["pivots"][0];
    assert_eq!(p["id"], json!("sheet:Sheet1/pivot:1"));
    assert_eq!(p["name"], json!("SalesPivot"));
    assert_eq!(p["cacheId"], json!(1));
    assert_eq!(p["source"]["sheet"], json!("Sheet1"));
    assert_eq!(p["source"]["ref"], json!("A1:C4"));
    assert_eq!(p["rows"], json!(["产品"]));
    assert_eq!(p["values"], json!(["金额"]));
    assert_eq!(
        object_keys(p),
        ["id", "name", "cacheId", "location", "source", "rows", "cols", "filters", "values"]
    );
    let wb = part_text(&out, "xl/workbook.xml");
    assert!(wb.contains("pivotCaches"), "{wb}");
    assert!(wb.contains(r#"cacheId="1""#), "{wb}");
    let def = part_text(&out, "xl/pivotCache/pivotCacheDefinition1.xml");
    assert!(def.contains(r#"refreshOnLoad="1""#), "{def}");
    assert!(def.contains(r#"sheet="Sheet1""#), "{def}");
    assert!(def.contains(r#"ref="A1:C4""#), "{def}");
    assert!(def.contains(r#"<s v="甲"/>"#), "{def}");
    assert!(
        !def.contains(r#"containsString="1""#),
        "字符串 sharedItems 不要写 containsString，Excel 16 会打不开：{def}"
    );
    let rec = part_text(&out, "xl/pivotCache/pivotCacheRecords1.xml");
    assert!(rec.contains(r#"<x v="0"/>"#), "{rec}");
    let table = part_text(&out, "xl/pivotTables/pivotTable1.xml");
    assert!(table.contains(r#"name="SalesPivot""#), "{table}");
    assert!(table.contains(r#"cacheId="1""#), "{table}");
    assert!(table.contains(r#"<item x="0"/>"#), "{table}");
    assert!(table.contains(r#"t="grand""#), "{table}");
    assert!(table.contains("PivotStyleLight16"), "{table}");
    let cts = part_text(&out, "[Content_Types].xml");
    assert!(cts.contains("pivotTable+xml"), "{cts}");
    assert!(cts.contains("pivotCacheDefinition+xml"), "{cts}");
    assert!(cts.contains("pivotCacheRecords+xml"), "{cts}");
    let rels = part_text(&out, "xl/worksheets/_rels/sheet1.xml.rels");
    assert!(rels.contains("/pivotTable"), "{rels}");
    let trels = part_text(&out, "xl/pivotTables/_rels/pivotTable1.xml.rels");
    assert!(trels.contains("/pivotCacheDefinition"), "{trels}");
    assert!(trels.contains("pivotCacheDefinition1.xml"), "{trels}");
}

#[test]
fn set_source_then_delete_leaves_cells() {
    let src = create_book("setdel");
    let mid = tmp("setdel-mid");
    apply(
        &src,
        &mid,
        json!([{
            "op": "insertPivotTable",
            "id": "sheet:Sheet1",
            "source": { "ref": "A1:C4" },
            "location": "E3",
            "rows": ["产品"],
            "values": ["金额"]
        }]),
    )
    .unwrap();
    let expanded = tmp("setdel-exp");
    apply(
        &mid,
        &expanded,
        json!([{
            "op": "setPivotTable",
            "id": "sheet:Sheet1/pivot:1",
            "source": { "sheet": "Sheet1", "ref": "A1:C5" }
        }]),
    )
    .unwrap();
    let sh = inspect_sheet(&expanded);
    assert_eq!(sh["pivots"][0]["source"]["ref"], json!("A1:C5"));
    let def = part_text(&expanded, "xl/pivotCache/pivotCacheDefinition1.xml");
    assert!(def.contains(r#"ref="A1:C5""#), "{def}");
    let gone = tmp("setdel-gone");
    apply(
        &expanded,
        &gone,
        json!([{ "op": "deletePivotTable", "id": "sheet:Sheet1/pivot:1" }]),
    )
    .unwrap();
    let sh2 = inspect_sheet(&gone);
    assert_eq!(sh2["pivotCount"], json!(0));
    assert_eq!(sh2["pivots"], json!([]));
    let wb = part_text(&gone, "xl/workbook.xml");
    assert!(!wb.contains("pivotCaches"), "{wb}");
    assert!(!part_exists(&gone, "xl/pivotTables/pivotTable1.xml"));
    assert!(!part_exists(
        &gone,
        "xl/pivotCache/pivotCacheDefinition1.xml"
    ));
    let a1 = sh2["cells"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["ref"] == "A1")
        .unwrap();
    assert_eq!(a1["value"], json!("产品"));
}

#[test]
fn missing_fields_are_invalid() {
    let src = create_book("miss");
    let out = tmp("miss-out");
    let err = apply(
        &src,
        &out,
        json!([{ "op": "insertPivotTable", "id": "sheet:Sheet1" }]),
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
            "op": "insertPivotTable",
            "id": "sheet:Sheet1",
            "source": { "ref": "A1:C4" },
            "location": "E3",
            "values": ["金额"]
        }]),
    )
    .unwrap();
    let out = tmp("setmiss-out");
    let err = apply(
        &mid,
        &out,
        json!([{ "op": "setPivotTable", "id": "sheet:Sheet1/pivot:1" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());
}

#[test]
fn overlap_and_unknown_field_are_invalid() {
    let src = create_book("bad");
    let out = tmp("bad-overlap");
    let err = apply(
        &src,
        &out,
        json!([{
            "op": "insertPivotTable",
            "id": "sheet:Sheet1",
            "source": { "ref": "A1:C4" },
            "location": "B2",
            "values": ["金额"]
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());
    let out2 = tmp("bad-field");
    let err2 = apply(
        &src,
        &out2,
        json!([{
            "op": "insertPivotTable",
            "id": "sheet:Sheet1",
            "source": { "ref": "A1:C4" },
            "location": "E3",
            "rows": ["不存在"],
            "values": ["金额"]
        }]),
    )
    .unwrap_err();
    assert_eq!(err2.code(), ErrorCode::InvalidField);
    assert!(!out2.exists());
}
