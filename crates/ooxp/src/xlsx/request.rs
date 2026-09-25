//! xlsx 自己的 scope 与 ops 解析。

use serde_json::Value;

use crate::error::{Error, ErrorCode};
use crate::json;

#[derive(Debug, Clone)]
pub struct InspectScope {
    pub sheet: String,
    pub range: String,
    pub max_cells: u32,
}

#[derive(Debug, Clone)]
pub enum SetCellValue {
    Number(serde_json::Number),
    Text(String),
    Bool(bool),
    Null,
}

#[derive(Debug, Clone)]
pub struct SetCellOp {
    pub id: String,
    pub value: Option<SetCellValue>,
    pub formula: Option<String>,
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
    json::deny_unknown(obj, &["sheet", "range", "maxCells"])?;
    let sheet = obj.get("sheet").and_then(Value::as_str);
    let range = obj.get("range").and_then(Value::as_str);
    let max_cells = match obj.get("maxCells") {
        None => None,
        Some(Value::Number(n)) => n.as_u64(),
        Some(_) => {
            return Err(Error::with_path(
                ErrorCode::InvalidField,
                "maxCells 必须是整数",
                "scope.maxCells",
            ));
        }
    };
    match (sheet, range, max_cells) {
        (None, None, None) => Ok(None),
        (Some(s), Some(r), m) => {
            let max = m.unwrap_or(5000);
            if !(1..=20000).contains(&max) {
                return Err(Error::with_path(
                    ErrorCode::TruncatedLimit,
                    "maxCells 允许范围是 1 到 20000",
                    "scope.maxCells",
                ));
            }
            Ok(Some(InspectScope {
                sheet: s.to_string(),
                range: r.to_string(),
                max_cells: max as u32,
            }))
        }
        _ => Err(Error::with_path(
            ErrorCode::ScopeRequired,
            "看单元格时必须同时给 sheet 和 range",
            "scope",
        )),
    }
}

pub fn parse_ops(ops: &[Value]) -> Result<Vec<SetCellOp>, Error> {
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
        if op_name == "setParaText" {
            return Err(Error::with_path(
                ErrorCode::InvalidField,
                "xlsx 不支持 setParaText",
                format!("ops[{i}].op"),
            ));
        }
        if op_name != "setCell" {
            return Err(Error::with_path(
                ErrorCode::UnsupportedOp,
                format!("不支持的 op：{op_name}"),
                format!("ops[{i}].op"),
            ));
        }
        for k in obj.keys() {
            if !matches!(k.as_str(), "op" | "id" | "value" | "formula") {
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
        let has_value = obj.contains_key("value");
        let has_formula = obj.contains_key("formula");
        if has_value && has_formula {
            return Err(Error::with_path(
                ErrorCode::InvalidField,
                "value 和 formula 不能同时出现",
                format!("ops[{i}]"),
            ));
        }
        if !has_value && !has_formula {
            return Err(Error::with_path(
                ErrorCode::InvalidField,
                "setCell 必须有 value 或 formula",
                format!("ops[{i}]"),
            ));
        }
        let formula = if has_formula {
            let f = obj.get("formula").and_then(Value::as_str).ok_or_else(|| {
                Error::with_path(
                    ErrorCode::InvalidField,
                    "formula 必须是字符串",
                    format!("ops[{i}].formula"),
                )
            })?;
            if !f.starts_with('=') {
                return Err(Error::with_path(
                    ErrorCode::InvalidField,
                    "formula 必须以 = 开头",
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
        out.push(SetCellOp {
            id: id.to_string(),
            value,
            formula,
        });
    }
    Ok(out)
}

pub fn parse_cell_value(v: &Value, path: &str) -> Result<SetCellValue, Error> {
    match v {
        Value::Null => Ok(SetCellValue::Null),
        Value::Bool(b) => Ok(SetCellValue::Bool(*b)),
        Value::Number(n) => Ok(SetCellValue::Number(n.clone())),
        Value::String(s) => Ok(SetCellValue::Text(s.clone())),
        _ => Err(Error::with_path(
            ErrorCode::InvalidField,
            "单元格值必须是数字、字符串、布尔或 null",
            path,
        )),
    }
}
