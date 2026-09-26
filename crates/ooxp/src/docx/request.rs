//! docx 自己的 scope 与 ops 解析。

use serde_json::Value;

use crate::error::{Error, ErrorCode};
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
        .ok_or_else(|| Error::with_path(ErrorCode::InvalidField, "scope 必须是对象", "scope"))?;
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
                Error::with_path(ErrorCode::InvalidField, "from 必须是整数", "scope.from")
            })?;
            let max = m.as_u64().ok_or_else(|| {
                Error::with_path(
                    ErrorCode::InvalidField,
                    "maxParas 必须是整数",
                    "scope.maxParas",
                )
            })?;
            if from < 1 {
                return Err(Error::with_path(
                    ErrorCode::InvalidField,
                    "from 从 1 起",
                    "scope.from",
                ));
            }
            if !(1..=5000).contains(&max) {
                return Err(Error::with_path(
                    ErrorCode::TruncatedLimit,
                    "maxParas 允许范围是 1 到 5000",
                    "scope.maxParas",
                ));
            }
            Ok(Some(InspectScope {
                from: from as u32,
                max_paras: max as u32,
            }))
        }
        _ => Err(Error::with_path(
            ErrorCode::ScopeRequired,
            "看段落正文时必须同时给 from 和 maxParas",
            "scope",
        )),
    }
}

pub fn parse_ops(ops: &[Value]) -> Result<Vec<SetParaTextOp>, Error> {
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
        if op_name == "setCell" || op_name == "setShapeText" {
            return Err(Error::with_path(
                ErrorCode::InvalidField,
                format!("docx 不支持 {op_name}"),
                format!("ops[{i}].op"),
            ));
        }
        if op_name != "setParaText" {
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
                    "setParaText 必须有 text",
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
