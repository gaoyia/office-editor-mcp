//! docx create 补齐剩余：链、书签、域、目录、编号、公式、内容控件与批注回复。

use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::ZipArchive;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-docx-create-remain");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn seq_path(tag: &str, ext: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}.{ext}"))
}

fn remain_document() -> Value {
    json!({
        "numberings": [{ "kind": "decimal" }],
        "paragraphs": [
            {
                "style": "Heading1",
                "text": "第一章",
                "bookmarks": [{ "name": "ch1" }],
                "comments": [
                    {
                        "text": "批注一",
                        "author": "ooxp",
                        "initials": "oo",
                        "replies": [{ "text": "回复一" }]
                    }
                ]
            },
            {
                "text": "列表项",
                "num": 1,
                "hyperlinks": [{
                    "text": "点这里",
                    "url": "https://example.com"
                }],
                "fields": [{ "instr": " DATE ", "result": "2026" }],
                "maths": [{ "text": "x+1", "kind": "inline" }],
                "contentControls": [{ "text": "控件值", "kind": "plainText" }]
            }
        ],
        "tocs": [{ "kind": "toc" }]
    })
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

fn create_err(document: Value) -> ooxp::Error {
    let path = seq_path("err", "docx");
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
    .unwrap_err()
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

fn zip_names(path: &Path) -> Vec<String> {
    let bytes = std::fs::read(path).unwrap();
    let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut names = Vec::new();
    for i in 0..zip.len() {
        names.push(zip.by_index(i).unwrap().name().replace('\\', "/"));
    }
    names
}

fn zip_has(path: &Path, name: &str) -> bool {
    zip_names(path).iter().any(|n| n == name)
}

fn part_text(path: &Path, name: &str) -> String {
    let bytes = std::fs::read(path).unwrap();
    let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut file = zip.by_name(name).unwrap();
    let mut s = String::new();
    std::io::Read::read_to_string(&mut file, &mut s).unwrap();
    s
}

#[test]
fn create_remain_writes_links_fields_math_sdt_toc_numbering_and_replies() {
    let path = create_docx("remain", remain_document());
    let tree0 = inspect(&path, json!({}));
    assert_eq!(tree0["hyperlinkCount"], 1);
    assert_eq!(tree0["bookmarkCount"], 1);
    assert!(tree0["fieldCount"].as_u64().unwrap() >= 2);
    assert_eq!(tree0["mathCount"], 1);
    assert_eq!(tree0["contentControlCount"], 1);
    assert_eq!(tree0["tocCount"], 1);
    assert_eq!(tree0["numberingCount"], 1);
    assert_eq!(tree0["commentCount"], 2);
    assert!(tree0.get("hyperlinks").is_none());
    assert!(tree0.get("tocs").is_none());
    assert!(tree0.get("comments").is_none());

    let tree = inspect(
        &path,
        json!({
            "from": 1,
            "maxParas": 20,
            "hyperlinks": true,
            "bookmarks": true,
            "fields": true,
            "tocs": true,
            "maths": true,
            "contentControls": true,
            "numbering": true,
            "comments": true
        }),
    );
    assert_eq!(tree["hyperlinks"][0]["url"], "https://example.com");
    assert_eq!(tree["hyperlinks"][0]["text"], "点这里");
    assert_eq!(tree["bookmarks"][0]["name"], "ch1");
    let fields = tree["fields"].as_array().unwrap();
    assert!(fields
        .iter()
        .any(|f| f["instr"].as_str().is_some_and(|s| s.contains("DATE"))));
    assert_eq!(tree["maths"][0]["text"], "x+1");
    assert_eq!(tree["maths"][0]["kind"], "inline");
    assert_eq!(tree["contentControls"][0]["text"], "控件值");
    assert_eq!(tree["contentControls"][0]["kind"], "plainText");
    assert_eq!(tree["tocs"].as_array().unwrap().len(), 1);
    assert_eq!(tree["numberings"][0]["id"], "num:1");
    let paras = tree["paragraphs"].as_array().unwrap();
    let listed = paras
        .iter()
        .find(|p| p["num"] == json!("num:1"))
        .expect("应有绑 num:1 的段");
    assert!(listed["text"]
        .as_str()
        .is_some_and(|s| s.contains("列表项")));
    let comments = tree["comments"].as_array().unwrap();
    assert_eq!(comments.len(), 2);
    let reply = comments
        .iter()
        .find(|c| c["parent"] == json!("comment:0"))
        .expect("应有回复");
    assert_eq!(reply["paragraphs"][0]["text"], "回复一");
    let parent = comments
        .iter()
        .find(|c| c["parent"].is_null())
        .expect("应有顶层批注");
    assert_eq!(parent["paragraphs"][0]["text"], "批注一");

    assert!(zip_has(&path, "word/numbering.xml"));
    let settings = part_text(&path, "word/settings.xml");
    assert!(!settings.contains("trackRevisions"));
    let rels = part_text(&path, "word/_rels/document.xml.rels");
    assert!(rels.contains("/hyperlink"));
}

#[test]
fn create_remain_rejects_url_and_anchor() {
    let err = create_err(json!({
        "paragraphs": [{
            "text": "hello",
            "hyperlinks": [{
                "text": "两头",
                "url": "https://example.com",
                "anchor": "ch1"
            }]
        }]
    }));
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

#[test]
fn create_remain_rejects_toc_after_past_last() {
    let err = create_err(json!({
        "paragraphs": [
            { "text": "一" },
            { "text": "二" }
        ],
        "tocs": [{ "kind": "toc", "after": 3 }]
    }));
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

#[test]
fn create_remain_rejects_nested_replies() {
    let err = create_err(json!({
        "paragraphs": [{
            "text": "hello",
            "comments": [{
                "text": "父",
                "replies": [{
                    "text": "子",
                    "replies": [{ "text": "孙" }]
                }]
            }]
        }]
    }));
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

#[test]
fn create_remain_rejects_ilvl_without_num() {
    let err = create_err(json!({
        "numberings": [{ "kind": "decimal" }],
        "paragraphs": [{ "text": "hello", "ilvl": 1 }]
    }));
    assert_eq!(err.code(), ErrorCode::InvalidField);
}
