//! 工作表 drawing 公式框：`a14:m` AlternateContent。
//! 不进 `textBoxes` / `shapes`。不是单元格 `f`。

use serde_json::{json, Map, Value};

use crate::error::{Error, ErrorKind, IdReason, TypeName};
use crate::opc::{resolve_id, Package, Relationships};
use crate::xml::{self, Document, Element, Node};

use super::a1;
use super::picture::{self, AnchorKind, EditAs};
use super::shape;
use super::REL_NS;

pub use crate::drawing_math::{collect_acs, key_matches, set_in_index, MathKey};

#[derive(Debug, Clone)]
pub struct MathTarget {
    pub sheet: String,
    pub key: MathKey,
}

#[derive(Debug, Clone)]
pub struct InsertMathOp {
    pub sheet: String,
    pub text: String,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
    pub from: a1::CellRef,
    pub to: Option<a1::CellRef>,
    pub anchor: AnchorKind,
    pub from_off_x: i64,
    pub from_off_y: i64,
    pub to_off_x: i64,
    pub to_off_y: i64,
}

#[derive(Debug, Clone)]
pub struct SetMathOp {
    pub id: String,
    pub target: MathTarget,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct DeleteMathOp {
    pub id: String,
    pub target: MathTarget,
}

#[derive(Debug, Clone)]
pub struct SetMathSizeOp {
    pub id: String,
    pub target: MathTarget,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
    pub to: Option<a1::CellRef>,
    pub to_off_x: Option<i64>,
    pub to_off_y: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct SetMathAnchorOp {
    pub id: String,
    pub target: MathTarget,
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
    pub rel: Option<super::request::RelBoxPatch>,
}

pub struct MathInfo {
    pub id: String,
    pub cnv_pr_id: Option<String>,
    pub name: String,
    pub anchor: AnchorKind,
    pub from: Option<String>,
    pub from_off_x: Option<i64>,
    pub from_off_y: Option<i64>,
    pub to: Option<String>,
    pub to_off_x: Option<i64>,
    pub to_off_y: Option<i64>,
    pub edit_as: Option<EditAs>,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
    pub pos_x: Option<i64>,
    pub pos_y: Option<i64>,
    pub text: String,
    pub stable: bool,
}

pub fn looks_like_id(id: &str) -> bool {
    id.contains("/math:") && id.starts_with("sheet:")
}

pub fn parse_id(id: &str, json_path: &str) -> Result<MathTarget, Error> {
    let rest = id.strip_prefix("sheet:").ok_or_else(|| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SheetMathPrefix,
            },
            json_path.to_string(),
        )
    })?;
    let Some((name, after)) = rest.split_once("/math:") else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: if rest.contains('/') {
                    IdReason::SheetMathPrefix
                } else {
                    IdReason::SheetMathContainer
                },
            },
            json_path.to_string(),
        ));
    };
    if name.is_empty() || after.is_empty() {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SheetMathPrefix,
            },
            json_path.to_string(),
        ));
    }
    let key = parse_key(after, json_path)?;
    Ok(MathTarget {
        sheet: name.to_string(),
        key,
    })
}

fn parse_key(s: &str, path: &str) -> Result<MathKey, Error> {
    if let Some(rest) = s.strip_prefix("index:") {
        let n: usize = rest.parse().map_err(|_| {
            Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::SheetMathPrefix,
                },
                path.to_string(),
            )
        })?;
        if n < 1 {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::SheetMathPrefix,
                },
                path.to_string(),
            ));
        }
        return Ok(MathKey::Index(n));
    }
    if s.is_empty() || s.contains('/') {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SheetMathPrefix,
            },
            path.to_string(),
        ));
    }
    Ok(MathKey::Id(s.to_string()))
}

pub fn collect(
    pkg: &Package,
    sheet_part: &str,
    sheet_root: &Element,
    sheet_name: &str,
) -> Result<Vec<MathInfo>, Error> {
    let mut out = Vec::new();
    let mut index = 0usize;
    for part in picture::drawing_parts_from(pkg, sheet_part, sheet_root)? {
        let doc = xml::parse(pkg.get(&part)?)?;
        picture::walk_anchors(&doc.root, &mut |anchor, kind| {
            let mut acs = Vec::new();
            collect_acs(anchor, &mut acs);
            for ac in acs {
                index += 1;
                out.push(info_of(
                    anchor,
                    kind,
                    ac,
                    &format!("sheet:{sheet_name}"),
                    index,
                )?);
            }
            Ok(())
        })?;
    }
    Ok(out)
}

fn info_of(
    anchor: &Element,
    kind: AnchorKind,
    ac: &Element,
    id_base: &str,
    index: usize,
) -> Result<MathInfo, Error> {
    let (raw, name) = crate::drawing_math::cnv_of_ac(ac);
    let stable = raw.as_ref().is_some_and(|s| !s.is_empty());
    let id = match &raw {
        Some(v) if !v.is_empty() => format!("{id_base}/math:{v}"),
        _ => format!("{id_base}/math:index:{index}"),
    };
    let (from, from_off_x, from_off_y) = picture::read_marker(anchor.child_by_local("from"))?;
    let (to, to_off_x, to_off_y) = picture::read_marker(anchor.child_by_local("to"))?;
    let (cx, cy) = picture::read_ext(anchor);
    let (pos_x, pos_y) = picture::read_pos(anchor);
    let edit_as = if kind == AnchorKind::TwoCell {
        Some(picture::parse_edit_as(anchor.attr_local("editAs")))
    } else {
        None
    };
    Ok(MathInfo {
        id,
        cnv_pr_id: raw.filter(|s| !s.is_empty()),
        name,
        anchor: kind,
        from: from.map(|c| a1::format_cell(c.col, c.row)),
        from_off_x: if kind == AnchorKind::Absolute {
            None
        } else {
            Some(from_off_x)
        },
        from_off_y: if kind == AnchorKind::Absolute {
            None
        } else {
            Some(from_off_y)
        },
        to: to.map(|c| a1::format_cell(c.col, c.row)),
        to_off_x: if kind == AnchorKind::TwoCell {
            Some(to_off_x)
        } else {
            None
        },
        to_off_y: if kind == AnchorKind::TwoCell {
            Some(to_off_y)
        } else {
            None
        },
        edit_as,
        cx,
        cy,
        pos_x,
        pos_y,
        text: crate::drawing_math::choice_omml_text(ac),
        stable,
    })
}

pub fn list_json(items: &[MathInfo]) -> Vec<Value> {
    items.iter().map(info_json).collect()
}

fn info_json(p: &MathInfo) -> Value {
    let mut m = Map::new();
    m.insert("id".into(), json!(p.id));
    m.insert("cNvPrId".into(), json!(p.cnv_pr_id));
    m.insert("name".into(), json!(p.name));
    m.insert("anchor".into(), json!(p.anchor.as_str()));
    match p.anchor {
        AnchorKind::TwoCell => {
            m.insert("from".into(), json!(p.from));
            m.insert("fromOffX".into(), json!(p.from_off_x.unwrap_or(0)));
            m.insert("fromOffY".into(), json!(p.from_off_y.unwrap_or(0)));
            m.insert("to".into(), json!(p.to));
            m.insert("toOffX".into(), json!(p.to_off_x.unwrap_or(0)));
            m.insert("toOffY".into(), json!(p.to_off_y.unwrap_or(0)));
            m.insert(
                "editAs".into(),
                json!(p.edit_as.unwrap_or(EditAs::TwoCell).as_str()),
            );
        }
        AnchorKind::OneCell => {
            m.insert("from".into(), json!(p.from));
            m.insert("fromOffX".into(), json!(p.from_off_x.unwrap_or(0)));
            m.insert("fromOffY".into(), json!(p.from_off_y.unwrap_or(0)));
            m.insert("cx".into(), json!(p.cx));
            m.insert("cy".into(), json!(p.cy));
        }
        AnchorKind::Absolute => {
            m.insert("posX".into(), json!(p.pos_x));
            m.insert("posY".into(), json!(p.pos_y));
            m.insert("cx".into(), json!(p.cx));
            m.insert("cy".into(), json!(p.cy));
        }
        AnchorKind::RelSize | AnchorKind::AbsSize => {}
    }
    m.insert("text".into(), json!(p.text));
    m.insert("stable".into(), json!(p.stable));
    Value::Object(m)
}

fn drawing_for(
    pkg: &Package,
    sheet_part: &str,
    sheet_root: &Element,
    key: &MathKey,
    id: &str,
    path: &str,
) -> Result<String, Error> {
    for part in picture::drawing_parts_from(pkg, sheet_part, sheet_root)? {
        let doc = xml::parse(pkg.get(&part)?)?;
        if find_ac(&doc.root, key).is_some() {
            return Ok(part);
        }
    }
    Err(Error::with_path(
        ErrorKind::MathDrawingNotFound { id: id.to_string() },
        path.to_string(),
    ))
}

fn find_ac<'a>(root: &'a Element, key: &MathKey) -> Option<&'a Element> {
    let mut acs = Vec::new();
    collect_acs(root, &mut acs);
    for (i, ac) in acs.into_iter().enumerate() {
        if key_matches(key, &crate::drawing_math::cnv_of_ac(ac).0, i + 1) {
            return Some(ac);
        }
    }
    None
}

pub fn insert(
    pkg: &mut Package,
    sheet_doc: &mut Document,
    sheet_part: &str,
    op: &InsertMathOp,
) -> Result<(), Error> {
    let (cx, cy) = match (op.cx, op.cy) {
        (Some(x), Some(y)) => (x, y),
        _ => (
            crate::drawing_math::DEFAULT_CX,
            crate::drawing_math::DEFAULT_CY,
        ),
    };
    let drawing_part = picture::ensure_drawing_part(pkg, sheet_doc, sheet_part)?;
    let mut draw_doc = xml::parse(pkg.get(&drawing_part)?)?;
    picture::ensure_xmlns(&mut draw_doc.root, "r", REL_NS);
    picture::ensure_xmlns(&mut draw_doc.root, "a", picture::A_NS);
    picture::ensure_xmlns(&mut draw_doc.root, "mc", crate::drawing_math::MC_NS);
    picture::ensure_xmlns(&mut draw_doc.root, "a14", crate::drawing_math::A14_NS);
    picture::ensure_xmlns(&mut draw_doc.root, "m", crate::drawing_math::MATH_NS);
    crate::drawing_math::ensure_ignorable(&mut draw_doc.root, "a14");
    let cnv = picture::next_cnv_id(pkg, sheet_part, &sheet_doc.root)?;
    let name = format!("公式 {cnv}");
    let ac = build_xlsx_ac(cnv, &name, &op.text, cx, cy);
    let mut anchor = Element::new(format!("xdr:{}", op.anchor.local_name()));
    if op.anchor == AnchorKind::TwoCell {
        anchor.set_attr("editAs", EditAs::TwoCell.as_str());
    }
    match op.anchor {
        AnchorKind::TwoCell => {
            anchor.push_element(picture::marker_el(
                "xdr:from",
                op.from,
                op.from_off_x,
                op.from_off_y,
            ));
            anchor.push_element(picture::marker_el(
                "xdr:to",
                op.to.unwrap_or(op.from),
                op.to_off_x,
                op.to_off_y,
            ));
        }
        AnchorKind::OneCell => {
            anchor.push_element(picture::marker_el(
                "xdr:from",
                op.from,
                op.from_off_x,
                op.from_off_y,
            ));
            anchor.push_element(
                Element::new("xdr:ext")
                    .with_attr("cx", cx.to_string())
                    .with_attr("cy", cy.to_string()),
            );
        }
        AnchorKind::Absolute => {
            return Err(Error::with_path(ErrorKind::InvalidXlsxAnchor, "ops.anchor"));
        }
        AnchorKind::RelSize | AnchorKind::AbsSize => {
            return Err(Error::with_path(ErrorKind::InvalidXlsxAnchor, "ops.anchor"));
        }
    }
    anchor.push_element(ac);
    anchor.push_element(Element::new("xdr:clientData"));
    draw_doc.root.push_element(anchor);
    pkg.set(&drawing_part, draw_doc.to_bytes()?)?;
    Ok(())
}

fn build_xlsx_ac(cnv: u32, name: &str, text: &str, cx: i64, cy: i64) -> Element {
    let mut ac = Element::new("mc:AlternateContent");
    let mut choice = Element::new("mc:Choice").with_attr("Requires", "a14");
    choice.push_element(build_xlsx_sp(cnv, name, cx, cy, true, text));
    ac.push_element(choice);
    let mut fallback = Element::new("mc:Fallback");
    fallback.push_element(build_xlsx_sp(cnv, name, cx, cy, false, text));
    ac.push_element(fallback);
    ac
}

fn build_xlsx_sp(cnv: u32, name: &str, cx: i64, cy: i64, math: bool, text: &str) -> Element {
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
    if math {
        tx.push_element(crate::drawing_math::math_p(text));
    } else {
        tx.push_element(crate::drawing_math::fallback_p(text));
    }
    let mut sp = Element::new("xdr:sp");
    sp.push_element(nv);
    sp.push_element(sp_pr);
    sp.push_element(tx);
    sp
}

pub fn set(
    pkg: &mut Package,
    sheet_doc: &Document,
    sheet_part: &str,
    op: &SetMathOp,
    op_index: usize,
) -> Result<(), Error> {
    let path = format!("ops[{op_index}].id");
    let drawing_part = drawing_for(
        pkg,
        sheet_part,
        &sheet_doc.root,
        &op.target.key,
        &op.id,
        &path,
    )?;
    let mut doc = xml::parse(pkg.get(&drawing_part)?)?;
    if !set_in(&mut doc.root, &op.target.key, &op.text) {
        return Err(Error::with_path(
            ErrorKind::MathDrawingNotFound { id: op.id.clone() },
            path,
        ));
    }
    pkg.set(&drawing_part, doc.to_bytes()?)?;
    Ok(())
}

fn draw_key(key: &MathKey) -> shape::DrawKey {
    match key {
        MathKey::Id(s) => shape::DrawKey::Cnv(s.clone()),
        MathKey::Index(n) => shape::DrawKey::Index(*n),
    }
}

/// 改外层 `xdr:ext` 或 `twoCell` 的 `to`。不改字。
pub fn set_size(
    pkg: &mut Package,
    sheet_root: &Element,
    sheet_part: &str,
    op: &SetMathSizeOp,
    op_index: usize,
) -> Result<(), Error> {
    shape::set_object_size(
        pkg,
        sheet_root,
        sheet_part,
        &draw_key(&op.target.key),
        crate::drawing_math::is_math_ac,
        picture::SizePatch {
            cx: op.cx,
            cy: op.cy,
            to: op.to,
            to_off_x: op.to_off_x,
            to_off_y: op.to_off_y,
        },
        ErrorKind::TwoCellMathSize,
        ErrorKind::MathDrawingNotFound { id: op.id.clone() },
        "setMathSize",
        op_index,
    )
}

/// 改包含 AlternateContent 的外层锚点。留下 AlternateContent 与 clientData。
pub fn set_anchor(
    pkg: &mut Package,
    sheet_root: &Element,
    sheet_part: &str,
    op: &SetMathAnchorOp,
    op_index: usize,
) -> Result<(), Error> {
    shape::set_object_anchor(
        pkg,
        sheet_root,
        sheet_part,
        &draw_key(&op.target.key),
        crate::drawing_math::is_math_ac,
        shape::AnchorWrite {
            op: "setMathAnchor",
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
            default_cx: crate::drawing_math::DEFAULT_CX,
            default_cy: crate::drawing_math::DEFAULT_CY,
            rel: op.rel,
        },
        op_index,
        ErrorKind::MathDrawingNotFound { id: op.id.clone() },
    )
}

fn set_in(el: &mut Element, key: &MathKey, text: &str) -> bool {
    set_in_index(el, key, text, &mut 0)
}

pub fn delete(
    pkg: &mut Package,
    sheet_doc: &mut Document,
    sheet_part: &str,
    op: &DeleteMathOp,
    op_index: usize,
) -> Result<(), Error> {
    let path = format!("ops[{op_index}].id");
    let drawing_part = drawing_for(
        pkg,
        sheet_part,
        &sheet_doc.root,
        &op.target.key,
        &op.id,
        &path,
    )?;
    let mut doc = xml::parse(pkg.get(&drawing_part)?)?;
    let rids = find_ac(&doc.root, &op.target.key)
        .map(crate::drawing_math::collect_embed_rids)
        .unwrap_or_default();
    if !remove_math(&mut doc.root, &op.target.key) {
        return Err(Error::with_path(
            ErrorKind::MathDrawingNotFound { id: op.id.clone() },
            path,
        ));
    }
    remove_empty_anchors(&mut doc.root);
    pkg.set(&drawing_part, doc.to_bytes()?)?;
    drop_rids(pkg, &drawing_part, &doc.root, &rids)?;
    Ok(())
}

fn remove_math(el: &mut Element, key: &MathKey) -> bool {
    remove_in(el, key, &mut 0)
}

fn remove_in(el: &mut Element, key: &MathKey, index: &mut usize) -> bool {
    let mut drop = None;
    for (i, n) in el.children.iter_mut().enumerate() {
        let Node::Element(child) = n else { continue };
        if child.local_name() == "AlternateContent" && crate::drawing_math::is_math_ac(child) {
            *index += 1;
            if key_matches(key, &crate::drawing_math::cnv_of_ac(child).0, *index) {
                drop = Some(i);
                break;
            }
        } else if remove_in(child, key, index) {
            if AnchorKind::from_local(el.local_name()).is_some() && anchor_empty(el) {
                return true;
            }
            return true;
        }
    }
    if let Some(i) = drop {
        el.children.remove(i);
        return true;
    }
    false
}

fn remove_empty_anchors(el: &mut Element) {
    el.children.retain(|n| {
        let Node::Element(child) = n else {
            return true;
        };
        !(AnchorKind::from_local(child.local_name()).is_some() && anchor_empty(child))
    });
    for n in el.children.iter_mut() {
        if let Node::Element(child) = n {
            remove_empty_anchors(child);
        }
    }
}

fn anchor_empty(el: &Element) -> bool {
    !el.element_children().any(|c| {
        let n = c.local_name();
        !matches!(n, "from" | "to" | "ext" | "pos" | "clientData")
    })
}

fn drop_rids(pkg: &mut Package, part: &str, root: &Element, rids: &[String]) -> Result<(), Error> {
    let mut rels = Relationships::from_package(pkg, part)?;
    let mut dirty = false;
    let mut media_parts = Vec::new();
    for rid in rids {
        if rid_still_used(root, rid) {
            continue;
        }
        if let Ok(m) = resolve_id(pkg, part, rid) {
            if !media_parts.iter().any(|x| x == &m) {
                media_parts.push(m);
            }
        }
        rels.remove_by_id(rid);
        dirty = true;
    }
    if dirty {
        if rels.rels.is_empty() {
            pkg.remove(&crate::opc::rels_part_name(part));
        } else {
            rels.write_to(pkg, part)?;
        }
    }
    for m in media_parts {
        if !picture::media_still_used(pkg, &m)? {
            pkg.remove(&m);
        }
    }
    Ok(())
}

fn rid_still_used(root: &Element, rid: &str) -> bool {
    let mut used = false;
    crate::model3d::walk(root, &mut |e| {
        if e.attr_local("embed") == Some(rid)
            || e.attr_local("id") == Some(rid) && e.local_name() != "cNvPr"
        {
            used = true;
        }
    });
    used
}

pub fn parse_create(v: &Value, prefix: &str, sheet: &str) -> Result<InsertMathOp, Error> {
    let obj = v.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: prefix.into(),
                expected: TypeName::Object,
            },
            prefix,
        )
    })?;
    crate::json::deny_unknown(
        obj,
        &[
            "text", "cx", "cy", "from", "to", "anchor", "fromOffX", "fromOffY", "toOffX", "toOffY",
            "expr", "kind",
        ],
    )?;
    crate::drawing_math::reject_expr_kind(obj, prefix)?;
    let text = crate::drawing_math::require_text(obj, prefix)?;
    let cx = parse_emu(obj.get("cx"), prefix, "cx")?;
    let cy = parse_emu(obj.get("cy"), prefix, "cy")?;
    if cx.is_some() != cy.is_some() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "insertMath".into(),
                field: "cx and cy together".into(),
            },
            prefix.to_string(),
        ));
    }
    let from = match obj.get("from") {
        Some(Value::String(s)) => a1::parse_cell(s).map_err(|e| e.at(format!("{prefix}.from")))?,
        None => a1::CellRef { col: 1, row: 1 },
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "from".into(),
                    expected: TypeName::String,
                },
                format!("{prefix}.from"),
            ));
        }
    };
    let anchor = match obj.get("anchor") {
        None => AnchorKind::OneCell,
        Some(Value::String(s)) => picture::AnchorKind::parse_insert(s).ok_or_else(|| {
            Error::with_path(ErrorKind::InvalidXlsxAnchor, format!("{prefix}.anchor"))
        })?,
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "anchor".into(),
                    expected: TypeName::String,
                },
                format!("{prefix}.anchor"),
            ));
        }
    };
    let to = match obj.get("to") {
        None => None,
        Some(Value::String(s)) => {
            Some(a1::parse_cell(s).map_err(|e| e.at(format!("{prefix}.to")))?)
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "to".into(),
                    expected: TypeName::String,
                },
                format!("{prefix}.to"),
            ));
        }
    };
    if anchor == AnchorKind::TwoCell && to.is_none() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "insertMath".into(),
                field: "to".into(),
            },
            format!("{prefix}.to"),
        ));
    }
    if anchor == AnchorKind::OneCell && to.is_some() {
        return Err(Error::with_path(
            ErrorKind::UnknownField { field: "to".into() },
            format!("{prefix}.to"),
        ));
    }
    Ok(InsertMathOp {
        sheet: sheet.to_string(),
        text,
        cx,
        cy,
        from,
        to,
        anchor,
        from_off_x: parse_off(obj.get("fromOffX"), prefix, "fromOffX")?.unwrap_or(0),
        from_off_y: parse_off(obj.get("fromOffY"), prefix, "fromOffY")?.unwrap_or(0),
        to_off_x: parse_off(obj.get("toOffX"), prefix, "toOffX")?.unwrap_or(0),
        to_off_y: parse_off(obj.get("toOffY"), prefix, "toOffY")?.unwrap_or(0),
    })
}

fn parse_emu(v: Option<&Value>, prefix: &str, field: &str) -> Result<Option<i64>, Error> {
    let Some(v) = v else {
        return Ok(None);
    };
    let path = format!("{prefix}.{field}");
    match v {
        Value::Number(n) => {
            if let Some(u) = n.as_u64() {
                if u == 0 || u > i64::MAX as u64 {
                    return Err(Error::with_path(
                        ErrorKind::MinValue {
                            field: field.into(),
                            min: 1,
                        },
                        path,
                    ));
                }
                return Ok(Some(u as i64));
            }
            Err(Error::with_path(
                ErrorKind::WrongType {
                    field: field.into(),
                    expected: TypeName::Integer,
                },
                path,
            ))
        }
        _ => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Integer,
            },
            path,
        )),
    }
}

fn parse_off(v: Option<&Value>, prefix: &str, field: &str) -> Result<Option<i64>, Error> {
    let Some(v) = v else {
        return Ok(None);
    };
    let path = format!("{prefix}.{field}");
    match v {
        Value::Number(n) => {
            if let Some(u) = n.as_u64() {
                if u > i64::MAX as u64 {
                    return Err(Error::with_path(
                        ErrorKind::WrongType {
                            field: field.into(),
                            expected: TypeName::Integer,
                        },
                        path,
                    ));
                }
                return Ok(Some(u as i64));
            }
            if let Some(i64v) = n.as_i64() {
                if i64v < 0 {
                    return Err(Error::with_path(
                        ErrorKind::MinValue {
                            field: field.into(),
                            min: 0,
                        },
                        path,
                    ));
                }
                return Ok(Some(i64v));
            }
            Err(Error::with_path(
                ErrorKind::WrongType {
                    field: field.into(),
                    expected: TypeName::Integer,
                },
                path,
            ))
        }
        _ => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Integer,
            },
            path,
        )),
    }
}
