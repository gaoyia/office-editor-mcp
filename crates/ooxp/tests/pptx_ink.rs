//! pptx 幻灯片墨迹：一笔一个 contentPart。不进 pictures。位置是画布 EMU。

use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, Lang, RunOutput};
use serde_json::{json, Value};
use zip::ZipArchive;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-pptx-ink");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn seq_path(tag: &str, ext: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}.{ext}"))
}

fn write_ink(tag: &str, body: &str) -> PathBuf {
    let path = seq_path(tag, "xml");
    std::fs::write(&path, body).unwrap();
    path
}

fn ink_a() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<inkml:ink xmlns:inkml="http://www.w3.org/2003/InkML"><inkml:trace>0 0,1 1</inkml:trace></inkml:ink>"#
        .into()
}

fn ink_b() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<inkml:ink xmlns:inkml="http://www.w3.org/2003/InkML"><inkml:trace>9 9</inkml:trace></inkml:ink>"#
        .into()
}

fn create_deck(tag: &str, slides: Value) -> PathBuf {
    let path = seq_path(tag, "pptx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "format": "pptx",
            "presentation": { "slides": slides }
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

fn part_text(path: &Path, name: &str) -> String {
    let bytes = std::fs::read(path).unwrap();
    let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut file = zip.by_name(name.trim_start_matches('/')).unwrap();
    let mut s = String::new();
    file.read_to_string(&mut s).unwrap();
    s
}

fn zip_has(path: &Path, name: &str) -> bool {
    let bytes = std::fs::read(path).unwrap();
    let zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let names: Vec<String> = zip.file_names().map(|n| n.replace('\\', "/")).collect();
    names.iter().any(|n| n == name)
}

#[test]
fn create_lists_ink_not_picture() {
    let file = write_ink("a", &ink_a());
    let path = create_deck(
        "one",
        json!([{
            "inks": [{
                "file": file.to_string_lossy(),
                "x": 269207,
                "y": 2177469,
                "cx": 5910840,
                "cy": 1041120
            }]
        }]),
    );
    let tree = inspect(&path, json!({ "inks": true, "pictures": true }));
    assert_eq!(tree["inkCount"], 1);
    assert_eq!(tree["pictureCount"], 0);
    assert_eq!(tree["slides"][0]["inkCount"], 1);
    assert_eq!(tree["slides"][0]["pictureCount"], 0);
    let item = &tree["slides"][0]["inks"][0];
    assert_eq!(item["name"], "墨迹 1");
    assert_eq!(item["x"], 269207);
    assert_eq!(item["y"], 2177469);
    assert_eq!(item["cx"], 5910840);
    assert_eq!(item["cy"], 1041120);
    assert_eq!(item["contentType"], "application/inkml+xml");
    assert_eq!(item["stable"], true);
    assert!(item.get("trace").is_none());
    assert_eq!(tree["slides"][0]["pictures"], json!([]));
    let id = item["id"].as_str().unwrap();
    assert!(id.starts_with("slide:1/ink:"), "{id}");
    let xml = part_text(&path, "ppt/slides/slide1.xml");
    assert!(xml.contains("Requires=\"p14\""));
    assert!(xml.contains("p:contentPart"));
    assert!(xml.contains("p14:bwMode=\"auto\""));
    assert!(zip_has(&path, "ppt/ink/ink1.xml"));
    let ct = part_text(&path, "[Content_Types].xml");
    assert!(ct.contains("application/inkml+xml"));
}

#[test]
fn apply_round_trip_and_export() {
    let a = write_ink("a", &ink_a());
    let b = write_ink("b", &ink_b());
    let src = create_deck("base", json!([{}]));
    let out = seq_path("ins", "pptx");
    apply(
        &src,
        &out,
        json!([
            {
                "op": "insertInk",
                "id": "slide:1",
                "file": a.to_string_lossy(),
                "x": 100,
                "y": 200,
                "cx": 300000,
                "cy": 400000
            },
            {
                "op": "insertInk",
                "id": "slide:1",
                "file": b.to_string_lossy()
            }
        ]),
    );
    let tree = inspect(&out, json!({ "inks": true }));
    assert_eq!(tree["inkCount"], 2);
    assert_eq!(tree["slides"][0]["inks"][0]["name"], "墨迹 1");
    assert_eq!(tree["slides"][0]["inks"][1]["name"], "墨迹 2");
    let id = tree["slides"][0]["inks"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let moved = seq_path("geom", "pptx");
    apply(
        &out,
        &moved,
        json!([
            { "op": "setInkPos", "id": id, "x": 111, "y": 222 },
            { "op": "setInkSize", "id": id, "cx": 333, "cy": 444 }
        ]),
    );
    let tree = inspect(&moved, json!({ "inks": true }));
    assert_eq!(tree["slides"][0]["inks"][0]["x"], 111);
    assert_eq!(tree["slides"][0]["inks"][0]["y"], 222);
    assert_eq!(tree["slides"][0]["inks"][0]["cx"], 333);
    assert_eq!(tree["slides"][0]["inks"][0]["cy"], 444);
    let xml = part_text(&moved, "ppt/slides/slide1.xml");
    assert!(xml.contains("x=\"111\""));
    assert!(xml.contains("cx=\"333\""));
    let replaced = seq_path("rep", "pptx");
    apply(
        &moved,
        &replaced,
        json!([{ "op": "replaceInk", "id": id, "file": b.to_string_lossy() }]),
    );
    let exp = seq_path("exp", "xml");
    let RunOutput::Written { content_type, .. } = run(
        &json!({
            "schemaVersion": 1,
            "action": "exportMedia",
            "path": replaced.to_string_lossy(),
            "id": id,
            "output": exp.to_string_lossy()
        })
        .to_string(),
        None,
    )
    .unwrap() else {
        panic!("exportMedia");
    };
    assert_eq!(content_type.as_deref(), Some("application/inkml+xml"));
    let got = std::fs::read_to_string(&exp).unwrap();
    assert!(got.contains("<inkml:trace>9 9</inkml:trace>"));
    let gone = seq_path("del", "pptx");
    apply(&replaced, &gone, json!([{ "op": "deleteInk", "id": id }]));
    let tree = inspect(&gone, json!({ "inks": true }));
    assert_eq!(tree["inkCount"], 1);
    assert_eq!(tree["slides"][0]["inks"][0]["name"], "墨迹 2");
}

#[test]
fn notes_ink_stays_off_root_count() {
    let file = write_ink("n", &ink_a());
    let path = create_deck(
        "notes",
        json!([{
            "notes": {
                "text": "备注",
                "inks": [{ "file": file.to_string_lossy(), "x": 10, "y": 20 }]
            }
        }]),
    );
    let tree = inspect(&path, json!({ "notes": true, "inks": true }));
    assert_eq!(tree["inkCount"], 0);
    assert_eq!(tree["slides"][0]["inkCount"], 0);
    assert_eq!(tree["slides"][0]["notes"]["inkCount"], 1);
    assert_eq!(tree["slides"][0]["notes"]["inks"][0]["x"], 10);
    assert!(tree["slides"][0]["notes"]["inks"][0]["id"]
        .as_str()
        .unwrap()
        .starts_with("slide:1/notes/ink:"));
}

#[test]
fn reject_junk_and_bare_slide_id() {
    let src = create_deck("rej", json!([{}]));
    let junk = write_ink("j", "<notink/>");
    let out = seq_path("should-not", "pptx");
    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": src.to_string_lossy(),
            "output": out.to_string_lossy(),
            "ops": [{
                "op": "insertInk",
                "id": "slide:1",
                "file": junk.to_string_lossy()
            }]
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!out.exists());

    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": src.to_string_lossy(),
            "output": out.to_string_lossy(),
            "ops": [{ "op": "deleteInk", "id": "slide:1" }]
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    let msg = err.to_json(Lang::ZhCn)["error"]["message"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(msg.contains("必须带 /ink"), "{msg}");
}

#[test]
fn inspect_office_deck_if_present() {
    let path = Path::new(r"C:\Users\GaoYi-AMD\Desktop\演示文稿2.pptx");
    if !path.is_file() {
        return;
    }
    let tree = inspect(path, json!({ "inks": true, "pictures": true }));
    assert_eq!(tree["inkCount"], 3);
    assert_eq!(tree["slides"][0]["inkCount"], 3);
    assert_eq!(tree["pictureCount"], 0);
    let inks = tree["slides"][0]["inks"].as_array().unwrap();
    let ids: Vec<&str> = inks.iter().map(|v| v["id"].as_str().unwrap()).collect();
    assert_eq!(ids, ["slide:1/ink:4", "slide:1/ink:5", "slide:1/ink:6"]);
    assert_eq!(inks[0]["name"], "墨迹 3");
    assert_eq!(inks[0]["x"], 269207);
    assert_eq!(inks[0]["y"], 2177469);
    assert_eq!(inks[0]["cx"], 5910840);
    assert_eq!(inks[0]["cy"], 1041120);
    assert!(inks
        .iter()
        .all(|v| v["contentType"] == "application/inkml+xml"));
    assert!(inks.iter().all(|v| v.get("trace").is_none()));
}
