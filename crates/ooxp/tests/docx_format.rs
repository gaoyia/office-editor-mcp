//! docx 段落与 run 直接格式：inspect、apply、xlsx/pptx 拒绝。

use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

const CT: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/><Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/></Types>"#;

const ROOT_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#;

const DOC_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/></Relationships>"#;

const STYLES: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:docDefaults><w:rPrDefault><w:rPr><w:sz w:val="22"/></w:rPr></w:rPrDefault></w:docDefaults><w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/><w:qFormat/></w:style></w:styles>"#;

const DOCUMENT: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml"><w:body><w:p w14:paraId="AAAA0001"><w:r><w:t>甲</w:t></w:r><w:r><w:rPr><w:color w:val="FF0000"/></w:rPr><w:t>乙</w:t></w:r></w:p><w:p w14:paraId="AAAA0002"><w:pPr><w:jc w:val="left"/><w:spacing w:before="100" w:after="80"/></w:pPr><w:r><w:t>第二段</w:t></w:r></w:p><w:sectPr><w:pgSz w:w="11906" w:h="16838"/></w:sectPr></w:body></w:document>"#;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-docx-format");
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

fn unzip(path: &Path, name: &str) -> String {
    let file = std::fs::File::open(path).unwrap();
    let mut zip = zip::ZipArchive::new(file).unwrap();
    let mut part = zip.by_name(name).unwrap();
    let mut s = String::new();
    part.read_to_string(&mut s).unwrap();
    s
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
fn run_count_without_scope_runs() {
    let path = write_sample("count");
    let tree = inspect(&path, json!({ "from": 1, "maxParas": 20 }));
    let p1 = para(&tree, "para:AAAA0001");
    assert_eq!(p1["runCount"], 2);
    assert!(p1.get("runs").is_none());
    assert_eq!(
        object_keys(p1),
        [
            "id",
            "index",
            "style",
            "align",
            "indent",
            "spacing",
            "eastAsia",
            "borders",
            "shd",
            "tabs",
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
    assert!(p1["align"].is_null());
    assert!(p1["indent"].is_null());
    assert!(p1["spacing"].is_null());
    let p2 = para(&tree, "para:AAAA0002");
    assert_eq!(p2["align"], "left");
    assert_eq!(p2["spacing"]["before"], 100);
    assert_eq!(p2["spacing"]["after"], 80);
}

#[test]
fn runs_list_with_scope() {
    let path = write_sample("list");
    let tree = inspect(&path, json!({ "from": 1, "maxParas": 20, "runs": true }));
    let p1 = para(&tree, "para:AAAA0001");
    let runs = p1["runs"].as_array().unwrap();
    assert_eq!(runs.len(), 2);
    assert_eq!(
        object_keys(&runs[0]),
        [
            "id",
            "index",
            "style",
            "text",
            "bold",
            "italic",
            "underline",
            "sz",
            "color",
            "font",
            "eastAsiaFont",
            "em",
            "eastAsianLayout",
            "highlight",
            "strike",
            "dstrike",
            "vertAlign",
            "spacing",
            "kern",
            "scale"
        ]
    );
    assert_eq!(runs[0]["id"], "para:AAAA0001/run:1");
    assert_eq!(runs[0]["index"], 1);
    assert_eq!(runs[0]["text"], "甲");
    assert!(runs[0]["bold"].is_null());
    assert!(runs[0]["color"].is_null());
    assert_eq!(runs[1]["id"], "para:AAAA0001/run:2");
    assert_eq!(runs[1]["text"], "乙");
    assert_eq!(runs[1]["color"], "FF0000");
    assert_eq!(object_keys(p1)[object_keys(p1).len() - 1], "runs");
}

#[test]
fn set_para_format_align_and_indent() {
    let path = write_sample("align");
    let out = tmp_dir().join("align-out.docx");
    apply(
        &path,
        &out,
        json!([{
            "op": "setParaFormat",
            "id": "para:AAAA0001",
            "align": "center",
            "left": 720
        }]),
    );
    let tree = inspect(&out, json!({ "from": 1, "maxParas": 20 }));
    let p1 = para(&tree, "para:AAAA0001");
    assert_eq!(p1["align"], "center");
    assert_eq!(p1["indent"]["left"], 720);
    let xml = unzip(&out, "word/document.xml");
    assert!(xml.contains("w:val=\"center\""));
    assert!(xml.contains("w:left=\"720\""));
    assert!(!xml.contains("w:start=\"720\""));
    assert_eq!(unzip(&out, "word/styles.xml"), STYLES);
}

#[test]
fn set_para_format_clear_align_and_spacing() {
    let path = write_sample("clear");
    let out = tmp_dir().join("clear-out.docx");
    apply(
        &path,
        &out,
        json!([{
            "op": "setParaFormat",
            "id": "para:AAAA0002",
            "align": null,
            "before": 240,
            "after": 120
        }]),
    );
    let tree = inspect(&out, json!({ "from": 1, "maxParas": 20 }));
    let p2 = para(&tree, "para:AAAA0002");
    assert!(p2["align"].is_null());
    assert_eq!(p2["spacing"]["before"], 240);
    assert_eq!(p2["spacing"]["after"], 120);
    let xml = unzip(&out, "word/document.xml");
    assert!(!xml.contains("w:jc"));
    assert!(xml.contains("w:before=\"240\""));
    assert!(xml.contains("w:after=\"120\""));
}

#[test]
fn set_run_format_bold_keeps_other_color() {
    let path = write_sample("bold");
    let out = tmp_dir().join("bold-out.docx");
    apply(
        &path,
        &out,
        json!([{
            "op": "setRunFormat",
            "id": "para:AAAA0001/run:1",
            "bold": true
        }]),
    );
    let tree = inspect(&out, json!({ "from": 1, "maxParas": 20, "runs": true }));
    let runs = para(&tree, "para:AAAA0001")["runs"].as_array().unwrap();
    assert_eq!(runs[0]["bold"], true);
    assert_eq!(runs[1]["color"], "FF0000");
    assert!(runs[1]["bold"].is_null());
    let xml = unzip(&out, "word/document.xml");
    assert!(xml.contains("<w:b") && xml.contains("w:val=\"FF0000\""));
    assert_eq!(unzip(&out, "word/styles.xml"), STYLES);
}

#[test]
fn set_run_format_off_writes_zero() {
    let path = write_sample("off");
    let primed = tmp_dir().join("off-on.docx");
    apply(
        &path,
        &primed,
        json!([{
            "op": "setRunFormat",
            "id": "para:AAAA0001/run:1",
            "bold": true
        }]),
    );
    let out = tmp_dir().join("off-out.docx");
    apply(
        &primed,
        &out,
        json!([{
            "op": "setRunFormat",
            "id": "para:AAAA0001/run:1",
            "bold": false
        }]),
    );
    let xml = unzip(&out, "word/document.xml");
    assert!(xml.contains("w:val=\"0\""));
    let tree = inspect(&out, json!({ "from": 1, "maxParas": 20, "runs": true }));
    assert_eq!(para(&tree, "para:AAAA0001")["runs"][0]["bold"], false);
}

#[test]
fn format_ops_reject_bad_input() {
    let path = write_sample("bad");
    let err_empty = apply_err(
        &path,
        json!([{ "op": "setParaFormat", "id": "para:AAAA0001" }]),
    );
    assert_eq!(err_empty.code(), ErrorCode::InvalidField);

    let err_align = apply_err(
        &path,
        json!([{ "op": "setParaFormat", "id": "para:AAAA0001", "align": "middle" }]),
    );
    assert_eq!(err_align.code(), ErrorCode::InvalidField);

    let err_both = apply_err(
        &path,
        json!([{
            "op": "setParaFormat",
            "id": "para:AAAA0001",
            "firstLine": 200,
            "hanging": 200
        }]),
    );
    assert_eq!(err_both.code(), ErrorCode::InvalidField);

    let err_run_empty = apply_err(
        &path,
        json!([{ "op": "setRunFormat", "id": "para:AAAA0001/run:1" }]),
    );
    assert_eq!(err_run_empty.code(), ErrorCode::InvalidField);

    let err_color = apply_err(
        &path,
        json!([{
            "op": "setRunFormat",
            "id": "para:AAAA0001/run:1",
            "color": "#ff0000"
        }]),
    );
    assert_eq!(err_color.code(), ErrorCode::InvalidField);

    let err_id = apply_err(
        &path,
        json!([{ "op": "setRunFormat", "id": "para:AAAA0001", "bold": true }]),
    );
    assert_eq!(err_id.code(), ErrorCode::InvalidId);

    let err_missing = apply_err(
        &path,
        json!([{ "op": "setRunFormat", "id": "para:AAAA0001/run:9", "bold": true }]),
    );
    assert_eq!(err_missing.code(), ErrorCode::IdNotFound);
}

#[test]
fn format_not_for_xlsx_or_pptx() {
    let xlsx = tmp_dir().join("fmt.xlsx");
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
            "scope": { "runs": true }
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    let out = tmp_dir().join("fmt-x-out.xlsx");
    let err_op = run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": xlsx.to_string_lossy(),
            "output": out.to_string_lossy(),
            "ops": [{ "op": "setParaFormat", "id": "para:1", "align": "center" }]
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err_op.code(), ErrorCode::InvalidField);

    let pptx = tmp_dir().join("fmt.pptx");
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
            "scope": { "runs": true }
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err_p.code(), ErrorCode::InvalidField);
    let pout = tmp_dir().join("fmt-p-out.pptx");
    let err_p_op = run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": pptx.to_string_lossy(),
            "output": pout.to_string_lossy(),
            "ops": [{ "op": "setRunFormat", "id": "para:1/run:1", "bold": true }]
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err_p_op.code(), ErrorCode::InvalidField);
}

#[test]
fn para_borders_shd_tabs_and_run_remain() {
    let path = write_sample("remain");
    let tree0 = inspect(&path, json!({ "from": 1, "maxParas": 20, "runs": true }));
    let p1 = para(&tree0, "para:AAAA0001");
    assert!(p1["borders"].is_null());
    assert!(p1["shd"].is_null());
    assert!(p1["tabs"].is_null());
    let r0 = &p1["runs"][0];
    assert!(r0["highlight"].is_null());
    assert!(r0["strike"].is_null());
    assert!(r0["dstrike"].is_null());
    assert!(r0["vertAlign"].is_null());
    assert!(r0["spacing"].is_null());
    assert!(r0["kern"].is_null());
    assert!(r0["scale"].is_null());

    let out = tmp_dir().join("remain-out.docx");
    apply(
        &path,
        &out,
        json!([
            {
                "op": "setParaFormat",
                "id": "para:AAAA0001",
                "borders": {
                    "top": { "val": "single", "sz": 12, "space": 1, "color": "000000" }
                },
                "shd": { "fill": "ffff00" },
                "tabs": [{ "val": "left", "pos": 720 }]
            },
            {
                "op": "setRunFormat",
                "id": "para:AAAA0001/run:1",
                "highlight": "yellow",
                "strike": true,
                "dstrike": true,
                "vertAlign": "superscript"
            }
        ]),
    );
    let tree = inspect(&out, json!({ "from": 1, "maxParas": 20, "runs": true }));
    let p = para(&tree, "para:AAAA0001");
    assert_eq!(p["borders"]["top"]["val"], "single");
    assert_eq!(p["borders"]["top"]["sz"], 12);
    assert_eq!(p["shd"]["fill"], "FFFF00");
    assert_eq!(p["tabs"][0]["val"], "left");
    assert_eq!(p["tabs"][0]["pos"], 720);
    let r = &p["runs"][0];
    assert_eq!(r["highlight"], "yellow");
    assert_eq!(r["strike"], true);
    assert_eq!(r["dstrike"], true);
    assert_eq!(r["vertAlign"], "superscript");
    let xml = unzip(&out, "word/document.xml");
    assert!(xml.contains("w:pBdr"), "{xml}");
    assert!(xml.contains("w:shd"), "{xml}");
    assert!(xml.contains("w:tabs"), "{xml}");
    assert!(xml.contains("w:highlight"), "{xml}");
    assert!(xml.contains("w:strike"), "{xml}");
    assert!(xml.contains("w:dstrike"), "{xml}");
    assert!(xml.contains("w:vertAlign"), "{xml}");

    let cleared = tmp_dir().join("remain-clear.docx");
    apply(
        &out,
        &cleared,
        json!([
            {
                "op": "setParaFormat",
                "id": "para:AAAA0001",
                "borders": null,
                "shd": null,
                "tabs": null
            },
            {
                "op": "setRunFormat",
                "id": "para:AAAA0001/run:1",
                "highlight": null,
                "strike": null,
                "dstrike": null,
                "vertAlign": null
            }
        ]),
    );
    let tree2 = inspect(&cleared, json!({ "from": 1, "maxParas": 20, "runs": true }));
    let p2 = para(&tree2, "para:AAAA0001");
    assert!(p2["borders"].is_null());
    assert!(p2["shd"].is_null());
    assert!(p2["tabs"].is_null());
    let r2 = &p2["runs"][0];
    assert!(r2["highlight"].is_null());
    assert!(r2["strike"].is_null());
    assert!(r2["dstrike"].is_null());
    assert!(r2["vertAlign"].is_null());

    let err_hl = apply_err(
        &path,
        json!([{
            "op": "setRunFormat",
            "id": "para:AAAA0001/run:1",
            "highlight": "pink"
        }]),
    );
    assert_eq!(err_hl.code(), ErrorCode::InvalidField);
    let err_tab = apply_err(
        &path,
        json!([{
            "op": "setParaFormat",
            "id": "para:AAAA0001",
            "tabs": [{ "val": "start", "pos": 720 }]
        }]),
    );
    assert_eq!(err_tab.code(), ErrorCode::InvalidField);
    let err_box = apply_err(
        &path,
        json!([{ "op": "setParaFormat", "id": "para:AAAA0001", "borders": {} }]),
    );
    assert_eq!(err_box.code(), ErrorCode::InvalidField);

    let xlsx = tmp_dir().join("remain.xlsx");
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
    let xout = tmp_dir().join("remain-x-out.xlsx");
    let err_x = run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": xlsx.to_string_lossy(),
            "output": xout.to_string_lossy(),
            "ops": [{
                "op": "setRunFormat",
                "id": "para:AAAA0001/run:1",
                "highlight": "yellow"
            }]
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err_x.code(), ErrorCode::InvalidField);
}

#[test]
fn set_run_format_spacing_and_kern() {
    let path = write_sample("spacing");
    let tree0 = inspect(&path, json!({ "from": 1, "maxParas": 20, "runs": true }));
    let r0 = &para(&tree0, "para:AAAA0001")["runs"][0];
    assert!(r0["spacing"].is_null());
    assert!(r0["kern"].is_null());

    let out = tmp_dir().join("spacing-out.docx");
    apply(
        &path,
        &out,
        json!([{
            "op": "setRunFormat",
            "id": "para:AAAA0001/run:1",
            "spacing": 40,
            "kern": 28
        }]),
    );
    let tree = inspect(&out, json!({ "from": 1, "maxParas": 20, "runs": true }));
    let r = &para(&tree, "para:AAAA0001")["runs"][0];
    assert_eq!(r["spacing"], 40);
    assert_eq!(r["kern"], 28);
    let xml = unzip(&out, "word/document.xml");
    assert!(xml.contains("w:spacing"), "{xml}");
    assert!(xml.contains("w:kern"), "{xml}");
    assert!(xml.contains(r#"w:val="40""#), "{xml}");
    assert!(xml.contains(r#"w:val="28""#), "{xml}");

    let neg = tmp_dir().join("spacing-neg.docx");
    apply(
        &out,
        &neg,
        json!([{
            "op": "setRunFormat",
            "id": "para:AAAA0001/run:1",
            "spacing": -20
        }]),
    );
    let tree_n = inspect(&neg, json!({ "from": 1, "maxParas": 20, "runs": true }));
    let rn = &para(&tree_n, "para:AAAA0001")["runs"][0];
    assert_eq!(rn["spacing"], -20);
    assert_eq!(rn["kern"], 28);

    let cleared = tmp_dir().join("spacing-clear.docx");
    apply(
        &neg,
        &cleared,
        json!([{
            "op": "setRunFormat",
            "id": "para:AAAA0001/run:1",
            "spacing": null,
            "kern": null
        }]),
    );
    let tree2 = inspect(&cleared, json!({ "from": 1, "maxParas": 20, "runs": true }));
    let r2 = &para(&tree2, "para:AAAA0001")["runs"][0];
    assert!(r2["spacing"].is_null());
    assert!(r2["kern"].is_null());
    let xml2 = unzip(&cleared, "word/document.xml");
    assert!(!xml2.contains("w:kern"), "{xml2}");

    let err_s = apply_err(
        &path,
        json!([{
            "op": "setRunFormat",
            "id": "para:AAAA0001/run:1",
            "spacing": "wide"
        }]),
    );
    assert_eq!(err_s.code(), ErrorCode::InvalidField);
    let err_k = apply_err(
        &path,
        json!([{
            "op": "setRunFormat",
            "id": "para:AAAA0001/run:1",
            "kern": -1
        }]),
    );
    assert_eq!(err_k.code(), ErrorCode::InvalidField);

    let created = tmp_dir().join("spacing-create.docx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": created.to_string_lossy(),
            "format": "docx",
            "document": {
                "paragraphs": [{
                    "runs": [{ "text": "字", "spacing": 40, "kern": 28 }]
                }]
            }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let tree_c = inspect(&created, json!({ "from": 1, "maxParas": 5, "runs": true }));
    let rc = &tree_c["paragraphs"][0]["runs"][0];
    assert_eq!(rc["spacing"], 40);
    assert_eq!(rc["kern"], 28);
}

#[test]
fn set_run_format_scale() {
    let path = write_sample("scale");
    let tree0 = inspect(&path, json!({ "from": 1, "maxParas": 20, "runs": true }));
    let r0 = &para(&tree0, "para:AAAA0001")["runs"][0];
    assert!(r0["scale"].is_null());

    let out = tmp_dir().join("scale-out.docx");
    apply(
        &path,
        &out,
        json!([{
            "op": "setRunFormat",
            "id": "para:AAAA0001/run:1",
            "scale": 200
        }]),
    );
    let tree = inspect(&out, json!({ "from": 1, "maxParas": 20, "runs": true }));
    let r = &para(&tree, "para:AAAA0001")["runs"][0];
    assert_eq!(r["scale"], 200);
    let xml = unzip(&out, "word/document.xml");
    assert!(xml.contains(r#"<w:w w:val="200""#), "{xml}");

    let half = tmp_dir().join("scale-50.docx");
    apply(
        &out,
        &half,
        json!([{
            "op": "setRunFormat",
            "id": "para:AAAA0001/run:1",
            "scale": 50
        }]),
    );
    let tree_h = inspect(&half, json!({ "from": 1, "maxParas": 20, "runs": true }));
    assert_eq!(para(&tree_h, "para:AAAA0001")["runs"][0]["scale"], 50);

    let cleared = tmp_dir().join("scale-clear.docx");
    apply(
        &half,
        &cleared,
        json!([{
            "op": "setRunFormat",
            "id": "para:AAAA0001/run:1",
            "scale": null
        }]),
    );
    let tree2 = inspect(&cleared, json!({ "from": 1, "maxParas": 20, "runs": true }));
    assert!(para(&tree2, "para:AAAA0001")["runs"][0]["scale"].is_null());
    let xml2 = unzip(&cleared, "word/document.xml");
    assert!(!xml2.contains("<w:w w:val"), "{xml2}");

    let err0 = apply_err(
        &path,
        json!([{ "op": "setRunFormat", "id": "para:AAAA0001/run:1", "scale": 0 }]),
    );
    assert_eq!(err0.code(), ErrorCode::InvalidField);
    let err601 = apply_err(
        &path,
        json!([{ "op": "setRunFormat", "id": "para:AAAA0001/run:1", "scale": 601 }]),
    );
    assert_eq!(err601.code(), ErrorCode::InvalidField);

    let created = tmp_dir().join("scale-create.docx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": created.to_string_lossy(),
            "format": "docx",
            "document": {
                "paragraphs": [{
                    "runs": [{ "text": "字", "scale": 200 }]
                }]
            }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let tree_c = inspect(&created, json!({ "from": 1, "maxParas": 5, "runs": true }));
    assert_eq!(tree_c["paragraphs"][0]["runs"][0]["scale"], 200);

    let xlsx = tmp_dir().join("scale.xlsx");
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
    let err_x = run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": xlsx.to_string_lossy(),
            "output": tmp_dir().join("scale-x-out.xlsx").to_string_lossy(),
            "ops": [{
                "op": "setRunFormat",
                "id": "para:AAAA0001/run:1",
                "scale": 200
            }]
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err_x.code(), ErrorCode::InvalidField);
}

#[test]
fn set_para_runs_splits_existing_paragraph() {
    let path = write_sample("para-runs");
    let out = tmp_dir().join("para-runs-out.docx");
    apply(
        &path,
        &out,
        json!([{
            "op": "setParaRuns",
            "id": "para:AAAA0002",
            "runs": [
                { "text": "第" },
                { "text": "二段", "color": "0000FF", "underline": "wave" }
            ]
        }]),
    );
    let tree = inspect(&out, json!({ "from": 1, "maxParas": 5, "runs": true }));
    let runs = para(&tree, "para:AAAA0002")["runs"].as_array().unwrap();
    assert_eq!(runs.len(), 2);
    assert_eq!(runs[0]["text"], "第");
    assert_eq!(runs[1]["text"], "二段");
    assert_eq!(runs[1]["color"], "0000FF");
    assert_eq!(runs[1]["underline"], "wave");

    let colored = tmp_dir().join("para-runs-fmt.docx");
    apply(
        &out,
        &colored,
        json!([{
            "op": "setRunFormat",
            "id": "para:AAAA0002/run:1",
            "color": "00AA00"
        }]),
    );
    let tree = inspect(&colored, json!({ "from": 1, "maxParas": 5, "runs": true }));
    let runs = para(&tree, "para:AAAA0002")["runs"].as_array().unwrap();
    assert_eq!(runs[0]["color"], "00AA00");
    assert_eq!(runs[1]["color"], "0000FF");

    let ranged = tmp_dir().join("para-runs-range.docx");
    apply(
        &path,
        &ranged,
        json!([{
            "op": "setParaRuns",
            "id": "para:AAAA0002",
            "runs": [
                { "start": 0, "end": 1 },
                { "start": 1, "end": 3, "underline": "wave" }
            ]
        }]),
    );
    let tree = inspect(&ranged, json!({ "from": 1, "maxParas": 5, "runs": true }));
    let runs = para(&tree, "para:AAAA0002")["runs"].as_array().unwrap();
    assert_eq!(runs[0]["text"], "第");
    assert_eq!(runs[1]["text"], "二段");
    assert_eq!(runs[1]["underline"], "wave");

    let matched = tmp_dir().join("para-runs-match.docx");
    apply(
        &path,
        &matched,
        json!([{
            "op": "setParaRuns",
            "id": "para:AAAA0002",
            "runs": [{ "match": "二段", "color": "FF0000", "underline": "wave" }]
        }]),
    );
    let tree = inspect(&matched, json!({ "from": 1, "maxParas": 5, "runs": true }));
    let runs = para(&tree, "para:AAAA0002")["runs"].as_array().unwrap();
    assert_eq!(runs.len(), 2);
    assert_eq!(runs[0]["text"], "第");
    assert!(runs[0]["color"].is_null());
    assert_eq!(runs[1]["text"], "二段");
    assert_eq!(runs[1]["color"], "FF0000");

    let err = apply_err(
        &path,
        json!([{
            "op": "setParaRuns",
            "id": "para:AAAA0002",
            "runs": [{ "text": "你好" }]
        }]),
    );
    assert_eq!(err.code(), ErrorCode::InvalidField);
    let err = apply_err(
        &path,
        json!([{
            "op": "setParaRuns",
            "id": "para:AAAA0002",
            "runs": [{ "match": "不存在" }]
        }]),
    );
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

#[test]
fn set_para_runs_match_picks_one_hit_in_a_long_sentence() {
    let path = write_sample("para-long");
    let filled = tmp_dir().join("para-long-filled.docx");
    apply(
        &path,
        &filled,
        json!([{
            "op": "setParaText",
            "id": "para:AAAA0002",
            "text": "甲方应当赔偿，乙方也应当赔偿。"
        }]),
    );
    let out = tmp_dir().join("para-long-out.docx");
    apply(
        &filled,
        &out,
        json!([{
            "op": "setParaRuns",
            "id": "para:AAAA0002",
            "runs": [{
                "match": "赔偿",
                "after": "乙方也应当",
                "color": "FF0000",
                "underline": "wave"
            }]
        }]),
    );
    let tree = inspect(&out, json!({ "from": 1, "maxParas": 5, "runs": true }));
    let runs = para(&tree, "para:AAAA0002")["runs"].as_array().unwrap();
    let colored: Vec<_> = runs
        .iter()
        .filter(|run| run["color"] == "FF0000")
        .map(|run| run["text"].as_str().unwrap())
        .collect();
    assert_eq!(colored, ["赔偿"]);
    let joined: String = runs
        .iter()
        .map(|run| run["text"].as_str().unwrap())
        .collect();
    assert_eq!(joined, "甲方应当赔偿，乙方也应当赔偿。");

    let by_nth = tmp_dir().join("para-long-nth.docx");
    apply(
        &filled,
        &by_nth,
        json!([{
            "op": "setParaRuns",
            "id": "para:AAAA0002",
            "runs": [{ "match": "赔偿", "nth": 2, "underline": "wave" }]
        }]),
    );
    let tree = inspect(&by_nth, json!({ "from": 1, "maxParas": 5, "runs": true }));
    let runs = para(&tree, "para:AAAA0002")["runs"].as_array().unwrap();
    let waved: Vec<_> = runs
        .iter()
        .filter(|run| run["underline"] == "wave")
        .map(|run| run["text"].as_str().unwrap())
        .collect();
    assert_eq!(waved, ["赔偿"]);
    let joined: String = runs
        .iter()
        .map(|run| run["text"].as_str().unwrap())
        .collect();
    assert_eq!(joined, "甲方应当赔偿，乙方也应当赔偿。");
}

#[test]
fn set_para_runs_cuts_between_two_anchors() {
    let path = write_sample("para-cuts");
    let filled = tmp_dir().join("para-cuts-filled.docx");
    let sentence = "前半这里是很长的中间，包括全部直接损失以及因此产生的合理费用结尾之后";
    apply(
        &path,
        &filled,
        json!([{
            "op": "setParaText",
            "id": "para:AAAA0002",
            "text": sentence
        }]),
    );
    let out = tmp_dir().join("para-cuts-out.docx");
    apply(
        &filled,
        &out,
        json!([{
            "op": "setParaRuns",
            "id": "para:AAAA0002",
            "runs": [{
                "after": "前半",
                "before": "结尾",
                "color": "FF0000",
                "underline": "wave"
            }]
        }]),
    );
    let tree = inspect(&out, json!({ "from": 1, "maxParas": 5, "runs": true }));
    let runs = para(&tree, "para:AAAA0002")["runs"].as_array().unwrap();
    let colored: Vec<_> = runs
        .iter()
        .filter(|run| run["color"] == "FF0000")
        .map(|run| run["text"].as_str().unwrap())
        .collect();
    assert_eq!(
        colored,
        ["这里是很长的中间，包括全部直接损失以及因此产生的合理费用"]
    );
    let joined: String = runs
        .iter()
        .map(|run| run["text"].as_str().unwrap())
        .collect();
    assert_eq!(joined, sentence);
}
