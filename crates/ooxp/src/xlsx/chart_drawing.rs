//! 图表工作表 Chart Drawing：`c:userShapes` / `cdr:relSizeAnchor`。

use crate::error::{Error, ErrorKind};
use crate::opc::{
    rels_part_name, resolve_rel_target, ContentTypes, Package, Relationship, Relationships,
};
use crate::xml::{self, Document, Element, Node, DEFAULT_DECL};
use serde_json::json;

use super::picture::{self, AnchorKind};
use super::request::{RelBox, RelBoxPatch};
use super::REL_NS;

pub const CDR_NS: &str = "http://schemas.openxmlformats.org/drawingml/2006/chartDrawing";
pub const REL_CHART_USER_SHAPES: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/chartUserShapes";
pub const CT_CHART_SHAPES: &str =
    "application/vnd.openxmlformats-officedocument.drawingml.chartshapes+xml";
pub const CHART_NS: &str = "http://schemas.openxmlformats.org/drawingml/2006/chart";

pub fn is_user_shapes(root: &Element) -> bool {
    root.local_name() == "userShapes"
}

pub fn user_shape_parts(
    pkg: &Package,
    sheet_part: &str,
    sheet_root: &Element,
) -> Result<Vec<String>, Error> {
    let mut out = Vec::new();
    for chart_part in chart_parts(pkg, sheet_part, sheet_root)? {
        let rels = Relationships::from_package(pkg, &chart_part)?;
        for rel in &rels.rels {
            if rel.is_external() {
                continue;
            }
            if !(rel.rel_type == REL_CHART_USER_SHAPES
                || rel.rel_type.ends_with("/chartUserShapes"))
            {
                continue;
            }
            let part = resolve_rel_target(&chart_part, &rel.target)?;
            if pkg.contains(&part) && !out.contains(&part) {
                out.push(part);
            }
        }
    }
    Ok(out)
}

pub fn object_drawing_parts_from(
    pkg: &Package,
    sheet_part: &str,
    sheet_root: &Element,
) -> Result<Vec<String>, Error> {
    let mut out = picture::drawing_parts_from(pkg, sheet_part, sheet_root)?;
    for part in user_shape_parts(pkg, sheet_part, sheet_root)? {
        if !out.contains(&part) {
            out.push(part);
        }
    }
    Ok(out)
}

/// 往图表工作表插图 / 形状 / 框 / 连接线时，写入该表那张图的 Chart Drawing。
pub fn ensure_user_shapes(
    pkg: &mut Package,
    sheet_part: &str,
    sheet_root: &Element,
) -> Result<String, Error> {
    if let Some(part) = user_shape_parts(pkg, sheet_part, sheet_root)?
        .into_iter()
        .next()
    {
        return Ok(part);
    }
    let chart_part = first_chart_part(pkg, sheet_part, sheet_root)?;
    let part = next_chartshapes_part(pkg);
    let root = Element::new("c:userShapes")
        .with_attr("xmlns:c", CHART_NS)
        .with_attr("xmlns:a", picture::A_NS)
        .with_attr("xmlns:r", REL_NS)
        .with_attr("xmlns:cdr", CDR_NS);
    pkg.set(
        &part,
        Document::new(Some(DEFAULT_DECL.to_string()), root).to_bytes()?,
    )?;
    let mut rels = Relationships::from_package(pkg, &chart_part)?;
    let rid = rels.next_rid();
    rels.push(Relationship {
        id: rid.clone(),
        rel_type: REL_CHART_USER_SHAPES.into(),
        target: picture::rel_target_from(&chart_part, &part),
        target_mode: None,
        extra: Vec::new(),
    });
    rels.write_to(pkg, &chart_part)?;
    let mut chart_doc = xml::parse(pkg.get(&chart_part)?)?;
    picture::ensure_xmlns(&mut chart_doc.root, "r", REL_NS);
    if chart_doc.root.child_by_local("userShapes").is_none() {
        chart_doc
            .root
            .push_element(Element::new("c:userShapes").with_attr("r:id", rid));
        pkg.set(&chart_part, chart_doc.to_bytes()?)?;
    }
    let mut cts = ContentTypes::from_package(pkg)?;
    cts.set_override(&part, CT_CHART_SHAPES)?;
    cts.write_to(pkg)?;
    Ok(part)
}

pub fn drop_user_shapes(
    pkg: &mut Package,
    sheet_part: &str,
    drawing_part: &str,
) -> Result<(), Error> {
    let sheet = xml::parse(pkg.get(sheet_part)?)?;
    for chart_part in chart_parts(pkg, sheet_part, &sheet.root)? {
        let mut rels = Relationships::from_package(pkg, &chart_part)?;
        let before = rels.rels.len();
        rels.remove_by_target_part(&chart_part, drawing_part);
        if rels.rels.len() != before {
            if rels.rels.is_empty() {
                pkg.remove(&rels_part_name(&chart_part));
            } else {
                rels.write_to(pkg, &chart_part)?;
            }
            let mut chart_doc = xml::parse(pkg.get(&chart_part)?)?;
            chart_doc.root.children.retain(|n| match n {
                Node::Element(e) => e.local_name() != "userShapes",
                _ => true,
            });
            pkg.set(&chart_part, chart_doc.to_bytes()?)?;
        }
    }
    pkg.remove(drawing_part);
    pkg.remove(&rels_part_name(drawing_part));
    let mut cts = ContentTypes::from_package(pkg)?;
    cts.remove_override(drawing_part);
    cts.write_to(pkg)?;
    Ok(())
}

pub fn push_rel_anchor(
    pkg: &mut Package,
    drawing_part: &str,
    rel: RelBox,
    mut payload: Element,
) -> Result<(), Error> {
    retag_xdr_to_cdr(&mut payload);
    let mut doc = xml::parse(pkg.get(drawing_part)?)?;
    picture::ensure_xmlns(&mut doc.root, "r", REL_NS);
    picture::ensure_xmlns(&mut doc.root, "a", picture::A_NS);
    picture::ensure_xmlns(&mut doc.root, "cdr", CDR_NS);
    picture::ensure_xmlns(&mut doc.root, "c", CHART_NS);
    doc.root.push_element(build_rel_size_anchor(rel, payload));
    pkg.set(drawing_part, doc.to_bytes()?)?;
    Ok(())
}

pub fn build_rel_size_anchor(rel: RelBox, payload: Element) -> Element {
    let mut anchor = Element::new("cdr:relSizeAnchor");
    anchor.push_element(rel_pt("cdr:from", rel.from_x, rel.from_y));
    anchor.push_element(rel_pt("cdr:to", rel.to_x, rel.to_y));
    anchor.push_element(payload);
    anchor
}

pub fn read_rel_box(anchor: &Element) -> (Option<f64>, Option<f64>, Option<f64>, Option<f64>) {
    let (fx, fy) = read_rel_pt(anchor.child_by_local("from"));
    let (tx, ty) = read_rel_pt(anchor.child_by_local("to"));
    (fx, fy, tx, ty)
}

pub fn patch_rel_box(anchor: &mut Element, patch: RelBoxPatch) -> Result<(), Error> {
    if let Some(from) = anchor.child_by_local_mut("from") {
        set_rel_coord(from, "x", patch.from_x);
        set_rel_coord(from, "y", patch.from_y);
    }
    if let Some(to) = anchor.child_by_local_mut("to") {
        set_rel_coord(to, "x", patch.to_x);
        set_rel_coord(to, "y", patch.to_y);
    }
    Ok(())
}

pub fn reject_cell_anchor_fields(
    from_cell: bool,
    to_cell: bool,
    from_off: bool,
    to_off: bool,
    op_index: usize,
) -> Result<(), Error> {
    for (hit, field) in [
        (from_cell, "from"),
        (to_cell, "to"),
        (from_off, "fromOffX"),
        (to_off, "toOffX"),
    ] {
        if hit {
            return Err(Error::with_path(
                ErrorKind::UnknownField {
                    field: field.into(),
                },
                format!("ops[{op_index}].{field}"),
            ));
        }
    }
    Ok(())
}

fn rel_pt(name: &str, x: f64, y: f64) -> Element {
    let mut el = Element::new(name);
    el.push_element(Element::new("cdr:x").with_text(fmt_frac(x)));
    el.push_element(Element::new("cdr:y").with_text(fmt_frac(y)));
    el
}

fn read_rel_pt(el: Option<&Element>) -> (Option<f64>, Option<f64>) {
    let Some(el) = el else {
        return (None, None);
    };
    let x = el
        .child_by_local("x")
        .and_then(|e| e.text_content().trim().parse().ok());
    let y = el
        .child_by_local("y")
        .and_then(|e| e.text_content().trim().parse().ok());
    (x, y)
}

fn set_rel_coord(el: &mut Element, local: &str, v: Option<f64>) {
    let Some(v) = v else {
        return;
    };
    if let Some(child) = el.child_by_local_mut(local) {
        child.set_text(fmt_frac(v));
        return;
    }
    el.push_element(Element::new(format!("cdr:{local}")).with_text(fmt_frac(v)));
}

fn fmt_frac(v: f64) -> String {
    let s = format!("{v}");
    if s.contains('.') || s.contains('e') || s.contains('E') {
        s
    } else {
        format!("{v:.1}")
    }
}

fn retag_xdr_to_cdr(el: &mut Element) {
    if let Some(rest) = el.name.strip_prefix("xdr:") {
        el.name = format!("cdr:{rest}");
    }
    for n in &mut el.children {
        if let Node::Element(c) = n {
            retag_xdr_to_cdr(c);
        }
    }
}

fn chart_parts(
    pkg: &Package,
    sheet_part: &str,
    sheet_root: &Element,
) -> Result<Vec<String>, Error> {
    let mut out = Vec::new();
    for drawing_part in picture::drawing_parts_from(pkg, sheet_part, sheet_root)? {
        let doc = xml::parse(pkg.get(&drawing_part)?)?;
        let rels = Relationships::from_package(pkg, &drawing_part)?;
        picture::walk_anchors(&doc.root, &mut |anchor, _| {
            if let Some(rid) = chart_rid(anchor) {
                if let Some(rel) = rels.find_by_id(&rid) {
                    if !rel.is_external() {
                        if let Ok(part) = resolve_rel_target(&drawing_part, &rel.target) {
                            if pkg.contains(&part) && !out.contains(&part) {
                                out.push(part);
                            }
                        }
                    }
                }
            }
            Ok(())
        })?;
    }
    Ok(out)
}

fn first_chart_part(
    pkg: &Package,
    sheet_part: &str,
    sheet_root: &Element,
) -> Result<String, Error> {
    chart_parts(pkg, sheet_part, sheet_root)?
        .into_iter()
        .next()
        .ok_or_else(|| Error::with_path(ErrorKind::UnknownField { field: "id".into() }, "ops.id"))
}

fn chart_rid(anchor: &Element) -> Option<String> {
    let mut found = None;
    picture::walk_all(anchor, &mut |e| {
        if found.is_none() && e.local_name() == "chart" && e.attr_local("id").is_some() {
            found = e.attr_local("id").map(str::to_string);
        }
    });
    found
}

fn next_chartshapes_part(pkg: &Package) -> String {
    let mut n = 1u32;
    loop {
        let name = format!("/xl/drawings/chartshapes{n}.xml");
        if !pkg.contains(&name) {
            return name;
        }
        n += 1;
    }
}

pub fn fill_rel_json(m: &mut serde_json::Map<String, serde_json::Value>, anchor: &Element) {
    let (fx, fy, tx, ty) = read_rel_box(anchor);
    let (cx, cy) = picture::read_ext(anchor);
    match AnchorKind::from_local(anchor.local_name()) {
        Some(AnchorKind::RelSize) => {
            m.insert("fromX".into(), json_num(fx));
            m.insert("fromY".into(), json_num(fy));
            m.insert("toX".into(), json_num(tx));
            m.insert("toY".into(), json_num(ty));
        }
        Some(AnchorKind::AbsSize) => {
            m.insert("fromX".into(), json_num(fx));
            m.insert("fromY".into(), json_num(fy));
            m.insert("cx".into(), json_int(cx));
            m.insert("cy".into(), json_int(cy));
        }
        _ => {}
    }
}

fn json_num(v: Option<f64>) -> serde_json::Value {
    v.map(|n| json!(n)).unwrap_or(serde_json::Value::Null)
}

fn json_int(v: Option<i64>) -> serde_json::Value {
    v.map(|n| json!(n)).unwrap_or(serde_json::Value::Null)
}
