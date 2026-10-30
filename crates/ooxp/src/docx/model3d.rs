//! 正文与页眉页脚里的 3D 模型：`wp:inline` 里的 `am3d:model3d`。
//! 不进 `pictures`。Fallback 海报不进 `pictures`。

use std::collections::HashMap;

use serde_json::{json, Value};

use crate::error::{Error, ErrorKind, IdReason};
use crate::opc::{resolve_id, ContentTypes, Package, Relationships};
use crate::xml::{self, Document, Element, Node};

use super::drawing_hlink;
use super::hf::HfPart;
use super::para::{self, HostPara, TxbxKey, TxbxTarget};
use super::picture::{self, PicKind, WrapKind, WrapPos};
use super::{A_NS, MC_NS, REL_IMAGE, REL_NS, WP_NS};

#[derive(Debug, Clone)]
pub struct InsertModel3dOp {
    pub id: String,
    pub host: HostPara,
    pub file: String,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct ReplaceModel3dOp {
    pub id: String,
    pub target: TxbxTarget,
    pub file: String,
}

#[derive(Debug, Clone)]
pub struct DeleteModel3dOp {
    pub id: String,
    pub target: TxbxTarget,
}

#[derive(Debug, Clone)]
pub struct SetModel3dSizeOp {
    pub id: String,
    pub target: TxbxTarget,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct SetModel3dWrapOp {
    pub id: String,
    pub target: TxbxTarget,
    pub kind: PicKind,
    pub wrap: Option<WrapKind>,
    pub pos: WrapPos,
}

#[derive(Debug, Clone)]
pub struct SetModel3dCameraOp {
    pub id: String,
    pub target: TxbxTarget,
    pub patch: crate::model3d::Model3dCameraPatch,
}

pub struct Model3dInfo {
    pub id: String,
    pub name: Option<String>,
    pub kind: PicKind,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
    pub wrap: Option<WrapKind>,
    pub stable: bool,
    pub camera: Value,
    pub rot: Value,
    pub light_rig: Value,
    pub hyperlink: Value,
}

pub fn looks_like_id(id: &str) -> bool {
    id.contains("/model3d:")
        && !id.contains("/pic:")
        && !id.contains("/txbx:")
        && !id.contains("/cxn:")
        && !id.contains("/ink:")
}

pub fn parse_id(id: &str, op_index: usize) -> Result<TxbxTarget, Error> {
    parse_id_at(id, &format!("ops[{op_index}].id"))
}

pub fn parse_id_at(id: &str, path: &str) -> Result<TxbxTarget, Error> {
    let path = path.to_string();
    if id.contains("/pic:") || id.contains("/txbx:") || id.contains("/cxn:") || id.contains("/ink:")
    {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::DocxModel3dPrefix,
            },
            path,
        ));
    }
    let Some((host, after)) = id.split_once("/model3d:") else {
        if looks_like_host(id) {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::DocxModel3dContainer,
                },
                path,
            ));
        }
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::DocxModel3dPrefix,
            },
            path,
        ));
    };
    let (key, tail) = parse_key(after, &path)?;
    if tail.is_some() {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::DocxModel3dPrefix,
            },
            path,
        ));
    }
    let host = para::parse_host_para_id_at(host, &path)?;
    match host {
        HostPara::Body(_) | HostPara::HeaderFooter { .. } => Ok(TxbxTarget { host, key }),
        HostPara::InNote { .. } | HostPara::InComment { .. } => Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::DocxModel3dPrefix,
            },
            path,
        )),
    }
}

fn parse_key<'a>(s: &'a str, path: &str) -> Result<(TxbxKey, Option<&'a str>), Error> {
    if let Some(rest) = s.strip_prefix("index:") {
        let (num, tail) = match rest.split_once('/') {
            Some((a, b)) => (a, Some(b)),
            None => (rest, None),
        };
        let n: usize = num.parse().map_err(|_| {
            Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::DocxModel3dIndexNotInt,
                },
                path.to_string(),
            )
        })?;
        if n < 1 {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::DocxModel3dIndexFromOne,
                },
                path.to_string(),
            ));
        }
        return Ok((TxbxKey::Index(n), tail));
    }
    let (key, tail) = match s.split_once('/') {
        Some((a, b)) => (a, Some(b)),
        None => (s, None),
    };
    if key.is_empty() {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::DocxModel3dPrefix,
            },
            path.to_string(),
        ));
    }
    Ok((TxbxKey::DocPr(key.to_string()), tail))
}

fn looks_like_host(id: &str) -> bool {
    id.starts_with("para:")
        || id.starts_with("footnote:")
        || id.starts_with("endnote:")
        || id.starts_with("comment:")
        || id == "docx:document"
        || super::hf::HfKind::parse(id.split_once(':').map(|(k, _)| k).unwrap_or("")).is_some()
}

pub fn count_in(pkg: &Package, host: &str, root: &Element) -> Result<usize, Error> {
    Ok(collect(pkg, host, root, "")?.len())
}

pub fn collect(
    pkg: &Package,
    host: &str,
    root: &Element,
    host_para_id: &str,
) -> Result<Vec<Model3dInfo>, Error> {
    let rels = Relationships::from_package(pkg, host).ok();
    let mut out = Vec::new();
    let mut index = 0usize;
    let mut frames = Vec::new();
    collect_frames(root, &mut frames);
    for frame in frames {
        index += 1;
        out.push(info_of(frame, host_para_id, index, rels.as_ref()));
    }
    Ok(out)
}

fn collect_frames<'a>(el: &'a Element, out: &mut Vec<&'a Element>) {
    crate::model3d::collect_frames(el, out, &["inline", "anchor"], true);
}

fn info_of(
    frame: &Element,
    host_para_id: &str,
    index: usize,
    rels: Option<&Relationships>,
) -> Model3dInfo {
    let kind = if frame.local_name() == "anchor" {
        PicKind::Anchor
    } else {
        PicKind::Inline
    };
    let wrap = if kind == PicKind::Anchor {
        frame
            .element_children()
            .find_map(|c| WrapKind::from_local(c.local_name()))
    } else {
        None
    };
    let extent = frame.child_by_local("extent");
    let cx = extent
        .and_then(|e| e.attr_local("cx"))
        .and_then(|s| s.parse().ok());
    let cy = extent
        .and_then(|e| e.attr_local("cy"))
        .and_then(|s| s.parse().ok());
    let pr = frame.child_by_local("docPr");
    let raw = pr.and_then(|p| p.attr_local("id")).map(str::to_string);
    let name = pr
        .and_then(|p| p.attr_local("name"))
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    let stable = raw.as_ref().is_some_and(|s| !s.is_empty());
    let id = match &raw {
        Some(v) if !v.is_empty() => format!("{host_para_id}/model3d:{v}"),
        _ => format!("{host_para_id}/model3d:index:{index}"),
    };
    let (camera, rot, light_rig) = crate::model3d::inspect_view(frame);
    Model3dInfo {
        id,
        name,
        kind,
        cx,
        cy,
        wrap,
        stable,
        camera,
        rot,
        light_rig,
        hyperlink: drawing_hlink::inspect_picture(frame, rels),
    }
}

pub fn list_json(items: &[Model3dInfo]) -> Vec<Value> {
    items
        .iter()
        .map(|c| {
            json!({
                "id": c.id,
                "name": c.name,
                "kind": c.kind.as_str(),
                "cx": c.cx,
                "cy": c.cy,
                "wrap": c.wrap.map(WrapKind::as_str),
                "contentType": crate::model3d::CT_GLB,
                "stable": c.stable,
                "camera": c.camera,
                "rot": c.rot,
                "lightRig": c.light_rig,
                "hyperlink": c.hyperlink,
            })
        })
        .collect()
}

pub fn export_media(bytes: &[u8], id: &str) -> Result<super::export::ExportedMedia, Error> {
    let pkg = Package::open_bytes(bytes.to_vec())?;
    let target = parse_id_at(id, "id")?;
    let doc_part = crate::opc::office_document_part(&pkg)?;
    let doc_xml = xml::parse(pkg.get(&doc_part)?)?;
    let catalog = super::hf::catalog(&pkg, &doc_part, &doc_xml.root)?;
    let host = part_for_target(&catalog, &doc_part, &target, id, "id")?;
    let xml_doc = if host == doc_part {
        doc_xml
    } else {
        xml::parse(pkg.get(&host)?)?
    };
    let frame = find_frame(&xml_doc.root, &target.key)
        .ok_or_else(|| Error::with_path(ErrorKind::Model3dNotFound { id: id.to_string() }, "id"))?;
    let rid = crate::model3d::glb_embed_rid(frame)
        .ok_or_else(|| Error::with_path(ErrorKind::Model3dNotFound { id: id.to_string() }, "id"))?;
    let media = resolve_id(&pkg, &host, &rid)?;
    let media_bytes = pkg.get(&media)?.to_vec();
    let cts = ContentTypes::from_package(&pkg)?;
    let content_type = cts.content_type_of(&media).map(str::to_string);
    Ok(super::export::ExportedMedia {
        bytes: media_bytes,
        content_type,
    })
}

fn find_frame<'a>(root: &'a Element, key: &TxbxKey) -> Option<&'a Element> {
    let mut frames = Vec::new();
    collect_frames(root, &mut frames);
    let mut index = 0usize;
    for frame in frames {
        index += 1;
        let token = frame
            .child_by_local("docPr")
            .and_then(|p| p.attr_local("id"))
            .map(str::to_string);
        if matches_key(&token, index, key) {
            return Some(frame);
        }
    }
    None
}

pub(crate) fn find_frame_mut<'a>(root: &'a mut Element, key: &TxbxKey) -> Option<&'a mut Element> {
    find_frame_walk_mut(root, key, &mut 0)
}

fn find_frame_walk_mut<'a>(
    el: &'a mut Element,
    key: &TxbxKey,
    index: &mut usize,
) -> Option<&'a mut Element> {
    crate::xml::search_mut(el, |el| {
        if el.local_name() == "txbxContent" {
            return crate::xml::SearchMut::Skip;
        }
        if el.local_name() == "AlternateContent" {
            if let Some(i) = el
                .children
                .iter()
                .position(|n| matches!(n, Node::Element(c) if c.local_name() == "Choice"))
            {
                if let Node::Element(choice) = &el.children[i] {
                    if crate::model3d::choice_requires_am3d(choice) {
                        return crate::xml::SearchMut::Only(i);
                    }
                }
            }
            return crate::xml::SearchMut::Skip;
        }
        if matches!(el.local_name(), "inline" | "anchor") && crate::model3d::is_model3d_graphic(el)
        {
            *index += 1;
            let token = el
                .child_by_local("docPr")
                .and_then(|p| p.attr_local("id"))
                .map(str::to_string);
            if matches_key(&token, *index, key) {
                return crate::xml::SearchMut::Hit;
            }
            return crate::xml::SearchMut::Skip;
        }
        crate::xml::SearchMut::down(el)
    })
}

fn matches_key(doc_pr_id: &Option<String>, index: usize, key: &TxbxKey) -> bool {
    match key {
        TxbxKey::Index(n) => *n == index,
        TxbxKey::DocPr(want) => {
            let Some(got) = doc_pr_id.as_deref() else {
                return false;
            };
            if let (Ok(a), Ok(b)) = (want.parse::<u32>(), got.parse::<u32>()) {
                a == b
            } else {
                want == got
            }
        }
    }
}

pub fn apply_insert(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[HfPart],
    op: &InsertModel3dOp,
    i: usize,
) -> Result<(), Error> {
    if matches!(
        &op.host,
        HostPara::InNote { .. } | HostPara::InComment { .. }
    ) {
        return Err(Error::with_path(
            ErrorKind::GraphicInRestrictedStory,
            format!("ops[{i}].id"),
        ));
    }
    let host = part_for_host(catalog, doc_part, &op.host, &op.id, i)?;
    let glb = crate::model3d::read_glb(&op.file).map_err(|e| e.at(format!("ops[{i}].file")))?;
    let (glb_rid, png_rid) = write_media(pkg, &host, glb)?;
    let doc = load_part(pkg, docs, &host)?;
    ensure_ns(&mut doc.root);
    let doc_pr = picture::next_docpr_id(&doc.root);
    let (cx, cy) = match (op.cx, op.cy) {
        (Some(x), Some(y)) => (x, y),
        _ => (crate::model3d::DEFAULT_CX, crate::model3d::DEFAULT_CY),
    };
    let p = para::find_paragraph_mut(&mut doc.root, op.host.body()).ok_or_else(|| {
        Error::with_path(
            ErrorKind::ParaNotFound { id: op.id.clone() },
            format!("ops[{i}].id"),
        )
    })?;
    super::textbox::insert_ac(p, build_docx_ac(doc_pr, &glb_rid, &png_rid, cx, cy));
    dirty.insert(host, ());
    Ok(())
}

pub fn apply_replace(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    doc_part: &str,
    catalog: &[HfPart],
    op: &ReplaceModel3dOp,
    i: usize,
) -> Result<(), Error> {
    let host = part_for_target(
        catalog,
        doc_part,
        &op.target,
        &op.id,
        &format!("ops[{i}].id"),
    )?;
    let glb = crate::model3d::read_glb(&op.file).map_err(|e| e.at(format!("ops[{i}].file")))?;
    let (glb_rid, png_rid) = {
        let doc = load_part(pkg, docs, &host)?;
        let frame = find_frame(&doc.root, &op.target.key).ok_or_else(|| missing(&op.id, i))?;
        let glb_rid = crate::model3d::glb_embed_rid(frame).ok_or_else(|| missing(&op.id, i))?;
        let png_rid = crate::model3d::raster_embed_rid(frame);
        (glb_rid, png_rid)
    };
    let glb_part = resolve_id(pkg, &host, &glb_rid)?;
    pkg.set(&glb_part, glb)?;
    if let Some(rid) = png_rid {
        if let Ok(png_part) = resolve_id(pkg, &host, &rid) {
            pkg.set(&png_part, crate::ole::PREVIEW_PNG.to_vec())?;
        }
    }
    Ok(())
}

pub fn apply_delete(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[HfPart],
    op: &DeleteModel3dOp,
    i: usize,
) -> Result<(), Error> {
    let host = part_for_target(
        catalog,
        doc_part,
        &op.target,
        &op.id,
        &format!("ops[{i}].id"),
    )?;
    let rids = {
        let doc = load_part(pkg, docs, &host)?;
        let frame = find_frame(&doc.root, &op.target.key).ok_or_else(|| missing(&op.id, i))?;
        crate::model3d::collect_embed_rids(frame)
    };
    let mut old_parts = Vec::new();
    for rid in &rids {
        if let Ok(p) = resolve_id(pkg, &host, rid) {
            if !old_parts.iter().any(|x| x == &p) {
                old_parts.push(p);
            }
        }
    }
    {
        let doc = load_part(pkg, docs, &host)?;
        if !remove_model3d(&mut doc.root, &op.target.key) {
            return Err(missing(&op.id, i));
        }
    }
    dirty.insert(host.clone(), ());
    let root_ref = &docs.get(&host).unwrap().root;
    let mut rels = Relationships::from_package(pkg, &host)?;
    let mut dirty_rels = false;
    for rid in &rids {
        if !rid_still_in(root_ref, rid) {
            rels.remove_by_id(rid);
            dirty_rels = true;
        }
    }
    if dirty_rels {
        rels.write_to(pkg, &host)?;
    }
    for part in old_parts {
        if !picture::media_still_used(pkg, &part)? {
            pkg.remove(&part);
        }
    }
    Ok(())
}

pub fn apply_set_size(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[HfPart],
    op: &SetModel3dSizeOp,
    i: usize,
) -> Result<(), Error> {
    let host = part_for_target(
        catalog,
        doc_part,
        &op.target,
        &op.id,
        &format!("ops[{i}].id"),
    )?;
    let doc = load_part(pkg, docs, &host)?;
    let ac = find_ac_mut(&mut doc.root, &op.target.key).ok_or_else(|| missing(&op.id, i))?;
    let ok = each_host_drawing_mut(ac, |drawing| Ok(picture::set_extent(drawing, op.cx, op.cy)))?;
    if !ok {
        return Err(missing(&op.id, i));
    }
    dirty.insert(host, ());
    Ok(())
}

pub fn apply_set_wrap(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[HfPart],
    op: &SetModel3dWrapOp,
    i: usize,
) -> Result<(), Error> {
    let host = part_for_target(
        catalog,
        doc_part,
        &op.target,
        &op.id,
        &format!("ops[{i}].id"),
    )?;
    let doc = load_part(pkg, docs, &host)?;
    ensure_ns(&mut doc.root);
    let ac = find_ac_mut(&mut doc.root, &op.target.key).ok_or_else(|| missing(&op.id, i))?;
    let ok = each_host_drawing_mut(ac, |drawing| {
        picture::apply_wrap(drawing, op.kind, op.wrap, &op.pos)
            .map_err(|e| e.at(format!("ops[{i}]")))?;
        Ok(true)
    })?;
    if !ok {
        return Err(missing(&op.id, i));
    }
    dirty.insert(host, ());
    Ok(())
}

pub fn apply_set_camera(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[HfPart],
    op: &SetModel3dCameraOp,
    i: usize,
) -> Result<(), Error> {
    let host = part_for_target(
        catalog,
        doc_part,
        &op.target,
        &op.id,
        &format!("ops[{i}].id"),
    )?;
    let doc = load_part(pkg, docs, &host)?;
    let ac = find_ac_mut(&mut doc.root, &op.target.key).ok_or_else(|| missing(&op.id, i))?;
    let mut ok = false;
    for child in &mut ac.children {
        let Node::Element(branch) = child else {
            continue;
        };
        if branch.local_name() != "Choice" {
            continue;
        }
        if let Some(model) = crate::model3d::model3d_in_mut(branch) {
            crate::model3d::apply_camera_patch(model, &op.patch)?;
            ok = true;
        }
    }
    if !ok {
        return Err(missing(&op.id, i));
    }
    dirty.insert(host, ());
    Ok(())
}

fn find_ac_mut<'a>(root: &'a mut Element, key: &TxbxKey) -> Option<&'a mut Element> {
    find_ac_walk_mut(root, key, &mut 0)
}

fn find_ac_walk_mut<'a>(
    el: &'a mut Element,
    key: &TxbxKey,
    index: &mut usize,
) -> Option<&'a mut Element> {
    crate::xml::search_mut(el, |el| {
        if el.local_name() == "txbxContent" {
            return crate::xml::SearchMut::Skip;
        }
        if el.local_name() == "AlternateContent" {
            if ac_matches(el, key, index) {
                return crate::xml::SearchMut::Hit;
            }
            return crate::xml::SearchMut::Skip;
        }
        crate::xml::SearchMut::down(el)
    })
}

fn ac_matches(ac: &Element, key: &TxbxKey, index: &mut usize) -> bool {
    let Some(choice) = ac.child_by_local("Choice") else {
        return false;
    };
    if !crate::model3d::choice_requires_am3d(choice) {
        return false;
    }
    let mut frames = Vec::new();
    collect_frames(choice, &mut frames);
    let mut hit = false;
    for frame in frames {
        *index += 1;
        let token = frame
            .child_by_local("docPr")
            .and_then(|p| p.attr_local("id"))
            .map(str::to_string);
        if matches_key(&token, *index, key) {
            hit = true;
        }
    }
    hit
}

fn each_host_drawing_mut(
    ac: &mut Element,
    mut f: impl FnMut(&mut Element) -> Result<bool, Error>,
) -> Result<bool, Error> {
    let mut any = false;
    for child in &mut ac.children {
        let Node::Element(branch) = child else {
            continue;
        };
        if !matches!(branch.local_name(), "Choice" | "Fallback") {
            continue;
        }
        for node in &mut branch.children {
            let Node::Element(drawing) = node else {
                continue;
            };
            if drawing.local_name() != "drawing" {
                continue;
            }
            if f(drawing)? {
                any = true;
            }
        }
    }
    Ok(any)
}

fn write_media(pkg: &mut Package, host: &str, glb: Vec<u8>) -> Result<(String, String), Error> {
    let png_part = picture::next_media_part(pkg, "png");
    crate::model3d::write_glb_media(
        pkg,
        host,
        "/word/media",
        glb,
        png_part,
        picture::rel_target_from,
        REL_IMAGE,
    )
}

fn build_docx_ac(doc_pr: u32, glb_rid: &str, png_rid: &str, cx: i64, cy: i64) -> Element {
    let mut ac = Element::new("mc:AlternateContent").with_attr("xmlns:mc", MC_NS);
    let mut choice = Element::new("mc:Choice")
        .with_attr("xmlns:am3d", crate::model3d::AM3D_NS)
        .with_attr("Requires", "am3d");
    let mut drawing = Element::new("w:drawing");
    drawing.push_element(build_inline(doc_pr, glb_rid, png_rid, cx, cy, false));
    choice.push_element(drawing);
    ac.push_element(choice);
    let mut fallback = Element::new("mc:Fallback");
    fallback.push_element(build_fallback_drawing(doc_pr, png_rid, cx, cy));
    ac.push_element(fallback);
    ac
}

fn build_inline(
    doc_pr: u32,
    glb_rid: &str,
    png_rid: &str,
    cx: i64,
    cy: i64,
    fallback: bool,
) -> Element {
    let mut inline = Element::new("wp:inline")
        .with_attr("distT", "0")
        .with_attr("distB", "0")
        .with_attr("distL", "0")
        .with_attr("distR", "0");
    inline.push_element(
        Element::new("wp:extent")
            .with_attr("cx", cx.to_string())
            .with_attr("cy", cy.to_string()),
    );
    inline.push_element(
        Element::new("wp:effectExtent")
            .with_attr("l", "0")
            .with_attr("t", "0")
            .with_attr("r", "0")
            .with_attr("b", "0"),
    );
    inline.push_element(
        Element::new("wp:docPr")
            .with_attr("id", doc_pr.to_string())
            .with_attr("name", format!("3D 模型 {doc_pr}")),
    );
    let mut frame_pr = Element::new("wp:cNvGraphicFramePr");
    frame_pr.push_element(Element::new("a:graphicFrameLocks").with_attr("noChangeAspect", "1"));
    inline.push_element(frame_pr);
    let mut graphic = Element::new("a:graphic");
    if fallback {
        let mut data = Element::new("a:graphicData").with_attr(
            "uri",
            "http://schemas.openxmlformats.org/drawingml/2006/picture",
        );
        data.push_element(build_pic(doc_pr, png_rid, cx, cy));
        graphic.push_element(data);
    } else {
        let mut data = Element::new("a:graphicData").with_attr("uri", crate::model3d::AM3D_NS);
        data.push_element(crate::model3d::build_model3d_el(glb_rid, png_rid, cx, cy));
        graphic.push_element(data);
    }
    inline.push_element(graphic);
    inline
}

fn build_fallback_drawing(doc_pr: u32, png_rid: &str, cx: i64, cy: i64) -> Element {
    let mut drawing = Element::new("w:drawing");
    drawing.push_element(build_inline(doc_pr, "", png_rid, cx, cy, true));
    drawing
}

fn build_pic(doc_pr: u32, rid: &str, cx: i64, cy: i64) -> Element {
    let mut pic = Element::new("pic:pic")
        .with_attr("xmlns:pic", super::PIC_NS)
        .with_attr("xmlns:r", REL_NS);
    let mut nv = Element::new("pic:nvPicPr");
    nv.push_element(
        Element::new("pic:cNvPr")
            .with_attr("id", doc_pr.to_string())
            .with_attr("name", format!("3D 模型 {doc_pr}")),
    );
    let mut cnv = Element::new("pic:cNvPicPr");
    cnv.push_element(Element::new("a:picLocks").with_attr("noChangeAspect", "1"));
    nv.push_element(cnv);
    pic.push_element(nv);
    let mut fill = Element::new("pic:blipFill");
    fill.push_element(Element::new("a:blip").with_attr("r:embed", rid));
    let mut stretch = Element::new("a:stretch");
    stretch.push_element(Element::new("a:fillRect"));
    fill.push_element(stretch);
    pic.push_element(fill);
    let mut sp_pr = Element::new("pic:spPr");
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
    pic.push_element(sp_pr);
    pic
}

fn ensure_ns(root: &mut Element) {
    picture::ensure_drawing_ns(root);
    ensure_xmlns(root, "mc", MC_NS);
    ensure_xmlns(root, "wp", WP_NS);
    ensure_xmlns(root, "a", A_NS);
}

fn ensure_xmlns(el: &mut Element, prefix: &str, uri: &str) {
    let key = format!("xmlns:{prefix}");
    if el.attr(&key).is_none() {
        el.set_attr(&key, uri);
    }
}

fn remove_model3d(el: &mut Element, key: &TxbxKey) -> bool {
    remove_in(el, key, &mut 0)
}

fn remove_in(el: &mut Element, key: &TxbxKey, index: &mut usize) -> bool {
    if el.local_name() == "txbxContent" {
        return false;
    }
    let mut drop = None;
    for (i, n) in el.children.iter_mut().enumerate() {
        let Node::Element(child) = n else { continue };
        if child.local_name() == "AlternateContent" {
            let mut frames = Vec::new();
            collect_frames(child, &mut frames);
            let mut hit = false;
            for frame in frames {
                *index += 1;
                let token = frame
                    .child_by_local("docPr")
                    .and_then(|p| p.attr_local("id"))
                    .map(str::to_string);
                if matches_key(&token, *index, key) {
                    hit = true;
                }
            }
            if hit {
                drop = Some(i);
                break;
            }
        } else if matches!(child.local_name(), "inline" | "anchor")
            && crate::model3d::is_model3d_graphic(child)
        {
            *index += 1;
            let token = child
                .child_by_local("docPr")
                .and_then(|p| p.attr_local("id"))
                .map(str::to_string);
            if matches_key(&token, *index, key) {
                drop = Some(i);
                break;
            }
        } else if remove_in(child, key, index) {
            return true;
        }
    }
    if let Some(i) = drop {
        el.children.remove(i);
        return true;
    }
    false
}

fn part_for_host(
    catalog: &[HfPart],
    doc_part: &str,
    host: &HostPara,
    id: &str,
    i: usize,
) -> Result<String, Error> {
    match host {
        HostPara::Body(_) => Ok(doc_part.to_string()),
        HostPara::HeaderFooter {
            kind, hf_type, n, ..
        } => {
            let container_id = format!("{}:{}:{n}", kind.as_str(), hf_type.as_str());
            let item = super::hf::find_by_id(catalog, &container_id).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::ParaNotFound { id: id.to_string() },
                    format!("ops[{i}].id"),
                )
            })?;
            Ok(item.part.clone())
        }
        HostPara::InNote { .. } | HostPara::InComment { .. } => Err(Error::with_path(
            ErrorKind::GraphicInRestrictedStory,
            format!("ops[{i}].id"),
        )),
    }
}

pub(crate) fn part_for_target(
    catalog: &[HfPart],
    doc_part: &str,
    target: &TxbxTarget,
    id: &str,
    path: &str,
) -> Result<String, Error> {
    match &target.host {
        HostPara::Body(_) => Ok(doc_part.to_string()),
        HostPara::HeaderFooter {
            kind, hf_type, n, ..
        } => {
            let container_id = format!("{}:{}:{n}", kind.as_str(), hf_type.as_str());
            let item = super::hf::find_by_id(catalog, &container_id).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::Model3dNotFound { id: id.to_string() },
                    path.to_string(),
                )
            })?;
            Ok(item.part.clone())
        }
        HostPara::InNote { .. } | HostPara::InComment { .. } => Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::DocxModel3dPrefix,
            },
            path.to_string(),
        )),
    }
}

fn load_part<'a>(
    pkg: &Package,
    docs: &'a mut HashMap<String, Document>,
    part_name: &str,
) -> Result<&'a mut Document, Error> {
    if !docs.contains_key(part_name) {
        let parsed = xml::parse(pkg.get(part_name)?)?;
        docs.insert(part_name.to_string(), parsed);
    }
    Ok(docs.get_mut(part_name).unwrap())
}

fn rid_still_in(root: &Element, rid: &str) -> bool {
    let mut found = false;
    crate::model3d::walk(root, &mut |e| {
        if (e.attr_local("embed") == Some(rid) || e.attr_local("id") == Some(rid))
            && e.local_name() != "cNvPr"
            && e.local_name() != "docPr"
        {
            found = true;
        }
    });
    found
}

fn missing(id: &str, i: usize) -> Error {
    Error::with_path(
        ErrorKind::Model3dNotFound { id: id.to_string() },
        format!("ops[{i}].id"),
    )
}
