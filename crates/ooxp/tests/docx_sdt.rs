//! docx 内容控件：insertContentControl / setContentControl / deleteContentControl。

use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::write::SimpleFileOptions;
use zip::CompressionMethod;
use zip::ZipWriter;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-docx-sdt");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn seq_path(tag: &str, ext: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}.{ext}"))
}

fn create_docx(tag: &str, document: Value) -> PathBuf {
    let path = seq_path(tag, "docx");
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

fn inspect(path: &Path, content_controls: bool, header_footer: bool) -> Value {
    let RunOutput::Inspect(tree) = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": path.to_string_lossy(),
            "scope": {
                "from": 1,
                "maxParas": 20,
                "contentControls": content_controls,
                "headerFooter": header_footer
            }
        })
        .to_string(),
        None,
    )
    .unwrap() else {
        panic!("inspect 应返回树");
    };
    tree
}

fn apply(path: &Path, out: &Path, ops: Value) {
    run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": path.to_string_lossy(),
            "output": out.to_string_lossy(),
            "ops": ops
        })
        .to_string(),
        None,
    )
    .unwrap();
}

fn apply_err(path: &Path, ops: Value) -> ooxp::Error {
    let out = seq_path("should-not-write", "docx");
    let _ = std::fs::remove_file(&out);
    run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": path.to_string_lossy(),
            "output": out.to_string_lossy(),
            "ops": ops
        })
        .to_string(),
        None,
    )
    .unwrap_err()
}

fn first_body_para_id(tree: &Value) -> String {
    tree["paragraphs"][0]["id"].as_str().unwrap().to_string()
}

fn object_keys(v: &Value) -> Vec<&str> {
    v.as_object().unwrap().keys().map(|k| k.as_str()).collect()
}

#[test]
fn insert_plain_text_lists_and_delete_keeps_text() {
    let src = create_docx(
        "sdt-src",
        json!({
            "paragraphs": [
                { "style": "Heading1", "text": "第一章" },
                { "text": "hello" }
            ]
        }),
    );
    let tree0 = inspect(&src, false, false);
    assert_eq!(tree0["contentControlCount"], 0);
    assert!(tree0.get("contentControls").is_none());
    let p1 = first_body_para_id(&tree0);
    let out = seq_path("sdt-out", "docx");
    apply(
        &src,
        &out,
        json!([{
            "op": "insertContentControl",
            "id": p1,
            "kind": "plainText",
            "tag": "name",
            "alias": "姓名",
            "text": "张三"
        }]),
    );
    let tree = inspect(&out, true, false);
    assert_eq!(tree["contentControlCount"], 1);
    assert_eq!(
        object_keys(&tree["contentControls"][0]),
        [
            "id", "index", "kind", "level", "host", "tag", "alias", "xmlId", "bound", "parent",
            "text"
        ]
    );
    assert_eq!(tree["contentControls"][0]["id"], "sdt:1");
    assert_eq!(tree["contentControls"][0]["kind"], "plainText");
    assert_eq!(tree["contentControls"][0]["level"], "inline");
    assert_eq!(tree["contentControls"][0]["text"], "张三");
    assert_eq!(tree["contentControls"][0]["tag"], "name");
    assert_eq!(tree["contentControls"][0]["alias"], "姓名");
    assert_eq!(tree["contentControls"][0]["bound"], false);
    assert!(tree["contentControls"][0]["parent"].is_null());
    assert_eq!(tree["contentControls"][0]["host"], p1);

    let set = seq_path("sdt-set", "docx");
    apply(
        &out,
        &set,
        json!([{ "op": "setContentControl", "id": "sdt:1", "text": "李四" }]),
    );
    let tree_set = inspect(&set, true, false);
    assert_eq!(tree_set["contentControls"][0]["text"], "李四");
    assert_eq!(tree_set["contentControls"][0]["kind"], "plainText");

    let gone = seq_path("sdt-del", "docx");
    apply(
        &set,
        &gone,
        json!([{ "op": "deleteContentControl", "id": "sdt:1", "keep": "content" }]),
    );
    let tree2 = inspect(&gone, true, false);
    assert_eq!(tree2["contentControlCount"], 0);
    let texts: Vec<&str> = tree2["paragraphs"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|p| p["text"].as_str())
        .collect();
    assert!(texts.iter().any(|t| t.contains("第一章")));
    assert!(texts.iter().any(|t| t.contains("李四")));
}

#[test]
fn insert_dropdown_and_document_end_and_header() {
    let src = create_docx(
        "dd-src",
        json!({
            "paragraphs": [{ "text": "甲" }],
            "headers": [{ "type": "default", "paragraphs": [{ "text": "页眉" }] }]
        }),
    );
    let p1 = first_body_para_id(&inspect(&src, false, true));
    let out = seq_path("dd", "docx");
    apply(
        &src,
        &out,
        json!([{
            "op": "insertContentControl",
            "id": p1,
            "kind": "dropDown",
            "text": "A",
            "items": ["A", "B"]
        }]),
    );
    let tree = inspect(&out, true, false);
    assert_eq!(tree["contentControls"][0]["kind"], "dropDown");
    assert_eq!(tree["contentControls"][0]["items"][0]["display"], "A");
    assert_eq!(tree["contentControls"][0]["items"][1]["value"], "B");

    let end = seq_path("end", "docx");
    apply(
        &src,
        &end,
        json!([{ "op": "insertContentControl", "id": "docx:document", "text": "末" }]),
    );
    let tree_end = inspect(&end, true, false);
    assert_eq!(tree_end["contentControlCount"], 1);
    assert_eq!(tree_end["contentControls"][0]["text"], "末");

    let hf_para = inspect(&src, false, true)["headers"][0]["paragraphs"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let hf = seq_path("hf", "docx");
    apply(
        &src,
        &hf,
        json!([{ "op": "insertContentControl", "id": hf_para, "text": "眉控" }]),
    );
    let tree_hf = inspect(&hf, true, true);
    assert_eq!(tree_hf["headers"][0]["contentControlCount"], 1);
    assert_eq!(
        tree_hf["headers"][0]["contentControls"][0]["id"],
        "header:default:1/sdt:1"
    );
    assert_eq!(tree_hf["headers"][0]["contentControls"][0]["text"], "眉控");
}

#[test]
fn set_para_text_blocked_and_rejects() {
    let src = create_docx(
        "blk-src",
        json!({ "paragraphs": [{ "text": "甲" }, { "text": "乙" }] }),
    );
    let p1 = first_body_para_id(&inspect(&src, false, false));
    let out = seq_path("blk", "docx");
    apply(
        &src,
        &out,
        json!([{ "op": "insertContentControl", "id": p1, "text": "控" }]),
    );
    let err = apply_err(
        &out,
        json!([{ "op": "setParaText", "id": p1, "text": "改" }]),
    );
    assert_eq!(err.code(), ErrorCode::NotImplemented);

    let err_kind = apply_err(
        &src,
        json!([{ "op": "insertContentControl", "id": p1, "kind": "checkbox", "text": "x" }]),
    );
    assert_eq!(err_kind.code(), ErrorCode::InvalidField);

    let err_id = apply_err(
        &src,
        json!([{ "op": "setContentControl", "id": "math:1", "text": "x" }]),
    );
    assert_eq!(err_id.code(), ErrorCode::InvalidId);

    let err_miss = apply_err(
        &src,
        json!([{ "op": "deleteContentControl", "id": "sdt:1" }]),
    );
    assert_eq!(err_miss.code(), ErrorCode::IdNotFound);
}

#[test]
fn xlsx_pptx_reject_sdt_ops() {
    let xlsx = seq_path("sdt-x", "xlsx");
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
    let xout = seq_path("sdt-x-out", "xlsx");
    let err_x = run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": xlsx.to_string_lossy(),
            "output": xout.to_string_lossy(),
            "ops": [{ "op": "insertContentControl", "id": "docx:document", "text": "x" }]
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err_x.code(), ErrorCode::InvalidField);

    let err_scope = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": xlsx.to_string_lossy(),
            "scope": { "sheet": "Sheet1", "range": "A1", "contentControls": true }
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err_scope.code(), ErrorCode::InvalidField);

    let pptx = seq_path("sdt-p", "pptx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": pptx.to_string_lossy(),
            "presentation": { "slides": [{ "shapes": [{ "text": "a" }] }] }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let pout = seq_path("sdt-p-out", "pptx");
    let err_p = run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": pptx.to_string_lossy(),
            "output": pout.to_string_lossy(),
            "ops": [{ "op": "deleteContentControl", "id": "sdt:1" }]
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err_p.code(), ErrorCode::InvalidField);
}

const CT: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>"#;

const ROOT_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#;

fn write_docx_xml(tag: &str, document: &str) -> PathBuf {
    let path = seq_path(tag, "docx");
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut zip = ZipWriter::new(&mut cursor);
        let opt = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        for (name, body) in [
            ("[Content_Types].xml", CT),
            ("_rels/.rels", ROOT_RELS),
            ("word/document.xml", document),
        ] {
            zip.start_file(name, opt).unwrap();
            zip.write_all(body.as_bytes()).unwrap();
        }
        zip.finish().unwrap();
    }
    std::fs::write(&path, cursor.into_inner()).unwrap();
    path
}

#[test]
fn gallery_sdt_skipped_nested_and_bound_not_implemented() {
    let src = write_docx_xml(
        "gallery",
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml"><w:body><w:sdt><w:sdtPr><w:docPartObj><w:docPartGallery w:val="Table of Contents"/></w:docPartObj></w:sdtPr><w:sdtContent><w:p w14:paraId="AAAA0001"><w:r><w:t>目录</w:t></w:r></w:p></w:sdtContent></w:sdt><w:p w14:paraId="AAAA0002"><w:sdt><w:sdtPr><w:alias w:val="外"/><w:id w:val="11"/><w:richText/></w:sdtPr><w:sdtContent><w:sdt><w:sdtPr><w:tag w:val="inner"/><w:id w:val="12"/><w:text/></w:sdtPr><w:sdtContent><w:r><w:t>内</w:t></w:r></w:sdtContent></w:sdt></w:sdtContent></w:sdt></w:p><w:p w14:paraId="AAAA0003"><w:sdt><w:sdtPr><w:id w:val="13"/><w:dataBinding w:xpath="/a"/><w:text/></w:sdtPr><w:sdtContent><w:r><w:t>绑</w:t></w:r></w:sdtContent></w:sdt></w:p><w:sectPr><w:pgSz w:w="11906" w:h="16838"/></w:sectPr></w:body></w:document>"#,
    );
    let tree = inspect(&src, true, false);
    assert_eq!(tree["contentControlCount"], 3);
    assert_eq!(tree["contentControls"][0]["kind"], "richText");
    assert_eq!(tree["contentControls"][1]["kind"], "plainText");
    assert_eq!(tree["contentControls"][1]["parent"], "sdt:1");
    assert_eq!(tree["contentControls"][1]["tag"], "inner");
    assert_eq!(tree["contentControls"][2]["bound"], true);
    assert_eq!(tree["contentControls"][2]["text"], "绑");

    let err = apply_err(
        &src,
        json!([{ "op": "setContentControl", "id": "sdt:3", "text": "新" }]),
    );
    assert_eq!(err.code(), ErrorCode::NotImplemented);
}
