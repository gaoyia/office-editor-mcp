//! 从主文档节属性收集页眉、页脚部件。

use std::collections::HashMap;

use crate::error::{Error, XmlIssue};
use crate::opc::{self, Package, Relationships};
use crate::xml::{Element, Node};

use super::WML_NS;

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

/// 一节上三种类型的链接状态：`Some(true)` 继承，`Some(false)` 显式，`None` 不生效。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HfLinkTriple {
    pub default: Option<bool>,
    pub even: Option<bool>,
    pub first: Option<bool>,
}

/// 一节的页眉、页脚链接状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SectionLinkState {
    pub header: HfLinkTriple,
    pub footer: HfLinkTriple,
}

/// 按节给出 `headerLink` / `footerLink`。
pub fn section_link_states(
    pkg: &Package,
    doc_part: &str,
    doc_root: &Element,
) -> Result<Vec<SectionLinkState>, Error> {
    let rels = Relationships::from_package(pkg, doc_part)?;
    let even_and_odd = read_even_and_odd(pkg, doc_part, &rels)?;
    let sects = collect_sect_prs(doc_root);
    let mut out = Vec::with_capacity(sects.len());
    for (i, sect) in sects.iter().enumerate() {
        let sec_num = (i + 1) as u32;
        let title_pg = bool_prop(sect, "titlePg");
        let eh = explicit_refs(sect, "headerReference", HfKind::Header, doc_part, &rels)?;
        let ef = explicit_refs(sect, "footerReference", HfKind::Footer, doc_part, &rels)?;
        out.push(SectionLinkState {
            header: link_triple(sec_num, &eh, title_pg, even_and_odd),
            footer: link_triple(sec_num, &ef, title_pg, even_and_odd),
        });
    }
    Ok(out)
}

fn link_triple(
    sec_num: u32,
    explicit: &HashMap<HfType, String>,
    title_pg: bool,
    even_and_odd: bool,
) -> HfLinkTriple {
    HfLinkTriple {
        default: link_flag(
            sec_num,
            HfType::Default,
            explicit.contains_key(&HfType::Default),
            title_pg,
            even_and_odd,
        ),
        even: link_flag(
            sec_num,
            HfType::Even,
            explicit.contains_key(&HfType::Even),
            title_pg,
            even_and_odd,
        ),
        first: link_flag(
            sec_num,
            HfType::First,
            explicit.contains_key(&HfType::First),
            title_pg,
            even_and_odd,
        ),
    }
}

fn link_flag(
    sec_num: u32,
    hf_type: HfType,
    has_explicit: bool,
    title_pg: bool,
    even_and_odd: bool,
) -> Option<bool> {
    if !type_is_active(hf_type, title_pg, even_and_odd) {
        return None;
    }
    if has_explicit {
        return Some(false);
    }
    if sec_num == 1 {
        return Some(false);
    }
    Some(true)
}

pub fn type_is_active(hf_type: HfType, title_pg: bool, even_and_odd: bool) -> bool {
    match hf_type {
        HfType::Default => true,
        HfType::Even => even_and_odd,
        HfType::First => title_pg,
    }
}

pub fn even_and_odd_headers(pkg: &Package, doc_part: &str) -> Result<bool, Error> {
    let rels = Relationships::from_package(pkg, doc_part)?;
    read_even_and_odd(pkg, doc_part, &rels)
}

pub fn has_explicit_ref(sect: &Element, kind: HfKind, hf_type: HfType) -> bool {
    let local = ref_local(kind);
    sect.element_children().any(|c| {
        c.local_name() == local
            && HfType::parse(c.attr_local("type").unwrap_or("")) == Some(hf_type)
    })
}

/// 从第 1 节走到 `section_n` 之前，取出该类型最后一次显式引用的部件。
pub fn inherited_part_path(
    pkg: &Package,
    doc_part: &str,
    root: &Element,
    section_n: usize,
    kind: HfKind,
    hf_type: HfType,
) -> Result<Option<String>, Error> {
    let rels = Relationships::from_package(pkg, doc_part)?;
    let sects = collect_sect_prs(root);
    let local = ref_local(kind);
    let mut prev: Option<String> = None;
    for sect in sects.iter().take(section_n.saturating_sub(1)) {
        let map = explicit_refs(sect, local, kind, doc_part, &rels)?;
        if let Some(p) = map.get(&hf_type) {
            prev = Some(p.clone());
        }
    }
    Ok(prev)
}

/// 本节该类型显式引用的部件路径。
pub fn explicit_part_path(
    pkg: &Package,
    doc_part: &str,
    root: &Element,
    section_n: usize,
    kind: HfKind,
    hf_type: HfType,
) -> Result<Option<String>, Error> {
    let rels = Relationships::from_package(pkg, doc_part)?;
    let sects = collect_sect_prs(root);
    let Some(sect) = sects.get(section_n.saturating_sub(1)) else {
        return Ok(None);
    };
    let map = explicit_refs(sect, ref_local(kind), kind, doc_part, &rels)?;
    Ok(map.get(&hf_type).cloned())
}

pub fn next_unused_hf_part(pkg: &Package, kind: HfKind) -> String {
    let stem = match kind {
        HfKind::Header => "header",
        HfKind::Footer => "footer",
    };
    let mut n = 1u32;
    loop {
        let name = format!("/word/{stem}{n}.xml");
        if !pkg.contains(&name) {
            return name;
        }
        n += 1;
    }
}

pub fn minimal_hf_xml(kind: HfKind) -> Vec<u8> {
    let tag = match kind {
        HfKind::Header => "hdr",
        HfKind::Footer => "ftr",
    };
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:{tag} xmlns:w="{WML_NS}"><w:p/></w:{tag}>"#
    )
    .into_bytes()
}

/// 源部件与目标部件同目录时写相对文件名，否则写去掉前导 `/` 的路径。
pub fn same_dir_rel_target(source_part: &str, target_opc: &str) -> String {
    let src_file = source_part.rsplit('/').next().unwrap_or(source_part);
    let src_dir = source_part.strip_suffix(src_file).unwrap_or("/");
    let tgt_file = target_opc.rsplit('/').next().unwrap_or(target_opc);
    let tgt_dir = target_opc.strip_suffix(tgt_file).unwrap_or("/");
    if src_dir == tgt_dir {
        tgt_file.to_string()
    } else {
        target_opc.trim_start_matches('/').to_string()
    }
}

pub fn rid_attr_name(sect: &Element) -> String {
    for child in sect.element_children() {
        let loc = child.local_name();
        if loc != "headerReference" && loc != "footerReference" {
            continue;
        }
        if let Some((k, _)) = child
            .attrs
            .iter()
            .find(|(k, _)| k.rsplit(':').next().unwrap_or(k) == "id")
        {
            return k.clone();
        }
    }
    "r:id".into()
}

pub fn insert_hf_ref(sect: &mut Element, kind: HfKind, hf_type: HfType, rid_attr: &str, rid: &str) {
    let local = ref_local(kind);
    let mut el = Element::new(sect.prefixed(local));
    el.set_attr(&sect.prefixed("type"), hf_type.as_str());
    el.set_attr(rid_attr, rid);
    let mut last_ref = None;
    for (i, n) in sect.children.iter().enumerate() {
        if let Node::Element(e) = n {
            if e.local_name() == "headerReference" || e.local_name() == "footerReference" {
                last_ref = Some(i);
            }
        }
    }
    let idx = last_ref.map(|i| i + 1).unwrap_or(0);
    sect.children.insert(idx, Node::Element(el));
}

pub fn remove_hf_ref(sect: &mut Element, kind: HfKind, hf_type: HfType) {
    let local = ref_local(kind);
    sect.children.retain(|n| match n {
        Node::Element(e) if e.local_name() == local => {
            HfType::parse(e.attr_local("type").unwrap_or("")) != Some(hf_type)
        }
        _ => true,
    });
}

fn ref_local(kind: HfKind) -> &'static str {
    match kind {
        HfKind::Header => "headerReference",
        HfKind::Footer => "footerReference",
    }
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
        let type_raw = child
            .attr_local("type")
            .ok_or_else(|| Error::xml_detail(XmlIssue::Parse, format!("{local} missing w:type")))?;
        let hf_type = HfType::parse(type_raw).ok_or_else(|| {
            Error::xml_detail(XmlIssue::Parse, format!("{local} bad w:type: {type_raw}"))
        })?;
        let rid = child
            .attr_local("id")
            .ok_or_else(|| Error::xml_detail(XmlIssue::Parse, format!("{local} missing r:id")))?;
        let rel = rels
            .find_by_id(rid)
            .ok_or_else(|| Error::xml_detail(XmlIssue::Parse, format!("dangling r:id {rid}")))?;
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
        let part = opc::resolve_rel_target(doc_part, &rel.target)
            .map_err(|_| Error::xml_detail(XmlIssue::Parse, format!("bad rel target for {rid}")))?;
        map.entry(hf_type).or_insert(part);
    }
    Ok(map)
}

pub(crate) fn bool_prop(el: &Element, local: &str) -> bool {
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
    let Some(rel) = doc_rels.find_by_type_suffix("/settings").into_iter().next() else {
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

pub fn collect_sect_prs(root: &Element) -> Vec<&Element> {
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

/// 从 1 起找第 `n` 个 `w:sectPr`。
pub fn find_sect_pr_mut(root: &mut Element, n: usize) -> Option<&mut Element> {
    let mut idx = 0usize;
    find_sect_mut(root, &mut idx, n)
}

fn find_sect_mut<'a>(el: &'a mut Element, idx: &mut usize, n: usize) -> Option<&'a mut Element> {
    if el.local_name() == "txbxContent" {
        return None;
    }
    if el.local_name() == "AlternateContent" {
        if let Some(i) = first_alt_child_index(el) {
            if let crate::xml::Node::Element(e) = &mut el.children[i] {
                return find_sect_mut(e, idx, n);
            }
        }
        return None;
    }
    if el.local_name() == "sectPr" {
        *idx += 1;
        if *idx == n {
            return Some(el);
        }
        return None;
    }
    let children = el.children.as_mut_slice();
    for child in children {
        if let crate::xml::Node::Element(e) = child {
            if let Some(found) = find_sect_mut(e, idx, n) {
                return Some(found);
            }
        }
    }
    None
}

/// 读 `w:pgMar` 上的 twip 属性；缺则为 `None`。
pub fn twip_attr(pg_mar: Option<&Element>, local: &str) -> Result<Option<i64>, Error> {
    let Some(el) = pg_mar else {
        return Ok(None);
    };
    let Some(raw) = el.attr_local(local) else {
        return Ok(None);
    };
    let t = raw.trim();
    let n: i64 = t.parse().map_err(|_| {
        Error::xml_detail(
            XmlIssue::Parse,
            format!("pgMar/@{local} not an integer: {raw}"),
        )
    })?;
    if n < 0 {
        return Err(Error::xml_detail(
            XmlIssue::Parse,
            format!("pgMar/@{local} must be non-negative: {raw}"),
        ));
    }
    Ok(Some(n))
}

/// 按 local name 写 twip 属性；已有则改原名，没有则用节前缀补上。
pub fn set_twip_attr(el: &mut Element, local: &str, value: u32) {
    if let Some((_, v)) = el
        .attrs
        .iter_mut()
        .find(|(k, _)| k.rsplit(':').next().unwrap_or(k) == local)
    {
        *v = value.to_string();
        return;
    }
    el.set_attr(&el.prefixed(local), value.to_string());
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
