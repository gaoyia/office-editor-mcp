//! 幻灯片里的文本形状：定位、可见文字、AlternateContent 只走一支。

use crate::error::{Error, ErrorKind, IdReason};
use crate::opc::{self, Package};
use crate::xml::{Element, Node};

pub struct SlideMeta {
    pub index: usize,
    pub part: String,
    pub name: Option<String>,
    pub hidden: bool,
}

pub enum ShapeKey {
    Id(String),
    Index(usize),
}

pub struct ShapeTarget {
    pub slide: usize,
    pub key: ShapeKey,
}

pub fn parse_shape_id(id: &str, op_index: usize) -> Result<ShapeTarget, Error> {
    let path = format!("ops[{op_index}].id");
    let rest = id.strip_prefix("slide:").ok_or_else(|| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::ShapePrefix,
            },
            path.clone(),
        )
    })?;
    let (n, after) = rest.split_once("/shape:").ok_or_else(|| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::ShapeForm,
            },
            path.clone(),
        )
    })?;
    let slide: usize = n.parse().map_err(|_| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SlideIndexNotInt,
            },
            path.clone(),
        )
    })?;
    if slide < 1 {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SlideIndexFromOne,
            },
            path,
        ));
    }
    if after.is_empty() {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::ShapeEmpty,
            },
            path,
        ));
    }
    if let Some(k) = after.strip_prefix("index:") {
        let idx: usize = k.parse().map_err(|_| {
            Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::ShapeIndexNotInt,
                },
                path.clone(),
            )
        })?;
        if idx < 1 {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::ShapeIndexFromOne,
                },
                path,
            ));
        }
        return Ok(ShapeTarget {
            slide,
            key: ShapeKey::Index(idx),
        });
    }
    Ok(ShapeTarget {
        slide,
        key: ShapeKey::Id(after.to_string()),
    })
}

/// `r:id` / `rel:id`，不要和 `p:sldId/@id` 弄混。后者的本地名也是 id。
pub fn rid_attr(el: &Element) -> Option<&str> {
    el.attrs.iter().find_map(|(k, v)| {
        if k.contains(':') && k.rsplit(':').next() == Some("id") {
            Some(v.as_str())
        } else {
            None
        }
    })
}

pub fn list_slides(
    pkg: &Package,
    pres_part: &str,
    pres_root: &Element,
) -> Result<Vec<SlideMeta>, Error> {
    let Some(lst) = pres_root.child_by_local("sldIdLst") else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for (i, sld_id) in lst
        .element_children()
        .filter(|e| e.local_name() == "sldId")
        .enumerate()
    {
        let rid = rid_attr(sld_id).ok_or_else(|| {
            Error::new(ErrorKind::SlideRelMissing { index: i + 1 })
        })?;
        let part = opc::resolve_id(pkg, pres_part, rid)?;
        let doc = crate::xml::parse(pkg.get(&part)?)?;
        let name = doc
            .root
            .child_by_local("cSld")
            .and_then(|c| c.attr("name"))
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        let hidden = match doc.root.attr("show") {
            Some("0") | Some("false") | Some("FALSE") => true,
            _ => false,
        };
        out.push(SlideMeta {
            index: i + 1,
            part,
            name,
            hidden,
        });
    }
    Ok(out)
}

pub fn collect_text_shapes(root: &Element) -> Vec<&Element> {
    let mut out = Vec::new();
    walk_choice(root, &mut |e| {
        if e.local_name() == "sp" && e.child_by_local("txBody").is_some() {
            out.push(e);
        }
    });
    out
}

pub fn cnv_pr(sp: &Element) -> Option<&Element> {
    sp.child_by_local("nvSpPr")
        .and_then(|nv| nv.child_by_local("cNvPr"))
}

pub fn shape_stable_id(sp: &Element, slide: usize, index: usize) -> (String, Option<String>, bool) {
    if let Some(id) = cnv_pr(sp)
        .and_then(|c| c.attr("id"))
        .filter(|s| !s.is_empty())
    {
        (
            format!("slide:{slide}/shape:{id}"),
            Some(id.to_string()),
            true,
        )
    } else {
        (format!("slide:{slide}/shape:index:{index}"), None, false)
    }
}

pub fn shape_name(sp: &Element) -> Option<String> {
    cnv_pr(sp)
        .and_then(|c| c.attr("name"))
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

pub fn visible_text(sp: &Element) -> String {
    let Some(tx) = sp.child_by_local("txBody") else {
        return String::new();
    };
    let mut out = String::new();
    walk_choice(tx, &mut |e| match e.local_name() {
        "t" => out.push_str(&e.text_content()),
        "br" => out.push('\n'),
        _ => {}
    });
    out
}

pub fn find_shape_mut<'a>(root: &'a mut Element, target: &ShapeKey) -> Option<&'a mut Element> {
    let mut idx = 0usize;
    find_sp_mut(root, &mut idx, target)
}

fn find_sp_mut<'a>(
    el: &'a mut Element,
    idx: &mut usize,
    target: &ShapeKey,
) -> Option<&'a mut Element> {
    if el.local_name() == "AlternateContent" {
        if let Some(i) = first_alt_child_index(el) {
            if let Node::Element(child) = &mut el.children[i] {
                return find_sp_mut(child, idx, target);
            }
        }
        return None;
    }
    if el.local_name() == "sp" && el.child_by_local("txBody").is_some() {
        *idx += 1;
        let hit = match target {
            ShapeKey::Index(n) => *idx == *n,
            ShapeKey::Id(req) => cnv_pr(el)
                .and_then(|c| c.attr("id"))
                .is_some_and(|id| ids_match(id, req)),
        };
        if hit {
            return Some(el);
        }
        return None;
    }
    for child in el.children.as_mut_slice() {
        if let Node::Element(e) = child {
            if let Some(found) = find_sp_mut(e, idx, target) {
                return Some(found);
            }
        }
    }
    None
}

fn ids_match(file: &str, req: &str) -> bool {
    match (file.parse::<u64>(), req.parse::<u64>()) {
        (Ok(a), Ok(b)) => a == b,
        _ => file == req,
    }
}

pub fn set_txbody_text(tx: &mut Element, text: &str) {
    let ppr = first_named_clone(tx, "pPr");
    let rpr = first_named_clone(tx, "rPr");
    tx.children.retain(|n| match n {
        Node::Element(e) => {
            let name = e.local_name();
            name != "p" && name != "AlternateContent"
        }
        Node::Text(_) => true,
    });
    tx.push_element(build_para(ppr, rpr, text));
}

fn first_named_clone(root: &Element, local: &str) -> Option<Element> {
    let mut found = None;
    walk_choice(root, &mut |e| {
        if found.is_none() && e.local_name() == local {
            found = Some(e.clone());
        }
    });
    found
}

fn build_para(ppr: Option<Element>, rpr: Option<Element>, text: &str) -> Element {
    let mut p = Element::new("a:p");
    if let Some(pr) = ppr {
        p.push_element(pr);
    }
    let lines: Vec<&str> = text.split('\n').collect();
    for (i, line) in lines.iter().enumerate() {
        if i > 0 {
            p.push_element(Element::new("a:br"));
        }
        let mut r = Element::new("a:r");
        if let Some(pr) = &rpr {
            r.push_element(pr.clone());
        }
        let mut t = Element::new("a:t");
        apply_t_text(&mut t, line);
        r.push_element(t);
        p.push_element(r);
    }
    p
}

fn apply_t_text(t: &mut Element, text: &str) {
    t.set_text(text.to_string());
    if needs_xml_space(text) {
        t.set_attr("xml:space", "preserve");
    } else {
        t.remove_attr("space");
        t.remove_attr("xml:space");
    }
}

fn needs_xml_space(text: &str) -> bool {
    text.starts_with(' ') || text.starts_with('\t') || text.ends_with(' ') || text.ends_with('\t')
}

/// 遇到 `mc:AlternateContent` 只进第一个 `Choice`，没有才进 `Fallback`。
pub fn walk_choice<'a>(el: &'a Element, f: &mut impl FnMut(&'a Element)) {
    if el.local_name() == "AlternateContent" {
        if let Some(i) = first_alt_child_index(el) {
            if let Node::Element(child) = &el.children[i] {
                walk_choice(child, f);
            }
        }
        return;
    }
    f(el);
    for n in &el.children {
        if let Node::Element(e) = n {
            walk_choice(e, f);
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
