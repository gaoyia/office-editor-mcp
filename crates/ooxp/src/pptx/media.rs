//! 幻灯片视频 / 音频：带 `a:videoFile` / `a:audioFile` 的 `p:pic`。不写 timing。

use std::path::Path;

use serde_json::{json, Map, Value};

use crate::error::{Error, ErrorKind, IdReason};
use crate::opc::{resolve_rel_target, ContentTypes, Package, Relationship, Relationships};
use crate::xml::{Document, Element, Node};

use super::picture::{self, PicKey};
use super::request::{DeleteMediaOp, InsertMediaOp, MediaBookmark, MediaTrackWrite, SetMediaOp};
use super::shape::{self, first_alt_child_index};
use super::{ExportedMedia, DML_NS, REL_NS};

pub const REL_MEDIA: &str = "http://schemas.microsoft.com/office/2007/relationships/media";
pub const REL_VIDEO: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/video";
pub const REL_AUDIO: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/audio";
pub const REL_TRACK: &str = "http://schemas.microsoft.com/office/2017/04/relationships/track";
pub const P14_NS: &str = "http://schemas.microsoft.com/office/powerpoint/2010/main";
const P174_NS: &str = "http://schemas.microsoft.com/office/powerpoint/2017/3/main";
const MEDIA_EXT_URI: &str = "{DAA4B4D4-6D71-4841-9C94-3DE7FCFB9230}";
const TRACK_EXT_URI: &str = "{3AFAAA56-56D3-431D-BCD4-E75A35582382}";

const DEFAULT_VIDEO_CX: i64 = 3_048_000;
const DEFAULT_VIDEO_CY: i64 = 2_286_000;
const DEFAULT_AUDIO_CX: i64 = 609_600;
const DEFAULT_AUDIO_CY: i64 = 609_600;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaKind {
    Video,
    Audio,
}

impl MediaKind {
    fn id_seg(self) -> &'static str {
        match self {
            Self::Video => "video",
            Self::Audio => "audio",
        }
    }

    fn default_size(self) -> (i64, i64) {
        match self {
            Self::Video => (DEFAULT_VIDEO_CX, DEFAULT_VIDEO_CY),
            Self::Audio => (DEFAULT_AUDIO_CX, DEFAULT_AUDIO_CY),
        }
    }

    fn iso_rel(self) -> &'static str {
        match self {
            Self::Video => REL_VIDEO,
            Self::Audio => REL_AUDIO,
        }
    }

    fn file_local(self) -> &'static str {
        match self {
            Self::Video => "videoFile",
            Self::Audio => "audioFile",
        }
    }
}

#[derive(Debug, Clone)]
pub struct MediaTarget {
    pub slide: usize,
    pub on_notes: bool,
    pub master: Option<(usize, Option<usize>)>,
    pub kind: MediaKind,
    pub key: PicKey,
}

pub struct MediaInfo {
    pub id: String,
    pub cnv_pr_id: Option<String>,
    pub name: String,
    pub x: Option<i64>,
    pub y: Option<i64>,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
    pub embedded: bool,
    pub file_name: Option<String>,
    pub content_type: Option<String>,
    pub stable: bool,
    pub st: f64,
    pub end: f64,
    pub bookmarks: Vec<(String, f64)>,
    pub tracks: Vec<TrackInfo>,
    pub track_display_loc: Option<String>,
}

pub struct TrackInfo {
    pub id: Option<String>,
    pub label: String,
    pub lang: Option<String>,
    pub file_name: Option<String>,
    pub embedded: bool,
}

pub fn is_media_id(id: &str) -> bool {
    (id.starts_with("slide:") || id.starts_with("master:"))
        && (id.contains("/video:") || id.contains("/audio:"))
}

pub fn parse_media_id(id: &str, json_path: &str) -> Result<MediaTarget, Error> {
    let path = json_path.to_string();
    let (host, after, kind) = if let Ok((host, after)) =
        picture::parse_object_id(id, json_path, "video", IdReason::SlideMediaForm)
    {
        (host, after, MediaKind::Video)
    } else if let Ok((host, after)) =
        picture::parse_object_id(id, json_path, "audio", IdReason::SlideMediaForm)
    {
        (host, after, MediaKind::Audio)
    } else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SlideMediaForm,
            },
            path,
        ));
    };
    let (slide, on_notes, master) = host.parts();
    let key = parse_media_key(after, &path)?;
    Ok(MediaTarget {
        slide,
        on_notes,
        master,
        kind,
        key,
    })
}

fn parse_media_key(after: &str, path: &str) -> Result<PicKey, Error> {
    if after.is_empty() {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SlideMediaForm,
            },
            path.to_string(),
        ));
    }
    if let Some(k) = after.strip_prefix("index:") {
        let idx: usize = k.parse().map_err(|_| {
            Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::PicIndexNotInt,
                },
                path.to_string(),
            )
        })?;
        if idx < 1 {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::PicIndexFromOne,
                },
                path.to_string(),
            ));
        }
        return Ok(PicKey::Index(idx));
    }
    Ok(PicKey::Id(after.to_string()))
}

pub fn parse_insert_slide(id: &str, op_index: usize) -> Result<picture::TreeHost, Error> {
    picture::parse_insert_host(id, op_index, IdReason::InsertMediaNeedsSlide)
}

/// 同时带 `videoFile` 时算视频。
pub(crate) fn media_kind(pic: &Element) -> Option<MediaKind> {
    if pic.local_name() != "pic" {
        return None;
    }
    let nv = nv_pr(pic)?;
    if nv.child_by_local("videoFile").is_some() {
        return Some(MediaKind::Video);
    }
    if nv.child_by_local("audioFile").is_some() || nv.child_by_local("wavAudioFile").is_some() {
        return Some(MediaKind::Audio);
    }
    None
}

fn nv_pr(pic: &Element) -> Option<&Element> {
    pic.child_by_local("nvPicPr")
        .and_then(|nv| nv.child_by_local("nvPr"))
}

pub fn count(root: &Element) -> (usize, usize) {
    let mut videos = 0usize;
    let mut audios = 0usize;
    shape::walk_choice(root, &mut |e| match media_kind(e) {
        Some(MediaKind::Video) => videos += 1,
        Some(MediaKind::Audio) => audios += 1,
        None => {}
    });
    (videos, audios)
}

pub fn collect(
    pkg: &Package,
    part: &str,
    root: &Element,
    host: &str,
) -> Result<(Vec<MediaInfo>, Vec<MediaInfo>), Error> {
    let rels = Relationships::from_package(pkg, part)?;
    let cts = ContentTypes::from_package(pkg)?;
    let mut videos = Vec::new();
    let mut audios = Vec::new();
    let mut v_idx = 0usize;
    let mut a_idx = 0usize;
    shape::walk_choice(root, &mut |e| match media_kind(e) {
        Some(MediaKind::Video) => {
            v_idx += 1;
            videos.push(media_info(
                e,
                host,
                MediaKind::Video,
                v_idx,
                part,
                &rels,
                pkg,
                &cts,
            ));
        }
        Some(MediaKind::Audio) => {
            a_idx += 1;
            audios.push(media_info(
                e,
                host,
                MediaKind::Audio,
                a_idx,
                part,
                &rels,
                pkg,
                &cts,
            ));
        }
        None => {}
    });
    Ok((videos, audios))
}

pub fn list_json(items: &[MediaInfo]) -> Vec<Value> {
    items.iter().map(media_to_json).collect()
}

fn media_to_json(p: &MediaInfo) -> Value {
    let mut m = Map::new();
    m.insert("id".into(), json!(p.id));
    m.insert("cNvPrId".into(), json!(p.cnv_pr_id));
    m.insert("name".into(), json!(p.name));
    m.insert("x".into(), json!(p.x));
    m.insert("y".into(), json!(p.y));
    m.insert("cx".into(), json!(p.cx));
    m.insert("cy".into(), json!(p.cy));
    m.insert("embedded".into(), json!(p.embedded));
    m.insert("fileName".into(), json!(p.file_name));
    m.insert("contentType".into(), json!(p.content_type));
    m.insert("stable".into(), json!(p.stable));
    m.insert("st".into(), json_time(p.st));
    m.insert("end".into(), json_time(p.end));
    let bookmarks: Vec<Value> = p
        .bookmarks
        .iter()
        .map(|(name, time)| {
            let mut b = Map::new();
            b.insert("name".into(), json!(name));
            b.insert("time".into(), json_time(*time));
            Value::Object(b)
        })
        .collect();
    m.insert("bookmarks".into(), Value::Array(bookmarks));
    let tracks: Vec<Value> = p
        .tracks
        .iter()
        .map(|t| {
            let mut o = Map::new();
            o.insert("id".into(), json!(t.id));
            o.insert("label".into(), json!(t.label));
            o.insert("lang".into(), json!(t.lang));
            o.insert("fileName".into(), json!(t.file_name));
            o.insert("embedded".into(), json!(t.embedded));
            Value::Object(o)
        })
        .collect();
    m.insert("tracks".into(), Value::Array(tracks));
    if let Some(loc) = &p.track_display_loc {
        m.insert("trackDisplayLoc".into(), json!(loc));
    }
    Value::Object(m)
}

fn media_info(
    pic: &Element,
    host: &str,
    kind: MediaKind,
    index: usize,
    part: &str,
    rels: &Relationships,
    pkg: &Package,
    cts: &ContentTypes,
) -> MediaInfo {
    let cnv = pic_cnv_pr(pic);
    let cnv_pr_id = cnv
        .and_then(|c| c.attr("id"))
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    let name = cnv.and_then(|c| c.attr("name")).unwrap_or("").to_string();
    let (id, stable) = match &cnv_pr_id {
        Some(n) => (format!("{host}/{}:{n}", kind.id_seg()), true),
        None => (format!("{host}/{}:index:{index}", kind.id_seg()), false),
    };
    let xfrm = pic
        .child_by_local("spPr")
        .and_then(|sp| sp.child_by_local("xfrm"));
    let (x, y) = match xfrm.and_then(|x| x.child_by_local("off")) {
        Some(off) => (parse_i64_attr(off, "x"), parse_i64_attr(off, "y")),
        None => (None, None),
    };
    let (cx, cy) = match xfrm.and_then(|x| x.child_by_local("ext")) {
        Some(ext) => (parse_i64_attr(ext, "cx"), parse_i64_attr(ext, "cy")),
        None => (None, None),
    };
    let (embedded, file_name, content_type) = rel_meta(pic, part, rels, pkg, cts);
    let clip = clip_inspect(pic, part, rels, pkg);
    MediaInfo {
        id,
        cnv_pr_id,
        name,
        x,
        y,
        cx,
        cy,
        embedded,
        file_name,
        content_type,
        stable,
        st: clip.st,
        end: clip.end,
        bookmarks: clip.bookmarks,
        tracks: clip.tracks,
        track_display_loc: clip.track_display_loc,
    }
}

fn rel_meta(
    pic: &Element,
    part: &str,
    rels: &Relationships,
    pkg: &Package,
    cts: &ContentTypes,
) -> (bool, Option<String>, Option<String>) {
    let Some(rid) = media_rel_id(pic) else {
        return (false, None, None);
    };
    let Some(rel) = rels.find_by_id(&rid) else {
        return (false, None, None);
    };
    let file_name = Some(last_segment(&rel.target));
    if rel.is_external() {
        return (false, file_name, None);
    }
    match resolve_rel_target(part, &rel.target) {
        Ok(opc) => {
            let embedded = pkg.contains(&opc);
            let content_type = if embedded {
                cts.content_type_of(&opc).map(str::to_string)
            } else {
                None
            };
            (embedded, file_name, content_type)
        }
        Err(_) => (false, file_name, None),
    }
}

fn last_segment(target: &str) -> String {
    target
        .replace('\\', "/")
        .rsplit('/')
        .next()
        .unwrap_or(target)
        .to_string()
}

fn media_rel_id(pic: &Element) -> Option<String> {
    p14_media_embed(pic).or_else(|| file_rel_id(pic))
}

fn p14_media_embed(pic: &Element) -> Option<String> {
    let nv = nv_pr(pic)?;
    let lst = nv.child_by_local("extLst")?;
    for ext in lst.element_children() {
        if let Some(m) = ext.child_by_local("media") {
            if let Some(id) = m.attr_local("embed").filter(|s| !s.is_empty()) {
                return Some(id.to_string());
            }
        }
    }
    None
}

fn file_rel_id(pic: &Element) -> Option<String> {
    let nv = nv_pr(pic)?;
    let el = nv
        .child_by_local("videoFile")
        .or_else(|| nv.child_by_local("audioFile"))
        .or_else(|| nv.child_by_local("wavAudioFile"))?;
    el.attr_local("embed")
        .or_else(|| el.attr_local("link"))
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn parse_i64_attr(el: &Element, name: &str) -> Option<i64> {
    el.attr(name)?.parse().ok()
}

fn pic_cnv_pr(pic: &Element) -> Option<&Element> {
    pic.child_by_local("nvPicPr")
        .and_then(|nv| nv.child_by_local("cNvPr"))
}

fn ids_match(file: &str, req: &str) -> bool {
    match (file.parse::<u64>(), req.parse::<u64>()) {
        (Ok(a), Ok(b)) => a == b,
        _ => file == req,
    }
}

fn pic_matches(pic: &Element, index: usize, key: &PicKey) -> bool {
    match key {
        PicKey::Index(n) => index == *n,
        PicKey::Id(req) => pic_cnv_pr(pic)
            .and_then(|c| c.attr("id"))
            .is_some_and(|id| ids_match(id, req)),
    }
}

fn find_media<'a>(root: &'a Element, kind: MediaKind, key: &PicKey) -> Option<&'a Element> {
    let mut idx = 0usize;
    let mut found = None;
    shape::walk_choice(root, &mut |e| {
        if found.is_some() || media_kind(e) != Some(kind) {
            return;
        }
        idx += 1;
        if pic_matches(e, idx, key) {
            found = Some(e);
        }
    });
    found
}

fn remove_media(root: &mut Element, kind: MediaKind, key: &PicKey) -> bool {
    let mut idx = 0usize;
    remove_media_in(root, &mut idx, kind, key)
}

fn remove_media_in(el: &mut Element, idx: &mut usize, kind: MediaKind, key: &PicKey) -> bool {
    if el.local_name() == "AlternateContent" {
        if let Some(i) = first_alt_child_index(el) {
            if let Node::Element(child) = &mut el.children[i] {
                return remove_media_in(child, idx, kind, key);
            }
        }
        return false;
    }
    let mut i = 0;
    while i < el.children.len() {
        let is_hit = match &el.children[i] {
            Node::Element(e) if media_kind(e) == Some(kind) => {
                *idx += 1;
                pic_matches(e, *idx, key)
            }
            _ => false,
        };
        if is_hit {
            el.children.remove(i);
            return true;
        }
        if let Node::Element(e) = &mut el.children[i] {
            if media_kind(e).is_none() && remove_media_in(e, idx, kind, key) {
                return true;
            }
        }
        i += 1;
    }
    false
}

fn collect_rids(el: &Element, out: &mut Vec<String>) {
    for (k, v) in &el.attrs {
        let local = k.rsplit(':').next().unwrap_or(k);
        if (local == "embed" || local == "link") && !v.is_empty() && !out.iter().any(|s| s == v) {
            out.push(v.clone());
        }
    }
    for c in el.element_children() {
        collect_rids(c, out);
    }
}

fn rid_still_on_slide(root: &Element, rid: &str) -> bool {
    let mut used = false;
    shape::walk_choice(root, &mut |e| {
        if used {
            return;
        }
        for (k, v) in &e.attrs {
            let local = k.rsplit(':').next().unwrap_or(k);
            if (local == "embed" || local == "link") && v == rid {
                used = true;
                return;
            }
        }
    });
    used
}

pub fn export(bytes: &[u8], id: &str) -> Result<ExportedMedia, Error> {
    let target = parse_media_id(id, "id")?;
    let pkg = Package::open_bytes(bytes.to_vec())?;
    let pres_part = crate::opc::office_document_part(&pkg)?;
    let pres = crate::xml::parse(pkg.get(&pres_part)?)?;
    let slides = shape::list_slides(&pkg, &pres_part, &pres.root)?;
    let host = picture::TreeHost::from_parts(target.slide, target.on_notes, target.master);
    let part = picture::resolve_tree_part(
        &pkg,
        &pres_part,
        &pres.root,
        &slides,
        &host,
        ErrorKind::MediaNotFound { id: id.to_string() },
        "id",
    )?;
    let doc = crate::xml::parse(pkg.get(&part)?)?;
    let pic = find_media(&doc.root, target.kind, &target.key)
        .ok_or_else(|| Error::with_path(ErrorKind::MediaNotFound { id: id.to_string() }, "id"))?;
    let rid = media_rel_id(pic)
        .ok_or_else(|| Error::with_path(ErrorKind::MediaNotFound { id: id.to_string() }, "id"))?;
    let media = crate::opc::resolve_id(&pkg, &part, &rid)?;
    let media_bytes = pkg.get(&media)?.to_vec();
    let cts = ContentTypes::from_package(&pkg)?;
    let content_type = cts.content_type_of(&media).map(str::to_string);
    Ok(ExportedMedia {
        bytes: media_bytes,
        content_type,
    })
}

pub fn insert(
    pkg: &mut Package,
    slide_doc: &mut Document,
    slide_part: &str,
    pres_root: &Element,
    op: &InsertMediaOp,
    file_field: &str,
    kind: MediaKind,
) -> Result<(), Error> {
    let clip = read_clip(&op.file, kind).map_err(|e| e.at(file_field))?;
    let (cx, cy) = match (op.cx, op.cy) {
        (Some(x), Some(y)) => (x, y),
        _ => kind.default_size(),
    };
    let (x, y) = match (op.x, op.y) {
        (Some(px), Some(py)) => (px, py),
        _ => picture::vacant_pos_host(op.on_notes, pres_root, &slide_doc.root, cx, cy),
    };
    picture::ensure_xmlns(&mut slide_doc.root, "r", REL_NS);
    picture::ensure_xmlns(&mut slide_doc.root, "a", DML_NS);
    let cnv = picture::next_cnv_id(&slide_doc.root);
    let clip_part = next_clip_part(pkg, &clip.ext);
    pkg.set(&clip_part, clip.bytes)?;
    let poster_part = picture::next_media_part(pkg, "png");
    pkg.set(&poster_part, crate::ole::PREVIEW_PNG.to_vec())?;
    let mut rels = Relationships::from_package(pkg, slide_part)?;
    let rid_media = rels.next_rid();
    rels.push(Relationship {
        id: rid_media.clone(),
        rel_type: REL_MEDIA.into(),
        target: picture::rel_target_from(slide_part, &clip_part),
        target_mode: None,
        extra: Vec::new(),
    });
    let rid_iso = rels.next_rid();
    rels.push(Relationship {
        id: rid_iso.clone(),
        rel_type: kind.iso_rel().into(),
        target: picture::rel_target_from(slide_part, &clip_part),
        target_mode: None,
        extra: Vec::new(),
    });
    let rid_img = rels.next_rid();
    rels.push(Relationship {
        id: rid_img.clone(),
        rel_type: picture::REL_IMAGE.into(),
        target: picture::rel_target_from(slide_part, &poster_part),
        target_mode: None,
        extra: Vec::new(),
    });
    let mut cts = ContentTypes::from_package(pkg)?;
    cts.ensure_default(&clip.ext, &clip.content_type);
    cts.ensure_default("png", "image/png");
    let track_rids = add_tracks(
        pkg,
        slide_part,
        &mut rels,
        &mut cts,
        op.tracks.as_deref().unwrap_or(&[]),
        "tracks",
    )?;
    rels.write_to(pkg, slide_part)?;
    cts.write_to(pkg)?;
    let name = Path::new(&op.file)
        .file_name()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| match kind {
            MediaKind::Video => format!("视频 {cnv}"),
            MediaKind::Audio => format!("音频 {cnv}"),
        });
    let pic = build_media_pic(
        cnv,
        &name,
        kind,
        &rid_iso,
        &rid_media,
        &rid_img,
        x,
        y,
        cx,
        cy,
        op.st.unwrap_or(0.0),
        op.end.unwrap_or(0.0),
        op.bookmarks.as_deref().unwrap_or(&[]),
        &track_rids,
        op.track_display_loc.as_deref().unwrap_or("media"),
    );
    picture::insert_into_sp_tree(&mut slide_doc.root, pic)?;
    Ok(())
}

pub fn delete(
    pkg: &mut Package,
    slide_doc: &mut Document,
    slide_part: &str,
    op: &DeleteMediaOp,
    op_index: usize,
) -> Result<(), Error> {
    let path = format!("ops[{op_index}].id");
    let pic = find_media(&slide_doc.root, op.target.kind, &op.target.key).ok_or_else(|| {
        Error::with_path(ErrorKind::MediaNotFound { id: op.id.clone() }, path.clone())
    })?;
    let spid = pic_cnv_pr(pic)
        .and_then(|c| c.attr("id"))
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    let mut rids = Vec::new();
    collect_rids(pic, &mut rids);
    let mut old_parts = Vec::new();
    for rid in &rids {
        if let Ok(p) = crate::opc::resolve_id(pkg, slide_part, rid) {
            if !old_parts.iter().any(|x| x == &p) {
                old_parts.push(p);
            }
        }
    }
    if !remove_media(&mut slide_doc.root, op.target.kind, &op.target.key) {
        return Err(Error::with_path(
            ErrorKind::MediaNotFound { id: op.id.clone() },
            path,
        ));
    }
    if let Some(spid) = spid {
        shape::strip_timing_for_spid(&mut slide_doc.root, &spid);
    }
    let mut rels = Relationships::from_package(pkg, slide_part)?;
    let mut dirty = false;
    for rid in &rids {
        if !rid_still_on_slide(&slide_doc.root, rid) {
            rels.remove_by_id(rid);
            dirty = true;
        }
    }
    if dirty {
        rels.write_to(pkg, slide_part)?;
    }
    for part in old_parts {
        if !picture::media_still_used(pkg, &part)? {
            pkg.remove(&part);
        }
    }
    Ok(())
}

struct ClipFile {
    bytes: Vec<u8>,
    ext: String,
    content_type: String,
}

fn read_clip(path: &str, kind: MediaKind) -> Result<ClipFile, Error> {
    let bytes = crate::host_files::read_user_file(path)?;
    match kind {
        MediaKind::Video => {
            if is_mp4(&bytes) {
                Ok(ClipFile {
                    bytes,
                    ext: "mp4".into(),
                    content_type: "video/mp4".into(),
                })
            } else {
                Err(Error::new(ErrorKind::InvalidMediaFile))
            }
        }
        MediaKind::Audio => {
            if is_wav(&bytes) {
                Ok(ClipFile {
                    bytes,
                    ext: "wav".into(),
                    content_type: "audio/x-wav".into(),
                })
            } else {
                Err(Error::new(ErrorKind::InvalidMediaFile))
            }
        }
    }
}

fn is_mp4(bytes: &[u8]) -> bool {
    bytes.len() >= 8 && &bytes[4..8] == b"ftyp"
}

fn is_wav(bytes: &[u8]) -> bool {
    bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WAVE"
}

fn next_clip_part(pkg: &Package, ext: &str) -> String {
    let mut n = 1u32;
    loop {
        let name = format!("/ppt/media/media{n}.{ext}");
        if !pkg.contains(&name) {
            return name;
        }
        n += 1;
    }
}

fn build_media_pic(
    cnv: u32,
    name: &str,
    kind: MediaKind,
    rid_iso: &str,
    rid_media: &str,
    rid_img: &str,
    x: i64,
    y: i64,
    cx: i64,
    cy: i64,
    st: f64,
    end: f64,
    bookmarks: &[MediaBookmark],
    tracks: &[(MediaTrackWrite, String)],
    display_loc: &str,
) -> Element {
    let mut pic = Element::new("p:pic");
    let mut nv = Element::new("p:nvPicPr");
    let mut cnv_pr = Element::new("p:cNvPr")
        .with_attr("id", cnv.to_string())
        .with_attr("name", name);
    cnv_pr.push_element(
        Element::new("a:hlinkClick")
            .with_attr("r:id", "")
            .with_attr("action", "ppaction://media"),
    );
    nv.push_element(cnv_pr);
    let mut cnv_pic = Element::new("p:cNvPicPr");
    cnv_pic.push_element(Element::new("a:picLocks").with_attr("noChangeAspect", "1"));
    nv.push_element(cnv_pic);
    let mut nv_pr = Element::new("p:nvPr");
    nv_pr.push_element(
        Element::new(format!("a:{}", kind.file_local())).with_attr("r:link", rid_iso),
    );
    let mut ext_lst = Element::new("p:extLst");
    let mut ext = Element::new("p:ext").with_attr("uri", MEDIA_EXT_URI);
    let mut media_el = Element::new("p14:media")
        .with_attr("xmlns:p14", P14_NS)
        .with_attr("r:embed", rid_media);
    fill_p14_media(
        &mut media_el,
        st,
        end,
        None,
        bookmarks,
        Some(tracks),
        display_loc,
    );
    ext.push_element(media_el);
    ext_lst.push_element(ext);
    nv_pr.push_element(ext_lst);
    nv.push_element(nv_pr);
    pic.push_element(nv);

    let mut fill = Element::new("p:blipFill");
    fill.push_element(Element::new("a:blip").with_attr("r:embed", rid_img));
    let mut stretch = Element::new("a:stretch");
    stretch.push_element(Element::new("a:fillRect"));
    fill.push_element(stretch);
    pic.push_element(fill);

    let mut sp_pr = Element::new("p:spPr");
    let mut xfrm = Element::new("a:xfrm");
    xfrm.push_element(
        Element::new("a:off")
            .with_attr("x", x.to_string())
            .with_attr("y", y.to_string()),
    );
    xfrm.push_element(
        Element::new("a:ext")
            .with_attr("cx", cx.to_string())
            .with_attr("cy", cy.to_string()),
    );
    sp_pr.push_element(xfrm);
    let mut geom = Element::new("a:prstGeom").with_attr("prst", "rect");
    geom.push_element(Element::new("a:avLst"));
    sp_pr.push_element(geom);
    pic.push_element(sp_pr);
    pic
}

fn fill_p14_media(
    media_el: &mut Element,
    st: f64,
    end: f64,
    fade: Option<Element>,
    bookmarks: &[MediaBookmark],
    tracks: Option<&[(MediaTrackWrite, String)]>,
    display_loc: &str,
) {
    let mut other_exts = Vec::new();
    let mut kept_track = None;
    if let Some(lst) = media_el.child_by_local("extLst") {
        for ext in lst.element_children() {
            if ext.attr("uri") == Some(TRACK_EXT_URI) {
                if tracks.is_none() {
                    kept_track = Some(ext.clone());
                }
            } else {
                other_exts.push(ext.clone());
            }
        }
    }
    media_el.children.clear();
    if st != 0.0 || end != 0.0 {
        media_el.push_element(
            Element::new("p14:trim")
                .with_attr("st", format_time(st))
                .with_attr("end", format_time(end)),
        );
    }
    if let Some(fade) = fade {
        media_el.push_element(fade);
    }
    if !bookmarks.is_empty() {
        let mut lst = Element::new("p14:bmkLst");
        for b in bookmarks {
            lst.push_element(
                Element::new("p14:bmk")
                    .with_attr("name", &b.name)
                    .with_attr("time", format_time(b.time)),
            );
        }
        media_el.push_element(lst);
    }
    let track_ext = match tracks {
        Some(ts) if !ts.is_empty() => Some(build_track_ext(ts, display_loc)),
        Some(_) => None,
        None => kept_track,
    };
    if track_ext.is_some() || !other_exts.is_empty() {
        let mut lst = Element::new("p14:extLst");
        for e in other_exts {
            lst.push_element(e);
        }
        if let Some(t) = track_ext {
            lst.push_element(t);
        }
        media_el.push_element(lst);
    }
}

fn build_track_ext(tracks: &[(MediaTrackWrite, String)], display_loc: &str) -> Element {
    let mut ext = Element::new("p14:ext").with_attr("uri", TRACK_EXT_URI);
    let mut info = Element::new("p174:tracksInfo")
        .with_attr("xmlns:p174", P174_NS)
        .with_attr("displayLoc", display_loc);
    let mut lst = Element::new("p174:trackLst");
    for (t, rid) in tracks {
        let id = t.id.clone().unwrap_or_else(super::section::new_guid);
        let mut tr = Element::new("p174:track")
            .with_attr("id", id)
            .with_attr("label", &t.label)
            .with_attr("r:embed", rid);
        if let Some(lang) = &t.lang {
            tr.set_attr("lang", lang);
        }
        lst.push_element(tr);
    }
    info.push_element(lst);
    ext.push_element(info);
    ext
}

pub fn is_track_guid(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() != 38 || b[0] != b'{' || b[37] != b'}' {
        return false;
    }
    if b[9] != b'-' || b[14] != b'-' || b[19] != b'-' || b[24] != b'-' {
        return false;
    }
    b[1..9]
        .iter()
        .chain(&b[10..14])
        .chain(&b[15..19])
        .chain(&b[20..24])
        .chain(&b[25..37])
        .all(|c| c.is_ascii_hexdigit())
}

fn json_time(v: f64) -> Value {
    if v.fract() == 0.0 && v.abs() <= i64::MAX as f64 {
        json!(v as i64)
    } else {
        json!(v)
    }
}

fn format_time(v: f64) -> String {
    if v.fract() == 0.0 {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}

fn parse_time_xml(s: &str) -> f64 {
    s.parse().unwrap_or(0.0)
}

struct ClipInspect {
    st: f64,
    end: f64,
    bookmarks: Vec<(String, f64)>,
    tracks: Vec<TrackInfo>,
    track_display_loc: Option<String>,
}

fn clip_inspect(pic: &Element, part: &str, rels: &Relationships, pkg: &Package) -> ClipInspect {
    let Some(media_el) = p14_media(pic) else {
        return ClipInspect {
            st: 0.0,
            end: 0.0,
            bookmarks: Vec::new(),
            tracks: Vec::new(),
            track_display_loc: None,
        };
    };
    let (st, end) = match media_el.child_by_local("trim") {
        Some(t) => (
            t.attr("st").map(parse_time_xml).unwrap_or(0.0),
            t.attr("end").map(parse_time_xml).unwrap_or(0.0),
        ),
        None => (0.0, 0.0),
    };
    let mut bookmarks = Vec::new();
    if let Some(lst) = media_el.child_by_local("bmkLst") {
        for b in lst.children_by_local("bmk") {
            bookmarks.push((
                b.attr("name").unwrap_or("").to_string(),
                b.attr("time").map(parse_time_xml).unwrap_or(0.0),
            ));
        }
    }
    let (tracks, loc) = read_tracks(media_el, part, rels, pkg);
    ClipInspect {
        st,
        end,
        bookmarks,
        tracks,
        track_display_loc: loc,
    }
}

fn p14_media(pic: &Element) -> Option<&Element> {
    let nv = nv_pr(pic)?;
    let lst = nv.child_by_local("extLst")?;
    for ext in lst.element_children() {
        if let Some(m) = ext.child_by_local("media") {
            return Some(m);
        }
    }
    None
}

fn p14_media_mut(pic: &mut Element) -> Option<&mut Element> {
    let nv = nv_pr_mut(pic)?;
    let lst = nv.child_by_local_mut("extLst")?;
    for ext in lst.element_children_mut() {
        if ext.child_by_local("media").is_some() {
            return ext.child_by_local_mut("media");
        }
    }
    None
}

fn nv_pr_mut(pic: &mut Element) -> Option<&mut Element> {
    pic.child_by_local_mut("nvPicPr")
        .and_then(|nv| nv.child_by_local_mut("nvPr"))
}

fn read_tracks(
    media_el: &Element,
    part: &str,
    rels: &Relationships,
    pkg: &Package,
) -> (Vec<TrackInfo>, Option<String>) {
    let Some(lst) = media_el.child_by_local("extLst") else {
        return (Vec::new(), None);
    };
    for ext in lst.element_children() {
        if ext.attr("uri") != Some(TRACK_EXT_URI) {
            continue;
        }
        let Some(info) = ext.child_by_local("tracksInfo") else {
            continue;
        };
        let loc = info
            .attr("displayLoc")
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| "media".into());
        let mut tracks = Vec::new();
        if let Some(tl) = info.child_by_local("trackLst") {
            for tr in tl.children_by_local("track") {
                let rid = tr
                    .attr_local("link")
                    .or_else(|| tr.attr_local("embed"))
                    .unwrap_or("");
                let (embedded, file_name) = track_rel_meta(rid, part, rels, pkg);
                tracks.push(TrackInfo {
                    id: tr.attr("id").filter(|s| !s.is_empty()).map(str::to_string),
                    label: tr.attr("label").unwrap_or("").to_string(),
                    lang: tr
                        .attr("lang")
                        .filter(|s| !s.is_empty())
                        .map(str::to_string),
                    file_name,
                    embedded,
                });
            }
        }
        if tracks.is_empty() {
            return (Vec::new(), None);
        }
        return (tracks, Some(loc));
    }
    (Vec::new(), None)
}

fn track_rel_meta(
    rid: &str,
    part: &str,
    rels: &Relationships,
    pkg: &Package,
) -> (bool, Option<String>) {
    if rid.is_empty() {
        return (false, None);
    }
    let Some(rel) = rels.find_by_id(rid) else {
        return (false, None);
    };
    let file_name = Some(last_segment(&rel.target));
    if rel.is_external() {
        return (false, file_name);
    }
    match resolve_rel_target(part, &rel.target) {
        Ok(opc) => (pkg.contains(&opc), file_name),
        Err(_) => (false, file_name),
    }
}

fn add_tracks(
    pkg: &mut Package,
    slide_part: &str,
    rels: &mut Relationships,
    cts: &mut ContentTypes,
    tracks: &[MediaTrackWrite],
    field: &str,
) -> Result<Vec<(MediaTrackWrite, String)>, Error> {
    let mut out = Vec::new();
    for (j, t) in tracks.iter().enumerate() {
        let bytes = read_vtt(&t.file).map_err(|e| e.at(format!("{field}[{j}].file")))?;
        let part = next_clip_part(pkg, "vtt");
        pkg.set(&part, bytes)?;
        cts.ensure_default("vtt", "text/vtt");
        let rid = rels.next_rid();
        rels.push(Relationship {
            id: rid.clone(),
            rel_type: REL_TRACK.into(),
            target: picture::rel_target_from(slide_part, &part),
            target_mode: None,
            extra: Vec::new(),
        });
        out.push((t.clone(), rid));
    }
    Ok(out)
}

fn read_vtt(path: &str) -> Result<Vec<u8>, Error> {
    let bytes = crate::host_files::read_user_file(path)?;
    if is_vtt(&bytes) {
        Ok(bytes)
    } else {
        Err(Error::new(ErrorKind::InvalidMediaFile))
    }
}

fn is_vtt(bytes: &[u8]) -> bool {
    let s = if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        &bytes[3..]
    } else {
        bytes
    };
    s.starts_with(b"WEBVTT")
}

pub fn set(
    pkg: &mut Package,
    slide_doc: &mut Document,
    slide_part: &str,
    op: &SetMediaOp,
    op_index: usize,
) -> Result<(), Error> {
    let path = format!("ops[{op_index}].id");
    let mut rels = Relationships::from_package(pkg, slide_part)?;
    let mut cts = ContentTypes::from_package(pkg)?;
    let old_track_rids;
    let replaced_tracks;
    {
        let pic = find_media_mut(&mut slide_doc.root, op.target.kind, &op.target.key).ok_or_else(
            || Error::with_path(ErrorKind::MediaNotFound { id: op.id.clone() }, path.clone()),
        )?;
        ensure_p14_media(slide_part, &mut rels, pic)?;
        let media_el = p14_media_mut(pic).ok_or_else(|| {
            Error::with_path(ErrorKind::MediaNotFound { id: op.id.clone() }, path)
        })?;
        let fade = media_el.child_by_local("fade").cloned();
        let (cur_st, end_cur) = match media_el.child_by_local("trim") {
            Some(t) => (
                t.attr("st").map(parse_time_xml).unwrap_or(0.0),
                t.attr("end").map(parse_time_xml).unwrap_or(0.0),
            ),
            None => (0.0, 0.0),
        };
        let st = op.st.unwrap_or(cur_st);
        let end = op.end.unwrap_or(end_cur);
        let bookmarks = match &op.bookmarks {
            Some(b) => b.clone(),
            None => read_bookmarks_owned(media_el),
        };
        let cur_loc = read_tracks(media_el, slide_part, &rels, pkg)
            .1
            .unwrap_or_else(|| "media".into());
        old_track_rids = collect_track_rids(media_el);
        let display_loc = op.track_display_loc.as_deref().unwrap_or(cur_loc.as_str());
        let new_tracks = if let Some(ts) = &op.tracks {
            Some(add_tracks(
                pkg,
                slide_part,
                &mut rels,
                &mut cts,
                ts,
                &format!("ops[{op_index}].tracks"),
            )?)
        } else {
            None
        };
        replaced_tracks = new_tracks.is_some();
        fill_p14_media(
            media_el,
            st,
            end,
            fade,
            &bookmarks,
            new_tracks.as_deref(),
            display_loc,
        );
    }
    if replaced_tracks {
        for rid in old_track_rids {
            if !rid_still_on_slide(&slide_doc.root, &rid) {
                if let Ok(part) = crate::opc::resolve_id(pkg, slide_part, &rid) {
                    rels.remove_by_id(&rid);
                    if !picture::media_still_used(pkg, &part)? {
                        pkg.remove(&part);
                    }
                } else {
                    rels.remove_by_id(&rid);
                }
            }
        }
    }
    rels.write_to(pkg, slide_part)?;
    cts.write_to(pkg)?;
    Ok(())
}

fn read_bookmarks_owned(media_el: &Element) -> Vec<MediaBookmark> {
    let mut out = Vec::new();
    if let Some(lst) = media_el.child_by_local("bmkLst") {
        for b in lst.children_by_local("bmk") {
            out.push(MediaBookmark {
                name: b.attr("name").unwrap_or("").to_string(),
                time: b.attr("time").map(parse_time_xml).unwrap_or(0.0),
            });
        }
    }
    out
}

fn collect_track_rids(media_el: &Element) -> Vec<String> {
    let mut out = Vec::new();
    let Some(lst) = media_el.child_by_local("extLst") else {
        return out;
    };
    for ext in lst.element_children() {
        if ext.attr("uri") != Some(TRACK_EXT_URI) {
            continue;
        }
        let Some(info) = ext.child_by_local("tracksInfo") else {
            continue;
        };
        if let Some(tl) = info.child_by_local("trackLst") {
            for tr in tl.children_by_local("track") {
                if let Some(id) = tr
                    .attr_local("embed")
                    .or_else(|| tr.attr_local("link"))
                    .filter(|s| !s.is_empty())
                {
                    out.push(id.to_string());
                }
            }
        }
    }
    out
}

pub(crate) fn find_media_mut<'a>(
    root: &'a mut Element,
    kind: MediaKind,
    key: &PicKey,
) -> Option<&'a mut Element> {
    let mut idx = 0usize;
    let mut path = Vec::new();
    if !find_media_path(root, &mut idx, kind, key, &mut path) {
        return None;
    }
    let mut cur = root;
    for i in path {
        match &mut cur.children[i] {
            Node::Element(e) => cur = e,
            _ => return None,
        }
    }
    Some(cur)
}

fn find_media_path(
    el: &Element,
    idx: &mut usize,
    kind: MediaKind,
    key: &PicKey,
    path: &mut Vec<usize>,
) -> bool {
    fn path_node<'b>(root: &'b Element, rel: &[usize]) -> Option<&'b Element> {
        let mut cur = root;
        for &i in rel {
            cur = match cur.children.get(i)? {
                Node::Element(child) => child,
                Node::Text(_) | Node::Raw(_) => return None,
            };
        }
        Some(cur)
    }
    struct Frame {
        rel: Vec<usize>,
        next: usize,
    }
    let mut stack = vec![Frame {
        rel: Vec::new(),
        next: 0,
    }];
    while !stack.is_empty() {
        let rel = stack.last().unwrap().rel.clone();
        let next = stack.last().unwrap().next;
        let Some(node) = path_node(el, &rel) else {
            stack.pop();
            continue;
        };
        if next == 0 && node.local_name() == "AlternateContent" {
            let child = first_alt_child_index(node)
                .filter(|i| matches!(&node.children[*i], Node::Element(_)));
            stack.pop();
            if let Some(i) = child {
                let mut rel = rel;
                rel.push(i);
                stack.push(Frame { rel, next: 0 });
            }
            continue;
        }
        if next >= node.children.len() {
            stack.pop();
            continue;
        }
        let step = match &node.children[next] {
            Node::Element(child) => {
                if media_kind(child) == Some(kind) {
                    let hit = pic_matches(child, *idx + 1, key);
                    Some((true, hit))
                } else if media_kind(child).is_none() {
                    Some((false, false))
                } else {
                    None
                }
            }
            Node::Text(_) | Node::Raw(_) => None,
        };
        stack.last_mut().unwrap().next += 1;
        match step {
            Some((true, hit)) => {
                *idx += 1;
                if hit {
                    path.extend_from_slice(&rel);
                    path.push(next);
                    return true;
                }
            }
            Some((false, _)) => {
                let mut child = rel;
                child.push(next);
                stack.push(Frame {
                    rel: child,
                    next: 0,
                });
            }
            None => {}
        }
    }
    false
}

fn ensure_p14_media(
    _slide_part: &str,
    rels: &mut Relationships,
    pic: &mut Element,
) -> Result<(), Error> {
    if p14_media(pic).is_some() {
        return Ok(());
    }
    let embed_rid = if let Some(rid) = p14_media_embed(pic) {
        rid
    } else if let Some(rid) = existing_media_rel(pic, rels) {
        rid
    } else if let Some(file_rid) = file_rel_id(pic) {
        let rel = rels.find_by_id(&file_rid).ok_or_else(|| {
            Error::with_path(ErrorKind::MediaNotFound { id: String::new() }, "id")
        })?;
        let target = rel.target.clone();
        let rid = rels.next_rid();
        rels.push(Relationship {
            id: rid.clone(),
            rel_type: REL_MEDIA.into(),
            target,
            target_mode: None,
            extra: Vec::new(),
        });
        rid
    } else {
        return Err(Error::with_path(
            ErrorKind::MediaNotFound { id: String::new() },
            "id",
        ));
    };
    let nv = nv_pr_mut(pic)
        .ok_or_else(|| Error::with_path(ErrorKind::MediaNotFound { id: String::new() }, "id"))?;
    let lst = if let Some(existing) = nv.child_by_local_mut("extLst") {
        existing
    } else {
        nv.push_element(Element::new("p:extLst"));
        nv.child_by_local_mut("extLst").unwrap()
    };
    let mut ext = Element::new("p:ext").with_attr("uri", MEDIA_EXT_URI);
    ext.push_element(
        Element::new("p14:media")
            .with_attr("xmlns:p14", P14_NS)
            .with_attr("r:embed", embed_rid),
    );
    lst.push_element(ext);
    Ok(())
}

fn existing_media_rel(pic: &Element, rels: &Relationships) -> Option<String> {
    let mut rids = Vec::new();
    collect_rids(pic, &mut rids);
    for rid in rids {
        if let Some(rel) = rels.find_by_id(&rid) {
            if rel.rel_type == REL_MEDIA {
                return Some(rid);
            }
        }
    }
    None
}
