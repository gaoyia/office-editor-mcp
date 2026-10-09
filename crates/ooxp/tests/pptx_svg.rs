//! pptx 嵌入 SVG：inspect contentType、insert/replace/export/create、无 PNG 后备也能导出。

use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

static SEQ: AtomicU32 = AtomicU32::new(0);

const SVG_96X48: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="96" height="48" viewBox="0 0 96 48"><rect width="96" height="48" fill="#3366cc"/><circle cx="48" cy="24" r="16" fill="#ffcc00"/></svg>"##;

const PNG_1X1: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
    0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08, 0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00,
    0x00, 0x03, 0x01, 0x01, 0x00, 0x18, 0xDD, 0x8D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E,
    0x44, 0xAE, 0x42, 0x60, 0x82,
];

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-pptx-svg");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn seq_path(tag: &str, ext: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}.{ext}"))
}

fn write_svg(tag: &str) -> PathBuf {
    let path = seq_path(tag, "svg");
    std::fs::write(&path, SVG_96X48.as_bytes()).unwrap();
    path
}

fn write_png(tag: &str) -> PathBuf {
    let path = seq_path(tag, "png");
    std::fs::write(&path, PNG_1X1).unwrap();
    path
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

fn inspect_pics(path: &Path) -> Value {
    let RunOutput::Inspect(tree) = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": path.to_string_lossy(),
            "scope": { "pictures": true, "notes": true }
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
    let out = seq_path("should-not-write", "pptx");
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

fn export_media(path: &Path, id: &str, output: &Path) -> RunOutput {
    run(
        &json!({
            "schemaVersion": 1,
            "action": "exportMedia",
            "path": path.to_string_lossy(),
            "id": id,
            "output": output.to_string_lossy()
        })
        .to_string(),
        None,
    )
    .unwrap()
}

fn part_text(path: &Path, name: &str) -> String {
    let bytes = std::fs::read(path).unwrap();
    let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut file = zip.by_name(name.trim_start_matches('/')).unwrap();
    let mut s = String::new();
    file.read_to_string(&mut s).unwrap();
    s
}

fn zip_names(path: &Path) -> Vec<String> {
    let bytes = std::fs::read(path).unwrap();
    let zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    zip.file_names().map(|s| s.to_string()).collect()
}

fn first_pic(tree: &Value) -> &Value {
    &tree["slides"][0]["pictures"][0]
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

fn strip_blip_embed(xml: &str) -> String {
    let start = xml.find("<a:blip").expect("a:blip");
    let after = &xml[start..];
    let tag_end = after.find('>').expect(">");
    let tag = &after[..tag_end];
    let key = "r:embed=\"";
    let e = tag.find(key).expect("a:blip r:embed");
    let val_start = start + e;
    let after_eq = &xml[val_start + key.len()..];
    let q = after_eq.find('"').expect("quote");
    let mut s = xml.to_string();
    s.replace_range(val_start..val_start + key.len() + q + 1, "");
    s
}

#[test]
fn create_svg_writes_svgblip_and_inspect_content_type() {
    let svg = write_svg("create");
    let path = create_deck(
        "create-svg",
        json!([{
            "shapes": [{ "text": "标题" }],
            "pictures": [{ "file": svg.to_string_lossy() }]
        }]),
    );
    let tree = inspect_pics(&path);
    let pic = first_pic(&tree);
    assert_eq!(pic["contentType"], "image/svg+xml");
    assert_eq!(pic["cx"], 914400);
    assert_eq!(pic["cy"], 457200);
    let keys: Vec<&str> = pic
        .as_object()
        .unwrap()
        .keys()
        .map(|k| k.as_str())
        .collect();
    assert_eq!(
        keys,
        vec![
            "id",
            "cNvPrId",
            "name",
            "x",
            "y",
            "cx",
            "cy",
            "stable",
            "contentType"
        ]
    );
    let names = zip_names(&path);
    assert!(names.iter().any(|n| n.ends_with(".svg")), "{names:?}");
    assert!(names.iter().any(|n| n.ends_with(".png")), "{names:?}");
    let slide = part_text(&path, "ppt/slides/slide1.xml");
    assert!(slide.contains("svgBlip"), "{slide}");
    assert!(
        slide.contains("{96DAC541-7B7A-43D3-8B79-37D633B846F1}"),
        "{slide}"
    );
    let ct = part_text(&path, "[Content_Types].xml");
    assert!(ct.contains("image/svg+xml"), "{ct}");
}

#[test]
fn export_media_prefers_svg_bytes() {
    let svg = write_svg("export");
    let path = create_deck(
        "export-svg",
        json!([{ "pictures": [{ "file": svg.to_string_lossy() }] }]),
    );
    let tree = inspect_pics(&path);
    let id = first_pic(&tree)["id"].as_str().unwrap();
    let out = seq_path("export", "svg");
    let result = export_media(&path, id, &out);
    let RunOutput::Written { content_type, .. } = result else {
        panic!("exportMedia 应写出文件");
    };
    assert_eq!(content_type.as_deref(), Some("image/svg+xml"));
    assert_eq!(std::fs::read(&out).unwrap(), SVG_96X48.as_bytes());
}

#[test]
fn export_media_pptx_native_svg_without_png_embed() {
    let svg = write_svg("native");
    let path = create_deck(
        "native-svg",
        json!([{ "pictures": [{ "file": svg.to_string_lossy() }] }]),
    );
    rewrite_part(&path, "ppt/slides/slide1.xml", |data| {
        let xml = String::from_utf8(data).unwrap();
        strip_blip_embed(&xml).into_bytes()
    });
    let tree = inspect_pics(&path);
    let pic = first_pic(&tree);
    assert_eq!(pic["contentType"], "image/svg+xml");
    let id = pic["id"].as_str().unwrap();
    let out = seq_path("native-export", "svg");
    let result = export_media(&path, id, &out);
    let RunOutput::Written { content_type, .. } = result else {
        panic!("exportMedia 应写出文件");
    };
    assert_eq!(content_type.as_deref(), Some("image/svg+xml"));
    assert_eq!(std::fs::read(&out).unwrap(), SVG_96X48.as_bytes());
}

#[test]
fn insert_svg_then_replace_with_png_strips_svgblip() {
    let src = create_deck("insert-src", json!([{ "shapes": [{ "text": "标题" }] }]));
    let svg = write_svg("insert");
    let inserted = seq_path("inserted", "pptx");
    apply(
        &src,
        &inserted,
        json!([{
            "op": "insertPicture",
            "id": "slide:1",
            "file": svg.to_string_lossy()
        }]),
    );
    let tree1 = inspect_pics(&inserted);
    let pic = first_pic(&tree1);
    assert_eq!(pic["contentType"], "image/svg+xml");
    let pic_id = pic["id"].as_str().unwrap().to_string();

    let png = write_png("replace");
    let replaced = seq_path("replaced", "pptx");
    apply(
        &inserted,
        &replaced,
        json!([{ "op": "replacePicture", "id": pic_id, "file": png.to_string_lossy() }]),
    );
    let tree2 = inspect_pics(&replaced);
    let pic2 = first_pic(&tree2);
    assert!(pic2.get("contentType").is_none(), "{pic2}");
    let slide = part_text(&replaced, "ppt/slides/slide1.xml");
    assert!(!slide.contains("svgBlip"), "{slide}");
    let names = zip_names(&replaced);
    assert!(!names.iter().any(|n| n.ends_with(".svg")), "{names:?}");
}

#[test]
fn delete_svg_removes_both_media() {
    let svg = write_svg("del");
    let src = create_deck(
        "del-src",
        json!([{ "pictures": [{ "file": svg.to_string_lossy() }] }]),
    );
    let tree = inspect_pics(&src);
    let id = first_pic(&tree)["id"].as_str().unwrap().to_string();
    let out = seq_path("deleted", "pptx");
    apply(&src, &out, json!([{ "op": "deletePicture", "id": id }]));
    let tree2 = inspect_pics(&out);
    assert_eq!(tree2["pictureCount"], 0);
    let names = zip_names(&out);
    assert!(
        !names.iter().any(|n| n.contains("ppt/media/image")),
        "{names:?}"
    );
}

#[test]
fn replace_png_with_svg_adds_svgblip() {
    let png = write_png("host");
    let src = create_deck(
        "png-host",
        json!([{ "pictures": [{ "file": png.to_string_lossy() }] }]),
    );
    let tree = inspect_pics(&src);
    let pic = first_pic(&tree);
    assert!(pic.get("contentType").is_none(), "{pic}");
    let pic_id = pic["id"].as_str().unwrap().to_string();
    let svg = write_svg("as-svg");
    let out = seq_path("png-to-svg", "pptx");
    apply(
        &src,
        &out,
        json!([{ "op": "replacePicture", "id": pic_id, "file": svg.to_string_lossy() }]),
    );
    let tree2 = inspect_pics(&out);
    let pic2 = first_pic(&tree2);
    assert_eq!(pic2["contentType"], "image/svg+xml");
    let names = zip_names(&out);
    assert!(names.iter().any(|n| n.ends_with(".svg")), "{names:?}");
}

#[test]
fn notes_svg_has_content_type() {
    let src = create_deck("notes-src", json!([{ "shapes": [{ "text": "标题" }] }]));
    let svg = write_svg("notes");
    let out = seq_path("notes", "pptx");
    apply(
        &src,
        &out,
        json!([{
            "op": "insertPicture",
            "id": "slide:1/notes",
            "file": svg.to_string_lossy()
        }]),
    );
    let tree = inspect_pics(&out);
    let pic = &tree["slides"][0]["notes"]["pictures"][0];
    assert_eq!(pic["contentType"], "image/svg+xml");
}

#[test]
fn reject_not_an_image() {
    let src = create_deck("rej-src", json!([{ "shapes": [{ "text": "标题" }] }]));
    let junk = seq_path("not-image", "bin");
    std::fs::write(&junk, b"not an image").unwrap();
    let err = apply_err(
        &src,
        json!([{
            "op": "insertPicture",
            "id": "slide:1",
            "file": junk.to_string_lossy()
        }]),
    );
    assert_eq!(err.code(), ErrorCode::InvalidField);
}
