//! 动画清单与预设效果写入：inspect `animCount` / `scope.timing`，以及 `insertAnim` / `setAnim` / `deleteAnim` / `moveAnim`。

use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-pptx-timing");
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
    inspect_scope_opt(path, None)
}

fn inspect_scope(path: &Path, scope: Value) -> Value {
    inspect_scope_opt(path, Some(scope))
}

fn inspect_scope_opt(path: &Path, scope: Option<Value>) -> Value {
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

fn inject_before_end(xml: &str, snippet: &str) -> String {
    xml.replace("</p:sld>", &format!("{snippet}</p:sld>"))
}

fn fade_click(spid: &str) -> String {
    format!(
        concat!(
            r#"<p:timing><p:tnLst><p:par>"#,
            r#"<p:cTn id="1" dur="indefinite" nodeType="tmRoot"><p:childTnLst>"#,
            r#"<p:seq><p:cTn id="2" dur="indefinite" nodeType="mainSeq"><p:childTnLst>"#,
            r#"<p:par><p:cTn id="5" presetID="10" presetClass="entr" fill="hold" nodeType="clickEffect">"#,
            r#"<p:childTnLst>"#,
            r#"<p:set><p:cBhvr><p:cTn id="6" dur="1" fill="hold"/>"#,
            r#"<p:tgtEl><p:spTgt spid="{spid}"/></p:tgtEl></p:cBhvr>"#,
            r#"<p:to><p:strVal val="visible"/></p:to></p:set>"#,
            r#"<p:animEffect transition="in" filter="fade">"#,
            r#"<p:cBhvr><p:cTn id="7" dur="500"/>"#,
            r#"<p:tgtEl><p:spTgt spid="{spid}"/></p:tgtEl></p:cBhvr></p:animEffect>"#,
            r#"</p:childTnLst></p:cTn></p:par>"#,
            r#"</p:childTnLst></p:cTn></p:seq>"#,
            r#"</p:childTnLst></p:cTn></p:par></p:tnLst></p:timing>"#
        ),
        spid = spid
    )
}

#[test]
fn create_has_zero_anim_count_without_list() {
    let path = create_deck("bare");
    let tree = inspect(&path);
    assert_eq!(tree["animCount"], 0);
    assert_eq!(tree["slides"][0]["animCount"], 0);
    assert!(tree["slides"][0].get("timing").is_none());
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
    let xml = part(&path, "ppt/slides/slide1.xml");
    assert!(!xml.contains("<p:timing"));
}

#[test]
fn timing_scope_does_not_need_slide() {
    let path = create_deck("scope");
    rewrite_part(&path, "ppt/slides/slide1.xml", |data| {
        inject_before_end(&String::from_utf8(data).unwrap(), &fade_click("2")).into_bytes()
    });
    let tree = inspect_scope(&path, json!({ "timing": true }));
    assert!(tree["slides"][0].get("shapes").is_none());
    assert_eq!(tree["animCount"], 1);
    assert_eq!(tree["slides"][0]["animCount"], 1);
    let list = tree["slides"][0]["timing"].as_array().unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0]["target"], "slide:1/shape:2");
    assert_eq!(list[0]["type"], "fade");
    assert_eq!(list[0]["trigger"], "click");
    assert_eq!(list[0]["delay"], 0);
    assert_eq!(list[0]["dur"], 500);
    assert_eq!(list[0]["id"], "slide:1/anim:1");
    assert_eq!(list[0]["repeat"], 1);
    assert_eq!(list[0]["sound"], Value::Null);
    assert_eq!(list[0]["para"], Value::Null);
    assert_eq!(list[0]["clickOn"], Value::Null);
    assert_eq!(
        object_keys(&list[0]),
        ["id", "target", "type", "trigger", "delay", "dur", "repeat", "sound", "para", "clickOn"]
    );
}

#[test]
fn inspect_reads_click_with_after_and_strips_filter() {
    let path = create_deck("multi");
    rewrite_part(&path, "ppt/slides/slide1.xml", |data| {
        inject_before_end(
            &String::from_utf8(data).unwrap(),
            concat!(
                r#"<p:timing><p:tnLst><p:par><p:cTn id="1" nodeType="tmRoot"><p:childTnLst>"#,
                r#"<p:cTn id="2" nodeType="clickEffect" presetClass="entr">"#,
                r#"<p:childTnLst><p:animEffect filter="fade">"#,
                r#"<p:cBhvr><p:tgtEl><p:spTgt spid="2"/></p:tgtEl></p:cBhvr></p:animEffect></p:childTnLst></p:cTn>"#,
                r#"<p:cTn id="3" nodeType="withEffect">"#,
                r#"<p:childTnLst><p:animEffect filter="blinds(horizontal)">"#,
                r#"<p:cBhvr><p:tgtEl><p:spTgt spid="3"/></p:tgtEl></p:cBhvr></p:animEffect></p:childTnLst></p:cTn>"#,
                r#"<p:cTn id="4" nodeType="afterEffect">"#,
                r#"<p:childTnLst><p:animMotion>"#,
                r#"<p:cBhvr><p:tgtEl><p:spTgt spid="99"/></p:tgtEl></p:cBhvr></p:animMotion></p:childTnLst></p:cTn>"#,
                r#"</p:childTnLst></p:cTn></p:par></p:tnLst></p:timing>"#
            ),
        )
        .into_bytes()
    });
    let bare = inspect(&path);
    assert_eq!(bare["animCount"], 3);
    assert!(bare["slides"][0].get("timing").is_none());
    let tree = inspect_scope(&path, json!({ "timing": true }));
    let list = tree["slides"][0]["timing"].as_array().unwrap();
    assert_eq!(list.len(), 3);
    assert_eq!(list[0]["id"], "slide:1/anim:1");
    assert_eq!(list[0]["type"], "fade");
    assert_eq!(list[0]["trigger"], "click");
    assert_eq!(list[0]["target"], "slide:1/shape:2");
    assert_eq!(list[1]["type"], "blinds");
    assert_eq!(list[1]["trigger"], "with");
    assert_eq!(list[1]["target"], "slide:1/shape:3");
    assert_eq!(list[2]["type"], "motion");
    assert_eq!(list[2]["trigger"], "after");
    assert_eq!(list[2]["target"], Value::Null);
}

#[test]
fn nested_effect_is_its_own_item() {
    let path = create_deck("nested");
    rewrite_part(&path, "ppt/slides/slide1.xml", |data| {
        inject_before_end(
            &String::from_utf8(data).unwrap(),
            concat!(
                r#"<p:timing><p:tnLst><p:par><p:cTn id="1" nodeType="clickEffect">"#,
                r#"<p:childTnLst><p:animEffect filter="fade">"#,
                r#"<p:cBhvr><p:tgtEl><p:spTgt spid="2"/></p:tgtEl></p:cBhvr></p:animEffect>"#,
                r#"<p:cTn id="2" nodeType="afterEffect" presetClass="exit">"#,
                r#"<p:childTnLst><p:animRot>"#,
                r#"<p:cBhvr><p:tgtEl><p:spTgt spid="3"/></p:tgtEl></p:cBhvr></p:animRot></p:childTnLst></p:cTn>"#,
                r#"</p:childTnLst></p:cTn></p:par></p:tnLst></p:timing>"#
            ),
        )
        .into_bytes()
    });
    let tree = inspect_scope(&path, json!({ "timing": true }));
    let list = tree["slides"][0]["timing"].as_array().unwrap();
    assert_eq!(list.len(), 2);
    assert_eq!(list[0]["id"], "slide:1/anim:1");
    assert_eq!(list[0]["type"], "fade");
    assert_eq!(list[0]["target"], "slide:1/shape:2");
    assert_eq!(list[1]["type"], "spin");
    assert_eq!(list[1]["trigger"], "after");
    assert_eq!(list[1]["target"], "slide:1/shape:3");
}

#[test]
fn type_falls_back_to_set_preset_and_other() {
    let path = create_deck("types");
    rewrite_part(&path, "ppt/slides/slide1.xml", |data| {
        inject_before_end(
            &String::from_utf8(data).unwrap(),
            concat!(
                r#"<p:timing><p:tnLst><p:par>"#,
                r#"<p:cTn id="1" nodeType="clickEffect"><p:childTnLst>"#,
                r#"<p:set><p:cBhvr><p:tgtEl><p:spTgt spid="2"/></p:tgtEl></p:cBhvr></p:set>"#,
                r#"</p:childTnLst></p:cTn>"#,
                r#"<p:cTn id="2" nodeType="clickEffect" presetClass="emph"><p:childTnLst>"#,
                r#"<p:tgtEl><p:spTgt spid="2"/></p:tgtEl></p:childTnLst></p:cTn>"#,
                r#"<p:cTn id="3" nodeType="clickEffect"/>"#,
                r#"</p:par></p:tnLst></p:timing>"#
            ),
        )
        .into_bytes()
    });
    let tree = inspect_scope(&path, json!({ "timing": true }));
    let list = tree["slides"][0]["timing"].as_array().unwrap();
    assert_eq!(list[0]["type"], "set");
    assert_eq!(list[1]["type"], "emph");
    assert_eq!(list[2]["type"], "other");
    assert_eq!(list[2]["target"], Value::Null);
}

#[test]
fn pic_and_table_targets() {
    let path = create_deck("objs");
    rewrite_part(&path, "ppt/slides/slide1.xml", |data| {
        let xml = String::from_utf8(data).unwrap();
        let xml = xml.replace(
            "</p:spTree>",
            concat!(
                r#"<p:pic><p:nvPicPr><p:cNvPr id="9" name="图"/><p:cNvPicPr/><p:nvPr/></p:nvPicPr>"#,
                r#"<p:blipFill/><p:spPr/></p:pic>"#,
                r#"<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="8" name="表"/><p:cNvGraphicFramePr/><p:nvPr/>"#,
                r#"</p:nvGraphicFramePr><p:xfrm/><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/table">"#,
                r#"<a:tbl/></a:graphicData></a:graphic></p:graphicFrame></p:spTree>"#
            ),
        );
        inject_before_end(
            &xml,
            concat!(
                r#"<p:timing><p:tnLst><p:par>"#,
                r#"<p:cTn id="1" nodeType="clickEffect"><p:childTnLst><p:animEffect filter="fade">"#,
                r#"<p:cBhvr><p:tgtEl><p:spTgt spid="9"/></p:tgtEl></p:cBhvr></p:animEffect></p:childTnLst></p:cTn>"#,
                r#"<p:cTn id="2" nodeType="clickEffect"><p:childTnLst><p:animEffect filter="fade">"#,
                r#"<p:cBhvr><p:tgtEl><p:spTgt spid="8"/></p:tgtEl></p:cBhvr></p:animEffect></p:childTnLst></p:cTn>"#,
                r#"</p:par></p:tnLst></p:timing>"#
            ),
        )
        .into_bytes()
    });
    let tree = inspect_scope(&path, json!({ "timing": true }));
    let list = tree["slides"][0]["timing"].as_array().unwrap();
    assert_eq!(list[0]["target"], "slide:1/pic:9");
    assert_eq!(list[1]["target"], "slide:1/table:8");
}

#[test]
fn set_shape_text_keeps_timing_list() {
    let src = create_deck("keep");
    rewrite_part(&src, "ppt/slides/slide1.xml", |data| {
        inject_before_end(&String::from_utf8(data).unwrap(), &fade_click("2")).into_bytes()
    });
    let tree0 = inspect_scope(&src, json!({ "slide": 1 }));
    let id = tree0["slides"][0]["shapes"][0]["id"].as_str().unwrap();
    let out = tmp("keep-out");
    apply(
        &src,
        &out,
        json!([{ "op": "setShapeText", "id": id, "text": "改过的正文" }]),
    )
    .unwrap();
    let tree = inspect_scope(&out, json!({ "timing": true, "slide": 1 }));
    assert_eq!(tree["slides"][0]["shapes"][0]["text"], "改过的正文");
    assert_eq!(tree["slides"][0]["timing"][0]["target"], "slide:1/shape:2");
    assert_eq!(tree["slides"][0]["timing"][0]["type"], "fade");
    let xml = part(&out, "ppt/slides/slide1.xml");
    assert!(xml.contains("<p:timing>"));
    assert!(xml.contains("spid=\"2\""));
}

#[test]
fn set_timing_is_unsupported_and_create_rejects_timing_field() {
    let src = create_deck("no-write");
    let dest = tmp("no-write-out");
    let err = apply(&src, &dest, json!([{ "op": "setTiming", "id": "slide:1" }])).unwrap_err();
    assert_eq!(err.code(), ErrorCode::UnsupportedOp);
    assert!(!dest.exists());

    let bad = tmp("create-timing");
    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": bad.to_string_lossy(),
            "presentation": {
                "slides": [{ "shapes": [{ "text": "甲" }], "timing": [] }]
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
fn xlsx_and_docx_reject_scope_timing() {
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
    let err = inspect_err(&xlsx, json!({ "timing": true }));
    assert_eq!(err.code(), ErrorCode::InvalidField);

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
    let err = inspect_err(&docx, json!({ "timing": true }));
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

#[test]
fn insert_fade_writes_office_tree() {
    let src = create_deck("ins");
    let out = tmp("ins-out");
    apply(
        &src,
        &out,
        json!([{ "op": "insertAnim", "id": "slide:1/shape:2", "type": "fade" }]),
    )
    .unwrap();
    let tree = inspect_scope(&out, json!({ "timing": true, "slide": 1 }));
    let list = tree["slides"][0]["timing"].as_array().unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0]["id"], "slide:1/anim:1");
    assert_eq!(list[0]["target"], "slide:1/shape:2");
    assert_eq!(list[0]["type"], "fade");
    assert_eq!(list[0]["trigger"], "click");
    assert_eq!(list[0]["delay"], 0);
    assert_eq!(list[0]["dur"], 500);
    assert_eq!(
        object_keys(&list[0]),
        ["id", "target", "type", "trigger", "delay", "dur", "repeat", "sound", "para", "clickOn"]
    );
    let xml = part(&out, "ppt/slides/slide1.xml");
    assert!(xml.contains("<p:timing"));
    assert!(xml.contains("nodeType=\"mainSeq\""));
    assert!(xml.contains("filter=\"fade\""));
    assert!(xml.contains("<p:bldLst"));
    assert!(xml.contains("spid=\"2\""));
    assert!(xml.contains("delay=\"indefinite\""));
    assert!(!xml.contains("evt=\"onBegin\""));
    assert!(!xml.contains("<p:txEl"));
}

#[test]
fn insert_second_click_appends() {
    let src = create_deck("ins2");
    let mid = tmp("ins2-mid");
    apply(
        &src,
        &mid,
        json!([{ "op": "insertAnim", "id": "slide:1/shape:2", "type": "fade" }]),
    )
    .unwrap();
    let out = tmp("ins2-out");
    apply(
        &mid,
        &out,
        json!([{ "op": "insertAnim", "id": "slide:1/shape:3", "type": "fade" }]),
    )
    .unwrap();
    let tree = inspect_scope(&out, json!({ "timing": true }));
    let list = tree["slides"][0]["timing"].as_array().unwrap();
    assert_eq!(list.len(), 2);
    assert_eq!(list[0]["id"], "slide:1/anim:1");
    assert_eq!(list[0]["target"], "slide:1/shape:2");
    assert_eq!(list[1]["id"], "slide:1/anim:2");
    assert_eq!(list[1]["target"], "slide:1/shape:3");
    assert_eq!(list[1]["trigger"], "click");
}

#[test]
fn insert_with_attaches_to_last_group() {
    let src = create_deck("with");
    let mid = tmp("with-mid");
    apply(
        &src,
        &mid,
        json!([{ "op": "insertAnim", "id": "slide:1/shape:2", "type": "fade" }]),
    )
    .unwrap();
    let out = tmp("with-out");
    apply(
        &mid,
        &out,
        json!([{
            "op": "insertAnim",
            "id": "slide:1/shape:3",
            "type": "fade",
            "trigger": "with"
        }]),
    )
    .unwrap();
    let tree = inspect_scope(&out, json!({ "timing": true }));
    let list = tree["slides"][0]["timing"].as_array().unwrap();
    assert_eq!(list.len(), 2);
    assert_eq!(list[0]["trigger"], "click");
    assert_eq!(list[1]["trigger"], "with");
    assert_eq!(list[1]["target"], "slide:1/shape:3");
}

#[test]
fn insert_anim_writes_delay_and_dur() {
    let src = create_deck("stagger");
    let out = tmp("stagger-out");
    apply(
        &src,
        &out,
        json!([
            { "op": "insertAnim", "id": "slide:1/shape:2", "type": "fade" },
            {
                "op": "insertAnim",
                "id": "slide:1/shape:3",
                "type": "fade",
                "trigger": "with",
                "delay": 380,
                "dur": 800
            }
        ]),
    )
    .unwrap();
    let tree = inspect_scope(&out, json!({ "timing": true }));
    let list = tree["slides"][0]["timing"].as_array().unwrap();
    assert_eq!(list.len(), 2);
    assert_eq!(list[0]["delay"], 0);
    assert_eq!(list[0]["dur"], 500);
    assert_eq!(list[1]["trigger"], "with");
    assert_eq!(list[1]["delay"], 380);
    assert_eq!(list[1]["dur"], 800);
    let xml = part(&out, "ppt/slides/slide1.xml");
    assert!(xml.contains("delay=\"380\""));
    assert!(xml.contains("dur=\"800\""));
    assert_eq!(xml.matches("delay=\"indefinite\"").count(), 1);
}

#[test]
fn delete_anim_strips_and_clears_timing() {
    let src = create_deck("del");
    let two = tmp("del-two");
    apply(
        &src,
        &two,
        json!([
            { "op": "insertAnim", "id": "slide:1/shape:2", "type": "fade" },
            { "op": "insertAnim", "id": "slide:1/shape:3", "type": "fade" }
        ]),
    )
    .unwrap();
    let one = tmp("del-one");
    apply(
        &two,
        &one,
        json!([{ "op": "deleteAnim", "id": "slide:1/anim:1" }]),
    )
    .unwrap();
    let tree = inspect_scope(&one, json!({ "timing": true }));
    let list = tree["slides"][0]["timing"].as_array().unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0]["id"], "slide:1/anim:1");
    assert_eq!(list[0]["target"], "slide:1/shape:3");
    let xml = part(&one, "ppt/slides/slide1.xml");
    assert!(xml.contains("<p:timing"));
    assert!(xml.contains("spid=\"3\""));
    assert!(!xml.contains("spid=\"2\""));
    assert_eq!(xml.matches("nodeType=\"clickEffect\"").count(), 1);
    assert_eq!(xml.matches("delay=\"indefinite\"").count(), 1);
    assert!(xml.contains("dur=\"1\""));
    assert!(xml.contains("dur=\"500\""));

    let empty = tmp("del-empty");
    apply(
        &one,
        &empty,
        json!([{ "op": "deleteAnim", "id": "slide:1/anim:1" }]),
    )
    .unwrap();
    let tree = inspect_scope(&empty, json!({ "timing": true }));
    assert_eq!(tree["slides"][0]["animCount"], 0);
    assert_eq!(tree["slides"][0]["timing"].as_array().unwrap().len(), 0);
    let xml = part(&empty, "ppt/slides/slide1.xml");
    assert!(!xml.contains("<p:timing"));
}

#[test]
fn insert_on_injected_tree_then_delete() {
    let src = create_deck("inj");
    rewrite_part(&src, "ppt/slides/slide1.xml", |data| {
        inject_before_end(&String::from_utf8(data).unwrap(), &fade_click("2")).into_bytes()
    });
    let out = tmp("inj-out");
    apply(
        &src,
        &out,
        json!([{ "op": "insertAnim", "id": "slide:1/shape:3", "type": "fade" }]),
    )
    .unwrap();
    let tree = inspect_scope(&out, json!({ "timing": true }));
    let list = tree["slides"][0]["timing"].as_array().unwrap();
    assert_eq!(list.len(), 2);
    assert_eq!(list[0]["target"], "slide:1/shape:2");
    assert_eq!(list[1]["target"], "slide:1/shape:3");
}

#[test]
fn insert_anim_rejects_bad_fields() {
    let src = create_deck("bad");
    let dest = tmp("bad-out");
    let err = apply(
        &src,
        &dest,
        json!([{ "op": "insertAnim", "id": "slide:1/shape:2", "type": "blinds" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!dest.exists());

    let err = apply(
        &src,
        &dest,
        json!([{ "op": "insertAnim", "id": "slide:1", "type": "fade" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    assert!(!dest.exists());

    let err = apply(
        &src,
        &dest,
        json!([{ "op": "insertAnim", "id": "slide:1/notes/shape:2", "type": "fade" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::IdNotFound);
    assert!(!dest.exists());

    let err = apply(
        &src,
        &dest,
        json!([{ "op": "insertAnim", "id": "slide:1/shape:99", "type": "fade" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::IdNotFound);
    assert!(!dest.exists());

    let err = apply(
        &src,
        &dest,
        json!([{ "op": "insertAnim", "id": "slide:1/shape:2", "type": "fade", "delay": 60001 }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!dest.exists());

    let err = apply(
        &src,
        &dest,
        json!([{ "op": "insertAnim", "id": "slide:1/shape:2", "type": "fade", "dur": 0 }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!dest.exists());

    let err = apply(
        &src,
        &dest,
        json!([{ "op": "deleteAnim", "id": "slide:1/anim:1" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::IdNotFound);
    assert!(!dest.exists());
}

#[test]
fn xlsx_and_docx_reject_insert_anim() {
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
    let dest = tmp("d-out").with_extension("docx");
    let err = apply(
        &docx,
        &dest,
        json!([{ "op": "insertAnim", "id": "slide:1/shape:2", "type": "fade" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!dest.exists());

    let xlsx = tmp("x").with_extension("xlsx");
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
    let dest = tmp("x-out").with_extension("xlsx");
    let err = apply(
        &xlsx,
        &dest,
        json!([{ "op": "insertAnim", "id": "slide:1/shape:2", "type": "fade" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!dest.exists());
}

#[test]
fn first_with_writes_onbegin_keeps_indefinite() {
    let src = create_deck("auto");
    let out = tmp("auto-out");
    apply(
        &src,
        &out,
        json!([{
            "op": "insertAnim",
            "id": "slide:1/shape:2",
            "type": "fade",
            "trigger": "with"
        }]),
    )
    .unwrap();
    let tree = inspect_scope(&out, json!({ "timing": true }));
    let list = tree["slides"][0]["timing"].as_array().unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0]["trigger"], "with");
    let xml = part(&out, "ppt/slides/slide1.xml");
    assert!(xml.contains("evt=\"onBegin\""));
    assert!(xml.contains("delay=\"indefinite\""));
    assert!(xml.contains("nodeType=\"withEffect\""));
    assert!(xml.contains("<p:tn val=\"2\""));
}

#[test]
fn set_anim_updates_delay_dur_trigger() {
    let src = create_deck("set");
    let two = tmp("set-two");
    apply(
        &src,
        &two,
        json!([
            { "op": "insertAnim", "id": "slide:1/shape:2", "type": "fade" },
            { "op": "insertAnim", "id": "slide:1/shape:3", "type": "fade" }
        ]),
    )
    .unwrap();
    let out = tmp("set-out");
    apply(
        &two,
        &out,
        json!([{
            "op": "setAnim",
            "id": "slide:1/anim:2",
            "trigger": "after",
            "delay": 200,
            "dur": 700
        }]),
    )
    .unwrap();
    let tree = inspect_scope(&out, json!({ "timing": true }));
    let list = tree["slides"][0]["timing"].as_array().unwrap();
    assert_eq!(list.len(), 2);
    assert_eq!(list[0]["trigger"], "click");
    assert_eq!(list[0]["delay"], 0);
    assert_eq!(list[0]["dur"], 500);
    assert_eq!(list[1]["trigger"], "after");
    assert_eq!(list[1]["delay"], 200);
    assert_eq!(list[1]["dur"], 700);
    let xml = part(&out, "ppt/slides/slide1.xml");
    assert!(xml.contains("delay=\"200\""));
    assert!(xml.contains("dur=\"700\""));
    assert!(xml.contains("nodeType=\"afterEffect\""));
}

#[test]
fn set_anim_first_with_adds_onbegin_click_removes() {
    let src = create_deck("set-first");
    let click = tmp("set-first-click");
    apply(
        &src,
        &click,
        json!([{ "op": "insertAnim", "id": "slide:1/shape:2", "type": "fade" }]),
    )
    .unwrap();
    let with = tmp("set-first-with");
    apply(
        &click,
        &with,
        json!([{ "op": "setAnim", "id": "slide:1/anim:1", "trigger": "with" }]),
    )
    .unwrap();
    let tree = inspect_scope(&with, json!({ "timing": true }));
    assert_eq!(tree["slides"][0]["timing"][0]["trigger"], "with");
    let xml = part(&with, "ppt/slides/slide1.xml");
    assert!(xml.contains("evt=\"onBegin\""));
    assert!(xml.contains("delay=\"indefinite\""));

    let back = tmp("set-first-back");
    apply(
        &with,
        &back,
        json!([{ "op": "setAnim", "id": "slide:1/anim:1", "trigger": "click" }]),
    )
    .unwrap();
    let tree = inspect_scope(&back, json!({ "timing": true }));
    assert_eq!(tree["slides"][0]["timing"][0]["trigger"], "click");
    let xml = part(&back, "ppt/slides/slide1.xml");
    assert!(!xml.contains("evt=\"onBegin\""));
    assert!(xml.contains("delay=\"indefinite\""));
}

#[test]
fn set_anim_splits_and_merges_click_groups() {
    let src = create_deck("regroup");
    let two = tmp("regroup-two");
    apply(
        &src,
        &two,
        json!([
            { "op": "insertAnim", "id": "slide:1/shape:2", "type": "fade" },
            { "op": "insertAnim", "id": "slide:1/shape:3", "type": "fade" }
        ]),
    )
    .unwrap();
    let merged = tmp("regroup-merged");
    apply(
        &two,
        &merged,
        json!([{ "op": "setAnim", "id": "slide:1/anim:2", "trigger": "with" }]),
    )
    .unwrap();
    let tree = inspect_scope(&merged, json!({ "timing": true }));
    assert_eq!(tree["slides"][0]["timing"][0]["trigger"], "click");
    assert_eq!(tree["slides"][0]["timing"][1]["trigger"], "with");
    let xml = part(&merged, "ppt/slides/slide1.xml");
    assert_eq!(xml.matches("delay=\"indefinite\"").count(), 1);

    let split = tmp("regroup-split");
    apply(
        &merged,
        &split,
        json!([{ "op": "setAnim", "id": "slide:1/anim:2", "trigger": "click" }]),
    )
    .unwrap();
    let tree = inspect_scope(&split, json!({ "timing": true }));
    assert_eq!(tree["slides"][0]["timing"][1]["trigger"], "click");
    let xml = part(&split, "ppt/slides/slide1.xml");
    assert_eq!(xml.matches("delay=\"indefinite\"").count(), 2);
}

#[test]
fn set_anim_rejects_empty_and_need_effect_dur() {
    let src = create_deck("set-bad");
    let dest = tmp("set-bad-out");
    let err = apply(
        &src,
        &dest,
        json!([{ "op": "setAnim", "id": "slide:1/anim:1", "delay": 10 }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::IdNotFound);
    assert!(!dest.exists());

    let with = tmp("set-empty-src");
    apply(
        &src,
        &with,
        json!([{ "op": "insertAnim", "id": "slide:1/shape:2", "type": "fade" }]),
    )
    .unwrap();
    let err = apply(
        &with,
        &dest,
        json!([{ "op": "setAnim", "id": "slide:1/anim:1" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!dest.exists());

    rewrite_part(&src, "ppt/slides/slide1.xml", |data| {
        inject_before_end(
            &String::from_utf8(data).unwrap(),
            concat!(
                r#"<p:timing><p:tnLst><p:par>"#,
                r#"<p:cTn id="1" nodeType="clickEffect"><p:childTnLst>"#,
                r#"<p:set><p:cBhvr><p:tgtEl><p:spTgt spid="2"/></p:tgtEl></p:cBhvr></p:set>"#,
                r#"</p:childTnLst></p:cTn></p:par></p:tnLst></p:timing>"#
            ),
        )
        .into_bytes()
    });
    let err = apply(
        &src,
        &dest,
        json!([{ "op": "setAnim", "id": "slide:1/anim:1", "dur": 400 }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!dest.exists());
}

#[test]
fn notes_anim_writes_and_stays_off_slide_count() {
    let src = create_deck("notes-anim");
    let noted = tmp("notes-src");
    apply(
        &src,
        &noted,
        json!([{ "op": "setNotesText", "id": "slide:1", "text": "演讲者备注" }]),
    )
    .unwrap();
    let out = tmp("notes-anim-out");
    apply(
        &noted,
        &out,
        json!([{
            "op": "insertAnim",
            "id": "slide:1/notes/shape:3",
            "type": "fade",
            "delay": 120,
            "dur": 400
        }]),
    )
    .unwrap();
    let tree = inspect_scope(&out, json!({ "notes": true, "timing": true }));
    assert_eq!(tree["animCount"], 0);
    assert_eq!(tree["slides"][0]["animCount"], 0);
    let notes = &tree["slides"][0]["notes"];
    assert_eq!(notes["animCount"], 1);
    assert_eq!(notes["timing"][0]["id"], "slide:1/notes/anim:1");
    assert_eq!(notes["timing"][0]["target"], "slide:1/notes/shape:3");
    assert_eq!(notes["timing"][0]["trigger"], "click");
    assert_eq!(notes["timing"][0]["delay"], 120);
    assert_eq!(notes["timing"][0]["dur"], 400);
    assert!(tree["slides"][0].get("timing").is_none() || tree["slides"][0]["timing"] == json!([]));
    let xml = part(&out, "ppt/notesSlides/notesSlide1.xml");
    assert!(xml.contains("<p:timing"));
    assert!(xml.contains("delay=\"120\""));
    assert!(xml.contains("dur=\"400\""));
    let slide_xml = part(&out, "ppt/slides/slide1.xml");
    assert!(!slide_xml.contains("<p:timing"));
}

#[test]
fn master_placeholder_anim_writes_on_master_xml() {
    let src = create_deck("master-anim");
    let with_ph = tmp("master-ph");
    apply(
        &src,
        &with_ph,
        json!([{
            "op": "insertPlaceholder",
            "id": "master:1",
            "type": "title",
            "text": "母版标题"
        }]),
    )
    .unwrap();
    let tree = inspect_scope(&with_ph, json!({ "masters": true }));
    let ph = tree["masters"][0]["placeholders"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(ph.starts_with("master:1/shape:"));
    let out = tmp("master-anim-out");
    apply(
        &with_ph,
        &out,
        json!([{ "op": "insertAnim", "id": ph, "type": "fade" }]),
    )
    .unwrap();
    let tree = inspect_scope(&out, json!({ "masters": true, "timing": true }));
    assert_eq!(tree["animCount"], 0);
    assert_eq!(tree["masters"][0]["animCount"], 1);
    assert_eq!(tree["masters"][0]["timing"][0]["id"], "master:1/anim:1");
    assert_eq!(tree["masters"][0]["timing"][0]["target"], ph);
    assert_eq!(tree["masters"][0]["layouts"][0]["animCount"], 0);
    let xml = part(&out, "ppt/slideMasters/slideMaster1.xml");
    assert!(xml.contains("<p:timing"));
    assert!(xml.contains("</p:sldLayoutIdLst><p:timing"));
    assert!(xml.contains("</p:timing><p:txStyles"));
    let layout = part(&out, "ppt/slideLayouts/slideLayout1.xml");
    assert!(!layout.contains("<p:timing"));
    let slide = part(&out, "ppt/slides/slide1.xml");
    assert!(!slide.contains("<p:timing"));
}

#[test]
fn xlsx_and_docx_reject_set_anim() {
    let docx = tmp("d-set").with_extension("docx");
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
    let dest = tmp("d-set-out").with_extension("docx");
    let err = apply(
        &docx,
        &dest,
        json!([{ "op": "setAnim", "id": "slide:1/anim:1", "delay": 10 }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!dest.exists());

    let xlsx = tmp("x-set").with_extension("xlsx");
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
    let dest = tmp("x-set-out").with_extension("xlsx");
    let err = apply(
        &xlsx,
        &dest,
        json!([{ "op": "setAnim", "id": "slide:1/anim:1", "delay": 10 }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!dest.exists());
}

fn tiny_wav() -> Vec<u8> {
    let data_len = 8u32;
    let mut v = Vec::new();
    v.extend_from_slice(b"RIFF");
    v.extend_from_slice(&(36u32 + data_len).to_le_bytes());
    v.extend_from_slice(b"WAVE");
    v.extend_from_slice(b"fmt ");
    v.extend_from_slice(&16u32.to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes());
    v.extend_from_slice(&8000u32.to_le_bytes());
    v.extend_from_slice(&16000u32.to_le_bytes());
    v.extend_from_slice(&2u16.to_le_bytes());
    v.extend_from_slice(&16u16.to_le_bytes());
    v.extend_from_slice(b"data");
    v.extend_from_slice(&data_len.to_le_bytes());
    v.extend_from_slice(&[0u8; 8]);
    v
}

fn write_wav(tag: &str) -> PathBuf {
    let path = tmp_dir().join(format!("{tag}.wav"));
    std::fs::write(&path, tiny_wav()).unwrap();
    path
}

fn add_second_para(xml: &str) -> String {
    xml.replacen(
        "</a:p></p:txBody>",
        "</a:p><a:p><a:r><a:t>第二段</a:t></a:r></a:p></p:txBody>",
        1,
    )
}

#[test]
fn insert_whitelist_presets_roundtrip() {
    let names = [
        "appear",
        "flyIn",
        "wipe",
        "split",
        "zoom",
        "spin",
        "growShrink",
        "fadeOut",
        "flyOut",
        "wipeOut",
        "pathRight",
    ];
    for name in names {
        let src = create_deck(&format!("p-{name}"));
        let out = tmp(&format!("p-{name}-out"));
        apply(
            &src,
            &out,
            json!([{ "op": "insertAnim", "id": "slide:1/shape:2", "type": name }]),
        )
        .unwrap();
        let tree = inspect_scope(&out, json!({ "timing": true }));
        let list = tree["slides"][0]["timing"].as_array().unwrap();
        assert_eq!(list.len(), 1, "{name}");
        assert_eq!(list[0]["type"], name, "{name}");
        assert_eq!(list[0]["target"], "slide:1/shape:2");
    }
}

#[test]
fn set_anim_changes_type_to_spin() {
    let src = create_deck("set-type");
    let mid = tmp("set-type-mid");
    apply(
        &src,
        &mid,
        json!([{ "op": "insertAnim", "id": "slide:1/shape:2", "type": "fade" }]),
    )
    .unwrap();
    let out = tmp("set-type-out");
    apply(
        &mid,
        &out,
        json!([{ "op": "setAnim", "id": "slide:1/anim:1", "type": "spin" }]),
    )
    .unwrap();
    let tree = inspect_scope(&out, json!({ "timing": true }));
    assert_eq!(tree["slides"][0]["timing"][0]["type"], "spin");
    let xml = part(&out, "ppt/slides/slide1.xml");
    assert!(xml.contains("presetClass=\"emph\""));
    assert!(xml.contains("<p:animRot"));
}

#[test]
fn insert_and_set_repeat() {
    let src = create_deck("rep");
    let out = tmp("rep-out");
    apply(
        &src,
        &out,
        json!([{ "op": "insertAnim", "id": "slide:1/shape:2", "type": "fade", "repeat": 3 }]),
    )
    .unwrap();
    let tree = inspect_scope(&out, json!({ "timing": true }));
    assert_eq!(tree["slides"][0]["timing"][0]["repeat"], 3);
    let xml = part(&out, "ppt/slides/slide1.xml");
    assert!(xml.contains("repeatCount=\"3000\""));

    let inf = tmp("rep-inf");
    apply(
        &out,
        &inf,
        json!([{ "op": "setAnim", "id": "slide:1/anim:1", "repeat": null }]),
    )
    .unwrap();
    let tree = inspect_scope(&inf, json!({ "timing": true }));
    assert_eq!(tree["slides"][0]["timing"][0]["repeat"], Value::Null);
    let xml = part(&inf, "ppt/slides/slide1.xml");
    assert!(xml.contains("repeatCount=\"indefinite\""));
}

#[test]
fn insert_and_clear_sound() {
    let wav = write_wav("anim-beep");
    let src = create_deck("snd");
    let out = tmp("snd-out");
    apply(
        &src,
        &out,
        json!([{
            "op": "insertAnim",
            "id": "slide:1/shape:2",
            "type": "fade",
            "sound": wav.to_string_lossy()
        }]),
    )
    .unwrap();
    let tree = inspect_scope(&out, json!({ "timing": true }));
    let sound = tree["slides"][0]["timing"][0]["sound"].as_str().unwrap();
    assert!(sound.ends_with(".wav"), "{sound}");

    let cleared = tmp("snd-clear");
    apply(
        &out,
        &cleared,
        json!([{ "op": "setAnim", "id": "slide:1/anim:1", "sound": null }]),
    )
    .unwrap();
    let tree = inspect_scope(&cleared, json!({ "timing": true }));
    assert_eq!(tree["slides"][0]["timing"][0]["sound"], Value::Null);
    let xml = part(&cleared, "ppt/slides/slide1.xml");
    assert!(!xml.contains("<p:sndAc"));
}

#[test]
fn insert_para_writes_txel() {
    let src = create_deck("para");
    rewrite_part(&src, "ppt/slides/slide1.xml", |data| {
        add_second_para(&String::from_utf8(data).unwrap()).into_bytes()
    });
    let out = tmp("para-out");
    apply(
        &src,
        &out,
        json!([{ "op": "insertAnim", "id": "slide:1/shape:2", "type": "fade", "para": 2 }]),
    )
    .unwrap();
    let tree = inspect_scope(&out, json!({ "timing": true }));
    assert_eq!(tree["slides"][0]["timing"][0]["para"], 2);
    let xml = part(&out, "ppt/slides/slide1.xml");
    assert!(xml.contains("st=\"1\""));
}

#[test]
fn insert_click_on_uses_interactive_seq() {
    let src = create_deck("ia");
    let out = tmp("ia-out");
    apply(
        &src,
        &out,
        json!([{
            "op": "insertAnim",
            "id": "slide:1/shape:2",
            "type": "fade",
            "trigger": "clickOn",
            "clickOn": "slide:1/shape:3"
        }]),
    )
    .unwrap();
    let tree = inspect_scope(&out, json!({ "timing": true }));
    let list = tree["slides"][0]["timing"].as_array().unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0]["trigger"], "clickOn");
    assert_eq!(list[0]["clickOn"], "slide:1/shape:3");
    let xml = part(&out, "ppt/slides/slide1.xml");
    assert!(xml.contains("nodeType=\"interactiveSeq\""));
    assert!(xml.contains("evt=\"onClick\""));
    assert!(!xml.contains("nodeType=\"mainSeq\""));
}

#[test]
fn move_anim_swaps_two_clicks() {
    let src = create_deck("mv");
    let two = tmp("mv-two");
    apply(
        &src,
        &two,
        json!([
            { "op": "insertAnim", "id": "slide:1/shape:2", "type": "fade" },
            { "op": "insertAnim", "id": "slide:1/shape:3", "type": "fade" }
        ]),
    )
    .unwrap();
    let out = tmp("mv-out");
    apply(
        &two,
        &out,
        json!([{ "op": "moveAnim", "id": "slide:1/anim:2", "index": 1 }]),
    )
    .unwrap();
    let tree = inspect_scope(&out, json!({ "timing": true }));
    let list = tree["slides"][0]["timing"].as_array().unwrap();
    assert_eq!(list.len(), 2);
    assert_eq!(list[0]["target"], "slide:1/shape:3");
    assert_eq!(list[1]["target"], "slide:1/shape:2");
}

#[test]
fn xlsx_and_docx_reject_move_anim() {
    let src = create_deck("mv-ok");
    let dest = tmp("mv-bad");
    let err = apply(
        &src,
        &dest,
        json!([{ "op": "moveAnim", "id": "slide:1/anim:1", "index": 0 }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!dest.exists());

    let docx = tmp("d-mv").with_extension("docx");
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
    let dest = tmp("d-mv-out").with_extension("docx");
    let err = apply(
        &docx,
        &dest,
        json!([{ "op": "moveAnim", "id": "slide:1/anim:1", "index": 1 }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!dest.exists());
}
