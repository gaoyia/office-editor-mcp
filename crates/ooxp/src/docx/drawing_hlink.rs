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
