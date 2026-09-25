//! 领域 JSON 解析与动作分发。

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::error::{io_path, Error, ErrorCode};
use crate::xlsx;

#[derive(Debug)]
pub enum RunOutput {
    Inspect(Value),
    Written {
        path: PathBuf,
        ops_applied: Option<usize>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Xlsx,
    Docx,
    Pptx,
}

pub fn run(input_json: &str, cli_output: Option<&Path>) -> Result<RunOutput, Error> {
    let value: Value = serde_json::from_str(input_json)
        .map_err(|e| Error::new(ErrorCode::InvalidJson, format!("入参不是 JSON：{e}")))?;
    if !value.is_object() {
        return Err(Error::new(ErrorCode::InvalidJson, "入参必须是 JSON 对象"));
    }
    let obj = value.as_object().unwrap();

    match obj.get("schemaVersion") {
        None => {
            return Err(Error::new(ErrorCode::InvalidJson, "缺少 schemaVersion"));
        }
        Some(v) if v.as_u64() != Some(1) && v.as_i64() != Some(1) => {
            return Err(Error::new(
                ErrorCode::UnsupportedSchema,
                "schemaVersion 必须是 1",
            ));
        }
        _ => {}
    }

    let action = obj
        .get("action")
        .and_then(Value::as_str)
        .ok_or_else(|| Error::new(ErrorCode::InvalidJson, "缺少 action"))?;

    match action {
        "inspect" => run_inspect(obj, &value),
        "create" => run_create(obj, &value, cli_output),
        "apply" => run_apply(obj, &value, cli_output),
        _ => Err(Error::new(
            ErrorCode::UnsupportedAction,
            format!("不支持的 action：{action}"),
        )),
    }
}

fn deny_unknown(obj: &serde_json::Map<String, Value>, allowed: &[&str]) -> Result<(), Error> {
    for k in obj.keys() {
        if !allowed.contains(&k.as_str()) {
            return Err(Error::with_path(
                ErrorCode::InvalidField,
                format!("不认识的字段：{k}"),
                k.clone(),
            ));
        }
    }
    Ok(())
}

fn infer_format(path: &str, format: Option<&str>) -> Result<Format, Error> {
    if let Some(f) = format {
        return parse_format(f);
    }
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "xlsx" => Ok(Format::Xlsx),
        "docx" => Ok(Format::Docx),
        "pptx" => Ok(Format::Pptx),
        _ => Err(Error::new(
            ErrorCode::UnsupportedFormat,
            format!("无法从路径推断格式：{path}"),
        )),
    }
}

fn parse_format(s: &str) -> Result<Format, Error> {
    match s {
        "xlsx" => Ok(Format::Xlsx),
        "docx" => Ok(Format::Docx),
        "pptx" => Ok(Format::Pptx),
        other => Err(Error::new(
            ErrorCode::UnsupportedFormat,
            format!("不支持的 format：{other}"),
        )),
    }
}

fn require_xlsx(fmt: Format) -> Result<(), Error> {
    match fmt {
        Format::Xlsx => Ok(()),
        Format::Docx | Format::Pptx => {
            Err(Error::new(ErrorCode::NotImplemented, "第一期只实现 xlsx"))
        }
    }
}

fn str_field<'a>(obj: &'a serde_json::Map<String, Value>, key: &str) -> Result<&'a str, Error> {
    match obj.get(key) {
        Some(Value::String(s)) if !s.is_empty() => Ok(s),
        Some(_) => Err(Error::with_path(
            ErrorCode::InvalidField,
            format!("{key} 必须是非空字符串"),
            key,
        )),
        None => Err(Error::with_path(
            ErrorCode::InvalidField,
            format!("缺少 {key}"),
            key,
        )),
    }
}

fn run_inspect(obj: &serde_json::Map<String, Value>, _raw: &Value) -> Result<RunOutput, Error> {
    deny_unknown(obj, &["schemaVersion", "action", "path", "format", "scope"])?;
    let path = str_field(obj, "path")?;
    let format = obj.get("format").and_then(Value::as_str);
    let fmt = infer_format(path, format)?;
    require_xlsx(fmt)?;

    let scope = parse_scope(obj.get("scope"))?;
    let bytes = fs::read(path).map_err(|e| io_path(e, PathBuf::from(path)))?;
    let tree = xlsx::inspect(&bytes, scope)?;
    Ok(RunOutput::Inspect(tree))
}

#[derive(Debug, Clone)]
pub struct InspectScope {
    pub sheet: String,
    pub range: String,
    pub max_cells: u32,
}

fn parse_scope(v: Option<&Value>) -> Result<Option<InspectScope>, Error> {
    let Some(v) = v else {
        return Ok(None);
    };
    let obj = v
        .as_object()
        .ok_or_else(|| Error::with_path(ErrorCode::InvalidField, "scope 必须是对象", "scope"))?;
    if obj.is_empty() {
        return Ok(None);
    }
    deny_unknown(obj, &["sheet", "range", "maxCells"])?;
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

fn run_create(
    obj: &serde_json::Map<String, Value>,
    _raw: &Value,
    cli_output: Option<&Path>,
) -> Result<RunOutput, Error> {
    deny_unknown(
        obj,
        &["schemaVersion", "action", "path", "format", "workbook"],
    )?;
    let path = obj
        .get("path")
        .and_then(Value::as_str)
        .map(PathBuf::from)
        .or_else(|| cli_output.map(Path::to_path_buf))
        .ok_or_else(|| Error::with_path(ErrorCode::InvalidField, "缺少 path", "path"))?;
    let format = obj.get("format").and_then(Value::as_str);
    let fmt = infer_format(&path.to_string_lossy(), format)?;
    require_xlsx(fmt)?;
    let workbook = obj.get("workbook").ok_or_else(|| {
        Error::with_path(
            ErrorCode::InvalidField,
            "create 必须有 workbook",
            "workbook",
        )
    })?;
    let bytes = xlsx::create(workbook)?;
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| io_path(e, parent.to_path_buf()))?;
        }
    }
    fs::write(&path, bytes).map_err(|e| io_path(e, path.clone()))?;
    Ok(RunOutput::Written {
        path,
        ops_applied: None,
    })
}

fn run_apply(
    obj: &serde_json::Map<String, Value>,
    _raw: &Value,
    cli_output: Option<&Path>,
) -> Result<RunOutput, Error> {
    deny_unknown(
        obj,
        &["schemaVersion", "action", "path", "format", "output", "ops"],
    )?;
    let path = str_field(obj, "path")?;
    let format = obj.get("format").and_then(Value::as_str);
    let fmt = infer_format(path, format)?;
    require_xlsx(fmt)?;
    let out = obj
        .get("output")
        .and_then(Value::as_str)
        .map(PathBuf::from)
        .or_else(|| cli_output.map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from(path));
    let ops = obj
        .get("ops")
        .ok_or_else(|| Error::with_path(ErrorCode::InvalidField, "apply 必须有 ops", "ops"))?;
    let ops = ops
        .as_array()
        .ok_or_else(|| Error::with_path(ErrorCode::InvalidField, "ops 必须是数组", "ops"))?;
    if ops.is_empty() {
        return Err(Error::with_path(
            ErrorCode::InvalidField,
            "ops 不能为空",
            "ops",
        ));
    }
    let parsed = parse_ops(ops)?;
    let bytes = fs::read(path).map_err(|e| io_path(e, PathBuf::from(path)))?;
    let out_bytes = xlsx::apply(&bytes, &parsed)?;
    if let Some(parent) = out.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| io_path(e, parent.to_path_buf()))?;
        }
    }
    fs::write(&out, out_bytes).map_err(|e| io_path(e, out.clone()))?;
    Ok(RunOutput::Written {
        path: out,
        ops_applied: Some(parsed.len()),
    })
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

fn parse_ops(ops: &[Value]) -> Result<Vec<SetCellOp>, Error> {
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

pub fn written_json(path: &Path, ops_applied: Option<usize>, pretty: bool) -> String {
    let mut body = json!({
        "ok": true,
        "path": path.display().to_string(),
    });
    if let Some(n) = ops_applied {
        body["opsApplied"] = json!(n);
    }
    if pretty {
        serde_json::to_string_pretty(&body).unwrap()
    } else {
        serde_json::to_string(&body).unwrap()
    }
}
