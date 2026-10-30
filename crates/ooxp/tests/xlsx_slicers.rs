//! 工作表切片器：计数、名单、插删；不执行筛选。

use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::json;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-xlsx-slicers");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn tmp(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}.xlsx"))
}

fn create_book(tag: &str) -> PathBuf {
    let path = tmp(tag);
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "workbook": {
            "sheets": [{
                "name": "Sheet1",
                "cells": [
                    { "ref": "A1", "value": "产品" },
                    { "ref": "B1", "value": "数量" },
                    { "ref": "C1", "value": "金额" },
                    { "ref": "A2", "value": "甲" },
                    { "ref": "B2", "value": 1 },
                    { "ref": "C2", "value": 10 },
                    { "ref": "A3", "value": "乙" },
                    { "ref": "B3", "value": 2 },
                    { "ref": "C3", "value": 20 },
                    { "ref": "A4", "value": "甲" },
                    { "ref": "B4", "value": 3 },
                    { "ref": "C4", "value": 30 }
                ]
            }]
        }
    })
    .to_string())
    .unwrap();
    path
}

fn apply(src: &Path, out: &Path, ops: serde_json::Value) -> Result<RunOutput, ooxp::Error> {
    run(&json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": out.to_string_lossy(),
        "ops": ops,
    })
    .to_string())
}

fn inspect(path: &Path, scope: serde_json::Value) -> serde_json::Value {
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

fn object_keys(v: &serde_json::Value) -> Vec<&str> {
    v.as_object().unwrap().keys().map(|k| k.as_str()).collect()
}

fn part_text(path: &Path, name: &str) -> String {
    let f = std::fs::File::open(path).unwrap();
    let mut zip = zip::ZipArchive::new(f).unwrap();
    let mut zf = zip.by_name(name).unwrap();
    let mut s = String::new();
    zf.read_to_string(&mut s).unwrap();
    s
}

fn part_exists(path: &Path, name: &str) -> bool {
    let f = std::fs::File::open(path).unwrap();
    let mut zip = zip::ZipArchive::new(f).unwrap();
    let ok = zip.by_name(name).is_ok();
    ok
}

fn with_table(tag: &str) -> PathBuf {
    let src = create_book(tag);
    let out = tmp(&format!("{tag}-tbl"));
    apply(
        &src,
        &out,
        json!([{
            "op": "insertSheetTable",
            "id": "sheet:Sheet1",
            "ref": "A1:C3",
            "name": "Sales",
            "columns": ["产品", "数量", "金额"]
        }]),
    )
    .unwrap();
    out
}

fn with_pivot(tag: &str) -> PathBuf {
    let src = create_book(tag);
    let out = tmp(&format!("{tag}-pt"));
    apply(
        &src,
        &out,
        json!([{
            "op": "insertPivotTable",
            "id": "sheet:Sheet1",
            "source": { "sheet": "Sheet1", "ref": "A1:C4" },
            "location": "E3",
            "name": "SalesPivot",
            "rows": ["产品"],
            "values": ["金额"]
        }]),
    )
    .unwrap();
    out
}

#[test]
fn empty_create_has_count_without_list() {
    let path = create_book("empty");
    let tree = inspect(&path, json!({}));
    assert_eq!(tree["slicerCount"], 0);
    assert_eq!(tree["sheets"][0]["slicerCount"], 0);
    assert!(tree["sheets"][0].get("slicers").is_none());
    let listed = inspect(&path, json!({ "slicers": true }));
    assert_eq!(listed["sheets"][0]["slicers"], json!([]));
}

#[test]
fn insert_table_slicer_lists_and_writes_parts() {
    let src = with_table("ins-tbl");
    let out = tmp("ins-tbl-out");
    apply(
        &src,
        &out,
        json!([{
            "op": "insertSlicer",
            "id": "sheet:Sheet1",
            "source": "sheet:Sheet1/table:1",
            "field": "产品",
            "from": "F2"
        }]),
    )
    .unwrap();
    let tree = inspect(&out, json!({ "slicers": true, "shapes": true }));
    assert_eq!(tree["slicerCount"], 1);
    assert_eq!(tree["sheets"][0]["slicerCount"], 1);
    let item = &tree["sheets"][0]["slicers"][0];
    let id = item["id"].as_str().unwrap();
    assert!(id.starts_with("sheet:Sheet1/slicer:"), "{id}");
    assert_eq!(item["sourceKind"], "table");
    assert_eq!(item["sourceName"], "Sales");
    assert_eq!(item["field"], "产品");
    assert_eq!(item["caption"], "产品");
    assert_eq!(item["name"], "产品");
    assert_eq!(item["rowHeight"], 381000);
    assert_eq!(item["columnCount"], 1);
    assert_eq!(item["showCaption"], true);
    assert_eq!(item["startItem"], 0);
    assert_eq!(item["lockedPosition"], false);
    assert_eq!(item["style"], serde_json::Value::Null);
    assert_eq!(item["selected"], json!([]));
    assert_eq!(item["anchor"], "twoCell");
    assert_eq!(item["from"], "F2");
    assert_eq!(item["stable"], true);
    assert_eq!(
        object_keys(item),
        [
            "id",
            "cNvPrId",
            "name",
            "caption",
            "rowHeight",
            "columnCount",
            "showCaption",
            "startItem",
            "lockedPosition",
            "style",
            "selected",
            "sourceKind",
            "sourceName",
            "field",
            "anchor",
            "from",
            "fromOffX",
            "fromOffY",
            "to",
            "toOffX",
            "toOffY",
            "editAs",
            "stable"
        ]
    );
    assert_eq!(tree["sheets"][0]["shapeCount"], 0);
    assert!(part_exists(&out, "xl/slicers/slicer1.xml"));
    assert!(part_exists(&out, "xl/slicerCaches/slicerCache1.xml"));
    let cache = part_text(&out, "xl/slicerCaches/slicerCache1.xml");
    assert!(cache.contains("tableSlicerCache"), "{cache}");
    assert!(cache.contains("tableId"), "{cache}");
    let wb = part_text(&out, "xl/workbook.xml");
    assert!(wb.contains("slicerCaches"), "{wb}");
    assert!(wb.contains("Slicer_产品"), "{wb}");
    assert!(wb.contains("#N/A"), "{wb}");
    assert!(wb.contains("xmlns:x15"), "{wb}");
    let sheet = part_text(&out, "xl/worksheets/sheet1.xml");
    assert!(sheet.contains("slicerList"), "{sheet}");
    assert!(sheet.contains("xmlns:x15"), "{sheet}");
    let drawing = part_text(&out, "xl/drawings/drawing1.xml");
    assert!(drawing.contains("sle:slicer"), "{drawing}");
    assert!(drawing.contains("Requires=\"sle15\""), "{drawing}");
    let i_anchor = drawing.find("xdr:twoCellAnchor").expect("twoCellAnchor");
    let i_ac = drawing
        .find("mc:AlternateContent")
        .expect("AlternateContent");
    assert!(i_anchor < i_ac, "{drawing}");
}

#[test]
fn insert_pivot_slicer_source_kind() {
    let src = with_pivot("ins-pt");
    let out = tmp("ins-pt-out");
    apply(
        &src,
        &out,
        json!([{
            "op": "insertSlicer",
            "id": "sheet:Sheet1",
            "source": "sheet:Sheet1/pivot:1",
            "field": "产品",
            "from": "H2"
        }]),
    )
    .unwrap();
    let tree = inspect(&out, json!({ "slicers": true }));
    assert_eq!(tree["slicerCount"], 1);
    let item = &tree["sheets"][0]["slicers"][0];
    assert_eq!(item["sourceKind"], "pivot");
    assert_eq!(item["sourceName"], "SalesPivot");
    assert_eq!(item["field"], "产品");
    assert_eq!(item["selected"], json!(["甲", "乙"]));
    let cache = part_text(&out, "xl/slicerCaches/slicerCache1.xml");
    assert!(cache.contains("tabular"), "{cache}");
    assert!(cache.contains("pivotCacheId=\"1\""), "{cache}");
    let pcd = part_text(&out, "xl/pivotCache/pivotCacheDefinition1.xml");
    assert!(pcd.contains("x14:pivotCacheDefinition"), "{pcd}");
    assert!(pcd.contains("pivotCacheId=\"1\""), "{pcd}");
    assert!(
        pcd.contains("{725AE2AE-9491-48be-B2B4-4EB974FC3084}")
            || pcd.contains("{725AE2AE-9491-48BE-B2B4-4EB974FC3084}"),
        "{pcd}"
    );
    let drawing = part_text(&out, "xl/drawings/drawing1.xml");
    assert!(drawing.contains("Requires=\"a14\""), "{drawing}");
}

#[test]
fn delete_slicer_drops_parts() {
    let src = with_table("del-src");
    let inserted = tmp("del-ins");
    apply(
        &src,
        &inserted,
        json!([{
            "op": "insertSlicer",
            "id": "sheet:Sheet1",
            "source": "sheet:Sheet1/table:1",
            "field": "产品",
            "from": "F2"
        }]),
    )
    .unwrap();
    let tree = inspect(&inserted, json!({ "slicers": true }));
    let id = tree["sheets"][0]["slicers"][0]["id"].as_str().unwrap();
    let out = tmp("del-out");
    apply(&inserted, &out, json!([{ "op": "deleteSlicer", "id": id }])).unwrap();
    let after = inspect(&out, json!({ "slicers": true }));
    assert_eq!(after["slicerCount"], 0);
    assert_eq!(after["sheets"][0]["slicers"], json!([]));
    assert!(!part_exists(&out, "xl/slicers/slicer1.xml"));
    assert!(!part_exists(&out, "xl/slicerCaches/slicerCache1.xml"));
    let wb = part_text(&out, "xl/workbook.xml");
    assert!(!wb.contains("Slicer_产品"), "{wb}");
    assert!(!wb.contains("slicerCaches"), "{wb}");
}

#[test]
fn wrong_ids_and_missing_from() {
    let src = with_table("bad");
    let out = tmp("bad-out");
    let err = apply(
        &src,
        &out,
        json!([{
            "op": "insertSlicer",
            "id": "sheet:Sheet1",
            "source": "sheet:Sheet1/table:1",
            "field": "产品"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());

    let err = apply(
        &src,
        &out,
        json!([{
            "op": "insertSlicer",
            "id": "slide:1",
            "source": "sheet:Sheet1/table:1",
            "field": "产品",
            "from": "F2"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    assert!(!out.exists());

    let err = apply(
        &src,
        &out,
        json!([{
            "op": "insertSlicer",
            "id": "sheet:Sheet1",
            "source": "sheet:Sheet1/table:1",
            "field": "不存在的列",
            "from": "F2"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());
}

fn with_date_pivot(tag: &str) -> PathBuf {
    let path = tmp(tag);
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "workbook": {
            "sheets": [{
                "name": "Sheet1",
                "cells": [
                    { "ref": "A1", "value": "日期" },
                    { "ref": "B1", "value": "数量" },
                    { "ref": "A2", "value": 44927 },
                    { "ref": "B2", "value": 1 },
                    { "ref": "A3", "value": 44958 },
                    { "ref": "B3", "value": 2 },
                    { "ref": "A4", "value": 44986 },
                    { "ref": "B4", "value": 3 }
                ]
            }]
        }
    })
    .to_string())
    .unwrap();
    let out = tmp(&format!("{tag}-pt"));
    apply(
        &path,
        &out,
        json!([{
            "op": "insertPivotTable",
            "id": "sheet:Sheet1",
            "source": { "sheet": "Sheet1", "ref": "A1:B4" },
            "location": "E3",
            "name": "DatePivot",
            "rows": ["日期"],
            "values": ["数量"]
        }]),
    )
    .unwrap();
    out
}

#[test]
fn insert_timeline_on_date_pivot() {
    let src = with_date_pivot("tl-src");
    let out = tmp("tl-ins");
    apply(
        &src,
        &out,
        json!([{
            "op": "insertSlicer",
            "id": "sheet:Sheet1",
            "source": "sheet:Sheet1/pivot:1",
            "field": "日期",
            "kind": "timeline",
            "from": "H2"
        }]),
    )
    .unwrap();
    let tree = inspect(&out, json!({ "slicers": true }));
    assert_eq!(tree["slicerCount"], 1);
    let item = &tree["sheets"][0]["slicers"][0];
    assert_eq!(item["sourceKind"], "timeline");
    assert_eq!(item["sourceName"], "DatePivot");
    assert_eq!(item["field"], "日期");
    assert_eq!(item["caption"], "日期");
    assert_eq!(item["showHeader"], true);
    assert_eq!(item["showSelectionLabel"], true);
    assert_eq!(item["showTimeLevel"], true);
    assert_eq!(item["showHorizontalScrollbar"], true);
    assert_eq!(item["level"], 2);
    assert_eq!(item["selectionLevel"], 2);
    assert_eq!(item["scrollPosition"], "2023-01-01T00:00:00");
    assert_eq!(item["style"], serde_json::Value::Null);
    assert_eq!(item["filter"], serde_json::Value::Null);
    assert!(item["id"]
        .as_str()
        .unwrap()
        .starts_with("sheet:Sheet1/slicer:"));
    assert!(part_exists(&out, "xl/timelines/timeline1.xml"));
    assert!(part_exists(&out, "xl/timelineCaches/timelineCache1.xml"));
    assert!(!part_exists(&out, "xl/slicers/slicer1.xml"));
    let drawing = part_text(&out, "xl/drawings/drawing1.xml");
    assert!(drawing.contains("timeslicer"), "{drawing}");
    assert!(drawing.contains("Requires=\"tsle\""), "{drawing}");
    let cache = part_text(&out, "xl/timelineCaches/timelineCache1.xml");
    assert!(
        cache.contains("startDate=\"2023-01-01T00:00:00\""),
        "{cache}"
    );
    assert!(cache.contains("endDate=\"2023-03-01T00:00:00\""), "{cache}");
    let view = part_text(&out, "xl/timelines/timeline1.xml");
    assert!(view.contains("level=\"2\""), "{view}");
    let wb = part_text(&out, "xl/workbook.xml");
    assert!(wb.contains("timelineCacheRefs"), "{wb}");
    let sheet = part_text(&out, "xl/worksheets/sheet1.xml");
    assert!(sheet.contains("timelineRefs"), "{sheet}");
    let pcd = part_text(&out, "xl/pivotCache/pivotCacheDefinition1.xml");
    assert!(pcd.contains("containsDate=\"1\""), "{pcd}");
    let rec = part_text(&out, "xl/pivotCache/pivotCacheRecords1.xml");
    assert!(
        rec.contains("<x v=\"0\"/>") || rec.contains("<d v="),
        "{rec}"
    );
    assert!(!rec.contains("44927"), "{rec}");
    let pt = part_text(&out, "xl/pivotTables/pivotTable1.xml");
    assert!(pt.contains("updatedVersion=\"8\""), "{pt}");
}

#[test]
fn delete_timeline_drops_parts() {
    let src = with_date_pivot("tl-del-src");
    let inserted = tmp("tl-del-ins");
    apply(
        &src,
        &inserted,
        json!([{
            "op": "insertSlicer",
            "id": "sheet:Sheet1",
            "source": "sheet:Sheet1/pivot:1",
            "field": "日期",
            "kind": "timeline",
            "from": "H2"
        }]),
    )
    .unwrap();
    let tree = inspect(&inserted, json!({ "slicers": true }));
    let id = tree["sheets"][0]["slicers"][0]["id"].as_str().unwrap();
    let out = tmp("tl-del-out");
    apply(&inserted, &out, json!([{ "op": "deleteSlicer", "id": id }])).unwrap();
    let after = inspect(&out, json!({ "slicers": true }));
    assert_eq!(after["slicerCount"], 0);
    assert_eq!(after["sheets"][0]["slicers"], json!([]));
    assert!(!part_exists(&out, "xl/timelines/timeline1.xml"));
    assert!(!part_exists(&out, "xl/timelineCaches/timelineCache1.xml"));
    let wb = part_text(&out, "xl/workbook.xml");
    assert!(!wb.contains("timelineCacheRefs"), "{wb}");
}

#[test]
fn timeline_rejects_table_source_and_text_field() {
    let table = with_table("tl-bad-tbl");
    let out = tmp("tl-bad-out");
    let err = apply(
        &table,
        &out,
        json!([{
            "op": "insertSlicer",
            "id": "sheet:Sheet1",
            "source": "sheet:Sheet1/table:1",
            "field": "产品",
            "kind": "timeline",
            "from": "F2"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());

    let src = with_pivot("tl-text");
    let err = apply(
        &src,
        &out,
        json!([{
            "op": "insertSlicer",
            "id": "sheet:Sheet1",
            "source": "sheet:Sheet1/pivot:1",
            "field": "产品",
            "kind": "timeline",
            "from": "H2"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());
}

fn insert_table_slicer(tag: &str) -> (PathBuf, String) {
    let src = with_table(tag);
    let out = tmp(&format!("{tag}-ins"));
    apply(
        &src,
        &out,
        json!([{
            "op": "insertSlicer",
            "id": "sheet:Sheet1",
            "source": "sheet:Sheet1/table:1",
            "field": "产品",
            "from": "F2"
        }]),
    )
    .unwrap();
    let tree = inspect(&out, json!({ "slicers": true }));
    let id = tree["sheets"][0]["slicers"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    (out, id)
}

#[test]
fn set_slicer_size_and_anchor() {
    let (src, id) = insert_table_slicer("geom");
    let moved = tmp("geom-moved");
    apply(
        &src,
        &moved,
        json!([{ "op": "setSlicerAnchor", "id": id, "from": "G4" }]),
    )
    .unwrap();
    let tree_m = inspect(&moved, json!({ "slicers": true }));
    let item_m = &tree_m["sheets"][0]["slicers"][0];
    assert_eq!(item_m["from"], "G4");
    assert_eq!(item_m["anchor"], "twoCell");
    let drawing_m = part_text(&moved, "xl/drawings/drawing1.xml");
    assert!(drawing_m.contains("sle:slicer"), "{drawing_m}");
    assert!(drawing_m.contains("mc:AlternateContent"), "{drawing_m}");
    assert!(drawing_m.contains("xdr:clientData"), "{drawing_m}");

    let sized = tmp("geom-sized");
    apply(
        &moved,
        &sized,
        json!([{ "op": "setSlicerSize", "id": id, "to": "H12" }]),
    )
    .unwrap();
    let tree_s = inspect(&sized, json!({ "slicers": true }));
    let item_s = &tree_s["sheets"][0]["slicers"][0];
    assert_eq!(item_s["to"], "H12");
    assert_eq!(item_s["from"], "G4");
    let drawing_s = part_text(&sized, "xl/drawings/drawing1.xml");
    assert!(drawing_s.contains("sle:slicer"), "{drawing_s}");
    let i_anchor = drawing_s.find("xdr:twoCellAnchor").expect("twoCellAnchor");
    let i_ac = drawing_s
        .find("mc:AlternateContent")
        .expect("AlternateContent");
    assert!(i_anchor < i_ac, "{drawing_s}");
}

#[test]
fn set_slicer_one_cell_then_size() {
    let (src, id) = insert_table_slicer("one");
    let one = tmp("one-cell");
    apply(
        &src,
        &one,
        json!([{
            "op": "setSlicerAnchor",
            "id": id,
            "anchor": "oneCell",
            "cx": 3657600,
            "cy": 1828800
        }]),
    )
    .unwrap();
    let tree_o = inspect(&one, json!({ "slicers": true }));
    let item_o = &tree_o["sheets"][0]["slicers"][0];
    assert_eq!(item_o["anchor"], "oneCell");
    assert_eq!(item_o["cx"], 3657600);
    assert_eq!(item_o["cy"], 1828800);
    assert!(item_o.get("to").is_none());

    let sized = tmp("one-sz");
    apply(
        &one,
        &sized,
        json!([{ "op": "setSlicerSize", "id": id, "cx": 4572000, "cy": 2286000 }]),
    )
    .unwrap();
    let tree_s = inspect(&sized, json!({ "slicers": true }));
    let item_s = &tree_s["sheets"][0]["slicers"][0];
    assert_eq!(item_s["cx"], 4572000);
    assert_eq!(item_s["cy"], 2286000);
    let drawing = part_text(&sized, "xl/drawings/drawing1.xml");
    assert!(drawing.contains("xdr:oneCellAnchor"), "{drawing}");
    assert!(drawing.contains("mc:AlternateContent"), "{drawing}");
}

#[test]
fn set_timeline_anchor() {
    let src = with_date_pivot("tl-an-src");
    let inserted = tmp("tl-an-ins");
    apply(
        &src,
        &inserted,
        json!([{
            "op": "insertSlicer",
            "id": "sheet:Sheet1",
            "source": "sheet:Sheet1/pivot:1",
            "field": "日期",
            "kind": "timeline",
            "from": "H2"
        }]),
    )
    .unwrap();
    let tree = inspect(&inserted, json!({ "slicers": true }));
    let id = tree["sheets"][0]["slicers"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let moved = tmp("tl-an-moved");
    apply(
        &inserted,
        &moved,
        json!([{ "op": "setSlicerAnchor", "id": id, "from": "I5" }]),
    )
    .unwrap();
    let after = inspect(&moved, json!({ "slicers": true }));
    let item = &after["sheets"][0]["slicers"][0];
    assert_eq!(item["from"], "I5");
    assert_eq!(item["sourceKind"], "timeline");
    let drawing = part_text(&moved, "xl/drawings/drawing1.xml");
    assert!(drawing.contains("tsle:timeslicer"), "{drawing}");
    assert!(drawing.contains("mc:AlternateContent"), "{drawing}");
}

#[test]
fn two_cell_slicer_size_rejects_cx() {
    let (src, id) = insert_table_slicer("tc-cx");
    let fail = tmp("tc-cx-fail");
    let err = apply(
        &src,
        &fail,
        json!([{ "op": "setSlicerSize", "id": id, "cx": 1000 }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!fail.exists());
}

#[test]
fn reject_picture_op_on_slicer_and_slicer_op_on_pic() {
    let (src, id) = insert_table_slicer("rej");
    let fail = tmp("rej-pic");
    let err = apply(
        &src,
        &fail,
        json!([{ "op": "setPictureSize", "id": id, "cx": 1000, "cy": 1000 }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    assert!(!fail.exists());

    let err = apply(
        &src,
        &fail,
        json!([{ "op": "setSlicerSize", "id": "sheet:Sheet1/pic:1", "to": "H12" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    assert!(!fail.exists());
}

#[test]
fn insert_slicer_view_fields_and_set_slicer_view() {
    let src = with_table("view-src");
    let inserted = tmp("view-ins");
    apply(
        &src,
        &inserted,
        json!([{
            "op": "insertSlicer",
            "id": "sheet:Sheet1",
            "source": "sheet:Sheet1/table:1",
            "field": "产品",
            "from": "F2",
            "caption": "类别",
            "columnCount": 2,
            "showCaption": false,
            "style": "SlicerStyleLight1"
        }]),
    )
    .unwrap();
    let tree = inspect(&inserted, json!({ "slicers": true }));
    let item = &tree["sheets"][0]["slicers"][0];
    let id = item["id"].as_str().unwrap().to_string();
    assert_eq!(item["caption"], "类别");
    assert_eq!(item["columnCount"], 2);
    assert_eq!(item["showCaption"], false);
    assert_eq!(item["style"], "SlicerStyleLight1");
    let xml = part_text(&inserted, "xl/slicers/slicer1.xml");
    assert!(xml.contains("caption=\"类别\""), "{xml}");
    assert!(xml.contains("columnCount=\"2\""), "{xml}");
    assert!(xml.contains("showCaption=\"0\""), "{xml}");
    assert!(xml.contains("style=\"SlicerStyleLight1\""), "{xml}");

    let patched = tmp("view-set");
    apply(
        &inserted,
        &patched,
        json!([{
            "op": "setSlicerView",
            "id": id,
            "caption": "产品线",
            "rowHeight": 228600,
            "startItem": 1,
            "lockedPosition": true
        }]),
    )
    .unwrap();
    let after = inspect(&patched, json!({ "slicers": true }));
    let item = &after["sheets"][0]["slicers"][0];
    assert_eq!(item["caption"], "产品线");
    assert_eq!(item["rowHeight"], 228600);
    assert_eq!(item["startItem"], 1);
    assert_eq!(item["lockedPosition"], true);
    assert_eq!(item["columnCount"], 2);
    let drawing = part_text(&patched, "xl/drawings/drawing1.xml");
    assert!(drawing.contains("sle:slicer"), "{drawing}");

    let cleared = tmp("view-style-null");
    apply(
        &patched,
        &cleared,
        json!([{ "op": "setSlicerView", "id": id, "style": null }]),
    )
    .unwrap();
    let item = &inspect(&cleared, json!({ "slicers": true }))["sheets"][0]["slicers"][0];
    assert_eq!(item["style"], serde_json::Value::Null);
    let xml = part_text(&cleared, "xl/slicers/slicer1.xml");
    assert!(!xml.contains("style="), "{xml}");
}

#[test]
fn set_timeline_view_and_reject_cross_kind() {
    let src = with_date_pivot("tl-view-src");
    let inserted = tmp("tl-view-ins");
    apply(
        &src,
        &inserted,
        json!([{
            "op": "insertSlicer",
            "id": "sheet:Sheet1",
            "source": "sheet:Sheet1/pivot:1",
            "field": "日期",
            "kind": "timeline",
            "from": "H2",
            "caption": "月份",
            "showHeader": false,
            "level": 1
        }]),
    )
    .unwrap();
    let tree = inspect(&inserted, json!({ "slicers": true }));
    let item = &tree["sheets"][0]["slicers"][0];
    let id = item["id"].as_str().unwrap().to_string();
    assert_eq!(item["caption"], "月份");
    assert_eq!(item["showHeader"], false);
    assert_eq!(item["level"], 1);
    assert_eq!(item["selectionLevel"], 1);

    let patched = tmp("tl-view-set");
    apply(
        &inserted,
        &patched,
        json!([{
            "op": "setSlicerView",
            "id": id,
            "showTimeLevel": false,
            "selectionLevel": 3,
            "scrollPosition": null
        }]),
    )
    .unwrap();
    let after = inspect(&patched, json!({ "slicers": true }));
    let item = &after["sheets"][0]["slicers"][0];
    assert_eq!(item["showTimeLevel"], false);
    assert_eq!(item["selectionLevel"], 3);
    assert_eq!(item["scrollPosition"], serde_json::Value::Null);
    let xml = part_text(&patched, "xl/timelines/timeline1.xml");
    assert!(!xml.contains("scrollPosition="), "{xml}");

    let fail = tmp("tl-view-cross");
    let err = apply(
        &inserted,
        &fail,
        json!([{ "op": "setSlicerView", "id": id, "columnCount": 2 }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!fail.exists());
}

#[test]
fn set_slicer_view_rejects_empty_and_wrong_kind_and_pic() {
    let (src, id) = insert_table_slicer("view-rej");
    let fail = tmp("view-empty");
    let err = apply(&src, &fail, json!([{ "op": "setSlicerView", "id": id }])).unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!fail.exists());

    let err = apply(
        &src,
        &fail,
        json!([{ "op": "setSlicerView", "id": id, "showHeader": false }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!fail.exists());

    let err = apply(
        &src,
        &fail,
        json!([{ "op": "setSlicerView", "id": "sheet:Sheet1/pic:1", "caption": "x" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    assert!(!fail.exists());

    let err = apply(
        &src,
        &fail,
        json!([{
            "op": "insertSlicer",
            "id": "sheet:Sheet1",
            "source": "sheet:Sheet1/table:1",
            "field": "产品",
            "from": "G2",
            "showHeader": false
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!fail.exists());
}

#[test]
fn table_slicer_selected_writes_autofilter() {
    let (src, id) = insert_table_slicer("sel-tbl");
    let out = tmp("sel-tbl-out");
    apply(
        &src,
        &out,
        json!([{
            "op": "setSlicerFilter",
            "id": id,
            "selected": ["乙"]
        }]),
    )
    .unwrap();
    let item = &inspect(&out, json!({ "slicers": true }))["sheets"][0]["slicers"][0];
    assert_eq!(item["selected"], json!(["乙"]));
    let table = part_text(&out, "xl/tables/table1.xml");
    assert!(table.contains("val=\"乙\""), "{table}");
    assert!(table.contains("filterColumn"), "{table}");

    let cleared = tmp("sel-tbl-clear");
    apply(
        &out,
        &cleared,
        json!([{
            "op": "setSlicerFilter",
            "id": id,
            "selected": []
        }]),
    )
    .unwrap();
    let item = &inspect(&cleared, json!({ "slicers": true }))["sheets"][0]["slicers"][0];
    assert_eq!(item["selected"], json!([]));
    let table = part_text(&cleared, "xl/tables/table1.xml");
    assert!(!table.contains("filterColumn"), "{table}");
}

#[test]
fn pivot_slicer_selected_writes_s_attr() {
    let src = with_pivot("sel-pt");
    let inserted = tmp("sel-pt-ins");
    apply(
        &src,
        &inserted,
        json!([{
            "op": "insertSlicer",
            "id": "sheet:Sheet1",
            "source": "sheet:Sheet1/pivot:1",
            "field": "产品",
            "from": "H2"
        }]),
    )
    .unwrap();
    let tree = inspect(&inserted, json!({ "slicers": true }));
    let id = tree["sheets"][0]["slicers"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let out = tmp("sel-pt-out");
    apply(
        &inserted,
        &out,
        json!([{
            "op": "setSlicerFilter",
            "id": id,
            "selected": ["乙"]
        }]),
    )
    .unwrap();
    let item = &inspect(&out, json!({ "slicers": true }))["sheets"][0]["slicers"][0];
    assert_eq!(item["selected"], json!(["乙"]));
    let cache = part_text(&out, "xl/slicerCaches/slicerCache1.xml");
    assert!(cache.contains("s=\"1\""), "{cache}");
}

#[test]
fn timeline_filter_roundtrip_and_clear() {
    let src = with_date_pivot("sel-tl");
    let inserted = tmp("sel-tl-ins");
    apply(
        &src,
        &inserted,
        json!([{
            "op": "insertSlicer",
            "id": "sheet:Sheet1",
            "source": "sheet:Sheet1/pivot:1",
            "field": "日期",
            "kind": "timeline",
            "from": "H2"
        }]),
    )
    .unwrap();
    let tree = inspect(&inserted, json!({ "slicers": true }));
    let id = tree["sheets"][0]["slicers"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(
        tree["sheets"][0]["slicers"][0]["filter"],
        serde_json::Value::Null
    );
    let out = tmp("sel-tl-out");
    apply(
        &inserted,
        &out,
        json!([{
            "op": "setSlicerFilter",
            "id": id,
            "filter": {
                "filterType": "dateBetween",
                "startDate": "2023-01-01T00:00:00",
                "endDate": "2023-03-01T00:00:00"
            }
        }]),
    )
    .unwrap();
    let item = &inspect(&out, json!({ "slicers": true }))["sheets"][0]["slicers"][0];
    assert_eq!(
        item["filter"],
        json!({
            "filterType": "dateBetween",
            "startDate": "2023-01-01T00:00:00",
            "endDate": "2023-03-01T00:00:00"
        })
    );
    let cache = part_text(&out, "xl/timelineCaches/timelineCache1.xml");
    assert!(!cache.contains("timelinePivotFilter"), "{cache}");
    assert!(cache.contains("filterType=\"dateBetween\""), "{cache}");
    assert!(cache.contains("selection"), "{cache}");

    let cleared = tmp("sel-tl-clear");
    apply(
        &out,
        &cleared,
        json!([{ "op": "setSlicerFilter", "id": id, "filter": null }]),
    )
    .unwrap();
    let item = &inspect(&cleared, json!({ "slicers": true }))["sheets"][0]["slicers"][0];
    assert_eq!(item["filter"], serde_json::Value::Null);
    let cache = part_text(&cleared, "xl/timelineCaches/timelineCache1.xml");
    assert!(!cache.contains("timelinePivotFilter"), "{cache}");
    assert!(cache.contains("filterType=\"unknown\""), "{cache}");
}

#[test]
fn set_slicer_filter_rejects_empty_unknown_and_wrong_id() {
    let (src, id) = insert_table_slicer("sel-rej");
    let fail = tmp("sel-empty");
    let err = apply(&src, &fail, json!([{ "op": "setSlicerFilter", "id": id }])).unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!fail.exists());

    let err = apply(
        &src,
        &fail,
        json!([{
            "op": "setSlicerFilter",
            "id": id,
            "selected": ["不存在"]
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!fail.exists());

    let err = apply(
        &src,
        &fail,
        json!([{
            "op": "setSlicerFilter",
            "id": id,
            "filter": {
                "filterType": "dateBetween",
                "startDate": "2023-01-01T00:00:00",
                "endDate": "2023-03-01T00:00:00"
            }
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!fail.exists());

    let err = apply(
        &src,
        &fail,
        json!([{
            "op": "setSlicerFilter",
            "id": "sheet:Sheet1/pic:1",
            "selected": ["乙"]
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    assert!(!fail.exists());
}
