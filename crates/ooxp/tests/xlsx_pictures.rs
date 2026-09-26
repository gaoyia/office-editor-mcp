//! 工作表图：名单、换/插/删、尺寸、锚点、导出。

use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::json;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

const PNG_1X1: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
    0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08, 0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00,
    0x00, 0x03, 0x01, 0x01, 0x00, 0x18, 0xDD, 0x8D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E,
    0x44, 0xAE, 0x42, 0x60, 0x82,
];

const PNG_1X1_B: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
    0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08, 0xD7, 0x63, 0x60, 0x60, 0x60, 0x00,
    0x00, 0x00, 0x04, 0x00, 0x01, 0xA3, 0x0A, 0x15, 0xE3, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E,
    0x44, 0xAE, 0x42, 0x60, 0x82,
];

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-xlsx-pictures");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn tmp(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}"))
}

fn write_png(tag: &str, bytes: &[u8]) -> PathBuf {
    let path = tmp(tag).with_extension("png");
    std::fs::write(&path, bytes).unwrap();
    path
}

fn create_book(tag: &str) -> PathBuf {
    let path = tmp(tag).with_extension("xlsx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "workbook": { "sheets": [{ "name": "Sheet1", "cells": [{ "ref": "A1", "value": 1 }] }] }
        })
        .to_string(),
        None,
    )
    .unwrap();
    path
}

fn apply(src: &Path, out: &Path, ops: serde_json::Value) -> Result<RunOutput, ooxp::Error> {
    run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": src.to_string_lossy(),
            "output": out.to_string_lossy(),
            "ops": ops,
        })
        .to_string(),
        None,
    )
}

fn inspect_pics(path: &Path) -> serde_json::Value {
    let RunOutput::Inspect(tree) = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": path.to_string_lossy(),
            "scope": { "pictures": true }
        })
        .to_string(),
        None,
    )
    .unwrap() else {
        panic!("inspect 应返回树");
    };
    tree
}

fn object_keys(v: &serde_json::Value) -> Vec<&str> {
    v.as_object().unwrap().keys().map(String::as_str).collect()
}

fn part_bytes(path: &Path, name: &str) -> Vec<u8> {
    let bytes = std::fs::read(path).unwrap();
    let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut file = zip.by_name(name).unwrap();
    let mut out = Vec::new();
    file.read_to_end(&mut out).unwrap();
    out
}

fn part_names(path: &Path) -> Vec<String> {
    let bytes = std::fs::read(path).unwrap();
    let zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    zip.file_names().map(str::to_string).collect()
}

fn rewrite_part(path: &Path, name: &str, f: impl FnOnce(Vec<u8>) -> Vec<u8>) {
    let bytes = std::fs::read(path).unwrap();
    let mut zip_in = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut out = Cursor::new(Vec::new());
    let mut transform = Some(f);
    {
        let mut zip_out = ZipWriter::new(&mut out);
        let opt = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        for i in 0..zip_in.len() {
            let mut file = zip_in.by_index(i).unwrap();
            let n = file.name().to_string();
            let mut data = Vec::new();
            file.read_to_end(&mut data).unwrap();
            if n == name {
                data = transform.take().expect("部件只应出现一次")(data);
            }
            zip_out.start_file(&n, opt).unwrap();
            zip_out.write_all(&data).unwrap();
        }
        zip_out.finish().unwrap();
    }
    std::fs::write(path, out.into_inner()).unwrap();
}

fn insert_one(src: &Path, out: &Path, png: &Path) {
    apply(
        src,
        out,
        json!([{
            "op": "insertPicture",
            "id": "sheet:Sheet1",
            "file": png.to_string_lossy(),
            "from": "B2",
            "cx": 914400,
            "cy": 914400
        }]),
    )
    .unwrap();
}

#[test]
fn inspect_picture_count_without_list() {
    let path = create_book("count");
    let RunOutput::Inspect(tree) = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": path.to_string_lossy()
        })
        .to_string(),
        None,
    )
    .unwrap() else {
        panic!();
    };
    assert_eq!(tree["pictureCount"], 0);
    assert_eq!(tree["sheets"][0]["pictureCount"], 0);
    assert!(tree["sheets"][0].get("pictures").is_none());
}

#[test]
fn insert_inspect_replace_size_anchor_delete_export() {
    let src = create_book("round");
    let png_a = write_png("a", PNG_1X1);
    let png_b = write_png("b", PNG_1X1_B);
    let inserted = tmp("ins").with_extension("xlsx");
    insert_one(&src, &inserted, &png_a);

    let tree = inspect_pics(&inserted);
    assert_eq!(tree["pictureCount"], 1);
    assert_eq!(tree["sheets"][0]["pictureCount"], 1);
    let pics = tree["sheets"][0]["pictures"].as_array().unwrap();
    assert_eq!(pics.len(), 1);
    assert_eq!(pics[0]["id"], "sheet:Sheet1/pic:1");
    assert_eq!(pics[0]["cNvPrId"], "1");
    assert_eq!(pics[0]["anchor"], "oneCell");
    assert_eq!(pics[0]["from"], "B2");
    assert_eq!(pics[0]["cx"], 914400);
    assert_eq!(pics[0]["cy"], 914400);
    assert_eq!(pics[0]["stable"], true);
    assert_eq!(
        object_keys(&pics[0]),
        [
            "id",
            "cNvPrId",
            "name",
            "anchor",
            "from",
            "fromOffX",
            "fromOffY",
            "cx",
            "cy",
            "stable"
        ]
    );
    assert_eq!(part_bytes(&inserted, "xl/media/image1.png"), PNG_1X1);

    let replaced = tmp("rep").with_extension("xlsx");
    apply(
        &inserted,
        &replaced,
        json!([{
            "op": "replacePicture",
            "id": "sheet:Sheet1/pic:1",
            "file": png_b.to_string_lossy()
        }]),
    )
    .unwrap();
    assert_eq!(part_bytes(&replaced, "xl/media/image2.png"), PNG_1X1_B);
    let after_rep = inspect_pics(&replaced);
    assert_eq!(after_rep["sheets"][0]["pictures"][0]["anchor"], "oneCell");
    assert_eq!(after_rep["sheets"][0]["pictures"][0]["from"], "B2");

    let sized = tmp("size").with_extension("xlsx");
    apply(
        &replaced,
        &sized,
        json!([{
            "op": "setPictureSize",
            "id": "sheet:Sheet1/pic:1",
            "cx": 1828800,
            "cy": 914400
        }]),
    )
    .unwrap();
    let after_size = inspect_pics(&sized);
    assert_eq!(after_size["sheets"][0]["pictures"][0]["cx"], 1828800);
    assert_eq!(after_size["sheets"][0]["pictures"][0]["cy"], 914400);

    let moved = tmp("move").with_extension("xlsx");
    apply(
        &sized,
        &moved,
        json!([{
            "op": "setPictureAnchor",
            "id": "sheet:Sheet1/pic:1",
            "from": "D5"
        }]),
    )
    .unwrap();
    let after_move = inspect_pics(&moved);
    assert_eq!(after_move["sheets"][0]["pictures"][0]["from"], "D5");
    assert_eq!(after_move["sheets"][0]["pictures"][0]["anchor"], "oneCell");

    let exported = tmp("exp").with_extension("png");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "exportMedia",
            "path": moved.to_string_lossy(),
            "id": "sheet:Sheet1/pic:1",
            "output": exported.to_string_lossy()
        })
        .to_string(),
        None,
    )
    .unwrap();
    assert_eq!(std::fs::read(&exported).unwrap(), PNG_1X1_B);

    let deleted = tmp("del").with_extension("xlsx");
    apply(
        &moved,
        &deleted,
        json!([{ "op": "deletePicture", "id": "sheet:Sheet1/pic:1" }]),
    )
    .unwrap();
    let after_del = inspect_pics(&deleted);
    assert_eq!(after_del["pictureCount"], 0);
    assert_eq!(after_del["sheets"][0]["pictures"], json!([]));
    let names = part_names(&deleted);
    assert!(!names.iter().any(|n| n.starts_with("xl/drawings/")));
    assert!(!names.iter().any(|n| n.starts_with("xl/media/")));
}

#[test]
fn insert_two_cell_and_reject_size() {
    let src = create_book("two");
    let png = write_png("two", PNG_1X1);
    let out = tmp("two-out").with_extension("xlsx");
    apply(
        &src,
        &out,
        json!([{
            "op": "insertPicture",
            "id": "sheet:Sheet1",
            "file": png.to_string_lossy(),
            "anchor": "twoCell",
            "from": "A1",
            "to": "C4",
            "cx": 914400,
            "cy": 914400
        }]),
    )
    .unwrap();
    let tree = inspect_pics(&out);
    let pic = &tree["sheets"][0]["pictures"][0];
    assert_eq!(pic["anchor"], "twoCell");
    assert_eq!(pic["from"], "A1");
    assert_eq!(pic["to"], "C4");
    assert!(pic.get("cx").is_none());
    assert_eq!(
        object_keys(pic),
        [
            "id",
            "cNvPrId",
            "name",
            "anchor",
            "from",
            "fromOffX",
            "fromOffY",
            "to",
            "toOffX",
            "toOffY",
            "editAs",
            "stable"
        ]
    );
    let err = apply(
        &out,
        &tmp("two-size").with_extension("xlsx"),
        json!([{
            "op": "setPictureSize",
            "id": "sheet:Sheet1/pic:1",
            "cx": 1828800
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::NotImplemented);
}

#[test]
fn convert_one_cell_to_two_cell() {
    let src = create_book("conv");
    let png = write_png("conv", PNG_1X1);
    let one = tmp("conv-one").with_extension("xlsx");
    insert_one(&src, &one, &png);
    let two = tmp("conv-two").with_extension("xlsx");
    apply(
        &one,
        &two,
        json!([{
            "op": "setPictureAnchor",
            "id": "sheet:Sheet1/pic:1",
            "anchor": "twoCell",
            "to": "E8"
        }]),
    )
    .unwrap();
    let pic = &inspect_pics(&two)["sheets"][0]["pictures"][0];
    assert_eq!(pic["anchor"], "twoCell");
    assert_eq!(pic["from"], "B2");
    assert_eq!(pic["to"], "E8");
}

#[test]
fn insert_and_set_cell_share_sheet() {
    let src = create_book("mix");
    let png = write_png("mix", PNG_1X1);
    let out = tmp("mix-out").with_extension("xlsx");
    apply(
        &src,
        &out,
        json!([
            {
                "op": "insertPicture",
                "id": "sheet:Sheet1",
                "file": png.to_string_lossy(),
                "from": "C3",
                "cx": 914400,
                "cy": 914400
            },
            { "op": "setCell", "id": "cell:Sheet1!A1", "value": 9 }
        ]),
    )
    .unwrap();
    let RunOutput::Inspect(tree) = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": out.to_string_lossy(),
            "scope": { "sheet": "Sheet1", "range": "A1", "pictures": true }
        })
        .to_string(),
        None,
    )
    .unwrap() else {
        panic!();
    };
    assert_eq!(tree["sheets"][0]["cells"][0]["value"], 9);
    assert_eq!(tree["sheets"][0]["pictures"][0]["from"], "C3");
}

#[test]
fn insert_rejects_cell_id() {
    let src = create_book("bad-id");
    let png = write_png("bad-id", PNG_1X1);
    let err = apply(
        &src,
        &tmp("bad-id-out").with_extension("xlsx"),
        json!([{
            "op": "insertPicture",
            "id": "cell:Sheet1!A1",
            "file": png.to_string_lossy(),
            "from": "A1"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
}

#[test]
fn set_picture_anchor_absolute_is_not_implemented() {
    let src = create_book("abs");
    let png = write_png("abs", PNG_1X1);
    let one = tmp("abs-one").with_extension("xlsx");
    insert_one(&src, &one, &png);
    let err = apply(
        &one,
        &tmp("abs-out").with_extension("xlsx"),
        json!([{
            "op": "setPictureAnchor",
            "id": "sheet:Sheet1/pic:1",
            "anchor": "absolute"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::NotImplemented);
}

#[test]
fn insert_into_excel_like_drawing_adds_r_ns_and_short_rel() {
    let src = create_book("excel-like");
    let png_a = write_png("excel-like-a", PNG_1X1);
    let png_b = write_png("excel-like-b", PNG_1X1_B);
    let first = tmp("excel-like-1").with_extension("xlsx");
    insert_one(&src, &first, &png_a);
    rewrite_part(&first, "xl/drawings/drawing1.xml", |data| {
        let text = String::from_utf8(data).unwrap();
        text.replacen(
            r#" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships""#,
            "",
            1,
        )
        .into_bytes()
    });
    let second = tmp("excel-like-2").with_extension("xlsx");
    apply(
        &first,
        &second,
        json!([{
            "op": "insertPicture",
            "id": "sheet:Sheet1",
            "file": png_b.to_string_lossy(),
            "from": "A10",
            "cx": 914400,
            "cy": 914400
        }]),
    )
    .unwrap();
    let drawing = String::from_utf8(part_bytes(&second, "xl/drawings/drawing1.xml")).unwrap();
    assert!(
        drawing.contains(
            r#"xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships""#
        ),
        "往无 xmlns:r 的 drawing 插图后应补上关系命名空间"
    );
    let rels = String::from_utf8(part_bytes(
        &second,
        "xl/drawings/_rels/drawing1.xml.rels",
    ))
    .unwrap();
    assert!(
        rels.contains(r#"Target="../media/image2.png""#),
        "drawing 到媒体的关系应写成 ../media/，实际：{rels}"
    );
    assert!(
        !rels.contains("../../xl/media/"),
        "不应再写出 ../../xl/media/ 这种绕路 Target"
    );
}
