//! pptx request：common。

use serde_json::{Map, Value};

use crate::error::{Error, ErrorKind, TypeName};

use super::types::*;

pub(super) fn parse_bool_field(v: Option<&Value>, field: &str) -> Result<bool, Error> {
    match v {
        None => Ok(false),
        Some(Value::Bool(b)) => Ok(*b),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Boolean,
            },
            format!("scope.{field}"),
        )),
    }
}

pub(super) fn parse_opt_bool(
    obj: &Map<String, Value>,
    i: usize,
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
            format!("ops[{i}].{field}"),
        )),
    }
}

pub(super) fn parse_color_field(
    obj: &Map<String, Value>,
    i: usize,
    op: &str,
) -> Result<Option<String>, Error> {
    match obj.get("color") {
        None => Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: op.into(),
                field: "color".into(),
            },
            format!("ops[{i}].color"),
        )),
        Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => {
            if s.len() == 6 && s.chars().all(|c| c.is_ascii_hexdigit()) {
                Ok(Some(s.clone()))
            } else {
                Err(Error::with_path(
                    ErrorKind::InvalidHexColor,
                    format!("ops[{i}].color"),
                ))
            }
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "color".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].color"),
        )),
    }
}

pub(super) fn parse_optional_text(obj: &Map<String, Value>, i: usize) -> Result<String, Error> {
    match obj.get("text") {
        None => Ok(String::new()),
        Some(Value::String(s)) => Ok(s.clone()),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "text".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].text"),
        )),
    }
}

pub(super) fn parse_optional_date(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<Option<String>, Error> {
    match obj.get("date") {
        None => Ok(None),
        Some(Value::String(s)) if !s.is_empty() => Ok(Some(s.clone())),
        Some(Value::String(_)) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "date".into(),
                expected: TypeName::NonEmptyString,
            },
            format!("ops[{i}].date"),
        )),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "date".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].date"),
        )),
    }
}

pub(super) fn parse_required_bool(
    v: Option<&Value>,
    i: usize,
    op: &str,
    field: &str,
) -> Result<bool, Error> {
    match v {
        Some(Value::Bool(b)) => Ok(*b),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Boolean,
            },
            format!("ops[{i}].{field}"),
        )),
        None => Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: op.into(),
                field: field.into(),
            },
            format!("ops[{i}].{field}"),
        )),
    }
}

pub(super) fn deny_keys(obj: &Map<String, Value>, i: usize, allowed: &[&str]) -> Result<(), Error> {
    deny_keys_at(obj, &format!("ops[{i}]"), allowed)
}

pub(super) fn deny_keys_at(
    obj: &Map<String, Value>,
    prefix: &str,
    allowed: &[&str],
) -> Result<(), Error> {
    for k in obj.keys() {
        if !allowed.contains(&k.as_str()) {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("{prefix}.{k}"),
            ));
        }
    }
    Ok(())
}

pub(super) fn require_id(obj: &Map<String, Value>, i: usize) -> Result<String, Error> {
    obj.get("id")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .ok_or_else(|| {
            Error::with_path(
                ErrorKind::MissingField { field: "id".into() },
                format!("ops[{i}].id"),
            )
        })
}

pub(super) fn require_file(obj: &Map<String, Value>, i: usize, op: &str) -> Result<String, Error> {
    match obj.get("file") {
        Some(Value::String(s)) if !s.is_empty() => Ok(s.clone()),
        Some(Value::String(_)) | None => Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: op.into(),
                field: "file".into(),
            },
            format!("ops[{i}].file"),
        )),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "file".into(),
                expected: TypeName::NonEmptyString,
            },
            format!("ops[{i}].file"),
        )),
    }
}

pub(super) fn parse_emu_field(
    v: Option<&Value>,
    i: usize,
    field: &str,
) -> Result<Option<i64>, Error> {
    parse_emu_at(v, &format!("ops[{i}].{field}"), field)
}

pub(super) fn parse_emu_at(
    v: Option<&Value>,
    path: &str,
    field: &str,
) -> Result<Option<i64>, Error> {
    let Some(v) = v else {
        return Ok(None);
    };
    match v {
        Value::Number(n) => {
            if let Some(u) = n.as_u64() {
                if u == 0 || u > i64::MAX as u64 {
                    return Err(Error::with_path(
                        ErrorKind::MinValue {
                            field: field.into(),
                            min: 1,
                        },
                        path.to_string(),
                    ));
                }
                return Ok(Some(u as i64));
            }
            Err(Error::with_path(
                ErrorKind::WrongType {
                    field: field.into(),
                    expected: TypeName::Integer,
                },
                path.to_string(),
            ))
        }
        _ => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Integer,
            },
            path.to_string(),
        )),
    }
}

pub(super) fn parse_off_field(
    v: Option<&Value>,
    i: usize,
    field: &str,
) -> Result<Option<i64>, Error> {
    parse_off_at(v, &format!("ops[{i}].{field}"), field)
}

pub(super) fn parse_off_at(
    v: Option<&Value>,
    path: &str,
    field: &str,
) -> Result<Option<i64>, Error> {
    let Some(v) = v else {
        return Ok(None);
    };
    match v {
        Value::Number(n) => {
            if let Some(u) = n.as_u64() {
                if u > i64::MAX as u64 {
                    return Err(Error::with_path(
                        ErrorKind::WrongType {
                            field: field.into(),
                            expected: TypeName::Integer,
                        },
                        path,
                    ));
                }
                return Ok(Some(u as i64));
            }
            if let Some(i64v) = n.as_i64() {
                if i64v < 0 {
                    return Err(Error::with_path(
                        ErrorKind::MinValue {
                            field: field.into(),
                            min: 0,
                        },
                        path,
                    ));
                }
                return Ok(Some(i64v));
            }
            Err(Error::with_path(
                ErrorKind::WrongType {
                    field: field.into(),
                    expected: TypeName::Integer,
                },
                path,
            ))
        }
        _ => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Integer,
            },
            path,
        )),
    }
}

pub(super) fn parse_anim_sound(v: Option<&Value>, i: usize) -> Result<Option<String>, Error> {
    match v {
        None => Ok(None),
        Some(Value::String(s)) if !s.is_empty() => Ok(Some(s.clone())),
        Some(Value::String(_)) | Some(Value::Null) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "sound".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].sound"),
        )),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "sound".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].sound"),
        )),
    }
}

pub(super) fn parse_create_anim_host(
    obj: &Map<String, Value>,
    prefix: &str,
) -> Result<CreateAnimHost, Error> {
    let mut found: Vec<(&str, CreateAnimHost)> = Vec::new();
    for (field, make) in [
        (
            "shape",
            CreateAnimHost::Shape as fn(usize) -> CreateAnimHost,
        ),
        (
            "picture",
            CreateAnimHost::Picture as fn(usize) -> CreateAnimHost,
        ),
        (
            "video",
            CreateAnimHost::Video as fn(usize) -> CreateAnimHost,
        ),
        (
            "audio",
            CreateAnimHost::Audio as fn(usize) -> CreateAnimHost,
        ),
        (
            "table",
            CreateAnimHost::Table as fn(usize) -> CreateAnimHost,
        ),
        (
            "chart",
            CreateAnimHost::Chart as fn(usize) -> CreateAnimHost,
        ),
        (
            "smartArt",
            CreateAnimHost::SmartArt as fn(usize) -> CreateAnimHost,
        ),
    ] {
        if obj.contains_key(field) {
            let n = parse_create_index(obj.get(field), &format!("{prefix}.{field}"), field)?;
            found.push((field, make(n)));
        }
    }
    match found.len() {
        0 => Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "create".into(),
                field: "shape".into(),
            },
            prefix.to_string(),
        )),
        1 => Ok(found.remove(0).1),
        _ => Err(Error::with_path(
            ErrorKind::MutuallyExclusive {
                a: found[0].0.into(),
                b: found[1].0.into(),
            },
            prefix.to_string(),
        )),
    }
}

pub(super) fn parse_optional_slot(
    obj: &Map<String, Value>,
    i: usize,
    field: &str,
) -> Result<Option<String>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].{field}"),
        )),
    }
}

pub(super) fn require_name(obj: &Map<String, Value>, i: usize) -> Result<String, Error> {
    match obj.get("name") {
        Some(Value::String(s)) => Ok(s.clone()),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "name".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].name"),
        )),
        None => Err(Error::with_path(
            ErrorKind::MissingField {
                field: "name".into(),
            },
            format!("ops[{i}].name"),
        )),
    }
}

pub(super) fn parse_create_click_on(v: &Value, prefix: &str) -> Result<CreateAnimHost, Error> {
    let obj = v.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "clickOn".into(),
                expected: TypeName::Object,
            },
            prefix.to_string(),
        )
    })?;
    deny_keys_at(
        obj,
        prefix,
        &[
            "shape", "picture", "video", "audio", "table", "chart", "smartArt",
        ],
    )?;
    parse_create_anim_host(obj, prefix)
}

pub(super) fn parse_create_index(
    v: Option<&Value>,
    path: &str,
    field: &str,
) -> Result<usize, Error> {
    match v {
        Some(Value::Number(n)) => {
            let Some(u) = n.as_u64() else {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: field.into(),
                        expected: TypeName::Integer,
                    },
                    path.to_string(),
                ));
            };
            if u < 1 || u > usize::MAX as u64 {
                return Err(Error::with_path(
                    ErrorKind::MinValue {
                        field: field.into(),
                        min: 1,
                    },
                    path.to_string(),
                ));
            }
            Ok(u as usize)
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Integer,
            },
            path.to_string(),
        )),
        None => Err(Error::with_path(
            ErrorKind::MissingField {
                field: field.into(),
            },
            path.to_string(),
        )),
    }
}

pub(super) fn parse_create_link_fields(
    obj: &Map<String, Value>,
    prefix: &str,
    required: bool,
) -> Result<Option<CreateHyperlink>, Error> {
    let url = parse_create_link_string(obj, prefix, "url", ErrorKind::EmptyHyperlinkUrl)?;
    let file = parse_create_link_string(obj, prefix, "file", ErrorKind::EmptyHyperlinkFile)?;
    let slide = match obj.get("slide") {
        None => None,
        Some(Value::Null) => {
            return Err(Error::with_path(
                ErrorKind::PptxHyperlinkNeedTarget,
                format!("{prefix}.slide"),
            ));
        }
        Some(Value::Number(n)) => {
            let Some(u) = n.as_u64() else {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: "slide".into(),
                        expected: TypeName::Integer,
                    },
                    format!("{prefix}.slide"),
                ));
            };
            if u < 1 || u > usize::MAX as u64 {
                return Err(Error::with_path(
                    ErrorKind::MinValue {
                        field: "slide".into(),
                        min: 1,
                    },
                    format!("{prefix}.slide"),
                ));
            }
            Some(u as usize)
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "slide".into(),
                    expected: TypeName::Integer,
                },
                format!("{prefix}.slide"),
            ));
        }
    };
    let tooltip = match obj.get("tooltip") {
        None => None,
        Some(Value::String(s)) => Some(s.clone()),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "tooltip".into(),
                    expected: TypeName::String,
                },
                format!("{prefix}.tooltip"),
            ));
        }
    };
    let target = match (url, file, slide) {
        (Some(u), None, None) => Some(PptxLinkKind::Url(u)),
        (None, Some(f), None) => Some(PptxLinkKind::File(f)),
        (None, None, Some(n)) => Some(PptxLinkKind::Slide(n)),
        (None, None, None) => None,
        _ => {
            return Err(Error::with_path(
                ErrorKind::PptxHyperlinkNeedTarget,
                prefix.to_string(),
            ));
        }
    };
    let event = match obj.get("event") {
        None => LinkEvent::Click,
        Some(Value::String(s)) if s == "click" => LinkEvent::Click,
        Some(Value::String(s)) if s == "hover" => LinkEvent::Hover,
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidHyperlinkEvent,
                format!("{prefix}.event"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "event".into(),
                    expected: TypeName::String,
                },
                format!("{prefix}.event"),
            ));
        }
    };
    match target {
        Some(target) => Ok(Some(CreateHyperlink {
            target,
            tooltip,
            on_run: false,
            event,
        })),
        None => {
            if required || tooltip.is_some() || obj.contains_key("event") {
                return Err(Error::with_path(
                    ErrorKind::PptxHyperlinkNeedTarget,
                    prefix.to_string(),
                ));
            }
            Ok(None)
        }
    }
}

pub(super) fn parse_create_link_string(
    obj: &Map<String, Value>,
    prefix: &str,
    field: &str,
    empty: ErrorKind,
) -> Result<Option<String>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::Null) => Err(Error::with_path(empty, format!("{prefix}.{field}"))),
        Some(Value::String(s)) if s.is_empty() => {
            Err(Error::with_path(empty, format!("{prefix}.{field}")))
        }
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::String,
            },
            format!("{prefix}.{field}"),
        )),
    }
}
