//! 批注部件：定位、插删、改字与完成标记。不做气泡排版。

use std::collections::{HashMap, HashSet};

use crate::error::{Error, ErrorKind};
use crate::opc::{self, ContentTypes, Package, Relationship, Relationships};
use crate::xml::{self, Document, Element, Node};

use super::hf::HfPart;
use super::notes;
use super::para::{self, BodyPara, NoteKind, ParaTarget};
use super::textbox;
use super::{MC_NS, REL_NS, W14_NS, WML_NS};

pub const W15_NS: &str = "http://schemas.microsoft.com/office/word/2012/wordml";

pub fn ids_match(a: &str, b: &str) -> bool {
    notes::ids_match(a, b)
}

pub fn inspect_id(w_id: &str) -> String {
    format!("comment:{w_id}")
}

pub fn part_name(pkg: &Package, doc_part: &str) -> Result<Option<String>, Error> {
    rel_target(pkg, doc_part, "/comments")
}

pub fn extended_part_name(pkg: &Package, doc_part: &str) -> Result<Option<String>, Error> {
    rel_target(pkg, doc_part, "/commentsExtended")
}

pub fn people_part_name(pkg: &Package, doc_part: &str) -> Result<Option<String>, Error> {
    rel_target(pkg, doc_part, "/people")
}

pub fn ids_part_name(pkg: &Package, doc_part: &str) -> Result<Option<String>, Error> {
    rel_target(pkg, doc_part, "/commentsIds")
}

pub fn extensible_part_name(pkg: &Package, doc_part: &str) -> Result<Option<String>, Error> {
    rel_target(pkg, doc_part, "/commentsExtensible")
}

fn rel_target(pkg: &Package, doc_part: &str, suffix: &str) -> Result<Option<String>, Error> {
    let rels = Relationships::from_package(pkg, doc_part)?;
    let Some(rel) = rels.find_by_type_suffix(suffix).into_iter().next() else {
        return Ok(None);
    };
    if rel.is_external() {
        return Ok(None);
    }
    Ok(Some(opc::resolve_rel_target(doc_part, &rel.target)?))
}

pub fn count(pkg: &Package, doc_part: &str) -> Result<usize, Error> {
    let Some(part) = part_name(pkg, doc_part)? else {
        return Ok(0);
    };
    let Ok(bytes) = pkg.get(&part) else {
        return Ok(0);
    };
    let doc = xml::parse(bytes)?;
    Ok(comment_elements(&doc.root).len())
}

pub fn comment_elements(root: &Element) -> Vec<&Element> {
    root.element_children()
        .filter(|e| e.local_name() == "comment")
        .collect()
}

pub fn find_comment<'a>(root: &'a Element, comment_id: &str) -> Option<&'a Element> {
    comment_elements(root)
        .into_iter()
        .find(|e| ids_match(e.attr_local("id").unwrap_or(""), comment_id))
}

pub fn find_comment_mut<'a>(root: &'a mut Element, comment_id: &str) -> Option<&'a mut Element> {
    for child in root.children.iter_mut() {
        if let Node::Element(e) = child {
            if e.local_name() == "comment"
                && ids_match(e.attr_local("id").unwrap_or(""), comment_id)
            {
                return Some(e);
            }
        }
    }
    None
}

pub fn is_ink(el: &Element) -> bool {
    ink_in(el)
}

fn ink_in(el: &Element) -> bool {
    let local = el.local_name();
    if local == "ink" || local.eq_ignore_ascii_case("inkml") {
        return true;
    }
    if el.name.contains("inkml") {
        return true;
    }
    el.element_children().any(ink_in)
}

pub fn last_para_id(comment: &Element) -> Option<String> {
    last_para(comment)
        .and_then(|p| p.attr_local("paraId"))
        .map(str::to_string)
}

pub fn last_para(comment: &Element) -> Option<&Element> {
    para::collect_paragraphs(comment).into_iter().last()
}

pub fn next_comment_id(root: &Element) -> String {
    let mut used_i = HashSet::new();
    let mut used_s = HashSet::new();
    for e in comment_elements(root) {
        if let Some(id) = e.attr_local("id") {
            used_s.insert(id.to_string());
            if let Ok(n) = id.parse::<i64>() {
                used_i.insert(n);
            }
        }
    }
    let mut n = 0i64;
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

pub fn para_ids_match(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
}

/// 范围起止与各段上的 `commentIds`（回复没有自己的标记时沿用线程根）。
pub struct CommentCoverage {
    pub start: HashMap<String, Option<String>>,
    pub end: HashMap<String, Option<String>>,
    pub on_para: HashMap<String, Vec<String>>,
}

pub fn collect_coverage(
    pkg: &Package,
    doc_part: &str,
    doc_root: &Element,
    catalog: &[HfPart],
    comments_root: &Element,
    extended_root: Option<&Element>,
) -> Result<CommentCoverage, Error> {
    let mut start = HashMap::new();
    let mut end = HashMap::new();
    let mut start_ord: HashMap<String, usize> = HashMap::new();
    let mut on_para: HashMap<String, Vec<String>> = HashMap::new();
    let mut ord = 0usize;
    walk_stories(
        pkg,
        doc_part,
        doc_root,
        catalog,
        &mut start,
        &mut end,
        &mut start_ord,
        &mut on_para,
        &mut ord,
    )?;

    let last_by_comment = last_para_map(comments_root);
    let parent_of = parent_map(comments_root, extended_root, &last_by_comment);
    let xml_ord = xml_order(comments_root);

    for el in comment_elements(comments_root) {
        let cid = el.attr_local("id").unwrap_or("").to_string();
        if cid.is_empty() {
            continue;
        }
        start.entry(cid.clone()).or_insert(None);
        end.entry(cid.clone()).or_insert(None);
        let root = thread_root(&cid, &parent_of);
        if root != cid {
            if start.get(&cid).and_then(|v| v.as_ref()).is_none() {
                if let Some(s) = start.get(&root).cloned().flatten() {
                    start.insert(cid.clone(), Some(s));
                }
            }
            if end.get(&cid).and_then(|v| v.as_ref()).is_none() {
                if let Some(e) = end.get(&root).cloned().flatten() {
                    end.insert(cid.clone(), Some(e));
                }
            }
            let paras: Vec<String> = on_para
                .iter()
                .filter(|(_, ids)| ids.iter().any(|id| id == &root))
                .map(|(p, _)| p.clone())
                .collect();
            for p in paras {
                let slot = on_para.entry(p).or_default();
                if !slot.iter().any(|id| id == &cid) {
                    slot.push(cid.clone());
                }
            }
        }
    }

    for ids in on_para.values_mut() {
        ids.sort_by(|a, b| {
            let ra = thread_root(a, &parent_of);
            let rb = thread_root(b, &parent_of);
            let oa = start_ord.get(&ra).copied().unwrap_or(usize::MAX);
            let ob = start_ord.get(&rb).copied().unwrap_or(usize::MAX);
            oa.cmp(&ob).then_with(|| {
                xml_ord
                    .get(a)
                    .copied()
                    .unwrap_or(usize::MAX)
                    .cmp(&xml_ord.get(b).copied().unwrap_or(usize::MAX))
            })
        });
    }

    let mut start_out = HashMap::new();
    let mut end_out = HashMap::new();
    for el in comment_elements(comments_root) {
        let cid = el.attr_local("id").unwrap_or("").to_string();
        start_out.insert(cid.clone(), start.get(&cid).cloned().flatten());
        end_out.insert(cid.clone(), end.get(&cid).cloned().flatten());
    }

    Ok(CommentCoverage {
        start: start_out,
        end: end_out,
        on_para,
    })
}

fn xml_order(root: &Element) -> HashMap<String, usize> {
    let mut m = HashMap::new();
    for (i, e) in comment_elements(root).into_iter().enumerate() {
        if let Some(id) = e.attr_local("id") {
            m.insert(id.to_string(), i);
        }
    }
    m
}

fn last_para_map(root: &Element) -> HashMap<String, String> {
    let mut m = HashMap::new();
    for e in comment_elements(root) {
        let Some(cid) = e.attr_local("id") else {
            continue;
        };
        if let Some(pid) = last_para_id(e) {
            m.insert(cid.to_string(), pid);
        }
    }
    m
}

/// commentEx 的 paraId → (done, paraIdParent)
fn comment_ex_by_para(extended: &Element) -> HashMap<String, (bool, Option<String>)> {
    let mut m = HashMap::new();
    for e in extended.element_children() {
        if e.local_name() != "commentEx" {
            continue;
        }
        let Some(pid) = e.attr_local("paraId") else {
            continue;
        };
        let done = matches!(e.attr_local("done").unwrap_or(""), "1" | "true" | "on");
        let parent = e
            .attr_local("paraIdParent")
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        m.insert(pid.to_string(), (done, parent));
    }
    m
}

pub fn done_and_parent(
    comment: &Element,
    comments_root: &Element,
    extended_root: Option<&Element>,
) -> (bool, Option<String>) {
    let Some(extended) = extended_root else {
        return (false, None);
    };
    let last = match last_para_id(comment) {
        Some(p) => p,
        None => return (false, None),
    };
    let ex = comment_ex_by_para(extended);
    let Some((_, parent_para)) = ex
        .iter()
        .find(|(pid, _)| para_ids_match(pid, &last))
        .map(|(_, v)| v.clone())
    else {
        return (false, None);
    };
    let done = ex
        .iter()
        .find(|(pid, _)| para_ids_match(pid, &last))
        .map(|(_, (d, _))| *d)
        .unwrap_or(false);
    let parent = parent_para.and_then(|pp| {
        for e in comment_elements(comments_root) {
            if last_para_id(e).is_some_and(|lp| para_ids_match(&lp, &pp)) {
                return e.attr_local("id").map(inspect_id);
            }
        }
        None
    });
    (done, parent)
}

fn parent_map(
    _comments_root: &Element,
    extended_root: Option<&Element>,
    last_by_comment: &HashMap<String, String>,
) -> HashMap<String, String> {
    let mut out = HashMap::new();
    let Some(extended) = extended_root else {
        return out;
    };
    let ex = comment_ex_by_para(extended);
    for (cid, last) in last_by_comment {
        let Some((_, (_, Some(pp)))) = ex.iter().find(|(pid, _)| para_ids_match(pid, last)) else {
            continue;
        };
        for (other, other_last) in last_by_comment {
            if other != cid && para_ids_match(other_last, pp) {
                out.insert(cid.clone(), other.clone());
                break;
            }
        }
    }
    out
}

fn thread_root(cid: &str, parent_of: &HashMap<String, String>) -> String {
    let mut cur = cid.to_string();
    let mut seen = HashSet::new();
    while let Some(p) = parent_of.get(&cur) {
        if !seen.insert(cur.clone()) {
            break;
        }
        cur = p.clone();
    }
    cur
}

fn walk_stories(
    pkg: &Package,
    doc_part: &str,
    doc_root: &Element,
    catalog: &[HfPart],
    start: &mut HashMap<String, Option<String>>,
    end: &mut HashMap<String, Option<String>>,
    start_ord: &mut HashMap<String, usize>,
    on_para: &mut HashMap<String, Vec<String>>,
    ord: &mut usize,
) -> Result<(), Error> {
    walk_root_story(doc_root, None, start, end, start_ord, on_para, ord)?;
    for item in catalog {
        let bytes = pkg.get(&item.part)?;
        let doc = xml::parse(bytes)?;
        walk_root_story(
            &doc.root,
            Some(&item.id),
            start,
            end,
            start_ord,
            on_para,
            ord,
        )?;
    }
    for kind in [NoteKind::Footnote, NoteKind::Endnote] {
        let Some(part) = notes::part_name(pkg, doc_part, kind)? else {
            continue;
        };
        let bytes = pkg.get(&part)?;
        let doc = xml::parse(bytes)?;
        for note in notes::user_notes(&doc.root, kind) {
            let nid = note.attr_local("id").unwrap_or("");
            let container = kind.inspect_id(nid);
            walk_container_paras(note, &container, start, end, start_ord, on_para, ord)?;
        }
    }
    Ok(())
}

fn walk_root_story(
    root: &Element,
    container: Option<&str>,
    start: &mut HashMap<String, Option<String>>,
    end: &mut HashMap<String, Option<String>>,
    start_ord: &mut HashMap<String, usize>,
    on_para: &mut HashMap<String, Vec<String>>,
    ord: &mut usize,
) -> Result<(), Error> {
    let paras = para::collect_paragraphs(root);
    let mut active: HashSet<String> = HashSet::new();
    for (i, p) in paras.iter().enumerate() {
        let (pid, _) = match container {
            Some(c) => para::para_stable_id_in(c, p, i + 1),
            None => para::para_stable_id(p, i + 1),
        };
        apply_markers(p, &pid, &mut active, start, end, start_ord, on_para, ord);
        let boxes = textbox::collect_textboxes(p, &pid)?;
        for b in boxes {
            let Ok(target) = para::parse_txbx_id_at(&b.id, "host") else {
                continue;
            };
            let Some(content) = textbox::find_txbx_content_by_key(p, &target.key) else {
                continue;
            };
            walk_txbx_story(content, &b.id, start, end, start_ord, on_para, ord);
        }
    }
    Ok(())
}

fn walk_container_paras(
    container_el: &Element,
    container_id: &str,
    start: &mut HashMap<String, Option<String>>,
    end: &mut HashMap<String, Option<String>>,
    start_ord: &mut HashMap<String, usize>,
    on_para: &mut HashMap<String, Vec<String>>,
    ord: &mut usize,
) -> Result<(), Error> {
    let paras = para::collect_paragraphs(container_el);
    let mut active: HashSet<String> = HashSet::new();
    for (i, p) in paras.iter().enumerate() {
        let (pid, _) = para::para_stable_id_in(container_id, p, i + 1);
        apply_markers(p, &pid, &mut active, start, end, start_ord, on_para, ord);
        let boxes = textbox::collect_textboxes(p, &pid)?;
        for b in boxes {
            let Ok(target) = para::parse_txbx_id_at(&b.id, "host") else {
                continue;
            };
            let Some(content) = textbox::find_txbx_content_by_key(p, &target.key) else {
                continue;
            };
            walk_txbx_story(content, &b.id, start, end, start_ord, on_para, ord);
        }
    }
    Ok(())
}

fn walk_txbx_story(
    content: &Element,
    box_id: &str,
    start: &mut HashMap<String, Option<String>>,
    end: &mut HashMap<String, Option<String>>,
    start_ord: &mut HashMap<String, usize>,
    on_para: &mut HashMap<String, Vec<String>>,
    ord: &mut usize,
) {
    let paras = para::collect_paragraphs_in_txbx(content);
    let mut active: HashSet<String> = HashSet::new();
    for (i, p) in paras.iter().enumerate() {
        let (pid, _) = para::para_stable_id_in(box_id, p, i + 1);
        apply_markers(p, &pid, &mut active, start, end, start_ord, on_para, ord);
    }
}

fn apply_markers(
    p: &Element,
    pid: &str,
    active: &mut HashSet<String>,
    start: &mut HashMap<String, Option<String>>,
    end: &mut HashMap<String, Option<String>>,
    start_ord: &mut HashMap<String, usize>,
    on_para: &mut HashMap<String, Vec<String>>,
    ord: &mut usize,
) {
    let mut starts = Vec::new();
    let mut ends = Vec::new();
    collect_markers(p, &mut starts, &mut ends, true);
    for id in &starts {
        if !start_ord.contains_key(id) {
            start_ord.insert(id.clone(), *ord);
            *ord += 1;
        }
        start.entry(id.clone()).or_insert(None);
        if start.get(id).and_then(|v| v.as_ref()).is_none() {
            start.insert(id.clone(), Some(pid.to_string()));
        }
        active.insert(id.clone());
    }
    for id in active.iter() {
        let slot = on_para.entry(pid.to_string()).or_default();
        if !slot.iter().any(|x| x == id) {
            slot.push(id.clone());
        }
    }
    for id in &ends {
        end.entry(id.clone()).or_insert(None);
        end.insert(id.clone(), Some(pid.to_string()));
        active.remove(id);
    }
}

fn collect_markers(
    el: &Element,
    starts: &mut Vec<String>,
    ends: &mut Vec<String>,
    stop_txbx: bool,
) {
    if stop_txbx && el.local_name() == "txbxContent" {
        return;
    }
    if el.local_name() == "AlternateContent" {
        if let Some(i) = first_alt_child_index(el) {
            if let Node::Element(child) = &el.children[i] {
                collect_markers(child, starts, ends, stop_txbx);
            }
        }
        return;
    }
    match el.local_name() {
        "commentRangeStart" => {
            if let Some(id) = el.attr_local("id") {
                starts.push(id.to_string());
            }
        }
        "commentRangeEnd" => {
            if let Some(id) = el.attr_local("id") {
                ends.push(id.to_string());
            }
        }
        _ => {}
    }
    for n in &el.children {
        if let Node::Element(e) = n {
            collect_markers(e, starts, ends, stop_txbx);
        }
    }
}

pub fn reject_ink(comment: &Element, id: &str, i: usize) -> Result<(), Error> {
    if is_ink(comment) {
        return Err(Error::with_path(
            ErrorKind::InkComment { id: id.to_string() },
            format!("ops[{i}].id"),
        ));
    }
    Ok(())
}

pub fn comment_missing(id: &str, i: usize) -> Error {
    Error::with_path(
        ErrorKind::CommentNotFound { id: id.to_string() },
        format!("ops[{i}].id"),
    )
}

pub fn need_para_id(i: usize) -> Error {
    Error::with_path(ErrorKind::NeedCommentParaId, format!("ops[{i}].id"))
}

/// 同一 OPC 部件且文档顺序上 start 不晚于 end。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoryKey {
    pub part: String,
    pub story: String,
}

pub fn story_key(doc_part: &str, target: &ParaTarget) -> Result<StoryKey, Error> {
    match target {
        ParaTarget::Body(_) => Ok(StoryKey {
            part: doc_part.to_string(),
            story: "body".into(),
        }),
        ParaTarget::HeaderFooter {
            kind, hf_type, n, ..
        } => Ok(StoryKey {
            part: format!("{}:{}:{n}", kind.as_str(), hf_type.as_str()),
            story: format!("hf:{}:{}:{n}", kind.as_str(), hf_type.as_str()),
        }),
        ParaTarget::InTextBox { host, key, .. } => {
            let host_key = match host {
                para::HostPara::Body(_) => "body".to_string(),
                para::HostPara::HeaderFooter {
                    kind, hf_type, n, ..
                } => format!("hf:{}:{}:{n}", kind.as_str(), hf_type.as_str()),
                para::HostPara::InNote { kind, note_id, .. } => {
                    format!("{}:{note_id}", kind.as_str())
                }
                para::HostPara::InComment { comment_id, .. } => format!("comment:{comment_id}"),
            };
            let box_key = match key {
                para::TxbxKey::DocPr(id) => format!("docpr:{id}"),
                para::TxbxKey::Index(i) => format!("index:{i}"),
            };
            Ok(StoryKey {
                part: host_key.clone(),
                story: format!("{host_key}/txbx:{box_key}"),
            })
        }
        ParaTarget::InNote { kind, note_id, .. } => Ok(StoryKey {
            part: kind.as_str().to_string(),
            story: format!("{}:{note_id}", kind.as_str()),
        }),
        ParaTarget::InComment { .. } => Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: crate::error::IdReason::CommentPrefix,
            },
            "ops.start",
        )),
    }
}

pub fn para_index_in_parent(parent: &Element, para: &BodyPara, in_txbx: bool) -> Option<usize> {
    let paras = if in_txbx {
        para::collect_paragraphs_in_txbx(parent)
    } else {
        para::collect_paragraphs(parent)
    };
    for (i, p) in paras.iter().enumerate() {
        let hit = match para {
            BodyPara::Index(n) => i + 1 == *n,
            BodyPara::Hex(hex) => p
                .attr_local("paraId")
                .is_some_and(|id| id.eq_ignore_ascii_case(hex)),
        };
        if hit {
            return Some(i);
        }
    }
    None
}

pub fn insert_range_start(p: &mut Element, w_id: &str) {
    let mut mark = Element::new(p.prefixed("commentRangeStart"));
    mark.set_attr(&p.prefixed("id"), w_id);
    let mut at = 0usize;
    for (i, n) in p.children.iter().enumerate() {
        if let Node::Element(e) = n {
            if e.local_name() == "pPr" {
                at = i + 1;
            }
        }
    }
    p.children.insert(at, Node::Element(mark));
}

pub fn insert_range_end(p: &mut Element, w_id: &str) {
    let mut mark = Element::new(p.prefixed("commentRangeEnd"));
    mark.set_attr(&p.prefixed("id"), w_id);
    p.push_element(mark);
    let mut rpr = Element::new(p.prefixed("rPr"));
    let mut style = Element::new(p.prefixed("rStyle"));
    style.set_attr(&p.prefixed("val"), "CommentReference");
    rpr.push_element(style);
    let mut rf = Element::new(p.prefixed("commentReference"));
    rf.set_attr(&p.prefixed("id"), w_id);
    let mut r = Element::new(p.prefixed("r"));
    r.push_element(rpr);
    r.push_element(rf);
    p.push_element(r);
}

fn run_has_comment_ref(r: &Element, comment_id: &str) -> bool {
    r.element_children().any(|e| {
        e.local_name() == "commentReference"
            && ids_match(e.attr_local("id").unwrap_or(""), comment_id)
    })
}

fn comment_ref_style(r: &Element) -> String {
    if let Some(rpr) = r.child_by_local("rPr") {
        if let Some(rs) = rpr.child_by_local("rStyle") {
            if let Some(v) = rs.attr_local("val") {
                return v.to_string();
            }
        }
    }
    "CommentReference".into()
}

fn insert_start_after_parent(el: &mut Element, parent_id: &str, reply_id: &str) -> bool {
    if el.local_name() == "AlternateContent" {
        if let Some(i) = first_alt_child_index(el) {
            if let Node::Element(child) = &mut el.children[i] {
                return insert_start_after_parent(child, parent_id, reply_id);
            }
        }
        return false;
    }
    let mut changed = false;
    let mut i = 0usize;
    while i < el.children.len() {
        let hit = if let Node::Element(e) = &el.children[i] {
            e.local_name() == "commentRangeStart"
                && ids_match(e.attr_local("id").unwrap_or(""), parent_id)
        } else {
            false
        };
        if hit {
            let prefix = if let Node::Element(e) = &el.children[i] {
                e.prefixed("commentRangeStart")
            } else {
                "w:commentRangeStart".into()
            };
            let id_attr = if let Node::Element(e) = &el.children[i] {
                e.prefixed("id")
            } else {
                "w:id".into()
            };
            let mut mark = Element::new(prefix);
            mark.set_attr(&id_attr, reply_id);
            el.children.insert(i + 1, Node::Element(mark));
            changed = true;
            i += 2;
            continue;
        }
        if let Node::Element(e) = &mut el.children[i] {
            changed |= insert_start_after_parent(e, parent_id, reply_id);
        }
        i += 1;
    }
    changed
}

fn insert_end_after_parent_ref(el: &mut Element, parent_id: &str, reply_id: &str) -> bool {
    if el.local_name() == "AlternateContent" {
        if let Some(i) = first_alt_child_index(el) {
            if let Node::Element(child) = &mut el.children[i] {
                return insert_end_after_parent_ref(child, parent_id, reply_id);
            }
        }
        return false;
    }
    let mut changed = false;
    let mut i = 0usize;
    while i < el.children.len() {
        let hit = if let Node::Element(e) = &el.children[i] {
            e.local_name() == "r" && run_has_comment_ref(e, parent_id)
        } else {
            false
        };
        if hit {
            let (end_name, id_attr, r_name, rpr_name, rstyle_name, val_attr, ref_name, style_val) =
                if let Node::Element(e) = &el.children[i] {
                    (
                        e.prefixed("commentRangeEnd"),
                        e.prefixed("id"),
                        e.prefixed("r"),
                        e.prefixed("rPr"),
                        e.prefixed("rStyle"),
                        e.prefixed("val"),
                        e.prefixed("commentReference"),
                        comment_ref_style(e),
                    )
                } else {
                    continue;
                };
            let mut mark = Element::new(end_name);
            mark.set_attr(&id_attr, reply_id);
            let mut rpr = Element::new(rpr_name);
            let mut style = Element::new(rstyle_name);
            style.set_attr(&val_attr, &style_val);
            rpr.push_element(style);
            let mut rf = Element::new(ref_name);
            rf.set_attr(&id_attr, reply_id);
            let mut r = Element::new(r_name);
            r.push_element(rpr);
            r.push_element(rf);
            el.children.insert(i + 1, Node::Element(mark));
            el.children.insert(i + 2, Node::Element(r));
            changed = true;
            i += 3;
            continue;
        }
        if let Node::Element(e) = &mut el.children[i] {
            changed |= insert_end_after_parent_ref(e, parent_id, reply_id);
        }
        i += 1;
    }
    changed
}

pub fn insert_reply_markers(root: &mut Element, parent_id: &str, reply_id: &str) -> bool {
    let a = insert_start_after_parent(root, parent_id, reply_id);
    let b = insert_end_after_parent_ref(root, parent_id, reply_id);
    a || b
}

pub fn insert_reply_markers_in_hosts(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[HfPart],
    parent_id: &str,
    reply_id: &str,
) -> Result<(), Error> {
    {
        let doc = load_into(pkg, docs, doc_part)?;
        if insert_reply_markers(&mut doc.root, parent_id, reply_id) {
            dirty.insert(doc_part.to_string(), ());
        }
    }
    for item in catalog {
        let doc = load_into(pkg, docs, &item.part)?;
        if insert_reply_markers(&mut doc.root, parent_id, reply_id) {
            dirty.insert(item.part.clone(), ());
        }
    }
    for kind in [NoteKind::Footnote, NoteKind::Endnote] {
        if let Some(part) = notes::part_name(pkg, doc_part, kind)? {
            let doc = load_into(pkg, docs, &part)?;
            if insert_reply_markers(&mut doc.root, parent_id, reply_id) {
                dirty.insert(part, ());
            }
        }
    }
    Ok(())
}

pub fn strip_markers(root: &mut Element, ids: &HashSet<String>) {
    strip_marks(root, ids);
}

fn strip_marks(el: &mut Element, ids: &HashSet<String>) {
    if el.local_name() == "AlternateContent" {
        if let Some(i) = first_alt_child_index(el) {
            if let Node::Element(child) = &mut el.children[i] {
                strip_marks(child, ids);
            }
        }
        return;
    }
    let mut drop = Vec::new();
    for (i, n) in el.children.iter_mut().enumerate() {
        if let Node::Element(e) = n {
            let local = e.local_name();
            if matches!(
                local,
                "commentRangeStart" | "commentRangeEnd" | "commentReference"
            ) && ids
                .iter()
                .any(|id| ids_match(id, e.attr_local("id").unwrap_or("")))
            {
                drop.push(i);
                continue;
            }
            strip_marks(e, ids);
            if e.local_name() == "r" && run_only_rpr(e) {
                drop.push(i);
            }
        }
    }
    drop.sort_unstable();
    drop.dedup();
    for i in drop.into_iter().rev() {
        el.children.remove(i);
    }
}

fn run_only_rpr(r: &Element) -> bool {
    !r.element_children().any(|e| e.local_name() != "rPr")
}

pub fn append_comment(
    root: &mut Element,
    w_id: &str,
    para_id: &str,
    text: &str,
    author: &str,
    initials: &str,
    date: Option<&str>,
    p_style: &str,
    r_style: &str,
) {
    ensure_xmlns(root, "w14", W14_NS);
    ensure_mc_ignorable(root, "w14");
    root.push_element(new_comment(
        root, w_id, para_id, text, author, initials, date, p_style, r_style,
    ));
}

/// 已有批注用的段落/标记样式。Word 中文文件常见 `a8` / `a7`，不是 `CommentText`。
pub fn styles_of(comment: &Element) -> (String, String) {
    let mut pstyle = "CommentText".to_string();
    let mut rstyle = "CommentReference".to_string();
    for n in &comment.children {
        let Node::Element(p) = n else {
            continue;
        };
        if p.local_name() != "p" {
            continue;
        }
        if let Some(ppr) = p.child_by_local("pPr") {
            if let Some(ps) = ppr.child_by_local("pStyle") {
                if let Some(v) = ps.attr_local("val") {
                    pstyle = v.to_string();
                }
            }
        }
        for rn in &p.children {
            let Node::Element(r) = rn else {
                continue;
            };
            if r.local_name() != "r" {
                continue;
            }
            if !r
                .element_children()
                .any(|c| c.local_name() == "annotationRef")
            {
                continue;
            }
            if let Some(rpr) = r.child_by_local("rPr") {
                if let Some(rs) = rpr.child_by_local("rStyle") {
                    if let Some(v) = rs.attr_local("val") {
                        rstyle = v.to_string();
                    }
                }
            }
            break;
        }
        break;
    }
    (pstyle, rstyle)
}

pub fn first_comment_styles(root: &Element) -> (String, String) {
    match comment_elements(root).into_iter().next() {
        Some(c) => styles_of(c),
        None => ("CommentText".into(), "CommentReference".into()),
    }
}

fn new_comment(
    parent: &Element,
    w_id: &str,
    para_id: &str,
    text: &str,
    author: &str,
    initials: &str,
    date: Option<&str>,
    p_style: &str,
    r_style: &str,
) -> Element {
    let mut c = Element::new(parent.prefixed("comment"));
    c.set_attr(&parent.prefixed("id"), w_id);
    c.set_attr(&parent.prefixed("author"), author);
    c.set_attr(&parent.prefixed("initials"), initials);
    if let Some(d) = date {
        c.set_attr(&parent.prefixed("date"), d);
    }
    let mut p = Element::new(parent.prefixed("p"));
    p.set_attr("w14:paraId", para_id);
    p.set_attr("w14:textId", para_id);
    let mut ppr = Element::new(parent.prefixed("pPr"));
    let mut pstyle = Element::new(parent.prefixed("pStyle"));
    pstyle.set_attr(&parent.prefixed("val"), p_style);
    ppr.push_element(pstyle);
    p.push_element(ppr);
    let mut rpr = Element::new(parent.prefixed("rPr"));
    let mut rstyle = Element::new(parent.prefixed("rStyle"));
    rstyle.set_attr(&parent.prefixed("val"), r_style);
    rpr.push_element(rstyle);
    let mut mark = Element::new(parent.prefixed("r"));
    mark.push_element(rpr);
    mark.push_element(Element::new(parent.prefixed("annotationRef")));
    p.push_element(mark);
    let mut t = Element::new(parent.prefixed("t"));
    if needs_xml_space(text) {
        t.set_attr("xml:space", "preserve");
    }
    t.set_text(text);
    let mut r = Element::new(parent.prefixed("r"));
    r.push_element(t);
    p.push_element(r);
    c.push_element(p);
    c
}

pub fn remove_comment(root: &mut Element, comment_id: &str) -> bool {
    let mut idx = None;
    for (i, n) in root.children.iter().enumerate() {
        if let Node::Element(e) = n {
            if e.local_name() == "comment"
                && ids_match(e.attr_local("id").unwrap_or(""), comment_id)
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

/// 被删批注及其子回复的 w:id。
pub fn cascade_ids(
    comments_root: &Element,
    extended_root: Option<&Element>,
    root_id: &str,
) -> Vec<String> {
    let last = last_para_map(comments_root);
    let parent_of = parent_map(comments_root, extended_root, &last);
    let mut kids: HashMap<String, Vec<String>> = HashMap::new();
    for (child, parent) in &parent_of {
        kids.entry(parent.clone()).or_default().push(child.clone());
    }
    let mut out = Vec::new();
    let mut stack = vec![root_id.to_string()];
    let mut seen = HashSet::new();
    while let Some(id) = stack.pop() {
        if !seen.insert(id.clone()) {
            continue;
        }
        out.push(id.clone());
        if let Some(ch) = kids.get(&id) {
            stack.extend(ch.iter().cloned());
        }
    }
    out
}

pub fn set_visible_keep_first(comment: &mut Element, text: &str) {
    let mut first = true;
    let mut drop_p = Vec::new();
    for (i, n) in comment.children.iter_mut().enumerate() {
        if let Node::Element(e) = n {
            if e.local_name() == "p" {
                if first {
                    para::set_visible_text(e, text);
                    first = false;
                } else {
                    drop_p.push(i);
                }
            }
        }
    }
    for i in drop_p.into_iter().rev() {
        comment.children.remove(i);
    }
}

pub fn ensure_comments_part(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
) -> Result<String, Error> {
    if let Some(existing) = part_name(pkg, doc_part)? {
        return Ok(existing);
    }
    let part = opc::resolve_rel_target(doc_part, "comments.xml")?;
    pkg.set(&part, comments_xml().into_bytes())?;
    let parsed = xml::parse(pkg.get(&part)?)?;
    docs.insert(part.clone(), parsed);
    dirty.insert(part.clone(), ());
    add_rel(pkg, doc_part, super::REL_COMMENTS, "comments.xml")?;
    set_override(pkg, &part, super::CT_COMMENTS)?;
    Ok(part)
}

pub fn ensure_extended_part(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
) -> Result<String, Error> {
    if let Some(existing) = extended_part_name(pkg, doc_part)? {
        return Ok(existing);
    }
    let part = opc::resolve_rel_target(doc_part, "commentsExtended.xml")?;
    pkg.set(&part, comments_ex_xml().into_bytes())?;
    let parsed = xml::parse(pkg.get(&part)?)?;
    docs.insert(part.clone(), parsed);
    dirty.insert(part.clone(), ());
    add_rel(
        pkg,
        doc_part,
        super::REL_COMMENTS_EXTENDED,
        "commentsExtended.xml",
    )?;
    set_override(pkg, &part, super::CT_COMMENTS_EXTENDED)?;
    Ok(part)
}

pub fn ensure_people_part(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
) -> Result<String, Error> {
    if let Some(existing) = people_part_name(pkg, doc_part)? {
        return Ok(existing);
    }
    let part = opc::resolve_rel_target(doc_part, "people.xml")?;
    pkg.set(&part, people_xml().into_bytes())?;
    let parsed = xml::parse(pkg.get(&part)?)?;
    docs.insert(part.clone(), parsed);
    dirty.insert(part.clone(), ());
    add_rel(pkg, doc_part, super::REL_PEOPLE, "people.xml")?;
    set_override(pkg, &part, super::CT_PEOPLE)?;
    Ok(part)
}

fn add_rel(pkg: &mut Package, doc_part: &str, rel_type: &str, target: &str) -> Result<(), Error> {
    let mut rels = Relationships::from_package(pkg, doc_part)?;
    let rid = rels.next_rid();
    rels.push(Relationship {
        id: rid,
        rel_type: rel_type.into(),
        target: target.into(),
        target_mode: None,
        extra: Vec::new(),
    });
    rels.write_to(pkg, doc_part)
}

fn set_override(pkg: &mut Package, part: &str, ct: &str) -> Result<(), Error> {
    let mut cts = ContentTypes::from_package(pkg)?;
    cts.set_override(part, ct)?;
    cts.write_to(pkg)
}

pub fn ensure_person(root: &mut Element, author: &str) {
    for e in root.element_children() {
        if e.local_name() == "person" && e.attr_local("author") == Some(author) {
            return;
        }
    }
    let mut person = Element::new(root.prefixed("person"));
    person.set_attr(&root.prefixed("author"), author);
    let mut info = Element::new(root.prefixed("presenceInfo"));
    info.set_attr(&root.prefixed("providerId"), "None");
    info.set_attr(&root.prefixed("userId"), author);
    person.push_element(info);
    root.push_element(person);
}

pub fn upsert_comment_ex(
    root: &mut Element,
    para_id: &str,
    para_id_parent: Option<&str>,
    done: Option<bool>,
) {
    if let Some(existing) = root.element_children_mut().find(|e| {
        e.local_name() == "commentEx"
            && para_ids_match(e.attr_local("paraId").unwrap_or(""), para_id)
    }) {
        if let Some(p) = para_id_parent {
            set_local_attr(existing, "paraIdParent", p);
        }
        if let Some(d) = done {
            set_local_attr(existing, "done", if d { "1" } else { "0" });
        }
        return;
    }
    let mut ex = Element::new(root.prefixed("commentEx"));
    ex.set_attr(&root.prefixed("paraId"), para_id);
    if let Some(p) = para_id_parent {
        ex.set_attr(&root.prefixed("paraIdParent"), p);
    }
    ex.set_attr(
        &root.prefixed("done"),
        if done.unwrap_or(false) { "1" } else { "0" },
    );
    root.push_element(ex);
}

pub fn retarget_para_id(root: &mut Element, old: &str, new: &str) {
    if para_ids_match(old, new) {
        return;
    }
    for e in root.element_children_mut() {
        if e.local_name() != "commentEx" {
            continue;
        }
        if para_ids_match(e.attr_local("paraId").unwrap_or(""), old) {
            set_local_attr(e, "paraId", new);
        }
        if para_ids_match(e.attr_local("paraIdParent").unwrap_or(""), old) {
            set_local_attr(e, "paraIdParent", new);
        }
    }
}

pub fn remove_comment_ex_for_para(root: &mut Element, para_id: &str) {
    let mut drop = Vec::new();
    for (i, n) in root.children.iter().enumerate() {
        if let Node::Element(e) = n {
            if e.local_name() == "commentEx"
                && para_ids_match(e.attr_local("paraId").unwrap_or(""), para_id)
            {
                drop.push(i);
            }
        }
    }
    for i in drop.into_iter().rev() {
        root.children.remove(i);
    }
}

pub fn fill_missing_comment_ex(comments_root: &Element, ex_root: &mut Element) -> Result<(), ()> {
    let existing: HashSet<String> = ex_root
        .element_children()
        .filter(|e| e.local_name() == "commentEx")
        .filter_map(|e| e.attr_local("paraId").map(str::to_string))
        .collect();
    for c in comment_elements(comments_root) {
        let Some(pid) = last_para_id(c) else {
            return Err(());
        };
        if existing.iter().any(|x| para_ids_match(x, &pid)) {
            continue;
        }
        upsert_comment_ex(ex_root, &pid, None, Some(false));
    }
    Ok(())
}

pub fn set_done_on_para(root: &mut Element, para_id: &str, done: bool) -> bool {
    for e in root.element_children_mut() {
        if e.local_name() == "commentEx"
            && para_ids_match(e.attr_local("paraId").unwrap_or(""), para_id)
        {
            set_local_attr(e, "done", if done { "1" } else { "0" });
            return true;
        }
    }
    false
}

pub fn patch_ids_parts(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    para_id: &str,
) -> Result<(), Error> {
    if let Some(part) = ids_part_name(pkg, doc_part)? {
        let doc = load_into(pkg, docs, &part)?;
        let durable = next_durable_id(&doc.root);
        append_comment_id(&mut doc.root, para_id, &durable);
        dirty.insert(part.clone(), ());
        if let Some(ext) = extensible_part_name(pkg, doc_part)? {
            let edoc = load_into(pkg, docs, &ext)?;
            append_comment_extensible(&mut edoc.root, &durable);
            dirty.insert(ext, ());
        }
    } else if let Some(ext) = extensible_part_name(pkg, doc_part)? {
        let edoc = load_into(pkg, docs, &ext)?;
        let durable = next_durable_id(&edoc.root);
        append_comment_extensible(&mut edoc.root, &durable);
        dirty.insert(ext, ());
    }
    Ok(())
}

pub fn retarget_ids_parts(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    old: &str,
    new: &str,
) -> Result<(), Error> {
    if para_ids_match(old, new) {
        return Ok(());
    }
    if let Some(part) = ids_part_name(pkg, doc_part)? {
        let doc = load_into(pkg, docs, &part)?;
        for e in doc.root.element_children_mut() {
            if e.local_name() == "commentId"
                && para_ids_match(e.attr_local("paraId").unwrap_or(""), old)
            {
                set_local_attr(e, "paraId", new);
            }
        }
        dirty.insert(part, ());
    }
    Ok(())
}

pub fn remove_ids_for_para(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    para_id: &str,
) -> Result<(), Error> {
    let mut durable: Option<String> = None;
    if let Some(part) = ids_part_name(pkg, doc_part)? {
        let doc = load_into(pkg, docs, &part)?;
        let mut drop = Vec::new();
        for (i, n) in doc.root.children.iter().enumerate() {
            if let Node::Element(e) = n {
                if e.local_name() == "commentId"
                    && para_ids_match(e.attr_local("paraId").unwrap_or(""), para_id)
                {
                    durable = e.attr_local("durableId").map(str::to_string);
                    drop.push(i);
                }
            }
        }
        for i in drop.into_iter().rev() {
            doc.root.children.remove(i);
        }
        dirty.insert(part, ());
    }
    if let (Some(ext), Some(dur)) = (extensible_part_name(pkg, doc_part)?, durable) {
        let doc = load_into(pkg, docs, &ext)?;
        let mut drop = Vec::new();
        for (i, n) in doc.root.children.iter().enumerate() {
            if let Node::Element(e) = n {
                if e.local_name() == "commentExtensible"
                    && e.attr_local("durableId") == Some(dur.as_str())
                {
                    drop.push(i);
                }
            }
        }
        for i in drop.into_iter().rev() {
            doc.root.children.remove(i);
        }
        dirty.insert(ext, ());
    }
    Ok(())
}

fn append_comment_id(root: &mut Element, para_id: &str, durable: &str) {
    let mut el = Element::new(root.prefixed("commentId"));
    el.set_attr(&root.prefixed("paraId"), para_id);
    el.set_attr(&root.prefixed("durableId"), durable);
    root.push_element(el);
}

fn append_comment_extensible(root: &mut Element, durable: &str) {
    let mut el = Element::new(root.prefixed("commentExtensible"));
    el.set_attr(&root.prefixed("durableId"), durable);
    // Word 16 手写的条目都带 dateUtc，缺了审阅窗格可能不收这条。
    el.set_attr(&root.prefixed("dateUtc"), "2026-01-01T00:00:00Z");
    root.push_element(el);
}

fn next_durable_id(root: &Element) -> String {
    let mut used = HashSet::new();
    let mut max = 0u32;
    for e in root.element_children() {
        if let Some(d) = e.attr_local("durableId") {
            used.insert(d.to_lowercase());
            if let Ok(n) = u32::from_str_radix(d, 16) {
                max = max.max(n);
            }
        }
    }
    let mut n = max.saturating_add(1).max(1);
    loop {
        let s = format!("{n:08X}");
        if !used.contains(&s.to_lowercase()) {
            return s;
        }
        n = n.wrapping_add(1);
        if n == 0 {
            return s;
        }
    }
}

pub fn load_into<'a>(
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

pub fn collect_blip_embeds(el: &Element) -> Vec<String> {
    notes::collect_blip_embeds(el)
}

fn comments_xml() -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:comments xmlns:w="{WML_NS}" xmlns:r="{REL_NS}" xmlns:w14="{W14_NS}" xmlns:mc="{MC_NS}" mc:Ignorable="w14"/>"#
    )
}

fn comments_ex_xml() -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w15:commentsEx xmlns:w15="{W15_NS}" xmlns:mc="{MC_NS}" mc:Ignorable="w15"/>"#
    )
}

fn people_xml() -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w15:people xmlns:w15="{W15_NS}"/>"#
    )
}

fn needs_xml_space(text: &str) -> bool {
    text.starts_with(' ')
        || text.starts_with('\t')
        || text.ends_with(' ')
        || text.ends_with('\t')
        || text.contains('\n')
}

fn ensure_xmlns(el: &mut Element, prefix: &str, uri: &str) {
    let attr = format!("xmlns:{prefix}");
    if el.attr(&attr).is_none() {
        el.set_attr(&attr, uri);
    }
}

fn ensure_mc_ignorable(el: &mut Element, token: &str) {
    ensure_xmlns(el, "mc", MC_NS);
    let cur = el.attr_local("Ignorable").unwrap_or("").to_string();
    if cur.split_whitespace().any(|t| t == token) {
        return;
    }
    let next = if cur.is_empty() {
        token.to_string()
    } else {
        format!("{cur} {token}")
    };
    set_local_attr(el, "Ignorable", &next);
}

fn set_local_attr(el: &mut Element, local: &str, val: &str) {
    if let Some((_, v)) = el
        .attrs
        .iter_mut()
        .find(|(k, _)| k.rsplit(':').next() == Some(local))
    {
        *v = val.to_string();
        return;
    }
    el.set_attr(&el.prefixed(local), val);
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

pub fn strip_markers_in_hosts(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[HfPart],
    ids: &HashSet<String>,
) -> Result<(), Error> {
    {
        let doc = load_into(pkg, docs, doc_part)?;
        strip_markers(&mut doc.root, ids);
        dirty.insert(doc_part.to_string(), ());
    }
    for item in catalog {
        let doc = load_into(pkg, docs, &item.part)?;
        strip_markers(&mut doc.root, ids);
        dirty.insert(item.part.clone(), ());
    }
    for kind in [NoteKind::Footnote, NoteKind::Endnote] {
        if let Some(part) = notes::part_name(pkg, doc_part, kind)? {
            let doc = load_into(pkg, docs, &part)?;
            strip_markers(&mut doc.root, ids);
            dirty.insert(part, ());
        }
    }
    Ok(())
}
