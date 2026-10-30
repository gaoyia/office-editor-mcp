//! 同一批里增删表、行、列之后，原来的 table / row / 格子 id 仍指向开始时的对象。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join("ooxp-docx-table-pin");
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(format!("{tag}-{n}.docx"))
}

fn create(tag: &str, document: Value) -> PathBuf {
    let path = tmp(tag);
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "format": "docx",
        "document": document
    })
    .to_string())
    .unwrap();
    path
}

fn apply(src: &Path, ops: Value) -> PathBuf {
    let out = tmp("out");
    run(&json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": out.to_string_lossy(),
        "ops": ops
    })
    .to_string())
    .unwrap();
    out
}

fn apply_err(src: &Path, ops: Value) -> ooxp::Error {
    let out = tmp("err");
    run(&json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": out.to_string_lossy(),
        "ops": ops
    })
    .to_string())
    .unwrap_err()
}

fn document_xml(path: &Path) -> String {
    let file = std::fs::File::open(path).unwrap();
    let mut zip = zip::ZipArchive::new(file).unwrap();
    let mut part = zip.by_name("word/document.xml").unwrap();
    let mut s = String::new();
    std::io::Read::read_to_string(&mut part, &mut s).unwrap();
    s
}

fn part_xml(path: &Path, name: &str) -> String {
    let file = std::fs::File::open(path).unwrap();
    let mut zip = zip::ZipArchive::new(file).unwrap();
    let mut part = zip.by_name(name).unwrap();
    let mut s = String::new();
    std::io::Read::read_to_string(&mut part, &mut s).unwrap();
    s
}

fn cell_with(xml: &str, text: &str) -> String {
    let needle = format!("<w:t>{text}</w:t>");
    let at = xml.find(&needle).unwrap_or_else(|| panic!("没有 {text}"));
    let start = xml[..at].rfind("<w:tc").unwrap();
    let end = xml[at..].find("</w:tc>").unwrap() + at;
    xml[start..=end].to_string()
}

#[test]
fn insert_row_then_original_cell() {
    let src = create(
        "rows",
        json!({
            "paragraphs": [{ "text": "前" }],
            "tables": [{ "cells": [["R1"], ["R2"], ["R3"]] }]
        }),
    );
    let out = apply(
        &src,
        json!([
            { "op": "insertRow", "id": "table:1/row:1" },
            { "op": "setCellFormat", "id": "table:1/r3c1", "fill": "00FF00" }
        ]),
    );
    let xml = document_xml(&out);
    let cell = cell_with(&xml, "R3");
    assert!(cell.contains("00FF00"), "{cell}");
    assert!(!xml.contains("ooxp:pin"), "{xml}");
}

#[test]
fn delete_col_then_original_cell() {
    let src = create(
        "cols",
        json!({
            "paragraphs": [{ "text": "前" }],
            "tables": [{ "cells": [["A", "B", "C"]] }]
        }),
    );
    let out = apply(
        &src,
        json!([
            { "op": "deleteCol", "id": "table:1", "col": 1 },
            { "op": "setCellFormat", "id": "table:1/r1c3", "fill": "0000FF" }
        ]),
    );
    let xml = document_xml(&out);
    assert!(!xml.contains("<w:t>A</w:t>"), "{xml}");
    let cell = cell_with(&xml, "C");
    assert!(cell.contains("0000FF"), "{cell}");
}

#[test]
fn delete_row_then_missing() {
    let src = create(
        "delrow",
        json!({
            "paragraphs": [{ "text": "前" }],
            "tables": [{ "cells": [["R1"], ["R2"], ["R3"]] }]
        }),
    );
    let err = apply_err(
        &src,
        json!([
            { "op": "deleteRow", "id": "table:1/row:2" },
            { "op": "setCellFormat", "id": "table:1/r2c1", "fill": "FF0000" }
        ]),
    );
    assert_eq!(err.code(), ErrorCode::IdNotFound);
    let out = apply(
        &src,
        json!([
            { "op": "deleteRow", "id": "table:1/row:2" },
            { "op": "setCellFormat", "id": "table:1/r3c1", "fill": "00FF00" }
        ]),
    );
    let cell = cell_with(&document_xml(&out), "R3");
    assert!(cell.contains("00FF00"), "{cell}");
}

#[test]
fn insert_table_then_original_table() {
    let src = create(
        "tables",
        json!({
            "paragraphs": [{ "text": "前" }],
            "tables": [
                { "cells": [["T1"]] },
                { "cells": [["T2"]] }
            ]
        }),
    );
    let out = apply(
        &src,
        json!([
            { "op": "insertTable", "id": "table:1", "rows": 1, "cols": 1, "cells": [["NEW"]] },
            { "op": "setCellFormat", "id": "table:2/r1c1", "fill": "FF00FF" }
        ]),
    );
    let xml = document_xml(&out);
    let cell = cell_with(&xml, "T2");
    assert!(cell.contains("FF00FF"), "{cell}");
    let new_cell = cell_with(&xml, "NEW");
    assert!(!new_cell.contains("FF00FF"), "{new_cell}");
}

#[test]
fn delete_table_then_missing() {
    let src = create(
        "deltbl",
        json!({
            "paragraphs": [{ "text": "前" }],
            "tables": [
                { "cells": [["T1"]] },
                { "cells": [["T2"]] }
            ]
        }),
    );
    let err = apply_err(
        &src,
        json!([
            { "op": "deleteTable", "id": "table:1" },
            { "op": "setCellFormat", "id": "table:1/r1c1", "fill": "FF0000" }
        ]),
    );
    assert_eq!(err.code(), ErrorCode::IdNotFound);
}

#[test]
fn insert_col_then_original_cell() {
    let src = create(
        "inscol",
        json!({
            "paragraphs": [{ "text": "前" }],
            "tables": [{ "cells": [["A", "B", "C"]] }]
        }),
    );
    let out = apply(
        &src,
        json!([
            { "op": "insertCol", "id": "table:1", "after": 1 },
            { "op": "setCellFormat", "id": "table:1/r1c3", "fill": "0000FF" }
        ]),
    );
    let cell = cell_with(&document_xml(&out), "C");
    assert!(cell.contains("0000FF"), "{cell}");
}

#[test]
fn delete_col_then_merge_original_columns() {
    let src = create(
        "merge",
        json!({
            "paragraphs": [{ "text": "前" }],
            "tables": [{ "cells": [["A", "B", "C"]] }]
        }),
    );
    let out = apply(
        &src,
        json!([
            { "op": "deleteCol", "id": "table:1", "col": 1 },
            {
                "op": "mergeCells",
                "id": "table:1",
                "startRow": 1,
                "startCol": 2,
                "endRow": 1,
                "endCol": 3
            }
        ]),
    );
    let xml = document_xml(&out);
    assert!(!xml.contains("<w:t>A</w:t>"), "{xml}");
    assert!(
        xml.contains("w:val=\"2\"") || xml.contains("w:val=\"2\""),
        "{xml}"
    );
    assert!(
        xml.contains("<w:t>B</w:t>") && xml.contains("<w:t>C</w:t>"),
        "{xml}"
    );
}

#[test]
fn header_table_then_original_cell() {
    let src = create(
        "hdr",
        json!({
            "paragraphs": [{ "text": "正文" }],
            "headers": [{
                "type": "default",
                "tables": [{ "cells": [["H1"], ["H2"], ["H3"]] }]
            }]
        }),
    );
    let tree = {
        let RunOutput::Inspect(tree) = run(&json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": src.to_string_lossy(),
            "scope": { "headerFooter": true, "tables": true, "from": 1, "maxParas": 5 }
        })
        .to_string())
        .unwrap() else {
            panic!("inspect")
        };
        tree
    };
    let id = tree["headers"][0]["tables"][0]["id"].as_str().unwrap();
    assert!(id.starts_with("header:"), "{id}");
    let out = apply(
        &src,
        json!([
            { "op": "insertRow", "id": format!("{id}/row:1") },
            { "op": "setCellFormat", "id": format!("{id}/r3c1"), "fill": "00FF00" }
        ]),
    );
    let xml = part_xml(&out, "word/header1.xml");
    let cell = cell_with(&xml, "H3");
    assert!(cell.contains("00FF00"), "{cell}");
}

#[test]
fn box_table_then_original_cell() {
    let src = create(
        "box",
        json!({
            "paragraphs": [{
                "text": "正文",
                "textBoxes": [{ "text": "框" }]
            }]
        }),
    );
    let RunOutput::Inspect(tree) = run(&json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": src.to_string_lossy(),
        "scope": { "from": 1, "maxParas": 5, "textBoxes": true }
    })
    .to_string())
    .unwrap() else {
        panic!("inspect")
    };
    let box_id = tree["paragraphs"][0]["textBoxes"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let made = apply(
        &src,
        json!([{
            "op": "insertTable",
            "id": box_id,
            "rows": 3,
            "cols": 1,
            "cells": [["B1"], ["B2"], ["B3"]]
        }]),
    );
    let RunOutput::Inspect(listed) = run(&json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": made.to_string_lossy(),
        "scope": { "from": 1, "maxParas": 5, "textBoxes": true, "tables": true }
    })
    .to_string())
    .unwrap() else {
        panic!("inspect")
    };
    let tid = listed["paragraphs"][0]["textBoxes"][0]["tables"][0]["id"]
        .as_str()
        .unwrap();
    let out = apply(
        &made,
        json!([
            { "op": "insertRow", "id": format!("{tid}/row:1") },
            { "op": "setCellFormat", "id": format!("{tid}/r3c1"), "fill": "00FF00" }
        ]),
    );
    let cell = cell_with(&document_xml(&out), "B3");
    assert!(cell.contains("00FF00"), "{cell}");
}
