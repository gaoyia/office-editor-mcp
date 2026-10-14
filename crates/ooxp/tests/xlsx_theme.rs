//! xlsx 主题：inspect 根上 `theme`，setThemeColors / setThemeFonts 等。

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

const FONT_KEYS: [&str; 9] = [
    "name",
    "majorLatin",
    "minorLatin",
    "majorEa",
    "minorEa",
    "majorCs",
    "minorCs",
    "majorScripts",
    "minorScripts",
];

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-xlsx-theme");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn tmp(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}.xlsx"))
}

fn create_xlsx(tag: &str) -> PathBuf {
    let path = tmp(tag);
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "format": "xlsx",
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
            if n == "xl/_rels/workbook.xml.rels" {
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
                    r#"<Override PartName="/xl/theme/theme1.xml" ContentType="application/vnd.openxmlformats-officedocument.theme+xml"/></Types>"#,
                );
                data = s.into_bytes();
            }
            zip_out.start_file(&n, opt).unwrap();
            zip_out.write_all(&data).unwrap();
        }
        zip_out.start_file("xl/theme/theme1.xml", opt).unwrap();
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
    let path = create_xlsx("empty");
    let tree = inspect(&path);
    assert!(tree["theme"].is_null());
    assert!(tree["themeOverride"].is_null());
    let keys = object_keys(&tree);
    let prot = keys
        .iter()
        .position(|k| *k == "workbookProtection")
        .unwrap();
    assert_eq!(keys[prot + 1], "theme");
    assert_eq!(keys[prot + 2], "themeOverride");
}

#[test]
fn inspect_theme_slots_and_order() {
    let path = inject_theme(&create_xlsx("inspect"));
    let tree = inspect(&path);
    let theme = &tree["theme"];
    assert_eq!(
        object_keys(theme),
        [
            "name",
            "colors",
            "fonts",
            "fmt",
            "extraSchemes",
            "objectDefaults"
        ]
    );
    assert_eq!(theme["name"], "Office Theme");
    assert_eq!(object_keys(&theme["colors"]), COLOR_KEYS);
    assert_eq!(object_keys(&theme["fonts"]), FONT_KEYS);
    assert_eq!(
        object_keys(&theme["fmt"]),
        ["name", "fills", "lines", "effects", "bgFills"]
    );
    assert_eq!(theme["fmt"]["name"], "Office");
    assert_eq!(theme["fmt"]["fills"][0]["kind"], "solid");
    assert_eq!(theme["fmt"]["lines"][0]["w"], 9525);
    assert!(tree["themeOverride"].is_null());
    assert_eq!(theme["colors"]["accent1"], "4F81BD");
    assert_eq!(theme["fonts"]["majorLatin"], "Calibri Light");
    assert_eq!(theme["extraSchemes"], json!([]));
    assert!(theme["objectDefaults"].is_null());
    assert_eq!(theme["fonts"]["majorScripts"], json!([]));
}

#[test]
fn set_theme_colors_and_fonts() {
    let src = inject_theme(&create_xlsx("apply-src"));
    let out = tmp("apply");
    apply(
        &src,
        &out,
        json!([
            { "op": "setThemeColors", "id": "xlsx:workbook", "accent1": "FF0000" },
            { "op": "setThemeFonts", "id": "xlsx:workbook", "majorLatin": "Arial" }
        ]),
    )
    .unwrap();
    let tree = inspect(&out);
    assert_eq!(tree["theme"]["colors"]["accent1"], "FF0000");
    assert_eq!(tree["theme"]["colors"]["accent2"], "C0504D");
    assert_eq!(tree["theme"]["fonts"]["majorLatin"], "Arial");
    assert_eq!(tree["theme"]["fonts"]["minorLatin"], "Calibri");
    let xml = part(&out, "xl/theme/theme1.xml");
    assert!(xml.contains(r#"<a:accent1><a:srgbClr val="FF0000"/>"#));
    assert!(xml.contains(r#"typeface="Arial""#));
    assert!(xml.contains(r#"panose="020F0302020204030204""#));
}

#[test]
fn missing_theme_is_id_not_found() {
    let src = create_xlsx("missing");
    let out = tmp("missing-out");
    let err = apply(
        &src,
        &out,
        json!([{ "op": "setThemeColors", "id": "xlsx:workbook", "accent1": "FF0000" }]),
    )
    .unwrap_err();
    assert_eq!(err_code(err), ErrorCode::IdNotFound);
    let themed = inject_theme(&create_xlsx("bad-id"));
    let err = apply(
        &themed,
        &out,
        json!([{ "op": "setThemeColors", "id": "sheet:Sheet1", "accent1": "FF0000" }]),
    )
    .unwrap_err();
    assert_eq!(err_code(err), ErrorCode::InvalidId);
    let err = apply(
        &themed,
        &out,
        json!([{ "op": "setThemeColors", "id": "xlsx:workbook" }]),
    )
    .unwrap_err();
    assert_eq!(err_code(err), ErrorCode::InvalidField);
}

#[test]
fn set_theme_fmt_line_width() {
    let src = inject_theme(&create_xlsx("fmt-src"));
    let out = tmp("fmt");
    apply(
        &src,
        &out,
        json!([{
            "op": "setThemeFmt",
            "id": "xlsx:workbook",
            "line": { "index": 0, "w": 38100 },
            "fill": { "index": 0, "kind": "grad" }
        }]),
    )
    .unwrap();
    let tree = inspect(&out);
    assert_eq!(tree["theme"]["fmt"]["lines"][0]["w"], 38100);
    assert_eq!(tree["theme"]["fmt"]["fills"][0]["kind"], "grad");
}

#[test]
fn set_theme_override_and_clear() {
    let src = inject_theme(&create_xlsx("ov-src"));
    let out = tmp("ov");
    apply(
        &src,
        &out,
        json!([{ "op": "setThemeOverride", "id": "xlsx:workbook", "accent1": "FF0000" }]),
    )
    .unwrap();
    let tree = inspect(&out);
    assert_eq!(tree["themeOverride"]["colors"]["accent1"], "FF0000");
    let xml = part(&out, "xl/theme/themeOverride1.xml");
    assert!(xml.contains("themeOverride"));
    assert!(xml.contains(r#"val="FF0000""#));
    let cleared = tmp("ov-clear");
    apply(
        &out,
        &cleared,
        json!([{ "op": "setThemeOverride", "id": "xlsx:workbook", "enabled": false }]),
    )
    .unwrap();
    let tree = inspect(&cleared);
    assert!(tree["themeOverride"].is_null());
}

#[test]
fn create_writes_theme_and_keeps_default_without() {
    let with_theme = tmp("create-theme");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": with_theme.to_string_lossy(),
            "format": "xlsx",
            "workbook": {
                "sheets": [{ "name": "Sheet1", "cells": [{ "ref": "A1", "value": 1 }] }],
                "theme": {
                    "name": "Office Theme",
                    "colors": { "accent1": "FF0000" },
                    "fonts": { "majorLatin": "Arial" }
                }
            }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let tree = inspect(&with_theme);
    assert_eq!(tree["theme"]["name"], "Office Theme");
    assert_eq!(tree["theme"]["colors"]["accent1"], "FF0000");
    assert_eq!(tree["theme"]["fonts"]["majorLatin"], "Arial");
    assert_eq!(tree["theme"]["fmt"]["name"], "Office");
    assert!(tree["theme"]["objectDefaults"].is_null());
    let bare = create_xlsx("still-null");
    assert!(inspect(&bare)["theme"].is_null());
}

#[test]
fn set_theme_names_scripts_and_extra() {
    let src = inject_theme(&create_xlsx("remain-src"));
    let out = tmp("remain");
    apply(
        &src,
        &out,
        json!([
            {
                "op": "setThemeColors",
                "id": "xlsx:workbook",
                "name": "Custom",
                "themeName": "Custom Theme"
            },
            {
                "op": "setThemeFonts",
                "id": "xlsx:workbook",
                "name": "CustomFonts",
                "majorScripts": [{ "script": "Jpan", "typeface": "Yu Mincho" }]
            },
            {
                "op": "setThemeFmt",
                "id": "xlsx:workbook",
                "name": "CustomFmt"
            },
            {
                "op": "setThemeExtraSchemes",
                "id": "xlsx:workbook",
                "extraSchemes": [{
                    "name": "Extra",
                    "colors": { "accent1": "0F6FC6" }
                }]
            }
        ]),
    )
    .unwrap();
    let tree = inspect(&out);
    assert_eq!(tree["theme"]["name"], "Custom Theme");
    assert_eq!(tree["theme"]["colors"]["name"], "Custom");
    assert_eq!(tree["theme"]["fonts"]["name"], "CustomFonts");
    assert_eq!(tree["theme"]["fmt"]["name"], "CustomFmt");
    assert_eq!(tree["theme"]["fonts"]["majorScripts"][0]["script"], "Jpan");
    assert_eq!(tree["theme"]["extraSchemes"][0]["name"], "Extra");
    assert_eq!(
        tree["theme"]["extraSchemes"][0]["colors"]["accent1"],
        "0F6FC6"
    );
    let xml = part(&out, "xl/theme/theme1.xml");
    assert!(xml.contains(r#"name="Custom Theme""#));
    assert!(xml.contains(r#"<a:font script="Jpan" typeface="Yu Mincho"/>"#));
    let cleared = tmp("remain-clear");
    apply(
        &out,
        &cleared,
        json!([
            { "op": "setThemeFonts", "id": "xlsx:workbook", "majorScripts": [] },
            { "op": "setThemeExtraSchemes", "id": "xlsx:workbook", "extraSchemes": null }
        ]),
    )
    .unwrap();
    let tree = inspect(&cleared);
    assert_eq!(tree["theme"]["fonts"]["majorScripts"], json!([]));
    assert_eq!(tree["theme"]["extraSchemes"], json!([]));
}

#[test]
fn set_theme_object_defaults_round_trip() {
    let src = inject_theme(&create_xlsx("od-src"));
    let out = tmp("od");
    apply(
        &src,
        &out,
        json!([{
            "op": "setThemeObjectDefaults",
            "id": "xlsx:workbook",
            "objectDefaults": {
                "sp": {
                    "style": { "lnRef": 1, "fontRef": "minor" },
                    "lnW": 38100
                }
            }
        }]),
    )
    .unwrap();
    let tree = inspect(&out);
    let od = &tree["theme"]["objectDefaults"];
    assert_eq!(object_keys(od), ["sp", "ln", "tx"]);
    assert_eq!(od["sp"]["lnW"], 38100);
    assert_eq!(od["sp"]["style"]["lnRef"], 1);
    assert_eq!(od["sp"]["style"]["fillRef"], 0);
    assert_eq!(od["sp"]["style"]["fontRef"], "minor");
    assert!(od["ln"].is_null());
    let xml = part(&out, "xl/theme/theme1.xml");
    assert!(xml.contains("<a:objectDefaults>"));
    assert!(xml.contains(r#"<a:lnRef idx="1">"#));
    assert!(xml.contains(r#"w="38100""#));
    let cleared = tmp("od-clear");
    apply(
        &out,
        &cleared,
        json!([{
            "op": "setThemeObjectDefaults",
            "id": "xlsx:workbook",
            "objectDefaults": null
        }]),
    )
    .unwrap();
    assert!(inspect(&cleared)["theme"]["objectDefaults"].is_null());
}

#[test]
fn set_theme_object_defaults_rejects() {
    let src = create_xlsx("od-missing");
    let out = tmp("od-missing-out");
    let err = apply(
        &src,
        &out,
        json!([{
            "op": "setThemeObjectDefaults",
            "id": "xlsx:workbook",
            "objectDefaults": { "sp": { "lnW": 1 } }
        }]),
    )
    .unwrap_err();
    assert_eq!(err_code(err), ErrorCode::IdNotFound);
    assert!(!out.exists());
    let themed = inject_theme(&create_xlsx("od-bad"));
    let err = apply(
        &themed,
        &out,
        json!([{
            "op": "setThemeObjectDefaults",
            "id": "sheet:Sheet1",
            "objectDefaults": { "sp": { "lnW": 1 } }
        }]),
    )
    .unwrap_err();
    assert_eq!(err_code(err), ErrorCode::InvalidId);
    let err = apply(
        &themed,
        &out,
        json!([{ "op": "setThemeObjectDefaults", "id": "xlsx:workbook" }]),
    )
    .unwrap_err();
    assert_eq!(err_code(err), ErrorCode::InvalidField);
    let err = apply(
        &themed,
        &out,
        json!([{
            "op": "setThemeObjectDefaults",
            "id": "xlsx:workbook",
            "objectDefaults": { "sp": { "style": { "fontRef": "latin" } } }
        }]),
    )
    .unwrap_err();
    assert_eq!(err_code(err), ErrorCode::InvalidField);
}
