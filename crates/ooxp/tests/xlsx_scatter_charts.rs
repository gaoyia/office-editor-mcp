//! 带标记的散点图出图：新建导出、读入已有图部件导出，以及图表工作表入口。
//! 不把官方 Excel/Slide 导出对照算作通过。

use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::json;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-xlsx-scatter-charts");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn tmp(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}.xlsx"))
}

fn create_book(tag: &str) -> PathBuf {
    let path = tmp(tag);
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "workbook": { "sheets": [{ "name": "Sheet1", "cells": [{ "ref": "A1", "value": 1 }] }] }
    })
    .to_string())
    .unwrap();
    path
}

fn apply(src: &Path, out: &Path, ops: serde_json::Value) -> Result<RunOutput, ooxp::Error> {
    run(&json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": out.to_string_lossy(),
        "ops": ops,
    })
    .to_string())
}

fn export_chart_png(path: &Path, id: &str, tag: &str) -> Result<PathBuf, ooxp::Error> {
    let out = tmp(tag).with_extension("png");
    run(&json!({
        "schemaVersion": 1,
        "action": "exportChartImage",
        "path": path.to_string_lossy(),
        "id": id,
        "output": out.to_string_lossy()
    })
    .to_string())?;
    Ok(out)
}

fn decode_rgb(png: &[u8]) -> (u32, u32, Vec<u8>) {
    let mut reader = png::Decoder::new(Cursor::new(png)).read_info().unwrap();
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).unwrap();
    let data = &buf[..info.buffer_size()];
    let rgb = match info.color_type {
        png::ColorType::Rgb => data.to_vec(),
        png::ColorType::Rgba => data
            .as_chunks::<4>()
            .0
            .iter()
            .flat_map(|p| {
                let a = u16::from(p[3]);
                (0..3).map(move |i| ((u16::from(p[i]) * a + 255 * (255 - a) + 127) / 255) as u8)
            })
            .collect(),
        _ => panic!("unexpected color type {:?}", info.color_type),
    };
    (info.width, info.height, rgb)
}

fn near(a: [u8; 3], b: [u8; 3], tol: i16) -> bool {
    (0..3).all(|i| (i16::from(a[i]) - i16::from(b[i])).abs() <= tol)
}

fn count_color(rgb: &[u8], target: [u8; 3], tol: i16) -> usize {
    rgb.as_chunks::<3>()
        .0
        .iter()
        .filter(|p| near(**p, target, tol))
        .count()
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

fn chart_part(path: &Path) -> String {
    part_names(path)
        .into_iter()
        .find(|n| n.starts_with("xl/charts/chart") && n.ends_with(".xml"))
        .expect("chart part")
}

fn insert_scatter(tag: &str) -> PathBuf {
    let src = create_book(tag);
    let out = tmp(&format!("{tag}-out"));
    apply(
        &src,
        &out,
        json!([{
            "op": "insertChart",
            "id": "sheet:Sheet1",
            "type": "scatter",
            "anchor": "oneCell",
            "from": "E2",
            "cx": 5_486_400,
            "cy": 3_200_400,
            "dataFrom": "A10",
            "series": [
                { "name": "S1", "categories": [1, 3, 6, 9], "values": [1, 4, 3, 2] },
                { "name": "S2", "values": [2, 1, 4, 3] }
            ]
        }]),
    )
    .unwrap();
    out
}

const ACCENT1: [u8; 3] = [0x15, 0x60, 0x82];
const ACCENT2: [u8; 3] = [0xE9, 0x71, 0x32];

#[test]
fn insert_marked_scatter_export_keeps_package_and_paints_series_colors() {
    let book = insert_scatter("create");
    let before = std::fs::read(&book).unwrap();
    let png_path = export_chart_png(&book, "sheet:Sheet1/chart:1", "create-png").unwrap();
    assert_eq!(std::fs::read(&book).unwrap(), before);
    let png = std::fs::read(&png_path).unwrap();
    assert_eq!(&png[0..8], &[137, 80, 78, 71, 13, 10, 26, 10]);
    let (w, h, rgb) = decode_rgb(&png);
    assert_eq!((w, h), (576, 336));
    assert!(
        count_color(&rgb, ACCENT1, 18) >= 20,
        "inserted scatter should paint the first series color"
    );
    assert!(
        count_color(&rgb, ACCENT2, 18) >= 20,
        "inserted scatter should paint the second series color"
    );
}

#[test]
fn read_existing_scatter_part_exports_and_unsupported_details_stay_ni() {
    let book = insert_scatter("read");
    let part = chart_part(&book);
    let xml = part_text(&book, &part);
    assert!(
        xml.contains("scatterChart"),
        "{part} should already be a scatter chart"
    );
    assert!(
        xml.contains("lineMarker") || !xml.contains("scatterStyle"),
        "insert path uses lineMarker or omits the style"
    );

    let marked = if xml.contains("scatterStyle") {
        xml.replace(
            r#"scatterStyle val="lineMarker""#,
            r#"scatterStyle val="marker""#,
        )
    } else {
        xml.replacen(
            "<c:scatterChart>",
            r#"<c:scatterChart><c:scatterStyle val="marker"/>"#,
            1,
        )
    };
    let existing = tmp("read-existing");
    rewrite_part(&book, &existing, &part, &marked);
    let before = std::fs::read(&existing).unwrap();
    let png_path = export_chart_png(&existing, "sheet:Sheet1/chart:1", "read-png").unwrap();
    assert_eq!(std::fs::read(&existing).unwrap(), before);
    let (w, h, rgb) = decode_rgb(&std::fs::read(&png_path).unwrap());
    assert_eq!((w, h), (576, 336));
    assert!(count_color(&rgb, ACCENT1, 18) >= 20);

    let line_only = xml.replace(
        r#"scatterStyle val="lineMarker""#,
        r#"scatterStyle val="line""#,
    );
    let line_path = tmp("read-line-only");
    rewrite_part(&book, &line_path, &part, &line_only);
    let err = export_chart_png(&line_path, "sheet:Sheet1/chart:1", "line-png").unwrap_err();
    assert_eq!(err.code(), ErrorCode::NotImplemented);

    let smooth = xml.replace(
        r#"scatterStyle val="lineMarker""#,
        r#"scatterStyle val="smoothMarker""#,
    );
    let smooth_path = tmp("read-smooth");
    rewrite_part(&book, &smooth_path, &part, &smooth);
    // Official-like smoothMarker must produce an image (close-enough
    // curve, not a unique interpolation lock). scatterStyle=line above
    // stays NI because Office does not save that COM name.
    let smooth_png =
        export_chart_png(&smooth_path, "sheet:Sheet1/chart:1", "smooth-png").unwrap();
    let (_, _, smooth_rgb) = decode_rgb(&std::fs::read(&smooth_png).unwrap());
    assert!(count_color(&smooth_rgb, ACCENT1, 18) >= 20);
}

#[test]
fn chartsheet_scatter_png_matches_chart_image_and_line_only_stays_ni() {
    let src = create_book("cs");
    let book = tmp("cs-scatter");
    apply(
        &src,
        &book,
        json!([{
            "op": "insertChartSheet",
            "id": "xlsx:workbook",
            "name": "Chart1",
            "dataFrom": "A10",
            "type": "scatter",
            "series": [
                { "name": "S1", "categories": [1, 2, 4], "values": [1, 4, 3] },
                { "name": "S2", "values": [2, 1, 5] }
            ]
        }]),
    )
    .unwrap();
    let before = std::fs::read(&book).unwrap();
    let sheet_png = tmp("cs-sheet").with_extension("png");
    let chart_png = tmp("cs-chart").with_extension("png");
    let RunOutput::Written {
        content_type,
        ops_applied,
        ..
    } = run(&json!({
        "schemaVersion": 1,
        "action": "exportSheetImage",
        "path": book.to_string_lossy(),
        "id": "sheet:Chart1",
        "output": sheet_png.to_string_lossy()
    })
    .to_string())
    .unwrap()
    else {
        panic!("exportSheetImage 应写出图表工作表上已支持的带标记散点图");
    };
    assert_eq!(content_type.as_deref(), Some("image/png"));
    assert!(ops_applied.is_none());
    let RunOutput::Written { .. } = run(&json!({
        "schemaVersion": 1,
        "action": "exportChartImage",
        "path": book.to_string_lossy(),
        "id": "sheet:Chart1/chart:2",
        "output": chart_png.to_string_lossy()
    })
    .to_string())
    .unwrap() else {
        panic!("exportChartImage 应写出同一张图");
    };
    assert_eq!(std::fs::read(&book).unwrap(), before);
    let sheet = std::fs::read(&sheet_png).unwrap();
    let chart = std::fs::read(&chart_png).unwrap();
    assert_eq!(&sheet[0..8], &[137, 80, 78, 71, 13, 10, 26, 10]);
    assert_eq!(sheet, chart);
    let width = u32::from_be_bytes(sheet[16..20].try_into().unwrap());
    let height = u32::from_be_bytes(sheet[20..24].try_into().unwrap());
    assert_eq!((width, height), (975, 638));

    let part = chart_part(&book);
    let xml = part_text(&book, &part);
    let line_only = xml.replace(
        r#"scatterStyle val="lineMarker""#,
        r#"scatterStyle val="line""#,
    );
    let ni_path = tmp("cs-line-only");
    rewrite_part(&book, &ni_path, &part, &line_only);
    for (action, id, tag) in [
        ("exportSheetImage", "sheet:Chart1", "cs-ni-sheet"),
        ("exportChartImage", "sheet:Chart1/chart:2", "cs-ni-chart"),
    ] {
        let err = run(&json!({
            "schemaVersion": 1,
            "action": action,
            "path": ni_path.to_string_lossy(),
            "id": id,
            "output": tmp(tag).with_extension("png").to_string_lossy()
        })
        .to_string())
        .unwrap_err();
        assert_eq!(err.code(), ErrorCode::NotImplemented, "{action}");
    }
}

#[test]
fn string_categories_converted_to_scatter_stay_not_implemented() {
    let src = create_book("str");
    let col = tmp("str-col");
    apply(
        &src,
        &col,
        json!([{
            "op": "insertChart",
            "id": "sheet:Sheet1",
            "type": "col",
            "anchor": "oneCell",
            "from": "E2",
            "cx": 5_486_400,
            "cy": 3_200_400,
            "dataFrom": "A10",
            "series": [{ "name": "S1", "categories": ["A", "B", "C"], "values": [1, 2, 3] }]
        }]),
    )
    .unwrap();
    let scatter = tmp("str-scatter");
    apply(
        &col,
        &scatter,
        json!([{
            "op": "setChartType",
            "id": "sheet:Sheet1/chart:1",
            "type": "scatter"
        }]),
    )
    .unwrap();
    let err = export_chart_png(&scatter, "sheet:Sheet1/chart:1", "str-png").unwrap_err();
    assert_eq!(err.code(), ErrorCode::NotImplemented);
}
