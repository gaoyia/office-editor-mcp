//! 文本框、脚注、尾注、批注里的图表与 SmartArt；注内 / 批注内不可 insertTextBox。

use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::ZipArchive;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-docx-container-drawing");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn seq_path(tag: &str, ext: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}.{ext}"))
}

fn create_docx(tag: &str) -> PathBuf {
    let path = seq_path(tag, "docx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "format": "docx",
            "document": {
                "paragraphs": [
                    { "style": "Heading1", "text": "第一章" },
                    { "text": "hello" }
                ]
            }
        })
        .to_string(),
        None,
    )
    .unwrap();
    path
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

fn part_text(path: &Path, name: &str) -> String {
    let bytes = std::fs::read(path).unwrap();
    let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut file = zip.by_name(name.trim_start_matches('/')).unwrap();
    let mut s = String::new();
    file.read_to_string(&mut s).unwrap();
    s
}

fn col_chart() -> Value {
    json!({
        "op": "insertChart",
        "type": "col",
        "title": "销量",
        "series": [{
            "name": "系列甲",
            "categories": ["甲", "乙"],
            "values": [1, 2]
        }]
    })
}

#[test]
fn insert_chart_in_comment_not_counted_on_root() {
    let src = create_docx("cmt-host");
    let tree0 = inspect(&src, json!({ "from": 1, "maxParas": 5 }));
    let para = tree0["paragraphs"][0]["id"].as_str().unwrap();
    let with_cmt = seq_path("cmt", "docx");
    apply(
        &src,
        &with_cmt,
        json!([{ "op": "insertComment", "start": para, "end": para, "text": "批注甲" }]),
    );
    let listed = inspect(&with_cmt, json!({ "comments": true }));
    let cmt_para = listed["comments"][0]["paragraphs"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let root_before = listed["chartCount"].as_u64().unwrap();

    let with_chart = seq_path("cmt-chart", "docx");
    let mut op = col_chart();
    op["id"] = json!(cmt_para);
    apply(&with_cmt, &with_chart, json!([op]));

    let tree = inspect(&with_chart, json!({ "comments": true, "charts": true }));
    assert_eq!(tree["chartCount"], root_before);
    assert_eq!(tree["comments"][0]["chartCount"], 1);
    let chart_id = tree["comments"][0]["charts"][0]["id"].as_str().unwrap();
    assert!(
        chart_id.starts_with("comment:"),
        "图表 id 应为批注前缀，实际 {chart_id}"
    );
    assert!(chart_id.contains("/chart:"));
    assert_eq!(tree["comments"][0]["charts"][0]["type"], "col");
    assert!(part_text(&with_chart, "word/_rels/comments.xml.rels").contains("/chart"));
    assert!(part_text(&with_chart, "[Content_Types].xml").contains("chart+xml"));

    let xlsx = seq_path("cmt-export", "xlsx");
    let RunOutput::Written { .. } = run(
        &json!({
            "schemaVersion": 1,
            "action": "exportChartData",
            "path": with_chart.to_string_lossy(),
            "id": chart_id,
            "output": xlsx.to_string_lossy()
        })
        .to_string(),
        None,
    )
    .unwrap() else {
        panic!("exportChartData 应写出文件");
    };
    assert!(xlsx.is_file());
}

#[test]
fn insert_smartart_in_comment() {
    let src = create_docx("sa-host");
    let tree0 = inspect(&src, json!({ "from": 1, "maxParas": 5 }));
    let para = tree0["paragraphs"][0]["id"].as_str().unwrap();
    let with_cmt = seq_path("sa-cmt", "docx");
    apply(
        &src,
        &with_cmt,
        json!([{ "op": "insertComment", "start": para, "end": para, "text": "批注甲" }]),
    );
    let listed = inspect(&with_cmt, json!({ "comments": true }));
    let cmt_para = listed["comments"][0]["paragraphs"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let root_before = listed["smartArtCount"].as_u64().unwrap();
    let out = seq_path("sa-out", "docx");
    apply(
        &with_cmt,
        &out,
        json!([{
            "op": "insertSmartArt",
            "id": cmt_para,
            "layout": "list",
            "points": [{ "text": "一" }, { "text": "二" }]
        }]),
    );
    let tree = inspect(&out, json!({ "comments": true, "smartArt": true }));
    assert_eq!(tree["smartArtCount"], root_before);
    assert_eq!(tree["comments"][0]["smartArtCount"], 1);
    let id = tree["comments"][0]["smartArts"][0]["id"].as_str().unwrap();
    assert!(id.starts_with("comment:"));
    assert!(id.contains("/smartdiagram:"));
}

#[test]
fn insert_textbox_in_note_or_comment_is_invalid_field() {
    let src = create_docx("fn-host");
    let tree0 = inspect(&src, json!({ "from": 1, "maxParas": 5 }));
    let para = tree0["paragraphs"][1]["id"].as_str().unwrap();
    let with_fn = seq_path("fn", "docx");
    apply(
        &src,
        &with_fn,
        json!([{ "op": "insertFootnote", "id": para, "text": "脚注原文" }]),
    );
    let listed = inspect(&with_fn, json!({ "notes": true }));
    let fn_para = listed["footnotes"][0]["paragraphs"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let err = apply_err(
        &with_fn,
        json!([{ "op": "insertTextBox", "id": fn_para, "text": "注内框" }]),
    );
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let tree_h = inspect(&src, json!({ "from": 1, "maxParas": 5 }));
    let host = tree_h["paragraphs"][0]["id"].as_str().unwrap();
    let with_cmt = seq_path("cmt-txbx", "docx");
    apply(
        &src,
        &with_cmt,
        json!([{ "op": "insertComment", "start": host, "end": host, "text": "批注甲" }]),
    );
    let listed_c = inspect(&with_cmt, json!({ "comments": true }));
    let cmt_para = listed_c["comments"][0]["paragraphs"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let err = apply_err(
        &with_cmt,
        json!([{ "op": "insertTextBox", "id": cmt_para, "text": "批注内框" }]),
    );
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

#[test]
fn insert_chart_in_body_textbox_not_counted_on_root() {
    let src = create_docx("txbx-host");
    let tree0 = inspect(&src, json!({ "from": 1, "maxParas": 5 }));
    let para = tree0["paragraphs"][1]["id"].as_str().unwrap();
    let with_box = seq_path("txbx", "docx");
    apply(
        &src,
        &with_box,
        json!([{ "op": "insertTextBox", "id": para, "text": "框内字" }]),
    );
    let listed = inspect(
        &with_box,
        json!({
            "from": 1,
            "maxParas": 10,
            "textBoxes": true,
            "textBoxContent": true
        }),
    );
    let inner = listed["paragraphs"][1]["textBoxes"][0]["paragraphs"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let root_before = listed["chartCount"].as_u64().unwrap();
    let out = seq_path("txbx-chart", "docx");
    let mut op = col_chart();
    op["id"] = json!(inner);
    apply(&with_box, &out, json!([op]));
    let tree = inspect(
        &out,
        json!({
            "from": 1,
            "maxParas": 10,
            "textBoxes": true,
            "textBoxContent": true,
            "charts": true
        }),
    );
    assert_eq!(tree["chartCount"], root_before);
    assert_eq!(tree["charts"].as_array().map(Vec::len).unwrap_or(0), 0);
    let box_node = &tree["paragraphs"][1]["textBoxes"][0];
    assert_eq!(box_node["chartCount"], 1);
    let chart_id = box_node["charts"][0]["id"].as_str().unwrap();
    assert!(chart_id.contains("/txbx:"));
    assert!(chart_id.contains("/chart:"));
}

#[test]
fn container_ids_rejected() {
    let src = create_docx("bad-id");
    let tree0 = inspect(&src, json!({ "from": 1, "maxParas": 5 }));
    let para = tree0["paragraphs"][0]["id"].as_str().unwrap();
    let with_cmt = seq_path("bad-cmt", "docx");
    apply(
        &src,
        &with_cmt,
        json!([{ "op": "insertComment", "start": para, "end": para, "text": "批注甲" }]),
    );
    let mut chart = col_chart();
    chart["id"] = json!("comment:0");
    let err = apply_err(&with_cmt, json!([chart]));
    assert_eq!(err.code(), ErrorCode::InvalidId);

    let with_box = seq_path("bad-txbx", "docx");
    apply(
        &src,
        &with_box,
        json!([{ "op": "insertTextBox", "id": para, "text": "框" }]),
    );
    let listed = inspect(
        &with_box,
        json!({ "from": 1, "maxParas": 5, "textBoxes": true }),
    );
    let box_id = listed["paragraphs"][0]["textBoxes"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let err = apply_err(
        &with_box,
        json!([{ "op": "insertTextBox", "id": box_id, "text": "套框" }]),
    );
    assert_eq!(err.code(), ErrorCode::InvalidId);
}
