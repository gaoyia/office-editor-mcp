//! 正文与页眉页脚里的 `w:hyperlink` 与配对书签：inspect 名单与插改删。

use std::collections::{HashMap, HashSet};

use serde_json::{json, Value};

use crate::error::{Error, ErrorKind, IdReason, XmlIssue};
use crate::opc::Relationships;
use crate::xml::{Element, Node};

use super::hf::{self, HfKind, HfPart, HfType};
use super::para::{self, BodyPara};

#[derive(Debug, Clone)]
pub enum LinkStory {
    Body,
    HeaderFooter {
        kind: HfKind,
        hf_type: HfType,
        n: u32,
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
    start: &BodyPara,
    end: &BodyPara,
    name: Option<&str>,
    id: &str,
    i: usize,
) -> Result<(), Error> {
    let start_path = para::find_paragraph_path(root, start).ok_or_else(|| missing_para(id, i))?;
    let end_path = para::find_paragraph_path(root, end).ok_or_else(|| missing_para(id, i))?;
    if para_is_before(root, end, start)? {
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
        let p = para::element_at_mut(root, &start_path).ok_or_else(|| missing_para(id, i))?;
        insert_bookmark_start(p, &xml_id, &name);
        insert_bookmark_end(p, &xml_id);
    } else {
        {
            let p = para::element_at_mut(root, &start_path).ok_or_else(|| missing_para(id, i))?;
            insert_bookmark_start(p, &xml_id, &name);
        }
        {
            let p = para::element_at_mut(root, &end_path).ok_or_else(|| missing_para(id, i))?;
            insert_bookmark_end(p, &xml_id);
        }
    }
    Ok(())
}

pub fn set_bookmark_name(
    root: &mut Element,
    n: usize,
    name: &str,
    id: &str,
    i: usize,
) -> Result<(), Error> {
    check_bookmark_name(name, i, "name")?;
    let pairs = collect_bookmark_pairs(root);
    if n < 1 || n > pairs.len() {
        return Err(missing_bookmark(id, i));
    }
    let pair = &pairs[n - 1];
    if pair.name == name {
        return Ok(());
    }
    let taken = all_bookmark_names(root);
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
    rename_hyperlink_anchors(root, &old, name);
    Ok(())
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
    walk_hyperlinks(root, &mut path, &mut out);
    out
}

fn walk_hyperlinks(el: &Element, path: &mut Vec<usize>, out: &mut Vec<Vec<usize>>) {
    if el.local_name() == "txbxContent" {
        return;
    }
    if el.local_name() == "AlternateContent" {
        if let Some(i) = first_alt(el) {
            path.push(i);
            if let Node::Element(c) = &el.children[i] {
                walk_hyperlinks(c, path, out);
            }
            path.pop();
        }
        return;
    }
    if el.local_name() == "hyperlink" {
        out.push(path.clone());
    }
    for (i, n) in el.children.iter().enumerate() {
        if let Node::Element(e) = n {
            path.push(i);
            walk_hyperlinks(e, path, out);
            path.pop();
        }
    }
}

fn collect_bookmark_pairs(root: &Element) -> Vec<BookmarkPair> {
    let mut starts = Vec::new();
    let mut ends: HashMap<String, Vec<usize>> = HashMap::new();
    let mut path = Vec::new();
    walk_bookmarks(root, &mut path, true, &mut starts, &mut ends);
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
    el: &Element,
    path: &mut Vec<usize>,
    skip_txbx: bool,
    starts: &mut Vec<(Vec<usize>, Option<String>, String)>,
    ends: &mut HashMap<String, Vec<usize>>,
) {
    if skip_txbx && el.local_name() == "txbxContent" {
        return;
    }
    if el.local_name() == "AlternateContent" {
        if let Some(i) = first_alt(el) {
            path.push(i);
            if let Node::Element(c) = &el.children[i] {
                walk_bookmarks(c, path, skip_txbx, starts, ends);
            }
            path.pop();
        }
        return;
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
    for (i, n) in el.children.iter().enumerate() {
        if let Node::Element(e) = n {
            path.push(i);
            walk_bookmarks(e, path, skip_txbx, starts, ends);
            path.pop();
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
    let mut path = Vec::new();
    walk_bookmarks(root, &mut path, true, &mut starts, &mut ends);
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
    if matches!(el.local_name(), "bookmarkStart" | "bookmarkEnd") {
        if let Some(id) = el.attr_local("id") {
            if let Ok(n) = id.parse::<i64>() {
                *max = (*max).max(n);
            }
        }
    }
    for n in &el.children {
        if let Node::Element(e) = n {
            walk_xml_ids(e, max);
        }
    }
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

fn rename_hyperlink_anchors(el: &mut Element, old: &str, new: &str) {
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

fn para_is_before(root: &Element, a: &BodyPara, b: &BodyPara) -> Result<bool, Error> {
    let paras = para::collect_paragraphs(root);
    let ai = paras
        .iter()
        .enumerate()
        .position(|(i, p)| para_matches(p, i + 1, a));
    let bi = paras
        .iter()
        .enumerate()
        .position(|(i, p)| para_matches(p, i + 1, b));
    match (ai, bi) {
        (Some(a), Some(b)) => Ok(a < b),
        _ => Err(xml_broken("bookmark para")),
    }
}

fn para_matches(p: &Element, index: usize, target: &BodyPara) -> bool {
    match target {
        BodyPara::Index(n) => index == *n,
        BodyPara::Hex(hex) => p
            .attr_local("paraId")
            .is_some_and(|id| id.eq_ignore_ascii_case(hex)),
    }
}

fn para_index_map(root: &Element) -> Vec<(*const Element, usize)> {
    para::collect_paragraphs(root)
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
