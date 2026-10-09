//! 备注页与幻灯片母版 / 版式上的对象：复用已有 op，只扩合法 id 宿主。
//! 根上 Count 仍只计幻灯片 XML。`notesMaster` / `handout` / 批注内除图以外仍拒绝。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};

const PNG_1X1: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
    0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08, 0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00,
    0x00, 0x03, 0x01, 0x01, 0x00, 0x18, 0xDD, 0x8D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E,
    0x44, 0xAE, 0x42, 0x60, 0x82,
];

/// 只含 `ftyp` 的最小 MP4，库按文件头认，不解码。
const MP4_FTYP: &[u8] = &[
    0x00, 0x00, 0x00, 0x18, b'f', b't', b'y', b'p', b'i', b's', b'o', b'm', 0x00, 0x00, 0x00, 0x00,
    b'i', b's', b'o', b'm', b'm', b'p', b'4', b'1',
];

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-pptx-notes-master-objects");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn tmp(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}"))
}

fn write_png(tag: &str) -> PathBuf {
    let path = tmp(tag).with_extension("png");
    std::fs::write(&path, PNG_1X1).unwrap();
    path
}

fn write_mp4(tag: &str) -> PathBuf {
    let path = tmp(tag).with_extension("mp4");
    std::fs::write(&path, MP4_FTYP).unwrap();
    path
}

fn create_deck(tag: &str) -> PathBuf {
    let path = tmp(tag).with_extension("pptx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "presentation": { "slides": [{ "shapes": [{ "text": "标题" }] }] }
        })
        .to_string(),
        None,
    )
    .unwrap();
    path
}

fn create_xlsx(tag: &str) -> PathBuf {
    let path = tmp(tag).with_extension("xlsx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "workbook": {
                "sheets": [{
                    "name": "Sheet1",
                    "cells": [{ "ref": "A1", "value": "嵌入" }]
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

fn object_keys(v: &Value) -> Vec<&str> {
    v.as_object().unwrap().keys().map(String::as_str).collect()
}

fn notes_body_id(path: &Path) -> String {
    let tree = inspect(path, json!({ "notes": true }));
    tree["slides"][0]["notes"]["shapes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["text"] == "备注正文")
        .expect("应有备注正文框")["id"]
        .as_str()
        .unwrap()
        .to_string()
}

fn first_id_starting(list: &Value, prefix: &str) -> String {
    list.as_array()
        .unwrap()
        .iter()
        .find_map(|o| {
            o["id"]
                .as_str()
                .filter(|s| s.starts_with(prefix))
                .map(str::to_string)
        })
        .unwrap_or_else(|| panic!("应有以 {prefix} 开头的 id"))
}

#[test]
fn notes_video_lists_and_skips_root_count() {
    let src = create_deck("nv");
    let mp4 = write_mp4("clip");
    let out = tmp("nv").with_extension("pptx");
    apply(
        &src,
        &out,
        json!([{
            "op": "insertVideo",
            "id": "slide:1/notes",
            "file": mp4.to_string_lossy()
        }]),
    )
    .unwrap();
    let tree = inspect(&out, json!({ "notes": true, "media": true }));
    assert_eq!(tree["videoCount"], 0);
    assert_eq!(tree["slides"][0]["videoCount"], 0);
    let notes = &tree["slides"][0]["notes"];
    assert_eq!(notes["videoCount"], 1);
    let id = notes["videos"][0]["id"].as_str().unwrap();
    assert!(id.starts_with("slide:1/notes/video:"), "{id}");
    let exported = tmp("nv-exp").with_extension("mp4");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "exportMedia",
            "path": out.to_string_lossy(),
            "id": id,
            "output": exported.to_string_lossy()
        })
        .to_string(),
        None,
    )
    .unwrap();
    assert_eq!(std::fs::read(&exported).unwrap(), MP4_FTYP);
}

#[test]
fn notes_hyperlink_and_field() {
    let src = create_deck("nl");
    let noted = tmp("noted").with_extension("pptx");
    apply(
        &src,
        &noted,
        json!([{ "op": "setNotesText", "id": "slide:1", "text": "备注正文" }]),
    )
    .unwrap();
    let body = notes_body_id(&noted);
    let linked = tmp("nl").with_extension("pptx");
    apply(
        &noted,
        &linked,
        json!([{
            "op": "insertHyperlink",
            "id": body,
            "url": "https://example.com/"
        }]),
    )
    .unwrap();
    let tree = inspect(
        &linked,
        json!({ "notes": true, "hyperlinks": true, "fields": true }),
    );
    assert_eq!(tree["hyperlinkCount"], 0);
    let notes = &tree["slides"][0]["notes"];
    assert_eq!(notes["hyperlinkCount"], 1);
    assert_eq!(notes["hyperlinks"][0]["id"], "slide:1/notes/link:1");
    assert_eq!(notes["hyperlinks"][0]["url"], "https://example.com/");

    let with_fld = tmp("nf").with_extension("pptx");
    apply(
        &linked,
        &with_fld,
        json!([{
            "op": "insertField",
            "id": notes_body_id(&linked),
            "type": "slidenum"
        }]),
    )
    .unwrap();
    let tree = inspect(&with_fld, json!({ "notes": true, "fields": true }));
    assert_eq!(tree["fieldCount"], 0);
    let notes = &tree["slides"][0]["notes"];
    assert!(
        notes["fieldCount"].as_u64().unwrap() >= 2,
        "备注页自带页码域，再插一条后至少 2：{}",
        notes["fieldCount"]
    );
    let fields = notes["fields"].as_array().unwrap();
    assert!(
        fields
            .iter()
            .any(|f| f["id"].as_str().unwrap().starts_with("slide:1/notes/fld:")),
        "{fields:?}"
    );
}

#[test]
fn notes_ole_and_connector() {
    let src = create_deck("no");
    let xlsx = create_xlsx("embed");
    let out = tmp("no").with_extension("pptx");
    apply(
        &src,
        &out,
        json!([
            { "op": "insertOle", "id": "slide:1/notes", "file": xlsx.to_string_lossy() },
            { "op": "insertConnector", "id": "slide:1/notes" }
        ]),
    )
    .unwrap();
    let tree = inspect(
        &out,
        json!({ "notes": true, "oleObjects": true, "connectors": true }),
    );
    assert_eq!(tree["oleObjectCount"], 0);
    assert_eq!(tree["connectorCount"], 0);
    let notes = &tree["slides"][0]["notes"];
    assert_eq!(notes["oleObjectCount"], 1);
    assert_eq!(notes["connectorCount"], 1);
    let oid = notes["oleObjects"][0]["id"].as_str().unwrap();
    assert!(oid.starts_with("slide:1/notes/ole:"), "{oid}");
    let cid = notes["connectors"][0]["id"].as_str().unwrap();
    assert!(cid.starts_with("slide:1/notes/cxn:"), "{cid}");
}

#[test]
fn master_picture_and_table() {
    let src = create_deck("mp");
    let png = write_png("pic");
    let out = tmp("mp").with_extension("pptx");
    apply(
        &src,
        &out,
        json!([
            {
                "op": "insertPicture",
                "id": "master:1",
                "file": png.to_string_lossy()
            },
            {
                "op": "insertTable",
                "id": "master:1",
                "rows": 1,
                "cols": 1,
                "cells": [["母版表"]]
            }
        ]),
    )
    .unwrap();
    let bare = inspect(&out, json!({ "masters": true }));
    assert_eq!(
        object_keys(&bare["masters"][0]),
        [
            "id",
            "index",
            "name",
            "theme",
            "transition",
            "animCount",
            "placeholders",
            "shapes",
            "layouts"
        ]
    );
    assert_eq!(bare["pictureCount"], 0);
    assert_eq!(bare["tableCount"], 0);

    let tree = inspect(
        &out,
        json!({ "masters": true, "pictures": true, "tables": true }),
    );
    assert_eq!(tree["pictureCount"], 0);
    assert_eq!(tree["tableCount"], 0);
    let m = &tree["masters"][0];
    assert_eq!(
        object_keys(m),
        [
            "id",
            "index",
            "name",
            "theme",
            "transition",
            "animCount",
            "placeholders",
            "shapes",
            "pictures",
            "tables",
            "layouts"
        ]
    );
    let pid = first_id_starting(&m["pictures"], "master:1/pic:");
    let tid = first_id_starting(&m["tables"], "master:1/table:");
    assert_eq!(m["tables"][0]["rows"][0]["cells"][0]["text"], "母版表");
    let exported = tmp("mp-exp").with_extension("png");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "exportMedia",
            "path": out.to_string_lossy(),
            "id": pid,
            "output": exported.to_string_lossy()
        })
        .to_string(),
        None,
    )
    .unwrap();
    assert_eq!(std::fs::read(&exported).unwrap(), PNG_1X1);
    let _ = tid;
}

#[test]
fn layout_picture_and_master_math() {
    let src = create_deck("lp");
    let png = write_png("lay");
    let out = tmp("lp").with_extension("pptx");
    apply(
        &src,
        &out,
        json!([
            {
                "op": "insertPicture",
                "id": "master:1/layout:1",
                "file": png.to_string_lossy()
            },
            { "op": "insertMath", "id": "master:1", "text": "E=mc^2" }
        ]),
    )
    .unwrap();
    let tree = inspect(
        &out,
        json!({ "masters": true, "pictures": true, "maths": true }),
    );
    assert_eq!(tree["pictureCount"], 0);
    assert_eq!(tree["mathCount"], 0);
    let layout = &tree["masters"][0]["layouts"][0];
    let _ = first_id_starting(&layout["pictures"], "master:1/layout:1/pic:");
    let mid = first_id_starting(&tree["masters"][0]["maths"], "master:1/math:");
    assert_eq!(tree["masters"][0]["maths"][0]["text"], "E=mc^2");
    let _ = mid;
}

#[test]
fn master_video_skips_root_count() {
    let src = create_deck("mv");
    let mp4 = write_mp4("mclip");
    let out = tmp("mv").with_extension("pptx");
    apply(
        &src,
        &out,
        json!([{
            "op": "insertVideo",
            "id": "master:1",
            "file": mp4.to_string_lossy()
        }]),
    )
    .unwrap();
    let tree = inspect(&out, json!({ "masters": true, "media": true }));
    assert_eq!(tree["videoCount"], 0);
    let vid = first_id_starting(&tree["masters"][0]["videos"], "master:1/video:");
    let _ = vid;
}

#[test]
fn rejects_notes_master_handout_comment_and_group() {
    let src = create_deck("rej");
    let png = write_png("bad");
    let mp4 = write_mp4("bad");
    let dest = tmp("nm-pic").with_extension("pptx");
    let err = apply(
        &src,
        &dest,
        json!([{
            "op": "insertPicture",
            "id": "notesMaster",
            "file": png.to_string_lossy()
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    assert!(!dest.exists());

    let dest = tmp("ho-pic").with_extension("pptx");
    let err = apply(
        &src,
        &dest,
        json!([{
            "op": "insertPicture",
            "id": "handout",
            "file": png.to_string_lossy()
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    assert!(!dest.exists());

    let dest = tmp("cm-vid").with_extension("pptx");
    let err = apply(
        &src,
        &dest,
        json!([{
            "op": "insertVideo",
            "id": "slide:1/comment:00000000-0000-0000-0000-000000000001",
            "file": mp4.to_string_lossy()
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    assert!(!dest.exists());

    let dest = tmp("grp-notes").with_extension("pptx");
    let err = apply(
        &src,
        &dest,
        json!([{
            "op": "groupShapes",
            "id": "slide:1/notes",
            "ids": ["slide:1/notes/shape:3", "slide:1/notes/shape:4"]
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    assert!(!dest.exists());
}
