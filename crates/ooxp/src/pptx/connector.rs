//! 幻灯片 `p:cxnSp`：inspect 名单、插删、改位置 / 尺寸 / 描边。

use serde_json::{json, Map, Value};

use crate::error::{Error, ErrorKind, IdReason};
use crate::xml::{Document, Element, Node};

use super::picture;
use super::request::{InsertConnectorOp, SetShapeLineOp, SetShapePosOp, SetShapeSizeOp};
use super::shape::{self, ShapeKey};
use super::DML_NS;

const DEFAULT_CX: i64 = 1_828_800;
const DEFAULT_CY: i64 = 762_000;
const LINE_W: i64 = 12_700;

#[derive(Debug, Clone)]
pub struct CxnTarget {
    pub slide: usize,
    pub on_notes: bool,
    pub master: Option<(usize, Option<usize>)>,
    pub key: ShapeKey,
}

pub fn parse_cxn_id(id: &str, op_index: usize) -> Result<CxnTarget, Error> {
    let path = format!("ops[{op_index}].id");
    let (host, after) = picture::parse_object_id(id, &path, "cxn", IdReason::SlideCxnForm)?;
    if after.is_empty() {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SlideCxnForm,
            },
            path,
        ));
    }
    let (slide, on_notes, master) = host.parts();
    if let Some(k) = after.strip_prefix("index:") {
        let idx: usize = k.parse().map_err(|_| {
            Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::ShapeIndexNotInt,
                },
                path.clone(),
            )
        })?;
        if idx < 1 {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::ShapeIndexFromOne,
                },
                path,
            ));
        }
        return Ok(CxnTarget {
            slide,
            on_notes,
            master,
            key: ShapeKey::Index(idx),
        });
    }
    Ok(CxnTarget {
        slide,
        on_notes,
        master,
        key: ShapeKey::Id(after.to_string()),
    })
}

pub fn parse_insert_slide_id(id: &str, op_index: usize) -> Result<picture::TreeHost, Error> {
    picture::parse_insert_host(id, op_index, IdReason::InsertConnectorNeedsSlide)
}

pub(crate) fn parse_slide_only(
    id: &str,
    op_index: usize,
    reason: IdReason,
) -> Result<usize, Error> {
    let path = format!("ops[{op_index}].id");
    let rest = id.strip_prefix("slide:").ok_or_else(|| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: reason.clone(),
            },
            path.clone(),
        )
    })?;
    if rest.is_empty() || rest.contains('/') {
        return Err(Error::with_path(ErrorKind::InvalidId { reason }, path));
    }
    let slide: usize = rest.parse().map_err(|_| {
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
    Ok(slide)
}

pub fn is_writable_prst(s: &str) -> bool {
    matches!(
        s,
        "straightConnector1"
            | "bentConnector2"
            | "bentConnector3"
            | "bentConnector4"
            | "bentConnector5"
            | "curvedConnector2"
            | "curvedConnector3"
            | "curvedConnector4"
            | "curvedConnector5"
    )
}

pub fn looks_like_cxn_id(id: &str) -> bool {
    (id.starts_with("slide:") || id.starts_with("master:")) && id.contains("/cxn:")
}

pub fn collect(root: &Element) -> Vec<&Element> {
    let mut out = Vec::new();
    shape::walk_choice(root, &mut |e| {
        if e.local_name() == "cxnSp" {
            out.push(e);
        }
    });
    out
}

pub fn count(root: &Element) -> usize {
    collect(root).len()
}

fn cnv_pr(el: &Element) -> Option<&Element> {
    el.child_by_local("nvCxnSpPr")
        .and_then(|nv| nv.child_by_local("cNvPr"))
}

fn stable_id(el: &Element, host: &str, index: usize) -> (String, Option<String>, bool) {
    if let Some(id) = cnv_pr(el)
        .and_then(|c| c.attr("id"))
        .filter(|s| !s.is_empty())
    {
        (format!("{host}/cxn:{id}"), Some(id.to_string()), true)
    } else {
        (format!("{host}/cxn:index:{index}"), None, false)
    }
}

fn node_json(el: &Element, host: &str, index: usize) -> Value {
    let (id, cnv, stable) = stable_id(el, host, index);
    let (x, y, cx, cy) = shape::geometry_json(el);
    let mut obj = Map::new();
    obj.insert("id".into(), json!(id));
    obj.insert("cNvPrId".into(), cnv.map_or(Value::Null, |s| json!(s)));
    obj.insert(
        "name".into(),
        json!(cnv_pr(el).and_then(|c| c.attr("name")).unwrap_or("")),
    );
    obj.insert("prst".into(), shape::prst_json(el));
    obj.insert("x".into(), x);
    obj.insert("y".into(), y);
    obj.insert("cx".into(), cx);
    obj.insert("cy".into(), cy);
    obj.insert("stable".into(), json!(stable));
    Value::Object(obj)
}

pub fn list_json(root: &Element, host: &str) -> Vec<Value> {
    collect(root)
        .into_iter()
        .enumerate()
        .map(|(i, el)| node_json(el, host, i + 1))
        .collect()
}

pub fn find_mut<'a>(root: &'a mut Element, key: &ShapeKey) -> Option<&'a mut Element> {
    let mut idx = 0usize;
    find_in(root, &mut idx, key)
}

fn find_in<'a>(el: &'a mut Element, idx: &mut usize, key: &ShapeKey) -> Option<&'a mut Element> {
    if el.local_name() == "AlternateContent" {
        if let Some(i) = shape::first_alt_child_index(el) {
            if let Node::Element(child) = &mut el.children[i] {
                return find_in(child, idx, key);
            }
        }
        return None;
    }
    if el.local_name() == "cxnSp" {
        *idx += 1;
        let hit = match key {
            ShapeKey::Index(n) => *idx == *n,
            ShapeKey::Id(req) => cnv_pr(el)
                .and_then(|c| c.attr("id"))
                .is_some_and(|id| shape::ids_match(id, req)),
        };
        if hit {
            return Some(el);
        }
        return None;
    }
    for child in el.children.as_mut_slice() {
        if let Node::Element(e) = child {
            if let Some(found) = find_in(e, idx, key) {
                return Some(found);
            }
        }
    }
    None
}

pub fn insert(doc: &mut Document, canvas: (i64, i64), op: &InsertConnectorOp) -> Result<(), Error> {
    let cx = op.cx.unwrap_or(DEFAULT_CX);
    let cy = op.cy.unwrap_or(DEFAULT_CY);
    let (x, y) = match (op.x, op.y) {
        (Some(px), Some(py)) => (px, py),
        _ => picture::vacant_pos_canvas(canvas, &doc.root, cx, cy),
    };
    picture::ensure_xmlns(&mut doc.root, "a", DML_NS);
    let cnv = picture::next_cnv_id(&doc.root);
    let name = format!("连接线 {cnv}");
    let cxn = build_cxn(cnv, &name, &op.prst, x, y, cx, cy);
    picture::insert_into_sp_tree(&mut doc.root, cxn)?;
    Ok(())
}

fn build_cxn(cnv: u32, name: &str, prst: &str, x: i64, y: i64, cx: i64, cy: i64) -> Element {
    let mut cxn = Element::new("p:cxnSp");
    let mut nv = Element::new("p:nvCxnSpPr");
    nv.push_element(
        Element::new("p:cNvPr")
            .with_attr("id", cnv.to_string())
            .with_attr("name", name),
    );
    nv.push_element(Element::new("p:cNvCxnSpPr"));
    nv.push_element(Element::new("p:nvPr"));
    cxn.push_element(nv);

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
    let mut geom = Element::new("a:prstGeom").with_attr("prst", prst);
    geom.push_element(Element::new("a:avLst"));
    sp_pr.push_element(geom);
    sp_pr.push_element(Element::new("a:noFill"));
    let mut ln = Element::new("a:ln").with_attr("w", LINE_W.to_string());
    let mut fill = Element::new("a:solidFill");
    fill.push_element(Element::new("a:srgbClr").with_attr("val", "000000"));
    ln.push_element(fill);
    sp_pr.push_element(ln);
    cxn.push_element(sp_pr);
    cxn
}

pub fn try_remove(root: &mut Element, key: &ShapeKey) -> Option<Option<String>> {
    let mut idx = 0usize;
    let mut spid = None;
    if !remove_in(root, &mut idx, key, &mut spid) {
        return None;
    }
    shape::prune_empty_groups(root);
    Some(spid)
}

fn remove_in(el: &mut Element, idx: &mut usize, key: &ShapeKey, spid: &mut Option<String>) -> bool {
    let mut i = 0;
    while i < el.children.len() {
        let hit_ac = match &el.children[i] {
            Node::Element(e) if e.local_name() == "AlternateContent" => {
                scan_alt_for_hit(e, idx, key)
            }
            _ => None,
        };
        if let Some(hit) = hit_ac {
            *spid = hit;
            el.children.remove(i);
            return true;
        }
        let is_hit = match &el.children[i] {
            Node::Element(e) if e.local_name() == "cxnSp" => {
                *idx += 1;
                if matches_key(e, *idx, key) {
                    *spid = cnv_pr(e)
                        .and_then(|c| c.attr("id"))
                        .filter(|s| !s.is_empty())
                        .map(str::to_string);
                    true
                } else {
                    false
                }
            }
            _ => false,
        };
        if is_hit {
            el.children.remove(i);
            return true;
        }
        if let Node::Element(e) = &mut el.children[i] {
            if e.local_name() != "AlternateContent" && remove_in(e, idx, key, spid) {
                return true;
            }
        }
        i += 1;
    }
    false
}

fn scan_alt_for_hit(ac: &Element, idx: &mut usize, key: &ShapeKey) -> Option<Option<String>> {
    let mut hit: Option<Option<String>> = None;
    shape::walk_choice(ac, &mut |e| {
        if hit.is_some() || e.local_name() != "cxnSp" {
            return;
        }
        *idx += 1;
        if matches_key(e, *idx, key) {
            hit = Some(
                cnv_pr(e)
                    .and_then(|c| c.attr("id"))
                    .filter(|s| !s.is_empty())
                    .map(str::to_string),
            );
        }
    });
    hit
}

fn matches_key(el: &Element, index: usize, key: &ShapeKey) -> bool {
    match key {
        ShapeKey::Index(n) => index == *n,
        ShapeKey::Id(req) => cnv_pr(el)
            .and_then(|c| c.attr("id"))
            .is_some_and(|id| shape::ids_match(id, req)),
    }
}

pub fn set_pos(doc: &mut Document, op: &SetShapePosOp, op_index: usize) -> Result<(), Error> {
    let target = op.cxn.as_ref().expect("连接线");
    let el = find_mut(&mut doc.root, &target.key).ok_or_else(|| {
        Error::with_path(
            ErrorKind::ConnectorNotFound { id: op.id.clone() },
            format!("ops[{op_index}].id"),
        )
    })?;
    shape::set_sp_pos(el, op.x, op.y);
    Ok(())
}

pub fn set_size(doc: &mut Document, op: &SetShapeSizeOp, op_index: usize) -> Result<(), Error> {
    let target = op.cxn.as_ref().expect("连接线");
    let el = find_mut(&mut doc.root, &target.key).ok_or_else(|| {
        Error::with_path(
            ErrorKind::ConnectorNotFound { id: op.id.clone() },
            format!("ops[{op_index}].id"),
        )
    })?;
    shape::set_sp_size(el, op.cx, op.cy);
    Ok(())
}

pub fn set_line(doc: &mut Document, op: &SetShapeLineOp, op_index: usize) -> Result<(), Error> {
    let target = op.cxn.as_ref().expect("连接线");
    let el = find_mut(&mut doc.root, &target.key).ok_or_else(|| {
        Error::with_path(
            ErrorKind::ConnectorNotFound { id: op.id.clone() },
            format!("ops[{op_index}].id"),
        )
    })?;
    shape::apply_line(el, op.color.as_deref(), op.width);
    Ok(())
}
