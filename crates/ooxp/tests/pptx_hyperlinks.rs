//! 幻灯片形状级与 run 级超链接：名单、插改删、跳转、跨字段拒绝。

use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::ZipArchive;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-pptx-hyperlinks");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn tmp(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}"))
}

fn create_deck(tag: &str) -> PathBuf {
    let path = tmp(tag).with_extension("pptx");
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

fn inspect_bare(path: &Path) -> Value {
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
        panic!("inspect 应返回树");
    };
    tree
}

fn inspect_links(path: &Path) -> Value {
    let RunOutput::Inspect(tree) = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": path.to_string_lossy(),
            "scope": { "hyperlinks": true }
        })
        .to_string(),
        None,
    )
    .unwrap() else {
        panic!("inspect 应返回树");
    };
    tree
}

fn inspect_shapes(path: &Path) -> Value {
    let RunOutput::Inspect(tree) = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": path.to_string_lossy(),
            "scope": { "slide": 1, "maxShapes": 20 }
        })
        .to_string(),
        None,
    )
    .unwrap() else {
        panic!("inspect 应返回树");
    };
    tree
}

fn object_keys(v: &Value) -> Vec<&str> {
    v.as_object().unwrap().keys().map(String::as_str).collect()
}

fn part_text(path: &Path, name: &str) -> String {
    let bytes = std::fs::read(path).unwrap();
    let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut file = zip.by_name(name).unwrap();
    let mut out = Vec::new();
    file.read_to_end(&mut out).unwrap();
    String::from_utf8(out).unwrap()
}

fn first_shape_id(path: &Path) -> String {
    inspect_shapes(path)["slides"][0]["shapes"][0]["id"]
        .as_str()
        .unwrap()
        .to_string()
}

fn second_shape_id(path: &Path) -> String {
    inspect_shapes(path)["slides"][0]["shapes"][1]["id"]
        .as_str()
        .unwrap()
        .to_string()
}

#[test]
fn inspect_hyperlink_count_without_list() {
    let path = create_deck("bare");
    let tree = inspect_bare(&path);
    assert_eq!(tree["hyperlinkCount"], 0);
    assert_eq!(tree["slides"][0]["hyperlinkCount"], 0);
    assert!(tree["slides"][0].get("hyperlinks").is_none());
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
            "notesSize",
            "customShowCount",
            "showPr",
            "modifyProtection",
            "props",
            "slides"
        ]
    );
    let listed = inspect_links(&path);
    assert_eq!(listed["slides"][0]["hyperlinks"], json!([]));
    assert_eq!(listed["slides"][1]["hyperlinks"], json!([]));
}

#[test]
fn insert_url_inspect_set_delete() {
    let src = create_deck("round");
    let shape = first_shape_id(&src);
    let inserted = tmp("ins").with_extension("pptx");
    apply(
        &src,
        &inserted,
        json!([{
            "op": "insertHyperlink",
            "id": shape,
            "url": "https://example.com/",
            "tooltip": "提示"
        }]),
    )
    .unwrap();
    let xml = part_text(&inserted, "ppt/slides/slide1.xml");
    assert!(xml.contains("hlinkClick"));
    assert!(xml.contains("tooltip=\"提示\""));
    assert!(!xml.contains("ppaction://"));
    let rels = part_text(&inserted, "ppt/slides/_rels/slide1.xml.rels");
    assert!(rels.contains("/hyperlink"));
    assert!(rels.contains("TargetMode=\"External\""));
    assert!(rels.contains("https://example.com/"));

    let tree = inspect_links(&inserted);
    assert_eq!(tree["hyperlinkCount"], 1);
    assert_eq!(tree["slides"][0]["hyperlinkCount"], 1);
    assert_eq!(tree["slides"][1]["hyperlinkCount"], 0);
    let links = tree["slides"][0]["hyperlinks"].as_array().unwrap();
    assert_eq!(links.len(), 1);
    assert_eq!(
        object_keys(&links[0]),
        [
            "id", "index", "kind", "on", "event", "host", "rId", "url", "file", "slide", "tooltip",
            "text"
        ]
    );
    assert_eq!(links[0]["id"], "slide:1/link:1");
    assert_eq!(links[0]["index"], 1);
    assert_eq!(links[0]["kind"], "url");
    assert_eq!(links[0]["on"], "shape");
    assert_eq!(links[0]["event"], "click");
    assert_eq!(links[0]["host"], shape);
    assert_eq!(links[0]["url"], "https://example.com/");
    assert!(links[0]["file"].is_null());
    assert!(links[0]["slide"].is_null());
    assert_eq!(links[0]["tooltip"], "提示");
    assert!(links[0]["text"].is_null());
    let rid = links[0]["rId"].as_str().unwrap();
    assert!(rid.starts_with("rId"));

    let set_out = tmp("set").with_extension("pptx");
    apply(
        &inserted,
        &set_out,
        json!([{
            "op": "setHyperlink",
            "id": "slide:1/link:1",
            "url": "https://example.org/x",
            "tooltip": null
        }]),
    )
    .unwrap();
    let tree_set = inspect_links(&set_out);
    let link = &tree_set["slides"][0]["hyperlinks"][0];
    assert_eq!(link["url"], "https://example.org/x");
    assert!(link["tooltip"].is_null());
    let xml_set = part_text(&set_out, "ppt/slides/slide1.xml");
    assert!(!xml_set.contains("tooltip="));

    let text_out = tmp("text").with_extension("pptx");
    apply(
        &set_out,
        &text_out,
        json!([{ "op": "setShapeText", "id": shape, "text": "改字仍留链" }]),
    )
    .unwrap();
    let tree_text = inspect_links(&text_out);
    assert_eq!(
        tree_text["slides"][0]["hyperlinks"][0]["url"],
        "https://example.org/x"
    );
    let shapes = inspect_shapes(&text_out);
    assert_eq!(shapes["slides"][0]["shapes"][0]["text"], "改字仍留链");

    let del = tmp("del").with_extension("pptx");
    apply(
        &text_out,
        &del,
        json!([{ "op": "deleteHyperlink", "id": "slide:1/link:1" }]),
    )
    .unwrap();
    let tree_del = inspect_links(&del);
    assert_eq!(tree_del["hyperlinkCount"], 0);
    assert_eq!(tree_del["slides"][0]["hyperlinks"], json!([]));
    let xml_del = part_text(&del, "ppt/slides/slide1.xml");
    assert!(!xml_del.contains("hlinkClick"));
    assert!(xml_del.contains("改字仍留链"));
    let rels_del = part_text(&del, "ppt/slides/_rels/slide1.xml.rels");
    assert!(!rels_del.contains("/hyperlink"));
}

#[test]
fn insert_slide_jump_and_file() {
    let src = create_deck("jump");
    let first = first_shape_id(&src);
    let second = second_shape_id(&src);
    let out = tmp("jump-out").with_extension("pptx");
    apply(
        &src,
        &out,
        json!([
            { "op": "insertHyperlink", "id": first, "slide": "slide:2" },
            { "op": "insertHyperlink", "id": second, "file": "file:///C:/data/deck.pptx" }
        ]),
    )
    .unwrap();
    let tree = inspect_links(&out);
    assert_eq!(tree["hyperlinkCount"], 2);
    let a = &tree["slides"][0]["hyperlinks"][0];
    let b = &tree["slides"][0]["hyperlinks"][1];
    assert_eq!(a["kind"], "slide");
    assert_eq!(a["slide"], "slide:2");
    assert!(a["url"].is_null());
    assert_eq!(b["kind"], "file");
    assert_eq!(b["file"], "file:///C:/data/deck.pptx");
    let xml = part_text(&out, "ppt/slides/slide1.xml");
    assert!(xml.contains("ppaction://hlinksldjump"));
    assert!(xml.contains("ppaction://hlinkfile"));
    let rels = part_text(&out, "ppt/slides/_rels/slide1.xml.rels");
    assert!(rels.contains("/slide"));
    assert!(rels.contains("slide2.xml"));
    assert!(rels.contains("/hyperlink"));
}

#[test]
fn reject_bad_id_and_missing_target() {
    let src = create_deck("bad");
    let out = tmp("bad-id").with_extension("pptx");
    let err = apply(
        &src,
        &out,
        json!([{ "op": "insertHyperlink", "id": "slide:1", "url": "https://example.com/" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    assert!(!out.exists());

    let err = apply(
        &src,
        &tmp("miss").with_extension("pptx"),
        json!([{ "op": "insertHyperlink", "id": first_shape_id(&src) }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let err = apply(
        &src,
        &tmp("self").with_extension("pptx"),
        json!([{
            "op": "insertHyperlink",
            "id": first_shape_id(&src),
            "slide": "slide:1"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let err = apply(
        &src,
        &tmp("gone").with_extension("pptx"),
        json!([{
            "op": "insertHyperlink",
            "id": first_shape_id(&src),
            "slide": "slide:9"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::IdNotFound);

    let err = apply(
        &src,
        &tmp("notes-master").with_extension("pptx"),
        json!([{
            "op": "insertHyperlink",
            "id": "notesMaster/shape:2",
            "url": "https://example.com/"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);

    let err = apply(
        &src,
        &tmp("text").with_extension("pptx"),
        json!([{
            "op": "insertHyperlink",
            "id": first_shape_id(&src),
            "url": "https://example.com/",
            "text": "字"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

#[test]
fn already_linked_and_create_unknown_field() {
    let src = create_deck("dup");
    let shape = first_shape_id(&src);
    let one = tmp("one").with_extension("pptx");
    apply(
        &src,
        &one,
        json!([{ "op": "insertHyperlink", "id": shape, "url": "https://example.com/" }]),
    )
    .unwrap();
    let err = apply(
        &one,
        &tmp("two").with_extension("pptx"),
        json!([{ "op": "insertHyperlink", "id": shape, "url": "https://example.org/" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": tmp("create-hl").with_extension("pptx").to_string_lossy(),
            "presentation": {
                "slides": [{ "shapes": [{ "text": "a" }], "hyperlinks": [] }]
            }
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

#[test]
fn set_url_to_slide_jump() {
    let src = create_deck("retarget");
    let shape = first_shape_id(&src);
    let inserted = tmp("ins").with_extension("pptx");
    apply(
        &src,
        &inserted,
        json!([{ "op": "insertHyperlink", "id": shape, "url": "https://example.com/" }]),
    )
    .unwrap();
    let out = tmp("jump").with_extension("pptx");
    apply(
        &inserted,
        &out,
        json!([{ "op": "setHyperlink", "id": "slide:1/link:1", "slide": "slide:2" }]),
    )
    .unwrap();
    let tree = inspect_links(&out);
    let link = &tree["slides"][0]["hyperlinks"][0];
    assert_eq!(link["kind"], "slide");
    assert_eq!(link["slide"], "slide:2");
    assert!(link["url"].is_null());
    let rels = part_text(&out, "ppt/slides/_rels/slide1.xml.rels");
    assert!(rels.contains("slide2.xml"));
    assert!(!rels.contains("/hyperlink"));
}

#[test]
fn insert_run_url_inspect_set_delete() {
    let src = create_deck("run");
    let shape = first_shape_id(&src);
    let inserted = tmp("run-ins").with_extension("pptx");
    apply(
        &src,
        &inserted,
        json!([{
            "op": "insertHyperlink",
            "id": shape,
            "on": "run",
            "url": "https://example.com/run"
        }]),
    )
    .unwrap();
    let xml = part_text(&inserted, "ppt/slides/slide1.xml");
    assert!(xml.contains("<a:hlinkClick"));
    assert!(xml.contains("rPr"));
    let cnv = xml
        .split("<p:cNvPr")
        .nth(1)
        .and_then(|s| s.split("</p:cNvPr>").next())
        .unwrap_or("");
    assert!(!cnv.contains("hlinkClick"));
    let tree = inspect_links(&inserted);
    assert_eq!(tree["hyperlinkCount"], 1);
    let link = &tree["slides"][0]["hyperlinks"][0];
    assert_eq!(link["on"], "run");
    assert_eq!(link["kind"], "url");
    assert_eq!(link["host"], shape);
    assert_eq!(link["url"], "https://example.com/run");
    assert_eq!(link["text"], "标题");

    let set_out = tmp("run-set").with_extension("pptx");
    apply(
        &inserted,
        &set_out,
        json!([{
            "op": "setHyperlink",
            "id": "slide:1/link:1",
            "url": "https://example.org/run"
        }]),
    )
    .unwrap();
    let tree_set = inspect_links(&set_out);
    assert_eq!(
        tree_set["slides"][0]["hyperlinks"][0]["url"],
        "https://example.org/run"
    );
    assert_eq!(tree_set["slides"][0]["hyperlinks"][0]["on"], "run");

    let del = tmp("run-del").with_extension("pptx");
    apply(
        &set_out,
        &del,
        json!([{ "op": "deleteHyperlink", "id": "slide:1/link:1" }]),
    )
    .unwrap();
    let tree_del = inspect_links(&del);
    assert_eq!(tree_del["hyperlinkCount"], 0);
    let xml_del = part_text(&del, "ppt/slides/slide1.xml");
    assert!(!xml_del.contains("hlinkClick"));
    assert!(xml_del.contains("标题"));
}

#[test]
fn insert_run_with_text_appends() {
    let src = create_deck("run-text");
    let shape = first_shape_id(&src);
    let out = tmp("run-text").with_extension("pptx");
    apply(
        &src,
        &out,
        json!([{
            "op": "insertHyperlink",
            "id": shape,
            "on": "run",
            "text": "链",
            "url": "https://example.com/t"
        }]),
    )
    .unwrap();
    let tree = inspect_links(&out);
    let link = &tree["slides"][0]["hyperlinks"][0];
    assert_eq!(link["on"], "run");
    assert_eq!(link["text"], "链");
    let shapes = inspect_shapes(&out);
    assert_eq!(shapes["slides"][0]["shapes"][0]["text"], "标题链");
}

#[test]
fn insert_shape_then_run_orders_shape_first() {
    let src = create_deck("mixed");
    let shape = first_shape_id(&src);
    let out = tmp("mixed").with_extension("pptx");
    apply(
        &src,
        &out,
        json!([
            { "op": "insertHyperlink", "id": shape, "url": "https://example.com/shape" },
            { "op": "insertHyperlink", "id": shape, "on": "run", "url": "https://example.com/run" }
        ]),
    )
    .unwrap();
    let tree = inspect_links(&out);
    assert_eq!(tree["hyperlinkCount"], 2);
    let links = tree["slides"][0]["hyperlinks"].as_array().unwrap();
    assert_eq!(links[0]["on"], "shape");
    assert!(links[0]["text"].is_null());
    assert_eq!(links[1]["on"], "run");
    assert_eq!(links[1]["text"], "标题");
}

#[test]
fn reject_bad_on_and_shape_text() {
    let src = create_deck("on-bad");
    let shape = first_shape_id(&src);
    let err = apply(
        &src,
        &tmp("on-other").with_extension("pptx"),
        json!([{
            "op": "insertHyperlink",
            "id": shape,
            "on": "other",
            "url": "https://example.com/"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let err = apply(
        &src,
        &tmp("shape-text").with_extension("pptx"),
        json!([{
            "op": "insertHyperlink",
            "id": shape,
            "text": "x",
            "url": "https://example.com/"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let linked = tmp("run-twice").with_extension("pptx");
    apply(
        &src,
        &linked,
        json!([{
            "op": "insertHyperlink",
            "id": shape,
            "on": "run",
            "url": "https://example.com/a"
        }]),
    )
    .unwrap();
    let twice_out = tmp("run-twice-out").with_extension("pptx");
    let err = apply(
        &linked,
        &twice_out,
        json!([{
            "op": "insertHyperlink",
            "id": shape,
            "on": "run",
            "url": "https://example.com/b"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!twice_out.exists());
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

fn inspect_scope(path: &Path, scope: Value) -> Value {
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

#[test]
fn insert_hover_on_shape() {
    let src = create_deck("hover");
    let shape = first_shape_id(&src);
    let out = tmp("hover").with_extension("pptx");
    apply(
        &src,
        &out,
        json!([{
            "op": "insertHyperlink",
            "id": shape,
            "event": "hover",
            "url": "https://example.com/hover"
        }]),
    )
    .unwrap();
    let xml = part_text(&out, "ppt/slides/slide1.xml");
    assert!(xml.contains("hlinkHover"));
    let in_cnv = xml.split("<p:cNvPr").any(|chunk| {
        chunk
            .split("</p:cNvPr>")
            .next()
            .unwrap_or("")
            .contains("hlinkHover")
    });
    assert!(in_cnv, "hlinkHover 应在 cNvPr 内");
    let tree = inspect_links(&out);
    let link = &tree["slides"][0]["hyperlinks"][0];
    assert_eq!(link["on"], "shape");
    assert_eq!(link["event"], "hover");
    assert_eq!(link["url"], "https://example.com/hover");
}

#[test]
fn insert_click_then_hover_orders_click_first() {
    let src = create_deck("both-ev");
    let shape = first_shape_id(&src);
    let out = tmp("both-ev").with_extension("pptx");
    apply(
        &src,
        &out,
        json!([
            { "op": "insertHyperlink", "id": shape, "event": "hover", "url": "https://example.com/h" },
            { "op": "insertHyperlink", "id": shape, "url": "https://example.com/c" }
        ]),
    )
    .unwrap();
    let tree = inspect_links(&out);
    let links = tree["slides"][0]["hyperlinks"].as_array().unwrap();
    assert_eq!(links.len(), 2);
    assert_eq!(links[0]["event"], "click");
    assert_eq!(links[0]["url"], "https://example.com/c");
    assert_eq!(links[1]["event"], "hover");
    assert_eq!(links[1]["url"], "https://example.com/h");
}

#[test]
fn insert_run_hover_writes_mouse_over() {
    let src = create_deck("run-hover");
    let shape = first_shape_id(&src);
    let out = tmp("run-hover").with_extension("pptx");
    apply(
        &src,
        &out,
        json!([{
            "op": "insertHyperlink",
            "id": shape,
            "on": "run",
            "event": "hover",
            "url": "https://example.com/rh"
        }]),
    )
    .unwrap();
    let xml = part_text(&out, "ppt/slides/slide1.xml");
    assert!(xml.contains("hlinkMouseOver"));
    let cnv = xml
        .split("<p:cNvPr")
        .nth(1)
        .and_then(|s| s.split("</p:cNvPr>").next())
        .unwrap_or("");
    assert!(!cnv.contains("hlinkHover"));
    let tree = inspect_links(&out);
    let link = &tree["slides"][0]["hyperlinks"][0];
    assert_eq!(link["on"], "run");
    assert_eq!(link["event"], "hover");
}

#[test]
fn insert_on_pic_table_chart_smartart_and_cell() {
    let src = create_deck("objs");
    let png = write_png("pic");
    let with_objs = tmp("objs-host").with_extension("pptx");
    apply(
        &src,
        &with_objs,
        json!([
            { "op": "insertPicture", "id": "slide:1", "file": png.to_string_lossy() },
            {
                "op": "insertTable",
                "id": "slide:1",
                "rows": 2,
                "cols": 2,
                "cells": [["甲", "乙"], ["丙", "丁"]]
            },
            {
                "op": "insertChart",
                "id": "slide:1",
                "type": "col",
                "series": [{ "name": "s", "categories": ["A", "B"], "values": [1, 2] }]
            },
            {
                "op": "insertSmartArt",
                "id": "slide:1",
                "layout": "list",
                "points": [{ "text": "甲" }, { "text": "乙" }]
            }
        ]),
    )
    .unwrap();
    let objs = inspect_scope(
        &with_objs,
        json!({ "pictures": true, "tables": true, "charts": true, "smartArt": true }),
    );
    let pic = objs["slides"][0]["pictures"][0]["id"].as_str().unwrap();
    let table = objs["slides"][0]["tables"][0]["id"].as_str().unwrap();
    let cell = objs["slides"][0]["tables"][0]["rows"][0]["cells"][0]["id"]
        .as_str()
        .unwrap();
    let chart = objs["slides"][0]["charts"][0]["id"].as_str().unwrap();
    let art = objs["slides"][0]["smartArts"][0]["id"].as_str().unwrap();
    assert!(art.contains("smartdiagram:"));
    let out = tmp("objs-links").with_extension("pptx");
    apply(
        &with_objs,
        &out,
        json!([
            { "op": "insertHyperlink", "id": pic, "url": "https://example.com/pic" },
            { "op": "insertHyperlink", "id": table, "url": "https://example.com/table" },
            { "op": "insertHyperlink", "id": cell, "url": "https://example.com/cell" },
            { "op": "insertHyperlink", "id": chart, "url": "https://example.com/chart" },
            { "op": "insertHyperlink", "id": art, "url": "https://example.com/sa" }
        ]),
    )
    .unwrap();
    let tree = inspect_links(&out);
    let links = tree["slides"][0]["hyperlinks"].as_array().unwrap();
    let by_on = |on: &str| {
        links
            .iter()
            .find(|l| l["on"] == on)
            .unwrap_or_else(|| panic!("缺少 on={on}"))
    };
    assert_eq!(by_on("pic")["url"], "https://example.com/pic");
    assert_eq!(by_on("pic")["host"], pic);
    assert_eq!(by_on("table")["url"], "https://example.com/table");
    assert_eq!(by_on("table")["host"], table);
    assert_eq!(by_on("cell")["url"], "https://example.com/cell");
    assert_eq!(by_on("cell")["host"], cell);
    assert_eq!(by_on("cell")["text"], "甲");
    assert_eq!(by_on("chart")["url"], "https://example.com/chart");
    assert_eq!(by_on("chart")["on"], "chart");
    assert_eq!(by_on("smartArt")["url"], "https://example.com/sa");
    assert_eq!(by_on("smartArt")["on"], "smartArt");
}

#[test]
fn create_run_and_hover_link() {
    let path = tmp("create-ev").with_extension("pptx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "presentation": {
                "slides": [{
                    "shapes": [{
                        "text": "悬停",
                        "url": "https://example.com/c-hover",
                        "event": "hover"
                    }, {
                        "text": "run链",
                        "url": "https://example.com/c-run",
                        "on": "run"
                    }]
                }]
            }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let tree = inspect_links(&path);
    let links = tree["slides"][0]["hyperlinks"].as_array().unwrap();
    assert_eq!(links[0]["event"], "hover");
    assert_eq!(links[0]["on"], "shape");
    assert_eq!(links[1]["on"], "run");
    assert_eq!(links[1]["event"], "click");
    let xml = part_text(&path, "ppt/slides/slide1.xml");
    assert!(xml.contains("hlinkHover"));
    assert!(xml.contains("hlinkClick"));
}

#[test]
fn reject_bad_event_and_on_mismatch() {
    let src = create_deck("bad-ev");
    let png = write_png("bad-pic");
    let with_pic = tmp("bad-pic-host").with_extension("pptx");
    apply(
        &src,
        &with_pic,
        json!([{ "op": "insertPicture", "id": "slide:1", "file": png.to_string_lossy() }]),
    )
    .unwrap();
    let pic = inspect_scope(&with_pic, json!({ "pictures": true }))["slides"][0]["pictures"][0]
        ["id"]
        .as_str()
        .unwrap()
        .to_string();
    let err = apply(
        &src,
        &tmp("ev-foo").with_extension("pptx"),
        json!([{
            "op": "insertHyperlink",
            "id": first_shape_id(&src),
            "event": "foo",
            "url": "https://example.com/"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let err = apply(
        &with_pic,
        &tmp("pic-run").with_extension("pptx"),
        json!([{
            "op": "insertHyperlink",
            "id": pic,
            "on": "run",
            "url": "https://example.com/"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}
