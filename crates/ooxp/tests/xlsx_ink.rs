//! xlsx 工作表墨迹：inspect、insert / replace / delete、锚点、create、exportMedia。
//! 一笔一个对象。不进 pictures。Fallback 海报不进 pictures。

use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::ZipArchive;

static SEQ: AtomicU32 = AtomicU32::new(0);

const INKML: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><ink xmlns="http://www.w3.org/2003/InkML"><trace>0 0, 10 10</trace></ink>"#;
const INKML_B: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><ink xmlns="http://www.w3.org/2003/InkML"><trace>1 1, 20 20</trace></ink>"#;

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-xlsx-ink");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn seq_path(tag: &str, ext: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}.{ext}"))
}

fn write_ink(tag: &str, text: &str) -> PathBuf {
    let path = seq_path(tag, "xml");
    std::fs::write(&path, text).unwrap();
    path
}

fn create_book(tag: &str, sheet: Value) -> PathBuf {
    let path = seq_path(tag, "xlsx");
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "format": "xlsx",
        "workbook": { "sheets": [sheet] }
    })
    .to_string())
    .unwrap();
    path
}

fn inspect(path: &Path, scope: Value) -> Value {
    let RunOutput::Inspect(tree) = run(&json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": path.to_string_lossy(),
        "scope": scope
    })
    .to_string())
    .unwrap() else {
        panic!("inspect 应返回树");
    };
    tree
}

fn apply(path: &Path, out: &Path, ops: Value) {
    run(&json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": path.to_string_lossy(),
        "output": out.to_string_lossy(),
        "ops": ops
    })
    .to_string())
    .unwrap();
}

fn export_media(path: &Path, id: &str, output: &Path) -> RunOutput {
    run(&json!({
        "schemaVersion": 1,
        "action": "exportMedia",
        "path": path.to_string_lossy(),
        "id": id,
        "output": output.to_string_lossy()
    })
    .to_string())
    .unwrap()
}

fn zip_names(path: &Path) -> Vec<String> {
    let bytes = std::fs::read(path).unwrap();
    let zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    zip.file_names().map(|s| s.replace('\\', "/")).collect()
}

fn part_text(path: &Path, name: &str) -> String {
    let bytes = std::fs::read(path).unwrap();
    let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut file = zip.by_name(name.trim_start_matches('/')).unwrap();
    let mut s = String::new();
    file.read_to_string(&mut s).unwrap();
    s
}

fn first_ink(tree: &Value) -> &Value {
    &tree["sheets"][0]["inks"][0]
}

#[test]
fn empty_file_count_is_zero() {
    let path = create_book(
        "empty",
        json!({ "name": "Sheet1", "cells": [{ "ref": "A1", "value": 1 }] }),
    );
    let tree = inspect(&path, json!({}));
    assert_eq!(tree["inkCount"], 0);
    assert_eq!(tree["sheets"][0]["inkCount"], 0);
    assert!(tree["sheets"][0].get("inks").is_none());
    let listed = inspect(&path, json!({ "inks": true }));
    assert_eq!(listed["sheets"][0]["inks"], json!([]));
}

#[test]
fn insert_inspect_export_replace_delete() {
    let src = create_book(
        "src",
        json!({ "name": "Sheet1", "cells": [{ "ref": "A1", "value": 1 }] }),
    );
    let ink = write_ink("in", INKML);
    let out = seq_path("ins", "xlsx");
    apply(
        &src,
        &out,
        json!([{
            "op": "insertInk",
            "id": "sheet:Sheet1",
            "file": ink.to_string_lossy(),
            "from": "B2"
        }]),
    );
    let tree = inspect(&out, json!({ "inks": true, "pictures": true }));
    assert_eq!(tree["inkCount"], 1);
    assert_eq!(tree["sheets"][0]["inkCount"], 1);
    assert_eq!(tree["pictureCount"], 0);
    assert_eq!(tree["sheets"][0]["pictures"], json!([]));
    let item = first_ink(&tree);
    assert_eq!(item["contentType"], "application/inkml+xml");
    assert_eq!(item["stable"], true);
    assert_eq!(item["name"], "墨迹 1");
    assert_eq!(item["anchor"], "oneCell");
    assert_eq!(item["from"], "B2");
    assert!(item.get("trace").is_none());
    let id = item["id"].as_str().unwrap().to_string();
    assert!(id.starts_with("sheet:Sheet1/ink:"));
    let drawing = part_text(&out, "xl/drawings/drawing1.xml");
    assert!(drawing.contains("Requires=\"xdr14\""), "{drawing}");
    assert!(drawing.contains("xdr:contentPart"), "{drawing}");
    let rels = part_text(&out, "xl/drawings/_rels/drawing1.xml.rels");
    assert!(rels.contains("customXml"), "{rels}");
    let ct = part_text(&out, "[Content_Types].xml");
    assert!(ct.contains("application/inkml+xml"), "{ct}");
    assert!(zip_names(&out).iter().any(|n| n.contains("xl/ink/ink")));

    let exp = seq_path("exp", "xml");
    let RunOutput::Written { content_type, .. } = export_media(&out, &id, &exp) else {
        panic!("export 应写出");
    };
    assert_eq!(content_type.as_deref(), Some("application/inkml+xml"));
    assert_eq!(std::fs::read(&exp).unwrap(), INKML.as_bytes());

    let ink_b = write_ink("b", INKML_B);
    let replaced = seq_path("rep", "xlsx");
    apply(
        &out,
        &replaced,
        json!([{ "op": "replaceInk", "id": id, "file": ink_b.to_string_lossy() }]),
    );
    let tree_r = inspect(&replaced, json!({ "inks": true }));
    let id2 = first_ink(&tree_r)["id"].as_str().unwrap().to_string();
    assert_eq!(id2, id);
    let exp2 = seq_path("exp2", "xml");
    export_media(&replaced, &id2, &exp2);
    assert_eq!(std::fs::read(&exp2).unwrap(), INKML_B.as_bytes());

    let moved = seq_path("move", "xlsx");
    apply(
        &replaced,
        &moved,
        json!([{
            "op": "setInkAnchor",
            "id": id,
            "anchor": "twoCell",
            "to": "F8"
        }]),
    );
    let tree_m = inspect(&moved, json!({ "inks": true }));
    assert_eq!(first_ink(&tree_m)["anchor"], "twoCell");
    assert_eq!(first_ink(&tree_m)["to"], "F8");
    assert_eq!(first_ink(&tree_m)["from"], "B2");

    let deleted = seq_path("del", "xlsx");
    apply(&moved, &deleted, json!([{ "op": "deleteInk", "id": id }]));
    let tree_d = inspect(&deleted, json!({ "inks": true, "pictures": true }));
    assert_eq!(tree_d["inkCount"], 0);
    assert_eq!(tree_d["sheets"][0]["inks"], json!([]));
    assert!(!zip_names(&deleted).iter().any(|n| n.contains("xl/ink/")));
}

#[test]
fn two_strokes_are_two_objects() {
    let src = create_book(
        "two",
        json!({ "name": "Sheet1", "cells": [{ "ref": "A1", "value": 1 }] }),
    );
    let a = write_ink("a", INKML);
    let b = write_ink("b2", INKML_B);
    let out = seq_path("two-out", "xlsx");
    apply(
        &src,
        &out,
        json!([
            { "op": "insertInk", "id": "sheet:Sheet1", "file": a.to_string_lossy() },
            { "op": "insertInk", "id": "sheet:Sheet1", "file": b.to_string_lossy() }
        ]),
    );
    let tree = inspect(&out, json!({ "inks": true, "pictures": true }));
    assert_eq!(tree["inkCount"], 2);
    assert_eq!(tree["pictureCount"], 0);
    let inks = tree["sheets"][0]["inks"].as_array().unwrap();
    assert_eq!(inks[0]["name"], "墨迹 1");
    assert_eq!(inks[1]["name"], "墨迹 2");
    assert_ne!(inks[0]["id"], inks[1]["id"]);
}

#[test]
fn create_writes_ink() {
    let ink = write_ink("c", INKML);
    let path = create_book(
        "create",
        json!({
            "name": "Sheet1",
            "cells": [{ "ref": "A1", "value": 1 }],
            "inks": [{ "file": ink.to_string_lossy(), "from": "C3" }]
        }),
    );
    let tree = inspect(&path, json!({ "inks": true, "pictures": true }));
    assert_eq!(tree["inkCount"], 1);
    assert_eq!(first_ink(&tree)["contentType"], "application/inkml+xml");
    assert_eq!(first_ink(&tree)["from"], "C3");
    assert_eq!(tree["sheets"][0]["pictures"], json!([]));
}

#[test]
fn reject_junk_and_bare_sheet_id() {
    let src = create_book(
        "rej",
        json!({ "name": "Sheet1", "cells": [{ "ref": "A1", "value": 1 }] }),
    );
    let junk = write_ink("j", "<notink/>");
    let out = seq_path("should-not", "xlsx");
    let err = run(&json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": out.to_string_lossy(),
        "ops": [{
            "op": "insertInk",
            "id": "sheet:Sheet1",
            "file": junk.to_string_lossy()
        }]
    })
    .to_string())
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());

    let err = run(&json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": out.to_string_lossy(),
        "ops": [{ "op": "deleteInk", "id": "sheet:Sheet1" }]
    })
    .to_string())
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    let msg = err.to_json(ooxp::Lang::ZhCn)["error"]["message"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(msg.contains("必须带 /ink"), "{msg}");
}

#[test]
fn inspect_office_workbook2_if_present() {
    let path = Path::new(r"C:\Users\GaoYi-AMD\Desktop\工作簿2.xlsx");
    if !path.is_file() {
        return;
    }
    let tree = inspect(path, json!({ "inks": true, "pictures": true }));
    assert_eq!(tree["inkCount"], 3);
    assert_eq!(tree["sheets"][0]["inkCount"], 3);
    assert_eq!(tree["pictureCount"], 0);
    let inks = tree["sheets"][0]["inks"].as_array().unwrap();
    let ids: Vec<&str> = inks.iter().map(|v| v["id"].as_str().unwrap()).collect();
    assert_eq!(
        ids,
        [
            "sheet:Sheet1/ink:2",
            "sheet:Sheet1/ink:3",
            "sheet:Sheet1/ink:4"
        ]
    );
    assert!(inks.iter().all(|v| v["anchor"] == "twoCell"));
    assert!(inks
        .iter()
        .all(|v| v["contentType"] == "application/inkml+xml"));
    assert!(inks.iter().all(|v| v.get("trace").is_none()));
}
