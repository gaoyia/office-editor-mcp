//! 幻灯片现代批注（MS-PPTX 2018/8）。不读 ECMA §19.4 旧式 commentN.xml。

use serde_json::{json, Map, Value};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::error::{Error, ErrorKind, IdReason};
use crate::opc::{self, ContentTypes, Package, Relationship, Relationships};
use crate::xml::{self, Document, Element, Node};

use super::picture;
use super::request::{
    DeleteCommentOp, DeletePictureOp, InsertCommentOp, InsertPictureOp, ReplacePictureOp,
    ReplyCommentOp, SetCommentDoneOp, SetCommentMetaOp, SetCommentTextOp,
};
use super::shape::{self, SlideMeta};
use super::{ExportedMedia, DML_NS, REL_NS};

pub const REL_MODERN_COMMENTS: &str =
    "http://schemas.microsoft.com/office/2018/10/relationships/comments";
pub const REL_AUTHORS: &str = "http://schemas.microsoft.com/office/2018/10/relationships/authors";
pub const CT_MODERN_COMMENTS: &str = "application/vnd.ms-powerpoint.comments+xml";
pub const CT_AUTHORS: &str = "application/vnd.ms-powerpoint.authors+xml";
pub const P188_NS: &str = "http://schemas.microsoft.com/office/powerpoint/2018/8/main";
pub const PC_NS: &str = "http://schemas.microsoft.com/office/powerpoint/2013/main/command";
const COMMENT_EXT_URI: &str = "{6950BFC3-D8DA-4A85-94F7-54DA5524770B}";
const AUTHORS_PART: &str = "/ppt/authors.xml";

static GUID_SEQ: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone)]
pub enum CommentKey {
    Top(String),
    Reply { parent: String, reply: String },
}

#[derive(Debug, Clone)]
pub struct CommentTarget {
    pub slide: usize,
    pub key: CommentKey,
}

#[derive(Clone)]
struct Author {
    id: String,
    name: String,
    initials: String,
}

pub fn parse_comment_id(id: &str, json_path: &str) -> Result<CommentTarget, Error> {
    let path = json_path.to_string();
    let rest = id.strip_prefix("slide:").ok_or_else(|| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::PptxCommentForm,
            },
            path.clone(),
        )
    })?;
    let (n, after) = rest.split_once("/comment:").ok_or_else(|| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::PptxCommentForm,
            },
            path.clone(),
        )
    })?;
    let slide: usize = n.parse().map_err(|_| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SlideIndexNotInt,
            },
            path.clone(),
        )
    })?;
    if slide < 1 {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SlideIndexFromOne,
            },
            path,
        ));
    }
    if after.is_empty() {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::PptxCommentForm,
            },
            path,
        ));
    }
    if let Some((parent, reply)) = after.split_once("/reply:") {
        if !looks_like_guid(parent) || !looks_like_guid(reply) {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::PptxCommentForm,
                },
                path,
            ));
        }
        return Ok(CommentTarget {
            slide,
            key: CommentKey::Reply {
                parent: parent.to_string(),
                reply: reply.to_string(),
            },
        });
    }
    if after.contains('/') || !looks_like_guid(after) {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::PptxCommentForm,
            },
            path,
        ));
    }
    Ok(CommentTarget {
        slide,
        key: CommentKey::Top(after.to_string()),
    })
}

pub fn parse_insert_slide(id: &str, op_index: usize) -> Result<usize, Error> {
    let (slide, on_notes) =
        picture::parse_insert_slide_or_notes(id, op_index, IdReason::InsertCommentNeedsSlide)?;
    if on_notes {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::InsertCommentNeedsSlide,
            },
            format!("ops[{op_index}].id"),
        ));
    }
    Ok(slide)
}

fn looks_like_guid(s: &str) -> bool {
    let inner = s
        .strip_prefix('{')
        .and_then(|t| t.strip_suffix('}'))
        .unwrap_or("");
    inner.len() == 36 && inner.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
}

pub fn count_on_slide(pkg: &Package, slide_part: &str) -> Result<usize, Error> {
    let Some(part) = comments_part(pkg, slide_part)? else {
        return Ok(0);
    };
    let doc = xml::parse(pkg.get(&part)?)?;
    Ok(cm_elements(&doc.root).count())
}

pub fn collect_on_slide(
    pkg: &Package,
    slide_part: &str,
    slide: usize,
    want_pictures: bool,
) -> Result<Vec<Value>, Error> {
    let Some(part) = comments_part(pkg, slide_part)? else {
        return Ok(Vec::new());
    };
    let doc = xml::parse(pkg.get(&part)?)?;
    let rels = Relationships::from_package(pkg, &part)?;
    let authors = load_authors(pkg)?;
    let mut out = Vec::new();
    for cm in cm_elements(&doc.root) {
        if let Some(v) = comment_json(cm, slide, &authors, want_pictures, Some(&rels)) {
            out.push(v);
        }
    }
    Ok(out)
}

fn load_authors(pkg: &Package) -> Result<Vec<Author>, Error> {
    let pres_part = opc::office_document_part(pkg)?;
    let rels = Relationships::from_package(pkg, &pres_part)?;
    let Some(rel) = rels.rels.iter().find(|r| r.rel_type == REL_AUTHORS) else {
        return Ok(Vec::new());
    };
    let part = opc::resolve_id(pkg, &pres_part, &rel.id)?;
    let doc = xml::parse(pkg.get(&part)?)?;
    Ok(author_elements(&doc.root)
        .map(|e| Author {
            id: e.attr("id").unwrap_or("").to_string(),
            name: e.attr("name").unwrap_or("").to_string(),
            initials: e.attr("initials").unwrap_or("").to_string(),
        })
        .collect())
}

pub(crate) fn comments_part(pkg: &Package, slide_part: &str) -> Result<Option<String>, Error> {
    let rels = Relationships::from_package(pkg, slide_part)?;
    let Some(rel) = rels.rels.iter().find(|r| r.rel_type == REL_MODERN_COMMENTS) else {
        return Ok(None);
    };
    Ok(Some(opc::resolve_id(pkg, slide_part, &rel.id)?))
}

fn cm_elements(root: &Element) -> impl Iterator<Item = &Element> {
    root.element_children().filter(|e| e.local_name() == "cm")
}

fn author_elements(root: &Element) -> impl Iterator<Item = &Element> {
    root.element_children()
        .filter(|e| e.local_name() == "author")
}

fn comment_json(
    cm: &Element,
    slide: usize,
    authors: &[Author],
    want_pictures: bool,
    rels: Option<&Relationships>,
) -> Option<Value> {
    let guid = cm.attr("id")?.to_string();
    let host = format!("slide:{slide}/comment:{guid}");
    let author_id = cm.attr("authorId").unwrap_or("");
    let (name, initials) = author_by_id(authors, author_id);
    let (x, y) = pos_xy(cm);
    let text = cm
        .child_by_local("txBody")
        .map(shape::txbody_visible_text)
        .unwrap_or_default();
    let pics = comment_pictures(&host, cm, true, rels);
    let mut replies = Vec::new();
    if let Some(lst) = cm.child_by_local("replyLst") {
        for reply in lst.element_children().filter(|e| e.local_name() == "reply") {
            if let Some(rg) = reply.attr("id") {
                let ra = reply.attr("authorId").unwrap_or("");
                let (rn, ri) = author_by_id(authors, ra);
                let rt = reply
                    .child_by_local("txBody")
                    .map(shape::txbody_visible_text)
                    .unwrap_or_default();
                let reply_id = format!("{host}/reply:{rg}");
                let reply_pics = comment_pictures(&reply_id, reply, false, rels);
                let mut robj = Map::new();
                robj.insert("id".into(), json!(reply_id));
                robj.insert("commentId".into(), json!(rg));
                robj.insert("author".into(), json!(rn));
                robj.insert("initials".into(), json!(ri));
                robj.insert("date".into(), json!(reply.attr("created")));
                robj.insert("done".into(), json!(status_done(reply)));
                robj.insert("kind".into(), json!(comment_kind(reply)));
                robj.insert("pictureCount".into(), json!(reply_pics.len()));
                if want_pictures {
                    robj.insert("pictures".into(), Value::Array(reply_pics));
                }
                robj.insert("text".into(), json!(rt));
                replies.push(Value::Object(robj));
            }
        }
    }
    let mut obj = Map::new();
    obj.insert("id".into(), json!(host));
    obj.insert("commentId".into(), json!(guid));
    obj.insert("author".into(), json!(name));
    obj.insert("initials".into(), json!(initials));
    obj.insert("date".into(), json!(cm.attr("created")));
    obj.insert("done".into(), json!(status_done(cm)));
    obj.insert("kind".into(), json!(comment_kind(cm)));
    obj.insert("x".into(), json!(x));
    obj.insert("y".into(), json!(y));
    obj.insert("pictureCount".into(), json!(pics.len()));
    if want_pictures {
        obj.insert("pictures".into(), Value::Array(pics));
    }
    obj.insert("text".into(), json!(text));
    obj.insert("replies".into(), Value::Array(replies));
    Some(Value::Object(obj))
}

fn comment_pictures(
    host: &str,
    el: &Element,
    skip_reply_lst: bool,
    rels: Option<&Relationships>,
) -> Vec<Value> {
    collect_counted_blips(el, skip_reply_lst)
        .into_iter()
        .enumerate()
        .map(|(i, blip)| {
            let mut m = Map::new();
            m.insert("id".into(), json!(format!("{host}/pic:{}", i + 1)));
            let rid = blip.attr_local("embed").filter(|s| !s.is_empty());
            if let Some(ct) = crate::image::inspect_content_type(
                crate::svg::content_type(blip).is_some(),
                rid,
                rels,
            ) {
                m.insert("contentType".into(), json!(ct));
            }
            Value::Object(m)
        })
        .collect()
}

fn collect_counted_blips(el: &Element, skip_reply_lst: bool) -> Vec<&Element> {
    let mut out = Vec::new();
    walk_counted_blips(el, skip_reply_lst, &mut out);
    out
}

fn walk_counted_blips<'a>(root: &'a Element, skip_reply_lst: bool, out: &mut Vec<&'a Element>) {
    let mut stack = vec![root];
    while let Some(el) = stack.pop() {
        if skip_reply_lst && el.local_name() == "replyLst" {
            continue;
        }
        if is_counted_blip(el) {
            out.push(el);
            continue;
        }
        for child in el.children.iter().rev() {
            if let Node::Element(inner) = child {
                stack.push(inner);
            }
        }
    }
}

fn is_counted_blip(el: &Element) -> bool {
    el.local_name() == "blip"
        && (el.attr_local("embed").is_some_and(|s| !s.is_empty())
            || crate::svg::blip_rid(el).is_some())
}

pub fn export_pic(
    pkg: &Package,
    slide_part: &str,
    cm: &picture::CommentPic,
    key: &picture::PicKey,
    id: &str,
) -> Result<ExportedMedia, Error> {
    let picture::PicKey::Index(k) = key else {
        return Err(Error::with_path(
            ErrorKind::PictureNotFound { id: id.to_string() },
            "id",
        ));
    };
    let part = comments_part(pkg, slide_part)?
        .ok_or_else(|| Error::with_path(ErrorKind::PictureNotFound { id: id.to_string() }, "id"))?;
    let doc = xml::parse(pkg.get(&part)?)?;
    let host = find_comment_host(&doc.root, cm)
        .ok_or_else(|| Error::with_path(ErrorKind::PictureNotFound { id: id.to_string() }, "id"))?;
    let skip_reply_lst = cm.reply.is_none();
    let blips = collect_counted_blips(host, skip_reply_lst);
    let blip = blips
        .get(*k - 1)
        .ok_or_else(|| Error::with_path(ErrorKind::PictureNotFound { id: id.to_string() }, "id"))?;
    let rid = crate::svg::blip_rid(blip)
        .or_else(|| {
            blip.attr_local("embed")
                .filter(|s| !s.is_empty())
                .map(str::to_string)
        })
        .ok_or_else(|| Error::with_path(ErrorKind::PictureNotFound { id: id.to_string() }, "id"))?;
    let media = opc::resolve_id(pkg, &part, &rid)?;
    let media_bytes = pkg.get(&media)?.to_vec();
    let cts = ContentTypes::from_package(pkg)?;
    let content_type = cts.content_type_of(&media).map(str::to_string);
    Ok(ExportedMedia {
        bytes: media_bytes,
        content_type,
    })
}

fn comment_id_of(target: &CommentTarget) -> String {
    match &target.key {
        CommentKey::Top(g) => format!("slide:{}/comment:{g}", target.slide),
        CommentKey::Reply { parent, reply } => {
            format!("slide:{}/comment:{parent}/reply:{reply}", target.slide)
        }
    }
}

fn key_from_pic(cm: &picture::CommentPic) -> CommentKey {
    match &cm.reply {
        None => CommentKey::Top(cm.guid.clone()),
        Some(r) => CommentKey::Reply {
            parent: cm.guid.clone(),
            reply: r.clone(),
        },
    }
}

fn skip_reply_lst(key: &CommentKey) -> bool {
    matches!(key, CommentKey::Top(_))
}

fn insert_blip(host: &mut Element, rid: &str, svg_rid: Option<&str>) {
    let mut blip = Element::new("a:blip").with_attr("r:embed", rid);
    if let Some(svg) = svg_rid {
        crate::svg::set_svg_blip(&mut blip, svg);
    }
    if let Some(i) = host
        .children
        .iter()
        .position(|n| matches!(n, Node::Element(e) if e.local_name() == "extLst"))
    {
        host.children.insert(i, Node::Element(blip));
    } else {
        host.push_element(blip);
    }
}

fn remove_nth_blip(host: &mut Element, skip: bool, index: usize) -> Option<Vec<String>> {
    let mut remaining = index;
    remove_nth_blip_walk(host, skip, &mut remaining)
}

fn remove_nth_blip_walk(
    el: &mut Element,
    skip: bool,
    remaining: &mut usize,
) -> Option<Vec<String>> {
    if skip && el.local_name() == "replyLst" {
        return None;
    }
    let mut i = 0;
    while i < el.children.len() {
        let is_target = matches!(
            &el.children[i],
            Node::Element(child) if is_counted_blip(child)
        );
        if is_target {
            if *remaining == 0 {
                let rids = match &el.children[i] {
                    Node::Element(child) => blip_image_rids(child),
                    _ => Vec::new(),
                };
                el.children.remove(i);
                return Some(rids);
            }
            *remaining = remaining.saturating_sub(1);
            i += 1;
            continue;
        }
        if let Node::Element(child) = &mut el.children[i] {
            if let Some(rids) = remove_nth_blip_walk(child, skip, remaining) {
                return Some(rids);
            }
        }
        i += 1;
    }
    None
}

fn blip_image_rids(blip: &Element) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(r) = blip
        .attr_local("embed")
        .filter(|s| !s.is_empty())
        .map(str::to_string)
    {
        out.push(r);
    }
    if let Some(r) = crate::svg::blip_rid(blip) {
        if !out.iter().any(|x| x == &r) {
            out.push(r);
        }
    }
    out
}

fn map_nth_blip<F>(
    el: &mut Element,
    skip: bool,
    remaining: &mut usize,
    f: &mut F,
) -> Result<bool, Error>
where
    F: FnMut(&mut Element) -> Result<(), Error>,
{
    if skip && el.local_name() == "replyLst" {
        return Ok(false);
    }
    let mut i = 0;
    while i < el.children.len() {
        let is_blip = matches!(&el.children[i], Node::Element(c) if is_counted_blip(c));
        if is_blip {
            if *remaining == 0 {
                let Node::Element(child) = &mut el.children[i] else {
                    return Ok(false);
                };
                f(child)?;
                return Ok(true);
            }
            *remaining = remaining.saturating_sub(1);
            i += 1;
            continue;
        }
        if let Node::Element(child) = &mut el.children[i] {
            if map_nth_blip(child, skip, remaining, f)? {
                return Ok(true);
            }
        }
        i += 1;
    }
    Ok(false)
}

fn blip_still_on_host(host: &Element, skip: bool, rid: &str) -> bool {
    collect_counted_blips(host, skip).iter().any(|b| {
        b.attr_local("embed") == Some(rid) || crate::svg::blip_rid(b).as_deref() == Some(rid)
    })
}

pub fn insert_picture(
    pkg: &mut Package,
    meta: &SlideMeta,
    op: &InsertPictureOp,
    op_index: usize,
) -> Result<(), Error> {
    let target = op.comment.as_ref().ok_or_else(|| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::PptxCommentForm,
            },
            format!("ops[{op_index}].id"),
        )
    })?;
    let part = comments_part(pkg, &meta.part)?.ok_or_else(|| {
        Error::with_path(
            ErrorKind::CommentNotFound {
                id: comment_id_of(target),
            },
            format!("ops[{op_index}].id"),
        )
    })?;
    let mut doc = xml::parse(pkg.get(&part)?)?;
    {
        let host = find_host_mut(&mut doc.root, &target.key).ok_or_else(|| {
            Error::with_path(
                ErrorKind::CommentNotFound {
                    id: comment_id_of(target),
                },
                format!("ops[{op_index}].id"),
            )
        })?;
        reject_ink(host, &comment_id_of(target), op_index)?;
    }
    let image = picture::read_image(&op.file).map_err(|e| e.at(format!("ops[{op_index}].file")))?;
    let (rid, svg_rid) = picture::write_picture_media(pkg, &part, &image)?;
    picture::ensure_xmlns(&mut doc.root, "r", REL_NS);
    picture::ensure_xmlns(&mut doc.root, "a", DML_NS);
    let host = find_host_mut(&mut doc.root, &target.key).ok_or_else(|| {
        Error::with_path(
            ErrorKind::CommentNotFound {
                id: comment_id_of(target),
            },
            format!("ops[{op_index}].id"),
        )
    })?;
    insert_blip(host, &rid, svg_rid.as_deref());
    pkg.set(&part, doc.to_bytes()?)?;
    Ok(())
}

pub fn replace_picture(
    pkg: &mut Package,
    meta: &SlideMeta,
    op: &ReplacePictureOp,
    op_index: usize,
) -> Result<(), Error> {
    let cm = op.target.comment.as_ref().ok_or_else(|| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SlidePicPrefix,
            },
            format!("ops[{op_index}].id"),
        )
    })?;
    let key = key_from_pic(cm);
    let skip = skip_reply_lst(&key);
    let k = match &op.target.key {
        picture::PicKey::Index(k) => *k,
        picture::PicKey::Id(_) => {
            return Err(Error::with_path(
                ErrorKind::PictureNotFound { id: op.id.clone() },
                format!("ops[{op_index}].id"),
            ));
        }
    };
    if k < 1 {
        return Err(Error::with_path(
            ErrorKind::PictureNotFound { id: op.id.clone() },
            format!("ops[{op_index}].id"),
        ));
    }
    let part = comments_part(pkg, &meta.part)?.ok_or_else(|| {
        Error::with_path(
            ErrorKind::PictureNotFound { id: op.id.clone() },
            format!("ops[{op_index}].id"),
        )
    })?;
    let mut doc = xml::parse(pkg.get(&part)?)?;
    let image = picture::read_image(&op.file).map_err(|e| e.at(format!("ops[{op_index}].file")))?;
    let dirty = {
        let host = find_host_mut(&mut doc.root, &key).ok_or_else(|| {
            Error::with_path(
                ErrorKind::PictureNotFound { id: op.id.clone() },
                format!("ops[{op_index}].id"),
            )
        })?;
        let mut remaining = k - 1;
        let mut xml_dirty = false;
        let found = map_nth_blip(host, skip, &mut remaining, &mut |blip| {
            xml_dirty = picture::apply_image_to_host(pkg, &part, blip, &image)?;
            Ok(())
        })?;
        if !found {
            return Err(Error::with_path(
                ErrorKind::PictureNotFound { id: op.id.clone() },
                format!("ops[{op_index}].id"),
            ));
        }
        xml_dirty
    };
    if dirty {
        pkg.set(&part, doc.to_bytes()?)?;
    }
    Ok(())
}

pub fn delete_picture(
    pkg: &mut Package,
    meta: &SlideMeta,
    op: &DeletePictureOp,
    op_index: usize,
) -> Result<(), Error> {
    let cm = op.target.comment.as_ref().ok_or_else(|| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SlidePicPrefix,
            },
            format!("ops[{op_index}].id"),
        )
    })?;
    let key = key_from_pic(cm);
    let skip = skip_reply_lst(&key);
    let k = match &op.target.key {
        picture::PicKey::Index(k) => *k,
        picture::PicKey::Id(_) => {
            return Err(Error::with_path(
                ErrorKind::PictureNotFound { id: op.id.clone() },
                format!("ops[{op_index}].id"),
            ));
        }
    };
    if k < 1 {
        return Err(Error::with_path(
            ErrorKind::PictureNotFound { id: op.id.clone() },
            format!("ops[{op_index}].id"),
        ));
    }
    let part = comments_part(pkg, &meta.part)?.ok_or_else(|| {
        Error::with_path(
            ErrorKind::PictureNotFound { id: op.id.clone() },
            format!("ops[{op_index}].id"),
        )
    })?;
    let mut doc = xml::parse(pkg.get(&part)?)?;
    let rids = {
        let host = find_host_mut(&mut doc.root, &key).ok_or_else(|| {
            Error::with_path(
                ErrorKind::PictureNotFound { id: op.id.clone() },
                format!("ops[{op_index}].id"),
            )
        })?;
        remove_nth_blip(host, skip, k - 1).ok_or_else(|| {
            Error::with_path(
                ErrorKind::PictureNotFound { id: op.id.clone() },
                format!("ops[{op_index}].id"),
            )
        })?
    };
    pkg.set(&part, doc.to_bytes()?)?;
    for rid in rids {
        let old_media = opc::resolve_id(pkg, &part, &rid).ok();
        let still = find_host_mut(&mut doc.root, &key)
            .map(|host| blip_still_on_host(host, skip, &rid))
            .unwrap_or(false);
        if !still {
            let mut rels = Relationships::from_package(pkg, &part)?;
            rels.remove_by_id(&rid);
            rels.write_to(pkg, &part)?;
        }
        if let Some(old) = old_media {
            if !picture::media_still_used(pkg, &old)? {
                pkg.remove(&old);
            }
        }
    }
    Ok(())
}

fn find_comment_host<'a>(root: &'a Element, cm: &picture::CommentPic) -> Option<&'a Element> {
    let parent = cm_elements(root).find(|e| e.attr("id") == Some(cm.guid.as_str()))?;
    match &cm.reply {
        None => Some(parent),
        Some(rg) => parent
            .child_by_local("replyLst")?
            .element_children()
            .find(|e| e.local_name() == "reply" && e.attr("id") == Some(rg.as_str())),
    }
}

fn author_by_id(authors: &[Author], id: &str) -> (Value, Value) {
    match authors.iter().find(|a| a.id == id) {
        Some(a) => (json!(a.name), json!(a.initials)),
        None => (Value::Null, Value::Null),
    }
}

fn pos_xy(cm: &Element) -> (Option<i64>, Option<i64>) {
    let Some(pos) = cm.child_by_local("pos") else {
        return (None, None);
    };
    (
        pos.attr("x").and_then(|s| s.parse().ok()),
        pos.attr("y").and_then(|s| s.parse().ok()),
    )
}

fn status_done(el: &Element) -> bool {
    matches!(el.attr("status"), Some("resolved") | Some("closed"))
}

fn comment_kind(el: &Element) -> &'static str {
    if has_ink(el) {
        "ink"
    } else {
        "normal"
    }
}

fn has_ink(el: &Element) -> bool {
    if el.local_name() == "ink" {
        return true;
    }
    el.element_children().any(has_ink)
}

pub fn apply_insert(
    pkg: &mut Package,
    pres_part: &str,
    pres: &Document,
    meta: &SlideMeta,
    op: &InsertCommentOp,
    op_index: usize,
) -> Result<(), Error> {
    let _ = op_index;
    let sld_id = slide_numeric_id(&pres.root, meta.index).unwrap_or(256);
    let (x, y) = match (op.x, op.y) {
        (Some(px), Some(py)) => (px, py),
        _ => {
            let doc = xml::parse(pkg.get(&meta.part)?)?;
            picture::vacant_pos(&pres.root, &doc.root, 914_400, 914_400)
        }
    };
    let author_id = ensure_author(pkg, pres_part, &op.author, &op.initials)?;
    let created = op.date.clone().unwrap_or_else(utc_now);
    let guid = new_guid();
    let mut slide_doc = xml::parse(pkg.get(&meta.part)?)?;
    let part = ensure_comments_part(pkg, &meta.part, &mut slide_doc, sld_id)?;
    pkg.set(&meta.part, slide_doc.to_bytes()?)?;
    let mut cdoc = xml::parse(pkg.get(&part)?)?;
    let cm = build_cm(&guid, &author_id, &created, x, y, sld_id, &op.text);
    cdoc.root.push_element(cm);
    pkg.set(&part, cdoc.to_bytes()?)?;
    Ok(())
}

pub fn apply_reply(
    pkg: &mut Package,
    pres_part: &str,
    meta: &SlideMeta,
    op: &ReplyCommentOp,
    op_index: usize,
) -> Result<(), Error> {
    let target = parse_comment_id(&op.id, &format!("ops[{op_index}].id"))?;
    let CommentKey::Top(parent) = &target.key else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::PptxCommentForm,
            },
            format!("ops[{op_index}].id"),
        ));
    };
    let part = comments_part(pkg, &meta.part)?.ok_or_else(|| {
        Error::with_path(
            ErrorKind::CommentNotFound { id: op.id.clone() },
            format!("ops[{op_index}].id"),
        )
    })?;
    {
        let mut doc = xml::parse(pkg.get(&part)?)?;
        let cm = find_cm_mut(&mut doc.root, parent).ok_or_else(|| {
            Error::with_path(
                ErrorKind::CommentNotFound { id: op.id.clone() },
                format!("ops[{op_index}].id"),
            )
        })?;
        reject_ink(cm, &op.id, op_index)?;
    }
    let author_id = ensure_author(pkg, pres_part, &op.author, &op.initials)?;
    let created = op.date.clone().unwrap_or_else(utc_now);
    let reply_id = new_guid();
    let reply = build_reply(&reply_id, &author_id, &created, &op.text);
    let mut doc = xml::parse(pkg.get(&part)?)?;
    let cm = find_cm_mut(&mut doc.root, parent).ok_or_else(|| {
        Error::with_path(
            ErrorKind::CommentNotFound { id: op.id.clone() },
            format!("ops[{op_index}].id"),
        )
    })?;
    if cm.child_by_local("replyLst").is_none() {
        insert_before_txbody(cm, Element::new("p188:replyLst"));
    }
    let Some(lst) = cm.child_by_local_mut("replyLst") else {
        return Err(Error::with_path(
            ErrorKind::CommentNotFound { id: op.id.clone() },
            format!("ops[{op_index}].id"),
        ));
    };
    lst.push_element(reply);
    pkg.set(&part, doc.to_bytes()?)?;
    Ok(())
}

pub fn apply_delete(
    pkg: &mut Package,
    meta: &SlideMeta,
    op: &DeleteCommentOp,
    op_index: usize,
) -> Result<(), Error> {
    let target = parse_comment_id(&op.id, &format!("ops[{op_index}].id"))?;
    let part = comments_part(pkg, &meta.part)?.ok_or_else(|| {
        Error::with_path(
            ErrorKind::CommentNotFound { id: op.id.clone() },
            format!("ops[{op_index}].id"),
        )
    })?;
    let mut doc = xml::parse(pkg.get(&part)?)?;
    match &target.key {
        CommentKey::Top(guid) => {
            if !remove_cm(&mut doc.root, guid) {
                return Err(Error::with_path(
                    ErrorKind::CommentNotFound { id: op.id.clone() },
                    format!("ops[{op_index}].id"),
                ));
            }
        }
        CommentKey::Reply { parent, reply } => {
            let cm = find_cm_mut(&mut doc.root, parent).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::CommentNotFound { id: op.id.clone() },
                    format!("ops[{op_index}].id"),
                )
            })?;
            if !remove_reply(cm, reply) {
                return Err(Error::with_path(
                    ErrorKind::CommentNotFound { id: op.id.clone() },
                    format!("ops[{op_index}].id"),
                ));
            }
        }
    }
    pkg.set(&part, doc.to_bytes()?)?;
    Ok(())
}

pub fn apply_set_text(
    pkg: &mut Package,
    meta: &SlideMeta,
    op: &SetCommentTextOp,
    op_index: usize,
) -> Result<(), Error> {
    let target = parse_comment_id(&op.id, &format!("ops[{op_index}].id"))?;
    let part = comments_part(pkg, &meta.part)?.ok_or_else(|| {
        Error::with_path(
            ErrorKind::CommentNotFound { id: op.id.clone() },
            format!("ops[{op_index}].id"),
        )
    })?;
    let mut doc = xml::parse(pkg.get(&part)?)?;
    let host = find_host_mut(&mut doc.root, &target.key).ok_or_else(|| {
        Error::with_path(
            ErrorKind::CommentNotFound { id: op.id.clone() },
            format!("ops[{op_index}].id"),
        )
    })?;
    reject_ink(host, &op.id, op_index)?;
    let tx = host.child_by_local_mut("txBody").ok_or_else(|| {
        Error::with_path(
            ErrorKind::CommentNotFound { id: op.id.clone() },
            format!("ops[{op_index}].id"),
        )
    })?;
    shape::set_txbody_text(tx, &op.text);
    pkg.set(&part, doc.to_bytes()?)?;
    Ok(())
}

pub fn apply_set_meta(
    pkg: &mut Package,
    pres_part: &str,
    meta: &SlideMeta,
    op: &SetCommentMetaOp,
    op_index: usize,
) -> Result<(), Error> {
    let target = parse_comment_id(&op.id, &format!("ops[{op_index}].id"))?;
    let part = comments_part(pkg, &meta.part)?.ok_or_else(|| {
        Error::with_path(
            ErrorKind::CommentNotFound { id: op.id.clone() },
            format!("ops[{op_index}].id"),
        )
    })?;
    let mut doc = xml::parse(pkg.get(&part)?)?;
    let host = find_host_mut(&mut doc.root, &target.key).ok_or_else(|| {
        Error::with_path(
            ErrorKind::CommentNotFound { id: op.id.clone() },
            format!("ops[{op_index}].id"),
        )
    })?;
    if let Some(date) = &op.date {
        host.set_attr("created", date);
    }
    if op.author.is_some() || op.initials.is_some() {
        let cur = host.attr("authorId").unwrap_or("").to_string();
        let authors = load_authors(pkg)?;
        let (mut name, mut initials) = authors
            .iter()
            .find(|a| a.id == cur)
            .map(|a| (a.name.clone(), a.initials.clone()))
            .unwrap_or_else(|| ("ooxp".into(), "oo".into()));
        if let Some(a) = &op.author {
            name = a.clone();
        }
        if let Some(i) = &op.initials {
            initials = i.clone();
        }
        let aid = ensure_author(pkg, pres_part, &name, &initials)?;
        host.set_attr("authorId", &aid);
    }
    pkg.set(&part, doc.to_bytes()?)?;
    Ok(())
}

pub fn apply_set_done(
    pkg: &mut Package,
    meta: &SlideMeta,
    op: &SetCommentDoneOp,
    op_index: usize,
) -> Result<(), Error> {
    let target = parse_comment_id(&op.id, &format!("ops[{op_index}].id"))?;
    let part = comments_part(pkg, &meta.part)?.ok_or_else(|| {
        Error::with_path(
            ErrorKind::CommentNotFound { id: op.id.clone() },
            format!("ops[{op_index}].id"),
        )
    })?;
    let mut doc = xml::parse(pkg.get(&part)?)?;
    let host = find_host_mut(&mut doc.root, &target.key).ok_or_else(|| {
        Error::with_path(
            ErrorKind::CommentNotFound { id: op.id.clone() },
            format!("ops[{op_index}].id"),
        )
    })?;
    if op.done {
        host.set_attr("status", "resolved");
    } else {
        host.remove_attr("status");
    }
    pkg.set(&part, doc.to_bytes()?)?;
    Ok(())
}

fn reject_ink(el: &Element, id: &str, op_index: usize) -> Result<(), Error> {
    if has_ink(el) {
        return Err(Error::with_path(
            ErrorKind::InkComment { id: id.to_string() },
            format!("ops[{op_index}].id"),
        ));
    }
    Ok(())
}

fn find_host_mut<'a>(root: &'a mut Element, key: &CommentKey) -> Option<&'a mut Element> {
    match key {
        CommentKey::Top(g) => find_cm_mut(root, g),
        CommentKey::Reply { parent, reply } => {
            let cm = find_cm_mut(root, parent)?;
            let lst = cm.child_by_local_mut("replyLst")?;
            lst.children.iter_mut().find_map(|n| match n {
                Node::Element(e)
                    if e.local_name() == "reply" && e.attr("id") == Some(reply.as_str()) =>
                {
                    Some(e)
                }
                _ => None,
            })
        }
    }
}

fn find_cm_mut<'a>(root: &'a mut Element, guid: &str) -> Option<&'a mut Element> {
    root.children.iter_mut().find_map(|n| match n {
        Node::Element(e) if e.local_name() == "cm" && e.attr("id") == Some(guid) => Some(e),
        _ => None,
    })
}

fn remove_cm(root: &mut Element, guid: &str) -> bool {
    let Some(i) = root.children.iter().position(
        |n| matches!(n, Node::Element(e) if e.local_name() == "cm" && e.attr("id") == Some(guid)),
    ) else {
        return false;
    };
    root.children.remove(i);
    true
}

fn remove_reply(cm: &mut Element, guid: &str) -> bool {
    let Some(lst) = cm.child_by_local_mut("replyLst") else {
        return false;
    };
    let Some(i) = lst.children.iter().position(|n| {
        matches!(n, Node::Element(e) if e.local_name() == "reply" && e.attr("id") == Some(guid))
    }) else {
        return false;
    };
    lst.children.remove(i);
    true
}

fn insert_before_txbody(cm: &mut Element, el: Element) {
    if let Some(i) = cm
        .children
        .iter()
        .position(|n| matches!(n, Node::Element(e) if e.local_name() == "txBody"))
    {
        cm.children.insert(i, Node::Element(el));
    } else {
        cm.push_element(el);
    }
}

fn ensure_comments_part(
    pkg: &mut Package,
    slide_part: &str,
    slide_doc: &mut Document,
    sld_id: u32,
) -> Result<String, Error> {
    if let Some(part) = comments_part(pkg, slide_part)? {
        return Ok(part);
    }
    let part = format!("/ppt/comments/modernComment_{sld_id}_0.xml");
    let mut n = 0u32;
    let part = loop {
        let name = if n == 0 {
            part.clone()
        } else {
            format!("/ppt/comments/modernComment_{sld_id}_{n}.xml")
        };
        if !pkg.contains(&name) {
            break name;
        }
        n += 1;
    };
    pkg.set(&part, empty_cm_lst().into_bytes())?;
    let mut rels = Relationships::from_package(pkg, slide_part)?;
    let rid = rels.next_rid();
    rels.push(Relationship {
        id: rid.clone(),
        rel_type: REL_MODERN_COMMENTS.into(),
        target: picture::rel_target_from(slide_part, &part),
        target_mode: None,
        extra: Vec::new(),
    });
    rels.write_to(pkg, slide_part)?;
    ensure_comment_rel_ext(&mut slide_doc.root, &rid);
    let mut cts = ContentTypes::from_package(pkg)?;
    cts.set_override(&part, CT_MODERN_COMMENTS)?;
    cts.write_to(pkg)?;
    Ok(part)
}

fn ensure_comment_rel_ext(root: &mut Element, rid: &str) {
    picture::ensure_xmlns(root, "r", REL_NS);
    picture::ensure_xmlns(root, "p188", P188_NS);
    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?><p:ext xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:r="{REL_NS}" xmlns:p188="{P188_NS}" uri="{COMMENT_EXT_URI}"><p188:commentRel r:id="{rid}"/></p:ext>"#
    );
    let Ok(frag) = xml::parse(xml.as_bytes()) else {
        return;
    };
    if let Some(lst) = root.child_by_local_mut("extLst") {
        if lst
            .element_children()
            .any(|e| e.local_name() == "ext" && e.attr("uri") == Some(COMMENT_EXT_URI))
        {
            return;
        }
        lst.push_element(frag.root);
        return;
    }
    let mut lst = Element::new(root.prefixed("extLst"));
    lst.push_element(frag.root);
    root.push_element(lst);
}

fn ensure_author(
    pkg: &mut Package,
    pres_part: &str,
    name: &str,
    initials: &str,
) -> Result<String, Error> {
    let mut rels = Relationships::from_package(pkg, pres_part)?;
    let part = if let Some(rel) = rels.rels.iter().find(|r| r.rel_type == REL_AUTHORS) {
        opc::resolve_id(pkg, pres_part, &rel.id)?
    } else {
        pkg.set(AUTHORS_PART, empty_authors().into_bytes())?;
        let rid = rels.next_rid();
        rels.push(Relationship {
            id: rid,
            rel_type: REL_AUTHORS.into(),
            target: picture::rel_target_from(pres_part, AUTHORS_PART),
            target_mode: None,
            extra: Vec::new(),
        });
        rels.write_to(pkg, pres_part)?;
        let mut cts = ContentTypes::from_package(pkg)?;
        cts.set_override(AUTHORS_PART, CT_AUTHORS)?;
        cts.write_to(pkg)?;
        AUTHORS_PART.to_string()
    };
    let mut doc = xml::parse(pkg.get(&part)?)?;
    if let Some(existing) = doc.root.element_children().find(|e| {
        e.local_name() == "author"
            && e.attr("name") == Some(name)
            && e.attr("initials") == Some(initials)
    }) {
        return Ok(existing.attr("id").unwrap_or("").to_string());
    }
    let id = new_guid();
    let user_id: String = id
        .chars()
        .filter(|c| c.is_ascii_hexdigit())
        .take(16)
        .collect::<String>()
        .to_ascii_lowercase();
    let author = Element::new("p188:author")
        .with_attr("id", &id)
        .with_attr("name", name)
        .with_attr("initials", initials)
        .with_attr("userId", user_id)
        .with_attr("providerId", "None");
    doc.root.push_element(author);
    pkg.set(&part, doc.to_bytes()?)?;
    Ok(id)
}

fn empty_cm_lst() -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><p188:cmLst xmlns:a="{DML_NS}" xmlns:r="{REL_NS}" xmlns:p188="{P188_NS}"/>"#
    )
}

fn empty_authors() -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><p188:authorLst xmlns:a="{DML_NS}" xmlns:r="{REL_NS}" xmlns:p188="{P188_NS}"/>"#
    )
}

fn build_cm(
    guid: &str,
    author_id: &str,
    created: &str,
    x: i64,
    y: i64,
    sld_id: u32,
    text: &str,
) -> Element {
    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?><p188:cm xmlns:a="{DML_NS}" xmlns:p188="{P188_NS}" xmlns:pc="{PC_NS}" id="{guid}" authorId="{author_id}" created="{created}"><pc:sldMkLst><pc:docMk/><pc:sldMk cId="0" sldId="{sld_id}"/></pc:sldMkLst><p188:pos x="{x}" y="{y}"/>{tx}</p188:cm>"#,
        tx = tx_body_xml(text)
    );
    xml::parse(xml.as_bytes())
        .map(|d| d.root)
        .unwrap_or_else(|_| Element::new("p188:cm"))
}

fn build_reply(guid: &str, author_id: &str, created: &str, text: &str) -> Element {
    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?><p188:reply xmlns:a="{DML_NS}" xmlns:p188="{P188_NS}" id="{guid}" authorId="{author_id}" created="{created}">{tx}</p188:reply>"#,
        tx = tx_body_xml(text)
    );
    xml::parse(xml.as_bytes())
        .map(|d| d.root)
        .unwrap_or_else(|_| Element::new("p188:reply"))
}

fn tx_body_xml(text: &str) -> String {
    let esc = xml_escape(text);
    format!(
        r#"<p188:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:rPr lang="zh-CN" altLang="en-US"/><a:t>{esc}</a:t></a:r></a:p></p188:txBody>"#
    )
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn slide_numeric_id(pres: &Element, index: usize) -> Option<u32> {
    let lst = pres.child_by_local("sldIdLst")?;
    lst.element_children()
        .filter(|e| e.local_name() == "sldId")
        .nth(index.saturating_sub(1))
        .and_then(|e| e.attr("id"))
        .and_then(|s| s.parse().ok())
}

fn new_guid() -> String {
    let t = crate::host_env::now_nanos();
    let n = GUID_SEQ.fetch_add(1, Ordering::Relaxed) as u128;
    let pid = crate::host_env::pid();
    let mix = t ^ (n << 48) ^ (pid << 16);
    let mut b = [0u8; 16];
    for (i, byte) in b.iter_mut().enumerate() {
        *byte = (mix >> ((i % 16) * 8)) as u8 ^ (i as u8).wrapping_mul(31);
    }
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    format!(
        "{{{:02X}{:02X}{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}}}",
        b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7], b[8], b[9], b[10], b[11], b[12], b[13], b[14], b[15]
    )
}

fn utc_now() -> String {
    let (secs, millis) = crate::host_env::unix_secs_millis();
    unix_to_dt(secs, millis)
}

fn unix_to_dt(secs: u64, millis: u32) -> String {
    let mut days = secs / 86_400;
    let rem = secs % 86_400;
    let hour = rem / 3600;
    let min = (rem % 3600) / 60;
    let sec = rem % 60;
    let mut year = 1970u64;
    loop {
        let len = if is_leap(year) { 366 } else { 365 };
        if days < len {
            break;
        }
        days -= len;
        year += 1;
    }
    let dim = [
        31u64,
        if is_leap(year) { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut month = 1u64;
    for (i, d) in dim.iter().enumerate() {
        if days < *d {
            month = i as u64 + 1;
            break;
        }
        days -= *d;
        month = i as u64 + 2;
    }
    let day = days + 1;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{min:02}:{sec:02}.{millis:03}")
}

fn is_leap(year: u64) -> bool {
    year.is_multiple_of(4) && !year.is_multiple_of(100) || year.is_multiple_of(400)
}
