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
