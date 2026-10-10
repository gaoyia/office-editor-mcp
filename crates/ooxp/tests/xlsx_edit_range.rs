//! 工作表允许编辑区域与 .xlsm 容器。

use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-xlsx-edit-range");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn tmp(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}.xlsx"))
}

fn tmp_xlsm(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}.xlsm"))
}

fn create_book(tag: &str) -> PathBuf {
    let path = tmp(tag);
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
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
            "ops": ops
        })
        .to_string(),
        None,
    )
}

fn inspect(path: &Path) -> Value {
    inspect_with(path, json!({}))
}

fn inspect_with(path: &Path, scope: Value) -> Value {
    let RunOutput::Inspect(tree) = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": path.to_string_lossy(),
            "scope": scope
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
    v.as_object().unwrap().keys().map(String::as_str).collect()
}

fn part_text(path: &Path, name: &str) -> String {
    let f = std::fs::File::open(path).unwrap();
    let mut zip = zip::ZipArchive::new(f).unwrap();
    let mut zf = zip.by_name(name).unwrap();
    let mut out = String::new();
    zf.read_to_string(&mut out).unwrap();
    out
}

fn part_bytes(path: &Path, name: &str) -> Vec<u8> {
    let f = std::fs::File::open(path).unwrap();
    let mut zip = zip::ZipArchive::new(f).unwrap();
    let mut zf = zip.by_name(name).unwrap();
    let mut out = Vec::new();
    zf.read_to_end(&mut out).unwrap();
    out
}

fn rewrite_part(src: &Path, dst: &Path, name: &str, f: impl FnOnce(&str) -> String) {
    let bytes = std::fs::read(src).unwrap();
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
                let xml = String::from_utf8(data).unwrap();
                data = transform.take().expect("部件只应出现一次")(&xml).into_bytes();
            }
            zip_out.start_file(&n, opt).unwrap();
            zip_out.write_all(&data).unwrap();
        }
        zip_out.finish().unwrap();
    }
    std::fs::write(dst, out.into_inner()).unwrap();
}

fn add_zip_file(src: &Path, dst: &Path, name: &str, payload: &[u8]) {
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
            zip_out.start_file(&n, opt).unwrap();
            zip_out.write_all(&data).unwrap();
        }
        zip_out.start_file(name, opt).unwrap();
        zip_out.write_all(payload).unwrap();
        zip_out.finish().unwrap();
    }
    std::fs::write(dst, out.into_inner()).unwrap();
}

fn assert_fail(src: &Path, ops: Value, want: ErrorCode) {
    let out = tmp("fail");
    let err = apply(src, &out, ops).unwrap_err();
    assert_eq!(err.code(), want);
    assert!(!out.exists());
}

#[test]
fn count_is_zero_and_scope_lists_empty() {
    let path = create_book("bare");
    let tree = inspect(&path);
    assert_eq!(tree["sheets"][0]["protectedRangeCount"], 0);
    assert!(tree["sheets"][0].get("protectedRanges").is_none());
    assert!(tree.get("protectedRangeCount").is_none());
    let listed = inspect_with(&path, json!({ "protectedRanges": true }));
    assert_eq!(listed["sheets"][0]["protectedRanges"], json!([]));
}

#[test]
fn insert_inspect_set_delete() {
    let src = create_book("round");
    let inserted = tmp("ins");
    apply(
        &src,
        &inserted,
        json!([{
            "op": "insertProtectedRange",
            "id": "sheet:Sheet1",
            "name": "Range1",
            "sqref": "A1:C5"
        }]),
    )
    .unwrap();
    let tree = inspect_with(&inserted, json!({ "protectedRanges": true }));
    assert_eq!(tree["sheets"][0]["protectedRangeCount"], 1);
    let item = &tree["sheets"][0]["protectedRanges"][0];
    assert_eq!(
        object_keys(item),
        ["id", "index", "name", "sqref", "hasPassword"]
    );
    assert_eq!(item["id"], "sheet:Sheet1/editRange:1");
    assert_eq!(item["index"], 1);
    assert_eq!(item["name"], "Range1");
    assert_eq!(item["sqref"], "A1:C5");
    assert_eq!(item["hasPassword"], false);
    let xml = part_text(&inserted, "xl/worksheets/sheet1.xml");
    assert!(xml.contains("<protectedRanges>"), "{xml}");
    assert!(xml.contains(r#"sqref="A1:C5""#), "{xml}");
    assert!(xml.contains(r#"name="Range1""#), "{xml}");
    assert!(!xml.contains("algorithmName"), "{xml}");

    let renamed = tmp("set");
    apply(
        &inserted,
        &renamed,
        json!([{
            "op": "setProtectedRange",
            "id": "sheet:Sheet1/editRange:1",
            "name": "Range2",
            "sqref": ["B2:D6", "E10"]
        }]),
    )
    .unwrap();
    let after = inspect_with(&renamed, json!({ "protectedRanges": true }));
    let item = &after["sheets"][0]["protectedRanges"][0];
    assert_eq!(item["name"], "Range2");
    assert_eq!(item["sqref"], "B2:D6 E10");

    let deleted = tmp("del");
    apply(
        &renamed,
        &deleted,
        json!([{
            "op": "deleteProtectedRange",
            "id": "sheet:Sheet1/editRange:1"
        }]),
    )
    .unwrap();
    let gone = inspect_with(&deleted, json!({ "protectedRanges": true }));
    assert_eq!(gone["sheets"][0]["protectedRangeCount"], 0);
    assert_eq!(gone["sheets"][0]["protectedRanges"], json!([]));
    let xml = part_text(&deleted, "xl/worksheets/sheet1.xml");
    assert!(!xml.contains("protectedRanges"), "{xml}");
}

#[test]
fn password_writes_sha512_and_stays_out_of_tree() {
    let src = create_book("pw");
    let out = tmp("prot");
    apply(
        &src,
        &out,
        json!([{
            "op": "insertProtectedRange",
            "id": "sheet:Sheet1",
            "name": "Secret",
            "sqref": "A1",
            "password": "secret"
        }]),
    )
    .unwrap();
    let tree = inspect_with(&out, json!({ "protectedRanges": true }));
    let item = &tree["sheets"][0]["protectedRanges"][0];
    assert_eq!(item["hasPassword"], true);
    let dump = tree.to_string();
    assert!(!dump.contains("secret"), "{dump}");
    assert!(!dump.contains("hashValue"));
    let xml = part_text(&out, "xl/worksheets/sheet1.xml");
    assert!(xml.contains(r#"algorithmName="SHA-512""#), "{xml}");
    assert!(xml.contains(r#"spinCount="100000""#), "{xml}");
    assert!(xml.contains("hashValue="), "{xml}");
    assert!(xml.contains("saltValue="), "{xml}");
    assert!(!xml.contains(r#" password=""#), "{xml}");

    assert_fail(
        &out,
        json!([{
            "op": "setProtectedRange",
            "id": "sheet:Sheet1/editRange:1",
            "name": "Other"
        }]),
        ErrorCode::NotImplemented,
    );
    assert_fail(
        &out,
        json!([{
            "op": "setProtectedRange",
            "id": "sheet:Sheet1/editRange:1",
            "name": "Other",
            "password": "wrong"
        }]),
        ErrorCode::InvalidField,
    );

    let renamed = tmp("pw-set");
    apply(
        &out,
        &renamed,
        json!([{
            "op": "setProtectedRange",
            "id": "sheet:Sheet1/editRange:1",
            "name": "Kept",
            "password": "secret"
        }]),
    )
    .unwrap();
    let after = inspect_with(&renamed, json!({ "protectedRanges": true }));
    assert_eq!(after["sheets"][0]["protectedRanges"][0]["name"], "Kept");
    assert_eq!(
        after["sheets"][0]["protectedRanges"][0]["hasPassword"],
        true
    );

    let deleted = tmp("pw-del");
    apply(
        &renamed,
        &deleted,
        json!([{
            "op": "deleteProtectedRange",
            "id": "sheet:Sheet1/editRange:1",
            "password": "secret"
        }]),
    )
    .unwrap();
    assert_eq!(inspect(&deleted)["sheets"][0]["protectedRangeCount"], 0);
}

#[test]
fn duplicate_name_and_chartsheet_and_missing_fields() {
    let src = create_book("err");
    let one = tmp("one");
    apply(
        &src,
        &one,
        json!([{
            "op": "insertProtectedRange",
            "id": "sheet:Sheet1",
            "name": "Range1",
            "sqref": "A1:C5"
        }]),
    )
    .unwrap();
    assert_fail(
        &one,
        json!([{
            "op": "insertProtectedRange",
            "id": "sheet:Sheet1",
            "name": "range1",
            "sqref": "B1"
        }]),
        ErrorCode::InvalidField,
    );
    assert_fail(
        &src,
        json!([{
            "op": "insertProtectedRange",
            "id": "sheet:Sheet1",
            "name": "   ",
            "sqref": "A1"
        }]),
        ErrorCode::InvalidField,
    );
    assert_fail(
        &one,
        json!([{
            "op": "setProtectedRange",
            "id": "sheet:Sheet1/editRange:1"
        }]),
        ErrorCode::InvalidField,
    );
    assert_fail(
        &one,
        json!([{
            "op": "insertProtectedRange",
            "id": "sheet:Sheet1/editRange:1",
            "name": "X",
            "sqref": "A1"
        }]),
        ErrorCode::InvalidId,
    );
    assert_fail(
        &one,
        json!([{
            "op": "setProtectedRange",
            "id": "sheet:Sheet1",
            "name": "X"
        }]),
        ErrorCode::InvalidId,
    );

    let with_cs = tmp("cs");
    apply(
        &src,
        &with_cs,
        json!([{
            "op": "insertChartSheet",
            "id": "xlsx:workbook",
            "name": "Chart1",
            "dataFrom": "A10",
            "type": "col",
            "title": "销量",
            "series": [{
                "name": "Q1",
                "categories": ["甲", "乙", "丙"],
                "values": [1, 2, 3]
            }]
        }]),
    )
    .unwrap();
    let listed_cs = inspect_with(&with_cs, json!({ "protectedRanges": true }));
    let cs = listed_cs["sheets"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["name"] == "Chart1")
        .unwrap();
    assert_eq!(cs["kind"], "chartsheet");
    assert!(cs.get("protectedRangeCount").is_none());
    assert!(cs.get("protectedRanges").is_none());
    assert_fail(
        &with_cs,
        json!([{
            "op": "insertProtectedRange",
            "id": "sheet:Chart1",
            "name": "Range1",
            "sqref": "A1"
        }]),
        ErrorCode::InvalidField,
    );

    let created = tmp("create-pr");
    let err_create = run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": created.to_string_lossy(),
            "workbook": {
                "sheets": [{
                    "name": "Sheet1",
                    "protectedRanges": [{ "name": "Range1", "sqref": "A1" }]
                }]
            }
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err_create.code(), ErrorCode::InvalidField);
}

#[test]
fn legacy_xor_is_not_implemented() {
    let src = create_book("xor-src");
    let inserted = tmp("ins");
    apply(
        &src,
        &inserted,
        json!([{
            "op": "insertProtectedRange",
            "id": "sheet:Sheet1",
            "name": "Range1",
            "sqref": "A1:C5"
        }]),
    )
    .unwrap();
    let hashed = tmp("xor");
    rewrite_part(&inserted, &hashed, "xl/worksheets/sheet1.xml", |xml| {
        xml.replace("<protectedRange ", r#"<protectedRange password="CA2E" "#)
    });
    let tree = inspect_with(&hashed, json!({ "protectedRanges": true }));
    assert_eq!(tree["sheets"][0]["protectedRanges"][0]["hasPassword"], true);
    assert_fail(
        &hashed,
        json!([{
            "op": "deleteProtectedRange",
            "id": "sheet:Sheet1/editRange:1",
            "password": "secret"
        }]),
        ErrorCode::NotImplemented,
    );
}

#[test]
fn xlsm_create_keeps_macro_ct_and_vba_bytes() {
    let path = tmp_xlsm("new");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "format": "xlsm",
            "workbook": {
                "sheets": [{ "name": "Sheet1", "cells": [{ "ref": "A1", "value": 1 }] }]
            }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let tree = inspect(&path);
    assert_eq!(tree["format"], "xlsx");
    let ct = part_text(&path, "[Content_Types].xml");
    assert!(
        ct.contains("application/vnd.ms-excel.sheet.macroEnabled.main+xml"),
        "{ct}"
    );
    assert!(!ct.contains("spreadsheetml.sheet.main+xml"), "{ct}");

    let marker = b"vba-sentinel-bytes";
    let injected = tmp_xlsm("vba");
    add_zip_file(&path, &injected, "xl/vbaProject.bin", marker);
    let applied = tmp_xlsm("applied");
    apply(
        &injected,
        &applied,
        json!([{
            "op": "insertProtectedRange",
            "id": "sheet:Sheet1",
            "name": "Range1",
            "sqref": "A1:C5"
        }]),
    )
    .unwrap();
    assert_eq!(part_bytes(&applied, "xl/vbaProject.bin"), marker);
    let ct2 = part_text(&applied, "[Content_Types].xml");
    assert!(
        ct2.contains("application/vnd.ms-excel.sheet.macroEnabled.main+xml"),
        "{ct2}"
    );
}

#[test]
fn docx_and_pptx_reject_ops_and_scope() {
    let docx = tmp("d").with_extension("docx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": docx.to_string_lossy(),
            "document": { "paragraphs": [{ "text": "hi" }] }
        })
        .to_string(),
        None,
    )
    .unwrap();
    assert_fail(
        &docx,
        json!([{
            "op": "insertProtectedRange",
            "id": "sheet:Sheet1",
            "name": "Range1",
            "sqref": "A1"
        }]),
        ErrorCode::InvalidField,
    );
    let err_scope = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": docx.to_string_lossy(),
            "scope": { "protectedRanges": true, "from": 1, "maxParas": 10 }
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err_scope.code(), ErrorCode::InvalidField);

    let pptx = tmp("p").with_extension("pptx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": pptx.to_string_lossy(),
            "presentation": { "slides": [{ "shapes": [{ "text": "a" }] }] }
        })
        .to_string(),
        None,
    )
    .unwrap();
    assert_fail(
        &pptx,
        json!([{
            "op": "insertProtectedRange",
            "id": "sheet:Sheet1",
            "name": "Range1",
            "sqref": "A1"
        }]),
        ErrorCode::InvalidField,
    );
    let err_pptx_scope = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": pptx.to_string_lossy(),
            "scope": { "protectedRanges": true, "slide": 1, "maxShapes": 10 }
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err_pptx_scope.code(), ErrorCode::InvalidField);
}
