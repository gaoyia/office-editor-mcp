//! docx 自己的 scope 与 ops 解析。

use serde_json::Value;

use crate::error::{Error, ErrorKind, ScopeNeed, TypeName};
use crate::json;

#[derive(Debug, Clone)]
pub struct InspectScope {
    pub from: u32,
    pub max_paras: u32,
}

#[derive(Debug, Clone)]
pub struct SetParaTextOp {
    pub id: String,
    pub text: String,
}

pub fn parse_scope(v: Option<&Value>) -> Result<Option<InspectScope>, Error> {
    let Some(v) = v else {
        return Ok(None);
    };
    let obj = v
        .as_object()
        .ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: "scope".into(),
                    expected: TypeName::Object,
                },
                "scope",
            )
        })?;
    if obj.is_empty() {
        return Ok(None);
    }
    json::deny_unknown(obj, &["from", "maxParas"])?;
    let from = obj.get("from");
    let max = obj.get("maxParas");
    match (from, max) {
        (None, None) => Ok(None),
        (Some(f), Some(m)) => {
            let from = f.as_u64().ok_or_else(|| {
                Error::with_path(
                    ErrorKind::WrongType {
                        field: "from".into(),
                        expected: TypeName::Integer,
                    },
                    "scope.from",
                )
            })?;
            let max = m.as_u64().ok_or_else(|| {
                Error::with_path(
                    ErrorKind::WrongType {
                        field: "maxParas".into(),
                        expected: TypeName::Integer,
                    },
                    "scope.maxParas",
                )
            })?;
            if from < 1 {
                return Err(Error::with_path(
                    ErrorKind::MinValue {
                        field: "from".into(),
                        min: 1,
                    },
                    "scope.from",
                ));
            }
            if !(1..=5000).contains(&max) {
                return Err(Error::with_path(
                    ErrorKind::OutOfRange {
                        field: "maxParas".into(),
                        min: 1,
                        max: 5000,
                    },
                    "scope.maxParas",
                ));
            }
            Ok(Some(InspectScope {
                from: from as u32,
                max_paras: max as u32,
            }))
        }
        _ => Err(Error::with_path(
            ErrorKind::ScopeRequired {
                need: ScopeNeed::FromAndMaxParas,
            },
            "scope",
        )),
    }
}

pub fn parse_ops(ops: &[Value]) -> Result<Vec<SetParaTextOp>, Error> {
    let mut out = Vec::new();
    for (i, op) in ops.iter().enumerate() {
        let obj = op.as_object().ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: format!("ops[{i}]"),
                    expected: TypeName::Object,
                },
                format!("ops[{i}]"),
            )
        })?;
        let op_name = obj.get("op").and_then(Value::as_str).ok_or_else(|| {
            Error::with_path(
                ErrorKind::MissingField {
                    field: "op".into(),
                },
                format!("ops[{i}].op"),
            )
        })?;
        if op_name == "setCell" || op_name == "setShapeText" {
            return Err(Error::with_path(
                ErrorKind::OpNotForFormat {
                    format: "docx".into(),
                    op: op_name.to_string(),
                },
                format!("ops[{i}].op"),
            ));
        }
        if op_name != "setParaText" {
            return Err(Error::with_path(
                ErrorKind::UnsupportedOp {
                    op: op_name.to_string(),
                },
                format!("ops[{i}].op"),
            ));
        }
        for k in obj.keys() {
            if !matches!(k.as_str(), "op" | "id" | "text") {
                return Err(Error::with_path(
                    ErrorKind::UnknownField { field: k.clone() },
                    format!("ops[{i}].{k}"),
                ));
            }
        }
        let id = obj
            .get("id")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                Error::with_path(
                    ErrorKind::MissingField {
                        field: "id".into(),
                    },
                    format!("ops[{i}].id"),
                )
            })?;
        let text = match obj.get("text") {
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
            None => {
                return Err(Error::with_path(
                    ErrorKind::OpRequiresField {
                        op: "setParaText".into(),
                        field: "text".into(),
                    },
                    format!("ops[{i}].text"),
                ));
            }
        };
        out.push(SetParaTextOp {
            id: id.to_string(),
            text,
        });
    }
    Ok(out)
}
