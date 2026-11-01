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
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "format": "docx",
        "document": document
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
    assert_eq!(tree1["model3ds"], json!([]));
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
            "camera",
            "rot",
            "lightRig",
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
    let err = run(&json!({
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
    .to_string())
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());
}

fn apply_err(path: &Path, out: &Path, ops: Value) -> ooxp::Error {
    run(&json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": path.to_string_lossy(),
        "output": out.to_string_lossy(),
        "ops": ops
    })
    .to_string())
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
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": xlsx.to_string_lossy(),
        "workbook": { "sheets": [{ "name": "Sheet1" }] }
    })
    .to_string())
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

fn camera_ops(id: &str) -> Value {
    json!([{
        "op": "setModel3dCamera",
        "id": id,
        "camera": { "pos": { "x": 50400000 }, "fov": 1892375 },
        "rot": { "ax": 1060962, "ay": -532862, "az": -169045 },
        "lightRig": { "rig": "freezing", "dir": "t", "rev": 18000000 }
    }])
}

fn assert_default_view(item: &Value) {
    assert_eq!(item["camera"]["pos"]["z"], 66519324);
    assert_eq!(item["camera"]["fov"], 2700000);
    assert!(item["rot"]["ax"].is_null());
    assert!(item["rot"]["ay"].is_null());
    assert!(item["rot"]["az"].is_null());
    assert!(item["lightRig"].is_null());
}

fn assert_patched_view(item: &Value) {
    assert_eq!(item["camera"]["pos"]["x"], 50400000);
    assert_eq!(item["camera"]["pos"]["z"], 66519324);
    assert_eq!(item["camera"]["fov"], 1892375);
    assert_eq!(item["rot"]["ax"], 1060962);
    assert_eq!(item["rot"]["ay"], -532862);
    assert_eq!(item["rot"]["az"], -169045);
    assert_eq!(item["lightRig"]["rig"], "freezing");
    assert_eq!(item["lightRig"]["dir"], "t");
    assert_eq!(item["lightRig"]["lat"], 0);
    assert_eq!(item["lightRig"]["rev"], 18000000);
}

#[test]
fn set_camera_rot_and_light() {
    let (src, id) = insert_one("cam");
    let tree0 = inspect(&src, json!({ "from": 1, "maxParas": 8, "model3ds": true }));
    assert_default_view(first_model(&tree0));

    let patched = seq_path("cam-set", "docx");
    apply(&src, &patched, camera_ops(&id));
    let tree = inspect(
        &patched,
        json!({ "from": 1, "maxParas": 8, "model3ds": true }),
    );
    assert_patched_view(first_model(&tree));
    assert!(part_text(&patched, "word/document.xml").contains("rev=\"18000000\""));

    let cleared = seq_path("cam-clear", "docx");
    apply(
        &patched,
        &cleared,
        json!([{ "op": "setModel3dCamera", "id": id, "lightRig": null }]),
    );
    let tree_c = inspect(
        &cleared,
        json!({ "from": 1, "maxParas": 8, "model3ds": true }),
    );
    let item_c = first_model(&tree_c);
    assert!(item_c["lightRig"].is_null());
    assert_eq!(item_c["camera"]["fov"], 1892375);
}

#[test]
fn header_set_camera() {
    let src = create_docx(
        "hf-cam",
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
    let glb = write_glb("hf-cam", &sample_glb());
    let inserted = seq_path("hf-cam-ins", "docx");
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
    let out = seq_path("hf-cam-fov", "docx");
    apply(
        &inserted,
        &out,
        json!([{ "op": "setModel3dCamera", "id": id, "camera": { "fov": 1892375 } }]),
    );
    let tree = inspect(&out, json!({ "headerFooter": true, "model3ds": true }));
    assert_eq!(tree["headers"][0]["model3ds"][0]["camera"]["fov"], 1892375);
    assert_eq!(tree["model3dCount"], 0);
}

#[test]
fn reject_empty_camera_and_neg_fov() {
    let (src, id) = insert_one("rej-cam");
    let out_empty = seq_path("empty-cam", "docx");
    let err_empty = apply_err(
        &src,
        &out_empty,
        json!([{ "op": "setModel3dCamera", "id": id }]),
    );
    assert_eq!(err_empty.code(), ErrorCode::InvalidField);
    assert!(!out_empty.exists());

    let out_fov = seq_path("neg-fov", "docx");
    let err_fov = apply_err(
        &src,
        &out_fov,
        json!([{ "op": "setModel3dCamera", "id": id, "camera": { "fov": -1 } }]),
    );
    assert_eq!(err_fov.code(), ErrorCode::InvalidField);
    assert!(!out_fov.exists());
}

#[test]
fn insert_anchor_writes_wrap_on_choice_and_fallback() {
    let src = create_docx("float", json!({ "paragraphs": [{ "text": "hello" }] }));
    let tree0 = inspect(&src, json!({ "from": 1, "maxParas": 4 }));
    let para = tree0["paragraphs"][0]["id"].as_str().unwrap();
    let glb = write_glb("float", &sample_glb());
    let out = seq_path("float-ins", "docx");
    apply(
        &src,
        &out,
        json!([{
            "op": "insertModel3d",
            "id": para,
            "file": glb.to_string_lossy(),
            "kind": "anchor",
            "wrap": "square",
            "alignH": "left",
            "alignV": "top"
        }]),
    );
    let tree = inspect(&out, json!({ "from": 1, "maxParas": 4, "model3ds": true }));
    let item = first_model(&tree);
    assert_eq!(item["kind"], "anchor");
    assert_eq!(item["wrap"], "square");
    let doc = part_text(&out, "word/document.xml");
    assert!(doc.contains("wp:anchor"), "{doc}");
    assert!(doc.contains("wrapSquare"), "{doc}");
    assert!(doc.contains(r#"relativeFrom="margin""#), "{doc}");
    assert_eq!(doc.matches("<wp:anchor ").count(), 2, "{doc}");
    let err = apply_err(
        &src,
        &seq_path("float-bad", "docx"),
        json!([{
            "op": "insertModel3d",
            "id": para,
            "file": glb.to_string_lossy(),
            "kind": "inline",
            "wrap": "square"
        }]),
    );
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

#[test]
fn notes_and_comments_list_inline_models_and_reject_float() {
    let src = create_docx("note-host", json!({ "paragraphs": [{ "text": "正文" }] }));
    let tree0 = inspect(&src, json!({ "from": 1, "maxParas": 4 }));
    let para = tree0["paragraphs"][0]["id"].as_str().unwrap();
    let hosted = seq_path("note-hosted", "docx");
    apply(
        &src,
        &hosted,
        json!([
            { "op": "insertFootnote", "id": para, "text": "注" },
            { "op": "insertEndnote", "id": para, "text": "尾" },
            { "op": "insertComment", "start": para, "end": para, "text": "批" }
        ]),
    );
    let tree = inspect(
        &hosted,
        json!({ "from": 1, "maxParas": 4, "notes": true, "comments": true }),
    );
    let foot = tree["footnotes"][0]["paragraphs"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let end = tree["endnotes"][0]["paragraphs"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let comment = tree["comments"][0]["paragraphs"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let glb = write_glb("note", &sample_glb());
    let inserted = seq_path("note-m3d", "docx");
    apply(
        &hosted,
        &inserted,
        json!([
            { "op": "insertModel3d", "id": foot, "file": glb.to_string_lossy() },
            { "op": "insertModel3d", "id": end, "file": glb.to_string_lossy() },
            { "op": "insertModel3d", "id": comment, "file": glb.to_string_lossy() }
        ]),
    );
    let listed = inspect(
        &inserted,
        json!({
            "from": 1,
            "maxParas": 4,
            "notes": true,
            "comments": true,
            "model3ds": true,
            "pictures": true
        }),
    );
    assert_eq!(listed["model3dCount"], 3);
    assert_eq!(listed["pictureCount"], 0);
    assert_eq!(listed["footnotes"][0]["pictureCount"], 0);
    let ids: Vec<_> = listed["model3ds"]
        .as_array()
        .unwrap()
        .iter()
        .map(|o| o["id"].as_str().unwrap().to_string())
        .collect();
    assert!(ids.iter().any(|id| id.starts_with("footnote:")), "{ids:?}");
    assert!(ids.iter().any(|id| id.starts_with("endnote:")), "{ids:?}");
    assert!(ids.iter().any(|id| id.starts_with("comment:")), "{ids:?}");
    assert!(ids.iter().all(|id| id.contains("/model3d:")), "{ids:?}");
    for item in listed["model3ds"].as_array().unwrap() {
        assert_eq!(item["kind"], "inline");
        assert_eq!(item["contentType"], "model/gltf.binary");
    }
    let notes = part_text(&inserted, "word/footnotes.xml");
    assert!(notes.contains("Requires=\"am3d\""), "{notes}");
    assert!(notes.contains("<wp:inline"), "{notes}");
    assert!(!notes.contains("<wp:anchor"), "{notes}");
    let rels = part_text(&inserted, "word/_rels/footnotes.xml.rels");
    assert!(
        rels.contains("http://schemas.microsoft.com/office/2017/06/relationships/model3d"),
        "{rels}"
    );
    assert!(rels.contains("media/model3d"), "{rels}");
    let ct = part_text(&inserted, "[Content_Types].xml");
    assert!(ct.contains("model/gltf.binary"), "{ct}");

    let foot_id = ids.iter().find(|id| id.starts_with("footnote:")).unwrap();
    let exported = seq_path("note-export", "glb");
    let RunOutput::Written { content_type, .. } = export_media(&inserted, foot_id, &exported)
    else {
        panic!("exportMedia 应写出脚注里的 GLB");
    };
    assert_eq!(content_type.as_deref(), Some("model/gltf.binary"));
    assert_eq!(std::fs::read(&exported).unwrap(), sample_glb());

    let glb2 = write_glb("note-b", &glb_b());
    let replaced = seq_path("note-replaced", "docx");
    apply(
        &inserted,
        &replaced,
        json!([{ "op": "replaceModel3d", "id": foot_id, "file": glb2.to_string_lossy() }]),
    );
    let exp2 = seq_path("note-exp2", "glb");
    export_media(&replaced, foot_id, &exp2);
    assert_eq!(std::fs::read(&exp2).unwrap(), glb_b());

    let sized = seq_path("note-sized", "docx");
    apply(
        &replaced,
        &sized,
        json!([{ "op": "setModel3dSize", "id": foot_id, "cx": 3_657_600, "cy": 1_828_800 }]),
    );
    let sized_tree = inspect(&sized, json!({ "model3ds": true }));
    let sized_item = sized_tree["model3ds"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["id"].as_str() == Some(foot_id.as_str()))
        .unwrap();
    assert_eq!(sized_item["cx"], 3_657_600);
    assert_eq!(sized_item["cy"], 1_828_800);

    let cam = seq_path("note-cam", "docx");
    apply(
        &sized,
        &cam,
        json!([{ "op": "setModel3dCamera", "id": foot_id, "camera": { "fov": 1892375 } }]),
    );
    let cam_tree = inspect(&cam, json!({ "model3ds": true }));
    let cam_item = cam_tree["model3ds"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["id"].as_str() == Some(foot_id.as_str()))
        .unwrap();
    assert_eq!(cam_item["camera"]["fov"], 1892375);

    let comment_id = ids.iter().find(|id| id.starts_with("comment:")).unwrap();
    let deleted = seq_path("note-del", "docx");
    apply(
        &cam,
        &deleted,
        json!([{ "op": "deleteModel3d", "id": comment_id }]),
    );
    let after = inspect(&deleted, json!({ "model3ds": true }));
    assert_eq!(after["model3dCount"], 2);
    let left: Vec<_> = after["model3ds"]
        .as_array()
        .unwrap()
        .iter()
        .map(|o| o["id"].as_str().unwrap().to_string())
        .collect();
    assert!(
        !left.iter().any(|id| id.starts_with("comment:")),
        "{left:?}"
    );
    let comments = part_text(&deleted, "word/comments.xml");
    assert!(!comments.contains("am3d:model3d"), "{comments}");

    let floated = seq_path("note-float", "docx");
    let err = apply_err(
        &hosted,
        &floated,
        json!([{
            "op": "insertModel3d",
            "id": foot,
            "file": glb.to_string_lossy(),
            "kind": "anchor",
            "wrap": "square"
        }]),
    );
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!floated.exists());
    let wrap_err = apply_err(
        &inserted,
        &seq_path("note-wrap", "docx"),
        json!([{
            "op": "setModel3dWrap",
            "id": foot_id,
            "kind": "anchor",
            "wrap": "square"
        }]),
    );
    assert_eq!(wrap_err.code(), ErrorCode::InvalidField);

    let lone_src = create_docx("lone-host", json!({ "paragraphs": [{ "text": "正文" }] }));
    let lone0 = inspect(&lone_src, json!({ "from": 1, "maxParas": 4 }));
    let lone_para = lone0["paragraphs"][0]["id"].as_str().unwrap();
    let lone_hosted = seq_path("lone-hosted", "docx");
    apply(
        &lone_src,
        &lone_hosted,
        json!([{ "op": "insertFootnote", "id": lone_para, "text": "注" }]),
    );
    let lone_tree = inspect(&lone_hosted, json!({ "notes": true }));
    let lone_foot = lone_tree["footnotes"][0]["paragraphs"][0]["id"]
        .as_str()
        .unwrap();
    let lone_ins = seq_path("lone-ins", "docx");
    apply(
        &lone_hosted,
        &lone_ins,
        json!([{ "op": "insertModel3d", "id": lone_foot, "file": glb.to_string_lossy() }]),
    );
    let lone_listed = inspect(&lone_ins, json!({ "model3ds": true }));
    let lone_id = lone_listed["model3ds"][0]["id"].as_str().unwrap();
    let lone_del = seq_path("lone-del", "docx");
    apply(
        &lone_ins,
        &lone_del,
        json!([{ "op": "deleteModel3d", "id": lone_id }]),
    );
    let lone_after = inspect(&lone_del, json!({ "model3ds": true }));
    assert_eq!(lone_after["model3dCount"], 0);
    assert!(!zip_names(&lone_del).iter().any(|n| n.ends_with(".glb")));
    let lone_rels = part_text(&lone_del, "word/_rels/footnotes.xml.rels");
    assert!(!lone_rels.contains("relationships/model3d"), "{lone_rels}");
}

#[test]
fn a_hand_written_note_model_without_docpr_uses_index() {
    let src = create_docx("hand-host", json!({ "paragraphs": [{ "text": "正文" }] }));
    let tree0 = inspect(&src, json!({ "from": 1, "maxParas": 4 }));
    let para = tree0["paragraphs"][0]["id"].as_str().unwrap();
    let hosted = seq_path("hand-hosted", "docx");
    apply(
        &src,
        &hosted,
        json!([{ "op": "insertFootnote", "id": para, "text": "注" }]),
    );
    let packed = seq_path("hand-packed", "docx");
    inject_note_model(&hosted, &packed);
    let tree = inspect(
        &packed,
        json!({ "notes": true, "model3ds": true, "pictures": true }),
    );
    assert_eq!(tree["model3dCount"], 1);
    assert_eq!(tree["pictureCount"], 0);
    let item = &tree["model3ds"][0];
    let id = item["id"].as_str().unwrap();
    assert!(id.starts_with("footnote:"), "{id}");
    assert!(id.ends_with("/model3d:index:1"), "{id}");
    assert_eq!(item["stable"], false);
    assert_eq!(item["kind"], "inline");
    assert_eq!(item["name"], "3D 模型 1");
    assert_eq!(item["cx"], 2_090_761);
}

fn inject_note_model(src: &Path, dest: &Path) {
    use std::io::{Cursor, Read, Write};
    use zip::write::SimpleFileOptions;
    use zip::{CompressionMethod, ZipArchive, ZipWriter};
    let drawing = r#"<w:r><mc:AlternateContent><mc:Choice Requires="am3d"><w:drawing><wp:inline distT="0" distB="0" distL="0" distR="0"><wp:extent cx="2090761" cy="2090761"/><wp:effectExtent l="0" t="0" r="0" b="0"/><wp:docPr name="3D 模型 1"/><a:graphic><a:graphicData uri="http://schemas.microsoft.com/office/drawing/2017/model3d"><am3d:model3d r:embed="rId1"/></a:graphicData></a:graphic></wp:inline></w:drawing></mc:Choice><mc:Fallback><w:drawing><wp:inline><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/picture"><pic:pic><pic:blipFill><a:blip r:embed="rId2"/></pic:blipFill></pic:pic></a:graphicData></a:graphic></wp:inline></w:drawing></mc:Fallback></mc:AlternateContent></w:r>"#;
    let bytes = std::fs::read(src).unwrap();
    let mut zin = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut buf = Cursor::new(Vec::new());
    {
        let mut zout = ZipWriter::new(&mut buf);
        let opt = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        for i in 0..zin.len() {
            let mut file = zin.by_index(i).unwrap();
            let name = file.name().to_string();
            let mut data = Vec::new();
            file.read_to_end(&mut data).unwrap();
            if name == "word/footnotes.xml" {
                let xml = String::from_utf8(data).unwrap();
                let end = xml.rfind("</w:p>").expect("footnote para");
                data = format!("{}{drawing}{}", &xml[..end], &xml[end..]).into_bytes();
            }
            zout.start_file(name, opt).unwrap();
            zout.write_all(&data).unwrap();
        }
        zout.finish().unwrap();
    }
    std::fs::write(dest, buf.into_inner()).unwrap();
}
