//! 工作表文本框字填充与轮廓。

use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-xlsx-text-fill");
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

fn inspect_boxes(path: &Path) -> Value {
    let RunOutput::Inspect(tree) = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": path.to_string_lossy(),
            "scope": { "textBoxes": true }
        })
        .to_string(),
        None,
    )
    .unwrap() else {
        panic!("inspect 应返回树");
    };
    tree
}

fn part(path: &Path, name: &str) -> String {
    let bytes = std::fs::read(path).unwrap();
    let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut file = zip.by_name(name).unwrap();
    let mut out = Vec::new();
    file.read_to_end(&mut out).unwrap();
    String::from_utf8(out).unwrap()
}

fn rewrite_part(path: &Path, name: &str, f: impl FnOnce(String) -> String) {
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
                let s = String::from_utf8(data).unwrap();
                data = transform.take().expect("部件只应出现一次")(s).into_bytes();
            }
            zip_out.start_file(&n, opt).unwrap();
            zip_out.write_all(&data).unwrap();
        }
        zip_out.finish().unwrap();
    }
    std::fs::write(path, out.into_inner()).unwrap();
}

fn insert_box(src: &Path, out: &Path, text: &str) -> String {
    apply(
        src,
        out,
        json!([{
            "op": "insertTextBox",
            "id": "sheet:Sheet1",
            "from": "B2",
            "text": text
        }]),
    )
    .unwrap();
    inspect_boxes(out)["sheets"][0]["textBoxes"][0]["id"]
        .as_str()
        .unwrap()
        .to_string()
}

fn gradient_fill() -> Value {
    json!({
        "type": "gradient",
        "stops": [
            { "pos": 0, "color": "FF0000" },
            { "pos": 100000, "color": "0000FF" }
        ]
    })
}

#[test]
fn insert_then_gradient_and_outline() {
    let src = create_book("box");
    let inserted = tmp("box-ins");
    let id = insert_box(&src, &inserted, "你好");
    let out = tmp("box-grad");
    apply(
        &inserted,
        &out,
        json!([{
            "op": "setShapeFormat",
            "id": id,
            "textFill": gradient_fill(),
            "textOutline": { "color": "FF0000", "width": 19050 }
        }]),
    )
    .unwrap();
    let box0 = &inspect_boxes(&out)["sheets"][0]["textBoxes"][0];
    assert_eq!(box0["color"], Value::Null);
    assert_eq!(box0["textFill"]["type"], "gradient");
    assert_eq!(box0["textFill"]["ang"], 5400000);
    assert_eq!(box0["textOutline"]["width"], 19050);
    let xml = part(&out, "xl/drawings/drawing1.xml");
    assert!(xml.contains(r#"<a:gradFill flip="none" rotWithShape="1">"#));
    assert!(xml.contains(r#"<a:ln w="19050">"#));
}

#[test]
fn text_fill_none_and_create_box() {
    let path = tmp("create");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "workbook": {
                "sheets": [{
                    "name": "Sheet1",
                    "textBoxes": [{
                        "from": "B2",
                        "text": "渐变",
                        "textFill": gradient_fill()
                    }]
                }]
            }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let box0 = &inspect_boxes(&path)["sheets"][0]["textBoxes"][0];
    assert_eq!(box0["textFill"]["type"], "gradient");
    let xml = part(&path, "xl/drawings/drawing1.xml");
    assert!(xml.contains("<a:gradFill"));

    let none_out = tmp("none");
    apply(
        &path,
        &none_out,
        json!([{
            "op": "setShapeFormat",
            "id": box0["id"],
            "textFill": { "type": "none" }
        }]),
    )
    .unwrap();
    let after = &inspect_boxes(&none_out)["sheets"][0]["textBoxes"][0];
    assert_eq!(after["color"], Value::Null);
    assert_eq!(after["textFill"]["type"], "none");
}

#[test]
fn mixed_runs_on_textbox() {
    let src = create_book("mix");
    let inserted = tmp("mix-ins");
    insert_box(&src, &inserted, "Aa");
    rewrite_part(&inserted, "xl/drawings/drawing1.xml", |xml| {
        xml.replace(
            r#"<a:r><a:rPr sz="1100"/><a:t>Aa</a:t></a:r>"#,
            concat!(
                r#"<a:r><a:rPr sz="2800"/><a:t>A</a:t></a:r>"#,
                r#"<a:r><a:rPr sz="1200"/><a:t>a</a:t></a:r>"#,
            ),
        )
    });
    let box0 = &inspect_boxes(&inserted)["sheets"][0]["textBoxes"][0];
    assert_eq!(box0["sz"], Value::Null);
    let runs = box0["runs"].as_array().unwrap();
    assert_eq!(runs.len(), 2);
    assert_eq!(runs[0]["sz"], 2800);
    assert_eq!(runs[1]["sz"], 1200);
}

#[test]
fn rejects_color_with_text_fill() {
    let src = create_book("bad");
    let inserted = tmp("box-bad");
    let id = insert_box(&src, &inserted, "你好");
    let dest = tmp("both");
    let err = apply(
        &inserted,
        &dest,
        json!([{
            "op": "setShapeFormat",
            "id": id,
            "color": "FF0000",
            "textFill": { "type": "none" }
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!dest.exists());
}
