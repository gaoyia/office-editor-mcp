//! 正文、页眉页脚与文本框里的 `w:hyperlink` 与配对书签：inspect 名单与插改删。

use std::collections::{HashMap, HashSet};

use serde_json::{json, Value};

use crate::error::{Error, ErrorKind, IdReason, XmlIssue};
use crate::opc::{Relationship, Relationships};
use crate::xml::{Element, Node};

use super::hf::{self, HfKind, HfPart, HfType};
use super::para::{self, HostPara, TxbxKey};

#[derive(Debug, Clone)]
pub enum LinkStory {
    Body,
    HeaderFooter {
        kind: HfKind,
        hf_type: HfType,
        n: u32,
    },
    TextBox {
        host: HostPara,
        key: TxbxKey,
    },
}

#[derive(Debug, Clone)]
pub struct LinkRef {
    pub story: LinkStory,
    pub n: usize,
}

#[derive(Debug, Clone)]
pub struct BookmarkRef {
    pub story: LinkStory,
    pub n: usize,
}

struct BookmarkPair {
    start: Vec<usize>,
    end: Vec<usize>,
    name: String,
    xml_id: Option<String>,
}

/// 含隐藏书签在内的配对书签，供题注 / 交叉引用使用。
#[derive(Debug, Clone)]
pub(crate) struct BookmarkSpan {
    pub start: Vec<usize>,
    pub end: Vec<usize>,
    pub name: String,
}

pub fn count_hyperlinks(root: &Element) -> usize {
    collect_hyperlink_paths(root).len()
}

pub fn count_bookmarks(root: &Element) -> usize {
    collect_bookmark_pairs(root).len()
}

pub fn hyperlinks_json(
    root: &Element,
    container: Option<&str>,
    rels: &Relationships,
) -> Result<Vec<Value>, Error> {
    let index_of = para_index_map(root);
    let paths = collect_hyperlink_paths(root);
    let mut out = Vec::new();
    for (i, path) in paths.iter().enumerate() {
        let el = para::element_at(root, path).ok_or_else(|| xml_broken("hyperlink"))?;
        let n = i + 1;
        let id = match container {
            Some(c) => format!("{c}/link:{n}"),
            None => format!("link:{n}"),
        };
        let rid = r_id(el);
        let kind = if rid.is_some() { "url" } else { "anchor" };
        let url = rid.and_then(|r| rels.find_by_id(r).map(|rel| rel.target.clone()));
        let anchor = el.attr_local("anchor").map(str::to_string);
        let tooltip = el.attr_local("tooltip").map(str::to_string);
        let host = host_para_id(root, path, container, &index_of);
        out.push(json!({
            "id": id,
            "index": n,
            "kind": kind,
            "rId": rid,
            "url": url,
            "anchor": anchor,
            "tooltip": tooltip,
            "host": host,
            "text": para::visible_text(el),
        }));
    }
    Ok(out)
}

pub fn bookmarks_json(root: &Element, container: Option<&str>) -> Result<Vec<Value>, Error> {
    let index_of = para_index_map(root);
    let pairs = collect_bookmark_pairs(root);
    let mut out = Vec::new();
    for (i, pair) in pairs.iter().enumerate() {
        let n = i + 1;
        let id = match container {
            Some(c) => format!("{c}/bookmark:{n}"),
            None => format!("bookmark:{n}"),
        };
        let start = host_para_id(root, &pair.start, container, &index_of);
        let end = host_para_id(root, &pair.end, container, &index_of);
        out.push(json!({
            "id": id,
            "index": n,
            "name": pair.name,
            "xmlId": pair.xml_id,
            "start": start,
            "end": end,
        }));
    }
    Ok(out)
}

pub fn parse_link_ref(id: &str, path: &str) -> Result<LinkRef, Error> {
    let (story, rest) = split_story(id, path, "link", IdReason::LinkPrefix)?;
    if rest.contains('/') {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::LinkPrefix,
            },
            path.to_string(),
        ));
    }
    let n = parse_n(
        rest,
        path,
        IdReason::LinkIndexNotInt,
        IdReason::LinkIndexFromOne,
    )?;
    Ok(LinkRef { story, n })
}

pub fn parse_bookmark_ref(id: &str, path: &str) -> Result<BookmarkRef, Error> {
    let (story, rest) = split_story(id, path, "bookmark", IdReason::BookmarkPrefix)?;
    if rest.contains('/') {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::BookmarkPrefix,
            },
            path.to_string(),
        ));
    }
    let n = parse_n(
        rest,
        path,
        IdReason::BookmarkIndexNotInt,
        IdReason::BookmarkIndexFromOne,
    )?;
    Ok(BookmarkRef { story, n })
}

pub fn part_for_story(
    catalog: &[HfPart],
    doc_part: &str,
    story: &LinkStory,
    missing: impl FnOnce() -> Error,
) -> Result<String, Error> {
    match story {
        LinkStory::Body => Ok(doc_part.to_string()),
        LinkStory::HeaderFooter { kind, hf_type, n } => {
            let cid = format!("{}:{}:{n}", kind.as_str(), hf_type.as_str());
            let item = hf::find_by_id(catalog, &cid).ok_or_else(missing)?;
            Ok(item.part.clone())
        }
        LinkStory::TextBox { host, .. } => match host {
            HostPara::Body(_) => Ok(doc_part.to_string()),
            HostPara::HeaderFooter {
                kind, hf_type, n, ..
            } => part_for_story(
                catalog,
                doc_part,
                &LinkStory::HeaderFooter {
                    kind: *kind,
                    hf_type: *hf_type,
                    n: *n,
                },
                missing,
            ),
            HostPara::InNote { .. } | HostPara::InComment { .. } => Err(missing()),
        },
    }
}

/// 段内是否有本期收入的 `w:hyperlink`。框内段落请不要调用。
pub fn para_blocks_set_text(p: &Element) -> bool {
    !collect_hyperlink_paths(p).is_empty()
}

pub fn r_id(el: &Element) -> Option<&str> {
    el.attr("r:id")
}

pub fn rid_used(root: &Element, rid: &str) -> bool {
    rid_used_walk(root, rid)
}

pub fn insert_hyperlink_el(
    p: &mut Element,
    text: &str,
    rid: Option<&str>,
    anchor: Option<&str>,
    tooltip: Option<&str>,
) {
    let mut hl = Element::new(p.prefixed("hyperlink"));
    if let Some(rid) = rid {
        hl.set_attr("r:id", rid);
    }
    if let Some(anchor) = anchor {
        let key = hl.prefixed("anchor");
        hl.set_attr(&key, anchor);
    }
    if let Some(tooltip) = tooltip {
        let key = hl.prefixed("tooltip");
        hl.set_attr(&key, tooltip);
    }
    hl.push_element(para::new_text_run(&hl, text));
    p.push_element(hl);
}

pub fn set_hyperlink_url(el: &mut Element, rid: &str) {
    el.set_attr("r:id", rid);
    remove_local_attr(el, "anchor");
}

pub fn set_hyperlink_anchor(el: &mut Element, anchor: &str) {
    let key = el.prefixed("anchor");
    el.set_attr(&key, anchor);
    el.remove_attr("r:id");
}

pub fn set_hyperlink_tooltip(el: &mut Element, tooltip: Option<&str>) {
    match tooltip {
        Some(t) => {
            let key = el.prefixed("tooltip");
            el.set_attr(&key, t);
        }
        None => remove_local_attr(el, "tooltip"),
    }
}

pub fn unwrap_hyperlink(root: &mut Element, n: usize, id: &str, i: usize) -> Result<(), Error> {
    let paths = collect_hyperlink_paths(root);
    if n < 1 || n > paths.len() {
        return Err(missing_link(id, i));
    }
    let path = paths[n - 1].clone();
    unwrap_at(root, &path)
}

pub fn hyperlink_at_mut<'a>(
    root: &'a mut Element,
    n: usize,
    id: &str,
    i: usize,
) -> Result<&'a mut Element, Error> {
    let paths = collect_hyperlink_paths(root);
    if n < 1 || n > paths.len() {
        return Err(missing_link(id, i));
    }
    let path = paths[n - 1].clone();
    para::element_at_mut(root, &path).ok_or_else(|| missing_link(id, i))
}

pub fn insert_bookmark(
    root: &mut Element,
    start_path: &[usize],
    end_path: &[usize],
    name: Option<&str>,
    id: &str,
    i: usize,
) -> Result<(), Error> {
    if end_path < start_path {
        return Err(Error::with_path(
            ErrorKind::BookmarkRangeReversed,
            format!("ops[{i}].end"),
        ));
    }
    let taken = all_bookmark_names(root);
    let name = match name {
        Some(n) => {
            check_bookmark_name(n, i, "name")?;
            if taken.contains(n) {
                return Err(Error::with_path(
                    ErrorKind::BookmarkNameTaken,
                    format!("ops[{i}].name"),
                ));
            }
            n.to_string()
        }
        None => unused_ooxp_name(&taken),
    };
    let xml_id = next_bookmark_xml_id(root);
    if start_path == end_path {
        let p = para::element_at_mut(root, start_path).ok_or_else(|| missing_para(id, i))?;
        insert_bookmark_start(p, &xml_id, &name);
        insert_bookmark_end(p, &xml_id);
    } else {
        {
            let p = para::element_at_mut(root, start_path).ok_or_else(|| missing_para(id, i))?;
            insert_bookmark_start(p, &xml_id, &name);
        }
        {
            let p = para::element_at_mut(root, end_path).ok_or_else(|| missing_para(id, i))?;
            insert_bookmark_end(p, &xml_id);
        }
    }
    Ok(())
}

pub fn set_bookmark_name(
    root: &mut Element,
    n: usize,
    name: &str,
    taken: &HashSet<String>,
    id: &str,
    i: usize,
) -> Result<Option<(String, String)>, Error> {
    check_bookmark_name(name, i, "name")?;
    let pairs = collect_bookmark_pairs(root);
    if n < 1 || n > pairs.len() {
        return Err(missing_bookmark(id, i));
    }
    let pair = &pairs[n - 1];
    if pair.name == name {
        return Ok(None);
    }
    if taken.contains(name) {
        return Err(Error::with_path(
            ErrorKind::BookmarkNameTaken,
            format!("ops[{i}].name"),
        ));
    }
    let old = pair.name.clone();
    let start_path = pair.start.clone();
    {
        let start =
            para::element_at_mut(root, &start_path).ok_or_else(|| missing_bookmark(id, i))?;
        let key = start.prefixed("name");
        start.set_attr(&key, name);
    }
    Ok(Some((old, name.to_string())))
}

pub fn delete_bookmark(root: &mut Element, n: usize, id: &str, i: usize) -> Result<(), Error> {
    let pairs = collect_bookmark_pairs(root);
    if n < 1 || n > pairs.len() {
        return Err(missing_bookmark(id, i));
    }
    let mut paths = vec![pairs[n - 1].start.clone(), pairs[n - 1].end.clone()];
    paths.sort_by(|a, b| b.cmp(a));
    for path in paths {
        remove_at(root, &path)?;
    }
    Ok(())
}

pub fn check_bookmark_name(name: &str, i: usize, field: &str) -> Result<(), Error> {
    if name.is_empty() || name.starts_with('_') || name.chars().any(char::is_whitespace) {
        return Err(Error::with_path(
            ErrorKind::InvalidBookmarkName,
            format!("ops[{i}].{field}"),
        ));
    }
    Ok(())
}

fn collect_hyperlink_paths(root: &Element) -> Vec<Vec<usize>> {
    let mut path = Vec::new();
    let mut out = Vec::new();
    para::walk_from_story_root(root, &mut path, |el, path| {
        walk_hyperlinks(el, path, &mut out);
    });
    out
}

fn walk_hyperlinks(root: &Element, path: &[usize], out: &mut Vec<Vec<usize>>) {
    let mut stack = vec![(root, path.to_vec())];
    while let Some((el, path)) = stack.pop() {
        if el.local_name() == "txbxContent" {
            continue;
        }
        if el.local_name() == "AlternateContent" {
            if let Some(i) = first_alt(el) {
                if let Node::Element(c) = &el.children[i] {
                    let mut next = path;
                    next.push(i);
                    stack.push((c, next));
                }
            }
            continue;
        }
        if el.local_name() == "hyperlink" {
            out.push(path.clone());
        }
        let mut kids = Vec::new();
        for (i, n) in el.children.iter().enumerate() {
            if let Node::Element(child) = n {
                let mut next = path.clone();
                next.push(i);
                kids.push((child, next));
            }
        }
        for item in kids.into_iter().rev() {
            stack.push(item);
        }
    }
}

fn collect_bookmark_pairs(root: &Element) -> Vec<BookmarkPair> {
    let mut starts = Vec::new();
    let mut ends: HashMap<String, Vec<usize>> = HashMap::new();
    let mut path = Vec::new();
    para::walk_from_story_root(root, &mut path, |el, path| {
        walk_bookmarks(el, path, true, &mut starts, &mut ends);
    });
    let mut out = Vec::new();
    for (start, xml_id, name) in starts {
        if name.starts_with('_') {
            continue;
        }
        let Some(xml_id) = xml_id else {
            continue;
        };
        let Some(end) = ends.get(&xml_id).cloned() else {
            continue;
        };
        out.push(BookmarkPair {
            start,
            end,
            name,
            xml_id: Some(xml_id),
        });
    }
    out
}

fn walk_bookmarks(
    root: &Element,
    path: &[usize],
    skip_txbx: bool,
    starts: &mut Vec<(Vec<usize>, Option<String>, String)>,
    ends: &mut HashMap<String, Vec<usize>>,
) {
    let mut stack = vec![(root, path.to_vec())];
    while let Some((el, path)) = stack.pop() {
        if skip_txbx && el.local_name() == "txbxContent" {
            continue;
        }
        if el.local_name() == "AlternateContent" {
            if let Some(i) = first_alt(el) {
                if let Node::Element(c) = &el.children[i] {
                    let mut next = path;
                    next.push(i);
                    stack.push((c, next));
                }
            }
            continue;
        }
        match el.local_name() {
            "bookmarkStart" => {
                let xml_id = el.attr_local("id").map(str::to_string);
                let name = el.attr_local("name").unwrap_or("").to_string();
                starts.push((path.clone(), xml_id, name));
            }
            "bookmarkEnd" => {
                if let Some(id) = el.attr_local("id") {
                    ends.entry(id.to_string()).or_insert_with(|| path.clone());
                }
            }
            _ => {}
        }
        let mut kids = Vec::new();
        for (i, n) in el.children.iter().enumerate() {
            if let Node::Element(child) = n {
                let mut next = path.clone();
                next.push(i);
                kids.push((child, next));
            }
        }
        for item in kids.into_iter().rev() {
            stack.push(item);
        }
    }
}

pub(crate) fn all_bookmark_names(root: &Element) -> HashSet<String> {
    let mut names = HashSet::new();
    collect_names(root, &mut names);
    names
}

pub(crate) fn all_bookmark_spans(root: &Element) -> Vec<BookmarkSpan> {
    let mut starts = Vec::new();
    let mut ends: HashMap<String, Vec<usize>> = HashMap::new();
    let path = Vec::new();
    walk_bookmarks(root, &path, true, &mut starts, &mut ends);
    let mut out = Vec::new();
    for (start, xml_id, name) in starts {
        let Some(xml_id) = xml_id else {
            continue;
        };
        let Some(end) = ends.get(&xml_id).cloned() else {
            continue;
        };
        out.push(BookmarkSpan { start, end, name });
    }
    out
}

fn collect_names(el: &Element, names: &mut HashSet<String>) {
    if el.local_name() == "bookmarkStart" {
        if let Some(n) = el.attr_local("name") {
            names.insert(n.to_string());
        }
    }
    for n in &el.children {
        if let Node::Element(e) = n {
            collect_names(e, names);
        }
    }
}

pub(crate) fn next_bookmark_xml_id(root: &Element) -> String {
    let mut max = -1i64;
    walk_xml_ids(root, &mut max);
    (max + 1).max(0).to_string()
}

fn walk_xml_ids(el: &Element, max: &mut i64) {
    crate::xml::walk_elements(el, &mut |node| {
        if matches!(node.local_name(), "bookmarkStart" | "bookmarkEnd") {
            if let Some(id) = node.attr_local("id") {
                if let Ok(n) = id.parse::<i64>() {
                    *max = (*max).max(n);
                }
            }
        }
    });
}

fn unused_ooxp_name(taken: &HashSet<String>) -> String {
    let mut n = 1u32;
    loop {
        let name = format!("ooxp{n}");
        if !taken.contains(&name) {
            return name;
        }
        n += 1;
    }
}

pub(crate) fn insert_bookmark_start(p: &mut Element, xml_id: &str, name: &str) {
    let mut start = Element::new(p.prefixed("bookmarkStart"));
    let id_key = start.prefixed("id");
    let name_key = start.prefixed("name");
    start.set_attr(&id_key, xml_id);
    start.set_attr(&name_key, name);
    let mut at = 0usize;
    for (i, n) in p.children.iter().enumerate() {
        if let Node::Element(e) = n {
            if e.local_name() == "pPr" {
                at = i + 1;
            }
        }
    }
    p.children.insert(at, Node::Element(start));
}

pub(crate) fn insert_bookmark_end(p: &mut Element, xml_id: &str) {
    let mut end = Element::new(p.prefixed("bookmarkEnd"));
    let id_key = end.prefixed("id");
    end.set_attr(&id_key, xml_id);
    p.push_element(end);
}

pub(crate) fn rename_hyperlink_anchors(el: &mut Element, old: &str, new: &str) {
    if el.local_name() == "hyperlink" && el.attr_local("anchor") == Some(old) {
        let key = el
            .attrs
            .iter()
            .find(|(k, _)| k.rsplit(':').next().unwrap_or(k) == "anchor")
            .map(|(k, _)| k.clone())
            .unwrap_or_else(|| el.prefixed("anchor"));
        el.set_attr(&key, new);
    }
    for n in &mut el.children {
        if let Node::Element(e) = n {
            rename_hyperlink_anchors(e, old, new);
        }
    }
}

fn para_index_map(root: &Element) -> Vec<(*const Element, usize)> {
    para::collect_story_paragraphs(root)
        .into_iter()
        .enumerate()
        .map(|(i, p)| (p as *const Element, i + 1))
        .collect()
}

fn host_para_id(
    root: &Element,
    path: &[usize],
    container: Option<&str>,
    index_of: &[(*const Element, usize)],
) -> Value {
    let mut cur = root;
    let mut host: Option<&Element> = None;
    for &i in path {
        match cur.children.get(i) {
            Some(Node::Element(e)) => {
                if e.local_name() == "p" {
                    host = Some(e);
                }
                cur = e;
            }
            _ => return Value::Null,
        }
    }
    let Some(p) = host else {
        return Value::Null;
    };
    let idx = index_of
        .iter()
        .find(|(ptr, _)| std::ptr::eq(*ptr, p))
        .map(|(_, i)| *i);
    let Some(index) = idx else {
        return Value::Null;
    };
    let (id, _) = match container {
        Some(c) => para::para_stable_id_in(c, p, index),
        None => para::para_stable_id(p, index),
    };
    json!(id)
}

pub(crate) fn split_story<'a>(
    id: &'a str,
    path: &str,
    kind: &str,
    reason: IdReason,
) -> Result<(LinkStory, &'a str), Error> {
    let prefix = format!("{kind}:");
    if let Some(rest) = id.strip_prefix(&prefix) {
        return Ok((LinkStory::Body, rest));
    }
    let needle = format!("/{kind}:");
    if let Some((pre, rest)) = id.split_once(&needle) {
        if let Some((kind, hf_type, n)) = para::parse_hf_container_id(pre) {
            return Ok((LinkStory::HeaderFooter { kind, hf_type, n }, rest));
        }
        if let Ok(tx) = para::parse_txbx_id_at(pre, path) {
            return match tx.host {
                HostPara::Body(_) | HostPara::HeaderFooter { .. } => Ok((
                    LinkStory::TextBox {
                        host: tx.host,
                        key: tx.key,
                    },
                    rest,
                )),
                HostPara::InNote { .. } | HostPara::InComment { .. } => Err(Error::with_path(
                    ErrorKind::InvalidId { reason },
                    path.to_string(),
                )),
            };
        }
    }
    Err(Error::with_path(
        ErrorKind::InvalidId { reason },
        path.to_string(),
    ))
}

pub(crate) fn parse_n(
    s: &str,
    path: &str,
    not_int: IdReason,
    from_one: IdReason,
) -> Result<usize, Error> {
    let n: usize = s.parse().map_err(|_| {
        Error::with_path(ErrorKind::InvalidId { reason: not_int }, path.to_string())
    })?;
    if n < 1 {
        return Err(Error::with_path(
            ErrorKind::InvalidId { reason: from_one },
            path.to_string(),
        ));
    }
    Ok(n)
}

fn rid_used_walk(el: &Element, rid: &str) -> bool {
    if el.attr("r:id") == Some(rid) {
        return true;
    }
    el.element_children().any(|c| rid_used_walk(c, rid))
}

fn remove_local_attr(el: &mut Element, local: &str) {
    el.attrs
        .retain(|(k, _)| k.rsplit(':').next().unwrap_or(k) != local);
}

fn unwrap_at(root: &mut Element, path: &[usize]) -> Result<(), Error> {
    let (parent_path, idx) = split_parent(path)?;
    let parent = para::element_at_mut(root, parent_path).ok_or_else(|| xml_broken("hyperlink"))?;
    if idx >= parent.children.len() {
        return Err(xml_broken("hyperlink"));
    }
    let Node::Element(el) = parent.children.remove(idx) else {
        return Err(xml_broken("hyperlink"));
    };
    parent.children.splice(idx..idx, el.children);
    Ok(())
}

fn remove_at(root: &mut Element, path: &[usize]) -> Result<(), Error> {
    let (parent_path, idx) = split_parent(path)?;
    let parent = para::element_at_mut(root, parent_path).ok_or_else(|| xml_broken("bookmark"))?;
    if idx >= parent.children.len() {
        return Err(xml_broken("bookmark"));
    }
    parent.children.remove(idx);
    Ok(())
}

fn split_parent(path: &[usize]) -> Result<(&[usize], usize), Error> {
    if path.is_empty() {
        return Err(xml_broken("path"));
    }
    Ok((&path[..path.len() - 1], path[path.len() - 1]))
}

fn first_alt(el: &Element) -> Option<usize> {
    crate::xml::alternate_child(el).unwrap_or(None)
}

fn xml_broken(detail: &str) -> Error {
    Error::with_path(
        ErrorKind::XmlBroken {
            issue: XmlIssue::Parse,
            detail: Some(detail.into()),
        },
        "document",
    )
}

fn missing_link(id: &str, i: usize) -> Error {
    Error::with_path(
        ErrorKind::HyperlinkNotFound { id: id.to_string() },
        format!("ops[{i}].id"),
    )
}

fn missing_bookmark(id: &str, i: usize) -> Error {
    Error::with_path(
        ErrorKind::BookmarkNotFound { id: id.to_string() },
        format!("ops[{i}].id"),
    )
}

fn missing_para(id: &str, i: usize) -> Error {
    Error::with_path(
        ErrorKind::ParaNotFound { id: id.to_string() },
        format!("ops[{i}].id"),
    )
}

pub fn missing_link_err(id: &str, i: usize) -> Error {
    missing_link(id, i)
}

pub fn missing_bookmark_err(id: &str, i: usize) -> Error {
    missing_bookmark(id, i)
}

use super::connector;
use super::para::ParaTarget;
use super::request::DeleteBookmarkOp;
use super::request::DeleteEditRangeOp;
use super::request::DeleteHyperlinkOp;
use super::request::HyperlinkTarget;
use super::request::InsertBookmarkOp;
use super::request::InsertEditRangeOp;
use super::request::InsertHyperlinkHost;
use super::request::InsertHyperlinkOp;
use super::request::SetBookmarkOp;
use super::request::SetHyperlinkOp;
use super::shape;
use super::REL_HYPERLINK;
use crate::opc::Package;
use crate::xml::Document;
// 从 apply 搬来的操作体。
pub(crate) fn refuse_set_text_if_hyperlink(el: &Element, i: usize) -> Result<(), Error> {
    if super::link::para_blocks_set_text(el) {
        return Err(Error::with_path(
            ErrorKind::SetParaTextHasHyperlink,
            format!("ops[{i}].id"),
        ));
    }
    Ok(())
}
pub(crate) fn push_hyperlink_rel(rels: &mut Relationships, url: &str) -> String {
    let id = rels.next_rid();
    rels.push(Relationship {
        id: id.clone(),
        rel_type: REL_HYPERLINK.to_string(),
        target: url.to_string(),
        target_mode: Some("External".into()),
        extra: Vec::new(),
    });
    id
}
pub(crate) fn drop_rel_if_unused(
    pkg: &mut Package,
    root: &Element,
    part: &str,
    rid: &str,
) -> Result<(), Error> {
    if super::link::rid_used(root, rid) {
        return Ok(());
    }
    let mut rels = Relationships::from_package(pkg, part)?;
    rels.remove_by_id(rid);
    rels.write_to(pkg, part)
}
pub(crate) fn apply_insert_hyperlink(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &InsertHyperlinkOp,
    i: usize,
) -> Result<(), Error> {
    match &op.host {
        InsertHyperlinkHost::Para { para, text } => apply_insert_para_hyperlink(
            pkg,
            docs,
            dirty,
            doc_part,
            catalog,
            &op.id,
            para,
            text,
            op.url.as_deref(),
            op.anchor.as_deref(),
            op.tooltip.as_deref(),
            i,
        ),
        InsertHyperlinkHost::Picture(target) => super::drawing_hlink::apply_insert_pic_hlink(
            pkg,
            docs,
            dirty,
            doc_part,
            catalog,
            &op.id,
            target,
            op.url.as_deref(),
            op.anchor.as_deref(),
            op.tooltip.as_deref(),
            i,
        ),
        InsertHyperlinkHost::TextBox(target) => super::drawing_hlink::apply_insert_txbx_hlink(
            pkg,
            docs,
            dirty,
            doc_part,
            catalog,
            &op.id,
            target,
            op.url.as_deref(),
            op.anchor.as_deref(),
            op.tooltip.as_deref(),
            i,
        ),
        InsertHyperlinkHost::Shape(target) => super::drawing_hlink::apply_insert_wsp_hlink(
            pkg,
            docs,
            dirty,
            doc_part,
            catalog,
            &op.id,
            target,
            op.url.as_deref(),
            op.anchor.as_deref(),
            op.tooltip.as_deref(),
            i,
            shape::has_click,
            shape::hlink_parent_mut,
            ErrorKind::ShapeNotFound { id: op.id.clone() },
        ),
        InsertHyperlinkHost::Connector(target) => super::drawing_hlink::apply_insert_wsp_hlink(
            pkg,
            docs,
            dirty,
            doc_part,
            catalog,
            &op.id,
            target,
            op.url.as_deref(),
            op.anchor.as_deref(),
            op.tooltip.as_deref(),
            i,
            connector::has_click,
            connector::hlink_parent_mut,
            ErrorKind::ConnectorNotFound { id: op.id.clone() },
        ),
        InsertHyperlinkHost::Ink(target) => super::drawing_hlink::apply_insert_ink_hlink(
            pkg,
            docs,
            dirty,
            doc_part,
            catalog,
            &op.id,
            target,
            op.url.as_deref(),
            op.anchor.as_deref(),
            op.tooltip.as_deref(),
            i,
        ),
        InsertHyperlinkHost::Model3d(target) => super::drawing_hlink::apply_insert_model3d_hlink(
            pkg,
            docs,
            dirty,
            doc_part,
            catalog,
            &op.id,
            target,
            op.url.as_deref(),
            op.anchor.as_deref(),
            op.tooltip.as_deref(),
            i,
        ),
        InsertHyperlinkHost::Chart(target) => super::drawing_hlink::apply_insert_chart_hlink(
            pkg,
            docs,
            dirty,
            doc_part,
            catalog,
            &op.id,
            target,
            op.url.as_deref(),
            op.anchor.as_deref(),
            op.tooltip.as_deref(),
            i,
        ),
        InsertHyperlinkHost::SmartArt(target) => super::drawing_hlink::apply_insert_smartart_hlink(
            pkg,
            docs,
            dirty,
            doc_part,
            catalog,
            &op.id,
            target,
            op.url.as_deref(),
            op.anchor.as_deref(),
            op.tooltip.as_deref(),
            i,
        ),
    }
}
pub(crate) fn apply_insert_para_hyperlink(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    id: &str,
    para: &ParaTarget,
    text: &str,
    url: Option<&str>,
    anchor: Option<&str>,
    tooltip: Option<&str>,
    i: usize,
) -> Result<(), Error> {
    let part_name = super::apply::part_for_para(pkg, catalog, doc_part, para, id, i)?;
    {
        let doc = super::apply::load_part(pkg, docs, &part_name)?;
        if super::apply::find_insert_para_mut(&mut doc.root, para).is_none() {
            return Err(Error::with_path(
                ErrorKind::ParaNotFound { id: id.to_string() },
                format!("ops[{i}].id"),
            ));
        }
    }
    let rid = if let Some(url) = url {
        let mut rels = Relationships::from_package(pkg, &part_name)?;
        let rid = push_hyperlink_rel(&mut rels, url);
        rels.write_to(pkg, &part_name)?;
        Some(rid)
    } else {
        None
    };
    let doc = super::apply::load_part(pkg, docs, &part_name)?;
    let el = super::apply::find_insert_para_mut(&mut doc.root, para).ok_or_else(|| {
        Error::with_path(
            ErrorKind::ParaNotFound { id: id.to_string() },
            format!("ops[{i}].id"),
        )
    })?;
    super::link::insert_hyperlink_el(el, text, rid.as_deref(), anchor, tooltip);
    dirty.insert(part_name, ());
    Ok(())
}
pub(crate) fn apply_set_hyperlink(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &SetHyperlinkOp,
    i: usize,
) -> Result<(), Error> {
    match &op.target {
        HyperlinkTarget::Link(target) => {
            apply_set_para_hyperlink(pkg, docs, dirty, doc_part, catalog, &op.id, target, op, i)
        }
        HyperlinkTarget::Picture(target) => super::drawing_hlink::apply_set_pic_hlink(
            pkg, docs, dirty, doc_part, catalog, &op.id, target, op, i,
        ),
        HyperlinkTarget::TextBox(target) => super::drawing_hlink::apply_set_txbx_hlink(
            pkg, docs, dirty, doc_part, catalog, &op.id, target, op, i,
        ),
        HyperlinkTarget::Shape(target) => super::drawing_hlink::apply_set_wsp_hlink(
            pkg,
            docs,
            dirty,
            doc_part,
            catalog,
            &op.id,
            target,
            op,
            i,
            shape::has_click,
            shape::hlink_parent_mut,
            ErrorKind::ShapeNotFound { id: op.id.clone() },
        ),
        HyperlinkTarget::Connector(target) => super::drawing_hlink::apply_set_wsp_hlink(
            pkg,
            docs,
            dirty,
            doc_part,
            catalog,
            &op.id,
            target,
            op,
            i,
            connector::has_click,
            connector::hlink_parent_mut,
            ErrorKind::ConnectorNotFound { id: op.id.clone() },
        ),
        HyperlinkTarget::Ink(target) => super::drawing_hlink::apply_set_ink_hlink(
            pkg, docs, dirty, doc_part, catalog, &op.id, target, op, i,
        ),
        HyperlinkTarget::Model3d(target) => super::drawing_hlink::apply_set_model3d_hlink(
            pkg, docs, dirty, doc_part, catalog, &op.id, target, op, i,
        ),
        HyperlinkTarget::Chart(target) => super::drawing_hlink::apply_set_chart_hlink(
            pkg, docs, dirty, doc_part, catalog, &op.id, target, op, i,
        ),
        HyperlinkTarget::SmartArt(target) => super::drawing_hlink::apply_set_smartart_hlink(
            pkg, docs, dirty, doc_part, catalog, &op.id, target, op, i,
        ),
    }
}
pub(crate) fn apply_set_para_hyperlink(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    id: &str,
    target: &super::link::LinkRef,
    op: &SetHyperlinkOp,
    i: usize,
) -> Result<(), Error> {
    let part_name = super::link::part_for_story(catalog, doc_part, &target.story, || {
        super::link::missing_link_err(id, i)
    })?;
    let old_rid = {
        let doc = super::apply::load_part(pkg, docs, &part_name)?;
        let story = super::apply::story_xml_mut(&mut doc.root, &target.story, || {
            super::link::missing_link_err(id, i)
        })?;
        let el = super::link::hyperlink_at_mut(story, target.n, id, i)?;
        super::link::r_id(el).map(str::to_string)
    };
    if let Some(url) = op.url.as_deref() {
        let mut rels = Relationships::from_package(pkg, &part_name)?;
        let rid = if let Some(existing) = old_rid.clone() {
            if let Some(rel) = rels.rels.iter_mut().find(|r| r.id == existing) {
                rel.target = url.to_string();
                existing
            } else {
                rels.push(Relationship {
                    id: existing.clone(),
                    rel_type: REL_HYPERLINK.to_string(),
                    target: url.to_string(),
                    target_mode: Some("External".into()),
                    extra: Vec::new(),
                });
                existing
            }
        } else {
            push_hyperlink_rel(&mut rels, url)
        };
        rels.write_to(pkg, &part_name)?;
        let doc = super::apply::load_part(pkg, docs, &part_name)?;
        let story = super::apply::story_xml_mut(&mut doc.root, &target.story, || {
            super::link::missing_link_err(id, i)
        })?;
        let el = super::link::hyperlink_at_mut(story, target.n, id, i)?;
        super::link::set_hyperlink_url(el, &rid);
    }
    if let Some(anchor) = op.anchor.as_deref() {
        {
            let doc = super::apply::load_part(pkg, docs, &part_name)?;
            let story = super::apply::story_xml_mut(&mut doc.root, &target.story, || {
                super::link::missing_link_err(id, i)
            })?;
            let el = super::link::hyperlink_at_mut(story, target.n, id, i)?;
            super::link::set_hyperlink_anchor(el, anchor);
        }
        if let Some(rid) = old_rid.as_deref() {
            let doc = super::apply::load_part(pkg, docs, &part_name)?;
            drop_rel_if_unused(pkg, &doc.root, &part_name, rid)?;
        }
    }
    if let Some(text) = op.text.as_deref() {
        let doc = super::apply::load_part(pkg, docs, &part_name)?;
        let story = super::apply::story_xml_mut(&mut doc.root, &target.story, || {
            super::link::missing_link_err(id, i)
        })?;
        let el = super::link::hyperlink_at_mut(story, target.n, id, i)?;
        para::set_visible_text(el, text);
    }
    if let Some(tooltip) = op.tooltip.as_ref() {
        let doc = super::apply::load_part(pkg, docs, &part_name)?;
        let story = super::apply::story_xml_mut(&mut doc.root, &target.story, || {
            super::link::missing_link_err(id, i)
        })?;
        let el = super::link::hyperlink_at_mut(story, target.n, id, i)?;
        super::link::set_hyperlink_tooltip(el, tooltip.as_deref());
    }
    dirty.insert(part_name, ());
    Ok(())
}
pub(crate) fn apply_delete_hyperlink(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &DeleteHyperlinkOp,
    i: usize,
) -> Result<(), Error> {
    match &op.target {
        HyperlinkTarget::Link(target) => {
            apply_delete_para_hyperlink(pkg, docs, dirty, doc_part, catalog, &op.id, target, i)
        }
        HyperlinkTarget::Picture(target) => super::drawing_hlink::apply_delete_pic_hlink(
            pkg, docs, dirty, doc_part, catalog, &op.id, target, i,
        ),
        HyperlinkTarget::TextBox(target) => super::drawing_hlink::apply_delete_txbx_hlink(
            pkg, docs, dirty, doc_part, catalog, &op.id, target, i,
        ),
        HyperlinkTarget::Shape(target) => super::drawing_hlink::apply_delete_wsp_hlink(
            pkg,
            docs,
            dirty,
            doc_part,
            catalog,
            &op.id,
            target,
            i,
            shape::hlink_parent_mut,
        ),
        HyperlinkTarget::Connector(target) => super::drawing_hlink::apply_delete_wsp_hlink(
            pkg,
            docs,
            dirty,
            doc_part,
            catalog,
            &op.id,
            target,
            i,
            connector::hlink_parent_mut,
        ),
        HyperlinkTarget::Ink(target) => super::drawing_hlink::apply_delete_ink_hlink(
            pkg, docs, dirty, doc_part, catalog, &op.id, target, i,
        ),
        HyperlinkTarget::Model3d(target) => super::drawing_hlink::apply_delete_model3d_hlink(
            pkg, docs, dirty, doc_part, catalog, &op.id, target, i,
        ),
        HyperlinkTarget::Chart(target) => super::drawing_hlink::apply_delete_chart_hlink(
            pkg, docs, dirty, doc_part, catalog, &op.id, target, i,
        ),
        HyperlinkTarget::SmartArt(target) => super::drawing_hlink::apply_delete_smartart_hlink(
            pkg, docs, dirty, doc_part, catalog, &op.id, target, i,
        ),
    }
}
pub(crate) fn apply_delete_para_hyperlink(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    id: &str,
    target: &super::link::LinkRef,
    i: usize,
) -> Result<(), Error> {
    let part_name = super::link::part_for_story(catalog, doc_part, &target.story, || {
        super::link::missing_link_err(id, i)
    })?;
    let old_rid = {
        let doc = super::apply::load_part(pkg, docs, &part_name)?;
        let story = super::apply::story_xml_mut(&mut doc.root, &target.story, || {
            super::link::missing_link_err(id, i)
        })?;
        let el = super::link::hyperlink_at_mut(story, target.n, id, i)?;
        super::link::r_id(el).map(str::to_string)
    };
    {
        let doc = super::apply::load_part(pkg, docs, &part_name)?;
        let story = super::apply::story_xml_mut(&mut doc.root, &target.story, || {
            super::link::missing_link_err(id, i)
        })?;
        super::link::unwrap_hyperlink(story, target.n, id, i)?;
    }
    if let Some(rid) = old_rid {
        let doc = super::apply::load_part(pkg, docs, &part_name)?;
        drop_rel_if_unused(pkg, &doc.root, &part_name, &rid)?;
    }
    dirty.insert(part_name, ());
    Ok(())
}
pub(crate) fn apply_insert_bookmark(
    pkg: &Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &InsertBookmarkOp,
    i: usize,
) -> Result<(), Error> {
    let part_name = super::apply::part_for_para(pkg, catalog, doc_part, &op.start, &op.id, i)?;
    let doc = super::apply::load_part(pkg, docs, &part_name)?;
    let start = super::apply::para_target_path(&doc.root, &op.start).ok_or_else(|| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: crate::error::IdReason::InsertBookmarkNeedsPara,
            },
            format!("ops[{i}].id"),
        )
    })?;
    let end = super::apply::para_target_path(&doc.root, &op.end).ok_or_else(|| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: crate::error::IdReason::InsertBookmarkNeedsPara,
            },
            format!("ops[{i}].end"),
        )
    })?;
    super::link::insert_bookmark(&mut doc.root, &start, &end, op.name.as_deref(), &op.id, i)?;
    dirty.insert(part_name, ());
    Ok(())
}
pub(crate) fn apply_insert_edit_range(
    pkg: &Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &InsertEditRangeOp,
    i: usize,
) -> Result<(), Error> {
    let part_name = super::apply::part_for_para(pkg, catalog, doc_part, &op.para, &op.id, i)?;
    let para = super::apply::story_body_para(&op.para).ok_or_else(|| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: crate::error::IdReason::InsertEditRangeNeedsPara,
            },
            format!("ops[{i}].id"),
        )
    })?;
    let doc = super::apply::load_part(pkg, docs, &part_name)?;
    super::perm::insert_edit_range(
        &mut doc.root,
        para,
        op.ed_grp.as_deref(),
        op.ed.as_deref(),
        &op.id,
        i,
    )?;
    dirty.insert(part_name, ());
    Ok(())
}
pub(crate) fn apply_delete_edit_range(
    pkg: &Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &DeleteEditRangeOp,
    i: usize,
) -> Result<(), Error> {
    let part_name = super::perm::part_for_story(catalog, doc_part, &op.target.story, || {
        super::perm::missing_perm_err(&op.id, i)
    })?;
    let doc = super::apply::load_part(pkg, docs, &part_name)?;
    super::perm::delete_edit_range(&mut doc.root, op.target.n, &op.id, i)?;
    dirty.insert(part_name, ());
    Ok(())
}
pub(crate) fn apply_set_bookmark(
    pkg: &Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &SetBookmarkOp,
    i: usize,
) -> Result<(), Error> {
    let part_name = super::link::part_for_story(catalog, doc_part, &op.target.story, || {
        super::link::missing_bookmark_err(&op.id, i)
    })?;
    let doc = super::apply::load_part(pkg, docs, &part_name)?;
    let taken = super::link::all_bookmark_names(&doc.root);
    let renamed = {
        let story = super::apply::story_xml_mut(&mut doc.root, &op.target.story, || {
            super::link::missing_bookmark_err(&op.id, i)
        })?;
        super::link::set_bookmark_name(story, op.target.n, &op.name, &taken, &op.id, i)?
    };
    if let Some((old, new)) = renamed {
        super::link::rename_hyperlink_anchors(&mut doc.root, &old, &new);
    }
    dirty.insert(part_name, ());
    Ok(())
}
pub(crate) fn apply_delete_bookmark(
    pkg: &Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &DeleteBookmarkOp,
    i: usize,
) -> Result<(), Error> {
    let part_name = super::link::part_for_story(catalog, doc_part, &op.target.story, || {
        super::link::missing_bookmark_err(&op.id, i)
    })?;
    let doc = super::apply::load_part(pkg, docs, &part_name)?;
    let story = super::apply::story_xml_mut(&mut doc.root, &op.target.story, || {
        super::link::missing_bookmark_err(&op.id, i)
    })?;
    super::link::delete_bookmark(story, op.target.n, &op.id, i)?;
    dirty.insert(part_name, ());
    Ok(())
}
