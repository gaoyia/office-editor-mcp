//! 零脏保存：部件字节与 ZIP CRC 都与打开时相同。

use std::collections::HashMap;
use std::io::Cursor;
use std::path::PathBuf;

use serde_json::{json, Value};
use zip::ZipArchive;

use super::Package;
use crate::run;

fn zip_file_crc(bytes: &[u8]) -> HashMap<String, u32> {
    let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut out = HashMap::new();
    for i in 0..zip.len() {
        let file = zip.by_index(i).unwrap();
        let name = file.name().replace('\\', "/");
        if name.ends_with('/') {
            continue;
        }
        out.insert(name, file.crc32());
    }
    out
}

fn assert_identity_save(src: Vec<u8>) {
    let crc_before = zip_file_crc(&src);
    let pkg = Package::open_bytes(src).unwrap();
    let names = pkg.part_names();
    assert!(!names.is_empty(), "包里没有部件");
    let parts: Vec<(String, Vec<u8>)> = names
        .iter()
        .map(|n| (n.clone(), pkg.get(n).unwrap().to_vec()))
        .collect();

    let out = pkg.save_to_vec().unwrap();
    let again = Package::open_bytes(out.clone()).unwrap();
    assert_eq!(again.part_names(), names);
    for (name, data) in &parts {
        assert_eq!(again.get(name).unwrap(), data.as_slice(), "{name} 字节变了");
    }

    let crc_after = zip_file_crc(&out);
    assert_eq!(crc_before.len(), crc_after.len(), "文件条目数变了");
    for (name, crc) in &crc_before {
        assert_eq!(crc_after.get(name), Some(crc), "{name} CRC 变了");
    }
}

fn create_file(name: &str, extra: Value) -> Vec<u8> {
    let dir = std::env::temp_dir().join("ooxp-opc-identity");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    let mut req = extra;
    req["schemaVersion"] = json!(1);
    req["action"] = json!("create");
    req["path"] = json!(path.to_string_lossy());
    run(&req.to_string(), None).unwrap();
    std::fs::read(&path).unwrap()
}

#[test]
fn excel_saved_xlsx_identity_save() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/excel-source.xlsx");
    assert!(
        path.is_file(),
        "缺少 {path:?}，见 crates/ooxp/tests/fixtures/excel-source.xlsx"
    );
    assert_identity_save(std::fs::read(path).unwrap());
}

#[test]
fn created_docx_identity_save() {
    let bytes = create_file(
        "identity.docx",
        json!({
            "format": "docx",
            "document": {
                "paragraphs": [
                    { "style": "Heading1", "text": "标题" },
                    { "text": "hello" }
                ]
            }
        }),
    );
    assert_identity_save(bytes);
}

#[test]
fn created_pptx_identity_save() {
    let bytes = create_file(
        "identity.pptx",
        json!({
            "format": "pptx",
            "presentation": {
                "slides": [
                    { "shapes": [{ "text": "标题" }, { "text": "hello" }] },
                    { "shapes": [{ "text": "第二页" }] }
                ]
            }
        }),
    );
    assert_identity_save(bytes);
}
