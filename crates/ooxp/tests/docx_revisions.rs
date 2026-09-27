//! docx 修订：名单、接受、拒绝、setParaText 守卫。

use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::json;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

const CT: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/><Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/><Override PartName="/word/header1.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.header+xml"/></Types>"#;

const ROOT_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#;

const DOC_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/header" Target="header1.xml"/></Relationships>"#;

const STYLES: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/></w:style></w:styles>"#;

const DOCUMENT: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><w:body><w:p w14:paraId="AAAA0001"><w:r><w:t>原文</w:t></w:r><w:ins w:id="1" w:author="Alice" w:date="2026-01-01T00:00:00Z"><w:r><w:rPr><w:ins w:id="1" w:author="Alice" w:date="2026-01-01T00:00:00Z"/></w:rPr><w:t>插入</w:t></w:r></w:ins></w:p><w:p w14:paraId="AAAA0002"><w:r><w:t>留下</w:t></w:r><w:del w:id="2" w:author="Bob"><w:r><w:rPr><w:del w:id="2" w:author="Bob"/></w:rPr><w:delText>删掉</w:delText></w:r></w:del></w:p><w:p w14:paraId="AAAA0003"><w:r><w:rPr><w:b/><w:rPrChange w:id="3" w:author="Alice"><w:rPr/></w:rPrChange></w:rPr><w:t>粗</w:t></w:r></w:p><w:p w14:paraId="AAAA0004"><w:pPr><w:jc w:val="center"/><w:pPrChange w:id="4"><w:pPr/></w:pPrChange></w:pPr><w:r><w:t>居中</w:t></w:r></w:p><w:p w14:paraId="AAAA0005"><w:r><w:t>干净</w:t></w:r></w:p><w:sectPr><w:headerReference w:type="default" r:id="rId2"/><w:pgSz w:w="11906" w:h="16838"/></w:sectPr></w:body></w:document>"#;

const HEADER: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:hdr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml"><w:p w14:paraId="HHHH0001"><w:r><w:t>页眉</w:t></w:r><w:ins w:id="9" w:author="HF"><w:r><w:t>新</w:t></w:r></w:ins></w:p></w:hdr>"#;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-docx-revisions");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_sample(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let path = tmp_dir().join(format!("{tag}-{n}.docx"));
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut zip = ZipWriter::new(&mut cursor);
        let opt = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        for (name, body) in [
            ("[Content_Types].xml", CT),
            ("_rels/.rels", ROOT_RELS),
            ("word/document.xml", DOCUMENT),
            ("word/_rels/document.xml.rels", DOC_RELS),
            ("word/styles.xml", STYLES),
            ("word/header1.xml", HEADER),
        ] {
            zip.start_file(name, opt).unwrap();
            zip.write_all(body.as_bytes()).unwrap();
        }
        zip.finish().unwrap();
    }
    std::fs::write(&path, cursor.into_inner()).unwrap();
    path
}

fn inspect(path: &Path, scope: serde_json::Value) -> serde_json::Value {
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

fn apply(path: &Path, out: &Path, ops: serde_json::Value) {
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

fn apply_err(path: &Path, ops: serde_json::Value) -> ooxp::Error {
    let out = tmp_dir().join("err.docx");
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

fn object_keys(v: &serde_json::Value) -> Vec<&str> {
    v.as_object().unwrap().keys().map(String::as_str).collect()
}

fn part_text(path: &Path, name: &str) -> String {
    let bytes = std::fs::read(path).unwrap();
    let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut file = zip.by_name(name).unwrap();
    let mut buf = Vec::new();
    file.read_to_end(&mut buf).unwrap();
    String::from_utf8(buf).unwrap()
}

fn listed(path: &Path) -> serde_json::Value {
    inspect(
        path,
        json!({ "revisions": true, "from": 1, "maxParas": 20 }),
    )
}

#[test]
fn inspect_counts_without_listing() {
    let path = write_sample("bare");
    let tree = inspect(&path, json!({}));
    assert_eq!(tree["revisionCount"], 4);
    assert!(tree.get("revisions").is_none());
    assert_eq!(tree["headers"][0]["revisionCount"], 1);
    assert!(tree["headers"][0].get("revisions").is_none());
}

#[test]
fn inspect_lists_kinds_and_hosts() {
    let path = write_sample("list");
    let tree = listed(&path);
    assert_eq!(
        object_keys(&tree["revisions"][0]),
        ["id", "index", "kind", "xmlId", "author", "date", "host", "text"]
    );
    let revs = tree["revisions"].as_array().unwrap();
    assert_eq!(revs.len(), 4);
    assert_eq!(revs[0]["id"], "rev:1");
    assert_eq!(revs[0]["kind"], "ins");
    assert_eq!(revs[0]["xmlId"], "1");
    assert_eq!(revs[0]["author"], "Alice");
    assert_eq!(revs[0]["date"], "2026-01-01T00:00:00Z");
    assert_eq!(revs[0]["host"], "para:AAAA0001");
    assert_eq!(revs[0]["text"], "插入");
    assert_eq!(revs[1]["kind"], "del");
    assert_eq!(revs[1]["author"], "Bob");
    assert!(revs[1]["date"].is_null());
    assert_eq!(revs[1]["host"], "para:AAAA0002");
    assert_eq!(revs[1]["text"], "删掉");
    assert_eq!(revs[2]["kind"], "rPrChange");
    assert!(revs[2]["text"].is_null());
    assert_eq!(revs[2]["host"], "para:AAAA0003");
    assert_eq!(revs[3]["kind"], "pPrChange");
    assert_eq!(revs[3]["xmlId"], "4");
    assert!(revs[3]["author"].is_null());
    let hf = &tree["headers"][0]["revisions"][0];
    assert_eq!(hf["id"], "header:default:1/rev:1");
    assert_eq!(hf["kind"], "ins");
    assert_eq!(hf["host"], "header:default:1/para:HHHH0001");
    assert_eq!(hf["text"], "新");
}

#[test]
fn accept_delete_drops_text() {
    let path = write_sample("acc-del");
    let out = tmp_dir().join("acc-del-out.docx");
    apply(
        &path,
        &out,
        json!([{ "op": "acceptRevision", "id": "rev:2" }]),
    );
    let tree = listed(&out);
    assert_eq!(tree["revisionCount"], 3);
    assert_eq!(tree["paragraphs"][1]["text"], "留下");
    let xml = part_text(&out, "word/document.xml");
    assert!(!xml.contains("删掉"));
    assert!(!xml.contains("<w:del "));
}

#[test]
fn reject_insert_drops_inserted() {
    let path = write_sample("rej-ins");
    let out = tmp_dir().join("rej-ins-out.docx");
    apply(
        &path,
        &out,
        json!([{ "op": "rejectRevision", "id": "rev:1" }]),
    );
    let tree = listed(&out);
    assert_eq!(tree["paragraphs"][0]["text"], "原文");
    let xml = part_text(&out, "word/document.xml");
    assert!(!xml.contains("插入"));
    assert!(!xml.contains("<w:ins "));
}

#[test]
fn accept_insert_keeps_text() {
    let path = write_sample("acc-ins");
    let out = tmp_dir().join("acc-ins-out.docx");
    apply(
        &path,
        &out,
        json!([{ "op": "acceptRevision", "id": "rev:1" }]),
    );
    let tree = listed(&out);
    assert_eq!(tree["paragraphs"][0]["text"], "原文插入");
    let xml = part_text(&out, "word/document.xml");
    assert!(xml.contains("插入"));
    assert!(!xml.contains("<w:ins "));
}

#[test]
fn reject_delete_restores_text() {
    let path = write_sample("rej-del");
    let out = tmp_dir().join("rej-del-out.docx");
    apply(
        &path,
        &out,
        json!([{ "op": "rejectRevision", "id": "rev:2" }]),
    );
    let tree = listed(&out);
    assert_eq!(tree["paragraphs"][1]["text"], "留下删掉");
    let xml = part_text(&out, "word/document.xml");
    assert!(xml.contains("<w:t>删掉</w:t>"));
    assert!(!xml.contains("delText"));
    assert!(!xml.contains("<w:del "));
}

#[test]
fn accept_rpr_change_keeps_bold() {
    let path = write_sample("acc-rpr");
    let out = tmp_dir().join("acc-rpr-out.docx");
    apply(
        &path,
        &out,
        json!([{ "op": "acceptRevision", "id": "rev:3" }]),
    );
    let xml = part_text(&out, "word/document.xml");
    assert!(xml.contains("<w:b/>"));
    assert!(!xml.contains("rPrChange"));
}

#[test]
fn reject_rpr_change_drops_bold() {
    let path = write_sample("rej-rpr");
    let out = tmp_dir().join("rej-rpr-out.docx");
    apply(
        &path,
        &out,
        json!([{ "op": "rejectRevision", "id": "rev:3" }]),
    );
    let xml = part_text(&out, "word/document.xml");
    assert!(!xml.contains("<w:b/>"));
    assert!(!xml.contains("rPrChange"));
}

#[test]
fn accept_ppr_change_keeps_center() {
    let path = write_sample("acc-ppr");
    let out = tmp_dir().join("acc-ppr-out.docx");
    apply(
        &path,
        &out,
        json!([{ "op": "acceptRevision", "id": "rev:4" }]),
    );
    let xml = part_text(&out, "word/document.xml");
    assert!(xml.contains("w:val=\"center\""));
    assert!(!xml.contains("pPrChange"));
}

#[test]
fn reject_ppr_change_drops_center() {
    let path = write_sample("rej-ppr");
    let out = tmp_dir().join("rej-ppr-out.docx");
    apply(
        &path,
        &out,
        json!([{ "op": "rejectRevision", "id": "rev:4" }]),
    );
    let xml = part_text(&out, "word/document.xml");
    assert!(!xml.contains("w:val=\"center\""));
    assert!(!xml.contains("pPrChange"));
}

#[test]
fn accept_header_insert() {
    let path = write_sample("hf-ins");
    let out = tmp_dir().join("hf-ins-out.docx");
    apply(
        &path,
        &out,
        json!([{ "op": "acceptRevision", "id": "header:default:1/rev:1" }]),
    );
    let tree = inspect(&out, json!({ "revisions": true, "headerFooter": true }));
    assert_eq!(tree["headers"][0]["revisionCount"], 0);
    assert_eq!(tree["headers"][0]["paragraphs"][0]["text"], "页眉新");
}

#[test]
fn set_para_text_refuses_tracked_para() {
    let path = write_sample("set-rev");
    let err = apply_err(
        &path,
        json!([{ "op": "setParaText", "id": "para:AAAA0001", "text": "x" }]),
    );
    assert_eq!(err.code(), ErrorCode::NotImplemented);

    let out = tmp_dir().join("set-clean-out.docx");
    apply(
        &path,
        &out,
        json!([{ "op": "setParaText", "id": "para:AAAA0005", "text": "改过" }]),
    );
    let tree = listed(&out);
    assert_eq!(tree["paragraphs"][4]["text"], "改过");
}

#[test]
fn bad_id_and_missing() {
    let path = write_sample("ids");
    let err = apply_err(
        &path,
        json!([{ "op": "acceptRevision", "id": "para:AAAA0001" }]),
    );
    assert_eq!(err.code(), ErrorCode::InvalidId);
    let err = apply_err(&path, json!([{ "op": "rejectRevision", "id": "rev:99" }]));
    assert_eq!(err.code(), ErrorCode::IdNotFound);
}
