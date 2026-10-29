//! 把正文或页眉页脚里的浮动图 / 文本框 / 连接线 / 无框形状包进或解开 `wpg:wgp`。组合本身不进树。

use crate::error::{Error, ErrorKind, IdReason};
use crate::xml::{Element, Node};

use super::connector;
use super::hf::{HfKind, HfType};
use super::para::{self, HostPara, TxbxKey, TxbxTarget};
use super::picture::{self, PicKey, PicKind, PicTarget};
use super::shape;
use super::textbox::{self, WPG_NS, WPS_NS};
use super::PIC_NS;

#[derive(Debug, Clone)]
pub enum GroupHost {
    Document,
    HeaderFooter {
        kind: HfKind,
        hf_type: HfType,
        n: u32,
    },
}

#[derive(Debug, Clone)]
pub enum GroupMember {
    Picture(PicTarget),
    TextBox(TxbxTarget),
    Connector(TxbxTarget),
    Shape(TxbxTarget),
}

#[derive(Clone, Copy)]
enum ChildKind {
    Picture,
    TextBox,
    Connector,
    Shape,
}

struct Payload {
    child: Element,
    cx: i64,
    cy: i64,
    doc_pr: String,
    name: String,
}

pub fn parse_group_host(id: &str, op_index: usize) -> Result<GroupHost, Error> {
    parse_group_host_at(id, &format!("ops[{op_index}].id"))
}

pub fn parse_group_host_at(id: &str, path: &str) -> Result<GroupHost, Error> {
    if id == "docx:document" {
        return Ok(GroupHost::Document);
    }
    if let Some((kind, hf_type, n)) = para::parse_hf_container_id(id) {
        return Ok(GroupHost::HeaderFooter { kind, hf_type, n });
    }
    Err(Error::with_path(
        ErrorKind::InvalidId {
            reason: IdReason::GroupNeedsDocument,
        },
        path.to_string(),
    ))
}

pub fn parse_member_id(id: &str, op_index: usize) -> Result<GroupMember, Error> {
    parse_member_id_at(id, &format!("ops[{op_index}].id"))
}

pub fn parse_member_id_at(id: &str, path: &str) -> Result<GroupMember, Error> {
    let path = path.to_string();
    if super::ink::looks_like_ink_id(id) {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::DocxInkPrefix,
            },
            path,
        ));
    }
    if connector::looks_like_cxn_id(id) {
        return Ok(GroupMember::Connector(connector::parse_cxn_id_at(
            id, &path,
        )?));
    }
    if shape::looks_like_shape_id(id) {
        return Ok(GroupMember::Shape(shape::parse_shape_id_at(id, &path)?));
    }
    if id.contains("/txbx:") {
        let after = id.rsplit_once("/txbx:").map(|(_, a)| a).unwrap_or("");
        if after.contains('/') {
            return Err(Error::with_path(ErrorKind::GroupMemberNested, path));
        }
        return Ok(GroupMember::TextBox(para::parse_txbx_id_at(id, &path)?));
    }
    if id.contains("/pic:") {
        let target = picture::parse_pic_id_at(id, &path)?;
        match &target {
            PicTarget::Body { .. } | PicTarget::HeaderFooter { .. } => {
                Ok(GroupMember::Picture(target))
            }
            _ => Err(Error::with_path(ErrorKind::GroupMemberNested, path)),
        }
    } else {
        Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::PicPrefix,
            },
            path,
        ))
    }
}

pub fn group_shapes(
    root: &mut Element,
    host: &GroupHost,
    members: &[GroupMember],
    op_index: usize,
) -> Result<(), Error> {
    if members.len() < 2 {
        return Err(Error::with_path(
            ErrorKind::GroupNeedsTwo,
            format!("ops[{op_index}].ids"),
        ));
    }
    let mut seen: Vec<String> = Vec::new();
    let mut payloads: Vec<(usize, Payload)> = Vec::new();
    let paras = para_indices(root);
    for (k, member) in members.iter().enumerate() {
        let path = format!("ops[{op_index}].ids[{k}]");
        check_host(member, host).map_err(|e| e.at(path.as_str()))?;
        let key = member_key(member);
        if seen.iter().any(|s| s == &key) {
            return Err(Error::with_path(ErrorKind::GroupNeedsTwo, path));
        }
        seen.push(key);
        let (para_i, payload) = take_payload(root, member, &path)?;
        payloads.push((para_i, payload));
    }
    let first_para = payloads[0].0;
    let mut off_x = 0i64;
    let mut max_cy = 0i64;
    let mut children = Vec::new();
    for (_, mut payload) in payloads {
        stamp_cnv_pr(&mut payload.child, &payload.doc_pr, &payload.name);
        set_child_off(&mut payload.child, off_x, 0, payload.cx, payload.cy);
        off_x += payload.cx;
        max_cy = max_cy.max(payload.cy);
        children.push(payload.child);
    }
    let box_cx = off_x.max(1);
    let box_cy = max_cy.max(1);
    let group_id = picture::next_docpr_id(root);
    let ac = build_group_ac(group_id, box_cx, box_cy, children);
    let para =
        para::find_paragraph_mut(root, &para_of_index(&paras, first_para)).ok_or_else(|| {
            Error::with_path(
                ErrorKind::GroupMemberNested,
                format!("ops[{op_index}].ids[0]"),
            )
        })?;
    textbox::insert_ac(para, ac);
    Ok(())
}

pub fn ungroup(root: &mut Element, member: &GroupMember, op_index: usize) -> Result<(), Error> {
    let path = format!("ops[{op_index}].id");
    if !ungroup_walk(root, member) {
        if member_exists_ungrouped(root, member) {
            return Err(Error::with_path(ErrorKind::NotGrouped, path));
        }
        return Err(missing_member(member, &path));
    }
    Ok(())
}

fn para_indices(root: &Element) -> Vec<para::BodyPara> {
    para::collect_paragraphs(root)
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let (id, _) = para::para_stable_id(p, i + 1);
            if let Ok(para::ParaTarget::Body(b)) = para::parse_para_id_at(&id, "id") {
                b
            } else if let Ok(para::ParaTarget::HeaderFooter { para: b, .. }) =
                para::parse_para_id_at(&id, "id")
            {
                b
            } else {
                para::BodyPara::Index(i + 1)
            }
        })
        .collect()
}

fn para_of_index(all: &[para::BodyPara], i: usize) -> para::BodyPara {
    all.get(i).cloned().unwrap_or(para::BodyPara::Index(i + 1))
}

fn check_host(member: &GroupMember, host: &GroupHost) -> Result<(), Error> {
    let ok = match (member, host) {
        (GroupMember::Picture(PicTarget::Body { .. }), GroupHost::Document) => true,
        (
            GroupMember::Picture(PicTarget::HeaderFooter {
                kind, hf_type, n, ..
            }),
            GroupHost::HeaderFooter {
                kind: hk,
                hf_type: ht,
                n: hn,
            },
        ) => kind == hk && hf_type == ht && n == hn,
        (
            GroupMember::TextBox(t) | GroupMember::Connector(t) | GroupMember::Shape(t),
            GroupHost::Document,
        ) => {
            matches!(t.host, HostPara::Body(_))
        }
        (
            GroupMember::TextBox(t) | GroupMember::Connector(t) | GroupMember::Shape(t),
            GroupHost::HeaderFooter {
                kind, hf_type, n, ..
            },
        ) => matches!(
            &t.host,
            HostPara::HeaderFooter {
                kind: hk,
                hf_type: ht,
                n: hn,
                ..
            } if kind == hk && hf_type == ht && n == hn
        ),
        _ => false,
    };
    if ok {
        Ok(())
    } else {
        Err(Error::new(ErrorKind::GroupMemberNested))
    }
}

fn member_key(member: &GroupMember) -> String {
    match member {
        GroupMember::Picture(PicTarget::Body { para, key }) => {
            format!("pic-body-{}-{}", para_tag(para), key_tag(key))
        }
        GroupMember::Picture(PicTarget::HeaderFooter { key, .. }) => {
            format!("pic-hf-{}", key_tag(key))
        }
        GroupMember::Picture(_) => "pic-other".into(),
        GroupMember::TextBox(t) => format!("txbx-{}", txbx_tag(&t.key)),
        GroupMember::Connector(t) => format!("cxn-{}", txbx_tag(&t.key)),
        GroupMember::Shape(t) => format!("shape-{}", txbx_tag(&t.key)),
    }
}

fn para_tag(p: &para::BodyPara) -> String {
    match p {
        para::BodyPara::Index(n) => format!("i{n}"),
        para::BodyPara::Hex(s) => s.clone(),
    }
}

fn key_tag(k: &PicKey) -> String {
    match k {
        PicKey::DocPr(s) => format!("d{s}"),
        PicKey::Index(n) => format!("i{n}"),
    }
}

fn txbx_tag(k: &TxbxKey) -> String {
    match k {
        TxbxKey::DocPr(s) => format!("d{s}"),
        TxbxKey::Index(n) => format!("i{n}"),
    }
}

enum Extracted {
    Ok(Payload),
    Nested,
    Missing,
}

fn take_payload(
    root: &mut Element,
    member: &GroupMember,
    path: &str,
) -> Result<(usize, Payload), Error> {
    let n = para::collect_paragraphs(root).len();
    let mut found = None;
    for i in 0..n {
        let paras = para::collect_paragraphs(root);
        let p = paras[i];
        let hit = match member {
            GroupMember::Picture(PicTarget::Body { para, key }) => {
                if para::same_para(p, para, i + 1) {
                    extract_pic(p, key)
                } else {
                    Extracted::Missing
                }
            }
            GroupMember::Picture(PicTarget::HeaderFooter { key, .. }) => extract_pic(p, key),
            GroupMember::Picture(_) => Extracted::Missing,
            GroupMember::TextBox(t) => {
                if host_para_matches(&t.host, p, i + 1) {
                    extract_txbx(p, &t.key)
                } else {
                    Extracted::Missing
                }
            }
            GroupMember::Connector(t) => {
                if host_para_matches(&t.host, p, i + 1) {
                    extract_cxn(p, &t.key)
                } else {
                    Extracted::Missing
                }
            }
            GroupMember::Shape(t) => {
                if host_para_matches(&t.host, p, i + 1) {
                    extract_shape(p, &t.key)
                } else {
                    Extracted::Missing
                }
            }
        };
        match hit {
            Extracted::Ok(payload) => {
                found = Some((i, payload));
                break;
            }
            Extracted::Nested => {
                return Err(Error::with_path(ErrorKind::GroupMemberNested, path));
            }
            Extracted::Missing => {}
        }
    }
    let Some((i, payload)) = found else {
        return Err(missing_member(member, path));
    };
    if !remove_member(root, member) {
        return Err(missing_member(member, path));
    }
    Ok((i, payload))
}

fn host_para_matches(host: &HostPara, p: &Element, index: usize) -> bool {
    para::same_para(p, host.body(), index)
}

fn extract_pic(host: &Element, key: &PicKey) -> Extracted {
    let Some(drawing) = picture::find_drawing(host, key) else {
        return Extracted::Missing;
    };
    if textbox::is_group(drawing) || drawing_kind(drawing) != PicKind::Anchor {
        return Extracted::Nested;
    }
    let Some(pic) = find_pic_el(drawing) else {
        return Extracted::Missing;
    };
    let (cx, cy) = extent_of(drawing);
    let (doc_pr, name) = doc_pr_of(drawing);
    Extracted::Ok(Payload {
        child: pic.clone(),
        cx: cx.unwrap_or(1),
        cy: cy.unwrap_or(1),
        doc_pr: doc_pr.unwrap_or_else(|| "1".into()),
        name: name.unwrap_or_else(|| "图片".into()),
    })
}

fn extract_txbx(host: &Element, key: &TxbxKey) -> Extracted {
    if let Some(drawing) = find_txbx_drawing(host, key) {
        if drawing_kind(drawing) != PicKind::Anchor {
            return Extracted::Nested;
        }
        let Some(wsp) = find_wsp(drawing) else {
            return Extracted::Missing;
        };
        if connector::is_connector_wsp(wsp) {
            return Extracted::Missing;
        }
        let (cx, cy) = extent_of(drawing);
        let (doc_pr, name) = doc_pr_of(drawing);
        return Extracted::Ok(Payload {
            child: wsp.clone(),
            cx: cx.unwrap_or(1),
            cy: cy.unwrap_or(1),
            doc_pr: doc_pr.unwrap_or_else(|| "1".into()),
            name: name.unwrap_or_else(|| "文本框".into()),
        });
    }
    if txbx_in_group(host, key) {
        Extracted::Nested
    } else {
        Extracted::Missing
    }
}

fn extract_cxn(host: &Element, key: &TxbxKey) -> Extracted {
    let Some(drawing) = connector::find_connector(host, key) else {
        return Extracted::Missing;
    };
    if textbox::is_group(drawing) || drawing_kind(drawing) != PicKind::Anchor {
        return Extracted::Nested;
    }
    let Some(wsp) = find_wsp(drawing) else {
        return Extracted::Missing;
    };
    let (cx, cy) = extent_of(drawing);
    let (doc_pr, name) = doc_pr_of(drawing);
    Extracted::Ok(Payload {
        child: wsp.clone(),
        cx: cx.unwrap_or(connector::DEFAULT_CX),
        cy: cy.unwrap_or(connector::DEFAULT_CY),
        doc_pr: doc_pr.unwrap_or_else(|| "1".into()),
        name: name.unwrap_or_else(|| "连接线".into()),
    })
}

fn extract_shape(host: &Element, key: &TxbxKey) -> Extracted {
    let Some(drawing) = shape::find_shape(host, key) else {
        return Extracted::Missing;
    };
    if textbox::is_group(drawing) || drawing_kind(drawing) != PicKind::Anchor {
        return Extracted::Nested;
    }
    let Some(wsp) = find_wsp(drawing) else {
        return Extracted::Missing;
    };
    if !shape::is_shape_wsp(wsp) {
        return Extracted::Missing;
    }
    let (cx, cy) = extent_of(drawing);
    let (doc_pr, name) = doc_pr_of(drawing);
    Extracted::Ok(Payload {
        child: wsp.clone(),
        cx: cx.unwrap_or(shape::DEFAULT_CX),
        cy: cy.unwrap_or(shape::DEFAULT_CY),
        doc_pr: doc_pr.unwrap_or_else(|| "1".into()),
        name: name.unwrap_or_else(|| "形状".into()),
    })
}

fn remove_member(root: &mut Element, member: &GroupMember) -> bool {
    match member {
        GroupMember::Picture(target) => remove_pic_target(root, target),
        GroupMember::TextBox(t) => {
            let host = para::find_paragraph_mut(root, t.host.body());
            match host {
                Some(p) => textbox::remove_textbox(p, &t.key),
                None => false,
            }
        }
        GroupMember::Connector(t) => {
            let host = para::find_paragraph_mut(root, t.host.body());
            match host {
                Some(p) => connector::remove_connector(p, &t.key),
                None => false,
            }
        }
        GroupMember::Shape(t) => {
            let host = para::find_paragraph_mut(root, t.host.body());
            match host {
                Some(p) => shape::remove_shape(p, &t.key),
                None => false,
            }
        }
    }
}

fn remove_pic_target(root: &mut Element, target: &PicTarget) -> bool {
    match target {
        PicTarget::Body { para, key } => {
            let Some(p) = para::find_paragraph_mut(root, para) else {
                return false;
            };
            picture::remove_picture(p, key)
        }
        PicTarget::HeaderFooter { key, .. } => picture::remove_picture(root, key),
        _ => false,
    }
}

fn ungroup_walk(el: &mut Element, member: &GroupMember) -> bool {
    let mut i = 0;
    while i < el.children.len() {
        let local = match &el.children[i] {
            Node::Element(c) => c.local_name().to_string(),
            Node::Text(_) => {
                i += 1;
                continue;
            }
        };
        if (local == "AlternateContent" || local == "drawing")
            && slot_is_group_with_member(&el.children[i], member)
        {
            if let Some(replacements) = explode_group_node(&el.children[i]) {
                el.children.remove(i);
                for (k, ac) in replacements.into_iter().enumerate() {
                    el.children.insert(i + k, Node::Element(ac));
                }
                return true;
            }
        }
        if let Node::Element(child) = &mut el.children[i] {
            if ungroup_walk(child, member) {
                return true;
            }
        }
        i += 1;
    }
    false
}

fn slot_is_group_with_member(node: &Node, member: &GroupMember) -> bool {
    let Some(drawing) = drawing_of_slot(node) else {
        return false;
    };
    if !textbox::is_group(drawing) {
        return false;
    }
    match member {
        GroupMember::Picture(PicTarget::Body { key, .. })
        | GroupMember::Picture(PicTarget::HeaderFooter { key, .. }) => {
            picture::find_drawing(drawing, key).is_some()
                || picture::find_drawing(
                    match node {
                        Node::Element(e) => e,
                        Node::Text(_) => return false,
                    },
                    key,
                )
                .is_some()
        }
        GroupMember::Picture(_) => false,
        GroupMember::TextBox(t) => grouped_txbx(drawing, &t.key),
        GroupMember::Connector(t) => connector::find_connector(drawing, &t.key).is_some(),
        GroupMember::Shape(t) => shape::find_shape(drawing, &t.key).is_some(),
    }
}

fn grouped_txbx(drawing: &Element, key: &TxbxKey) -> bool {
    let mut index = 0usize;
    let mut hit = false;
    walk_all(drawing, &mut |e| {
        if e.local_name() == "wsp" && has_txbx_el(e) {
            index += 1;
            let (id, _) = cnv_pr_of(e);
            if para::matches_txbx_key(&id, index, key) {
                hit = true;
            }
        }
    });
    hit
}

fn txbx_in_group(host: &Element, key: &TxbxKey) -> bool {
    fn walk(el: &Element, key: &TxbxKey, hit: &mut bool) {
        if el.local_name() == "txbxContent" {
            return;
        }
        if el.local_name() == "drawing" {
            if textbox::is_group(el) && grouped_txbx(el, key) {
                *hit = true;
            }
            return;
        }
        if el.local_name() == "AlternateContent" {
            if let Some(i) = el
                .children
                .iter()
                .position(|n| matches!(n, Node::Element(e) if e.local_name() == "Choice"))
            {
                if let Node::Element(c) = &el.children[i] {
                    walk(c, key, hit);
                }
            }
            return;
        }
        for n in &el.children {
            if let Node::Element(e) = n {
                walk(e, key, hit);
            }
        }
    }
    let mut hit = false;
    walk(host, key, &mut hit);
    hit
}

fn explode_group_node(node: &Node) -> Option<Vec<Element>> {
    let drawing = drawing_of_slot(node)?;
    if !textbox::is_group(drawing) {
        return None;
    }
    let (cx, cy) = extent_of(drawing);
    let wrap_name = wrap_local(drawing);
    let mut children = Vec::new();
    collect_group_children(drawing, &mut children);
    if children.len() < 2 {
        return None;
    }
    let mut out = Vec::new();
    for child in children {
        out.push(wrap_independent(
            child,
            cx.unwrap_or(1),
            cy.unwrap_or(1),
            wrap_name,
        ));
    }
    Some(out)
}

fn collect_group_children(el: &Element, out: &mut Vec<(Element, ChildKind)>) {
    if el.local_name() == "txbxContent" {
        return;
    }
    if el.local_name() == "wsp" {
        if connector::is_connector_wsp(el) {
            out.push((el.clone(), ChildKind::Connector));
            return;
        }
        if has_txbx_el(el) {
            out.push((el.clone(), ChildKind::TextBox));
            return;
        }
        if shape::is_shape_wsp(el) {
            out.push((el.clone(), ChildKind::Shape));
            return;
        }
    }
    if el.local_name() == "pic" {
        out.push((el.clone(), ChildKind::Picture));
        return;
    }
    for n in &el.children {
        if let Node::Element(e) = n {
            collect_group_children(e, out);
        }
    }
}

fn wrap_independent(child: (Element, ChildKind), cx: i64, cy: i64, wrap: &str) -> Element {
    let (mut el, kind) = child;
    let (id, name) = cnv_pr_of(&el);
    let id = id.unwrap_or_else(|| "1".into());
    let name = name.unwrap_or_else(|| match kind {
        ChildKind::Picture => format!("图片 {id}"),
        ChildKind::TextBox => format!("文本框 {id}"),
        ChildKind::Connector => format!("连接线 {id}"),
        ChildKind::Shape => format!("形状 {id}"),
    });
    let (child_cx, child_cy) = xfrm_of(&el).unwrap_or((cx, cy));
    set_child_off(&mut el, 0, 0, child_cx, child_cy);
    match kind {
        ChildKind::Picture => wrap_pic_drawing(el, &id, &name, child_cx, child_cy, wrap),
        ChildKind::TextBox | ChildKind::Connector | ChildKind::Shape => {
            wrap_wps_ac(el, &id, &name, child_cx, child_cy, wrap)
        }
    }
}

fn wrap_pic_drawing(pic: Element, id: &str, name: &str, cx: i64, cy: i64, wrap: &str) -> Element {
    let mut drawing = Element::new("w:drawing");
    let mut frame = floating_anchor(cx, cy, wrap, id, name);
    let mut graphic = Element::new("a:graphic");
    let mut data = Element::new("a:graphicData").with_attr("uri", PIC_NS);
    data.push_element(pic);
    graphic.push_element(data);
    frame.push_element(graphic);
    drawing.push_element(frame);
    drawing
}

fn wrap_wps_ac(el: Element, id: &str, name: &str, cx: i64, cy: i64, wrap: &str) -> Element {
    let mut ac = Element::new("mc:AlternateContent");
    let mut choice = Element::new("mc:Choice").with_attr("Requires", "wps");
    let mut drawing = Element::new("w:drawing");
    let mut frame = floating_anchor(cx, cy, wrap, id, name);
    let mut graphic = Element::new("a:graphic");
    let mut data = Element::new("a:graphicData").with_attr("uri", WPS_NS);
    data.push_element(el);
    graphic.push_element(data);
    frame.push_element(graphic);
    drawing.push_element(frame);
    choice.push_element(drawing);
    ac.push_element(choice);
    let mut fallback = Element::new("mc:Fallback");
    fallback.push_element(Element::new("w:pict"));
    ac.push_element(fallback);
    ac
}

fn floating_anchor(cx: i64, cy: i64, wrap: &str, id: &str, name: &str) -> Element {
    let mut frame = Element::new("wp:anchor")
        .with_attr("distT", "0")
        .with_attr("distB", "0")
        .with_attr("distL", "114300")
        .with_attr("distR", "114300")
        .with_attr("simplePos", "0")
        .with_attr("relativeHeight", "251659264")
        .with_attr("behindDoc", "0")
        .with_attr("locked", "0")
        .with_attr("layoutInCell", "1")
        .with_attr("allowOverlap", "1");
    frame.push_element(
        Element::new("wp:simplePos")
            .with_attr("x", "0")
            .with_attr("y", "0"),
    );
    let mut pos_h = Element::new("wp:positionH").with_attr("relativeFrom", "margin");
    pos_h.push_element(Element::new("wp:align").with_text("left"));
    let mut pos_v = Element::new("wp:positionV").with_attr("relativeFrom", "margin");
    pos_v.push_element(Element::new("wp:align").with_text("top"));
    frame.push_element(pos_h);
    frame.push_element(pos_v);
    frame.push_element(
        Element::new("wp:extent")
            .with_attr("cx", cx.to_string())
            .with_attr("cy", cy.to_string()),
    );
    frame.push_element(
        Element::new("wp:effectExtent")
            .with_attr("l", "0")
            .with_attr("t", "0")
            .with_attr("r", "0")
            .with_attr("b", "0"),
    );
    let wrap_el = if wrap.is_empty() || wrap == "wrapSquare" {
        Element::new("wp:wrapSquare").with_attr("wrapText", "bothSides")
    } else if wrap == "wrapNone" {
        Element::new("wp:wrapNone")
    } else if wrap == "wrapTopAndBottom" {
        Element::new("wp:wrapTopAndBottom")
    } else {
        Element::new(format!("wp:{wrap}")).with_attr("wrapText", "bothSides")
    };
    frame.push_element(wrap_el);
    frame.push_element(
        Element::new("wp:docPr")
            .with_attr("id", id)
            .with_attr("name", name),
    );
    frame.push_element(Element::new("wp:cNvGraphicFramePr"));
    frame
}

fn member_exists_ungrouped(root: &Element, member: &GroupMember) -> bool {
    match member {
        GroupMember::Picture(PicTarget::Body { para, key }) => para::find_paragraph(root, para)
            .is_some_and(|p| picture::find_drawing(p, key).is_some_and(|d| !textbox::is_group(d))),
        GroupMember::Picture(PicTarget::HeaderFooter { key, .. }) => {
            picture::find_drawing(root, key).is_some_and(|d| !textbox::is_group(d))
        }
        GroupMember::Picture(_) => false,
        GroupMember::TextBox(t) => para::find_paragraph(root, t.host.body())
            .is_some_and(|p| find_txbx_drawing(p, &t.key).is_some_and(|d| !textbox::is_group(d))),
        GroupMember::Connector(t) => para::find_paragraph(root, t.host.body()).is_some_and(|p| {
            connector::find_connector(p, &t.key).is_some_and(|d| !textbox::is_group(d))
        }),
        GroupMember::Shape(t) => para::find_paragraph(root, t.host.body())
            .is_some_and(|p| shape::find_shape(p, &t.key).is_some_and(|d| !textbox::is_group(d))),
    }
}

fn missing_member(member: &GroupMember, path: &str) -> Error {
    match member {
        GroupMember::Picture(_) => Error::with_path(
            ErrorKind::PictureNotFound {
                id: path.to_string(),
            },
            path.to_string(),
        ),
        GroupMember::TextBox(_) => Error::with_path(
            ErrorKind::TextBoxNotFound {
                id: path.to_string(),
            },
            path.to_string(),
        ),
        GroupMember::Connector(_) => Error::with_path(
            ErrorKind::ConnectorNotFound {
                id: path.to_string(),
            },
            path.to_string(),
        ),
        GroupMember::Shape(_) => Error::with_path(
            ErrorKind::ShapeNotFound {
                id: path.to_string(),
            },
            path.to_string(),
        ),
    }
}

fn build_group_ac(doc_pr: u32, cx: i64, cy: i64, children: Vec<Element>) -> Element {
    let mut ac = Element::new("mc:AlternateContent");
    let mut choice = Element::new("mc:Choice").with_attr("Requires", "wpg");
    let mut drawing = Element::new("w:drawing");
    let mut frame = Element::new("wp:anchor")
        .with_attr("distT", "0")
        .with_attr("distB", "0")
        .with_attr("distL", "114300")
        .with_attr("distR", "114300")
        .with_attr("simplePos", "0")
        .with_attr("relativeHeight", "251659264")
        .with_attr("behindDoc", "0")
        .with_attr("locked", "0")
        .with_attr("layoutInCell", "1")
        .with_attr("allowOverlap", "1");
    frame.push_element(
        Element::new("wp:simplePos")
            .with_attr("x", "0")
            .with_attr("y", "0"),
    );
    let mut pos_h = Element::new("wp:positionH").with_attr("relativeFrom", "margin");
    pos_h.push_element(Element::new("wp:align").with_text("left"));
    let mut pos_v = Element::new("wp:positionV").with_attr("relativeFrom", "margin");
    pos_v.push_element(Element::new("wp:align").with_text("top"));
    frame.push_element(pos_h);
    frame.push_element(pos_v);
    frame.push_element(
        Element::new("wp:extent")
            .with_attr("cx", cx.to_string())
            .with_attr("cy", cy.to_string()),
    );
    frame.push_element(
        Element::new("wp:effectExtent")
            .with_attr("l", "0")
            .with_attr("t", "0")
            .with_attr("r", "0")
            .with_attr("b", "0"),
    );
    frame.push_element(Element::new("wp:wrapSquare").with_attr("wrapText", "bothSides"));
    frame.push_element(
        Element::new("wp:docPr")
            .with_attr("id", doc_pr.to_string())
            .with_attr("name", format!("组合 {doc_pr}")),
    );
    frame.push_element(Element::new("wp:cNvGraphicFramePr"));
    let mut graphic = Element::new("a:graphic");
    let mut data = Element::new("a:graphicData").with_attr("uri", WPG_NS);
    let mut wgp = Element::new("wpg:wgp");
    wgp.push_element(Element::new("wpg:cNvGrpSpPr"));
    let mut grp_pr = Element::new("wpg:grpSpPr");
    let mut xfrm = Element::new("a:xfrm");
    xfrm.push_element(
        Element::new("a:off")
            .with_attr("x", "0")
            .with_attr("y", "0"),
    );
    xfrm.push_element(
        Element::new("a:ext")
            .with_attr("cx", cx.to_string())
            .with_attr("cy", cy.to_string()),
    );
    xfrm.push_element(
        Element::new("a:chOff")
            .with_attr("x", "0")
            .with_attr("y", "0"),
    );
    xfrm.push_element(
        Element::new("a:chExt")
            .with_attr("cx", cx.to_string())
            .with_attr("cy", cy.to_string()),
    );
    grp_pr.push_element(xfrm);
    wgp.push_element(grp_pr);
    for child in children {
        wgp.push_element(child);
    }
    data.push_element(wgp);
    graphic.push_element(data);
    frame.push_element(graphic);
    drawing.push_element(frame);
    choice.push_element(drawing);
    ac.push_element(choice);
    let mut fallback = Element::new("mc:Fallback");
    fallback.push_element(Element::new("w:pict"));
    ac.push_element(fallback);
    ac
}

fn stamp_cnv_pr(el: &mut Element, id: &str, name: &str) {
    if let Some(cnv) = find_local_mut(el, "cNvPr") {
        set_attr_local(cnv, "id", id);
        set_attr_local(cnv, "name", name);
        return;
    }
    let mut cnv = Element::new(match el.local_name() {
        "pic" => "pic:cNvPr",
        _ => "wps:cNvPr",
    });
    cnv.set_attr("id", id);
    cnv.set_attr("name", name);
    el.children.insert(0, Node::Element(cnv));
}

fn set_child_off(el: &mut Element, x: i64, y: i64, cx: i64, cy: i64) {
    if let Some(xfrm) = find_local_mut(el, "xfrm") {
        if let Some(off) = xfrm.child_by_local_mut("off") {
            set_attr_local(off, "x", &x.to_string());
            set_attr_local(off, "y", &y.to_string());
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
            set_attr_local(ext, "cx", &cx.to_string());
            set_attr_local(ext, "cy", &cy.to_string());
        }
        return;
    }
    let sp = ensure_sp_pr(el);
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
    sp.children.insert(0, Node::Element(xfrm));
}

fn ensure_sp_pr(el: &mut Element) -> &mut Element {
    if el.child_by_local("spPr").is_none() {
        let name = if el.local_name() == "pic" {
            "pic:spPr"
        } else {
            "wps:spPr"
        };
        el.push_element(Element::new(name));
    }
    el.child_by_local_mut("spPr").unwrap()
}

fn find_local_mut<'a>(el: &'a mut Element, local: &str) -> Option<&'a mut Element> {
    if el.local_name() == local {
        return Some(el);
    }
    for n in &mut el.children {
        if let Node::Element(e) = n {
            if let Some(found) = find_local_mut(e, local) {
                return Some(found);
            }
        }
    }
    None
}

fn set_attr_local(el: &mut Element, local: &str, value: &str) {
    if let Some((k, _)) = el
        .attrs
        .iter()
        .find(|(k, _)| k.rsplit(':').next().unwrap_or(k) == local)
    {
        let k = k.clone();
        el.set_attr(&k, value);
        return;
    }
    el.set_attr(local, value);
}

fn drawing_of_slot(node: &Node) -> Option<&Element> {
    let Node::Element(el) = node else {
        return None;
    };
    if el.local_name() == "drawing" {
        return Some(el);
    }
    if el.local_name() == "AlternateContent" {
        let i = el
            .children
            .iter()
            .position(|n| matches!(n, Node::Element(e) if e.local_name() == "Choice"))?;
        if let Node::Element(choice) = &el.children[i] {
            return choice
                .element_children()
                .find(|e| e.local_name() == "drawing");
        }
    }
    None
}

fn find_pic_el(drawing: &Element) -> Option<&Element> {
    fn walk(el: &Element) -> Option<&Element> {
        if el.local_name() == "txbxContent" {
            return None;
        }
        if el.local_name() == "pic" {
            return Some(el);
        }
        for n in &el.children {
            if let Node::Element(e) = n {
                if let Some(f) = walk(e) {
                    return Some(f);
                }
            }
        }
        None
    }
    walk(drawing)
}

fn find_wsp(drawing: &Element) -> Option<&Element> {
    fn walk(el: &Element) -> Option<&Element> {
        if el.local_name() == "txbxContent" {
            return None;
        }
        if el.local_name() == "wsp" {
            return Some(el);
        }
        for n in &el.children {
            if let Node::Element(e) = n {
                if let Some(f) = walk(e) {
                    return Some(f);
                }
            }
        }
        None
    }
    walk(drawing)
}

fn find_txbx_drawing<'a>(host: &'a Element, key: &TxbxKey) -> Option<&'a Element> {
    let mut index = 0usize;
    fn walk<'a>(el: &'a Element, key: &TxbxKey, index: &mut usize) -> Option<&'a Element> {
        if el.local_name() == "txbxContent" {
            return None;
        }
        if el.local_name() == "drawing" {
            if textbox::is_group(el) {
                return None;
            }
            if has_txbx_el(el) && !connector_drawing(el) {
                *index += 1;
                let (id, _) = doc_pr_of(el);
                if para::matches_txbx_key(&id, *index, key) {
                    return Some(el);
                }
            }
            return None;
        }
        if el.local_name() == "AlternateContent" {
            if let Some(i) = el
                .children
                .iter()
                .position(|n| matches!(n, Node::Element(e) if e.local_name() == "Choice"))
            {
                if let Node::Element(c) = &el.children[i] {
                    return walk(c, key, index);
                }
            }
            return None;
        }
        for n in &el.children {
            if let Node::Element(e) = n {
                if let Some(f) = walk(e, key, index) {
                    return Some(f);
                }
            }
        }
        None
    }
    walk(host, key, &mut index)
}

fn connector_drawing(drawing: &Element) -> bool {
    let mut hit = false;
    walk_all(drawing, &mut |e| {
        if connector::is_connector_wsp(e) {
            hit = true;
        }
    });
    hit
}

fn drawing_kind(drawing: &Element) -> PicKind {
    if drawing
        .element_children()
        .any(|e| e.local_name() == "anchor")
    {
        PicKind::Anchor
    } else {
        PicKind::Inline
    }
}

fn extent_of(drawing: &Element) -> (Option<i64>, Option<i64>) {
    let frame = drawing
        .element_children()
        .find(|e| e.local_name() == "inline" || e.local_name() == "anchor");
    let extent = frame.and_then(|f| f.child_by_local("extent"));
    (
        extent
            .and_then(|e| e.attr_local("cx"))
            .and_then(|s| s.parse().ok()),
        extent
            .and_then(|e| e.attr_local("cy"))
            .and_then(|s| s.parse().ok()),
    )
}

fn doc_pr_of(drawing: &Element) -> (Option<String>, Option<String>) {
    let frame = drawing
        .element_children()
        .find(|e| e.local_name() == "inline" || e.local_name() == "anchor");
    let pr = frame.and_then(|f| f.child_by_local("docPr"));
    (
        pr.and_then(|p| p.attr_local("id")).map(str::to_string),
        pr.and_then(|p| p.attr_local("name")).map(str::to_string),
    )
}

fn cnv_pr_of(el: &Element) -> (Option<String>, Option<String>) {
    let mut id = None;
    let mut name = None;
    walk_all(el, &mut |e| {
        if e.local_name() == "cNvPr" && id.is_none() {
            id = e.attr_local("id").map(str::to_string);
            name = e.attr_local("name").map(str::to_string);
        }
    });
    (id, name)
}

fn wrap_local(drawing: &Element) -> &'static str {
    let frame = drawing
        .element_children()
        .find(|e| e.local_name() == "anchor");
    let Some(frame) = frame else {
        return "wrapSquare";
    };
    for c in frame.element_children() {
        match c.local_name() {
            "wrapSquare" => return "wrapSquare",
            "wrapTight" => return "wrapTight",
            "wrapThrough" => return "wrapThrough",
            "wrapTopAndBottom" => return "wrapTopAndBottom",
            "wrapNone" => return "wrapNone",
            _ => {}
        }
    }
    "wrapSquare"
}

fn xfrm_of(el: &Element) -> Option<(i64, i64)> {
    let mut cx = None;
    let mut cy = None;
    walk_all(el, &mut |e| {
        if e.local_name() == "ext" && cx.is_none() {
            cx = e.attr_local("cx").and_then(|s| s.parse().ok());
            cy = e.attr_local("cy").and_then(|s| s.parse().ok());
        }
    });
    Some((cx?, cy?))
}

fn has_txbx_el(el: &Element) -> bool {
    let mut found = false;
    walk_all(el, &mut |e| {
        if e.local_name() == "txbxContent"
            || e.local_name() == "txbx"
            || e.local_name() == "linkedTxbx"
        {
            found = true;
        }
    });
    found
}

fn walk_all(el: &Element, f: &mut impl FnMut(&Element)) {
    crate::xml::walk_elements_if(el, |node| {
        if node.local_name() == "txbxContent" {
            return false;
        }
        f(node);
        true
    });
}
