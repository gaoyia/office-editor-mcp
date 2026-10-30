//! xlsx request：common。

use serde_json::Value;

use crate::error::{Error, ErrorKind, IdReason, TypeName};

use super::super::a1;
use super::super::picture::{self};

use super::types::*;
pub(super) use crate::op_fields::{
    deny_keys, deny_keys_at, parse_bool_field, parse_emu_field, parse_off_field, parse_opt_bool,
    parse_optional_date, parse_optional_text, parse_required_bool, require_file, require_id,
};

pub(super) fn parse_patch_string(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    field: &str,
) -> Result<Patch<String>, Error> {
    match obj.get(field) {
        None => Ok(Patch::Absent),
        Some(Value::Null) => Ok(Patch::Clear),
        Some(Value::String(s)) if !s.is_empty() => Ok(Patch::Set(s.clone())),
        Some(Value::String(_)) => Err(Error::with_path(
            ErrorKind::InvalidCellStyle {
                field: field.into(),
            },
            format!("ops[{i}].{field}"),
        )),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].{field}"),
        )),
    }
}

pub(super) fn parse_patch_object<T>(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    field: &str,
    parse: impl Fn(&serde_json::Map<String, Value>, usize, &str) -> Result<T, Error>,
) -> Result<Patch<T>, Error> {
    match obj.get(field) {
        None => Ok(Patch::Absent),
        Some(Value::Null) => Ok(Patch::Clear),
        Some(Value::Object(m)) => Ok(Patch::Set(parse(m, i, field)?)),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Object,
            },
            format!("ops[{i}].{field}"),
        )),
    }
}

pub(super) fn deny_sub_keys(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    prefix: &str,
    allowed: &[&str],
) -> Result<(), Error> {
    for k in obj.keys() {
        if !allowed.contains(&k.as_str()) {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("ops[{i}].{prefix}.{k}"),
            ));
        }
    }
    Ok(())
}

pub(super) fn parse_sub_bool(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    prefix: &str,
    field: &str,
) -> Result<Option<bool>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::Bool(b)) => Ok(Some(*b)),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Boolean,
            },
            format!("ops[{i}].{prefix}.{field}"),
        )),
    }
}

pub(super) fn parse_opt_u32_field(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    prefix: &str,
    field: &str,
) -> Result<Option<u32>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::Number(n)) => {
            if let Some(u) = n.as_u64().filter(|v| *v <= u32::MAX as u64) {
                Ok(Some(u as u32))
            } else {
                Err(Error::with_path(
                    ErrorKind::InvalidCellStyle {
                        field: field.into(),
                    },
                    format!("ops[{i}].{prefix}.{field}"),
                ))
            }
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::InvalidCellStyle {
                field: field.into(),
            },
            format!("ops[{i}].{prefix}.{field}"),
        )),
    }
}

pub(super) fn parse_opt_a1_range(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    field: &str,
) -> Result<Option<(a1::CellRef, a1::CellRef)>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::String(s)) => {
            let (start, end) = a1::parse_range(s).map_err(|e| e.at(format!("ops[{i}].{field}")))?;
            Ok(Some((start, end)))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].{field}"),
        )),
    }
}

pub(super) fn parse_opt_a1(
    v: Option<&Value>,
    i: usize,
    field: &str,
) -> Result<Option<a1::CellRef>, Error> {
    match v {
        None => Ok(None),
        Some(Value::String(s)) => Ok(Some(
            a1::parse_cell(s).map_err(|e| e.at(format!("ops[{i}].{field}")))?,
        )),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].{field}"),
        )),
    }
}

pub(super) fn parse_required_a1(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    field: &str,
) -> Result<a1::CellRef, Error> {
    match obj.get(field) {
        Some(Value::String(s)) => a1::parse_cell(s).map_err(|e| e.at(format!("ops[{i}].{field}"))),
        None => Err(Error::with_path(
            ErrorKind::MissingField {
                field: field.into(),
            },
            format!("ops[{i}].{field}"),
        )),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].{field}"),
        )),
    }
}

pub(super) fn parse_opt_nonempty_string(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    field: &str,
) -> Result<Option<String>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::String(s)) if !s.is_empty() => Ok(Some(s.clone())),
        Some(Value::String(_)) => Err(Error::with_path(
            ErrorKind::InvalidConnection {
                field: field.into(),
            },
            format!("ops[{i}].{field}"),
        )),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].{field}"),
        )),
    }
}

pub(super) fn parse_sheet_only_id(
    id: &str,
    op_index: usize,
    field: &str,
    reason: IdReason,
) -> Result<String, Error> {
    if id.contains("/chart:")
        || id.contains("/pic:")
        || id.contains("/hf:")
        || id.contains("/dv:")
        || id.contains("/cf:")
        || id.contains("/table:")
        || id.contains("/pivot:")
        || id.contains("/link:")
        || id.contains("/spark:")
        || id.contains("/ole:")
        || id.contains("/model3d:")
        || id.contains("/math:")
        || id.contains("/slicer:")
        || id.contains("/editRange:")
        || id.starts_with("cell:")
    {
        return Err(Error::with_path(
            ErrorKind::InvalidId { reason },
            format!("ops[{op_index}].{field}"),
        ));
    }
    let Some(name) = id.strip_prefix("sheet:") else {
        return Err(Error::with_path(
            ErrorKind::InvalidId { reason },
            format!("ops[{op_index}].{field}"),
        ));
    };
    if name.is_empty() || name.contains('/') {
        return Err(Error::with_path(
            ErrorKind::InvalidId { reason },
            format!("ops[{op_index}].{field}"),
        ));
    }
    Ok(name.to_string())
}

pub(super) fn parse_drawing_size(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    op: &str,
) -> Result<DrawingSizePatch, Error> {
    let cx = parse_emu_field(obj.get("cx"), i, "cx")?;
    let cy = parse_emu_field(obj.get("cy"), i, "cy")?;
    let to = parse_opt_a1(obj.get("to"), i, "to")?;
    let to_off_x = parse_off_field(obj.get("toOffX"), i, "toOffX")?;
    let to_off_y = parse_off_field(obj.get("toOffY"), i, "toOffY")?;
    if (cx.is_some() || cy.is_some()) && (to.is_some() || to_off_x.is_some() || to_off_y.is_some())
    {
        return Err(Error::with_path(
            ErrorKind::MutuallyExclusive {
                a: "cx/cy".into(),
                b: "to".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    if (to_off_x.is_some() || to_off_y.is_some()) && to.is_none() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: op.into(),
                field: "to".into(),
            },
            format!("ops[{i}].to"),
        ));
    }
    if cx.is_none() && cy.is_none() && to.is_none() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: op.into(),
                field: "cx or cy or to".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(DrawingSizePatch {
        cx,
        cy,
        to,
        to_off_x,
        to_off_y,
    })
}

pub(super) fn parse_drawing_anchor_patch(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    op: &str,
) -> Result<DrawingAnchorPatch, Error> {
    let anchor = match obj.get("anchor") {
        None => None,
        Some(Value::String(s)) => Some(picture::AnchorKind::parse_set(s).ok_or_else(|| {
            Error::with_path(ErrorKind::InvalidXlsxAnchor, format!("ops[{i}].anchor"))
        })?),
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
    let edit_as = match obj.get("editAs") {
        None => None,
        Some(Value::String(s)) => Some(picture::EditAs::parse(s).ok_or_else(|| {
            Error::with_path(ErrorKind::InvalidXlsxEditAs, format!("ops[{i}].editAs"))
        })?),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "editAs".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].editAs"),
            ));
        }
    };
    let from = parse_opt_a1(obj.get("from"), i, "from")?;
    let to = parse_opt_a1(obj.get("to"), i, "to")?;
    let from_off_x = parse_off_field(obj.get("fromOffX"), i, "fromOffX")?;
    let from_off_y = parse_off_field(obj.get("fromOffY"), i, "fromOffY")?;
    let to_off_x = parse_off_field(obj.get("toOffX"), i, "toOffX")?;
    let to_off_y = parse_off_field(obj.get("toOffY"), i, "toOffY")?;
    let pos_x = parse_off_field(obj.get("posX"), i, "posX")?;
    let pos_y = parse_off_field(obj.get("posY"), i, "posY")?;
    let cx = parse_emu_field(obj.get("cx"), i, "cx")?;
    let cy = parse_emu_field(obj.get("cy"), i, "cy")?;
    let rel = parse_rel_box_patch(obj, i)?;
    let has_any = anchor.is_some()
        || from.is_some()
        || to.is_some()
        || from_off_x.is_some()
        || from_off_y.is_some()
        || to_off_x.is_some()
        || to_off_y.is_some()
        || pos_x.is_some()
        || pos_y.is_some()
        || edit_as.is_some()
        || cx.is_some()
        || cy.is_some()
        || rel.is_some();
    if !has_any {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: op.into(),
                field: "anchor, from, to, offsets, pos, editAs, or size".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(DrawingAnchorPatch {
        anchor,
        from,
        to,
        from_off_x,
        from_off_y,
        to_off_x,
        to_off_y,
        pos_x,
        pos_y,
        edit_as,
        cx,
        cy,
        rel,
    })
}

pub(super) fn parse_rel_box(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<Option<RelBox>, Error> {
    let fx = parse_unit_frac(obj, i, "fromX")?;
    let fy = parse_unit_frac(obj, i, "fromY")?;
    let tx = parse_unit_frac(obj, i, "toX")?;
    let ty = parse_unit_frac(obj, i, "toY")?;
    match (fx, fy, tx, ty) {
        (None, None, None, None) => Ok(None),
        (Some(from_x), Some(from_y), Some(to_x), Some(to_y)) => Ok(Some(RelBox {
            from_x,
            from_y,
            to_x,
            to_y,
        })),
        _ => Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "insert".into(),
                field: "fromX/fromY/toX/toY together".into(),
            },
            format!("ops[{i}]"),
        )),
    }
}

pub(super) fn parse_rel_box_patch(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<Option<RelBoxPatch>, Error> {
    let from_x = parse_unit_frac(obj, i, "fromX")?;
    let from_y = parse_unit_frac(obj, i, "fromY")?;
    let to_x = parse_unit_frac(obj, i, "toX")?;
    let to_y = parse_unit_frac(obj, i, "toY")?;
    if from_x.is_none() && from_y.is_none() && to_x.is_none() && to_y.is_none() {
        return Ok(None);
    }
    Ok(Some(RelBoxPatch {
        from_x,
        from_y,
        to_x,
        to_y,
    }))
}

pub(super) fn parse_unit_frac(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    field: &str,
) -> Result<Option<f64>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::Number(n)) => {
            let v = n.as_f64().ok_or_else(|| {
                Error::with_path(
                    ErrorKind::WrongType {
                        field: field.into(),
                        expected: TypeName::Integer,
                    },
                    format!("ops[{i}].{field}"),
                )
            })?;
            if !(0.0..=1.0).contains(&v) {
                return Err(Error::with_path(
                    ErrorKind::InvalidXlsxAnchor,
                    format!("ops[{i}].{field}"),
                ));
            }
            Ok(Some(v))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Integer,
            },
            format!("ops[{i}].{field}"),
        )),
    }
}
