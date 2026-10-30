//! docx 文档保护：inspect 根上 `protection`，setProtection 开关无密码标记。

use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

static SEQ: AtomicU32 = AtomicU32::new(0);

const PROT_KEYS: [&str; 5] = [
    "enforced",
    "edit",
    "formatting",
    "hasPassword",
    "readOnlyRecommended",
];

const ROOT_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#;

const STYLES: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/></w:style></w:styles>"#;

const DOC: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>hello</w:t></w:r></w:p><w:sectPr/></w:body></w:document>"#;

const DOC_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/settings" Target="settings.xml"/></Relationships>"#;

const CT: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/><Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/><Override PartName="/word/settings.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.settings+xml"/></Types>"#;

const SETTINGS_HASH: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:settings xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:documentProtection w:edit="readOnly" w:enforcement="1" w:hash="YWJj" w:salt="ZGVm"/><w:compat><w:compatSetting w:name="compatibilityMode" w:uri="http://schemas.microsoft.com/office/word" w:val="15"/></w:compat></w:settings>"#;

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-docx-protect");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn seq_path(tag: &str, ext: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}.{ext}"))
}

fn object_keys(v: &Value) -> Vec<&str> {
    v.as_object().unwrap().keys().map(String::as_str).collect()
}

fn run_json(req: &Value) -> Result<RunOutput, ooxp::Error> {
    run(&req.to_string())
}

fn inspect(path: &Path) -> Value {
    let RunOutput::Inspect(tree) = run_json(&json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": path.to_string_lossy()
    }))
    .unwrap() else {
        panic!("inspect 应返回树");
    };
    tree
}

fn apply(src: &Path, out: &Path, ops: Value) {
    run_json(&json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": out.to_string_lossy(),
        "ops": ops
    }))
    .unwrap();
}

fn apply_err(src: &Path, ops: Value) -> ooxp::Error {
    let out = seq_path("should-not-write", "docx");
    let _ = std::fs::remove_file(&out);
    let err = run_json(&json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": out.to_string_lossy(),
        "ops": ops
    }))
    .unwrap_err();
    assert!(!out.exists(), "失败的 apply 不应写出文件");
    err
}

fn create_docx(tag: &str, protection: Option<Value>) -> PathBuf {
    let path = seq_path(tag, "docx");
    let mut document = json!({
        "paragraphs": [
            { "style": "Heading1", "text": "标题" },
            { "text": "hello" }
        ]
    });
    if let Some(p) = protection {
        document["protection"] = p;
    }
    run_json(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "format": "docx",
        "document": document
    }))
    .unwrap();
    path
}

fn part_text(path: &Path, name: &str) -> String {
    let bytes = std::fs::read(path).unwrap();
    let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut file = zip.by_name(name.trim_start_matches('/')).unwrap();
    let mut s = String::new();
    file.read_to_string(&mut s).unwrap();
    s
}

fn write_zip(tag: &str, files: &[(&str, &[u8])]) -> PathBuf {
    let path = seq_path(tag, "docx");
    let mut buf = Vec::new();
    {
        let mut zip = ZipWriter::new(Cursor::new(&mut buf));
        let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        for (name, data) in files {
            zip.start_file(*name, opts).unwrap();
            zip.write_all(data).unwrap();
        }
        zip.finish().unwrap();
    }
    std::fs::write(&path, buf).unwrap();
    path
}

fn hashed_sample(tag: &str) -> PathBuf {
    write_zip(
        tag,
        &[
            ("[Content_Types].xml", CT.as_bytes()),
            ("_rels/.rels", ROOT_RELS.as_bytes()),
            ("word/document.xml", DOC.as_bytes()),
            ("word/_rels/document.xml.rels", DOC_RELS.as_bytes()),
            ("word/styles.xml", STYLES.as_bytes()),
            ("word/settings.xml", SETTINGS_HASH.as_bytes()),
        ],
    )
}

#[test]
fn empty_create_has_protection_off() {
    let path = create_docx("empty", None);
    let tree = inspect(&path);
    let p = &tree["protection"];
    assert_eq!(object_keys(p), PROT_KEYS);
    assert_eq!(p["enforced"], false);
    assert!(p["edit"].is_null());
    assert_eq!(p["formatting"], false);
    assert_eq!(p["hasPassword"], false);
    assert_eq!(p["readOnlyRecommended"], false);
}

#[test]
fn set_readonly_then_off() {
    let src = create_docx("ro-host", None);
    let on = seq_path("ro-on", "docx");
    apply(
        &src,
        &on,
        json!([{
            "op": "setProtection",
            "id": "docx:document",
            "enforced": true,
            "edit": "readOnly"
        }]),
    );
    let tree = inspect(&on);
    assert_eq!(tree["protection"]["enforced"], true);
    assert_eq!(tree["protection"]["edit"], "readOnly");
    assert_eq!(tree["protection"]["hasPassword"], false);
    let settings = part_text(&on, "word/settings.xml");
    assert!(settings.contains("documentProtection"), "{settings}");
    assert!(settings.contains("readOnly"), "{settings}");
    assert!(!settings.contains("hash"), "{settings}");

    let off = seq_path("ro-off", "docx");
    apply(
        &on,
        &off,
        json!([{ "op": "setProtection", "id": "docx:document", "enforced": false }]),
    );
    let tree2 = inspect(&off);
    assert_eq!(tree2["protection"]["enforced"], false);
    assert!(tree2["protection"]["edit"].is_null());
    let settings2 = part_text(&off, "word/settings.xml");
    assert!(!settings2.contains("documentProtection"), "{settings2}");
}

#[test]
fn set_forms_writes_form_prot() {
    let src = create_docx("forms-host", None);
    let out = seq_path("forms-on", "docx");
    apply(
        &src,
        &out,
        json!([{
            "op": "setProtection",
            "id": "docx:document",
            "enforced": true,
            "edit": "forms"
        }]),
    );
    let tree = inspect(&out);
    assert_eq!(tree["protection"]["edit"], "forms");
    let doc = part_text(&out, "word/document.xml");
    assert!(doc.contains("formProt"), "{doc}");
}

#[test]
fn read_only_recommended() {
    let src = create_docx("ror-host", None);
    let out = seq_path("ror-on", "docx");
    apply(
        &src,
        &out,
        json!([{
            "op": "setProtection",
            "id": "docx:document",
            "readOnlyRecommended": true
        }]),
    );
    let tree = inspect(&out);
    assert_eq!(tree["protection"]["readOnlyRecommended"], true);
    assert_eq!(tree["protection"]["enforced"], false);
    let settings = part_text(&out, "word/settings.xml");
    assert!(settings.contains("writeProtection"), "{settings}");
    assert!(settings.contains("recommended"), "{settings}");
}

#[test]
fn hashed_file_is_read_only_and_apply_is_not_implemented() {
    let src = hashed_sample("hashed");
    let tree = inspect(&src);
    assert_eq!(tree["protection"]["enforced"], true);
    assert_eq!(tree["protection"]["edit"], "readOnly");
    assert_eq!(tree["protection"]["hasPassword"], true);
    let raw = serde_json::to_string(&tree).unwrap();
    assert!(!raw.contains("secret"), "{raw}");
    assert!(!raw.contains("YWJj"), "{raw}");

    let err = apply_err(
        &src,
        json!([{ "op": "setProtection", "id": "docx:document", "enforced": false }]),
    );
    assert_eq!(err.code(), ErrorCode::NotImplemented);
}

#[test]
fn create_password_sets_protection() {
    let path = seq_path("pw-create", "docx");
    run_json(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "format": "docx",
        "document": {
            "paragraphs": [{ "text": "hello" }],
            "protection": { "enforced": true, "edit": "readOnly", "password": "secret" }
        }
    }))
    .unwrap();
    let tree = inspect(&path);
    assert_eq!(tree["protection"]["enforced"], true);
    assert_eq!(tree["protection"]["hasPassword"], true);
    assert!(tree["protection"].get("password").is_none());
}

#[test]
fn create_with_protection() {
    let path = create_docx(
        "create-prot",
        Some(json!({ "enforced": true, "edit": "comments" })),
    );
    let tree = inspect(&path);
    assert_eq!(tree["protection"]["enforced"], true);
    assert_eq!(tree["protection"]["edit"], "comments");
}

#[test]
fn wrong_id_and_empty_fields() {
    let src = create_docx("bad-id", None);
    let err = apply_err(
        &src,
        json!([{ "op": "setProtection", "id": "para:00000001", "enforced": true }]),
    );
    assert_eq!(err.code(), ErrorCode::InvalidId);
    let err2 = apply_err(
        &src,
        json!([{ "op": "setProtection", "id": "docx:document" }]),
    );
    assert_eq!(err2.code(), ErrorCode::InvalidField);
}

#[test]
fn xlsx_and_pptx_reject_set_protection() {
    let xlsx = seq_path("rej", "xlsx");
    run_json(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": xlsx.to_string_lossy(),
        "workbook": { "sheets": [{ "name": "Sheet1", "cells": [{ "ref": "A1", "value": 1 }] }] }
    }))
    .unwrap();
    let err = apply_err(
        &xlsx,
        json!([{ "op": "setProtection", "id": "xlsx:workbook", "edit": "readOnly" }]),
    );
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let pptx = seq_path("rej", "pptx");
    run_json(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": pptx.to_string_lossy(),
        "presentation": { "slides": [{ "shapes": [{ "text": "a" }] }] }
    }))
    .unwrap();
    let err = apply_err(
        &pptx,
        json!([{ "op": "setProtection", "id": "pptx:presentation", "edit": "readOnly" }]),
    );
    assert_eq!(err.code(), ErrorCode::InvalidField);
}
