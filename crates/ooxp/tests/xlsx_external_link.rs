//! 工作簿外部引用：计数、名单、插删；不打开目标文件。

use std::io::{Cursor, Read, Write};
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
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "workbook": {
            "sheets": [{ "name": "Sheet1", "cells": [{ "ref": "A1", "value": 1 }] }]
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

fn inject_break_fixture(path: &Path) {
    use zip::write::SimpleFileOptions;
    use zip::{CompressionMethod, ZipArchive, ZipWriter};
    let bytes = std::fs::read(path).unwrap();
    let mut zin = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut buf = Cursor::new(Vec::new());
    {
        let mut zout = ZipWriter::new(&mut buf);
        let opt = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        for i in 0..zin.len() {
            let mut file = zin.by_index(i).unwrap();
            let name = file.name().to_string();
            let mut data = Vec::new();
            file.read_to_end(&mut data).unwrap();
            if name == "xl/worksheets/sheet1.xml" {
                let xml = String::from_utf8(data).unwrap();
                let xml = xml.replace(
                    r#"<c r="A1"><f>[1]Sheet1!A1</f></c>"#,
                    r#"<c r="A1"><f>[1]Sheet1!A1</f><v>11</v></c>"#,
                );
                data = xml.into_bytes();
            }
            zout.start_file(name, opt).unwrap();
            zout.write_all(&data).unwrap();
        }
        let chart = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart"><c:chart><c:plotArea><c:lineChart><c:ser><c:val><c:numRef><c:f>[1]Sheet1!A1:A2</c:f><c:numCache><c:pt idx="0"><c:v>11</c:v></c:pt></c:numCache></c:numRef></c:val><c:cat><c:numRef><c:f>[2]Sheet1!B1</c:f></c:numRef></c:cat></c:ser></c:lineChart></c:plotArea></c:chart></c:chartSpace>"#;
        zout.start_file("xl/charts/chart1.xml", opt).unwrap();
        zout.write_all(chart).unwrap();
        zout.finish().unwrap();
    }
    std::fs::write(path, buf.into_inner()).unwrap();
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
fn delete_breaks_formulas_and_renumbers() {
    let src = create_book("del");
    let linked = tmp("linked");
    apply(
        &src,
        &linked,
        json!([
            { "op": "insertExternalLink", "id": "xlsx:workbook", "path": "a.xlsx" },
            { "op": "insertExternalLink", "id": "xlsx:workbook", "path": "b.xlsx" },
            { "op": "setCell", "id": "cell:Sheet1!A1", "formula": "=[1]Sheet1!A1" },
            { "op": "setCell", "id": "cell:Sheet1!A2", "formula": "=\"[1]Sheet1!A1\"" },
            { "op": "setCell", "id": "cell:Sheet1!A3", "formula": "=[2]Sheet1!A1" },
            { "op": "setCell", "id": "cell:Sheet1!B1", "formula": "=[1]Sheet1!A1", "sharedRef": "B1:B2" },
            { "op": "setCell", "id": "cell:Sheet1!C1", "formula": "=[1]Sheet1!A1:A2", "arrayRef": "C1:C2" },
            { "op": "insertDefinedName", "id": "xlsx:workbook", "name": "ExtMix", "formula": "SUM([1]Sheet1!A1,[2]Sheet1!A1)" }
        ]),
    )
    .unwrap();
    inject_break_fixture(&linked);
    let out = tmp("broken");
    apply(
        &linked,
        &out,
        json!([{ "op": "deleteExternalLink", "id": "extlink:1" }]),
    )
    .unwrap();
    let sheet = part_text(&out, "xl/worksheets/sheet1.xml");
    assert!(sheet.contains(r#"<c r="A1"><v>11</v></c>"#), "{sheet}");
    assert!(sheet.contains("&quot;[1]Sheet1!A1&quot;"), "{sheet}");
    assert!(sheet.contains(">[1]Sheet1!A1</f>"), "{sheet}");
    assert!(!sheet.contains(">[2]Sheet1!A1</f>"), "{sheet}");
    assert!(!sheet.contains("t=\"shared\""), "{sheet}");
    assert!(!sheet.contains("t=\"array\""), "{sheet}");
    assert!(
        sheet.contains(r#"<c r="B2"><v>0</v></c>"#) || sheet.contains(r#"<c r="B2"><v>0</v>"#),
        "{sheet}"
    );
    let wb = part_text(&out, "xl/workbook.xml");
    assert!(
        wb.contains(">#REF!</definedName>") || wb.contains("#REF!"),
        "{wb}"
    );
    assert!(wb.contains("[1]Sheet1!A1"), "{wb}");
    assert!(!wb.contains("[2]"), "{wb}");
    let tree = inspect(&out, json!({ "externalLinks": true }));
    assert_eq!(tree["externalLinkCount"], 1);
    assert_eq!(tree["externalLinks"][0]["path"], "b.xlsx");
    let chart = part_text(&out, "xl/charts/chart1.xml");
    assert!(!chart.contains("[1]Sheet1!A1:A2"), "{chart}");
    assert!(chart.contains("<c:v>11</c:v>"), "{chart}");
    assert!(chart.contains("[1]Sheet1!B1"), "{chart}");
    assert!(!chart.contains("[2]Sheet1!B1"), "{chart}");
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
    let out = tmp("ten-out");
    apply(
        &with_f,
        &out,
        json!([{ "op": "deleteExternalLink", "id": "extlink:2" }]),
    )
    .unwrap();
    let sheet = part_text(&out, "xl/worksheets/sheet1.xml");
    assert!(sheet.contains(">[9]Sheet1!A1</f>"), "{sheet}");
    assert!(!sheet.contains(">[1]Sheet1!A1</f>"), "{sheet}");
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
fn create_empty_external_links_is_ok() {
    let path = tmp("create-el");
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "workbook": {
            "externalLinks": [],
            "sheets": [{ "name": "Sheet1" }]
        }
    })
    .to_string())
    .unwrap();
    let tree = inspect(&path, json!({ "externalLinks": true }));
    assert_eq!(tree["externalLinkCount"], 0);
    assert_eq!(tree["externalLinks"], json!([]));
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
