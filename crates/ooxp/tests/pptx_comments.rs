//! 幻灯片现代批注：名单、插删改、回复、完成标记、跨格式拒绝。

use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-pptx-comments");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn tmp(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}"))
}

fn create_deck(tag: &str) -> PathBuf {
    let path = tmp(tag).with_extension("pptx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "presentation": {
                "slides": [
                    { "shapes": [{ "text": "标题" }, { "text": "第二框" }] },
                    { "shapes": [{ "text": "第二页" }] }
                ]
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
    let bytes = std::fs::read(path).unwrap();
    let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut file = zip.by_name(name).unwrap();
    let mut out = Vec::new();
    file.read_to_end(&mut out).unwrap();
    String::from_utf8(out).unwrap()
}

fn part_names(path: &Path) -> Vec<String> {
    let bytes = std::fs::read(path).unwrap();
    let zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    zip.file_names().map(str::to_string).collect()
}

fn rewrite_part(path: &Path, name: &str, f: impl FnOnce(Vec<u8>) -> Vec<u8>) {
    let bytes = std::fs::read(path).unwrap();
    let mut zip_in = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut out = Cursor::new(Vec::new());
    let mut transform = Some(f);
    {
        let mut zip_out = ZipWriter::new(&mut out);
        let opt = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        for i in 0..zip_in.len() {
            let mut file = zip_in.by_index(i).unwrap();
            let n = file.name().to_string();
            let mut data = Vec::new();
            file.read_to_end(&mut data).unwrap();
            if n == name {
                data = transform.take().expect("部件只应出现一次")(data);
            }
            zip_out.start_file(&n, opt).unwrap();
            zip_out.write_all(&data).unwrap();
        }
        zip_out.finish().unwrap();
    }
    std::fs::write(path, out.into_inner()).unwrap();
}

fn insert_one(src: &Path, out: &Path, text: &str) {
    apply(
        src,
        out,
        json!([{
            "op": "insertComment",
            "id": "slide:1",
            "text": text,
            "author": "Alice",
            "initials": "A",
            "date": "2026-09-01T00:00:00.000",
            "x": 914400,
            "y": 914400
        }]),
    )
    .unwrap();
}

#[test]
fn inspect_comment_count_without_list() {
    let path = create_deck("bare");
    let tree = inspect_bare(&path);
    assert_eq!(tree["commentCount"], 0);
    assert_eq!(tree["slides"][0]["commentCount"], 0);
    assert!(tree["slides"][0].get("comments").is_none());
    assert_eq!(
        object_keys(&tree),
        [
            "schemaVersion",
            "format",
            "id",
            "slideCount",
            "pictureCount",
            "videoCount",
            "audioCount",
            "chartCount",
            "smartArtCount",
            "commentCount",
            "hyperlinkCount",
            "tableCount",
            "animCount",
            "masterCount",
            "hasHandout",
            "sectionCount",
            "props",
            "slides"
        ]
    );
    let expanded = inspect_comments(&path);
    assert_eq!(expanded["slides"][0]["comments"], json!([]));
    assert_eq!(expanded["slides"][1]["comments"], json!([]));
}

#[test]
fn insert_comment_inspects_and_writes_modern_parts() {
    let src = create_deck("insert");
    let out = tmp("inserted").with_extension("pptx");
    insert_one(&src, &out, "批注甲");
    let names = part_names(&out);
    assert!(names
        .iter()
        .any(|n| n.starts_with("ppt/comments/modernComment_")));
    assert!(names.iter().any(|n| n == "ppt/authors.xml"));
    let rels = part_text(&out, "ppt/slides/_rels/slide1.xml.rels");
    assert!(rels.contains("office/2018/10/relationships/comments"));
    assert!(!rels.contains("officeDocument/2006/relationships/comments"));
    let pres_rels = part_text(&out, "ppt/_rels/presentation.xml.rels");
    assert!(pres_rels.contains("office/2018/10/relationships/authors"));
    let slide = part_text(&out, "ppt/slides/slide1.xml");
    assert!(slide.contains("6950BFC3-D8DA-4A85-94F7-54DA5524770B"));
    assert!(slide.contains("commentRel"));

    let tree = inspect_comments(&out);
    assert_eq!(tree["commentCount"], 1);
    assert_eq!(tree["slides"][0]["commentCount"], 1);
    assert_eq!(tree["slides"][1]["commentCount"], 0);
    let c = &tree["slides"][0]["comments"][0];
    let id = c["id"].as_str().unwrap();
    assert!(id.starts_with("slide:1/comment:{"));
    assert!(id.ends_with('}'));
    assert_eq!(
        c["commentId"].as_str().unwrap(),
        &id["slide:1/comment:".len()..]
    );
    assert_eq!(c["author"], "Alice");
    assert_eq!(c["initials"], "A");
    assert_eq!(c["date"], "2026-09-01T00:00:00.000");
    assert_eq!(c["done"], false);
    assert_eq!(c["kind"], "normal");
    assert_eq!(c["x"], 914400);
    assert_eq!(c["y"], 914400);
    assert_eq!(c["text"], "批注甲");
    assert_eq!(c["replies"], json!([]));
    assert_eq!(
        object_keys(c),
        [
            "id",
            "commentId",
            "author",
            "initials",
            "date",
            "done",
            "kind",
            "x",
            "y",
            "pictureCount",
            "text",
            "replies"
        ]
    );
}

#[test]
fn two_comments_share_one_part() {
    let src = create_deck("two");
    let mid = tmp("mid").with_extension("pptx");
    insert_one(&src, &mid, "批注甲");
    let out = tmp("two").with_extension("pptx");
    apply(
        &mid,
        &out,
        json!([{
            "op": "insertComment",
            "id": "slide:1",
            "text": "批注乙",
            "author": "Bob",
            "initials": "B"
        }]),
    )
    .unwrap();
    let names: Vec<_> = part_names(&out)
        .into_iter()
        .filter(|n| n.starts_with("ppt/comments/"))
        .collect();
    assert_eq!(names.len(), 1);
    let tree = inspect_comments(&out);
    assert_eq!(tree["commentCount"], 2);
    assert_eq!(tree["slides"][0]["comments"][0]["text"], "批注甲");
    assert_eq!(tree["slides"][0]["comments"][1]["text"], "批注乙");
    assert_eq!(tree["slides"][0]["comments"][1]["author"], "Bob");
}

#[test]
fn reply_text_meta_done_and_delete() {
    let src = create_deck("thread");
    let inserted = tmp("ins").with_extension("pptx");
    insert_one(&src, &inserted, "批注甲");
    let tree0 = inspect_comments(&inserted);
    let cid = tree0["slides"][0]["comments"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();

    let replied = tmp("replied").with_extension("pptx");
    apply(
        &inserted,
        &replied,
        json!([{
            "op": "replyComment",
            "id": cid,
            "text": "回复乙",
            "author": "Bob",
            "initials": "B",
            "date": "2026-09-01T01:00:00.000"
        }]),
    )
    .unwrap();
    let tree_r = inspect_comments(&replied);
    let replies = tree_r["slides"][0]["comments"][0]["replies"]
        .as_array()
        .unwrap();
    assert_eq!(replies.len(), 1);
    let rid = replies[0]["id"].as_str().unwrap().to_string();
    assert!(rid.starts_with(&format!("{cid}/reply:")));
    assert_eq!(replies[0]["text"], "回复乙");
    assert_eq!(replies[0]["author"], "Bob");
    assert_eq!(replies[0]["done"], false);
    assert_eq!(tree_r["commentCount"], 1);

    let texted = tmp("texted").with_extension("pptx");
    apply(
        &replied,
        &texted,
        json!([{ "op": "setCommentText", "id": cid, "text": "改过的批注" }]),
    )
    .unwrap();
    let tree_t = inspect_comments(&texted);
    assert_eq!(tree_t["slides"][0]["comments"][0]["text"], "改过的批注");
    assert_eq!(
        tree_t["slides"][0]["comments"][0]["replies"][0]["text"],
        "回复乙"
    );

    let meta = tmp("meta").with_extension("pptx");
    apply(
        &texted,
        &meta,
        json!([{ "op": "setCommentMeta", "id": cid, "author": "Carol", "initials": "C" }]),
    )
    .unwrap();
    let tree_m = inspect_comments(&meta);
    assert_eq!(tree_m["slides"][0]["comments"][0]["author"], "Carol");
    assert_eq!(tree_m["slides"][0]["comments"][0]["initials"], "C");
    assert_eq!(
        tree_m["slides"][0]["comments"][0]["date"],
        "2026-09-01T00:00:00.000"
    );
    assert_eq!(
        tree_m["slides"][0]["comments"][0]["replies"][0]["author"],
        "Bob"
    );

    let done = tmp("done").with_extension("pptx");
    apply(
        &meta,
        &done,
        json!([{ "op": "setCommentDone", "id": cid, "done": true }]),
    )
    .unwrap();
    let tree_d = inspect_comments(&done);
    assert_eq!(tree_d["slides"][0]["comments"][0]["done"], true);
    assert_eq!(
        tree_d["slides"][0]["comments"][0]["replies"][0]["done"],
        false
    );
    let xml = part_text(
        &done,
        part_names(&done)
            .iter()
            .find(|n| n.starts_with("ppt/comments/"))
            .unwrap(),
    );
    assert!(xml.contains("status=\"resolved\""));

    let undone = tmp("undone").with_extension("pptx");
    apply(
        &done,
        &undone,
        json!([{ "op": "setCommentDone", "id": cid, "done": false }]),
    )
    .unwrap();
    let tree_u = inspect_comments(&undone);
    assert_eq!(tree_u["slides"][0]["comments"][0]["done"], false);

    let del_reply = tmp("del-reply").with_extension("pptx");
    apply(
        &undone,
        &del_reply,
        json!([{ "op": "deleteComment", "id": rid }]),
    )
    .unwrap();
    let tree_dr = inspect_comments(&del_reply);
    assert_eq!(
        tree_dr["slides"][0]["comments"][0]["replies"]
            .as_array()
            .unwrap()
            .len(),
        0
    );

    let replied2 = tmp("replied2").with_extension("pptx");
    apply(
        &del_reply,
        &replied2,
        json!([{ "op": "replyComment", "id": cid, "text": "再回复" }]),
    )
    .unwrap();
    let deleted = tmp("deleted").with_extension("pptx");
    apply(
        &replied2,
        &deleted,
        json!([{ "op": "deleteComment", "id": cid }]),
    )
    .unwrap();
    let tree_del = inspect_comments(&deleted);
    assert_eq!(tree_del["commentCount"], 0);
    assert_eq!(tree_del["slides"][0]["comments"], json!([]));
    let names = part_names(&deleted);
    assert!(names.iter().any(|n| n.starts_with("ppt/comments/")));
    assert!(names.iter().any(|n| n == "ppt/authors.xml"));
}

#[test]
fn reject_notes_id_and_foreign_ops() {
    let src = create_deck("reject");
    let err = apply(
        &src,
        &tmp("notes").with_extension("pptx"),
        json!([{ "op": "insertComment", "id": "slide:1/notes", "text": "x" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);

    let err = apply(
        &src,
        &tmp("shape").with_extension("pptx"),
        json!([{ "op": "insertComment", "id": "slide:1/shape:2", "text": "x" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);

    let err = apply(
        &src,
        &tmp("pic").with_extension("pptx"),
        json!([{
            "op": "insertPicture",
            "id": "slide:1/comment:{00000000-0000-4000-8000-000000000001}",
            "file": "nope.png"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);

    let xlsx = tmp("book").with_extension("xlsx");
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
    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": xlsx.to_string_lossy(),
            "output": tmp("xlsx").with_extension("xlsx").to_string_lossy(),
            "ops": [{ "op": "insertComment", "id": "slide:1", "text": "x" }]
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
}

#[test]
fn reply_id_must_be_parent_comment() {
    let src = create_deck("reply-id");
    let inserted = tmp("ins").with_extension("pptx");
    insert_one(&src, &inserted, "批注甲");
    let tree = inspect_comments(&inserted);
    let cid = tree["slides"][0]["comments"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let replied = tmp("r").with_extension("pptx");
    apply(
        &inserted,
        &replied,
        json!([{ "op": "replyComment", "id": cid, "text": "回" }]),
    )
    .unwrap();
    let rid = inspect_comments(&replied)["slides"][0]["comments"][0]["replies"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let err = apply(
        &replied,
        &tmp("nested").with_extension("pptx"),
        json!([{ "op": "replyComment", "id": rid, "text": "嵌套" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
}

#[test]
fn ink_allows_meta_and_done_rejects_text() {
    let src = create_deck("ink0");
    let with = tmp("ink-in").with_extension("pptx");
    insert_one(&src, &with, "墨迹批");
    let comments = part_names(&with)
        .into_iter()
        .find(|n| n.contains("/comments/") && n.ends_with(".xml"))
        .expect("应有批注部件");
    rewrite_part(&with, &comments, |data| {
        String::from_utf8(data)
            .unwrap()
            .replace("</p188:txBody>", "</p188:txBody><ink/>")
            .into_bytes()
    });
    let tree = inspect_comments(&with);
    let art = &tree["slides"][0]["comments"][0];
    assert_eq!(art["kind"], "ink");
    let cid = art["id"].as_str().unwrap().to_string();

    let err = apply(
        &with,
        &tmp("ink-text").with_extension("pptx"),
        json!([{ "op": "setCommentText", "id": cid, "text": "x" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::NotImplemented);
    let err = apply(
        &with,
        &tmp("ink-reply").with_extension("pptx"),
        json!([{ "op": "replyComment", "id": cid, "text": "x" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::NotImplemented);

    let meta = tmp("ink-meta").with_extension("pptx");
    apply(
        &with,
        &meta,
        json!([
            { "op": "setCommentMeta", "id": cid, "author": "Ink2", "initials": "K" },
            { "op": "setCommentDone", "id": cid, "done": true }
        ]),
    )
    .unwrap();
    let tree = inspect_comments(&meta);
    let art = &tree["slides"][0]["comments"][0];
    assert_eq!(art["kind"], "ink");
    assert_eq!(art["author"], "Ink2");
    assert_eq!(art["initials"], "K");
    assert_eq!(art["done"], true);
}
