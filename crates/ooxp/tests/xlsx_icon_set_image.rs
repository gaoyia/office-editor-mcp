//! 图标集最小子集出图：3Arrows / 3ArrowsGray / 3TrafficLights1，以及 showValue。
//! 字形为程序近似，不与官方像素做 differ=0。未实现的集合仍 NI。

use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::json;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp(tag: &str, ext: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-xlsx-icon-set-img");
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

fn book(tag: &str, cells: Vec<serde_json::Value>) -> PathBuf {
    let path = tmp(tag, "xlsx");
    run_ok(json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "workbook": { "sheets": [{ "name": "Sheet1", "cells": cells }] }
    }));
    path
}

fn insert_icon(path: &Path, sqref: &str, set: &str, show_value: bool, reverse: bool) {
    run_ok(json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": path.to_string_lossy(),
        "ops": [{
            "op": "insertConditionalFormat",
            "id": "sheet:Sheet1",
            "sqref": [sqref],
            "type": "iconSet",
            "iconSet": { "set": set, "showValue": show_value, "reverse": reverse }
        }]
    }));
}

fn export_sheet(path: &Path, dpi: Option<i64>) -> Result<Vec<u8>, ooxp::Error> {
    let png = tmp("export", "png");
    let mut req = json!({
        "schemaVersion": 1,
        "action": "exportSheetImage",
        "path": path.to_string_lossy(),
        "id": "sheet:Sheet1",
        "output": png.to_string_lossy(),
    });
    if let Some(dpi) = dpi {
        req["dpi"] = json!(dpi);
    }
    let written = run(&req.to_string())?;
    let RunOutput::Written {
        path: out,
        content_type,
        ..
    } = written
    else {
        panic!("应写出 PNG");
    };
    assert_eq!(content_type.as_deref(), Some("image/png"));
    assert_eq!(out, png);
    Ok(std::fs::read(png).unwrap())
}

fn rgb_at(rgb: &[u8], w: u32, x: u32, y: u32) -> [u8; 3] {
    let i = ((y * w + x) * 3) as usize;
    [rgb[i], rgb[i + 1], rgb[i + 2]]
}

fn is_grid_or_white(p: [u8; 3]) -> bool {
    p == [255, 255, 255] || p == [224, 224, 224]
}

fn is_near_black(p: [u8; 3]) -> bool {
    p[0] < 80 && p[1] < 80 && p[2] < 80
}

fn count_ink(rgb: &[u8], w: u32, h: u32, x0: u32, y0: u32, x1: u32, y1: u32) -> usize {
    let mut n = 0;
    for y in y0..y1.min(h) {
        for x in x0..x1.min(w) {
            if !is_grid_or_white(rgb_at(rgb, w, x, y)) {
                n += 1;
            }
        }
    }
    n
}

fn count_black(rgb: &[u8], w: u32, h: u32, x0: u32, y0: u32, x1: u32, y1: u32) -> usize {
    let mut n = 0;
    for y in y0..y1.min(h) {
        for x in x0..x1.min(w) {
            if is_near_black(rgb_at(rgb, w, x, y)) {
                n += 1;
            }
        }
    }
    n
}

#[test]
fn icon_set_3arrows_paints_and_hides_or_shows_value() {
    let path = book(
        "3arrows",
        vec![
            json!({ "ref": "A1", "value": 10 }),
            json!({ "ref": "A2", "value": 50 }),
            json!({ "ref": "A3", "value": 90 }),
        ],
    );
    insert_icon(&path, "A1:A3", "3Arrows", true, false);
    let png = export_sheet(&path, Some(106)).unwrap();
    let (w, h, rgb) = decode_png(&png);
    #[cfg(windows)]
    {
        // 缺省列 72 + 1 边框；自动行因图标抬到 18 磅 → 27 + 1。三行 27*3+1=82。
        assert_eq!((w, h), (73, 82), "106 DPI 官方对照尺寸");
    }
    let left = count_ink(&rgb, w, h, 1, 1, 22, h.saturating_sub(1));
    let right_black = count_black(
        &rgb,
        w,
        h,
        w.saturating_sub(24),
        1,
        w.saturating_sub(2),
        h.saturating_sub(1),
    );
    assert!(left > 40, "左侧应画出图标 ink={left}");
    assert!(
        right_black > 8,
        "showValue 打开时应在右侧画出数字 black={right_black}"
    );

    let hidden = book(
        "hide",
        vec![
            json!({ "ref": "A1", "value": 10 }),
            json!({ "ref": "A2", "value": 50 }),
            json!({ "ref": "A3", "value": 90 }),
        ],
    );
    insert_icon(&hidden, "A1:A3", "3Arrows", false, false);
    let png = export_sheet(&hidden, Some(106)).unwrap();
    let (w, h, rgb) = decode_png(&png);
    let left = count_ink(&rgb, w, h, 1, 1, 22, h.saturating_sub(1));
    let right_black = count_black(&rgb, w, h, 28, 1, w.saturating_sub(1), h.saturating_sub(1));
    assert!(left > 40, "showValue=0 仍应画出左侧图标 ink={left}");
    assert_eq!(right_black, 0, "showValue=0 右侧不得画出数字");
}

#[test]
fn icon_set_reverse_and_other_minimal_sets_export() {
    for (tag, set) in [
        ("gray", "3ArrowsGray"),
        ("lights", "3TrafficLights1"),
        ("rev", "3Arrows"),
    ] {
        let path = book(
            tag,
            vec![
                json!({ "ref": "A1", "value": 10 }),
                json!({ "ref": "A2", "value": 50 }),
                json!({ "ref": "A3", "value": 90 }),
            ],
        );
        insert_icon(&path, "A1:A3", set, true, tag == "rev");
        let png = export_sheet(&path, Some(106)).unwrap();
        let (w, h, rgb) = decode_png(&png);
        let ink = count_ink(&rgb, w, h, 1, 1, 22, h.saturating_sub(1));
        assert!(ink > 40, "{set} 应画出图标 ink={ink}");
    }
}

#[test]
fn unsupported_icon_set_still_not_implemented() {
    let path = book(
        "flags",
        vec![
            json!({ "ref": "A1", "value": 10 }),
            json!({ "ref": "A2", "value": 50 }),
            json!({ "ref": "A3", "value": 90 }),
        ],
    );
    insert_icon(&path, "A1:A3", "3Flags", true, false);
    let err = export_sheet(&path, Some(106)).unwrap_err();
    assert_eq!(err.code(), ErrorCode::NotImplemented);
}

#[test]
fn single_cell_at_106_matches_official_default_canvas() {
    let path = book("one", vec![json!({ "ref": "A1", "value": 10 })]);
    insert_icon(&path, "A1", "3Arrows", true, false);
    let png = export_sheet(&path, Some(106)).unwrap();
    let (w, h, rgb) = decode_png(&png);
    #[cfg(windows)]
    assert_eq!((w, h), (73, 28));
    let left = count_ink(&rgb, w, h, 1, 1, 22, h.saturating_sub(1));
    assert!(left > 20, "单格也应画出图标 ink={left}");
}
