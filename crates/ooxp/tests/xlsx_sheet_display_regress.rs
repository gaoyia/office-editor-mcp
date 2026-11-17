//! 日期时间窄列 ###、公式错误绿三角、wrap/indent 空格缩进的出图回归。
//! 不把单一官方截图尺寸或 DPI 写死；绿标只覆盖 t=e 且尊重 ignoredErrors。

use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, RunOutput};
use serde_json::json;

static SEQ: AtomicU32 = AtomicU32::new(0);
const GREEN: [u8; 3] = [0x00, 0x80, 0x00];

fn tmp(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-xlsx-display-regress");
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
    std::fs::read(&out).unwrap()
}

fn png_rgb(bytes: &[u8]) -> (u32, u32, Vec<[u8; 3]>) {
    let decoder = png::Decoder::new(Cursor::new(bytes));
    let mut reader = decoder.read_info().unwrap();
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).unwrap();
    let src = &buf[..info.buffer_size()];
    let mut pixels = Vec::with_capacity((info.width * info.height) as usize);
    match info.color_type {
        png::ColorType::Rgb => {
            for chunk in src.chunks(3) {
                pixels.push([chunk[0], chunk[1], chunk[2]]);
            }
        }
        png::ColorType::Rgba => {
            for chunk in src.chunks(4) {
                pixels.push([chunk[0], chunk[1], chunk[2]]);
            }
        }
        other => panic!("unexpected color type {other:?}"),
    }
    (info.width, info.height, pixels)
}

fn count_rgb(pixels: &[[u8; 3]], rgb: [u8; 3]) -> usize {
    pixels.iter().filter(|px| **px == rgb).count()
}

fn rewrite_sheet1(path: &Path, edit: impl Fn(String) -> String) {
    use std::io::{Read, Write};
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
                data = edit(String::from_utf8(data).unwrap()).into_bytes();
            }
            zip_out.start_file(&name, opt).unwrap();
            zip_out.write_all(&data).unwrap();
        }
    }
    std::fs::write(path, out.into_inner()).unwrap();
}

fn replace_cell(xml: &str, refer: &str, cell_xml: &str) -> String {
    let needle = format!("<c r=\"{refer}\"");
    let start = xml
        .find(&needle)
        .unwrap_or_else(|| panic!("missing {needle}"));
    let rest = &xml[start..];
    let end_rel = rest.find("</c>").expect("</c>") + 4;
    format!("{}{}{}", &xml[..start], cell_xml, &xml[start + end_rel..])
}

fn insert_before_end(xml: &str, fragment: &str) -> String {
    let key = "</worksheet>";
    let start = xml.rfind(key).expect("</worksheet>");
    format!("{}{}{}", &xml[..start], fragment, &xml[start..])
}

#[test]
fn formula_eval_error_paints_green_triangle_but_not_na_or_text() {
    let path = tmp("eval-error-marks");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "workbook": {
            "sheets": [{
                "name": "Sheet1",
                "cells": [
                    { "ref": "A1", "value": 1 },
                    { "ref": "B1", "value": 1 },
                    { "ref": "C1", "value": "#DIV/0!" },
                    { "ref": "D1", "value": 1 }
                ]
            }]
        }
    }));
    rewrite_sheet1(&path, |xml| {
        let xml = replace_cell(
            &xml,
            "A1",
            r#"<c r="A1" t="e"><f>1/0</f><v>#DIV/0!</v></c>"#,
        );
        let xml = replace_cell(&xml, "B1", r#"<c r="B1" t="e"><f>NA()</f><v>#N/A</v></c>"#);
        let xml = replace_cell(
            &xml,
            "D1",
            r#"<c r="D1" t="e"><f>1+"x"</f><v>#VALUE!</v></c>"#,
        );
        insert_before_end(
            &xml,
            r#"<ignoredErrors><ignoredError sqref="D1" evalError="1"/></ignoredErrors>"#,
        )
    });
    let png = export_png(&path);
    let (_w, _h, pixels) = png_rgb(&png);
    assert_eq!(
        count_rgb(&pixels, GREEN),
        21,
        "only A1 should have the 6px triangle"
    );
}

#[test]
fn narrow_datetime_overflows_to_hashes_not_general_numbers() {
    let path = tmp("datetime-hash");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "workbook": {
            "sheets": [{
                "name": "Sheet1",
                "cells": [
                    { "ref": "A1", "value": 44941.6046875 },
                    { "ref": "B1", "value": 44941.6046875 },
                    { "ref": "C1", "value": 12345.0 }
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
            { "op": "setCellStyle", "id": "cell:Sheet1!B1", "numFmt": "yyyy/mm/dd" },
            { "op": "setColWidth", "id": "sheet:Sheet1", "col": "A", "width": 4.0 },
            { "op": "setColWidth", "id": "sheet:Sheet1", "col": "B", "width": 20.0 },
            { "op": "setColWidth", "id": "sheet:Sheet1", "col": "C", "width": 4.0 }
        ]
    }));
    let png = export_png(&path);
    let (w, h, pixels) = png_rgb(&png);
    assert_eq!(count_rgb(&pixels, GREEN), 0);
    assert!(w > 10 && h > 5, "png {w}x{h}");
    // 列宽 4 : 20 : 4，按比例切开，比较墨迹掩码：窄日期应是 ###，与宽日期和同窄列 General 数值都不同。
    let a_end = ((w as f64) * 4.0 / 28.0).round() as u32;
    let b_end = ((w as f64) * 24.0 / 28.0).round() as u32;
    let mask = |x0: u32, x1: u32| -> Vec<u8> {
        let mut out = Vec::new();
        for y in 0..h {
            for x in x0..x1.min(w) {
                let px = pixels[(y * w + x) as usize];
                let dark = px[0] < 80 && px[1] < 80 && px[2] < 80;
                out.push(u8::from(dark));
            }
        }
        out
    };
    let a = mask(0, a_end);
    let b = mask(a_end, b_end);
    let c = mask(b_end, w);
    assert!(a.contains(&1), "narrow date should still paint hashes");
    assert!(b.contains(&1), "wide date should paint the date text");
    assert!(c.contains(&1), "narrow general number should still paint");
    assert_ne!(
        &a, &b,
        "narrow datetime must not be a clipped copy of the wide date"
    );
    assert_ne!(&a, &c, "datetime ### must not apply to a general number");
}

#[test]
fn indent_without_wrap_keeps_full_word_not_clip_to_prefix() {
    let path = tmp("indent-clip");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "workbook": {
            "sheets": [{
                "name": "Sheet1",
                "cells": [
                    { "ref": "A1", "value": "indent" },
                    { "ref": "B1", "value": "inder" }
                ]
            }]
        }
    }));
    run_ok(json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": path.to_string_lossy(),
        "ops": [
            { "op": "setCellStyle", "id": "cell:Sheet1!A1", "align": { "indent": 2, "horizontal": "left", "vertical": "top" } },
            { "op": "setCellStyle", "id": "cell:Sheet1!B1", "align": { "indent": 2, "horizontal": "left", "vertical": "top" } },
            { "op": "setColWidth", "id": "sheet:Sheet1", "col": "A", "width": 10.625 },
            { "op": "setColWidth", "id": "sheet:Sheet1", "col": "B", "width": 10.625 }
        ]
    }));
    let png = export_png(&path);
    let (w, h, pixels) = png_rgb(&png);
    let mid = w / 2;
    let width_of = |x0: u32, x1: u32| -> u32 {
        let mut cols = vec![false; (x1 - x0) as usize];
        for y in 0..h {
            for x in x0..x1 {
                let px = pixels[(y * w + x) as usize];
                if px[0] < 80 && px[1] < 80 && px[2] < 80 {
                    cols[(x - x0) as usize] = true;
                }
            }
        }
        let first = cols.iter().position(|c| *c);
        let last = cols.iter().rposition(|c| *c);
        match (first, last) {
            (Some(a), Some(b)) => (b - a + 1) as u32,
            _ => 0,
        }
    };
    let a_w = width_of(0, mid);
    let b_w = width_of(mid, w);
    assert!(b_w > 0, "control 'inder' should be visible");
    assert!(
        a_w > b_w,
        "indent text should be wider than the clipped prefix 'inder' (a={a_w} b={b_w} png={w}x{h})"
    );
}
