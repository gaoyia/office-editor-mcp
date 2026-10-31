use serde_json::{Map, Value};

use crate::error::{Error, ErrorKind, TypeName};

use super::super::a1::{self, CellRef};
use super::super::picture::AnchorKind;

pub(crate) fn parse_object_array<'a>(
    v: Option<&'a Value>,
    field: &str,
    prefix: &str,
) -> Result<Option<&'a [Value]>, Error> {
    let Some(v) = v else {
        return Ok(None);
    };
    let arr = v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Array,
            },
            prefix.to_string(),
        )
    })?;
    Ok(Some(arr.as_slice()))
}

pub(crate) fn deny_unknown(
    obj: &Map<String, Value>,
    prefix: &str,
    allowed: &[&str],
) -> Result<(), Error> {
    for k in obj.keys() {
        if !allowed.iter().any(|a| *a == k) {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("{prefix}.{k}"),
            ));
        }
    }
    Ok(())
}

pub(crate) fn require_object<'a>(
    v: &'a Value,
    field: &str,
    prefix: &str,
) -> Result<&'a Map<String, Value>, Error> {
    v.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Object,
            },
            prefix.to_string(),
        )
    })
}

pub(crate) struct CreateAnchor {
    pub(crate) from: CellRef,
    pub(crate) to: Option<CellRef>,
    pub(crate) anchor: AnchorKind,
    pub(crate) from_off_x: i64,
    pub(crate) from_off_y: i64,
    pub(crate) to_off_x: i64,
    pub(crate) to_off_y: i64,
    pub(crate) cx: Option<i64>,
    pub(crate) cy: Option<i64>,
}

pub(crate) fn parse_create_anchor(
    obj: &Map<String, Value>,
    prefix: &str,
) -> Result<CreateAnchor, Error> {
    let from = parse_required_a1_field(obj.get("from"), prefix, "from")?;
    let cx = parse_emu(obj.get("cx"), prefix, "cx")?;
    let cy = parse_emu(obj.get("cy"), prefix, "cy")?;
    if cx.is_some() != cy.is_some() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "create".into(),
                field: "cx and cy together".into(),
            },
            prefix.to_string(),
        ));
    }
    let anchor = match obj.get("anchor") {
        None => AnchorKind::OneCell,
        Some(Value::String(s)) => AnchorKind::parse_insert(s).ok_or_else(|| {
            Error::with_path(ErrorKind::InvalidXlsxAnchor, format!("{prefix}.anchor"))
        })?,
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "anchor".into(),
                    expected: TypeName::String,
                },
                format!("{prefix}.anchor"),
            ));
        }
    };
    let to = match obj.get("to") {
        None => None,
        Some(Value::String(s)) => {
            Some(a1::parse_cell(s).map_err(|e| e.at(format!("{prefix}.to")))?)
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "to".into(),
                    expected: TypeName::String,
                },
                format!("{prefix}.to"),
            ));
        }
    };
    if anchor == AnchorKind::TwoCell && to.is_none() {
        return Err(Error::with_path(
            ErrorKind::MissingField { field: "to".into() },
            format!("{prefix}.to"),
        ));
    }
    if anchor == AnchorKind::OneCell && to.is_some() {
        return Err(Error::with_path(
            ErrorKind::UnknownField { field: "to".into() },
            format!("{prefix}.to"),
        ));
    }
    Ok(CreateAnchor {
        from,
        to,
        anchor,
        from_off_x: parse_off(obj.get("fromOffX"), prefix, "fromOffX")?.unwrap_or(0),
        from_off_y: parse_off(obj.get("fromOffY"), prefix, "fromOffY")?.unwrap_or(0),
        to_off_x: parse_off(obj.get("toOffX"), prefix, "toOffX")?.unwrap_or(0),
        to_off_y: parse_off(obj.get("toOffY"), prefix, "toOffY")?.unwrap_or(0),
        cx,
        cy,
    })
}

pub(crate) fn parse_opt_text_field(
    obj: &Map<String, Value>,
    prefix: &str,
) -> Result<Option<String>, Error> {
    match obj.get("text") {
        None => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "text".into(),
                expected: TypeName::String,
            },
            format!("{prefix}.text"),
        )),
    }
}

pub(crate) fn parse_required_a1_field(
    v: Option<&Value>,
    prefix: &str,
    field: &str,
) -> Result<CellRef, Error> {
    match v {
        Some(Value::String(s)) => a1::parse_cell(s).map_err(|e| e.at(format!("{prefix}.{field}"))),
        None => Err(Error::with_path(
            ErrorKind::MissingField {
                field: field.into(),
            },
            format!("{prefix}.{field}"),
        )),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::String,
            },
            format!("{prefix}.{field}"),
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

pub(crate) fn parse_off(
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
