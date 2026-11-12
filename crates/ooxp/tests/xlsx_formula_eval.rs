//! 公式缓存对照 Excel 16.0 已经记下的结果。库自己算，不启动电子表格。

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, RunOutput};
use serde_json::{json, Value};

static SEQ: AtomicU32 = AtomicU32::new(0);

fn book() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-xlsx-formula-eval");
    std::fs::create_dir_all(&dir).unwrap();
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    dir.join(format!("oracle-{n}.xlsx"))
}

fn cell(tree: &Value, id: &str) -> Value {
    tree["sheets"][0]["cells"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == id)
        .unwrap_or_else(|| panic!("没有 {id}"))
        .clone()
}

#[test]
fn cached_values_match_excel_16() {
    let path = book();
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "format": "xlsx",
        "workbook": {
            "definedNames": [
                { "name": "Rate", "formula": "数据!$B$1" },
                { "name": "LocalAdd", "formula": "2", "localSheet": "数据" }
            ],
            "sheets": [
                {
                    "name": "数据",
                    "cells": [
                        { "ref": "A1", "value": 2 },
                        { "ref": "A2", "value": 11 },
                        { "ref": "B1", "value": 3 },
                        { "ref": "C1", "formula": "=A1+B1" },
                        { "ref": "C2", "formula": "=来源!A1" },
                        { "ref": "C3", "formula": "=Rate*2" },
                        { "ref": "D1", "formula": "=A1*2", "sharedRef": "D1:D2" },
                        { "ref": "E1", "formula": "=A1:A2*10", "arrayRef": "E1:E2" },
                        { "ref": "G2", "value": 10 },
                        { "ref": "G3", "value": 20 },
                        { "ref": "H1", "formula": "=SUM(QtyTable[Qty])" },
                        { "ref": "I1", "formula": "=1/0" },
                        { "ref": "J1", "formula": "=DATE(2026,1,2)" },
                        { "ref": "K1", "formula": "=HYPERLINK(\"#A1\",A1)" },
                        { "ref": "L1", "formula": "=SUM(A1,B1)" },
                        { "ref": "M1", "formula": "=LocalAdd" },
                        { "ref": "N1", "formula": "=TRUE" },
                        { "ref": "O1", "formula": "=NoSuchName" },
                        { "ref": "P1", "formula": "=\"abc\"+1" },
                        { "ref": "Q1", "formula": "=NA()" },
                        { "ref": "R1", "formula": "=SQRT(-1)" },
                        { "ref": "T1", "formula": "=1/3" },
                        { "ref": "U1", "formula": "=B1%" },
                        { "ref": "V1", "formula": "=A1>B1" },
                        { "ref": "W1", "formula": "=A1&\"x\"" },
                        { "ref": "X1", "formula": "=SUM((A1,B1))" },
                        { "ref": "Y1", "formula": "=A1:B2 B1:C1" },
                        { "ref": "Z1", "formula": "=A1:A2 B1:B2" },
                        { "ref": "AA1", "formula": "=SUM(数据:来源!A1)" },
                        { "ref": "AB1", "formula": "=SUM({1,2,3})" },
                        { "ref": "AC1", "formula": "=ABS(A1:A2)" }
                    ],
                    "tables": [{ "ref": "G1:G3", "name": "QtyTable", "columns": ["Qty"] }]
                },
                { "name": "来源", "cells": [{ "ref": "A1", "value": 7 }] }
            ]
        }
    })
    .to_string())
    .unwrap();

    let RunOutput::Inspect(tree) = run(&json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": path.to_string_lossy(),
        "scope": { "sheet": "数据", "range": "A1:AC3" }
    })
    .to_string())
    .unwrap() else {
        panic!("inspect");
    };

    let num = |id: &str, expect: f64| {
        let c = cell(&tree, id);
        let got = c["value"]
            .as_f64()
            .unwrap_or_else(|| panic!("{id} 没有数字：{c}"));
        assert!(
            (got - expect).abs() < 1e-9,
            "{id} got {got} expect {expect}"
        );
    };
    num("cell:数据!C1", 5.0);
    num("cell:数据!C2", 7.0);
    num("cell:数据!C3", 6.0);
    num("cell:数据!D1", 4.0);
    num("cell:数据!D2", 22.0);
    num("cell:数据!E1", 20.0);
    num("cell:数据!E2", 110.0);
    num("cell:数据!H1", 30.0);
    num("cell:数据!J1", 46024.0);
    num("cell:数据!K1", 2.0);
    num("cell:数据!L1", 5.0);
    num("cell:数据!M1", 2.0);
    num("cell:数据!T1", 1.0 / 3.0);
    num("cell:数据!U1", 0.03);
    num("cell:数据!X1", 5.0);
    num("cell:数据!Y1", 3.0);
    num("cell:数据!AA1", 9.0);
    num("cell:数据!AB1", 6.0);
    num("cell:数据!AC1", 2.0);

    let flag = |id: &str, expect: bool| {
        let c = cell(&tree, id);
        assert_eq!(c["type"], "boolean", "{id} {c}");
        assert_eq!(c["value"], expect, "{id}");
    };
    flag("cell:数据!N1", true);
    flag("cell:数据!V1", false);

    let text = cell(&tree, "cell:数据!W1");
    assert_eq!(text["type"], "text");
    assert_eq!(text["value"], "2x");

    let err = |id: &str, token: &str| {
        let c = cell(&tree, id);
        assert_eq!(c["type"], "error", "{id} {c}");
        assert_eq!(c["value"], token, "{id}");
    };
    err("cell:数据!I1", "#DIV/0!");
    err("cell:数据!O1", "#NAME?");
    err("cell:数据!P1", "#VALUE!");
    err("cell:数据!Q1", "#N/A");
    err("cell:数据!R1", "#NUM!");
    err("cell:数据!Z1", "#NULL!");
}

/// 1904 日期系统：2026-01-02 的序列是 44562。创建时没有这个开关，补上后再改公式应重算。
#[test]
fn date_1904_serial_is_44562() {
    let path = book();
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "format": "xlsx",
        "workbook": { "sheets": [{ "name": "Sheet1", "cells": [{ "ref": "A1", "formula": "=DATE(2026,1,2)" }] }] }
    }).to_string())
    .unwrap();

    let file = std::fs::File::open(&path).unwrap();
    let mut archive = zip::ZipArchive::new(file).unwrap();
    let mut parts = Vec::new();
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).unwrap();
        let name = entry.name().to_string();
        let mut buf = Vec::new();
        std::io::Read::read_to_end(&mut entry, &mut buf).unwrap();
        if name == "xl/workbook.xml" {
            let text = String::from_utf8(buf).unwrap();
            buf = text
                .replace("<sheets", "<workbookPr date1904=\"1\"/><sheets")
                .into_bytes();
        }
        parts.push((name, buf));
    }
    drop(archive);
    let file = std::fs::File::create(&path).unwrap();
    let mut writer = zip::ZipWriter::new(file);
    let opts = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    for (name, buf) in parts {
        writer.start_file(name, opts).unwrap();
        use std::io::Write;
        writer.write_all(&buf).unwrap();
    }
    writer.finish().unwrap();

    run(&json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": path.to_string_lossy(),
        "format": "xlsx",
        "ops": [{ "op": "setCell", "id": "cell:Sheet1!A1", "formula": "=DATE(2026,1,2)" }]
    })
    .to_string())
    .unwrap();

    let RunOutput::Inspect(tree) = run(&json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": path.to_string_lossy(),
        "scope": { "sheet": "Sheet1", "range": "A1" }
    })
    .to_string())
    .unwrap() else {
        panic!("inspect");
    };
    let got = cell(&tree, "cell:Sheet1!A1")["value"].as_f64().unwrap();
    assert!((got - 44562.0).abs() < 1e-9, "{got}");
}

fn inspect_range(path: &std::path::Path, range: &str) -> Value {
    let RunOutput::Inspect(tree) = run(&json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": path.to_string_lossy(),
        "scope": { "sheet": "Sheet1", "range": range }
    })
    .to_string())
    .unwrap() else {
        panic!("inspect");
    };
    tree
}

fn part_text(path: &std::path::Path, name: &str) -> String {
    let file = std::fs::File::open(path).unwrap();
    let mut zip = zip::ZipArchive::new(file).unwrap();
    let mut zf = zip.by_name(name).unwrap();
    let mut s = String::new();
    std::io::Read::read_to_string(&mut zf, &mut s).unwrap();
    s
}

fn num_at(tree: &Value, id: &str) -> f64 {
    cell(tree, id)["value"].as_f64().unwrap()
}

#[test]
fn circular_reference_writes_value_error_and_calculate_always() {
    let path = book();
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "workbook": { "sheets": [{ "name": "Sheet1", "cells": [
            { "ref": "A1", "formula": "=B1+1" },
            { "ref": "B1", "formula": "=A1+1" }
        ]}] }
    })
    .to_string())
    .unwrap();
    let tree = inspect_range(&path, "A1:B1");
    assert_eq!(cell(&tree, "cell:Sheet1!A1")["value"], json!("#VALUE!"));
    assert_eq!(cell(&tree, "cell:Sheet1!B1")["value"], json!("#VALUE!"));
    let xml = part_text(&path, "xl/worksheets/sheet1.xml");
    assert!(xml.contains(r#"ca="1""#), "{xml}");
    assert!(xml.contains("#VALUE!"), "{xml}");
}

#[test]
fn data_table_caches_column_row_and_two_d() {
    let col = book();
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": col.to_string_lossy(),
        "workbook": { "sheets": [{ "name": "Sheet1", "cells": [
            { "ref": "C1", "value": 10 },
            { "ref": "B1", "formula": "=C1*2" },
            { "ref": "A2", "value": 1 },
            { "ref": "A3", "value": 2 },
            { "ref": "A4", "value": 3 },
            { "ref": "B2", "formula": "=TABLE(,C1)", "dataTableRef": "B2:B4" }
        ]}] }
    })
    .to_string())
    .unwrap();
    let tree = inspect_range(&col, "B2:B4");
    assert!((num_at(&tree, "cell:Sheet1!B2") - 2.0).abs() < 1e-9);
    assert!((num_at(&tree, "cell:Sheet1!B3") - 4.0).abs() < 1e-9);
    assert!((num_at(&tree, "cell:Sheet1!B4") - 6.0).abs() < 1e-9);

    let row = book();
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": row.to_string_lossy(),
        "workbook": { "sheets": [{ "name": "Sheet1", "cells": [
            { "ref": "A2", "formula": "=C10*3" },
            { "ref": "B1", "value": 2 },
            { "ref": "C1", "value": 4 },
            { "ref": "D1", "value": 5 },
            { "ref": "B2", "formula": "=TABLE(C10,)", "dataTableRef": "B2:D2" }
        ]}] }
    })
    .to_string())
    .unwrap();
    let tree = inspect_range(&row, "B2:D2");
    assert!((num_at(&tree, "cell:Sheet1!B2") - 6.0).abs() < 1e-9);
    assert!((num_at(&tree, "cell:Sheet1!C2") - 12.0).abs() < 1e-9);
    assert!((num_at(&tree, "cell:Sheet1!D2") - 15.0).abs() < 1e-9);

    let two = book();
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": two.to_string_lossy(),
        "workbook": { "sheets": [{ "name": "Sheet1", "cells": [
            { "ref": "A1", "formula": "=X1+Y1" },
            { "ref": "B1", "value": 10 },
            { "ref": "C1", "value": 20 },
            { "ref": "A2", "value": 2 },
            { "ref": "A3", "value": 3 },
            { "ref": "B2", "formula": "=TABLE(X1,Y1)", "dataTableRef": "B2:C3" }
        ]}] }
    })
    .to_string())
    .unwrap();
    let tree = inspect_range(&two, "B2:C3");
    assert!((num_at(&tree, "cell:Sheet1!B2") - 12.0).abs() < 1e-9);
    assert!((num_at(&tree, "cell:Sheet1!C2") - 22.0).abs() < 1e-9);
    assert!((num_at(&tree, "cell:Sheet1!B3") - 13.0).abs() < 1e-9);
    assert!((num_at(&tree, "cell:Sheet1!C3") - 23.0).abs() < 1e-9);
}

#[test]
fn external_workbook_caches_cell_and_sheet_data_set() {
    let src = book();
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": src.to_string_lossy(),
        "workbook": { "sheets": [{ "name": "Sheet1", "cells": [
            { "ref": "A1", "value": 9 }
        ]}] }
    })
    .to_string())
    .unwrap();
    let host = book();
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": host.to_string_lossy(),
        "workbook": { "sheets": [{ "name": "Sheet1", "cells": [
            { "ref": "A1", "value": 1 }
        ]}] }
    })
    .to_string())
    .unwrap();
    let out = book();
    run(&json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": host.to_string_lossy(),
        "output": out.to_string_lossy(),
        "ops": [
            {
                "op": "insertExternalLink",
                "id": "xlsx:workbook",
                "path": src.to_string_lossy(),
                "sheets": ["Sheet1"]
            },
            { "op": "setCell", "id": "cell:Sheet1!B1", "formula": "=[1]Sheet1!A1" }
        ]
    })
    .to_string())
    .unwrap();
    let tree = inspect_range(&out, "B1");
    assert!((num_at(&tree, "cell:Sheet1!B1") - 9.0).abs() < 1e-9);
    let xml = part_text(&out, "xl/externalLinks/externalLink1.xml");
    assert!(xml.contains("<v>9</v>"), "{xml}");
}

#[test]
fn chart_cache_follows_local_cells() {
    let src = book();
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": src.to_string_lossy(),
        "workbook": { "sheets": [{ "name": "Sheet1" }] }
    })
    .to_string())
    .unwrap();
    let charted = book();
    run(&json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": charted.to_string_lossy(),
        "ops": [
            {
                "op": "insertChart",
                "id": "sheet:Sheet1",
                "type": "col",
                "from": "E2",
                "dataFrom": "A10",
                "series": [{ "name": "Q1", "values": [9, 9, 9] }]
            },
            { "op": "setCell", "id": "cell:Sheet1!B11", "value": 4 }
        ]
    })
    .to_string())
    .unwrap();
    let xml = part_text(&charted, "xl/charts/chart1.xml");
    assert!(
        xml.contains(r#"<c:pt idx="0"><c:v>4</c:v></c:pt>"#),
        "{xml}"
    );
    assert!(xml.contains(r#"<c:v>9</c:v>"#), "{xml}");
}

#[test]
fn pivot_row_sum_writes_display() {
    let src = book();
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": src.to_string_lossy(),
        "workbook": { "sheets": [{ "name": "Sheet1", "cells": [
            { "ref": "A1", "value": "产品" },
            { "ref": "B1", "value": "金额" },
            { "ref": "A2", "value": "甲" },
            { "ref": "B2", "value": 10 },
            { "ref": "A3", "value": "甲" },
            { "ref": "B3", "value": 5 },
            { "ref": "A4", "value": "乙" },
            { "ref": "B4", "value": 7 }
        ]}] }
    })
    .to_string())
    .unwrap();
    let out = book();
    run(&json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": out.to_string_lossy(),
        "ops": [{
            "op": "insertPivotTable",
            "id": "sheet:Sheet1",
            "source": { "sheet": "Sheet1", "ref": "A1:B4" },
            "location": "E1",
            "rows": ["产品"],
            "values": ["金额"]
        }]
    })
    .to_string())
    .unwrap();
    let tree = inspect_range(&out, "E1:F4");
    assert_eq!(cell(&tree, "cell:Sheet1!E1")["value"], json!("产品"));
    assert_eq!(cell(&tree, "cell:Sheet1!F1")["value"], json!("金额"));
    assert_eq!(cell(&tree, "cell:Sheet1!E2")["value"], json!("甲"));
    assert!((num_at(&tree, "cell:Sheet1!F2") - 15.0).abs() < 1e-9);
    assert_eq!(cell(&tree, "cell:Sheet1!E3")["value"], json!("乙"));
    assert!((num_at(&tree, "cell:Sheet1!F3") - 7.0).abs() < 1e-9);
    assert_eq!(cell(&tree, "cell:Sheet1!E4")["value"], json!("总计"));
    assert!((num_at(&tree, "cell:Sheet1!F4") - 22.0).abs() < 1e-9);
}

/// 逻辑函数对照 Excel 16.0 样本。库自己算，不启动电子表格。
#[test]
fn logical_functions_match_excel_16() {
    let path = book();
    let cells = vec![
        json!({"ref": "A1", "formula": "=IF(TRUE,1,1/0)"}),
        json!({"ref": "A2", "formula": "=IF(FALSE,1/0,2)"}),
        json!({"ref": "A3", "formula": "=IFERROR(1/0,7)"}),
        json!({"ref": "A4", "formula": "=IFERROR(3,1/0)"}),
        json!({"ref": "A5", "formula": "=AND(TRUE,TRUE)"}),
        json!({"ref": "A6", "formula": "=AND(TRUE,FALSE)"}),
        json!({"ref": "A7", "formula": "=AND(FALSE,1/0)"}),
        json!({"ref": "A8", "formula": "=OR(TRUE,1/0)"}),
        json!({"ref": "A9", "formula": "=OR(FALSE,1/0)"}),
        json!({"ref": "A10", "formula": "=NOT(TRUE)"}),
        json!({"ref": "A11", "formula": "=NOT(0)"}),
        json!({"ref": "A12", "formula": "=NOT(\"a\")"}),
        json!({"ref": "A13", "formula": "=TRUE()"}),
        json!({"ref": "A14", "formula": "=FALSE()"}),
        json!({"ref": "A15", "formula": "=IF(TRUE,5)"}),
        json!({"ref": "A16", "formula": "=IF(FALSE,5)"}),
        json!({"ref": "A17", "formula": "=AND(1,2)"}),
        json!({"ref": "A18", "formula": "=OR(0,0)"}),
        json!({"ref": "A19", "formula": "=if(true,1,1/0)"}),
        json!({"ref": "A20", "formula": "=AND(TRUE,\"x\")"}),
        json!({"ref": "A21", "formula": "=AND(\"x\")"}),
        json!({"ref": "A22", "formula": "=AND(\"TRUE\")"}),
        json!({"ref": "A23", "formula": "=IF(\"TRUE\",1,2)"}),
        json!({"ref": "A24", "formula": "=IF(\" TRUE \",1,2)"}),
        json!({"ref": "A25", "formula": "=IF(\"a\",1/0,2)"}),
        json!({"ref": "A26", "formula": "=IF(-1,8,9)"}),
        json!({"ref": "A27", "formula": "=AND(-1)"}),
        json!({"ref": "A28", "formula": "=AND({0,1})"}),
        json!({"ref": "A29", "formula": "=AND({TRUE,\"x\"})"}),
        json!({"ref": "A30", "formula": "=AND({\"TRUE\"})"}),
        json!({"ref": "A31", "formula": "=NOT({FALSE,TRUE})"}),
        json!({"ref": "A32", "formula": "=IF({TRUE,FALSE},1,0)"}),
        json!({"ref": "A33", "formula": "=ECMA.IF(TRUE,4,1/0)"}),
        json!({"ref": "A34", "formula": "=AND()"}),
        json!({"ref": "A35", "formula": "=IF(TRUE,1,NOSUCH())"}),
        json!({"ref": "A36", "formula": "=IF(TRUE,NOSUCH(),1)"}),
        json!({"ref": "A37", "formula": "=AND(FALSE,NOSUCH())"}),
        json!({"ref": "A38", "formula": "=AND(1/0,NA())"}),
        json!({"ref": "A39", "formula": "=AND(NA(),1/0)"}),
        json!({"ref": "A40", "formula": "=IFERROR(1,\"e\")"}),
        json!({"ref": "A41", "formula": "=NOT(\"FALSE\")"}),
        json!({"ref": "A42", "formula": "=OR(\"false\",\"x\")"}),
        json!({"ref": "B1", "value": true}),
        json!({"ref": "B2", "value": "x"}),
        json!({"ref": "B3", "value": 0}),
        json!({"ref": "B4", "value": false}),
        json!({"ref": "B5", "formula": "=1/0"}),
        json!({"ref": "B7", "value": "FALSE"}),
        json!({"ref": "C1", "formula": "=AND(B1:B4)"}),
        json!({"ref": "C2", "formula": "=AND(B1:B2)"}),
        json!({"ref": "C3", "formula": "=AND(B2)"}),
        json!({"ref": "C4", "formula": "=AND(B6)"}),
        json!({"ref": "C5", "formula": "=IF(B6,1,2)"}),
        json!({"ref": "C6", "formula": "=IFERROR(B6,\"e\")"}),
        json!({"ref": "C7", "formula": "=NOT(B6)"}),
        json!({"ref": "C8", "formula": "=AND(B1:B5)"}),
        json!({"ref": "C9", "formula": "=IF(B1,\"yes\",\"no\")"}),
        json!({"ref": "C10", "formula": "=NOT(B7)"}),
        json!({"ref": "C11", "formula": "=AND(B7)"}),
        json!({"ref": "C12", "formula": "=IF(B7,3,4)"}),
        json!({"ref": "C13", "formula": "=OR(B3:B4)"}),
        json!({"ref": "C14", "formula": "=AND(B8:B9)"}),
    ];
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "workbook": { "sheets": [{ "name": "Sheet1", "cells": cells }] }
    })
    .to_string())
    .unwrap();

    let tree = inspect_range(&path, "A1:C42");
    let num = |id: &str, expect: f64| {
        let c = cell(&tree, id);
        let got = c["value"].as_f64().unwrap_or_else(|| panic!("{id} {c}"));
        assert!((got - expect).abs() < 1e-9, "{id} {got}");
    };
    let flag = |id: &str, expect: bool| {
        let c = cell(&tree, id);
        assert_eq!(c["type"], "boolean", "{id} {c}");
        assert_eq!(c["value"], expect, "{id}");
    };
    let err = |id: &str, token: &str| {
        let c = cell(&tree, id);
        assert_eq!(c["type"], "error", "{id} {c}");
        assert_eq!(c["value"], token, "{id}");
    };
    let blank = |id: &str| {
        let c = cell(&tree, id);
        assert!(c["value"].is_null(), "{id} {c}");
    };

    num("cell:Sheet1!A1", 1.0);
    num("cell:Sheet1!A2", 2.0);
    num("cell:Sheet1!A3", 7.0);
    num("cell:Sheet1!A4", 3.0);
    flag("cell:Sheet1!A5", true);
    flag("cell:Sheet1!A6", false);
    err("cell:Sheet1!A7", "#DIV/0!");
    err("cell:Sheet1!A8", "#DIV/0!");
    err("cell:Sheet1!A9", "#DIV/0!");
    flag("cell:Sheet1!A10", false);
    flag("cell:Sheet1!A11", true);
    err("cell:Sheet1!A12", "#VALUE!");
    flag("cell:Sheet1!A13", true);
    flag("cell:Sheet1!A14", false);
    num("cell:Sheet1!A15", 5.0);
    flag("cell:Sheet1!A16", false);
    flag("cell:Sheet1!A17", true);
    flag("cell:Sheet1!A18", false);
    num("cell:Sheet1!A19", 1.0);
    flag("cell:Sheet1!A20", true);
    err("cell:Sheet1!A21", "#VALUE!");
    flag("cell:Sheet1!A22", true);
    num("cell:Sheet1!A23", 1.0);
    err("cell:Sheet1!A24", "#VALUE!");
    err("cell:Sheet1!A25", "#VALUE!");
    num("cell:Sheet1!A26", 8.0);
    flag("cell:Sheet1!A27", true);
    flag("cell:Sheet1!A28", false);
    flag("cell:Sheet1!A29", true);
    err("cell:Sheet1!A30", "#VALUE!");
    flag("cell:Sheet1!A31", true);
    num("cell:Sheet1!A32", 1.0);
    blank("cell:Sheet1!A33");
    blank("cell:Sheet1!A34");
    num("cell:Sheet1!A35", 1.0);
    blank("cell:Sheet1!A36");
    blank("cell:Sheet1!A37");
    err("cell:Sheet1!A38", "#DIV/0!");
    err("cell:Sheet1!A39", "#N/A");
    num("cell:Sheet1!A40", 1.0);
    flag("cell:Sheet1!A41", true);
    flag("cell:Sheet1!A42", false);
    flag("cell:Sheet1!C1", false);
    flag("cell:Sheet1!C2", true);
    err("cell:Sheet1!C3", "#VALUE!");
    err("cell:Sheet1!C4", "#VALUE!");
    num("cell:Sheet1!C5", 2.0);
    num("cell:Sheet1!C6", 0.0);
    flag("cell:Sheet1!C7", true);
    err("cell:Sheet1!C8", "#DIV/0!");
    let yes = cell(&tree, "cell:Sheet1!C9");
    assert_eq!(yes["type"], "text");
    assert_eq!(yes["value"], "yes");
    flag("cell:Sheet1!C10", true);
    err("cell:Sheet1!C11", "#VALUE!");
    num("cell:Sheet1!C12", 4.0);
    flag("cell:Sheet1!C13", false);
    err("cell:Sheet1!C14", "#VALUE!");
}
