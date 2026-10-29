//! 脚注、尾注部件：定位、插删用户注、改编号属性。不改注区怎么画。

use std::collections::{HashMap, HashSet};

use crate::error::Error;
use crate::opc::{self, ContentTypes, Package, Relationship, Relationships};
use crate::xml::{self, Document, Element, Node};

use super::hf::HfPart;
use super::para::{self, NoteKind};
use super::textbox;
use super::{MC_NS, REL_NS, W14_NS, WML_NS};

pub fn rel_type(kind: NoteKind) -> &'static str {
    match kind {
        NoteKind::Footnote => super::REL_FOOTNOTES,
        NoteKind::Endnote => super::REL_ENDNOTES,
    }
}

pub fn content_type(kind: NoteKind) -> &'static str {
    match kind {
        NoteKind::Footnote => super::CT_FOOTNOTES,
        NoteKind::Endnote => super::CT_ENDNOTES,
    }
}

pub fn ids_match(a: &str, b: &str) -> bool {
    if a == b {
        return true;
    }
    match (a.parse::<i64>(), b.parse::<i64>()) {
        (Ok(x), Ok(y)) => x == y,
        _ => false,
    }
}

pub fn is_user_note(el: &Element) -> bool {
    matches!(el.attr_local("type"), None | Some("normal"))
}

pub fn sep_type(el: &Element) -> Option<&str> {
    match el.attr_local("type") {
        Some(t @ ("separator" | "continuationSeparator" | "continuationNotice")) => Some(t),
        _ => None,
    }
}

pub fn part_name(pkg: &Package, doc_part: &str, kind: NoteKind) -> Result<Option<String>, Error> {
    let rels = Relationships::from_package(pkg, doc_part)?;
    let Some(rel) = rels
        .find_by_type_suffix(kind.rel_suffix())
        .into_iter()
        .next()
    else {
        return Ok(None);
    };
    if rel.is_external() {
        return Ok(None);
    }
    Ok(Some(opc::resolve_rel_target(doc_part, &rel.target)?))
}

pub fn settings_part_name(pkg: &Package, doc_part: &str) -> Result<Option<String>, Error> {
    let rels = Relationships::from_package(pkg, doc_part)?;
    let Some(rel) = rels.find_by_type_suffix("/settings").into_iter().next() else {
        return Ok(None);
    };
    if rel.is_external() {
        return Ok(None);
    }
    Ok(Some(opc::resolve_rel_target(doc_part, &rel.target)?))
}

pub fn user_count(pkg: &Package, doc_part: &str, kind: NoteKind) -> Result<usize, Error> {
    let Some(part) = part_name(pkg, doc_part, kind)? else {
        return Ok(0);
    };
    let Ok(bytes) = pkg.get(&part) else {
        return Ok(0);
    };
    let doc = xml::parse(bytes)?;
    Ok(user_notes(&doc.root, kind).len())
}

pub fn user_notes(root: &Element, kind: NoteKind) -> Vec<&Element> {
    let local = kind.item_local();
    root.element_children()
        .filter(|e| e.local_name() == local && is_user_note(e))
        .collect()
}

pub fn seps(root: &Element, kind: NoteKind) -> Vec<&Element> {
    let local = kind.item_local();
    root.element_children()
        .filter(|e| e.local_name() == local && sep_type(e).is_some())
        .collect()
}

pub fn find_user_note<'a>(root: &'a Element, kind: NoteKind, note_id: &str) -> Option<&'a Element> {
    user_notes(root, kind)
        .into_iter()
        .find(|e| ids_match(e.attr_local("id").unwrap_or(""), note_id))
}

pub fn find_user_note_mut<'a>(
    root: &'a mut Element,
    kind: NoteKind,
    note_id: &str,
) -> Option<&'a mut Element> {
    let local = kind.item_local();
    for child in root.children.iter_mut() {
        if let Node::Element(e) = child {
            if e.local_name() == local
                && is_user_note(e)
                && ids_match(e.attr_local("id").unwrap_or(""), note_id)
            {
                return Some(e);
            }
        }
    }
    None
}

pub fn next_user_id(root: &Element, kind: NoteKind) -> String {
    let local = kind.item_local();
    let mut used_i = HashSet::new();
    let mut used_s = HashSet::new();
    for e in root.element_children() {
        if e.local_name() != local {
            continue;
        }
        if let Some(id) = e.attr_local("id") {
            used_s.insert(id.to_string());
            if let Ok(n) = id.parse::<i64>() {
                used_i.insert(n);
            }
        }
    }
    let mut n = 2i64;
    loop {
        let s = n.to_string();
        if !used_i.contains(&n) && !used_s.contains(&s) {
            return s;
        }
        n += 1;
        if n > 1_000_000 {
            return s;
        }
    }
}

/// 该段自身的引用（遇 `w:txbxContent` 停住，框内引用算框内段的）。
pub fn refs_in_para(p: &Element, kind: NoteKind) -> Vec<String> {
    let mut out = Vec::new();
    collect_refs(p, kind, &mut out, true);
    out
}

pub fn collect_blip_embeds(el: &Element) -> Vec<String> {
    let mut out = Vec::new();
    walk_embeds(el, &mut out);
    out
}

pub fn collect_hosts(
    pkg: &Package,
    doc_part: &str,
    doc_root: &Element,
    catalog: &[HfPart],
    kind: NoteKind,
) -> Result<Vec<(String, String)>, Error> {
    let mut out = Vec::new();
    collect_hosts_in_root(doc_root, None, kind, &mut out)?;
    for item in catalog {
        let bytes = pkg.get(&item.part)?;
        let doc = xml::parse(bytes)?;
        collect_hosts_in_root(&doc.root, Some(&item.id), kind, &mut out)?;
    }
    let _ = doc_part;
    Ok(out)
}

pub fn append_reference(p: &mut Element, kind: NoteKind, w_id: &str) {
    let mut rpr = Element::new(p.prefixed("rPr"));
    let mut style = Element::new(p.prefixed("rStyle"));
    style.set_attr(&p.prefixed("val"), kind.ref_style());
    rpr.push_element(style);
    let mut rf = Element::new(p.prefixed(kind.reference_local()));
    rf.set_attr(&p.prefixed("id"), w_id);
    let mut r = Element::new(p.prefixed("r"));
    r.push_element(rpr);
    r.push_element(rf);
    p.children.push(Node::Element(r));
}

pub fn append_user_note(root: &mut Element, kind: NoteKind, w_id: &str, para_id: &str, text: &str) {
    ensure_xmlns(root, "w14", W14_NS);
    root.push_element(new_user_note(root, kind, w_id, para_id, text));
}

pub fn remove_user_note(root: &mut Element, kind: NoteKind, note_id: &str) -> bool {
    let local = kind.item_local();
    let mut idx = None;
    for (i, n) in root.children.iter().enumerate() {
        if let Node::Element(e) = n {
            if e.local_name() == local
                && is_user_note(e)
                && ids_match(e.attr_local("id").unwrap_or(""), note_id)
            {
                idx = Some(i);
                break;
            }
        }
    }
    if let Some(i) = idx {
        root.children.remove(i);
        true
    } else {
        false
    }
}

pub fn remove_references(root: &mut Element, kind: NoteKind, note_id: &str) {
    strip_refs(root, kind, note_id);
}

pub fn ensure_part(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    kind: NoteKind,
) -> Result<String, Error> {
    if let Some(existing) = part_name(pkg, doc_part, kind)? {
        return Ok(existing);
    }
    let target = kind.file_name();
    let part = opc::resolve_rel_target(doc_part, target)?;
    let xml = minimal_part_xml(kind);
    pkg.set(&part, xml.into_bytes())?;
    let parsed = xml::parse(pkg.get(&part)?)?;
    docs.insert(part.clone(), parsed);
    dirty.insert(part.clone(), ());

    let mut rels = Relationships::from_package(pkg, doc_part)?;
    let rid = rels.next_rid();
    rels.push(Relationship {
        id: rid,
        rel_type: rel_type(kind).into(),
        target: target.into(),
        target_mode: None,
        extra: Vec::new(),
    });
    rels.write_to(pkg, doc_part)?;

    let mut cts = ContentTypes::from_package(pkg)?;
    cts.set_override(&part, content_type(kind))?;
    cts.write_to(pkg)?;

    if let Some(settings) = settings_part_name(pkg, doc_part)? {
        let doc = load_into(pkg, docs, &settings)?;
        ensure_special_refs(&mut doc.root, kind, &["0", "1"]);
        dirty.insert(settings, ());
    }
    Ok(part)
}

pub fn apply_numbering(
    parent: &mut Element,
    kind: NoteKind,
    num_fmt: Option<&str>,
    num_start: Option<u32>,
    num_restart: Option<&str>,
) {
    let pr = if parent.local_name() == "sectPr" {
        ensure_sect_note_pr(parent, kind)
    } else {
        ensure_child(parent, kind.pr_local())
    };
    if let Some(fmt) = num_fmt {
        set_val_child(pr, "numFmt", fmt);
    }
    if let Some(start) = num_start {
        set_val_child(pr, "numStart", &start.to_string());
    }
    if let Some(restart) = num_restart {
        set_val_child(pr, "numRestart", restart);
    }
}

pub fn ensure_special_refs(settings_root: &mut Element, kind: NoteKind, ids: &[&str]) {
    let pr = ensure_child(settings_root, kind.pr_local());
    let child_local = kind.item_local();
    for id in ids {
        let exists = pr.element_children().any(|e| {
            e.local_name() == child_local && ids_match(e.attr_local("id").unwrap_or(""), id)
        });
        if exists {
            continue;
        }
        let mut el = Element::new(pr.prefixed(child_local));
        el.set_attr(&pr.prefixed("id"), *id);
        pr.push_element(el);
    }
}

fn load_into<'a>(
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

fn collect_hosts_in_root(
    root: &Element,
    container: Option<&str>,
    kind: NoteKind,
    out: &mut Vec<(String, String)>,
) -> Result<(), Error> {
    let paras = para::collect_paragraphs(root);
    for (i, p) in paras.iter().enumerate() {
        let index = i + 1;
        let (pid, _) = match container {
            Some(c) => para::para_stable_id_in(c, p, index),
            None => para::para_stable_id(p, index),
        };
        for rid in refs_in_para(p, kind) {
            out.push((rid, pid.clone()));
        }
        let boxes = textbox::collect_textboxes(p, &pid)?;
        for b in boxes {
            let Ok(target) = para::parse_txbx_id_at(&b.id, "host") else {
                continue;
            };
            let Some(content) = textbox::find_txbx_content_by_key(p, &target.key) else {
                continue;
            };
            let inner = para::collect_paragraphs_in_txbx(content);
            for (j, ip) in inner.iter().enumerate() {
                let (iid, _) = para::para_stable_id_in(&b.id, ip, j + 1);
                for rid in refs_in_para(ip, kind) {
                    out.push((rid, iid.clone()));
                }
            }
        }
    }
    Ok(())
}

fn collect_refs(el: &Element, kind: NoteKind, out: &mut Vec<String>, stop_txbx: bool) {
    if stop_txbx && el.local_name() == "txbxContent" {
        return;
    }
    if el.local_name() == "AlternateContent" {
        if let Some(i) = first_alt_child_index(el) {
            if let Node::Element(child) = &el.children[i] {
                collect_refs(child, kind, out, stop_txbx);
            }
        }
        return;
    }
    if el.local_name() == kind.reference_local() {
        if let Some(id) = el.attr_local("id") {
            out.push(id.to_string());
        }
        return;
    }
    for n in &el.children {
        if let Node::Element(e) = n {
            collect_refs(e, kind, out, stop_txbx);
        }
    }
}

fn walk_embeds(el: &Element, out: &mut Vec<String>) {
    crate::xml::walk_elements(el, &mut |node| {
        if node.local_name() == "blip" {
            if let Some(rid) = node.attr_local("embed") {
                out.push(rid.to_string());
            }
        }
    });
}

fn strip_refs(el: &mut Element, kind: NoteKind, note_id: &str) {
    if el.local_name() == "AlternateContent" {
        if let Some(i) = first_alt_child_index(el) {
            if let Node::Element(child) = &mut el.children[i] {
                strip_refs(child, kind, note_id);
            }
        }
        return;
    }
    let mut drop = Vec::new();
    for (i, n) in el.children.iter_mut().enumerate() {
        if let Node::Element(e) = n {
            if e.local_name() == kind.reference_local()
                && ids_match(e.attr_local("id").unwrap_or(""), note_id)
            {
                drop.push(i);
                continue;
            }
            strip_refs(e, kind, note_id);
            if e.local_name() == "r" && run_only_rpr(e) {
                drop.push(i);
            }
        }
    }
    for i in drop.into_iter().rev() {
        el.children.remove(i);
    }
}

fn run_only_rpr(r: &Element) -> bool {
    !r.element_children().any(|e| e.local_name() != "rPr")
}

fn new_user_note(
    parent: &Element,
    kind: NoteKind,
    w_id: &str,
    para_id: &str,
    text: &str,
) -> Element {
    let mut note = Element::new(parent.prefixed(kind.item_local()));
    note.set_attr(&parent.prefixed("id"), w_id);
    let mut p = Element::new(parent.prefixed("p"));
    p.set_attr("w14:paraId", para_id);
    p.set_attr("w14:textId", para_id);
    let mut rpr = Element::new(parent.prefixed("rPr"));
    let mut style = Element::new(parent.prefixed("rStyle"));
    style.set_attr(&parent.prefixed("val"), kind.ref_style());
    rpr.push_element(style);
    let mut mark = Element::new(parent.prefixed("r"));
    mark.push_element(rpr);
    mark.push_element(Element::new(parent.prefixed(kind.ref_mark_local())));
    p.push_element(mark);
    if !text.is_empty() {
        let mut t = Element::new(parent.prefixed("t"));
        if needs_xml_space(text) {
            t.set_attr("xml:space", "preserve");
        }
        t.set_text(text);
        let mut r = Element::new(parent.prefixed("r"));
        r.push_element(t);
        p.push_element(r);
    }
    note.push_element(p);
    note
}

fn needs_xml_space(text: &str) -> bool {
    text.starts_with(' ')
        || text.starts_with('\t')
        || text.ends_with(' ')
        || text.ends_with('\t')
        || text.contains('\n')
}

fn minimal_part_xml(kind: NoteKind) -> String {
    let tag = match kind {
        NoteKind::Footnote => "footnotes",
        NoteKind::Endnote => "endnotes",
    };
    let item = kind.item_local();
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:{tag} xmlns:w="{WML_NS}" xmlns:r="{REL_NS}" xmlns:w14="{W14_NS}" xmlns:mc="{MC_NS}" mc:Ignorable="w14"><w:{item} w:type="separator" w:id="0"><w:p w14:paraId="00000001" w14:textId="00000001"><w:r><w:separator/></w:r></w:p></w:{item}><w:{item} w:type="continuationSeparator" w:id="1"><w:p w14:paraId="00000002" w14:textId="00000002"><w:r><w:continuationSeparator/></w:r></w:p></w:{item}></w:{tag}>"#
    )
}

fn ensure_child<'a>(parent: &'a mut Element, local: &str) -> &'a mut Element {
    if parent.child_by_local(local).is_some() {
        return parent.child_by_local_mut(local).unwrap();
    }
    parent.push_element(Element::new(parent.prefixed(local)));
    parent.child_by_local_mut(local).unwrap()
}

/// `EG_SectPrContents`：`footnotePr` / `endnotePr` 在页眉页脚引用之后、`pgSz` 之前。
fn ensure_sect_note_pr(sect: &mut Element, kind: NoteKind) -> &mut Element {
    let local = kind.pr_local();
    let dest = insert_index_by_order(sect, local, SECT_PR_ORDER);
    if let Some(i) = child_index(sect, local) {
        if dest != i && dest != i + 1 {
            let node = sect.children.remove(i);
            let dest = if dest > i { dest - 1 } else { dest };
            sect.children.insert(dest, node);
        }
        return sect.child_by_local_mut(local).unwrap();
    }
    sect.children
        .insert(dest, Node::Element(Element::new(sect.prefixed(local))));
    sect.child_by_local_mut(local).unwrap()
}

const SECT_PR_ORDER: &[&str] = &[
    "headerReference",
    "footerReference",
    "footnotePr",
    "endnotePr",
    "type",
    "pgSz",
    "pgMar",
    "paperSrc",
    "pgBorders",
    "lnNumType",
    "pgNumType",
    "cols",
    "formProt",
    "vAlign",
    "noEndnote",
    "titlePg",
    "textDirection",
    "bidi",
    "rtlGutter",
    "docGrid",
    "printerSettings",
];

fn set_val_child(parent: &mut Element, local: &str, val: &str) {
    if let Some(i) = child_index(parent, local) {
        if let Node::Element(child) = &mut parent.children[i] {
            set_val_attr(child, val);
        }
        let dest = prop_insert_index(parent, local);
        if dest != i && dest != i + 1 {
            let node = parent.children.remove(i);
            let dest = if dest > i { dest - 1 } else { dest };
            parent.children.insert(dest, node);
        }
        return;
    }
    let mut child = Element::new(parent.prefixed(local));
    child.set_attr(&parent.prefixed("val"), val);
    let at = prop_insert_index(parent, local);
    parent.children.insert(at, Node::Element(child));
}

fn child_index(parent: &Element, local: &str) -> Option<usize> {
    parent.children.iter().position(|n| match n {
        Node::Element(e) => e.local_name() == local,
        Node::Text(_) => false,
    })
}

/// `CT_FtnProps` / `CT_EdnProps` 在特殊引用之前：pos、numFmt、numStart、numRestart。
fn prop_insert_index(parent: &Element, local: &str) -> usize {
    const ORDER: &[&str] = &["pos", "numFmt", "numStart", "numRestart"];
    let my = ORDER
        .iter()
        .position(|&x| x == local)
        .unwrap_or(ORDER.len());
    let mut after_lower = 0usize;
    for (i, n) in parent.children.iter().enumerate() {
        let Node::Element(e) = n else {
            continue;
        };
        let name = e.local_name();
        if name == "footnote" || name == "endnote" {
            return after_lower.max(i);
        }
        if let Some(rank) = ORDER.iter().position(|&x| x == name) {
            if rank < my {
                after_lower = i + 1;
            } else if rank > my {
                return i;
            }
        }
    }
    after_lower
}

fn insert_index_by_order(parent: &Element, local: &str, order: &[&str]) -> usize {
    let my = order
        .iter()
        .position(|&x| x == local)
        .unwrap_or(order.len());
    let mut after_lower = 0usize;
    for (i, n) in parent.children.iter().enumerate() {
        let Node::Element(e) = n else {
            continue;
        };
        let Some(rank) = order.iter().position(|&x| x == e.local_name()) else {
            continue;
        };
        if rank < my {
            after_lower = i + 1;
        } else if rank > my {
            return i;
        }
    }
    after_lower
}

fn set_val_attr(el: &mut Element, val: &str) {
    if let Some((_, v)) = el
        .attrs
        .iter_mut()
        .find(|(k, _)| k.rsplit(':').next() == Some("val"))
    {
        *v = val.to_string();
        return;
    }
    el.set_attr(&el.prefixed("val"), val);
}

fn ensure_xmlns(el: &mut Element, prefix: &str, uri: &str) {
    let attr = format!("xmlns:{prefix}");
    if el.attr(&attr).is_none() {
        el.set_attr(&attr, uri);
    }
}

fn first_alt_child_index(el: &Element) -> Option<usize> {
    let mut fallback = None;
    for (i, n) in el.children.iter().enumerate() {
        if let Node::Element(e) = n {
            if e.local_name() == "Choice" {
                return Some(i);
            }
            if e.local_name() == "Fallback" && fallback.is_none() {
                fallback = Some(i);
            }
        }
    }
    fallback
}
