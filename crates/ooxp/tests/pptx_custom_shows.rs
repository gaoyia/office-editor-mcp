//! 自定义放映：inspect `customShowCount` / `scope.customShows`，以及三条 op。

use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::ZipArchive;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-pptx-custom-shows");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn tmp(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}"))
}

fn create_deck(tag: &str, n: usize) -> PathBuf {
    let path = tmp(tag).with_extension("pptx");
    let slides: Vec<Value> = (1..=n)
        .map(|i| json!({ "shapes": [{ "text": format!("页{i}") }] }))
        .collect();
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

fn inspect_scope(path: &Path, scope: Option<Value>) -> Value {
    let mut req = json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": path.to_string_lossy()
    });
    if let Some(scope) = scope {
        req["scope"] = scope;
    }
    let RunOutput::Inspect(tree) = run(&req.to_string(), None).unwrap() else {
        panic!("inspect 应返回树");
    };
    tree
}

fn inspect_err(path: &Path, scope: Value) -> ooxp::Error {
    run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": path.to_string_lossy(),
            "scope": scope
        })
        .to_string(),
        None,
    )
    .unwrap_err()
}

fn object_keys(v: &Value) -> Vec<&str> {
    v.as_object().unwrap().keys().map(String::as_str).collect()
}

fn part_text(path: &Path, name: &str) -> String {
    let bytes = std::fs::read(path).unwrap();
    let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut file = zip.by_name(name).unwrap();
    let mut out = Vec::new();
    file.read_to_end(&mut out).unwrap();
    String::from_utf8(out).unwrap()
}

fn insert_tour(src: &Path) -> PathBuf {
    let out = tmp("tour").with_extension("pptx");
    apply(
        src,
        &out,
        json!([{
            "op": "insertCustomShow",
            "id": "pptx:presentation",
            "name": "Tour",
            "slides": ["slide:1", "slide:3"]
        }]),
    )
    .unwrap();
    out
}

#[test]
fn create_has_zero_custom_show_count_without_list() {
    let path = create_deck("bare", 2);
    let tree = inspect_scope(&path, None);
    assert_eq!(tree["customShowCount"], 0);
    assert!(tree.get("customShows").is_none());
    assert_eq!(
        object_keys(&tree),
        [
            "schemaVersion",
            "format",
            "id",
            "slideCount",
            "pictureCount",
            "videoCount",
            "audioCount",
            "chartCount",
            "smartArtCount",
            "commentCount",
            "hyperlinkCount",
            "tableCount",
            "oleObjectCount",
            "zoomCount",
            "connectorCount",
            "animCount",
            "masterCount",
            "hasHandout",
            "sectionCount",
            "slideSize",
            "customShowCount",
            "props",
            "slides"
        ]
    );
    let listed = inspect_scope(&path, Some(json!({ "customShows": true })));
    assert_eq!(listed["customShowCount"], 0);
    assert_eq!(listed["customShows"], json!([]));
    assert!(listed["slides"][0].get("shapes").is_none());
    let xml = part_text(&path, "ppt/presentation.xml");
    assert!(!xml.contains("custShowLst"));
}

#[test]
fn insert_lists_name_and_slides_with_rid() {
    let src = create_deck("cs2", 3);
    let out = insert_tour(&src);
    let tree = inspect_scope(&out, Some(json!({ "customShows": true })));
    assert_eq!(tree["customShowCount"], 1);
    let shows = tree["customShows"].as_array().unwrap();
    assert_eq!(object_keys(&shows[0]), ["id", "name", "slides"]);
    assert_eq!(shows[0]["id"], "show:1");
    assert_eq!(shows[0]["name"], "Tour");
    assert_eq!(shows[0]["slides"], json!(["slide:1", "slide:3"]));
    let xml = part_text(&out, "ppt/presentation.xml");
    let notes = xml.find("notesSz").expect("应有 notesSz");
    let cust = xml.find("custShowLst").expect("应有 custShowLst");
    assert!(cust > notes, "custShowLst 应在 notesSz 之后");
    assert!(xml.contains("name=\"Tour\""));
    assert!(xml.contains("id=\"0\""));
    assert!(xml.contains("<p:sld r:id="));
    let photo = xml.find("photoAlbum");
    if let Some(photo) = photo {
        assert!(cust < photo);
    }
}

#[test]
fn empty_and_duplicate_slides_allowed() {
    let src = create_deck("empty", 2);
    let empty = tmp("empty").with_extension("pptx");
    apply(
        &src,
        &empty,
        json!([{
            "op": "insertCustomShow",
            "id": "pptx:presentation",
            "name": "Empty"
        }]),
    )
    .unwrap();
    let tree = inspect_scope(&empty, Some(json!({ "customShows": true })));
    assert_eq!(tree["customShows"][0]["slides"], json!([]));
    let xml = part_text(&empty, "ppt/presentation.xml");
    assert!(xml.contains("<p:sldLst"));

    let dup = tmp("dup").with_extension("pptx");
    apply(
        &src,
        &dup,
        json!([{
            "op": "insertCustomShow",
            "id": "pptx:presentation",
            "name": "Dup",
            "slides": ["slide:1", "slide:1"]
        }]),
    )
    .unwrap();
    let tree = inspect_scope(&dup, Some(json!({ "customShows": true })));
    assert_eq!(
        tree["customShows"][0]["slides"],
        json!(["slide:1", "slide:1"])
    );
}

#[test]
fn after_inserts_between_and_ids_increment() {
    let src = create_deck("after", 2);
    let first = tmp("first").with_extension("pptx");
    apply(
        &src,
        &first,
        json!([
            { "op": "insertCustomShow", "id": "pptx:presentation", "name": "A" },
            { "op": "insertCustomShow", "id": "pptx:presentation", "name": "C" }
        ]),
    )
    .unwrap();
    let out = tmp("mid").with_extension("pptx");
    apply(
        &first,
        &out,
        json!([{
            "op": "insertCustomShow",
            "id": "pptx:presentation",
            "name": "B",
            "after": "show:1"
        }]),
    )
    .unwrap();
    let tree = inspect_scope(&out, Some(json!({ "customShows": true })));
    assert_eq!(tree["customShowCount"], 3);
    assert_eq!(tree["customShows"][0]["name"], "A");
    assert_eq!(tree["customShows"][1]["name"], "B");
    assert_eq!(tree["customShows"][2]["name"], "C");
    let xml = part_text(&out, "ppt/presentation.xml");
    assert!(xml.contains("id=\"0\""));
    assert!(xml.contains("id=\"1\""));
    assert!(xml.contains("id=\"2\""));
}

#[test]
fn set_renames_and_replaces_slides() {
    let src = create_deck("set", 3);
    let tour = insert_tour(&src);
    let out = tmp("set").with_extension("pptx");
    apply(
        &tour,
        &out,
        json!([{
            "op": "setCustomShow",
            "id": "show:1",
            "name": "Guide",
            "slides": ["slide:3", "slide:1"]
        }]),
    )
    .unwrap();
    let tree = inspect_scope(&out, Some(json!({ "customShows": true })));
    assert_eq!(tree["customShows"][0]["name"], "Guide");
    assert_eq!(
        tree["customShows"][0]["slides"],
        json!(["slide:3", "slide:1"])
    );
}

#[test]
fn delete_last_drops_list() {
    let src = create_deck("del", 2);
    let with = tmp("with").with_extension("pptx");
    apply(
        &src,
        &with,
        json!([{ "op": "insertCustomShow", "id": "pptx:presentation", "name": "Tour" }]),
    )
    .unwrap();
    let out = tmp("gone").with_extension("pptx");
    apply(
        &with,
        &out,
        json!([{ "op": "deleteCustomShow", "id": "show:1" }]),
    )
    .unwrap();
    let tree = inspect_scope(&out, Some(json!({ "customShows": true })));
    assert_eq!(tree["customShowCount"], 0);
    assert_eq!(tree["customShows"], json!([]));
    let xml = part_text(&out, "ppt/presentation.xml");
    assert!(!xml.contains("custShowLst"));
}

#[test]
fn delete_slide_strips_rid_but_keeps_empty_show() {
    let src = create_deck("delsld", 3);
    let with = tmp("with-sld").with_extension("pptx");
    apply(
        &src,
        &with,
        json!([{
            "op": "insertCustomShow",
            "id": "pptx:presentation",
            "name": "Tour",
            "slides": ["slide:1"]
        }]),
    )
    .unwrap();
    let xml_before = part_text(&with, "ppt/presentation.xml");
    let rid = xml_before
        .split("<p:sld r:id=\"")
        .nth(1)
        .and_then(|s| s.split('"').next())
        .expect("应有 p:sld r:id")
        .to_string();
    let out = tmp("delsld-out").with_extension("pptx");
    apply(
        &with,
        &out,
        json!([{ "op": "deleteSlide", "id": "slide:1" }]),
    )
    .unwrap();
    let tree = inspect_scope(&out, Some(json!({ "customShows": true })));
    assert_eq!(tree["slideCount"], 2);
    assert_eq!(tree["customShowCount"], 1);
    assert_eq!(tree["customShows"][0]["name"], "Tour");
    assert_eq!(tree["customShows"][0]["slides"], json!([]));
    let xml = part_text(&out, "ppt/presentation.xml");
    assert!(xml.contains("custShowLst"));
    assert!(
        !xml.contains(&format!("r:id=\"{rid}\"")),
        "被删页的关系 id 应从自定义放映消失"
    );
}

#[test]
fn insert_slide_does_not_touch_custom_show() {
    let src = create_deck("ins", 2);
    let with = tmp("with-ins").with_extension("pptx");
    apply(
        &src,
        &with,
        json!([{
            "op": "insertCustomShow",
            "id": "pptx:presentation",
            "name": "Tour",
            "slides": ["slide:1", "slide:2"]
        }]),
    )
    .unwrap();
    let out = tmp("ins-out").with_extension("pptx");
    apply(
        &with,
        &out,
        json!([{ "op": "insertSlide", "id": "pptx:presentation" }]),
    )
    .unwrap();
    let tree = inspect_scope(&out, Some(json!({ "customShows": true })));
    assert_eq!(tree["slideCount"], 3);
    assert_eq!(
        tree["customShows"][0]["slides"],
        json!(["slide:1", "slide:2"])
    );
}

#[test]
fn rejects_bad_id_empty_name_taken_and_need_field() {
    let src = create_deck("bad", 2);
    let with = tmp("named").with_extension("pptx");
    apply(
        &src,
        &with,
        json!([{ "op": "insertCustomShow", "id": "pptx:presentation", "name": "Tour" }]),
    )
    .unwrap();

    let bad_id = tmp("bad-id").with_extension("pptx");
    let err = apply(
        &src,
        &bad_id,
        json!([{ "op": "insertCustomShow", "id": "slide:1", "name": "Tour" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    assert!(!bad_id.exists());

    let empty = tmp("empty-name").with_extension("pptx");
    let err = apply(
        &src,
        &empty,
        json!([{ "op": "insertCustomShow", "id": "pptx:presentation", "name": "" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!empty.exists());

    let long = tmp("long-name").with_extension("pptx");
    let err = apply(
        &src,
        &long,
        json!([{
            "op": "insertCustomShow",
            "id": "pptx:presentation",
            "name": "abcdefghijklmnopqrstuvwxyz012345"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!long.exists());

    let taken = tmp("taken").with_extension("pptx");
    let err = apply(
        &with,
        &taken,
        json!([{ "op": "insertCustomShow", "id": "pptx:presentation", "name": "Tour" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!taken.exists());

    let need = tmp("need").with_extension("pptx");
    let err = apply(
        &with,
        &need,
        json!([{ "op": "setCustomShow", "id": "show:1" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!need.exists());

    let missing = tmp("missing").with_extension("pptx");
    let err = apply(
        &src,
        &missing,
        json!([{ "op": "deleteCustomShow", "id": "show:1" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::IdNotFound);
    assert!(!missing.exists());
}

#[test]
fn create_rejects_presentation_custom_shows() {
    let bad = tmp("create-cs").with_extension("pptx");
    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": bad.to_string_lossy(),
            "presentation": {
                "customShows": [],
                "slides": [{ "shapes": [{ "text": "甲" }] }]
            }
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!bad.exists());
}

#[test]
fn xlsx_and_docx_reject_custom_shows() {
    let xlsx = tmp_dir().join("scope.xlsx");
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
    let err = inspect_err(&xlsx, json!({ "customShows": true }));
    assert_eq!(err.code(), ErrorCode::InvalidField);
    let out = tmp_dir().join("xlsx-cs.xlsx");
    let err = apply(
        &xlsx,
        &out,
        json!([{ "op": "insertCustomShow", "id": "pptx:presentation", "name": "Tour" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());

    let docx = tmp_dir().join("scope.docx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": docx.to_string_lossy(),
            "document": { "paragraphs": [{ "text": "甲" }] }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let err = inspect_err(&docx, json!({ "customShows": true }));
    assert_eq!(err.code(), ErrorCode::InvalidField);
    let out = tmp_dir().join("docx-cs.docx");
    let err = apply(
        &docx,
        &out,
        json!([{ "op": "insertCustomShow", "id": "pptx:presentation", "name": "Tour" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());
}
