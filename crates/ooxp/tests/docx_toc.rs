//! docx 目录：insertToc / deleteToc、inspect tocCount、xlsx/pptx 拒绝。

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
    let dir = std::env::temp_dir().join("ooxp-docx-toc");
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

fn inspect(path: &Path, tocs: bool, fields: bool) -> Value {
    let RunOutput::Inspect(tree) = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": path.to_string_lossy(),
            "scope": { "from": 1, "maxParas": 20, "tocs": tocs, "fields": fields, "headerFooter": true }
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

#[test]
fn insert_toc_lists_and_delete_keeps_heading() {
    let src = create_docx(
        "toc-src",
        json!({
            "paragraphs": [
                { "style": "Heading1", "text": "第一章" },
                { "text": "hello" }
            ]
        }),
    );
    let tree0 = inspect(&src, false, false);
    assert_eq!(tree0["tocCount"], 0);
    assert!(tree0.get("tocs").is_none());
    let p1 = first_body_para_id(&tree0);
    let out = seq_path("toc-out", "docx");
    apply(
        &src,
        &out,
        json!([{ "op": "insertToc", "id": p1, "kind": "toc" }]),
    );
    let tree = inspect(&out, true, true);
    assert_eq!(tree["tocCount"], 1);
    assert_eq!(tree["fieldCount"], 1);
    assert_eq!(
        tree["tocs"][0]
            .as_object()
            .unwrap()
            .keys()
            .map(|k| k.as_str())
            .collect::<Vec<_>>(),
        ["id", "index", "kind", "field", "instr", "host", "end", "text"]
    );
    assert_eq!(tree["tocs"][0]["id"], "toc:1");
    assert_eq!(tree["tocs"][0]["kind"], "toc");
    assert_eq!(tree["tocs"][0]["field"], "field:1");
    assert_eq!(tree["fields"][0]["code"], "TOC");
    assert_eq!(tree["paragraphs"][0]["text"], "第一章");

    let gone = seq_path("toc-del", "docx");
    apply(&out, &gone, json!([{ "op": "deleteToc", "id": "toc:1" }]));
    let tree2 = inspect(&gone, true, true);
    assert_eq!(tree2["tocCount"], 0);
    assert_eq!(tree2["fieldCount"], 0);
    assert_eq!(tree2["tocs"].as_array().unwrap().len(), 0);
    let texts: Vec<&str> = tree2["paragraphs"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|p| p["text"].as_str())
        .collect();
    assert!(texts.contains(&"第一章"));
    assert!(texts.contains(&"hello"));
}

#[test]
fn insert_tof_and_toa_kinds() {
    let src = create_docx(
        "kind-src",
        json!({ "paragraphs": [{ "text": "甲" }, { "text": "乙" }] }),
    );
    let p1 = first_body_para_id(&inspect(&src, false, false));
    let tof = seq_path("tof", "docx");
    apply(
        &src,
        &tof,
        json!([{ "op": "insertToc", "id": p1, "kind": "tof" }]),
    );
    let t = inspect(&tof, true, false);
    assert_eq!(t["tocs"][0]["kind"], "tof");

    let toa = seq_path("toa", "docx");
    apply(
        &src,
        &toa,
        json!([{ "op": "insertToc", "id": "docx:document", "kind": "toa" }]),
    );
    let t2 = inspect(&toa, true, true);
    assert_eq!(t2["tocs"][0]["kind"], "toa");
    assert_eq!(t2["fields"][0]["code"], "TOA");
}

#[test]
fn insert_toc_rejects_bad_id_and_kind() {
    let src = create_docx(
        "bad-src",
        json!({
            "paragraphs": [{ "text": "甲" }],
            "headers": [{ "type": "default", "paragraphs": [{ "text": "页眉" }] }]
        }),
    );
    let tree = inspect(&src, false, false);
    assert!(tree["headers"][0].get("tocCount").is_none());
    let hf_para = tree["headers"][0]["paragraphs"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let err_hf = apply_err(&src, json!([{ "op": "insertToc", "id": hf_para }]));
    assert_eq!(err_hf.code(), ErrorCode::InvalidId);

    let err_kind = apply_err(
        &src,
        json!([{ "op": "insertToc", "id": "docx:document", "kind": "nope" }]),
    );
    assert_eq!(err_kind.code(), ErrorCode::InvalidField);

    let err_instr = apply_err(
        &src,
        json!([{ "op": "insertToc", "id": "docx:document", "instr": "PAGE" }]),
    );
    assert_eq!(err_instr.code(), ErrorCode::InvalidField);

    let err_del = apply_err(&src, json!([{ "op": "deleteToc", "id": "toc:1" }]));
    assert_eq!(err_del.code(), ErrorCode::IdNotFound);

    let err_id = apply_err(&src, json!([{ "op": "deleteToc", "id": "field:1" }]));
    assert_eq!(err_id.code(), ErrorCode::InvalidId);
}

#[test]
fn toc_ops_not_for_xlsx_or_pptx() {
    let xlsx = seq_path("toc-x", "xlsx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": xlsx.to_string_lossy(),
            "workbook": { "sheets": [{ "name": "Sheet1" }] }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let xout = seq_path("toc-x-out", "xlsx");
    let err_x = run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": xlsx.to_string_lossy(),
            "output": xout.to_string_lossy(),
            "ops": [{ "op": "insertToc", "id": "docx:document" }]
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err_x.code(), ErrorCode::InvalidField);

    let pptx = seq_path("toc-p", "pptx");
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
    let pout = seq_path("toc-p-out", "pptx");
    let err_p = run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": pptx.to_string_lossy(),
            "output": pout.to_string_lossy(),
            "ops": [{ "op": "deleteToc", "id": "toc:1" }]
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
fn delete_toc_drops_gallery_sdt() {
    let src = write_docx_xml(
        "sdt",
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml"><w:body><w:sdt><w:sdtPr><w:docPartObj><w:docPartGallery w:val="Table of Contents"/></w:docPartObj></w:sdtPr><w:sdtContent><w:p w14:paraId="AAAA0001"><w:r><w:t>目录</w:t></w:r></w:p><w:p w14:paraId="AAAA0002"><w:r><w:fldChar w:fldCharType="begin"/></w:r><w:r><w:instrText xml:space="preserve"> TOC \o "1-3" </w:instrText></w:r><w:r><w:fldChar w:fldCharType="separate"/></w:r><w:r><w:t>条目</w:t></w:r><w:r><w:fldChar w:fldCharType="end"/></w:r></w:p></w:sdtContent></w:sdt><w:p w14:paraId="AAAA0003"><w:r><w:t>第一章</w:t></w:r></w:p><w:sectPr><w:pgSz w:w="11906" w:h="16838"/></w:sectPr></w:body></w:document>"#,
    );
    let tree0 = inspect(&src, true, false);
    assert_eq!(tree0["tocCount"], 1);
    let out = seq_path("sdt-out", "docx");
    apply(&src, &out, json!([{ "op": "deleteToc", "id": "toc:1" }]));
    let tree = inspect(&out, true, true);
    assert_eq!(tree["tocCount"], 0);
    assert_eq!(tree["fieldCount"], 0);
    let texts: Vec<&str> = tree["paragraphs"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|p| p["text"].as_str())
        .collect();
    assert!(texts.contains(&"第一章"));
    assert!(!texts
        .iter()
        .any(|t| t.contains("目录") || t.contains("条目")));
}

#[test]
fn delete_toc_rejects_sect_pr_para() {
    let src = write_docx_xml(
        "sect",
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml"><w:body><w:p w14:paraId="AAAA0001"><w:r><w:t>甲</w:t></w:r></w:p><w:p w14:paraId="AAAA0002"><w:pPr><w:sectPr><w:type w:val="nextPage"/><w:pgSz w:w="11906" w:h="16838"/></w:sectPr></w:pPr><w:r><w:fldChar w:fldCharType="begin"/></w:r><w:r><w:instrText xml:space="preserve"> TOC </w:instrText></w:r><w:r><w:fldChar w:fldCharType="separate"/></w:r><w:r><w:fldChar w:fldCharType="end"/></w:r></w:p><w:p w14:paraId="AAAA0003"><w:r><w:t>乙</w:t></w:r></w:p><w:sectPr><w:pgSz w:w="11906" w:h="16838"/></w:sectPr></w:body></w:document>"#,
    );
    let err = apply_err(&src, json!([{ "op": "deleteToc", "id": "toc:1" }]));
    assert_eq!(err.code(), ErrorCode::NotImplemented);
}
