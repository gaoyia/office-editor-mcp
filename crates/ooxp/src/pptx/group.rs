//! 把 `spTree` 直接子节点包进 / 解开 `p:grpSp`。组合本身不进 inspect 树。

use crate::error::{Error, ErrorKind};
use crate::xml::{Document, Element, Node};

use super::connector::{self, CxnTarget};
use super::picture;
use super::request::{GroupShapesOp, UngroupOp};
use super::shape::{self, ShapeKey, ShapeTarget};

#[derive(Debug, Clone)]
pub enum MemberId {
    Shape(ShapeTarget),
    Cxn(CxnTarget),
}

pub fn parse_member_id(id: &str, op_index: usize) -> Result<MemberId, Error> {
    if connector::looks_like_cxn_id(id) {
        Ok(MemberId::Cxn(connector::parse_cxn_id(id, op_index)?))
    } else {
        Ok(MemberId::Shape(shape::parse_shape_id(id, op_index)?))
    }
}

pub fn group(doc: &mut Document, op: &GroupShapesOp, op_index: usize) -> Result<(), Error> {
    if op.members.len() < 2 {
        return Err(Error::with_path(
            ErrorKind::GroupNeedsTwo,
            format!("ops[{op_index}].ids"),
        ));
    }
    let mut seen = Vec::new();
    for m in &op.members {
        match m {
            MemberId::Shape(t) if t.slide != op.slide || t.on_notes => {
                return Err(Error::with_path(
                    ErrorKind::GroupMemberNested,
                    format!("ops[{op_index}].ids"),
                ));
            }
            MemberId::Cxn(t) if t.slide != op.slide => {
                return Err(Error::with_path(
                    ErrorKind::GroupMemberNested,
                    format!("ops[{op_index}].ids"),
                ));
            }
            _ => {}
        }
        let key = member_key(m);
        if seen.iter().any(|k| member_key_eq(k, &key)) {
            return Err(Error::with_path(
                ErrorKind::GroupNeedsTwo,
                format!("ops[{op_index}].ids"),
            ));
        }
        seen.push(key);
    }

    let mut indices = Vec::new();
    let mut boxes = Vec::new();
    {
        let root = &doc.root;
        for (k, m) in op.members.iter().enumerate() {
            let el = resolve(root, m).ok_or_else(|| member_not_found(&op.ids[k], op_index))?;
            let Some(idx) = direct_child_index(root, el) else {
                return Err(Error::with_path(
                    ErrorKind::GroupMemberNested,
                    format!("ops[{op_index}].ids"),
                ));
            };
            if let Some(b) = emu_box(el) {
                boxes.push(b);
            }
            indices.push(idx);
        }
    }
    if indices.len() < 2 {
        return Err(Error::with_path(
            ErrorKind::GroupNeedsTwo,
            format!("ops[{op_index}].ids"),
        ));
    }

    let insert_at = *indices.iter().min().expect("至少两个");
    let cnv = picture::next_cnv_id(&doc.root);
    let members = {
        let tree = picture::sp_tree_mut(&mut doc.root).ok_or_else(|| {
            Error::new(ErrorKind::XmlMissing {
                parent: "p:sld".into(),
                child: "p:spTree".into(),
            })
        })?;
        let mut taken: Vec<(usize, Node)> = Vec::new();
        let mut sorted_idx = indices.clone();
        sorted_idx.sort_unstable();
        sorted_idx.reverse();
        for idx in sorted_idx {
            taken.push((idx, tree.children.remove(idx)));
        }
        taken.sort_by_key(|(i, _)| *i);
        taken.into_iter().map(|(_, n)| n).collect::<Vec<_>>()
    };

    let (x, y, cx, cy) = union_boxes(&boxes);
    let mut grp = Element::new("p:grpSp");
    let mut nv = Element::new("p:nvGrpSpPr");
    nv.push_element(
        Element::new("p:cNvPr")
            .with_attr("id", cnv.to_string())
            .with_attr("name", format!("组合 {cnv}")),
    );
    nv.push_element(Element::new("p:cNvGrpSpPr"));
    nv.push_element(Element::new("p:nvPr"));
    grp.push_element(nv);
    let mut grp_pr = Element::new("p:grpSpPr");
    let mut xfrm = Element::new("a:xfrm");
    xfrm.push_element(
        Element::new("a:off")
            .with_attr("x", x.to_string())
            .with_attr("y", y.to_string()),
    );
    xfrm.push_element(
        Element::new("a:ext")
            .with_attr("cx", cx.to_string())
            .with_attr("cy", cy.to_string()),
    );
    xfrm.push_element(
        Element::new("a:chOff")
            .with_attr("x", x.to_string())
            .with_attr("y", y.to_string()),
    );
    xfrm.push_element(
        Element::new("a:chExt")
            .with_attr("cx", cx.to_string())
            .with_attr("cy", cy.to_string()),
    );
    grp_pr.push_element(xfrm);
    grp.push_element(grp_pr);
    for n in members {
        grp.children.push(n);
    }

    let tree = picture::sp_tree_mut(&mut doc.root).expect("刚才还在");
    let at = insert_at.min(tree.children.len());
    tree.children.insert(at, Node::Element(grp));
    Ok(())
}

pub fn ungroup(doc: &mut Document, op: &UngroupOp, op_index: usize) -> Result<(), Error> {
    let path =
        find_path(&doc.root, &op.member).ok_or_else(|| member_not_found(&op.id, op_index))?;
    let mut grp_depth = None;
    let mut cur = &doc.root;
    for (d, &i) in path.iter().enumerate() {
        let Node::Element(e) = cur
            .children
            .get(i)
            .ok_or_else(|| member_not_found(&op.id, op_index))?
        else {
            return Err(member_not_found(&op.id, op_index));
        };
        if e.local_name() == "grpSp" {
            grp_depth = Some(d);
        }
        cur = e;
    }
    let Some(grp_depth) = grp_depth else {
        return Err(Error::with_path(
            ErrorKind::NotGrouped,
            format!("ops[{op_index}].id"),
        ));
    };
    let parent_path = &path[..grp_depth];
    let grp_index = path[grp_depth];
    let parent =
        at_mut(&mut doc.root, parent_path).ok_or_else(|| member_not_found(&op.id, op_index))?;
    let Node::Element(grp) = parent.children.remove(grp_index) else {
        return Err(member_not_found(&op.id, op_index));
    };
    let members: Vec<Node> = grp
        .children
        .into_iter()
        .filter(|n| match n {
            Node::Element(e) => !matches!(e.local_name(), "nvGrpSpPr" | "grpSpPr"),
            _ => true,
        })
        .collect();
    for (k, n) in members.into_iter().enumerate() {
        parent.children.insert(grp_index + k, n);
    }
    Ok(())
}

fn member_not_found(id: &str, op_index: usize) -> Error {
    if connector::looks_like_cxn_id(id) {
        Error::with_path(
            ErrorKind::ConnectorNotFound { id: id.to_string() },
            format!("ops[{op_index}].id"),
        )
    } else {
        Error::with_path(
            ErrorKind::ShapeNotFound { id: id.to_string() },
            format!("ops[{op_index}].id"),
        )
    }
}

fn member_key(m: &MemberId) -> MemberKey {
    match m {
        MemberId::Shape(t) => MemberKey::Shape(t.key.clone()),
        MemberId::Cxn(t) => MemberKey::Cxn(t.key.clone()),
    }
}

fn member_key_eq(a: &MemberKey, b: &MemberKey) -> bool {
    match (a, b) {
        (MemberKey::Shape(x), MemberKey::Shape(y)) | (MemberKey::Cxn(x), MemberKey::Cxn(y)) => {
            match (x, y) {
                (ShapeKey::Id(a), ShapeKey::Id(b)) => shape::ids_match(a, b),
                (ShapeKey::Index(a), ShapeKey::Index(b)) => a == b,
                _ => false,
            }
        }
        _ => false,
    }
}

#[derive(Clone)]
enum MemberKey {
    Shape(ShapeKey),
    Cxn(ShapeKey),
}

fn resolve<'a>(root: &'a Element, member: &MemberId) -> Option<&'a Element> {
    match member {
        MemberId::Shape(t) => {
            let all = shape::collect_text_shapes(root);
            match &t.key {
                ShapeKey::Index(n) => all.get(n.saturating_sub(1)).copied(),
                ShapeKey::Id(req) => all.into_iter().find(|e| {
                    shape::cnv_pr(e)
                        .and_then(|c| c.attr("id"))
                        .is_some_and(|id| shape::ids_match(id, req))
                }),
            }
        }
        MemberId::Cxn(t) => {
            let all = connector::collect(root);
            match &t.key {
                ShapeKey::Index(n) => all.get(n.saturating_sub(1)).copied(),
                ShapeKey::Id(req) => all.into_iter().find(|e| {
                    e.child_by_local("nvCxnSpPr")
                        .and_then(|nv| nv.child_by_local("cNvPr"))
                        .and_then(|c| c.attr("id"))
                        .is_some_and(|id| shape::ids_match(id, req))
                }),
            }
        }
    }
}

fn direct_child_index(root: &Element, target: &Element) -> Option<usize> {
    let tree = picture::sp_tree(root)?;
    tree.children.iter().enumerate().find_map(|(i, n)| match n {
        Node::Element(e) if std::ptr::eq(e, target) => Some(i),
        _ => None,
    })
}

fn emu_box(el: &Element) -> Option<(i64, i64, i64, i64)> {
    let xfrm = el
        .child_by_local("spPr")
        .and_then(|p| p.child_by_local("xfrm"))?;
    let off = xfrm.child_by_local("off")?;
    let ext = xfrm.child_by_local("ext")?;
    Some((
        off.attr("x")?.parse().ok()?,
        off.attr("y")?.parse().ok()?,
        ext.attr("cx")?.parse().ok()?,
        ext.attr("cy")?.parse().ok()?,
    ))
}

fn union_boxes(boxes: &[(i64, i64, i64, i64)]) -> (i64, i64, i64, i64) {
    if boxes.is_empty() {
        return (0, 0, 0, 0);
    }
    let min_x = boxes.iter().map(|b| b.0).min().unwrap_or(0);
    let min_y = boxes.iter().map(|b| b.1).min().unwrap_or(0);
    let max_x = boxes
        .iter()
        .map(|b| b.0.saturating_add(b.2))
        .max()
        .unwrap_or(0);
    let max_y = boxes
        .iter()
        .map(|b| b.1.saturating_add(b.3))
        .max()
        .unwrap_or(0);
    (
        min_x,
        min_y,
        max_x.saturating_sub(min_x),
        max_y.saturating_sub(min_y),
    )
}

fn find_path(root: &Element, member: &MemberId) -> Option<Vec<usize>> {
    let mut acc = Vec::new();
    let mut shape_idx = 0usize;
    let mut cxn_idx = 0usize;
    if rec_path(root, member, &mut shape_idx, &mut cxn_idx, &mut acc) {
        Some(acc)
    } else {
        None
    }
}

fn rec_path(
    el: &Element,
    member: &MemberId,
    shape_idx: &mut usize,
    cxn_idx: &mut usize,
    acc: &mut Vec<usize>,
) -> bool {
    if el.local_name() == "AlternateContent" {
        if let Some(i) = shape::first_alt_child_index(el) {
            acc.push(i);
            let hit = match &el.children[i] {
                Node::Element(child) => rec_path(child, member, shape_idx, cxn_idx, acc),
                _ => false,
            };
            if !hit {
                acc.pop();
            }
            return hit;
        }
        return false;
    }
    if matches_member(el, member, shape_idx, cxn_idx) {
        return true;
    }
    for (i, n) in el.children.iter().enumerate() {
        if let Node::Element(e) = n {
            acc.push(i);
            if rec_path(e, member, shape_idx, cxn_idx, acc) {
                return true;
            }
            acc.pop();
        }
    }
    false
}

fn matches_member(
    el: &Element,
    member: &MemberId,
    shape_idx: &mut usize,
    cxn_idx: &mut usize,
) -> bool {
    match member {
        MemberId::Shape(t) => {
            if el.local_name() == "sp" && el.child_by_local("txBody").is_some() {
                *shape_idx += 1;
                match &t.key {
                    ShapeKey::Index(n) => *shape_idx == *n,
                    ShapeKey::Id(req) => shape::cnv_pr(el)
                        .and_then(|c| c.attr("id"))
                        .is_some_and(|id| shape::ids_match(id, req)),
                }
            } else {
                false
            }
        }
        MemberId::Cxn(t) => {
            if el.local_name() == "cxnSp" {
                *cxn_idx += 1;
                match &t.key {
                    ShapeKey::Index(n) => *cxn_idx == *n,
                    ShapeKey::Id(req) => el
                        .child_by_local("nvCxnSpPr")
                        .and_then(|nv| nv.child_by_local("cNvPr"))
                        .and_then(|c| c.attr("id"))
                        .is_some_and(|id| shape::ids_match(id, req)),
                }
            } else {
                false
            }
        }
    }
}

fn at_mut<'a>(el: &'a mut Element, path: &[usize]) -> Option<&'a mut Element> {
    if path.is_empty() {
        return Some(el);
    }
    let (first, rest) = path.split_first()?;
    let Node::Element(child) = el.children.get_mut(*first)? else {
        return None;
    };
    at_mut(child, rest)
}
