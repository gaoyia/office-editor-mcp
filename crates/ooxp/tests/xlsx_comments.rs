//! 工作表线程化批注：名单、插删改、回复、完成标记、跨格式拒绝。

use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-xlsx-comments");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn tmp(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}.xlsx"))
}

fn create_book(tag: &str) -> PathBuf {
    let path = tmp(tag);
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "workbook": {
                "sheets": [{
                    "name": "Sheet1",
                    "cells": [{ "ref": "A1", "value": 1 }]
                }]
            }
        })
        .to_string(),
        None,
    )
    .unwrap();
    path
}

fn apply(src: &Path, out: &Path, ops: Value) -> Result<RunOutput, ooxp::Error> {
    run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": src.to_string_lossy(),
            "output": out.to_string_lossy(),
            "ops": ops,
        })
        .to_string(),
        None,
    )
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

fn inspect_comments(path: &Path) -> Value {
    let RunOutput::Inspect(tree) = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": path.to_string_lossy(),
            "scope": { "comments": true }
        })
        .to_string(),
        None,
    )
    .unwrap() else {
        panic!("inspect 应返回树");
    };
    tree
}

fn object_keys(v: &Value) -> Vec<&str> {
    v.as_object().unwrap().keys().map(String::as_str).collect()
}

fn part_text(path: &Path, name: &str) -> String {
    let f = std::fs::File::open(path).unwrap();
    let mut zip = zip::ZipArchive::new(f).unwrap();
    let mut zf = zip.by_name(name).unwrap();
    let mut out = String::new();
    zf.read_to_string(&mut out).unwrap();
    out
}

fn part_names(path: &Path) -> Vec<String> {
    let f = std::fs::File::open(path).unwrap();
    let zip = zip::ZipArchive::new(f).unwrap();
    zip.file_names().map(str::to_string).collect()
}

fn insert_one(src: &Path, out: &Path, text: &str) {
    apply(
        src,
        out,
        json!([{
            "op": "insertComment",
            "id": "cell:Sheet1!A1",
            "text": text,
            "author": "Alice",
            "initials": "A",
            "date": "2026-09-01T00:00:00.000"
        }]),
    )
    .unwrap();
}

#[test]
fn inspect_comment_count_without_list() {
    let path = create_book("bare");
    let tree = inspect_bare(&path);
    assert_eq!(tree["commentCount"], 0);
    assert_eq!(tree["sheets"][0]["commentCount"], 0);
    assert!(tree["sheets"][0].get("comments").is_none());
}

#[test]
fn insert_list_reply_text_meta_done_delete() {
    let src = create_book("round");
    let inserted = tmp("ins");
    insert_one(&src, &inserted, "批注甲");
    let names = part_names(&inserted);
    assert!(names.iter().any(|n| n.starts_with("xl/threadedComments/")));
    assert!(names.iter().any(|n| n == "xl/persons/person.xml"));
    assert!(names.iter().any(|n| n.starts_with("xl/comments")));
    assert!(names.iter().any(|n| n.contains("vmlDrawing")));
    let sheet = part_text(&inserted, "xl/worksheets/sheet1.xml");
    assert!(sheet.contains("legacyDrawing"));
    let tc = part_text(&inserted, "xl/threadedComments/threadedComment1.xml");
    assert!(tc.contains("批注甲"));
    assert!(
        tc.contains("providerId") || part_text(&inserted, "xl/persons/person.xml").contains("None")
    );
    let people = part_text(&inserted, "xl/persons/person.xml");
    assert!(people.contains("providerId=\"None\""));
    assert!(people.contains("userId=\"Name\""));
    assert!(people.contains("Alice"));

    let tree = inspect_comments(&inserted);
    assert_eq!(tree["commentCount"], 1);
    assert_eq!(tree["sheets"][0]["commentCount"], 1);
    let comments = tree["sheets"][0]["comments"].as_array().unwrap();
    assert_eq!(comments.len(), 1);
    assert_eq!(
        object_keys(&comments[0]),
        [
            "id",
            "commentId",
            "author",
            "initials",
            "date",
            "done",
            "kind",
            "ref",
            "text",
            "replies"
        ]
    );
    assert_eq!(comments[0]["author"], "Alice");
    assert!(comments[0]["initials"].is_null());
    assert_eq!(comments[0]["kind"], "normal");
    assert_eq!(comments[0]["ref"], "A1");
    assert_eq!(comments[0]["text"], "批注甲");
    assert_eq!(comments[0]["done"], false);
    let cid = comments[0]["id"].as_str().unwrap().to_string();
    assert!(cid.starts_with("cell:Sheet1!A1/comment:{"));
    assert!(cid.contains('}'));

    let replied = tmp("reply");
    apply(
        &inserted,
        &replied,
        json!([{
            "op": "replyComment",
            "id": cid,
            "text": "回复一",
            "author": "Bob",
            "date": "2026-09-01T01:00:00.000"
        }]),
    )
    .unwrap();
    let tree_r = inspect_comments(&replied);
    let replies = tree_r["sheets"][0]["comments"][0]["replies"]
        .as_array()
        .unwrap();
    assert_eq!(replies.len(), 1);
    assert_eq!(replies[0]["text"], "回复一");
    assert_eq!(replies[0]["author"], "Bob");
    assert!(replies[0]["id"].as_str().unwrap().contains("/reply:{"));
    assert_eq!(tree_r["commentCount"], 1);
    let rid = replies[0]["id"].as_str().unwrap().to_string();

    let texted = tmp("text");
    apply(
        &replied,
        &texted,
        json!([{ "op": "setCommentText", "id": cid, "text": "改后的甲" }]),
    )
    .unwrap();
    let tree_t = inspect_comments(&texted);
    assert_eq!(tree_t["sheets"][0]["comments"][0]["text"], "改后的甲");

    let meta = tmp("meta");
    apply(
        &texted,
        &meta,
        json!([{ "op": "setCommentMeta", "id": cid, "author": "Carol" }]),
    )
    .unwrap();
    let tree_m = inspect_comments(&meta);
    assert_eq!(tree_m["sheets"][0]["comments"][0]["author"], "Carol");
    assert_eq!(
        tree_m["sheets"][0]["comments"][0]["replies"][0]["author"],
        "Bob"
    );

    let done = tmp("done");
    apply(
        &meta,
        &done,
        json!([{ "op": "setCommentDone", "id": cid, "done": true }]),
    )
    .unwrap();
    let tree_d = inspect_comments(&done);
    assert_eq!(tree_d["sheets"][0]["comments"][0]["done"], true);
    let undone = tmp("undone");
    apply(
        &done,
        &undone,
        json!([{ "op": "setCommentDone", "id": cid, "done": false }]),
    )
    .unwrap();
    let tree_u = inspect_comments(&undone);
    assert_eq!(tree_u["sheets"][0]["comments"][0]["done"], false);

    let del_reply = tmp("delr");
    apply(
        &undone,
        &del_reply,
        json!([{ "op": "deleteComment", "id": rid }]),
    )
    .unwrap();
    let tree_dr = inspect_comments(&del_reply);
    assert_eq!(
        tree_dr["sheets"][0]["comments"][0]["replies"]
            .as_array()
            .unwrap()
            .len(),
        0
    );
    assert_eq!(tree_dr["commentCount"], 1);

    let deleted = tmp("del");
    apply(
        &del_reply,
        &deleted,
        json!([{ "op": "deleteComment", "id": cid }]),
    )
    .unwrap();
    let tree_del = inspect_comments(&deleted);
    assert_eq!(tree_del["commentCount"], 0);
    assert_eq!(tree_del["sheets"][0]["comments"], json!([]));
    let names = part_names(&deleted);
    assert!(names.iter().any(|n| n.starts_with("xl/threadedComments/")));
    assert!(names.iter().any(|n| n == "xl/persons/person.xml"));
}

#[test]
fn reject_bad_id_and_start_end() {
    let src = create_book("reject");
    let err = apply(
        &src,
        &tmp("slide"),
        json!([{ "op": "insertComment", "id": "slide:1", "text": "x" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);

    let err = apply(
        &src,
        &tmp("sheet"),
        json!([{ "op": "insertComment", "id": "sheet:Sheet1", "text": "x" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);

    let err = apply(
        &src,
        &tmp("start"),
        json!([{
            "op": "insertComment",
            "id": "cell:Sheet1!A1",
            "start": "para:1",
            "text": "x"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let err = apply(
        &src,
        &tmp("xy"),
        json!([{
            "op": "insertComment",
            "id": "cell:Sheet1!A1",
            "x": 1,
            "y": 1,
            "text": "x"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let err = apply(
        &src,
        &tmp("meta"),
        json!([{ "op": "setCommentMeta", "id": "cell:Sheet1!A1/comment:{00000000-0000-4000-8000-000000000001}" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let inserted = tmp("ins");
    insert_one(&src, &inserted, "甲");
    let tree = inspect_comments(&inserted);
    let cid = tree["sheets"][0]["comments"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let err = apply(
        &inserted,
        &tmp("reply-cell"),
        json!([{ "op": "replyComment", "id": "cell:Sheet1!A1", "text": "x" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);

    apply(
        &inserted,
        &tmp("ok-reply"),
        json!([{ "op": "replyComment", "id": cid, "text": "r" }]),
    )
    .unwrap();
}

#[test]
fn comments_scope_does_not_need_cells() {
    let src = create_book("scope");
    let inserted = tmp("ins");
    insert_one(&src, &inserted, "只开 comments");
    let tree = inspect_comments(&inserted);
    assert!(tree["sheets"][0].get("cells").is_none());
    assert_eq!(tree["sheets"][0]["comments"][0]["text"], "只开 comments");
}
