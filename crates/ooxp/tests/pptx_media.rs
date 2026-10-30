//! 幻灯片视频 / 音频：名单、插、删、导出、与图分家。

use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::json;
use zip::ZipArchive;

/// 只含 `ftyp` 的最小 MP4，库按文件头认，不解码。
const MP4_FTYP: &[u8] = &[
    0x00, 0x00, 0x00, 0x18, b'f', b't', b'y', b'p', b'i', b's', b'o', b'm', 0x00, 0x00, 0x00, 0x00,
    b'i', b's', b'o', b'm', b'm', b'p', b'4', b'1',
];

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-pptx-media");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn tmp(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}"))
}

fn tiny_wav() -> Vec<u8> {
    let data_len = 8u32;
    let mut v = Vec::new();
    v.extend_from_slice(b"RIFF");
    v.extend_from_slice(&(36u32 + data_len).to_le_bytes());
    v.extend_from_slice(b"WAVE");
    v.extend_from_slice(b"fmt ");
    v.extend_from_slice(&16u32.to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes());
    v.extend_from_slice(&8000u32.to_le_bytes());
    v.extend_from_slice(&16000u32.to_le_bytes());
    v.extend_from_slice(&2u16.to_le_bytes());
    v.extend_from_slice(&16u16.to_le_bytes());
    v.extend_from_slice(b"data");
    v.extend_from_slice(&data_len.to_le_bytes());
    v.extend_from_slice(&[0u8; 8]);
    v
}

fn write_mp4(tag: &str) -> PathBuf {
    let path = tmp(tag).with_extension("mp4");
    std::fs::write(&path, MP4_FTYP).unwrap();
    path
}

fn write_wav(tag: &str) -> PathBuf {
    let path = tmp(tag).with_extension("wav");
    std::fs::write(&path, tiny_wav()).unwrap();
    path
}

fn create_deck(tag: &str) -> PathBuf {
    let path = tmp(tag).with_extension("pptx");
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "presentation": { "slides": [{ "shapes": [{ "text": "标题" }] }] }
    })
    .to_string())
    .unwrap();
    path
}

fn apply(src: &Path, out: &Path, ops: serde_json::Value) -> Result<RunOutput, ooxp::Error> {
    run(&json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": out.to_string_lossy(),
        "ops": ops,
    })
    .to_string())
}

fn inspect_bare(path: &Path) -> serde_json::Value {
    let RunOutput::Inspect(tree) = run(&json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": path.to_string_lossy()
    })
    .to_string())
    .unwrap() else {
        panic!("inspect 应返回树");
    };
    tree
}

fn inspect_media(path: &Path) -> serde_json::Value {
    let RunOutput::Inspect(tree) = run(&json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": path.to_string_lossy(),
        "scope": { "media": true, "pictures": true }
    })
    .to_string())
    .unwrap() else {
        panic!("inspect 应返回树");
    };
    tree
}

fn object_keys(v: &serde_json::Value) -> Vec<&str> {
    v.as_object().unwrap().keys().map(String::as_str).collect()
}

fn part_bytes(path: &Path, name: &str) -> Vec<u8> {
    let bytes = std::fs::read(path).unwrap();
    let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut file = zip.by_name(name).unwrap();
    let mut out = Vec::new();
    file.read_to_end(&mut out).unwrap();
    out
}

fn part_names(path: &Path) -> Vec<String> {
    let bytes = std::fs::read(path).unwrap();
    let zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    zip.file_names().map(str::to_string).collect()
}

#[test]
fn inspect_media_count_without_list() {
    let path = create_deck("count");
    let tree = inspect_bare(&path);
    assert_eq!(tree["videoCount"], 0);
    assert_eq!(tree["audioCount"], 0);
    assert_eq!(tree["slides"][0]["videoCount"], 0);
    assert_eq!(tree["slides"][0]["audioCount"], 0);
    assert!(tree["slides"][0].get("videos").is_none());
    assert!(tree["slides"][0].get("audios").is_none());
}

#[test]
fn insert_video_inspect_export_delete() {
    let src = create_deck("v");
    let mp4 = write_mp4("clip");
    let inserted = tmp("ins").with_extension("pptx");
    apply(
        &src,
        &inserted,
        json!([{
            "op": "insertVideo",
            "id": "slide:1",
            "file": mp4.to_string_lossy()
        }]),
    )
    .unwrap();

    let bare = inspect_bare(&inserted);
    assert_eq!(bare["pictureCount"], 0);
    assert_eq!(bare["videoCount"], 1);
    assert_eq!(bare["audioCount"], 0);
    assert!(bare["slides"][0].get("videos").is_none());

    let tree = inspect_media(&inserted);
    assert_eq!(tree["pictureCount"], 0);
    assert_eq!(tree["slides"][0]["pictureCount"], 0);
    assert_eq!(tree["slides"][0]["pictures"], json!([]));
    let videos = tree["slides"][0]["videos"].as_array().unwrap();
    assert_eq!(videos.len(), 1);
    assert_eq!(videos[0]["id"], "slide:1/video:3");
    assert_eq!(videos[0]["embedded"], true);
    assert_eq!(videos[0]["fileName"], "media1.mp4");
    assert_eq!(videos[0]["contentType"], "video/mp4");
    assert_eq!(videos[0]["cx"], 3048000);
    assert_eq!(videos[0]["cy"], 2286000);
    assert_eq!(videos[0]["stable"], true);
    assert_eq!(videos[0]["st"], 0);
    assert_eq!(videos[0]["end"], 0);
    assert_eq!(videos[0]["bookmarks"], json!([]));
    assert_eq!(videos[0]["tracks"], json!([]));
    assert_eq!(
        object_keys(&videos[0]),
        [
            "id",
            "cNvPrId",
            "name",
            "x",
            "y",
            "cx",
            "cy",
            "embedded",
            "fileName",
            "contentType",
            "stable",
            "st",
            "end",
            "bookmarks",
            "tracks"
        ]
    );
    assert_eq!(tree["slides"][0]["audios"], json!([]));

    let xml = String::from_utf8(part_bytes(&inserted, "ppt/slides/slide1.xml")).unwrap();
    assert!(xml.contains("a:videoFile"));
    assert!(xml.contains("p14:media"));
    assert!(xml.contains("ppaction://media"));
    assert!(!xml.contains("<p:timing"));
    let rels =
        String::from_utf8(part_bytes(&inserted, "ppt/slides/_rels/slide1.xml.rels")).unwrap();
    assert!(rels.contains("/media"));
    assert!(rels.contains("/video"));
    assert!(rels.contains("/image"));
    assert_eq!(part_bytes(&inserted, "ppt/media/media1.mp4"), MP4_FTYP);
    assert!(part_names(&inserted)
        .iter()
        .any(|n| n == "ppt/media/image1.png"));

    let exported = tmp("exp").with_extension("mp4");
    run(&json!({
        "schemaVersion": 1,
        "action": "exportMedia",
        "path": inserted.to_string_lossy(),
        "id": "slide:1/video:3",
        "output": exported.to_string_lossy()
    })
    .to_string())
    .unwrap();
    assert_eq!(std::fs::read(&exported).unwrap(), MP4_FTYP);

    let deleted = tmp("del").with_extension("pptx");
    apply(
        &inserted,
        &deleted,
        json!([{ "op": "deleteMedia", "id": "slide:1/video:3" }]),
    )
    .unwrap();
    let after = inspect_media(&deleted);
    assert_eq!(after["videoCount"], 0);
    assert_eq!(after["slides"][0]["videos"], json!([]));
    let names = part_names(&deleted);
    assert!(!names.iter().any(|n| n.contains("media1.mp4")));
}

#[test]
fn insert_audio_and_wrong_file() {
    let src = create_deck("a");
    let wav = write_wav("beep");
    let inserted = tmp("ins-a").with_extension("pptx");
    apply(
        &src,
        &inserted,
        json!([{
            "op": "insertAudio",
            "id": "slide:1",
            "file": wav.to_string_lossy(),
            "x": 100,
            "y": 200
        }]),
    )
    .unwrap();
    let tree = inspect_media(&inserted);
    assert_eq!(tree["audioCount"], 1);
    assert_eq!(tree["videoCount"], 0);
    let audios = tree["slides"][0]["audios"].as_array().unwrap();
    assert_eq!(audios[0]["id"], "slide:1/audio:3");
    assert_eq!(audios[0]["embedded"], true);
    assert_eq!(audios[0]["fileName"], "media1.wav");
    assert_eq!(audios[0]["contentType"], "audio/x-wav");
    assert_eq!(audios[0]["cx"], 609600);
    assert_eq!(audios[0]["x"], 100);
    assert_eq!(audios[0]["y"], 200);
    let xml = String::from_utf8(part_bytes(&inserted, "ppt/slides/slide1.xml")).unwrap();
    assert!(xml.contains("a:audioFile"));
    assert!(!xml.contains("<p:timing"));

    let exported = tmp("exp-a").with_extension("wav");
    run(&json!({
        "schemaVersion": 1,
        "action": "exportMedia",
        "path": inserted.to_string_lossy(),
        "id": "slide:1/audio:3",
        "output": exported.to_string_lossy()
    })
    .to_string())
    .unwrap();
    assert_eq!(std::fs::read(&exported).unwrap(), tiny_wav());

    let mp4 = write_mp4("not-wav");
    let err = apply(
        &src,
        &tmp("bad-a").with_extension("pptx"),
        json!([{
            "op": "insertAudio",
            "id": "slide:1",
            "file": mp4.to_string_lossy()
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert_eq!(err.field_path(), Some("ops[0].file"));

    let err = apply(
        &src,
        &tmp("bad-v").with_extension("pptx"),
        json!([{
            "op": "insertVideo",
            "id": "slide:1",
            "file": wav.to_string_lossy()
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

#[test]
fn insert_rejects_notes_master_and_root_id() {
    let src = create_deck("bad-id");
    let mp4 = write_mp4("clip");
    let err = apply(
        &src,
        &tmp("notes-master").with_extension("pptx"),
        json!([{
            "op": "insertVideo",
            "id": "notesMaster",
            "file": mp4.to_string_lossy()
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);

    let err = apply(
        &src,
        &tmp("root").with_extension("pptx"),
        json!([{
            "op": "insertVideo",
            "id": "pptx:presentation",
            "file": mp4.to_string_lossy()
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);

    let err = apply(
        &src,
        &tmp("pic").with_extension("pptx"),
        json!([{
            "op": "deleteMedia",
            "id": "slide:1/pic:3"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
}

#[test]
fn xlsx_and_docx_reject_media_ops_and_scope() {
    let xlsx = tmp("wb").with_extension("xlsx");
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": xlsx.to_string_lossy(),
        "workbook": { "sheets": [{ "name": "Sheet1" }] }
    })
    .to_string())
    .unwrap();
    let mp4 = write_mp4("x");
    let err = apply(
        &xlsx,
        &tmp("x-out").with_extension("xlsx"),
        json!([{
            "op": "insertVideo",
            "id": "slide:1",
            "file": mp4.to_string_lossy()
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let err = run(&json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": xlsx.to_string_lossy(),
        "scope": { "media": true }
    })
    .to_string())
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

fn write_vtt(tag: &str) -> PathBuf {
    let p = tmp(tag).with_extension("vtt");
    std::fs::write(&p, "WEBVTT\n\n00:00:00.000 --> 00:00:00.800\nhello\n").unwrap();
    p
}

#[test]
fn set_media_trim_bookmarks_and_tracks() {
    let src = create_deck("trim");
    let mp4 = write_mp4("clip");
    let vtt = write_vtt("cap");
    let inserted = tmp("ins-t").with_extension("pptx");
    apply(
        &src,
        &inserted,
        json!([{
            "op": "insertVideo",
            "id": "slide:1",
            "file": mp4.to_string_lossy()
        }]),
    )
    .unwrap();
    let patched = tmp("set-t").with_extension("pptx");
    apply(
        &inserted,
        &patched,
        json!([{
            "op": "setMedia",
            "id": "slide:1/video:3",
            "st": 100,
            "end": 200,
            "bookmarks": [{ "name": "MarkA", "time": 300 }],
            "tracks": [{ "label": "English", "lang": "en", "file": vtt.to_string_lossy() }]
        }]),
    )
    .unwrap();
    let tree = inspect_media(&patched);
    let v = &tree["slides"][0]["videos"][0];
    assert_eq!(v["st"], 100);
    assert_eq!(v["end"], 200);
    assert_eq!(v["bookmarks"][0]["name"], "MarkA");
    assert_eq!(v["bookmarks"][0]["time"], 300);
    assert_eq!(v["tracks"][0]["label"], "English");
    assert_eq!(v["tracks"][0]["lang"], "en");
    assert!(v["tracks"][0]["fileName"]
        .as_str()
        .unwrap()
        .ends_with(".vtt"));
    assert_eq!(v["trackDisplayLoc"], "media");
    let xml = String::from_utf8(part_bytes(&patched, "ppt/slides/slide1.xml")).unwrap();
    assert!(xml.contains("p14:trim"));
    assert!(xml.contains("name=\"MarkA\""));
    assert!(xml.contains("tracksInfo"));
    assert!(!xml.contains("<p:timing"));
    let rels = String::from_utf8(part_bytes(&patched, "ppt/slides/_rels/slide1.xml.rels")).unwrap();
    assert!(rels.contains("/track"));

    let cleared = tmp("clr-t").with_extension("pptx");
    apply(
        &patched,
        &cleared,
        json!([{
            "op": "setMedia",
            "id": "slide:1/video:3",
            "st": 0,
            "end": 0,
            "bookmarks": [],
            "tracks": []
        }]),
    )
    .unwrap();
    let after = inspect_media(&cleared);
    let v2 = &after["slides"][0]["videos"][0];
    assert_eq!(v2["st"], 0);
    assert_eq!(v2["end"], 0);
    assert_eq!(v2["bookmarks"], json!([]));
    assert_eq!(v2["tracks"], json!([]));
    let xml2 = String::from_utf8(part_bytes(&cleared, "ppt/slides/slide1.xml")).unwrap();
    assert!(!xml2.contains("<p14:trim"));
    assert!(!xml2.contains("bmkLst"));
    assert!(!xml2.contains("tracksInfo"));

    let err = apply(
        &inserted,
        &tmp("bad-t").with_extension("pptx"),
        json!([{ "op": "setMedia", "id": "slide:1" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    let err = apply(
        &inserted,
        &tmp("empty-t").with_extension("pptx"),
        json!([{ "op": "setMedia", "id": "slide:1/video:3" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    let err = apply(
        &inserted,
        &tmp("dup-t").with_extension("pptx"),
        json!([{
            "op": "setMedia",
            "id": "slide:1/video:3",
            "bookmarks": [
                { "name": "A", "time": 1 },
                { "name": "A", "time": 2 }
            ]
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

#[test]
fn insert_video_clip_fields() {
    let src = create_deck("ins-clip");
    let mp4 = write_mp4("clip-i");
    let vtt = write_vtt("cap-i");
    let inserted = tmp("ins-clip").with_extension("pptx");
    apply(
        &src,
        &inserted,
        json!([{
            "op": "insertVideo",
            "id": "slide:1",
            "file": mp4.to_string_lossy(),
            "st": 100,
            "end": 200,
            "bookmarks": [{ "name": "MarkA", "time": 300 }],
            "tracks": [{ "label": "English", "lang": "en", "file": vtt.to_string_lossy() }]
        }]),
    )
    .unwrap();
    let tree = inspect_media(&inserted);
    let v = &tree["slides"][0]["videos"][0];
    assert_eq!(v["st"], 100);
    assert_eq!(v["end"], 200);
    assert_eq!(v["bookmarks"][0]["name"], "MarkA");
    assert_eq!(v["tracks"][0]["label"], "English");
    assert_eq!(v["trackDisplayLoc"], "media");
    let xml = String::from_utf8(part_bytes(&inserted, "ppt/slides/slide1.xml")).unwrap();
    assert!(xml.contains("p14:trim"));
    assert!(xml.contains("name=\"MarkA\""));
    assert!(xml.contains("tracksInfo"));
    assert!(!xml.contains("<p:timing"));
}

fn inspect_notes_media(path: &Path) -> serde_json::Value {
    let RunOutput::Inspect(tree) = run(&json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": path.to_string_lossy(),
        "scope": { "media": true, "notes": true }
    })
    .to_string())
    .unwrap() else {
        panic!("inspect 应返回树");
    };
    tree
}

#[test]
fn set_picture_geom_on_video_and_audio() {
    let src = create_deck("geom");
    let mp4 = write_mp4("clip-g");
    let wav = write_wav("beep-g");
    let inserted = tmp("ins-g").with_extension("pptx");
    apply(
        &src,
        &inserted,
        json!([
            {
                "op": "insertVideo",
                "id": "slide:1",
                "file": mp4.to_string_lossy()
            },
            {
                "op": "insertAudio",
                "id": "slide:1",
                "file": wav.to_string_lossy()
            }
        ]),
    )
    .unwrap();
    let tree0 = inspect_media(&inserted);
    let vid = tree0["slides"][0]["videos"][0]["id"].as_str().unwrap();
    let aud = tree0["slides"][0]["audios"][0]["id"].as_str().unwrap();
    let old_cx = tree0["slides"][0]["videos"][0]["cx"].clone();
    let old_cy = tree0["slides"][0]["videos"][0]["cy"].clone();

    let sized = tmp("sz-g").with_extension("pptx");
    apply(
        &inserted,
        &sized,
        json!([
            { "op": "setPictureSize", "id": vid, "cx": 1828800, "cy": 914400 },
            { "op": "setPicturePos", "id": vid, "x": 914400, "y": 457200 },
            { "op": "setPicturePos", "id": aud, "x": 1000, "y": 2000 }
        ]),
    )
    .unwrap();
    let tree = inspect_media(&sized);
    let v = &tree["slides"][0]["videos"][0];
    let a = &tree["slides"][0]["audios"][0];
    assert_eq!(v["cx"], 1828800);
    assert_eq!(v["cy"], 914400);
    assert_eq!(v["x"], 914400);
    assert_eq!(v["y"], 457200);
    assert_eq!(a["x"], 1000);
    assert_eq!(a["y"], 2000);
    assert_ne!(old_cx, json!(1828800));
    assert_ne!(old_cy, json!(914400));
    let xml = String::from_utf8(part_bytes(&sized, "ppt/slides/slide1.xml")).unwrap();
    assert!(xml.contains("cx=\"1828800\""));
    assert!(xml.contains("cy=\"914400\""));
    assert!(xml.contains("x=\"914400\""));
    assert!(xml.contains("y=\"457200\""));
    assert!(!xml.contains("<p:timing"));

    let trimmed = tmp("trim-g").with_extension("pptx");
    apply(
        &sized,
        &trimmed,
        json!([{ "op": "setMedia", "id": vid, "st": 100, "end": 200 }]),
    )
    .unwrap();
    let after = inspect_media(&trimmed);
    let v2 = &after["slides"][0]["videos"][0];
    assert_eq!(v2["cx"], 1828800);
    assert_eq!(v2["cy"], 914400);
    assert_eq!(v2["x"], 914400);
    assert_eq!(v2["y"], 457200);
    assert_eq!(v2["st"], 100);
    assert_eq!(v2["end"], 200);

    let notes_src = tmp("notes-g").with_extension("pptx");
    apply(
        &src,
        &notes_src,
        json!([{
            "op": "insertVideo",
            "id": "slide:1/notes",
            "file": mp4.to_string_lossy()
        }]),
    )
    .unwrap();
    let notes_tree0 = inspect_notes_media(&notes_src);
    assert_eq!(notes_tree0["videoCount"], 0);
    let n_id = notes_tree0["slides"][0]["notes"]["videos"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let notes_sz = tmp("notes-sz-g").with_extension("pptx");
    apply(
        &notes_src,
        &notes_sz,
        json!([{ "op": "setPictureSize", "id": n_id, "cx": 1828800, "cy": 914400 }]),
    )
    .unwrap();
    let notes_tree = inspect_notes_media(&notes_sz);
    assert_eq!(notes_tree["videoCount"], 0);
    assert_eq!(notes_tree["slides"][0]["notes"]["videos"][0]["cx"], 1828800);
    assert_eq!(notes_tree["slides"][0]["notes"]["videos"][0]["cy"], 914400);
    let notes_xml =
        String::from_utf8(part_bytes(&notes_sz, "ppt/notesSlides/notesSlide1.xml")).unwrap();
    assert!(notes_xml.contains("cx=\"1828800\""));

    let deleted = tmp("del-g").with_extension("pptx");
    apply(
        &inserted,
        &deleted,
        json!([{ "op": "deleteMedia", "id": vid }]),
    )
    .unwrap();
    let missing = tmp("miss-g").with_extension("pptx");
    let err = apply(
        &deleted,
        &missing,
        json!([{ "op": "setPictureSize", "id": vid, "cx": 1000, "cy": 1000 }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::IdNotFound);
    assert!(!missing.exists());

    let bad_pos = tmp("bad-pos-g").with_extension("pptx");
    let err = apply(
        &inserted,
        &bad_pos,
        json!([{ "op": "setPicturePos", "id": vid }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert!(!bad_pos.exists());

    let bad_media = tmp("bad-media-g").with_extension("pptx");
    let err = apply(
        &src,
        &bad_media,
        json!([{ "op": "setMedia", "id": "slide:1/pic:3", "st": 1 }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    assert!(!bad_media.exists());
}
