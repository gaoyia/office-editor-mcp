//! 正文、页眉页脚与文本框里的 `w:ruby`：inspect 名单与插改删。
//! 注内、批注内本期不编。不算排版引擎。

use serde_json::{json, Value};

use crate::error::{Error, ErrorKind, IdReason, XmlIssue};
use crate::xml::{Element, Node};

use super::hf::HfPart;
use super::link::{self, LinkStory};
use super::para::{self, BodyPara};

pub const ALIGN_SET: &[&str] = &[
    "center",
    "distributeLetter",
    "distributeSpace",
    "left",
    "right",
    "rightVertical",
];

const RUBY_PR_ORDER: &[&str] = &["rubyAlign", "hps", "hpsRaise", "hpsBaseText", "lid"];

pub struct InsertSpec<'a> {
    pub base: &'a str,
    pub rt: &'a str,
    pub align: &'a str,
    pub hps: u32,
    pub hps_raise: u32,
    pub hps_base_text: u32,
    pub lid: &'a str,
}

pub struct SetPatch<'a> {
    pub base: Option<&'a str>,
    pub rt: Option<&'a str>,
    pub align: Option<&'a str>,
    pub hps: Option<u32>,
    pub hps_raise: Option<u32>,
    pub hps_base_text: Option<u32>,
    pub lid: Option<&'a str>,
}

#[derive(Debug, Clone)]
pub struct RubyRef {
    pub story: LinkStory,
    pub n: usize,
}

struct RubyRec {
    path: Vec<usize>,
}

pub fn count_rubies(root: &Element) -> usize {
    collect_rubies(root).len()
}

pub fn rubies_json(root: &Element, container: Option<&str>) -> Result<Vec<Value>, Error> {
    let index_of = para_index_map(root);
    let recs = collect_rubies(root);
    let mut out = Vec::new();
    for (i, rec) in recs.iter().enumerate() {
        let n = i + 1;
        let id = match container {
            Some(c) => format!("{c}/ruby:{n}"),
            None => format!("ruby:{n}"),
        };
        let ruby = para::element_at(root, &rec.path).ok_or_else(|| xml_broken("ruby"))?;
        let pr = ruby.child_by_local("rubyPr");
        out.push(json!({
            "id": id,
            "index": n,
            "host": host_para_id(root, &rec.path, container, &index_of),
            "base": side_text(ruby, "rubyBase"),
            "rt": side_text(ruby, "rt"),
            "align": pr.and_then(|e| child_val(e, "rubyAlign")),
            "hps": pr.and_then(|e| child_int(e, "hps")),
            "hpsRaise": pr.and_then(|e| child_int(e, "hpsRaise")),
            "hpsBaseText": pr.and_then(|e| child_int(e, "hpsBaseText")),
            "lid": pr.and_then(|e| child_val(e, "lid")),
        }));
    }
    Ok(out)
}

pub fn parse_ruby_ref(id: &str, path: &str) -> Result<RubyRef, Error> {
    let (story, rest) = link::split_story(id, path, "ruby", IdReason::RubyPrefix)?;
    if rest.contains('/') {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::RubyPrefix,
            },
            path.to_string(),
        ));
    }
    let n = link::parse_n(
        rest,
        path,
        IdReason::RubyIndexNotInt,
        IdReason::RubyIndexFromOne,
    )?;
    Ok(RubyRef { story, n })
}

pub fn part_for_story(
    catalog: &[HfPart],
    doc_part: &str,
    story: &LinkStory,
    missing: impl FnOnce() -> Error,
) -> Result<String, Error> {
    link::part_for_story(catalog, doc_part, story, missing)
}

pub fn missing_ruby_err(id: &str, i: usize) -> Error {
    Error::with_path(
        ErrorKind::RubyNotFound { id: id.to_string() },
        format!("ops[{i}].id"),
    )
}

/// 目标段落是否落在本期收入的 `w:ruby` 范围内。
pub fn para_blocks_set_text(root: &Element, target: &BodyPara) -> bool {
    let Some(p_path) = para::find_paragraph_path(root, target) else {
        return false;
    };
    collect_rubies(root)
        .iter()
        .any(|rec| path_starts_with(&rec.path, &p_path))
}

pub fn insert_ruby(
    root: &mut Element,
    after: Option<&[usize]>,
    spec: &InsertSpec<'_>,
    err_id: &str,
    op_i: usize,
) -> Result<(), Error> {
    match after {
        Some(path) => {
            let el = para::element_at_mut(root, path).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::ParaNotFound {
                        id: err_id.to_string(),
                    },
                    format!("ops[{op_i}].id"),
                )
            })?;
            el.push_element(new_ruby_run(el, spec));
            Ok(())
        }
        None => insert_at_body_end(root, spec, op_i),
    }
}

pub fn set_ruby(
    root: &mut Element,
    n: usize,
    patch: &SetPatch<'_>,
    id: &str,
    i: usize,
) -> Result<(), Error> {
    let rec = ruby_at(root, n, id, i)?;
    let ruby = para::element_at_mut(root, &rec.path).ok_or_else(|| missing_ruby(id, i))?;
    {
        let pr = ensure_ruby_pr(ruby);
        if let Some(align) = patch.align {
            set_pr_val(pr, "rubyAlign", align);
        }
        if let Some(hps) = patch.hps {
            set_pr_val(pr, "hps", &hps.to_string());
        }
        if let Some(v) = patch.hps_raise {
            set_pr_val(pr, "hpsRaise", &v.to_string());
        }
        if let Some(v) = patch.hps_base_text {
            set_pr_val(pr, "hpsBaseText", &v.to_string());
        }
        if let Some(lid) = patch.lid {
            set_pr_val(pr, "lid", lid);
        }
    }
    let rt_hps = ruby
        .child_by_local("rubyPr")
        .and_then(|pr| pr_int(pr, "hps"))
        .unwrap_or(12);
    let base_hps = ruby
        .child_by_local("rubyPr")
        .and_then(|pr| pr_int(pr, "hpsBaseText"))
        .unwrap_or(24);
    if let Some(base) = patch.base {
        replace_side(ruby, "rubyBase", base, base_hps);
    }
    if let Some(rt) = patch.rt {
        replace_side(ruby, "rt", rt, rt_hps);
    } else if patch.hps.is_some() {
        if let Some(side) = ruby.child_by_local_mut("rt") {
            stamp_side_sz(side, rt_hps);
        }
    }
    if patch.hps_base_text.is_some() && patch.base.is_none() {
        if let Some(side) = ruby.child_by_local_mut("rubyBase") {
            stamp_side_sz(side, base_hps);
        }
    }
    Ok(())
}

pub fn delete_ruby(root: &mut Element, n: usize, id: &str, i: usize) -> Result<(), Error> {
    let rec = ruby_at(root, n, id, i)?;
    let drop = drop_path(root, &rec.path);
    remove_at(root, &drop)?;
    Ok(())
}

fn insert_at_body_end(root: &mut Element, spec: &InsertSpec<'_>, op_i: usize) -> Result<(), Error> {
    let new_id = para::next_para_hex(root);
    let body = para::story_body_mut(root).ok_or_else(|| {
        Error::with_path(
            ErrorKind::XmlMissing {
                parent: "document".into(),
                child: "body".into(),
            },
            format!("ops[{op_i}].id"),
        )
    })?;
    let mut new_p = para::new_paragraph(body, &new_id, "");
    let run = new_ruby_run(&new_p, spec);
    new_p.push_element(run);
    para::append_before_body_sect_pr(body, new_p);
    Ok(())
}

fn new_ruby_run(parent: &Element, spec: &InsertSpec<'_>) -> Element {
    let mut ruby = Element::new(parent.prefixed("ruby"));
    let mut pr = Element::new(parent.prefixed("rubyPr"));
    push_pr_val(&mut pr, "rubyAlign", spec.align);
    push_pr_val(&mut pr, "hps", &spec.hps.to_string());
    push_pr_val(&mut pr, "hpsRaise", &spec.hps_raise.to_string());
    push_pr_val(&mut pr, "hpsBaseText", &spec.hps_base_text.to_string());
    push_pr_val(&mut pr, "lid", spec.lid);
    ruby.push_element(pr);
    let mut rt = Element::new(parent.prefixed("rt"));
    rt.push_element(ruby_text_run(&rt, spec.rt, spec.hps));
    ruby.push_element(rt);
    let mut base = Element::new(parent.prefixed("rubyBase"));
    base.push_element(ruby_text_run(&base, spec.base, spec.hps_base_text));
    ruby.push_element(base);
    let mut r = Element::new(parent.prefixed("r"));
    r.push_element(ruby);
    r
}

fn ruby_text_run(parent: &Element, text: &str, hps: u32) -> Element {
    let mut rpr = Element::new(parent.prefixed("rPr"));
    for local in ["sz", "szCs"] {
        let mut sz = Element::new(parent.prefixed(local));
        let key = sz.prefixed("val");
        sz.set_attr(&key, hps.to_string());
        rpr.push_element(sz);
    }
    let mut t = Element::new(parent.prefixed("t"));
    t.set_text(text.to_string());
    if needs_xml_space(text) {
        t.set_attr("xml:space", "preserve");
    }
    let mut r = Element::new(parent.prefixed("r"));
    r.push_element(rpr);
    r.push_element(t);
    r
}

fn push_pr_val(pr: &mut Element, local: &str, val: &str) {
    let mut el = Element::new(pr.prefixed(local));
    let key = el.prefixed("val");
    el.set_attr(&key, val);
    pr.push_element(el);
}

fn set_pr_val(pr: &mut Element, local: &str, val: &str) {
    let dest = super::hf::ensure_child_in_order(pr, local, RUBY_PR_ORDER);
    let key = dest.prefixed("val");
    dest.set_attr(&key, val);
}

fn ensure_ruby_pr(ruby: &mut Element) -> &mut Element {
    if ruby.child_by_local("rubyPr").is_none() {
        let pr = Element::new(ruby.prefixed("rubyPr"));
        ruby.children.insert(0, Node::Element(pr));
    }
    ruby.child_by_local_mut("rubyPr").expect("刚写入 rubyPr")
}

fn replace_side(ruby: &mut Element, local: &str, text: &str, hps: u32) {
    let side = match ruby.child_by_local_mut(local) {
        Some(s) => s,
        None => {
            let el = Element::new(ruby.prefixed(local));
            ruby.push_element(el);
            ruby.child_by_local_mut(local).expect("刚写入拼音侧")
        }
    };
    side.children.clear();
    let run = ruby_text_run(side, text, hps);
    side.push_element(run);
}

fn stamp_side_sz(side: &mut Element, hps: u32) {
    stamp_sz_walk(side, hps);
}

fn stamp_sz_walk(el: &mut Element, hps: u32) {
    if matches!(el.local_name(), "sz" | "szCs") {
        let key = el.prefixed("val");
        el.set_attr(&key, hps.to_string());
    }
    let children = el.children.as_mut_slice();
    for child in children {
        if let Node::Element(e) = child {
            stamp_sz_walk(e, hps);
        }
    }
}

fn collect_rubies(root: &Element) -> Vec<RubyRec> {
    let mut path = Vec::new();
    let mut out = Vec::new();
    para::walk_from_story_root(root, &mut path, |el, path| {
        walk_rubies(el, path, false, &mut out);
    });
    out
}

fn walk_rubies(root: &Element, path: &[usize], in_del: bool, out: &mut Vec<RubyRec>) {
    let mut stack = vec![(root, path.to_vec(), in_del)];
    while let Some((el, path, in_del)) = stack.pop() {
        if el.local_name() == "txbxContent" {
            continue;
        }
        if el.local_name() == "AlternateContent" {
            if let Some(i) = para::first_alt_child_index(el) {
                if let Node::Element(c) = &el.children[i] {
                    let mut next = path;
                    next.push(i);
                    stack.push((c, next, in_del));
                }
            }
            continue;
        }
        let in_del = in_del || el.local_name() == "del";
        if el.local_name() == "ruby" {
            if !in_del {
                out.push(RubyRec { path: path.clone() });
            }
            continue;
        }
        let mut kids = Vec::new();
        for (i, n) in el.children.iter().enumerate() {
            if let Node::Element(child) = n {
                let mut next = path.clone();
                next.push(i);
                kids.push((child, next, in_del));
            }
        }
        for item in kids.into_iter().rev() {
            stack.push(item);
        }
    }
}

fn ruby_at(root: &Element, n: usize, id: &str, i: usize) -> Result<RubyRec, Error> {
    let recs = collect_rubies(root);
    if n < 1 || n > recs.len() {
        return Err(missing_ruby(id, i));
    }
    Ok(recs.into_iter().nth(n - 1).expect("n 已检查"))
}

fn drop_path(root: &Element, ruby_path: &[usize]) -> Vec<usize> {
    if ruby_path.is_empty() {
        return ruby_path.to_vec();
    }
    let parent_path = &ruby_path[..ruby_path.len() - 1];
    let idx = *ruby_path.last().expect("path 非空");
    let Some(parent) = para::element_at(root, parent_path) else {
        return ruby_path.to_vec();
    };
    if parent.local_name() != "r" {
        return ruby_path.to_vec();
    }
    let only = parent.children.iter().enumerate().all(|(i, n)| match n {
        Node::Element(e) => e.local_name() == "rPr" || i == idx,
        _ => true,
    });
    if only {
        parent_path.to_vec()
    } else {
        ruby_path.to_vec()
    }
}

fn side_text(ruby: &Element, local: &str) -> String {
    let Some(side) = ruby.child_by_local(local) else {
        return String::new();
    };
    collect_t(side)
}

fn collect_t(el: &Element) -> String {
    let mut out = String::new();
    walk_t(el, false, &mut out);
    out
}

fn walk_t(root: &Element, in_del: bool, out: &mut String) {
    let mut stack = vec![(root, in_del)];
    while let Some((el, in_del)) = stack.pop() {
        let in_del = in_del || el.local_name() == "del";
        if el.local_name() == "t" && !in_del {
            out.push_str(&el.text_content());
            continue;
        }
        let children: Vec<&Element> = el.element_children().collect();
        for child in children.into_iter().rev() {
            stack.push((child, in_del));
        }
    }
}

fn child_val(parent: &Element, local: &str) -> Option<String> {
    parent
        .child_by_local(local)
        .and_then(|e| e.attr_local("val"))
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn child_int(parent: &Element, local: &str) -> Option<i64> {
    parent
        .child_by_local(local)
        .and_then(|e| e.attr_local("val"))
        .and_then(|s| s.parse().ok())
}

fn pr_int(pr: &Element, local: &str) -> Option<u32> {
    pr.child_by_local(local)
        .and_then(|e| e.attr_local("val"))
        .and_then(|s| s.parse().ok())
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

fn path_starts_with(path: &[usize], prefix: &[usize]) -> bool {
    path.len() >= prefix.len() && path[..prefix.len()] == *prefix
}

fn remove_at(root: &mut Element, path: &[usize]) -> Result<(), Error> {
    let (parent_path, idx) = split_parent(path)?;
    let parent = para::element_at_mut(root, parent_path).ok_or_else(|| xml_broken("ruby"))?;
    if idx >= parent.children.len() {
        return Err(xml_broken("ruby"));
    }
    parent.children.remove(idx);
    Ok(())
}

fn split_parent(path: &[usize]) -> Result<(&[usize], usize), Error> {
    let (last, prefix) = path.split_last().ok_or_else(|| xml_broken("ruby"))?;
    Ok((prefix, *last))
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

fn missing_ruby(id: &str, i: usize) -> Error {
    Error::with_path(
        ErrorKind::RubyNotFound { id: id.to_string() },
        format!("ops[{i}].id"),
    )
}

fn needs_xml_space(text: &str) -> bool {
    text.starts_with(' ')
        || text.starts_with('\t')
        || text.ends_with(' ')
        || text.ends_with('\t')
        || text.contains('\n')
}
