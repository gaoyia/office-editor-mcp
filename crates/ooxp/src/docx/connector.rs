//! Word 连接线：`wps:wsp` 加连接线几何或 `cNvCnPr`，不是独立 `cxnSp`。

use serde_json::{json, Value};

use crate::error::{Error, ErrorKind, IdReason};
use crate::opc::Relationships;
use crate::xml::{Element, Node};

use super::drawing_hlink;
use super::para::{self, TxbxKey, TxbxTarget};
use super::picture::{self, PicKind, WrapKind};
use super::textbox::{self, WPS_NS};
use super::{MC_NS, WP_NS};

pub const DEFAULT_CX: i64 = 1_828_800;
pub const DEFAULT_CY: i64 = 762_000;
const LINE_W: i64 = 12_700;

#[derive(Clone)]
pub struct CxnInfo {
    pub id: String,
    pub doc_pr_id: Option<String>,
    pub name: Option<String>,
    pub kind: PicKind,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
    pub wrap: Option<WrapKind>,
    pub prst: Option<String>,
    pub stable: bool,
    pub hyperlink: Value,
}

pub fn is_writable_prst(s: &str) -> bool {
    crate::drawing_geom::is_writable_connector_prst(s)
}

pub fn is_connector_geom(prst: &str) -> bool {
    crate::drawing_geom::is_writable_connector_prst(prst)
}

/// `wps:wsp` 且没有文本框内容，几何是连接线，或带 `cNvCnPr`。
pub fn is_connector_wsp(wsp: &Element) -> bool {
    if wsp.local_name() != "wsp" {
        return false;
    }
    if has_txbx(wsp) {
        return false;
    }
    if has_local(wsp, "cNvCnPr") {
        return true;
    }
    prst_of(wsp).is_some_and(|p| is_connector_geom(&p))
}

pub fn collect_connectors(
    root: &Element,
    host_para_id: &str,
    rels: Option<&Relationships>,
) -> Result<Vec<CxnInfo>, Error> {
    let mut out = Vec::new();
    let mut index = 0usize;
    walk_host(root, &mut |drawing| {
        if super::watermark::is_watermark_drawing(drawing) {
            return Ok(());
        }
        if textbox::is_group(drawing) {
            collect_group_cxns(drawing, host_para_id, &mut index, &mut out, rels)?;
        } else if is_connector_drawing(drawing) {
            index += 1;
            out.push(cxn_from_drawing(drawing, host_para_id, index, rels)?);
        }
        Ok(())
    })?;
    Ok(out)
}

pub fn connectors_json(cxns: &[CxnInfo]) -> Vec<Value> {
    cxns.iter()
        .map(|c| {
            json!({
                "id": c.id,
                "docPrId": c.doc_pr_id,
                "name": c.name,
                "kind": c.kind.as_str(),
                "cx": c.cx,
                "cy": c.cy,
                "wrap": c.wrap.map(WrapKind::as_str),
                "prst": c.prst,
                "stable": c.stable,
                "hyperlink": c.hyperlink,
            })
        })
        .collect()
}

pub fn parse_cxn_id(id: &str, op_index: usize) -> Result<TxbxTarget, Error> {
    parse_cxn_id_at(id, &format!("ops[{op_index}].id"))
}

pub fn parse_cxn_id_at(id: &str, path: &str) -> Result<TxbxTarget, Error> {
    let path = path.to_string();
    if id.contains("/pic:")
        || id.contains("/txbx:")
        || id.contains("/ink:")
        || id.contains("/model3d:")
        || id.contains("/shape:")
    {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::DocxCxnPrefix,
            },
            path,
        ));
    }
    let Some((host, after)) = id.split_once("/cxn:") else {
        if looks_like_host(id) {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::DocxCxnContainer,
                },
                path,
            ));
        }
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::DocxCxnPrefix,
            },
            path,
        ));
    };
    let (key, tail) = parse_cxn_key(after, &path)?;
    if tail.is_some() {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::DocxCxnPrefix,
            },
            path,
        ));
    }
    let host = para::parse_host_para_id_at(host, &path)?;
    Ok(TxbxTarget { host, key })
}

pub fn looks_like_cxn_id(id: &str) -> bool {
    id.contains("/cxn:")
        && !id.contains("/pic:")
        && !id.contains("/txbx:")
        && !id.contains("/shape:")
}

pub fn insert_connector(p: &mut Element, doc_pr: u32, cx: i64, cy: i64, prst: &str) {
    textbox::insert_ac(p, build_connector_ac(doc_pr, cx, cy, prst));
}

pub fn remove_connector(host: &mut Element, key: &TxbxKey) -> bool {
    let mut index = 0usize;
    remove_cxn_walk(host, key, &mut index)
}

pub fn find_connector<'a>(host: &'a Element, key: &TxbxKey) -> Option<&'a Element> {
    let mut index = 0usize;
    find_cxn_walk(host, key, &mut index)
}

/// 连接线上写 `a:hlinkClick` 的父节点。
pub fn hlink_parent_mut<'a>(host: &'a mut Element, key: &TxbxKey) -> Option<&'a mut Element> {
    let mut index = 0usize;
    let wsp = find_cxn_wsp_mut(host, key, &mut index)?;
    drawing_hlink::textbox_click_parent_mut(wsp)
}

/// 该连接线是否已有 `hlinkClick`。找不到为 `None`。
pub fn has_click(host: &mut Element, key: &TxbxKey) -> Option<bool> {
    let mut index = 0usize;
    let wsp = find_cxn_wsp_mut(host, key, &mut index)?;
    Some(drawing_hlink::textbox_has_click(wsp))
}

fn collect_group_cxns(
    drawing: &Element,
    host_para_id: &str,
    index: &mut usize,
    out: &mut Vec<CxnInfo>,
    rels: Option<&Relationships>,
) -> Result<(), Error> {
    let kind = drawing_kind(drawing);
    let wrap = drawing_wrap(drawing, kind);
    walk_all(drawing, &mut |e| {
        if is_connector_wsp(e) {
            *index += 1;
            if let Ok(info) = cxn_from_wsp(e, host_para_id, *index, kind, wrap, drawing, rels) {
                out.push(info);
            }
        }
    });
    Ok(())
}

fn cxn_from_drawing(
    drawing: &Element,
    host: &str,
    index: usize,
    rels: Option<&Relationships>,
) -> Result<CxnInfo, Error> {
    let kind = drawing_kind(drawing);
    let wrap = drawing_wrap(drawing, kind);
    let (cx, cy) = drawing_extent(drawing)?;
    let (doc_pr_id, name) = doc_pr(drawing);
    let (id, stable) = cxn_id(host, &doc_pr_id, index);
    let prst = prst_of_drawing(drawing);
    Ok(CxnInfo {
        id,
        doc_pr_id,
        name,
        kind,
        cx,
        cy,
        wrap,
        prst,
        stable,
        hyperlink: drawing_hlink::inspect_textbox(drawing, rels),
    })
}

fn cxn_from_wsp(
    wsp: &Element,
    host: &str,
    index: usize,
    kind: PicKind,
    wrap: Option<WrapKind>,
    drawing: &Element,
    rels: Option<&Relationships>,
) -> Result<CxnInfo, Error> {
    let (mut doc_pr_id, name) = cnv_pr(wsp);
    if doc_pr_id.is_none() {
        let (outer, outer_name) = doc_pr(drawing);
        doc_pr_id = outer;
        let name = name.or(outer_name);
        let (id, stable) = cxn_id(host, &doc_pr_id, index);
        let (cx, cy) = cxn_extent(wsp, drawing)?;
        return Ok(CxnInfo {
            id,
            doc_pr_id,
            name,
            kind,
            cx,
            cy,
            wrap,
            prst: prst_of(wsp),
            stable,
            hyperlink: drawing_hlink::inspect_textbox(wsp, rels),
        });
    }
    let (id, stable) = cxn_id(host, &doc_pr_id, index);
    let (cx, cy) = cxn_extent(wsp, drawing)?;
    Ok(CxnInfo {
        id,
        doc_pr_id,
        name,
        kind,
        cx,
        cy,
        wrap,
        prst: prst_of(wsp),
        stable,
        hyperlink: drawing_hlink::inspect_textbox(wsp, rels),
    })
}

fn is_connector_drawing(drawing: &Element) -> bool {
    if textbox::is_group(drawing) {
        return false;
    }
    let mut hit = false;
    walk_all(drawing, &mut |e| {
        if is_connector_wsp(e) {
            hit = true;
        }
    });
    hit
}

fn cxn_id(host: &str, doc_pr_id: &Option<String>, index: usize) -> (String, bool) {
    match doc_pr_id {
        Some(v) if !v.is_empty() => (format!("{host}/cxn:{v}"), true),
        _ => (format!("{host}/cxn:index:{index}"), false),
    }
}

fn parse_cxn_key<'a>(s: &'a str, path: &str) -> Result<(TxbxKey, Option<&'a str>), Error> {
    if let Some(rest) = s.strip_prefix("index:") {
        let (num, tail) = match rest.split_once('/') {
            Some((a, b)) => (a, Some(b)),
            None => (rest, None),
        };
        let n: usize = num.parse().map_err(|_| {
            Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::CxnIndexNotInt,
                },
                path.to_string(),
            )
        })?;
        if n < 1 {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::CxnIndexFromOne,
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
                reason: IdReason::DocxCxnPrefix,
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

fn find_cxn_walk<'a>(el: &'a Element, key: &TxbxKey, index: &mut usize) -> Option<&'a Element> {
    crate::xml::search(el, |el| {
        if el.local_name() == "txbxContent" {
            return crate::xml::Search::Skip;
        }
        if el.local_name() == "drawing" {
            if textbox::is_group(el) {
                let mut hit = None;
                walk_all(el, &mut |e| {
                    if is_connector_wsp(e) {
                        *index += 1;
                        let (doc_pr_id, _) = cnv_pr(e);
                        let id = if doc_pr_id.is_some() {
                            doc_pr_id
                        } else {
                            doc_pr(el).0
                        };
                        if hit.is_none() && matches_key(&id, *index, key) {
                            hit = Some(());
                        }
                    }
                });
                if hit.is_some() {
                    return crate::xml::Search::Hit(el);
                }
                return crate::xml::Search::Skip;
            }
            if is_connector_drawing(el) {
                *index += 1;
                let (doc_pr_id, _) = doc_pr(el);
                if matches_key(&doc_pr_id, *index, key) {
                    return crate::xml::Search::Hit(el);
                }
            }
            return crate::xml::Search::Skip;
        }
        if el.local_name() == "AlternateContent" {
            if let Some(i) = first_choice(el) {
                if let Node::Element(child) = &el.children[i] {
                    return crate::xml::Search::Only(child);
                }
            }
            return crate::xml::Search::Skip;
        }
        crate::xml::Search::down(el)
    })
}

fn find_cxn_wsp_mut<'a>(
    el: &'a mut Element,
    key: &TxbxKey,
    index: &mut usize,
) -> Option<&'a mut Element> {
    let found = crate::xml::search_mut(el, |el| {
        if el.local_name() == "txbxContent" {
            return crate::xml::SearchMut::Skip;
        }
        if el.local_name() == "AlternateContent" {
            if let Some(i) = first_choice(el) {
                if matches!(&el.children[i], Node::Element(_)) {
                    return crate::xml::SearchMut::Only(i);
                }
            }
            return crate::xml::SearchMut::Skip;
        }
        if el.local_name() == "drawing" {
            if textbox::is_group(el) || is_connector_drawing(el) {
                if is_connector_drawing(el) && !textbox::is_group(el) {
                    *index += 1;
                    let (doc_pr_id, _) = doc_pr(el);
                    if !matches_key(&doc_pr_id, *index, key) {
                        return crate::xml::SearchMut::Skip;
                    }
                }
                return crate::xml::SearchMut::Hit;
            }
            return crate::xml::SearchMut::Skip;
        }
        crate::xml::SearchMut::down(el)
    })?;
    if textbox::is_group(found) {
        let outer = doc_pr(found).0;
        find_group_cxn_wsp_in(found, key, index, &outer)
    } else {
        find_first_cxn_wsp_mut(found)
    }
}

fn find_group_cxn_wsp_in<'a>(
    el: &'a mut Element,
    key: &TxbxKey,
    index: &mut usize,
    outer_doc_pr: &Option<String>,
) -> Option<&'a mut Element> {
    crate::xml::search_mut(el, |el| {
        if el.local_name() == "txbxContent" {
            return crate::xml::SearchMut::Skip;
        }
        if is_connector_wsp(el) {
            *index += 1;
            let (doc_pr_id, _) = cnv_pr(el);
            let id = if doc_pr_id.is_some() {
                doc_pr_id
            } else {
                outer_doc_pr.clone()
            };
            if matches_key(&id, *index, key) {
                return crate::xml::SearchMut::Hit;
            }
            return crate::xml::SearchMut::Skip;
        }
        crate::xml::SearchMut::down(el)
    })
}

fn find_first_cxn_wsp_mut(el: &mut Element) -> Option<&mut Element> {
    crate::xml::search_mut(el, |el| {
        if el.local_name() == "txbxContent" {
            return crate::xml::SearchMut::Skip;
        }
        if is_connector_wsp(el) {
            return crate::xml::SearchMut::Hit;
        }
        crate::xml::SearchMut::down(el)
    })
}

fn remove_cxn_walk(el: &mut Element, key: &TxbxKey, index: &mut usize) -> bool {
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
            if ac_is_group(&el.children[i]) {
                if let Node::Element(ac) = &mut el.children[i] {
                    if let Some(idx) = first_choice(ac) {
                        if let Node::Element(branch) = &mut ac.children[idx] {
                            if remove_cxn_walk(branch, key, index) {
                                if !textbox::group_has_member(ac) {
                                    el.children.remove(i);
                                }
                                return true;
                            }
                        }
                    }
                }
                i += 1;
                continue;
            }
            if ac_has_cxn(&el.children[i], key, index) {
                el.children.remove(i);
                return true;
            }
            i += 1;
            continue;
        }
        if local == "drawing" {
            let grouped = matches!(&el.children[i], Node::Element(d) if textbox::is_group(d));
            if grouped {
                if let Node::Element(d) = &mut el.children[i] {
                    if remove_group_cxn(d, key, index) {
                        if !textbox::group_has_member(d) {
                            el.children.remove(i);
                        }
                        return true;
                    }
                }
                i += 1;
                continue;
            }
            let hit = match &el.children[i] {
                Node::Element(d) if is_connector_drawing(d) => {
                    *index += 1;
                    let (doc_pr_id, _) = doc_pr(d);
                    matches_key(&doc_pr_id, *index, key)
                }
                _ => false,
            };
            if hit {
                el.children.remove(i);
                return true;
            }
            i += 1;
            continue;
        }
        if let Node::Element(child) = &mut el.children[i] {
            if remove_cxn_walk(child, key, index) {
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

fn remove_group_cxn(el: &mut Element, key: &TxbxKey, index: &mut usize) -> bool {
    if el.local_name() == "txbxContent" {
        return false;
    }
    let mut i = 0;
    while i < el.children.len() {
        let is_cxn = matches!(&el.children[i], Node::Element(c) if is_connector_wsp(c));
        if is_cxn {
            *index += 1;
            let hit = match &el.children[i] {
                Node::Element(c) => {
                    let (mut id, _) = cnv_pr(c);
                    if id.is_none() {
                        id = doc_pr(el).0;
                    }
                    matches_key(&id, *index, key)
                }
                Node::Text(_) | Node::Raw(_) => false,
            };
            if hit {
                el.children.remove(i);
                return true;
            }
        }
        if let Node::Element(child) = &mut el.children[i] {
            if remove_group_cxn(child, key, index) {
                return true;
            }
        }
        i += 1;
    }
    false
}

fn ac_is_group(node: &Node) -> bool {
    let Node::Element(ac) = node else {
        return false;
    };
    let Some(i) = first_choice(ac) else {
        return false;
    };
    let Node::Element(branch) = &ac.children[i] else {
        return false;
    };
    let mut group = false;
    walk_all(branch, &mut |e| {
        if e.local_name() == "wgp" {
            group = true;
        }
    });
    group
}

fn ac_has_cxn(node: &Node, key: &TxbxKey, index: &mut usize) -> bool {
    let Node::Element(ac) = node else {
        return false;
    };
    let Some(i) = first_choice(ac) else {
        return false;
    };
    let Node::Element(branch) = &ac.children[i] else {
        return false;
    };
    find_cxn_walk(branch, key, index).is_some()
}

fn first_choice(ac: &Element) -> Option<usize> {
    crate::xml::alternate_child(ac).unwrap_or(None)
}

fn walk_host(
    root: &Element,
    f: &mut impl FnMut(&Element) -> Result<(), Error>,
) -> Result<(), Error> {
    let mut stack = vec![root];
    while let Some(el) = stack.pop() {
        if el.local_name() == "txbxContent" {
            continue;
        }
        if el.local_name() == "AlternateContent" {
            if let Some(i) = first_choice(el) {
                if let Node::Element(child) = &el.children[i] {
                    stack.push(child);
                }
            }
            continue;
        }
        if el.local_name() == "drawing" {
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

fn walk_all(el: &Element, f: &mut impl FnMut(&Element)) {
    crate::xml::walk_elements_if(el, |node| {
        if node.local_name() == "txbxContent" {
            return false;
        }
        f(node);
        true
    });
}

fn has_txbx(el: &Element) -> bool {
    let mut found = false;
    walk_all(el, &mut |e| {
        if e.local_name() == "txbxContent"
            || e.local_name() == "txbx"
            || e.local_name() == "linkedTxbx"
        {
            found = true;
        }
    });
    found
}

fn has_local(el: &Element, local: &str) -> bool {
    let mut found = false;
    walk_all(el, &mut |e| {
        if e.local_name() == local {
            found = true;
        }
    });
    found
}

fn prst_of(el: &Element) -> Option<String> {
    let mut prst = None;
    walk_all(el, &mut |e| {
        if e.local_name() == "prstGeom" && prst.is_none() {
            prst = e.attr_local("prst").map(str::to_string);
        }
    });
    prst
}

fn prst_of_drawing(drawing: &Element) -> Option<String> {
    prst_of(drawing)
}

fn doc_pr(drawing: &Element) -> (Option<String>, Option<String>) {
    let frame = drawing
        .element_children()
        .find(|e| e.local_name() == "inline" || e.local_name() == "anchor");
    let pr = frame.and_then(|f| f.child_by_local("docPr"));
    (
        pr.and_then(|p| p.attr_local("id")).map(str::to_string),
        pr.and_then(|p| p.attr_local("name")).map(str::to_string),
    )
}

fn cnv_pr(wsp: &Element) -> (Option<String>, Option<String>) {
    let mut id = None;
    let mut name = None;
    walk_all(wsp, &mut |e| {
        if e.local_name() == "cNvPr" && id.is_none() {
            id = e.attr_local("id").map(str::to_string);
            name = e.attr_local("name").map(str::to_string);
        }
    });
    (id, name)
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

fn drawing_extent(drawing: &Element) -> Result<(Option<i64>, Option<i64>), Error> {
    let frame = drawing
        .element_children()
        .find(|e| e.local_name() == "inline" || e.local_name() == "anchor");
    let extent = frame.and_then(|f| f.child_by_local("extent"));
    Ok((parse_emu(extent, "cx")?, parse_emu(extent, "cy")?))
}

fn xfrm_extent(el: &Element) -> Result<(Option<i64>, Option<i64>), Error> {
    let mut cx = None;
    let mut cy = None;
    walk_all(el, &mut |e| {
        if e.local_name() == "ext" && cx.is_none() {
            if let Ok(Some(v)) = parse_emu(Some(e), "cx") {
                cx = Some(v);
            }
            if let Ok(Some(v)) = parse_emu(Some(e), "cy") {
                cy = Some(v);
            }
        }
    });
    Ok((cx, cy))
}

fn cxn_extent(wsp: &Element, drawing: &Element) -> Result<(Option<i64>, Option<i64>), Error> {
    let inner = xfrm_extent(wsp)?;
    if inner.0.is_none() && inner.1.is_none() {
        drawing_extent(drawing)
    } else {
        Ok(inner)
    }
}

fn parse_emu(el: Option<&Element>, local: &str) -> Result<Option<i64>, Error> {
    let Some(el) = el else {
        return Ok(None);
    };
    let Some(raw) = el.attr_local(local) else {
        return Ok(None);
    };
    raw.parse::<i64>().map(Some).map_err(|_| {
        Error::xml_detail(
            crate::error::XmlIssue::AttrRead,
            format!("wp:extent @{local}={raw}"),
        )
    })
}

fn is_empty_run(el: &Element) -> bool {
    el.local_name() == "r" && el.element_children().all(|c| c.local_name() == "rPr")
}

fn build_connector_ac(doc_pr: u32, cx: i64, cy: i64, prst: &str) -> Element {
    let mut ac = Element::new("mc:AlternateContent");
    let mut choice = Element::new("mc:Choice").with_attr("Requires", "wps");
    let mut drawing = Element::new("w:drawing");
    let mut frame = Element::new("wp:anchor")
        .with_attr("distT", "0")
        .with_attr("distB", "0")
        .with_attr("distL", "114300")
        .with_attr("distR", "114300")
        .with_attr("simplePos", "0")
        .with_attr("relativeHeight", "251659264")
        .with_attr("behindDoc", "0")
        .with_attr("locked", "0")
        .with_attr("layoutInCell", "1")
        .with_attr("allowOverlap", "1");
    frame.push_element(
        Element::new("wp:simplePos")
            .with_attr("x", "0")
            .with_attr("y", "0"),
    );
    let mut pos_h = Element::new("wp:positionH").with_attr("relativeFrom", "margin");
    pos_h.push_element(Element::new("wp:align").with_text("left"));
    let mut pos_v = Element::new("wp:positionV").with_attr("relativeFrom", "margin");
    pos_v.push_element(Element::new("wp:align").with_text("top"));
    frame.push_element(pos_h);
    frame.push_element(pos_v);
    frame.push_element(
        Element::new("wp:extent")
            .with_attr("cx", cx.to_string())
            .with_attr("cy", cy.to_string()),
    );
    frame.push_element(
        Element::new("wp:effectExtent")
            .with_attr("l", "0")
            .with_attr("t", "0")
            .with_attr("r", "0")
            .with_attr("b", "0"),
    );
    frame.push_element(Element::new("wp:wrapSquare").with_attr("wrapText", "bothSides"));
    frame.push_element(
        Element::new("wp:docPr")
            .with_attr("id", doc_pr.to_string())
            .with_attr("name", format!("连接线 {doc_pr}")),
    );
    frame.push_element(Element::new("wp:cNvGraphicFramePr"));
    let mut graphic = Element::new("a:graphic");
    let mut data = Element::new("a:graphicData").with_attr("uri", WPS_NS);
    data.push_element(build_connector_wsp(doc_pr, cx, cy, prst));
    graphic.push_element(data);
    frame.push_element(graphic);
    drawing.push_element(frame);
    choice.push_element(drawing);
    ac.push_element(choice);
    let mut fallback = Element::new("mc:Fallback");
    fallback.push_element(Element::new("w:pict"));
    ac.push_element(fallback);
    let _ = (MC_NS, WP_NS);
    ac
}

fn build_connector_wsp(doc_pr: u32, cx: i64, cy: i64, prst: &str) -> Element {
    let mut wsp = Element::new("wps:wsp");
    wsp.push_element(
        Element::new("wps:cNvPr")
            .with_attr("id", doc_pr.to_string())
            .with_attr("name", format!("连接线 {doc_pr}")),
    );
    wsp.push_element(Element::new("wps:cNvCnPr"));
    let mut sp = Element::new("wps:spPr");
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
    sp.push_element(xfrm);
    let mut geom = Element::new("a:prstGeom").with_attr("prst", prst);
    geom.push_element(Element::new("a:avLst"));
    sp.push_element(geom);
    sp.push_element(Element::new("a:noFill"));
    let mut ln = Element::new("a:ln").with_attr("w", LINE_W.to_string());
    let mut ln_fill = Element::new("a:solidFill");
    ln_fill.push_element(Element::new("a:srgbClr").with_attr("val", "000000"));
    ln.push_element(ln_fill);
    sp.push_element(ln);
    wsp.push_element(sp);
    wsp.push_element(Element::new("wps:bodyPr"));
    wsp
}

use super::hf;
use super::para::HostPara;
use super::request::DeleteConnectorOp;
use super::request::InsertConnectorOp;
use crate::opc::Package;
use crate::xml::Document;
use std::collections::HashMap;
// 从 apply 搬来的操作体。
pub(crate) fn cxn_missing(id: &str, i: usize) -> Error {
    Error::with_path(
        ErrorKind::ConnectorNotFound { id: id.to_string() },
        format!("ops[{i}].id"),
    )
}
pub(crate) fn apply_insert_connector(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &InsertConnectorOp,
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
    let part_name = super::apply::part_for_host(pkg, catalog, doc_part, &op.host, &op.id, i)?;
    let doc = super::apply::load_part(pkg, docs, &part_name)?;
    textbox::ensure_textbox_ns(&mut doc.root);
    let doc_pr = picture::next_docpr_id(&doc.root);
    let (cx, cy) = match (op.cx, op.cy) {
        (Some(x), Some(y)) => (x, y),
        _ => (super::connector::DEFAULT_CX, super::connector::DEFAULT_CY),
    };
    let p = para::find_paragraph_mut(&mut doc.root, op.host.body()).ok_or_else(|| {
        Error::with_path(
            ErrorKind::ParaNotFound { id: op.id.clone() },
            format!("ops[{i}].id"),
        )
    })?;
    super::connector::insert_connector(p, doc_pr, cx, cy, &op.prst);
    dirty.insert(part_name, ());
    Ok(())
}
pub(crate) fn apply_delete_connector(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &DeleteConnectorOp,
    i: usize,
) -> Result<(), Error> {
    let part_name = super::apply::part_for_txbx(pkg, catalog, doc_part, &op.target, &op.id, i)?;
    let doc = super::apply::load_part(pkg, docs, &part_name)?;
    let host = para::find_paragraph_mut(&mut doc.root, op.target.host.body())
        .ok_or_else(|| cxn_missing(&op.id, i))?;
    if !super::connector::remove_connector(host, &op.target.key) {
        return Err(cxn_missing(&op.id, i));
    }
    dirty.insert(part_name, ());
    Ok(())
}
