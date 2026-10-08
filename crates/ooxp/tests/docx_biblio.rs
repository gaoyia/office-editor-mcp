//! docx 参考文献：insertSource / insertCitation / insertBibliography、inspect 计数、xlsx/pptx 拒绝。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-docx-biblio");
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

fn inspect(path: &Path, sources: bool, citations: bool, biblios: bool, fields: bool) -> Value {
    let RunOutput::Inspect(tree) = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": path.to_string_lossy(),
            "scope": {
                "from": 1,
                "maxParas": 20,
                "sources": sources,
                "citations": citations,
                "bibliographies": biblios,
                "fields": fields,
                "headerFooter": true
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

fn inspect_bare(path: &Path) -> Value {
    let RunOutput::Inspect(tree) = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": path.to_string_lossy()
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

fn source_op() -> Value {
    json!({
        "op": "insertSource",
        "id": "docx:document",
        "tag": "Book01",
        "sourceType": "Book",
        "title": "Office Open XML",
        "year": "2006",
        "last": "Ecma",
        "first": "International",
        "city": "Geneva",
        "publisher": "Ecma International"
    })
}

#[test]
fn insert_source_citation_bibliography_then_delete() {
    let src = create_docx(
        "bi-src",
        json!({
            "paragraphs": [
                { "style": "Heading1", "text": "第一章" },
                { "text": "hello" }
            ],
            "headers": [{ "type": "default", "paragraphs": [{ "text": "页眉" }] }]
        }),
    );
    let bare = inspect_bare(&src);
    assert_eq!(bare["sourceCount"], 0);
    assert_eq!(bare["citationCount"], 0);
    assert_eq!(bare["bibliographyCount"], 0);
    assert!(bare.get("sources").is_none());
    assert!(bare.get("citations").is_none());
    assert!(bare.get("bibliographies").is_none());

    let tree0 = inspect(&src, false, false, false, false);
    assert_eq!(tree0["sourceCount"], 0);
    assert!(tree0.get("sources").is_none());
    let p2 = tree0["paragraphs"][1]["id"].as_str().unwrap().to_string();

    let with_src = seq_path("bi-src-ins", "docx");
    apply(&src, &with_src, json!([source_op()]));
    let tree_src = inspect(&with_src, true, false, false, false);
    assert_eq!(tree_src["sourceCount"], 1);
    assert_eq!(
        object_keys(&tree_src["sources"][0]),
        [
            "id",
            "index",
            "tag",
            "sourceType",
            "title",
            "year",
            "author"
        ]
    );
    assert_eq!(tree_src["sources"][0]["id"], "source:1");
    assert_eq!(tree_src["sources"][0]["tag"], "Book01");
    assert_eq!(tree_src["sources"][0]["sourceType"], "Book");
    assert_eq!(tree_src["sources"][0]["title"], "Office Open XML");
    assert_eq!(tree_src["sources"][0]["year"], "2006");
    assert_eq!(tree_src["sources"][0]["author"], "Ecma, International");

    let with_cite = seq_path("bi-cite", "docx");
    apply(
        &with_src,
        &with_cite,
        json!([{ "op": "insertCitation", "id": p2, "tag": "Book01" }]),
    );
    let tree_cite = inspect(&with_cite, true, true, false, true);
    assert_eq!(tree_cite["citationCount"], 1);
    assert_eq!(tree_cite["contentControlCount"], 0);
    assert_eq!(
        object_keys(&tree_cite["citations"][0]),
        ["id", "index", "field", "instr", "tag", "host", "end", "text"]
    );
    assert_eq!(tree_cite["citations"][0]["id"], "cite:1");
    assert_eq!(tree_cite["citations"][0]["tag"], "Book01");
    assert_eq!(tree_cite["fields"][0]["code"], "CITATION");

    let with_bib = seq_path("bi-bib", "docx");
    apply(
        &with_cite,
        &with_bib,
        json!([{ "op": "insertBibliography", "id": "docx:document" }]),
    );
    let tree = inspect(&with_bib, true, true, true, true);
    assert_eq!(tree["sourceCount"], 1);
    assert_eq!(tree["citationCount"], 1);
    assert_eq!(tree["bibliographyCount"], 1);
    assert_eq!(tree["contentControlCount"], 0);
    assert_eq!(
        object_keys(&tree["bibliographies"][0]),
        ["id", "index", "field", "instr", "host", "end", "text"]
    );
    assert_eq!(tree["bibliographies"][0]["id"], "biblio:1");
    assert!(tree["bibliographies"][0]["instr"]
        .as_str()
        .unwrap()
        .to_ascii_uppercase()
        .contains("BIBLIOGRAPHY"));
    assert_eq!(tree["paragraphs"][0]["text"], "第一章");
    let fld_codes: Vec<&str> = tree["fields"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["code"].as_str().unwrap())
        .collect();
    assert!(fld_codes.contains(&"CITATION"));
    assert!(fld_codes.contains(&"BIBLIOGRAPHY"));

    let gone_bib = seq_path("bi-del-bib", "docx");
    apply(
        &with_bib,
        &gone_bib,
        json!([{ "op": "deleteBibliography", "id": "biblio:1" }]),
    );
    let tree_db = inspect(&gone_bib, true, true, true, true);
    assert_eq!(tree_db["bibliographyCount"], 0);
    assert_eq!(tree_db["citationCount"], 1);
    assert_eq!(tree_db["sourceCount"], 1);
    assert_eq!(tree_db["paragraphs"][0]["text"], "第一章");

    let gone_cite = seq_path("bi-del-cite", "docx");
    apply(
        &gone_bib,
        &gone_cite,
        json!([{ "op": "deleteCitation", "id": "cite:1" }]),
    );
    let tree_dc = inspect(&gone_cite, true, true, true, true);
    assert_eq!(tree_dc["citationCount"], 0);
    assert_eq!(tree_dc["sourceCount"], 1);
    assert_eq!(tree_dc["paragraphs"][1]["text"], "hello");

    let gone = seq_path("bi-del-src", "docx");
    apply(
        &gone_cite,
        &gone,
        json!([{ "op": "deleteSource", "id": "source:1" }]),
    );
    let tree2 = inspect(&gone, true, true, true, true);
    assert_eq!(tree2["sourceCount"], 0);
    assert_eq!(tree2["citationCount"], 0);
    assert_eq!(tree2["bibliographyCount"], 0);
    assert_eq!(tree2["paragraphs"][0]["text"], "第一章");

    let hf = tree0["headers"][0]["paragraphs"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let err_hf = apply_err(&src, json!([{ "op": "insertBibliography", "id": hf }]));
    assert_eq!(err_hf.code(), ErrorCode::InvalidId);

    let err_tag = apply_err(
        &src,
        json!([{
            "op": "insertSource",
            "id": "docx:document",
            "tag": "  ",
            "title": "Office Open XML"
        }]),
    );
    assert_eq!(err_tag.code(), ErrorCode::InvalidField);

    let err_type = apply_err(
        &src,
        json!([{
            "op": "insertSource",
            "id": "docx:document",
            "tag": "Book01",
            "sourceType": "NotAType",
            "title": "Office Open XML"
        }]),
    );
    assert_eq!(err_type.code(), ErrorCode::InvalidField);

    let err_dup = apply_err(
        &with_src,
        json!([{
            "op": "insertSource",
            "id": "docx:document",
            "tag": "Book01",
            "title": "Another"
        }]),
    );
    assert_eq!(err_dup.code(), ErrorCode::InvalidField);

    let p1 = first_body_para_id(&tree0);
    let err_cite = apply_err(
        &src,
        json!([{ "op": "insertCitation", "id": p1, "tag": "Book01" }]),
    );
    assert_eq!(err_cite.code(), ErrorCode::IdNotFound);

    let err_del = apply_err(&src, json!([{ "op": "deleteSource", "id": "source:1" }]));
    assert_eq!(err_del.code(), ErrorCode::IdNotFound);

    let err_id = apply_err(&src, json!([{ "op": "deleteSource", "id": "cite:1" }]));
    assert_eq!(err_id.code(), ErrorCode::InvalidId);
}

#[test]
fn biblio_ops_not_for_xlsx_or_pptx() {
    let xlsx = seq_path("bi-x", "xlsx");
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
    let xout = seq_path("bi-x-out", "xlsx");
    let err_x = run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": xlsx.to_string_lossy(),
            "output": xout.to_string_lossy(),
            "ops": [{ "op": "insertBibliography", "id": "docx:document" }]
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err_x.code(), ErrorCode::InvalidField);
    assert!(!xout.exists());

    let pptx = seq_path("bi-p", "pptx");
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
    let pout = seq_path("bi-p-out", "pptx");
    let err_p = run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": pptx.to_string_lossy(),
            "output": pout.to_string_lossy(),
            "ops": [{ "op": "insertSource", "id": "docx:document", "tag": "Book01", "title": "t" }]
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err_p.code(), ErrorCode::InvalidField);
}

#[test]
fn header_has_no_biblio_counts() {
    let src = create_docx(
        "bi-hf",
        json!({
            "paragraphs": [{ "text": "body" }],
            "headers": [{ "type": "default", "paragraphs": [{ "text": "页眉" }] }]
        }),
    );
    let tree = inspect(&src, true, true, true, true);
    assert!(tree["headers"][0].get("sourceCount").is_none());
    assert!(tree["headers"][0].get("citationCount").is_none());
    assert!(tree["headers"][0].get("bibliographyCount").is_none());
}
