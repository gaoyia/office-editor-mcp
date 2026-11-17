//! 工作表旋转与两端对齐：只画合同已锁定、几何可独立核验的那一档。
//! 官方 Excel 16.0 CopyPicture 对照未做，不把内部自测算验收通过。

use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::json;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-xlsx-align-rotate");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn tmp(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}.xlsx"))
}

fn run_ok(value: serde_json::Value) -> RunOutput {
    run(&value.to_string()).unwrap()
}

fn create_sheet(tag: &str, cells: serde_json::Value) -> PathBuf {
    let path = tmp(tag);
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "workbook": { "sheets": [{ "name": "Sheet1", "cells": cells }] }
    }));
    path
}

fn apply_align(path: &Path, ops: serde_json::Value) {
    run_ok(json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": path.to_string_lossy(),
        "ops": ops
    }));
}

fn export_png(path: &Path) -> Result<Vec<u8>, ooxp::Error> {
    let out = path.with_extension("png");
    let written = run(&json!({
        "schemaVersion": 1,
        "action": "exportSheetImage",
        "path": path.to_string_lossy(),
        "id": "sheet:Sheet1",
        "output": out.to_string_lossy(),
    })
    .to_string())?;
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
    Ok(std::fs::read(out).unwrap())
}

fn export_ok(path: &Path) -> Vec<u8> {
    export_png(path).unwrap()
}

fn export_ni(path: &Path) {
    let err = export_png(path).unwrap_err();
    assert_eq!(err.code(), ErrorCode::NotImplemented);
}

fn png_rgb(png: &[u8]) -> (u32, u32, Vec<[u8; 3]>) {
    let mut reader = png::Decoder::new(Cursor::new(png)).read_info().unwrap();
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).unwrap();
    let data = &buf[..info.buffer_size()];
    let rgb: Vec<[u8; 3]> = match info.color_type {
        png::ColorType::Rgb => data
            .as_chunks::<3>()
            .0
            .iter()
            .map(|p| [p[0], p[1], p[2]])
            .collect(),
        png::ColorType::Rgba => data
            .as_chunks::<4>()
            .0
            .iter()
            .map(|p| {
                let a = u16::from(p[3]);
                let conv = |i: usize| ((u16::from(p[i]) * a + 255 * (255 - a) + 127) / 255) as u8;
                [conv(0), conv(1), conv(2)]
            })
            .collect(),
        _ => panic!("unexpected color type {:?}", info.color_type),
    };
    (info.width, info.height, rgb)
}

fn wide_col_ops(id_prefix: &str, horizontal: &str, extra: serde_json::Value) -> serde_json::Value {
    let mut align = json!({
        "horizontal": horizontal,
        "vertical": "top"
    });
    if let Some(obj) = extra.as_object() {
        for (k, v) in obj {
            align[k] = v.clone();
        }
    }
    json!([
        { "op": "setCellStyle", "id": format!("cell:Sheet1!{id_prefix}"), "align": align },
        { "op": "setColWidth", "id": "sheet:Sheet1", "col": "A", "width": 24.0 },
        { "op": "setRowHeight", "id": "sheet:Sheet1", "row": 1, "height": 30.0 }
    ])
}

fn rewrite_part(src: &Path, dest: &Path, name: &str, edit: impl FnOnce(&str) -> String) {
    let file = std::fs::File::open(src).unwrap();
    let mut inp = ZipArchive::new(file).unwrap();
    let out_file = std::fs::File::create(dest).unwrap();
    let mut out = ZipWriter::new(out_file);
    let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    let mut edit = Some(edit);
    for i in 0..inp.len() {
        let mut part = inp.by_index(i).unwrap();
        let part_name = part.name().to_string();
        out.start_file(part_name.clone(), opts).unwrap();
        let mut buf = Vec::new();
        part.read_to_end(&mut buf).unwrap();
        if part_name == name {
            let next = edit.take().expect("part appears twice")(&String::from_utf8(buf).unwrap());
            out.write_all(next.as_bytes()).unwrap();
        } else {
            out.write_all(&buf).unwrap();
        }
    }
    assert!(edit.is_none(), "missing part {name}");
    out.finish().unwrap();
}

fn dark_count(rgb: &[[u8; 3]], w: u32, x0: u32, x1: u32, h: u32) -> usize {
    let mut n = 0usize;
    for y in 0..h {
        for x in x0..x1 {
            let px = rgb[(y * w + x) as usize];
            if px[0] < 80 && px[1] < 80 && px[2] < 80 {
                n += 1;
            }
        }
    }
    n
}

fn dark_row_bands(rgb: &[[u8; 3]], w: u32, h: u32) -> Vec<(u32, u32)> {
    let mut bands = Vec::new();
    let mut start: Option<u32> = None;
    for y in 0..h {
        let has = (0..w).any(|x| {
            let px = rgb[(y * w + x) as usize];
            px[0] < 80 && px[1] < 80 && px[2] < 80
        });
        match (has, start) {
            (true, None) => start = Some(y),
            (false, Some(s)) => {
                bands.push((s, y));
                start = None;
            }
            _ => {}
        }
    }
    if let Some(s) = start {
        bands.push((s, h));
    }
    bands
}

fn dark_xs_in_rows(rgb: &[[u8; 3]], w: u32, y0: u32, y1: u32) -> (u32, u32) {
    let mut min_x = w;
    let mut max_x = 0u32;
    for y in y0..y1 {
        for x in 0..w {
            let px = rgb[(y * w + x) as usize];
            if px[0] < 80 && px[1] < 80 && px[2] < 80 {
                min_x = min_x.min(x);
                max_x = max_x.max(x);
            }
        }
    }
    (min_x, max_x)
}

#[test]
fn justify_without_soft_wrap_matches_left_pixels_and_keeps_package() {
    let left = create_sheet("just-left", json!([{ "ref": "A1", "value": "hello" }]));
    apply_align(&left, wide_col_ops("A1", "left", json!({})));
    let just = create_sheet("just-just", json!([{ "ref": "A1", "value": "hello" }]));
    apply_align(&just, wide_col_ops("A1", "justify", json!({})));
    let before = std::fs::read(&just).unwrap();
    let left_png = export_ok(&left);
    let just_png = export_ok(&just);
    assert_eq!(std::fs::read(&just).unwrap(), before);
    assert_eq!(
        left_png, just_png,
        "ECMA: if no line wraps, justify is not justified; this slice paints it as left"
    );
}

#[test]
fn justify_hard_breaks_without_soft_wrap_match_left() {
    let left = create_sheet("hard-left", json!([{ "ref": "A1", "value": "one\ntwo" }]));
    apply_align(&left, wide_col_ops("A1", "left", json!({ "wrap": true })));
    let just = create_sheet("hard-just", json!([{ "ref": "A1", "value": "one\ntwo" }]));
    apply_align(
        &just,
        wide_col_ops("A1", "justify", json!({ "wrap": true })),
    );
    assert_eq!(export_ok(&left), export_ok(&just));
}

#[test]
fn justify_indent_and_last_line_stay_ni() {
    let indented = create_sheet("just-ind", json!([{ "ref": "A1", "value": "hello" }]));
    apply_align(
        &indented,
        wide_col_ops("A1", "justify", json!({ "indent": 1 })),
    );
    export_ni(&indented);

    let last = create_sheet("just-last", json!([{ "ref": "A1", "value": "hello" }]));
    apply_align(&last, wide_col_ops("A1", "justify", json!({})));
    let last_ni = tmp("just-last-xml");
    rewrite_part(&last, &last_ni, "xl/styles.xml", |xml| {
        assert!(xml.contains("horizontal=\"justify\""), "{xml}");
        xml.replacen(
            r#"horizontal="justify""#,
            r#"horizontal="justify" justifyLastLine="1""#,
            1,
        )
    });
    export_ni(&last_ni);
}

#[test]
fn justify_number_and_bool_without_soft_wrap_match_left() {
    let num_left = create_sheet("num-left", json!([{ "ref": "A1", "value": 12.5 }]));
    apply_align(&num_left, wide_col_ops("A1", "left", json!({})));
    let num_just = create_sheet("num-just", json!([{ "ref": "A1", "value": 12.5 }]));
    apply_align(&num_just, wide_col_ops("A1", "justify", json!({})));
    assert_eq!(
        export_ok(&num_left),
        export_ok(&num_just),
        "no-wrap numeric justify follows the same left fallback as text"
    );

    let bool_left = create_sheet("bool-left", json!([{ "ref": "A1", "value": true }]));
    apply_align(&bool_left, wide_col_ops("A1", "left", json!({})));
    let bool_just = create_sheet("bool-just", json!([{ "ref": "A1", "value": true }]));
    apply_align(&bool_just, wide_col_ops("A1", "justify", json!({})));
    let bool_gen = create_sheet("bool-gen", json!([{ "ref": "A1", "value": true }]));
    apply_align(&bool_gen, wide_col_ops("A1", "general", json!({})));
    let just_png = export_ok(&bool_just);
    assert_eq!(
        export_ok(&bool_left),
        just_png,
        "no-wrap boolean justify is left, not general/center"
    );
    assert_ne!(export_ok(&bool_gen), just_png);
}

#[test]
fn justify_soft_wrap_stretches_non_last_lines_and_keeps_last_line_left() {
    let text = "one two three four five six";
    let left = create_sheet("wrap-left", json!([{ "ref": "A1", "value": text }]));
    apply_align(
        &left,
        json!([
            { "op": "setCellStyle", "id": "cell:Sheet1!A1", "align": { "horizontal": "left", "wrap": true, "vertical": "top" } },
            { "op": "setColWidth", "id": "sheet:Sheet1", "col": "A", "width": 8.0 },
            { "op": "setRowHeight", "id": "sheet:Sheet1", "row": 1, "height": 60.0 }
        ]),
    );
    let just = create_sheet("wrap-just", json!([{ "ref": "A1", "value": text }]));
    apply_align(
        &just,
        json!([
            { "op": "setCellStyle", "id": "cell:Sheet1!A1", "align": { "horizontal": "justify", "wrap": true, "vertical": "top" } },
            { "op": "setColWidth", "id": "sheet:Sheet1", "col": "A", "width": 8.0 },
            { "op": "setRowHeight", "id": "sheet:Sheet1", "row": 1, "height": 60.0 }
        ]),
    );
    let left_png = export_ok(&left);
    let just_png = export_ok(&just);
    assert_ne!(
        left_png, just_png,
        "soft-wrapped justify must not collapse to packed left on every line"
    );
    let (w, h, left_rgb) = png_rgb(&left_png);
    let (jw, jh, just_rgb) = png_rgb(&just_png);
    assert_eq!((w, h), (jw, jh));
    let bands = dark_row_bands(&left_rgb, w, h);
    assert!(
        bands.len() >= 2,
        "expected at least two wrapped lines, got {bands:?}"
    );
    let (ly0, ly1) = *bands.last().unwrap();
    let last_left = dark_xs_in_rows(&left_rgb, w, ly0, ly1);
    let last_just = dark_xs_in_rows(&just_rgb, w, ly0, ly1);
    assert_eq!(
        last_left, last_just,
        "last line of a wrapped justified paragraph stays left"
    );
    let (fy0, fy1) = bands[0];
    let first_left_max = dark_xs_in_rows(&left_rgb, w, fy0, fy1).1;
    let first_just_max = dark_xs_in_rows(&just_rgb, w, fy0, fy1).1;
    assert!(
        first_just_max > first_left_max,
        "non-last justified line should extend farther right; left_max={first_left_max} just_max={first_just_max}"
    );
}

#[test]
fn distributed_two_words_are_flush_left_and_right() {
    let dist = create_sheet("dist-two", json!([{ "ref": "A1", "value": "ab cd" }]));
    apply_align(&dist, wide_col_ops("A1", "distributed", json!({})));
    let left = create_sheet("dist-left", json!([{ "ref": "A1", "value": "ab cd" }]));
    apply_align(&left, wide_col_ops("A1", "left", json!({})));
    let dist_png = export_ok(&dist);
    let left_png = export_ok(&left);
    assert_ne!(
        dist_png, left_png,
        "two-word distributed must not collapse to a packed left line"
    );
    let (w, h, dist_rgb) = png_rgb(&dist_png);
    let (lw, lh, left_rgb) = png_rgb(&left_png);
    assert_eq!((w, h), (lw, lh));
    let mid = w / 2;
    let dist_right = dark_count(&dist_rgb, w, mid, w, h);
    let left_right = dark_count(&left_rgb, w, mid, w, h);
    assert!(
        dist_right > left_right,
        "second distributed word should sit on the right; dist_right={dist_right} left_right={left_right}"
    );
    let dist_left = dark_count(&dist_rgb, w, 0, w / 4, h);
    assert!(
        dist_left > 0,
        "first distributed word should still sit on the left"
    );
}

#[test]
fn distributed_not_at_least_two_words_stays_ni() {
    for (tag, value) in [("one", "abcd"), ("empty-ish", "ab")] {
        let path = create_sheet(tag, json!([{ "ref": "A1", "value": value }]));
        apply_align(&path, wide_col_ops("A1", "distributed", json!({})));
        export_ni(&path);
    }
}

#[test]
fn distributed_three_words_span_left_mid_and_right() {
    let dist = create_sheet("dist-three", json!([{ "ref": "A1", "value": "ab cd ef" }]));
    apply_align(&dist, wide_col_ops("A1", "distributed", json!({})));
    let left = create_sheet(
        "dist-three-left",
        json!([{ "ref": "A1", "value": "ab cd ef" }]),
    );
    apply_align(&left, wide_col_ops("A1", "left", json!({})));
    let dist_png = export_ok(&dist);
    let left_png = export_ok(&left);
    assert_ne!(dist_png, left_png);
    let (w, h, dist_rgb) = png_rgb(&dist_png);
    let (lw, lh, left_rgb) = png_rgb(&left_png);
    assert_eq!((w, h), (lw, lh));
    let third = w / 3;
    let dist_mid = dark_count(&dist_rgb, w, third, 2 * third, h);
    let left_mid = dark_count(&left_rgb, w, third, 2 * third, h);
    let dist_right = dark_count(&dist_rgb, w, 2 * third, w, h);
    let left_right = dark_count(&left_rgb, w, 2 * third, w, h);
    assert!(
        dist_mid > left_mid,
        "middle word should sit near the center; dist_mid={dist_mid} left_mid={left_mid}"
    );
    assert!(
        dist_right > left_right,
        "third word should sit on the right; dist_right={dist_right} left_right={left_right}"
    );
    assert!(dark_count(&dist_rgb, w, 0, third, h) > 0);
}

#[test]
fn fill_center_continuous_and_vertical_justify_stay_ni() {
    for (tag, align) in [
        ("fill", json!({ "horizontal": "fill" })),
        ("cc", json!({ "horizontal": "centerContinuous" })),
        (
            "vjust",
            json!({ "horizontal": "left", "vertical": "justify" }),
        ),
        (
            "vdist",
            json!({ "horizontal": "left", "vertical": "distributed" }),
        ),
    ] {
        let path = create_sheet(tag, json!([{ "ref": "A1", "value": "hello" }]));
        apply_align(
            &path,
            json!([
                { "op": "setCellStyle", "id": "cell:Sheet1!A1", "align": align },
                { "op": "setColWidth", "id": "sheet:Sheet1", "col": "A", "width": 16.0 }
            ]),
        );
        export_ni(&path);
    }
}

#[test]
fn nonzero_text_rotation_on_text_stays_ni() {
    for rot in [45, 90, 180] {
        let path = create_sheet(
            &format!("rot-{rot}"),
            json!([{ "ref": "A1", "value": "hello" }]),
        );
        apply_align(
            &path,
            wide_col_ops("A1", "left", json!({ "rotation": rot })),
        );
        export_ni(&path);
    }
    // 255 是 Excel 堆积竖排，apply 的 rotation 只写 0–180，这里直接改部件。
    let path = create_sheet("rot-255", json!([{ "ref": "A1", "value": "hello" }]));
    apply_align(&path, wide_col_ops("A1", "left", json!({})));
    let stacked = tmp("rot-255-xml");
    rewrite_part(&path, &stacked, "xl/styles.xml", |xml| {
        if let Some(idx) = xml.find("<alignment") {
            if xml[idx..].starts_with("<alignment/>") {
                return xml.replacen("<alignment/>", r#"<alignment textRotation="255"/>"#, 1);
            }
            return xml.replacen("<alignment", r#"<alignment textRotation="255""#, 1);
        }
        xml.replacen("</xf>", r#"<alignment textRotation="255"/></xf>"#, 1)
    });
    export_ni(&stacked);
}

#[test]
fn unused_rotated_style_does_not_fail_the_sheet() {
    let path = create_sheet("unused-rot", json!([{ "ref": "A1", "value": "hello" }]));
    apply_align(&path, wide_col_ops("A1", "left", json!({})));
    let before = export_ok(&path);
    let next = tmp("unused-rot-xml");
    rewrite_part(&path, &next, "xl/styles.xml", |xml| {
        let xf = r#"<xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0" applyAlignment="1"><alignment textRotation="90"/></xf>"#;
        if let Some(idx) = xml.rfind("</cellXfs>") {
            let mut out = String::new();
            out.push_str(&xml[..idx]);
            out.push_str(xf);
            out.push_str(&xml[idx..]);
            // bump count if present
            if let Some(start) = out.find(r#"<cellXfs count=""#) {
                let digit_start = start + "<cellXfs count=\"".len();
                let digit_end = out[digit_start..].find('"').unwrap() + digit_start;
                let n: u32 = out[digit_start..digit_end].parse().unwrap();
                out.replace_range(digit_start..digit_end, &(n + 1).to_string());
            }
            return out;
        }
        panic!("missing cellXfs: {xml}");
    });
    let after = export_ok(&next);
    assert_eq!(before, after);
}

#[test]
fn wrap_indent_ordinary_left_is_unchanged_beside_justify() {
    let path = create_sheet(
        "wrap-keep",
        json!([
            { "ref": "A1", "value": "one two three four five" },
            { "ref": "B1", "value": "ok" }
        ]),
    );
    apply_align(
        &path,
        json!([
            { "op": "setCellStyle", "id": "cell:Sheet1!A1", "align": { "wrap": true, "indent": 1, "horizontal": "left", "vertical": "top" } },
            { "op": "setCellStyle", "id": "cell:Sheet1!B1", "align": { "horizontal": "justify", "vertical": "top" } },
            { "op": "setColWidth", "id": "sheet:Sheet1", "col": "A", "width": 10.625 },
            { "op": "setColWidth", "id": "sheet:Sheet1", "col": "B", "width": 12.0 },
            { "op": "setRowHeight", "id": "sheet:Sheet1", "row": 1, "height": 45.0 }
        ]),
    );
    let png = export_ok(&path);
    assert!(png.starts_with(&[0x89, b'P', b'N', b'G']));
    assert!(png.len() > 100);
}
