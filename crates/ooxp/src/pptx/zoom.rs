//! 幻灯片 Zoom：MS-PPTX 的 `sldZm` / `sectionZm` / `summaryZm`。
//! 不实现放映跳转，不把预览图算进 pictures。

use serde_json::{json, Map, Value};

use crate::error::{Error, ErrorKind, IdReason};
use crate::ole;
use crate::opc::{resolve_id, ContentTypes, Package, Relationship, Relationships};
use crate::xml::{Document, Element, Node};

use super::picture;
use super::section;
use super::shape::{self, first_alt_child_index, SlideMeta};
use super::{DML_NS, REL_NS, REL_SLIDE};

pub const SLIDE_ZOOM_URI: &str = "http://schemas.microsoft.com/office/powerpoint/2016/slidezoom";
pub const SECTION_ZOOM_URI: &str =
    "http://schemas.microsoft.com/office/powerpoint/2016/sectionzoom";
pub const SUMMARY_ZOOM_URI: &str =
    "http://schemas.microsoft.com/office/powerpoint/2016/summaryzoom";
const P166_NS: &str = "http://schemas.microsoft.com/office/powerpoint/2016/6/main";
const MC_NS: &str = "http://schemas.openxmlformats.org/markup-compatibility/2006";

const DEFAULT_CX: i64 = 3_657_600;
const DEFAULT_CY: i64 = 2_743_200;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZoomType {
    Slide,
    Section,
    Summary,
}

impl ZoomType {
    fn as_str(self) -> &'static str {
        match self {
            Self::Slide => "slide",
            Self::Section => "section",
            Self::Summary => "summary",
        }
    }
}

#[derive(Debug, Clone)]
pub enum ZoomKey {
    Id(String),
    Index(usize),
}

#[derive(Debug, Clone)]
pub struct ZoomTarget {
    pub slide: usize,
    pub key: ZoomKey,
}

#[derive(Debug, Clone)]
pub enum InsertKind {
    Slide { target: usize },
    Section { target: usize },
}

#[derive(Debug, Clone)]
pub struct InsertZoomOp {
    pub slide: usize,
    pub kind: InsertKind,
    pub x: Option<i64>,
    pub y: Option<i64>,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct DeleteZoomOp {
    pub id: String,
    pub target: ZoomTarget,
}

pub fn parse_insert_slide_id(id: &str, op_index: usize) -> Result<usize, Error> {
    let path = format!("ops[{op_index}].id");
    parse_slide_ref(
        id,
        &path,
        IdReason::InsertZoomNeedsSlide,
        IdReason::InsertZoomNeedsSlide,
    )
}

pub fn parse_slide_target(id: &str, json_path: &str) -> Result<usize, Error> {
    parse_slide_ref(
        id,
        json_path,
        IdReason::ZoomTargetSlide,
        IdReason::ZoomTargetSlide,
    )
}

fn parse_slide_ref(
    id: &str,
    path: &str,
    prefix_reason: IdReason,
    empty_reason: IdReason,
) -> Result<usize, Error> {
    let rest = id.strip_prefix("slide:").ok_or_else(|| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: prefix_reason,
            },
            path.to_string(),
        )
    })?;
    if rest.is_empty() || rest.contains('/') {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: empty_reason,
            },
            path.to_string(),
        ));
    }
    parse_slide_n(rest, path)
}

pub fn parse_zoom_id(id: &str, json_path: &str) -> Result<ZoomTarget, Error> {
    let rest = id.strip_prefix("slide:").ok_or_else(|| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SlideZoomPrefix,
            },
            json_path.to_string(),
        )
    })?;
    let Some((n, after)) = rest.split_once("/zoom:") else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SlideZoomContainer,
            },
            json_path.to_string(),
        ));
    };
    if after.is_empty() {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SlideZoomPrefix,
            },
            json_path.to_string(),
        ));
    }
    let slide = parse_slide_n(n, json_path)?;
    let key = if let Some(rest) = after.strip_prefix("index:") {
        let k: usize = rest.parse().map_err(|_| {
            Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::ZoomIndexNotInt,
                },
                json_path.to_string(),
            )
        })?;
        if k < 1 {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::ZoomIndexFromOne,
                },
                json_path.to_string(),
            ));
        }
        ZoomKey::Index(k)
    } else {
        ZoomKey::Id(after.to_string())
    };
    Ok(ZoomTarget { slide, key })
}

pub fn count(root: &Element) -> usize {
    collect_nodes(root).len()
}

pub fn collect(
    root: &Element,
    host: &str,
    slides: &[SlideMeta],
    pres_root: &Element,
) -> Vec<Value> {
    let nodes = collect_nodes(root);
    let mut out = Vec::new();
    for (i, node) in nodes.iter().enumerate() {
        out.push(node_json(node, host, i + 1, slides, pres_root));
    }
    out
}

pub fn insert(
    pkg: &mut Package,
    slide_doc: &mut Document,
    slide_part: &str,
    pres_root: &Element,
    slides: &[SlideMeta],
    op: &InsertZoomOp,
    i: usize,
) -> Result<(), Error> {
    let (target_part, sld_id, section_guid, prefix, uri, obj_name, kind_local) = match &op.kind {
        InsertKind::Slide { target } => {
            if *target == op.slide {
                return Err(Error::with_path(
                    ErrorKind::ZoomTargetSelf,
                    format!("ops[{i}].target"),
                ));
            }
            let meta = slides.get(target - 1).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::SlideNotFound { n: *target as u32 },
                    format!("ops[{i}].target"),
                )
            })?;
            let sld_id = meta.sld_id.ok_or_else(|| {
                Error::with_path(
                    ErrorKind::SlideNotFound { n: *target as u32 },
                    format!("ops[{i}].target"),
                )
            })?;
            (
                meta.part.clone(),
                Some(sld_id),
                None,
                "pslz",
                SLIDE_ZOOM_URI,
                "sldZmObj",
                "sldZm",
            )
        }
        InsertKind::Section { target } => {
            let secs = section::collect(pres_root);
            let info = secs
                .into_iter()
                .find(|s| s.index == *target)
                .ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::SectionNotFound {
                            id: format!("section:{target}"),
                        },
                        format!("ops[{i}].target"),
                    )
                })?;
            let guid = info.guid.clone().filter(|s| !s.is_empty()).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::SectionNotFound {
                        id: format!("section:{target}"),
                    },
                    format!("ops[{i}].target"),
                )
            })?;
            let first_sid = *info.sids.first().ok_or_else(|| {
                Error::with_path(
                    ErrorKind::SectionNotFound {
                        id: format!("section:{target}"),
                    },
                    format!("ops[{i}].target"),
                )
            })?;
            let meta = slides
                .iter()
                .find(|s| s.sld_id == Some(first_sid))
                .ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::SectionNotFound {
                            id: format!("section:{target}"),
                        },
                        format!("ops[{i}].target"),
                    )
                })?;
            (
                meta.part.clone(),
                None,
                Some(guid),
                "psez",
                SECTION_ZOOM_URI,
                "sectionZmObj",
                "sectionZm",
            )
        }
    };

    let png_part = picture::next_media_part(pkg, "png");
    pkg.set(&png_part, ole::PREVIEW_PNG.to_vec())?;

    let mut rels = Relationships::from_package(pkg, slide_part)?;
    let img_rid = rels.next_rid();
    rels.push(Relationship {
        id: img_rid.clone(),
        rel_type: picture::REL_IMAGE.into(),
        target: picture::rel_target_from(slide_part, &png_part),
        target_mode: None,
        extra: Vec::new(),
    });
    let slide_rid = ensure_slide_rel(&mut rels, slide_part, target_part.as_str());
    rels.write_to(pkg, slide_part)?;

    let mut cts = ContentTypes::from_package(pkg)?;
    cts.ensure_default("png", "image/png");
    cts.write_to(pkg)?;

    let cx = op.cx.unwrap_or(DEFAULT_CX);
    let cy = op.cy.unwrap_or(DEFAULT_CY);
    let (x, y) = match (op.x, op.y) {
        (Some(x), Some(y)) => (x, y),
        _ => picture::vacant_pos(pres_root, &slide_doc.root, cx, cy),
    };

    picture::ensure_xmlns(&mut slide_doc.root, "r", REL_NS);
    picture::ensure_xmlns(&mut slide_doc.root, "a", DML_NS);
    picture::ensure_xmlns(&mut slide_doc.root, "mc", MC_NS);
    picture::ensure_xmlns(&mut slide_doc.root, prefix, uri);
    picture::ensure_xmlns(&mut slide_doc.root, "p166", P166_NS);
    ensure_ignorable(&mut slide_doc.root, &[prefix, "p166"]);

    let cnv = picture::next_cnv_id(&slide_doc.root);
    let zoom_n = count(&slide_doc.root) + 1;
    let ac = build_zoom(
        cnv,
        zoom_n,
        prefix,
        uri,
        kind_local,
        obj_name,
        sld_id,
        section_guid.as_deref(),
        &img_rid,
        &slide_rid,
        x,
        y,
        cx,
        cy,
        matches!(op.kind, InsertKind::Slide { .. }),
    );
    picture::insert_into_sp_tree(&mut slide_doc.root, ac)?;
    Ok(())
}

pub fn delete(
    pkg: &mut Package,
    slide_doc: &mut Document,
    slide_part: &str,
    op: &DeleteZoomOp,
    op_index: usize,
) -> Result<(), Error> {
    let path = format!("ops[{op_index}].id");
    let node = find_node(&slide_doc.root, &op.target.key)?.ok_or_else(|| {
        Error::with_path(ErrorKind::ZoomNotFound { id: op.id.clone() }, path.clone())
    })?;
    let spid = cnv_of(node).0.filter(|s| !s.is_empty());
    let mut rids = Vec::new();
    collect_rids(node, &mut rids);
    let mut old_parts = Vec::new();
    for rid in &rids {
        if let Ok(p) = resolve_id(pkg, slide_part, rid) {
            if !old_parts.iter().any(|x| x == &p) {
                old_parts.push(p);
            }
        }
    }
    if !remove_node(&mut slide_doc.root, &op.target.key) {
        return Err(Error::with_path(
            ErrorKind::ZoomNotFound { id: op.id.clone() },
            path,
        ));
    }
    if let Some(id) = spid {
        shape::strip_timing_for_spid(&mut slide_doc.root, &id);
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
        if part.contains("/media/") && !picture::media_still_used(pkg, &part)? {
            pkg.remove(&part);
            let mut cts = ContentTypes::from_package(pkg)?;
            cts.remove_override(&part);
            cts.write_to(pkg)?;
        }
    }
    Ok(())
}

fn collect_nodes(root: &Element) -> Vec<&Element> {
    let mut out = Vec::new();
    collect_in(root, &mut out);
    out
}

fn collect_in<'a>(el: &'a Element, out: &mut Vec<&'a Element>) {
    if el.local_name() == "AlternateContent" {
        if zoom_in(el).is_some() {
            out.push(el);
            return;
        }
        if let Some(i) = first_alt_child_index(el) {
            if let Node::Element(child) = &el.children[i] {
                collect_in(child, out);
            }
        }
        return;
    }
    if is_zoom_frame(el) {
        out.push(el);
        return;
    }
    for n in &el.children {
        if let Node::Element(e) = n {
            collect_in(e, out);
        }
    }
}

fn zoom_in(el: &Element) -> Option<ZoomType> {
    if let Some(t) = zoom_type_of(el) {
        return Some(t);
    }
    if el.local_name() == "AlternateContent" {
        if let Some(i) = first_alt_child_index(el) {
            if let Node::Element(child) = &el.children[i] {
                return zoom_in(child);
            }
        }
    }
    for n in &el.children {
        if let Node::Element(e) = n {
            if let Some(t) = zoom_in(e) {
                return Some(t);
            }
        }
    }
    None
}

fn zoom_type_of(el: &Element) -> Option<ZoomType> {
    match el.local_name() {
        "sldZm" => Some(ZoomType::Slide),
        "sectionZm" => Some(ZoomType::Section),
        "summaryZm" => Some(ZoomType::Summary),
        "graphicFrame" => graphic_uri(el).and_then(|u| {
            if uri_is(&u, SLIDE_ZOOM_URI) {
                Some(ZoomType::Slide)
            } else if uri_is(&u, SECTION_ZOOM_URI) {
                Some(ZoomType::Section)
            } else if uri_is(&u, SUMMARY_ZOOM_URI) {
                Some(ZoomType::Summary)
            } else {
                None
            }
        }),
        _ => None,
    }
}

fn is_zoom_frame(el: &Element) -> bool {
    el.local_name() == "graphicFrame" && zoom_type_of(el).is_some()
}

fn graphic_uri(frame: &Element) -> Option<String> {
    frame
        .child_by_local("graphic")
        .and_then(|g| g.child_by_local("graphicData"))
        .and_then(|d| d.attr("uri"))
        .map(str::to_string)
}

fn uri_is(uri: &str, want: &str) -> bool {
    let a = uri.trim_end_matches('/');
    let b = want.trim_end_matches('/');
    a.eq_ignore_ascii_case(b)
}

fn node_json(
    node: &Element,
    host: &str,
    index: usize,
    slides: &[SlideMeta],
    pres_root: &Element,
) -> Value {
    let (raw, _) = cnv_of(node);
    let (id, stable) = match &raw {
        Some(s) => (format!("{host}/zoom:{s}"), true),
        None => (format!("{host}/zoom:index:{index}"), false),
    };
    let kind = zoom_in(node).unwrap_or(ZoomType::Slide);
    let (x, y, cx, cy) = xfrm_of(node);
    let mut m = Map::new();
    m.insert("id".into(), json!(id));
    m.insert("type".into(), json!(kind.as_str()));
    match kind {
        ZoomType::Slide => {
            m.insert("target".into(), json!(slide_target_of(node, slides)));
        }
        ZoomType::Section => {
            m.insert("target".into(), json!(section_target_of(node, pres_root)));
        }
        ZoomType::Summary => {
            m.insert("targets".into(), json!(summary_targets_of(node, pres_root)));
        }
    }
    m.insert("x".into(), json!(x));
    m.insert("y".into(), json!(y));
    m.insert("cx".into(), json!(cx));
    m.insert("cy".into(), json!(cy));
    m.insert("stable".into(), json!(stable));
    Value::Object(m)
}

fn slide_target_of(node: &Element, slides: &[SlideMeta]) -> Option<String> {
    let obj = find_local(node, "sldZmObj")?;
    let sid = obj.attr("sldId")?.parse::<u32>().ok()?;
    slides
        .iter()
        .find(|s| s.sld_id == Some(sid))
        .map(|s| format!("slide:{}", s.index))
}

fn section_target_of(node: &Element, pres_root: &Element) -> Option<String> {
    let obj = find_local(node, "sectionZmObj")?;
    let guid = obj.attr("sectionId")?;
    section_id_of(pres_root, guid)
}

fn summary_targets_of(node: &Element, pres_root: &Element) -> Vec<String> {
    let mut out = Vec::new();
    collect_summary_guids(node, &mut out);
    out.into_iter()
        .filter_map(|g| section_id_of(pres_root, &g))
        .collect()
}

fn collect_summary_guids(el: &Element, out: &mut Vec<String>) {
    if el.local_name() == "summaryZmObj" {
        if let Some(g) = el.attr("sectionId") {
            out.push(g.to_string());
        }
    }
    for n in &el.children {
        if let Node::Element(e) = n {
            collect_summary_guids(e, out);
        }
    }
}

fn section_id_of(pres_root: &Element, guid: &str) -> Option<String> {
    let want = norm_guid(guid);
    section::collect(pres_root)
        .into_iter()
        .find(|s| s.guid.as_deref().is_some_and(|g| norm_guid(g) == want))
        .map(|s| format!("section:{}", s.index))
}

fn norm_guid(s: &str) -> String {
    s.trim()
        .trim_start_matches('{')
        .trim_end_matches('}')
        .to_ascii_uppercase()
}

fn xfrm_of(node: &Element) -> (Option<i64>, Option<i64>, Option<i64>, Option<i64>) {
    if let Some(frame) = find_local(node, "graphicFrame") {
        return xfrm_from(frame.child_by_local("xfrm"));
    }
    if let Some(sp) = find_local(node, "spPr") {
        return xfrm_from(sp.child_by_local("xfrm"));
    }
    if let Some(pic) = find_local(node, "pic") {
        if let Some(sp) = pic.child_by_local("spPr") {
            return xfrm_from(sp.child_by_local("xfrm"));
        }
    }
    (None, None, None, None)
}

fn xfrm_from(xfrm: Option<&Element>) -> (Option<i64>, Option<i64>, Option<i64>, Option<i64>) {
    let Some(xfrm) = xfrm else {
        return (None, None, None, None);
    };
    let off = xfrm.child_by_local("off");
    let ext = xfrm.child_by_local("ext");
    (
        off.and_then(|o| parse_i64_attr(o, "x")),
        off.and_then(|o| parse_i64_attr(o, "y")),
        ext.and_then(|e| parse_i64_attr(e, "cx")),
        ext.and_then(|e| parse_i64_attr(e, "cy")),
    )
}

fn cnv_of(node: &Element) -> (Option<String>, String) {
    if let Some(frame) = find_local(node, "graphicFrame") {
        if let Some(nv) = frame.child_by_local("nvGraphicFramePr") {
            if let Some(pr) = nv.child_by_local("cNvPr") {
                return (
                    pr.attr("id").filter(|s| !s.is_empty()).map(str::to_string),
                    pr.attr("name").unwrap_or("").to_string(),
                );
            }
        }
    }
    if let Some(pic) = find_fallback_pic(node) {
        if let Some(nv) = pic.child_by_local("nvPicPr") {
            if let Some(pr) = nv.child_by_local("cNvPr") {
                return (
                    pr.attr("id").filter(|s| !s.is_empty()).map(str::to_string),
                    pr.attr("name").unwrap_or("").to_string(),
                );
            }
        }
    }
    (None, String::new())
}

fn find_fallback_pic(el: &Element) -> Option<&Element> {
    if el.local_name() == "AlternateContent" {
        for n in &el.children {
            if let Node::Element(e) = n {
                if e.local_name() == "Fallback" {
                    return find_local(e, "pic");
                }
            }
        }
    }
    None
}

fn find_local<'a>(el: &'a Element, name: &str) -> Option<&'a Element> {
    if el.local_name() == name {
        return Some(el);
    }
    for n in &el.children {
        if let Node::Element(e) = n {
            if let Some(found) = find_local(e, name) {
                return Some(found);
            }
        }
    }
    None
}

fn find_node<'a>(root: &'a Element, key: &ZoomKey) -> Result<Option<&'a Element>, Error> {
    let nodes = collect_nodes(root);
    for (i, n) in nodes.iter().enumerate() {
        if node_matches(n, i + 1, key) {
            return Ok(Some(*n));
        }
    }
    Ok(None)
}

fn node_matches(node: &Element, index: usize, key: &ZoomKey) -> bool {
    match key {
        ZoomKey::Index(n) => index == *n,
        ZoomKey::Id(req) => cnv_of(node).0.is_some_and(|id| ids_match(&id, req)),
    }
}

fn remove_node(root: &mut Element, key: &ZoomKey) -> bool {
    let mut idx = 0usize;
    remove_in(root, &mut idx, key)
}

fn remove_in(el: &mut Element, idx: &mut usize, key: &ZoomKey) -> bool {
    if el.local_name() == "AlternateContent" {
        if zoom_in(el).is_some() {
            *idx += 1;
            return false;
        }
        if let Some(i) = first_alt_child_index(el) {
            if let Node::Element(child) = &mut el.children[i] {
                return remove_in(child, idx, key);
            }
        }
        return false;
    }
    let mut i = 0;
    while i < el.children.len() {
        let is_hit = match &el.children[i] {
            Node::Element(e) if is_zoom_unit(e) => {
                *idx += 1;
                node_matches(e, *idx, key)
            }
            _ => false,
        };
        if is_hit {
            el.children.remove(i);
            return true;
        }
        if let Node::Element(e) = &mut el.children[i] {
            if !is_zoom_unit(e) && remove_in(e, idx, key) {
                return true;
            }
        }
        i += 1;
    }
    false
}

fn is_zoom_unit(el: &Element) -> bool {
    (el.local_name() == "AlternateContent" && zoom_in(el).is_some()) || is_zoom_frame(el)
}

fn collect_rids(el: &Element, out: &mut Vec<String>) {
    for (k, v) in &el.attrs {
        if k.rsplit(':').next() == Some("id")
            && k.contains(':')
            && !v.is_empty()
            && !out.iter().any(|x| x == v)
        {
            out.push(v.clone());
        }
        if k == "r:embed" && !v.is_empty() && !out.iter().any(|x| x == v) {
            out.push(v.clone());
        }
    }
    for n in &el.children {
        if let Node::Element(e) = n {
            collect_rids(e, out);
        }
    }
}

fn rid_still_on_slide(el: &Element, rid: &str) -> bool {
    for (k, v) in &el.attrs {
        if v == rid && (k.rsplit(':').next() == Some("id") && k.contains(':') || k == "r:embed") {
            return true;
        }
    }
    el.children.iter().any(|n| match n {
        Node::Element(e) => rid_still_on_slide(e, rid),
        _ => false,
    })
}

fn ensure_slide_rel(rels: &mut Relationships, slide_part: &str, target_part: &str) -> String {
    let want = picture::rel_target_from(slide_part, target_part);
    if let Some(r) = rels
        .rels
        .iter()
        .find(|r| r.rel_type.ends_with("/slide") && !r.is_external() && r.target == want)
    {
        return r.id.clone();
    }
    let rid = rels.next_rid();
    rels.push(Relationship {
        id: rid.clone(),
        rel_type: REL_SLIDE.into(),
        target: want,
        target_mode: None,
        extra: Vec::new(),
    });
    rid
}

fn ensure_ignorable(root: &mut Element, tokens: &[&str]) {
    let cur = root.attr("mc:Ignorable").unwrap_or("").to_string();
    let mut parts: Vec<String> = cur
        .split_whitespace()
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect();
    for t in tokens {
        if !parts.iter().any(|p| p == t) {
            parts.push((*t).to_string());
        }
    }
    root.set_attr("mc:Ignorable", parts.join(" "));
}

fn build_zoom(
    cnv: u32,
    zoom_n: usize,
    prefix: &str,
    uri: &str,
    kind_local: &str,
    obj_name: &str,
    sld_id: Option<u32>,
    section_guid: Option<&str>,
    img_rid: &str,
    slide_rid: &str,
    x: i64,
    y: i64,
    cx: i64,
    cy: i64,
    return_to_parent: bool,
) -> Element {
    let guid = section::new_guid();
    let mut zm_pr = Element::new(format!("{prefix}:zmPr")).with_attr("id", guid);
    if return_to_parent {
        zm_pr = zm_pr.with_attr("returnToParent", "1");
    }
    let mut fill = Element::new("p166:blipFill").with_attr("xmlns:p166", P166_NS);
    fill.push_element(Element::new("a:blip").with_attr("r:embed", img_rid));
    let mut stretch = Element::new("a:stretch");
    stretch.push_element(Element::new("a:fillRect"));
    fill.push_element(stretch);
    zm_pr.push_element(fill);
    let mut sp_pr = Element::new("p166:spPr").with_attr("xmlns:p166", P166_NS);
    let mut xfrm = Element::new("a:xfrm");
    xfrm.push_element(
        Element::new("a:off")
            .with_attr("x", "0")
            .with_attr("y", "0"),
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
    zm_pr.push_element(sp_pr);

    let mut obj = Element::new(format!("{prefix}:{obj_name}"));
    if let Some(sid) = sld_id {
        obj = obj.with_attr("sldId", sid.to_string());
    }
    if let Some(g) = section_guid {
        obj = obj.with_attr("sectionId", g);
    }
    obj.push_element(zm_pr);
    let mut zm = Element::new(format!("{prefix}:{kind_local}"));
    zm.push_element(obj);

    let mut nv = Element::new("p:nvGraphicFramePr");
    nv.push_element(
        Element::new("p:cNvPr")
            .with_attr("id", cnv.to_string())
            .with_attr("name", format!("Zoom {zoom_n}")),
    );
    let mut locks = Element::new("p:cNvGraphicFramePr");
    locks.push_element(Element::new("a:graphicFrameLocks").with_attr("noChangeAspect", "1"));
    nv.push_element(locks);
    nv.push_element(Element::new("p:nvPr"));
    let mut frame_xfrm = Element::new("p:xfrm");
    frame_xfrm.push_element(
        Element::new("a:off")
            .with_attr("x", x.to_string())
            .with_attr("y", y.to_string()),
    );
    frame_xfrm.push_element(
        Element::new("a:ext")
            .with_attr("cx", cx.to_string())
            .with_attr("cy", cy.to_string()),
    );
    let mut data = Element::new("a:graphicData").with_attr("uri", uri);
    data.push_element(zm);
    let mut graphic = Element::new("a:graphic");
    graphic.push_element(data);
    let mut frame = Element::new("p:graphicFrame");
    frame.push_element(nv);
    frame.push_element(frame_xfrm);
    frame.push_element(graphic);

    let mut choice = Element::new("mc:Choice")
        .with_attr(format!("xmlns:{prefix}"), uri)
        .with_attr("Requires", prefix);
    choice.push_element(frame);

    let mut pic = Element::new("p:pic");
    let mut nv_pic = Element::new("p:nvPicPr");
    let mut cnv_pr = Element::new("p:cNvPr")
        .with_attr("id", cnv.to_string())
        .with_attr("name", format!("Zoom {zoom_n}"));
    cnv_pr.push_element(
        Element::new("a:hlinkClick")
            .with_attr("r:id", slide_rid)
            .with_attr("action", "ppaction://hlinksldjump"),
    );
    nv_pic.push_element(cnv_pr);
    let mut cnv_pic = Element::new("p:cNvPicPr");
    cnv_pic.push_element(Element::new("a:picLocks").with_attr("noChangeAspect", "1"));
    nv_pic.push_element(cnv_pic);
    nv_pic.push_element(Element::new("p:nvPr"));
    pic.push_element(nv_pic);
    let mut blip_fill = Element::new("p:blipFill");
    blip_fill.push_element(Element::new("a:blip").with_attr("r:embed", img_rid));
    let mut stretch2 = Element::new("a:stretch");
    stretch2.push_element(Element::new("a:fillRect"));
    blip_fill.push_element(stretch2);
    pic.push_element(blip_fill);
    let mut pic_sp = Element::new("p:spPr");
    let mut pic_xfrm = Element::new("a:xfrm");
    pic_xfrm.push_element(
        Element::new("a:off")
            .with_attr("x", x.to_string())
            .with_attr("y", y.to_string()),
    );
    pic_xfrm.push_element(
        Element::new("a:ext")
            .with_attr("cx", cx.to_string())
            .with_attr("cy", cy.to_string()),
    );
    pic_sp.push_element(pic_xfrm);
    let mut pic_geom = Element::new("a:prstGeom").with_attr("prst", "rect");
    pic_geom.push_element(Element::new("a:avLst"));
    pic_sp.push_element(pic_geom);
    pic.push_element(pic_sp);

    let mut fallback = Element::new("mc:Fallback");
    fallback.push_element(pic);
    let mut ac = Element::new("mc:AlternateContent").with_attr("xmlns:mc", MC_NS);
    ac.push_element(choice);
    ac.push_element(fallback);
    ac
}

fn parse_i64_attr(el: &Element, name: &str) -> Option<i64> {
    el.attr(name)?.parse().ok()
}

fn parse_slide_n(n: &str, path: &str) -> Result<usize, Error> {
    let slide: usize = n.parse().map_err(|_| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SlideIndexNotInt,
            },
            path.to_string(),
        )
    })?;
    if slide < 1 {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SlideIndexFromOne,
            },
            path.to_string(),
        ));
    }
    Ok(slide)
}

fn ids_match(file: &str, req: &str) -> bool {
    match (file.parse::<u64>(), req.parse::<u64>()) {
        (Ok(a), Ok(b)) => a == b,
        _ => file == req,
    }
}
