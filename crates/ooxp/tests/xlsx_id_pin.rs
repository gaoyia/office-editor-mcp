//! xlsx 位置序号：先删前面一个，再改原来的第 n 个，应改到原来那条。

use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, RunOutput};
use serde_json::{json, Value};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

const PNG_1X1: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
    0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08, 0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00,
    0x00, 0x03, 0x01, 0x01, 0x00, 0x18, 0xDD, 0x8D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E,
    0x44, 0xAE, 0x42, 0x60, 0x82,
];

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-xlsx-id-pin");
    std::fs::create_dir_all(&dir).unwrap();
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    dir.join(format!("{tag}-{n}.xlsx"))
}

fn create_book(tag: &str) -> PathBuf {
    let path = tmp(tag);
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "workbook": { "sheets": [{ "name": "Sheet1", "cells": [{ "ref": "A1", "value": 1 }] }] }
    })
    .to_string())
    .unwrap();
    path
}

fn apply(src: &Path, out: &Path, ops: Value) {
    run(&json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": out.to_string_lossy(),
        "ops": ops,
    })
    .to_string())
    .unwrap();
}

fn inspect(path: &Path) -> Value {
    let RunOutput::Inspect(tree) = run(&json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": path.to_string_lossy(),
        "scope": { "sheet": "Sheet1", "range": "A1:Z30", "pictures": true }
    })
    .to_string())
    .unwrap() else {
        panic!("inspect");
    };
    tree
}

fn inspect_book(path: &Path) -> Value {
    let RunOutput::Inspect(tree) = run(&json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": path.to_string_lossy()
    })
    .to_string())
    .unwrap() else {
        panic!("inspect");
    };
    tree
}

fn part_text(path: &Path, name: &str) -> String {
    let bytes = std::fs::read(path).unwrap();
    let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut file = zip.by_name(name).unwrap();
    let mut data = String::new();
    file.read_to_string(&mut data).unwrap();
    data
}

fn dv(formula: &str, sqref: &str) -> Value {
    json!({
        "op": "insertDataValidation",
        "id": "sheet:Sheet1",
        "sqref": sqref,
        "type": "list",
        "formula1": formula
    })
}

#[test]
fn delete_dv_then_original_third() {
    let src = create_book("dv");
    let mid = tmp("dv-mid");
    apply(
        &src,
        &mid,
        json!([dv("one", "A1"), dv("two", "B1"), dv("three", "C1")]),
    );
    let out = tmp("dv-out");
    apply(
        &mid,
        &out,
        json!([
            { "op": "deleteDataValidation", "id": "sheet:Sheet1/dv:1" },
            {
                "op": "setDataValidation",
                "id": "sheet:Sheet1/dv:3",
                "prompt": "第三条"
            }
        ]),
    );
    let sh = &inspect(&out)["sheets"][0];
    let rules = sh["dataValidations"].as_array().unwrap();
    assert_eq!(rules.len(), 2);
    let prompts: Vec<_> = rules
        .iter()
        .map(|r| r["prompt"].as_str().unwrap_or(""))
        .collect();
    assert!(
        prompts.contains(&"第三条"),
        "原来的第三条应被改到: {prompts:?}"
    );
    let xml = part_text(&out, "xl/worksheets/sheet1.xml");
    assert!(xml.contains("two"), "{xml}");
    assert!(xml.contains("three"), "{xml}");
    assert!(!xml.contains(">one<") && !xml.contains("\"one\""), "{xml}");
}

fn cf(sqref: &str, formula: &str) -> Value {
    json!({
        "op": "insertConditionalFormat",
        "id": "sheet:Sheet1",
        "sqref": sqref,
        "type": "cellIs",
        "operator": "greaterThan",
        "formulas": [formula]
    })
}

#[test]
fn delete_cf_then_original_second() {
    let src = create_book("cf");
    let mid = tmp("cf-mid");
    apply(&src, &mid, json!([cf("A1", "1"), cf("B1", "2")]));
    let out = tmp("cf-out");
    apply(
        &mid,
        &out,
        json!([
            { "op": "deleteConditionalFormat", "id": "sheet:Sheet1/cf:1" },
            {
                "op": "setConditionalFormat",
                "id": "sheet:Sheet1/cf:2",
                "sqref": "B1:B9"
            }
        ]),
    );
    let sh = &inspect(&out)["sheets"][0];
    let rules = sh["conditionalFormats"].as_array().unwrap();
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0]["sqref"], json!(["B1:B9"]));
    assert_eq!(rules[0]["formulas"], json!(["2"]));
}

#[test]
fn insert_then_edit_original_name() {
    let src = create_book("name");
    let mid = tmp("name-mid");
    apply(
        &src,
        &mid,
        json!([
            {
                "op": "insertDefinedName",
                "id": "xlsx:workbook",
                "name": "NameA",
                "formula": "Sheet1!$A$1"
            },
            {
                "op": "insertDefinedName",
                "id": "xlsx:workbook",
                "name": "NameB",
                "formula": "Sheet1!$B$1"
            }
        ]),
    );
    let out = tmp("name-out");
    apply(
        &mid,
        &out,
        json!([
            {
                "op": "insertDefinedName",
                "id": "xlsx:workbook",
                "name": "NameC",
                "formula": "Sheet1!$C$1"
            },
            {
                "op": "deleteDefinedName",
                "id": "name:1"
            },
            {
                "op": "setDefinedName",
                "id": "name:2",
                "formula": "Sheet1!$B$9"
            }
        ]),
    );
    let tree = inspect_book(&out);
    let names = tree["definedNames"].as_array().unwrap();
    let formula = |name: &str| {
        names
            .iter()
            .find(|n| n["name"] == name)
            .unwrap_or_else(|| panic!("missing {name}: {names:?}"))["formula"]
            .as_str()
            .unwrap()
            .to_string()
    };
    assert!(names.iter().all(|n| n["name"] != "NameA"));
    assert_eq!(formula("NameB"), "Sheet1!$B$9");
    assert_eq!(formula("NameC"), "Sheet1!$C$1");
}

#[test]
fn delete_link_then_original_third() {
    let src = create_book("link");
    let mid = tmp("link-mid");
    let link = |refer: &str, url: &str| {
        json!({
            "op": "insertHyperlink",
            "id": "sheet:Sheet1",
            "ref": refer,
            "url": url
        })
    };
    apply(
        &src,
        &mid,
        json!([
            link("A1", "https://example.com/a"),
            link("B1", "https://example.com/b"),
            link("C1", "https://example.com/c")
        ]),
    );
    let out = tmp("link-out");
    apply(
        &mid,
        &out,
        json!([
            { "op": "deleteHyperlink", "id": "sheet:Sheet1/link:1" },
            {
                "op": "setHyperlink",
                "id": "sheet:Sheet1/link:3",
                "tooltip": "第三条"
            }
        ]),
    );
    let xml = part_text(&out, "xl/worksheets/sheet1.xml");
    assert!(xml.contains(r#"ref="B1""#), "{xml}");
    assert!(
        xml.contains(r#"ref="C1""#) && xml.contains("tooltip=\"第三条\""),
        "{xml}"
    );
    assert!(!xml.contains(r#"ref="A1""#), "{xml}");
}

#[test]
fn delete_spark_then_original_third() {
    let src = create_book("spark");
    let mid = tmp("spark-mid");
    let spark = |refer: &str, formula: &str| {
        json!({
            "op": "insertSparkline",
            "id": "sheet:Sheet1",
            "ref": refer,
            "formula": formula
        })
    };
    apply(
        &src,
        &mid,
        json!([
            spark("A1", "B1:F1"),
            spark("A2", "B2:F2"),
            spark("A3", "B3:F3")
        ]),
    );
    let out = tmp("spark-out");
    apply(
        &mid,
        &out,
        json!([
            { "op": "deleteSparkline", "id": "sheet:Sheet1/spark:1" },
            {
                "op": "setSparkline",
                "id": "sheet:Sheet1/spark:3",
                "type": "column"
            }
        ]),
    );
    let xml = part_text(&out, "xl/worksheets/sheet1.xml");
    assert!(xml.contains("B2:F2"), "{xml}");
    assert!(xml.contains("B3:F3"), "{xml}");
    assert!(!xml.contains("B1:F1"), "{xml}");
    assert_eq!(xml.matches("type=\"column\"").count(), 1, "{xml}");
}

#[test]
fn delete_range_then_original_third() {
    let src = create_book("range");
    let mid = tmp("range-mid");
    let range = |name: &str, sqref: &str| {
        json!({
            "op": "insertProtectedRange",
            "id": "sheet:Sheet1",
            "name": name,
            "sqref": sqref
        })
    };
    apply(
        &src,
        &mid,
        json!([range("R1", "A1"), range("R2", "B1"), range("R3", "C1")]),
    );
    let out = tmp("range-out");
    apply(
        &mid,
        &out,
        json!([
            { "op": "deleteProtectedRange", "id": "sheet:Sheet1/editRange:1" },
            {
                "op": "setProtectedRange",
                "id": "sheet:Sheet1/editRange:3",
                "name": "R3kept"
            }
        ]),
    );
    let xml = part_text(&out, "xl/worksheets/sheet1.xml");
    assert!(xml.contains("R2"), "{xml}");
    assert!(xml.contains("R3kept"), "{xml}");
    assert!(!xml.contains("name=\"R1\""), "{xml}");
    assert!(!xml.contains("name=\"R3\""), "{xml}");
}

#[test]
fn delete_table_then_original_third() {
    let src = create_book("table");
    let mid = tmp("table-mid");
    let table = |refer: &str, name: &str| {
        json!({
            "op": "insertSheetTable",
            "id": "sheet:Sheet1",
            "ref": refer,
            "name": name,
            "columns": ["甲", "乙"]
        })
    };
    apply(
        &src,
        &mid,
        json!([
            table("A1:B2", "TableA"),
            table("A4:B5", "TableB"),
            table("A7:B8", "TableC")
        ]),
    );
    let out = tmp("table-out");
    apply(
        &mid,
        &out,
        json!([
            { "op": "deleteSheetTable", "id": "sheet:Sheet1/table:1" },
            {
                "op": "setSheetTable",
                "id": "sheet:Sheet1/table:3",
                "ref": "A7:B9"
            }
        ]),
    );
    let bytes = std::fs::read(&out).unwrap();
    let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut refs = Vec::new();
    for i in 0..zip.len() {
        let mut file = zip.by_index(i).unwrap();
        let name = file.name().to_string();
        if !name.contains("xl/tables/table") {
            continue;
        }
        let mut data = String::new();
        file.read_to_string(&mut data).unwrap();
        refs.push(data);
    }
    let joined = refs.join("\n");
    assert!(joined.contains("TableB"), "{joined}");
    assert!(joined.contains("TableC"), "{joined}");
    assert!(!joined.contains("TableA"), "{joined}");
    assert!(joined.contains("A7:B9"), "{joined}");
    assert!(joined.contains("A4:B5"), "{joined}");
}

#[test]
fn delete_pic_then_resize_original_third() {
    let src = create_book("pic");
    let png = tmp("pic").with_extension("png");
    std::fs::write(&png, PNG_1X1).unwrap();
    let mid = tmp("pic-mid");
    let pic = |from: &str, cx: i64| {
        json!({
            "op": "insertPicture",
            "id": "sheet:Sheet1",
            "file": png.to_string_lossy(),
            "from": from,
            "cx": cx,
            "cy": 100
        })
    };
    apply(
        &src,
        &mid,
        json!([pic("B2", 111), pic("C2", 222), pic("D2", 333)]),
    );
    strip_cnv_ids(&mid);
    let out = tmp("pic-out");
    apply(
        &mid,
        &out,
        json!([
            { "op": "deletePicture", "id": "sheet:Sheet1/pic:index:1" },
            {
                "op": "setPictureSize",
                "id": "sheet:Sheet1/pic:index:3",
                "cx": 333000,
                "cy": 100
            }
        ]),
    );
    let tree = inspect(&out);
    let pics = tree["sheets"][0]["pictures"].as_array().unwrap();
    assert_eq!(pics.len(), 2, "{pics:?}");
    let cx_of = |from: &str| {
        pics.iter()
            .find(|p| p["from"] == from)
            .unwrap_or_else(|| panic!("缺 {from}: {pics:?}"))["cx"]
            .as_i64()
            .unwrap()
    };
    assert_eq!(cx_of("C2"), 222);
    assert_eq!(cx_of("D2"), 333000);
}

fn strip_cnv_ids(path: &Path) {
    let bytes = std::fs::read(path).unwrap();
    let mut zip_in = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut out = Cursor::new(Vec::new());
    {
        let mut zip_out = ZipWriter::new(&mut out);
        let opt = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        for i in 0..zip_in.len() {
            let mut file = zip_in.by_index(i).unwrap();
            let name = file.name().to_string();
            let mut data = Vec::new();
            file.read_to_end(&mut data).unwrap();
            if name.contains("xl/drawings/") {
                let text = String::from_utf8(data).unwrap();
                let text = strip_cnv_id_attrs(&text);
                data = text.into_bytes();
            }
            zip_out.start_file(&name, opt).unwrap();
            zip_out.write_all(&data).unwrap();
        }
        zip_out.finish().unwrap();
    }
    std::fs::write(path, out.into_inner()).unwrap();
}

fn strip_cnv_id_attrs(xml: &str) -> String {
    let mut out = String::new();
    let mut rest = xml;
    while let Some(at) = rest.find("cNvPr") {
        out.push_str(&rest[..at + 5]);
        rest = &rest[at + 5..];
        if let Some(end) = rest.find('>') {
            let tag = &rest[..end];
            let tag = strip_id_attr(tag);
            out.push_str(&tag);
            out.push('>');
            rest = &rest[end + 1..];
        }
    }
    out.push_str(rest);
    out
}

fn strip_id_attr(tag: &str) -> String {
    let mut s = tag.to_string();
    if let Some(start) = s.find(" id=\"") {
        if let Some(rel_end) = s[start + 5..].find('"') {
            s.replace_range(start..start + 5 + rel_end + 1, "");
        }
    }
    s
}
