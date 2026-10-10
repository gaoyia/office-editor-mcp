//! 工作表 DrawingML 链、图表工作表 Chart Drawing 叠加对象。

use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};

const PNG_1X1: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
    0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08, 0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00,
    0x00, 0x03, 0x01, 0x01, 0x00, 0x18, 0xDD, 0x8D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E,
    0x44, 0xAE, 0x42, 0x60, 0x82,
];

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-xlsx-drawing-hlink");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn tmp(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}.xlsx"))
}

fn write_png(tag: &str) -> PathBuf {
    let path = tmp_dir().join(format!("{tag}-{}.png", SEQ.fetch_add(1, Ordering::Relaxed)));
    std::fs::write(&path, PNG_1X1).unwrap();
    path
}

fn create_book(tag: &str) -> PathBuf {
    let path = tmp(tag);
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "workbook": {
                "sheets": [{
                    "name": "Sheet1",
                    "cells": [{ "ref": "A1", "value": 1 }]
                }]
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

fn object_keys(v: &Value) -> Vec<&str> {
    v.as_object().unwrap().keys().map(String::as_str).collect()
}

fn part_text(path: &Path, name: &str) -> String {
    let f = std::fs::File::open(path).unwrap();
    let mut zip = zip::ZipArchive::new(f).unwrap();
    let mut zf = zip.by_name(name).unwrap();
    let mut out = String::new();
    zf.read_to_string(&mut out).unwrap();
    out
}

fn part_names(path: &Path) -> Vec<String> {
    let f = std::fs::File::open(path).unwrap();
    let zip = zip::ZipArchive::new(f).unwrap();
    zip.file_names().map(str::to_string).collect()
}

fn chartsheet_op() -> Value {
    json!({
        "op": "insertChartSheet",
        "id": "xlsx:workbook",
        "name": "Chart1",
        "dataFrom": "A10",
        "type": "col",
        "title": "销量",
        "series": [{
            "name": "Q1",
            "categories": ["甲", "乙", "丙"],
            "values": [1, 2, 3]
        }]
    })
}

fn rel_box() -> Value {
    json!({
        "fromX": 0.25,
        "fromY": 0.25,
        "toX": 0.75,
        "toY": 0.75
    })
}

#[test]
fn picture_drawing_hyperlink_insert_set_delete() {
    let src = create_book("pic-hlink");
    let png = write_png("a");
    let with_pic = tmp("pic");
    apply(
        &src,
        &with_pic,
        json!([{
            "op": "insertPicture",
            "id": "sheet:Sheet1",
            "file": png.to_string_lossy(),
            "from": "B2",
            "cx": 914400,
            "cy": 914400
        }]),
    )
    .unwrap();
    let tree0 = inspect(&with_pic, json!({ "pictures": true, "hyperlinks": true }));
    assert_eq!(tree0["hyperlinkCount"], 0);
    assert_eq!(tree0["sheets"][0]["hyperlinkCount"], 0);
    assert_eq!(tree0["sheets"][0]["hyperlinks"], json!([]));
    let pic_id = tree0["sheets"][0]["pictures"][0]["id"].as_str().unwrap();
    assert!(tree0["sheets"][0]["pictures"][0]["hyperlink"].is_null());

    let with_link = tmp("link");
    apply(
        &with_pic,
        &with_link,
        json!([{
            "op": "insertHyperlink",
            "id": pic_id,
            "url": "https://example.com/pic",
            "tooltip": "图链"
        }]),
    )
    .unwrap();
    let tree = inspect(&with_link, json!({ "pictures": true, "hyperlinks": true }));
    assert_eq!(tree["hyperlinkCount"], 0);
    assert_eq!(tree["sheets"][0]["hyperlinks"], json!([]));
    let hl = &tree["sheets"][0]["pictures"][0]["hyperlink"];
    assert_eq!(object_keys(hl), ["kind", "url", "location", "tooltip"]);
    assert_eq!(hl["kind"], "url");
    assert_eq!(hl["url"], "https://example.com/pic");
    assert!(hl["location"].is_null());
    assert_eq!(hl["tooltip"], "图链");
    let drawing = part_text(&with_link, "xl/drawings/drawing1.xml");
    assert!(drawing.contains("hlinkClick"), "{drawing}");
    let rels = part_text(&with_link, "xl/drawings/_rels/drawing1.xml.rels");
    assert!(rels.contains("/hyperlink"), "{rels}");
    assert!(rels.contains("TargetMode=\"External\""), "{rels}");
    assert!(rels.contains("https://example.com/pic"), "{rels}");
    let sheet = part_text(&with_link, "xl/worksheets/sheet1.xml");
    assert!(!sheet.contains("<hyperlinks"), "{sheet}");

    let set_out = tmp("set");
    apply(
        &with_link,
        &set_out,
        json!([{
            "op": "setHyperlink",
            "id": pic_id,
            "url": "https://example.org/pic2"
        }]),
    )
    .unwrap();
    let tree_set = inspect(&set_out, json!({ "pictures": true }));
    assert_eq!(
        tree_set["sheets"][0]["pictures"][0]["hyperlink"]["url"],
        "https://example.org/pic2"
    );

    let del = tmp("del");
    apply(
        &set_out,
        &del,
        json!([{ "op": "deleteHyperlink", "id": pic_id }]),
    )
    .unwrap();
    let tree_del = inspect(&del, json!({ "pictures": true }));
    assert_eq!(tree_del["pictureCount"], 1);
    assert!(tree_del["sheets"][0]["pictures"][0]["hyperlink"].is_null());
    let drawing_del = part_text(&del, "xl/drawings/drawing1.xml");
    assert!(!drawing_del.contains("hlinkClick"), "{drawing_del}");
    let names = part_names(&del);
    if names
        .iter()
        .any(|n| n == "xl/drawings/_rels/drawing1.xml.rels")
    {
        let rels_del = part_text(&del, "xl/drawings/_rels/drawing1.xml.rels");
        assert!(!rels_del.contains("/hyperlink"), "{rels_del}");
    }
}

#[test]
fn picture_drawing_hyperlink_location() {
    let src = create_book("pic-loc");
    let png = write_png("loc");
    let with_pic = tmp("pic-loc");
    apply(
        &src,
        &with_pic,
        json!([{
            "op": "insertPicture",
            "id": "sheet:Sheet1",
            "file": png.to_string_lossy(),
            "from": "B2"
        }]),
    )
    .unwrap();
    let pic_id = inspect(&with_pic, json!({ "pictures": true }))["sheets"][0]["pictures"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let out = tmp("loc-out");
    apply(
        &with_pic,
        &out,
        json!([{
            "op": "insertHyperlink",
            "id": pic_id,
            "location": "Sheet1!A1"
        }]),
    )
    .unwrap();
    let tree = inspect(&out, json!({ "pictures": true }));
    let hl = &tree["sheets"][0]["pictures"][0]["hyperlink"];
    assert_eq!(hl["kind"], "location");
    assert!(hl["url"].is_null());
    assert_eq!(hl["location"], "Sheet1!A1");
    let rels = part_text(&out, "xl/drawings/_rels/drawing1.xml.rels");
    assert!(rels.contains("Target=\"#Sheet1!A1\""), "{rels}");
    assert!(!rels.contains("TargetMode"), "{rels}");
}

#[test]
fn chartsheet_insert_shape_relsize() {
    let src = create_book("cs-sp");
    let with_cs = tmp("cs");
    apply(&src, &with_cs, json!([chartsheet_op()])).unwrap();
    let out = tmp("cs-sp-out");
    let mut op = json!({
        "op": "insertShape",
        "id": "sheet:Chart1",
        "prst": "rect"
    });
    op.as_object_mut()
        .unwrap()
        .extend(rel_box().as_object().unwrap().clone());
    apply(&with_cs, &out, json!([op])).unwrap();

    let tree = inspect(&out, json!({ "shapes": true, "pictures": true }));
    let cs = tree["sheets"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["name"] == "Chart1")
        .unwrap();
    assert_eq!(cs["kind"], "chartsheet");
    assert_eq!(
        object_keys(cs).into_iter().take(13).collect::<Vec<_>>(),
        [
            "id",
            "name",
            "kind",
            "hidden",
            "pictureCount",
            "chartCount",
            "textBoxCount",
            "mathCount",
            "shapeCount",
            "connectorCount",
            "hasHeaderFooter",
            "print",
            "view"
        ]
    );
    assert_eq!(cs["shapeCount"], 1);
    assert_eq!(tree["shapeCount"], 1);
    let sp = &cs["shapes"][0];
    assert_eq!(sp["anchor"], "relSize");
    assert_eq!(sp["fromX"], json!(0.25));
    assert_eq!(sp["fromY"], json!(0.25));
    assert_eq!(sp["toX"], json!(0.75));
    assert_eq!(sp["toY"], json!(0.75));
    assert!(sp.get("from").is_none());
    assert!(sp["hyperlink"].is_null());
    let names = part_names(&out);
    assert!(names.iter().any(|n| n.contains("chartshapes")), "{names:?}");
    let chart = part_text(&out, "xl/charts/chart1.xml");
    assert!(chart.contains("userShapes"), "{chart}");
    let shapes = names.iter().find(|n| n.contains("chartshapes")).unwrap();
    let xml = part_text(&out, shapes);
    assert!(xml.contains("relSizeAnchor"), "{xml}");
    assert!(xml.contains("cdr:"), "{xml}");
    let drawing = part_text(&out, "xl/drawings/drawing1.xml");
    assert!(
        !drawing.contains("relSizeAnchor"),
        "叠加对象不应写进 SpreadsheetDrawing：{drawing}"
    );
}

#[test]
fn chartsheet_insert_picture_relsize() {
    let src = create_book("cs-pic");
    let with_cs = tmp("cs-pic-mid");
    apply(&src, &with_cs, json!([chartsheet_op()])).unwrap();
    let png = write_png("cs");
    let out = tmp("cs-pic-out");
    let mut op = json!({
        "op": "insertPicture",
        "id": "sheet:Chart1",
        "file": png.to_string_lossy()
    });
    op.as_object_mut()
        .unwrap()
        .extend(rel_box().as_object().unwrap().clone());
    apply(&with_cs, &out, json!([op])).unwrap();
    let tree = inspect(&out, json!({ "pictures": true }));
    assert!(tree["pictureCount"].as_u64().unwrap() >= 1);
    let cs = tree["sheets"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["name"] == "Chart1")
        .unwrap();
    assert!(cs["pictureCount"].as_u64().unwrap() >= 1);
    let pic = &cs["pictures"][0];
    assert_eq!(pic["anchor"], "relSize");
    assert_eq!(pic["fromX"], json!(0.25));
    assert_eq!(pic["toY"], json!(0.75));
    assert!(pic["hyperlink"].is_null());
    assert_eq!(
        object_keys(pic),
        [
            "id",
            "cNvPrId",
            "name",
            "anchor",
            "fromX",
            "fromY",
            "toX",
            "toY",
            "stable",
            "hyperlink"
        ]
    );
}

#[test]
fn rejects_anchor_and_drawing_ref_mismatch() {
    let src = create_book("rej");
    let png = write_png("rej");
    let with_pic = tmp("rej-pic");
    apply(
        &src,
        &with_pic,
        json!([{
            "op": "insertPicture",
            "id": "sheet:Sheet1",
            "file": png.to_string_lossy(),
            "from": "B2"
        }]),
    )
    .unwrap();
    let pic_id = inspect(&with_pic, json!({ "pictures": true }))["sheets"][0]["pictures"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();

    let err = apply(
        &src,
        &tmp("ws-fromx"),
        json!([{
            "op": "insertPicture",
            "id": "sheet:Sheet1",
            "file": png.to_string_lossy(),
            "fromX": 0.1,
            "fromY": 0.1,
            "toX": 0.5,
            "toY": 0.5
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert_eq!(err.field_path(), Some("ops[0].fromX"));

    let with_cs = tmp("rej-cs");
    apply(&src, &with_cs, json!([chartsheet_op()])).unwrap();
    let cs_from = tmp("cs-from");
    let err = apply(
        &with_cs,
        &cs_from,
        json!([{
            "op": "insertShape",
            "id": "sheet:Chart1",
            "from": "B2"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert_eq!(err.field_path(), Some("ops[0].from"));
    assert!(!cs_from.exists());

    let err = apply(
        &with_pic,
        &tmp("pic-ref"),
        json!([{
            "op": "insertHyperlink",
            "id": pic_id,
            "url": "https://example.com/",
            "ref": "A1"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert_eq!(err.field_path(), Some("ops[0].ref"));
}

#[test]
fn header_footer_slots_list_drawing_counts() {
    let src = create_book("hf");
    let out = tmp("hf-out");
    apply(
        &src,
        &out,
        json!([{
            "op": "setHeaderFooterText",
            "id": "sheet:Sheet1/hf:odd:header",
            "text": "&C页眉"
        }]),
    )
    .unwrap();
    let tree = inspect(&out, json!({ "headerFooter": true }));
    let slot = &tree["sheets"][0]["headerFooters"][0];
    assert_eq!(
        object_keys(slot),
        [
            "id",
            "kind",
            "side",
            "text",
            "pictureCount",
            "textBoxCount",
            "shapeCount",
            "connectorCount"
        ]
    );
}

#[test]
fn delete_chart_drawing_picture_keeps_sheet_drawing() {
    let src = create_book("drop");
    let with_cs = tmp("drop-cs");
    apply(&src, &with_cs, json!([chartsheet_op()])).unwrap();
    let png = write_png("drop");
    let with_pic = tmp("drop-pic");
    let mut op = json!({
        "op": "insertPicture",
        "id": "sheet:Chart1",
        "file": png.to_string_lossy()
    });
    op.as_object_mut()
        .unwrap()
        .extend(rel_box().as_object().unwrap().clone());
    apply(&with_cs, &with_pic, json!([op])).unwrap();
    let pic_id = inspect(&with_pic, json!({ "pictures": true }))["sheets"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["name"] == "Chart1")
        .unwrap()["pictures"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let gone = tmp("drop-out");
    apply(
        &with_pic,
        &gone,
        json!([{ "op": "deletePicture", "id": pic_id }]),
    )
    .unwrap();
    let tree = inspect(&gone, json!({ "pictures": true, "charts": true }));
    let cs = tree["sheets"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["name"] == "Chart1")
        .unwrap();
    assert_eq!(cs["pictureCount"], 0);
    assert_eq!(cs["chartCount"], 1);
    let names = part_names(&gone);
    assert!(
        names.iter().any(|n| n.contains("drawings/drawing")),
        "{names:?}"
    );
    assert!(
        !names.iter().any(|n| n.contains("chartshapes")),
        "{names:?}"
    );
    let cs_xml = part_text(&gone, "xl/chartsheets/sheet1.xml");
    assert!(cs_xml.contains("drawing"), "{cs_xml}");
}
