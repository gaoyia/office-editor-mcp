//! 同一批里增删段内对象之后，原来的位置 id 仍指向开始时的对象。

use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join("ooxp-docx-inline-pin");
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(format!("{tag}-{n}.docx"))
}

fn create(tag: &str, document: Value) -> PathBuf {
    let path = tmp(tag);
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "format": "docx",
        "document": document
    })
    .to_string())
    .unwrap();
    path
}

fn apply(src: &Path, ops: Value) -> PathBuf {
    let out = tmp("out");
    run(&json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": out.to_string_lossy(),
        "ops": ops
    })
    .to_string())
    .unwrap();
    out
}

fn apply_err(src: &Path, ops: Value) -> ooxp::Error {
    let out = tmp("err");
    run(&json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": out.to_string_lossy(),
        "ops": ops
    })
    .to_string())
    .unwrap_err()
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
        panic!("inspect")
    };
    tree
}

fn part(path: &Path, name: &str) -> String {
    let file = std::fs::File::open(path).unwrap();
    let mut zip = ZipArchive::new(file).unwrap();
    let mut entry = zip.by_name(name).unwrap();
    let mut s = String::new();
    entry.read_to_string(&mut s).unwrap();
    s
}

fn column(tree: &Value, list: &str, field: &str) -> Vec<String> {
    tree[list]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item[field].as_str().unwrap().to_string())
        .collect()
}

fn host_para(tag: &str) -> (PathBuf, String) {
    let src = create(tag, json!({ "paragraphs": [{ "text": "宿主" }] }));
    let tree = inspect(&src, json!({ "from": 1, "maxParas": 5 }));
    let id = tree["paragraphs"][0]["id"].as_str().unwrap().to_string();
    (src, id)
}

const STYLES: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/></w:style></w:styles>"#;

fn write_parts(tag: &str, parts: &[(&str, &str)]) -> PathBuf {
    let path = tmp(tag);
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut zip = ZipWriter::new(&mut cursor);
        let opt = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        for (name, body) in parts {
            zip.start_file(name, opt).unwrap();
            zip.write_all(body.as_bytes()).unwrap();
        }
        zip.finish().unwrap();
    }
    std::fs::write(&path, cursor.into_inner()).unwrap();
    path
}

fn document_xml(inner: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture" xmlns:wps="http://schemas.microsoft.com/office/word/2010/wordprocessingShape" xmlns:wpc="http://schemas.microsoft.com/office/word/2010/wordprocessingCanvas" xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" xmlns:dgm="http://schemas.openxmlformats.org/drawingml/2006/diagram" xmlns:o="urn:schemas-microsoft-com:office:office" xmlns:am3d="http://schemas.microsoft.com/office/drawing/2017/model3d">
<w:body><w:p w14:paraId="AAAA0001">{inner}</w:p></w:body></w:document>"#
    )
}

fn bare_package(tag: &str, document: &str, rels: &str, extra: &[(&str, String)]) -> PathBuf {
    let ct = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Default Extension="bin" ContentType="application/octet-stream"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/><Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/><Override PartName="/word/charts/chart1.xml" ContentType="application/vnd.openxmlformats-officedocument.drawingml.chart+xml"/><Override PartName="/word/charts/chart2.xml" ContentType="application/vnd.openxmlformats-officedocument.drawingml.chart+xml"/><Override PartName="/word/charts/chart3.xml" ContentType="application/vnd.openxmlformats-officedocument.drawingml.chart+xml"/><Override PartName="/word/ink/ink1.xml" ContentType="application/inkml+xml"/><Override PartName="/word/ink/ink2.xml" ContentType="application/inkml+xml"/><Override PartName="/word/ink/ink3.xml" ContentType="application/inkml+xml"/></Types>"#;
    let root = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#;
    let mut parts = vec![
        ("[Content_Types].xml".to_string(), ct.to_string()),
        ("_rels/.rels".to_string(), root.to_string()),
        ("word/document.xml".to_string(), document.to_string()),
        ("word/_rels/document.xml.rels".to_string(), rels.to_string()),
        ("word/styles.xml".to_string(), STYLES.to_string()),
    ];
    for (name, body) in extra {
        parts.push(((*name).to_string(), body.clone()));
    }
    let refs: Vec<(&str, &str)> = parts
        .iter()
        .map(|(n, b)| (n.as_str(), b.as_str()))
        .collect();
    write_parts(tag, &refs)
}

fn style_rels() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rIdStyle" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/></Relationships>"#.to_string()
}

fn assert_kept(xml: &str, gone: &[&str], kept: &str) {
    for marker in gone {
        assert!(!xml.contains(marker), "不该留下 {marker}\n{xml}");
    }
    assert!(xml.contains(kept), "该留下 {kept}\n{xml}");
    assert!(!xml.contains("ooxp:"), "{xml}");
}

fn delete_two(
    tag: &str,
    inner: &str,
    rels: &str,
    extra: &[(&str, String)],
    op: &str,
    prefix: &str,
    markers: &[&str],
) {
    let (gone, kept) = markers.split_at(markers.len() - 1);
    let src = bare_package(tag, &document_xml(inner), rels, extra);
    let out = apply(
        &src,
        json!([
            { "op": op, "id": format!("{prefix}:1") },
            { "op": op, "id": format!("{prefix}:2") }
        ]),
    );
    assert_kept(&part(&out, "word/document.xml"), gone, kept[0]);
}

#[test]
fn delete_then_rename_original_bookmark() {
    let (src, para) = host_para("bm");
    let made = apply(
        &src,
        json!([
            { "op": "insertBookmark", "id": para, "name": "Aa" },
            { "op": "insertBookmark", "id": para, "name": "Bb" },
            { "op": "insertBookmark", "id": para, "name": "Cc" }
        ]),
    );
    let tree = inspect(
        &made,
        json!({ "bookmarks": true, "from": 1, "maxParas": 5 }),
    );
    let names = column(&tree, "bookmarks", "name");
    let out = apply(
        &made,
        json!([
            { "op": "deleteBookmark", "id": "bookmark:1" },
            { "op": "setBookmark", "id": "bookmark:2", "name": "Renamed" }
        ]),
    );
    let after = column(
        &inspect(&out, json!({ "bookmarks": true, "from": 1, "maxParas": 5 })),
        "bookmarks",
        "name",
    );
    assert!(!after.contains(&names[0]));
    assert!(!after.contains(&names[1]));
    assert!(after.contains(&names[2]));
    assert!(after.contains(&"Renamed".to_string()));
    assert!(!part(&out, "word/document.xml").contains("ooxp:"));
}

#[test]
fn insert_then_original_bookmark() {
    let (src, para) = host_para("bm-new");
    let made = apply(
        &src,
        json!([
            { "op": "insertBookmark", "id": para, "name": "Aa" },
            { "op": "insertBookmark", "id": para, "name": "Bb" }
        ]),
    );
    let names = column(
        &inspect(
            &made,
            json!({ "bookmarks": true, "from": 1, "maxParas": 5 }),
        ),
        "bookmarks",
        "name",
    );
    let out = apply(
        &made,
        json!([
            { "op": "insertBookmark", "id": para, "name": "NewOne" },
            { "op": "setBookmark", "id": "bookmark:1", "name": "Renamed" }
        ]),
    );
    let after = column(
        &inspect(&out, json!({ "bookmarks": true, "from": 1, "maxParas": 5 })),
        "bookmarks",
        "name",
    );
    assert!(after.contains(&"NewOne".to_string()));
    assert!(after.contains(&"Renamed".to_string()));
    assert!(!after.contains(&names[0]));
    assert!(after.contains(&names[1]));
}

#[test]
fn deleted_bookmark_is_not_found() {
    let (src, para) = host_para("bm-gone");
    let made = apply(
        &src,
        json!([
            { "op": "insertBookmark", "id": para, "name": "Aa" },
            { "op": "insertBookmark", "id": para, "name": "Bb" }
        ]),
    );
    let err = apply_err(
        &made,
        json!([
            { "op": "deleteBookmark", "id": "bookmark:1" },
            { "op": "setBookmark", "id": "bookmark:1", "name": "Nope" }
        ]),
    );
    assert_eq!(err.code(), ErrorCode::IdNotFound);
}

#[test]
fn delete_then_set_original_field() {
    let (src, para) = host_para("fld");
    let made = apply(
        &src,
        json!([
            { "op": "insertField", "id": para, "instr": " QUOTE aaa " },
            { "op": "insertField", "id": para, "instr": " QUOTE bbb " },
            { "op": "insertField", "id": para, "instr": " QUOTE ccc " }
        ]),
    );
    let before = column(
        &inspect(&made, json!({ "fields": true, "from": 1, "maxParas": 5 })),
        "fields",
        "instr",
    );
    let out = apply(
        &made,
        json!([
            { "op": "deleteField", "id": "field:1" },
            { "op": "setField", "id": "field:2", "instr": " QUOTE BBB " }
        ]),
    );
    let after = column(
        &inspect(&out, json!({ "fields": true, "from": 1, "maxParas": 5 })),
        "fields",
        "instr",
    );
    assert!(!after.iter().any(|s| s.contains("aaa")));
    assert!(!after.iter().any(|s| s.contains("bbb")));
    assert!(after.iter().any(|s| s.contains("BBB")));
    assert!(after.iter().any(|s| s.contains("ccc")));
    assert_eq!(before.len(), 3);
}

#[test]
fn delete_then_set_original_link() {
    let (src, para) = host_para("link");
    let made = apply(
        &src,
        json!([
            { "op": "insertHyperlink", "id": para, "text": "甲", "url": "https://a.example" },
            { "op": "insertHyperlink", "id": para, "text": "乙", "url": "https://b.example" },
            { "op": "insertHyperlink", "id": para, "text": "丙", "url": "https://c.example" }
        ]),
    );
    let out = apply(
        &made,
        json!([
            { "op": "deleteHyperlink", "id": "link:1" },
            { "op": "setHyperlink", "id": "link:2", "url": "https://b2.example" }
        ]),
    );
    let urls = column(
        &inspect(
            &out,
            json!({ "hyperlinks": true, "from": 1, "maxParas": 5 }),
        ),
        "hyperlinks",
        "url",
    );
    assert!(!urls.iter().any(|s| s.contains("a.example")));
    assert!(!urls.iter().any(|s| s.contains("https://b.example")));
    assert!(urls.iter().any(|s| s.contains("b2.example")));
    assert!(urls.iter().any(|s| s.contains("c.example")));
}

#[test]
fn delete_then_set_original_math() {
    let (src, para) = host_para("math");
    let made = apply(
        &src,
        json!([
            { "op": "insertMath", "id": para, "text": "aaa" },
            { "op": "insertMath", "id": para, "text": "bbb" },
            { "op": "insertMath", "id": para, "text": "ccc" }
        ]),
    );
    let out = apply(
        &made,
        json!([
            { "op": "deleteMath", "id": "math:1" },
            { "op": "setMath", "id": "math:2", "text": "BBB" }
        ]),
    );
    let texts = column(
        &inspect(&out, json!({ "maths": true, "from": 1, "maxParas": 5 })),
        "maths",
        "text",
    );
    assert_eq!(texts, vec!["BBB".to_string(), "ccc".to_string()]);
}

#[test]
fn delete_then_set_original_ruby() {
    let (src, para) = host_para("ruby");
    let made = apply(
        &src,
        json!([
            { "op": "insertRuby", "id": para, "base": "汉", "rt": "aaa" },
            { "op": "insertRuby", "id": para, "base": "字", "rt": "bbb" },
            { "op": "insertRuby", "id": para, "base": "词", "rt": "ccc" }
        ]),
    );
    let out = apply(
        &made,
        json!([
            { "op": "deleteRuby", "id": "ruby:1" },
            { "op": "setRuby", "id": "ruby:2", "rt": "BBB" }
        ]),
    );
    let rts = column(
        &inspect(&out, json!({ "rubies": true, "from": 1, "maxParas": 5 })),
        "rubies",
        "rt",
    );
    assert_eq!(rts, vec!["BBB".to_string(), "ccc".to_string()]);
}

#[test]
fn delete_then_set_original_sdt() {
    let (src, para) = host_para("sdt");
    let made = apply(
        &src,
        json!([
            { "op": "insertContentControl", "id": para, "kind": "plainText", "text": "aaa" },
            { "op": "insertContentControl", "id": para, "kind": "plainText", "text": "bbb" },
            { "op": "insertContentControl", "id": para, "kind": "plainText", "text": "ccc" }
        ]),
    );
    let out = apply(
        &made,
        json!([
            { "op": "deleteContentControl", "id": "sdt:1" },
            { "op": "setContentControl", "id": "sdt:2", "text": "BBB" }
        ]),
    );
    let texts = column(
        &inspect(
            &out,
            json!({ "contentControls": true, "from": 1, "maxParas": 5 }),
        ),
        "contentControls",
        "text",
    );
    assert_eq!(texts, vec!["BBB".to_string(), "ccc".to_string()]);
}

#[test]
fn delete_then_original_perm() {
    let (src, para) = host_para("perm");
    let made = apply(
        &src,
        json!([
            { "op": "insertEditRange", "id": para, "ed": "aaa" },
            { "op": "insertEditRange", "id": para, "ed": "bbb" },
            { "op": "insertEditRange", "id": para, "ed": "ccc" }
        ]),
    );
    let before = column(
        &inspect(
            &made,
            json!({ "editRanges": true, "from": 1, "maxParas": 5 }),
        ),
        "editRanges",
        "ed",
    );
    let out = apply(
        &made,
        json!([
            { "op": "deleteEditRange", "id": "perm:1" },
            { "op": "deleteEditRange", "id": "perm:2" }
        ]),
    );
    let after = column(
        &inspect(
            &out,
            json!({ "editRanges": true, "from": 1, "maxParas": 5 }),
        ),
        "editRanges",
        "ed",
    );
    assert_eq!(before.len(), 3);
    assert_eq!(after, vec![before[2].clone()]);
}

#[test]
fn delete_then_original_revision() {
    let inner = r#"<w:ins w:id="1" w:author="a"><w:r><w:t>Gone1</w:t></w:r></w:ins><w:ins w:id="2" w:author="a"><w:r><w:t>Stay2</w:t></w:r></w:ins>"#;
    let src = bare_package("rev", &document_xml(inner), &style_rels(), &[]);
    let out = apply(
        &src,
        json!([
            { "op": "rejectRevision", "id": "rev:1" },
            { "op": "acceptRevision", "id": "rev:2" }
        ]),
    );
    let xml = part(&out, "word/document.xml");
    assert!(!xml.contains("Gone1"), "{xml}");
    assert!(xml.contains("Stay2"), "{xml}");
    assert!(!xml.contains("w:ins"), "{xml}");
}

#[test]
fn delete_then_format_original_run() {
    let inner = r#"<w:ins w:id="1" w:author="a"><w:r><w:t>AAA</w:t></w:r></w:ins><w:r><w:t>BBB</w:t></w:r><w:r><w:t>CCC</w:t></w:r>"#;
    let src = bare_package("run", &document_xml(inner), &style_rels(), &[]);
    let out = apply(
        &src,
        json!([
            { "op": "rejectRevision", "id": "rev:1" },
            { "op": "setRunFormat", "id": "para:AAAA0001/run:2", "bold": true }
        ]),
    );
    let tree = inspect(&out, json!({ "runs": true, "from": 1, "maxParas": 5 }));
    let runs = tree["paragraphs"][0]["runs"].as_array().unwrap();
    assert_eq!(runs[0]["text"], "BBB");
    assert_eq!(runs[0]["bold"], true);
    assert_eq!(runs[1]["text"], "CCC");
    assert!(runs[1]["bold"].is_null());
}

#[test]
fn header_bookmark_then_original() {
    let src = create(
        "hdr-bm",
        json!({
            "paragraphs": [{ "text": "正文" }],
            "headers": [{ "type": "default", "paragraphs": [{ "text": "眉" }] }]
        }),
    );
    let tree = inspect(
        &src,
        json!({ "headerFooter": true, "from": 1, "maxParas": 5 }),
    );
    let para = tree["headers"][0]["paragraphs"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let made = apply(
        &src,
        json!([
            { "op": "insertBookmark", "id": para, "name": "Ha" },
            { "op": "insertBookmark", "id": para, "name": "Hb" },
            { "op": "insertBookmark", "id": para, "name": "Hc" }
        ]),
    );
    let listed = inspect(
        &made,
        json!({ "headerFooter": true, "bookmarks": true, "from": 1, "maxParas": 5 }),
    );
    let names = column(&listed["headers"][0], "bookmarks", "name");
    let id2 = listed["headers"][0]["bookmarks"][1]["id"].as_str().unwrap();
    let id1 = listed["headers"][0]["bookmarks"][0]["id"].as_str().unwrap();
    let out = apply(
        &made,
        json!([
            { "op": "deleteBookmark", "id": id1 },
            { "op": "setBookmark", "id": id2, "name": "Hren" }
        ]),
    );
    let after = inspect(
        &out,
        json!({ "headerFooter": true, "bookmarks": true, "from": 1, "maxParas": 5 }),
    );
    let got = column(&after["headers"][0], "bookmarks", "name");
    assert!(!got.contains(&names[0]));
    assert!(!got.contains(&names[1]));
    assert!(got.contains(&names[2]));
    assert!(got.contains(&"Hren".to_string()));
}

#[test]
fn box_field_then_original() {
    let src = create(
        "box",
        json!({
            "paragraphs": [{ "text": "外", "textBoxes": [{ "text": "框" }] }]
        }),
    );
    let tree = inspect(
        &src,
        json!({ "textBoxes": true, "textBoxContent": true, "from": 1, "maxParas": 5 }),
    );
    let inner = tree["paragraphs"][0]["textBoxes"][0]["paragraphs"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let made = apply(
        &src,
        json!([
            { "op": "insertField", "id": inner, "instr": " QUOTE aaa " },
            { "op": "insertField", "id": inner, "instr": " QUOTE bbb " },
            { "op": "insertField", "id": inner, "instr": " QUOTE ccc " }
        ]),
    );
    let listed = inspect(
        &made,
        json!({ "textBoxes": true, "fields": true, "from": 1, "maxParas": 5 }),
    );
    let fields = &listed["paragraphs"][0]["textBoxes"][0]["fields"];
    let id1 = fields[0]["id"].as_str().unwrap();
    let id2 = fields[1]["id"].as_str().unwrap();
    let out = apply(
        &made,
        json!([
            { "op": "deleteField", "id": id1 },
            { "op": "setField", "id": id2, "instr": " QUOTE BBB " }
        ]),
    );
    let after = inspect(
        &out,
        json!({ "textBoxes": true, "fields": true, "from": 1, "maxParas": 5 }),
    );
    let instrs = column(&after["paragraphs"][0]["textBoxes"][0], "fields", "instr");
    assert!(instrs.iter().any(|s| s.contains("BBB")));
    assert!(instrs.iter().any(|s| s.contains("ccc")));
    assert!(!instrs.iter().any(|s| s.contains("aaa")));
    assert!(!instrs.iter().any(|s| s.contains("bbb")));
}

#[test]
fn box_math_then_original() {
    let src = create(
        "box-math",
        json!({
            "paragraphs": [{ "text": "外", "textBoxes": [{ "text": "框" }] }]
        }),
    );
    let tree = inspect(
        &src,
        json!({ "textBoxes": true, "textBoxContent": true, "from": 1, "maxParas": 5 }),
    );
    let inner = tree["paragraphs"][0]["textBoxes"][0]["paragraphs"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let made = apply(
        &src,
        json!([
            { "op": "insertMath", "id": inner, "text": "aaa" },
            { "op": "insertMath", "id": inner, "text": "bbb" },
            { "op": "insertMath", "id": inner, "text": "ccc" }
        ]),
    );
    let listed = inspect(
        &made,
        json!({ "textBoxes": true, "maths": true, "from": 1, "maxParas": 5 }),
    );
    let maths = &listed["paragraphs"][0]["textBoxes"][0]["maths"];
    let id1 = maths[0]["id"].as_str().unwrap();
    let id2 = maths[1]["id"].as_str().unwrap();
    let out = apply(
        &made,
        json!([
            { "op": "deleteMath", "id": id1 },
            { "op": "setMath", "id": id2, "text": "BBB" }
        ]),
    );
    let after = inspect(
        &out,
        json!({ "textBoxes": true, "maths": true, "from": 1, "maxParas": 5 }),
    );
    let texts = column(&after["paragraphs"][0]["textBoxes"][0], "maths", "text");
    assert_eq!(texts, vec!["BBB".to_string(), "ccc".to_string()]);
}

fn pic(mark: &str) -> String {
    format!(
        r#"<w:r><w:drawing><wp:inline><a:graphic><a:graphicData><pic:pic><pic:blipFill><a:blip r:embed="{mark}"/></pic:blipFill></pic:pic></a:graphicData></a:graphic></wp:inline></w:drawing></w:r>"#
    )
}

#[test]
fn picture_index_then_original() {
    let inner = format!("{}{}{}", pic("rIdA"), pic("rIdB"), pic("rIdC"));
    delete_two(
        "pic",
        &inner,
        &style_rels(),
        &[],
        "deletePicture",
        "para:AAAA0001/pic:index",
        &["rIdA", "rIdB", "rIdC"],
    );
}

#[test]
fn header_picture_index_then_original() {
    let header = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:hdr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture"><w:p>{}{}{}</w:p></w:hdr>"#,
        pic("rIdA"),
        pic("rIdB"),
        pic("rIdC")
    );
    let document = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><w:body><w:p><w:r><w:t>正文</w:t></w:r></w:p><w:sectPr><w:headerReference w:type="default" r:id="rIdH"/></w:sectPr></w:body></w:document>"#;
    let rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rIdStyle" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/><Relationship Id="rIdH" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/header" Target="header1.xml"/></Relationships>"#;
    let ct = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/><Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/><Override PartName="/word/header1.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.header+xml"/></Types>"#;
    let root = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#;
    let src = write_parts(
        "hdr-pic",
        &[
            ("[Content_Types].xml", ct),
            ("_rels/.rels", root),
            ("word/document.xml", document),
            ("word/_rels/document.xml.rels", rels),
            ("word/styles.xml", STYLES),
            ("word/header1.xml", &header),
        ],
    );
    let out = apply(
        &src,
        json!([
            { "op": "deletePicture", "id": "header:default:1/pic:index:1" },
            { "op": "deletePicture", "id": "header:default:1/pic:index:2" }
        ]),
    );
    assert_kept(&part(&out, "word/header1.xml"), &["rIdA", "rIdB"], "rIdC");
}

fn txbx(mark: &str) -> String {
    format!(
        r#"<w:r><w:drawing><wps:wsp><wps:txbx><w:txbxContent><w:p><w:r><w:t>{mark}</w:t></w:r></w:p></w:txbxContent></wps:txbx></wps:wsp></w:drawing></w:r>"#
    )
}

#[test]
fn textbox_index_then_original() {
    let inner = format!("{}{}{}", txbx("MarkA"), txbx("MarkB"), txbx("MarkC"));
    delete_two(
        "txbx",
        &inner,
        &style_rels(),
        &[],
        "deleteTextBox",
        "para:AAAA0001/txbx:index",
        &["MarkA", "MarkB", "MarkC"],
    );
}

fn shape(mark: &str) -> String {
    format!(
        r#"<w:r><w:drawing><wps:wsp><wps:spPr><a:prstGeom prst="{mark}"/></wps:spPr></wps:wsp></w:drawing></w:r>"#
    )
}

#[test]
fn shape_index_then_original() {
    let inner = format!("{}{}{}", shape("rect"), shape("ellipse"), shape("triangle"));
    delete_two(
        "shape",
        &inner,
        &style_rels(),
        &[],
        "deleteShape",
        "para:AAAA0001/shape:index",
        &["rect", "ellipse", "triangle"],
    );
}

fn cxn(mark: &str) -> String {
    format!(
        r#"<w:r><w:drawing><wps:wsp><wps:cNvCnPr descr="{mark}"/><wps:spPr><a:prstGeom prst="straightConnector1"/></wps:spPr></wps:wsp></w:drawing></w:r>"#
    )
}

#[test]
fn connector_index_then_original() {
    let inner = format!("{}{}{}", cxn("CxnA"), cxn("CxnB"), cxn("CxnC"));
    delete_two(
        "cxn",
        &inner,
        &style_rels(),
        &[],
        "deleteConnector",
        "para:AAAA0001/cxn:index",
        &["CxnA", "CxnB", "CxnC"],
    );
}

fn chart(mark: &str) -> String {
    format!(
        r#"<w:r><w:drawing><wp:inline><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/chart"><c:chart r:id="{mark}"/></a:graphicData></a:graphic></wp:inline></w:drawing></w:r>"#
    )
}

#[test]
fn chart_index_then_original() {
    let inner = format!("{}{}{}", chart("rIdC1"), chart("rIdC2"), chart("rIdC3"));
    let rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rIdStyle" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/><Relationship Id="rIdC1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart" Target="charts/chart1.xml"/><Relationship Id="rIdC2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart" Target="charts/chart2.xml"/><Relationship Id="rIdC3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart" Target="charts/chart3.xml"/></Relationships>"#;
    let chart_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart"/>"#.to_string();
    delete_two(
        "chart",
        &inner,
        rels,
        &[
            ("word/charts/chart1.xml", chart_xml.clone()),
            ("word/charts/chart2.xml", chart_xml.clone()),
            ("word/charts/chart3.xml", chart_xml),
        ],
        "deleteChart",
        "para:AAAA0001/chart:index",
        &["rIdC1", "rIdC2", "rIdC3"],
    );
}

fn diagram(mark: &str) -> String {
    format!(
        r#"<w:r><w:drawing><wp:inline><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/diagram"><dgm:relIds r:dm="{mark}"/></a:graphicData></a:graphic></wp:inline></w:drawing></w:r>"#
    )
}

#[test]
fn smartart_index_then_original() {
    let inner = format!(
        "{}{}{}",
        diagram("rIdD1"),
        diagram("rIdD2"),
        diagram("rIdD3")
    );
    delete_two(
        "dgm",
        &inner,
        &style_rels(),
        &[],
        "deleteSmartArt",
        "para:AAAA0001/smartdiagram:index",
        &["rIdD1", "rIdD2", "rIdD3"],
    );
}

fn ole(mark: &str) -> String {
    format!(
        r#"<w:r><w:object><o:OLEObject r:id="{mark}" ProgID="Package" Type="Embed"/></w:object></w:r>"#
    )
}

#[test]
fn ole_index_then_original() {
    let inner = format!("{}{}{}", ole("rIdO1"), ole("rIdO2"), ole("rIdO3"));
    let rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rIdStyle" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/><Relationship Id="rIdO1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/oleObject" Target="embeddings/a.bin"/><Relationship Id="rIdO2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/oleObject" Target="embeddings/b.bin"/><Relationship Id="rIdO3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/oleObject" Target="embeddings/c.bin"/></Relationships>"#;
    delete_two(
        "ole",
        &inner,
        rels,
        &[
            ("word/embeddings/a.bin", "a".into()),
            ("word/embeddings/b.bin", "b".into()),
            ("word/embeddings/c.bin", "c".into()),
        ],
        "deleteOle",
        "para:AAAA0001/ole:index",
        &["rIdO1", "rIdO2", "rIdO3"],
    );
}

fn ink(mark: &str) -> String {
    format!(
        r#"<w:r><w:drawing><wpc:wpc><w14:contentPart r:id="{mark}"/></wpc:wpc></w:drawing></w:r>"#
    )
}

#[test]
fn ink_index_then_original() {
    let inner = format!("{}{}{}", ink("rIdI1"), ink("rIdI2"), ink("rIdI3"));
    let rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rIdStyle" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/><Relationship Id="rIdI1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/customXml" Target="ink/ink1.xml"/><Relationship Id="rIdI2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/customXml" Target="ink/ink2.xml"/><Relationship Id="rIdI3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/customXml" Target="ink/ink3.xml"/></Relationships>"#;
    let ink_xml =
        r#"<?xml version="1.0" encoding="UTF-8"?><ink xmlns="http://www.w3.org/2003/InkML"/>"#
            .to_string();
    delete_two(
        "ink",
        &inner,
        rels,
        &[
            ("word/ink/ink1.xml", ink_xml.clone()),
            ("word/ink/ink2.xml", ink_xml.clone()),
            ("word/ink/ink3.xml", ink_xml),
        ],
        "deleteInkCanvas",
        "para:AAAA0001/ink:index",
        &["rIdI1", "rIdI2", "rIdI3"],
    );
}

fn model(mark: &str) -> String {
    format!(
        r#"<w:r><w:drawing><wp:inline><a:graphic><a:graphicData uri="http://schemas.microsoft.com/office/drawing/2017/model3d"><am3d:model3d r:embed="{mark}"/></a:graphicData></a:graphic></wp:inline></w:drawing></w:r>"#
    )
}

#[test]
fn model3d_index_then_original() {
    let inner = format!("{}{}{}", model("rIdM1"), model("rIdM2"), model("rIdM3"));
    delete_two(
        "m3d",
        &inner,
        &style_rels(),
        &[],
        "deleteModel3d",
        "para:AAAA0001/model3d:index",
        &["rIdM1", "rIdM2", "rIdM3"],
    );
}

fn watermark(mark: &str) -> String {
    format!(
        r#"<w:r><w:drawing><wp:anchor><wp:docPr name="PowerPlusWaterMarkObject{mark}"/></wp:anchor></w:drawing></w:r>"#
    )
}

#[test]
fn watermark_index_then_original() {
    let header = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:hdr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing"><w:p>{}{}{}</w:p></w:hdr>"#,
        watermark("Aa"),
        watermark("Bb"),
        watermark("Cc")
    );
    let document = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><w:body><w:p><w:r><w:t>正文</w:t></w:r></w:p><w:sectPr><w:headerReference w:type="default" r:id="rIdH"/></w:sectPr></w:body></w:document>"#;
    let rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rIdStyle" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/><Relationship Id="rIdH" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/header" Target="header1.xml"/></Relationships>"#;
    let ct = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/><Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/><Override PartName="/word/header1.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.header+xml"/></Types>"#;
    let root = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#;
    let src = write_parts(
        "wm",
        &[
            ("[Content_Types].xml", ct),
            ("_rels/.rels", root),
            ("word/document.xml", document),
            ("word/_rels/document.xml.rels", rels),
            ("word/styles.xml", STYLES),
            ("word/header1.xml", &header),
        ],
    );
    let out = apply(
        &src,
        json!([
            { "op": "deleteWatermark", "id": "header:default:1/watermark:index:1" },
            { "op": "deleteWatermark", "id": "header:default:1/watermark:index:2" }
        ]),
    );
    assert_kept(
        &part(&out, "word/header1.xml"),
        &["PowerPlusWaterMarkObjectAa", "PowerPlusWaterMarkObjectBb"],
        "PowerPlusWaterMarkObjectCc",
    );
}
