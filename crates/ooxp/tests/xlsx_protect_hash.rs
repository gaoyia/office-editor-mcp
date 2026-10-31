//! 工作表 / 工作簿保护：setProtection 写 SHA-512，inspect 只报 hasPassword。

use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-xlsx-protect-hash");
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

fn part_text(path: &Path, name: &str) -> String {
    let f = std::fs::File::open(path).unwrap();
    let mut zip = zip::ZipArchive::new(f).unwrap();
    let mut zf = zip.by_name(name).unwrap();
    let mut out = String::new();
    zf.read_to_string(&mut out).unwrap();
    out
}

fn assert_fail(src: &Path, ops: Value, want: ErrorCode) {
    let out = tmp("fail");
    let err = apply(src, &out, ops).unwrap_err();
    assert_eq!(err.code(), want);
    assert!(!out.exists());
}

#[test]
fn sheet_password_sets_sha512_and_stays_out_of_tree() {
    let src = create_book("sheet");
    let out = tmp("prot");
    apply(
        &src,
        &out,
        json!([{
            "op": "setProtection",
            "id": "sheet:Sheet1",
            "enforced": true,
            "password": "secret"
        }]),
    )
    .unwrap();
    let tree = inspect(&out);
    let sp = &tree["sheets"][0]["sheetProtection"];
    assert_eq!(sp["enforced"], true);
    assert_eq!(sp["hasPassword"], true);
    assert!(sp.get("hashValue").is_none());
    assert!(sp.get("saltValue").is_none());
    assert!(sp.get("algorithmName").is_none());
    assert!(sp.get("spinCount").is_none());
    assert!(sp.get("password").is_none());
    let dump = tree.to_string();
    assert!(!dump.contains("secret"), "{dump}");
    assert!(!dump.contains("hashValue"));
    let xml = part_text(&out, "xl/worksheets/sheet1.xml");
    assert!(xml.contains(r#"algorithmName="SHA-512""#), "{xml}");
    assert!(xml.contains(r#"spinCount="100000""#), "{xml}");
    assert!(xml.contains("hashValue="), "{xml}");
    assert!(xml.contains("saltValue="), "{xml}");
    assert!(!xml.contains(r#" password=""#), "{xml}");
    assert!(!xml.contains(">secret<"), "{xml}");
}

#[test]
fn workbook_password_sets_prefixed_sha512() {
    let src = create_book("wb");
    let out = tmp("wb");
    apply(
        &src,
        &out,
        json!([{
            "op": "setProtection",
            "id": "xlsx:workbook",
            "lockStructure": true,
            "password": "secret"
        }]),
    )
    .unwrap();
    let tree = inspect(&out);
    assert_eq!(tree["workbookProtection"]["lockStructure"], true);
    assert_eq!(tree["workbookProtection"]["hasPassword"], true);
    assert!(tree["workbookProtection"]
        .get("workbookHashValue")
        .is_none());
    let dump = tree.to_string();
    assert!(!dump.contains("secret"), "{dump}");
    let xml = part_text(&out, "xl/workbook.xml");
    assert!(xml.contains(r#"workbookAlgorithmName="SHA-512""#), "{xml}");
    assert!(xml.contains(r#"workbookSpinCount="100000""#), "{xml}");
    assert!(xml.contains("workbookHashValue="), "{xml}");
    assert!(xml.contains("workbookSaltValue="), "{xml}");
    assert!(!xml.contains("workbookPassword="), "{xml}");
}

#[test]
fn matching_password_unprotects_and_wrong_password_fails() {
    let src = create_book("round");
    let prot = tmp("prot");
    apply(
        &src,
        &prot,
        json!([{
            "op": "setProtection",
            "id": "sheet:Sheet1",
            "enforced": true,
            "password": "secret"
        }]),
    )
    .unwrap();

    assert_fail(
        &prot,
        json!([{
            "op": "setProtection",
            "id": "sheet:Sheet1",
            "enforced": false
        }]),
        ErrorCode::NotImplemented,
    );
    assert_fail(
        &prot,
        json!([{
            "op": "setProtection",
            "id": "sheet:Sheet1",
            "enforced": false,
            "password": "wrong"
        }]),
        ErrorCode::InvalidField,
    );

    let opened = tmp("open");
    apply(
        &prot,
        &opened,
        json!([{
            "op": "setProtection",
            "id": "sheet:Sheet1",
            "enforced": false,
            "password": "secret"
        }]),
    )
    .unwrap();
    let xml = part_text(&opened, "xl/worksheets/sheet1.xml");
    assert!(!xml.contains("sheetProtection"), "{xml}");
    assert_eq!(
        inspect(&opened)["sheets"][0]["sheetProtection"]["hasPassword"],
        false
    );
    assert_eq!(
        inspect(&opened)["sheets"][0]["sheetProtection"]["enforced"],
        false
    );
}

#[test]
fn blank_password_and_password_only_are_invalid() {
    let src = create_book("blank");
    assert_fail(
        &src,
        json!([{
            "op": "setProtection",
            "id": "sheet:Sheet1",
            "enforced": true,
            "password": ""
        }]),
        ErrorCode::InvalidField,
    );
    assert_fail(
        &src,
        json!([{
            "op": "setProtection",
            "id": "sheet:Sheet1",
            "password": "secret"
        }]),
        ErrorCode::InvalidField,
    );
}

#[test]
fn other_hash_stays_not_implemented_even_with_password() {
    let src = create_book("xor");
    let prot = tmp("plain");
    apply(
        &src,
        &prot,
        json!([{ "op": "setProtection", "id": "sheet:Sheet1", "enforced": true }]),
    )
    .unwrap();
    let hashed = tmp("xor-hash");
    {
        use std::io::{Cursor, Write};
        use zip::write::SimpleFileOptions;
        use zip::{CompressionMethod, ZipArchive, ZipWriter};
        let bytes = std::fs::read(&prot).unwrap();
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
                if n == "xl/worksheets/sheet1.xml" {
                    let xml = String::from_utf8(data).unwrap();
                    data = xml
                        .replace("<sheetProtection ", r#"<sheetProtection hashValue="YWJj" "#)
                        .into_bytes();
                }
                zip_out.start_file(&n, opt).unwrap();
                zip_out.write_all(&data).unwrap();
            }
            zip_out.finish().unwrap();
        }
        std::fs::write(&hashed, out.into_inner()).unwrap();
    }
    assert_eq!(
        inspect(&hashed)["sheets"][0]["sheetProtection"]["hasPassword"],
        true
    );
    assert_fail(
        &hashed,
        json!([{
            "op": "setProtection",
            "id": "sheet:Sheet1",
            "enforced": false,
            "password": "secret"
        }]),
        ErrorCode::NotImplemented,
    );
}

#[test]
fn legacy_xor_password_rewrites_to_sha512() {
    let src = create_book("xor2");
    let prot = tmp("xor2-plain");
    apply(
        &src,
        &prot,
        json!([{ "op": "setProtection", "id": "sheet:Sheet1", "enforced": true }]),
    )
    .unwrap();
    let hashed = tmp("xor2-hash");
    {
        use std::io::{Cursor, Write};
        use zip::write::SimpleFileOptions;
        use zip::{CompressionMethod, ZipArchive, ZipWriter};
        let bytes = std::fs::read(&prot).unwrap();
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
                if n == "xl/worksheets/sheet1.xml" {
                    let xml = String::from_utf8(data).unwrap();
                    data = xml
                        .replace("<sheetProtection ", r#"<sheetProtection password="DAA7" "#)
                        .into_bytes();
                }
                zip_out.start_file(&n, opt).unwrap();
                zip_out.write_all(&data).unwrap();
            }
            zip_out.finish().unwrap();
        }
        std::fs::write(&hashed, out.into_inner()).unwrap();
    }
    assert_fail(
        &hashed,
        json!([{
            "op": "setProtection",
            "id": "sheet:Sheet1",
            "enforced": false,
            "password": "wrong"
        }]),
        ErrorCode::InvalidField,
    );
    let opened = tmp("xor2-open");
    apply(
        &hashed,
        &opened,
        json!([{
            "op": "setProtection",
            "id": "sheet:Sheet1",
            "enforced": true,
            "password": "secret"
        }]),
    )
    .unwrap();
    let xml = part_text(&opened, "xl/worksheets/sheet1.xml");
    assert!(xml.contains(r#"algorithmName="SHA-512""#), "{xml}");
    assert!(!xml.contains("password="), "{xml}");
}
