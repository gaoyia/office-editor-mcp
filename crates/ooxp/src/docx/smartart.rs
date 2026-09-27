//! 正文、页眉页脚、文本框、脚注、尾注与批注里的 SmartArt：名单、改字、增删点、插删、配色与版式。不实现版式算法。

use std::collections::HashMap;

use serde_json::{json, Map, Value};

use crate::error::{Error, ErrorKind, IdReason, TypeName};
use crate::opc::{rels_part_name, Package, Relationships};
use crate::smartart::{
    self, colors_xml, style_xml, RelIds, CT_COLORS, CT_LAYOUT, CT_STYLE, DEFAULT_CX, DEFAULT_CY,
    DGM_NS, REL_CS, REL_LO, REL_QS,
};
use crate::xml::{self, Document, Element, Node};

use super::comments;
use super::hf::{HfKind, HfPart, HfType};
use super::notes;
use super::para::{self, BodyPara, HostPara, ParaTarget, TxbxKey};
use super::picture::{self, PicKind, WrapKind, WrapPos};
use super::textbox;
use super::REL_NS;

pub use crate::smartart::{
    parse_colors_field, parse_layout_field, parse_role_field, parse_style_field, ColorsKind,
    LayoutKind, NewPoint, Role, StyleKind,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagramKey {
    DocPr(String),
    Index(usize),
}

#[derive(Debug, Clone)]
pub enum DiagramTarget {
    Body {
        para: BodyPara,
        key: DiagramKey,
    },
    HeaderFooter {
        kind: HfKind,
        hf_type: HfType,
        n: u32,
        para: BodyPara,
        key: DiagramKey,
    },
    InTextBox {
        host: HostPara,
        box_key: TxbxKey,
        para: BodyPara,
        key: DiagramKey,
    },
    InNote {
        kind: para::NoteKind,
        note_id: String,
        para: BodyPara,
        key: DiagramKey,
    },
    InComment {
        comment_id: String,
        para: BodyPara,
        key: DiagramKey,
    },
}

#[derive(Debug, Clone)]
pub struct PointTarget {
    pub diagram: DiagramTarget,
    pub model_id: String,
}

#[derive(Debug, Clone)]
pub enum AddParent {
    Diagram(DiagramTarget),
    Point(PointTarget),
}

#[derive(Debug, Clone)]
pub struct InsertSpec {
    pub layout: LayoutKind,
    pub points: Option<Vec<NewPoint>>,
    pub kind: PicKind,
    pub wrap: Option<WrapKind>,
    pub pos: WrapPos,
    pub cx: i64,
    pub cy: i64,
}

#[derive(Debug, Clone)]
pub struct InsertSmartArtOp {
    pub id: String,
    pub para: ParaTarget,
    pub spec: InsertSpec,
}

#[derive(Debug, Clone)]
pub struct DeleteSmartArtOp {
    pub id: String,
    pub target: DiagramTarget,
}

#[derive(Debug, Clone)]
pub struct SetSmartArtTextOp {
    pub id: String,
    pub target: PointTarget,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct AddSmartArtPointOp {
    pub id: String,
    pub parent: AddParent,
    pub text: String,
    pub role: Role,
}

#[derive(Debug, Clone)]
pub struct DeleteSmartArtPointOp {
    pub id: String,
    pub target: PointTarget,
}

#[derive(Debug, Clone)]
pub struct SetSmartArtSizeOp {
    pub id: String,
    pub target: DiagramTarget,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct SetSmartArtWrapOp {
    pub id: String,
    pub target: DiagramTarget,
    pub kind: PicKind,
    pub wrap: Option<WrapKind>,
    pub pos: WrapPos,
}

#[derive(Debug, Clone)]
pub struct SetSmartArtColorsOp {
    pub id: String,
    pub target: DiagramTarget,
    pub colors: ColorsKind,
}

#[derive(Debug, Clone)]
pub struct SetSmartArtStyleOp {
    pub id: String,
    pub target: DiagramTarget,
    pub style: StyleKind,
}

#[derive(Debug, Clone)]
pub struct SetSmartArtLayoutOp {
    pub id: String,
    pub target: DiagramTarget,
    pub layout: LayoutKind,
}

pub fn count_smartarts(root: &Element) -> Result<usize, Error> {
    let mut n = 0usize;
    picture::walk_drawings(root, &mut |d| {
        if super::watermark::is_watermark_drawing(d) {
            return Ok(());
        }
        if is_smartart(d) {
            n += 1;
        }
        Ok(())
    })?;
    Ok(n)
}

pub fn collect_body(
    pkg: &Package,
    doc_part: &str,
    paras: &[&Element],
) -> Result<Vec<Value>, Error> {
    let mut out = Vec::new();
    for (i, p) in paras.iter().enumerate() {
        let (id, _) = para::para_stable_id(p, i + 1);
        let mut index = 0usize;
        out.extend(collect_in_para(pkg, doc_part, p, &id, &mut index)?);
    }
    Ok(out)
}

pub fn collect_hf(pkg: &Package, item: &HfPart, paras: &[&Element]) -> Result<Vec<Value>, Error> {
    let mut out = Vec::new();
    let mut index = 0usize;
    for (i, p) in paras.iter().enumerate() {
        let (id, _) = para::para_stable_id_in(&item.id, p, i + 1);
        out.extend(collect_in_para(pkg, &item.part, p, &id, &mut index)?);
    }
    Ok(out)
}

pub fn collect_in_para(
    pkg: &Package,
    host_part: &str,
    para: &Element,
    para_id: &str,
    index: &mut usize,
) -> Result<Vec<Value>, Error> {
    let mut out = Vec::new();
    picture::walk_drawings(para, &mut |drawing| {
        if super::watermark::is_watermark_drawing(drawing) {
            return Ok(());
        }
        if !is_smartart(drawing) {
            return Ok(());
        }
        *index += 1;
        out.push(smartart_json(pkg, host_part, drawing, para_id, *index)?);
        Ok(())
    })?;
    Ok(out)
}

pub fn parse_diagram_id(id: &str, op_index: usize) -> Result<DiagramTarget, Error> {
    parse_diagram_id_at(id, &format!("ops[{op_index}].id"))
}

pub fn parse_diagram_id_at(id: &str, path: &str) -> Result<DiagramTarget, Error> {
    if id.contains("/pt:") {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SmartArtPrefix,
            },
            path.to_string(),
        ));
    }
    let Some((left, key_s)) = id.rsplit_once("/smartdiagram:") else {
        if looks_like_para_or_container(id) {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::SmartArtContainer,
                },
                path.to_string(),
            ));
        }
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SmartArtPrefix,
            },
            path.to_string(),
        ));
    };
    let key = parse_diagram_key(key_s, path)?;
    match para::parse_para_id_at(left, path)? {
        ParaTarget::Body(para) => Ok(DiagramTarget::Body { para, key }),
        ParaTarget::HeaderFooter {
            kind,
            hf_type,
            n,
            para,
        } => Ok(DiagramTarget::HeaderFooter {
            kind,
            hf_type,
            n,
            para,
            key,
        }),
        ParaTarget::InTextBox {
            host,
            key: box_key,
            para,
        } => Ok(DiagramTarget::InTextBox {
            host,
            box_key,
            para,
            key,
        }),
        ParaTarget::InNote {
            kind,
            note_id,
            para,
        } => Ok(DiagramTarget::InNote {
            kind,
            note_id,
            para,
            key,
        }),
        ParaTarget::InComment { comment_id, para } => Ok(DiagramTarget::InComment {
            comment_id,
            para,
            key,
        }),
    }
}

pub fn parse_point_id(id: &str, op_index: usize) -> Result<PointTarget, Error> {
    let path = format!("ops[{op_index}].id");
    let Some((left, model_id)) = id.rsplit_once("/pt:") else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SmartArtPtPrefix,
            },
            path,
        ));
    };
    if model_id.is_empty() {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SmartArtPtPrefix,
            },
            path,
        ));
    }
    let diagram = parse_diagram_id_at(left, &path)?;
    Ok(PointTarget {
        diagram,
        model_id: model_id.to_string(),
    })
}

pub fn parse_add_parent(id: &str, op_index: usize) -> Result<AddParent, Error> {
    if id.contains("/pt:") {
        Ok(AddParent::Point(parse_point_id(id, op_index)?))
    } else {
        Ok(AddParent::Diagram(parse_diagram_id(id, op_index)?))
    }
}

pub fn parse_insert_para_id(id: &str, op_index: usize) -> Result<ParaTarget, Error> {
    let path = format!("ops[{op_index}].id");
    if id.contains("/smartdiagram:")
        || id.contains("/pt:")
        || id.contains("/chart:")
        || id.contains("/series:")
        || id.contains("/pic:")
        || id.starts_with("section:")
    {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::InsertSmartArtNeedsPara,
            },
            path,
        ));
    }
    if let Some((_, after)) = id.split_once("/txbx:") {
        if !after.contains("/para:") {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::InsertSmartArtNeedsPara,
                },
                path,
            ));
        }
    }
    let target = para::parse_para_id_at(id, &path)?;
    match &target {
        ParaTarget::Body(_)
        | ParaTarget::HeaderFooter { .. }
        | ParaTarget::InTextBox { .. }
        | ParaTarget::InNote { .. }
        | ParaTarget::InComment { .. } => Ok(target),
    }
}

pub fn parse_spec(obj: &Map<String, Value>, path: &str) -> Result<InsertSpec, Error> {
    let (layout, points) = smartart::parse_layout_and_points(obj, path)?;
    let kind = match obj.get("kind") {
        None => PicKind::Anchor,
        Some(Value::String(s)) => picture::parse_pic_kind(s)
            .ok_or_else(|| Error::with_path(ErrorKind::InvalidPicKind, format!("{path}.kind")))?,
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "kind".into(),
                    expected: TypeName::String,
                },
                format!("{path}.kind"),
            ));
        }
    };
    let wrap =
        match obj.get("wrap") {
            None | Some(Value::Null) => {
                if kind == PicKind::Anchor {
                    Some(WrapKind::Square)
                } else {
                    None
                }
            }
            Some(Value::String(s)) => Some(picture::parse_wrap(s).ok_or_else(|| {
                Error::with_path(ErrorKind::InvalidPicWrap, format!("{path}.wrap"))
            })?),
            Some(_) => {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: "wrap".into(),
                        expected: TypeName::String,
                    },
                    format!("{path}.wrap"),
                ));
            }
        };
    if kind == PicKind::Inline && wrap.is_some() {
        return Err(Error::with_path(
            ErrorKind::InvalidPicWrap,
            format!("{path}.wrap"),
        ));
    }
    let cx = smartart::parse_emu(obj.get("cx"), path, "cx")?;
    let cy = smartart::parse_emu(obj.get("cy"), path, "cy")?;
    if cx.is_some() != cy.is_some() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "insertSmartArt".into(),
                field: "cx and cy together".into(),
            },
            path.to_string(),
        ));
    }
    Ok(InsertSpec {
        layout,
        points,
        kind,
        wrap,
        pos: WrapPos::default(),
        cx: cx.unwrap_or(DEFAULT_CX),
        cy: cy.unwrap_or(DEFAULT_CY),
    })
}

pub fn apply_insert(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[HfPart],
    op: &InsertSmartArtOp,
    i: usize,
) -> Result<(), Error> {
    let host = part_for_para(pkg, catalog, doc_part, &op.para, &op.id, i)?;
    if let ParaTarget::InComment { comment_id, .. } = &op.para {
        let doc = load_part(pkg, docs, &host)?;
        let comment = comments::find_comment(&doc.root, comment_id).ok_or_else(|| {
            Error::with_path(
                ErrorKind::ParaNotFound { id: op.id.clone() },
                format!("ops[{i}].id"),
            )
        })?;
        comments::reject_ink(comment, &op.id, i)?;
    }
    {
        let doc = load_part(pkg, docs, &host)?;
        if find_para_mut(&mut doc.root, &op.para).is_none() {
            return Err(Error::with_path(
                ErrorKind::ParaNotFound { id: op.id.clone() },
                format!("ops[{i}].id"),
            ));
        }
    }
    let rids = smartart::write_four_parts(
        pkg,
        &host,
        "/word/diagrams",
        op.spec.layout,
        op.spec.points.as_deref(),
    )?;
    let doc = load_part(pkg, docs, &host)?;
    ensure_dgm_ns(&mut doc.root);
    let doc_pr = picture::next_docpr_id(&doc.root);
    let p = find_para_mut(&mut doc.root, &op.para).ok_or_else(|| {
        Error::with_path(
            ErrorKind::ParaNotFound { id: op.id.clone() },
            format!("ops[{i}].id"),
        )
    })?;
    let mut drawing = build_drawing(doc_pr, &rids, op.spec.cx, op.spec.cy);
    picture::apply_wrap(&mut drawing, op.spec.kind, op.spec.wrap, &op.spec.pos)
        .map_err(|e| e.at(format!("ops[{i}]")))?;
    picture::insert_drawing_into_para(p, drawing);
    dirty.insert(host, ());
    Ok(())
}

pub fn attach_to_para(
    pkg: &mut Package,
    host: &str,
    root: &mut Element,
    para_index: usize,
    spec: &InsertSpec,
) -> Result<(), Error> {
    let rids = smartart::write_four_parts(
        pkg,
        host,
        "/word/diagrams",
        spec.layout,
        spec.points.as_deref(),
    )?;
    ensure_dgm_ns(root);
    let doc_pr = picture::next_docpr_id(root);
    let para = nth_story_para_mut(root, para_index).ok_or_else(|| {
        Error::xml_detail(
            crate::error::XmlIssue::Parse,
            format!("{host} missing paragraph {para_index}"),
        )
    })?;
    let mut drawing = build_drawing(doc_pr, &rids, spec.cx, spec.cy);
    picture::apply_wrap(&mut drawing, spec.kind, spec.wrap, &spec.pos)?;
    picture::insert_drawing_into_para(para, drawing);
    Ok(())
}

pub fn apply_delete(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[HfPart],
    op: &DeleteSmartArtOp,
    i: usize,
) -> Result<(), Error> {
    let host = part_for_target(
        pkg,
        catalog,
        doc_part,
        &op.target,
        &op.id,
        &format!("ops[{i}].id"),
    )?;
    let (rids, parts, blips) = {
        let doc = load_part(pkg, docs, &host)?;
        let drawing = find_drawing(&doc.root, &op.target).ok_or_else(|| {
            Error::with_path(
                ErrorKind::SmartArtNotFound { id: op.id.clone() },
                format!("ops[{i}].id"),
            )
        })?;
        let rids = smartart::rel_ids(drawing);
        let mut parts = Vec::new();
        for rid in rids.iter().flatten() {
            if let Ok(p) = crate::opc::resolve_id(pkg, &host, rid) {
                parts.push(p);
            }
        }
        if let Some(dm) = rids.dm.as_ref() {
            if let Ok(data) = crate::opc::resolve_id(pkg, &host, dm) {
                parts.extend(smartart::collect_drawing_caches(pkg, &host, &data)?);
            }
        }
        let blips = collect_blip_rids(drawing);
        (rids, parts, blips)
    };
    let mut media = Vec::new();
    for rid in &blips {
        if let Ok(m) = picture::resolve_embed(pkg, &host, rid) {
            media.push(m);
        }
    }
    {
        let doc = load_part(pkg, docs, &host)?;
        if !remove_smartart(&mut doc.root, &op.target) {
            return Err(Error::with_path(
                ErrorKind::SmartArtNotFound { id: op.id.clone() },
                format!("ops[{i}].id"),
            ));
        }
    }
    dirty.insert(host.clone(), ());
    let root = &docs.get(&host).unwrap().root;
    for rid in rids.iter().flatten() {
        remove_rel_if_unused(pkg, &host, rid, root)?;
    }
    for rid in &blips {
        picture::remove_rel_if_unused(pkg, &host, rid, root)?;
    }
    for p in &parts {
        smartart::drop_unused(pkg, p)?;
    }
    // data 部件卸掉之后，原先只被它的 .rels 指着的 drawing 缓存才能卸。
    for p in &parts {
        smartart::drop_unused(pkg, p)?;
    }
    for m in media {
        if !picture::media_still_used(pkg, &m)? {
            pkg.remove(&m);
        }
    }
    Ok(())
}

pub fn apply_text(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    doc_part: &str,
    catalog: &[HfPart],
    op: &SetSmartArtTextOp,
    i: usize,
) -> Result<(), Error> {
    let host = part_for_target(
        pkg,
        catalog,
        doc_part,
        &op.target.diagram,
        &op.id,
        &format!("ops[{i}].id"),
    )?;
    let data_part = data_part_of(pkg, docs, &host, &op.target.diagram, &op.id, i)?;
    let mut data = xml::parse(pkg.get(&data_part)?)?;
    let pt =
        smartart::find_visible_pt_mut(&mut data.root, &op.target.model_id).ok_or_else(|| {
            Error::with_path(
                ErrorKind::SmartArtPointNotFound { id: op.id.clone() },
                format!("ops[{i}].id"),
            )
        })?;
    smartart::set_point_text(pt, &op.text);
    pkg.set(&data_part, data.to_bytes()?)?;
    if !smartart::sync_drawing_text(pkg, &host, &data_part, &op.target.model_id, &op.text)? {
        smartart::drop_drawing_caches(pkg, &host, &data_part)?;
    }
    Ok(())
}

pub fn apply_add_point(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    doc_part: &str,
    catalog: &[HfPart],
    op: &AddSmartArtPointOp,
    i: usize,
) -> Result<(), Error> {
    let (diagram, parent_model, id) = match &op.parent {
        AddParent::Diagram(t) => (t.clone(), None, op.id.clone()),
        AddParent::Point(p) => (p.diagram.clone(), Some(p.model_id.clone()), op.id.clone()),
    };
    let host = part_for_target(
        pkg,
        catalog,
        doc_part,
        &diagram,
        &id,
        &format!("ops[{i}].id"),
    )?;
    let layout_uid = layout_unique_id(pkg, docs, &host, &diagram, &id, i).ok();
    if smartart::family_of(layout_uid.as_deref().unwrap_or("")) == Some(LayoutKind::List) {
        if op.role == Role::Asst {
            return Err(Error::with_path(
                ErrorKind::SmartArtAsstOnList,
                format!("ops[{i}].role"),
            ));
        }
        if parent_model.is_some() {
            return Err(Error::with_path(
                ErrorKind::SmartArtListDepth,
                format!("ops[{i}].id"),
            ));
        }
    }
    let data_part = data_part_of(pkg, docs, &host, &diagram, &id, i)?;
    let mut data = xml::parse(pkg.get(&data_part)?)?;
    let parent_id = match &parent_model {
        Some(mid) => {
            if smartart::find_visible_pt(&data.root, mid).is_none() {
                return Err(Error::with_path(
                    ErrorKind::SmartArtPointNotFound { id: id.clone() },
                    format!("ops[{i}].id"),
                ));
            }
            mid.clone()
        }
        None => smartart::ensure_doc_pt(&mut data.root),
    };
    let (pt_id, cxn_id) = smartart::next_model_ids(&data.root);
    smartart::append_visible_pt(&mut data.root, &pt_id, op.role, &op.text);
    smartart::append_par_of(&mut data.root, &cxn_id, &parent_id, &pt_id);
    pkg.set(&data_part, data.to_bytes()?)?;
    smartart::drop_drawing_caches(pkg, &host, &data_part)?;
    Ok(())
}

pub fn apply_delete_point(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    doc_part: &str,
    catalog: &[HfPart],
    op: &DeleteSmartArtPointOp,
    i: usize,
) -> Result<(), Error> {
    let host = part_for_target(
        pkg,
        catalog,
        doc_part,
        &op.target.diagram,
        &op.id,
        &format!("ops[{i}].id"),
    )?;
    let data_part = data_part_of(pkg, docs, &host, &op.target.diagram, &op.id, i)?;
    let mut data = xml::parse(pkg.get(&data_part)?)?;
    if smartart::find_visible_pt(&data.root, &op.target.model_id).is_none() {
        return Err(Error::with_path(
            ErrorKind::SmartArtPointNotFound { id: op.id.clone() },
            format!("ops[{i}].id"),
        ));
    }
    let visible = smartart::visible_model_ids(&data.root);
    let doomed = smartart::descendants_of(&data.root, &op.target.model_id);
    if doomed.iter().filter(|id| visible.contains(*id)).count() >= visible.len() {
        return Err(Error::with_path(
            ErrorKind::LastSmartArtPoint,
            format!("ops[{i}].id"),
        ));
    }
    smartart::delete_points_and_cxns(&mut data.root, &doomed);
    pkg.set(&data_part, data.to_bytes()?)?;
    smartart::drop_drawing_caches(pkg, &host, &data_part)?;
    Ok(())
}

pub fn apply_size(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[HfPart],
    op: &SetSmartArtSizeOp,
    i: usize,
) -> Result<(), Error> {
    let host = part_for_target(
        pkg,
        catalog,
        doc_part,
        &op.target,
        &op.id,
        &format!("ops[{i}].id"),
    )?;
    let doc = load_part(pkg, docs, &host)?;
    let drawing = find_drawing_mut(&mut doc.root, &op.target).ok_or_else(|| {
        Error::with_path(
            ErrorKind::SmartArtNotFound { id: op.id.clone() },
            format!("ops[{i}].id"),
        )
    })?;
    if !picture::set_extent(drawing, op.cx, op.cy) {
        return Err(Error::with_path(
            ErrorKind::SmartArtNotFound { id: op.id.clone() },
            format!("ops[{i}].id"),
        ));
    }
    dirty.insert(host, ());
    Ok(())
}

pub fn apply_wrap(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[HfPart],
    op: &SetSmartArtWrapOp,
    i: usize,
) -> Result<(), Error> {
    let host = part_for_target(
        pkg,
        catalog,
        doc_part,
        &op.target,
        &op.id,
        &format!("ops[{i}].id"),
    )?;
    let doc = load_part(pkg, docs, &host)?;
    ensure_dgm_ns(&mut doc.root);
    let drawing = find_drawing_mut(&mut doc.root, &op.target).ok_or_else(|| {
        Error::with_path(
            ErrorKind::SmartArtNotFound { id: op.id.clone() },
            format!("ops[{i}].id"),
        )
    })?;
    picture::apply_wrap(drawing, op.kind, op.wrap, &op.pos)
        .map_err(|e| e.at(format!("ops[{i}]")))?;
    dirty.insert(host, ());
    Ok(())
}

pub fn apply_colors(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[HfPart],
    op: &SetSmartArtColorsOp,
    i: usize,
) -> Result<(), Error> {
    retarget_satellite(
        pkg,
        docs,
        dirty,
        doc_part,
        catalog,
        &op.target,
        &op.id,
        i,
        "cs",
        REL_CS,
        CT_COLORS,
        "colors",
        &op.colors.unique_id(),
        &colors_xml(op.colors),
    )
}

pub fn apply_style(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[HfPart],
    op: &SetSmartArtStyleOp,
    i: usize,
) -> Result<(), Error> {
    retarget_satellite(
        pkg,
        docs,
        dirty,
        doc_part,
        catalog,
        &op.target,
        &op.id,
        i,
        "qs",
        REL_QS,
        CT_STYLE,
        "quickStyle",
        op.style.unique_id(),
        &style_xml(op.style),
    )
}

pub fn apply_layout(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[HfPart],
    op: &SetSmartArtLayoutOp,
    i: usize,
) -> Result<(), Error> {
    let host = part_for_target(
        pkg,
        catalog,
        doc_part,
        &op.target,
        &op.id,
        &format!("ops[{i}].id"),
    )?;
    let current = layout_unique_id(pkg, docs, &host, &op.target, &op.id, i)?;
    if current == op.layout.unique_id() {
        return Ok(());
    }
    let cur_fam = smartart::family_of(&current);
    let tgt_fam = Some(op.layout);
    if cur_fam != tgt_fam {
        return Err(Error::with_path(
            ErrorKind::SmartArtLayoutNotConvertible,
            format!("ops[{i}].layout"),
        ));
    }
    retarget_satellite(
        pkg,
        docs,
        dirty,
        doc_part,
        catalog,
        &op.target,
        &op.id,
        i,
        "lo",
        REL_LO,
        CT_LAYOUT,
        "layout",
        op.layout.unique_id(),
        op.layout.layout_xml(),
    )?;
    if let Ok(data) = data_part_of(pkg, docs, &host, &op.target, &op.id, i) {
        smartart::drop_drawing_caches(pkg, &host, &data)?;
    }
    Ok(())
}

fn retarget_satellite(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[HfPart],
    target: &DiagramTarget,
    id: &str,
    i: usize,
    attr: &str,
    rel_type: &str,
    ct: &str,
    stem: &str,
    unique_id: &str,
    xml_body: &str,
) -> Result<(), Error> {
    let host = part_for_target(pkg, catalog, doc_part, target, id, &format!("ops[{i}].id"))?;
    let doc = load_part(pkg, docs, &host)?;
    let drawing = find_drawing_mut(&mut doc.root, target).ok_or_else(|| {
        Error::with_path(
            ErrorKind::SmartArtNotFound { id: id.to_string() },
            format!("ops[{i}].id"),
        )
    })?;
    smartart::retarget_satellite(
        pkg,
        &host,
        "/word/diagrams",
        drawing,
        attr,
        rel_type,
        ct,
        stem,
        unique_id,
        xml_body,
    )?;
    dirty.insert(host, ());
    Ok(())
}

fn smartart_json(
    pkg: &Package,
    host_part: &str,
    drawing: &Element,
    para_id: &str,
    index: usize,
) -> Result<Value, Error> {
    let frame = frame(drawing);
    let kind = match frame.map(Element::local_name) {
        Some("anchor") => PicKind::Anchor,
        _ => PicKind::Inline,
    };
    let doc_pr = frame.and_then(|f| f.child_by_local("docPr"));
    let raw_id = doc_pr
        .and_then(|d| d.attr_local("id"))
        .filter(|s| !s.is_empty());
    let name = doc_pr
        .and_then(|d| d.attr_local("name"))
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    let (id, stable, doc_pr_id) = match raw_id {
        Some(v) => (
            format!("{para_id}/smartdiagram:{v}"),
            true,
            Some(v.to_string()),
        ),
        None => (format!("{para_id}/smartdiagram:index:{index}"), false, None),
    };
    let extent = frame.and_then(|f| f.child_by_local("extent"));
    let cx = parse_emu_attr(extent, "cx")?;
    let cy = parse_emu_attr(extent, "cy")?;
    let wrap = if kind == PicKind::Anchor {
        frame.and_then(|f| {
            f.element_children()
                .find_map(|c| WrapKind::from_local(c.local_name()))
        })
    } else {
        None
    };
    let rids = smartart::rel_ids(drawing);
    let layout = rids
        .lo
        .as_ref()
        .and_then(|rid| smartart::unique_id_of(pkg, host_part, rid));
    let colors = rids
        .cs
        .as_ref()
        .and_then(|rid| smartart::unique_id_of(pkg, host_part, rid));
    let style = rids
        .qs
        .as_ref()
        .and_then(|rid| smartart::unique_id_of(pkg, host_part, rid));
    let points = match &rids.dm {
        Some(rid) => match crate::opc::resolve_id(pkg, host_part, rid) {
            Ok(part) => match pkg.get(&part).ok().and_then(|b| xml::parse(b).ok()) {
                Some(doc) => smartart::points_json(&id, &doc.root),
                None => Vec::new(),
            },
            Err(_) => Vec::new(),
        },
        None => Vec::new(),
    };
    Ok(json!({
        "id": id,
        "docPrId": doc_pr_id,
        "name": name,
        "kind": kind.as_str(),
        "cx": cx,
        "cy": cy,
        "wrap": wrap.map(WrapKind::as_str),
        "layout": layout,
        "colors": colors,
        "style": style,
        "points": points,
        "stable": stable,
    }))
}

fn is_smartart(drawing: &Element) -> bool {
    graphic_uri(drawing).is_some_and(|u| smartart::is_diagram_uri(&u))
}

fn graphic_uri(drawing: &Element) -> Option<String> {
    let mut found = None;
    walk_outside_txbx(drawing, &mut |e| {
        if e.local_name() == "graphicData" && found.is_none() {
            found = e.attr_local("uri").map(str::to_string);
        }
    });
    found
}

fn looks_like_para_or_container(id: &str) -> bool {
    id.starts_with("para:")
        || id.starts_with("header:")
        || id.starts_with("footer:")
        || id.starts_with("footnote:")
        || id.starts_with("endnote:")
        || id.starts_with("comment:")
}

fn parse_diagram_key(rest: &str, path: &str) -> Result<DiagramKey, Error> {
    if let Some(n) = rest.strip_prefix("index:") {
        let idx: usize = n.parse().map_err(|_| {
            Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::SmartArtIndexNotInt,
                },
                path.to_string(),
            )
        })?;
        if idx < 1 {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::SmartArtIndexFromOne,
                },
                path.to_string(),
            ));
        }
        return Ok(DiagramKey::Index(idx));
    }
    if rest.is_empty() {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SmartArtPrefix,
            },
            path.to_string(),
        ));
    }
    Ok(DiagramKey::DocPr(rest.to_string()))
}

fn matches_key(drawing: &Element, key: &DiagramKey, index: usize) -> bool {
    match key {
        DiagramKey::Index(n) => index == *n,
        DiagramKey::DocPr(want) => {
            let Some(got) = frame(drawing)
                .and_then(|f| f.child_by_local("docPr"))
                .and_then(|d| d.attr_local("id"))
            else {
                return false;
            };
            if let (Ok(a), Ok(b)) = (want.parse::<u32>(), got.parse::<u32>()) {
                a == b
            } else {
                want == got
            }
        }
    }
}

fn diagram_key(target: &DiagramTarget) -> &DiagramKey {
    match target {
        DiagramTarget::Body { key, .. }
        | DiagramTarget::HeaderFooter { key, .. }
        | DiagramTarget::InTextBox { key, .. }
        | DiagramTarget::InNote { key, .. }
        | DiagramTarget::InComment { key, .. } => key,
    }
}

fn find_target_para<'a>(root: &'a Element, target: &DiagramTarget) -> Option<&'a Element> {
    match target {
        DiagramTarget::Body { para, .. } | DiagramTarget::HeaderFooter { para, .. } => {
            para::find_paragraph(root, para)
        }
        DiagramTarget::InTextBox {
            host,
            box_key,
            para,
            ..
        } => {
            let host_p = para::find_paragraph(root, host.body())?;
            let content = textbox::find_txbx_content_by_key(host_p, box_key)?;
            para::find_paragraph_in_txbx(content, para)
        }
        DiagramTarget::InNote {
            kind,
            note_id,
            para,
            ..
        } => {
            let note = notes::find_user_note(root, *kind, note_id)?;
            para::find_paragraph(note, para)
        }
        DiagramTarget::InComment {
            comment_id, para, ..
        } => {
            let comment = comments::find_comment(root, comment_id)?;
            para::find_paragraph(comment, para)
        }
    }
}

fn find_drawing<'a>(root: &'a Element, target: &DiagramTarget) -> Option<&'a Element> {
    match target {
        DiagramTarget::HeaderFooter {
            para,
            key: DiagramKey::Index(n),
            ..
        } => find_hf_index(root, para, *n),
        _ => {
            let p = find_target_para(root, target)?;
            find_walk(p, &mut 0, diagram_key(target))
        }
    }
}

fn find_hf_index<'a>(root: &'a Element, para: &BodyPara, n: usize) -> Option<&'a Element> {
    let local = hf_local_index(root, para, n)?;
    let p = para::find_paragraph(root, para)?;
    find_walk(p, &mut 0, &DiagramKey::Index(local))
}

fn find_target_para_mut<'a>(
    root: &'a mut Element,
    target: &DiagramTarget,
) -> Option<&'a mut Element> {
    match target {
        DiagramTarget::Body { para, .. } | DiagramTarget::HeaderFooter { para, .. } => {
            para::find_paragraph_mut(root, para)
        }
        DiagramTarget::InTextBox {
            host,
            box_key,
            para,
            ..
        } => {
            let host_p = para::find_paragraph_mut(root, host.body())?;
            let content = textbox::find_txbx_content_by_key_mut(host_p, box_key)?;
            para::find_paragraph_in_txbx_mut(content, para)
        }
        DiagramTarget::InNote {
            kind,
            note_id,
            para,
            ..
        } => {
            let note = notes::find_user_note_mut(root, *kind, note_id)?;
            para::find_paragraph_mut(note, para)
        }
        DiagramTarget::InComment {
            comment_id, para, ..
        } => {
            let comment = comments::find_comment_mut(root, comment_id)?;
            para::find_paragraph_mut(comment, para)
        }
    }
}

fn find_drawing_mut<'a>(root: &'a mut Element, target: &DiagramTarget) -> Option<&'a mut Element> {
    match target {
        DiagramTarget::HeaderFooter {
            para,
            key: DiagramKey::Index(n),
            ..
        } => find_hf_index_mut(root, para, *n),
        _ => {
            let p = find_target_para_mut(root, target)?;
            find_walk_mut(p, &mut 0, diagram_key(target))
        }
    }
}

fn find_hf_index_mut<'a>(
    root: &'a mut Element,
    para: &BodyPara,
    n: usize,
) -> Option<&'a mut Element> {
    let local = hf_local_index(root, para, n)?;
    let p = para::find_paragraph_mut(root, para)?;
    find_walk_mut(p, &mut 0, &DiagramKey::Index(local))
}

fn hf_local_index(root: &Element, para: &BodyPara, n: usize) -> Option<usize> {
    let mut idx = 0usize;
    for (i, p) in para::collect_paragraphs(root).into_iter().enumerate() {
        let is_target = para_matches(p, i + 1, para);
        let start = idx;
        let _ = picture::walk_drawings(p, &mut |d| {
            if is_smartart(d) {
                idx += 1;
            }
            Ok(())
        });
        if is_target && start < n && n <= idx {
            return Some(n - start);
        }
    }
    None
}

fn para_matches(p: &Element, index: usize, para: &BodyPara) -> bool {
    match para {
        BodyPara::Index(n) => index == *n,
        BodyPara::Hex(hex) => p
            .attr_local("paraId")
            .is_some_and(|id| id.eq_ignore_ascii_case(hex)),
    }
}

fn find_walk<'a>(el: &'a Element, idx: &mut usize, key: &DiagramKey) -> Option<&'a Element> {
    if el.local_name() == "txbxContent" || el.local_name() == "object" {
        return None;
    }
    if el.local_name() == "AlternateContent" {
        if let Some(i) = first_alt_child_index(el) {
            if let Node::Element(e) = &el.children[i] {
                return find_walk(e, idx, key);
            }
        }
        return None;
    }
    if el.local_name() == "drawing" && is_smartart(el) {
        *idx += 1;
        if matches_key(el, key, *idx) {
            return Some(el);
        }
        return None;
    }
    for n in &el.children {
        if let Node::Element(e) = n {
            if let Some(found) = find_walk(e, idx, key) {
                return Some(found);
            }
        }
    }
    None
}

fn find_walk_mut<'a>(
    el: &'a mut Element,
    idx: &mut usize,
    key: &DiagramKey,
) -> Option<&'a mut Element> {
    if el.local_name() == "txbxContent" || el.local_name() == "object" {
        return None;
    }
    if el.local_name() == "AlternateContent" {
        if let Some(i) = first_alt_child_index(el) {
            if let Node::Element(e) = &mut el.children[i] {
                return find_walk_mut(e, idx, key);
            }
        }
        return None;
    }
    if el.local_name() == "drawing" && is_smartart(el) {
        *idx += 1;
        if matches_key(el, key, *idx) {
            return Some(el);
        }
        return None;
    }
    let children = el.children.as_mut_slice();
    for child in children {
        if let Node::Element(e) = child {
            if let Some(found) = find_walk_mut(e, idx, key) {
                return Some(found);
            }
        }
    }
    None
}

fn remove_smartart(root: &mut Element, target: &DiagramTarget) -> bool {
    match target {
        DiagramTarget::HeaderFooter {
            para,
            key: DiagramKey::Index(n),
            ..
        } => match hf_local_index(root, para, *n) {
            Some(local) => match para::find_paragraph_mut(root, para) {
                Some(p) => remove_walk(p, &mut 0, &DiagramKey::Index(local)),
                None => false,
            },
            None => false,
        },
        _ => match find_target_para_mut(root, target) {
            Some(p) => remove_walk(p, &mut 0, diagram_key(target)),
            None => false,
        },
    }
}

fn remove_walk(el: &mut Element, idx: &mut usize, key: &DiagramKey) -> bool {
    if el.local_name() == "txbxContent" || el.local_name() == "object" {
        return false;
    }
    let mut i = 0;
    while i < el.children.len() {
        let local = match &el.children[i] {
            Node::Element(c) => c.local_name().to_string(),
            Node::Text(_) => {
                i += 1;
                continue;
            }
        };
        if local == "AlternateContent" {
            if ac_has_smartart(&el.children[i], idx, key) {
                el.children.remove(i);
                return true;
            }
            i += 1;
            continue;
        }
        if local == "drawing" {
            let is_s = matches!(&el.children[i], Node::Element(c) if is_smartart(c));
            if is_s {
                *idx += 1;
                let hit = match &el.children[i] {
                    Node::Element(c) => matches_key(c, key, *idx),
                    Node::Text(_) => false,
                };
                if hit {
                    el.children.remove(i);
                    return true;
                }
            }
            i += 1;
            continue;
        }
        if local == "txbxContent" || local == "object" {
            i += 1;
            continue;
        }
        if let Node::Element(child) = &mut el.children[i] {
            if remove_walk(child, idx, key) {
                if child.local_name() == "r"
                    && child.element_children().all(|c| c.local_name() == "rPr")
                {
                    el.children.remove(i);
                }
                return true;
            }
        }
        i += 1;
    }
    false
}

fn ac_has_smartart(node: &Node, idx: &mut usize, key: &DiagramKey) -> bool {
    let Node::Element(ac) = node else {
        return false;
    };
    let Some(i) = first_alt_child_index(ac) else {
        return false;
    };
    let Node::Element(branch) = &ac.children[i] else {
        return false;
    };
    let mut hit = false;
    let _ = picture::walk_drawings(branch, &mut |d| {
        if is_smartart(d) {
            *idx += 1;
            if matches_key(d, key, *idx) {
                hit = true;
            }
        }
        Ok(())
    });
    hit
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

fn frame(drawing: &Element) -> Option<&Element> {
    drawing
        .element_children()
        .find(|e| e.local_name() == "inline" || e.local_name() == "anchor")
}

fn parse_emu_attr(el: Option<&Element>, local: &str) -> Result<Option<i64>, Error> {
    let Some(el) = el else {
        return Ok(None);
    };
    let Some(raw) = el.attr_local(local) else {
        return Ok(None);
    };
    raw.parse::<i64>().map(Some).map_err(|_| {
        Error::xml_detail(
            crate::error::XmlIssue::AttrRead,
            format!("wp:extent @{local}={raw}"),
        )
    })
}

fn walk_outside_txbx(el: &Element, f: &mut impl FnMut(&Element)) {
    if el.local_name() == "txbxContent" || el.local_name() == "object" {
        return;
    }
    f(el);
    for n in &el.children {
        if let Node::Element(e) = n {
            walk_outside_txbx(e, f);
        }
    }
}

fn build_drawing(doc_pr: u32, rids: &RelIds, cx: i64, cy: i64) -> Element {
    let mut drawing = Element::new("w:drawing");
    let mut inline = Element::new("wp:inline")
        .with_attr("distT", "0")
        .with_attr("distB", "0")
        .with_attr("distL", "0")
        .with_attr("distR", "0");
    inline.push_element(
        Element::new("wp:extent")
            .with_attr("cx", cx.to_string())
            .with_attr("cy", cy.to_string()),
    );
    inline.push_element(
        Element::new("wp:effectExtent")
            .with_attr("l", "0")
            .with_attr("t", "0")
            .with_attr("r", "0")
            .with_attr("b", "0"),
    );
    inline.push_element(
        Element::new("wp:docPr")
            .with_attr("id", doc_pr.to_string())
            .with_attr("name", format!("SmartArt {doc_pr}")),
    );
    let mut frame_pr = Element::new("wp:cNvGraphicFramePr");
    frame_pr.push_element(Element::new("a:graphicFrameLocks").with_attr("noGrp", "1"));
    inline.push_element(frame_pr);
    let mut graphic = Element::new("a:graphic");
    let mut data = Element::new("a:graphicData").with_attr("uri", DGM_NS);
    data.push_element(
        Element::new("dgm:relIds")
            .with_attr("xmlns:dgm", DGM_NS)
            .with_attr("xmlns:r", REL_NS)
            .with_attr("r:dm", rids.dm.as_deref().unwrap_or(""))
            .with_attr("r:lo", rids.lo.as_deref().unwrap_or(""))
            .with_attr("r:qs", rids.qs.as_deref().unwrap_or(""))
            .with_attr("r:cs", rids.cs.as_deref().unwrap_or("")),
    );
    graphic.push_element(data);
    inline.push_element(graphic);
    drawing.push_element(inline);
    drawing
}

fn ensure_dgm_ns(root: &mut Element) {
    picture::ensure_drawing_ns(root);
    ensure_xmlns(root, "dgm", DGM_NS);
}

fn ensure_xmlns(el: &mut Element, prefix: &str, uri: &str) {
    let key = format!("xmlns:{prefix}");
    if el.attr(&key).is_none() {
        el.set_attr(&key, uri);
    }
}

fn load_part<'a>(
    pkg: &Package,
    docs: &'a mut HashMap<String, Document>,
    part: &str,
) -> Result<&'a mut Document, Error> {
    if !docs.contains_key(part) {
        let parsed = xml::parse(pkg.get(part)?)?;
        docs.insert(part.to_string(), parsed);
    }
    Ok(docs.get_mut(part).unwrap())
}

fn part_for_para(
    pkg: &Package,
    catalog: &[HfPart],
    doc_part: &str,
    para: &ParaTarget,
    id: &str,
    i: usize,
) -> Result<String, Error> {
    match para {
        ParaTarget::Body(_) => Ok(doc_part.to_string()),
        ParaTarget::HeaderFooter {
            kind, hf_type, n, ..
        } => {
            let container_id = format!("{}:{}:{n}", kind.as_str(), hf_type.as_str());
            let item = super::hf::find_by_id(catalog, &container_id).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::ParaNotFound { id: id.to_string() },
                    format!("ops[{i}].id"),
                )
            })?;
            Ok(item.part.clone())
        }
        ParaTarget::InTextBox { host, .. } => {
            part_for_host_para(pkg, catalog, doc_part, host, id, i, true)
        }
        ParaTarget::InNote { kind, .. } => {
            notes::part_name(pkg, doc_part, *kind)?.ok_or_else(|| {
                Error::with_path(
                    ErrorKind::ParaNotFound { id: id.to_string() },
                    format!("ops[{i}].id"),
                )
            })
        }
        ParaTarget::InComment { .. } => comments::part_name(pkg, doc_part)?.ok_or_else(|| {
            Error::with_path(
                ErrorKind::ParaNotFound { id: id.to_string() },
                format!("ops[{i}].id"),
            )
        }),
    }
}

fn part_for_host_para(
    pkg: &Package,
    catalog: &[HfPart],
    doc_part: &str,
    host: &HostPara,
    id: &str,
    i: usize,
    insert: bool,
) -> Result<String, Error> {
    let missing = || {
        if insert {
            Error::with_path(
                ErrorKind::ParaNotFound { id: id.to_string() },
                format!("ops[{i}].id"),
            )
        } else {
            Error::with_path(
                ErrorKind::SmartArtNotFound { id: id.to_string() },
                format!("ops[{i}].id"),
            )
        }
    };
    match host {
        HostPara::Body(_) => Ok(doc_part.to_string()),
        HostPara::HeaderFooter {
            kind, hf_type, n, ..
        } => {
            let container_id = format!("{}:{}:{n}", kind.as_str(), hf_type.as_str());
            let item = super::hf::find_by_id(catalog, &container_id).ok_or_else(missing)?;
            Ok(item.part.clone())
        }
        HostPara::InNote { kind, .. } => {
            notes::part_name(pkg, doc_part, *kind)?.ok_or_else(missing)
        }
        HostPara::InComment { .. } => comments::part_name(pkg, doc_part)?.ok_or_else(missing),
    }
}

fn part_for_target(
    pkg: &Package,
    catalog: &[HfPart],
    doc_part: &str,
    target: &DiagramTarget,
    id: &str,
    path: &str,
) -> Result<String, Error> {
    let missing = || {
        Error::with_path(
            ErrorKind::SmartArtNotFound { id: id.to_string() },
            path.to_string(),
        )
    };
    match target {
        DiagramTarget::Body { .. } => Ok(doc_part.to_string()),
        DiagramTarget::HeaderFooter {
            kind, hf_type, n, ..
        } => {
            let container_id = format!("{}:{}:{n}", kind.as_str(), hf_type.as_str());
            let item = super::hf::find_by_id(catalog, &container_id).ok_or_else(missing)?;
            Ok(item.part.clone())
        }
        DiagramTarget::InTextBox { host, .. } => match host {
            HostPara::Body(_) => Ok(doc_part.to_string()),
            HostPara::HeaderFooter {
                kind, hf_type, n, ..
            } => {
                let container_id = format!("{}:{}:{n}", kind.as_str(), hf_type.as_str());
                let item = super::hf::find_by_id(catalog, &container_id).ok_or_else(missing)?;
                Ok(item.part.clone())
            }
            HostPara::InNote { kind, .. } => {
                notes::part_name(pkg, doc_part, *kind)?.ok_or_else(missing)
            }
            HostPara::InComment { .. } => comments::part_name(pkg, doc_part)?.ok_or_else(missing),
        },
        DiagramTarget::InNote { kind, .. } => {
            notes::part_name(pkg, doc_part, *kind)?.ok_or_else(missing)
        }
        DiagramTarget::InComment { .. } => comments::part_name(pkg, doc_part)?.ok_or_else(missing),
    }
}

fn find_para_mut<'a>(root: &'a mut Element, target: &ParaTarget) -> Option<&'a mut Element> {
    match target {
        ParaTarget::Body(p) | ParaTarget::HeaderFooter { para: p, .. } => {
            para::find_paragraph_mut(root, p)
        }
        ParaTarget::InTextBox { host, key, para } => {
            let host_p = para::find_paragraph_mut(root, host.body())?;
            let content = textbox::find_txbx_content_by_key_mut(host_p, key)?;
            para::find_paragraph_in_txbx_mut(content, para)
        }
        ParaTarget::InNote {
            kind,
            note_id,
            para,
        } => {
            let note = notes::find_user_note_mut(root, *kind, note_id)?;
            para::find_paragraph_mut(note, para)
        }
        ParaTarget::InComment { comment_id, para } => {
            let comment = comments::find_comment_mut(root, comment_id)?;
            para::find_paragraph_mut(comment, para)
        }
    }
}

fn nth_story_para_mut(root: &mut Element, index: usize) -> Option<&mut Element> {
    let parent = if root.local_name() == "document" {
        root.child_by_local_mut("body")?
    } else {
        root
    };
    let mut n = 0usize;
    for child in parent.children.iter_mut() {
        let Node::Element(e) = child else {
            continue;
        };
        if e.local_name() != "p" {
            continue;
        }
        if n == index {
            return Some(e);
        }
        n += 1;
    }
    None
}

fn missing_data(id: &str, i: usize) -> Error {
    Error::with_path(
        ErrorKind::PartMissing {
            name: format!("{id} data"),
        },
        format!("ops[{i}].id"),
    )
}

fn data_part_of(
    pkg: &Package,
    docs: &mut HashMap<String, Document>,
    host: &str,
    target: &DiagramTarget,
    id: &str,
    i: usize,
) -> Result<String, Error> {
    let doc = load_part(pkg, docs, host)?;
    let drawing = find_drawing(&doc.root, target).ok_or_else(|| {
        Error::with_path(
            ErrorKind::SmartArtNotFound { id: id.to_string() },
            format!("ops[{i}].id"),
        )
    })?;
    let rid = smartart::rel_ids(drawing)
        .dm
        .ok_or_else(|| missing_data(id, i))?;
    crate::opc::resolve_id(pkg, host, &rid).map_err(|_| missing_data(id, i))
}

fn layout_unique_id(
    pkg: &Package,
    docs: &mut HashMap<String, Document>,
    host: &str,
    target: &DiagramTarget,
    id: &str,
    i: usize,
) -> Result<String, Error> {
    let doc = load_part(pkg, docs, host)?;
    let drawing = find_drawing(&doc.root, target).ok_or_else(|| {
        Error::with_path(
            ErrorKind::SmartArtNotFound { id: id.to_string() },
            format!("ops[{i}].id"),
        )
    })?;
    let rid = smartart::rel_ids(drawing).lo.ok_or_else(|| {
        Error::with_path(
            ErrorKind::SmartArtLayoutNotConvertible,
            format!("ops[{i}].id"),
        )
    })?;
    smartart::unique_id_of(pkg, host, &rid).ok_or_else(|| {
        Error::with_path(
            ErrorKind::SmartArtLayoutNotConvertible,
            format!("ops[{i}].id"),
        )
    })
}

fn collect_blip_rids(el: &Element) -> Vec<String> {
    let mut out = Vec::new();
    collect_blip_rids_walk(el, &mut out);
    out
}

fn collect_blip_rids_walk(el: &Element, out: &mut Vec<String>) {
    if el.local_name() == "blip" {
        if let Some(id) = el.attr_local("embed") {
            out.push(id.to_string());
        }
    }
    for n in &el.children {
        if let Node::Element(e) = n {
            collect_blip_rids_walk(e, out);
        }
    }
}

fn remove_rel_if_unused(
    pkg: &mut Package,
    part: &str,
    rid: &str,
    root: &Element,
) -> Result<(), Error> {
    let mut found = false;
    walk_outside_txbx(root, &mut |e| {
        if e.local_name() == "relIds"
            && (e.attr_local("dm") == Some(rid)
                || e.attr_local("lo") == Some(rid)
                || e.attr_local("qs") == Some(rid)
                || e.attr_local("cs") == Some(rid))
        {
            found = true;
        }
    });
    if found {
        return Ok(());
    }
    let mut rels = Relationships::from_package(pkg, part)?;
    rels.remove_by_id(rid);
    if rels.rels.is_empty() {
        pkg.remove(&rels_part_name(part));
    } else {
        rels.write_to(pkg, part)?;
    }
    Ok(())
}
