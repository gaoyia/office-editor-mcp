//! docx 嵌入对象：计数、名单、导出、插删、create；预览图不进 pictures。

use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

const ROOT_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#;

const STYLES: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/></w:style></w:styles>"#;

const DOC_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/oleObject" Target="embeddings/Microsoft_Excel_Worksheet1.xlsx"/><Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/image1.png"/></Relationships>"#;

const DOC_RELS_PACKAGE: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/package" Target="embeddings/Microsoft_Excel_Worksheet1.xlsx"/><Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/image1.png"/></Relationships>"#;

const CT: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Default Extension="xlsx" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"/><Default Extension="png" ContentType="image/png"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/><Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/></Types>"#;

const PNG_1X1: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
    0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08, 0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00,
    0x00, 0x03, 0x01, 0x01, 0x00, 0x18, 0xDD, 0x8D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E,
    0x44, 0xAE, 0x42, 0x60, 0x82,
];

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-docx-ole");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_zip(tag: &str, files: &[(&str, &[u8])]) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let path = tmp_dir().join(format!("{tag}-{n}.docx"));
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut zip = ZipWriter::new(&mut cursor);
        let opt = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        for (name, body) in files {
            zip.start_file(*name, opt).unwrap();
            zip.write_all(body).unwrap();
        }
        zip.finish().unwrap();
    }
    std::fs::write(&path, cursor.into_inner()).unwrap();
    path
}

fn embed_xlsx() -> Vec<u8> {
    let path = tmp_dir().join(format!(
        "embed-{}.xlsx",
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "workbook": {
                "sheets": [{
                    "name": "Sheet1",
                    "cells": [{ "ref": "A1", "value": "嵌入" }]
                }]
            }
        })
        .to_string(),
        None,
    )
    .unwrap();
    std::fs::read(&path).unwrap()
}

fn sample_doc() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:o="urn:schemas-microsoft-com:office:office" xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture" xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml"><w:body><w:p w14:paraId="AAAA0001"><w:r><w:object w:dxaOrig="1440" w:dyaOrig="1440"><w:drawing><wp:inline><wp:extent cx="1" cy="1"/><wp:docPr id="9" name="OLE 预览"/><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/picture"><pic:pic><pic:blipFill><a:blip r:embed="rId3"/></pic:blipFill></pic:pic></a:graphicData></a:graphic></wp:inline></w:drawing><o:OLEObject Type="Embed" ProgID="Excel.Sheet.12" DrawAspect="Content" ObjectID="_1797" r:id="rId2"/></w:object></w:r></w:p><w:sectPr/></w:body></w:document>"#.into()
}

fn write_sample(tag: &str) -> (PathBuf, Vec<u8>) {
    write_sample_rels(tag, DOC_RELS.as_bytes())
}

fn write_sample_package_rel(tag: &str) -> (PathBuf, Vec<u8>) {
    write_sample_rels(tag, DOC_RELS_PACKAGE.as_bytes())
}

fn write_sample_rels(tag: &str, rels: &[u8]) -> (PathBuf, Vec<u8>) {
    let xlsx = embed_xlsx();
    let doc = sample_doc();
    let path = write_zip(
        tag,
        &[
            ("[Content_Types].xml", CT.as_bytes()),
            ("_rels/.rels", ROOT_RELS.as_bytes()),
            ("word/document.xml", doc.as_bytes()),
            ("word/_rels/document.xml.rels", rels),
            ("word/styles.xml", STYLES.as_bytes()),
            ("word/embeddings/Microsoft_Excel_Worksheet1.xlsx", &xlsx),
            ("word/media/image1.png", PNG_1X1),
        ],
    );
    (path, xlsx)
}

fn create_docx(tag: &str) -> PathBuf {
    let path = tmp_dir().join(format!(
        "{tag}-{}.docx",
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "format": "docx",
            "document": {
                "paragraphs": [{ "text": "宿主" }]
            }
        })
        .to_string(),
        None,
    )
    .unwrap();
    path
}

fn inspect(path: &Path, scope: Value) -> Value {
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
    let out = tmp_dir().join(format!(
        "should-not-write-{}.docx",
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
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

fn object_keys(v: &Value) -> Vec<&str> {
    v.as_object().unwrap().keys().map(String::as_str).collect()
}

fn part_bytes(path: &Path, name: &str) -> Vec<u8> {
    let bytes = std::fs::read(path).unwrap();
    let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut file = zip.by_name(name.trim_start_matches('/')).unwrap();
    let mut buf = Vec::new();
    file.read_to_end(&mut buf).unwrap();
    buf
}

fn zip_names(path: &Path) -> Vec<String> {
    let bytes = std::fs::read(path).unwrap();
    let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut names = Vec::new();
    for i in 0..zip.len() {
        names.push(zip.by_index(i).unwrap().name().replace('\\', "/"));
    }
    names
}

fn write_xlsx(tag: &str) -> PathBuf {
    let path = tmp_dir().join(format!(
        "{tag}-{}.xlsx",
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
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

#[test]
fn empty_create_has_count_without_list() {
    let path = create_docx("empty");
    let tree = inspect(&path, json!({}));
    assert_eq!(tree["oleObjectCount"], 0);
    assert!(tree.get("oleObjects").is_none());
}

#[test]
fn inspect_lists_word_style_object_and_skips_preview_picture() {
    let (path, _) = write_sample("list");
    let tree = inspect(&path, json!({}));
    assert_eq!(tree["oleObjectCount"], 1);
    assert!(tree.get("oleObjects").is_none());
    assert_eq!(tree["pictureCount"], 0);

    let listed = inspect(&path, json!({ "oleObjects": true }));
    assert_eq!(listed["oleObjectCount"], 1);
    let items = listed["oleObjects"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    let o = &items[0];
    assert_eq!(o["id"], "para:AAAA0001/ole:_1797");
    assert_eq!(o["progId"], "Excel.Sheet.12");
    assert_eq!(o["name"], "OLE 预览");
    assert_eq!(o["kind"], "embed");
    assert_eq!(
        o["contentType"],
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
    );
    assert_eq!(o["stable"], true);
    assert_eq!(
        object_keys(o),
        ["id", "progId", "name", "kind", "contentType", "stable"]
    );
    assert_eq!(listed["pictureCount"], 0);
}

#[test]
fn inspect_lists_word_package_relationship() {
    let (path, _) = write_sample_package_rel("pkg-rel");
    let tree = inspect(&path, json!({ "oleObjects": true }));
    assert_eq!(tree["oleObjectCount"], 1);
    assert_eq!(tree["oleObjects"][0]["progId"], "Excel.Sheet.12");
    assert_eq!(tree["oleObjects"][0]["id"], "para:AAAA0001/ole:_1797");
}

#[test]
fn export_media_matches_embedding_bytes() {
    let (path, xlsx) = write_sample("export");
    let listed = inspect(&path, json!({ "oleObjects": true }));
    let id = listed["oleObjects"][0]["id"].as_str().unwrap();
    let out = tmp_dir().join(format!(
        "export-{}.xlsx",
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    let RunOutput::Written { content_type, .. } = run(
        &json!({
            "schemaVersion": 1,
            "action": "exportMedia",
            "path": path.to_string_lossy(),
            "id": id,
            "output": out.to_string_lossy()
        })
        .to_string(),
        None,
    )
    .unwrap() else {
        panic!("exportMedia 应写出文件");
    };
    assert_eq!(std::fs::read(&out).unwrap(), xlsx);
    assert_eq!(
        part_bytes(&path, "word/embeddings/Microsoft_Excel_Worksheet1.xlsx"),
        xlsx
    );
    assert_eq!(
        content_type.as_deref(),
        Some("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet")
    );
}

#[test]
fn insert_ole_xlsx_lists_and_export_matches_and_styles_untouched() {
    let path = create_docx("insert-host");
    let styles_before = part_bytes(&path, "word/styles.xml");
    let xlsx = write_xlsx("insert-src");
    let xlsx_bytes = std::fs::read(&xlsx).unwrap();
    let tree0 = inspect(&path, json!({ "from": 1, "maxParas": 5 }));
    let para = tree0["paragraphs"][0]["id"].as_str().unwrap().to_string();
    let out = tmp_dir().join(format!(
        "insert-out-{}.docx",
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    apply(
        &path,
        &out,
        json!([{ "op": "insertOle", "id": para, "file": xlsx.to_string_lossy() }]),
    );
    let tree = inspect(&out, json!({ "oleObjects": true }));
    assert_eq!(tree["oleObjectCount"], 1);
    let o = &tree["oleObjects"][0];
    assert_eq!(o["progId"], "Excel.Sheet.12");
    assert_eq!(o["kind"], "embed");
    assert!(o["id"].as_str().unwrap().contains("/ole:"));
    assert_eq!(part_bytes(&out, "word/styles.xml"), styles_before);
    let exp = tmp_dir().join(format!(
        "insert-export-{}.xlsx",
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    run(
        &json!({
            "schemaVersion": 1,
            "action": "exportMedia",
            "path": out.to_string_lossy(),
            "id": o["id"],
            "output": exp.to_string_lossy()
        })
        .to_string(),
        None,
    )
    .unwrap();
    assert_eq!(std::fs::read(&exp).unwrap(), xlsx_bytes);
    let names = zip_names(&out);
    assert!(names
        .iter()
        .any(|n| n.contains("word/embeddings/Microsoft_Excel_Worksheet") && n.ends_with(".xlsx")));
    let rels = String::from_utf8(part_bytes(&out, "word/_rels/document.xml.rels")).unwrap();
    assert!(rels.contains("/package"));
}

#[test]
fn delete_ole_drops_embedding_part() {
    let (path, _) = write_sample("delete");
    let listed = inspect(&path, json!({ "oleObjects": true }));
    let id = listed["oleObjects"][0]["id"].as_str().unwrap();
    let out = tmp_dir().join(format!(
        "delete-out-{}.docx",
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    apply(&path, &out, json!([{ "op": "deleteOle", "id": id }]));
    let tree = inspect(&out, json!({ "oleObjects": true }));
    assert_eq!(tree["oleObjectCount"], 0);
    assert!(tree.get("oleObjects").is_none() || tree["oleObjects"].as_array().unwrap().is_empty());
    let names = zip_names(&out);
    assert!(!names.iter().any(|n| n.contains("word/embeddings/")));
}

#[test]
fn create_with_ole_objects() {
    let xlsx = write_xlsx("create-src");
    let path = tmp_dir().join(format!(
        "create-{}.docx",
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "format": "docx",
            "document": {
                "paragraphs": [{
                    "text": "带 OLE",
                    "oleObjects": [{ "file": xlsx.to_string_lossy() }]
                }]
            }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let tree = inspect(&path, json!({ "oleObjects": true }));
    assert_eq!(tree["oleObjectCount"], 1);
    assert_eq!(tree["oleObjects"][0]["progId"], "Excel.Sheet.12");
}

#[test]
fn wrong_id_is_invalid_id() {
    let (path, _) = write_sample("bad-id");
    let listed = inspect(&path, json!({ "oleObjects": true }));
    let ole_id = listed["oleObjects"][0]["id"].as_str().unwrap();
    let err = apply_err(&path, json!([{ "op": "deleteOle", "id": "para:AAAA0001" }]));
    assert_eq!(err.code(), ErrorCode::InvalidId);
    let xlsx = write_xlsx("bad-insert");
    let err = apply_err(
        &path,
        json!([{ "op": "insertOle", "id": ole_id, "file": xlsx.to_string_lossy() }]),
    );
    assert_eq!(err.code(), ErrorCode::InvalidId);
}

#[test]
fn xlsx_rejects_ole_ops_pptx_uses_slide_ids() {
    let book = write_xlsx("reject-book");
    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": book.to_string_lossy(),
            "output": tmp_dir().join("x.xlsx").to_string_lossy(),
            "ops": [{ "op": "insertOle", "id": "sheet:Sheet1", "file": book.to_string_lossy() }]
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": book.to_string_lossy(),
            "output": tmp_dir().join("x2.xlsx").to_string_lossy(),
            "ops": [{ "op": "deleteOle", "id": "para:1/ole:_1" }]
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);

    let pptx = tmp_dir().join(format!(
        "reject-{}.pptx",
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": pptx.to_string_lossy(),
            "presentation": { "slides": [{ "shapes": [{ "text": "标题" }] }] }
        })
        .to_string(),
        None,
    )
    .unwrap();
    run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": pptx.to_string_lossy(),
            "output": tmp_dir().join("p.pptx").to_string_lossy(),
            "ops": [{ "op": "insertOle", "id": "slide:1", "file": book.to_string_lossy() }]
        })
        .to_string(),
        None,
    )
    .unwrap();
    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": pptx.to_string_lossy(),
            "output": tmp_dir().join("p2.pptx").to_string_lossy(),
            "ops": [{ "op": "deleteOle", "id": "para:1/ole:_1" }]
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
}

#[test]
fn insert_package_writes_cfb_bin() {
    let path = create_docx("pkg-host");
    let txt = tmp_dir().join(format!("notes-{}.txt", SEQ.fetch_add(1, Ordering::Relaxed)));
    std::fs::write(&txt, b"hello-ole-package").unwrap();
    let tree0 = inspect(&path, json!({ "from": 1, "maxParas": 5 }));
    let para = tree0["paragraphs"][0]["id"].as_str().unwrap().to_string();
    let out = tmp_dir().join(format!(
        "pkg-out-{}.docx",
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    apply(
        &path,
        &out,
        json!([{ "op": "insertOle", "id": para, "file": txt.to_string_lossy() }]),
    );
    let tree = inspect(&out, json!({ "oleObjects": true }));
    assert_eq!(tree["oleObjectCount"], 1);
    assert_eq!(tree["oleObjects"][0]["kind"], "package");
    assert_eq!(tree["oleObjects"][0]["progId"], "Package");
    let names = zip_names(&out);
    let bin_name = names
        .iter()
        .find(|n| n.contains("word/embeddings/") && n.ends_with(".bin"))
        .cloned()
        .expect("应写出 oleObjectN.bin");
    let bin = part_bytes(&out, &bin_name);
    assert_eq!(&bin[..8], &[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1]);
}
