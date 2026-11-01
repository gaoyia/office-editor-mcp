//! docx 文本框里的链、书签、域、公式、拼音与控件：inspect 计数与名单、apply 插改、拒写注内。

use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-docx-box-fields");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn tmp(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}.docx"))
}

fn create_doc(tag: &str, document: Value) -> PathBuf {
    let path = tmp(tag);
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

fn inspect(path: &Path, scope: Value) -> Value {
    let RunOutput::Inspect(tree) = run(&json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": path.to_string_lossy(),
        "scope": scope
    })
    .to_string())
    .unwrap() else {
        panic!("inspect");
    };
    tree
}

fn apply(src: &Path, out: &Path, ops: Value) {
    run(&json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": out.to_string_lossy(),
        "ops": ops
    })
    .to_string())
    .unwrap();
}

fn apply_err(src: &Path, ops: Value) -> ooxp::Error {
    let out = tmp("should-not");
    run(&json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": out.to_string_lossy(),
        "ops": ops
    })
    .to_string())
    .unwrap_err()
}

fn object_keys(v: &Value) -> Vec<&str> {
    v.as_object().unwrap().keys().map(String::as_str).collect()
}

fn box_node(tree: &Value) -> &Value {
    &tree["paragraphs"][0]["textBoxes"][0]
}

fn unzip_document(path: &Path) -> String {
    let file = std::fs::File::open(path).unwrap();
    let mut zip = zip::ZipArchive::new(file).unwrap();
    let mut part = zip.by_name("word/document.xml").unwrap();
    let mut s = String::new();
    part.read_to_string(&mut s).unwrap();
    s
}

fn listed_scope() -> Value {
    json!({
        "from": 1,
        "maxParas": 10,
        "textBoxes": true,
        "textBoxContent": true,
        "hyperlinks": true,
        "bookmarks": true,
        "fields": true,
        "maths": true,
        "rubies": true,
        "contentControls": true
    })
}

#[test]
fn box_fields_insert_inspect_and_refuse() {
    let src = create_doc(
        "src",
        json!({
            "paragraphs": [{
                "text": "外",
                "textBoxes": [{ "text": "框" }]
            }]
        }),
    );
    let tree0 = inspect(
        &src,
        json!({ "from": 1, "maxParas": 10, "textBoxes": true }),
    );
    assert_eq!(
        object_keys(box_node(&tree0)),
        [
            "id",
            "docPrId",
            "name",
            "kind",
            "cx",
            "cy",
            "wrap",
            "textAnchor",
            "link",
            "paraCount",
            "pictureCount",
            "chartCount",
            "smartArtCount",
            "tableCount",
            "hyperlinkCount",
            "bookmarkCount",
            "fieldCount",
            "tocCount",
            "captionCount",
            "crossRefCount",
            "indexCount",
            "indexEntryCount",
            "citationCount",
            "bibliographyCount",
            "mathCount",
            "rubyCount",
            "contentControlCount",
            "stable",
            "hyperlink"
        ]
    );
    for k in [
        "hyperlinkCount",
        "bookmarkCount",
        "fieldCount",
        "mathCount",
        "rubyCount",
        "contentControlCount",
    ] {
        assert_eq!(box_node(&tree0)[k], 0);
    }
    assert!(box_node(&tree0).get("hyperlinks").is_none());
    let box_id = box_node(&tree0)["id"].as_str().unwrap().to_string();
    let root_links = tree0["hyperlinkCount"].as_u64().unwrap();
    let root_bm = tree0["bookmarkCount"].as_u64().unwrap();
    let root_fields = tree0["fieldCount"].as_u64().unwrap();
    let root_math = tree0["mathCount"].as_u64().unwrap();
    let root_ruby = tree0["rubyCount"].as_u64().unwrap();
    let root_sdt = tree0["contentControlCount"].as_u64().unwrap();

    let with_content = inspect(
        &src,
        json!({
            "from": 1,
            "maxParas": 10,
            "textBoxes": true,
            "textBoxContent": true
        }),
    );
    let inner = box_node(&with_content)["paragraphs"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(inner.starts_with(&format!("{box_id}/para:")));

    let linked = tmp("link");
    apply(
        &src,
        &linked,
        json!([{
            "op": "insertHyperlink",
            "id": inner,
            "text": "例",
            "url": "https://example.com"
        }]),
    );
    let tree_l = inspect(&linked, listed_scope());
    let b = box_node(&tree_l);
    assert_eq!(b["hyperlinkCount"], 1);
    assert_eq!(tree_l["hyperlinkCount"], root_links);
    assert_eq!(b["hyperlinks"][0]["id"], format!("{box_id}/link:1"));
    assert_eq!(b["hyperlinks"][0]["url"], "https://example.com");
    assert_eq!(b["hyperlinks"][0]["host"], inner);
    let xml = unzip_document(&linked);
    assert!(xml.contains("w:hyperlink"), "{xml}");
    assert!(xml.contains("txbxContent"), "{xml}");

    let set_l = tmp("set-link");
    apply(
        &linked,
        &set_l,
        json!([{
            "op": "setHyperlink",
            "id": format!("{box_id}/link:1"),
            "url": "https://example.org"
        }]),
    );
    let tree_s = inspect(&set_l, listed_scope());
    assert_eq!(
        box_node(&tree_s)["hyperlinks"][0]["url"],
        "https://example.org"
    );

    let with_rest = tmp("rest");
    apply(
        &set_l,
        &with_rest,
        json!([
            { "op": "insertBookmark", "id": inner, "name": "BoxBm" },
            { "op": "insertField", "id": inner, "code": "DATE" },
            { "op": "insertMath", "id": inner, "text": "x" },
            { "op": "insertRuby", "id": inner, "base": "汉", "rt": "hàn" },
            {
                "op": "insertContentControl",
                "id": inner,
                "kind": "plainText",
                "text": "控"
            }
        ]),
    );
    let tree_r = inspect(&with_rest, listed_scope());
    let br = box_node(&tree_r);
    assert_eq!(br["bookmarkCount"], 1);
    assert_eq!(br["bookmarks"][0]["id"], format!("{box_id}/bookmark:1"));
    assert_eq!(br["bookmarks"][0]["name"], "BoxBm");
    assert_eq!(tree_r["bookmarkCount"], root_bm);
    assert_eq!(br["fieldCount"], 1);
    assert_eq!(br["fields"][0]["id"], format!("{box_id}/field:1"));
    assert_eq!(tree_r["fieldCount"], root_fields);
    assert_eq!(br["mathCount"], 1);
    assert_eq!(br["maths"][0]["id"], format!("{box_id}/math:1"));
    assert_eq!(tree_r["mathCount"], root_math);
    assert_eq!(br["rubyCount"], 1);
    assert_eq!(br["rubies"][0]["id"], format!("{box_id}/ruby:1"));
    assert_eq!(tree_r["rubyCount"], root_ruby);
    assert_eq!(br["contentControlCount"], 1);
    assert_eq!(br["contentControls"][0]["id"], format!("{box_id}/sdt:1"));
    assert_eq!(tree_r["contentControlCount"], root_sdt);

    let err_text = apply_err(
        &with_rest,
        json!([{ "op": "setParaText", "id": inner, "text": "改" }]),
    );
    assert_eq!(err_text.code(), ErrorCode::NotImplemented);

    let gone = tmp("del-link");
    apply(
        &linked,
        &gone,
        json!([{ "op": "deleteHyperlink", "id": format!("{box_id}/link:1") }]),
    );
    let tree_d = inspect(&gone, listed_scope());
    assert_eq!(box_node(&tree_d)["hyperlinkCount"], 0);
    assert_eq!(box_node(&tree_d)["hyperlinks"], json!([]));
}

#[test]
fn box_fields_rejects_note_and_keeps_xlsx() {
    let src = create_doc(
        "fn",
        json!({
            "paragraphs": [{
                "text": "正文",
                "footnote": "注文"
            }]
        }),
    );
    let tree = inspect(&src, json!({ "from": 1, "maxParas": 10, "notes": true }));
    let fn_para = tree["footnotes"][0]["paragraphs"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let err = apply_err(
        &src,
        json!([{ "op": "insertField", "id": fn_para, "code": "DATE" }]),
    );
    assert_eq!(err.code(), ErrorCode::InvalidId);

    let xlsx = tmp_dir().join("t.xlsx");
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": xlsx.to_string_lossy(),
        "workbook": { "sheets": [{ "name": "Sheet1" }] }
    })
    .to_string())
    .unwrap();
    let err_x = run(&json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": xlsx.to_string_lossy(),
        "output": tmp_dir().join("t-out.xlsx").to_string_lossy(),
        "ops": [{
            "op": "insertField",
            "id": "para:1/txbx:1/para:1",
            "code": "DATE"
        }]
    })
    .to_string())
    .unwrap_err();
    assert_eq!(err_x.code(), ErrorCode::InvalidField);
}
