//! 工作簿外部数据连接：计数、名单、插删；不刷新。

use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::json;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-xlsx-connections");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn tmp(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}.xlsx"))
}

fn create_book(tag: &str) -> PathBuf {
    let path = tmp(tag);
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "workbook": {
                "sheets": [{ "name": "Sheet1", "cells": [{ "ref": "A1", "value": 1 }] }]
            }
        })
        .to_string(),
        None,
    )
    .unwrap();
    path
}

fn apply(src: &Path, out: &Path, ops: serde_json::Value) -> Result<RunOutput, ooxp::Error> {
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

fn inspect(path: &Path, scope: serde_json::Value) -> serde_json::Value {
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

fn object_keys(v: &serde_json::Value) -> Vec<&str> {
    v.as_object().unwrap().keys().map(|k| k.as_str()).collect()
}

fn part_text(path: &Path, name: &str) -> String {
    let f = std::fs::File::open(path).unwrap();
    let mut zip = zip::ZipArchive::new(f).unwrap();
    let mut zf = zip.by_name(name).unwrap();
    let mut s = String::new();
    zf.read_to_string(&mut s).unwrap();
    s
}

fn part_exists(path: &Path, name: &str) -> bool {
    let f = std::fs::File::open(path).unwrap();
    let mut zip = zip::ZipArchive::new(f).unwrap();
    let ok = zip.by_name(name).is_ok();
    ok
}

#[test]
fn empty_create_has_count_without_list() {
    let path = create_book("empty");
    let tree = inspect(&path, json!({}));
    assert_eq!(tree["connectionCount"], 0);
    assert!(tree.get("connections").is_none());
    let keys = object_keys(&tree);
    assert!(keys
        .windows(2)
        .any(|w| w == ["slicerCount", "connectionCount"]));
    assert!(keys
        .windows(2)
        .any(|w| w == ["connectionCount", "externalLinkCount"]));
    assert!(keys
        .windows(2)
        .any(|w| w == ["externalLinkCount", "customViewCount"]));
    assert!(keys
        .windows(2)
        .any(|w| w == ["customViewCount", "definedNameCount"]));
    let listed = inspect(&path, json!({ "connections": true }));
    assert_eq!(listed["connections"], json!([]));
}

#[test]
fn insert_web_lists_and_writes_part() {
    let src = create_book("web");
    let out = tmp("web-out");
    apply(
        &src,
        &out,
        json!([{
            "op": "insertConnection",
            "id": "xlsx:workbook",
            "name": "WebDemo",
            "type": "web",
            "url": "http://example.com",
            "description": "demo"
        }]),
    )
    .unwrap();
    let tree = inspect(&out, json!({ "connections": true }));
    assert_eq!(tree["connectionCount"], 1);
    let item = &tree["connections"][0];
    assert_eq!(item["id"], "connection:1");
    assert_eq!(item["xmlId"], "1");
    assert_eq!(item["name"], "WebDemo");
    assert_eq!(item["kind"], "web");
    assert_eq!(item["url"], "http://example.com");
    assert_eq!(item["sourceFile"], serde_json::Value::Null);
    assert_eq!(item["description"], "demo");
    assert_eq!(item["stable"], true);
    assert_eq!(
        object_keys(item),
        [
            "id",
            "xmlId",
            "name",
            "kind",
            "url",
            "sourceFile",
            "description",
            "stable"
        ]
    );
    let xml = part_text(&out, "xl/connections.xml");
    assert!(xml.contains("type=\"4\""), "{xml}");
    assert!(xml.contains("htmlFormat=\"all\""), "{xml}");
    assert!(!xml.contains("xr16:uid"), "{xml}");
    assert!(!xml.contains("refreshOnLoad"), "{xml}");
    assert!(!part_exists(&out, "xl/queryTables/queryTable1.xml"));
    let rels = part_text(&out, "xl/_rels/workbook.xml.rels");
    assert!(rels.contains("/relationships/connections"), "{rels}");
    assert!(rels.contains("Target=\"connections.xml\""), "{rels}");
}

#[test]
fn insert_text_writes_textpr() {
    let src = create_book("text");
    let out = tmp("text-out");
    apply(
        &src,
        &out,
        json!([{
            "op": "insertConnection",
            "id": "xlsx:workbook",
            "name": "CsvSrc",
            "type": "text",
            "sourceFile": r"C:\data\src.csv"
        }]),
    )
    .unwrap();
    let tree = inspect(&out, json!({ "connections": true }));
    let item = &tree["connections"][0];
    assert_eq!(item["kind"], "text");
    assert_eq!(item["sourceFile"], r"C:\data\src.csv");
    assert_eq!(item["url"], serde_json::Value::Null);
    let xml = part_text(&out, "xl/connections.xml");
    assert!(xml.contains("type=\"6\""), "{xml}");
    assert!(xml.contains("codePage=\"65001\""), "{xml}");
    assert!(
        xml.contains("<textField/>") || xml.contains("<textField />"),
        "{xml}"
    );
}

#[test]
fn delete_connection_drops_part() {
    let src = create_book("del-src");
    let inserted = tmp("del-ins");
    apply(
        &src,
        &inserted,
        json!([{
            "op": "insertConnection",
            "id": "xlsx:workbook",
            "name": "WebDemo",
            "type": "web",
            "url": "http://example.com"
        }]),
    )
    .unwrap();
    let tree = inspect(&inserted, json!({ "connections": true }));
    let id = tree["connections"][0]["id"].as_str().unwrap();
    let out = tmp("del-out");
    apply(
        &inserted,
        &out,
        json!([{ "op": "deleteConnection", "id": id }]),
    )
    .unwrap();
    let after = inspect(&out, json!({ "connections": true }));
    assert_eq!(after["connectionCount"], 0);
    assert_eq!(after["connections"], json!([]));
    assert!(!part_exists(&out, "xl/connections.xml"));
    let rels = part_text(&out, "xl/_rels/workbook.xml.rels");
    assert!(!rels.contains("/relationships/connections"), "{rels}");
}

#[test]
fn wrong_ids_and_missing_type() {
    let src = create_book("bad");
    let out = tmp("bad-out");
    let err = apply(
        &src,
        &out,
        json!([{
            "op": "insertConnection",
            "id": "xlsx:workbook",
            "name": "WebDemo"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());

    let err = apply(
        &src,
        &out,
        json!([{
            "op": "insertConnection",
            "id": "slide:1",
            "name": "WebDemo",
            "type": "web",
            "url": "http://example.com"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    assert!(!out.exists());

    let err = apply(
        &src,
        &out,
        json!([{
            "op": "insertConnection",
            "id": "xlsx:workbook",
            "name": "WebDemo",
            "type": "odbc",
            "url": "http://example.com"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());
}

#[test]
fn create_rejects_connections() {
    let path = tmp("create-conn");
    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "workbook": {
                "connections": [],
                "sheets": [{ "name": "Sheet1" }]
            }
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!path.exists());
}
