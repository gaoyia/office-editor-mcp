//! DrawingML 对象上的 `a:hlinkClick`。

use serde_json::{json, Value};

use crate::opc::{Relationship, Relationships};
use crate::xml::{Element, Node};

use super::{A_NS, REL_HYPERLINK, REL_NS};

pub fn inspect_picture(drawing: &Element, rels: Option<&Relationships>) -> Value {
    if let Some(click) = doc_pr_click(drawing) {
        return click_json(click, rels);
    }
    if let Some(click) = first_pic_cnv_click(drawing) {
        return click_json(click, rels);
    }
    Value::Null
}

pub fn inspect_pic_nv(pic: &Element, rels: Option<&Relationships>) -> Value {
    match cnv_click(pic) {
        Some(click) => click_json(click, rels),
        None => Value::Null,
    }
}

pub fn inspect_textbox(drawing: &Element, rels: Option<&Relationships>) -> Value {
    if let Some(click) = cnv_click(drawing) {
        return click_json(click, rels);
    }
    if let Some(click) = doc_pr_click(drawing) {
        return click_json(click, rels);
    }
    Value::Null
}

pub fn picture_has_click(drawing: &Element) -> bool {
    doc_pr_click(drawing).is_some() || first_pic_cnv_click(drawing).is_some()
}

pub fn textbox_has_click(drawing: &Element) -> bool {
    cnv_click(drawing).is_some() || doc_pr_click(drawing).is_some()
}

pub fn picture_click_parent_mut(drawing: &mut Element) -> Option<&mut Element> {
    let use_cnv = !doc_pr_has_click(drawing) && first_pic_cnv_click(drawing).is_some();
    if use_cnv {
        first_pic_cnv_pr_mut(drawing)
    } else {
        doc_pr_mut(drawing)
    }
}

pub fn textbox_click_parent_mut(drawing: &mut Element) -> Option<&mut Element> {
    let prefer_cnv =
        cnv_click(drawing).is_some() || (cnv_pr(drawing).is_some() && !doc_pr_has_click(drawing));
    if prefer_cnv {
        cnv_pr_mut(drawing)
    } else {
        doc_pr_mut(drawing)
    }
}

pub fn insert_click(parent: &mut Element, rid: &str, tooltip: Option<&str>) {
    write_click(parent, rid, tooltip);
}

pub fn set_click(parent: &mut Element, rid: Option<&str>, tooltip: Option<Option<&str>>) -> bool {
    let Some(click) = parent.child_by_local_mut("hlinkClick") else {
        return false;
    };
    if let Some(rid) = rid {
        remove_attr_local(click, "id");
        click.set_attr("r:id", rid);
    }
    match tooltip {
        None => {}
        Some(None) => remove_attr_local(click, "tooltip"),
        Some(Some(t)) => click.set_attr("tooltip", t),
    }
    true
}

/// 没有 `hlinkClick` 则 `None`；去掉后得到原来的 `r:id`（可缺）。
pub fn delete_click(parent: &mut Element) -> Option<Option<String>> {
    let rid = parent
        .child_by_local("hlinkClick")
        .and_then(|e| e.attr_local("id").map(str::to_string));
    let before = parent.children.len();
    parent.children.retain(|n| match n {
        Node::Element(e) => e.local_name() != "hlinkClick",
        _ => true,
    });
    if parent.children.len() == before {
        return None;
    }
    Some(rid)
}

pub fn click_rid(parent: &Element) -> Option<String> {
    parent
        .child_by_local("hlinkClick")
        .and_then(|e| e.attr_local("id").map(str::to_string))
}

pub fn ensure_ns(root: &mut Element) {
    ensure_xmlns(root, "r", REL_NS);
    ensure_xmlns(root, "a", A_NS);
}

fn ensure_xmlns(el: &mut Element, prefix: &str, uri: &str) {
    let key = format!("xmlns:{prefix}");
    if el.attr(&key).is_none() {
        el.set_attr(&key, uri);
    }
}

pub fn push_rel(rels: &mut Relationships, url: Option<&str>, anchor: Option<&str>) -> String {
    set_rel(rels, None, url, anchor)
}

pub fn set_rel(
    rels: &mut Relationships,
    old_rid: Option<&str>,
    url: Option<&str>,
    anchor: Option<&str>,
) -> String {
    let (target, mode) = if let Some(url) = url {
        (url.to_string(), Some("External".into()))
    } else {
        let name = anchor.unwrap_or("");
        let target = if name.starts_with('#') {
            name.to_string()
        } else {
            format!("#{name}")
        };
        (target, None)
    };
    if let Some(old) = old_rid {
        if let Some(rel) = rels.find_by_id_mut(old) {
            rel.target = target;
            rel.target_mode = mode;
            rel.rel_type = REL_HYPERLINK.to_string();
            return old.to_string();
        }
        rels.push(Relationship {
            id: old.to_string(),
            rel_type: REL_HYPERLINK.to_string(),
            target,
            target_mode: mode,
            extra: Vec::new(),
        });
        return old.to_string();
    }
    let id = rels.next_rid();
    rels.push(Relationship {
        id: id.clone(),
        rel_type: REL_HYPERLINK.to_string(),
        target,
        target_mode: mode,
        extra: Vec::new(),
    });
    id
}

fn click_json(click: &Element, rels: Option<&Relationships>) -> Value {
    let rid = click.attr_local("id").map(str::to_string);
    let rel = rid
        .as_deref()
        .and_then(|id| rels.and_then(|r| r.find_by_id(id)));
    let mut url = Value::Null;
    let mut anchor = Value::Null;
    let mut kind = "anchor";
    if let Some(rel) = rel {
        if rel.target.starts_with('#') {
            kind = "anchor";
            anchor = json!(rel.target.trim_start_matches('#').to_string());
        } else {
            kind = "url";
            url = json!(rel.target.clone());
        }
    }
    json!({
        "kind": kind,
        "rId": rid,
        "url": url,
        "anchor": anchor,
        "tooltip": click.attr("tooltip").map(|s| json!(s)).unwrap_or(Value::Null),
    })
}

fn write_click(parent: &mut Element, rid: &str, tooltip: Option<&str>) {
    let mut el = Element::new("a:hlinkClick").with_attr("r:id", rid);
    if let Some(t) = tooltip {
        el.set_attr("tooltip", t);
    }
    parent.children.insert(0, Node::Element(el));
}

fn frame(drawing: &Element) -> Option<&Element> {
    if drawing.local_name() == "inline" || drawing.local_name() == "anchor" {
        return Some(drawing);
    }
    drawing
        .element_children()
        .find(|e| e.local_name() == "inline" || e.local_name() == "anchor")
}

fn frame_mut(drawing: &mut Element) -> Option<&mut Element> {
    if drawing.local_name() == "inline" || drawing.local_name() == "anchor" {
        return Some(drawing);
    }
    drawing
        .element_children_mut()
        .find(|e| e.local_name() == "inline" || e.local_name() == "anchor")
}

fn doc_pr_mut(drawing: &mut Element) -> Option<&mut Element> {
    frame_mut(drawing)?.child_by_local_mut("docPr")
}

fn doc_pr_click(drawing: &Element) -> Option<&Element> {
    frame(drawing)
        .and_then(|f| f.child_by_local("docPr"))
        .and_then(|d| d.child_by_local("hlinkClick"))
}

fn doc_pr_has_click(drawing: &Element) -> bool {
    doc_pr_click(drawing).is_some()
}

fn first_pic_cnv_click(drawing: &Element) -> Option<&Element> {
    let pic = find_local_outside_txbx(drawing, "pic")?;
    cnv_click(pic)
}

fn first_pic_cnv_pr_mut(drawing: &mut Element) -> Option<&mut Element> {
    let pic = find_local_outside_txbx_mut(drawing, "pic")?;
    cnv_pr_mut(pic)
}

fn cnv_click(el: &Element) -> Option<&Element> {
    cnv_pr(el)?.child_by_local("hlinkClick")
}

fn cnv_pr(el: &Element) -> Option<&Element> {
    find_local_outside_txbx(el, "cNvPr")
}

fn cnv_pr_mut(el: &mut Element) -> Option<&mut Element> {
    find_local_outside_txbx_mut(el, "cNvPr")
}

fn find_local_outside_txbx<'a>(el: &'a Element, local: &str) -> Option<&'a Element> {
    crate::xml::search(el, |el| {
        if el.local_name() == "txbxContent" || el.local_name() == "object" {
            return crate::xml::Search::Skip;
        }
        if el.local_name() == local {
            return crate::xml::Search::Hit(el);
        }
        crate::xml::Search::down(el)
    })
}

fn find_local_outside_txbx_mut<'a>(el: &'a mut Element, local: &str) -> Option<&'a mut Element> {
    crate::xml::search_mut(el, |el| {
        if el.local_name() == "txbxContent" || el.local_name() == "object" {
            return crate::xml::SearchMut::Skip;
        }
        if el.local_name() == local {
            return crate::xml::SearchMut::Hit;
        }
        crate::xml::SearchMut::down(el)
    })
}

fn remove_attr_local(el: &mut Element, local: &str) {
    el.attrs
        .retain(|(k, _)| k.rsplit(':').next().unwrap_or(k) != local);
}

use super::charts;
use super::hf;
use super::ink;
use super::model3d;
use super::para;
use super::picture;
use super::request::SetHyperlinkOp;
use super::smartart;
use super::textbox;
use crate::error::Error;
use crate::error::ErrorKind;
use crate::opc::Package;
use crate::xml::Document;
use std::collections::HashMap;
// 从 apply 搬来的操作体。
pub(crate) fn apply_insert_pic_hlink(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    id: &str,
    target: &picture::PicTarget,
    url: Option<&str>,
    anchor: Option<&str>,
    tooltip: Option<&str>,
    i: usize,
) -> Result<(), Error> {
    let part_name = super::apply::part_for_pic(pkg, catalog, doc_part, target, id, i)?;
    {
        let doc = super::apply::load_part(pkg, docs, &part_name)?;
        let drawing =
            super::picture::find_target_drawing_mut(&mut doc.root, target).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::PictureNotFound { id: id.to_string() },
                    format!("ops[{i}].id"),
                )
            })?;
        if picture_hlink_exists(drawing, target) {
            return Err(Error::with_path(
                ErrorKind::ShapeAlreadyHasHyperlink,
                format!("ops[{i}].id"),
            ));
        }
    }
    let rid = {
        let mut rels = Relationships::from_package(pkg, &part_name)?;
        let rid = super::drawing_hlink::push_rel(&mut rels, url, anchor);
        rels.write_to(pkg, &part_name)?;
        rid
    };
    let doc = super::apply::load_part(pkg, docs, &part_name)?;
    super::drawing_hlink::ensure_ns(&mut doc.root);
    let drawing =
        super::picture::find_target_drawing_mut(&mut doc.root, target).ok_or_else(|| {
            Error::with_path(
                ErrorKind::PictureNotFound { id: id.to_string() },
                format!("ops[{i}].id"),
            )
        })?;
    let parent = picture_hlink_parent_mut(drawing, target).ok_or_else(|| {
        Error::with_path(
            ErrorKind::PictureNotFound { id: id.to_string() },
            format!("ops[{i}].id"),
        )
    })?;
    super::drawing_hlink::insert_click(parent, &rid, tooltip);
    dirty.insert(part_name, ());
    Ok(())
}
pub(crate) fn apply_insert_txbx_hlink(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    id: &str,
    target: &para::TxbxTarget,
    url: Option<&str>,
    anchor: Option<&str>,
    tooltip: Option<&str>,
    i: usize,
) -> Result<(), Error> {
    let part_name = super::apply::part_for_txbx(pkg, catalog, doc_part, target, id, i)?;
    {
        let doc = super::apply::load_part(pkg, docs, &part_name)?;
        let host = para::find_paragraph_mut(&mut doc.root, target.host.body())
            .ok_or_else(|| super::textbox::txbx_missing(id, i))?;
        let drawing = textbox::find_textbox_drawing_mut(host, &target.key)
            .ok_or_else(|| super::textbox::txbx_missing(id, i))?;
        if super::drawing_hlink::textbox_has_click(drawing) {
            return Err(Error::with_path(
                ErrorKind::ShapeAlreadyHasHyperlink,
                format!("ops[{i}].id"),
            ));
        }
    }
    let rid = {
        let mut rels = Relationships::from_package(pkg, &part_name)?;
        let rid = super::drawing_hlink::push_rel(&mut rels, url, anchor);
        rels.write_to(pkg, &part_name)?;
        rid
    };
    let doc = super::apply::load_part(pkg, docs, &part_name)?;
    super::drawing_hlink::ensure_ns(&mut doc.root);
    let host = para::find_paragraph_mut(&mut doc.root, target.host.body())
        .ok_or_else(|| super::textbox::txbx_missing(id, i))?;
    let drawing = textbox::find_textbox_drawing_mut(host, &target.key)
        .ok_or_else(|| super::textbox::txbx_missing(id, i))?;
    let parent = super::drawing_hlink::textbox_click_parent_mut(drawing)
        .ok_or_else(|| super::textbox::txbx_missing(id, i))?;
    super::drawing_hlink::insert_click(parent, &rid, tooltip);
    dirty.insert(part_name, ());
    Ok(())
}
pub(crate) fn apply_insert_click_hlink(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    part_name: &str,
    id: &str,
    url: Option<&str>,
    anchor: Option<&str>,
    tooltip: Option<&str>,
    i: usize,
    mut has_click: impl FnMut(&mut Element) -> Option<bool>,
    mut parent_mut: impl FnMut(&mut Element) -> Option<&mut Element>,
    missing: ErrorKind,
) -> Result<(), Error> {
    {
        let doc = super::apply::load_part(pkg, docs, part_name)?;
        match has_click(&mut doc.root) {
            Some(true) => {
                return Err(Error::with_path(
                    ErrorKind::ShapeAlreadyHasHyperlink,
                    format!("ops[{i}].id"),
                ));
            }
            Some(false) => {}
            None => {
                return Err(Error::with_path(missing, format!("ops[{i}].id")));
            }
        }
    }
    let rid = {
        let mut rels = Relationships::from_package(pkg, part_name)?;
        let rid = super::drawing_hlink::push_rel(&mut rels, url, anchor);
        rels.write_to(pkg, part_name)?;
        rid
    };
    let doc = super::apply::load_part(pkg, docs, part_name)?;
    super::drawing_hlink::ensure_ns(&mut doc.root);
    let parent = parent_mut(&mut doc.root).ok_or_else(|| {
        Error::with_path(
            ErrorKind::HyperlinkNotFound { id: id.to_string() },
            format!("ops[{i}].id"),
        )
    })?;
    super::drawing_hlink::insert_click(parent, &rid, tooltip);
    dirty.insert(part_name.to_string(), ());
    Ok(())
}
pub(crate) fn apply_insert_wsp_hlink(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    id: &str,
    target: &para::TxbxTarget,
    url: Option<&str>,
    anchor: Option<&str>,
    tooltip: Option<&str>,
    i: usize,
    has_click: fn(&mut Element, &para::TxbxKey) -> Option<bool>,
    parent_mut: for<'a> fn(&'a mut Element, &para::TxbxKey) -> Option<&'a mut Element>,
    missing: ErrorKind,
) -> Result<(), Error> {
    let part_name = super::apply::part_for_txbx(pkg, catalog, doc_part, target, id, i)?;
    let key = target.key.clone();
    apply_insert_click_hlink(
        pkg,
        docs,
        dirty,
        &part_name,
        id,
        url,
        anchor,
        tooltip,
        i,
        |root| {
            let host = para::find_paragraph_mut(root, target.host.body())?;
            has_click(host, &key)
        },
        |root| {
            let host = para::find_paragraph_mut(root, target.host.body())?;
            parent_mut(host, &key)
        },
        missing,
    )
}
pub(crate) fn apply_insert_ink_hlink(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    id: &str,
    target: &para::TxbxTarget,
    url: Option<&str>,
    anchor: Option<&str>,
    tooltip: Option<&str>,
    i: usize,
) -> Result<(), Error> {
    let part_name = ink::part_for_target(catalog, doc_part, target, id, &format!("ops[{i}].id"))?;
    let key = target.key.clone();
    super::apply::load_part(pkg, docs, &part_name)?;
    {
        let ctx = ink::HostCtx::load(pkg, &part_name)?;
        let doc = super::apply::ink_doc_mut(docs, &part_name)?;
        match ink::find_drawing_mut(&mut doc.root, &ctx, &key) {
            None => {
                return Err(Error::with_path(
                    ErrorKind::InkNotFound { id: id.to_string() },
                    format!("ops[{i}].id"),
                ));
            }
            Some(d) if super::drawing_hlink::picture_has_click(d) => {
                return Err(Error::with_path(
                    ErrorKind::ShapeAlreadyHasHyperlink,
                    format!("ops[{i}].id"),
                ));
            }
            Some(_) => {}
        }
    }
    let rid = {
        let mut rels = Relationships::from_package(pkg, &part_name)?;
        let rid = super::drawing_hlink::push_rel(&mut rels, url, anchor);
        rels.write_to(pkg, &part_name)?;
        rid
    };
    {
        let ctx = ink::HostCtx::load(pkg, &part_name)?;
        let doc = super::apply::ink_doc_mut(docs, &part_name)?;
        super::drawing_hlink::ensure_ns(&mut doc.root);
        let drawing = ink::find_drawing_mut(&mut doc.root, &ctx, &key).ok_or_else(|| {
            Error::with_path(
                ErrorKind::InkNotFound { id: id.to_string() },
                format!("ops[{i}].id"),
            )
        })?;
        let parent = super::drawing_hlink::picture_click_parent_mut(drawing).ok_or_else(|| {
            Error::with_path(
                ErrorKind::InkNotFound { id: id.to_string() },
                format!("ops[{i}].id"),
            )
        })?;
        super::drawing_hlink::insert_click(parent, &rid, tooltip);
    }
    dirty.insert(part_name, ());
    Ok(())
}
pub(crate) fn apply_insert_model3d_hlink(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    id: &str,
    target: &para::TxbxTarget,
    url: Option<&str>,
    anchor: Option<&str>,
    tooltip: Option<&str>,
    i: usize,
) -> Result<(), Error> {
    let part_name =
        model3d::part_for_target(catalog, doc_part, target, id, &format!("ops[{i}].id"))?;
    let key = target.key.clone();
    apply_insert_click_hlink(
        pkg,
        docs,
        dirty,
        &part_name,
        id,
        url,
        anchor,
        tooltip,
        i,
        |root| {
            let frame = model3d::find_frame_mut(root, &key)?;
            Some(super::drawing_hlink::picture_has_click(frame))
        },
        |root| {
            let frame = model3d::find_frame_mut(root, &key)?;
            super::drawing_hlink::picture_click_parent_mut(frame)
        },
        ErrorKind::Model3dNotFound { id: id.to_string() },
    )
}
pub(crate) fn apply_insert_chart_hlink(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    id: &str,
    target: &charts::ChartTarget,
    url: Option<&str>,
    anchor: Option<&str>,
    tooltip: Option<&str>,
    i: usize,
) -> Result<(), Error> {
    let part_name =
        charts::part_for_target(pkg, catalog, doc_part, target, id, &format!("ops[{i}].id"))?;
    apply_insert_click_hlink(
        pkg,
        docs,
        dirty,
        &part_name,
        id,
        url,
        anchor,
        tooltip,
        i,
        |root| {
            let drawing = charts::find_drawing_mut(root, target)?;
            Some(super::drawing_hlink::picture_has_click(drawing))
        },
        |root| {
            let drawing = charts::find_drawing_mut(root, target)?;
            super::drawing_hlink::picture_click_parent_mut(drawing)
        },
        ErrorKind::ChartNotFound { id: id.to_string() },
    )
}
pub(crate) fn apply_insert_smartart_hlink(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    id: &str,
    target: &smartart::DiagramTarget,
    url: Option<&str>,
    anchor: Option<&str>,
    tooltip: Option<&str>,
    i: usize,
) -> Result<(), Error> {
    let part_name =
        smartart::part_for_target(pkg, catalog, doc_part, target, id, &format!("ops[{i}].id"))?;
    apply_insert_click_hlink(
        pkg,
        docs,
        dirty,
        &part_name,
        id,
        url,
        anchor,
        tooltip,
        i,
        |root| {
            let drawing = smartart::find_drawing_mut(root, target)?;
            Some(super::drawing_hlink::picture_has_click(drawing))
        },
        |root| {
            let drawing = smartart::find_drawing_mut(root, target)?;
            super::drawing_hlink::picture_click_parent_mut(drawing)
        },
        ErrorKind::SmartArtNotFound { id: id.to_string() },
    )
}
pub(crate) fn apply_set_pic_hlink(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    id: &str,
    target: &picture::PicTarget,
    op: &SetHyperlinkOp,
    i: usize,
) -> Result<(), Error> {
    apply_set_drawing_hlink(
        pkg,
        docs,
        dirty,
        &super::apply::part_for_pic(pkg, catalog, doc_part, target, id, i)?,
        id,
        i,
        op,
        |root| {
            let drawing = super::picture::find_target_drawing_mut(root, target)?;
            picture_hlink_parent_mut(drawing, target)
        },
        |root| {
            let drawing = super::picture::find_target_drawing_mut(root, target)?;
            Some(picture_hlink_exists(drawing, target))
        },
        ErrorKind::PictureNotFound { id: id.to_string() },
    )
}
pub(crate) fn apply_set_txbx_hlink(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    id: &str,
    target: &para::TxbxTarget,
    op: &SetHyperlinkOp,
    i: usize,
) -> Result<(), Error> {
    apply_set_drawing_hlink(
        pkg,
        docs,
        dirty,
        &super::apply::part_for_txbx(pkg, catalog, doc_part, target, id, i)?,
        id,
        i,
        op,
        |root| {
            let host = para::find_paragraph_mut(root, target.host.body())?;
            let drawing = textbox::find_textbox_drawing_mut(host, &target.key)?;
            super::drawing_hlink::textbox_click_parent_mut(drawing)
        },
        |root| {
            let host = para::find_paragraph_mut(root, target.host.body())?;
            let drawing = textbox::find_textbox_drawing_mut(host, &target.key)?;
            Some(super::drawing_hlink::textbox_has_click(drawing))
        },
        ErrorKind::TextBoxNotFound { id: id.to_string() },
    )
}
pub(crate) fn apply_set_wsp_hlink(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    id: &str,
    target: &para::TxbxTarget,
    op: &SetHyperlinkOp,
    i: usize,
    has_click: fn(&mut Element, &para::TxbxKey) -> Option<bool>,
    parent_mut: for<'a> fn(&'a mut Element, &para::TxbxKey) -> Option<&'a mut Element>,
    missing: ErrorKind,
) -> Result<(), Error> {
    let part_name = super::apply::part_for_txbx(pkg, catalog, doc_part, target, id, i)?;
    let key = target.key.clone();
    apply_set_drawing_hlink(
        pkg,
        docs,
        dirty,
        &part_name,
        id,
        i,
        op,
        |root| {
            let host = para::find_paragraph_mut(root, target.host.body())?;
            parent_mut(host, &key)
        },
        |root| {
            let host = para::find_paragraph_mut(root, target.host.body())?;
            has_click(host, &key)
        },
        missing,
    )
}
pub(crate) fn apply_set_ink_hlink(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    id: &str,
    target: &para::TxbxTarget,
    op: &SetHyperlinkOp,
    i: usize,
) -> Result<(), Error> {
    let part_name = ink::part_for_target(catalog, doc_part, target, id, &format!("ops[{i}].id"))?;
    let key = target.key.clone();
    super::apply::load_part(pkg, docs, &part_name)?;
    {
        let ctx = ink::HostCtx::load(pkg, &part_name)?;
        let doc = super::apply::ink_doc_mut(docs, &part_name)?;
        match ink::find_drawing_mut(&mut doc.root, &ctx, &key) {
            None => {
                return Err(Error::with_path(
                    ErrorKind::InkNotFound { id: id.to_string() },
                    format!("ops[{i}].id"),
                ));
            }
            Some(d) if super::drawing_hlink::picture_has_click(d) => {}
            Some(_) => {
                return Err(Error::with_path(
                    ErrorKind::HyperlinkNotFound { id: id.to_string() },
                    format!("ops[{i}].id"),
                ));
            }
        }
    }
    let old_rid = {
        let ctx = ink::HostCtx::load(pkg, &part_name)?;
        let doc = super::apply::ink_doc_mut(docs, &part_name)?;
        let drawing = ink::find_drawing_mut(&mut doc.root, &ctx, &key).ok_or_else(|| {
            Error::with_path(
                ErrorKind::HyperlinkNotFound { id: id.to_string() },
                format!("ops[{i}].id"),
            )
        })?;
        let parent = super::drawing_hlink::picture_click_parent_mut(drawing).ok_or_else(|| {
            Error::with_path(
                ErrorKind::HyperlinkNotFound { id: id.to_string() },
                format!("ops[{i}].id"),
            )
        })?;
        super::drawing_hlink::click_rid(parent)
    };
    let new_rid = if op.url.is_some() || op.anchor.is_some() {
        let mut rels = Relationships::from_package(pkg, &part_name)?;
        let rid = super::drawing_hlink::set_rel(
            &mut rels,
            old_rid.as_deref(),
            op.url.as_deref(),
            op.anchor.as_deref(),
        );
        rels.write_to(pkg, &part_name)?;
        Some(rid)
    } else {
        None
    };
    {
        let ctx = ink::HostCtx::load(pkg, &part_name)?;
        let doc = super::apply::ink_doc_mut(docs, &part_name)?;
        super::drawing_hlink::ensure_ns(&mut doc.root);
        let drawing = ink::find_drawing_mut(&mut doc.root, &ctx, &key).ok_or_else(|| {
            Error::with_path(
                ErrorKind::HyperlinkNotFound { id: id.to_string() },
                format!("ops[{i}].id"),
            )
        })?;
        let parent = super::drawing_hlink::picture_click_parent_mut(drawing).ok_or_else(|| {
            Error::with_path(
                ErrorKind::HyperlinkNotFound { id: id.to_string() },
                format!("ops[{i}].id"),
            )
        })?;
        let tooltip = op.tooltip.as_ref().map(|t| t.as_deref());
        if !super::drawing_hlink::set_click(parent, new_rid.as_deref(), tooltip) {
            return Err(Error::with_path(
                ErrorKind::HyperlinkNotFound { id: id.to_string() },
                format!("ops[{i}].id"),
            ));
        }
    }
    if let (Some(old), Some(new)) = (old_rid.as_deref(), new_rid.as_deref()) {
        if old != new {
            let doc = super::apply::load_part(pkg, docs, &part_name)?;
            super::link::drop_rel_if_unused(pkg, &doc.root, &part_name, old)?;
        }
    }
    dirty.insert(part_name, ());
    Ok(())
}
pub(crate) fn apply_set_model3d_hlink(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    id: &str,
    target: &para::TxbxTarget,
    op: &SetHyperlinkOp,
    i: usize,
) -> Result<(), Error> {
    let part_name =
        model3d::part_for_target(catalog, doc_part, target, id, &format!("ops[{i}].id"))?;
    let key = target.key.clone();
    apply_set_drawing_hlink(
        pkg,
        docs,
        dirty,
        &part_name,
        id,
        i,
        op,
        |root| {
            let frame = model3d::find_frame_mut(root, &key)?;
            super::drawing_hlink::picture_click_parent_mut(frame)
        },
        |root| {
            let frame = model3d::find_frame_mut(root, &key)?;
            Some(super::drawing_hlink::picture_has_click(frame))
        },
        ErrorKind::Model3dNotFound { id: id.to_string() },
    )
}
pub(crate) fn apply_set_chart_hlink(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    id: &str,
    target: &charts::ChartTarget,
    op: &SetHyperlinkOp,
    i: usize,
) -> Result<(), Error> {
    let part_name =
        charts::part_for_target(pkg, catalog, doc_part, target, id, &format!("ops[{i}].id"))?;
    apply_set_drawing_hlink(
        pkg,
        docs,
        dirty,
        &part_name,
        id,
        i,
        op,
        |root| {
            let drawing = charts::find_drawing_mut(root, target)?;
            super::drawing_hlink::picture_click_parent_mut(drawing)
        },
        |root| {
            let drawing = charts::find_drawing_mut(root, target)?;
            Some(super::drawing_hlink::picture_has_click(drawing))
        },
        ErrorKind::ChartNotFound { id: id.to_string() },
    )
}
pub(crate) fn apply_set_smartart_hlink(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    id: &str,
    target: &smartart::DiagramTarget,
    op: &SetHyperlinkOp,
    i: usize,
) -> Result<(), Error> {
    let part_name =
        smartart::part_for_target(pkg, catalog, doc_part, target, id, &format!("ops[{i}].id"))?;
    apply_set_drawing_hlink(
        pkg,
        docs,
        dirty,
        &part_name,
        id,
        i,
        op,
        |root| {
            let drawing = smartart::find_drawing_mut(root, target)?;
            super::drawing_hlink::picture_click_parent_mut(drawing)
        },
        |root| {
            let drawing = smartart::find_drawing_mut(root, target)?;
            Some(super::drawing_hlink::picture_has_click(drawing))
        },
        ErrorKind::SmartArtNotFound { id: id.to_string() },
    )
}
pub(crate) fn apply_set_drawing_hlink(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    part_name: &str,
    id: &str,
    i: usize,
    op: &SetHyperlinkOp,
    mut parent_mut: impl FnMut(&mut Element) -> Option<&mut Element>,
    mut exists: impl FnMut(&mut Element) -> Option<bool>,
    missing: ErrorKind,
) -> Result<(), Error> {
    {
        let doc = super::apply::load_part(pkg, docs, part_name)?;
        match exists(&mut doc.root) {
            Some(true) => {}
            Some(false) => {
                return Err(Error::with_path(
                    ErrorKind::HyperlinkNotFound { id: id.to_string() },
                    format!("ops[{i}].id"),
                ));
            }
            None => {
                return Err(Error::with_path(missing, format!("ops[{i}].id")));
            }
        }
    }
    let old_rid = {
        let doc = super::apply::load_part(pkg, docs, part_name)?;
        let parent = parent_mut(&mut doc.root).ok_or_else(|| {
            Error::with_path(
                ErrorKind::HyperlinkNotFound { id: id.to_string() },
                format!("ops[{i}].id"),
            )
        })?;
        super::drawing_hlink::click_rid(parent)
    };
    let new_rid = if op.url.is_some() || op.anchor.is_some() {
        let mut rels = Relationships::from_package(pkg, part_name)?;
        let rid = super::drawing_hlink::set_rel(
            &mut rels,
            old_rid.as_deref(),
            op.url.as_deref(),
            op.anchor.as_deref(),
        );
        rels.write_to(pkg, part_name)?;
        Some(rid)
    } else {
        None
    };
    {
        let doc = super::apply::load_part(pkg, docs, part_name)?;
        super::drawing_hlink::ensure_ns(&mut doc.root);
        let parent = parent_mut(&mut doc.root).ok_or_else(|| {
            Error::with_path(
                ErrorKind::HyperlinkNotFound { id: id.to_string() },
                format!("ops[{i}].id"),
            )
        })?;
        let tooltip = op.tooltip.as_ref().map(|t| t.as_deref());
        if !super::drawing_hlink::set_click(parent, new_rid.as_deref(), tooltip) {
            return Err(Error::with_path(
                ErrorKind::HyperlinkNotFound { id: id.to_string() },
                format!("ops[{i}].id"),
            ));
        }
    }
    if let (Some(old), Some(new)) = (old_rid.as_deref(), new_rid.as_deref()) {
        if old != new {
            let doc = super::apply::load_part(pkg, docs, part_name)?;
            super::link::drop_rel_if_unused(pkg, &doc.root, part_name, old)?;
        }
    }
    dirty.insert(part_name.to_string(), ());
    Ok(())
}
pub(crate) fn apply_delete_pic_hlink(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    id: &str,
    target: &picture::PicTarget,
    i: usize,
) -> Result<(), Error> {
    let part_name = super::apply::part_for_pic(pkg, catalog, doc_part, target, id, i)?;
    apply_delete_drawing_hlink(pkg, docs, dirty, &part_name, id, i, |root| {
        let drawing = super::picture::find_target_drawing_mut(root, target)?;
        picture_hlink_parent_mut(drawing, target)
    })
}
pub(crate) fn apply_delete_txbx_hlink(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    id: &str,
    target: &para::TxbxTarget,
    i: usize,
) -> Result<(), Error> {
    let part_name = super::apply::part_for_txbx(pkg, catalog, doc_part, target, id, i)?;
    apply_delete_drawing_hlink(pkg, docs, dirty, &part_name, id, i, |root| {
        let host = para::find_paragraph_mut(root, target.host.body())?;
        let drawing = textbox::find_textbox_drawing_mut(host, &target.key)?;
        super::drawing_hlink::textbox_click_parent_mut(drawing)
    })
}
pub(crate) fn apply_delete_wsp_hlink(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    id: &str,
    target: &para::TxbxTarget,
    i: usize,
    parent_mut: for<'a> fn(&'a mut Element, &para::TxbxKey) -> Option<&'a mut Element>,
) -> Result<(), Error> {
    let part_name = super::apply::part_for_txbx(pkg, catalog, doc_part, target, id, i)?;
    let key = target.key.clone();
    apply_delete_drawing_hlink(pkg, docs, dirty, &part_name, id, i, |root| {
        let host = para::find_paragraph_mut(root, target.host.body())?;
        parent_mut(host, &key)
    })
}
pub(crate) fn apply_delete_ink_hlink(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    id: &str,
    target: &para::TxbxTarget,
    i: usize,
) -> Result<(), Error> {
    let part_name = ink::part_for_target(catalog, doc_part, target, id, &format!("ops[{i}].id"))?;
    let key = target.key.clone();
    super::apply::load_part(pkg, docs, &part_name)?;
    let old_rid = {
        let ctx = ink::HostCtx::load(pkg, &part_name)?;
        let doc = super::apply::ink_doc_mut(docs, &part_name)?;
        let drawing = ink::find_drawing_mut(&mut doc.root, &ctx, &key).ok_or_else(|| {
            Error::with_path(
                ErrorKind::HyperlinkNotFound { id: id.to_string() },
                format!("ops[{i}].id"),
            )
        })?;
        let parent = super::drawing_hlink::picture_click_parent_mut(drawing).ok_or_else(|| {
            Error::with_path(
                ErrorKind::HyperlinkNotFound { id: id.to_string() },
                format!("ops[{i}].id"),
            )
        })?;
        super::drawing_hlink::delete_click(parent).ok_or_else(|| {
            Error::with_path(
                ErrorKind::HyperlinkNotFound { id: id.to_string() },
                format!("ops[{i}].id"),
            )
        })?
    };
    if let Some(rid) = old_rid {
        let doc = super::apply::load_part(pkg, docs, &part_name)?;
        super::link::drop_rel_if_unused(pkg, &doc.root, &part_name, &rid)?;
    }
    dirty.insert(part_name, ());
    Ok(())
}
pub(crate) fn apply_delete_model3d_hlink(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    id: &str,
    target: &para::TxbxTarget,
    i: usize,
) -> Result<(), Error> {
    let part_name =
        model3d::part_for_target(catalog, doc_part, target, id, &format!("ops[{i}].id"))?;
    let key = target.key.clone();
    apply_delete_drawing_hlink(pkg, docs, dirty, &part_name, id, i, |root| {
        let frame = model3d::find_frame_mut(root, &key)?;
        super::drawing_hlink::picture_click_parent_mut(frame)
    })
}
pub(crate) fn apply_delete_chart_hlink(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    id: &str,
    target: &charts::ChartTarget,
    i: usize,
) -> Result<(), Error> {
    let part_name =
        charts::part_for_target(pkg, catalog, doc_part, target, id, &format!("ops[{i}].id"))?;
    apply_delete_drawing_hlink(pkg, docs, dirty, &part_name, id, i, |root| {
        let drawing = charts::find_drawing_mut(root, target)?;
        super::drawing_hlink::picture_click_parent_mut(drawing)
    })
}
pub(crate) fn apply_delete_smartart_hlink(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    id: &str,
    target: &smartart::DiagramTarget,
    i: usize,
) -> Result<(), Error> {
    let part_name =
        smartart::part_for_target(pkg, catalog, doc_part, target, id, &format!("ops[{i}].id"))?;
    apply_delete_drawing_hlink(pkg, docs, dirty, &part_name, id, i, |root| {
        let drawing = smartart::find_drawing_mut(root, target)?;
        super::drawing_hlink::picture_click_parent_mut(drawing)
    })
}
pub(crate) fn apply_delete_drawing_hlink(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    part_name: &str,
    id: &str,
    i: usize,
    mut parent_mut: impl FnMut(&mut Element) -> Option<&mut Element>,
) -> Result<(), Error> {
    let old_rid = {
        let doc = super::apply::load_part(pkg, docs, part_name)?;
        let parent = parent_mut(&mut doc.root).ok_or_else(|| {
            Error::with_path(
                ErrorKind::HyperlinkNotFound { id: id.to_string() },
                format!("ops[{i}].id"),
            )
        })?;
        super::drawing_hlink::delete_click(parent).ok_or_else(|| {
            Error::with_path(
                ErrorKind::HyperlinkNotFound { id: id.to_string() },
                format!("ops[{i}].id"),
            )
        })?
    };
    if let Some(rid) = old_rid {
        let doc = super::apply::load_part(pkg, docs, part_name)?;
        super::link::drop_rel_if_unused(pkg, &doc.root, part_name, &rid)?;
    }
    dirty.insert(part_name.to_string(), ());
    Ok(())
}
pub(crate) fn picture_hlink_exists(drawing: &Element, target: &picture::PicTarget) -> bool {
    if picture::is_group_drawing(drawing) {
        picture::grouped_pic_has_click(drawing, target.key())
    } else {
        super::drawing_hlink::picture_has_click(drawing)
    }
}
pub(crate) fn picture_hlink_parent_mut<'a>(
    drawing: &'a mut Element,
    target: &picture::PicTarget,
) -> Option<&'a mut Element> {
    if picture::is_group_drawing(drawing) {
        picture::find_grouped_pic_cnv_pr_mut(drawing, target.key())
    } else {
        super::drawing_hlink::picture_click_parent_mut(drawing)
    }
}
