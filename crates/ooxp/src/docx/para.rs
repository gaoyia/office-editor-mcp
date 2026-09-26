//! 主文档里的段落：定位、可见文本、大纲级别。

use crate::error::{Error, ErrorKind, IdReason};
use crate::xml::{Element, Node};

use super::hf::{HfKind, HfType};

pub enum ParaTarget {
    /// 主文档里的段落。
    Body(BodyPara),
    /// 页眉或页脚部件里的段落。
    HeaderFooter {
        kind: HfKind,
        hf_type: HfType,
        n: u32,
        para: BodyPara,
    },
}

#[derive(Debug, Clone)]
pub enum BodyPara {
    Index(usize),
    Hex(String),
}

pub fn parse_para_id(id: &str, op_index: usize) -> Result<ParaTarget, Error> {
    let path = format!("ops[{op_index}].id");
    if let Some(rest) = id.strip_prefix("para:") {
        return Ok(ParaTarget::Body(parse_body_para(rest, &path)?));
    }
    if let Some((kind_s, rest)) = id.split_once(':') {
        if let Some(kind) = HfKind::parse(kind_s) {
            // header:default:1 或 header:default:1/para:…
            let (container, para_part) = match rest.split_once('/') {
                Some((c, p)) => (c, Some(p)),
                None => (rest, None),
            };
            let Some((type_s, n_s)) = container.split_once(':') else {
                return Err(Error::with_path(
                    ErrorKind::InvalidId {
                        reason: IdReason::ParaPrefix,
                    },
                    path,
                ));
            };
            let hf_type = HfType::parse(type_s).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::InvalidId {
                        reason: IdReason::ParaPrefix,
                    },
                    path.clone(),
                )
            })?;
            let n: u32 = n_s.parse().map_err(|_| {
                Error::with_path(
                    ErrorKind::InvalidId {
                        reason: IdReason::ParaPrefix,
                    },
                    path.clone(),
                )
            })?;
            if n < 1 {
                return Err(Error::with_path(
                    ErrorKind::InvalidId {
                        reason: IdReason::ParaPrefix,
                    },
                    path,
                ));
            }
            let Some(para_part) = para_part else {
                return Err(Error::with_path(
                    ErrorKind::InvalidId {
                        reason: IdReason::HdrFtrContainer,
                    },
                    path,
                ));
            };
            let para_rest = para_part.strip_prefix("para:").ok_or_else(|| {
                Error::with_path(
                    ErrorKind::InvalidId {
                        reason: IdReason::ParaPrefix,
                    },
                    path.clone(),
                )
            })?;
            return Ok(ParaTarget::HeaderFooter {
                kind,
                hf_type,
                n,
                para: parse_body_para(para_rest, &path)?,
            });
        }
    }
    Err(Error::with_path(
        ErrorKind::InvalidId {
            reason: IdReason::ParaPrefix,
        },
        path,
    ))
}

fn parse_body_para(rest: &str, path: &str) -> Result<BodyPara, Error> {
    if let Some(n) = rest.strip_prefix("index:") {
        let idx: usize = n.parse().map_err(|_| {
            Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::ParaIndexNotInt,
                },
                path.to_string(),
            )
        })?;
        if idx < 1 {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::ParaIndexFromOne,
                },
                path.to_string(),
            ));
        }
        return Ok(BodyPara::Index(idx));
    }
    if rest.is_empty() || rest.chars().any(|c| !c.is_ascii_hexdigit()) {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::ParaIdNotHex,
            },
            path.to_string(),
        ));
    }
    Ok(BodyPara::Hex(rest.to_string()))
}

pub fn collect_paragraphs(root: &Element) -> Vec<&Element> {
    let mut out = Vec::new();
    walk_paras(root, &mut |e| {
        if e.local_name() == "p" {
            out.push(e);
        }
    });
    out
}

/// 遇到 `w:txbxContent` 停住；`mc:AlternateContent` 只进第一个 `Choice`，没有才进 `Fallback`。
fn walk_paras<'a>(el: &'a Element, f: &mut impl FnMut(&'a Element)) {
    if el.local_name() == "txbxContent" {
        return;
    }
    if el.local_name() == "AlternateContent" {
        if let Some(i) = first_alt_child_index(el) {
            if let Node::Element(child) = &el.children[i] {
                walk_paras(child, f);
            }
        }
        return;
    }
    f(el);
    for n in &el.children {
        if let Node::Element(e) = n {
            walk_paras(e, f);
        }
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

pub fn para_stable_id(p: &Element, index: usize) -> (String, bool) {
    // `w14:paraId`：前缀跟着源文件的命名空间声明走，只能比 local name。
    if let Some(id) = p.attr_local("paraId").filter(|s| !s.is_empty()) {
        (format!("para:{id}"), true)
    } else {
        (format!("para:index:{index}"), false)
    }
}

pub fn para_stable_id_in(container_id: &str, p: &Element, index: usize) -> (String, bool) {
    let (base, stable) = para_stable_id(p, index);
    (format!("{container_id}/{base}"), stable)
}

pub fn heading_level(p: &Element) -> Option<u8> {
    let ppr = p.child_by_local("pPr")?;
    if let Some(ps) = ppr.child_by_local("pStyle") {
        if let Some(val) = ps.attr_local("val") {
            if let Some(n) = parse_heading_style(val) {
                return Some(n);
            }
        }
    }
    if let Some(ol) = ppr.child_by_local("outlineLvl") {
        if let Some(val) = ol.attr_local("val") {
            if let Ok(n) = val.parse::<u8>() {
                if n <= 8 {
                    return Some(n + 1);
                }
            }
        }
    }
    None
}

fn parse_heading_style(s: &str) -> Option<u8> {
    let lower = s.to_ascii_lowercase();
    if let Some(rest) = lower.strip_prefix("heading") {
        let n: u8 = rest.parse().ok()?;
        if (1..=9).contains(&n) {
            return Some(n);
        }
    }
    if let Some(rest) = s.strip_prefix("标题") {
        let n: u8 = rest.parse().ok()?;
        if (1..=9).contains(&n) {
            return Some(n);
        }
    }
    None
}

pub fn p_style(p: &Element) -> Option<String> {
    p.child_by_local("pPr")
        .and_then(|ppr| ppr.child_by_local("pStyle"))
        .and_then(|ps| ps.attr_local("val"))
        .map(str::to_string)
}

pub fn visible_text(p: &Element) -> String {
    let mut out = String::new();
    walk_visible(p, false, &mut out);
    out
}

fn walk_visible(el: &Element, in_del: bool, out: &mut String) {
    if el.local_name() == "txbxContent" {
        return;
    }
    if el.local_name() == "AlternateContent" {
        if let Some(i) = first_alt_child_index(el) {
            if let Node::Element(child) = &el.children[i] {
                walk_visible(child, in_del, out);
            }
        }
        return;
    }
    let in_del = in_del || el.local_name() == "del";
    if el.local_name() == "instrText" {
        return;
    }
    if !in_del {
        match el.local_name() {
            "t" => out.push_str(&el.text_content()),
            "tab" => out.push('\t'),
            "br" => out.push('\n'),
            _ => {}
        }
    }
    for n in &el.children {
        if let Node::Element(e) = n {
            walk_visible(e, in_del, out);
        }
    }
}

pub fn find_paragraph_mut<'a>(
    root: &'a mut Element,
    target: &BodyPara,
) -> Option<&'a mut Element> {
    let mut idx = 0usize;
    find_p_mut(root, &mut idx, target)
}

fn find_p_mut<'a>(
    el: &'a mut Element,
    idx: &mut usize,
    target: &BodyPara,
) -> Option<&'a mut Element> {
    if el.local_name() == "txbxContent" {
        return None;
    }
    if el.local_name() == "AlternateContent" {
        if let Some(i) = first_alt_child_index(el) {
            if let Node::Element(e) = &mut el.children[i] {
                return find_p_mut(e, idx, target);
            }
        }
        return None;
    }
    if el.local_name() == "p" {
        *idx += 1;
        let hit = match target {
            BodyPara::Index(n) => *idx == *n,
            BodyPara::Hex(hex) => el
                .attr_local("paraId")
                .is_some_and(|id| id.eq_ignore_ascii_case(hex)),
        };
        if hit {
            return Some(el);
        }
    }
    let children = el.children.as_mut_slice();
    for child in children {
        if let Node::Element(e) = child {
            if let Some(found) = find_p_mut(e, idx, target) {
                return Some(found);
            }
        }
    }
    None
}

pub fn set_visible_text(p: &mut Element, text: &str) {
    let mut first = false;
    rewrite_runs(p, &mut first, text);
    if !first {
        let run = new_text_run(p, text);
        let mut insert_at = 0;
        for (i, n) in p.children.iter().enumerate() {
            if let Node::Element(e) = n {
                if e.local_name() == "pPr" {
                    insert_at = i + 1;
                }
            }
        }
        p.children.insert(insert_at, Node::Element(run));
    }
}

fn rewrite_runs(el: &mut Element, first: &mut bool, text: &str) {
    if el.local_name() == "del" || el.local_name() == "txbxContent" {
        return;
    }
    if el.local_name() == "AlternateContent" {
        if let Some(i) = first_alt_child_index(el) {
            if let Node::Element(e) = &mut el.children[i] {
                rewrite_runs(e, first, text);
            }
        }
        return;
    }
    if el.local_name() == "r" {
        let has_t = el.element_children().any(|c| c.local_name() == "t");
        if has_t {
            if !*first {
                fill_first_run(el, text);
                *first = true;
            } else {
                remove_all_t(el);
            }
        }
        return;
    }
    let mut remove = Vec::new();
    for (i, child) in el.children.iter_mut().enumerate() {
        if let Node::Element(e) = child {
            let was_r = e.local_name() == "r";
            rewrite_runs(e, first, text);
            if was_r && is_empty_run(e) {
                remove.push(i);
            }
        }
    }
    for i in remove.into_iter().rev() {
        el.children.remove(i);
    }
}

fn fill_first_run(r: &mut Element, text: &str) {
    let mut seen_t = false;
    let mut drop_t = Vec::new();
    for (i, child) in r.children.iter_mut().enumerate() {
        if let Node::Element(e) = child {
            if e.local_name() == "t" {
                if !seen_t {
                    apply_t_text(e, text);
                    seen_t = true;
                } else {
                    drop_t.push(i);
                }
            }
        }
    }
    for i in drop_t.into_iter().rev() {
        r.children.remove(i);
    }
}

fn apply_t_text(t: &mut Element, text: &str) {
    t.set_text(text.to_string());
    if needs_xml_space(text) {
        t.set_attr("xml:space", "preserve");
    } else {
        // `xml` 是 XML 规范的保留前缀，不能重新映射，所以精确名就够了。
        t.remove_attr("xml:space");
    }
}

fn remove_all_t(r: &mut Element) {
    r.children.retain(|n| match n {
        Node::Element(e) => e.local_name() != "t",
        Node::Text(_) => true,
    });
}

fn is_empty_run(r: &Element) -> bool {
    !r.element_children().any(|e| e.local_name() != "rPr")
}

fn new_text_run(parent: &Element, text: &str) -> Element {
    let mut t = Element::new(parent.prefixed("t"));
    apply_t_text(&mut t, text);
    let mut r = Element::new(parent.prefixed("r"));
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
