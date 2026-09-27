//! 正文与页眉页脚里的 `w:fldChar` / `w:fldSimple`：inspect 名单与插改删。

use serde_json::{json, Value};

use crate::error::{Error, ErrorKind, IdReason, XmlIssue};
use crate::xml::{Element, Node};

use super::hf::HfPart;
use super::link::{self, LinkStory};
use super::para::{self, BodyPara};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldKeep {
    Result,
    Instr,
}

#[derive(Debug, Clone)]
pub struct FieldRef {
    pub story: LinkStory,
    pub n: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FieldKind {
    Simple,
    Complex,
}

struct FieldRec {
    kind: FieldKind,
    start: Vec<usize>,
    end: Vec<usize>,
    separate: Option<Vec<usize>>,
    instr: String,
    instr_text: Vec<Vec<usize>>,
    result: String,
}

pub fn count_fields(root: &Element) -> usize {
    collect_fields(root).len()
}

pub fn fields_json(root: &Element, container: Option<&str>) -> Result<Vec<Value>, Error> {
    let index_of = para_index_map(root);
    let recs = collect_fields(root);
    let mut out = Vec::new();
    for (i, rec) in recs.iter().enumerate() {
        let n = i + 1;
        let id = match container {
            Some(c) => format!("{c}/field:{n}"),
            None => format!("field:{n}"),
        };
        let kind = match rec.kind {
            FieldKind::Simple => "simple",
            FieldKind::Complex => "complex",
        };
        out.push(json!({
            "id": id,
            "index": n,
            "kind": kind,
            "code": field_code(&rec.instr),
            "instr": rec.instr,
            "host": host_para_id(root, &rec.start, container, &index_of),
            "end": host_para_id(root, &rec.end, container, &index_of),
            "text": rec.result,
        }));
    }
    Ok(out)
}

pub fn parse_field_ref(id: &str, path: &str) -> Result<FieldRef, Error> {
    let (story, rest) = link::split_story(id, path, "field", IdReason::FieldPrefix)?;
    if rest.contains('/') {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::FieldPrefix,
            },
            path.to_string(),
        ));
    }
    let n = link::parse_n(
        rest,
        path,
        IdReason::FieldIndexNotInt,
        IdReason::FieldIndexFromOne,
    )?;
    Ok(FieldRef { story, n })
}

pub fn part_for_story(
    catalog: &[HfPart],
    doc_part: &str,
    story: &LinkStory,
    missing: impl FnOnce() -> Error,
) -> Result<String, Error> {
    link::part_for_story(catalog, doc_part, story, missing)
}

/// 目标段落是否落在本期收入的域范围内。
pub fn para_blocks_set_text(root: &Element, target: &BodyPara) -> bool {
    let Some(p_path) = para::find_paragraph_path(root, target) else {
        return false;
    };
    let para_paths = collect_para_paths(root);
    let recs = collect_fields(root);
    recs.iter()
        .any(|rec| para_overlaps(root, &p_path, &rec.start, &rec.end, &para_paths))
}

pub fn insert_field(p: &mut Element, instr: &str, result: Option<&str>) {
    p.push_element(fld_char_run(p, "begin", true));
    p.push_element(instr_run(p, instr));
    p.push_element(fld_char_run(p, "separate", false));
    if let Some(text) = result {
        if !text.is_empty() {
            p.push_element(para::new_text_run(p, text));
        }
    }
    p.push_element(fld_char_run(p, "end", false));
}

pub fn set_field_instr(
    root: &mut Element,
    n: usize,
    instr: &str,
    id: &str,
    i: usize,
) -> Result<(), Error> {
    let rec = field_at(root, n, id, i)?;
    match rec.kind {
        FieldKind::Simple => {
            let start = rec.start.clone();
            let el = para::element_at_mut(root, &start).ok_or_else(|| missing_field(id, i))?;
            let key = el.prefixed("instr");
            el.set_attr(&key, instr);
        }
        FieldKind::Complex => {
            let begin = rec.start.clone();
            let begin_block = enclosing_block(root, &begin);
            let mut drop = Vec::new();
            for nested in nested_in_instruction(root, &rec) {
                drop.extend(blocks_in_closed_range(root, &nested.start, &nested.end));
            }
            let mut same_run = Vec::new();
            for p in &rec.instr_text {
                let block = enclosing_block(root, p);
                if block == begin_block {
                    same_run.push(p.clone());
                } else {
                    drop.push(block);
                }
            }
            remove_paths(root, drop)?;
            same_run.sort_by(|a, b| path_cmp(b, a));
            for p in same_run {
                if para::element_at(root, &p).is_some() {
                    remove_at(root, &p)?;
                }
            }
            let run_path = enclosing_block(root, &begin);
            let (parent_path, idx) = split_parent(&run_path)?;
            let parent =
                para::element_at_mut(root, parent_path).ok_or_else(|| xml_broken("field"))?;
            let run = instr_run(parent, instr);
            parent.children.insert(idx + 1, Node::Element(run));
        }
    }
    Ok(())
}

pub fn delete_field(
    root: &mut Element,
    n: usize,
    keep: FieldKeep,
    id: &str,
    i: usize,
) -> Result<(), Error> {
    let rec = field_at(root, n, id, i)?;
    match rec.kind {
        FieldKind::Simple => match keep {
            FieldKeep::Result => unwrap_at(root, &rec.start),
            FieldKeep::Instr => {
                let instr = rec.instr.clone();
                let (parent_path, idx) = split_parent(&rec.start)?;
                let parent =
                    para::element_at_mut(root, parent_path).ok_or_else(|| xml_broken("field"))?;
                if idx >= parent.children.len() {
                    return Err(xml_broken("field"));
                }
                let run = para::new_text_run(parent, &instr);
                parent.children[idx] = Node::Element(run);
                Ok(())
            }
        },
        FieldKind::Complex => match keep {
            FieldKeep::Result => {
                let mut drop = Vec::new();
                drop.push(enclosing_block(root, &rec.start));
                drop.push(enclosing_block(root, &rec.end));
                if let Some(sep) = &rec.separate {
                    drop.push(enclosing_block(root, sep));
                }
                for p in &rec.instr_text {
                    drop.push(enclosing_block(root, p));
                }
                for nested in nested_in_instruction(root, &rec) {
                    drop.extend(blocks_in_closed_range(root, &nested.start, &nested.end));
                }
                remove_paths(root, drop)
            }
            FieldKeep::Instr => {
                for p in rec.instr_text.clone() {
                    let el = para::element_at_mut(root, &p).ok_or_else(|| xml_broken("field"))?;
                    rename_local(el, "t");
                }
                let mut drop = Vec::new();
                drop.push(enclosing_block(root, &rec.start));
                drop.push(enclosing_block(root, &rec.end));
                if let Some(sep) = &rec.separate {
                    drop.push(enclosing_block(root, sep));
                    drop.extend(blocks_strictly_between(root, sep, &rec.end));
                }
                remove_paths(root, drop)
            }
        },
    }
}

pub fn missing_field_err(id: &str, i: usize) -> Error {
    missing_field(id, i)
}

fn field_at(root: &Element, n: usize, id: &str, i: usize) -> Result<FieldRec, Error> {
    let recs = collect_fields(root);
    if n < 1 || n > recs.len() {
        return Err(missing_field(id, i));
    }
    Ok(recs.into_iter().nth(n - 1).expect("n 已检查"))
}

fn collect_fields(root: &Element) -> Vec<FieldRec> {
    let mut path = Vec::new();
    let mut stack = Vec::new();
    let mut out = Vec::new();
    walk_fields(root, &mut path, &mut stack, &mut out);
    // 按起点文档顺序编号：嵌套域的外层 begin 早于内层，应排在前面。
    out.sort_by(|a, b| path_cmp(&a.start, &b.start));
    out
}

struct OpenField {
    start: Vec<usize>,
    separate: Option<Vec<usize>>,
    instr: String,
    instr_text: Vec<Vec<usize>>,
    result: String,
    in_result: bool,
}

fn walk_fields(
    el: &Element,
    path: &mut Vec<usize>,
    stack: &mut Vec<OpenField>,
    out: &mut Vec<FieldRec>,
) {
    if el.local_name() == "txbxContent" {
        return;
    }
    if el.local_name() == "AlternateContent" {
        if let Some(i) = first_alt(el) {
            path.push(i);
            if let Node::Element(c) = &el.children[i] {
                walk_fields(c, path, stack, out);
            }
            path.pop();
        }
        return;
    }
    match el.local_name() {
        "fldSimple" => {
            let instr = el.attr_local("instr").unwrap_or("").to_string();
            out.push(FieldRec {
                kind: FieldKind::Simple,
                start: path.clone(),
                end: path.clone(),
                separate: None,
                instr,
                instr_text: Vec::new(),
                result: para::visible_text(el),
            });
        }
        "fldChar" => match el.attr_local("fldCharType") {
            Some("begin") => stack.push(OpenField {
                start: path.clone(),
                separate: None,
                instr: String::new(),
                instr_text: Vec::new(),
                result: String::new(),
                in_result: false,
            }),
            Some("separate") => {
                if let Some(top) = stack.last_mut() {
                    top.separate = Some(path.clone());
                    top.in_result = true;
                }
            }
            Some("end") => {
                if let Some(open) = stack.pop() {
                    out.push(FieldRec {
                        kind: FieldKind::Complex,
                        start: open.start,
                        end: path.clone(),
                        separate: open.separate,
                        instr: open.instr,
                        instr_text: open.instr_text,
                        result: open.result,
                    });
                }
            }
            _ => {}
        },
        "instrText" => {
            if let Some(top) = stack.last_mut() {
                if !top.in_result {
                    top.instr.push_str(&el.text_content());
                    top.instr_text.push(path.clone());
                }
            }
        }
        "t" => {
            for open in stack.iter_mut() {
                if open.in_result {
                    open.result.push_str(&el.text_content());
                }
            }
        }
        "tab" => {
            for open in stack.iter_mut() {
                if open.in_result {
                    open.result.push('\t');
                }
            }
        }
        "br" => {
            for open in stack.iter_mut() {
                if open.in_result {
                    open.result.push('\n');
                }
            }
        }
        _ => {}
    }
    for (i, n) in el.children.iter().enumerate() {
        if let Node::Element(e) = n {
            path.push(i);
            walk_fields(e, path, stack, out);
            path.pop();
        }
    }
}

fn nested_in_instruction(root: &Element, rec: &FieldRec) -> Vec<FieldRec> {
    let limit = rec.separate.as_ref().unwrap_or(&rec.end);
    collect_fields(root)
        .into_iter()
        .filter(|n| {
            path_cmp(&n.start, &rec.start) == std::cmp::Ordering::Greater
                && path_cmp(&n.start, limit) == std::cmp::Ordering::Less
        })
        .collect()
}

fn field_code(instr: &str) -> String {
    let s = instr.trim();
    if s.starts_with('=') {
        return "=".into();
    }
    s.split_whitespace()
        .next()
        .unwrap_or("")
        .to_ascii_uppercase()
}

fn para_overlaps(
    root: &Element,
    p_path: &[usize],
    start: &[usize],
    end: &[usize],
    para_paths: &[Vec<usize>],
) -> bool {
    let Some(sp) = para_prefix_of(root, start) else {
        return false;
    };
    let Some(ep) = para_prefix_of(root, end) else {
        return false;
    };
    let Some(si) = para_paths.iter().position(|p| p == &sp) else {
        return false;
    };
    let Some(ei) = para_paths.iter().position(|p| p == &ep) else {
        return false;
    };
    let Some(pi) = para_paths.iter().position(|p| p == p_path) else {
        return false;
    };
    pi >= si && pi <= ei
}

fn collect_para_paths(root: &Element) -> Vec<Vec<usize>> {
    let mut path = Vec::new();
    let mut out = Vec::new();
    collect_p_paths(root, &mut path, &mut out);
    out
}

fn collect_p_paths(el: &Element, path: &mut Vec<usize>, out: &mut Vec<Vec<usize>>) {
    if el.local_name() == "txbxContent" {
        return;
    }
    if el.local_name() == "AlternateContent" {
        if let Some(i) = first_alt(el) {
            path.push(i);
            if let Node::Element(c) = &el.children[i] {
                collect_p_paths(c, path, out);
            }
            path.pop();
        }
        return;
    }
    if el.local_name() == "p" {
        out.push(path.clone());
    }
    for (i, n) in el.children.iter().enumerate() {
        if let Node::Element(e) = n {
            path.push(i);
            collect_p_paths(e, path, out);
            path.pop();
        }
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

fn enclosing_block(root: &Element, path: &[usize]) -> Vec<usize> {
    let mut cur = root;
    let mut acc = Vec::new();
    let mut block = path.to_vec();
    for &i in path {
        acc.push(i);
        match cur.children.get(i) {
            Some(Node::Element(e)) => {
                if matches!(e.local_name(), "r" | "fldSimple") {
                    block = acc.clone();
                }
                cur = e;
            }
            _ => break,
        }
    }
    block
}

fn collect_blocks(root: &Element) -> Vec<Vec<usize>> {
    let mut path = Vec::new();
    let mut out = Vec::new();
    walk_blocks(root, &mut path, &mut out);
    out
}

fn walk_blocks(el: &Element, path: &mut Vec<usize>, out: &mut Vec<Vec<usize>>) {
    if el.local_name() == "txbxContent" {
        return;
    }
    if el.local_name() == "AlternateContent" {
        if let Some(i) = first_alt(el) {
            path.push(i);
            if let Node::Element(c) = &el.children[i] {
                walk_blocks(c, path, out);
            }
            path.pop();
        }
        return;
    }
    if matches!(el.local_name(), "r" | "fldSimple") && !path.is_empty() {
        out.push(path.clone());
    }
    for (i, n) in el.children.iter().enumerate() {
        if let Node::Element(e) = n {
            path.push(i);
            walk_blocks(e, path, out);
            path.pop();
        }
    }
}

fn blocks_in_closed_range(root: &Element, start: &[usize], end: &[usize]) -> Vec<Vec<usize>> {
    let sb = enclosing_block(root, start);
    let eb = enclosing_block(root, end);
    collect_blocks(root)
        .into_iter()
        .filter(|b| {
            path_cmp(b, &sb) != std::cmp::Ordering::Less
                && path_cmp(b, &eb) != std::cmp::Ordering::Greater
        })
        .collect()
}

fn blocks_strictly_between(root: &Element, start: &[usize], end: &[usize]) -> Vec<Vec<usize>> {
    let sb = enclosing_block(root, start);
    let eb = enclosing_block(root, end);
    collect_blocks(root)
        .into_iter()
        .filter(|b| {
            path_cmp(b, &sb) == std::cmp::Ordering::Greater
                && path_cmp(b, &eb) == std::cmp::Ordering::Less
        })
        .collect()
}

fn path_cmp(a: &[usize], b: &[usize]) -> std::cmp::Ordering {
    let n = a.len().min(b.len());
    for i in 0..n {
        match a[i].cmp(&b[i]) {
            std::cmp::Ordering::Equal => {}
            o => return o,
        }
    }
    a.len().cmp(&b.len())
}

fn remove_paths(root: &mut Element, mut paths: Vec<Vec<usize>>) -> Result<(), Error> {
    paths.sort_by(|a, b| path_cmp(b, a));
    paths.dedup();
    for p in paths {
        if para::element_at(root, &p).is_none() {
            continue;
        }
        remove_at(root, &p)?;
    }
    Ok(())
}

fn fld_char_run(parent: &Element, kind: &str, dirty: bool) -> Element {
    let mut ch = Element::new(parent.prefixed("fldChar"));
    let ty = ch.prefixed("fldCharType");
    ch.set_attr(&ty, kind);
    if dirty {
        let d = ch.prefixed("dirty");
        ch.set_attr(&d, "true");
    }
    let mut r = Element::new(parent.prefixed("r"));
    r.push_element(ch);
    r
}

fn instr_run(parent: &Element, instr: &str) -> Element {
    let mut t = Element::new(parent.prefixed("instrText"));
    t.set_attr("xml:space", "preserve");
    t.set_text(instr.to_string());
    let mut r = Element::new(parent.prefixed("r"));
    r.push_element(t);
    r
}

fn rename_local(el: &mut Element, local: &str) {
    el.name = match el.name.split_once(':') {
        Some((pfx, _)) => format!("{pfx}:{local}"),
        None => local.to_string(),
    };
}

fn split_parent(path: &[usize]) -> Result<(&[usize], usize), Error> {
    if path.is_empty() {
        return Err(xml_broken("path"));
    }
    Ok((&path[..path.len() - 1], path[path.len() - 1]))
}

fn unwrap_at(root: &mut Element, path: &[usize]) -> Result<(), Error> {
    let (parent_path, idx) = split_parent(path)?;
    let parent = para::element_at_mut(root, parent_path).ok_or_else(|| xml_broken("field"))?;
    if idx >= parent.children.len() {
        return Err(xml_broken("field"));
    }
    let Node::Element(el) = parent.children.remove(idx) else {
        return Err(xml_broken("field"));
    };
    parent.children.splice(idx..idx, el.children);
    Ok(())
}

fn remove_at(root: &mut Element, path: &[usize]) -> Result<(), Error> {
    let (parent_path, idx) = split_parent(path)?;
    let parent = para::element_at_mut(root, parent_path).ok_or_else(|| xml_broken("field"))?;
    if idx >= parent.children.len() {
        return Err(xml_broken("field"));
    }
    parent.children.remove(idx);
    Ok(())
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

fn missing_field(id: &str, i: usize) -> Error {
    Error::with_path(
        ErrorKind::FieldNotFound { id: id.to_string() },
        format!("ops[{i}].id"),
    )
}

fn para_prefix_of(root: &Element, path: &[usize]) -> Option<Vec<usize>> {
    let mut cur = root;
    let mut acc = Vec::new();
    let mut found = None;
    for &i in path {
        acc.push(i);
        match cur.children.get(i) {
            Some(Node::Element(e)) => {
                if e.local_name() == "p" {
                    found = Some(acc.clone());
                }
                cur = e;
            }
            _ => return found,
        }
    }
    found
}
