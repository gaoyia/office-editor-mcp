use serde_json::Value;

use crate::error::{Error, ErrorKind, TypeName};

use super::super::field;
use super::super::numbering::NumberingKind;
use super::create_types::*;

pub(crate) fn parse_fields(v: Option<&Value>, prefix: &str) -> Result<Vec<NewField>, Error> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    let path = format!("{prefix}.fields");
    let arr = v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "fields".into(),
                expected: TypeName::Array,
            },
            path.clone(),
        )
    })?;
    let mut out = Vec::new();
    for (j, item) in arr.iter().enumerate() {
        let item_path = format!("{path}[{j}]");
        let obj = item.as_object().ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: item_path.clone(),
                    expected: TypeName::Object,
                },
                item_path.clone(),
            )
        })?;
        for k in obj.keys() {
            if !matches!(
                k.as_str(),
                "instr" | "result" | "code" | "arg" | "name" | "items" | "checked"
            ) {
                return Err(Error::with_path(
                    ErrorKind::UnknownField { field: k.clone() },
                    format!("{item_path}.{k}"),
                ));
            }
        }
        let body = super::super::field::parse_field_body(obj, &item_path)?;
        out.push(NewField {
            instr: body.instr,
            result: body.result,
            form: body.form,
        });
    }
    Ok(out)
}

pub(crate) fn parse_numberings(v: Option<&Value>, path: &str) -> Result<Vec<NumberingKind>, Error> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    let arr = v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "numberings".into(),
                expected: TypeName::Array,
            },
            path,
        )
    })?;
    let mut out = Vec::new();
    for (i, item) in arr.iter().enumerate() {
        let item_path = format!("{path}[{i}]");
        let obj = item.as_object().ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: item_path.clone(),
                    expected: TypeName::Object,
                },
                item_path.clone(),
            )
        })?;
        for k in obj.keys() {
            if k.as_str() != "kind" {
                return Err(Error::with_path(
                    ErrorKind::UnknownField { field: k.clone() },
                    format!("{item_path}.{k}"),
                ));
            }
        }
        let kind = match obj.get("kind") {
            Some(Value::String(s)) => super::super::numbering::parse_kind(s).ok_or_else(|| {
                Error::with_path(ErrorKind::InvalidNumberingKind, format!("{item_path}.kind"))
            })?,
            Some(_) => {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: "kind".into(),
                        expected: TypeName::String,
                    },
                    format!("{item_path}.kind"),
                ));
            }
            None => {
                return Err(Error::with_path(
                    ErrorKind::MissingField {
                        field: "kind".into(),
                    },
                    format!("{item_path}.kind"),
                ));
            }
        };
        out.push(kind);
    }
    Ok(out)
}

pub(crate) fn parse_tocs(
    v: Option<&Value>,
    path: &str,
    para_n: usize,
) -> Result<Vec<NewToc>, Error> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    let arr = v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "tocs".into(),
                expected: TypeName::Array,
            },
            path,
        )
    })?;
    let mut out = Vec::new();
    for (i, item) in arr.iter().enumerate() {
        let item_path = format!("{path}[{i}]");
        let obj = item.as_object().ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: item_path.clone(),
                    expected: TypeName::Object,
                },
                item_path.clone(),
            )
        })?;
        for k in obj.keys() {
            if !matches!(k.as_str(), "kind" | "instr" | "after") {
                return Err(Error::with_path(
                    ErrorKind::UnknownField { field: k.clone() },
                    format!("{item_path}.{k}"),
                ));
            }
        }
        let kind = match obj.get("kind") {
            None => None,
            Some(Value::String(s)) => Some(
                field::parse_toc_kind(s)
                    .map_err(|kind| Error::with_path(kind, format!("{item_path}.kind")))?,
            ),
            Some(_) => {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: "kind".into(),
                        expected: TypeName::String,
                    },
                    format!("{item_path}.kind"),
                ));
            }
        };
        let instr = match obj.get("instr") {
            None => None,
            Some(Value::String(s)) => {
                if s.trim().is_empty() {
                    return Err(Error::with_path(
                        ErrorKind::EmptyFieldInstr,
                        format!("{item_path}.instr"),
                    ));
                }
                if !field::instr_is_toc_or_toa(s) {
                    return Err(Error::with_path(
                        ErrorKind::InvalidTocKind,
                        format!("{item_path}.instr"),
                    ));
                }
                Some(s.clone())
            }
            Some(_) => {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: "instr".into(),
                        expected: TypeName::String,
                    },
                    format!("{item_path}.instr"),
                ));
            }
        };
        let instr = match instr {
            Some(s) => s,
            None => kind
                .unwrap_or(field::TocKind::Toc)
                .default_instr()
                .to_string(),
        };
        let after =
            super::create_util::parse_index1(obj.get("after"), &format!("{item_path}.after"))?;
        if let Some(n) = after {
            if n as usize > para_n {
                return Err(Error::with_path(
                    ErrorKind::CreateIndexOutOfRange {
                        field: "after".into(),
                        max: para_n as i64,
                    },
                    format!("{item_path}.after"),
                ));
            }
        }
        out.push(NewToc { after, instr });
    }
    Ok(out)
}
