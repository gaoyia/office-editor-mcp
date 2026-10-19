//! 工作表嵌入对象：计数、名单、导出、插删；预览图不进 pictures，dummy 不进 shapes。

use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::ZipArchive;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-xlsx-ole");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn tmp(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}"))
}

fn create_book(tag: &str) -> PathBuf {
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

fn create_embed(tag: &str) -> PathBuf {
    let path = tmp(tag).with_extension("xlsx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "workbook": {
                "sheets": [{
                    "name": "Sheet1",
                    "cells": [{ "ref": "A1", "value": 42 }]
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

fn part_text(path: &Path, name: &str) -> String {
    String::from_utf8(zip_entry(path, name).unwrap_or_default()).unwrap()
}

#[test]
fn empty_create_has_count_without_list() {
    let path = create_book("empty");
    let tree = inspect(&path, json!({}));
    assert_eq!(tree["oleObjectCount"], 0);
    assert_eq!(tree["sheets"][0]["oleObjectCount"], 0);
    assert!(tree["sheets"][0].get("oleObjects").is_none());
    let listed = inspect(&path, json!({ "oleObjects": true }));
    assert_eq!(listed["sheets"][0]["oleObjects"], json!([]));
}

#[test]
fn insert_xlsx_lists_package_kind_and_skips_dummy() {
    let src = create_book("ins-host");
    let embed = create_embed("embed");
    let out = tmp("ins").with_extension("xlsx");
    apply(
        &src,
        &out,
        json!([{
            "op": "insertOle",
            "id": "sheet:Sheet1",
            "file": embed.to_string_lossy(),
            "from": "B2"
        }]),
    )
    .unwrap();
    let tree = inspect(
        &out,
        json!({ "oleObjects": true, "pictures": true, "shapes": true }),
    );
    assert_eq!(tree["oleObjectCount"], 1);
    assert_eq!(tree["sheets"][0]["oleObjectCount"], 1);
    assert_eq!(tree["pictureCount"], 0);
    assert_eq!(tree["shapeCount"], 0);
    assert_eq!(tree["sheets"][0]["pictures"], json!([]));
    assert_eq!(tree["sheets"][0]["shapes"], json!([]));
    let items = tree["sheets"][0]["oleObjects"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    let o = &items[0];
    assert_eq!(o["id"], "sheet:Sheet1/ole:1025");
    assert_eq!(o["shapeId"], "1025");
    assert_eq!(o["name"], "Object 1");
    assert_eq!(o["progId"], "Worksheet");
    assert_eq!(o["kind"], "package");
    assert_eq!(
        o["contentType"],
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
    );
    assert_eq!(o["showAsIcon"], false);
    assert_eq!(o["stable"], true);
    assert_eq!(o["anchor"], "twoCell");
    assert_eq!(o["from"], "B2");
    assert_eq!(
        object_keys(o),
        [
            "id",
            "shapeId",
            "name",
            "progId",
            "kind",
            "contentType",
            "showAsIcon",
            "anchor",
            "from",
            "fromOffX",
            "fromOffY",
            "to",
            "toOffX",
            "toOffY",
            "editAs",
            "stable"
        ]
    );
    assert!(zip_has_prefix(&out, "xl/embeddings/"));
    let sheet = part_text(&out, "xl/worksheets/sheet1.xml");
    assert!(sheet.contains("AlternateContent"));
    assert!(sheet.contains("oleObject"));
    assert!(sheet.contains("progId=\"Worksheet\""));
    assert!(sheet.contains("shapeId=\"1025\""));
    assert!(!sheet.contains("a16:creationId"));
    let drawing = part_text(&out, "xl/drawings/drawing1.xml");
    assert!(drawing.contains("Object 1"));
    assert!(drawing.contains("a14:compatExt"));
    assert!(drawing.contains("editAs=\"oneCell\""));
    assert!(drawing.contains("hidden=\"1\""));
    assert!(drawing.contains("<mc:Choice"));
    let choice_at = drawing.find("<mc:Choice").unwrap();
    let anchor_at = drawing.find("<xdr:twoCellAnchor").unwrap();
    assert!(choice_at < anchor_at, "twoCellAnchor 必须包在 Choice 里");
    assert!(!drawing.contains("a16:creationId"));
    let vml = part_text(&out, "xl/drawings/vmlDrawing1.vml");
    assert!(vml.contains("_x0000_s1025"));
    assert!(vml.contains("ObjectType=\"Pict\""));
    assert!(vml.contains("<x:CF>Pict</x:CF>"));
    assert!(!vml.contains("_x0000_t202"));
    assert!(sheet.contains("<from>"));
    assert!(!sheet.contains("<xdr:from>"));
}

#[test]
fn export_media_matches_embedding_bytes() {
    let src = create_book("exp-host");
    let embed = create_embed("embed-exp");
    let src_bytes = std::fs::read(&embed).unwrap();
    let book = tmp("exp-book").with_extension("xlsx");
    apply(
        &src,
        &book,
        json!([{
            "op": "insertOle",
            "id": "sheet:Sheet1",
            "file": embed.to_string_lossy(),
            "from": "B2"
        }]),
    )
    .unwrap();
    let tree = inspect(&book, json!({ "oleObjects": true }));
    let id = tree["sheets"][0]["oleObjects"][0]["id"].as_str().unwrap();
    let out = tmp("export").with_extension("xlsx");
    let RunOutput::Written { content_type, .. } = run(
        &json!({
            "schemaVersion": 1,
            "action": "exportMedia",
            "path": book.to_string_lossy(),
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
    let src = create_book("del-host");
    let embed = create_embed("embed-del");
    let with_ole = tmp("del-src").with_extension("xlsx");
    apply(
        &src,
        &with_ole,
        json!([{
            "op": "insertOle",
            "id": "sheet:Sheet1",
            "file": embed.to_string_lossy(),
            "from": "B2"
        }]),
    )
    .unwrap();
    let listed = inspect(&with_ole, json!({ "oleObjects": true }));
    let id = listed["sheets"][0]["oleObjects"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let out = tmp("del").with_extension("xlsx");
    apply(&with_ole, &out, json!([{ "op": "deleteOle", "id": id }])).unwrap();
    let tree = inspect(&out, json!({ "oleObjects": true, "shapes": true }));
    assert_eq!(tree["oleObjectCount"], 0);
    assert_eq!(tree["sheets"][0]["oleObjectCount"], 0);
    let list = tree["sheets"][0]["oleObjects"].as_array().unwrap();
    assert!(list.is_empty());
    assert_eq!(tree["shapeCount"], 0);
    assert!(!zip_has_prefix(&out, "xl/embeddings/"));
    assert!(!zip_has_prefix(&out, "xl/drawings/"));
}

#[test]
fn insert_txt_is_package_icon() {
    let src = create_book("txt-host");
    let txt = tmp("notes").with_extension("txt");
    std::fs::write(&txt, "hello ole").unwrap();
    let out = tmp("txt").with_extension("xlsx");
    apply(
        &src,
        &out,
        json!([{
            "op": "insertOle",
            "id": "sheet:Sheet1",
            "file": txt.to_string_lossy(),
            "from": "C3"
        }]),
    )
    .unwrap();
    let tree = inspect(&out, json!({ "oleObjects": true }));
    let o = &tree["sheets"][0]["oleObjects"][0];
    assert_eq!(o["progId"], "Package");
    assert_eq!(o["kind"], "embed");
    assert_eq!(o["showAsIcon"], true);
    assert_eq!(
        o["contentType"],
        "application/vnd.openxmlformats-officedocument.oleObject"
    );
}

#[test]
fn wrong_ids_and_missing_from() {
    let src = create_book("bad-host");
    let embed = create_embed("bad-embed");
    let out = tmp("bad").with_extension("xlsx");
    let err = apply(
        &src,
        &out,
        json!([{
            "op": "insertOle",
            "id": "sheet:Sheet1",
            "file": embed.to_string_lossy()
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());

    let err = apply(
        &src,
        &out,
        json!([{
            "op": "insertOle",
            "id": "slide:1",
            "file": embed.to_string_lossy(),
            "from": "B2"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    assert!(!out.exists());

    let err = apply(
        &src,
        &out,
        json!([{
            "op": "insertOle",
            "id": "sheet:Sheet1/shape:2",
            "file": embed.to_string_lossy(),
            "from": "B2"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    assert!(!out.exists());

    let err = apply(
        &src,
        &out,
        json!([{ "op": "deleteOle", "id": "sheet:Sheet1" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    assert!(!out.exists());
}

#[test]
fn set_ole_size_moves_two_cell_corner() {
    let src = create_book("size-host");
    let embed = create_embed("size-embed");
    let inserted = tmp("size-in").with_extension("xlsx");
    apply(
        &src,
        &inserted,
        json!([{
            "op": "insertOle",
            "id": "sheet:Sheet1",
            "file": embed.to_string_lossy(),
            "from": "B2"
        }]),
    )
    .unwrap();
    let listed = inspect(&inserted, json!({ "oleObjects": true }));
    let id = listed["sheets"][0]["oleObjects"][0]["id"].as_str().unwrap();
    assert_eq!(listed["sheets"][0]["oleObjects"][0]["anchor"], "twoCell");
    let out = tmp("size-out").with_extension("xlsx");
    apply(
        &inserted,
        &out,
        json!([{ "op": "setOleSize", "id": id, "to": "E6" }]),
    )
    .unwrap();
    let tree = inspect(&out, json!({ "oleObjects": true }));
    let o = &tree["sheets"][0]["oleObjects"][0];
    assert_eq!(o["from"], "B2");
    assert_eq!(o["to"], "E6");
    let sheet = part_text(&out, "xl/worksheets/sheet1.xml");
    assert!(sheet.contains("<xdr:col>4</xdr:col>"));
    assert!(sheet.contains("<xdr:row>5</xdr:row>"));
    let drawing = part_text(&out, "xl/drawings/drawing1.xml");
    assert!(drawing.contains("<xdr:col>4</xdr:col>"));
    assert!(drawing.contains("<xdr:row>5</xdr:row>"));

    let err = apply(
        &out,
        &tmp("size-bad").with_extension("xlsx"),
        json!([{ "op": "setOleSize", "id": id, "cx": 914400 }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}
