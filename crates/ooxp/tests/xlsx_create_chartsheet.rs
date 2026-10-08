//! xlsx create：图表工作表。

use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-xlsx-create-chartsheet");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn tmp(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}.xlsx"))
}

fn create_book(path: &Path, sheets: Value) {
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "workbook": { "sheets": sheets }
        })
        .to_string(),
        None,
    )
    .unwrap();
}

fn create_err(sheets: Value) -> ooxp::Error {
    let path = tmp("err");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "workbook": { "sheets": sheets }
        })
        .to_string(),
        None,
    )
    .unwrap_err()
}

fn inspect(path: &Path, scope: Value) -> Value {
    let RunOutput::Inspect(tree) = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": path.to_string_lossy(),
            "scope": scope
        })
        .to_string(),
        None,
    )
    .unwrap() else {
        panic!("inspect 应返回树");
    };
    tree
}

fn part_text(path: &Path, name: &str) -> String {
    let f = std::fs::File::open(path).unwrap();
    let mut zip = zip::ZipArchive::new(f).unwrap();
    let mut zf = zip.by_name(name).unwrap();
    let mut out = String::new();
    zf.read_to_string(&mut out).unwrap();
    out
}

fn chartsheet_item() -> Value {
    json!({
        "name": "Chart1",
        "kind": "chartsheet",
        "type": "col",
        "title": "销量",
        "dataFrom": "A10",
        "dataSheet": "Sheet1",
        "series": [{
            "name": "Q1",
            "categories": ["甲", "乙", "丙"],
            "values": [1, 2, 3]
        }]
    })
}

#[test]
fn create_writes_chartsheet_and_data() {
    let path = tmp("full");
    create_book(
        &path,
        json!([
            { "name": "Sheet1" },
            chartsheet_item()
        ]),
    );

    let tree = inspect(
        &path,
        json!({ "charts": true, "sheet": "Sheet1", "range": "A10:B13" }),
    );
    let sheets = tree["sheets"].as_array().unwrap();
    assert_eq!(sheets.len(), 2);
    assert_eq!(sheets[0]["name"], "Sheet1");
    assert_eq!(sheets[0]["kind"], "worksheet");
    let cs = &sheets[1];
    assert_eq!(cs["name"], "Chart1");
    assert_eq!(cs["kind"], "chartsheet");
    assert_eq!(cs["id"], "sheet:Chart1");
    assert_eq!(cs["chartCount"], 1);
    assert_eq!(tree["chartCount"], 1);
    let charts = cs["charts"].as_array().unwrap();
    assert_eq!(charts.len(), 1);
    assert_eq!(charts[0]["id"], "sheet:Chart1/chart:2");
    assert_eq!(charts[0]["type"], "col");
    assert_eq!(charts[0]["title"], "销量");
    assert_eq!(charts[0]["anchor"], "absolute");
    let ser = &charts[0]["series"][0];
    assert_eq!(ser["values"], json!([1, 2, 3]));
    assert_eq!(ser["catRef"], "Sheet1!$A$11:$A$13");
    assert_eq!(ser["valRef"], "Sheet1!$B$11:$B$13");

    let sheet = sheets.iter().find(|s| s["name"] == "Sheet1").unwrap();
    let cells = sheet["cells"].as_array().unwrap();
    assert!(cells
        .iter()
        .any(|c| c["id"] == "cell:Sheet1!B11" && c["value"] == 1));

    let cs_xml = part_text(&path, "xl/chartsheets/sheet1.xml");
    assert!(cs_xml.contains("<sheetViews>"));
    assert!(cs_xml.contains("drawing"));
    let drawing = part_text(&path, "xl/drawings/drawing1.xml");
    assert!(drawing.contains("absoluteAnchor"));
    let rels = part_text(&path, "xl/_rels/workbook.xml.rels");
    assert!(rels.contains("/chartsheet"));
    let app = part_text(&path, "docProps/app.xml");
    assert!(app.contains("Charts"));
    let wb = part_text(&path, "xl/workbook.xml");
    let sheet1 = wb.find(r#"name="Sheet1""#).unwrap();
    let chart1 = wb.find(r#"name="Chart1""#).unwrap();
    assert!(sheet1 < chart1);
}

#[test]
fn create_preserves_chartsheet_first() {
    let path = tmp("order");
    create_book(
        &path,
        json!([
            chartsheet_item(),
            { "name": "Sheet1" }
        ]),
    );
    let tree = inspect(&path, json!({ "charts": true }));
    let sheets = tree["sheets"].as_array().unwrap();
    assert_eq!(sheets[0]["name"], "Chart1");
    assert_eq!(sheets[0]["kind"], "chartsheet");
    assert_eq!(sheets[1]["name"], "Sheet1");
    assert_eq!(sheets[1]["kind"], "worksheet");
}

#[test]
fn create_rejects_chartsheet_without_worksheet() {
    let err = create_err(json!([chartsheet_item()]));
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert_eq!(err.field_path(), Some("workbook.sheets"));
}

#[test]
fn create_rejects_chartsheet_cells_and_missing_type() {
    let err = create_err(json!([
        { "name": "Sheet1" },
        {
            "name": "Chart1",
            "kind": "chartsheet",
            "type": "col",
            "dataFrom": "A10",
            "series": [{ "values": [1, 2, 3] }],
            "cells": [{ "ref": "A1", "value": 1 }]
        }
    ]));
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert_eq!(err.field_path(), Some("workbook.sheets[1].cells"));

    let err = create_err(json!([
        { "name": "Sheet1" },
        { "name": "Chart1", "kind": "chartsheet", "dataFrom": "A10", "series": [{ "values": [1] }] }
    ]));
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert_eq!(err.field_path(), Some("workbook.sheets[1].type"));
}

#[test]
fn create_rejects_dialogsheet_and_bad_data_sheet() {
    let err = create_err(json!([{ "name": "Sheet1", "kind": "dialogsheet" }]));
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert_eq!(err.field_path(), Some("workbook.sheets[0].kind"));

    let err = create_err(json!([
        { "name": "Sheet1" },
        {
            "name": "Chart1",
            "kind": "chartsheet",
            "type": "col",
            "dataFrom": "A10",
            "dataSheet": "NoSuch",
            "series": [{ "values": [1, 2, 3] }]
        }
    ]));
    assert_eq!(err.code(), ErrorCode::IdNotFound);
    assert_eq!(err.field_path(), Some("workbook.sheets[1].dataSheet"));

    let err = create_err(json!([
        { "name": "Sheet1" },
        {
            "name": "ChartA",
            "kind": "chartsheet",
            "type": "col",
            "dataFrom": "A10",
            "series": [{ "values": [1, 2, 3] }]
        },
        {
            "name": "ChartB",
            "kind": "chartsheet",
            "type": "col",
            "dataFrom": "A10",
            "dataSheet": "ChartA",
            "series": [{ "values": [1, 2, 3] }]
        }
    ]));
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert_eq!(err.field_path(), Some("workbook.sheets[2].dataSheet"));
}
