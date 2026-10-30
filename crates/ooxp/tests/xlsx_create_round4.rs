//! xlsx create 第四轮：复用 apply 写列宽、表格、保护、迷你图等。

use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-xlsx-create-round4");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn tmp(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}.xlsx"))
}

fn create(path: &Path, workbook: Value) {
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "workbook": workbook
    })
    .to_string())
    .unwrap();
}

fn create_err(path: &Path, workbook: Value) -> ooxp::Error {
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "workbook": workbook
    })
    .to_string())
    .unwrap_err()
}

fn inspect(path: &Path, scope: Value) -> Value {
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
    let exists = zip.by_name(name).is_ok();
    exists
}

fn cell<'a>(tree: &'a Value, id: &str) -> &'a Value {
    tree["sheets"][0]["cells"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == id)
        .unwrap_or_else(|| panic!("没有 {id}"))
}

#[test]
fn create_writes_layout_style_table_and_protect() {
    let path = tmp("main");
    create(
        &path,
        json!({
            "definedNames": [{ "name": "Sales", "formula": "Sheet1!$A$1:$C$3" }],
            "sheets": [{
                "name": "Sheet1",
                "cells": [
                    { "ref": "A1", "value": "产品", "font": { "bold": true } },
                    { "ref": "B1", "value": "数量" },
                    { "ref": "C1", "value": "金额" },
                    { "ref": "A2", "value": "甲" },
                    { "ref": "B2", "value": 1 },
                    { "ref": "C2", "value": 10 },
                    { "ref": "A3", "value": "乙" },
                    { "ref": "B3", "value": 2 },
                    { "ref": "C3", "value": 20 },
                    { "ref": "E1", "formula": "=B2", "shared": true, "sharedRef": "E1:E2" },
                    { "ref": "G1", "value": 1, "numFmt": "0.00" }
                ],
                "cols": [{ "col": "A", "width": 14 }],
                "rows": [{ "row": 1, "height": 22 }],
                "merges": ["G1:G2"],
                "dataValidations": [{
                    "sqref": "B2:B3",
                    "type": "whole",
                    "operator": "greaterThan",
                    "formula1": "0"
                }],
                "conditionalFormats": [{
                    "sqref": "C2:C3",
                    "type": "cellIs",
                    "operator": "greaterThan",
                    "formulas": ["0"],
                    "dxf": { "font": { "bold": true } }
                }],
                "tables": [{
                    "ref": "A1:C3",
                    "name": "SalesTbl",
                    "columns": ["产品", "数量", "金额"]
                }],
                "print": { "orientation": "landscape" },
                "view": { "frozenRows": 1 },
                "autoFilter": { "ref": "A1:C3" },
                "groupRows": ["2:3"],
                "outline": { "summaryBelow": true },
                "sheetProtection": { "enforced": true },
                "protectedRanges": [{ "name": "Range1", "sqref": "A1:C3" }],
                "sparklines": [{ "ref": "D2", "formula": "B2:C2" }]
            }]
        }),
    );
    let tree = inspect(
        &path,
        json!({
            "sheet": "Sheet1",
            "range": "A1:G3",
            "sparklines": true,
            "protectedRanges": true
        }),
    );
    let sh = &tree["sheets"][0];
    assert_eq!(sh["cols"][0]["width"], json!(14));
    assert_eq!(sh["rows"][0]["height"], json!(22));
    assert_eq!(sh["merges"], json!(["G1:G2"]));
    assert_eq!(sh["dataValidations"][0]["type"], "whole");
    assert_eq!(sh["conditionalFormats"][0]["type"], "cellIs");
    assert_eq!(sh["tableCount"], 1);
    assert_eq!(sh["tables"][0]["name"], "SalesTbl");
    assert_eq!(sh["tables"][0]["id"], "sheet:Sheet1/table:1");
    assert_eq!(sh["print"]["orientation"], "landscape");
    assert_eq!(sh["view"]["frozenRows"], 1);
    assert_eq!(sh["autoFilter"], "A1:C3");
    assert_eq!(sh["outline"]["summaryBelow"], true);
    assert_eq!(sh["sheetProtection"]["enforced"], true);
    assert_eq!(sh["sheetProtection"]["hasPassword"], false);
    assert_eq!(sh["protectedRanges"][0]["name"], "Range1");
    assert_eq!(sh["sparklineCount"], 1);
    assert_eq!(sh["sparklines"][0]["sparklines"][0]["ref"], "D2");
    assert_eq!(tree["definedNameCount"], 1);
    assert_eq!(tree["definedNames"][0]["name"], "Sales");
    let a1 = cell(&tree, "cell:Sheet1!A1");
    assert_eq!(a1["font"]["bold"], true);
    let g1 = cell(&tree, "cell:Sheet1!G1");
    assert_eq!(g1["numFmt"], "0.00");
    let e1 = cell(&tree, "cell:Sheet1!E1");
    assert_eq!(e1["shared"], true);
    assert_eq!(e1["sharedRef"], "E1:E2");
    let xml = part_text(&path, "xl/worksheets/sheet1.xml");
    assert!(xml.contains("protectedRanges"), "{xml}");
    assert!(xml.contains("sparklineGroups"), "{xml}");
    assert!(part_exists(&path, "xl/tables/table1.xml"));
}

#[test]
fn create_style_only_cell_and_groups() {
    let path = tmp("style-grp");
    create(
        &path,
        json!({
            "sheets": [{
                "name": "Sheet1",
                "cells": [{ "ref": "D5", "font": { "italic": true } }],
                "shapes": [
                    { "from": "B2", "prst": "rect", "text": "甲" },
                    { "from": "E4", "prst": "ellipse", "text": "乙" }
                ],
                "groups": [{ "shapes": [1, 2] }]
            }]
        }),
    );
    let tree = inspect(
        &path,
        json!({
            "sheet": "Sheet1",
            "range": "D5:D5",
            "shapes": true
        }),
    );
    let d5 = cell(&tree, "cell:Sheet1!D5");
    assert_eq!(d5["font"]["italic"], true);
    assert_eq!(tree["sheets"][0]["shapeCount"], 2);
    let drawing = part_text(&path, "xl/drawings/drawing1.xml");
    assert!(drawing.contains("xdr:grpSp"), "{drawing}");
}

#[test]
fn create_writes_ole_slicer_pivot_connection_and_hash() {
    let embed = tmp("embed");
    create(
        &embed,
        json!({ "sheets": [{ "name": "Sheet1", "cells": [{ "ref": "A1", "value": 42 }] }] }),
    );
    let path = tmp("all");
    create(
        &path,
        json!({
            "connections": [{
                "name": "WebDemo",
                "type": "web",
                "url": "http://example.com"
            }],
            "externalLinks": [{ "path": r"C:\data\source.xlsx" }],
            "customViews": [{ "name": "V1" }],
            "workbookProtection": { "lockStructure": true },
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
                ],
                "tables": [{
                    "ref": "A1:C4",
                    "name": "Sales",
                    "columns": ["产品", "数量", "金额"]
                }],
                "pivots": [{
                    "source": { "sheet": "Sheet1", "ref": "A1:C4" },
                    "location": "E3",
                    "name": "SalesPivot",
                    "rows": ["产品"],
                    "values": ["金额"]
                }],
                "oleObjects": [{
                    "file": embed.to_string_lossy(),
                    "from": "B8"
                }],
                "slicers": [{
                    "source": "sheet:Sheet1/table:1",
                    "field": "产品",
                    "from": "H2"
                }],
                "sheetProtection": { "enforced": true, "password": "secret" }
            }]
        }),
    );
    let tree = inspect(
        &path,
        json!({
            "sheet": "Sheet1",
            "range": "A1:H10",
            "oleObjects": true,
            "slicers": true,
            "connections": true,
            "externalLinks": true,
            "customViews": true
        }),
    );
    assert_eq!(tree["connectionCount"], 1);
    assert_eq!(tree["connections"][0]["name"], "WebDemo");
    assert_eq!(tree["externalLinkCount"], 1);
    assert_eq!(tree["customViewCount"], 1);
    assert_eq!(tree["workbookProtection"]["lockStructure"], true);
    let sh = &tree["sheets"][0];
    assert_eq!(sh["pivotCount"], 1);
    assert_eq!(sh["pivots"][0]["id"], "sheet:Sheet1/pivot:1");
    assert_eq!(sh["oleObjectCount"], 1);
    assert_eq!(sh["slicerCount"], 1);
    assert_eq!(sh["slicers"][0]["field"], "产品");
    assert_eq!(sh["sheetProtection"]["hasPassword"], true);
    let dump = tree.to_string();
    assert!(!dump.contains("secret"), "{dump}");
    let xml = part_text(&path, "xl/worksheets/sheet1.xml");
    assert!(xml.contains(r#"algorithmName="SHA-512""#), "{xml}");
    assert!(part_exists(&path, "xl/connections.xml"));
    assert!(part_exists(&path, "xl/slicers/slicer1.xml"));
}

#[test]
fn create_chartsheet_print_and_rejects_remain() {
    let path = tmp("cs");
    create(
        &path,
        json!({
            "sheets": [
                { "name": "Sheet1", "cells": [{ "ref": "A10", "value": 1 }] },
                {
                    "name": "Chart1",
                    "kind": "chartsheet",
                    "type": "col",
                    "dataFrom": "A10",
                    "series": [{ "name": "Q1", "values": [1, 2, 3] }],
                    "print": { "orientation": "landscape" },
                    "view": { "zoom": 80 },
                    "sheetProtection": { "enforced": true }
                }
            ]
        }),
    );
    let tree = inspect(&path, json!({}));
    let cs = tree["sheets"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["name"] == "Chart1")
        .unwrap();
    assert_eq!(cs["print"]["orientation"], "landscape");
    assert_eq!(cs["view"]["zoom"], 80);
    assert_eq!(cs["sheetProtection"]["enforced"], true);

    let err = create_err(
        &tmp("cs-cols"),
        json!({
            "sheets": [
                { "name": "Sheet1" },
                {
                    "name": "Chart1",
                    "kind": "chartsheet",
                    "type": "col",
                    "dataFrom": "A10",
                    "series": [{ "name": "Q1", "values": [1] }],
                    "cols": [{ "col": "A", "width": 12 }]
                }
            ]
        }),
    );
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert_eq!(err.field_path(), Some("workbook.sheets[1].cols"));
}

#[test]
fn create_remain_errors_remap_and_reject_meta() {
    let err = create_err(
        &tmp("bad-col"),
        json!({
            "sheets": [{
                "name": "Sheet1",
                "cols": [{ "width": 12 }]
            }]
        }),
    );
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert_eq!(err.field_path(), Some("workbook.sheets[0].cols[0].col"));

    let err = create_err(
        &tmp("op-id"),
        json!({
            "sheets": [{
                "name": "Sheet1",
                "protectedRanges": [{
                    "op": "insertProtectedRange",
                    "name": "Range1",
                    "sqref": "A1"
                }]
            }]
        }),
    );
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert_eq!(
        err.field_path(),
        Some("workbook.sheets[0].protectedRanges[0].op")
    );

    let err = create_err(
        &tmp("dt"),
        json!({
            "sheets": [{
                "name": "Sheet1",
                "cells": [{ "ref": "A1", "formula": "=A2", "dataTable": true }]
            }]
        }),
    );
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert_eq!(
        err.field_path(),
        Some("workbook.sheets[0].cells[0].dataTable")
    );

    let err = create_err(
        &tmp("grp"),
        json!({
            "sheets": [{
                "name": "Sheet1",
                "shapes": [{ "from": "B2", "prst": "rect" }],
                "groups": [{ "shapes": [1] }]
            }]
        }),
    );
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert_eq!(err.field_path(), Some("workbook.sheets[0].groups[0]"));

    let err = create_err(
        &tmp("idx"),
        json!({
            "sheets": [{
                "name": "Sheet1",
                "shapes": [{ "from": "B2", "prst": "rect" }],
                "groups": [{ "shapes": [1, 2] }]
            }]
        }),
    );
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert_eq!(
        err.field_path(),
        Some("workbook.sheets[0].groups[0].shapes[1]")
    );
}
