//! 幻灯片形状级超链接：名单、插改删、跳转、跨字段拒绝。

use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::ZipArchive;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-pptx-hyperlinks");
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

fn inspect_links(path: &Path) -> Value {
    let RunOutput::Inspect(tree) = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": path.to_string_lossy(),
            "scope": { "hyperlinks": true }
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

fn second_shape_id(path: &Path) -> String {
    inspect_shapes(path)["slides"][0]["shapes"][1]["id"]
        .as_str()
        .unwrap()
        .to_string()
}

#[test]
fn inspect_hyperlink_count_without_list() {
    let path = create_deck("bare");
    let tree = inspect_bare(&path);
    assert_eq!(tree["hyperlinkCount"], 0);
    assert_eq!(tree["slides"][0]["hyperlinkCount"], 0);
    assert!(tree["slides"][0].get("hyperlinks").is_none());
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
    let listed = inspect_links(&path);
    assert_eq!(listed["slides"][0]["hyperlinks"], json!([]));
    assert_eq!(listed["slides"][1]["hyperlinks"], json!([]));
}

#[test]
fn insert_url_inspect_set_delete() {
    let src = create_deck("round");
    let shape = first_shape_id(&src);
    let inserted = tmp("ins").with_extension("pptx");
    apply(
        &src,
        &inserted,
        json!([{
            "op": "insertHyperlink",
            "id": shape,
            "url": "https://example.com/",
            "tooltip": "提示"
        }]),
    )
    .unwrap();
    let xml = part_text(&inserted, "ppt/slides/slide1.xml");
    assert!(xml.contains("hlinkClick"));
    assert!(xml.contains("tooltip=\"提示\""));
    assert!(!xml.contains("ppaction://"));
    let rels = part_text(&inserted, "ppt/slides/_rels/slide1.xml.rels");
    assert!(rels.contains("/hyperlink"));
    assert!(rels.contains("TargetMode=\"External\""));
    assert!(rels.contains("https://example.com/"));

    let tree = inspect_links(&inserted);
    assert_eq!(tree["hyperlinkCount"], 1);
    assert_eq!(tree["slides"][0]["hyperlinkCount"], 1);
    assert_eq!(tree["slides"][1]["hyperlinkCount"], 0);
    let links = tree["slides"][0]["hyperlinks"].as_array().unwrap();
    assert_eq!(links.len(), 1);
    assert_eq!(
        object_keys(&links[0]),
        ["id", "index", "kind", "host", "rId", "url", "file", "slide", "tooltip"]
    );
    assert_eq!(links[0]["id"], "slide:1/link:1");
    assert_eq!(links[0]["index"], 1);
    assert_eq!(links[0]["kind"], "url");
    assert_eq!(links[0]["host"], shape);
    assert_eq!(links[0]["url"], "https://example.com/");
    assert!(links[0]["file"].is_null());
    assert!(links[0]["slide"].is_null());
    assert_eq!(links[0]["tooltip"], "提示");
    let rid = links[0]["rId"].as_str().unwrap();
    assert!(rid.starts_with("rId"));

    let set_out = tmp("set").with_extension("pptx");
    apply(
        &inserted,
        &set_out,
        json!([{
            "op": "setHyperlink",
            "id": "slide:1/link:1",
            "url": "https://example.org/x",
            "tooltip": null
        }]),
    )
    .unwrap();
    let tree_set = inspect_links(&set_out);
    let link = &tree_set["slides"][0]["hyperlinks"][0];
    assert_eq!(link["url"], "https://example.org/x");
    assert!(link["tooltip"].is_null());
    let xml_set = part_text(&set_out, "ppt/slides/slide1.xml");
    assert!(!xml_set.contains("tooltip="));

    let text_out = tmp("text").with_extension("pptx");
    apply(
        &set_out,
        &text_out,
        json!([{ "op": "setShapeText", "id": shape, "text": "改字仍留链" }]),
    )
    .unwrap();
    let tree_text = inspect_links(&text_out);
    assert_eq!(
        tree_text["slides"][0]["hyperlinks"][0]["url"],
        "https://example.org/x"
    );
    let shapes = inspect_shapes(&text_out);
    assert_eq!(shapes["slides"][0]["shapes"][0]["text"], "改字仍留链");

    let del = tmp("del").with_extension("pptx");
    apply(
        &text_out,
        &del,
        json!([{ "op": "deleteHyperlink", "id": "slide:1/link:1" }]),
    )
    .unwrap();
    let tree_del = inspect_links(&del);
    assert_eq!(tree_del["hyperlinkCount"], 0);
    assert_eq!(tree_del["slides"][0]["hyperlinks"], json!([]));
    let xml_del = part_text(&del, "ppt/slides/slide1.xml");
    assert!(!xml_del.contains("hlinkClick"));
    assert!(xml_del.contains("改字仍留链"));
    let rels_del = part_text(&del, "ppt/slides/_rels/slide1.xml.rels");
    assert!(!rels_del.contains("/hyperlink"));
}

#[test]
fn insert_slide_jump_and_file() {
    let src = create_deck("jump");
    let first = first_shape_id(&src);
    let second = second_shape_id(&src);
    let out = tmp("jump-out").with_extension("pptx");
    apply(
        &src,
        &out,
        json!([
            { "op": "insertHyperlink", "id": first, "slide": "slide:2" },
            { "op": "insertHyperlink", "id": second, "file": "file:///C:/data/deck.pptx" }
        ]),
    )
    .unwrap();
    let tree = inspect_links(&out);
    assert_eq!(tree["hyperlinkCount"], 2);
    let a = &tree["slides"][0]["hyperlinks"][0];
    let b = &tree["slides"][0]["hyperlinks"][1];
    assert_eq!(a["kind"], "slide");
    assert_eq!(a["slide"], "slide:2");
    assert!(a["url"].is_null());
    assert_eq!(b["kind"], "file");
    assert_eq!(b["file"], "file:///C:/data/deck.pptx");
    let xml = part_text(&out, "ppt/slides/slide1.xml");
    assert!(xml.contains("ppaction://hlinksldjump"));
    assert!(xml.contains("ppaction://hlinkfile"));
    let rels = part_text(&out, "ppt/slides/_rels/slide1.xml.rels");
    assert!(rels.contains("/slide"));
    assert!(rels.contains("slide2.xml"));
    assert!(rels.contains("/hyperlink"));
}

#[test]
fn reject_bad_id_and_missing_target() {
    let src = create_deck("bad");
    let out = tmp("bad-id").with_extension("pptx");
    let err = apply(
        &src,
        &out,
        json!([{ "op": "insertHyperlink", "id": "slide:1", "url": "https://example.com/" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    assert!(!out.exists());

    let err = apply(
        &src,
        &tmp("miss").with_extension("pptx"),
        json!([{ "op": "insertHyperlink", "id": first_shape_id(&src) }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let err = apply(
        &src,
        &tmp("self").with_extension("pptx"),
        json!([{
            "op": "insertHyperlink",
            "id": first_shape_id(&src),
            "slide": "slide:1"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let err = apply(
        &src,
        &tmp("gone").with_extension("pptx"),
        json!([{
            "op": "insertHyperlink",
            "id": first_shape_id(&src),
            "slide": "slide:9"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::IdNotFound);

    let err = apply(
        &src,
        &tmp("notes").with_extension("pptx"),
        json!([{
            "op": "insertHyperlink",
            "id": "slide:1/notes/shape:2",
            "url": "https://example.com/"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);

    let err = apply(
        &src,
        &tmp("text").with_extension("pptx"),
        json!([{
            "op": "insertHyperlink",
            "id": first_shape_id(&src),
            "url": "https://example.com/",
            "text": "字"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

#[test]
fn already_linked_and_create_unknown_field() {
    let src = create_deck("dup");
    let shape = first_shape_id(&src);
    let one = tmp("one").with_extension("pptx");
    apply(
        &src,
        &one,
        json!([{ "op": "insertHyperlink", "id": shape, "url": "https://example.com/" }]),
    )
    .unwrap();
    let err = apply(
        &one,
        &tmp("two").with_extension("pptx"),
        json!([{ "op": "insertHyperlink", "id": shape, "url": "https://example.org/" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": tmp("create-hl").with_extension("pptx").to_string_lossy(),
            "presentation": {
                "slides": [{ "shapes": [{ "text": "a" }], "hyperlinks": [] }]
            }
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

#[test]
fn set_url_to_slide_jump() {
    let src = create_deck("retarget");
    let shape = first_shape_id(&src);
    let inserted = tmp("ins").with_extension("pptx");
    apply(
        &src,
        &inserted,
        json!([{ "op": "insertHyperlink", "id": shape, "url": "https://example.com/" }]),
    )
    .unwrap();
    let out = tmp("jump").with_extension("pptx");
    apply(
        &inserted,
        &out,
        json!([{ "op": "setHyperlink", "id": "slide:1/link:1", "slide": "slide:2" }]),
    )
    .unwrap();
    let tree = inspect_links(&out);
    let link = &tree["slides"][0]["hyperlinks"][0];
    assert_eq!(link["kind"], "slide");
    assert_eq!(link["slide"], "slide:2");
    assert!(link["url"].is_null());
    let rels = part_text(&out, "ppt/slides/_rels/slide1.xml.rels");
    assert!(rels.contains("slide2.xml"));
    assert!(!rels.contains("/hyperlink"));
}
