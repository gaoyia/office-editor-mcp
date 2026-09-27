//! 正文与页眉页脚里的修订痕迹：inspect 名单与接受 / 拒绝。

use serde_json::{json, Value};

use crate::error::{Error, ErrorKind, IdReason, XmlIssue};
use crate::xml::{Element, Node};

use super::hf::{self, HfKind, HfPart, HfType};
use super::para;

#[derive(Debug, Clone)]
pub enum RevStory {
    Body,
    HeaderFooter {
        kind: HfKind,
        hf_type: HfType,
        n: u32,
    },
}

#[derive(Debug, Clone)]
pub struct RevRef {
    pub story: RevStory,
    pub n: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RevKind {
    Ins,
    Del,
    RPrChange,
    PPrChange,
}

impl RevKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Ins => "ins",
            Self::Del => "del",
            Self::RPrChange => "rPrChange",
            Self::PPrChange => "pPrChange",
        }
    }

    fn from_name(name: &str) -> Option<Self> {
        match name {
            "ins" => Some(Self::Ins),
            "del" => Some(Self::Del),
            "rPrChange" => Some(Self::RPrChange),
            "pPrChange" => Some(Self::PPrChange),
            _ => None,
        }
    }
}

pub fn count_revisions(root: &Element) -> usize {
    collect_rev_paths(root).len()
}

pub fn revisions_json(root: &Element, container: Option<&str>) -> Result<Vec<Value>, Error> {
    let paras = para::collect_paragraphs(root);
    let mut index_of: Vec<(*const Element, usize)> = Vec::new();
    for (i, p) in paras.iter().enumerate() {
        index_of.push((*p as *const Element, i + 1));
    }
    let paths = collect_rev_paths(root);
    let mut out = Vec::new();
    for (i, path) in paths.iter().enumerate() {
        let el = para::element_at(root, path).ok_or_else(|| xml_broken("revision"))?;
        let kind = RevKind::from_name(el.local_name()).ok_or_else(|| xml_broken("revision"))?;
        let n = i + 1;
        let id = match container {
            Some(c) => format!("{c}/rev:{n}"),
            None => format!("rev:{n}"),
        };
        let xml_id = el.attr_local("id");
        let author = el.attr_local("author");
        let date = el.attr_local("date");
        let host = host_para_id(root, path, container, &index_of);
        let text = match kind {
            RevKind::Ins => json!(markup_text(el, false)),
            RevKind::Del => json!(markup_text(el, true)),
            RevKind::RPrChange | RevKind::PPrChange => Value::Null,
        };
        out.push(json!({
            "id": id,
            "index": n,
            "kind": kind.as_str(),
            "xmlId": xml_id,
            "author": author,
            "date": date,
            "host": host,
            "text": text,
        }));
    }
    Ok(out)
}

pub fn parse_rev_ref(id: &str, path: &str) -> Result<RevRef, Error> {
    let (story, rest) = split_story(id, path)?;
    if rest.contains('/') {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::RevPrefix,
            },
            path.to_string(),
        ));
    }
    let n = parse_n(rest, path)?;
    Ok(RevRef { story, n })
}

pub fn part_for_story(
    catalog: &[HfPart],
    doc_part: &str,
    story: &RevStory,
    id: &str,
    i: usize,
) -> Result<String, Error> {
    match story {
        RevStory::Body => Ok(doc_part.to_string()),
        RevStory::HeaderFooter { kind, hf_type, n } => {
            let cid = format!("{}:{}:{n}", kind.as_str(), hf_type.as_str());
            let item = hf::find_by_id(catalog, &cid).ok_or_else(|| missing(id, i))?;
            Ok(item.part.clone())
        }
    }
}

/// 段内是否有本期收入的修订节点。框内段落同样按本段子孙判定。
pub fn para_blocks_set_text(p: &Element) -> bool {
    !collect_rev_paths(p).is_empty()
}

pub fn apply_revision(
    root: &mut Element,
    n: usize,
    accept: bool,
    id: &str,
    i: usize,
) -> Result<(), Error> {
    let paths = collect_rev_paths(root);
    if n < 1 || n > paths.len() {
        return Err(missing(id, i));
    }
    let path = paths[n - 1].clone();
    let kind = {
        let el = para::element_at(root, &path).ok_or_else(|| missing(id, i))?;
        RevKind::from_name(el.local_name()).ok_or_else(|| xml_broken("revision"))?
    };
    match (kind, accept) {
        (RevKind::Ins, true) | (RevKind::Del, false) => {
            unwrap_at(root, &path, kind == RevKind::Del)?
        }
        (RevKind::Ins, false) | (RevKind::Del, true) => remove_at(root, &path)?,
        (RevKind::RPrChange, true) | (RevKind::PPrChange, true) => remove_at(root, &path)?,
        (RevKind::RPrChange, false) => restore_prev(root, &path, "rPr")?,
        (RevKind::PPrChange, false) => restore_prev(root, &path, "pPr")?,
    }
    Ok(())
}

fn collect_rev_paths(root: &Element) -> Vec<Vec<usize>> {
    let mut path = Vec::new();
    let mut out = Vec::new();
    walk_rev(root, &mut path, false, &mut out);
    out
}

fn walk_rev(el: &Element, path: &mut Vec<usize>, in_prop: bool, out: &mut Vec<Vec<usize>>) {
    if el.local_name() == "txbxContent" {
        return;
    }
    if el.local_name() == "AlternateContent" {
        if let Some(i) = first_alt(el) {
            path.push(i);
            if let Node::Element(c) = &el.children[i] {
                walk_rev(c, path, in_prop, out);
            }
            path.pop();
        }
        return;
    }
    let name = el.local_name();
    if matches!(name, "ins" | "del") {
        if is_run_ins_del(el, in_prop) {
            out.push(path.clone());
        }
    } else if matches!(name, "rPrChange" | "pPrChange") {
        out.push(path.clone());
    }
    let next_prop = in_prop || is_prop_container(name);
    for (i, n) in el.children.iter().enumerate() {
        if let Node::Element(e) = n {
            path.push(i);
            walk_rev(e, path, next_prop, out);
            path.pop();
        }
    }
}

fn is_prop_container(name: &str) -> bool {
    matches!(
        name,
        "pPr" | "rPr" | "trPr" | "tblPr" | "tcPr" | "sectPr" | "numPr"
    )
}

fn is_run_ins_del(el: &Element, in_prop: bool) -> bool {
    if in_prop {
        return false;
    }
    !el.element_children()
        .any(|c| matches!(c.local_name(), "p" | "tbl"))
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

fn markup_text(el: &Element, deleted: bool) -> String {
    let mut out = String::new();
    collect_markup_text(el, deleted, &mut out);
    out
}

fn collect_markup_text(el: &Element, deleted: bool, out: &mut String) {
    let name = el.local_name();
    if deleted {
        if name == "delText" || name == "t" {
            out.push_str(&el.text_content());
            return;
        }
    } else if name == "t" {
        out.push_str(&el.text_content());
        return;
    }
    if name == "txbxContent" {
        return;
    }
    if name == "AlternateContent" {
        if let Some(i) = first_alt(el) {
            if let Node::Element(c) = &el.children[i] {
                collect_markup_text(c, deleted, out);
            }
        }
        return;
    }
    for n in &el.children {
        if let Node::Element(e) = n {
            collect_markup_text(e, deleted, out);
        }
    }
}

fn split_story<'a>(id: &'a str, path: &str) -> Result<(RevStory, &'a str), Error> {
    if let Some(rest) = id.strip_prefix("rev:") {
        return Ok((RevStory::Body, rest));
    }
    if let Some((pre, rest)) = id.split_once("/rev:") {
        if let Some((kind, hf_type, n)) = para::parse_hf_container_id(pre) {
            return Ok((RevStory::HeaderFooter { kind, hf_type, n }, rest));
        }
    }
    Err(Error::with_path(
        ErrorKind::InvalidId {
            reason: IdReason::RevPrefix,
        },
        path.to_string(),
    ))
}

fn parse_n(s: &str, path: &str) -> Result<usize, Error> {
    let n: usize = s.parse().map_err(|_| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::RevIndexNotInt,
            },
            path.to_string(),
        )
    })?;
    if n < 1 {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::RevIndexFromOne,
            },
            path.to_string(),
        ));
    }
    Ok(n)
}

fn remove_at(root: &mut Element, path: &[usize]) -> Result<(), Error> {
    let (parent_path, idx) = split_parent(path)?;
    let parent = para::element_at_mut(root, parent_path).ok_or_else(|| xml_broken("revision"))?;
    if idx >= parent.children.len() {
        return Err(xml_broken("revision"));
    }
    parent.children.remove(idx);
    Ok(())
}

fn unwrap_at(root: &mut Element, path: &[usize], restore_del: bool) -> Result<(), Error> {
    let (parent_path, idx) = split_parent(path)?;
    let parent = para::element_at_mut(root, parent_path).ok_or_else(|| xml_broken("revision"))?;
    if idx >= parent.children.len() {
        return Err(xml_broken("revision"));
    }
    let Node::Element(mut el) = parent.children.remove(idx) else {
        return Err(xml_broken("revision"));
    };
    if restore_del {
        restore_del_text(&mut el);
    }
    // Word 会把同一条修订再写进 rPr 的 ins/del；拆外壳后必须去掉，否则仍显示为修订。
    strip_prop_ins_del(&mut el);
    parent.children.splice(idx..idx, el.children);
    Ok(())
}

fn restore_prev(root: &mut Element, path: &[usize], inner: &str) -> Result<(), Error> {
    let prev_children = {
        let change = para::element_at(root, path).ok_or_else(|| xml_broken("revision"))?;
        let prev = change
            .child_by_local(inner)
            .ok_or_else(|| xml_broken(inner))?;
        prev.children.clone()
    };
    let (parent_path, _) = split_parent(path)?;
    let parent = para::element_at_mut(root, parent_path).ok_or_else(|| xml_broken("revision"))?;
    parent.children = prev_children;
    Ok(())
}

fn restore_del_text(el: &mut Element) {
    if el.local_name() == "delText" {
        el.name = el.prefixed("t");
    }
    for n in &mut el.children {
        if let Node::Element(e) = n {
            restore_del_text(e);
        }
    }
}

fn strip_prop_ins_del(el: &mut Element) {
    if is_prop_container(el.local_name()) {
        el.children.retain(|n| match n {
            Node::Element(e) => !matches!(e.local_name(), "ins" | "del"),
            _ => true,
        });
    }
    for n in &mut el.children {
        if let Node::Element(e) = n {
            strip_prop_ins_del(e);
        }
    }
}

fn split_parent(path: &[usize]) -> Result<(&[usize], usize), Error> {
    if path.is_empty() {
        return Err(xml_broken("revision"));
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

fn missing(id: &str, i: usize) -> Error {
    Error::with_path(
        ErrorKind::RevisionNotFound { id: id.to_string() },
        format!("ops[{i}].id"),
    )
}
