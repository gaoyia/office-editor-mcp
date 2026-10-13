//! docx 正文 3D 模型：inspect、insert / replace / delete、create、exportMedia。
//! 不进 pictures。不是 GLB 报 InvalidField。

use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, sample_glb, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::ZipArchive;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-docx-model3d");
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

fn create_docx(tag: &str, document: Value) -> PathBuf {
    let path = seq_path(tag, "docx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "format": "docx",
            "document": document
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
    let paras = tree["paragraphs"].as_array().unwrap();
    for p in paras {
        if let Some(items) = p["model3ds"].as_array() {
            if let Some(item) = items.first() {
                return item;
            }
        }
    }
    panic!("树上没有 3D 模型：{tree}");
}

fn glb_b() -> Vec<u8> {
    let mut b = sample_glb();
    let n = b.len();
    b[n - 4] ^= 1;
    b
}

#[test]
fn empty_file_count_is_zero() {
    let path = create_docx("empty", json!({ "paragraphs": [{ "text": "hello" }] }));
    let tree = inspect(&path, json!({}));
    assert_eq!(tree["model3dCount"], 0);
    assert!(tree.get("model3ds").is_none());
    let sliced = inspect(&path, json!({ "from": 1, "maxParas": 8, "model3ds": true }));
    assert_eq!(sliced["paragraphs"][0]["model3dCount"], 0);
    assert_eq!(sliced["paragraphs"][0]["model3ds"], json!([]));
}

#[test]
fn insert_inspect_export_replace_delete() {
    let src = create_docx("host", json!({ "paragraphs": [{ "text": "hello" }] }));
    let tree0 = inspect(&src, json!({ "from": 1, "maxParas": 8 }));
    let para = tree0["paragraphs"][0]["id"].as_str().unwrap().to_string();
    let glb = write_glb("a", &sample_glb());
    let inserted = seq_path("inserted", "docx");
    apply(
        &src,
        &inserted,
        json!([{
            "op": "insertModel3d",
            "id": para,
            "file": glb.to_string_lossy()
        }]),
    );
    let tree1 = inspect(
        &inserted,
        json!({ "from": 1, "maxParas": 8, "model3ds": true, "pictures": true }),
    );
    assert_eq!(tree1["model3dCount"], 1);
    assert!(tree1.get("model3ds").is_none());
    assert_eq!(tree1["pictureCount"], 0);
    assert_eq!(tree1["paragraphs"][0]["pictureCount"], 0);
    assert_eq!(tree1["paragraphs"][0]["model3dCount"], 1);
    let item = first_model(&tree1);
    assert_eq!(item["contentType"], "model/gltf.binary");
    assert_eq!(item["kind"], "inline");
    assert_eq!(item["cx"], 914_400);
    assert_eq!(item["cy"], 914_400);
    assert_eq!(item["stable"], true);
    assert!(item["id"].as_str().unwrap().contains("/model3d:"));
    let keys: Vec<&str> = item
        .as_object()
        .unwrap()
        .keys()
        .map(|k| k.as_str())
        .collect();
    assert_eq!(
        keys,
        [
            "id",
            "name",
            "kind",
            "cx",
            "cy",
            "wrap",
            "contentType",
            "stable",
            "hyperlink"
        ]
    );
    let names = zip_names(&inserted);
    assert!(names.iter().any(|n| n.ends_with(".glb")), "{names:?}");
    let doc = part_text(&inserted, "word/document.xml");
    assert!(doc.contains("am3d:model3d"), "{doc}");
    assert!(doc.contains("Requires=\"am3d\""), "{doc}");
    let ct = part_text(&inserted, "[Content_Types].xml");
    assert!(ct.contains("model/gltf.binary"), "{ct}");

    let id = item["id"].as_str().unwrap().to_string();
    let exported = seq_path("export", "glb");
    let RunOutput::Written { content_type, .. } = export_media(&inserted, &id, &exported) else {
        panic!("exportMedia 应写出文件");
    };
    assert_eq!(content_type.as_deref(), Some("model/gltf.binary"));
    assert_eq!(std::fs::read(&exported).unwrap(), sample_glb());

    let glb2 = write_glb("b", &glb_b());
    let replaced = seq_path("replaced", "docx");
    apply(
        &inserted,
        &replaced,
        json!([{ "op": "replaceModel3d", "id": id, "file": glb2.to_string_lossy() }]),
    );
    let tree_r = inspect(
        &replaced,
        json!({ "from": 1, "maxParas": 8, "model3ds": true }),
    );
    let id2 = first_model(&tree_r)["id"].as_str().unwrap().to_string();
    assert_eq!(id2, id);
    let exp2 = seq_path("exp2", "glb");
    export_media(&replaced, &id2, &exp2);
    assert_eq!(std::fs::read(&exp2).unwrap(), glb_b());

    let deleted = seq_path("deleted", "docx");
    apply(
        &replaced,
        &deleted,
        json!([{ "op": "deleteModel3d", "id": id }]),
    );
    let tree_d = inspect(
        &deleted,
        json!({ "from": 1, "maxParas": 8, "model3ds": true }),
    );
    assert_eq!(tree_d["model3dCount"], 0);
    assert_eq!(tree_d["paragraphs"][0]["model3dCount"], 0);
    assert!(!zip_names(&deleted).iter().any(|n| n.ends_with(".glb")));
}

#[test]
fn create_writes_glb() {
    let glb = write_glb("c", &sample_glb());
    let path = create_docx(
        "create",
        json!({
            "paragraphs": [{
                "text": "hello",
                "model3ds": [{ "file": glb.to_string_lossy() }]
            }]
        }),
    );
    let tree = inspect(
        &path,
        json!({ "from": 1, "maxParas": 8, "model3ds": true, "pictures": true }),
    );
    assert_eq!(tree["model3dCount"], 1);
    assert_eq!(first_model(&tree)["contentType"], "model/gltf.binary");
    assert_eq!(tree["paragraphs"][0]["pictures"], json!([]));
}

#[test]
fn reject_junk() {
    let src = create_docx("rej", json!({ "paragraphs": [{ "text": "hello" }] }));
    let tree0 = inspect(&src, json!({ "from": 1, "maxParas": 8 }));
    let para = tree0["paragraphs"][0]["id"].as_str().unwrap().to_string();
    let junk = write_glb("j", b"not a glb");
    let out = seq_path("should-not", "docx");
    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": src.to_string_lossy(),
            "output": out.to_string_lossy(),
            "ops": [{
                "op": "insertModel3d",
                "id": para,
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

fn apply_err(path: &Path, out: &Path, ops: Value) -> ooxp::Error {
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
    .unwrap_err()
}

fn insert_one(tag: &str) -> (PathBuf, String) {
    let src = create_docx(tag, json!({ "paragraphs": [{ "text": "hello" }] }));
    let tree0 = inspect(&src, json!({ "from": 1, "maxParas": 8 }));
    let para = tree0["paragraphs"][0]["id"].as_str().unwrap().to_string();
    let glb = write_glb(tag, &sample_glb());
    let inserted = seq_path(&format!("{tag}-ins"), "docx");
    apply(
        &src,
        &inserted,
        json!([{
            "op": "insertModel3d",
            "id": para,
            "file": glb.to_string_lossy()
        }]),
    );
    let tree = inspect(
        &inserted,
        json!({ "from": 1, "maxParas": 8, "model3ds": true }),
    );
    let id = first_model(&tree)["id"].as_str().unwrap().to_string();
    (inserted, id)
}

#[test]
fn set_size_and_wrap() {
    let (src, id) = insert_one("geom");
    let sized = seq_path("sized", "docx");
    apply(
        &src,
        &sized,
        json!([{ "op": "setModel3dSize", "id": id, "cx": 3657600, "cy": 1828800 }]),
    );
    let tree_s = inspect(
        &sized,
        json!({ "from": 1, "maxParas": 8, "model3ds": true }),
    );
    let item_s = first_model(&tree_s);
    assert_eq!(item_s["cx"], 3657600);
    assert_eq!(item_s["cy"], 1828800);
    assert_eq!(item_s["kind"], "inline");

    let wrapped = seq_path("wrapped", "docx");
    apply(
        &sized,
        &wrapped,
        json!([{
            "op": "setModel3dWrap",
            "id": id,
            "kind": "anchor",
            "wrap": "square",
            "alignH": "left",
            "alignV": "top"
        }]),
    );
    let tree_w = inspect(
        &wrapped,
        json!({ "from": 1, "maxParas": 8, "model3ds": true }),
    );
    let item_w = first_model(&tree_w);
    assert_eq!(item_w["kind"], "anchor");
    assert_eq!(item_w["wrap"], "square");
    assert_eq!(item_w["cx"], 3657600);

    let inlined = seq_path("inlined", "docx");
    apply(
        &wrapped,
        &inlined,
        json!([{ "op": "setModel3dWrap", "id": id, "kind": "inline" }]),
    );
    let tree_i = inspect(
        &inlined,
        json!({ "from": 1, "maxParas": 8, "model3ds": true }),
    );
    let item_i = first_model(&tree_i);
    assert_eq!(item_i["kind"], "inline");
    assert!(item_i["wrap"].is_null());

    let glb2 = write_glb("geom-b", &glb_b());
    let replaced = seq_path("geom-rep", "docx");
    apply(
        &sized,
        &replaced,
        json!([{ "op": "replaceModel3d", "id": id, "file": glb2.to_string_lossy() }]),
    );
    let tree_r = inspect(
        &replaced,
        json!({ "from": 1, "maxParas": 8, "model3ds": true }),
    );
    let item_r = first_model(&tree_r);
    assert_eq!(item_r["cx"], 3657600);
    assert_eq!(item_r["cy"], 1828800);
}

#[test]
fn header_set_size() {
    let src = create_docx(
        "hf",
        json!({
            "paragraphs": [{ "text": "body" }],
            "headers": [{ "type": "default", "paragraphs": [{ "text": "hdr" }] }]
        }),
    );
    let tree0 = inspect(&src, json!({ "headerFooter": true, "model3ds": true }));
    let para = tree0["headers"][0]["paragraphs"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let glb = write_glb("hf", &sample_glb());
    let inserted = seq_path("hf-ins", "docx");
    apply(
        &src,
        &inserted,
        json!([{ "op": "insertModel3d", "id": para, "file": glb.to_string_lossy() }]),
    );
    let tree1 = inspect(&inserted, json!({ "headerFooter": true, "model3ds": true }));
    let id = tree1["headers"][0]["model3ds"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let sized = seq_path("hf-sz", "docx");
    apply(
        &inserted,
        &sized,
        json!([{ "op": "setModel3dSize", "id": id, "cx": 3657600, "cy": 1828800 }]),
    );
    let tree_s = inspect(&sized, json!({ "headerFooter": true, "model3ds": true }));
    assert_eq!(tree_s["headers"][0]["model3ds"][0]["cx"], 3657600);
    assert_eq!(tree_s["model3dCount"], 0);
}

#[test]
fn reject_picture_op_and_empty_size() {
    let (src, id) = insert_one("rej-geom");
    let out_pic = seq_path("pic-on-3d", "docx");
    let err_pic = apply_err(
        &src,
        &out_pic,
        json!([{ "op": "setPictureSize", "id": id, "cx": 1000, "cy": 1000 }]),
    );
    assert_eq!(err_pic.code(), ErrorCode::InvalidId);
    assert!(!out_pic.exists());

    let out_empty = seq_path("empty-size", "docx");
    let err_empty = apply_err(
        &src,
        &out_empty,
        json!([{ "op": "setModel3dSize", "id": id }]),
    );
    assert_eq!(err_empty.code(), ErrorCode::InvalidField);
    assert!(!out_empty.exists());

    let xlsx = seq_path("xlsx-host", "xlsx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": xlsx.to_string_lossy(),
            "workbook": { "sheets": [{ "name": "Sheet1" }] }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let out_x = seq_path("xlsx-wrap", "xlsx");
    let err_x = apply_err(
        &xlsx,
        &out_x,
        json!([{ "op": "setModel3dWrap", "id": "sheet:Sheet1/model3d:1", "kind": "inline" }]),
    );
    assert_eq!(err_x.code(), ErrorCode::InvalidField);
    assert!(!out_x.exists());
}
