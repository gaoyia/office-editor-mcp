//! 把工作表 drawing 锚点上的直接子对象包进 / 解开 `xdr:grpSp`。组合本身不进 inspect 树。

use crate::error::{Error, ErrorKind};
use crate::opc::Package;
use crate::xml::{Document, Element, Node};

use super::layout;
use super::picture::{self, AnchorKind};
use super::request::{GroupShapesOp, UngroupOp};
use super::shape::{self, DrawKey, DrawObjKind, SheetDrawId};

#[derive(Clone, Copy)]
struct EmuBox {
    x: i64,
    y: i64,
    cx: i64,
    cy: i64,
}

pub fn group_shapes(
    pkg: &mut Package,
    sheet_doc: &mut Document,
    sheet_part: &str,
    op: &GroupShapesOp,
    op_index: usize,
) -> Result<(), Error> {
    if op.members.len() < 2 {
        return Err(Error::with_path(
            ErrorKind::GroupNeedsTwo,
            format!("ops[{op_index}].ids"),
        ));
    }
    let mut seen: Vec<(String, DrawObjKind, DrawKey)> = Vec::new();
    for (k, (id, kind)) in op.members.iter().enumerate() {
        if !id.sheet.eq_ignore_ascii_case(&op.sheet) {
            return Err(Error::with_path(
                ErrorKind::GroupMemberNested,
                format!("ops[{op_index}].ids[{k}]"),
            ));
        }
        let key = (id.sheet.clone(), *kind, id.key.clone());
        if seen.iter().any(|s| member_eq(s, &key)) {
            return Err(Error::with_path(
                ErrorKind::GroupNeedsTwo,
                format!("ops[{op_index}].ids"),
            ));
        }
        seen.push(key);
    }

    let parts = picture::drawing_parts_from(pkg, sheet_part, &sheet_doc.root)?;
    let mut hits: Vec<Hit> = Vec::new();
    for (k, (id, kind)) in op.members.iter().enumerate() {
        let hit = locate_member(pkg, &parts, id, *kind)?.ok_or_else(|| {
            Error::with_path(
                kind.not_found(op.ids[k].clone()),
                format!("ops[{op_index}].ids[{k}]"),
            )
        })?;
        if hit.nested {
            return Err(Error::with_path(
                ErrorKind::GroupMemberNested,
                format!("ops[{op_index}].ids[{k}]"),
            ));
        }
        hits.push(hit);
    }
    let part = hits[0].part.clone();
    if hits.iter().any(|h| h.part != part) {
        return Err(Error::with_path(
            ErrorKind::GroupMemberNested,
            format!("ops[{op_index}].ids"),
        ));
    }

    let mut draw_doc = super::list_pin::parse_part(pkg, &part)?;
    picture::ensure_xmlns(&mut draw_doc.root, "a", picture::A_NS);

    let mut pieces: Vec<(usize, Element, EmuBox)> = Vec::new();
    for hit in &hits {
        let payload = direct_payload(&draw_doc.root, hit.anchor_index, hit.kind, &hit.key)
            .ok_or_else(|| {
                Error::with_path(
                    hit.kind.not_found(String::new()),
                    format!("ops[{op_index}].ids"),
                )
            })?;
        let bbox =
            anchor_box(&sheet_doc.root, &draw_doc.root, hit.anchor_index)?.unwrap_or(EmuBox {
                x: 0,
                y: 0,
                cx: shape::DEFAULT_CX,
                cy: shape::DEFAULT_CY,
            });
        pieces.push((hit.anchor_index, payload, bbox));
    }

    let union = union_boxes(pieces.iter().map(|p| p.2));
    let mut members_xml = Vec::new();
    for (_, mut payload, bbox) in pieces.iter().cloned() {
        set_body_xfrm(&mut payload, bbox.x, bbox.y, bbox.cx, bbox.cy);
        members_xml.push(payload);
    }

    let cnv = picture::next_cnv_id(pkg, sheet_part, &sheet_doc.root)?;
    let grp = build_grp_sp(cnv, union, members_xml);
    let (from, from_off_x, from_off_y) = layout::emu_to_marker(&sheet_doc.root, union.x, union.y);
    let (to, to_off_x, to_off_y) = layout::emu_to_marker(
        &sheet_doc.root,
        union.x.saturating_add(union.cx),
        union.y.saturating_add(union.cy),
    );
    let mut anchor = Element::new("xdr:twoCellAnchor");
    anchor.push_element(picture::marker_el("xdr:from", from, from_off_x, from_off_y));
    anchor.push_element(picture::marker_el("xdr:to", to, to_off_x, to_off_y));
    anchor.push_element(grp);
    anchor.push_element(Element::new("xdr:clientData"));

    let mut drop: Vec<usize> = pieces.iter().map(|p| p.0).collect();
    drop.sort_unstable();
    drop.dedup();
    let insert_at = *drop.first().unwrap_or(&0);
    drop.reverse();
    for idx in drop {
        if idx < draw_doc.root.children.len() {
            draw_doc.root.children.remove(idx);
        }
    }
    let at = insert_at.min(draw_doc.root.children.len());
    draw_doc.root.children.insert(at, Node::Element(anchor));
    super::list_pin::write_part(pkg, &part, &draw_doc)?;
    Ok(())
}

pub fn ungroup(
    pkg: &mut Package,
    sheet_doc: &mut Document,
    sheet_part: &str,
    op: &UngroupOp,
    op_index: usize,
) -> Result<(), Error> {
    let parts = picture::drawing_parts_from(pkg, sheet_part, &sheet_doc.root)?;
    let hit = locate_member(pkg, &parts, &op.target, op.kind)?.ok_or_else(|| {
        Error::with_path(
            op.kind.not_found(op.id.clone()),
            format!("ops[{op_index}].id"),
        )
    })?;
    if !hit.nested {
        return Err(Error::with_path(
            ErrorKind::NotGrouped,
            format!("ops[{op_index}].id"),
        ));
    }
    let mut draw_doc = super::list_pin::parse_part(pkg, &hit.part)?;
    let Some(anchor) = anchor_at_mut(&mut draw_doc.root, hit.anchor_index) else {
        return Err(Error::with_path(
            op.kind.not_found(op.id.clone()),
            format!("ops[{op_index}].id"),
        ));
    };
    let Some(grp_idx) = innermost_grp_index(anchor, &op.target.key, op.kind) else {
        return Err(Error::with_path(
            ErrorKind::NotGrouped,
            format!("ops[{op_index}].id"),
        ));
    };
    let Node::Element(grp) = anchor.children.remove(grp_idx) else {
        return Err(Error::with_path(
            ErrorKind::NotGrouped,
            format!("ops[{op_index}].id"),
        ));
    };
    let mut grp = grp;
    let children: Vec<Element> = grp
        .take_children()
        .into_iter()
        .filter_map(|n| match n {
            Node::Element(e) if !is_group_chrome(e.local_name()) => Some(e),
            _ => None,
        })
        .collect();
    if children.is_empty() {
        if !anchor_has_other_payload(anchor) {
            draw_doc.root.children.remove(hit.anchor_index);
        }
        super::list_pin::write_part(pkg, &hit.part, &draw_doc)?;
        return Ok(());
    }
    let mut new_anchors = Vec::new();
    for child in children {
        let bbox = xfrm_box(&child).unwrap_or(EmuBox {
            x: 0,
            y: 0,
            cx: shape::DEFAULT_CX,
            cy: shape::DEFAULT_CY,
        });
        new_anchors.push(anchor_from_box(&sheet_doc.root, child, bbox));
    }
    draw_doc.root.children.remove(hit.anchor_index);
    let at = hit.anchor_index.min(draw_doc.root.children.len());
    for (offset, a) in new_anchors.into_iter().enumerate() {
        draw_doc.root.children.insert(at + offset, Node::Element(a));
    }
    super::list_pin::write_part(pkg, &hit.part, &draw_doc)?;
    Ok(())
}

fn member_eq(a: &(String, DrawObjKind, DrawKey), b: &(String, DrawObjKind, DrawKey)) -> bool {
    a.0.eq_ignore_ascii_case(&b.0) && a.1 == b.1 && keys_eq(&a.2, &b.2)
}

fn keys_eq(a: &DrawKey, b: &DrawKey) -> bool {
    match (a, b) {
        (DrawKey::Index(x), DrawKey::Index(y)) => x == y,
        (DrawKey::Cnv(x), DrawKey::Cnv(y)) => {
            if let (Ok(m), Ok(n)) = (x.parse::<u32>(), y.parse::<u32>()) {
                m == n
            } else {
                x == y
            }
        }
        _ => false,
    }
}

struct Hit {
    part: String,
    anchor_index: usize,
    nested: bool,
    kind: DrawObjKind,
    key: DrawKey,
}

fn locate_member(
    pkg: &Package,
    parts: &[String],
    id: &SheetDrawId,
    kind: DrawObjKind,
) -> Result<Option<Hit>, Error> {
    for part in parts {
        let doc = super::list_pin::parse_part(pkg, part)?;
        if let Some((anchor_index, nested)) = locate_in(&doc.root, kind, &id.key) {
            return Ok(Some(Hit {
                part: part.clone(),
                anchor_index,
                nested,
                kind,
                key: id.key.clone(),
            }));
        }
    }
    Ok(None)
}

fn locate_in(root: &Element, kind: DrawObjKind, key: &DrawKey) -> Option<(usize, bool)> {
    let is_obj = kind.matcher();
    let mut index = 0usize;
    let mut nested = None;
    let mut direct = None;
    for (i, n) in root.children.iter().enumerate() {
        let Node::Element(child) = n else {
            continue;
        };
        let host = if child.local_name() == "AlternateContent" {
            first_choice(child)
        } else {
            Some(child)
        };
        let Some(host) = host else {
            continue;
        };
        if AnchorKind::from_local(host.local_name()).is_none() {
            continue;
        }
        for p in host.element_children() {
            if is_geom(p.local_name()) {
                continue;
            }
            if p.local_name() == "grpSp" {
                if scan_grp(p, is_obj, key, &mut index) {
                    nested = Some(i);
                }
            } else if is_obj(p) {
                index += 1;
                if shape::key_matches(p, key, &shape::cnv_of_object(p).0, index) {
                    direct = Some(i);
                }
            }
        }
    }
    if let Some(i) = direct {
        Some((i, false))
    } else {
        nested.map(|i| (i, true))
    }
}

fn scan_grp(grp: &Element, is_obj: fn(&Element) -> bool, key: &DrawKey, index: &mut usize) -> bool {
    let mut hit = false;
    for c in grp.element_children() {
        if is_group_chrome(c.local_name()) {
            continue;
        }
        if c.local_name() == "grpSp" {
            if scan_grp(c, is_obj, key, index) {
                hit = true;
            }
        } else if is_obj(c) {
            *index += 1;
            if shape::key_matches(c, key, &shape::cnv_of_object(c).0, *index) {
                hit = true;
            }
        }
    }
    hit
}

fn direct_payload(
    root: &Element,
    anchor_index: usize,
    kind: DrawObjKind,
    key: &DrawKey,
) -> Option<Element> {
    let host = anchor_host(root, anchor_index)?;
    let is_obj = kind.matcher();
    let mut index = 0usize;
    // 与 locate_in 相同的全局序号：先扫过前面的锚点。
    for (i, n) in root.children.iter().enumerate() {
        if i >= anchor_index {
            break;
        }
        let Node::Element(child) = n else {
            continue;
        };
        let Some(h) = (if child.local_name() == "AlternateContent" {
            first_choice(child)
        } else {
            Some(child)
        }) else {
            continue;
        };
        if AnchorKind::from_local(h.local_name()).is_none() {
            continue;
        }
        bump_index(h, is_obj, &mut index);
    }
    for p in host.element_children() {
        if is_geom(p.local_name()) || p.local_name() == "grpSp" {
            if p.local_name() == "grpSp" {
                bump_index(p, is_obj, &mut index);
            }
            continue;
        }
        if is_obj(p) {
            index += 1;
            if shape::key_matches(p, key, &shape::cnv_of_object(p).0, index) {
                return Some(p.clone());
            }
        }
    }
    None
}

fn bump_index(el: &Element, is_obj: fn(&Element) -> bool, index: &mut usize) {
    if el.local_name() == "grpSp" {
        for c in el.element_children() {
            if is_group_chrome(c.local_name()) {
                continue;
            }
            bump_index(c, is_obj, index);
        }
        return;
    }
    if is_obj(el) {
        *index += 1;
    } else if AnchorKind::from_local(el.local_name()).is_some() {
        for c in el.element_children() {
            if is_geom(c.local_name()) {
                continue;
            }
            bump_index(c, is_obj, index);
        }
    }
}

fn anchor_host(root: &Element, index: usize) -> Option<&Element> {
    let Node::Element(child) = root.children.get(index)? else {
        return None;
    };
    if child.local_name() == "AlternateContent" {
        first_choice(child)
    } else {
        Some(child)
    }
}

fn anchor_at_mut(root: &mut Element, index: usize) -> Option<&mut Element> {
    let Node::Element(child) = root.children.get_mut(index)? else {
        return None;
    };
    if child.local_name() == "AlternateContent" {
        child.child_by_local_mut("Choice")
    } else {
        Some(child)
    }
}

fn innermost_grp_index(anchor: &Element, key: &DrawKey, kind: DrawObjKind) -> Option<usize> {
    let is_obj = kind.matcher();
    for (i, n) in anchor.children.iter().enumerate() {
        let Node::Element(e) = n else {
            continue;
        };
        if e.local_name() != "grpSp" {
            continue;
        }
        if grp_contains(e, is_obj, key) {
            return Some(i);
        }
    }
    None
}

fn grp_contains(grp: &Element, is_obj: fn(&Element) -> bool, key: &DrawKey) -> bool {
    let mut index = 0usize;
    scan_grp(grp, is_obj, key, &mut index)
}

fn anchor_has_other_payload(anchor: &Element) -> bool {
    anchor.element_children().any(|c| {
        !is_geom(c.local_name()) && c.local_name() != "grpSp" && !is_group_chrome(c.local_name())
    })
}

fn anchor_box(
    sheet: &Element,
    root: &Element,
    anchor_index: usize,
) -> Result<Option<EmuBox>, Error> {
    let Some(host) = anchor_host(root, anchor_index) else {
        return Ok(None);
    };
    let Some(kind) = AnchorKind::from_local(host.local_name()) else {
        return Ok(None);
    };
    match kind {
        AnchorKind::Absolute => {
            let (x, y) = picture::read_pos(host);
            let (cx, cy) = picture::read_ext(host);
            Ok(Some(EmuBox {
                x: x.unwrap_or(0),
                y: y.unwrap_or(0),
                cx: cx.unwrap_or(shape::DEFAULT_CX),
                cy: cy.unwrap_or(shape::DEFAULT_CY),
            }))
        }
        AnchorKind::OneCell => {
            let (from, off_x, off_y) = picture::read_marker(host.child_by_local("from"))?;
            let Some(from) = from else {
                return Ok(None);
            };
            let (cx, cy) = picture::read_ext(host);
            let (x, y) = (
                layout::col_origin_emu(sheet, from.col).saturating_add(off_x),
                layout::row_origin_emu(sheet, from.row).saturating_add(off_y),
            );
            Ok(Some(EmuBox {
                x,
                y,
                cx: cx.unwrap_or(shape::DEFAULT_CX),
                cy: cy.unwrap_or(shape::DEFAULT_CY),
            }))
        }
        AnchorKind::TwoCell => {
            let (from, off_x, off_y) = picture::read_marker(host.child_by_local("from"))?;
            let (to, to_x, to_y) = picture::read_marker(host.child_by_local("to"))?;
            let Some(from) = from else {
                return Ok(None);
            };
            let to = to.unwrap_or(from);
            let x = layout::col_origin_emu(sheet, from.col).saturating_add(off_x);
            let y = layout::row_origin_emu(sheet, from.row).saturating_add(off_y);
            let x2 = layout::col_origin_emu(sheet, to.col).saturating_add(to_x);
            let y2 = layout::row_origin_emu(sheet, to.row).saturating_add(to_y);
            Ok(Some(EmuBox {
                x,
                y,
                cx: (x2 - x).max(1),
                cy: (y2 - y).max(1),
            }))
        }
        AnchorKind::RelSize | AnchorKind::AbsSize => Ok(None),
    }
}

fn xfrm_box(el: &Element) -> Option<EmuBox> {
    let xfrm = if el.local_name() == "grpSp" {
        el.child_by_local("grpSpPr")
            .and_then(|p| p.child_by_local("xfrm"))
    } else {
        el.child_by_local("spPr")
            .and_then(|p| p.child_by_local("xfrm"))
    }?;
    let off = xfrm.child_by_local("off")?;
    let ext = xfrm.child_by_local("ext")?;
    Some(EmuBox {
        x: off.attr("x")?.parse().ok()?,
        y: off.attr("y")?.parse().ok()?,
        cx: ext.attr("cx")?.parse().ok()?,
        cy: ext.attr("cy")?.parse().ok()?,
    })
}

fn set_body_xfrm(el: &mut Element, x: i64, y: i64, cx: i64, cy: i64) {
    let pr_name = if el.local_name() == "grpSp" {
        "xdr:grpSpPr"
    } else {
        "xdr:spPr"
    };
    let local = if el.local_name() == "grpSp" {
        "grpSpPr"
    } else {
        "spPr"
    };
    if el.child_by_local(local).is_none() {
        el.children.insert(
            1.min(el.children.len()),
            Node::Element(Element::new(pr_name)),
        );
    }
    let Some(pr) = el.child_by_local_mut(local) else {
        return;
    };
    if pr.child_by_local("xfrm").is_none() {
        pr.children.insert(0, Node::Element(Element::new("a:xfrm")));
    }
    let Some(xfrm) = pr.child_by_local_mut("xfrm") else {
        return;
    };
    write_off_ext(xfrm, x, y, cx, cy);
}

fn write_off_ext(xfrm: &mut Element, x: i64, y: i64, cx: i64, cy: i64) {
    if let Some(off) = xfrm.child_by_local_mut("off") {
        picture::set_attr_local(off, "x", &x.to_string());
        picture::set_attr_local(off, "y", &y.to_string());
    } else {
        xfrm.children.insert(
            0,
            Node::Element(
                Element::new("a:off")
                    .with_attr("x", x.to_string())
                    .with_attr("y", y.to_string()),
            ),
        );
    }
    if let Some(ext) = xfrm.child_by_local_mut("ext") {
        picture::set_attr_local(ext, "cx", &cx.to_string());
        picture::set_attr_local(ext, "cy", &cy.to_string());
    } else {
        let off_i = xfrm
            .children
            .iter()
            .position(|n| matches!(n, Node::Element(e) if e.local_name() == "off"))
            .map(|i| i + 1)
            .unwrap_or(0);
        xfrm.children.insert(
            off_i,
            Node::Element(
                Element::new("a:ext")
                    .with_attr("cx", cx.to_string())
                    .with_attr("cy", cy.to_string()),
            ),
        );
    }
}

fn build_grp_sp(cnv: u32, box_: EmuBox, members: Vec<Element>) -> Element {
    let mut grp = Element::new("xdr:grpSp");
    let mut nv = Element::new("xdr:nvGrpSpPr");
    nv.push_element(
        Element::new("xdr:cNvPr")
            .with_attr("id", cnv.to_string())
            .with_attr("name", format!("组合 {cnv}")),
    );
    nv.push_element(Element::new("xdr:cNvGrpSpPr"));
    grp.push_element(nv);
    let mut grp_pr = Element::new("xdr:grpSpPr");
    let mut xfrm = Element::new("a:xfrm");
    xfrm.push_element(
        Element::new("a:off")
            .with_attr("x", box_.x.to_string())
            .with_attr("y", box_.y.to_string()),
    );
    xfrm.push_element(
        Element::new("a:ext")
            .with_attr("cx", box_.cx.to_string())
            .with_attr("cy", box_.cy.to_string()),
    );
    xfrm.push_element(
        Element::new("a:chOff")
            .with_attr("x", box_.x.to_string())
            .with_attr("y", box_.y.to_string()),
    );
    xfrm.push_element(
        Element::new("a:chExt")
            .with_attr("cx", box_.cx.to_string())
            .with_attr("cy", box_.cy.to_string()),
    );
    grp_pr.push_element(xfrm);
    grp.push_element(grp_pr);
    for m in members {
        grp.push_element(m);
    }
    grp
}

fn anchor_from_box(sheet: &Element, payload: Element, bbox: EmuBox) -> Element {
    let (from, from_off_x, from_off_y) = layout::emu_to_marker(sheet, bbox.x, bbox.y);
    let (to, to_off_x, to_off_y) = layout::emu_to_marker(
        sheet,
        bbox.x.saturating_add(bbox.cx),
        bbox.y.saturating_add(bbox.cy),
    );
    let mut anchor = Element::new("xdr:twoCellAnchor");
    anchor.push_element(picture::marker_el("xdr:from", from, from_off_x, from_off_y));
    anchor.push_element(picture::marker_el("xdr:to", to, to_off_x, to_off_y));
    anchor.push_element(payload);
    anchor.push_element(Element::new("xdr:clientData"));
    anchor
}

fn union_boxes(boxes: impl Iterator<Item = EmuBox>) -> EmuBox {
    let mut min_x = i64::MAX;
    let mut min_y = i64::MAX;
    let mut max_x = i64::MIN;
    let mut max_y = i64::MIN;
    let mut any = false;
    for b in boxes {
        any = true;
        min_x = min_x.min(b.x);
        min_y = min_y.min(b.y);
        max_x = max_x.max(b.x.saturating_add(b.cx));
        max_y = max_y.max(b.y.saturating_add(b.cy));
    }
    if !any {
        return EmuBox {
            x: 0,
            y: 0,
            cx: shape::DEFAULT_CX,
            cy: shape::DEFAULT_CY,
        };
    }
    EmuBox {
        x: min_x,
        y: min_y,
        cx: (max_x - min_x).max(1),
        cy: (max_y - min_y).max(1),
    }
}

fn is_geom(local: &str) -> bool {
    matches!(local, "from" | "to" | "ext" | "pos" | "clientData")
}

fn is_group_chrome(local: &str) -> bool {
    matches!(local, "nvGrpSpPr" | "grpSpPr")
}

fn first_choice(el: &Element) -> Option<&Element> {
    let index = crate::xml::alternate_child(el).unwrap_or(None)?;
    match el.children.get(index) {
        Some(Node::Element(child)) => Some(child),
        _ => None,
    }
}
