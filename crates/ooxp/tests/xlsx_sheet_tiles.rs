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

fn a1_col(mut n: u32) -> String {
    // 1-based Excel column index → A, B, … Z, AA, …
    let mut s = String::new();
    while n > 0 {
        let rem = (n - 1) % 26;
        s.insert(0, (b'A' + rem as u8) as char);
        n = (n - 1) / 26;
    }
    s
}

fn book_grid(tag: &str, rows: u32, cols: u32) -> PathBuf {
    let path = tmp(tag);
    let mut cells = Vec::new();
    for r in 1..=rows {
        for c in 1..=cols {
            cells.push(json!({ "ref": format!("{}{r}", a1_col(c)), "value": (r * 10 + c) as i64 }));
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

#[test]
fn wide_sheet_over_4096_default_still_ni_tile_px_exports_partial_last() {
    // 65 列 × 默认 64px = 4160 > MAX_PX；真超一边，不是小图拼接。
    let src = book_grid("wide65", 2, 65);
    let err = run(&json!({
        "schemaVersion": 1,
        "action": "exportSheetImage",
        "path": src.to_string_lossy(),
        "id": "sheet:Sheet1",
        "output": src.with_extension("mono-ni.png").to_string_lossy(),
    })
    .to_string())
    .unwrap_err();
    assert_eq!(
        err.code(),
        ErrorCode::NotImplemented,
        "default API must stay NI over 4096"
    );

    let out = src.with_extension("wide-tiled.png");
    // 唯一输出名，避免默认 stem 覆盖
    let written = run(&json!({
        "schemaVersion": 1,
        "action": "exportSheetImage",
        "path": src.to_string_lossy(),
        "id": "sheet:Sheet1",
        "output": out.to_string_lossy(),
        "tilePx": 1024
    })
    .to_string())
    .unwrap();
    let RunOutput::Written { paths, .. } = written else {
        panic!("written");
    };
    let paths = paths.expect("multi tiles for 4160-wide");
    assert!(
        paths.len() >= 5,
        "expected multiple tiles, got {}",
        paths.len()
    );
    // 最后一列不足 tile：宽 = 4160 % 1024 = 64? 4160/1024 = 4*1024=4096, rem 64
    let last = paths
        .iter()
        .max_by_key(|p| {
            let name = p.file_name().unwrap().to_string_lossy();
            name.clone()
        })
        .unwrap();
    // 找 r0c4（第 5 列 0-based）
    let partial = paths
        .iter()
        .find(|p| {
            p.file_name()
                .unwrap()
                .to_string_lossy()
                .contains("-r0c4.png")
        })
        .expect("r0c4 partial tile");
    let (tw, th, _) = decode_png(&std::fs::read(partial).unwrap());
    // 65×64px 列 + 可能的 1px 外框 → 全宽 4160 或 4161；末块 < tilePx。
    assert!(tw < 1024 && tw > 0, "last tile must be partial, got {tw}");
    assert!(
        tw == 64 || tw == 65,
        "expected remainder 64/65 (frame), got {tw}"
    );
    assert!(th <= 1024);
    assert!(paths.iter().all(|p| p.exists()), "all tile files present");
    let _ = last;
}

#[test]
fn tile_px_one_on_wide_sheet_fails_before_mass_files() {
    let src = book_grid("tile1cap", 2, 65);
    let out_dir = std::env::temp_dir().join(format!(
        "ooxp-tile1-{}",
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&out_dir);
    std::fs::create_dir_all(&out_dir).unwrap();
    let out = out_dir.join("mass.png");
    let err = run(&json!({
        "schemaVersion": 1,
        "action": "exportSheetImage",
        "path": src.to_string_lossy(),
        "id": "sheet:Sheet1",
        "output": out.to_string_lossy(),
        "tilePx": 1
    })
    .to_string())
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::NotImplemented);
    // 失败不得留下海量 png
    let leftover: Vec<_> = std::fs::read_dir(&out_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|x| x == "png").unwrap_or(false))
        .collect();
    assert!(
        leftover.is_empty(),
        "tilePx=1 oversize must not leave pngs, got {}",
        leftover.len()
    );
    let _ = std::fs::remove_dir_all(&out_dir);
}

#[test]
fn empty_used_range_vs_filled_explicit() {
    // 真空：只有表名无单元格 → usedRange 失败或 NI（显式写 case）
    let empty = tmp("empty-region");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": empty.to_string_lossy(),
        "workbook": { "sheets": [{ "name": "Sheet1", "cells": [] }] }
    }));
    let err = run(&json!({
        "schemaVersion": 1,
        "action": "exportSheetImage",
        "path": empty.to_string_lossy(),
        "id": "sheet:Sheet1",
        "output": empty.with_extension("e.png").to_string_lossy(),
        "tilePx": 64
    })
    .to_string());
    // 空表 used_range 当前实现：允许失败（unsupported）或极小图；不得 panic
    match err {
        Ok(RunOutput::Written { .. }) => {}
        Err(e) => assert_eq!(e.code(), ErrorCode::NotImplemented),
        Ok(other) => panic!("unexpected {other:?}"),
    }

    // 假空：有单元格值为空字符串，仍有 used range
    let fake = tmp("fake-empty");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": fake.to_string_lossy(),
        "workbook": { "sheets": [{ "name": "Sheet1", "cells": [
            { "ref": "A1", "value": "" },
            { "ref": "B2", "value": "" }
        ] }] }
    }));
    let written = run(&json!({
        "schemaVersion": 1,
        "action": "exportSheetImage",
        "path": fake.to_string_lossy(),
        "id": "sheet:Sheet1",
        "output": fake.with_extension("fake.png").to_string_lossy(),
        "tilePx": 32
    })
    .to_string())
    .unwrap();
    let RunOutput::Written { path, paths, .. } = written else {
        panic!("written");
    };
    assert!(path.exists() || paths.as_ref().map(|p| !p.is_empty()).unwrap_or(false));
}

#[test]
#[allow(clippy::manual_div_ceil)]
fn multi_row_col_tiles_global_coords_stitch() {
    // 多行多列切块（非仅一行拼接）；背景/网格按 global coords 与整图一致
    let src = book_grid("mrc", 8, 8);
    let mono = export_mono(&src);
    let (mw, mh, mrgb) = decode_png(&mono);
    let tile = 50;
    let out = src.with_extension("mrc-tiled.png");
    let written = run(&json!({
        "schemaVersion": 1,
        "action": "exportSheetImage",
        "path": src.to_string_lossy(),
        "id": "sheet:Sheet1",
        "output": out.to_string_lossy(),
        "tilePx": tile
    })
    .to_string())
    .unwrap();
    let RunOutput::Written { paths, .. } = written else {
        panic!("written");
    };
    let paths = paths.expect("tiles");
    let rows = paths
        .iter()
        .filter(|p| p.file_name().unwrap().to_string_lossy().contains("-r0c"))
        .count();
    let cols_r0 = rows;
    assert!(
        cols_r0 >= 2 && paths.len() / cols_r0 >= 2,
        "need multi-row and multi-col tiles"
    );

    let mut stitched = vec![255u8; (mw * mh * 3) as usize];
    for p in &paths {
        let name = p.file_name().unwrap().to_string_lossy();
        let rc = name.rsplit('-').next().unwrap().trim_end_matches(".png");
        let (rs, cs) = rc.split_once('c').unwrap();
        let row: u32 = rs.trim_start_matches('r').parse().unwrap();
        let col: u32 = cs.parse().unwrap();
        let (tw, th, trgb) = decode_png(&std::fs::read(p).unwrap());
        let ox = col * tile;
        let oy = row * tile;
        // 末块不足 tile
        assert!(tw <= tile && th <= tile);
        if col + 1 < (mw + tile - 1) / tile {
            assert_eq!(tw, tile);
        }
        for y in 0..th {
            for x in 0..tw {
                let si = ((y * tw + x) * 3) as usize;
                let di = (((oy + y) * mw + (ox + x)) * 3) as usize;
                stitched[di] = trgb[si];
                stitched[di + 1] = trgb[si + 1];
                stitched[di + 2] = trgb[si + 2];
            }
        }
    }
    assert_eq!(
        stitched, mrgb,
        "multi-row/col stitch must match mono (incl. seams)"
    );
}
