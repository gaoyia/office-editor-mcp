//! 图标集：选择语义已单测；出图在无合法字形前仍 NI（不冒充整图 PASS）。

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode};
use serde_json::json;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-xlsx-icon-set-img");
    std::fs::create_dir_all(&dir).unwrap();
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    dir.join(format!("{tag}-{n}.xlsx"))
}

#[test]
fn icon_set_sheet_export_still_not_implemented() {
    let path = tmp("3arrows");
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "workbook": { "sheets": [{ "name": "Sheet1", "cells": [
            { "ref": "A1", "value": 10 },
            { "ref": "A2", "value": 50 },
            { "ref": "A3", "value": 90 }
        ] }] }
    })
    .to_string())
    .unwrap();
    run(&json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": path.to_string_lossy(),
        "ops": [{
            "op": "insertConditionalFormat",
            "id": "sheet:Sheet1",
            "sqref": ["A1:A3"],
            "type": "iconSet",
            "iconSet": { "set": "3Arrows", "showValue": true, "reverse": false }
        }]
    })
    .to_string())
    .unwrap();

    let err = run(&json!({
        "schemaVersion": 1,
        "action": "exportSheetImage",
        "path": path.to_string_lossy(),
        "id": "sheet:Sheet1",
        "output": path.with_extension("ni.png").to_string_lossy(),
    })
    .to_string())
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::NotImplemented);
}
