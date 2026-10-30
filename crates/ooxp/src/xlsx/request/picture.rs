//! xlsx request：picture。

use serde_json::Value;

use crate::error::{Error, ErrorKind, TypeName};

use super::super::a1;
use super::super::picture::{self};

use super::common::*;
use super::types::*;

pub(super) fn parse_set_hf_text(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetHeaderFooterTextOp, Error> {
    deny_keys(obj, i, &["op", "id", "text"])?;
    let id = require_id(obj, i)?;
    let (sheet, slot) = super::super::hf::parse_slot_id(&id, i)?;
    let text = match obj.get("text") {
        Some(Value::String(s)) => s.clone(),
        None => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "text".into(),
                },
                format!("ops[{i}].text"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "text".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].text"),
            ));
        }
    };
    Ok(SetHeaderFooterTextOp { sheet, slot, text })
}

pub(super) fn parse_replace_pic(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<ReplacePictureOp, Error> {
    deny_keys(obj, i, &["op", "id", "file"])?;
    let id = require_id(obj, i)?;
    let target = picture::parse_pic_id(&id, i)?;
    let file = require_file(obj, i, "replacePicture")?;
    Ok(ReplacePictureOp { id, target, file })
}

pub(super) fn parse_insert_pic(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertPictureOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op", "id", "file", "cx", "cy", "from", "to", "anchor", "fromOffX", "fromOffY",
            "toOffX", "toOffY", "fromX", "fromY", "toX", "toY",
        ],
    )?;
    let id = require_id(obj, i)?;
    let (sheet, hf) = picture::parse_insert_sheet_id(&id, i)?;
    let file = require_file(obj, i, "insertPicture")?;
    let cx = parse_emu_field(obj.get("cx"), i, "cx")?;
    let cy = parse_emu_field(obj.get("cy"), i, "cy")?;
    if cx.is_some() != cy.is_some() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "insertPicture".into(),
                field: "cx and cy together".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    if hf.is_some() {
        for field in [
            "from", "to", "anchor", "fromOffX", "fromOffY", "toOffX", "toOffY", "fromX", "fromY",
            "toX", "toY",
        ] {
            if obj.contains_key(field) {
                return Err(Error::with_path(
                    ErrorKind::UnknownField {
                        field: field.into(),
                    },
                    format!("ops[{i}].{field}"),
                ));
            }
        }
        return Ok(InsertPictureOp {
            sheet,
            hf,
            file,
            cx,
            cy,
            from: a1::CellRef { col: 1, row: 1 },
            to: None,
            anchor: picture::AnchorKind::OneCell,
            from_off_x: 0,
            from_off_y: 0,
            to_off_x: 0,
            to_off_y: 0,
            rel: None,
        });
    }
    let rel = parse_rel_box(obj, i)?;
    if rel.is_some() {
        for field in [
            "from", "to", "anchor", "fromOffX", "fromOffY", "toOffX", "toOffY",
        ] {
            if obj.contains_key(field) {
                return Err(Error::with_path(
                    ErrorKind::UnknownField {
                        field: field.into(),
                    },
                    format!("ops[{i}].{field}"),
                ));
            }
        }
        return Ok(InsertPictureOp {
            sheet,
            hf: None,
            file,
            cx,
            cy,
            from: a1::CellRef { col: 1, row: 1 },
            to: None,
            anchor: picture::AnchorKind::RelSize,
            from_off_x: 0,
            from_off_y: 0,
            to_off_x: 0,
            to_off_y: 0,
            rel,
        });
    }
    let from = match obj.get("from") {
        Some(Value::String(s)) => a1::parse_cell(s).map_err(|e| e.at(format!("ops[{i}].from")))?,
        None => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "from".into(),
                },
                format!("ops[{i}].from"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "from".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].from"),
            ));
        }
    };
    let anchor = match obj.get("anchor") {
        None => picture::AnchorKind::OneCell,
        Some(Value::String(s)) => picture::AnchorKind::parse_insert(s).ok_or_else(|| {
            Error::with_path(ErrorKind::InvalidXlsxAnchor, format!("ops[{i}].anchor"))
        })?,
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "anchor".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].anchor"),
            ));
        }
    };
    let to = match obj.get("to") {
        None => None,
        Some(Value::String(s)) => {
            Some(a1::parse_cell(s).map_err(|e| e.at(format!("ops[{i}].to")))?)
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "to".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].to"),
            ));
        }
    };
    if anchor == picture::AnchorKind::TwoCell && to.is_none() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "insertPicture".into(),
                field: "to".into(),
            },
            format!("ops[{i}].to"),
        ));
    }
    if anchor == picture::AnchorKind::OneCell && to.is_some() {
        return Err(Error::with_path(
            ErrorKind::UnknownField { field: "to".into() },
            format!("ops[{i}].to"),
        ));
    }
    Ok(InsertPictureOp {
        sheet,
        hf: None,
        file,
        cx,
        cy,
        from,
        to,
        anchor,
        from_off_x: parse_off_field(obj.get("fromOffX"), i, "fromOffX")?.unwrap_or(0),
        from_off_y: parse_off_field(obj.get("fromOffY"), i, "fromOffY")?.unwrap_or(0),
        to_off_x: parse_off_field(obj.get("toOffX"), i, "toOffX")?.unwrap_or(0),
        to_off_y: parse_off_field(obj.get("toOffY"), i, "toOffY")?.unwrap_or(0),
        rel: None,
    })
}

pub(super) fn parse_delete_pic(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeletePictureOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let target = picture::parse_pic_id(&id, i)?;
    Ok(DeletePictureOp { id, target })
}

pub(super) fn parse_set_pic_size(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetPictureSizeOp, Error> {
    deny_keys(obj, i, &["op", "id", "cx", "cy", "to", "toOffX", "toOffY"])?;
    let id = require_id(obj, i)?;
    let target = picture::parse_pic_id(&id, i)?;
    let s = parse_drawing_size(obj, i, "setPictureSize")?;
    Ok(SetPictureSizeOp {
        id,
        target,
        cx: s.cx,
        cy: s.cy,
        to: s.to,
        to_off_x: s.to_off_x,
        to_off_y: s.to_off_y,
    })
}

pub(super) fn parse_set_pic_anchor(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetPictureAnchorOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op", "id", "anchor", "from", "to", "fromOffX", "fromOffY", "toOffX", "toOffY", "posX",
            "posY", "editAs", "cx", "cy", "fromX", "fromY", "toX", "toY",
        ],
    )?;
    let id = require_id(obj, i)?;
    let target = picture::parse_pic_id(&id, i)?;
    let p = parse_drawing_anchor_patch(obj, i, "setPictureAnchor")?;
    Ok(SetPictureAnchorOp {
        id,
        target,
        anchor: p.anchor,
        from: p.from,
        to: p.to,
        from_off_x: p.from_off_x,
        from_off_y: p.from_off_y,
        to_off_x: p.to_off_x,
        to_off_y: p.to_off_y,
        pos_x: p.pos_x,
        pos_y: p.pos_y,
        edit_as: p.edit_as,
        cx: p.cx,
        cy: p.cy,
        rel: p.rel,
    })
}

pub(super) fn parse_insert_anchor(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    op: &str,
) -> Result<InsertAnchorSpec, Error> {
    let rel = parse_rel_box(obj, i)?;
    if rel.is_some() {
        for field in [
            "from", "to", "anchor", "fromOffX", "fromOffY", "toOffX", "toOffY",
        ] {
            if obj.contains_key(field) {
                return Err(Error::with_path(
                    ErrorKind::UnknownField {
                        field: field.into(),
                    },
                    format!("ops[{i}].{field}"),
                ));
            }
        }
        let cx = parse_emu_field(obj.get("cx"), i, "cx")?;
        let cy = parse_emu_field(obj.get("cy"), i, "cy")?;
        if cx.is_some() != cy.is_some() {
            return Err(Error::with_path(
                ErrorKind::OpRequiresField {
                    op: op.into(),
                    field: "cx and cy together".into(),
                },
                format!("ops[{i}]"),
            ));
        }
        return Ok(InsertAnchorSpec {
            from: a1::CellRef { col: 1, row: 1 },
            to: None,
            anchor: picture::AnchorKind::RelSize,
            from_off_x: 0,
            from_off_y: 0,
            to_off_x: 0,
            to_off_y: 0,
            cx,
            cy,
            rel,
        });
    }
    let from = parse_required_a1(obj, i, "from")?;
    let anchor = match obj.get("anchor") {
        None => picture::AnchorKind::OneCell,
        Some(Value::String(s)) => picture::AnchorKind::parse_insert(s).ok_or_else(|| {
            Error::with_path(ErrorKind::InvalidXlsxAnchor, format!("ops[{i}].anchor"))
        })?,
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "anchor".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].anchor"),
            ));
        }
    };
    let to = parse_opt_a1(obj.get("to"), i, "to")?;
    if anchor == picture::AnchorKind::TwoCell && to.is_none() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: op.into(),
                field: "to".into(),
            },
            format!("ops[{i}].to"),
        ));
    }
    if anchor == picture::AnchorKind::OneCell && to.is_some() {
        return Err(Error::with_path(
            ErrorKind::UnknownField { field: "to".into() },
            format!("ops[{i}].to"),
        ));
    }
    let cx = parse_emu_field(obj.get("cx"), i, "cx")?;
    let cy = parse_emu_field(obj.get("cy"), i, "cy")?;
    if cx.is_some() != cy.is_some() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: op.into(),
                field: "cx and cy together".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(InsertAnchorSpec {
        from,
        to,
        anchor,
        from_off_x: parse_off_field(obj.get("fromOffX"), i, "fromOffX")?.unwrap_or(0),
        from_off_y: parse_off_field(obj.get("fromOffY"), i, "fromOffY")?.unwrap_or(0),
        to_off_x: parse_off_field(obj.get("toOffX"), i, "toOffX")?.unwrap_or(0),
        to_off_y: parse_off_field(obj.get("toOffY"), i, "toOffY")?.unwrap_or(0),
        cx,
        cy,
        rel: None,
    })
}
