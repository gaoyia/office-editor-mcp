//! 配色映射与页级覆盖：`clrMap` / `clrMapOvr` / `themeOverride`。

use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::ZipArchive;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-pptx-theme-clrmap");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn tmp(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}.pptx"))
}

fn create_deck(tag: &str) -> PathBuf {
    let path = tmp(tag);
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "presentation": {
                "handout": { "header": "讲义头" },
                "slides": [
                    {
                        "shapes": [{ "text": "标题" }],
                        "notes": "备注"
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

fn inspect_scope(path: &Path, scope: Value) -> Value {
    let req = json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": path.to_string_lossy(),
        "scope": scope
    });
    let RunOutput::Inspect(tree) = run(&req.to_string(), None).unwrap() else {
        panic!("inspect 应返回树");
    };
    tree
}

fn object_keys(v: &Value) -> Vec<&str> {
    v.as_object().unwrap().keys().map(|s| s.as_str()).collect()
}

fn part_names(path: &Path) -> Vec<String> {
    let bytes = std::fs::read(path).unwrap();
    let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut names = Vec::new();
    for i in 0..zip.len() {
        names.push(zip.by_index(i).unwrap().name().to_string());
    }
    names
}

fn part(path: &Path, name: &str) -> String {
    let bytes = std::fs::read(path).unwrap();
    let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut file = zip.by_name(name).unwrap();
    let mut out = String::new();
    file.read_to_string(&mut out).unwrap();
    out
}

#[test]
fn inspect_clr_map_and_ovr_keys() {
    let path = create_deck("keys");
    let tree = inspect_scope(
        &path,
        json!({ "masters": true, "notesMaster": true, "handout": true, "notes": true }),
    );
    let m = &tree["masters"][0];
    assert_eq!(
        object_keys(m)[..6],
        ["id", "index", "name", "theme", "clrMap", "transition"]
    );
    assert_eq!(
        object_keys(&m["clrMap"]),
        [
            "bg1", "tx1", "bg2", "tx2", "accent1", "accent2", "accent3", "accent4", "accent5",
            "accent6", "hlink", "folHlink"
        ]
    );
    assert_eq!(m["clrMap"]["bg1"], "lt1");
    assert_eq!(m["clrMap"]["tx1"], "dk1");
    let layout = &m["layouts"][0];
    assert_eq!(layout["clrMapOvr"], "master");
    let slide = &tree["slides"][0];
    assert_eq!(slide["clrMapOvr"], "master");
    assert!(slide["themeOverride"].is_null());
    assert_eq!(tree["notesMaster"]["clrMap"]["bg1"], "lt1");
    assert_eq!(tree["handout"]["clrMap"]["bg1"], "lt1");
    assert_eq!(slide["notes"]["clrMapOvr"], "master");
    assert!(slide["notes"]["themeOverride"].is_null());
}

#[test]
fn set_clr_map_writes_master_and_hosts() {
    let src = create_deck("map");
    let out = tmp("map-out");
    apply(
        &src,
        &out,
        json!([
            { "op": "setClrMap", "id": "master:1", "bg1": "dk1" },
            { "op": "setClrMap", "id": "notesMaster", "tx1": "lt1" },
            { "op": "setClrMap", "id": "handout", "bg2": "dk2" }
        ]),
    )
    .unwrap();
    let tree = inspect_scope(
        &out,
        json!({ "masters": true, "notesMaster": true, "handout": true }),
    );
    assert_eq!(tree["masters"][0]["clrMap"]["bg1"], "dk1");
    assert_eq!(tree["masters"][0]["clrMap"]["tx1"], "dk1");
    assert_eq!(tree["notesMaster"]["clrMap"]["tx1"], "lt1");
    assert_eq!(tree["handout"]["clrMap"]["bg2"], "dk2");
    let xml = part(&out, "ppt/slideMasters/slideMaster1.xml");
    assert!(xml.contains(r#"bg1="dk1""#));
}

#[test]
fn set_clr_map_ovr_master_and_object() {
    let src = create_deck("ovr");
    let out = tmp("ovr-obj");
    apply(
        &src,
        &out,
        json!([{
            "op": "setClrMapOvr",
            "id": "slide:1",
            "clrMapOvr": { "bg1": "dk1" }
        }]),
    )
    .unwrap();
    let tree = inspect_scope(&out, json!({ "masters": true, "notes": true }));
    assert_eq!(tree["slides"][0]["clrMapOvr"]["bg1"], "dk1");
    assert_eq!(tree["slides"][0]["clrMapOvr"]["tx1"], "dk1");
    let xml = part(&out, "ppt/slides/slide1.xml");
    assert!(xml.contains("overrideClrMapping"));
    assert!(xml.contains(r#"bg1="dk1""#));

    let layout_out = tmp("ovr-layout");
    apply(
        &out,
        &layout_out,
        json!([{
            "op": "setClrMapOvr",
            "id": "master:1/layout:1",
            "clrMapOvr": { "tx1": "lt1" }
        }]),
    )
    .unwrap();
    let tree = inspect_scope(&layout_out, json!({ "masters": true }));
    assert_eq!(tree["masters"][0]["layouts"][0]["clrMapOvr"]["tx1"], "lt1");

    let notes_out = tmp("ovr-notes");
    apply(
        &src,
        &notes_out,
        json!([{
            "op": "setClrMapOvr",
            "id": "slide:1/notes",
            "clrMapOvr": { "bg1": "dk1" }
        }]),
    )
    .unwrap();
    let tree = inspect_scope(&notes_out, json!({ "notes": true }));
    assert_eq!(tree["slides"][0]["notes"]["clrMapOvr"]["bg1"], "dk1");

    let master_out = tmp("ovr-master");
    apply(
        &out,
        &master_out,
        json!([{
            "op": "setClrMapOvr",
            "id": "slide:1",
            "clrMapOvr": "master"
        }]),
    )
    .unwrap();
    let tree = inspect_scope(&master_out, json!({}));
    assert_eq!(tree["slides"][0]["clrMapOvr"], "master");
    let xml = part(&master_out, "ppt/slides/slide1.xml");
    assert!(xml.contains("masterClrMapping"));
    assert!(!xml.contains("overrideClrMapping"));
}

#[test]
fn set_theme_override_creates_and_clears() {
    let src = create_deck("ov");
    let out = tmp("ov-set");
    apply(
        &src,
        &out,
        json!([{ "op": "setThemeOverride", "id": "slide:1", "accent1": "FF0000" }]),
    )
    .unwrap();
    let tree = inspect_scope(&out, json!({ "notes": true }));
    assert_eq!(
        tree["slides"][0]["themeOverride"]["colors"]["accent1"],
        "FF0000"
    );
    let names = part_names(&out);
    assert!(names.iter().any(|n| n.contains("themeOverride")));
    let xml = part(&out, "ppt/slides/_rels/slide1.xml.rels");
    assert!(xml.contains("themeOverride"));

    let notes_out = tmp("ov-notes");
    apply(
        &src,
        &notes_out,
        json!([{ "op": "setThemeOverride", "id": "slide:1/notes", "accent1": "00FF00" }]),
    )
    .unwrap();
    let tree = inspect_scope(&notes_out, json!({ "notes": true }));
    assert_eq!(
        tree["slides"][0]["notes"]["themeOverride"]["colors"]["accent1"],
        "00FF00"
    );

    let cleared = tmp("ov-clear");
    apply(
        &out,
        &cleared,
        json!([{ "op": "setThemeOverride", "id": "slide:1", "enabled": false }]),
    )
    .unwrap();
    let tree = inspect_scope(&cleared, json!({}));
    assert!(tree["slides"][0]["themeOverride"].is_null());
    let names = part_names(&cleared);
    assert!(!names.iter().any(|n| n.contains("themeOverride")));
}

#[test]
fn reject_clr_map_ids_and_tokens() {
    let src = create_deck("bad");
    let out = tmp("map-slide");
    let err = apply(
        &src,
        &out,
        json!([{ "op": "setClrMap", "id": "slide:1", "bg1": "dk1" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    assert!(!out.exists());

    let out = tmp("map-empty");
    let err = apply(&src, &out, json!([{ "op": "setClrMap", "id": "master:1" }])).unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());

    let out = tmp("map-token");
    let err = apply(
        &src,
        &out,
        json!([{ "op": "setClrMap", "id": "master:1", "bg1": "nope" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());

    let out = tmp("ovr-master");
    let err = apply(
        &src,
        &out,
        json!([{ "op": "setThemeOverride", "id": "master:1", "accent1": "FF0000" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    assert!(!out.exists());

    let out = tmp("ovr-empty");
    let err = apply(
        &src,
        &out,
        json!([{ "op": "setClrMapOvr", "id": "slide:1" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());
}

#[test]
fn xlsx_and_docx_reject_clr_map_ops() {
    let xlsx = tmp_dir().join("tcm.xlsx");
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
    let out = tmp_dir().join("xlsx-tcm.xlsx");
    let err = apply(
        &xlsx,
        &out,
        json!([{ "op": "setClrMap", "id": "master:1", "bg1": "dk1" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());

    let out = tmp_dir().join("xlsx-tcm-ovr.xlsx");
    let err = apply(
        &xlsx,
        &out,
        json!([{ "op": "setClrMapOvr", "id": "slide:1", "clrMapOvr": "master" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());

    let docx = tmp_dir().join("tcm.docx");
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
    let out = tmp_dir().join("docx-tcm.docx");
    let err = apply(
        &docx,
        &out,
        json!([{ "op": "setClrMap", "id": "master:1", "bg1": "dk1" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());
}
