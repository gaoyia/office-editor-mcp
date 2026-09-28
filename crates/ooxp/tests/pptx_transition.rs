//! 幻灯片切换：inspect 名单、setTransition、清除，以及改字不丢 timing。

use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-pptx-transition");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn tmp(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}.pptx"))
}

fn create_deck(tag: &str) -> PathBuf {
    let path = tmp(tag);
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

fn inspect_scope(path: &Path, scope: Value) -> Value {
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

fn part(path: &Path, name: &str) -> String {
    String::from_utf8(part_bytes(path, name)).unwrap()
}

fn part_bytes(path: &Path, name: &str) -> Vec<u8> {
    let bytes = std::fs::read(path).unwrap();
    let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut file = zip.by_name(name).unwrap();
    let mut out = Vec::new();
    file.read_to_end(&mut out).unwrap();
    out
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

fn with_p14(xml: &str) -> String {
    xml.replace(
        r#"xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main""#,
        concat!(
            r#"xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" "#,
            r#"xmlns:p14="http://schemas.microsoft.com/office/powerpoint/2010/main""#
        ),
    )
}

fn inject_before_end(xml: &str, snippet: &str) -> String {
    xml.replace("</p:sld>", &format!("{snippet}</p:sld>"))
}

#[test]
fn create_has_null_transition() {
    let path = create_deck("bare");
    let tree = inspect(&path);
    assert_eq!(tree["slides"][0]["transition"], Value::Null);
    assert_eq!(tree["slides"][1]["transition"], Value::Null);
    let xml = part(&path, "ppt/slides/slide1.xml");
    assert!(!xml.contains("<p:transition"));
}

#[test]
fn inspect_reads_fade_and_other() {
    let fade = create_deck("fade-src");
    rewrite_part(&fade, "ppt/slides/slide1.xml", |data| {
        let xml = with_p14(&String::from_utf8(data).unwrap());
        inject_before_end(
            &xml,
            r#"<p:transition spd="slow" p14:dur="2000"><p:fade/></p:transition>"#,
        )
        .into_bytes()
    });
    let tree = inspect(&fade);
    assert_eq!(tree["slides"][0]["transition"]["type"], "fade");
    assert_eq!(tree["slides"][0]["transition"]["dur"], 2000);
    assert_eq!(tree["slides"][0]["transition"]["advClick"], true);
    assert_eq!(tree["slides"][0]["transition"]["advTm"], Value::Null);
    assert_eq!(tree["slides"][1]["transition"], Value::Null);

    let other = create_deck("other-src");
    rewrite_part(&other, "ppt/slides/slide1.xml", |data| {
        let xml = String::from_utf8(data).unwrap();
        inject_before_end(
            &xml,
            r#"<p:transition spd="med"><p:prstTrans prst="flash"/></p:transition>"#,
        )
        .into_bytes()
    });
    let tree = inspect(&other);
    assert_eq!(tree["slides"][0]["transition"]["type"], "other");
    assert_eq!(tree["slides"][0]["transition"]["dur"], 750);

    let ripple = create_deck("ripple-src");
    rewrite_part(&ripple, "ppt/slides/slide1.xml", |data| {
        let xml = with_p14(&String::from_utf8(data).unwrap());
        inject_before_end(
            &xml,
            concat!(
                r#"<mc:AlternateContent xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006">"#,
                r#"<mc:Choice Requires="p14">"#,
                r#"<p:transition spd="fast" p14:dur="800"><p14:ripple/></p:transition>"#,
                r#"</mc:Choice>"#,
                r#"<mc:Fallback><p:transition spd="fast"><p:fade/></p:transition></mc:Fallback>"#,
                r#"</mc:AlternateContent>"#
            ),
        )
        .into_bytes()
    });
    let tree = inspect(&ripple);
    assert_eq!(tree["slides"][0]["transition"]["type"], "ripple");
    assert_eq!(tree["slides"][0]["transition"]["dur"], 800);
}

#[test]
fn set_fade_then_clear() {
    let src = create_deck("set");
    rewrite_part(&src, "ppt/slides/slide1.xml", |data| {
        let xml = String::from_utf8(data).unwrap();
        inject_before_end(
            &xml,
            r#"<p:timing><p:tnLst><p:par><p:cTn id="1" dur="indefinite"/></p:par></p:tnLst></p:timing>"#,
        )
        .into_bytes()
    });
    let faded = tmp("faded");
    apply(
        &src,
        &faded,
        json!([{ "op": "setTransition", "id": "slide:1", "type": "fade" }]),
    )
    .unwrap();
    let tree = inspect(&faded);
    assert_eq!(tree["slides"][0]["transition"]["type"], "fade");
    assert_eq!(tree["slides"][0]["transition"]["dur"], 500);
    assert_eq!(tree["slides"][1]["transition"], Value::Null);
    let xml = part(&faded, "ppt/slides/slide1.xml");
    assert!(xml.contains("<p:fade"));
    assert!(xml.contains("p14:dur=\"500\""));
    assert!(xml.contains("<p:timing"));

    let slow = tmp("slow");
    apply(
        &faded,
        &slow,
        json!([{ "op": "setTransition", "id": "slide:1", "type": "fade", "dur": 2000 }]),
    )
    .unwrap();
    let tree = inspect(&slow);
    assert_eq!(tree["slides"][0]["transition"]["dur"], 2000);
    let xml = part(&slow, "ppt/slides/slide1.xml");
    assert!(xml.contains("spd=\"slow\""));

    let cleared = tmp("cleared");
    apply(
        &slow,
        &cleared,
        json!([{ "op": "setTransition", "id": "slide:1", "type": null }]),
    )
    .unwrap();
    let tree = inspect(&cleared);
    assert_eq!(tree["slides"][0]["transition"], Value::Null);
    let xml = part(&cleared, "ppt/slides/slide1.xml");
    assert!(!xml.contains("<p:transition"));
    assert!(xml.contains("<p:timing"));

    let again = tmp("again");
    apply(
        &cleared,
        &again,
        json!([{ "op": "setTransition", "id": "slide:1", "type": null }]),
    )
    .unwrap();
    assert_eq!(inspect(&again)["slides"][0]["transition"], Value::Null);
}

#[test]
fn set_ripple_uses_alternate_content() {
    let src = create_deck("ripple");
    let out = tmp("ripple-out");
    apply(
        &src,
        &out,
        json!([{ "op": "setTransition", "id": "slide:1", "type": "ripple", "dur": 900 }]),
    )
    .unwrap();
    let tree = inspect(&out);
    assert_eq!(tree["slides"][0]["transition"]["type"], "ripple");
    assert_eq!(tree["slides"][0]["transition"]["dur"], 900);
    let xml = part(&out, "ppt/slides/slide1.xml");
    assert!(xml.contains("p14:ripple"));
    assert!(xml.contains("AlternateContent"));
    assert!(xml.contains("<p:fade"));
}

#[test]
fn set_shape_text_keeps_transition_and_timing() {
    let src = create_deck("keep");
    rewrite_part(&src, "ppt/slides/slide1.xml", |data| {
        let xml = with_p14(&String::from_utf8(data).unwrap());
        inject_before_end(
            &xml,
            concat!(
                r#"<p:transition spd="fast" p14:dur="500"><p:fade/></p:transition>"#,
                r#"<p:timing><p:tnLst><p:par><p:cTn id="1" dur="indefinite">"#,
                r#"<p:childTnLst>"#,
                r#"<p:set><p:cBhvr><p:cTn id="2"/><p:tgtEl><p:spTgt spid="2"/></p:tgtEl></p:cBhvr></p:set>"#,
                r#"</p:childTnLst></p:cTn></p:par></p:tnLst></p:timing>"#
            ),
        )
        .into_bytes()
    });
    let tree0 = inspect_scope(&src, json!({ "slide": 1 }));
    let id = tree0["slides"][0]["shapes"][0]["id"].as_str().unwrap();
    let out = tmp("keep-out");
    apply(
        &src,
        &out,
        json!([{ "op": "setShapeText", "id": id, "text": "改过的正文" }]),
    )
    .unwrap();
    let tree = inspect_scope(&out, json!({ "slide": 1 }));
    assert_eq!(tree["slides"][0]["transition"]["type"], "fade");
    assert_eq!(tree["slides"][0]["shapes"][0]["text"], "改过的正文");
    let xml = part(&out, "ppt/slides/slide1.xml");
    assert!(xml.contains("<p:transition"));
    assert!(xml.contains("<p:fade"));
    assert!(xml.contains("<p:timing>"));
    assert!(xml.contains("spid=\"2\""));
}

#[test]
fn wrong_ids_and_types() {
    let src = create_deck("bad");
    let dest = tmp("bad-id");
    let err = apply(
        &src,
        &dest,
        json!([{ "op": "setTransition", "id": "slide:1/shape:2", "type": "fade" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    assert!(!dest.exists());

    let dest = tmp("bad-other");
    let err = apply(
        &src,
        &dest,
        json!([{ "op": "setTransition", "id": "slide:1", "type": "other" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!dest.exists());

    let dest = tmp("bad-empty");
    let err = apply(
        &src,
        &dest,
        json!([{ "op": "setTransition", "id": "slide:1", "type": "" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!dest.exists());

    let dest = tmp("bad-dur");
    let err = apply(
        &src,
        &dest,
        json!([{ "op": "setTransition", "id": "slide:1", "type": "fade", "dur": 0 }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!dest.exists());

    let dest = tmp("missing-type");
    let err = apply(
        &src,
        &dest,
        json!([{ "op": "setTransition", "id": "slide:1" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!dest.exists());
}

#[test]
fn create_rejects_transition_field() {
    let path = tmp("create-tn");
    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "presentation": {
                "slides": [{ "shapes": [{ "text": "a" }], "transition": { "type": "fade" } }]
            }
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!path.exists());
}

#[test]
fn docx_and_xlsx_reject_set_transition() {
    let docx = tmp("d").with_extension("docx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": docx.to_string_lossy(),
            "document": { "paragraphs": [{ "text": "a" }] }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let dest = tmp("d-out").with_extension("docx");
    let err = apply(
        &docx,
        &dest,
        json!([{ "op": "setTransition", "id": "slide:1", "type": "fade" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!dest.exists());

    let xlsx = tmp("x").with_extension("xlsx");
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
    let dest = tmp("x-out").with_extension("xlsx");
    let err = apply(
        &xlsx,
        &dest,
        json!([{ "op": "setTransition", "id": "slide:1", "type": "fade" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!dest.exists());
}

#[test]
fn inspect_auto_dir_and_prst() {
    let auto = create_deck("auto-src");
    rewrite_part(&auto, "ppt/slides/slide1.xml", |data| {
        let xml = String::from_utf8(data).unwrap();
        inject_before_end(
            &xml,
            r#"<p:transition advClick="0" advTm="3000"><p:wipe dir="r"/></p:transition>"#,
        )
        .into_bytes()
    });
    let tree = inspect(&auto);
    let tr = &tree["slides"][0]["transition"];
    assert_eq!(tr["type"], "wipe");
    assert_eq!(tr["advClick"], false);
    assert_eq!(tr["advTm"], 3000);
    assert_eq!(tr["dir"], "r");

    let prst = create_deck("prst-src");
    rewrite_part(&prst, "ppt/slides/slide1.xml", |data| {
        let xml = with_p14(&String::from_utf8(data).unwrap()).replace(
            r#"xmlns:p14="http://schemas.microsoft.com/office/powerpoint/2010/main""#,
            concat!(
                r#"xmlns:p14="http://schemas.microsoft.com/office/powerpoint/2010/main" "#,
                r#"xmlns:p15="http://schemas.microsoft.com/office/powerpoint/2012/main""#
            ),
        );
        inject_before_end(
            &xml,
            concat!(
                r#"<mc:AlternateContent xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006">"#,
                r#"<mc:Choice xmlns:p15="http://schemas.microsoft.com/office/powerpoint/2012/main" Requires="p15">"#,
                r#"<p:transition><p15:prstTrans prst="fallOver" invX="1"/></p:transition>"#,
                r#"</mc:Choice>"#,
                r#"<mc:Fallback><p:transition><p:fade/></p:transition></mc:Fallback>"#,
                r#"</mc:AlternateContent>"#
            ),
        )
        .into_bytes()
    });
    let tree = inspect(&prst);
    let tr = &tree["slides"][0]["transition"];
    assert_eq!(tr["type"], "fallOver");
    assert_eq!(tr["invX"], true);
    assert!(tr.get("invY").is_none());
}

#[test]
fn set_auto_wipe_and_fall_over() {
    let src = create_deck("remain-set");
    let auto = tmp("remain-auto");
    apply(
        &src,
        &auto,
        json!([{ "op": "setTransition", "id": "slide:1", "type": "fade", "advTm": 2000 }]),
    )
    .unwrap();
    let tr = &inspect(&auto)["slides"][0]["transition"];
    assert_eq!(tr["type"], "fade");
    assert_eq!(tr["advClick"], true);
    assert_eq!(tr["advTm"], 2000);
    let xml = part(&auto, "ppt/slides/slide1.xml");
    assert!(xml.contains("advTm=\"2000\""));
    assert!(!xml.contains("advClick"));

    let wipe = tmp("remain-wipe");
    apply(
        &src,
        &wipe,
        json!([{ "op": "setTransition", "id": "slide:1", "type": "wipe", "dir": "r" }]),
    )
    .unwrap();
    let tr = &inspect(&wipe)["slides"][0]["transition"];
    assert_eq!(tr["type"], "wipe");
    assert_eq!(tr["dir"], "r");
    assert_eq!(tr["advTm"], Value::Null);
    let xml = part(&wipe, "ppt/slides/slide1.xml");
    assert!(xml.contains("<p:wipe dir=\"r\""));

    let fall = tmp("remain-fall");
    apply(
        &src,
        &fall,
        json!([{ "op": "setTransition", "id": "slide:1", "type": "fallOver", "invX": true }]),
    )
    .unwrap();
    let tr = &inspect(&fall)["slides"][0]["transition"];
    assert_eq!(tr["type"], "fallOver");
    assert_eq!(tr["invX"], true);
    let xml = part(&fall, "ppt/slides/slide1.xml");
    assert!(xml.contains("p15:prstTrans"));
    assert!(xml.contains("prst=\"fallOver\""));
    assert!(xml.contains("invX=\"1\""));
    assert!(xml.contains("AlternateContent"));
}

#[test]
fn reject_dir_and_inv_on_fade() {
    let src = create_deck("remain-bad");
    let dest = tmp("remain-dir");
    let err = apply(
        &src,
        &dest,
        json!([{ "op": "setTransition", "id": "slide:1", "type": "fade", "dir": "r" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!dest.exists());

    let dest = tmp("remain-inv");
    let err = apply(
        &src,
        &dest,
        json!([{ "op": "setTransition", "id": "slide:1", "type": "fade", "invX": true }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!dest.exists());

    let dest = tmp("remain-adv");
    let err = apply(
        &src,
        &dest,
        json!([{ "op": "setTransition", "id": "slide:1", "type": "fade", "advTm": 0 }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!dest.exists());
}
