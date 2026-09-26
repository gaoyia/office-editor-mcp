//! pptx 自己的 scope 与 ops 解析。

use serde_json::Value;

use crate::error::{Error, ErrorCode};
use crate::json;

#[derive(Debug, Clone)]
pub struct InspectScope {
    pub slide: u32,
    pub max_shapes: u32,
}

#[derive(Debug, Clone)]
pub struct SetShapeTextOp {
    pub id: String,
    pub text: String,
}

pub fn parse_scope(v: Option<&Value>) -> Result<Option<InspectScope>, Error> {
    let Some(v) = v else {
        return Ok(None);
    };
    let obj = v
        .as_object()
        .ok_or_else(|| Error::with_path(ErrorCode::InvalidField, "scope 必须是对象", "scope"))?;
    if obj.is_empty() {
        return Ok(None);
    }
    json::deny_unknown(obj, &["slide", "maxShapes"])?;
    let slide = obj.get("slide");
    let max = obj.get("maxShapes");
    match (slide, max) {
        (None, None) => Ok(None),
        (None, Some(_)) => Err(Error::with_path(
            ErrorCode::ScopeRequired,
            "看形状时必须给 slide",
            "scope.slide",
        )),
        (Some(s), m) => {
            let slide = s.as_u64().ok_or_else(|| {
                Error::with_path(ErrorCode::InvalidField, "slide 必须是整数", "scope.slide")
            })?;
            if slide < 1 {
                return Err(Error::with_path(
                    ErrorCode::InvalidField,
                    "slide 从 1 起",
                    "scope.slide",
                ));
            }
            let max = match m {
                None => 2000,
                Some(v) => {
                    let n = v.as_u64().ok_or_else(|| {
                        Error::with_path(
                            ErrorCode::InvalidField,
                            "maxShapes 必须是整数",
                            "scope.maxShapes",
                        )
                    })?;
                    if !(1..=5000).contains(&n) {
                        return Err(Error::with_path(
                            ErrorCode::TruncatedLimit,
                            "maxShapes 允许范围是 1 到 5000",
                            "scope.maxShapes",
                        ));
                    }
                    n
                }
            };
            Ok(Some(InspectScope {
                slide: slide as u32,
                max_shapes: max as u32,
            }))
        }
    }
}

pub fn parse_ops(ops: &[Value]) -> Result<Vec<SetShapeTextOp>, Error> {
    let mut out = Vec::new();
    for (i, op) in ops.iter().enumerate() {
        let obj = op.as_object().ok_or_else(|| {
            Error::with_path(
                ErrorCode::InvalidField,
                "ops 的每一项必须是对象",
                format!("ops[{i}]"),
            )
        })?;
        let op_name = obj.get("op").and_then(Value::as_str).ok_or_else(|| {
            Error::with_path(ErrorCode::InvalidField, "缺少 op", format!("ops[{i}].op"))
        })?;
        if op_name == "setCell" || op_name == "setParaText" {
            return Err(Error::with_path(
                ErrorCode::InvalidField,
                format!("pptx 不支持 {op_name}"),
                format!("ops[{i}].op"),
            ));
        }
        if op_name != "setShapeText" {
            return Err(Error::with_path(
                ErrorCode::UnsupportedOp,
                format!("不支持的 op：{op_name}"),
                format!("ops[{i}].op"),
            ));
        }
        for k in obj.keys() {
            if !matches!(k.as_str(), "op" | "id" | "text") {
                return Err(Error::with_path(
                    ErrorCode::InvalidField,
                    format!("不认识的字段：{k}"),
                    format!("ops[{i}].{k}"),
                ));
            }
        }
        let id = obj
            .get("id")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                Error::with_path(ErrorCode::InvalidField, "缺少 id", format!("ops[{i}].id"))
            })?;
        let text = match obj.get("text") {
            Some(Value::String(s)) => s.clone(),
            Some(_) => {
                return Err(Error::with_path(
                    ErrorCode::InvalidField,
                    "text 必须是字符串",
                    format!("ops[{i}].text"),
                ));
            }
            None => {
                return Err(Error::with_path(
                    ErrorCode::InvalidField,
                    "setShapeText 必须有 text",
                    format!("ops[{i}].text"),
                ));
            }
        };
        out.push(SetShapeTextOp {
            id: id.to_string(),
            text,
        });
    }
    Ok(out)
}
