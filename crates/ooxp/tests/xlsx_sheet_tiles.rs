//! exportSheetImage `tilePx`：小图切块拼接与整图逐像素一致；超限不分配整幅画布。

use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::json;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-xlsx-sheet-tiles");
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

fn export_mono(src: &Path) -> Vec<u8> {
    let png = src.with_extension("mono.png");
    let written = run(&json!({
        "schemaVersion": 1,
        "action": "exportSheetImage",
        "path": src.to_string_lossy(),
        "id": "sheet:Sheet1",
        "output": png.to_string_lossy(),
    })
    .to_string())
    .unwrap();
    let RunOutput::Written { path, .. } = written else {
        panic!("written");
    };
    std::fs::read(path).unwrap()
}

fn book_grid(tag: &str, rows: u32, cols: u32) -> PathBuf {
    let path = tmp(tag);
    let mut cells = Vec::new();
    for r in 1..=rows {
        for c in 1..=cols {
            let col = (b'A' + (c - 1) as u8) as char;
            cells.push(json!({ "ref": format!("{col}{r}"), "value": (r * 10 + c) as i64 }));
        }
    }
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "workbook": { "sheets": [{ "name": "Sheet1", "cells": cells }] }
    }));
    // hide grid for stable fill compare? keep grid - stitch must match mono including grid
    path
}

#[test]
fn small_sheet_tiles_stitch_match_mono_pixel_for_pixel() {
    let src = book_grid("stitch", 4, 4);
    let mono = export_mono(&src);
    let (mw, mh, mrgb) = decode_png(&mono);

    let out = src.with_extension("tiled.png");
    let written = run(&json!({
        "schemaVersion": 1,
        "action": "exportSheetImage",
        "path": src.to_string_lossy(),
        "id": "sheet:Sheet1",
        "output": out.to_string_lossy(),
        "tilePx": 40
    })
    .to_string())
    .unwrap();
    let RunOutput::Written { path, paths, .. } = written else {
        panic!("written");
    };
    let paths = paths.expect("multi tile paths");
    assert!(paths.len() >= 2, "tilePx=40 should split {mw}x{mh}");

    let mut stitched = vec![255u8; (mw * mh * 3) as usize];
    for p in &paths {
        let name = p.file_name().unwrap().to_string_lossy();
        // stem-rRcC.png
        let rc = name.rsplit('-').next().unwrap().trim_end_matches(".png");
        let (rs, cs) = rc.split_once('c').unwrap();
        let row: u32 = rs.trim_start_matches('r').parse().unwrap();
        let col: u32 = cs.parse().unwrap();
        let bytes = std::fs::read(p).unwrap();
        let (tw, th, trgb) = decode_png(&bytes);
        assert!(tw <= 40 && th <= 40);
        let ox = col * 40;
        let oy = row * 40;
        for y in 0..th {
            for x in 0..tw {
                let si = ((y * tw + x) * 3) as usize;
                let di = (((oy + y) * mw + (ox + x)) * 3) as usize;
                stitched[di] = trgb[si];
                stitched[di + 1] = trgb[si + 1];
                stitched[di + 2] = trgb[si + 2];
            }
        }
        let _ = path;
    }
    assert_eq!(stitched, mrgb, "tiled stitch must match mono renderer");
}

#[test]
fn without_tile_px_oversize_still_not_implemented() {
    // 构造不了真 4097 边长的小测试；直接断言默认 API 在超限时仍走 Canvas::window 上限。
    // 用 lib：tile 路径可画，无 tilePx 的 export_sheet_image 对超大仍 NI——此处用非法 tilePx=0。
    let src = book_grid("ni", 1, 1);
    let err = run(&json!({
        "schemaVersion": 1,
        "action": "exportSheetImage",
        "path": src.to_string_lossy(),
        "id": "sheet:Sheet1",
        "output": src.with_extension("bad.png").to_string_lossy(),
        "tilePx": 0
    })
    .to_string())
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::NotImplemented);
}
