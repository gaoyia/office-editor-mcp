//! xlsx request：drawing。

use serde_json::Value;

use crate::error::{Error, ErrorKind, IdReason, TypeName};

use super::super::shape::{self};
use super::super::smartart as xlsx_smartart;

use super::common::*;
use super::picture::*;
use super::types::*;

pub(super) fn parse_opt_text(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<Option<String>, Error> {
    match obj.get("text") {
        None => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "text".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].text"),
        )),
    }
}

pub(super) fn parse_require_text(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    op: &str,
) -> Result<String, Error> {
    match obj.get("text") {
        Some(Value::String(s)) => Ok(s.clone()),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "text".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].text"),
        )),
        None => Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: op.into(),
                field: "text".into(),
            },
            format!("ops[{i}].text"),
        )),
    }
}

pub(super) fn parse_insert_textbox(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertTextBoxOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op",
            "id",
            "text",
            "cx",
            "cy",
            "from",
            "to",
            "anchor",
            "fromOffX",
            "fromOffY",
            "toOffX",
            "toOffY",
            "fromX",
            "fromY",
            "toX",
            "toY",
            "sz",
            "color",
            "textFill",
            "textOutline",
            "bold",
            "italic",
            "underline",
            "font",
            "eastAsiaFont",
            "align",
            "textAnchor",
            "spcBef",
            "spcAft",
            "lnSpc",
            "spc",
            "kern",
        ],
    )?;
    let id = require_id(obj, i)?;
    let sheet = shape::parse_insert_sheet_id(&id, i, IdReason::InsertTextBoxNeedsSheet)?;
    let a = parse_insert_anchor(obj, i, "insertTextBox")?;
    Ok(InsertTextBoxOp {
        sheet,
        text: parse_opt_text(obj, i)?,
        cx: a.cx,
        cy: a.cy,
        from: a.from,
        to: a.to,
        anchor: a.anchor,
        from_off_x: a.from_off_x,
        from_off_y: a.from_off_y,
        to_off_x: a.to_off_x,
        to_off_y: a.to_off_y,
        format: crate::drawing_text::parse(obj, &format!("ops[{i}]"), false)?,
        rel: a.rel,
    })
}

pub(super) fn parse_insert_shape(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertShapeOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op",
            "id",
            "text",
            "cx",
            "cy",
            "from",
            "to",
            "anchor",
            "fromOffX",
            "fromOffY",
            "toOffX",
            "toOffY",
            "fromX",
            "fromY",
            "toX",
            "toY",
            "prst",
            "sz",
            "color",
            "textFill",
            "textOutline",
            "bold",
            "italic",
            "underline",
            "font",
            "eastAsiaFont",
            "align",
            "textAnchor",
            "spcBef",
            "spcAft",
            "lnSpc",
            "spc",
            "kern",
        ],
    )?;
    let id = require_id(obj, i)?;
    let sheet = shape::parse_insert_sheet_id(&id, i, IdReason::InsertShapeNeedsSheet)?;
    let a = parse_insert_anchor(obj, i, "insertShape")?;
    Ok(InsertShapeOp {
        sheet,
        text: parse_opt_text(obj, i)?,
        cx: a.cx,
        cy: a.cy,
        from: a.from,
        to: a.to,
        anchor: a.anchor,
        from_off_x: a.from_off_x,
        from_off_y: a.from_off_y,
        to_off_x: a.to_off_x,
        to_off_y: a.to_off_y,
        prst: parse_shape_prst(obj, i)?,
        format: crate::drawing_text::parse(obj, &format!("ops[{i}]"), false)?,
        rel: a.rel,
    })
}

pub(super) fn parse_insert_connector(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertConnectorOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op", "id", "cx", "cy", "from", "to", "anchor", "fromOffX", "fromOffY", "toOffX",
            "toOffY", "fromX", "fromY", "toX", "toY", "prst",
        ],
    )?;
    let id = require_id(obj, i)?;
    let sheet = shape::parse_insert_sheet_id(&id, i, IdReason::InsertConnectorNeedsSheet)?;
    let a = parse_insert_anchor(obj, i, "insertConnector")?;
    Ok(InsertConnectorOp {
        sheet,
        cx: a.cx,
        cy: a.cy,
        from: a.from,
        to: a.to,
        anchor: a.anchor,
        from_off_x: a.from_off_x,
        from_off_y: a.from_off_y,
        to_off_x: a.to_off_x,
        to_off_y: a.to_off_y,
        prst: parse_connector_prst(obj, i)?,
        rel: a.rel,
    })
}

pub(super) fn parse_shape_prst(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<String, Error> {
    match obj.get("prst") {
        None => Ok("rect".to_string()),
        Some(Value::String(s)) => {
            if crate::pptx::shape::is_writable_shape_prst(s) {
                Ok(s.clone())
            } else {
                Err(Error::with_path(
                    ErrorKind::InvalidShapePrst,
                    format!("ops[{i}].prst"),
                ))
            }
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "prst".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].prst"),
        )),
    }
}

pub(super) fn parse_connector_prst(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<String, Error> {
    match obj.get("prst") {
        None => Ok("straightConnector1".to_string()),
        Some(Value::String(s)) => {
            if s == "line" || crate::pptx::connector::is_writable_prst(s) {
                Ok(s.clone())
            } else {
                Err(Error::with_path(
                    ErrorKind::InvalidConnectorPrst,
                    format!("ops[{i}].prst"),
                ))
            }
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "prst".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].prst"),
        )),
    }
}

pub(super) fn parse_delete_textbox(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteTextBoxOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let target = shape::parse_sp_id(&id, i)?;
    Ok(DeleteTextBoxOp { id, target })
}

pub(super) fn parse_delete_shape(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteShapeOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let target = shape::parse_shape_id(&id, i)?;
    Ok(DeleteShapeOp { id, target })
}

pub(super) fn parse_delete_connector(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteConnectorOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let target = shape::parse_cxn_id(&id, i)?;
    Ok(DeleteConnectorOp { id, target })
}

pub(super) fn parse_group_shapes(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<GroupShapesOp, Error> {
    deny_keys(obj, i, &["op", "id", "ids"])?;
    let id = require_id(obj, i)?;
    let sheet = shape::parse_insert_sheet_id(&id, i, IdReason::GroupNeedsSheet)?;
    let ids = match obj.get("ids") {
        Some(Value::Array(arr)) => {
            if arr.len() < 2 {
                return Err(Error::with_path(
                    ErrorKind::GroupNeedsTwo,
                    format!("ops[{i}].ids"),
                ));
            }
            let mut out = Vec::new();
            for (k, v) in arr.iter().enumerate() {
                let s = v.as_str().ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::WrongType {
                            field: format!("ids[{k}]"),
                            expected: TypeName::String,
                        },
                        format!("ops[{i}].ids[{k}]"),
                    )
                })?;
                if s.is_empty() {
                    return Err(Error::with_path(
                        ErrorKind::WrongType {
                            field: format!("ids[{k}]"),
                            expected: TypeName::NonEmptyString,
                        },
                        format!("ops[{i}].ids[{k}]"),
                    ));
                }
                out.push(s.to_string());
            }
            out
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "ids".into(),
                    expected: TypeName::Array,
                },
                format!("ops[{i}].ids"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::OpRequiresField {
                    op: "groupShapes".into(),
                    field: "ids".into(),
                },
                format!("ops[{i}].ids"),
            ));
        }
    };
    let mut members = Vec::new();
    for (k, mid) in ids.iter().enumerate() {
        let parsed =
            shape::parse_drawing_obj_id(mid, i).map_err(|e| e.at(format!("ops[{i}].ids[{k}]")))?;
        members.push(parsed);
    }
    Ok(GroupShapesOp {
        sheet,
        ids,
        members,
    })
}

pub(super) fn parse_ungroup(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<UngroupOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let (target, kind) = shape::parse_drawing_obj_id(&id, i)?;
    Ok(UngroupOp { id, target, kind })
}

pub(super) fn parse_set_textbox_size(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetTextBoxSizeOp, Error> {
    deny_keys(obj, i, &["op", "id", "cx", "cy", "to", "toOffX", "toOffY"])?;
    let id = require_id(obj, i)?;
    let (target, kind) = shape::parse_drawing_obj_id(&id, i)?;
    let s = parse_drawing_size(obj, i, "setTextBoxSize")?;
    Ok(SetTextBoxSizeOp {
        id,
        target,
        kind,
        cx: s.cx,
        cy: s.cy,
        to: s.to,
        to_off_x: s.to_off_x,
        to_off_y: s.to_off_y,
    })
}

pub(super) fn parse_set_textbox_anchor(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetTextBoxAnchorOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op", "id", "anchor", "from", "to", "fromOffX", "fromOffY", "toOffX", "toOffY", "posX",
            "posY", "editAs", "cx", "cy", "fromX", "fromY", "toX", "toY",
        ],
    )?;
    let id = require_id(obj, i)?;
    let (target, kind) = shape::parse_drawing_obj_id(&id, i)?;
    let p = parse_drawing_anchor_patch(obj, i, "setTextBoxAnchor")?;
    Ok(SetTextBoxAnchorOp {
        id,
        target,
        kind,
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

pub(super) fn parse_set_shape_text(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetShapeTextOp, Error> {
    deny_keys(obj, i, &["op", "id", "text"])?;
    let id = require_id(obj, i)?;
    let (target, kind) = shape::parse_shape_text_id(&id, i)?;
    let text = parse_require_text(obj, i, "setShapeText")?;
    Ok(SetShapeTextOp {
        id,
        target,
        kind,
        text,
    })
}

pub(super) fn parse_set_shape_format(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetShapeFormatOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op",
            "id",
            "sz",
            "color",
            "textFill",
            "textOutline",
            "bold",
            "italic",
            "underline",
            "font",
            "eastAsiaFont",
            "align",
            "textAnchor",
            "spcBef",
            "spcAft",
            "lnSpc",
            "spc",
            "kern",
        ],
    )?;
    let id = require_id(obj, i)?;
    let (target, kind) = shape::parse_shape_text_id(&id, i)?;
    let format = crate::drawing_text::parse(obj, &format!("ops[{i}]"), true)?
        .expect("require_one 已保证有补丁");
    Ok(SetShapeFormatOp {
        id,
        target,
        kind,
        format,
    })
}

pub(super) fn parse_insert_smartart(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertSmartArtOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op", "id", "layout", "points", "cx", "cy", "from", "to", "anchor", "fromOffX",
            "fromOffY", "toOffX", "toOffY",
        ],
    )?;
    let id = require_id(obj, i)?;
    let sheet = shape::parse_insert_sheet_id(&id, i, IdReason::InsertSmartArtNeedsSheet)?;
    let (layout, points) = crate::smartart::parse_layout_and_points(obj, &format!("ops[{i}]"))?;
    let a = parse_insert_anchor(obj, i, "insertSmartArt")?;
    Ok(InsertSmartArtOp {
        sheet,
        layout,
        points,
        cx: a.cx,
        cy: a.cy,
        from: a.from,
        to: a.to,
        anchor: a.anchor,
        from_off_x: a.from_off_x,
        from_off_y: a.from_off_y,
        to_off_x: a.to_off_x,
        to_off_y: a.to_off_y,
    })
}

pub(super) fn parse_delete_smartart(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteSmartArtOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let target = xlsx_smartart::parse_diagram_id(&id, i)?;
    Ok(DeleteSmartArtOp { id, target })
}

pub(super) fn parse_set_smartart_text(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetSmartArtTextOp, Error> {
    deny_keys(obj, i, &["op", "id", "text"])?;
    let id = require_id(obj, i)?;
    let target = xlsx_smartart::parse_point_id(&id, i)?;
    let text = parse_require_text(obj, i, "setSmartArtText")?;
    Ok(SetSmartArtTextOp { id, target, text })
}

pub(super) fn parse_add_smartart_point(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<AddSmartArtPointOp, Error> {
    deny_keys(obj, i, &["op", "id", "text", "role"])?;
    let id = require_id(obj, i)?;
    let parent = xlsx_smartart::parse_add_parent(&id, i)?;
    let text = match obj.get("text") {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(s)) => s.clone(),
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
    let role = crate::smartart::parse_role_field(obj.get("role"), &format!("ops[{i}].role"))?;
    Ok(AddSmartArtPointOp {
        id,
        parent,
        text,
        role,
    })
}

pub(super) fn parse_delete_smartart_point(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteSmartArtPointOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let target = xlsx_smartart::parse_point_id(&id, i)?;
    Ok(DeleteSmartArtPointOp { id, target })
}

pub(super) fn parse_set_smartart_size(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetSmartArtSizeOp, Error> {
    deny_keys(obj, i, &["op", "id", "cx", "cy", "to", "toOffX", "toOffY"])?;
    let id = require_id(obj, i)?;
    let target = xlsx_smartart::parse_diagram_id(&id, i)?;
    let s = parse_drawing_size(obj, i, "setSmartArtSize")?;
    Ok(SetSmartArtSizeOp {
        id,
        target,
        cx: s.cx,
        cy: s.cy,
        to: s.to,
        to_off_x: s.to_off_x,
        to_off_y: s.to_off_y,
    })
}

pub(super) fn parse_set_smartart_anchor(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetSmartArtAnchorOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op", "id", "anchor", "from", "to", "fromOffX", "fromOffY", "toOffX", "toOffY", "posX",
            "posY", "editAs", "cx", "cy",
        ],
    )?;
    let id = require_id(obj, i)?;
    let target = xlsx_smartart::parse_diagram_id(&id, i)?;
    let p = parse_drawing_anchor_patch(obj, i, "setSmartArtAnchor")?;
    Ok(SetSmartArtAnchorOp {
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
    })
}

pub(super) fn parse_set_smartart_colors(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetSmartArtColorsOp, Error> {
    deny_keys(obj, i, &["op", "id", "colors"])?;
    let id = require_id(obj, i)?;
    let target = xlsx_smartart::parse_diagram_id(&id, i)?;
    let colors =
        crate::smartart::parse_colors_field(obj.get("colors"), &format!("ops[{i}].colors"))?;
    Ok(SetSmartArtColorsOp { id, target, colors })
}

pub(super) fn parse_set_smartart_style(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetSmartArtStyleOp, Error> {
    deny_keys(obj, i, &["op", "id", "style"])?;
    let id = require_id(obj, i)?;
    let target = xlsx_smartart::parse_diagram_id(&id, i)?;
    let style = crate::smartart::parse_style_field(obj.get("style"), &format!("ops[{i}].style"))?;
    Ok(SetSmartArtStyleOp { id, target, style })
}

pub(super) fn parse_set_smartart_layout(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetSmartArtLayoutOp, Error> {
    deny_keys(obj, i, &["op", "id", "layout"])?;
    let id = require_id(obj, i)?;
    let target = xlsx_smartart::parse_diagram_id(&id, i)?;
    let layout =
        crate::smartart::parse_layout_field(obj.get("layout"), &format!("ops[{i}].layout"))?;
    Ok(SetSmartArtLayoutOp { id, target, layout })
}
