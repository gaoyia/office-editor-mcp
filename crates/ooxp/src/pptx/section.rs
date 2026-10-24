//! 幻灯片节：`p14:sectionLst` 分组元数据。页顺序仍以 `p:sldIdLst` 为准。

use serde_json::{json, Map, Value};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::error::{Error, ErrorKind, IdReason};
use crate::opc::Package;
use crate::xml::{Document, Element, Node};

use super::request::{
    DeleteSlideSectionOp, InsertSlideSectionOp, SetSlideSectionNameOp, SetSlideSectionOp,
};

pub const P14_NS: &str = "http://schemas.microsoft.com/office/powerpoint/2010/main";
pub const SECTION_EXT_URI: &str = "{521415D9-36F7-43E2-AB2F-B90AF26B5E84}";

static GUID_SEQ: AtomicU64 = AtomicU64::new(1);

pub struct SectionInfo {
    pub index: usize,
    pub guid: Option<String>,
    pub name: String,
    pub sids: Vec<u32>,
}

/// 把新页的 `ST_SlideId` 放进已有节名单。
#[derive(Clone, Copy)]
pub enum Place {
    Front,
    After(u32),
    Append,
}

pub fn count(root: &Element) -> usize {
    section_lst(root)
        .map(|lst| {
            lst.element_children()
                .filter(|e| e.local_name() == "section")
                .count()
        })
        .unwrap_or(0)
}

pub fn collect(root: &Element) -> Vec<SectionInfo> {
    let Some(lst) = section_lst(root) else {
        return Vec::new();
    };
    lst.element_children()
        .filter(|e| e.local_name() == "section")
        .enumerate()
        .map(|(i, sec)| SectionInfo {
            index: i + 1,
            guid: sec.attr("id").map(str::to_string),
            name: sec.attr("name").unwrap_or("").to_string(),
            sids: section_sids(sec),
        })
        .collect()
}

pub fn list_json(root: &Element) -> Vec<Value> {
    collect(root)
        .into_iter()
        .map(|s| {
            let slides: Vec<Value> = s
                .sids
                .iter()
                .filter_map(|&sid| slide_n_of(root, sid))
                .map(|n| json!(format!("slide:{n}")))
                .collect();
            let mut obj = Map::new();
            obj.insert("id".into(), json!(format!("section:{}", s.index)));
            obj.insert("guid".into(), json!(s.guid));
            obj.insert("name".into(), json!(s.name));
            obj.insert("slides".into(), Value::Array(slides));
            Value::Object(obj)
        })
        .collect()
}

pub fn parse_section_id(id: &str, json_path: &str) -> Result<usize, Error> {
    let Some(rest) = id.strip_prefix("section:") else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SectionPrefix,
            },
            json_path,
        ));
    };
    if rest.is_empty() || rest.contains('/') {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SectionPrefix,
            },
            json_path,
        ));
    }
    let n: usize = rest.parse().map_err(|_| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SectionIndexNotInt,
            },
            json_path,
        )
    })?;
    if n < 1 {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SectionIndexFromOne,
            },
            json_path,
        ));
    }
    Ok(n)
}

pub fn insert(
    pkg: &mut Package,
    pres_part: &str,
    pres: &mut Document,
    op: &InsertSlideSectionOp,
    op_index: usize,
) -> Result<(), Error> {
    let sids = presentation_sids(&pres.root);
    if let Some(start) = op.start {
        if start < 1 || start > sids.len() {
            return Err(Error::with_path(
                ErrorKind::SlideNotFound { n: start as u32 },
                format!("ops[{op_index}].start"),
            ));
        }
    }
    if let Some(after) = op.after {
        if section_lst(&pres.root).is_none() {
            return Err(Error::with_path(
                ErrorKind::SectionNotFound {
                    id: format!("section:{after}"),
                },
                format!("ops[{op_index}].after"),
            ));
        }
        ensure_section_index(&pres.root, after, op_index, "after")?;
    }
    if section_lst(&pres.root).is_none() {
        insert_first(&mut pres.root, op, &sids);
    } else {
        insert_into_existing(&mut pres.root, op, &sids, op_index)?;
    }
    pkg.set(pres_part, pres.to_bytes()?)?;
    Ok(())
}

pub fn set_name(
    pkg: &mut Package,
    pres_part: &str,
    pres: &mut Document,
    op: &SetSlideSectionNameOp,
    op_index: usize,
) -> Result<(), Error> {
    let idx = ensure_section_index(&pres.root, op.section, op_index, "id")?;
    let lst = section_lst_mut(&mut pres.root).expect("刚确认有节名单");
    let sec = section_at_mut(lst, idx).expect("刚确认节下标");
    sec.set_attr("name", op.name.clone());
    pkg.set(pres_part, pres.to_bytes()?)?;
    Ok(())
}

pub fn delete(
    pkg: &mut Package,
    pres_part: &str,
    pres: &mut Document,
    op: &DeleteSlideSectionOp,
    op_index: usize,
) -> Result<(), Error> {
    let idx = ensure_section_index(&pres.root, op.section, op_index, "id")?;
    if count(&pres.root) <= 1 {
        drop_section_lst(&mut pres.root);
        pkg.set(pres_part, pres.to_bytes()?)?;
        return Ok(());
    }
    let sids = {
        let lst = section_lst(&pres.root).expect("刚确认有节名单");
        let sec = section_at(lst, idx).expect("刚确认节下标");
        section_sids(sec)
    };
    let merge_into = if idx == 0 { 1 } else { idx - 1 };
    {
        let lst = section_lst_mut(&mut pres.root).expect("刚确认有节名单");
        if let Some(target) = section_at_mut(lst, merge_into) {
            let sld_lst = ensure_section_sld_lst(target);
            for sid in sids {
                if section_sids_of(sld_lst).contains(&sid) {
                    continue;
                }
                let el = sld_id_el(sld_lst, sid);
                sld_lst.children.push(Node::Element(el));
            }
        }
        remove_section_at(lst, idx);
    }
    pkg.set(pres_part, pres.to_bytes()?)?;
    Ok(())
}

pub fn set_slide(
    pkg: &mut Package,
    pres_part: &str,
    pres: &mut Document,
    op: &SetSlideSectionOp,
    op_index: usize,
) -> Result<(), Error> {
    let sids = presentation_sids(&pres.root);
    if op.slide < 1 || op.slide > sids.len() {
        return Err(Error::with_path(
            ErrorKind::SlideNotFound { n: op.slide as u32 },
            format!("ops[{op_index}].id"),
        ));
    }
    if section_lst(&pres.root).is_none() {
        return Err(Error::with_path(
            ErrorKind::NeedsPptxSectionList,
            format!("ops[{op_index}].section"),
        ));
    }
    let _ = ensure_section_index(&pres.root, op.section, op_index, "section")?;
    let sid = sids[op.slide - 1];
    {
        let lst = section_lst_mut(&mut pres.root).expect("刚确认有节名单");
        remove_sid_from_section_lst(lst, sid);
        let target = section_at_mut(lst, op.section - 1).expect("刚确认节下标");
        let sld_lst = ensure_section_sld_lst(target);
        if !section_sids_of(sld_lst).contains(&sid) {
            let el = sld_id_el(sld_lst, sid);
            sld_lst.children.push(Node::Element(el));
        }
    }
    adopt_orphans(&mut pres.root);
    reorder_sld_id_lst(&mut pres.root);
    pkg.set(pres_part, pres.to_bytes()?)?;
    Ok(())
}

pub fn place_sid(root: &mut Element, sid: u32, place: Place) {
    let Some(lst) = section_lst_mut(root) else {
        return;
    };
    remove_sid_from_section_lst(lst, sid);
    let sections = section_indices(lst);
    if sections.is_empty() {
        return;
    }
    let target_section = match place {
        Place::Front => sections[0],
        Place::Append => *sections.last().unwrap(),
        Place::After(after) => sections
            .iter()
            .copied()
            .find(|&i| match &lst.children[i] {
                Node::Element(sec) => section_sids(sec).contains(&after),
                _ => false,
            })
            .unwrap_or(*sections.last().unwrap()),
    };
    let Node::Element(sec) = &mut lst.children[target_section] else {
        return;
    };
    let sld_lst = ensure_section_sld_lst(sec);
    let el = sld_id_el(sld_lst, sid);
    match place {
        Place::Front => insert_sld_id(sld_lst, 0, el),
        Place::Append => sld_lst.children.push(Node::Element(el)),
        Place::After(after) => {
            let pos = section_sids_of(sld_lst).iter().position(|&id| id == after);
            if let Some(pos) = pos {
                insert_sld_id(sld_lst, pos + 1, el);
            } else {
                sld_lst.children.push(Node::Element(el));
            }
        }
    }
}

pub fn remove_sid(root: &mut Element, sid: u32) {
    if let Some(lst) = section_lst_mut(root) {
        remove_sid_from_section_lst(lst, sid);
    }
}

fn insert_first(root: &mut Element, op: &InsertSlideSectionOp, sids: &[u32]) {
    if let Some(start) = op.start {
        if start > 1 {
            let (before, rest) = sids.split_at(start - 1);
            let lst = ensure_section_lst(root);
            lst.push_element(new_section("", before));
            lst.push_element(new_section(&op.name, rest));
            return;
        }
    }
    let lst = ensure_section_lst(root);
    lst.push_element(new_section(&op.name, sids));
}

fn insert_into_existing(
    root: &mut Element,
    op: &InsertSlideSectionOp,
    sids: &[u32],
    op_index: usize,
) -> Result<(), Error> {
    if let Some(start) = op.start {
        split_at(root, sids[start - 1], &op.name, op_index)?;
        return Ok(());
    }
    let insert_at = op.after.unwrap_or_else(|| count(root));
    let lst = section_lst_mut(root).expect("已有节名单");
    insert_section_at(lst, insert_at, new_section(&op.name, &[]));
    Ok(())
}

fn split_at(root: &mut Element, sid: u32, name: &str, op_index: usize) -> Result<(), Error> {
    let lst = section_lst_mut(root).expect("已有节名单");
    let Some(sec_i) = section_indices(lst)
        .into_iter()
        .find(|&i| match &lst.children[i] {
            Node::Element(sec) => section_sids(sec).contains(&sid),
            _ => false,
        })
    else {
        return Err(Error::with_path(
            ErrorKind::SlideNotInPptxSection,
            format!("ops[{op_index}].start"),
        ));
    };
    let old = match &lst.children[sec_i] {
        Node::Element(sec) => section_sids(sec),
        _ => return Ok(()),
    };
    let pos = old.iter().position(|&id| id == sid).unwrap_or(0);
    let keep = old[..pos].to_vec();
    let take = old[pos..].to_vec();
    let nth = section_index_to_nth(lst, sec_i);
    if let Node::Element(sec) = &mut lst.children[sec_i] {
        replace_section_sids(sec, &keep);
    }
    insert_section_at(lst, nth + 1, new_section(name, &take));
    Ok(())
}

fn section_index_to_nth(lst: &Element, child_i: usize) -> usize {
    lst.children[..child_i]
        .iter()
        .filter(|n| matches!(n, Node::Element(e) if e.local_name() == "section"))
        .count()
}

fn ensure_section_index(
    root: &Element,
    n: usize,
    op_index: usize,
    field: &str,
) -> Result<usize, Error> {
    let total = count(root);
    if n < 1 || n > total {
        return Err(Error::with_path(
            ErrorKind::SectionNotFound {
                id: format!("section:{n}"),
            },
            format!("ops[{op_index}].{field}"),
        ));
    }
    Ok(n - 1)
}

fn adopt_orphans(root: &mut Element) {
    let pres = presentation_sids(root);
    let listed: Vec<u32> = collect(root).into_iter().flat_map(|s| s.sids).collect();
    let orphans: Vec<u32> = pres
        .into_iter()
        .filter(|sid| !listed.contains(sid))
        .collect();
    if orphans.is_empty() {
        return;
    }
    let Some(lst) = section_lst_mut(root) else {
        return;
    };
    let Some(&last) = section_indices(lst).last() else {
        return;
    };
    let Node::Element(sec) = &mut lst.children[last] else {
        return;
    };
    let sld_lst = ensure_section_sld_lst(sec);
    for sid in orphans {
        if !section_sids_of(sld_lst).contains(&sid) {
            let el = sld_id_el(sld_lst, sid);
            sld_lst.children.push(Node::Element(el));
        }
    }
}

fn reorder_sld_id_lst(root: &mut Element) {
    let order: Vec<u32> = collect(root).into_iter().flat_map(|s| s.sids).collect();
    let Some(lst) = root.child_by_local_mut("sldIdLst") else {
        return;
    };
    let mut elems: Vec<(u32, Node)> = Vec::new();
    let mut other = Vec::new();
    for n in lst.children.drain(..) {
        match n {
            Node::Element(e) if e.local_name() == "sldId" => {
                if let Some(sid) = e.attr("id").and_then(|s| s.parse::<u32>().ok()) {
                    elems.push((sid, Node::Element(e)));
                } else {
                    other.push(Node::Element(e));
                }
            }
            other_n => other.push(other_n),
        }
    }
    let mut out = Vec::new();
    for sid in &order {
        if let Some(pos) = elems.iter().position(|(s, _)| s == sid) {
            out.push(elems.remove(pos).1);
        }
    }
    for (_, n) in elems {
        out.push(n);
    }
    out.extend(other);
    lst.children = out;
}

fn presentation_sids(root: &Element) -> Vec<u32> {
    let Some(lst) = root.child_by_local("sldIdLst") else {
        return Vec::new();
    };
    lst.element_children()
        .filter(|e| e.local_name() == "sldId")
        .filter_map(|e| e.attr("id")?.parse().ok())
        .collect()
}

fn slide_n_of(root: &Element, sid: u32) -> Option<usize> {
    presentation_sids(root)
        .iter()
        .position(|&id| id == sid)
        .map(|i| i + 1)
}

fn section_sids(sec: &Element) -> Vec<u32> {
    let Some(lst) = sec.child_by_local("sldIdLst") else {
        return Vec::new();
    };
    section_sids_of(lst)
}

fn section_sids_of(lst: &Element) -> Vec<u32> {
    lst.element_children()
        .filter(|e| e.local_name() == "sldId")
        .filter_map(|e| e.attr("id")?.parse().ok())
        .collect()
}

fn section_lst(root: &Element) -> Option<&Element> {
    let ext_lst = root.child_by_local("extLst")?;
    for ext in ext_lst.element_children() {
        if ext.local_name() == "ext" && ext.attr("uri") == Some(SECTION_EXT_URI) {
            if let Some(lst) = ext.child_by_local("sectionLst") {
                return Some(lst);
            }
        }
    }
    for ext in ext_lst.element_children() {
        if let Some(lst) = ext.child_by_local("sectionLst") {
            return Some(lst);
        }
    }
    None
}

fn section_lst_mut(root: &mut Element) -> Option<&mut Element> {
    let path = section_lst_path(root)?;
    Some(follow_path(root, &path))
}

fn section_lst_path(root: &Element) -> Option<Vec<usize>> {
    let ext_lst_i = root
        .children
        .iter()
        .position(|n| matches!(n, Node::Element(e) if e.local_name() == "extLst"))?;
    let Node::Element(ext_lst) = &root.children[ext_lst_i] else {
        return None;
    };
    let mut found: Option<usize> = None;
    let mut fallback: Option<usize> = None;
    for (i, n) in ext_lst.children.iter().enumerate() {
        let Node::Element(ext) = n else {
            continue;
        };
        if ext.local_name() != "ext" {
            continue;
        }
        if ext.child_by_local("sectionLst").is_none() {
            continue;
        }
        if fallback.is_none() {
            fallback = Some(i);
        }
        if ext.attr("uri") == Some(SECTION_EXT_URI) {
            found = Some(i);
            break;
        }
    }
    let ext_i = found.or(fallback)?;
    let Node::Element(ext) = &ext_lst.children[ext_i] else {
        return None;
    };
    let lst_i = ext
        .children
        .iter()
        .position(|n| matches!(n, Node::Element(e) if e.local_name() == "sectionLst"))?;
    Some(vec![ext_lst_i, ext_i, lst_i])
}

fn follow_path<'a>(el: &'a mut Element, path: &[usize]) -> &'a mut Element {
    let mut cur = el;
    for &i in path {
        let Node::Element(next) = &mut cur.children[i] else {
            panic!("节扩展路径不是元素");
        };
        cur = next;
    }
    cur
}

fn ensure_section_lst(root: &mut Element) -> &mut Element {
    if section_lst(root).is_some() {
        return section_lst_mut(root).expect("刚确认有节名单");
    }
    if root.child_by_local("extLst").is_none() {
        root.push_element(Element::new("p:extLst"));
    }
    let ext_lst = root.child_by_local_mut("extLst").expect("刚插入 extLst");
    let mut ext = Element::new("p:ext").with_attr("uri", SECTION_EXT_URI);
    ext.push_element(Element::new("p14:sectionLst").with_attr("xmlns:p14", P14_NS));
    ext_lst.push_element(ext);
    section_lst_mut(root).expect("刚插入 sectionLst")
}

fn drop_section_lst(root: &mut Element) {
    let Some(path) = section_lst_path(root) else {
        return;
    };
    if path.len() != 3 {
        return;
    }
    let ext_lst_i = path[0];
    let ext_i = path[1];
    let lst_i = path[2];
    let Node::Element(ext_lst) = &mut root.children[ext_lst_i] else {
        return;
    };
    let Node::Element(ext) = &mut ext_lst.children[ext_i] else {
        return;
    };
    ext.children.remove(lst_i);
    let ext_empty = ext.element_children().next().is_none();
    if ext_empty {
        ext_lst.children.remove(ext_i);
    }
    let lst_empty = {
        let Node::Element(ext_lst) = &root.children[ext_lst_i] else {
            return;
        };
        ext_lst.element_children().next().is_none()
    };
    if lst_empty {
        root.children.remove(ext_lst_i);
    }
}

fn section_indices(lst: &Element) -> Vec<usize> {
    lst.children
        .iter()
        .enumerate()
        .filter_map(|(i, n)| match n {
            Node::Element(e) if e.local_name() == "section" => Some(i),
            _ => None,
        })
        .collect()
}

fn section_at(lst: &Element, nth: usize) -> Option<&Element> {
    lst.element_children()
        .filter(|e| e.local_name() == "section")
        .nth(nth)
}

fn section_at_mut(lst: &mut Element, nth: usize) -> Option<&mut Element> {
    let i = *section_indices(lst).get(nth)?;
    match &mut lst.children[i] {
        Node::Element(e) => Some(e),
        _ => None,
    }
}

fn insert_section_at(lst: &mut Element, nth: usize, el: Element) {
    let mut seen = 0usize;
    for (i, n) in lst.children.iter().enumerate() {
        let Node::Element(e) = n else {
            continue;
        };
        if e.local_name() != "section" {
            continue;
        }
        if seen == nth {
            lst.children.insert(i, Node::Element(el));
            return;
        }
        seen += 1;
    }
    lst.children.push(Node::Element(el));
}

fn remove_section_at(lst: &mut Element, nth: usize) {
    if let Some(&i) = section_indices(lst).get(nth) {
        lst.children.remove(i);
    }
}

fn ensure_section_sld_lst(sec: &mut Element) -> &mut Element {
    if sec.child_by_local("sldIdLst").is_none() {
        sec.push_element(Element::new("p14:sldIdLst"));
    }
    sec.child_by_local_mut("sldIdLst").expect("刚插入 sldIdLst")
}

fn replace_section_sids(sec: &mut Element, sids: &[u32]) {
    let tag = sec
        .child_by_local("sldIdLst")
        .and_then(|lst| {
            lst.element_children()
                .find(|e| e.local_name() == "sldId")
                .map(|e| e.name.clone())
        })
        .unwrap_or_else(|| "p14:sldId".into());
    let lst_tag = sec
        .child_by_local("sldIdLst")
        .map(|e| e.name.clone())
        .unwrap_or_else(|| "p14:sldIdLst".into());
    sec.children.retain(|n| match n {
        Node::Element(e) => e.local_name() != "sldIdLst",
        _ => true,
    });
    let mut lst = Element::new(lst_tag);
    for sid in sids {
        lst.push_element(Element::new(tag.clone()).with_attr("id", sid.to_string()));
    }
    sec.push_element(lst);
}

fn new_section(name: &str, sids: &[u32]) -> Element {
    let mut lst = Element::new("p14:sldIdLst");
    for sid in sids {
        lst.push_element(Element::new("p14:sldId").with_attr("id", sid.to_string()));
    }
    let mut sec = Element::new("p14:section")
        .with_attr("name", name)
        .with_attr("id", new_guid());
    sec.push_element(lst);
    sec
}

fn sld_id_el(lst: &Element, sid: u32) -> Element {
    let tag = lst
        .element_children()
        .find(|e| e.local_name() == "sldId")
        .map(|e| e.name.clone())
        .unwrap_or_else(|| "p14:sldId".into());
    Element::new(tag).with_attr("id", sid.to_string())
}

fn insert_sld_id(lst: &mut Element, pos: usize, el: Element) {
    let mut seen = 0usize;
    for (i, n) in lst.children.iter().enumerate() {
        let Node::Element(e) = n else {
            continue;
        };
        if e.local_name() != "sldId" {
            continue;
        }
        if seen == pos {
            lst.children.insert(i, Node::Element(el));
            return;
        }
        seen += 1;
    }
    lst.children.push(Node::Element(el));
}

fn remove_sid_from_section_lst(lst: &mut Element, sid: u32) {
    for child in lst.element_children_mut() {
        if child.local_name() != "section" {
            continue;
        }
        if let Some(sld_lst) = child.child_by_local_mut("sldIdLst") {
            sld_lst.children.retain(|n| match n {
                Node::Element(e) => {
                    !(e.local_name() == "sldId"
                        && e.attr("id").and_then(|s| s.parse::<u32>().ok()) == Some(sid))
                }
                _ => true,
            });
        }
    }
}

pub(crate) fn new_guid() -> String {
    let t = crate::host_env::now_nanos();
    let n = GUID_SEQ.fetch_add(1, Ordering::Relaxed) as u128;
    let pid = crate::host_env::pid();
    let mix = t ^ (n << 48) ^ (pid << 16);
    let mut b = [0u8; 16];
    for (i, byte) in b.iter_mut().enumerate() {
        *byte = (mix >> ((i % 16) * 8)) as u8 ^ (i as u8).wrapping_mul(31);
    }
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    format!(
        "{{{:02X}{:02X}{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}}}",
        b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7], b[8], b[9], b[10], b[11], b[12], b[13], b[14],
        b[15]
    )
}
