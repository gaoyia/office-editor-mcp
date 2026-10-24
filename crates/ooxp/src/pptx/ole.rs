//! 幻灯片嵌入对象：`p:graphicFrame` 里的 `p:oleObj`。
//! 不执行 OLE 服务器，不把嵌入字节塞进 inspect 树。改尺寸只动预览框。


use serde_json::{json, Map, Value};

use crate::error::{Error, ErrorKind, IdReason};
use crate::ole::{self, FileSpec};
use crate::opc::{resolve_id, ContentTypes, Package, Relationship, Relationships};
use crate::xml::{Document, Element, Node};

use super::picture;
use super::shape::{self, first_alt_child_index};
use super::{DML_NS, REL_NS};

pub const OLE_URI: &str = "http://schemas.openxmlformats.org/presentationml/2006/ole";
pub const OLE_URI_ISO: &str = "http://purl.oclc.org/ooxml/presentationml/ole";
const MC_NS: &str = "http://schemas.openxmlformats.org/markup-compatibility/2006";
const VML_NS: &str = "urn:schemas-microsoft-com:vml";

const DEFAULT_CX: i64 = 3_657_600;
const DEFAULT_CY: i64 = 2_743_200;

#[derive(Debug, Clone)]
pub enum OleKey {
    Id(String),
    Index(usize),
}

#[derive(Debug, Clone)]
pub struct OleTarget {
    pub slide: usize,
    pub on_notes: bool,
    pub master: Option<(usize, Option<usize>)>,
    pub key: OleKey,
}

#[derive(Debug, Clone)]
pub struct InsertOleOp {
    pub slide: usize,
    pub on_notes: bool,
    pub master: Option<(usize, Option<usize>)>,
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

pub fn parse_insert_slide_id(id: &str, op_index: usize) -> Result<picture::TreeHost, Error> {
    picture::parse_insert_host(id, op_index, IdReason::InsertOleNeedsSlide)
}

pub fn parse_ole_id(id: &str, json_path: &str) -> Result<OleTarget, Error> {
    if picture::is_bare_host(id) {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SlideOleContainer,
            },
            json_path.to_string(),
        ));
    }
    let (host, after) = picture::parse_object_id(id, json_path, "ole", IdReason::SlideOlePrefix)?;
    if after.is_empty() {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SlideOlePrefix,
            },
            json_path.to_string(),
        ));
    }
    let (slide, on_notes, master) = host.parts();
    let key = if let Some(rest) = after.strip_prefix("index:") {
        let k: usize = rest.parse().map_err(|_| {
            Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::OleIndexNotInt,
                },
                json_path.to_string(),
            )
        })?;
        if k < 1 {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::OleIndexFromOne,
                },
                json_path.to_string(),
            ));
        }
        OleKey::Index(k)
    } else {
        OleKey::Id(after.to_string())
    };
    Ok(OleTarget {
        slide,
        on_notes,
        master,
        key,
    })
}

pub fn is_ole_id(id: &str) -> bool {
    id.contains("/ole:")
}

pub fn count(pkg: &Package, slide_part: &str, root: &Element) -> Result<usize, Error> {
    Ok(collect_frames(pkg, slide_part, root)?.len())
}

pub fn collect(
    pkg: &Package,
    slide_part: &str,
    root: &Element,
    host: &str,
) -> Result<Vec<Value>, Error> {
    let frames = collect_frames(pkg, slide_part, root)?;
    let cts = ContentTypes::from_package(pkg)?;
    let mut out = Vec::new();
    for (i, frame) in frames.iter().enumerate() {
        out.push(frame_json(pkg, slide_part, &cts, frame, host, i + 1)?);
    }
    Ok(out)
}

pub fn export(bytes: &[u8], id: &str) -> Result<super::ExportedMedia, Error> {
    let target = parse_ole_id(id, "id")?;
    let pkg = Package::open_bytes(bytes.to_vec())?;
    let pres_part = crate::opc::office_document_part(&pkg)?;
    let pres = crate::xml::parse(pkg.get(&pres_part)?)?;
    let slides = shape::list_slides(&pkg, &pres_part, &pres.root)?;
    let host = picture::TreeHost::from_parts(target.slide, target.on_notes, target.master);
    let host_part = picture::resolve_tree_part(
        &pkg,
        &pres_part,
        &pres.root,
        &slides,
        &host,
        ErrorKind::OleNotFound { id: id.to_string() },
        "id",
    )?;
    let doc = crate::xml::parse(pkg.get(&host_part)?)?;
    let frame = find_frame(&pkg, &host_part, &doc.root, &target.key)?
        .ok_or_else(|| Error::with_path(ErrorKind::OleNotFound { id: id.to_string() }, "id"))?;
    let obj = find_ole_obj(frame)
        .ok_or_else(|| Error::with_path(ErrorKind::OleNotFound { id: id.to_string() }, "id"))?;
    let rid = shape::rid_attr(obj).ok_or_else(|| {
        Error::with_path(
            ErrorKind::PartMissing {
                name: format!("{id} oleObject"),
            },
            "id",
        )
    })?;
    let part = resolve_id(&pkg, &host_part, rid)?;
    let embed_bytes = pkg.get(&part)?.to_vec();
    let cts = ContentTypes::from_package(&pkg)?;
    let content_type = cts.content_type_of(&part).map(str::to_string);
    Ok(super::ExportedMedia {
        bytes: embed_bytes,
        content_type,
    })
}

pub fn insert(
    pkg: &mut Package,
    slide_doc: &mut Document,
    slide_part: &str,
    pres_root: &Element,
    op: &InsertOleOp,
    _i: usize,
) -> Result<(), Error> {
    let spec = ole::classify(&op.file);
    let bytes = crate::host_files::read_user_file(&op.file)?;
    let payload = match &spec {
        FileSpec::Office { .. } => bytes,
        FileSpec::Package => ole::wrap_package(&op.file, &bytes),
    };
    let embed_part = ole::next_numbered(pkg, "/ppt/embeddings", spec.stem(), spec.ext());
    pkg.set(&embed_part, payload)?;
    let png_part = picture::next_media_part(pkg, "png");
    pkg.set(&png_part, ole::PREVIEW_PNG.to_vec())?;

    let mut rels = Relationships::from_package(pkg, slide_part)?;
    let ole_rid = rels.next_rid();
    rels.push(Relationship {
        id: ole_rid.clone(),
        rel_type: spec.rel_type().into(),
        target: picture::rel_target_from(slide_part, &embed_part),
        target_mode: None,
        extra: Vec::new(),
    });
    let img_rid = rels.next_rid();
    rels.push(Relationship {
        id: img_rid.clone(),
        rel_type: picture::REL_IMAGE.into(),
        target: picture::rel_target_from(slide_part, &png_part),
        target_mode: None,
        extra: Vec::new(),
    });
    rels.write_to(pkg, slide_part)?;

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

    let (cx, cy) = (DEFAULT_CX, DEFAULT_CY);
    let (x, y) = picture::vacant_pos_host(op.on_notes, pres_root, &slide_doc.root, cx, cy);
    picture::ensure_xmlns(&mut slide_doc.root, "r", REL_NS);
    picture::ensure_xmlns(&mut slide_doc.root, "a", DML_NS);
    picture::ensure_xmlns(&mut slide_doc.root, "mc", MC_NS);
    let cnv = picture::next_cnv_id(&slide_doc.root);
    let frame = build_frame(cnv, &spec, &ole_rid, &img_rid, x, y, cx, cy);
    picture::insert_into_sp_tree(&mut slide_doc.root, frame)?;
    Ok(())
}

pub fn delete(
    pkg: &mut Package,
    slide_doc: &mut Document,
    slide_part: &str,
    op: &DeleteOleOp,
    op_index: usize,
) -> Result<(), Error> {
    let path = format!("ops[{op_index}].id");
    let frame = find_frame(pkg, slide_part, &slide_doc.root, &op.target.key)?.ok_or_else(|| {
        Error::with_path(ErrorKind::OleNotFound { id: op.id.clone() }, path.clone())
    })?;
    let spid = cnv_of_frame(frame).0.filter(|s| !s.is_empty());
    let mut rids = Vec::new();
    collect_rids(frame, &mut rids);
    let mut old_parts = Vec::new();
    for rid in &rids {
        if let Ok(p) = resolve_id(pkg, slide_part, rid) {
            if !old_parts.iter().any(|x| x == &p) {
                old_parts.push(p);
            }
        }
    }
    if !remove_frame(&mut slide_doc.root, pkg, slide_part, &op.target.key)? {
        return Err(Error::with_path(
            ErrorKind::OleNotFound { id: op.id.clone() },
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
        if !picture::media_still_used(pkg, &part)? {
            pkg.remove(&part);
            let mut cts = ContentTypes::from_package(pkg)?;
            cts.remove_override(&part);
            cts.write_to(pkg)?;
        }
    }
    Ok(())
}

pub fn set_size(
    slide_doc: &mut Document,
    slide_part: &str,
    pkg: &Package,
    op: &SetOleSizeOp,
    op_index: usize,
) -> Result<(), Error> {
    let path = format!("ops[{op_index}].id");
    let rels = Relationships::from_package(pkg, slide_part)?;
    if !patch_frame_size(
        &mut slide_doc.root,
        &rels,
        &mut 0,
        &op.target.key,
        op.cx,
        op.cy,
    ) {
        return Err(Error::with_path(
            ErrorKind::OleNotFound { id: op.id.clone() },
            path,
        ));
    }
    Ok(())
}

fn collect_frames<'a>(
    pkg: &Package,
    slide_part: &str,
    root: &'a Element,
) -> Result<Vec<&'a Element>, Error> {
    let rels = Relationships::from_package(pkg, slide_part)?;
    let mut out = Vec::new();
    collect_in(root, &rels, &mut out);
    Ok(out)
}

fn collect_in<'a>(el: &'a Element, rels: &Relationships, out: &mut Vec<&'a Element>) {
    if el.local_name() == "AlternateContent" {
        if let Some(i) = first_alt_child_index(el) {
            if let Node::Element(child) = &el.children[i] {
                collect_in(child, rels, out);
            }
        }
        return;
    }
    if is_counted_frame(el, rels) {
        out.push(el);
        return;
    }
    for n in &el.children {
        if let Node::Element(e) = n {
            collect_in(e, rels, out);
        }
    }
}

fn is_counted_frame(el: &Element, rels: &Relationships) -> bool {
    if el.local_name() != "graphicFrame" {
        return false;
    }
    if !graphic_uri(el).is_some_and(|u| is_ole_uri(&u)) {
        return false;
    }
    let Some(obj) = find_ole_obj(el) else {
        return false;
    };
    if obj.child_by_local("link").is_some() {
        return false;
    }
    if obj.child_by_local("embed").is_none() {
        return false;
    }
    let Some(rid) = shape::rid_attr(obj) else {
        return false;
    };
    rels.rels
        .iter()
        .any(|r| r.id == rid && !r.is_external() && is_ole_rel(&r.rel_type))
}

fn is_ole_uri(u: &str) -> bool {
    let n = u.trim_end_matches('/');
    n == OLE_URI || n == OLE_URI_ISO
}

fn is_ole_rel(rel_type: &str) -> bool {
    rel_type.ends_with("/oleObject") || rel_type.ends_with("/package")
}

fn graphic_uri(el: &Element) -> Option<String> {
    let mut found = None;
    shape::walk_choice(el, &mut |e| {
        if e.local_name() == "graphicData" && found.is_none() {
            found = e
                .attr("uri")
                .or_else(|| e.attr_local("uri"))
                .map(str::to_string);
        }
    });
    found
}

fn find_ole_obj(frame: &Element) -> Option<&Element> {
    let mut found = None;
    shape::walk_choice(frame, &mut |e| {
        if e.local_name() == "oleObj" && found.is_none() {
            found = Some(e);
        }
    });
    found
}

fn frame_json(
    pkg: &Package,
    slide_part: &str,
    cts: &ContentTypes,
    frame: &Element,
    host: &str,
    index: usize,
) -> Result<Value, Error> {
    let (cnv, _) = cnv_of_frame(frame);
    let obj = find_ole_obj(frame);
    let prog_id = obj
        .and_then(|o| o.attr("progId"))
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    let name = obj.and_then(|o| o.attr("name")).unwrap_or("").to_string();
    let show_as_icon = obj.is_some_and(|o| {
        matches!(
            o.attr("showAsIcon").map(str::trim),
            Some("1") | Some("true") | Some("True")
        )
    });
    let (id, stable) = match &cnv {
        Some(n) if !n.is_empty() => (format!("{host}/ole:{n}"), true),
        _ => (format!("{host}/ole:index:{index}"), false),
    };
    let kind = match obj.and_then(shape::rid_attr) {
        Some(rid) => {
            let rels = Relationships::from_package(pkg, slide_part)?;
            match rels.rels.iter().find(|r| r.id == rid) {
                Some(r) if r.rel_type.ends_with("/package") => "package",
                _ => "embed",
            }
        }
        None => "embed",
    };
    let content_type = match obj.and_then(shape::rid_attr) {
        Some(rid) => match resolve_id(pkg, slide_part, rid) {
            Ok(part) => cts.content_type_of(&part).map(str::to_string),
            Err(_) => None,
        },
        None => None,
    };
    let xfrm = frame.child_by_local("xfrm");
    let (x, y) = match xfrm.and_then(|x| x.child_by_local("off")) {
        Some(off) => (parse_i64_attr(off, "x"), parse_i64_attr(off, "y")),
        None => (None, None),
    };
    let (cx, cy) = match xfrm.and_then(|x| x.child_by_local("ext")) {
        Some(ext) => (parse_i64_attr(ext, "cx"), parse_i64_attr(ext, "cy")),
        None => (None, None),
    };
    let mut m = Map::new();
    m.insert("id".into(), json!(id));
    m.insert("progId".into(), json!(prog_id));
    m.insert("name".into(), json!(name));
    m.insert("kind".into(), json!(kind));
    m.insert("contentType".into(), json!(content_type));
    m.insert("showAsIcon".into(), json!(show_as_icon));
    m.insert("x".into(), json!(x));
    m.insert("y".into(), json!(y));
    m.insert("cx".into(), json!(cx));
    m.insert("cy".into(), json!(cy));
    m.insert("stable".into(), json!(stable));
    Ok(Value::Object(m))
}

fn cnv_of_frame(frame: &Element) -> (Option<String>, String) {
    let mut raw = None;
    let mut name = String::new();
    if let Some(nv) = frame.child_by_local("nvGraphicFramePr") {
        if let Some(pr) = nv.child_by_local("cNvPr") {
            raw = pr.attr("id").filter(|s| !s.is_empty()).map(str::to_string);
            name = pr.attr("name").unwrap_or("").to_string();
        }
    }
    (raw, name)
}

fn parse_i64_attr(el: &Element, name: &str) -> Option<i64> {
    el.attr(name)?.parse().ok()
}

fn ids_match(file: &str, req: &str) -> bool {
    match (file.parse::<u64>(), req.parse::<u64>()) {
        (Ok(a), Ok(b)) => a == b,
        _ => file == req,
    }
}

fn frame_matches(frame: &Element, index: usize, key: &OleKey) -> bool {
    match key {
        OleKey::Index(n) => index == *n,
        OleKey::Id(req) => cnv_of_frame(frame).0.is_some_and(|id| ids_match(&id, req)),
    }
}

fn find_frame<'a>(
    pkg: &Package,
    slide_part: &str,
    root: &'a Element,
    key: &OleKey,
) -> Result<Option<&'a Element>, Error> {
    let frames = collect_frames(pkg, slide_part, root)?;
    for (i, f) in frames.iter().enumerate() {
        if frame_matches(f, i + 1, key) {
            return Ok(Some(*f));
        }
    }
    Ok(None)
}

fn patch_frame_size(
    el: &mut Element,
    rels: &Relationships,
    idx: &mut usize,
    key: &OleKey,
    cx: Option<i64>,
    cy: Option<i64>,
) -> bool {
    if el.local_name() == "AlternateContent" {
        if let Some(i) = first_alt_child_index(el) {
            if let Node::Element(child) = &mut el.children[i] {
                return patch_frame_size(child, rels, idx, key, cx, cy);
            }
        }
        return false;
    }
    let mut i = 0;
    while i < el.children.len() {
        let is_frame = matches!(&el.children[i], Node::Element(e) if is_counted_frame(e, rels));
        if is_frame {
            *idx += 1;
            let hit = matches!(&el.children[i], Node::Element(e) if frame_matches(e, *idx, key));
            if hit {
                if let Node::Element(e) = &mut el.children[i] {
                    apply_frame_size(e, cx, cy);
                }
                return true;
            }
        } else if let Node::Element(e) = &mut el.children[i] {
            if patch_frame_size(e, rels, idx, key, cx, cy) {
                return true;
            }
        }
        i += 1;
    }
    false
}

fn apply_frame_size(frame: &mut Element, cx: Option<i64>, cy: Option<i64>) {
    let (cur_x, cur_y) = frame
        .child_by_local("xfrm")
        .and_then(|x| x.child_by_local("ext"))
        .map(|ext| (parse_i64_attr(ext, "cx"), parse_i64_attr(ext, "cy")))
        .unwrap_or((None, None));
    let nx = cx.or(cur_x);
    let ny = cy.or(cur_y);
    walk_mut(frame, &mut |e| {
        if e.local_name() == "ext" && (e.attr_local("cx").is_some() || e.attr_local("cy").is_some())
        {
            if let Some(v) = nx {
                set_local_attr(e, "cx", &v.to_string());
            }
            if let Some(v) = ny {
                set_local_attr(e, "cy", &v.to_string());
            }
        }
        if e.local_name() == "oleObj" {
            if let Some(v) = nx {
                set_local_attr(e, "imgW", &v.to_string());
            }
            if let Some(v) = ny {
                set_local_attr(e, "imgH", &v.to_string());
            }
        }
    });
}

fn walk_mut(el: &mut Element, f: &mut impl FnMut(&mut Element)) {
    f(el);
    let n = el.children.len();
    for i in 0..n {
        if let Node::Element(e) = &mut el.children[i] {
            walk_mut(e, f);
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

fn remove_frame(
    root: &mut Element,
    pkg: &Package,
    slide_part: &str,
    key: &OleKey,
) -> Result<bool, Error> {
    let rels = Relationships::from_package(pkg, slide_part)?;
    let mut idx = 0usize;
    Ok(remove_in(root, &rels, &mut idx, key))
}

fn remove_in(el: &mut Element, rels: &Relationships, idx: &mut usize, key: &OleKey) -> bool {
    if el.local_name() == "AlternateContent" {
        if let Some(i) = first_alt_child_index(el) {
            if let Node::Element(child) = &mut el.children[i] {
                return remove_in(child, rels, idx, key);
            }
        }
        return false;
    }
    let mut i = 0;
    while i < el.children.len() {
        let is_hit = match &el.children[i] {
            Node::Element(e) if is_counted_frame(e, rels) => {
                *idx += 1;
                frame_matches(e, *idx, key)
            }
            _ => false,
        };
        if is_hit {
            el.children.remove(i);
            return true;
        }
        if let Node::Element(e) = &mut el.children[i] {
            if !is_counted_frame(e, rels) && remove_in(e, rels, idx, key) {
                return true;
            }
        }
        i += 1;
    }
    false
}

fn collect_rids(el: &Element, out: &mut Vec<String>) {
    // 删时两支都要扫，预览图只在 Fallback。
    walk_all(el, &mut |e| {
        if e.local_name() == "oleObj" {
            if let Some(rid) = shape::rid_attr(e) {
                push_rid(out, rid);
            }
        }
        if e.local_name() == "blip" {
            if let Some(rid) = e.attr_local("embed").or_else(|| e.attr("embed")) {
                if !rid.is_empty() {
                    push_rid(out, rid);
                }
            }
        }
    });
}

fn walk_all(el: &Element, f: &mut impl FnMut(&Element)) {
    f(el);
    for n in &el.children {
        if let Node::Element(e) = n {
            walk_all(e, f);
        }
    }
}

fn push_rid(out: &mut Vec<String>, rid: &str) {
    if !out.iter().any(|x| x == rid) {
        out.push(rid.to_string());
    }
}

fn rid_still_on_slide(root: &Element, rid: &str) -> bool {
    let mut found = false;
    walk_all(root, &mut |e| {
        for (k, v) in &e.attrs {
            if v != rid {
                continue;
            }
            let local = k.rsplit(':').next().unwrap_or(k);
            if matches!(local, "id" | "embed" | "link") {
                found = true;
            }
        }
    });
    found
}

fn build_frame(
    cnv: u32,
    spec: &FileSpec,
    ole_rid: &str,
    img_rid: &str,
    x: i64,
    y: i64,
    cx: i64,
    cy: i64,
) -> Element {
    let mut nv = Element::new("p:nvGraphicFramePr");
    nv.push_element(
        Element::new("p:cNvPr")
            .with_attr("id", cnv.to_string())
            .with_attr("name", format!("对象 {cnv}")),
    );
    let mut locks = Element::new("p:cNvGraphicFramePr");
    locks.push_element(Element::new("a:graphicFrameLocks").with_attr("noChangeAspect", "1"));
    nv.push_element(locks);
    nv.push_element(Element::new("p:nvPr"));
    let mut xfrm = Element::new("p:xfrm");
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
    let choice_obj = build_ole_obj(spec, ole_rid, cx, cy);
    let mut fallback_obj = build_ole_obj(spec, ole_rid, cx, cy);
    fallback_obj.push_element(build_preview_pic(img_rid, x, y, cx, cy));
    let mut choice = Element::new("mc:Choice")
        .with_attr("xmlns:v", VML_NS)
        .with_attr("Requires", "v");
    choice.push_element(choice_obj);
    let mut fallback = Element::new("mc:Fallback");
    fallback.push_element(fallback_obj);
    let mut ac = Element::new("mc:AlternateContent").with_attr("xmlns:mc", MC_NS);
    ac.push_element(choice);
    ac.push_element(fallback);
    let mut data = Element::new("a:graphicData").with_attr("uri", OLE_URI);
    data.push_element(ac);
    let mut graphic = Element::new("a:graphic");
    graphic.push_element(data);
    let mut frame = Element::new("p:graphicFrame");
    frame.push_element(nv);
    frame.push_element(xfrm);
    frame.push_element(graphic);
    frame
}

fn build_ole_obj(spec: &FileSpec, ole_rid: &str, cx: i64, cy: i64) -> Element {
    let mut obj = Element::new("p:oleObj")
        .with_attr("name", spec.ole_obj_name())
        .with_attr("r:id", ole_rid)
        .with_attr("imgW", cx.to_string())
        .with_attr("imgH", cy.to_string())
        .with_attr("progId", spec.prog_id());
    if spec.show_as_icon() {
        obj = obj.with_attr("showAsIcon", "1");
    }
    obj.push_element(Element::new("p:embed"));
    obj
}

fn build_preview_pic(img_rid: &str, x: i64, y: i64, cx: i64, cy: i64) -> Element {
    let mut pic = Element::new("p:pic");
    let mut nv = Element::new("p:nvPicPr");
    nv.push_element(
        Element::new("p:cNvPr")
            .with_attr("id", "0")
            .with_attr("name", ""),
    );
    nv.push_element(Element::new("p:cNvPicPr"));
    nv.push_element(Element::new("p:nvPr"));
    pic.push_element(nv);
    let mut fill = Element::new("p:blipFill");
    fill.push_element(Element::new("a:blip").with_attr("r:embed", img_rid));
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
