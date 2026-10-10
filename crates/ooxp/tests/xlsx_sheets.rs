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
    inspect_with(path, None)
}

fn inspect_with(path: &Path, scope: Option<serde_json::Value>) -> serde_json::Value {
    let mut req = json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": path.to_string_lossy()
    });
    if let Some(scope) = scope {
        req["scope"] = scope;
    }
    let RunOutput::Inspect(tree) = run(&req.to_string(), None).unwrap() else {
        panic!("inspect 应返回树");
    };
    tree
}

fn inspect_err(path: &Path, scope: serde_json::Value) -> ooxp::Error {
    run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": path.to_string_lossy(),
            "scope": scope,
        })
        .to_string(),
        None,
    )
    .unwrap_err()
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

fn rewrite_package(
    path: &Path,
    mut map: impl FnMut(&str, Vec<u8>) -> Option<(String, Vec<u8>)>,
    extra: &[(&str, Vec<u8>)],
) {
    let bytes = std::fs::read(path).unwrap();
    let mut zip_in = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut out = Cursor::new(Vec::new());
    {
        let mut zip_out = ZipWriter::new(&mut out);
        let opt = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        for i in 0..zip_in.len() {
            let mut file = zip_in.by_index(i).unwrap();
            let n = file.name().replace('\\', "/");
            let mut data = Vec::new();
            file.read_to_end(&mut data).unwrap();
            if let Some((out_name, data)) = map(&n, data) {
                zip_out.start_file(&out_name, opt).unwrap();
                zip_out.write_all(&data).unwrap();
            }
        }
        for (name, data) in extra {
            zip_out.start_file(*name, opt).unwrap();
            zip_out.write_all(data).unwrap();
        }
        zip_out.finish().unwrap();
    }
    std::fs::write(path, out.into_inner()).unwrap();
}

const REL_WORKSHEET: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet";
const REL_DIALOGSHEET: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/dialogsheet";
const REL_MACROSHEET: &str = "http://schemas.microsoft.com/office/2006/relationships/xlMacrosheet";
const REL_INTL_MACROSHEET: &str =
    "http://schemas.microsoft.com/office/2006/relationships/xlIntlMacrosheet";
const CT_WORKSHEET: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml";
const CT_DIALOGSHEET: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.dialogsheet+xml";
const CT_MACROSHEET: &str = "application/vnd.ms-excel.macrosheet+xml";
const CT_INTL_MACROSHEET: &str = "application/vnd.ms-excel.intlmacrosheet+xml";

const DIALOG_XML: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<dialogsheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <sheetViews>
    <sheetView workbookViewId="0" showRowColHeaders="0"/>
  </sheetViews>
  <sheetFormatPr defaultRowHeight="6.75"/>
  <headerFooter>
    <oddHeader>&amp;CDlg</oddHeader>
  </headerFooter>
  <pageMargins left="0.7" right="0.7" top="0.75" bottom="0.75" header="0.3" footer="0.3"/>
</dialogsheet>"#;

const MACRO_XML: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<xm:macrosheet xmlns:xm="http://schemas.microsoft.com/office/excel/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <sheetViews>
    <sheetView workbookViewId="0" showFormulas="1"/>
  </sheetViews>
  <sheetFormatPr defaultRowHeight="15"/>
  <sheetData/>
  <pageMargins left="0.7" right="0.7" top="0.75" bottom="0.75" header="0.3" footer="0.3"/>
</xm:macrosheet>"#;

fn inject_special_sheet(
    path: &Path,
    src_part: &str,
    dst_part: &str,
    rel_type: &str,
    content_type: &str,
    xml: &str,
) {
    let src_target = src_part.strip_prefix("xl/").unwrap_or(src_part);
    let dst_target = dst_part.strip_prefix("xl/").unwrap_or(dst_part);
    let src_override = format!("/{src_part}");
    let dst_override = format!("/{dst_part}");
    let xml_bytes = xml.as_bytes().to_vec();
    rewrite_package(
        path,
        |name, data| {
            if name == src_part {
                return None;
            }
            if name == "xl/_rels/workbook.xml.rels" {
                let s = String::from_utf8(data).unwrap();
                let s = s.replace(
                    &format!("Target=\"{src_target}\""),
                    &format!("Target=\"{dst_target}\""),
                );
                let s = s.replace(
                    &format!("Type=\"{REL_WORKSHEET}\" Target=\"{dst_target}\""),
                    &format!("Type=\"{rel_type}\" Target=\"{dst_target}\""),
                );
                return Some((name.to_string(), s.into_bytes()));
            }
            if name == "[Content_Types].xml" {
                let s = String::from_utf8(data).unwrap();
                let s = s.replace(
                    &format!("PartName=\"{src_override}\" ContentType=\"{CT_WORKSHEET}\""),
                    &format!("PartName=\"{dst_override}\" ContentType=\"{content_type}\""),
                );
                return Some((name.to_string(), s.into_bytes()));
            }
            Some((name.to_string(), data))
        },
        &[(dst_part, xml_bytes)],
    );
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
            "textBoxCount",
            "mathCount",
            "shapeCount",
            "connectorCount",
            "smartArtCount",
            "dataValidationCount",
            "conditionalFormatCount",
            "tableCount",
            "pivotCount",
            "commentCount",
            "hyperlinkCount",
            "sparklineCount",
            "oleObjectCount",
            "model3dCount",
            "slicerCount",
            "protectedRangeCount",
            "hasHeaderFooter",
            "print",
            "view",
            "autoFilter",
            "sortState",
            "outline",
            "sheetProtection"
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
fn dialog_and_macrosheet_kind_and_refuse_write() {
    let src = create_book(
        "dlg-macro",
        json!([
            { "name": "Sheet1", "cells": [{ "ref": "A1", "value": 1 }] },
            { "name": "Dialog1", "cells": [{ "ref": "A1", "value": 2 }] },
            { "name": "Macro1", "cells": [{ "ref": "A1", "value": 3 }] },
            { "name": "Intl1", "cells": [{ "ref": "A1", "value": 4 }] }
        ]),
    );
    inject_special_sheet(
        &src,
        "xl/worksheets/sheet2.xml",
        "xl/dialogsheets/sheet1.xml",
        REL_DIALOGSHEET,
        CT_DIALOGSHEET,
        DIALOG_XML,
    );
    inject_special_sheet(
        &src,
        "xl/worksheets/sheet3.xml",
        "xl/macrosheets/sheet1.xml",
        REL_MACROSHEET,
        CT_MACROSHEET,
        MACRO_XML,
    );
    inject_special_sheet(
        &src,
        "xl/worksheets/sheet4.xml",
        "xl/macrosheets/intlsheet1.xml",
        REL_INTL_MACROSHEET,
        CT_INTL_MACROSHEET,
        MACRO_XML,
    );

    let tree = inspect(&src);
    assert_eq!(tree["sheets"][0]["kind"], "worksheet");
    assert_eq!(tree["sheets"][1]["kind"], "dialogsheet");
    assert_eq!(tree["sheets"][1]["name"], "Dialog1");
    assert_eq!(tree["sheets"][1]["hidden"], false);
    assert_eq!(tree["sheets"][1]["hasHeaderFooter"], true);
    assert_eq!(
        object_keys(&tree["sheets"][1]),
        ["id", "name", "kind", "hidden", "hasHeaderFooter"]
    );
    assert_eq!(tree["sheets"][2]["kind"], "macrosheet");
    assert_eq!(tree["sheets"][2]["name"], "Macro1");
    assert_eq!(tree["sheets"][2]["hasHeaderFooter"], false);
    assert_eq!(
        object_keys(&tree["sheets"][2]),
        ["id", "name", "kind", "hidden", "hasHeaderFooter"]
    );
    assert_eq!(tree["sheets"][3]["kind"], "macrosheet");
    assert_eq!(tree["sheets"][3]["name"], "Intl1");
    assert_eq!(
        object_keys(&tree["sheets"][3]),
        ["id", "name", "kind", "hidden", "hasHeaderFooter"]
    );

    let hf = inspect_with(&src, Some(json!({ "headerFooter": true })));
    assert_eq!(
        object_keys(&hf["sheets"][1]),
        [
            "id",
            "name",
            "kind",
            "hidden",
            "hasHeaderFooter",
            "headerFooters"
        ]
    );
    assert_eq!(hf["sheets"][1]["headerFooters"][0]["text"], "&CDlg");
    assert!(hf["sheets"][1]["headerFooters"][0]
        .get("pictureCount")
        .is_none());
    assert_eq!(
        hf["sheets"][2]["headerFooters"].as_array().unwrap().len(),
        6
    );
    assert!(hf["sheets"][2]["headerFooters"][0]
        .get("pictureCount")
        .is_none());

    let err = inspect_err(&src, json!({ "sheet": "Dialog1", "range": "A1" }));
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert_eq!(err.field_path(), Some("scope.sheet"));
    let err = inspect_err(&src, json!({ "sheet": "Macro1", "range": "A1" }));
    assert_eq!(err.code(), ErrorCode::InvalidField);

    for (name, op) in [
        (
            "Dialog1",
            json!([{ "op": "deleteSheet", "id": "sheet:Dialog1" }]),
        ),
        (
            "Macro1",
            json!([{ "op": "deleteSheet", "id": "sheet:Macro1" }]),
        ),
        (
            "Intl1",
            json!([{ "op": "renameSheet", "id": "sheet:Intl1", "name": "X" }]),
        ),
        (
            "Dialog1",
            json!([{ "op": "setSheetHidden", "id": "sheet:Dialog1", "hidden": true }]),
        ),
        (
            "Macro1",
            json!([{ "op": "moveSheet", "id": "sheet:Macro1" }]),
        ),
    ] {
        let err = apply(&src, &tmp(&format!("refuse-{name}")), op).unwrap_err();
        assert_eq!(err.code(), ErrorCode::NotImplemented, "{name}");
    }
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
