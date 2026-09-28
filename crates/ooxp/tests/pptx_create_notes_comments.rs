//! create 写出演讲者备注与现代批注。

use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::ZipArchive;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-pptx-create-notes-comments");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn tmp(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}.pptx"))
}

fn create(path: &Path, slides: Value) {
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "presentation": { "slides": slides }
        })
        .to_string(),
        None,
    )
    .unwrap();
}

fn create_err(path: &Path, presentation: Value) -> ooxp::Error {
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "presentation": presentation
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

fn inspect_bare(path: &Path) -> Value {
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
        panic!("inspect 应返回树");
    };
    tree
}

fn part_names(path: &Path) -> Vec<String> {
    let bytes = std::fs::read(path).unwrap();
    let zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    zip.file_names().map(str::to_string).collect()
}

fn part_text(path: &Path, name: &str) -> String {
    let bytes = std::fs::read(path).unwrap();
    let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut file = zip.by_name(name).unwrap();
    let mut out = Vec::new();
    file.read_to_end(&mut out).unwrap();
    String::from_utf8(out).unwrap()
}

fn notes_body(notes: &Value) -> String {
    notes["shapes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["id"] == "slide:1/notes/shape:3")
        .and_then(|s| s["text"].as_str())
        .unwrap_or("")
        .to_string()
}

fn example_slides() -> Value {
    json!([
        {
            "shapes": [{ "text": "标题" }],
            "notes": "演讲者备注",
            "comments": [{
                "text": "一条批注",
                "author": "Alice",
                "initials": "A",
                "date": "2026-09-01T00:00:00.000",
                "x": 914400,
                "y": 914400
            }]
        },
        { "shapes": [{ "text": "第二页" }] }
    ])
}

#[test]
fn create_writes_notes_and_modern_comment() {
    let path = tmp("both");
    create(&path, example_slides());

    let names = part_names(&path);
    assert!(names.iter().any(|n| n == "ppt/notesSlides/notesSlide1.xml"));
    assert!(names
        .iter()
        .any(|n| n == "ppt/notesMasters/notesMaster1.xml"));
    assert!(names
        .iter()
        .any(|n| n == "ppt/notesMasters/_rels/notesMaster1.xml.rels"));
    let master_rels = part_text(&path, "ppt/notesMasters/_rels/notesMaster1.xml.rels");
    assert!(master_rels.contains("/theme"));
    assert!(names.iter().any(|n| n == "ppt/theme/theme2.xml"));
    assert!(!master_rels.contains("theme1.xml"));
    assert!(names
        .iter()
        .any(|n| n.starts_with("ppt/comments/modernComment_")));
    assert!(names.iter().any(|n| n == "ppt/authors.xml"));
    assert!(!names.iter().any(|n| n.contains("notesSlide2")));

    let pres = part_text(&path, "ppt/presentation.xml");
    assert!(pres.contains("notesMasterIdLst"));
    let slide_rels = part_text(&path, "ppt/slides/_rels/slide1.xml.rels");
    assert!(slide_rels.contains("/notesSlide"));
    assert!(slide_rels.contains("office/2018/10/relationships/comments"));
    let slide2_rels = part_text(&path, "ppt/slides/_rels/slide2.xml.rels");
    assert!(!slide2_rels.contains("/notesSlide"));
    assert!(!slide2_rels.contains("relationships/comments"));

    let bare = inspect_bare(&path);
    assert_eq!(bare["slides"][0]["hasNotes"], true);
    assert_eq!(bare["slides"][1]["hasNotes"], false);
    assert!(bare["slides"][0].get("notes").is_none());
    assert!(bare["slides"][0].get("comments").is_none());
    assert_eq!(bare["commentCount"], 1);
    assert_eq!(bare["slides"][0]["commentCount"], 1);
    assert_eq!(bare["slides"][1]["commentCount"], 0);

    let notes_tree = inspect(&path, json!({ "notes": true }));
    assert_eq!(notes_tree["slides"][0]["notes"]["id"], "slide:1/notes");
    assert_eq!(notes_body(&notes_tree["slides"][0]["notes"]), "演讲者备注");
    assert_eq!(notes_tree["slides"][1]["notes"], Value::Null);

    let comments_tree = inspect(&path, json!({ "comments": true }));
    let c = &comments_tree["slides"][0]["comments"][0];
    let id = c["id"].as_str().unwrap();
    assert!(id.starts_with("slide:1/comment:{"));
    assert_eq!(c["text"], "一条批注");
    assert_eq!(c["author"], "Alice");
    assert_eq!(c["initials"], "A");
    assert_eq!(c["date"], "2026-09-01T00:00:00.000");
    assert_eq!(c["x"], 914400);
    assert_eq!(c["y"], 914400);
    assert_eq!(c["kind"], "normal");
    assert_eq!(c["replies"], json!([]));
    assert_eq!(comments_tree["slides"][1]["comments"], json!([]));
}

#[test]
fn create_empty_notes_still_writes_notes_slide() {
    let path = tmp("empty-notes");
    create(
        &path,
        json!([{ "shapes": [{ "text": "甲" }], "notes": "" }]),
    );
    let bare = inspect_bare(&path);
    assert_eq!(bare["slides"][0]["hasNotes"], true);
    let tree = inspect(&path, json!({ "notes": true }));
    assert_eq!(notes_body(&tree["slides"][0]["notes"]), "");
}

#[test]
fn create_omits_notes_and_comments_when_absent() {
    let path = tmp("omit");
    create(&path, json!([{ "shapes": [{ "text": "甲" }] }]));
    let names = part_names(&path);
    assert!(!names.iter().any(|n| n.contains("notesSlide")));
    assert!(!names.iter().any(|n| n.contains("modernComment")));
    let bare = inspect_bare(&path);
    assert_eq!(bare["slides"][0]["hasNotes"], false);
    assert_eq!(bare["commentCount"], 0);
}

#[test]
fn create_two_comments_share_one_part() {
    let path = tmp("two-cm");
    create(
        &path,
        json!([{
            "shapes": [{ "text": "甲" }],
            "comments": [
                { "text": "批注甲", "author": "Alice", "initials": "A" },
                { "text": "批注乙", "author": "Bob", "initials": "B" }
            ]
        }]),
    );
    let n: Vec<_> = part_names(&path)
        .into_iter()
        .filter(|n| n.starts_with("ppt/comments/"))
        .collect();
    assert_eq!(n.len(), 1);
    let tree = inspect(&path, json!({ "comments": true }));
    assert_eq!(tree["commentCount"], 2);
    assert_eq!(tree["slides"][0]["comments"][0]["text"], "批注甲");
    assert_eq!(tree["slides"][0]["comments"][1]["text"], "批注乙");
}

#[test]
fn create_rejects_notes_on_presentation_root_and_unknown_comment_fields() {
    let bad = tmp("root-notes");
    let err = create_err(
        &bad,
        json!({
            "notes": "根上",
            "slides": [{ "shapes": [{ "text": "甲" }] }]
        }),
    );
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!bad.exists());

    let bad = tmp("replies");
    let err = create_err(
        &bad,
        json!({
            "slides": [{
                "shapes": [{ "text": "甲" }],
                "comments": [{ "text": "甲", "replies": [] }]
            }]
        }),
    );
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!bad.exists());

    let bad = tmp("xy");
    let err = create_err(
        &bad,
        json!({
            "slides": [{
                "shapes": [{ "text": "甲" }],
                "comments": [{ "text": "甲", "x": 1 }]
            }]
        }),
    );
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!bad.exists());

    let bad = tmp("shape-notes");
    let err = create_err(
        &bad,
        json!({
            "slides": [{
                "shapes": [{ "text": "甲", "notes": "框上" }]
            }]
        }),
    );
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!bad.exists());
}
