//! 工作表 SpreadsheetDrawing 文本框：名单、插删、改字、尺寸与锚点。

use serde_json::{json, Map, Value};

use crate::error::{Error, ErrorKind, IdReason};
use crate::opc::{rels_part_name, Package, Relationships};
use crate::xml::{self, Document, Element, Node};

use super::a1;
use super::picture::{self, AnchorKind, EditAs};
use super::request::{
    DeleteTextBoxOp, InsertTextBoxOp, SetShapeTextOp, SetTextBoxAnchorOp, SetTextBoxSizeOp,
};

pub const DEFAULT_CX: i64 = 1_828_800;
pub const DEFAULT_CY: i64 = 914_400;

#[derive(Debug, Clone)]
pub enum DrawKey {
    Cnv(String),
    Index(usize),
}

#[derive(Debug, Clone)]
pub struct SheetDrawId {
    pub sheet: String,
    pub key: DrawKey,
}

pub fn parse_insert_sheet_id(id: &str, op_index: usize, reason: IdReason) -> Result<String, Error> {
    if id.contains('/') {
        return Err(invalid_id(reason, op_index));
    }
    let Some(name) = id.strip_prefix("sheet:") else {
        return Err(invalid_id(reason, op_index));
    };
    if name.is_empty() {
        return Err(invalid_id(reason, op_index));
    }
    Ok(name.to_string())
}

pub fn parse_sp_id(id: &str, op_index: usize) -> Result<SheetDrawId, Error> {
    parse_sheet_draw_id(
        id,
        &format!("ops[{op_index}].id"),
        "sp",
        IdReason::SheetSpPrefix,
        IdReason::SheetSpContainer,
        IdReason::SpIndexNotInt,
        IdReason::SpIndexFromOne,
    )
}

pub fn parse_sheet_draw_id(
    id: &str,
    field_path: &str,
    prefix: &str,
    prefix_reason: IdReason,
    container_reason: IdReason,
    index_not_int: IdReason,
    index_from_one: IdReason,
) -> Result<SheetDrawId, Error> {
    let path = field_path.to_string();
    let Some(name) = id.strip_prefix("sheet:") else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: prefix_reason,
            },
            path,
        ));
    };
    let marker = format!("/{prefix}:");
    let Some((sheet, rest)) = name.split_once(&marker) else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: container_reason,
            },
            path,
        ));
    };
    if sheet.is_empty() || rest.is_empty() || rest.contains('/') {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: prefix_reason,
            },
            path,
        ));
    }
    if let Some(idx) = rest.strip_prefix("index:") {
        let n: usize = idx.parse().map_err(|_| {
            Error::with_path(
                ErrorKind::InvalidId {
                    reason: index_not_int,
                },
                path.clone(),
            )
        })?;
        if n < 1 {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: index_from_one,
                },
                path,
            ));
        }
        return Ok(SheetDrawId {
            sheet: sheet.to_string(),
            key: DrawKey::Index(n),
        });
    }
    Ok(SheetDrawId {
        sheet: sheet.to_string(),
        key: DrawKey::Cnv(rest.to_string()),
    })
}

fn invalid_id(reason: IdReason, op_index: usize) -> Error {
    Error::with_path(
        ErrorKind::InvalidId { reason },
        format!("ops[{op_index}].id"),
    )
}

pub fn collect_textboxes(
    pkg: &Package,
    sheet_part: &str,
    sheet_name: &str,
) -> Result<Vec<Value>, Error> {
    let mut out = Vec::new();
    let mut index = 0usize;
    for_each_object(pkg, sheet_part, is_textbox, |anchor, kind, el| {
        index += 1;
        out.push(textbox_json(anchor, kind, el, sheet_name, index));
        Ok(())
    })?;
    Ok(out)
}

fn textbox_json(
    anchor: &Element,
    kind: AnchorKind,
    sp: &Element,
    sheet_name: &str,
    index: usize,
) -> Value {
    let (raw_id, name) = cnv_of_object(sp);
    let stable = raw_id.is_some();
    let id = match &raw_id {
        Some(v) => format!("sheet:{sheet_name}/sp:{v}"),
        None => format!("sheet:{sheet_name}/sp:index:{index}"),
    };
    let mut m = geom_map(anchor, kind, id, raw_id, name, stable);
    let stable_v = m.remove("stable").unwrap_or(json!(stable));
    m.insert("text".into(), json!(crate::pptx::shape::visible_text(sp)));
    m.insert("stable".into(), stable_v);
    Value::Object(m)
}

pub(crate) fn geom_map(
    anchor: &Element,
    kind: AnchorKind,
    id: String,
    raw_id: Option<String>,
    name: String,
    stable: bool,
) -> Map<String, Value> {
    let (from, from_off_x, from_off_y) = picture::read_marker(anchor.child_by_local("from"));
    let (to, to_off_x, to_off_y) = picture::read_marker(anchor.child_by_local("to"));
    let (cx, cy) = picture::read_ext(anchor);
    let (pos_x, pos_y) = picture::read_pos(anchor);
    let mut m = Map::new();
    m.insert("id".into(), json!(id));
    m.insert("cNvPrId".into(), json!(raw_id));
    m.insert("name".into(), json!(name));
    m.insert("anchor".into(), json!(kind.as_str()));
    match kind {
        AnchorKind::TwoCell => {
            m.insert(
                "from".into(),
                json!(from.map(|c| a1::format_cell(c.col, c.row))),
            );
            m.insert("fromOffX".into(), json!(from_off_x));
            m.insert("fromOffY".into(), json!(from_off_y));
            m.insert(
                "to".into(),
                json!(to.map(|c| a1::format_cell(c.col, c.row))),
            );
            m.insert("toOffX".into(), json!(to_off_x));
            m.insert("toOffY".into(), json!(to_off_y));
            m.insert(
                "editAs".into(),
                json!(picture::parse_edit_as(anchor.attr("editAs")).as_str()),
            );
        }
        AnchorKind::OneCell => {
            m.insert(
                "from".into(),
                json!(from.map(|c| a1::format_cell(c.col, c.row))),
            );
            m.insert("fromOffX".into(), json!(from_off_x));
            m.insert("fromOffY".into(), json!(from_off_y));
            m.insert("cx".into(), json!(cx));
            m.insert("cy".into(), json!(cy));
        }
        AnchorKind::Absolute => {
            m.insert("posX".into(), json!(pos_x));
            m.insert("posY".into(), json!(pos_y));
            m.insert("cx".into(), json!(cx));
            m.insert("cy".into(), json!(cy));
        }
    }
    m.insert("stable".into(), json!(stable));
    m
}

pub fn insert_textbox(
    pkg: &mut Package,
    sheet_doc: &mut Document,
    sheet_part: &str,
    op: &InsertTextBoxOp,
) -> Result<(), Error> {
    let cx = op.cx.unwrap_or(DEFAULT_CX);
    let cy = op.cy.unwrap_or(DEFAULT_CY);
    let drawing_part = picture::ensure_drawing_part(pkg, sheet_doc, sheet_part)?;
    let mut draw_doc = xml::parse(pkg.get(&drawing_part)?)?;
    picture::ensure_xmlns(&mut draw_doc.root, "r", super::REL_NS);
    picture::ensure_xmlns(&mut draw_doc.root, "a", picture::A_NS);
    let cnv = picture::next_cnv_id(pkg, sheet_part, &sheet_doc.root)?;
    let name = format!("文本框 {cnv}");
    let sp = build_textbox_sp(cnv, &name, cx, cy, op.text.as_deref().unwrap_or(""));
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
        Some(sp),
    );
    draw_doc.root.push_element(anchor);
    pkg.set(&drawing_part, draw_doc.to_bytes()?)?;
    Ok(())
}

fn build_textbox_sp(cnv: u32, name: &str, cx: i64, cy: i64, text: &str) -> Element {
    let mut nv = Element::new("xdr:nvSpPr");
    nv.push_element(
        Element::new("xdr:cNvPr")
            .with_attr("id", cnv.to_string())
            .with_attr("name", name),
    );
    nv.push_element(Element::new("xdr:cNvSpPr").with_attr("txBox", "1"));
    let mut sp_pr = Element::new("xdr:spPr");
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
    sp_pr.push_element(xfrm);
    let mut geom = Element::new("a:prstGeom").with_attr("prst", "rect");
    geom.push_element(Element::new("a:avLst"));
    sp_pr.push_element(geom);
    let mut fill = Element::new("a:solidFill");
    fill.push_element(Element::new("a:srgbClr").with_attr("val", "FFFFFF"));
    sp_pr.push_element(fill);
    let mut ln = Element::new("a:ln").with_attr("w", "9525");
    let mut ln_fill = Element::new("a:solidFill");
    ln_fill.push_element(Element::new("a:srgbClr").with_attr("val", "000000"));
    ln.push_element(ln_fill);
    sp_pr.push_element(ln);
    let mut tx = Element::new("xdr:txBody");
    tx.push_element(
        Element::new("a:bodyPr")
            .with_attr("wrap", "square")
            .with_attr("rtlCol", "0"),
    );
    tx.push_element(Element::new("a:lstStyle"));
    let mut p = Element::new("a:p");
    if text.is_empty() {
        p.push_element(Element::new("a:endParaRPr").with_attr("sz", "1100"));
    } else {
        let mut r = Element::new("a:r");
        r.push_element(Element::new("a:rPr").with_attr("sz", "1100"));
        r.push_element(Element::new("a:t").with_text(text));
        p.push_element(r);
    }
    tx.push_element(p);
    let mut sp = Element::new("xdr:sp");
    sp.push_element(nv);
    sp.push_element(sp_pr);
    sp.push_element(tx);
    sp
}

pub fn delete_textbox(
    pkg: &mut Package,
    sheet_doc: &mut Document,
    sheet_part: &str,
    op: &DeleteTextBoxOp,
    op_index: usize,
) -> Result<(), Error> {
    let path = format!("ops[{op_index}].id");
    let drawing_part =
        find_drawing_for(pkg, sheet_part, &sheet_doc.root, is_textbox, &op.target.key).ok_or_else(
            || {
                Error::with_path(
                    ErrorKind::TextBoxNotFound { id: op.id.clone() },
                    path.clone(),
                )
            },
        )?;
    let mut doc = xml::parse(pkg.get(&drawing_part)?)?;
    if !remove_object(&mut doc.root, is_textbox, &op.target.key) {
        return Err(Error::with_path(
            ErrorKind::TextBoxNotFound { id: op.id.clone() },
            path,
        ));
    }
    write_or_drop_drawing(pkg, sheet_doc, sheet_part, &drawing_part, doc)?;
    Ok(())
}

pub fn set_shape_text(
    pkg: &mut Package,
    sheet_root: &Element,
    sheet_part: &str,
    op: &SetShapeTextOp,
    op_index: usize,
) -> Result<(), Error> {
    let path = format!("ops[{op_index}].id");
    let drawing_part = find_drawing_for(pkg, sheet_part, sheet_root, is_textbox, &op.target.key)
        .ok_or_else(|| {
            Error::with_path(
                ErrorKind::TextBoxNotFound { id: op.id.clone() },
                path.clone(),
            )
        })?;
    let mut doc = xml::parse(pkg.get(&drawing_part)?)?;
    let sp = find_object_mut(&mut doc.root, is_textbox, &op.target.key).ok_or_else(|| {
        Error::with_path(
            ErrorKind::TextBoxNotFound { id: op.id.clone() },
            path.clone(),
        )
    })?;
    let Some(tx) = sp.child_by_local_mut("txBody") else {
        return Err(Error::with_path(
            ErrorKind::ShapeHasNoText { id: op.id.clone() },
            path,
        ));
    };
    crate::pptx::shape::set_txbody_text(tx, &op.text);
    pkg.set(&drawing_part, doc.to_bytes()?)?;
    Ok(())
}

pub fn set_textbox_size(
    pkg: &mut Package,
    sheet_root: &Element,
    sheet_part: &str,
    op: &SetTextBoxSizeOp,
    op_index: usize,
) -> Result<(), Error> {
    set_object_size(
        pkg,
        sheet_root,
        sheet_part,
        &op.target.key,
        is_textbox,
        op.cx,
        op.cy,
        ErrorKind::TwoCellTextBoxSize,
        ErrorKind::TextBoxNotFound { id: op.id.clone() },
        &format!("ops[{op_index}].id"),
    )
}

pub fn set_textbox_anchor(
    pkg: &mut Package,
    sheet_root: &Element,
    sheet_part: &str,
    op: &SetTextBoxAnchorOp,
    op_index: usize,
) -> Result<(), Error> {
    set_object_anchor(
        pkg,
        sheet_root,
        sheet_part,
        &op.target.key,
        is_textbox,
        AnchorWrite {
            op: "setTextBoxAnchor",
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
        },
        op_index,
        ErrorKind::TextBoxNotFound { id: op.id.clone() },
    )
}

pub(crate) struct AnchorWrite {
    pub op: &'static str,
    pub anchor: Option<AnchorKind>,
    pub from: Option<a1::CellRef>,
    pub to: Option<a1::CellRef>,
    pub from_off_x: Option<i64>,
    pub from_off_y: Option<i64>,
    pub to_off_x: Option<i64>,
    pub to_off_y: Option<i64>,
    pub pos_x: Option<i64>,
    pub pos_y: Option<i64>,
    pub edit_as: Option<EditAs>,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
    pub default_cx: i64,
    pub default_cy: i64,
}

pub(crate) fn set_object_size(
    pkg: &mut Package,
    sheet_root: &Element,
    sheet_part: &str,
    key: &DrawKey,
    is_obj: fn(&Element) -> bool,
    cx: Option<i64>,
    cy: Option<i64>,
    two_cell: ErrorKind,
    not_found: ErrorKind,
    field_path: &str,
) -> Result<(), Error> {
    let drawing_part = find_drawing_for(pkg, sheet_part, sheet_root, is_obj, key)
        .ok_or_else(|| Error::with_path(not_found.clone(), field_path))?;
    let mut doc = xml::parse(pkg.get(&drawing_part)?)?;
    let kind = find_outer_kind(&doc.root, is_obj, key)
        .ok_or_else(|| Error::with_path(not_found.clone(), field_path))?;
    if kind == AnchorKind::TwoCell {
        return Err(Error::with_path(two_cell, field_path));
    }
    if !update_outer_extent(&mut doc.root, is_obj, key, cx, cy) {
        return Err(Error::with_path(not_found, field_path));
    }
    pkg.set(&drawing_part, doc.to_bytes()?)?;
    Ok(())
}

pub(crate) fn set_object_anchor(
    pkg: &mut Package,
    sheet_root: &Element,
    sheet_part: &str,
    key: &DrawKey,
    is_obj: fn(&Element) -> bool,
    w: AnchorWrite,
    op_index: usize,
    not_found: ErrorKind,
) -> Result<(), Error> {
    let path = format!("ops[{op_index}].id");
    let drawing_part = find_drawing_for(pkg, sheet_part, sheet_root, is_obj, key)
        .ok_or_else(|| Error::with_path(not_found.clone(), &path))?;
    let mut doc = xml::parse(pkg.get(&drawing_part)?)?;
    let current = find_outer_kind(&doc.root, is_obj, key)
        .ok_or_else(|| Error::with_path(not_found.clone(), &path))?;
    let target_kind = w.anchor.unwrap_or(current);
    if current == AnchorKind::Absolute {
        if w.anchor.is_some_and(|a| a != AnchorKind::Absolute) || w.from.is_some() || w.to.is_some()
        {
            return Err(Error::with_path(ErrorKind::PictureAnchorAbsolute, path));
        }
        if !update_absolute_pos(&mut doc.root, is_obj, key, w.pos_x, w.pos_y) {
            return Err(Error::with_path(not_found, path));
        }
        pkg.set(&drawing_part, doc.to_bytes()?)?;
        return Ok(());
    }
    if target_kind == AnchorKind::Absolute || w.pos_x.is_some() || w.pos_y.is_some() {
        return Err(Error::with_path(ErrorKind::PictureAnchorAbsolute, path));
    }
    picture::ensure_xmlns(&mut doc.root, "r", super::REL_NS);
    picture::ensure_xmlns(&mut doc.root, "a", picture::A_NS);
    let snapshot = snapshot_outer(&doc.root, is_obj, key)
        .ok_or_else(|| Error::with_path(not_found.clone(), &path))?;
    let from = w.from.or(snapshot.from).ok_or_else(|| {
        Error::with_path(
            ErrorKind::OpRequiresField {
                op: w.op.into(),
                field: "from".into(),
            },
            format!("ops[{op_index}].from"),
        )
    })?;
    let to = if target_kind == AnchorKind::TwoCell {
        Some(w.to.or(snapshot.to).ok_or_else(|| {
            Error::with_path(
                ErrorKind::OpRequiresField {
                    op: w.op.into(),
                    field: "to".into(),
                },
                format!("ops[{op_index}].to"),
            )
        })?)
    } else {
        None
    };
    let cx = if target_kind == AnchorKind::OneCell {
        w.cx.or(snapshot.cx).ok_or_else(|| {
            Error::with_path(
                ErrorKind::OpRequiresField {
                    op: w.op.into(),
                    field: "cx and cy".into(),
                },
                format!("ops[{op_index}]"),
            )
        })?
    } else {
        snapshot.cx.unwrap_or(w.default_cx)
    };
    let cy = if target_kind == AnchorKind::OneCell {
        w.cy.or(snapshot.cy).ok_or_else(|| {
            Error::with_path(
                ErrorKind::OpRequiresField {
                    op: w.op.into(),
                    field: "cx and cy".into(),
                },
                format!("ops[{op_index}]"),
            )
        })?
    } else {
        snapshot.cy.unwrap_or(w.default_cy)
    };
    let new_anchor = picture::build_anchor(
        target_kind,
        from,
        to,
        w.from_off_x.unwrap_or(snapshot.from_off_x),
        w.from_off_y.unwrap_or(snapshot.from_off_y),
        w.to_off_x.unwrap_or(snapshot.to_off_x),
        w.to_off_y.unwrap_or(snapshot.to_off_y),
        None,
        None,
        cx,
        cy,
        w.edit_as.unwrap_or(snapshot.edit_as),
        0,
        "",
        "",
        Some(snapshot.payload),
    );
    if !replace_outer(&mut doc.root, is_obj, key, new_anchor) {
        return Err(Error::with_path(not_found, path));
    }
    pkg.set(&drawing_part, doc.to_bytes()?)?;
    Ok(())
}

pub(crate) fn write_or_drop_drawing(
    pkg: &mut Package,
    sheet_doc: &mut Document,
    sheet_part: &str,
    drawing_part: &str,
    doc: Document,
) -> Result<(), Error> {
    if picture::drawing_is_empty(&doc.root) {
        pkg.remove(drawing_part);
        pkg.remove(&rels_part_name(drawing_part));
        picture::remove_drawing_ref(&mut sheet_doc.root, sheet_part, pkg, drawing_part)?;
        let mut cts = crate::opc::ContentTypes::from_package(pkg)?;
        cts.remove_override(drawing_part);
        cts.write_to(pkg)?;
    } else {
        pkg.set(drawing_part, doc.to_bytes()?)?;
    }
    Ok(())
}

pub(crate) fn for_each_object<F>(
    pkg: &Package,
    sheet_part: &str,
    is_obj: fn(&Element) -> bool,
    mut f: F,
) -> Result<(), Error>
where
    F: FnMut(&Element, AnchorKind, &Element) -> Result<(), Error>,
{
    let sheet = xml::parse(pkg.get(sheet_part)?)?;
    for drawing_part in picture::drawing_parts_from(pkg, sheet_part, &sheet.root)? {
        let doc = xml::parse(pkg.get(&drawing_part)?)?;
        picture::walk_anchors(&doc.root, &mut |anchor, kind| {
            visit_payloads(anchor, &mut |el| {
                if is_obj(el) {
                    f(anchor, kind, el)?;
                }
                Ok(())
            })
        })?;
    }
    Ok(())
}

pub(crate) fn find_drawing_for(
    pkg: &Package,
    sheet_part: &str,
    sheet_root: &Element,
    is_obj: fn(&Element) -> bool,
    key: &DrawKey,
) -> Option<String> {
    let mut index = 0usize;
    let Ok(parts) = picture::drawing_parts_from(pkg, sheet_part, sheet_root) else {
        return None;
    };
    for part in parts {
        let Ok(bytes) = pkg.get(&part) else {
            continue;
        };
        let Ok(doc) = xml::parse(bytes) else {
            continue;
        };
        let mut hit = false;
        let _ = picture::walk_anchors(&doc.root, &mut |anchor, _| {
            if hit {
                return Ok(());
            }
            visit_payloads(anchor, &mut |el| {
                if is_obj(el) {
                    index += 1;
                    if key_matches(key, &cnv_of_object(el).0, index) {
                        hit = true;
                    }
                }
                Ok(())
            })
        });
        if hit {
            return Some(part);
        }
    }
    None
}

pub(crate) fn is_textbox(el: &Element) -> bool {
    if el.local_name() != "sp" {
        return false;
    }
    let Some(nv) = el.child_by_local("nvSpPr") else {
        return false;
    };
    let Some(pr) = nv.child_by_local("cNvSpPr") else {
        return false;
    };
    matches!(
        pr.attr_local("txBox"),
        Some("1") | Some("true") | Some("True")
    )
}

pub(crate) fn key_matches(key: &DrawKey, raw: &Option<String>, index: usize) -> bool {
    match key {
        DrawKey::Index(n) => index == *n,
        DrawKey::Cnv(want) => {
            let Some(got) = raw else {
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

pub(crate) fn cnv_of_object(el: &Element) -> (Option<String>, String) {
    let nv = el
        .child_by_local("nvSpPr")
        .or_else(|| el.child_by_local("nvGraphicFramePr"));
    if let Some(nv) = nv {
        if let Some(pr) = nv.child_by_local("cNvPr") {
            let raw = pr
                .attr_local("id")
                .filter(|s| !s.is_empty())
                .map(str::to_string);
            let name = pr.attr_local("name").unwrap_or("").to_string();
            return (raw, name);
        }
    }
    (None, String::new())
}

fn is_geom(local: &str) -> bool {
    matches!(local, "from" | "to" | "ext" | "pos" | "clientData")
}

fn is_group_chrome(local: &str) -> bool {
    matches!(local, "nvGrpSpPr" | "grpSpPr")
}

fn first_choice(el: &Element) -> Option<&Element> {
    el.child_by_local("Choice")
        .or_else(|| el.child_by_local("Fallback"))
}

pub(crate) fn visit_payloads<F>(anchor: &Element, f: &mut F) -> Result<(), Error>
where
    F: FnMut(&Element) -> Result<(), Error>,
{
    for child in anchor.element_children() {
        if is_geom(child.local_name()) {
            continue;
        }
        visit_one(child, f)?;
    }
    Ok(())
}

fn visit_one<F>(el: &Element, f: &mut F) -> Result<(), Error>
where
    F: FnMut(&Element) -> Result<(), Error>,
{
    if el.local_name() == "AlternateContent" {
        if let Some(c) = first_choice(el) {
            return visit_one(c, f);
        }
        return Ok(());
    }
    if el.local_name() == "grpSp" {
        for c in el.element_children() {
            if is_group_chrome(c.local_name()) {
                continue;
            }
            visit_one(c, f)?;
        }
        return Ok(());
    }
    f(el)
}

fn outer_contains(
    anchor: &Element,
    is_obj: fn(&Element) -> bool,
    key: &DrawKey,
    index: &mut usize,
) -> bool {
    let mut hit = false;
    let _ = visit_payloads(anchor, &mut |el| {
        if is_obj(el) {
            *index += 1;
            if key_matches(key, &cnv_of_object(el).0, *index) {
                hit = true;
            }
        }
        Ok(())
    });
    hit
}

fn find_outer_kind(
    root: &Element,
    is_obj: fn(&Element) -> bool,
    key: &DrawKey,
) -> Option<AnchorKind> {
    let mut index = 0usize;
    let mut found = None;
    let _ = picture::walk_anchors(root, &mut |anchor, kind| {
        if found.is_some() {
            return Ok(());
        }
        if outer_contains(anchor, is_obj, key, &mut index) {
            found = Some(kind);
        }
        Ok(())
    });
    found
}

struct OuterSnap {
    from: Option<a1::CellRef>,
    to: Option<a1::CellRef>,
    from_off_x: i64,
    from_off_y: i64,
    to_off_x: i64,
    to_off_y: i64,
    cx: Option<i64>,
    cy: Option<i64>,
    edit_as: EditAs,
    payload: Element,
}

fn snapshot_outer(
    root: &Element,
    is_obj: fn(&Element) -> bool,
    key: &DrawKey,
) -> Option<OuterSnap> {
    let mut index = 0usize;
    let mut found = None;
    let _ = picture::walk_anchors(root, &mut |anchor, kind| {
        if found.is_some() {
            return Ok(());
        }
        if !outer_contains(anchor, is_obj, key, &mut index) {
            return Ok(());
        }
        let (from, from_off_x, from_off_y) = picture::read_marker(anchor.child_by_local("from"));
        let (to, to_off_x, to_off_y) = picture::read_marker(anchor.child_by_local("to"));
        let (cx, cy) = picture::read_ext(anchor);
        let Some(payload) = payload_of(anchor) else {
            return Ok(());
        };
        found = Some(OuterSnap {
            from,
            to,
            from_off_x,
            from_off_y,
            to_off_x,
            to_off_y,
            cx,
            cy,
            edit_as: if kind == AnchorKind::TwoCell {
                picture::parse_edit_as(anchor.attr("editAs"))
            } else {
                EditAs::TwoCell
            },
            payload,
        });
        Ok(())
    });
    found
}

fn payload_of(anchor: &Element) -> Option<Element> {
    anchor
        .element_children()
        .find(|c| !is_geom(c.local_name()))
        .cloned()
}

fn replace_outer(
    root: &mut Element,
    is_obj: fn(&Element) -> bool,
    key: &DrawKey,
    new_anchor: Element,
) -> bool {
    replace_outer_in(root, is_obj, key, &mut 0, new_anchor)
}

fn replace_outer_in(
    el: &mut Element,
    is_obj: fn(&Element) -> bool,
    key: &DrawKey,
    index: &mut usize,
    new_anchor: Element,
) -> bool {
    for i in 0..el.children.len() {
        let Node::Element(child) = &el.children[i] else {
            continue;
        };
        if AnchorKind::from_local(child.local_name()).is_some() {
            if outer_contains(child, is_obj, key, index) {
                el.children[i] = Node::Element(new_anchor);
                return true;
            }
            continue;
        }
        if let Node::Element(child) = &mut el.children[i] {
            if replace_outer_in(child, is_obj, key, index, new_anchor.clone()) {
                return true;
            }
        }
    }
    false
}

fn update_outer_extent(
    root: &mut Element,
    is_obj: fn(&Element) -> bool,
    key: &DrawKey,
    cx: Option<i64>,
    cy: Option<i64>,
) -> bool {
    let mut index = 0usize;
    picture::walk_anchors_mut(root, &mut |anchor, _| {
        if !outer_contains(anchor, is_obj, key, &mut index) {
            return false;
        }
        if let Some(ext) = anchor.child_by_local_mut("ext") {
            if let Some(v) = cx {
                picture::set_attr_local(ext, "cx", &v.to_string());
            }
            if let Some(v) = cy {
                picture::set_attr_local(ext, "cy", &v.to_string());
            }
        }
        if let Some(xfrm) = picture::find_local_mut(anchor, "xfrm") {
            if let Some(ext) = xfrm.child_by_local_mut("ext") {
                if let Some(v) = cx {
                    picture::set_attr_local(ext, "cx", &v.to_string());
                }
                if let Some(v) = cy {
                    picture::set_attr_local(ext, "cy", &v.to_string());
                }
            }
        }
        true
    })
}

fn update_absolute_pos(
    root: &mut Element,
    is_obj: fn(&Element) -> bool,
    key: &DrawKey,
    pos_x: Option<i64>,
    pos_y: Option<i64>,
) -> bool {
    let mut index = 0usize;
    picture::walk_anchors_mut(root, &mut |anchor, _| {
        if !outer_contains(anchor, is_obj, key, &mut index) {
            return false;
        }
        if let Some(pos) = anchor.child_by_local_mut("pos") {
            if let Some(v) = pos_x {
                picture::set_attr_local(pos, "x", &v.to_string());
            }
            if let Some(v) = pos_y {
                picture::set_attr_local(pos, "y", &v.to_string());
            }
        }
        true
    })
}

pub(crate) fn find_object_mut<'a>(
    root: &'a mut Element,
    is_obj: fn(&Element) -> bool,
    key: &DrawKey,
) -> Option<&'a mut Element> {
    find_object_in(root, is_obj, key, &mut 0)
}

fn find_object_in<'a>(
    el: &'a mut Element,
    is_obj: fn(&Element) -> bool,
    key: &DrawKey,
    index: &mut usize,
) -> Option<&'a mut Element> {
    if el.local_name() == "AlternateContent" {
        let i = el
            .children
            .iter()
            .position(|n| matches!(n, Node::Element(e) if e.local_name() == "Choice"))
            .or_else(|| {
                el.children
                    .iter()
                    .position(|n| matches!(n, Node::Element(e) if e.local_name() == "Fallback"))
            })?;
        let Node::Element(child) = &mut el.children[i] else {
            return None;
        };
        return find_object_in(child, is_obj, key, index);
    }
    if is_obj(el) {
        *index += 1;
        if key_matches(key, &cnv_of_object(el).0, *index) {
            return Some(el);
        }
        return None;
    }
    if el.local_name() == "grpSp" {
        for child in el.children.as_mut_slice() {
            if let Node::Element(e) = child {
                if is_group_chrome(e.local_name()) {
                    continue;
                }
                if let Some(found) = find_object_in(e, is_obj, key, index) {
                    return Some(found);
                }
            }
        }
        return None;
    }
    for child in el.children.as_mut_slice() {
        if let Node::Element(e) = child {
            if is_geom(e.local_name()) {
                continue;
            }
            if let Some(found) = find_object_in(e, is_obj, key, index) {
                return Some(found);
            }
        }
    }
    None
}

pub(crate) fn find_object<'a>(
    root: &'a Element,
    is_obj: fn(&Element) -> bool,
    key: &DrawKey,
) -> Option<&'a Element> {
    find_object_immut(root, is_obj, key, &mut 0)
}

fn find_object_immut<'a>(
    el: &'a Element,
    is_obj: fn(&Element) -> bool,
    key: &DrawKey,
    index: &mut usize,
) -> Option<&'a Element> {
    if el.local_name() == "AlternateContent" {
        if let Some(c) = first_choice(el) {
            return find_object_immut(c, is_obj, key, index);
        }
        return None;
    }
    if is_obj(el) {
        *index += 1;
        if key_matches(key, &cnv_of_object(el).0, *index) {
            return Some(el);
        }
        return None;
    }
    if el.local_name() == "grpSp" {
        for c in el.element_children() {
            if is_group_chrome(c.local_name()) {
                continue;
            }
            if let Some(found) = find_object_immut(c, is_obj, key, index) {
                return Some(found);
            }
        }
        return None;
    }
    for c in el.element_children() {
        if is_geom(c.local_name()) {
            continue;
        }
        if let Some(found) = find_object_immut(c, is_obj, key, index) {
            return Some(found);
        }
    }
    None
}

pub(crate) fn remove_object(
    root: &mut Element,
    is_obj: fn(&Element) -> bool,
    key: &DrawKey,
) -> bool {
    remove_in(root, is_obj, key, &mut 0)
}

fn remove_in(
    el: &mut Element,
    is_obj: fn(&Element) -> bool,
    key: &DrawKey,
    index: &mut usize,
) -> bool {
    let mut i = 0;
    while i < el.children.len() {
        let is_ac =
            matches!(&el.children[i], Node::Element(e) if e.local_name() == "AlternateContent");
        if is_ac {
            let mut found = false;
            if let Node::Element(ac) = &el.children[i] {
                if let Some(c) = first_choice(ac) {
                    peek_match(c, is_obj, key, index, &mut found);
                }
            }
            if found {
                el.children.remove(i);
                return true;
            }
            i += 1;
            continue;
        }
        let is_anchor = matches!(
            &el.children[i],
            Node::Element(e) if AnchorKind::from_local(e.local_name()).is_some()
        );
        if is_anchor {
            let hit = {
                let Node::Element(anchor) = &mut el.children[i] else {
                    i += 1;
                    continue;
                };
                remove_in(anchor, is_obj, key, index)
            };
            if hit {
                if let Node::Element(anchor) = &el.children[i] {
                    if !anchor_has_payload(anchor) {
                        el.children.remove(i);
                    }
                }
                return true;
            }
            i += 1;
            continue;
        }
        let is_grp = matches!(&el.children[i], Node::Element(e) if e.local_name() == "grpSp");
        if is_grp {
            let hit = {
                let Node::Element(g) = &mut el.children[i] else {
                    i += 1;
                    continue;
                };
                remove_in(g, is_obj, key, index)
            };
            if hit {
                if let Node::Element(g) = &el.children[i] {
                    if !grp_has_payload(g) {
                        el.children.remove(i);
                    }
                }
                return true;
            }
            i += 1;
            continue;
        }
        let is_hit = match &el.children[i] {
            Node::Element(e) if is_obj(e) => {
                *index += 1;
                key_matches(key, &cnv_of_object(e).0, *index)
            }
            _ => false,
        };
        if is_hit {
            el.children.remove(i);
            return true;
        }
        if let Node::Element(e) = &mut el.children[i] {
            if !is_geom(e.local_name())
                && !is_group_chrome(e.local_name())
                && !is_obj(e)
                && remove_in(e, is_obj, key, index)
            {
                return true;
            }
        }
        i += 1;
    }
    false
}

fn peek_match(
    el: &Element,
    is_obj: fn(&Element) -> bool,
    key: &DrawKey,
    index: &mut usize,
    found: &mut bool,
) {
    if el.local_name() == "AlternateContent" {
        if let Some(c) = first_choice(el) {
            peek_match(c, is_obj, key, index, found);
        }
        return;
    }
    if el.local_name() == "grpSp" {
        for c in el.element_children() {
            if is_group_chrome(c.local_name()) {
                continue;
            }
            peek_match(c, is_obj, key, index, found);
        }
        return;
    }
    if is_obj(el) {
        *index += 1;
        if key_matches(key, &cnv_of_object(el).0, *index) {
            *found = true;
        }
        return;
    }
    for c in el.element_children() {
        if is_geom(c.local_name()) {
            continue;
        }
        peek_match(c, is_obj, key, index, found);
    }
}

fn anchor_has_payload(anchor: &Element) -> bool {
    anchor
        .element_children()
        .any(|c| !is_geom(c.local_name()) && (c.local_name() != "grpSp" || grp_has_payload(c)))
}

fn grp_has_payload(grp: &Element) -> bool {
    grp.element_children().any(|c| {
        if is_group_chrome(c.local_name()) {
            return false;
        }
        if c.local_name() == "grpSp" {
            return grp_has_payload(c);
        }
        true
    })
}

pub(crate) fn drop_unused_rels(
    pkg: &mut Package,
    host: &str,
    rids: impl IntoIterator<Item = String>,
    still_used: impl Fn(&Element, &str) -> bool,
    host_root: &Element,
) -> Result<(), Error> {
    let mut rels = Relationships::from_package(pkg, host)?;
    let mut dirty = false;
    for rid in rids {
        if !still_used(host_root, &rid) {
            rels.remove_by_id(&rid);
            dirty = true;
        }
    }
    if dirty {
        if rels.rels.is_empty() {
            pkg.remove(&rels_part_name(host));
        } else {
            rels.write_to(pkg, host)?;
        }
    }
    Ok(())
}
