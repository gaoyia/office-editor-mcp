//! 定义名：inspect 名单、insert / set / delete。

use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::json;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-xlsx-defined-name");
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
                "sheets": [
                    {
                        "name": "Sheet1",
                        "cells": [
                            { "ref": "A1", "value": 1 },
                            { "ref": "A10", "value": 10 },
                            { "ref": "A20", "value": 20 }
                        ]
                    },
                    {
                        "name": "Sheet2",
                        "cells": [{ "ref": "A1", "value": 2 }]
                    }
                ]
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

fn inspect_tree(path: &Path) -> serde_json::Value {
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

fn part_text(path: &Path, name: &str) -> String {
    let f = std::fs::File::open(path).unwrap();
    let mut zip = zip::ZipArchive::new(f).unwrap();
    let mut zf = zip.by_name(name).unwrap();
    let mut s = String::new();
    zf.read_to_string(&mut s).unwrap();
    s
}

fn object_keys(v: &serde_json::Value) -> Vec<&str> {
    v.as_object().unwrap().keys().map(|k| k.as_str()).collect()
}

#[test]
fn inspect_empty_on_create() {
    let path = create_book("empty");
    let tree = inspect_tree(&path);
    assert_eq!(tree["definedNameCount"], json!(0));
    assert_eq!(tree["definedNames"], json!([]));
    assert_eq!(
        object_keys(&tree),
        [
            "schemaVersion",
            "format",
            "id",
            "pictureCount",
            "chartCount",
            "textBoxCount",
            "mathCount",
            "shapeCount",
            "connectorCount",
            "smartArtCount",
            "commentCount",
            "hyperlinkCount",
            "sparklineCount",
            "oleObjectCount",
            "model3dCount",
            "slicerCount",
            "connectionCount",
            "externalLinkCount",
            "customViewCount",
            "definedNameCount",
            "cellStyleCount",
            "props",
            "workbookProtection",
            "theme",
            "themeOverride",
            "sheets",
            "definedNames"
        ]
    );
}

#[test]
fn insert_inspect_set_delete() {
    let src = create_book("round");
    let inserted = tmp("ins");
    apply(
        &src,
        &inserted,
        json!([{
            "op": "insertDefinedName",
            "id": "xlsx:workbook",
            "name": "Sales",
            "formula": "Sheet1!$A$1:$A$10"
        }]),
    )
    .unwrap();
    let tree = inspect_tree(&inserted);
    assert_eq!(tree["definedNameCount"], json!(1));
    let n = &tree["definedNames"][0];
    assert_eq!(n["id"], json!("name:1"));
    assert_eq!(n["name"], json!("Sales"));
    assert_eq!(n["formula"], json!("Sheet1!$A$1:$A$10"));
    assert_eq!(n["hidden"], json!(false));
    assert_eq!(n["localSheet"], json!(null));
    assert_eq!(
        object_keys(n),
        ["id", "name", "formula", "hidden", "localSheet"]
    );
    let wb = part_text(&inserted, "xl/workbook.xml");
    assert!(wb.contains("<definedNames"), "{wb}");
    assert!(wb.contains(r#"name="Sales""#), "{wb}");
    assert!(wb.contains("Sheet1!$A$1:$A$10"), "{wb}");
    assert!(
        wb.find("<definedNames").unwrap() < wb.find("<calcPr").unwrap_or(wb.len()),
        "{wb}"
    );

    let updated = tmp("set");
    apply(
        &inserted,
        &updated,
        json!([{
            "op": "setDefinedName",
            "id": "name:1",
            "formula": "Sheet1!$A$1:$A$20",
            "hidden": true
        }]),
    )
    .unwrap();
    let tree = inspect_tree(&updated);
    assert_eq!(
        tree["definedNames"][0]["formula"],
        json!("Sheet1!$A$1:$A$20")
    );
    assert_eq!(tree["definedNames"][0]["hidden"], json!(true));
    let wb = part_text(&updated, "xl/workbook.xml");
    assert!(wb.contains(r#"hidden="1""#), "{wb}");

    let gone = tmp("del");
    apply(
        &updated,
        &gone,
        json!([{ "op": "deleteDefinedName", "id": "name:1" }]),
    )
    .unwrap();
    let tree = inspect_tree(&gone);
    assert_eq!(tree["definedNameCount"], json!(0));
    assert_eq!(tree["definedNames"], json!([]));
    let wb = part_text(&gone, "xl/workbook.xml");
    assert!(!wb.contains("definedNames"), "{wb}");
    assert!(!wb.contains("Sales"), "{wb}");
}

#[test]
fn local_sheet_and_print_area() {
    let src = create_book("local");
    let out = tmp("local-out");
    apply(
        &src,
        &out,
        json!([
            {
                "op": "insertDefinedName",
                "id": "xlsx:workbook",
                "name": "Sales",
                "formula": "Sheet1!$A$1:$A$10",
                "localSheet": "Sheet1"
            },
            {
                "op": "insertDefinedName",
                "id": "xlsx:workbook",
                "name": "Sales",
                "formula": "Sheet2!$A$1",
                "localSheet": "Sheet2"
            },
            {
                "op": "insertDefinedName",
                "id": "xlsx:workbook",
                "name": "_xlnm.Print_Area",
                "formula": "Sheet1!$A$1:$A$10",
                "localSheet": "Sheet1"
            }
        ]),
    )
    .unwrap();
    let tree = inspect_tree(&out);
    assert_eq!(tree["definedNameCount"], json!(3));
    assert_eq!(tree["definedNames"][0]["localSheet"], json!("Sheet1"));
    assert_eq!(tree["definedNames"][1]["localSheet"], json!("Sheet2"));
    assert_eq!(tree["definedNames"][2]["name"], json!("_xlnm.Print_Area"));
    let wb = part_text(&out, "xl/workbook.xml");
    assert!(wb.contains(r#"localSheetId="0""#), "{wb}");
    assert!(wb.contains(r#"localSheetId="1""#), "{wb}");

    let book = tmp("book-level");
    apply(
        &out,
        &book,
        json!([{ "op": "setDefinedName", "id": "name:1", "localSheet": null }]),
    )
    .unwrap();
    let tree = inspect_tree(&book);
    assert_eq!(tree["definedNames"][0]["localSheet"], json!(null));
}

#[test]
fn missing_fields_and_bad_names() {
    let src = create_book("miss");
    let out = tmp("miss-out");
    let err = apply(
        &src,
        &out,
        json!([{ "op": "insertDefinedName", "id": "xlsx:workbook" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());

    for name in ["A1", "R", "C", "LOG10", "R1C1", ""] {
        let err = apply(
            &src,
            &tmp("bad-name"),
            json!([{
                "op": "insertDefinedName",
                "id": "xlsx:workbook",
                "name": name,
                "formula": "Sheet1!$A$1"
            }]),
        )
        .unwrap_err();
        assert_eq!(err.code(), ErrorCode::InvalidField, "name={name}");
    }

    let err = apply(
        &src,
        &tmp("eq"),
        json!([{
            "op": "insertDefinedName",
            "id": "xlsx:workbook",
            "name": "Sales",
            "formula": "=Sheet1!$A$1"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let err = apply(
        &src,
        &tmp("bad-id"),
        json!([{
            "op": "insertDefinedName",
            "id": "sheet:Sheet1",
            "name": "Sales",
            "formula": "Sheet1!$A$1"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
}

#[test]
fn duplicate_and_table_conflict() {
    let src = create_book("dup");
    let one = tmp("dup-one");
    apply(
        &src,
        &one,
        json!([{
            "op": "insertDefinedName",
            "id": "xlsx:workbook",
            "name": "Sales",
            "formula": "Sheet1!$A$1:$A$10"
        }]),
    )
    .unwrap();
    let err = apply(
        &one,
        &tmp("dup-two"),
        json!([{
            "op": "insertDefinedName",
            "id": "xlsx:workbook",
            "name": "sales",
            "formula": "Sheet1!$A$1"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let with_table = tmp("tbl");
    apply(
        &src,
        &with_table,
        json!([{
            "op": "insertSheetTable",
            "id": "sheet:Sheet1",
            "ref": "A1:A10",
            "name": "TblSales",
            "columns": ["列1"]
        }]),
    )
    .unwrap();
    let err = apply(
        &with_table,
        &tmp("tbl-name"),
        json!([{
            "op": "insertDefinedName",
            "id": "xlsx:workbook",
            "name": "TblSales",
            "formula": "Sheet1!$A$1"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

#[test]
fn set_without_fields_and_missing_id() {
    let src = create_book("setmiss");
    let mid = tmp("setmiss-mid");
    apply(
        &src,
        &mid,
        json!([{
            "op": "insertDefinedName",
            "id": "xlsx:workbook",
            "name": "Sales",
            "formula": "Sheet1!$A$1"
        }]),
    )
    .unwrap();
    let err = apply(
        &mid,
        &tmp("setmiss-out"),
        json!([{ "op": "setDefinedName", "id": "name:1" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let err = apply(
        &mid,
        &tmp("gone"),
        json!([{ "op": "deleteDefinedName", "id": "name:9" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::IdNotFound);

    let err = apply(
        &mid,
        &tmp("sheet-id"),
        json!([{ "op": "setDefinedName", "id": "sheet:Sheet1", "formula": "Sheet1!$A$1" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
}

#[test]
fn unicode_name_allowed() {
    let src = create_book("zh");
    let out = tmp("zh-out");
    apply(
        &src,
        &out,
        json!([{
            "op": "insertDefinedName",
            "id": "xlsx:workbook",
            "name": "销售",
            "formula": "Sheet1!$A$1:$A$10"
        }]),
    )
    .unwrap();
    let tree = inspect_tree(&out);
    assert_eq!(tree["definedNames"][0]["name"], json!("销售"));
}
