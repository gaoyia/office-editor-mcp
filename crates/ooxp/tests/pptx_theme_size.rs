//! 主题与幻灯片尺寸：inspect `slideSize` / `theme`，以及 `setSlideSize` / `setThemeColors` / `setThemeFonts` / `setThemeFmt` / `setThemeExtraSchemes`。

use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-pptx-theme-size");
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
                "slides": [
                    { "shapes": [{ "text": "标题" }] }
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

fn inspect(path: &Path) -> Value {
    inspect_scope(path, None)
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

fn object_keys(v: &Value) -> Vec<&str> {
    v.as_object().unwrap().keys().map(|s| s.as_str()).collect()
}

fn part(path: &Path, name: &str) -> String {
    let bytes = std::fs::read(path).unwrap();
    let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut file = zip.by_name(name).unwrap();
    let mut out = String::new();
    file.read_to_string(&mut out).unwrap();
    out
}

fn rewrite_part(path: &Path, name: &str, f: impl FnOnce(Vec<u8>) -> Vec<u8>) {
    let bytes = std::fs::read(path).unwrap();
    let mut zip_in = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut out = Cursor::new(Vec::new());
    let mut transform = Some(f);
    {
        let mut zip_out = ZipWriter::new(&mut out);
        let opt = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        for i in 0..zip_in.len() {
            let mut file = zip_in.by_index(i).unwrap();
            let n = file.name().to_string();
            let mut data = Vec::new();
            file.read_to_end(&mut data).unwrap();
            if n == name {
                data = transform.take().expect("部件只应出现一次")(data);
            }
            zip_out.start_file(&n, opt).unwrap();
            zip_out.write_all(&data).unwrap();
        }
        zip_out.finish().unwrap();
    }
    std::fs::write(path, out.into_inner()).unwrap();
}

fn notes_sz_snip(xml: &str) -> &str {
    let start = xml.find("<p:notesSz").expect("应有 notesSz");
    let rest = &xml[start..];
    let end = rest.find('>').expect("notesSz 应闭合") + 1;
    &rest[..end]
}

#[test]
fn inspect_slide_size_and_theme_keys() {
    let path = create_deck("keys");
    let tree = inspect(&path);
    assert_eq!(tree["slideSize"]["cx"], 12192000);
    assert_eq!(tree["slideSize"]["cy"], 6858000);
    assert!(tree["slideSize"]["type"].is_null());
    assert_eq!(object_keys(&tree["slideSize"]), ["cx", "cy", "type"]);

    let masters = inspect_scope(&path, Some(json!({ "masters": true })));
    let m = &masters["masters"][0];
    assert_eq!(
        object_keys(m),
        [
            "id",
            "index",
            "name",
            "theme",
            "clrMap",
            "transition",
            "animCount",
            "placeholders",
            "shapes",
            "layouts"
        ]
    );
    let theme = &m["theme"];
    assert_eq!(
        object_keys(theme),
        ["name", "colors", "fonts", "fmt", "extraSchemes"]
    );
    assert_eq!(theme["name"], "Office Theme");
    assert_eq!(
        object_keys(&theme["colors"]),
        [
            "name", "dk1", "lt1", "dk2", "lt2", "accent1", "accent2", "accent3", "accent4",
            "accent5", "accent6", "hlink", "folHlink"
        ]
    );
    assert_eq!(theme["colors"]["name"], "Office");
    assert_eq!(theme["colors"]["dk1"], "000000");
    assert_eq!(theme["colors"]["lt1"], "FFFFFF");
    assert_eq!(theme["colors"]["accent1"], "4F81BD");
    assert_eq!(
        object_keys(&theme["fonts"]),
        [
            "name",
            "majorLatin",
            "minorLatin",
            "majorEa",
            "minorEa",
            "majorCs",
            "minorCs",
            "majorScripts",
            "minorScripts"
        ]
    );
    assert_eq!(theme["fonts"]["name"], "Office");
    assert_eq!(theme["fonts"]["majorLatin"], "Calibri Light");
    assert_eq!(theme["fonts"]["minorLatin"], "Calibri");
    assert_eq!(theme["fonts"]["majorEa"], "");
    assert_eq!(theme["fonts"]["majorScripts"], json!([]));
    assert_eq!(theme["fonts"]["minorScripts"], json!([]));
    assert_eq!(
        object_keys(&theme["fmt"]),
        ["name", "fills", "lines", "effects", "bgFills"]
    );
    assert_eq!(theme["extraSchemes"], json!([]));
}

#[test]
fn set_slide_size_writes_sld_sz_and_leaves_shapes() {
    let src = create_deck("size");
    let before = inspect_scope(&src, Some(json!({ "slide": 1 })));
    let sp = &before["slides"][0]["shapes"][0];
    let x = sp["x"].clone();
    let y = sp["y"].clone();
    let cx = sp["cx"].clone();
    let cy = sp["cy"].clone();
    let notes0 = notes_sz_snip(&part(&src, "ppt/presentation.xml")).to_string();
    let slide0 = part(&src, "ppt/slides/slide1.xml");

    let out = tmp("size-out");
    apply(
        &src,
        &out,
        json!([{
            "op": "setSlideSize",
            "id": "pptx:presentation",
            "cx": 9144000,
            "cy": 6858000
        }]),
    )
    .unwrap();
    let tree = inspect(&out);
    assert_eq!(tree["slideSize"]["cx"], 9144000);
    assert_eq!(tree["slideSize"]["cy"], 6858000);
    assert!(tree["slideSize"]["type"].is_null());

    let after = inspect_scope(&out, Some(json!({ "slide": 1 })));
    let sp = &after["slides"][0]["shapes"][0];
    assert_eq!(sp["x"], x);
    assert_eq!(sp["y"], y);
    assert_eq!(sp["cx"], cx);
    assert_eq!(sp["cy"], cy);

    let pres = part(&out, "ppt/presentation.xml");
    assert!(pres.contains(r#"cx="9144000""#));
    assert!(!pres.contains(r#"type="#));
    assert_eq!(notes_sz_snip(&pres), notes0);
    assert_eq!(part(&out, "ppt/slides/slide1.xml"), slide0);
}

#[test]
fn set_slide_size_type_and_insert_when_missing() {
    let src = create_deck("type");
    let typed = tmp("typed");
    apply(
        &src,
        &typed,
        json!([{
            "op": "setSlideSize",
            "id": "pptx:presentation",
            "cx": 9144000,
            "cy": 6858000,
            "type": "custom"
        }]),
    )
    .unwrap();
    let tree = inspect(&typed);
    assert_eq!(tree["slideSize"]["type"], "custom");
    assert!(part(&typed, "ppt/presentation.xml").contains(r#"type="custom""#));

    rewrite_part(&src, "ppt/presentation.xml", |data| {
        let xml = String::from_utf8(data).unwrap();
        xml.replace(r#"<p:sldSz cx="12192000" cy="6858000"/>"#, "")
            .into_bytes()
    });
    assert!(inspect(&src)["slideSize"].is_null());
    let neu = tmp("insert-sz");
    apply(
        &src,
        &neu,
        json!([{
            "op": "setSlideSize",
            "id": "pptx:presentation",
            "cx": 12192000,
            "cy": 6858000
        }]),
    )
    .unwrap();
    let tree = inspect(&neu);
    assert_eq!(tree["slideSize"]["cx"], 12192000);
    assert_eq!(tree["slideSize"]["cy"], 6858000);
    assert!(tree["slideSize"]["type"].is_null());
    let xml = part(&neu, "ppt/presentation.xml");
    let sld = xml.find("<p:sldSz").expect("应写入 sldSz");
    let notes = xml.find("<p:notesSz").expect("应保留 notesSz");
    assert!(sld < notes, "sldSz 应在 notesSz 之前");
}

#[test]
fn set_theme_colors_accent1_keeps_sys_slots_and_fmt() {
    let src = create_deck("colors");
    let theme0 = part(&src, "ppt/theme/theme1.xml");
    let fmt0 = {
        let a = theme0.find("<a:fmtScheme").expect("应有 fmtScheme");
        let b = theme0.find("</a:fmtScheme>").expect("fmtScheme 应闭合") + "</a:fmtScheme>".len();
        theme0[a..b].to_string()
    };
    let out = tmp("colors-out");
    apply(
        &src,
        &out,
        json!([{ "op": "setThemeColors", "id": "master:1", "accent1": "FF0000" }]),
    )
    .unwrap();
    let tree = inspect_scope(&out, Some(json!({ "masters": true })));
    assert_eq!(tree["masters"][0]["theme"]["colors"]["accent1"], "FF0000");
    assert_eq!(tree["masters"][0]["theme"]["colors"]["dk1"], "000000");
    assert_eq!(tree["masters"][0]["theme"]["colors"]["accent2"], "C0504D");
    let xml = part(&out, "ppt/theme/theme1.xml");
    assert!(xml.contains(r#"<a:accent1><a:srgbClr val="FF0000"/></a:accent1>"#));
    assert!(xml.contains(r#"<a:sysClr val="windowText" lastClr="000000"/>"#));
    let a = xml.find("<a:fmtScheme").expect("应有 fmtScheme");
    let b = xml.find("</a:fmtScheme>").expect("fmtScheme 应闭合") + "</a:fmtScheme>".len();
    assert_eq!(&xml[a..b], fmt0);
    assert_eq!(
        inspect_scope(&out, Some(json!({ "slide": 1 })))["slides"][0]["shapes"][0]["x"],
        838200
    );
}

#[test]
fn set_theme_fonts_major_latin() {
    let src = create_deck("fonts");
    let out = tmp("fonts-out");
    apply(
        &src,
        &out,
        json!([{ "op": "setThemeFonts", "id": "master:1", "majorLatin": "Arial" }]),
    )
    .unwrap();
    let tree = inspect_scope(&out, Some(json!({ "masters": true })));
    assert_eq!(tree["masters"][0]["theme"]["fonts"]["majorLatin"], "Arial");
    assert_eq!(
        tree["masters"][0]["theme"]["fonts"]["minorLatin"],
        "Calibri"
    );
    let xml = part(&out, "ppt/theme/theme1.xml");
    assert!(xml.contains(r#"<a:latin typeface="Arial"/>"#));
    assert!(xml.contains(r#"<a:latin typeface="Calibri"/>"#));
}

#[test]
fn set_theme_fonts_empty_ea_ok() {
    let src = create_deck("ea");
    let out = tmp("ea-out");
    apply(
        &src,
        &out,
        json!([{ "op": "setThemeFonts", "id": "master:1", "majorEa": "" }]),
    )
    .unwrap();
    let tree = inspect_scope(&out, Some(json!({ "masters": true })));
    assert_eq!(tree["masters"][0]["theme"]["fonts"]["majorEa"], "");
}

#[test]
fn reject_bad_ids_range_and_other_formats() {
    let src = create_deck("bad");
    let out = tmp("slide-id");
    let err = apply(
        &src,
        &out,
        json!([{
            "op": "setSlideSize",
            "id": "slide:1",
            "cx": 9144000,
            "cy": 6858000
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    assert!(!out.exists());

    let out = tmp("layout-theme");
    let err = apply(
        &src,
        &out,
        json!([{ "op": "setThemeColors", "id": "master:1/layout:1", "accent1": "FF0000" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    assert!(!out.exists());

    let out = tmp("missing-master");
    let err = apply(
        &src,
        &out,
        json!([{ "op": "setThemeFonts", "id": "master:9", "majorLatin": "Arial" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::IdNotFound);
    assert!(!out.exists());

    let out = tmp("cx-low");
    let err = apply(
        &src,
        &out,
        json!([{
            "op": "setSlideSize",
            "id": "pptx:presentation",
            "cx": 1,
            "cy": 6858000
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());

    let out = tmp("type-other");
    let err = apply(
        &src,
        &out,
        json!([{
            "op": "setSlideSize",
            "id": "pptx:presentation",
            "cx": 9144000,
            "cy": 6858000,
            "type": "other"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());

    let out = tmp("empty-latin");
    let err = apply(
        &src,
        &out,
        json!([{ "op": "setThemeFonts", "id": "master:1", "majorLatin": "" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());

    let out = tmp("no-slot");
    let err = apply(
        &src,
        &out,
        json!([{ "op": "setThemeColors", "id": "master:1" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());

    let xlsx = tmp_dir().join("ts.xlsx");
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
    let out = tmp_dir().join("xlsx-ts.xlsx");
    let err = apply(
        &xlsx,
        &out,
        json!([{
            "op": "setSlideSize",
            "id": "pptx:presentation",
            "cx": 9144000,
            "cy": 6858000
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());

    let docx = tmp_dir().join("ts.docx");
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
    let out = tmp_dir().join("docx-ts.docx");
    let err = apply(
        &docx,
        &out,
        json!([{ "op": "setThemeColors", "id": "master:1", "accent1": "FF0000" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    assert!(!out.exists());
}

#[test]
fn set_theme_fmt_line_and_fill() {
    let src = create_deck("fmt");
    let out = tmp("fmt-out");
    apply(
        &src,
        &out,
        json!([{
            "op": "setThemeFmt",
            "id": "master:1",
            "line": { "index": 0, "w": 38100 },
            "fill": { "index": 0, "kind": "solid" }
        }]),
    )
    .unwrap();
    let tree = inspect_scope(&out, Some(json!({ "masters": true })));
    assert_eq!(tree["masters"][0]["theme"]["fmt"]["lines"][0]["w"], 38100);
    assert_eq!(
        tree["masters"][0]["theme"]["fmt"]["fills"][0]["kind"],
        "solid"
    );
}

#[test]
fn set_theme_names_scripts_and_extra() {
    let src = create_deck("remain");
    let out = tmp("remain-out");
    apply(
        &src,
        &out,
        json!([
            {
                "op": "setThemeColors",
                "id": "master:1",
                "name": "Custom",
                "themeName": "Custom Theme"
            },
            {
                "op": "setThemeFonts",
                "id": "master:1",
                "name": "CustomFonts",
                "majorScripts": [{ "script": "Jpan", "typeface": "Yu Mincho" }]
            },
            {
                "op": "setThemeFmt",
                "id": "master:1",
                "name": "CustomFmt"
            },
            {
                "op": "setThemeExtraSchemes",
                "id": "master:1",
                "extraSchemes": [{
                    "name": "Extra",
                    "colors": { "accent1": "0F6FC6" }
                }]
            }
        ]),
    )
    .unwrap();
    let tree = inspect_scope(&out, Some(json!({ "masters": true })));
    let theme = &tree["masters"][0]["theme"];
    assert_eq!(theme["name"], "Custom Theme");
    assert_eq!(theme["colors"]["name"], "Custom");
    assert_eq!(theme["fonts"]["name"], "CustomFonts");
    assert_eq!(theme["fmt"]["name"], "CustomFmt");
    assert_eq!(theme["fonts"]["majorScripts"][0]["script"], "Jpan");
    assert_eq!(theme["extraSchemes"][0]["name"], "Extra");
    assert_eq!(theme["extraSchemes"][0]["colors"]["accent1"], "0F6FC6");
    let xml = part(&out, "ppt/theme/theme1.xml");
    assert!(xml.contains(r#"name="Custom Theme""#));
    assert!(xml.contains(r#"<a:font script="Jpan" typeface="Yu Mincho"/>"#));
    let cleared = tmp("remain-clear");
    apply(
        &out,
        &cleared,
        json!([
            { "op": "setThemeFonts", "id": "master:1", "majorScripts": [] },
            { "op": "setThemeExtraSchemes", "id": "master:1", "extraSchemes": null }
        ]),
    )
    .unwrap();
    let tree = inspect_scope(&cleared, Some(json!({ "masters": true })));
    assert_eq!(
        tree["masters"][0]["theme"]["fonts"]["majorScripts"],
        json!([])
    );
    assert_eq!(tree["masters"][0]["theme"]["extraSchemes"], json!([]));
}

#[test]
fn reject_theme_remain_ids_and_empty() {
    let src = create_deck("remain-bad");
    let out = tmp("fmt-slide");
    let err = apply(
        &src,
        &out,
        json!([{ "op": "setThemeFmt", "id": "slide:1", "name": "X" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    assert!(!out.exists());

    let out = tmp("fmt-empty");
    let err = apply(
        &src,
        &out,
        json!([{ "op": "setThemeFmt", "id": "master:1" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());

    let out = tmp("notes-theme");
    let err = apply(
        &src,
        &out,
        json!([{ "op": "setThemeColors", "id": "notesMaster", "accent1": "FF0000" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    assert!(!out.exists());

    let out = tmp("override");
    let err = apply(
        &src,
        &out,
        json!([{ "op": "setThemeOverride", "id": "master:1", "accent1": "FF0000" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    assert!(!out.exists());
}
