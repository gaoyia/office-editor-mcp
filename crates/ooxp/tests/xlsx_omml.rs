//! xlsx 工作表公式框：inspect、insertMath / setMath / deleteMath、create。
//! 不进 textBoxes / shapes。出现 expr 或空 text 报 InvalidField。

use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::ZipArchive;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-xlsx-omml");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn seq_path(tag: &str, ext: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}.{ext}"))
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

fn part_text(path: &Path, name: &str) -> String {
    let bytes = std::fs::read(path).unwrap();
    let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut file = zip.by_name(name.trim_start_matches('/')).unwrap();
    let mut s = String::new();
    file.read_to_string(&mut s).unwrap();
    s
}

fn first_math(tree: &Value) -> &Value {
    &tree["sheets"][0]["maths"][0]
}

#[test]
fn empty_file_count_is_zero() {
    let path = create_book(
        "empty",
        json!({ "name": "Sheet1", "cells": [{ "ref": "A1", "value": 1 }] }),
    );
    let tree = inspect(&path, json!({}));
    assert_eq!(tree["mathCount"], 0);
    assert_eq!(tree["sheets"][0]["mathCount"], 0);
    assert!(tree["sheets"][0].get("maths").is_none());
    let listed = inspect(&path, json!({ "maths": true }));
    assert_eq!(listed["sheets"][0]["maths"], json!([]));
}

#[test]
fn insert_inspect_set_delete() {
    let src = create_book(
        "src",
        json!({ "name": "Sheet1", "cells": [{ "ref": "A1", "value": 1 }] }),
    );
    let out = seq_path("ins", "xlsx");
    apply(
        &src,
        &out,
        json!([{
            "op": "insertMath",
            "id": "sheet:Sheet1",
            "text": "E=mc^2",
            "from": "B2"
        }]),
    );
    let tree = inspect(
        &out,
        json!({ "maths": true, "textBoxes": true, "shapes": true }),
    );
    assert_eq!(tree["mathCount"], 1);
    assert_eq!(tree["sheets"][0]["mathCount"], 1);
    assert_eq!(tree["textBoxCount"], 0);
    assert_eq!(tree["sheets"][0]["textBoxes"], json!([]));
    assert_eq!(tree["sheets"][0]["shapes"], json!([]));
    let item = first_math(&tree);
    assert_eq!(item["text"], "E=mc^2");
    assert_eq!(item["from"], "B2");
    assert_eq!(item["stable"], true);
    assert!(item["id"]
        .as_str()
        .unwrap()
        .starts_with("sheet:Sheet1/math:"));
    let keys: Vec<&str> = item
        .as_object()
        .unwrap()
        .keys()
        .map(|k| k.as_str())
        .collect();
    assert_eq!(keys.last(), Some(&"stable"));
    assert!(keys.contains(&"text"));
    let drawing = part_text(&out, "xl/drawings/drawing1.xml");
    assert!(drawing.contains("a14:m"), "{drawing}");
    assert!(drawing.contains("Requires=\"a14\""), "{drawing}");
    assert!(drawing.contains("E=mc^2"), "{drawing}");
    assert!(!drawing.contains("mathplaceholder"), "{drawing}");
    let id = item["id"].as_str().unwrap().to_string();

    let replaced = seq_path("set", "xlsx");
    apply(
        &out,
        &replaced,
        json!([{ "op": "setMath", "id": id, "text": "a^2+b^2=c^2" }]),
    );
    let tree_s = inspect(&replaced, json!({ "maths": true }));
    let item_s = first_math(&tree_s);
    assert_eq!(item_s["id"], id);
    assert_eq!(item_s["text"], "a^2+b^2=c^2");

    let deleted = seq_path("del", "xlsx");
    apply(
        &replaced,
        &deleted,
        json!([{ "op": "deleteMath", "id": id }]),
    );
    let tree_d = inspect(&deleted, json!({ "maths": true, "textBoxes": true }));
    assert_eq!(tree_d["mathCount"], 0);
    assert_eq!(tree_d["sheets"][0]["maths"], json!([]));
}

#[test]
fn create_writes_math() {
    let path = create_book(
        "create",
        json!({
            "name": "Sheet1",
            "cells": [{ "ref": "A1", "value": 1 }],
            "maths": [{ "text": "E=mc^2", "from": "B2" }]
        }),
    );
    let tree = inspect(&path, json!({ "maths": true, "textBoxes": true }));
    assert_eq!(tree["mathCount"], 1);
    assert_eq!(first_math(&tree)["text"], "E=mc^2");
    assert_eq!(tree["sheets"][0]["textBoxes"], json!([]));
}

#[test]
fn reject_expr_and_empty_text() {
    let src = create_book(
        "rej",
        json!({ "name": "Sheet1", "cells": [{ "ref": "A1", "value": 1 }] }),
    );
    let out = seq_path("should-not", "xlsx");
    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": src.to_string_lossy(),
            "output": out.to_string_lossy(),
            "ops": [{
                "op": "insertMath",
                "id": "sheet:Sheet1",
                "text": "x",
                "expr": { "type": "text", "text": "x" }
            }]
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());

    let err_empty = run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": src.to_string_lossy(),
            "output": out.to_string_lossy(),
            "ops": [{
                "op": "insertMath",
                "id": "sheet:Sheet1",
                "text": ""
            }]
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err_empty.code(), ErrorCode::InvalidField);
    assert!(!out.exists());
}
