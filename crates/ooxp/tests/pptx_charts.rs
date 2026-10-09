//! 幻灯片图表：名单、插删、改系列与画布坐标、导出 embedding。

use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::ZipArchive;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-pptx-charts");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn tmp(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}"))
}

fn create_deck(tag: &str) -> PathBuf {
    let path = tmp(tag).with_extension("pptx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "presentation": { "slides": [{ "shapes": [{ "text": "标题" }] }] }
        })
        .to_string(),
        None,
    )
    .unwrap();
    path
}

fn apply(src: &Path, out: &Path, ops: Value) -> Result<RunOutput, ooxp::Error> {
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

fn inspect_bare(path: &Path) -> Value {
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
        panic!("inspect 应返回树");
    };
    tree
}

fn inspect_charts(path: &Path) -> Value {
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
        panic!("inspect 应返回树");
    };
    tree
}

fn object_keys(v: &Value) -> Vec<&str> {
    v.as_object().unwrap().keys().map(String::as_str).collect()
}

fn part_bytes(path: &Path, name: &str) -> Vec<u8> {
    let bytes = std::fs::read(path).unwrap();
    let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut file = zip.by_name(name).unwrap();
    let mut out = Vec::new();
    file.read_to_end(&mut out).unwrap();
    out
}

fn part_names(path: &Path) -> Vec<String> {
    let bytes = std::fs::read(path).unwrap();
    let zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    zip.file_names().map(str::to_string).collect()
}

fn embed_part(path: &Path) -> String {
    part_names(path)
        .into_iter()
        .find(|n| n.starts_with("ppt/embeddings/") && n.ends_with(".xlsx"))
        .expect("应有 embedding")
}

fn insert_col(src: &Path, out: &Path) {
    apply(
        src,
        out,
        json!([{
            "op": "insertChart",
            "id": "slide:1",
            "type": "col",
            "title": "销量",
            "x": 914400,
            "y": 1371600,
            "cx": 5486400,
            "cy": 3200400,
            "series": [{
                "name": "系列甲",
                "categories": ["甲", "乙", "丙"],
                "values": [1, 2, 3]
            }]
        }]),
    )
    .unwrap();
}

fn nums(v: &Value) -> Vec<i64> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_i64().or_else(|| x.as_f64().map(|f| f as i64)).unwrap())
        .collect()
}

#[test]
fn inspect_chart_count_without_list() {
    let path = create_deck("count");
    let tree = inspect_bare(&path);
    assert_eq!(tree["chartCount"], 0);
    assert_eq!(tree["slides"][0]["chartCount"], 0);
    assert!(tree["slides"][0].get("charts").is_none());
    assert_eq!(
        object_keys(&tree),
        [
            "schemaVersion",
            "format",
            "id",
            "slideCount",
            "pictureCount",
            "mathCount",
            "videoCount",
            "audioCount",
            "chartCount",
            "smartArtCount",
            "commentCount",
            "hyperlinkCount",
            "fieldCount",
            "tableCount",
            "oleObjectCount",
            "model3dCount",
            "zoomCount",
            "connectorCount",
            "animCount",
            "masterCount",
            "hasHandout",
            "sectionCount",
            "slideSize",
            "customShowCount",
            "showPr",
            "modifyProtection",
            "props",
            "slides"
        ]
    );
}

#[test]
fn charts_scope_does_not_need_slide() {
    let src = create_deck("scope");
    let out = tmp("scope").with_extension("pptx");
    insert_col(&src, &out);
    let tree = inspect_charts(&out);
    assert!(tree["slides"][0].get("shapes").is_none());
    assert_eq!(tree["chartCount"], 1);
    assert_eq!(tree["slides"][0]["charts"].as_array().unwrap().len(), 1);
}

#[test]
fn insert_inspect_series_pos_type_delete_export() {
    let src = create_deck("round");
    let inserted = tmp("ins").with_extension("pptx");
    insert_col(&src, &inserted);

    let tree = inspect_charts(&inserted);
    assert_eq!(tree["chartCount"], 1);
    assert_eq!(tree["slides"][0]["chartCount"], 1);
    let charts = tree["slides"][0]["charts"].as_array().unwrap();
    assert_eq!(charts.len(), 1);
    assert_eq!(charts[0]["id"], "slide:1/chart:3");
    assert_eq!(charts[0]["cNvPrId"], "3");
    assert_eq!(charts[0]["type"], "col");
    assert_eq!(charts[0]["title"], "销量");
    assert_eq!(charts[0]["x"], 914400);
    assert_eq!(charts[0]["y"], 1371600);
    assert_eq!(charts[0]["cx"], 5486400);
    assert_eq!(charts[0]["cy"], 3200400);
    assert_eq!(charts[0]["stable"], true);
    assert_eq!(
        object_keys(&charts[0]),
        [
            "id",
            "cNvPrId",
            "name",
            "x",
            "y",
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
    assert_eq!(ser["id"], "slide:1/chart:3/series:0");
    assert_eq!(ser["index"], 0);
    assert_eq!(ser["name"], "系列甲");
    assert_eq!(ser["categories"], json!(["甲", "乙", "丙"]));
    assert_eq!(nums(&ser["values"]), vec![1, 2, 3]);
    assert_eq!(ser["axis"], "primary");
    assert!(ser.get("cacheStale").is_none());

    let names = part_names(&inserted);
    assert!(names.iter().any(|n| n == "ppt/charts/chart1.xml"));
    assert!(names
        .iter()
        .any(|n| n.starts_with("ppt/embeddings/") && n.ends_with(".xlsx")));
    let chart_xml = String::from_utf8(part_bytes(&inserted, "ppt/charts/chart1.xml")).unwrap();
    assert!(chart_xml.contains("externalData"));

    let embed_name = embed_part(&inserted);
    let embed = part_bytes(&inserted, &embed_name);
    let exported = tmp("exp").with_extension("xlsx");
    let RunOutput::Written { content_type, .. } = run(
        &json!({
            "schemaVersion": 1,
            "action": "exportChartData",
            "path": inserted.to_string_lossy(),
            "id": "slide:1/chart:3",
            "output": exported.to_string_lossy()
        })
        .to_string(),
        None,
    )
    .unwrap() else {
        panic!("exportChartData 应写出文件");
    };
    assert_eq!(std::fs::read(&exported).unwrap(), embed);
    assert_eq!(
        content_type.as_deref(),
        Some("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet")
    );

    let updated = tmp("ser").with_extension("pptx");
    apply(
        &inserted,
        &updated,
        json!([{
            "op": "setChartSeries",
            "id": "slide:1/chart:3/series:0",
            "values": [10, 20, 30]
        }]),
    )
    .unwrap();
    let after = inspect_charts(&updated);
    assert_eq!(
        nums(&after["slides"][0]["charts"][0]["series"][0]["values"]),
        vec![10, 20, 30]
    );
    assert!(after["slides"][0]["charts"][0]["series"][0]
        .get("cacheStale")
        .is_none());
    let embed2_name = embed_part(&updated);
    let embed2 = part_bytes(&updated, &embed2_name);
    let exported2 = tmp("exp2").with_extension("xlsx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "exportChartData",
            "path": updated.to_string_lossy(),
            "id": "slide:1/chart:3",
            "output": exported2.to_string_lossy()
        })
        .to_string(),
        None,
    )
    .unwrap();
    assert_eq!(std::fs::read(&exported2).unwrap(), embed2);
    let RunOutput::Inspect(cells) = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": exported2.to_string_lossy(),
            "scope": { "sheet": "Sheet1", "range": "B2:B4" }
        })
        .to_string(),
        None,
    )
    .unwrap() else {
        panic!("应能 inspect embedding");
    };
    let list = cells["sheets"][0]["cells"].as_array().unwrap();
    assert!(list
        .iter()
        .any(|c| c["id"] == "cell:Sheet1!B2" && nums(&json!([c["value"].clone()])) == vec![10]));
    assert!(list
        .iter()
        .any(|c| c["id"] == "cell:Sheet1!B4" && nums(&json!([c["value"].clone()])) == vec![30]));

    let lined = tmp("line").with_extension("pptx");
    apply(
        &updated,
        &lined,
        json!([{ "op": "setChartType", "id": "slide:1/chart:3", "type": "line" }]),
    )
    .unwrap();
    assert_eq!(
        inspect_charts(&lined)["slides"][0]["charts"][0]["type"],
        "line"
    );

    let titled = tmp("title").with_extension("pptx");
    apply(
        &lined,
        &titled,
        json!([{ "op": "setChartTitle", "id": "slide:1/chart:3", "title": "新标题" }]),
    )
    .unwrap();
    assert_eq!(
        inspect_charts(&titled)["slides"][0]["charts"][0]["title"],
        "新标题"
    );

    let sized = tmp("size").with_extension("pptx");
    apply(
        &titled,
        &sized,
        json!([{
            "op": "setChartSize",
            "id": "slide:1/chart:3",
            "cx": 1828800,
            "cy": 914400
        }]),
    )
    .unwrap();
    let after_sz = inspect_charts(&sized);
    assert_eq!(after_sz["slides"][0]["charts"][0]["cx"], 1828800);
    assert_eq!(after_sz["slides"][0]["charts"][0]["cy"], 914400);
    assert_eq!(after_sz["slides"][0]["charts"][0]["x"], 914400);

    let moved = tmp("pos").with_extension("pptx");
    apply(
        &sized,
        &moved,
        json!([{
            "op": "setChartPos",
            "id": "slide:1/chart:3",
            "x": 1828800,
            "y": 0
        }]),
    )
    .unwrap();
    let after_pos = inspect_charts(&moved);
    assert_eq!(after_pos["slides"][0]["charts"][0]["x"], 1828800);
    assert_eq!(after_pos["slides"][0]["charts"][0]["y"], 0);
    assert_eq!(after_pos["slides"][0]["charts"][0]["cx"], 1828800);

    let deleted = tmp("del").with_extension("pptx");
    apply(
        &moved,
        &deleted,
        json!([{ "op": "deleteChart", "id": "slide:1/chart:3" }]),
    )
    .unwrap();
    let after_del = inspect_charts(&deleted);
    assert_eq!(after_del["chartCount"], 0);
    assert_eq!(after_del["slides"][0]["charts"], json!([]));
    let names = part_names(&deleted);
    assert!(!names.iter().any(|n| n.starts_with("ppt/charts/")));
    assert!(!names.iter().any(|n| n.starts_with("ppt/embeddings/")));
}

#[test]
fn insert_without_coords_avoids_origin() {
    let src = create_deck("auto");
    let out = tmp("auto").with_extension("pptx");
    apply(
        &src,
        &out,
        json!([{
            "op": "insertChart",
            "id": "slide:1",
            "type": "col",
            "series": [{
                "name": "S1",
                "categories": ["a", "b"],
                "values": [1, 2]
            }]
        }]),
    )
    .unwrap();
    let c = &inspect_charts(&out)["slides"][0]["charts"][0];
    assert_eq!(c["cx"], 5486400);
    assert_eq!(c["cy"], 3200400);
    let x = c["x"].as_i64().unwrap();
    let y = c["y"].as_i64().unwrap();
    assert!(x > 0 || y > 0, "缺坐标时应避开原点 x={x} y={y}");
}

#[test]
fn add_and_delete_series() {
    let src = create_deck("add");
    let inserted = tmp("add-ins").with_extension("pptx");
    insert_col(&src, &inserted);
    let added = tmp("added").with_extension("pptx");
    apply(
        &inserted,
        &added,
        json!([{
            "op": "addChartSeries",
            "id": "slide:1/chart:3",
            "name": "系列乙",
            "values": [4, 5, 6]
        }]),
    )
    .unwrap();
    let tree = inspect_charts(&added);
    let series = tree["slides"][0]["charts"][0]["series"].as_array().unwrap();
    assert_eq!(series.len(), 2);
    assert_eq!(series[1]["name"], "系列乙");
    assert_eq!(nums(&series[1]["values"]), vec![4, 5, 6]);

    let trimmed = tmp("trim").with_extension("pptx");
    apply(
        &added,
        &trimmed,
        json!([{ "op": "deleteChartSeries", "id": "slide:1/chart:3/series:1" }]),
    )
    .unwrap();
    assert_eq!(
        inspect_charts(&trimmed)["slides"][0]["charts"][0]["series"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let err = apply(
        &trimmed,
        &tmp("last").with_extension("pptx"),
        json!([{ "op": "deleteChartSeries", "id": "slide:1/chart:3/series:0" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

#[test]
fn rejects_foreign_chart_fields_and_set_chart_pos_on_other_formats() {
    let src = create_deck("rej");
    let err = apply(
        &src,
        &tmp("from").with_extension("pptx"),
        json!([{
            "op": "insertChart",
            "id": "slide:1",
            "type": "col",
            "from": "A1",
            "dataFrom": "A10",
            "series": [{ "values": [1, 2], "categories": ["a", "b"] }]
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let err = apply(
        &src,
        &tmp("kind").with_extension("pptx"),
        json!([{
            "op": "insertChart",
            "id": "slide:1",
            "type": "col",
            "kind": "anchor",
            "wrap": "square",
            "series": [{ "values": [1, 2], "categories": ["a", "b"] }]
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let inserted = tmp("ok").with_extension("pptx");
    insert_col(&src, &inserted);
    let err = apply(
        &inserted,
        &tmp("wrap").with_extension("pptx"),
        json!([{
            "op": "setChartWrap",
            "id": "slide:1/chart:3",
            "kind": "anchor",
            "wrap": "square"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    let err = apply(
        &inserted,
        &tmp("anchor").with_extension("pptx"),
        json!([{
            "op": "setChartAnchor",
            "id": "slide:1/chart:3",
            "from": "A1"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    let err = apply(
        &inserted,
        &tmp("para").with_extension("pptx"),
        json!([{ "op": "setChartTitle", "id": "para:AAAA0001/chart:7", "title": "x" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    let err = apply(
        &inserted,
        &tmp("needs-slide").with_extension("pptx"),
        json!([{
            "op": "insertChart",
            "id": "slide:1/chart:3",
            "type": "col",
            "series": [{ "values": [1], "categories": ["a"] }]
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);

    let xlsx = tmp("book").with_extension("xlsx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": xlsx.to_string_lossy(),
            "workbook": { "sheets": [{ "name": "Sheet1", "cells": [{ "ref": "A1", "value": 1 }] }] }
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
            "output": tmp("xlsx-pos").with_extension("xlsx").to_string_lossy(),
            "ops": [{ "op": "setChartPos", "id": "sheet:Sheet1/chart:1", "x": 1, "y": 1 }]
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "exportChartData",
            "path": xlsx.to_string_lossy(),
            "id": "sheet:Sheet1/chart:1",
            "output": tmp("xlsx-exp").with_extension("xlsx").to_string_lossy()
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let docx = tmp("doc").with_extension("docx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": docx.to_string_lossy(),
            "document": { "paragraphs": [{ "text": "hi" }] }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": docx.to_string_lossy(),
            "output": tmp("docx-pos").with_extension("docx").to_string_lossy(),
            "ops": [{ "op": "setChartPos", "id": "para:AAAA0001/chart:7", "x": 1, "y": 1 }]
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

#[test]
fn create_writes_slide_chart() {
    let path = tmp("create-chart").with_extension("pptx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "presentation": {
                "slides": [{
                    "shapes": [{ "text": "标题" }],
                    "charts": [{
                        "type": "col",
                        "title": "销量",
                        "x": 914400,
                        "y": 1828800,
                        "cx": 5486400,
                        "cy": 3200400,
                        "series": [{
                            "name": "系列甲",
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
    assert_eq!(tree["slides"][0]["chartCount"], 1);
    let c = &tree["slides"][0]["charts"][0];
    assert_eq!(c["id"], "slide:1/chart:3");
    assert_eq!(c["type"], "col");
    assert_eq!(c["title"], "销量");
    assert_eq!(c["x"], 914400);
    assert_eq!(c["y"], 1828800);
    assert_eq!(nums(&c["series"][0]["values"]), vec![1, 2, 3]);
    let names = part_names(&path);
    assert!(names.iter().any(|n| n == "ppt/charts/chart1.xml"));
    assert!(names
        .iter()
        .any(|n| n.starts_with("ppt/embeddings/") && n.ends_with(".xlsx")));
    let embed = part_bytes(&path, &embed_part(&path));
    let exported = tmp("create-exp").with_extension("xlsx");
    let RunOutput::Written { .. } = run(
        &json!({
            "schemaVersion": 1,
            "action": "exportChartData",
            "path": path.to_string_lossy(),
            "id": "slide:1/chart:3",
            "output": exported.to_string_lossy()
        })
        .to_string(),
        None,
    )
    .unwrap() else {
        panic!("exportChartData 应写出文件");
    };
    assert_eq!(std::fs::read(&exported).unwrap(), embed);
}

#[test]
fn create_empty_charts_writes_no_chart() {
    let path = tmp("empty-charts").with_extension("pptx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "presentation": {
                "slides": [{ "shapes": [{ "text": "a" }], "charts": [] }]
            }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let tree = inspect_charts(&path);
    assert_eq!(tree["chartCount"], 0);
    assert_eq!(tree["slides"][0]["charts"], json!([]));
    assert!(!part_names(&path).iter().any(|n| n.contains("charts/chart")));
}

#[test]
fn create_picture_then_chart_ids() {
    let png = tmp("pic").with_extension("png");
    std::fs::write(
        &png,
        [
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48,
            0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00,
            0x00, 0x90, 0x77, 0x53, 0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08,
            0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00, 0x00, 0x03, 0x01, 0x01, 0x00, 0x18, 0xDD, 0x8D,
            0xB4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
        ],
    )
    .unwrap();
    let path = tmp("pic-then-chart").with_extension("pptx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "presentation": {
                "slides": [{
                    "shapes": [{ "text": "标题" }],
                    "pictures": [{
                        "file": png.to_string_lossy(),
                        "x": 0,
                        "y": 0,
                        "cx": 914400,
                        "cy": 914400
                    }],
                    "charts": [{
                        "type": "col",
                        "series": [{ "categories": ["a", "b"], "values": [1, 2] }]
                    }]
                }]
            }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let tree = inspect_charts(&path);
    assert_eq!(tree["pictureCount"], 1);
    assert_eq!(tree["chartCount"], 1);
    assert_eq!(tree["slides"][0]["charts"][0]["id"], "slide:1/chart:4");
}

#[test]
fn create_rejects_data_from_kind_and_root_charts() {
    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": tmp("data-from").with_extension("pptx").to_string_lossy(),
            "presentation": {
                "slides": [{
                    "charts": [{
                        "type": "col",
                        "from": "A1",
                        "dataFrom": "A10",
                        "series": [{ "values": [1], "categories": ["a"] }]
                    }]
                }]
            }
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": tmp("kind").with_extension("pptx").to_string_lossy(),
            "presentation": {
                "slides": [{
                    "charts": [{
                        "type": "col",
                        "kind": "anchor",
                        "series": [{ "values": [1], "categories": ["a"] }]
                    }]
                }]
            }
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": tmp("root-charts").with_extension("pptx").to_string_lossy(),
            "presentation": {
                "charts": [{ "type": "col", "series": [{ "values": [1] }] }],
                "slides": [{ "shapes": [{ "text": "a" }] }]
            }
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

fn insert_col_deck(tag: &str) -> PathBuf {
    let src = create_deck(tag);
    let inserted = tmp(&format!("{tag}-ins")).with_extension("pptx");
    apply(
        &src,
        &inserted,
        json!([{
            "op": "insertChart",
            "id": "slide:1",
            "type": "col",
            "title": "销量",
            "series": [
                { "name": "系列甲", "categories": ["甲", "乙", "丙"], "values": [1, 2, 3] },
                { "name": "系列乙", "categories": ["甲", "乙", "丙"], "values": [4, 5, 6] }
            ]
        }]),
    )
    .unwrap();
    inserted
}

#[test]
fn trendline_set_delete_and_axis() {
    let src = insert_col_deck("tl");
    let out = tmp("tl-out").with_extension("pptx");
    apply(
        &src,
        &out,
        json!([{
            "op": "insertTrendline",
            "id": "slide:1/chart:3/series:0",
            "type": "linear"
        }]),
    )
    .unwrap();
    let tree = inspect_charts(&out);
    let ser = &tree["slides"][0]["charts"][0]["series"][0];
    assert_eq!(ser["axis"], "primary");
    assert_eq!(ser["trendlines"][0]["type"], "linear");
    let tl_id = ser["trendlines"][0]["id"].as_str().unwrap().to_string();
    let out2 = tmp("tl-set").with_extension("pptx");
    apply(
        &out,
        &out2,
        json!([{ "op": "setTrendline", "id": tl_id, "type": "exp" }]),
    )
    .unwrap();
    let tree2 = inspect_charts(&out2);
    assert_eq!(
        tree2["slides"][0]["charts"][0]["series"][0]["trendlines"][0]["type"],
        "exp"
    );
    let tl_id2 = tree2["slides"][0]["charts"][0]["series"][0]["trendlines"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let out3 = tmp("tl-del").with_extension("pptx");
    apply(
        &out2,
        &out3,
        json!([{ "op": "deleteTrendline", "id": tl_id2 }]),
    )
    .unwrap();
    let tree3 = inspect_charts(&out3);
    assert!(tree3["slides"][0]["charts"][0]["series"][0]
        .get("trendlines")
        .is_none());
    let out4 = tmp("ax").with_extension("pptx");
    apply(
        &src,
        &out4,
        json!([{
            "op": "setChartSeriesAxis",
            "id": "slide:1/chart:3/series:1",
            "axis": "secondary"
        }]),
    )
    .unwrap();
    let tree4 = inspect_charts(&out4);
    assert_eq!(
        tree4["slides"][0]["charts"][0]["series"][1]["axis"],
        "secondary"
    );
    let xml = String::from_utf8(part_bytes(&out4, "ppt/charts/chart1.xml")).unwrap();
    let second = xml.split("<c:barChart>").nth(2).unwrap_or("");
    let wrap_end = second.find("</c:barChart>").unwrap_or(second.len());
    let wrap = &second[..wrap_end];
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
fn insert_stock_surface_ofpie() {
    let src = create_deck("rare");
    let out = tmp("ins-rare").with_extension("pptx");
    apply(
        &src,
        &out,
        json!([
            {
                "op": "insertChart",
                "id": "slide:1",
                "type": "stock",
                "title": "股价",
                "x": 914400,
                "y": 200000,
                "cx": 3657600,
                "cy": 1828800,
                "series": [
                    { "name": "高", "categories": ["甲", "乙", "丙"], "values": [30, 40, 50] },
                    { "name": "低", "categories": ["甲", "乙", "丙"], "values": [10, 15, 20] },
                    { "name": "收", "categories": ["甲", "乙", "丙"], "values": [20, 25, 35] }
                ]
            },
            {
                "op": "insertChart",
                "id": "slide:1",
                "type": "surface",
                "x": 914400,
                "y": 2200000,
                "cx": 3657600,
                "cy": 1828800,
                "series": [
                    { "name": "甲", "categories": ["A", "B"], "values": [1, 2] },
                    { "name": "乙", "categories": ["A", "B"], "values": [3, 3] },
                    { "name": "丙", "categories": ["A", "B"], "values": [2, 4] }
                ]
            },
            {
                "op": "insertChart",
                "id": "slide:1",
                "type": "ofPie",
                "ofPieType": "bar",
                "x": 4700000,
                "y": 200000,
                "cx": 3657600,
                "cy": 1828800,
                "series": [{ "name": "份额", "categories": ["A", "B", "C"], "values": [1, 2, 3] }]
            }
        ]),
    )
    .unwrap();
    let tree = inspect_charts(&out);
    let charts = tree["slides"][0]["charts"].as_array().unwrap();
    let stock = charts.iter().find(|c| c["type"] == "stock").unwrap();
    assert_eq!(stock["series"].as_array().unwrap().len(), 3);
    let xml = String::from_utf8(part_bytes(&out, "ppt/charts/chart1.xml")).unwrap();
    assert!(xml.contains("stockChart"));
    assert!(xml.contains("hiLowLines"));
    let surface = charts.iter().find(|c| c["type"] == "surface").unwrap();
    assert_eq!(surface["series"].as_array().unwrap().len(), 3);
    let sxml = String::from_utf8(part_bytes(&out, "ppt/charts/chart2.xml")).unwrap();
    assert!(sxml.contains("surface3DChart"));
    assert!(sxml.contains("view3D"));
    assert!(sxml.contains("serAx"));
    let ofpie = charts.iter().find(|c| c["type"] == "ofPie").unwrap();
    assert_eq!(ofpie["ofPieType"], "bar");
    let oxml = String::from_utf8(part_bytes(&out, "ppt/charts/chart3.xml")).unwrap();
    assert!(oxml.contains("ofPieChart"));
    assert!(oxml.contains(r#"<c:ofPieType val="bar"/>"#));
    assert!(oxml.contains(r#"gapWidth val="150""#));
    let out2 = tmp("ins-rare-type").with_extension("pptx");
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
    let charts2 = tree2["slides"][0]["charts"].as_array().unwrap();
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
    let src = create_deck("rare-bad");
    let err = apply(
        &src,
        &tmp("rare-stock-2").with_extension("pptx"),
        json!([{
            "op": "insertChart",
            "id": "slide:1",
            "type": "stock",
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
        &tmp("rare-chartex").with_extension("pptx"),
        json!([{
            "op": "insertChart",
            "id": "slide:1",
            "type": "chartex",
            "series": [{ "values": [1, 2] }]
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    let col = insert_col_deck("rare-col");
    let err = apply(
        &col,
        &tmp("rare-to-stock").with_extension("pptx"),
        json!([{
            "op": "setChartType",
            "id": "slide:1/chart:3",
            "type": "stock"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::NotImplemented);
    let err = apply(
        &col,
        &tmp("rare-ofpie-on-col").with_extension("pptx"),
        json!([{
            "op": "setChartType",
            "id": "slide:1/chart:3",
            "type": "col",
            "ofPieType": "pie"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    let stock = tmp("rare-add").with_extension("pptx");
    apply(
        &src,
        &stock,
        json!([{
            "op": "insertChart",
            "id": "slide:1",
            "type": "stock",
            "series": [
                { "name": "高", "categories": ["甲", "乙"], "values": [3, 4] },
                { "name": "低", "categories": ["甲", "乙"], "values": [1, 2] },
                { "name": "收", "categories": ["甲", "乙"], "values": [2, 3] }
            ]
        }]),
    )
    .unwrap();
    let tree = inspect_charts(&stock);
    let stock_id = tree["slides"][0]["charts"][0]["id"].as_str().unwrap();
    let before = std::fs::read(&stock).unwrap();
    let err = apply(
        &stock,
        &tmp("rare-add-out").with_extension("pptx"),
        json!([{
            "op": "addChartSeries",
            "id": stock_id,
            "values": [1, 2]
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::NotImplemented);
    assert_eq!(std::fs::read(&stock).unwrap(), before);
}
