//! docx 墨迹画布：inspect、insert / replace / delete、尺寸与绕排、create、exportMedia；xlsx / pptx 拒这些 op。

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
    let dir = std::env::temp_dir().join("ooxp-docx-ink");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn seq_path(tag: &str, ext: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}.{ext}"))
}

fn write_ink(tag: &str, body: &str) -> PathBuf {
    let path = seq_path(tag, "xml");
    std::fs::write(&path, body.as_bytes()).unwrap();
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

fn inspect_tree(path: &Path, scope: Value) -> Value {
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

fn apply_err(path: &Path, ops: Value) -> ooxp::Error {
    let out = seq_path("should-not-write", "docx");
    let _ = std::fs::remove_file(&out);
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

fn part_text(path: &Path, name: &str) -> String {
    let bytes = std::fs::read(path).unwrap();
    let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut file = zip.by_name(name.trim_start_matches('/')).unwrap();
    let mut s = String::new();
    file.read_to_string(&mut s).unwrap();
    s
}

fn zip_names(path: &Path) -> Vec<String> {
    let bytes = std::fs::read(path).unwrap();
    let zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    zip.file_names().map(|s| s.to_string()).collect()
}

fn first_ink(tree: &Value) -> &Value {
    let paras = tree["paragraphs"].as_array().unwrap();
    for p in paras {
        if let Some(items) = p["inkCanvases"].as_array() {
            if let Some(item) = items.first() {
                return item;
            }
        }
    }
    panic!("树上没有墨迹画布：{tree}");
}

#[test]
fn empty_file_count_is_zero() {
    let path = create_docx("empty", json!({ "paragraphs": [{ "text": "hello" }] }));
    let tree = inspect_tree(&path, json!({}));
    assert_eq!(tree["inkCanvasCount"], 0);
    assert!(tree.get("inkCanvases").is_none());
    let sliced = inspect_tree(
        &path,
        json!({ "from": 1, "maxParas": 8, "inkCanvases": true }),
    );
    assert_eq!(sliced["paragraphs"][0]["inkCanvasCount"], 0);
    assert_eq!(sliced["paragraphs"][0]["inkCanvases"], json!([]));
}

#[test]
fn insert_inspect_export_replace_delete() {
    let src = create_docx("host", json!({ "paragraphs": [{ "text": "hello" }] }));
    let tree0 = inspect_tree(&src, json!({ "from": 1, "maxParas": 8 }));
    let para = tree0["paragraphs"][0]["id"].as_str().unwrap().to_string();
    let ink = write_ink("a", INKML);
    let inserted = seq_path("inserted", "docx");
    apply(
        &src,
        &inserted,
        json!([{
            "op": "insertInkCanvas",
            "id": para,
            "file": ink.to_string_lossy()
        }]),
    );
    let tree1 = inspect_tree(
        &inserted,
        json!({ "from": 1, "maxParas": 8, "inkCanvases": true }),
    );
    assert_eq!(tree1["inkCanvasCount"], 1);
    assert!(tree1.get("inkCanvases").is_none());
    assert_eq!(tree1["paragraphs"][0]["inkCanvasCount"], 1);
    let item = first_ink(&tree1);
    assert_eq!(item["contentType"], "application/inkml+xml");
    assert_eq!(item["kind"], "inline");
    assert_eq!(item["cx"], 1_828_800);
    assert_eq!(item["cy"], 914_400);
    assert_eq!(item["stable"], true);
    assert!(item["id"].as_str().unwrap().contains("/ink:"));
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
    assert!(
        names
            .iter()
            .any(|n| n.replace('\\', "/").contains("word/ink/")),
        "{names:?}"
    );
    let ct = part_text(&inserted, "[Content_Types].xml");
    assert!(ct.contains("application/inkml+xml"), "{ct}");
    let doc = part_text(&inserted, "word/document.xml");
    assert!(doc.contains("wpc:wpc"), "{doc}");
    assert!(doc.contains("w14:contentPart"), "{doc}");
    assert!(doc.contains("Requires=\"wpc\""), "{doc}");
    assert!(!doc.contains("xmlns:w15"), "{doc}");

    let ink_id = item["id"].as_str().unwrap().to_string();
    let exported = seq_path("export", "xml");
    let result = export_media(&inserted, &ink_id, &exported);
    let RunOutput::Written { content_type, .. } = result else {
        panic!("exportMedia 应写出文件");
    };
    assert_eq!(content_type.as_deref(), Some("application/inkml+xml"));
    assert_eq!(std::fs::read(&exported).unwrap(), INKML.as_bytes());

    let ink_b = write_ink("b", INKML_B);
    let replaced = seq_path("replaced", "docx");
    apply(
        &inserted,
        &replaced,
        json!([{
            "op": "replaceInkCanvas",
            "id": ink_id,
            "file": ink_b.to_string_lossy()
        }]),
    );
    let tree2 = inspect_tree(
        &replaced,
        json!({ "from": 1, "maxParas": 8, "inkCanvases": true }),
    );
    let item2 = first_ink(&tree2);
    let ink_id2 = item2["id"].as_str().unwrap().to_string();
    let exported2 = seq_path("export2", "xml");
    export_media(&replaced, &ink_id2, &exported2);
    assert_eq!(std::fs::read(&exported2).unwrap(), INKML_B.as_bytes());

    let deleted = seq_path("deleted", "docx");
    apply(
        &replaced,
        &deleted,
        json!([{ "op": "deleteInkCanvas", "id": ink_id2 }]),
    );
    let tree3 = inspect_tree(
        &deleted,
        json!({ "from": 1, "maxParas": 8, "inkCanvases": true }),
    );
    assert_eq!(tree3["inkCanvasCount"], 0);
    assert_eq!(tree3["paragraphs"][0]["inkCanvasCount"], 0);
    let names = zip_names(&deleted);
    assert!(
        !names
            .iter()
            .any(|n| n.replace('\\', "/").contains("word/ink/")),
        "{names:?}"
    );
}

#[test]
fn set_size_wrap_and_replace_keeps_geom() {
    let src = create_docx("geom-host", json!({ "paragraphs": [{ "text": "hello" }] }));
    let tree0 = inspect_tree(&src, json!({ "from": 1, "maxParas": 8 }));
    let para = tree0["paragraphs"][0]["id"].as_str().unwrap();
    let ink = write_ink("geom", INKML);
    let inserted = seq_path("geom-ins", "docx");
    apply(
        &src,
        &inserted,
        json!([{
            "op": "insertInkCanvas",
            "id": para,
            "file": ink.to_string_lossy()
        }]),
    );
    let tree1 = inspect_tree(
        &inserted,
        json!({ "from": 1, "maxParas": 8, "inkCanvases": true }),
    );
    let ink_id = first_ink(&tree1)["id"].as_str().unwrap().to_string();
    assert_eq!(first_ink(&tree1)["cx"], 1_828_800);
    assert_eq!(first_ink(&tree1)["cy"], 914_400);
    assert_eq!(first_ink(&tree1)["kind"], "inline");

    let sized = seq_path("geom-size", "docx");
    apply(
        &inserted,
        &sized,
        json!([{
            "op": "setInkCanvasSize",
            "id": ink_id,
            "cx": 3_657_600,
            "cy": 1_828_800
        }]),
    );
    let tree2 = inspect_tree(
        &sized,
        json!({ "from": 1, "maxParas": 8, "inkCanvases": true }),
    );
    let item2 = first_ink(&tree2);
    assert_eq!(item2["cx"], 3_657_600);
    assert_eq!(item2["cy"], 1_828_800);
    let xml2 = part_text(&sized, "word/document.xml");
    let extent_hits = xml2.matches("cx=\"3657600\"").count();
    assert!(extent_hits >= 2, "wp:extent 与 a:ext 都应写入 cx：{xml2}");

    let wrapped = seq_path("geom-wrap", "docx");
    apply(
        &sized,
        &wrapped,
        json!([{
            "op": "setInkCanvasWrap",
            "id": ink_id,
            "kind": "anchor",
            "wrap": "square",
            "alignH": "left",
            "alignV": "top"
        }]),
    );
    let tree3 = inspect_tree(
        &wrapped,
        json!({ "from": 1, "maxParas": 8, "inkCanvases": true }),
    );
    let item3 = first_ink(&tree3);
    assert_eq!(item3["kind"], "anchor");
    assert_eq!(item3["wrap"], "square");
    assert_eq!(item3["cx"], 3_657_600);

    let inline = seq_path("geom-inline", "docx");
    apply(
        &wrapped,
        &inline,
        json!([{
            "op": "setInkCanvasWrap",
            "id": ink_id,
            "kind": "inline"
        }]),
    );
    let tree4 = inspect_tree(
        &inline,
        json!({ "from": 1, "maxParas": 8, "inkCanvases": true }),
    );
    let item4 = first_ink(&tree4);
    assert_eq!(item4["kind"], "inline");
    assert_eq!(item4["wrap"], Value::Null);

    let ink_b = write_ink("geom-b", INKML_B);
    let replaced = seq_path("geom-rep", "docx");
    apply(
        &sized,
        &replaced,
        json!([{
            "op": "replaceInkCanvas",
            "id": ink_id,
            "file": ink_b.to_string_lossy()
        }]),
    );
    let tree5 = inspect_tree(
        &replaced,
        json!({ "from": 1, "maxParas": 8, "inkCanvases": true }),
    );
    let item5 = first_ink(&tree5);
    assert_eq!(item5["cx"], 3_657_600);
    assert_eq!(item5["cy"], 1_828_800);
    assert_eq!(item5["kind"], "inline");

    let err = apply_err(
        &inserted,
        json!([{ "op": "setInkCanvasSize", "id": ink_id }]),
    );
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let err = apply_err(
        &inserted,
        json!([{
            "op": "setPictureSize",
            "id": ink_id,
            "cx": 1000
        }]),
    );
    assert_eq!(err.code(), ErrorCode::InvalidId);
}

#[test]
fn create_writes_ink_part() {
    let ink = write_ink("create", INKML);
    let path = create_docx(
        "create",
        json!({
            "paragraphs": [{
                "text": "hello",
                "inkCanvases": [{ "file": ink.to_string_lossy() }]
            }]
        }),
    );
    let tree = inspect_tree(
        &path,
        json!({ "from": 1, "maxParas": 8, "inkCanvases": true }),
    );
    assert_eq!(tree["inkCanvasCount"], 1);
    let item = first_ink(&tree);
    assert_eq!(item["contentType"], "application/inkml+xml");
    let ct = part_text(&path, "[Content_Types].xml");
    assert!(ct.contains("application/inkml+xml"), "{ct}");
}

#[test]
fn header_insert() {
    let src = create_docx(
        "hf-src",
        json!({
            "paragraphs": [{ "text": "body" }],
            "headers": [{ "type": "default", "paragraphs": [{ "text": "hdr" }] }]
        }),
    );
    let tree0 = inspect_tree(&src, json!({ "headerFooter": true, "inkCanvases": true }));
    assert_eq!(tree0["headers"][0]["inkCanvasCount"], 0);
    let para = tree0["headers"][0]["paragraphs"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let ink = write_ink("hf", INKML);
    let out = seq_path("hf", "docx");
    apply(
        &src,
        &out,
        json!([{
            "op": "insertInkCanvas",
            "id": para,
            "file": ink.to_string_lossy()
        }]),
    );
    let tree = inspect_tree(&out, json!({ "headerFooter": true, "inkCanvases": true }));
    assert_eq!(tree["inkCanvasCount"], 0);
    assert_eq!(tree["headers"][0]["inkCanvasCount"], 1);
    let listed = tree["headers"][0]["inkCanvases"].as_array().unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0]["contentType"], "application/inkml+xml");
}

#[test]
fn reject_bad_inkml_and_foreign_ops() {
    let src = create_docx("rej", json!({ "paragraphs": [{ "text": "hello" }] }));
    let tree = inspect_tree(&src, json!({ "from": 1, "maxParas": 8 }));
    let para = tree["paragraphs"][0]["id"].as_str().unwrap();
    let junk = seq_path("not-ink", "xml");
    std::fs::write(&junk, b"<root/>").unwrap();
    let err = apply_err(
        &src,
        json!([{
            "op": "insertInkCanvas",
            "id": para,
            "file": junk.to_string_lossy()
        }]),
    );
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let xlsx = seq_path("xlsx", "xlsx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": xlsx.to_string_lossy(),
            "format": "xlsx",
            "workbook": { "sheets": [{ "name": "Sheet1" }] }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let ink = write_ink("xlsx", INKML);
    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": xlsx.to_string_lossy(),
            "output": seq_path("xlsx-out", "xlsx").to_string_lossy(),
            "ops": [{
                "op": "insertInkCanvas",
                "id": "sheet:Sheet1",
                "file": ink.to_string_lossy()
            }]
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": xlsx.to_string_lossy(),
            "output": seq_path("xlsx-size", "xlsx").to_string_lossy(),
            "ops": [{
                "op": "setInkCanvasSize",
                "id": "sheet:Sheet1/ink:1",
                "cx": 1000
            }]
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": xlsx.to_string_lossy(),
            "output": seq_path("xlsx-wrap", "xlsx").to_string_lossy(),
            "ops": [{
                "op": "setInkCanvasWrap",
                "id": "sheet:Sheet1/ink:1",
                "kind": "inline"
            }]
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": xlsx.to_string_lossy(),
            "scope": { "inkCanvases": true }
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let pptx = seq_path("pptx", "pptx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": pptx.to_string_lossy(),
            "format": "pptx",
            "presentation": { "slides": [{ "shapes": [{ "text": "hi" }] }] }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": pptx.to_string_lossy(),
            "output": seq_path("pptx-size", "pptx").to_string_lossy(),
            "ops": [{
                "op": "setInkCanvasSize",
                "id": "slide:1/ink:1",
                "cx": 1000
            }]
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

#[test]
fn insert_on_footnote_is_restricted() {
    let src = create_docx(
        "note",
        json!({ "paragraphs": [{ "text": "body", "footnote": "注文" }] }),
    );
    let tree = inspect_tree(&src, json!({ "notes": true }));
    let para = tree["footnotes"][0]["paragraphs"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let ink = write_ink("note", INKML);
    let err = apply_err(
        &src,
        json!([{
            "op": "insertInkCanvas",
            "id": para,
            "file": ink.to_string_lossy()
        }]),
    );
    assert_eq!(err.code(), ErrorCode::InvalidField);
}
