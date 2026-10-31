//! 幻灯片名单：插入空白页、删除、重排、隐藏。

use crate::error::{Error, ErrorKind};
use crate::opc::{
    rels_part_name, resolve_rel_target, ContentTypes, Package, Relationship, Relationships,
};
use crate::xml::{self, Document, Element, Node};

use super::custom_show;
use super::picture;
use super::request::{DeleteSlideOp, InsertSlideOp, MoveSlideOp, SetSlideHiddenOp};
use super::section;
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
        Some(after) => slide_pos(slides, after, op_index, "after")? + 1,
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
    let neighbor_sid = neighbor.and_then(|m| m.sld_id);
    let lst = sld_id_lst_mut(&mut pres.root)?;
    let sid = next_sld_id(lst);
    let tag = sld_id_tag(lst);
    let el = Element::new(tag)
        .with_attr("id", sid.to_string())
        .with_attr("r:id", rid);
    insert_sld_id(lst, insert_at, el);
    section::place_sid(
        &mut pres.root,
        sid,
        if op.after.is_some() {
            neighbor_sid
                .map(section::Place::After)
                .unwrap_or(section::Place::Append)
        } else {
            section::Place::Append
        },
    );
    let mut cts = ContentTypes::from_package(pkg)?;
    cts.set_override(&slide_part, CT_SLIDE)?;
    cts.write_to(pkg)?;
    super::list_pin::write_part(pkg, pres_part, pres)?;
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
    let sid = meta.sld_id.ok_or_else(|| {
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
    section::remove_sid(&mut pres.root, sid);
    custom_show::remove_rid(&mut pres.root, &rid);
    let mut cts = ContentTypes::from_package(pkg)?;
    pkg.remove(&meta.part);
    pkg.remove(&rels_part_name(&meta.part));
    cts.remove_override(&meta.part);
    for (part, rel_type) in children {
        drop_if_unused(pkg, &mut cts, &part, &rel_type)?;
    }
    cts.write_to(pkg)?;
    super::list_pin::write_part(pkg, pres_part, pres)?;
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
    let dest = match op.after {
        None => 0,
        Some(after) => {
            if after == op.slide {
                return Err(Error::with_path(
                    ErrorKind::MoveSlideAfterSelf,
                    format!("ops[{op_index}].after"),
                ));
            }
            slide_pos(slides, after, op_index, "after")? + 1
        }
    };
    if from + 1 == dest || (op.after.is_none() && from == 0) {
        return Ok(());
    }
    let sid = slides[from].sld_id.ok_or_else(|| {
        Error::new(ErrorKind::XmlMissing {
            parent: "sldIdLst".into(),
            child: "sldId".into(),
        })
    })?;
    let after_sid = op
        .after
        .and_then(|n| shape::slide_at_start(slides, n).and_then(|meta| meta.sld_id));
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
        None => section::Place::Front,
        Some(_) => after_sid
            .map(section::Place::After)
            .unwrap_or(section::Place::Front),
    };
    section::place_sid(&mut pres.root, sid, place);
    super::list_pin::write_part(pkg, pres_part, pres)?;
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
    let mut doc = super::list_pin::parse_part(pkg, &part)?;
    let currently = is_hidden(&doc.root);
    if currently == op.hidden {
        return Ok(());
    }
    if op.hidden {
        doc.root.set_attr("show", "0");
    } else {
        doc.root.remove_attr("show");
    }
    super::list_pin::write_part(pkg, &part, &doc)?;
    Ok(())
}

fn slide_pos(slides: &[SlideMeta], n: usize, op_index: usize, field: &str) -> Result<usize, Error> {
    shape::pos_at_start(slides, n).ok_or_else(|| {
        Error::with_path(
            ErrorKind::SlideNotFound { n: n as u32 },
            format!("ops[{op_index}].{field}"),
        )
    })
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
        Node::Text(_) | Node::Raw(_) => false,
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
