//! pptx request：picture。

use serde_json::{Map, Value};

use crate::error::{Error, ErrorKind, TypeName};

use super::super::comments;
use super::super::media;
use super::super::picture;

use super::common::*;
use super::link::*;
use super::types::*;

pub(super) fn parse_replace_pic(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<ReplacePictureOp, Error> {
    deny_keys(obj, i, &["op", "id", "file"])?;
    let id = require_id(obj, i)?;
    let target = picture::parse_pic_id(&id, &format!("ops[{i}].id"))?;
    let file = require_file(obj, i, "replacePicture")?;
    Ok(ReplacePictureOp { id, target, file })
}

pub(super) fn parse_insert_pic(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<InsertPictureOp, Error> {
    let id = require_id(obj, i)?;
    if id.contains("/comment:") {
        deny_keys(obj, i, &["op", "id", "file"])?;
        let target = comments::parse_comment_id(&id, &format!("ops[{i}].id"))?;
        let file = require_file(obj, i, "insertPicture")?;
        return Ok(InsertPictureOp {
            slide: target.slide,
            on_notes: false,
            master: None,
            comment: Some(target),
            file,
            x: None,
            y: None,
            cx: None,
            cy: None,
        });
    }
    deny_keys(obj, i, &["op", "id", "file", "x", "y", "cx", "cy"])?;
    let host = picture::parse_insert_host(&id, i, crate::error::IdReason::InsertNeedsSlide)?;
    let (slide, on_notes, master) = host.parts();
    let file = require_file(obj, i, "insertPicture")?;
    let x = parse_off_field(obj.get("x"), i, "x")?;
    let y = parse_off_field(obj.get("y"), i, "y")?;
    if x.is_some() != y.is_some() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "insertPicture".into(),
                field: "x and y together".into(),
            },
            format!("ops[{i}]"),
        ));
    }
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
    Ok(InsertPictureOp {
        slide,
        on_notes,
        master,
        comment: None,
        file,
        x,
        y,
        cx,
        cy,
    })
}

pub(super) fn parse_delete_pic(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<DeletePictureOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let target = picture::parse_pic_id(&id, &format!("ops[{i}].id"))?;
    Ok(DeletePictureOp { id, target })
}

fn parse_pic_geom_id(id: &str, path: &str) -> Result<PicGeomTarget, Error> {
    if media::is_media_id(id) {
        Ok(PicGeomTarget::Media(media::parse_media_id(id, path)?))
    } else {
        let target = picture::parse_pic_id(id, path)?;
        picture::reject_comment_pic(&target, path)?;
        Ok(PicGeomTarget::Picture(target))
    }
}

pub(super) fn parse_set_pic_size(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<SetPictureSizeOp, Error> {
    deny_keys(obj, i, &["op", "id", "cx", "cy"])?;
    let id = require_id(obj, i)?;
    let target = parse_pic_geom_id(&id, &format!("ops[{i}].id"))?;
    let cx = parse_emu_field(obj.get("cx"), i, "cx")?;
    let cy = parse_emu_field(obj.get("cy"), i, "cy")?;
    if cx.is_none() && cy.is_none() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setPictureSize".into(),
                field: "cx or cy".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(SetPictureSizeOp { id, target, cx, cy })
}

pub(super) fn parse_set_pic_pos(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<SetPicturePosOp, Error> {
    deny_keys(obj, i, &["op", "id", "x", "y"])?;
    let id = require_id(obj, i)?;
    let target = parse_pic_geom_id(&id, &format!("ops[{i}].id"))?;
    let x = parse_off_field(obj.get("x"), i, "x")?;
    let y = parse_off_field(obj.get("y"), i, "y")?;
    if x.is_none() && y.is_none() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setPicturePos".into(),
                field: "x or y".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(SetPicturePosOp { id, target, x, y })
}

pub(crate) fn parse_create_picture(
    v: &Value,
    prefix: &str,
    slide: usize,
) -> Result<CreatePicture, Error> {
    let obj = v.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "pictures".into(),
                expected: TypeName::Object,
            },
            prefix.to_string(),
        )
    })?;
    for k in obj.keys() {
        if !matches!(k.as_str(), "file" | "x" | "y" | "cx" | "cy" | "hyperlink") {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("{prefix}.{k}"),
            ));
        }
    }
    let file = match obj.get("file") {
        Some(Value::String(s)) if !s.is_empty() => s.clone(),
        Some(Value::String(_)) | None => {
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
                    expected: TypeName::NonEmptyString,
                },
                format!("{prefix}.file"),
            ));
        }
    };
    let x = parse_off_at(obj.get("x"), &format!("{prefix}.x"), "x")?;
    let y = parse_off_at(obj.get("y"), &format!("{prefix}.y"), "y")?;
    if x.is_some() != y.is_some() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "create".into(),
                field: "x and y together".into(),
            },
            prefix.to_string(),
        ));
    }
    let cx = parse_emu_at(obj.get("cx"), &format!("{prefix}.cx"), "cx")?;
    let cy = parse_emu_at(obj.get("cy"), &format!("{prefix}.cy"), "cy")?;
    if cx.is_some() != cy.is_some() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "create".into(),
                field: "cx and cy together".into(),
            },
            prefix.to_string(),
        ));
    }
    let op = InsertPictureOp {
        slide,
        on_notes: false,
        master: None,
        comment: None,
        file,
        x,
        y,
        cx,
        cy,
    };
    let link = parse_create_opt_hyperlink(obj, prefix)?;
    Ok(CreatePicture { op, link })
}
