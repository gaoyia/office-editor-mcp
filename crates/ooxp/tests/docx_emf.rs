//! docx 嵌入 EMF / WMF / TIFF。

mod common;

use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::ZipArchive;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-docx-emf");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn seq_path(tag: &str, ext: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}.{ext}"))
}

fn write_bytes(tag: &str, ext: &str, bytes: &[u8]) -> PathBuf {
    let path = seq_path(tag, ext);
    std::fs::write(&path, bytes).unwrap();
    path
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

fn inspect_pics(path: &Path) -> Value {
    let RunOutput::Inspect(tree) = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": path.to_string_lossy(),
            "scope": { "from": 1, "maxParas": 8, "pictures": true, "headerFooter": true }
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

fn zip_names(path: &Path) -> Vec<String> {
    let bytes = std::fs::read(path).unwrap();
    let zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    zip.file_names().map(|s| s.to_string()).collect()
}

fn part_text(path: &Path, name: &str) -> String {
    let bytes = std::fs::read(path).unwrap();
    let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut file = zip.by_name(name.trim_start_matches('/')).unwrap();
    let mut s = String::new();
    file.read_to_string(&mut s).unwrap();
    s
}

fn first_pic(tree: &Value) -> &Value {
    let paras = tree["paragraphs"].as_array().unwrap();
    for p in paras {
        if let Some(pics) = p["pictures"].as_array() {
            if let Some(pic) = pics.first() {
                return pic;
            }
        }
    }
    panic!("树上没有图：{tree}");
}

#[test]
fn insert_emf_writes_original_and_content_type() {
    let src = create_docx("src", json!({ "paragraphs": [{ "text": "host" }] }));
    let tree0 = inspect_pics(&src);
    let para = tree0["paragraphs"][0]["id"].as_str().unwrap();
    let emf = write_bytes("in", "emf", &common::emf_header());
    let out = seq_path("ins", "docx");
    apply(
        &src,
        &out,
        json!([{ "op": "insertPicture", "id": para, "file": emf.to_string_lossy() }]),
    );
    let tree = inspect_pics(&out);
    let pic = first_pic(&tree);
    assert_eq!(pic["contentType"], "image/x-emf");
    let names = zip_names(&out);
    assert!(names.iter().any(|n| n.ends_with(".emf")), "{names:?}");
    let doc = part_text(&out, "word/document.xml");
    assert!(!doc.contains("svgBlip"), "{doc}");
    let exp = seq_path("exp", "emf");
    let RunOutput::Written { content_type, .. } = run(
        &json!({
            "schemaVersion": 1,
            "action": "exportMedia",
            "path": out.to_string_lossy(),
            "id": pic["id"],
            "output": exp.to_string_lossy()
        })
        .to_string(),
        None,
    )
    .unwrap() else {
        panic!("export 应写出");
    };
    assert_eq!(content_type.as_deref(), Some("image/x-emf"));
    assert_eq!(std::fs::read(&exp).unwrap(), common::emf_header());
}

#[test]
fn create_wmf() {
    let wmf = write_bytes("c", "wmf", &common::placeable_wmf());
    let path = create_docx(
        "create-wmf",
        json!({
            "paragraphs": [{
                "text": "host",
                "pictures": [{ "file": wmf.to_string_lossy() }]
            }]
        }),
    );
    assert_eq!(
        first_pic(&inspect_pics(&path))["contentType"],
        "image/x-wmf"
    );
}

#[test]
fn reject_junk() {
    let src = create_docx("rej", json!({ "paragraphs": [{ "text": "host" }] }));
    let para = inspect_pics(&src)["paragraphs"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let junk = write_bytes("j", "bin", b"not-an-image");
    let out = seq_path("should-not", "docx");
    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": src.to_string_lossy(),
            "output": out.to_string_lossy(),
            "ops": [{ "op": "insertPicture", "id": para, "file": junk.to_string_lossy() }]
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());
}
