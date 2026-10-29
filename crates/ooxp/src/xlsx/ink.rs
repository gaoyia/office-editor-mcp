//! 工作表墨迹：一笔一个 `xdr:contentPart`，InkML 在 `/xl/ink/inkN.xml`。
//! 不进 `pictures`。Fallback 海报不进 `pictures`。不把笔画放进 inspect 树。

use serde_json::{json, Map, Value};

use crate::error::{Error, ErrorKind, IdReason, TypeName};
use crate::opc::{resolve_id, ContentTypes, Package, Relationship, Relationships};
use crate::xml::{self, Document, Element, Node};

use super::a1;
use super::picture::{self, AnchorKind, EditAs};
use super::shape;
use super::{ExportedMedia, REL_NS};

const MC_NS: &str = "http://schemas.openxmlformats.org/markup-compatibility/2006";
const XDR14_NS: &str = "http://schemas.microsoft.com/office/excel/2010/spreadsheetDrawing";
const INKML_NS: &str = "http://www.w3.org/2003/InkML";
pub const CT_INKML: &str = "application/inkml+xml";
const REL_CUSTOM_XML: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/customXml";

const DEFAULT_CX: i64 = 914_400;
const DEFAULT_CY: i64 = 914_400;

#[derive(Debug, Clone)]
pub enum InkKey {
    Id(String),
    Index(usize),
}

#[derive(Debug, Clone)]
pub struct InkTarget {
    pub sheet: String,
    pub key: InkKey,
}

#[derive(Debug, Clone)]
pub struct InsertInkOp {
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
pub struct ReplaceInkOp {
    pub id: String,
    pub target: InkTarget,
    pub file: String,
}

#[derive(Debug, Clone)]
pub struct DeleteInkOp {
    pub id: String,
    pub target: InkTarget,
}

#[derive(Debug, Clone)]
pub struct SetInkSizeOp {
    pub id: String,
    pub target: InkTarget,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
    pub to: Option<a1::CellRef>,
    pub to_off_x: Option<i64>,
    pub to_off_y: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct SetInkAnchorOp {
    pub id: String,
    pub target: InkTarget,
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
}

struct InkInfo {
    id: String,
    cnv_pr_id: Option<String>,
    name: String,
    anchor: AnchorKind,
    from: Option<String>,
    from_off_x: Option<i64>,
    from_off_y: Option<i64>,
    to: Option<String>,
    to_off_x: Option<i64>,
    to_off_y: Option<i64>,
    edit_as: Option<EditAs>,
    cx: Option<i64>,
    cy: Option<i64>,
    pos_x: Option<i64>,
    pos_y: Option<i64>,
    stable: bool,
}

pub fn looks_like_id(id: &str) -> bool {
    id.contains("/ink:") && id.starts_with("sheet:")
}

/// 工作表 drawing 里的墨迹 `contentPart`。形状几何另走 `nvSpPr`，这里补 `nvContentPartPr`。
pub(crate) fn is_ink_content_part(el: &Element) -> bool {
    el.local_name() == "contentPart"
}

pub fn parse_id(id: &str, json_path: &str) -> Result<InkTarget, Error> {
    let rest = id.strip_prefix("sheet:").ok_or_else(|| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SheetInkPrefix,
            },
            json_path.to_string(),
        )
    })?;
    let Some((name, after)) = rest.split_once("/ink:") else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: if rest.contains('/') {
                    IdReason::SheetInkPrefix
                } else {
                    IdReason::SheetInkContainer
                },
            },
            json_path.to_string(),
        ));
    };
    if name.is_empty() || after.is_empty() {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SheetInkPrefix,
            },
            json_path.to_string(),
        ));
    }
    let key = parse_key(after, json_path)?;
    Ok(InkTarget {
        sheet: name.to_string(),
        key,
    })
}

fn parse_key(s: &str, path: &str) -> Result<InkKey, Error> {
    if let Some(rest) = s.strip_prefix("index:") {
        let n: usize = rest.parse().map_err(|_| {
            Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::SheetInkPrefix,
                },
                path.to_string(),
            )
        })?;
        if n < 1 {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::SheetInkPrefix,
                },
                path.to_string(),
            ));
        }
        return Ok(InkKey::Index(n));
    }
    if s.is_empty() || s.contains('/') {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SheetInkPrefix,
            },
            path.to_string(),
        ));
    }
    Ok(InkKey::Id(s.to_string()))
}

pub fn count(pkg: &Package, sheet_part: &str, sheet_root: &Element) -> Result<usize, Error> {
    Ok(collect(pkg, sheet_part, sheet_root, "")?.len())
}

pub fn collect(
    pkg: &Package,
    sheet_part: &str,
    sheet_root: &Element,
    sheet_name: &str,
) -> Result<Vec<Value>, Error> {
    let mut out = Vec::new();
    let mut index = 0usize;
    for part in picture::drawing_parts_from(pkg, sheet_part, sheet_root)? {
        let doc = xml::parse(pkg.get(&part)?)?;
        picture::walk_anchors(&doc.root, &mut |anchor, kind| {
            let mut parts = Vec::new();
            collect_parts(anchor, &mut parts);
            for ink in parts {
                index += 1;
                out.push(info_json(&info_of(
                    anchor,
                    kind,
                    ink,
                    &format!("sheet:{sheet_name}"),
                    index,
                )?));
            }
            Ok(())
        })?;
    }
    Ok(out)
}

fn collect_parts<'a>(el: &'a Element, out: &mut Vec<&'a Element>) {
    if el.local_name() == "AlternateContent" {
        if let Some(choice) = el.child_by_local("Choice") {
            if choice_is_ink(choice) {
                for c in choice.element_children() {
                    if c.local_name() == "contentPart" {
                        out.push(c);
                    }
                }
            }
        }
        return;
    }
    for c in el.element_children() {
        collect_parts(c, out);
    }
}

fn choice_is_ink(choice: &Element) -> bool {
    let req = choice.attr_local("Requires").unwrap_or("");
    req.split_whitespace().any(|t| t == "xdr14")
        && choice
            .element_children()
            .any(|c| c.local_name() == "contentPart")
}

fn info_of(
    anchor: &Element,
    kind: AnchorKind,
    part: &Element,
    id_base: &str,
    index: usize,
) -> Result<InkInfo, Error> {
    let (raw, name) = cnv_of(part);
    let stable = raw.is_some();
    let id = match &raw {
        Some(v) => format!("{id_base}/ink:{v}"),
        None => format!("{id_base}/ink:index:{index}"),
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
    Ok(InkInfo {
        id,
        cnv_pr_id: raw,
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
        stable,
    })
}

fn cnv_of(part: &Element) -> (Option<String>, String) {
    let Some(nv) = part.child_by_local("nvContentPartPr") else {
        return (None, String::new());
    };
    let Some(pr) = nv.child_by_local("cNvPr") else {
        return (None, String::new());
    };
    (
        pr.attr_local("id")
            .filter(|s| !s.is_empty())
            .map(str::to_string),
        pr.attr_local("name").unwrap_or("").to_string(),
    )
}

fn info_json(p: &InkInfo) -> Value {
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
    m.insert("contentType".into(), json!(CT_INKML));
    m.insert("stable".into(), json!(p.stable));
    Value::Object(m)
}

pub fn export(bytes: &[u8], id: &str) -> Result<ExportedMedia, Error> {
    let pkg = Package::open_bytes(bytes.to_vec())?;
    let target = parse_id(id, "id")?;
    let sheet_part = picture::lookup_sheet_part(&pkg, &target.sheet, "id")?;
    let sheet_doc = xml::parse(pkg.get(&sheet_part)?)?;
    let drawing_part = drawing_for(&pkg, &sheet_part, &sheet_doc.root, &target.key, id, "id")?;
    let doc = xml::parse(pkg.get(&drawing_part)?)?;
    let part = find_part(&doc.root, &target.key)
        .ok_or_else(|| Error::with_path(ErrorKind::InkNotFound { id: id.to_string() }, "id"))?;
    let rid = part
        .attr_local("id")
        .ok_or_else(|| Error::with_path(ErrorKind::InkNotFound { id: id.to_string() }, "id"))?;
    let media = resolve_id(&pkg, &drawing_part, rid)?;
    let media_bytes = pkg.get(&media)?.to_vec();
    let cts = ContentTypes::from_package(&pkg)?;
    let content_type = cts.content_type_of(&media).map(str::to_string);
    Ok(ExportedMedia {
        bytes: media_bytes,
        content_type,
    })
}

fn drawing_for(
    pkg: &Package,
    sheet_part: &str,
    sheet_root: &Element,
    key: &InkKey,
    id: &str,
    path: &str,
) -> Result<String, Error> {
    for part in picture::drawing_parts_from(pkg, sheet_part, sheet_root)? {
        let doc = xml::parse(pkg.get(&part)?)?;
        if find_part(&doc.root, key).is_some() {
            return Ok(part);
        }
    }
    Err(Error::with_path(
        ErrorKind::InkNotFound { id: id.to_string() },
        path.to_string(),
    ))
}

fn find_part<'a>(root: &'a Element, key: &InkKey) -> Option<&'a Element> {
    let mut parts = Vec::new();
    collect_parts(root, &mut parts);
    for (i, part) in parts.into_iter().enumerate() {
        if key_matches(key, &cnv_of(part).0, i + 1) {
            return Some(part);
        }
    }
    None
}

fn key_matches(key: &InkKey, raw: &Option<String>, index: usize) -> bool {
    match key {
        InkKey::Index(n) => index == *n,
        InkKey::Id(want) => {
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

pub fn insert(
    pkg: &mut Package,
    sheet_doc: &mut Document,
    sheet_part: &str,
    op: &InsertInkOp,
    file_field: &str,
) -> Result<(), Error> {
    let inkml = read_inkml(&op.file, file_field)?;
    let (cx, cy) = match (op.cx, op.cy) {
        (Some(x), Some(y)) => (x, y),
        _ => (DEFAULT_CX, DEFAULT_CY),
    };
    let drawing_part = picture::ensure_drawing_part(pkg, sheet_doc, sheet_part)?;
    let mut draw_doc = xml::parse(pkg.get(&drawing_part)?)?;
    picture::ensure_xmlns(&mut draw_doc.root, "r", REL_NS);
    picture::ensure_xmlns(&mut draw_doc.root, "a", picture::A_NS);
    picture::ensure_xmlns(&mut draw_doc.root, "mc", MC_NS);
    picture::ensure_xmlns(&mut draw_doc.root, "xdr14", XDR14_NS);
    let existing = collect(pkg, sheet_part, &sheet_doc.root, "")?.len();
    let cnv = picture::next_cnv_id(pkg, sheet_part, &sheet_doc.root)?;
    let (ink_rid, png_rid) = write_media(pkg, &drawing_part, inkml)?;
    let name = format!("墨迹 {}", existing + 1);
    let ac = build_ac(
        cnv,
        &name,
        &ink_rid,
        &png_rid,
        cx,
        cy,
        op.from_off_x,
        op.from_off_y,
    );
    let mut anchor = Element::new(format!("xdr:{}", op.anchor.local_name()));
    if op.anchor == AnchorKind::TwoCell {
        anchor.set_attr("editAs", EditAs::OneCell.as_str());
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
        AnchorKind::Absolute | AnchorKind::RelSize | AnchorKind::AbsSize => {
            return Err(Error::with_path(ErrorKind::InvalidXlsxAnchor, "ops.anchor"));
        }
    }
    anchor.push_element(ac);
    anchor.push_element(Element::new("xdr:clientData"));
    draw_doc.root.push_element(anchor);
    pkg.set(&drawing_part, draw_doc.to_bytes()?)?;
    Ok(())
}

fn write_media(
    pkg: &mut Package,
    drawing_part: &str,
    inkml: Vec<u8>,
) -> Result<(String, String), Error> {
    let ink_part = crate::ole::next_numbered(pkg, "/xl/ink", "ink", "xml");
    pkg.set(&ink_part, inkml)?;
    let png_part = picture::next_media_part(pkg, "png");
    pkg.set(&png_part, crate::ole::PREVIEW_PNG.to_vec())?;
    let mut rels = Relationships::from_package(pkg, drawing_part)?;
    let ink_rid = rels.next_rid();
    rels.push(Relationship {
        id: ink_rid.clone(),
        rel_type: REL_CUSTOM_XML.into(),
        target: picture::rel_target_from(drawing_part, &ink_part),
        target_mode: None,
        extra: Vec::new(),
    });
    let png_rid = rels.next_rid();
    rels.push(Relationship {
        id: png_rid.clone(),
        rel_type: picture::REL_IMAGE.into(),
        target: picture::rel_target_from(drawing_part, &png_part),
        target_mode: None,
        extra: Vec::new(),
    });
    rels.write_to(pkg, drawing_part)?;
    let mut cts = ContentTypes::from_package(pkg)?;
    cts.set_override(&ink_part, CT_INKML)?;
    cts.ensure_default("png", "image/png");
    cts.write_to(pkg)?;
    Ok((ink_rid, png_rid))
}

fn build_ac(
    cnv: u32,
    name: &str,
    ink_rid: &str,
    png_rid: &str,
    cx: i64,
    cy: i64,
    off_x: i64,
    off_y: i64,
) -> Element {
    let mut ac = Element::new("mc:AlternateContent").with_attr("xmlns:mc", MC_NS);
    let mut choice = Element::new("mc:Choice")
        .with_attr("xmlns:xdr14", XDR14_NS)
        .with_attr("Requires", "xdr14");
    let mut part = Element::new("xdr:contentPart")
        .with_attr("xmlns:r", REL_NS)
        .with_attr("r:id", ink_rid);
    let mut nv = Element::new("xdr14:nvContentPartPr");
    nv.push_element(
        Element::new("xdr14:cNvPr")
            .with_attr("id", cnv.to_string())
            .with_attr("name", name),
    );
    nv.push_element(Element::new("xdr14:cNvContentPartPr"));
    part.push_element(nv);
    let mut xfrm = Element::new("xdr14:xfrm");
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
    part.push_element(xfrm);
    choice.push_element(part);
    ac.push_element(choice);
    let mut fallback = Element::new("mc:Fallback");
    fallback.push_element(build_fallback_pic(cnv, name, png_rid, cx, cy, off_x, off_y));
    ac.push_element(fallback);
    ac
}

fn build_fallback_pic(
    cnv: u32,
    name: &str,
    rid: &str,
    cx: i64,
    cy: i64,
    off_x: i64,
    off_y: i64,
) -> Element {
    let mut pic = Element::new("xdr:pic");
    let mut nv = Element::new("xdr:nvPicPr");
    nv.push_element(
        Element::new("xdr:cNvPr")
            .with_attr("id", cnv.to_string())
            .with_attr("name", name),
    );
    let mut cnv_pic = Element::new("xdr:cNvPicPr");
    cnv_pic.push_element(Element::new("a:picLocks").with_attr("noChangeAspect", "1"));
    nv.push_element(cnv_pic);
    pic.push_element(nv);
    let mut blip_fill = Element::new("xdr:blipFill");
    blip_fill.push_element(
        Element::new("a:blip")
            .with_attr("xmlns:r", REL_NS)
            .with_attr("r:embed", rid),
    );
    let mut stretch = Element::new("a:stretch");
    stretch.push_element(Element::new("a:fillRect"));
    blip_fill.push_element(stretch);
    pic.push_element(blip_fill);
    let mut sp_pr = Element::new("xdr:spPr");
    let mut xfrm = Element::new("a:xfrm");
    xfrm.push_element(
        Element::new("a:off")
            .with_attr("x", off_x.to_string())
            .with_attr("y", off_y.to_string()),
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
    pic.push_element(sp_pr);
    pic
}

pub fn replace(
    pkg: &mut Package,
    sheet_part: &str,
    op: &ReplaceInkOp,
    op_index: usize,
) -> Result<(), Error> {
    let path = format!("ops[{op_index}].id");
    let inkml = read_inkml(&op.file, &format!("ops[{op_index}].file"))?;
    let sheet_doc = xml::parse(pkg.get(sheet_part)?)?;
    let drawing_part = drawing_for(
        pkg,
        sheet_part,
        &sheet_doc.root,
        &op.target.key,
        &op.id,
        &path,
    )?;
    let doc = xml::parse(pkg.get(&drawing_part)?)?;
    let part = find_part(&doc.root, &op.target.key).ok_or_else(|| {
        Error::with_path(ErrorKind::InkNotFound { id: op.id.clone() }, path.clone())
    })?;
    let rid = part.attr_local("id").ok_or_else(|| {
        Error::with_path(ErrorKind::InkNotFound { id: op.id.clone() }, path.clone())
    })?;
    let ink_part = resolve_id(pkg, &drawing_part, rid)?;
    pkg.set(&ink_part, inkml)?;
    Ok(())
}

pub fn delete(
    pkg: &mut Package,
    sheet_doc: &mut Document,
    sheet_part: &str,
    op: &DeleteInkOp,
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
        .map(|ac| {
            let mut ids = Vec::new();
            collect_rids(ac, &mut ids);
            ids
        })
        .unwrap_or_default();
    if !remove_ink(&mut doc.root, &op.target.key) {
        return Err(Error::with_path(
            ErrorKind::InkNotFound { id: op.id.clone() },
            path,
        ));
    }
    remove_empty_anchors(&mut doc.root);
    pkg.set(&drawing_part, doc.to_bytes()?)?;
    drop_rids(pkg, &drawing_part, &doc.root, &rids)?;
    Ok(())
}

fn draw_key(key: &InkKey) -> shape::DrawKey {
    match key {
        InkKey::Id(s) => shape::DrawKey::Cnv(s.clone()),
        InkKey::Index(n) => shape::DrawKey::Index(*n),
    }
}

pub fn set_size(
    pkg: &mut Package,
    sheet_root: &Element,
    sheet_part: &str,
    op: &SetInkSizeOp,
    op_index: usize,
) -> Result<(), Error> {
    shape::set_object_size(
        pkg,
        sheet_root,
        sheet_part,
        &draw_key(&op.target.key),
        is_ink_content_part,
        picture::SizePatch {
            cx: op.cx,
            cy: op.cy,
            to: op.to,
            to_off_x: op.to_off_x,
            to_off_y: op.to_off_y,
        },
        ErrorKind::TwoCellInkSize,
        ErrorKind::InkNotFound { id: op.id.clone() },
        "setInkSize",
        op_index,
    )
}

pub fn set_anchor(
    pkg: &mut Package,
    sheet_root: &Element,
    sheet_part: &str,
    op: &SetInkAnchorOp,
    op_index: usize,
) -> Result<(), Error> {
    shape::set_object_anchor(
        pkg,
        sheet_root,
        sheet_part,
        &draw_key(&op.target.key),
        is_ink_content_part,
        shape::AnchorWrite {
            op: "setInkAnchor",
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
        ErrorKind::InkNotFound { id: op.id.clone() },
    )
}

fn find_ac<'a>(root: &'a Element, key: &InkKey) -> Option<&'a Element> {
    find_ac_walk(root, key, &mut 0)
}

fn find_ac_walk<'a>(el: &'a Element, key: &InkKey, index: &mut usize) -> Option<&'a Element> {
    crate::xml::search(el, |el| {
        if el.local_name() == "AlternateContent" {
            let mut parts = Vec::new();
            collect_parts(el, &mut parts);
            if !parts.is_empty() {
                let mut hit = false;
                for part in &parts {
                    *index += 1;
                    if key_matches(key, &cnv_of(part).0, *index) {
                        hit = true;
                    }
                }
                if hit {
                    return crate::xml::Search::Hit(el);
                }
                return crate::xml::Search::Skip;
            }
        }
        crate::xml::Search::down(el)
    })
}

fn collect_rids(el: &Element, out: &mut Vec<String>) {
    if el.local_name() == "contentPart" {
        if let Some(id) = el.attr_local("id").filter(|s| !s.is_empty()) {
            out.push(id.to_string());
        }
    }
    if el.local_name() == "blip" {
        if let Some(id) = el.attr_local("embed").filter(|s| !s.is_empty()) {
            out.push(id.to_string());
        }
    }
    for c in el.element_children() {
        collect_rids(c, out);
    }
}

fn remove_ink(el: &mut Element, key: &InkKey) -> bool {
    remove_in(el, key, &mut 0)
}

fn remove_in(el: &mut Element, key: &InkKey, index: &mut usize) -> bool {
    let mut drop_at = None;
    for (i, n) in el.children.iter_mut().enumerate() {
        let Node::Element(child) = n else { continue };
        if child.local_name() == "AlternateContent" {
            let mut parts = Vec::new();
            collect_parts(child, &mut parts);
            let mut hit = false;
            for part in parts {
                *index += 1;
                if key_matches(key, &cnv_of(part).0, *index) {
                    hit = true;
                }
            }
            if hit {
                drop_at = Some(i);
                break;
            }
        } else if remove_in(child, key, index) {
            return true;
        }
    }
    if let Some(i) = drop_at {
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
    let mut cts = ContentTypes::from_package(pkg)?;
    let mut cts_dirty = false;
    for m in media_parts {
        if !picture::media_still_used(pkg, &m)? {
            pkg.remove(&m);
            if m.contains("/ink/") {
                cts.remove_override(&m);
                cts_dirty = true;
            }
        }
    }
    if cts_dirty {
        cts.write_to(pkg)?;
    }
    Ok(())
}

fn rid_still_used(root: &Element, rid: &str) -> bool {
    let mut used = false;
    walk(root, &mut |e| {
        if e.local_name() == "contentPart" && e.attr_local("id") == Some(rid) {
            used = true;
        }
        if e.local_name() == "blip" && e.attr_local("embed") == Some(rid) {
            used = true;
        }
    });
    used
}

fn walk(el: &Element, f: &mut dyn FnMut(&Element)) {
    crate::xml::walk_elements(el, &mut |node| f(node));
}

fn read_inkml(path: &str, json_path: &str) -> Result<Vec<u8>, Error> {
    let bytes = crate::host_files::read_user_file(path)?;
    validate_inkml(&bytes, json_path)?;
    Ok(bytes)
}

fn validate_inkml(bytes: &[u8], json_path: &str) -> Result<(), Error> {
    let err = || Error::with_path(ErrorKind::InvalidInkMl, json_path.to_string());
    let doc = xml::parse(bytes).map_err(|_| err())?;
    if !is_inkml_root(&doc.root) {
        return Err(err());
    }
    Ok(())
}

pub fn parse_create(v: &Value, prefix: &str, sheet: &str) -> Result<InsertInkOp, Error> {
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
            "file", "cx", "cy", "from", "to", "anchor", "fromOffX", "fromOffY", "toOffX", "toOffY",
        ],
    )?;
    let file = match obj.get("file") {
        Some(Value::String(s)) if !s.is_empty() => s.clone(),
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "file".into(),
                    expected: TypeName::NonEmptyString,
                },
                format!("{prefix}.file"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "file".into(),
                },
                format!("{prefix}.file"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "file".into(),
                    expected: TypeName::String,
                },
                format!("{prefix}.file"),
            ));
        }
    };
    let cx = parse_emu(obj.get("cx"), prefix, "cx")?;
    let cy = parse_emu(obj.get("cy"), prefix, "cy")?;
    if cx.is_some() != cy.is_some() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "insertInk".into(),
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
                op: "insertInk".into(),
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
    Ok(InsertInkOp {
        sheet: sheet.to_string(),
        file,
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
    let n = v.as_i64().filter(|n| *n > 0).ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Integer,
            },
            format!("{prefix}.{field}"),
        )
    })?;
    Ok(Some(n))
}

fn parse_off(v: Option<&Value>, prefix: &str, field: &str) -> Result<Option<i64>, Error> {
    let Some(v) = v else {
        return Ok(None);
    };
    let n = v.as_i64().filter(|n| *n >= 0).ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Integer,
            },
            format!("{prefix}.{field}"),
        )
    })?;
    Ok(Some(n))
}

fn is_inkml_root(el: &Element) -> bool {
    if el.local_name() != "ink" {
        return false;
    }
    el.attrs
        .iter()
        .any(|(k, v)| (k == "xmlns" || k.starts_with("xmlns:")) && v == INKML_NS)
}
