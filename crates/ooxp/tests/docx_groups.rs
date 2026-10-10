//! docx 组合与连接线：inspect 计数、插删连接线、组合浮动框/图、xlsx / setShapeGeom 拒绝。

use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::ZipArchive;

static SEQ: AtomicU32 = AtomicU32::new(0);

const PNG_1X1: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
    0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08, 0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00,
    0x00, 0x03, 0x01, 0x01, 0x00, 0x18, 0xDD, 0x8D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E,
    0x44, 0xAE, 0x42, 0x60, 0x82,
];

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-docx-groups");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn seq_path(tag: &str, ext: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}.{ext}"))
}

fn create_docx(tag: &str) -> PathBuf {
    let path = seq_path(tag, "docx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "format": "docx",
            "document": {
                "paragraphs": [
                    { "style": "Heading1", "text": "第一章" },
                    { "text": "hello" }
                ]
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

fn apply(src: &Path, out: &Path, ops: Value) {
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
    .unwrap();
}

fn apply_err(src: &Path, ops: Value) -> ooxp::Error {
    let out = seq_path("should-not-write", "docx");
    let _ = std::fs::remove_file(&out);
    let err = run(
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
    .unwrap_err();
    assert!(!out.exists(), "失败的 apply 不应写出文件");
    err
}

fn part_text(path: &Path, name: &str) -> String {
    let bytes = std::fs::read(path).unwrap();
    let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut file = zip.by_name(name.trim_start_matches('/')).unwrap();
    let mut s = String::new();
    file.read_to_string(&mut s).unwrap();
    s
}

fn write_png() -> PathBuf {
    let path = seq_path("pic", "png");
    std::fs::write(&path, PNG_1X1).unwrap();
    path
}

fn body_para(tree: &Value, index: usize) -> &str {
    tree["paragraphs"][index]["id"].as_str().unwrap()
}

fn wrap_anchor(id: &str) -> Value {
    json!({
        "op": "setPictureWrap",
        "id": id,
        "kind": "anchor",
        "wrap": "square"
    })
}

#[test]
fn inspect_has_connector_count_without_list() {
    let src = create_docx("bare");
    let tree = inspect(&src, json!({}));
    assert_eq!(tree["connectorCount"], 0);
    assert!(tree.get("connectors").is_none());
    assert!(tree.get("paragraphs").is_none());
}

#[test]
fn insert_and_delete_connector() {
    let src = create_docx("cxn");
    let sliced = inspect(&src, json!({ "from": 1, "maxParas": 10 }));
    let para = body_para(&sliced, 1).to_string();
    let inserted = seq_path("cxn-ins", "docx");
    apply(
        &src,
        &inserted,
        json!([{ "op": "insertConnector", "id": para }]),
    );
    let tree = inspect(
        &inserted,
        json!({ "from": 1, "maxParas": 10, "connectors": true }),
    );
    assert_eq!(tree["connectorCount"], 1);
    assert_eq!(tree["paragraphs"][1]["connectorCount"], 1);
    let cxn = &tree["paragraphs"][1]["connectors"][0];
    assert_eq!(cxn["prst"], "straightConnector1");
    assert_eq!(cxn["kind"], "anchor");
    assert_eq!(cxn["stable"], true);
    let id = cxn["id"].as_str().unwrap();
    assert!(id.contains("/cxn:"));
    let xml = part_text(&inserted, "word/document.xml");
    assert!(xml.contains("straightConnector1"));
    assert!(xml.contains("cNvCnPr"));
    assert!(xml.contains("Requires=\"wps\""));
    assert!(!xml.contains("txbxContent"));

    let deleted = seq_path("cxn-del", "docx");
    apply(
        &inserted,
        &deleted,
        json!([{ "op": "deleteConnector", "id": id }]),
    );
    let tree2 = inspect(
        &deleted,
        json!({ "from": 1, "maxParas": 10, "connectors": true }),
    );
    assert_eq!(tree2["connectorCount"], 0);
    assert_eq!(tree2["paragraphs"][1]["connectorCount"], 0);
}

#[test]
fn group_and_ungroup_textboxes() {
    let src = create_docx("txbx");
    let sliced = inspect(&src, json!({ "from": 1, "maxParas": 10 }));
    let p0 = body_para(&sliced, 0).to_string();
    let p1 = body_para(&sliced, 1).to_string();
    let boxed = seq_path("txbx-two", "docx");
    apply(
        &src,
        &boxed,
        json!([
            { "op": "insertTextBox", "id": p0, "text": "框甲" },
            { "op": "insertTextBox", "id": p1, "text": "框乙" }
        ]),
    );
    let listed = inspect(
        &boxed,
        json!({ "from": 1, "maxParas": 10, "textBoxes": true }),
    );
    assert_eq!(listed["textBoxCount"], 2);
    let a = listed["paragraphs"][0]["textBoxes"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let b = listed["paragraphs"][1]["textBoxes"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let grouped = seq_path("txbx-grp", "docx");
    apply(
        &boxed,
        &grouped,
        json!([{
            "op": "groupShapes",
            "id": "docx:document",
            "ids": [a, b]
        }]),
    );
    let after = inspect(
        &grouped,
        json!({ "from": 1, "maxParas": 10, "textBoxes": true }),
    );
    assert_eq!(after["textBoxCount"], 2);
    assert_eq!(after["paragraphs"][0]["textBoxCount"], 2);
    assert_eq!(after["paragraphs"][1]["textBoxCount"], 0);
    let xml = part_text(&grouped, "word/document.xml");
    assert!(xml.contains("wpg:wgp"));
    assert!(xml.contains("Requires=\"wpg\""));
    let ma = after["paragraphs"][0]["textBoxes"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();

    let ungrouped = seq_path("txbx-ung", "docx");
    apply(&grouped, &ungrouped, json!([{ "op": "ungroup", "id": ma }]));
    let done = inspect(
        &ungrouped,
        json!({ "from": 1, "maxParas": 10, "textBoxes": true }),
    );
    assert_eq!(done["textBoxCount"], 2);
    let xml2 = part_text(&ungrouped, "word/document.xml");
    assert!(!xml2.contains("wpg:wgp"));
}

#[test]
fn group_floating_pictures_keeps_picture_count() {
    let src = create_docx("pic");
    let png = write_png();
    let sliced = inspect(&src, json!({ "from": 1, "maxParas": 10 }));
    let para = body_para(&sliced, 1).to_string();
    let with_pic = seq_path("pic-two", "docx");
    apply(
        &src,
        &with_pic,
        json!([
            { "op": "insertPicture", "id": para, "file": png.to_string_lossy() },
            { "op": "insertPicture", "id": para, "file": png.to_string_lossy() }
        ]),
    );
    let listed = inspect(
        &with_pic,
        json!({ "from": 1, "maxParas": 10, "pictures": true }),
    );
    assert_eq!(listed["pictureCount"], 2);
    let a = listed["paragraphs"][1]["pictures"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let b = listed["paragraphs"][1]["pictures"][1]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let floated = seq_path("pic-float", "docx");
    apply(
        &with_pic,
        &floated,
        json!([wrap_anchor(&a), wrap_anchor(&b)]),
    );
    let grouped = seq_path("pic-grp", "docx");
    apply(
        &floated,
        &grouped,
        json!([{
            "op": "groupShapes",
            "id": "docx:document",
            "ids": [a, b]
        }]),
    );
    let after = inspect(
        &grouped,
        json!({ "from": 1, "maxParas": 10, "pictures": true }),
    );
    assert_eq!(after["pictureCount"], 2);
    assert_eq!(after["paragraphs"][1]["pictureCount"], 2);
    let xml = part_text(&grouped, "word/document.xml");
    assert!(xml.contains("wpg:wgp"));
    assert!(xml.contains("pic:pic"));
}

#[test]
fn header_insert_connector() {
    let src = create_docx("hf");
    let hf = seq_path("hf", "docx");
    apply(
        &src,
        &hf,
        json!([{
            "op": "insertHeaderFooter",
            "id": "section:1",
            "kind": "header",
            "type": "default"
        }]),
    );
    let tree_hf = inspect(&hf, json!({ "headerFooter": true }));
    let para = tree_hf["headers"][0]["paragraphs"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let out = seq_path("hf-cxn", "docx");
    apply(&hf, &out, json!([{ "op": "insertConnector", "id": para }]));
    let listed = inspect(&out, json!({ "connectors": true }));
    assert_eq!(listed["connectorCount"], 0);
    assert_eq!(listed["headers"][0]["connectorCount"], 1);
    let cxns = listed["headers"][0]["connectors"].as_array().unwrap();
    assert_eq!(cxns.len(), 1);
    assert!(cxns[0]["id"].as_str().unwrap().contains("/cxn:"));
}

#[test]
fn reject_inline_group_member_and_bad_ids() {
    let src = create_docx("bad");
    let png = write_png();
    let sliced = inspect(&src, json!({ "from": 1, "maxParas": 10 }));
    let para = body_para(&sliced, 1).to_string();
    let with_pic = seq_path("inline", "docx");
    apply(
        &src,
        &with_pic,
        json!([
            { "op": "insertPicture", "id": para, "file": png.to_string_lossy() },
            { "op": "insertPicture", "id": para, "file": png.to_string_lossy() }
        ]),
    );
    let listed = inspect(
        &with_pic,
        json!({ "from": 1, "maxParas": 10, "pictures": true }),
    );
    let a = listed["paragraphs"][1]["pictures"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let b = listed["paragraphs"][1]["pictures"][1]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let err = apply_err(
        &with_pic,
        json!([{
            "op": "groupShapes",
            "id": "docx:document",
            "ids": [a, b]
        }]),
    );
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let err_host = apply_err(
        &src,
        json!([{
            "op": "groupShapes",
            "id": para,
            "ids": ["x", "y"]
        }]),
    );
    assert_eq!(err_host.code(), ErrorCode::InvalidId);

    let err_cxn = apply_err(
        &src,
        json!([{ "op": "deleteConnector", "id": format!("{para}/cxn:9") }]),
    );
    assert_eq!(err_cxn.code(), ErrorCode::IdNotFound);

    let err_shape = apply_err(&src, json!([{ "op": "setShapeGeom", "id": para }]));
    assert_eq!(err_shape.code(), ErrorCode::InvalidField);
}

#[test]
fn connector_and_shape_ops_not_for_xlsx() {
    let xlsx = seq_path("x", "xlsx");
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
    let xout = seq_path("x-out", "xlsx");
    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": xlsx.to_string_lossy(),
            "output": xout.to_string_lossy(),
            "ops": [{ "op": "insertConnector", "id": "para:1" }]
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert!(!xout.exists());
    assert_eq!(err.code(), ErrorCode::InvalidId);
}
