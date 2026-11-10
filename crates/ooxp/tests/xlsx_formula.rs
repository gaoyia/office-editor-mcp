//! 共享公式与数组公式：inspect 展开、主格保住或拆组、数组读写。

use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::json;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-xlsx-formula");
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
                    { "ref": "A1", "value": 10 },
                    { "ref": "A2", "value": 11 }
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

fn inspect_cells(path: &Path) -> serde_json::Value {
    inspect_cells_range(path, "A1:C3")
}

fn inspect_cells_range(path: &Path, range: &str) -> serde_json::Value {
    let RunOutput::Inspect(tree) = run(&json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": path.to_string_lossy(),
        "scope": { "sheet": "Sheet1", "range": range }
    })
    .to_string())
    .unwrap() else {
        panic!("inspect 应返回树");
    };
    tree["sheets"][0]["cells"].clone()
}

fn replace_sheet1(path: &Path, sheet_xml: &str) {
    let bytes = std::fs::read(path).unwrap();
    let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut files = Vec::new();
    for i in 0..zip.len() {
        let mut f = zip.by_index(i).unwrap();
        let name = f.name().to_string();
        let mut buf = Vec::new();
        f.read_to_end(&mut buf).unwrap();
        files.push((name, buf));
    }
    drop(zip);
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut zw = ZipWriter::new(&mut cursor);
        let opt = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        for (name, buf) in files {
            zw.start_file(&name, opt).unwrap();
            if name == "xl/worksheets/sheet1.xml" {
                zw.write_all(sheet_xml.as_bytes()).unwrap();
            } else {
                zw.write_all(&buf).unwrap();
            }
        }
        zw.finish().unwrap();
    }
    std::fs::write(path, cursor.into_inner()).unwrap();
}

const COL_TABLE: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData><row r="1"><c r="C1"><v>10</v></c><c r="D1"><v>20</v></c></row><row r="2"><c r="A2"><v>1</v></c><c r="B2"><f t="dataTable" ref="B2:B4" dt2D="0" dtr="0" r1="C1"/></c></row><row r="3"><c r="A3"><v>2</v></c><c r="B3"><v>4</v></c></row><row r="4"><c r="A4"><v>3</v></c><c r="B4"><v>6</v></c></row></sheetData></worksheet>"#;

fn cell_of<'a>(cells: &'a serde_json::Value, id: &str) -> &'a serde_json::Value {
    cells
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == id)
        .unwrap_or_else(|| panic!("树里没有 {id}"))
}

fn with_shared(tag: &str) -> PathBuf {
    let src = create_book(tag);
    let out = tmp(&format!("{tag}-shared"));
    apply(
        &src,
        &out,
        json!([{
            "op": "setCell",
            "id": "cell:Sheet1!B1",
            "formula": "=A1*2",
            "sharedRef": "B1:B2"
        }]),
    )
    .unwrap();
    out
}

#[test]
fn inspect_expands_shared_dependent() {
    let path = with_shared("insp");
    let cells = inspect_cells(&path);
    let b1 = cell_of(&cells, "cell:Sheet1!B1");
    let b2 = cell_of(&cells, "cell:Sheet1!B2");
    assert_eq!(b1["formula"], json!("=A1*2"));
    assert_eq!(b1["shared"], json!(true));
    assert_eq!(b1["sharedRef"], json!("B1:B2"));
    assert_eq!(b2["formula"], json!("=A2*2"));
    assert_eq!(b2["shared"], json!(true));
    assert!(b2.get("sharedRef").is_none());
}

#[test]
fn keep_shared_master_formula() {
    let src = with_shared("keep");
    let out = tmp("keep-out");
    apply(
        &src,
        &out,
        json!([{ "op": "setCell", "id": "cell:Sheet1!B1", "formula": "=A1*3" }]),
    )
    .unwrap();
    let cells = inspect_cells(&out);
    let b1 = cell_of(&cells, "cell:Sheet1!B1");
    let b2 = cell_of(&cells, "cell:Sheet1!B2");
    assert_eq!(b1["formula"], json!("=A1*3"));
    assert_eq!(b1["shared"], json!(true));
    assert_eq!(b1["sharedRef"], json!("B1:B2"));
    assert_eq!(b2["formula"], json!("=A2*3"));
    assert_eq!(b2["shared"], json!(true));
}

#[test]
fn explode_shared_group() {
    let src = with_shared("exp");
    let out = tmp("exp-out");
    apply(
        &src,
        &out,
        json!([{
            "op": "setCell",
            "id": "cell:Sheet1!B1",
            "formula": "=A1*4",
            "shared": false
        }]),
    )
    .unwrap();
    let cells = inspect_cells(&out);
    let b1 = cell_of(&cells, "cell:Sheet1!B1");
    let b2 = cell_of(&cells, "cell:Sheet1!B2");
    assert_eq!(b1["formula"], json!("=A1*4"));
    assert!(b1.get("shared").is_none());
    assert_eq!(b2["formula"], json!("=A2*2"));
    assert!(b2.get("shared").is_none());
}

#[test]
fn dependent_still_becomes_independent() {
    let src = with_shared("dep");
    let out = tmp("dep-out");
    apply(
        &src,
        &out,
        json!([{ "op": "setCell", "id": "cell:Sheet1!B2", "formula": "=A2*9" }]),
    )
    .unwrap();
    let cells = inspect_cells(&out);
    let b1 = cell_of(&cells, "cell:Sheet1!B1");
    let b2 = cell_of(&cells, "cell:Sheet1!B2");
    assert_eq!(b1["formula"], json!("=A1*2"));
    assert_eq!(b1["shared"], json!(true));
    assert_eq!(b2["formula"], json!("=A2*9"));
    assert!(b2.get("shared").is_none());
}

#[test]
fn write_and_inspect_array() {
    let src = create_book("arr");
    let out = tmp("arr-out");
    apply(
        &src,
        &out,
        json!([{
            "op": "setCell",
            "id": "cell:Sheet1!C1",
            "formula": "=A1:A2",
            "arrayRef": "C1:C2"
        }]),
    )
    .unwrap();
    let cells = inspect_cells(&out);
    let c1 = cell_of(&cells, "cell:Sheet1!C1");
    let c2 = cell_of(&cells, "cell:Sheet1!C2");
    assert_eq!(c1["formula"], json!("=A1:A2"));
    assert_eq!(c1["array"], json!(true));
    assert_eq!(c1["arrayRef"], json!("C1:C2"));
    assert_eq!(c2["array"], json!(true));
    assert!(c2["formula"].is_null());
}

#[test]
fn array_part_is_not_implemented() {
    let src = create_book("arrp");
    let mid = tmp("arrp-mid");
    apply(
        &src,
        &mid,
        json!([{
            "op": "setCell",
            "id": "cell:Sheet1!C1",
            "formula": "=A1:A2",
            "arrayRef": "C1:C2"
        }]),
    )
    .unwrap();
    let out = tmp("arrp-out");
    let err = apply(
        &mid,
        &out,
        json!([{ "op": "setCell", "id": "cell:Sheet1!C2", "value": 1 }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::NotImplemented);
    assert!(!out.exists());
}

#[test]
fn shared_true_without_ref_on_plain_cell() {
    let src = create_book("needref");
    let out = tmp("needref-out");
    let err = apply(
        &src,
        &out,
        json!([{
            "op": "setCell",
            "id": "cell:Sheet1!B1",
            "formula": "=A1",
            "shared": true
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());
}

#[test]
fn array_ref_not_top_left() {
    let src = create_book("nottl");
    let out = tmp("nottl-out");
    let err = apply(
        &src,
        &out,
        json!([{
            "op": "setCell",
            "id": "cell:Sheet1!C2",
            "formula": "=A1:A2",
            "arrayRef": "C1:C2"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());
}

#[test]
fn expand_shared_ref() {
    let src = with_shared("grow");
    let out = tmp("grow-out");
    apply(
        &src,
        &out,
        json!([{
            "op": "setCell",
            "id": "cell:Sheet1!B1",
            "formula": "=A1*2",
            "sharedRef": "B1:B3"
        }]),
    )
    .unwrap();
    let cells = inspect_cells(&out);
    let b1 = cell_of(&cells, "cell:Sheet1!B1");
    let b3 = cell_of(&cells, "cell:Sheet1!B3");
    assert_eq!(b1["sharedRef"], json!("B1:B3"));
    assert_eq!(b3["formula"], json!("=A3*2"));
    assert_eq!(b3["shared"], json!(true));
}

fn with_shared_at(tag: &str, cell: &str, formula: &str, refer: &str) -> PathBuf {
    let src = create_book(tag);
    let out = tmp(&format!("{tag}-shared"));
    apply(
        &src,
        &out,
        json!([{
            "op": "setCell",
            "id": format!("cell:Sheet1!{cell}"),
            "formula": formula,
            "sharedRef": refer
        }]),
    )
    .unwrap();
    out
}

#[test]
fn move_shared_master_up() {
    let src = with_shared_at("mvup", "B2", "=A2*2", "B2:B3");
    let out = tmp("mvup-out");
    apply(
        &src,
        &out,
        json!([{
            "op": "setCell",
            "id": "cell:Sheet1!B1",
            "formula": "=A1*2",
            "sharedRef": "B1:B3"
        }]),
    )
    .unwrap();
    let cells = inspect_cells(&out);
    let b1 = cell_of(&cells, "cell:Sheet1!B1");
    let b2 = cell_of(&cells, "cell:Sheet1!B2");
    let b3 = cell_of(&cells, "cell:Sheet1!B3");
    assert_eq!(b1["formula"], json!("=A1*2"));
    assert_eq!(b1["shared"], json!(true));
    assert_eq!(b1["sharedRef"], json!("B1:B3"));
    assert_eq!(b2["formula"], json!("=A2*2"));
    assert_eq!(b2["shared"], json!(true));
    assert!(b2.get("sharedRef").is_none());
    assert_eq!(b3["formula"], json!("=A3*2"));
    assert_eq!(b3["shared"], json!(true));
    assert!(b3.get("sharedRef").is_none());
}

#[test]
fn shrink_shared_master_down() {
    let src = with_shared_at("mvdown", "B1", "=A1*2", "B1:B3");
    let out = tmp("mvdown-out");
    apply(
        &src,
        &out,
        json!([{
            "op": "setCell",
            "id": "cell:Sheet1!B2",
            "formula": "=A2*2",
            "sharedRef": "B2:B3"
        }]),
    )
    .unwrap();
    let cells = inspect_cells(&out);
    let b1 = cell_of(&cells, "cell:Sheet1!B1");
    let b2 = cell_of(&cells, "cell:Sheet1!B2");
    let b3 = cell_of(&cells, "cell:Sheet1!B3");
    assert_eq!(b1["formula"], json!("=A1*2"));
    assert!(b1.get("shared").is_none());
    assert_eq!(b2["formula"], json!("=A2*2"));
    assert_eq!(b2["shared"], json!(true));
    assert_eq!(b2["sharedRef"], json!("B2:B3"));
    assert_eq!(b3["formula"], json!("=A3*2"));
    assert_eq!(b3["shared"], json!(true));
    assert!(b3.get("sharedRef").is_none());
}

#[test]
fn overlap_two_shared_groups_is_not_implemented() {
    let src = with_shared("two");
    let mid = tmp("two-d");
    apply(
        &src,
        &mid,
        json!([{
            "op": "setCell",
            "id": "cell:Sheet1!D1",
            "formula": "=A1*2",
            "sharedRef": "D1:D2"
        }]),
    )
    .unwrap();
    let out = tmp("two-out");
    let err = apply(
        &mid,
        &out,
        json!([{
            "op": "setCell",
            "id": "cell:Sheet1!B1",
            "formula": "=A1*2",
            "sharedRef": "B1:D2"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::NotImplemented);
    assert!(!out.exists());
}

#[test]
fn dependent_shared_true_needs_ref() {
    let src = with_shared("dep-keep");
    let out = tmp("dep-keep-out");
    let err = apply(
        &src,
        &out,
        json!([{
            "op": "setCell",
            "id": "cell:Sheet1!B2",
            "formula": "=A2*2",
            "shared": true
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());
}

#[test]
fn dependent_shared_ref_must_be_top_left() {
    let src = with_shared("dep-tl");
    let out = tmp("dep-tl-out");
    let err = apply(
        &src,
        &out,
        json!([{
            "op": "setCell",
            "id": "cell:Sheet1!B2",
            "formula": "=A2*2",
            "sharedRef": "B1:B2"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());
}

fn with_col_table(tag: &str) -> PathBuf {
    let path = create_book(tag);
    replace_sheet1(&path, COL_TABLE);
    path
}

fn part_text(path: &Path, name: &str) -> String {
    let f = std::fs::File::open(path).unwrap();
    let mut zip = ZipArchive::new(f).unwrap();
    let mut zf = zip.by_name(name).unwrap();
    let mut s = String::new();
    zf.read_to_string(&mut s).unwrap();
    s
}

#[test]
fn inspect_expands_data_table() {
    let path = with_col_table("dt-insp");
    let cells = inspect_cells_range(&path, "A1:D4");
    let b2 = cell_of(&cells, "cell:Sheet1!B2");
    let b3 = cell_of(&cells, "cell:Sheet1!B3");
    assert_eq!(b2["formula"], json!("=TABLE(,C1)"));
    assert_eq!(b2["dataTable"], json!(true));
    assert_eq!(b2["dataTableRef"], json!("B2:B4"));
    assert_eq!(b3["dataTable"], json!(true));
    assert!(b3["formula"].is_null());
    assert!(b3.get("dataTableRef").is_none());
}

#[test]
fn data_table_master_updates_r1() {
    let src = with_col_table("dt-r1");
    let out = tmp("dt-r1-out");
    apply(
        &src,
        &out,
        json!([{ "op": "setCell", "id": "cell:Sheet1!B2", "formula": "=TABLE(,D1)" }]),
    )
    .unwrap();
    let cells = inspect_cells_range(&out, "A1:D4");
    let b2 = cell_of(&cells, "cell:Sheet1!B2");
    assert_eq!(b2["formula"], json!("=TABLE(,D1)"));
    assert_eq!(b2["dataTable"], json!(true));
    assert_eq!(b2["dataTableRef"], json!("B2:B4"));
    let xml = part_text(&out, "xl/worksheets/sheet1.xml");
    assert!(xml.contains(r#"r1="D1""#), "{xml}");
    assert!(
        xml.contains(r#"t="dataTable""#) && !xml.contains(">TABLE"),
        "{xml}"
    );
}

#[test]
fn data_table_non_table_formula_is_invalid() {
    let src = with_col_table("dt-bad");
    let out = tmp("dt-bad-out");
    let err = apply(
        &src,
        &out,
        json!([{ "op": "setCell", "id": "cell:Sheet1!B2", "formula": "=A1" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());
}

#[test]
fn data_table_dependent_is_not_implemented() {
    let src = with_col_table("dt-dep");
    let out = tmp("dt-dep-out");
    let err = apply(
        &src,
        &out,
        json!([{ "op": "setCell", "id": "cell:Sheet1!B3", "formula": "=A3" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::NotImplemented);
    assert!(!out.exists());
}

#[test]
fn inspect_row_and_two_d_data_table() {
    let row_path = create_book("dt-row");
    replace_sheet1(
        &row_path,
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData><row r="2"><c r="B2"><f t="dataTable" ref="B2:D2" dt2D="0" dtr="1" r1="C10"/></c><c r="C2"/><c r="D2"/></row></sheetData></worksheet>"#,
    );
    let row_cells = inspect_cells_range(&row_path, "A1:D2");
    let b2 = cell_of(&row_cells, "cell:Sheet1!B2");
    assert_eq!(b2["formula"], json!("=TABLE(C10,)"));
    assert_eq!(b2["dataTableRef"], json!("B2:D2"));

    let two = create_book("dt-2d");
    replace_sheet1(
        &two,
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData><row r="2"><c r="B2"><f t="dataTable" ref="B2:C3" dt2D="1" dtr="1" r1="X1" r2="Y1"/></c><c r="C2"/></row><row r="3"><c r="B3"/><c r="C3"/></row></sheetData></worksheet>"#,
    );
    let two_cells = inspect_cells_range(&two, "A1:C3");
    let master = cell_of(&two_cells, "cell:Sheet1!B2");
    assert_eq!(master["formula"], json!("=TABLE(X1,Y1)"));
    assert_eq!(master["dataTableRef"], json!("B2:C3"));
    apply(
        &two,
        &tmp("dt-2d-out"),
        json!([{ "op": "setCell", "id": "cell:Sheet1!B2", "formula": "=TABLE(A1,B1)" }]),
    )
    .unwrap();
}

#[test]
fn create_data_table_writes_empty_master() {
    let path = tmp("dt-new");
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "workbook": {
            "sheets": [{
                "name": "Sheet1",
                "cells": [
                    { "ref": "C1", "value": 10 },
                    { "ref": "A2", "value": 1 },
                    { "ref": "B2", "formula": "=TABLE(,C1)", "dataTableRef": "B2:B4" }
                ]
            }]
        }
    })
    .to_string())
    .unwrap();
    let cells = inspect_cells_range(&path, "A1:D4");
    let b2 = cell_of(&cells, "cell:Sheet1!B2");
    let b3 = cell_of(&cells, "cell:Sheet1!B3");
    assert_eq!(b2["formula"], json!("=TABLE(,C1)"));
    assert_eq!(b2["dataTable"], json!(true));
    assert_eq!(b2["dataTableRef"], json!("B2:B4"));
    assert!(b2["value"].is_null());
    assert_eq!(b3["dataTable"], json!(true));
    assert!(b3["formula"].is_null());
    let xml = part_text(&path, "xl/worksheets/sheet1.xml");
    assert!(xml.contains(r#"t="dataTable""#), "{xml}");
    assert!(xml.contains(r#"ref="B2:B4""#), "{xml}");
    assert!(xml.contains(r#"r1="C1""#), "{xml}");
    assert!(!xml.contains(">TABLE"), "{xml}");
}

#[test]
fn create_row_and_two_d_data_table() {
    let row = tmp("dt-new-row");
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": row.to_string_lossy(),
        "workbook": { "sheets": [{ "name": "Sheet1", "cells": [
            { "ref": "B2", "formula": "=TABLE(C10,)", "dataTableRef": "B2:D2" }
        ]}] }
    })
    .to_string())
    .unwrap();
    let row_cells = inspect_cells_range(&row, "A1:D2");
    let b2 = cell_of(&row_cells, "cell:Sheet1!B2");
    assert_eq!(b2["formula"], json!("=TABLE(C10,)"));
    assert_eq!(b2["dataTableRef"], json!("B2:D2"));

    let two = tmp("dt-new-2d");
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": two.to_string_lossy(),
        "workbook": { "sheets": [{ "name": "Sheet1", "cells": [
            { "ref": "B2", "formula": "=TABLE(X1,Y1)", "dataTableRef": "B2:C3" }
        ]}] }
    })
    .to_string())
    .unwrap();
    let two_cells = inspect_cells_range(&two, "A1:C3");
    let master = cell_of(&two_cells, "cell:Sheet1!B2");
    assert_eq!(master["formula"], json!("=TABLE(X1,Y1)"));
    assert_eq!(master["dataTableRef"], json!("B2:C3"));
    let xml = part_text(&two, "xl/worksheets/sheet1.xml");
    assert!(xml.contains(r#"dt2D="1""#), "{xml}");
    assert!(xml.contains(r#"r2="Y1""#), "{xml}");
}

#[test]
fn data_table_ref_cannot_resize() {
    let src = with_col_table("dt-resize");
    let out = tmp("dt-resize-out");
    let err = apply(
        &src,
        &out,
        json!([{
            "op": "setCell",
            "id": "cell:Sheet1!B2",
            "formula": "=TABLE(,C1)",
            "dataTableRef": "B2:B5"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::NotImplemented);
    assert!(!out.exists());
}
