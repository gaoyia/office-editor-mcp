//! docx 页面背景与节页面边框：inspect / apply、xlsx 拒绝。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-docx-page-bg");
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

fn inspect(path: &Path) -> Value {
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

fn object_keys(v: &Value) -> Vec<&str> {
    v.as_object().unwrap().keys().map(|k| k.as_str()).collect()
}

fn part_text(path: &Path, name: &str) -> String {
    let f = std::fs::File::open(path).unwrap();
    let mut zip = zip::ZipArchive::new(f).unwrap();
    let mut zf = zip.by_name(name).unwrap();
    let mut buf = Vec::new();
    std::io::Read::read_to_end(&mut zf, &mut buf).unwrap();
    String::from_utf8_lossy(&buf).into_owned()
}

fn side() -> Value {
    json!({ "val": "single", "sz": 4, "space": 24, "color": "auto" })
}

#[test]
fn inspect_has_page_background_after_font_embed() {
    let src = create_docx("bare");
    let tree = inspect(&src);
    assert!(tree["pageBackground"]["color"].is_null());
    assert_eq!(tree["pageBackground"]["display"], false);
    assert!(tree["sections"][0]["pageBorders"].is_null());
    let keys = object_keys(&tree);
    let font = keys.iter().position(|k| *k == "fontEmbed").unwrap();
    assert_eq!(keys[font + 1], "pageBackground");
    assert_eq!(keys[font + 2], "eastAsianSettings");
    assert_eq!(object_keys(&tree["pageBackground"]), ["color", "display"]);
    let sect_keys = object_keys(&tree["sections"][0]);
    assert_eq!(*sect_keys.last().unwrap(), "pageBorders");
}

#[test]
fn set_and_clear_page_background() {
    let src = create_docx("bg");
    let colored = seq_path("bg-red", "docx");
    apply(
        &src,
        &colored,
        json!([{
            "op": "setPageBackground",
            "id": "docx:document",
            "color": "ff0000"
        }]),
    );
    let tree = inspect(&colored);
    assert_eq!(tree["pageBackground"]["color"], "FF0000");
    assert_eq!(tree["pageBackground"]["display"], true);
    let doc = part_text(&colored, "word/document.xml");
    assert!(doc.contains("w:background"));
    assert!(doc.contains("FF0000"));
    let settings = part_text(&colored, "word/settings.xml");
    assert!(settings.contains("displayBackgroundShape"));
    assert!(!settings.contains("footnotes"));

    let cleared = seq_path("bg-clear", "docx");
    apply(
        &colored,
        &cleared,
        json!([{
            "op": "setPageBackground",
            "id": "docx:document",
            "color": null
        }]),
    );
    let tree = inspect(&cleared);
    assert!(tree["pageBackground"]["color"].is_null());
    assert_eq!(tree["pageBackground"]["display"], false);
    let doc = part_text(&cleared, "word/document.xml");
    assert!(!doc.contains("w:background"));
}

#[test]
fn set_and_clear_page_borders() {
    let src = create_docx("bd");
    let bordered = seq_path("bd-set", "docx");
    apply(
        &src,
        &bordered,
        json!([{
            "op": "setPageBorders",
            "id": "section:1",
            "offsetFrom": "page",
            "top": side(),
            "left": side(),
            "bottom": side(),
            "right": side()
        }]),
    );
    let tree = inspect(&bordered);
    let b = &tree["sections"][0]["pageBorders"];
    assert_eq!(b["offsetFrom"], "page");
    assert_eq!(b["top"]["val"], "single");
    assert_eq!(b["top"]["sz"], 4);
    assert_eq!(b["top"]["space"], 24);
    assert_eq!(b["top"]["color"], "auto");
    assert_eq!(b["left"]["val"], "single");
    assert_eq!(b["right"]["val"], "single");
    assert_eq!(
        object_keys(b),
        [
            "offsetFrom",
            "display",
            "zOrder",
            "top",
            "left",
            "bottom",
            "right"
        ]
    );
    let xml = part_text(&bordered, "word/document.xml");
    assert!(xml.contains("w:pgBorders"));
    let pg_mar = xml.find("w:pgMar").unwrap();
    let pg_borders = xml.find("w:pgBorders").unwrap();
    assert!(pg_mar < pg_borders);

    let cleared = seq_path("bd-clear", "docx");
    apply(
        &bordered,
        &cleared,
        json!([{
            "op": "setPageBorders",
            "id": "section:1",
            "enabled": false
        }]),
    );
    let tree = inspect(&cleared);
    assert!(tree["sections"][0]["pageBorders"].is_null());
}

#[test]
fn page_bg_ops_reject_bad_fields() {
    let src = create_docx("err");
    let err_color = apply_err(
        &src,
        json!([{
            "op": "setPageBackground",
            "id": "docx:document",
            "color": "red"
        }]),
    );
    assert_eq!(err_color.code(), ErrorCode::InvalidField);

    let err_id = apply_err(
        &src,
        json!([{
            "op": "setPageBackground",
            "id": "section:1",
            "color": "FF0000"
        }]),
    );
    assert_eq!(err_id.code(), ErrorCode::InvalidId);

    let err_empty = apply_err(
        &src,
        json!([{
            "op": "setPageBorders",
            "id": "section:1"
        }]),
    );
    assert_eq!(err_empty.code(), ErrorCode::InvalidField);

    let err_val = apply_err(
        &src,
        json!([{
            "op": "setPageBorders",
            "id": "section:1",
            "top": { "val": "art" }
        }]),
    );
    assert_eq!(err_val.code(), ErrorCode::InvalidField);
}

#[test]
fn page_bg_ops_not_for_xlsx() {
    let xlsx = seq_path("pb-x", "xlsx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": xlsx.to_string_lossy(),
            "workbook": { "sheets": [{ "name": "Sheet1" }] }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let xout = seq_path("pb-x-out", "xlsx");
    let err_x = run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": xlsx.to_string_lossy(),
            "output": xout.to_string_lossy(),
            "ops": [{
                "op": "setPageBackground",
                "id": "docx:document",
                "color": "FF0000"
            }]
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err_x.code(), ErrorCode::InvalidField);
    assert!(!xout.exists());
}
