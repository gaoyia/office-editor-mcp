//! 幻灯片 3D 模型：`p:graphicFrame` 里的 `am3d:model3d`。
//! 不进 `pictures`。Fallback 海报不进 `pictures`。

use serde_json::{json, Map, Value};

use crate::error::{Error, ErrorKind, IdReason};
use crate::opc::{resolve_id, ContentTypes, Package, Relationships};
use crate::xml::{Document, Element, Node};

use super::picture;
use super::shape;
use super::{DML_NS, REL_NS};

const MC_NS: &str = "http://schemas.openxmlformats.org/markup-compatibility/2006";

pub use crate::model3d::Model3dKey;

pub(crate) fn pin_models(root: &mut Element) {
    crate::xml::pin_counted(root, "model3d", crate::model3d::is_model3d_graphic, |_| {
        false
    });
}

fn model_hit(el: &Element, key: &Model3dKey, index: usize) -> bool {
    if let Model3dKey::Index(n) = key {
        return crate::xml::pin_matches(el, &crate::xml::pin_attr("model3d"), *n, index);
    }
    let raw = cnv_of(el).0;
    crate::model3d::key_matches(key, &raw, index)
}

#[derive(Debug, Clone)]
pub struct Model3dTarget {
    pub slide: usize,
    pub on_notes: bool,
    pub master: Option<(usize, Option<usize>)>,
    pub key: Model3dKey,
}

#[derive(Debug, Clone)]
pub struct InsertModel3dOp {
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
pub struct ReplaceModel3dOp {
    pub id: String,
    pub target: Model3dTarget,
    pub file: String,
}

#[derive(Debug, Clone)]
pub struct DeleteModel3dOp {
    pub id: String,
    pub target: Model3dTarget,
}

#[derive(Debug, Clone)]
pub struct SetModel3dSizeOp {
    pub id: String,
    pub target: Model3dTarget,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct SetModel3dPosOp {
    pub id: String,
    pub target: Model3dTarget,
    pub x: Option<i64>,
    pub y: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct SetModel3dCameraOp {
    pub id: String,
    pub target: Model3dTarget,
    pub patch: crate::model3d::Model3dCameraPatch,
}

pub struct Model3dInfo {
    pub id: String,
    pub cnv_pr_id: Option<String>,
    pub name: String,
    pub x: Option<i64>,
    pub y: Option<i64>,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
    pub stable: bool,
    pub camera: Value,
    pub rot: Value,
    pub light_rig: Value,
}

pub fn looks_like_id(id: &str) -> bool {
    id.contains("/model3d:") && (id.starts_with("slide:") || id.starts_with("master:"))
}

pub fn parse_id(id: &str, json_path: &str) -> Result<Model3dTarget, Error> {
    if picture::is_bare_host(id) {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SlideModel3dContainer,
            },
            json_path.to_string(),
        ));
    }
    let (host, after) =
        picture::parse_object_id(id, json_path, "model3d", IdReason::SlideModel3dPrefix)?;
    if after.is_empty() {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SlideModel3dPrefix,
            },
            json_path.to_string(),
        ));
    }
    let (slide, on_notes, master) = host.parts();
    Ok(Model3dTarget {
        slide,
        on_notes,
        master,
        key: parse_key(after, json_path)?,
    })
}

fn parse_key(s: &str, path: &str) -> Result<Model3dKey, Error> {
    if let Some(rest) = s.strip_prefix("index:") {
        let n: usize = rest.parse().map_err(|_| {
            Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::SlideModel3dPrefix,
                },
                path.to_string(),
            )
        })?;
        if n < 1 {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::SlideModel3dPrefix,
                },
                path.to_string(),
            ));
        }
        return Ok(Model3dKey::Index(n));
    }
    if s.is_empty() || s.contains('/') {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SlideModel3dPrefix,
            },
            path.to_string(),
        ));
    }
    Ok(Model3dKey::Id(s.to_string()))
}

pub fn count(root: &Element) -> usize {
    collect(root, "").len()
}

pub fn collect(root: &Element, host: &str) -> Vec<Model3dInfo> {
    let mut out = Vec::new();
    let mut index = 0usize;
    let mut frames = Vec::new();
    collect_frames(root, &mut frames);
    for frame in frames {
        index += 1;
        out.push(info_of(frame, host, index));
    }
    out
}

fn collect_frames<'a>(el: &'a Element, out: &mut Vec<&'a Element>) {
    crate::model3d::collect_frames(el, out, &["graphicFrame"], false);
}

fn info_of(frame: &Element, host: &str, index: usize) -> Model3dInfo {
    let (raw, name) = cnv_of(frame);
    let stable = raw.as_ref().is_some_and(|s| !s.is_empty());
    let id = match &raw {
        Some(v) if !v.is_empty() => format!("{host}/model3d:{v}"),
        _ => format!("{host}/model3d:index:{index}"),
    };
    let xfrm = frame.child_by_local("xfrm");
    let (x, y) = match xfrm.and_then(|x| x.child_by_local("off")) {
        Some(off) => (parse_i64(off, "x"), parse_i64(off, "y")),
        None => (None, None),
    };
    let (cx, cy) = match xfrm.and_then(|x| x.child_by_local("ext")) {
        Some(ext) => (parse_i64(ext, "cx"), parse_i64(ext, "cy")),
        None => (None, None),
    };
    let (camera, rot, light_rig) = crate::model3d::inspect_view(frame);
    Model3dInfo {
        id,
        cnv_pr_id: raw.filter(|s| !s.is_empty()),
        name,
        x,
        y,
        cx,
        cy,
        stable,
        camera,
        rot,
        light_rig,
    }
}

fn cnv_of(frame: &Element) -> (Option<String>, String) {
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

fn parse_i64(el: &Element, name: &str) -> Option<i64> {
    el.attr(name)?.parse().ok()
}

pub fn list_json(items: &[Model3dInfo]) -> Vec<Value> {
    items.iter().map(info_json).collect()
}

fn info_json(p: &Model3dInfo) -> Value {
    let mut m = Map::new();
    m.insert("id".into(), json!(p.id));
    m.insert("cNvPrId".into(), json!(p.cnv_pr_id));
    m.insert("name".into(), json!(p.name));
    m.insert("x".into(), json!(p.x));
    m.insert("y".into(), json!(p.y));
    m.insert("cx".into(), json!(p.cx));
    m.insert("cy".into(), json!(p.cy));
    m.insert("contentType".into(), json!(crate::model3d::CT_GLB));
    m.insert("stable".into(), json!(p.stable));
    m.insert("camera".into(), p.camera.clone());
    m.insert("rot".into(), p.rot.clone());
    m.insert("lightRig".into(), p.light_rig.clone());
    Value::Object(m)
}

pub fn export(bytes: &[u8], id: &str) -> Result<super::ExportedMedia, Error> {
    let pkg = Package::open_bytes(bytes.to_vec())?;
    let target = parse_id(id, "id")?;
    let pres_part = crate::opc::office_document_part(&pkg)?;
    let pres = super::list_pin::parse_part(&pkg, &pres_part)?;
    let slides = shape::list_slides(&pkg, &pres_part, &pres.root)?;
    let host = super::picture::TreeHost::from_parts(target.slide, target.on_notes, target.master);
    let part = super::picture::resolve_tree_part(
        &pkg,
        &pres_part,
        &pres.root,
        &slides,
        &host,
        ErrorKind::Model3dNotFound { id: id.to_string() },
        "id",
    )?;
    let doc = super::list_pin::parse_part(&pkg, &part)?;
    let frame = find_frame(&doc.root, &target.key)
        .ok_or_else(|| Error::with_path(ErrorKind::Model3dNotFound { id: id.to_string() }, "id"))?;
    let rid = crate::model3d::glb_embed_rid(frame)
        .ok_or_else(|| Error::with_path(ErrorKind::Model3dNotFound { id: id.to_string() }, "id"))?;
    let media = resolve_id(&pkg, &part, &rid)?;
    let media_bytes = pkg.get(&media)?.to_vec();
    let cts = ContentTypes::from_package(&pkg)?;
    let content_type = cts.content_type_of(&media).map(str::to_string);
    Ok(super::ExportedMedia {
        bytes: media_bytes,
        content_type,
    })
}

fn find_frame<'a>(root: &'a Element, key: &Model3dKey) -> Option<&'a Element> {
    let mut frames = Vec::new();
    collect_frames(root, &mut frames);
    let mut index = 0usize;
    for frame in frames {
        index += 1;
        if model_hit(frame, key, index) {
            return Some(frame);
        }
    }
    None
}

pub fn insert(
    pkg: &mut Package,
    slide_doc: &mut Document,
    slide_part: &str,
    op: &InsertModel3dOp,
    file_field: &str,
) -> Result<(), Error> {
    let glb = crate::model3d::read_glb(&op.file).map_err(|e| e.at(file_field))?;
    let (cx, cy) = match (op.cx, op.cy) {
        (Some(x), Some(y)) => (x, y),
        _ => (crate::model3d::DEFAULT_CX, crate::model3d::DEFAULT_CY),
    };
    let x = op.x.unwrap_or(0);
    let y = op.y.unwrap_or(0);
    let (glb_rid, png_rid) = write_media(pkg, slide_part, glb)?;
    picture::ensure_xmlns(&mut slide_doc.root, "r", REL_NS);
    picture::ensure_xmlns(&mut slide_doc.root, "a", DML_NS);
    picture::ensure_xmlns(&mut slide_doc.root, "mc", MC_NS);
    let cnv = picture::next_cnv_id(&slide_doc.root);
    let name = format!("3D 模型 {cnv}");
    let ac = build_pptx_ac(cnv, &name, &glb_rid, &png_rid, x, y, cx, cy);
    picture::insert_into_sp_tree(&mut slide_doc.root, ac)?;
    Ok(())
}

fn write_media(
    pkg: &mut Package,
    host_part: &str,
    glb: Vec<u8>,
) -> Result<(String, String), Error> {
    let png_part = picture::next_media_part(pkg, "png");
    crate::model3d::write_glb_media(
        pkg,
        host_part,
        "/ppt/media",
        glb,
        png_part,
        picture::rel_target_from,
        picture::REL_IMAGE,
    )
}

fn build_pptx_ac(
    cnv: u32,
    name: &str,
    glb_rid: &str,
    png_rid: &str,
    x: i64,
    y: i64,
    cx: i64,
    cy: i64,
) -> Element {
    let mut ac = Element::new("mc:AlternateContent").with_attr("xmlns:mc", MC_NS);
    let mut choice = Element::new("mc:Choice")
        .with_attr("xmlns:am3d", crate::model3d::AM3D_NS)
        .with_attr("Requires", "am3d");
    let mut frame = Element::new("p:graphicFrame");
    let mut nv = Element::new("p:nvGraphicFramePr");
    nv.push_element(
        Element::new("p:cNvPr")
            .with_attr("id", cnv.to_string())
            .with_attr("name", name),
    );
    let mut locks = Element::new("p:cNvGraphicFramePr");
    locks.push_element(Element::new("a:graphicFrameLocks").with_attr("noChangeAspect", "1"));
    nv.push_element(locks);
    nv.push_element(Element::new("p:nvPr"));
    frame.push_element(nv);
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
    frame.push_element(xfrm);
    let mut graphic = Element::new("a:graphic");
    let mut data = Element::new("a:graphicData").with_attr("uri", crate::model3d::AM3D_NS);
    data.push_element(crate::model3d::build_model3d_el(glb_rid, png_rid, cx, cy));
    graphic.push_element(data);
    frame.push_element(graphic);
    choice.push_element(frame);
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

pub fn replace(
    pkg: &mut Package,
    slide_part: &str,
    slide_root: &Element,
    op: &ReplaceModel3dOp,
    op_index: usize,
) -> Result<(), Error> {
    let path = format!("ops[{op_index}].id");
    let glb =
        crate::model3d::read_glb(&op.file).map_err(|e| e.at(format!("ops[{op_index}].file")))?;
    let frame = find_frame(slide_root, &op.target.key).ok_or_else(|| {
        Error::with_path(
            ErrorKind::Model3dNotFound { id: op.id.clone() },
            path.clone(),
        )
    })?;
    let glb_rid = crate::model3d::glb_embed_rid(frame).ok_or_else(|| {
        Error::with_path(
            ErrorKind::Model3dNotFound { id: op.id.clone() },
            path.clone(),
        )
    })?;
    let png_rid = crate::model3d::raster_embed_rid(frame);
    let glb_part = resolve_id(pkg, slide_part, &glb_rid)?;
    pkg.set(&glb_part, glb)?;
    if let Some(rid) = png_rid {
        if let Ok(png_part) = resolve_id(pkg, slide_part, &rid) {
            pkg.set(&png_part, crate::ole::PREVIEW_PNG.to_vec())?;
        }
    }
    Ok(())
}

pub fn delete(
    pkg: &mut Package,
    slide_doc: &mut Document,
    slide_part: &str,
    op: &DeleteModel3dOp,
    op_index: usize,
) -> Result<(), Error> {
    let path = format!("ops[{op_index}].id");
    let frame = find_frame(&slide_doc.root, &op.target.key).ok_or_else(|| {
        Error::with_path(
            ErrorKind::Model3dNotFound { id: op.id.clone() },
            path.clone(),
        )
    })?;
    let spid = cnv_of(frame).0.filter(|s| !s.is_empty());
    let rids = crate::model3d::collect_embed_rids(frame);
    let mut old_parts = Vec::new();
    for rid in &rids {
        if let Ok(p) = resolve_id(pkg, slide_part, rid) {
            if !old_parts.iter().any(|x| x == &p) {
                old_parts.push(p);
            }
        }
    }
    if !remove_model3d(&mut slide_doc.root, &op.target.key) {
        return Err(Error::with_path(
            ErrorKind::Model3dNotFound { id: op.id.clone() },
            path,
        ));
    }
    if let Some(id) = spid {
        shape::strip_timing_for_spid(&mut slide_doc.root, &id);
    }
    let mut rels = Relationships::from_package(pkg, slide_part)?;
    let mut dirty = false;
    for rid in &rids {
        if !rid_still_used(&slide_doc.root, rid) {
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
        }
    }
    Ok(())
}

pub fn apply_size(slide_doc: &mut Document, op: &SetModel3dSizeOp, i: usize) -> Result<(), Error> {
    let path = format!("ops[{i}].id");
    let ac = find_ac_mut(&mut slide_doc.root, &op.target.key)
        .ok_or_else(|| Error::with_path(ErrorKind::Model3dNotFound { id: op.id.clone() }, path))?;
    patch_ac_geom(ac, op.cx, op.cy, None, None);
    Ok(())
}

pub fn apply_pos(slide_doc: &mut Document, op: &SetModel3dPosOp, i: usize) -> Result<(), Error> {
    let path = format!("ops[{i}].id");
    let ac = find_ac_mut(&mut slide_doc.root, &op.target.key)
        .ok_or_else(|| Error::with_path(ErrorKind::Model3dNotFound { id: op.id.clone() }, path))?;
    patch_ac_geom(ac, None, None, op.x, op.y);
    Ok(())
}

pub fn apply_camera(
    slide_doc: &mut Document,
    op: &SetModel3dCameraOp,
    i: usize,
) -> Result<(), Error> {
    let path = format!("ops[{i}].id");
    let ac = find_ac_mut(&mut slide_doc.root, &op.target.key)
        .ok_or_else(|| Error::with_path(ErrorKind::Model3dNotFound { id: op.id.clone() }, path))?;
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
        return Err(Error::with_path(
            ErrorKind::Model3dNotFound { id: op.id.clone() },
            format!("ops[{i}].id"),
        ));
    }
    Ok(())
}

fn find_ac_mut<'a>(root: &'a mut Element, key: &Model3dKey) -> Option<&'a mut Element> {
    find_ac_walk_mut(root, key, &mut 0)
}

fn find_ac_walk_mut<'a>(
    el: &'a mut Element,
    key: &Model3dKey,
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

fn ac_matches(ac: &Element, key: &Model3dKey, index: &mut usize) -> bool {
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
        if model_hit(frame, key, *index) {
            hit = true;
        }
    }
    hit
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
                if let Some(frame) = branch.child_by_local_mut("graphicFrame") {
                    patch_frame_xfrm(frame, cx, cy, x, y);
                    patch_am3d_xfrm(frame, cx, cy, x, y);
                }
            }
            "Fallback" => {
                if let Some(pic) = branch.child_by_local_mut("pic") {
                    patch_pic_xfrm(pic, cx, cy, x, y);
                }
            }
            _ => {}
        }
    }
}

fn patch_frame_xfrm(
    frame: &mut Element,
    cx: Option<i64>,
    cy: Option<i64>,
    x: Option<i64>,
    y: Option<i64>,
) {
    if frame.child_by_local("xfrm").is_none() {
        frame.push_element(Element::new("p:xfrm"));
    }
    let xfrm = frame.child_by_local_mut("xfrm").expect("刚插入的 p:xfrm");
    patch_off_ext(xfrm, cx, cy, x, y);
}

fn patch_pic_xfrm(
    pic: &mut Element,
    cx: Option<i64>,
    cy: Option<i64>,
    x: Option<i64>,
    y: Option<i64>,
) {
    let sp_pr = picture::ensure_child(pic, "spPr");
    let xfrm = picture::ensure_child(sp_pr, "xfrm");
    patch_off_ext(xfrm, cx, cy, x, y);
}

fn patch_am3d_xfrm(
    frame: &mut Element,
    cx: Option<i64>,
    cy: Option<i64>,
    x: Option<i64>,
    y: Option<i64>,
) {
    let Some(model) = find_local_mut(frame, "model3d") else {
        return;
    };
    let Some(sp) = model.child_by_local_mut("spPr") else {
        return;
    };
    let Some(xfrm) = sp.child_by_local_mut("xfrm") else {
        return;
    };
    patch_off_ext(xfrm, cx, cy, x, y);
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

fn find_local_mut<'a>(el: &'a mut Element, local: &str) -> Option<&'a mut Element> {
    crate::xml::find_by_local_mut(el, local)
}

fn remove_model3d(el: &mut Element, key: &Model3dKey) -> bool {
    remove_in(el, key, &mut 0)
}

fn remove_in(el: &mut Element, key: &Model3dKey, index: &mut usize) -> bool {
    let mut drop = None;
    for (i, n) in el.children.iter_mut().enumerate() {
        let Node::Element(child) = n else { continue };
        if child.local_name() == "AlternateContent" {
            let mut frames = Vec::new();
            collect_frames(child, &mut frames);
            let mut hit = false;
            for frame in frames {
                *index += 1;
                if model_hit(frame, key, *index) {
                    hit = true;
                }
            }
            if hit {
                drop = Some(i);
                break;
            }
        } else if child.local_name() == "graphicFrame" && crate::model3d::is_model3d_graphic(child)
        {
            *index += 1;
            if model_hit(child, key, *index) {
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

fn rid_still_used(root: &Element, rid: &str) -> bool {
    let mut used = false;
    crate::model3d::walk(root, &mut |e| {
        if e.attr_local("embed") == Some(rid) {
            used = true;
        }
    });
    used
}

pub(crate) fn parse_create(
    v: &Value,
    prefix: &str,
    slide: usize,
    on_notes: bool,
) -> Result<InsertModel3dOp, Error> {
    let obj = v.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: prefix.into(),
                expected: crate::error::TypeName::Object,
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
                    expected: crate::error::TypeName::NonEmptyString,
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
                    expected: crate::error::TypeName::String,
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
                op: "insertModel3d".into(),
                field: "cx and cy together".into(),
            },
            prefix.to_string(),
        ));
    }
    Ok(InsertModel3dOp {
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
                        expected: crate::error::TypeName::Integer,
                    },
                    format!("{prefix}.{field}"),
                )
            })
            .map(Some),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: crate::error::TypeName::Integer,
            },
            format!("{prefix}.{field}"),
        )),
    }
}
