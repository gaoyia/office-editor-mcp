//! 领域 JSON 信封：schemaVersion、action、路径与格式。不含单元格或段落语义。

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{json, Map, Value};

use crate::error::{io_path, Error, ErrorCode};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Inspect,
    Create,
    Apply,
}

pub struct Request {
    pub action: Action,
    pub obj: Map<String, Value>,
}

pub fn parse_request(input_json: &str) -> Result<Request, Error> {
    let value: Value = serde_json::from_str(input_json)
        .map_err(|e| Error::new(ErrorCode::InvalidJson, format!("入参不是 JSON：{e}")))?;
    let obj = value
        .as_object()
        .cloned()
        .ok_or_else(|| Error::new(ErrorCode::InvalidJson, "入参必须是 JSON 对象"))?;

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
    let action = match action {
        "inspect" => Action::Inspect,
        "create" => Action::Create,
        "apply" => Action::Apply,
        other => {
            return Err(Error::new(
                ErrorCode::UnsupportedAction,
                format!("不支持的 action：{other}"),
            ));
        }
    };
    Ok(Request { action, obj })
}

pub fn deny_unknown(obj: &Map<String, Value>, allowed: &[&str]) -> Result<(), Error> {
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

pub fn infer_format(path: &str, format: Option<&str>) -> Result<Format, Error> {
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

pub fn parse_format(s: &str) -> Result<Format, Error> {
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

pub fn str_field<'a>(obj: &'a Map<String, Value>, key: &str) -> Result<&'a str, Error> {
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

pub fn require_ops(obj: &Map<String, Value>) -> Result<&[Value], Error> {
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
    Ok(ops)
}

/// 先写同目录临时文件再改名，避免截断原文件。
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), Error> {
    let parent = path.parent().filter(|p| !p.as_os_str().is_empty());
    if let Some(dir) = parent {
        fs::create_dir_all(dir).map_err(|e| io_path(e, dir.to_path_buf()))?;
    }
    let tmp = {
        let mut name = path
            .file_name()
            .unwrap_or_else(|| std::ffi::OsStr::new("ooxp"))
            .to_os_string();
        name.push(".ooxp.tmp");
        match parent {
            Some(dir) => dir.join(name),
            None => PathBuf::from(name),
        }
    };
    let bak = {
        let mut name = path
            .file_name()
            .unwrap_or_else(|| std::ffi::OsStr::new("ooxp"))
            .to_os_string();
        name.push(".ooxp.bak");
        match parent {
            Some(dir) => dir.join(name),
            None => PathBuf::from(name),
        }
    };

    fs::write(&tmp, bytes).map_err(|e| io_path(e, tmp.clone()))?;

    let had_dest = path.exists();
    if had_dest {
        fs::rename(path, &bak).map_err(|e| io_path(e, path.to_path_buf()))?;
    }
    if let Err(e) = fs::rename(&tmp, path) {
        if had_dest {
            let _ = fs::rename(&bak, path);
        }
        let _ = fs::remove_file(&tmp);
        return Err(io_path(e, path.to_path_buf()));
    }
    if had_dest {
        let _ = fs::remove_file(&bak);
    }
    Ok(())
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
