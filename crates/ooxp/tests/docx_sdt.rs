//! docx 内容控件：insertContentControl / setContentControl / deleteContentControl。

use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::write::SimpleFileOptions;
use zip::CompressionMethod;
use zip::ZipArchive;
use zip::ZipWriter;

static SEQ: AtomicU32 = AtomicU32::new(0);

/// 1×1 红 PNG。
const PNG_1X1: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
    0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08, 0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00,
    0x00, 0x03, 0x01, 0x01, 0x00, 0x18, 0xDD, 0x8D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E,
    0x44, 0xAE, 0x42, 0x60, 0x82,
];

/// 另一份 1×1 PNG，用来换图片控件。
const PNG_1X1_B: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
    0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08, 0xD7, 0x63, 0x60, 0x60, 0x60, 0x00,
    0x00, 0x00, 0x04, 0x00, 0x01, 0xA3, 0x0A, 0x15, 0xE3, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E,
    0x44, 0xAE, 0x42, 0x60, 0x82,
];

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
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "format": "docx",
        "document": document
    })
    .to_string())
    .unwrap();
    path
}

fn inspect(path: &Path, content_controls: bool, header_footer: bool) -> Value {
    let RunOutput::Inspect(tree) = run(&json!({
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
    .to_string())
    .unwrap() else {
        panic!("inspect 应返回树");
    };
    tree
}

fn apply(path: &Path, out: &Path, ops: Value) {
    run(&json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": path.to_string_lossy(),
        "output": out.to_string_lossy(),
        "ops": ops
    })
    .to_string())
    .unwrap();
}

fn apply_err(path: &Path, ops: Value) -> ooxp::Error {
    let out = seq_path("should-not-write", "docx");
    let _ = std::fs::remove_file(&out);
    run(&json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": path.to_string_lossy(),
        "output": out.to_string_lossy(),
        "ops": ops
    })
    .to_string())
    .unwrap_err()
}

fn first_body_para_id(tree: &Value) -> String {
    tree["paragraphs"][0]["id"].as_str().unwrap().to_string()
}

fn object_keys(v: &Value) -> Vec<&str> {
    v.as_object().unwrap().keys().map(|k| k.as_str()).collect()
}

fn write_png(tag: &str, bytes: &[u8]) -> PathBuf {
    let path = seq_path(tag, "png");
    std::fs::write(&path, bytes).unwrap();
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

fn part_bytes(path: &Path, name: &str) -> Vec<u8> {
    let bytes = std::fs::read(path).unwrap();
    let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut file = zip.by_name(name.trim_start_matches('/')).unwrap();
    let mut out = Vec::new();
    file.read_to_end(&mut out).unwrap();
    out
}

fn zip_has(path: &Path, name: &str) -> bool {
    let bytes = std::fs::read(path).unwrap();
    let zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let needle = name.trim_start_matches('/');
    let names: Vec<String> = zip.file_names().map(|s| s.to_string()).collect();
    names.iter().any(|n| n == needle)
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
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": xlsx.to_string_lossy(),
        "workbook": { "sheets": [{ "name": "Sheet1", "cells": [{ "ref": "A1", "value": 1 }] }] }
    })
    .to_string())
    .unwrap();
    let xout = seq_path("sdt-x-out", "xlsx");
    let err_x = run(&json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": xlsx.to_string_lossy(),
        "output": xout.to_string_lossy(),
        "ops": [{ "op": "insertContentControl", "id": "docx:document", "text": "x" }]
    })
    .to_string())
    .unwrap_err();
    assert_eq!(err_x.code(), ErrorCode::InvalidField);

    let err_scope = run(&json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": xlsx.to_string_lossy(),
        "scope": { "sheet": "Sheet1", "range": "A1", "contentControls": true }
    })
    .to_string())
    .unwrap_err();
    assert_eq!(err_scope.code(), ErrorCode::InvalidField);

    let pptx = seq_path("sdt-p", "pptx");
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": pptx.to_string_lossy(),
        "presentation": { "slides": [{ "shapes": [{ "text": "a" }] }] }
    })
    .to_string())
    .unwrap();
    let pout = seq_path("sdt-p-out", "pptx");
    let err_p = run(&json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": pptx.to_string_lossy(),
        "output": pout.to_string_lossy(),
        "ops": [{ "op": "deleteContentControl", "id": "sdt:1" }]
    })
    .to_string())
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

#[test]
fn insert_checkbox_picture_group_equation() {
    let png_a = write_png("sdt-a", PNG_1X1);
    let png_b = write_png("sdt-b", PNG_1X1_B);
    let src = create_docx("sdt-types-src", json!({ "paragraphs": [{ "text": "甲" }] }));
    let p1 = first_body_para_id(&inspect(&src, false, false));
    let out = seq_path("sdt-types", "docx");
    apply(
        &src,
        &out,
        json!([
            { "op": "insertContentControl", "id": p1, "kind": "checkbox", "checked": true },
            { "op": "insertContentControl", "id": p1, "kind": "picture", "file": png_a },
            { "op": "insertContentControl", "id": p1, "kind": "group", "text": "组" },
            { "op": "insertContentControl", "id": p1, "kind": "equation", "text": "x" }
        ]),
    );
    let tree = inspect(&out, true, false);
    assert_eq!(tree["contentControlCount"], 4);
    assert_eq!(
        object_keys(&tree["contentControls"][0]),
        [
            "id", "index", "kind", "level", "host", "tag", "alias", "xmlId", "bound", "parent",
            "text", "checked"
        ]
    );
    assert_eq!(tree["contentControls"][0]["kind"], "checkbox");
    assert_eq!(tree["contentControls"][0]["checked"], true);
    assert_eq!(tree["contentControls"][0]["text"], "☒");
    assert_eq!(tree["contentControls"][1]["kind"], "picture");
    assert_eq!(tree["contentControls"][2]["kind"], "group");
    assert_eq!(tree["contentControls"][2]["text"], "组");
    assert_eq!(tree["contentControls"][3]["kind"], "equation");
    assert_eq!(tree["contentControls"][3]["text"], "x");
    let xml = part_text(&out, "word/document.xml");
    assert!(xml.contains("w14:checkbox"));
    assert!(xml.contains("<w:picture/>") || xml.contains("<w:picture "));
    assert!(xml.contains("<w:group/>") || xml.contains("<w:group "));
    assert!(xml.contains("<w:equation/>") || xml.contains("<w:equation "));
    assert_eq!(part_bytes(&out, "word/media/image1.png"), PNG_1X1);

    let RunOutput::Inspect(pic_tree) = run(&json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": out.to_string_lossy(),
        "scope": { "from": 1, "maxParas": 20, "pictures": true }
    })
    .to_string())
    .unwrap() else {
        panic!("inspect 应返回树");
    };
    assert!(pic_tree["pictureCount"].as_u64().unwrap() >= 1);

    let set_cb = seq_path("sdt-cb-set", "docx");
    apply(
        &out,
        &set_cb,
        json!([{ "op": "setContentControl", "id": "sdt:1", "checked": false }]),
    );
    let tree_cb = inspect(&set_cb, true, false);
    assert_eq!(tree_cb["contentControls"][0]["checked"], false);
    assert_eq!(tree_cb["contentControls"][0]["text"], "☐");

    let set_pic = seq_path("sdt-pic-set", "docx");
    apply(
        &out,
        &set_pic,
        json!([{
            "op": "setContentControl",
            "id": "sdt:2",
            "file": png_b
        }]),
    );
    assert_eq!(part_bytes(&set_pic, "word/media/image2.png"), PNG_1X1_B);
    assert!(!zip_has(&set_pic, "word/media/image1.png"));

    let set_eq = seq_path("sdt-eq-set", "docx");
    apply(
        &out,
        &set_eq,
        json!([{ "op": "setContentControl", "id": "sdt:4", "text": "y" }]),
    );
    assert_eq!(
        inspect(&set_eq, true, false)["contentControls"][3]["text"],
        "y"
    );

    let err_group = apply_err(
        &out,
        json!([{ "op": "setContentControl", "id": "sdt:3", "text": "改" }]),
    );
    assert_eq!(err_group.code(), ErrorCode::NotImplemented);

    let err_pic = apply_err(
        &src,
        json!([{ "op": "insertContentControl", "id": p1, "kind": "picture" }]),
    );
    assert_eq!(err_pic.code(), ErrorCode::InvalidField);

    let err_pic_text = apply_err(
        &src,
        json!([{
            "op": "insertContentControl",
            "id": p1,
            "kind": "picture",
            "file": png_a,
            "text": "x"
        }]),
    );
    assert_eq!(err_pic_text.code(), ErrorCode::InvalidField);
}

#[test]
fn create_sdt_types_and_handmade_inspect() {
    let png = write_png("sdt-create", PNG_1X1);
    let path = create_docx(
        "sdt-create-types",
        json!({
            "paragraphs": [{
                "text": "段",
                "contentControls": [
                    { "kind": "checkbox", "checked": true },
                    { "kind": "picture", "file": png },
                    { "kind": "group", "text": "组" },
                    { "kind": "equation", "text": "x" }
                ]
            }]
        }),
    );
    let tree = inspect(&path, true, false);
    assert_eq!(tree["contentControlCount"], 4);
    assert_eq!(tree["contentControls"][0]["kind"], "checkbox");
    assert_eq!(tree["contentControls"][0]["checked"], true);
    assert_eq!(tree["contentControls"][1]["kind"], "picture");
    assert_eq!(tree["contentControls"][2]["kind"], "group");
    assert_eq!(tree["contentControls"][3]["kind"], "equation");
    let xml = part_text(&path, "word/document.xml");
    assert!(xml.contains("<w:equation/>") || xml.contains("<w:equation "));

    let err = run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": seq_path("sdt-create-bad", "docx").to_string_lossy(),
        "format": "docx",
        "document": {
            "paragraphs": [{
                "text": "段",
                "contentControls": [{ "kind": "picture" }]
            }]
        }
    })
    .to_string())
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let handmade = write_docx_xml(
        "sdt-kinds",
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml"><w:body><w:p w14:paraId="AAAA0001"><w:sdt><w:sdtPr><w:id w:val="1"/><w14:checkbox><w14:checked w14:val="0"/><w14:checkedState w14:val="2612" w14:font="MS Gothic"/><w14:uncheckedState w14:val="2610" w14:font="MS Gothic"/></w14:checkbox></w:sdtPr><w:sdtContent><w:r><w:t>☐</w:t></w:r></w:sdtContent></w:sdt></w:p><w:p w14:paraId="AAAA0002"><w:sdt><w:sdtPr><w:id w:val="2"/><w:picture/></w:sdtPr><w:sdtContent><w:r><w:t></w:t></w:r></w:sdtContent></w:sdt></w:p><w:p w14:paraId="AAAA0003"><w:sdt><w:sdtPr><w:id w:val="3"/><w:group/></w:sdtPr><w:sdtContent><w:r><w:t>组</w:t></w:r></w:sdtContent></w:sdt></w:p><w:p w14:paraId="AAAA0004"><w:sdt><w:sdtPr><w:id w:val="4"/><w:equation/></w:sdtPr><w:sdtContent><w:r><w:t>x</w:t></w:r></w:sdtContent></w:sdt></w:p><w:sectPr><w:pgSz w:w="11906" w:h="16838"/></w:sectPr></w:body></w:document>"#,
    );
    let kinds = inspect(&handmade, true, false);
    assert_eq!(kinds["contentControlCount"], 4);
    assert_eq!(kinds["contentControls"][0]["kind"], "checkbox");
    assert_eq!(kinds["contentControls"][0]["checked"], false);
    assert_eq!(kinds["contentControls"][1]["kind"], "picture");
    assert_eq!(kinds["contentControls"][2]["kind"], "group");
    assert_eq!(kinds["contentControls"][3]["kind"], "equation");
}
