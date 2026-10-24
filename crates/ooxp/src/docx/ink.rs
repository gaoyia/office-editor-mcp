//! 正文与页眉页脚里的墨迹画布（非批注）：收集、导出、插删换、改尺寸与绕排。
//! 不把手写识别成字，不把 InkML 原文塞进 inspect 树。

use std::collections::HashMap;

use serde_json::{json, Value};

use crate::error::{Error, ErrorKind, IdReason};
use crate::opc::{
    rels_part_name, resolve_id, resolve_rel_target, ContentTypes, Package, Relationship,
    Relationships,
};
use crate::xml::{self, Document, Element, Node};

use super::hf::HfPart;
use super::para::{self, HostPara, TxbxKey, TxbxTarget};
use super::picture::{self, PicKind, WrapKind, WrapPos};
use super::{A_NS, MC_NS, REL_NS, W14_NS, WP_NS};

/// Word 16.0 另存的画布命名空间。
pub const WPC_NS: &str = "http://schemas.microsoft.com/office/word/2010/wordprocessingCanvas";
/// Word 另存会声明，写入 Choice 不走这条 URI。
pub const WPI_NS: &str = "http://schemas.microsoft.com/office/word/2010/wordprocessingInk";
pub const INKML_NS: &str = "http://www.w3.org/2003/InkML";
pub const CT_INKML: &str = "application/inkml+xml";
pub const REL_CUSTOM_XML: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/customXml";

/// Word 16.0 另存 144×72 磅。
pub const DEFAULT_CX: i64 = 1_828_800;
pub const DEFAULT_CY: i64 = 914_400;

#[derive(Debug, Clone)]
pub struct InsertInkCanvasOp {
    pub id: String,
    pub host: HostPara,
    pub file: String,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct ReplaceInkCanvasOp {
    pub id: String,
    pub target: TxbxTarget,
    pub file: String,
}

#[derive(Debug, Clone)]
pub struct DeleteInkCanvasOp {
    pub id: String,
    pub target: TxbxTarget,
}

#[derive(Debug, Clone)]
pub struct SetInkCanvasSizeOp {
    pub id: String,
    pub target: TxbxTarget,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct SetInkCanvasWrapOp {
    pub id: String,
    pub target: TxbxTarget,
    pub kind: PicKind,
    pub wrap: Option<WrapKind>,
    pub pos: WrapPos,
}

pub struct InkInfo {
    pub id: String,
    pub name: Option<String>,
    pub kind: PicKind,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
    pub wrap: Option<WrapKind>,
    pub stable: bool,
    pub hyperlink: Value,
}

pub(crate) struct HostCtx<'a> {
    pkg: &'a Package,
    host: &'a str,
    rels: Relationships,
    cts: ContentTypes,
}

impl<'a> HostCtx<'a> {
    pub(crate) fn load(pkg: &'a Package, host: &'a str) -> Result<Self, Error> {
        Ok(Self {
            pkg,
            host,
            rels: Relationships::from_package(pkg, host)?,
            cts: ContentTypes::from_package(pkg)?,
        })
    }

    fn is_ink(&self, part_el: &Element) -> bool {
        let Some(rid) = part_el.attr_local("id") else {
            return false;
        };
        let Some(rel) = self.rels.find_by_id(rid) else {
            return false;
        };
        if rel.is_external() {
            return false;
        }
        let Ok(part) = resolve_rel_target(self.host, &rel.target) else {
            return false;
        };
        if self.cts.content_type_of(&part) == Some(CT_INKML) {
            return true;
        }
        let Ok(bytes) = self.pkg.get(&part) else {
            return false;
        };
        xml::parse(bytes)
            .ok()
            .is_some_and(|d| is_inkml_root(&d.root))
    }
}

pub fn count_in(pkg: &Package, host: &str, root: &Element) -> Result<usize, Error> {
    Ok(collect(pkg, host, root, "")?.len())
}

pub fn collect(
    pkg: &Package,
    host: &str,
    root: &Element,
    host_para_id: &str,
) -> Result<Vec<InkInfo>, Error> {
    let ctx = HostCtx::load(pkg, host)?;
    let mut out = Vec::new();
    let mut index = 0usize;
    walk_host(root, &mut |drawing| {
        collect_in_drawing(&ctx, drawing, host_para_id, &mut index, &mut out);
        Ok(())
    })?;
    Ok(out)
}

pub fn canvases_json(items: &[InkInfo]) -> Vec<Value> {
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
                "contentType": CT_INKML,
                "stable": c.stable,
                "hyperlink": c.hyperlink,
            })
        })
        .collect()
}

pub fn looks_like_ink_id(id: &str) -> bool {
    id.contains("/ink:")
        && !id.contains("/pic:")
        && !id.contains("/txbx:")
        && !id.contains("/cxn:")
        && !id.contains("/model3d:")
}

pub fn parse_ink_id(id: &str, op_index: usize) -> Result<TxbxTarget, Error> {
    parse_ink_id_at(id, &format!("ops[{op_index}].id"))
}

pub fn parse_ink_id_at(id: &str, path: &str) -> Result<TxbxTarget, Error> {
    let path = path.to_string();
    if id.contains("/pic:") || id.contains("/txbx:") || id.contains("/cxn:") {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::DocxInkPrefix,
            },
            path,
        ));
    }
    let Some((host, after)) = id.split_once("/ink:") else {
        if looks_like_host(id) {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::DocxInkContainer,
                },
                path,
            ));
        }
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::DocxInkPrefix,
            },
            path,
        ));
    };
    let (key, tail) = parse_ink_key(after, &path)?;
    if tail.is_some() {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::DocxInkPrefix,
            },
            path,
        ));
    }
    let host = para::parse_host_para_id_at(host, &path)?;
    match host {
        HostPara::Body(_) | HostPara::HeaderFooter { .. } => Ok(TxbxTarget { host, key }),
        HostPara::InNote { .. } | HostPara::InComment { .. } => Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::DocxInkPrefix,
            },
            path,
        )),
    }
}

pub fn export_media(bytes: &[u8], id: &str) -> Result<super::export::ExportedMedia, Error> {
    let pkg = Package::open_bytes(bytes.to_vec())?;
    let target = parse_ink_id_at(id, "id")?;
    let doc_part = crate::opc::office_document_part(&pkg)?;
    let doc_xml = xml::parse(pkg.get(&doc_part)?)?;
    let catalog = super::hf::catalog(&pkg, &doc_part, &doc_xml.root)?;
    let host = part_for_target(&catalog, &doc_part, &target, id, "id")?;
    let xml_doc = if host == doc_part {
        doc_xml
    } else {
        xml::parse(pkg.get(&host)?)?
    };
    let ctx = HostCtx::load(&pkg, &host)?;
    let rid = find_rid(&ctx, &xml_doc.root, &target.key)
        .ok_or_else(|| Error::with_path(ErrorKind::InkNotFound { id: id.to_string() }, "id"))?;
    let part = resolve_id(&pkg, &host, &rid)?;
    let ink_bytes = pkg.get(&part)?.to_vec();
    let cts = ContentTypes::from_package(&pkg)?;
    let content_type = cts.content_type_of(&part).map(str::to_string);
    Ok(super::export::ExportedMedia {
        bytes: ink_bytes,
        content_type,
    })
}

pub fn apply_insert(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[HfPart],
    op: &InsertInkCanvasOp,
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
    let bytes = read_inkml(&op.file, i)?;
    let ink_part = crate::ole::next_numbered(pkg, "/word/ink", "ink", "xml");
    pkg.set(&ink_part, bytes)?;

    let mut host_rels = Relationships::from_package(pkg, &host)?;
    let rid = host_rels.next_rid();
    host_rels.push(Relationship {
        id: rid.clone(),
        rel_type: REL_CUSTOM_XML.into(),
        target: picture::rel_target_from(&host, &ink_part),
        target_mode: None,
        extra: Vec::new(),
    });
    host_rels.write_to(pkg, &host)?;

    let mut cts = ContentTypes::from_package(pkg)?;
    cts.set_override(&ink_part, CT_INKML)?;
    cts.write_to(pkg)?;

    let doc = load_part(pkg, docs, &host)?;
    ensure_ink_ns(&mut doc.root);
    let canvas_id = picture::next_docpr_id(&doc.root);
    let ink_id = canvas_id + 1;
    let (cx, cy) = match (op.cx, op.cy) {
        (Some(x), Some(y)) => (x, y),
        _ => (DEFAULT_CX, DEFAULT_CY),
    };
    let p = para::find_paragraph_mut(&mut doc.root, op.host.body()).ok_or_else(|| {
        Error::with_path(
            ErrorKind::ParaNotFound { id: op.id.clone() },
            format!("ops[{i}].id"),
        )
    })?;
    super::textbox::insert_ac(p, build_ink_ac(canvas_id, ink_id, &rid, cx, cy));
    dirty.insert(host, ());
    Ok(())
}

pub fn apply_replace(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    doc_part: &str,
    catalog: &[HfPart],
    op: &ReplaceInkCanvasOp,
    i: usize,
) -> Result<(), Error> {
    let host = part_for_target(
        catalog,
        doc_part,
        &op.target,
        &op.id,
        &format!("ops[{i}].id"),
    )?;
    let rid = {
        let doc = load_part(pkg, docs, &host)?;
        let ctx = HostCtx::load(pkg, &host)?;
        find_rid(&ctx, &doc.root, &op.target.key).ok_or_else(|| ink_missing(&op.id, i))?
    };
    let part = resolve_id(pkg, &host, &rid)?;
    let bytes = read_inkml(&op.file, i)?;
    pkg.set(&part, bytes)?;
    let mut cts = ContentTypes::from_package(pkg)?;
    cts.set_override(&part, CT_INKML)?;
    cts.write_to(pkg)?;
    Ok(())
}

pub fn apply_delete(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[HfPart],
    op: &DeleteInkCanvasOp,
    i: usize,
) -> Result<(), Error> {
    let host = part_for_target(
        catalog,
        doc_part,
        &op.target,
        &op.id,
        &format!("ops[{i}].id"),
    )?;
    let (rid, drop_part) = {
        let doc = load_part(pkg, docs, &host)?;
        let ctx = HostCtx::load(pkg, &host)?;
        let rid =
            find_rid(&ctx, &doc.root, &op.target.key).ok_or_else(|| ink_missing(&op.id, i))?;
        let part = resolve_id(pkg, &host, &rid).ok();
        (rid, part)
    };
    {
        let doc = load_part(pkg, docs, &host)?;
        let ctx = HostCtx::load(pkg, &host)?;
        if !remove_ink(&mut doc.root, &ctx, &op.target.key) {
            return Err(ink_missing(&op.id, i));
        }
    }
    dirty.insert(host.clone(), ());
    let root_ref = &docs.get(&host).unwrap().root;
    remove_rel_if_unused(pkg, &host, &rid, root_ref)?;
    if let Some(part) = drop_part {
        drop_unused(pkg, &part)?;
    }
    Ok(())
}

pub fn apply_set_size(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[HfPart],
    op: &SetInkCanvasSizeOp,
    i: usize,
) -> Result<(), Error> {
    let host = part_for_target(
        catalog,
        doc_part,
        &op.target,
        &op.id,
        &format!("ops[{i}].id"),
    )?;
    let ctx = HostCtx::load(pkg, &host)?;
    let doc = load_part(pkg, docs, &host)?;
    let drawing = find_drawing_mut(&mut doc.root, &ctx, &op.target.key)
        .ok_or_else(|| ink_missing(&op.id, i))?;
    if !picture::set_extent(drawing, op.cx, op.cy) {
        return Err(ink_missing(&op.id, i));
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
    op: &SetInkCanvasWrapOp,
    i: usize,
) -> Result<(), Error> {
    let host = part_for_target(
        catalog,
        doc_part,
        &op.target,
        &op.id,
        &format!("ops[{i}].id"),
    )?;
    let ctx = HostCtx::load(pkg, &host)?;
    let doc = load_part(pkg, docs, &host)?;
    ensure_ink_ns(&mut doc.root);
    let drawing = find_drawing_mut(&mut doc.root, &ctx, &op.target.key)
        .ok_or_else(|| ink_missing(&op.id, i))?;
    picture::apply_wrap(drawing, op.kind, op.wrap, &op.pos)
        .map_err(|e| e.at(format!("ops[{i}]")))?;
    dirty.insert(host, ());
    Ok(())
}

fn collect_in_drawing(
    ctx: &HostCtx<'_>,
    drawing: &Element,
    host_para_id: &str,
    index: &mut usize,
    out: &mut Vec<InkInfo>,
) {
    let kind = drawing_kind(drawing);
    let wrap = drawing_wrap(drawing, kind);
    let (cx, cy) = drawing_extent(drawing);
    let drawing_pr = doc_pr(drawing);
    walk_all(drawing, &mut |e| {
        if e.local_name() == "contentPart" && ctx.is_ink(e) {
            *index += 1;
            let cnv = cnv_pr(e);
            let token = cnv.0.clone().or(drawing_pr.0.clone());
            let (id, stable) = match token {
                Some(v) if !v.is_empty() => (format!("{host_para_id}/ink:{v}"), true),
                _ => (format!("{host_para_id}/ink:index:{index}"), false),
            };
            let name = cnv.1.or(drawing_pr.1.clone());
            out.push(InkInfo {
                id,
                name,
                kind,
                cx,
                cy,
                wrap,
                stable,
                hyperlink: super::drawing_hlink::inspect_picture(drawing, Some(&ctx.rels)),
            });
        }
    });
}

pub(crate) fn find_drawing_mut<'a>(
    root: &'a mut Element,
    ctx: &HostCtx<'_>,
    key: &TxbxKey,
) -> Option<&'a mut Element> {
    find_drawing_walk(root, ctx, key, &mut 0)
}

fn find_drawing_walk<'a>(
    el: &'a mut Element,
    ctx: &HostCtx<'_>,
    key: &TxbxKey,
    index: &mut usize,
) -> Option<&'a mut Element> {
    if el.local_name() == "txbxContent" {
        return None;
    }
    if el.local_name() == "AlternateContent" {
        if let Some(i) = first_choice(el) {
            if let Node::Element(child) = &mut el.children[i] {
                return find_drawing_walk(child, ctx, key, index);
            }
        }
        return None;
    }
    if el.local_name() == "drawing" {
        if drawing_has_ink_key(el, ctx, key, index) {
            return Some(el);
        }
        return None;
    }
    let children = el.children.as_mut_slice();
    for child in children {
        if let Node::Element(e) = child {
            if let Some(found) = find_drawing_walk(e, ctx, key, index) {
                return Some(found);
            }
        }
    }
    None
}

fn drawing_has_ink_key(
    drawing: &Element,
    ctx: &HostCtx<'_>,
    key: &TxbxKey,
    index: &mut usize,
) -> bool {
    let drawing_pr = doc_pr(drawing).0;
    let mut hit = false;
    walk_all(drawing, &mut |e| {
        if hit {
            return;
        }
        if e.local_name() == "contentPart" && ctx.is_ink(e) {
            *index += 1;
            let token = cnv_pr(e).0.or(drawing_pr.clone());
            if matches_key(&token, *index, key) {
                hit = true;
            }
        }
    });
    hit
}

fn find_rid(ctx: &HostCtx<'_>, root: &Element, key: &TxbxKey) -> Option<String> {
    let mut index = 0usize;
    let mut rid = None;
    walk_host(root, &mut |drawing| {
        if rid.is_some() {
            return Ok(());
        }
        let drawing_pr = doc_pr(drawing).0;
        walk_all(drawing, &mut |e| {
            if rid.is_some() {
                return;
            }
            if e.local_name() == "contentPart" && ctx.is_ink(e) {
                index += 1;
                let token = cnv_pr(e).0.or(drawing_pr.clone());
                if matches_key(&token, index, key) {
                    rid = e.attr_local("id").map(str::to_string);
                }
            }
        });
        Ok(())
    })
    .ok()?;
    rid
}

fn remove_ink(root: &mut Element, ctx: &HostCtx<'_>, key: &TxbxKey) -> bool {
    let mut index = 0usize;
    remove_walk(root, ctx, key, &mut index)
}

fn remove_walk(el: &mut Element, ctx: &HostCtx<'_>, key: &TxbxKey, index: &mut usize) -> bool {
    if el.local_name() == "txbxContent" {
        return false;
    }
    let mut i = 0;
    while i < el.children.len() {
        let local = match &el.children[i] {
            Node::Element(c) => c.local_name().to_string(),
            Node::Text(_) => {
                i += 1;
                continue;
            }
        };
        if local == "AlternateContent" {
            if strip_from_node(&mut el.children[i], ctx, key, index) {
                if node_wpc_empty(&el.children[i]) {
                    el.children.remove(i);
                }
                return true;
            }
            i += 1;
            continue;
        }
        if local == "drawing" {
            if strip_from_node(&mut el.children[i], ctx, key, index) {
                if node_wpc_empty(&el.children[i]) {
                    el.children.remove(i);
                }
                return true;
            }
            i += 1;
            continue;
        }
        if let Node::Element(child) = &mut el.children[i] {
            if remove_walk(child, ctx, key, index) {
                if is_empty_run(child) {
                    el.children.remove(i);
                }
                return true;
            }
        }
        i += 1;
    }
    false
}

fn strip_from_node(node: &mut Node, ctx: &HostCtx<'_>, key: &TxbxKey, index: &mut usize) -> bool {
    let Node::Element(el) = node else {
        return false;
    };
    if el.local_name() == "AlternateContent" {
        let Some(ci) = first_choice(el) else {
            return false;
        };
        if let Node::Element(branch) = &mut el.children[ci] {
            return strip_from_element(branch, ctx, key, index, None);
        }
        return false;
    }
    strip_from_element(el, ctx, key, index, None)
}

fn strip_from_element(
    el: &mut Element,
    ctx: &HostCtx<'_>,
    key: &TxbxKey,
    index: &mut usize,
    drawing_pr: Option<String>,
) -> bool {
    if el.local_name() == "txbxContent" {
        return false;
    }
    let drawing_pr = if el.local_name() == "drawing" {
        doc_pr(el).0
    } else {
        drawing_pr
    };
    let mut i = 0;
    while i < el.children.len() {
        let is_ink_part = matches!(
            &el.children[i],
            Node::Element(c) if c.local_name() == "contentPart" && ctx.is_ink(c)
        );
        if is_ink_part {
            *index += 1;
            let hit = match &el.children[i] {
                Node::Element(e) => {
                    let token = cnv_pr(e).0.or(drawing_pr.clone());
                    matches_key(&token, *index, key)
                }
                Node::Text(_) => false,
            };
            if hit {
                el.children.remove(i);
                return true;
            }
            i += 1;
            continue;
        }
        if let Node::Element(child) = &mut el.children[i] {
            if strip_from_element(child, ctx, key, index, drawing_pr.clone()) {
                return true;
            }
        }
        i += 1;
    }
    false
}

fn node_wpc_empty(node: &Node) -> bool {
    let Node::Element(el) = node else {
        return false;
    };
    wpc_empty(el)
}

fn wpc_empty(el: &Element) -> bool {
    if let Some(wpc) = find_wpc(el) {
        return !wpc.element_children().any(|c| {
            let n = c.local_name();
            n != "bg" && n != "whole"
        });
    }
    let mut any = false;
    walk_all(el, &mut |e| {
        if e.local_name() == "contentPart" {
            any = true;
        }
    });
    !any
}

fn find_wpc(el: &Element) -> Option<&Element> {
    if el.local_name() == "wpc" {
        return Some(el);
    }
    for n in &el.children {
        if let Node::Element(e) = n {
            if e.local_name() == "txbxContent" {
                continue;
            }
            if let Some(hit) = find_wpc(e) {
                return Some(hit);
            }
        }
    }
    None
}

fn build_ink_ac(canvas_id: u32, ink_id: u32, rid: &str, cx: i64, cy: i64) -> Element {
    let mut ac = Element::new("mc:AlternateContent");
    let mut choice = Element::new("mc:Choice").with_attr("Requires", "wpc");
    let mut drawing = Element::new("w:drawing");
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
            .with_attr("id", canvas_id.to_string())
            .with_attr("name", format!("墨迹画布 {canvas_id}")),
    );
    let mut frame_pr = Element::new("wp:cNvGraphicFramePr");
    frame_pr.push_element(Element::new("a:graphicFrameLocks").with_attr("noChangeAspect", "1"));
    inline.push_element(frame_pr);
    let mut graphic = Element::new("a:graphic");
    let mut data = Element::new("a:graphicData").with_attr("uri", WPC_NS);
    let mut wpc = Element::new("wpc:wpc");
    wpc.push_element(Element::new("wpc:bg"));
    wpc.push_element(Element::new("wpc:whole"));
    let mut part = Element::new("w14:contentPart")
        .with_attr("bwMode", "clr")
        .with_attr("r:id", rid);
    let mut nv = Element::new("w14:nvContentPartPr");
    nv.push_element(
        Element::new("w14:cNvPr")
            .with_attr("id", ink_id.to_string())
            .with_attr("name", format!("墨迹 {ink_id}")),
    );
    nv.push_element(Element::new("w14:cNvContentPartPr"));
    part.push_element(nv);
    let mut xfrm = Element::new("w14:xfrm");
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
    part.push_element(xfrm);
    wpc.push_element(part);
    data.push_element(wpc);
    graphic.push_element(data);
    inline.push_element(graphic);
    drawing.push_element(inline);
    choice.push_element(drawing);
    ac.push_element(choice);
    let mut fallback = Element::new("mc:Fallback");
    fallback.push_element(Element::new("w:pict"));
    ac.push_element(fallback);
    let _ = (MC_NS, WP_NS, A_NS, W14_NS, REL_NS);
    ac
}

fn ensure_ink_ns(root: &mut Element) {
    picture::ensure_drawing_ns(root);
    ensure_xmlns(root, "mc", MC_NS);
    ensure_xmlns(root, "wpc", WPC_NS);
    ensure_xmlns(root, "w14", W14_NS);
    ensure_xmlns(root, "wpi", WPI_NS);
    ensure_xmlns(root, "wp", WP_NS);
}

fn ensure_xmlns(el: &mut Element, prefix: &str, uri: &str) {
    let key = format!("xmlns:{prefix}");
    if el.attr(&key).is_none() {
        el.set_attr(&key, uri);
    }
}

fn read_inkml(path: &str, i: usize) -> Result<Vec<u8>, Error> {
    let bytes = crate::host_files::read_user_file(path)?;
    validate_inkml(&bytes, i)?;
    Ok(bytes)
}

fn validate_inkml(bytes: &[u8], i: usize) -> Result<(), Error> {
    let err = || Error::with_path(ErrorKind::InvalidInkMl, format!("ops[{i}].file"));
    let doc = xml::parse(bytes).map_err(|_| err())?;
    if !is_inkml_root(&doc.root) {
        return Err(err());
    }
    Ok(())
}

fn is_inkml_root(el: &Element) -> bool {
    if el.local_name() != "ink" {
        return false;
    }
    el.attrs
        .iter()
        .any(|(k, v)| (k == "xmlns" || k.starts_with("xmlns:")) && v == INKML_NS)
}

fn parse_ink_key<'a>(s: &'a str, path: &str) -> Result<(TxbxKey, Option<&'a str>), Error> {
    if let Some(rest) = s.strip_prefix("index:") {
        let (num, tail) = match rest.split_once('/') {
            Some((a, b)) => (a, Some(b)),
            None => (rest, None),
        };
        let n: usize = num.parse().map_err(|_| {
            Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::DocxInkIndexNotInt,
                },
                path.to_string(),
            )
        })?;
        if n < 1 {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::DocxInkIndexFromOne,
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
                reason: IdReason::DocxInkPrefix,
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

fn walk_host(el: &Element, f: &mut impl FnMut(&Element) -> Result<(), Error>) -> Result<(), Error> {
    if el.local_name() == "txbxContent" {
        return Ok(());
    }
    if el.local_name() == "AlternateContent" {
        if let Some(i) = first_choice(el) {
            if let Node::Element(child) = &el.children[i] {
                return walk_host(child, f);
            }
        }
        return Ok(());
    }
    if el.local_name() == "drawing" {
        return f(el);
    }
    for n in &el.children {
        if let Node::Element(e) = n {
            walk_host(e, f)?;
        }
    }
    Ok(())
}

fn walk_all<'a>(el: &'a Element, f: &mut impl FnMut(&'a Element)) {
    if el.local_name() == "txbxContent" {
        return;
    }
    f(el);
    for n in &el.children {
        if let Node::Element(e) = n {
            walk_all(e, f);
        }
    }
}

fn first_choice(ac: &Element) -> Option<usize> {
    ac.children
        .iter()
        .position(|n| matches!(n, Node::Element(e) if e.local_name() == "Choice"))
}

fn drawing_kind(drawing: &Element) -> PicKind {
    if drawing
        .element_children()
        .any(|e| e.local_name() == "anchor")
    {
        PicKind::Anchor
    } else {
        PicKind::Inline
    }
}

fn drawing_wrap(drawing: &Element, kind: PicKind) -> Option<WrapKind> {
    if kind != PicKind::Anchor {
        return None;
    }
    let frame = drawing
        .element_children()
        .find(|e| e.local_name() == "anchor")?;
    frame
        .element_children()
        .find_map(|c| WrapKind::from_local(c.local_name()))
}

fn drawing_extent(drawing: &Element) -> (Option<i64>, Option<i64>) {
    let frame = drawing
        .element_children()
        .find(|e| e.local_name() == "inline" || e.local_name() == "anchor");
    let extent = frame.and_then(|f| f.child_by_local("extent"));
    (
        extent
            .and_then(|e| e.attr_local("cx"))
            .and_then(|s| s.parse().ok()),
        extent
            .and_then(|e| e.attr_local("cy"))
            .and_then(|s| s.parse().ok()),
    )
}

fn doc_pr(drawing: &Element) -> (Option<String>, Option<String>) {
    let frame = drawing
        .element_children()
        .find(|e| e.local_name() == "inline" || e.local_name() == "anchor");
    let pr = frame.and_then(|f| f.child_by_local("docPr"));
    (
        pr.and_then(|p| p.attr_local("id")).map(str::to_string),
        pr.and_then(|p| p.attr_local("name"))
            .filter(|s| !s.is_empty())
            .map(str::to_string),
    )
}

fn cnv_pr(el: &Element) -> (Option<String>, Option<String>) {
    let mut id = None;
    let mut name = None;
    walk_all(el, &mut |e| {
        if e.local_name() == "cNvPr" && id.is_none() {
            id = e.attr_local("id").map(str::to_string);
            name = e
                .attr_local("name")
                .filter(|s| !s.is_empty())
                .map(str::to_string);
        }
    });
    (id, name)
}

fn is_empty_run(el: &Element) -> bool {
    el.local_name() == "r" && el.element_children().all(|c| c.local_name() == "rPr")
}

fn ink_missing(id: &str, i: usize) -> Error {
    Error::with_path(
        ErrorKind::InkNotFound { id: id.to_string() },
        format!("ops[{i}].id"),
    )
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
                    ErrorKind::InkNotFound { id: id.to_string() },
                    path.to_string(),
                )
            })?;
            Ok(item.part.clone())
        }
        HostPara::InNote { .. } | HostPara::InComment { .. } => Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::DocxInkPrefix,
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
    walk_all(root, &mut |e| {
        for (k, v) in &e.attrs {
            if v != rid {
                continue;
            }
            let local = k.rsplit(':').next().unwrap_or(k);
            let pfx = k.split(':').next();
            if pfx == Some("r") && matches!(local, "id" | "embed" | "link") {
                found = true;
            }
        }
    });
    found
}

fn remove_rel_if_unused(
    pkg: &mut Package,
    part: &str,
    rid: &str,
    root: &Element,
) -> Result<(), Error> {
    if rid_still_in(root, rid) {
        return Ok(());
    }
    let mut rels = Relationships::from_package(pkg, part)?;
    rels.remove_by_id(rid);
    if rels.rels.is_empty() {
        pkg.remove(&rels_part_name(part));
    } else {
        rels.write_to(pkg, part)?;
    }
    Ok(())
}

fn drop_unused(pkg: &mut Package, part: &str) -> Result<(), Error> {
    if picture::media_still_used(pkg, part)? {
        return Ok(());
    }
    let rels = rels_part_name(part);
    pkg.remove(part);
    pkg.remove(&rels);
    let mut cts = ContentTypes::from_package(pkg)?;
    cts.remove_override(part);
    cts.write_to(pkg)?;
    Ok(())
}
