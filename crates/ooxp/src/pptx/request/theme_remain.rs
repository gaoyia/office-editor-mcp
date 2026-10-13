//! pptx request：母版 theme 剩余字段（方案名、脚本字体、fmt、补充色板）。

use serde_json::{Map, Value};

use crate::error::{Error, ErrorKind, IdReason, TypeName};
use crate::theme::{ExtraScheme, FmtFillKind, ScriptFont};

use super::common::*;
use super::types::*;

pub(super) fn parse_opt_null_name(
    obj: &Map<String, Value>,
    i: usize,
    field: &str,
) -> Result<Option<Option<String>>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(None)),
        Some(Value::String(s)) if !s.is_empty() => Ok(Some(Some(s.clone()))),
        Some(Value::String(_)) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::NonEmptyString,
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

pub(super) fn parse_opt_scripts(
    obj: &Map<String, Value>,
    i: usize,
    field: &str,
) -> Result<Option<Vec<ScriptFont>>, Error> {
    parse_scripts_at(obj.get(field), &format!("ops[{i}].{field}"))
}

fn parse_scripts_at(v: Option<&Value>, path: &str) -> Result<Option<Vec<ScriptFont>>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(Vec::new())),
        Some(Value::Array(arr)) => {
            let mut out = Vec::with_capacity(arr.len());
            for (j, item) in arr.iter().enumerate() {
                out.push(parse_script_item(item, &format!("{path}[{j}]"))?);
            }
            Ok(Some(out))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "scripts".into(),
                expected: TypeName::Array,
            },
            path.to_string(),
        )),
    }
}

fn parse_script_item(v: &Value, path: &str) -> Result<ScriptFont, Error> {
    let obj = v.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "script font".into(),
                expected: TypeName::Object,
            },
            path.to_string(),
        )
    })?;
    for k in obj.keys() {
        if !matches!(k.as_str(), "script" | "typeface") {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("{path}.{k}"),
            ));
        }
    }
    let script = match obj.get("script") {
        Some(Value::String(s)) if !s.is_empty() => s.clone(),
        Some(Value::String(_)) | None => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "script".into(),
                    expected: TypeName::NonEmptyString,
                },
                format!("{path}.script"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "script".into(),
                    expected: TypeName::String,
                },
                format!("{path}.script"),
            ));
        }
    };
    let typeface = match obj.get("typeface") {
        Some(Value::String(s)) => s.clone(),
        None => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "typeface".into(),
                },
                format!("{path}.typeface"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "typeface".into(),
                    expected: TypeName::String,
                },
                format!("{path}.typeface"),
            ));
        }
    };
    Ok(ScriptFont { script, typeface })
}

pub(super) fn parse_set_theme_fmt(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<SetThemeFmtOp, Error> {
    deny_keys(obj, i, &["op", "id", "line", "fill", "bgFill", "name"])?;
    let id = require_id(obj, i)?;
    let master = super::super::master::parse_master_only_id(&id, i, IdReason::SetThemeNeedsMaster)?;
    let line = parse_fmt_line(obj.get("line"), i, "line")?;
    let fill = parse_fmt_fill(obj.get("fill"), i, "fill")?;
    let bg_fill = parse_fmt_fill(obj.get("bgFill"), i, "bgFill")?;
    let name = parse_opt_null_name(obj, i, "name")?;
    if line.is_none() && fill.is_none() && bg_fill.is_none() && name.is_none() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setThemeFmt".into(),
                field: "line, fill, bgFill, or name".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(SetThemeFmtOp {
        master,
        line,
        fill,
        bg_fill,
        name,
    })
}

pub(super) fn parse_set_theme_extra_schemes(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<SetThemeExtraSchemesOp, Error> {
    deny_keys(obj, i, &["op", "id", "extraSchemes"])?;
    let id = require_id(obj, i)?;
    let master = super::super::master::parse_master_only_id(&id, i, IdReason::SetThemeNeedsMaster)?;
    let schemes = match obj.get("extraSchemes") {
        None => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "extraSchemes".into(),
                },
                format!("ops[{i}].extraSchemes"),
            ));
        }
        Some(Value::Null) => Vec::new(),
        Some(Value::Array(arr)) => {
            parse_extra_scheme_array(arr, &format!("ops[{i}].extraSchemes"))?
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "extraSchemes".into(),
                    expected: TypeName::Array,
                },
                format!("ops[{i}].extraSchemes"),
            ));
        }
    };
    Ok(SetThemeExtraSchemesOp { master, schemes })
}

fn parse_fmt_line(v: Option<&Value>, i: usize, field: &str) -> Result<Option<(usize, u32)>, Error> {
    let Some(v) = v else {
        return Ok(None);
    };
    let obj = v.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Object,
            },
            format!("ops[{i}].{field}"),
        )
    })?;
    for k in obj.keys() {
        if !matches!(k.as_str(), "index" | "w") {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("ops[{i}].{field}.{k}"),
            ));
        }
    }
    let index = parse_fmt_index(obj.get("index"), i, &format!("{field}.index"))?;
    let w = parse_fmt_w(obj.get("w"), i, &format!("{field}.w"))?;
    Ok(Some((index, w)))
}

fn parse_fmt_fill(
    v: Option<&Value>,
    i: usize,
    field: &str,
) -> Result<Option<(usize, FmtFillKind)>, Error> {
    let Some(v) = v else {
        return Ok(None);
    };
    let obj = v.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Object,
            },
            format!("ops[{i}].{field}"),
        )
    })?;
    for k in obj.keys() {
        if !matches!(k.as_str(), "index" | "kind") {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("ops[{i}].{field}.{k}"),
            ));
        }
    }
    let index = parse_fmt_index(obj.get("index"), i, &format!("{field}.index"))?;
    let kind = match obj.get("kind") {
        Some(Value::String(s)) if s == "solid" => FmtFillKind::Solid,
        Some(Value::String(s)) if s == "grad" => FmtFillKind::Grad,
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidThemeFmtKind,
                format!("ops[{i}].{field}.kind"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: format!("{field}.kind"),
                    expected: TypeName::String,
                },
                format!("ops[{i}].{field}.kind"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::OpRequiresField {
                    op: "setThemeFmt".into(),
                    field: "kind".into(),
                },
                format!("ops[{i}].{field}.kind"),
            ));
        }
    };
    Ok(Some((index, kind)))
}

fn parse_fmt_index(v: Option<&Value>, i: usize, field: &str) -> Result<usize, Error> {
    match v {
        Some(Value::Number(n)) => {
            if let Some(u) = n.as_u64() {
                if u <= 2 {
                    return Ok(u as usize);
                }
                return Err(Error::with_path(
                    ErrorKind::InvalidThemeFmtIndex,
                    format!("ops[{i}].{field}"),
                ));
            }
            Err(Error::with_path(
                ErrorKind::WrongType {
                    field: field.into(),
                    expected: TypeName::Integer,
                },
                format!("ops[{i}].{field}"),
            ))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Integer,
            },
            format!("ops[{i}].{field}"),
        )),
        None => Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setThemeFmt".into(),
                field: "index".into(),
            },
            format!("ops[{i}].{field}"),
        )),
    }
}

fn parse_fmt_w(v: Option<&Value>, i: usize, field: &str) -> Result<u32, Error> {
    match v {
        Some(Value::Number(n)) => {
            if let Some(u) = n.as_u64() {
                if u > u32::MAX as u64 {
                    return Err(Error::with_path(
                        ErrorKind::MinValue {
                            field: field.into(),
                            min: 0,
                        },
                        format!("ops[{i}].{field}"),
                    ));
                }
                return Ok(u as u32);
            }
            if n.as_i64().is_some_and(|x| x < 0) {
                return Err(Error::with_path(
                    ErrorKind::MinValue {
                        field: field.into(),
                        min: 0,
                    },
                    format!("ops[{i}].{field}"),
                ));
            }
            Err(Error::with_path(
                ErrorKind::WrongType {
                    field: field.into(),
                    expected: TypeName::Integer,
                },
                format!("ops[{i}].{field}"),
            ))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Integer,
            },
            format!("ops[{i}].{field}"),
        )),
        None => Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setThemeFmt".into(),
                field: "w".into(),
            },
            format!("ops[{i}].{field}"),
        )),
    }
}

fn parse_extra_scheme_array(arr: &[Value], path: &str) -> Result<Vec<ExtraScheme>, Error> {
    let mut out = Vec::with_capacity(arr.len());
    for (j, item) in arr.iter().enumerate() {
        out.push(parse_extra_scheme_item(item, &format!("{path}[{j}]"))?);
    }
    Ok(out)
}

fn parse_extra_scheme_item(v: &Value, path: &str) -> Result<ExtraScheme, Error> {
    let obj = v.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "extraSchemes[]".into(),
                expected: TypeName::Object,
            },
            path.to_string(),
        )
    })?;
    for k in obj.keys() {
        if !matches!(k.as_str(), "name" | "colors") {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("{path}.{k}"),
            ));
        }
    }
    let item_name = parse_scheme_name(obj.get("name"), &format!("{path}.name"))?;
    let (colors_name, slots) = match obj.get("colors") {
        None => (None, Vec::new()),
        Some(Value::Object(m)) => {
            let parsed = parse_extra_colors(m, &format!("{path}.colors"))?;
            (parsed.name, parsed.slots)
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "colors".into(),
                    expected: TypeName::Object,
                },
                format!("{path}.colors"),
            ));
        }
    };
    let name = match (item_name, colors_name) {
        (Some(a), Some(b)) if a != b => {
            return Err(Error::with_path(
                ErrorKind::MutuallyExclusive {
                    a: "name".into(),
                    b: "colors.name".into(),
                },
                format!("{path}.name"),
            ));
        }
        (Some(a), _) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    };
    Ok(ExtraScheme { name, slots })
}

fn parse_scheme_name(v: Option<&Value>, path: &str) -> Result<Option<String>, Error> {
    match v {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) if !s.is_empty() => Ok(Some(s.clone())),
        Some(Value::String(_)) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "name".into(),
                expected: TypeName::NonEmptyString,
            },
            path.to_string(),
        )),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "name".into(),
                expected: TypeName::String,
            },
            path.to_string(),
        )),
    }
}

struct NamedColorSlots {
    name: Option<String>,
    slots: Vec<(String, String)>,
}

fn parse_extra_colors(obj: &Map<String, Value>, path: &str) -> Result<NamedColorSlots, Error> {
    for k in obj.keys() {
        if k != "name" && !crate::theme::COLOR_SLOTS.contains(&k.as_str()) {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("{path}.{k}"),
            ));
        }
    }
    let name = parse_scheme_name(obj.get("name"), &format!("{path}.name"))?;
    let mut slots = Vec::new();
    for slot in crate::theme::COLOR_SLOTS {
        if let Some(v) = obj.get(*slot) {
            slots.push((
                (*slot).to_string(),
                parse_hex_at(v, &format!("{path}.{slot}"))?,
            ));
        }
    }
    Ok(NamedColorSlots { name, slots })
}

fn parse_hex_at(v: &Value, path: &str) -> Result<String, Error> {
    match v {
        Value::String(s) if s.len() == 6 && s.chars().all(|c| c.is_ascii_hexdigit()) => {
            Ok(s.clone())
        }
        Value::String(_) => Err(Error::with_path(
            ErrorKind::InvalidHexColor,
            path.to_string(),
        )),
        _ => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "color".into(),
                expected: TypeName::String,
            },
            path.to_string(),
        )),
    }
}
