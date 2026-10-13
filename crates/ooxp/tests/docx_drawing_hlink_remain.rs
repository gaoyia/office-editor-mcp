//! docx DrawingML 链剩余：框内图、形状、连接线、图表、SmartArt、墨迹、3D、注内图、OLE 拒绝。

use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, sample_glb, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::ZipArchive;

const PNG_1X1: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
    0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08, 0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00,
    0x00, 0x03, 0x01, 0x01, 0x00, 0x18, 0xDD, 0x8D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E,
    0x44, 0xAE, 0x42, 0x60, 0x82,
];

const INKML: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><ink xmlns="http://www.w3.org/2003/InkML"><trace>0 0, 10 10</trace></ink>"#;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-docx-drawing-hlink-remain");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn tmp(tag: &str, ext: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}.{ext}"))
}

fn write_png(tag: &str) -> PathBuf {
    let path = tmp(tag, "png");
    std::fs::write(&path, PNG_1X1).unwrap();
    path
}

fn write_ink(tag: &str) -> PathBuf {
    let path = tmp(tag, "xml");
    std::fs::write(&path, INKML.as_bytes()).unwrap();
    path
}

fn write_glb(tag: &str) -> PathBuf {
    let path = tmp(tag, "glb");
    std::fs::write(&path, sample_glb()).unwrap();
    path
}

fn write_xlsx(tag: &str) -> PathBuf {
    let path = tmp(tag, "xlsx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "workbook": {
                "sheets": [{
                    "name": "Sheet1",
                    "cells": [{ "ref": "A1", "value": "嵌入" }]
                }]
            }
        })
        .to_string(),
        None,
    )
    .unwrap();
    path
}

fn create_doc(tag: &str, document: Value) -> PathBuf {
    let path = tmp(tag, "docx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "format": "docx",
            "document": document
        })
        .to_string(),
        None,
    )
    .unwrap();
    path
}

fn apply(src: &Path, out: &Path, ops: Value) {
    run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": src.to_string_lossy(),
            "output": out.to_string_lossy(),
            "ops": ops
        })
        .to_string(),
        None,
    )
    .unwrap();
}

fn apply_err(src: &Path, ops: Value) -> ooxp::Error {
    let out = tmp("err", "docx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": src.to_string_lossy(),
            "output": out.to_string_lossy(),
            "ops": ops
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
    let mut zip = ZipArchive::new(f).unwrap();
    let mut zf = zip.by_name(name).unwrap();
    let mut out = String::new();
    zf.read_to_string(&mut out).unwrap();
    out
}

fn host_docx() -> PathBuf {
    create_doc("host", json!({ "paragraphs": [{ "text": "段" }] }))
}

fn para_id(path: &Path) -> String {
    inspect(path, json!({ "from": 1, "maxParas": 8 }))["paragraphs"][0]["id"]
        .as_str()
        .unwrap()
        .to_string()
}

#[test]
fn inner_picture_drawing_hyperlink() {
    let png = write_png("inner");
    let src = create_doc(
        "box",
        json!({
            "paragraphs": [{
                "text": "外",
                "textBoxes": [{ "text": "框" }]
            }]
        }),
    );
    let inner_para = inspect(
        &src,
        json!({
            "from": 1,
            "maxParas": 8,
            "textBoxes": true,
            "textBoxContent": true
        }),
    )["paragraphs"][0]["textBoxes"][0]["paragraphs"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let with_pic = tmp("inner-pic", "docx");
    apply(
        &src,
        &with_pic,
        json!([{
            "op": "insertPicture",
            "id": inner_para,
            "file": png.to_string_lossy()
        }]),
    );
    let pic_id = inspect(
        &with_pic,
        json!({
            "from": 1,
            "maxParas": 8,
            "pictures": true,
            "textBoxes": true,
            "textBoxContent": true
        }),
    )["paragraphs"][0]["textBoxes"][0]["paragraphs"][0]["pictures"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let out = tmp("inner-hlink", "docx");
    apply(
        &with_pic,
        &out,
        json!([{
            "op": "insertHyperlink",
            "id": pic_id,
            "url": "https://example.com/inner"
        }]),
    );
    let tree = inspect(
        &out,
        json!({
            "from": 1,
            "maxParas": 8,
            "pictures": true,
            "textBoxes": true,
            "textBoxContent": true,
            "hyperlinks": true
        }),
    );
    assert_eq!(tree["hyperlinkCount"], 0);
    assert_eq!(
        tree["paragraphs"][0]["textBoxes"][0]["paragraphs"][0]["pictures"][0]["hyperlink"]["url"],
        "https://example.com/inner"
    );
    assert!(part_text(&out, "word/document.xml").contains("hlinkClick"));
}

#[test]
fn shape_and_connector_drawing_hyperlink_insert_set_delete() {
    let src = host_docx();
    let para = para_id(&src);
    let with_objs = tmp("shape-cxn", "docx");
    apply(
        &src,
        &with_objs,
        json!([
            { "op": "insertShape", "id": para, "prst": "ellipse" },
            { "op": "insertConnector", "id": para }
        ]),
    );
    let tree0 = inspect(
        &with_objs,
        json!({ "from": 1, "maxParas": 8, "shapes": true, "connectors": true }),
    );
    assert!(tree0["paragraphs"][0]["shapes"][0]["hyperlink"].is_null());
    assert!(tree0["paragraphs"][0]["connectors"][0]["hyperlink"].is_null());
    let sid = tree0["paragraphs"][0]["shapes"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let cid = tree0["paragraphs"][0]["connectors"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();

    let linked = tmp("shape-link", "docx");
    apply(
        &with_objs,
        &linked,
        json!([
            { "op": "insertHyperlink", "id": sid, "url": "https://example.com/shape" },
            { "op": "insertHyperlink", "id": cid, "url": "https://example.com/cxn" }
        ]),
    );
    let tree = inspect(
        &linked,
        json!({
            "from": 1,
            "maxParas": 8,
            "shapes": true,
            "connectors": true,
            "hyperlinks": true
        }),
    );
    assert_eq!(tree["hyperlinkCount"], 0);
    assert_eq!(
        tree["paragraphs"][0]["shapes"][0]["hyperlink"]["url"],
        "https://example.com/shape"
    );
    assert_eq!(
        tree["paragraphs"][0]["connectors"][0]["hyperlink"]["url"],
        "https://example.com/cxn"
    );

    let set_out = tmp("shape-set", "docx");
    apply(
        &linked,
        &set_out,
        json!([{
            "op": "setHyperlink",
            "id": sid,
            "url": "https://example.org/shape2"
        }]),
    );
    let tree_set = inspect(
        &set_out,
        json!({ "from": 1, "maxParas": 8, "shapes": true }),
    );
    assert_eq!(
        tree_set["paragraphs"][0]["shapes"][0]["hyperlink"]["url"],
        "https://example.org/shape2"
    );

    let del = tmp("shape-del", "docx");
    apply(
        &set_out,
        &del,
        json!([{ "op": "deleteHyperlink", "id": sid }]),
    );
    let tree_del = inspect(
        &del,
        json!({ "from": 1, "maxParas": 8, "shapes": true, "connectors": true }),
    );
    assert_eq!(tree_del["shapeCount"], 1);
    assert!(tree_del["paragraphs"][0]["shapes"][0]["hyperlink"].is_null());
    assert_eq!(
        tree_del["paragraphs"][0]["connectors"][0]["hyperlink"]["url"],
        "https://example.com/cxn"
    );
}

#[test]
fn create_shape_hyperlink() {
    let src = create_doc(
        "create-shape",
        json!({
            "paragraphs": [{
                "text": "形",
                "shapes": [{
                    "prst": "ellipse",
                    "hyperlink": { "url": "https://example.com/create-shape" }
                }]
            }]
        }),
    );
    let tree = inspect(
        &src,
        json!({ "from": 1, "maxParas": 8, "shapes": true, "hyperlinks": true }),
    );
    assert_eq!(tree["hyperlinkCount"], 0);
    assert_eq!(
        tree["paragraphs"][0]["shapes"][0]["hyperlink"]["url"],
        "https://example.com/create-shape"
    );
}

#[test]
fn chart_smartart_ink_model3d_drawing_hyperlink() {
    let src = host_docx();
    let para = para_id(&src);
    let ink = write_ink("ink");
    let glb = write_glb("m3d");
    let with_objs = tmp("misc", "docx");
    apply(
        &src,
        &with_objs,
        json!([
            {
                "op": "insertChart",
                "id": para,
                "type": "col",
                "series": [{
                    "name": "系列",
                    "categories": ["A"],
                    "values": [1]
                }]
            },
            {
                "op": "insertSmartArt",
                "id": para,
                "layout": "list",
                "points": [{ "text": "一" }]
            },
            {
                "op": "insertInkCanvas",
                "id": para,
                "file": ink.to_string_lossy()
            },
            {
                "op": "insertModel3d",
                "id": para,
                "file": glb.to_string_lossy()
            }
        ]),
    );
    let tree0 = inspect(
        &with_objs,
        json!({
            "from": 1,
            "maxParas": 8,
            "charts": true,
            "smartArt": true,
            "inkCanvases": true,
            "model3ds": true
        }),
    );
    let chart_id = tree0["charts"][0]["id"].as_str().unwrap().to_string();
    let sa_id = tree0["smartArts"][0]["id"].as_str().unwrap().to_string();
    let ink_id = tree0["paragraphs"][0]["inkCanvases"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let m3d_id = tree0["paragraphs"][0]["model3ds"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(tree0["charts"][0]["hyperlink"].is_null());
    assert!(tree0["smartArts"][0]["hyperlink"].is_null());
    assert!(tree0["paragraphs"][0]["inkCanvases"][0]["hyperlink"].is_null());
    assert!(tree0["paragraphs"][0]["model3ds"][0]["hyperlink"].is_null());

    let linked = tmp("misc-link", "docx");
    apply(
        &with_objs,
        &linked,
        json!([
            { "op": "insertHyperlink", "id": chart_id, "url": "https://example.com/chart" },
            { "op": "insertHyperlink", "id": sa_id, "url": "https://example.com/smartart" },
            { "op": "insertHyperlink", "id": ink_id, "url": "https://example.com/ink" },
            { "op": "insertHyperlink", "id": m3d_id, "url": "https://example.com/model3d" }
        ]),
    );
    let tree = inspect(
        &linked,
        json!({
            "from": 1,
            "maxParas": 8,
            "charts": true,
            "smartArt": true,
            "inkCanvases": true,
            "model3ds": true,
            "hyperlinks": true
        }),
    );
    assert_eq!(tree["hyperlinkCount"], 0);
    assert_eq!(
        tree["charts"][0]["hyperlink"]["url"],
        "https://example.com/chart"
    );
    assert_eq!(
        tree["smartArts"][0]["hyperlink"]["url"],
        "https://example.com/smartart"
    );
    assert_eq!(
        tree["paragraphs"][0]["inkCanvases"][0]["hyperlink"]["url"],
        "https://example.com/ink"
    );
    assert_eq!(
        tree["paragraphs"][0]["model3ds"][0]["hyperlink"]["url"],
        "https://example.com/model3d"
    );
}

#[test]
fn note_picture_drawing_hyperlink() {
    let png = write_png("note");
    let src = host_docx();
    let para = para_id(&src);
    let with_note = tmp("note", "docx");
    apply(
        &src,
        &with_note,
        json!([{ "op": "insertFootnote", "id": para, "text": "脚注" }]),
    );
    let note_para = inspect(
        &with_note,
        json!({ "from": 1, "maxParas": 8, "notes": true }),
    )["footnotes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["type"] == "normal")
        .unwrap()["paragraphs"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let with_pic = tmp("note-pic", "docx");
    apply(
        &with_note,
        &with_pic,
        json!([{
            "op": "insertPicture",
            "id": note_para,
            "file": png.to_string_lossy()
        }]),
    );
    let pic_id = inspect(
        &with_pic,
        json!({ "from": 1, "maxParas": 8, "notes": true, "pictures": true }),
    )["footnotes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["pictures"].as_array().is_some_and(|a| !a.is_empty()))
        .unwrap()["pictures"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let out = tmp("note-hlink", "docx");
    apply(
        &with_pic,
        &out,
        json!([{
            "op": "insertHyperlink",
            "id": pic_id,
            "url": "https://example.com/note"
        }]),
    );
    let tree = inspect(
        &out,
        json!({
            "from": 1,
            "maxParas": 8,
            "notes": true,
            "pictures": true,
            "hyperlinks": true
        }),
    );
    assert_eq!(tree["hyperlinkCount"], 0);
    let note = tree["footnotes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["pictures"].as_array().is_some_and(|a| !a.is_empty()))
        .unwrap();
    assert_eq!(
        note["pictures"][0]["hyperlink"]["url"],
        "https://example.com/note"
    );
    assert!(part_text(&out, "word/footnotes.xml").contains("hlinkClick"));
    assert!(part_text(&out, "word/_rels/footnotes.xml.rels").contains("/hyperlink"));
}

#[test]
fn ole_insert_hyperlink_is_invalid_id() {
    let src = host_docx();
    let para = para_id(&src);
    let xlsx = write_xlsx("ole");
    let with_ole = tmp("ole", "docx");
    apply(
        &src,
        &with_ole,
        json!([{
            "op": "insertOle",
            "id": para,
            "file": xlsx.to_string_lossy()
        }]),
    );
    let ole_id = inspect(&with_ole, json!({ "oleObjects": true }))["oleObjects"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let err = apply_err(
        &with_ole,
        json!([{
            "op": "insertHyperlink",
            "id": ole_id,
            "url": "https://example.com/ole"
        }]),
    );
    assert_eq!(err.code(), ErrorCode::InvalidId);
}
