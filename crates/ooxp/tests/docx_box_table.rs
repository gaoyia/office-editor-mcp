//! docx 文本框里的表：inspect 计数与名单、apply 结构与外观、拒写注内。

use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-docx-box-table");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn tmp(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}.docx"))
}

fn create_doc(tag: &str, document: Value) -> PathBuf {
    let path = tmp(tag);
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "format": "docx",
            "document": document
        })
        .to_string(),
        None,
    )
    .unwrap();
    path
}

fn inspect(path: &Path, scope: Value) -> Value {
    let RunOutput::Inspect(tree) = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": path.to_string_lossy(),
            "scope": scope
        })
        .to_string(),
        None,
    )
    .unwrap() else {
        panic!("inspect");
    };
    tree
}

fn apply(src: &Path, out: &Path, ops: Value) {
    run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": src.to_string_lossy(),
            "output": out.to_string_lossy(),
            "ops": ops
        })
        .to_string(),
        None,
    )
    .unwrap();
}

fn apply_err(src: &Path, ops: Value) -> ooxp::Error {
    let out = tmp("should-not");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": src.to_string_lossy(),
            "output": out.to_string_lossy(),
            "ops": ops
        })
        .to_string(),
        None,
    )
    .unwrap_err()
}

fn object_keys(v: &Value) -> Vec<&str> {
    v.as_object().unwrap().keys().map(String::as_str).collect()
}

fn box_node(tree: &Value) -> &Value {
    &tree["paragraphs"][0]["textBoxes"][0]
}

fn unzip_document(path: &Path) -> String {
    let file = std::fs::File::open(path).unwrap();
    let mut zip = zip::ZipArchive::new(file).unwrap();
    let mut part = zip.by_name("word/document.xml").unwrap();
    let mut s = String::new();
    part.read_to_string(&mut s).unwrap();
    s
}

#[test]
fn box_table_insert_inspect_and_format() {
    let src = create_doc(
        "src",
        json!({
            "paragraphs": [{
                "text": "外",
                "textBoxes": [{ "text": "框" }]
            }]
        }),
    );
    let tree0 = inspect(
        &src,
        json!({ "from": 1, "maxParas": 10, "textBoxes": true }),
    );
    let keys = object_keys(box_node(&tree0));
    assert!(keys.contains(&"tableCount"), "{keys:?}");
    assert_eq!(box_node(&tree0)["tableCount"], 0);
    assert!(box_node(&tree0).get("tables").is_none());
    let box_id = box_node(&tree0)["id"].as_str().unwrap().to_string();
    assert!(box_id.contains("/txbx:"));
    let root_tables = tree0["tableCount"].as_u64().unwrap();

    let out = tmp("ins");
    apply(
        &src,
        &out,
        json!([{
            "op": "insertTable",
            "id": box_id,
            "rows": 2,
            "cols": 2,
            "cells": [["A1", "B1"], ["A2", "B2"]]
        }]),
    );
    let listed = inspect(
        &out,
        json!({
            "from": 1,
            "maxParas": 10,
            "textBoxes": true,
            "tables": true
        }),
    );
    assert_eq!(listed["tableCount"], root_tables);
    assert!(listed.get("tables").unwrap().as_array().unwrap().is_empty());
    let b = box_node(&listed);
    assert_eq!(b["tableCount"], 1);
    let tid = b["tables"][0]["id"].as_str().unwrap();
    assert!(tid.starts_with(&format!("{box_id}/table:")), "{tid}");
    assert_eq!(b["tables"][0]["rowCount"], 2);
    assert_eq!(b["tables"][0]["colCount"], 2);
    let cell_para = b["tables"][0]["rows"][0]["cells"][0]["paragraphs"][0]["id"]
        .as_str()
        .unwrap();
    assert!(
        cell_para.starts_with(&format!("{box_id}/para:")),
        "{cell_para}"
    );

    let fmt = tmp("fmt");
    apply(
        &out,
        &fmt,
        json!([
            { "op": "insertRow", "id": tid },
            {
                "op": "setCellFormat",
                "id": format!("{tid}/r1c1"),
                "fill": "FFFF00"
            }
        ]),
    );
    let after = inspect(
        &fmt,
        json!({
            "from": 1,
            "maxParas": 10,
            "textBoxes": true,
            "tables": true
        }),
    );
    let t = &box_node(&after)["tables"][0];
    assert_eq!(t["rowCount"], 3);
    assert_eq!(t["rows"][0]["cells"][0]["fill"], "FFFF00");
    let xml = unzip_document(&fmt);
    assert!(xml.contains("<w:tbl>"));
    assert!(xml.contains("FFFF00"));
}

#[test]
fn box_table_nested_and_style() {
    let src = create_doc(
        "nest",
        json!({
            "paragraphs": [{
                "text": "外",
                "textBoxes": [{ "text": "框" }]
            }]
        }),
    );
    let box_id = box_node(&inspect(
        &src,
        json!({ "from": 1, "maxParas": 10, "textBoxes": true }),
    ))["id"]
        .as_str()
        .unwrap()
        .to_string();
    let with_tbl = tmp("outer");
    apply(
        &src,
        &with_tbl,
        json!([{
            "op": "insertTable",
            "id": box_id,
            "rows": 1,
            "cols": 1,
            "cells": [["格"]]
        }]),
    );
    let cell = format!("{box_id}/table:1/r1c1");
    let nested = tmp("nested");
    apply(
        &with_tbl,
        &nested,
        json!([{
            "op": "insertTable",
            "id": cell,
            "rows": 1,
            "cols": 1,
            "cells": [["内"]]
        }]),
    );
    let tree = inspect(
        &nested,
        json!({
            "from": 1,
            "maxParas": 10,
            "textBoxes": true,
            "tables": true
        }),
    );
    let tables = box_node(&tree)["tables"].as_array().unwrap();
    assert_eq!(tables.len(), 2);
    assert_eq!(tables[0]["id"], format!("{box_id}/table:1"));
    assert_eq!(tables[1]["id"], format!("{box_id}/table:2"));
    assert_eq!(tables[1]["nested"], true);
    let xml = unzip_document(&nested);
    assert!(xml.contains("</w:tbl><w:p"), "嵌套表后必须跟段落：{xml}");
    let cell_text = xml.find(">格<").expect("外层格文本");
    let nest_tbl = xml
        .find("<w:tblGrid><w:gridCol w:w=\"1440\"/></w:tblGrid>")
        .expect("nested grid");
    assert!(
        nest_tbl < cell_text,
        "嵌套表应在原有单元格文本之前，否则 Word 框内会 Repair"
    );

    let mid = tmp("style-ins");
    apply(
        &nested,
        &mid,
        json!([{
            "op": "insertStyle",
            "id": "docx:document",
            "styleId": "ooxpBoxTbl",
            "type": "table",
            "name": "框内表"
        }]),
    );
    let styled = tmp("styled");
    apply(
        &mid,
        &styled,
        json!([{
            "op": "setTableStyle",
            "id": format!("{box_id}/table:1"),
            "style": "style:ooxpBoxTbl",
            "look": { "bandedRows": true }
        }]),
    );
    let tree_s = inspect(
        &styled,
        json!({
            "from": 1,
            "maxParas": 10,
            "textBoxes": true,
            "tables": true
        }),
    );
    let t0 = &box_node(&tree_s)["tables"][0];
    assert_eq!(t0["style"], "ooxpBoxTbl");
    assert_eq!(t0["look"]["bandedRows"], true);
}

#[test]
fn box_table_rejects_note_and_keeps_xlsx() {
    let src = create_doc(
        "fn",
        json!({
            "paragraphs": [{
                "text": "正文",
                "footnote": "注文"
            }]
        }),
    );
    let tree = inspect(&src, json!({ "from": 1, "maxParas": 10, "notes": true }));
    let fn_para = tree["footnotes"][0]["paragraphs"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let err = apply_err(
        &src,
        json!([{
            "op": "insertTable",
            "id": fn_para,
            "rows": 1,
            "cols": 1,
            "cells": [["x"]]
        }]),
    );
    assert_eq!(err.code(), ErrorCode::InvalidId);

    let xlsx = tmp_dir().join("t.xlsx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": xlsx.to_string_lossy(),
            "workbook": { "sheets": [{ "name": "Sheet1" }] }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let err_x = run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": xlsx.to_string_lossy(),
            "output": tmp_dir().join("t-out.xlsx").to_string_lossy(),
            "ops": [{ "op": "insertTable", "id": "para:1/txbx:1", "rows": 1, "cols": 1 }]
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err_x.code(), ErrorCode::InvalidField);
}
