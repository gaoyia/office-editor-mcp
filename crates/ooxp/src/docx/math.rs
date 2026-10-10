//! 正文、页眉页脚与文本框里的顶层 `m:oMath`：inspect 名单与插改删。

use serde_json::{json, Value};

use crate::error::{Error, ErrorKind, IdReason, XmlIssue};
use crate::xml::{Element, Node};

use super::hf::HfPart;
use super::link::{self, LinkStory};
use super::para::{self, BodyPara};
use super::MATH_NS;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MathKind {
    Inline,
    Display,
}

pub const MATH_JC: &[&str] = &["center", "centerGroup", "left", "right"];

impl MathKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Inline => "inline",
            Self::Display => "display",
        }
    }
}

#[derive(Debug, Clone)]
pub struct MathRef {
    pub story: LinkStory,
    pub n: usize,
}

struct MathRec {
    path: Vec<usize>,
    kind: MathKind,
}

pub fn count_maths(root: &Element) -> usize {
    collect_maths(root).len()
}

pub fn maths_json(root: &Element, container: Option<&str>) -> Result<Vec<Value>, Error> {
    let index_of = para_index_map(root);
    let recs = collect_maths(root);
    let mut out = Vec::new();
    for (i, rec) in recs.iter().enumerate() {
        let n = i + 1;
        let id = match container {
            Some(c) => format!("{c}/math:{n}"),
            None => format!("math:{n}"),
        };
        let om = para::element_at(root, &rec.path).ok_or_else(|| xml_broken("math"))?;
        let jc = read_jc(root, &rec.path)?;
        out.push(json!({
            "id": id,
            "index": n,
            "kind": rec.kind.as_str(),
            "jc": jc,
            "host": host_para_id(root, &rec.path, container, &index_of),
            "text": math_text(om),
            "expr": super::math_expr::expr_of_omath(om),
        }));
    }
    Ok(out)
}

pub fn parse_math_ref(id: &str, path: &str) -> Result<MathRef, Error> {
    let (story, rest) = link::split_story(id, path, "math", IdReason::MathPrefix)?;
    if rest.contains('/') {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::MathPrefix,
            },
            path.to_string(),
        ));
    }
    let n = link::parse_n(
        rest,
        path,
        IdReason::MathIndexNotInt,
        IdReason::MathIndexFromOne,
    )?;
    Ok(MathRef { story, n })
}

pub fn part_for_story(
    catalog: &[HfPart],
    doc_part: &str,
    story: &LinkStory,
    missing: impl FnOnce() -> Error,
) -> Result<String, Error> {
    link::part_for_story(catalog, doc_part, story, missing)
}

pub fn missing_math_err(id: &str, i: usize) -> Error {
    Error::with_path(
        ErrorKind::MathNotFound { id: id.to_string() },
        format!("ops[{i}].id"),
    )
}

/// 目标段落是否落在本期收入的顶层 `oMath` 范围内。
pub fn para_blocks_set_text(root: &Element, target: &BodyPara) -> bool {
    let Some(p_path) = para::find_paragraph_path(root, target) else {
        return false;
    };
    collect_maths(root)
        .iter()
        .any(|rec| path_starts_with(&rec.path, &p_path))
}

pub fn ensure_math_ns(root: &mut Element) {
    let key = "xmlns:m";
    if root.attr(key).is_none() {
        root.set_attr(key, MATH_NS);
    }
}

pub fn append_inline(p: &mut Element, body: &super::math_expr::MathBody) {
    p.push_element(body.to_omath());
}

pub fn insert_math(
    root: &mut Element,
    after: Option<&[usize]>,
    kind: MathKind,
    body: &super::math_expr::MathBody,
    jc: Option<&str>,
    err_id: &str,
    op_i: usize,
) -> Result<(), Error> {
    if jc.is_some() && kind == MathKind::Inline {
        return Err(Error::with_path(
            ErrorKind::MathJcOnInline,
            format!("ops[{op_i}].jc"),
        ));
    }
    match after {
        Some(path) => match kind {
            MathKind::Inline => {
                let el = para::element_at_mut(root, path).ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::ParaNotFound {
                            id: err_id.to_string(),
                        },
                        format!("ops[{op_i}].id"),
                    )
                })?;
                append_inline(el, body);
                Ok(())
            }
            MathKind::Display => insert_display_after(root, path, body, jc, err_id, op_i),
        },
        None => insert_at_body_end(root, kind, body, jc, op_i),
    }
}

pub fn set_math(
    root: &mut Element,
    n: usize,
    body: Option<&super::math_expr::MathBody>,
    jc: Option<Option<&str>>,
    id: &str,
    i: usize,
) -> Result<(), Error> {
    let rec = math_at(root, n, id, i)?;
    if jc.is_some() && rec.kind == MathKind::Inline {
        return Err(Error::with_path(
            ErrorKind::MathJcOnInline,
            format!("ops[{i}].jc"),
        ));
    }
    if let Some(body) = body {
        let om = para::element_at_mut(root, &rec.path).ok_or_else(|| missing_math(id, i))?;
        body.apply_to_omath(om);
    }
    if let Some(jc) = jc {
        set_omath_jc(root, &rec.path, jc, i)?;
    }
    Ok(())
}

pub fn delete_math(root: &mut Element, n: usize, id: &str, i: usize) -> Result<(), Error> {
    let rec = math_at(root, n, id, i)?;
    let drop = drop_path(root, &rec.path);
    if blocked_by_sect_pr(root, &drop) {
        return Err(Error::with_path(
            ErrorKind::SectionBreakPara,
            format!("ops[{i}].id"),
        ));
    }
    let host_p = enclosing_para(root, &drop);
    remove_at(root, &drop)?;
    if let Some(pp) = host_p {
        drop_empty_host_para(root, &pp)?;
    }
    Ok(())
}

fn insert_display_after(
    root: &mut Element,
    path: &[usize],
    body: &super::math_expr::MathBody,
    jc: Option<&str>,
    err_id: &str,
    op_i: usize,
) -> Result<(), Error> {
    let new_id = para::next_para_hex(root);
    let new_p = {
        let parent_path = path[..path.len().saturating_sub(1)].to_vec();
        let parent = para::element_at(root, &parent_path).ok_or_else(|| {
            Error::with_path(
                ErrorKind::ParaNotFound {
                    id: err_id.to_string(),
                },
                format!("ops[{op_i}].id"),
            )
        })?;
        let mut p = para::new_paragraph(parent, &new_id, "");
        p.push_element(body.to_omath_para_jc(jc));
        p
    };
    let (host, idx) = para::parent_and_index_mut(root, path).ok_or_else(|| {
        Error::with_path(
            ErrorKind::ParaNotFound {
                id: err_id.to_string(),
            },
            format!("ops[{op_i}].id"),
        )
    })?;
    host.children.insert(idx + 1, Node::Element(new_p));
    Ok(())
}

fn insert_at_body_end(
    root: &mut Element,
    kind: MathKind,
    math: &super::math_expr::MathBody,
    jc: Option<&str>,
    op_i: usize,
) -> Result<(), Error> {
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
    match kind {
        MathKind::Inline => new_p.push_element(math.to_omath()),
        MathKind::Display => new_p.push_element(math.to_omath_para_jc(jc)),
    }
    para::append_before_body_sect_pr(body, new_p);
    Ok(())
}

fn collect_maths(root: &Element) -> Vec<MathRec> {
    let mut path = Vec::new();
    let mut out = Vec::new();
    para::walk_from_story_root(root, &mut path, |el, path| {
        walk_maths(el, path, "", &mut out);
    });
    out
}

fn walk_maths(el: &Element, path: &mut Vec<usize>, parent: &str, out: &mut Vec<MathRec>) {
    if el.local_name() == "txbxContent" {
        return;
    }
    if el.local_name() == "AlternateContent" {
        if let Some(i) = first_alt(el) {
            path.push(i);
            if let Node::Element(c) = &el.children[i] {
                walk_maths(c, path, "AlternateContent", out);
            }
            path.pop();
        }
        return;
    }
    if el.local_name() == "oMath" {
        let kind = if parent == "oMathPara" {
            MathKind::Display
        } else {
            MathKind::Inline
        };
        out.push(MathRec {
            path: path.clone(),
            kind,
        });
        return;
    }
    let name = el.local_name();
    for (i, n) in el.children.iter().enumerate() {
        if let Node::Element(c) = n {
            path.push(i);
            walk_maths(c, path, name, out);
            path.pop();
        }
    }
}

fn math_at(root: &Element, n: usize, id: &str, i: usize) -> Result<MathRec, Error> {
    let recs = collect_maths(root);
    if n < 1 || n > recs.len() {
        return Err(missing_math(id, i));
    }
    Ok(recs.into_iter().nth(n - 1).expect("n 已检查"))
}

fn drop_path(root: &Element, omath_path: &[usize]) -> Vec<usize> {
    if omath_path.is_empty() {
        return omath_path.to_vec();
    }
    let parent_path = &omath_path[..omath_path.len() - 1];
    let Some(parent) = para::element_at(root, parent_path) else {
        return omath_path.to_vec();
    };
    if parent.local_name() != "oMathPara" {
        return omath_path.to_vec();
    }
    let remaining = parent
        .element_children()
        .filter(|e| e.local_name() == "oMath")
        .count();
    if remaining <= 1 {
        parent_path.to_vec()
    } else {
        omath_path.to_vec()
    }
}

fn blocked_by_sect_pr(root: &Element, drop: &[usize]) -> bool {
    let Some(para_path) = enclosing_para(root, drop) else {
        return false;
    };
    let Some(p) = para::element_at(root, &para_path) else {
        return false;
    };
    if !para::para_has_sect_pr(p) {
        return false;
    }
    let leftover = p.children.iter().enumerate().any(|(i, n)| {
        let Node::Element(e) = n else {
            return false;
        };
        if e.local_name() == "pPr" {
            return false;
        }
        let mut cpath = para_path.clone();
        cpath.push(i);
        !path_starts_with(drop, &cpath)
    });
    !leftover
}

fn drop_empty_host_para(root: &mut Element, para_path: &[usize]) -> Result<(), Error> {
    if para_path.is_empty() {
        return Ok(());
    }
    let parent_path = &para_path[..para_path.len() - 1];
    let parent_name = match para::element_at(root, parent_path) {
        Some(p) => p.local_name().to_string(),
        None => return Ok(()),
    };
    if parent_name == "tc" {
        return Ok(());
    }
    {
        let Some(p) = para::element_at(root, para_path) else {
            return Ok(());
        };
        if p.local_name() != "p" {
            return Ok(());
        }
        if para::para_has_sect_pr(p) || para_has_block_content(p) {
            return Ok(());
        }
    }
    remove_at(root, para_path)?;
    if parent_name == "body" {
        ensure_body_has_para(root)?;
    }
    Ok(())
}

fn ensure_body_has_para(root: &mut Element) -> Result<(), Error> {
    let hex = para::next_para_hex(root);
    let body = para::story_body_mut(root).ok_or_else(|| {
        Error::with_path(
            ErrorKind::XmlMissing {
                parent: "document".into(),
                child: "body".into(),
            },
            "document",
        )
    })?;
    if body.element_children().any(|e| e.local_name() == "p") {
        return Ok(());
    }
    let new_p = para::new_paragraph(body, &hex, "");
    para::append_before_body_sect_pr(body, new_p);
    Ok(())
}

fn para_has_block_content(p: &Element) -> bool {
    p.element_children().any(|e| e.local_name() != "pPr")
}

fn enclosing_para(root: &Element, path: &[usize]) -> Option<Vec<usize>> {
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
            _ => break,
        }
    }
    found
}

fn read_jc(root: &Element, omath_path: &[usize]) -> Result<Value, Error> {
    if omath_path.is_empty() {
        return Ok(Value::Null);
    }
    let parent_path = &omath_path[..omath_path.len() - 1];
    let Some(parent) = para::element_at(root, parent_path) else {
        return Ok(Value::Null);
    };
    if parent.local_name() != "oMathPara" {
        return Ok(Value::Null);
    }
    let Some(pr) = parent.child_by_local("oMathParaPr") else {
        return Ok(Value::Null);
    };
    let Some(jc) = pr.child_by_local("jc") else {
        return Ok(Value::Null);
    };
    let Some(raw) = jc.attr_local("val") else {
        return Ok(Value::Null);
    };
    if !MATH_JC.contains(&raw) {
        return Err(Error::xml_detail(
            XmlIssue::Parse,
            format!("m:jc/@m:val is not a known justification: {raw}"),
        ));
    }
    Ok(json!(raw))
}

fn set_omath_jc(
    root: &mut Element,
    omath_path: &[usize],
    jc: Option<&str>,
    i: usize,
) -> Result<(), Error> {
    if omath_path.is_empty() {
        return Err(Error::with_path(
            ErrorKind::MathJcOnInline,
            format!("ops[{i}].jc"),
        ));
    }
    let parent_path = &omath_path[..omath_path.len() - 1];
    let parent = para::element_at_mut(root, parent_path)
        .ok_or_else(|| Error::with_path(ErrorKind::MathJcOnInline, format!("ops[{i}].jc")))?;
    if parent.local_name() != "oMathPara" {
        return Err(Error::with_path(
            ErrorKind::MathJcOnInline,
            format!("ops[{i}].jc"),
        ));
    }
    match jc {
        Some(val) => {
            if parent.child_by_local("oMathParaPr").is_none() {
                parent
                    .children
                    .insert(0, Node::Element(Element::new("m:oMathParaPr")));
            }
            let pr = parent.child_by_local_mut("oMathParaPr").expect("刚补过");
            pr.children.retain(|n| match n {
                Node::Element(e) => e.local_name() != "jc",
                _ => true,
            });
            let mut jc_el = Element::new("m:jc");
            jc_el.set_attr("m:val", val);
            pr.children.insert(0, Node::Element(jc_el));
        }
        None => {
            let empty = {
                let Some(pr) = parent.child_by_local_mut("oMathParaPr") else {
                    return Ok(());
                };
                pr.children.retain(|n| match n {
                    Node::Element(e) => e.local_name() != "jc",
                    _ => true,
                });
                pr.element_children().next().is_none()
            };
            if empty {
                parent.children.retain(|n| match n {
                    Node::Element(e) => e.local_name() != "oMathParaPr",
                    _ => true,
                });
            }
        }
    }
    Ok(())
}

pub(crate) fn math_text(el: &Element) -> String {
    let mut out = String::new();
    collect_t(el, &mut out);
    out
}

fn collect_t(el: &Element, out: &mut String) {
    if el.local_name() == "t" {
        out.push_str(&el.text_content());
        return;
    }
    for c in el.element_children() {
        collect_t(c, out);
    }
}

pub(crate) fn new_omath(text: &str) -> Element {
    let mut om = Element::new("m:oMath");
    om.push_element(new_math_run(text));
    om
}

pub(crate) fn math_run_text(el: &Element) -> String {
    math_text(el)
}

pub(crate) fn new_math_run(text: &str) -> Element {
    new_math_run_brk(text, false)
}

pub(crate) fn new_math_run_brk(text: &str, brk: bool) -> Element {
    let mut t = Element::new("m:t");
    if needs_xml_space(text) {
        t.set_attr("xml:space", "preserve");
    }
    t.set_text(text);
    let mut r = Element::new("m:r");
    if brk {
        let mut rpr = Element::new("m:rPr");
        rpr.push_element(Element::new("m:brk"));
        r.push_element(rpr);
    }
    r.push_element(t);
    r
}

fn needs_xml_space(text: &str) -> bool {
    text.starts_with(' ')
        || text.starts_with('\t')
        || text.ends_with(' ')
        || text.ends_with('\t')
        || text.contains('\n')
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

fn path_starts_with(path: &[usize], prefix: &[usize]) -> bool {
    path.len() >= prefix.len() && path[..prefix.len()] == *prefix
}

fn remove_at(root: &mut Element, path: &[usize]) -> Result<(), Error> {
    let (parent_path, idx) = split_parent(path)?;
    let parent = para::element_at_mut(root, parent_path).ok_or_else(|| xml_broken("math"))?;
    if idx >= parent.children.len() {
        return Err(xml_broken("math"));
    }
    parent.children.remove(idx);
    Ok(())
}

fn split_parent(path: &[usize]) -> Result<(&[usize], usize), Error> {
    let (last, prefix) = path.split_last().ok_or_else(|| xml_broken("math"))?;
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

fn missing_math(id: &str, i: usize) -> Error {
    Error::with_path(
        ErrorKind::MathNotFound { id: id.to_string() },
        format!("ops[{i}].id"),
    )
}
