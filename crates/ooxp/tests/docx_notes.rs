//! docx 脚注、尾注：计数、名单、改字、插删、编号、注内段与图。

use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::json;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

const CT: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Default Extension="png" ContentType="image/png"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/><Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/><Override PartName="/word/settings.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.settings+xml"/><Override PartName="/word/footnotes.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.footnotes+xml"/><Override PartName="/word/endnotes.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.endnotes+xml"/></Types>"#;

const ROOT_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#;

const DOC_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/settings" Target="settings.xml"/><Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/footnotes" Target="footnotes.xml"/><Relationship Id="rId4" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/endnotes" Target="endnotes.xml"/></Relationships>"#;

const STYLES: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/></w:style></w:styles>"#;

const SETTINGS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:settings xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:footnotePr><w:footnote w:id="0"/><w:footnote w:id="1"/></w:footnotePr><w:endnotePr><w:endnote w:id="0"/><w:endnote w:id="1"/></w:endnotePr></w:settings>"#;

const DOCUMENT: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><w:body><w:p w14:paraId="AAAA0001"><w:r><w:t>标题</w:t></w:r></w:p><w:p w14:paraId="AAAA0002"><w:r><w:t>正文</w:t></w:r><w:r><w:footnoteReference w:id="2"/></w:r><w:r><w:endnoteReference w:id="2"/></w:r></w:p><w:p w14:paraId="AAAA0003"><w:r><w:t>第三段</w:t></w:r></w:p><w:sectPr><w:pgSz w:w="11906" w:h="16838"/></w:sectPr></w:body></w:document>"#;

const FOOTNOTES: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:footnotes xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml"><w:footnote w:type="separator" w:id="0"><w:p w14:paraId="F0000001"><w:r><w:separator/></w:r></w:p></w:footnote><w:footnote w:type="continuationSeparator" w:id="1"><w:p w14:paraId="F0000002"><w:r><w:continuationSeparator/></w:r></w:p></w:footnote><w:footnote w:id="2"><w:p w14:paraId="F00A0002"><w:r><w:footnoteRef/></w:r><w:r><w:t>脚注原文</w:t></w:r></w:p></w:footnote></w:footnotes>"#;

const ENDNOTES: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:endnotes xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml"><w:endnote w:type="separator" w:id="0"><w:p w14:paraId="E0000001"><w:r><w:separator/></w:r></w:p></w:endnote><w:endnote w:type="continuationSeparator" w:id="1"><w:p w14:paraId="E0000002"><w:r><w:continuationSeparator/></w:r></w:p></w:endnote><w:endnote w:id="2"><w:p w14:paraId="E00A0002"><w:r><w:endnoteRef/></w:r><w:r><w:t>尾注原文</w:t></w:r></w:p></w:endnote></w:endnotes>"#;

const PNG_1X1: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
    0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08, 0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00,
    0x00, 0x03, 0x01, 0x01, 0x00, 0x18, 0xDD, 0x8D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E,
    0x44, 0xAE, 0x42, 0x60, 0x82,
];

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-docx-notes");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_sample(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let path = tmp_dir().join(format!("{tag}-{n}.docx"));
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut zip = ZipWriter::new(&mut cursor);
        let opt = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        for (name, body) in [
            ("[Content_Types].xml", CT.as_bytes()),
            ("_rels/.rels", ROOT_RELS.as_bytes()),
            ("word/document.xml", DOCUMENT.as_bytes()),
            ("word/_rels/document.xml.rels", DOC_RELS.as_bytes()),
            ("word/styles.xml", STYLES.as_bytes()),
            ("word/settings.xml", SETTINGS.as_bytes()),
            ("word/footnotes.xml", FOOTNOTES.as_bytes()),
            ("word/endnotes.xml", ENDNOTES.as_bytes()),
        ] {
            zip.start_file(name, opt).unwrap();
            zip.write_all(body).unwrap();
        }
        zip.finish().unwrap();
    }
    std::fs::write(&path, cursor.into_inner()).unwrap();
    path
}

fn write_png(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let path = tmp_dir().join(format!("{tag}-{n}.png"));
    std::fs::write(&path, PNG_1X1).unwrap();
    path
}

fn inspect(path: &Path, scope: serde_json::Value) -> serde_json::Value {
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

fn apply(path: &Path, out: &Path, ops: serde_json::Value) {
    run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": path.to_string_lossy(),
            "output": out.to_string_lossy(),
            "ops": ops
        })
        .to_string(),
        None,
    )
    .unwrap();
}

fn apply_err(path: &Path, ops: serde_json::Value) -> ooxp::Error {
    let out = tmp_dir().join("err.docx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": path.to_string_lossy(),
            "output": out.to_string_lossy(),
            "ops": ops
        })
        .to_string(),
        None,
    )
    .unwrap_err()
}

fn object_keys(v: &serde_json::Value) -> Vec<&str> {
    v.as_object().unwrap().keys().map(String::as_str).collect()
}

fn part_text(path: &Path, name: &str) -> String {
    let bytes = std::fs::read(path).unwrap();
    let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut file = zip.by_name(name.trim_start_matches('/')).unwrap();
    let mut s = String::new();
    std::io::Read::read_to_string(&mut file, &mut s).unwrap();
    s
}

#[test]
fn inspect_counts_without_lists() {
    let path = write_sample("counts");
    let tree = inspect(&path, json!({}));
    assert_eq!(tree["footnoteCount"], 1);
    assert_eq!(tree["endnoteCount"], 1);
    assert_eq!(tree["commentCount"], 0);
    assert!(tree.get("comments").is_none());
    assert!(tree.get("footnotes").is_none());
    assert!(tree.get("endnotes").is_none());
    assert!(tree.get("noteSeparators").is_none());
    assert_eq!(
        object_keys(&tree),
        [
            "schemaVersion",
            "format",
            "id",
            "paraCount",
            "pictureCount",
            "textBoxCount",
            "connectorCount",
            "shapeCount",
            "inkCanvasCount",
            "footnoteCount",
            "endnoteCount",
            "commentCount",
            "chartCount",
            "smartArtCount",
            "oleObjectCount",
            "model3dCount",
            "watermarkCount",
            "tableCount",
            "revisionCount",
            "hyperlinkCount",
            "bookmarkCount",
            "fieldCount",
            "tocCount",
            "captionCount",
            "crossRefCount",
            "indexCount",
            "indexEntryCount",
            "sourceCount",
            "citationCount",
            "bibliographyCount",
            "docPartCount",
            "mathCount",
            "rubyCount",
            "contentControlCount",
            "numberingCount",
            "styleCount",
            "embeddedFontCount",
            "outline",
            "headers",
            "footers",
            "sections",
            "mirrorMargins",
            "gutterAtTop",
            "props",
            "protection",
            "theme",
            "themeOverride",
            "fontEmbed",
            "pageBackground",
            "eastAsianSettings"
        ]
    );
}

#[test]
fn inspect_notes_lists_hosts_and_separators() {
    let path = write_sample("lists");
    let tree = inspect(&path, json!({ "from": 1, "maxParas": 20, "notes": true }));
    assert_eq!(tree["footnotes"][0]["id"], "footnote:2");
    assert_eq!(tree["footnotes"][0]["noteId"], "2");
    assert_eq!(tree["footnotes"][0]["type"], "normal");
    assert_eq!(tree["footnotes"][0]["host"][0], "para:AAAA0002");
    assert_eq!(tree["footnotes"][0]["paraCount"], 1);
    assert_eq!(
        tree["footnotes"][0]["paragraphs"][0]["id"],
        "footnote:2/para:F00A0002"
    );
    assert_eq!(tree["footnotes"][0]["paragraphs"][0]["text"], "脚注原文");
    assert_eq!(tree["endnotes"][0]["id"], "endnote:2");
    assert_eq!(tree["endnotes"][0]["host"][0], "para:AAAA0002");
    let seps = tree["noteSeparators"].as_array().unwrap();
    assert!(seps.iter().any(|s| s["id"] == "footnote:sep:separator"));
    assert!(seps
        .iter()
        .any(|s| s["id"] == "endnote:sep:continuationSeparator"));
    assert_eq!(tree["paragraphs"][1]["footnoteRef"][0], "footnote:2");
    assert_eq!(tree["paragraphs"][1]["endnoteRef"][0], "endnote:2");
    assert!(tree["paragraphs"][0].get("footnoteRef").is_none());
    assert_eq!(
        object_keys(&tree["footnotes"][0]),
        [
            "id",
            "noteId",
            "type",
            "host",
            "paraCount",
            "pictureCount",
            "textBoxCount",
            "connectorCount",
            "chartCount",
            "smartArtCount",
            "paragraphs"
        ]
    );
}

#[test]
fn set_para_text_in_note() {
    let path = write_sample("set-text");
    let out = tmp_dir().join("set-text-out.docx");
    apply(
        &path,
        &out,
        json!([{
            "op": "setParaText",
            "id": "footnote:2/para:F00A0002",
            "text": "改过的注文"
        }]),
    );
    let tree = inspect(&out, json!({ "notes": true }));
    assert_eq!(tree["footnotes"][0]["paragraphs"][0]["text"], "改过的注文");
    let body = inspect(&out, json!({ "from": 1, "maxParas": 20 }));
    assert_eq!(body["paragraphs"][1]["text"], "正文");
    let xml = part_text(&out, "word/footnotes.xml");
    assert!(xml.contains("footnoteRef"));
    assert!(!part_text(&out, "word/document.xml").contains("改过的注文"));
}

#[test]
fn insert_and_delete_footnote() {
    let path = write_sample("ins-del");
    let inserted = tmp_dir().join("ins.docx");
    apply(
        &path,
        &inserted,
        json!([{
            "op": "insertFootnote",
            "id": "para:AAAA0003",
            "text": "新脚注"
        }]),
    );
    let tree = inspect(
        &inserted,
        json!({ "from": 1, "maxParas": 20, "notes": true }),
    );
    assert_eq!(tree["footnoteCount"], 2);
    let new_id = tree["footnotes"][1]["id"].as_str().unwrap();
    assert_eq!(new_id, "footnote:3");
    assert!(tree["paragraphs"][2]["footnoteRef"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v == "footnote:3"));
    assert!(part_text(&inserted, "word/footnotes.xml").contains("新脚注"));

    let deleted = tmp_dir().join("del.docx");
    apply(
        &inserted,
        &deleted,
        json!([{ "op": "deleteFootnote", "id": "footnote:3" }]),
    );
    let tree = inspect(
        &deleted,
        json!({ "from": 1, "maxParas": 20, "notes": true }),
    );
    assert_eq!(tree["footnoteCount"], 1);
    assert!(tree["paragraphs"][2].get("footnoteRef").is_none());
    assert!(part_text(&deleted, "word/footnotes.xml").contains(r#"w:type="separator""#));
    assert!(!part_text(&deleted, "word/footnotes.xml").contains("新脚注"));
}

#[test]
fn insert_footnote_creates_part_on_create_file() {
    let path = tmp_dir().join("create-host.docx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "document": { "paragraphs": [{ "text": "hello" }] }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let tree0 = inspect(&path, json!({ "from": 1, "maxParas": 5 }));
    let para = tree0["paragraphs"][0]["id"].as_str().unwrap();
    let out = tmp_dir().join("create-fn.docx");
    apply(
        &path,
        &out,
        json!([{ "op": "insertFootnote", "id": para, "text": "从无到有" }]),
    );
    let tree = inspect(&out, json!({ "notes": true, "from": 1, "maxParas": 5 }));
    assert_eq!(tree["footnoteCount"], 1);
    assert_eq!(tree["footnotes"][0]["id"], "footnote:2");
    assert!(part_text(&out, "word/settings.xml").contains("footnotePr"));
}

#[test]
fn set_note_numbering_and_reject_separator() {
    let path = write_sample("num");
    let out = tmp_dir().join("num-out.docx");
    apply(
        &path,
        &out,
        json!([{
            "op": "setNoteNumbering",
            "id": "docx:document",
            "kind": "footnote",
            "numFmt": "upperRoman"
        }]),
    );
    let settings = part_text(&out, "word/settings.xml");
    let fmt = settings.find("numFmt").expect("numFmt");
    let spec = settings.find(r#"w:footnote w:id="0""#).expect("sep ref");
    assert!(fmt < spec, "numFmt 必须写在分隔符引用前面");
    assert!(settings.contains("upperRoman"));
    let body = part_text(&out, "word/document.xml");
    let sect_fmt = body.find("numFmt").expect("sect numFmt");
    let pgsz = body.find("pgSz").expect("pgSz");
    assert!(sect_fmt < pgsz, "sectPr 里 numFmt 必须写在 pgSz 前面");

    let err = apply_err(
        &path,
        json!([{ "op": "setParaText", "id": "footnote:sep:separator", "text": "x" }]),
    );
    assert_eq!(err.code(), ErrorCode::InvalidId);
    let err = apply_err(
        &path,
        json!([{ "op": "deleteFootnote", "id": "footnote:sep:separator" }]),
    );
    assert_eq!(err.code(), ErrorCode::InvalidId);

    let out_sect = tmp_dir().join("num-sect.docx");
    apply(
        &path,
        &out_sect,
        json!([{
            "op": "setNoteNumbering",
            "id": "section:1",
            "kind": "endnote",
            "numFmt": "lowerLetter"
        }]),
    );
    let body_sect = part_text(&out_sect, "word/document.xml");
    assert!(body_sect.contains("lowerLetter"));
    let end_pr = body_sect.find("endnotePr").expect("endnotePr");
    let pgsz2 = body_sect.find("pgSz").expect("pgSz");
    assert!(end_pr < pgsz2, "endnotePr 必须写在 pgSz 前面");
    assert!(!part_text(&out_sect, "word/settings.xml").contains("lowerLetter"));
}

#[test]
fn insert_para_and_picture_in_note() {
    let path = write_sample("para-pic");
    let png = write_png("note-pic");
    let out = tmp_dir().join("para-pic-out.docx");
    apply(
        &path,
        &out,
        json!([
            { "op": "insertPara", "id": "footnote:2", "text": "注内第二段" },
            {
                "op": "insertPicture",
                "id": "footnote:2/para:F00A0002",
                "file": png.to_string_lossy()
            }
        ]),
    );
    let tree = inspect(&out, json!({ "notes": true, "pictures": true }));
    assert_eq!(tree["footnotes"][0]["paraCount"], 2);
    assert_eq!(tree["footnotes"][0]["pictureCount"], 1);
    assert!(tree["footnotes"][0]["pictures"][0]["id"]
        .as_str()
        .unwrap()
        .starts_with("footnote:2/para:"));
    assert_eq!(tree["pictureCount"], 0);

    let err = apply_err(
        &path,
        json!([{ "op": "deletePara", "id": "footnote:2/para:F00A0002" }]),
    );
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

#[test]
fn body_para_id_does_not_hit_note() {
    let path = write_sample("no-cross");
    let err = apply_err(
        &path,
        json!([{ "op": "setParaText", "id": "para:F00A0002", "text": "x" }]),
    );
    assert_eq!(err.code(), ErrorCode::IdNotFound);
}
