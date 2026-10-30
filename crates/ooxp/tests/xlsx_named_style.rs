//! 命名单元格样式：名单、增删、套到格子上。不改主题。

use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::json;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-xlsx-named-style");
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
                "cells": [
                    { "ref": "A1", "value": 1.2 },
                    { "ref": "B1", "value": 3 }
                ]
            }]
        }
    })
    .to_string())
    .unwrap();
    path
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

fn inspect(path: &Path, scope: serde_json::Value) -> serde_json::Value {
    let RunOutput::Inspect(tree) = run(&json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": path.to_string_lossy(),
        "scope": scope
    })
    .to_string())
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

fn err_code(e: ooxp::Error) -> ErrorCode {
    e.code()
}

#[test]
fn empty_create_has_count_without_list() {
    let path = create_book("empty");
    let tree = inspect(&path, json!({}));
    assert_eq!(tree["cellStyleCount"], 1);
    assert!(tree.get("cellStyles").is_none());
    let keys = object_keys(&tree);
    assert!(keys
        .windows(2)
        .any(|w| w == ["definedNameCount", "cellStyleCount"]));
    assert!(keys.windows(2).any(|w| w == ["cellStyleCount", "props"]));
    let listed = inspect(&path, json!({ "cellStyles": true }));
    assert_eq!(listed["cellStyleCount"], 1);
    let item = &listed["cellStyles"][0];
    assert_eq!(item["id"], "cellStyle:1");
    assert_eq!(item["index"], 1);
    assert_eq!(item["name"], "Normal");
    assert_eq!(item["xfId"], 0);
    assert_eq!(item["builtinId"], 0);
    assert_eq!(
        object_keys(item),
        ["id", "index", "name", "xfId", "builtinId"]
    );
}

#[test]
fn insert_lists_and_writes_xml() {
    let src = create_book("ins");
    let out = tmp("ins-out");
    apply(
        &src,
        &out,
        json!([{
            "op": "insertCellStyle",
            "id": "xlsx:workbook",
            "name": "OoxpNamed",
            "font": { "bold": true },
            "fill": { "pattern": "solid", "fg": { "rgb": "FFFFFF00" } }
        }]),
    )
    .unwrap();
    let tree = inspect(&out, json!({ "cellStyles": true }));
    assert_eq!(tree["cellStyleCount"], 2);
    let item = &tree["cellStyles"][1];
    assert_eq!(item["id"], "cellStyle:2");
    assert_eq!(item["name"], "OoxpNamed");
    assert_eq!(item["builtinId"], serde_json::Value::Null);
    assert_eq!(item["xfId"], 1);
    let styles = part_text(&out, "xl/styles.xml");
    assert!(styles.contains(r#"name="OoxpNamed""#), "{styles}");
    assert!(styles.contains(r#"xfId="1""#), "{styles}");
    assert!(
        !styles.contains("applyFont")
            && !styles.contains("applyFill")
            && !styles.contains("applyBorder")
            && !styles.contains("applyNumberFormat")
            && !styles.contains("applyAlignment"),
        "{styles}"
    );
    assert!(
        !styles.contains("builtinId") || styles.matches("builtinId").count() == 1,
        "{styles}"
    );
}

#[test]
fn set_named_points_cell_s() {
    let src = create_book("set");
    let mid = tmp("set-mid");
    apply(
        &src,
        &mid,
        json!([{
            "op": "insertCellStyle",
            "id": "xlsx:workbook",
            "name": "OoxpNamed",
            "font": { "bold": true },
            "fill": { "pattern": "solid", "fg": { "rgb": "FFFFFF00" } }
        }]),
    )
    .unwrap();
    let out = tmp("set-out");
    apply(
        &mid,
        &out,
        json!([{
            "op": "setCellNamedStyle",
            "id": "cell:Sheet1!A1",
            "style": "cellStyle:2"
        }]),
    )
    .unwrap();
    let tree = inspect(
        &out,
        json!({ "sheet": "Sheet1", "range": "A1:B1", "cellStyles": true }),
    );
    let a1 = tree["sheets"][0]["cells"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "cell:Sheet1!A1")
        .unwrap();
    assert_eq!(a1["fill"]["pattern"], "solid");
    assert_eq!(a1["font"]["bold"], true);
    let sheet = part_text(&out, "xl/worksheets/sheet1.xml");
    assert!(
        sheet.contains(r#"s="1""#) || sheet.contains("s=\"1\""),
        "{sheet}"
    );
}

#[test]
fn delete_custom_keeps_cell_s() {
    let src = create_book("del");
    let mid = tmp("del-mid");
    apply(
        &src,
        &mid,
        json!([
            {
                "op": "insertCellStyle",
                "id": "xlsx:workbook",
                "name": "OoxpNamed",
                "fill": { "pattern": "solid", "fg": { "rgb": "FFFFFF00" } }
            },
            {
                "op": "setCellNamedStyle",
                "id": "cell:Sheet1!A1",
                "style": "cellStyle:2"
            }
        ]),
    )
    .unwrap();
    let out = tmp("del-out");
    apply(
        &mid,
        &out,
        json!([{ "op": "deleteCellStyle", "id": "cellStyle:2" }]),
    )
    .unwrap();
    let tree = inspect(
        &out,
        json!({ "sheet": "Sheet1", "range": "A1", "cellStyles": true }),
    );
    assert_eq!(tree["cellStyleCount"], 1);
    assert_eq!(tree["cellStyles"][0]["name"], "Normal");
    let a1 = tree["sheets"][0]["cells"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "cell:Sheet1!A1")
        .unwrap();
    assert_eq!(a1["fill"]["pattern"], "solid");
}

#[test]
fn reject_builtin_delete_and_missing_fields() {
    let src = create_book("rej");
    let out = tmp("rej-out");
    let e = apply(
        &src,
        &out,
        json!([{ "op": "deleteCellStyle", "id": "cellStyle:1" }]),
    )
    .unwrap_err();
    assert_eq!(err_code(e), ErrorCode::InvalidField);
    assert!(!out.exists());

    let e = apply(
        &src,
        &out,
        json!([{ "op": "insertCellStyle", "id": "xlsx:workbook", "name": "X" }]),
    )
    .unwrap_err();
    assert_eq!(err_code(e), ErrorCode::InvalidField);

    let e = apply(
        &src,
        &out,
        json!([{ "op": "setCellNamedStyle", "id": "cell:Sheet1!A1" }]),
    )
    .unwrap_err();
    assert_eq!(err_code(e), ErrorCode::InvalidField);
}

#[test]
fn reject_duplicate_name_and_bad_id() {
    let src = create_book("dup");
    let out = tmp("dup-out");
    let e = apply(
        &src,
        &out,
        json!([{
            "op": "insertCellStyle",
            "id": "xlsx:workbook",
            "name": "Normal",
            "font": { "bold": true }
        }]),
    )
    .unwrap_err();
    assert_eq!(err_code(e), ErrorCode::InvalidField);

    let e = apply(
        &src,
        &out,
        json!([{
            "op": "insertCellStyle",
            "id": "sheet:Sheet1",
            "name": "OoxpNamed",
            "font": { "bold": true }
        }]),
    )
    .unwrap_err();
    assert_eq!(err_code(e), ErrorCode::InvalidId);
}

#[test]
fn missing_style_is_id_not_found() {
    let src = create_book("miss");
    let out = tmp("miss-out");
    let e = apply(
        &src,
        &out,
        json!([{ "op": "deleteCellStyle", "id": "cellStyle:9" }]),
    )
    .unwrap_err();
    assert_eq!(err_code(e), ErrorCode::IdNotFound);

    let e = apply(
        &src,
        &out,
        json!([{
            "op": "setCellNamedStyle",
            "id": "cell:Sheet1!A1",
            "style": "cellStyle:9"
        }]),
    )
    .unwrap_err();
    assert_eq!(err_code(e), ErrorCode::IdNotFound);
}

#[test]
fn pptx_rejects_scope_and_op() {
    let pptx = tmp_dir().join("pptx-ns.pptx");
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": pptx.to_string_lossy(),
        "presentation": { "slides": [{ "shapes": [{ "text": "a" }] }] }
    })
    .to_string())
    .unwrap();
    let err = run(&json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": pptx.to_string_lossy(),
        "scope": { "cellStyles": true }
    })
    .to_string())
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let out = tmp_dir().join("pptx-ns-out.pptx");
    let err = run(&json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": pptx.to_string_lossy(),
        "output": out.to_string_lossy(),
        "ops": [{
            "op": "insertCellStyle",
            "id": "xlsx:workbook",
            "name": "OoxpNamed",
            "font": { "bold": true }
        }]
    })
    .to_string())
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());
}
