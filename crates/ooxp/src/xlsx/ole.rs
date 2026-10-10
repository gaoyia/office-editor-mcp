//! 工作表嵌入对象：`<oleObjects>`。不执行 OLE 服务器，不把嵌入字节塞进 inspect 树。

use std::collections::HashSet;
use std::path::PathBuf;

use serde_json::{json, Map, Value};

use crate::error::{io_path, Error, ErrorKind, IdReason};
use crate::ole::{self, FileSpec};
use crate::opc::{resolve_id, ContentTypes, Package, Relationship, Relationships};
use crate::xml::{self, Document, Element, Node};

use super::a1;
use super::layout;
use super::picture::{self, AnchorKind};
use super::REL_NS;

const MC_NS: &str = "http://schemas.openxmlformats.org/markup-compatibility/2006";
const X14_NS: &str = "http://schemas.microsoft.com/office/spreadsheetml/2009/9/main";
const A14_NS: &str = "http://schemas.microsoft.com/office/drawing/2010/main";
const REL_VML: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/vmlDrawing";
const CT_VML: &str = "application/vnd.openxmlformats-officedocument.vmlDrawing";
const COMPAT_URI: &str = "{63B3BB69-23CF-44E3-9099-C40C66FF867C}";

const DEFAULT_CX: i64 = 1_828_800;
const DEFAULT_CY: i64 = 914_400;

const SHAPE_LAYOUT: &str =
    r#"<o:shapelayout v:ext="edit"><o:idmap v:ext="edit" data="1"/></o:shapelayout>"#;
const T75: &str = r#"<v:shapetype id="_x0000_t75" coordsize="21600,21600" o:spt="75" o:preferrelative="t" path="m@4@5l@4@11@9@11@9@5xe" filled="f" stroked="f"><v:stroke joinstyle="miter"/><v:formulas><v:f eqn="if lineDrawn pixelLineWidth 0"/><v:f eqn="sum @0 1 0"/><v:f eqn="sum 0 0 @1"/><v:f eqn="prod @2 1 2"/><v:f eqn="prod @3 21600 pixelWidth"/><v:f eqn="prod @3 21600 pixelHeight"/><v:f eqn="sum @0 0 1"/><v:f eqn="prod @6 1 2"/><v:f eqn="prod @7 21600 pixelWidth"/><v:f eqn="sum @8 21600 0"/><v:f eqn="prod @7 21600 pixelHeight"/><v:f eqn="sum @10 21600 0"/></v:formulas><v:path o:extrusionok="f" gradientshapeok="t" o:connecttype="rect"/><o:lock v:ext="edit" aspectratio="t"/></v:shapetype>"#;

#[derive(Debug, Clone)]
pub enum OleKey {
    Id(String),
    Index(usize),
}

#[derive(Debug, Clone)]
pub struct OleTarget {
    pub sheet: String,
    pub key: OleKey,
}

#[derive(Debug, Clone)]
pub struct InsertOleOp {
    pub sheet: String,
    pub file: String,
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
pub struct DeleteOleOp {
    pub id: String,
    pub target: OleTarget,
}

pub fn parse_ole_id(id: &str, json_path: &str) -> Result<OleTarget, Error> {
    let rest = id.strip_prefix("sheet:").ok_or_else(|| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SheetOlePrefix,
            },
            json_path.to_string(),
        )
    })?;
    let Some((name, after)) = rest.split_once("/ole:") else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SheetOleContainer,
            },
            json_path.to_string(),
        ));
    };
    if name.is_empty() || after.is_empty() {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SheetOlePrefix,
            },
            json_path.to_string(),
        ));
    }
    let key = if let Some(rest) = after.strip_prefix("index:") {
        let k: usize = rest.parse().map_err(|_| {
            Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::OleIndexNotInt,
                },
                json_path.to_string(),
            )
        })?;
        if k < 1 {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::OleIndexFromOne,
                },
                json_path.to_string(),
            ));
        }
        OleKey::Index(k)
    } else {
        OleKey::Id(after.to_string())
    };
    Ok(OleTarget {
        sheet: name.to_string(),
        key,
    })
}

pub fn is_ole_id(id: &str) -> bool {
    id.contains("/ole:")
}

/// 该表全部 `oleObject/@shapeId`，含未收入树的条目。dummy `xdr:sp` 按此排除。
pub fn dummy_ids(sheet: &Element) -> HashSet<String> {
    let mut out = HashSet::new();
    for_each_ole_el(sheet, |obj| {
        if let Some(sid) = obj.attr("shapeId").filter(|s| !s.is_empty()) {
            out.insert(sid.to_string());
        }
    });
    out
}

pub fn count(pkg: &Package, sheet_part: &str, sheet: &Element) -> Result<usize, Error> {
    Ok(collect_objs(pkg, sheet_part, sheet)?.len())
}

pub fn collect(
    pkg: &Package,
    sheet_part: &str,
    sheet: &Element,
    sheet_name: &str,
) -> Result<Vec<Value>, Error> {
    let objs = collect_objs(pkg, sheet_part, sheet)?;
    let cts = ContentTypes::from_package(pkg)?;
    let dummy_names = dummy_names(pkg, sheet_part, sheet)?;
    let mut out = Vec::new();
    for (i, obj) in objs.iter().enumerate() {
        out.push(obj_json(
            pkg,
            sheet_part,
            sheet,
            &cts,
            obj,
            sheet_name,
            i + 1,
            &dummy_names,
        )?);
    }
    Ok(out)
}

pub fn export(bytes: &[u8], id: &str) -> Result<super::ExportedMedia, Error> {
    let target = parse_ole_id(id, "id")?;
    let pkg = Package::open_bytes(bytes.to_vec())?;
    let sheet_part = picture_lookup(&pkg, &target.sheet, "id")?;
    let sheet = xml::parse(pkg.get(&sheet_part)?)?;
    let obj = find_obj(&pkg, &sheet_part, &sheet.root, &target.key)?
        .ok_or_else(|| Error::with_path(ErrorKind::OleNotFound { id: id.to_string() }, "id"))?;
    let rid = rid_attr(obj).ok_or_else(|| {
        Error::with_path(
            ErrorKind::PartMissing {
                name: format!("{id} oleObject"),
            },
            "id",
        )
    })?;
    let part = resolve_id(&pkg, &sheet_part, rid)?;
    let embed_bytes = pkg.get(&part)?.to_vec();
    let cts = ContentTypes::from_package(&pkg)?;
    let content_type = cts.content_type_of(&part).map(str::to_string);
    Ok(super::ExportedMedia {
        bytes: embed_bytes,
        content_type,
    })
}

pub fn insert(
    pkg: &mut Package,
    sheet_doc: &mut Document,
    sheet_part: &str,
    op: &InsertOleOp,
    _file_field: &str,
) -> Result<(), Error> {
    let spec = ole::classify(&op.file);
    let src = PathBuf::from(&op.file);
    let bytes = std::fs::read(&src).map_err(|e| io_path(e, src.clone()))?;
    let payload = match &spec {
        FileSpec::Office { .. } => bytes,
        FileSpec::Package => ole::wrap_package(&op.file, &bytes),
    };
    let cx = op.cx.unwrap_or(DEFAULT_CX);
    let cy = op.cy.unwrap_or(DEFAULT_CY);
    let (to, to_off_x, to_off_y) = object_pr_to(&sheet_doc.root, op, cx, cy);
    let existing = counted_len(pkg, sheet_part, &sheet_doc.root)?;
    let vml_now = read_vml(pkg, sheet_part)?;
    let sid = next_shape_id(&sheet_doc.root, vml_now.as_deref());
    let name = format!("Object {}", existing + 1);

    let embed_part = ole::next_numbered(pkg, "/xl/embeddings", spec.stem(), spec.ext());
    pkg.set(&embed_part, payload)?;
    let png_part = picture::next_media_part(pkg, "png");
    pkg.set(&png_part, ole::PREVIEW_PNG.to_vec())?;

    let ole_rid = add_rel(pkg, sheet_part, spec.rel_type(), &embed_part)?;
    let img_rid = picture::add_image_rel(pkg, sheet_part, &png_part)?;

    let mut cts = ContentTypes::from_package(pkg)?;
    cts.ensure_default("png", "image/png");
    match &spec {
        FileSpec::Office {
            ext, content_type, ..
        } => {
            cts.ensure_default(ext, content_type);
            cts.set_override(&embed_part, content_type)?;
        }
        FileSpec::Package => {
            cts.ensure_default("bin", ole::CT_OLE);
            cts.set_override(&embed_part, ole::CT_OLE)?;
        }
    }
    cts.write_to(pkg)?;

    picture::ensure_xmlns(&mut sheet_doc.root, "r", REL_NS);
    picture::ensure_xmlns(&mut sheet_doc.root, "xdr", picture::XDR_NS);
    picture::ensure_xmlns(&mut sheet_doc.root, "mc", MC_NS);
    picture::ensure_xmlns(&mut sheet_doc.root, "x14", X14_NS);
    if sheet_doc.root.attr("mc:Ignorable").is_none() {
        sheet_doc.root.set_attr("mc:Ignorable", "x14");
    }

    let ac = build_ole_ac(
        &spec,
        sid,
        &ole_rid,
        &img_rid,
        op.from,
        op.from_off_x,
        op.from_off_y,
        to,
        to_off_x,
        to_off_y,
    );
    let parent = ensure_ole_objects(&mut sheet_doc.root);
    parent.push_element(ac);

    let drawing_part = picture::ensure_drawing_part(pkg, sheet_doc, sheet_part)?;
    let mut draw_doc = xml::parse(pkg.get(&drawing_part)?)?;
    picture::ensure_xmlns(&mut draw_doc.root, "r", REL_NS);
    picture::ensure_xmlns(&mut draw_doc.root, "a", picture::A_NS);
    draw_doc.root.push_element(build_dummy_anchor(
        sid,
        &name,
        op.from,
        op.from_off_x,
        op.from_off_y,
        to,
        to_off_x,
        to_off_y,
    ));
    pkg.set(&drawing_part, draw_doc.to_bytes()?)?;

    let vml_part = ensure_vml_part(pkg, sheet_part, &mut sheet_doc.root)?;
    let vml_img = picture::add_image_rel(pkg, &vml_part, &png_part)?;
    append_vml_pict(pkg, &vml_part, sid, &vml_img, op.from, to, cx, cy)?;
    Ok(())
}

pub fn delete(
    pkg: &mut Package,
    sheet_doc: &mut Document,
    sheet_part: &str,
    op: &DeleteOleOp,
    op_index: usize,
) -> Result<(), Error> {
    let path = format!("ops[{op_index}].id");
    let objs = collect_objs(pkg, sheet_part, &sheet_doc.root)?;
    let hit = objs
        .iter()
        .enumerate()
        .find(|(i, e)| key_matches(&op.target.key, e, *i + 1));
    let Some((_, obj)) = hit else {
        return Err(Error::with_path(
            ErrorKind::OleNotFound { id: op.id.clone() },
            path,
        ));
    };
    let sid = obj.attr("shapeId").unwrap_or("").to_string();
    let mut rids = Vec::new();
    collect_rids(obj, &mut rids);
    if let Some(pr) = obj.child_by_local("objectPr") {
        collect_rids(pr, &mut rids);
    }

    if !remove_ole_entry(&mut sheet_doc.root, &op.target.key) {
        return Err(Error::with_path(
            ErrorKind::OleNotFound { id: op.id.clone() },
            path,
        ));
    }
    if let Some(parent) = sheet_doc.root.child_by_local("oleObjects") {
        if parent.element_children().next().is_none() {
            sheet_doc
                .root
                .children
                .retain(|n| !matches!(n, Node::Element(e) if e.local_name() == "oleObjects"));
        }
    }

    if let Some(drawing_part) = picture::drawing_parts_from(pkg, sheet_part, &sheet_doc.root)?
        .into_iter()
        .next()
    {
        let mut draw = xml::parse(pkg.get(&drawing_part)?)?;
        if !sid.is_empty() {
            remove_dummy_anchor(&mut draw.root, &sid);
        }
        if picture::drawing_is_empty(&draw.root) {
            pkg.remove(&drawing_part);
            pkg.remove(&crate::opc::rels_part_name(&drawing_part));
            picture::remove_drawing_ref(&mut sheet_doc.root, sheet_part, pkg, &drawing_part)?;
            let mut cts = ContentTypes::from_package(pkg)?;
            cts.remove_override(&drawing_part);
            cts.write_to(pkg)?;
        } else {
            pkg.set(&drawing_part, draw.to_bytes()?)?;
        }
    }

    if !sid.is_empty() {
        remove_vml_shape(pkg, sheet_part, &sid)?;
    }

    drop_unused_sheet_rels(pkg, sheet_part, &sheet_doc.root, &rids)?;
    if let Some(vml) = vml_part(pkg, sheet_part)? {
        drop_unused_rels(pkg, &vml, &rids)?;
        cleanup_vml_media(pkg, &vml)?;
    }
    drop_vml_if_empty(pkg, sheet_part, &mut sheet_doc.root)?;
    Ok(())
}

fn picture_lookup(pkg: &Package, sheet: &str, path: &str) -> Result<String, Error> {
    let wb_part = crate::opc::office_document_part(pkg)?;
    let wb = xml::parse(pkg.get(&wb_part)?)?;
    let Some(sheets) = wb.root.child_by_local("sheets") else {
        return Err(Error::with_path(
            ErrorKind::SheetNotFound {
                name: sheet.to_string(),
            },
            path,
        ));
    };
    for sh in sheets.children_by_local("sheet") {
        let name = sh.attr("name").unwrap_or("");
        if !name.eq_ignore_ascii_case(sheet) {
            continue;
        }
        let Some(rid) = rid_attr(sh) else {
            continue;
        };
        return resolve_id(pkg, &wb_part, rid);
    }
    Err(Error::with_path(
        ErrorKind::SheetNotFound {
            name: sheet.to_string(),
        },
        path,
    ))
}

fn collect_objs<'a>(
    pkg: &Package,
    sheet_part: &str,
    sheet: &'a Element,
) -> Result<Vec<&'a Element>, Error> {
    let rels = Relationships::from_package(pkg, sheet_part)?;
    let mut out = Vec::new();
    for_each_ole_el(sheet, |obj| {
        if counted(obj, &rels) {
            out.push(obj);
        }
    });
    Ok(out)
}

fn counted_len(pkg: &Package, sheet_part: &str, sheet: &Element) -> Result<usize, Error> {
    Ok(collect_objs(pkg, sheet_part, sheet)?.len())
}

fn counted(obj: &Element, rels: &Relationships) -> bool {
    if obj.attr("link").is_some() {
        return false;
    }
    let Some(rid) = rid_attr(obj) else {
        return false;
    };
    let Some(rel) = rels.rels.iter().find(|r| r.id == rid) else {
        return false;
    };
    if rel.is_external() {
        return false;
    }
    rel.rel_type.ends_with("/oleObject") || rel.rel_type.ends_with("/package")
}

fn for_each_ole_el<'a>(sheet: &'a Element, mut f: impl FnMut(&'a Element)) {
    let Some(parent) = sheet.child_by_local("oleObjects") else {
        return;
    };
    for child in parent.element_children() {
        if child.local_name() == "AlternateContent" {
            if let Some(choice) = child.child_by_local("Choice") {
                if let Some(obj) = find_ole_el(choice) {
                    f(obj);
                    continue;
                }
            }
            if let Some(fb) = child.child_by_local("Fallback") {
                if let Some(obj) = find_ole_el(fb) {
                    f(obj);
                }
            }
        } else if child.local_name() == "oleObject" {
            f(child);
        }
    }
}

fn find_ole_el(el: &Element) -> Option<&Element> {
    if el.local_name() == "oleObject" {
        return Some(el);
    }
    el.element_children()
        .find(|c| c.local_name() == "oleObject")
}

fn find_obj<'a>(
    pkg: &Package,
    sheet_part: &str,
    sheet: &'a Element,
    key: &OleKey,
) -> Result<Option<&'a Element>, Error> {
    let objs = collect_objs(pkg, sheet_part, sheet)?;
    Ok(objs
        .into_iter()
        .enumerate()
        .find(|(i, e)| key_matches(key, e, *i + 1))
        .map(|(_, e)| e))
}

fn key_matches(key: &OleKey, obj: &Element, index: usize) -> bool {
    match key {
        OleKey::Index(n) => index == *n,
        OleKey::Id(want) => obj.attr("shapeId") == Some(want.as_str()),
    }
}

fn obj_json(
    pkg: &Package,
    sheet_part: &str,
    sheet: &Element,
    cts: &ContentTypes,
    obj: &Element,
    sheet_name: &str,
    index: usize,
    dummy_names: &[(String, String)],
) -> Result<Value, Error> {
    let sid = obj.attr("shapeId").filter(|s| !s.is_empty());
    let (id, stable) = match sid {
        Some(n) => (format!("sheet:{sheet_name}/ole:{n}"), true),
        None => (format!("sheet:{sheet_name}/ole:index:{index}"), false),
    };
    let name = sid
        .and_then(|s| {
            dummy_names
                .iter()
                .find(|(k, _)| k == s)
                .map(|(_, n)| n.clone())
        })
        .unwrap_or_default();
    let prog_id = obj.attr("progId");
    let kind = match rid_attr(obj).and_then(|rid| {
        Relationships::from_package(pkg, sheet_part)
            .ok()
            .and_then(|rels| rels.rels.into_iter().find(|r| r.id == rid))
    }) {
        Some(r) if r.rel_type.ends_with("/package") => "package",
        _ => "embed",
    };
    let content_type = match rid_attr(obj).and_then(|rid| resolve_id(pkg, sheet_part, rid).ok()) {
        Some(part) => cts.content_type_of(&part).map(str::to_string),
        None => None,
    };
    let show_as_icon = obj.attr("dvAspect") == Some("DVASPECT_ICON");
    let mut m = Map::new();
    m.insert("id".into(), json!(id));
    m.insert("shapeId".into(), json!(sid));
    m.insert("name".into(), json!(name));
    m.insert("progId".into(), json!(prog_id));
    m.insert("kind".into(), json!(kind));
    m.insert("contentType".into(), json!(content_type));
    m.insert("showAsIcon".into(), json!(show_as_icon));
    fill_geom(&mut m, pkg, sheet_part, sheet, obj, sid)?;
    m.insert("stable".into(), json!(stable));
    Ok(Value::Object(m))
}

fn fill_geom(
    m: &mut Map<String, Value>,
    pkg: &Package,
    sheet_part: &str,
    sheet: &Element,
    obj: &Element,
    sid: Option<&str>,
) -> Result<(), Error> {
    if let Some(pr) = obj.child_by_local("objectPr") {
        if let Some(anchor) = pr.child_by_local("anchor") {
            fill_two_cell(m, anchor);
            return Ok(());
        }
    }
    if let Some(sid) = sid {
        if let Some((kind, el)) = find_dummy_anchor(pkg, sheet_part, sheet, sid)? {
            match kind {
                AnchorKind::TwoCell => fill_two_cell(m, &el),
                AnchorKind::OneCell => fill_one_cell(m, &el),
                AnchorKind::Absolute => fill_abs(m, &el),
                AnchorKind::RelSize | AnchorKind::AbsSize => {}
            }
            return Ok(());
        }
    }
    m.insert("anchor".into(), json!("twoCell"));
    m.insert("from".into(), json!("A1"));
    m.insert("fromOffX".into(), json!(0));
    m.insert("fromOffY".into(), json!(0));
    m.insert("to".into(), json!("A1"));
    m.insert("toOffX".into(), json!(0));
    m.insert("toOffY".into(), json!(0));
    m.insert("editAs".into(), json!("twoCell"));
    Ok(())
}

fn fill_two_cell(m: &mut Map<String, Value>, anchor: &Element) {
    let (from, fox, foy) = picture::read_marker(anchor.child_by_local("from"));
    let (to, tox, toy) = picture::read_marker(anchor.child_by_local("to"));
    m.insert("anchor".into(), json!("twoCell"));
    m.insert(
        "from".into(),
        json!(from.map(|c| a1::format_cell(c.col, c.row))),
    );
    m.insert("fromOffX".into(), json!(fox));
    m.insert("fromOffY".into(), json!(foy));
    m.insert(
        "to".into(),
        json!(to.map(|c| a1::format_cell(c.col, c.row))),
    );
    m.insert("toOffX".into(), json!(tox));
    m.insert("toOffY".into(), json!(toy));
    m.insert("editAs".into(), json!("twoCell"));
}

fn fill_one_cell(m: &mut Map<String, Value>, anchor: &Element) {
    let (from, fox, foy) = picture::read_marker(anchor.child_by_local("from"));
    m.insert("anchor".into(), json!("oneCell"));
    m.insert(
        "from".into(),
        json!(from.map(|c| a1::format_cell(c.col, c.row))),
    );
    m.insert("fromOffX".into(), json!(fox));
    m.insert("fromOffY".into(), json!(foy));
    let (cx, cy) = read_ext(anchor);
    m.insert("cx".into(), json!(cx));
    m.insert("cy".into(), json!(cy));
}

fn fill_abs(m: &mut Map<String, Value>, anchor: &Element) {
    m.insert("anchor".into(), json!("absolute"));
    let (x, y) = read_pos(anchor);
    let (cx, cy) = read_ext(anchor);
    m.insert("posX".into(), json!(x));
    m.insert("posY".into(), json!(y));
    m.insert("cx".into(), json!(cx));
    m.insert("cy".into(), json!(cy));
}

fn read_ext(anchor: &Element) -> (Option<i64>, Option<i64>) {
    let Some(ext) = anchor.child_by_local("ext") else {
        return (None, None);
    };
    (
        ext.attr("cx").and_then(|s| s.parse().ok()),
        ext.attr("cy").and_then(|s| s.parse().ok()),
    )
}

fn read_pos(anchor: &Element) -> (Option<i64>, Option<i64>) {
    let Some(pos) = anchor.child_by_local("pos") else {
        return (None, None);
    };
    (
        pos.attr("x").and_then(|s| s.parse().ok()),
        pos.attr("y").and_then(|s| s.parse().ok()),
    )
}

fn dummy_names(
    pkg: &Package,
    sheet_part: &str,
    sheet: &Element,
) -> Result<Vec<(String, String)>, Error> {
    let mut out = Vec::new();
    for part in picture::drawing_parts_from(pkg, sheet_part, sheet)? {
        let doc = xml::parse(pkg.get(&part)?)?;
        picture::walk_all(&doc.root, &mut |e| {
            if e.local_name() == "cNvPr" {
                if let Some(id) = e.attr_local("id") {
                    let name = e.attr_local("name").unwrap_or("").to_string();
                    out.push((id.to_string(), name));
                }
            }
        });
    }
    Ok(out)
}

fn find_dummy_anchor(
    pkg: &Package,
    sheet_part: &str,
    sheet: &Element,
    sid: &str,
) -> Result<Option<(AnchorKind, Element)>, Error> {
    for part in picture::drawing_parts_from(pkg, sheet_part, sheet)? {
        let doc = xml::parse(pkg.get(&part)?)?;
        let mut found = None;
        picture::walk_anchors(&doc.root, &mut |anchor, kind| {
            if found.is_some() {
                return Ok(());
            }
            walk_payloads(anchor, &mut |el| {
                if el.local_name() == "sp" {
                    if let Some(id) = cnv_id(el) {
                        if id == sid {
                            found = Some((kind, anchor.clone()));
                        }
                    }
                }
            });
            Ok(())
        })?;
        if let Some(v) = found {
            return Ok(Some(v));
        }
    }
    Ok(None)
}

fn object_pr_to(sheet: &Element, op: &InsertOleOp, cx: i64, cy: i64) -> (a1::CellRef, i64, i64) {
    if op.anchor == AnchorKind::TwoCell {
        return (op.to.unwrap_or(op.from), op.to_off_x, op.to_off_y);
    }
    let x =
        layout::col_origin_emu(sheet, op.from.col).saturating_add(op.from_off_x.saturating_add(cx));
    let y =
        layout::row_origin_emu(sheet, op.from.row).saturating_add(op.from_off_y.saturating_add(cy));
    let (cell, ox, oy) = layout::emu_to_marker(sheet, x, y);
    (cell, ox, oy)
}

fn next_shape_id(sheet: &Element, vml: Option<&str>) -> u32 {
    let mut max = 1024u32;
    for_each_ole_el(sheet, |obj| {
        if let Some(n) = obj.attr("shapeId").and_then(|s| s.parse::<u32>().ok()) {
            max = max.max(n);
        }
    });
    if let Some(vml) = vml {
        let mut from = 0usize;
        while let Some(i) = vml[from..].find("_x0000_s") {
            let abs = from + i + 8;
            let n: String = vml[abs..]
                .chars()
                .take_while(|c| c.is_ascii_digit())
                .collect();
            if let Ok(v) = n.parse::<u32>() {
                max = max.max(v);
            }
            from = abs;
        }
    }
    max + 1
}

fn build_ole_ac(
    spec: &FileSpec,
    sid: u32,
    ole_rid: &str,
    img_rid: &str,
    from: a1::CellRef,
    from_off_x: i64,
    from_off_y: i64,
    to: a1::CellRef,
    to_off_x: i64,
    to_off_y: i64,
) -> Element {
    let mut choice_obj = Element::new("oleObject")
        .with_attr("progId", spec.ole_obj_name())
        .with_attr("shapeId", sid.to_string())
        .with_attr("r:id", ole_rid);
    if spec.show_as_icon() {
        choice_obj.set_attr("dvAspect", "DVASPECT_ICON");
    }
    let mut pr = Element::new("objectPr")
        .with_attr("defaultSize", "0")
        .with_attr("r:id", img_rid);
    let mut anchor = Element::new("anchor").with_attr("moveWithCells", "1");
    // Excel 16.0 写无前缀 from/to，子元素仍带 xdr:。
    anchor.push_element(picture::marker_el("from", from, from_off_x, from_off_y));
    anchor.push_element(picture::marker_el("to", to, to_off_x, to_off_y));
    pr.push_element(anchor);
    choice_obj.push_element(pr);
    let mut fallback_obj = Element::new("oleObject")
        .with_attr("progId", spec.ole_obj_name())
        .with_attr("shapeId", sid.to_string())
        .with_attr("r:id", ole_rid);
    if spec.show_as_icon() {
        fallback_obj.set_attr("dvAspect", "DVASPECT_ICON");
    }
    let mut choice = Element::new("mc:Choice").with_attr("Requires", "x14");
    choice.push_element(choice_obj);
    let mut fallback = Element::new("mc:Fallback");
    fallback.push_element(fallback_obj);
    let mut ac = Element::new("mc:AlternateContent").with_attr("xmlns:mc", MC_NS);
    ac.push_element(choice);
    ac.push_element(fallback);
    ac
}

fn build_dummy_anchor(
    sid: u32,
    name: &str,
    from: a1::CellRef,
    from_off_x: i64,
    from_off_y: i64,
    to: a1::CellRef,
    to_off_x: i64,
    to_off_y: i64,
) -> Element {
    // Excel 16.0 把整段 twoCellAnchor 包在 Choice 里；锚点里塞 AlternateContent 会打不开。
    let mut cnv = Element::new("xdr:cNvPr")
        .with_attr("id", sid.to_string())
        .with_attr("name", name)
        .with_attr("hidden", "1");
    let mut ext = Element::new("a:ext").with_attr("uri", COMPAT_URI);
    ext.push_element(Element::new("a14:compatExt").with_attr("spid", format!("_x0000_s{sid}")));
    let mut lst = Element::new("a:extLst");
    lst.push_element(ext);
    cnv.push_element(lst);
    let mut nv = Element::new("xdr:nvSpPr");
    nv.push_element(cnv);
    nv.push_element(Element::new("xdr:cNvSpPr"));
    let mut xfrm = Element::new("a:xfrm");
    xfrm.push_element(
        Element::new("a:off")
            .with_attr("x", "0")
            .with_attr("y", "0"),
    );
    xfrm.push_element(
        Element::new("a:ext")
            .with_attr("cx", "0")
            .with_attr("cy", "0"),
    );
    let mut geom = Element::new("a:prstGeom").with_attr("prst", "rect");
    geom.push_element(Element::new("a:avLst"));
    let mut sp_pr = Element::new("xdr:spPr").with_attr("bwMode", "auto");
    sp_pr.push_element(xfrm);
    sp_pr.push_element(geom);
    let mut fill = Element::new("a:solidFill");
    fill.push_element(Element::new("a:srgbClr").with_attr("val", "FFFFFF"));
    sp_pr.push_element(fill);
    let mut sp = Element::new("xdr:sp")
        .with_attr("macro", "")
        .with_attr("textlink", "");
    sp.push_element(nv);
    sp.push_element(sp_pr);
    let mut anchor = Element::new("xdr:twoCellAnchor").with_attr("editAs", "oneCell");
    anchor.push_element(picture::marker_el("xdr:from", from, from_off_x, from_off_y));
    anchor.push_element(picture::marker_el("xdr:to", to, to_off_x, to_off_y));
    anchor.push_element(sp);
    anchor.push_element(Element::new("xdr:clientData"));
    let mut choice = Element::new("mc:Choice")
        .with_attr("xmlns:a14", A14_NS)
        .with_attr("Requires", "a14");
    choice.push_element(anchor);
    let fallback = Element::new("mc:Fallback");
    let mut ac = Element::new("mc:AlternateContent").with_attr("xmlns:mc", MC_NS);
    ac.push_element(choice);
    ac.push_element(fallback);
    ac
}

fn ensure_ole_objects(sheet: &mut Element) -> &mut Element {
    if sheet.child_by_local("oleObjects").is_none() {
        layout::insert_child_ordered(sheet, Element::new("oleObjects"), "oleObjects");
    }
    sheet
        .child_by_local_mut("oleObjects")
        .expect("刚插入 oleObjects")
}

fn remove_ole_entry(sheet: &mut Element, key: &OleKey) -> bool {
    let Some(parent) = sheet.child_by_local_mut("oleObjects") else {
        return false;
    };
    let mut idx = 0usize;
    let mut i = 0;
    while i < parent.children.len() {
        let is_hit = match &parent.children[i] {
            Node::Element(e) => {
                let obj = if e.local_name() == "AlternateContent" {
                    e.child_by_local("Choice")
                        .and_then(find_ole_el)
                        .or_else(|| e.child_by_local("Fallback").and_then(find_ole_el))
                } else if e.local_name() == "oleObject" {
                    Some(e)
                } else {
                    None
                };
                if let Some(obj) = obj {
                    idx += 1;
                    key_matches(key, obj, idx)
                } else {
                    false
                }
            }
            _ => false,
        };
        if is_hit {
            parent.children.remove(i);
            return true;
        }
        i += 1;
    }
    false
}

fn remove_dummy_anchor(root: &mut Element, sid: &str) -> bool {
    let mut i = 0;
    while i < root.children.len() {
        let hit = match &root.children[i] {
            Node::Element(e)
                if e.local_name() == "AlternateContent"
                    || AnchorKind::from_local(e.local_name()).is_some() =>
            {
                dummy_in_anchor(e, sid)
            }
            _ => false,
        };
        if hit {
            root.children.remove(i);
            return true;
        }
        if let Node::Element(e) = &mut root.children[i] {
            if remove_dummy_anchor(e, sid) {
                return true;
            }
        }
        i += 1;
    }
    false
}

fn dummy_in_anchor(anchor: &Element, sid: &str) -> bool {
    let mut hit = false;
    walk_payloads(anchor, &mut |el| {
        if el.local_name() == "sp" {
            if let Some(id) = cnv_id(el) {
                if id == sid {
                    hit = true;
                }
            }
        }
    });
    hit
}

fn cnv_id(el: &Element) -> Option<String> {
    el.child_by_local("nvSpPr")
        .and_then(|nv| nv.child_by_local("cNvPr"))
        .and_then(|pr| pr.attr_local("id"))
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn walk_payloads(el: &Element, f: &mut impl FnMut(&Element)) {
    if el.local_name() == "AlternateContent" {
        if let Some(c) = el
            .child_by_local("Choice")
            .or_else(|| el.child_by_local("Fallback"))
        {
            walk_payloads(c, f);
        }
        return;
    }
    f(el);
    for c in el.element_children() {
        if matches!(c.local_name(), "from" | "to" | "ext" | "pos" | "clientData") {
            continue;
        }
        walk_payloads(c, f);
    }
}

fn rid_attr(el: &Element) -> Option<&str> {
    el.attrs.iter().find_map(|(k, v)| {
        if k.contains(':') && k.rsplit(':').next() == Some("id") {
            Some(v.as_str())
        } else {
            None
        }
    })
}

fn collect_rids(el: &Element, out: &mut Vec<String>) {
    picture::walk_all(el, &mut |e| {
        if let Some(rid) = rid_attr(e) {
            if !out.iter().any(|x| x == rid) {
                out.push(rid.to_string());
            }
        }
    });
}

fn add_rel(pkg: &mut Package, source: &str, rel_type: &str, target: &str) -> Result<String, Error> {
    let mut rels = Relationships::from_package(pkg, source)?;
    let rid = rels.next_rid();
    rels.push(Relationship {
        id: rid.clone(),
        rel_type: rel_type.into(),
        target: picture::rel_target_from(source, target),
        target_mode: None,
        extra: Vec::new(),
    });
    rels.write_to(pkg, source)?;
    Ok(rid)
}

fn drop_unused_sheet_rels(
    pkg: &mut Package,
    source: &str,
    sheet: &Element,
    rids: &[String],
) -> Result<(), Error> {
    if rids.is_empty() {
        return Ok(());
    }
    let mut still = Vec::new();
    picture::walk_all(sheet, &mut |e| {
        if let Some(rid) = rid_attr(e) {
            still.push(rid.to_string());
        }
    });
    let mut rels = Relationships::from_package(pkg, source)?;
    let before = rels.rels.len();
    let mut dropped_targets = Vec::new();
    rels.rels.retain(|r| {
        if !rids.contains(&r.id) {
            return true;
        }
        if still.iter().any(|x| x == &r.id) {
            return true;
        }
        if !r.is_external() {
            if let Ok(part) = resolve_id(pkg, source, &r.id) {
                dropped_targets.push(part);
            }
        }
        false
    });
    if rels.rels.len() != before {
        rels.write_to(pkg, source)?;
    }
    for part in dropped_targets {
        if !picture::media_still_used(pkg, &part)? {
            pkg.remove(&part);
        }
    }
    Ok(())
}

fn drop_unused_rels(pkg: &mut Package, source: &str, rids: &[String]) -> Result<(), Error> {
    if rids.is_empty() {
        return Ok(());
    }
    let Some(xml_bytes) = pkg.get(source).ok().map(|b| b.to_vec()) else {
        return Ok(());
    };
    let text = String::from_utf8_lossy(&xml_bytes);
    let mut rels = Relationships::from_package(pkg, source)?;
    let before = rels.rels.len();
    let mut dropped_targets = Vec::new();
    rels.rels.retain(|r| {
        if !rids.contains(&r.id) {
            return true;
        }
        if text.contains(&r.id) {
            return true;
        }
        if !r.is_external() {
            if let Ok(part) = resolve_id(pkg, source, &r.id) {
                dropped_targets.push(part);
            }
        }
        false
    });
    if rels.rels.len() != before {
        rels.write_to(pkg, source)?;
    }
    for part in dropped_targets {
        if !picture::media_still_used(pkg, &part)? {
            pkg.remove(&part);
        }
    }
    Ok(())
}

fn cleanup_vml_media(pkg: &mut Package, vml_part: &str) -> Result<(), Error> {
    let Ok(bytes) = pkg.get(vml_part) else {
        return Ok(());
    };
    let text = String::from_utf8_lossy(bytes).into_owned();
    let mut rels = Relationships::from_package(pkg, vml_part)?;
    let mut dropped = Vec::new();
    rels.rels.retain(|r| {
        if r.rel_type.ends_with("/image") && !text.contains(&r.id) {
            if let Ok(part) = resolve_id(pkg, vml_part, &r.id) {
                dropped.push(part);
            }
            return false;
        }
        true
    });
    rels.write_to(pkg, vml_part)?;
    for part in dropped {
        if !picture::media_still_used(pkg, &part)? {
            pkg.remove(&part);
        }
    }
    Ok(())
}

fn drop_vml_if_empty(
    pkg: &mut Package,
    sheet_part: &str,
    sheet: &mut Element,
) -> Result<(), Error> {
    let Some(part) = vml_part(pkg, sheet_part)? else {
        return Ok(());
    };
    if pkg.contains(&part) {
        let text = String::from_utf8_lossy(pkg.get(&part)?);
        if text.contains("<v:shape") {
            return Ok(());
        }
    }
    pkg.remove(&part);
    pkg.remove(&crate::opc::rels_part_name(&part));
    sheet
        .children
        .retain(|n| !matches!(n, Node::Element(e) if e.local_name() == "legacyDrawing"));
    let mut rels = Relationships::from_package(pkg, sheet_part)?;
    rels.rels.retain(|r| r.rel_type != REL_VML);
    rels.write_to(pkg, sheet_part)?;
    Ok(())
}

fn vml_part(pkg: &Package, sheet_part: &str) -> Result<Option<String>, Error> {
    let rels = Relationships::from_package(pkg, sheet_part)?;
    let Some(rel) = rels.rels.iter().find(|r| r.rel_type == REL_VML) else {
        return Ok(None);
    };
    Ok(Some(resolve_id(pkg, sheet_part, &rel.id)?))
}

fn read_vml(pkg: &Package, sheet_part: &str) -> Result<Option<String>, Error> {
    let Some(part) = vml_part(pkg, sheet_part)? else {
        return Ok(None);
    };
    if !pkg.contains(&part) {
        return Ok(None);
    }
    Ok(Some(String::from_utf8_lossy(pkg.get(&part)?).into_owned()))
}

fn ensure_vml_part(
    pkg: &mut Package,
    sheet_part: &str,
    sheet: &mut Element,
) -> Result<String, Error> {
    if let Some(el) = sheet
        .element_children()
        .find(|e| e.local_name() == "legacyDrawing")
    {
        if let Some(rid) = rid_attr(el) {
            if let Ok(part) = resolve_id(pkg, sheet_part, rid) {
                if !pkg.contains(&part) {
                    pkg.set(&part, empty_vml())?;
                }
                return Ok(part);
            }
        }
    }
    if let Some(part) = vml_part(pkg, sheet_part)? {
        if !pkg.contains(&part) {
            pkg.set(&part, empty_vml())?;
        }
        if sheet
            .element_children()
            .all(|e| e.local_name() != "legacyDrawing")
        {
            let rels = Relationships::from_package(pkg, sheet_part)?;
            let rid = rels
                .rels
                .iter()
                .find(|r| r.rel_type == REL_VML)
                .map(|r| r.id.clone())
                .unwrap_or_else(|| "rId1".into());
            layout::insert_child_ordered(
                sheet,
                Element::new("legacyDrawing").with_attr("r:id", rid),
                "legacyDrawing",
            );
        }
        return Ok(part);
    }
    let mut n = 1u32;
    let part = loop {
        let name = format!("/xl/drawings/vmlDrawing{n}.vml");
        if !pkg.contains(&name) {
            break name;
        }
        n += 1;
    };
    pkg.set(&part, empty_vml())?;
    let rid = add_rel(pkg, sheet_part, REL_VML, &part)?;
    if sheet
        .element_children()
        .all(|e| e.local_name() != "legacyDrawing")
    {
        layout::insert_child_ordered(
            sheet,
            Element::new("legacyDrawing").with_attr("r:id", rid),
            "legacyDrawing",
        );
    }
    let mut cts = ContentTypes::from_package(pkg)?;
    cts.ensure_default("vml", CT_VML);
    cts.write_to(pkg)?;
    Ok(part)
}

fn empty_vml() -> Vec<u8> {
    format!(
        r#"<xml xmlns:v="urn:schemas-microsoft-com:vml" xmlns:o="urn:schemas-microsoft-com:office:office" xmlns:x="urn:schemas-microsoft-com:office:excel">{SHAPE_LAYOUT}{T75}</xml>"#
    )
    .into_bytes()
}

fn append_vml_pict(
    pkg: &mut Package,
    vml_part: &str,
    sid: u32,
    img_rid: &str,
    from: a1::CellRef,
    to: a1::CellRef,
    cx: i64,
    cy: i64,
) -> Result<(), Error> {
    let mut vml = String::from_utf8_lossy(pkg.get(vml_part)?).into_owned();
    if !vml.contains(r#"id="_x0000_t75""#) {
        vml = insert_before_xml_end(&vml, T75);
    }
    let w_pt = (cx as f64) / 12700.0;
    let h_pt = (cy as f64) / 12700.0;
    let fc = from.col.saturating_sub(1);
    let fr = from.row.saturating_sub(1);
    let tc = to.col.saturating_sub(1);
    let tr = to.row.saturating_sub(1);
    let shape = format!(
        r##"<v:shape id="_x0000_s{sid}" type="#_x0000_t75" style="position:absolute;margin-left:0;margin-top:0;width:{w_pt}pt;height:{h_pt}pt;z-index:1" filled="t" fillcolor="window [65]" stroked="t" strokecolor="windowText [64]" o:insetmode="auto"><v:fill color2="window [65]"/><v:imagedata o:relid="{img_rid}" o:title=""/><x:ClientData ObjectType="Pict"><x:SizeWithCells/><x:Anchor>{fc}, 0, {fr}, 0, {tc}, 0, {tr}, 0</x:Anchor><x:CF>Pict</x:CF><x:AutoPict/></x:ClientData></v:shape>"##
    );
    vml = insert_before_xml_end(&vml, &shape);
    pkg.set(vml_part, vml.into_bytes())?;
    Ok(())
}

fn insert_before_xml_end(vml: &str, chunk: &str) -> String {
    if let Some(i) = vml.rfind("</xml>") {
        let mut s = String::with_capacity(vml.len() + chunk.len());
        s.push_str(&vml[..i]);
        s.push_str(chunk);
        s.push_str(&vml[i..]);
        s
    } else {
        format!("{vml}{chunk}")
    }
}

fn remove_vml_shape(pkg: &mut Package, sheet_part: &str, sid: &str) -> Result<(), Error> {
    let Some(part) = vml_part(pkg, sheet_part)? else {
        return Ok(());
    };
    if !pkg.contains(&part) {
        return Ok(());
    }
    let vml = String::from_utf8_lossy(pkg.get(&part)?).into_owned();
    let needle = format!(r#"id="_x0000_s{sid}""#);
    let mut from = 0usize;
    while let Some(rel) = vml[from..].find("<v:shape") {
        let start = from + rel;
        let Some(rel_end) = vml[start..].find("</v:shape>") else {
            break;
        };
        let end = start + rel_end + "</v:shape>".len();
        let chunk = &vml[start..end];
        if chunk.contains(&needle) && chunk.contains(r#"ObjectType="Pict""#) {
            let mut out = String::with_capacity(vml.len());
            out.push_str(&vml[..start]);
            out.push_str(&vml[end..]);
            pkg.set(&part, out.into_bytes())?;
            return Ok(());
        }
        from = end;
    }
    Ok(())
}
