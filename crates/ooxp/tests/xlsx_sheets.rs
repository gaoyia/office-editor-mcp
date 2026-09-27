//! 工作表管理：插、删、改名、重排、隐藏，以及引用拒绝。

use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::json;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-xlsx-sheets");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn tmp(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}.xlsx"))
}

fn create_book(tag: &str, sheets: serde_json::Value) -> PathBuf {
    let path = tmp(tag);
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "workbook": { "sheets": sheets }
        })
        .to_string(),
        None,
    )
    .unwrap();
    path
}

fn apply(src: &Path, out: &Path, ops: serde_json::Value) -> Result<RunOutput, ooxp::Error> {
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

fn inspect(path: &Path) -> serde_json::Value {
    let RunOutput::Inspect(tree) = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": path.to_string_lossy()
        })
        .to_string(),
        None,
    )
    .unwrap() else {
        panic!("inspect 应返回树");
    };
    tree
}

fn object_keys(v: &serde_json::Value) -> Vec<&str> {
    v.as_object().unwrap().keys().map(String::as_str).collect()
}

fn names(tree: &serde_json::Value) -> Vec<String> {
    tree["sheets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["name"].as_str().unwrap().to_string())
        .collect()
}

fn rewrite_part(path: &Path, name: &str, f: impl FnOnce(Vec<u8>) -> Vec<u8>) {
    let bytes = std::fs::read(path).unwrap();
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
                data = transform.take().expect("部件只应出现一次")(data);
            }
            zip_out.start_file(&n, opt).unwrap();
            zip_out.write_all(&data).unwrap();
        }
        zip_out.finish().unwrap();
    }
    std::fs::write(path, out.into_inner()).unwrap();
}

#[test]
fn inspect_lists_hidden() {
    let path = create_book(
        "hidden",
        json!([{ "name": "Sheet1", "cells": [{ "ref": "A1", "value": 1 }] }]),
    );
    let tree = inspect(&path);
    assert_eq!(tree["sheets"][0]["hidden"], false);
    assert_eq!(
        object_keys(&tree["sheets"][0]),
        [
            "id",
            "name",
            "kind",
            "hidden",
            "pictureCount",
            "chartCount",
            "dataValidationCount",
            "hasHeaderFooter"
        ]
    );
}

#[test]
fn insert_rename_move_hide_delete() {
    let src = create_book(
        "round",
        json!([
            { "name": "Sheet1", "cells": [{ "ref": "A1", "value": 1 }] },
            { "name": "Sheet2", "cells": [{ "ref": "A1", "value": 2 }] }
        ]),
    );
    let inserted = tmp("ins");
    apply(
        &src,
        &inserted,
        json!([{
            "op": "insertSheet",
            "id": "xlsx:workbook",
            "name": "Sheet3",
            "after": "sheet:Sheet1"
        }]),
    )
    .unwrap();
    let tree = inspect(&inserted);
    assert_eq!(names(&tree), ["Sheet1", "Sheet3", "Sheet2"]);
    assert_eq!(tree["sheets"][1]["kind"], "worksheet");
    assert_eq!(tree["sheets"][1]["hidden"], false);

    let renamed = tmp("ren");
    apply(
        &inserted,
        &renamed,
        json!([{ "op": "renameSheet", "id": "sheet:Sheet3", "name": "附录" }]),
    )
    .unwrap();
    let tree = inspect(&renamed);
    assert_eq!(names(&tree), ["Sheet1", "附录", "Sheet2"]);
    let err = apply(
        &renamed,
        &tmp("old-id"),
        json!([{ "op": "setCell", "id": "cell:Sheet3!A1", "value": 9 }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::IdNotFound);
    apply(
        &renamed,
        &tmp("new-id"),
        json!([{ "op": "setCell", "id": "cell:附录!A1", "value": 9 }]),
    )
    .unwrap();

    let moved = tmp("mov");
    apply(
        &renamed,
        &moved,
        json!([{ "op": "moveSheet", "id": "sheet:附录" }]),
    )
    .unwrap();
    assert_eq!(names(&inspect(&moved)), ["附录", "Sheet1", "Sheet2"]);

    let hidden = tmp("hid");
    apply(
        &moved,
        &hidden,
        json!([{ "op": "setSheetHidden", "id": "sheet:Sheet2", "hidden": true }]),
    )
    .unwrap();
    let tree = inspect(&hidden);
    assert_eq!(tree["sheets"][2]["name"], "Sheet2");
    assert_eq!(tree["sheets"][2]["hidden"], true);

    let deleted = tmp("del");
    apply(
        &hidden,
        &deleted,
        json!([{ "op": "deleteSheet", "id": "sheet:Sheet2" }]),
    )
    .unwrap();
    assert_eq!(names(&inspect(&deleted)), ["附录", "Sheet1"]);
}

#[test]
fn refuse_last_visible_and_last_sheet() {
    let src = create_book(
        "last",
        json!([
            { "name": "Sheet1", "cells": [{ "ref": "A1", "value": 1 }] },
            { "name": "Sheet2", "cells": [{ "ref": "A1", "value": 2 }] }
        ]),
    );
    let hidden = tmp("hid2");
    apply(
        &src,
        &hidden,
        json!([{ "op": "setSheetHidden", "id": "sheet:Sheet2", "hidden": true }]),
    )
    .unwrap();
    let err = apply(
        &hidden,
        &tmp("hid1"),
        json!([{ "op": "setSheetHidden", "id": "sheet:Sheet1", "hidden": true }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    let err = apply(
        &hidden,
        &tmp("del1"),
        json!([{ "op": "deleteSheet", "id": "sheet:Sheet1" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert_eq!(inspect(&hidden)["sheets"].as_array().unwrap().len(), 2);

    let one = create_book(
        "one",
        json!([{ "name": "Sheet1", "cells": [{ "ref": "A1", "value": 1 }] }]),
    );
    let err = apply(
        &one,
        &tmp("del-one"),
        json!([{ "op": "deleteSheet", "id": "sheet:Sheet1" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

#[test]
fn refuse_delete_when_formula_refers() {
    let src = create_book(
        "fref",
        json!([
            { "name": "Sheet1", "cells": [{ "ref": "A1", "formula": "=Sheet2!A1" }] },
            { "name": "Sheet2", "cells": [{ "ref": "A1", "value": 1 }] }
        ]),
    );
    let err = apply(
        &src,
        &tmp("del-ref"),
        json!([{ "op": "deleteSheet", "id": "sheet:Sheet2" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::NotImplemented);
    assert_eq!(inspect(&src)["sheets"].as_array().unwrap().len(), 2);

    let err = apply(
        &src,
        &tmp("ren-ref"),
        json!([{ "op": "renameSheet", "id": "sheet:Sheet2", "name": "数据" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::NotImplemented);
}

#[test]
fn delete_sheet_on_chartsheet_is_invalid_id() {
    let src = create_book(
        "cs",
        json!([{ "name": "Sheet1", "cells": [{ "ref": "A1", "value": 1 }] }]),
    );
    let with_cs = tmp("cs-ins");
    apply(
        &src,
        &with_cs,
        json!([{
            "op": "insertChartSheet",
            "id": "xlsx:workbook",
            "name": "Chart1",
            "type": "col",
            "dataFrom": "A10",
            "series": [{ "name": "Q1", "values": [1, 2, 3] }]
        }]),
    )
    .unwrap();
    let err = apply(
        &with_cs,
        &tmp("cs-del"),
        json!([{ "op": "deleteSheet", "id": "sheet:Chart1" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
}

#[test]
fn macrosheet_kind_and_refuse_delete() {
    let src = create_book(
        "macro",
        json!([
            { "name": "Sheet1", "cells": [{ "ref": "A1", "value": 1 }] },
            { "name": "Macro1", "cells": [{ "ref": "A1", "value": 2 }] }
        ]),
    );
    rewrite_part(&src, "xl/_rels/workbook.xml.rels", |bytes| {
        let s = String::from_utf8(bytes).unwrap();
        let s = s.replace(
            "http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet",
            "http://schemas.microsoft.com/office/2006/relationships/xlMacrosheet",
        );
        // 只改 Macro1 那条：上面会把两条都改掉。按 Target 再改回 Sheet1。
        let s = s.replacen(
            "http://schemas.microsoft.com/office/2006/relationships/xlMacrosheet",
            "http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet",
            1,
        );
        s.into_bytes()
    });
    let tree = inspect(&src);
    assert_eq!(tree["sheets"][0]["kind"], "worksheet");
    assert_eq!(tree["sheets"][1]["kind"], "macrosheet");
    assert_eq!(tree["sheets"][1]["hidden"], false);
    assert!(tree["sheets"][1].get("pictureCount").is_none());
    let err = apply(
        &src,
        &tmp("macro-del"),
        json!([{ "op": "deleteSheet", "id": "sheet:Macro1" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::NotImplemented);
}

#[test]
fn insert_sheet_on_docx_is_invalid_field() {
    let path = tmp("docx").with_extension("docx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "document": { "paragraphs": [{ "text": "hi" }] }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let err = apply(
        &path,
        &tmp("docx-out").with_extension("docx"),
        json!([{ "op": "insertSheet", "id": "xlsx:workbook", "name": "Sheet3" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}
