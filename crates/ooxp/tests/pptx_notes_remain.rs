//! 备注页图表 / SmartArt，以及批注内图 exportMedia。

use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

const PNG_1X1: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
    0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08, 0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00,
    0x00, 0x03, 0x01, 0x01, 0x00, 0x18, 0xDD, 0x8D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E,
    0x44, 0xAE, 0x42, 0x60, 0x82,
];

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-pptx-notes-remain");
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
                "slides": [{ "shapes": [{ "text": "标题" }] }]
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

fn nums(v: &Value) -> Vec<i64> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_i64().or_else(|| x.as_f64().map(|f| f as i64)).unwrap())
        .collect()
}

fn part_names(path: &Path) -> Vec<String> {
    let bytes = std::fs::read(path).unwrap();
    let zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    zip.file_names().map(str::to_string).collect()
}

fn part_bytes(path: &Path, name: &str) -> Vec<u8> {
    let bytes = std::fs::read(path).unwrap();
    let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut file = zip.by_name(name).unwrap();
    let mut out = Vec::new();
    file.read_to_end(&mut out).unwrap();
    out
}

fn upsert_zip(path: &Path, replace: &[(&str, Vec<u8>)], extra: &[(&str, Vec<u8>)]) {
    let bytes = std::fs::read(path).unwrap();
    let mut zip_in = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut out = Cursor::new(Vec::new());
    {
        let mut zip_out = ZipWriter::new(&mut out);
        let opt = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        let mut seen = std::collections::HashSet::new();
        for i in 0..zip_in.len() {
            let mut file = zip_in.by_index(i).unwrap();
            let n = file.name().to_string();
            let mut data = Vec::new();
            file.read_to_end(&mut data).unwrap();
            if let Some((_, neu)) = replace.iter().find(|(k, _)| *k == n) {
                data = neu.clone();
            }
            zip_out.start_file(&n, opt).unwrap();
            zip_out.write_all(&data).unwrap();
            seen.insert(n);
        }
        for (n, data) in extra {
            if seen.contains(*n) {
                continue;
            }
            zip_out.start_file(*n, opt).unwrap();
            zip_out.write_all(data).unwrap();
        }
        zip_out.finish().unwrap();
    }
    std::fs::write(path, out.into_inner()).unwrap();
}

#[test]
fn notes_insert_chart_not_counted_on_slide() {
    let src = create_deck("chart");
    let out = tmp("chart-ins").with_extension("pptx");
    apply(
        &src,
        &out,
        json!([{
            "op": "insertChart",
            "id": "slide:1/notes",
            "type": "col",
            "title": "备注柱",
            "series": [{
                "name": "系列甲",
                "categories": ["甲", "乙", "丙"],
                "values": [1, 2, 3]
            }]
        }]),
    )
    .unwrap();

    let tree = inspect(&out, json!({ "notes": true, "charts": true }));
    assert_eq!(tree["chartCount"], 0);
    assert_eq!(tree["slides"][0]["chartCount"], 0);
    assert!(tree["slides"][0]
        .get("charts")
        .unwrap()
        .as_array()
        .unwrap()
        .is_empty());
    let notes = &tree["slides"][0]["notes"];
    assert_eq!(notes["chartCount"], 1);
    let id = notes["charts"][0]["id"].as_str().unwrap();
    assert!(id.starts_with("slide:1/notes/chart:"), "{id}");
    assert_eq!(notes["charts"][0]["type"], "col");
    assert_eq!(
        nums(&notes["charts"][0]["series"][0]["values"]),
        vec![1, 2, 3]
    );
    let ser = format!("{id}/series:0");

    let names = part_names(&out);
    assert!(names.iter().any(|n| n.starts_with("ppt/notesSlides/")));
    assert!(names.iter().any(|n| n.starts_with("ppt/charts/")));

    let updated = tmp("chart-ser").with_extension("pptx");
    apply(
        &out,
        &updated,
        json!([{ "op": "setChartSeries", "id": ser, "values": [10, 20, 30] }]),
    )
    .unwrap();
    let after = inspect(&updated, json!({ "notes": true, "charts": true }));
    assert_eq!(after["chartCount"], 0);
    assert_eq!(
        nums(&after["slides"][0]["notes"]["charts"][0]["series"][0]["values"]),
        vec![10, 20, 30]
    );

    let embed_name = names
        .iter()
        .find(|n| n.starts_with("ppt/embeddings/") && n.ends_with(".xlsx"))
        .cloned();
    let exported = tmp("chart-exp").with_extension("xlsx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "exportChartData",
            "path": updated.to_string_lossy(),
            "id": id,
            "output": exported.to_string_lossy()
        })
        .to_string(),
        None,
    )
    .unwrap();
    assert!(exported.exists());
    let _ = embed_name;
}

#[test]
fn notes_insert_smartart_not_counted_on_slide() {
    let src = create_deck("sa");
    let out = tmp("sa-ins").with_extension("pptx");
    apply(
        &src,
        &out,
        json!([{
            "op": "insertSmartArt",
            "id": "slide:1/notes",
            "layout": "list",
            "points": [
                { "text": "甲" },
                { "text": "乙" },
                { "text": "丙" }
            ]
        }]),
    )
    .unwrap();
    let tree = inspect(&out, json!({ "notes": true, "smartArt": true }));
    assert_eq!(tree["smartArtCount"], 0);
    assert_eq!(tree["slides"][0]["smartArtCount"], 0);
    let notes = &tree["slides"][0]["notes"];
    assert_eq!(notes["smartArtCount"], 1);
    let id = notes["smartArts"][0]["id"].as_str().unwrap();
    assert!(id.starts_with("slide:1/notes/smartdiagram:"), "{id}");
    assert_eq!(notes["smartArts"][0]["points"].as_array().unwrap().len(), 3);
}

#[test]
fn comment_blip_inspect_export_and_reject_insert() {
    let src = create_deck("cm");
    let with_cm = tmp("cm-ins").with_extension("pptx");
    apply(
        &src,
        &with_cm,
        json!([{
            "op": "insertComment",
            "id": "slide:1",
            "text": "带图批注",
            "author": "Alice",
            "initials": "A",
            "date": "2026-09-01T00:00:00.000",
            "x": 914400,
            "y": 914400
        }]),
    )
    .unwrap();

    let names = part_names(&with_cm);
    let comments = names
        .iter()
        .find(|n| n.starts_with("ppt/comments/") && n.ends_with(".xml") && !n.contains("/_rels/"))
        .cloned()
        .expect("应有现代批注部件");
    let rels = {
        let (dir, file) = comments.rsplit_once('/').unwrap();
        format!("{dir}/_rels/{file}.rels")
    };
    let mut xml = String::from_utf8(part_bytes(&with_cm, &comments)).unwrap();
    assert!(xml.contains("</p188:cm>"));
    xml = xml.replace("</p188:cm>", r#"<a:blip r:embed="rId1"/></p188:cm>"#);
    let mut cts = String::from_utf8(part_bytes(&with_cm, "[Content_Types].xml")).unwrap();
    if !cts.contains(r#"Extension="png""#) {
        cts = cts.replace(
            "<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\">",
            "<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"png\" ContentType=\"image/png\"/>",
        );
    }
    let rels_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/image1.png"/></Relationships>"#;
    upsert_zip(
        &with_cm,
        &[
            (comments.as_str(), xml.into_bytes()),
            ("[Content_Types].xml", cts.into_bytes()),
        ],
        &[
            ("ppt/media/image1.png", PNG_1X1.to_vec()),
            (rels.as_str(), rels_xml.as_bytes().to_vec()),
        ],
    );

    let tree = inspect(&with_cm, json!({ "comments": true, "pictures": true }));
    assert_eq!(tree["pictureCount"], 0);
    let c = &tree["slides"][0]["comments"][0];
    assert_eq!(c["pictureCount"], 1);
    let pic_id = c["pictures"][0]["id"].as_str().unwrap().to_string();
    let cid = c["id"].as_str().unwrap().to_string();
    assert_eq!(pic_id, format!("{cid}/pic:1"));

    let exported = tmp("cm-pic").with_extension("png");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "exportMedia",
            "path": with_cm.to_string_lossy(),
            "id": pic_id,
            "output": exported.to_string_lossy()
        })
        .to_string(),
        None,
    )
    .unwrap();
    assert_eq!(std::fs::read(&exported).unwrap(), PNG_1X1);

    let err = apply(
        &with_cm,
        &tmp("cm-ins-pic").with_extension("pptx"),
        json!([{
            "op": "insertPicture",
            "id": cid,
            "file": exported.to_string_lossy()
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);

    let err = apply(
        &with_cm,
        &tmp("cm-rep").with_extension("pptx"),
        json!([{
            "op": "replacePicture",
            "id": pic_id,
            "file": exported.to_string_lossy()
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
}
