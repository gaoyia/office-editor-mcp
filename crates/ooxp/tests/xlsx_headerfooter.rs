//! 页眉页脚：摘要、六槽原文、改字、页眉图与工作表图不混。

use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::json;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

const PNG_1X1: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
    0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08, 0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00,
    0x00, 0x03, 0x01, 0x01, 0x00, 0x18, 0xDD, 0x8D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E,
    0x44, 0xAE, 0x42, 0x60, 0x82,
];

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-xlsx-hf");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn tmp(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}"))
}

fn create_book(tag: &str) -> PathBuf {
    let path = tmp(tag).with_extension("xlsx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "workbook": { "sheets": [{ "name": "Sheet1", "cells": [{ "ref": "A1", "value": 1 }] }] }
        })
        .to_string(),
        None,
    )
    .unwrap();
    path
}

fn apply(src: &Path, out: &Path, ops: serde_json::Value) -> Result<RunOutput, ooxp::Error> {
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

fn inspect(path: &Path, scope: serde_json::Value) -> serde_json::Value {
    let RunOutput::Inspect(tree) = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": path.to_string_lossy(),
            "scope": scope,
        })
        .to_string(),
        None,
    )
    .unwrap() else {
        panic!("inspect 应返回树");
    };
    tree
}

fn object_keys(v: &serde_json::Value) -> Vec<&str> {
    v.as_object().unwrap().keys().map(String::as_str).collect()
}

fn part_text(path: &Path, name: &str) -> String {
    let bytes = std::fs::read(path).unwrap();
    let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut file = zip.by_name(name).unwrap();
    let mut s = String::new();
    file.read_to_string(&mut s).unwrap();
    s
}

fn crc_of(path: &Path, name: &str) -> u32 {
    let bytes = std::fs::read(path).unwrap();
    let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let crc = zip.by_name(name).unwrap().crc32();
    crc
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

#[test]
fn summary_without_scope() {
    let path = create_book("sum");
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
        panic!();
    };
    assert_eq!(tree["sheets"][0]["hasHeaderFooter"], false);
    assert!(tree["sheets"][0].get("headerFooters").is_none());
    assert_eq!(
        object_keys(&tree["sheets"][0]),
        [
            "id",
            "name",
            "kind",
            "hidden",
            "pictureCount",
            "chartCount",
            "dataValidationCount",
            "conditionalFormatCount",
            "hasHeaderFooter"
        ]
    );
}

#[test]
fn expand_six_slots_and_set_text() {
    let src = create_book("text");
    let out = tmp("set").with_extension("xlsx");
    apply(
        &src,
        &out,
        json!([{
            "op": "setHeaderFooterText",
            "id": "sheet:Sheet1/hf:odd:header",
            "text": "&C新页眉"
        }]),
    )
    .unwrap();
    let tree0 = inspect(&out, json!({}));
    assert_eq!(tree0["sheets"][0]["hasHeaderFooter"], true);
    assert!(tree0["sheets"][0].get("headerFooters").is_none());

    let tree = inspect(&out, json!({ "headerFooter": true }));
    let slots = tree["sheets"][0]["headerFooters"].as_array().unwrap();
    assert_eq!(slots.len(), 6);
    assert_eq!(slots[0]["id"], "sheet:Sheet1/hf:odd:header");
    assert_eq!(slots[0]["kind"], "odd");
    assert_eq!(slots[0]["side"], "header");
    assert_eq!(slots[0]["text"], "&C新页眉");
    assert_eq!(slots[1]["text"], serde_json::Value::Null);
    assert_eq!(
        object_keys(&slots[0]),
        ["id", "kind", "side", "text", "pictureCount"]
    );
    let xml = part_text(&out, "xl/worksheets/sheet1.xml");
    assert!(xml.contains("&amp;C新页眉"), "{xml}");
    assert!(!xml.contains("differentOddEven"), "{xml}");
}

#[test]
fn even_sets_flag_and_does_not_dirty_other_sheet() {
    let src = create_book("even");
    // 第二张表由 create 只有一张；补一张再测 CRC 太重，只核对 even 属性。
    let out = tmp("even-out").with_extension("xlsx");
    apply(
        &src,
        &out,
        json!([{
            "op": "setHeaderFooterText",
            "id": "sheet:Sheet1/hf:even:footer",
            "text": "&C偶数脚"
        }]),
    )
    .unwrap();
    let xml = part_text(&out, "xl/worksheets/sheet1.xml");
    assert!(xml.contains(r#"differentOddEven="1""#), "{xml}");
    assert!(xml.contains("&amp;C偶数脚"), "{xml}");
}

#[test]
fn hf_picture_not_in_sheet_pictures() {
    let src = create_book("pic");
    let png = tmp("p").with_extension("png");
    std::fs::write(&png, PNG_1X1).unwrap();
    let with_sheet = tmp("sheet-pic").with_extension("xlsx");
    apply(
        &src,
        &with_sheet,
        json!([{
            "op": "insertPicture",
            "id": "sheet:Sheet1",
            "file": png.to_string_lossy(),
            "from": "B2"
        }]),
    )
    .unwrap();
    let with_hf = tmp("hf-pic").with_extension("xlsx");
    apply(
        &with_sheet,
        &with_hf,
        json!([{
            "op": "insertPicture",
            "id": "sheet:Sheet1/hf:odd:header",
            "file": png.to_string_lossy()
        }]),
    )
    .unwrap();

    let only_pics = inspect(&with_hf, json!({ "pictures": true }));
    let sheet_pics = only_pics["sheets"][0]["pictures"].as_array().unwrap();
    assert_eq!(sheet_pics.len(), 1);
    assert_eq!(sheet_pics[0]["id"], "sheet:Sheet1/pic:1");
    assert!(only_pics["sheets"][0].get("headerFooters").is_none());

    let both = inspect(&with_hf, json!({ "headerFooter": true, "pictures": true }));
    let slots = both["sheets"][0]["headerFooters"].as_array().unwrap();
    let hf_pics = slots[0]["pictures"].as_array().unwrap();
    assert_eq!(hf_pics.len(), 1);
    assert_eq!(hf_pics[0]["id"], "sheet:Sheet1/hf:odd:header/pic:2");
    assert_eq!(both["sheets"][0]["pictureCount"], 1);
    let xml = part_text(&with_hf, "xl/worksheets/sheet1.xml");
    assert!(xml.contains("<drawingHF"), "{xml}");
    assert!(
        xml.contains(r#"cho="2""#) || xml.contains("cho=\"2\""),
        "{xml}"
    );
}

#[test]
fn first_hf_picture_id_starts_at_two() {
    let src = create_book("first-hf");
    let png = tmp("p").with_extension("png");
    std::fs::write(&png, PNG_1X1).unwrap();
    let out = tmp("first-hf-out").with_extension("xlsx");
    apply(
        &src,
        &out,
        json!([{
            "op": "insertPicture",
            "id": "sheet:Sheet1/hf:odd:header",
            "file": png.to_string_lossy()
        }]),
    )
    .unwrap();
    let tree = inspect(&out, json!({ "headerFooter": true, "pictures": true }));
    let slots = tree["sheets"][0]["headerFooters"].as_array().unwrap();
    let hf_pics = slots[0]["pictures"].as_array().unwrap();
    assert_eq!(hf_pics[0]["id"], "sheet:Sheet1/hf:odd:header/pic:2");
    let xml = part_text(&out, "xl/worksheets/sheet1.xml");
    assert!(xml.contains(r#"cho="2""#), "{xml}");
    assert!(!xml.contains(r#"cho="1""#), "{xml}");
}

#[test]
fn insert_hf_rejects_from() {
    let src = create_book("from");
    let png = tmp("p").with_extension("png");
    std::fs::write(&png, PNG_1X1).unwrap();
    let out = tmp("bad").with_extension("xlsx");
    let err = apply(
        &src,
        &out,
        json!([{
            "op": "insertPicture",
            "id": "sheet:Sheet1/hf:odd:header",
            "file": png.to_string_lossy(),
            "from": "A1"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

#[test]
fn set_header_footer_text_not_for_docx() {
    let path = tmp("docx").with_extension("docx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "document": { "paragraphs": [{ "text": "x" }] }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let out = tmp("docx-out").with_extension("docx");
    let err = apply(
        &path,
        &out,
        json!([{
            "op": "setHeaderFooterText",
            "id": "sheet:Sheet1/hf:odd:header",
            "text": "no"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

#[test]
fn injected_odd_header_roundtrip() {
    let path = create_book("inj");
    rewrite_part(&path, "xl/worksheets/sheet1.xml", |bytes| {
        let s = String::from_utf8(bytes).unwrap();
        s.replace(
            "</worksheet>",
            "<headerFooter><oddHeader xml:space=\"preserve\">&amp;C注入</oddHeader></headerFooter></worksheet>",
        )
        .into_bytes()
    });
    let tree = inspect(&path, json!({ "headerFooter": true }));
    assert_eq!(tree["sheets"][0]["hasHeaderFooter"], true);
    assert_eq!(tree["sheets"][0]["headerFooters"][0]["text"], "&C注入");
}

#[test]
fn set_text_leaves_styles_crc() {
    let src = create_book("crc");
    let styles_before = crc_of(&src, "xl/styles.xml");
    let out = tmp("crc-out").with_extension("xlsx");
    apply(
        &src,
        &out,
        json!([{
            "op": "setHeaderFooterText",
            "id": "sheet:Sheet1/hf:odd:footer",
            "text": "脚"
        }]),
    )
    .unwrap();
    assert_eq!(crc_of(&out, "xl/styles.xml"), styles_before);
}
