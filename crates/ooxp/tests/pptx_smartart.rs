//! 幻灯片 SmartArt：名单、插删、改点、画布坐标与 create。

use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

const UID_LIST: &str = "urn:microsoft.com/office/officeart/2005/8/layout/default";
const UID_ORG: &str = "urn:microsoft.com/office/officeart/2005/8/layout/orgChart1";
const UID_CYCLE: &str = "urn:microsoft.com/office/officeart/2005/8/layout/cycle2";

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-pptx-smartart");
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
            "presentation": { "slides": [{ "shapes": [{ "text": "标题" }] }] }
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

fn inspect_smart(path: &Path) -> Value {
    let RunOutput::Inspect(tree) = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": path.to_string_lossy(),
            "scope": { "smartArt": true }
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

fn part_names(path: &Path) -> Vec<String> {
    let bytes = std::fs::read(path).unwrap();
    let zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    zip.file_names().map(str::to_string).collect()
}

fn part_text(path: &Path, name: &str) -> String {
    let bytes = std::fs::read(path).unwrap();
    let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut file = zip.by_name(name).unwrap();
    let mut out = Vec::new();
    file.read_to_end(&mut out).unwrap();
    String::from_utf8(out).unwrap()
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

fn insert_list(src: &Path, out: &Path) {
    apply(
        src,
        out,
        json!([{
            "op": "insertSmartArt",
            "id": "slide:1",
            "layout": "list",
            "points": [
                { "text": "甲" },
                { "text": "乙" },
                { "text": "丙" }
            ]
        }]),
    )
    .unwrap();
}

#[test]
fn inspect_count_without_list() {
    let path = create_deck("count");
    let tree = inspect_bare(&path);
    assert_eq!(tree["smartArtCount"], 0);
    assert_eq!(tree["slides"][0]["smartArtCount"], 0);
    assert!(tree["slides"][0].get("smartArts").is_none());
    assert_eq!(
        object_keys(&tree),
        [
            "schemaVersion",
            "format",
            "id",
            "slideCount",
            "pictureCount",
            "chartCount",
            "smartArtCount",
            "commentCount",
            "tableCount",
            "props",
            "slides"
        ]
    );
}

#[test]
fn smartart_scope_does_not_need_slide() {
    let src = create_deck("scope");
    let out = tmp("scope").with_extension("pptx");
    insert_list(&src, &out);
    let tree = inspect_smart(&out);
    assert!(tree["slides"][0].get("shapes").is_none());
    assert_eq!(tree["smartArtCount"], 1);
    assert_eq!(tree["slides"][0]["smartArts"].as_array().unwrap().len(), 1);
}

#[test]
fn insert_inspect_id_is_smartdiagram_3() {
    let src = create_deck("id3");
    let out = tmp("id3").with_extension("pptx");
    insert_list(&src, &out);
    let tree = inspect_smart(&out);
    let arts = tree["slides"][0]["smartArts"].as_array().unwrap();
    assert_eq!(arts.len(), 1);
    assert_eq!(arts[0]["id"], "slide:1/smartdiagram:3");
    assert_eq!(arts[0]["cNvPrId"], "3");
    assert_eq!(arts[0]["stable"], true);
    assert_eq!(arts[0]["layout"], UID_LIST);
    assert_eq!(
        object_keys(&arts[0]),
        [
            "id", "cNvPrId", "name", "x", "y", "cx", "cy", "layout", "colors", "style", "points",
            "stable"
        ]
    );
    let pts = arts[0]["points"].as_array().unwrap();
    assert_eq!(pts.len(), 3);
    assert_eq!(pts[0]["text"], "甲");
    assert_eq!(pts[1]["text"], "乙");
    assert_eq!(pts[2]["text"], "丙");
    assert_eq!(pts[0]["id"], "slide:1/smartdiagram:3/pt:1");
    assert!(arts[0].get("kind").is_none());
    assert!(arts[0].get("wrap").is_none());
    let names = part_names(&out);
    assert!(names.iter().any(|n| n == "ppt/diagrams/data1.xml"));
    assert!(names.iter().any(|n| n == "ppt/diagrams/layout1.xml"));
    assert!(!names.iter().any(|n| n.contains("diagrams/drawing")));
}

#[test]
fn set_text_add_delete_point_size_pos_layout() {
    let src = create_deck("ops");
    let inserted = tmp("ins").with_extension("pptx");
    insert_list(&src, &inserted);
    let id = "slide:1/smartdiagram:3";

    let texted = tmp("text").with_extension("pptx");
    apply(
        &inserted,
        &texted,
        json!([{ "op": "setSmartArtText", "id": format!("{id}/pt:2"), "text": "改乙" }]),
    )
    .unwrap();
    let tree = inspect_smart(&texted);
    assert_eq!(
        tree["slides"][0]["smartArts"][0]["points"][1]["text"],
        "改乙"
    );

    let added = tmp("add").with_extension("pptx");
    apply(
        &texted,
        &added,
        json!([{ "op": "addSmartArtPoint", "id": id, "text": "丁" }]),
    )
    .unwrap();
    let tree = inspect_smart(&added);
    let pts = tree["slides"][0]["smartArts"][0]["points"]
        .as_array()
        .unwrap();
    assert_eq!(pts.len(), 4);
    assert_eq!(pts[3]["text"], "丁");
    let last_id = pts[3]["id"].as_str().unwrap().to_string();

    let dropped = tmp("delpt").with_extension("pptx");
    apply(
        &added,
        &dropped,
        json!([{ "op": "deleteSmartArtPoint", "id": last_id }]),
    )
    .unwrap();
    let tree = inspect_smart(&dropped);
    assert_eq!(
        tree["slides"][0]["smartArts"][0]["points"]
            .as_array()
            .unwrap()
            .len(),
        3
    );

    let sized = tmp("size").with_extension("pptx");
    apply(
        &dropped,
        &sized,
        json!([
            { "op": "setSmartArtSize", "id": id, "cx": 2000000, "cy": 1000000 },
            { "op": "setSmartArtPos", "id": id, "x": 100000, "y": 200000 }
        ]),
    )
    .unwrap();
    let tree = inspect_smart(&sized);
    let art = &tree["slides"][0]["smartArts"][0];
    assert_eq!(art["cx"], 2000000);
    assert_eq!(art["cy"], 1000000);
    assert_eq!(art["x"], 100000);
    assert_eq!(art["y"], 200000);
    let slide = part_text(&sized, "ppt/slides/slide1.xml");
    assert!(slide.contains(r#"cx="2000000""#));
    assert!(slide.contains(r#"x="100000""#));

    let same = tmp("lay-same").with_extension("pptx");
    apply(
        &sized,
        &same,
        json!([{ "op": "setSmartArtLayout", "id": id, "layout": "list" }]),
    )
    .unwrap();
    let tree = inspect_smart(&same);
    assert_eq!(tree["slides"][0]["smartArts"][0]["layout"], UID_LIST);

    let org = tmp("lay-org").with_extension("pptx");
    apply(
        &same,
        &org,
        json!([{ "op": "setSmartArtLayout", "id": id, "layout": "org" }]),
    )
    .unwrap();
    let tree = inspect_smart(&org);
    assert_eq!(tree["slides"][0]["smartArts"][0]["layout"], UID_ORG);
    assert!(!tree["slides"][0]["smartArts"][0]["points"]
        .as_array()
        .unwrap()
        .is_empty());

    let cyc = tmp("lay-cyc").with_extension("pptx");
    apply(
        &org,
        &cyc,
        json!([{ "op": "setSmartArtLayout", "id": id, "layout": "cycle" }]),
    )
    .unwrap();
    let tree = inspect_smart(&cyc);
    assert_eq!(tree["slides"][0]["smartArts"][0]["layout"], UID_CYCLE);

    let gone = tmp("del").with_extension("pptx");
    apply(&cyc, &gone, json!([{ "op": "deleteSmartArt", "id": id }])).unwrap();
    let tree = inspect_smart(&gone);
    assert_eq!(tree["smartArtCount"], 0);
    assert_eq!(tree["slides"][0]["smartArts"], json!([]));
    let names = part_names(&gone);
    assert!(!names.iter().any(|n| n.contains("ppt/diagrams/")));
    let slide = part_text(&gone, "ppt/slides/slide1.xml");
    assert!(!slide.contains("graphicFrame"));
    assert!(!slide.contains("Fallback"));
}

#[test]
fn set_layout_rejects_unknown_family() {
    let src = create_deck("unk0");
    let with = tmp("unk-in").with_extension("pptx");
    insert_list(&src, &with);
    let layout = part_names(&with)
        .into_iter()
        .find(|n| n.contains("diagrams/layout") && n.ends_with(".xml"))
        .expect("应有 layout 部件");
    rewrite_part(&with, &layout, |data| {
        String::from_utf8(data)
            .unwrap()
            .replace(
                UID_LIST,
                "urn:microsoft.com/office/officeart/2005/8/layout/pyramid1",
            )
            .into_bytes()
    });
    let id = inspect_smart(&with)["slides"][0]["smartArts"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let err = apply(
        &with,
        &tmp("unk-out").with_extension("pptx"),
        json!([{ "op": "setSmartArtLayout", "id": id, "layout": "list" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::NotImplemented);
}

#[test]
fn insert_rejects_kind_wrap() {
    let src = create_deck("kind");
    let err = apply(
        &src,
        &tmp("kind").with_extension("pptx"),
        json!([{
            "op": "insertSmartArt",
            "id": "slide:1",
            "layout": "list",
            "kind": "anchor"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    let err = apply(
        &src,
        &tmp("wrap").with_extension("pptx"),
        json!([{
            "op": "insertSmartArt",
            "id": "slide:1",
            "layout": "list",
            "wrap": "square"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

#[test]
fn create_smartarts_and_empty_and_rejects() {
    let path = tmp("create").with_extension("pptx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "presentation": {
                "slides": [{
                    "shapes": [{ "text": "标题" }],
                    "smartArts": [{
                        "layout": "list",
                        "points": [
                            { "text": "甲" },
                            { "text": "乙" },
                            { "text": "丙" }
                        ]
                    }]
                }]
            }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let tree = inspect_smart(&path);
    assert_eq!(tree["smartArtCount"], 1);
    assert_eq!(tree["slides"][0]["smartArtCount"], 1);
    let art = &tree["slides"][0]["smartArts"][0];
    assert_eq!(art["id"], "slide:1/smartdiagram:3");
    assert_eq!(art["points"][0]["text"], "甲");
    assert!(part_names(&path)
        .iter()
        .any(|n| n == "ppt/diagrams/data1.xml"));

    let empty = tmp("empty").with_extension("pptx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": empty.to_string_lossy(),
            "presentation": {
                "slides": [{ "shapes": [{ "text": "a" }], "smartArts": [] }]
            }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let tree = inspect_smart(&empty);
    assert_eq!(tree["smartArtCount"], 0);
    assert_eq!(tree["slides"][0]["smartArts"], json!([]));
    assert!(!part_names(&empty)
        .iter()
        .any(|n| n.contains("diagrams/data")));

    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": tmp("kind").with_extension("pptx").to_string_lossy(),
            "presentation": {
                "slides": [{
                    "shapes": [{ "text": "a" }],
                    "smartArts": [{ "layout": "list", "kind": "anchor" }]
                }]
            }
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": tmp("wrap").with_extension("pptx").to_string_lossy(),
            "presentation": {
                "slides": [{
                    "shapes": [{ "text": "a" }],
                    "smartArts": [{ "layout": "list", "wrap": "square" }]
                }]
            }
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": tmp("from").with_extension("pptx").to_string_lossy(),
            "presentation": {
                "slides": [{
                    "shapes": [{ "text": "a" }],
                    "smartArts": [{ "layout": "list", "from": "A1" }]
                }]
            }
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": tmp("root").with_extension("pptx").to_string_lossy(),
            "presentation": {
                "smartArts": [{ "layout": "list" }],
                "slides": [{ "shapes": [{ "text": "a" }] }]
            }
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

#[test]
fn xlsx_accepts_smartart_scope() {
    let path = tmp("book").with_extension("xlsx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "workbook": { "sheets": [{ "name": "Sheet1", "cells": [] }] }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let RunOutput::Inspect(tree) = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": path.to_string_lossy(),
            "scope": { "smartArt": true }
        })
        .to_string(),
        None,
    )
    .unwrap() else {
        panic!("inspect 应返回树");
    };
    assert_eq!(tree["smartArtCount"], 0);
    assert_eq!(tree["sheets"][0]["smartArts"], json!([]));
}

#[test]
fn docx_xlsx_reject_set_smartart_pos() {
    let docx = tmp("d").with_extension("docx");
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
    let err = apply(
        &docx,
        &tmp("d-out").with_extension("docx"),
        json!([{ "op": "setSmartArtPos", "id": "para:1/smartdiagram:1", "x": 0 }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let xlsx = tmp("x").with_extension("xlsx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": xlsx.to_string_lossy(),
            "workbook": { "sheets": [{ "name": "Sheet1", "cells": [] }] }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let err = apply(
        &xlsx,
        &tmp("x-out").with_extension("xlsx"),
        json!([{ "op": "setSmartArtPos", "id": "sheet:Sheet1/smartdiagram:1", "x": 0 }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

#[test]
fn pptx_rejects_set_smartart_wrap() {
    let src = create_deck("wrap-op");
    let inserted = tmp("wrap-ins").with_extension("pptx");
    insert_list(&src, &inserted);
    let err = apply(
        &inserted,
        &tmp("wrap-out").with_extension("pptx"),
        json!([{
            "op": "setSmartArtWrap",
            "id": "slide:1/smartdiagram:3",
            "kind": "anchor"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}
