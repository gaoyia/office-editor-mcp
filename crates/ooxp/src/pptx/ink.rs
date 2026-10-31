//! 幻灯片墨迹：一笔一个 `p:contentPart`，InkML 在 `/ppt/ink/inkN.xml`。
//! 不进 `pictures`。Fallback 海报不进 `pictures`。不把笔画放进 inspect 树。
//! 位置是画布 EMU，不是单元格锚点。

use serde_json::{json, Map, Value};

use crate::error::{Error, ErrorKind, IdReason, TypeName};
use crate::opc::{resolve_id, ContentTypes, Package};
use crate::xml::{Document, Element, Node};

use super::picture;
use super::shape;
use super::{DML_NS, REL_NS};

const MC_NS: &str = "http://schemas.openxmlformats.org/markup-compatibility/2006";
const P14_NS: &str = "http://schemas.microsoft.com/office/powerpoint/2010/main";
use crate::ink::{self, read_inkml, InkKey, CT_INKML};

pub(crate) fn pin_inks(root: &mut Element) {
    let mut n = 0usize;
    pin_ink_in(root, &mut n);
}

fn pin_ink_in(el: &mut Element, n: &mut usize) {
    if el.local_name() == "AlternateContent" {
        if let Some(choice) = el.child_by_local_mut("Choice") {
            if choice_requires_p14(choice) {
                if let Some(part) = choice.child_by_local_mut("contentPart") {
                    *n += 1;
                    let attr = crate::xml::pin_attr("ink");
                    if part.attr(&attr).is_none() {
                        part.set_attr(&attr, n.to_string());
                    }
                }
            }
        }
        return;
    }
    let kids: Vec<usize> = el
        .children
        .iter()
        .enumerate()
        .filter_map(|(i, node)| matches!(node, Node::Element(_)).then_some(i))
        .collect();
    for i in kids {
        if let Node::Element(child) = &mut el.children[i] {
            pin_ink_in(child, n);
        }
    }
}

fn ink_hit(el: &Element, key: &InkKey, index: usize) -> bool {
    if let InkKey::Index(n) = key {
        return crate::xml::pin_matches(el, &crate::xml::pin_attr("ink"), *n, index);
    }
    let raw = cnv_of(el).0;
    crate::ink::key_matches(key, &raw, index)
}

const DEFAULT_CX: i64 = 914_400;
const DEFAULT_CY: i64 = 914_400;

#[derive(Debug, Clone)]
pub struct InkTarget {
    pub slide: usize,
    pub on_notes: bool,
    pub master: Option<(usize, Option<usize>)>,
    pub key: InkKey,
}

#[derive(Debug, Clone)]
pub struct InsertInkOp {
    pub slide: usize,
    pub on_notes: bool,
    pub master: Option<(usize, Option<usize>)>,
    pub file: String,
    pub x: Option<i64>,
    pub y: Option<i64>,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct ReplaceInkOp {
    pub id: String,
    pub target: InkTarget,
    pub file: String,
}

#[derive(Debug, Clone)]
pub struct DeleteInkOp {
    pub id: String,
    pub target: InkTarget,
}

#[derive(Debug, Clone)]
pub struct SetInkSizeOp {
    pub id: String,
    pub target: InkTarget,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct SetInkPosOp {
    pub id: String,
    pub target: InkTarget,
    pub x: Option<i64>,
    pub y: Option<i64>,
}

pub(crate) struct InkInfo {
    id: String,
    cnv_pr_id: Option<String>,
    name: String,
    x: Option<i64>,
    y: Option<i64>,
    cx: Option<i64>,
    cy: Option<i64>,
    stable: bool,
}

pub fn looks_like_id(id: &str) -> bool {
    id.contains("/ink:") && (id.starts_with("slide:") || id.starts_with("master:"))
}

pub fn parse_id(id: &str, json_path: &str) -> Result<InkTarget, Error> {
    if picture::is_bare_host(id) {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SlideInkContainer,
            },
            json_path.to_string(),
        ));
    }
    let (host, after) = picture::parse_object_id(id, json_path, "ink", IdReason::SlideInkPrefix)?;
    if after.is_empty() {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SlideInkPrefix,
            },
            json_path.to_string(),
        ));
    }
    let (slide, on_notes, master) = host.parts();
    Ok(InkTarget {
        slide,
        on_notes,
        master,
        key: parse_key(after, json_path)?,
    })
}

fn parse_key(s: &str, path: &str) -> Result<InkKey, Error> {
    if let Some(rest) = s.strip_prefix("index:") {
        let n: usize = rest.parse().map_err(|_| {
            Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::SlideInkPrefix,
                },
                path.to_string(),
            )
        })?;
        if n < 1 {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::SlideInkPrefix,
                },
                path.to_string(),
            ));
        }
        return Ok(InkKey::Index(n));
    }
    if s.is_empty() || s.contains('/') {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SlideInkPrefix,
            },
            path.to_string(),
        ));
    }
    Ok(InkKey::Id(s.to_string()))
}

pub fn count(root: &Element) -> usize {
    collect(root, "").len()
}

pub fn collect(root: &Element, host: &str) -> Vec<InkInfo> {
    let mut parts = Vec::new();
    collect_parts(root, &mut parts);
    parts
        .into_iter()
        .enumerate()
        .map(|(i, part)| info_of(part, host, i + 1))
        .collect()
}

fn collect_parts<'a>(el: &'a Element, out: &mut Vec<&'a Element>) {
    if el.local_name() == "AlternateContent" {
        if let Some(choice) = el.child_by_local("Choice") {
            if choice_requires_p14(choice) {
                if let Some(part) = choice.child_by_local("contentPart") {
                    out.push(part);
                }
            }
        }
        return;
    }
    for c in el.element_children() {
        collect_parts(c, out);
    }
}

fn choice_requires_p14(choice: &Element) -> bool {
    choice
        .attr("Requires")
        .is_some_and(|s| s.split_whitespace().any(|t| t == "p14"))
}

fn info_of(part: &Element, host: &str, index: usize) -> InkInfo {
    let (raw, name) = cnv_of(part);
    let stable = raw.as_ref().is_some_and(|s| !s.is_empty());
    let id = match &raw {
        Some(v) if !v.is_empty() => format!("{host}/ink:{v}"),
        _ => format!("{host}/ink:index:{index}"),
    };
    let xfrm = part.child_by_local("xfrm");
    let (x, y) = match xfrm.and_then(|x| x.child_by_local("off")) {
        Some(off) => (parse_i64(off, "x"), parse_i64(off, "y")),
        None => (None, None),
    };
    let (cx, cy) = match xfrm.and_then(|x| x.child_by_local("ext")) {
        Some(ext) => (parse_i64(ext, "cx"), parse_i64(ext, "cy")),
        None => (None, None),
    };
    InkInfo {
        id,
        cnv_pr_id: raw.filter(|s| !s.is_empty()),
        name,
        x,
        y,
        cx,
        cy,
        stable,
    }
}

fn cnv_of(part: &Element) -> (Option<String>, String) {
    let mut raw = None;
    let mut name = String::new();
    if let Some(nv) = part.child_by_local("nvContentPartPr") {
        if let Some(pr) = nv.child_by_local("cNvPr") {
            raw = pr
                .attr_local("id")
                .filter(|s| !s.is_empty())
                .map(str::to_string);
            name = pr.attr_local("name").unwrap_or("").to_string();
        }
    }
    (raw, name)
}

fn parse_i64(el: &Element, name: &str) -> Option<i64> {
    el.attr_local(name)?.parse().ok()
}

pub fn list_json(items: &[InkInfo]) -> Vec<Value> {
    items.iter().map(info_json).collect()
}

fn info_json(p: &InkInfo) -> Value {
    let mut m = Map::new();
    m.insert("id".into(), json!(p.id));
    m.insert("cNvPrId".into(), json!(p.cnv_pr_id));
    m.insert("name".into(), json!(p.name));
    m.insert("x".into(), json!(p.x));
    m.insert("y".into(), json!(p.y));
    m.insert("cx".into(), json!(p.cx));
    m.insert("cy".into(), json!(p.cy));
    m.insert("contentType".into(), json!(CT_INKML));
    m.insert("stable".into(), json!(p.stable));
    Value::Object(m)
}

pub fn export(bytes: &[u8], id: &str) -> Result<super::ExportedMedia, Error> {
    let pkg = Package::open_bytes(bytes.to_vec())?;
    let target = parse_id(id, "id")?;
    let pres_part = crate::opc::office_document_part(&pkg)?;
    let pres = super::list_pin::parse_part(&pkg, &pres_part)?;
    let slides = shape::list_slides(&pkg, &pres_part, &pres.root)?;
    let host = picture::TreeHost::from_parts(target.slide, target.on_notes, target.master);
    let part = picture::resolve_tree_part(
        &pkg,
        &pres_part,
        &pres.root,
        &slides,
        &host,
        ErrorKind::InkNotFound { id: id.to_string() },
        "id",
    )?;
    let doc = super::list_pin::parse_part(&pkg, &part)?;
    let content = find_part(&doc.root, &target.key)
        .ok_or_else(|| Error::with_path(ErrorKind::InkNotFound { id: id.to_string() }, "id"))?;
    let rid = content
        .attr_local("id")
        .ok_or_else(|| Error::with_path(ErrorKind::InkNotFound { id: id.to_string() }, "id"))?;
    let ink_part = resolve_id(&pkg, &part, rid)?;
    let ink_bytes = pkg.get(&ink_part)?.to_vec();
    let cts = ContentTypes::from_package(&pkg)?;
    let content_type = cts.content_type_of(&ink_part).map(str::to_string);
    Ok(super::ExportedMedia {
        bytes: ink_bytes,
        content_type,
    })
}

fn find_part<'a>(root: &'a Element, key: &InkKey) -> Option<&'a Element> {
    let mut parts = Vec::new();
    collect_parts(root, &mut parts);
    let mut index = 0usize;
    for part in parts {
        index += 1;
        if ink_hit(part, key, index) {
            return Some(part);
        }
    }
    None
}

pub fn insert(
    pkg: &mut Package,
    slide_doc: &mut Document,
    slide_part: &str,
    op: &InsertInkOp,
    file_field: &str,
) -> Result<(), Error> {
    let inkml = read_inkml(&op.file, file_field)?;
    let (cx, cy) = match (op.cx, op.cy) {
        (Some(x), Some(y)) => (x, y),
        _ => (DEFAULT_CX, DEFAULT_CY),
    };
    let x = op.x.unwrap_or(0);
    let y = op.y.unwrap_or(0);
    let (ink_rid, png_rid) = write_media(pkg, slide_part, inkml)?;
    picture::ensure_xmlns(&mut slide_doc.root, "r", REL_NS);
    picture::ensure_xmlns(&mut slide_doc.root, "a", DML_NS);
    picture::ensure_xmlns(&mut slide_doc.root, "mc", MC_NS);
    let cnv = picture::next_cnv_id(&slide_doc.root);
    let n = collect(&slide_doc.root, "").len() + 1;
    let name = format!("墨迹 {n}");
    let ac = build_ac(cnv, &name, &ink_rid, &png_rid, x, y, cx, cy);
    picture::insert_into_sp_tree(&mut slide_doc.root, ac)?;
    Ok(())
}

fn write_media(
    pkg: &mut Package,
    host_part: &str,
    inkml: Vec<u8>,
) -> Result<(String, String), Error> {
    let png_part = picture::next_media_part(pkg, "png");
    let (ink_rid, png_rid) = ink::write_media(
        pkg,
        host_part,
        "/ppt/ink",
        inkml,
        Some(png_part),
        picture::rel_target_from,
        picture::REL_IMAGE,
    )?;
    Ok((ink_rid, png_rid.expect("ink preview")))
}

fn build_ac(
    cnv: u32,
    name: &str,
    ink_rid: &str,
    png_rid: &str,
    x: i64,
    y: i64,
    cx: i64,
    cy: i64,
) -> Element {
    let mut ac = Element::new("mc:AlternateContent")
        .with_attr("xmlns:mc", MC_NS)
        .with_attr("xmlns:p14", P14_NS);
    let mut choice = Element::new("mc:Choice").with_attr("Requires", "p14");
    let mut part = Element::new("p:contentPart")
        .with_attr("p14:bwMode", "auto")
        .with_attr("r:id", ink_rid);
    let mut nv = Element::new("p14:nvContentPartPr");
    nv.push_element(
        Element::new("p14:cNvPr")
            .with_attr("id", cnv.to_string())
            .with_attr("name", name),
    );
    nv.push_element(Element::new("p14:cNvContentPartPr"));
    nv.push_element(Element::new("p14:nvPr"));
    part.push_element(nv);
    let mut xfrm = Element::new("p14:xfrm");
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
    part.push_element(xfrm);
    choice.push_element(part);
    ac.push_element(choice);
    let mut fallback = Element::new("mc:Fallback");
    fallback.push_element(build_fallback_pic(cnv, name, png_rid, x, y, cx, cy));
    ac.push_element(fallback);
    ac
}

fn build_fallback_pic(
    cnv: u32,
    name: &str,
    rid: &str,
    x: i64,
    y: i64,
    cx: i64,
    cy: i64,
) -> Element {
    let mut pic = Element::new("p:pic");
    let mut nv = Element::new("p:nvPicPr");
    nv.push_element(
        Element::new("p:cNvPr")
            .with_attr("id", cnv.to_string())
            .with_attr("name", name),
    );
    nv.push_element(Element::new("p:cNvPicPr"));
    nv.push_element(Element::new("p:nvPr"));
    pic.push_element(nv);
    let mut fill = Element::new("p:blipFill");
    fill.push_element(Element::new("a:blip").with_attr("r:embed", rid));
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

pub fn replace(
    pkg: &mut Package,
    host_part: &str,
    host_root: &Element,
    op: &ReplaceInkOp,
    op_index: usize,
) -> Result<(), Error> {
    let path = format!("ops[{op_index}].id");
    let inkml = read_inkml(&op.file, &format!("ops[{op_index}].file"))?;
    let part = find_part(host_root, &op.target.key).ok_or_else(|| {
        Error::with_path(ErrorKind::InkNotFound { id: op.id.clone() }, path.clone())
    })?;
    let rid = part.attr_local("id").ok_or_else(|| {
        Error::with_path(ErrorKind::InkNotFound { id: op.id.clone() }, path.clone())
    })?;
    let ink_part = resolve_id(pkg, host_part, rid)?;
    pkg.set(&ink_part, inkml)?;
    Ok(())
}

pub fn delete(
    pkg: &mut Package,
    slide_doc: &mut Document,
    slide_part: &str,
    op: &DeleteInkOp,
    op_index: usize,
) -> Result<(), Error> {
    let path = format!("ops[{op_index}].id");
    let ac = find_ac(&slide_doc.root, &op.target.key).ok_or_else(|| {
        Error::with_path(ErrorKind::InkNotFound { id: op.id.clone() }, path.clone())
    })?;
    let spid = ac
        .child_by_local("Choice")
        .and_then(|c| c.child_by_local("contentPart"))
        .and_then(|p| cnv_of(p).0)
        .filter(|s| !s.is_empty());
    let mut rids = Vec::new();
    collect_rids(ac, &mut rids);
    if !remove_ink(&mut slide_doc.root, &op.target.key) {
        return Err(Error::with_path(
            ErrorKind::InkNotFound { id: op.id.clone() },
            path,
        ));
    }
    if let Some(id) = spid {
        shape::strip_timing_for_spid(&mut slide_doc.root, &id);
    }
    drop_rids(pkg, slide_part, &slide_doc.root, &rids)?;
    Ok(())
}

pub fn apply_size(slide_doc: &mut Document, op: &SetInkSizeOp, i: usize) -> Result<(), Error> {
    let path = format!("ops[{i}].id");
    let ac = find_ac_mut(&mut slide_doc.root, &op.target.key)
        .ok_or_else(|| Error::with_path(ErrorKind::InkNotFound { id: op.id.clone() }, path))?;
    patch_ac_geom(ac, op.cx, op.cy, None, None);
    Ok(())
}

pub fn apply_pos(slide_doc: &mut Document, op: &SetInkPosOp, i: usize) -> Result<(), Error> {
    let path = format!("ops[{i}].id");
    let ac = find_ac_mut(&mut slide_doc.root, &op.target.key)
        .ok_or_else(|| Error::with_path(ErrorKind::InkNotFound { id: op.id.clone() }, path))?;
    patch_ac_geom(ac, None, None, op.x, op.y);
    Ok(())
}

fn find_ac<'a>(root: &'a Element, key: &InkKey) -> Option<&'a Element> {
    find_ac_walk(root, key, &mut 0)
}

fn find_ac_walk<'a>(el: &'a Element, key: &InkKey, index: &mut usize) -> Option<&'a Element> {
    crate::xml::search(el, |el| {
        if el.local_name() == "AlternateContent" {
            if ac_matches(el, key, index) {
                return crate::xml::Search::Hit(el);
            }
            return crate::xml::Search::Skip;
        }
        crate::xml::Search::down(el)
    })
}

fn find_ac_mut<'a>(root: &'a mut Element, key: &InkKey) -> Option<&'a mut Element> {
    find_ac_walk_mut(root, key, &mut 0)
}

fn find_ac_walk_mut<'a>(
    el: &'a mut Element,
    key: &InkKey,
    index: &mut usize,
) -> Option<&'a mut Element> {
    crate::xml::search_mut(el, |el| {
        if el.local_name() == "AlternateContent" {
            if ac_matches(el, key, index) {
                return crate::xml::SearchMut::Hit;
            }
            return crate::xml::SearchMut::Skip;
        }
        crate::xml::SearchMut::down(el)
    })
}

fn ac_matches(ac: &Element, key: &InkKey, index: &mut usize) -> bool {
    let Some(choice) = ac.child_by_local("Choice") else {
        return false;
    };
    if !choice_requires_p14(choice) {
        return false;
    }
    let Some(part) = choice.child_by_local("contentPart") else {
        return false;
    };
    *index += 1;
    ink_hit(part, key, *index)
}

fn patch_ac_geom(
    ac: &mut Element,
    cx: Option<i64>,
    cy: Option<i64>,
    x: Option<i64>,
    y: Option<i64>,
) {
    for child in &mut ac.children {
        let Node::Element(branch) = child else {
            continue;
        };
        match branch.local_name() {
            "Choice" => {
                if let Some(part) = branch.child_by_local_mut("contentPart") {
                    if part.child_by_local("xfrm").is_none() {
                        part.push_element(Element::new("p14:xfrm"));
                    }
                    let xfrm = part.child_by_local_mut("xfrm").expect("刚插入的 p14:xfrm");
                    patch_off_ext(xfrm, cx, cy, x, y);
                }
            }
            "Fallback" => {
                if let Some(pic) = branch.child_by_local_mut("pic") {
                    let sp_pr = picture::ensure_child(pic, "spPr");
                    let xfrm = picture::ensure_child(sp_pr, "xfrm");
                    patch_off_ext(xfrm, cx, cy, x, y);
                }
            }
            _ => {}
        }
    }
}

fn patch_off_ext(
    xfrm: &mut Element,
    cx: Option<i64>,
    cy: Option<i64>,
    x: Option<i64>,
    y: Option<i64>,
) {
    if cx.is_some() || cy.is_some() {
        let ext = ensure_a(xfrm, "ext");
        if let Some(v) = cx {
            picture::set_attr_local(ext, "cx", &v.to_string());
        }
        if let Some(v) = cy {
            picture::set_attr_local(ext, "cy", &v.to_string());
        }
    }
    if x.is_some() || y.is_some() {
        let off = ensure_a(xfrm, "off");
        if let Some(v) = x {
            picture::set_attr_local(off, "x", &v.to_string());
        }
        if let Some(v) = y {
            picture::set_attr_local(off, "y", &v.to_string());
        }
    }
}

fn ensure_a<'a>(parent: &'a mut Element, local: &str) -> &'a mut Element {
    if parent.child_by_local(local).is_none() {
        parent.push_element(Element::new(format!("a:{local}")));
    }
    parent.child_by_local_mut(local).expect("刚插入的 a 子元素")
}

fn remove_ink(el: &mut Element, key: &InkKey) -> bool {
    remove_in(el, key, &mut 0)
}

fn remove_in(el: &mut Element, key: &InkKey, index: &mut usize) -> bool {
    let mut drop_at = None;
    for (i, n) in el.children.iter_mut().enumerate() {
        let Node::Element(child) = n else { continue };
        if child.local_name() == "AlternateContent" {
            let mut parts = Vec::new();
            collect_parts(child, &mut parts);
            let mut hit = false;
            for part in parts {
                *index += 1;
                if ink_hit(part, key, *index) {
                    hit = true;
                }
            }
            if hit {
                drop_at = Some(i);
                break;
            }
        } else if remove_in(child, key, index) {
            return true;
        }
    }
    if let Some(i) = drop_at {
        el.children.remove(i);
        return true;
    }
    false
}

fn collect_rids(el: &Element, out: &mut Vec<String>) {
    if el.local_name() == "contentPart" {
        if let Some(id) = el.attr_local("id") {
            if !id.is_empty() && !out.iter().any(|x| x == id) {
                out.push(id.to_string());
            }
        }
    }
    if el.local_name() == "blip" {
        if let Some(id) = el.attr_local("embed") {
            if !id.is_empty() && !out.iter().any(|x| x == id) {
                out.push(id.to_string());
            }
        }
    }
    for c in el.element_children() {
        collect_rids(c, out);
    }
}

fn drop_rids(pkg: &mut Package, part: &str, root: &Element, rids: &[String]) -> Result<(), Error> {
    ink::drop_rids(pkg, part, root, rids, picture::media_still_used)
}

pub fn parse_create(
    v: &Value,
    prefix: &str,
    slide: usize,
    on_notes: bool,
) -> Result<InsertInkOp, Error> {
    let obj = v.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: prefix.into(),
                expected: TypeName::Object,
            },
            prefix,
        )
    })?;
    crate::json::deny_unknown(obj, &["file", "x", "y", "cx", "cy"])?;
    let file = match obj.get("file") {
        Some(Value::String(s)) if !s.is_empty() => s.clone(),
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "file".into(),
                    expected: TypeName::NonEmptyString,
                },
                format!("{prefix}.file"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "file".into(),
                },
                format!("{prefix}.file"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "file".into(),
                    expected: TypeName::String,
                },
                format!("{prefix}.file"),
            ));
        }
    };
    let cx = parse_emu(obj.get("cx"), prefix, "cx")?;
    let cy = parse_emu(obj.get("cy"), prefix, "cy")?;
    if cx.is_some() != cy.is_some() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "insertInk".into(),
                field: "cx and cy together".into(),
            },
            prefix.to_string(),
        ));
    }
    Ok(InsertInkOp {
        slide,
        on_notes,
        master: None,
        file,
        x: parse_emu(obj.get("x"), prefix, "x")?,
        y: parse_emu(obj.get("y"), prefix, "y")?,
        cx,
        cy,
    })
}

fn parse_emu(v: Option<&Value>, prefix: &str, field: &str) -> Result<Option<i64>, Error> {
    match v {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(n)) => n
            .as_i64()
            .ok_or_else(|| {
                Error::with_path(
                    ErrorKind::WrongType {
                        field: field.into(),
                        expected: TypeName::Integer,
                    },
                    format!("{prefix}.{field}"),
                )
            })
            .map(Some),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Integer,
            },
            format!("{prefix}.{field}"),
        )),
    }
}
