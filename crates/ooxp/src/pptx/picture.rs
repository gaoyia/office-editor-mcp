//! 幻灯片 `p:pic`：收集、换、插、删、改尺寸与画布坐标。不引用 xlsx / docx。

use std::path::{Path, PathBuf};

use serde_json::{json, Map, Value};

use crate::error::{io_path, Error, ErrorKind, IdReason};
use crate::opc::{
    resolve_rel_target, split_dir_file, ContentTypes, Package, Relationship, Relationships,
};
use crate::xml::{Document, Element, Node};

use super::request::{
    DeletePictureOp, InsertPictureOp, ReplacePictureOp, SetPicturePosOp, SetPictureSizeOp,
};
use super::shape::{self, first_alt_child_index};
use super::{ExportedMedia, DML_NS, REL_NS};

pub const REL_IMAGE: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/image";

const GAP_EMU: i64 = 360_000;
const DEFAULT_SLIDE_CX: i64 = 12_192_000;
const DEFAULT_SLIDE_CY: i64 = 6_858_000;

pub struct ImageFile {
    pub bytes: Vec<u8>,
    pub ext: String,
    pub content_type: String,
    pub cx: i64,
    pub cy: i64,
}

#[derive(Debug, Clone)]
pub enum PicKey {
    Id(String),
    Index(usize),
}

#[derive(Debug, Clone)]
pub struct PicTarget {
    pub slide: usize,
    pub on_notes: bool,
    pub key: PicKey,
}

pub struct PicInfo {
    pub id: String,
    pub cnv_pr_id: Option<String>,
    pub name: String,
    pub x: Option<i64>,
    pub y: Option<i64>,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
    pub stable: bool,
}

pub fn parse_pic_id(id: &str, json_path: &str) -> Result<PicTarget, Error> {
    let path = json_path.to_string();
    let rest = id.strip_prefix("slide:").ok_or_else(|| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SlidePicPrefix,
            },
            path.clone(),
        )
    })?;
    let (n, after, on_notes) = if let Some((n, after)) = rest.split_once("/notes/pic:") {
        (n, after, true)
    } else if let Some((n, after)) = rest.split_once("/pic:") {
        (n, after, false)
    } else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SlidePicContainer,
            },
            path,
        ));
    };
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
                reason: IdReason::SlidePicContainer,
            },
            path,
        ));
    }
    if let Some(k) = after.strip_prefix("index:") {
        let idx: usize = k.parse().map_err(|_| {
            Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::PicIndexNotInt,
                },
                path.clone(),
            )
        })?;
        if idx < 1 {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::PicIndexFromOne,
                },
                path,
            ));
        }
        return Ok(PicTarget {
            slide,
            on_notes,
            key: PicKey::Index(idx),
        });
    }
    Ok(PicTarget {
        slide,
        on_notes,
        key: PicKey::Id(after.to_string()),
    })
}

pub fn parse_insert_slide_or_notes(
    id: &str,
    op_index: usize,
    bad: IdReason,
) -> Result<(usize, bool), Error> {
    let path = format!("ops[{op_index}].id");
    let rest = id.strip_prefix("slide:").ok_or_else(|| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: bad.clone(),
            },
            path.clone(),
        )
    })?;
    let (n, on_notes) = if let Some(n) = rest.strip_suffix("/notes") {
        if n.is_empty() || n.contains('/') {
            return Err(Error::with_path(ErrorKind::InvalidId { reason: bad }, path));
        }
        (n, true)
    } else if rest.is_empty() || rest.contains('/') {
        return Err(Error::with_path(ErrorKind::InvalidId { reason: bad }, path));
    } else {
        (rest, false)
    };
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
    Ok((slide, on_notes))
}

pub fn collect_pictures(root: &Element, host: &str) -> Vec<PicInfo> {
    let mut out = Vec::new();
    let mut index = 0usize;
    shape::walk_choice(root, &mut |e| {
        if !is_counted_pic(e) {
            return;
        }
        index += 1;
        out.push(pic_info(e, host, index));
    });
    out
}

pub fn pictures_json(pics: &[PicInfo]) -> Vec<Value> {
    pics.iter().map(pic_to_json).collect()
}

fn pic_to_json(p: &PicInfo) -> Value {
    let mut m = Map::new();
    m.insert("id".into(), json!(p.id));
    m.insert("cNvPrId".into(), json!(p.cnv_pr_id));
    m.insert("name".into(), json!(p.name));
    m.insert("x".into(), json!(p.x));
    m.insert("y".into(), json!(p.y));
    m.insert("cx".into(), json!(p.cx));
    m.insert("cy".into(), json!(p.cy));
    m.insert("stable".into(), json!(p.stable));
    Value::Object(m)
}

fn pic_info(pic: &Element, host: &str, index: usize) -> PicInfo {
    let cnv = pic_cnv_pr(pic);
    let cnv_pr_id = cnv
        .and_then(|c| c.attr("id"))
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    let name = cnv.and_then(|c| c.attr("name")).unwrap_or("").to_string();
    let (id, stable) = match &cnv_pr_id {
        Some(n) => (format!("{host}/pic:{n}"), true),
        None => (format!("{host}/pic:index:{index}"), false),
    };
    let xfrm = pic_xfrm(pic);
    let (x, y) = match xfrm.and_then(|x| x.child_by_local("off")) {
        Some(off) => (parse_i64_attr(off, "x"), parse_i64_attr(off, "y")),
        None => (None, None),
    };
    let (cx, cy) = match xfrm.and_then(|x| x.child_by_local("ext")) {
        Some(ext) => (parse_i64_attr(ext, "cx"), parse_i64_attr(ext, "cy")),
        None => (None, None),
    };
    PicInfo {
        id,
        cnv_pr_id,
        name,
        x,
        y,
        cx,
        cy,
        stable,
    }
}

fn parse_i64_attr(el: &Element, name: &str) -> Option<i64> {
    el.attr(name)?.parse().ok()
}

fn is_counted_pic(el: &Element) -> bool {
    el.local_name() == "pic" && blip_el(el).is_some()
}

fn blip_el(pic: &Element) -> Option<&Element> {
    pic.child_by_local("blipFill")
        .and_then(|f| f.child_by_local("blip"))
}

fn blip_embed(pic: &Element) -> Option<String> {
    blip_el(pic)
        .and_then(|b| b.attr_local("embed"))
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn pic_cnv_pr(pic: &Element) -> Option<&Element> {
    pic.child_by_local("nvPicPr")
        .and_then(|nv| nv.child_by_local("cNvPr"))
}

fn pic_xfrm(pic: &Element) -> Option<&Element> {
    pic.child_by_local("spPr")
        .and_then(|sp| sp.child_by_local("xfrm"))
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

fn find_pic_mut<'a>(root: &'a mut Element, key: &PicKey) -> Option<&'a mut Element> {
    let mut idx = 0usize;
    find_pic_in(root, &mut idx, key)
}

fn find_pic_in<'a>(el: &'a mut Element, idx: &mut usize, key: &PicKey) -> Option<&'a mut Element> {
    if el.local_name() == "AlternateContent" {
        if let Some(i) = first_alt_child_index(el) {
            if let Node::Element(child) = &mut el.children[i] {
                return find_pic_in(child, idx, key);
            }
        }
        return None;
    }
    if is_counted_pic(el) {
        *idx += 1;
        if pic_matches(el, *idx, key) {
            return Some(el);
        }
        return None;
    }
    for child in el.children.as_mut_slice() {
        if let Node::Element(e) = child {
            if let Some(found) = find_pic_in(e, idx, key) {
                return Some(found);
            }
        }
    }
    None
}

fn remove_pic(root: &mut Element, key: &PicKey) -> bool {
    let mut idx = 0usize;
    remove_pic_in(root, &mut idx, key)
}

fn remove_pic_in(el: &mut Element, idx: &mut usize, key: &PicKey) -> bool {
    if el.local_name() == "AlternateContent" {
        if let Some(i) = first_alt_child_index(el) {
            if let Node::Element(child) = &mut el.children[i] {
                return remove_pic_in(child, idx, key);
            }
        }
        return false;
    }
    let mut i = 0;
    while i < el.children.len() {
        let is_hit = match &el.children[i] {
            Node::Element(e) if is_counted_pic(e) => {
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
            if !is_counted_pic(e) && remove_pic_in(e, idx, key) {
                return true;
            }
        }
        i += 1;
    }
    false
}

pub(crate) fn next_cnv_id(root: &Element) -> u32 {
    let mut max = 1u32;
    shape::walk_choice(root, &mut |e| {
        if e.local_name() != "cNvPr" {
            return;
        }
        if let Some(n) = e.attr("id").and_then(|s| s.parse::<u32>().ok()) {
            max = max.max(n);
        }
    });
    max.saturating_add(1).max(2)
}

fn slide_size(pres_root: &Element) -> (i64, i64) {
    let Some(sz) = pres_root.child_by_local("sldSz") else {
        return (DEFAULT_SLIDE_CX, DEFAULT_SLIDE_CY);
    };
    let cx = parse_i64_attr(sz, "cx").unwrap_or(DEFAULT_SLIDE_CX);
    let cy = parse_i64_attr(sz, "cy").unwrap_or(DEFAULT_SLIDE_CY);
    (cx, cy)
}

pub(crate) fn canvas_size(pres_root: &Element) -> (i64, i64) {
    slide_size(pres_root)
}

struct BoxEmu {
    x: i64,
    y: i64,
    cx: i64,
    cy: i64,
}

fn occupied_boxes(root: &Element) -> Vec<BoxEmu> {
    let mut out = Vec::new();
    shape::walk_choice(root, &mut |e| {
        let xfrm = match e.local_name() {
            "sp" | "pic" | "cxnSp" => e
                .child_by_local("spPr")
                .and_then(|sp| sp.child_by_local("xfrm")),
            "graphicFrame" => e.child_by_local("xfrm"),
            "grpSp" => e
                .child_by_local("grpSpPr")
                .and_then(|g| g.child_by_local("xfrm")),
            _ => None,
        };
        let Some(xfrm) = xfrm else {
            return;
        };
        let off = xfrm.child_by_local("off");
        let ext = xfrm.child_by_local("ext");
        let x = off.and_then(|o| parse_i64_attr(o, "x")).unwrap_or(0);
        let y = off.and_then(|o| parse_i64_attr(o, "y")).unwrap_or(0);
        let cx = ext.and_then(|o| parse_i64_attr(o, "cx")).unwrap_or(0);
        let cy = ext.and_then(|o| parse_i64_attr(o, "cy")).unwrap_or(0);
        out.push(BoxEmu { x, y, cx, cy });
    });
    out
}

pub(crate) fn vacant_pos(
    pres_root: &Element,
    slide_root: &Element,
    pic_cx: i64,
    pic_cy: i64,
) -> (i64, i64) {
    vacant_pos_canvas(slide_size(pres_root), slide_root, pic_cx, pic_cy)
}

pub(crate) fn vacant_pos_canvas(
    (slide_cx, slide_cy): (i64, i64),
    slide_root: &Element,
    pic_cx: i64,
    pic_cy: i64,
) -> (i64, i64) {
    let boxes = occupied_boxes(slide_root);
    if boxes.is_empty() {
        return (GAP_EMU, GAP_EMU);
    }
    let max_right = boxes
        .iter()
        .map(|b| b.x.saturating_add(b.cx))
        .max()
        .unwrap_or(0);
    let min_y = boxes
        .iter()
        .map(|b| b.y)
        .min()
        .unwrap_or(GAP_EMU)
        .max(GAP_EMU);
    let max_bottom = boxes
        .iter()
        .map(|b| b.y.saturating_add(b.cy))
        .max()
        .unwrap_or(0);
    let right_x = max_right.saturating_add(GAP_EMU);
    if right_x.saturating_add(pic_cx) <= slide_cx {
        let y = if min_y.saturating_add(pic_cy) <= slide_cy {
            min_y
        } else {
            GAP_EMU
        };
        return (right_x.max(GAP_EMU), y);
    }
    let below_y = max_bottom.saturating_add(GAP_EMU);
    let y = if below_y <= 0 { GAP_EMU } else { below_y };
    let _ = slide_cy;
    (GAP_EMU, y.max(GAP_EMU))
}

pub fn export_media(bytes: &[u8], id: &str) -> Result<ExportedMedia, Error> {
    let target = parse_pic_id(id, "id")?;
    let pkg = Package::open_bytes(bytes.to_vec())?;
    let pres_part = crate::opc::office_document_part(&pkg)?;
    let pres = crate::xml::parse(pkg.get(&pres_part)?)?;
    let slides = shape::list_slides(&pkg, &pres_part, &pres.root)?;
    let meta = slides.get(target.slide - 1).ok_or_else(|| {
        Error::with_path(
            ErrorKind::SlideNotFound {
                n: target.slide as u32,
            },
            "id",
        )
    })?;
    let part = if target.on_notes {
        super::notes::notes_slide_part(&pkg, &meta.part)?.ok_or_else(|| {
            Error::with_path(ErrorKind::PictureNotFound { id: id.to_string() }, "id")
        })?
    } else {
        meta.part.clone()
    };
    let doc = crate::xml::parse(pkg.get(&part)?)?;
    let pic = find_pic_in_immut(&doc.root, &target.key)
        .ok_or_else(|| Error::with_path(ErrorKind::PictureNotFound { id: id.to_string() }, "id"))?;
    let rid = blip_embed(pic)
        .ok_or_else(|| Error::with_path(ErrorKind::PictureNotFound { id: id.to_string() }, "id"))?;
    let media = crate::opc::resolve_id(&pkg, &part, &rid)?;
    let media_bytes = pkg.get(&media)?.to_vec();
    let cts = ContentTypes::from_package(&pkg)?;
    let content_type = cts.content_type_of(&media).map(str::to_string);
    Ok(ExportedMedia {
        bytes: media_bytes,
        content_type,
    })
}

fn find_pic_in_immut<'a>(root: &'a Element, key: &PicKey) -> Option<&'a Element> {
    let mut idx = 0usize;
    let mut found = None;
    shape::walk_choice(root, &mut |e| {
        if found.is_some() || !is_counted_pic(e) {
            return;
        }
        idx += 1;
        if pic_matches(e, idx, key) {
            found = Some(e);
        }
    });
    found
}

pub fn replace_picture(
    pkg: &mut Package,
    slide_part: &str,
    slide_root: &Element,
    op: &ReplacePictureOp,
    op_index: usize,
) -> Result<(), Error> {
    let path = format!("ops[{op_index}].id");
    let pic = find_pic_in_immut(slide_root, &op.target.key)
        .ok_or_else(|| Error::with_path(ErrorKind::PictureNotFound { id: op.id.clone() }, path))?;
    let rid = blip_embed(pic).ok_or_else(|| {
        Error::with_path(
            ErrorKind::PictureNotFound { id: op.id.clone() },
            format!("ops[{op_index}].id"),
        )
    })?;
    let old_media = crate::opc::resolve_id(pkg, slide_part, &rid)?;
    let image = read_image(&op.file).map_err(|e| e.at(format!("ops[{op_index}].file")))?;
    let media = next_media_part(pkg, &image.ext);
    pkg.set(&media, image.bytes)?;
    update_rel_target(pkg, slide_part, &rid, &media)?;
    let mut cts = ContentTypes::from_package(pkg)?;
    cts.ensure_default(&image.ext, &image.content_type);
    cts.write_to(pkg)?;
    if old_media != media && !media_still_used(pkg, &old_media)? {
        pkg.remove(&old_media);
    }
    Ok(())
}

pub fn insert_picture(
    pkg: &mut Package,
    slide_doc: &mut Document,
    slide_part: &str,
    pres_root: &Element,
    op: &InsertPictureOp,
    file_field: &str,
) -> Result<(), Error> {
    let image = read_image(&op.file).map_err(|e| e.at(file_field))?;
    let (cx, cy) = match (op.cx, op.cy) {
        (Some(x), Some(y)) => (x, y),
        _ => (image.cx, image.cy),
    };
    let (x, y) = match (op.x, op.y) {
        (Some(px), Some(py)) => (px, py),
        _ => {
            let canvas = if op.on_notes {
                super::notes::notes_size(pres_root)
            } else {
                slide_size(pres_root)
            };
            vacant_pos_canvas(canvas, &slide_doc.root, cx, cy)
        }
    };
    ensure_xmlns(&mut slide_doc.root, "r", REL_NS);
    ensure_xmlns(&mut slide_doc.root, "a", DML_NS);
    let cnv = next_cnv_id(&slide_doc.root);
    let media = next_media_part(pkg, &image.ext);
    pkg.set(&media, image.bytes)?;
    let rid = add_image_rel(pkg, slide_part, &media)?;
    let mut cts = ContentTypes::from_package(pkg)?;
    cts.ensure_default(&image.ext, &image.content_type);
    cts.write_to(pkg)?;
    let name = format!("图片 {cnv}");
    let pic = build_pic(cnv, &name, &rid, x, y, cx, cy);
    insert_into_sp_tree(&mut slide_doc.root, pic)?;
    Ok(())
}

pub fn delete_picture(
    pkg: &mut Package,
    slide_doc: &mut Document,
    slide_part: &str,
    op: &DeletePictureOp,
    op_index: usize,
) -> Result<(), Error> {
    let path = format!("ops[{op_index}].id");
    let rid = find_pic_in_immut(&slide_doc.root, &op.target.key).and_then(blip_embed);
    if !remove_pic(&mut slide_doc.root, &op.target.key) {
        return Err(Error::with_path(
            ErrorKind::PictureNotFound { id: op.id.clone() },
            path,
        ));
    }
    let Some(rid) = rid else {
        return Ok(());
    };
    let old_media = match crate::opc::resolve_id(pkg, slide_part, &rid) {
        Ok(p) => p,
        Err(_) => return Ok(()),
    };
    let mut rels = Relationships::from_package(pkg, slide_part)?;
    if !embed_still_on_slide(&slide_doc.root, &rid) {
        rels.remove_by_id(&rid);
        rels.write_to(pkg, slide_part)?;
    }
    if !media_still_used(pkg, &old_media)? {
        pkg.remove(&old_media);
    }
    Ok(())
}

fn embed_still_on_slide(root: &Element, rid: &str) -> bool {
    let mut used = false;
    shape::walk_choice(root, &mut |e| {
        if used {
            return;
        }
        if blip_embed(e).as_deref() == Some(rid) {
            used = true;
        }
    });
    used
}

pub fn set_picture_size(
    slide_doc: &mut Document,
    op: &SetPictureSizeOp,
    op_index: usize,
) -> Result<(), Error> {
    let path = format!("ops[{op_index}].id");
    let pic = find_pic_mut(&mut slide_doc.root, &op.target.key)
        .ok_or_else(|| Error::with_path(ErrorKind::PictureNotFound { id: op.id.clone() }, path))?;
    let xfrm = ensure_xfrm(pic);
    let ext = ensure_child(xfrm, "ext");
    if let Some(cx) = op.cx {
        set_attr_local(ext, "cx", &cx.to_string());
    }
    if let Some(cy) = op.cy {
        set_attr_local(ext, "cy", &cy.to_string());
    }
    Ok(())
}

pub fn set_picture_pos(
    slide_doc: &mut Document,
    op: &SetPicturePosOp,
    op_index: usize,
) -> Result<(), Error> {
    let path = format!("ops[{op_index}].id");
    let pic = find_pic_mut(&mut slide_doc.root, &op.target.key)
        .ok_or_else(|| Error::with_path(ErrorKind::PictureNotFound { id: op.id.clone() }, path))?;
    let xfrm = ensure_xfrm(pic);
    let off = ensure_child(xfrm, "off");
    if let Some(x) = op.x {
        set_attr_local(off, "x", &x.to_string());
    }
    if let Some(y) = op.y {
        set_attr_local(off, "y", &y.to_string());
    }
    Ok(())
}

fn ensure_xfrm(pic: &mut Element) -> &mut Element {
    let sp_pr = ensure_child(pic, "spPr");
    ensure_child(sp_pr, "xfrm")
}

pub(crate) fn ensure_child<'a>(parent: &'a mut Element, local: &str) -> &'a mut Element {
    if parent.child_by_local(local).is_none() {
        let child = Element::new(parent.prefixed(local));
        parent.push_element(child);
    }
    parent.child_by_local_mut(local).expect("刚插入的子元素")
}

pub(crate) fn insert_into_sp_tree(root: &mut Element, pic: Element) -> Result<(), Error> {
    let parent = root.name.clone();
    let tree = root
        .child_by_local_mut("cSld")
        .and_then(|c| c.child_by_local_mut("spTree"))
        .ok_or_else(|| {
            Error::new(ErrorKind::XmlMissing {
                parent,
                child: "p:spTree".into(),
            })
        })?;
    if let Some(i) = tree
        .children
        .iter()
        .position(|n| matches!(n, Node::Element(e) if e.local_name() == "extLst"))
    {
        tree.children.insert(i, Node::Element(pic));
    } else {
        tree.push_element(pic);
    }
    Ok(())
}

fn build_pic(cnv: u32, name: &str, rid: &str, x: i64, y: i64, cx: i64, cy: i64) -> Element {
    let mut pic = Element::new("p:pic");
    let mut nv = Element::new("p:nvPicPr");
    nv.push_element(
        Element::new("p:cNvPr")
            .with_attr("id", cnv.to_string())
            .with_attr("name", name),
    );
    let mut cnv_pic = Element::new("p:cNvPicPr");
    cnv_pic.push_element(Element::new("a:picLocks").with_attr("noChangeAspect", "1"));
    nv.push_element(cnv_pic);
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

fn next_media_part(pkg: &Package, ext: &str) -> String {
    let mut n = 1u32;
    loop {
        let name = format!("/ppt/media/image{n}.{ext}");
        if !pkg.contains(&name) {
            return name;
        }
        n += 1;
    }
}

pub(crate) fn rel_target_from(source_part: &str, target_opc: &str) -> String {
    let src_dir = split_dir_file(source_part).0;
    let tgt = target_opc.trim_start_matches('/');
    let src = src_dir.trim_start_matches('/');
    if src.is_empty() || src == "/" {
        return tgt.to_string();
    }
    if let Some(rest) = tgt.strip_prefix(&format!("{src}/")) {
        return rest.to_string();
    }
    let src_parts: Vec<&str> = src.split('/').filter(|s| !s.is_empty()).collect();
    let tgt_parts: Vec<&str> = tgt.split('/').filter(|s| !s.is_empty()).collect();
    let mut common = 0usize;
    while common < src_parts.len()
        && common < tgt_parts.len()
        && src_parts[common] == tgt_parts[common]
    {
        common += 1;
    }
    let up = src_parts.len() - common;
    let rest = tgt_parts[common..].join("/");
    format!("{}{rest}", "../".repeat(up))
}

fn add_image_rel(pkg: &mut Package, part: &str, media_opc: &str) -> Result<String, Error> {
    let mut rels = Relationships::from_package(pkg, part)?;
    let rid = rels.next_rid();
    rels.push(Relationship {
        id: rid.clone(),
        rel_type: REL_IMAGE.into(),
        target: rel_target_from(part, media_opc),
        target_mode: None,
        extra: Vec::new(),
    });
    rels.write_to(pkg, part)?;
    Ok(rid)
}

fn update_rel_target(
    pkg: &mut Package,
    part: &str,
    rid: &str,
    new_target_opc: &str,
) -> Result<(), Error> {
    let mut rels = Relationships::from_package(pkg, part)?;
    if let Some(rel) = rels.rels.iter_mut().find(|r| r.id == rid) {
        rel.target = rel_target_from(part, new_target_opc);
    }
    rels.write_to(pkg, part)
}

pub(crate) fn media_still_used(pkg: &Package, media_opc: &str) -> Result<bool, Error> {
    for name in pkg.part_names() {
        if !name.ends_with(".rels") {
            continue;
        }
        let Some(source) = source_from_rels(&name) else {
            continue;
        };
        let rels = Relationships::parse(pkg.get(&name)?)?;
        for r in &rels.rels {
            if r.is_external() {
                continue;
            }
            if let Ok(p) = resolve_rel_target(&source, &r.target) {
                if p == media_opc {
                    return Ok(true);
                }
            }
        }
    }
    Ok(false)
}

fn source_from_rels(rels_part: &str) -> Option<String> {
    let trimmed = rels_part.strip_suffix(".rels")?;
    let (dir, file) = split_dir_file(trimmed);
    let parent = dir.trim_end_matches("/_rels").trim_end_matches("_rels");
    if parent.is_empty() || parent == "/" {
        Some(format!("/{file}"))
    } else {
        Some(format!("{parent}/{file}"))
    }
}

pub(crate) fn ensure_xmlns(root: &mut Element, prefix: &str, ns: &str) {
    let key = format!("xmlns:{prefix}");
    if root.attr(&key).is_none() && root.attr("xmlns") != Some(ns) {
        root.set_attr(&key, ns);
    }
}

pub(crate) fn set_attr_local(el: &mut Element, local: &str, value: &str) {
    if let Some((k, _)) = el
        .attrs
        .iter()
        .find(|(k, _)| k.rsplit(':').next().unwrap_or(k) == local)
    {
        let name = k.clone();
        el.set_attr(&name, value);
        return;
    }
    el.set_attr(local, value);
}

fn read_image(path: &str) -> Result<ImageFile, Error> {
    let p = PathBuf::from(path);
    let bytes = std::fs::read(&p).map_err(|e| io_path(e, p.clone()))?;
    classify_image(bytes, &p)
}

fn classify_image(bytes: Vec<u8>, path: &Path) -> Result<ImageFile, Error> {
    if bytes.len() >= 8 && bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) {
        let (cx, cy) = png_size(&bytes).unwrap_or((914400, 914400));
        return Ok(ImageFile {
            bytes,
            ext: "png".into(),
            content_type: "image/png".into(),
            cx,
            cy,
        });
    }
    if bytes.len() >= 3 && bytes[0] == 0xFF && bytes[1] == 0xD8 && bytes[2] == 0xFF {
        let (cx, cy) = jpeg_size(&bytes).unwrap_or((914400, 914400));
        return Ok(ImageFile {
            bytes,
            ext: "jpeg".into(),
            content_type: "image/jpeg".into(),
            cx,
            cy,
        });
    }
    if bytes.len() >= 6 && (bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a")) {
        let (cx, cy) = gif_size(&bytes).unwrap_or((914400, 914400));
        return Ok(ImageFile {
            bytes,
            ext: "gif".into(),
            content_type: "image/gif".into(),
            cx,
            cy,
        });
    }
    if bytes.len() >= 26 && bytes.starts_with(b"BM") {
        let (cx, cy) = bmp_size(&bytes).unwrap_or((914400, 914400));
        return Ok(ImageFile {
            bytes,
            ext: "bmp".into(),
            content_type: "image/bmp".into(),
            cx,
            cy,
        });
    }
    let _ = path;
    Err(Error::new(ErrorKind::InvalidImageFile))
}

fn px_to_emu(px: u32) -> i64 {
    (px as i64).saturating_mul(914400) / 96
}

fn png_size(bytes: &[u8]) -> Option<(i64, i64)> {
    if bytes.len() < 24 {
        return None;
    }
    let w = u32::from_be_bytes(bytes[16..20].try_into().ok()?);
    let h = u32::from_be_bytes(bytes[20..24].try_into().ok()?);
    Some((px_to_emu(w), px_to_emu(h)))
}

fn gif_size(bytes: &[u8]) -> Option<(i64, i64)> {
    if bytes.len() < 10 {
        return None;
    }
    let w = u16::from_le_bytes(bytes[6..8].try_into().ok()?) as u32;
    let h = u16::from_le_bytes(bytes[8..10].try_into().ok()?) as u32;
    Some((px_to_emu(w), px_to_emu(h)))
}

fn bmp_size(bytes: &[u8]) -> Option<(i64, i64)> {
    if bytes.len() < 26 {
        return None;
    }
    let w = i32::from_le_bytes(bytes[18..22].try_into().ok()?).unsigned_abs();
    let h = i32::from_le_bytes(bytes[22..26].try_into().ok()?).unsigned_abs();
    Some((px_to_emu(w), px_to_emu(h)))
}

fn jpeg_size(bytes: &[u8]) -> Option<(i64, i64)> {
    let mut i = 2usize;
    while i + 9 < bytes.len() {
        if bytes[i] != 0xFF {
            i += 1;
            continue;
        }
        let marker = bytes[i + 1];
        if marker == 0xD8 || marker == 0xD9 || (0xD0..=0xD7).contains(&marker) {
            i += 2;
            continue;
        }
        if i + 3 >= bytes.len() {
            break;
        }
        let len = u16::from_be_bytes([bytes[i + 2], bytes[i + 3]]) as usize;
        if matches!(marker, 0xC0..=0xC2) && i + 8 < bytes.len() {
            let h = u16::from_be_bytes([bytes[i + 5], bytes[i + 6]]) as u32;
            let w = u16::from_be_bytes([bytes[i + 7], bytes[i + 8]]) as u32;
            return Some((px_to_emu(w), px_to_emu(h)));
        }
        i += 2 + len;
    }
    None
}
