//! 工作表 / 工作簿保护：inspect 摘要，setProtection 开关无密码标记。

use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

static SEQ: AtomicU32 = AtomicU32::new(0);

const SHEET_PROT_KEYS: [&str; 17] = [
    "enforced",
    "hasPassword",
    "objects",
    "scenarios",
    "formatCells",
    "formatColumns",
    "formatRows",
    "insertColumns",
    "insertRows",
    "insertHyperlinks",
    "deleteColumns",
    "deleteRows",
    "selectLockedCells",
    "sort",
    "autoFilter",
    "pivotTables",
    "selectUnlockedCells",
];

fn default_sheet_protection() -> Value {
    json!({
        "enforced": false,
        "hasPassword": false,
        "objects": false,
        "scenarios": false,
        "formatCells": true,
        "formatColumns": true,
        "formatRows": true,
        "insertColumns": true,
        "insertRows": true,
        "insertHyperlinks": true,
        "deleteColumns": true,
        "deleteRows": true,
        "selectLockedCells": false,
        "sort": true,
        "autoFilter": true,
        "pivotTables": true,
        "selectUnlockedCells": false
    })
}

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-xlsx-protect");
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

fn apply(src: &Path, out: &Path, ops: Value) -> Result<RunOutput, ooxp::Error> {
    run(&json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": out.to_string_lossy(),
        "ops": ops
    })
    .to_string())
}

fn inspect(path: &Path) -> Value {
    let RunOutput::Inspect(tree) = run(&json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": path.to_string_lossy()
    })
    .to_string())
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

fn sheet_by_name<'a>(tree: &'a Value, name: &str) -> &'a Value {
    tree["sheets"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["name"] == name)
        .unwrap_or_else(|| panic!("找不到表 {name}"))
}

fn insert_chartsheet_op() -> Value {
    json!({
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
    })
}

#[test]
fn inspect_defaults_without_element() {
    let path = create_book("bare");
    let tree = inspect(&path);
    let wb = &tree["workbookProtection"];
    assert_eq!(
        object_keys(wb),
        [
            "lockStructure",
            "lockWindows",
            "lockRevision",
            "hasPassword"
        ]
    );
    assert_eq!(wb["lockStructure"], false);
    assert_eq!(wb["lockWindows"], false);
    assert_eq!(wb["lockRevision"], false);
    assert_eq!(wb["hasPassword"], false);
    let sp = &tree["sheets"][0]["sheetProtection"];
    assert_eq!(object_keys(sp), SHEET_PROT_KEYS);
    assert_eq!(sp, &default_sheet_protection());
}

#[test]
fn protect_unprotect_sheet_and_format_cells() {
    let src = create_book("round");
    let protected = tmp("prot");
    apply(
        &src,
        &protected,
        json!([{ "op": "setProtection", "id": "sheet:Sheet1", "enforced": true }]),
    )
    .unwrap();
    let xml = part_text(&protected, "xl/worksheets/sheet1.xml");
    assert!(xml.contains("sheetProtection"), "{xml}");
    assert!(xml.contains(r#"sheet="1""#), "{xml}");
    assert!(xml.contains(r#"objects="1""#), "{xml}");
    assert!(xml.contains(r#"scenarios="1""#), "{xml}");
    assert!(!xml.contains("formatCells"), "{xml}");
    assert!(!xml.contains("algorithmName"), "{xml}");
    assert!(!xml.contains("hashValue"), "{xml}");
    assert!(!xml.contains("password="), "{xml}");
    let tree = inspect(&protected);
    let sp = &tree["sheets"][0]["sheetProtection"];
    assert_eq!(sp["enforced"], true);
    assert_eq!(sp["hasPassword"], false);
    assert_eq!(sp["objects"], true);
    assert_eq!(sp["scenarios"], true);
    assert_eq!(sp["formatCells"], true);

    let fmt = tmp("fmt");
    apply(
        &protected,
        &fmt,
        json!([{ "op": "setProtection", "id": "sheet:Sheet1", "formatCells": false }]),
    )
    .unwrap();
    let xml_fmt = part_text(&fmt, "xl/worksheets/sheet1.xml");
    assert!(xml_fmt.contains(r#"formatCells="0""#), "{xml_fmt}");
    assert_eq!(
        inspect(&fmt)["sheets"][0]["sheetProtection"]["formatCells"],
        false
    );

    let opened = tmp("open");
    apply(
        &fmt,
        &opened,
        json!([{ "op": "setProtection", "id": "sheet:Sheet1", "enforced": false }]),
    )
    .unwrap();
    let xml_open = part_text(&opened, "xl/worksheets/sheet1.xml");
    assert!(!xml_open.contains("sheetProtection"), "{xml_open}");
    assert_eq!(
        inspect(&opened)["sheets"][0]["sheetProtection"],
        default_sheet_protection()
    );
}

#[test]
fn workbook_lock_structure_order_and_clear() {
    let src = create_book("wb");
    let locked = tmp("lock");
    apply(
        &src,
        &locked,
        json!([{ "op": "setProtection", "id": "xlsx:workbook", "lockStructure": true }]),
    )
    .unwrap();
    let wb = part_text(&locked, "xl/workbook.xml");
    let i_prot = wb.find("<workbookProtection").expect("workbookProtection");
    if let Some(i_pr) = wb.find("<workbookPr>").or_else(|| wb.find("<workbookPr ")) {
        assert!(i_pr < i_prot, "{wb}");
    }
    if let Some(i_views) = wb.find("<bookViews") {
        assert!(i_prot < i_views, "{wb}");
    } else {
        let i_sheets = wb.find("<sheets").expect("sheets");
        assert!(i_prot < i_sheets, "{wb}");
    }
    assert!(wb.contains(r#"lockStructure="1""#), "{wb}");
    assert!(!wb.contains("lockWindows"), "{wb}");
    assert!(!wb.contains("hashValue"), "{wb}");
    let tree = inspect(&locked);
    assert_eq!(tree["workbookProtection"]["lockStructure"], true);
    assert_eq!(tree["workbookProtection"]["lockWindows"], false);
    assert_eq!(tree["workbookProtection"]["hasPassword"], false);

    let cleared = tmp("clear");
    apply(
        &locked,
        &cleared,
        json!([{ "op": "setProtection", "id": "xlsx:workbook", "lockStructure": false }]),
    )
    .unwrap();
    let wb2 = part_text(&cleared, "xl/workbook.xml");
    assert!(!wb2.contains("workbookProtection"), "{wb2}");
    assert_eq!(
        inspect(&cleared)["workbookProtection"]["lockStructure"],
        false
    );
}

#[test]
fn hashed_sheet_is_not_implemented() {
    let src = create_book("hash-src");
    let protected = tmp("hash-prot");
    apply(
        &src,
        &protected,
        json!([{ "op": "setProtection", "id": "sheet:Sheet1", "enforced": true }]),
    )
    .unwrap();
    let hashed = tmp("hash");
    rewrite_part(&protected, &hashed, "xl/worksheets/sheet1.xml", |xml| {
        xml.replace("<sheetProtection ", r#"<sheetProtection hashValue="YWJj" "#)
    });
    let tree = inspect(&hashed);
    assert_eq!(tree["sheets"][0]["sheetProtection"]["hasPassword"], true);
    assert_eq!(tree["sheets"][0]["sheetProtection"]["enforced"], true);
    assert!(!tree.to_string().contains("YWJj"));
    let out = tmp("hash-out");
    let err = apply(
        &hashed,
        &out,
        json!([{ "op": "setProtection", "id": "sheet:Sheet1", "enforced": false }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::NotImplemented);
    assert!(!out.exists());
}

#[test]
fn missing_field_wrong_id_password_and_create() {
    let src = create_book("err");
    let out = tmp("missing");
    let err = apply(
        &src,
        &out,
        json!([{ "op": "setProtection", "id": "sheet:Sheet1" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());

    let err_id = apply(
        &src,
        &out,
        json!([{ "op": "setProtection", "id": "cell:Sheet1!A1", "enforced": true }]),
    )
    .unwrap_err();
    assert_eq!(err_id.code(), ErrorCode::InvalidId);

    let created = tmp("create-prot");
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": created.to_string_lossy(),
        "workbook": {
            "sheets": [{
                "name": "Sheet1",
                "sheetProtection": { "enforced": true }
            }]
        }
    })
    .to_string())
    .unwrap();
    assert_eq!(
        inspect(&created)["sheets"][0]["sheetProtection"]["enforced"],
        true
    );

    let created_wb = tmp("create-wb");
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": created_wb.to_string_lossy(),
        "workbook": {
            "workbookProtection": { "lockStructure": true },
            "sheets": [{ "name": "Sheet1" }]
        }
    })
    .to_string())
    .unwrap();
    assert_eq!(
        inspect(&created_wb)["workbookProtection"]["lockStructure"],
        true
    );
}

#[test]
fn chartsheet_protects_and_rejects_worksheet_flags() {
    let src = create_book("cs");
    let with_cs = tmp("cs-ins");
    apply(&src, &with_cs, json!([insert_chartsheet_op()])).unwrap();
    let tree0 = inspect(&with_cs);
    let cs = sheet_by_name(&tree0, "Chart1");
    assert_eq!(
        object_keys(&cs["sheetProtection"]),
        ["enforced", "hasPassword", "objects"]
    );
    assert_eq!(cs["sheetProtection"]["enforced"], false);
    assert_eq!(cs["sheetProtection"]["objects"], false);

    let prot = tmp("cs-prot");
    apply(
        &with_cs,
        &prot,
        json!([{ "op": "setProtection", "id": "sheet:Chart1", "enforced": true }]),
    )
    .unwrap();
    let xml = part_text(&prot, "xl/chartsheets/sheet1.xml");
    assert!(xml.contains(r#"content="1""#), "{xml}");
    assert!(xml.contains(r#"objects="1""#), "{xml}");
    assert!(!xml.contains(r#"sheet="1""#), "{xml}");
    let tree = inspect(&prot);
    let cs2 = sheet_by_name(&tree, "Chart1");
    assert_eq!(cs2["sheetProtection"]["enforced"], true);
    assert_eq!(cs2["sheetProtection"]["objects"], true);

    let bad = tmp("cs-bad");
    let err = apply(
        &prot,
        &bad,
        json!([{ "op": "setProtection", "id": "sheet:Chart1", "formatCells": false }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!bad.exists());
}
