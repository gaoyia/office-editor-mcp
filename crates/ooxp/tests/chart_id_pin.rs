//! 图表系列、趋势线在一次 apply 里按开始时的编号查找。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, RunOutput};
use serde_json::{json, Value};

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp(tag: &str, ext: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-chart-id-pin");
    std::fs::create_dir_all(&dir).unwrap();
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    dir.join(format!("{tag}-{n}.{ext}"))
}

fn apply(src: &Path, out: &Path, ops: Value) {
    run(&json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": out.to_string_lossy(),
        "ops": ops,
    })
    .to_string())
    .unwrap();
}

fn inspect(path: &Path) -> Value {
    let RunOutput::Inspect(tree) = run(&json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": path.to_string_lossy(),
        "scope": { "charts": true }
    })
    .to_string())
    .unwrap() else {
        panic!("inspect");
    };
    tree
}

fn series_of(tree: &Value) -> Vec<(String, String)> {
    let mut out = Vec::new();
    fn walk(v: &Value, out: &mut Vec<(String, String)>) {
        match v {
            Value::Array(items) => {
                for item in items {
                    walk(item, out);
                }
            }
            Value::Object(map) => {
                if let Some(series) = map.get("series").and_then(|s| s.as_array()) {
                    for ser in series {
                        let id = ser.get("id").and_then(|s| s.as_str());
                        let name = ser.get("name").and_then(|s| s.as_str());
                        if let (Some(id), Some(name)) = (id, name) {
                            out.push((id.to_string(), name.to_string()));
                        }
                    }
                }
                for child in map.values() {
                    walk(child, out);
                }
            }
            _ => {}
        }
    }
    walk(tree, &mut out);
    out
}

fn trendline_types(tree: &Value, series_name: &str) -> Vec<String> {
    let mut out = Vec::new();
    fn walk(v: &Value, name: &str, out: &mut Vec<String>) {
        match v {
            Value::Array(items) => {
                for item in items {
                    walk(item, name, out);
                }
            }
            Value::Object(map) => {
                if map.get("name").and_then(|s| s.as_str()) == Some(name) {
                    if let Some(lines) = map.get("trendlines").and_then(|s| s.as_array()) {
                        for line in lines {
                            if let Some(ty) = line.get("type").and_then(|s| s.as_str()) {
                                out.push(ty.to_string());
                            }
                        }
                    }
                }
                for child in map.values() {
                    walk(child, name, out);
                }
            }
            _ => {}
        }
    }
    walk(tree, series_name, &mut out);
    out
}

fn two_series() -> Value {
    json!([
        { "name": "Q1", "categories": ["甲", "乙"], "values": [1, 2] },
        { "name": "Q2", "categories": ["甲", "乙"], "values": [3, 4] }
    ])
}

fn xlsx_with_chart() -> PathBuf {
    let src = tmp("xlsx-src", "xlsx");
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": src.to_string_lossy(),
        "workbook": { "sheets": [{ "name": "Sheet1", "cells": [{ "ref": "A1", "value": 1 }] }] }
    })
    .to_string())
    .unwrap();
    let out = tmp("xlsx-chart", "xlsx");
    apply(
        &src,
        &out,
        json!([{
            "op": "insertChart",
            "id": "sheet:Sheet1",
            "type": "col",
            "from": "E2",
            "dataFrom": "A10",
            "series": two_series()
        }]),
    );
    out
}

fn pptx_with_chart() -> PathBuf {
    let src = tmp("pptx-src", "pptx");
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": src.to_string_lossy(),
        "presentation": { "slides": [{ "shapes": [{ "text": "标题" }] }] }
    })
    .to_string())
    .unwrap();
    let out = tmp("pptx-chart", "pptx");
    apply(
        &src,
        &out,
        json!([{
            "op": "insertChart",
            "id": "slide:1",
            "type": "col",
            "title": "销量",
            "series": two_series()
        }]),
    );
    out
}

fn docx_with_chart() -> PathBuf {
    let out = tmp("docx-chart", "docx");
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": out.to_string_lossy(),
        "document": {
            "paragraphs": [{
                "text": "hello",
                "charts": [{
                    "type": "col",
                    "title": "销量",
                    "series": two_series()
                }]
            }]
        }
    })
    .to_string())
    .unwrap();
    out
}

fn assert_trendline_pin(src: &Path, ext: &str) {
    let series = series_of(&inspect(src));
    let q1 = series
        .iter()
        .find(|(_, n)| n == "Q1")
        .expect("Q1")
        .0
        .clone();
    let lined = tmp("lined", ext);
    apply(
        src,
        &lined,
        json!([
            { "op": "insertTrendline", "id": q1, "type": "linear" },
            { "op": "insertTrendline", "id": q1, "type": "log" }
        ]),
    );
    let series = series_of(&inspect(&lined));
    let q1 = series
        .iter()
        .find(|(_, n)| n == "Q1")
        .expect("Q1")
        .0
        .clone();
    let out = tmp("tl-out", ext);
    apply(
        &lined,
        &out,
        json!([
            { "op": "deleteTrendline", "id": format!("{q1}/trendline:1") },
            { "op": "setTrendline", "id": format!("{q1}/trendline:2"), "type": "exp" }
        ]),
    );
    assert_eq!(
        trendline_types(&inspect(&out), "Q1"),
        vec!["exp".to_string()]
    );
}

fn assert_series_pin(src: &Path, ext: &str) {
    let series = series_of(&inspect(src));
    let q1 = series
        .iter()
        .find(|(_, n)| n == "Q1")
        .expect("Q1")
        .0
        .clone();
    let q2 = series
        .iter()
        .find(|(_, n)| n == "Q2")
        .expect("Q2")
        .0
        .clone();
    let out = tmp("ser-out", ext);
    apply(
        src,
        &out,
        json!([
            { "op": "deleteChartSeries", "id": q1 },
            { "op": "insertTrendline", "id": q2, "type": "linear" }
        ]),
    );
    let tree = inspect(&out);
    let left = series_of(&tree);
    assert!(left.iter().all(|(_, n)| n != "Q1"));
    assert!(left.iter().any(|(_, n)| n == "Q2"));
    assert_eq!(trendline_types(&tree, "Q2"), vec!["linear".to_string()]);
    assert!(trendline_types(&tree, "Q1").is_empty());
}

#[test]
fn xlsx_delete_trendline_then_set_original_second() {
    assert_trendline_pin(&xlsx_with_chart(), "xlsx");
}

#[test]
fn xlsx_delete_series_then_trendline_on_original_second() {
    assert_series_pin(&xlsx_with_chart(), "xlsx");
}

#[test]
fn pptx_delete_trendline_then_set_original_second() {
    assert_trendline_pin(&pptx_with_chart(), "pptx");
}

#[test]
fn pptx_delete_series_then_trendline_on_original_second() {
    assert_series_pin(&pptx_with_chart(), "pptx");
}

#[test]
fn docx_delete_trendline_then_set_original_second() {
    assert_trendline_pin(&docx_with_chart(), "docx");
}

#[test]
fn docx_delete_series_then_trendline_on_original_second() {
    assert_series_pin(&docx_with_chart(), "docx");
}
