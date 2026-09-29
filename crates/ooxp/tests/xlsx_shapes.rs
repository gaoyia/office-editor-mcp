//! 工作表文本框与 SmartArt：名单、插删改、锚点、跨格式拒绝。

use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};

const UID_LIST: &str = "urn:microsoft.com/office/officeart/2005/8/layout/default";

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-xlsx-shapes");
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
            "workbook": {
                "sheets": [{
                    "name": "Sheet1",
                    "cells": [{ "ref": "A1", "value": 1 }]
                }]
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
    v.as_object().unwrap().keys().map(String::as_str).collect()
}

#[test]
fn default_counts_and_empty_lists() {
    let path = create_book("bare");
    let tree = inspect_bare(&path);
    assert_eq!(tree["textBoxCount"], 0);
    assert_eq!(tree["shapeCount"], 0);
    assert_eq!(tree["connectorCount"], 0);
    assert_eq!(tree["smartArtCount"], 0);
    assert_eq!(tree["sheets"][0]["textBoxCount"], 0);
    assert_eq!(tree["sheets"][0]["shapeCount"], 0);
    assert_eq!(tree["sheets"][0]["connectorCount"], 0);
    assert_eq!(tree["sheets"][0]["smartArtCount"], 0);
    assert!(tree["sheets"][0].get("textBoxes").is_none());
    assert!(tree["sheets"][0].get("shapes").is_none());
    assert!(tree["sheets"][0].get("connectors").is_none());
    assert!(tree["sheets"][0].get("smartArts").is_none());

    let listed = inspect_scope(
        &path,
        json!({ "textBoxes": true, "shapes": true, "connectors": true, "smartArt": true }),
    );
    assert_eq!(listed["sheets"][0]["textBoxes"], json!([]));
    assert_eq!(listed["sheets"][0]["shapes"], json!([]));
    assert_eq!(listed["sheets"][0]["connectors"], json!([]));
    assert_eq!(listed["sheets"][0]["smartArts"], json!([]));
}

#[test]
fn insert_textbox_set_text_delete() {
    let src = create_book("box");
    let inserted = tmp("box-ins");
    apply(
        &src,
        &inserted,
        json!([{
            "op": "insertTextBox",
            "id": "sheet:Sheet1",
            "from": "B2",
            "text": "你好"
        }]),
    )
    .unwrap();
    let tree = inspect_scope(&inserted, json!({ "textBoxes": true }));
    assert_eq!(tree["textBoxCount"], 1);
    assert_eq!(tree["sheets"][0]["textBoxCount"], 1);
    let boxes = tree["sheets"][0]["textBoxes"].as_array().unwrap();
    assert_eq!(boxes.len(), 1);
    assert_eq!(
        object_keys(&boxes[0])[..4],
        ["id", "cNvPrId", "name", "anchor"]
    );
    assert_eq!(boxes[0]["anchor"], "oneCell");
    assert_eq!(boxes[0]["from"], "B2");
    assert_eq!(boxes[0]["text"], "你好");
    assert_eq!(boxes[0]["stable"], true);
    let id = boxes[0]["id"].as_str().unwrap().to_string();
    assert!(id.starts_with("sheet:Sheet1/sp:"));

    let patched = tmp("box-text");
    apply(
        &inserted,
        &patched,
        json!([{ "op": "setShapeText", "id": id, "text": "改过" }]),
    )
    .unwrap();
    let after = inspect_scope(&patched, json!({ "textBoxes": true }));
    assert_eq!(after["sheets"][0]["textBoxes"][0]["text"], "改过");

    let gone = tmp("box-del");
    apply(
        &patched,
        &gone,
        json!([{ "op": "deleteTextBox", "id": after["sheets"][0]["textBoxes"][0]["id"] }]),
    )
    .unwrap();
    let empty = inspect_scope(&gone, json!({ "textBoxes": true }));
    assert_eq!(empty["textBoxCount"], 0);
    assert_eq!(empty["sheets"][0]["textBoxes"], json!([]));
}

#[test]
fn insert_smartart_list_and_delete() {
    let src = create_book("art");
    let inserted = tmp("art-ins");
    apply(
        &src,
        &inserted,
        json!([{
            "op": "insertSmartArt",
            "id": "sheet:Sheet1",
            "layout": "list",
            "from": "D4"
        }]),
    )
    .unwrap();
    let tree = inspect_scope(&inserted, json!({ "smartArt": true }));
    assert_eq!(tree["smartArtCount"], 1);
    let arts = tree["sheets"][0]["smartArts"].as_array().unwrap();
    assert_eq!(arts.len(), 1);
    assert_eq!(arts[0]["layout"], UID_LIST);
    assert_eq!(arts[0]["anchor"], "oneCell");
    assert_eq!(arts[0]["from"], "D4");
    let pts = arts[0]["points"].as_array().unwrap();
    assert!(!pts.is_empty());
    let id = arts[0]["id"].as_str().unwrap().to_string();
    assert!(id.starts_with("sheet:Sheet1/smartdiagram:"));

    let gone = tmp("art-del");
    apply(
        &inserted,
        &gone,
        json!([{ "op": "deleteSmartArt", "id": id }]),
    )
    .unwrap();
    let empty = inspect_scope(&gone, json!({ "smartArt": true }));
    assert_eq!(empty["smartArtCount"], 0);
    assert_eq!(empty["sheets"][0]["smartArts"], json!([]));
}

#[test]
fn insert_textbox_requires_from() {
    let src = create_book("nofrom");
    let out = tmp("nofrom-out");
    let err = apply(
        &src,
        &out,
        json!([{ "op": "insertTextBox", "id": "sheet:Sheet1" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());
}

#[test]
fn create_rejects_textboxes_and_smartarts() {
    let path = tmp("create-bad");
    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "workbook": {
                "sheets": [{
                    "name": "Sheet1",
                    "cells": [],
                    "textBoxes": [{ "text": "框" }]
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
            "path": path.to_string_lossy(),
            "workbook": {
                "sheets": [{
                    "name": "Sheet1",
                    "cells": [],
                    "smartArts": [{ "layout": "list" }]
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
            "path": path.to_string_lossy(),
            "workbook": {
                "sheets": [{
                    "name": "Sheet1",
                    "cells": [],
                    "shapes": [{ "prst": "rect" }]
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
            "path": path.to_string_lossy(),
            "workbook": {
                "sheets": [{
                    "name": "Sheet1",
                    "cells": [],
                    "connectors": [{ "prst": "line" }]
                }]
            }
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

#[test]
fn word_box_attrs_and_smartart_pos_rejected() {
    let src = create_book("reject");
    let out = tmp("reject-out");
    let err = apply(
        &src,
        &out,
        json!([{ "op": "setTextBoxWrap", "id": "sheet:Sheet1/sp:2", "wrap": "square" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    let err = apply(
        &src,
        &out,
        json!([{ "op": "setSmartArtPos", "id": "sheet:Sheet1/smartdiagram:3", "x": 0, "y": 0 }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

#[test]
fn set_shape_text_needs_textbox_id() {
    let src = create_book("badid");
    let out = tmp("badid-out");
    let err = apply(
        &src,
        &out,
        json!([{ "op": "setShapeText", "id": "sheet:Sheet1", "text": "x" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
}

#[test]
fn insert_shape_and_connector_roundtrip() {
    let src = create_book("geom");
    let inserted = tmp("geom-ins");
    apply(
        &src,
        &inserted,
        json!([
            {
                "op": "insertShape",
                "id": "sheet:Sheet1",
                "from": "B2",
                "text": "矩形"
            },
            {
                "op": "insertConnector",
                "id": "sheet:Sheet1",
                "from": "D4"
            }
        ]),
    )
    .unwrap();
    let tree = inspect_scope(
        &inserted,
        json!({ "textBoxes": true, "shapes": true, "connectors": true }),
    );
    assert_eq!(tree["shapeCount"], 1);
    assert_eq!(tree["connectorCount"], 1);
    assert_eq!(tree["textBoxCount"], 0);
    assert_eq!(tree["sheets"][0]["textBoxes"], json!([]));
    let shapes = tree["sheets"][0]["shapes"].as_array().unwrap();
    assert_eq!(shapes.len(), 1);
    assert!(shapes[0]["id"]
        .as_str()
        .unwrap()
        .starts_with("sheet:Sheet1/shape:"));
    assert_eq!(shapes[0]["prst"], "rect");
    assert_eq!(shapes[0]["text"], "矩形");
    assert_eq!(shapes[0]["from"], "B2");
    assert_eq!(shapes[0]["stable"], true);
    let cxns = tree["sheets"][0]["connectors"].as_array().unwrap();
    assert_eq!(cxns.len(), 1);
    assert!(cxns[0]["id"]
        .as_str()
        .unwrap()
        .starts_with("sheet:Sheet1/cxn:"));
    assert_eq!(cxns[0]["prst"], "straightConnector1");
    assert!(cxns[0].get("text").is_none());
    let shape_id = shapes[0]["id"].as_str().unwrap().to_string();
    let cxn_id = cxns[0]["id"].as_str().unwrap().to_string();

    let patched = tmp("geom-text");
    apply(
        &inserted,
        &patched,
        json!([
            { "op": "setShapeText", "id": shape_id, "text": "改过" },
            { "op": "setTextBoxSize", "id": shape_id, "cx": 2000000, "cy": 1000000 }
        ]),
    )
    .unwrap();
    let after = inspect_scope(&patched, json!({ "shapes": true, "connectors": true }));
    assert_eq!(after["sheets"][0]["shapes"][0]["text"], "改过");
    assert_eq!(after["sheets"][0]["shapes"][0]["cx"], 2_000_000);
    let shape_id = after["sheets"][0]["shapes"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();

    let gone = tmp("geom-del");
    apply(
        &patched,
        &gone,
        json!([
            { "op": "deleteShape", "id": shape_id },
            { "op": "deleteConnector", "id": cxn_id }
        ]),
    )
    .unwrap();
    let empty = inspect_scope(&gone, json!({ "shapes": true, "connectors": true }));
    assert_eq!(empty["shapeCount"], 0);
    assert_eq!(empty["connectorCount"], 0);
    assert_eq!(empty["sheets"][0]["shapes"], json!([]));
    assert_eq!(empty["sheets"][0]["connectors"], json!([]));
}

#[test]
fn insert_shape_requires_from() {
    let src = create_book("noshape");
    let out = tmp("noshape-out");
    let err = apply(
        &src,
        &out,
        json!([{ "op": "insertShape", "id": "sheet:Sheet1" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());
}

#[test]
fn insert_shape_rejects_xy_and_connector_prst() {
    let src = create_book("badprst");
    let out = tmp("badprst-out");
    let err = apply(
        &src,
        &out,
        json!([{ "op": "insertShape", "id": "sheet:Sheet1", "from": "A1", "x": 0, "y": 0 }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    let err = apply(
        &src,
        &out,
        json!([{
            "op": "insertShape",
            "id": "sheet:Sheet1",
            "from": "A1",
            "prst": "straightConnector1"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

#[test]
fn delete_shape_rejects_textbox_id_and_set_text_rejects_cxn() {
    let src = create_book("badid2");
    let out = tmp("badid2-out");
    let err = apply(
        &src,
        &out,
        json!([{ "op": "deleteShape", "id": "sheet:Sheet1/sp:2" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    let err = apply(
        &src,
        &out,
        json!([{ "op": "setShapeText", "id": "sheet:Sheet1/cxn:3", "text": "x" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    assert!(!out.exists());
}

fn drawing_xml(path: &Path) -> String {
    let file = std::fs::File::open(path).unwrap();
    let mut zip = zip::ZipArchive::new(file).unwrap();
    let mut part = zip.by_name("xl/drawings/drawing1.xml").unwrap();
    let mut s = String::new();
    part.read_to_string(&mut s).unwrap();
    s
}

#[test]
fn group_and_ungroup_keep_child_ids() {
    let src = create_book("grp");
    let inserted = tmp("grp-ins");
    apply(
        &src,
        &inserted,
        json!([
            { "op": "insertShape", "id": "sheet:Sheet1", "from": "B2", "text": "甲" },
            { "op": "insertShape", "id": "sheet:Sheet1", "from": "E4", "text": "乙" }
        ]),
    )
    .unwrap();
    let tree = inspect_scope(&inserted, json!({ "shapes": true }));
    let a = tree["sheets"][0]["shapes"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let b = tree["sheets"][0]["shapes"][1]["id"]
        .as_str()
        .unwrap()
        .to_string();

    let grouped = tmp("grp-out");
    apply(
        &inserted,
        &grouped,
        json!([{ "op": "groupShapes", "id": "sheet:Sheet1", "ids": [a, b] }]),
    )
    .unwrap();
    let after = inspect_scope(&grouped, json!({ "shapes": true }));
    let ids: Vec<_> = after["sheets"][0]["shapes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["id"].as_str().unwrap().to_string())
        .collect();
    assert!(ids.contains(&a));
    assert!(ids.contains(&b));
    let xml = drawing_xml(&grouped);
    assert!(xml.contains("<xdr:grpSp"));
    assert!(xml.contains("组合 "));
    assert!(!xml.contains("editAs"));
    assert!(!xml.contains("xdr:nvPr"));

    let dest = tmp("grp-again");
    let err = apply(
        &grouped,
        &dest,
        json!([{ "op": "groupShapes", "id": "sheet:Sheet1", "ids": [a, b] }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!dest.exists());

    let opened = tmp("ungroup-out");
    apply(&grouped, &opened, json!([{ "op": "ungroup", "id": a }])).unwrap();
    let xml2 = drawing_xml(&opened);
    assert!(!xml2.contains("<xdr:grpSp"));
    let tree2 = inspect_scope(&opened, json!({ "shapes": true }));
    let ids2: Vec<_> = tree2["sheets"][0]["shapes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["id"].as_str().unwrap().to_string())
        .collect();
    assert!(ids2.contains(&a));
    assert!(ids2.contains(&b));
}

#[test]
fn group_shape_with_connector_and_rejects() {
    let src = create_book("grpcxn");
    let inserted = tmp("grpcxn-ins");
    apply(
        &src,
        &inserted,
        json!([
            { "op": "insertShape", "id": "sheet:Sheet1", "from": "B2" },
            { "op": "insertConnector", "id": "sheet:Sheet1", "from": "D4" }
        ]),
    )
    .unwrap();
    let tree = inspect_scope(&inserted, json!({ "shapes": true, "connectors": true }));
    let sid = tree["sheets"][0]["shapes"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let cid = tree["sheets"][0]["connectors"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let grouped = tmp("grpcxn-out");
    apply(
        &inserted,
        &grouped,
        json!([{ "op": "groupShapes", "id": "sheet:Sheet1", "ids": [sid, cid] }]),
    )
    .unwrap();
    let xml = drawing_xml(&grouped);
    assert!(xml.contains("<xdr:grpSp"));
    assert!(xml.contains("<xdr:cxnSp"));
    let listed = inspect_scope(&grouped, json!({ "shapes": true, "connectors": true }));
    assert_eq!(listed["shapeCount"], 1);
    assert_eq!(listed["connectorCount"], 1);

    let dest = tmp("grp-one");
    let err = apply(
        &inserted,
        &dest,
        json!([{ "op": "groupShapes", "id": "sheet:Sheet1", "ids": [sid] }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!dest.exists());

    let dest = tmp("grp-slide");
    let err = apply(
        &inserted,
        &dest,
        json!([{
            "op": "groupShapes",
            "id": "slide:1",
            "ids": [sid, cid]
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    assert!(!dest.exists());

    let dest = tmp("ungroup-bare");
    let err = apply(&inserted, &dest, json!([{ "op": "ungroup", "id": sid }])).unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!dest.exists());
}
