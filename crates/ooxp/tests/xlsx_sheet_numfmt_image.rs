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

#[test]
fn general_error_aligns_center_text_lookalike_stays_left() {
    fn book(tag: &str) -> PathBuf {
        let path = tmp(tag);
        run_ok(json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "workbook": {
                "sheets": [{
                    "name": "Sheet1",
                    "cells": [{ "ref": "A1", "value": 1 }]
                }]
            }
        }));
        run_ok(json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": path.to_string_lossy(),
            "ops": [
                { "op": "setColWidth", "id": "sheet:Sheet1", "col": "A", "width": 18.0 }
            ]
        }));
        path
    }

    fn rewrite_keep_style(path: &Path, after_open: &str) {
        use std::io::{Cursor, Read, Write};
        use zip::write::SimpleFileOptions;
        use zip::{CompressionMethod, ZipArchive, ZipWriter};
        let bytes = std::fs::read(path).unwrap();
        let mut zip_in = ZipArchive::new(Cursor::new(bytes)).unwrap();
        let mut out = Cursor::new(Vec::new());
        {
            let mut zip_out = ZipWriter::new(&mut out);
            let opt = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
            for i in 0..zip_in.len() {
                let mut file = zip_in.by_index(i).unwrap();
                let name = file.name().to_string();
                let mut data = Vec::new();
                file.read_to_end(&mut data).unwrap();
                if name == "xl/worksheets/sheet1.xml" {
                    let xml = String::from_utf8(data).unwrap();
                    let start = xml.find(r#"<c r="A1""#).expect("A1 cell");
                    let rest = &xml[start..];
                    let end_rel = rest.find("</c>").expect("</c>") + 4;
                    let old = &xml[start..start + end_rel];
                    let style = if let Some(s) = old.split(r#" s=""#).nth(1) {
                        let n = s.split('"').next().unwrap();
                        format!(r#" s="{n}""#)
                    } else {
                        String::new()
                    };
                    let next = format!(
                        r#"{}<c r="A1"{}{}</c>{}"#,
                        &xml[..start],
                        style,
                        after_open,
                        &xml[start + end_rel..]
                    );
                    data = next.into_bytes();
                }
                zip_out.start_file(&name, opt).unwrap();
                zip_out.write_all(&data).unwrap();
            }
        }
        std::fs::write(path, out.into_inner()).unwrap();
    }

    let general = book("error-general");
    rewrite_keep_style(&general, r#" t="e"><v>#DIV/0!</v>"#);

    let center = book("error-center");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": center.to_string_lossy(),
        "ops": [{ "op": "setCellStyle", "id": "cell:Sheet1!A1", "align": { "horizontal": "center" } }]
    }));
    rewrite_keep_style(&center, r#" t="e"><v>#DIV/0!</v>"#);

    let left = book("error-left");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": left.to_string_lossy(),
        "ops": [{ "op": "setCellStyle", "id": "cell:Sheet1!A1", "align": { "horizontal": "left" } }]
    }));
    rewrite_keep_style(&left, r#" t="e"><v>#DIV/0!</v>"#);

    let text = book("error-text");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": text.to_string_lossy(),
        "ops": [{ "op": "setCell", "id": "cell:Sheet1!A1", "value": "#DIV/0!" }]
    }));

    let png_general = export_png(&general);
    let png_center = export_png(&center);
    let png_left = export_png(&left);
    let png_text = export_png(&text);
    assert_eq!(png_general, png_center, "常规错误值应与显式居中同像素");
    assert_ne!(png_general, png_left, "常规错误值不应与显式左对齐同像素");
    assert_ne!(png_text, png_center, "共享弦同形文字不应居中");
}

#[test]
fn cached_div0_error_exports_png() {
    let path = tmp("error-div0");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "workbook": {
            "sheets": [{
                "name": "Sheet1",
                "cells": [{ "ref": "A1", "value": 1 }]
            }]
        }
    }));
    rewrite_a1_cell(&path, r#"<c r="A1" t="e"><v>#DIV/0!</v></c>"#);
    let png = export_png(&path);
    assert!(png.starts_with(&[0x89, b'P', b'N', b'G']));
}

#[test]
fn unknown_spill_error_still_not_implemented() {
    let path = tmp("error-spill");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "workbook": {
            "sheets": [{
                "name": "Sheet1",
                "cells": [{ "ref": "A1", "value": 1 }]
            }]
        }
    }));
    rewrite_a1_cell(&path, r#"<c r="A1" t="e"><v>#SPILL!</v></c>"#);
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
fn text_literal_div0_still_exports_as_text() {
    let path = tmp("error-literal");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "workbook": {
            "sheets": [{
                "name": "Sheet1",
                "cells": [{ "ref": "A1", "value": "#DIV/0!" }]
            }]
        }
    }));
    let png = export_png(&path);
    assert!(png.starts_with(&[0x89, b'P', b'N', b'G']));
}

#[test]
fn wrap_and_indent_exports_png() {
    let path = tmp("wrap-indent");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "workbook": {
            "sheets": [{
                "name": "Sheet1",
                "cells": [
                    { "ref": "A1", "value": "one two three four five" },
                    { "ref": "B1", "value": "indent" },
                    { "ref": "A2", "value": "line1\nline2" },
                    { "ref": "B2", "value": "supercalifragilistic" }
                ]
            }]
        }
    }));
    run_ok(json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": path.to_string_lossy(),
        "ops": [
            { "op": "setCellStyle", "id": "cell:Sheet1!A1", "align": { "wrap": true, "horizontal": "left", "vertical": "top" } },
            { "op": "setCellStyle", "id": "cell:Sheet1!B1", "align": { "wrap": true, "indent": 2, "horizontal": "left", "vertical": "top" } },
            { "op": "setCellStyle", "id": "cell:Sheet1!A2", "align": { "wrap": true, "horizontal": "left", "vertical": "top" } },
            { "op": "setCellStyle", "id": "cell:Sheet1!B2", "align": { "wrap": true, "horizontal": "left", "vertical": "top" } },
            { "op": "setRowHeight", "id": "sheet:Sheet1", "row": 1, "height": 45.0 },
            { "op": "setRowHeight", "id": "sheet:Sheet1", "row": 2, "height": 45.0 },
            { "op": "setColWidth", "id": "sheet:Sheet1", "col": "A", "width": 10.625 },
            { "op": "setColWidth", "id": "sheet:Sheet1", "col": "B", "width": 10.625 }
        ]
    }));
    let png = export_png(&path);
    assert!(png.starts_with(&[0x89, b'P', b'N', b'G']));
    assert!(png.len() > 100);
}

fn rewrite_a1_cell(path: &Path, cell_xml: &str) {
    use std::io::{Cursor, Read, Write};
    use zip::write::SimpleFileOptions;
    use zip::{CompressionMethod, ZipArchive, ZipWriter};
    let bytes = std::fs::read(path).unwrap();
    let mut zip_in = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut out = Cursor::new(Vec::new());
    {
        let mut zip_out = ZipWriter::new(&mut out);
        let opt = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        for i in 0..zip_in.len() {
            let mut file = zip_in.by_index(i).unwrap();
            let name = file.name().to_string();
            let mut data = Vec::new();
            file.read_to_end(&mut data).unwrap();
            if name == "xl/worksheets/sheet1.xml" {
                let xml = String::from_utf8(data).unwrap();
                let start = xml.find(r#"<c r="A1""#).expect("A1 cell");
                let rest = &xml[start..];
                let end_rel = rest.find("</c>").expect("</c>") + 4;
                let mut next = String::new();
                next.push_str(&xml[..start]);
                next.push_str(cell_xml);
                next.push_str(&xml[start + end_rel..]);
                data = next.into_bytes();
            }
            zip_out.start_file(&name, opt).unwrap();
            zip_out.write_all(&data).unwrap();
        }
    }
    std::fs::write(path, out.into_inner()).unwrap();
}
