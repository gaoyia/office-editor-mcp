//! 工作表背景图：不进 drawing，不计 pictureCount。

use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::json;
use zip::ZipArchive;

static SEQ: AtomicU32 = AtomicU32::new(0);

const PNG_A: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
    0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08, 0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00,
    0x00, 0x03, 0x01, 0x01, 0x00, 0x18, 0xDD, 0x8D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E,
    0x44, 0xAE, 0x42, 0x60, 0x82,
];

const PNG_B: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
    0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08, 0xD7, 0x63, 0x60, 0x60, 0x60, 0x00,
    0x00, 0x00, 0x04, 0x00, 0x01, 0xA3, 0x0A, 0x15, 0xE3, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E,
    0x44, 0xAE, 0x42, 0x60, 0x82,
];

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-xlsx-background");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn tmp(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}"))
}

fn write_png(tag: &str, bytes: &[u8]) -> PathBuf {
    let path = tmp(tag).with_extension("png");
    std::fs::write(&path, bytes).unwrap();
    path
}

fn inspect(path: &Path) -> serde_json::Value {
    let RunOutput::Inspect(tree) = run(&json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": path.to_string_lossy(),
        "scope": { "pictures": true }
    })
    .to_string())
    .unwrap() else {
        panic!("inspect 应返回树");
    };
    tree
}

fn apply(src: &Path, out: &Path, ops: serde_json::Value) -> Result<RunOutput, ooxp::Error> {
    run(&json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": out.to_string_lossy(),
        "ops": ops,
    })
    .to_string())
}

fn part_text(path: &Path, name: &str) -> String {
    let file = std::fs::File::open(path).unwrap();
    let mut zip = ZipArchive::new(file).unwrap();
    let mut f = zip.by_name(name).unwrap();
    let mut s = String::new();
    f.read_to_string(&mut s).unwrap();
    s
}

fn part_names(path: &Path) -> Vec<String> {
    let file = std::fs::File::open(path).unwrap();
    let mut zip = ZipArchive::new(file).unwrap();
    (0..zip.len())
        .map(|i| zip.by_index(i).unwrap().name().to_string())
        .collect()
}

#[test]
fn create_set_replace_delete_and_export() {
    let png_a = write_png("a", PNG_A);
    let png_b = write_png("b", PNG_B);
    let created = tmp("create").with_extension("xlsx");
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": created.to_string_lossy(),
        "workbook": {
            "sheets": [
                { "name": "Sheet1", "background": png_a.to_string_lossy() },
                { "name": "Sheet2" }
            ]
        }
    })
    .to_string())
    .unwrap();

    let tree = inspect(&created);
    assert_eq!(tree["pictureCount"], 0);
    assert_eq!(tree["sheets"][0]["pictureCount"], 0);
    assert_eq!(
        tree["sheets"][0]["background"]["id"],
        "sheet:Sheet1/background"
    );
    assert_eq!(tree["sheets"][0]["background"]["contentType"], "image/png");
    assert_eq!(tree["sheets"][0]["pictures"], json!([]));
    assert_eq!(tree["sheets"][1]["background"], serde_json::Value::Null);
    let xml = part_text(&created, "xl/worksheets/sheet1.xml");
    assert!(xml.contains("<picture"));
    assert!(xml.contains("r:id="));
    assert!(!part_names(&created).iter().any(|n| n.contains("drawing")));
    assert!(part_names(&created)
        .iter()
        .any(|n| n.contains("image1.png")));

    let exported = tmp("exp").with_extension("png");
    run(&json!({
        "schemaVersion": 1,
        "action": "exportMedia",
        "path": created.to_string_lossy(),
        "id": "sheet:Sheet1/background",
        "output": exported.to_string_lossy()
    })
    .to_string())
    .unwrap();
    assert_eq!(std::fs::read(&exported).unwrap(), PNG_A);

    let replaced = tmp("replace").with_extension("xlsx");
    apply(
        &created,
        &replaced,
        json!([{
            "op": "setSheetBackground",
            "id": "sheet:Sheet1",
            "file": png_b.to_string_lossy()
        }]),
    )
    .unwrap();
    let tree_r = inspect(&replaced);
    assert_eq!(
        tree_r["sheets"][0]["background"]["id"],
        "sheet:Sheet1/background"
    );
    assert_eq!(tree_r["pictureCount"], 0);
    assert_eq!(
        part_text(&replaced, "xl/worksheets/sheet1.xml")
            .matches("<picture")
            .count(),
        1
    );
    let exported_b = tmp("exp-b").with_extension("png");
    run(&json!({
        "schemaVersion": 1,
        "action": "exportMedia",
        "path": replaced.to_string_lossy(),
        "id": "sheet:Sheet1/background",
        "output": exported_b.to_string_lossy()
    })
    .to_string())
    .unwrap();
    assert_eq!(std::fs::read(&exported_b).unwrap(), PNG_B);
    let names_r = part_names(&replaced);
    assert!(names_r.iter().any(|n| n.ends_with("image2.png")));
    assert!(!names_r.iter().any(|n| n.ends_with("image1.png")));

    let deleted = tmp("del").with_extension("xlsx");
    apply(
        &replaced,
        &deleted,
        json!([{ "op": "deleteSheetBackground", "id": "sheet:Sheet1/background" }]),
    )
    .unwrap();
    let tree_d = inspect(&deleted);
    assert_eq!(tree_d["sheets"][0]["background"], serde_json::Value::Null);
    assert!(!part_text(&deleted, "xl/worksheets/sheet1.xml").contains("<picture"));
    let names_d = part_names(&deleted);
    assert!(!names_d.iter().any(|n| n.contains("/media/")));
    assert!(!names_d.iter().any(|n| n.contains("sheet1.xml.rels")));

    let missing = apply(
        &deleted,
        &tmp("miss").with_extension("xlsx"),
        json!([{ "op": "deleteSheetBackground", "id": "sheet:Sheet1/background" }]),
    )
    .unwrap_err();
    assert_eq!(missing.code(), ErrorCode::IdNotFound);
}

#[test]
fn rejects_wrong_id_and_non_worksheet() {
    let png = write_png("id", PNG_A);
    let src = tmp("src").with_extension("xlsx");
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": src.to_string_lossy(),
        "workbook": { "sheets": [{ "name": "Sheet1", "cells": [{ "ref": "A1", "value": 1 }] }] }
    })
    .to_string())
    .unwrap();

    let cell = apply(
        &src,
        &tmp("cell").with_extension("xlsx"),
        json!([{ "op": "setSheetBackground", "id": "cell:Sheet1!A1", "file": png.to_string_lossy() }]),
    )
    .unwrap_err();
    assert_eq!(cell.code(), ErrorCode::InvalidId);

    let pic = apply(
        &src,
        &tmp("pic").with_extension("xlsx"),
        json!([{ "op": "deleteSheetBackground", "id": "sheet:Sheet1/pic:1" }]),
    )
    .unwrap_err();
    assert_eq!(pic.code(), ErrorCode::InvalidId);

    let sheet_id = apply(
        &src,
        &tmp("sheet-id").with_extension("xlsx"),
        json!([{ "op": "deleteSheetBackground", "id": "sheet:Sheet1" }]),
    )
    .unwrap_err();
    assert_eq!(sheet_id.code(), ErrorCode::InvalidId);

    let gone = apply(
        &src,
        &tmp("gone").with_extension("xlsx"),
        json!([{ "op": "setSheetBackground", "id": "sheet:NoSuch", "file": png.to_string_lossy() }]),
    )
    .unwrap_err();
    assert_eq!(gone.code(), ErrorCode::IdNotFound);

    let charted = tmp("chart").with_extension("xlsx");
    apply(
        &src,
        &charted,
        json!([{
            "op": "insertChartSheet",
            "id": "xlsx:workbook",
            "name": "Chart1",
            "dataFrom": "A10",
            "type": "col",
            "series": [{ "name": "Q1", "categories": ["甲"], "values": [1] }]
        }]),
    )
    .unwrap();
    let tree = inspect(&charted);
    let cs = tree["sheets"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["name"] == "Chart1")
        .unwrap();
    assert!(cs.get("background").is_none());
    let blocked = apply(
        &charted,
        &tmp("cs").with_extension("xlsx"),
        json!([{ "op": "setSheetBackground", "id": "sheet:Chart1", "file": png.to_string_lossy() }]),
    )
    .unwrap_err();
    assert_eq!(blocked.code(), ErrorCode::InvalidField);

    let docx = tmp("doc").with_extension("docx");
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": docx.to_string_lossy(),
        "document": { "paragraphs": [{ "text": "a" }] }
    })
    .to_string())
    .unwrap();
    let foreign = apply(
        &docx,
        &tmp("doc-out").with_extension("docx"),
        json!([{ "op": "setSheetBackground", "id": "sheet:Sheet1", "file": png.to_string_lossy() }]),
    )
    .unwrap_err();
    assert_eq!(foreign.code(), ErrorCode::InvalidField);
}
