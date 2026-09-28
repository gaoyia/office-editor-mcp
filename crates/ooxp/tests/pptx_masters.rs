//! 母版与版式：inspect `masterCount` / `scope.masters` / `layoutId`，以及 apply 换版式与改占位符。

use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

static SEQ: AtomicU32 = AtomicU32::new(0);

const TITLE_PH: &str = concat!(
    r#"<p:sp>"#,
    r#"<p:nvSpPr>"#,
    r#"<p:cNvPr id="2" name="标题 1"/>"#,
    r#"<p:cNvSpPr><a:spLocks noGrp="1"/></p:cNvSpPr>"#,
    r#"<p:nvPr><p:ph type="title"/></p:nvPr>"#,
    r#"</p:nvSpPr>"#,
    r#"<p:spPr><a:xfrm>"#,
    r#"<a:off x="838200" y="365125"/>"#,
    r#"<a:ext cx="10515600" cy="1325563"/>"#,
    r#"</a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></p:spPr>"#,
    r#"<p:txBody><a:bodyPr/><a:lstStyle/>"#,
    r#"<a:p><a:r><a:t>版式标题</a:t></a:r></a:p>"#,
    r#"</p:txBody></p:sp>"#
);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-pptx-masters");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn tmp(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}.pptx"))
}

fn create_deck(tag: &str) -> PathBuf {
    let path = tmp(tag);
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "presentation": {
                "slides": [
                    { "shapes": [{ "text": "标题" }, { "text": "第二框" }] },
                    { "shapes": [{ "text": "第二页" }] }
                ]
            }
        })
        .to_string(),
        None,
    )
    .unwrap();
    path
}

fn apply(src: &Path, out: &Path, ops: Value) -> Result<RunOutput, ooxp::Error> {
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

fn inspect(path: &Path) -> Value {
    inspect_scope(path, None)
}

fn inspect_scope(path: &Path, scope: Option<Value>) -> Value {
    let mut req = json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": path.to_string_lossy()
    });
    if let Some(scope) = scope {
        req["scope"] = scope;
    }
    let RunOutput::Inspect(tree) = run(&req.to_string(), None).unwrap() else {
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

fn object_keys(v: &Value) -> Vec<&str> {
    v.as_object().unwrap().keys().map(|s| s.as_str()).collect()
}

fn part(path: &Path, name: &str) -> String {
    String::from_utf8(part_bytes(path, name)).unwrap()
}

fn part_bytes(path: &Path, name: &str) -> Vec<u8> {
    let bytes = std::fs::read(path).unwrap();
    let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut file = zip.by_name(name).unwrap();
    let mut out = Vec::new();
    file.read_to_end(&mut out).unwrap();
    out
}

fn rewrite_part(path: &Path, name: &str, f: impl FnOnce(Vec<u8>) -> Vec<u8>) {
    let bytes = std::fs::read(path).unwrap();
    let mut zip_in = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut out = Cursor::new(Vec::new());
    {
        let mut zip_out = ZipWriter::new(&mut out);
        let opt = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        let mut transform = Some(f);
        for i in 0..zip_in.len() {
            let mut file = zip_in.by_index(i).unwrap();
            let n = file.name().to_string();
            let mut data = Vec::new();
            file.read_to_end(&mut data).unwrap();
            if n == name {
                data = transform.take().expect("部件只应出现一次")(data);
            }
            zip_out.start_file(&n, opt).unwrap();
            zip_out.write_all(&data).unwrap();
        }
        zip_out.finish().unwrap();
    }
    std::fs::write(path, out.into_inner()).unwrap();
}

fn inject_placeholder(path: &Path) {
    rewrite_part(path, "ppt/slideLayouts/slideLayout1.xml", |data| {
        let xml = String::from_utf8(data).unwrap();
        xml.replace("</p:spTree>", &format!("{TITLE_PH}</p:spTree>"))
            .into_bytes()
    });
}

fn add_second_layout(path: &Path) {
    let layout1 = part_bytes(path, "ppt/slideLayouts/slideLayout1.xml");
    let bytes = std::fs::read(path).unwrap();
    let mut zip_in = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut out = Cursor::new(Vec::new());
    {
        let mut zip_out = ZipWriter::new(&mut out);
        let opt = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        let mut wrote_layout2 = false;
        for i in 0..zip_in.len() {
            let mut file = zip_in.by_index(i).unwrap();
            let n = file.name().to_string();
            let mut data = Vec::new();
            file.read_to_end(&mut data).unwrap();
            if n == "[Content_Types].xml" {
                let xml = String::from_utf8(data).unwrap();
                data = xml
                    .replace(
                        "</Types>",
                        concat!(
                            r#"<Override PartName="/ppt/slideLayouts/slideLayout2.xml" "#,
                            r#"ContentType="application/vnd.openxmlformats-officedocument.presentationml.slideLayout+xml"/>"#,
                            "</Types>"
                        ),
                    )
                    .into_bytes();
            }
            if n == "ppt/slideMasters/_rels/slideMaster1.xml.rels" {
                let xml = String::from_utf8(data).unwrap();
                data = xml
                    .replace(
                        "</Relationships>",
                        concat!(
                            r#"<Relationship Id="rId3" "#,
                            r#"Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" "#,
                            r#"Target="../slideLayouts/slideLayout2.xml"/>"#,
                            "</Relationships>"
                        ),
                    )
                    .into_bytes();
            }
            if n == "ppt/slideMasters/slideMaster1.xml" {
                let xml = String::from_utf8(data).unwrap();
                data = xml
                    .replace(
                        "</p:sldLayoutIdLst>",
                        r#"<p:sldLayoutId id="2147483650" r:id="rId3"/></p:sldLayoutIdLst>"#,
                    )
                    .into_bytes();
            }
            zip_out.start_file(&n, opt).unwrap();
            zip_out.write_all(&data).unwrap();
            if n == "ppt/slideLayouts/slideLayout1.xml" {
                zip_out
                    .start_file("ppt/slideLayouts/slideLayout2.xml", opt)
                    .unwrap();
                zip_out.write_all(&layout1).unwrap();
                wrote_layout2 = true;
            }
            if n == "ppt/slideLayouts/_rels/slideLayout1.xml.rels" {
                zip_out
                    .start_file("ppt/slideLayouts/_rels/slideLayout2.xml.rels", opt)
                    .unwrap();
                zip_out.write_all(&data).unwrap();
            }
        }
        assert!(wrote_layout2, "应写入第二份版式");
        zip_out.finish().unwrap();
    }
    std::fs::write(path, out.into_inner()).unwrap();
}

#[test]
fn create_has_master_count_without_list() {
    let path = create_deck("bare");
    let tree = inspect(&path);
    assert_eq!(tree["masterCount"], 1);
    assert!(tree.get("masters").is_none());
    assert_eq!(tree["slides"][0]["layoutId"], "master:1/layout:1");
    assert_eq!(tree["slides"][1]["layoutId"], "master:1/layout:1");
    assert_eq!(
        object_keys(&tree),
        [
            "schemaVersion",
            "format",
            "id",
            "slideCount",
            "pictureCount",
            "videoCount",
            "audioCount",
            "chartCount",
            "smartArtCount",
            "commentCount",
            "hyperlinkCount",
            "tableCount",
            "animCount",
            "masterCount",
            "hasHandout",
            "props",
            "slides"
        ]
    );
}

#[test]
fn masters_scope_lists_blank_layout() {
    let path = create_deck("scope");
    let tree = inspect_scope(&path, Some(json!({ "masters": true })));
    assert!(tree["slides"][0].get("shapes").is_none());
    assert_eq!(tree["masterCount"], 1);
    let masters = tree["masters"].as_array().unwrap();
    assert_eq!(masters.len(), 1);
    assert_eq!(masters[0]["id"], "master:1");
    assert_eq!(masters[0]["index"], 1);
    assert_eq!(masters[0]["placeholders"].as_array().unwrap().len(), 0);
    let layouts = masters[0]["layouts"].as_array().unwrap();
    assert_eq!(layouts.len(), 1);
    assert_eq!(layouts[0]["id"], "master:1/layout:1");
    assert_eq!(layouts[0]["name"], "空白");
    assert_eq!(layouts[0]["kind"], "blank");
    assert_eq!(layouts[0]["placeholders"].as_array().unwrap().len(), 0);
    assert_eq!(
        object_keys(&tree),
        [
            "schemaVersion",
            "format",
            "id",
            "slideCount",
            "pictureCount",
            "videoCount",
            "audioCount",
            "chartCount",
            "smartArtCount",
            "commentCount",
            "hyperlinkCount",
            "tableCount",
            "animCount",
            "masterCount",
            "hasHandout",
            "props",
            "masters",
            "slides"
        ]
    );
}

#[test]
fn inspect_and_edit_layout_placeholder() {
    let path = create_deck("ph");
    inject_placeholder(&path);
    let tree = inspect_scope(&path, Some(json!({ "masters": true })));
    let ph = &tree["masters"][0]["layouts"][0]["placeholders"][0];
    assert_eq!(ph["id"], "master:1/layout:1/shape:2");
    assert_eq!(ph["shapeId"], "2");
    assert_eq!(ph["name"], "标题 1");
    assert_eq!(ph["type"], "title");
    assert_eq!(ph["idx"], Value::Null);
    assert_eq!(ph["text"], "版式标题");
    assert_eq!(ph["x"], 838200);
    assert_eq!(ph["stable"], true);
    assert_eq!(
        object_keys(ph),
        ["id", "shapeId", "name", "type", "idx", "x", "y", "cx", "cy", "text", "stable"]
    );

    let out = tmp("ph-text");
    apply(
        &path,
        &out,
        json!([{
            "op": "setShapeText",
            "id": "master:1/layout:1/shape:2",
            "text": "改过的标题"
        }]),
    )
    .unwrap();
    let tree = inspect_scope(&out, Some(json!({ "masters": true })));
    assert_eq!(
        tree["masters"][0]["layouts"][0]["placeholders"][0]["text"],
        "改过的标题"
    );
    assert_eq!(
        inspect_scope(&out, Some(json!({ "slide": 1 })))["slides"][0]["shapes"][0]["text"],
        "标题"
    );

    let moved = tmp("ph-pos");
    apply(
        &out,
        &moved,
        json!([{
            "op": "setShapePos",
            "id": "master:1/layout:1/shape:2",
            "x": 100000,
            "y": 200000
        }]),
    )
    .unwrap();
    let tree = inspect_scope(&moved, Some(json!({ "masters": true })));
    assert_eq!(
        tree["masters"][0]["layouts"][0]["placeholders"][0]["x"],
        100000
    );
    assert_eq!(
        tree["masters"][0]["layouts"][0]["placeholders"][0]["y"],
        200000
    );
}

#[test]
fn set_slide_layout_changes_rel_keeps_user_shapes() {
    let path = create_deck("layout");
    inject_placeholder(&path);
    add_second_layout(&path);
    let tree = inspect_scope(&path, Some(json!({ "masters": true })));
    assert_eq!(tree["masters"][0]["layouts"].as_array().unwrap().len(), 2);
    assert_eq!(tree["slides"][0]["layoutId"], "master:1/layout:1");

    let out = tmp("switched");
    apply(
        &path,
        &out,
        json!([{
            "op": "setSlideLayout",
            "id": "slide:1",
            "layout": "master:1/layout:2"
        }]),
    )
    .unwrap();
    let tree = inspect_scope(&out, Some(json!({ "slide": 1, "masters": true })));
    assert_eq!(tree["slides"][0]["layoutId"], "master:1/layout:2");
    assert_eq!(tree["slides"][0]["shapes"][0]["text"], "标题");
    assert_eq!(tree["slides"][0]["shapes"][1]["text"], "第二框");
    let rels = part(&out, "ppt/slides/_rels/slide1.xml.rels");
    assert!(rels.contains("slideLayout2.xml"));
    let slide = part(&out, "ppt/slides/slide1.xml");
    assert!(slide.contains("标题"));
}

#[test]
fn set_slide_layout_missing_and_bad_id() {
    let path = create_deck("bad");
    let missing = tmp("missing-layout");
    let err = apply(
        &path,
        &missing,
        json!([{
            "op": "setSlideLayout",
            "id": "slide:1",
            "layout": "master:1/layout:9"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::IdNotFound);
    assert!(!missing.exists());

    let bad_id = tmp("bad-id");
    let err = apply(
        &path,
        &bad_id,
        json!([{
            "op": "setSlideLayout",
            "id": "master:1/layout:1",
            "layout": "master:1/layout:1"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    assert!(!bad_id.exists());

    let bad_layout = tmp("bad-layout");
    let err = apply(
        &path,
        &bad_layout,
        json!([{
            "op": "setSlideLayout",
            "id": "slide:1",
            "layout": "layout:1"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    assert!(!bad_layout.exists());
}

#[test]
fn fill_on_master_id_is_invalid_id() {
    let path = create_deck("fill");
    inject_placeholder(&path);
    let out = tmp("fill-out");
    let err = apply(
        &path,
        &out,
        json!([{
            "op": "setShapeFill",
            "id": "master:1/layout:1/shape:2",
            "color": "FF0000"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    assert!(!out.exists());
}

#[test]
fn create_rejects_presentation_masters() {
    let bad = tmp("create-masters");
    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": bad.to_string_lossy(),
            "presentation": {
                "masters": [],
                "slides": [{ "shapes": [{ "text": "甲" }] }]
            }
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!bad.exists());
}

#[test]
fn xlsx_and_docx_reject_masters_and_set_slide_layout() {
    let xlsx = tmp_dir().join("scope.xlsx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": xlsx.to_string_lossy(),
            "workbook": { "sheets": [{ "name": "Sheet1", "cells": [{ "ref": "A1", "value": 1 }] }] }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let err = inspect_err(&xlsx, json!({ "masters": true }));
    assert_eq!(err.code(), ErrorCode::InvalidField);
    let out = tmp_dir().join("xlsx-layout.xlsx");
    let err = apply(
        &xlsx,
        &out,
        json!([{ "op": "setSlideLayout", "id": "slide:1", "layout": "master:1/layout:1" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());

    let docx = tmp_dir().join("scope.docx");
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
    let err = inspect_err(&docx, json!({ "masters": true }));
    assert_eq!(err.code(), ErrorCode::InvalidField);
    let out = tmp_dir().join("docx-layout.docx");
    let err = apply(
        &docx,
        &out,
        json!([{ "op": "setSlideLayout", "id": "slide:1", "layout": "master:1/layout:1" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());
}
