//! 迷你图日期轴：新建后写入 dateAxis 与组上日期公式，以及读入已有部件导出。
//! 折线、非负柱形、盈亏都按日期比例定位。不把官方 CopyPicture 对照算作通过。

use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::json;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-xlsx-sparkline-date-axis");
    std::fs::create_dir_all(&dir).unwrap();
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    dir.join(format!("{tag}-{n}.xlsx"))
}

fn run_ok(value: serde_json::Value) -> RunOutput {
    run(&value.to_string()).unwrap()
}

fn create_book(tag: &str, dates: [f64; 3], values: [f64; 3]) -> PathBuf {
    let path = tmp(tag);
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "workbook": {
            "sheets": [{
                "name": "Sheet1",
                "cells": [
                    { "ref": "A1", "value": dates[0] },
                    { "ref": "B1", "value": dates[1] },
                    { "ref": "C1", "value": dates[2] },
                    { "ref": "A2", "value": values[0] },
                    { "ref": "B2", "value": values[1] },
                    { "ref": "C2", "value": values[2] }
                ]
            }]
        }
    }));
    path
}

fn apply(src: &Path, tag: &str, ops: serde_json::Value) -> PathBuf {
    let out = tmp(tag);
    run_ok(json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": out.to_string_lossy(),
        "ops": ops
    }));
    out
}

fn insert_spark_at(
    src: &Path,
    tag: &str,
    host: &str,
    formula: &str,
    kind: Option<&str>,
) -> PathBuf {
    let mut op = json!({
        "op": "insertSparkline",
        "id": "sheet:Sheet1",
        "ref": host,
        "formula": formula
    });
    if let Some(kind) = kind {
        op["type"] = json!(kind);
    }
    apply(src, tag, json!([op]))
}

fn insert_spark(src: &Path, tag: &str, kind: Option<&str>) -> PathBuf {
    insert_spark_at(src, tag, "E2", "A2:C2", kind)
}

fn with_grid_lines_off(xml: &str) -> String {
    if xml.contains(r#"showGridLines="0""#) || xml.contains(r#"showGridLines="false""#) {
        xml.to_string()
    } else if xml.contains(r#"<sheetView "#) {
        xml.replacen(r#"<sheetView "#, r#"<sheetView showGridLines="0" "#, 1)
    } else if xml.contains("<sheetView>") {
        xml.replacen("<sheetView>", r#"<sheetView showGridLines="0">"#, 1)
    } else if xml.contains("<sheetData") {
        xml.replacen(
            "<sheetData",
            r#"<sheetViews><sheetView workbookViewId="0" showGridLines="0"/></sheetViews><sheetData"#,
            1,
        )
    } else {
        panic!("cannot turn off grid lines in {xml}");
    }
}

fn with_uniform_host_metrics(xml: &str) -> String {
    let next = with_grid_lines_off(xml);
    if next.contains("sheetFormatPr") {
        next
    } else if next.contains("<sheetViews>") {
        next.replacen(
            "<sheetViews>",
            r#"<sheetFormatPr defaultRowHeight="15" customHeight="1"/><sheetViews>"#,
            1,
        )
    } else if next.contains("<sheetData") {
        next.replacen(
            "<sheetData",
            r#"<sheetFormatPr defaultRowHeight="15" customHeight="1"/><sheetData"#,
            1,
        )
    } else {
        panic!("cannot set row height in {next}");
    }
}

fn export_spark(path: &Path, tag: &str) -> Result<PathBuf, ooxp::Error> {
    let out = tmp(tag).with_extension("png");
    run(&json!({
        "schemaVersion": 1,
        "action": "exportSparklineImage",
        "path": path.to_string_lossy(),
        "id": "sheet:Sheet1/spark:1",
        "output": out.to_string_lossy()
    })
    .to_string())?;
    Ok(out)
}

fn export_sheet(path: &Path, tag: &str) -> Result<PathBuf, ooxp::Error> {
    let out = tmp(tag).with_extension("png");
    run(&json!({
        "schemaVersion": 1,
        "action": "exportSheetImage",
        "path": path.to_string_lossy(),
        "id": "sheet:Sheet1",
        "output": out.to_string_lossy()
    })
    .to_string())?;
    Ok(out)
}

fn decode_rgb(png: &[u8]) -> (u32, u32, Vec<u8>) {
    let mut reader = png::Decoder::new(Cursor::new(png)).read_info().unwrap();
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).unwrap();
    (info.width, info.height, buf[..info.buffer_size()].to_vec())
}

fn part_names(path: &Path) -> Vec<String> {
    let file = std::fs::File::open(path).unwrap();
    let mut zip = ZipArchive::new(file).unwrap();
    (0..zip.len())
        .map(|i| zip.by_index(i).unwrap().name().to_string())
        .collect()
}

fn part_text(path: &Path, name: &str) -> String {
    let file = std::fs::File::open(path).unwrap();
    let mut zip = ZipArchive::new(file).unwrap();
    let mut part = zip.by_name(name).unwrap();
    let mut s = String::new();
    part.read_to_string(&mut s).unwrap();
    s
}

fn sheet_part(path: &Path) -> String {
    part_names(path)
        .into_iter()
        .find(|n| n.starts_with("xl/worksheets/sheet") && n.ends_with(".xml"))
        .expect("worksheet part")
}

fn rewrite_part(src: &Path, dest: &Path, name: &str, next: &str) {
    let file = std::fs::File::open(src).unwrap();
    let mut inp = ZipArchive::new(file).unwrap();
    let out_file = std::fs::File::create(dest).unwrap();
    let mut out = ZipWriter::new(out_file);
    let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    for i in 0..inp.len() {
        let mut part = inp.by_index(i).unwrap();
        let part_name = part.name().to_string();
        out.start_file(part_name.clone(), opts).unwrap();
        if part_name == name {
            out.write_all(next.as_bytes()).unwrap();
        } else {
            let mut buf = Vec::new();
            part.read_to_end(&mut buf).unwrap();
            out.write_all(&buf).unwrap();
        }
    }
    out.finish().unwrap();
}

fn with_date_axis_attr(xml: &str) -> String {
    if xml.contains(r#"<x14:sparklineGroup "#) {
        xml.replacen(
            r#"<x14:sparklineGroup "#,
            r#"<x14:sparklineGroup dateAxis="1" "#,
            1,
        )
    } else {
        xml.replacen(
            "<x14:sparklineGroup>",
            r#"<x14:sparklineGroup dateAxis="1">"#,
            1,
        )
    }
}

fn with_date_axis(src: &Path, tag: &str, date_formula: &str) -> PathBuf {
    let part = sheet_part(src);
    let xml = part_text(src, &part);
    assert!(
        xml.contains("sparklineGroup"),
        "expected a sparkline group in {part}"
    );
    let mut next = with_date_axis_attr(&xml);
    if next.contains("<x14:sparklines>") {
        next = next.replacen(
            "<x14:sparklines>",
            &format!("<xm:f>{date_formula}</xm:f><x14:sparklines>"),
            1,
        );
    }
    let dest = tmp(tag);
    rewrite_part(src, &dest, &part, &next);
    dest
}

fn peak_x(rgb: &[u8], w: u32, h: u32, color: [u8; 3]) -> u32 {
    let mut best_y = h;
    let mut sum = 0u64;
    let mut n = 0u64;
    for y in 0..h {
        for x in 0..w {
            let i = ((y * w + x) * 3) as usize;
            if [rgb[i], rgb[i + 1], rgb[i + 2]] == color {
                if y < best_y {
                    best_y = y;
                    sum = u64::from(x);
                    n = 1;
                } else if y == best_y {
                    sum += u64::from(x);
                    n += 1;
                }
            }
        }
    }
    assert!(n > 0, "expected spark ink");
    (sum / n) as u32
}

const SERIES: [u8; 3] = [0x37, 0x60, 0x92];

#[test]
fn insert_then_date_axis_keeps_package_and_equal_dates_match_category() {
    let src = create_book("eq", [10.0, 11.0, 12.0], [1.0, 4.0, 2.0]);
    let line = insert_spark(&src, "eq-line", None);
    let category = std::fs::read(export_spark(&line, "eq-cat").unwrap()).unwrap();
    let dated = with_date_axis(&line, "eq-dated", "Sheet1!A1:C1");
    let before = std::fs::read(&dated).unwrap();
    let png_path = export_spark(&dated, "eq-png").unwrap();
    assert_eq!(std::fs::read(&dated).unwrap(), before);
    let dated_png = std::fs::read(&png_path).unwrap();
    assert_eq!(&dated_png[0..8], &[137, 80, 78, 71, 13, 10, 26, 10]);
    assert_eq!(category, dated_png);

    let col_src = insert_spark(&src, "eq-col", Some("column"));
    let col_cat = std::fs::read(export_spark(&col_src, "eq-col-cat").unwrap()).unwrap();
    let col_dated = with_date_axis(&col_src, "eq-col-dated", "Sheet1!A1:C1");
    let col_png = std::fs::read(export_spark(&col_dated, "eq-col-png").unwrap()).unwrap();
    assert_eq!(col_cat, col_png);
}

#[test]
fn uneven_dates_differ_from_category_and_move_the_peak() {
    let src = create_book("un", [0.0, 1.0, 10.0], [1.0, 8.0, 1.0]);
    let line = insert_spark(&src, "un-line", None);
    let category = std::fs::read(export_spark(&line, "un-cat").unwrap()).unwrap();
    let dated = with_date_axis(&line, "un-dated", "Sheet1!A1:C1");
    let dated_png = std::fs::read(export_spark(&dated, "un-png").unwrap()).unwrap();
    assert_ne!(category, dated_png);
    let (w, h, rgb) = decode_rgb(&dated_png);
    let (cw, ch, crgb) = decode_rgb(&category);
    assert_eq!((w, h), (cw, ch));
    let left = peak_x(&rgb, w, h, SERIES);
    let mid = peak_x(&crgb, cw, ch, SERIES);
    assert!(
        left < mid,
        "an early clustered date should pull the peak left of the category mid; left={left} mid={mid}"
    );
}

#[test]
fn create_and_read_paths_export_line_column_and_win_loss() {
    let src = create_book("rw", [0.0, 9.0, 10.0], [1.0, 4.0, 2.0]);
    for kind in [None, Some("column"), Some("stacked")] {
        let inserted = insert_spark(&src, &format!("rw-{}", kind.unwrap_or("line")), kind);
        let dated = with_date_axis(
            &inserted,
            &format!("rw-dated-{}", kind.unwrap_or("line")),
            "Sheet1!A1:C1",
        );
        let before = std::fs::read(&dated).unwrap();
        let png = std::fs::read(export_spark(&dated, "rw-png").unwrap()).unwrap();
        assert_eq!(std::fs::read(&dated).unwrap(), before);
        assert_eq!(&png[0..8], &[137, 80, 78, 71, 13, 10, 26, 10]);
        let category = std::fs::read(export_spark(&inserted, "rw-cat").unwrap()).unwrap();
        assert_ne!(png, category, "{kind:?} uneven dates should move the bars");
    }
}

#[test]
fn sheet_export_paints_the_same_host_pixels_as_spark_export() {
    let src = tmp("sh-src");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": src.to_string_lossy(),
        "workbook": {
            "sheets": [{
                "name": "Sheet1",
                "cells": [
                    { "ref": "A2", "value": 1.0 },
                    { "ref": "B2", "value": 8.0 },
                    { "ref": "C2", "value": 1.0 },
                    { "ref": "A3", "value": 0.0 },
                    { "ref": "B3", "value": 1.0 },
                    { "ref": "C3", "value": 10.0 }
                ]
            }]
        }
    }));
    let line = insert_spark_at(&src, "sh-line", "A1", "A2:C2", None);
    let dated = with_date_axis(&line, "sh-dated", "Sheet1!A3:C3");
    let part = sheet_part(&dated);
    let no_grid = tmp("sh-nogrid");
    rewrite_part(
        &dated,
        &no_grid,
        &part,
        &with_uniform_host_metrics(&part_text(&dated, &part)),
    );
    let before = std::fs::read(&no_grid).unwrap();
    let spark = std::fs::read(export_spark(&no_grid, "sh-spark").unwrap()).unwrap();
    let sheet = std::fs::read(export_sheet(&no_grid, "sh-sheet").unwrap()).unwrap();
    assert_eq!(
        std::fs::read(&no_grid).unwrap(),
        before,
        "sheet export must not rewrite the package"
    );
    let (sw, sh, srgb) = decode_rgb(&spark);
    let (tw, _, trgb) = decode_rgb(&sheet);
    assert!(sw >= 1 && sh >= 1, "spark size {sw}x{sh}");
    assert!(tw >= sw);
    for y in 0..sh {
        for x in 0..sw {
            let a = ((y * sw + x) * 3) as usize;
            let b = ((y * tw + x) * 3) as usize;
            assert_eq!(&srgb[a..a + 3], &trgb[b..b + 3], "host pixel {x},{y}");
        }
    }
    assert!(
        srgb.chunks(3).any(|p| p == SERIES),
        "dated sheet host should paint the series color"
    );
}

#[test]
fn unsupported_date_axis_inputs_stay_not_implemented() {
    let src = create_book("ni", [0.0, 1.0, 10.0], [1.0, 4.0, 2.0]);
    let line = insert_spark(&src, "ni-line", None);
    let part = sheet_part(&line);
    let xml = part_text(&line, &part);

    // dateAxis 打开但没有组上的日期公式。
    let no_f = tmp("ni-no-f");
    rewrite_part(&line, &no_f, &part, &with_date_axis_attr(&xml));
    let err = export_spark(&no_f, "ni-no-f-png").unwrap_err();
    assert_eq!(err.code(), ErrorCode::NotImplemented);

    // 长度不合：日期只有两格。
    let short = with_date_axis(&line, "ni-short", "Sheet1!A1:B1");
    let err = export_spark(&short, "ni-short-png").unwrap_err();
    assert_eq!(err.code(), ErrorCode::NotImplemented);

    // 二维区域。
    let two_d = with_date_axis(&line, "ni-2d", "Sheet1!A1:C2");
    let err = export_spark(&two_d, "ni-2d-png").unwrap_err();
    assert_eq!(err.code(), ErrorCode::NotImplemented);

    // 标记仍不受支持，即使日期合法。
    let marked = tmp("ni-markers");
    let dated_xml = part_text(&with_date_axis(&line, "ni-mark-src", "Sheet1!A1:C1"), &part);
    rewrite_part(
        &with_date_axis(&line, "ni-mark-base", "Sheet1!A1:C1"),
        &marked,
        &part,
        &dated_xml.replacen(r#"dateAxis="1""#, r#"dateAxis="1" markers="1""#, 1),
    );
    let err = export_spark(&marked, "ni-mark-png").unwrap_err();
    assert_eq!(err.code(), ErrorCode::NotImplemented);

    // 至少两点但日期全部相同，不回退成等距分类轴。
    let same = create_book("ni-same", [5.0, 5.0, 5.0], [1.0, 4.0, 2.0]);
    let same_line = insert_spark(&same, "ni-same-line", None);
    let same_dated = with_date_axis(&same_line, "ni-same-dated", "Sheet1!A1:C1");
    let err = export_spark(&same_dated, "ni-same-png").unwrap_err();
    assert_eq!(err.code(), ErrorCode::NotImplemented);

    // 日期格子是文本，不是有限数值。
    let text_src = tmp("ni-text-src");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": text_src.to_string_lossy(),
        "workbook": {
            "sheets": [{
                "name": "Sheet1",
                "cells": [
                    { "ref": "A1", "value": "2020-01-01" },
                    { "ref": "B1", "value": 1.0 },
                    { "ref": "C1", "value": 10.0 },
                    { "ref": "A2", "value": 1.0 },
                    { "ref": "B2", "value": 4.0 },
                    { "ref": "C2", "value": 2.0 }
                ]
            }]
        }
    }));
    let text_line = insert_spark(&text_src, "ni-text-line", None);
    let text_dated = with_date_axis(&text_line, "ni-text-dated", "Sheet1!A1:C1");
    let err = export_spark(&text_dated, "ni-text-png").unwrap_err();
    assert_eq!(err.code(), ErrorCode::NotImplemented);
}
