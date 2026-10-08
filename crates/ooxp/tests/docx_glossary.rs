//! docx 构建基块：insertDocPart / insertDocPartContent / deleteDocPart、inspect 计数、xlsx/pptx 拒绝。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-docx-glossary");
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

fn inspect(path: &Path, doc_parts: bool) -> Value {
    let RunOutput::Inspect(tree) = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": path.to_string_lossy(),
            "scope": {
                "from": 1,
                "maxParas": 20,
                "docParts": doc_parts,
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

fn object_keys(v: &Value) -> Vec<&str> {
    v.as_object().unwrap().keys().map(|k| k.as_str()).collect()
}

fn part_exists(path: &Path, name: &str) -> bool {
    let f = std::fs::File::open(path).unwrap();
    let mut zip = zip::ZipArchive::new(f).unwrap();
    let ok = zip.by_name(name).is_ok();
    ok
}

fn part_text(path: &Path, name: &str) -> String {
    let f = std::fs::File::open(path).unwrap();
    let mut zip = zip::ZipArchive::new(f).unwrap();
    let mut zf = zip.by_name(name).unwrap();
    let mut s = String::new();
    std::io::Read::read_to_string(&mut zf, &mut s).unwrap();
    s
}

fn insert_op() -> Value {
    json!({
        "op": "insertDocPart",
        "id": "docx:document",
        "name": "Clause1",
        "text": "glossary clause"
    })
}

#[test]
fn insert_doc_part_content_then_delete() {
    let src = create_docx(
        "gl-src",
        json!({
            "paragraphs": [
                { "style": "Heading1", "text": "第一章" },
                { "text": "hello" }
            ],
            "headers": [{ "type": "default", "paragraphs": [{ "text": "页眉" }] }]
        }),
    );
    let bare = inspect_bare(&src);
    assert_eq!(bare["docPartCount"], 0);
    assert!(bare.get("docParts").is_none());
    let keys = object_keys(&bare);
    let bib = keys.iter().position(|k| *k == "bibliographyCount").unwrap();
    let dp = keys.iter().position(|k| *k == "docPartCount").unwrap();
    let math = keys.iter().position(|k| *k == "mathCount").unwrap();
    assert!(bib < dp && dp < math, "{keys:?}");

    let with_part = seq_path("gl-insert", "docx");
    apply(&src, &with_part, json!([insert_op()]));
    let tree = inspect(&with_part, true);
    assert_eq!(tree["docPartCount"], 1);
    assert_eq!(tree["docParts"].as_array().unwrap().len(), 1);
    let item = &tree["docParts"][0];
    assert_eq!(
        object_keys(item),
        ["id", "index", "name", "gallery", "category", "type", "behavior", "guid", "text"]
    );
    assert_eq!(item["id"], "docPart:1");
    assert_eq!(item["index"], 1);
    assert_eq!(item["name"], "Clause1");
    assert_eq!(item["gallery"], "custQuickParts");
    assert_eq!(item["category"], "General");
    assert!(item["type"].is_null());
    assert_eq!(item["behavior"], "content");
    let guid = item["guid"].as_str().unwrap();
    assert!(
        guid.starts_with('{') && guid.ends_with('}') && guid.len() == 38,
        "{guid}"
    );
    assert_eq!(item["text"], "glossary clause");
    assert!(part_exists(&with_part, "word/glossary/document.xml"));
    assert!(part_exists(&with_part, "word/glossary/styles.xml"));
    assert!(part_exists(&with_part, "word/glossary/settings.xml"));
    assert!(part_exists(&with_part, "word/glossary/webSettings.xml"));
    assert!(part_exists(&with_part, "word/glossary/fontTable.xml"));
    assert!(part_exists(
        &with_part,
        "word/glossary/_rels/document.xml.rels"
    ));
    let rels = part_text(&with_part, "word/_rels/document.xml.rels");
    assert!(rels.contains("/glossaryDocument"), "{rels}");
    let xml = part_text(&with_part, "word/glossary/document.xml");
    assert!(xml.contains("w:val=\"Clause1\""), "{xml}");
    assert!(xml.contains("w:val=\"custQuickParts\""), "{xml}");
    assert!(xml.contains("mc:Ignorable=\"w14\""), "{xml}");

    let with_body = seq_path("gl-content", "docx");
    apply(
        &with_part,
        &with_body,
        json!([{
            "op": "insertDocPartContent",
            "id": "docx:document",
            "docPart": "docPart:1"
        }]),
    );
    let tree_c = inspect(&with_body, true);
    assert_eq!(tree_c["docPartCount"], 1);
    let texts: Vec<&str> = tree_c["paragraphs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["text"].as_str().unwrap())
        .collect();
    assert!(texts.contains(&"glossary clause"), "{texts:?}");
    assert!(texts.contains(&"hello"), "{texts:?}");

    let gone = seq_path("gl-del", "docx");
    apply(
        &with_body,
        &gone,
        json!([{ "op": "deleteDocPart", "id": "docPart:1" }]),
    );
    let tree2 = inspect(&gone, true);
    assert_eq!(tree2["docPartCount"], 0);
    assert_eq!(tree2["docParts"].as_array().unwrap().len(), 0);
    assert!(!part_exists(&gone, "word/glossary/document.xml"));
    assert!(!part_exists(&gone, "word/glossary/styles.xml"));
    assert!(!part_exists(&gone, "word/glossary/settings.xml"));
    assert!(!part_exists(&gone, "word/glossary/webSettings.xml"));
    assert!(!part_exists(&gone, "word/glossary/fontTable.xml"));
    assert!(!part_exists(&gone, "word/glossary/_rels/document.xml.rels"));
    let rels2 = part_text(&gone, "word/_rels/document.xml.rels");
    assert!(!rels2.contains("/glossaryDocument"), "{rels2}");

    let tree0 = inspect(&src, false);
    let hf = tree0["headers"][0]["paragraphs"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let err_hf = apply_err(
        &src,
        json!([{
            "op": "insertDocPartContent",
            "id": hf,
            "docPart": "docPart:1"
        }]),
    );
    assert_eq!(err_hf.code(), ErrorCode::InvalidId);

    let err_name = apply_err(
        &src,
        json!([{
            "op": "insertDocPart",
            "id": "docx:document",
            "name": "  ",
            "text": "glossary clause"
        }]),
    );
    assert_eq!(err_name.code(), ErrorCode::InvalidField);

    let err_text = apply_err(
        &src,
        json!([{
            "op": "insertDocPart",
            "id": "docx:document",
            "name": "Clause1",
            "text": ""
        }]),
    );
    assert_eq!(err_text.code(), ErrorCode::InvalidField);

    let err_gallery = apply_err(
        &src,
        json!([{
            "op": "insertDocPart",
            "id": "docx:document",
            "name": "Clause1",
            "text": "glossary clause",
            "gallery": "general"
        }]),
    );
    assert_eq!(err_gallery.code(), ErrorCode::InvalidField);

    let err_dup = apply_err(&with_part, json!([insert_op()]));
    assert_eq!(err_dup.code(), ErrorCode::InvalidField);

    let err_id = apply_err(
        &src,
        json!([{
            "op": "insertDocPart",
            "id": tree0["paragraphs"][0]["id"].as_str().unwrap(),
            "name": "Clause1",
            "text": "glossary clause"
        }]),
    );
    assert_eq!(err_id.code(), ErrorCode::InvalidId);

    let err_del = apply_err(&src, json!([{ "op": "deleteDocPart", "id": "docPart:1" }]));
    assert_eq!(err_del.code(), ErrorCode::IdNotFound);

    let err_prefix = apply_err(&src, json!([{ "op": "deleteDocPart", "id": "source:1" }]));
    assert_eq!(err_prefix.code(), ErrorCode::InvalidId);

    let err_content = apply_err(
        &src,
        json!([{
            "op": "insertDocPartContent",
            "id": "docx:document",
            "docPart": "docPart:1"
        }]),
    );
    assert_eq!(err_content.code(), ErrorCode::IdNotFound);
}

#[test]
fn glossary_ops_not_for_xlsx_or_pptx() {
    let xlsx = seq_path("gl-x", "xlsx");
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
    let xout = seq_path("gl-x-out", "xlsx");
    let err_x = run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": xlsx.to_string_lossy(),
            "output": xout.to_string_lossy(),
            "ops": [{
                "op": "insertDocPart",
                "id": "docx:document",
                "name": "Clause1",
                "text": "glossary clause"
            }]
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err_x.code(), ErrorCode::InvalidField);
    assert!(!xout.exists());

    let err_scope = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": xlsx.to_string_lossy(),
            "scope": { "docParts": true }
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err_scope.code(), ErrorCode::InvalidField);

    let pptx = seq_path("gl-p", "pptx");
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
    let pout = seq_path("gl-p-out", "pptx");
    let err_p = run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": pptx.to_string_lossy(),
            "output": pout.to_string_lossy(),
            "ops": [{
                "op": "insertDocPart",
                "id": "docx:document",
                "name": "Clause1",
                "text": "glossary clause"
            }]
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err_p.code(), ErrorCode::InvalidField);
}

#[test]
fn header_has_no_doc_part_count() {
    let src = create_docx(
        "gl-hf",
        json!({
            "paragraphs": [{ "text": "body" }],
            "headers": [{ "type": "default", "paragraphs": [{ "text": "页眉" }] }]
        }),
    );
    let tree = inspect(&src, true);
    assert!(tree["headers"][0].get("docPartCount").is_none());
    assert!(tree["headers"][0].get("docParts").is_none());
}
