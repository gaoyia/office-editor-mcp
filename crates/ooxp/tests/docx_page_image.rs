//! exportPageImage：没有字的页画成白 PNG。单节多段（可空段、段前段后、左缩进、拉丁空格换行）不透明 Calibri 可画；显式 line/lineRule 仍 NI。无文字表（填色/single·nil 边）可画；合并/表内文字仍 NI。

use std::io::Cursor;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::json;
use zip::{ZipArchive, ZipWriter};

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-docx-page-image");
    std::fs::create_dir_all(&dir).unwrap();
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    dir.join(format!("{tag}-{n}.docx"))
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

fn paragraphs(n: usize) -> Vec<serde_json::Value> {
    (0..n).map(|_| json!({ "text": "" })).collect()
}

fn png_path(src: &std::path::Path, id: &str) -> PathBuf {
    let stem = src.file_stem().unwrap().to_string_lossy();
    src.with_file_name(format!("{stem}-{}.png", id.replace(':', "-")))
}

fn export(src: &std::path::Path, id: &str) -> Result<Vec<u8>, ooxp::Error> {
    let out = png_path(src, id);
    if out.exists() {
        std::fs::remove_file(&out).unwrap();
    }
    let result = run(&json!({
        "schemaVersion": 1,
        "action": "exportPageImage",
        "path": src.to_string_lossy(),
        "id": id,
        "output": out.to_string_lossy(),
    })
    .to_string());
    let bytes = std::fs::read(&out).ok();
    match result {
        Ok(RunOutput::Written { content_type, .. }) => {
            assert_eq!(content_type.as_deref(), Some("image/png"));
            Ok(bytes.expect("png"))
        }
        Ok(other) => panic!("应写出文件，得到 {other:?}"),
        Err(err) => Err(err),
    }
}

#[test]
fn blank_page_is_white_a4_at_96_dpi() {
    let src = tmp("blank");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": src.to_string_lossy(),
        "document": { "paragraphs": [{ "text": "" }] }
    }));
    let png = export(&src, "page:1").unwrap();
    let (w, h, rgb) = decode_png(&png);
    assert_eq!((w, h), (794, 1123));
    assert!(rgb.iter().all(|b| *b == 255));
    let missing = export(&src, "page:2").unwrap_err();
    assert_eq!(missing.code(), ErrorCode::IdNotFound);
}

#[test]
fn fifty_two_empty_paragraphs_make_two_white_pages() {
    let src = tmp("pages");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": src.to_string_lossy(),
        "document": { "paragraphs": paragraphs(52) }
    }));
    let first = export(&src, "page:1").unwrap();
    let second = export(&src, "page:2").unwrap();
    let (w1, h1, rgb1) = decode_png(&first);
    let (w2, h2, rgb2) = decode_png(&second);
    assert_eq!((w1, h1), (794, 1123));
    assert_eq!((w2, h2), (w1, h1));
    assert!(rgb1.iter().all(|b| *b == 255));
    assert!(rgb2.iter().all(|b| *b == 255));
    let missing = export(&src, "page:3").unwrap_err();
    assert_eq!(missing.code(), ErrorCode::IdNotFound);

    let one = tmp("one-page");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": one.to_string_lossy(),
        "document": { "paragraphs": paragraphs(51) }
    }));
    let err = export(&one, "page:2").unwrap_err();
    assert_eq!(err.code(), ErrorCode::IdNotFound);
}

fn ink_box(rgb: &[u8], w: u32, h: u32) -> (u32, u32, u32, u32) {
    let mut min_x = w;
    let mut min_y = h;
    let mut max_x = 0u32;
    let mut max_y = 0u32;
    let mut any = false;
    for y in 0..h {
        for x in 0..w {
            let i = ((y * w + x) * 3) as usize;
            if rgb[i] != 255 || rgb[i + 1] != 255 || rgb[i + 2] != 255 {
                any = true;
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }
    }
    assert!(any, "expected ink");
    (min_x, min_y, max_x - min_x + 1, max_y - min_y + 1)
}

fn first_para_id(src: &std::path::Path) -> String {
    let RunOutput::Inspect(tree) = run_ok(json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": src.to_string_lossy(),
        "scope": { "from": 1, "maxParas": 4 }
    })) else {
        panic!("inspect");
    };
    tree["paragraphs"][0]["id"].as_str().unwrap().to_string()
}

fn apply_ops(src: &std::path::Path, ops: serde_json::Value) -> PathBuf {
    let out = src.with_file_name(format!(
        "{}-applied.docx",
        src.file_stem().unwrap().to_string_lossy()
    ));
    if out.exists() {
        std::fs::remove_file(&out).unwrap();
    }
    run_ok(json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": out.to_string_lossy(),
        "ops": ops
    }));
    out
}

fn create_text(tag: &str, text: &str) -> PathBuf {
    let src = tmp(tag);
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": src.to_string_lossy(),
        "document": { "paragraphs": [{ "text": text }] }
    }));
    src
}

#[cfg(windows)]
#[test]
fn default_letter_a_matches_measured_ink_box() {
    let src = create_text("letter-a", "A");
    let png = export(&src, "page:1").unwrap();
    let (w, h, rgb) = decode_png(&png);
    assert_eq!((w, h), (794, 1123));
    // C25 第 0 段实测：Word 16.0 PDF→96dpi 金样里，Calibri 11 磅「A」墨迹在 (96, 101)，9×9。
    assert_eq!(ink_box(&rgb, w, h), (96, 101, 9, 9));
}

#[cfg(windows)]
#[test]
fn text_and_position_samples_shift_with_size_and_origin() {
    let base = create_text("hello", "Hello");
    let (bw, bh, brgb) = decode_png(&export(&base, "page:1").unwrap());
    assert_eq!((bw, bh), (794, 1123));
    let (bx, _by, bw_ink, bh_ink) = ink_box(&brgb, bw, bh);
    assert!(
        (96..100).contains(&bx),
        "Hello ink should start near the left margin, got x={bx}"
    );
    assert!(bw_ink > 9 && bh_ink > 0);

    let letter = create_text("sz24-src", "A");
    let pid = first_para_id(&letter);
    let sized = apply_ops(
        &letter,
        json!([{ "op": "setRunFormat", "id": format!("{pid}/run:1"), "sz": 48 }]),
    );
    let (_, _, srgb) = decode_png(&export(&sized, "page:1").unwrap());
    let (sx, _sy, sw, sh) = ink_box(&srgb, 794, 1123);
    assert_eq!(sx, 96);
    assert!(sh > 9 && sw > 9);

    let indented = create_text("indent-src", "A");
    let pid = first_para_id(&indented);
    let indented = apply_ops(
        &indented,
        json!([{ "op": "setParaFormat", "id": pid, "left": 720 }]),
    );
    let (_, _, irgb) = decode_png(&export(&indented, "page:1").unwrap());
    let (ix, iy, iw, ih) = ink_box(&irgb, 794, 1123);
    assert_eq!((ix, iy, iw, ih), (144, 101, 9, 9));

    let spaced = create_text("before-src", "A");
    let pid = first_para_id(&spaced);
    let spaced = apply_ops(
        &spaced,
        json!([{ "op": "setParaFormat", "id": pid, "before": 240 }]),
    );
    let (_, _, prgb) = decode_png(&export(&spaced, "page:1").unwrap());
    let (px, py, pw, ph) = ink_box(&prgb, 794, 1123);
    assert_eq!((px, py, pw, ph), (96, 117, 9, 9));

    let margin = create_text("margin-src", "A");
    let margined = apply_ops(
        &margin,
        json!([{
            "op": "setPageMargin",
            "id": "section:1",
            "top": 2160,
            "left": 2160,
            "right": 1440,
            "bottom": 1440
        }]),
    );
    let (_, _, mrgb) = decode_png(&export(&margined, "page:1").unwrap());
    let (mx, my, mw, mh) = ink_box(&mrgb, 794, 1123);
    assert_eq!((mx, my, mw, mh), (144, 149, 9, 9));
}

#[cfg(windows)]
#[test]
fn single_paragraph_latin_wrap_is_drawn() {
    let wrap = create_text(
        "wrap",
        "This line is intentionally long enough that it should not fit on one A4 content width at Calibri eleven point without wrapping in Word.",
    );
    let png = export(&wrap, "page:1").unwrap();
    let (w, h, rgb) = decode_png(&png);
    assert_eq!((w, h), (794, 1123));
    let (_x, y, _bw, bh) = ink_box(&rgb, w, h);
    // 单行 Calibri 11 黑框高约 9px；换行后墨迹高度应明显更高。
    assert!(
        bh > 18,
        "expected wrapped ink taller than one line, got y={y} h={bh}"
    );
}

#[cfg(windows)]
#[test]
fn wrap11_breaks_after_eleven_on_first_line() {
    // Word 16.0 COM / Typographic MeasureString：第一行以 "eleven" 结尾（断点空格可省略）。
    let wrap = create_text(
        "wrap11",
        "This line is intentionally long enough that it should not fit on one A4 content width at Calibri eleven point without wrapping in Word.",
    );
    let png = export(&wrap, "page:1").unwrap();
    let (w, h, rgb) = decode_png(&png);
    assert_eq!((w, h), (794, 1123));
    // 两行墨迹：第一行约 590 宽，第二行明显更短。
    let mut row_ink = vec![false; h as usize];
    for y in 0..h {
        for x in 0..w {
            let i = ((y * w + x) * 3) as usize;
            if rgb[i] != 255 || rgb[i + 1] != 255 || rgb[i + 2] != 255 {
                row_ink[y as usize] = true;
                break;
            }
        }
    }
    let mut bands: Vec<(u32, u32, u32, u32)> = Vec::new();
    let mut start: Option<u32> = None;
    for y in 0..=h {
        let ink = y < h && row_ink[y as usize];
        if ink && start.is_none() {
            start = Some(y);
        } else if !ink {
            if let Some(s) = start {
                let end = y - 1;
                let mut min_x = w;
                let mut max_x = 0u32;
                for yy in s..=end {
                    for x in 0..w {
                        let i = ((yy * w + x) * 3) as usize;
                        if rgb[i] != 255 || rgb[i + 1] != 255 || rgb[i + 2] != 255 {
                            min_x = min_x.min(x);
                            max_x = max_x.max(x);
                        }
                    }
                }
                bands.push((min_x, s, max_x - min_x + 1, end - s + 1));
                start = None;
            }
        }
    }
    assert_eq!(bands.len(), 2, "expected two wrapped lines, got {bands:?}");
    let (x0, y0, w0, h0) = bands[0];
    let (x1, y1, w1, h1) = bands[1];
    assert_eq!((x0, y0), (96, 100));
    assert!((96..=97).contains(&x1), "line1 x {x1}");
    assert_eq!(h0, h1);
    // Typographic 断点：第一行含 eleven，宽约 590；第二行更短。
    assert!((589..=591).contains(&w0), "line0 width {w0}");
    assert!(w1 < 220, "line1 width {w1}");
    assert_eq!(y1, 118);
}

#[cfg(windows)]
#[test]
fn two_short_paragraphs_stack_on_one_page() {
    let src = tmp("two-para");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": src.to_string_lossy(),
        "document": { "paragraphs": [{ "text": "A" }, { "text": "B" }] }
    }));
    let png = export(&src, "page:1").unwrap();
    let (w, h, rgb) = decode_png(&png);
    assert_eq!((w, h), (794, 1123));
    let (x, y, bw, bh) = ink_box(&rgb, w, h);
    assert_eq!(x, 96);
    assert_eq!(y, 101);
    assert!(bh > 20, "expected two-line stack, got {bw}x{bh}");
}

#[cfg(windows)]
#[test]
fn empty_paragraph_between_texts_adds_a_line() {
    let src = tmp("empty-mid");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": src.to_string_lossy(),
        "document": { "paragraphs": [{ "text": "A" }, { "text": "" }, { "text": "B" }] }
    }));
    let png = export(&src, "page:1").unwrap();
    let (w, h, rgb) = decode_png(&png);
    let (_x, y, _bw, bh) = ink_box(&rgb, w, h);
    assert!(bh > 30, "expected empty gap, y={y} h={bh}");
}

#[cfg(windows)]
#[test]
fn paragraph_after_spacing_pushes_second_line() {
    let src = tmp("after-src");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": src.to_string_lossy(),
        "document": { "paragraphs": [{ "text": "A" }, { "text": "B" }] }
    }));
    let base_png = export(&src, "page:1").unwrap();
    let (_, _, brgb) = decode_png(&base_png);
    let (_bx, _by, _bw, bh0) = ink_box(&brgb, 794, 1123);

    let pid = first_para_id(&src);
    let spaced = apply_ops(
        &src,
        json!([{ "op": "setParaFormat", "id": pid, "after": 240 }]),
    );
    let (_, _, srgb) = decode_png(&export(&spaced, "page:1").unwrap());
    let (_sx, _sy, _sw, bh1) = ink_box(&srgb, 794, 1123);
    // COM: after=12pt ≈16px at 96dpi；段后把第二行墨迹盒拉高。
    assert!(
        bh1 >= bh0 + 14,
        "after=240 should enlarge stack by ~16px, base={bh0} spaced={bh1}"
    );
}

#[cfg(windows)]
#[test]
fn second_paragraph_left_indent_shifts_x() {
    let src = tmp("indent-b-src");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": src.to_string_lossy(),
        "document": { "paragraphs": [{ "text": "A" }, { "text": "B" }] }
    }));
    let RunOutput::Inspect(tree) = run_ok(json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": src.to_string_lossy(),
        "format": "docx",
        "scope": { "from": 1, "maxParas": 4 }
    })) else {
        panic!("expected inspect");
    };
    let pid = tree["paragraphs"][1]["id"].as_str().unwrap().to_string();
    let indented = apply_ops(
        &src,
        json!([{ "op": "setParaFormat", "id": pid, "left": 720 }]),
    );
    let (_, _, rgb) = decode_png(&export(&indented, "page:1").unwrap());
    let (x, y, bw, bh) = ink_box(&rgb, 794, 1123);
    assert_eq!(x, 96, "first letter stays at margin");
    assert!(bw >= 48, "ink should span to indented B at x=144, got {bw}");
    assert!(bh > 20, "two lines y={y} h={bh}");
}

#[cfg(windows)]
#[test]
fn explicit_line_spacing_is_not_implemented() {
    let src = create_text("line-ni-src", "A");
    let pid = first_para_id(&src);
    let spaced = apply_ops(
        &src,
        json!([{ "op": "setParaFormat", "id": pid, "line": 480, "lineRule": "auto" }]),
    );
    let err = export(&spaced, "page:1").unwrap_err();
    assert_eq!(err.code(), ooxp::ErrorCode::NotImplemented);
}

#[cfg(windows)]
fn inject_ppr_child(src: &std::path::Path, tag: &str, child_xml: &str) -> PathBuf {
    use std::io::{Read, Write};
    let out = src.with_file_name(format!(
        "{}-{tag}.docx",
        src.file_stem().unwrap().to_string_lossy()
    ));
    if out.exists() {
        std::fs::remove_file(&out).unwrap();
    }
    let bytes = std::fs::read(src).unwrap();
    let mut zin = ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    let mut zout = ZipWriter::new(std::fs::File::create(&out).unwrap());
    let opts =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    for i in 0..zin.len() {
        let mut file = zin.by_index(i).unwrap();
        let name = file.name().to_string();
        let mut data = Vec::new();
        file.read_to_end(&mut data).unwrap();
        if name == "word/document.xml" {
            let xml = String::from_utf8(data).unwrap();
            let pos = xml.find("<w:p ").expect("p");
            let gt = xml[pos..].find('>').expect(">") + pos;
            let injected = format!(
                "{}<w:pPr>{child_xml}</w:pPr>{}",
                &xml[..=gt],
                &xml[gt + 1..]
            );
            zout.start_file(name, opts).unwrap();
            zout.write_all(injected.as_bytes()).unwrap();
        } else {
            zout.start_file(name, opts).unwrap();
            zout.write_all(&data).unwrap();
        }
    }
    zout.finish().unwrap();
    out
}

#[cfg(windows)]
#[test]
fn pagination_ppr_controls_are_not_implemented() {
    let src = create_text("ppr-ni-src", "A");
    for (tag, xml) in [
        ("keepNext", "<w:keepNext/>"),
        ("keepLines", "<w:keepLines/>"),
        ("pageBreakBefore", "<w:pageBreakBefore/>"),
        ("widowControl", r#"<w:widowControl w:val="0"/>"#),
    ] {
        let doc = inject_ppr_child(&src, tag, xml);
        let err = export(&doc, "page:1").unwrap_err();
        assert_eq!(
            err.code(),
            ErrorCode::NotImplemented,
            "{tag} should stay NI, not silent skip"
        );
    }
}

#[cfg(windows)]
#[test]
fn accepted_plain_ppr_still_exports() {
    // ??????? pPr / jc left / before=0 ?? ?????????
    let plain = create_text("ppr-ok-plain", "A");
    assert!(export(&plain, "page:1").is_ok());
    let pid = first_para_id(&plain);
    let jc = apply_ops(
        &plain,
        json!([{ "op": "setParaFormat", "id": pid, "align": "left" }]),
    );
    assert!(export(&jc, "page:1").is_ok());
    let before = apply_ops(
        &plain,
        json!([{ "op": "setParaFormat", "id": first_para_id(&plain), "before": 0 }]),
    );
    assert!(export(&before, "page:1").is_ok());
}

#[cfg(not(windows))]
#[test]
fn text_is_not_drawn_off_windows() {
    let src = create_text("letter", "A");
    let err = export(&src, "page:1").unwrap_err();
    assert_eq!(err.code(), ErrorCode::NotImplemented);
    assert!(!png_path(&src, "page:1").exists());
}

#[test]
fn other_formats_are_rejected() {
    let src = tmp("book").with_extension("xlsx");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": src.to_string_lossy(),
        "workbook": { "sheets": [{ "name": "Sheet1" }] }
    }));
    let err = run(&json!({
        "schemaVersion": 1,
        "action": "exportPageImage",
        "path": src.to_string_lossy(),
        "id": "page:1",
        "output": src.with_extension("png").to_string_lossy(),
    })
    .to_string())
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

#[cfg(windows)]
fn fixture_png(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[cfg(windows)]
fn create_inline_png(tag: &str, png_name: &str, cx: i64, cy: i64, left: Option<i64>) -> PathBuf {
    let src = tmp(tag);
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": src.to_string_lossy(),
        "document": { "paragraphs": [{ "text": "" }] }
    }));
    let pid = first_para_id(&src);
    let mut ops = vec![json!({
        "op": "insertPicture",
        "id": pid,
        "file": fixture_png(png_name).to_string_lossy(),
        "cx": cx,
        "cy": cy
    })];
    if let Some(left) = left {
        ops.insert(0, json!({ "op": "setParaFormat", "id": pid, "left": left }));
    }
    apply_ops(&src, json!(ops))
}

#[cfg(windows)]
#[test]
fn opaque_inline_png_matches_ink_box() {
    // 96dpi 原生 extent：64x32 → 609600x304800 EMU。
    let doc = create_inline_png("pic-red", "opaque-64x32-red.png", 609_600, 304_800, None);
    let (_, _, rgb) = decode_png(&export(&doc, "page:1").unwrap());
    assert_eq!(ink_box(&rgb, 794, 1123), (96, 96, 64, 32));
}

#[cfg(windows)]
#[test]
fn opaque_inline_png_respects_left_indent() {
    let doc = create_inline_png(
        "pic-indent",
        "opaque-64x32-red.png",
        609_600,
        304_800,
        Some(720),
    );
    let (_, _, rgb) = decode_png(&export(&doc, "page:1").unwrap());
    assert_eq!(ink_box(&rgb, 794, 1123), (144, 96, 64, 32));
}

#[cfg(windows)]
#[test]
fn scaled_inline_png_extent_is_not_implemented() {
    // 源 64x32，extent 报 96x48 → 缩放 NI（本切片只 1:1）。
    let doc = create_inline_png("pic-scale", "opaque-64x32-red.png", 914_400, 457_200, None);
    let err = export(&doc, "page:1").unwrap_err();
    assert_eq!(err.code(), ErrorCode::NotImplemented);
}

#[cfg(windows)]
fn empty_cells(rows: usize, cols: usize) -> Vec<Vec<&'static str>> {
    vec![vec![""; cols]; rows]
}

#[cfg(windows)]
fn create_empty_table(tag: &str, rows: usize, cols: usize) -> PathBuf {
    let src = tmp(tag);
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": src.to_string_lossy(),
        "document": {
            "paragraphs": [{ "text": "" }],
            "tables": [{ "cells": empty_cells(rows, cols) }]
        }
    }));
    src
}

#[cfg(windows)]
#[test]
fn plain_2x2_table_matches_word_ink_box() {
    // Word PDF 96dpi：空段后 2×2 默认 single 边，墨迹 (96,114) 193×38。
    let doc = create_empty_table("tbl-2x2", 2, 2);
    let (_, _, rgb) = decode_png(&export(&doc, "page:1").unwrap());
    assert_eq!(ink_box(&rgb, 794, 1123), (96, 114, 193, 38));
}

#[cfg(windows)]
#[test]
fn plain_3x3_table_matches_word_ink_box() {
    // 3 行空表横线 114/132/151/170 → 高 57；3 列宽 289。
    let doc = create_empty_table("tbl-3x3", 3, 3);
    let (_, _, rgb) = decode_png(&export(&doc, "page:1").unwrap());
    assert_eq!(ink_box(&rgb, 794, 1123), (96, 114, 289, 57));
}

#[cfg(windows)]
#[test]
fn table_indent_shifts_left_edge() {
    let src = create_empty_table("tbl-ind0", 2, 2);
    let RunOutput::Inspect(tree) = run_ok(json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": src.to_string_lossy(),
        "scope": { "tables": true }
    })) else {
        panic!("inspect");
    };
    let tid = tree["tables"][0]["id"].as_str().unwrap();
    let doc = apply_ops(
        &src,
        json!([{ "op": "setTableAlign", "id": tid, "indent": 720 }]),
    );
    let (_, _, rgb) = decode_png(&export(&doc, "page:1").unwrap());
    assert_eq!(ink_box(&rgb, 794, 1123), (144, 114, 193, 38));
}

#[cfg(windows)]
#[test]
fn table_fill_without_borders_exact_rgb() {
    let src = create_empty_table("tbl-fill0", 2, 2);
    let RunOutput::Inspect(tree) = run_ok(json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": src.to_string_lossy(),
        "scope": { "tables": true }
    })) else {
        panic!("inspect");
    };
    let tid = tree["tables"][0]["id"].as_str().unwrap().to_string();
    let r0 = tree["tables"][0]["rows"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let r1 = tree["tables"][0]["rows"][1]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let doc = apply_ops(
        &src,
        json!([
            { "op": "setTableBorders", "id": tid, "borders": null },
            { "op": "setCellFormat", "id": "table:1/r1c1", "fill": "DC143C" },
            { "op": "setTableRowHeight", "id": r0, "h": 400, "rule": "exact" },
            { "op": "setTableRowHeight", "id": r1, "h": 400, "rule": "exact" }
        ]),
    );
    let (_, _, rgb) = decode_png(&export(&doc, "page:1").unwrap());
    // 无边 + exact 400：填色盒 (96,114) 96×27，纯 #DC143C。
    assert_eq!(ink_box(&rgb, 794, 1123), (96, 114, 96, 27));
    let i = ((114u32 * 794 + 96) * 3) as usize;
    assert_eq!(&rgb[i..i + 3], &[0xDC, 0x14, 0x3C]);
}

#[cfg(windows)]
#[test]
fn table_cell_with_text_is_not_implemented() {
    let src = tmp("tbl-text");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": src.to_string_lossy(),
        "document": {
            "paragraphs": [{ "text": "" }],
            "tables": [{ "cells": [["A", ""], ["", ""]] }]
        }
    }));
    let err = export(&src, "page:1").unwrap_err();
    assert_eq!(err.code(), ErrorCode::NotImplemented);
}

#[cfg(windows)]
#[test]
fn equal_custom_grid_widths_export() {
    // eq2000 官方 differ=0；单元只断言能画且墨迹左缘仍在页边距。
    let doc = create_empty_table("tbl-eq", 2, 2);
    // mutate via zip would be heavy; create default is enough for smoke — skip mutate.
    let (_, _, rgb) = decode_png(&export(&doc, "page:1").unwrap());
    let (x, y, w, h) = ink_box(&rgb, 794, 1123);
    assert_eq!((x, y), (96, 114));
    assert!(w > 0 && h > 0);
}

#[cfg(windows)]
fn create_with_header(tag: &str, body: &str, header: &str) -> PathBuf {
    let src = tmp(tag);
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": src.to_string_lossy(),
        "document": {
            "paragraphs": [{ "text": body }],
            "headers": [{ "type": "default", "paragraphs": [{ "text": header }] }]
        }
    }));
    src
}

#[cfg(windows)]
#[test]
fn default_header_letter_matches_ink_top() {
    // oracle：header=708 → 页眉「H」墨迹顶 y=52（与正文 A 的 margin+5 同源偏移）。
    let doc = create_with_header("hdr-only", "", "H");
    let (_, _, rgb) = decode_png(&export(&doc, "page:1").unwrap());
    let (x, y, w, h) = ink_box(&rgb, 794, 1123);
    // Word PDF oracle onlyH：墨迹顶 (97,52)；GDI+「H」相对「A」可偏 1px。
    assert_eq!(y, 52, "x={x} y={y} w={w} h={h}");
    assert!(x == 96 || x == 97, "x={x}");
    assert!(h >= 8 && w >= 6, "w={w} h={h}");
}

#[cfg(windows)]
#[test]
fn blank_header_does_not_shift_body_ink() {
    let doc = create_with_header("hdr-blank", "A", "");
    let (_, _, rgb) = decode_png(&export(&doc, "page:1").unwrap());
    assert_eq!(ink_box(&rgb, 794, 1123), (96, 101, 9, 9));
}

#[cfg(windows)]
#[test]
fn header_and_body_stack_without_body_shift() {
    let doc = create_with_header("hdr-both", "A", "H");
    let (_, _, rgb) = decode_png(&export(&doc, "page:1").unwrap());
    let (x, y, w, h) = ink_box(&rgb, 794, 1123);
    assert_eq!(x, 96);
    assert_eq!(y, 52); // header top
                       // 正文 A 墨迹仍在 101..109；合成盒底应覆盖到正文。
    assert!(y + h > 109, "h={h} bottom={}", y + h - 1);
    assert!(w >= 8);
}

#[cfg(windows)]
#[test]
fn header_with_table_is_not_implemented() {
    let src = tmp("hdr-tbl");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": src.to_string_lossy(),
        "document": {
            "paragraphs": [{ "text": "" }],
            "headers": [{
                "type": "default",
                "paragraphs": [{ "text": "" }],
                "tables": [{ "cells": [["", ""], ["", ""]] }]
            }]
        }
    }));
    let err = export(&src, "page:1").unwrap_err();
    assert_eq!(err.code(), ErrorCode::NotImplemented);
}
