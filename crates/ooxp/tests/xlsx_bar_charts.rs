//! 簇状条形图出图：新建导出、读入已有图部件导出，以及图表工作表入口。
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
    let dir = std::env::temp_dir().join("ooxp-xlsx-bar-charts");
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

fn count_in(rgb: &[u8], w: u32, rect: (u32, u32, u32, u32), target: [u8; 3], tol: i16) -> usize {
    let (x0, x1, y0, y1) = rect;
    let mut n = 0;
    for y in y0..y1 {
        for x in x0..x1 {
            let i = ((y * w + x) * 3) as usize;
            if near([rgb[i], rgb[i + 1], rgb[i + 2]], target, tol) {
                n += 1;
            }
        }
    }
    n
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

fn insert_bar(tag: &str) -> PathBuf {
    let src = create_book(tag);
    let out = tmp(&format!("{tag}-out"));
    apply(
        &src,
        &out,
        json!([{
            "op": "insertChart",
            "id": "sheet:Sheet1",
            "type": "bar",
            "anchor": "oneCell",
            "from": "E2",
            "cx": 5_486_400,
            "cy": 3_200_400,
            "dataFrom": "A10",
            "series": [
                { "name": "S1", "categories": ["A", "B"], "values": [8, 1] },
                { "name": "S2", "values": [2, 3] }
            ]
        }]),
    )
    .unwrap();
    out
}

fn insert_col(tag: &str) -> PathBuf {
    let src = create_book(tag);
    let out = tmp(&format!("{tag}-out"));
    apply(
        &src,
        &out,
        json!([{
            "op": "insertChart",
            "id": "sheet:Sheet1",
            "type": "col",
            "anchor": "oneCell",
            "from": "E2",
            "cx": 5_486_400,
            "cy": 3_200_400,
            "dataFrom": "A10",
            "series": [
                { "name": "S1", "categories": ["A", "B"], "values": [8, 1] },
                { "name": "S2", "values": [2, 3] }
            ]
        }]),
    )
    .unwrap();
    out
}

const ACCENT1: [u8; 3] = [0x15, 0x60, 0x82];
const ACCENT2: [u8; 3] = [0xE9, 0x71, 0x32];

fn assert_bar_axes(xml: &str) {
    assert!(
        xml.contains(r#"<c:barDir val="bar"/>"#),
        "expected barDir=bar: {xml}"
    );
    let cat = xml
        .split("<c:catAx>")
        .nth(1)
        .and_then(|rest| rest.split("</c:catAx>").next())
        .expect("catAx");
    let val = xml
        .split("<c:valAx>")
        .nth(1)
        .and_then(|rest| rest.split("</c:valAx>").next())
        .expect("valAx");
    assert!(
        cat.contains(r#"<c:axPos val="l"/>"#),
        "bar catAx should sit on the left: {cat}"
    );
    assert!(
        cat.contains(r#"<c:orientation val="maxMin"/>"#),
        "insert/set bar catAx should use maxMin (first category at the top): {cat}"
    );
    assert!(
        val.contains(r#"<c:axPos val="b"/>"#),
        "bar valAx should sit at the bottom: {val}"
    );
}

fn assert_col_axes(xml: &str) {
    assert!(
        xml.contains(r#"<c:barDir val="col"/>"#),
        "expected barDir=col: {xml}"
    );
    let cat = xml
        .split("<c:catAx>")
        .nth(1)
        .and_then(|rest| rest.split("</c:catAx>").next())
        .expect("catAx");
    let val = xml
        .split("<c:valAx>")
        .nth(1)
        .and_then(|rest| rest.split("</c:valAx>").next())
        .expect("valAx");
    assert!(
        cat.contains(r#"<c:axPos val="b"/>"#),
        "col catAx should sit at the bottom: {cat}"
    );
    assert!(
        cat.contains(r#"<c:orientation val="minMax"/>"#),
        "col catAx should use minMax: {cat}"
    );
    assert!(
        val.contains(r#"<c:axPos val="l"/>"#),
        "col valAx should sit on the left: {val}"
    );
}

#[test]
fn insert_clustered_bar_export_keeps_package_and_paints_series_colors() {
    let book = insert_bar("create");
    let part = chart_part(&book);
    let xml = part_text(&book, &part);
    assert_bar_axes(&xml);
    let before = std::fs::read(&book).unwrap();
    let png_path = export_chart_png(&book, "sheet:Sheet1/chart:1", "create-png").unwrap();
    assert_eq!(std::fs::read(&book).unwrap(), before);
    let png = std::fs::read(&png_path).unwrap();
    assert_eq!(&png[0..8], &[137, 80, 78, 71, 13, 10, 26, 10]);
    let (w, h, rgb) = decode_rgb(&png);
    assert_eq!((w, h), (576, 336));
    assert!(
        count_color(&rgb, ACCENT1, 18) >= 20,
        "inserted bar should paint the first series color"
    );
    assert!(
        count_color(&rgb, ACCENT2, 18) >= 20,
        "inserted bar should paint the second series color"
    );

    // insert 默认 maxMin：第一类（值为 8）的长条应出现在上半绘图区右侧。
    let top_right = count_in(&rgb, w, (w / 2, w * 3 / 4, 20, h / 2), ACCENT1, 18);
    let bot_right = count_in(&rgb, w, (w / 2, w * 3 / 4, h / 2, h - 20), ACCENT1, 18);
    assert!(
        top_right > bot_right,
        "first category of an inserted bar should sit at the top; top={top_right} bottom={bot_right}"
    );

    let col = insert_col("create-col");
    let col_png =
        std::fs::read(export_chart_png(&col, "sheet:Sheet1/chart:1", "create-col-png").unwrap())
            .unwrap();
    assert_ne!(
        png, col_png,
        "bar and column exports of the same data should differ"
    );
}

#[test]
fn set_chart_type_rewrites_bar_and_column_axes() {
    let col = insert_col("from-col");
    assert_col_axes(&part_text(&col, &chart_part(&col)));
    let bar = tmp("to-bar");
    apply(
        &col,
        &bar,
        json!([{ "op": "setChartType", "id": "sheet:Sheet1/chart:1", "type": "bar" }]),
    )
    .unwrap();
    let xml = part_text(&bar, &chart_part(&bar));
    assert_bar_axes(&xml);
    let png_path = export_chart_png(&bar, "sheet:Sheet1/chart:1", "typed-bar").unwrap();
    let png = std::fs::read(&png_path).unwrap();
    let (w, h, rgb) = decode_rgb(&png);
    assert_eq!((w, h), (576, 336));
    assert!(count_color(&rgb, ACCENT1, 18) >= 20);

    let back = tmp("back-col");
    apply(
        &bar,
        &back,
        json!([{ "op": "setChartType", "id": "sheet:Sheet1/chart:1", "type": "col" }]),
    )
    .unwrap();
    assert_col_axes(&part_text(&back, &chart_part(&back)));
    export_chart_png(&back, "sheet:Sheet1/chart:1", "typed-col").unwrap();
}

#[test]
fn read_existing_bar_part_exports_and_unsupported_details_stay_ni() {
    let book = insert_bar("read");
    let part = chart_part(&book);
    let xml = part_text(&book, &part);
    assert!(
        xml.contains(r#"barDir val="bar""#),
        "{part} should already be a bar chart"
    );

    let existing = tmp("read-existing");
    rewrite_part(&book, &existing, &part, &xml);
    let before = std::fs::read(&existing).unwrap();
    let png_path = export_chart_png(&existing, "sheet:Sheet1/chart:1", "read-png").unwrap();
    assert_eq!(std::fs::read(&existing).unwrap(), before);
    let (w, h, rgb) = decode_rgb(&std::fs::read(&png_path).unwrap());
    assert_eq!((w, h), (576, 336));
    assert!(count_color(&rgb, ACCENT1, 18) >= 20);

    let cases: [(&str, String); 6] = [
        (
            "stacked",
            xml.replace(r#"grouping val="clustered""#, r#"grouping val="stacked""#),
        ),
        (
            "percent",
            xml.replace(
                r#"grouping val="clustered""#,
                r#"grouping val="percentStacked""#,
            ),
        ),
        (
            "labels",
            xml.replace(r#"<c:showVal val="0"/>"#, r#"<c:showVal val="1"/>"#),
        ),
        (
            "blanks-zero",
            if xml.contains("dispBlanksAs") {
                xml.replace(r#"dispBlanksAs val="gap""#, r#"dispBlanksAs val="zero""#)
            } else {
                xml.replacen("<c:chart>", r#"<c:chart><c:dispBlanksAs val="zero"/>"#, 1)
            },
        ),
        (
            "blanks-span",
            if xml.contains("dispBlanksAs") {
                xml.replace(r#"dispBlanksAs val="gap""#, r#"dispBlanksAs val="span""#)
            } else {
                xml.replacen("<c:chart>", r#"<c:chart><c:dispBlanksAs val="span"/>"#, 1)
            },
        ),
        (
            "column-axes",
            xml.replace(r#"<c:axPos val="l"/>"#, r#"<c:axPos val="__cat__"/>"#)
                .replace(r#"<c:axPos val="b"/>"#, r#"<c:axPos val="l"/>"#)
                .replace(r#"<c:axPos val="__cat__"/>"#, r#"<c:axPos val="b"/>"#),
        ),
    ];
    // The last case has the double-replace problem. Fix below after write? We'll patch.
    for (tag, next) in cases {
        let ni_path = tmp(tag);
        rewrite_part(&book, &ni_path, &part, &next);
        let err =
            export_chart_png(&ni_path, "sheet:Sheet1/chart:1", &format!("{tag}-png")).unwrap_err();
        assert_eq!(err.code(), ErrorCode::NotImplemented, "{tag}");
    }
}

#[test]
fn chartsheet_bar_png_matches_chart_image_and_stacked_stays_ni() {
    let src = create_book("cs");
    let book = tmp("cs-bar");
    apply(
        &src,
        &book,
        json!([{
            "op": "insertChartSheet",
            "id": "xlsx:workbook",
            "name": "Chart1",
            "dataFrom": "A10",
            "type": "bar",
            "series": [
                { "name": "S1", "categories": ["A", "B"], "values": [8, 1] },
                { "name": "S2", "values": [2, 3] }
            ]
        }]),
    )
    .unwrap();
    assert_bar_axes(&part_text(&book, &chart_part(&book)));
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
        panic!("exportSheetImage 应写出图表工作表上已支持的簇状条形图");
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
    assert_eq!((width, height), (576, 336));
    let (_, _, rgb) = decode_rgb(&sheet);
    assert!(count_color(&rgb, ACCENT1, 18) >= 20);
    assert!(count_color(&rgb, ACCENT2, 18) >= 20);

    let part = chart_part(&book);
    let xml = part_text(&book, &part);
    let stacked = xml.replace(r#"grouping val="clustered""#, r#"grouping val="stacked""#);
    let ni_path = tmp("cs-stacked");
    rewrite_part(&book, &ni_path, &part, &stacked);
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
