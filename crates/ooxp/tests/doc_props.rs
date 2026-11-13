//! 三格式文件属性：inspect 根上 `props`，setProps / setCustomProp / deleteCustomProp。

use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::ZipArchive;

static SEQ: AtomicU32 = AtomicU32::new(0);

const PROPS_KEYS: [&str; 12] = [
    "title",
    "subject",
    "creator",
    "description",
    "keywords",
    "category",
    "lastModifiedBy",
    "created",
    "modified",
    "company",
    "manager",
    "custom",
];

const CUSTOM_KEYS: [&str; 4] = ["id", "name", "type", "value"];

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-doc-props");
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
    let out = seq_path("should-not-write", "bin");
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

fn part_text(path: &Path, name: &str) -> Option<String> {
    let bytes = std::fs::read(path).unwrap();
    let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let Ok(mut file) = zip.by_name(name.trim_start_matches('/')) else {
        return None;
    };
    let mut s = String::new();
    file.read_to_string(&mut s).unwrap();
    Some(s)
}

fn part_bytes(path: &Path, name: &str) -> Vec<u8> {
    let bytes = std::fs::read(path).unwrap();
    let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut file = zip.by_name(name.trim_start_matches('/')).unwrap();
    let mut buf = Vec::new();
    file.read_to_end(&mut buf).unwrap();
    buf
}

fn create_xlsx(tag: &str, props: Option<Value>) -> PathBuf {
    let path = seq_path(tag, "xlsx");
    let mut workbook = json!({
        "sheets": [{ "name": "Sheet1", "cells": [{ "ref": "A1", "value": 1 }] }]
    });
    if let Some(p) = props {
        workbook["props"] = p;
    }
    run_json(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "format": "xlsx",
        "workbook": workbook
    }))
    .unwrap();
    path
}

fn create_docx(tag: &str, props: Option<Value>) -> PathBuf {
    let path = seq_path(tag, "docx");
    let mut document = json!({
        "paragraphs": [
            { "style": "Heading1", "text": "第一章" },
            { "text": "hello" }
        ]
    });
    if let Some(p) = props {
        document["props"] = p;
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

fn create_pptx(tag: &str, props: Option<Value>) -> PathBuf {
    let path = seq_path(tag, "pptx");
    let mut presentation = json!({
        "slides": [{ "shapes": [{ "text": "标题" }] }]
    });
    if let Some(p) = props {
        presentation["props"] = p;
    }
    run_json(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "format": "pptx",
        "presentation": presentation
    }))
    .unwrap();
    path
}

fn assert_default_props(tree: &Value) {
    let props = &tree["props"];
    assert_eq!(object_keys(props), PROPS_KEYS);
    assert_eq!(props["title"], "");
    assert_eq!(props["creator"], "ooxp");
    assert!(props["subject"].is_null());
    assert!(props["company"].is_null());
    assert!(props["manager"].is_null());
    assert_eq!(props["custom"], json!([]));
}

#[test]
fn inspect_default_props_on_three_formats() {
    let xlsx = create_xlsx("bare-xlsx", None);
    let docx = create_docx("bare-docx", None);
    let pptx = create_pptx("bare-pptx", None);
    let tx = inspect(&xlsx);
    let td = inspect(&docx);
    let tp = inspect(&pptx);
    assert_eq!(tx["id"], "xlsx:workbook");
    assert_eq!(td["id"], "docx:document");
    assert_eq!(tp["id"], "pptx:presentation");
    assert_eq!(
        object_keys(&tx)[3..],
        [
            "pictureCount",
            "chartCount",
            "textBoxCount",
            "mathCount",
            "shapeCount",
            "connectorCount",
            "smartArtCount",
            "commentCount",
            "hyperlinkCount",
            "sparklineCount",
            "oleObjectCount",
            "formControlCount",
            "model3dCount",
            "inkCount",
            "slicerCount",
            "connectionCount",
            "externalLinkCount",
            "customViewCount",
            "definedNameCount",
            "cellStyleCount",
            "props",
            "workbookProtection",
            "theme",
            "themeOverride",
            "sheets",
            "definedNames"
        ]
    );
    assert_eq!(
        object_keys(&td)[object_keys(&td).len() - 1],
        "bibliographyStyle"
    );
    assert_eq!(object_keys(&tp)[object_keys(&tp).len() - 2], "props");
    assert_eq!(object_keys(&tp)[object_keys(&tp).len() - 1], "slides");
    assert_default_props(&tx);
    assert_default_props(&td);
    assert_default_props(&tp);
}

#[test]
fn set_title_keeps_company_on_three_formats() {
    let props = json!({ "company": "测试公司" });
    for (src, root, host) in [
        (
            create_xlsx("co-xlsx", Some(props.clone())),
            "xlsx:workbook",
            "xl/workbook.xml",
        ),
        (
            create_docx("co-docx", Some(props.clone())),
            "docx:document",
            "word/document.xml",
        ),
        (
            create_pptx("co-pptx", Some(props.clone())),
            "pptx:presentation",
            "ppt/presentation.xml",
        ),
    ] {
        let before = inspect(&src);
        assert_eq!(before["props"]["company"], "测试公司");
        assert_eq!(before["props"]["title"], "");
        let host_before = part_bytes(&src, host);
        let ext = src.extension().unwrap().to_string_lossy().into_owned();
        let out = seq_path("title-only", &ext);
        apply(
            &src,
            &out,
            json!([{ "op": "setProps", "id": root, "title": "新标题" }]),
        );
        let after = inspect(&out);
        assert_eq!(after["props"]["title"], "新标题");
        assert_eq!(after["props"]["company"], "测试公司");
        assert_eq!(after["props"]["creator"], "ooxp");
        assert_eq!(part_bytes(&out, host), host_before);
        if ext == "xlsx" {
            let app = part_text(&out, "docProps/app.xml").unwrap();
            assert!(app.contains("HeadingPairs"), "{app}");
            assert!(app.contains("TitlesOfParts"), "{app}");
            assert!(app.contains("测试公司"), "{app}");
        }
        if ext == "pptx" {
            let app = part_text(&out, "docProps/app.xml").unwrap();
            assert!(app.contains("<Slides>"), "{app}");
            assert!(app.contains("测试公司"), "{app}");
        }
    }
}

#[test]
fn custom_prop_upsert_and_delete_removes_part() {
    let src = create_docx("custom-host", None);
    let with_one = seq_path("custom-one", "docx");
    apply(
        &src,
        &with_one,
        json!([{
            "op": "setCustomProp",
            "id": "docx:document",
            "name": "部门",
            "value": "研发"
        }]),
    );
    let tree = inspect(&with_one);
    let custom = tree["props"]["custom"].as_array().unwrap();
    assert_eq!(custom.len(), 1);
    assert_eq!(object_keys(&custom[0]), CUSTOM_KEYS);
    assert_eq!(custom[0]["id"], "prop:部门");
    assert_eq!(custom[0]["name"], "部门");
    assert_eq!(custom[0]["type"], "lpwstr");
    assert_eq!(custom[0]["value"], "研发");
    let xml1 = part_text(&with_one, "docProps/custom.xml").unwrap();
    assert!(xml1.contains("pid=\"2\""), "{xml1}");
    assert!(
        xml1.contains("fmtid=\"{D5CDD505-2E9C-101B-9397-08002B2CF9AE}\""),
        "{xml1}"
    );

    let with_two = seq_path("custom-two", "docx");
    apply(
        &with_one,
        &with_two,
        json!([
            {
                "op": "setCustomProp",
                "id": "docx:document",
                "name": "部门",
                "value": "市场"
            },
            {
                "op": "setCustomProp",
                "id": "docx:document",
                "name": "人数",
                "value": 12
            }
        ]),
    );
    let tree2 = inspect(&with_two);
    let custom2 = tree2["props"]["custom"].as_array().unwrap();
    assert_eq!(custom2.len(), 2);
    assert_eq!(custom2[0]["name"], "部门");
    assert_eq!(custom2[0]["value"], "市场");
    assert_eq!(custom2[1]["name"], "人数");
    assert_eq!(custom2[1]["type"], "i4");
    assert_eq!(custom2[1]["value"], 12);
    let xml2 = part_text(&with_two, "docProps/custom.xml").unwrap();
    assert!(xml2.contains("pid=\"2\""), "{xml2}");
    assert!(xml2.contains("pid=\"3\""), "{xml2}");

    let dropped = seq_path("custom-drop-one", "docx");
    apply(
        &with_two,
        &dropped,
        json!([{ "op": "deleteCustomProp", "id": "prop:部门" }]),
    );
    let tree3 = inspect(&dropped);
    let custom3 = tree3["props"]["custom"].as_array().unwrap();
    assert_eq!(custom3.len(), 1);
    assert_eq!(custom3[0]["name"], "人数");

    let empty = seq_path("custom-empty", "docx");
    apply(
        &dropped,
        &empty,
        json!([{ "op": "deleteCustomProp", "id": "prop:人数" }]),
    );
    let tree4 = inspect(&empty);
    assert_eq!(tree4["props"]["custom"], json!([]));
    assert!(part_text(&empty, "docProps/custom.xml").is_none());
    let cts = part_text(&empty, "[Content_Types].xml").unwrap();
    assert!(!cts.contains("custom-properties"), "{cts}");
    let rels = part_text(&empty, "_rels/.rels").unwrap();
    assert!(!rels.contains("custom-properties"), "{rels}");
}

#[test]
fn create_props_and_custom() {
    let path = create_xlsx(
        "create-props",
        Some(json!({
            "title": "工作簿标题",
            "company": "测试公司",
            "custom": [{ "name": "部门", "value": "研发" }]
        })),
    );
    let tree = inspect(&path);
    assert_eq!(tree["props"]["title"], "工作簿标题");
    assert_eq!(tree["props"]["company"], "测试公司");
    assert_eq!(tree["props"]["creator"], "ooxp");
    assert_eq!(tree["props"]["custom"][0]["id"], "prop:部门");
    let app = part_text(&path, "docProps/app.xml").unwrap();
    assert!(app.contains("HeadingPairs"), "{app}");
    assert!(app.contains("测试公司"), "{app}");
}

#[test]
fn set_props_rejects_wrong_id_and_empty_fields() {
    let xlsx = create_xlsx("bad-xlsx", None);
    let docx = create_docx("bad-docx", None);
    let pptx = create_pptx("bad-pptx", None);

    let e1 = apply_err(
        &xlsx,
        json!([{ "op": "setProps", "id": "cell:Sheet1!A1", "title": "x" }]),
    );
    assert_eq!(e1.code(), ErrorCode::InvalidId);

    let e2 = apply_err(
        &docx,
        json!([{ "op": "setProps", "id": "para:1", "title": "x" }]),
    );
    assert_eq!(e2.code(), ErrorCode::InvalidId);

    let e3 = apply_err(
        &pptx,
        json!([{ "op": "setProps", "id": "slide:1/shape:2", "title": "x" }]),
    );
    assert_eq!(e3.code(), ErrorCode::InvalidId);

    let e4 = apply_err(&docx, json!([{ "op": "setProps", "id": "docx:document" }]));
    assert_eq!(e4.code(), ErrorCode::InvalidField);

    let e5 = apply_err(&docx, json!([{ "op": "deleteCustomProp", "id": "para:1" }]));
    assert_eq!(e5.code(), ErrorCode::InvalidId);

    let e6 = apply_err(
        &docx,
        json!([{ "op": "deleteCustomProp", "id": "prop:不存在" }]),
    );
    assert_eq!(e6.code(), ErrorCode::IdNotFound);

    let e7 = apply_err(
        &docx,
        json!([{
            "op": "setProps",
            "id": "docx:document",
            "title": "x",
            "lastModifiedBy": "someone"
        }]),
    );
    assert_eq!(e7.code(), ErrorCode::InvalidField);
}
