//! docx 图与文本框 DrawingML 链。

use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::ZipArchive;

const PNG_1X1: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
    0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08, 0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00,
    0x00, 0x03, 0x01, 0x01, 0x00, 0x18, 0xDD, 0x8D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E,
    0x44, 0xAE, 0x42, 0x60, 0x82,
];

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-docx-drawing-hlink");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn tmp(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}.docx"))
}

fn write_png(tag: &str) -> PathBuf {
    let path = tmp_dir().join(format!("{tag}-{}.png", SEQ.fetch_add(1, Ordering::Relaxed)));
    std::fs::write(&path, PNG_1X1).unwrap();
    path
}

fn create_doc(tag: &str, document: Value) -> PathBuf {
    let path = tmp(tag);
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

fn apply(src: &Path, out: &Path, ops: Value) {
    run(&json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": out.to_string_lossy(),
        "ops": ops
    })
    .to_string())
    .unwrap();
}

fn apply_err(src: &Path, ops: Value) -> ooxp::Error {
    let out = tmp("err");
    run(&json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": out.to_string_lossy(),
        "ops": ops
    })
    .to_string())
    .unwrap_err()
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

fn object_keys(v: &Value) -> Vec<&str> {
    v.as_object().unwrap().keys().map(String::as_str).collect()
}

fn part_text(path: &Path, name: &str) -> String {
    let f = std::fs::File::open(path).unwrap();
    let mut zip = ZipArchive::new(f).unwrap();
    let mut zf = zip.by_name(name).unwrap();
    let mut out = String::new();
    zf.read_to_string(&mut out).unwrap();
    out
}

fn listed(path: &Path) -> Value {
    inspect(
        path,
        json!({
            "from": 1,
            "maxParas": 20,
            "pictures": true,
            "textBoxes": true,
            "hyperlinks": true
        }),
    )
}

#[test]
fn picture_drawing_hyperlink_insert_set_delete() {
    let png = write_png("a");
    let src = create_doc(
        "pic",
        json!({
            "paragraphs": [{
                "text": "图",
                "pictures": [{ "file": png.to_string_lossy() }]
            }]
        }),
    );
    let tree0 = listed(&src);
    assert_eq!(tree0["hyperlinkCount"], 0);
    assert_eq!(tree0["hyperlinks"], json!([]));
    let pic = &tree0["paragraphs"][0]["pictures"][0];
    assert!(pic["hyperlink"].is_null());
    assert_eq!(
        object_keys(pic),
        [
            "id",
            "docPrId",
            "name",
            "kind",
            "cx",
            "cy",
            "wrap",
            "stable",
            "hyperlink"
        ]
    );
    let pic_id = pic["id"].as_str().unwrap().to_string();

    let with_link = tmp("link");
    apply(
        &src,
        &with_link,
        json!([{
            "op": "insertHyperlink",
            "id": pic_id,
            "url": "https://example.com/pic"
        }]),
    );
    let tree = listed(&with_link);
    assert_eq!(tree["hyperlinkCount"], 0);
    assert_eq!(tree["hyperlinks"], json!([]));
    let h = &tree["paragraphs"][0]["pictures"][0]["hyperlink"];
    assert_eq!(h["kind"], "url");
    assert_eq!(h["url"], "https://example.com/pic");
    assert!(h["anchor"].is_null());
    assert!(!h["rId"].as_str().unwrap_or("").is_empty());
    let doc = part_text(&with_link, "word/document.xml");
    assert!(doc.contains("hlinkClick"), "{doc}");
    let rels = part_text(&with_link, "word/_rels/document.xml.rels");
    assert!(rels.contains("/hyperlink"), "{rels}");
    assert!(rels.contains("TargetMode=\"External\""), "{rels}");

    let set_out = tmp("set");
    apply(
        &with_link,
        &set_out,
        json!([{
            "op": "setHyperlink",
            "id": pic_id,
            "url": "https://example.org/pic2"
        }]),
    );
    let tree_set = listed(&set_out);
    assert_eq!(
        tree_set["paragraphs"][0]["pictures"][0]["hyperlink"]["url"],
        "https://example.org/pic2"
    );

    let del = tmp("del");
    apply(
        &set_out,
        &del,
        json!([{ "op": "deleteHyperlink", "id": pic_id }]),
    );
    let tree_del = listed(&del);
    assert_eq!(tree_del["pictureCount"], 1);
    assert!(tree_del["paragraphs"][0]["pictures"][0]["hyperlink"].is_null());
    let doc_del = part_text(&del, "word/document.xml");
    assert!(!doc_del.contains("hlinkClick"), "{doc_del}");
}

#[test]
fn picture_drawing_hyperlink_anchor() {
    let png = write_png("bm");
    let src = create_doc(
        "anchor",
        json!({
            "paragraphs": [
                {
                    "text": "图",
                    "pictures": [{ "file": png.to_string_lossy() }],
                    "bookmarks": [{ "name": "SrcBm" }]
                }
            ]
        }),
    );
    let tree0 = listed(&src);
    let pic_id = tree0["paragraphs"][0]["pictures"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let out = tmp("anchor-out");
    apply(
        &src,
        &out,
        json!([{
            "op": "insertHyperlink",
            "id": pic_id,
            "anchor": "SrcBm"
        }]),
    );
    let tree = listed(&out);
    let h = &tree["paragraphs"][0]["pictures"][0]["hyperlink"];
    assert_eq!(h["kind"], "anchor");
    assert_eq!(h["anchor"], "SrcBm");
    assert!(h["url"].is_null());
    let rels = part_text(&out, "word/_rels/document.xml.rels");
    assert!(rels.contains("Target=\"#SrcBm\""), "{rels}");
    assert!(!rels.contains("TargetMode"), "{rels}");
}

#[test]
fn textbox_drawing_hyperlink() {
    let src = create_doc(
        "txbx",
        json!({
            "paragraphs": [{
                "text": "外",
                "textBoxes": [{ "text": "框" }]
            }]
        }),
    );
    let tree0 = listed(&src);
    let box_id = tree0["paragraphs"][0]["textBoxes"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(tree0["paragraphs"][0]["textBoxes"][0]["hyperlink"].is_null());
    let keys = object_keys(&tree0["paragraphs"][0]["textBoxes"][0]);
    assert!(keys.contains(&"hyperlink"), "{keys:?}");
    let out = tmp("txbx-link");
    apply(
        &src,
        &out,
        json!([{
            "op": "insertHyperlink",
            "id": box_id,
            "url": "https://example.net/box",
            "tooltip": "框链"
        }]),
    );
    let tree = listed(&out);
    assert_eq!(tree["hyperlinkCount"], 0);
    let h = &tree["paragraphs"][0]["textBoxes"][0]["hyperlink"];
    assert_eq!(h["kind"], "url");
    assert_eq!(h["url"], "https://example.net/box");
    assert_eq!(h["tooltip"], "框链");
}

#[test]
fn drawing_hyperlink_rejects() {
    let png = write_png("rej");
    let src = create_doc(
        "rej",
        json!({
            "paragraphs": [{
                "text": "外",
                "pictures": [{ "file": png.to_string_lossy() }],
                "textBoxes": [{ "text": "框" }]
            }]
        }),
    );
    let tree = listed(&src);
    let pic_id = tree["paragraphs"][0]["pictures"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let err_text = apply_err(
        &src,
        json!([{
            "op": "insertHyperlink",
            "id": pic_id,
            "text": "不要",
            "url": "https://example.com"
        }]),
    );
    assert_eq!(err_text.code(), ErrorCode::InvalidField);

    let inner_para = inspect(
        &src,
        json!({
            "from": 1,
            "maxParas": 20,
            "textBoxes": true,
            "textBoxContent": true
        }),
    )["paragraphs"][0]["textBoxes"][0]["paragraphs"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let with_inner = tmp("inner-pic");
    apply(
        &src,
        &with_inner,
        json!([{
            "op": "insertPicture",
            "id": inner_para,
            "file": png.to_string_lossy()
        }]),
    );
    let listed_inner = inspect(
        &with_inner,
        json!({
            "from": 1,
            "maxParas": 20,
            "pictures": true,
            "textBoxes": true,
            "textBoxContent": true
        }),
    );
    let inner_pic = listed_inner["paragraphs"][0]["textBoxes"][0]["paragraphs"][0]["pictures"][0]
        ["id"]
        .as_str()
        .unwrap()
        .to_string();
    let linked_inner = tmp("inner-link");
    apply(
        &with_inner,
        &linked_inner,
        json!([{
            "op": "insertHyperlink",
            "id": inner_pic,
            "url": "https://example.com/inner"
        }]),
    );
    let tree_inner = inspect(
        &linked_inner,
        json!({
            "from": 1,
            "maxParas": 20,
            "pictures": true,
            "textBoxes": true,
            "textBoxContent": true,
            "hyperlinks": true
        }),
    );
    assert_eq!(tree_inner["hyperlinkCount"], 0);
    assert_eq!(
        tree_inner["paragraphs"][0]["textBoxes"][0]["paragraphs"][0]["pictures"][0]["hyperlink"]
            ["url"],
        "https://example.com/inner"
    );

    let with_link = tmp("dup");
    apply(
        &src,
        &with_link,
        json!([{
            "op": "insertHyperlink",
            "id": pic_id,
            "url": "https://example.com"
        }]),
    );
    let err_dup = apply_err(
        &with_link,
        json!([{
            "op": "insertHyperlink",
            "id": pic_id,
            "url": "https://example.org"
        }]),
    );
    assert_eq!(err_dup.code(), ErrorCode::InvalidField);
}
