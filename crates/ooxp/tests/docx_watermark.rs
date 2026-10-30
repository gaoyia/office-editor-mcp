//! docx 页眉 DrawingML 水印：计数、名单、插删、create；不进 pictures / textBoxes。

use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

static SEQ: AtomicU32 = AtomicU32::new(0);

const PNG_1X1: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
    0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08, 0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00,
    0x00, 0x03, 0x01, 0x01, 0x00, 0x18, 0xDD, 0x8D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E,
    0x44, 0xAE, 0x42, 0x60, 0x82,
];

const ROOT_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#;

const STYLES: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/></w:style></w:styles>"#;

const DOC: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><w:body><w:p><w:r><w:t>hello</w:t></w:r></w:p><w:sectPr><w:headerReference w:type="default" r:id="rId2"/></w:sectPr></w:body></w:document>"#;

const DOC_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/header" Target="header1.xml"/></Relationships>"#;

const CT: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/><Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/><Override PartName="/word/header1.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.header+xml"/></Types>"#;

const HEADER_WM: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:hdr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><w:p><w:r><w:drawing><wp:anchor behindDoc="1" distT="0" distB="0" distL="0" distR="0" simplePos="0" relativeHeight="1" locked="0" layoutInCell="1" allowOverlap="1"><wp:simplePos x="0" y="0"/><wp:positionH relativeFrom="page"><wp:align>center</wp:align></wp:positionH><wp:positionV relativeFrom="page"><wp:align>center</wp:align></wp:positionV><wp:extent cx="1" cy="1"/><wp:wrapNone/><wp:docPr id="7" name="PowerPlusWaterMarkObject1"/><a:graphic><a:graphicData uri="http://schemas.microsoft.com/office/word/2010/wordprocessingShape"/></a:graphic></wp:anchor></w:drawing></w:r></w:p></w:hdr>"#;

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-docx-watermark");
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
    inspect_scope(path, json!({}))
}

fn inspect_scope(path: &Path, scope: Value) -> Value {
    let RunOutput::Inspect(tree) = run_json(&json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": path.to_string_lossy(),
        "scope": scope
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

fn create_docx(tag: &str) -> PathBuf {
    let path = seq_path(tag, "docx");
    run_json(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "format": "docx",
        "document": {
            "paragraphs": [
                { "style": "Heading1", "text": "标题" },
                { "text": "hello" }
            ]
        }
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

fn zip_has_part(path: &Path, name: &str) -> bool {
    let bytes = std::fs::read(path).unwrap();
    let zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let found = zip.file_names().any(|n| n == name);
    found
}

fn write_png() -> PathBuf {
    let path = seq_path("wm", "png");
    std::fs::write(&path, PNG_1X1).unwrap();
    path
}

fn named_sample(tag: &str) -> PathBuf {
    let path = seq_path(tag, "docx");
    let mut buf = Vec::new();
    {
        let mut zip = ZipWriter::new(Cursor::new(&mut buf));
        let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        for (name, data) in [
            ("[Content_Types].xml", CT.as_bytes()),
            ("_rels/.rels", ROOT_RELS.as_bytes()),
            ("word/document.xml", DOC.as_bytes()),
            ("word/_rels/document.xml.rels", DOC_RELS.as_bytes()),
            ("word/styles.xml", STYLES.as_bytes()),
            ("word/header1.xml", HEADER_WM.as_bytes()),
        ] {
            zip.start_file(name, opts).unwrap();
            zip.write_all(data).unwrap();
        }
        zip.finish().unwrap();
    }
    std::fs::write(&path, buf).unwrap();
    path
}

#[test]
fn empty_create_has_zero_watermark() {
    let path = create_docx("empty");
    let tree = inspect(&path);
    assert_eq!(tree["watermarkCount"], 0);
    assert!(tree.get("watermarks").is_none());
}

#[test]
fn named_header_drawing_is_watermark_not_picture() {
    let path = named_sample("named");
    let tree = inspect(&path);
    assert_eq!(tree["watermarkCount"], 1);
    let listed = inspect_scope(&path, json!({ "watermarks": true, "pictures": true }));
    assert_eq!(listed["watermarkCount"], 1);
    let wms = listed["watermarks"].as_array().unwrap();
    assert_eq!(wms.len(), 1);
    assert_eq!(
        object_keys(&wms[0]),
        ["id", "kind", "name", "text", "stable"]
    );
    assert_eq!(wms[0]["id"], "header:default:1/watermark:7");
    assert_eq!(wms[0]["kind"], "text");
    assert_eq!(wms[0]["name"], "PowerPlusWaterMarkObject1");
    assert_eq!(wms[0]["stable"], true);
    assert_eq!(listed["headers"][0]["watermarkCount"], 1);
    assert_eq!(listed["headers"][0]["pictureCount"], 0);
    assert!(
        listed["headers"][0].get("pictures").is_none()
            || listed["headers"][0]["pictures"]
                .as_array()
                .unwrap()
                .is_empty()
    );
}

#[test]
fn insert_text_then_delete_all() {
    let src = create_docx("text-host");
    let on = seq_path("text-on", "docx");
    apply(
        &src,
        &on,
        json!([{
            "op": "insertWatermark",
            "id": "docx:document",
            "text": "CONFIDENTIAL"
        }]),
    );
    let tree = inspect(&on);
    assert_eq!(tree["watermarkCount"], 1);
    assert!(zip_has_part(&on, "word/header1.xml"));
    let listed = inspect_scope(
        &on,
        json!({ "watermarks": true, "pictures": true, "textBoxes": true }),
    );
    let wms = listed["watermarks"].as_array().unwrap();
    assert_eq!(wms.len(), 1);
    assert_eq!(wms[0]["kind"], "text");
    assert_eq!(wms[0]["text"], "CONFIDENTIAL");
    let id = wms[0]["id"].as_str().unwrap();
    assert!(id.starts_with("header:default:1/watermark:"), "{id}");
    assert_eq!(listed["headers"][0]["watermarkCount"], 1);
    assert_eq!(listed["headers"][0]["pictureCount"], 0);
    assert_eq!(listed["headers"][0]["textBoxCount"], 0);
    let hdr = part_text(&on, "word/header1.xml");
    assert!(hdr.contains("ooxpWatermarkText"), "{hdr}");
    assert!(
        hdr.contains("C0A5BD15-81BF-4B96-A0C4-0C0A5BD15001"),
        "{hdr}"
    );
    assert!(!hdr.contains("w:pict"), "{hdr}");
    assert!(!hdr.contains("mc:Fallback"), "{hdr}");

    let off = seq_path("text-off", "docx");
    apply(
        &on,
        &off,
        json!([{ "op": "deleteWatermark", "id": "docx:document" }]),
    );
    let tree2 = inspect(&off);
    assert_eq!(tree2["watermarkCount"], 0);
    let listed2 = inspect_scope(&off, json!({ "watermarks": true }));
    assert_eq!(listed2["watermarks"].as_array().unwrap().len(), 0);
}

#[test]
fn insert_picture_watermark() {
    let src = create_docx("pic-host");
    let png = write_png();
    let on = seq_path("pic-on", "docx");
    apply(
        &src,
        &on,
        json!([{
            "op": "insertWatermark",
            "id": "docx:document",
            "file": png.to_string_lossy()
        }]),
    );
    let listed = inspect_scope(&on, json!({ "watermarks": true, "pictures": true }));
    assert_eq!(listed["watermarkCount"], 1);
    assert_eq!(listed["watermarks"][0]["kind"], "picture");
    assert!(listed["watermarks"][0]["text"].is_null());
    assert_eq!(listed["headers"][0]["pictureCount"], 0);
    assert!(
        zip_has_part(&on, "word/media/image1.png") || zip_has_part(&on, "word/media/image1.jpeg")
    );
    let hdr = part_text(&on, "word/header1.xml");
    assert!(hdr.contains("ooxpWatermarkPicture"), "{hdr}");
}

#[test]
fn ordinary_header_picture_is_not_watermark() {
    let src = create_docx("plain-host");
    let png = write_png();
    let hf = seq_path("plain-hf", "docx");
    apply(
        &src,
        &hf,
        json!([{
            "op": "insertHeaderFooter",
            "id": "section:1",
            "kind": "header",
            "type": "default"
        }]),
    );
    let tree_hf = inspect_scope(&hf, json!({ "headerFooter": true }));
    let para_id = tree_hf["headers"][0]["paragraphs"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let out = seq_path("plain-pic", "docx");
    apply(
        &hf,
        &out,
        json!([{
            "op": "insertPicture",
            "id": para_id,
            "file": png.to_string_lossy()
        }]),
    );
    let listed = inspect_scope(&out, json!({ "pictures": true, "watermarks": true }));
    assert_eq!(listed["watermarkCount"], 0);
    assert_eq!(listed["headers"][0]["watermarkCount"], 0);
    assert!(listed["headers"][0]["pictureCount"].as_u64().unwrap() >= 1);
    let pics = listed["headers"][0]["pictures"].as_array().unwrap();
    assert!(!pics.is_empty());
    assert!(!pics[0]["name"]
        .as_str()
        .unwrap_or("")
        .starts_with("ooxpWatermark"));
}

#[test]
fn delete_one_by_id() {
    let src = create_docx("one-host");
    let on = seq_path("one-on", "docx");
    apply(
        &src,
        &on,
        json!([{
            "op": "insertWatermark",
            "id": "docx:document",
            "text": "DRAFT"
        }]),
    );
    let listed = inspect_scope(&on, json!({ "watermarks": true }));
    let id = listed["watermarks"][0]["id"].as_str().unwrap().to_string();
    let off = seq_path("one-off", "docx");
    apply(&on, &off, json!([{ "op": "deleteWatermark", "id": id }]));
    assert_eq!(inspect(&off)["watermarkCount"], 0);
}

#[test]
fn reject_bad_insert_and_missing_delete() {
    let src = create_docx("bad-host");
    let err_id = apply_err(
        &src,
        json!([{
            "op": "insertWatermark",
            "id": "para:1",
            "text": "X"
        }]),
    );
    assert_eq!(err_id.code(), ErrorCode::InvalidId);

    let png = write_png();
    let err_both = apply_err(
        &src,
        json!([{
            "op": "insertWatermark",
            "id": "docx:document",
            "text": "X",
            "file": png.to_string_lossy()
        }]),
    );
    assert_eq!(err_both.code(), ErrorCode::InvalidField);

    let err_none = apply_err(
        &src,
        json!([{ "op": "insertWatermark", "id": "docx:document" }]),
    );
    assert_eq!(err_none.code(), ErrorCode::InvalidField);

    let err_missing = apply_err(
        &src,
        json!([{
            "op": "deleteWatermark",
            "id": "header:default:1/watermark:99"
        }]),
    );
    assert_eq!(err_missing.code(), ErrorCode::IdNotFound);
}

#[test]
fn create_writes_last_watermark() {
    let path = seq_path("create-wm", "docx");
    run_json(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "format": "docx",
        "document": {
            "paragraphs": [{ "text": "hello" }],
            "watermarks": [
                { "text": "FIRST" },
                { "text": "LAST" }
            ]
        }
    }))
    .unwrap();
    let listed = inspect_scope(&path, json!({ "watermarks": true }));
    assert_eq!(listed["watermarkCount"], 1);
    assert_eq!(listed["watermarks"][0]["text"], "LAST");
}
