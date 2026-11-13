//! 旧式页眉图：只有 legacyDrawingHF 时查看、换字节、删除。有 drawingHF 时不进这份名单。

use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

const PNG_A: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
    0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08, 0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00,
    0x00, 0x03, 0x01, 0x01, 0x00, 0x18, 0xDD, 0x8D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E,
    0x44, 0xAE, 0x42, 0x60, 0x82,
];

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-xlsx-legacy-hf");
    std::fs::create_dir_all(&dir).unwrap();
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    dir.join(format!("{tag}-{n}"))
}

fn create_book(tag: &str) -> PathBuf {
    let path = tmp(tag).with_extension("xlsx");
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

fn inspect(path: &Path, scope: Value) -> Value {
    let RunOutput::Inspect(tree) = run(&json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": path.to_string_lossy(),
        "scope": scope
    })
    .to_string())
    .unwrap() else {
        panic!("inspect 应返回树");
    };
    tree
}

fn apply(src: &Path, out: &Path, ops: Value) -> Result<(), ooxp::Error> {
    run(&json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": out.to_string_lossy(),
        "ops": ops
    })
    .to_string())
    .map(|_| ())
}

fn part_text(path: &Path, name: &str) -> String {
    let bytes = std::fs::read(path).unwrap();
    let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut file = zip.by_name(name).unwrap();
    let mut s = String::new();
    file.read_to_string(&mut s).unwrap();
    s
}

fn part_exists(path: &Path, name: &str) -> bool {
    let bytes = std::fs::read(path).unwrap();
    let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let found = zip.by_name(name).is_ok();
    found
}

fn vml_xml() -> String {
    r##"<?xml version="1.0" encoding="UTF-8"?>
<xml xmlns:v="urn:schemas-microsoft-com:vml" xmlns:o="urn:schemas-microsoft-com:office:office">
<v:shape id="CH" o:spid="_x0000_s1025" type="#_x0000_t75"><v:imagedata o:relid="rId1" o:title="a"/></v:shape>
<v:shape id="LF" o:spid="_x0000_s1026" type="#_x0000_t75"><v:imagedata o:relid="rId2" o:title="b"/></v:shape>
</xml>"##
        .to_string()
}

fn vml_rels() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/image9.png"/>
<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/image10.png"/>
</Relationships>"#
        .to_string()
}

fn inject(src: &Path, dest: &Path) {
    let bytes = std::fs::read(src).unwrap();
    let mut zip_in = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut entries = Vec::new();
    for i in 0..zip_in.len() {
        let mut file = zip_in.by_index(i).unwrap();
        let name = file.name().to_string();
        let mut data = Vec::new();
        file.read_to_end(&mut data).unwrap();
        entries.push((name, data));
    }
    let mut has_rels = false;
    for (name, data) in entries.iter_mut() {
        if name == "xl/worksheets/sheet1.xml" {
            let mut xml = String::from_utf8(std::mem::take(data)).unwrap();
            assert!(
                xml.contains("</worksheet>"),
                "工作表根不是 worksheet: {xml}"
            );
            if !xml.contains("xmlns:r=") {
                xml = xml.replacen(
                    "<worksheet",
                    r#"<worksheet xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships""#,
                    1,
                );
            }
            if !xml.contains("<headerFooter") {
                xml = xml.replacen(
                    "</worksheet>",
                    r#"<headerFooter><oddHeader>&amp;C&amp;G</oddHeader><oddFooter>&amp;L&amp;G</oddFooter></headerFooter></worksheet>"#,
                    1,
                );
            }
            if !xml.contains("legacyDrawingHF") {
                xml = xml.replacen(
                    "</worksheet>",
                    r#"<legacyDrawingHF r:id="rId9"/></worksheet>"#,
                    1,
                );
            }
            *data = xml.into_bytes();
        }
        if name == "[Content_Types].xml" {
            let mut xml = String::from_utf8(std::mem::take(data)).unwrap();
            if !xml.contains("Extension=\"vml\"") {
                xml = xml.replacen(
                    "</Types>",
                    r#"<Default Extension="vml" ContentType="application/vnd.openxmlformats-officedocument.vmlDrawing"/></Types>"#,
                    1,
                );
            }
            if !xml.contains("Extension=\"png\"") {
                xml = xml.replacen(
                    "</Types>",
                    r#"<Default Extension="png" ContentType="image/png"/></Types>"#,
                    1,
                );
            }
            *data = xml.into_bytes();
        }
        if name == "xl/worksheets/_rels/sheet1.xml.rels" {
            has_rels = true;
            let mut xml = String::from_utf8(std::mem::take(data)).unwrap();
            if !xml.contains("vmlDrawing") {
                xml = xml.replacen(
                    "</Relationships>",
                    r#"<Relationship Id="rId9" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/vmlDrawing" Target="../drawings/vmlDrawing1.vml"/></Relationships>"#,
                    1,
                );
            }
            *data = xml.into_bytes();
        }
    }
    if !has_rels {
        entries.push((
            "xl/worksheets/_rels/sheet1.xml.rels".into(),
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId9" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/vmlDrawing" Target="../drawings/vmlDrawing1.vml"/>
</Relationships>"#
                .to_string()
                .into_bytes(),
        ));
    }
    let mut png_b = PNG_A.to_vec();
    png_b.push(0x01);
    entries.push(("xl/drawings/vmlDrawing1.vml".into(), vml_xml().into_bytes()));
    entries.push((
        "xl/drawings/_rels/vmlDrawing1.vml.rels".into(),
        vml_rels().into_bytes(),
    ));
    entries.push(("xl/media/image9.png".into(), PNG_A.to_vec()));
    entries.push(("xl/media/image10.png".into(), png_b));

    let mut out = Cursor::new(Vec::new());
    {
        let mut zip_out = ZipWriter::new(&mut out);
        let opt = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        for (name, data) in entries {
            zip_out.start_file(name, opt).unwrap();
            zip_out.write_all(&data).unwrap();
        }
        zip_out.finish().unwrap();
    }
    std::fs::write(dest, out.into_inner()).unwrap();
}

fn legacy_book(tag: &str) -> PathBuf {
    let src = create_book(tag);
    let path = tmp(&format!("{tag}-vml")).with_extension("xlsx");
    inject(&src, &path);
    path
}

#[test]
fn lists_odd_header_and_footer_only() {
    let path = legacy_book("list");
    let plain = inspect(&path, json!({}));
    assert_eq!(plain["legacyPictureCount"], 2);
    assert_eq!(plain["sheets"][0]["legacyPictureCount"], 2);
    assert_eq!(plain["pictureCount"], 0);
    assert!(plain["sheets"][0].get("headerFooters").is_none());

    let tree = inspect(
        &path,
        json!({ "headerFooter": true, "legacyPictures": true, "pictures": true }),
    );
    let slots = tree["sheets"][0]["headerFooters"].as_array().unwrap();
    let header = &slots[0];
    let footer = &slots[1];
    assert_eq!(header["legacyPictureCount"], 1);
    assert_eq!(footer["legacyPictureCount"], 1);
    assert_eq!(header["pictureCount"], 0);
    assert_eq!(header["pictures"], json!([]));
    let pics = header["legacyPictures"].as_array().unwrap();
    assert_eq!(pics.len(), 1);
    assert_eq!(pics[0]["id"], "sheet:Sheet1/hf:odd:header/legacyPic:CH");
    assert_eq!(pics[0]["shapeId"], "CH");
    assert_eq!(pics[0]["contentType"], "image/png");
    assert_eq!(
        footer["legacyPictures"][0]["id"],
        "sheet:Sheet1/hf:odd:footer/legacyPic:LF"
    );
    assert!(slots[2]
        .get("legacyPictures")
        .unwrap()
        .as_array()
        .unwrap()
        .is_empty());
}

#[test]
fn replace_keeps_vml_and_header_text() {
    let src = legacy_book("repl");
    let file = tmp("new").with_extension("png");
    let mut bytes = PNG_A.to_vec();
    bytes.push(0x02);
    std::fs::write(&file, &bytes).unwrap();
    let out = tmp("repl-out").with_extension("xlsx");
    apply(
        &src,
        &out,
        json!([{
            "op": "replaceLegacyPicture",
            "id": "sheet:Sheet1/hf:odd:header/legacyPic:CH",
            "file": file.to_string_lossy()
        }]),
    )
    .unwrap();
    let xml = part_text(&out, "xl/worksheets/sheet1.xml");
    assert!(xml.contains("&amp;C&amp;G"), "{xml}");
    assert!(xml.contains("legacyDrawingHF"), "{xml}");
    assert!(!xml.contains("drawingHF"), "{xml}");
    let exported = tmp("exp").with_extension("png");
    run(&json!({
        "schemaVersion": 1,
        "action": "exportMedia",
        "path": out.to_string_lossy(),
        "id": "sheet:Sheet1/hf:odd:header/legacyPic:CH",
        "output": exported.to_string_lossy()
    })
    .to_string())
    .unwrap();
    assert_eq!(std::fs::read(&exported).unwrap(), bytes);
    let tree = inspect(
        &out,
        json!({ "headerFooter": true, "legacyPictures": true }),
    );
    assert_eq!(tree["sheets"][0]["legacyPictureCount"], 2);
    assert_eq!(
        tree["sheets"][0]["headerFooters"][1]["legacyPictures"][0]["shapeId"],
        "LF"
    );
}

#[test]
fn delete_one_then_last_keeps_header_text() {
    let src = legacy_book("del");
    let mid = tmp("del-mid").with_extension("xlsx");
    apply(
        &src,
        &mid,
        json!([{
            "op": "deleteLegacyPicture",
            "id": "sheet:Sheet1/hf:odd:header/legacyPic:CH"
        }]),
    )
    .unwrap();
    let tree = inspect(
        &mid,
        json!({ "headerFooter": true, "legacyPictures": true }),
    );
    assert_eq!(tree["legacyPictureCount"], 1);
    assert!(tree["sheets"][0]["headerFooters"][0]["legacyPictures"]
        .as_array()
        .unwrap()
        .is_empty());
    assert_eq!(
        tree["sheets"][0]["headerFooters"][1]["legacyPictures"][0]["shapeId"],
        "LF"
    );
    let xml = part_text(&mid, "xl/worksheets/sheet1.xml");
    assert!(xml.contains("legacyDrawingHF"), "{xml}");
    assert!(xml.contains("&amp;C&amp;G"), "{xml}");

    let gone = tmp("del-gone").with_extension("xlsx");
    apply(
        &mid,
        &gone,
        json!([{
            "op": "deleteLegacyPicture",
            "id": "sheet:Sheet1/hf:odd:footer/legacyPic:LF"
        }]),
    )
    .unwrap();
    let xml = part_text(&gone, "xl/worksheets/sheet1.xml");
    assert!(!xml.contains("legacyDrawingHF"), "{xml}");
    assert!(xml.contains("&amp;C&amp;G"), "{xml}");
    assert!(xml.contains("&amp;L&amp;G"), "{xml}");
    assert!(!part_exists(&gone, "xl/drawings/vmlDrawing1.vml"));
    let tree = inspect(&gone, json!({}));
    assert_eq!(tree["legacyPictureCount"], 0);
}

#[test]
fn drawing_hf_hides_legacy_list() {
    let src = create_book("both");
    let png = tmp("pic").with_extension("png");
    std::fs::write(&png, PNG_A).unwrap();
    let drawn = tmp("drawn").with_extension("xlsx");
    apply(
        &src,
        &drawn,
        json!([{
            "op": "insertPicture",
            "id": "sheet:Sheet1/hf:odd:header",
            "file": png.to_string_lossy()
        }]),
    )
    .unwrap();
    let both = tmp("both-vml").with_extension("xlsx");
    inject(&drawn, &both);
    let tree = inspect(
        &both,
        json!({ "headerFooter": true, "legacyPictures": true, "pictures": true }),
    );
    assert_eq!(tree["sheets"][0]["legacyPictureCount"], 0);
    let header = &tree["sheets"][0]["headerFooters"][0];
    assert_eq!(header["legacyPictureCount"], 0);
    assert!(header["legacyPictures"].as_array().unwrap().is_empty());
    assert_eq!(header["pictureCount"], 1);
    let err = apply(
        &both,
        &tmp("nope").with_extension("xlsx"),
        json!([{
            "op": "replaceLegacyPicture",
            "id": "sheet:Sheet1/hf:odd:header/legacyPic:CH",
            "file": png.to_string_lossy()
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::IdNotFound);
}

#[test]
fn bad_id_and_svg_are_rejected() {
    let src = legacy_book("bad");
    let png = tmp("real").with_extension("png");
    std::fs::write(&png, PNG_A).unwrap();
    let err = apply(
        &src,
        &tmp("bad-out").with_extension("xlsx"),
        json!([{
            "op": "replaceLegacyPicture",
            "id": "sheet:Sheet1/hf:odd:header/legacyPic:LH",
            "file": png.to_string_lossy()
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::IdNotFound);

    let err = apply(
        &src,
        &tmp("id-out").with_extension("xlsx"),
        json!([{
            "op": "deleteLegacyPicture",
            "id": "sheet:Sheet1/pic:1"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);

    let svg = tmp("pic").with_extension("svg");
    std::fs::write(&svg, b"<svg xmlns=\"http://www.w3.org/2000/svg\"></svg>").unwrap();
    let err = apply(
        &src,
        &tmp("svg-out").with_extension("xlsx"),
        json!([{
            "op": "replaceLegacyPicture",
            "id": "sheet:Sheet1/hf:odd:header/legacyPic:CH",
            "file": svg.to_string_lossy()
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}
