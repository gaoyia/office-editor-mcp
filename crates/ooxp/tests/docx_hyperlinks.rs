//! docx 超链接与书签：名单、插改删、setParaText 守卫。

use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::json;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

const CT: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/><Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/><Override PartName="/word/header1.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.header+xml"/></Types>"#;

const ROOT_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#;

const DOC_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/header" Target="header1.xml"/><Relationship Id="rId9" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="https://example.com" TargetMode="External"/></Relationships>"#;

const HDR_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="https://header.example" TargetMode="External"/></Relationships>"#;

const STYLES: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/></w:style></w:styles>"#;

const DOCUMENT: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><w:body><w:p w14:paraId="AAAA0001"><w:hyperlink r:id="rId9" w:tooltip="tip"><w:r><w:t>链甲</w:t></w:r></w:hyperlink></w:p><w:p w14:paraId="AAAA0002"><w:bookmarkStart w:id="1" w:name="SrcBm"/><w:r><w:t>书乙</w:t></w:r><w:bookmarkEnd w:id="1"/><w:bookmarkStart w:id="2" w:name="_GoBack"/><w:bookmarkEnd w:id="2"/></w:p><w:p w14:paraId="AAAA0003"><w:r><w:t>干净</w:t></w:r></w:p><w:p w14:paraId="AAAA0004"><w:hyperlink w:anchor="SrcBm"><w:r><w:t>锚丙</w:t></w:r></w:hyperlink></w:p><w:sectPr><w:headerReference w:type="default" r:id="rId2"/><w:pgSz w:w="11906" w:h="16838"/></w:sectPr></w:body></w:document>"#;

const HEADER: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:hdr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><w:p w14:paraId="HHHH0001"><w:bookmarkStart w:id="0" w:name="HfBm"/><w:hyperlink r:id="rId1"><w:r><w:t>页眉链</w:t></w:r></w:hyperlink><w:bookmarkEnd w:id="0"/></w:p></w:hdr>"#;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-docx-hyperlinks");
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
            ("word/_rels/header1.xml.rels", HDR_RELS),
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
    let RunOutput::Inspect(tree) = run(&json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": path.to_string_lossy(),
        "scope": scope
    })
    .to_string())
    .unwrap() else {
        panic!("inspect 应返回树");
    };
    tree
}

fn apply(path: &Path, out: &Path, ops: serde_json::Value) {
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

fn apply_err(path: &Path, ops: serde_json::Value) -> ooxp::Error {
    let out = tmp_dir().join("err.docx");
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
        json!({
            "hyperlinks": true,
            "bookmarks": true,
            "headerFooter": true,
            "from": 1,
            "maxParas": 20
        }),
    )
}

#[test]
fn inspect_counts_without_listing() {
    let path = write_sample("bare");
    let tree = inspect(&path, json!({}));
    assert_eq!(tree["hyperlinkCount"], 2);
    assert_eq!(tree["bookmarkCount"], 1);
    assert!(tree.get("hyperlinks").is_none());
    assert!(tree.get("bookmarks").is_none());
    assert_eq!(tree["headers"][0]["hyperlinkCount"], 1);
    assert_eq!(tree["headers"][0]["bookmarkCount"], 1);
    assert!(tree["headers"][0].get("hyperlinks").is_none());
}

#[test]
fn inspect_lists_links_and_bookmarks() {
    let path = write_sample("list");
    let tree = listed(&path);
    assert_eq!(
        object_keys(&tree["hyperlinks"][0]),
        ["id", "index", "kind", "rId", "url", "anchor", "tooltip", "host", "text"]
    );
    let links = tree["hyperlinks"].as_array().unwrap();
    assert_eq!(links.len(), 2);
    assert_eq!(links[0]["id"], "link:1");
    assert_eq!(links[0]["kind"], "url");
    assert_eq!(links[0]["rId"], "rId9");
    assert_eq!(links[0]["url"], "https://example.com");
    assert_eq!(links[0]["tooltip"], "tip");
    assert_eq!(links[0]["host"], "para:AAAA0001");
    assert_eq!(links[0]["text"], "链甲");
    assert_eq!(links[1]["kind"], "anchor");
    assert_eq!(links[1]["anchor"], "SrcBm");
    assert!(links[1]["url"].is_null());
    assert_eq!(links[1]["text"], "锚丙");
    let bms = tree["bookmarks"].as_array().unwrap();
    assert_eq!(bms.len(), 1);
    assert_eq!(
        object_keys(&bms[0]),
        ["id", "index", "name", "xmlId", "start", "end"]
    );
    assert_eq!(bms[0]["id"], "bookmark:1");
    assert_eq!(bms[0]["name"], "SrcBm");
    assert_eq!(bms[0]["xmlId"], "1");
    assert_eq!(bms[0]["start"], "para:AAAA0002");
    assert_eq!(bms[0]["end"], "para:AAAA0002");
    assert_eq!(
        tree["headers"][0]["hyperlinks"][0]["id"],
        "header:default:1/link:1"
    );
    assert_eq!(
        tree["headers"][0]["hyperlinks"][0]["url"],
        "https://header.example"
    );
    assert_eq!(tree["headers"][0]["bookmarks"][0]["name"], "HfBm");
}

#[test]
fn set_hyperlink_changes_url() {
    let path = write_sample("set-url");
    let out = tmp_dir().join("set-url-out.docx");
    apply(
        &path,
        &out,
        json!([{ "op": "setHyperlink", "id": "link:1", "url": "https://example.org" }]),
    );
    let tree = listed(&out);
    assert_eq!(tree["hyperlinks"][0]["url"], "https://example.org");
    let rels = part_text(&out, "word/_rels/document.xml.rels");
    assert!(rels.contains("https://example.org"));
    assert!(!rels.contains("https://example.com"));
}

#[test]
fn insert_and_delete_hyperlink() {
    let path = write_sample("ins-del");
    let inserted = tmp_dir().join("ins-out.docx");
    apply(
        &path,
        &inserted,
        json!([{
            "op": "insertHyperlink",
            "id": "para:AAAA0003",
            "text": "新链",
            "url": "https://new.example"
        }]),
    );
    let tree = listed(&inserted);
    assert_eq!(tree["hyperlinkCount"], 3);
    let new = tree["hyperlinks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|h| h["text"] == "新链")
        .unwrap();
    assert_eq!(new["url"], "https://new.example");
    let new_id = new["id"].as_str().unwrap().to_string();

    let deleted = tmp_dir().join("del-out.docx");
    apply(
        &inserted,
        &deleted,
        json!([{ "op": "deleteHyperlink", "id": new_id }]),
    );
    let after = listed(&deleted);
    assert_eq!(after["hyperlinkCount"], 2);
    let xml = part_text(&deleted, "word/document.xml");
    assert!(xml.contains("新链"));
    assert!(!xml.contains("https://new.example"));
    let rels = part_text(&deleted, "word/_rels/document.xml.rels");
    assert!(!rels.contains("https://new.example"));
}

#[test]
fn insert_set_delete_bookmark() {
    let path = write_sample("bm");
    let inserted = tmp_dir().join("bm-ins.docx");
    apply(
        &path,
        &inserted,
        json!([{ "op": "insertBookmark", "id": "para:AAAA0003", "name": "NewBm" }]),
    );
    let tree = listed(&inserted);
    assert_eq!(tree["bookmarkCount"], 2);
    let bm = tree["bookmarks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["name"] == "NewBm")
        .unwrap();
    assert_eq!(bm["start"], "para:AAAA0003");
    let bm_id = bm["id"].as_str().unwrap().to_string();

    let renamed = tmp_dir().join("bm-ren.docx");
    apply(
        &inserted,
        &renamed,
        json!([{ "op": "setBookmark", "id": bm_id, "name": "RenBm" }]),
    );
    let mid = listed(&renamed);
    assert!(mid["bookmarks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|b| b["name"] == "RenBm"));

    let gone = tmp_dir().join("bm-del.docx");
    let ren_id = mid["bookmarks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["name"] == "RenBm")
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();
    apply(
        &renamed,
        &gone,
        json!([{ "op": "deleteBookmark", "id": ren_id }]),
    );
    let after = listed(&gone);
    assert_eq!(after["bookmarkCount"], 1);
    assert_eq!(after["bookmarks"][0]["name"], "SrcBm");
}

#[test]
fn set_bookmark_renames_hyperlink_anchor() {
    let path = write_sample("bm-anchor");
    let out = tmp_dir().join("bm-anchor-out.docx");
    apply(
        &path,
        &out,
        json!([{ "op": "setBookmark", "id": "bookmark:1", "name": "MovedBm" }]),
    );
    let tree = listed(&out);
    assert_eq!(tree["bookmarks"][0]["name"], "MovedBm");
    let anchor = tree["hyperlinks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|h| h["kind"] == "anchor")
        .unwrap();
    assert_eq!(anchor["anchor"], "MovedBm");
}

#[test]
fn set_para_text_refuses_hyperlink_para() {
    let path = write_sample("set-hl");
    let err = apply_err(
        &path,
        json!([{ "op": "setParaText", "id": "para:AAAA0001", "text": "x" }]),
    );
    assert_eq!(err.code(), ErrorCode::NotImplemented);

    let out = tmp_dir().join("set-clean-out.docx");
    apply(
        &path,
        &out,
        json!([{ "op": "setParaText", "id": "para:AAAA0003", "text": "改过" }]),
    );
    let tree = inspect(&out, json!({ "from": 1, "maxParas": 20 }));
    assert_eq!(tree["paragraphs"][2]["text"], "改过");
}

#[test]
fn insert_hyperlink_rejects_both_targets() {
    let path = write_sample("both");
    let err = apply_err(
        &path,
        json!([{
            "op": "insertHyperlink",
            "id": "para:AAAA0003",
            "text": "x",
            "url": "https://example.com",
            "anchor": "SrcBm"
        }]),
    );
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

#[test]
fn header_insert_hyperlink() {
    let path = write_sample("hf");
    let out = tmp_dir().join("hf-out.docx");
    apply(
        &path,
        &out,
        json!([{
            "op": "insertHyperlink",
            "id": "header:default:1/para:index:1",
            "text": "页眉新",
            "url": "https://hf.example"
        }]),
    );
    let tree = listed(&out);
    assert_eq!(tree["headers"][0]["hyperlinkCount"], 2);
    assert!(tree["headers"][0]["hyperlinks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|h| h["url"] == "https://hf.example"));
}
