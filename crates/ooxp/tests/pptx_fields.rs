//! 幻灯片文本形状里的 `a:fld`：名单、插改删、跨格式拒绝。

use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::ZipArchive;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-pptx-fields");
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
                    { "shapes": [{ "text": "页1" }, { "text": "第二框" }] },
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

fn inspect_fields(path: &Path) -> Value {
    let RunOutput::Inspect(tree) = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": path.to_string_lossy(),
            "scope": { "fields": true }
        })
        .to_string(),
        None,
    )
    .unwrap() else {
        panic!("inspect 应返回树");
    };
    tree
}

fn inspect_shapes(path: &Path) -> Value {
    let RunOutput::Inspect(tree) = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": path.to_string_lossy(),
            "scope": { "slide": 1, "maxShapes": 20 }
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

fn first_shape_id(path: &Path) -> String {
    inspect_shapes(path)["slides"][0]["shapes"][0]["id"]
        .as_str()
        .unwrap()
        .to_string()
}

#[test]
fn bare_inspect_has_zero_field_count() {
    let path = create_deck("bare");
    let tree = inspect_bare(&path);
    assert_eq!(tree["fieldCount"], 0);
    assert_eq!(tree["slides"][0]["fieldCount"], 0);
    assert!(tree["slides"][0].get("fields").is_none());
    assert_eq!(
        object_keys(&tree),
        [
            "schemaVersion",
            "format",
            "id",
            "slideCount",
            "pictureCount",
            "mathCount",
            "videoCount",
            "audioCount",
            "chartCount",
            "smartArtCount",
            "commentCount",
            "hyperlinkCount",
            "fieldCount",
            "tableCount",
            "oleObjectCount",
            "model3dCount",
            "zoomCount",
            "connectorCount",
            "animCount",
            "masterCount",
            "hasHandout",
            "sectionCount",
            "slideSize",
            "customShowCount",
            "showPr",
            "modifyProtection",
            "props",
            "slides"
        ]
    );
    let listed = inspect_fields(&path);
    assert_eq!(listed["slides"][0]["fields"], json!([]));
    assert_eq!(listed["slides"][1]["fields"], json!([]));
}

#[test]
fn insert_slidenum_set_delete_roundtrip() {
    let src = create_deck("round");
    let shape = first_shape_id(&src);
    let inserted = tmp("ins").with_extension("pptx");
    apply(
        &src,
        &inserted,
        json!([{
            "op": "insertField",
            "id": shape,
            "type": "slidenum",
            "text": "1"
        }]),
    )
    .unwrap();

    let tree = inspect_fields(&inserted);
    assert_eq!(tree["fieldCount"], 1);
    assert_eq!(tree["slides"][0]["fieldCount"], 1);
    assert_eq!(tree["slides"][1]["fieldCount"], 0);
    let list = tree["slides"][0]["fields"].as_array().unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(object_keys(&list[0]), ["id", "type", "host", "text"]);
    assert_eq!(list[0]["type"], "slidenum");
    assert_eq!(list[0]["host"], shape);
    assert_eq!(list[0]["text"], "1");
    let fid = list[0]["id"].as_str().unwrap();
    assert!(fid.starts_with("slide:1/fld:{"), "{fid}");
    assert!(fid.ends_with('}'), "{fid}");
    let xml = part_text(&inserted, "ppt/slides/slide1.xml");
    assert!(xml.contains(r#"type="slidenum""#));
    assert!(xml.contains("<a:fld "));
    assert!(!xml.contains("<a:pPr"));
    assert_eq!(
        inspect_shapes(&inserted)["slides"][0]["shapes"][0]["text"],
        "页1"
    );

    let typed = tmp("set").with_extension("pptx");
    apply(
        &inserted,
        &typed,
        json!([{ "op": "setField", "id": fid, "type": "datetime1", "text": "2026/9/3" }]),
    )
    .unwrap();
    let after_set = inspect_fields(&typed);
    assert_eq!(after_set["slides"][0]["fields"][0]["type"], "datetime1");
    assert_eq!(after_set["slides"][0]["fields"][0]["text"], "2026/9/3");
    assert_eq!(after_set["slides"][0]["fields"][0]["id"], fid);
    let xml2 = part_text(&typed, "ppt/slides/slide1.xml");
    assert!(xml2.contains(r#"type="datetime1""#));
    assert!(xml2.contains(fid.trim_start_matches("slide:1/fld:")));

    let deleted = tmp("del").with_extension("pptx");
    apply(
        &typed,
        &deleted,
        json!([{ "op": "deleteField", "id": fid }]),
    )
    .unwrap();
    let tree_del = inspect_fields(&deleted);
    assert_eq!(tree_del["fieldCount"], 0);
    assert_eq!(tree_del["slides"][0]["fields"], json!([]));
    let xml3 = part_text(&deleted, "ppt/slides/slide1.xml");
    assert!(!xml3.contains("<a:fld"));
    assert!(xml3.contains("<a:t>2026/9/3</a:t>"));
}

#[test]
fn delete_keep_instr_writes_type_as_run() {
    let src = create_deck("keep");
    let shape = first_shape_id(&src);
    let inserted = tmp("ins-keep").with_extension("pptx");
    apply(
        &src,
        &inserted,
        json!([{ "op": "insertField", "id": shape, "type": "datetime8" }]),
    )
    .unwrap();
    let fid = inspect_fields(&inserted)["slides"][0]["fields"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let out = tmp("keep-instr").with_extension("pptx");
    apply(
        &inserted,
        &out,
        json!([{ "op": "deleteField", "id": fid, "keep": "instr" }]),
    )
    .unwrap();
    let xml = part_text(&out, "ppt/slides/slide1.xml");
    assert!(!xml.contains("<a:fld"));
    assert!(xml.contains("<a:t>datetime8</a:t>"));
}

#[test]
fn create_slides_fields_is_unknown() {
    let path = tmp("create-fields").with_extension("pptx");
    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "presentation": {
                "slides": [{ "shapes": [{ "text": "a" }], "fields": [] }]
            }
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

#[test]
fn reject_bad_id_type_and_foreign() {
    let src = create_deck("bad");
    let out = tmp("bad-id").with_extension("pptx");
    let err = apply(
        &src,
        &out,
        json!([{ "op": "insertField", "id": "slide:1", "type": "slidenum" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    assert!(!out.exists());

    let err = apply(
        &src,
        &tmp("notes").with_extension("pptx"),
        json!([{
            "op": "insertField",
            "id": "slide:1/notes/shape:2",
            "type": "slidenum"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);

    let err = apply(
        &src,
        &tmp("other").with_extension("pptx"),
        json!([{
            "op": "insertField",
            "id": first_shape_id(&src),
            "type": "other"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let err = apply(
        &src,
        &tmp("instr").with_extension("pptx"),
        json!([{
            "op": "insertField",
            "id": first_shape_id(&src),
            "instr": " PAGE "
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let missing = tmp("gone").with_extension("pptx");
    let err = apply(
        &src,
        &missing,
        json!([{
            "op": "deleteField",
            "id": "slide:1/fld:{00000000-0000-0000-0000-000000000001}"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::IdNotFound);
    assert!(!missing.exists());

    let book = tmp("xlsx").with_extension("xlsx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": book.to_string_lossy(),
            "workbook": { "sheets": [{ "name": "Sheet1" }] }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let err = apply(
        &book,
        &tmp("xlsx-out").with_extension("xlsx"),
        json!([{
            "op": "insertField",
            "id": first_shape_id(&src),
            "type": "slidenum"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

#[test]
fn set_field_needs_type_or_text() {
    let src = create_deck("need");
    let shape = first_shape_id(&src);
    let inserted = tmp("ins-need").with_extension("pptx");
    apply(
        &src,
        &inserted,
        json!([{ "op": "insertField", "id": shape, "type": "slidenum" }]),
    )
    .unwrap();
    let fid = inspect_fields(&inserted)["slides"][0]["fields"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let out = tmp("set-empty").with_extension("pptx");
    let err = apply(&inserted, &out, json!([{ "op": "setField", "id": fid }])).unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());
}
