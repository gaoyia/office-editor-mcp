//! docx create 补齐：图表、脚注、尾注、批注与已有多节写进同一份 JSON。

use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::ZipArchive;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-docx-create-full");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn seq_path(tag: &str, ext: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}.{ext}"))
}

fn full_document() -> Value {
    json!({
        "paragraphs": [
            {
                "style": "Heading1",
                "text": "第一章",
                "sectionType": "nextPage",
                "footnote": "脚注一",
                "comments": [{ "text": "批注一", "author": "ooxp", "initials": "oo" }]
            },
            {
                "text": "hello",
                "endnote": "尾注一",
                "charts": [{
                    "type": "pie",
                    "title": "份额",
                    "series": [{
                        "name": "S1",
                        "categories": ["甲", "乙"],
                        "values": [3, 1]
                    }]
                }]
            }
        ]
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

#[test]
fn create_without_extras_skips_follow_up_parts() {
    let path = create_docx(
        "plain",
        json!({
            "paragraphs": [
                { "style": "Heading1", "text": "标题" },
                { "text": "hello" }
            ]
        }),
    );
    let tree = inspect(&path, json!({}));
    assert_eq!(tree["chartCount"], 0);
    assert_eq!(tree["footnoteCount"], 0);
    assert_eq!(tree["endnoteCount"], 0);
    assert_eq!(tree["commentCount"], 0);
    assert_eq!(tree["sections"].as_array().unwrap().len(), 1);
    let names = zip_names(&path);
    assert!(!names.iter().any(|n| n.starts_with("word/charts/")));
    assert!(!names.iter().any(|n| n == "word/footnotes.xml"));
    assert!(!names.iter().any(|n| n == "word/endnotes.xml"));
    assert!(!names.iter().any(|n| n == "word/comments.xml"));
}

#[test]
fn create_full_writes_chart_notes_comments_and_two_sections() {
    let path = create_docx("full", full_document());
    let tree0 = inspect(&path, json!({}));
    assert_eq!(tree0["chartCount"], 1);
    assert_eq!(tree0["footnoteCount"], 1);
    assert_eq!(tree0["endnoteCount"], 1);
    assert_eq!(tree0["commentCount"], 1);
    assert!(tree0.get("charts").is_none());
    assert!(tree0.get("footnotes").is_none());
    assert!(tree0.get("comments").is_none());
    assert_eq!(tree0["sections"].as_array().unwrap().len(), 2);

    let tree = inspect(
        &path,
        json!({
            "from": 1,
            "maxParas": 20,
            "charts": true,
            "notes": true,
            "comments": true
        }),
    );
    assert_eq!(tree["charts"][0]["type"], "pie");
    assert_eq!(tree["charts"][0]["title"], "份额");
    assert_eq!(tree["charts"][0]["kind"], "anchor");
    assert_eq!(tree["charts"][0]["wrap"], "square");
    assert_eq!(tree["charts"][0]["id"], "para:00000002/chart:1");
    assert_eq!(tree["footnotes"][0]["paragraphs"][0]["text"], "脚注一");
    assert_eq!(tree["endnotes"][0]["paragraphs"][0]["text"], "尾注一");
    assert_eq!(tree["comments"][0]["paragraphs"][0]["text"], "批注一");
    assert_eq!(tree["comments"][0]["author"], "ooxp");
    assert_eq!(tree["comments"][0]["initials"], "oo");
    assert_eq!(tree["comments"][0]["start"], "para:00000001");
    assert_eq!(tree["comments"][0]["end"], "para:00000001");
    assert_eq!(tree["paragraphs"][0]["text"], "第一章");
    assert_eq!(tree["paragraphs"][1]["text"], "hello");

    assert!(zip_has(&path, "word/charts/chart1.xml"));
    assert!(zip_has(&path, "word/footnotes.xml"));
    assert!(zip_has(&path, "word/endnotes.xml"));
    assert!(zip_has(&path, "word/comments.xml"));
    let cts = {
        let bytes = std::fs::read(&path).unwrap();
        let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
        let mut file = zip.by_name("[Content_Types].xml").unwrap();
        let mut s = String::new();
        std::io::Read::read_to_string(&mut file, &mut s).unwrap();
        s
    };
    assert!(cts.contains(
        "application/vnd.openxmlformats-officedocument.wordprocessingml.commentsExtended+xml"
    ));
    assert!(!cts.contains("vnd.ms-word.commentsExtended+xml"));
    let xml = {
        let bytes = std::fs::read(&path).unwrap();
        let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
        let mut file = zip.by_name("word/document.xml").unwrap();
        let mut s = String::new();
        std::io::Read::read_to_string(&mut file, &mut s).unwrap();
        s
    };
    assert!(xml.contains(r#"<w:type w:val="nextPage"/>"#));
}

#[test]
fn create_empty_charts_writes_no_chart() {
    let path = create_docx(
        "empty-charts",
        json!({
            "paragraphs": [{ "text": "hello", "charts": [] }]
        }),
    );
    let tree = inspect(&path, json!({ "charts": true }));
    assert_eq!(tree["chartCount"], 0);
    assert_eq!(tree["charts"], json!([]));
    assert!(!zip_names(&path)
        .iter()
        .any(|n| n.starts_with("word/charts/")));
}

#[test]
fn create_rejects_xlsx_chart_fields() {
    let err = create_err(json!({
        "paragraphs": [{
            "text": "hello",
            "charts": [{
                "type": "pie",
                "from": "A1",
                "series": [{ "values": [1, 2] }]
            }]
        }]
    }));
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

#[test]
fn create_rejects_section_type_on_last_para() {
    let err = create_err(json!({
        "paragraphs": [{ "text": "仅一段", "sectionType": "nextPage" }]
    }));
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

#[test]
fn create_header_chart_and_comment() {
    let path = create_docx(
        "hf-extra",
        json!({
            "paragraphs": [{ "text": "正文" }],
            "headers": [{
                "type": "default",
                "paragraphs": [{
                    "text": "页眉",
                    "charts": [{
                        "type": "pie",
                        "series": [{ "values": [1, 2] }]
                    }],
                    "comments": [{ "text": "页眉批注" }]
                }]
            }]
        }),
    );
    let tree = inspect(
        &path,
        json!({
            "headerFooter": true,
            "charts": true,
            "comments": true
        }),
    );
    assert_eq!(tree["chartCount"], 0);
    assert_eq!(tree["headers"][0]["chartCount"], 1);
    assert_eq!(tree["headers"][0]["charts"][0]["type"], "pie");
    assert_eq!(tree["commentCount"], 1);
    assert_eq!(tree["comments"][0]["paragraphs"][0]["text"], "页眉批注");
    assert_eq!(
        tree["comments"][0]["start"],
        "header:default:1/para:00000001"
    );
}
