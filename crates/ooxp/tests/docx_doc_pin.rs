//! 同一批里增删文档级名单之后，原来的编号仍指向开始时的那一项。

use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, RunOutput};
use serde_json::{json, Value};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join("ooxp-docx-doc-pin");
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

fn para_id(path: &Path) -> String {
    let tree = inspect(path, json!({ "from": 1, "maxParas": 5 }));
    tree["paragraphs"][0]["id"].as_str().unwrap().to_string()
}

fn host() -> (PathBuf, String) {
    let src = create("host", json!({ "paragraphs": [{ "text": "宿主" }] }));
    let id = para_id(&src);
    (src, id)
}

#[test]
fn insert_section_then_original_margin() {
    let src = create(
        "sect",
        json!({
            "paragraphs": [
                { "text": "一", "sectionType": "nextPage" },
                { "text": "二" }
            ]
        }),
    );
    let tree = inspect(&src, json!({ "from": 1, "maxParas": 5 }));
    let p1 = tree["paragraphs"][0]["id"].as_str().unwrap();
    let out = apply(
        &src,
        json!([
            { "op": "insertSection", "id": p1, "type": "continuous" },
            { "op": "setPageMargin", "id": "section:2", "top": 2222 }
        ]),
    );
    let after = inspect(&out, json!({ "from": 1, "maxParas": 8 }));
    let sections = after["sections"].as_array().unwrap();
    assert!(sections.len() >= 3, "{sections:?}");
    let last = sections.last().unwrap();
    assert_eq!(last["top"], 2222, "{sections:?}");
    assert_ne!(sections[1]["top"], 2222, "{sections:?}");
}

#[test]
fn unlink_then_original_header() {
    let src = write_two_headers();
    let out = apply(
        &src,
        json!([
            {
                "op": "setHeaderFooterLink",
                "id": "section:2",
                "kind": "header",
                "type": "default",
                "linked": false
            },
            {
                "op": "setParaText",
                "id": "header:default:2/para:CCCC0002",
                "text": "改二"
            }
        ]),
    );
    let h1 = part_xml(&out, "word/header1.xml");
    let h2 = part_xml(&out, "word/header2.xml");
    assert!(h2.contains("改二"), "{h2}");
    assert!(!h2.contains("眉二"), "{h2}");
    assert!(h1.contains("眉一"), "{h1}");
    assert!(!h1.contains("改二"), "{h1}");
}

fn part_xml(path: &Path, name: &str) -> String {
    let file = std::fs::File::open(path).unwrap();
    let mut zip = ZipArchive::new(file).unwrap();
    let mut entry = zip.by_name(name).unwrap();
    let mut s = String::new();
    std::io::Read::read_to_string(&mut entry, &mut s).unwrap();
    s
}

#[test]
fn delete_then_original_toc() {
    let (src, para) = host();
    let made = apply(
        &src,
        json!([
            { "op": "insertToc", "id": para, "instr": " TOC \\o \"1-1\" " },
            { "op": "insertToc", "id": para, "instr": " TOC \\o \"1-2\" " },
            { "op": "insertToc", "id": para, "instr": " TOC \\o \"1-3\" " }
        ]),
    );
    let before = inspect(&made, json!({ "tocs": true, "from": 1, "maxParas": 10 }));
    let kept = before["tocs"][2]["instr"].as_str().unwrap().to_string();
    let out = apply(
        &made,
        json!([
            { "op": "deleteToc", "id": "toc:1" },
            { "op": "deleteToc", "id": "toc:2" }
        ]),
    );
    let tree = inspect(&out, json!({ "tocs": true, "from": 1, "maxParas": 10 }));
    let tocs = tree["tocs"].as_array().unwrap();
    assert_eq!(tocs.len(), 1);
    assert_eq!(tocs[0]["instr"].as_str().unwrap(), kept);
}

#[test]
fn delete_then_original_caption() {
    let (src, para) = host();
    let made = apply(
        &src,
        json!([
            { "op": "insertCaption", "id": para, "label": "Figure", "title": "甲" },
            { "op": "insertCaption", "id": para, "label": "Figure", "title": "乙" },
            { "op": "insertCaption", "id": para, "label": "Figure", "title": "丙" }
        ]),
    );
    let before = inspect(
        &made,
        json!({ "captions": true, "from": 1, "maxParas": 10 }),
    );
    let kept = before["captions"][2].clone();
    let out = apply(
        &made,
        json!([
            { "op": "deleteCaption", "id": "caption:1" },
            { "op": "deleteCaption", "id": "caption:2" }
        ]),
    );
    let tree = inspect(&out, json!({ "captions": true, "from": 1, "maxParas": 10 }));
    let caps = tree["captions"].as_array().unwrap();
    assert_eq!(caps.len(), 1);
    assert_eq!(caps[0]["text"], kept["text"]);
}

#[test]
fn delete_then_original_source() {
    let src = create("src", json!({ "paragraphs": [{ "text": "宿主" }] }));
    let made = apply(
        &src,
        json!([
            { "op": "insertSource", "id": "docx:document", "tag": "Aaa", "title": "甲" },
            { "op": "insertSource", "id": "docx:document", "tag": "Bbb", "title": "乙" },
            { "op": "insertSource", "id": "docx:document", "tag": "Ccc", "title": "丙" }
        ]),
    );
    let out = apply(
        &made,
        json!([
            { "op": "deleteSource", "id": "source:1" },
            { "op": "deleteSource", "id": "source:2" }
        ]),
    );
    let tree = inspect(&out, json!({ "sources": true }));
    let sources = tree["sources"].as_array().unwrap();
    assert_eq!(sources.len(), 1);
    assert_eq!(sources[0]["tag"], "Ccc");
}

#[test]
fn delete_then_original_doc_part() {
    let src = create("part", json!({ "paragraphs": [{ "text": "宿主" }] }));
    let made = apply(
        &src,
        json!([
            { "op": "insertDocPart", "id": "docx:document", "name": "Aaa", "text": "甲" },
            { "op": "insertDocPart", "id": "docx:document", "name": "Bbb", "text": "乙" },
            { "op": "insertDocPart", "id": "docx:document", "name": "Ccc", "text": "丙" }
        ]),
    );
    let out = apply(
        &made,
        json!([
            { "op": "deleteDocPart", "id": "docPart:1" },
            { "op": "deleteDocPart", "id": "docPart:2" }
        ]),
    );
    let tree = inspect(&out, json!({ "docParts": true }));
    let parts = tree["docParts"].as_array().unwrap();
    assert_eq!(parts.len(), 1);
    assert_eq!(parts[0]["name"], "Ccc");
}

#[test]
fn delete_then_original_xref() {
    let (src, para) = host();
    let made = apply(
        &src,
        json!([
            { "op": "insertBookmark", "id": para, "name": "Aa" },
            { "op": "insertCrossRef", "id": para, "bookmark": "Aa", "kind": "ref" },
            { "op": "insertCrossRef", "id": para, "bookmark": "Aa", "kind": "page" },
            { "op": "insertCrossRef", "id": para, "bookmark": "Aa", "kind": "ref" }
        ]),
    );
    let before = inspect(
        &made,
        json!({ "crossRefs": true, "from": 1, "maxParas": 10 }),
    );
    let kept = before["crossRefs"][2]["instr"].clone();
    let out = apply(
        &made,
        json!([
            { "op": "deleteCrossRef", "id": "xref:1" },
            { "op": "deleteCrossRef", "id": "xref:2" }
        ]),
    );
    let tree = inspect(
        &out,
        json!({ "crossRefs": true, "from": 1, "maxParas": 10 }),
    );
    let refs = tree["crossRefs"].as_array().unwrap();
    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0]["instr"], kept);
}

#[test]
fn delete_then_original_index() {
    let src = create("idx", json!({ "paragraphs": [{ "text": "宿主" }] }));
    let made = apply(
        &src,
        json!([
            { "op": "insertIndex", "id": "docx:document", "instr": " INDEX \\h \"A\" " },
            { "op": "insertIndex", "id": "docx:document", "instr": " INDEX \\h \"B\" " },
            { "op": "insertIndex", "id": "docx:document", "instr": " INDEX \\h \"C\" " }
        ]),
    );
    let before = inspect(&made, json!({ "indexes": true, "from": 1, "maxParas": 10 }));
    let kept = before["indexes"][2]["instr"].clone();
    let out = apply(
        &made,
        json!([
            { "op": "deleteIndex", "id": "index:1" },
            { "op": "deleteIndex", "id": "index:2" }
        ]),
    );
    let tree = inspect(&out, json!({ "indexes": true, "from": 1, "maxParas": 10 }));
    let items = tree["indexes"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["instr"], kept);
}

#[test]
fn delete_then_original_index_entry() {
    let (src, para) = host();
    let made = apply(
        &src,
        json!([
            { "op": "insertIndexEntry", "id": para, "entry": "Aaa" },
            { "op": "insertIndexEntry", "id": para, "entry": "Bbb" },
            { "op": "insertIndexEntry", "id": para, "entry": "Ccc" }
        ]),
    );
    let before = inspect(
        &made,
        json!({ "indexEntries": true, "from": 1, "maxParas": 10 }),
    );
    let kept = before["indexEntries"][2]["entry"].clone();
    let out = apply(
        &made,
        json!([
            { "op": "deleteIndexEntry", "id": "xe:1" },
            { "op": "deleteIndexEntry", "id": "xe:2" }
        ]),
    );
    let tree = inspect(
        &out,
        json!({ "indexEntries": true, "from": 1, "maxParas": 10 }),
    );
    let items = tree["indexEntries"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["entry"], kept);
}

#[test]
fn delete_then_original_citation_and_bibliography() {
    let (src, para) = host();
    let made = apply(
        &src,
        json!([
            { "op": "insertSource", "id": "docx:document", "tag": "Aaa", "title": "甲" },
            { "op": "insertCitation", "id": para, "tag": "Aaa" },
            { "op": "insertCitation", "id": para, "tag": "Aaa" },
            { "op": "insertCitation", "id": para, "tag": "Aaa" },
            { "op": "insertBibliography", "id": "docx:document" },
            { "op": "insertBibliography", "id": "docx:document" },
            { "op": "insertBibliography", "id": "docx:document" }
        ]),
    );
    let before = inspect(
        &made,
        json!({ "citations": true, "bibliographies": true, "from": 1, "maxParas": 20 }),
    );
    assert_eq!(before["citations"].as_array().unwrap().len(), 3);
    assert_eq!(before["bibliographies"].as_array().unwrap().len(), 3);
    let out = apply(
        &made,
        json!([
            { "op": "deleteCitation", "id": "cite:1" },
            { "op": "deleteCitation", "id": "cite:2" },
            { "op": "deleteBibliography", "id": "biblio:1" },
            { "op": "deleteBibliography", "id": "biblio:2" }
        ]),
    );
    let tree = inspect(
        &out,
        json!({ "citations": true, "bibliographies": true, "from": 1, "maxParas": 20 }),
    );
    assert_eq!(tree["citations"].as_array().unwrap().len(), 1);
    assert_eq!(tree["bibliographies"].as_array().unwrap().len(), 1);
}

#[test]
fn delete_then_original_font() {
    let font_path = std::env::temp_dir()
        .join("ooxp-docx-doc-pin")
        .join("sample.ttf");
    std::fs::create_dir_all(font_path.parent().unwrap()).unwrap();
    // 只要过 TTF 文件头检查。名字来自请求，不读本机字体。
    let mut font_bytes = vec![0, 1, 0, 0];
    font_bytes.resize(64, 0x11);
    std::fs::write(&font_path, font_bytes).unwrap();
    let font = font_path.to_string_lossy();
    let src = create("font", json!({ "paragraphs": [{ "text": "宿主" }] }));
    let made = apply(
        &src,
        json!([
            { "op": "insertEmbeddedFont", "id": "docx:document", "name": "Calibri", "file": font, "style": "regular" },
            { "op": "insertEmbeddedFont", "id": "docx:document", "name": "Calibri", "file": font, "style": "bold" },
            { "op": "insertEmbeddedFont", "id": "docx:document", "name": "Calibri", "file": font, "style": "italic" }
        ]),
    );
    let before = inspect(&made, json!({ "embeddedFonts": true }));
    let kept = before["embeddedFonts"][2]["style"].clone();
    let out = apply(
        &made,
        json!([
            { "op": "deleteEmbeddedFont", "id": "font:1" },
            { "op": "deleteEmbeddedFont", "id": "font:2" }
        ]),
    );
    let tree = inspect(&out, json!({ "embeddedFonts": true }));
    let fonts = tree["embeddedFonts"].as_array().unwrap();
    assert_eq!(fonts.len(), 1);
    assert_eq!(fonts[0]["style"], kept);
}

fn write_two_headers() -> PathBuf {
    let path = tmp("hdr");
    let ct = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/><Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/><Override PartName="/word/header1.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.header+xml"/><Override PartName="/word/header2.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.header+xml"/></Types>"#;
    let root = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#;
    let rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rIdStyle" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/><Relationship Id="rIdH1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/header" Target="header1.xml"/><Relationship Id="rIdH2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/header" Target="header2.xml"/></Relationships>"#;
    let styles = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/></w:style></w:styles>"#;
    let document = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml"><w:body><w:p w14:paraId="AAAA0001"><w:pPr><w:sectPr><w:headerReference w:type="default" r:id="rIdH1"/><w:type w:val="nextPage"/><w:pgMar w:top="1440" w:bottom="1440" w:left="1440" w:right="1440"/></w:sectPr></w:pPr><w:r><w:t>一</w:t></w:r></w:p><w:p w14:paraId="AAAA0002"><w:pPr><w:sectPr><w:type w:val="nextPage"/><w:pgMar w:top="1440" w:bottom="1440" w:left="1440" w:right="1440"/></w:sectPr></w:pPr><w:r><w:t>二</w:t></w:r></w:p><w:p w14:paraId="AAAA0003"><w:r><w:t>三</w:t></w:r></w:p><w:sectPr><w:headerReference w:type="default" r:id="rIdH2"/><w:pgMar w:top="1440" w:bottom="1440" w:left="1440" w:right="1440"/></w:sectPr></w:body></w:document>"#;
    let h1 = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:hdr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml"><w:p w14:paraId="BBBB0001"><w:r><w:t>眉一</w:t></w:r></w:p></w:hdr>"#;
    let h2 = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:hdr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml"><w:p w14:paraId="CCCC0002"><w:r><w:t>眉二</w:t></w:r></w:p></w:hdr>"#;
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut zip = ZipWriter::new(&mut cursor);
        let opt = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        for (name, body) in [
            ("[Content_Types].xml", ct),
            ("_rels/.rels", root),
            ("word/document.xml", document),
            ("word/_rels/document.xml.rels", rels),
            ("word/styles.xml", styles),
            ("word/header1.xml", h1),
            ("word/header2.xml", h2),
        ] {
            zip.start_file(name, opt).unwrap();
            zip.write_all(body.as_bytes()).unwrap();
        }
        zip.finish().unwrap();
    }
    std::fs::write(&path, cursor.into_inner()).unwrap();
    path
}
