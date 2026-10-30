//! 备注与讲义主题：inspect `notesMaster.theme` / `handout.theme`，以及对应宿主的 theme op。

use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::ZipArchive;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-pptx-theme-notes-handout");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn tmp(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}.pptx"))
}

fn create_deck(tag: &str) -> PathBuf {
    let path = tmp(tag);
    run(&json!({
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
    .to_string())
    .unwrap();
    path
}

fn create_plain(tag: &str) -> PathBuf {
    let path = tmp(tag);
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "presentation": {
            "slides": [{ "shapes": [{ "text": "标题" }] }]
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

fn inspect_scope(path: &Path, scope: Value) -> Value {
    let req = json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": path.to_string_lossy(),
        "scope": scope
    });
    let RunOutput::Inspect(tree) = run(&req.to_string()).unwrap() else {
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
fn inspect_notes_and_handout_theme_after_id() {
    let path = create_deck("keys");
    let tree = inspect_scope(
        &path,
        json!({ "masters": true, "notesMaster": true, "handout": true }),
    );
    let notes = &tree["notesMaster"];
    let handout = &tree["handout"];
    assert_eq!(object_keys(notes)[..3], ["id", "theme", "clrMap"]);
    assert_eq!(object_keys(handout)[..3], ["id", "theme", "clrMap"]);
    assert_eq!(notes["theme"]["colors"]["accent1"], "4F81BD");
    assert_eq!(handout["theme"]["colors"]["accent1"], "4F81BD");
    assert_eq!(tree["masters"][0]["theme"]["colors"]["accent1"], "4F81BD");
    let names = part_names(&path);
    let themes: Vec<_> = names
        .iter()
        .filter(|n| n.starts_with("ppt/theme/theme") && n.ends_with(".xml"))
        .collect();
    assert!(themes.len() >= 3, "独立 theme 副本不够: {themes:?}");
}

#[test]
fn set_theme_colors_notes_does_not_change_master() {
    let src = create_deck("colors");
    let out = tmp("colors-out");
    apply(
        &src,
        &out,
        json!([
            { "op": "setThemeColors", "id": "notesMaster", "accent1": "FF0000" },
            { "op": "setThemeColors", "id": "handout", "accent1": "00FF00" }
        ]),
    )
    .unwrap();
    let tree = inspect_scope(
        &out,
        json!({ "masters": true, "notesMaster": true, "handout": true }),
    );
    assert_eq!(tree["notesMaster"]["theme"]["colors"]["accent1"], "FF0000");
    assert_eq!(tree["handout"]["theme"]["colors"]["accent1"], "00FF00");
    assert_eq!(tree["masters"][0]["theme"]["colors"]["accent1"], "4F81BD");
    let notes_xml = part(&out, "ppt/notesMasters/_rels/notesMaster1.xml.rels");
    assert!(notes_xml.contains("/theme"));
}

#[test]
fn set_theme_fonts_and_fmt_on_notes_master() {
    let src = create_deck("fmt");
    let out = tmp("fmt-out");
    apply(
        &src,
        &out,
        json!([
            { "op": "setThemeFonts", "id": "notesMaster", "majorLatin": "Arial" },
            {
                "op": "setThemeFmt",
                "id": "notesMaster",
                "line": { "index": 0, "w": 38100 }
            }
        ]),
    )
    .unwrap();
    let tree = inspect_scope(&out, json!({ "notesMaster": true }));
    assert_eq!(tree["notesMaster"]["theme"]["fonts"]["majorLatin"], "Arial");
    assert_eq!(tree["notesMaster"]["theme"]["fmt"]["lines"][0]["w"], 38100);
}

#[test]
fn set_theme_extra_schemes_on_handout() {
    let src = create_deck("extra");
    let out = tmp("extra-out");
    apply(
        &src,
        &out,
        json!([{
            "op": "setThemeExtraSchemes",
            "id": "handout",
            "extraSchemes": [{
                "name": "Extra",
                "colors": { "accent1": "0F6FC6" }
            }]
        }]),
    )
    .unwrap();
    let tree = inspect_scope(&out, json!({ "handout": true }));
    assert_eq!(tree["handout"]["theme"]["extraSchemes"][0]["name"], "Extra");
    assert_eq!(
        tree["handout"]["theme"]["extraSchemes"][0]["colors"]["accent1"],
        "0F6FC6"
    );
}

#[test]
fn reject_page_id_and_missing_notes_master() {
    let src = create_deck("bad-page");
    let out = tmp("page");
    let err = apply(
        &src,
        &out,
        json!([{ "op": "setThemeColors", "id": "slide:1", "accent1": "FF0000" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    assert!(!out.exists());

    let plain = create_plain("no-notes");
    let out = tmp("missing");
    let err = apply(
        &plain,
        &out,
        json!([{ "op": "setThemeColors", "id": "notesMaster", "accent1": "FF0000" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::IdNotFound);
    assert!(!out.exists());

    let xlsx = tmp_dir().join("tnh.xlsx");
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": xlsx.to_string_lossy(),
        "workbook": { "sheets": [{ "name": "Sheet1", "cells": [{ "ref": "A1", "value": 1 }] }] }
    })
    .to_string())
    .unwrap();
    let out = tmp_dir().join("xlsx-tnh.xlsx");
    let err = apply(
        &xlsx,
        &out,
        json!([{ "op": "setThemeColors", "id": "notesMaster", "accent1": "FF0000" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    assert!(!out.exists());
}
