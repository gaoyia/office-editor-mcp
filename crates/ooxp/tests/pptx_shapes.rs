//! 形状增删与几何：插文本框、改位置尺寸填色、删除，以及 timing 保真。

use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-pptx-shapes");
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
    String::from_utf8(part_bytes(path, name)).unwrap()
}

fn part_bytes(path: &Path, name: &str) -> Vec<u8> {
    let bytes = std::fs::read(path).unwrap();
    let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut file = zip.by_name(name).unwrap();
    let mut out = Vec::new();
    file.read_to_end(&mut out).unwrap();
    out
}

fn rewrite_part(path: &Path, name: &str, f: impl FnOnce(Vec<u8>) -> Vec<u8>) {
    let bytes = std::fs::read(path).unwrap();
    let mut zip_in = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut out = Cursor::new(Vec::new());
    let mut transform = Some(f);
    {
        let mut zip_out = ZipWriter::new(&mut out);
        let opt = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        for i in 0..zip_in.len() {
            let mut file = zip_in.by_index(i).unwrap();
            let n = file.name().to_string();
            let mut data = Vec::new();
            file.read_to_end(&mut data).unwrap();
            if n == name {
                data = transform.take().expect("部件只应出现一次")(data);
            }
            zip_out.start_file(&n, opt).unwrap();
            zip_out.write_all(&data).unwrap();
        }
        zip_out.finish().unwrap();
    }
    std::fs::write(path, out.into_inner()).unwrap();
}

fn inject_timing(xml: &str) -> String {
    xml.replace(
        "</p:sld>",
        concat!(
            "<p:timing><p:tnLst><p:par><p:cTn id=\"1\" dur=\"indefinite\">",
            "<p:childTnLst>",
            "<p:set><p:cBhvr><p:cTn id=\"2\"/><p:tgtEl><p:spTgt spid=\"2\"/></p:tgtEl></p:cBhvr></p:set>",
            "<p:set><p:cBhvr><p:cTn id=\"3\"/><p:tgtEl><p:spTgt spid=\"3\"/></p:tgtEl></p:cBhvr></p:set>",
            "</p:childTnLst></p:cTn></p:par></p:tnLst></p:timing></p:sld>"
        ),
    )
}

#[test]
fn insert_shape_after_create() {
    let src = create_deck("ins");
    let out = tmp("ins-out");
    apply(
        &src,
        &out,
        json!([{
            "op": "insertShape",
            "id": "slide:1",
            "text": "新框",
            "x": 100,
            "y": 200,
            "cx": 1828800,
            "cy": 914400
        }]),
    )
    .unwrap();
    let tree = inspect_scope(&out, json!({ "slide": 1 }));
    let shapes = tree["slides"][0]["shapes"].as_array().unwrap();
    assert_eq!(shapes.len(), 3);
    let neu = shapes.iter().find(|s| s["text"] == "新框").unwrap();
    assert_eq!(neu["id"], "slide:1/shape:4");
    assert_eq!(neu["stable"], true);
    assert_eq!(neu["x"], 100);
    assert_eq!(neu["y"], 200);
    assert_eq!(neu["cx"], 1828800);
    assert_eq!(neu["cy"], 914400);
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
    assert!(xml.contains("txBox=\"1\""));
    assert!(xml.contains("prst=\"rect\""));
    assert_eq!(tree["slides"][1]["shapes"], Value::Null);
}

#[test]
fn create_inspect_has_geometry() {
    let src = create_deck("geo");
    let tree = inspect_scope(&src, json!({ "slide": 1 }));
    let s0 = &tree["slides"][0]["shapes"][0];
    assert_eq!(s0["x"], 838200);
    assert_eq!(s0["y"], 685800);
    assert_eq!(s0["cx"], 10515600);
    assert_eq!(s0["cy"], 800100);
}

#[test]
fn pos_size_fill_line_and_text() {
    let src = create_deck("mut");
    let inserted = tmp("mut-ins");
    apply(
        &src,
        &inserted,
        json!([{ "op": "insertShape", "id": "slide:1", "text": "框" }]),
    )
    .unwrap();
    let out = tmp("mut-out");
    apply(
        &inserted,
        &out,
        json!([
            {
                "op": "setShapePos",
                "id": "slide:1/shape:4",
                "x": 914400,
                "y": 1371600
            },
            {
                "op": "setShapeSize",
                "id": "slide:1/shape:4",
                "cx": 1828800,
                "cy": 914400
            },
            {
                "op": "setShapeFill",
                "id": "slide:1/shape:4",
                "color": "FF0000"
            },
            {
                "op": "setShapeLine",
                "id": "slide:1/shape:4",
                "color": "0000FF",
                "width": 12700
            },
            {
                "op": "setShapeText",
                "id": "slide:1/shape:4",
                "text": "改过"
            }
        ]),
    )
    .unwrap();
    let tree = inspect_scope(&out, json!({ "slide": 1 }));
    let neu = tree["slides"][0]["shapes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["id"] == "slide:1/shape:4")
        .unwrap();
    assert_eq!(neu["text"], "改过");
    assert_eq!(neu["x"], 914400);
    assert_eq!(neu["y"], 1371600);
    assert_eq!(neu["cx"], 1828800);
    assert_eq!(neu["cy"], 914400);
    let xml = part(&out, "ppt/slides/slide1.xml");
    assert!(xml.contains("val=\"FF0000\""));
    assert!(xml.contains("val=\"0000FF\""));
    assert!(xml.contains("w=\"12700\""));
    assert_eq!(tree["slides"][0]["shapes"][0]["text"], "标题");
}

#[test]
fn delete_keeps_other_cnv_ids() {
    let src = create_deck("del");
    let inserted = tmp("del-ins");
    apply(
        &src,
        &inserted,
        json!([{ "op": "insertShape", "id": "slide:1", "text": "将删" }]),
    )
    .unwrap();
    let out = tmp("del-out");
    apply(
        &inserted,
        &out,
        json!([{ "op": "deleteShape", "id": "slide:1/shape:4" }]),
    )
    .unwrap();
    let tree = inspect_scope(&out, json!({ "slide": 1 }));
    let shapes = tree["slides"][0]["shapes"].as_array().unwrap();
    assert_eq!(shapes.len(), 2);
    assert_eq!(shapes[0]["id"], "slide:1/shape:2");
    assert_eq!(shapes[1]["id"], "slide:1/shape:3");
    let xml = part(&out, "ppt/slides/slide1.xml");
    assert!(!xml.contains("将删"));
    assert!(xml.contains("id=\"2\""));
    assert!(xml.contains("id=\"3\""));
}

#[test]
fn timing_kept_on_insert_pos_and_cleared_on_delete() {
    let src = create_deck("tm");
    rewrite_part(&src, "ppt/slides/slide1.xml", |data| {
        let xml = String::from_utf8(data).unwrap();
        inject_timing(&xml).into_bytes()
    });
    let inserted = tmp("tm-ins");
    apply(
        &src,
        &inserted,
        json!([
            { "op": "insertShape", "id": "slide:1", "text": "新" },
            {
                "op": "setShapePos",
                "id": "slide:1/shape:3",
                "x": 10
            }
        ]),
    )
    .unwrap();
    let xml_ins = part(&inserted, "ppt/slides/slide1.xml");
    assert!(xml_ins.contains("spid=\"2\""));
    assert!(xml_ins.contains("spid=\"3\""));

    let sized = tmp("tm-size");
    apply(
        &inserted,
        &sized,
        json!([{ "op": "setShapeSize", "id": "slide:1/shape:2", "cx": 100000 }]),
    )
    .unwrap();
    let xml_sz = part(&sized, "ppt/slides/slide1.xml");
    assert!(xml_sz.contains("spid=\"2\""));
    assert!(xml_sz.contains("spid=\"3\""));

    let out = tmp("tm-del");
    apply(
        &sized,
        &out,
        json!([{ "op": "deleteShape", "id": "slide:1/shape:2" }]),
    )
    .unwrap();
    let xml = part(&out, "ppt/slides/slide1.xml");
    assert!(!xml.contains("spid=\"2\""));
    assert!(xml.contains("spid=\"3\""));
    assert!(xml.contains("<p:timing>"));
}

#[test]
fn wrong_ids_and_missing_fields() {
    let src = create_deck("bad");
    let err = apply(
        &src,
        &tmp("bad-ins"),
        json!([{ "op": "insertShape", "id": "slide:1/shape:2" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);

    let err = apply(
        &src,
        &tmp("bad-del"),
        json!([{ "op": "deleteShape", "id": "pptx:presentation" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);

    let dest = tmp("bad-cx");
    let err = apply(
        &src,
        &dest,
        json!([{ "op": "insertShape", "id": "slide:1", "cx": 100 }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!dest.exists());

    let dest = tmp("bad-color");
    let err = apply(
        &src,
        &dest,
        json!([{ "op": "setShapeFill", "id": "slide:1/shape:2" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!dest.exists());

    let dest = tmp("bad-tb");
    let err = apply(
        &src,
        &dest,
        json!([{ "op": "insertTextBox", "id": "slide:1" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!dest.exists());
}

#[test]
fn no_txbody_is_not_found() {
    let src = create_deck("notx");
    rewrite_part(&src, "ppt/slides/slide1.xml", |data| {
        let mut xml = String::from_utf8(data).unwrap();
        xml = xml.replace(
            "</p:spTree>",
            concat!(
                "<p:sp><p:nvSpPr><p:cNvPr id=\"9\" name=\"无字\"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>",
                "<p:spPr/></p:sp></p:spTree>"
            ),
        );
        xml.into_bytes()
    });
    let tree = inspect_scope(&src, json!({ "slide": 1 }));
    let ids: Vec<_> = tree["slides"][0]["shapes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["id"].as_str().unwrap())
        .collect();
    assert!(!ids.iter().any(|id| id.contains("shape:9")));

    let dest = tmp("notx-set");
    let err = apply(
        &src,
        &dest,
        json!([{ "op": "setShapeText", "id": "slide:1/shape:9", "text": "x" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::IdNotFound);
    assert!(!dest.exists());

    let dest = tmp("notx-del");
    let err = apply(
        &src,
        &dest,
        json!([{ "op": "deleteShape", "id": "slide:1/shape:9" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::IdNotFound);
    assert!(!dest.exists());
}

#[test]
fn foreign_format_rejects() {
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
            "output": tmp("xlsx-out").with_extension("xlsx").to_string_lossy(),
            "ops": [{ "op": "insertShape", "id": "sheet:Sheet1" }]
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
            "output": tmp("docx-out").with_extension("docx").to_string_lossy(),
            "ops": [{ "op": "deleteShape", "id": "slide:1/shape:2" }]
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}
