//! 工作表 `<hyperlinks>`：inspect 名单，apply 写 XML 与工作表关系。

use serde_json::{json, Map, Value};

use super::a1::{self, CellRef};
use super::layout;
use super::picture;
use super::request::{
    DeleteHyperlinkOp, DrawingHlinkHost, InsertHyperlinkOp, Patch, SetHyperlinkOp,
};
use super::shape;
use super::smartart;
use super::{REL_HYPERLINK, REL_NS};
use crate::error::{Error, ErrorKind};
use crate::opc::{Package, Relationship, Relationships};
use crate::xml::{self, Element, Node};

pub(crate) fn count(root: &Element) -> usize {
    root.child_by_local("hyperlinks")
        .map(|p| p.children_by_local("hyperlink").len())
        .unwrap_or(0)
}

pub(crate) fn inspect(root: &Element, sheet: &str, rels: &Relationships) -> Value {
    let Some(parent) = root.child_by_local("hyperlinks") else {
        return json!([]);
    };
    let mut out = Vec::new();
    for (i, el) in parent
        .children_by_local("hyperlink")
        .into_iter()
        .enumerate()
    {
        out.push(inspect_one(el, sheet, i + 1, rels));
    }
    Value::Array(out)
}

fn inspect_one(el: &Element, sheet: &str, n: usize, rels: &Relationships) -> Value {
    let rid = el.attr_local("id").map(str::to_string);
    let kind = if rid.is_some() { "url" } else { "location" };
    let refer = match el.attr("ref") {
        Some(raw) => match a1::parse_range(raw) {
            Ok((a, b)) => json!(a1::format_range(a, b)),
            Err(_) => json!(raw),
        },
        None => Value::Null,
    };
    let url = rid
        .as_deref()
        .and_then(|id| rels.find_by_id(id))
        .map(|r| json!(r.target))
        .unwrap_or(Value::Null);
    let mut obj = Map::new();
    obj.insert("id".into(), json!(format!("sheet:{sheet}/link:{n}")));
    obj.insert("index".into(), json!(n));
    obj.insert("kind".into(), json!(kind));
    obj.insert("ref".into(), refer);
    obj.insert("rId".into(), rid.map(Value::String).unwrap_or(Value::Null));
    obj.insert("url".into(), url);
    obj.insert("location".into(), opt_str(el.attr("location")));
    obj.insert("tooltip".into(), opt_str(el.attr("tooltip")));
    obj.insert("display".into(), opt_str(el.attr("display")));
    Value::Object(obj)
}

fn opt_str(v: Option<&str>) -> Value {
    v.map(|s| json!(s)).unwrap_or(Value::Null)
}

pub(crate) fn apply_insert(
    pkg: &mut Package,
    worksheet: &mut Element,
    sheet_part: &str,
    op: &InsertHyperlinkOp,
    ref_path: &str,
) -> Result<(), Error> {
    if overlaps(worksheet, op.start, op.end, None) {
        return Err(Error::with_path(
            ErrorKind::HyperlinkOverlap,
            ref_path.to_string(),
        ));
    }
    let rid = if let Some(url) = op.url.as_deref() {
        picture::ensure_xmlns(worksheet, "r", REL_NS);
        Some(push_hyperlink_rel(pkg, sheet_part, url)?)
    } else {
        None
    };
    let mut el = Element::new("hyperlink");
    el.set_attr("ref", &op.refer);
    if let Some(rid) = rid.as_deref() {
        el.set_attr("r:id", rid);
    }
    if let Some(loc) = op.location.as_deref() {
        el.set_attr("location", loc);
    }
    if let Some(t) = op.tooltip.as_deref() {
        el.set_attr("tooltip", t);
    }
    if let Some(d) = op.display.as_deref() {
        el.set_attr("display", d);
    }
    let parent = ensure_hyperlinks(worksheet);
    parent.push_element(el);
    Ok(())
}

pub(crate) fn apply_set(
    pkg: &mut Package,
    worksheet: &mut Element,
    sheet_part: &str,
    op: &SetHyperlinkOp,
    i: usize,
) -> Result<(), Error> {
    if let Some((_, start, end)) = &op.refer {
        if overlaps(worksheet, *start, *end, Some(op.index)) {
            return Err(Error::with_path(
                ErrorKind::HyperlinkOverlap,
                format!("ops[{i}].ref"),
            ));
        }
    }
    let old_rid = {
        let el = nth(worksheet, op.index, i, &op.id)?;
        preview_target(el, op, i)?;
        el.attr_local("id").map(str::to_string)
    };
    if matches!(&op.url, Patch::Set(_) | Patch::Clear) {
        picture::ensure_xmlns(worksheet, "r", REL_NS);
    }
    let new_rid = match &op.url {
        Patch::Absent => None,
        Patch::Clear => {
            let el = nth_mut(worksheet, op.index, i, &op.id)?;
            remove_attr_local(el, "id");
            None
        }
        Patch::Set(url) => {
            let mut rels = Relationships::from_package(pkg, sheet_part)?;
            let rid = set_or_create_url(&mut rels, old_rid.as_deref(), url);
            rels.write_to(pkg, sheet_part)?;
            let el = nth_mut(worksheet, op.index, i, &op.id)?;
            remove_attr_local(el, "id");
            el.set_attr("r:id", &rid);
            Some(rid)
        }
    };
    {
        let el = nth_mut(worksheet, op.index, i, &op.id)?;
        match &op.location {
            Patch::Absent => {}
            Patch::Clear => {
                el.remove_attr("location");
            }
            Patch::Set(v) => el.set_attr("location", v),
        }
        match &op.tooltip {
            Patch::Absent => {}
            Patch::Clear => {
                el.remove_attr("tooltip");
            }
            Patch::Set(v) => el.set_attr("tooltip", v),
        }
        match &op.display {
            Patch::Absent => {}
            Patch::Clear => {
                el.remove_attr("display");
            }
            Patch::Set(v) => el.set_attr("display", v),
        }
        if let Some((refer, _, _)) = &op.refer {
            el.set_attr("ref", refer);
        }
    }
    if let Some(old) = old_rid.as_deref() {
        let keep = new_rid.as_deref() == Some(old);
        if !keep {
            drop_hyperlink_rel_if_unused(pkg, sheet_part, worksheet, old)?;
        }
    }
    Ok(())
}

pub(crate) fn apply_delete(
    pkg: &mut Package,
    worksheet: &mut Element,
    sheet_part: &str,
    op: &DeleteHyperlinkOp,
    i: usize,
) -> Result<(), Error> {
    let Some(pos) = worksheet
        .children
        .iter()
        .position(|n| matches!(n, Node::Element(e) if e.local_name() == "hyperlinks"))
    else {
        return Err(not_found(&op.id, i));
    };
    let rid = {
        let Node::Element(parent) = &worksheet.children[pos] else {
            return Err(not_found(&op.id, i));
        };
        let Some(el) = super::list_pin::find_child(parent, "hyperlink", "link", op.index) else {
            return Err(not_found(&op.id, i));
        };
        el.attr_local("id").map(str::to_string)
    };
    let Node::Element(parent) = &mut worksheet.children[pos] else {
        return Err(not_found(&op.id, i));
    };
    let attr = crate::xml::pin_attr("link");
    let mut live = 0usize;
    let mut removed = false;
    parent.children.retain(|n| match n {
        Node::Element(e) if e.local_name() == "hyperlink" => {
            live += 1;
            let drop = crate::xml::pin_matches(e, &attr, op.index, live);
            if drop {
                removed = true;
            }
            !drop
        }
        _ => true,
    });
    if !removed {
        return Err(not_found(&op.id, i));
    }
    if parent.children_by_local("hyperlink").is_empty() {
        worksheet.children.remove(pos);
    }
    if let Some(rid) = rid.as_deref() {
        drop_hyperlink_rel_if_unused(pkg, sheet_part, worksheet, rid)?;
    }
    Ok(())
}

fn preview_target(el: &Element, op: &SetHyperlinkOp, i: usize) -> Result<(), Error> {
    let mut has_rid = el.attr_local("id").is_some();
    let mut has_loc = el.attr("location").is_some();
    match &op.url {
        Patch::Absent => {}
        Patch::Clear => has_rid = false,
        Patch::Set(_) => has_rid = true,
    }
    match &op.location {
        Patch::Absent => {}
        Patch::Clear => has_loc = false,
        Patch::Set(_) => has_loc = true,
    }
    if !has_rid && !has_loc {
        return Err(Error::with_path(
            ErrorKind::XlsxHyperlinkNeedTarget,
            format!("ops[{i}]"),
        ));
    }
    Ok(())
}

fn overlaps(worksheet: &Element, start: CellRef, end: CellRef, skip: Option<usize>) -> bool {
    let Some(parent) = worksheet.child_by_local("hyperlinks") else {
        return false;
    };
    for (i, el) in parent
        .children_by_local("hyperlink")
        .into_iter()
        .enumerate()
    {
        if skip == Some(i + 1) {
            continue;
        }
        let Some(raw) = el.attr("ref") else {
            continue;
        };
        let Ok((a, b)) = a1::parse_range(raw) else {
            continue;
        };
        if ranges_intersect(start, end, a, b) {
            return true;
        }
    }
    false
}

fn ranges_intersect(a1: CellRef, b1: CellRef, a2: CellRef, b2: CellRef) -> bool {
    !(b1.col < a2.col || b2.col < a1.col || b1.row < a2.row || b2.row < a1.row)
}

fn ensure_hyperlinks(worksheet: &mut Element) -> &mut Element {
    if worksheet.child_by_local("hyperlinks").is_none() {
        layout::insert_child_ordered(worksheet, Element::new("hyperlinks"), "hyperlinks");
    }
    worksheet.child_by_local_mut("hyperlinks").unwrap()
}

fn nth<'a>(worksheet: &'a Element, index: usize, i: usize, id: &str) -> Result<&'a Element, Error> {
    let parent = worksheet
        .child_by_local("hyperlinks")
        .ok_or_else(|| not_found(id, i))?;
    super::list_pin::find_child(parent, "hyperlink", "link", index).ok_or_else(|| not_found(id, i))
}

fn nth_mut<'a>(
    worksheet: &'a mut Element,
    index: usize,
    i: usize,
    id: &str,
) -> Result<&'a mut Element, Error> {
    let parent = worksheet
        .child_by_local_mut("hyperlinks")
        .ok_or_else(|| not_found(id, i))?;
    super::list_pin::find_child_mut(parent, "hyperlink", "link", index)
        .ok_or_else(|| not_found(id, i))
}

fn not_found(id: &str, i: usize) -> Error {
    Error::with_path(
        ErrorKind::HyperlinkNotFound { id: id.to_string() },
        format!("ops[{i}].id"),
    )
}

fn push_hyperlink_rel(pkg: &mut Package, sheet_part: &str, url: &str) -> Result<String, Error> {
    let mut rels = Relationships::from_package(pkg, sheet_part)?;
    let id = rels.next_rid();
    rels.push(Relationship {
        id: id.clone(),
        rel_type: REL_HYPERLINK.to_string(),
        target: url.to_string(),
        target_mode: Some("External".into()),
        extra: Vec::new(),
    });
    rels.write_to(pkg, sheet_part)?;
    Ok(id)
}

fn set_or_create_url(rels: &mut Relationships, existing_rid: Option<&str>, url: &str) -> String {
    if let Some(rid) = existing_rid {
        if let Some(rel) = rels.rels.iter_mut().find(|r| r.id == rid) {
            if rel.rel_type.ends_with("/hyperlink") {
                rel.target = url.to_string();
                rel.target_mode = Some("External".into());
                return rid.to_string();
            }
        }
    }
    let id = rels.next_rid();
    rels.push(Relationship {
        id: id.clone(),
        rel_type: REL_HYPERLINK.to_string(),
        target: url.to_string(),
        target_mode: Some("External".into()),
        extra: Vec::new(),
    });
    id
}

fn drop_hyperlink_rel_if_unused(
    pkg: &mut Package,
    sheet_part: &str,
    worksheet: &Element,
    rid: &str,
) -> Result<(), Error> {
    if rid_used(worksheet, rid) {
        return Ok(());
    }
    let mut rels = Relationships::from_package(pkg, sheet_part)?;
    let is_hl = rels
        .find_by_id(rid)
        .is_some_and(|r| r.rel_type.ends_with("/hyperlink"));
    if is_hl {
        rels.remove_by_id(rid);
        rels.write_to(pkg, sheet_part)?;
    }
    Ok(())
}

fn rid_used(worksheet: &Element, rid: &str) -> bool {
    let Some(parent) = worksheet.child_by_local("hyperlinks") else {
        return false;
    };
    parent
        .children_by_local("hyperlink")
        .iter()
        .any(|el| el.attr_local("id") == Some(rid))
}

fn remove_attr_local(el: &mut Element, local: &str) {
    el.attrs
        .retain(|(k, _)| k.rsplit(':').next().unwrap_or(k) != local);
}

pub(crate) fn inspect_cnv(el: &Element, rels: Option<&Relationships>) -> Value {
    let Some(cnv) = find_cnv_pr(el) else {
        return Value::Null;
    };
    let Some(click) = cnv.child_by_local("hlinkClick") else {
        return Value::Null;
    };
    let rid = click.attr_local("id");
    let rel = rid.and_then(|id| rels.and_then(|r| r.find_by_id(id)));
    let mut url = Value::Null;
    let mut location = Value::Null;
    let mut kind = "url";
    if let Some(rel) = rel {
        if rel.target.starts_with('#') {
            kind = "location";
            location = json!(rel.target.trim_start_matches('#').to_string());
        } else {
            url = json!(rel.target.clone());
        }
    }
    json!({
        "kind": kind,
        "url": url,
        "location": location,
        "tooltip": click.attr("tooltip").map(|s| json!(s)).unwrap_or(Value::Null),
    })
}

fn find_cnv_pr(el: &Element) -> Option<&Element> {
    crate::xml::find_by_local(el, "cNvPr")
}

fn find_cnv_pr_mut(el: &mut Element) -> Option<&mut Element> {
    crate::xml::find_by_local_mut(el, "cNvPr")
}

pub(crate) fn apply_insert_drawing(
    pkg: &mut Package,
    sheet_part: &str,
    host: &DrawingHlinkHost,
    url: Option<&str>,
    location: Option<&str>,
    tooltip: Option<&str>,
    i: usize,
) -> Result<(), Error> {
    let (drawing_part, hf_el, mut doc) = load_host_drawing(pkg, sheet_part, host, i)?;
    picture::ensure_xmlns(&mut doc.root, "r", REL_NS);
    picture::ensure_xmlns(&mut doc.root, "a", picture::A_NS);
    let obj = host_element_mut(&mut doc.root, host, hf_el.as_ref(), i)?;
    let cnv = find_cnv_pr_mut(obj).ok_or_else(|| {
        Error::with_path(
            ErrorKind::HyperlinkNotFound { id: host_id(host) },
            format!("ops[{i}].id"),
        )
    })?;
    if cnv.child_by_local("hlinkClick").is_some() {
        return Err(Error::with_path(
            ErrorKind::ShapeAlreadyHasHyperlink,
            format!("ops[{i}].id"),
        ));
    }
    let rid = push_drawing_hlink_rel(pkg, &drawing_part, url, location)?;
    insert_hlink_click(cnv, &rid, tooltip);
    pkg.set(&drawing_part, doc.to_bytes()?)?;
    Ok(())
}

pub(crate) fn apply_set_drawing(
    pkg: &mut Package,
    sheet_part: &str,
    host: &DrawingHlinkHost,
    op: &SetHyperlinkOp,
    i: usize,
) -> Result<(), Error> {
    let (drawing_part, hf_el, mut doc) = load_host_drawing(pkg, sheet_part, host, i)?;
    let obj = host_element_mut(&mut doc.root, host, hf_el.as_ref(), i)?;
    let cnv = find_cnv_pr_mut(obj).ok_or_else(|| {
        Error::with_path(
            ErrorKind::HyperlinkNotFound { id: op.id.clone() },
            format!("ops[{i}].id"),
        )
    })?;
    let click = cnv.child_by_local_mut("hlinkClick").ok_or_else(|| {
        Error::with_path(
            ErrorKind::HyperlinkNotFound { id: op.id.clone() },
            format!("ops[{i}].id"),
        )
    })?;
    let old_rid = click.attr_local("id").map(str::to_string);
    match &op.tooltip {
        Patch::Absent => {}
        Patch::Clear => {
            click.remove_attr("tooltip");
        }
        Patch::Set(v) => click.set_attr("tooltip", v),
    }
    let new_rid = match (&op.url, &op.location) {
        (Patch::Absent, Patch::Absent) => None,
        (Patch::Clear, Patch::Absent)
        | (Patch::Absent, Patch::Clear)
        | (Patch::Clear, Patch::Clear) => {
            return Err(Error::with_path(
                ErrorKind::XlsxHyperlinkNeedTarget,
                format!("ops[{i}]"),
            ));
        }
        (Patch::Set(url), Patch::Absent) | (Patch::Set(url), Patch::Clear) => Some(
            set_drawing_hlink_rel(pkg, &drawing_part, old_rid.as_deref(), Some(url), None)?,
        ),
        (Patch::Absent, Patch::Set(loc)) | (Patch::Clear, Patch::Set(loc)) => Some(
            set_drawing_hlink_rel(pkg, &drawing_part, old_rid.as_deref(), None, Some(loc))?,
        ),
        (Patch::Set(_), Patch::Set(_)) => {
            return Err(Error::with_path(
                ErrorKind::MutuallyExclusive {
                    a: "url".into(),
                    b: "location".into(),
                },
                format!("ops[{i}]"),
            ));
        }
    };
    if let Some(rid) = new_rid.as_deref() {
        remove_attr_local(click, "id");
        click.set_attr("r:id", rid);
    }
    pkg.set(&drawing_part, doc.to_bytes()?)?;
    if let Some(old) = old_rid.as_deref() {
        if new_rid.as_deref() != Some(old) {
            drop_drawing_hlink_rel_if_unused(pkg, &drawing_part, &doc.root, old)?;
        }
    }
    Ok(())
}

pub(crate) fn apply_delete_drawing(
    pkg: &mut Package,
    sheet_part: &str,
    host: &DrawingHlinkHost,
    id: &str,
    i: usize,
) -> Result<(), Error> {
    let (drawing_part, hf_el, mut doc) = load_host_drawing(pkg, sheet_part, host, i)?;
    let obj = host_element_mut(&mut doc.root, host, hf_el.as_ref(), i)?;
    let cnv = find_cnv_pr_mut(obj).ok_or_else(|| {
        Error::with_path(
            ErrorKind::HyperlinkNotFound { id: id.to_string() },
            format!("ops[{i}].id"),
        )
    })?;
    let rid = cnv
        .child_by_local("hlinkClick")
        .and_then(|e| e.attr_local("id").map(str::to_string));
    let before = cnv.children.len();
    cnv.children.retain(|n| match n {
        Node::Element(e) => e.local_name() != "hlinkClick",
        _ => true,
    });
    if cnv.children.len() == before {
        return Err(Error::with_path(
            ErrorKind::HyperlinkNotFound { id: id.to_string() },
            format!("ops[{i}].id"),
        ));
    }
    pkg.set(&drawing_part, doc.to_bytes()?)?;
    if let Some(rid) = rid.as_deref() {
        drop_drawing_hlink_rel_if_unused(pkg, &drawing_part, &doc.root, rid)?;
    }
    Ok(())
}

fn load_host_drawing(
    pkg: &Package,
    sheet_part: &str,
    host: &DrawingHlinkHost,
    i: usize,
) -> Result<(String, Option<Element>, crate::xml::Document), Error> {
    let sheet = xml::parse(pkg.get(sheet_part)?)?;
    let hf_el = sheet.root.child_by_local("drawingHF").cloned();
    let part = match host {
        DrawingHlinkHost::Picture(target) => picture::find_drawing_for(
            pkg,
            sheet_part,
            target,
            &host_id(host),
            &format!("ops[{i}].id"),
        )?,
        DrawingHlinkHost::TextBox(d) => shape::find_drawing_for(
            pkg,
            sheet_part,
            &sheet.root,
            shape::DrawObjKind::TextBox.matcher(),
            &d.key,
        )
        .ok_or_else(|| {
            Error::with_path(
                ErrorKind::TextBoxNotFound { id: host_id(host) },
                format!("ops[{i}].id"),
            )
        })?,
        DrawingHlinkHost::Shape(d) => shape::find_drawing_for(
            pkg,
            sheet_part,
            &sheet.root,
            shape::DrawObjKind::Shape.matcher(),
            &d.key,
        )
        .ok_or_else(|| {
            Error::with_path(
                ErrorKind::ShapeNotFound { id: host_id(host) },
                format!("ops[{i}].id"),
            )
        })?,
        DrawingHlinkHost::SmartArt(d) => shape::find_drawing_for(
            pkg,
            sheet_part,
            &sheet.root,
            smartart::is_diagram_frame,
            &d.key,
        )
        .ok_or_else(|| {
            Error::with_path(
                ErrorKind::SmartArtNotFound { id: host_id(host) },
                format!("ops[{i}].id"),
            )
        })?,
    };
    let doc = xml::parse(pkg.get(&part)?)?;
    Ok((part, hf_el, doc))
}

fn host_element_mut<'a>(
    root: &'a mut Element,
    host: &DrawingHlinkHost,
    hf_el: Option<&Element>,
    i: usize,
) -> Result<&'a mut Element, Error> {
    match host {
        DrawingHlinkHost::Picture(target) => picture::find_pic_element_mut(root, target, hf_el)
            .ok_or_else(|| {
                Error::with_path(
                    ErrorKind::PictureNotFound { id: host_id(host) },
                    format!("ops[{i}].id"),
                )
            }),
        DrawingHlinkHost::TextBox(d) => {
            shape::find_object_mut(root, shape::DrawObjKind::TextBox.matcher(), &d.key).ok_or_else(
                || {
                    Error::with_path(
                        ErrorKind::TextBoxNotFound { id: host_id(host) },
                        format!("ops[{i}].id"),
                    )
                },
            )
        }
        DrawingHlinkHost::Shape(d) => {
            shape::find_object_mut(root, shape::DrawObjKind::Shape.matcher(), &d.key).ok_or_else(
                || {
                    Error::with_path(
                        ErrorKind::ShapeNotFound { id: host_id(host) },
                        format!("ops[{i}].id"),
                    )
                },
            )
        }
        DrawingHlinkHost::SmartArt(d) => {
            shape::find_object_mut(root, smartart::is_diagram_frame, &d.key).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::SmartArtNotFound { id: host_id(host) },
                    format!("ops[{i}].id"),
                )
            })
        }
    }
}

fn host_id(host: &DrawingHlinkHost) -> String {
    match host {
        DrawingHlinkHost::Picture(p) => format!("sheet:{}/pic", p.sheet),
        DrawingHlinkHost::TextBox(d) => format!("sheet:{}/sp", d.sheet),
        DrawingHlinkHost::Shape(d) => format!("sheet:{}/shape", d.sheet),
        DrawingHlinkHost::SmartArt(d) => format!("sheet:{}/smartdiagram", d.sheet),
    }
}

fn insert_hlink_click(cnv: &mut Element, rid: &str, tooltip: Option<&str>) {
    let mut el = Element::new("a:hlinkClick").with_attr("r:id", rid);
    if let Some(t) = tooltip {
        el.set_attr("tooltip", t);
    }
    cnv.children.insert(0, Node::Element(el));
}

fn push_drawing_hlink_rel(
    pkg: &mut Package,
    drawing_part: &str,
    url: Option<&str>,
    location: Option<&str>,
) -> Result<String, Error> {
    let mut rels = Relationships::from_package(pkg, drawing_part)?;
    let id = rels.next_rid();
    let (target, mode) = if let Some(url) = url {
        (url.to_string(), Some("External".into()))
    } else {
        let loc = location.unwrap_or("");
        let target = if loc.starts_with('#') {
            loc.to_string()
        } else {
            format!("#{loc}")
        };
        (target, None)
    };
    rels.push(Relationship {
        id: id.clone(),
        rel_type: super::REL_HYPERLINK.to_string(),
        target,
        target_mode: mode,
        extra: Vec::new(),
    });
    rels.write_to(pkg, drawing_part)?;
    Ok(id)
}

fn set_drawing_hlink_rel(
    pkg: &mut Package,
    drawing_part: &str,
    old_rid: Option<&str>,
    url: Option<&str>,
    location: Option<&str>,
) -> Result<String, Error> {
    let mut rels = Relationships::from_package(pkg, drawing_part)?;
    let (target, mode) = if let Some(url) = url {
        (url.to_string(), Some("External".into()))
    } else {
        let loc = location.unwrap_or("");
        let target = if loc.starts_with('#') {
            loc.to_string()
        } else {
            format!("#{loc}")
        };
        (target, None)
    };
    if let Some(rid) = old_rid {
        if let Some(rel) = rels.rels.iter_mut().find(|r| r.id == rid) {
            if rel.rel_type.ends_with("/hyperlink") {
                rel.target = target;
                rel.target_mode = mode;
                rels.write_to(pkg, drawing_part)?;
                return Ok(rid.to_string());
            }
        }
    }
    let id = rels.next_rid();
    rels.push(Relationship {
        id: id.clone(),
        rel_type: super::REL_HYPERLINK.to_string(),
        target,
        target_mode: mode,
        extra: Vec::new(),
    });
    rels.write_to(pkg, drawing_part)?;
    Ok(id)
}

fn drop_drawing_hlink_rel_if_unused(
    pkg: &mut Package,
    drawing_part: &str,
    root: &Element,
    rid: &str,
) -> Result<(), Error> {
    let mut used = false;
    picture::walk_all(root, &mut |e| {
        if e.local_name() == "hlinkClick" && e.attr_local("id") == Some(rid) {
            used = true;
        }
    });
    if used {
        return Ok(());
    }
    let mut rels = Relationships::from_package(pkg, drawing_part)?;
    rels.remove_by_id(rid);
    if rels.rels.is_empty() {
        pkg.remove(&crate::opc::rels_part_name(drawing_part));
    } else {
        rels.write_to(pkg, drawing_part)?;
    }
    Ok(())
}
