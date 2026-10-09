//! 单元格样式：inspect 摘要、apply 写 xf、未点名格子的 s 不变。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::json;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-xlsx-cellstyle");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn tmp(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}.xlsx"))
}

fn create_book(tag: &str) -> PathBuf {
    let path = tmp(tag);
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "workbook": {
                "sheets": [{
                    "name": "Sheet1",
                    "cells": [
                        { "ref": "A1", "value": 1.2 },
                        { "ref": "B1", "value": 3 }
                    ]
                }]
            }
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

fn inspect_cells(path: &Path) -> serde_json::Value {
    let RunOutput::Inspect(tree) = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": path.to_string_lossy(),
            "scope": { "sheet": "Sheet1", "range": "A1:B1" }
        })
        .to_string(),
        None,
    )
    .unwrap() else {
        panic!("inspect 应返回树");
    };
    tree["sheets"][0]["cells"].clone()
}

fn cell<'a>(cells: &'a serde_json::Value, id: &str) -> &'a serde_json::Value {
    cells
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == id)
        .unwrap()
}

fn part_text(path: &Path, name: &str) -> String {
    let pkg = {
        let bytes = std::fs::read(path).unwrap();
        // 通过再 inspect 不方便拿部件；用 zip。
        bytes
    };
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(pkg)).unwrap();
    let mut file = zip.by_name(name).unwrap();
    let mut s = String::new();
    std::io::Read::read_to_string(&mut file, &mut s).unwrap();
    s
}

fn xf_count(styles: &str) -> usize {
    let start = styles.find("<cellXfs").expect("cellXfs");
    let rest = &styles[start..];
    let end = rest.find("</cellXfs>").unwrap_or(rest.len());
    rest[..end].matches("<xf ").count() + rest[..end].matches("<xf>").count()
}

#[test]
fn inspect_default_num_fmt() {
    let path = create_book("ins");
    let cells = inspect_cells(&path);
    let a1 = cell(&cells, "cell:Sheet1!A1");
    assert_eq!(a1["numFmt"], "General");
    assert!(a1.get("font").is_none());
    assert!(a1.get("fill").is_none());
    assert_eq!(a1["value"], 1.2);
}

#[test]
fn apply_num_fmt_and_fill() {
    let src = create_book("fmt");
    let out = tmp("fmt-out");
    apply(
        &src,
        &out,
        json!([{
            "op": "setCellStyle",
            "id": "cell:Sheet1!A1",
            "numFmt": "0.00",
            "fill": { "pattern": "solid", "fg": { "rgb": "FFFF00" } }
        }]),
    )
    .unwrap();
    let cells = inspect_cells(&out);
    let a1 = cell(&cells, "cell:Sheet1!A1");
    assert_eq!(a1["numFmt"], "0.00");
    assert_eq!(a1["fill"]["pattern"], "solid");
    assert_eq!(a1["fill"]["fg"]["rgb"], "FFFFFF00");
    assert_eq!(a1["value"], 1.2);
    let b1 = cell(&cells, "cell:Sheet1!B1");
    assert_eq!(b1["numFmt"], "General");
    assert!(b1.get("fill").is_none());
}

#[test]
fn unnamed_cell_s_unchanged() {
    let src = create_book("keep-s");
    let sheet0 = part_text(&src, "xl/worksheets/sheet1.xml");
    assert!(
        !sheet0.contains(r#"r="B1" s=""#) && !sheet0.contains(r#"s="0" r="B1""#),
        "create 的 B1 不应带 s：{sheet0}"
    );
    let out = tmp("keep-s-out");
    apply(
        &src,
        &out,
        json!([{
            "op": "setCellStyle",
            "id": "cell:Sheet1!A1",
            "numFmt": "0.00"
        }]),
    )
    .unwrap();
    let sheet = part_text(&out, "xl/worksheets/sheet1.xml");
    let b = sheet
        .split("<c ")
        .find(|c| c.starts_with(r#"r="B1""#) || c.contains(r#"r="B1""#))
        .unwrap_or("");
    assert!(!b.contains(" s="), "未点名的 B1 不应被写上 s：{b}");
}

#[test]
fn reuse_same_xf() {
    let src = create_book("reuse");
    let once = tmp("reuse-1");
    apply(
        &src,
        &once,
        json!([{
            "op": "setCellStyle",
            "id": "cell:Sheet1!A1",
            "numFmt": "0.00",
            "fill": { "fg": { "rgb": "FFFFFF00" } }
        }]),
    )
    .unwrap();
    let n1 = xf_count(&part_text(&once, "xl/styles.xml"));
    let twice = tmp("reuse-2");
    apply(
        &once,
        &twice,
        json!([{
            "op": "setCellStyle",
            "id": "cell:Sheet1!B1",
            "numFmt": "0.00",
            "fill": { "fg": { "rgb": "FFFFFF00" } }
        }]),
    )
    .unwrap();
    let n2 = xf_count(&part_text(&twice, "xl/styles.xml"));
    assert_eq!(n1, n2, "相同定义应复用 xf，不要再追加");
    let sheet = part_text(&twice, "xl/worksheets/sheet1.xml");
    let a_s = sheet
        .split("<c ")
        .find(|c| c.contains(r#"r="A1""#))
        .and_then(|c| c.split("s=\"").nth(1))
        .map(|s| s.split('"').next().unwrap().to_string());
    let b_s = sheet
        .split("<c ")
        .find(|c| c.contains(r#"r="B1""#))
        .and_then(|c| c.split("s=\"").nth(1))
        .map(|s| s.split('"').next().unwrap().to_string());
    assert_eq!(a_s, b_s);
}

#[test]
fn set_cell_then_style_same_ops() {
    let src = create_book("order");
    let out = tmp("order-out");
    apply(
        &src,
        &out,
        json!([
            { "op": "setCell", "id": "cell:Sheet1!A1", "value": 9 },
            {
                "op": "setCellStyle",
                "id": "cell:Sheet1!A1",
                "numFmt": "0.00",
                "font": { "bold": true }
            }
        ]),
    )
    .unwrap();
    let cells = inspect_cells(&out);
    let a1 = cell(&cells, "cell:Sheet1!A1");
    assert_eq!(a1["value"], 9);
    assert_eq!(a1["numFmt"], "0.00");
    assert_eq!(a1["font"]["bold"], true);
}

#[test]
fn style_then_set_cell_keeps_s() {
    let src = create_book("keep-style");
    let out = tmp("keep-style-out");
    apply(
        &src,
        &out,
        json!([
            {
                "op": "setCellStyle",
                "id": "cell:Sheet1!A1",
                "numFmt": "0.00",
                "align": { "horizontal": "center", "wrap": true }
            },
            { "op": "setCell", "id": "cell:Sheet1!A1", "value": 8 }
        ]),
    )
    .unwrap();
    let cells = inspect_cells(&out);
    let a1 = cell(&cells, "cell:Sheet1!A1");
    assert_eq!(a1["value"], 8);
    assert_eq!(a1["numFmt"], "0.00");
    assert_eq!(a1["align"]["horizontal"], "center");
    assert_eq!(a1["align"]["wrap"], true);
}

#[test]
fn need_at_least_one_field() {
    let src = create_book("empty-op");
    let out = tmp("empty-op-out");
    let err = apply(
        &src,
        &out,
        json!([{ "op": "setCellStyle", "id": "cell:Sheet1!A1" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());
}

#[test]
fn not_for_docx() {
    let path = tmp_dir().join(format!(
        "docx-host-{}.docx",
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "format": "docx",
            "document": { "paragraphs": [{ "text": "hi" }] }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let out = tmp_dir().join(format!(
        "docx-style-out-{}.docx",
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    let err = apply(
        &path,
        &out,
        json!([{ "op": "setCellStyle", "id": "cell:Sheet1!A1", "numFmt": "0.00" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

#[test]
fn custom_num_fmt_and_theme_color() {
    let src = create_book("custom");
    let out = tmp("custom-out");
    apply(
        &src,
        &out,
        json!([{
            "op": "setCellStyle",
            "id": "cell:Sheet1!A1",
            "numFmt": "yyyy-mm-dd",
            "font": { "color": { "theme": 1 } },
            "border": { "style": "thin", "color": { "rgb": "FF000000" } }
        }]),
    )
    .unwrap();
    let cells = inspect_cells(&out);
    let a1 = cell(&cells, "cell:Sheet1!A1");
    assert_eq!(a1["numFmt"], "yyyy-mm-dd");
    assert_eq!(a1["font"]["color"]["theme"], 1);
    assert_eq!(a1["border"]["left"]["style"], "thin");
}

#[test]
fn diagonal_indent_rotation() {
    let src = create_book("diag");
    let out = tmp("diag-out");
    apply(
        &src,
        &out,
        json!([{
            "op": "setCellStyle",
            "id": "cell:Sheet1!A1",
            "border": { "diagonal": { "style": "thin" } },
            "align": { "indent": 2, "rotation": 45 }
        }]),
    )
    .unwrap();
    let cells = inspect_cells(&out);
    let a1 = cell(&cells, "cell:Sheet1!A1");
    assert_eq!(a1["border"]["diagonal"]["style"], "thin");
    assert_eq!(a1["border"]["diagonalUp"], true);
    assert_eq!(a1["border"]["diagonalDown"], true);
    assert_eq!(a1["align"]["indent"], 2);
    assert_eq!(a1["align"]["rotation"], 45);
    assert_eq!(a1["align"]["horizontal"], "left");
    let styles = part_text(&out, "xl/styles.xml");
    assert!(styles.contains("diagonalUp=\"1\""), "{styles}");
    assert!(styles.contains("textRotation=\"45\""), "{styles}");
    assert!(styles.contains("indent=\"2\""), "{styles}");
}
