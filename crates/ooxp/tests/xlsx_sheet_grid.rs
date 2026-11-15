//! 关掉网格后，网格像素留白，填充和边框仍在原来的位置。

use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, RunOutput};
use serde_json::json;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

static SEQ: AtomicU32 = AtomicU32::new(0);

const GRID: [u8; 3] = [224, 224, 224];

fn tmp(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-xlsx-sheet-grid");
    std::fs::create_dir_all(&dir).unwrap();
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    dir.join(format!("{tag}-{n}.xlsx"))
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

fn pixel(rgb: &[u8], width: u32, x: u32, y: u32) -> [u8; 3] {
    let i = ((y * width + x) * 3) as usize;
    [rgb[i], rgb[i + 1], rgb[i + 2]]
}

fn book(tag: &str) -> PathBuf {
    let path = tmp(tag);
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "workbook": {
            "sheets": [{
                "name": "Sheet1",
                "cells": [
                    { "ref": "A1", "value": "" },
                    { "ref": "B1", "value": "" },
                    { "ref": "A2", "value": "" }
                ]
            }]
        }
    }));
    path
}

fn styled(tag: &str) -> PathBuf {
    let src = book(tag);
    let out = tmp("styled");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": out.to_string_lossy(),
        "ops": [{
            "op": "setCellStyle",
            "id": "cell:Sheet1!A1",
            "fill": { "pattern": "solid", "fg": { "rgb": "FFEE4455" } },
            "border": {
                "left": { "style": "thin", "color": { "rgb": "FF000000" } },
                "top": { "style": "thin", "color": { "rgb": "FF112233" } },
                "right": { "style": "medium", "color": { "rgb": "FF00AA66" } }
            }
        }]
    }));
    out
}

fn export(src: &Path) -> Vec<u8> {
    let png = src.with_extension("png");
    if png.exists() {
        std::fs::remove_file(&png).unwrap();
    }
    let written = run(&json!({
        "schemaVersion": 1,
        "action": "exportSheetImage",
        "path": src.to_string_lossy(),
        "id": "sheet:Sheet1",
        "output": png.to_string_lossy(),
    })
    .to_string())
    .unwrap();
    let RunOutput::Written {
        path, content_type, ..
    } = written
    else {
        panic!("应写出文件");
    };
    assert_eq!(content_type.as_deref(), Some("image/png"));
    assert_eq!(path, png);
    std::fs::read(png).unwrap()
}

fn rewrite_sheet(src: &Path, dst: &Path, edit: impl FnOnce(String) -> String) {
    let bytes = std::fs::read(src).unwrap();
    let mut zip_in = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut out = Cursor::new(Vec::new());
    let mut edit = Some(edit);
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
                data = edit.take().unwrap()(xml).into_bytes();
            }
            zip_out.start_file(&name, opt).unwrap();
            zip_out.write_all(&data).unwrap();
        }
        zip_out.finish().unwrap();
    }
    std::fs::write(dst, out.into_inner()).unwrap();
}

fn hide_grid(src: &Path, flag: &str) -> PathBuf {
    let dst = tmp("grid-off");
    let flag = flag.to_string();
    rewrite_sheet(src, &dst, move |xml| {
        assert!(!xml.contains("showGridLines"));
        xml.replace(
            "<sheetData>",
            &format!(
                "<sheetViews><sheetView showGridLines=\"{flag}\" workbookViewId=\"0\"/></sheetViews><sheetData>"
            ),
        )
    });
    dst
}

#[test]
fn grid_off_clears_grid_pixels_and_keeps_fill_and_borders() {
    let src = styled("keep");
    let on = export(&src);
    let off_path = hide_grid(&src, "0");
    let before = std::fs::read(&off_path).unwrap();
    let off = export(&off_path);
    assert_eq!(std::fs::read(&off_path).unwrap(), before);

    let (w, h, on_px) = decode_png(&on);
    let (w2, h2, off_px) = decode_png(&off);
    assert_eq!((w, h), (129, 41));
    assert_eq!((w2, h2), (w, h));
    assert_eq!(pixel(&on_px, w, 128, 10), GRID);
    assert_eq!(pixel(&on_px, w, 1, 1), [0xEE, 0x44, 0x55]);

    let mut gray = 0u32;
    for y in 0..h {
        for x in 0..w {
            let before = pixel(&on_px, w, x, y);
            let after = pixel(&off_px, w, x, y);
            if before == GRID {
                assert_eq!(after, [255, 255, 255], "网格像素 {x},{y}");
                gray += 1;
            } else {
                assert_eq!(before, after, "非网格像素 {x},{y}");
            }
        }
    }
    assert!(gray > 0);

    let as_false = hide_grid(&src, "false");
    let (wf, hf, false_px) = decode_png(&export(&as_false));
    assert_eq!((wf, hf), (w, h));
    assert_eq!(false_px, off_px);
}

#[test]
fn grid_colored_fill_stays_when_grid_is_off() {
    let src = book("gray-fill");
    let painted = tmp("gray-painted");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": painted.to_string_lossy(),
        "ops": [{
            "op": "setCellStyle",
            "id": "cell:Sheet1!B1",
            "fill": { "pattern": "solid", "fg": { "rgb": "FFE0E0E0" } }
        }]
    }));
    let off = hide_grid(&painted, "0");
    let (w, h, px) = decode_png(&export(&off));
    assert_eq!((w, h), (129, 41));
    assert_eq!(pixel(&px, w, 0, 10), [255, 255, 255]);
    assert_eq!(pixel(&px, w, 70, 10), GRID);
}

fn gap_book(tag: &str) -> PathBuf {
    let path = tmp(tag);
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "workbook": {
            "sheets": [{
                "name": "Sheet1",
                "cells": [
                    { "ref": "A1", "value": "" },
                    { "ref": "A3", "value": "" }
                ]
            }]
        }
    }));
    let painted = tmp("gap-painted");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": path.to_string_lossy(),
        "output": painted.to_string_lossy(),
        "ops": [
            {
                "op": "setCellStyle",
                "id": "cell:Sheet1!A1",
                "fill": { "pattern": "solid", "fg": { "rgb": "FFEE4455" } }
            },
            {
                "op": "setCellStyle",
                "id": "cell:Sheet1!A3",
                "fill": { "pattern": "solid", "fg": { "rgb": "FF112233" } }
            }
        ]
    }));
    hide_grid(&painted, "0")
}

fn with_format(src: &Path, format: &str) -> PathBuf {
    let dst = tmp("row-format");
    let format = format.to_string();
    rewrite_sheet(src, &dst, move |xml| {
        assert!(!xml.contains("sheetFormatPr"));
        xml.replace("<sheetData>", &format!("{format}<sheetData>"))
    });
    dst
}

#[test]
fn missing_default_row_height_is_fourteen_point_two_five() {
    let src = gap_book("gap");
    let sheet = std::fs::read(&src).unwrap();
    assert!(!String::from_utf8_lossy(&sheet).contains("defaultRowHeight"));
    let (w, h, px) = decode_png(&export(&src));
    assert_eq!((w, h), (65, 60));
    let red = [0xEE, 0x44, 0x55];
    let blue = [0x11, 0x22, 0x33];
    let white = [255, 255, 255];
    assert_eq!(pixel(&px, w, 0, 1), white);
    assert_eq!(pixel(&px, w, 1, 1), red);
    assert_eq!(pixel(&px, w, 1, 20), red);
    assert_eq!(pixel(&px, w, 1, 21), white);
    assert_eq!(pixel(&px, w, 1, 38), white);
    assert_eq!(pixel(&px, w, 1, 39), blue);
    assert_eq!(pixel(&px, w, 1, 59), blue);

    let ignored = with_format(&src, r#"<sheetFormatPr defaultRowHeight="18"/>"#);
    let (w, h, px) = decode_png(&export(&ignored));
    assert_eq!((w, h), (65, 61));
    assert_eq!(pixel(&px, w, 1, 20), red);
    assert_eq!(pixel(&px, w, 1, 21), white);
    assert_eq!(pixel(&px, w, 1, 39), white);
    assert_eq!(pixel(&px, w, 1, 40), blue);
    assert_eq!(pixel(&px, w, 1, 60), blue);

    let locked = with_format(
        &src,
        r#"<sheetFormatPr defaultRowHeight="12" customHeight="1"/>"#,
    );
    let (w, h, px) = decode_png(&export(&locked));
    assert_eq!((w, h), (65, 49));
    assert_eq!(pixel(&px, w, 1, 16), red);
    assert_eq!(pixel(&px, w, 1, 17), white);
    assert_eq!(pixel(&px, w, 1, 31), white);
    assert_eq!(pixel(&px, w, 1, 32), blue);
    assert_eq!(pixel(&px, w, 1, 48), blue);
}

#[test]
fn right_to_left_without_text_is_drawn() {
    let src = book("rtl");
    let dst = tmp("rtl-sheet");
    rewrite_sheet(&src, &dst, |xml| {
        xml.replace(
            "<sheetData>",
            "<sheetViews><sheetView rightToLeft=\"1\" showGridLines=\"0\" workbookViewId=\"0\"/></sheetViews><sheetData>",
        )
    });
    let (w, h, px) = decode_png(&export(&dst));
    assert_eq!((w, h), (129, 41));
    assert_eq!(pixel(&px, w, 0, 0), [255, 255, 255]);
}
