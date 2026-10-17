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
            "inkCount",
            "zoomCount",
            "connectorCount",
            "animCount",
            "masterCount",
            "hasHandout",
            "sectionCount",
            "slideSize",
            "notesSize",
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
    let msg = err.to_json(ooxp::Lang::ZhCn)["error"]["message"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(msg.contains("必须带 /zoom"), "{msg}");
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
                "slides": [{ "shapes": [{ "text": "a" }], "zooms": [{}] }]
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

fn create_sectioned_deck(tag: &str) -> PathBuf {
    let path = tmp(tag).with_extension("pptx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "presentation": {
                "sections": [
                    { "name": "开场", "start": 1 },
                    { "name": "内容", "start": 2 }
                ],
                "slides": [
                    { "shapes": [{ "text": "甲" }] },
                    { "shapes": [{ "text": "乙" }] },
                    { "shapes": [{ "text": "丙" }] }
                ]
            }
        })
        .to_string(),
        None,
    )
    .unwrap();
    path
}

const PNG_1X1: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
    0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08, 0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00,
    0x00, 0x03, 0x01, 0x01, 0x00, 0x18, 0xDD, 0x8D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E,
    0x44, 0xAE, 0x42, 0x60, 0x82,
];

fn write_png(tag: &str) -> PathBuf {
    let path = tmp(tag).with_extension("png");
    std::fs::write(&path, PNG_1X1).unwrap();
    path
}

fn zip_media_png(path: &Path) -> Option<Vec<u8>> {
    let bytes = std::fs::read(path).unwrap();
    let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    for i in 0..zip.len() {
        let mut f = zip.by_index(i).unwrap();
        let name = f.name().replace('\\', "/");
        if name.starts_with("ppt/media/") && name.ends_with(".png") {
            let mut buf = Vec::new();
            f.read_to_end(&mut buf).unwrap();
            return Some(buf);
        }
    }
    None
}

#[test]
fn insert_summary_zoom_lists_targets() {
    let src = create_sectioned_deck("sum-src");
    let out = tmp("sum").with_extension("pptx");
    apply(
        &src,
        &out,
        json!([{
            "op": "insertZoom",
            "id": "slide:1",
            "type": "summary",
            "targets": ["section:1", "section:2"]
        }]),
    )
    .unwrap();
    let tree = inspect(&out, json!({ "zooms": true, "pictures": true }));
    let z = &tree["slides"][0]["zooms"][0];
    assert_eq!(z["type"], "summary");
    assert_eq!(z["targets"], json!(["section:1", "section:2"]));
    assert!(z.get("target").is_none());
    assert_eq!(tree["pictureCount"], 0);
    let xml =
        String::from_utf8(zip_entry(&out, "ppt/slides/slide1.xml").unwrap_or_default()).unwrap();
    assert!(xml.contains("summaryZm"));
    assert!(xml.contains("Requires=\"psuz\""));
    assert!(xml.contains("gridLayout"));
}

#[test]
fn set_zoom_changes_slide_target_and_cover() {
    let src = create_deck("set-host", 3);
    let with = insert_slide_zoom(&src);
    let listed = inspect(&with, json!({ "zooms": true }));
    let id = listed["slides"][0]["zooms"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let out = tmp("set-tgt").with_extension("pptx");
    apply(
        &with,
        &out,
        json!([{ "op": "setZoom", "id": id, "target": "slide:3" }]),
    )
    .unwrap();
    let tree = inspect(&out, json!({ "zooms": true }));
    assert_eq!(tree["slides"][0]["zooms"][0]["target"], "slide:3");

    let png = write_png("cover");
    let cover = tmp("set-cover").with_extension("pptx");
    apply(
        &out,
        &cover,
        json!([{
            "op": "setZoom",
            "id": tree["slides"][0]["zooms"][0]["id"],
            "file": png.to_string_lossy()
        }]),
    )
    .unwrap();
    let media = zip_media_png(&cover).unwrap();
    assert_eq!(media, PNG_1X1);
    let xml =
        String::from_utf8(zip_entry(&cover, "ppt/slides/slide1.xml").unwrap_or_default()).unwrap();
    assert!(xml.contains("imageType=\"cover\""));
    let tree2 = inspect(&cover, json!({ "zooms": true, "pictures": true }));
    assert_eq!(tree2["pictureCount"], 0);
}

#[test]
fn set_zoom_rejects_wrong_fields() {
    let src = create_deck("set-bad", 3);
    let with = insert_slide_zoom(&src);
    let id = inspect(&with, json!({ "zooms": true }))["slides"][0]["zooms"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let out = tmp("set-empty").with_extension("pptx");
    let err = apply(&with, &out, json!([{ "op": "setZoom", "id": id }])).unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());

    let id = inspect(&with, json!({ "zooms": true }))["slides"][0]["zooms"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let err = apply(
        &with,
        &out,
        json!([{ "op": "setZoom", "id": id, "targets": ["section:1"] }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let err = apply(
        &src,
        &out,
        json!([{ "op": "setZoom", "id": "slide:1", "target": "slide:2" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);

    let book = tmp("xlsx-set-zoom").with_extension("xlsx");
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
        &tmp("xlsx-set-zoom-out").with_extension("xlsx"),
        json!([{ "op": "setZoom", "id": "slide:1/zoom:3", "target": "slide:2" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

#[test]
fn set_zoom_geom_pos_size_and_rejects() {
    let src = create_deck("geom-host", 3);
    let with = insert_slide_zoom(&src);
    let listed = inspect(&with, json!({ "zooms": true }));
    let id = listed["slides"][0]["zooms"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let old_target = listed["slides"][0]["zooms"][0]["target"].clone();

    let sized = tmp("geom-sz").with_extension("pptx");
    apply(
        &with,
        &sized,
        json!([{
            "op": "setZoom",
            "id": id,
            "x": 914400,
            "y": 457200,
            "cx": 1828800,
            "cy": 914400
        }]),
    )
    .unwrap();
    let tree = inspect(&sized, json!({ "zooms": true }));
    let z = &tree["slides"][0]["zooms"][0];
    assert_eq!(z["x"], 914400);
    assert_eq!(z["y"], 457200);
    assert_eq!(z["cx"], 1828800);
    assert_eq!(z["cy"], 914400);
    assert_eq!(z["target"], old_target);
    assert_eq!(z["type"], "slide");
    let xml =
        String::from_utf8(zip_entry(&sized, "ppt/slides/slide1.xml").unwrap_or_default()).unwrap();
    assert!(xml.contains("cx=\"1828800\""));
    assert!(xml.contains("cy=\"914400\""));
    assert!(xml.contains("x=\"914400\""));
    assert!(!xml.contains("<p:timing"));

    let retarget = tmp("geom-tgt").with_extension("pptx");
    apply(
        &sized,
        &retarget,
        json!([{ "op": "setZoom", "id": id, "target": "slide:3" }]),
    )
    .unwrap();
    let after = inspect(&retarget, json!({ "zooms": true }));
    let z2 = &after["slides"][0]["zooms"][0];
    assert_eq!(z2["target"], "slide:3");
    assert_eq!(z2["cx"], 1828800);
    assert_eq!(z2["x"], 914400);

    let notes_src = tmp("geom-notes").with_extension("pptx");
    apply(
        &src,
        &notes_src,
        json!([{
            "op": "insertZoom",
            "id": "slide:1/notes",
            "type": "slide",
            "target": "slide:2"
        }]),
    )
    .unwrap();
    let notes0 = inspect(&notes_src, json!({ "zooms": true, "notes": true }));
    assert_eq!(notes0["zoomCount"], 0);
    let n_id = notes0["slides"][0]["notes"]["zooms"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let notes_sz = tmp("geom-notes-sz").with_extension("pptx");
    apply(
        &notes_src,
        &notes_sz,
        json!([{ "op": "setZoom", "id": n_id, "cx": 1828800, "cy": 914400 }]),
    )
    .unwrap();
    let notes = inspect(&notes_sz, json!({ "zooms": true, "notes": true }));
    assert_eq!(notes["zoomCount"], 0);
    assert_eq!(notes["slides"][0]["notes"]["zooms"][0]["cx"], 1828800);
    assert_eq!(notes["slides"][0]["notes"]["zooms"][0]["cy"], 914400);

    let bad_empty = tmp("geom-empty").with_extension("pptx");
    let err = apply(&with, &bad_empty, json!([{ "op": "setZoom", "id": id }])).unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!bad_empty.exists());

    let bad_pair = tmp("geom-pair").with_extension("pptx");
    let err = apply(
        &with,
        &bad_pair,
        json!([{ "op": "setZoom", "id": id, "cx": 1828800 }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!bad_pair.exists());
}
