//! xlsx 嵌入 EMF / WMF / TIFF：inspect contentType、insert/replace/export/create。

mod common;

use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::ZipArchive;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-xlsx-emf");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn seq_path(tag: &str, ext: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}.{ext}"))
}

fn write_bytes(tag: &str, ext: &str, bytes: &[u8]) -> PathBuf {
    let path = seq_path(tag, ext);
    std::fs::write(&path, bytes).unwrap();
    path
}

fn create_book(tag: &str, sheet: Value) -> PathBuf {
    let path = seq_path(tag, "xlsx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "format": "xlsx",
            "workbook": { "sheets": [sheet] }
        })
        .to_string(),
        None,
    )
    .unwrap();
    path
}

fn inspect_pics(path: &Path) -> Value {
    let RunOutput::Inspect(tree) = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": path.to_string_lossy(),
            "scope": { "pictures": true }
        })
        .to_string(),
        None,
    )
    .unwrap() else {
        panic!("inspect 应返回树");
    };
    tree
}

fn apply(path: &Path, out: &Path, ops: Value) {
    run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": path.to_string_lossy(),
            "output": out.to_string_lossy(),
            "ops": ops
        })
        .to_string(),
        None,
    )
    .unwrap();
}

fn export_media(path: &Path, id: &str, output: &Path) -> RunOutput {
    run(
        &json!({
            "schemaVersion": 1,
            "action": "exportMedia",
            "path": path.to_string_lossy(),
            "id": id,
            "output": output.to_string_lossy()
        })
        .to_string(),
        None,
    )
    .unwrap()
}

fn zip_names(path: &Path) -> Vec<String> {
    let bytes = std::fs::read(path).unwrap();
    let zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    zip.file_names().map(|s| s.to_string()).collect()
}

fn part_text(path: &Path, name: &str) -> String {
    let bytes = std::fs::read(path).unwrap();
    let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut file = zip.by_name(name.trim_start_matches('/')).unwrap();
    let mut s = String::new();
    file.read_to_string(&mut s).unwrap();
    s
}

fn first_pic(tree: &Value) -> &Value {
    &tree["sheets"][0]["pictures"][0]
}

#[test]
fn insert_emf_writes_original_and_content_type() {
    let src = create_book(
        "src",
        json!({ "name": "Sheet1", "cells": [{ "ref": "A1", "value": 1 }] }),
    );
    let emf = write_bytes("in", "emf", &common::emf_header());
    let out = seq_path("ins", "xlsx");
    apply(
        &src,
        &out,
        json!([{
            "op": "insertPicture",
            "id": "sheet:Sheet1",
            "file": emf.to_string_lossy(),
            "from": "B2"
        }]),
    );
    let tree = inspect_pics(&out);
    let pic = first_pic(&tree);
    assert_eq!(pic["contentType"], "image/x-emf");
    let keys: Vec<&str> = pic
        .as_object()
        .unwrap()
        .keys()
        .map(|k| k.as_str())
        .collect();
    assert!(keys.last() == Some(&"hyperlink"), "{keys:?}");
    let names = zip_names(&out);
    assert!(names.iter().any(|n| n.ends_with(".emf")), "{names:?}");
    assert!(!names.iter().any(|n| n.ends_with(".svg")), "{names:?}");
    let drawing = part_text(&out, "xl/drawings/drawing1.xml");
    assert!(!drawing.contains("svgBlip"), "{drawing}");
    let exp = seq_path("exp", "emf");
    let RunOutput::Written { content_type, .. } =
        export_media(&out, pic["id"].as_str().unwrap(), &exp)
    else {
        panic!("export 应写出");
    };
    assert_eq!(content_type.as_deref(), Some("image/x-emf"));
    assert_eq!(std::fs::read(&exp).unwrap(), common::emf_header());
}

#[test]
fn create_wmf_and_tiff() {
    let wmf = write_bytes("c", "wmf", &common::placeable_wmf());
    let tiff = write_bytes("c", "tif", &common::tiff_32x16());
    let path = create_book(
        "create-meta",
        json!({
            "name": "Sheet1",
            "cells": [{ "ref": "A1", "value": 1 }],
            "pictures": [
                { "file": wmf.to_string_lossy(), "from": "B2" },
                { "file": tiff.to_string_lossy(), "from": "D2" }
            ]
        }),
    );
    let tree = inspect_pics(&path);
    let pics = tree["sheets"][0]["pictures"].as_array().unwrap();
    assert_eq!(pics[0]["contentType"], "image/x-wmf");
    assert_eq!(pics[1]["contentType"], "image/tiff");
    let names = zip_names(&path);
    assert!(names.iter().any(|n| n.ends_with(".wmf")), "{names:?}");
    assert!(names.iter().any(|n| n.ends_with(".tif")), "{names:?}");
}

#[test]
fn replace_emf_with_png_drops_content_type() {
    let emf = write_bytes("r", "emf", &common::emf_header());
    let png = write_bytes("r", "png", common::PNG_1X1);
    let src = create_book(
        "rep-src",
        json!({
            "name": "Sheet1",
            "cells": [{ "ref": "A1", "value": 1 }],
            "pictures": [{ "file": emf.to_string_lossy(), "from": "B2" }]
        }),
    );
    let id = first_pic(&inspect_pics(&src))["id"]
        .as_str()
        .unwrap()
        .to_string();
    let out = seq_path("rep", "xlsx");
    apply(
        &src,
        &out,
        json!([{ "op": "replacePicture", "id": id, "file": png.to_string_lossy() }]),
    );
    let tree = inspect_pics(&out);
    let pic = first_pic(&tree);
    assert!(pic.get("contentType").is_none(), "{pic}");
    let names = zip_names(&out);
    assert!(!names.iter().any(|n| n.ends_with(".emf")), "{names:?}");
}

#[test]
fn reject_webp_and_junk() {
    let src = create_book(
        "rej",
        json!({ "name": "Sheet1", "cells": [{ "ref": "A1", "value": 1 }] }),
    );
    let junk = write_bytes("j", "webp", b"RIFF....WEBP");
    let out = seq_path("should-not", "xlsx");
    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": src.to_string_lossy(),
            "output": out.to_string_lossy(),
            "ops": [{
                "op": "insertPicture",
                "id": "sheet:Sheet1",
                "file": junk.to_string_lossy(),
                "from": "B2"
            }]
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());
}
