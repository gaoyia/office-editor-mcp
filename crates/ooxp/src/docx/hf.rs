//! 从主文档节属性收集页眉、页脚部件。

use std::collections::HashMap;

use crate::error::{Error, XmlIssue};
use crate::opc::{self, Package, Relationships};
use crate::xml::Element;

/// 页眉或页脚。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HfKind {
    Header,
    Footer,
}

impl HfKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Header => "header",
            Self::Footer => "footer",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "header" => Some(Self::Header),
            "footer" => Some(Self::Footer),
            _ => None,
        }
    }
}

/// `ST_HdrFtr`：default / even / first。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HfType {
    Default,
    Even,
    First,
}

impl HfType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Even => "even",
            Self::First => "first",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "default" => Some(Self::Default),
            "even" => Some(Self::Even),
            "first" => Some(Self::First),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct HfPart {
    pub kind: HfKind,
    pub hf_type: HfType,
    pub part: String,
    pub sections: Vec<u32>,
    /// 如 `header:default:1`
    pub id: String,
}

/// 按文档顺序收集唯一页眉、页脚部件，并赋稳定 id。
pub fn catalog(pkg: &Package, doc_part: &str, doc_root: &Element) -> Result<Vec<HfPart>, Error> {
    let rels = Relationships::from_package(pkg, doc_part)?;
    let even_and_odd = read_even_and_odd(pkg, doc_part, &rels)?;
    let sects = collect_sect_prs(doc_root);

    let mut prev_header: [Option<String>; 3] = [None, None, None];
    let mut prev_footer: [Option<String>; 3] = [None, None, None];
    let mut by_part: HashMap<String, usize> = HashMap::new();
    let mut ordered: Vec<HfPart> = Vec::new();
    let mut counters: HashMap<(HfKind, HfType), u32> = HashMap::new();

    for (i, sect) in sects.iter().enumerate() {
        let sec_num = (i + 1) as u32;
        let title_pg = bool_prop(sect, "titlePg");
        let explicit_h = explicit_refs(sect, "headerReference", HfKind::Header, doc_part, &rels)?;
        let explicit_f = explicit_refs(sect, "footerReference", HfKind::Footer, doc_part, &rels)?;

        resolve_section(
            HfKind::Header,
            sec_num,
            &explicit_h,
            &mut prev_header,
            title_pg,
            even_and_odd,
            &mut by_part,
            &mut ordered,
            &mut counters,
        );
        resolve_section(
            HfKind::Footer,
            sec_num,
            &explicit_f,
            &mut prev_footer,
            title_pg,
            even_and_odd,
            &mut by_part,
            &mut ordered,
            &mut counters,
        );
    }

    Ok(ordered)
}

/// 按容器 id（如 `header:default:1`）在目录里查找。
pub fn find_by_id<'a>(items: &'a [HfPart], id: &str) -> Option<&'a HfPart> {
    items.iter().find(|h| h.id == id)
}

fn resolve_section(
    kind: HfKind,
    sec_num: u32,
    explicit: &HashMap<HfType, String>,
    prev: &mut [Option<String>; 3],
    title_pg: bool,
    even_and_odd: bool,
    by_part: &mut HashMap<String, usize>,
    ordered: &mut Vec<HfPart>,
    counters: &mut HashMap<(HfKind, HfType), u32>,
) {
    for hf_type in [HfType::Default, HfType::Even, HfType::First] {
        let idx = type_index(hf_type);
        let part = if let Some(p) = explicit.get(&hf_type) {
            prev[idx] = Some(p.clone());
            Some(p.clone())
        } else {
            let inherit = match hf_type {
                HfType::Default => true,
                HfType::Even => even_and_odd,
                HfType::First => title_pg,
            };
            if inherit {
                prev[idx].clone()
            } else {
                None
            }
        };
        let Some(part) = part else {
            continue;
        };
        if let Some(&i) = by_part.get(&part) {
            if !ordered[i].sections.contains(&sec_num) {
                ordered[i].sections.push(sec_num);
            }
        } else {
            let n = counters.entry((kind, hf_type)).or_insert(0);
            *n += 1;
            let id = format!("{}:{}:{}", kind.as_str(), hf_type.as_str(), *n);
            by_part.insert(part.clone(), ordered.len());
            ordered.push(HfPart {
                kind,
                hf_type,
                part,
                sections: vec![sec_num],
                id,
            });
        }
    }
}

fn type_index(t: HfType) -> usize {
    match t {
        HfType::Default => 0,
        HfType::Even => 1,
        HfType::First => 2,
    }
}

fn explicit_refs(
    sect: &Element,
    local: &str,
    kind: HfKind,
    doc_part: &str,
    rels: &Relationships,
) -> Result<HashMap<HfType, String>, Error> {
    let mut map = HashMap::new();
    for child in sect.element_children() {
        if child.local_name() != local {
            continue;
        }
        let type_raw = child.attr_local("type").ok_or_else(|| {
            Error::xml_detail(XmlIssue::Parse, format!("{local} missing w:type"))
        })?;
        let hf_type = HfType::parse(type_raw).ok_or_else(|| {
            Error::xml_detail(
                XmlIssue::Parse,
                format!("{local} bad w:type: {type_raw}"),
            )
        })?;
        let rid = child.attr_local("id").ok_or_else(|| {
            Error::xml_detail(XmlIssue::Parse, format!("{local} missing r:id"))
        })?;
        let rel = rels.find_by_id(rid).ok_or_else(|| {
            Error::xml_detail(XmlIssue::Parse, format!("dangling r:id {rid}"))
        })?;
        if rel.is_external() {
            return Err(Error::xml_detail(
                XmlIssue::Parse,
                format!("external r:id {rid}"),
            ));
        }
        let suffix = match kind {
            HfKind::Header => "/header",
            HfKind::Footer => "/footer",
        };
        if !rel.rel_type.ends_with(suffix) {
            return Err(Error::xml_detail(
                XmlIssue::Parse,
                format!("r:id {rid} is not a {suffix} relationship"),
            ));
        }
        let part = opc::resolve_rel_target(doc_part, &rel.target).map_err(|_| {
            Error::xml_detail(XmlIssue::Parse, format!("bad rel target for {rid}"))
        })?;
        map.entry(hf_type).or_insert(part);
    }
    Ok(map)
}

fn bool_prop(el: &Element, local: &str) -> bool {
    let Some(child) = el.child_by_local(local) else {
        return false;
    };
    match child.attr_local("val") {
        None => true,
        Some(v) => {
            let lower = v.to_ascii_lowercase();
            matches!(lower.as_str(), "1" | "true" | "on")
        }
    }
}

fn read_even_and_odd(
    pkg: &Package,
    doc_part: &str,
    doc_rels: &Relationships,
) -> Result<bool, Error> {
    let Some(rel) = doc_rels
        .find_by_type_suffix("/settings")
        .into_iter()
        .next()
    else {
        return Ok(false);
    };
    if rel.is_external() {
        return Ok(false);
    }
    let settings_part = match opc::resolve_rel_target(doc_part, &rel.target) {
        Ok(p) => p,
        Err(_) => return Ok(false),
    };
    let Ok(bytes) = pkg.get(&settings_part) else {
        return Ok(false);
    };
    let doc = crate::xml::parse(bytes)?;
    Ok(bool_prop(&doc.root, "evenAndOddHeaders"))
}

fn collect_sect_prs(root: &Element) -> Vec<&Element> {
    let mut out = Vec::new();
    walk_sect(root, &mut |e| {
        if e.local_name() == "sectPr" {
            out.push(e);
        }
    });
    out
}

/// 遇到 `w:txbxContent` 停住；`mc:AlternateContent` 只进一支。
fn walk_sect<'a>(el: &'a Element, f: &mut impl FnMut(&'a Element)) {
    if el.local_name() == "txbxContent" {
        return;
    }
    if el.local_name() == "AlternateContent" {
        if let Some(i) = first_alt_child_index(el) {
            if let crate::xml::Node::Element(child) = &el.children[i] {
                walk_sect(child, f);
            }
        }
        return;
    }
    f(el);
    for n in &el.children {
        if let crate::xml::Node::Element(e) = n {
            walk_sect(e, f);
        }
    }
}

fn first_alt_child_index(el: &Element) -> Option<usize> {
    let mut fallback = None;
    for (i, n) in el.children.iter().enumerate() {
        if let crate::xml::Node::Element(e) = n {
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
