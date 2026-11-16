//! 货币多段与日期时间出图：string 语义 + export 能出图；方括号颜色仍 NI。
//! 整图 CopyPicture 在本机 ~106dpi 下 sizeDiffer，不在此冒充 px 0。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::json;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-xlsx-numfmt-image");
    std::fs::create_dir_all(&dir).unwrap();
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    dir.join(format!("{tag}-{n}.xlsx"))
}

fn run_ok(value: serde_json::Value) -> RunOutput {
    run(&value.to_string()).unwrap()
}

fn export_png(path: &Path) -> Vec<u8> {
    let out = path.with_extension("png");
    let written = run_ok(json!({
        "schemaVersion": 1,
        "action": "exportSheetImage",
        "path": path.to_string_lossy(),
        "id": "sheet:Sheet1",
        "output": out.to_string_lossy(),
    }));
    let RunOutput::Written {
        path: written_path,
        content_type,
        ..
    } = written
    else {
        panic!("应写出文件");
    };
    assert_eq!(content_type.as_deref(), Some("image/png"));
    assert_eq!(written_path, out);
    std::fs::read(out).unwrap()
}

#[test]
fn currency_without_brackets_exports_png() {
    let path = tmp("currency");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "workbook": {
            "sheets": [{
                "name": "Sheet1",
                "cells": [
                    { "ref": "A1", "value": 1234.5 },
                    { "ref": "B1", "value": -1234.5 },
                    { "ref": "C1", "value": 0.0 }
                ]
            }]
        }
    }));
    run_ok(json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": path.to_string_lossy(),
        "ops": [
            { "op": "setCellStyle", "id": "cell:Sheet1!A1", "numFmt": "$#,##0.00" },
            { "op": "setCellStyle", "id": "cell:Sheet1!B1", "numFmt": "$#,##0.00;($#,##0.00)" },
            { "op": "setCellStyle", "id": "cell:Sheet1!C1", "numFmt": "$#,##0.00;($#,##0.00);\"-\"" }
        ]
    }));
    let png = export_png(&path);
    assert!(png.starts_with(&[0x89, b'P', b'N', b'G']));
    assert!(png.len() > 100);
}

#[test]
fn currency_bracket_color_still_not_implemented() {
    let path = tmp("currency-bracket");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "workbook": {
            "sheets": [{
                "name": "Sheet1",
                "cells": [{ "ref": "A1", "value": 1234.5 }]
            }]
        }
    }));
    run_ok(json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": path.to_string_lossy(),
        "ops": [{
            "op": "setCellStyle",
            "id": "cell:Sheet1!A1",
            "numFmt": "$#,##0.00;[红色]($#,##0.00)"
        }]
    }));
    let err = run(&json!({
        "schemaVersion": 1,
        "action": "exportSheetImage",
        "path": path.to_string_lossy(),
        "id": "sheet:Sheet1",
        "output": path.with_extension("png").to_string_lossy(),
    })
    .to_string())
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::NotImplemented);
}

#[test]
fn datetime_named_formats_export_png() {
    let path = tmp("datetime");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "workbook": {
            "sheets": [{
                "name": "Sheet1",
                "cells": [
                    { "ref": "A1", "value": 44941.0 },
                    { "ref": "B1", "value": 44941.6046875 },
                    { "ref": "C1", "value": 60.0 }
                ]
            }]
        }
    }));
    run_ok(json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": path.to_string_lossy(),
        "ops": [
            { "op": "setCellStyle", "id": "cell:Sheet1!A1", "numFmt": "yyyy/mm/dd" },
            { "op": "setCellStyle", "id": "cell:Sheet1!B1", "numFmt": "m/d/yy h:mm" },
            { "op": "setCellStyle", "id": "cell:Sheet1!C1", "numFmt": "yyyy/mm/dd" }
        ]
    }));
    let png = export_png(&path);
    assert!(png.starts_with(&[0x89, b'P', b'N', b'G']));
    assert!(png.len() > 100);
}
