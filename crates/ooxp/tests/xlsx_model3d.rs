//! xlsx 工作表 3D 模型：inspect、insert / replace / delete、create、exportMedia。
//! 不进 pictures。不是 GLB 报 InvalidField。

use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, sample_glb, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::ZipArchive;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-xlsx-model3d");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn seq_path(tag: &str, ext: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}.{ext}"))
}

fn write_glb(tag: &str, bytes: &[u8]) -> PathBuf {
    let path = seq_path(tag, "glb");
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

fn first_model(tree: &Value) -> &Value {
    &tree["sheets"][0]["model3ds"][0]
}

fn glb_b() -> Vec<u8> {
    let mut b = sample_glb();
    let n = b.len();
    b[n - 4] ^= 1;
    b
}

#[test]
fn empty_file_count_is_zero() {
    let path = create_book(
        "empty",
        json!({ "name": "Sheet1", "cells": [{ "ref": "A1", "value": 1 }] }),
    );
    let tree = inspect(&path, json!({}));
    assert_eq!(tree["model3dCount"], 0);
    assert_eq!(tree["sheets"][0]["model3dCount"], 0);
    assert!(tree["sheets"][0].get("model3ds").is_none());
    let listed = inspect(&path, json!({ "model3ds": true }));
    assert_eq!(listed["sheets"][0]["model3ds"], json!([]));
}

#[test]
fn insert_inspect_export_replace_delete() {
    let src = create_book(
        "src",
        json!({ "name": "Sheet1", "cells": [{ "ref": "A1", "value": 1 }] }),
    );
    let glb = write_glb("in", &sample_glb());
    let out = seq_path("ins", "xlsx");
    apply(
        &src,
        &out,
        json!([{
            "op": "insertModel3d",
            "id": "sheet:Sheet1",
            "file": glb.to_string_lossy(),
            "from": "B2"
        }]),
    );
    let tree = inspect(&out, json!({ "model3ds": true, "pictures": true }));
    assert_eq!(tree["model3dCount"], 1);
    assert_eq!(tree["sheets"][0]["model3dCount"], 1);
    assert_eq!(tree["pictureCount"], 0);
    assert_eq!(tree["sheets"][0]["pictures"], json!([]));
    let item = first_model(&tree);
    assert_eq!(item["contentType"], "model/gltf.binary");
    assert_eq!(item["stable"], true);
    assert!(item["id"]
        .as_str()
        .unwrap()
        .starts_with("sheet:Sheet1/model3d:"));
    assert_eq!(item["cx"], 914_400);
    assert_eq!(item["cy"], 914_400);
    let keys: Vec<&str> = item
        .as_object()
        .unwrap()
        .keys()
        .map(|k| k.as_str())
        .collect();
    assert!(keys.contains(&"contentType"));
    assert_eq!(keys.last(), Some(&"stable"));
    let names = zip_names(&out);
    assert!(names.iter().any(|n| n.ends_with(".glb")), "{names:?}");
    let drawing = part_text(&out, "xl/drawings/drawing1.xml");
    assert!(drawing.contains("am3d:model3d"), "{drawing}");
    assert!(drawing.contains("Requires=\"am3d\""), "{drawing}");
    let ct = part_text(&out, "[Content_Types].xml");
    assert!(ct.contains("model/gltf.binary"), "{ct}");
    let id = item["id"].as_str().unwrap().to_string();
    let exp = seq_path("exp", "glb");
    let RunOutput::Written { content_type, .. } = export_media(&out, &id, &exp) else {
        panic!("export 应写出");
    };
    assert_eq!(content_type.as_deref(), Some("model/gltf.binary"));
    assert_eq!(std::fs::read(&exp).unwrap(), sample_glb());

    let glb2 = write_glb("b", &glb_b());
    let replaced = seq_path("rep", "xlsx");
    apply(
        &out,
        &replaced,
        json!([{ "op": "replaceModel3d", "id": id, "file": glb2.to_string_lossy() }]),
    );
    let tree_r = inspect(&replaced, json!({ "model3ds": true }));
    let id2 = first_model(&tree_r)["id"].as_str().unwrap().to_string();
    assert_eq!(id2, id);
    let exp2 = seq_path("exp2", "glb");
    export_media(&replaced, &id2, &exp2);
    assert_eq!(std::fs::read(&exp2).unwrap(), glb_b());

    let deleted = seq_path("del", "xlsx");
    apply(
        &replaced,
        &deleted,
        json!([{ "op": "deleteModel3d", "id": id }]),
    );
    let tree_d = inspect(&deleted, json!({ "model3ds": true, "pictures": true }));
    assert_eq!(tree_d["model3dCount"], 0);
    assert_eq!(tree_d["sheets"][0]["model3ds"], json!([]));
    assert!(!zip_names(&deleted).iter().any(|n| n.ends_with(".glb")));
}

#[test]
fn create_writes_glb() {
    let glb = write_glb("c", &sample_glb());
    let path = create_book(
        "create",
        json!({
            "name": "Sheet1",
            "cells": [{ "ref": "A1", "value": 1 }],
            "model3ds": [{ "file": glb.to_string_lossy(), "from": "B2" }]
        }),
    );
    let tree = inspect(&path, json!({ "model3ds": true, "pictures": true }));
    assert_eq!(tree["model3dCount"], 1);
    assert_eq!(first_model(&tree)["contentType"], "model/gltf.binary");
    assert_eq!(tree["sheets"][0]["pictures"], json!([]));
}

#[test]
fn reject_junk() {
    let src = create_book(
        "rej",
        json!({ "name": "Sheet1", "cells": [{ "ref": "A1", "value": 1 }] }),
    );
    let junk = write_glb("j", b"not a glb");
    let out = seq_path("should-not", "xlsx");
    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": src.to_string_lossy(),
            "output": out.to_string_lossy(),
            "ops": [{
                "op": "insertModel3d",
                "id": "sheet:Sheet1",
                "file": junk.to_string_lossy()
            }]
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());
}
