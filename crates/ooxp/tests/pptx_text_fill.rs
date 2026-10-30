//! 形状字填充与轮廓：渐变字、字轮廓、混排 runs。

use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-pptx-text-fill");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn tmp(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}.pptx"))
}

fn create_deck(tag: &str, text: &str) -> PathBuf {
    let path = tmp(tag);
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "presentation": {
            "slides": [
                { "shapes": [{ "text": text }] }
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

fn inspect_slide(path: &Path) -> Value {
    let RunOutput::Inspect(tree) = run(&json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": path.to_string_lossy(),
        "scope": { "slide": 1 }
    })
    .to_string())
    .unwrap() else {
        panic!("inspect 应返回树");
    };
    tree
}

fn part(path: &Path, name: &str) -> String {
    let bytes = std::fs::read(path).unwrap();
    let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut file = zip.by_name(name).unwrap();
    let mut out = Vec::new();
    file.read_to_end(&mut out).unwrap();
    String::from_utf8(out).unwrap()
}

fn rewrite_part(path: &Path, name: &str, f: impl FnOnce(String) -> String) {
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
                let s = String::from_utf8(data).unwrap();
                data = transform.take().expect("部件只应出现一次")(s).into_bytes();
            }
            zip_out.start_file(&n, opt).unwrap();
            zip_out.write_all(&data).unwrap();
        }
        zip_out.finish().unwrap();
    }
    std::fs::write(path, out.into_inner()).unwrap();
}

fn replace_nth(s: &str, from: &str, to: &str, nth: usize) -> String {
    let mut count = 0;
    let mut out = String::new();
    let mut rest = s;
    while let Some(i) = rest.find(from) {
        out.push_str(&rest[..i]);
        if count == nth {
            out.push_str(to);
        } else {
            out.push_str(from);
        }
        rest = &rest[i + from.len()..];
        count += 1;
    }
    out.push_str(rest);
    out
}

fn gradient_fill() -> Value {
    json!({
        "type": "gradient",
        "ang": 5400000,
        "stops": [
            { "pos": 0, "color": "FF0000" },
            { "pos": 100000, "color": "0000FF" }
        ]
    })
}

#[test]
fn set_gradient_fill_inspects_and_writes_skeleton() {
    let src = create_deck("grad", "标题");
    let out = tmp("grad-out");
    apply(
        &src,
        &out,
        json!([{
            "op": "setShapeFormat",
            "id": "slide:1/shape:2",
            "textFill": gradient_fill()
        }]),
    )
    .unwrap();
    let sh = &inspect_slide(&out)["slides"][0]["shapes"][0];
    assert_eq!(sh["color"], Value::Null);
    assert_eq!(sh["textFill"]["type"], "gradient");
    assert_eq!(sh["textFill"]["ang"], 5400000);
    assert_eq!(sh["textFill"]["stops"][0]["pos"], 0);
    assert_eq!(sh["textFill"]["stops"][0]["color"], "FF0000");
    assert_eq!(sh["textFill"]["stops"][1]["pos"], 100000);
    assert_eq!(sh["textFill"]["stops"][1]["color"], "0000FF");
    let xml = part(&out, "ppt/slides/slide1.xml");
    assert!(xml.contains(r#"<a:gradFill flip="none" rotWithShape="1">"#));
    assert!(xml.contains(r#"<a:gs pos="0"><a:srgbClr val="FF0000"/></a:gs>"#));
    assert!(xml.contains(r#"<a:gs pos="100000"><a:srgbClr val="0000FF"/></a:gs>"#));
    assert!(xml.contains(r#"<a:lin ang="5400000" scaled="1"/>"#));
    assert!(xml.contains("<a:tileRect/>"));
    assert!(!xml.contains("<a:path"));
}

#[test]
fn set_text_outline_writes_ln_before_fill() {
    let src = create_deck("ln", "标题");
    let out = tmp("ln-out");
    apply(
        &src,
        &out,
        json!([{
            "op": "setShapeFormat",
            "id": "slide:1/shape:2",
            "textOutline": { "color": "FF0000", "width": 19050 }
        }]),
    )
    .unwrap();
    let sh = &inspect_slide(&out)["slides"][0]["shapes"][0];
    assert_eq!(sh["textOutline"]["color"], "FF0000");
    assert_eq!(sh["textOutline"]["width"], 19050);
    let xml = part(&out, "ppt/slides/slide1.xml");
    let rpr = xml
        .split("<a:rPr")
        .nth(1)
        .expect("有 rPr")
        .split("</a:rPr>")
        .next()
        .unwrap();
    let ln = rpr.find("<a:ln w=\"19050\">").expect("有字轮廓");
    let fill = rpr.find("<a:solidFill>").expect("有填色");
    assert!(ln < fill);
}

#[test]
fn text_fill_none_clears_color() {
    let src = create_deck("none", "标题");
    let out = tmp("none-out");
    apply(
        &src,
        &out,
        json!([{
            "op": "setShapeFormat",
            "id": "slide:1/shape:2",
            "textFill": { "type": "none" }
        }]),
    )
    .unwrap();
    let sh = &inspect_slide(&out)["slides"][0]["shapes"][0];
    assert_eq!(sh["color"], Value::Null);
    assert_eq!(sh["textFill"]["type"], "none");
    let xml = part(&out, "ppt/slides/slide1.xml");
    assert!(xml.contains("<a:noFill/>"));
}

#[test]
fn mixed_run_sizes_list_runs() {
    let path = create_deck("mix", "大\n小");
    rewrite_part(&path, "ppt/slides/slide1.xml", |xml| {
        let once = replace_nth(&xml, r#"sz="1800""#, r#"sz="2800""#, 0);
        replace_nth(&once, r#"sz="1800""#, r#"sz="1200""#, 0)
    });
    let sh = &inspect_slide(&path)["slides"][0]["shapes"][0];
    assert_eq!(sh["sz"], Value::Null);
    let runs = sh["runs"].as_array().unwrap();
    assert_eq!(runs.len(), 2);
    assert_eq!(runs[0]["text"], "大");
    assert_eq!(runs[0]["sz"], 2800);
    assert_eq!(runs[1]["text"], "小");
    assert_eq!(runs[1]["sz"], 1200);
}

#[test]
fn create_gradient_skips_default_black() {
    let path = tmp("create-grad");
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "presentation": {
            "slides": [{
                "shapes": [{
                    "text": "渐变",
                    "textFill": gradient_fill()
                }]
            }]
        }
    })
    .to_string())
    .unwrap();
    let sh = &inspect_slide(&path)["slides"][0]["shapes"][0];
    assert_eq!(sh["color"], Value::Null);
    assert_eq!(sh["textFill"]["type"], "gradient");
    let xml = part(&path, "ppt/slides/slide1.xml");
    assert!(xml.contains(r#"<a:gradFill flip="none" rotWithShape="1">"#));
    let rpr = xml
        .split("<a:rPr")
        .nth(1)
        .unwrap()
        .split("</a:rPr>")
        .next()
        .unwrap();
    assert!(!rpr.contains(r#"val="000000""#));
}

#[test]
fn rejects_color_with_text_fill_and_empty_op_and_docx() {
    let src = create_deck("bad", "标题");
    let dest = tmp("both");
    let err = apply(
        &src,
        &dest,
        json!([{
            "op": "setShapeFormat",
            "id": "slide:1/shape:2",
            "color": "FF0000",
            "textFill": { "type": "none" }
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!dest.exists());

    let dest = tmp("empty");
    let err = apply(
        &src,
        &dest,
        json!([{ "op": "setShapeFormat", "id": "slide:1/shape:2" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!dest.exists());

    let dest = tmp("solid");
    let err = apply(
        &src,
        &dest,
        json!([{
            "op": "setShapeFormat",
            "id": "slide:1/shape:2",
            "textFill": { "type": "solid" }
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!dest.exists());

    let docx = tmp("doc").with_extension("docx");
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": docx.to_string_lossy(),
        "document": { "paragraphs": [{ "text": "hi" }] }
    })
    .to_string())
    .unwrap();
    let err = run(&json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": docx.to_string_lossy(),
        "output": tmp("docx-out").with_extension("docx").to_string_lossy(),
        "ops": [{
            "op": "setShapeFormat",
            "id": "slide:1/shape:2",
            "textFill": { "type": "none" }
        }]
    })
    .to_string())
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}
