//! Strict 包可以读，apply 在写盘前拒绝。

use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, Lang, RunOutput};
use serde_json::json;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

static SEQ: AtomicU32 = AtomicU32::new(0);

const REL_TRANSITIONAL: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument";
const REL_STRICT: &str = "http://purl.oclc.org/ooxml/officeDocument/relationships/officeDocument";

fn tmp(ext: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join("ooxp-strict");
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(format!("strict-{n}.{ext}"))
}

fn create(ext: &str, body: serde_json::Value) -> PathBuf {
    let path = tmp(ext);
    let mut request = json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
    });
    request
        .as_object_mut()
        .unwrap()
        .extend(body.as_object().unwrap().clone());
    run(&request.to_string()).unwrap();
    path
}

fn rewrite_text(path: &Path, pairs: &[(&str, &str)]) {
    let bytes = std::fs::read(path).unwrap();
    let mut zip_in = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut out = Cursor::new(Vec::new());
    {
        let mut zip_out = ZipWriter::new(&mut out);
        let opt = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        for i in 0..zip_in.len() {
            let mut file = zip_in.by_index(i).unwrap();
            let name = file.name().to_string();
            let mut data = Vec::new();
            file.read_to_end(&mut data).unwrap();
            let text_part = name.ends_with(".xml") || name.ends_with(".rels");
            if text_part {
                if let Ok(text) = String::from_utf8(data.clone()) {
                    let mut next = text;
                    for (from, to) in pairs {
                        next = next.replace(from, to);
                    }
                    data = next.into_bytes();
                }
            }
            zip_out.start_file(&name, opt).unwrap();
            zip_out.write_all(&data).unwrap();
        }
        zip_out.finish().unwrap();
    }
    std::fs::write(path, out.into_inner()).unwrap();
}

fn inspect(path: &Path) -> RunOutput {
    run(&json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": path.to_string_lossy(),
    })
    .to_string())
    .unwrap()
}

fn apply_err(path: &Path, output: Option<&Path>, op: serde_json::Value) -> ooxp::Error {
    let mut request = json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": path.to_string_lossy(),
        "ops": [op],
    });
    if let Some(output) = output {
        request["output"] = json!(output.to_string_lossy());
    }
    match run(&request.to_string()) {
        Ok(other) => panic!("Strict 包的 apply 应拒绝，得到 {other:?}"),
        Err(err) => err,
    }
}

fn assert_strict_rejected(path: &Path, op: serde_json::Value) {
    let before = std::fs::read(path).unwrap();
    inspect(path);
    let out = path.with_extension(format!(
        "out.{}",
        path.extension().unwrap().to_string_lossy()
    ));
    if out.exists() {
        std::fs::remove_file(&out).unwrap();
    }
    let err = apply_err(path, Some(&out), op.clone());
    assert_eq!(err.code(), ErrorCode::NotImplemented);
    assert!(err.to_string().contains("Strict"));
    assert!(err.to_string().contains("Transitional"));
    let zh = err.to_json(Lang::ZhCn);
    assert!(zh["error"]["message"].as_str().unwrap().contains("另存"));
    assert!(!out.exists(), "拒绝之后不应生成目标文件");
    assert_eq!(
        std::fs::read(path).unwrap(),
        before,
        "另存被拒时源文件不能变"
    );
    let err = apply_err(path, None, op);
    assert_eq!(err.code(), ErrorCode::NotImplemented);
    assert_eq!(
        std::fs::read(path).unwrap(),
        before,
        "覆盖被拒时源文件不能变"
    );
}

#[test]
fn docx_strict_can_be_read_and_apply_is_rejected() {
    let path = create(
        "docx",
        json!({ "document": { "paragraphs": [{ "text": "P1" }] } }),
    );
    rewrite_text(
        &path,
        &[
            (REL_TRANSITIONAL, REL_STRICT),
            (
                "http://schemas.openxmlformats.org/wordprocessingml/2006/main",
                "http://purl.oclc.org/ooxml/wordprocessingml/main",
            ),
        ],
    );
    assert_strict_rejected(
        &path,
        json!({ "op": "setParaText", "id": "para:index:1", "text": "Z" }),
    );
}

#[test]
fn strict_point_length_is_read_as_twips() {
    let path = create(
        "docx",
        json!({ "document": { "paragraphs": [{ "text": "P1" }] } }),
    );
    rewrite_text(&path, &[("w:header=\"708\"", "w:header=\"42.55pt\"")]);
    let RunOutput::Inspect(tree) = inspect(&path) else {
        panic!("inspect 应返回树");
    };
    assert_eq!(tree["sections"][0]["headerDistance"], 851);
}

#[test]
fn docx_strict_rel_type_alone_is_enough() {
    let path = create(
        "docx",
        json!({ "document": { "paragraphs": [{ "text": "P1" }] } }),
    );
    rewrite_text(&path, &[(REL_TRANSITIONAL, REL_STRICT)]);
    let before = std::fs::read(&path).unwrap();
    let err = apply_err(
        &path,
        None,
        json!({ "op": "setParaText", "id": "para:index:1", "text": "Z" }),
    );
    assert_eq!(err.code(), ErrorCode::NotImplemented);
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

#[test]
fn docx_strict_main_namespace_alone_is_enough() {
    let path = create(
        "docx",
        json!({ "document": { "paragraphs": [{ "text": "P1" }] } }),
    );
    let before = std::fs::read(&path).unwrap();
    rewrite_text(
        &path,
        &[(
            "http://schemas.openxmlformats.org/wordprocessingml/2006/main",
            "http://purl.oclc.org/ooxml/wordprocessingml/main",
        )],
    );
    let strict = std::fs::read(&path).unwrap();
    let err = apply_err(
        &path,
        None,
        json!({ "op": "setParaText", "id": "para:index:1", "text": "Z" }),
    );
    assert_eq!(err.code(), ErrorCode::NotImplemented);
    assert_eq!(std::fs::read(&path).unwrap(), strict);
    assert_ne!(strict, before);
}

#[test]
fn xlsx_strict_can_be_read_and_apply_is_rejected() {
    let path = create(
        "xlsx",
        json!({
            "workbook": { "sheets": [{ "name": "Sheet1", "cells": [{ "ref": "A1", "value": 1 }] }] }
        }),
    );
    rewrite_text(
        &path,
        &[
            (REL_TRANSITIONAL, REL_STRICT),
            (
                "http://schemas.openxmlformats.org/spreadsheetml/2006/main",
                "http://purl.oclc.org/ooxml/spreadsheetml/main",
            ),
        ],
    );
    assert_strict_rejected(
        &path,
        json!({ "op": "setCell", "id": "cell:Sheet1!A1", "value": 2 }),
    );
}

#[test]
fn pptx_strict_can_be_read_and_apply_is_rejected() {
    let path = create(
        "pptx",
        json!({
            "presentation": { "slides": [{ "shapes": [{ "text": "标题" }] }] }
        }),
    );
    rewrite_text(
        &path,
        &[
            (REL_TRANSITIONAL, REL_STRICT),
            (
                "http://schemas.openxmlformats.org/presentationml/2006/main",
                "http://purl.oclc.org/ooxml/presentationml/main",
            ),
        ],
    );
    assert_strict_rejected(
        &path,
        json!({ "op": "setShapeText", "id": "slide:1/shape:1", "text": "Z" }),
    );
}
