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

fn formula_marks_ca(xml: &str, body: &str) -> bool {
    let needle = format!(">{body}</f>");
    let Some(idx) = xml.find(&needle) else {
        panic!("没有公式 {body}");
    };
    let start = xml[..idx].rfind("<f").unwrap_or(idx);
    xml[start..idx].contains("ca=")
}

#[test]
fn information_functions_match_excel_16() {
    let path = book();
    let cells = vec![
        json!({"ref": "A2", "value": 0}),
        json!({"ref": "A3", "value": ""}),
        json!({"ref": "A4", "value": "hi"}),
        json!({"ref": "A5", "value": true}),
        json!({"ref": "A6", "formula": "=1/0"}),
        json!({"ref": "A7", "value": 8}),
        json!({"ref": "A8", "value": 2.9}),
        json!({"ref": "A9", "value": 3}),
        json!({"ref": "B1", "formula": "=ISBLANK(A1:A3)"}),
        json!({"ref": "B2", "formula": "=ISBLANK(A2)"}),
        json!({"ref": "B3", "formula": "=ISBLANK(A3)"}),
        json!({"ref": "B4", "formula": "=ISNUMBER(A4:A6)"}),
        json!({"ref": "B5", "formula": "=ISLOGICAL(A4:A6)"}),
        json!({"ref": "B6", "formula": "=ISERR(A4:A6)"}),
        json!({"ref": "B7", "formula": "=ISNUMBER(A7:A9)"}),
        json!({"ref": "B10", "formula": "=ISEVEN(A7:A9)"}),
        json!({"ref": "B11", "formula": "=ERROR.TYPE(A7:A9)"}),
        json!({"ref": "B12", "formula": "=ISERR(A7:A9)"}),
        json!({"ref": "B13", "formula": "=ISNUMBER(A7:A9)"}),
        json!({"ref": "B14", "formula": "=ISBLANK(A1)"}),
        json!({"ref": "B15", "formula": "=ISEVEN(A8)"}),
        json!({"ref": "B16", "formula": "=ISEVEN(A9)"}),
        json!({"ref": "B17", "formula": "=ISEVEN(-2.9)"}),
        json!({"ref": "B18", "formula": "=ISODD(-3)"}),
        json!({"ref": "B19", "formula": "=ISEVEN(A5)"}),
        json!({"ref": "B20", "formula": "=ISODD(A1)"}),
        json!({"ref": "B21", "formula": "=ISLOGICAL({TRUE,2})"}),
        json!({"ref": "B22", "formula": "=ISLOGICAL({2,TRUE})"}),
        json!({"ref": "B23", "formula": "=ISNA(NA())"}),
        json!({"ref": "B24", "formula": "=ISNA(A6)"}),
        json!({"ref": "B25", "formula": "=ISNONTEXT(A6)"}),
        json!({"ref": "B26", "formula": "=ISREF(A1)"}),
        json!({"ref": "B27", "formula": "=ISREF(1)"}),
        json!({"ref": "B28", "formula": "=ISREF(#REF!)"}),
        json!({"ref": "B29", "formula": "=ISREF(Sheet1:Sheet2!A1)"}),
        json!({"ref": "B30", "formula": "=ISREF(Sheet2!A1)"}),
        json!({"ref": "C2", "formula": "=N(A7:A9)"}),
        json!({"ref": "C3", "formula": "=N(A4:A5)"}),
        json!({"ref": "C4", "formula": "=TYPE(A4:A6)"}),
        json!({"ref": "C5", "formula": "=TYPE(A5)"}),
        json!({"ref": "C6", "formula": "=TYPE(A6)"}),
        json!({"ref": "C7", "formula": "=N(A5)"}),
        json!({"ref": "C8", "formula": "=TYPE(A4:A6)"}),
        json!({"ref": "C9", "formula": "=TYPE({1,2})"}),
        json!({"ref": "C10", "formula": "=N({12.5,13.6})"}),
        json!({"ref": "C11", "formula": "=N(A6)"}),
        json!({"ref": "C12", "formula": "=N(A1)"}),
        json!({"ref": "C13", "formula": "=TYPE(A1)"}),
        json!({"ref": "C14", "formula": "=ERROR.TYPE(A6)"}),
        json!({"ref": "C15", "formula": "=ERROR.TYPE(1)"}),
        json!({"ref": "C16", "formula": "=ERROR.TYPE(#NULL!)"}),
        json!({"ref": "C17", "formula": "=ERROR.TYPE(#NUM!)"}),
        json!({"ref": "C18", "formula": "=ERROR.TYPE(NA())"}),
        json!({"ref": "D1", "formula": "=CELL(\"address\",A4)"}),
        json!({"ref": "D2", "formula": "=CELL(\"address\",A1:B2)"}),
        json!({"ref": "D3", "formula": "=CELL(\"col\",A4)"}),
        json!({"ref": "D4", "formula": "=CELL(\"row\",A8)"}),
        json!({"ref": "D5", "formula": "=CELL(\"contents\",A4)"}),
        json!({"ref": "D6", "formula": "=CELL(\"contents\",A5)"}),
        json!({"ref": "D7", "formula": "=CELL(\"contents\",A6)"}),
        json!({"ref": "D8", "formula": "=CELL(\"contents\",A1)"}),
        json!({"ref": "D9", "formula": "=CELL(\"type\",A1)"}),
        json!({"ref": "D10", "formula": "=CELL(\"type\",A4)"}),
        json!({"ref": "D11", "formula": "=CELL(\"type\",A7)"}),
        json!({"ref": "D12", "formula": "=CELL(\"type\",A5)"}),
        json!({"ref": "D13", "formula": "=CELL(\"type\",A6)"}),
        json!({"ref": "D14", "formula": "=CELL(\"filename\",A1)"}),
        json!({"ref": "D15", "formula": "=INFO(\"system\")"}),
        json!({"ref": "D16", "formula": "=INFO(\"recalc\")"}),
        json!({"ref": "D17", "formula": "=INFO(\"release\")"}),
        json!({"ref": "D18", "formula": "=INFO(\"numfile\")"}),
        json!({"ref": "D19", "formula": "=INFO(\"origin\")"}),
        json!({"ref": "D20", "formula": "=INFO(\"nope\")"}),
        json!({"ref": "D21", "formula": "=INFO(\"directory\")"}),
        json!({"ref": "D22", "formula": "=ECMA.ISBLANK(A1)"}),
        json!({"ref": "D23", "formula": "=ISBLANK()"}),
        json!({"ref": "D24", "formula": "=ISTEXT(A4)"}),
        json!({"ref": "D25", "formula": "=ISTEXT(A1)"}),
        json!({"ref": "D26", "formula": "=ISERROR(A6)"}),
        json!({"ref": "D27", "formula": "=ISERROR(0)"}),
    ];
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "workbook": { "sheets": [
            { "name": "Sheet1", "cells": cells },
            { "name": "Sheet2", "cells": [{ "ref": "A1", "value": 9 }] }
        ] }
    })
    .to_string())
    .unwrap();

    let tree = inspect_range(&path, "A1:D30");
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
    let text = |id: &str, expect: &str| {
        let c = cell(&tree, id);
        assert_eq!(c["type"], "text", "{id} {c}");
        assert_eq!(c["value"], expect, "{id}");
    };
    let blank = |id: &str| {
        let c = cell(&tree, id);
        assert!(c["value"].is_null(), "{id} {c}");
    };

    flag("cell:Sheet1!B1", true);
    flag("cell:Sheet1!B2", false);
    flag("cell:Sheet1!B3", false);
    flag("cell:Sheet1!B4", false);
    flag("cell:Sheet1!B5", true);
    flag("cell:Sheet1!B6", true);
    flag("cell:Sheet1!B7", true);
    err("cell:Sheet1!B10", "#VALUE!");
    num("cell:Sheet1!B11", 3.0);
    flag("cell:Sheet1!B12", true);
    flag("cell:Sheet1!B13", false);
    flag("cell:Sheet1!B14", true);
    flag("cell:Sheet1!B15", true);
    flag("cell:Sheet1!B16", false);
    flag("cell:Sheet1!B17", true);
    flag("cell:Sheet1!B18", true);
    err("cell:Sheet1!B19", "#VALUE!");
    flag("cell:Sheet1!B20", false);
    flag("cell:Sheet1!B21", true);
    flag("cell:Sheet1!B22", false);
    flag("cell:Sheet1!B23", true);
    flag("cell:Sheet1!B24", false);
    flag("cell:Sheet1!B25", true);
    flag("cell:Sheet1!B26", true);
    flag("cell:Sheet1!B27", false);
    flag("cell:Sheet1!B28", false);
    flag("cell:Sheet1!B29", false);
    flag("cell:Sheet1!B30", true);
    num("cell:Sheet1!C2", 8.0);
    num("cell:Sheet1!C3", 0.0);
    num("cell:Sheet1!C4", 2.0);
    num("cell:Sheet1!C5", 4.0);
    num("cell:Sheet1!C6", 16.0);
    num("cell:Sheet1!C7", 1.0);
    num("cell:Sheet1!C8", 16.0);
    num("cell:Sheet1!C9", 64.0);
    num("cell:Sheet1!C10", 12.5);
    err("cell:Sheet1!C11", "#DIV/0!");
    num("cell:Sheet1!C12", 0.0);
    num("cell:Sheet1!C13", 1.0);
    num("cell:Sheet1!C14", 2.0);
    err("cell:Sheet1!C15", "#N/A");
    num("cell:Sheet1!C16", 1.0);
    num("cell:Sheet1!C17", 6.0);
    num("cell:Sheet1!C18", 7.0);
    text("cell:Sheet1!D1", "$A$4");
    text("cell:Sheet1!D2", "$A$1");
    num("cell:Sheet1!D3", 1.0);
    num("cell:Sheet1!D4", 8.0);
    text("cell:Sheet1!D5", "hi");
    flag("cell:Sheet1!D6", true);
    err("cell:Sheet1!D7", "#DIV/0!");
    num("cell:Sheet1!D8", 0.0);
    text("cell:Sheet1!D9", "b");
    text("cell:Sheet1!D10", "l");
    text("cell:Sheet1!D11", "v");
    text("cell:Sheet1!D12", "v");
    text("cell:Sheet1!D13", "v");
    blank("cell:Sheet1!D14");
    if cfg!(windows) {
        text("cell:Sheet1!D15", "pcdos");
    } else {
        blank("cell:Sheet1!D15");
    }
    text("cell:Sheet1!D16", "Automatic");
    text("cell:Sheet1!D17", "16.0");
    num("cell:Sheet1!D18", 2.0);
    text("cell:Sheet1!D19", "$A:$A$1");
    err("cell:Sheet1!D20", "#VALUE!");
    blank("cell:Sheet1!D21");
    blank("cell:Sheet1!D22");
    blank("cell:Sheet1!D23");
    flag("cell:Sheet1!D24", true);
    flag("cell:Sheet1!D25", false);
    flag("cell:Sheet1!D26", true);
    flag("cell:Sheet1!D27", false);

    let xml = part_text(&path, "xl/worksheets/sheet1.xml");
    assert!(formula_marks_ca(&xml, "CELL(&quot;address&quot;,A4)"));
    assert!(formula_marks_ca(&xml, "INFO(&quot;recalc&quot;)"));
    assert!(formula_marks_ca(&xml, "INFO(&quot;nope&quot;)"));
    assert!(!formula_marks_ca(&xml, "ISBLANK(A1)"));
    assert!(!formula_marks_ca(&xml, "INFO(&quot;directory&quot;)"));
    assert!(!formula_marks_ca(&xml, "ECMA.ISBLANK(A1)"));
}

#[test]
fn math_functions_match_excel_16() {
    let path = book();
    let mut cells = vec![
        json!({"ref": "A1", "value": 1}),
        json!({"ref": "B1", "value": 2}),
        json!({"ref": "A2", "value": 3}),
        json!({"ref": "B2", "value": 4}),
        json!({"ref": "E1", "value": 1}),
        json!({"ref": "E2", "value": 2}),
        json!({"ref": "E3", "value": 1}),
        json!({"ref": "E4", "value": 3}),
        json!({"ref": "F1", "value": 10}),
        json!({"ref": "F2", "value": 20}),
        json!({"ref": "F3", "value": 30}),
        json!({"ref": "F4", "value": 40}),
        json!({"ref": "H1", "value": 1}),
        json!({"ref": "H2", "value": 2}),
        json!({"ref": "H3", "value": 3}),
        json!({"ref": "G1", "formula": "=SUBTOTAL(9,F1)"}),
        json!({"ref": "G2", "value": 5}),
    ];
    let formulas = [
        ("C1", "=ACOS(0)", std::f64::consts::FRAC_PI_2),
        ("C2", "=ACOSH(1)", 0.0),
        ("C3", "=ASIN(0)", 0.0),
        ("C4", "=ASINH(0)", 0.0),
        ("C5", "=ATAN(1)", std::f64::consts::FRAC_PI_4),
        ("C6", "=ATAN2(1,1)", std::f64::consts::FRAC_PI_4),
        ("C7", "=ATANH(0)", 0.0),
        ("C8", "=COS(0)", 1.0),
        ("C9", "=COSH(0)", 1.0),
        ("C10", "=SIN(0)", 0.0),
        ("C11", "=SINH(0)", 0.0),
        ("C12", "=TAN(0)", 0.0),
        ("C13", "=TANH(0)", 0.0),
        ("C14", "=CEILING(2.5,1)", 3.0),
        ("C15", "=CEILING(-2.5,-1)", -3.0),
        ("C16", "=CEILING(-2.5,1)", -2.0),
        ("C17", "=ECMA.CEILING(2.5,1)", 3.0),
        ("C18", "=ISO.CEILING(-2.5,-1)", -2.0),
        ("C19", "=FLOOR(2.5,1)", 2.0),
        ("C20", "=FLOOR(-2.5,-1)", -2.0),
        ("C21", "=FLOOR(-2.5,1)", -3.0),
        ("C22", "=INT(-8.9)", -9.0),
        ("C23", "=EVEN(3)", 4.0),
        ("C24", "=EVEN(-3)", -4.0),
        ("C25", "=ODD(2)", 3.0),
        ("C26", "=ODD(-2)", -3.0),
        ("C27", "=MROUND(10,3)", 9.0),
        ("C28", "=MROUND(-10,-3)", -9.0),
        ("C29", "=ROUND(2.15,1)", 2.2),
        ("C30", "=ROUND(1.25,1)", 1.3),
        ("C31", "=ROUND(-1.5,0)", -2.0),
        ("C32", "=ROUNDDOWN(2.15,1)", 2.1),
        ("C33", "=ROUNDUP(2.15,1)", 2.2),
        ("C34", "=TRUNC(-2.9)", -2.0),
        ("C35", "=TRUNC(1.29,1)", 1.2),
        ("C36", "=EXP(1)", std::f64::consts::E),
        ("C37", "=LN(1)", 0.0),
        ("C38", "=LOG(8,2)", 3.0),
        ("C39", "=LOG10(100)", 2.0),
        ("C40", "=POWER(-2,3)", -8.0),
        ("C41", "=SQRTPI(1)", 1.772453850905516),
        ("C42", "=COMBIN(5,2)", 10.0),
        ("C43", "=FACT(5)", 120.0),
        ("C44", "=FACTDOUBLE(5)", 15.0),
        ("C45", "=GCD(12,8,20)", 4.0),
        ("C46", "=LCM(4,6)", 12.0),
        ("C47", "=MOD(10,3)", 1.0),
        ("C48", "=MULTINOMIAL(2,3,4)", 1260.0),
        ("C49", "=QUOTIENT(5,2)", 2.0),
        ("C50", "=SIGN(-5)", -1.0),
        ("C51", "=SIGN(0)", 0.0),
        ("C52", "=DEGREES(PI())", 180.0),
        ("C53", "=PI()", std::f64::consts::PI),
        ("C54", "=MDETERM(A1:B2)", -2.0),
        ("C55", "=PRODUCT(2,3,4)", 24.0),
        ("C56", "=SUMIF(E1:E4,1,F1:F4)", 40.0),
        ("C57", "=SUMIFS(F1:F4,E1:E4,1)", 40.0),
        ("C58", "=SUMPRODUCT(E1:E2,F1:F2)", 50.0),
        ("C59", "=SUMSQ(3,4)", 25.0),
        ("C60", "=SUMX2MY2(E1:E2,F1:F2)", -495.0),
        ("C61", "=SUMX2PY2(E1:E2,F1:F2)", 505.0),
        ("C62", "=SUMXMY2(E1:E2,F1:F2)", 405.0),
        ("C63", "=SERIESSUM(2,0,1,H1:H3)", 17.0),
        ("C64", "=SUBTOTAL(9,F1:F4)", 100.0),
        ("C65", "=SUBTOTAL(109,F1:F4)", 80.0),
        ("C66", "=SUBTOTAL(9,G1:G2)", 5.0),
        ("C67", "=MMULT(A1:B2,A1:B2)", 7.0),
    ];
    for (cell, formula, _) in formulas {
        cells.push(json!({"ref": cell, "formula": formula}));
    }
    cells.push(json!({"ref": "J1", "formula": "=MMULT(A1:B2,A1:B2)", "arrayRef": "J1:K2"}));
    cells.push(json!({"ref": "M1", "formula": "=MINVERSE(A1:B2)", "arrayRef": "M1:N2"}));
    cells.push(json!({"ref": "C68", "formula": "=RAND()"}));
    cells.push(json!({"ref": "C69", "formula": "=RANDBETWEEN(1,10)"}));
    cells.push(json!({"ref": "C70", "formula": "=ROMAN(12)"}));
    cells.push(json!({"ref": "C71", "formula": "=ROMAN(1999)"}));
    cells.push(json!({"ref": "C72", "formula": "=ACOS(2)"}));
    cells.push(json!({"ref": "C73", "formula": "=CEILING(2.5,-1)"}));
    cells.push(json!({"ref": "C74", "formula": "=FLOOR(2.5,-1)"}));
    cells.push(json!({"ref": "C75", "formula": "=FLOOR(1,0)"}));
    cells.push(json!({"ref": "C76", "formula": "=CEILING(1,0)"}));
    cells.push(json!({"ref": "C77", "formula": "=MOD(1,0)"}));
    cells.push(json!({"ref": "C78", "formula": "=COMBIN(5,6)"}));
    cells.push(json!({"ref": "C79", "formula": "=ECMA.SIN(0)"}));
    cells.push(json!({"ref": "C80", "formula": "=RADIANS(180)"}));
    cells.push(json!({"ref": "C81", "formula": "=LOG(8)"}));
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "workbook": { "sheets": [{
            "name": "Sheet1",
            "rows": [{ "row": 2, "hidden": true }],
            "cells": cells
        }] }
    })
    .to_string())
    .unwrap();
    let tree = inspect_range(&path, "A1:N4");
    let wide = inspect_range(&path, "C1:C81");
    let near = |tree: &Value, id: &str, expect: f64| {
        let c = cell(tree, id);
        let got = c["value"].as_f64().unwrap_or_else(|| panic!("{id} {c}"));
        assert!(
            (got - expect).abs() < 1e-8,
            "{id} got {got} expect {expect}"
        );
    };
    for (cell_ref, _, expect) in formulas {
        near(&wide, &format!("cell:Sheet1!{cell_ref}"), expect);
    }
    near(&tree, "cell:Sheet1!J1", 7.0);
    near(&tree, "cell:Sheet1!K1", 10.0);
    near(&tree, "cell:Sheet1!J2", 15.0);
    near(&tree, "cell:Sheet1!K2", 22.0);
    near(&tree, "cell:Sheet1!M1", -2.0);
    near(&tree, "cell:Sheet1!N1", 1.0);
    near(&tree, "cell:Sheet1!M2", 1.5);
    near(&tree, "cell:Sheet1!N2", -0.5);
    let text = |id: &str, expect: &str| {
        let c = cell(&wide, id);
        assert_eq!(c["value"], expect, "{id} {c}");
    };
    let err = |id: &str, token: &str| {
        let c = cell(&wide, id);
        assert_eq!(c["type"], "error", "{id} {c}");
        assert_eq!(c["value"], token, "{id}");
    };
    text("cell:Sheet1!C70", "XII");
    text("cell:Sheet1!C71", "MCMXCIX");
    err("cell:Sheet1!C72", "#NUM!");
    err("cell:Sheet1!C73", "#NUM!");
    err("cell:Sheet1!C74", "#NUM!");
    err("cell:Sheet1!C75", "#DIV/0!");
    near(&wide, "cell:Sheet1!C76", 0.0);
    err("cell:Sheet1!C77", "#DIV/0!");
    err("cell:Sheet1!C78", "#NUM!");
    let ecma = cell(&wide, "cell:Sheet1!C79");
    assert!(ecma["value"].is_null(), "{ecma}");
    near(&wide, "cell:Sheet1!C80", std::f64::consts::PI);
    near(&wide, "cell:Sheet1!C81", 8.0_f64.log10());
    let rand = cell(&wide, "cell:Sheet1!C68")["value"].as_f64().unwrap();
    assert!((0.0..1.0).contains(&rand), "{rand}");
    let between = cell(&wide, "cell:Sheet1!C69")["value"].as_f64().unwrap();
    assert!((1.0..=10.0).contains(&between), "{between}");
    let xml = part_text(&path, "xl/worksheets/sheet1.xml");
    assert!(formula_marks_ca(&xml, "RAND()"));
    assert!(formula_marks_ca(&xml, "RANDBETWEEN(1,10)"));
    assert!(!formula_marks_ca(&xml, "COS(0)"));
}

#[test]
fn text_functions_match_excel_16() {
    let path = book();
    let mut cells = vec![
        json!({"ref": "A1", "value": "AbC 中文"}),
        json!({"ref": "A2", "value": "  hi  "}),
        json!({"ref": "A3", "value": 1234.5}),
        json!({"ref": "A4", "value": "ＡＢ１２"}),
    ];
    let texts = [
        ("C1", "=LEFT(A1,3)", "AbC"),
        ("C2", "=RIGHT(A1,2)", "中文"),
        ("C3", "=MID(A1,4,2)", " 中"),
        ("C4", "=LOWER(A1)", "abc 中文"),
        ("C5", "=UPPER(A1)", "ABC 中文"),
        ("C6", "=PROPER(A1)", "Abc 中文"),
        ("C7", "=TRIM(A2)", "hi"),
        ("C8", "=REPT(\"ab\",3)", "ababab"),
        ("C9", "=CONCATENATE(A1,\"!\",1)", "AbC 中文!1"),
        ("C10", "=T(A1)", "AbC 中文"),
        ("C11", "=T(A3)", ""),
        ("C12", "=CHAR(65)", "A"),
        ("C13", "=TEXT(A3,\"0.00\")", "1234.50"),
        ("C14", "=TEXT(0.5,\"0%\")", "50%"),
        ("C15", "=DOLLAR(A3)", "¥1,234.50"),
        ("C16", "=DOLLAR(-1234.5)", "¥-1,234.50"),
        ("C17", "=FIXED(A3,1)", "1,234.5"),
        ("C18", "=FIXED(A3,1,TRUE)", "1234.5"),
        ("C19", "=SUBSTITUTE(A1,\"中文\",\"文\")", "AbC 文"),
        ("C20", "=SUBSTITUTE(\"a-a-a\",\"a\",\"b\",2)", "a-b-a"),
        ("C21", "=REPLACE(A1,1,2,\"x\")", "xC 中文"),
        ("C22", "=ASC(A4)", "AB12"),
        ("C23", "=BAHTTEXT(1234)", "หนึ่งพันสองร้อยสามสิบสี่บาทถ้วน"),
        ("C24", "=BAHTTEXT(1)", "หนึ่งบาทถ้วน"),
        ("C25", "=BAHTTEXT(21)", "ยี่สิบเอ็ดบาทถ้วน"),
        ("C26", "=BAHTTEXT(0)", "ศูนย์บาทถ้วน"),
        ("C27", "=PHONETIC(A1)", "AbC 中文"),
        ("C28", "=CLEAN(\"hi\"&CHAR(7)&\"x\")", "hix"),
    ];
    let nums = [
        ("C29", "=LEN(A1)", 6.0),
        ("C30", "=FIND(\"C\",A1)", 3.0),
        ("C31", "=SEARCH(\"c\",A1)", 3.0),
        ("C32", "=SEARCH(\"中?\",A1)", 5.0),
        ("C33", "=SEARCH(\"A*\",A1)", 1.0),
        ("C34", "=VALUE(\"1,234.5\")", 1234.5),
        ("C35", "=CODE(\"A\")", 65.0),
    ];
    let dbcs_text = [
        ("C40", "=LEFTB(A1,5)", "AbC  "),
        ("C41", "=RIGHTB(A1,3)", " 文"),
        ("C42", "=MIDB(A1,4,2)", "  "),
        ("C43", "=REPLACEB(A1,5,2,\"X\")", "AbC X文"),
        ("C44", "=REPLACEB(A1,4,2,\"Z\")", "AbCZ 文"),
        ("C45", "=LEFTB(A4,3)", "Ａ "),
        ("C46", "=CHAR(54992)", "中"),
    ];
    let dbcs_num = [
        ("C47", "=LENB(A1)", 8.0),
        ("C48", "=LENB(A4)", 8.0),
        ("C49", "=FINDB(\"中\",A1)", 5.0),
        ("C50", "=SEARCHB(\"文\",A1)", 7.0),
        ("C51", "=CODE(\"中\")", 54992.0),
        ("C52", "=CODE(A4)", 41921.0),
    ];
    for (cell_ref, formula, _) in texts.iter().chain(dbcs_text.iter()) {
        cells.push(json!({"ref": cell_ref, "formula": formula}));
    }
    for (cell_ref, formula, _) in nums.iter().chain(dbcs_num.iter()) {
        cells.push(json!({"ref": cell_ref, "formula": formula}));
    }
    cells.push(json!({"ref": "C36", "formula": "=FIND(\"c\",A1)"}));
    cells.push(json!({"ref": "C37", "formula": "=EXACT(\"Ab\",\"ab\")"}));
    cells.push(json!({"ref": "C38", "formula": "=ECMA.LEFT(A1,1)"}));
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "workbook": { "sheets": [{ "name": "Sheet1", "cells": cells }] }
    })
    .to_string())
    .unwrap();
    let tree = inspect_range(&path, "A1:C52");
    let text = |id: &str, expect: &str| {
        let c = cell(&tree, id);
        assert_eq!(c["value"], expect, "{id} {c}");
    };
    let num = |id: &str, expect: f64| {
        let c = cell(&tree, id);
        let got = c["value"].as_f64().unwrap_or_else(|| panic!("{id} {c}"));
        assert!((got - expect).abs() < 1e-8, "{id} got {got}");
    };
    for (cell_ref, _, expect) in texts {
        text(&format!("cell:Sheet1!{cell_ref}"), expect);
    }
    for (cell_ref, _, expect) in nums {
        num(&format!("cell:Sheet1!{cell_ref}"), expect);
    }
    let missed = cell(&tree, "cell:Sheet1!C36");
    assert_eq!(missed["type"], "error", "{missed}");
    assert_eq!(missed["value"], "#VALUE!");
    let exact = cell(&tree, "cell:Sheet1!C37");
    assert_eq!(exact["type"], "boolean", "{exact}");
    assert_eq!(exact["value"], false);
    let ecma = cell(&tree, "cell:Sheet1!C38");
    assert!(ecma["value"].is_null(), "{ecma}");
    #[cfg(windows)]
    {
        for (cell_ref, _, expect) in dbcs_text {
            text(&format!("cell:Sheet1!{cell_ref}"), expect);
        }
        for (cell_ref, _, expect) in dbcs_num {
            num(&format!("cell:Sheet1!{cell_ref}"), expect);
        }
    }
    #[cfg(not(windows))]
    {
        for (cell_ref, _, _) in dbcs_text.iter().chain(dbcs_num.iter()) {
            let c = cell(&tree, &format!("cell:Sheet1!{cell_ref}"));
            assert!(c["value"].is_null(), "{cell_ref} {c}");
        }
    }
}
