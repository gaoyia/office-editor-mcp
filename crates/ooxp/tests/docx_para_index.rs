//! `para:index` 在同一次 apply 里按开始时的段落解释。

use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::json;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join("ooxp-docx-para-index");
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(format!("{tag}-{n}.docx"))
}

fn create_five(tag: &str) -> PathBuf {
    let path = tmp(tag);
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "document": {
                "paragraphs": [
                    { "text": "P1" },
                    { "text": "P2" },
                    { "text": "P3" },
                    { "text": "P4" },
                    { "text": "P5" }
                ]
            }
        })
        .to_string(),
        None,
    )
    .unwrap();
    path
}

fn part(path: &Path, name: &str) -> String {
    let bytes = std::fs::read(path).unwrap();
    let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut file = zip.by_name(name).unwrap();
    let mut out = String::new();
    file.read_to_string(&mut out).unwrap();
    out
}

fn rewrite(path: &Path, name: &str, next: &str) {
    let bytes = std::fs::read(path).unwrap();
    let mut zip_in = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut out = Cursor::new(Vec::new());
    {
        let mut zip_out = ZipWriter::new(&mut out);
        let opt = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        for i in 0..zip_in.len() {
            let mut file = zip_in.by_index(i).unwrap();
            let n = file.name().to_string();
            let mut data = Vec::new();
            file.read_to_end(&mut data).unwrap();
            if n == name {
                data = next.as_bytes().to_vec();
            }
            zip_out.start_file(&n, opt).unwrap();
            zip_out.write_all(&data).unwrap();
        }
        zip_out.finish().unwrap();
    }
    std::fs::write(path, out.into_inner()).unwrap();
}

fn strip_attr(xml: &str, name: &str) -> String {
    let key = format!(" {name}=\"");
    let mut out = String::new();
    let mut rest = xml;
    while let Some(i) = rest.find(&key) {
        out.push_str(&rest[..i]);
        let after = &rest[i + key.len()..];
        let end = after.find('"').unwrap();
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    out
}

fn strip_para_id(path: &Path) {
    let xml = part(path, "word/document.xml");
    let stripped = strip_attr(&strip_attr(&xml, "w14:paraId"), "w14:textId");
    rewrite(path, "word/document.xml", &stripped);
}

fn texts(path: &Path) -> Vec<String> {
    let RunOutput::Inspect(tree) = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": path.to_string_lossy(),
            "scope": { "from": 1, "maxParas": 10 }
        })
        .to_string(),
        None,
    )
    .unwrap() else {
        panic!("inspect");
    };
    tree["paragraphs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["text"].as_str().unwrap_or("").to_string())
        .collect()
}

#[test]
fn insert_then_edit_keeps_original_paragraph() {
    let src = create_five("pin");
    strip_para_id(&src);
    let out = tmp("pin-out");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": src.to_string_lossy(),
            "output": out.to_string_lossy(),
            "ops": [
                { "op": "insertPara", "id": "para:index:1", "text": "NEW" },
                { "op": "setParaText", "id": "para:index:4", "text": "EDIT" }
            ]
        })
        .to_string(),
        None,
    )
    .unwrap();
    assert_eq!(texts(&out), ["P1", "NEW", "P2", "P3", "EDIT", "P5"]);
    let xml = part(&out, "word/document.xml");
    assert!(!xml.contains("ooxp:pin"), "{xml}");
}

#[test]
fn delete_then_reuse_index_is_not_found() {
    let src = create_five("del");
    strip_para_id(&src);
    let before = std::fs::read(&src).unwrap();
    let out = tmp("del-out");
    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": src.to_string_lossy(),
            "output": out.to_string_lossy(),
            "ops": [
                { "op": "deletePara", "id": "para:index:2" },
                { "op": "setParaText", "id": "para:index:2", "text": "x" }
            ]
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::IdNotFound);
    assert_eq!(std::fs::read(&src).unwrap(), before);
    assert!(!out.exists());
}
