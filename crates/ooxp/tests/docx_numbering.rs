//! docx 编号列表：名单、段上 numPr、插改绑、xlsx/pptx 拒绝。

use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

const CT: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/><Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/><Override PartName="/word/numbering.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.numbering+xml"/></Types>"#;

const CT_PLAIN: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/><Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/></Types>"#;

const ROOT_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#;

const DOC_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/numbering" Target="numbering.xml"/></Relationships>"#;

const DOC_RELS_PLAIN: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/></Relationships>"#;

const STYLES: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/></w:style><w:style w:type="paragraph" w:styleId="ListPara"><w:name w:val="List Paragraph"/><w:pPr><w:numPr><w:ilvl w:val="0"/><w:numId w:val="1"/></w:numPr></w:pPr></w:style></w:styles>"#;

const NUMBERING: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:numbering xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:abstractNum w:abstractNumId="0"><w:nsid w:val="AABBCCDD"/><w:multiLevelType w:val="hybridMultilevel"/><w:lvl w:ilvl="0"><w:start w:val="1"/><w:numFmt w:val="decimal"/><w:lvlText w:val="%1."/><w:lvlJc w:val="left"/></w:lvl><w:lvl w:ilvl="1"><w:start w:val="1"/><w:numFmt w:val="lowerLetter"/><w:lvlText w:val="%1.%2."/><w:lvlJc w:val="left"/></w:lvl></w:abstractNum><w:num w:numId="0"><w:abstractNumId w:val="0"/></w:num><w:num w:numId="1"><w:abstractNumId w:val="0"/></w:num></w:numbering>"#;

const DOCUMENT: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml"><w:body><w:p w14:paraId="AAAA0001"><w:pPr><w:numPr><w:ilvl w:val="0"/><w:numId w:val="1"/></w:numPr></w:pPr><w:r><w:t>甲</w:t></w:r></w:p><w:p w14:paraId="AAAA0002"><w:pPr><w:pStyle w:val="ListPara"/></w:pPr><w:r><w:t>乙</w:t></w:r></w:p><w:p w14:paraId="AAAA0003"><w:r><w:t>丙</w:t></w:r></w:p><w:sectPr><w:pgSz w:w="11906" w:h="16838"/></w:sectPr></w:body></w:document>"#;

const DOCUMENT_PLAIN: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml"><w:body><w:p w14:paraId="AAAA0001"><w:r><w:t>一</w:t></w:r></w:p><w:p w14:paraId="AAAA0002"><w:r><w:t>二</w:t></w:r></w:p><w:sectPr><w:pgSz w:w="11906" w:h="16838"/></w:sectPr></w:body></w:document>"#;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-docx-numbering");
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
            ("word/numbering.xml", NUMBERING),
        ],
    )
}

fn write_plain(tag: &str) -> PathBuf {
    write_zip(
        tag,
        &[
            ("[Content_Types].xml", CT_PLAIN),
            ("_rels/.rels", ROOT_RELS),
            ("word/document.xml", DOCUMENT_PLAIN),
            ("word/_rels/document.xml.rels", DOC_RELS_PLAIN),
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
    inspect(
        path,
        json!({ "numbering": true, "from": 1, "maxParas": 20 }),
    )
}

#[test]
fn numbering_count_without_scope() {
    let path = write_sample("count");
    let tree = inspect(&path, json!({}));
    assert_eq!(tree["numberingCount"], 1);
    assert!(tree.get("numberings").is_none());
    assert!(!object_keys(&tree).contains(&"numberings"));
}

#[test]
fn numbering_list_and_para_fields() {
    let path = write_sample("list");
    let tree = listed(&path);
    assert_eq!(tree["numberingCount"], 1);
    let nums = tree["numberings"].as_array().unwrap();
    assert_eq!(nums.len(), 1);
    assert_eq!(
        object_keys(&nums[0]),
        [
            "id",
            "numId",
            "abstractId",
            "abstractNumId",
            "multiLevelType",
            "levels"
        ]
    );
    assert_eq!(nums[0]["id"], "num:1");
    assert_eq!(nums[0]["numId"], 1);
    assert_eq!(nums[0]["abstractId"], "abstractNum:0");
    assert_eq!(nums[0]["abstractNumId"], 0);
    assert_eq!(nums[0]["multiLevelType"], "hybridMultilevel");
    let lvls = nums[0]["levels"].as_array().unwrap();
    assert_eq!(lvls.len(), 2);
    assert_eq!(
        object_keys(&lvls[0]),
        ["ilvl", "numFmt", "lvlText", "start", "lvlJc"]
    );
    assert_eq!(lvls[0]["ilvl"], 0);
    assert_eq!(lvls[0]["numFmt"], "decimal");
    assert_eq!(lvls[0]["lvlText"], "%1.");
    assert_eq!(lvls[0]["start"], 1);
    assert_eq!(lvls[1]["numFmt"], "lowerLetter");

    let paras = tree["paragraphs"].as_array().unwrap();
    let p1 = paras.iter().find(|p| p["id"] == "para:AAAA0001").unwrap();
    assert_eq!(
        object_keys(p1),
        [
            "id",
            "index",
            "style",
            "num",
            "ilvl",
            "align",
            "indent",
            "spacing",
            "eastAsia",
            "text",
            "stable",
            "pictureCount",
            "textBoxCount",
            "connectorCount",
            "shapeCount",
            "inkCanvasCount",
            "model3dCount",
            "runCount"
        ]
    );
    assert_eq!(p1["num"], "num:1");
    assert_eq!(p1["ilvl"], 0);
    let p2 = paras.iter().find(|p| p["id"] == "para:AAAA0002").unwrap();
    assert!(p2.get("num").is_none());
    let p3 = paras.iter().find(|p| p["id"] == "para:AAAA0003").unwrap();
    assert!(p3.get("num").is_none());
}

#[test]
fn insert_numbering_creates_part() {
    let path = write_plain("insert");
    let out = tmp_dir().join("insert-out.docx");
    apply(
        &path,
        &out,
        json!([{
            "op": "insertNumbering",
            "id": "docx:document",
            "kind": "decimal"
        }]),
    );
    let tree = listed(&out);
    assert_eq!(tree["numberingCount"], 1);
    let n = &tree["numberings"][0];
    assert_eq!(n["id"], "num:1");
    assert_eq!(n["abstractId"], "abstractNum:1");
    assert_eq!(n["levels"].as_array().unwrap().len(), 9);
    assert_eq!(n["levels"][0]["numFmt"], "decimal");
    assert_eq!(n["levels"][0]["lvlText"], "%1.");
}

#[test]
fn set_numbering_changes_level() {
    let path = write_sample("set");
    let out = tmp_dir().join("set-out.docx");
    apply(
        &path,
        &out,
        json!([{
            "op": "setNumbering",
            "id": "abstractNum:0",
            "ilvl": 0,
            "numFmt": "bullet",
            "lvlText": "o",
            "start": 1
        }]),
    );
    let tree = listed(&out);
    let lvl0 = &tree["numberings"][0]["levels"][0];
    assert_eq!(lvl0["numFmt"], "bullet");
    assert_eq!(lvl0["lvlText"], "o");
}

#[test]
fn bind_and_unbind_para() {
    let path = write_sample("bind");
    let out = tmp_dir().join("bind-out.docx");
    apply(
        &path,
        &out,
        json!([{
            "op": "setParaNumbering",
            "id": "para:AAAA0003",
            "num": "num:1",
            "ilvl": 1
        }]),
    );
    let tree = listed(&out);
    let p3 = tree["paragraphs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == "para:AAAA0003")
        .unwrap();
    assert_eq!(p3["num"], "num:1");
    assert_eq!(p3["ilvl"], 1);

    let out2 = tmp_dir().join("unbind-out.docx");
    apply(
        &out,
        &out2,
        json!([{
            "op": "setParaNumbering",
            "id": "para:AAAA0003",
            "num": null
        }]),
    );
    let tree2 = listed(&out2);
    let p3b = tree2["paragraphs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == "para:AAAA0003")
        .unwrap();
    assert!(p3b.get("num").is_none());
}

#[test]
fn numbering_ops_reject_bad_ids() {
    let path = write_sample("bad");
    let err = apply_err(
        &path,
        json!([{ "op": "insertNumbering", "id": "para:AAAA0001", "kind": "decimal" }]),
    );
    assert_eq!(err.code(), ErrorCode::InvalidId);

    let err_kind = apply_err(
        &path,
        json!([{ "op": "insertNumbering", "id": "docx:document", "kind": "roman" }]),
    );
    assert_eq!(err_kind.code(), ErrorCode::InvalidField);

    let err_abs = apply_err(
        &path,
        json!([{
            "op": "setNumbering",
            "id": "abstractNum:99",
            "ilvl": 0,
            "numFmt": "decimal"
        }]),
    );
    assert_eq!(err_abs.code(), ErrorCode::IdNotFound);

    let err_ilvl = apply_err(
        &path,
        json!([{
            "op": "setNumbering",
            "id": "abstractNum:0",
            "ilvl": 5,
            "numFmt": "decimal"
        }]),
    );
    assert_eq!(err_ilvl.code(), ErrorCode::InvalidField);

    let err_num = apply_err(
        &path,
        json!([{ "op": "setParaNumbering", "id": "para:AAAA0003", "num": "num:9" }]),
    );
    assert_eq!(err_num.code(), ErrorCode::IdNotFound);

    let err_need = apply_err(
        &path,
        json!([{ "op": "setParaNumbering", "id": "para:AAAA0003" }]),
    );
    assert_eq!(err_need.code(), ErrorCode::InvalidField);
}

#[test]
fn numbering_not_for_xlsx_or_pptx() {
    let xlsx = tmp_dir().join("nl.xlsx");
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
            "scope": { "numbering": true }
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    let out = tmp_dir().join("nl-x-out.xlsx");
    let err_op = run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": xlsx.to_string_lossy(),
            "output": out.to_string_lossy(),
            "ops": [{ "op": "insertNumbering", "id": "docx:document", "kind": "decimal" }]
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err_op.code(), ErrorCode::InvalidField);

    let pptx = tmp_dir().join("nl.pptx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": pptx.to_string_lossy(),
            "presentation": { "slides": [{ "shapes": [{ "text": "一页" }] }] }
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
            "scope": { "numbering": true }
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err_p.code(), ErrorCode::InvalidField);
}
