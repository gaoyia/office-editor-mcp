//! 幻灯片嵌入对象：计数、名单、导出、插删；预览图不进 pictures。

use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::ZipArchive;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-pptx-ole");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn tmp(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}"))
}

fn create_deck(tag: &str) -> PathBuf {
    let path = tmp(tag).with_extension("pptx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "presentation": { "slides": [{ "shapes": [{ "text": "标题" }] }] }
        })
        .to_string(),
        None,
    )
    .unwrap();
    path
}

fn create_xlsx(tag: &str) -> PathBuf {
    let path = tmp(tag).with_extension("xlsx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "workbook": {
                "sheets": [{
                    "name": "Sheet1",
                    "cells": [{ "ref": "A1", "value": "嵌入" }]
                }]
            }
        })
        .to_string(),
        None,
    )
    .unwrap();
    path
}

fn inspect(path: &Path, scope: Value) -> Value {
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

fn object_keys(v: &Value) -> Vec<&str> {
    v.as_object().unwrap().keys().map(String::as_str).collect()
}

fn zip_entry(path: &Path, name: &str) -> Option<Vec<u8>> {
    let bytes = std::fs::read(path).unwrap();
    let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let want = name.replace('\\', "/");
    for i in 0..zip.len() {
        let mut f = zip.by_index(i).unwrap();
        if f.name().replace('\\', "/") == want {
            let mut buf = Vec::new();
            f.read_to_end(&mut buf).unwrap();
            return Some(buf);
        }
    }
    None
}

fn zip_has_prefix(path: &Path, prefix: &str) -> bool {
    let bytes = std::fs::read(path).unwrap();
    let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    for i in 0..zip.len() {
        let name = zip.by_index(i).unwrap().name().replace('\\', "/");
        if name.starts_with(prefix) {
            return true;
        }
    }
    false
}

#[test]
fn empty_create_has_count_without_list() {
    let path = create_deck("empty");
    let tree = inspect(&path, json!({}));
    assert_eq!(tree["oleObjectCount"], 0);
    assert_eq!(tree["slides"][0]["oleObjectCount"], 0);
    assert!(tree["slides"][0].get("oleObjects").is_none());
    assert_eq!(
        object_keys(&tree),
        [
            "schemaVersion",
            "format",
            "id",
            "slideCount",
            "pictureCount",
            "videoCount",
            "audioCount",
            "chartCount",
            "smartArtCount",
            "commentCount",
            "hyperlinkCount",
            "tableCount",
            "oleObjectCount",
            "zoomCount",
            "connectorCount",
            "animCount",
            "masterCount",
            "hasHandout",
            "sectionCount",
            "slideSize",
            "customShowCount",
            "showPr",
            "modifyProtection",
            "props",
            "slides"
        ]
    );
}

#[test]
fn insert_xlsx_lists_package_kind_and_skips_preview() {
    let src = create_deck("ins-host");
    let xlsx = create_xlsx("embed");
    let out = tmp("ins").with_extension("pptx");
    apply(
        &src,
        &out,
        json!([{ "op": "insertOle", "id": "slide:1", "file": xlsx.to_string_lossy() }]),
    )
    .unwrap();
    let tree = inspect(&out, json!({ "oleObjects": true, "pictures": true }));
    assert_eq!(tree["oleObjectCount"], 1);
    assert_eq!(tree["slides"][0]["oleObjectCount"], 1);
    assert_eq!(tree["pictureCount"], 0);
    assert_eq!(tree["slides"][0]["pictures"], json!([]));
    let items = tree["slides"][0]["oleObjects"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    let o = &items[0];
    let id = o["id"].as_str().unwrap();
    assert!(id.starts_with("slide:1/ole:"));
    assert!(!id.contains("index:"));
    assert_eq!(o["progId"], "Excel.Sheet.12");
    assert_eq!(o["name"], "Worksheet");
    assert_eq!(o["kind"], "package");
    assert_eq!(
        o["contentType"],
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
    );
    assert_eq!(o["showAsIcon"], false);
    assert_eq!(o["stable"], true);
    assert_eq!(
        object_keys(o),
        [
            "id",
            "progId",
            "name",
            "kind",
            "contentType",
            "showAsIcon",
            "x",
            "y",
            "cx",
            "cy",
            "stable"
        ]
    );
    assert!(zip_has_prefix(&out, "ppt/embeddings/"));
    let xml =
        String::from_utf8(zip_entry(&out, "ppt/slides/slide1.xml").unwrap_or_default()).unwrap();
    assert!(xml.contains("AlternateContent"));
    assert!(xml.contains("p:oleObj"));
    assert!(!xml.contains("a16:creationId"));
}

#[test]
fn export_media_matches_embedding_bytes() {
    let src = create_deck("exp-host");
    let xlsx = create_xlsx("embed-exp");
    let src_bytes = std::fs::read(&xlsx).unwrap();
    let deck = tmp("exp-deck").with_extension("pptx");
    apply(
        &src,
        &deck,
        json!([{ "op": "insertOle", "id": "slide:1", "file": xlsx.to_string_lossy() }]),
    )
    .unwrap();
    let tree = inspect(&deck, json!({ "oleObjects": true }));
    let id = tree["slides"][0]["oleObjects"][0]["id"].as_str().unwrap();
    let out = tmp("export").with_extension("xlsx");
    let RunOutput::Written { content_type, .. } = run(
        &json!({
            "schemaVersion": 1,
            "action": "exportMedia",
            "path": deck.to_string_lossy(),
            "id": id,
            "output": out.to_string_lossy()
        })
        .to_string(),
        None,
    )
    .unwrap() else {
        panic!("exportMedia 应写出文件");
    };
    assert_eq!(std::fs::read(&out).unwrap(), src_bytes);
    assert_eq!(
        content_type.as_deref(),
        Some("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet")
    );
}

#[test]
fn delete_removes_object_and_embedding() {
    let src = create_deck("del-host");
    let xlsx = create_xlsx("embed-del");
    let with_ole = tmp("del-src").with_extension("pptx");
    apply(
        &src,
        &with_ole,
        json!([{ "op": "insertOle", "id": "slide:1", "file": xlsx.to_string_lossy() }]),
    )
    .unwrap();
    let listed = inspect(&with_ole, json!({ "oleObjects": true }));
    let id = listed["slides"][0]["oleObjects"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let out = tmp("del").with_extension("pptx");
    apply(&with_ole, &out, json!([{ "op": "deleteOle", "id": id }])).unwrap();
    let tree = inspect(&out, json!({ "oleObjects": true }));
    assert_eq!(tree["oleObjectCount"], 0);
    assert_eq!(tree["slides"][0]["oleObjectCount"], 0);
    let list = tree["slides"][0]["oleObjects"].as_array().unwrap();
    assert!(list.is_empty());
    assert!(!zip_has_prefix(&out, "ppt/embeddings/"));
}

#[test]
fn insert_txt_is_package_icon() {
    let src = create_deck("txt-host");
    let txt = tmp("notes").with_extension("txt");
    std::fs::write(&txt, "hello ole").unwrap();
    let out = tmp("txt").with_extension("pptx");
    apply(
        &src,
        &out,
        json!([{ "op": "insertOle", "id": "slide:1", "file": txt.to_string_lossy() }]),
    )
    .unwrap();
    let tree = inspect(&out, json!({ "oleObjects": true }));
    let o = &tree["slides"][0]["oleObjects"][0];
    assert_eq!(o["progId"], "Package");
    assert_eq!(o["kind"], "embed");
    assert_eq!(o["showAsIcon"], true);
    assert_eq!(
        o["contentType"],
        "application/vnd.openxmlformats-officedocument.oleObject"
    );
}

#[test]
fn wrong_ids_and_xlsx_reject() {
    let deck = create_deck("bad-host");
    let xlsx_src = create_xlsx("bad-embed");
    let out = tmp("bad").with_extension("pptx");
    let err = apply(
        &deck,
        &out,
        json!([{ "op": "insertOle", "id": "slide:1/ole:4", "file": xlsx_src.to_string_lossy() }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    assert!(!out.exists());

    let err = apply(&deck, &out, json!([{ "op": "deleteOle", "id": "slide:1" }])).unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    assert!(!out.exists());

    let book = tmp("xlsx-ole").with_extension("xlsx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": book.to_string_lossy(),
            "workbook": { "sheets": [{ "name": "Sheet1" }] }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let err = apply(
        &book,
        &tmp("xlsx-ole-out").with_extension("xlsx"),
        json!([{ "op": "insertOle", "id": "sheet:Sheet1", "file": xlsx_src.to_string_lossy() }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": book.to_string_lossy(),
            "scope": { "oleObjects": true }
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": tmp("create-ole").with_extension("pptx").to_string_lossy(),
            "presentation": {
                "slides": [{ "shapes": [{ "text": "a" }], "oleObjects": [{ "file": "a.xlsx" }] }]
            }
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

#[test]
fn xlsx_export_media_rejects_pptx_ole_id() {
    let book = tmp("xlsx-export").with_extension("xlsx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": book.to_string_lossy(),
            "workbook": { "sheets": [{ "name": "Sheet1" }] }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "exportMedia",
            "path": book.to_string_lossy(),
            "id": "slide:1/ole:4",
            "output": tmp("xlsx-ole.png").to_string_lossy()
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
}
