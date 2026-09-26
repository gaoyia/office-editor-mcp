//! 工作表 SpreadsheetDrawing 图片：收集、换、插、删、改尺寸与锚点。

use std::path::{Path, PathBuf};

use serde_json::{json, Map, Value};

use crate::error::{io_path, Error, ErrorKind, IdReason};
use crate::opc::{
    rels_part_name, resolve_rel_target, split_dir_file, ContentTypes, Package, Relationship,
    Relationships,
};
use crate::xml::{self, Document, Element, Node, DEFAULT_DECL};

use super::a1;
use super::request::{
    DeletePictureOp, InsertPictureOp, ReplacePictureOp, SetPictureAnchorOp, SetPictureSizeOp,
};
use super::REL_NS;

pub const XDR_NS: &str = "http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing";
pub const A_NS: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";
pub const REL_DRAWING: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/drawing";
pub const REL_IMAGE: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/image";
pub const CT_DRAWING: &str = "application/vnd.openxmlformats-officedocument.drawing+xml";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnchorKind {
    TwoCell,
    OneCell,
    Absolute,
}

impl AnchorKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TwoCell => "twoCell",
            Self::OneCell => "oneCell",
            Self::Absolute => "absolute",
        }
    }

    pub fn parse_insert(s: &str) -> Option<Self> {
        match s {
            "oneCell" => Some(Self::OneCell),
            "twoCell" => Some(Self::TwoCell),
            _ => None,
        }
    }

    fn from_local(name: &str) -> Option<Self> {
        match name {
            "twoCellAnchor" => Some(Self::TwoCell),
            "oneCellAnchor" => Some(Self::OneCell),
            "absoluteAnchor" => Some(Self::Absolute),
            _ => None,
        }
    }

    fn local_name(self) -> &'static str {
        match self {
            Self::TwoCell => "twoCellAnchor",
            Self::OneCell => "oneCellAnchor",
            Self::Absolute => "absoluteAnchor",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditAs {
    TwoCell,
    OneCell,
    Absolute,
}

impl EditAs {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TwoCell => "twoCell",
            Self::OneCell => "oneCell",
            Self::Absolute => "absolute",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "twoCell" => Some(Self::TwoCell),
            "oneCell" => Some(Self::OneCell),
            "absolute" => Some(Self::Absolute),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum PicKey {
    Cnv(String),
    Index(usize),
}

#[derive(Debug, Clone)]
pub struct SheetPicId {
    pub sheet: String,
    pub key: PicKey,
}

pub struct PicInfo {
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
    pub pos_x: Option<i64>,
    pub pos_y: Option<i64>,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
    pub edit_as: Option<EditAs>,
    pub stable: bool,
}

pub struct ImageFile {
    pub bytes: Vec<u8>,
    pub ext: String,
    pub content_type: String,
    pub cx: i64,
    pub cy: i64,
}

pub fn parse_insert_sheet_id(id: &str, op_index: usize) -> Result<String, Error> {
    if id.contains("/pic:") {
        return Err(invalid_id(IdReason::InsertNeedsSheet, op_index));
    }
    let Some(name) = id.strip_prefix("sheet:") else {
        return Err(invalid_id(IdReason::InsertNeedsSheet, op_index));
    };
    if name.is_empty() {
        return Err(invalid_id(IdReason::InsertNeedsSheet, op_index));
    }
    Ok(name.to_string())
}

pub fn parse_pic_id(id: &str, op_index: usize) -> Result<SheetPicId, Error> {
    parse_pic_id_at(id, &format!("ops[{op_index}].id"))
}

pub fn parse_pic_id_at(id: &str, field_path: &str) -> Result<SheetPicId, Error> {
    if let Some(name) = id.strip_prefix("sheet:") {
        if let Some((sheet, rest)) = name.split_once("/pic:") {
            if sheet.is_empty() || rest.is_empty() {
                return Err(Error::with_path(
                    ErrorKind::InvalidId {
                        reason: IdReason::SheetPicPrefix,
                    },
                    field_path,
                ));
            }
            let key = if let Some(idx) = rest.strip_prefix("index:") {
                let n: usize = idx.parse().map_err(|_| {
                    Error::with_path(
                        ErrorKind::InvalidId {
                            reason: IdReason::PicIndexNotInt,
                        },
                        field_path,
                    )
                })?;
                if n == 0 {
                    return Err(Error::with_path(
                        ErrorKind::InvalidId {
                            reason: IdReason::PicIndexFromOne,
                        },
                        field_path,
                    ));
                }
                PicKey::Index(n)
            } else {
                PicKey::Cnv(rest.to_string())
            };
            return Ok(SheetPicId {
                sheet: sheet.to_string(),
                key,
            });
        }
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SheetPicContainer,
            },
            field_path,
        ));
    }
    Err(Error::with_path(
        ErrorKind::InvalidId {
            reason: IdReason::SheetPicPrefix,
        },
        field_path,
    ))
}

fn invalid_id(reason: IdReason, op_index: usize) -> Error {
    Error::with_path(ErrorKind::InvalidId { reason }, format!("ops[{op_index}].id"))
}

pub fn collect_sheet_pictures(
    pkg: &Package,
    sheet_part: &str,
    sheet_name: &str,
) -> Result<Vec<PicInfo>, Error> {
    let mut out = Vec::new();
    let mut index = 0usize;
    for drawing_part in drawing_parts(pkg, sheet_part)? {
        let doc = xml::parse(pkg.get(&drawing_part)?)?;
        walk_anchors(&doc.root, &mut |anchor, kind| {
            let Some(pic) = direct_pic(anchor) else {
                return Ok(());
            };
            if !has_blip(pic) {
                return Ok(());
            }
            index += 1;
            out.push(pic_info(anchor, kind, pic, sheet_name, index)?);
            Ok(())
        })?;
    }
    Ok(out)
}

pub fn pictures_json(pics: &[PicInfo]) -> Vec<Value> {
    pics.iter().map(pic_to_json).collect()
}

fn pic_to_json(p: &PicInfo) -> Value {
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
    }
    m.insert("stable".into(), json!(p.stable));
    Value::Object(m)
}

pub fn export_media(bytes: &[u8], id: &str) -> Result<super::ExportedMedia, Error> {
    let pkg = Package::open_bytes(bytes.to_vec())?;
    let target = parse_pic_id_at(id, "id")?;
    let sheet_part = lookup_sheet_part(&pkg, &target.sheet, "id")?;
    let drawing_part = find_drawing_for(&pkg, &sheet_part, &target, id, "id")?;
    let doc = xml::parse(pkg.get(&drawing_part)?)?;
    let rid = find_blip_embed(&doc.root, &target.key).ok_or_else(|| {
        Error::with_path(
            ErrorKind::PartMissing {
                name: format!("{id} a:blip/@r:embed"),
            },
            "id",
        )
    })?;
    let media = crate::opc::resolve_id(&pkg, &drawing_part, &rid)?;
    let media_bytes = pkg.get(&media)?.to_vec();
    let cts = ContentTypes::from_package(&pkg)?;
    let content_type = cts.content_type_of(&media).map(str::to_string);
    Ok(super::ExportedMedia {
        bytes: media_bytes,
        content_type,
    })
}

pub fn replace_picture(
    pkg: &mut Package,
    sheet_part: &str,
    op: &ReplacePictureOp,
    op_index: usize,
) -> Result<(), Error> {
    let path = format!("ops[{op_index}].id");
    let drawing_part = find_drawing_for(pkg, sheet_part, &op.target, &op.id, &path)?;
    let doc = xml::parse(pkg.get(&drawing_part)?)?;
    let rid = find_blip_embed(&doc.root, &op.target.key).ok_or_else(|| {
        Error::with_path(
            ErrorKind::PictureNotFound {
                id: op.id.clone(),
            },
            path,
        )
    })?;
    let old_media = crate::opc::resolve_id(pkg, &drawing_part, &rid)?;
    let image = read_image(&op.file).map_err(|e| e.at(format!("ops[{op_index}].file")))?;
    let media = next_media_part(pkg, &image.ext);
    pkg.set(&media, image.bytes)?;
    update_rel_target(pkg, &drawing_part, &rid, &media)?;
    let mut cts = ContentTypes::from_package(pkg)?;
    cts.ensure_default(&image.ext, &image.content_type);
    cts.write_to(pkg)?;
    if old_media != media && !media_still_used(pkg, &old_media)? {
        pkg.remove(&old_media);
    }
    Ok(())
}

pub fn insert_picture(
    pkg: &mut Package,
    sheet_doc: &mut Document,
    sheet_part: &str,
    op: &InsertPictureOp,
    op_index: usize,
) -> Result<(), Error> {
    let image = read_image(&op.file).map_err(|e| e.at(format!("ops[{op_index}].file")))?;
    let (cx, cy) = match (op.cx, op.cy) {
        (Some(x), Some(y)) => (x, y),
        _ => (image.cx, image.cy),
    };
    let drawing_part = ensure_drawing_part(pkg, sheet_doc, sheet_part)?;
    let mut draw_doc = xml::parse(pkg.get(&drawing_part)?)?;
    // Excel 存的 wsDr 常常只在 a:blip 上写 xmlns:r，根上没有。新锚点用 r:embed，必须把关系命名空间补到根上。
    ensure_xmlns(&mut draw_doc.root, "r", REL_NS);
    ensure_xmlns(&mut draw_doc.root, "a", A_NS);
    let cnv = next_cnv_id(pkg, sheet_part)?;
    let media = next_media_part(pkg, &image.ext);
    pkg.set(&media, image.bytes)?;
    let rid = add_image_rel(pkg, &drawing_part, &media)?;
    let mut cts = ContentTypes::from_package(pkg)?;
    cts.ensure_default(&image.ext, &image.content_type);
    cts.set_override(&drawing_part, CT_DRAWING)?;
    cts.write_to(pkg)?;
    let name = format!("图片 {cnv}");
    let anchor = build_anchor(
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
        &rid,
        None,
    );
    draw_doc.root.push_element(anchor);
    pkg.set(&drawing_part, draw_doc.to_bytes()?)?;
    Ok(())
}

pub fn delete_picture(
    pkg: &mut Package,
    sheet_doc: &mut Document,
    sheet_part: &str,
    op: &DeletePictureOp,
    op_index: usize,
) -> Result<(), Error> {
    let path = format!("ops[{op_index}].id");
    let drawing_part = find_drawing_for(pkg, sheet_part, &op.target, &op.id, &path)?;
    let mut doc = xml::parse(pkg.get(&drawing_part)?)?;
    let rid = find_blip_rid(&doc.root, &op.target.key).ok_or_else(|| {
        Error::with_path(
            ErrorKind::PictureNotFound {
                id: op.id.clone(),
            },
            path.clone(),
        )
    })?;
    let media = crate::opc::resolve_id(pkg, &drawing_part, &rid).ok();
    if !remove_anchor(&mut doc.root, &op.target.key) {
        return Err(Error::with_path(
            ErrorKind::PictureNotFound {
                id: op.id.clone(),
            },
            path,
        ));
    }
    if drawing_is_empty(&doc.root) {
        pkg.remove(&drawing_part);
        pkg.remove(&rels_part_name(&drawing_part));
        remove_drawing_ref(&mut sheet_doc.root, sheet_part, pkg, &drawing_part)?;
        let mut cts = ContentTypes::from_package(pkg)?;
        cts.remove_override(&drawing_part);
        cts.write_to(pkg)?;
    } else {
        pkg.set(&drawing_part, doc.to_bytes()?)?;
        let mut rels = Relationships::from_package(pkg, &drawing_part)?;
        rels.remove_by_id(&rid);
        if rels.rels.is_empty() {
            pkg.remove(&rels_part_name(&drawing_part));
        } else {
            rels.write_to(pkg, &drawing_part)?;
        }
    }
    if let Some(media) = media {
        if !media_still_used(pkg, &media)? {
            pkg.remove(&media);
        }
    }
    Ok(())
}

pub fn set_picture_size(
    pkg: &mut Package,
    sheet_part: &str,
    op: &SetPictureSizeOp,
    op_index: usize,
) -> Result<(), Error> {
    let path = format!("ops[{op_index}].id");
    let drawing_part = find_drawing_for(pkg, sheet_part, &op.target, &op.id, &path)?;
    let mut doc = xml::parse(pkg.get(&drawing_part)?)?;
    let kind = find_anchor_kind(&doc.root, &op.target.key).ok_or_else(|| {
        Error::with_path(
            ErrorKind::PictureNotFound {
                id: op.id.clone(),
            },
            path.clone(),
        )
    })?;
    if kind == AnchorKind::TwoCell {
        return Err(Error::with_path(ErrorKind::TwoCellPictureSize, path));
    }
    if !update_extent(&mut doc.root, &op.target.key, op.cx, op.cy) {
        return Err(Error::with_path(
            ErrorKind::PictureNotFound {
                id: op.id.clone(),
            },
            path,
        ));
    }
    pkg.set(&drawing_part, doc.to_bytes()?)?;
    Ok(())
}

pub fn set_picture_anchor(
    pkg: &mut Package,
    sheet_part: &str,
    op: &SetPictureAnchorOp,
    op_index: usize,
) -> Result<(), Error> {
    let path = format!("ops[{op_index}].id");
    let drawing_part = find_drawing_for(pkg, sheet_part, &op.target, &op.id, &path)?;
    let mut doc = xml::parse(pkg.get(&drawing_part)?)?;
    let current = find_anchor_kind(&doc.root, &op.target.key).ok_or_else(|| {
        Error::with_path(
            ErrorKind::PictureNotFound {
                id: op.id.clone(),
            },
            path.clone(),
        )
    })?;
    let target_kind = op.anchor.unwrap_or(current);
    if current == AnchorKind::Absolute {
        if op.anchor.is_some_and(|a| a != AnchorKind::Absolute)
            || op.from.is_some()
            || op.to.is_some()
        {
            return Err(Error::with_path(ErrorKind::PictureAnchorAbsolute, path));
        }
        if !update_absolute_pos(&mut doc.root, &op.target.key, op.pos_x, op.pos_y) {
            return Err(Error::with_path(
                ErrorKind::PictureNotFound {
                    id: op.id.clone(),
                },
                path,
            ));
        }
        pkg.set(&drawing_part, doc.to_bytes()?)?;
        return Ok(());
    }
    if target_kind == AnchorKind::Absolute || op.pos_x.is_some() || op.pos_y.is_some() {
        return Err(Error::with_path(ErrorKind::PictureAnchorAbsolute, path));
    }
    // 转锚点会重写 xdr:pic；Excel 原 drawing 根上常常没有 xmlns:r。
    ensure_xmlns(&mut doc.root, "r", REL_NS);
    ensure_xmlns(&mut doc.root, "a", A_NS);
    let snapshot = snapshot_anchor(&doc.root, &op.target.key).ok_or_else(|| {
        Error::with_path(
            ErrorKind::PictureNotFound {
                id: op.id.clone(),
            },
            path.clone(),
        )
    })?;
    let from = op.from.or(snapshot.from).ok_or_else(|| {
        Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setPictureAnchor".into(),
                field: "from".into(),
            },
            format!("ops[{op_index}].from"),
        )
    })?;
    let to = if target_kind == AnchorKind::TwoCell {
        Some(op.to.or(snapshot.to).ok_or_else(|| {
            Error::with_path(
                ErrorKind::OpRequiresField {
                    op: "setPictureAnchor".into(),
                    field: "to".into(),
                },
                format!("ops[{op_index}].to"),
            )
        })?)
    } else {
        None
    };
    let cx = if target_kind == AnchorKind::OneCell {
        op.cx.or(snapshot.cx).ok_or_else(|| {
            Error::with_path(
                ErrorKind::OpRequiresField {
                    op: "setPictureAnchor".into(),
                    field: "cx and cy".into(),
                },
                format!("ops[{op_index}]"),
            )
        })?
    } else {
        snapshot.cx.unwrap_or(914400)
    };
    let cy = if target_kind == AnchorKind::OneCell {
        op.cy.or(snapshot.cy).ok_or_else(|| {
            Error::with_path(
                ErrorKind::OpRequiresField {
                    op: "setPictureAnchor".into(),
                    field: "cx and cy".into(),
                },
                format!("ops[{op_index}]"),
            )
        })?
    } else {
        snapshot.cy.unwrap_or(914400)
    };
    let from_off_x = op.from_off_x.unwrap_or(snapshot.from_off_x);
    let from_off_y = op.from_off_y.unwrap_or(snapshot.from_off_y);
    let to_off_x = op.to_off_x.unwrap_or(snapshot.to_off_x);
    let to_off_y = op.to_off_y.unwrap_or(snapshot.to_off_y);
    let edit_as = op.edit_as.unwrap_or(snapshot.edit_as);
    replace_anchor(
        &mut doc.root,
        &op.target.key,
        build_anchor(
            target_kind,
            from,
            to,
            from_off_x,
            from_off_y,
            to_off_x,
            to_off_y,
            None,
            None,
            cx,
            cy,
            edit_as,
            snapshot.cnv,
            &snapshot.name,
            &snapshot.rid,
            Some(snapshot.pic),
        ),
    );
    pkg.set(&drawing_part, doc.to_bytes()?)?;
    Ok(())
}

struct AnchorSnap {
    from: Option<a1::CellRef>,
    to: Option<a1::CellRef>,
    from_off_x: i64,
    from_off_y: i64,
    to_off_x: i64,
    to_off_y: i64,
    cx: Option<i64>,
    cy: Option<i64>,
    edit_as: EditAs,
    cnv: u32,
    name: String,
    rid: String,
    pic: Element,
}

fn snapshot_anchor(root: &Element, key: &PicKey) -> Option<AnchorSnap> {
    let mut index = 0usize;
    let mut found = None;
    let _ = walk_anchors(root, &mut |anchor, kind| {
        if found.is_some() {
            return Ok(());
        }
        let Some(pic) = direct_pic(anchor) else {
            return Ok(());
        };
        if !has_blip(pic) {
            return Ok(());
        }
        index += 1;
        if !key_matches(key, &cnv_of(pic).0, index) {
            return Ok(());
        }
        let (from, from_off_x, from_off_y) = read_marker(anchor.child_by_local("from"));
        let (to, to_off_x, to_off_y) = read_marker(anchor.child_by_local("to"));
        let (cx, cy) = read_ext(anchor);
        let rid = blip_rid(pic).unwrap_or_default();
        let (cnv_raw, name) = cnv_of(pic);
        let cnv = cnv_raw.and_then(|s| s.parse().ok()).unwrap_or(1);
        found = Some(AnchorSnap {
            from,
            to,
            from_off_x,
            from_off_y,
            to_off_x,
            to_off_y,
            cx,
            cy,
            edit_as: if kind == AnchorKind::TwoCell {
                parse_edit_as(anchor.attr("editAs"))
            } else {
                EditAs::TwoCell
            },
            cnv,
            name,
            rid,
            pic: pic.clone(),
        });
        Ok(())
    });
    found
}

fn lookup_sheet_part(pkg: &Package, sheet_name: &str, field_path: &str) -> Result<String, Error> {
    let wb_part = crate::opc::office_document_part(pkg)?;
    let wb = xml::parse(pkg.get(&wb_part)?)?;
    let rels = Relationships::from_package(pkg, &wb_part)?;
    let sheets = super::inspect::list_sheets(&wb.root, &rels, &wb_part, pkg)?;
    sheets
        .iter()
        .find(|s| s.name.eq_ignore_ascii_case(sheet_name))
        .map(|s| s.part.clone())
        .ok_or_else(|| {
            Error::with_path(
                ErrorKind::SheetNotFound {
                    name: sheet_name.to_string(),
                },
                field_path,
            )
        })
}

fn drawing_parts(pkg: &Package, sheet_part: &str) -> Result<Vec<String>, Error> {
    let rels = Relationships::from_package(pkg, sheet_part)?;
    let mut out = Vec::new();
    for rel in rels
        .rels
        .iter()
        .filter(|r| r.rel_type == REL_DRAWING || r.rel_type.ends_with("/drawing"))
    {
        if rel.is_external() {
            continue;
        }
        let part = resolve_rel_target(sheet_part, &rel.target)?;
        if pkg.contains(&part) {
            out.push(part);
        }
    }
    Ok(out)
}

fn find_drawing_for(
    pkg: &Package,
    sheet_part: &str,
    target: &SheetPicId,
    id: &str,
    field_path: &str,
) -> Result<String, Error> {
    for part in drawing_parts(pkg, sheet_part)? {
        let doc = xml::parse(pkg.get(&part)?)?;
        if find_blip_rid(&doc.root, &target.key).is_some() {
            return Ok(part);
        }
    }
    Err(Error::with_path(
        ErrorKind::PictureNotFound { id: id.to_string() },
        field_path,
    ))
}

fn walk_anchors<F>(root: &Element, f: &mut F) -> Result<(), Error>
where
    F: FnMut(&Element, AnchorKind) -> Result<(), Error>,
{
    walk_anchor_nodes(root, f)
}

fn walk_anchor_nodes<F>(el: &Element, f: &mut F) -> Result<(), Error>
where
    F: FnMut(&Element, AnchorKind) -> Result<(), Error>,
{
    if el.local_name() == "AlternateContent" {
        if let Some(choice) = el.child_by_local("Choice") {
            return walk_anchor_nodes(choice, f);
        }
        if let Some(fallback) = el.child_by_local("Fallback") {
            return walk_anchor_nodes(fallback, f);
        }
        return Ok(());
    }
    if let Some(kind) = AnchorKind::from_local(el.local_name()) {
        return f(el, kind);
    }
    if el.local_name() == "grpSp" {
        return Ok(());
    }
    for child in el.element_children() {
        walk_anchor_nodes(child, f)?;
    }
    Ok(())
}

fn walk_anchors_mut<F>(el: &mut Element, f: &mut F) -> bool
where
    F: FnMut(&mut Element, AnchorKind) -> bool,
{
    if el.local_name() == "AlternateContent" {
        if let Some(choice) = el.child_by_local_mut("Choice") {
            return walk_anchors_mut(choice, f);
        }
        if let Some(fallback) = el.child_by_local_mut("Fallback") {
            return walk_anchors_mut(fallback, f);
        }
        return false;
    }
    if let Some(kind) = AnchorKind::from_local(el.local_name()) {
        return f(el, kind);
    }
    if el.local_name() == "grpSp" {
        return false;
    }
    let n = el.children.len();
    for i in 0..n {
        if let Node::Element(child) = &mut el.children[i] {
            if walk_anchors_mut(child, f) {
                return true;
            }
        }
    }
    false
}

fn direct_pic(anchor: &Element) -> Option<&Element> {
    anchor
        .element_children()
        .find(|c| c.local_name() == "pic" && has_blip(c))
}

fn has_blip(el: &Element) -> bool {
    let mut found = false;
    walk_all(el, &mut |e| {
        if e.local_name() == "blip" {
            found = true;
        }
    });
    found
}

fn walk_all(el: &Element, f: &mut impl FnMut(&Element)) {
    f(el);
    for c in el.element_children() {
        walk_all(c, f);
    }
}

fn pic_info(
    anchor: &Element,
    kind: AnchorKind,
    pic: &Element,
    sheet_name: &str,
    index: usize,
) -> Result<PicInfo, Error> {
    let (raw_id, name) = cnv_of(pic);
    let stable = raw_id.is_some();
    let id = match &raw_id {
        Some(v) => format!("sheet:{sheet_name}/pic:{v}"),
        None => format!("sheet:{sheet_name}/pic:index:{index}"),
    };
    let (from, from_off_x, from_off_y) = read_marker(anchor.child_by_local("from"));
    let (to, to_off_x, to_off_y) = read_marker(anchor.child_by_local("to"));
    let (cx, cy) = read_ext(anchor);
    let (pos_x, pos_y) = read_pos(anchor);
    Ok(PicInfo {
        id,
        cnv_pr_id: raw_id,
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
        pos_x,
        pos_y,
        cx: if kind == AnchorKind::TwoCell { None } else { cx },
        cy: if kind == AnchorKind::TwoCell { None } else { cy },
        edit_as: if kind == AnchorKind::TwoCell {
            Some(parse_edit_as(anchor.attr("editAs")))
        } else {
            None
        },
        stable,
    })
}

fn cnv_of(pic: &Element) -> (Option<String>, String) {
    let mut raw = None;
    let mut name = String::new();
    walk_all(pic, &mut |e| {
        if e.local_name() == "cNvPr" && raw.is_none() {
            raw = e.attr_local("id").map(str::to_string);
            name = e.attr_local("name").unwrap_or("").to_string();
        }
    });
    (raw, name)
}

fn read_marker(el: Option<&Element>) -> (Option<a1::CellRef>, i64, i64) {
    let Some(el) = el else {
        return (None, 0, 0);
    };
    let col = el
        .child_by_local("col")
        .map(|e| e.text_content())
        .and_then(|s| s.trim().parse::<i64>().ok())
        .unwrap_or(0);
    let row = el
        .child_by_local("row")
        .map(|e| e.text_content())
        .and_then(|s| s.trim().parse::<i64>().ok())
        .unwrap_or(0);
    let col_off = el
        .child_by_local("colOff")
        .map(|e| e.text_content())
        .and_then(|s| s.trim().parse::<i64>().ok())
        .unwrap_or(0);
    let row_off = el
        .child_by_local("rowOff")
        .map(|e| e.text_content())
        .and_then(|s| s.trim().parse::<i64>().ok())
        .unwrap_or(0);
    let cell = a1::CellRef {
        col: (col + 1).max(1) as u32,
        row: (row + 1).max(1) as u32,
    };
    (Some(cell), col_off, row_off)
}

fn read_ext(anchor: &Element) -> (Option<i64>, Option<i64>) {
    let ext = anchor.child_by_local("ext");
    let cx = ext.and_then(|e| e.attr_local("cx")).and_then(|s| s.parse().ok());
    let cy = ext.and_then(|e| e.attr_local("cy")).and_then(|s| s.parse().ok());
    (cx, cy)
}

fn read_pos(anchor: &Element) -> (Option<i64>, Option<i64>) {
    let pos = anchor.child_by_local("pos");
    let x = pos.and_then(|e| e.attr_local("x")).and_then(|s| s.parse().ok());
    let y = pos.and_then(|e| e.attr_local("y")).and_then(|s| s.parse().ok());
    (x, y)
}

fn parse_edit_as(s: Option<&str>) -> EditAs {
    match s {
        Some("oneCell") => EditAs::OneCell,
        Some("absolute") => EditAs::Absolute,
        _ => EditAs::TwoCell,
    }
}

fn blip_rid(pic: &Element) -> Option<String> {
    let mut found = None;
    walk_all(pic, &mut |e| {
        if e.local_name() == "blip" && found.is_none() {
            found = e
                .attr_local("embed")
                .or_else(|| e.attr_local("link"))
                .map(str::to_string);
        }
    });
    found
}

fn find_blip_rid(root: &Element, key: &PicKey) -> Option<String> {
    find_blip_attr(root, key, false)
}

fn find_blip_embed(root: &Element, key: &PicKey) -> Option<String> {
    find_blip_attr(root, key, true)
}

fn find_blip_attr(root: &Element, key: &PicKey, embed_only: bool) -> Option<String> {
    let mut index = 0usize;
    let mut rid = None;
    let _ = walk_anchors(root, &mut |anchor, _| {
        if rid.is_some() {
            return Ok(());
        }
        let Some(pic) = direct_pic(anchor) else {
            return Ok(());
        };
        if !has_blip(pic) {
            return Ok(());
        }
        index += 1;
        if key_matches(key, &cnv_of(pic).0, index) {
            rid = if embed_only {
                blip_embed(pic)
            } else {
                blip_rid(pic)
            };
        }
        Ok(())
    });
    rid
}

fn blip_embed(pic: &Element) -> Option<String> {
    let mut found = None;
    walk_all(pic, &mut |e| {
        if e.local_name() == "blip" && found.is_none() {
            found = e.attr_local("embed").map(str::to_string);
        }
    });
    found
}

fn find_anchor_kind(root: &Element, key: &PicKey) -> Option<AnchorKind> {
    let mut index = 0usize;
    let mut kind = None;
    let _ = walk_anchors(root, &mut |anchor, k| {
        if kind.is_some() {
            return Ok(());
        }
        let Some(pic) = direct_pic(anchor) else {
            return Ok(());
        };
        if !has_blip(pic) {
            return Ok(());
        }
        index += 1;
        if key_matches(key, &cnv_of(pic).0, index) {
            kind = Some(k);
        }
        Ok(())
    });
    kind
}

fn key_matches(key: &PicKey, raw: &Option<String>, index: usize) -> bool {
    match key {
        PicKey::Index(n) => index == *n,
        PicKey::Cnv(want) => {
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

fn pics_match_in(el: &Element, key: &PicKey, index: &mut usize) -> bool {
    let mut matched = false;
    let _ = walk_anchors(el, &mut |anchor, _| {
        let Some(pic) = direct_pic(anchor) else {
            return Ok(());
        };
        if !has_blip(pic) {
            return Ok(());
        }
        *index += 1;
        if key_matches(key, &cnv_of(pic).0, *index) {
            matched = true;
        }
        Ok(())
    });
    matched
}

fn remove_anchor(root: &mut Element, key: &PicKey) -> bool {
    remove_anchor_in(root, key, &mut 0)
}

fn remove_anchor_in(el: &mut Element, key: &PicKey, index: &mut usize) -> bool {
    if el.local_name() == "grpSp" {
        return false;
    }
    let mut drop = None;
    for (i, n) in el.children.iter_mut().enumerate() {
        let Node::Element(child) = n else { continue };
        if child.local_name() == "AlternateContent" {
            let branch = child
                .child_by_local("Choice")
                .or_else(|| child.child_by_local("Fallback"));
            if let Some(branch) = branch {
                if pics_match_in(branch, key, index) {
                    drop = Some(i);
                    break;
                }
            }
        } else if AnchorKind::from_local(child.local_name()).is_some() {
            if let Some(pic) = direct_pic(child) {
                if has_blip(pic) {
                    *index += 1;
                    if key_matches(key, &cnv_of(pic).0, *index) {
                        drop = Some(i);
                        break;
                    }
                }
            }
        } else if remove_anchor_in(child, key, index) {
            return true;
        }
    }
    if let Some(i) = drop {
        el.children.remove(i);
        return true;
    }
    false
}

fn replace_anchor(root: &mut Element, key: &PicKey, new_anchor: Element) {
    replace_anchor_in(root, key, &mut 0, new_anchor);
}

fn replace_anchor_in(
    el: &mut Element,
    key: &PicKey,
    index: &mut usize,
    new_anchor: Element,
) -> bool {
    if el.local_name() == "grpSp" {
        return false;
    }
    if el.local_name() == "AlternateContent" {
        if let Some(choice) = el.child_by_local_mut("Choice") {
            return replace_anchor_in(choice, key, index, new_anchor);
        }
        return false;
    }
    for n in el.children.iter_mut() {
        let Node::Element(child) = n else { continue };
        if AnchorKind::from_local(child.local_name()).is_some() {
            if let Some(pic) = direct_pic(child) {
                if has_blip(pic) {
                    *index += 1;
                    if key_matches(key, &cnv_of(pic).0, *index) {
                        *child = new_anchor;
                        return true;
                    }
                }
            }
        } else if replace_anchor_in(child, key, index, new_anchor.clone()) {
            return true;
        }
    }
    false
}

fn update_extent(root: &mut Element, key: &PicKey, cx: Option<i64>, cy: Option<i64>) -> bool {
    let mut index = 0usize;
    walk_anchors_mut(root, &mut |anchor, _| {
        let Some(pic) = direct_pic(anchor) else {
            return false;
        };
        if !has_blip(pic) {
            return false;
        }
        index += 1;
        if !key_matches(key, &cnv_of(pic).0, index) {
            return false;
        }
        if let Some(ext) = anchor.child_by_local_mut("ext") {
            if let Some(v) = cx {
                set_attr_local(ext, "cx", &v.to_string());
            }
            if let Some(v) = cy {
                set_attr_local(ext, "cy", &v.to_string());
            }
        }
        if let Some(xfrm) = find_local_mut(anchor, "xfrm") {
            if let Some(ext) = xfrm.child_by_local_mut("ext") {
                if let Some(v) = cx {
                    set_attr_local(ext, "cx", &v.to_string());
                }
                if let Some(v) = cy {
                    set_attr_local(ext, "cy", &v.to_string());
                }
            }
        }
        true
    })
}

fn update_absolute_pos(
    root: &mut Element,
    key: &PicKey,
    pos_x: Option<i64>,
    pos_y: Option<i64>,
) -> bool {
    let mut index = 0usize;
    walk_anchors_mut(root, &mut |anchor, _| {
        let Some(pic) = direct_pic(anchor) else {
            return false;
        };
        if !has_blip(pic) {
            return false;
        }
        index += 1;
        if !key_matches(key, &cnv_of(pic).0, index) {
            return false;
        }
        if let Some(pos) = anchor.child_by_local_mut("pos") {
            if let Some(v) = pos_x {
                set_attr_local(pos, "x", &v.to_string());
            }
            if let Some(v) = pos_y {
                set_attr_local(pos, "y", &v.to_string());
            }
        }
        true
    })
}

fn drawing_is_empty(root: &Element) -> bool {
    !root.element_children().any(|c| {
        AnchorKind::from_local(c.local_name()).is_some()
            || c.local_name() == "AlternateContent"
            || c.local_name() == "grpSp"
            || c.local_name() == "sp"
            || c.local_name() == "graphicFrame"
            || c.local_name() == "cxnSp"
    })
}

fn ensure_drawing_part(
    pkg: &mut Package,
    sheet_doc: &mut Document,
    sheet_part: &str,
) -> Result<String, Error> {
    if let Some(part) = drawing_parts(pkg, sheet_part)?.into_iter().next() {
        return Ok(part);
    }
    let part = next_drawing_part(pkg);
    let root = Element::new("xdr:wsDr")
        .with_attr("xmlns:xdr", XDR_NS)
        .with_attr("xmlns:a", A_NS)
        .with_attr("xmlns:r", REL_NS);
    let doc = Document {
        decl: Some(DEFAULT_DECL.to_string()),
        root,
    };
    pkg.set(&part, doc.to_bytes()?)?;
    let mut rels = Relationships::from_package(pkg, sheet_part)?;
    let rid = rels.next_rid();
    rels.push(Relationship {
        id: rid.clone(),
        rel_type: REL_DRAWING.into(),
        target: rel_target_from(sheet_part, &part),
        target_mode: None,
        extra: Vec::new(),
    });
    rels.write_to(pkg, sheet_part)?;
    ensure_xmlns(&mut sheet_doc.root, "r", REL_NS);
    insert_drawing_el(&mut sheet_doc.root, &rid);
    let mut cts = ContentTypes::from_package(pkg)?;
    cts.set_override(&part, CT_DRAWING)?;
    cts.write_to(pkg)?;
    Ok(part)
}

fn insert_drawing_el(worksheet: &mut Element, rid: &str) {
    if worksheet
        .element_children()
        .any(|e| e.local_name() == "drawing")
    {
        return;
    }
    let after = [
        "legacyDrawing",
        "legacyDrawingHF",
        "picture",
        "oleObjects",
        "controls",
        "webPublishItems",
        "tableParts",
        "extLst",
    ];
    let mut at = worksheet.children.len();
    for (i, n) in worksheet.children.iter().enumerate() {
        let Node::Element(e) = n else { continue };
        if after.contains(&e.local_name()) {
            at = i;
            break;
        }
    }
    worksheet.children.insert(
        at,
        Node::Element(Element::new("drawing").with_attr("r:id", rid)),
    );
}

fn remove_drawing_ref(
    worksheet: &mut Element,
    sheet_part: &str,
    pkg: &mut Package,
    drawing_part: &str,
) -> Result<(), Error> {
    worksheet.children.retain(|n| match n {
        Node::Element(e) => e.local_name() != "drawing",
        _ => true,
    });
    let mut rels = Relationships::from_package(pkg, sheet_part)?;
    rels.remove_by_target_part(sheet_part, drawing_part);
    rels.write_to(pkg, sheet_part)?;
    Ok(())
}

fn next_drawing_part(pkg: &Package) -> String {
    let mut n = 1u32;
    loop {
        let name = format!("/xl/drawings/drawing{n}.xml");
        if !pkg.contains(&name) {
            return name;
        }
        n += 1;
    }
}

fn next_cnv_id(pkg: &Package, sheet_part: &str) -> Result<u32, Error> {
    let mut max = 0u32;
    for part in drawing_parts(pkg, sheet_part)? {
        let doc = xml::parse(pkg.get(&part)?)?;
        walk_all(&doc.root, &mut |e| {
            if e.local_name() == "cNvPr" {
                if let Some(id) = e.attr_local("id") {
                    if let Ok(n) = id.parse::<u32>() {
                        max = max.max(n);
                    }
                }
            }
        });
    }
    Ok(max + 1.max(1))
}

fn next_media_part(pkg: &Package, ext: &str) -> String {
    let mut n = 1u32;
    loop {
        let name = format!("/xl/media/image{n}.{ext}");
        if !pkg.contains(&name) {
            return name;
        }
        n += 1;
    }
}

fn rel_target_from(source_part: &str, target_opc: &str) -> String {
    let src_dir = split_dir_file(source_part).0;
    let tgt = target_opc.trim_start_matches('/');
    let src = src_dir.trim_start_matches('/');
    if src.is_empty() || src == "/" {
        return tgt.to_string();
    }
    if let Some(rest) = tgt.strip_prefix(&format!("{src}/")) {
        return rest.to_string();
    }
    let src_parts: Vec<&str> = src.split('/').filter(|s| !s.is_empty()).collect();
    let tgt_parts: Vec<&str> = tgt.split('/').filter(|s| !s.is_empty()).collect();
    let mut common = 0usize;
    while common < src_parts.len()
        && common < tgt_parts.len()
        && src_parts[common] == tgt_parts[common]
    {
        common += 1;
    }
    let up = src_parts.len() - common;
    let rest = tgt_parts[common..].join("/");
    format!("{}{rest}", "../".repeat(up))
}

fn add_image_rel(pkg: &mut Package, part: &str, media_opc: &str) -> Result<String, Error> {
    let mut rels = Relationships::from_package(pkg, part)?;
    let rid = rels.next_rid();
    rels.push(Relationship {
        id: rid.clone(),
        rel_type: REL_IMAGE.into(),
        target: rel_target_from(part, media_opc),
        target_mode: None,
        extra: Vec::new(),
    });
    rels.write_to(pkg, part)?;
    Ok(rid)
}

fn update_rel_target(
    pkg: &mut Package,
    part: &str,
    rid: &str,
    new_target_opc: &str,
) -> Result<(), Error> {
    let mut rels = Relationships::from_package(pkg, part)?;
    if let Some(rel) = rels.rels.iter_mut().find(|r| r.id == rid) {
        rel.target = rel_target_from(part, new_target_opc);
    }
    rels.write_to(pkg, part)
}

fn media_still_used(pkg: &Package, media_opc: &str) -> Result<bool, Error> {
    for name in pkg.part_names() {
        if !name.ends_with(".rels") {
            continue;
        }
        let Some(source) = source_from_rels(&name) else {
            continue;
        };
        let rels = Relationships::parse(pkg.get(&name)?)?;
        for r in &rels.rels {
            if r.is_external() {
                continue;
            }
            if resolve_rel_target(&source, &r.target).ok().as_deref() == Some(media_opc) {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn source_from_rels(rels_part: &str) -> Option<String> {
    // /xl/drawings/_rels/drawing1.xml.rels → /xl/drawings/drawing1.xml
    let trimmed = rels_part.strip_suffix(".rels")?;
    let (dir, file) = split_dir_file(trimmed);
    let parent = dir.trim_end_matches("/_rels").trim_end_matches("_rels");
    if parent.is_empty() || parent == "/" {
        Some(format!("/{file}"))
    } else {
        Some(format!("{parent}/{file}"))
    }
}

fn ensure_xmlns(root: &mut Element, prefix: &str, ns: &str) {
    let key = format!("xmlns:{prefix}");
    if root.attr(&key).is_none() && root.attr("xmlns") != Some(ns) {
        root.set_attr(&key, ns);
    }
}

fn set_attr_local(el: &mut Element, local: &str, value: &str) {
    if let Some((k, _)) = el.attrs.iter().find(|(k, _)| {
        k.rsplit(':').next().unwrap_or(k) == local
    }) {
        let name = k.clone();
        el.set_attr(&name, value);
        return;
    }
    el.set_attr(local, value);
}

fn find_local_mut<'a>(el: &'a mut Element, local: &str) -> Option<&'a mut Element> {
    if el.local_name() == local {
        return Some(el);
    }
    let idx = el.children.iter().position(|n| match n {
        Node::Element(c) => contains_local(c, local),
        _ => false,
    })?;
    let Node::Element(child) = &mut el.children[idx] else {
        return None;
    };
    find_local_mut(child, local)
}

fn contains_local(el: &Element, local: &str) -> bool {
    el.local_name() == local || el.element_children().any(|c| contains_local(c, local))
}

fn build_anchor(
    kind: AnchorKind,
    from: a1::CellRef,
    to: Option<a1::CellRef>,
    from_off_x: i64,
    from_off_y: i64,
    to_off_x: i64,
    to_off_y: i64,
    pos_x: Option<i64>,
    pos_y: Option<i64>,
    cx: i64,
    cy: i64,
    edit_as: EditAs,
    cnv: u32,
    name: &str,
    rid: &str,
    existing_pic: Option<Element>,
) -> Element {
    let mut anchor = Element::new(format!("xdr:{}", kind.local_name()));
    if kind == AnchorKind::TwoCell {
        anchor.set_attr("editAs", edit_as.as_str());
    }
    match kind {
        AnchorKind::TwoCell => {
            anchor.push_element(marker_el("xdr:from", from, from_off_x, from_off_y));
            anchor.push_element(marker_el(
                "xdr:to",
                to.unwrap_or(from),
                to_off_x,
                to_off_y,
            ));
        }
        AnchorKind::OneCell => {
            anchor.push_element(marker_el("xdr:from", from, from_off_x, from_off_y));
            anchor.push_element(
                Element::new("xdr:ext")
                    .with_attr("cx", cx.to_string())
                    .with_attr("cy", cy.to_string()),
            );
        }
        AnchorKind::Absolute => {
            anchor.push_element(
                Element::new("xdr:pos")
                    .with_attr("x", pos_x.unwrap_or(0).to_string())
                    .with_attr("y", pos_y.unwrap_or(0).to_string()),
            );
            anchor.push_element(
                Element::new("xdr:ext")
                    .with_attr("cx", cx.to_string())
                    .with_attr("cy", cy.to_string()),
            );
        }
    }
    let pic = match existing_pic {
        Some(mut pic) => {
            if kind != AnchorKind::TwoCell {
                set_pic_ext(&mut pic, cx, cy);
            }
            pic
        }
        None => build_pic(cnv, name, rid, cx, cy),
    };
    anchor.push_element(pic);
    anchor.push_element(Element::new("xdr:clientData"));
    anchor
}

fn set_pic_ext(pic: &mut Element, cx: i64, cy: i64) {
    if let Some(xfrm) = find_local_mut(pic, "xfrm") {
        if let Some(ext) = xfrm.child_by_local_mut("ext") {
            set_attr_local(ext, "cx", &cx.to_string());
            set_attr_local(ext, "cy", &cy.to_string());
        }
    }
}

fn marker_el(name: &str, cell: a1::CellRef, off_x: i64, off_y: i64) -> Element {
    let mut el = Element::new(name);
    el.push_element(Element::new("xdr:col").with_text((cell.col.saturating_sub(1)).to_string()));
    el.push_element(Element::new("xdr:colOff").with_text(off_x.to_string()));
    el.push_element(Element::new("xdr:row").with_text((cell.row.saturating_sub(1)).to_string()));
    el.push_element(Element::new("xdr:rowOff").with_text(off_y.to_string()));
    el
}

fn build_pic(cnv: u32, name: &str, rid: &str, cx: i64, cy: i64) -> Element {
    let mut nv = Element::new("xdr:nvPicPr");
    nv.push_element(
        Element::new("xdr:cNvPr")
            .with_attr("id", cnv.to_string())
            .with_attr("name", name),
    );
    let mut cnv_pic = Element::new("xdr:cNvPicPr");
    cnv_pic.push_element(Element::new("a:picLocks").with_attr("noChangeAspect", "1"));
    nv.push_element(cnv_pic);
    let mut blip_fill = Element::new("xdr:blipFill");
    blip_fill.push_element(
        Element::new("a:blip")
            .with_attr("xmlns:r", REL_NS)
            .with_attr("r:embed", rid),
    );
    let mut stretch = Element::new("a:stretch");
    stretch.push_element(Element::new("a:fillRect"));
    blip_fill.push_element(stretch);
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
    let mut pic = Element::new("xdr:pic");
    pic.push_element(nv);
    pic.push_element(blip_fill);
    pic.push_element(sp_pr);
    pic
}

fn read_image(path: &str) -> Result<ImageFile, Error> {
    let p = PathBuf::from(path);
    let bytes = std::fs::read(&p).map_err(|e| io_path(e, p.clone()))?;
    classify_image(bytes, &p)
}

fn classify_image(bytes: Vec<u8>, path: &Path) -> Result<ImageFile, Error> {
    if bytes.len() >= 8 && bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) {
        let (cx, cy) = png_size(&bytes).unwrap_or((914400, 914400));
        return Ok(ImageFile {
            bytes,
            ext: "png".into(),
            content_type: "image/png".into(),
            cx,
            cy,
        });
    }
    if bytes.len() >= 3 && bytes[0] == 0xFF && bytes[1] == 0xD8 && bytes[2] == 0xFF {
        let (cx, cy) = jpeg_size(&bytes).unwrap_or((914400, 914400));
        return Ok(ImageFile {
            bytes,
            ext: "jpeg".into(),
            content_type: "image/jpeg".into(),
            cx,
            cy,
        });
    }
    if bytes.len() >= 6 && (bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a")) {
        let (cx, cy) = gif_size(&bytes).unwrap_or((914400, 914400));
        return Ok(ImageFile {
            bytes,
            ext: "gif".into(),
            content_type: "image/gif".into(),
            cx,
            cy,
        });
    }
    if bytes.len() >= 26 && bytes.starts_with(b"BM") {
        let (cx, cy) = bmp_size(&bytes).unwrap_or((914400, 914400));
        return Ok(ImageFile {
            bytes,
            ext: "bmp".into(),
            content_type: "image/bmp".into(),
            cx,
            cy,
        });
    }
    let _ = path;
    Err(Error::new(ErrorKind::InvalidImageFile))
}

fn px_to_emu(px: u32) -> i64 {
    (px as i64).saturating_mul(914400) / 96
}

fn png_size(bytes: &[u8]) -> Option<(i64, i64)> {
    if bytes.len() < 24 {
        return None;
    }
    let w = u32::from_be_bytes(bytes[16..20].try_into().ok()?);
    let h = u32::from_be_bytes(bytes[20..24].try_into().ok()?);
    Some((px_to_emu(w), px_to_emu(h)))
}

fn gif_size(bytes: &[u8]) -> Option<(i64, i64)> {
    if bytes.len() < 10 {
        return None;
    }
    let w = u16::from_le_bytes(bytes[6..8].try_into().ok()?) as u32;
    let h = u16::from_le_bytes(bytes[8..10].try_into().ok()?) as u32;
    Some((px_to_emu(w), px_to_emu(h)))
}

fn bmp_size(bytes: &[u8]) -> Option<(i64, i64)> {
    if bytes.len() < 26 {
        return None;
    }
    let w = i32::from_le_bytes(bytes[18..22].try_into().ok()?).unsigned_abs();
    let h = i32::from_le_bytes(bytes[22..26].try_into().ok()?).unsigned_abs();
    Some((px_to_emu(w), px_to_emu(h)))
}

fn jpeg_size(bytes: &[u8]) -> Option<(i64, i64)> {
    let mut i = 2usize;
    while i + 9 < bytes.len() {
        if bytes[i] != 0xFF {
            i += 1;
            continue;
        }
        let marker = bytes[i + 1];
        if marker == 0xD8 || marker == 0xD9 || (0xD0..=0xD7).contains(&marker) {
            i += 2;
            continue;
        }
        if i + 3 >= bytes.len() {
            break;
        }
        let len = u16::from_be_bytes([bytes[i + 2], bytes[i + 3]]) as usize;
        if matches!(marker, 0xC0 | 0xC1 | 0xC2) && i + 8 < bytes.len() {
            let h = u16::from_be_bytes([bytes[i + 5], bytes[i + 6]]) as u32;
            let w = u16::from_be_bytes([bytes[i + 7], bytes[i + 8]]) as u32;
            return Some((px_to_emu(w), px_to_emu(h)));
        }
        i += 2 + len;
    }
    None
}
