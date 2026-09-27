//! 打印设置：inspect 摘要、setPrintSetup。

use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::json;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-xlsx-print");
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
                "sheets": [{
                    "name": "Sheet1",
                    "cells": [
                        { "ref": "A1", "value": 1 },
                        { "ref": "A10", "value": 10 }
                    ]
                }]
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
fn inspect_defaults_on_create() {
    let path = create_book("empty");
    let tree = inspect_tree(&path);
    let p = &tree["sheets"][0]["print"];
    assert_eq!(p["left"], json!(0.7));
    assert_eq!(p["right"], json!(0.7));
    assert_eq!(p["top"], json!(0.75));
    assert_eq!(p["bottom"], json!(0.75));
    assert_eq!(p["header"], json!(0.3));
    assert_eq!(p["footer"], json!(0.3));
    assert_eq!(p["paperSize"], json!(1));
    assert_eq!(p["orientation"], json!("portrait"));
    assert_eq!(p["scale"], json!(100));
    assert_eq!(p["fitToPage"], json!(false));
    assert_eq!(p["fitToWidth"], json!(1));
    assert_eq!(p["fitToHeight"], json!(1));
    assert_eq!(p["gridLines"], json!(false));
    assert_eq!(p["headings"], json!(false));
    assert_eq!(p["hCenter"], json!(false));
    assert_eq!(p["vCenter"], json!(false));
    assert_eq!(p["printArea"], serde_json::Value::Null);
    assert_eq!(p["rowBreaks"], json!([]));
    assert_eq!(p["colBreaks"], json!([]));
    assert_eq!(
        object_keys(p),
        [
            "left",
            "right",
            "top",
            "bottom",
            "header",
            "footer",
            "paperSize",
            "orientation",
            "scale",
            "fitToPage",
            "fitToWidth",
            "fitToHeight",
            "gridLines",
            "headings",
            "hCenter",
            "vCenter",
            "printArea",
            "rowBreaks",
            "colBreaks"
        ]
    );
}

#[test]
fn set_margin_orientation_area_grid() {
    let src = create_book("round");
    let out = tmp("set");
    apply(
        &src,
        &out,
        json!([
            {
                "op": "setHeaderFooterText",
                "id": "sheet:Sheet1/hf:odd:header",
                "text": "&C页眉还在"
            },
            {
                "op": "setPrintSetup",
                "id": "sheet:Sheet1",
                "left": 1,
                "orientation": "landscape",
                "printArea": "A1:A10",
                "gridLines": true,
                "paperSize": 9,
                "scale": 80,
                "rowBreaks": [25]
            }
        ]),
    )
    .unwrap();
    let tree = inspect_tree(&out);
    let p = &tree["sheets"][0]["print"];
    assert_eq!(p["left"], json!(1.0));
    assert_eq!(p["orientation"], json!("landscape"));
    assert_eq!(p["printArea"], json!("Sheet1!$A$1:$A$10"));
    assert_eq!(p["gridLines"], json!(true));
    assert_eq!(p["paperSize"], json!(9));
    assert_eq!(p["scale"], json!(80));
    assert_eq!(p["fitToPage"], json!(false));
    assert_eq!(p["rowBreaks"], json!([25]));
    let names = tree["definedNames"].as_array().unwrap();
    assert_eq!(names.len(), 1);
    assert_eq!(names[0]["name"], json!("_xlnm.Print_Area"));
    assert_eq!(names[0]["localSheet"], json!("Sheet1"));
    let ws = part_text(&out, "xl/worksheets/sheet1.xml");
    assert!(ws.contains(r#"left="1""#), "{ws}");
    assert!(ws.contains(r#"orientation="landscape""#), "{ws}");
    assert!(ws.contains(r#"paperSize="9""#), "{ws}");
    assert!(ws.contains(r#"scale="80""#), "{ws}");
    assert!(ws.contains(r#"gridLines="1""#), "{ws}");
    assert!(ws.contains(r#"id="24""#), "{ws}");
    assert!(ws.contains("oddHeader"), "{ws}");
    assert!(ws.contains("页眉还在"), "{ws}");
    let wb = part_text(&out, "xl/workbook.xml");
    assert!(wb.contains("_xlnm.Print_Area"), "{wb}");
    assert!(wb.contains(r#"localSheetId="0""#), "{wb}");
    assert!(wb.contains("Sheet1!$A$1:$A$10"), "{wb}");

    let cleared = tmp("clear");
    apply(
        &out,
        &cleared,
        json!([{
            "op": "setPrintSetup",
            "id": "sheet:Sheet1",
            "printArea": null
        }]),
    )
    .unwrap();
    let tree = inspect_tree(&cleared);
    assert_eq!(
        tree["sheets"][0]["print"]["printArea"],
        serde_json::Value::Null
    );
    assert_eq!(tree["definedNameCount"], json!(0));
}

#[test]
fn missing_field_is_invalid() {
    let src = create_book("miss");
    let out = tmp("miss-out");
    let err = apply(
        &src,
        &out,
        json!([{ "op": "setPrintSetup", "id": "sheet:Sheet1" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());
}

#[test]
fn cell_id_is_invalid_id() {
    let src = create_book("id");
    let out = tmp("id-out");
    let err = apply(
        &src,
        &out,
        json!([{
            "op": "setPrintSetup",
            "id": "cell:Sheet1!A1",
            "left": 1
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
}
