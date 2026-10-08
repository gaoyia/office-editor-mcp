//! docx 主题：inspect 根上 `theme`，setThemeColors / setThemeFonts。

use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

static SEQ: AtomicU32 = AtomicU32::new(0);

const THEME_XML: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="Office Theme"><a:themeElements><a:clrScheme name="Office"><a:dk1><a:sysClr val="windowText" lastClr="000000"/></a:dk1><a:lt1><a:sysClr val="window" lastClr="FFFFFF"/></a:lt1><a:dk2><a:srgbClr val="1F497D"/></a:dk2><a:lt2><a:srgbClr val="EEECE1"/></a:lt2><a:accent1><a:srgbClr val="4F81BD"/></a:accent1><a:accent2><a:srgbClr val="C0504D"/></a:accent2><a:accent3><a:srgbClr val="9BBB59"/></a:accent3><a:accent4><a:srgbClr val="8064A2"/></a:accent4><a:accent5><a:srgbClr val="4BACC6"/></a:accent5><a:accent6><a:srgbClr val="F79646"/></a:accent6><a:hlink><a:srgbClr val="0000FF"/></a:hlink><a:folHlink><a:srgbClr val="800080"/></a:folHlink></a:clrScheme><a:fontScheme name="Office"><a:majorFont><a:latin typeface="Calibri Light" panose="020F0302020204030204"/><a:ea typeface=""/><a:cs typeface=""/></a:majorFont><a:minorFont><a:latin typeface="Calibri"/><a:ea typeface=""/><a:cs typeface=""/></a:minorFont></a:fontScheme><a:fmtScheme name="Office"><a:fillStyleLst><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:fillStyleLst><a:lnStyleLst><a:ln w="9525"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:ln></a:lnStyleLst><a:effectStyleLst><a:effectStyle><a:effectLst/></a:effectStyle></a:effectStyleLst><a:bgFillStyleLst><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:bgFillStyleLst></a:fmtScheme></a:themeElements></a:theme>"#;

const COLOR_KEYS: [&str; 13] = [
    "name", "dk1", "lt1", "dk2", "lt2", "accent1", "accent2", "accent3", "accent4", "accent5",
    "accent6", "hlink", "folHlink",
];

const FONT_KEYS: [&str; 7] = [
    "name",
    "majorLatin",
    "minorLatin",
    "majorEa",
    "minorEa",
    "majorCs",
    "minorCs",
];

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-docx-theme");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn tmp(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}.docx"))
}

fn create_docx(tag: &str) -> PathBuf {
    let path = tmp(tag);
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "format": "docx",
            "document": {
                "paragraphs": [{ "text": "hello" }]
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

fn object_keys(v: &Value) -> Vec<&str> {
    v.as_object().unwrap().keys().map(|s| s.as_str()).collect()
}

fn err_code(err: ooxp::Error) -> ErrorCode {
    err.code()
}

fn inject_theme(src: &Path) -> PathBuf {
    let dest = tmp("with-theme");
    let bytes = std::fs::read(src).unwrap();
    let mut zip_in = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut out = Cursor::new(Vec::new());
    {
        let mut zip_out = ZipWriter::new(&mut out);
        let opt = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        for i in 0..zip_in.len() {
            let mut file = zip_in.by_index(i).unwrap();
            let n = file.name().to_string();
            let mut data = Vec::new();
            file.read_to_end(&mut data).unwrap();
            if n == "word/_rels/document.xml.rels" {
                let s = String::from_utf8(data).unwrap();
                let s = s.replace(
                    "</Relationships>",
                    r#"<Relationship Id="rIdTheme" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="theme/theme1.xml"/></Relationships>"#,
                );
                data = s.into_bytes();
            } else if n == "[Content_Types].xml" {
                let s = String::from_utf8(data).unwrap();
                let s = s.replace(
                    "</Types>",
                    r#"<Override PartName="/word/theme/theme1.xml" ContentType="application/vnd.openxmlformats-officedocument.theme+xml"/></Types>"#,
                );
                data = s.into_bytes();
            }
            zip_out.start_file(&n, opt).unwrap();
            zip_out.write_all(&data).unwrap();
        }
        zip_out.start_file("word/theme/theme1.xml", opt).unwrap();
        zip_out.write_all(THEME_XML.as_bytes()).unwrap();
        zip_out.finish().unwrap();
    }
    std::fs::write(&dest, out.into_inner()).unwrap();
    dest
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
fn create_theme_is_null_after_protection() {
    let path = create_docx("empty");
    let tree = inspect(&path);
    assert!(tree["theme"].is_null());
    let keys = object_keys(&tree);
    let prot = keys.iter().position(|k| *k == "protection").unwrap();
    assert_eq!(keys[prot + 1], "theme");
}

#[test]
fn inspect_theme_slots_and_order() {
    let path = inject_theme(&create_docx("inspect"));
    let tree = inspect(&path);
    let theme = &tree["theme"];
    assert_eq!(object_keys(theme), ["name", "colors", "fonts"]);
    assert_eq!(theme["name"], "Office Theme");
    assert_eq!(object_keys(&theme["colors"]), COLOR_KEYS);
    assert_eq!(object_keys(&theme["fonts"]), FONT_KEYS);
    assert_eq!(theme["colors"]["accent1"], "4F81BD");
    assert_eq!(theme["colors"]["dk1"], "000000");
    assert_eq!(theme["fonts"]["majorLatin"], "Calibri Light");
    assert_eq!(theme["fonts"]["minorLatin"], "Calibri");
    assert_eq!(theme["fonts"]["majorEa"], "");
}

#[test]
fn set_theme_colors_and_fonts() {
    let src = inject_theme(&create_docx("apply-src"));
    let out = tmp("apply");
    apply(
        &src,
        &out,
        json!([
            { "op": "setThemeColors", "id": "docx:document", "accent1": "FF0000" },
            { "op": "setThemeFonts", "id": "docx:document", "majorLatin": "Arial" }
        ]),
    )
    .unwrap();
    let tree = inspect(&out);
    assert_eq!(tree["theme"]["colors"]["accent1"], "FF0000");
    assert_eq!(tree["theme"]["colors"]["accent2"], "C0504D");
    assert_eq!(tree["theme"]["fonts"]["majorLatin"], "Arial");
    assert_eq!(tree["theme"]["fonts"]["minorLatin"], "Calibri");
    let xml = part(&out, "word/theme/theme1.xml");
    assert!(xml.contains(r#"<a:accent1><a:srgbClr val="FF0000"/>"#));
    assert!(xml.contains(r#"typeface="Arial""#));
    assert!(xml.contains(r#"panose="020F0302020204030204""#));
    assert!(xml.contains(r#"<a:fmtScheme name="Office">"#));
}

#[test]
fn missing_theme_is_id_not_found() {
    let src = create_docx("missing");
    let out = tmp("missing-out");
    let err = apply(
        &src,
        &out,
        json!([{ "op": "setThemeColors", "id": "docx:document", "accent1": "FF0000" }]),
    )
    .unwrap_err();
    assert_eq!(err_code(err), ErrorCode::IdNotFound);
    assert!(!out.exists());
}

#[test]
fn reject_bad_id_and_empty_slots() {
    let src = inject_theme(&create_docx("bad"));
    let out = tmp("bad-out");
    let err = apply(
        &src,
        &out,
        json!([{ "op": "setThemeColors", "id": "para:index:1", "accent1": "FF0000" }]),
    )
    .unwrap_err();
    assert_eq!(err_code(err), ErrorCode::InvalidId);
    let err = apply(
        &src,
        &out,
        json!([{ "op": "setThemeColors", "id": "docx:document" }]),
    )
    .unwrap_err();
    assert_eq!(err_code(err), ErrorCode::InvalidField);
    let err = apply(
        &src,
        &out,
        json!([{ "op": "setThemeFonts", "id": "docx:document", "majorLatin": "" }]),
    )
    .unwrap_err();
    assert_eq!(err_code(err), ErrorCode::InvalidField);
}

#[test]
fn xlsx_rejects_theme_ops() {
    let path = tmp_dir().join(format!(
        "xlsx-reject-{}.xlsx",
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "format": "xlsx",
            "workbook": { "sheets": [{ "name": "Sheet1", "cells": [{ "ref": "A1", "value": 1 }] }] }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let out = tmp("xlsx-out");
    let err = apply(
        &path,
        &out,
        json!([{ "op": "setThemeColors", "id": "xlsx:workbook", "accent1": "FF0000" }]),
    )
    .unwrap_err();
    assert_eq!(err_code(err), ErrorCode::InvalidField);
}
