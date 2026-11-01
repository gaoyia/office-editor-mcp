//! docx 索引：insertIndex / insertIndexEntry、inspect 计数、xlsx/pptx 拒绝。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-docx-index");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn seq_path(tag: &str, ext: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}.{ext}"))
}

fn create_docx(tag: &str, document: Value) -> PathBuf {
    let path = seq_path(tag, "docx");
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "format": "docx",
        "document": document
    })
    .to_string())
    .unwrap();
    path
}

fn inspect(path: &Path, indexes: bool, entries: bool, fields: bool) -> Value {
    let RunOutput::Inspect(tree) = run(&json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": path.to_string_lossy(),
        "scope": {
            "from": 1,
            "maxParas": 20,
            "indexes": indexes,
            "indexEntries": entries,
            "fields": fields,
            "headerFooter": true
        }
    })
    .to_string())
    .unwrap() else {
        panic!("inspect 应返回树");
    };
    tree
}

fn inspect_bare(path: &Path) -> Value {
    let RunOutput::Inspect(tree) = run(&json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": path.to_string_lossy()
    })
    .to_string())
    .unwrap() else {
        panic!("inspect 应返回树");
    };
    tree
}

fn apply(path: &Path, out: &Path, ops: Value) {
    run(&json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": path.to_string_lossy(),
        "output": out.to_string_lossy(),
        "ops": ops
    })
    .to_string())
    .unwrap();
}

fn apply_err(path: &Path, ops: Value) -> ooxp::Error {
    let out = seq_path("should-not-write", "docx");
    let _ = std::fs::remove_file(&out);
    run(&json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": path.to_string_lossy(),
        "output": out.to_string_lossy(),
        "ops": ops
    })
    .to_string())
    .unwrap_err()
}

fn first_body_para_id(tree: &Value) -> String {
    tree["paragraphs"][0]["id"].as_str().unwrap().to_string()
}

fn object_keys(v: &Value) -> Vec<&str> {
    v.as_object().unwrap().keys().map(|k| k.as_str()).collect()
}

#[test]
fn insert_index_and_entry_then_delete() {
    let src = create_docx(
        "ix-src",
        json!({
            "paragraphs": [
                { "style": "Heading1", "text": "第一章" },
                { "text": "the quick brown fox" }
            ],
            "headers": [{ "type": "default", "paragraphs": [{ "text": "页眉" }] }]
        }),
    );
    let bare = inspect_bare(&src);
    assert_eq!(bare["indexCount"], 0);
    assert_eq!(bare["indexEntryCount"], 0);
    assert!(bare.get("indexes").is_none());
    assert!(bare.get("indexEntries").is_none());

    let tree0 = inspect(&src, false, false, false);
    assert_eq!(tree0["indexCount"], 0);
    assert!(tree0.get("indexes").is_none());
    let p1 = first_body_para_id(&tree0);
    let p2 = tree0["paragraphs"][1]["id"].as_str().unwrap().to_string();

    let with_xe = seq_path("ix-xe", "docx");
    apply(
        &src,
        &with_xe,
        json!([{ "op": "insertIndexEntry", "id": p2, "entry": "fox" }]),
    );
    let tree_xe = inspect(&with_xe, false, true, true);
    assert_eq!(tree_xe["indexEntryCount"], 1);
    assert_eq!(tree_xe["indexCount"], 0);
    assert_eq!(
        object_keys(&tree_xe["indexEntries"][0]),
        ["id", "index", "field", "instr", "entry", "host", "end", "text"]
    );
    assert_eq!(tree_xe["indexEntries"][0]["id"], "xe:1");
    assert_eq!(tree_xe["indexEntries"][0]["entry"], "fox");
    assert_eq!(tree_xe["fields"][0]["code"], "XE");
    assert!(!tree_xe["paragraphs"][1]["text"]
        .as_str()
        .unwrap()
        .contains("XE"));

    let with_ix = seq_path("ix-ix", "docx");
    apply(
        &with_xe,
        &with_ix,
        json!([{ "op": "insertIndex", "id": "docx:document" }]),
    );
    let tree = inspect(&with_ix, true, true, true);
    assert_eq!(tree["indexCount"], 1);
    assert_eq!(tree["indexEntryCount"], 1);
    assert_eq!(
        object_keys(&tree["indexes"][0]),
        ["id", "index", "field", "instr", "host", "end", "text"]
    );
    assert_eq!(tree["indexes"][0]["id"], "index:1");
    assert!(tree["indexes"][0]["instr"]
        .as_str()
        .unwrap()
        .to_ascii_uppercase()
        .contains("INDEX"));
    assert_eq!(tree["paragraphs"][0]["text"], "第一章");

    let gone_ix = seq_path("ix-del-ix", "docx");
    apply(
        &with_ix,
        &gone_ix,
        json!([{ "op": "deleteIndex", "id": "index:1" }]),
    );
    let tree_dx = inspect(&gone_ix, true, true, true);
    assert_eq!(tree_dx["indexCount"], 0);
    assert_eq!(tree_dx["indexEntryCount"], 1);
    assert_eq!(tree_dx["paragraphs"][0]["text"], "第一章");

    let gone = seq_path("ix-del-xe", "docx");
    apply(
        &gone_ix,
        &gone,
        json!([{ "op": "deleteIndexEntry", "id": "xe:1" }]),
    );
    let tree2 = inspect(&gone, true, true, true);
    assert_eq!(tree2["indexEntryCount"], 0);
    assert_eq!(tree2["indexCount"], 0);
    assert_eq!(tree2["paragraphs"][0]["text"], "第一章");
    assert_eq!(tree2["paragraphs"][1]["text"], "the quick brown fox");

    let hf = tree0["headers"][0]["paragraphs"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let before = std::fs::read(&src).unwrap();
    let err_hf = apply_err(&src, json!([{ "op": "insertIndex", "id": hf }]));
    assert_eq!(err_hf.code(), ErrorCode::InvalidId);
    assert_eq!(std::fs::read(&src).unwrap(), before);

    let err_entry = apply_err(
        &src,
        json!([{ "op": "insertIndexEntry", "id": p1, "entry": "fox\"bad" }]),
    );
    assert_eq!(err_entry.code(), ErrorCode::InvalidField);

    let err_empty = apply_err(
        &src,
        json!([{ "op": "insertIndexEntry", "id": p1, "entry": "  " }]),
    );
    assert_eq!(err_empty.code(), ErrorCode::InvalidField);

    let err_instr = apply_err(
        &src,
        json!([{ "op": "insertIndex", "id": p1, "instr": " TOC \\o \"1-3\"" }]),
    );
    assert_eq!(err_instr.code(), ErrorCode::InvalidField);

    let err_del = apply_err(&src, json!([{ "op": "deleteIndex", "id": "index:1" }]));
    assert_eq!(err_del.code(), ErrorCode::IdNotFound);

    let err_id = apply_err(&src, json!([{ "op": "deleteIndex", "id": "field:1" }]));
    assert_eq!(err_id.code(), ErrorCode::InvalidId);
}

#[test]
fn index_ops_not_for_xlsx_or_pptx() {
    let xlsx = seq_path("ix-x", "xlsx");
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": xlsx.to_string_lossy(),
        "workbook": { "sheets": [{ "name": "Sheet1" }] }
    })
    .to_string())
    .unwrap();
    let xout = seq_path("ix-x-out", "xlsx");
    let err_x = run(&json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": xlsx.to_string_lossy(),
        "output": xout.to_string_lossy(),
        "ops": [{ "op": "insertIndex", "id": "docx:document" }]
    })
    .to_string())
    .unwrap_err();
    assert_eq!(err_x.code(), ErrorCode::InvalidField);
    assert!(!xout.exists());

    let pptx = seq_path("ix-p", "pptx");
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": pptx.to_string_lossy(),
        "presentation": { "slides": [{ "shapes": [{ "text": "a" }] }] }
    })
    .to_string())
    .unwrap();
    let pout = seq_path("ix-p-out", "pptx");
    let err_p = run(&json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": pptx.to_string_lossy(),
        "output": pout.to_string_lossy(),
        "ops": [{ "op": "insertIndexEntry", "id": "para:1", "entry": "fox" }]
    })
    .to_string())
    .unwrap_err();
    assert_eq!(err_p.code(), ErrorCode::InvalidField);
}

#[test]
fn index_in_a_header_or_text_box_is_rejected_but_an_old_one_can_be_deleted() {
    let src = create_docx(
        "ix-reject",
        json!({
            "paragraphs": [{ "text": "正文" }],
            "headers": [{ "type": "default", "paragraphs": [{ "text": "页眉" }] }]
        }),
    );
    let tree = inspect(&src, true, false, false);
    let body = tree["paragraphs"][0]["id"].as_str().unwrap();
    let header = tree["headers"][0]["paragraphs"][0]["id"].as_str().unwrap();
    let boxed = seq_path("ix-box", "docx");
    apply(
        &src,
        &boxed,
        json!([{ "op": "insertTextBox", "id": body, "text": "框" }]),
    );
    let inner = run(&json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": boxed.to_string_lossy(),
        "scope": { "from": 1, "maxParas": 4, "textBoxes": true, "textBoxContent": true, "headerFooter": true }
    }).to_string()).unwrap();
    let RunOutput::Inspect(inner) = inner else {
        panic!("inspect");
    };
    let tx = inner["paragraphs"][0]["textBoxes"][0]["paragraphs"][0]["id"]
        .as_str()
        .unwrap();
    let before = std::fs::read(&boxed).unwrap();
    assert_eq!(
        apply_err(&boxed, json!([{ "op": "insertIndex", "id": header }])).code(),
        ErrorCode::InvalidId
    );
    assert_eq!(
        apply_err(&boxed, json!([{ "op": "insertIndex", "id": tx }])).code(),
        ErrorCode::InvalidId
    );
    assert_eq!(std::fs::read(&boxed).unwrap(), before);

    let old = seq_path("ix-old", "docx");
    inject_header_index(&src, &old);
    let listed = inspect(&old, true, false, false);
    assert_eq!(listed["indexCount"], 0);
    assert_eq!(listed["headers"][0]["indexCount"], 1);
    assert_eq!(
        listed["headers"][0]["indexes"][0]["id"],
        "header:default:1/index:1"
    );
    let deleted = seq_path("ix-old-del", "docx");
    apply(
        &old,
        &deleted,
        json!([{ "op": "deleteIndex", "id": "header:default:1/index:1" }]),
    );
    let after = inspect(&deleted, true, false, false);
    assert_eq!(after["headers"][0]["indexCount"], 0);
}

fn inject_header_index(src: &Path, dest: &Path) {
    use std::io::{Cursor, Read, Write};
    use zip::write::SimpleFileOptions;
    use zip::{CompressionMethod, ZipArchive, ZipWriter};
    let bytes = std::fs::read(src).unwrap();
    let mut zin = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut buf = Cursor::new(Vec::new());
    {
        let mut zout = ZipWriter::new(&mut buf);
        let opt = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        for i in 0..zin.len() {
            let mut file = zin.by_index(i).unwrap();
            let name = file.name().to_string();
            let mut data = Vec::new();
            file.read_to_end(&mut data).unwrap();
            if name == "word/header1.xml" {
                let xml = String::from_utf8(data).unwrap();
                let start = xml.find("<w:r>").expect("header run");
                let end = xml.find("</w:p>").expect("header para");
                let field = r#"<w:r><w:fldChar w:fldCharType="begin"/></w:r><w:r><w:instrText xml:space="preserve"> INDEX \h "A" \c "1" </w:instrText></w:r><w:r><w:fldChar w:fldCharType="separate"/></w:r><w:r><w:t>1</w:t></w:r><w:r><w:fldChar w:fldCharType="end"/></w:r>"#;
                data = format!("{}{field}{}", &xml[..start], &xml[end..]).into_bytes();
            }
            zout.start_file(name, opt).unwrap();
            zout.write_all(&data).unwrap();
        }
        zout.finish().unwrap();
    }
    std::fs::write(dest, buf.into_inner()).unwrap();
}

#[test]
fn header_index_counts_start_at_zero() {
    let src = create_docx(
        "ix-hf",
        json!({
            "paragraphs": [{ "text": "body" }],
            "headers": [{ "type": "default", "paragraphs": [{ "text": "页眉" }] }]
        }),
    );
    let tree = inspect(&src, true, true, true);
    assert_eq!(tree["headers"][0]["indexCount"], 0);
    assert_eq!(tree["headers"][0]["indexEntryCount"], 0);
}
