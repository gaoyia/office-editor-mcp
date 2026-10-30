//! docx 插删节：create 多节、insertSection、deleteSection、xlsx/pptx 拒绝。

use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::ZipArchive;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-docx-insert-section");
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

fn inspect(path: &Path) -> Value {
    let RunOutput::Inspect(tree) = run(&json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": path.to_string_lossy(),
        "scope": { "from": 1, "maxParas": 20, "headerFooter": true }
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

fn part_text(path: &Path, name: &str) -> String {
    let file = std::fs::File::open(path).unwrap();
    let mut zip = ZipArchive::new(file).unwrap();
    let mut part = zip.by_name(name).unwrap();
    let mut s = String::new();
    part.read_to_string(&mut s).unwrap();
    s
}

fn zip_has_part(path: &Path, name: &str) -> bool {
    let bytes = std::fs::read(path).unwrap();
    let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let ok = zip.by_name(name).is_ok();
    ok
}

fn first_body_para_id(tree: &Value) -> String {
    tree["paragraphs"][0]["id"].as_str().unwrap().to_string()
}

#[test]
fn create_two_sections() {
    let path = create_docx(
        "create-two",
        json!({
            "paragraphs": [
                { "text": "第一节", "sectionType": "nextPage" },
                { "text": "第二节" }
            ]
        }),
    );
    let tree = inspect(&path);
    assert_eq!(tree["sections"].as_array().unwrap().len(), 2);
    assert_eq!(tree["sections"][0]["headerLink"]["default"], false);
    assert_eq!(tree["sections"][1]["headerLink"]["default"], true);
    let xml = part_text(&path, "word/document.xml");
    assert!(xml.contains(r#"<w:type w:val="nextPage"/>"#));
    assert!(xml.contains("第一节"));
    let last = xml.rfind("<w:sectPr>").unwrap();
    let first_type = xml.find(r#"<w:type w:val="nextPage"/>"#).unwrap();
    assert!(first_type < last);
}

#[test]
fn create_section_type_on_last_para_rejected() {
    let path = seq_path("last-type", "docx");
    let err = run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "format": "docx",
        "document": {
            "paragraphs": [
                { "text": "仅一段", "sectionType": "nextPage" }
            ]
        }
    })
    .to_string())
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

#[test]
fn create_headers_hang_on_first_section() {
    let path = create_docx(
        "create-hf",
        json!({
            "paragraphs": [
                { "text": "第一节", "sectionType": "continuous" },
                { "text": "第二节" }
            ],
            "headers": [{ "type": "default", "paragraphs": [{ "text": "页眉" }] }]
        }),
    );
    let tree = inspect(&path);
    assert_eq!(tree["sections"].as_array().unwrap().len(), 2);
    assert_eq!(tree["sections"][0]["headerLink"]["default"], false);
    assert_eq!(tree["sections"][1]["headerLink"]["default"], true);
    let xml = part_text(&path, "word/document.xml");
    assert_eq!(xml.matches("headerReference").count(), 1);
    let href = xml.find("headerReference").unwrap();
    let body_sect = xml.rfind("<w:sectPr>").unwrap();
    assert!(href < body_sect);
    assert!(!xml[body_sect..].contains("headerReference"));
}

#[test]
fn insert_section_after_first_para() {
    let src = create_docx(
        "ins-src",
        json!({ "paragraphs": [{ "text": "甲" }, { "text": "乙" }] }),
    );
    let tree0 = inspect(&src);
    assert_eq!(tree0["sections"].as_array().unwrap().len(), 1);
    let p1 = first_body_para_id(&tree0);
    let out = seq_path("ins-out", "docx");
    apply(
        &src,
        &out,
        json!([{ "op": "insertSection", "id": p1, "type": "nextPage" }]),
    );
    let tree = inspect(&out);
    assert_eq!(tree["sections"].as_array().unwrap().len(), 2);
    assert_eq!(tree["sections"][1]["headerLink"]["default"], true);
    let xml = part_text(&out, "word/document.xml");
    assert!(xml.contains(r#"w:val="nextPage""#));
}

#[test]
fn insert_section_at_document_end() {
    let src = create_docx(
        "end-src",
        json!({ "paragraphs": [{ "text": "甲" }, { "text": "乙" }] }),
    );
    let out = seq_path("end-out", "docx");
    apply(
        &src,
        &out,
        json!([{ "op": "insertSection", "id": "docx:document" }]),
    );
    let tree = inspect(&out);
    assert_eq!(tree["sections"].as_array().unwrap().len(), 2);
    assert_eq!(tree["paragraphs"].as_array().unwrap().len(), 3);
}

#[test]
fn insert_on_existing_break_adds_empty_section() {
    let src = create_docx(
        "exist-src",
        json!({
            "paragraphs": [
                { "text": "一", "sectionType": "nextPage" },
                { "text": "二" }
            ]
        }),
    );
    let tree0 = inspect(&src);
    let p1 = first_body_para_id(&tree0);
    let out = seq_path("exist-out", "docx");
    apply(
        &src,
        &out,
        json!([{ "op": "insertSection", "id": p1, "type": "continuous" }]),
    );
    let tree = inspect(&out);
    assert_eq!(tree["sections"].as_array().unwrap().len(), 3);
}

#[test]
fn delete_section_merges_into_next() {
    let src = create_docx(
        "del-src",
        json!({
            "paragraphs": [
                { "text": "一", "sectionType": "nextPage" },
                { "text": "二" }
            ]
        }),
    );
    let out = seq_path("del-out", "docx");
    apply(
        &src,
        &out,
        json!([{ "op": "deleteSection", "id": "section:1" }]),
    );
    let tree = inspect(&out);
    assert_eq!(tree["sections"].as_array().unwrap().len(), 1);
    assert_eq!(tree["paragraphs"].as_array().unwrap().len(), 2);
}

#[test]
fn cannot_delete_last_section() {
    let src = create_docx(
        "last-src",
        json!({ "paragraphs": [{ "text": "甲" }, { "text": "乙" }] }),
    );
    let err = apply_err(&src, json!([{ "op": "deleteSection", "id": "section:1" }]));
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let two = create_docx(
        "last-two",
        json!({
            "paragraphs": [
                { "text": "一", "sectionType": "nextPage" },
                { "text": "二" }
            ]
        }),
    );
    let err2 = apply_err(&two, json!([{ "op": "deleteSection", "id": "section:2" }]));
    assert_eq!(err2.code(), ErrorCode::InvalidField);
}

#[test]
fn insert_section_rejects_bad_id_and_type() {
    let src = create_docx(
        "bad-src",
        json!({
            "paragraphs": [{ "text": "甲" }],
            "headers": [{ "type": "default", "paragraphs": [{ "text": "页眉" }] }]
        }),
    );
    let tree = inspect(&src);
    let hf_para = tree["headers"][0]["paragraphs"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let err_hf = apply_err(&src, json!([{ "op": "insertSection", "id": hf_para }]));
    assert_eq!(err_hf.code(), ErrorCode::InvalidId);

    let err_sec = apply_err(&src, json!([{ "op": "insertSection", "id": "section:1" }]));
    assert_eq!(err_sec.code(), ErrorCode::InvalidId);

    let p1 = first_body_para_id(&tree);
    let err_ty = apply_err(
        &src,
        json!([{ "op": "insertSection", "id": p1, "type": "nope" }]),
    );
    assert_eq!(err_ty.code(), ErrorCode::InvalidField);
}

#[test]
fn delete_section_drops_orphan_header() {
    let src = create_docx(
        "orphan-src",
        json!({
            "paragraphs": [
                { "text": "一", "sectionType": "nextPage" },
                { "text": "二" }
            ],
            "headers": [{ "type": "default", "paragraphs": [{ "text": "页眉" }] }]
        }),
    );
    let mid = seq_path("orphan-mid", "docx");
    apply(
        &src,
        &mid,
        json!([{
            "op": "setHeaderFooterLink",
            "id": "section:2",
            "kind": "header",
            "type": "default",
            "linked": false
        }]),
    );
    assert!(zip_has_part(&mid, "word/header1.xml"));
    assert!(zip_has_part(&mid, "word/header2.xml"));
    let out = seq_path("orphan-out", "docx");
    apply(
        &mid,
        &out,
        json!([{ "op": "deleteSection", "id": "section:1" }]),
    );
    let tree = inspect(&out);
    assert_eq!(tree["sections"].as_array().unwrap().len(), 1);
    assert!(zip_has_part(&out, "word/header2.xml"));
    assert!(!zip_has_part(&out, "word/header1.xml"));
}

#[test]
fn section_ops_not_for_xlsx_or_pptx() {
    let xlsx = seq_path("sec-x", "xlsx");
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": xlsx.to_string_lossy(),
        "workbook": { "sheets": [{ "name": "Sheet1" }] }
    })
    .to_string())
    .unwrap();
    let xout = seq_path("sec-x-out", "xlsx");
    let err_x = run(&json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": xlsx.to_string_lossy(),
        "output": xout.to_string_lossy(),
        "ops": [{ "op": "insertSection", "id": "docx:document" }]
    })
    .to_string())
    .unwrap_err();
    assert_eq!(err_x.code(), ErrorCode::InvalidField);

    let pptx = seq_path("sec-p", "pptx");
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": pptx.to_string_lossy(),
        "presentation": { "slides": [{ "shapes": [{ "text": "a" }] }] }
    })
    .to_string())
    .unwrap();
    let pout = seq_path("sec-p-out", "pptx");
    let err_p = run(&json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": pptx.to_string_lossy(),
        "output": pout.to_string_lossy(),
        "ops": [{ "op": "deleteSection", "id": "section:1" }]
    })
    .to_string())
    .unwrap_err();
    assert_eq!(err_p.code(), ErrorCode::InvalidField);
}
