//! 幻灯片名单：插入空白页、删除、重排、隐藏。

use crate::error::{Error, ErrorKind};
use crate::opc::{
    rels_part_name, resolve_rel_target, ContentTypes, Package, Relationship, Relationships,
};
use crate::xml::{self, Document, Element, Node};

use super::picture;
use super::request::{DeleteSlideOp, InsertSlideOp, MoveSlideOp, SetSlideHiddenOp};
use super::shape::{self, SlideMeta};
use super::{CT_SLIDE, DML_NS, PML_NS, REL_NS, REL_SLIDE, REL_SLIDE_LAYOUT};

pub fn insert(
    pkg: &mut Package,
    pres_part: &str,
    pres: &mut Document,
    slides: &[SlideMeta],
    op: &InsertSlideOp,
    op_index: usize,
) -> Result<(), Error> {
    let insert_at = match op.after {
        None => slides.len(),
        Some(after) => {
            slide_pos(slides, after, op_index, "after")?;
            after
        }
    };
    let neighbor = if insert_at == 0 {
        slides.first()
    } else {
        slides.get(insert_at - 1)
    };
    let layout = layout_for_insert(pkg, pres_part, &pres.root, neighbor, op_index)?;
    let slide_part = next_slide_part(pkg);
    pkg.set(&slide_part, blank_slide().to_bytes()?)?;
    let mut slide_rels = Relationships { rels: Vec::new() };
    slide_rels.push(Relationship {
        id: "rId1".into(),
        rel_type: REL_SLIDE_LAYOUT.into(),
        target: picture::rel_target_from(&slide_part, &layout),
        target_mode: None,
        extra: Vec::new(),
    });
    slide_rels.write_to(pkg, &slide_part)?;
    let mut pres_rels = Relationships::from_package(pkg, pres_part)?;
    let rid = pres_rels.next_rid();
    pres_rels.push(Relationship {
        id: rid.clone(),
        rel_type: REL_SLIDE.into(),
        target: picture::rel_target_from(pres_part, &slide_part),
        target_mode: None,
        extra: Vec::new(),
    });
    pres_rels.write_to(pkg, pres_part)?;
    picture::ensure_xmlns(&mut pres.root, "r", REL_NS);
    ensure_sld_id_lst(&mut pres.root);
    let neighbor_sid = neighbor.and_then(|m| slide_st_id(&pres.root, m.index));
    let lst = sld_id_lst_mut(&mut pres.root)?;
    let sid = next_sld_id(lst);
    let tag = sld_id_tag(lst);
    let el = Element::new(tag)
        .with_attr("id", sid.to_string())
        .with_attr("r:id", rid);
    insert_sld_id(lst, insert_at, el);
    place_sid_in_sections(
        &mut pres.root,
        sid,
        if op.after.is_some() {
            neighbor_sid.map(Place::After).unwrap_or(Place::Append)
        } else {
            Place::Append
        },
    );
    let mut cts = ContentTypes::from_package(pkg)?;
    cts.set_override(&slide_part, CT_SLIDE)?;
    cts.write_to(pkg)?;
    pkg.set(pres_part, pres.to_bytes()?)?;
    Ok(())
}

pub fn delete(
    pkg: &mut Package,
    pres_part: &str,
    pres: &mut Document,
    slides: &[SlideMeta],
    op: &DeleteSlideOp,
    op_index: usize,
) -> Result<(), Error> {
    let pos = slide_pos(slides, op.slide, op_index, "id")?;
    if slides.len() <= 1 {
        return Err(Error::with_path(
            ErrorKind::LastSlide,
            format!("ops[{op_index}].id"),
        ));
    }
    let meta = &slides[pos];
    let sid = slide_st_id(&pres.root, meta.index).ok_or_else(|| {
        Error::new(ErrorKind::XmlMissing {
            parent: "sldIdLst".into(),
            child: "sldId".into(),
        })
    })?;
    let children = internal_rel_targets(pkg, &meta.part)?;
    let rid = remove_sld_id(&mut pres.root, pos).ok_or_else(|| {
        Error::new(ErrorKind::XmlMissing {
            parent: "sldIdLst".into(),
            child: "sldId".into(),
        })
    })?;
    let mut pres_rels = Relationships::from_package(pkg, pres_part)?;
    pres_rels.remove_by_id(&rid);
    pres_rels.write_to(pkg, pres_part)?;
    remove_sid_from_sections(&mut pres.root, sid);
    if let Some(cust) = find_mut_by_local(&mut pres.root, "custShowLst") {
        remove_sld_refs(cust, &sid.to_string());
    }
    let mut cts = ContentTypes::from_package(pkg)?;
    pkg.remove(&meta.part);
    pkg.remove(&rels_part_name(&meta.part));
    cts.remove_override(&meta.part);
    for (part, rel_type) in children {
        drop_if_unused(pkg, &mut cts, &part, &rel_type)?;
    }
    cts.write_to(pkg)?;
    pkg.set(pres_part, pres.to_bytes()?)?;
    Ok(())
}

pub fn move_slide(
    pres_part: &str,
    pres: &mut Document,
    pkg: &mut Package,
    slides: &[SlideMeta],
    op: &MoveSlideOp,
    op_index: usize,
) -> Result<(), Error> {
    let from = slide_pos(slides, op.slide, op_index, "id")?;
    if let Some(after) = op.after {
        if after == op.slide {
            return Err(Error::with_path(
                ErrorKind::MoveSlideAfterSelf,
                format!("ops[{op_index}].after"),
            ));
        }
        slide_pos(slides, after, op_index, "after")?;
    }
    let dest = op.after.unwrap_or_default();
    if from == dest {
        return Ok(());
    }
    let sid = slide_st_id(&pres.root, op.slide).ok_or_else(|| {
        Error::new(ErrorKind::XmlMissing {
            parent: "sldIdLst".into(),
            child: "sldId".into(),
        })
    })?;
    let after_sid = op.after.and_then(|n| slide_st_id(&pres.root, n));
    let lst = sld_id_lst_mut(&mut pres.root)?;
    let from_node = sld_id_child_index(lst, from).ok_or_else(|| {
        Error::new(ErrorKind::XmlMissing {
            parent: "sldIdLst".into(),
            child: "sldId".into(),
        })
    })?;
    let dest_node = if dest == 0 {
        sld_id_child_index(lst, 0).unwrap_or(0)
    } else {
        let after_idx = sld_id_child_index(lst, dest - 1).ok_or_else(|| {
            Error::new(ErrorKind::XmlMissing {
                parent: "sldIdLst".into(),
                child: "sldId".into(),
            })
        })?;
        after_idx + 1
    };
    if from_node != dest_node {
        let node = lst.children.remove(from_node);
        let insert_at = if from_node < dest_node {
            dest_node - 1
        } else {
            dest_node
        };
        lst.children.insert(insert_at, node);
    }
    let place = match op.after {
        None => Place::Front,
        Some(_) => after_sid.map(Place::After).unwrap_or(Place::Front),
    };
    place_sid_in_sections(&mut pres.root, sid, place);
    pkg.set(pres_part, pres.to_bytes()?)?;
    Ok(())
}

pub fn set_hidden(
    pkg: &mut Package,
    slides: &[SlideMeta],
    op: &SetSlideHiddenOp,
    op_index: usize,
) -> Result<(), Error> {
    let pos = slide_pos(slides, op.slide, op_index, "id")?;
    let part = slides[pos].part.clone();
    let mut doc = xml::parse(pkg.get(&part)?)?;
    let currently = is_hidden(&doc.root);
    if currently == op.hidden {
        return Ok(());
    }
    if op.hidden {
        doc.root.set_attr("show", "0");
    } else {
        doc.root.remove_attr("show");
    }
    pkg.set(&part, doc.to_bytes()?)?;
    Ok(())
}

fn slide_pos(slides: &[SlideMeta], n: usize, op_index: usize, field: &str) -> Result<usize, Error> {
    if n < 1 || n > slides.len() {
        return Err(Error::with_path(
            ErrorKind::SlideNotFound { n: n as u32 },
            format!("ops[{op_index}].{field}"),
        ));
    }
    Ok(n - 1)
}

fn is_hidden(root: &Element) -> bool {
    matches!(root.attr("show"), Some("0") | Some("false") | Some("FALSE"))
}

fn layout_for_insert(
    pkg: &Package,
    pres_part: &str,
    pres_root: &Element,
    neighbor: Option<&SlideMeta>,
    op_index: usize,
) -> Result<String, Error> {
    if let Some(meta) = neighbor {
        if let Some(part) = slide_layout_of(pkg, &meta.part)? {
            return Ok(part);
        }
    }
    if let Some(part) = first_master_layout(pkg, pres_part, pres_root)? {
        return Ok(part);
    }
    Err(Error::with_path(
        ErrorKind::SlideLayoutMissing,
        format!("ops[{op_index}].id"),
    ))
}

pub(crate) fn slide_layout_of(pkg: &Package, slide_part: &str) -> Result<Option<String>, Error> {
    let rels = Relationships::from_package(pkg, slide_part)?;
    let Some(rel) = rels
        .rels
        .iter()
        .find(|r| r.rel_type.ends_with("/slideLayout"))
    else {
        return Ok(None);
    };
    Ok(Some(crate::opc::resolve_id(pkg, slide_part, &rel.id)?))
}

fn first_master_layout(
    pkg: &Package,
    pres_part: &str,
    pres_root: &Element,
) -> Result<Option<String>, Error> {
    let mut masters = Vec::new();
    if let Some(lst) = pres_root.child_by_local("sldMasterIdLst") {
        for el in lst
            .element_children()
            .filter(|e| e.local_name() == "sldMasterId")
        {
            if let Some(rid) = shape::rid_attr(el) {
                masters.push(crate::opc::resolve_id(pkg, pres_part, rid)?);
            }
        }
    }
    if masters.is_empty() {
        let rels = Relationships::from_package(pkg, pres_part)?;
        for rel in rels
            .rels
            .iter()
            .filter(|r| r.rel_type.ends_with("/slideMaster"))
        {
            masters.push(crate::opc::resolve_id(pkg, pres_part, &rel.id)?);
        }
    }
    for master in masters {
        let rels = Relationships::from_package(pkg, &master)?;
        if let Some(rel) = rels
            .rels
            .iter()
            .find(|r| r.rel_type.ends_with("/slideLayout"))
        {
            return Ok(Some(crate::opc::resolve_id(pkg, &master, &rel.id)?));
        }
    }
    Ok(None)
}

fn next_slide_part(pkg: &Package) -> String {
    let mut n = 1u32;
    loop {
        let name = format!("/ppt/slides/slide{n}.xml");
        if !pkg.contains(&name) {
            return name;
        }
        n += 1;
    }
}

fn blank_slide() -> Document {
    let tree = r#"<p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/><a:chOff x="0" y="0"/><a:chExt cx="0" cy="0"/></a:xfrm></p:grpSpPr>"#;
    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><p:sld xmlns:a="{DML_NS}" xmlns:r="{REL_NS}" xmlns:p="{PML_NS}"><p:cSld><p:spTree>{tree}</p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:sld>"#
    );
    xml::parse(xml.as_bytes()).expect("空白页 XML 是常量")
}

fn ensure_sld_id_lst(root: &mut Element) {
    if root.child_by_local("sldIdLst").is_some() {
        return;
    }
    let lst = Element::new("p:sldIdLst");
    if let Some(i) = root.children.iter().position(|n| match n {
        Node::Element(e) => e.local_name() == "sldMasterIdLst",
        Node::Text(_) => false,
    }) {
        root.children.insert(i + 1, Node::Element(lst));
    } else {
        root.children.insert(0, Node::Element(lst));
    }
}

fn sld_id_lst_mut(root: &mut Element) -> Result<&mut Element, Error> {
    root.child_by_local_mut("sldIdLst").ok_or_else(|| {
        Error::new(ErrorKind::XmlMissing {
            parent: "presentation".into(),
            child: "sldIdLst".into(),
        })
    })
}

fn sld_id_tag(lst: &Element) -> String {
    lst.element_children()
        .find(|e| e.local_name() == "sldId")
        .map(|e| e.name.clone())
        .unwrap_or_else(|| "p:sldId".into())
}

fn next_sld_id(lst: &Element) -> u32 {
    let mut max = 255u32;
    for e in lst.element_children().filter(|e| e.local_name() == "sldId") {
        if let Some(id) = e.attr("id").and_then(|s| s.parse::<u32>().ok()) {
            max = max.max(id);
        }
    }
    max + 1
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

fn sld_id_child_index(lst: &Element, pos: usize) -> Option<usize> {
    let mut seen = 0usize;
    for (i, n) in lst.children.iter().enumerate() {
        let Node::Element(e) = n else {
            continue;
        };
        if e.local_name() != "sldId" {
            continue;
        }
        if seen == pos {
            return Some(i);
        }
        seen += 1;
    }
    None
}

fn remove_sld_id(root: &mut Element, pos: usize) -> Option<String> {
    let lst = root.child_by_local_mut("sldIdLst")?;
    let idx = sld_id_child_index(lst, pos)?;
    let Node::Element(el) = lst.children.remove(idx) else {
        return None;
    };
    shape::rid_attr(&el).map(str::to_string)
}

fn slide_st_id(pres_root: &Element, index: usize) -> Option<u32> {
    let lst = pres_root.child_by_local("sldIdLst")?;
    lst.element_children()
        .filter(|e| e.local_name() == "sldId")
        .nth(index.checked_sub(1)?)
        .and_then(|e| e.attr("id")?.parse().ok())
}

#[derive(Clone, Copy)]
enum Place {
    Front,
    After(u32),
    Append,
}

fn place_sid_in_sections(root: &mut Element, sid: u32, place: Place) {
    let Some(lst) = find_mut_by_local(root, "sectionLst") else {
        return;
    };
    remove_sid_from_section_lst(lst, sid);
    let sections: Vec<usize> = lst
        .children
        .iter()
        .enumerate()
        .filter_map(|(i, n)| match n {
            Node::Element(e) if e.local_name() == "section" => Some(i),
            _ => None,
        })
        .collect();
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
                Node::Element(sec) => section_has_sid(sec, after),
                _ => false,
            })
            .unwrap_or(*sections.last().unwrap()),
    };
    let Node::Element(sec) = &mut lst.children[target_section] else {
        return;
    };
    let sld_lst = ensure_section_sld_lst(sec);
    let tag = sld_lst
        .element_children()
        .find(|e| e.local_name() == "sldId")
        .map(|e| e.name.clone())
        .unwrap_or_else(|| "p14:sldId".into());
    let el = Element::new(tag).with_attr("id", sid.to_string());
    match place {
        Place::Front => insert_sld_id(sld_lst, 0, el),
        Place::Append => sld_lst.children.push(Node::Element(el)),
        Place::After(after) => {
            let pos = sld_lst
                .element_children()
                .filter(|e| e.local_name() == "sldId")
                .position(|e| e.attr("id").and_then(|s| s.parse::<u32>().ok()) == Some(after));
            if let Some(pos) = pos {
                insert_sld_id(sld_lst, pos + 1, el);
            } else {
                sld_lst.children.push(Node::Element(el));
            }
        }
    }
}

fn remove_sid_from_sections(root: &mut Element, sid: u32) {
    if let Some(lst) = find_mut_by_local(root, "sectionLst") {
        remove_sid_from_section_lst(lst, sid);
    }
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

fn section_has_sid(sec: &Element, sid: u32) -> bool {
    let Some(lst) = sec.child_by_local("sldIdLst") else {
        return false;
    };
    lst.element_children()
        .filter(|e| e.local_name() == "sldId")
        .any(|e| e.attr("id").and_then(|s| s.parse::<u32>().ok()) == Some(sid))
}

fn ensure_section_sld_lst(sec: &mut Element) -> &mut Element {
    if sec.child_by_local("sldIdLst").is_none() {
        sec.push_element(Element::new("p14:sldIdLst"));
    }
    sec.child_by_local_mut("sldIdLst").expect("刚插入 sldIdLst")
}

fn remove_sld_refs(el: &mut Element, sid: &str) {
    el.children.retain(|n| match n {
        Node::Element(e) => !(e.local_name() == "sld" && e.attr("id") == Some(sid)),
        _ => true,
    });
    for child in el.element_children_mut() {
        remove_sld_refs(child, sid);
    }
}

fn find_mut_by_local<'a>(el: &'a mut Element, local: &str) -> Option<&'a mut Element> {
    let path = find_local_path(el, local)?;
    Some(follow_path(el, &path))
}

fn find_local_path(el: &Element, local: &str) -> Option<Vec<usize>> {
    if el.local_name() == local {
        return Some(Vec::new());
    }
    for (i, n) in el.children.iter().enumerate() {
        let Node::Element(c) = n else {
            continue;
        };
        if let Some(mut path) = find_local_path(c, local) {
            path.insert(0, i);
            return Some(path);
        }
    }
    None
}

fn follow_path<'a>(el: &'a mut Element, path: &[usize]) -> &'a mut Element {
    let mut cur = el;
    for &i in path {
        cur = match &mut cur.children[i] {
            Node::Element(c) => c,
            Node::Text(_) => unreachable!("路径只指向元素"),
        };
    }
    cur
}

fn keep_shared(rel_type: &str) -> bool {
    rel_type.ends_with("/slideLayout")
        || rel_type.ends_with("/slideMaster")
        || rel_type.ends_with("/notesMaster")
        || rel_type.ends_with("/theme")
        || rel_type.ends_with("/themeOverride")
}

fn internal_rel_targets(pkg: &Package, part: &str) -> Result<Vec<(String, String)>, Error> {
    let rels = Relationships::from_package(pkg, part)?;
    let mut out = Vec::new();
    for r in &rels.rels {
        if r.is_external() {
            continue;
        }
        if let Ok(p) = resolve_rel_target(part, &r.target) {
            if pkg.contains(&p) {
                out.push((p, r.rel_type.clone()));
            }
        }
    }
    Ok(out)
}

fn drop_if_unused(
    pkg: &mut Package,
    cts: &mut ContentTypes,
    part: &str,
    rel_type: &str,
) -> Result<(), Error> {
    if keep_shared(rel_type) {
        return Ok(());
    }
    if picture::media_still_used(pkg, part)? {
        return Ok(());
    }
    let children = internal_rel_targets(pkg, part)?;
    pkg.remove(part);
    pkg.remove(&rels_part_name(part));
    cts.remove_override(part);
    for (child, ty) in children {
        drop_if_unused(pkg, cts, &child, &ty)?;
    }
    Ok(())
}
