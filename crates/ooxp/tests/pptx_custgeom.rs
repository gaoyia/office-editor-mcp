//! 自定义几何：custGeom 路径与连接线绑点。

use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::ZipArchive;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-pptx-custgeom");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn tmp(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}.pptx"))
}

fn create_deck(tag: &str) -> PathBuf {
    let path = tmp(tag);
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "presentation": {
                "slides": [
                    { "shapes": [{ "text": "甲" }, { "text": "乙" }] }
                ]
            }
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

fn inspect_scope(path: &Path, scope: Value) -> Value {
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

fn object_keys(v: &Value) -> Vec<&str> {
    v.as_object().unwrap().keys().map(|s| s.as_str()).collect()
}

fn part(path: &Path, name: &str) -> String {
    let bytes = std::fs::read(path).unwrap();
    let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut file = zip.by_name(name).unwrap();
    let mut out = Vec::new();
    file.read_to_end(&mut out).unwrap();
    String::from_utf8(out).unwrap()
}

fn sample_paths() -> Value {
    json!([{
        "w": 1905001,
        "h": 1270001,
        "ops": [
            { "type": "moveTo", "x": 0, "y": 0 },
            { "type": "lnTo", "x": 1905000, "y": 0 },
            { "type": "lnTo", "x": 1905000, "y": 1270000 },
            { "type": "lnTo", "x": 0, "y": 1270000 }
        ]
    }])
}

#[test]
fn insert_shape_geom_lists_custgeom() {
    let src = create_deck("ins");
    let out = tmp("ins-out");
    apply(
        &src,
        &out,
        json!([{
            "op": "insertShape",
            "id": "slide:1",
            "geom": { "paths": sample_paths() },
            "x": 914400,
            "y": 914400,
            "cx": 1905001,
            "cy": 1270001
        }]),
    )
    .unwrap();
    let tree = inspect_scope(&out, json!({ "slide": 1 }));
    let neu = tree["slides"][0]["shapes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s.get("geom").is_some())
        .unwrap();
    assert_eq!(neu["prst"], Value::Null);
    assert_eq!(neu["geom"]["paths"][0]["w"], 1905001);
    assert_eq!(neu["geom"]["paths"][0]["ops"][0]["type"], "moveTo");
    assert_eq!(neu["geom"]["paths"][0]["ops"][1]["type"], "lnTo");
    assert_eq!(neu["geom"]["paths"][0]["ops"][1]["x"], 1905000);
    let keys = object_keys(neu);
    let prst = keys.iter().position(|k| *k == "prst").unwrap();
    let geom = keys.iter().position(|k| *k == "geom").unwrap();
    let x = keys.iter().position(|k| *k == "x").unwrap();
    assert!(prst < geom && geom < x);
    let old = &tree["slides"][0]["shapes"][0];
    assert!(old.get("geom").is_none());
    let xml = part(&out, "ppt/slides/slide1.xml");
    assert!(xml.contains("<a:custGeom>"));
    assert!(xml.contains("<a:avLst/>"));
    assert!(xml.contains("<a:gdLst/>"));
    assert!(xml.contains("<a:ahLst/>"));
    assert!(xml.contains("<a:cxnLst/>"));
    assert!(xml.contains("l=\"0\""));
    assert!(xml.contains("<a:moveTo>"));
    assert_eq!(xml.matches("txBox=\"1\"").count(), 2);
}

#[test]
fn set_shape_geom_replaces_prst() {
    let src = create_deck("set");
    let out = tmp("set-out");
    apply(
        &src,
        &out,
        json!([{
            "op": "setShapeGeom",
            "id": "slide:1/shape:2",
            "paths": sample_paths()
        }]),
    )
    .unwrap();
    let tree = inspect_scope(&out, json!({ "slide": 1 }));
    let sh = &tree["slides"][0]["shapes"][0];
    assert_eq!(sh["id"], "slide:1/shape:2");
    assert_eq!(sh["prst"], Value::Null);
    assert_eq!(sh["geom"]["paths"][0]["h"], 1270001);
    let xml = part(&out, "ppt/slides/slide1.xml");
    assert!(xml.contains("<a:custGeom>"));
    assert!(!xml.contains("prst=\"rect\"") || xml.matches("prst=\"rect\"").count() == 1);
}

#[test]
fn insert_connector_writes_st_cxn() {
    let src = create_deck("cxn");
    let out = tmp("cxn-out");
    apply(
        &src,
        &out,
        json!([{
            "op": "insertConnector",
            "id": "slide:1",
            "x": 635000,
            "y": 4064000,
            "start": { "id": "slide:1/shape:2", "site": 1 },
            "end": { "id": "slide:1/shape:3", "site": 1 }
        }]),
    )
    .unwrap();
    let tree = inspect_scope(&out, json!({ "connectors": true, "slide": 1 }));
    let cxn = &tree["slides"][0]["connectors"][0];
    assert_eq!(cxn["start"]["id"], "slide:1/shape:2");
    assert_eq!(cxn["start"]["site"], 1);
    assert_eq!(cxn["end"]["id"], "slide:1/shape:3");
    assert_eq!(
        object_keys(cxn),
        ["id", "cNvPrId", "name", "prst", "start", "end", "x", "y", "cx", "cy", "stable"]
    );
    let xml = part(&out, "ppt/slides/slide1.xml");
    assert!(xml.contains("<a:stCxn id=\"2\" idx=\"0\"/>"));
    assert!(xml.contains("<a:endCxn id=\"3\" idx=\"0\"/>"));
}

#[test]
fn set_connector_changes_end() {
    let src = create_deck("setc");
    let mid = tmp("setc-mid");
    apply(
        &src,
        &mid,
        json!([{
            "op": "insertConnector",
            "id": "slide:1",
            "start": { "id": "slide:1/shape:2" },
            "end": { "id": "slide:1/shape:3" }
        }]),
    )
    .unwrap();
    let tree = inspect_scope(&mid, json!({ "connectors": true }));
    let cxn_id = tree["slides"][0]["connectors"][0]["id"].as_str().unwrap();
    let out = tmp("setc-out");
    apply(
        &mid,
        &out,
        json!([{
            "op": "setConnector",
            "id": cxn_id,
            "end": { "id": "slide:1/shape:2", "site": 2 }
        }]),
    )
    .unwrap();
    let xml = part(&out, "ppt/slides/slide1.xml");
    assert!(xml.contains("<a:stCxn id=\"2\" idx=\"0\"/>"));
    assert!(xml.contains("<a:endCxn id=\"2\" idx=\"1\"/>"));
    let tree2 = inspect_scope(&out, json!({ "connectors": true }));
    assert_eq!(tree2["slides"][0]["connectors"][0]["end"]["site"], 2);
}

#[test]
fn rejects_bad_geom_and_wrong_format() {
    let src = create_deck("rej");
    let dest = tmp("rej-out");
    let err = apply(
        &src,
        &dest,
        json!([{
            "op": "insertShape",
            "id": "slide:1",
            "prst": "rect",
            "geom": { "paths": sample_paths() }
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!dest.exists());

    let err = apply(
        &src,
        &dest,
        json!([{
            "op": "setShapeGeom",
            "id": "slide:1/shape:2",
            "paths": [{
                "w": 1,
                "h": 1,
                "ops": [{ "type": "cubicBezTo", "x": 0, "y": 0 }]
            }]
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let err = apply(
        &src,
        &dest,
        json!([{
            "op": "setShapeGeom",
            "id": "slide:1/cxn:4",
            "paths": sample_paths()
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);

    let xlsx = tmp("xlsx").with_extension("xlsx");
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
    let xout = tmp("xlsx-out").with_extension("xlsx");
    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": xlsx.to_string_lossy(),
            "output": xout.to_string_lossy(),
            "ops": [{ "op": "setShapeGeom", "id": "slide:1/shape:2", "paths": sample_paths() }]
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}
