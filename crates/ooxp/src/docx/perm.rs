//! 例外编辑区：`w:permStart` / `w:permEnd` 的 inspect 与插删。
//! 框内、注内、批注内不编。不写 IRM。

use std::collections::HashMap;

use serde_json::{json, Value};

use crate::error::{Error, ErrorKind, IdReason};
use crate::xml::{Element, Node};

use super::hf::HfPart;
use super::link::{self, LinkStory};
use super::para::{self, BodyPara};

const ED_GRP: &[&str] = &[
    "everyone",
    "administrators",
    "contributors",
    "editors",
    "owners",
    "current",
];

#[derive(Debug, Clone)]
pub struct PermRef {
    pub story: LinkStory,
    pub n: usize,
}

struct PermHit {
    start: Vec<usize>,
    end: Option<Vec<usize>>,
    xml_id: Option<String>,
    ed_grp: Option<String>,
    ed: Option<String>,
}

struct PermStartRec {
    path: Vec<usize>,
    xml_id: Option<String>,
    ed_grp: Option<String>,
    ed: Option<String>,
}

pub fn ed_grp_ok(s: &str) -> bool {
    ED_GRP.contains(&s)
}

pub fn count_edit_ranges(root: &Element) -> usize {
    collect_hits(root).len()
}

pub fn edit_ranges_json(root: &Element, container: Option<&str>) -> Result<Vec<Value>, Error> {
    let index_of = para_index_map(root);
    let hits = collect_hits(root);
    let mut out = Vec::new();
    for (i, hit) in hits.iter().enumerate() {
        let n = i + 1;
        let id = match container {
            Some(c) => format!("{c}/perm:{n}"),
            None => format!("perm:{n}"),
        };
        let text = match &hit.end {
            Some(end) => text_between(root, &hit.start, end),
            None => String::new(),
        };
        out.push(json!({
            "id": id,
            "index": n,
            "xmlId": hit.xml_id,
            "edGrp": hit.ed_grp,
            "ed": hit.ed,
            "host": host_para_id(root, &hit.start, container, &index_of),
            "text": text,
        }));
    }
    Ok(out)
}

pub fn parse_perm_ref(id: &str, path: &str) -> Result<PermRef, Error> {
    let (story, rest) = link::split_story(id, path, "perm", IdReason::PermPrefix)?;
    if matches!(story, LinkStory::TextBox { .. }) || rest.contains('/') {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::PermPrefix,
            },
            path.to_string(),
        ));
    }
    let n = link::parse_n(
        rest,
        path,
        IdReason::PermIndexNotInt,
        IdReason::PermIndexFromOne,
    )?;
    Ok(PermRef { story, n })
}

pub fn part_for_story(
    catalog: &[HfPart],
    doc_part: &str,
    story: &LinkStory,
    missing: impl FnOnce() -> Error,
) -> Result<String, Error> {
    link::part_for_story(catalog, doc_part, story, missing)
}

pub fn missing_perm_err(id: &str, i: usize) -> Error {
    Error::with_path(
        ErrorKind::EditRangeNotFound { id: id.to_string() },
        format!("ops[{i}].id"),
    )
}

pub fn insert_edit_range(
    root: &mut Element,
    para: &BodyPara,
    ed_grp: Option<&str>,
    ed: Option<&str>,
    err_id: &str,
    op_i: usize,
) -> Result<(), Error> {
    let path = para::find_paragraph_path(root, para).ok_or_else(|| {
        Error::with_path(
            ErrorKind::ParaNotFound {
                id: err_id.to_string(),
            },
            format!("ops[{op_i}].id"),
        )
    })?;
    let xml_id = next_perm_xml_id(root);
    let p = para::element_at_mut(root, &path).ok_or_else(|| {
        Error::with_path(
            ErrorKind::ParaNotFound {
                id: err_id.to_string(),
            },
            format!("ops[{op_i}].id"),
        )
    })?;
    insert_perm_start(p, &xml_id, ed_grp, ed);
    insert_perm_end(p, &xml_id);
    Ok(())
}

pub fn delete_edit_range(root: &mut Element, n: usize, id: &str, i: usize) -> Result<(), Error> {
    let hits = collect_hits(root);
    if n < 1 || n > hits.len() {
        return Err(missing_perm_err(id, i));
    }
    let hit = &hits[n - 1];
    let mut paths = vec![hit.start.clone()];
    if let Some(end) = &hit.end {
        paths.push(end.clone());
    }
    paths.sort_by(|a, b| cmp_path(a, b).reverse());
    for path in paths {
        remove_at(root, &path)?;
    }
    Ok(())
}

fn collect_hits(root: &Element) -> Vec<PermHit> {
    let mut starts = Vec::new();
    let mut ends: HashMap<String, Vec<usize>> = HashMap::new();
    let mut path = Vec::new();
    walk_perm(root, &mut path, true, &mut starts, &mut ends);
    let mut out = Vec::new();
    for rec in starts {
        let end = rec.xml_id.as_ref().and_then(|id| ends.get(id).cloned());
        out.push(PermHit {
            start: rec.path,
            end,
            xml_id: rec.xml_id,
            ed_grp: rec.ed_grp,
            ed: rec.ed,
        });
    }
    out
}

fn walk_perm(
    el: &Element,
    path: &mut Vec<usize>,
    skip_txbx: bool,
    starts: &mut Vec<PermStartRec>,
    ends: &mut HashMap<String, Vec<usize>>,
) {
    if skip_txbx && el.local_name() == "txbxContent" {
        return;
    }
    if el.local_name() == "AlternateContent" {
        if let Some(i) = first_alt(el) {
            path.push(i);
            if let Node::Element(c) = &el.children[i] {
                walk_perm(c, path, skip_txbx, starts, ends);
            }
            path.pop();
        }
        return;
    }
    match el.local_name() {
        "permStart" => {
            starts.push(PermStartRec {
                path: path.clone(),
                xml_id: el.attr_local("id").map(str::to_string),
                ed_grp: el.attr_local("edGrp").map(str::to_string),
                ed: el.attr_local("ed").map(str::to_string),
            });
        }
        "permEnd" => {
            if let Some(id) = el.attr_local("id") {
                ends.entry(id.to_string()).or_insert_with(|| path.clone());
            }
        }
        _ => {}
    }
    for (i, n) in el.children.iter().enumerate() {
        if let Node::Element(e) = n {
            path.push(i);
            walk_perm(e, path, skip_txbx, starts, ends);
            path.pop();
        }
    }
}

fn next_perm_xml_id(root: &Element) -> String {
    let mut max = -1i64;
    walk_xml_ids(root, &mut max);
    (max + 1).max(0).to_string()
}

fn walk_xml_ids(el: &Element, max: &mut i64) {
    if matches!(el.local_name(), "permStart" | "permEnd") {
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

fn insert_perm_start(p: &mut Element, xml_id: &str, ed_grp: Option<&str>, ed: Option<&str>) {
    let mut start = Element::new(p.prefixed("permStart"));
    let id_key = start.prefixed("id");
    start.set_attr(&id_key, xml_id);
    if let Some(g) = ed_grp {
        let key = start.prefixed("edGrp");
        start.set_attr(&key, g);
    }
    if let Some(e) = ed {
        let key = start.prefixed("ed");
        start.set_attr(&key, e);
    }
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

fn insert_perm_end(p: &mut Element, xml_id: &str) {
    let mut end = Element::new(p.prefixed("permEnd"));
    let id_key = end.prefixed("id");
    end.set_attr(&id_key, xml_id);
    p.push_element(end);
}

fn text_between(root: &Element, start: &[usize], end: &[usize]) -> String {
    let mut out = String::new();
    let mut path = Vec::new();
    let mut on = false;
    walk_text(root, &mut path, start, end, &mut on, &mut out);
    out
}

fn walk_text(
    el: &Element,
    path: &mut Vec<usize>,
    start: &[usize],
    end: &[usize],
    on: &mut bool,
    out: &mut String,
) {
    if el.local_name() == "txbxContent" {
        return;
    }
    if el.local_name() == "AlternateContent" {
        if let Some(i) = first_alt(el) {
            path.push(i);
            if let Node::Element(c) = &el.children[i] {
                walk_text(c, path, start, end, on, out);
            }
            path.pop();
        }
        return;
    }
    if path.as_slice() == start {
        *on = true;
    } else if path.as_slice() == end {
        *on = false;
        return;
    } else if *on && el.local_name() == "t" {
        out.push_str(&el.text_content());
        return;
    }
    for (i, n) in el.children.iter().enumerate() {
        if let Node::Element(e) = n {
            path.push(i);
            walk_text(e, path, start, end, on, out);
            path.pop();
        }
    }
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

fn para_index_map(root: &Element) -> Vec<(*const Element, usize)> {
    para::collect_paragraphs(root)
        .into_iter()
        .enumerate()
        .map(|(i, p)| (p as *const Element, i + 1))
        .collect()
}

fn remove_at(root: &mut Element, path: &[usize]) -> Result<(), Error> {
    let (last, prefix) = path.split_last().ok_or_else(xml_broken)?;
    let parent = para::element_at_mut(root, prefix).ok_or_else(xml_broken)?;
    if *last >= parent.children.len() {
        return Err(xml_broken());
    }
    parent.children.remove(*last);
    Ok(())
}

fn cmp_path(a: &[usize], b: &[usize]) -> std::cmp::Ordering {
    for (x, y) in a.iter().zip(b.iter()) {
        match x.cmp(y) {
            std::cmp::Ordering::Equal => continue,
            o => return o,
        }
    }
    a.len().cmp(&b.len())
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

fn xml_broken() -> Error {
    Error::xml_detail(
        crate::error::XmlIssue::Parse,
        "permStart/permEnd marker is not well formed",
    )
}
