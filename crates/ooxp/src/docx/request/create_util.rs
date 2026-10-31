use serde_json::Value;

use crate::error::{Error, ErrorKind, TypeName};

use super::super::picture::{self};

pub(crate) fn parse_align_h(
    v: Option<&Value>,
    item_path: &str,
) -> Result<Option<picture::AlignH>, Error> {
    match v {
        None => Ok(None),
        Some(Value::String(s)) => picture::parse_align_h(s).map(Some).ok_or_else(|| {
            Error::with_path(ErrorKind::InvalidPicAlign, format!("{item_path}.alignH"))
        }),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "alignH".into(),
                expected: TypeName::String,
            },
            format!("{item_path}.alignH"),
        )),
    }
}

pub(crate) fn parse_align_v(
    v: Option<&Value>,
    item_path: &str,
) -> Result<Option<picture::AlignV>, Error> {
    match v {
        None => Ok(None),
        Some(Value::String(s)) => picture::parse_align_v(s).map(Some).ok_or_else(|| {
            Error::with_path(ErrorKind::InvalidPicAlign, format!("{item_path}.alignV"))
        }),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "alignV".into(),
                expected: TypeName::String,
            },
            format!("{item_path}.alignV"),
        )),
    }
}

pub(crate) fn parse_emu(
    v: Option<&Value>,
    prefix: &str,
    field: &str,
) -> Result<Option<i64>, Error> {
    let Some(v) = v else {
        return Ok(None);
    };
    let path = format!("{prefix}.{field}");
    match v {
        Value::Number(n) => {
            if let Some(u) = n.as_u64() {
                if u == 0 || u > i64::MAX as u64 {
                    return Err(Error::with_path(
                        ErrorKind::MinValue {
                            field: field.into(),
                            min: 1,
                        },
                        path,
                    ));
                }
                return Ok(Some(u as i64));
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

pub(crate) fn parse_i64(
    v: Option<&Value>,
    prefix: &str,
    field: &str,
) -> Result<Option<i64>, Error> {
    let Some(v) = v else {
        return Ok(None);
    };
    let path = format!("{prefix}.{field}");
    match v {
        Value::Number(n) => n.as_i64().map(Some).ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: field.into(),
                    expected: TypeName::Integer,
                },
                path,
            )
        }),
        _ => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Integer,
            },
            path,
        )),
    }
}

pub(crate) fn canonicalize_style(s: &str, i: usize) -> Result<Option<&'static str>, Error> {
    if s.eq_ignore_ascii_case("Normal") {
        return Ok(None);
    }
    for c in ["Heading1", "Heading2", "Heading3"] {
        if s.eq_ignore_ascii_case(c) {
            return Ok(Some(c));
        }
    }
    Err(Error::with_path(
        ErrorKind::InvalidParaStyle,
        format!("document.paragraphs[{i}].style"),
    ))
}

pub(crate) fn parse_optional_str(
    v: Option<&Value>,
    path: &str,
    field: &str,
) -> Result<Option<String>, Error> {
    match v {
        None => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::String,
            },
            format!("{path}.{field}"),
        )),
    }
}

pub(crate) fn parse_optional_bool(
    v: Option<&Value>,
    path: &str,
    field: &str,
) -> Result<Option<bool>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Bool(b)) => Ok(Some(*b)),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Boolean,
            },
            format!("{path}.{field}"),
        )),
    }
}

pub(crate) fn parse_comment_attr(
    v: Option<&Value>,
    path: &str,
    field: &str,
) -> Result<Option<String>, Error> {
    match v {
        None => Ok(None),
        Some(Value::String(s)) if !s.is_empty() => Ok(Some(s.clone())),
        Some(Value::String(_)) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::NonEmptyString,
            },
            format!("{path}.{field}"),
        )),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::String,
            },
            format!("{path}.{field}"),
        )),
    }
}

pub(crate) fn parse_datum_list(v: Option<&Value>, path: &str) -> Result<Option<Vec<Value>>, Error> {
    match v {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Array(a)) => {
            let mut out = Vec::new();
            for (j, item) in a.iter().enumerate() {
                match item {
                    Value::String(_) | Value::Number(_) => out.push(item.clone()),
                    _ => {
                        return Err(Error::with_path(
                            ErrorKind::WrongType {
                                field: format!("[{j}]"),
                                expected: TypeName::String,
                            },
                            format!("{path}[{j}]"),
                        ));
                    }
                }
            }
            Ok(Some(out))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "array".into(),
                expected: TypeName::Array,
            },
            path.to_string(),
        )),
    }
}

pub(crate) fn parse_index1(v: Option<&Value>, path: &str) -> Result<Option<u32>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Number(n)) => {
            let Some(u) = n.as_u64() else {
                if n.as_i64().is_some_and(|x| x < 1) {
                    return Err(Error::with_path(
                        ErrorKind::MinValue {
                            field: path.rsplit('.').next().unwrap_or("index").into(),
                            min: 1,
                        },
                        path,
                    ));
                }
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: path.rsplit('.').next().unwrap_or("index").into(),
                        expected: TypeName::Integer,
                    },
                    path,
                ));
            };
            if u == 0 {
                return Err(Error::with_path(
                    ErrorKind::MinValue {
                        field: path.rsplit('.').next().unwrap_or("index").into(),
                        min: 1,
                    },
                    path,
                ));
            }
            if u > u32::MAX as u64 {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: path.rsplit('.').next().unwrap_or("index").into(),
                        expected: TypeName::Integer,
                    },
                    path,
                ));
            }
            Ok(Some(u as u32))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: path.rsplit('.').next().unwrap_or("index").into(),
                expected: TypeName::Integer,
            },
            path,
        )),
    }
}
