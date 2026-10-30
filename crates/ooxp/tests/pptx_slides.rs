//! 幻灯片管理：插、删、重排、隐藏，以及 id 拒绝与未点名页保真。

use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-pptx-slides");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn tmp(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}.pptx"))
}

fn create_deck(tag: &str) -> PathBuf {
    let path = tmp(tag);
    run(&json!({
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
    .to_string())
    .unwrap();
    path
}

fn create_one(tag: &str) -> PathBuf {
    let path = tmp(tag);
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "presentation": {
            "slides": [{ "shapes": [{ "text": "仅一页" }] }]
        }
    })
    .to_string())
    .unwrap();
    path
}

fn apply(src: &Path, out: &Path, ops: Value) -> Result<RunOutput, ooxp::Error> {
    run(&json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": out.to_string_lossy(),
        "ops": ops,
    })
    .to_string())
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
    let RunOutput::Inspect(tree) = run(&req.to_string()).unwrap() else {
        panic!("inspect 应返回树");
    };
    tree
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
    let mut transform = Some(f);
    {
        let mut zip_out = ZipWriter::new(&mut out);
        let opt = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
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

fn slide_texts(tree: &Value, i: usize) -> Vec<String> {
    tree["slides"][i]["shapes"]
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .map(|s| s["text"].as_str().unwrap_or("").to_string())
        .collect()
}

#[test]
fn inspect_lists_hidden() {
    let path = create_deck("hidden");
    let tree = inspect(&path);
    assert_eq!(tree["slideCount"], 2);
    assert_eq!(tree["slides"][0]["id"], "slide:1");
    assert_eq!(tree["slides"][1]["id"], "slide:2");
    assert_eq!(tree["slides"][0]["hidden"], false);
    assert_eq!(tree["slides"][1]["hidden"], false);
}

#[test]
fn insert_after_first_is_blank_and_shifts() {
    let src = create_deck("ins");
    let out = tmp("ins-out");
    apply(
        &src,
        &out,
        json!([{
            "op": "insertSlide",
            "id": "pptx:presentation",
            "after": "slide:1"
        }]),
    )
    .unwrap();
    let tree = inspect_scope(&out, Some(json!({ "slide": 2 })));
    assert_eq!(tree["slideCount"], 3);
    assert_eq!(tree["slides"][1]["id"], "slide:2");
    let shapes = tree["slides"][1]["shapes"].as_array().unwrap();
    assert!(shapes.is_empty());
    let page3 = inspect_scope(&out, Some(json!({ "slide": 3 })));
    assert_eq!(slide_texts(&page3, 2), ["第二页"]);
}

#[test]
fn move_last_to_front() {
    let src = create_deck("mov");
    let inserted = tmp("mov-ins");
    apply(
        &src,
        &inserted,
        json!([{
            "op": "insertSlide",
            "id": "pptx:presentation",
            "after": "slide:1"
        }]),
    )
    .unwrap();
    let out = tmp("mov-out");
    apply(
        &inserted,
        &out,
        json!([{ "op": "moveSlide", "id": "slide:3" }]),
    )
    .unwrap();
    let tree = inspect_scope(&out, Some(json!({ "slide": 1 })));
    assert_eq!(slide_texts(&tree, 0), ["第二页"]);
}

#[test]
fn hide_and_unhide() {
    let src = create_deck("hid");
    let hidden = tmp("hid-out");
    apply(
        &src,
        &hidden,
        json!([{ "op": "setSlideHidden", "id": "slide:2", "hidden": true }]),
    )
    .unwrap();
    let tree = inspect(&hidden);
    assert_eq!(tree["slides"][1]["hidden"], true);
    assert_eq!(tree["slides"][0]["hidden"], false);
    let shown = tmp("hid-show");
    apply(
        &hidden,
        &shown,
        json!([{ "op": "setSlideHidden", "id": "slide:2", "hidden": false }]),
    )
    .unwrap();
    assert_eq!(inspect(&shown)["slides"][1]["hidden"], false);
}

#[test]
fn refuse_last_slide_and_missing_hidden() {
    let one = create_one("one");
    let dest = tmp("del-one");
    let err = apply(
        &one,
        &dest,
        json!([{ "op": "deleteSlide", "id": "slide:1" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!dest.exists());
    assert_eq!(inspect(&one)["slideCount"], 1);

    let src = create_deck("miss");
    let err = apply(
        &src,
        &tmp("no-hidden"),
        json!([{ "op": "setSlideHidden", "id": "slide:1" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

#[test]
fn delete_middle_and_wrong_ids() {
    let src = create_deck("del");
    let inserted = tmp("del-ins");
    apply(
        &src,
        &inserted,
        json!([{
            "op": "insertSlide",
            "id": "pptx:presentation",
            "after": "slide:1"
        }]),
    )
    .unwrap();
    let out = tmp("del-out");
    apply(
        &inserted,
        &out,
        json!([{ "op": "deleteSlide", "id": "slide:2" }]),
    )
    .unwrap();
    let tree = inspect_scope(&out, Some(json!({ "slide": 2 })));
    assert_eq!(tree["slideCount"], 2);
    assert_eq!(slide_texts(&tree, 1), ["第二页"]);

    let err = apply(
        &out,
        &tmp("ins-bad"),
        json!([{ "op": "insertSlide", "id": "slide:1" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);

    let err = apply(
        &out,
        &tmp("del-bad"),
        json!([{ "op": "deleteSlide", "id": "pptx:presentation" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);

    let err = apply(
        &out,
        &tmp("mov-self"),
        json!([{ "op": "moveSlide", "id": "slide:1", "after": "slide:1" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

#[test]
fn later_op_keeps_original_slide() {
    let src = create_deck("seq");
    let out = tmp("seq-out");
    apply(
        &src,
        &out,
        json!([
            {
                "op": "insertSlide",
                "id": "pptx:presentation",
                "after": "slide:1"
            },
            {
                "op": "setShapeText",
                "id": "slide:2/shape:2",
                "text": "改过的第二页"
            }
        ]),
    )
    .unwrap();
    let tree = inspect_scope(&out, Some(json!({ "slide": 3 })));
    assert_eq!(slide_texts(&tree, 2), ["改过的第二页"]);
}

#[test]
fn delete_then_edit_uses_original_slide() {
    let path = tmp("pin-del");
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "presentation": {
            "slides": [
                { "shapes": [{ "text": "第一页" }] },
                { "shapes": [{ "text": "第二页" }] },
                { "shapes": [{ "text": "第三页" }] }
            ]
        }
    })
    .to_string())
    .unwrap();
    let out = tmp("pin-del-out");
    apply(
        &path,
        &out,
        json!([
            { "op": "deleteSlide", "id": "slide:1" },
            { "op": "setShapeText", "id": "slide:3/shape:2", "text": "仍是第三页" }
        ]),
    )
    .unwrap();
    let tree = inspect_scope(&out, Some(json!({ "slide": 2 })));
    assert_eq!(slide_texts(&tree, 1), ["仍是第三页"]);
}

#[test]
fn move_then_edit_uses_original_slide() {
    let src = create_deck("pin-mov");
    let out = tmp("pin-mov-out");
    apply(
        &src,
        &out,
        json!([
            { "op": "moveSlide", "id": "slide:2" },
            { "op": "setShapeText", "id": "slide:2/shape:2", "text": "搬走的第二页" }
        ]),
    )
    .unwrap();
    let tree = inspect_scope(&out, Some(json!({ "slide": 1 })));
    assert_eq!(slide_texts(&tree, 0), ["搬走的第二页"]);
}

#[test]
fn insert_keeps_named_slide_bytes() {
    let src = create_deck("fid");
    rewrite_part(&src, "ppt/slides/slide1.xml", |data| {
        let mut xml = String::from_utf8(data).unwrap();
        xml = xml.replace(
            "</p:sld>",
            "<p:timing><p:tnLst><p:par><p:cTn id=\"1\" dur=\"indefinite\" restart=\"never\"/></p:par></p:tnLst></p:timing></p:sld>",
        );
        xml.into_bytes()
    });
    let before = part_bytes(&src, "ppt/slides/slide1.xml");
    let out = tmp("fid-out");
    apply(
        &src,
        &out,
        json!([{
            "op": "insertSlide",
            "id": "pptx:presentation",
            "after": "slide:1"
        }]),
    )
    .unwrap();
    assert_eq!(part_bytes(&out, "ppt/slides/slide1.xml"), before);
    let moved = tmp("fid-mov");
    apply(
        &out,
        &moved,
        json!([{ "op": "moveSlide", "id": "slide:3" }]),
    )
    .unwrap();
    assert_eq!(part_bytes(&moved, "ppt/slides/slide1.xml"), before);
}

fn four_slides(tag: &str) -> PathBuf {
    let path = tmp(tag);
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "presentation": {
            "slides": [
                { "shapes": [{ "text": "S1" }] },
                { "shapes": [{ "text": "S2" }] },
                { "shapes": [{ "text": "S3" }] },
                { "shapes": [{ "text": "S4" }] }
            ]
        }
    })
    .to_string())
    .unwrap();
    path
}

fn counts(path: &Path, key: &str) -> Vec<u64> {
    inspect(path)["slides"]
        .as_array()
        .unwrap()
        .iter()
        .map(|slide| slide[key].as_u64().unwrap_or(0))
        .collect()
}

fn shape_on_third(path: &Path) -> String {
    let tree = inspect_scope(path, Some(json!({ "slide": 3 })));
    tree["slides"][2]["shapes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|shape| shape["text"] == "S3")
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string()
}

#[test]
fn delete_then_object_ops_keep_original_slide() {
    let src = four_slides("obj");
    let shape = shape_on_third(&src);
    let cases = [
        (
            "table",
            json!({ "op": "insertTable", "id": "slide:3", "rows": 1, "cols": 1 }),
            "tableCount",
        ),
        (
            "connector",
            json!({ "op": "insertConnector", "id": "slide:3" }),
            "connectorCount",
        ),
        (
            "math",
            json!({ "op": "insertMath", "id": "slide:3", "text": "x" }),
            "mathCount",
        ),
        (
            "field",
            json!({ "op": "insertField", "id": shape.clone(), "type": "slidenum", "text": "3" }),
            "fieldCount",
        ),
        (
            "link",
            json!({ "op": "insertHyperlink", "id": shape, "url": "https://example.com/" }),
            "hyperlinkCount",
        ),
    ];
    for (tag, op, key) in cases {
        let out = tmp(tag);
        apply(
            &src,
            &out,
            json!([
                { "op": "deleteSlide", "id": "slide:1" },
                op
            ]),
        )
        .unwrap();
        assert_eq!(counts(&out, key), [0, 1, 0], "{tag}");
    }
}

fn part_text(path: &Path, name: &str) -> String {
    String::from_utf8(part_bytes(path, name)).unwrap()
}

fn rel_target_of(path: &Path, rels: &str, rid: &str) -> String {
    let xml = part_text(path, rels);
    let at = xml.find(&format!("Id=\"{rid}\"")).unwrap();
    let rest = &xml[at..];
    let start = rest.find("Target=\"").unwrap() + "Target=\"".len();
    rest[start..start + rest[start..].find('"').unwrap()].to_string()
}

/// A 节是 S1–S3，B 节是 S4。
fn four_slides_two_sections(tag: &str) -> PathBuf {
    let src = four_slides(&format!("{tag}-src"));
    let out = tmp(tag);
    apply(
        &src,
        &out,
        json!([
            { "op": "insertSlideSection", "id": "pptx:presentation", "name": "A" },
            { "op": "insertSlideSection", "id": "pptx:presentation", "name": "B" },
            { "op": "setSlideSection", "id": "slide:4", "section": "section:2" }
        ]),
    )
    .unwrap();
    out
}

fn section_slides(path: &Path) -> Vec<Value> {
    inspect_scope(path, Some(json!({ "sections": true })))["sections"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["slides"].clone())
        .collect()
}

#[test]
fn delete_then_slide_refs_keep_original_slide() {
    let src = four_slides("refs");

    let zoom = tmp("refs-zoom");
    apply(
        &src,
        &zoom,
        json!([
            { "op": "deleteSlide", "id": "slide:1" },
            { "op": "insertZoom", "id": "slide:2", "type": "slide", "target": "slide:3" }
        ]),
    )
    .unwrap();
    let rels = part_text(&zoom, "ppt/slides/_rels/slide2.xml.rels");
    assert!(rels.contains("Target=\"slide3.xml\""), "{rels}");
    assert!(!rels.contains("Target=\"slide4.xml\""), "{rels}");

    let show = tmp("refs-show");
    apply(
        &src,
        &show,
        json!([
            { "op": "deleteSlide", "id": "slide:1" },
            { "op": "insertCustomShow", "id": "pptx:presentation", "name": "T", "slides": ["slide:3"] }
        ]),
    )
    .unwrap();
    let pres = part_text(&show, "ppt/presentation.xml");
    let at = pres.find("<p:sld r:id=\"").unwrap() + "<p:sld r:id=\"".len();
    let rid = &pres[at..at + pres[at..].find('"').unwrap()];
    assert_eq!(
        rel_target_of(&show, "ppt/_rels/presentation.xml.rels", rid),
        "slides/slide3.xml"
    );
}

#[test]
fn delete_then_section_ops_keep_original_slide() {
    let src = four_slides_two_sections("secs");

    let moved = tmp("secs-set");
    apply(
        &src,
        &moved,
        json!([
            { "op": "deleteSlide", "id": "slide:1" },
            { "op": "setSlideSection", "id": "slide:3", "section": "section:2" }
        ]),
    )
    .unwrap();
    assert_eq!(
        section_slides(&moved),
        [json!(["slide:1"]), json!(["slide:2", "slide:3"])]
    );

    let started = tmp("secs-start");
    apply(
        &src,
        &started,
        json!([
            { "op": "deleteSlide", "id": "slide:1" },
            { "op": "insertSlideSection", "id": "pptx:presentation", "name": "C", "start": "slide:3" }
        ]),
    )
    .unwrap();
    assert_eq!(
        section_slides(&started),
        [json!(["slide:1"]), json!(["slide:2"]), json!(["slide:3"])]
    );

    let inserted = tmp("secs-insert");
    apply(
        &src,
        &inserted,
        json!([
            { "op": "deleteSlide", "id": "slide:1" },
            { "op": "insertSlide", "id": "pptx:presentation", "after": "slide:3" }
        ]),
    )
    .unwrap();
    assert_eq!(
        section_slides(&inserted),
        [json!(["slide:1", "slide:2", "slide:3"]), json!(["slide:4"])]
    );
}

#[test]
fn foreign_format_rejects() {
    let xlsx = tmp("book").with_extension("xlsx");
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": xlsx.to_string_lossy(),
        "workbook": { "sheets": [{ "name": "Sheet1", "cells": [{ "ref": "A1", "value": 1 }] }] }
    })
    .to_string())
    .unwrap();
    let err = run(&json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": xlsx.to_string_lossy(),
        "output": tmp("xlsx-out").with_extension("xlsx").to_string_lossy(),
        "ops": [{ "op": "insertSlide", "id": "pptx:presentation" }]
    })
    .to_string())
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let docx = tmp("doc").with_extension("docx");
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": docx.to_string_lossy(),
        "document": { "paragraphs": [{ "text": "hi" }] }
    })
    .to_string())
    .unwrap();
    let err = run(&json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": docx.to_string_lossy(),
        "output": tmp("docx-out").with_extension("docx").to_string_lossy(),
        "ops": [{ "op": "deleteSlide", "id": "slide:1" }]
    })
    .to_string())
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}
