//! 修改保护与放映设置：inspect `showPr` / `modifyProtection`，setShowPr / setProtection。

use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-pptx-protect");
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
                "slides": [{ "shapes": [{ "text": "页1" }] }]
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

fn rewrite_part(src: &Path, dst: &Path, name: &str, f: impl FnOnce(&str) -> String) {
    let bytes = std::fs::read(src).unwrap();
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
                let xml = String::from_utf8(data).unwrap();
                data = transform.take().expect("部件只应出现一次")(&xml).into_bytes();
            }
            zip_out.start_file(&n, opt).unwrap();
            zip_out.write_all(&data).unwrap();
        }
        zip_out.finish().unwrap();
    }
    std::fs::write(dst, out.into_inner()).unwrap();
}

#[test]
fn create_defaults_and_root_key_order() {
    let path = create_deck("bare");
    let tree = inspect(&path);
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
            "oleObjectCount",
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
    assert_eq!(
        object_keys(&tree["showPr"]),
        [
            "type",
            "loop",
            "showNarration",
            "showAnimation",
            "useTimings"
        ]
    );
    assert_eq!(tree["showPr"]["type"], "present");
    assert_eq!(tree["showPr"]["loop"], false);
    assert_eq!(tree["showPr"]["showNarration"], true);
    assert_eq!(tree["showPr"]["showAnimation"], true);
    assert_eq!(tree["showPr"]["useTimings"], true);
    assert_eq!(object_keys(&tree["modifyProtection"]), ["hasPassword"]);
    assert_eq!(tree["modifyProtection"]["hasPassword"], false);
    let xml = part_text(&path, "ppt/presProps.xml");
    assert!(!xml.contains("showPr"));
}

#[test]
fn set_show_pr_kiosk_writes_loop_and_narration() {
    let src = create_deck("kiosk");
    let out = tmp("kiosk").with_extension("pptx");
    apply(
        &src,
        &out,
        json!([{
            "op": "setShowPr",
            "id": "pptx:presentation",
            "type": "kiosk",
            "loop": true
        }]),
    )
    .unwrap();
    let tree = inspect(&out);
    assert_eq!(tree["showPr"]["type"], "kiosk");
    assert_eq!(tree["showPr"]["loop"], true);
    assert_eq!(tree["showPr"]["showNarration"], true);
    let xml = part_text(&out, "ppt/presProps.xml");
    assert!(xml.contains(r#"loop="1""#), "{xml}");
    assert!(xml.contains(r#"showNarration="1""#), "{xml}");
    assert!(xml.contains("<p:kiosk"), "{xml}");
    assert!(xml.contains("<p:sldAll"), "{xml}");
    assert!(!xml.contains("penClr"), "{xml}");
    let clr = xml.find("clrMru").expect("应有 clrMru");
    let show = xml.find("showPr").expect("应有 showPr");
    assert!(show < clr, "showPr 应在 clrMru 之前");
}

#[test]
fn set_show_pr_browse_then_revert_drops_element() {
    let src = create_deck("browse");
    let browse = tmp("browse").with_extension("pptx");
    apply(
        &src,
        &browse,
        json!([{
            "op": "setShowPr",
            "id": "pptx:presentation",
            "type": "browse"
        }]),
    )
    .unwrap();
    assert_eq!(inspect(&browse)["showPr"]["type"], "browse");
    let xml = part_text(&browse, "ppt/presProps.xml");
    assert!(xml.contains("<p:browse"), "{xml}");
    assert!(xml.contains(r#"showNarration="1""#), "{xml}");
    assert!(xml.contains("browseMode"), "{xml}");
    assert!(xml.contains(r#"showStatus="0""#), "{xml}");

    let back = tmp("present").with_extension("pptx");
    apply(
        &browse,
        &back,
        json!([{
            "op": "setShowPr",
            "id": "pptx:presentation",
            "type": "present",
            "loop": false
        }]),
    )
    .unwrap();
    let tree = inspect(&back);
    assert_eq!(tree["showPr"]["type"], "present");
    assert_eq!(tree["showPr"]["loop"], false);
    assert_eq!(tree["showPr"]["showNarration"], true);
    let xml = part_text(&back, "ppt/presProps.xml");
    assert!(!xml.contains("showPr"), "{xml}");
}

#[test]
fn schema_defaults_when_element_exists() {
    let src = create_deck("schema");
    let injected = tmp("schema-in").with_extension("pptx");
    rewrite_part(&src, &injected, "ppt/presProps.xml", |xml| {
        xml.replace(
            "<p:clrMru/>",
            r#"<p:showPr><p:present/><p:sldAll/></p:showPr><p:clrMru/>"#,
        )
    });
    let tree = inspect(&injected);
    assert_eq!(tree["showPr"]["type"], "present");
    assert_eq!(tree["showPr"]["loop"], false);
    assert_eq!(tree["showPr"]["showNarration"], false);
    assert_eq!(tree["showPr"]["showAnimation"], true);
    assert_eq!(tree["showPr"]["useTimings"], true);
}

#[test]
fn empty_verifier_can_be_deleted_hashed_cannot() {
    let src = create_deck("ver");
    let empty = tmp("empty-ver").with_extension("pptx");
    rewrite_part(&src, &empty, "ppt/presentation.xml", |xml| {
        xml.replace("</p:presentation>", "<p:modifyVerifier/></p:presentation>")
    });
    assert_eq!(inspect(&empty)["modifyProtection"]["hasPassword"], false);
    let cleared = tmp("empty-cleared").with_extension("pptx");
    apply(
        &empty,
        &cleared,
        json!([{ "op": "setProtection", "id": "pptx:presentation", "enforced": false }]),
    )
    .unwrap();
    assert!(!part_text(&cleared, "ppt/presentation.xml").contains("modifyVerifier"));
    assert_eq!(inspect(&cleared)["modifyProtection"]["hasPassword"], false);

    let hashed = tmp("hash-ver").with_extension("pptx");
    rewrite_part(&src, &hashed, "ppt/presentation.xml", |xml| {
        xml.replace(
            "</p:presentation>",
            r#"<p:modifyVerifier algorithmName="SHA-512" hashValue="YWJj" saltValue="ZGVm" spinCount="100000"/></p:presentation>"#,
        )
    });
    let tree = inspect(&hashed);
    assert_eq!(tree["modifyProtection"]["hasPassword"], true);
    assert!(!tree.to_string().contains("YWJj"));
    let out = tmp("hash-out").with_extension("pptx");
    let err = apply(
        &hashed,
        &out,
        json!([{ "op": "setProtection", "id": "pptx:presentation", "enforced": false }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::NotImplemented);
    assert!(!out.exists());
}

#[test]
fn protection_false_without_element_is_noop() {
    let src = create_deck("noop");
    let out = tmp("noop").with_extension("pptx");
    apply(
        &src,
        &out,
        json!([{ "op": "setProtection", "id": "pptx:presentation", "enforced": false }]),
    )
    .unwrap();
    assert_eq!(inspect(&out)["modifyProtection"]["hasPassword"], false);
    assert!(!part_text(&out, "ppt/presentation.xml").contains("modifyVerifier"));
}

#[test]
fn rejects_password_enforced_true_wrong_id_and_foreign() {
    let src = create_deck("err");
    let out = tmp("should-not").with_extension("pptx");

    let err = apply(
        &src,
        &out,
        json!([{ "op": "setProtection", "id": "pptx:presentation", "enforced": true }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::NotImplemented);
    assert!(!out.exists());

    let err = apply(
        &src,
        &out,
        json!([{
            "op": "setProtection",
            "id": "pptx:presentation",
            "enforced": false,
            "password": "secret"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::NotImplemented);

    let err = apply(
        &src,
        &out,
        json!([{ "op": "setProtection", "id": "pptx:presentation" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let err = apply(
        &src,
        &out,
        json!([{ "op": "setShowPr", "id": "slide:1", "loop": true }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);

    let err = apply(
        &src,
        &out,
        json!([{ "op": "setShowPr", "id": "pptx:presentation" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let err = apply(
        &src,
        &out,
        json!([{ "op": "setShowPr", "id": "pptx:presentation", "type": "other" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);

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
        json!([{ "op": "setShowPr", "id": "pptx:presentation", "loop": true }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let docx = tmp("docx").with_extension("docx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": docx.to_string_lossy(),
            "document": { "paragraphs": [{ "text": "甲" }] }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let err = apply(
        &docx,
        &tmp("docx-out").with_extension("docx"),
        json!([{ "op": "setShowPr", "id": "pptx:presentation", "loop": true }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let created = tmp("create-show").with_extension("pptx");
    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": created.to_string_lossy(),
            "presentation": {
                "showPr": { "type": "kiosk" },
                "slides": [{ "shapes": [{ "text": "a" }] }]
            }
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}
