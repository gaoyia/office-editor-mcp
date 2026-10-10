//! docx 公式：insertMath / setMath / deleteMath、inspect mathCount。
//! xlsx / pptx 上 Word 段落 id 报 InvalidId；xlsx 认 scope.maths。

use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::write::SimpleFileOptions;
use zip::CompressionMethod;
use zip::ZipWriter;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-docx-math");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn seq_path(tag: &str, ext: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}.{ext}"))
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

fn inspect(path: &Path, maths: bool, header_footer: bool) -> Value {
    let RunOutput::Inspect(tree) = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": path.to_string_lossy(),
            "scope": {
                "from": 1,
                "maxParas": 20,
                "maths": maths,
                "headerFooter": header_footer
            }
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
    let out = seq_path("should-not-write", "docx");
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

fn first_body_para_id(tree: &Value) -> String {
    tree["paragraphs"][0]["id"].as_str().unwrap().to_string()
}

fn object_keys(v: &Value) -> Vec<&str> {
    v.as_object().unwrap().keys().map(|k| k.as_str()).collect()
}

#[test]
fn insert_inline_lists_and_delete_keeps_heading() {
    let src = create_docx(
        "math-src",
        json!({
            "paragraphs": [
                { "style": "Heading1", "text": "第一章" },
                { "text": "hello" }
            ]
        }),
    );
    let tree0 = inspect(&src, false, false);
    assert_eq!(tree0["mathCount"], 0);
    assert!(tree0.get("maths").is_none());
    let p1 = first_body_para_id(&tree0);
    let out = seq_path("math-out", "docx");
    apply(
        &src,
        &out,
        json!([{ "op": "insertMath", "id": p1, "text": "E=mc^2" }]),
    );
    let tree = inspect(&out, true, false);
    assert_eq!(tree["mathCount"], 1);
    assert_eq!(
        object_keys(&tree["maths"][0]),
        ["id", "index", "kind", "jc", "host", "text", "expr"]
    );
    assert_eq!(tree["maths"][0]["id"], "math:1");
    assert_eq!(tree["maths"][0]["kind"], "inline");
    assert!(tree["maths"][0]["jc"].is_null());
    assert_eq!(tree["maths"][0]["text"], "E=mc^2");
    assert_eq!(
        tree["maths"][0]["expr"],
        json!([{ "type": "text", "text": "E=mc^2" }])
    );
    assert_eq!(tree["maths"][0]["host"], p1);
    assert!(tree["paragraphs"][0]["text"]
        .as_str()
        .unwrap()
        .contains("第一章"));

    let gone = seq_path("math-del", "docx");
    apply(&out, &gone, json!([{ "op": "deleteMath", "id": "math:1" }]));
    let tree2 = inspect(&gone, true, false);
    assert_eq!(tree2["mathCount"], 0);
    assert_eq!(tree2["maths"].as_array().unwrap().len(), 0);
    let texts: Vec<&str> = tree2["paragraphs"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|p| p["text"].as_str())
        .collect();
    assert!(texts.contains(&"第一章"));
    assert!(texts.contains(&"hello"));
}

#[test]
fn insert_display_and_set_math() {
    let src = create_docx(
        "disp-src",
        json!({ "paragraphs": [{ "text": "甲" }, { "text": "乙" }] }),
    );
    let p1 = first_body_para_id(&inspect(&src, false, false));
    let out = seq_path("disp", "docx");
    apply(
        &src,
        &out,
        json!([{ "op": "insertMath", "id": p1, "kind": "display", "text": "a^2+b^2=c^2" }]),
    );
    let tree = inspect(&out, true, false);
    assert_eq!(tree["mathCount"], 1);
    assert_eq!(tree["maths"][0]["kind"], "display");
    assert_eq!(tree["maths"][0]["text"], "a^2+b^2=c^2");
    assert_eq!(tree["paragraphs"][0]["text"], "甲");

    let set = seq_path("set", "docx");
    apply(
        &out,
        &set,
        json!([{ "op": "setMath", "id": "math:1", "text": "x=1" }]),
    );
    let tree2 = inspect(&set, true, false);
    assert_eq!(tree2["maths"][0]["kind"], "display");
    assert_eq!(tree2["maths"][0]["text"], "x=1");
}

#[test]
fn insert_at_document_end_and_header() {
    let src = create_docx(
        "end-src",
        json!({
            "paragraphs": [{ "text": "甲" }],
            "headers": [{ "type": "default", "paragraphs": [{ "text": "页眉" }] }]
        }),
    );
    let end = seq_path("end", "docx");
    apply(
        &src,
        &end,
        json!([{ "op": "insertMath", "id": "docx:document", "kind": "inline", "text": "1+1" }]),
    );
    let tree = inspect(&end, true, true);
    assert_eq!(tree["mathCount"], 1);
    assert_eq!(tree["maths"][0]["text"], "1+1");
    assert_eq!(tree["headers"][0]["mathCount"], 0);

    let hf_para = tree["headers"][0]["paragraphs"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let hf = seq_path("hf", "docx");
    apply(
        &end,
        &hf,
        json!([{ "op": "insertMath", "id": hf_para, "text": "h=1" }]),
    );
    let tree2 = inspect(&hf, true, true);
    assert_eq!(tree2["mathCount"], 1);
    assert_eq!(tree2["headers"][0]["mathCount"], 1);
    assert_eq!(
        tree2["headers"][0]["maths"][0]["id"],
        "header:default:1/math:1"
    );
    assert_eq!(tree2["headers"][0]["maths"][0]["text"], "h=1");
    assert_eq!(tree2["headers"][0]["maths"][0]["kind"], "inline");
}

#[test]
fn set_para_text_blocked_and_bad_ids() {
    let src = create_docx("guard-src", json!({ "paragraphs": [{ "text": "甲" }] }));
    let p1 = first_body_para_id(&inspect(&src, false, false));
    let out = seq_path("guard", "docx");
    apply(
        &src,
        &out,
        json!([{ "op": "insertMath", "id": p1, "text": "E=mc^2" }]),
    );
    let err = apply_err(
        &out,
        json!([{ "op": "setParaText", "id": p1, "text": "改" }]),
    );
    assert_eq!(err.code(), ErrorCode::NotImplemented);

    let err_empty = apply_err(&src, json!([{ "op": "insertMath", "id": p1, "text": "" }]));
    assert_eq!(err_empty.code(), ErrorCode::InvalidField);

    let err_kind = apply_err(
        &src,
        json!([{ "op": "insertMath", "id": p1, "kind": "nope", "text": "x" }]),
    );
    assert_eq!(err_kind.code(), ErrorCode::InvalidField);

    let err_id = apply_err(
        &src,
        json!([{ "op": "setMath", "id": "field:1", "text": "x" }]),
    );
    assert_eq!(err_id.code(), ErrorCode::InvalidId);

    let err_miss = apply_err(&src, json!([{ "op": "deleteMath", "id": "math:1" }]));
    assert_eq!(err_miss.code(), ErrorCode::IdNotFound);
}

#[test]
fn xlsx_pptx_reject_math_ops() {
    let xlsx = seq_path("math-x", "xlsx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": xlsx.to_string_lossy(),
            "workbook": { "sheets": [{ "name": "Sheet1", "cells": [{ "ref": "A1", "value": 1 }] }] }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let xout = seq_path("math-x-out", "xlsx");
    let err_x = run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": xlsx.to_string_lossy(),
            "output": xout.to_string_lossy(),
            "ops": [{ "op": "insertMath", "id": "docx:document", "text": "x" }]
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err_x.code(), ErrorCode::InvalidId);

    let listed = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": xlsx.to_string_lossy(),
            "scope": { "sheet": "Sheet1", "range": "A1", "maths": true }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let RunOutput::Inspect(tree) = listed else {
        panic!("inspect 应返回树");
    };
    assert_eq!(tree["mathCount"], 0);
    assert_eq!(tree["sheets"][0]["maths"], json!([]));

    let pptx = seq_path("math-p", "pptx");
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
    let pout = seq_path("math-p-out", "pptx");
    let err_p = run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": pptx.to_string_lossy(),
            "output": pout.to_string_lossy(),
            "ops": [{ "op": "deleteMath", "id": "math:1" }]
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err_p.code(), ErrorCode::InvalidId);
}

const CT: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>"#;

const ROOT_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#;

fn write_docx_xml(tag: &str, document: &str) -> PathBuf {
    let path = seq_path(tag, "docx");
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut zip = ZipWriter::new(&mut cursor);
        let opt = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        for (name, body) in [
            ("[Content_Types].xml", CT),
            ("_rels/.rels", ROOT_RELS),
            ("word/document.xml", document),
        ] {
            zip.start_file(name, opt).unwrap();
            zip.write_all(body.as_bytes()).unwrap();
        }
        zip.finish().unwrap();
    }
    std::fs::write(&path, cursor.into_inner()).unwrap();
    path
}

#[test]
fn inspect_flattens_fraction_and_set_math_replaces_children() {
    let src = write_docx_xml(
        "frac",
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math" xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml"><w:body><w:p w14:paraId="AAAA0001"><m:oMath><m:f><m:num><m:r><m:t>1</m:t></m:r></m:num><m:den><m:r><m:t>2</m:t></m:r></m:den></m:f></m:oMath></w:p><w:p w14:paraId="AAAA0002"><w:r><w:t>hello</w:t></w:r></w:p><w:sectPr><w:pgSz w:w="11906" w:h="16838"/></w:sectPr></w:body></w:document>"#,
    );
    let tree0 = inspect(&src, true, false);
    assert_eq!(tree0["mathCount"], 1);
    assert_eq!(tree0["maths"][0]["kind"], "inline");
    assert_eq!(tree0["maths"][0]["text"], "12");
    assert_eq!(
        tree0["maths"][0]["expr"],
        json!([{
            "type": "frac",
            "num": [{ "type": "text", "text": "1" }],
            "den": [{ "type": "text", "text": "2" }]
        }])
    );

    let out = seq_path("frac-set", "docx");
    apply(
        &src,
        &out,
        json!([{ "op": "setMath", "id": "math:1", "text": "3" }]),
    );
    let tree = inspect(&out, true, false);
    assert_eq!(tree["maths"][0]["text"], "3");
    assert_eq!(tree["maths"][0]["kind"], "inline");
}

#[test]
fn nested_omath_is_one_item_display_wraps() {
    let src = write_docx_xml(
        "nested",
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math" xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml"><w:body><w:p w14:paraId="AAAA0001"><m:oMathPara><m:oMath><m:sSub><m:e><m:r><m:t>a</m:t></m:r></m:e><m:sub><m:r><m:t>1</m:t></m:r></m:sub></m:sSub></m:oMath></m:oMathPara></w:p><w:sectPr><w:pgSz w:w="11906" w:h="16838"/></w:sectPr></w:body></w:document>"#,
    );
    let tree = inspect(&src, true, false);
    assert_eq!(tree["mathCount"], 1);
    assert_eq!(tree["maths"][0]["kind"], "display");
    assert_eq!(tree["maths"][0]["text"], "a1");
    assert_eq!(tree["maths"][0]["expr"][0]["type"], "sSub");
}

fn frac_expr() -> Value {
    json!([{
        "type": "frac",
        "num": [{ "type": "text", "text": "a" }],
        "den": [{ "type": "text", "text": "b" }]
    }])
}

#[test]
fn insert_and_create_frac_expr() {
    let src = create_docx("frac-src", json!({ "paragraphs": [{ "text": "甲" }] }));
    let p1 = first_body_para_id(&inspect(&src, false, false));
    let out = seq_path("frac-ins", "docx");
    apply(
        &src,
        &out,
        json!([{ "op": "insertMath", "id": p1, "kind": "display", "expr": frac_expr() }]),
    );
    let tree = inspect(&out, true, false);
    assert_eq!(tree["mathCount"], 1);
    assert_eq!(tree["maths"][0]["kind"], "display");
    assert_eq!(tree["maths"][0]["text"], "ab");
    assert_eq!(tree["maths"][0]["expr"], frac_expr());

    let created = create_docx(
        "frac-create",
        json!({
            "paragraphs": [{
                "text": "甲",
                "maths": [{ "kind": "inline", "expr": frac_expr() }]
            }]
        }),
    );
    let tree_c = inspect(&created, true, false);
    assert_eq!(tree_c["maths"][0]["expr"][0]["type"], "frac");
    assert_eq!(tree_c["maths"][0]["text"], "ab");
}

#[test]
fn set_math_expr_keeps_structure_text_flattens() {
    let src = create_docx("set-src", json!({ "paragraphs": [{ "text": "甲" }] }));
    let p1 = first_body_para_id(&inspect(&src, false, false));
    let out = seq_path("frac-set-src", "docx");
    apply(
        &src,
        &out,
        json!([{ "op": "insertMath", "id": p1, "text": "x" }]),
    );
    let structured = seq_path("frac-set-expr", "docx");
    apply(
        &out,
        &structured,
        json!([{ "op": "setMath", "id": "math:1", "expr": frac_expr() }]),
    );
    let tree = inspect(&structured, true, false);
    assert_eq!(tree["maths"][0]["expr"][0]["type"], "frac");
    assert_eq!(tree["maths"][0]["text"], "ab");

    let flat = seq_path("frac-flat", "docx");
    apply(
        &structured,
        &flat,
        json!([{ "op": "setMath", "id": "math:1", "text": "z" }]),
    );
    let tree2 = inspect(&flat, true, false);
    assert_eq!(tree2["maths"][0]["text"], "z");
    assert_eq!(
        tree2["maths"][0]["expr"],
        json!([{ "type": "text", "text": "z" }])
    );
}

#[test]
fn insert_math_rejects_both_or_neither_or_other() {
    let src = create_docx("rej-src", json!({ "paragraphs": [{ "text": "甲" }] }));
    let p1 = first_body_para_id(&inspect(&src, false, false));
    let both = apply_err(
        &src,
        json!([{ "op": "insertMath", "id": p1, "text": "x", "expr": frac_expr() }]),
    );
    assert_eq!(both.code(), ErrorCode::InvalidField);

    let neither = apply_err(&src, json!([{ "op": "insertMath", "id": p1 }]));
    assert_eq!(neither.code(), ErrorCode::InvalidField);

    let other = apply_err(
        &src,
        json!([{
            "op": "insertMath",
            "id": p1,
            "expr": [{ "type": "other", "name": "box", "text": "x" }]
        }]),
    );
    assert_eq!(other.code(), ErrorCode::InvalidField);
}

#[test]
fn insert_rad_nary_and_nested() {
    let src = create_docx("more-src", json!({ "paragraphs": [{ "text": "甲" }] }));
    let p1 = first_body_para_id(&inspect(&src, false, false));
    let out = seq_path("more", "docx");
    apply(
        &src,
        &out,
        json!([{
            "op": "insertMath",
            "id": p1,
            "expr": [{
                "type": "sSup",
                "e": [{
                    "type": "delim",
                    "args": [[
                        {
                            "type": "frac",
                            "num": [{ "type": "text", "text": "a" }],
                            "den": [{ "type": "text", "text": "b" }]
                        }
                    ]]
                }],
                "sup": [{ "type": "text", "text": "2" }]
            }]
        }]),
    );
    let tree = inspect(&out, true, false);
    assert_eq!(tree["maths"][0]["expr"][0]["type"], "sSup");
    assert_eq!(tree["maths"][0]["text"], "ab2");

    let rad = seq_path("rad", "docx");
    apply(
        &src,
        &rad,
        json!([{
            "op": "insertMath",
            "id": p1,
            "expr": [{
                "type": "rad",
                "degHide": true,
                "e": [{ "type": "text", "text": "x" }]
            }]
        }]),
    );
    let tree_r = inspect(&rad, true, false);
    assert_eq!(tree_r["maths"][0]["expr"][0]["type"], "rad");
    assert_eq!(tree_r["maths"][0]["expr"][0]["degHide"], true);
    assert_eq!(tree_r["maths"][0]["text"], "x");

    let nary = seq_path("nary", "docx");
    apply(
        &src,
        &nary,
        json!([{
            "op": "insertMath",
            "id": p1,
            "expr": [{
                "type": "nary",
                "chr": "∑",
                "sub": [{ "type": "text", "text": "i=1" }],
                "sup": [{ "type": "text", "text": "n" }],
                "e": [{ "type": "text", "text": "i" }]
            }]
        }]),
    );
    let tree_n = inspect(&nary, true, false);
    assert_eq!(tree_n["maths"][0]["expr"][0]["type"], "nary");
    assert_eq!(tree_n["maths"][0]["expr"][0]["chr"], "∑");
    assert_eq!(tree_n["maths"][0]["text"], "i=1ni");
}
