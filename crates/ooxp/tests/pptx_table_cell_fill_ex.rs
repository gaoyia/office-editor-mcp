//! 表单元格非实心填充：渐变、图案、图片，以及 fill 对象契约。

use std::io::Read;
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
    let dir = std::env::temp_dir().join("ooxp-pptx-table-cell-fill-ex");
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

fn inspect_tables(path: &Path) -> Value {
    let RunOutput::Inspect(tree) = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": path.to_string_lossy(),
            "scope": { "tables": true }
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
    v.as_object().unwrap().keys().map(|s| s.as_str()).collect()
}

fn slide_xml(path: &Path) -> String {
    let f = std::fs::File::open(path).unwrap();
    let mut zip = ZipArchive::new(f).unwrap();
    let mut s = String::new();
    zip.by_name("ppt/slides/slide1.xml")
        .unwrap()
        .read_to_string(&mut s)
        .unwrap();
    s
}

fn create_table_deck(tag: &str) -> PathBuf {
    let path = tmp(tag).with_extension("pptx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "presentation": {
                "slides": [{
                    "shapes": [{ "text": "标题" }],
                    "tables": [{
                        "rows": 2,
                        "cols": 2,
                        "x": 914400,
                        "y": 1828800,
                        "cells": [
                            [
                                { "text": "甲", "fill": "FF0000" },
                                "乙"
                            ],
                            ["丙", "丁"]
                        ]
                    }]
                }]
            }
        })
        .to_string(),
        None,
    )
    .unwrap();
    path
}

#[test]
fn create_solid_fill_is_object() {
    let path = create_table_deck("solid");
    let tree = inspect_tables(&path);
    let c = &tree["slides"][0]["tables"][0]["rows"][0]["cells"][0];
    assert_eq!(c["fill"], json!({ "type": "solid", "color": "FF0000" }));
    assert_eq!(object_keys(&c["fill"]), ["type", "color"]);
    assert_eq!(
        tree["slides"][0]["tables"][0]["rows"][0]["cells"][1]["fill"],
        Value::Null
    );
}

#[test]
fn set_table_cell_fill_grad_patt_blip() {
    let src = create_table_deck("ex");
    let png = write_png("cell");
    let out = tmp("filled").with_extension("pptx");
    apply(
        &src,
        &out,
        json!([
            {
                "op": "setTableCellFill",
                "id": "slide:1/table:3/r:1/c:1",
                "fill": {
                    "type": "grad",
                    "ang": 5400000,
                    "stops": [
                        { "pos": 0, "color": "FF0000" },
                        { "pos": 100000, "color": "0000FF" }
                    ]
                }
            },
            {
                "op": "setTableCellFill",
                "id": "slide:1/table:3/r:1/c:2",
                "fill": {
                    "type": "patt",
                    "prst": "smCheck",
                    "fg": "FF0000",
                    "bg": "FFFFFF"
                }
            },
            {
                "op": "setTableCellFill",
                "id": "slide:1/table:3/r:2/c:1",
                "fill": { "type": "blip" },
                "file": png.to_string_lossy()
            }
        ]),
    )
    .unwrap();
    let tree = inspect_tables(&out);
    let c00 = &tree["slides"][0]["tables"][0]["rows"][0]["cells"][0]["fill"];
    assert_eq!(c00["type"], "grad");
    assert_eq!(c00["ang"], 5400000);
    assert_eq!(
        c00["stops"],
        json!([
            { "pos": 0, "color": "FF0000" },
            { "pos": 100000, "color": "0000FF" }
        ])
    );
    assert_eq!(object_keys(c00), ["type", "ang", "stops"]);
    assert_eq!(object_keys(&c00["stops"][0]), ["pos", "color"]);

    let c01 = &tree["slides"][0]["tables"][0]["rows"][0]["cells"][1]["fill"];
    assert_eq!(
        c01,
        &json!({
            "type": "patt",
            "prst": "smCheck",
            "fg": "FF0000",
            "bg": "FFFFFF"
        })
    );
    assert_eq!(object_keys(c01), ["type", "prst", "fg", "bg"]);

    let c10 = &tree["slides"][0]["tables"][0]["rows"][1]["cells"][0]["fill"];
    assert_eq!(c10["type"], "blip");
    let rid = c10["rId"].as_str().unwrap();
    assert!(rid.starts_with("rId"));
    assert_eq!(c10["contentType"], "image/png");
    assert_eq!(object_keys(c10), ["type", "rId", "contentType"]);

    let xml = slide_xml(&out);
    assert!(xml.contains("<a:gradFill"));
    assert!(xml.contains("prst=\"smCheck\""));
    assert!(xml.contains("<a:blipFill"));
    assert!(xml.contains("r:embed"));

    let exported = tmp("cell-export").with_extension("png");
    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "exportMedia",
            "path": out.to_string_lossy(),
            "id": "slide:1/table:3/r:2/c:1",
            "output": exported.to_string_lossy()
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    assert!(!exported.exists());
}

#[test]
fn create_cell_fill_objects() {
    let png = write_png("create-blip");
    let path = tmp("create-obj").with_extension("pptx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "presentation": {
                "slides": [{
                    "shapes": [{ "text": "标题" }],
                    "tables": [{
                        "rows": 1,
                        "cols": 2,
                        "x": 914400,
                        "y": 1828800,
                        "cells": [[
                            {
                                "text": "渐变",
                                "fill": {
                                    "type": "grad",
                                    "stops": [
                                        { "pos": 0, "color": "111111" },
                                        { "pos": 100000, "color": "EEEEEE" }
                                    ]
                                }
                            },
                            {
                                "text": "图",
                                "fill": {
                                    "type": "blip",
                                    "file": png.to_string_lossy()
                                }
                            }
                        ]]
                    }]
                }]
            }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let tree = inspect_tables(&path);
    let c0 = &tree["slides"][0]["tables"][0]["rows"][0]["cells"][0]["fill"];
    assert_eq!(c0["type"], "grad");
    assert_eq!(c0["ang"], 5400000);
    let c1 = &tree["slides"][0]["tables"][0]["rows"][0]["cells"][1]["fill"];
    assert_eq!(c1["type"], "blip");
    assert_eq!(c1["contentType"], "image/png");
}

#[test]
fn set_table_cell_fill_rejects_bad_payload() {
    let src = create_table_deck("bad");
    let id = "slide:1/table:3/r:1/c:1";

    let err = apply(
        &src,
        &tmp("type").with_extension("pptx"),
        json!([{ "op": "setTableCellFill", "id": id, "fill": { "color": "FF0000" } }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let err = apply(
        &src,
        &tmp("other").with_extension("pptx"),
        json!([{ "op": "setTableCellFill", "id": id, "fill": { "type": "other" } }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let err = apply(
        &src,
        &tmp("none").with_extension("pptx"),
        json!([{ "op": "setTableCellFill", "id": id, "fill": { "type": "none" } }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let err = apply(
        &src,
        &tmp("prst").with_extension("pptx"),
        json!([{
            "op": "setTableCellFill",
            "id": id,
            "fill": { "type": "patt", "prst": "foo", "fg": "FF0000", "bg": "FFFFFF" }
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let err = apply(
        &src,
        &tmp("stops").with_extension("pptx"),
        json!([{
            "op": "setTableCellFill",
            "id": id,
            "fill": { "type": "grad", "stops": [{ "pos": 0, "color": "FF0000" }] }
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let err = apply(
        &src,
        &tmp("blip-file").with_extension("pptx"),
        json!([{ "op": "setTableCellFill", "id": id, "fill": { "type": "blip" } }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let png = write_png("file-on-solid");
    let err = apply(
        &src,
        &tmp("file-solid").with_extension("pptx"),
        json!([{
            "op": "setTableCellFill",
            "id": id,
            "fill": { "type": "solid", "color": "FF0000" },
            "file": png.to_string_lossy()
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let err = apply(
        &src,
        &tmp("table-id").with_extension("pptx"),
        json!([{
            "op": "setTableCellFill",
            "id": "slide:1/table:3",
            "fill": { "type": "solid", "color": "FF0000" }
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);

    let xlsx = tmp("xlsx").with_extension("xlsx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": xlsx.to_string_lossy(),
            "workbook": { "sheets": [{ "name": "Sheet1" }] }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let err = apply(
        &xlsx,
        &tmp("xlsx-out").with_extension("xlsx"),
        json!([{
            "op": "setTableCellFill",
            "id": id,
            "fill": { "type": "solid", "color": "FF0000" }
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}
