//! 幻灯片 Zoom：计数、名单、插删；预览图不进 pictures。

use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::ZipArchive;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-pptx-zoom");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn tmp(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}"))
}

fn create_deck(tag: &str, n: usize) -> PathBuf {
    let path = tmp(tag).with_extension("pptx");
    let slides: Vec<Value> = (1..=n)
        .map(|i| json!({ "shapes": [{ "text": format!("页{i}") }] }))
        .collect();
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "presentation": { "slides": slides }
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

fn inspect_err(path: &Path, scope: Value) -> ooxp::Error {
    run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": path.to_string_lossy(),
            "scope": scope
        })
        .to_string(),
        None,
    )
    .unwrap_err()
}

fn apply(src: &Path, out: &Path, ops: Value) -> Result<RunOutput, ooxp::Error> {
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
}

fn object_keys(v: &Value) -> Vec<&str> {
    v.as_object().unwrap().keys().map(String::as_str).collect()
}

fn zip_entry(path: &Path, name: &str) -> Option<Vec<u8>> {
    let bytes = std::fs::read(path).unwrap();
    let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let want = name.replace('\\', "/");
    for i in 0..zip.len() {
        let mut f = zip.by_index(i).unwrap();
        if f.name().replace('\\', "/") == want {
            let mut buf = Vec::new();
            f.read_to_end(&mut buf).unwrap();
            return Some(buf);
        }
    }
    None
}

fn zip_has_prefix(path: &Path, prefix: &str) -> bool {
    let bytes = std::fs::read(path).unwrap();
    let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    for i in 0..zip.len() {
        let name = zip.by_index(i).unwrap().name().replace('\\', "/");
        if name.starts_with(prefix) {
            return true;
        }
    }
    false
}

fn insert_slide_zoom(src: &Path) -> PathBuf {
    let out = tmp("ins").with_extension("pptx");
    apply(
        src,
        &out,
        json!([{
            "op": "insertZoom",
            "id": "slide:1",
            "type": "slide",
            "target": "slide:2"
        }]),
    )
    .unwrap();
    out
}

#[test]
fn empty_create_has_count_without_list() {
    let path = create_deck("empty", 1);
    let tree = inspect(&path, json!({}));
    assert_eq!(tree["zoomCount"], 0);
    assert_eq!(tree["slides"][0]["zoomCount"], 0);
    assert!(tree["slides"][0].get("zooms").is_none());
    assert_eq!(
        object_keys(&tree),
        [
            "schemaVersion",
            "format",
            "id",
            "slideCount",
            "pictureCount",
            "mathCount",
            "videoCount",
            "audioCount",
            "chartCount",
            "smartArtCount",
            "commentCount",
            "hyperlinkCount",
            "fieldCount",
            "tableCount",
            "oleObjectCount",
            "model3dCount",
            "zoomCount",
            "connectorCount",
            "animCount",
            "masterCount",
            "hasHandout",
            "sectionCount",
            "slideSize",
            "customShowCount",
            "showPr",
            "modifyProtection",
            "props",
            "slides"
        ]
    );
}

#[test]
fn insert_slide_zoom_lists_and_skips_preview() {
    let src = create_deck("ins-host", 3);
    let out = insert_slide_zoom(&src);
    let tree = inspect(&out, json!({ "zooms": true, "pictures": true }));
    assert_eq!(tree["zoomCount"], 1);
    assert_eq!(tree["slides"][0]["zoomCount"], 1);
    assert_eq!(tree["slides"][1]["zoomCount"], 0);
    assert_eq!(tree["pictureCount"], 0);
    assert_eq!(tree["slides"][0]["pictures"], json!([]));
    let items = tree["slides"][0]["zooms"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    let z = &items[0];
    let id = z["id"].as_str().unwrap();
    assert!(id.starts_with("slide:1/zoom:"));
    assert!(!id.contains("index:"));
    assert_eq!(z["type"], "slide");
    assert_eq!(z["target"], "slide:2");
    assert_eq!(z["cx"], 3_657_600);
    assert_eq!(z["cy"], 2_743_200);
    assert_eq!(z["stable"], true);
    assert_eq!(
        object_keys(z),
        ["id", "type", "target", "x", "y", "cx", "cy", "stable"]
    );
    assert!(zip_has_prefix(&out, "ppt/media/"));
    let xml =
        String::from_utf8(zip_entry(&out, "ppt/slides/slide1.xml").unwrap_or_default()).unwrap();
    assert!(xml.contains("AlternateContent"));
    assert!(xml.contains("graphicFrame"));
    assert!(xml.contains("sldZm"));
    assert!(xml.contains("sldId=\"257\""));
    assert!(xml.contains("ppaction://hlinksldjump"));
}

#[test]
fn delete_removes_zoom_and_preview() {
    let src = create_deck("del-host", 3);
    let with = insert_slide_zoom(&src);
    let listed = inspect(&with, json!({ "zooms": true }));
    let id = listed["slides"][0]["zooms"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let out = tmp("del").with_extension("pptx");
    apply(&with, &out, json!([{ "op": "deleteZoom", "id": id }])).unwrap();
    let tree = inspect(&out, json!({ "zooms": true }));
    assert_eq!(tree["zoomCount"], 0);
    assert_eq!(tree["slides"][0]["zoomCount"], 0);
    let list = tree["slides"][0]["zooms"].as_array().unwrap();
    assert!(list.is_empty());
    assert!(!zip_has_prefix(&out, "ppt/media/"));
}

#[test]
fn insert_section_zoom_targets_section() {
    let src = tmp("sec-src").with_extension("pptx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": src.to_string_lossy(),
            "presentation": {
                "sections": [
                    { "name": "开场", "start": 1 },
                    { "name": "内容", "start": 2 }
                ],
                "slides": [
                    { "shapes": [{ "text": "甲" }] },
                    { "shapes": [{ "text": "乙" }] }
                ]
            }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let out = tmp("sec").with_extension("pptx");
    apply(
        &src,
        &out,
        json!([{
            "op": "insertZoom",
            "id": "slide:1",
            "type": "section",
            "target": "section:2"
        }]),
    )
    .unwrap();
    let tree = inspect(&out, json!({ "zooms": true }));
    let z = &tree["slides"][0]["zooms"][0];
    assert_eq!(z["type"], "section");
    assert_eq!(z["target"], "section:2");
    let xml =
        String::from_utf8(zip_entry(&out, "ppt/slides/slide1.xml").unwrap_or_default()).unwrap();
    assert!(xml.contains("sectionZm"));
    assert!(xml.contains("Requires=\"psez\""));
}

#[test]
fn wrong_ids_and_foreign_reject() {
    let deck = create_deck("bad-host", 3);
    let out = tmp("bad").with_extension("pptx");
    let err = apply(
        &deck,
        &out,
        json!([{
            "op": "insertZoom",
            "id": "slide:1/zoom:3",
            "type": "slide",
            "target": "slide:2"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    assert!(!out.exists());

    let err = apply(
        &deck,
        &out,
        json!([{ "op": "deleteZoom", "id": "slide:1" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    assert!(!out.exists());

    let err = apply(
        &deck,
        &out,
        json!([{
            "op": "insertZoom",
            "id": "slide:1",
            "type": "summary",
            "target": "slide:2"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());

    let err = apply(
        &deck,
        &out,
        json!([{
            "op": "insertZoom",
            "id": "slide:1",
            "type": "slide",
            "target": "slide:1"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());

    let book = tmp("xlsx-zoom").with_extension("xlsx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": book.to_string_lossy(),
            "workbook": { "sheets": [{ "name": "Sheet1" }] }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let err = apply(
        &book,
        &tmp("xlsx-zoom-out").with_extension("xlsx"),
        json!([{
            "op": "insertZoom",
            "id": "slide:1",
            "type": "slide",
            "target": "slide:2"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let err = inspect_err(&book, json!({ "zooms": true }));
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": tmp("create-zoom").with_extension("pptx").to_string_lossy(),
            "presentation": {
                "slides": [{ "shapes": [{ "text": "a" }], "zooms": [] }]
            }
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let docx = tmp("docx-zoom").with_extension("docx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": docx.to_string_lossy(),
            "document": { "paragraphs": [{ "text": "甲" }] }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let err = apply(
        &docx,
        &tmp("docx-zoom-out").with_extension("docx"),
        json!([{
            "op": "insertZoom",
            "id": "slide:1",
            "type": "slide",
            "target": "slide:2"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}
