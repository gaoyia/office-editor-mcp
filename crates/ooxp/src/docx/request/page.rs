//! docx request：page。

use serde_json::Value;

use crate::error::{Error, ErrorKind, IdReason, TypeName};

use super::common::*;
use super::sect::*;
use super::types::*;

pub(super) fn parse_set_page_background(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetPageBackgroundOp, Error> {
    deny_keys(obj, i, &["op", "id", "color", "fill"])?;
    let id = require_op_id(obj, i)?;
    require_document_id(&id, i, IdReason::SetPageBackgroundNeedsDocument)?;
    let color = match obj.get("color") {
        None => {
            return Err(Error::with_path(
                ErrorKind::OpRequiresField {
                    op: "setPageBackground".into(),
                    field: "color".into(),
                },
                format!("ops[{i}].color"),
            ));
        }
        Some(Value::Null) => None,
        Some(Value::String(s)) => {
            let parsed = super::super::page_bg::parse_color(s).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::InvalidPageBackgroundColor,
                    format!("ops[{i}].color"),
                )
            })?;
            Some(parsed)
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "color".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].color"),
            ));
        }
    };
    let fill = match obj.get("fill") {
        None => None,
        Some(v) => Some(super::super::page_bg::parse_fill(
            v,
            &format!("ops[{i}].fill"),
        )?),
    };
    if color.is_none() && fill.is_some() {
        return Err(Error::with_path(
            ErrorKind::InvalidPageBackgroundFill,
            format!("ops[{i}].fill"),
        ));
    }
    Ok(SetPageBackgroundOp { color, fill })
}

pub(super) fn parse_set_page_borders(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetPageBordersOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op",
            "id",
            "enabled",
            "offsetFrom",
            "display",
            "zOrder",
            "top",
            "left",
            "bottom",
            "right",
        ],
    )?;
    let id = require_op_id(obj, i)?;
    let n = parse_section_id(&id, i)?;
    let offset_from = parse_page_border_enum(
        obj.get("offsetFrom"),
        i,
        "offsetFrom",
        ErrorKind::InvalidPageBorderOffset,
        &["page", "text"],
    )?;
    let display = parse_page_border_enum(
        obj.get("display"),
        i,
        "display",
        ErrorKind::InvalidPageBorderDisplay,
        &["allPages", "firstPage", "notFirstPage"],
    )?;
    let z_order = parse_page_border_enum(
        obj.get("zOrder"),
        i,
        "zOrder",
        ErrorKind::InvalidPageBorderZOrder,
        &["front", "back"],
    )?;
    let top = parse_border_side(obj.get("top"), i, "top")?;
    let left = parse_border_side(obj.get("left"), i, "left")?;
    let bottom = parse_border_side(obj.get("bottom"), i, "bottom")?;
    let right = parse_border_side(obj.get("right"), i, "right")?;
    let other = offset_from.is_some()
        || display.is_some()
        || z_order.is_some()
        || top.is_some()
        || left.is_some()
        || bottom.is_some()
        || right.is_some();
    let enabled = parse_enabled_false(obj, i, other)?;
    if enabled.is_none() && !other {
        return Err(Error::with_path(
            ErrorKind::NeedPageBordersField,
            format!("ops[{i}]"),
        ));
    }
    Ok(SetPageBordersOp {
        id,
        n,
        enabled,
        offset_from,
        display,
        z_order,
        top,
        left,
        bottom,
        right,
    })
}

pub(super) fn parse_set_form_prot(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetFormProtOp, Error> {
    deny_keys(obj, i, &["op", "id", "formProt"])?;
    let id = require_op_id(obj, i)?;
    let n = parse_section_id(&id, i)?;
    let form_prot = match parse_opt_null_bool(obj, i, "formProt")? {
        Some(v) => v,
        None => {
            return Err(Error::with_path(
                ErrorKind::OpRequiresField {
                    op: "setFormProt".into(),
                    field: "formProt".into(),
                },
                format!("ops[{i}].formProt"),
            ));
        }
    };
    Ok(SetFormProtOp { id, n, form_prot })
}

pub(super) fn parse_page_border_enum(
    v: Option<&Value>,
    i: usize,
    field: &str,
    kind: ErrorKind,
    allowed: &[&str],
) -> Result<Option<String>, Error> {
    match v {
        None => Ok(None),
        Some(Value::String(s)) if allowed.contains(&s.as_str()) => Ok(Some(s.clone())),
        Some(Value::String(_)) => Err(Error::with_path(kind, format!("ops[{i}].{field}"))),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].{field}"),
        )),
    }
}

pub(super) fn parse_border_side(
    v: Option<&Value>,
    i: usize,
    field: &str,
) -> Result<Option<Option<super::super::page_bg::BorderSide>>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(None)),
        Some(Value::Object(map)) => {
            for k in map.keys() {
                if !["val", "sz", "space", "color"].contains(&k.as_str()) {
                    return Err(Error::with_path(
                        ErrorKind::UnknownField { field: k.clone() },
                        format!("ops[{i}].{field}.{k}"),
                    ));
                }
            }
            let val = match map.get("val") {
                Some(Value::String(s)) => {
                    super::super::page_bg::parse_border_val(s).ok_or_else(|| {
                        Error::with_path(
                            ErrorKind::InvalidPageBorderVal,
                            format!("ops[{i}].{field}.val"),
                        )
                    })?
                }
                Some(_) => {
                    return Err(Error::with_path(
                        ErrorKind::WrongType {
                            field: format!("{field}.val"),
                            expected: TypeName::String,
                        },
                        format!("ops[{i}].{field}.val"),
                    ));
                }
                None => {
                    return Err(Error::with_path(
                        ErrorKind::OpRequiresField {
                            op: "setPageBorders".into(),
                            field: "val".into(),
                        },
                        format!("ops[{i}].{field}.val"),
                    ));
                }
            };
            let sz = parse_twip_field(map.get("sz"), i, &format!("{field}.sz"))?;
            let space = parse_twip_field(map.get("space"), i, &format!("{field}.space"))?;
            let color = match map.get("color") {
                None => None,
                Some(Value::String(s)) => {
                    Some(super::super::page_bg::parse_side_color(s).ok_or_else(|| {
                        Error::with_path(
                            ErrorKind::InvalidPageBorderColor,
                            format!("ops[{i}].{field}.color"),
                        )
                    })?)
                }
                Some(_) => {
                    return Err(Error::with_path(
                        ErrorKind::WrongType {
                            field: format!("{field}.color"),
                            expected: TypeName::String,
                        },
                        format!("ops[{i}].{field}.color"),
                    ));
                }
            };
            Ok(Some(Some(super::super::page_bg::BorderSide {
                val: val.to_string(),
                sz,
                space,
                color,
            })))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Object,
            },
            format!("ops[{i}].{field}"),
        )),
    }
}
