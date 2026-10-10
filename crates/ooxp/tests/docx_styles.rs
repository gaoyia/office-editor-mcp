//! docx 样式：名单、换 pStyle、改定义、xlsx/pptx 拒绝。

use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

const CT: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/><Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/></Types>"#;

const ROOT_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#;

const DOC_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/></Relationships>"#;

const STYLES: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:docDefaults><w:rPrDefault><w:rPr><w:sz w:val="22"/></w:rPr></w:rPrDefault></w:docDefaults><w:latentStyles w:count="1"/><w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/><w:qFormat/></w:style><w:style w:type="paragraph" w:styleId="Heading1"><w:name w:val="heading 1"/><w:basedOn w:val="Normal"/><w:pPr><w:outlineLvl w:val="0"/></w:pPr><w:rPr><w:b/><w:sz w:val="32"/></w:rPr></w:style><w:style w:type="paragraph" w:styleId="Heading2"><w:name w:val="heading 2"/><w:basedOn w:val="Normal"/><w:pPr><w:outlineLvl w:val="1"/></w:pPr></w:style><w:style w:type="character" w:styleId="DefaultParagraphFont"><w:name w:val="Default Paragraph Font"/><w:semiHidden/></w:style><w:style w:styleId=""><w:name w:val="empty"/></w:style></w:styles>"#;

const DOCUMENT: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml"><w:body><w:p w14:paraId="AAAA0001"><w:r><w:t>正文</w:t></w:r></w:p><w:p w14:paraId="AAAA0002"><w:pPr><w:pStyle w:val="Heading1"/></w:pPr><w:r><w:t>标题</w:t></w:r></w:p><w:sectPr><w:pgSz w:w="11906" w:h="16838"/></w:sectPr></w:body></w:document>"#;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-docx-styles");
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
            ("word/styles.xml", STYLES),
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

fn object_keys(v: &Value) -> Vec<&str> {
    v.as_object().unwrap().keys().map(String::as_str).collect()
}

fn listed(path: &Path) -> Value {
    inspect(path, json!({ "styles": true, "from": 1, "maxParas": 20 }))
}

#[test]
fn style_count_without_scope() {
    let path = write_sample("count");
    let tree = inspect(&path, json!({}));
    assert_eq!(tree["styleCount"], 4);
    assert!(tree.get("styles").is_none());
}

#[test]
fn style_list_skips_empty_and_latent() {
    let path = write_sample("list");
    let tree = listed(&path);
    assert_eq!(tree["styleCount"], 4);
    let styles = tree["styles"].as_array().unwrap();
    assert_eq!(styles.len(), 4);
    assert_eq!(
        object_keys(&styles[0]),
        ["id", "styleId", "type", "name", "basedOn", "default"]
    );
    assert_eq!(styles[0]["id"], "style:Normal");
    assert_eq!(styles[0]["styleId"], "Normal");
    assert_eq!(styles[0]["type"], "paragraph");
    assert_eq!(styles[0]["name"], "Normal");
    assert!(styles[0]["basedOn"].is_null());
    assert_eq!(styles[0]["default"], true);
    assert_eq!(styles[1]["id"], "style:Heading1");
    assert_eq!(styles[1]["basedOn"], "Normal");
    assert_eq!(styles[1]["default"], false);
    assert_eq!(styles[3]["id"], "style:DefaultParagraphFont");
    assert_eq!(styles[3]["type"], "character");
}

#[test]
fn set_para_style_heading_updates_outline() {
    let path = write_sample("para");
    let out = tmp_dir().join("para-out.docx");
    apply(
        &path,
        &out,
        json!([{
            "op": "setParaStyle",
            "id": "para:AAAA0001",
            "style": "style:Heading2"
        }]),
    );
    let tree = listed(&out);
    let p1 = tree["paragraphs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == "para:AAAA0001")
        .unwrap();
    assert_eq!(p1["style"], "Heading2");
    assert_eq!(p1["level"], 2);
    let outline = tree["outline"].as_array().unwrap();
    assert!(outline
        .iter()
        .any(|o| o["id"] == "para:AAAA0001" && o["level"] == 2));
}

#[test]
fn set_style_writes_rpr_and_spacing() {
    let path = write_sample("set");
    let out = tmp_dir().join("set-out.docx");
    apply(
        &path,
        &out,
        json!([{
            "op": "setStyle",
            "id": "style:Normal",
            "font": "Calibri",
            "eastAsiaFont": "宋体",
            "sz": 48,
            "bold": false,
            "color": "ff0000",
            "before": 120,
            "after": 240
        }]),
    );
    let unpacked = unzip_styles(&out);
    assert!(unpacked.contains("styleId=\"Normal\""));
    assert!(unpacked.contains("val=\"48\""));
    assert!(unpacked.contains("ascii=\"Calibri\""));
    assert!(unpacked.contains("eastAsia=\"宋体\""));
    assert!(unpacked.contains("val=\"FF0000\""));
    assert!(unpacked.contains("before=\"120\""));
    assert!(unpacked.contains("after=\"240\""));
    assert!(unpacked.contains("val=\"0\""));
}

fn unzip_styles(path: &Path) -> String {
    let file = std::fs::File::open(path).unwrap();
    let mut zip = zip::ZipArchive::new(file).unwrap();
    let mut part = zip.by_name("word/styles.xml").unwrap();
    let mut s = String::new();
    std::io::Read::read_to_string(&mut part, &mut s).unwrap();
    s
}

#[test]
fn unbind_para_style() {
    let path = write_sample("unbind");
    let out = tmp_dir().join("unbind-out.docx");
    apply(
        &path,
        &out,
        json!([{
            "op": "setParaStyle",
            "id": "para:AAAA0002",
            "style": null
        }]),
    );
    let tree = listed(&out);
    let p2 = tree["paragraphs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == "para:AAAA0002")
        .unwrap();
    assert!(p2["style"].is_null());
}

#[test]
fn style_ops_reject_bad_ids() {
    let path = write_sample("bad");
    let err = apply_err(
        &path,
        json!([{ "op": "setStyle", "id": "para:AAAA0001", "sz": 24 }]),
    );
    assert_eq!(err.code(), ErrorCode::InvalidId);

    let err_missing = apply_err(
        &path,
        json!([{ "op": "setStyle", "id": "style:NoSuch", "sz": 24 }]),
    );
    assert_eq!(err_missing.code(), ErrorCode::IdNotFound);

    let err_char_spacing = apply_err(
        &path,
        json!([{ "op": "setStyle", "id": "style:DefaultParagraphFont", "before": 100 }]),
    );
    assert_eq!(err_char_spacing.code(), ErrorCode::InvalidField);

    let err_empty = apply_err(&path, json!([{ "op": "setStyle", "id": "style:Normal" }]));
    assert_eq!(err_empty.code(), ErrorCode::InvalidField);

    let err_color = apply_err(
        &path,
        json!([{ "op": "setStyle", "id": "style:Normal", "color": "#ff0000" }]),
    );
    assert_eq!(err_color.code(), ErrorCode::InvalidField);

    let err_para_char = apply_err(
        &path,
        json!([{
            "op": "setParaStyle",
            "id": "para:AAAA0001",
            "style": "style:DefaultParagraphFont"
        }]),
    );
    assert_eq!(err_para_char.code(), ErrorCode::InvalidField);

    let err_need = apply_err(
        &path,
        json!([{ "op": "setParaStyle", "id": "para:AAAA0001" }]),
    );
    assert_eq!(err_need.code(), ErrorCode::InvalidField);
}

#[test]
fn styles_not_for_xlsx_or_pptx() {
    let xlsx = tmp_dir().join("st.xlsx");
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
    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": xlsx.to_string_lossy(),
            "scope": { "styles": true }
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    let out = tmp_dir().join("st-x-out.xlsx");
    let err_op = run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": xlsx.to_string_lossy(),
            "output": out.to_string_lossy(),
            "ops": [{ "op": "setStyle", "id": "style:Normal", "sz": 48 }]
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err_op.code(), ErrorCode::InvalidField);

    let pptx = tmp_dir().join("st.pptx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": pptx.to_string_lossy(),
            "presentation": { "slides": [{ "shapes": [{ "text": "a" }] }] }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let err_p = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": pptx.to_string_lossy(),
            "scope": { "styles": true }
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err_p.code(), ErrorCode::InvalidField);
}

#[test]
fn insert_and_delete_character_style() {
    let path = write_sample("ins-char");
    let mid = tmp_dir().join("ins-char.docx");
    apply(
        &path,
        &mid,
        json!([{
            "op": "insertStyle",
            "id": "docx:document",
            "styleId": "ooxpChar",
            "type": "character"
        }]),
    );
    let tree = listed(&mid);
    assert_eq!(tree["styleCount"], 5);
    let found = tree["styles"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["id"] == "style:ooxpChar")
        .unwrap();
    assert_eq!(found["type"], "character");
    assert_eq!(found["name"], "ooxpChar");

    apply(
        &mid,
        &tmp_dir().join("ins-char-sz.docx"),
        json!([{ "op": "setStyle", "id": "style:ooxpChar", "color": "00FF00", "bold": true }]),
    );
    let xml = unzip_styles(&tmp_dir().join("ins-char-sz.docx"));
    assert!(xml.contains("styleId=\"ooxpChar\""));
    assert!(xml.contains("00FF00") || xml.contains("val=\"00FF00\""));

    let out = tmp_dir().join("del-char.docx");
    apply(
        &mid,
        &out,
        json!([{ "op": "deleteStyle", "id": "style:ooxpChar" }]),
    );
    let after = listed(&out);
    assert_eq!(after["styleCount"], 4);
    assert!(after["styles"]
        .as_array()
        .unwrap()
        .iter()
        .all(|s| s["id"] != "style:ooxpChar"));

    let err_normal = apply_err(
        &path,
        json!([{ "op": "deleteStyle", "id": "style:Normal" }]),
    );
    assert_eq!(err_normal.code(), ErrorCode::InvalidField);
    let err_dup = apply_err(
        &mid,
        json!([{
            "op": "insertStyle",
            "id": "docx:document",
            "styleId": "ooxpChar",
            "type": "character"
        }]),
    );
    assert_eq!(err_dup.code(), ErrorCode::InvalidField);
    let err_num = apply_err(
        &path,
        json!([{
            "op": "insertStyle",
            "id": "docx:document",
            "styleId": "n1",
            "type": "numbering"
        }]),
    );
    assert_eq!(err_num.code(), ErrorCode::InvalidField);
}

#[test]
fn set_run_style_binds_rstyle() {
    let path = write_sample("run-st");
    let mid = tmp_dir().join("run-st-ins.docx");
    apply(
        &path,
        &mid,
        json!([{
            "op": "insertStyle",
            "id": "docx:document",
            "styleId": "ooxpChar",
            "type": "character"
        }]),
    );
    let out = tmp_dir().join("run-st.docx");
    apply(
        &mid,
        &out,
        json!([{
            "op": "setRunStyle",
            "id": "para:AAAA0001/run:1",
            "style": "style:ooxpChar"
        }]),
    );
    let tree = inspect(&out, json!({ "from": 1, "maxParas": 20, "runs": true }));
    let p = tree["paragraphs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == "para:AAAA0001")
        .unwrap();
    assert_eq!(p["runs"][0]["style"], "ooxpChar");
    let err_para = apply_err(
        &out,
        json!([{
            "op": "setRunStyle",
            "id": "para:AAAA0001/run:1",
            "style": "style:Normal"
        }]),
    );
    assert_eq!(err_para.code(), ErrorCode::InvalidField);
}
