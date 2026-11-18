//! 导出可选 dpi：省略或 96 与历史出图一致；其它整数 DPI 原生重算，不是拉伸。
//! exportSlideImage / exportPageImage 本切片不接受此字段。

use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::json;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp(tag: &str, ext: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-xlsx-export-dpi");
    std::fs::create_dir_all(&dir).unwrap();
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    dir.join(format!("{tag}-{n}.{ext}"))
}

fn run_ok(value: serde_json::Value) -> RunOutput {
    run(&value.to_string()).unwrap()
}

fn decode_png(bytes: &[u8]) -> (u32, u32, Vec<u8>) {
    let decoder = png::Decoder::new(Cursor::new(bytes));
    let mut reader = decoder.read_info().unwrap();
    let mut buf = vec![0; reader.output_buffer_size()];
    let frame = reader.next_frame(&mut buf).unwrap();
    (
        frame.width,
        frame.height,
        buf[..frame.buffer_size()].to_vec(),
    )
}

/// 仅非默认 DPI 应写入 pHYs（像素/米）。96 / 省略不得带此块，以免改既有字节。
fn png_pixel_dims(bytes: &[u8]) -> Option<png::PixelDimensions> {
    let decoder = png::Decoder::new(Cursor::new(bytes));
    let reader = decoder.read_info().unwrap();
    reader.info().pixel_dims
}

fn assert_no_phys(bytes: &[u8]) {
    assert!(png_pixel_dims(bytes).is_none(), "默认 96 DPI 不得写入 pHYs");
}

fn assert_phys_dpi(bytes: &[u8], dpi: u32) {
    let ppm = (f64::from(dpi) / 0.0254).round() as u32;
    let dims = png_pixel_dims(bytes).expect("非默认 DPI 应写入 pHYs");
    assert_eq!(dims.xppu, ppm);
    assert_eq!(dims.yppu, ppm);
    assert_eq!(dims.unit, png::Unit::Meter);
}

fn book_line(tag: &str) -> PathBuf {
    let path = tmp(tag, "xlsx");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "workbook": {
            "sheets": [{
                "name": "Sheet1",
                "cells": [
                    { "ref": "B1", "value": 1 },
                    { "ref": "C1", "value": 4 },
                    { "ref": "D1", "value": 2 }
                ]
            }]
        }
    }));
    let out = tmp("line", "xlsx");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": path.to_string_lossy(),
        "output": out.to_string_lossy(),
        "ops": [{
            "op": "insertSparkline",
            "id": "sheet:Sheet1",
            "ref": "A1",
            "formula": "B1:D1"
        }]
    }));
    out
}

fn book_chart(tag: &str) -> PathBuf {
    let path = tmp(tag, "xlsx");
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
    let out = tmp("chart", "xlsx");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": path.to_string_lossy(),
        "output": out.to_string_lossy(),
        "ops": [{
            "op": "insertChart",
            "id": "sheet:Sheet1",
            "type": "col",
            "from": "E2",
            "dataFrom": "A10",
            "title": "销量",
            "series": [{
                "name": "Q1",
                "categories": ["甲", "乙", "丙"],
                "values": [1, 2, 3]
            }]
        }]
    }));
    out
}

fn export_bytes(
    action: &str,
    src: &Path,
    id: &str,
    dpi: Option<i64>,
) -> Result<Vec<u8>, ooxp::Error> {
    let png = tmp(action, "png");
    if png.exists() {
        std::fs::remove_file(&png).unwrap();
    }
    let mut req = json!({
        "schemaVersion": 1,
        "action": action,
        "path": src.to_string_lossy(),
        "id": id,
        "output": png.to_string_lossy(),
    });
    if let Some(dpi) = dpi {
        req["dpi"] = json!(dpi);
    }
    let written = run(&req.to_string())?;
    let RunOutput::Written {
        path, content_type, ..
    } = written
    else {
        panic!("应写出文件");
    };
    assert_eq!(content_type.as_deref(), Some("image/png"));
    assert_eq!(path, png);
    Ok(std::fs::read(png).unwrap())
}

fn not_a_scaled_bitmap(src: &[u8], dst: &[u8], sw: u32, sh: u32, dw: u32, dh: u32) {
    // 最近邻放大 96 图不应等于原生重绘。
    let mut scaled = vec![0u8; (dw * dh * 3) as usize];
    for y in 0..dh {
        let sy = (y as u64 * sh as u64 / dh as u64) as u32;
        for x in 0..dw {
            let sx = (x as u64 * sw as u64 / dw as u64) as u32;
            let si = ((sy * sw + sx) * 3) as usize;
            let di = ((y * dw + x) * 3) as usize;
            scaled[di..di + 3].copy_from_slice(&src[si..si + 3]);
        }
    }
    assert_ne!(scaled, dst, "106 dpi 不得等于把 96 图拉伸后的位图");
}

#[test]
fn omitted_and_explicit_96_match_historical_spark_size() {
    let src = book_line("default");
    let before = std::fs::read(&src).unwrap();
    let omitted = export_bytes("exportSparklineImage", &src, "sheet:Sheet1/spark:1", None).unwrap();
    let explicit = export_bytes(
        "exportSparklineImage",
        &src,
        "sheet:Sheet1/spark:1",
        Some(96),
    )
    .unwrap();
    assert_eq!(std::fs::read(&src).unwrap(), before, "导出不得改源包");
    assert_eq!(omitted, explicit);
    let (w, h, _) = decode_png(&omitted);
    assert_eq!((w, h), (64, 20));
    assert_no_phys(&omitted);
    assert_no_phys(&explicit);
}

#[test]
fn sparkline_106_is_native_and_larger() {
    let src = book_line("dpi106");
    let before = std::fs::read(&src).unwrap();
    let at96 = export_bytes(
        "exportSparklineImage",
        &src,
        "sheet:Sheet1/spark:1",
        Some(96),
    )
    .unwrap();
    let at106 = export_bytes(
        "exportSparklineImage",
        &src,
        "sheet:Sheet1/spark:1",
        Some(106),
    )
    .unwrap();
    assert_eq!(std::fs::read(&src).unwrap(), before);
    let (w96, h96, rgb96) = decode_png(&at96);
    let (w106, h106, rgb106) = decode_png(&at106);
    assert_eq!((w96, h96), (64, 20));
    // Calibri 11 default at 106 is 72 (MDW remasured), not scale_px(64)=71.
    assert_eq!((w106, h106), (72, 22));
    assert_ne!(at96, at106);
    assert_no_phys(&at96);
    assert_phys_dpi(&at106, 106);
    not_a_scaled_bitmap(&rgb96, &rgb106, w96, h96, w106, h106);
}

#[test]
fn illegal_dpi_is_rejected_and_source_stays() {
    let src = book_line("bad");
    let before = std::fs::read(&src).unwrap();
    for (dpi, code) in [
        (serde_json::json!(0), ErrorCode::TruncatedLimit),
        (serde_json::json!(-1), ErrorCode::TruncatedLimit),
        (serde_json::json!(23), ErrorCode::TruncatedLimit),
        (serde_json::json!(601), ErrorCode::TruncatedLimit),
        (serde_json::json!(24.5), ErrorCode::InvalidField),
        (serde_json::json!("96"), ErrorCode::InvalidField),
        (serde_json::json!(true), ErrorCode::InvalidField),
        (serde_json::json!(null), ErrorCode::InvalidField),
        (serde_json::json!([]), ErrorCode::InvalidField),
        (serde_json::json!({}), ErrorCode::InvalidField),
    ] {
        let png = tmp("bad", "png");
        let err = run(&json!({
            "schemaVersion": 1,
            "action": "exportSparklineImage",
            "path": src.to_string_lossy(),
            "id": "sheet:Sheet1/spark:1",
            "output": png.to_string_lossy(),
            "dpi": dpi,
        })
        .to_string())
        .unwrap_err();
        assert_eq!(err.code(), code, "dpi={dpi}");
    }
    assert_eq!(std::fs::read(&src).unwrap(), before);
    // 边界合法
    let lo = export_bytes(
        "exportSparklineImage",
        &src,
        "sheet:Sheet1/spark:1",
        Some(24),
    )
    .unwrap();
    let (w, h, _) = decode_png(&lo);
    // 24 dpi remasures MDW (Calibri 11 -> 2 on this host) so default is 8*2+8=24,
    // not the old scale_px(64)=16. Height 5 is layout-round of the package's 15 pt row.
    // 24 dpi is a supported bound, not official CopyPicture gold.
    assert_eq!((w, h), (24, 5));
}

#[test]
fn chart_and_sheet_honor_dpi_and_keep_default() {
    let src = book_chart("chart");
    let before = std::fs::read(&src).unwrap();
    let omitted = export_bytes("exportChartImage", &src, "sheet:Sheet1/chart:1", None).unwrap();
    let explicit =
        export_bytes("exportChartImage", &src, "sheet:Sheet1/chart:1", Some(96)).unwrap();
    let at106 = export_bytes("exportChartImage", &src, "sheet:Sheet1/chart:1", Some(106)).unwrap();
    let sheet96 = export_bytes("exportSheetImage", &src, "sheet:Sheet1", Some(96)).unwrap();
    let sheet_omitted = export_bytes("exportSheetImage", &src, "sheet:Sheet1", None).unwrap();
    let sheet106 = export_bytes("exportSheetImage", &src, "sheet:Sheet1", Some(106)).unwrap();
    assert_eq!(std::fs::read(&src).unwrap(), before);
    assert_eq!(omitted, explicit);
    assert_eq!(sheet96, sheet_omitted);
    let (cw96, ch96, crgb96) = decode_png(&omitted);
    let (cw106, ch106, crgb106) = decode_png(&at106);
    assert_eq!((cw96, ch96), (576, 336));
    assert_eq!((cw106, ch106), (636, 371));
    assert_no_phys(&omitted);
    assert_no_phys(&explicit);
    assert_phys_dpi(&at106, 106);
    not_a_scaled_bitmap(&crgb96, &crgb106, cw96, ch96, cw106, ch106);
    let (sw96, sh96, _) = decode_png(&sheet96);
    let (sw106, sh106, _) = decode_png(&sheet106);
    assert!(
        sw106 > sw96 && sh106 > sh96,
        "sheet 96={sw96}x{sh96} 106={sw106}x{sh106}"
    );
    assert_no_phys(&sheet96);
    assert_phys_dpi(&sheet106, 106);
}

#[test]
fn slide_and_page_export_reject_dpi_field() {
    let src = book_line("notslide");
    let png = tmp("slide", "png");
    let err = run(&json!({
        "schemaVersion": 1,
        "action": "exportSlideImage",
        "path": src.to_string_lossy(),
        "id": "slide:1",
        "output": png.to_string_lossy(),
        "dpi": 106,
    })
    .to_string())
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    let err = run(&json!({
        "schemaVersion": 1,
        "action": "exportPageImage",
        "path": src.to_string_lossy(),
        "id": "page:1",
        "output": png.to_string_lossy(),
        "dpi": 106,
    })
    .to_string())
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}
#[cfg(windows)]
fn dark_ink_bbox(rgb: &[u8], w: u32, h: u32) -> (i32, i32, i32, i32) {
    let mut min_x = w as i32;
    let mut min_y = h as i32;
    let mut max_x = -1i32;
    let mut max_y = -1i32;
    for y in 0..h as i32 {
        for x in 0..w as i32 {
            let i = ((y as u32 * w + x as u32) * 3) as usize;
            // 网格是浅灰（通常 >= 200）。含抗锯齿的字形墨水，避免只量到近黑核、把浅边滤掉。
            if rgb[i] < 180 && rgb[i + 1] < 180 && rgb[i + 2] < 180 {
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }
    }
    assert!(max_x >= min_x && max_y >= min_y, "应画出文字");
    (min_x, min_y, max_x + 1, max_y + 1)
}

/// 单元格字形必须按请求 dpi 原生重画。曾经 GDI+ 位图分辨率写死 96，
/// 106 只放大了格子、Calibri 11 仍是 96 dpi 字高。
#[cfg(windows)]
#[test]
fn sheet_cell_text_at_106_dpi_is_native_not_96_glyphs() {
    let path = tmp("cell-text", "xlsx");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "workbook": {
            "sheets": [{
                "name": "Sheet1",
                "cells": [{ "ref": "A1", "value": "Hg" }]
            }]
        }
    }));
    let before = std::fs::read(&path).unwrap();
    let at96 = export_bytes("exportSheetImage", &path, "sheet:Sheet1", Some(96)).unwrap();
    let omitted = export_bytes("exportSheetImage", &path, "sheet:Sheet1", None).unwrap();
    let at106 = export_bytes("exportSheetImage", &path, "sheet:Sheet1", Some(106)).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), before, "导出不得改源包");
    assert_eq!(at96, omitted);
    assert_no_phys(&at96);
    assert_phys_dpi(&at106, 106);
    let (w96, h96, rgb96) = decode_png(&at96);
    let (w106, h106, rgb106) = decode_png(&at106);
    assert!(
        w106 > w96 && h106 > h96,
        "canvas 96={w96}x{h96} 106={w106}x{h106}"
    );
    let (_, y0_96, _, y1_96) = dark_ink_bbox(&rgb96, w96, h96);
    let (_, y0_106, _, y1_106) = dark_ink_bbox(&rgb106, w106, h106);
    let ink96 = y1_96 - y0_96;
    let ink106 = y1_106 - y0_106;
    assert!(
        ink106 > ink96,
        "106 dpi 字形应高于 96：96={ink96}px 106={ink106}px"
    );
}
