//! 四件套部件、关系 Target、drawing 缓存。

use crate::error::Error;
use crate::opc::{
    rels_part_name, resolve_rel_target, ContentTypes, Package, Relationship, Relationships,
};
use crate::xml::{self, Element};

use super::data::{build_data_xml, RelIds};
use super::{
    colors_xml, style_xml, ColorsKind, LayoutKind, NewPoint, StyleKind, CT_COLORS, CT_DATA,
    CT_LAYOUT, CT_STYLE, REL_CS, REL_DM, REL_LO, REL_QS,
};

/// 从源部件到目标部件的相对关系 Target，算法与 `pptx/picture.rs` 相同。
pub fn rel_target_from(source_part: &str, target_opc: &str) -> String {
    let src_dir = split_dir_file(source_part).0;
    let tgt = target_opc.trim_start_matches('/');
    let src = src_dir.trim_start_matches('/');
    if src.is_empty() || src == "/" {
        return tgt.to_string();
    }
    if let Some(rest) = tgt.strip_prefix(&format!("{src}/")) {
        return rest.to_string();
    }
    let src_parts: Vec<&str> = src.split('/').filter(|s| !s.is_empty()).collect();
    let tgt_parts: Vec<&str> = tgt.split('/').filter(|s| !s.is_empty()).collect();
    let mut common = 0usize;
    while common < src_parts.len()
        && common < tgt_parts.len()
        && src_parts[common] == tgt_parts[common]
    {
        common += 1;
    }
    let up = src_parts.len() - common;
    let rest = tgt_parts[common..].join("/");
    format!("{}{rest}", "../".repeat(up))
}

fn split_dir_file(part: &str) -> (&str, &str) {
    match part.rsplit_once('/') {
        Some((dir, file)) => (dir, file),
        None => ("", part),
    }
}

pub fn write_four_parts(
    pkg: &mut Package,
    host: &str,
    diagrams_dir: &str,
    layout: LayoutKind,
    points: Option<&[NewPoint]>,
) -> Result<RelIds, Error> {
    let data_xml = build_data_xml(layout, points);
    let data_part = next_numbered(pkg, diagrams_dir, "data", ".xml");
    let layout_part = next_numbered(pkg, diagrams_dir, "layout", ".xml");
    let colors_part = next_numbered(pkg, diagrams_dir, "colors", ".xml");
    let style_part = next_numbered(pkg, diagrams_dir, "quickStyle", ".xml");
    pkg.set(&data_part, data_xml.into_bytes())?;
    pkg.set(&layout_part, layout.layout_xml().as_bytes().to_vec())?;
    pkg.set(&colors_part, colors_xml(ColorsKind::Accent1).into_bytes())?;
    pkg.set(&style_part, style_xml(StyleKind::Simple).into_bytes())?;

    let mut host_rels = Relationships::from_package(pkg, host)?;
    let dm = push_rel(&mut host_rels, host, REL_DM, &data_part);
    let lo = push_rel(&mut host_rels, host, REL_LO, &layout_part);
    let qs = push_rel(&mut host_rels, host, REL_QS, &style_part);
    let cs = push_rel(&mut host_rels, host, REL_CS, &colors_part);
    host_rels.write_to(pkg, host)?;

    let mut cts = ContentTypes::from_package(pkg)?;
    cts.set_override(&data_part, CT_DATA)?;
    cts.set_override(&layout_part, CT_LAYOUT)?;
    cts.set_override(&colors_part, CT_COLORS)?;
    cts.set_override(&style_part, CT_STYLE)?;
    cts.write_to(pkg)?;

    Ok(RelIds {
        dm: Some(dm),
        lo: Some(lo),
        qs: Some(qs),
        cs: Some(cs),
    })
}

fn push_rel(rels: &mut Relationships, host: &str, rel_type: &str, part: &str) -> String {
    let id = rels.next_rid();
    rels.push(Relationship {
        id: id.clone(),
        rel_type: rel_type.into(),
        target: rel_target_from(host, part),
        target_mode: None,
        extra: Vec::new(),
    });
    id
}

fn next_numbered(pkg: &Package, dir: &str, stem: &str, ext: &str) -> String {
    let mut n = 1u32;
    loop {
        let name = format!("{dir}/{stem}{n}{ext}");
        if !pkg.contains(&name) {
            return name;
        }
        n += 1;
    }
}

pub fn family_of(uid: &str) -> Option<LayoutKind> {
    let tail = uid.rsplit('/').next().unwrap_or(uid).to_ascii_lowercase();
    if tail == "default" || tail.starts_with("hlist") || tail.starts_with("vlist") {
        Some(LayoutKind::List)
    } else if tail == "orgchart1" || tail.starts_with("orgchart") {
        Some(LayoutKind::Org)
    } else if tail == "cycle2" || tail.starts_with("cycle") {
        Some(LayoutKind::Cycle)
    } else {
        None
    }
}

fn walk_skip_txbx(el: &Element, f: &mut impl FnMut(&Element)) {
    crate::xml::walk_elements_if(el, |node| {
        if node.local_name() == "txbxContent" {
            return false;
        }
        f(node);
        true
    });
}

pub fn rel_ids(host_el: &Element) -> RelIds {
    let mut out = RelIds::default();
    walk_skip_txbx(host_el, &mut |e| {
        if e.local_name() == "relIds" {
            out.dm = e.attr_local("dm").map(str::to_string);
            out.lo = e.attr_local("lo").map(str::to_string);
            out.qs = e.attr_local("qs").map(str::to_string);
            out.cs = e.attr_local("cs").map(str::to_string);
        }
    });
    out
}

pub fn find_rel_ids_mut(host_el: &mut Element) -> Option<&mut Element> {
    find_local_mut(host_el, "relIds")
}

pub fn find_local<'a>(el: &'a Element, local: &str) -> Option<&'a Element> {
    crate::xml::find_by_local(el, local)
}

pub fn find_local_mut<'a>(el: &'a mut Element, local: &str) -> Option<&'a mut Element> {
    crate::xml::find_by_local_mut(el, local)
}

pub fn set_rid_attr(el: &mut Element, local: &str, rid: &str) {
    if let Some((_, v)) = el
        .attrs
        .iter_mut()
        .find(|(k, _)| k.rsplit(':').next() == Some(local))
    {
        *v = rid.to_string();
        return;
    }
    el.set_attr(&format!("r:{local}"), rid);
}

/// 换 colors / style / layout 卫星部件。`host_el` 是含 `dgm:relIds` 的宿主节点。
/// 若换了关系 id，返回旧 `(rid, 部件)`，调用方在宿主 XML 的可变借用结束后再 `drop_replaced_satellite`。
pub fn retarget_satellite(
    pkg: &mut Package,
    host: &str,
    diagrams_dir: &str,
    host_el: &mut Element,
    attr: &str,
    rel_type: &str,
    ct: &str,
    stem: &str,
    unique_id: &str,
    xml_body: &str,
) -> Result<Option<(String, String)>, Error> {
    let old_rid = rel_ids(host_el).get(attr).cloned();
    let old_part = old_rid
        .as_ref()
        .and_then(|rid| crate::opc::resolve_id(pkg, host, rid).ok());
    let part = if let Some(existing) = find_part_by_unique_id(pkg, unique_id) {
        existing
    } else {
        let part = next_numbered(pkg, diagrams_dir, stem, ".xml");
        pkg.set(&part, xml_body.as_bytes().to_vec())?;
        let mut cts = ContentTypes::from_package(pkg)?;
        cts.set_override(&part, ct)?;
        cts.write_to(pkg)?;
        part
    };
    let mut host_rels = Relationships::from_package(pkg, host)?;
    let rid = if let Some(existing) = host_rels.rels.iter().find_map(|r| {
        if r.is_external() {
            return None;
        }
        resolve_rel_target(host, &r.target)
            .ok()
            .filter(|p| p == &part)
            .map(|_| r.id.clone())
    }) {
        existing
    } else {
        let rid = host_rels.next_rid();
        host_rels.push(Relationship {
            id: rid.clone(),
            rel_type: rel_type.into(),
            target: rel_target_from(host, &part),
            target_mode: None,
            extra: Vec::new(),
        });
        host_rels.write_to(pkg, host)?;
        rid
    };
    if let Some(rel) = find_rel_ids_mut(host_el) {
        set_rid_attr(rel, attr, &rid);
    }
    match (old_rid, old_part) {
        (Some(old_rid), Some(old)) if old_rid != rid || old != part => Ok(Some((old_rid, old))),
        _ => Ok(None),
    }
}

fn rid_used_in(el: &Element, rid: &str) -> bool {
    if el.attrs.iter().any(|(_, v)| v == rid) {
        return true;
    }
    el.element_children().any(|c| rid_used_in(c, rid))
}

/// 宿主 XML 不再引用旧 rid 时，从 `.rels` 摘掉并卸无主卫星部件。
pub fn drop_replaced_satellite(
    pkg: &mut Package,
    host: &str,
    host_root: &Element,
    old_rid: &str,
    old_part: &str,
) -> Result<(), Error> {
    if rid_used_in(host_root, old_rid) {
        return Ok(());
    }
    let mut rels = Relationships::from_package(pkg, host)?;
    let before = rels.rels.len();
    rels.remove_by_id(old_rid);
    if rels.rels.len() != before {
        if rels.rels.is_empty() {
            pkg.remove(&rels_part_name(host));
        } else {
            rels.write_to(pkg, host)?;
        }
    }
    drop_unused(pkg, old_part)
}

fn find_part_by_unique_id(pkg: &Package, unique_id: &str) -> Option<String> {
    for name in pkg.part_names() {
        if !name.contains("/diagrams/") {
            continue;
        }
        let Ok(bytes) = pkg.get(&name) else {
            continue;
        };
        let Ok(doc) = xml::parse(bytes) else {
            continue;
        };
        if doc.root.attr_local("uniqueId") == Some(unique_id) {
            return Some(name);
        }
    }
    None
}

pub fn drop_drawing_caches(pkg: &mut Package, host: &str, data_part: &str) -> Result<(), Error> {
    let parts = collect_drawing_caches(pkg, host, data_part)?;
    for part in parts {
        drop_unused(pkg, &part)?;
        strip_rels_to(pkg, data_part, &part)?;
        strip_rels_to(pkg, host, &part)?;
    }
    Ok(())
}

pub fn collect_drawing_caches(
    pkg: &Package,
    host: &str,
    data_part: &str,
) -> Result<Vec<String>, Error> {
    let mut parts = drawing_cache_parts(pkg, data_part)?;
    parts.extend(host_drawing_cache_parts(pkg, host)?);
    if let Some(sib) = sibling_drawing_part(data_part) {
        if pkg.contains(&sib) {
            parts.push(sib);
        }
    }
    parts.sort();
    parts.dedup();
    Ok(parts)
}

/// Word 另存时常把 drawing 缓存写成与 data 同号的部件，却不写 diagramDrawing 关系。
fn sibling_drawing_part(data_part: &str) -> Option<String> {
    let (dir, file) = data_part.rsplit_once('/')?;
    let rest = file.strip_prefix("data")?;
    Some(format!("{dir}/drawing{rest}"))
}

fn drawing_cache_parts(pkg: &Package, data_part: &str) -> Result<Vec<String>, Error> {
    parts_by_rel_type(pkg, data_part, "diagramDrawing")
}

fn host_drawing_cache_parts(pkg: &Package, host: &str) -> Result<Vec<String>, Error> {
    parts_by_rel_type(pkg, host, "diagramDrawing")
}

fn parts_by_rel_type(pkg: &Package, source: &str, needle: &str) -> Result<Vec<String>, Error> {
    let rels = Relationships::from_package(pkg, source)?;
    let mut out = Vec::new();
    for r in &rels.rels {
        if r.is_external() || !r.rel_type.contains(needle) {
            continue;
        }
        if let Ok(p) = resolve_rel_target(source, &r.target) {
            out.push(p);
        }
    }
    Ok(out)
}

fn strip_rels_to(pkg: &mut Package, source: &str, target: &str) -> Result<(), Error> {
    let mut rels = Relationships::from_package(pkg, source)?;
    let before = rels.rels.len();
    rels.remove_by_target_part(source, target);
    if rels.rels.len() != before {
        if rels.rels.is_empty() {
            pkg.remove(&rels_part_name(source));
        } else {
            rels.write_to(pkg, source)?;
        }
    }
    Ok(())
}

pub fn drop_unused(pkg: &mut Package, part: &str) -> Result<(), Error> {
    if part_still_used(pkg, part)? {
        return Ok(());
    }
    let rels = rels_part_name(part);
    pkg.remove(part);
    pkg.remove(&rels);
    let mut cts = ContentTypes::from_package(pkg)?;
    cts.remove_override(part);
    cts.write_to(pkg)?;
    Ok(())
}

fn part_still_used(pkg: &Package, target: &str) -> Result<bool, Error> {
    for name in pkg.part_names() {
        if !name.ends_with(".rels") {
            continue;
        }
        let Some(source) = source_from_rels(&name) else {
            continue;
        };
        let rels = Relationships::parse(pkg.get(&name)?)?;
        for r in &rels.rels {
            if r.is_external() {
                continue;
            }
            if let Ok(p) = resolve_rel_target(&source, &r.target) {
                if p == target {
                    return Ok(true);
                }
            }
        }
    }
    Ok(false)
}

fn source_from_rels(rels_name: &str) -> Option<String> {
    let name = rels_name.strip_prefix('/')?;
    let (parent, file) = name.rsplit_once('/')?;
    let stem = file.strip_suffix(".rels")?;
    if parent.is_empty() || parent == "_rels" {
        Some(format!("/{stem}"))
    } else {
        let dir = parent.strip_suffix("/_rels")?;
        Some(format!("/{dir}/{stem}"))
    }
}
