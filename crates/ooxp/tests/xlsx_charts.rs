//! 工作表图表：名单、插删、改系列与类型。

use std::path::Path;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::json;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-xlsx-charts");
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

fn inspect_cells(path: &Path, range: &str) -> serde_json::Value {
    let RunOutput::Inspect(tree) = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": path.to_string_lossy(),
            "scope": { "sheet": "Sheet1", "range": range }
        })
        .to_string(),
        None,
    )
    .unwrap() else {
        panic!();
    };
    tree
}

fn object_keys(v: &serde_json::Value) -> Vec<&str> {
    v.as_object().unwrap().keys().map(String::as_str).collect()
}

fn part_text(path: &Path, name: &str) -> String {
    let file = std::fs::File::open(path).unwrap();
    let mut zip = zip::ZipArchive::new(file).unwrap();
    let mut part = zip.by_name(name).unwrap();
    let mut s = String::new();
    std::io::Read::read_to_string(&mut part, &mut s).unwrap();
    s
}

#[test]
fn inspect_chart_count_without_list() {
    let path = create_book("count");
    let RunOutput::Inspect(tree) = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": path.to_string_lossy()
        })
        .to_string(),
        None,
    )
    .unwrap() else {
        panic!();
    };
    assert_eq!(tree["chartCount"], 0);
    assert_eq!(tree["sheets"][0]["chartCount"], 0);
    assert!(tree["sheets"][0].get("charts").is_none());
}

#[test]
fn insert_inspect_series_type_delete() {
    let src = create_book("round");
    let inserted = tmp("ins");
    apply(
        &src,
        &inserted,
        json!([{
            "op": "insertChart",
            "id": "sheet:Sheet1",
            "type": "col",
            "from": "E2",
            "dataFrom": "A10",
            "title": "销量",
            "series": [{
                "name": "Q1",
                "categories": ["甲", "乙", "丙"],
                "values": [1, 2, 3]
            }]
        }]),
    )
    .unwrap();

    let tree = inspect_charts(&inserted);
    assert_eq!(tree["chartCount"], 1);
    assert_eq!(tree["sheets"][0]["chartCount"], 1);
    let charts = tree["sheets"][0]["charts"].as_array().unwrap();
    assert_eq!(charts.len(), 1);
    assert_eq!(charts[0]["id"], "sheet:Sheet1/chart:1");
    assert_eq!(charts[0]["type"], "col");
    assert_eq!(charts[0]["title"], "销量");
    assert_eq!(charts[0]["anchor"], "oneCell");
    assert_eq!(charts[0]["from"], "E2");
    assert_eq!(charts[0]["stable"], true);
    assert_eq!(
        object_keys(&charts[0]),
        [
            "id",
            "cNvPrId",
            "name",
            "anchor",
            "from",
            "fromOffX",
            "fromOffY",
            "cx",
            "cy",
            "type",
            "title",
            "catAxisTitle",
            "valAxisTitle",
            "series",
            "legend",
            "dataLabels",
            "flags",
            "stable"
        ]
    );
    let ser = &charts[0]["series"][0];
    assert_eq!(ser["id"], "sheet:Sheet1/chart:1/series:0");
    assert_eq!(ser["index"], 0);
    assert_eq!(ser["name"], "Q1");
    assert_eq!(ser["categories"], json!(["甲", "乙", "丙"]));
    assert_eq!(ser["values"], json!([1, 2, 3]));
    assert_eq!(ser["catRef"], "Sheet1!$A$11:$A$13");
    assert_eq!(ser["valRef"], "Sheet1!$B$11:$B$13");
    assert_eq!(ser["axis"], "primary");

    let cells = inspect_cells(&inserted, "A10:B13");
    let list = cells["sheets"][0]["cells"].as_array().unwrap();
    assert!(list
        .iter()
        .any(|c| c["id"] == "cell:Sheet1!B10" && c["value"] == "Q1"));
    assert!(list
        .iter()
        .any(|c| c["id"] == "cell:Sheet1!A11" && c["value"] == "甲"));
    assert!(list
        .iter()
        .any(|c| c["id"] == "cell:Sheet1!B11" && c["value"] == 1));

    let chart_xml = part_text(&inserted, "xl/charts/chart1.xml");
    assert!(
        chart_xml.contains("<c:f>Sheet1!$B$11:$B$13</c:f>") || chart_xml.contains("Sheet1!$B$11")
    );
    assert!(!chart_xml.contains("externalData"));

    let updated = tmp("ser");
    apply(
        &inserted,
        &updated,
        json!([{
            "op": "setChartSeries",
            "id": "sheet:Sheet1/chart:1/series:0",
            "values": [10, 20, 30]
        }]),
    )
    .unwrap();
    let after = inspect_charts(&updated);
    assert_eq!(
        after["sheets"][0]["charts"][0]["series"][0]["values"],
        json!([10, 20, 30])
    );
    let cells2 = inspect_cells(&updated, "B11:B13");
    let list2 = cells2["sheets"][0]["cells"].as_array().unwrap();
    assert!(list2
        .iter()
        .any(|c| c["id"] == "cell:Sheet1!B11" && c["value"] == 10));

    let lined = tmp("line");
    apply(
        &updated,
        &lined,
        json!([{
            "op": "setChartType",
            "id": "sheet:Sheet1/chart:1",
            "type": "line"
        }]),
    )
    .unwrap();
    let after_type = inspect_charts(&lined);
    assert_eq!(after_type["sheets"][0]["charts"][0]["type"], "line");

    let titled = tmp("title");
    apply(
        &lined,
        &titled,
        json!([{
            "op": "setChartTitle",
            "id": "sheet:Sheet1/chart:1",
            "title": "新标题"
        }]),
    )
    .unwrap();
    assert_eq!(
        inspect_charts(&titled)["sheets"][0]["charts"][0]["title"],
        "新标题"
    );

    let sized = tmp("size");
    apply(
        &titled,
        &sized,
        json!([{
            "op": "setChartSize",
            "id": "sheet:Sheet1/chart:1",
            "cx": 1828800,
            "cy": 914400
        }]),
    )
    .unwrap();
    let after_size = inspect_charts(&sized);
    assert_eq!(after_size["sheets"][0]["charts"][0]["cx"], 1828800);

    let deleted = tmp("del");
    apply(
        &sized,
        &deleted,
        json!([{ "op": "deleteChart", "id": "sheet:Sheet1/chart:1" }]),
    )
    .unwrap();
    let after_del = inspect_charts(&deleted);
    assert_eq!(after_del["chartCount"], 0);
    assert_eq!(after_del["sheets"][0]["charts"], json!([]));
    let cells3 = inspect_cells(&deleted, "A10:B13");
    let list3 = cells3["sheets"][0]["cells"].as_array().unwrap();
    assert!(list3
        .iter()
        .any(|c| c["id"] == "cell:Sheet1!B10" && c["value"] == "Q1"));
}

#[test]
fn rejects_wrap_and_export_chart_data() {
    let src = create_book("rej");
    let out = tmp("rej-out");
    apply(
        &src,
        &out,
        json!([{
            "op": "insertChart",
            "id": "sheet:Sheet1",
            "type": "col",
            "from": "E2",
            "dataFrom": "A10",
            "series": [{ "values": [1, 2], "categories": ["a", "b"] }]
        }]),
    )
    .unwrap();
    let err = apply(
        &out,
        &tmp("wrap"),
        json!([{
            "op": "setChartWrap",
            "id": "sheet:Sheet1/chart:1",
            "kind": "anchor",
            "wrap": "square"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "exportChartData",
            "path": out.to_string_lossy(),
            "id": "sheet:Sheet1/chart:1",
            "output": tmp("exp").with_extension("xlsx").to_string_lossy()
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

#[test]
fn two_cell_size_rejected() {
    let src = create_book("two");
    let out = tmp("two-out");
    apply(
        &src,
        &out,
        json!([{
            "op": "insertChart",
            "id": "sheet:Sheet1",
            "type": "col",
            "anchor": "twoCell",
            "from": "A1",
            "to": "C8",
            "dataFrom": "A10",
            "series": [{ "values": [1, 2], "categories": ["a", "b"] }]
        }]),
    )
    .unwrap();
    let tree = inspect_charts(&out);
    assert_eq!(tree["sheets"][0]["charts"][0]["anchor"], "twoCell");
    let err = apply(
        &out,
        &tmp("size-fail"),
        json!([{
            "op": "setChartSize",
            "id": "sheet:Sheet1/chart:1",
            "cx": 1000,
            "cy": 1000
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::NotImplemented);
}

#[test]
fn insert_requires_sheet_and_data_from() {
    let src = create_book("need");
    let err = apply(
        &src,
        &tmp("bad-id"),
        json!([{
            "op": "insertChart",
            "id": "sheet:Sheet1/chart:1",
            "type": "col",
            "from": "E2",
            "dataFrom": "A10",
            "series": [{ "values": [1] }]
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);

    let err = apply(
        &src,
        &tmp("no-from"),
        json!([{
            "op": "insertChart",
            "id": "sheet:Sheet1",
            "type": "col",
            "dataFrom": "A10",
            "series": [{ "values": [1] }]
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

#[test]
fn create_writes_worksheet_chart() {
    let path = tmp("create-chart");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "workbook": {
                "sheets": [{
                    "name": "Sheet1",
                    "charts": [{
                        "type": "col",
                        "title": "销量",
                        "from": "E2",
                        "dataFrom": "A10",
                        "series": [{
                            "name": "Q1",
                            "categories": ["甲", "乙", "丙"],
                            "values": [1, 2, 3]
                        }]
                    }]
                }]
            }
        })
        .to_string(),
        None,
    )
    .unwrap();

    let tree = inspect_charts(&path);
    assert_eq!(tree["chartCount"], 1);
    assert_eq!(tree["sheets"][0]["chartCount"], 1);
    let c = &tree["sheets"][0]["charts"][0];
    assert_eq!(c["id"], "sheet:Sheet1/chart:1");
    assert_eq!(c["type"], "col");
    assert_eq!(c["from"], "E2");
    assert_eq!(c["title"], "销量");
    assert_eq!(c["anchor"], "oneCell");
    let ser = &c["series"][0];
    assert_eq!(ser["values"], json!([1, 2, 3]));
    assert_eq!(ser["categories"], json!(["甲", "乙", "丙"]));
    assert_eq!(ser["catRef"], "Sheet1!$A$11:$A$13");
    assert_eq!(ser["valRef"], "Sheet1!$B$11:$B$13");

    let cells = inspect_cells(&path, "A10:B13");
    let list = cells["sheets"][0]["cells"].as_array().unwrap();
    assert!(list
        .iter()
        .any(|c| c["id"] == "cell:Sheet1!B10" && c["value"] == "Q1"));
    assert!(list
        .iter()
        .any(|c| c["id"] == "cell:Sheet1!A11" && c["value"] == "甲"));
    assert!(list
        .iter()
        .any(|c| c["id"] == "cell:Sheet1!B11" && c["value"] == 1));

    let chart_xml = part_text(&path, "xl/charts/chart1.xml");
    assert!(chart_xml.contains("Sheet1!$B$11"));
    assert!(!chart_xml.contains("externalData"));
    let drawing = part_text(&path, "xl/drawings/drawing1.xml");
    assert!(drawing.contains("xdr:clientData"));
}

#[test]
fn create_rejects_chart_wrap() {
    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": tmp("create-wrap").to_string_lossy(),
            "workbook": {
                "sheets": [{
                    "name": "Sheet1",
                    "charts": [{
                        "type": "col",
                        "from": "E2",
                        "dataFrom": "A10",
                        "wrap": "square",
                        "series": [{ "values": [1, 2, 3] }]
                    }]
                }]
            }
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert_eq!(err.field_path(), Some("workbook.sheets[0].charts[0].wrap"));
}

fn insert_col(tag: &str) -> PathBuf {
    let src = create_book(tag);
    let inserted = tmp(&format!("{tag}-ins"));
    apply(
        &src,
        &inserted,
        json!([{
            "op": "insertChart",
            "id": "sheet:Sheet1",
            "type": "col",
            "from": "E2",
            "dataFrom": "A10",
            "series": [{
                "name": "Q1",
                "categories": ["甲", "乙", "丙"],
                "values": [1, 2, 3]
            }, {
                "name": "Q2",
                "categories": ["甲", "乙", "丙"],
                "values": [4, 5, 6]
            }]
        }]),
    )
    .unwrap();
    inserted
}

fn rewrite_part(src: &Path, dest: &Path, part: &str, f: impl FnOnce(String) -> String) {
    use std::io::{Cursor, Read, Write};
    use zip::write::SimpleFileOptions;
    let bytes = std::fs::read(src).unwrap();
    let mut zin = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut buf = Cursor::new(Vec::new());
    let mut transform = Some(f);
    {
        let mut zout = zip::ZipWriter::new(&mut buf);
        let opt = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        for i in 0..zin.len() {
            let mut file = zin.by_index(i).unwrap();
            let name = file.name().to_string();
            let mut data = Vec::new();
            file.read_to_end(&mut data).unwrap();
            if name == part || name.replace('\\', "/") == part {
                data = transform.take().expect("部件只应出现一次")(
                    String::from_utf8(data).unwrap(),
                )
                .into_bytes();
            }
            zout.start_file(&name, opt).unwrap();
            zout.write_all(&data).unwrap();
        }
        zout.finish().unwrap();
    }
    std::fs::write(dest, buf.into_inner()).unwrap();
}

#[test]
fn trendline_and_secondary_axis() {
    let src = insert_col("tl");
    let out = tmp("tl-out");
    apply(
        &src,
        &out,
        json!([
            {
                "op": "insertTrendline",
                "id": "sheet:Sheet1/chart:1/series:0",
                "type": "linear"
            },
            {
                "op": "setChartSeriesAxis",
                "id": "sheet:Sheet1/chart:1/series:1",
                "axis": "secondary"
            }
        ]),
    )
    .unwrap();
    let tree = inspect_charts(&out);
    let c = &tree["sheets"][0]["charts"][0];
    assert_eq!(c["series"][0]["trendlines"][0]["type"], "linear");
    assert_eq!(c["series"][0]["axis"], "primary");
    assert_eq!(c["series"][1]["axis"], "secondary");
    let flags = c["flags"].as_array().unwrap();
    assert!(flags.iter().any(|f| f == "trendline"));
    assert!(flags.iter().any(|f| f == "secondaryAxis"));
    let xml = part_text(&out, "xl/charts/chart1.xml");
    let second = xml.split("<c:barChart>").nth(2).unwrap_or("");
    let wrap_end = second.find("</c:barChart>").unwrap_or(second.len());
    let wrap = &second[..wrap_end];
    let ser_at = wrap.find("<c:ser>").unwrap_or(usize::MAX);
    let dl_at = wrap.find("<c:dLbls>").unwrap_or(usize::MAX);
    assert!(ser_at < dl_at, "次轴包裹里 ser 必须在 dLbls 之前：{wrap}");
    assert!(
        !wrap.contains(r#"<c:axId val="1"/>"#),
        "次轴包裹不能共用主分类轴：{wrap}"
    );
    assert!(
        xml.contains(r#"<c:delete val="1"/>"#),
        "次轴必须带隐藏分类轴"
    );
}

#[test]
fn named_range_set_series() {
    let src = insert_col("nm");
    let named = tmp("nm-named");
    rewrite_part(&src, &named, "xl/workbook.xml", |s| {
        s.replace(
            "</workbook>",
            r#"<definedNames><definedName name="ChartVals">Sheet1!$B$11:$B$13</definedName></definedNames></workbook>"#,
        )
    });
    let named2 = tmp("nm-cf");
    rewrite_part(&named, &named2, "xl/charts/chart1.xml", |s| {
        s.replacen("Sheet1!$B$11:$B$13", "ChartVals", 1)
    });
    let tree = inspect_charts(&named2);
    assert_eq!(
        tree["sheets"][0]["charts"][0]["series"][0]["values"],
        json!([1, 2, 3])
    );
    assert_eq!(
        tree["sheets"][0]["charts"][0]["series"][0]["valRef"],
        "ChartVals"
    );
    let out = tmp("nm-out");
    apply(
        &named2,
        &out,
        json!([{
            "op": "setChartSeries",
            "id": "sheet:Sheet1/chart:1/series:0",
            "values": [9, 8, 7]
        }]),
    )
    .unwrap();
    let cells = inspect_cells(&out, "B11:B13");
    let list = cells["sheets"][0]["cells"].as_array().unwrap();
    assert!(list
        .iter()
        .any(|c| c["id"] == "cell:Sheet1!B11" && c["value"] == 9));
    assert!(list
        .iter()
        .any(|c| c["id"] == "cell:Sheet1!B13" && c["value"] == 7));
    let xml = part_text(&out, "xl/charts/chart1.xml");
    assert!(xml.contains("ChartVals"));
    let err = apply(
        &out,
        &tmp("nm-bad"),
        json!([{
            "op": "setChartSeries",
            "id": "sheet:Sheet1/chart:1/series:0",
            "values": [1, 2]
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::NotImplemented);
}

#[test]
fn union_formula_and_3d_reject() {
    let src = insert_col("un");
    let union = tmp("un-cf");
    rewrite_part(&src, &union, "xl/charts/chart1.xml", |s| {
        s.replacen("Sheet1!$B$11:$B$13", "Sheet1!$B$11,Sheet1!$B$13", 1)
    });
    let out = tmp("un-out");
    apply(
        &union,
        &out,
        json!([{
            "op": "setChartSeries",
            "id": "sheet:Sheet1/chart:1/series:0",
            "values": [11, 13]
        }]),
    )
    .unwrap();
    let cells = inspect_cells(&out, "B11:B13");
    let list = cells["sheets"][0]["cells"].as_array().unwrap();
    assert!(list
        .iter()
        .any(|c| c["id"] == "cell:Sheet1!B11" && c["value"] == 11));
    assert!(list
        .iter()
        .any(|c| c["id"] == "cell:Sheet1!B13" && c["value"] == 13));
    let xml = part_text(&out, "xl/charts/chart1.xml");
    assert!(xml.contains("Sheet1!$B$11,Sheet1!$B$13"));

    let three = tmp("3d-cf");
    rewrite_part(&src, &three, "xl/charts/chart1.xml", |s| {
        s.replacen("Sheet1!$B$11:$B$13", "Sheet1:Sheet2!$B$11", 1)
    });
    let err = apply(
        &three,
        &tmp("3d-out"),
        json!([{
            "op": "setChartSeries",
            "id": "sheet:Sheet1/chart:1/series:0",
            "values": [1]
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::NotImplemented);
}
