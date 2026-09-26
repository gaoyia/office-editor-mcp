//! docx SmartArt：计数、名单、改字、增删点、插删、配色与版式。

use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

const ROOT_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#;

const STYLES: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/></w:style></w:styles>"#;

const DOC_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/diagramData" Target="diagrams/data1.xml"/><Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/diagramLayout" Target="diagrams/layout1.xml"/><Relationship Id="rId4" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/diagramQuickStyle" Target="diagrams/quickStyle1.xml"/><Relationship Id="rId5" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/diagramColors" Target="diagrams/colors1.xml"/></Relationships>"#;

const CT: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/><Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/><Override PartName="/word/diagrams/data1.xml" ContentType="application/vnd.openxmlformats-officedocument.drawingml.diagramData+xml"/><Override PartName="/word/diagrams/layout1.xml" ContentType="application/vnd.openxmlformats-officedocument.drawingml.diagramLayout+xml"/><Override PartName="/word/diagrams/colors1.xml" ContentType="application/vnd.openxmlformats-officedocument.drawingml.diagramColors+xml"/><Override PartName="/word/diagrams/quickStyle1.xml" ContentType="application/vnd.openxmlformats-officedocument.drawingml.diagramQuickStyle+xml"/></Types>"#;

const UID_LIST: &str = "urn:microsoft.com/office/officeart/2005/8/layout/default";
const UID_CYCLE: &str = "urn:microsoft.com/office/officeart/2005/8/layout/cycle2";
const UID_COLOR: &str = "urn:microsoft.com/office/officeart/2005/8/colors/accent1_2";
const UID_STYLE: &str = "urn:microsoft.com/office/officeart/2005/8/quickstyle/simple1";

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-docx-smartart");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_zip(tag: &str, files: &[(&str, &[u8])]) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let path = tmp_dir().join(format!("{tag}-{n}.docx"));
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut zip = ZipWriter::new(&mut cursor);
        let opt = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        for (name, body) in files {
            zip.start_file(*name, opt).unwrap();
            zip.write_all(body).unwrap();
        }
        zip.finish().unwrap();
    }
    std::fs::write(&path, cursor.into_inner()).unwrap();
    path
}

fn document_xml() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:dgm="http://schemas.openxmlformats.org/drawingml/2006/diagram" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006"><w:body><w:p w14:paraId="AAAA0001"><w:r><w:t>带图示</w:t></w:r><w:r><mc:AlternateContent><mc:Choice Requires="wps"><w:drawing><wp:inline distT="0" distB="0" distL="0" distR="0"><wp:extent cx="5486400" cy="3200400"/><wp:docPr id="4" name="SmartArt 1"/><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/diagram"><dgm:relIds r:dm="rId2" r:lo="rId3" r:qs="rId4" r:cs="rId5"/></a:graphicData></a:graphic></wp:inline></w:drawing></mc:Choice><mc:Fallback><w:drawing><wp:inline><wp:extent cx="1" cy="1"/><wp:docPr id="99" name="fallback"/><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/picture"/></a:graphic></wp:inline></w:drawing></mc:Fallback></mc:AlternateContent></w:r></w:p><w:sectPr><w:pgSz w:w="11906" w:h="16838"/></w:sectPr></w:body></w:document>"#.into()
}

fn data_xml() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><dgm:dataModel xmlns:dgm="http://schemas.openxmlformats.org/drawingml/2006/diagram" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><dgm:ptLst><dgm:pt type="doc" modelId="0"/><dgm:pt modelId="1"><dgm:t><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>甲</a:t></a:r></a:p></dgm:t></dgm:pt><dgm:pt modelId="2"><dgm:t><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>乙</a:t></a:r></a:p></dgm:t></dgm:pt><dgm:pt modelId="3"><dgm:t><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>丙</a:t></a:r></a:p></dgm:t></dgm:pt></dgm:ptLst><dgm:cxnLst><dgm:cxn modelId="4" srcId="0" destId="1" srcOrd="0" destOrd="0"/><dgm:cxn modelId="5" srcId="0" destId="2" srcOrd="1" destOrd="0"/><dgm:cxn modelId="6" srcId="0" destId="3" srcOrd="2" destOrd="0"/></dgm:cxnLst><dgm:bg/><dgm:whole/></dgm:dataModel>"#.into()
}

fn layout_xml(uid: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><dgm:layoutDef xmlns:dgm="http://schemas.openxmlformats.org/drawingml/2006/diagram" uniqueId="{uid}" minVer="12.0"><dgm:title lang="" val="x"/><dgm:desc lang="" val=""/><dgm:layoutNode name="n0"><dgm:alg type="snake"/><dgm:shape/></dgm:layoutNode></dgm:layoutDef>"#
    )
}

fn colors_xml() -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><dgm:colorsDef xmlns:dgm="http://schemas.openxmlformats.org/drawingml/2006/diagram" uniqueId="{UID_COLOR}" minVer="12.0"><dgm:title lang="" val=""/><dgm:desc lang="" val=""/></dgm:colorsDef>"#
    )
}

fn style_xml() -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><dgm:styleDef xmlns:dgm="http://schemas.openxmlformats.org/drawingml/2006/diagram" uniqueId="{UID_STYLE}" minVer="12.0"><dgm:title lang="" val=""/><dgm:desc lang="" val=""/></dgm:styleDef>"#
    )
}

fn write_sample(tag: &str, cycle: bool) -> PathBuf {
    let uid = if cycle { UID_CYCLE } else { UID_LIST };
    let doc = document_xml();
    let data = data_xml();
    let layout = layout_xml(uid);
    let colors = colors_xml();
    let style = style_xml();
    write_zip(
        tag,
        &[
            ("[Content_Types].xml", CT.as_bytes()),
            ("_rels/.rels", ROOT_RELS.as_bytes()),
            ("word/document.xml", doc.as_bytes()),
            ("word/_rels/document.xml.rels", DOC_RELS.as_bytes()),
            ("word/styles.xml", STYLES.as_bytes()),
            ("word/diagrams/data1.xml", data.as_bytes()),
            ("word/diagrams/layout1.xml", layout.as_bytes()),
            ("word/diagrams/colors1.xml", colors.as_bytes()),
            ("word/diagrams/quickStyle1.xml", style.as_bytes()),
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
        panic!("expected inspect");
    };
    tree
}

fn apply_ops(path: &Path, out: &Path, ops: Value) {
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

fn object_keys(v: &Value) -> Vec<&str> {
    v.as_object().unwrap().keys().map(String::as_str).collect()
}

fn zip_names(path: &Path) -> Vec<String> {
    let bytes = std::fs::read(path).unwrap();
    let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut names = Vec::new();
    for i in 0..zip.len() {
        names.push(zip.by_index(i).unwrap().name().replace('\\', "/"));
    }
    names
}

fn part_text(path: &Path, name: &str) -> String {
    let bytes = std::fs::read(path).unwrap();
    let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut file = zip.by_name(name.trim_start_matches('/')).unwrap();
    let mut buf = Vec::new();
    file.read_to_end(&mut buf).unwrap();
    String::from_utf8(buf).unwrap()
}

#[test]
fn inspect_count_without_list() {
    let path = write_sample("count", false);
    let tree = inspect(&path, json!({}));
    assert_eq!(tree["smartArtCount"], 1);
    assert!(tree.get("smartArts").is_none());
    assert_eq!(tree["chartCount"], 0);
}

#[test]
fn inspect_lists_points_and_parent() {
    let path = write_sample("list", false);
    let tree = inspect(&path, json!({ "smartArt": true }));
    assert_eq!(tree["smartArtCount"], 1);
    assert_eq!(
        object_keys(&tree)[object_keys(&tree).len() - 1],
        "smartArts"
    );
    let d = &tree["smartArts"][0];
    assert_eq!(d["id"], "para:AAAA0001/smartdiagram:4");
    assert_eq!(d["docPrId"], "4");
    assert_eq!(d["name"], "SmartArt 1");
    assert_eq!(d["kind"], "inline");
    assert_eq!(d["cx"], 5486400);
    assert_eq!(d["cy"], 3200400);
    assert!(d["wrap"].is_null());
    assert_eq!(d["layout"], UID_LIST);
    assert_eq!(d["colors"], UID_COLOR);
    assert_eq!(d["style"], UID_STYLE);
    assert_eq!(d["stable"], true);
    assert_eq!(
        object_keys(d),
        [
            "id", "docPrId", "name", "kind", "cx", "cy", "wrap", "layout", "colors", "style",
            "points", "stable"
        ]
    );
    let pts = d["points"].as_array().unwrap();
    assert_eq!(pts.len(), 3);
    assert_eq!(pts[0]["id"], "para:AAAA0001/smartdiagram:4/pt:1");
    assert_eq!(pts[0]["modelId"], "1");
    assert_eq!(pts[0]["role"], "node");
    assert_eq!(pts[0]["text"], "甲");
    assert!(pts[0]["parent"].is_null());
    assert_eq!(pts[1]["text"], "乙");
    assert_eq!(
        object_keys(&pts[0]),
        ["id", "modelId", "role", "text", "parent"]
    );
}

#[test]
fn set_text_roundtrip() {
    let path = write_sample("text", false);
    let out = tmp_dir().join("text-out.docx");
    apply_ops(
        &path,
        &out,
        json!([{
            "op": "setSmartArtText",
            "id": "para:AAAA0001/smartdiagram:4/pt:2",
            "text": "新条目"
        }]),
    );
    let tree = inspect(&out, json!({ "smartArt": true }));
    assert_eq!(tree["smartArts"][0]["points"][1]["text"], "新条目");
    assert_eq!(tree["smartArts"][0]["points"][0]["text"], "甲");
}

#[test]
fn insert_list_and_delete() {
    let path = write_sample("ins", false);
    let tree0 = inspect(&path, json!({ "from": 1, "maxParas": 5 }));
    let para = tree0["paragraphs"][0]["id"].as_str().unwrap().to_string();
    let ins = tmp_dir().join("ins-out.docx");
    apply_ops(
        &path,
        &ins,
        json!([{
            "op": "insertSmartArt",
            "id": para,
            "layout": "list",
            "points": [{ "text": "一" }, { "text": "二" }]
        }]),
    );
    let tree = inspect(&ins, json!({ "smartArt": true }));
    assert_eq!(tree["smartArtCount"], 2);
    let added = tree["smartArts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["id"] != "para:AAAA0001/smartdiagram:4")
        .unwrap();
    assert_eq!(added["points"].as_array().unwrap().len(), 2);
    assert_eq!(added["points"][0]["text"], "一");
    assert_eq!(added["kind"], "anchor");
    assert_eq!(added["wrap"], "square");
    assert_eq!(added["layout"], UID_LIST);

    let del = tmp_dir().join("del-out.docx");
    apply_ops(
        &ins,
        &del,
        json!([{ "op": "deleteSmartArt", "id": "para:AAAA0001/smartdiagram:4" }]),
    );
    let tree = inspect(&del, json!({ "smartArt": true }));
    assert_eq!(tree["smartArtCount"], 1);
    let names = zip_names(&del);
    assert!(!names.iter().any(|n| n.contains("data1.xml")));
    let doc = part_text(&del, "word/document.xml");
    assert!(!doc.contains("Fallback"));
    assert!(!doc.contains(r#"name="SmartArt 1""#));
}

#[test]
fn add_and_delete_point() {
    let path = write_sample("pts", false);
    let add = tmp_dir().join("add-pt.docx");
    apply_ops(
        &path,
        &add,
        json!([{
            "op": "addSmartArtPoint",
            "id": "para:AAAA0001/smartdiagram:4",
            "text": "丁"
        }]),
    );
    let tree = inspect(&add, json!({ "smartArt": true }));
    let pts = tree["smartArts"][0]["points"].as_array().unwrap();
    assert_eq!(pts.len(), 4);
    assert_eq!(pts[3]["text"], "丁");
    let last_id = pts[3]["id"].as_str().unwrap().to_string();

    let del = tmp_dir().join("del-pt.docx");
    apply_ops(
        &add,
        &del,
        json!([{ "op": "deleteSmartArtPoint", "id": last_id }]),
    );
    let tree = inspect(&del, json!({ "smartArt": true }));
    assert_eq!(tree["smartArts"][0]["points"].as_array().unwrap().len(), 3);

    let err = apply_err(
        &del,
        json!([
            { "op": "deleteSmartArtPoint", "id": "para:AAAA0001/smartdiagram:4/pt:1" },
            { "op": "deleteSmartArtPoint", "id": "para:AAAA0001/smartdiagram:4/pt:2" },
            { "op": "deleteSmartArtPoint", "id": "para:AAAA0001/smartdiagram:4/pt:3" }
        ]),
    );
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

#[test]
fn list_rejects_asst_and_nested() {
    let path = write_sample("list-rules", false);
    let err = apply_err(
        &path,
        json!([{
            "op": "addSmartArtPoint",
            "id": "para:AAAA0001/smartdiagram:4/pt:1",
            "text": "子"
        }]),
    );
    assert_eq!(err.code(), ErrorCode::InvalidField);
    let err = apply_err(
        &path,
        json!([{
            "op": "addSmartArtPoint",
            "id": "para:AAAA0001/smartdiagram:4",
            "role": "asst"
        }]),
    );
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

#[test]
fn set_layout_same_family_and_reject_cycle() {
    let path = write_sample("lay", false);
    let out = tmp_dir().join("lay-out.docx");
    apply_ops(
        &path,
        &out,
        json!([{
            "op": "setSmartArtLayout",
            "id": "para:AAAA0001/smartdiagram:4",
            "layout": "list"
        }]),
    );
    let tree = inspect(&out, json!({ "smartArt": true }));
    assert_eq!(tree["smartArts"][0]["layout"], UID_LIST);

    let cycle = write_sample("cycle", true);
    let err = apply_err(
        &cycle,
        json!([{
            "op": "setSmartArtLayout",
            "id": "para:AAAA0001/smartdiagram:4",
            "layout": "list"
        }]),
    );
    assert_eq!(err.code(), ErrorCode::NotImplemented);
}

#[test]
fn set_colors_style_size() {
    let path = write_sample("skin", false);
    let out = tmp_dir().join("skin-out.docx");
    apply_ops(
        &path,
        &out,
        json!([
            { "op": "setSmartArtColors", "id": "para:AAAA0001/smartdiagram:4", "colors": "accent3" },
            { "op": "setSmartArtStyle", "id": "para:AAAA0001/smartdiagram:4", "style": "intense" },
            { "op": "setSmartArtSize", "id": "para:AAAA0001/smartdiagram:4", "cx": 2000000, "cy": 1000000 }
        ]),
    );
    let tree = inspect(&out, json!({ "smartArt": true }));
    assert!(tree["smartArts"][0]["colors"]
        .as_str()
        .unwrap()
        .contains("accent3"));
    assert!(tree["smartArts"][0]["style"]
        .as_str()
        .unwrap()
        .contains("intense"));
    assert_eq!(tree["smartArts"][0]["cx"], 2000000);
    assert_eq!(tree["smartArts"][0]["cy"], 1000000);
}

#[test]
fn create_with_smartart() {
    let path = tmp_dir().join("create.docx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "format": "docx",
            "document": {
                "paragraphs": [
                    { "text": "", "smartArts": [{ "layout": "org" }] }
                ]
            }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let tree = inspect(&path, json!({ "smartArt": true }));
    assert_eq!(tree["smartArtCount"], 1);
    let pts = tree["smartArts"][0]["points"].as_array().unwrap();
    assert_eq!(pts.len(), 3);
    assert_eq!(pts[0]["text"], "文本 1");
    assert!(pts[0]["parent"].is_null());
    assert_eq!(pts[1]["parent"], "1");
}

#[test]
fn xlsx_rejects_smartart_scope_and_op() {
    let path = tmp_dir().join("book.xlsx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "workbook": { "sheets": [{ "name": "Sheet1", "cells": [] }] }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": path.to_string_lossy(),
            "scope": { "smartArt": true }
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": path.to_string_lossy(),
            "output": tmp_dir().join("x.xlsx").to_string_lossy(),
            "ops": [{ "op": "insertSmartArt", "id": "Sheet1!A1", "layout": "list" }]
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

#[test]
fn header_smartart_lists_and_set_text() {
    let path = tmp_dir().join("hf.docx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "format": "docx",
            "document": {
                "paragraphs": [{ "text": "正文" }],
                "headers": [{
                    "type": "default",
                    "paragraphs": [{
                        "text": "",
                        "smartArts": [{
                            "layout": "list",
                            "points": [{ "text": "眉甲" }, { "text": "眉乙" }]
                        }]
                    }]
                }]
            }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let tree = inspect(&path, json!({ "headerFooter": true, "smartArt": true }));
    assert_eq!(tree["smartArtCount"], 0);
    assert!(tree.get("smartArts").is_some());
    assert_eq!(tree["smartArts"].as_array().unwrap().len(), 0);
    let hf = &tree["headers"][0];
    assert_eq!(hf["smartArtCount"], 1);
    let d = &hf["smartArts"][0];
    let id = d["id"].as_str().unwrap();
    assert!(id.starts_with("header:default:1/para:"));
    assert!(id.contains("/smartdiagram:"));
    assert_eq!(d["points"][0]["text"], "眉甲");
    let pt = d["points"][1]["id"].as_str().unwrap().to_string();

    let out = tmp_dir().join("hf-text.docx");
    apply_ops(
        &path,
        &out,
        json!([{ "op": "setSmartArtText", "id": pt, "text": "眉新" }]),
    );
    let tree = inspect(&out, json!({ "headerFooter": true, "smartArt": true }));
    assert_eq!(
        tree["headers"][0]["smartArts"][0]["points"][1]["text"],
        "眉新"
    );
}

#[test]
fn insert_rejects_diagram_id() {
    let path = write_sample("bad-id", false);
    let err = apply_err(
        &path,
        json!([{
            "op": "insertSmartArt",
            "id": "para:AAAA0001/smartdiagram:4",
            "layout": "list"
        }]),
    );
    assert_eq!(err.code(), ErrorCode::InvalidId);
}

#[test]
fn delete_drops_orphan_drawing_cache() {
    let doc = document_xml();
    let data = data_xml();
    let layout = layout_xml(UID_LIST);
    let colors = colors_xml();
    let style = style_xml();
    let drawing = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><dsp:drawing xmlns:dsp="http://schemas.microsoft.com/office/drawing/2008/diagram"/>"#;
    let ct = CT.replacen(
        "</Types>",
        r#"<Override PartName="/word/diagrams/drawing1.xml" ContentType="application/vnd.openxmlformats-officedocument.drawingml.diagramDrawing+xml"/></Types>"#,
        1,
    );
    let path = write_zip(
        "orphan-draw",
        &[
            ("[Content_Types].xml", ct.as_bytes()),
            ("_rels/.rels", ROOT_RELS.as_bytes()),
            ("word/document.xml", doc.as_bytes()),
            ("word/_rels/document.xml.rels", DOC_RELS.as_bytes()),
            ("word/styles.xml", STYLES.as_bytes()),
            ("word/diagrams/data1.xml", data.as_bytes()),
            ("word/diagrams/layout1.xml", layout.as_bytes()),
            ("word/diagrams/colors1.xml", colors.as_bytes()),
            ("word/diagrams/quickStyle1.xml", style.as_bytes()),
            ("word/diagrams/drawing1.xml", drawing.as_bytes()),
        ],
    );
    let out = tmp_dir().join("orphan-del.docx");
    apply_ops(
        &path,
        &out,
        json!([{ "op": "deleteSmartArt", "id": "para:AAAA0001/smartdiagram:4" }]),
    );
    let names = zip_names(&out);
    assert!(!names.iter().any(|n| n.contains("drawing1.xml")));
    assert!(!names.iter().any(|n| n.contains("data1.xml")));
}

#[test]
fn missing_data_still_lists() {
    let doc = document_xml();
    let path = write_zip(
        "nodata",
        &[
            ("[Content_Types].xml", CT.as_bytes()),
            ("_rels/.rels", ROOT_RELS.as_bytes()),
            ("word/document.xml", doc.as_bytes()),
            ("word/_rels/document.xml.rels", DOC_RELS.as_bytes()),
            ("word/styles.xml", STYLES.as_bytes()),
        ],
    );
    let tree = inspect(&path, json!({ "smartArt": true }));
    assert_eq!(tree["smartArtCount"], 1);
    assert_eq!(tree["smartArts"][0]["points"], json!([]));
    assert!(tree["smartArts"][0]["layout"].is_null());
}
