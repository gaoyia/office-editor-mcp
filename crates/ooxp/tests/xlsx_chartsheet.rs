//! 图表工作表：新建、删除，以及仅剩一张图时 deleteChart 仍拒绝。

use std::path::Path;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::json;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-xlsx-chartsheet");
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
            "workbook": { "sheets": [{ "name": "Sheet1", "cells": [{ "ref": "A1", "value": 1 }] }] }
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

fn inspect_charts(path: &Path) -> serde_json::Value {
    let RunOutput::Inspect(tree) = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": path.to_string_lossy(),
            "scope": { "charts": true }
        })
        .to_string(),
        None,
    )
    .unwrap() else {
        panic!();
    };
    tree
}

fn inspect_cells(path: &Path, sheet: &str, range: &str) -> serde_json::Value {
    let RunOutput::Inspect(tree) = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": path.to_string_lossy(),
            "scope": { "sheet": sheet, "range": range }
        })
        .to_string(),
        None,
    )
    .unwrap() else {
        panic!();
    };
    tree
}

fn part_text(path: &Path, name: &str) -> String {
    let file = std::fs::File::open(path).unwrap();
    let mut zip = zip::ZipArchive::new(file).unwrap();
    let mut f = zip.by_name(name).unwrap();
    let mut s = String::new();
    std::io::Read::read_to_string(&mut f, &mut s).unwrap();
    s
}

fn insert_op() -> serde_json::Value {
    json!({
        "op": "insertChartSheet",
        "id": "xlsx:workbook",
        "name": "Chart1",
        "dataFrom": "A10",
        "type": "col",
        "title": "销量",
        "series": [{
            "name": "Q1",
            "categories": ["甲", "乙", "丙"],
            "values": [1, 2, 3]
        }]
    })
}

#[test]
fn insert_inspect_kind_series_and_data() {
    let src = create_book("round");
    let inserted = tmp("ins");
    apply(&src, &inserted, json!([insert_op()])).unwrap();

    let tree = inspect_charts(&inserted);
    assert_eq!(tree["sheets"].as_array().unwrap().len(), 2);
    assert_eq!(tree["sheets"][0]["kind"], "worksheet");
    let cs = &tree["sheets"][1];
    assert_eq!(cs["name"], "Chart1");
    assert_eq!(cs["kind"], "chartsheet");
    assert_eq!(cs["id"], "sheet:Chart1");
    assert_eq!(cs["chartCount"], 1);
    assert_eq!(tree["chartCount"], 1);
    let charts = cs["charts"].as_array().unwrap();
    assert_eq!(charts.len(), 1);
    assert_eq!(charts[0]["id"], "sheet:Chart1/chart:2");
    assert_eq!(charts[0]["cNvPrId"], "2");
    assert_eq!(charts[0]["type"], "col");
    assert_eq!(charts[0]["title"], "销量");
    assert_eq!(charts[0]["anchor"], "absolute");
    assert_eq!(charts[0]["posX"], 0);
    assert_eq!(charts[0]["posY"], 0);
    assert_eq!(charts[0]["stable"], true);
    let ser = &charts[0]["series"][0];
    assert_eq!(ser["id"], "sheet:Chart1/chart:2/series:0");
    assert_eq!(ser["name"], "Q1");
    assert_eq!(ser["categories"], json!(["甲", "乙", "丙"]));
    assert_eq!(ser["values"], json!([1, 2, 3]));
    assert_eq!(ser["catRef"], "Sheet1!$A$11:$A$13");
    assert_eq!(ser["valRef"], "Sheet1!$B$11:$B$13");

    let cells = inspect_cells(&inserted, "Sheet1", "A10:B13");
    let sheet = cells["sheets"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["name"] == "Sheet1")
        .unwrap();
    let list = sheet["cells"].as_array().unwrap();
    assert!(list
        .iter()
        .any(|c| c["id"] == "cell:Sheet1!B10" && c["value"] == "Q1"));
    assert!(list
        .iter()
        .any(|c| c["id"] == "cell:Sheet1!A11" && c["value"] == "甲"));
    assert!(list
        .iter()
        .any(|c| c["id"] == "cell:Sheet1!B11" && c["value"] == 1));

    let cs_xml = part_text(&inserted, "xl/chartsheets/sheet1.xml");
    assert!(cs_xml.contains("<sheetViews>"));
    assert!(cs_xml.contains("drawing"));
    let drawing = part_text(&inserted, "xl/drawings/drawing1.xml");
    assert!(drawing.contains("absoluteAnchor"));
    assert!(drawing.contains(r#"id="2""#));
    let chart_xml = part_text(&inserted, "xl/charts/chart1.xml");
    assert!(
        chart_xml.contains("<c:f>Sheet1!$B$11:$B$13</c:f>") || chart_xml.contains("Sheet1!$B$11")
    );
    assert!(!chart_xml.contains("externalData"));
    let wb = part_text(&inserted, "xl/workbook.xml");
    assert!(wb.contains(r#"name="Chart1""#));
    let rels = part_text(&inserted, "xl/_rels/workbook.xml.rels");
    assert!(rels.contains("/chartsheet"));
}

#[test]
fn delete_removes_sheet_keeps_data() {
    let src = create_book("del-src");
    let inserted = tmp("del-ins");
    apply(&src, &inserted, json!([insert_op()])).unwrap();
    let deleted = tmp("del");
    apply(
        &inserted,
        &deleted,
        json!([{
            "op": "deleteChartSheet",
            "id": "sheet:Chart1"
        }]),
    )
    .unwrap();

    let tree = inspect_charts(&deleted);
    assert_eq!(tree["sheets"].as_array().unwrap().len(), 1);
    assert_eq!(tree["sheets"][0]["name"], "Sheet1");
    assert_eq!(tree["sheets"][0]["kind"], "worksheet");
    assert_eq!(tree["chartCount"], 0);

    let cells = inspect_cells(&deleted, "Sheet1", "A10:B13");
    let sheet = cells["sheets"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["name"] == "Sheet1")
        .unwrap();
    let list = sheet["cells"].as_array().unwrap();
    assert!(list
        .iter()
        .any(|c| c["id"] == "cell:Sheet1!B10" && c["value"] == "Q1"));
    assert!(list
        .iter()
        .any(|c| c["id"] == "cell:Sheet1!B11" && c["value"] == 1));

    let file = std::fs::File::open(&deleted).unwrap();
    let mut zip = zip::ZipArchive::new(file).unwrap();
    assert!(zip.by_name("xl/chartsheets/sheet1.xml").is_err());
}

#[test]
fn delete_on_worksheet_is_invalid_id() {
    let src = create_book("ws");
    let err = apply(
        &src,
        &tmp("bad-ws"),
        json!([{
            "op": "deleteChartSheet",
            "id": "sheet:Sheet1"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
}

#[test]
fn delete_chart_on_only_chartsheet_chart_is_not_implemented() {
    let src = create_book("last");
    let inserted = tmp("last-ins");
    apply(&src, &inserted, json!([insert_op()])).unwrap();
    let err = apply(
        &inserted,
        &tmp("last-del"),
        json!([{
            "op": "deleteChart",
            "id": "sheet:Chart1/chart:2"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::NotImplemented);
}

#[test]
fn insert_chart_on_chartsheet_is_invalid_field() {
    let src = create_book("ins-cs");
    let inserted = tmp("ins-cs-out");
    apply(&src, &inserted, json!([insert_op()])).unwrap();
    let err = apply(
        &inserted,
        &tmp("ins-cs-bad"),
        json!([{
            "op": "insertChart",
            "id": "sheet:Chart1",
            "type": "col",
            "from": "E2",
            "dataFrom": "A20",
            "series": [{ "values": [1] }]
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

#[test]
fn insert_rejects_from_and_wrong_id() {
    let src = create_book("rej");
    let err = apply(
        &src,
        &tmp("from"),
        json!([{
            "op": "insertChartSheet",
            "id": "xlsx:workbook",
            "name": "Chart1",
            "from": "E2",
            "dataFrom": "A10",
            "type": "col",
            "series": [{ "values": [1] }]
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let err = apply(
        &src,
        &tmp("bad-id"),
        json!([{
            "op": "insertChartSheet",
            "id": "sheet:Sheet1",
            "name": "Chart1",
            "dataFrom": "A10",
            "type": "col",
            "series": [{ "values": [1] }]
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
}

#[test]
fn insert_with_data_sheet_name() {
    let path = tmp("two");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "workbook": {
                "sheets": [
                    { "name": "Sheet1", "cells": [{ "ref": "A1", "value": 1 }] },
                    { "name": "Data", "cells": [{ "ref": "A1", "value": 9 }] }
                ]
            }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let out = tmp("two-out");
    apply(
        &path,
        &out,
        json!([{
            "op": "insertChartSheet",
            "id": "xlsx:workbook",
            "name": "Chart1",
            "dataSheet": "Data",
            "dataFrom": "A10",
            "type": "col",
            "series": [{
                "name": "Q1",
                "categories": ["甲", "乙"],
                "values": [4, 5]
            }]
        }]),
    )
    .unwrap();
    let tree = inspect_charts(&out);
    let cs = tree["sheets"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["kind"] == "chartsheet")
        .unwrap();
    assert_eq!(cs["charts"][0]["series"][0]["valRef"], "Data!$B$11:$B$12");
    let cells = inspect_cells(&out, "Data", "A10:B12");
    let sheet = cells["sheets"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["name"] == "Data")
        .unwrap();
    let list = sheet["cells"].as_array().unwrap();
    assert!(list
        .iter()
        .any(|c| c["id"] == "cell:Data!B11" && c["value"] == 4));
}
