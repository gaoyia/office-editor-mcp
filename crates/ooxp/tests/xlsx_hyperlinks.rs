//! 工作表单元格超链接：名单、插改删、重叠拒绝、跨格式拒绝。

use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-xlsx-hyperlinks");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn tmp(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}.xlsx"))
}

fn create_book(tag: &str) -> PathBuf {
    let path = tmp(tag);
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "workbook": {
            "sheets": [{
                "name": "Sheet1",
                "cells": [{ "ref": "A1", "value": "链字" }]
            }]
        }
    })
    .to_string())
    .unwrap();
    path
}

fn apply(src: &Path, out: &Path, ops: Value) -> Result<RunOutput, ooxp::Error> {
    run(&json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": out.to_string_lossy(),
        "ops": ops,
    })
    .to_string())
}

fn inspect_bare(path: &Path) -> Value {
    let RunOutput::Inspect(tree) = run(&json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": path.to_string_lossy()
    })
    .to_string())
    .unwrap() else {
        panic!("inspect 应返回树");
    };
    tree
}

fn inspect_links(path: &Path) -> Value {
    let RunOutput::Inspect(tree) = run(&json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": path.to_string_lossy(),
        "scope": { "hyperlinks": true }
    })
    .to_string())
    .unwrap() else {
        panic!("inspect 应返回树");
    };
    tree
}

fn inspect_cell(path: &Path) -> Value {
    let RunOutput::Inspect(tree) = run(&json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": path.to_string_lossy(),
        "scope": { "sheet": "Sheet1", "range": "A1" }
    })
    .to_string())
    .unwrap() else {
        panic!("inspect 应返回树");
    };
    tree
}

fn object_keys(v: &Value) -> Vec<&str> {
    v.as_object().unwrap().keys().map(String::as_str).collect()
}

fn part_text(path: &Path, name: &str) -> String {
    let f = std::fs::File::open(path).unwrap();
    let mut zip = zip::ZipArchive::new(f).unwrap();
    let mut zf = zip.by_name(name).unwrap();
    let mut out = String::new();
    zf.read_to_string(&mut out).unwrap();
    out
}

fn part_names(path: &Path) -> Vec<String> {
    let f = std::fs::File::open(path).unwrap();
    let zip = zip::ZipArchive::new(f).unwrap();
    zip.file_names().map(str::to_string).collect()
}

#[test]
fn inspect_hyperlink_count_without_list() {
    let path = create_book("bare");
    let tree = inspect_bare(&path);
    assert_eq!(tree["hyperlinkCount"], 0);
    assert_eq!(tree["sheets"][0]["hyperlinkCount"], 0);
    assert!(tree["sheets"][0].get("hyperlinks").is_none());
    let listed = inspect_links(&path);
    assert_eq!(listed["sheets"][0]["hyperlinks"], json!([]));
}

#[test]
fn insert_url_inspect_set_delete() {
    let src = create_book("round");
    let inserted = tmp("ins");
    apply(
        &src,
        &inserted,
        json!([{
            "op": "insertHyperlink",
            "id": "sheet:Sheet1",
            "ref": "A1",
            "url": "https://example.com/",
            "tooltip": "提示"
        }]),
    )
    .unwrap();
    let sheet = part_text(&inserted, "xl/worksheets/sheet1.xml");
    assert!(sheet.contains("<hyperlinks>"));
    assert!(sheet.contains("r:id="));
    assert!(sheet.contains("tooltip=\"提示\""));
    let rels = part_text(&inserted, "xl/worksheets/_rels/sheet1.xml.rels");
    assert!(rels.contains("/hyperlink"));
    assert!(rels.contains("TargetMode=\"External\""));
    assert!(rels.contains("https://example.com/"));

    let tree = inspect_links(&inserted);
    assert_eq!(tree["hyperlinkCount"], 1);
    assert_eq!(tree["sheets"][0]["hyperlinkCount"], 1);
    let links = tree["sheets"][0]["hyperlinks"].as_array().unwrap();
    assert_eq!(links.len(), 1);
    assert_eq!(
        object_keys(&links[0]),
        ["id", "index", "kind", "ref", "rId", "url", "location", "tooltip", "display"]
    );
    assert_eq!(links[0]["id"], "sheet:Sheet1/link:1");
    assert_eq!(links[0]["index"], 1);
    assert_eq!(links[0]["kind"], "url");
    assert_eq!(links[0]["ref"], "A1");
    assert_eq!(links[0]["url"], "https://example.com/");
    assert!(links[0]["location"].is_null());
    assert_eq!(links[0]["tooltip"], "提示");
    assert!(links[0]["display"].is_null());
    let rid = links[0]["rId"].as_str().unwrap();
    assert!(rid.starts_with("rId"));

    let set_out = tmp("set");
    apply(
        &inserted,
        &set_out,
        json!([{
            "op": "setHyperlink",
            "id": "sheet:Sheet1/link:1",
            "url": "https://example.org/x",
            "location": "Sheet1!B1",
            "tooltip": null
        }]),
    )
    .unwrap();
    let tree_set = inspect_links(&set_out);
    let link = &tree_set["sheets"][0]["hyperlinks"][0];
    assert_eq!(link["url"], "https://example.org/x");
    assert_eq!(link["location"], "Sheet1!B1");
    assert!(link["tooltip"].is_null());
    assert_eq!(link["kind"], "url");

    let del = tmp("del");
    apply(
        &set_out,
        &del,
        json!([{ "op": "deleteHyperlink", "id": "sheet:Sheet1/link:1" }]),
    )
    .unwrap();
    let tree_del = inspect_links(&del);
    assert_eq!(tree_del["hyperlinkCount"], 0);
    assert_eq!(tree_del["sheets"][0]["hyperlinks"], json!([]));
    let sheet_del = part_text(&del, "xl/worksheets/sheet1.xml");
    assert!(!sheet_del.contains("<hyperlinks"));
    let cell = inspect_cell(&del);
    assert_eq!(cell["sheets"][0]["cells"][0]["value"], "链字");
    let names = part_names(&del);
    if names
        .iter()
        .any(|n| n == "xl/worksheets/_rels/sheet1.xml.rels")
    {
        let rels_del = part_text(&del, "xl/worksheets/_rels/sheet1.xml.rels");
        assert!(!rels_del.contains("/hyperlink"));
    }
}

#[test]
fn insert_location_has_no_rel() {
    let src = create_book("loc");
    let out = tmp("loc-out");
    apply(
        &src,
        &out,
        json!([{
            "op": "insertHyperlink",
            "id": "sheet:Sheet1",
            "ref": "A1",
            "location": "Sheet1!B1"
        }]),
    )
    .unwrap();
    let tree = inspect_links(&out);
    let link = &tree["sheets"][0]["hyperlinks"][0];
    assert_eq!(link["kind"], "location");
    assert!(link["url"].is_null());
    assert!(link["rId"].is_null());
    assert_eq!(link["location"], "Sheet1!B1");
    let names = part_names(&out);
    if names
        .iter()
        .any(|n| n == "xl/worksheets/_rels/sheet1.xml.rels")
    {
        let rels = part_text(&out, "xl/worksheets/_rels/sheet1.xml.rels");
        assert!(!rels.contains("/hyperlink"));
    }
}

#[test]
fn missing_ref_or_target_is_invalid_field() {
    let src = create_book("miss");
    let out = tmp("miss-out");
    let err = apply(
        &src,
        &out,
        json!([{ "op": "insertHyperlink", "id": "sheet:Sheet1" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());
    let err2 = apply(
        &src,
        &out,
        json!([{ "op": "insertHyperlink", "id": "sheet:Sheet1", "ref": "A1" }]),
    )
    .unwrap_err();
    assert_eq!(err2.code(), ErrorCode::InvalidField);
    assert!(!out.exists());
}

#[test]
fn overlap_is_invalid_field() {
    let src = create_book("ov");
    let one = tmp("ov-one");
    apply(
        &src,
        &one,
        json!([{
            "op": "insertHyperlink",
            "id": "sheet:Sheet1",
            "ref": "A1",
            "url": "https://example.com/"
        }]),
    )
    .unwrap();
    let two = tmp("ov-two");
    let err = apply(
        &one,
        &two,
        json!([{
            "op": "insertHyperlink",
            "id": "sheet:Sheet1",
            "ref": "A1:B2",
            "url": "https://example.org/"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!two.exists());
}

#[test]
fn text_anchor_and_para_id_rejected() {
    let src = create_book("deny");
    let out = tmp("deny-out");
    let err = apply(
        &src,
        &out,
        json!([{
            "op": "insertHyperlink",
            "id": "para:1",
            "text": "x",
            "url": "https://example.com/"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    let err2 = apply(
        &src,
        &out,
        json!([{
            "op": "insertHyperlink",
            "id": "sheet:Sheet1",
            "ref": "A1",
            "anchor": "foo"
        }]),
    )
    .unwrap_err();
    assert_eq!(err2.code(), ErrorCode::InvalidField);
}

#[test]
fn create_rejects_hyperlink_without_target() {
    let path = tmp("create");
    let err = run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "workbook": {
            "sheets": [{
                "name": "Sheet1",
                "hyperlinks": [{ "ref": "A1" }]
            }]
        }
    })
    .to_string())
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert_eq!(err.field_path(), Some("workbook.sheets[0].hyperlinks[0]"));
}
