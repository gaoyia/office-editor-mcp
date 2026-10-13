//! docx create 第四轮：复用 apply 写样式、表外观、形状、拼音、题注、书目、嵌入字体与 DrawingML 链。

use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::ZipArchive;

const PNG_1X1: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
    0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08, 0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00,
    0x00, 0x03, 0x01, 0x01, 0x00, 0x18, 0xDD, 0x8D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E,
    0x44, 0xAE, 0x42, 0x60, 0x82,
];

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-docx-create-round4");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn seq_path(tag: &str, ext: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}.{ext}"))
}

fn png_path() -> PathBuf {
    let path = seq_path("pic", "png");
    std::fs::write(&path, PNG_1X1).unwrap();
    path
}

fn calibri() -> &'static str {
    r"C:\Windows\Fonts\calibri.ttf"
}

fn create_docx(tag: &str, document: Value) -> PathBuf {
    let path = seq_path(tag, "docx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "format": "docx",
            "document": document
        })
        .to_string(),
        None,
    )
    .unwrap();
    path
}

fn create_err(document: Value) -> ooxp::Error {
    let path = seq_path("err", "docx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "format": "docx",
            "document": document
        })
        .to_string(),
        None,
    )
    .unwrap_err()
}

fn inspect(path: &Path, scope: Value) -> Value {
    let RunOutput::Inspect(tree) = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": path.to_string_lossy(),
            "scope": scope
        })
        .to_string(),
        None,
    )
    .unwrap() else {
        panic!("inspect 应返回树");
    };
    tree
}

fn zip_has(path: &Path, name: &str) -> bool {
    let bytes = std::fs::read(path).unwrap();
    let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    for i in 0..zip.len() {
        if zip.by_index(i).unwrap().name().replace('\\', "/") == name {
            return true;
        }
    }
    false
}

fn part_text(path: &Path, name: &str) -> String {
    let bytes = std::fs::read(path).unwrap();
    let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut file = zip.by_name(name).unwrap();
    let mut s = String::new();
    std::io::Read::read_to_string(&mut file, &mut s).unwrap();
    s
}

fn full_scope() -> Value {
    json!({
        "from": 1,
        "maxParas": 30,
        "pictures": true,
        "shapes": true,
        "connectors": true,
        "tables": true,
        "styles": true,
        "rubies": true,
        "captions": true,
        "indexes": true,
        "indexEntries": true,
        "sources": true,
        "citations": true,
        "bibliographies": true,
        "crossRefs": true,
        "editRanges": true,
        "embeddedFonts": true,
        "docParts": true
    })
}

fn round4_document(pic: &Path, with_font: bool) -> Value {
    let mut document = json!({
        "styles": [{ "styleId": "ooxpTbl", "type": "table", "name": "ooxpTbl" }],
        "sources": [{ "tag": "Book1", "title": "Sample Book", "sourceType": "Book" }],
        "bibliographyStyle": "MLA",
        "fontEmbed": { "trueType": true },
        "eastAsianSettings": { "characterSpacingControl": "compressPunctuation" },
        "docParts": [{ "name": "qp1", "text": "基块正文" }],
        "paragraphs": [
            {
                "style": "Heading1",
                "text": "标题",
                "bookmarks": [{ "name": "ch1" }]
            },
            {
                "text": "hello",
                "pictures": [{
                    "file": pic.to_string_lossy(),
                    "hyperlink": { "url": "https://example.com" }
                }],
                "shapes": [{
                    "prst": "ellipse",
                    "hyperlink": { "url": "https://example.com/shape" }
                }],
                "connectors": [{ "prst": "straightConnector1" }],
                "rubies": [{ "base": "汉字", "rt": "hànzì" }],
                "citations": [{ "tag": "Book1" }],
                "indexEntries": [{ "entry": "hello" }],
                "crossRefs": [{ "bookmark": "ch1" }],
                "editRange": true
            }
        ],
        "tables": [{
            "cells": [["A", "B"]],
            "style": "style:ooxpTbl",
            "look": { "firstRow": true },
            "cellFormats": [{ "r": 1, "c": 1, "fill": "FF0000" }]
        }],
        "captions": [{ "label": "Figure", "title": "图一" }],
        "indexes": [{}],
        "bibliographies": [{}]
    });
    if with_font {
        document["embeddedFonts"] = json!([{ "name": "Calibri", "file": calibri() }]);
    }
    document
}

#[test]
fn create_round4_writes_styles_table_shapes_ruby_biblio_and_drawing_link() {
    let pic = png_path();
    let with_font = Path::new(calibri()).is_file();
    let path = create_docx("round4", round4_document(&pic, with_font));
    let tree = inspect(&path, full_scope());

    assert!(tree["styleCount"].as_u64().unwrap() >= 5, "{tree}");
    let custom = tree["styles"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["styleId"] == "ooxpTbl")
        .unwrap();
    assert_eq!(custom["type"], "table");

    assert_eq!(tree["tableCount"], 1);
    assert_eq!(tree["tables"][0]["style"], "ooxpTbl");
    assert_eq!(tree["tables"][0]["look"]["firstRow"], true);
    assert_eq!(tree["tables"][0]["rows"][0]["cells"][0]["fill"], "FF0000");

    assert_eq!(tree["shapeCount"], 1);
    assert_eq!(tree["paragraphs"][1]["shapes"][0]["prst"], "ellipse");
    assert_eq!(
        tree["paragraphs"][1]["shapes"][0]["hyperlink"]["url"],
        "https://example.com/shape"
    );
    assert_eq!(tree["connectorCount"], 1);
    assert_eq!(
        tree["paragraphs"][1]["connectors"][0]["prst"],
        "straightConnector1"
    );
    assert_eq!(tree["rubyCount"], 1);
    assert_eq!(tree["rubies"][0]["base"], "汉字");
    assert_eq!(tree["rubies"][0]["rt"], "hànzì");

    assert_eq!(tree["sourceCount"], 1);
    assert_eq!(tree["sources"][0]["tag"], "Book1");
    assert_eq!(tree["bibliographyStyle"]["styleName"], "MLA");
    assert_eq!(tree["citationCount"], 1);
    assert_eq!(tree["captionCount"], 1);
    assert_eq!(tree["indexCount"], 1);
    assert_eq!(tree["indexEntryCount"], 1);
    assert_eq!(tree["bibliographyCount"], 1);
    assert_eq!(tree["crossRefCount"], 1);
    assert_eq!(tree["editRangeCount"], 1);
    assert_eq!(tree["editRanges"][0]["edGrp"], "everyone");
    assert_eq!(tree["docPartCount"], 1);

    assert_eq!(tree["pictureCount"], 1);
    let pic_node = &tree["paragraphs"][1]["pictures"][0];
    assert_eq!(pic_node["hyperlink"]["url"], "https://example.com");
    assert_eq!(tree["hyperlinkCount"], 0);

    assert_eq!(
        tree["eastAsianSettings"]["characterSpacingControl"],
        "compressPunctuation"
    );
    assert_eq!(tree["fontEmbed"]["trueType"], true);
    if with_font {
        assert_eq!(tree["embeddedFontCount"], 1);
        assert_eq!(tree["embeddedFonts"][0]["name"], "Calibri");
        assert!(zip_has(&path, "word/fontTable.xml"));
    }

    let settings = part_text(&path, "word/settings.xml");
    assert!(settings.contains("compressPunctuation"), "{settings}");
    let styles = part_text(&path, "word/styles.xml");
    assert!(styles.contains("styleId=\"ooxpTbl\""), "{styles}");
}

#[test]
fn create_round4_rejects_password_edit_range_false_and_foreign_fields() {
    let err = create_err(json!({
        "paragraphs": [{ "text": "a" }],
        "protection": { "password": "secret" }
    }));
    assert_eq!(err.code(), ErrorCode::NotImplemented);

    let err = create_err(json!({
        "paragraphs": [{ "text": "a", "editRange": false }]
    }));
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let xlsx = seq_path("xlsx", "xlsx");
    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": xlsx.to_string_lossy(),
            "format": "xlsx",
            "workbook": {
                "sheets": [{ "name": "Sheet1" }],
                "styles": [{ "styleId": "x" }]
            }
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let pptx = seq_path("pptx", "pptx");
    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": pptx.to_string_lossy(),
            "format": "pptx",
            "presentation": {
                "slides": [{ "shapes": [{ "text": "a" }] }],
                "styles": [{ "styleId": "x" }]
            }
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}
