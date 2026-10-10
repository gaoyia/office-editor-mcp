//! docx 无框形状：inspect 计数、插删改尺寸/填充/线、组合、页眉、拒连接线几何。

use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::ZipArchive;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-docx-shapes");
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

fn body_para(tree: &Value, index: usize) -> &str {
    tree["paragraphs"][index]["id"].as_str().unwrap()
}

fn object_keys(v: &Value) -> Vec<&str> {
    v.as_object().unwrap().keys().map(|k| k.as_str()).collect()
}

#[test]
fn inspect_has_shape_count_without_list() {
    let src = create_docx("bare");
    let tree = inspect(&src, json!({}));
    assert_eq!(tree["shapeCount"], 0);
    assert!(tree.get("shapes").is_none());
    assert!(tree.get("paragraphs").is_none());
    let keys = object_keys(&tree);
    let cxn = keys.iter().position(|k| *k == "connectorCount").unwrap();
    assert_eq!(keys[cxn + 1], "shapeCount");
    assert_eq!(keys[cxn + 2], "inkCanvasCount");
}

#[test]
fn insert_rect_and_ellipse_then_delete() {
    let src = create_docx("ins");
    let sliced = inspect(&src, json!({ "from": 1, "maxParas": 10 }));
    let para = body_para(&sliced, 1).to_string();
    let inserted = seq_path("ins-rect", "docx");
    apply(
        &src,
        &inserted,
        json!([{ "op": "insertShape", "id": para }]),
    );
    let tree = inspect(
        &inserted,
        json!({ "from": 1, "maxParas": 10, "shapes": true }),
    );
    assert_eq!(tree["shapeCount"], 1);
    assert_eq!(tree["connectorCount"], 0);
    assert_eq!(tree["paragraphs"][1]["shapeCount"], 1);
    let pkeys = object_keys(&tree["paragraphs"][1]);
    let cxn = pkeys.iter().position(|k| *k == "connectorCount").unwrap();
    assert_eq!(pkeys[cxn + 1], "shapeCount");
    assert_eq!(pkeys[cxn + 2], "inkCanvasCount");
    let sh = &tree["paragraphs"][1]["shapes"][0];
    assert_eq!(sh["prst"], "rect");
    assert_eq!(sh["kind"], "anchor");
    assert_eq!(sh["wrap"], "square");
    assert_eq!(sh["cx"], 1_828_800);
    assert_eq!(sh["cy"], 914_400);
    assert_eq!(sh["stable"], true);
    assert!(sh.get("text").is_none());
    let id = sh["id"].as_str().unwrap().to_string();
    assert!(id.contains("/shape:"));
    assert_eq!(
        object_keys(sh),
        ["id", "docPrId", "name", "kind", "cx", "cy", "wrap", "prst", "stable"]
    );
    let xml = part_text(&inserted, "word/document.xml");
    assert!(xml.contains("Requires=\"wps\""));
    assert!(xml.contains("prst=\"rect\""));
    assert!(xml.contains("cNvSpPr"));
    assert!(!xml.contains("txBox"));
    assert!(!xml.contains("cNvCnPr"));
    assert!(!xml.contains("txbxContent"));
    assert!(!xml.contains("wps:style"));
    assert!(xml.contains("形状 "));

    let oval = seq_path("ins-ellipse", "docx");
    apply(
        &src,
        &oval,
        json!([{ "op": "insertShape", "id": para, "prst": "ellipse" }]),
    );
    let tree_e = inspect(&oval, json!({ "from": 1, "maxParas": 10, "shapes": true }));
    assert_eq!(tree_e["paragraphs"][1]["shapes"][0]["prst"], "ellipse");
    let xml_e = part_text(&oval, "word/document.xml");
    assert!(xml_e.contains("prst=\"ellipse\""));

    let deleted = seq_path("ins-del", "docx");
    apply(
        &inserted,
        &deleted,
        json!([{ "op": "deleteShape", "id": id }]),
    );
    let tree2 = inspect(
        &deleted,
        json!({ "from": 1, "maxParas": 10, "shapes": true }),
    );
    assert_eq!(tree2["shapeCount"], 0);
    assert_eq!(tree2["paragraphs"][1]["shapeCount"], 0);
}

#[test]
fn set_size_pos_fill_and_line() {
    let src = create_docx("style");
    let sliced = inspect(&src, json!({ "from": 1, "maxParas": 10 }));
    let para = body_para(&sliced, 1).to_string();
    let inserted = seq_path("style-ins", "docx");
    apply(
        &src,
        &inserted,
        json!([{ "op": "insertShape", "id": para }]),
    );
    let listed = inspect(
        &inserted,
        json!({ "from": 1, "maxParas": 10, "shapes": true }),
    );
    let id = listed["paragraphs"][1]["shapes"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let styled = seq_path("style-out", "docx");
    apply(
        &inserted,
        &styled,
        json!([
            { "op": "setShapeSize", "id": id, "cx": 914400, "cy": 914400 },
            { "op": "setShapePos", "id": id, "x": 0, "y": 12700 },
            { "op": "setShapeFill", "id": id, "color": "FF0000" },
            { "op": "setShapeLine", "id": id, "color": "0000FF", "width": 12700 }
        ]),
    );
    let tree = inspect(
        &styled,
        json!({ "from": 1, "maxParas": 10, "shapes": true }),
    );
    let sh = &tree["paragraphs"][1]["shapes"][0];
    assert_eq!(sh["cx"], 914_400);
    assert_eq!(sh["cy"], 914_400);
    let xml = part_text(&styled, "word/document.xml");
    assert!(xml.contains("val=\"FF0000\""));
    assert!(xml.contains("val=\"0000FF\""));
    assert!(xml.contains("w=\"12700\""));
    assert!(xml.contains("posOffset"));
    assert!(xml.contains(">0<") || xml.contains(">12700<"));
}

#[test]
fn header_insert_shape_does_not_count_on_body() {
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
    let hkeys = object_keys(&tree_hf["headers"][0]);
    let cxn = hkeys.iter().position(|k| *k == "connectorCount").unwrap();
    assert_eq!(hkeys[cxn + 1], "shapeCount");
    assert_eq!(hkeys[cxn + 2], "inkCanvasCount");
    let out = seq_path("hf-sh", "docx");
    apply(&hf, &out, json!([{ "op": "insertShape", "id": para }]));
    let listed = inspect(&out, json!({ "shapes": true }));
    assert_eq!(listed["shapeCount"], 0);
    assert_eq!(listed["headers"][0]["shapeCount"], 1);
    let shapes = listed["headers"][0]["shapes"].as_array().unwrap();
    assert_eq!(shapes.len(), 1);
    assert!(shapes[0]["id"].as_str().unwrap().contains("/shape:"));
}

#[test]
fn group_and_ungroup_shapes() {
    let src = create_docx("grp");
    let sliced = inspect(&src, json!({ "from": 1, "maxParas": 10 }));
    let p0 = body_para(&sliced, 0).to_string();
    let p1 = body_para(&sliced, 1).to_string();
    let two = seq_path("grp-two", "docx");
    apply(
        &src,
        &two,
        json!([
            { "op": "insertShape", "id": p0, "prst": "rect" },
            { "op": "insertShape", "id": p1, "prst": "ellipse" }
        ]),
    );
    let listed = inspect(&two, json!({ "from": 1, "maxParas": 10, "shapes": true }));
    assert_eq!(listed["shapeCount"], 2);
    let a = listed["paragraphs"][0]["shapes"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let b = listed["paragraphs"][1]["shapes"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let grouped = seq_path("grp-on", "docx");
    apply(
        &two,
        &grouped,
        json!([{
            "op": "groupShapes",
            "id": "docx:document",
            "ids": [a, b]
        }]),
    );
    let after = inspect(
        &grouped,
        json!({ "from": 1, "maxParas": 10, "shapes": true }),
    );
    assert_eq!(after["shapeCount"], 2);
    assert_eq!(after["paragraphs"][0]["shapeCount"], 2);
    assert_eq!(after["paragraphs"][1]["shapeCount"], 0);
    let xml = part_text(&grouped, "word/document.xml");
    assert!(xml.contains("wpg:wgp"));
    let ids: Vec<String> = after["paragraphs"][0]["shapes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["id"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(ids.len(), 2);
    let filled = seq_path("grp-fill", "docx");
    apply(
        &grouped,
        &filled,
        json!([{ "op": "setShapeFill", "id": ids[1], "color": "00FF00" }]),
    );
    let xml_f = part_text(&filled, "word/document.xml");
    assert!(xml_f.contains("val=\"00FF00\""));
    assert!(xml_f.contains("val=\"FFFFFF\""));

    let ungrouped = seq_path("grp-ung", "docx");
    apply(
        &filled,
        &ungrouped,
        json!([{ "op": "ungroup", "id": ids[0] }]),
    );
    let done = inspect(
        &ungrouped,
        json!({ "from": 1, "maxParas": 10, "shapes": true }),
    );
    assert_eq!(done["shapeCount"], 2);
    let xml2 = part_text(&ungrouped, "word/document.xml");
    assert!(!xml2.contains("wpg:wgp"));
    assert!(xml2.contains("Requires=\"wps\""));
}

#[test]
fn reject_connector_prst_text_geom_and_restricted_hosts() {
    let src = create_docx("bad");
    let sliced = inspect(&src, json!({ "from": 1, "maxParas": 10 }));
    let para = body_para(&sliced, 1).to_string();
    let err_prst = apply_err(
        &src,
        json!([{ "op": "insertShape", "id": para, "prst": "straightConnector1" }]),
    );
    assert_eq!(err_prst.code(), ErrorCode::InvalidField);

    let err_text = apply_err(
        &src,
        json!([{ "op": "insertShape", "id": para, "text": "框" }]),
    );
    assert_eq!(err_text.code(), ErrorCode::InvalidField);

    let err_geom = apply_err(
        &src,
        json!([{ "op": "setShapeGeom", "id": format!("{para}/shape:1") }]),
    );
    assert_eq!(err_geom.code(), ErrorCode::InvalidField);

    let err_box = apply_err(
        &src,
        json!([{ "op": "insertShape", "id": format!("{para}/txbx:1") }]),
    );
    assert_eq!(err_box.code(), ErrorCode::InvalidId);

    let err_note = apply_err(
        &src,
        json!([{ "op": "insertShape", "id": "footnote:2/para:1" }]),
    );
    assert_eq!(err_note.code(), ErrorCode::InvalidField);

    let err_comment = apply_err(
        &src,
        json!([{ "op": "insertShape", "id": "comment:1/para:1" }]),
    );
    assert_eq!(err_comment.code(), ErrorCode::InvalidField);

    let inserted = seq_path("bad-ins", "docx");
    apply(
        &src,
        &inserted,
        json!([{ "op": "insertShape", "id": para }]),
    );
    let listed = inspect(
        &inserted,
        json!({ "from": 1, "maxParas": 10, "shapes": true }),
    );
    let sid = listed["paragraphs"][1]["shapes"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let err_hlink = apply_err(
        &inserted,
        json!([{ "op": "insertHyperlink", "id": sid, "url": "https://example.com" }]),
    );
    assert_eq!(err_hlink.code(), ErrorCode::InvalidId);
}

#[test]
fn shape_ops_not_for_xlsx() {
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
            "ops": [{ "op": "insertShape", "id": "para:1" }]
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert!(!xout.exists());
    assert_eq!(err.code(), ErrorCode::InvalidId);
}
