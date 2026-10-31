//! 幻灯片 `p:graphicFrame` 图示：名单、插删、改点与画布坐标。数据走 `crate::smartart`。

use serde_json::{json, Map, Value};

use crate::error::{Error, ErrorKind, IdReason};
use crate::opc::{rels_part_name, Package, Relationships};
use crate::smartart::{
    self, collect_blip_rids, colors_xml, missing_data, style_xml, Role, CT_COLORS, CT_LAYOUT,
    CT_STYLE, DEFAULT_CX, DEFAULT_CY, DGM_NS, REL_CS, REL_LO, REL_QS,
};
use crate::xml::{self, Document, Element, Node};

use super::picture;
use super::request::{
    AddSmartArtPointOp, DeleteSmartArtOp, DeleteSmartArtPointOp, InsertSmartArtOp,
    SetSmartArtColorsOp, SetSmartArtLayoutOp, SetSmartArtPosOp, SetSmartArtSizeOp,
    SetSmartArtStyleOp, SetSmartArtTextOp,
};
use super::shape;
use super::{DML_NS, REL_NS};

const DIAGRAMS_DIR: &str = "/ppt/diagrams";

#[derive(Debug, Clone)]
pub enum SmartArtKey {
    Id(String),
    Index(usize),
}

#[derive(Debug, Clone)]
pub struct SmartArtTarget {
    pub slide: usize,
    pub on_notes: bool,
    pub master: Option<(usize, Option<usize>)>,
    pub key: SmartArtKey,
}

#[derive(Debug, Clone)]
pub struct PointTarget {
    pub diagram: SmartArtTarget,
    pub model_id: String,
}

#[derive(Debug, Clone)]
pub enum AddParent {
    Diagram(SmartArtTarget),
    Point(PointTarget),
}

pub fn parse_diagram_id(id: &str, json_path: &str) -> Result<SmartArtTarget, Error> {
    let path = json_path.to_string();
    let (host, after) =
        picture::parse_object_id(id, json_path, "smartdiagram", IdReason::SlideSmartArtPrefix)?;
    if after.is_empty() {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SlideSmartArtContainer,
            },
            path,
        ));
    }
    let (slide, on_notes, master) = host.parts();
    if let Some(rest) = after.strip_prefix("index:") {
        let k: usize = rest.parse().map_err(|_| {
            Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::SmartArtIndexNotInt,
                },
                path.clone(),
            )
        })?;
        if k < 1 {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::SmartArtIndexFromOne,
                },
                path,
            ));
        }
        return Ok(SmartArtTarget {
            slide,
            on_notes,
            master,
            key: SmartArtKey::Index(k),
        });
    }
    if after.contains('/') {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SlideSmartArtPrefix,
            },
            path,
        ));
    }
    Ok(SmartArtTarget {
        slide,
        on_notes,
        master,
        key: SmartArtKey::Id(after.to_string()),
    })
}

pub fn parse_point_id(id: &str, json_path: &str) -> Result<PointTarget, Error> {
    let path = json_path.to_string();
    let Some((left, model_id)) = id.rsplit_once("/pt:") else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SmartArtPtPrefix,
            },
            path,
        ));
    };
    if model_id.is_empty() {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SmartArtPtPrefix,
            },
            path,
        ));
    }
    let diagram = parse_diagram_id(left, json_path)?;
    Ok(PointTarget {
        diagram,
        model_id: model_id.to_string(),
    })
}

pub fn parse_add_parent(id: &str, json_path: &str) -> Result<AddParent, Error> {
    if id.contains("/pt:") {
        Ok(AddParent::Point(parse_point_id(id, json_path)?))
    } else {
        Ok(AddParent::Diagram(parse_diagram_id(id, json_path)?))
    }
}

pub fn parse_insert_slide_id(id: &str, op_index: usize) -> Result<picture::TreeHost, Error> {
    picture::parse_insert_host(id, op_index, IdReason::InsertSmartArtNeedsSlide)
}

pub fn count_smartarts(root: &Element) -> usize {
    let mut n = 0usize;
    count_in(root, &mut n);
    n
}

fn count_in(el: &Element, n: &mut usize) {
    if el.local_name() == "AlternateContent" {
        if let Some(i) = shape::first_alt_child_index(el) {
            if let Node::Element(child) = &el.children[i] {
                count_in(child, n);
            }
        }
        return;
    }
    if is_diagram_frame(el) {
        *n += 1;
        return;
    }
    for child in &el.children {
        if let Node::Element(e) = child {
            count_in(e, n);
        }
    }
}

pub fn collect_smartarts(
    pkg: &Package,
    root: &Element,
    host: &str,
    slide_part: &str,
) -> Result<Vec<Value>, Error> {
    let mut out = Vec::new();
    let mut index = 0usize;
    collect_in(pkg, root, host, slide_part, &mut index, &mut out)?;
    Ok(out)
}

fn collect_in(
    pkg: &Package,
    el: &Element,
    host: &str,
    slide_part: &str,
    index: &mut usize,
    out: &mut Vec<Value>,
) -> Result<(), Error> {
    if el.local_name() == "AlternateContent" {
        if let Some(i) = shape::first_alt_child_index(el) {
            if let Node::Element(child) = &el.children[i] {
                return collect_in(pkg, child, host, slide_part, index, out);
            }
        }
        return Ok(());
    }
    if is_diagram_frame(el) {
        *index += 1;
        out.push(smartart_json(pkg, el, host, slide_part, *index)?);
        return Ok(());
    }
    for n in &el.children {
        if let Node::Element(e) = n {
            collect_in(pkg, e, host, slide_part, index, out)?;
        }
    }
    Ok(())
}

fn is_diagram_frame(el: &Element) -> bool {
    el.local_name() == "graphicFrame"
        && graphic_uri(el).is_some_and(|u| smartart::is_diagram_uri(&u))
}

fn graphic_uri(el: &Element) -> Option<String> {
    let mut found = None;
    shape::walk_choice(el, &mut |e| {
        if e.local_name() == "graphicData" && found.is_none() {
            found = e.attr_local("uri").map(str::to_string);
        }
    });
    found
}

fn cnv_of_frame(frame: &Element) -> (Option<String>, String) {
    let mut raw = None;
    let mut name = String::new();
    if let Some(nv) = frame.child_by_local("nvGraphicFramePr") {
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

fn parse_i64_attr(el: &Element, name: &str) -> Option<i64> {
    el.attr_local(name).and_then(|s| s.parse().ok())
}

fn frame_xfrm(frame: &Element) -> Option<&Element> {
    frame.child_by_local("xfrm")
}

fn smartart_json(
    pkg: &Package,
    frame: &Element,
    host: &str,
    slide_part: &str,
    index: usize,
) -> Result<Value, Error> {
    let (raw_id, name) = cnv_of_frame(frame);
    let stable = raw_id.is_some();
    let id = match &raw_id {
        Some(v) => format!("{host}/smartdiagram:{v}"),
        None => format!("{host}/smartdiagram:index:{index}"),
    };
    let xfrm = frame_xfrm(frame);
    let (x, y) = match xfrm.and_then(|x| x.child_by_local("off")) {
        Some(off) => (parse_i64_attr(off, "x"), parse_i64_attr(off, "y")),
        None => (None, None),
    };
    let (cx, cy) = match xfrm.and_then(|x| x.child_by_local("ext")) {
        Some(ext) => (parse_i64_attr(ext, "cx"), parse_i64_attr(ext, "cy")),
        None => (None, None),
    };
    let rids = smartart::rel_ids(frame);
    let layout = rids
        .lo
        .as_ref()
        .and_then(|rid| smartart::unique_id_of(pkg, slide_part, rid));
    let colors = rids
        .cs
        .as_ref()
        .and_then(|rid| smartart::unique_id_of(pkg, slide_part, rid));
    let style = rids
        .qs
        .as_ref()
        .and_then(|rid| smartart::unique_id_of(pkg, slide_part, rid));
    let points = match &rids.dm {
        Some(rid) => match crate::opc::resolve_id(pkg, slide_part, rid) {
            Ok(part) => match pkg.get(&part).ok().and_then(|b| xml::parse(b).ok()) {
                Some(doc) => smartart::points_json(&id, &doc.root),
                None => Vec::new(),
            },
            Err(_) => Vec::new(),
        },
        None => Vec::new(),
    };
    let mut m = Map::new();
    m.insert("id".into(), json!(id));
    m.insert("cNvPrId".into(), json!(raw_id));
    m.insert("name".into(), json!(name));
    m.insert("x".into(), json!(x));
    m.insert("y".into(), json!(y));
    m.insert("cx".into(), json!(cx));
    m.insert("cy".into(), json!(cy));
    m.insert("layout".into(), json!(layout));
    m.insert("colors".into(), json!(colors));
    m.insert("style".into(), json!(style));
    m.insert("points".into(), Value::Array(points));
    m.insert("stable".into(), json!(stable));
    Ok(Value::Object(m))
}

pub(crate) fn pin_smart(root: &mut Element) {
    crate::xml::pin_counted(root, "smartdiagram", is_diagram_frame, |_| false);
}

fn key_matches(frame: &Element, index: usize, key: &SmartArtKey) -> bool {
    match key {
        SmartArtKey::Index(k) => {
            crate::xml::pin_matches(frame, &crate::xml::pin_attr("smartdiagram"), *k, index)
        }
        SmartArtKey::Id(id) => cnv_of_frame(frame).0.as_deref() == Some(id.as_str()),
    }
}

pub(crate) fn find_frame_mut<'a>(
    root: &'a mut Element,
    key: &SmartArtKey,
) -> Option<&'a mut Element> {
    let mut idx = 0usize;
    find_frame_in(root, &mut idx, key)
}

fn find_frame_in<'a>(
    el: &'a mut Element,
    idx: &mut usize,
    key: &SmartArtKey,
) -> Option<&'a mut Element> {
    crate::xml::search_mut(el, |el| {
        if el.local_name() == "AlternateContent" {
            if let Some(i) = shape::first_alt_child_index(el) {
                if matches!(&el.children[i], Node::Element(_)) {
                    return crate::xml::SearchMut::Only(i);
                }
            }
            return crate::xml::SearchMut::Skip;
        }
        if is_diagram_frame(el) {
            *idx += 1;
            if key_matches(el, *idx, key) {
                return crate::xml::SearchMut::Hit;
            }
            return crate::xml::SearchMut::Skip;
        }
        crate::xml::SearchMut::down(el)
    })
}

fn find_frame_immut<'a>(root: &'a Element, key: &SmartArtKey) -> Option<&'a Element> {
    let mut idx = 0usize;
    find_frame_immut_in(root, &mut idx, key)
}

fn find_frame_immut_in<'a>(
    el: &'a Element,
    idx: &mut usize,
    key: &SmartArtKey,
) -> Option<&'a Element> {
    crate::xml::search(el, |el| {
        if el.local_name() == "AlternateContent" {
            if let Some(i) = shape::first_alt_child_index(el) {
                if let Node::Element(child) = &el.children[i] {
                    return crate::xml::Search::Only(child);
                }
            }
            return crate::xml::Search::Skip;
        }
        if is_diagram_frame(el) {
            *idx += 1;
            if key_matches(el, *idx, key) {
                return crate::xml::Search::Hit(el);
            }
            return crate::xml::Search::Skip;
        }
        crate::xml::Search::down(el)
    })
}

/// 图示在 Choice 里则整段 AlternateContent 删掉，否则只删 graphicFrame。
fn remove_smartart(root: &mut Element, key: &SmartArtKey) -> bool {
    let mut idx = 0usize;
    remove_in(root, &mut idx, key)
}

fn remove_in(el: &mut Element, idx: &mut usize, key: &SmartArtKey) -> bool {
    let mut i = 0;
    while i < el.children.len() {
        let is_ac =
            matches!(&el.children[i], Node::Element(e) if e.local_name() == "AlternateContent");
        if is_ac {
            let mut found = false;
            if let Node::Element(ac) = &el.children[i] {
                peek_choice(ac, idx, key, &mut found);
            }
            if found {
                el.children.remove(i);
                return true;
            }
            i += 1;
            continue;
        }
        let is_hit = match &el.children[i] {
            Node::Element(e) if is_diagram_frame(e) => {
                *idx += 1;
                key_matches(e, *idx, key)
            }
            _ => false,
        };
        if is_hit {
            el.children.remove(i);
            return true;
        }
        if let Node::Element(e) = &mut el.children[i] {
            if !is_diagram_frame(e) && remove_in(e, idx, key) {
                return true;
            }
        }
        i += 1;
    }
    false
}

fn peek_choice(ac: &Element, idx: &mut usize, key: &SmartArtKey, found: &mut bool) {
    if let Some(i) = shape::first_alt_child_index(ac) {
        if let Node::Element(child) = &ac.children[i] {
            mark_if_match(child, idx, key, found);
        }
    }
}

fn mark_if_match(el: &Element, idx: &mut usize, key: &SmartArtKey, found: &mut bool) {
    if el.local_name() == "AlternateContent" {
        peek_choice(el, idx, key, found);
        return;
    }
    if is_diagram_frame(el) {
        *idx += 1;
        if key_matches(el, *idx, key) {
            *found = true;
        }
        return;
    }
    for n in &el.children {
        if let Node::Element(e) = n {
            mark_if_match(e, idx, key, found);
        }
    }
}

fn find_removal_el<'a>(root: &'a Element, key: &SmartArtKey) -> Option<&'a Element> {
    let mut idx = 0usize;
    find_removal_in(root, &mut idx, key)
}

fn find_removal_in<'a>(el: &'a Element, idx: &mut usize, key: &SmartArtKey) -> Option<&'a Element> {
    let mut stack: Vec<&Element> = el.element_children().collect();
    stack.reverse();
    while let Some(e) = stack.pop() {
        if e.local_name() == "AlternateContent" {
            let mut found = false;
            peek_choice(e, idx, key, &mut found);
            if found {
                return Some(e);
            }
            continue;
        }
        if is_diagram_frame(e) {
            *idx += 1;
            if key_matches(e, *idx, key) {
                return Some(e);
            }
            continue;
        }
        let kids: Vec<&Element> = e.element_children().collect();
        for child in kids.into_iter().rev() {
            stack.push(child);
        }
    }
    None
}

fn rid_still_on_slide(root: &Element, rid: &str) -> bool {
    let mut used = false;
    shape::walk_choice(root, &mut |e| {
        if used {
            return;
        }
        if e.local_name() != "relIds" {
            return;
        }
        for local in ["dm", "lo", "qs", "cs"] {
            if e.attr_local(local) == Some(rid) {
                used = true;
                return;
            }
        }
    });
    used
}

fn not_found(id: &str, path: String) -> Error {
    Error::with_path(ErrorKind::SmartArtNotFound { id: id.to_string() }, path)
}

fn data_part_of(
    pkg: &Package,
    slide_root: &Element,
    slide_part: &str,
    key: &SmartArtKey,
    id: &str,
    i: usize,
) -> Result<String, Error> {
    let frame =
        find_frame_immut(slide_root, key).ok_or_else(|| not_found(id, format!("ops[{i}].id")))?;
    let rid = smartart::rel_ids(frame)
        .dm
        .ok_or_else(|| missing_data(id, i))?;
    crate::opc::resolve_id(pkg, slide_part, &rid).map_err(|_| missing_data(id, i))
}

fn layout_unique_id(
    pkg: &Package,
    slide_root: &Element,
    slide_part: &str,
    key: &SmartArtKey,
    id: &str,
    i: usize,
) -> Result<String, Error> {
    let frame =
        find_frame_immut(slide_root, key).ok_or_else(|| not_found(id, format!("ops[{i}].id")))?;
    let rid = smartart::rel_ids(frame).lo.ok_or_else(|| {
        Error::with_path(
            ErrorKind::SmartArtLayoutNotConvertible,
            format!("ops[{i}].id"),
        )
    })?;
    smartart::unique_id_of(pkg, slide_part, &rid).ok_or_else(|| {
        Error::with_path(
            ErrorKind::SmartArtLayoutNotConvertible,
            format!("ops[{i}].id"),
        )
    })
}

pub fn insert_smartart(
    pkg: &mut Package,
    slide_doc: &mut Document,
    slide_part: &str,
    pres_root: &Element,
    op: &InsertSmartArtOp,
    _i: usize,
) -> Result<(), Error> {
    let rids = smartart::write_four_parts(
        pkg,
        slide_part,
        DIAGRAMS_DIR,
        op.layout,
        op.points.as_deref(),
    )?;
    let (cx, cy) = (op.cx.unwrap_or(DEFAULT_CX), op.cy.unwrap_or(DEFAULT_CY));
    let (x, y) = match (op.x, op.y) {
        (Some(px), Some(py)) => (px, py),
        _ => {
            if op.on_notes {
                picture::vacant_pos_canvas(
                    super::notes::notes_size(pres_root),
                    &slide_doc.root,
                    cx,
                    cy,
                )
            } else {
                picture::vacant_pos(pres_root, &slide_doc.root, cx, cy)
            }
        }
    };
    picture::ensure_xmlns(&mut slide_doc.root, "r", REL_NS);
    picture::ensure_xmlns(&mut slide_doc.root, "a", DML_NS);
    picture::ensure_xmlns(&mut slide_doc.root, "dgm", DGM_NS);
    let cnv = picture::next_cnv_id(&slide_doc.root);
    let frame = build_graphic_frame(
        cnv,
        &format!("SmartArt {cnv}"),
        rids.dm.as_deref().unwrap_or(""),
        rids.lo.as_deref().unwrap_or(""),
        rids.qs.as_deref().unwrap_or(""),
        rids.cs.as_deref().unwrap_or(""),
        x,
        y,
        cx,
        cy,
    );
    picture::insert_into_sp_tree(&mut slide_doc.root, frame)?;
    Ok(())
}

fn build_graphic_frame(
    cnv: u32,
    name: &str,
    dm: &str,
    lo: &str,
    qs: &str,
    cs: &str,
    x: i64,
    y: i64,
    cx: i64,
    cy: i64,
) -> Element {
    let mut nv = Element::new("p:nvGraphicFramePr");
    nv.push_element(
        Element::new("p:cNvPr")
            .with_attr("id", cnv.to_string())
            .with_attr("name", name),
    );
    let mut locks = Element::new("p:cNvGraphicFramePr");
    locks.push_element(Element::new("a:graphicFrameLocks").with_attr("noGrp", "1"));
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
    let mut data = Element::new("a:graphicData").with_attr("uri", DGM_NS);
    data.push_element(
        Element::new("dgm:relIds")
            .with_attr("xmlns:dgm", DGM_NS)
            .with_attr("xmlns:r", REL_NS)
            .with_attr("r:dm", dm)
            .with_attr("r:lo", lo)
            .with_attr("r:qs", qs)
            .with_attr("r:cs", cs),
    );
    let mut graphic = Element::new("a:graphic");
    graphic.push_element(data);
    let mut frame = Element::new("p:graphicFrame");
    frame.push_element(nv);
    frame.push_element(xfrm);
    frame.push_element(graphic);
    frame
}

pub fn delete_smartart(
    pkg: &mut Package,
    slide_doc: &mut Document,
    slide_part: &str,
    op: &DeleteSmartArtOp,
    i: usize,
) -> Result<(), Error> {
    let path = format!("ops[{i}].id");
    let (rids, parts, blips) = {
        let removal = find_removal_el(&slide_doc.root, &op.target.key)
            .ok_or_else(|| not_found(&op.id, path.clone()))?;
        let frame = find_frame_immut(&slide_doc.root, &op.target.key)
            .ok_or_else(|| not_found(&op.id, path.clone()))?;
        let rids = smartart::rel_ids(frame);
        let mut parts = Vec::new();
        for rid in rids.iter().flatten() {
            if let Ok(p) = crate::opc::resolve_id(pkg, slide_part, rid) {
                parts.push(p);
            }
        }
        if let Some(dm) = rids.dm.as_ref() {
            if let Ok(data) = crate::opc::resolve_id(pkg, slide_part, dm) {
                parts.extend(smartart::collect_drawing_caches(pkg, slide_part, &data)?);
            }
        }
        let blips = collect_blip_rids(removal);
        (rids, parts, blips)
    };
    let mut media = Vec::new();
    for rid in &blips {
        if let Ok(m) = crate::opc::resolve_id(pkg, slide_part, rid) {
            media.push((rid.clone(), m));
        }
    }
    if !remove_smartart(&mut slide_doc.root, &op.target.key) {
        return Err(not_found(&op.id, path));
    }
    let mut slide_rels = Relationships::from_package(pkg, slide_part)?;
    let mut rels_dirty = false;
    for rid in rids.iter().flatten() {
        if !rid_still_on_slide(&slide_doc.root, rid) {
            slide_rels.remove_by_id(rid);
            rels_dirty = true;
        }
    }
    for (rid, _) in &media {
        if !blip_still_on_slide(&slide_doc.root, rid) {
            slide_rels.remove_by_id(rid);
            rels_dirty = true;
        }
    }
    if rels_dirty {
        if slide_rels.rels.is_empty() {
            pkg.remove(&rels_part_name(slide_part));
        } else {
            slide_rels.write_to(pkg, slide_part)?;
        }
    }
    for p in &parts {
        smartart::drop_unused(pkg, p)?;
    }
    for p in &parts {
        smartart::drop_unused(pkg, p)?;
    }
    for (_, m) in media {
        if !picture::media_still_used(pkg, &m)? {
            pkg.remove(&m);
        }
    }
    Ok(())
}

fn blip_still_on_slide(root: &Element, rid: &str) -> bool {
    let mut used = false;
    shape::walk_choice(root, &mut |e| {
        if used {
            return;
        }
        if e.local_name() == "blip" && e.attr_local("embed") == Some(rid) {
            used = true;
        }
    });
    used
}

pub fn apply_text(
    pkg: &mut Package,
    slide_root: &Element,
    slide_part: &str,
    op: &SetSmartArtTextOp,
    i: usize,
) -> Result<(), Error> {
    let data_part = data_part_of(
        pkg,
        slide_root,
        slide_part,
        &op.target.diagram.key,
        &op.id,
        i,
    )?;
    let mut data = super::list_pin::parse_part(pkg, &data_part)?;
    let pt =
        smartart::find_visible_pt_mut(&mut data.root, &op.target.model_id).ok_or_else(|| {
            Error::with_path(
                ErrorKind::SmartArtPointNotFound { id: op.id.clone() },
                format!("ops[{i}].id"),
            )
        })?;
    smartart::set_point_text(pt, &op.text);
    super::list_pin::write_part(pkg, &data_part, &data)?;
    if !smartart::sync_drawing_text(pkg, slide_part, &data_part, &op.target.model_id, &op.text)? {
        smartart::drop_drawing_caches(pkg, slide_part, &data_part)?;
    }
    Ok(())
}

pub fn apply_add_point(
    pkg: &mut Package,
    slide_root: &Element,
    slide_part: &str,
    op: &AddSmartArtPointOp,
    i: usize,
) -> Result<(), Error> {
    let (diagram, parent_model, id) = match &op.parent {
        AddParent::Diagram(t) => (t.clone(), None, op.id.clone()),
        AddParent::Point(p) => (p.diagram.clone(), Some(p.model_id.clone()), op.id.clone()),
    };
    let layout_uid = layout_unique_id(pkg, slide_root, slide_part, &diagram.key, &id, i).ok();
    if smartart::family_of(layout_uid.as_deref().unwrap_or("")).is_some_and(|k| k.is_flat()) {
        if op.role == Role::Asst {
            return Err(Error::with_path(
                ErrorKind::SmartArtAsstOnList,
                format!("ops[{i}].role"),
            ));
        }
        if parent_model.is_some() {
            return Err(Error::with_path(
                ErrorKind::SmartArtListDepth,
                format!("ops[{i}].id"),
            ));
        }
    }
    let data_part = data_part_of(pkg, slide_root, slide_part, &diagram.key, &id, i)?;
    let mut data = super::list_pin::parse_part(pkg, &data_part)?;
    let parent_id = match &parent_model {
        Some(mid) => {
            if smartart::find_visible_pt(&data.root, mid).is_none() {
                return Err(Error::with_path(
                    ErrorKind::SmartArtPointNotFound { id: id.clone() },
                    format!("ops[{i}].id"),
                ));
            }
            mid.clone()
        }
        None => smartart::ensure_doc_pt(&mut data.root),
    };
    let (pt_id, cxn_id) = smartart::next_model_ids(&data.root);
    smartart::append_visible_pt(&mut data.root, &pt_id, op.role, &op.text);
    smartart::append_par_of(&mut data.root, &cxn_id, &parent_id, &pt_id);
    super::list_pin::write_part(pkg, &data_part, &data)?;
    smartart::drop_drawing_caches(pkg, slide_part, &data_part)?;
    Ok(())
}

pub fn apply_delete_point(
    pkg: &mut Package,
    slide_root: &Element,
    slide_part: &str,
    op: &DeleteSmartArtPointOp,
    i: usize,
) -> Result<(), Error> {
    let data_part = data_part_of(
        pkg,
        slide_root,
        slide_part,
        &op.target.diagram.key,
        &op.id,
        i,
    )?;
    let mut data = super::list_pin::parse_part(pkg, &data_part)?;
    if smartart::find_visible_pt(&data.root, &op.target.model_id).is_none() {
        return Err(Error::with_path(
            ErrorKind::SmartArtPointNotFound { id: op.id.clone() },
            format!("ops[{i}].id"),
        ));
    }
    let visible = smartart::visible_model_ids(&data.root);
    let doomed = smartart::descendants_of(&data.root, &op.target.model_id);
    if doomed.iter().filter(|id| visible.contains(*id)).count() >= visible.len() {
        return Err(Error::with_path(
            ErrorKind::LastSmartArtPoint,
            format!("ops[{i}].id"),
        ));
    }
    smartart::delete_points_and_cxns(&mut data.root, &doomed);
    super::list_pin::write_part(pkg, &data_part, &data)?;
    smartart::drop_drawing_caches(pkg, slide_part, &data_part)?;
    Ok(())
}

pub fn apply_size(slide_doc: &mut Document, op: &SetSmartArtSizeOp, i: usize) -> Result<(), Error> {
    let path = format!("ops[{i}].id");
    let frame = find_frame_mut(&mut slide_doc.root, &op.target.key)
        .ok_or_else(|| not_found(&op.id, path))?;
    let xfrm = ensure_frame_xfrm(frame);
    let ext = ensure_a(xfrm, "ext");
    if let Some(cx) = op.cx {
        picture::set_attr_local(ext, "cx", &cx.to_string());
    }
    if let Some(cy) = op.cy {
        picture::set_attr_local(ext, "cy", &cy.to_string());
    }
    Ok(())
}

pub fn apply_pos(slide_doc: &mut Document, op: &SetSmartArtPosOp, i: usize) -> Result<(), Error> {
    let path = format!("ops[{i}].id");
    let frame = find_frame_mut(&mut slide_doc.root, &op.target.key)
        .ok_or_else(|| not_found(&op.id, path))?;
    let xfrm = ensure_frame_xfrm(frame);
    let off = ensure_a(xfrm, "off");
    if let Some(x) = op.x {
        picture::set_attr_local(off, "x", &x.to_string());
    }
    if let Some(y) = op.y {
        picture::set_attr_local(off, "y", &y.to_string());
    }
    Ok(())
}

fn ensure_frame_xfrm(frame: &mut Element) -> &mut Element {
    if frame.child_by_local("xfrm").is_none() {
        frame.push_element(Element::new("p:xfrm"));
    }
    frame.child_by_local_mut("xfrm").expect("刚插入的 p:xfrm")
}

fn ensure_a<'a>(parent: &'a mut Element, local: &str) -> &'a mut Element {
    if parent.child_by_local(local).is_none() {
        parent.push_element(Element::new(format!("a:{local}")));
    }
    parent.child_by_local_mut(local).expect("刚插入的 a 子元素")
}

pub fn apply_colors(
    pkg: &mut Package,
    slide_doc: &mut Document,
    slide_part: &str,
    op: &SetSmartArtColorsOp,
    i: usize,
) -> Result<(), Error> {
    retarget(
        pkg,
        slide_doc,
        slide_part,
        &op.target,
        &op.id,
        i,
        "cs",
        REL_CS,
        CT_COLORS,
        "colors",
        &op.colors.unique_id(),
        &colors_xml(op.colors),
    )
}

pub fn apply_style(
    pkg: &mut Package,
    slide_doc: &mut Document,
    slide_part: &str,
    op: &SetSmartArtStyleOp,
    i: usize,
) -> Result<(), Error> {
    retarget(
        pkg,
        slide_doc,
        slide_part,
        &op.target,
        &op.id,
        i,
        "qs",
        REL_QS,
        CT_STYLE,
        "quickStyle",
        op.style.unique_id(),
        &style_xml(op.style),
    )
}

pub fn apply_layout(
    pkg: &mut Package,
    slide_doc: &mut Document,
    slide_part: &str,
    op: &SetSmartArtLayoutOp,
    i: usize,
) -> Result<(), Error> {
    let current = layout_unique_id(pkg, &slide_doc.root, slide_part, &op.target.key, &op.id, i)?;
    if current == op.layout.unique_id() {
        return Ok(());
    }
    if smartart::family_of(&current).is_none() {
        return Err(Error::with_path(
            ErrorKind::SmartArtLayoutNotConvertible,
            format!("ops[{i}].layout"),
        ));
    }
    retarget(
        pkg,
        slide_doc,
        slide_part,
        &op.target,
        &op.id,
        i,
        "lo",
        REL_LO,
        CT_LAYOUT,
        "layout",
        op.layout.unique_id(),
        op.layout.layout_xml(),
    )?;
    if let Ok(data_part) = data_part_of(pkg, &slide_doc.root, slide_part, &op.target.key, &op.id, i)
    {
        if op.layout.is_flat() {
            let mut data = super::list_pin::parse_part(pkg, &data_part)?;
            smartart::demote_asst(&mut data.root);
            super::list_pin::write_part(pkg, &data_part, &data)?;
        }
        smartart::drop_drawing_caches(pkg, slide_part, &data_part)?;
    }
    Ok(())
}

fn retarget(
    pkg: &mut Package,
    slide_doc: &mut Document,
    slide_part: &str,
    target: &SmartArtTarget,
    id: &str,
    i: usize,
    attr: &str,
    rel_type: &str,
    ct: &str,
    stem: &str,
    unique_id: &str,
    xml_body: &str,
) -> Result<(), Error> {
    let leftover = {
        let frame = find_frame_mut(&mut slide_doc.root, &target.key)
            .ok_or_else(|| not_found(id, format!("ops[{i}].id")))?;
        smartart::retarget_satellite(
            pkg,
            slide_part,
            DIAGRAMS_DIR,
            frame,
            attr,
            rel_type,
            ct,
            stem,
            unique_id,
            xml_body,
        )?
    };
    if let Some((old_rid, old_part)) = leftover {
        smartart::drop_replaced_satellite(pkg, slide_part, &slide_doc.root, &old_rid, &old_part)?;
    }
    Ok(())
}

pub(crate) fn op_insert_smart_art(
    pkg: &mut crate::opc::Package,
    pres_part: &str,
    pres: &mut crate::xml::Document,
    slides: &[super::shape::SlideMeta],
    op: &super::request::InsertSmartArtOp,
    i: usize,
) -> Result<(), crate::error::Error> {
    let part = super::apply::ensure_notes_or_slide(
        pkg,
        pres_part,
        pres,
        slides,
        op.slide,
        op.on_notes,
        op.master,
        format!("ops[{i}].id"),
    )?;
    let mut doc = super::list_pin::parse_part(pkg, &part)?;
    insert_smartart(pkg, &mut doc, &part, &pres.root, op, i)?;
    super::list_pin::write_part(pkg, &part, &doc)?;
    Ok(())
}

pub(crate) fn op_delete_smart_art(
    pkg: &mut crate::opc::Package,
    pres_part: &str,
    pres: &crate::xml::Document,
    slides: &[super::shape::SlideMeta],
    op: &super::request::DeleteSmartArtOp,
    i: usize,
) -> Result<(), crate::error::Error> {
    let part = super::apply::notes_or_slide_part(
        pkg,
        pres_part,
        &pres.root,
        slides,
        op.target.slide,
        op.target.on_notes,
        op.target.master,
        ErrorKind::SmartArtNotFound { id: op.id.clone() },
        format!("ops[{i}].id"),
    )?;
    let mut doc = super::list_pin::parse_part(pkg, &part)?;
    delete_smartart(pkg, &mut doc, &part, op, i)?;
    super::list_pin::write_part(pkg, &part, &doc)?;
    Ok(())
}

pub(crate) fn op_set_smart_art_text(
    pkg: &mut crate::opc::Package,
    pres_part: &str,
    pres: &crate::xml::Document,
    slides: &[super::shape::SlideMeta],
    op: &super::request::SetSmartArtTextOp,
    i: usize,
) -> Result<(), crate::error::Error> {
    let part = super::apply::notes_or_slide_part(
        pkg,
        pres_part,
        &pres.root,
        slides,
        op.target.diagram.slide,
        op.target.diagram.on_notes,
        op.target.diagram.master,
        ErrorKind::SmartArtNotFound { id: op.id.clone() },
        format!("ops[{i}].id"),
    )?;
    let doc = super::list_pin::parse_part(pkg, &part)?;
    apply_text(pkg, &doc.root, &part, op, i)?;
    Ok(())
}

pub(crate) fn op_add_smart_art_point(
    pkg: &mut crate::opc::Package,
    pres_part: &str,
    pres: &crate::xml::Document,
    slides: &[super::shape::SlideMeta],
    op: &super::request::AddSmartArtPointOp,
    i: usize,
) -> Result<(), crate::error::Error> {
    let (slide, on_notes, master) = match &op.parent {
        AddParent::Diagram(t) => (t.slide, t.on_notes, t.master),
        AddParent::Point(p) => (p.diagram.slide, p.diagram.on_notes, p.diagram.master),
    };
    let part = super::apply::notes_or_slide_part(
        pkg,
        pres_part,
        &pres.root,
        slides,
        slide,
        on_notes,
        master,
        ErrorKind::SmartArtNotFound { id: op.id.clone() },
        format!("ops[{i}].id"),
    )?;
    let doc = super::list_pin::parse_part(pkg, &part)?;
    apply_add_point(pkg, &doc.root, &part, op, i)?;
    Ok(())
}

pub(crate) fn op_delete_smart_art_point(
    pkg: &mut crate::opc::Package,
    pres_part: &str,
    pres: &crate::xml::Document,
    slides: &[super::shape::SlideMeta],
    op: &super::request::DeleteSmartArtPointOp,
    i: usize,
) -> Result<(), crate::error::Error> {
    let part = super::apply::notes_or_slide_part(
        pkg,
        pres_part,
        &pres.root,
        slides,
        op.target.diagram.slide,
        op.target.diagram.on_notes,
        op.target.diagram.master,
        ErrorKind::SmartArtNotFound { id: op.id.clone() },
        format!("ops[{i}].id"),
    )?;
    let doc = super::list_pin::parse_part(pkg, &part)?;
    apply_delete_point(pkg, &doc.root, &part, op, i)?;
    Ok(())
}

pub(crate) fn op_set_smart_art_size(
    pkg: &mut crate::opc::Package,
    pres_part: &str,
    pres: &crate::xml::Document,
    slides: &[super::shape::SlideMeta],
    op: &super::request::SetSmartArtSizeOp,
    i: usize,
) -> Result<(), crate::error::Error> {
    let part = super::apply::notes_or_slide_part(
        pkg,
        pres_part,
        &pres.root,
        slides,
        op.target.slide,
        op.target.on_notes,
        op.target.master,
        ErrorKind::SmartArtNotFound { id: op.id.clone() },
        format!("ops[{i}].id"),
    )?;
    let mut doc = super::list_pin::parse_part(pkg, &part)?;
    apply_size(&mut doc, op, i)?;
    super::list_pin::write_part(pkg, &part, &doc)?;
    Ok(())
}

pub(crate) fn op_set_smart_art_pos(
    pkg: &mut crate::opc::Package,
    pres_part: &str,
    pres: &crate::xml::Document,
    slides: &[super::shape::SlideMeta],
    op: &super::request::SetSmartArtPosOp,
    i: usize,
) -> Result<(), crate::error::Error> {
    let part = super::apply::notes_or_slide_part(
        pkg,
        pres_part,
        &pres.root,
        slides,
        op.target.slide,
        op.target.on_notes,
        op.target.master,
        ErrorKind::SmartArtNotFound { id: op.id.clone() },
        format!("ops[{i}].id"),
    )?;
    let mut doc = super::list_pin::parse_part(pkg, &part)?;
    apply_pos(&mut doc, op, i)?;
    super::list_pin::write_part(pkg, &part, &doc)?;
    Ok(())
}

pub(crate) fn op_set_smart_art_colors(
    pkg: &mut crate::opc::Package,
    pres_part: &str,
    pres: &crate::xml::Document,
    slides: &[super::shape::SlideMeta],
    op: &super::request::SetSmartArtColorsOp,
    i: usize,
) -> Result<(), crate::error::Error> {
    let part = super::apply::notes_or_slide_part(
        pkg,
        pres_part,
        &pres.root,
        slides,
        op.target.slide,
        op.target.on_notes,
        op.target.master,
        ErrorKind::SmartArtNotFound { id: op.id.clone() },
        format!("ops[{i}].id"),
    )?;
    let mut doc = super::list_pin::parse_part(pkg, &part)?;
    apply_colors(pkg, &mut doc, &part, op, i)?;
    super::list_pin::write_part(pkg, &part, &doc)?;
    Ok(())
}

pub(crate) fn op_set_smart_art_style(
    pkg: &mut crate::opc::Package,
    pres_part: &str,
    pres: &crate::xml::Document,
    slides: &[super::shape::SlideMeta],
    op: &super::request::SetSmartArtStyleOp,
    i: usize,
) -> Result<(), crate::error::Error> {
    let part = super::apply::notes_or_slide_part(
        pkg,
        pres_part,
        &pres.root,
        slides,
        op.target.slide,
        op.target.on_notes,
        op.target.master,
        ErrorKind::SmartArtNotFound { id: op.id.clone() },
        format!("ops[{i}].id"),
    )?;
    let mut doc = super::list_pin::parse_part(pkg, &part)?;
    apply_style(pkg, &mut doc, &part, op, i)?;
    super::list_pin::write_part(pkg, &part, &doc)?;
    Ok(())
}

pub(crate) fn op_set_smart_art_layout(
    pkg: &mut crate::opc::Package,
    pres_part: &str,
    pres: &crate::xml::Document,
    slides: &[super::shape::SlideMeta],
    op: &super::request::SetSmartArtLayoutOp,
    i: usize,
) -> Result<(), crate::error::Error> {
    let part = super::apply::notes_or_slide_part(
        pkg,
        pres_part,
        &pres.root,
        slides,
        op.target.slide,
        op.target.on_notes,
        op.target.master,
        ErrorKind::SmartArtNotFound { id: op.id.clone() },
        format!("ops[{i}].id"),
    )?;
    let mut doc = super::list_pin::parse_part(pkg, &part)?;
    apply_layout(pkg, &mut doc, &part, op, i)?;
    super::list_pin::write_part(pkg, &part, &doc)?;
    Ok(())
}
