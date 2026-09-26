//! 幻灯片图：名单、换/插/删、尺寸、位置、导出。

use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::json;
use zip::ZipArchive;

const PNG_1X1: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
    0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08, 0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00,
    0x00, 0x03, 0x01, 0x01, 0x00, 0x18, 0xDD, 0x8D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E,
    0x44, 0xAE, 0x42, 0x60, 0x82,
];

const PNG_1X1_B: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
    0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08, 0xD7, 0x63, 0x60, 0x60, 0x60, 0x00,
    0x00, 0x00, 0x04, 0x00, 0x01, 0xA3, 0x0A, 0x15, 0xE3, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E,
    0x44, 0xAE, 0x42, 0x60, 0x82,
];

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-pptx-pictures");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn tmp(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}"))
}

fn write_png(tag: &str, bytes: &[u8]) -> PathBuf {
    let path = tmp(tag).with_extension("png");
    std::fs::write(&path, bytes).unwrap();
    path
}

fn create_deck(tag: &str) -> PathBuf {
    let path = tmp(tag).with_extension("pptx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "presentation": { "slides": [{ "shapes": [{ "text": "标题" }] }] }
        })
        .to_string(),
        None,
    )
    .unwrap();
    path
}

fn apply(src: &Path, out: &Path, ops: serde_json::Value) -> Result<RunOutput, ooxp::Error> {
    run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": src.to_string_lossy(),
            "output": out.to_string_lossy(),
            "ops": ops,
        })
        .to_string(),
        None,
    )
}

fn inspect_pics(path: &Path) -> serde_json::Value {
    let RunOutput::Inspect(tree) = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": path.to_string_lossy(),
            "scope": { "pictures": true }
        })
        .to_string(),
        None,
    )
    .unwrap() else {
        panic!("inspect 应返回树");
    };
    tree
}

fn object_keys(v: &serde_json::Value) -> Vec<&str> {
    v.as_object().unwrap().keys().map(String::as_str).collect()
}

fn part_bytes(path: &Path, name: &str) -> Vec<u8> {
    let bytes = std::fs::read(path).unwrap();
    let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut file = zip.by_name(name).unwrap();
    let mut out = Vec::new();
    file.read_to_end(&mut out).unwrap();
    out
}

fn part_names(path: &Path) -> Vec<String> {
    let bytes = std::fs::read(path).unwrap();
    let zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    zip.file_names().map(str::to_string).collect()
}

fn insert_one(src: &Path, out: &Path, png: &Path) {
    apply(
        src,
        out,
        json!([{
            "op": "insertPicture",
            "id": "slide:1",
            "file": png.to_string_lossy(),
            "x": 914400,
            "y": 685800,
            "cx": 1828800,
            "cy": 914400
        }]),
    )
    .unwrap();
}

#[test]
fn inspect_picture_count_without_list() {
    let path = create_deck("count");
    let RunOutput::Inspect(tree) = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": path.to_string_lossy()
        })
        .to_string(),
        None,
    )
    .unwrap() else {
        panic!();
    };
    assert_eq!(tree["pictureCount"], 0);
    assert_eq!(tree["slides"][0]["pictureCount"], 0);
    assert!(tree["slides"][0].get("pictures").is_none());
}

#[test]
fn pictures_scope_does_not_need_slide() {
    let src = create_deck("scope");
    let png = write_png("a", PNG_1X1);
    let inserted = tmp("ins").with_extension("pptx");
    insert_one(&src, &inserted, &png);
    let tree = inspect_pics(&inserted);
    assert!(tree["slides"][0].get("shapes").is_none());
    assert_eq!(tree["slides"][0]["pictures"].as_array().unwrap().len(), 1);
}

#[test]
fn insert_inspect_replace_size_pos_delete_export() {
    let src = create_deck("round");
    let png_a = write_png("a", PNG_1X1);
    let png_b = write_png("b", PNG_1X1_B);
    let inserted = tmp("ins").with_extension("pptx");
    insert_one(&src, &inserted, &png_a);

    let tree = inspect_pics(&inserted);
    assert_eq!(tree["pictureCount"], 1);
    assert_eq!(tree["slides"][0]["pictureCount"], 1);
    let pics = tree["slides"][0]["pictures"].as_array().unwrap();
    assert_eq!(pics.len(), 1);
    assert_eq!(pics[0]["id"], "slide:1/pic:3");
    assert_eq!(pics[0]["cNvPrId"], "3");
    assert_eq!(pics[0]["x"], 914400);
    assert_eq!(pics[0]["y"], 685800);
    assert_eq!(pics[0]["cx"], 1828800);
    assert_eq!(pics[0]["cy"], 914400);
    assert_eq!(pics[0]["stable"], true);
    assert_eq!(
        object_keys(&pics[0]),
        ["id", "cNvPrId", "name", "x", "y", "cx", "cy", "stable"]
    );
    assert_eq!(part_bytes(&inserted, "ppt/media/image1.png"), PNG_1X1);

    let xml = String::from_utf8(part_bytes(&inserted, "ppt/slides/slide1.xml")).unwrap();
    assert!(xml.contains(r#"id="3""#));
    assert!(!xml.contains(r#"<p:cNvPr id="1" name="图片"#));

    let replaced = tmp("rep").with_extension("pptx");
    apply(
        &inserted,
        &replaced,
        json!([{
            "op": "replacePicture",
            "id": "slide:1/pic:3",
            "file": png_b.to_string_lossy()
        }]),
    )
    .unwrap();
    let after_rep = inspect_pics(&replaced);
    assert_eq!(after_rep["slides"][0]["pictures"][0]["x"], 914400);
    assert_eq!(after_rep["slides"][0]["pictures"][0]["cx"], 1828800);
    assert_eq!(part_bytes(&replaced, "ppt/media/image2.png"), PNG_1X1_B);
    let names = part_names(&replaced);
    assert!(!names.iter().any(|n| n == "ppt/media/image1.png"));

    let sized = tmp("sz").with_extension("pptx");
    apply(
        &replaced,
        &sized,
        json!([{
            "op": "setPictureSize",
            "id": "slide:1/pic:3",
            "cx": 2743200,
            "cy": 1371600
        }]),
    )
    .unwrap();
    let after_sz = inspect_pics(&sized);
    assert_eq!(after_sz["slides"][0]["pictures"][0]["cx"], 2743200);
    assert_eq!(after_sz["slides"][0]["pictures"][0]["cy"], 1371600);
    assert_eq!(after_sz["slides"][0]["pictures"][0]["x"], 914400);

    let moved = tmp("pos").with_extension("pptx");
    apply(
        &sized,
        &moved,
        json!([{
            "op": "setPicturePos",
            "id": "slide:1/pic:3",
            "x": 1828800,
            "y": 0
        }]),
    )
    .unwrap();
    let after_pos = inspect_pics(&moved);
    assert_eq!(after_pos["slides"][0]["pictures"][0]["x"], 1828800);
    assert_eq!(after_pos["slides"][0]["pictures"][0]["y"], 0);
    assert_eq!(after_pos["slides"][0]["pictures"][0]["cx"], 2743200);

    let exported = tmp("exp").with_extension("png");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "exportMedia",
            "path": moved.to_string_lossy(),
            "id": "slide:1/pic:3",
            "output": exported.to_string_lossy()
        })
        .to_string(),
        None,
    )
    .unwrap();
    assert_eq!(std::fs::read(&exported).unwrap(), PNG_1X1_B);

    let deleted = tmp("del").with_extension("pptx");
    apply(
        &moved,
        &deleted,
        json!([{ "op": "deletePicture", "id": "slide:1/pic:3" }]),
    )
    .unwrap();
    let after_del = inspect_pics(&deleted);
    assert_eq!(after_del["pictureCount"], 0);
    assert_eq!(
        after_del["slides"][0]["pictures"].as_array().unwrap().len(),
        0
    );
    let names = part_names(&deleted);
    assert!(!names.iter().any(|n| n.starts_with("ppt/media/")));
}

#[test]
fn insert_without_coords_avoids_origin() {
    let src = create_deck("auto");
    let png = write_png("a", PNG_1X1);
    let out = tmp("auto").with_extension("pptx");
    apply(
        &src,
        &out,
        json!([{
            "op": "insertPicture",
            "id": "slide:1",
            "file": png.to_string_lossy()
        }]),
    )
    .unwrap();
    let tree = inspect_pics(&out);
    let pic = &tree["slides"][0]["pictures"][0];
    assert_ne!((pic["x"].as_i64(), pic["y"].as_i64()), (Some(0), Some(0)));
}

#[test]
fn insert_rejects_shape_id_and_wrap_fields() {
    let src = create_deck("bad");
    let png = write_png("a", PNG_1X1);
    let out = tmp("bad").with_extension("pptx");
    let err = apply(
        &src,
        &out,
        json!([{
            "op": "insertPicture",
            "id": "slide:1/shape:2",
            "file": png.to_string_lossy()
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);

    let err = apply(
        &src,
        &out,
        json!([{
            "op": "insertPicture",
            "id": "slide:1",
            "file": png.to_string_lossy(),
            "from": "A1"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

#[test]
fn wrap_and_anchor_still_rejected_on_pptx() {
    let src = create_deck("wrap");
    let png = write_png("a", PNG_1X1);
    let inserted = tmp("ins").with_extension("pptx");
    insert_one(&src, &inserted, &png);
    let out = tmp("wrap").with_extension("pptx");
    let err = apply(
        &inserted,
        &out,
        json!([{
            "op": "setPictureWrap",
            "id": "slide:1/pic:3",
            "kind": "inline"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    let err = apply(
        &inserted,
        &out,
        json!([{
            "op": "setPictureAnchor",
            "id": "slide:1/pic:3",
            "from": "A1"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

#[test]
fn set_picture_pos_rejected_on_xlsx_and_docx() {
    let xlsx = tmp("pos").with_extension("xlsx");
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
    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": xlsx.to_string_lossy(),
            "ops": [{ "op": "setPicturePos", "id": "sheet:Sheet1/pic:1", "x": 1 }]
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let docx = tmp("pos").with_extension("docx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": docx.to_string_lossy(),
            "document": { "paragraphs": [{ "text": "a" }] }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": docx.to_string_lossy(),
            "ops": [{ "op": "setPicturePos", "id": "para:index:1/pic:1", "x": 1 }]
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

#[test]
fn set_shape_text_does_not_touch_picture() {
    let src = create_deck("mix");
    let png = write_png("a", PNG_1X1);
    let inserted = tmp("ins").with_extension("pptx");
    insert_one(&src, &inserted, &png);
    let out = tmp("mix").with_extension("pptx");
    apply(
        &inserted,
        &out,
        json!([{ "op": "setShapeText", "id": "slide:1/shape:2", "text": "新标题" }]),
    )
    .unwrap();
    let tree = inspect_pics(&out);
    assert_eq!(tree["slides"][0]["pictures"][0]["id"], "slide:1/pic:3");
    let RunOutput::Inspect(scoped) = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": out.to_string_lossy(),
            "scope": { "slide": 1 }
        })
        .to_string(),
        None,
    )
    .unwrap() else {
        panic!();
    };
    assert_eq!(scoped["slides"][0]["shapes"][0]["text"], "新标题");
}
