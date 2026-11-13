//! 工作表迷你图：名单、插改删、重叠拒绝、跨格式拒绝。

use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-xlsx-sparklines");
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
                    { "ref": "B1", "value": 1 },
                    { "ref": "C1", "value": 2 },
                    { "ref": "D1", "value": 3 },
                    { "ref": "E1", "value": 4 },
                    { "ref": "F1", "value": 5 }
                ]
            }]
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

fn inspect_bare(path: &Path) -> Value {
    let RunOutput::Inspect(tree) = run(&json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": path.to_string_lossy()
    })
    .to_string())
    .unwrap() else {
        panic!("inspect 应返回树");
    };
    tree
}

fn inspect_sparks(path: &Path) -> Value {
    let RunOutput::Inspect(tree) = run(&json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": path.to_string_lossy(),
        "scope": { "sparklines": true }
    })
    .to_string())
    .unwrap() else {
        panic!("inspect 应返回树");
    };
    tree
}

fn inspect_cells(path: &Path) -> Value {
    let RunOutput::Inspect(tree) = run(&json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": path.to_string_lossy(),
        "scope": { "sheet": "Sheet1", "range": "B1:F1" }
    })
    .to_string())
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

fn rewrite_part(src: &Path, dst: &Path, name: &str, f: impl FnOnce(&str) -> String) {
    let bytes = std::fs::read(src).unwrap();
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
                let xml = String::from_utf8(data).unwrap();
                data = transform.take().expect("部件只应出现一次")(&xml).into_bytes();
            }
            zip_out.start_file(&n, opt).unwrap();
            zip_out.write_all(&data).unwrap();
        }
        zip_out.finish().unwrap();
    }
    std::fs::write(dst, out.into_inner()).unwrap();
}

#[test]
fn inspect_sparkline_count_without_list() {
    let path = create_book("bare");
    let tree = inspect_bare(&path);
    assert_eq!(tree["sparklineCount"], 0);
    assert_eq!(tree["sheets"][0]["sparklineCount"], 0);
    assert!(tree["sheets"][0].get("sparklines").is_none());
    let listed = inspect_sparks(&path);
    assert_eq!(listed["sheets"][0]["sparklines"], json!([]));
}

#[test]
fn insert_line_inspect_set_delete() {
    let src = create_book("round");
    let inserted = tmp("ins");
    apply(
        &src,
        &inserted,
        json!([{
            "op": "insertSparkline",
            "id": "sheet:Sheet1",
            "ref": "A1",
            "formula": "B1:F1"
        }]),
    )
    .unwrap();
    let sheet = part_text(&inserted, "xl/worksheets/sheet1.xml");
    assert!(sheet.contains("sparklineGroups"));
    assert!(sheet.contains("displayEmptyCellsAs=\"gap\""));
    assert!(sheet.contains("colorSeries"));
    assert!(sheet.contains("xmlns:x14="));
    assert!(sheet.contains("xmlns:xm="));
    assert!(sheet.contains("{05C60535-1F16-4fd2-B633-F4F36F0B64E0}"));
    assert!(sheet.contains("Sheet1!B1:F1"));
    assert!(sheet.contains(">A1<"));
    assert!(!sheet.contains("xr2:uid"));
    assert!(!sheet.contains("type=\"line\""));
    let names = part_names(&inserted);
    assert!(!names
        .iter()
        .any(|n| n.contains("diagrams") || n.contains("drawings")));

    let tree = inspect_sparks(&inserted);
    assert_eq!(tree["sparklineCount"], 1);
    assert_eq!(tree["sheets"][0]["sparklineCount"], 1);
    let groups = tree["sheets"][0]["sparklines"].as_array().unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(
        object_keys(&groups[0]),
        [
            "id",
            "index",
            "type",
            "displayEmptyCellsAs",
            "colors",
            "sparklines"
        ]
    );
    assert_eq!(groups[0]["id"], "sheet:Sheet1/spark:1");
    assert_eq!(groups[0]["index"], 1);
    assert_eq!(groups[0]["type"], "line");
    assert_eq!(groups[0]["displayEmptyCellsAs"], "gap");
    let items = groups[0]["sparklines"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(object_keys(&items[0]), ["ref", "formula", "values"]);
    assert_eq!(items[0]["ref"], "A1");
    assert_eq!(items[0]["formula"], "Sheet1!B1:F1");
    assert_eq!(items[0]["values"], json!([1, 2, 3, 4, 5]));

    let set_out = tmp("set");
    apply(
        &inserted,
        &set_out,
        json!([{
            "op": "setSparkline",
            "id": "sheet:Sheet1/spark:1",
            "formula": "B1:D1",
            "type": "column"
        }]),
    )
    .unwrap();
    let tree_set = inspect_sparks(&set_out);
    let g = &tree_set["sheets"][0]["sparklines"][0];
    assert_eq!(g["type"], "column");
    assert_eq!(g["sparklines"][0]["formula"], "Sheet1!B1:D1");
    assert_eq!(g["sparklines"][0]["values"], json!([1, 2, 3]));
    let sheet_set = part_text(&set_out, "xl/worksheets/sheet1.xml");
    assert!(sheet_set.contains("type=\"column\""));

    let line_out = tmp("line");
    apply(
        &set_out,
        &line_out,
        json!([{
            "op": "setSparkline",
            "id": "sheet:Sheet1/spark:1",
            "type": "line"
        }]),
    )
    .unwrap();
    let sheet_line = part_text(&line_out, "xl/worksheets/sheet1.xml");
    assert!(!sheet_line.contains("type=\""));
    assert_eq!(
        inspect_sparks(&line_out)["sheets"][0]["sparklines"][0]["type"],
        "line"
    );

    let del = tmp("del");
    apply(
        &line_out,
        &del,
        json!([{ "op": "deleteSparkline", "id": "sheet:Sheet1/spark:1" }]),
    )
    .unwrap();
    let tree_del = inspect_sparks(&del);
    assert_eq!(tree_del["sparklineCount"], 0);
    assert_eq!(tree_del["sheets"][0]["sparklines"], json!([]));
    let sheet_del = part_text(&del, "xl/worksheets/sheet1.xml");
    assert!(!sheet_del.contains("sparkline"));
    assert!(!sheet_del.contains("<extLst"));
    let cells = inspect_cells(&del);
    assert_eq!(cells["sheets"][0]["cells"][0]["value"], 1);
}

#[test]
fn missing_ref_or_formula_is_invalid_field() {
    let src = create_book("miss");
    let out = tmp("miss-out");
    let err = apply(
        &src,
        &out,
        json!([{ "op": "insertSparkline", "id": "sheet:Sheet1" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());
    let err2 = apply(
        &src,
        &out,
        json!([{ "op": "insertSparkline", "id": "sheet:Sheet1", "ref": "A1" }]),
    )
    .unwrap_err();
    assert_eq!(err2.code(), ErrorCode::InvalidField);
    assert!(!out.exists());
}

#[test]
fn overlap_and_host_range_rejected() {
    let src = create_book("ov");
    let one = tmp("ov-one");
    apply(
        &src,
        &one,
        json!([{
            "op": "insertSparkline",
            "id": "sheet:Sheet1",
            "ref": "A1",
            "formula": "B1:F1"
        }]),
    )
    .unwrap();
    let two = tmp("ov-two");
    let err = apply(
        &one,
        &two,
        json!([{
            "op": "insertSparkline",
            "id": "sheet:Sheet1",
            "ref": "A1",
            "formula": "B1:C1"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!two.exists());

    let range_out = tmp("host-range");
    let err2 = apply(
        &src,
        &range_out,
        json!([{
            "op": "insertSparkline",
            "id": "sheet:Sheet1",
            "ref": "A1:B1",
            "formula": "B1:F1"
        }]),
    )
    .unwrap_err();
    assert_eq!(err2.code(), ErrorCode::InvalidField);
    assert!(!range_out.exists());
}

#[test]
fn formula_must_be_line_and_gets_sheet_prefix() {
    let src = create_book("fm");
    let bad = tmp("block");
    let err = apply(
        &src,
        &bad,
        json!([{
            "op": "insertSparkline",
            "id": "sheet:Sheet1",
            "ref": "A1",
            "formula": "B1:C2"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!bad.exists());

    let ok = tmp("prefix");
    apply(
        &src,
        &ok,
        json!([{
            "op": "insertSparkline",
            "id": "sheet:Sheet1",
            "ref": "A1",
            "formula": "B1:F1"
        }]),
    )
    .unwrap();
    let g = &inspect_sparks(&ok)["sheets"][0]["sparklines"][0];
    assert_eq!(g["sparklines"][0]["formula"], "Sheet1!B1:F1");
}

#[test]
fn create_empty_sparklines_is_ok() {
    let path = tmp("create");
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "workbook": {
            "sheets": [{
                "name": "Sheet1",
                "sparklines": []
            }]
        }
    })
    .to_string())
    .unwrap();
    assert_eq!(inspect_sparks(&path)["sheets"][0]["sparklineCount"], 0);
}

#[test]
fn insert_wrong_id_and_need_set_field() {
    let src = create_book("id");
    let out = tmp("id-out");
    let err = apply(
        &src,
        &out,
        json!([{
            "op": "insertSparkline",
            "id": "xlsx:workbook",
            "ref": "A1",
            "formula": "B1:F1"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    let err2 = apply(
        &src,
        &out,
        json!([{
            "op": "insertSparkline",
            "id": "sheet:Sheet1/spark:1",
            "ref": "A1",
            "formula": "B1:F1"
        }]),
    )
    .unwrap_err();
    assert_eq!(err2.code(), ErrorCode::InvalidId);

    let one = tmp("one");
    apply(
        &src,
        &one,
        json!([{
            "op": "insertSparkline",
            "id": "sheet:Sheet1",
            "ref": "A1",
            "formula": "B1:F1"
        }]),
    )
    .unwrap();
    let empty = tmp("empty-set");
    let err3 = apply(
        &one,
        &empty,
        json!([{ "op": "setSparkline", "id": "sheet:Sheet1/spark:1" }]),
    )
    .unwrap_err();
    assert_eq!(err3.code(), ErrorCode::InvalidField);
}

#[test]
fn group_with_many_cannot_set_ref() {
    let src = create_book("many");
    let one = tmp("many-one");
    apply(
        &src,
        &one,
        json!([{
            "op": "insertSparkline",
            "id": "sheet:Sheet1",
            "ref": "A1",
            "formula": "B1:F1"
        }]),
    )
    .unwrap();
    let many = tmp("many-xml");
    rewrite_part(&one, &many, "xl/worksheets/sheet1.xml", |xml| {
        xml.replace(
            "</x14:sparklines>",
            "<x14:sparkline><xm:f>Sheet1!B2:F2</xm:f><xm:sqref>A2</xm:sqref></x14:sparkline></x14:sparklines>",
        )
    });
    let tree = inspect_sparks(&many);
    assert_eq!(tree["sparklineCount"], 1);
    assert_eq!(
        tree["sheets"][0]["sparklines"][0]["sparklines"]
            .as_array()
            .unwrap()
            .len(),
        2
    );

    let bad = tmp("many-ref");
    let err = apply(
        &many,
        &bad,
        json!([{
            "op": "setSparkline",
            "id": "sheet:Sheet1/spark:1",
            "ref": "A3"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!bad.exists());

    let ok = tmp("many-type");
    apply(
        &many,
        &ok,
        json!([{
            "op": "setSparkline",
            "id": "sheet:Sheet1/spark:1",
            "type": "column"
        }]),
    )
    .unwrap();
    assert_eq!(
        inspect_sparks(&ok)["sheets"][0]["sparklines"][0]["type"],
        "column"
    );
}

#[test]
fn insert_markers_and_custom_axis() {
    let src = create_book("axis");
    let out = tmp("axis-out");
    apply(
        &src,
        &out,
        json!([{
            "op": "insertSparkline",
            "id": "sheet:Sheet1",
            "ref": "A1",
            "formula": "B1:F1",
            "markers": true,
            "minAxisType": "custom",
            "manualMin": -2,
            "colors": { "series": { "rgb": "FF112233" } }
        }]),
    )
    .unwrap();
    let g = &inspect_sparks(&out)["sheets"][0]["sparklines"][0];
    assert_eq!(g["markers"], true);
    assert_eq!(g["minAxisType"], "custom");
    assert_eq!(g["manualMin"], json!(-2));
    assert_eq!(g["colors"]["series"]["rgb"], "FF112233");
    assert_eq!(g["colors"]["axis"]["rgb"], "FF000000");
    let xml = part_text(&out, "xl/worksheets/sheet1.xml");
    assert!(xml.contains("markers=\"1\""), "{xml}");
    assert!(xml.contains("minAxisType=\"custom\""), "{xml}");
    assert!(xml.contains("manualMin=\"-2\""), "{xml}");
    assert!(xml.contains("FF112233"), "{xml}");

    let set_out = tmp("axis-set");
    apply(
        &out,
        &set_out,
        json!([{
            "op": "setSparkline",
            "id": "sheet:Sheet1/spark:1",
            "markers": false,
            "lineWeight": 1.5,
            "colors": { "negative": { "rgb": "FF00FF00" } }
        }]),
    )
    .unwrap();
    let g2 = &inspect_sparks(&set_out)["sheets"][0]["sparklines"][0];
    assert!(g2.get("markers").is_none());
    assert_eq!(g2["lineWeight"], json!(1.5));
    assert_eq!(g2["colors"]["series"]["rgb"], "FF112233");
    assert_eq!(g2["colors"]["negative"]["rgb"], "FF00FF00");
}

#[test]
fn custom_axis_without_manual_rejected() {
    let src = create_book("axis-bad");
    let out = tmp("axis-bad-out");
    let err = apply(
        &src,
        &out,
        json!([{
            "op": "insertSparkline",
            "id": "sheet:Sheet1",
            "ref": "A1",
            "formula": "B1:F1",
            "minAxisType": "custom"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());
}

#[test]
fn sparkline_values_follow_cached_cells_and_stay_out_of_the_package() {
    let path = tmp("values");
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "workbook": { "sheets": [
            { "name": "Sheet1", "cells": [
                { "ref": "B1", "value": 1 },
                { "ref": "D1", "formula": "=1/0" }
            ] },
            { "name": "Sheet2", "cells": [] }
        ] }
    })
    .to_string())
    .unwrap();
    let out = tmp("values-out");
    apply(
        &path,
        &out,
        json!([
            { "op": "insertSparkline", "id": "sheet:Sheet1", "ref": "A1", "formula": "B1:D1" },
            { "op": "insertSparkline", "id": "sheet:Sheet2", "ref": "A1", "formula": "Sheet1!B1" }
        ]),
    )
    .unwrap();
    let tree = inspect_sparks(&out);
    let g = &tree["sheets"][0]["sparklines"][0]["sparklines"][0];
    assert_eq!(g["values"], json!([1, null, "#DIV/0!"]));
    let other = &tree["sheets"][1]["sparklines"][0]["sparklines"][0];
    assert_eq!(other["values"], json!([1]));
    let xml = part_text(&out, "xl/worksheets/sheet1.xml");
    assert!(!xml.contains("values"), "{xml}");

    let bad = tmp("values-bad");
    apply(
        &out,
        &bad,
        json!([{
            "op": "setSparkline",
            "id": "sheet:Sheet1/spark:1",
            "formula": "SUM(B1)"
        }]),
    )
    .unwrap();
    let item = &inspect_sparks(&bad)["sheets"][0]["sparklines"][0]["sparklines"][0];
    assert!(item.get("values").is_none(), "{item}");
    assert_eq!(item["formula"], "SUM(B1)");
}
