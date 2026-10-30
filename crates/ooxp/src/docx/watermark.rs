//! 页眉 DrawingML 水印：inspect / 插 / 删。不编辑 VML。

use std::collections::HashMap;

use serde_json::{json, Value};

use crate::error::{Error, ErrorKind, IdReason, TypeName};
use crate::op_fields::require_id;
use crate::opc::{office_document_part, ContentTypes, Package, Relationship, Relationships};
use crate::xml::{self, Document, Element, Node};

use super::hf::{self, HfKind, HfPart, HfType};
use super::para;
use super::picture::{self, AlignH, AlignV, PicKind, WrapKind, WrapPos};
use super::textbox::{self, WPS_NS};
use super::{CT_HEADER, REL_HEADER, REL_NS, W14_NS};

/// 文字水印 `wp:docPr/@name`。
pub const NAME_TEXT: &str = "ooxpWatermarkText";
/// 图片水印 `wp:docPr/@name`。
pub const NAME_PIC: &str = "ooxpWatermarkPicture";
/// 扩展标记，inspect 也认。
pub const EXT_URI: &str = "{C0A5BD15-81BF-4B96-A0C4-0C0A5BD15001}";

const TEXT_CX: i64 = 5_400_000;
const TEXT_CY: i64 = 2_160_000;
const ROT_315: &str = "18900000";

#[derive(Debug, Clone)]
pub struct InsertWatermarkOp {
    pub text: Option<String>,
    pub file: Option<String>,
}

#[derive(Debug, Clone)]
pub enum DeleteWatermarkOp {
    All,
    One {
        kind: HfKind,
        hf_type: HfType,
        n: u32,
        key: WmKey,
    },
}

#[derive(Debug, Clone)]
pub enum WmKey {
    DocPr(String),
    Index(usize),
}

pub struct WmInfo {
    pub id: String,
    pub kind: &'static str,
    pub name: Option<String>,
    pub text: Option<String>,
    pub stable: bool,
}

/// 名称前缀或扩展 uri 命中即为水印 drawing。
pub fn is_watermark_drawing(drawing: &Element) -> bool {
    if drawing.local_name() != "drawing" {
        return false;
    }
    if let Some(name) = doc_pr_name(drawing) {
        if is_watermark_name(&name) {
            return true;
        }
    }
    has_ext_uri(drawing, EXT_URI)
}

fn is_watermark_name(name: &str) -> bool {
    name.starts_with("ooxpWatermark")
        || name.starts_with("PowerPlusWaterMarkObject")
        || name.starts_with("WordPictureWatermark")
}

fn doc_pr_name(drawing: &Element) -> Option<String> {
    frame(drawing)
        .and_then(|f| f.child_by_local("docPr"))
        .and_then(|d| d.attr_local("name"))
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn doc_pr_id(drawing: &Element) -> Option<String> {
    frame(drawing)
        .and_then(|f| f.child_by_local("docPr"))
        .and_then(|d| d.attr_local("id"))
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn frame(drawing: &Element) -> Option<&Element> {
    drawing
        .element_children()
        .find(|e| e.local_name() == "inline" || e.local_name() == "anchor")
}

fn has_ext_uri(el: &Element, uri: &str) -> bool {
    if el.local_name() == "ext" && el.attr_local("uri").is_some_and(|u| u == uri) {
        return true;
    }
    el.element_children().any(|c| has_ext_uri(c, uri))
}

fn has_local(el: &Element, local: &str) -> bool {
    crate::xml::find_by_local(el, local).is_some()
}

fn wm_kind(drawing: &Element) -> &'static str {
    if has_local(drawing, "pic") && has_local(drawing, "blip") {
        "picture"
    } else {
        "text"
    }
}

fn visible_text(el: &Element, out: &mut String) {
    if el.local_name() == "t" {
        out.push_str(&el.text_content());
        return;
    }
    for n in &el.children {
        match n {
            Node::Element(e) => visible_text(e, out),
            Node::Text(t) if el.local_name() == "t" => out.push_str(t),
            Node::Text(_) | Node::Raw(_) => {}
        }
    }
}

pub fn collect(root: &Element, container_id: &str) -> Vec<WmInfo> {
    let mut out = Vec::new();
    let mut index = 0usize;
    let _ = picture::walk_drawings(root, &mut |drawing| {
        if !is_watermark_drawing(drawing) {
            return Ok(());
        }
        index += 1;
        let name = doc_pr_name(drawing);
        let kind = wm_kind(drawing);
        let text = if kind == "text" {
            let mut s = String::new();
            visible_text(drawing, &mut s);
            Some(s)
        } else {
            None
        };
        let (id, stable) = match doc_pr_id(drawing) {
            Some(v) => (format!("{container_id}/watermark:{v}"), true),
            None => (format!("{container_id}/watermark:index:{index}"), false),
        };
        out.push(WmInfo {
            id,
            kind,
            name,
            text,
            stable,
        });
        Ok(())
    });
    out
}

pub fn infos_json(items: &[WmInfo]) -> Vec<Value> {
    items
        .iter()
        .map(|w| {
            json!({
                "id": w.id,
                "kind": w.kind,
                "name": w.name,
                "text": w.text,
                "stable": w.stable,
            })
        })
        .collect()
}

pub fn count_in_headers(pkg: &Package, catalog: &[HfPart]) -> Result<usize, Error> {
    let mut n = 0usize;
    for item in catalog {
        if item.kind != HfKind::Header {
            continue;
        }
        n += collect_part(pkg, item)?.len();
    }
    Ok(n)
}

fn collect_part(pkg: &Package, item: &HfPart) -> Result<Vec<WmInfo>, Error> {
    let Ok(bytes) = pkg.get(&item.part) else {
        return Ok(Vec::new());
    };
    let doc = xml::parse(bytes)?;
    Ok(collect(&doc.root, &item.id))
}

pub fn list_all(pkg: &Package, catalog: &[HfPart]) -> Result<Vec<Value>, Error> {
    let mut out = Vec::new();
    for item in catalog {
        if item.kind != HfKind::Header {
            continue;
        }
        out.extend(infos_json(&collect_part(pkg, item)?));
    }
    Ok(out)
}

pub fn apply_insert(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    op: &InsertWatermarkOp,
    i: usize,
) -> Result<(), Error> {
    ensure_section1_default_header(pkg, docs, dirty, doc_part)?;
    let catalog = {
        let doc = load_cached(pkg, docs, doc_part)?;
        hf::catalog(pkg, doc_part, &doc.root)?
    };
    let headers: Vec<HfPart> = catalog
        .into_iter()
        .filter(|h| h.kind == HfKind::Header)
        .collect();
    if headers.is_empty() {
        return Err(Error::with_path(
            ErrorKind::SectionNotFound {
                id: "section:1".into(),
            },
            format!("ops[{i}].id"),
        ));
    }
    for item in &headers {
        insert_into_header(pkg, docs, dirty, item, op, i)?;
    }
    Ok(())
}

pub fn apply_delete(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    op: &DeleteWatermarkOp,
    i: usize,
) -> Result<(), Error> {
    let catalog = {
        let doc = load_cached(pkg, docs, doc_part)?;
        hf::catalog(pkg, doc_part, &doc.root)?
    };
    match op {
        DeleteWatermarkOp::All => {
            for item in &catalog {
                if item.kind != HfKind::Header {
                    continue;
                }
                clear_header(pkg, docs, dirty, item)?;
            }
            Ok(())
        }
        DeleteWatermarkOp::One {
            kind,
            hf_type,
            n,
            key,
        } => {
            let container_id = format!("{}:{}:{n}", kind.as_str(), hf_type.as_str());
            let item = hf::find_by_id(&catalog, &container_id).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::WatermarkNotFound {
                        id: format!("{container_id}/watermark:…"),
                    },
                    format!("ops[{i}].id"),
                )
            })?;
            if item.kind != HfKind::Header {
                return Err(Error::with_path(
                    ErrorKind::InvalidId {
                        reason: IdReason::WatermarkPrefix,
                    },
                    format!("ops[{i}].id"),
                ));
            }
            let id_for_err = format!("{container_id}/watermark:{}", key_debug(key));
            delete_one(pkg, docs, dirty, item, key, &id_for_err, i)
        }
    }
}

fn key_debug(key: &WmKey) -> String {
    match key {
        WmKey::DocPr(s) => s.clone(),
        WmKey::Index(n) => format!("index:{n}"),
    }
}

pub fn apply_create(pkg: &mut Package, v: Option<&Value>) -> Result<(), Error> {
    let Some(v) = v else {
        return Ok(());
    };
    let arr = v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "document.watermarks".into(),
                expected: TypeName::Array,
            },
            "document.watermarks",
        )
    })?;
    if arr.is_empty() {
        return Ok(());
    }
    let doc_part = office_document_part(pkg)?;
    let mut docs = HashMap::new();
    let mut dirty = HashMap::new();
    // 多条时后者覆盖前者：契约是整篇一套水印。
    let last = arr.last().unwrap();
    let op = parse_create_item(last, arr.len() - 1)?;
    apply_insert(pkg, &mut docs, &mut dirty, &doc_part, &op, 0)?;
    for name in dirty.keys() {
        let doc = docs.get(name).unwrap();
        pkg.set(name, doc.to_bytes()?)?;
    }
    Ok(())
}

fn parse_create_item(v: &Value, i: usize) -> Result<InsertWatermarkOp, Error> {
    let path = format!("document.watermarks[{i}]");
    let obj = v.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "watermarks[]".into(),
                expected: TypeName::Object,
            },
            path.clone(),
        )
    })?;
    for k in obj.keys() {
        if !matches!(k.as_str(), "text" | "file") {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("{path}.{k}"),
            ));
        }
    }
    parse_text_or_file(obj, &path)
}

pub fn parse_insert(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertWatermarkOp, Error> {
    for k in obj.keys() {
        if !matches!(k.as_str(), "op" | "id" | "text" | "file") {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("ops[{i}].{k}"),
            ));
        }
    }
    let id = require_id(obj, i)?;
    if id != "docx:document" {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::InsertWatermarkNeedsDocument,
            },
            format!("ops[{i}].id"),
        ));
    }
    parse_text_or_file(obj, &format!("ops[{i}]"))
}

pub fn parse_delete(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteWatermarkOp, Error> {
    for k in obj.keys() {
        if !matches!(k.as_str(), "op" | "id") {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("ops[{i}].{k}"),
            ));
        }
    }
    let id = require_id(obj, i)?;
    if id == "docx:document" {
        return Ok(DeleteWatermarkOp::All);
    }
    parse_watermark_id(&id, i)
}

fn parse_text_or_file(
    obj: &serde_json::Map<String, Value>,
    path: &str,
) -> Result<InsertWatermarkOp, Error> {
    let text = match obj.get("text") {
        None => None,
        Some(Value::String(s)) if !s.is_empty() => Some(s.clone()),
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "text".into(),
                    expected: TypeName::NonEmptyString,
                },
                format!("{path}.text"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "text".into(),
                    expected: TypeName::String,
                },
                format!("{path}.text"),
            ));
        }
    };
    let file = match obj.get("file") {
        None => None,
        Some(Value::String(s)) if !s.is_empty() => Some(s.clone()),
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "file".into(),
                    expected: TypeName::NonEmptyString,
                },
                format!("{path}.file"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "file".into(),
                    expected: TypeName::String,
                },
                format!("{path}.file"),
            ));
        }
    };
    match (text.is_some(), file.is_some()) {
        (true, false) | (false, true) => Ok(InsertWatermarkOp { text, file }),
        _ => Err(Error::with_path(ErrorKind::WatermarkNeedTextOrFile, path)),
    }
}

fn parse_watermark_id(id: &str, i: usize) -> Result<DeleteWatermarkOp, Error> {
    let path = format!("ops[{i}].id");
    let Some((left, key_s)) = id.rsplit_once("/watermark:") else {
        if id.starts_with("header:") {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::WatermarkContainer,
                },
                path,
            ));
        }
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::WatermarkPrefix,
            },
            path,
        ));
    };
    let (kind, hf_type, n) = parse_header_container(left, &path)?;
    let key = parse_wm_key(key_s, &path)?;
    Ok(DeleteWatermarkOp::One {
        kind,
        hf_type,
        n,
        key,
    })
}

fn parse_header_container(s: &str, path: &str) -> Result<(HfKind, HfType, u32), Error> {
    let mut it = s.splitn(3, ':');
    let kind_s = it.next().unwrap_or("");
    let type_s = it.next().unwrap_or("");
    let n_s = it.next().unwrap_or("");
    if kind_s != "header" || type_s.is_empty() || n_s.is_empty() || it.next().is_some() {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::WatermarkPrefix,
            },
            path.to_string(),
        ));
    }
    let kind = HfKind::parse(kind_s).ok_or_else(|| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::WatermarkPrefix,
            },
            path.to_string(),
        )
    })?;
    let hf_type = HfType::parse(type_s).ok_or_else(|| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::WatermarkPrefix,
            },
            path.to_string(),
        )
    })?;
    let n: u32 = n_s.parse().map_err(|_| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::WatermarkPrefix,
            },
            path.to_string(),
        )
    })?;
    if n < 1 {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::WatermarkPrefix,
            },
            path.to_string(),
        ));
    }
    Ok((kind, hf_type, n))
}

fn parse_wm_key(s: &str, path: &str) -> Result<WmKey, Error> {
    if let Some(rest) = s.strip_prefix("index:") {
        let n: usize = rest.parse().map_err(|_| {
            Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::WatermarkIndexNotInt,
                },
                path.to_string(),
            )
        })?;
        if n < 1 {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::WatermarkIndexFromOne,
                },
                path.to_string(),
            ));
        }
        return Ok(WmKey::Index(n));
    }
    if s.is_empty() {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::WatermarkPrefix,
            },
            path.to_string(),
        ));
    }
    Ok(WmKey::DocPr(s.to_string()))
}

fn matches_key(drawing: &Element, key: &WmKey, index: usize) -> bool {
    match key {
        WmKey::Index(n) => index == *n,
        WmKey::DocPr(want) => match doc_pr_id(drawing) {
            Some(got) => {
                if let (Ok(a), Ok(b)) = (want.parse::<u32>(), got.parse::<u32>()) {
                    a == b
                } else {
                    want == &got
                }
            }
            None => false,
        },
    }
}

fn insert_into_header(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    item: &HfPart,
    op: &InsertWatermarkOp,
    _i: usize,
) -> Result<(), Error> {
    clear_header(pkg, docs, dirty, item)?;
    let drawing = if let Some(file) = op.file.as_deref() {
        let added = picture::add_media(pkg, &item.part, file, None, None)?;
        let doc = load_cached(pkg, docs, &item.part)?;
        ensure_wm_ns(&mut doc.root);
        let doc_pr = picture::next_docpr_id(&doc.root);
        let mut drawing = picture::build_inline_drawing(
            doc_pr,
            NAME_PIC,
            &added.blip_rid,
            added.svg_rid.as_deref(),
            added.cx,
            added.cy,
        );
        picture::apply_wrap(
            &mut drawing,
            PicKind::Anchor,
            Some(WrapKind::None),
            &WrapPos {
                align_h: Some(AlignH::Center),
                align_v: Some(AlignV::Center),
                offset_x: None,
                offset_y: None,
            },
        )?;
        set_page_center(&mut drawing);
        stamp_doc_pr(&mut drawing, NAME_PIC);
        drawing
    } else {
        let text = op.text.as_deref().unwrap_or("");
        let doc = load_cached(pkg, docs, &item.part)?;
        ensure_wm_ns(&mut doc.root);
        let doc_pr = picture::next_docpr_id(&doc.root);
        let para_id = para::next_para_hex(&doc.root);
        build_text_drawing(doc_pr, text, &para_id)
    };
    let doc = load_cached(pkg, docs, &item.part)?;
    ensure_wm_ns(&mut doc.root);
    let p = first_para_mut(&mut doc.root).ok_or_else(|| {
        Error::xml_detail(
            crate::error::XmlIssue::Parse,
            format!("{} has no paragraph", item.part),
        )
    })?;
    picture::insert_drawing_into_para(p, drawing);
    dirty.insert(item.part.clone(), ());
    Ok(())
}

fn clear_header(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    item: &HfPart,
) -> Result<(), Error> {
    let rids = {
        let doc = load_cached(pkg, docs, &item.part)?;
        let rids = collect_blip_rids(&doc.root);
        let changed = remove_all_watermarks(&mut doc.root);
        if changed {
            dirty.insert(item.part.clone(), ());
        }
        rids
    };
    for rid in rids {
        let still = {
            let doc = docs.get(&item.part).unwrap();
            blip_still_used(&doc.root, &rid)
        };
        if !still {
            let media = picture::resolve_embed(pkg, &item.part, &rid).ok();
            picture::remove_rel_if_unused(
                pkg,
                &item.part,
                &rid,
                &docs.get(&item.part).unwrap().root,
            )?;
            if let Some(m) = media {
                if !picture::media_still_used(pkg, &m)? {
                    pkg.remove(&m);
                }
            }
        }
    }
    Ok(())
}

fn delete_one(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    item: &HfPart,
    key: &WmKey,
    id: &str,
    i: usize,
) -> Result<(), Error> {
    let rids = {
        let doc = load_cached(pkg, docs, &item.part)?;
        let rids = find_wm_embeds(&doc.root, key);
        if !remove_one_watermark(&mut doc.root, key) {
            return Err(Error::with_path(
                ErrorKind::WatermarkNotFound { id: id.into() },
                format!("ops[{i}].id"),
            ));
        }
        dirty.insert(item.part.clone(), ());
        rids
    };
    for r in rids {
        let media = picture::resolve_embed(pkg, &item.part, &r).ok();
        picture::remove_rel_if_unused(pkg, &item.part, &r, &docs.get(&item.part).unwrap().root)?;
        if let Some(m) = media {
            if !picture::media_still_used(pkg, &m)? {
                pkg.remove(&m);
            }
        }
    }
    Ok(())
}

fn collect_blip_rids(root: &Element) -> Vec<String> {
    let mut out = Vec::new();
    let _ = picture::walk_drawings(root, &mut |d| {
        if is_watermark_drawing(d) {
            out.extend(picture::embed_rids(d));
        }
        Ok(())
    });
    out
}

fn blip_still_used(root: &Element, rid: &str) -> bool {
    let mut used = false;
    let _ = picture::walk_drawings(root, &mut |d| {
        if picture::embed_rids(d).iter().any(|x| x == rid) {
            used = true;
        }
        Ok(())
    });
    used
}

fn find_wm_embeds(root: &Element, key: &WmKey) -> Vec<String> {
    let mut index = 0usize;
    let mut rids = Vec::new();
    let _ = picture::walk_drawings(root, &mut |d| {
        if !is_watermark_drawing(d) {
            return Ok(());
        }
        index += 1;
        if matches_key(d, key, index) {
            rids = picture::embed_rids(d);
        }
        Ok(())
    });
    rids
}

fn remove_all_watermarks(root: &mut Element) -> bool {
    let mut changed = false;
    loop {
        let mut index = 0usize;
        if !remove_watermark_walk(root, &mut index, None) {
            break;
        }
        changed = true;
    }
    changed
}

fn remove_one_watermark(root: &mut Element, key: &WmKey) -> bool {
    let mut index = 0usize;
    remove_watermark_walk(root, &mut index, Some(key))
}

fn remove_watermark_walk(el: &mut Element, index: &mut usize, key: Option<&WmKey>) -> bool {
    if el.local_name() == "txbxContent" || el.local_name() == "object" {
        return false;
    }
    let mut i = 0;
    while i < el.children.len() {
        let local = match &el.children[i] {
            Node::Element(c) => c.local_name().to_string(),
            Node::Text(_) | Node::Raw(_) => {
                i += 1;
                continue;
            }
        };
        if local == "AlternateContent" {
            if ac_has_wm(&el.children[i], index, key) {
                el.children.remove(i);
                prune_empty_run(el);
                return true;
            }
            i += 1;
            continue;
        }
        if local == "drawing" {
            let is_wm = matches!(&el.children[i], Node::Element(c) if is_watermark_drawing(c));
            if is_wm {
                *index += 1;
                let hit = match (&el.children[i], key) {
                    (Node::Element(c), Some(k)) => matches_key(c, k, *index),
                    (Node::Element(_), None) => true,
                    _ => false,
                };
                if hit {
                    el.children.remove(i);
                    prune_empty_run(el);
                    return true;
                }
            }
            i += 1;
            continue;
        }
        if let Node::Element(child) = &mut el.children[i] {
            if remove_watermark_walk(child, index, key) {
                return true;
            }
        }
        i += 1;
    }
    false
}

fn ac_has_wm(node: &Node, index: &mut usize, key: Option<&WmKey>) -> bool {
    let Node::Element(ac) = node else {
        return false;
    };
    let mut found = false;
    let _ = picture::walk_drawings(ac, &mut |d| {
        if !is_watermark_drawing(d) {
            return Ok(());
        }
        *index += 1;
        found = match key {
            Some(k) => matches_key(d, k, *index),
            None => true,
        };
        Ok(())
    });
    found
}

fn prune_empty_run(parent: &mut Element) {
    if parent.local_name() != "r" {
        return;
    }
    let empty = parent.element_children().next().is_none();
    if empty {
        parent.children.clear();
    }
}

fn first_para_mut(root: &mut Element) -> Option<&mut Element> {
    if root.element_children().any(|e| e.local_name() == "p") {
        return root.element_children_mut().find(|e| e.local_name() == "p");
    }
    let mut p = Element::new(root.prefixed("p"));
    p.set_attr("w14:paraId", "00000001");
    p.set_attr("w14:textId", "00000001");
    root.children.push(Node::Element(p));
    root.element_children_mut().find(|e| e.local_name() == "p")
}

fn ensure_wm_ns(root: &mut Element) {
    textbox::ensure_textbox_ns(root);
    picture::ensure_drawing_ns(root);
    ensure_xmlns(root, "r", REL_NS);
    ensure_xmlns(root, "w14", W14_NS);
    ensure_ignorable(root, "w14");
}

fn ensure_xmlns(el: &mut Element, prefix: &str, uri: &str) {
    let key = format!("xmlns:{prefix}");
    if el.attr(&key).is_none() {
        el.set_attr(&key, uri);
    }
}

fn ensure_ignorable(el: &mut Element, token: &str) {
    let key = "mc:Ignorable";
    if let Some((_, v)) = el.attrs.iter_mut().find(|(k, _)| k == key) {
        if !v.split_whitespace().any(|t| t == token) {
            if !v.is_empty() {
                v.push(' ');
            }
            v.push_str(token);
        }
        return;
    }
    el.set_attr(key, token);
}

fn stamp_doc_pr(drawing: &mut Element, name: &str) {
    let Some(frame) = drawing
        .element_children_mut()
        .find(|e| e.local_name() == "inline" || e.local_name() == "anchor")
    else {
        return;
    };
    if let Some(pr) = frame.child_by_local_mut("docPr") {
        if let Some((_, v)) = pr
            .attrs
            .iter_mut()
            .find(|(k, _)| k.rsplit(':').next().unwrap_or(k) == "name")
        {
            *v = name.to_string();
        } else {
            pr.set_attr("name", name);
        }
        if pr.child_by_local("extLst").is_none() {
            let mut lst = Element::new("a:extLst");
            lst.push_element(Element::new("a:ext").with_attr("uri", EXT_URI));
            pr.push_element(lst);
        }
    }
}

fn set_page_center(drawing: &mut Element) {
    let Some(frame) = drawing
        .element_children_mut()
        .find(|e| e.local_name() == "anchor")
    else {
        return;
    };
    for child in frame.element_children_mut() {
        if child.local_name() == "positionH" || child.local_name() == "positionV" {
            if let Some((_, v)) = child
                .attrs
                .iter_mut()
                .find(|(k, _)| k.rsplit(':').next().unwrap_or(k) == "relativeFrom")
            {
                *v = "page".into();
            }
        }
    }
}

fn build_text_drawing(doc_pr: u32, text: &str, para_id: &str) -> Element {
    let mut drawing = Element::new("w:drawing");
    let mut frame = Element::new("wp:anchor")
        .with_attr("distT", "0")
        .with_attr("distB", "0")
        .with_attr("distL", "0")
        .with_attr("distR", "0")
        .with_attr("simplePos", "0")
        .with_attr("relativeHeight", "251658240")
        .with_attr("behindDoc", "1")
        .with_attr("locked", "0")
        .with_attr("layoutInCell", "1")
        .with_attr("allowOverlap", "1");
    frame.push_element(
        Element::new("wp:simplePos")
            .with_attr("x", "0")
            .with_attr("y", "0"),
    );
    let mut pos_h = Element::new("wp:positionH").with_attr("relativeFrom", "page");
    pos_h.push_element(Element::new("wp:align").with_text("center"));
    let mut pos_v = Element::new("wp:positionV").with_attr("relativeFrom", "page");
    pos_v.push_element(Element::new("wp:align").with_text("center"));
    frame.push_element(pos_h);
    frame.push_element(pos_v);
    frame.push_element(
        Element::new("wp:extent")
            .with_attr("cx", TEXT_CX.to_string())
            .with_attr("cy", TEXT_CY.to_string()),
    );
    frame.push_element(
        Element::new("wp:effectExtent")
            .with_attr("l", "0")
            .with_attr("t", "0")
            .with_attr("r", "0")
            .with_attr("b", "0"),
    );
    frame.push_element(Element::new("wp:wrapNone"));
    let mut pr = Element::new("wp:docPr")
        .with_attr("id", doc_pr.to_string())
        .with_attr("name", NAME_TEXT);
    let mut lst = Element::new("a:extLst");
    lst.push_element(Element::new("a:ext").with_attr("uri", EXT_URI));
    pr.push_element(lst);
    frame.push_element(pr);
    let mut frame_pr = Element::new("wp:cNvGraphicFramePr");
    frame_pr.push_element(Element::new("a:graphicFrameLocks").with_attr("noChangeAspect", "1"));
    frame.push_element(frame_pr);
    let mut graphic = Element::new("a:graphic");
    let mut data = Element::new("a:graphicData").with_attr("uri", WPS_NS);
    data.push_element(build_text_wsp(text, para_id));
    graphic.push_element(data);
    frame.push_element(graphic);
    drawing.push_element(frame);
    drawing
}

fn build_text_wsp(text: &str, para_id: &str) -> Element {
    let mut wsp = Element::new("wps:wsp");
    wsp.push_element(Element::new("wps:cNvSpPr").with_attr("txBox", "1"));
    let mut sp = Element::new("wps:spPr");
    let mut xfrm = Element::new("a:xfrm").with_attr("rot", ROT_315);
    xfrm.push_element(
        Element::new("a:off")
            .with_attr("x", "0")
            .with_attr("y", "0"),
    );
    xfrm.push_element(
        Element::new("a:ext")
            .with_attr("cx", TEXT_CX.to_string())
            .with_attr("cy", TEXT_CY.to_string()),
    );
    sp.push_element(xfrm);
    let mut geom = Element::new("a:prstGeom").with_attr("prst", "rect");
    geom.push_element(Element::new("a:avLst"));
    sp.push_element(geom);
    sp.push_element(Element::new("a:noFill"));
    let mut ln = Element::new("a:ln");
    ln.push_element(Element::new("a:noFill"));
    sp.push_element(ln);
    wsp.push_element(sp);
    let mut txbx = Element::new("wps:txbx");
    let mut content = Element::new("w:txbxContent");
    let mut inner = Element::new("w:p")
        .with_attr("w14:paraId", para_id)
        .with_attr("w14:textId", para_id);
    let mut rpr = Element::new("w:rPr");
    rpr.push_element(Element::new("w:sz").with_attr("w:val", "96"));
    rpr.push_element(Element::new("w:szCs").with_attr("w:val", "96"));
    rpr.push_element(Element::new("w:color").with_attr("w:val", "C0C0C0"));
    let mut r = Element::new("w:r");
    r.push_element(rpr);
    let mut t = Element::new("w:t");
    t.set_text(text);
    if text.starts_with(' ') || text.ends_with(' ') {
        t.set_attr("xml:space", "preserve");
    }
    r.push_element(t);
    inner.push_element(r);
    content.push_element(inner);
    txbx.push_element(content);
    wsp.push_element(txbx);
    let mut body_pr = Element::new("wps:bodyPr")
        .with_attr("wrap", "none")
        .with_attr("lIns", "0")
        .with_attr("tIns", "0")
        .with_attr("rIns", "0")
        .with_attr("bIns", "0");
    body_pr.push_element(Element::new("a:noAutofit"));
    wsp.push_element(body_pr);
    wsp
}

fn ensure_section1_default_header(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
) -> Result<(), Error> {
    let has = {
        let doc = load_cached(pkg, docs, doc_part)?;
        let sects = hf::collect_sect_prs(&doc.root);
        if sects.is_empty() {
            false
        } else {
            hf::has_explicit_ref(sects[0], HfKind::Header, HfType::Default)
        }
    };
    if has {
        return Ok(());
    }
    let new_part = hf::next_unused_hf_part(pkg, HfKind::Header);
    pkg.set(&new_part, hf::minimal_hf_xml(HfKind::Header))?;
    let mut rels = Relationships::from_package(pkg, doc_part)?;
    let rid = rels.next_rid();
    rels.push(Relationship {
        id: rid.clone(),
        rel_type: REL_HEADER.into(),
        target: hf::same_dir_rel_target(doc_part, &new_part),
        target_mode: None,
        extra: Vec::new(),
    });
    rels.write_to(pkg, doc_part)?;
    let mut cts = ContentTypes::from_package(pkg)?;
    cts.set_override(&new_part, CT_HEADER)?;
    cts.write_to(pkg)?;
    let doc = load_cached(pkg, docs, doc_part)?;
    let sect = hf::find_sect_pr_mut(&mut doc.root, 1).ok_or_else(|| {
        Error::with_path(
            ErrorKind::SectionNotFound {
                id: "section:1".into(),
            },
            "ops[0].id",
        )
    })?;
    let rid_attr = hf::rid_attr_name(sect);
    hf::insert_hf_ref(sect, HfKind::Header, HfType::Default, &rid_attr, &rid);
    dirty.insert(doc_part.to_string(), ());
    Ok(())
}

fn load_cached<'a>(
    pkg: &Package,
    docs: &'a mut HashMap<String, Document>,
    name: &str,
) -> Result<&'a mut Document, Error> {
    if !docs.contains_key(name) {
        let bytes = pkg.get(name)?;
        docs.insert(name.to_string(), xml::parse(bytes)?);
    }
    Ok(docs.get_mut(name).unwrap())
}
