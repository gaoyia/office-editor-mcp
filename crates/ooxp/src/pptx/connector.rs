//! 幻灯片 `p:cxnSp`：inspect 名单、插删、改位置 / 尺寸 / 描边。

use serde_json::{json, Map, Value};

use crate::error::{Error, ErrorKind, IdReason};
use crate::xml::{Document, Element, Node};

use super::picture;
use super::request::{
    InsertConnectorOp, SetConnectorOp, SetShapeLineOp, SetShapePosOp, SetShapeSizeOp,
};
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

#[derive(Debug, Clone)]
pub struct CxnBind {
    pub id: String,
    pub slide: usize,
    pub on_notes: bool,
    pub master: Option<(usize, Option<usize>)>,
    pub key: ShapeKey,
    pub is_cxn: bool,
    pub site: u32,
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

fn node_json(el: &Element, root: &Element, host: &str, index: usize) -> Value {
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
    if let Some(start) = bind_json(el, root, host, "stCxn") {
        obj.insert("start".into(), start);
    }
    if let Some(end) = bind_json(el, root, host, "endCxn") {
        obj.insert("end".into(), end);
    }
    obj.insert("x".into(), x);
    obj.insert("y".into(), y);
    obj.insert("cx".into(), cx);
    obj.insert("cy".into(), cy);
    obj.insert("stable".into(), json!(stable));
    Value::Object(obj)
}

fn bind_json(el: &Element, root: &Element, host: &str, local: &str) -> Option<Value> {
    let node = el
        .child_by_local("nvCxnSpPr")
        .and_then(|nv| nv.child_by_local("cNvCxnSpPr"))
        .and_then(|pr| pr.child_by_local(local))?;
    let cnv = node.attr_local("id").filter(|s| !s.is_empty())?;
    let idx = node
        .attr_local("idx")
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(0);
    let mut obj = Map::new();
    obj.insert("id".into(), json!(inspect_id_for_cnv(root, host, cnv)));
    obj.insert("site".into(), json!(idx + 1));
    Some(Value::Object(obj))
}

fn inspect_id_for_cnv(root: &Element, host: &str, cnv: &str) -> String {
    for (i, sp) in shape::collect_text_shapes(root).into_iter().enumerate() {
        if let Some(id) = shape::cnv_pr(sp).and_then(|c| c.attr("id")) {
            if shape::ids_match(id, cnv) {
                let (inspect, _, _) = shape::shape_stable_id(sp, host, i + 1);
                return inspect;
            }
        }
    }
    for (i, cxn) in collect(root).into_iter().enumerate() {
        if let Some(id) = cnv_pr(cxn).and_then(|c| c.attr("id")) {
            if shape::ids_match(id, cnv) {
                return if id.is_empty() {
                    format!("{host}/cxn:index:{}", i + 1)
                } else {
                    format!("{host}/cxn:{id}")
                };
            }
        }
    }
    format!("{host}/shape:{cnv}")
}

pub fn list_json(root: &Element, host: &str) -> Vec<Value> {
    collect(root)
        .into_iter()
        .enumerate()
        .map(|(i, el)| node_json(el, root, host, i + 1))
        .collect()
}

pub fn find_mut<'a>(root: &'a mut Element, key: &ShapeKey) -> Option<&'a mut Element> {
    let mut idx = 0usize;
    find_in(root, &mut idx, key)
}

fn find_in<'a>(el: &'a mut Element, idx: &mut usize, key: &ShapeKey) -> Option<&'a mut Element> {
    crate::xml::search_mut(el, |el| {
        if el.local_name() == "AlternateContent" {
            if let Some(i) = shape::first_alt_child_index(el) {
                if matches!(&el.children[i], Node::Element(_)) {
                    return crate::xml::SearchMut::Only(i);
                }
            }
            return crate::xml::SearchMut::Skip;
        }
        if el.local_name() == "cxnSp" {
            *idx += 1;
            if match key {
                ShapeKey::Index(n) => *idx == *n,
                ShapeKey::Id(req) => cnv_pr(el)
                    .and_then(|c| c.attr("id"))
                    .is_some_and(|id| shape::ids_match(id, req)),
            } {
                return crate::xml::SearchMut::Hit;
            }
            return crate::xml::SearchMut::Skip;
        }
        crate::xml::SearchMut::down(el)
    })
}

pub fn insert(
    doc: &mut Document,
    canvas: (i64, i64),
    op: &InsertConnectorOp,
    op_index: usize,
) -> Result<(), Error> {
    let cx = op.cx.unwrap_or(DEFAULT_CX);
    let cy = op.cy.unwrap_or(DEFAULT_CY);
    let (x, y) = match (op.x, op.y) {
        (Some(px), Some(py)) => (px, py),
        _ => picture::vacant_pos_canvas(canvas, &doc.root, cx, cy),
    };
    let start = match &op.start {
        Some(b) => Some(resolve_bind(
            &doc.root,
            b,
            op.slide,
            op.on_notes,
            op.master,
            op_index,
        )?),
        None => None,
    };
    let end = match &op.end {
        Some(b) => Some(resolve_bind(
            &doc.root,
            b,
            op.slide,
            op.on_notes,
            op.master,
            op_index,
        )?),
        None => None,
    };
    picture::ensure_xmlns(&mut doc.root, "a", DML_NS);
    let cnv = picture::next_cnv_id(&doc.root);
    let name = format!("连接线 {cnv}");
    let cxn = build_cxn(
        cnv,
        &name,
        &op.prst,
        x,
        y,
        cx,
        cy,
        start.as_ref(),
        end.as_ref(),
    );
    picture::insert_into_sp_tree(&mut doc.root, cxn)?;
    Ok(())
}

pub fn set_binds(doc: &mut Document, op: &SetConnectorOp, op_index: usize) -> Result<(), Error> {
    let start = match &op.start {
        Some(b) => Some(resolve_bind(
            &doc.root,
            b,
            op.target.slide,
            op.target.on_notes,
            op.target.master,
            op_index,
        )?),
        None => None,
    };
    let end = match &op.end {
        Some(b) => Some(resolve_bind(
            &doc.root,
            b,
            op.target.slide,
            op.target.on_notes,
            op.target.master,
            op_index,
        )?),
        None => None,
    };
    let el = find_mut(&mut doc.root, &op.target.key).ok_or_else(|| {
        Error::with_path(
            ErrorKind::ConnectorNotFound { id: op.id.clone() },
            format!("ops[{op_index}].id"),
        )
    })?;
    if let Some((ref cnv, site)) = start {
        upsert_cxn_end(el, "stCxn", cnv, site);
    }
    if let Some((ref cnv, site)) = end {
        upsert_cxn_end(el, "endCxn", cnv, site);
    }
    Ok(())
}

fn resolve_bind(
    root: &Element,
    bind: &CxnBind,
    slide: usize,
    on_notes: bool,
    master: Option<(usize, Option<usize>)>,
    op_index: usize,
) -> Result<(String, u32), Error> {
    if bind.slide != slide || bind.on_notes != on_notes || bind.master != master {
        return Err(Error::with_path(
            ErrorKind::ConnectorEndHost,
            format!("ops[{op_index}]"),
        ));
    }
    let cnv = if bind.is_cxn {
        let el = collect(root)
            .into_iter()
            .enumerate()
            .find(|(i, el)| match &bind.key {
                ShapeKey::Index(n) => *i + 1 == *n,
                ShapeKey::Id(req) => cnv_pr(el)
                    .and_then(|c| c.attr("id"))
                    .is_some_and(|id| shape::ids_match(id, req)),
            })
            .ok_or_else(|| {
                Error::with_path(
                    ErrorKind::ConnectorNotFound {
                        id: bind.id.clone(),
                    },
                    format!("ops[{op_index}]"),
                )
            })?;
        cnv_pr(el.1)
            .and_then(|c| c.attr("id"))
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                Error::with_path(
                    ErrorKind::ConnectorNotFound {
                        id: bind.id.clone(),
                    },
                    format!("ops[{op_index}]"),
                )
            })?
            .to_string()
    } else {
        let list = shape::collect_text_shapes(root);
        let sp = match &bind.key {
            ShapeKey::Index(n) => list.get(n - 1).copied(),
            ShapeKey::Id(req) => list.iter().copied().find(|sp| {
                shape::cnv_pr(sp)
                    .and_then(|c| c.attr("id"))
                    .is_some_and(|id| shape::ids_match(id, req))
            }),
        }
        .ok_or_else(|| {
            Error::with_path(
                ErrorKind::ShapeNotFound {
                    id: bind.id.clone(),
                },
                format!("ops[{op_index}]"),
            )
        })?;
        shape::cnv_pr(sp)
            .and_then(|c| c.attr("id"))
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                Error::with_path(
                    ErrorKind::ShapeNotFound {
                        id: bind.id.clone(),
                    },
                    format!("ops[{op_index}]"),
                )
            })?
            .to_string()
    };
    Ok((cnv, bind.site))
}

fn upsert_cxn_end(cxn: &mut Element, local: &str, cnv: &str, site: u32) {
    let nv = match cxn.child_by_local_mut("nvCxnSpPr") {
        Some(nv) => nv,
        None => return,
    };
    let pr = picture::ensure_child(nv, "cNvCxnSpPr");
    pr.children
        .retain(|n| !matches!(n, Node::Element(e) if e.local_name() == local));
    let mut e = Element::new(format!("a:{local}"));
    e.set_attr("id", cnv.to_string());
    e.set_attr("idx", (site.saturating_sub(1)).to_string());
    if local == "stCxn" {
        pr.children.insert(0, Node::Element(e));
    } else if let Some(i) = pr
        .children
        .iter()
        .position(|n| matches!(n, Node::Element(e) if e.local_name() == "stCxn"))
    {
        pr.children.insert(i + 1, Node::Element(e));
    } else {
        pr.children.insert(0, Node::Element(e));
    }
}

fn build_cxn(
    cnv: u32,
    name: &str,
    prst: &str,
    x: i64,
    y: i64,
    cx: i64,
    cy: i64,
    start: Option<&(String, u32)>,
    end: Option<&(String, u32)>,
) -> Element {
    let mut cxn = Element::new("p:cxnSp");
    let mut nv = Element::new("p:nvCxnSpPr");
    nv.push_element(
        Element::new("p:cNvPr")
            .with_attr("id", cnv.to_string())
            .with_attr("name", name),
    );
    let mut cxn_pr = Element::new("p:cNvCxnSpPr");
    if let Some((id, site)) = start {
        cxn_pr.push_element(
            Element::new("a:stCxn")
                .with_attr("id", id)
                .with_attr("idx", (site.saturating_sub(1)).to_string()),
        );
    }
    if let Some((id, site)) = end {
        cxn_pr.push_element(
            Element::new("a:endCxn")
                .with_attr("id", id)
                .with_attr("idx", (site.saturating_sub(1)).to_string()),
        );
    }
    nv.push_element(cxn_pr);
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
