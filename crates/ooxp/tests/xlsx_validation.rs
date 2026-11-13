//! 数据验证：inspect 名单、insert / set / delete、公式当字符串。

use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::json;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-xlsx-validation");
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
            "sheets": [{
                "name": "Sheet1",
                "cells": [{ "ref": "A1", "value": 1 }]
            }]
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

fn inspect_sheet(path: &Path) -> serde_json::Value {
    let RunOutput::Inspect(tree) = run(&json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": path.to_string_lossy(),
        "scope": { "sheet": "Sheet1", "range": "A1:C3" }
    })
    .to_string())
    .unwrap() else {
        panic!("inspect 应返回树");
    };
    tree["sheets"][0].clone()
}

fn inspect_noscope(path: &Path) -> serde_json::Value {
    let RunOutput::Inspect(tree) = run(&json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": path.to_string_lossy(),
    })
    .to_string())
    .unwrap() else {
        panic!("inspect 应返回树");
    };
    tree["sheets"][0].clone()
}

fn rewrite_part(path: &Path, name: &str, next: &str) {
    let bytes = std::fs::read(path).unwrap();
    let mut zip_in = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut out = Cursor::new(Vec::new());
    {
        let mut zip_out = ZipWriter::new(&mut out);
        let opt = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        for i in 0..zip_in.len() {
            let mut file = zip_in.by_index(i).unwrap();
            let n = file.name().to_string();
            let mut data = Vec::new();
            file.read_to_end(&mut data).unwrap();
            if n == name {
                data = next.as_bytes().to_vec();
            }
            zip_out.start_file(&n, opt).unwrap();
            zip_out.write_all(&data).unwrap();
        }
        zip_out.finish().unwrap();
    }
    std::fs::write(path, out.into_inner()).unwrap();
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
fn inspect_empty_on_create() {
    let path = create_book("empty");
    let no = inspect_noscope(&path);
    assert_eq!(no["dataValidationCount"], json!(0));
    assert!(no.get("dataValidations").is_none());
    let sh = inspect_sheet(&path);
    assert_eq!(sh["dataValidationCount"], json!(0));
    assert_eq!(sh["dataValidations"], json!([]));
}

#[test]
fn insert_list_and_inspect() {
    let src = create_book("list");
    let out = tmp("list-out");
    apply(
        &src,
        &out,
        json!([{
            "op": "insertDataValidation",
            "id": "sheet:Sheet1",
            "sqref": "A1",
            "type": "list",
            "formula1": "1,2,3"
        }]),
    )
    .unwrap();
    let sh = inspect_sheet(&out);
    assert_eq!(sh["dataValidationCount"], json!(1));
    let dv = &sh["dataValidations"][0];
    assert_eq!(dv["id"], json!("sheet:Sheet1/dv:1"));
    assert_eq!(dv["sqref"], json!(["A1"]));
    assert_eq!(dv["type"], json!("list"));
    assert_eq!(dv["formula1"], json!("\"1,2,3\""));
    assert_eq!(dv["dropdown"], json!(true));
    assert_eq!(dv["allowBlank"], json!(true));
    assert_eq!(dv["items"], json!(["1", "2", "3"]));
    assert_eq!(dv["results"], json!([{ "ref": "A1", "ok": true }]));
    assert_eq!(object_keys(dv)[0..3], ["id", "sqref", "type"]);
    let xml = part_text(&out, "xl/worksheets/sheet1.xml");
    assert!(xml.contains("dataValidations"), "{xml}");
    assert!(xml.contains(r#"type="list""#), "{xml}");
    assert!(xml.contains("1,2,3"), "{xml}");
    assert!(!xml.contains("showDropDown"), "{xml}");
}

#[test]
fn multi_range_sqref_roundtrip() {
    let src = create_book("multi");
    let out = tmp("multi-out");
    apply(
        &src,
        &out,
        json!([{
            "op": "insertDataValidation",
            "id": "sheet:Sheet1",
            "sqref": ["A1", "C1"],
            "type": "list",
            "formula1": "1,2,3"
        }]),
    )
    .unwrap();
    let sh = inspect_sheet(&out);
    assert_eq!(sh["dataValidations"][0]["sqref"], json!(["A1", "C1"]));
    let xml = part_text(&out, "xl/worksheets/sheet1.xml");
    assert!(xml.contains(r#"sqref="A1 C1""#), "{xml}");
}

#[test]
fn set_and_delete() {
    let src = create_book("set");
    let mid = tmp("set-mid");
    apply(
        &src,
        &mid,
        json!([{
            "op": "insertDataValidation",
            "id": "sheet:Sheet1",
            "sqref": "A1",
            "type": "list",
            "formula1": "1,2,3"
        }]),
    )
    .unwrap();
    let out = tmp("set-out");
    apply(
        &mid,
        &out,
        json!([{
            "op": "setDataValidation",
            "id": "sheet:Sheet1/dv:1",
            "sqref": "A1:A3",
            "prompt": "选一个"
        }]),
    )
    .unwrap();
    let sh = inspect_sheet(&out);
    assert_eq!(sh["dataValidations"][0]["sqref"], json!(["A1:A3"]));
    assert_eq!(sh["dataValidations"][0]["prompt"], json!("选一个"));
    let gone = tmp("del-out");
    apply(
        &out,
        &gone,
        json!([{ "op": "deleteDataValidation", "id": "sheet:Sheet1/dv:1" }]),
    )
    .unwrap();
    let sh2 = inspect_sheet(&gone);
    assert_eq!(sh2["dataValidationCount"], json!(0));
    assert_eq!(sh2["dataValidations"], json!([]));
    let xml = part_text(&gone, "xl/worksheets/sheet1.xml");
    assert!(!xml.contains("dataValidations"), "{xml}");
}

#[test]
fn insert_missing_fields() {
    let src = create_book("miss");
    let err = apply(
        &src,
        &tmp("miss-out"),
        json!([{ "op": "insertDataValidation", "id": "sheet:Sheet1" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

#[test]
fn set_missing_fields() {
    let src = create_book("setmiss");
    let mid = tmp("setmiss-mid");
    apply(
        &src,
        &mid,
        json!([{
            "op": "insertDataValidation",
            "id": "sheet:Sheet1",
            "sqref": "A1",
            "type": "list",
            "formula1": "1,2,3"
        }]),
    )
    .unwrap();
    let err = apply(
        &mid,
        &tmp("setmiss-out"),
        json!([{ "op": "setDataValidation", "id": "sheet:Sheet1/dv:1" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

#[test]
fn delete_missing_id() {
    let src = create_book("noid");
    let err = apply(
        &src,
        &tmp("noid-out"),
        json!([{ "op": "deleteDataValidation", "id": "sheet:Sheet1/dv:1" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::IdNotFound);
}

#[test]
fn whole_needs_formula2_when_between() {
    let src = create_book("whole");
    let err = apply(
        &src,
        &tmp("whole-out"),
        json!([{
            "op": "insertDataValidation",
            "id": "sheet:Sheet1",
            "sqref": "A1",
            "type": "whole",
            "formula1": "0"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    apply(
        &src,
        &tmp("whole-ok"),
        json!([{
            "op": "insertDataValidation",
            "id": "sheet:Sheet1",
            "sqref": "A1",
            "type": "whole",
            "operator": "greaterThan",
            "formula1": "0"
        }]),
    )
    .unwrap();
}

#[test]
fn hide_dropdown() {
    let src = create_book("hide");
    let out = tmp("hide-out");
    apply(
        &src,
        &out,
        json!([{
            "op": "insertDataValidation",
            "id": "sheet:Sheet1",
            "sqref": "A1",
            "type": "list",
            "formula1": "1,2,3",
            "dropdown": false
        }]),
    )
    .unwrap();
    let sh = inspect_sheet(&out);
    assert_eq!(sh["dataValidations"][0]["dropdown"], json!(false));
    let xml = part_text(&out, "xl/worksheets/sheet1.xml");
    assert!(xml.contains(r#"showDropDown="1""#), "{xml}");
}

#[test]
fn prompt_newline_survives_another_cell_edit() {
    let src = create_book("prompt-nl");
    let with_dv = tmp("prompt-nl-dv");
    apply(
        &src,
        &with_dv,
        json!([{
            "op": "insertDataValidation",
            "id": "sheet:Sheet1",
            "sqref": "A1",
            "type": "list",
            "formula1": "1,2",
            "prompt": "KEEPME"
        }]),
    )
    .unwrap();
    let sheet = "xl/worksheets/sheet1.xml";
    let xml = part_text(&with_dv, sheet)
        .replace(r#"prompt="KEEPME""#, r#"prompt="line1&#10;line2&#9;x""#);
    assert!(xml.contains("&#10;"), "{xml}");
    rewrite_part(&with_dv, sheet, &xml);
    let out = tmp("prompt-nl-out");
    apply(
        &with_dv,
        &out,
        json!([{ "op": "setCell", "id": "cell:Sheet1!B1", "value": 2 }]),
    )
    .unwrap();
    let written = part_text(&out, sheet);
    assert!(
        written.contains(r#"prompt="line1&#10;line2&#9;x""#),
        "{written}"
    );
    assert!(!written.contains("prompt=\"line1\n"), "{written}");
}
