//! docx 东亚排版：段禁则、run 强调 / 压缩、拼音、整篇 settings。

use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

const CT: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>"#;

const ROOT_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#;

const DOC_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/officeDocument/2006/relationships"></Relationships>"#;

const DOCUMENT: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml"><w:body><w:p w14:paraId="AAAA0001"><w:r><w:t>汉</w:t></w:r></w:p><w:sectPr><w:pgSz w:w="11906" w:h="16838"/></w:sectPr></w:body></w:document>"#;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-docx-east-asia");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_zip(tag: &str, files: &[(&str, &str)]) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let path = tmp_dir().join(format!("{tag}-{n}.docx"));
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut zip = ZipWriter::new(&mut cursor);
        let opt = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        for (name, body) in files {
            zip.start_file(*name, opt).unwrap();
            zip.write_all(body.as_bytes()).unwrap();
        }
        zip.finish().unwrap();
    }
    std::fs::write(&path, cursor.into_inner()).unwrap();
    path
}

fn write_sample(tag: &str) -> PathBuf {
    write_zip(
        tag,
        &[
            ("[Content_Types].xml", CT),
            ("_rels/.rels", ROOT_RELS),
            ("word/document.xml", DOCUMENT),
            ("word/_rels/document.xml.rels", DOC_RELS),
        ],
    )
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

fn apply(path: &Path, out: &Path, ops: Value) {
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

fn apply_err(path: &Path, ops: Value) -> ooxp::Error {
    let out = tmp_dir().join("should-not-write.docx");
    let _ = std::fs::remove_file(&out);
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

fn unzip(path: &Path, name: &str) -> String {
    let file = std::fs::File::open(path).unwrap();
    let mut zip = zip::ZipArchive::new(file).unwrap();
    let mut part = zip.by_name(name).unwrap();
    let mut s = String::new();
    part.read_to_string(&mut s).unwrap();
    s
}

fn object_keys(v: &Value) -> Vec<&str> {
    v.as_object().unwrap().keys().map(String::as_str).collect()
}

fn para<'a>(tree: &'a Value, id: &str) -> &'a Value {
    tree["paragraphs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == id)
        .unwrap()
}

#[test]
fn inspect_always_has_ruby_count_and_settings() {
    let path = write_sample("bare");
    let tree = inspect(&path, json!({}));
    assert_eq!(tree["rubyCount"], 0);
    assert!(tree.get("rubies").is_none());
    assert_eq!(
        object_keys(&tree["eastAsianSettings"]),
        [
            "characterSpacingControl",
            "strictFirstAndLastChars",
            "noLineBreaksBefore",
            "noLineBreaksAfter",
            "themeFontLang"
        ]
    );
    assert!(tree["eastAsianSettings"]["characterSpacingControl"].is_null());
    assert!(tree["eastAsianSettings"]["themeFontLang"].is_null());
    let keys = object_keys(&tree);
    let math = keys.iter().position(|k| *k == "mathCount").unwrap();
    assert_eq!(keys[math + 1], "rubyCount");
    let bg = keys.iter().position(|k| *k == "pageBackground").unwrap();
    assert_eq!(keys[bg + 1], "eastAsianSettings");
    assert_eq!(keys[bg + 2], "bibliographyStyle");
}

#[test]
fn set_para_east_asia_and_order() {
    let path = write_sample("ppr");
    let out = tmp_dir().join("ppr-out.docx");
    apply(
        &path,
        &out,
        json!([{
            "op": "setParaFormat",
            "id": "para:AAAA0001",
            "kinsoku": true,
            "overflowPunct": true,
            "autoSpaceDE": false,
            "before": 120
        }]),
    );
    let tree = inspect(&out, json!({ "from": 1, "maxParas": 20 }));
    let p = para(&tree, "para:AAAA0001");
    assert_eq!(p["eastAsia"]["kinsoku"], true);
    assert_eq!(p["eastAsia"]["overflowPunct"], true);
    assert!(p["eastAsia"]["topLinePunct"].is_null());
    assert_eq!(p["eastAsia"]["autoSpaceDE"], false);
    assert!(p["eastAsia"]["autoSpaceDN"].is_null());
    assert_eq!(
        object_keys(&p["eastAsia"]),
        [
            "kinsoku",
            "overflowPunct",
            "topLinePunct",
            "autoSpaceDE",
            "autoSpaceDN"
        ]
    );
    let xml = unzip(&out, "word/document.xml");
    let k = xml.find("w:kinsoku").unwrap();
    let sp = xml.find("w:spacing").unwrap();
    assert!(k < sp, "{xml}");
}

#[test]
fn set_run_em_and_layout() {
    let path = write_sample("run");
    let out = tmp_dir().join("run-out.docx");
    apply(
        &path,
        &out,
        json!([{
            "op": "setRunFormat",
            "id": "para:AAAA0001/run:1",
            "em": "dot",
            "eastAsianLayout": {
                "id": -1,
                "combine": true,
                "combineBrackets": "curly"
            }
        }]),
    );
    let tree = inspect(&out, json!({ "from": 1, "maxParas": 20, "runs": true }));
    let r = &para(&tree, "para:AAAA0001")["runs"][0];
    assert_eq!(r["em"], "dot");
    assert_eq!(r["eastAsianLayout"]["id"], -1);
    assert_eq!(r["eastAsianLayout"]["combine"], true);
    assert_eq!(r["eastAsianLayout"]["combineBrackets"], "curly");
    assert!(r["eastAsianLayout"]["vert"].is_null());
    let xml = unzip(&out, "word/document.xml");
    assert!(xml.contains("w:em"));
    assert!(xml.contains("w:eastAsianLayout"));
    let cs_pos = xml.find("w:em").unwrap();
    let layout_pos = xml.find("w:eastAsianLayout").unwrap();
    assert!(cs_pos < layout_pos, "{xml}");
}

#[test]
fn insert_set_delete_ruby() {
    let path = write_sample("ruby");
    let inserted = tmp_dir().join("ruby-ins.docx");
    apply(
        &path,
        &inserted,
        json!([{
            "op": "insertRuby",
            "id": "para:AAAA0001",
            "base": "汉",
            "rt": "hàn"
        }]),
    );
    let tree = inspect(
        &inserted,
        json!({ "from": 1, "maxParas": 20, "rubies": true }),
    );
    assert_eq!(tree["rubyCount"], 1);
    assert_eq!(tree["rubies"][0]["id"], "ruby:1");
    assert_eq!(tree["rubies"][0]["base"], "汉");
    assert_eq!(tree["rubies"][0]["rt"], "hàn");
    assert_eq!(tree["rubies"][0]["align"], "distributeSpace");
    assert_eq!(tree["rubies"][0]["hps"], 12);
    assert_eq!(tree["rubies"][0]["lid"], "zh-CN");
    assert_eq!(tree["rubies"][0]["host"], "para:AAAA0001");
    assert_eq!(
        object_keys(&tree["rubies"][0]),
        [
            "id",
            "index",
            "host",
            "base",
            "rt",
            "align",
            "hps",
            "hpsRaise",
            "hpsBaseText",
            "lid"
        ]
    );
    let xml = unzip(&inserted, "word/document.xml");
    assert!(xml.contains("w:ruby"));
    assert!(xml.contains("w:rubyBase"));

    let set = tmp_dir().join("ruby-set.docx");
    apply(
        &inserted,
        &set,
        json!([{ "op": "setRuby", "id": "ruby:1", "rt": "han" }]),
    );
    let tree2 = inspect(&set, json!({ "rubies": true }));
    assert_eq!(tree2["rubies"][0]["rt"], "han");
    assert_eq!(tree2["rubies"][0]["base"], "汉");

    let err_text = apply_err(
        &set,
        json!([{ "op": "setParaText", "id": "para:AAAA0001", "text": "x" }]),
    );
    assert_eq!(err_text.code(), ErrorCode::NotImplemented);

    let gone = tmp_dir().join("ruby-del.docx");
    apply(&set, &gone, json!([{ "op": "deleteRuby", "id": "ruby:1" }]));
    let tree3 = inspect(&gone, json!({ "from": 1, "maxParas": 20, "rubies": true }));
    assert_eq!(tree3["rubyCount"], 0);
    assert_eq!(tree3["rubies"].as_array().unwrap().len(), 0);
    assert_eq!(para(&tree3, "para:AAAA0001")["text"], "汉");
    let xml3 = unzip(&gone, "word/document.xml");
    assert!(!xml3.contains("w:ruby"));
}

#[test]
fn set_east_asian_settings_creates_part() {
    let path = write_sample("set");
    let out = tmp_dir().join("set-out.docx");
    apply(
        &path,
        &out,
        json!([{
            "op": "setEastAsianSettings",
            "id": "docx:document",
            "characterSpacingControl": "compressPunctuation",
            "strictFirstAndLastChars": false,
            "noLineBreaksBefore": { "lang": "zh-CN", "chars": "([{（" }
        }]),
    );
    let tree = inspect(&out, json!({}));
    assert_eq!(
        tree["eastAsianSettings"]["characterSpacingControl"],
        "compressPunctuation"
    );
    assert_eq!(tree["eastAsianSettings"]["strictFirstAndLastChars"], false);
    assert_eq!(
        tree["eastAsianSettings"]["noLineBreaksBefore"]["lang"],
        "zh-CN"
    );
    let xml = unzip(&out, "word/settings.xml");
    assert!(xml.contains("w:characterSpacingControl"));
    assert!(xml.contains("w:strictFirstAndLastChars"));
    assert!(xml.contains("w:noLineBreaksBefore"));
    let csc = xml.find("w:characterSpacingControl").unwrap();
    let strict = xml.find("w:strictFirstAndLastChars").unwrap();
    assert!(csc < strict, "{xml}");
    let lang_out = tmp_dir().join("set-lang-out.docx");
    apply(
        &out,
        &lang_out,
        json!([{
            "op": "setEastAsianSettings",
            "id": "docx:document",
            "themeFontLang": { "val": "ja-JP" }
        }]),
    );
    let tree = inspect(&lang_out, json!({}));
    assert_eq!(tree["eastAsianSettings"]["themeFontLang"]["val"], "ja-JP");
    assert!(tree["eastAsianSettings"]["themeFontLang"]["eastAsia"].is_null());
    let xml = unzip(&lang_out, "word/settings.xml");
    assert!(xml.contains(r#"w:val="ja-JP""#));
    assert!(xml.contains("w:themeFontLang"));
}

#[test]
fn rejects_empty_base_and_xlsx() {
    let path = write_sample("err");
    let err = apply_err(
        &path,
        json!([{
            "op": "insertRuby",
            "id": "para:AAAA0001",
            "base": "",
            "rt": "a"
        }]),
    );
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let xlsx = tmp_dir().join("ea.xlsx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": xlsx.to_string_lossy(),
            "workbook": { "sheets": [{ "name": "Sheet1" }] }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let out = tmp_dir().join("ea-x-out.xlsx");
    let err_x = run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": xlsx.to_string_lossy(),
            "output": out.to_string_lossy(),
            "ops": [{
                "op": "insertRuby",
                "id": "para:AAAA0001",
                "base": "汉",
                "rt": "han"
            }]
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err_x.code(), ErrorCode::InvalidField);
    let err_scope = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": xlsx.to_string_lossy(),
            "scope": { "rubies": true }
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err_scope.code(), ErrorCode::InvalidField);
}
