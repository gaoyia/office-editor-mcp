//! 工作表 SpreadsheetDrawing 图片：收集、换、插、删、改尺寸与锚点。

use serde_json::{json, Map, Value};

use crate::error::{Error, ErrorKind, IdReason};
use crate::opc::{
    rels_part_name, resolve_rel_target, ContentTypes, Package, Relationship, Relationships,
};
use crate::xml::{Document, Element, Node, DEFAULT_DECL};

use super::a1;
use super::hf;
use super::request::{
    DeletePictureOp, HfSlot, InsertPictureOp, ReplacePictureOp, SetPictureAnchorOp,
    SetPictureSizeOp,
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
    RelSize,
    AbsSize,
}

impl AnchorKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TwoCell => "twoCell",
            Self::OneCell => "oneCell",
            Self::Absolute => "absolute",
            Self::RelSize => "relSize",
            Self::AbsSize => "absSize",
        }
    }

    pub fn parse_insert(s: &str) -> Option<Self> {
        match s {
            "oneCell" => Some(Self::OneCell),
            "twoCell" => Some(Self::TwoCell),
            _ => None,
        }
    }

    /// `set*Anchor` 认三种；`insert*` 仍只走 `parse_insert`。
    pub fn parse_set(s: &str) -> Option<Self> {
        match s {
            "oneCell" => Some(Self::OneCell),
            "twoCell" => Some(Self::TwoCell),
            "absolute" => Some(Self::Absolute),
            "relSize" => Some(Self::RelSize),
            _ => None,
        }
    }

    pub(crate) fn from_local(name: &str) -> Option<Self> {
        match name {
            "twoCellAnchor" => Some(Self::TwoCell),
            "oneCellAnchor" => Some(Self::OneCell),
            "absoluteAnchor" => Some(Self::Absolute),
            "relSizeAnchor" => Some(Self::RelSize),
            "absSizeAnchor" => Some(Self::AbsSize),
            _ => None,
        }
    }

    pub(crate) fn local_name(self) -> &'static str {
        match self {
            Self::TwoCell => "twoCellAnchor",
            Self::OneCell => "oneCellAnchor",
            Self::Absolute => "absoluteAnchor",
            Self::RelSize => "relSizeAnchor",
            Self::AbsSize => "absSizeAnchor",
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

/// 规划锚点时占位；`absoluteAnchor` 不写 `xdr:from`。
const PLACEHOLDER_CELL: a1::CellRef = a1::CellRef { col: 1, row: 1 };
const DEFAULT_PIC_CX: i64 = 914_400;
const DEFAULT_PIC_CY: i64 = 914_400;

#[derive(Debug, Clone, Copy)]
pub(crate) struct AnchorGeom {
    pub kind: AnchorKind,
    pub from: Option<a1::CellRef>,
    pub to: Option<a1::CellRef>,
    pub from_off_x: i64,
    pub from_off_y: i64,
    pub to_off_x: i64,
    pub to_off_y: i64,
    pub pos_x: Option<i64>,
    pub pos_y: Option<i64>,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
    pub edit_as: EditAs,
}

pub(crate) struct AnchorPatch {
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

pub(crate) struct PlannedAnchor {
    pub kind: AnchorKind,
    pub from: a1::CellRef,
    pub to: Option<a1::CellRef>,
    pub from_off_x: i64,
    pub from_off_y: i64,
    pub to_off_x: i64,
    pub to_off_y: i64,
    pub pos_x: Option<i64>,
    pub pos_y: Option<i64>,
    pub cx: i64,
    pub cy: i64,
    pub edit_as: EditAs,
    pub pos_only: bool,
}

pub(crate) struct SizePatch {
    pub cx: Option<i64>,
    pub cy: Option<i64>,
    pub to: Option<a1::CellRef>,
    pub to_off_x: Option<i64>,
    pub to_off_y: Option<i64>,
}

pub(crate) enum SizeAction {
    Extent {
        cx: Option<i64>,
        cy: Option<i64>,
    },
    TwoCell {
        to: a1::CellRef,
        to_off_x: Option<i64>,
        to_off_y: Option<i64>,
    },
}

fn op_requires(op: &str, field: &str, path: String) -> Error {
    Error::with_path(
        ErrorKind::OpRequiresField {
            op: op.into(),
            field: field.into(),
        },
        path,
    )
}

fn unknown_field(field: &str, path: String) -> Error {
    Error::with_path(
        ErrorKind::UnknownField {
            field: field.into(),
        },
        path,
    )
}

/// 三种锚点互转。不算列宽行高；缺字段就拒绝。
pub(crate) fn plan_anchor(
    geom: &AnchorGeom,
    patch: &AnchorPatch,
    op: &str,
    op_index: usize,
    default_cx: i64,
    default_cy: i64,
) -> Result<PlannedAnchor, Error> {
    let target = patch.anchor.unwrap_or(geom.kind);
    let converting = target != geom.kind;
    if geom.kind == AnchorKind::Absolute && !converting {
        if patch.from.is_some() || patch.to.is_some() {
            return Err(op_requires(op, "anchor", format!("ops[{op_index}].anchor")));
        }
        if patch.cx.is_some() {
            return Err(unknown_field("cx", format!("ops[{op_index}].cx")));
        }
        if patch.cy.is_some() {
            return Err(unknown_field("cy", format!("ops[{op_index}].cy")));
        }
        if patch.from_off_x.is_some() {
            return Err(unknown_field(
                "fromOffX",
                format!("ops[{op_index}].fromOffX"),
            ));
        }
        if patch.from_off_y.is_some() {
            return Err(unknown_field(
                "fromOffY",
                format!("ops[{op_index}].fromOffY"),
            ));
        }
        if patch.to_off_x.is_some() {
            return Err(unknown_field("toOffX", format!("ops[{op_index}].toOffX")));
        }
        if patch.to_off_y.is_some() {
            return Err(unknown_field("toOffY", format!("ops[{op_index}].toOffY")));
        }
        if patch.edit_as.is_some() {
            return Err(unknown_field("editAs", format!("ops[{op_index}].editAs")));
        }
        return Ok(PlannedAnchor {
            kind: AnchorKind::Absolute,
            from: PLACEHOLDER_CELL,
            to: None,
            from_off_x: 0,
            from_off_y: 0,
            to_off_x: 0,
            to_off_y: 0,
            pos_x: patch.pos_x,
            pos_y: patch.pos_y,
            cx: default_cx,
            cy: default_cy,
            edit_as: geom.edit_as,
            pos_only: true,
        });
    }
    if target != AnchorKind::Absolute && (patch.pos_x.is_some() || patch.pos_y.is_some()) {
        return Err(op_requires(op, "anchor", format!("ops[{op_index}].anchor")));
    }
    if target == AnchorKind::Absolute
        && converting
        && (patch.pos_x.is_none() || patch.pos_y.is_none())
    {
        return Err(op_requires(op, "posX and posY", format!("ops[{op_index}]")));
    }
    let from = if target == AnchorKind::Absolute {
        geom.from.or(patch.from).unwrap_or(PLACEHOLDER_CELL)
    } else {
        patch
            .from
            .or(geom.from)
            .ok_or_else(|| op_requires(op, "from", format!("ops[{op_index}].from")))?
    };
    let to = if target == AnchorKind::TwoCell {
        let existing = if geom.kind == AnchorKind::TwoCell {
            geom.to
        } else {
            None
        };
        Some(
            patch
                .to
                .or(existing)
                .ok_or_else(|| op_requires(op, "to", format!("ops[{op_index}].to")))?,
        )
    } else {
        None
    };
    let from_off_x = patch.from_off_x.unwrap_or(if geom.from.is_some() {
        geom.from_off_x
    } else {
        0
    });
    let from_off_y = patch.from_off_y.unwrap_or(if geom.from.is_some() {
        geom.from_off_y
    } else {
        0
    });
    let to_off_x = patch
        .to_off_x
        .unwrap_or(if geom.to.is_some() { geom.to_off_x } else { 0 });
    let to_off_y = patch
        .to_off_y
        .unwrap_or(if geom.to.is_some() { geom.to_off_y } else { 0 });
    let (cx, cy) = extent_for(geom, patch, target, op, op_index, default_cx, default_cy)?;
    let pos_x = if target == AnchorKind::Absolute {
        patch.pos_x.or(geom.pos_x)
    } else {
        None
    };
    let pos_y = if target == AnchorKind::Absolute {
        patch.pos_y.or(geom.pos_y)
    } else {
        None
    };
    Ok(PlannedAnchor {
        kind: target,
        from,
        to,
        from_off_x,
        from_off_y,
        to_off_x,
        to_off_y,
        pos_x,
        pos_y,
        cx,
        cy,
        edit_as: patch.edit_as.unwrap_or(geom.edit_as),
        pos_only: false,
    })
}

fn extent_for(
    geom: &AnchorGeom,
    patch: &AnchorPatch,
    target: AnchorKind,
    op: &str,
    op_index: usize,
    default_cx: i64,
    default_cy: i64,
) -> Result<(i64, i64), Error> {
    if target == AnchorKind::TwoCell {
        return Ok((default_cx, default_cy));
    }
    let from_two = geom.kind == AnchorKind::TwoCell;
    let need = || op_requires(op, "cx and cy", format!("ops[{op_index}]"));
    let cx = if from_two {
        patch.cx.ok_or_else(need)?
    } else {
        patch.cx.or(geom.cx).ok_or_else(need)?
    };
    let cy = if from_two {
        patch.cy.ok_or_else(need)?
    } else {
        patch.cy.or(geom.cy).ok_or_else(need)?
    };
    Ok((cx, cy))
}

pub(crate) fn plan_size(
    kind: AnchorKind,
    patch: &SizePatch,
    two_cell: ErrorKind,
    op: &str,
    op_index: usize,
) -> Result<SizeAction, Error> {
    match kind {
        AnchorKind::TwoCell => {
            if patch.cx.is_some() || patch.cy.is_some() {
                return Err(Error::with_path(two_cell, format!("ops[{op_index}].id")));
            }
            let to = patch
                .to
                .ok_or_else(|| op_requires(op, "to", format!("ops[{op_index}].to")))?;
            Ok(SizeAction::TwoCell {
                to,
                to_off_x: patch.to_off_x,
                to_off_y: patch.to_off_y,
            })
        }
        AnchorKind::OneCell | AnchorKind::Absolute => {
            if patch.to.is_some() {
                return Err(unknown_field("to", format!("ops[{op_index}].to")));
            }
            if patch.to_off_x.is_some() {
                return Err(unknown_field("toOffX", format!("ops[{op_index}].toOffX")));
            }
            if patch.to_off_y.is_some() {
                return Err(unknown_field("toOffY", format!("ops[{op_index}].toOffY")));
            }
            if patch.cx.is_none() && patch.cy.is_none() {
                return Err(op_requires(op, "cx or cy", format!("ops[{op_index}]")));
            }
            Ok(SizeAction::Extent {
                cx: patch.cx,
                cy: patch.cy,
            })
        }
        AnchorKind::RelSize | AnchorKind::AbsSize => {
            Err(unknown_field("cx", format!("ops[{op_index}].cx")))
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
    pub hf: Option<HfSlot>,
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
    pub content_type: Option<String>,
    pub hyperlink: Value,
    pub from_x: Option<f64>,
    pub from_y: Option<f64>,
    pub to_x: Option<f64>,
    pub to_y: Option<f64>,
}

pub(crate) use crate::image::{read_image, ImageFile};

pub fn parse_insert_sheet_id(id: &str, op_index: usize) -> Result<(String, Option<HfSlot>), Error> {
    if id.contains("/pic:") {
        return Err(invalid_id(IdReason::InsertNeedsSheet, op_index));
    }
    let Some(name) = id.strip_prefix("sheet:") else {
        return Err(invalid_id(IdReason::InsertNeedsSheet, op_index));
    };
    if name.is_empty() {
        return Err(invalid_id(IdReason::InsertNeedsSheet, op_index));
    }
    if let Some((sheet, rest)) = name.split_once("/hf:") {
        if sheet.is_empty() {
            return Err(invalid_id(IdReason::InsertNeedsSheet, op_index));
        }
        let slot =
            hf::parse_slot(rest).ok_or_else(|| invalid_id(IdReason::InsertNeedsSheet, op_index))?;
        return Ok((sheet.to_string(), Some(slot)));
    }
    Ok((name.to_string(), None))
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
            let (sheet, hf) = if let Some((s, slot_s)) = sheet.split_once("/hf:") {
                if s.is_empty() {
                    return Err(Error::with_path(
                        ErrorKind::InvalidId {
                            reason: IdReason::SheetPicPrefix,
                        },
                        field_path,
                    ));
                }
                let slot = hf::parse_slot(slot_s).ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::InvalidId {
                            reason: IdReason::SheetPicPrefix,
                        },
                        field_path,
                    )
                })?;
                (s.to_string(), Some(slot))
            } else {
                (sheet.to_string(), None)
            };
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
            return Ok(SheetPicId { sheet, hf, key });
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
    Error::with_path(
        ErrorKind::InvalidId { reason },
        format!("ops[{op_index}].id"),
    )
}

pub fn collect_sheet_pictures(
    pkg: &Package,
    sheet_part: &str,
    sheet_name: &str,
) -> Result<Vec<PicInfo>, Error> {
    let mut out = Vec::new();
    let mut index = 0usize;
    let sheet = super::list_pin::parse_part(pkg, sheet_part)?;
    for drawing_part in
        super::chart_drawing::object_drawing_parts_from(pkg, sheet_part, &sheet.root)?
    {
        let doc = super::list_pin::parse_part(pkg, &drawing_part)?;
        let rels = Relationships::from_package(pkg, &drawing_part)?;
        walk_anchors(&doc.root, &mut |anchor, kind| {
            let Some(pic) = direct_pic(anchor) else {
                return Ok(());
            };
            if !has_blip(pic) {
                return Ok(());
            }
            index += 1;
            out.push(pic_info(anchor, kind, pic, sheet_name, index, Some(&rels))?);
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
        AnchorKind::RelSize => {
            m.insert("fromX".into(), json!(p.from_x));
            m.insert("fromY".into(), json!(p.from_y));
            m.insert("toX".into(), json!(p.to_x));
            m.insert("toY".into(), json!(p.to_y));
        }
        AnchorKind::AbsSize => {
            m.insert("fromX".into(), json!(p.from_x));
            m.insert("fromY".into(), json!(p.from_y));
            m.insert("cx".into(), json!(p.cx));
            m.insert("cy".into(), json!(p.cy));
        }
    }
    m.insert("stable".into(), json!(p.stable));
    if let Some(ct) = &p.content_type {
        m.insert("contentType".into(), json!(ct));
    }
    m.insert("hyperlink".into(), p.hyperlink.clone());
    Value::Object(m)
}

pub fn export_media(bytes: &[u8], id: &str) -> Result<super::ExportedMedia, Error> {
    if super::ole::is_ole_id(id) {
        return super::ole::export(bytes, id);
    }
    if super::model3d::looks_like_id(id) {
        return super::model3d::export(bytes, id);
    }
    if super::ink::looks_like_id(id) {
        return super::ink::export(bytes, id);
    }
    if super::omml::looks_like_id(id) {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SheetMathPrefix,
            },
            "id",
        ));
    }
    if super::background::looks_like_id(id) {
        return super::background::export(bytes, id);
    }
    let pkg = Package::open_bytes(bytes.to_vec())?;
    let target = parse_pic_id_at(id, "id")?;
    let sheet_part = lookup_sheet_part(&pkg, &target.sheet, "id")?;
    let drawing_part = find_drawing_for(&pkg, &sheet_part, &target, id, "id")?;
    let doc = super::list_pin::parse_part(&pkg, &drawing_part)?;
    let hf_el = load_hf_el(&pkg, &sheet_part)?;
    let search = PicSearch {
        target: &target,
        hf_el: hf_el.as_ref(),
    };
    let rid = find_export_rid(&doc.root, &search).ok_or_else(|| {
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
    let mut doc = super::list_pin::parse_part(pkg, &drawing_part)?;
    let hf_el = load_hf_el(pkg, sheet_part)?;
    let search = PicSearch {
        target: &op.target,
        hf_el: hf_el.as_ref(),
    };
    let image = read_image(&op.file).map_err(|e| e.at(format!("ops[{op_index}].file")))?;
    let mut xml_dirty = false;
    let mut drop_svg = None;
    let found = map_pic_mut(&mut doc.root, &search, |pic| {
        let old_svg = crate::svg::blip_rid(pic);
        if let Some(svg) = &image.svg {
            if blip_embed(pic).is_none() {
                let png_part = next_media_part(pkg, "png");
                pkg.set(&png_part, crate::ole::PREVIEW_PNG.to_vec())?;
                let rid = add_image_rel(pkg, &drawing_part, &png_part)?;
                let mut cts = ContentTypes::from_package(pkg)?;
                cts.ensure_default("png", "image/png");
                cts.write_to(pkg)?;
                crate::svg::set_blip_embed(pic, &rid);
            }
            write_or_update_svg(pkg, &drawing_part, pic, &old_svg, svg.clone())?;
            xml_dirty = true;
            return Ok(());
        }
        let rid = blip_embed(pic).ok_or_else(|| {
            Error::with_path(
                ErrorKind::PictureNotFound { id: op.id.clone() },
                path.clone(),
            )
        })?;
        let old_media = crate::opc::resolve_id(pkg, &drawing_part, &rid)?;
        let media = next_media_part(pkg, &image.ext);
        pkg.set(&media, image.bytes.clone())?;
        update_rel_target(pkg, &drawing_part, &rid, &media)?;
        let mut cts = ContentTypes::from_package(pkg)?;
        cts.ensure_default(&image.ext, &image.content_type);
        cts.write_to(pkg)?;
        if old_media != media && !media_still_used(pkg, &old_media)? {
            pkg.remove(&old_media);
        }
        if old_svg.is_some() {
            crate::svg::strip_svg_blip(pic);
            xml_dirty = true;
            drop_svg = old_svg;
        }
        Ok(())
    })?;
    if !found {
        return Err(Error::with_path(
            ErrorKind::PictureNotFound { id: op.id.clone() },
            path,
        ));
    }
    if xml_dirty {
        super::list_pin::write_part(pkg, &drawing_part, &doc)?;
    }
    if let Some(rid) = drop_svg {
        drop_rel_if_unused(pkg, &drawing_part, &doc.root, &rid)?;
    }
    Ok(())
}

pub fn insert_picture(
    pkg: &mut Package,
    sheet_doc: &mut Document,
    sheet_part: &str,
    op: &InsertPictureOp,
    file_field: &str,
) -> Result<(), Error> {
    let image = read_image(&op.file).map_err(|e| e.at(file_field))?;
    let (cx, cy) = match (op.cx, op.cy) {
        (Some(x), Some(y)) => (x, y),
        _ => (image.cx, image.cy),
    };
    if let Some(rel) = op.rel {
        let drawing_part =
            super::chart_drawing::ensure_user_shapes(pkg, sheet_part, &sheet_doc.root)?;
        let cnv = next_cnv_id(pkg, sheet_part, &sheet_doc.root)?;
        let (rid, svg_rid) = write_picture_media(pkg, &drawing_part, &image)?;
        let name = format!("图片 {cnv}");
        let mut pic = build_pic(cnv, &name, &rid, cx, cy);
        if let Some(svg) = svg_rid {
            crate::svg::set_svg_blip(&mut pic, &svg);
        }
        super::chart_drawing::push_rel_anchor(pkg, &drawing_part, rel, pic)?;
        return Ok(());
    }
    let drawing_part = ensure_drawing_part(pkg, sheet_doc, sheet_part)?;
    let mut draw_doc = super::list_pin::parse_part(pkg, &drawing_part)?;
    // Excel 存的 wsDr 常常只在 a:blip 上写 xmlns:r，根上没有。新锚点用 r:embed，必须把关系命名空间补到根上。
    ensure_xmlns(&mut draw_doc.root, "r", REL_NS);
    ensure_xmlns(&mut draw_doc.root, "a", A_NS);
    let cnv = next_cnv_id(pkg, sheet_part, &sheet_doc.root)?;
    let (rid, svg_rid) = write_picture_media(pkg, &drawing_part, &image)?;
    let mut cts = ContentTypes::from_package(pkg)?;
    cts.set_override(&drawing_part, CT_DRAWING)?;
    cts.write_to(pkg)?;
    let name = format!("图片 {cnv}");
    let mut anchor = build_anchor(
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
    if let Some(svg) = svg_rid {
        crate::svg::set_svg_blip(&mut anchor, &svg);
    }
    draw_doc.root.push_element(anchor);
    super::list_pin::write_part(pkg, &drawing_part, &draw_doc)?;
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
    let mut doc = super::list_pin::parse_part(pkg, &drawing_part)?;
    let hf_el = load_hf_el(pkg, sheet_part)?;
    let search = PicSearch {
        target: &op.target,
        hf_el: hf_el.as_ref(),
    };
    let cnv_for_hf = match &op.target.key {
        PicKey::Cnv(s) => Some(s.clone()),
        PicKey::Index(_) => None,
    };
    let rids = find_image_rids(&doc.root, &search).ok_or_else(|| {
        Error::with_path(
            ErrorKind::PictureNotFound { id: op.id.clone() },
            path.clone(),
        )
    })?;
    let medias: Vec<String> = rids
        .iter()
        .filter_map(|rid| crate::opc::resolve_id(pkg, &drawing_part, rid).ok())
        .collect();
    let cnv_for_hf = cnv_for_hf.or_else(|| {
        // index 命中时从当前 drawing 再找一次 cNvPr。
        let mut found = None;
        let mut index = 0usize;
        let _ = walk_anchors(&doc.root, &mut |anchor, _| {
            let Some(pic) = direct_pic(anchor) else {
                return Ok(());
            };
            if !has_blip(pic) {
                return Ok(());
            }
            if search.hit(pic, &mut index) {
                found = cnv_of(pic).0;
            }
            Ok(())
        });
        found
    });
    if !remove_anchor(&mut doc.root, &search) {
        return Err(Error::with_path(
            ErrorKind::PictureNotFound { id: op.id.clone() },
            path,
        ));
    }
    if let Some(cnv) = cnv_for_hf {
        hf::clear_cnv_attrs(&mut sheet_doc.root, &cnv);
    }
    if drawing_is_empty(&doc.root) {
        if super::chart_drawing::is_user_shapes(&doc.root) {
            super::chart_drawing::drop_user_shapes(pkg, sheet_part, &drawing_part)?;
        } else {
            pkg.remove(&drawing_part);
            pkg.remove(&rels_part_name(&drawing_part));
            if op.target.hf.is_some() {
                hf::remove_drawing_hf_ref(&mut sheet_doc.root, sheet_part, pkg, &drawing_part)?;
            } else {
                remove_drawing_ref(&mut sheet_doc.root, sheet_part, pkg, &drawing_part)?;
            }
            let mut cts = ContentTypes::from_package(pkg)?;
            cts.remove_override(&drawing_part);
            cts.write_to(pkg)?;
        }
    } else {
        super::list_pin::write_part(pkg, &drawing_part, &doc)?;
        let mut rels = Relationships::from_package(pkg, &drawing_part)?;
        for rid in &rids {
            rels.remove_by_id(rid);
        }
        if rels.rels.is_empty() {
            pkg.remove(&rels_part_name(&drawing_part));
        } else {
            rels.write_to(pkg, &drawing_part)?;
        }
    }
    for media in medias {
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
    let mut doc = super::list_pin::parse_part(pkg, &drawing_part)?;
    let hf_el = load_hf_el(pkg, sheet_part)?;
    let search = PicSearch {
        target: &op.target,
        hf_el: hf_el.as_ref(),
    };
    let kind = find_anchor_kind(&doc.root, &search).ok_or_else(|| {
        Error::with_path(
            ErrorKind::PictureNotFound { id: op.id.clone() },
            path.clone(),
        )
    })?;
    let action = plan_size(
        kind,
        &SizePatch {
            cx: op.cx,
            cy: op.cy,
            to: op.to,
            to_off_x: op.to_off_x,
            to_off_y: op.to_off_y,
        },
        ErrorKind::TwoCellPictureSize,
        "setPictureSize",
        op_index,
    )?;
    let ok = match action {
        SizeAction::Extent { cx, cy } => update_extent(&mut doc.root, &search, cx, cy),
        SizeAction::TwoCell {
            to,
            to_off_x,
            to_off_y,
        } => update_two_cell_to(&mut doc.root, &search, to, to_off_x, to_off_y)?,
    };
    if !ok {
        return Err(Error::with_path(
            ErrorKind::PictureNotFound { id: op.id.clone() },
            path,
        ));
    }
    super::list_pin::write_part(pkg, &drawing_part, &doc)?;
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
    let mut doc = super::list_pin::parse_part(pkg, &drawing_part)?;
    let hf_el = load_hf_el(pkg, sheet_part)?;
    let search = PicSearch {
        target: &op.target,
        hf_el: hf_el.as_ref(),
    };
    let snapshot = snapshot_anchor(&doc.root, &search).ok_or_else(|| {
        Error::with_path(
            ErrorKind::PictureNotFound { id: op.id.clone() },
            path.clone(),
        )
    })?;
    if matches!(snapshot.kind, AnchorKind::RelSize | AnchorKind::AbsSize) {
        if op.anchor.is_some() && op.anchor != Some(snapshot.kind) {
            return Err(unknown_field("anchor", format!("ops[{op_index}].anchor")));
        }
        super::chart_drawing::reject_cell_anchor_fields(
            op.from.is_some(),
            op.to.is_some(),
            op.from_off_x.is_some() || op.from_off_y.is_some(),
            op.to_off_x.is_some() || op.to_off_y.is_some(),
            op_index,
        )?;
        if op.pos_x.is_some() {
            return Err(unknown_field("posX", format!("ops[{op_index}].posX")));
        }
        if op.pos_y.is_some() {
            return Err(unknown_field("posY", format!("ops[{op_index}].posY")));
        }
        let Some(rel) = op.rel else {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "fromX".into(),
                },
                format!("ops[{op_index}].fromX"),
            ));
        };
        let patched = patch_matched_rel(&mut doc.root, &search, rel)?;
        if !patched {
            return Err(Error::with_path(
                ErrorKind::PictureNotFound { id: op.id.clone() },
                path,
            ));
        }
        super::list_pin::write_part(pkg, &drawing_part, &doc)?;
        return Ok(());
    }
    if op.rel.is_some() {
        return Err(unknown_field("fromX", format!("ops[{op_index}].fromX")));
    }
    let planned = plan_anchor(
        &snapshot.geom(),
        &AnchorPatch {
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
        },
        "setPictureAnchor",
        op_index,
        DEFAULT_PIC_CX,
        DEFAULT_PIC_CY,
    )?;
    if planned.pos_only {
        if !update_absolute_pos(&mut doc.root, &search, planned.pos_x, planned.pos_y) {
            return Err(Error::with_path(
                ErrorKind::PictureNotFound { id: op.id.clone() },
                path,
            ));
        }
        super::list_pin::write_part(pkg, &drawing_part, &doc)?;
        return Ok(());
    }
    // 转锚点会重写 xdr:pic；Excel 原 drawing 根上常常没有 xmlns:r。
    ensure_xmlns(&mut doc.root, "r", REL_NS);
    ensure_xmlns(&mut doc.root, "a", A_NS);
    replace_anchor(
        &mut doc.root,
        &search,
        build_anchor(
            planned.kind,
            planned.from,
            planned.to,
            planned.from_off_x,
            planned.from_off_y,
            planned.to_off_x,
            planned.to_off_y,
            planned.pos_x,
            planned.pos_y,
            planned.cx,
            planned.cy,
            planned.edit_as,
            snapshot.cnv,
            &snapshot.name,
            &snapshot.rid,
            Some(snapshot.pic),
        ),
    );
    super::list_pin::write_part(pkg, &drawing_part, &doc)?;
    Ok(())
}

struct AnchorSnap {
    kind: AnchorKind,
    from: Option<a1::CellRef>,
    to: Option<a1::CellRef>,
    from_off_x: i64,
    from_off_y: i64,
    to_off_x: i64,
    to_off_y: i64,
    pos_x: Option<i64>,
    pos_y: Option<i64>,
    cx: Option<i64>,
    cy: Option<i64>,
    edit_as: EditAs,
    cnv: u32,
    name: String,
    rid: String,
    pic: Element,
}

impl AnchorSnap {
    fn geom(&self) -> AnchorGeom {
        AnchorGeom {
            kind: self.kind,
            from: self.from,
            to: self.to,
            from_off_x: self.from_off_x,
            from_off_y: self.from_off_y,
            to_off_x: self.to_off_x,
            to_off_y: self.to_off_y,
            pos_x: self.pos_x,
            pos_y: self.pos_y,
            cx: self.cx,
            cy: self.cy,
            edit_as: self.edit_as,
        }
    }
}

fn snapshot_anchor(root: &Element, search: &PicSearch<'_>) -> Option<AnchorSnap> {
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
        if !search.hit(pic, &mut index) {
            return Ok(());
        }
        let (from, from_off_x, from_off_y) = read_marker(anchor.child_by_local("from"))?;
        let (to, to_off_x, to_off_y) = read_marker(anchor.child_by_local("to"))?;
        let (cx, cy) = read_ext(anchor);
        let (pos_x, pos_y) = read_pos(anchor);
        let rid = blip_rid(pic).unwrap_or_default();
        let (cnv_raw, name) = cnv_of(pic);
        let cnv = cnv_raw.and_then(|s| s.parse().ok()).unwrap_or(1);
        found = Some(AnchorSnap {
            kind,
            from,
            to,
            from_off_x,
            from_off_y,
            to_off_x,
            to_off_y,
            pos_x,
            pos_y,
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

pub(crate) fn lookup_sheet_part(
    pkg: &Package,
    sheet_name: &str,
    field_path: &str,
) -> Result<String, Error> {
    let wb_part = crate::opc::office_document_part(pkg)?;
    let wb = super::list_pin::parse_part(pkg, &wb_part)?;
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

pub(crate) fn drawing_parts(pkg: &Package, sheet_part: &str) -> Result<Vec<String>, Error> {
    let doc = super::list_pin::parse_part(pkg, sheet_part)?;
    drawing_parts_from(pkg, sheet_part, &doc.root)
}

pub(crate) fn drawing_hf_parts(pkg: &Package, sheet_part: &str) -> Result<Vec<String>, Error> {
    let doc = super::list_pin::parse_part(pkg, sheet_part)?;
    drawing_hf_parts_from(pkg, sheet_part, &doc.root)
}

/// 按当前工作表根上的 `<drawing r:id>` 找 SpreadsheetDrawing，不跟 `drawingHF`。
/// apply / create 连续插入时真源是内存里的表 XML，包里那份可能还没写回。
pub(crate) fn drawing_parts_from(
    pkg: &Package,
    sheet_part: &str,
    root: &Element,
) -> Result<Vec<String>, Error> {
    parts_from_root(pkg, sheet_part, root, "drawing")
}

pub(crate) fn drawing_hf_parts_from(
    pkg: &Package,
    sheet_part: &str,
    root: &Element,
) -> Result<Vec<String>, Error> {
    parts_from_root(pkg, sheet_part, root, "drawingHF")
}

fn parts_from_root(
    pkg: &Package,
    sheet_part: &str,
    root: &Element,
    local: &str,
) -> Result<Vec<String>, Error> {
    let rels = Relationships::from_package(pkg, sheet_part)?;
    let mut out = Vec::new();
    for el in root.children_by_local(local) {
        let Some(rid) = el.attr_local("id") else {
            continue;
        };
        let Some(rel) = rels.find_by_id(rid) else {
            continue;
        };
        if rel.is_external() {
            continue;
        }
        if !(rel.rel_type == REL_DRAWING || rel.rel_type.ends_with("/drawing")) {
            continue;
        }
        let part = resolve_rel_target(sheet_part, &rel.target)?;
        if pkg.contains(&part) {
            out.push(part);
        }
    }
    Ok(out)
}

fn load_hf_el(pkg: &Package, sheet_part: &str) -> Result<Option<Element>, Error> {
    let doc = super::list_pin::parse_part(pkg, sheet_part)?;
    Ok(doc.root.child_by_local("drawingHF").cloned())
}

struct PicSearch<'a> {
    target: &'a SheetPicId,
    hf_el: Option<&'a Element>,
}

impl PicSearch<'_> {
    fn hit(&self, pic: &Element, index: &mut usize) -> bool {
        let cnv = cnv_of(pic).0;
        if !self.in_list(&cnv) {
            return false;
        }
        *index += 1;
        match &self.target.key {
            PicKey::Index(n) => super::list_pin::index_matches(pic, *n, *index),
            PicKey::Cnv(_) => key_matches(&self.target.key, &cnv, *index),
        }
    }

    fn in_list(&self, cnv: &Option<String>) -> bool {
        match self.target.hf {
            None => true,
            Some(slot) => hf::slot_for_cnv(self.hf_el, cnv.as_deref()) == slot,
        }
    }
}

pub(crate) fn find_drawing_for(
    pkg: &Package,
    sheet_part: &str,
    target: &SheetPicId,
    id: &str,
    field_path: &str,
) -> Result<String, Error> {
    let hf_el = load_hf_el(pkg, sheet_part)?;
    let search = PicSearch {
        target,
        hf_el: hf_el.as_ref(),
    };
    let parts = if target.hf.is_some() {
        drawing_hf_parts(pkg, sheet_part)?
    } else {
        let sheet = super::list_pin::parse_part(pkg, sheet_part)?;
        super::chart_drawing::object_drawing_parts_from(pkg, sheet_part, &sheet.root)?
    };
    for part in parts {
        let doc = super::list_pin::parse_part(pkg, &part)?;
        if find_blip_rid(&doc.root, &search).is_some() {
            return Ok(part);
        }
    }
    Err(Error::with_path(
        ErrorKind::PictureNotFound { id: id.to_string() },
        field_path,
    ))
}

pub(crate) fn find_pic_element_mut<'a>(
    root: &'a mut Element,
    target: &SheetPicId,
    hf_el: Option<&Element>,
) -> Option<&'a mut Element> {
    let search = PicSearch { target, hf_el };
    find_pic_mut_in(root, &search, &mut 0)
}

fn find_pic_mut_in<'a>(
    el: &'a mut Element,
    search: &PicSearch<'_>,
    index: &mut usize,
) -> Option<&'a mut Element> {
    crate::xml::search_mut(el, |el| {
        if el.local_name() == "AlternateContent" {
            if let Some(choice) = crate::xml::alternate_child(el).unwrap_or(None) {
                if matches!(&el.children[choice], Node::Element(_)) {
                    return crate::xml::SearchMut::Only(choice);
                }
            }
            return crate::xml::SearchMut::Skip;
        }
        if AnchorKind::from_local(el.local_name()).is_some() {
            let pic_i = el.children.iter().position(
                |n| matches!(n, Node::Element(e) if e.local_name() == "pic" && has_blip(e)),
            );
            if let Some(pic_i) = pic_i {
                let matched = if let Node::Element(pic) = &el.children[pic_i] {
                    search.hit(pic, index)
                } else {
                    false
                };
                if matched {
                    return crate::xml::SearchMut::HitChild(pic_i);
                }
            }
            return crate::xml::SearchMut::Skip;
        }
        if el.local_name() == "grpSp" {
            return crate::xml::SearchMut::Skip;
        }
        crate::xml::SearchMut::down(el)
    })
}

fn patch_matched_rel(
    root: &mut Element,
    search: &PicSearch<'_>,
    rel: super::request::RelBoxPatch,
) -> Result<bool, Error> {
    let mut index = 0usize;
    let mut hit = false;
    walk_anchors_mut(root, &mut |anchor, _| {
        let matched = {
            let Some(pic) = direct_pic(anchor) else {
                return Ok(false);
            };
            if !has_blip(pic) {
                return Ok(false);
            }
            search.hit(pic, &mut index)
        };
        if matched {
            let _ = super::chart_drawing::patch_rel_box(anchor, rel);
            hit = true;
            Ok(true)
        } else {
            Ok(false)
        }
    })?;
    Ok(hit)
}

pub(crate) fn walk_anchors<F>(root: &Element, f: &mut F) -> Result<(), Error>
where
    F: FnMut(&Element, AnchorKind) -> Result<(), Error>,
{
    walk_anchor_nodes(root, f)
}

fn walk_anchor_nodes<F>(root: &Element, f: &mut F) -> Result<(), Error>
where
    F: FnMut(&Element, AnchorKind) -> Result<(), Error>,
{
    let mut stack = vec![root];
    while let Some(el) = stack.pop() {
        if el.local_name() == "AlternateContent" {
            if let Some(index) = crate::xml::alternate_child(el)? {
                if let Node::Element(child) = &el.children[index] {
                    stack.push(child);
                }
            }
            continue;
        }
        if let Some(kind) = AnchorKind::from_local(el.local_name()) {
            f(el, kind)?;
            continue;
        }
        if el.local_name() == "grpSp" {
            continue;
        }
        for child in el.children.iter().rev() {
            if let Node::Element(inner) = child {
                stack.push(inner);
            }
        }
    }
    Ok(())
}

pub(crate) fn walk_anchors_mut<F>(root: &mut Element, f: &mut F) -> Result<bool, Error>
where
    F: FnMut(&mut Element, AnchorKind) -> Result<bool, Error>,
{
    let mut stack = vec![Vec::<usize>::new()];
    while let Some(path) = stack.pop() {
        let local = element_at(root, &path).local_name().to_string();
        if local == "AlternateContent" {
            let index = {
                let el = element_at(root, &path);
                crate::xml::alternate_child(el)?
            };
            if let Some(index) = index {
                let mut child = path;
                child.push(index);
                stack.push(child);
            }
            continue;
        }
        if let Some(kind) = AnchorKind::from_local(&local) {
            if f(element_at(root, &path), kind)? {
                return Ok(true);
            }
            continue;
        }
        if local == "grpSp" {
            continue;
        }
        let indexes: Vec<usize> = element_at(root, &path)
            .children
            .iter()
            .enumerate()
            .filter_map(|(index, node)| matches!(node, Node::Element(_)).then_some(index))
            .collect();
        for index in indexes.into_iter().rev() {
            let mut child = path.clone();
            child.push(index);
            stack.push(child);
        }
    }
    Ok(false)
}

fn element_at<'a>(root: &'a mut Element, path: &[usize]) -> &'a mut Element {
    let mut cur = root;
    for &index in path {
        cur = match &mut cur.children[index] {
            Node::Element(child) => child,
            Node::Text(_) | Node::Raw(_) => unreachable!("锚点路径只指向元素"),
        };
    }
    cur
}

pub(crate) fn direct_pic(anchor: &Element) -> Option<&Element> {
    anchor
        .element_children()
        .find(|c| c.local_name() == "pic" && has_blip(c))
}

pub(crate) fn has_blip(el: &Element) -> bool {
    let mut found = false;
    walk_all(el, &mut |e| {
        if e.local_name() == "blip" {
            found = true;
        }
    });
    found
}

pub(crate) fn walk_all(el: &Element, f: &mut impl FnMut(&Element)) {
    crate::xml::walk_elements(el, f);
}

fn pic_info(
    anchor: &Element,
    kind: AnchorKind,
    pic: &Element,
    sheet_name: &str,
    index: usize,
    rels: Option<&Relationships>,
) -> Result<PicInfo, Error> {
    pic_info_at(
        anchor,
        kind,
        pic,
        &format!("sheet:{sheet_name}"),
        index,
        rels,
    )
}

pub(crate) fn pic_info_at(
    anchor: &Element,
    kind: AnchorKind,
    pic: &Element,
    id_base: &str,
    index: usize,
    rels: Option<&Relationships>,
) -> Result<PicInfo, Error> {
    let (raw_id, name) = cnv_of(pic);
    let stable = raw_id.is_some();
    let id = match &raw_id {
        Some(v) => format!("{id_base}/pic:{v}"),
        None => format!("{id_base}/pic:index:{index}"),
    };
    let (from, from_off_x, from_off_y) = read_marker(anchor.child_by_local("from"))?;
    let (to, to_off_x, to_off_y) = read_marker(anchor.child_by_local("to"))?;
    let (cx, cy) = read_ext(anchor);
    let (pos_x, pos_y) = read_pos(anchor);
    let (from_x, from_y, to_x, to_y) = super::chart_drawing::read_rel_box(anchor);
    let cell = matches!(kind, AnchorKind::TwoCell | AnchorKind::OneCell);
    Ok(PicInfo {
        id,
        cnv_pr_id: raw_id,
        name,
        anchor: kind,
        from: if cell {
            from.map(|c| a1::format_cell(c.col, c.row))
        } else {
            None
        },
        from_off_x: if cell { Some(from_off_x) } else { None },
        from_off_y: if cell { Some(from_off_y) } else { None },
        to: if kind == AnchorKind::TwoCell {
            to.map(|c| a1::format_cell(c.col, c.row))
        } else {
            None
        },
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
        cx: if matches!(kind, AnchorKind::TwoCell | AnchorKind::RelSize) {
            None
        } else {
            cx
        },
        cy: if matches!(kind, AnchorKind::TwoCell | AnchorKind::RelSize) {
            None
        } else {
            cy
        },
        edit_as: if kind == AnchorKind::TwoCell {
            Some(parse_edit_as(anchor.attr("editAs")))
        } else {
            None
        },
        stable,
        content_type: crate::image::inspect_content_type(
            crate::svg::content_type(pic).is_some(),
            blip_embed(pic).as_deref(),
            rels,
        ),
        hyperlink: super::hyperlink::inspect_cnv(pic, rels),
        from_x: if matches!(kind, AnchorKind::RelSize | AnchorKind::AbsSize) {
            from_x
        } else {
            None
        },
        from_y: if matches!(kind, AnchorKind::RelSize | AnchorKind::AbsSize) {
            from_y
        } else {
            None
        },
        to_x: if kind == AnchorKind::RelSize {
            to_x
        } else {
            None
        },
        to_y: if kind == AnchorKind::RelSize {
            to_y
        } else {
            None
        },
    })
}

pub(crate) fn cnv_of(pic: &Element) -> (Option<String>, String) {
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

pub(crate) fn read_marker(el: Option<&Element>) -> Result<(Option<a1::CellRef>, i64, i64), Error> {
    let Some(el) = el else {
        return Ok((None, 0, 0));
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
        col: marker_index(col, i64::from(super::layout::MAX_COL))?,
        row: marker_index(row, i64::from(super::layout::MAX_ROW))?,
    };
    Ok((Some(cell), col_off, row_off))
}

fn marker_index(raw: i64, max: i64) -> Result<u32, Error> {
    let Some(next) = raw.checked_add(1) else {
        return Err(Error::xml(crate::error::XmlIssue::Parse));
    };
    let n = next.max(1);
    if n > max {
        return Err(Error::xml(crate::error::XmlIssue::Parse));
    }
    Ok(n as u32)
}

pub(crate) fn read_ext(anchor: &Element) -> (Option<i64>, Option<i64>) {
    let ext = anchor.child_by_local("ext");
    let cx = ext
        .and_then(|e| e.attr_local("cx"))
        .and_then(|s| s.parse().ok());
    let cy = ext
        .and_then(|e| e.attr_local("cy"))
        .and_then(|s| s.parse().ok());
    (cx, cy)
}

pub(crate) fn read_pos(anchor: &Element) -> (Option<i64>, Option<i64>) {
    let pos = anchor.child_by_local("pos");
    let x = pos
        .and_then(|e| e.attr_local("x"))
        .and_then(|s| s.parse().ok());
    let y = pos
        .and_then(|e| e.attr_local("y"))
        .and_then(|s| s.parse().ok());
    (x, y)
}

pub(crate) fn parse_edit_as(s: Option<&str>) -> EditAs {
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

fn find_blip_rid(root: &Element, search: &PicSearch<'_>) -> Option<String> {
    find_blip_attr(root, search, false)
}

fn find_export_rid(root: &Element, search: &PicSearch<'_>) -> Option<String> {
    let mut index = 0usize;
    let mut rid = None;
    let _ = walk_anchors(root, &mut |anchor, _| {
        if rid.is_some() {
            return Ok(());
        }
        let Some(pic) = direct_pic(anchor) else {
            return Ok(());
        };
        if search.hit(pic, &mut index) {
            rid = crate::svg::blip_rid(pic).or_else(|| blip_embed(pic));
        }
        Ok(())
    });
    rid
}

fn find_image_rids(root: &Element, search: &PicSearch<'_>) -> Option<Vec<String>> {
    let mut index = 0usize;
    let mut rids = None;
    let _ = walk_anchors(root, &mut |anchor, _| {
        if rids.is_some() {
            return Ok(());
        }
        let Some(pic) = direct_pic(anchor) else {
            return Ok(());
        };
        if search.hit(pic, &mut index) {
            rids = Some(image_rids(pic));
        }
        Ok(())
    });
    rids
}

fn image_rids(pic: &Element) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(r) = blip_embed(pic).or_else(|| blip_rid(pic)) {
        out.push(r);
    }
    if let Some(r) = crate::svg::blip_rid(pic) {
        if !out.iter().any(|x| x == &r) {
            out.push(r);
        }
    }
    out
}

fn map_pic_mut<F>(root: &mut Element, search: &PicSearch<'_>, mut f: F) -> Result<bool, Error>
where
    F: FnMut(&mut Element) -> Result<(), Error>,
{
    let mut index = 0usize;
    let mut found = false;
    walk_anchors_mut(root, &mut |anchor, _| {
        if found {
            return Ok(true);
        }
        let Some(pic) = direct_pic_mut(anchor) else {
            return Ok(false);
        };
        if !has_blip(pic) {
            return Ok(false);
        }
        if search.hit(pic, &mut index) {
            found = true;
            f(pic)?;
            return Ok(true);
        }
        Ok(false)
    })?;
    Ok(found)
}

fn direct_pic_mut(anchor: &mut Element) -> Option<&mut Element> {
    let idx = anchor.children.iter().position(|n| match n {
        Node::Element(c) => c.local_name() == "pic" && has_blip(c),
        _ => false,
    })?;
    match &mut anchor.children[idx] {
        Node::Element(pic) => Some(pic),
        _ => None,
    }
}

fn find_blip_attr(root: &Element, search: &PicSearch<'_>, embed_only: bool) -> Option<String> {
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
        if search.hit(pic, &mut index) {
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

fn find_anchor_kind(root: &Element, search: &PicSearch<'_>) -> Option<AnchorKind> {
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
        if search.hit(pic, &mut index) {
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

fn pics_match_in(el: &Element, search: &PicSearch<'_>, index: &mut usize) -> bool {
    let mut matched = false;
    let _ = walk_anchors(el, &mut |anchor, _| {
        let Some(pic) = direct_pic(anchor) else {
            return Ok(());
        };
        if !has_blip(pic) {
            return Ok(());
        }
        if search.hit(pic, index) {
            matched = true;
        }
        Ok(())
    });
    matched
}

fn remove_anchor(root: &mut Element, search: &PicSearch<'_>) -> bool {
    remove_anchor_in(root, search, &mut 0)
}

fn remove_anchor_in(el: &mut Element, search: &PicSearch<'_>, index: &mut usize) -> bool {
    if el.local_name() == "grpSp" {
        return false;
    }
    let mut drop = None;
    for (i, n) in el.children.iter_mut().enumerate() {
        let Node::Element(child) = n else { continue };
        if child.local_name() == "AlternateContent" {
            let branch = crate::xml::alternate_child(child)
                .unwrap_or(None)
                .and_then(|index| match &child.children[index] {
                    Node::Element(branch) => Some(branch),
                    Node::Text(_) | Node::Raw(_) => None,
                });
            if let Some(branch) = branch {
                if pics_match_in(branch, search, index) {
                    drop = Some(i);
                    break;
                }
            }
        } else if AnchorKind::from_local(child.local_name()).is_some() {
            if let Some(pic) = direct_pic(child) {
                if has_blip(pic) && search.hit(pic, index) {
                    drop = Some(i);
                    break;
                }
            }
        } else if remove_anchor_in(child, search, index) {
            return true;
        }
    }
    if let Some(i) = drop {
        el.children.remove(i);
        return true;
    }
    false
}

fn replace_anchor(root: &mut Element, search: &PicSearch<'_>, new_anchor: Element) {
    replace_anchor_in(root, search, &mut 0, new_anchor);
}

fn replace_anchor_in(
    el: &mut Element,
    search: &PicSearch<'_>,
    index: &mut usize,
    new_anchor: Element,
) -> bool {
    if el.local_name() == "grpSp" {
        return false;
    }
    if el.local_name() == "AlternateContent" {
        if let Some(choice) = el.child_by_local_mut("Choice") {
            return replace_anchor_in(choice, search, index, new_anchor);
        }
        return false;
    }
    for n in el.children.iter_mut() {
        let Node::Element(child) = n else { continue };
        if AnchorKind::from_local(child.local_name()).is_some() {
            if let Some(pic) = direct_pic(child) {
                if has_blip(pic) && search.hit(pic, index) {
                    *child = new_anchor;
                    return true;
                }
            }
        } else if replace_anchor_in(child, search, index, new_anchor.clone()) {
            return true;
        }
    }
    false
}

fn update_extent(
    root: &mut Element,
    search: &PicSearch<'_>,
    cx: Option<i64>,
    cy: Option<i64>,
) -> bool {
    let mut index = 0usize;
    walk_anchors_mut(root, &mut |anchor, _| {
        let Some(pic) = direct_pic(anchor) else {
            return Ok(false);
        };
        if !has_blip(pic) {
            return Ok(false);
        }
        if !search.hit(pic, &mut index) {
            return Ok(false);
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
        Ok(true)
    })
    .unwrap_or(false)
}

fn update_absolute_pos(
    root: &mut Element,
    search: &PicSearch<'_>,
    pos_x: Option<i64>,
    pos_y: Option<i64>,
) -> bool {
    let mut index = 0usize;
    walk_anchors_mut(root, &mut |anchor, _| {
        let Some(pic) = direct_pic(anchor) else {
            return Ok(false);
        };
        if !has_blip(pic) {
            return Ok(false);
        }
        if !search.hit(pic, &mut index) {
            return Ok(false);
        }
        if let Some(pos) = anchor.child_by_local_mut("pos") {
            if let Some(v) = pos_x {
                set_attr_local(pos, "x", &v.to_string());
            }
            if let Some(v) = pos_y {
                set_attr_local(pos, "y", &v.to_string());
            }
        }
        Ok(true)
    })
    .unwrap_or(false)
}

fn update_two_cell_to(
    root: &mut Element,
    search: &PicSearch<'_>,
    to: a1::CellRef,
    to_off_x: Option<i64>,
    to_off_y: Option<i64>,
) -> Result<bool, Error> {
    let mut index = 0usize;
    walk_anchors_mut(root, &mut |anchor, _| {
        let Some(pic) = direct_pic(anchor) else {
            return Ok(false);
        };
        if !has_blip(pic) {
            return Ok(false);
        }
        if !search.hit(pic, &mut index) {
            return Ok(false);
        }
        apply_two_cell_to(anchor, to, to_off_x, to_off_y)?;
        Ok(true)
    })
}

pub(crate) fn drawing_is_empty(root: &Element) -> bool {
    !root.element_children().any(|c| {
        AnchorKind::from_local(c.local_name()).is_some()
            || c.local_name() == "AlternateContent"
            || c.local_name() == "grpSp"
            || c.local_name() == "sp"
            || c.local_name() == "graphicFrame"
            || c.local_name() == "cxnSp"
    })
}

pub(crate) fn ensure_drawing_part(
    pkg: &mut Package,
    sheet_doc: &mut Document,
    sheet_part: &str,
) -> Result<String, Error> {
    if let Some(part) = drawing_parts_from(pkg, sheet_part, &sheet_doc.root)?
        .into_iter()
        .next()
    {
        return Ok(part);
    }
    let part = next_drawing_part(pkg);
    let root = Element::new("xdr:wsDr")
        .with_attr("xmlns:xdr", XDR_NS)
        .with_attr("xmlns:a", A_NS)
        .with_attr("xmlns:r", REL_NS);
    let doc = Document::new(Some(DEFAULT_DECL.to_string()), root);
    super::list_pin::write_part(pkg, &part, &doc)?;
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

pub(crate) fn remove_drawing_ref(
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

pub(crate) fn next_drawing_part(pkg: &Package) -> String {
    let mut n = 1u32;
    loop {
        let name = format!("/xl/drawings/drawing{n}.xml");
        if !pkg.contains(&name) {
            return name;
        }
        n += 1;
    }
}

pub(crate) fn next_cnv_id(
    pkg: &Package,
    sheet_part: &str,
    sheet_root: &Element,
) -> Result<u32, Error> {
    let mut max = 0u32;
    let mut scan = |parts: Vec<String>| -> Result<(), Error> {
        for part in parts {
            let doc = super::list_pin::parse_part(pkg, &part)?;
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
        Ok(())
    };
    scan(drawing_parts_from(pkg, sheet_part, sheet_root)?)?;
    scan(drawing_hf_parts_from(pkg, sheet_part, sheet_root)?)?;
    scan(super::chart_drawing::user_shape_parts(
        pkg, sheet_part, sheet_root,
    )?)?;
    Ok(max + 1)
}

pub(crate) fn next_media_part(pkg: &Package, ext: &str) -> String {
    pkg.first_free(|n| format!("/xl/media/image{n}.{ext}"))
}

pub(crate) fn rel_target_from(source_part: &str, target_opc: &str) -> String {
    crate::opc::rel_target(source_part, target_opc, true)
}

pub(crate) fn add_image_rel(
    pkg: &mut Package,
    part: &str,
    media_opc: &str,
) -> Result<String, Error> {
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

pub(crate) fn write_picture_media(
    pkg: &mut Package,
    part: &str,
    image: &ImageFile,
) -> Result<(String, Option<String>), Error> {
    let media = next_media_part(pkg, &image.ext);
    pkg.set(&media, image.bytes.clone())?;
    let rid = add_image_rel(pkg, part, &media)?;
    let mut cts = ContentTypes::from_package(pkg)?;
    cts.ensure_default(&image.ext, &image.content_type);
    let svg_rid = if let Some(svg) = &image.svg {
        let svg_part = next_media_part(pkg, "svg");
        pkg.set(&svg_part, svg.clone())?;
        let svg_rid = add_image_rel(pkg, part, &svg_part)?;
        cts.ensure_default("svg", crate::svg::CT);
        Some(svg_rid)
    } else {
        None
    };
    cts.write_to(pkg)?;
    Ok((rid, svg_rid))
}

fn write_or_update_svg(
    pkg: &mut Package,
    part: &str,
    host: &mut Element,
    old_svg_rid: &Option<String>,
    svg: Vec<u8>,
) -> Result<(), Error> {
    if let Some(rid) = old_svg_rid {
        let old_media = crate::opc::resolve_id(pkg, part, rid)?;
        if old_media
            .rsplit('.')
            .next()
            .unwrap_or("")
            .eq_ignore_ascii_case("svg")
        {
            pkg.set(&old_media, svg)?;
        } else {
            let new_media = next_media_part(pkg, "svg");
            pkg.set(&new_media, svg)?;
            update_rel_target(pkg, part, rid, &new_media)?;
            if !media_still_used(pkg, &old_media)? {
                pkg.remove(&old_media);
            }
        }
        let mut cts = ContentTypes::from_package(pkg)?;
        cts.ensure_default("svg", crate::svg::CT);
        cts.write_to(pkg)?;
        return Ok(());
    }
    let svg_part = next_media_part(pkg, "svg");
    pkg.set(&svg_part, svg)?;
    let svg_rid = add_image_rel(pkg, part, &svg_part)?;
    let mut cts = ContentTypes::from_package(pkg)?;
    cts.ensure_default("svg", crate::svg::CT);
    cts.write_to(pkg)?;
    crate::svg::set_svg_blip(host, &svg_rid);
    Ok(())
}

fn drop_rel_if_unused(
    pkg: &mut Package,
    part: &str,
    root: &Element,
    rid: &str,
) -> Result<(), Error> {
    if rid_in_tree(root, rid) {
        return Ok(());
    }
    let media = crate::opc::resolve_id(pkg, part, rid).ok();
    let mut rels = Relationships::from_package(pkg, part)?;
    rels.remove_by_id(rid);
    if rels.rels.is_empty() {
        pkg.remove(&rels_part_name(part));
    } else {
        rels.write_to(pkg, part)?;
    }
    if let Some(m) = media {
        if !media_still_used(pkg, &m)? {
            pkg.remove(&m);
        }
    }
    Ok(())
}

fn rid_in_tree(root: &Element, rid: &str) -> bool {
    let mut used = false;
    walk_all(root, &mut |e| {
        if used {
            return;
        }
        let local = e.local_name();
        if (local == "blip" || local == "svgBlip")
            && (e.attr_local("embed") == Some(rid) || e.attr_local("link") == Some(rid))
        {
            used = true;
        }
    });
    used
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

pub(crate) fn media_still_used(pkg: &Package, media_opc: &str) -> Result<bool, Error> {
    crate::opc::part_still_referenced(
        pkg,
        media_opc,
        |name| crate::opc::source_part_of_rels(name, false),
        crate::opc::same_part_name,
    )
}

pub(crate) fn ensure_xmlns(root: &mut Element, prefix: &str, ns: &str) {
    let key = format!("xmlns:{prefix}");
    if root.attr(&key).is_none() && root.attr("xmlns") != Some(ns) {
        root.set_attr(&key, ns);
    }
}

pub(crate) fn set_attr_local(el: &mut Element, local: &str, value: &str) {
    if let Some((k, _)) = el
        .attrs
        .iter()
        .find(|(k, _)| k.rsplit(':').next().unwrap_or(k) == local)
    {
        let name = k.clone();
        el.set_attr(&name, value);
        return;
    }
    el.set_attr(local, value);
}

pub(crate) fn find_local_mut<'a>(el: &'a mut Element, local: &str) -> Option<&'a mut Element> {
    crate::xml::find_by_local_mut(el, local)
}

pub(crate) fn build_anchor(
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
            anchor.push_element(marker_el("xdr:to", to.unwrap_or(from), to_off_x, to_off_y));
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
        AnchorKind::RelSize | AnchorKind::AbsSize => {}
    }
    let pic = match existing_pic {
        Some(mut pic) => {
            if kind != AnchorKind::TwoCell {
                set_body_ext(&mut pic, cx, cy);
            }
            pic
        }
        None => build_pic(cnv, name, rid, cx, cy),
    };
    anchor.push_element(pic);
    anchor.push_element(Element::new("xdr:clientData"));
    anchor
}

fn set_body_ext(pic: &mut Element, cx: i64, cy: i64) {
    if let Some(xfrm) = find_local_mut(pic, "xfrm") {
        if let Some(ext) = xfrm.child_by_local_mut("ext") {
            set_attr_local(ext, "cx", &cx.to_string());
            set_attr_local(ext, "cy", &cy.to_string());
        }
    }
}

pub(crate) fn apply_two_cell_to(
    anchor: &mut Element,
    to: a1::CellRef,
    to_off_x: Option<i64>,
    to_off_y: Option<i64>,
) -> Result<(), Error> {
    let (_, cur_x, cur_y) = read_marker(anchor.child_by_local("to"))?;
    rewrite_to_marker(
        anchor,
        to,
        to_off_x.unwrap_or(cur_x),
        to_off_y.unwrap_or(cur_y),
    );
    Ok(())
}

fn rewrite_to_marker(anchor: &mut Element, to: a1::CellRef, off_x: i64, off_y: i64) {
    let new_to = marker_el("xdr:to", to, off_x, off_y);
    for n in anchor.children.iter_mut() {
        if let Node::Element(child) = n {
            if child.local_name() == "to" {
                *child = new_to;
                return;
            }
        }
    }
    let mut at = anchor.children.len();
    for (i, n) in anchor.children.iter().enumerate() {
        if let Node::Element(child) = n {
            if child.local_name() == "from" {
                at = i + 1;
                break;
            }
        }
    }
    anchor.children.insert(at, Node::Element(new_to));
}

pub(crate) fn marker_el(name: &str, cell: a1::CellRef, off_x: i64, off_y: i64) -> Element {
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

#[cfg(test)]
mod marker_tests {
    use super::read_marker;
    use crate::xml::Element;

    #[test]
    fn column_past_excel_max_is_xml_broken() {
        let mut from = Element::new("xdr:from");
        from.push_element(Element::new("xdr:col").with_text("16384"));
        from.push_element(Element::new("xdr:row").with_text("0"));
        let err = read_marker(Some(&from)).unwrap_err();
        assert_eq!(err.code(), crate::error::ErrorCode::XmlBroken);
    }

    #[test]
    fn last_excel_column_is_kept() {
        let mut from = Element::new("xdr:from");
        from.push_element(Element::new("xdr:col").with_text("16383"));
        from.push_element(Element::new("xdr:row").with_text("1048575"));
        let (cell, _, _) = read_marker(Some(&from)).unwrap();
        let cell = cell.unwrap();
        assert_eq!(cell.col, 16_384);
        assert_eq!(cell.row, 1_048_576);
    }

    #[test]
    fn anchor_walk_surfaces_column_overflow() {
        let mut anchor = Element::new("xdr:twoCellAnchor");
        let mut from = Element::new("xdr:from");
        from.push_element(Element::new("xdr:col").with_text("16384"));
        from.push_element(Element::new("xdr:row").with_text("0"));
        anchor.push_element(from);
        let mut root = Element::new("xdr:wsDr");
        root.push_element(anchor);
        let err = super::walk_anchors_mut(&mut root, &mut |anchor, _| {
            let _ = super::read_marker(anchor.child_by_local("from"))?;
            Ok(true)
        })
        .unwrap_err();
        assert_eq!(err.code(), crate::error::ErrorCode::XmlBroken);
    }
}
