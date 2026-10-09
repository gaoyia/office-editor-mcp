//! 形状几何剩余：非矩形 prst、连接线、组合。

use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::ZipArchive;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-pptx-shape-geom");
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
                    { "shapes": [{ "text": "标题" }, { "text": "第二框" }] },
                    { "shapes": [{ "text": "第二页" }] }
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

fn has_group_el(xml: &str) -> bool {
    xml.contains("<p:grpSp>") || xml.contains("<p:grpSp ")
}

#[test]
fn insert_ellipse_has_prst_and_fill() {
    let src = create_deck("ell");
    let out = tmp("ell-out");
    apply(
        &src,
        &out,
        json!([{
            "op": "insertShape",
            "id": "slide:1",
            "prst": "ellipse",
            "text": "椭圆",
            "x": 914400,
            "y": 1371600,
            "cx": 1828800,
            "cy": 914400
        }]),
    )
    .unwrap();
    let tree = inspect_scope(&out, json!({ "slide": 1 }));
    assert_eq!(tree["connectorCount"], 0);
    assert!(tree["slides"][0].get("connectors").is_none());
    let neu = tree["slides"][0]["shapes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["text"] == "椭圆")
        .unwrap();
    assert_eq!(neu["id"], "slide:1/shape:4");
    assert_eq!(neu["prst"], "ellipse");
    assert_eq!(neu["stable"], true);
    assert_eq!(
        object_keys(neu),
        [
            "id",
            "shapeId",
            "name",
            "prst",
            "x",
            "y",
            "cx",
            "cy",
            "text",
            "sz",
            "color",
            "bold",
            "italic",
            "underline",
            "font",
            "eastAsiaFont",
            "align",
            "textAnchor",
            "stable"
        ]
    );
    let xml = part(&out, "ppt/slides/slide1.xml");
    assert!(xml.contains("prst=\"ellipse\""));
    assert_eq!(xml.matches("txBox=\"1\"").count(), 2);
    assert!(xml.contains("5B9BD5"));
    assert!(xml.contains("2E75B6"));
    assert!(xml.contains("形状 4"));
    let old = &tree["slides"][0]["shapes"][0];
    assert_eq!(old["prst"], "rect");
}

#[test]
fn insert_connector_lists_and_mutates() {
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
            "cx": 1828800,
            "cy": 762000
        }]),
    )
    .unwrap();
    let bare = inspect_scope(&out, json!({}));
    assert_eq!(bare["connectorCount"], 1);
    assert_eq!(bare["slides"][0]["connectorCount"], 1);
    assert!(bare["slides"][0].get("connectors").is_none());

    let tree = inspect_scope(&out, json!({ "connectors": true, "slide": 1 }));
    let list = tree["slides"][0]["connectors"].as_array().unwrap();
    assert_eq!(list.len(), 1);
    let cxn = &list[0];
    assert_eq!(cxn["id"], "slide:1/cxn:4");
    assert_eq!(cxn["cNvPrId"], "4");
    assert_eq!(cxn["prst"], "straightConnector1");
    assert_eq!(cxn["stable"], true);
    assert_eq!(cxn["x"], 635000);
    assert_eq!(cxn["y"], 4064000);
    assert_eq!(
        object_keys(cxn),
        ["id", "cNvPrId", "name", "prst", "x", "y", "cx", "cy", "stable"]
    );
    let xml = part(&out, "ppt/slides/slide1.xml");
    assert!(xml.contains("<p:cxnSp"));
    assert!(xml.contains("prst=\"straightConnector1\""));
    assert!(xml.contains("连接线 4"));
    let start = xml.find("<p:cxnSp").expect("有连接线");
    let end = start + xml[start..].find("</p:cxnSp>").expect("连接线闭合");
    assert!(!xml[start..end].contains("<p:txBody"));

    let moved = tmp("cxn-pos");
    apply(
        &out,
        &moved,
        json!([
            { "op": "setShapePos", "id": "slide:1/cxn:4", "x": 100000, "y": 200000 },
            { "op": "setShapeSize", "id": "slide:1/cxn:4", "cx": 300000, "cy": 400000 },
            { "op": "setShapeLine", "id": "slide:1/cxn:4", "color": "FF0000", "width": 25400 }
        ]),
    )
    .unwrap();
    let after = inspect_scope(&moved, json!({ "connectors": true }));
    let c = &after["slides"][0]["connectors"][0];
    assert_eq!(c["x"], 100000);
    assert_eq!(c["y"], 200000);
    assert_eq!(c["cx"], 300000);
    assert_eq!(c["cy"], 400000);
    let xml = part(&moved, "ppt/slides/slide1.xml");
    assert!(xml.contains("FF0000"));
    assert!(xml.contains("w=\"25400\""));

    let dest = tmp("cxn-text");
    let err = apply(
        &out,
        &dest,
        json!([{ "op": "setShapeText", "id": "slide:1/cxn:4", "text": "x" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    assert!(!dest.exists());

    let dest = tmp("cxn-fill");
    let err = apply(
        &out,
        &dest,
        json!([{ "op": "setShapeFill", "id": "slide:1/cxn:4", "color": "FF0000" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    assert!(!dest.exists());

    let dest = tmp("cxn-delshape");
    let err = apply(
        &out,
        &dest,
        json!([{ "op": "deleteShape", "id": "slide:1/cxn:4" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    assert!(!dest.exists());

    let gone = tmp("cxn-del");
    apply(
        &out,
        &gone,
        json!([{ "op": "deleteConnector", "id": "slide:1/cxn:4" }]),
    )
    .unwrap();
    let tree = inspect_scope(&gone, json!({ "connectors": true }));
    assert_eq!(tree["connectorCount"], 0);
    assert_eq!(tree["slides"][0]["connectors"].as_array().unwrap().len(), 0);
    assert!(!part(&gone, "ppt/slides/slide1.xml").contains("<p:cxnSp"));
}

#[test]
fn insert_anim_binds_connector() {
    let src = create_deck("anim-cxn");
    let mid = tmp("cxn-mid");
    apply(
        &src,
        &mid,
        json!([{ "op": "insertConnector", "id": "slide:1" }]),
    )
    .unwrap();
    let out = tmp("anim-out");
    apply(
        &mid,
        &out,
        json!([{ "op": "insertAnim", "id": "slide:1/cxn:4", "type": "fade" }]),
    )
    .unwrap();
    let tree = inspect_scope(&out, json!({ "timing": true }));
    assert_eq!(tree["slides"][0]["timing"][0]["target"], "slide:1/cxn:4");
    let xml = part(&out, "ppt/slides/slide1.xml");
    assert!(xml.contains("spid=\"4\""));
}

#[test]
fn group_and_ungroup_keep_child_ids() {
    let src = create_deck("grp");
    let grouped = tmp("grp-out");
    apply(
        &src,
        &grouped,
        json!([{
            "op": "groupShapes",
            "id": "slide:1",
            "ids": ["slide:1/shape:2", "slide:1/shape:3"]
        }]),
    )
    .unwrap();
    let tree = inspect_scope(&grouped, json!({ "slide": 1 }));
    let ids: Vec<_> = tree["slides"][0]["shapes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["id"].as_str().unwrap().to_string())
        .collect();
    assert!(ids.contains(&"slide:1/shape:2".to_string()));
    assert!(ids.contains(&"slide:1/shape:3".to_string()));
    let xml = part(&grouped, "ppt/slides/slide1.xml");
    assert!(has_group_el(&xml));
    assert!(xml.contains("组合 4"));
    assert!(xml.contains("id=\"2\""));
    assert!(xml.contains("id=\"3\""));

    let dest = tmp("grp-again");
    let err = apply(
        &grouped,
        &dest,
        json!([{
            "op": "groupShapes",
            "id": "slide:1",
            "ids": ["slide:1/shape:2", "slide:1/shape:3"]
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!dest.exists());

    let opened = tmp("ungroup-out");
    apply(
        &grouped,
        &opened,
        json!([{ "op": "ungroup", "id": "slide:1/shape:2" }]),
    )
    .unwrap();
    let xml = part(&opened, "ppt/slides/slide1.xml");
    assert!(!has_group_el(&xml), "{xml}");
    assert!(!xml.contains("组合 4"));
    let tree = inspect_scope(&opened, json!({ "slide": 1 }));
    let ids: Vec<_> = tree["slides"][0]["shapes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["id"].as_str().unwrap().to_string())
        .collect();
    assert!(ids.contains(&"slide:1/shape:2".to_string()));
    assert!(ids.contains(&"slide:1/shape:3".to_string()));
}

#[test]
fn rejects_illegal_prst_group_and_ids() {
    let src = create_deck("rej");

    let dest = tmp("prst-cxn");
    let err = apply(
        &src,
        &dest,
        json!([{ "op": "insertShape", "id": "slide:1", "prst": "straightConnector1" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!dest.exists());

    let dest = tmp("grp-one");
    let err = apply(
        &src,
        &dest,
        json!([{
            "op": "groupShapes",
            "id": "slide:1",
            "ids": ["slide:1/shape:2"]
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!dest.exists());

    let dest = tmp("ins-cxn-shape");
    let err = apply(
        &src,
        &dest,
        json!([{ "op": "insertConnector", "id": "slide:1/shape:2" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    assert!(!dest.exists());

    let dest = tmp("ungroup-bare");
    let err = apply(
        &src,
        &dest,
        json!([{ "op": "ungroup", "id": "slide:1/shape:2" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!dest.exists());

    let dest = tmp("del-missing");
    let err = apply(
        &src,
        &dest,
        json!([{ "op": "deleteConnector", "id": "slide:1/cxn:9" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::IdNotFound);
    assert!(!dest.exists());
}

#[test]
fn xlsx_and_docx_reject_pptx_connector_ids() {
    let xlsx = tmp("scope").with_extension("xlsx");
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
    let tree = inspect_scope(&xlsx, json!({ "connectors": true }));
    assert_eq!(tree["connectorCount"], 0);
    assert_eq!(tree["sheets"][0]["connectors"], json!([]));
    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": xlsx.to_string_lossy(),
            "output": tmp("xlsx-out").with_extension("xlsx").to_string_lossy(),
            "ops": [{ "op": "insertConnector", "id": "slide:1" }]
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);

    let docx = tmp("scope").with_extension("docx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": docx.to_string_lossy(),
            "document": { "paragraphs": [{ "text": "甲" }] }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let tree = inspect_scope(&docx, json!({ "connectors": true }));
    assert_eq!(tree["connectorCount"], 0);
    assert!(tree.get("connectors").is_none());
    let dest = tmp("docx-out").with_extension("docx");
    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": docx.to_string_lossy(),
            "output": dest.to_string_lossy(),
            "ops": [{ "op": "insertConnector", "id": "slide:1" }]
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": docx.to_string_lossy(),
            "output": dest.to_string_lossy(),
            "ops": [{ "op": "groupShapes", "id": "slide:1", "ids": ["slide:1/shape:2", "slide:1/shape:3"] }]
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    assert!(!dest.exists());
}
