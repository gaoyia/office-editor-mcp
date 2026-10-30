//! 母版 / 版式装饰形：inspect `shapes`、insertShape / deleteShape、改字几何与动画。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::ZipArchive;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-pptx-master-deco");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn tmp(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}.pptx"))
}

fn create_deck(tag: &str) -> PathBuf {
    let path = tmp(tag);
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "presentation": {
            "slides": [
                { "shapes": [{ "text": "标题" }] }
            ]
        }
    })
    .to_string())
    .unwrap();
    path
}

fn apply(src: &Path, out: &Path, ops: Value) -> Result<RunOutput, ooxp::Error> {
    run(&json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": out.to_string_lossy(),
        "ops": ops,
    })
    .to_string())
}

fn inspect_masters(path: &Path) -> Value {
    inspect_scope(path, json!({ "masters": true }))
}

fn inspect_scope(path: &Path, scope: Value) -> Value {
    let RunOutput::Inspect(tree) = run(&json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": path.to_string_lossy(),
        "scope": scope
    })
    .to_string())
    .unwrap() else {
        panic!("inspect 应返回树");
    };
    tree
}

fn part(path: &Path, name: &str) -> String {
    let bytes = std::fs::read(path).unwrap();
    let mut zip = ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    let mut file = zip.by_name(name).unwrap();
    let mut out = String::new();
    std::io::Read::read_to_string(&mut file, &mut out).unwrap();
    out
}

#[test]
fn insert_shape_on_master_lists_deco() {
    let src = create_deck("ins");
    let out = tmp("ins-out");
    apply(
        &src,
        &out,
        json!([{
            "op": "insertShape",
            "id": "master:1",
            "text": "徽标",
            "x": 200000,
            "y": 200000
        }]),
    )
    .unwrap();
    let tree = inspect_masters(&out);
    let shapes = tree["masters"][0]["shapes"].as_array().unwrap();
    assert_eq!(shapes.len(), 1);
    assert_eq!(shapes[0]["text"], "徽标");
    assert_eq!(shapes[0]["prst"], "rect");
    assert_eq!(shapes[0]["x"], 200000);
    let id = shapes[0]["id"].as_str().unwrap();
    assert!(id.starts_with("master:1/shape:"));
    assert!(tree["masters"][0]["placeholders"]
        .as_array()
        .unwrap()
        .is_empty());
    let slide_tree = inspect_scope(&out, json!({ "slide": 1 }));
    assert_eq!(slide_tree["slides"][0]["shapes"][0]["text"], "标题");
    assert_eq!(
        slide_tree["slides"][0]["shapes"].as_array().unwrap().len(),
        1
    );
    let xml = part(&out, "ppt/slideMasters/slideMaster1.xml");
    assert!(xml.contains(">徽标</a:t>"));
    assert!(!xml.contains("<p:ph"));
    let slide_xml = part(&out, "ppt/slides/slide1.xml");
    assert!(!slide_xml.contains("徽标"));
}

#[test]
fn set_text_and_pos_on_deco() {
    let src = create_deck("edit");
    let with = tmp("ins");
    apply(
        &src,
        &with,
        json!([{ "op": "insertShape", "id": "master:1", "text": "徽标" }]),
    )
    .unwrap();
    let id = inspect_masters(&with)["masters"][0]["shapes"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let out = tmp("edit-out");
    apply(
        &with,
        &out,
        json!([
            { "op": "setShapeText", "id": id, "text": "改过" },
            { "op": "setShapePos", "id": id, "x": 200000 }
        ]),
    )
    .unwrap();
    let sp = &inspect_masters(&out)["masters"][0]["shapes"][0];
    assert_eq!(sp["text"], "改过");
    assert_eq!(sp["x"], 200000);
}

#[test]
fn insert_anim_on_deco() {
    let src = create_deck("anim");
    let with = tmp("ins");
    apply(
        &src,
        &with,
        json!([{ "op": "insertShape", "id": "master:1", "text": "徽标" }]),
    )
    .unwrap();
    let id = inspect_masters(&with)["masters"][0]["shapes"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let out = tmp("anim-out");
    apply(
        &with,
        &out,
        json!([{ "op": "insertAnim", "id": id, "type": "fade" }]),
    )
    .unwrap();
    let tree = inspect_scope(&out, json!({ "masters": true, "timing": true }));
    assert_eq!(tree["masters"][0]["animCount"], 1);
    assert_eq!(tree["masters"][0]["timing"][0]["target"], id);
    assert_eq!(tree["masters"][0]["timing"][0]["type"], "fade");
    let xml = part(&out, "ppt/slideMasters/slideMaster1.xml");
    assert!(xml.contains("<p:timing"));
    let slide = part(&out, "ppt/slides/slide1.xml");
    assert!(!slide.contains("<p:timing"));
}

#[test]
fn delete_deco_keeps_placeholder_locked() {
    let src = create_deck("del");
    let with = tmp("ins");
    apply(
        &src,
        &with,
        json!([
            { "op": "insertShape", "id": "master:1", "text": "徽标" },
            { "op": "insertPlaceholder", "id": "master:1", "type": "title", "text": "母版标题" }
        ]),
    )
    .unwrap();
    let tree = inspect_masters(&with);
    let deco = tree["masters"][0]["shapes"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let ph = tree["masters"][0]["placeholders"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let fail = tmp("del-ph");
    let err = apply(&with, &fail, json!([{ "op": "deleteShape", "id": ph }])).unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    assert!(!fail.exists());
    let out = tmp("del-deco");
    apply(&with, &out, json!([{ "op": "deleteShape", "id": deco }])).unwrap();
    let tree = inspect_masters(&out);
    assert!(tree["masters"][0]["shapes"].as_array().unwrap().is_empty());
    assert_eq!(tree["masters"][0]["placeholders"][0]["text"], "母版标题");
}

#[test]
fn insert_shape_on_layout_and_reject_bad_ids() {
    let src = create_deck("ly");
    let out = tmp("ly-out");
    apply(
        &src,
        &out,
        json!([{
            "op": "insertShape",
            "id": "master:1/layout:1",
            "text": "版式徽标"
        }]),
    )
    .unwrap();
    let tree = inspect_masters(&out);
    let shapes = tree["masters"][0]["layouts"][0]["shapes"]
        .as_array()
        .unwrap();
    assert_eq!(shapes.len(), 1);
    assert_eq!(shapes[0]["text"], "版式徽标");
    assert!(tree["masters"][0]["shapes"].as_array().unwrap().is_empty());
    let xml = part(&out, "ppt/slideLayouts/slideLayout1.xml");
    assert!(xml.contains("版式徽标"));
    assert!(!xml.contains("<p:ph"));

    let err = apply(
        &src,
        &tmp("bad-pres"),
        json!([{ "op": "insertShape", "id": "pptx:presentation", "text": "x" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);

    let err = apply(
        &out,
        &tmp("bad-shape"),
        json!([{ "op": "insertShape", "id": "slide:1/shape:2", "text": "x" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
}

#[test]
fn fill_on_deco_rejects_placeholder() {
    let src = create_deck("fill");
    let with = tmp("ins");
    apply(
        &src,
        &with,
        json!([
            { "op": "insertShape", "id": "master:1", "text": "徽标" },
            { "op": "insertPlaceholder", "id": "master:1", "type": "title" }
        ]),
    )
    .unwrap();
    let tree = inspect_masters(&with);
    let deco = tree["masters"][0]["shapes"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let ph = tree["masters"][0]["placeholders"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let out = tmp("fill-ok");
    apply(
        &with,
        &out,
        json!([{ "op": "setShapeFill", "id": deco, "color": "FF0000" }]),
    )
    .unwrap();
    let xml = part(&out, "ppt/slideMasters/slideMaster1.xml");
    assert!(xml.contains("FF0000") || xml.contains("ff0000"));
    let fail = tmp("fill-ph");
    let err = apply(
        &with,
        &fail,
        json!([{ "op": "setShapeFill", "id": ph, "color": "FF0000" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    assert!(!fail.exists());
}
