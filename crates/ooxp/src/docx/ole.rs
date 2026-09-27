//! 正文与页眉页脚里的嵌入对象：收集、导出、插删。
//! 不执行 OLE 服务器，不把嵌入字节塞进 inspect 树。

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::error::{io_path, Error, ErrorKind, IdReason};
use crate::opc::{
    rels_part_name, resolve_id, resolve_rel_target, ContentTypes, Package, Relationship,
    Relationships,
};
use crate::xml::{self, Document, Element, Node};

use super::hf::HfPart;
use super::para::{self, ParaTarget};
use super::picture;
use super::{CT_OLE, REL_IMAGE, REL_NS, REL_OLE, WP_NS};

pub const VML_NS: &str = "urn:schemas-microsoft-com:vml";
pub const OFFICE_NS: &str = "urn:schemas-microsoft-com:office:office";

const REL_PACKAGE: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/package";

const CT_XLSX: &str = "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet";
const CT_XLSM: &str = "application/vnd.ms-excel.sheet.macroEnabled.12";
const CT_XLS: &str = "application/vnd.ms-excel";
const CT_DOCX: &str = "application/vnd.openxmlformats-officedocument.wordprocessingml.document";
const CT_DOCM: &str = "application/vnd.ms-word.document.macroEnabled.12";
const CT_PPTX: &str = "application/vnd.openxmlformats-officedocument.presentationml.presentation";
const CT_PPTM: &str = "application/vnd.ms-powerpoint.presentation.macroEnabled.12";

/// 1×1 PNG，插入时当预览占位，不画图标。
const PNG_1X1: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
    0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08, 0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00,
    0x00, 0x03, 0x01, 0x01, 0x00, 0x18, 0xDD, 0x8D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E,
    0x44, 0xAE, 0x42, 0x60, 0x82,
];

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
        /// 解析自 id 左侧段落，查找按整部件计。
        #[allow(dead_code)]
        para: para::BodyPara,
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
            para,
        } => Ok(OleTarget::HeaderFooter {
            kind,
            hf_type,
            n,
            para,
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
    let spec = classify_file(&op.file);
    let src = PathBuf::from(&op.file);
    let bytes = std::fs::read(&src).map_err(|e| io_path(e, src.clone()))?;
    let payload = match &spec {
        FileSpec::Office { .. } => bytes,
        FileSpec::Package => wrap_package(&op.file, &bytes),
    };
    let embed_part = next_numbered(pkg, "/word/embeddings", spec.stem(), spec.ext());
    pkg.set(&embed_part, payload)?;
    let png_part = picture::next_media_part(pkg, "png");
    pkg.set(&png_part, PNG_1X1.to_vec())?;

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
            cts.ensure_default("bin", CT_OLE);
            cts.set_override(&embed_part, CT_OLE)?;
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
        json!({
            "id": format!("{para_id}/ole:{token}"),
            "progId": prog,
            "name": name,
            "kind": kind,
            "contentType": content_type,
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
    fn rec(el: &Element) -> Option<&Element> {
        if el.local_name() == "OLEObject" && !is_link_type(el) {
            return Some(el);
        }
        for c in el.element_children() {
            if let Some(f) = rec(c) {
                return Some(f);
            }
        }
        None
    }
    rec(obj)
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
    if el.local_name() == local {
        return true;
    }
    el.element_children().any(|c| has_local(c, local))
}

fn find_desc<'a>(el: &'a Element, local: &str) -> Option<&'a Element> {
    if el.local_name() == local {
        return Some(el);
    }
    for c in el.element_children() {
        if let Some(f) = find_desc(c, local) {
            return Some(f);
        }
    }
    None
}

fn walk_all(el: &Element, f: &mut impl FnMut(&Element)) {
    f(el);
    for n in &el.children {
        if let Node::Element(e) = n {
            walk_all(e, f);
        }
    }
}

fn walk_objects(
    el: &Element,
    f: &mut impl FnMut(&Element) -> Result<(), Error>,
) -> Result<(), Error> {
    if el.local_name() == "txbxContent" {
        return Ok(());
    }
    if el.local_name() == "AlternateContent" {
        if let Some(i) = para::first_alt_child_index(el) {
            if let Node::Element(child) = &el.children[i] {
                return walk_objects(child, f);
            }
        }
        return Ok(());
    }
    if el.local_name() == "object" {
        return f(el);
    }
    for n in &el.children {
        if let Node::Element(e) = n {
            walk_objects(e, f)?;
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
    if el.local_name() == "txbxContent" {
        return None;
    }
    if el.local_name() == "AlternateContent" {
        if let Some(i) = para::first_alt_child_index(el) {
            if let Node::Element(e) = &el.children[i] {
                return find_object_walk(e, idx, key);
            }
        }
        return None;
    }
    if el.local_name() == "object" {
        if is_listed_markup(el) {
            *idx += 1;
            if matches_key(el, key, *idx) {
                return Some(el);
            }
        }
        return None;
    }
    for n in &el.children {
        if let Node::Element(e) = n {
            if let Some(found) = find_object_walk(e, idx, key) {
                return Some(found);
            }
        }
    }
    None
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
            Node::Text(_) => {
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
                Node::Text(_) => false,
            };
            if listed {
                *idx += 1;
                let hit = match &el.children[i] {
                    Node::Element(c) => matches_key(c, key, *idx),
                    Node::Text(_) => false,
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

enum FileSpec {
    Office {
        prog_id: &'static str,
        stem: &'static str,
        ext: &'static str,
        content_type: &'static str,
    },
    Package,
}

impl FileSpec {
    fn prog_id(&self) -> &'static str {
        match self {
            Self::Office { prog_id, .. } => prog_id,
            Self::Package => "Package",
        }
    }

    fn stem(&self) -> &'static str {
        match self {
            Self::Office { stem, .. } => stem,
            Self::Package => "oleObject",
        }
    }

    fn ext(&self) -> &'static str {
        match self {
            Self::Office { ext, .. } => ext,
            Self::Package => "bin",
        }
    }

    fn rel_type(&self) -> &'static str {
        match self {
            Self::Office { .. } => REL_PACKAGE,
            Self::Package => REL_OLE,
        }
    }
}

fn classify_file(path: &str) -> FileSpec {
    let ext = Path::new(path)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "xlsx" => FileSpec::Office {
            prog_id: "Excel.Sheet.12",
            stem: "Microsoft_Excel_Worksheet",
            ext: "xlsx",
            content_type: CT_XLSX,
        },
        "xlsm" => FileSpec::Office {
            prog_id: "Excel.SheetMacroEnabled.12",
            stem: "Microsoft_Excel_Worksheet",
            ext: "xlsm",
            content_type: CT_XLSM,
        },
        "xls" => FileSpec::Office {
            prog_id: "Excel.Sheet.8",
            stem: "Microsoft_Excel_Worksheet",
            ext: "xls",
            content_type: CT_XLS,
        },
        "docx" => FileSpec::Office {
            prog_id: "Word.Document.12",
            stem: "Microsoft_Word_Document",
            ext: "docx",
            content_type: CT_DOCX,
        },
        "docm" => FileSpec::Office {
            prog_id: "Word.DocumentMacroEnabled.12",
            stem: "Microsoft_Word_Document",
            ext: "docm",
            content_type: CT_DOCM,
        },
        "pptx" => FileSpec::Office {
            prog_id: "PowerPoint.Show.12",
            stem: "Microsoft_PowerPoint_Presentation",
            ext: "pptx",
            content_type: CT_PPTX,
        },
        "pptm" => FileSpec::Office {
            prog_id: "PowerPoint.ShowMacroEnabled.12",
            stem: "Microsoft_PowerPoint_Presentation",
            ext: "pptm",
            content_type: CT_PPTM,
        },
        _ => FileSpec::Package,
    }
}

fn next_numbered(pkg: &Package, dir: &str, stem: &str, ext: &str) -> String {
    let mut n = 1u32;
    loop {
        let name = format!("{dir}/{stem}{n}.{ext}");
        if !pkg.contains(&name) {
            return name;
        }
        n += 1;
    }
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

/// 把普通文件打成 OLE Package 复合文档（Ole10Native）。
fn wrap_package(filename: &str, data: &[u8]) -> Vec<u8> {
    let native = ole10_native(filename, data);
    write_cfb(vec![
        ("\u{0001}Ole".into(), ole_stream()),
        ("\u{0001}CompObj".into(), comp_obj_stream()),
        ("\u{0001}Ole10Native".into(), native),
    ])
}

fn ole10_native(filename: &str, data: &[u8]) -> Vec<u8> {
    let name = ascii_filename(filename);
    let mut inner = Vec::new();
    inner.extend_from_slice(&2u16.to_le_bytes());
    inner.extend_from_slice(name.as_bytes());
    inner.push(0);
    inner.extend_from_slice(name.as_bytes());
    inner.push(0);
    inner.extend_from_slice(&0u32.to_le_bytes());
    inner.extend_from_slice(&(data.len() as u32).to_le_bytes());
    inner.extend_from_slice(data);
    let mut out = Vec::new();
    out.extend_from_slice(&(inner.len() as u32).to_le_bytes());
    out.extend(inner);
    out
}

fn ascii_filename(path: &str) -> String {
    let name = Path::new(path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("file.bin");
    name.chars()
        .map(|c| if c.is_ascii() && c != '\0' { c } else { '_' })
        .collect()
}

fn ole_stream() -> Vec<u8> {
    let mut v = vec![0x01, 0x00, 0x00, 0x02];
    v.extend_from_slice(&[0u8; 16]);
    v
}

fn comp_obj_stream() -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(&[0x01, 0x00, 0x09, 0x02, 0x00, 0x00, 0x00, 0x00]);
    let package = b"Package\0";
    v.extend_from_slice(&12u32.to_le_bytes());
    v.extend_from_slice(package);
    v.extend_from_slice(&[0, 0, 0, 0]);
    let clip = b"OLE10 Native\0";
    v.extend_from_slice(&(clip.len() as u32).to_le_bytes());
    v.extend_from_slice(clip);
    v.extend_from_slice(&[0u8; 8]);
    v
}

const CFB_SIG: [u8; 8] = [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];
const SEC: usize = 512;
const EOC: u32 = 0xFFFF_FFFE;
const FATSECT: u32 = 0xFFFF_FFFD;
const FREE: u32 = 0xFFFF_FFFF;
const NONE: u32 = 0xFFFF_FFFF;

fn write_cfb(mut streams: Vec<(String, Vec<u8>)>) -> Vec<u8> {
    streams.sort_by_key(|a| a.0.to_ascii_lowercase());
    let dir_secs = 1usize;
    let stream_secs: Vec<usize> = streams
        .iter()
        .map(|(_, d)| d.len().max(1).div_ceil(SEC).max(1))
        .collect();
    let data_secs: usize = stream_secs.iter().sum();
    let mut fat_secs = 1usize;
    loop {
        let total = fat_secs + dir_secs + data_secs;
        let need = total.div_ceil(128);
        if need <= fat_secs {
            break;
        }
        fat_secs = need;
    }
    let mut fat = vec![FREE; fat_secs * 128];
    for slot in fat.iter_mut().take(fat_secs) {
        *slot = FATSECT;
    }
    let dir_start = fat_secs as u32;
    chain(&mut fat, dir_start, dir_secs);
    let mut next = fat_secs + dir_secs;
    let mut dir = vec![DirEnt::root()];
    for ((name, data), nsec) in streams.iter().zip(stream_secs.iter()) {
        chain(&mut fat, next as u32, *nsec);
        dir.push(DirEnt::stream(name, next as u32, data.len() as u64));
        next += *nsec;
    }
    dir[0].child = 1;
    let last = dir.len() - 1;
    for (i, ent) in dir.iter_mut().enumerate().take(last + 1).skip(1) {
        ent.color = 1;
        ent.left = NONE;
        ent.right = if i < last { i as u32 + 1 } else { NONE };
    }
    while dir.len() < 4 {
        dir.push(DirEnt::empty());
    }

    let mut out = vec![0u8; SEC + (fat_secs + dir_secs + data_secs) * SEC];
    write_header(&mut out[..SEC], fat_secs as u32, dir_start);
    for (i, ent) in fat.iter().take(fat_secs * 128).enumerate() {
        let off = SEC + (i / 128) * SEC + (i % 128) * 4;
        out[off..off + 4].copy_from_slice(&ent.to_le_bytes());
    }
    let dir_off = SEC + fat_secs * SEC;
    for (i, ent) in dir.iter().enumerate() {
        write_dirent(&mut out[dir_off + i * 128..dir_off + (i + 1) * 128], ent);
    }
    let mut data_off = SEC + (fat_secs + dir_secs) * SEC;
    for ((_, data), nsec) in streams.iter().zip(stream_secs.iter()) {
        let n = *nsec * SEC;
        out[data_off..data_off + data.len()].copy_from_slice(data);
        data_off += n;
    }
    out
}

fn chain(fat: &mut [u32], start: u32, n: usize) {
    for i in 0..n {
        let cur = start as usize + i;
        fat[cur] = if i + 1 == n {
            EOC
        } else {
            start + i as u32 + 1
        };
    }
}

fn write_header(buf: &mut [u8], fat_secs: u32, dir_start: u32) {
    buf[..8].copy_from_slice(&CFB_SIG);
    buf[0x18..0x1A].copy_from_slice(&0x003Eu16.to_le_bytes());
    buf[0x1A..0x1C].copy_from_slice(&0x0003u16.to_le_bytes());
    buf[0x1C..0x1E].copy_from_slice(&0xFFFEu16.to_le_bytes());
    buf[0x1E..0x20].copy_from_slice(&9u16.to_le_bytes());
    buf[0x20..0x22].copy_from_slice(&6u16.to_le_bytes());
    buf[0x2C..0x30].copy_from_slice(&fat_secs.to_le_bytes());
    buf[0x30..0x34].copy_from_slice(&dir_start.to_le_bytes());
    buf[0x38..0x3C].copy_from_slice(&4096u32.to_le_bytes());
    buf[0x3C..0x40].copy_from_slice(&EOC.to_le_bytes());
    buf[0x44..0x48].copy_from_slice(&EOC.to_le_bytes());
    for i in 0..109 {
        let off = 0x4C + i * 4;
        let v = if (i as u32) < fat_secs {
            i as u32
        } else {
            FREE
        };
        buf[off..off + 4].copy_from_slice(&v.to_le_bytes());
    }
}

struct DirEnt {
    name: String,
    kind: u8,
    color: u8,
    left: u32,
    right: u32,
    child: u32,
    start: u32,
    size: u64,
}

impl DirEnt {
    fn root() -> Self {
        Self {
            name: "Root Entry".into(),
            kind: 5,
            color: 1,
            left: NONE,
            right: NONE,
            child: NONE,
            start: EOC,
            size: 0,
        }
    }

    fn stream(name: &str, start: u32, size: u64) -> Self {
        Self {
            name: name.into(),
            kind: 2,
            color: 1,
            left: NONE,
            right: NONE,
            child: NONE,
            start,
            size,
        }
    }

    fn empty() -> Self {
        Self {
            name: String::new(),
            kind: 0,
            color: 0,
            left: NONE,
            right: NONE,
            child: NONE,
            start: EOC,
            size: 0,
        }
    }
}

fn write_dirent(buf: &mut [u8], e: &DirEnt) {
    let utf16: Vec<u16> = e.name.encode_utf16().collect();
    let mut name_bytes = [0u8; 64];
    for (i, u) in utf16.iter().take(31).enumerate() {
        name_bytes[i * 2..i * 2 + 2].copy_from_slice(&u.to_le_bytes());
    }
    buf[..64].copy_from_slice(&name_bytes);
    let name_len = if e.kind == 0 {
        0u16
    } else {
        ((utf16.len() + 1) * 2) as u16
    };
    buf[0x40..0x42].copy_from_slice(&name_len.to_le_bytes());
    buf[0x42] = e.kind;
    buf[0x43] = e.color;
    buf[0x44..0x48].copy_from_slice(&e.left.to_le_bytes());
    buf[0x48..0x4C].copy_from_slice(&e.right.to_le_bytes());
    buf[0x4C..0x50].copy_from_slice(&e.child.to_le_bytes());
    buf[0x74..0x78].copy_from_slice(&e.start.to_le_bytes());
    buf[0x78..0x80].copy_from_slice(&e.size.to_le_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_package_is_cfb_and_keeps_bytes() {
        let payload = b"hello-ole-package";
        let bin = wrap_package("notes.txt", payload);
        assert_eq!(&bin[..8], &CFB_SIG);
        assert!(bin.windows(payload.len()).any(|w| w == payload));
    }
}
