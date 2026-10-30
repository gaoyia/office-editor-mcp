//! 正文与页眉页脚里的嵌入对象：收集、导出、插删、改预览框尺寸。
//! 不执行 OLE 服务器，不把嵌入字节塞进 inspect 树。

use std::collections::HashMap;

use serde_json::{json, Value};

use crate::error::{Error, ErrorKind, IdReason};
use crate::opc::{
    rels_part_name, resolve_id, resolve_rel_target, ContentTypes, Package, Relationship,
    Relationships,
};
use crate::xml::{self, Document, Element, Node};

use crate::ole::{self, FileSpec};

use super::hf::HfPart;
use super::para::{self, ParaTarget};
use super::picture;
use super::{REL_IMAGE, REL_NS, WP_NS};

pub const VML_NS: &str = "urn:schemas-microsoft-com:vml";
pub const OFFICE_NS: &str = "urn:schemas-microsoft-com:office:office";

#[derive(Debug, Clone)]
pub enum OleKey {
    Token(String),
    Index(usize),
}

#[derive(Debug, Clone)]
pub enum OleTarget {
    Body {
        para: para::BodyPara,
        key: OleKey,
    },
    HeaderFooter {
        kind: super::hf::HfKind,
        hf_type: super::hf::HfType,
        n: u32,
        key: OleKey,
    },
}

#[derive(Debug, Clone)]
pub struct InsertOleOp {
    pub id: String,
    pub para: ParaTarget,
    pub file: String,
}

#[derive(Debug, Clone)]
pub struct DeleteOleOp {
    pub id: String,
    pub target: OleTarget,
}

#[derive(Debug, Clone)]
pub struct SetOleSizeOp {
    pub id: String,
    pub target: OleTarget,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
}

struct HostCtx<'a> {
    host: &'a str,
    rels: Relationships,
    cts: ContentTypes,
}

pub fn count_in(pkg: &Package, host: &str, root: &Element) -> Result<usize, Error> {
    let ctx = HostCtx::load(pkg, host)?;
    let mut n = 0usize;
    walk_objects(root, &mut |obj| {
        if ctx.accepts(obj) {
            n += 1;
        }
        Ok(())
    })?;
    Ok(n)
}

pub fn collect_body(
    pkg: &Package,
    doc_part: &str,
    paras: &[&Element],
) -> Result<Vec<Value>, Error> {
    let ctx = HostCtx::load(pkg, doc_part)?;
    let mut out = Vec::new();
    for (i, p) in paras.iter().enumerate() {
        let (id, _) = para::para_stable_id(p, i + 1);
        let mut idx = 0usize;
        collect_into(&ctx, p, &id, &mut idx, &mut out)?;
    }
    Ok(out)
}

pub fn collect_hf(pkg: &Package, item: &HfPart, paras: &[&Element]) -> Result<Vec<Value>, Error> {
    let ctx = HostCtx::load(pkg, &item.part)?;
    let mut out = Vec::new();
    let mut idx = 0usize;
    for (i, p) in paras.iter().enumerate() {
        let (id, _) = para::para_stable_id_in(&item.id, p, i + 1);
        collect_into(&ctx, p, &id, &mut idx, &mut out)?;
    }
    Ok(out)
}

pub fn parse_ole_id(id: &str, op_index: usize) -> Result<OleTarget, Error> {
    parse_ole_id_at(id, &format!("ops[{op_index}].id"))
}

pub fn parse_ole_id_at(id: &str, path: &str) -> Result<OleTarget, Error> {
    let path = path.to_string();
    let Some((left, key_s)) = id.rsplit_once("/ole:") else {
        if looks_like_para_or_container(id) {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::OleContainer,
                },
                path,
            ));
        }
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::OlePrefix,
            },
            path,
        ));
    };
    let key = parse_ole_key(key_s, &path)?;
    match para::parse_para_id_at(left, &path)? {
        ParaTarget::Body(para) => Ok(OleTarget::Body { para, key }),
        ParaTarget::HeaderFooter {
            kind,
            hf_type,
            n,
            para: _,
        } => Ok(OleTarget::HeaderFooter {
            kind,
            hf_type,
            n,
            key,
        }),
        ParaTarget::InTextBox { .. } | ParaTarget::InNote { .. } | ParaTarget::InComment { .. } => {
            Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::OlePrefix,
                },
                path,
            ))
        }
    }
}

pub fn parse_insert_para_id(id: &str, op_index: usize) -> Result<ParaTarget, Error> {
    let path = format!("ops[{op_index}].id");
    if id.contains("/ole:")
        || id.contains("/pic:")
        || id.contains("/chart:")
        || id.contains("/series:")
        || id.contains("/smartdiagram:")
        || id.contains("/pt:")
        || id.contains("/txbx:")
        || id.starts_with("footnote:")
        || id.starts_with("endnote:")
        || id.starts_with("comment:")
        || id.starts_with("section:")
    {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::InsertOleNeedsPara,
            },
            path,
        ));
    }
    let target = para::parse_para_id_at(id, &path)?;
    match target {
        ParaTarget::Body(_) | ParaTarget::HeaderFooter { .. } => Ok(target),
        ParaTarget::InTextBox { .. } | ParaTarget::InNote { .. } | ParaTarget::InComment { .. } => {
            Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::InsertOleNeedsPara,
                },
                path,
            ))
        }
    }
}

pub fn export_media(bytes: &[u8], id: &str) -> Result<super::export::ExportedMedia, Error> {
    let pkg = Package::open_bytes(bytes.to_vec())?;
    let target = parse_ole_id_at(id, "id")?;
    let doc_part = crate::opc::office_document_part(&pkg)?;
    let doc_xml = xml::parse(pkg.get(&doc_part)?)?;
    let catalog = super::hf::catalog(&pkg, &doc_part, &doc_xml.root)?;
    let host = part_for_target(&pkg, &catalog, &doc_part, &target, id, "id")?;
    let xml_doc = if host == doc_part {
        doc_xml
    } else {
        xml::parse(pkg.get(&host)?)?
    };
    let obj = find_object(&xml_doc.root, &target)
        .ok_or_else(|| Error::with_path(ErrorKind::OleNotFound { id: id.to_string() }, "id"))?;
    let rid = ole_rid(obj).ok_or_else(|| {
        Error::with_path(
            ErrorKind::PartMissing {
                name: format!("{id} oleObject"),
            },
            "id",
        )
    })?;
    let part = resolve_id(&pkg, &host, rid)?;
    let embed_bytes = pkg.get(&part)?.to_vec();
    let cts = ContentTypes::from_package(&pkg)?;
    let content_type = cts.content_type_of(&part).map(str::to_string);
    Ok(super::export::ExportedMedia {
        bytes: embed_bytes,
        content_type,
    })
}

pub fn apply_insert(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[HfPart],
    op: &InsertOleOp,
    i: usize,
) -> Result<(), Error> {
    let host = part_for_para(catalog, doc_part, &op.para, &op.id, i)?;
    {
        let doc = load_part(pkg, docs, &host)?;
        if find_para_mut(&mut doc.root, &op.para).is_none() {
            return Err(Error::with_path(
                ErrorKind::ParaNotFound { id: op.id.clone() },
                format!("ops[{i}].id"),
            ));
        }
    }
    let spec = ole::classify(&op.file);
    let bytes = crate::host_files::read_user_file(&op.file)?;
    let payload = match &spec {
        FileSpec::Office { .. } => bytes,
        FileSpec::Package => ole::wrap_package(&op.file, &bytes),
    };
    let embed_part = ole::next_numbered(pkg, "/word/embeddings", spec.stem(), spec.ext());
    pkg.set(&embed_part, payload)?;
    let png_part = picture::next_media_part(pkg, "png");
    pkg.set(&png_part, ole::PREVIEW_PNG.to_vec())?;

    let mut host_rels = Relationships::from_package(pkg, &host)?;
    let ole_rid = host_rels.next_rid();
    host_rels.push(Relationship {
        id: ole_rid.clone(),
        rel_type: spec.rel_type().into(),
        target: picture::rel_target_from(&host, &embed_part),
        target_mode: None,
        extra: Vec::new(),
    });
    let img_rid = host_rels.next_rid();
    host_rels.push(Relationship {
        id: img_rid.clone(),
        rel_type: REL_IMAGE.into(),
        target: picture::rel_target_from(&host, &png_part),
        target_mode: None,
        extra: Vec::new(),
    });
    host_rels.write_to(pkg, &host)?;

    let mut cts = ContentTypes::from_package(pkg)?;
    cts.ensure_default("png", "image/png");
    match &spec {
        FileSpec::Office { content_type, .. } => {
            cts.set_override(&embed_part, content_type)?;
        }
        FileSpec::Package => {
            cts.ensure_default("bin", ole::CT_OLE);
            cts.set_override(&embed_part, ole::CT_OLE)?;
        }
    }
    cts.write_to(pkg)?;

    let doc = load_part(pkg, docs, &host)?;
    ensure_ole_ns(&mut doc.root);
    let object_id = next_object_id(&doc.root);
    let shape_n = next_shape_i(&doc.root);
    let p = find_para_mut(&mut doc.root, &op.para).ok_or_else(|| {
        Error::with_path(
            ErrorKind::ParaNotFound { id: op.id.clone() },
            format!("ops[{i}].id"),
        )
    })?;
    let object = build_object(spec.prog_id(), &ole_rid, &img_rid, object_id, shape_n);
    insert_object_into_para(p, object);
    dirty.insert(host, ());
    Ok(())
}

pub fn apply_delete(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[HfPart],
    op: &DeleteOleOp,
    i: usize,
) -> Result<(), Error> {
    let host = part_for_target(
        pkg,
        catalog,
        doc_part,
        &op.target,
        &op.id,
        &format!("ops[{i}].id"),
    )?;
    let (ole_rid, img_rids, drop_parts) = {
        let doc = load_part(pkg, docs, &host)?;
        let obj = find_object(&doc.root, &op.target).ok_or_else(|| {
            Error::with_path(
                ErrorKind::OleNotFound { id: op.id.clone() },
                format!("ops[{i}].id"),
            )
        })?;
        let ole_rid = ole_rid(obj).unwrap_or("").to_string();
        let img_rids = image_rids(obj);
        let mut parts = Vec::new();
        if !ole_rid.is_empty() {
            if let Ok(p) = resolve_id(pkg, &host, &ole_rid) {
                parts.push(p);
            }
        }
        for r in &img_rids {
            if let Ok(p) = resolve_id(pkg, &host, r) {
                parts.push(p);
            }
        }
        (ole_rid, img_rids, parts)
    };
    {
        let doc = load_part(pkg, docs, &host)?;
        if !remove_object(&mut doc.root, &op.target) {
            return Err(Error::with_path(
                ErrorKind::OleNotFound { id: op.id.clone() },
                format!("ops[{i}].id"),
            ));
        }
    }
    dirty.insert(host.clone(), ());
    let root_ref = &docs.get(&host).unwrap().root;
    if !ole_rid.is_empty() {
        remove_rel_if_unused(pkg, &host, &ole_rid, root_ref)?;
    }
    for r in img_rids {
        remove_rel_if_unused(pkg, &host, &r, root_ref)?;
    }
    for part in drop_parts {
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
    op: &SetOleSizeOp,
    i: usize,
) -> Result<(), Error> {
    let host = part_for_target(
        pkg,
        catalog,
        doc_part,
        &op.target,
        &op.id,
        &format!("ops[{i}].id"),
    )?;
    let doc = load_part(pkg, docs, &host)?;
    if !patch_object_size(&mut doc.root, &op.target, op.cx, op.cy) {
        return Err(Error::with_path(
            ErrorKind::OleNotFound { id: op.id.clone() },
            format!("ops[{i}].id"),
        ));
    }
    dirty.insert(host, ());
    Ok(())
}

impl HostCtx<'_> {
    fn load<'a>(pkg: &'a Package, host: &'a str) -> Result<HostCtx<'a>, Error> {
        Ok(HostCtx {
            host,
            rels: Relationships::from_package(pkg, host)?,
            cts: ContentTypes::from_package(pkg)?,
        })
    }

    fn accepts(&self, obj: &Element) -> bool {
        if has_local(obj, "control") || has_local(obj, "movie") {
            return false;
        }
        if has_local(obj, "objectLink") && embed_elem(obj).is_none() {
            return false;
        }
        let Some(rid) = ole_rid(obj) else {
            return false;
        };
        let Some(rel) = self.rels.find_by_id(rid) else {
            return false;
        };
        !rel.is_external() && is_embed_rel(&rel.rel_type)
    }

    fn item_json(&self, obj: &Element, para_id: &str, index: usize) -> Value {
        let (token, stable) = object_token(obj, index);
        let prog = prog_id(obj);
        let kind = if prog
            .as_deref()
            .is_some_and(|p| p.eq_ignore_ascii_case("Package"))
        {
            "package"
        } else {
            "embed"
        };
        let name = find_desc(obj, "docPr")
            .and_then(|d| d.attr_local("name"))
            .filter(|s| !s.is_empty());
        let content_type = ole_rid(obj).and_then(|rid| {
            let rel = self.rels.find_by_id(rid)?;
            if rel.is_external() {
                return None;
            }
            let part = resolve_rel_target(self.host, &rel.target).ok()?;
            self.cts.content_type_of(&part).map(str::to_string)
        });
        let (cx, cy) = object_size(obj);
        json!({
            "id": format!("{para_id}/ole:{token}"),
            "progId": prog,
            "name": name,
            "kind": kind,
            "contentType": content_type,
            "cx": cx,
            "cy": cy,
            "stable": stable,
        })
    }
}

fn collect_into(
    ctx: &HostCtx<'_>,
    root: &Element,
    para_id: &str,
    idx: &mut usize,
    out: &mut Vec<Value>,
) -> Result<(), Error> {
    walk_objects(root, &mut |obj| {
        if ctx.accepts(obj) {
            *idx += 1;
            out.push(ctx.item_json(obj, para_id, *idx));
        }
        Ok(())
    })
}

fn looks_like_para_or_container(id: &str) -> bool {
    id.starts_with("para:")
        || id.starts_with("header:")
        || id.starts_with("footer:")
        || id.starts_with("footnote:")
        || id.starts_with("endnote:")
        || id.starts_with("comment:")
}

fn parse_ole_key(rest: &str, path: &str) -> Result<OleKey, Error> {
    if let Some(n) = rest.strip_prefix("index:") {
        let idx: usize = n.parse().map_err(|_| {
            Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::OleIndexNotInt,
                },
                path.to_string(),
            )
        })?;
        if idx < 1 {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::OleIndexFromOne,
                },
                path.to_string(),
            ));
        }
        return Ok(OleKey::Index(idx));
    }
    if rest.is_empty() {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::OlePrefix,
            },
            path.to_string(),
        ));
    }
    Ok(OleKey::Token(rest.to_string()))
}

fn object_token(obj: &Element, index: usize) -> (String, bool) {
    if let Some(id) = embed_elem(obj)
        .and_then(|e| e.attr_local("ObjectID"))
        .filter(|s| !s.is_empty())
    {
        return (id.to_string(), true);
    }
    if let Some(id) = find_desc(obj, "objectEmbed")
        .and_then(|e| e.attr_local("shapeId"))
        .filter(|s| !s.is_empty())
    {
        return (id.to_string(), true);
    }
    if let Some(id) = find_desc(obj, "docPr")
        .and_then(|d| d.attr_local("id"))
        .filter(|s| !s.is_empty())
    {
        return (id.to_string(), true);
    }
    (format!("index:{index}"), false)
}

fn token_eq(a: &str, b: &str) -> bool {
    if let (Ok(x), Ok(y)) = (a.parse::<i64>(), b.parse::<i64>()) {
        x == y
    } else {
        a == b
    }
}

fn matches_key(obj: &Element, key: &OleKey, index: usize) -> bool {
    match key {
        OleKey::Index(n) => index == *n,
        OleKey::Token(want) => {
            let (got, _) = object_token(obj, index);
            token_eq(want, &got)
        }
    }
}

fn prog_id(obj: &Element) -> Option<String> {
    embed_elem(obj)
        .and_then(|e| e.attr_local("ProgID").or_else(|| e.attr_local("progId")))
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .or_else(|| {
            find_desc(obj, "objectEmbed")
                .and_then(|e| e.attr_local("progId"))
                .filter(|s| !s.is_empty())
                .map(str::to_string)
        })
}

fn embed_elem(obj: &Element) -> Option<&Element> {
    crate::xml::search(obj, |el| {
        if el.local_name() == "OLEObject" && !is_link_type(el) {
            return crate::xml::Search::Hit(el);
        }
        crate::xml::Search::down(el)
    })
}

fn is_link_type(el: &Element) -> bool {
    el.attr_local("Type")
        .is_some_and(|t| t.eq_ignore_ascii_case("link"))
}

fn is_listed_markup(obj: &Element) -> bool {
    if has_local(obj, "control") || has_local(obj, "movie") {
        return false;
    }
    ole_rid(obj).is_some()
}

fn is_embed_rel(rel_type: &str) -> bool {
    rel_type.ends_with("/oleObject") || rel_type.ends_with("/package")
}

fn ole_rid(obj: &Element) -> Option<&str> {
    if let Some(e) = embed_elem(obj) {
        if let Some(id) = e.attr_local("id").filter(|s| !s.is_empty()) {
            return Some(id);
        }
    }
    find_desc(obj, "objectEmbed")
        .and_then(|e| e.attr_local("id"))
        .filter(|s| !s.is_empty())
}

fn image_rids(obj: &Element) -> Vec<String> {
    let mut out = Vec::new();
    walk_all(obj, &mut |e| {
        if e.local_name() == "imagedata" {
            if let Some(id) = e.attr_local("id").filter(|s| !s.is_empty()) {
                out.push(id.to_string());
            }
        }
        if e.local_name() == "blip" {
            if let Some(id) = e.attr_local("embed").filter(|s| !s.is_empty()) {
                out.push(id.to_string());
            }
        }
    });
    out
}

fn has_local(el: &Element, local: &str) -> bool {
    crate::xml::find_by_local(el, local).is_some()
}

fn find_desc<'a>(el: &'a Element, local: &str) -> Option<&'a Element> {
    crate::xml::find_by_local(el, local)
}

fn walk_all(el: &Element, f: &mut impl FnMut(&Element)) {
    crate::xml::walk_elements(el, f);
}

fn walk_objects(
    root: &Element,
    f: &mut impl FnMut(&Element) -> Result<(), Error>,
) -> Result<(), Error> {
    let mut stack = vec![root];
    while let Some(el) = stack.pop() {
        if el.local_name() == "txbxContent" {
            continue;
        }
        if el.local_name() == "AlternateContent" {
            if let Some(i) = para::first_alt_child_index(el) {
                if let Node::Element(child) = &el.children[i] {
                    stack.push(child);
                }
            }
            continue;
        }
        if el.local_name() == "object" {
            f(el)?;
            continue;
        }
        for node in el.children.iter().rev() {
            if let Node::Element(child) = node {
                stack.push(child);
            }
        }
    }
    Ok(())
}

fn find_object<'a>(root: &'a Element, target: &OleTarget) -> Option<&'a Element> {
    match target {
        OleTarget::Body { para, key } => {
            let p = para::find_paragraph(root, para)?;
            find_object_walk(p, &mut 0, key)
        }
        OleTarget::HeaderFooter { key, .. } => find_object_walk(root, &mut 0, key),
    }
}

fn find_object_walk<'a>(el: &'a Element, idx: &mut usize, key: &OleKey) -> Option<&'a Element> {
    crate::xml::search(el, |el| {
        if el.local_name() == "txbxContent" {
            return crate::xml::Search::Skip;
        }
        if el.local_name() == "AlternateContent" {
            if let Some(i) = para::first_alt_child_index(el) {
                if let Node::Element(e) = &el.children[i] {
                    return crate::xml::Search::Only(e);
                }
            }
            return crate::xml::Search::Skip;
        }
        if el.local_name() == "object" {
            if is_listed_markup(el) {
                *idx += 1;
                if matches_key(el, key, *idx) {
                    return crate::xml::Search::Hit(el);
                }
            }
            return crate::xml::Search::Skip;
        }
        crate::xml::Search::down(el)
    })
}

fn patch_object_size(
    root: &mut Element,
    target: &OleTarget,
    cx: Option<i64>,
    cy: Option<i64>,
) -> bool {
    match target {
        OleTarget::Body { para, key } => match para::find_paragraph_mut(root, para) {
            Some(p) => patch_object_walk(p, &mut 0, key, cx, cy),
            None => false,
        },
        OleTarget::HeaderFooter { key, .. } => patch_object_walk(root, &mut 0, key, cx, cy),
    }
}

fn patch_object_walk(
    el: &mut Element,
    idx: &mut usize,
    key: &OleKey,
    cx: Option<i64>,
    cy: Option<i64>,
) -> bool {
    if el.local_name() == "txbxContent" {
        return false;
    }
    if el.local_name() == "AlternateContent" {
        if let Some(i) = para::first_alt_child_index(el) {
            if let Node::Element(e) = &mut el.children[i] {
                return patch_object_walk(e, idx, key, cx, cy);
            }
        }
        return false;
    }
    if el.local_name() == "object" {
        if is_listed_markup(el) {
            *idx += 1;
            if matches_key(el, key, *idx) {
                apply_object_size(el, cx, cy);
                return true;
            }
        }
        return false;
    }
    let n = el.children.len();
    for i in 0..n {
        if let Node::Element(e) = &mut el.children[i] {
            if patch_object_walk(e, idx, key, cx, cy) {
                return true;
            }
        }
    }
    false
}

/// 预览框尺寸。优先 `wp:extent`，其次 VML `style`，再次 `dxaOrig` / `dyaOrig`。
fn object_size(obj: &Element) -> (Option<i64>, Option<i64>) {
    let mut extent = None;
    let mut style = None;
    walk_all(obj, &mut |e| {
        if extent.is_none() && e.local_name() == "extent" {
            let cx = e.attr_local("cx").and_then(|s| s.parse().ok());
            let cy = e.attr_local("cy").and_then(|s| s.parse().ok());
            if cx.is_some() || cy.is_some() {
                extent = Some((cx, cy));
            }
        }
        if style.is_none() && e.local_name() == "shape" {
            if let Some(s) = e.attr_local("style") {
                let cx = style_pt_emu(s, "width");
                let cy = style_pt_emu(s, "height");
                if cx.is_some() || cy.is_some() {
                    style = Some((cx, cy));
                }
            }
        }
    });
    if let Some(v) = extent {
        return v;
    }
    if let Some(v) = style {
        return v;
    }
    let cx = obj
        .attr_local("dxaOrig")
        .and_then(|s| s.parse::<i64>().ok())
        .map(|t| t.saturating_mul(635));
    let cy = obj
        .attr_local("dyaOrig")
        .and_then(|s| s.parse::<i64>().ok())
        .map(|t| t.saturating_mul(635));
    (cx, cy)
}

fn apply_object_size(obj: &mut Element, cx: Option<i64>, cy: Option<i64>) {
    let (cur_x, cur_y) = object_size(obj);
    let nx = cx.or(cur_x);
    let ny = cy.or(cur_y);
    let mut wrote = false;
    walk_mut(obj, &mut |e| {
        if e.local_name() == "extent" || (e.local_name() == "ext" && e.attr_local("cx").is_some()) {
            if let Some(v) = nx {
                set_local_attr(e, "cx", &v.to_string());
                wrote = true;
            }
            if let Some(v) = ny {
                set_local_attr(e, "cy", &v.to_string());
                wrote = true;
            }
        }
        if e.local_name() == "shape" {
            if let Some(style) = e.attr_local("style").map(|s| s.to_string()) {
                let next = rewrite_style(&style, nx, ny);
                set_local_attr(e, "style", &next);
                wrote = true;
            }
        }
        if e.local_name() == "object" {
            if let Some(v) = nx {
                if e.attr_local("dxaOrig").is_some() {
                    set_local_attr(e, "dxaOrig", &emu_to_twip(v).to_string());
                    wrote = true;
                }
            }
            if let Some(v) = ny {
                if e.attr_local("dyaOrig").is_some() {
                    set_local_attr(e, "dyaOrig", &emu_to_twip(v).to_string());
                    wrote = true;
                }
            }
        }
    });
    if !wrote {
        if let Some(v) = nx {
            set_local_attr(obj, "dxaOrig", &emu_to_twip(v).to_string());
        }
        if let Some(v) = ny {
            set_local_attr(obj, "dyaOrig", &emu_to_twip(v).to_string());
        }
    }
}

fn walk_mut(root: &mut Element, f: &mut impl FnMut(&mut Element)) {
    let mut stack = vec![Vec::<usize>::new()];
    while let Some(path) = stack.pop() {
        let indexes: Vec<usize> = {
            let el = crate::xml::element_at_mut(root, &path);
            f(el);
            el.children
                .iter()
                .enumerate()
                .filter_map(|(index, node)| matches!(node, Node::Element(_)).then_some(index))
                .collect()
        };
        for index in indexes.into_iter().rev() {
            let mut child = path.clone();
            child.push(index);
            stack.push(child);
        }
    }
}

fn set_local_attr(el: &mut Element, local: &str, value: &str) {
    let name = el
        .attrs
        .iter()
        .find(|(k, _)| k.rsplit(':').next().unwrap_or(k) == local)
        .map(|(k, _)| k.clone())
        .unwrap_or_else(|| local.to_string());
    el.set_attr(&name, value);
}

fn style_pt_emu(style: &str, key: &str) -> Option<i64> {
    let prefix = format!("{key}:");
    for part in style.split(';') {
        let part = part.trim();
        if part.len() >= prefix.len() && part[..prefix.len()].eq_ignore_ascii_case(&prefix) {
            let raw = part[prefix.len()..].trim();
            let raw = raw.strip_suffix("pt").unwrap_or(raw).trim();
            let pt: f64 = raw.parse().ok()?;
            return Some((pt * 12_700.0).round() as i64);
        }
    }
    None
}

fn rewrite_style(style: &str, cx: Option<i64>, cy: Option<i64>) -> String {
    let mut parts: Vec<String> = style
        .split(';')
        .filter(|p| !p.trim().is_empty())
        .map(|p| p.trim().to_string())
        .collect();
    let mut saw_w = false;
    let mut saw_h = false;
    for p in &mut parts {
        let lower = p.to_ascii_lowercase();
        if let Some(v) = cx {
            if lower.starts_with("width:") {
                *p = format!("width:{}", crate::ole::format_pt(v));
                saw_w = true;
            }
        }
        if let Some(v) = cy {
            if lower.starts_with("height:") {
                *p = format!("height:{}", crate::ole::format_pt(v));
                saw_h = true;
            }
        }
    }
    if let Some(v) = cx {
        if !saw_w {
            parts.push(format!("width:{}", crate::ole::format_pt(v)));
        }
    }
    if let Some(v) = cy {
        if !saw_h {
            parts.push(format!("height:{}", crate::ole::format_pt(v)));
        }
    }
    parts.join(";")
}

fn emu_to_twip(emu: i64) -> i64 {
    (emu as f64 / 635.0).round() as i64
}

fn remove_object(root: &mut Element, target: &OleTarget) -> bool {
    match target {
        OleTarget::Body { para, key } => match para::find_paragraph_mut(root, para) {
            Some(p) => remove_object_walk(p, &mut 0, key),
            None => false,
        },
        OleTarget::HeaderFooter { key, .. } => remove_object_walk(root, &mut 0, key),
    }
}

fn remove_object_walk(el: &mut Element, idx: &mut usize, key: &OleKey) -> bool {
    if el.local_name() == "txbxContent" {
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
            if ac_has_object(&el.children[i], idx, key) {
                el.children.remove(i);
                return true;
            }
            i += 1;
            continue;
        }
        if local == "object" {
            let listed = match &el.children[i] {
                Node::Element(c) => is_listed_markup(c),
                Node::Text(_) | Node::Raw(_) => false,
            };
            if listed {
                *idx += 1;
                let hit = match &el.children[i] {
                    Node::Element(c) => matches_key(c, key, *idx),
                    Node::Text(_) | Node::Raw(_) => false,
                };
                if hit {
                    el.children.remove(i);
                    return true;
                }
            }
            i += 1;
            continue;
        }
        if local == "txbxContent" {
            i += 1;
            continue;
        }
        if let Node::Element(child) = &mut el.children[i] {
            if remove_object_walk(child, idx, key) {
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

fn ac_has_object(node: &Node, idx: &mut usize, key: &OleKey) -> bool {
    let Node::Element(ac) = node else {
        return false;
    };
    let Some(i) = para::first_alt_child_index(ac) else {
        return false;
    };
    let Node::Element(branch) = &ac.children[i] else {
        return false;
    };
    find_object_walk(branch, idx, key).is_some()
}

fn is_empty_run(el: &Element) -> bool {
    el.local_name() == "r" && el.element_children().all(|c| c.local_name() == "rPr")
}

fn find_para_mut<'a>(root: &'a mut Element, target: &ParaTarget) -> Option<&'a mut Element> {
    match target {
        ParaTarget::Body(p) | ParaTarget::HeaderFooter { para: p, .. } => {
            para::find_paragraph_mut(root, p)
        }
        ParaTarget::InTextBox { .. } | ParaTarget::InNote { .. } | ParaTarget::InComment { .. } => {
            None
        }
    }
}

fn part_for_para(
    catalog: &[HfPart],
    doc_part: &str,
    para: &ParaTarget,
    id: &str,
    i: usize,
) -> Result<String, Error> {
    match para {
        ParaTarget::Body(_) => Ok(doc_part.to_string()),
        ParaTarget::HeaderFooter {
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
        ParaTarget::InTextBox { .. } | ParaTarget::InNote { .. } | ParaTarget::InComment { .. } => {
            Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::InsertOleNeedsPara,
                },
                format!("ops[{i}].id"),
            ))
        }
    }
}

fn part_for_target(
    _pkg: &Package,
    catalog: &[HfPart],
    doc_part: &str,
    target: &OleTarget,
    id: &str,
    path: &str,
) -> Result<String, Error> {
    match target {
        OleTarget::Body { .. } => Ok(doc_part.to_string()),
        OleTarget::HeaderFooter {
            kind, hf_type, n, ..
        } => {
            let container_id = format!("{}:{}:{n}", kind.as_str(), hf_type.as_str());
            let item = super::hf::find_by_id(catalog, &container_id).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::OleNotFound { id: id.to_string() },
                    path.to_string(),
                )
            })?;
            Ok(item.part.clone())
        }
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

fn ensure_ole_ns(root: &mut Element) {
    ensure_xmlns(root, "r", REL_NS);
    ensure_xmlns(root, "v", VML_NS);
    ensure_xmlns(root, "o", OFFICE_NS);
    ensure_xmlns(root, "wp", WP_NS);
}

fn ensure_xmlns(el: &mut Element, prefix: &str, uri: &str) {
    let key = format!("xmlns:{prefix}");
    if el.attr(&key).is_none() {
        el.set_attr(&key, uri);
    }
}

fn insert_object_into_para(p: &mut Element, object: Element) {
    let mut run = Element::new(p.prefixed("r"));
    run.push_element(object);
    p.children.push(Node::Element(run));
}

fn next_object_id(root: &Element) -> u32 {
    let mut max = 0u32;
    walk_all(root, &mut |e| {
        if let Some(id) = e.attr_local("ObjectID") {
            let n = id.strip_prefix('_').unwrap_or(id);
            if let Ok(v) = n.parse::<u32>() {
                max = max.max(v);
            }
        }
    });
    max + 1
}

fn next_shape_i(root: &Element) -> u32 {
    let mut max = 1024u32;
    walk_all(root, &mut |e| {
        if let Some(id) = e.attr_local("id") {
            if let Some(rest) = id.strip_prefix("_x0000_i") {
                if let Ok(v) = rest.parse::<u32>() {
                    max = max.max(v);
                }
            }
        }
    });
    max + 1
}

fn build_object(
    prog_id: &str,
    ole_rid: &str,
    img_rid: &str,
    object_id: u32,
    shape_n: u32,
) -> Element {
    let shape_id = format!("_x0000_i{shape_n}");
    let mut object = Element::new("w:object")
        .with_attr("w:dxaOrig", "1440")
        .with_attr("w:dyaOrig", "1440");
    object.push_element(shapetype_75());
    let mut shape = Element::new("v:shape")
        .with_attr("id", &shape_id)
        .with_attr("type", "#_x0000_t75")
        .with_attr("style", "width:72pt;height:72pt")
        .with_attr("o:ole", "");
    shape.push_element(
        Element::new("v:imagedata")
            .with_attr("r:id", img_rid)
            .with_attr("o:title", ""),
    );
    object.push_element(shape);
    object.push_element(
        Element::new("o:OLEObject")
            .with_attr("Type", "Embed")
            .with_attr("ProgID", prog_id)
            .with_attr("ShapeID", &shape_id)
            .with_attr("DrawAspect", "Content")
            .with_attr("ObjectID", format!("_{object_id}"))
            .with_attr("r:id", ole_rid),
    );
    object
}

fn shapetype_75() -> Element {
    let mut st = Element::new("v:shapetype")
        .with_attr("id", "_x0000_t75")
        .with_attr("coordsize", "21600,21600")
        .with_attr("o:spt", "75")
        .with_attr("o:preferrelative", "t")
        .with_attr("path", "m@4@5l@4@11@9@11@9@5xe")
        .with_attr("filled", "f")
        .with_attr("stroked", "f");
    st.push_element(Element::new("v:stroke").with_attr("joinstyle", "miter"));
    let mut formulas = Element::new("v:formulas");
    for eqn in [
        "if lineDrawn pixelLineWidth 0",
        "sum @0 1 0",
        "sum 0 0 @1",
        "prod @2 1 2",
        "prod @3 21600 pixelWidth",
        "prod @3 21600 pixelHeight",
        "sum @0 0 1",
        "prod @6 1 2",
        "prod @7 21600 pixelWidth",
        "sum @8 21600 0",
        "prod @7 21600 pixelHeight",
        "sum @10 21600 0",
    ] {
        formulas.push_element(Element::new("v:f").with_attr("eqn", eqn));
    }
    st.push_element(formulas);
    st.push_element(
        Element::new("v:path")
            .with_attr("o:extrusionok", "f")
            .with_attr("gradientshapeok", "t")
            .with_attr("o:connecttype", "rect"),
    );
    st.push_element(
        Element::new("o:lock")
            .with_attr("v:ext", "edit")
            .with_attr("aspectratio", "t"),
    );
    st
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
