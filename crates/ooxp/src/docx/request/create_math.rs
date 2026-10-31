use serde_json::Value;

use crate::error::{Error, ErrorKind, TypeName};

use super::super::math::MathKind;
use super::super::math_expr::{self};
use super::create_types::*;

pub(crate) fn parse_maths(v: Option<&Value>, prefix: &str) -> Result<Vec<NewMath>, Error> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    let path = format!("{prefix}.maths");
    let arr = v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "maths".into(),
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
            if !matches!(k.as_str(), "text" | "kind" | "expr" | "jc") {
                return Err(Error::with_path(
                    ErrorKind::UnknownField { field: k.clone() },
                    format!("{item_path}.{k}"),
                ));
            }
        }
        let body = math_expr::parse_math_body(obj, &item_path)?;
        let kind = match obj.get("kind") {
            None => MathKind::Inline,
            Some(Value::String(s)) if s == "inline" => MathKind::Inline,
            Some(Value::String(s)) if s == "display" => MathKind::Display,
            Some(Value::String(_)) => {
                return Err(Error::with_path(
                    ErrorKind::InvalidMathKind,
                    format!("{item_path}.kind"),
                ));
            }
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
        let jc = match obj.get("jc") {
            None => None,
            Some(Value::String(s)) if super::super::math::MATH_JC.contains(&s.as_str()) => {
                Some(s.clone())
            }
            Some(Value::String(_)) => {
                return Err(Error::with_path(
                    ErrorKind::InvalidMathJc,
                    format!("{item_path}.jc"),
                ));
            }
            Some(_) => {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: "jc".into(),
                        expected: TypeName::String,
                    },
                    format!("{item_path}.jc"),
                ));
            }
        };
        if jc.is_some() && kind == MathKind::Inline {
            return Err(Error::with_path(
                ErrorKind::MathJcOnInline,
                format!("{item_path}.jc"),
            ));
        }
        out.push(NewMath { body, kind, jc });
    }
    Ok(out)
}
