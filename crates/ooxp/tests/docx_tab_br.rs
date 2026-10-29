//! 写入 `\t` / `\n` 时还原成 `w:tab` / `w:br`，再读回同一段文字。

use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, RunOutput};
use serde_json::json;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join("ooxp-docx-tab-br");
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(format!("{tag}-{n}.docx"))
}

fn create_one(tag: &str, text: &str) -> PathBuf {
    let path = tmp(tag);
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "document": { "paragraphs": [{ "text": text }] }
        })
        .to_string(),
        None,
    )
    .unwrap();
    path
}

fn part(path: &Path) -> String {
    let bytes = std::fs::read(path).unwrap();
    let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut file = zip.by_name("word/document.xml").unwrap();
    let mut out = String::new();
    file.read_to_string(&mut out).unwrap();
    out
}

fn rewrite(path: &Path, xml: &str) {
    let bytes = std::fs::read(path).unwrap();
    let mut zip_in = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut out = Cursor::new(Vec::new());
    {
        let mut zip_out = ZipWriter::new(&mut out);
        let opt = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        for i in 0..zip_in.len() {
            let mut file = zip_in.by_index(i).unwrap();
            let name = file.name().to_string();
            let mut data = Vec::new();
            file.read_to_end(&mut data).unwrap();
            if name == "word/document.xml" {
                data = xml.as_bytes().to_vec();
            }
            zip_out.start_file(&name, opt).unwrap();
            zip_out.write_all(&data).unwrap();
        }
        zip_out.finish().unwrap();
    }
    std::fs::write(path, out.into_inner()).unwrap();
}

fn para_text(path: &Path) -> String {
    let RunOutput::Inspect(tree) = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": path.to_string_lossy(),
            "scope": { "from": 1, "maxParas": 5 }
        })
        .to_string(),
        None,
    )
    .unwrap() else {
        panic!("inspect");
    };
    tree["paragraphs"][0]["text"].as_str().unwrap().to_string()
}

fn para_id(path: &Path) -> String {
    let RunOutput::Inspect(tree) = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": path.to_string_lossy(),
            "scope": { "from": 1, "maxParas": 5 }
        })
        .to_string(),
        None,
    )
    .unwrap() else {
        panic!("inspect");
    };
    tree["paragraphs"][0]["id"].as_str().unwrap().to_string()
}

fn set_text(src: &Path, out: &Path, id: &str, text: &str) {
    run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": src.to_string_lossy(),
            "output": out.to_string_lossy(),
            "ops": [{ "op": "setParaText", "id": id, "text": text }]
        })
        .to_string(),
        None,
    )
    .unwrap();
}

#[test]
fn create_writes_tab_and_break() {
    let path = create_one("create", "a\tb\nc");
    let xml = part(&path);
    assert!(
        xml.contains("<w:t>a</w:t><w:tab/><w:t>b</w:t><w:br/><w:t>c</w:t>"),
        "{xml}"
    );
    assert_eq!(para_text(&path), "a\tb\nc");
}

#[test]
fn handwritten_break_is_cleared_and_roundtrips() {
    let src = create_one("hand", "placeholder");
    let xml = part(&src).replace("<w:t>placeholder</w:t>", "<w:t>a</w:t><w:br/><w:t>b</w:t>");
    rewrite(&src, &xml);
    assert_eq!(para_text(&src), "a\nb");
    let id = para_id(&src);

    let single = tmp("single");
    set_text(&src, &single, &id, "one");
    let xml = part(&single);
    assert!(!xml.contains("<w:br"), "{xml}");
    assert_eq!(para_text(&single), "one");

    let multi = tmp("multi");
    set_text(&single, &multi, &id, "a\tb\nc");
    let xml = part(&multi);
    assert!(
        xml.contains("<w:t>a</w:t><w:tab/><w:t>b</w:t><w:br/><w:t>c</w:t>"),
        "{xml}"
    );
    assert_eq!(para_text(&multi), "a\tb\nc");

    let back = tmp("back");
    set_text(&multi, &back, &id, "one");
    assert!(!part(&back).contains("<w:br"));
    assert!(!part(&back).contains("<w:tab"));
    assert_eq!(para_text(&back), "one");
}
