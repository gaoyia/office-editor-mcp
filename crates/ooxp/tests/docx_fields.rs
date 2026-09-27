//! docx 域：名单、插改删、setParaText 守卫。

use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::json;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

const CT: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/><Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/><Override PartName="/word/header1.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.header+xml"/></Types>"#;

const ROOT_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#;

const DOC_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/header" Target="header1.xml"/></Relationships>"#;

const STYLES: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/></w:style></w:styles>"#;

const DOCUMENT: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml"><w:body><w:p w14:paraId="AAAA0001"><w:r><w:fldChar w:fldCharType="begin"/></w:r><w:r><w:instrText xml:space="preserve"> PAGE </w:instrText></w:r><w:r><w:fldChar w:fldCharType="separate"/></w:r><w:r><w:t>1</w:t></w:r><w:r><w:fldChar w:fldCharType="end"/></w:r></w:p><w:p w14:paraId="AAAA0002"><w:fldSimple w:instr=" DATE \@ &quot;yyyy&quot; "><w:r><w:t>2026</w:t></w:r></w:fldSimple></w:p><w:p w14:paraId="AAAA0003"><w:r><w:fldChar w:fldCharType="begin"/></w:r><w:r><w:instrText xml:space="preserve"> HYPERLINK &quot;https://example.com&quot; </w:instrText></w:r><w:r><w:fldChar w:fldCharType="separate"/></w:r><w:r><w:t>点这里</w:t></w:r><w:r><w:fldChar w:fldCharType="end"/></w:r></w:p><w:p w14:paraId="AAAA0004"><w:r><w:fldChar w:fldCharType="begin"/></w:r><w:r><w:instrText xml:space="preserve"> AUTHOR </w:instrText></w:r><w:r><w:t>未闭合</w:t></w:r></w:p><w:p w14:paraId="AAAA0005"><w:r><w:fldChar w:fldCharType="begin"/></w:r><w:r><w:instrText xml:space="preserve"> IF </w:instrText></w:r><w:fldSimple w:instr=" DATE "><w:r><w:t>1</w:t></w:r></w:fldSimple><w:r><w:instrText xml:space="preserve"> = 1 &quot;yes&quot; &quot;no&quot; </w:instrText></w:r><w:r><w:fldChar w:fldCharType="separate"/></w:r><w:r><w:t>yes</w:t></w:r><w:r><w:fldChar w:fldCharType="end"/></w:r></w:p><w:p w14:paraId="AAAA0006"><w:r><w:t>干净</w:t></w:r></w:p><w:p w14:paraId="AAAA0007"><w:r><w:fldChar w:fldCharType="begin"/></w:r><w:r><w:instrText xml:space="preserve"> = 1+1 </w:instrText></w:r><w:r><w:fldChar w:fldCharType="separate"/></w:r><w:r><w:t>2</w:t></w:r><w:r><w:fldChar w:fldCharType="end"/></w:r></w:p><w:sectPr><w:headerReference w:type="default" r:id="rId2"/><w:pgSz w:w="11906" w:h="16838"/></w:sectPr></w:body></w:document>"#;

const HEADER: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:hdr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml"><w:p w14:paraId="BBBB0001"><w:r><w:fldChar w:fldCharType="begin"/></w:r><w:r><w:instrText xml:space="preserve"> PAGE </w:instrText></w:r><w:r><w:fldChar w:fldCharType="separate"/></w:r><w:r><w:t>1</w:t></w:r><w:r><w:fldChar w:fldCharType="end"/></w:r></w:p></w:hdr>"#;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-docx-fields");
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
            ("[Content_Types].xml", CT),
            ("_rels/.rels", ROOT_RELS),
            ("word/document.xml", DOCUMENT),
            ("word/_rels/document.xml.rels", DOC_RELS),
            ("word/styles.xml", STYLES),
            ("word/header1.xml", HEADER),
        ] {
            zip.start_file(name, opt).unwrap();
            zip.write_all(body.as_bytes()).unwrap();
        }
        zip.finish().unwrap();
    }
    std::fs::write(&path, cursor.into_inner()).unwrap();
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
    let mut file = zip.by_name(name).unwrap();
    let mut buf = Vec::new();
    file.read_to_end(&mut buf).unwrap();
    String::from_utf8(buf).unwrap()
}

fn listed(path: &Path) -> serde_json::Value {
    inspect(
        path,
        json!({
            "fields": true,
            "hyperlinks": true,
            "headerFooter": true,
            "from": 1,
            "maxParas": 20
        }),
    )
}

#[test]
fn inspect_counts_without_listing() {
    let path = write_sample("bare");
    let tree = inspect(&path, json!({}));
    assert_eq!(tree["fieldCount"], 6);
    assert!(tree.get("fields").is_none());
    assert_eq!(tree["headers"][0]["fieldCount"], 1);
    assert!(tree["headers"][0].get("fields").is_none());
}

#[test]
fn inspect_lists_fields_in_start_order() {
    let path = write_sample("list");
    let tree = listed(&path);
    assert_eq!(
        object_keys(&tree["fields"][0]),
        ["id", "index", "kind", "code", "instr", "host", "end", "text"]
    );
    let fields = tree["fields"].as_array().unwrap();
    assert_eq!(fields.len(), 6);
    assert_eq!(fields[0]["id"], "field:1");
    assert_eq!(fields[0]["kind"], "complex");
    assert_eq!(fields[0]["code"], "PAGE");
    assert_eq!(fields[0]["instr"], " PAGE ");
    assert_eq!(fields[0]["host"], "para:AAAA0001");
    assert_eq!(fields[0]["end"], "para:AAAA0001");
    assert_eq!(fields[0]["text"], "1");

    assert_eq!(fields[1]["kind"], "simple");
    assert_eq!(fields[1]["code"], "DATE");
    assert_eq!(fields[1]["host"], "para:AAAA0002");
    assert_eq!(fields[1]["text"], "2026");

    assert_eq!(fields[2]["code"], "HYPERLINK");
    assert_eq!(fields[2]["text"], "点这里");
    assert_eq!(tree["hyperlinks"].as_array().unwrap().len(), 0);

    assert_eq!(fields[3]["code"], "IF");
    assert_eq!(fields[3]["host"], "para:AAAA0005");
    assert!(!fields[3]["instr"].as_str().unwrap().contains("DATE"));
    assert_eq!(fields[3]["text"], "yes");

    assert_eq!(fields[4]["code"], "DATE");
    assert_eq!(fields[4]["kind"], "simple");
    assert_eq!(fields[4]["host"], "para:AAAA0005");
    assert_eq!(fields[4]["instr"], " DATE ");

    assert_eq!(fields[5]["code"], "=");
    assert_eq!(fields[5]["host"], "para:AAAA0007");

    let codes: Vec<&str> = fields.iter().map(|f| f["code"].as_str().unwrap()).collect();
    assert!(!codes.contains(&"AUTHOR"));

    let p4 = tree["paragraphs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == "para:AAAA0004")
        .unwrap();
    assert_eq!(p4["text"], "未闭合");

    let hf = &tree["headers"][0]["fields"][0];
    assert_eq!(hf["id"], "header:default:1/field:1");
    assert_eq!(hf["code"], "PAGE");
    assert_eq!(hf["host"], "header:default:1/para:BBBB0001");
}

#[test]
fn insert_field_appends_complex_dirty_field() {
    let path = write_sample("ins");
    let out = tmp_dir().join("ins-out.docx");
    apply(
        &path,
        &out,
        json!([{
            "op": "insertField",
            "id": "para:AAAA0006",
            "instr": " DATE ",
            "result": "x"
        }]),
    );
    let tree = listed(&out);
    assert_eq!(tree["fieldCount"], 7);
    let new = tree["fields"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["host"] == "para:AAAA0006")
        .unwrap();
    assert_eq!(new["kind"], "complex");
    assert_eq!(new["code"], "DATE");
    assert_eq!(new["text"], "x");
    let xml = part_text(&out, "word/document.xml");
    assert!(xml.contains(r#"w:fldCharType="begin""#));
    assert!(xml.contains(r#"w:dirty="true""#));
    assert!(xml.contains("xml:space=\"preserve\""));
}

#[test]
fn insert_field_on_header_para() {
    let path = write_sample("hf-ins");
    let out = tmp_dir().join("hf-ins-out.docx");
    apply(
        &path,
        &out,
        json!([{
            "op": "insertField",
            "id": "header:default:1/para:BBBB0001",
            "instr": " DATE "
        }]),
    );
    let tree = listed(&out);
    assert_eq!(tree["headers"][0]["fieldCount"], 2);
    let dates: Vec<_> = tree["headers"][0]["fields"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["code"] == "DATE")
        .collect();
    assert_eq!(dates.len(), 1);
    assert_eq!(dates[0]["text"], "");
}

#[test]
fn set_field_rewrites_instr_only() {
    let path = write_sample("set");
    let out = tmp_dir().join("set-out.docx");
    apply(
        &path,
        &out,
        json!([{ "op": "setField", "id": "field:1", "instr": " DATE " }]),
    );
    let tree = listed(&out);
    assert_eq!(tree["fields"][0]["code"], "DATE");
    assert_eq!(tree["fields"][0]["instr"], " DATE ");
    assert_eq!(tree["fields"][0]["text"], "1");
    apply(
        &out,
        &tmp_dir().join("set-simple.docx"),
        json!([{ "op": "setField", "id": "field:2", "instr": " TIME " }]),
    );
    let simple = listed(&tmp_dir().join("set-simple.docx"));
    assert_eq!(simple["fields"][1]["code"], "TIME");
    assert_eq!(simple["fields"][1]["text"], "2026");
}

#[test]
fn delete_field_keep_result_leaves_cache() {
    let path = write_sample("del-res");
    let out = tmp_dir().join("del-res-out.docx");
    apply(
        &path,
        &out,
        json!([{ "op": "deleteField", "id": "field:2" }]),
    );
    let tree = listed(&out);
    assert_eq!(tree["fieldCount"], 5);
    assert!(!tree["fields"]
        .as_array()
        .unwrap()
        .iter()
        .any(|f| f["host"] == "para:AAAA0002"));
    let p2 = tree["paragraphs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == "para:AAAA0002")
        .unwrap();
    assert_eq!(p2["text"], "2026");
    let xml = part_text(&out, "word/document.xml");
    assert_eq!(xml.matches("<w:fldSimple").count(), 1);
}

#[test]
fn delete_field_keep_instr_leaves_code() {
    let path = write_sample("del-instr");
    let out = tmp_dir().join("del-instr-out.docx");
    apply(
        &path,
        &out,
        json!([{ "op": "deleteField", "id": "field:1", "keep": "instr" }]),
    );
    let tree = listed(&out);
    assert!(!tree["fields"]
        .as_array()
        .unwrap()
        .iter()
        .any(|f| f["code"] == "PAGE" && f["host"] == "para:AAAA0001"));
    let p1 = tree["paragraphs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == "para:AAAA0001")
        .unwrap();
    assert!(p1["text"].as_str().unwrap().contains("PAGE"));
    assert_ne!(p1["text"], "1");
}

#[test]
fn delete_nested_keep_result_drops_instruction_inner() {
    let path = write_sample("del-nest");
    let out = tmp_dir().join("del-nest-out.docx");
    apply(
        &path,
        &out,
        json!([{ "op": "deleteField", "id": "field:4", "keep": "result" }]),
    );
    let tree = listed(&out);
    let in_p5: Vec<_> = tree["fields"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["host"] == "para:AAAA0005")
        .collect();
    assert!(in_p5.is_empty());
    let p5 = tree["paragraphs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == "para:AAAA0005")
        .unwrap();
    assert_eq!(p5["text"], "yes");
}

#[test]
fn set_para_text_refuses_field_para() {
    let path = write_sample("set-text");
    let err = apply_err(
        &path,
        json!([{ "op": "setParaText", "id": "para:AAAA0001", "text": "x" }]),
    );
    assert_eq!(err.code(), ErrorCode::NotImplemented);

    let err_hf = apply_err(
        &path,
        json!([{
            "op": "setParaText",
            "id": "header:default:1/para:BBBB0001",
            "text": "x"
        }]),
    );
    assert_eq!(err_hf.code(), ErrorCode::NotImplemented);

    let out = tmp_dir().join("set-clean-out.docx");
    apply(
        &path,
        &out,
        json!([{ "op": "setParaText", "id": "para:AAAA0006", "text": "改干净" }]),
    );
    let tree = listed(&out);
    let p6 = tree["paragraphs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == "para:AAAA0006")
        .unwrap();
    assert_eq!(p6["text"], "改干净");
}

#[test]
fn insert_field_rejects_empty_instr_and_non_para() {
    let path = write_sample("bad");
    let err = apply_err(
        &path,
        json!([{ "op": "insertField", "id": "para:AAAA0006", "instr": "   " }]),
    );
    assert_eq!(err.code(), ErrorCode::InvalidField);
    let err_id = apply_err(
        &path,
        json!([{ "op": "insertField", "id": "field:1", "instr": " PAGE " }]),
    );
    assert_eq!(err_id.code(), ErrorCode::InvalidId);
    let err_keep = apply_err(
        &path,
        json!([{ "op": "deleteField", "id": "field:1", "keep": "both" }]),
    );
    assert_eq!(err_keep.code(), ErrorCode::InvalidField);
    let err_miss = apply_err(
        &path,
        json!([{ "op": "setField", "id": "field:99", "instr": " DATE " }]),
    );
    assert_eq!(err_miss.code(), ErrorCode::IdNotFound);
}
