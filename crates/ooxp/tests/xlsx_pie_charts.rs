//! 普通饼图出图：新建导出、读入已有图部件导出，以及图表工作表两个入口。
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
    let dir = std::env::temp_dir().join("ooxp-xlsx-pie-charts");
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

fn insert_pie(tag: &str) -> PathBuf {
    let src = create_book(tag);
    let out = tmp(&format!("{tag}-out"));
    apply(
        &src,
        &out,
        json!([{
            "op": "insertChart",
            "id": "sheet:Sheet1",
            "type": "pie",
            "anchor": "oneCell",
            "from": "E2",
            "cx": 5_486_400,
            "cy": 3_200_400,
            "dataFrom": "A10",
            "series": [
                { "name": "S1", "categories": ["A", "B", "C", "D"], "values": [1, 4, 3, 2] }
            ]
        }]),
    )
    .unwrap();
    out
}

const ACCENT1: [u8; 3] = [0x15, 0x60, 0x82];
const ACCENT2: [u8; 3] = [0xE9, 0x71, 0x32];
const DPT_RED: [u8; 3] = [0xCC, 0x00, 0x00];

#[test]
fn insert_standard_pie_export_keeps_package_and_paints_slice_colors() {
    let book = insert_pie("create");
    let xml = part_text(&book, &chart_part(&book));
    assert!(
        xml.contains("pieChart") && !xml.contains("pie3DChart") && !xml.contains("doughnutChart"),
        "insertChart type=pie should write a pieChart"
    );
    let before = std::fs::read(&book).unwrap();
    let png_path = export_chart_png(&book, "sheet:Sheet1/chart:1", "create-png").unwrap();
    assert_eq!(std::fs::read(&book).unwrap(), before);
    let png = std::fs::read(&png_path).unwrap();
    assert_eq!(&png[0..8], &[137, 80, 78, 71, 13, 10, 26, 10]);
    let (w, h, rgb) = decode_rgb(&png);
    assert_eq!((w, h), (576, 336));
    assert!(
        count_color(&rgb, ACCENT1, 18) >= 80,
        "inserted pie chart should paint the first slice color"
    );
    assert!(
        count_color(&rgb, ACCENT2, 18) >= 80,
        "inserted pie chart should paint the second slice color"
    );
}

#[test]
fn read_existing_pie_chart_part_exports_and_unsupported_details_stay_ni() {
    let book = insert_pie("read");
    let part = chart_part(&book);
    let xml = part_text(&book, &part);
    assert!(
        xml.contains("pieChart"),
        "{part} should already be a pie chart"
    );

    let decorated = if xml.contains("dPt") {
        xml.clone()
    } else {
        xml.replacen(
            "<c:ser>",
            "<c:ser><c:dPt><c:idx val=\"0\"/><c:spPr><a:solidFill><a:srgbClr val=\"CC0000\"/></a:solidFill></c:spPr></c:dPt>",
            1,
        )
    };
    let decorated = if decorated.contains("firstSliceAng") {
        decorated
    } else {
        decorated.replacen(
            "</c:pieChart>",
            "<c:firstSliceAng val=\"90\"/></c:pieChart>",
            1,
        )
    };
    let decorated = if decorated.contains("dispBlanksAs") {
        decorated
    } else {
        decorated.replace("<c:plotArea>", "<c:dispBlanksAs val=\"gap\"/><c:plotArea>")
    };
    let existing = tmp("read-existing");
    rewrite_part(&book, &existing, &part, &decorated);
    let before = std::fs::read(&existing).unwrap();
    let png_path = export_chart_png(&existing, "sheet:Sheet1/chart:1", "read-png").unwrap();
    assert_eq!(std::fs::read(&existing).unwrap(), before);
    let (w, h, rgb) = decode_rgb(&std::fs::read(&png_path).unwrap());
    assert_eq!((w, h), (576, 336));
    assert!(
        count_color(&rgb, DPT_RED, 18) >= 80,
        "read-in dPt solid fill should paint the first slice"
    );
    assert!(count_color(&rgb, ACCENT2, 18) >= 80);

    let doughnut = xml
        .replace("<c:pieChart>", "<c:doughnutChart>")
        .replace("</c:pieChart>", "</c:doughnutChart>");
    let doughnut_path = tmp("read-doughnut");
    rewrite_part(&book, &doughnut_path, &part, &doughnut);
    let err = export_chart_png(&doughnut_path, "sheet:Sheet1/chart:1", "doughnut-png").unwrap_err();
    assert_eq!(err.code(), ErrorCode::NotImplemented);

    let exploded = if xml.contains("explosion") {
        xml.clone()
    } else {
        xml.replacen("<c:ser>", "<c:ser><c:explosion val=\"10\"/>", 1)
    };
    let exploded_path = tmp("read-exploded");
    rewrite_part(&book, &exploded_path, &part, &exploded);
    let err = export_chart_png(&exploded_path, "sheet:Sheet1/chart:1", "exploded-png").unwrap_err();
    assert_eq!(err.code(), ErrorCode::NotImplemented);
}

#[test]
fn chartsheet_standard_pie_png_matches_chart_image_and_unsupported_stays_ni() {
    let src = create_book("cs");
    let pie = tmp("cs-pie");
    apply(
        &src,
        &pie,
        json!([{
            "op": "insertChartSheet",
            "id": "xlsx:workbook",
            "name": "Chart1",
            "dataFrom": "A10",
            "type": "pie",
            "series": [
                { "name": "S1", "categories": ["A", "B", "C"], "values": [1, 4, 3] }
            ]
        }]),
    )
    .unwrap();
    let xml = part_text(&pie, &chart_part(&pie));
    assert!(
        xml.contains("pieChart"),
        "insertChartSheet type=pie should write pieChart"
    );

    let before = std::fs::read(&pie).unwrap();
    let sheet_png = tmp("cs-pie-sheet").with_extension("png");
    let chart_png = tmp("cs-pie-chart").with_extension("png");
    run(&json!({
        "schemaVersion": 1,
        "action": "exportSheetImage",
        "path": pie.to_string_lossy(),
        "id": "sheet:Chart1",
        "output": sheet_png.to_string_lossy()
    })
    .to_string())
    .unwrap();
    run(&json!({
        "schemaVersion": 1,
        "action": "exportChartImage",
        "path": pie.to_string_lossy(),
        "id": "sheet:Chart1/chart:2",
        "output": chart_png.to_string_lossy()
    })
    .to_string())
    .unwrap();
    assert_eq!(std::fs::read(&pie).unwrap(), before);
    let sheet = std::fs::read(&sheet_png).unwrap();
    let chart = std::fs::read(&chart_png).unwrap();
    assert_eq!(
        sheet, chart,
        "chartsheet sheet and chart entries must match"
    );
    let (w, h, rgb) = decode_rgb(&chart);
    assert!(w > 1 && h > 1);
    assert!(
        count_color(&rgb, ACCENT1, 18) >= 80 && count_color(&rgb, ACCENT2, 18) >= 80,
        "chartsheet pie export should paint slice colors"
    );

    let doughnut = xml
        .replace("<c:pieChart>", "<c:doughnutChart>")
        .replace("</c:pieChart>", "</c:doughnutChart>");
    let doughnut_path = tmp("cs-doughnut");
    rewrite_part(&pie, &doughnut_path, &chart_part(&pie), &doughnut);
    for (action, id, tag) in [
        ("exportSheetImage", "sheet:Chart1", "cs-sheet-ni"),
        ("exportChartImage", "sheet:Chart1/chart:2", "cs-chart-ni"),
    ] {
        let err = run(&json!({
            "schemaVersion": 1,
            "action": action,
            "path": doughnut_path.to_string_lossy(),
            "id": id,
            "output": tmp(tag).with_extension("png").to_string_lossy()
        })
        .to_string())
        .unwrap_err();
        assert_eq!(err.code(), ErrorCode::NotImplemented, "{action}");
    }
}
