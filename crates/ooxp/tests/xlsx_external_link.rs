//! 工作簿外部引用：计数、名单、插删；不打开目标文件。

use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::json;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-xlsx-extlink");
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
    assert_eq!(tree["externalLinkCount"], 0);
    assert!(tree.get("externalLinks").is_none());
    let keys = object_keys(&tree);
    assert!(keys
        .windows(2)
        .any(|w| w == ["connectionCount", "externalLinkCount"]));
    assert!(keys
        .windows(2)
        .any(|w| w == ["externalLinkCount", "customViewCount"]));
    let listed = inspect(&path, json!({ "externalLinks": true }));
    assert_eq!(listed["externalLinks"], json!([]));
}

#[test]
fn insert_lists_and_writes_part() {
    let src = create_book("ins");
    let out = tmp("ins-out");
    apply(
        &src,
        &out,
        json!([{
            "op": "insertExternalLink",
            "id": "xlsx:workbook",
            "path": r"C:\data\source.xlsx",
            "sheets": ["Sheet1"]
        }]),
    )
    .unwrap();
    let tree = inspect(&out, json!({ "externalLinks": true }));
    assert_eq!(tree["externalLinkCount"], 1);
    let item = &tree["externalLinks"][0];
    assert_eq!(item["id"], "extlink:1");
    assert_eq!(item["index"], 1);
    assert_eq!(item["kind"], "book");
    assert_eq!(item["path"], r"file:///C:\data\source.xlsx");
    assert_eq!(item["sheets"], json!(["Sheet1"]));
    assert_eq!(object_keys(item), ["id", "index", "kind", "path", "sheets"]);
    let xml = part_text(&out, "xl/externalLinks/externalLink1.xml");
    assert!(xml.contains("externalBook"), "{xml}");
    assert!(xml.contains("sheetId=\"0\""), "{xml}");
    assert!(!xml.contains("xxl21"), "{xml}");
    assert!(!xml.contains("<v>"), "{xml}");
    let rels = part_text(&out, "xl/externalLinks/_rels/externalLink1.xml.rels");
    assert!(rels.contains("externalLinkPath"), "{rels}");
    assert!(rels.contains(r#"TargetMode="External""#), "{rels}");
    assert!(rels.contains(r"file:///C:\data\source.xlsx"), "{rels}");
    let wb = part_text(&out, "xl/workbook.xml");
    assert!(wb.contains("externalReferences"), "{wb}");
    let wb_rels = part_text(&out, "xl/_rels/workbook.xml.rels");
    assert!(wb_rels.contains("/relationships/externalLink"), "{wb_rels}");
}

#[test]
fn insert_relative_path_kept() {
    let src = create_book("rel");
    let out = tmp("rel-out");
    apply(
        &src,
        &out,
        json!([{
            "op": "insertExternalLink",
            "id": "xlsx:workbook",
            "path": "source.xlsx"
        }]),
    )
    .unwrap();
    let tree = inspect(&out, json!({ "externalLinks": true }));
    assert_eq!(tree["externalLinks"][0]["path"], "source.xlsx");
    assert_eq!(tree["externalLinks"][0]["sheets"], json!(["Sheet1"]));
}

#[test]
fn delete_empty_ok_and_in_use_refuses() {
    let src = create_book("del");
    let linked = tmp("linked");
    apply(
        &src,
        &linked,
        json!([{
            "op": "insertExternalLink",
            "id": "xlsx:workbook",
            "path": "source.xlsx"
        }]),
    )
    .unwrap();

    let with_f = tmp("with-f");
    apply(
        &linked,
        &with_f,
        json!([{ "op": "setCell", "id": "cell:Sheet1!B1", "formula": "=[1]Sheet1!A1" }]),
    )
    .unwrap();
    let blocked = tmp("blocked");
    let err = apply(
        &with_f,
        &blocked,
        json!([{ "op": "deleteExternalLink", "id": "extlink:1" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::NotImplemented);
    assert!(!blocked.exists());

    let cleared = tmp("cleared");
    apply(
        &with_f,
        &cleared,
        json!([{ "op": "setCell", "id": "cell:Sheet1!B1", "value": null }]),
    )
    .unwrap();
    let out = tmp("del-out");
    apply(
        &cleared,
        &out,
        json!([{ "op": "deleteExternalLink", "id": "extlink:1" }]),
    )
    .unwrap();
    let after = inspect(&out, json!({ "externalLinks": true }));
    assert_eq!(after["externalLinkCount"], 0);
    assert_eq!(after["externalLinks"], json!([]));
    assert!(!part_exists(&out, "xl/externalLinks/externalLink1.xml"));
    let wb = part_text(&out, "xl/workbook.xml");
    assert!(!wb.contains("externalReferences"), "{wb}");
}

#[test]
fn bracket_ten_is_not_one() {
    let src = create_book("ten");
    let two = tmp("two");
    apply(
        &src,
        &two,
        json!([
            { "op": "insertExternalLink", "id": "xlsx:workbook", "path": "a.xlsx" },
            { "op": "insertExternalLink", "id": "xlsx:workbook", "path": "b.xlsx" }
        ]),
    )
    .unwrap();
    let with_f = tmp("ten-f");
    apply(
        &two,
        &with_f,
        json!([{ "op": "setCell", "id": "cell:Sheet1!B1", "formula": "=[10]Sheet1!A1" }]),
    )
    .unwrap();
    let blocked = tmp("ten-block");
    let err = apply(
        &with_f,
        &blocked,
        json!([{ "op": "deleteExternalLink", "id": "extlink:2" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::NotImplemented);
    assert!(!blocked.exists());
}

#[test]
fn wrong_ids_and_missing_path() {
    let src = create_book("bad");
    let out = tmp("bad-out");
    let err = apply(
        &src,
        &out,
        json!([{
            "op": "insertExternalLink",
            "id": "xlsx:workbook"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());

    let err = apply(
        &src,
        &out,
        json!([{
            "op": "insertExternalLink",
            "id": "slide:1",
            "path": "source.xlsx"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    assert!(!out.exists());

    let err = apply(
        &src,
        &out,
        json!([{ "op": "deleteExternalLink", "id": "extlink:1" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::IdNotFound);
    assert!(!out.exists());
}

#[test]
fn create_rejects_external_links() {
    let path = tmp("create-el");
    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "workbook": {
                "externalLinks": [],
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

#[test]
fn set_external_link_changes_target() {
    let src = create_book("setp");
    let linked = tmp("setp-in");
    apply(
        &src,
        &linked,
        json!([{
            "op": "insertExternalLink",
            "id": "xlsx:workbook",
            "path": r"C:\data\source.xlsx",
            "sheets": ["Sheet1"]
        }]),
    )
    .unwrap();
    let with_f = tmp("setp-f");
    apply(
        &linked,
        &with_f,
        json!([{ "op": "setCell", "id": "cell:Sheet1!B1", "formula": "=[1]Sheet1!A1" }]),
    )
    .unwrap();
    let out = tmp("setp-out");
    apply(
        &with_f,
        &out,
        json!([{
            "op": "setExternalLink",
            "id": "extlink:1",
            "path": r"C:\data\other.xlsx"
        }]),
    )
    .unwrap();
    let tree = inspect(&out, json!({ "externalLinks": true }));
    assert_eq!(
        tree["externalLinks"][0]["path"],
        r"file:///C:\data\other.xlsx"
    );
    let rels = part_text(&out, "xl/externalLinks/_rels/externalLink1.xml.rels");
    assert!(rels.contains("other.xlsx"), "{rels}");
    assert!(!rels.contains("source.xlsx"), "{rels}");
    let xml = part_text(&out, "xl/externalLinks/externalLink1.xml");
    assert!(!xml.contains("xxl21"), "{xml}");
    let cells = inspect(&out, json!({ "sheet": "Sheet1", "range": "A1:B1" }));
    let b1 = &cells["sheets"][0]["cells"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "cell:Sheet1!B1")
        .unwrap();
    assert_eq!(b1["formula"], json!("=[1]Sheet1!A1"));
}

#[test]
fn set_external_link_rejects_bad_id_and_path() {
    let src = create_book("setp-bad");
    let linked = tmp("setp-bad-in");
    apply(
        &src,
        &linked,
        json!([{
            "op": "insertExternalLink",
            "id": "xlsx:workbook",
            "path": "a.xlsx"
        }]),
    )
    .unwrap();
    let out = tmp("setp-bad-out");
    let err = apply(
        &linked,
        &out,
        json!([{ "op": "setExternalLink", "id": "extlink:1" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());

    let err = apply(
        &linked,
        &out,
        json!([{
            "op": "setExternalLink",
            "id": "slide:1",
            "path": "b.xlsx"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    assert!(!out.exists());

    let err = apply(
        &src,
        &out,
        json!([{
            "op": "setExternalLink",
            "id": "extlink:1",
            "path": "b.xlsx"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::IdNotFound);
    assert!(!out.exists());
}
