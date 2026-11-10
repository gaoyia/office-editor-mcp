//! xlsx request：cell。

use serde_json::Value;

use crate::error::{Error, ErrorKind, IdReason, TypeName};

use super::super::style;

use super::common::*;
use super::types::*;

pub(super) fn parse_set_cell(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetCellOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op",
            "id",
            "value",
            "formula",
            "shared",
            "sharedRef",
            "array",
            "arrayRef",
            "dataTableRef",
        ],
    )?;
    let id = require_id(obj, i)?;
    let has_value = obj.contains_key("value");
    let has_formula = obj.contains_key("formula");
    if has_value && has_formula {
        return Err(Error::with_path(
            ErrorKind::MutuallyExclusive {
                a: "value".into(),
                b: "formula".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    if !has_value && !has_formula {
        return Err(Error::with_path(
            ErrorKind::SetCellNeedsValueOrFormula,
            format!("ops[{i}]"),
        ));
    }
    let formula = if has_formula {
        let f = obj.get("formula").and_then(Value::as_str).ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: "formula".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].formula"),
            )
        })?;
        if !f.starts_with('=') {
            return Err(Error::with_path(
                ErrorKind::FormulaMustStartWithEquals,
                format!("ops[{i}].formula"),
            ));
        }
        Some(f.to_string())
    } else {
        None
    };
    let value = if has_value {
        Some(parse_cell_value(&obj["value"], &format!("ops[{i}].value"))?)
    } else {
        None
    };
    let shared = parse_opt_bool(obj, i, "shared")?;
    let array = parse_opt_bool(obj, i, "array")?;
    let shared_ref = parse_opt_a1_range(obj, i, "sharedRef")?;
    let array_ref = parse_opt_a1_range(obj, i, "arrayRef")?;
    let data_table_ref = parse_opt_a1_range(obj, i, "dataTableRef")?;
    if shared_ref.is_some() && array_ref.is_some() {
        return Err(Error::with_path(
            ErrorKind::MutuallyExclusive {
                a: "sharedRef".into(),
                b: "arrayRef".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    if shared == Some(true) && array == Some(true) {
        return Err(Error::with_path(
            ErrorKind::MutuallyExclusive {
                a: "shared".into(),
                b: "array".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    if shared == Some(false) && shared_ref.is_some() {
        return Err(Error::with_path(
            ErrorKind::MutuallyExclusive {
                a: "shared".into(),
                b: "sharedRef".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    if array == Some(false) && array_ref.is_some() {
        return Err(Error::with_path(
            ErrorKind::MutuallyExclusive {
                a: "array".into(),
                b: "arrayRef".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    if data_table_ref.is_some()
        && (shared.is_some() || shared_ref.is_some() || array.is_some() || array_ref.is_some())
    {
        return Err(Error::with_path(
            ErrorKind::MutuallyExclusive {
                a: "dataTableRef".into(),
                b: "shared".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    if has_value
        && (shared == Some(true)
            || array == Some(true)
            || shared_ref.is_some()
            || array_ref.is_some()
            || data_table_ref.is_some())
    {
        let b = if shared == Some(true) {
            "shared"
        } else if array == Some(true) {
            "array"
        } else if shared_ref.is_some() {
            "sharedRef"
        } else if array_ref.is_some() {
            "arrayRef"
        } else {
            "dataTableRef"
        };
        return Err(Error::with_path(
            ErrorKind::MutuallyExclusive {
                a: "value".into(),
                b: b.into(),
            },
            format!("ops[{i}]"),
        ));
    }
    if (shared_ref.is_some()
        || array_ref.is_some()
        || array == Some(true)
        || data_table_ref.is_some())
        && formula.is_none()
    {
        return Err(Error::with_path(
            ErrorKind::MissingField {
                field: "formula".into(),
            },
            format!("ops[{i}].formula"),
        ));
    }
    Ok(SetCellOp {
        id,
        value,
        formula,
        shared,
        shared_ref,
        array,
        array_ref,
        data_table_ref,
    })
}

pub(super) fn parse_set_cell_style(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetCellStyleOp, Error> {
    deny_keys(
        obj,
        i,
        &["op", "id", "numFmt", "font", "fill", "border", "align"],
    )?;
    let id = require_id(obj, i)?;
    let num_fmt = parse_patch_string(obj, i, "numFmt")?;
    let font = parse_patch_object(obj, i, "font", parse_font_patch)?;
    let fill = parse_patch_object(obj, i, "fill", parse_fill_patch)?;
    let border = parse_patch_object(obj, i, "border", parse_border_patch)?;
    let align = parse_patch_object(obj, i, "align", parse_align_patch)?;
    if num_fmt.is_absent()
        && font.is_absent()
        && fill.is_absent()
        && border.is_absent()
        && align.is_absent()
    {
        return Err(Error::with_path(
            ErrorKind::NeedSetCellStyleField,
            format!("ops[{i}]"),
        ));
    }
    Ok(SetCellStyleOp {
        id,
        num_fmt,
        font,
        fill,
        border,
        align,
    })
}

pub(super) fn parse_font_patch(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    prefix: &str,
) -> Result<FontPatch, Error> {
    deny_sub_keys(
        obj,
        i,
        prefix,
        &["name", "sz", "bold", "italic", "underline", "color"],
    )?;
    let name = match obj.get("name") {
        None => None,
        Some(Value::String(s)) if !s.is_empty() => Some(s.clone()),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::InvalidCellStyle {
                    field: "name".into(),
                },
                format!("ops[{i}].{prefix}.name"),
            ));
        }
    };
    let sz = match obj.get("sz") {
        None => None,
        Some(Value::Number(n)) => {
            let v = n.as_f64().unwrap_or(f64::NAN);
            if !v.is_finite() || v <= 0.0 || v > 409.0 {
                return Err(Error::with_path(
                    ErrorKind::InvalidCellStyle { field: "sz".into() },
                    format!("ops[{i}].{prefix}.sz"),
                ));
            }
            Some(v)
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "sz".into(),
                    expected: TypeName::Integer,
                },
                format!("ops[{i}].{prefix}.sz"),
            ));
        }
    };
    let bold = parse_sub_bool(obj, i, prefix, "bold")?;
    let italic = parse_sub_bool(obj, i, prefix, "italic")?;
    let underline = match obj.get("underline") {
        None => None,
        Some(Value::Null) | Some(Value::Bool(false)) => Some(None),
        Some(Value::Bool(true)) => Some(Some("single".into())),
        Some(Value::String(s)) => {
            if matches!(
                s.as_str(),
                "single" | "double" | "singleAccounting" | "doubleAccounting" | "none"
            ) {
                if s == "none" {
                    Some(None)
                } else {
                    Some(Some(s.clone()))
                }
            } else {
                return Err(Error::with_path(
                    ErrorKind::InvalidCellStyle {
                        field: "underline".into(),
                    },
                    format!("ops[{i}].{prefix}.underline"),
                ));
            }
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::InvalidCellStyle {
                    field: "underline".into(),
                },
                format!("ops[{i}].{prefix}.underline"),
            ));
        }
    };
    let color = parse_opt_color(obj, i, prefix, "color")?;
    Ok(FontPatch {
        name,
        sz,
        bold,
        italic,
        underline,
        color,
    })
}

pub(super) fn parse_fill_patch(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    prefix: &str,
) -> Result<FillPatch, Error> {
    deny_sub_keys(obj, i, prefix, &["pattern", "fg", "bg"])?;
    let pattern = match obj.get("pattern") {
        None => None,
        Some(Value::String(s)) => {
            if s == "gradient" || !PATTERN_TYPES.contains(&s.as_str()) {
                return Err(Error::with_path(
                    ErrorKind::InvalidCellStyle {
                        field: "pattern".into(),
                    },
                    format!("ops[{i}].{prefix}.pattern"),
                ));
            }
            Some(s.clone())
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::InvalidCellStyle {
                    field: "pattern".into(),
                },
                format!("ops[{i}].{prefix}.pattern"),
            ));
        }
    };
    Ok(FillPatch {
        pattern,
        fg: parse_opt_color(obj, i, prefix, "fg")?,
        bg: parse_opt_color(obj, i, prefix, "bg")?,
    })
}

pub(super) fn parse_border_patch(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    prefix: &str,
) -> Result<BorderPatch, Error> {
    deny_sub_keys(
        obj,
        i,
        prefix,
        &[
            "style",
            "color",
            "left",
            "right",
            "top",
            "bottom",
            "diagonal",
            "diagonalUp",
            "diagonalDown",
        ],
    )?;
    Ok(BorderPatch {
        style: parse_opt_border_style(obj, i, prefix, "style")?,
        color: parse_opt_color(obj, i, prefix, "color")?,
        left: parse_opt_border_side(obj, i, prefix, "left")?,
        right: parse_opt_border_side(obj, i, prefix, "right")?,
        top: parse_opt_border_side(obj, i, prefix, "top")?,
        bottom: parse_opt_border_side(obj, i, prefix, "bottom")?,
        diagonal: parse_opt_border_side(obj, i, prefix, "diagonal")?,
        diagonal_up: parse_sub_bool(obj, i, prefix, "diagonalUp")?,
        diagonal_down: parse_sub_bool(obj, i, prefix, "diagonalDown")?,
    })
}

pub(super) fn parse_opt_border_side(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    prefix: &str,
    field: &str,
) -> Result<Option<BorderSidePatch>, Error> {
    let path = format!("{prefix}.{field}");
    match obj.get(field) {
        None => Ok(None),
        Some(Value::Object(m)) => {
            deny_sub_keys(m, i, &path, &["style", "color"])?;
            Ok(Some(BorderSidePatch {
                style: parse_opt_border_style(m, i, &path, "style")?,
                color: parse_opt_color(m, i, &path, "color")?,
            }))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Object,
            },
            format!("ops[{i}].{path}"),
        )),
    }
}

pub(super) fn parse_opt_border_style(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    prefix: &str,
    field: &str,
) -> Result<Option<String>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::String(s)) if BORDER_STYLES.contains(&s.as_str()) => Ok(Some(s.clone())),
        Some(_) => Err(Error::with_path(
            ErrorKind::InvalidCellStyle {
                field: "style".into(),
            },
            format!("ops[{i}].{prefix}.{field}"),
        )),
    }
}

pub(super) fn parse_align_patch(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    prefix: &str,
) -> Result<AlignPatch, Error> {
    deny_sub_keys(
        obj,
        i,
        prefix,
        &[
            "horizontal",
            "vertical",
            "wrap",
            "indent",
            "rotation",
            "shrink",
        ],
    )?;
    let horizontal = match obj.get("horizontal") {
        None => None,
        Some(Value::String(s)) if H_ALIGN.contains(&s.as_str()) => Some(s.clone()),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::InvalidCellStyle {
                    field: "horizontal".into(),
                },
                format!("ops[{i}].{prefix}.horizontal"),
            ));
        }
    };
    let vertical = match obj.get("vertical") {
        None => None,
        Some(Value::String(s)) if V_ALIGN.contains(&s.as_str()) => Some(s.clone()),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::InvalidCellStyle {
                    field: "vertical".into(),
                },
                format!("ops[{i}].{prefix}.vertical"),
            ));
        }
    };
    let indent = parse_opt_u32_field(obj, i, prefix, "indent")?;
    if let Some(n) = indent {
        if n > 250 {
            return Err(Error::with_path(
                ErrorKind::InvalidCellStyle {
                    field: "indent".into(),
                },
                format!("ops[{i}].{prefix}.indent"),
            ));
        }
    }
    let rotation = parse_opt_u32_field(obj, i, prefix, "rotation")?;
    if let Some(n) = rotation {
        if n > 180 {
            return Err(Error::with_path(
                ErrorKind::InvalidCellStyle {
                    field: "rotation".into(),
                },
                format!("ops[{i}].{prefix}.rotation"),
            ));
        }
    }
    Ok(AlignPatch {
        horizontal,
        vertical,
        wrap: parse_sub_bool(obj, i, prefix, "wrap")?,
        indent,
        rotation,
        shrink: parse_sub_bool(obj, i, prefix, "shrink")?,
    })
}

pub(super) fn parse_opt_color(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    prefix: &str,
    field: &str,
) -> Result<Option<ColorSpec>, Error> {
    let path = format!("{prefix}.{field}");
    match obj.get(field) {
        None => Ok(None),
        Some(Value::Object(m)) => Ok(Some(parse_color_spec(m, i, &path)?)),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Object,
            },
            format!("ops[{i}].{path}"),
        )),
    }
}

pub(super) fn parse_color_spec(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    prefix: &str,
) -> Result<ColorSpec, Error> {
    deny_sub_keys(obj, i, prefix, &["rgb", "theme", "indexed", "tint"])?;
    let rgb = match obj.get("rgb") {
        None => None,
        Some(Value::String(s)) if is_cell_rgb(s) => Some(s.to_ascii_uppercase()),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::InvalidCellStyle {
                    field: "color".into(),
                },
                format!("ops[{i}].{prefix}.rgb"),
            ));
        }
    };
    let theme = parse_opt_u32_field(obj, i, prefix, "theme")?;
    let indexed = parse_opt_u32_field(obj, i, prefix, "indexed")?;
    let tint = match obj.get("tint") {
        None => None,
        Some(Value::Number(n)) => n.as_f64(),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::InvalidCellStyle {
                    field: "tint".into(),
                },
                format!("ops[{i}].{prefix}.tint"),
            ));
        }
    };
    let kinds = [rgb.is_some(), theme.is_some(), indexed.is_some()]
        .into_iter()
        .filter(|b| *b)
        .count();
    if kinds != 1 || (tint.is_some() && theme.is_none()) {
        return Err(Error::with_path(
            ErrorKind::InvalidCellStyle {
                field: "color".into(),
            },
            format!("ops[{i}].{prefix}"),
        ));
    }
    Ok(ColorSpec {
        rgb,
        theme,
        indexed,
        tint,
    })
}

pub(super) fn is_cell_rgb(s: &str) -> bool {
    (s.len() == 6 || s.len() == 8) && s.chars().all(|c| c.is_ascii_hexdigit())
}

pub fn parse_cell_value(v: &Value, path: &str) -> Result<SetCellValue, Error> {
    match v {
        Value::Null => Ok(SetCellValue::Null),
        Value::Bool(b) => Ok(SetCellValue::Bool(*b)),
        Value::Number(n) => Ok(SetCellValue::Number(n.clone())),
        Value::String(s) => Ok(SetCellValue::Text(s.clone())),
        _ => Err(Error::with_path(ErrorKind::InvalidCellValue, path)),
    }
}

pub(super) fn parse_insert_cell_style(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<style::InsertCellStyleOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op", "id", "name", "numFmt", "font", "fill", "border", "align",
        ],
    )?;
    let id = require_id(obj, i)?;
    if id != "xlsx:workbook" {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::InsertCellStyleNeedsWorkbook,
            },
            format!("ops[{i}].id"),
        ));
    }
    let name = match obj.get("name") {
        Some(Value::String(s)) if !s.is_empty() => s.clone(),
        None => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "name".into(),
                },
                format!("ops[{i}].name"),
            ));
        }
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidNamedCellStyle {
                    field: "name".into(),
                },
                format!("ops[{i}].name"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "name".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].name"),
            ));
        }
    };
    let num_fmt = parse_patch_string(obj, i, "numFmt")?;
    let font = parse_patch_object(obj, i, "font", parse_font_patch)?;
    let fill = parse_patch_object(obj, i, "fill", parse_fill_patch)?;
    let border = parse_patch_object(obj, i, "border", parse_border_patch)?;
    let align = parse_patch_object(obj, i, "align", parse_align_patch)?;
    if num_fmt.is_absent()
        && font.is_absent()
        && fill.is_absent()
        && border.is_absent()
        && align.is_absent()
    {
        return Err(Error::with_path(
            ErrorKind::NeedInsertCellStyleField,
            format!("ops[{i}]"),
        ));
    }
    Ok(style::InsertCellStyleOp {
        name,
        num_fmt,
        font,
        fill,
        border,
        align,
    })
}

pub(super) fn parse_delete_cell_style(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<style::DeleteCellStyleOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let index = style::parse_cell_style_id(&id, &format!("ops[{i}].id"))?;
    Ok(style::DeleteCellStyleOp { index })
}

pub(super) fn parse_set_cell_named_style(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<style::SetCellNamedStyleOp, Error> {
    deny_keys(obj, i, &["op", "id", "style"])?;
    let id = require_id(obj, i)?;
    let style_id = match obj.get("style") {
        Some(Value::String(s)) if !s.is_empty() => s.clone(),
        None => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "style".into(),
                },
                format!("ops[{i}].style"),
            ));
        }
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidNamedCellStyle {
                    field: "style".into(),
                },
                format!("ops[{i}].style"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "style".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].style"),
            ));
        }
    };
    let style_index = style::parse_cell_style_id(&style_id, &format!("ops[{i}].style"))?;
    Ok(style::SetCellNamedStyleOp { id, style_index })
}
