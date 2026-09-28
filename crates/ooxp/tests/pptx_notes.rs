//! 幻灯片备注：名单、改演讲者备注、备注图、跨格式拒绝。

use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::ZipArchive;

const PNG_1X1: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
    0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08, 0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00,
    0x00, 0x03, 0x01, 0x01, 0x00, 0x18, 0xDD, 0x8D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E,
    0x44, 0xAE, 0x42, 0x60, 0x82,
];

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-pptx-notes");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn tmp(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}"))
}

fn write_png(tag: &str) -> PathBuf {
    let path = tmp(tag).with_extension("png");
    std::fs::write(&path, PNG_1X1).unwrap();
    path
}

fn create_deck(tag: &str) -> PathBuf {
    let path = tmp(tag).with_extension("pptx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "presentation": {
                "slides": [
                    { "shapes": [{ "text": "标题" }, { "text": "第二框" }] },
                    { "shapes": [{ "text": "第二页" }] }
                ]
            }
        })
        .to_string(),
        None,
    )
    .unwrap();
    path
}

fn apply(src: &Path, out: &Path, ops: Value) -> Result<RunOutput, ooxp::Error> {
    run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": src.to_string_lossy(),
            "output": out.to_string_lossy(),
            "ops": ops,
        })
        .to_string(),
        None,
    )
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

fn inspect_notes(path: &Path, pictures: bool) -> Value {
    let mut scope = json!({ "notes": true });
    if pictures {
        scope["pictures"] = json!(true);
    }
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

fn object_keys(v: &Value) -> Vec<&str> {
    v.as_object().unwrap().keys().map(String::as_str).collect()
}

fn part_bytes(path: &Path, name: &str) -> Vec<u8> {
    let bytes = std::fs::read(path).unwrap();
    let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut file = zip.by_name(name).unwrap();
    let mut out = Vec::new();
    file.read_to_end(&mut out).unwrap();
    out
}

fn part_names(path: &Path) -> Vec<String> {
    let bytes = std::fs::read(path).unwrap();
    let zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    zip.file_names().map(str::to_string).collect()
}

fn set_notes(src: &Path, out: &Path, id: &str, text: &str) {
    apply(
        src,
        out,
        json!([{ "op": "setNotesText", "id": id, "text": text }]),
    )
    .unwrap();
}

fn body_text(notes: &Value) -> String {
    notes["shapes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["text"].as_str().is_some_and(|t| !t.is_empty()))
        .and_then(|s| s["text"].as_str())
        .unwrap_or("")
        .to_string()
}

#[test]
fn inspect_has_notes_without_notes_object() {
    let path = create_deck("bare");
    let tree = inspect_bare(&path);
    assert_eq!(tree["slides"][0]["hasNotes"], false);
    assert!(tree["slides"][0].get("notes").is_none());
    assert_eq!(
        object_keys(&tree["slides"][0]),
        [
            "id",
            "index",
            "name",
            "layoutId",
            "hidden",
            "transition",
            "hasNotes",
            "pictureCount",
            "videoCount",
            "audioCount",
            "chartCount",
            "smartArtCount",
            "commentCount",
            "hyperlinkCount",
            "tableCount",
            "animCount"
        ]
    );
    let expanded = inspect_notes(&path, false);
    assert_eq!(expanded["slides"][0]["notes"], Value::Null);
    assert_eq!(expanded["slides"][1]["notes"], Value::Null);
}

#[test]
fn set_notes_text_creates_parts_and_inspects() {
    let src = create_deck("create-notes");
    let out = tmp("noted").with_extension("pptx");
    set_notes(&src, &out, "slide:1", "演讲者备注");
    let names = part_names(&out);
    assert!(names.iter().any(|n| n == "ppt/notesSlides/notesSlide1.xml"));
    assert!(names
        .iter()
        .any(|n| n == "ppt/notesMasters/notesMaster1.xml"));
    assert!(names
        .iter()
        .any(|n| n == "ppt/notesMasters/_rels/notesMaster1.xml.rels"));
    assert!(names.iter().any(|n| n == "ppt/theme/theme2.xml"));
    let pres = String::from_utf8(part_bytes(&out, "ppt/presentation.xml")).unwrap();
    assert!(pres.contains("notesMasterIdLst"));
    let tree = inspect_notes(&out, false);
    assert_eq!(tree["slides"][0]["hasNotes"], true);
    assert_eq!(tree["slides"][1]["hasNotes"], false);
    assert_eq!(tree["slides"][1]["notes"], Value::Null);
    let notes = &tree["slides"][0]["notes"];
    assert_eq!(notes["id"], "slide:1/notes");
    assert_eq!(notes["pictureCount"], 0);
    assert!(notes.get("pictures").is_none());
    assert_eq!(
        object_keys(notes),
        [
            "id",
            "pictureCount",
            "chartCount",
            "smartArtCount",
            "shapes"
        ]
    );
    assert_eq!(body_text(notes), "演讲者备注");
    let body = notes["shapes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["id"] == "slide:1/notes/shape:3")
        .unwrap();
    assert_eq!(body["text"], "演讲者备注");
    assert_eq!(body["stable"], true);
}

#[test]
fn set_notes_text_accepts_notes_container_id() {
    let src = create_deck("notes-id");
    let out = tmp("notes-id").with_extension("pptx");
    set_notes(&src, &out, "slide:1/notes", "容器 id");
    let tree = inspect_notes(&out, false);
    assert_eq!(body_text(&tree["slides"][0]["notes"]), "容器 id");
}

#[test]
fn set_shape_text_on_notes_and_rejects_slide_image() {
    let src = create_deck("shape");
    let noted = tmp("noted").with_extension("pptx");
    set_notes(&src, &noted, "slide:1", "原稿");
    let out = tmp("shaped").with_extension("pptx");
    apply(
        &noted,
        &out,
        json!([{
            "op": "setShapeText",
            "id": "slide:1/notes/shape:3",
            "text": "改过的备注"
        }]),
    )
    .unwrap();
    let tree = inspect_notes(&out, false);
    assert_eq!(body_text(&tree["slides"][0]["notes"]), "改过的备注");
    let scoped = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": out.to_string_lossy(),
            "scope": { "slide": 1 }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let RunOutput::Inspect(slide_tree) = scoped else {
        panic!();
    };
    assert_eq!(slide_tree["slides"][0]["shapes"][0]["text"], "标题");

    let err = apply(
        &out,
        &tmp("sldimg").with_extension("pptx"),
        json!([{
            "op": "setShapeText",
            "id": "slide:1/notes/shape:2",
            "text": "缩略图"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

#[test]
fn notes_and_slide_text_do_not_cross() {
    let src = create_deck("cross");
    let noted = tmp("noted").with_extension("pptx");
    set_notes(&src, &noted, "slide:1", "备注原文");
    let out = tmp("cross").with_extension("pptx");
    apply(
        &noted,
        &out,
        json!([
            { "op": "setShapeText", "id": "slide:1/shape:2", "text": "幻灯片新字" },
            { "op": "setNotesText", "id": "slide:1", "text": "备注新字" }
        ]),
    )
    .unwrap();
    let notes = inspect_notes(&out, false);
    assert_eq!(body_text(&notes["slides"][0]["notes"]), "备注新字");
    let RunOutput::Inspect(slide_tree) = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": out.to_string_lossy(),
            "scope": { "slide": 1 }
        })
        .to_string(),
        None,
    )
    .unwrap() else {
        panic!();
    };
    assert_eq!(slide_tree["slides"][0]["shapes"][0]["text"], "幻灯片新字");
}

#[test]
fn insert_picture_on_notes_does_not_join_slide_count() {
    let src = create_deck("pic");
    let png = write_png("a");
    let out = tmp("pic").with_extension("pptx");
    apply(
        &src,
        &out,
        json!([{
            "op": "insertPicture",
            "id": "slide:1/notes",
            "file": png.to_string_lossy(),
            "x": 914400,
            "y": 4572000,
            "cx": 914400,
            "cy": 914400
        }]),
    )
    .unwrap();
    let tree = inspect_notes(&out, true);
    assert_eq!(tree["pictureCount"], 0);
    assert_eq!(tree["slides"][0]["pictureCount"], 0);
    assert!(
        tree["slides"][0].get("pictures").is_none() || tree["slides"][0]["pictures"] == json!([])
    );
    let notes = &tree["slides"][0]["notes"];
    assert_eq!(notes["pictureCount"], 1);
    assert_eq!(notes["pictures"][0]["id"], "slide:1/notes/pic:5");
    assert_eq!(
        object_keys(notes),
        [
            "id",
            "pictureCount",
            "chartCount",
            "smartArtCount",
            "pictures",
            "shapes"
        ]
    );

    let exported = tmp("exp").with_extension("png");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "exportMedia",
            "path": out.to_string_lossy(),
            "id": "slide:1/notes/pic:5",
            "output": exported.to_string_lossy()
        })
        .to_string(),
        None,
    )
    .unwrap();
    assert_eq!(std::fs::read(&exported).unwrap(), PNG_1X1);

    let moved = tmp("pos").with_extension("pptx");
    apply(
        &out,
        &moved,
        json!([{
            "op": "setPicturePos",
            "id": "slide:1/notes/pic:5",
            "x": 1828800,
            "y": 0
        }]),
    )
    .unwrap();
    let after = inspect_notes(&moved, true);
    assert_eq!(after["slides"][0]["notes"]["pictures"][0]["x"], 1828800);

    let deleted = tmp("del").with_extension("pptx");
    apply(
        &moved,
        &deleted,
        json!([{ "op": "deletePicture", "id": "slide:1/notes/pic:5" }]),
    )
    .unwrap();
    let after_del = inspect_notes(&deleted, true);
    assert_eq!(after_del["slides"][0]["notes"]["pictureCount"], 0);
}

#[test]
fn rejects_bad_ids_and_foreign_formats() {
    let src = create_deck("bad");
    let err = apply(
        &src,
        &tmp("shape").with_extension("pptx"),
        json!([{ "op": "setNotesText", "id": "slide:1/shape:2", "text": "x" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    let err = apply(
        &src,
        &tmp("pres").with_extension("pptx"),
        json!([{ "op": "setNotesText", "id": "pptx:presentation", "text": "x" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);

    let xlsx = tmp("book").with_extension("xlsx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": xlsx.to_string_lossy(),
            "workbook": { "sheets": [{ "name": "Sheet1", "cells": [{ "ref": "A1", "value": 1 }] }] }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": xlsx.to_string_lossy(),
            "output": tmp("xlsx").with_extension("xlsx").to_string_lossy(),
            "ops": [{ "op": "setNotesText", "id": "slide:1", "text": "x" }]
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let docx = tmp("doc").with_extension("docx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": docx.to_string_lossy(),
            "document": { "paragraphs": [{ "text": "hi" }] }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": docx.to_string_lossy(),
            "output": tmp("docx").with_extension("docx").to_string_lossy(),
            "ops": [{ "op": "setNotesText", "id": "slide:1", "text": "x" }]
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}
