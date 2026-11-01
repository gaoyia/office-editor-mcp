//! 工作表图表：名单、插删、改系列与类型。

use std::io::{Cursor, Read, Write};
use std::path::Path;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

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
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "workbook": { "sheets": [{ "name": "Sheet1", "cells": [{ "ref": "A1", "value": 1 }] }] }
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

fn inspect_charts(path: &Path) -> serde_json::Value {
    let RunOutput::Inspect(tree) = run(&json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": path.to_string_lossy(),
        "scope": { "charts": true }
    })
    .to_string())
    .unwrap() else {
        panic!();
    };
    tree
}

fn inspect_cells(path: &Path, range: &str) -> serde_json::Value {
    let RunOutput::Inspect(tree) = run(&json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": path.to_string_lossy(),
        "scope": { "sheet": "Sheet1", "range": range }
    })
    .to_string())
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
    let RunOutput::Inspect(tree) = run(&json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": path.to_string_lossy()
    })
    .to_string())
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

    let err = run(&json!({
        "schemaVersion": 1,
        "action": "exportChartData",
        "path": out.to_string_lossy(),
        "id": "sheet:Sheet1/chart:1",
        "output": tmp("exp").with_extension("xlsx").to_string_lossy()
    })
    .to_string())
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
    assert_eq!(err.code(), ErrorCode::InvalidField);
    let sized = tmp("two-to");
    apply(
        &out,
        &sized,
        json!([{
            "op": "setChartSize",
            "id": "sheet:Sheet1/chart:1",
            "to": "E10"
        }]),
    )
    .unwrap();
    let after = inspect_charts(&sized);
    assert_eq!(after["sheets"][0]["charts"][0]["anchor"], "twoCell");
    assert_eq!(after["sheets"][0]["charts"][0]["to"], "E10");
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
    run(&json!({
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
    .to_string())
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
    let err = run(&json!({
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
    .to_string())
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

#[test]
fn insert_stock_surface_ofpie() {
    let src = create_book("rare");
    let out = tmp("ins-rare");
    apply(
        &src,
        &out,
        json!([
            {
                "op": "insertChart",
                "id": "sheet:Sheet1",
                "type": "stock",
                "from": "E2",
                "dataFrom": "A10",
                "title": "股价",
                "series": [
                    { "name": "高", "categories": ["甲", "乙", "丙"], "values": [30, 40, 50] },
                    { "name": "低", "categories": ["甲", "乙", "丙"], "values": [10, 15, 20] },
                    { "name": "收", "categories": ["甲", "乙", "丙"], "values": [20, 25, 35] }
                ]
            },
            {
                "op": "insertChart",
                "id": "sheet:Sheet1",
                "type": "surface",
                "from": "E20",
                "dataFrom": "F10",
                "series": [
                    { "name": "甲", "categories": ["A", "B"], "values": [1, 2] },
                    { "name": "乙", "categories": ["A", "B"], "values": [3, 3] },
                    { "name": "丙", "categories": ["A", "B"], "values": [2, 4] }
                ]
            },
            {
                "op": "insertChart",
                "id": "sheet:Sheet1",
                "type": "ofPie",
                "ofPieType": "bar",
                "from": "E38",
                "dataFrom": "K10",
                "series": [{ "name": "份额", "categories": ["A", "B", "C"], "values": [1, 2, 3] }]
            }
        ]),
    )
    .unwrap();
    let tree = inspect_charts(&out);
    let charts = tree["sheets"][0]["charts"].as_array().unwrap();
    let stock = charts.iter().find(|c| c["type"] == "stock").unwrap();
    assert_eq!(stock["series"].as_array().unwrap().len(), 3);
    assert_eq!(stock["stockKind"], "hlc");
    let xml = part_text(&out, "xl/charts/chart1.xml");
    assert!(xml.contains("stockChart"));
    assert!(xml.contains("hiLowLines"));
    let surface = charts.iter().find(|c| c["type"] == "surface").unwrap();
    assert_eq!(surface["series"].as_array().unwrap().len(), 3);
    assert_eq!(surface["surfaceKind"], "surface3d");
    let sxml = part_text(&out, "xl/charts/chart2.xml");
    assert!(sxml.contains("surface3DChart"));
    assert!(sxml.contains("view3D"));
    assert!(sxml.contains("serAx"));
    let ofpie = charts.iter().find(|c| c["type"] == "ofPie").unwrap();
    assert_eq!(ofpie["ofPieType"], "bar");
    let oxml = part_text(&out, "xl/charts/chart3.xml");
    assert!(oxml.contains("ofPieChart"));
    assert!(oxml.contains(r#"<c:ofPieType val="bar"/>"#));
    assert!(oxml.contains(r#"gapWidth val="150""#));
    let out2 = tmp("ins-rare-type");
    apply(
        &out,
        &out2,
        json!([
            { "op": "setChartType", "id": stock["id"], "type": "col" },
            { "op": "setChartType", "id": ofpie["id"], "type": "pie" }
        ]),
    )
    .unwrap();
    let tree2 = inspect_charts(&out2);
    let charts2 = tree2["sheets"][0]["charts"].as_array().unwrap();
    assert_eq!(
        charts2.iter().find(|c| c["id"] == stock["id"]).unwrap()["type"],
        "col"
    );
    assert_eq!(
        charts2.iter().find(|c| c["id"] == ofpie["id"]).unwrap()["type"],
        "pie"
    );
}

#[test]
fn insert_rare_rejects_wrong_series_and_chartex() {
    let src = create_book("rare-bad");
    let err = apply(
        &src,
        &tmp("rare-stock-2"),
        json!([{
            "op": "insertChart",
            "id": "sheet:Sheet1",
            "type": "stock",
            "from": "E2",
            "dataFrom": "A10",
            "series": [
                { "values": [1, 2] },
                { "values": [3, 4] }
            ]
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    let err = apply(
        &src,
        &tmp("rare-chartex"),
        json!([{
            "op": "insertChart",
            "id": "sheet:Sheet1",
            "type": "chartex",
            "from": "E2",
            "dataFrom": "A10",
            "series": [{ "values": [1, 2] }]
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    let col = insert_col("rare-col");
    let err = apply(
        &col,
        &tmp("rare-to-stock"),
        json!([{
            "op": "setChartType",
            "id": "sheet:Sheet1/chart:1",
            "type": "stock"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::NotImplemented);
    let err = apply(
        &col,
        &tmp("rare-ofpie-on-col"),
        json!([{
            "op": "setChartType",
            "id": "sheet:Sheet1/chart:1",
            "type": "col",
            "ofPieType": "pie"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    let stock = tmp("rare-add");
    apply(
        &src,
        &stock,
        json!([{
            "op": "insertChart",
            "id": "sheet:Sheet1",
            "type": "stock",
            "from": "E2",
            "dataFrom": "A10",
            "series": [
                { "name": "高", "categories": ["甲", "乙"], "values": [3, 4] },
                { "name": "低", "categories": ["甲", "乙"], "values": [1, 2] },
                { "name": "收", "categories": ["甲", "乙"], "values": [2, 3] }
            ]
        }]),
    )
    .unwrap();
    let before = std::fs::read(&stock).unwrap();
    let err = apply(
        &stock,
        &tmp("rare-add-out"),
        json!([{
            "op": "addChartSeries",
            "id": "sheet:Sheet1/chart:1",
            "values": [1, 2]
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::NotImplemented);
    assert_eq!(std::fs::read(&stock).unwrap(), before);
}

#[test]
fn insert_ohlc_contour_chartex_and_cust_error_bars() {
    let src = create_book("cx-remain");
    let out = tmp("cx-remain");
    apply(
        &src,
        &out,
        json!([
            {
                "op": "insertChart",
                "id": "sheet:Sheet1",
                "type": "stock",
                "stockKind": "ohlc",
                "from": "E2",
                "dataFrom": "A10",
                "series": [
                    { "name": "开", "categories": ["甲", "乙"], "values": [20, 25] },
                    { "name": "高", "categories": ["甲", "乙"], "values": [30, 40] },
                    { "name": "低", "categories": ["甲", "乙"], "values": [10, 15] },
                    { "name": "收", "categories": ["甲", "乙"], "values": [22, 28] }
                ]
            },
            {
                "op": "insertChart",
                "id": "sheet:Sheet1",
                "type": "surface",
                "surfaceKind": "contourWire",
                "from": "E20",
                "dataFrom": "F10",
                "series": [
                    { "name": "甲", "categories": ["A", "B"], "values": [1, 2] },
                    { "name": "乙", "categories": ["A", "B"], "values": [3, 3] },
                    { "name": "丙", "categories": ["A", "B"], "values": [2, 4] }
                ]
            },
            {
                "op": "insertChart",
                "id": "sheet:Sheet1",
                "type": "chartex",
                "layoutId": "funnel",
                "from": "E38",
                "dataFrom": "K10",
                "title": "漏斗",
                "series": [{ "name": "阶段", "categories": ["A", "B", "C"], "values": [10, 6, 2] }]
            }
        ]),
    )
    .unwrap();
    let tree = inspect_charts(&out);
    let charts = tree["sheets"][0]["charts"].as_array().unwrap();
    let ohlc = charts.iter().find(|c| c["type"] == "stock").unwrap();
    assert_eq!(ohlc["stockKind"], "ohlc");
    assert_eq!(ohlc["series"].as_array().unwrap().len(), 4);
    let xml = part_text(&out, "xl/charts/chart1.xml");
    assert!(xml.contains("upDownBars"));
    let surface = charts.iter().find(|c| c["type"] == "surface").unwrap();
    assert_eq!(surface["surfaceKind"], "contourWire");
    let sxml = part_text(&out, "xl/charts/chart2.xml");
    assert!(sxml.contains("surfaceChart"));
    assert!(sxml.contains(r#"<c:wireframe val="1"/>"#));
    let cx = charts.iter().find(|c| c["type"] == "chartex").unwrap();
    assert_eq!(cx["layoutId"], "funnel");
    assert_eq!(cx["title"], "漏斗");
    let cxml = part_text(&out, "xl/charts/chartEx1.xml");
    assert!(cxml.contains("cx:chartSpace"));
    assert!(cxml.contains(r#"layoutId="funnel""#));
    assert!(cxml.contains("Sheet1"));
    let crels = part_text(&out, "xl/charts/_rels/chartEx1.xml.rels");
    assert!(crels.contains("chartStyle"));
    assert!(crels.contains("chartColorStyle"));
    let draw = part_text(&out, "xl/drawings/drawing1.xml");
    assert!(draw.contains("http://schemas.microsoft.com/office/drawing/2014/chartex"));
    let vol = tmp("cx-vhlc");
    apply(
        &src,
        &vol,
        json!([{
            "op": "insertChart",
            "id": "sheet:Sheet1",
            "type": "stock",
            "stockKind": "vhlc",
            "from": "E2",
            "dataFrom": "A10",
            "series": [
                { "name": "量", "categories": ["甲", "乙"], "values": [100, 80] },
                { "name": "高", "categories": ["甲", "乙"], "values": [30, 40] },
                { "name": "低", "categories": ["甲", "乙"], "values": [10, 15] },
                { "name": "收", "categories": ["甲", "乙"], "values": [22, 28] }
            ]
        }]),
    )
    .unwrap();
    let vch = &inspect_charts(&vol)["sheets"][0]["charts"][0];
    assert_eq!(vch["type"], "combo");
    assert!(vch.get("stockKind").is_none());
    let col = insert_col("cx-eb");
    let eb = tmp("cx-eb");
    apply(
        &col,
        &eb,
        json!([{
            "op": "insertErrorBars",
            "id": "sheet:Sheet1/chart:1/series:0",
            "valType": "cust",
            "plus": [0.5, 0.4, 0.3],
            "minus": [0.2, 0.1, 0.1]
        }]),
    )
    .unwrap();
    let bars = &inspect_charts(&eb)["sheets"][0]["charts"][0]["series"][0]["errorBars"][0];
    assert_eq!(bars["dir"], "y");
    assert_eq!(bars["valType"], "cust");
    assert_eq!(bars["plus"], json!([0.5, 0.4, 0.3]));
    let exml = part_text(&eb, "xl/charts/chart1.xml");
    assert!(!exml.contains("<c:errDir"));
    let err = apply(
        &src,
        &tmp("cx-region"),
        json!([{
            "op": "insertChart",
            "id": "sheet:Sheet1",
            "type": "chartex",
            "layoutId": "regionMap",
            "from": "E2",
            "dataFrom": "A10",
            "series": [{ "values": [1] }]
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::NotImplemented);
    let err = apply(
        &col,
        &tmp("cx-set-type"),
        json!([{
            "op": "setChartType",
            "id": "sheet:Sheet1/chart:1",
            "type": "chartex",
            "layoutId": "funnel"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    let err = apply(
        &col,
        &tmp("cx-lcaps"),
        json!([{
            "op": "insertErrorBars",
            "id": "sheet:Sheet1/chart:1/series:0",
            "valType": "lCaps"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

#[test]
fn chartex_series_title_and_legend() {
    let src = create_book("cx-edit");
    let made = tmp("cx-made");
    apply(
        &src,
        &made,
        json!([{
            "op": "insertChart",
            "id": "sheet:Sheet1",
            "type": "chartex",
            "layoutId": "waterfall",
            "from": "E2",
            "dataFrom": "A1",
            "title": "旧标题",
            "series": [{
                "name": "值",
                "categories": ["甲", "乙"],
                "values": [10, 4]
            }]
        }]),
    )
    .unwrap();
    let edited = tmp("cx-edited");
    apply(
        &made,
        &edited,
        json!([
            {
                "op": "setChartSeries",
                "id": "sheet:Sheet1/chart:1/series:0",
                "values": [42, 7]
            },
            {
                "op": "setChartTitle",
                "id": "sheet:Sheet1/chart:1",
                "title": "新标题"
            },
            {
                "op": "setChartLegend",
                "id": "sheet:Sheet1/chart:1",
                "show": true,
                "pos": "b"
            }
        ]),
    )
    .unwrap();
    let chart = part_text(&edited, "xl/charts/chartEx1.xml");
    assert!(chart.contains(">42</cx:pt>"), "{chart}");
    assert!(chart.contains(">7</cx:pt>"), "{chart}");
    assert!(chart.contains("新标题"), "{chart}");
    assert!(chart.contains(r#"pos="b""#), "{chart}");
    let sheet = part_text(&edited, "xl/worksheets/sheet1.xml");
    assert!(sheet.contains(">42<"), "{sheet}");
    assert!(sheet.contains(">7<"), "{sheet}");
    let err = apply(
        &edited,
        &tmp("cx-trend"),
        json!([{
            "op": "insertTrendline",
            "id": "sheet:Sheet1/chart:1/series:0",
            "type": "linear"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::NotImplemented);
    let added = tmp("cx-added");
    apply(
        &edited,
        &added,
        json!([{
            "op": "addChartSeries",
            "id": "sheet:Sheet1/chart:1",
            "name": "第二",
            "values": [1, 2]
        }]),
    )
    .unwrap();
    let chart = part_text(&added, "xl/charts/chartEx1.xml");
    assert!(chart.matches("<cx:series ").count() >= 2, "{chart}");
    assert!(chart.contains(">1</cx:pt>"), "{chart}");
    let sheet = part_text(&added, "xl/worksheets/sheet1.xml");
    assert!(sheet.contains(">1<"), "{sheet}");
    let trimmed = tmp("cx-trimmed");
    apply(
        &added,
        &trimmed,
        json!([{
            "op": "deleteChartSeries",
            "id": "sheet:Sheet1/chart:1/series:1"
        }]),
    )
    .unwrap();
    let chart = part_text(&trimmed, "xl/charts/chartEx1.xml");
    assert_eq!(chart.matches("<cx:series ").count(), 1, "{chart}");
    let last = apply(
        &trimmed,
        &tmp("cx-last"),
        json!([{
            "op": "deleteChartSeries",
            "id": "sheet:Sheet1/chart:1/series:0"
        }]),
    )
    .unwrap_err();
    assert_eq!(last.code(), ErrorCode::InvalidField);
}

fn strip_lvl(xml: &str) -> String {
    let mut out = String::new();
    let mut rest = xml;
    while let Some(i) = rest.find("<cx:lvl") {
        out.push_str(&rest[..i]);
        let after = &rest[i..];
        let end = after.find("</cx:lvl>").expect("cx:lvl");
        rest = &after[end + "</cx:lvl>".len()..];
    }
    out.push_str(rest);
    out
}

fn rewrite_parts(src: &Path, dest: &Path, parts: &[(&str, &str)]) {
    let bytes = std::fs::read(src).unwrap();
    let mut zin = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut buf = Cursor::new(Vec::new());
    {
        let mut zout = ZipWriter::new(&mut buf);
        let opt = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        for i in 0..zin.len() {
            let mut file = zin.by_index(i).unwrap();
            let name = file.name().to_string();
            let mut data = Vec::new();
            file.read_to_end(&mut data).unwrap();
            if let Some((_, text)) = parts.iter().find(|(n, _)| *n == name) {
                data = text.as_bytes().to_vec();
            }
            zout.start_file(name, opt).unwrap();
            zout.write_all(&data).unwrap();
        }
        zout.finish().unwrap();
    }
    std::fs::write(dest, buf.into_inner()).unwrap();
}

#[test]
fn chartex_reads_cells_when_the_cache_is_gone() {
    let src = create_book("cx-cache");
    let made = tmp("cx-cache-made");
    apply(
        &src,
        &made,
        json!([{
            "op": "insertChart",
            "id": "sheet:Sheet1",
            "type": "chartex",
            "layoutId": "waterfall",
            "from": "E2",
            "dataFrom": "A1",
            "series": [{
                "name": "值",
                "categories": ["甲", "乙"],
                "values": [10, 4]
            }]
        }]),
    )
    .unwrap();
    let chart = part_text(&made, "xl/charts/chartEx1.xml");
    assert!(chart.contains("Sheet1!$A$2:$A$3"), "{chart}");
    assert!(chart.contains("Sheet1!$B$2:$B$3"), "{chart}");
    let bare_chart = strip_lvl(&chart);
    assert!(!bare_chart.contains("<cx:lvl"), "{bare_chart}");
    let bare = tmp("cx-cache-bare");
    rewrite_parts(
        &made,
        &bare,
        &[("xl/charts/chartEx1.xml", bare_chart.as_str())],
    );
    let ser = &inspect_charts(&bare)["sheets"][0]["charts"][0]["series"][0];
    assert_eq!(ser["categories"], json!(["甲", "乙"]));
    assert_eq!(ser["values"], json!([10, 4]));
    assert_eq!(ser["name"], "值");
    assert_eq!(ser["valRef"], "Sheet1!$B$2:$B$3");
    assert!(ser.get("cacheStale").is_none());

    let named_chart = bare_chart
        .replace("Sheet1!$A$2:$A$3", "_xlchart.v1.0")
        .replace("Sheet1!$B$1", "_xlchart.v1.1")
        .replace("Sheet1!$B$2:$B$3", "_xlchart.v1.2");
    let wb = part_text(&made, "xl/workbook.xml").replace(
        "</workbook>",
        r#"<definedNames><definedName name="_xlchart.v1.0" hidden="1">Sheet1!$A$2:$A$3</definedName><definedName name="_xlchart.v1.1" hidden="1">Sheet1!$B$1</definedName><definedName name="_xlchart.v1.2" hidden="1">Sheet1!$B$2:$B$3</definedName></definedNames></workbook>"#,
    );
    let named = tmp("cx-cache-named");
    rewrite_parts(
        &made,
        &named,
        &[
            ("xl/charts/chartEx1.xml", named_chart.as_str()),
            ("xl/workbook.xml", wb.as_str()),
        ],
    );
    let ser = &inspect_charts(&named)["sheets"][0]["charts"][0]["series"][0];
    assert_eq!(ser["categories"], json!(["甲", "乙"]));
    assert_eq!(ser["values"], json!([10, 4]));
    assert_eq!(ser["name"], "值");
    assert_eq!(ser["catRef"], "_xlchart.v1.0");
    assert_eq!(ser["nameRef"], "_xlchart.v1.1");
    assert_eq!(ser["valRef"], "_xlchart.v1.2");

    let edited = tmp("cx-cache-set");
    apply(
        &named,
        &edited,
        json!([{
            "op": "setChartSeries",
            "id": "sheet:Sheet1/chart:1/series:0",
            "values": [7, 8]
        }]),
    )
    .unwrap();
    let ser = &inspect_charts(&edited)["sheets"][0]["charts"][0]["series"][0];
    assert_eq!(ser["values"], json!([7, 8]));
}

fn wrap_marked(xml: &str, open: &str, close: &str, specs: &[(&str, &str)]) -> String {
    let mut ranges = Vec::new();
    let mut rest = 0usize;
    while let Some(rel) = xml[rest..].find(open) {
        let start = rest + rel;
        let end_rel = xml[start..]
            .find(close)
            .unwrap_or_else(|| panic!("{close}"));
        let end = start + end_rel + close.len();
        ranges.push((start, end));
        rest = end;
    }
    assert_eq!(ranges.len(), specs.len(), "{xml}");
    let mut out = xml.to_string();
    for (i, (requires, ns)) in specs.iter().enumerate().rev() {
        let (start, end) = ranges[i];
        let inner = out[start..end].to_string();
        let id = 90 + i;
        let fallback = format!(
            r#"<xdr:sp><xdr:nvSpPr><xdr:cNvPr id="{id}" name="海报{id}"/><xdr:cNvSpPr/></xdr:nvSpPr><xdr:spPr/></xdr:sp>"#
        );
        let wrapped = format!(
            r#"<mc:AlternateContent xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006"><mc:Choice xmlns:{requires}="{ns}" Requires="{requires}">{inner}</mc:Choice><mc:Fallback>{fallback}</mc:Fallback></mc:AlternateContent>"#
        );
        out.replace_range(start..end, &wrapped);
    }
    out
}

#[test]
fn chartex_alternate_content_prefixes_stay_charts() {
    let src = create_book("cx-ac");
    let made = tmp("cx-ac-made");
    apply(
        &src,
        &made,
        json!([
            {
                "op": "insertChart", "id": "sheet:Sheet1", "type": "chartex",
                "layoutId": "waterfall", "from": "E2", "dataFrom": "A1", "title": "瀑布",
                "series": [{ "name": "值", "categories": ["甲", "乙"], "values": [10, 4] }]
            },
            {
                "op": "insertChart", "id": "sheet:Sheet1", "type": "chartex",
                "layoutId": "funnel", "from": "E16", "dataFrom": "A10", "title": "漏斗",
                "series": [{ "name": "值", "categories": ["甲", "乙"], "values": [10, 4] }]
            },
            {
                "op": "insertChart", "id": "sheet:Sheet1", "type": "chartex",
                "layoutId": "clusteredColumn", "from": "E30", "dataFrom": "A20", "title": "直方",
                "series": [{ "name": "值", "categories": ["甲", "乙"], "values": [10, 4] }]
            }
        ]),
    )
    .unwrap();
    let drawing = part_text(&made, "xl/drawings/drawing1.xml");
    let wrapped = wrap_marked(
        &drawing,
        "<xdr:graphicFrame",
        "</xdr:graphicFrame>",
        &[
            (
                "cx1",
                "http://schemas.microsoft.com/office/drawing/2015/9/8/chartex",
            ),
            (
                "cx2",
                "http://schemas.microsoft.com/office/drawing/2015/10/21/chartex",
            ),
            (
                "cx7",
                "http://schemas.microsoft.com/office/drawing/2016/5/13/chartex",
            ),
        ],
    );
    assert!(wrapped.contains(r#"Requires="cx2""#));
    assert!(wrapped.contains("海报90"));
    let packed = tmp("cx-ac-packed");
    rewrite_part(&made, &packed, "xl/drawings/drawing1.xml", |_| wrapped);
    let tree = inspect_charts(&packed);
    let sheet = &tree["sheets"][0];
    assert_eq!(sheet["chartCount"], 3);
    assert_eq!(sheet["shapeCount"], 0);
    let charts = sheet["charts"].as_array().unwrap();
    let layouts: Vec<_> = charts
        .iter()
        .map(|c| c["layoutId"].as_str().unwrap())
        .collect();
    assert!(layouts.contains(&"waterfall"), "{layouts:?}");
    assert!(layouts.contains(&"funnel"), "{layouts:?}");
    assert!(layouts.contains(&"clusteredColumn"), "{layouts:?}");
    let funnel = charts.iter().find(|c| c["layoutId"] == "funnel").unwrap();
    let edited = tmp("cx-ac-set");
    apply(
        &packed,
        &edited,
        json!([{
            "op": "setChartSeries",
            "id": format!("{}/series:0", funnel["id"].as_str().unwrap()),
            "values": [7, 8]
        }]),
    )
    .unwrap();
    let again = part_text(&edited, "xl/drawings/drawing1.xml");
    assert!(again.contains(r#"Requires="cx1""#), "{again}");
    assert!(again.contains(r#"Requires="cx2""#), "{again}");
    assert!(again.contains(r#"Requires="cx7""#), "{again}");
    assert!(again.contains("海报90"), "{again}");
    assert!(again.contains("<mc:Fallback>"), "{again}");
    let back = inspect_charts(&edited);
    let funnel = back["sheets"][0]["charts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["layoutId"] == "funnel")
        .unwrap();
    assert_eq!(funnel["series"][0]["values"], json!([7, 8]));
    assert_eq!(back["sheets"][0]["shapeCount"], 0);
}
