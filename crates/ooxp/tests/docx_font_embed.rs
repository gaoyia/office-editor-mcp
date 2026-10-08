//! docx 嵌入字体：insertEmbeddedFont / deleteEmbeddedFont / setFontEmbed、inspect 计数、exportMedia、xlsx/pptx 拒绝。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-docx-font-embed");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn seq_path(tag: &str, ext: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}.{ext}"))
}

fn calibri() -> &'static str {
    r"C:\Windows\Fonts\calibri.ttf"
}

fn create_docx(tag: &str, document: Value) -> PathBuf {
    let path = seq_path(tag, "docx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "format": "docx",
            "document": document
        })
        .to_string(),
        None,
    )
    .unwrap();
    path
}

fn inspect(path: &Path, embedded: bool) -> Value {
    let RunOutput::Inspect(tree) = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": path.to_string_lossy(),
            "scope": {
                "from": 1,
                "maxParas": 20,
                "embeddedFonts": embedded
            }
        })
        .to_string(),
        None,
    )
    .unwrap() else {
        panic!("inspect 应返回树");
    };
    tree
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

fn part_exists(path: &Path, name: &str) -> bool {
    let f = std::fs::File::open(path).unwrap();
    let mut zip = zip::ZipArchive::new(f).unwrap();
    let ok = zip.by_name(name).is_ok();
    ok
}

fn part_bytes(path: &Path, name: &str) -> Vec<u8> {
    let f = std::fs::File::open(path).unwrap();
    let mut zip = zip::ZipArchive::new(f).unwrap();
    let mut zf = zip.by_name(name).unwrap();
    let mut buf = Vec::new();
    std::io::Read::read_to_end(&mut zf, &mut buf).unwrap();
    buf
}

fn part_text(path: &Path, name: &str) -> String {
    String::from_utf8_lossy(&part_bytes(path, name)).into_owned()
}

fn insert_op() -> Value {
    json!({
        "op": "insertEmbeddedFont",
        "id": "docx:document",
        "name": "Calibri",
        "file": calibri(),
        "style": "regular"
    })
}

#[test]
fn insert_export_set_flags_then_delete() {
    assert!(Path::new(calibri()).is_file(), "本机应有 Calibri.ttf");
    let src = create_docx(
        "fe-src",
        json!({
            "paragraphs": [
                { "style": "Heading1", "text": "第一章" },
                { "text": "hello" }
            ]
        }),
    );
    let bare = inspect_bare(&src);
    assert_eq!(bare["embeddedFontCount"], 0);
    assert!(bare.get("embeddedFonts").is_none());
    assert_eq!(bare["fontEmbed"]["trueType"], false);
    assert_eq!(bare["fontEmbed"]["systemFonts"], false);
    assert_eq!(bare["fontEmbed"]["subset"], false);
    let keys = object_keys(&bare);
    let style = keys.iter().position(|k| *k == "styleCount").unwrap();
    let efc = keys.iter().position(|k| *k == "embeddedFontCount").unwrap();
    let outline = keys.iter().position(|k| *k == "outline").unwrap();
    assert!(style < efc && efc < outline, "{keys:?}");
    let theme = keys.iter().position(|k| *k == "theme").unwrap();
    assert_eq!(keys[theme + 1], "fontEmbed");
    assert_eq!(
        object_keys(&bare["fontEmbed"]),
        ["trueType", "systemFonts", "subset"]
    );

    let with_font = seq_path("fe-insert", "docx");
    apply(&src, &with_font, json!([insert_op()]));
    let tree = inspect(&with_font, true);
    assert_eq!(tree["embeddedFontCount"], 1);
    assert_eq!(tree["embeddedFonts"][0]["id"], "font:1");
    assert_eq!(tree["embeddedFonts"][0]["index"], 1);
    assert_eq!(tree["embeddedFonts"][0]["name"], "Calibri");
    assert_eq!(tree["embeddedFonts"][0]["style"], "regular");
    assert_eq!(tree["embeddedFonts"][0]["subsetted"], false);
    assert!(tree["embeddedFonts"][0]["fontKey"]
        .as_str()
        .unwrap()
        .starts_with('{'));
    assert_eq!(tree["fontEmbed"]["trueType"], true);
    assert_eq!(
        object_keys(&tree["embeddedFonts"][0]),
        ["id", "index", "name", "style", "subsetted", "fontKey"]
    );
    assert!(part_exists(&with_font, "word/fonts/font1.odttf"));
    let settings = part_text(&with_font, "word/settings.xml");
    assert!(settings.contains("embedTrueTypeFonts"));
    assert!(!settings.contains("subsetted"));

    let ttf = std::fs::read(calibri()).unwrap();
    let odttf = part_bytes(&with_font, "word/fonts/font1.odttf");
    assert_eq!(odttf.len(), ttf.len());
    assert_ne!(&odttf[..32], &ttf[..32]);

    let exp = seq_path("fe-export", "odttf");
    let RunOutput::Written { content_type, .. } = run(
        &json!({
            "schemaVersion": 1,
            "action": "exportMedia",
            "path": with_font.to_string_lossy(),
            "id": "font:1",
            "output": exp.to_string_lossy()
        })
        .to_string(),
        None,
    )
    .unwrap() else {
        panic!("exportMedia 应写出文件");
    };
    assert_eq!(std::fs::read(&exp).unwrap(), odttf);
    assert_eq!(
        content_type.as_deref(),
        Some("application/vnd.openxmlformats-officedocument.obfuscatedFont")
    );

    let flagged = seq_path("fe-flag", "docx");
    apply(
        &with_font,
        &flagged,
        json!([{
            "op": "setFontEmbed",
            "id": "docx:document",
            "subset": true
        }]),
    );
    let tree_f = inspect_bare(&flagged);
    assert_eq!(tree_f["fontEmbed"]["trueType"], true);
    assert_eq!(tree_f["fontEmbed"]["subset"], true);
    assert_eq!(tree_f["embeddedFontCount"], 1);

    let gone = seq_path("fe-del", "docx");
    apply(
        &flagged,
        &gone,
        json!([{ "op": "deleteEmbeddedFont", "id": "font:1" }]),
    );
    let tree_d = inspect(&gone, true);
    assert_eq!(tree_d["embeddedFontCount"], 0);
    assert_eq!(tree_d["embeddedFonts"], json!([]));
    assert!(!part_exists(&gone, "word/fonts/font1.odttf"));
    assert!(part_exists(&gone, "word/fontTable.xml"));
}

#[test]
fn insert_rejects_bad_fields_and_missing_delete() {
    let src = create_docx(
        "fe-err",
        json!({
            "paragraphs": [
                { "style": "Heading1", "text": "第一章" },
                { "text": "hello" }
            ]
        }),
    );
    let with_font = seq_path("fe-dup-host", "docx");
    apply(&src, &with_font, json!([insert_op()]));

    let err_name = apply_err(
        &src,
        json!([{
            "op": "insertEmbeddedFont",
            "id": "docx:document",
            "name": "  ",
            "file": calibri()
        }]),
    );
    assert_eq!(err_name.code(), ErrorCode::InvalidField);

    let err_style = apply_err(
        &src,
        json!([{
            "op": "insertEmbeddedFont",
            "id": "docx:document",
            "name": "Calibri",
            "file": calibri(),
            "style": "underline"
        }]),
    );
    assert_eq!(err_style.code(), ErrorCode::InvalidField);

    let err_subsetted = apply_err(
        &src,
        json!([{
            "op": "insertEmbeddedFont",
            "id": "docx:document",
            "name": "Calibri",
            "file": calibri(),
            "subsetted": true
        }]),
    );
    assert_eq!(err_subsetted.code(), ErrorCode::InvalidField);

    let err_dup = apply_err(&with_font, json!([insert_op()]));
    assert_eq!(err_dup.code(), ErrorCode::InvalidField);

    let err_id = apply_err(
        &src,
        json!([{
            "op": "insertEmbeddedFont",
            "id": "para:1",
            "name": "Calibri",
            "file": calibri()
        }]),
    );
    assert_eq!(err_id.code(), ErrorCode::InvalidId);

    let err_del = apply_err(
        &src,
        json!([{ "op": "deleteEmbeddedFont", "id": "font:1" }]),
    );
    assert_eq!(err_del.code(), ErrorCode::IdNotFound);

    let err_prefix = apply_err(
        &src,
        json!([{ "op": "deleteEmbeddedFont", "id": "docPart:1" }]),
    );
    assert_eq!(err_prefix.code(), ErrorCode::InvalidId);

    let err_flag = apply_err(
        &src,
        json!([{ "op": "setFontEmbed", "id": "docx:document" }]),
    );
    assert_eq!(err_flag.code(), ErrorCode::InvalidField);

    let junk = seq_path("fe-not-font", "txt");
    std::fs::write(&junk, b"not a font").unwrap();
    let err_file = apply_err(
        &src,
        json!([{
            "op": "insertEmbeddedFont",
            "id": "docx:document",
            "name": "Calibri",
            "file": junk.to_string_lossy()
        }]),
    );
    assert_eq!(err_file.code(), ErrorCode::InvalidField);
}

#[test]
fn font_ops_not_for_xlsx_or_pptx() {
    let xlsx = seq_path("fe-x", "xlsx");
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
    let xout = seq_path("fe-x-out", "xlsx");
    let err_x = run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": xlsx.to_string_lossy(),
            "output": xout.to_string_lossy(),
            "ops": [insert_op()]
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err_x.code(), ErrorCode::InvalidField);
    assert!(!xout.exists());

    let err_scope = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": xlsx.to_string_lossy(),
            "scope": { "embeddedFonts": true }
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err_scope.code(), ErrorCode::InvalidField);

    let pptx = seq_path("fe-p", "pptx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": pptx.to_string_lossy(),
            "presentation": { "slides": [{ "shapes": [{ "text": "a" }] }] }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let pout = seq_path("fe-p-out", "pptx");
    let err_p = run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": pptx.to_string_lossy(),
            "output": pout.to_string_lossy(),
            "ops": [insert_op()]
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err_p.code(), ErrorCode::InvalidField);
    assert!(!pout.exists());

    let err_scope_p = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": pptx.to_string_lossy(),
            "scope": { "embeddedFonts": true }
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err_scope_p.code(), ErrorCode::InvalidField);
}
