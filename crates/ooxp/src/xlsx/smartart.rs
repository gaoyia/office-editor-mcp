//! 工作表 diagram `xdr:graphicFrame`：名单、插删、改点与锚点。数据走 `crate::smartart`。

use serde_json::{json, Value};

use crate::error::{Error, ErrorKind, IdReason};
use crate::opc::Package;
use crate::smartart::{
    self, colors_xml, style_xml, Role, CT_COLORS, CT_LAYOUT, CT_STYLE, DEFAULT_CX, DEFAULT_CY,
    DGM_NS, REL_CS, REL_LO, REL_QS,
};
use crate::xml::{self, Document, Element};

use super::picture::{self, EditAs};
use super::request::{
    AddSmartArtPointOp, DeleteSmartArtOp, DeleteSmartArtPointOp, InsertSmartArtOp,
    SetSmartArtAnchorOp, SetSmartArtColorsOp, SetSmartArtLayoutOp, SetSmartArtSizeOp,
    SetSmartArtStyleOp, SetSmartArtTextOp,
};
use super::shape::{self, DrawKey, SheetDrawId};
use super::REL_NS;

const DIAGRAMS_DIR: &str = "/xl/diagrams";

#[derive(Debug, Clone)]
pub enum AddParent {
    Diagram(SheetDrawId),
    Point(PointTarget),
}

#[derive(Debug, Clone)]
pub struct PointTarget {
    pub diagram: SheetDrawId,
    pub model_id: String,
}

pub fn parse_diagram_id(id: &str, op_index: usize) -> Result<SheetDrawId, Error> {
    shape::parse_sheet_draw_id(
        id,
        &format!("ops[{op_index}].id"),
        "smartdiagram",
        IdReason::SheetSmartArtPrefix,
        IdReason::SheetSmartArtContainer,
        IdReason::SmartArtIndexNotInt,
        IdReason::SmartArtIndexFromOne,
    )
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
            format!("ops[{op_index}].id"),
        ));
    }
    let diagram = parse_diagram_id(left, op_index)?;
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

pub fn collect_smartarts(
    pkg: &Package,
    sheet_part: &str,
    sheet_name: &str,
) -> Result<Vec<Value>, Error> {
    let sheet = xml::parse(pkg.get(sheet_part)?)?;
    let mut out = Vec::new();
    let mut index = 0usize;
    for drawing_part in picture::drawing_parts_from(pkg, sheet_part, &sheet.root)? {
        let doc = xml::parse(pkg.get(&drawing_part)?)?;
        picture::walk_anchors(&doc.root, &mut |anchor, kind| {
            shape::visit_payloads(anchor, &mut |el| {
                if is_diagram_frame(el) {
                    index += 1;
                    out.push(smartart_json(
                        pkg,
                        &drawing_part,
                        anchor,
                        kind,
                        el,
                        sheet_name,
                        index,
                    )?);
                }
                Ok(())
            })
        })?;
    }
    Ok(out)
}

fn smartart_json(
    pkg: &Package,
    drawing_part: &str,
    anchor: &Element,
    kind: picture::AnchorKind,
    frame: &Element,
    sheet_name: &str,
    index: usize,
) -> Result<Value, Error> {
    let (raw_id, name) = shape::cnv_of_object(frame);
    let stable = raw_id.is_some();
    let id = match &raw_id {
        Some(v) => format!("sheet:{sheet_name}/smartdiagram:{v}"),
        None => format!("sheet:{sheet_name}/smartdiagram:index:{index}"),
    };
    let mut m = shape::geom_map(anchor, kind, id.clone(), raw_id, name, stable)?;
    let rids = smartart::rel_ids(frame);
    let layout = rids
        .lo
        .as_ref()
        .and_then(|rid| smartart::unique_id_of(pkg, drawing_part, rid));
    let colors = rids
        .cs
        .as_ref()
        .and_then(|rid| smartart::unique_id_of(pkg, drawing_part, rid));
    let style = rids
        .qs
        .as_ref()
        .and_then(|rid| smartart::unique_id_of(pkg, drawing_part, rid));
    let points = match &rids.dm {
        Some(rid) => match crate::opc::resolve_id(pkg, drawing_part, rid) {
            Ok(part) => match pkg.get(&part).ok().and_then(|b| xml::parse(b).ok()) {
                Some(doc) => smartart::points_json(&id, &doc.root),
                None => Vec::new(),
            },
            Err(_) => Vec::new(),
        },
        None => Vec::new(),
    };
    let stable_v = m.remove("stable").unwrap_or(json!(stable));
    m.insert("layout".into(), json!(layout));
    m.insert("colors".into(), json!(colors));
    m.insert("style".into(), json!(style));
    m.insert("points".into(), Value::Array(points));
    m.insert("stable".into(), stable_v);
    let rels = crate::opc::Relationships::from_package(pkg, drawing_part).ok();
    m.insert(
        "hyperlink".into(),
        super::hyperlink::inspect_cnv(frame, rels.as_ref()),
    );
    Ok(Value::Object(m))
}

pub(crate) fn is_diagram_frame(el: &Element) -> bool {
    el.local_name() == "graphicFrame"
        && graphic_uri(el).is_some_and(|u| smartart::is_diagram_uri(&u))
}

fn graphic_uri(el: &Element) -> Option<String> {
    let mut found = None;
    picture::walk_all(el, &mut |e| {
        if e.local_name() == "graphicData" && found.is_none() {
            found = e.attr_local("uri").map(str::to_string);
        }
    });
    found
}

pub fn insert_smartart(
    pkg: &mut Package,
    sheet_doc: &mut Document,
    sheet_part: &str,
    op: &InsertSmartArtOp,
) -> Result<(), Error> {
    let drawing_part = picture::ensure_drawing_part(pkg, sheet_doc, sheet_part)?;
    let rids = smartart::write_four_parts(
        pkg,
        &drawing_part,
        DIAGRAMS_DIR,
        op.layout,
        op.points.as_deref(),
    )?;
    let mut draw_doc = xml::parse(pkg.get(&drawing_part)?)?;
    picture::ensure_xmlns(&mut draw_doc.root, "r", REL_NS);
    picture::ensure_xmlns(&mut draw_doc.root, "a", picture::A_NS);
    picture::ensure_xmlns(&mut draw_doc.root, "dgm", DGM_NS);
    let cnv = picture::next_cnv_id(pkg, sheet_part, &sheet_doc.root)?;
    let (cx, cy) = (op.cx.unwrap_or(DEFAULT_CX), op.cy.unwrap_or(DEFAULT_CY));
    let name = format!("图示 {cnv}");
    let frame = build_graphic_frame(
        cnv,
        &name,
        rids.dm.as_deref().unwrap_or(""),
        rids.lo.as_deref().unwrap_or(""),
        rids.qs.as_deref().unwrap_or(""),
        rids.cs.as_deref().unwrap_or(""),
        cx,
        cy,
    );
    let anchor = picture::build_anchor(
        op.anchor,
        op.from,
        op.to,
        op.from_off_x,
        op.from_off_y,
        op.to_off_x,
        op.to_off_y,
        None,
        None,
        cx,
        cy,
        EditAs::TwoCell,
        cnv,
        &name,
        "",
        Some(frame),
    );
    draw_doc.root.push_element(anchor);
    pkg.set(&drawing_part, draw_doc.to_bytes()?)?;
    Ok(())
}

fn build_graphic_frame(
    cnv: u32,
    name: &str,
    dm: &str,
    lo: &str,
    qs: &str,
    cs: &str,
    cx: i64,
    cy: i64,
) -> Element {
    let mut nv = Element::new("xdr:nvGraphicFramePr");
    nv.push_element(
        Element::new("xdr:cNvPr")
            .with_attr("id", cnv.to_string())
            .with_attr("name", name),
    );
    let mut locks = Element::new("xdr:cNvGraphicFramePr");
    locks.push_element(Element::new("a:graphicFrameLocks").with_attr("noGrp", "1"));
    nv.push_element(locks);
    let mut xfrm = Element::new("xdr:xfrm");
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
    let mut data = Element::new("a:graphicData").with_attr("uri", DGM_NS);
    data.push_element(
        Element::new("dgm:relIds")
            .with_attr("xmlns:dgm", DGM_NS)
            .with_attr("xmlns:r", REL_NS)
            .with_attr("r:dm", dm)
            .with_attr("r:lo", lo)
            .with_attr("r:qs", qs)
            .with_attr("r:cs", cs),
    );
    let mut graphic = Element::new("a:graphic");
    graphic.push_element(data);
    let mut frame = Element::new("xdr:graphicFrame");
    frame.push_element(nv);
    frame.push_element(xfrm);
    frame.push_element(graphic);
    frame
}

pub fn delete_smartart(
    pkg: &mut Package,
    sheet_doc: &mut Document,
    sheet_part: &str,
    op: &DeleteSmartArtOp,
    op_index: usize,
) -> Result<(), Error> {
    let path = format!("ops[{op_index}].id");
    let drawing_part = shape::find_drawing_for(
        pkg,
        sheet_part,
        &sheet_doc.root,
        is_diagram_frame,
        &op.target.key,
    )
    .ok_or_else(|| {
        Error::with_path(
            ErrorKind::SmartArtNotFound { id: op.id.clone() },
            path.clone(),
        )
    })?;
    let mut doc = xml::parse(pkg.get(&drawing_part)?)?;
    let frame =
        shape::find_object(&doc.root, is_diagram_frame, &op.target.key).ok_or_else(|| {
            Error::with_path(
                ErrorKind::SmartArtNotFound { id: op.id.clone() },
                path.clone(),
            )
        })?;
    let rids = smartart::rel_ids(frame);
    let mut parts = Vec::new();
    for rid in rids.iter().flatten() {
        if let Ok(p) = crate::opc::resolve_id(pkg, &drawing_part, rid) {
            parts.push(p);
        }
    }
    if let Some(dm) = rids.dm.as_ref() {
        if let Ok(data_part) = crate::opc::resolve_id(pkg, &drawing_part, dm) {
            smartart::drop_drawing_caches(pkg, &drawing_part, &data_part)?;
        }
    }
    if !shape::remove_object(&mut doc.root, is_diagram_frame, &op.target.key) {
        return Err(Error::with_path(
            ErrorKind::SmartArtNotFound { id: op.id.clone() },
            path,
        ));
    }
    let used: Vec<String> = rids.iter().flatten().cloned().collect();
    shape::write_or_drop_drawing(pkg, sheet_doc, sheet_part, &drawing_part, doc)?;
    if pkg.contains(&drawing_part) {
        let draw = xml::parse(pkg.get(&drawing_part)?)?;
        shape::drop_unused_rels(pkg, &drawing_part, used, rid_still_on_drawing, &draw.root)?;
    }
    for p in &parts {
        smartart::drop_unused(pkg, p)?;
    }
    Ok(())
}

fn rid_still_on_drawing(root: &Element, rid: &str) -> bool {
    let mut used = false;
    picture::walk_all(root, &mut |e| {
        if used {
            return;
        }
        if e.local_name() != "relIds" {
            return;
        }
        for local in ["dm", "lo", "qs", "cs"] {
            if e.attr_local(local) == Some(rid) {
                used = true;
                return;
            }
        }
    });
    used
}

fn not_found(id: &str, path: String) -> Error {
    Error::with_path(ErrorKind::SmartArtNotFound { id: id.to_string() }, path)
}

fn missing_data(id: &str, i: usize) -> Error {
    Error::with_path(
        ErrorKind::PartMissing {
            name: format!("{id} data"),
        },
        format!("ops[{i}].id"),
    )
}

fn load_drawing(
    pkg: &Package,
    sheet_root: &Element,
    sheet_part: &str,
    key: &DrawKey,
    id: &str,
    i: usize,
) -> Result<(Document, String), Error> {
    let drawing_part = shape::find_drawing_for(pkg, sheet_part, sheet_root, is_diagram_frame, key)
        .ok_or_else(|| not_found(id, format!("ops[{i}].id")))?;
    let doc = xml::parse(pkg.get(&drawing_part)?)?;
    Ok((doc, drawing_part))
}

fn data_part_of(
    pkg: &Package,
    draw_root: &Element,
    drawing_part: &str,
    key: &DrawKey,
    id: &str,
    i: usize,
) -> Result<String, Error> {
    let frame = shape::find_object(draw_root, is_diagram_frame, key)
        .ok_or_else(|| not_found(id, format!("ops[{i}].id")))?;
    let rid = smartart::rel_ids(frame)
        .dm
        .ok_or_else(|| missing_data(id, i))?;
    crate::opc::resolve_id(pkg, drawing_part, &rid).map_err(|_| missing_data(id, i))
}

fn layout_unique_id(
    pkg: &Package,
    draw_root: &Element,
    drawing_part: &str,
    key: &DrawKey,
    id: &str,
    i: usize,
) -> Result<String, Error> {
    let frame = shape::find_object(draw_root, is_diagram_frame, key)
        .ok_or_else(|| not_found(id, format!("ops[{i}].id")))?;
    let rid = smartart::rel_ids(frame).lo.ok_or_else(|| {
        Error::with_path(
            ErrorKind::SmartArtLayoutNotConvertible,
            format!("ops[{i}].id"),
        )
    })?;
    smartart::unique_id_of(pkg, drawing_part, &rid).ok_or_else(|| {
        Error::with_path(
            ErrorKind::SmartArtLayoutNotConvertible,
            format!("ops[{i}].id"),
        )
    })
}

pub fn apply_text(
    pkg: &mut Package,
    sheet_root: &Element,
    sheet_part: &str,
    op: &SetSmartArtTextOp,
    i: usize,
) -> Result<(), Error> {
    let (draw, drawing_part) = load_drawing(
        pkg,
        sheet_root,
        sheet_part,
        &op.target.diagram.key,
        &op.id,
        i,
    )?;
    let data_part = data_part_of(
        pkg,
        &draw.root,
        &drawing_part,
        &op.target.diagram.key,
        &op.id,
        i,
    )?;
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
    if !smartart::sync_drawing_text(
        pkg,
        &drawing_part,
        &data_part,
        &op.target.model_id,
        &op.text,
    )? {
        smartart::drop_drawing_caches(pkg, &drawing_part, &data_part)?;
    }
    Ok(())
}

pub fn apply_add_point(
    pkg: &mut Package,
    sheet_root: &Element,
    sheet_part: &str,
    op: &AddSmartArtPointOp,
    i: usize,
) -> Result<(), Error> {
    let (diagram, parent_model, id) = match &op.parent {
        AddParent::Diagram(t) => (t.clone(), None, op.id.clone()),
        AddParent::Point(p) => (p.diagram.clone(), Some(p.model_id.clone()), op.id.clone()),
    };
    let (draw, drawing_part) = load_drawing(pkg, sheet_root, sheet_part, &diagram.key, &id, i)?;
    let layout_uid = layout_unique_id(pkg, &draw.root, &drawing_part, &diagram.key, &id, i).ok();
    if smartart::family_of(layout_uid.as_deref().unwrap_or("")).is_some_and(|k| k.is_flat()) {
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
    let data_part = data_part_of(pkg, &draw.root, &drawing_part, &diagram.key, &id, i)?;
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
    smartart::drop_drawing_caches(pkg, &drawing_part, &data_part)?;
    Ok(())
}

pub fn apply_delete_point(
    pkg: &mut Package,
    sheet_root: &Element,
    sheet_part: &str,
    op: &DeleteSmartArtPointOp,
    i: usize,
) -> Result<(), Error> {
    let (draw, drawing_part) = load_drawing(
        pkg,
        sheet_root,
        sheet_part,
        &op.target.diagram.key,
        &op.id,
        i,
    )?;
    let data_part = data_part_of(
        pkg,
        &draw.root,
        &drawing_part,
        &op.target.diagram.key,
        &op.id,
        i,
    )?;
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
    smartart::drop_drawing_caches(pkg, &drawing_part, &data_part)?;
    Ok(())
}

pub fn set_smartart_size(
    pkg: &mut Package,
    sheet_root: &Element,
    sheet_part: &str,
    op: &SetSmartArtSizeOp,
    op_index: usize,
) -> Result<(), Error> {
    shape::set_object_size(
        pkg,
        sheet_root,
        sheet_part,
        &op.target.key,
        is_diagram_frame,
        picture::SizePatch {
            cx: op.cx,
            cy: op.cy,
            to: op.to,
            to_off_x: op.to_off_x,
            to_off_y: op.to_off_y,
        },
        ErrorKind::TwoCellSmartArtSize,
        ErrorKind::SmartArtNotFound { id: op.id.clone() },
        "setSmartArtSize",
        op_index,
    )
}

pub fn set_smartart_anchor(
    pkg: &mut Package,
    sheet_root: &Element,
    sheet_part: &str,
    op: &SetSmartArtAnchorOp,
    op_index: usize,
) -> Result<(), Error> {
    shape::set_object_anchor(
        pkg,
        sheet_root,
        sheet_part,
        &op.target.key,
        is_diagram_frame,
        shape::AnchorWrite {
            op: "setSmartArtAnchor",
            anchor: op.anchor,
            from: op.from,
            to: op.to,
            from_off_x: op.from_off_x,
            from_off_y: op.from_off_y,
            to_off_x: op.to_off_x,
            to_off_y: op.to_off_y,
            pos_x: op.pos_x,
            pos_y: op.pos_y,
            edit_as: op.edit_as,
            cx: op.cx,
            cy: op.cy,
            default_cx: DEFAULT_CX,
            default_cy: DEFAULT_CY,
            rel: None,
        },
        op_index,
        ErrorKind::SmartArtNotFound { id: op.id.clone() },
    )
}

pub fn apply_colors(
    pkg: &mut Package,
    sheet_root: &Element,
    sheet_part: &str,
    op: &SetSmartArtColorsOp,
    i: usize,
) -> Result<(), Error> {
    let (mut draw, drawing_part) =
        load_drawing(pkg, sheet_root, sheet_part, &op.target.key, &op.id, i)?;
    retarget(
        pkg,
        &mut draw,
        &drawing_part,
        &op.target.key,
        &op.id,
        i,
        "cs",
        REL_CS,
        CT_COLORS,
        "colors",
        &op.colors.unique_id(),
        &colors_xml(op.colors),
    )?;
    pkg.set(&drawing_part, draw.to_bytes()?)?;
    Ok(())
}

pub fn apply_style(
    pkg: &mut Package,
    sheet_root: &Element,
    sheet_part: &str,
    op: &SetSmartArtStyleOp,
    i: usize,
) -> Result<(), Error> {
    let (mut draw, drawing_part) =
        load_drawing(pkg, sheet_root, sheet_part, &op.target.key, &op.id, i)?;
    retarget(
        pkg,
        &mut draw,
        &drawing_part,
        &op.target.key,
        &op.id,
        i,
        "qs",
        REL_QS,
        CT_STYLE,
        "quickStyle",
        op.style.unique_id(),
        &style_xml(op.style),
    )?;
    pkg.set(&drawing_part, draw.to_bytes()?)?;
    Ok(())
}

pub fn apply_layout(
    pkg: &mut Package,
    sheet_root: &Element,
    sheet_part: &str,
    op: &SetSmartArtLayoutOp,
    i: usize,
) -> Result<(), Error> {
    let (mut draw, drawing_part) =
        load_drawing(pkg, sheet_root, sheet_part, &op.target.key, &op.id, i)?;
    let current = layout_unique_id(pkg, &draw.root, &drawing_part, &op.target.key, &op.id, i)?;
    if current == op.layout.unique_id() {
        return Ok(());
    }
    if smartart::family_of(&current).is_none() {
        return Err(Error::with_path(
            ErrorKind::SmartArtLayoutNotConvertible,
            format!("ops[{i}].layout"),
        ));
    }
    retarget(
        pkg,
        &mut draw,
        &drawing_part,
        &op.target.key,
        &op.id,
        i,
        "lo",
        REL_LO,
        CT_LAYOUT,
        "layout",
        op.layout.unique_id(),
        op.layout.layout_xml(),
    )?;
    if let Ok(data_part) = data_part_of(pkg, &draw.root, &drawing_part, &op.target.key, &op.id, i) {
        if op.layout.is_flat() {
            let mut data = xml::parse(pkg.get(&data_part)?)?;
            smartart::demote_asst(&mut data.root);
            pkg.set(&data_part, data.to_bytes()?)?;
        }
        smartart::drop_drawing_caches(pkg, &drawing_part, &data_part)?;
    }
    pkg.set(&drawing_part, draw.to_bytes()?)?;
    Ok(())
}

fn retarget(
    pkg: &mut Package,
    draw_doc: &mut Document,
    drawing_part: &str,
    key: &DrawKey,
    id: &str,
    i: usize,
    attr: &str,
    rel_type: &str,
    ct: &str,
    stem: &str,
    unique_id: &str,
    xml_body: &str,
) -> Result<(), Error> {
    let leftover = {
        let frame = shape::find_object_mut(&mut draw_doc.root, is_diagram_frame, key)
            .ok_or_else(|| not_found(id, format!("ops[{i}].id")))?;
        smartart::retarget_satellite(
            pkg,
            drawing_part,
            DIAGRAMS_DIR,
            frame,
            attr,
            rel_type,
            ct,
            stem,
            unique_id,
            xml_body,
        )?
    };
    if let Some((old_rid, old_part)) = leftover {
        smartart::drop_replaced_satellite(pkg, drawing_part, &draw_doc.root, &old_rid, &old_part)?;
    }
    Ok(())
}
