//! 领域 JSON 信封：schemaVersion、action、路径与格式。不含单元格或段落语义。

use std::path::{Path, PathBuf};

use serde_json::{json, Map, Value};

use crate::error::{Error, ErrorKind, TypeName};

#[derive(Debug)]
pub enum RunOutput {
    Inspect(Value),
    Help(Value),
    DryRun {
        ops_applied: usize,
        parts: Vec<String>,
    },
    Written {
        path: PathBuf,
        ops_applied: Option<usize>,
        content_type: Option<String>,
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
    ExportMedia,
    ExportChartData,
    ExportChartImage,
    ExportSlideImage,
    LayoutCheck,
    Help,
}

/// 这个动作的信封允许哪些字段。领域 JSON 的字段名以这里为准。
pub fn envelope_fields(action: Action) -> &'static [&'static str] {
    match action {
        Action::Help => &[
            "schemaVersion",
            "action",
            "format",
            "q",
            "op",
            "name",
            "locale",
        ],
        Action::Inspect => &[
            "schemaVersion",
            "action",
            "path",
            "format",
            "scope",
            "locale",
        ],
        Action::Create => &[
            "schemaVersion",
            "action",
            "path",
            "format",
            "workbook",
            "document",
            "presentation",
            "locale",
        ],
        Action::Apply => &[
            "schemaVersion",
            "action",
            "path",
            "format",
            "output",
            "ops",
            "dryRun",
            "locale",
        ],
        Action::LayoutCheck => &["schemaVersion", "action", "path", "format", "locale"],
        Action::ExportMedia
        | Action::ExportChartData
        | Action::ExportChartImage
        | Action::ExportSlideImage => &[
            "schemaVersion",
            "action",
            "path",
            "format",
            "id",
            "output",
            "locale",
        ],
    }
}

pub struct Request {
    pub action: Action,
    pub obj: Map<String, Value>,
}

pub fn parse_request(input_json: &str) -> Result<Request, Error> {
    let value: Value = serde_json::from_str(input_json).map_err(|e| {
        Error::new(ErrorKind::InvalidJsonParse {
            detail: e.to_string(),
        })
    })?;
    let obj = value
        .as_object()
        .cloned()
        .ok_or_else(|| Error::new(ErrorKind::InvalidJsonNotObject))?;

    match obj.get("schemaVersion") {
        None => {}
        Some(v) if v.as_u64() == Some(1) || v.as_i64() == Some(1) => {}
        Some(_) => return Err(Error::new(ErrorKind::UnsupportedSchema)),
    }

    let action = obj.get("action").and_then(Value::as_str).ok_or_else(|| {
        Error::new(ErrorKind::MissingEnvelopeField {
            field: "action".into(),
        })
    })?;
    let action = match action {
        "inspect" => Action::Inspect,
        "create" => Action::Create,
        "apply" => Action::Apply,
        "exportMedia" => Action::ExportMedia,
        "exportChartData" => Action::ExportChartData,
        "exportChartImage" => Action::ExportChartImage,
        "exportSlideImage" => Action::ExportSlideImage,
        "layoutCheck" => Action::LayoutCheck,
        "help" => Action::Help,
        other => {
            return Err(Error::new(ErrorKind::UnsupportedAction {
                action: other.to_string(),
            }));
        }
    };
    reject_illegal_xml_chars(&value)?;
    Ok(Request { action, obj })
}

/// XML 1.0 不允许的码位。TAB、LF、CR 允许。代理项在 Rust 的 `char` 里不会出现。
fn illegal_xml_scalar(c: char) -> bool {
    let scalar = c as u32;
    scalar <= 0x0008
        || scalar == 0x000B
        || scalar == 0x000C
        || (0x000E..=0x001F).contains(&scalar)
        || scalar == 0xFFFE
        || scalar == 0xFFFF
}

fn reject_illegal_xml_chars(value: &Value) -> Result<(), Error> {
    fn child_key(parent: &str, key: &str) -> String {
        if parent.is_empty() {
            key.to_string()
        } else {
            format!("{parent}.{key}")
        }
    }
    fn child_index(parent: &str, index: usize) -> String {
        if parent.is_empty() {
            format!("[{index}]")
        } else {
            format!("{parent}[{index}]")
        }
    }
    fn check(text: &str, path: &str) -> Result<(), Error> {
        if let Some(scalar) = text.chars().find(|c| illegal_xml_scalar(*c)) {
            return Err(Error::with_path(
                ErrorKind::IllegalXmlChar {
                    scalar: scalar as u32,
                },
                path,
            ));
        }
        Ok(())
    }
    fn walk(value: &Value, path: &str) -> Result<(), Error> {
        match value {
            Value::String(text) => check(text, path),
            Value::Array(items) => {
                for (index, item) in items.iter().enumerate() {
                    walk(item, &child_index(path, index))?;
                }
                Ok(())
            }
            Value::Object(map) => {
                for (key, item) in map {
                    check(key, path)?;
                    walk(item, &child_key(path, key))?;
                }
                Ok(())
            }
            Value::Null | Value::Bool(_) | Value::Number(_) => Ok(()),
        }
    }
    walk(value, "")
}

pub fn deny_unknown(obj: &Map<String, Value>, allowed: &[&str]) -> Result<(), Error> {
    for k in obj.keys() {
        if !allowed.contains(&k.as_str()) {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
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
        "xlsx" | "xlsm" => Ok(Format::Xlsx),
        "docx" => Ok(Format::Docx),
        "pptx" => Ok(Format::Pptx),
        _ => Err(Error::new(ErrorKind::CannotInferFormat {
            path: path.to_string(),
        })),
    }
}

pub fn parse_format(s: &str) -> Result<Format, Error> {
    match s {
        "xlsx" | "xlsm" => Ok(Format::Xlsx),
        "docx" => Ok(Format::Docx),
        "pptx" => Ok(Format::Pptx),
        other => Err(Error::new(ErrorKind::UnsupportedFormat {
            format: other.to_string(),
        })),
    }
}

pub fn str_field<'a>(obj: &'a Map<String, Value>, key: &str) -> Result<&'a str, Error> {
    match obj.get(key) {
        Some(Value::String(s)) if !s.is_empty() => Ok(s),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: key.to_string(),
                expected: TypeName::NonEmptyString,
            },
            key,
        )),
        None => Err(Error::with_path(
            ErrorKind::MissingField {
                field: key.to_string(),
            },
            key,
        )),
    }
}

pub fn require_ops(obj: &Map<String, Value>) -> Result<&[Value], Error> {
    let ops = obj.get("ops").ok_or_else(|| {
        Error::with_path(
            ErrorKind::MissingField {
                field: "ops".into(),
            },
            "ops",
        )
    })?;
    let ops = ops.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "ops".into(),
                expected: TypeName::Array,
            },
            "ops",
        )
    })?;
    if ops.is_empty() {
        return Err(Error::with_path(
            ErrorKind::EmptyArray {
                field: "ops".into(),
            },
            "ops",
        ));
    }
    Ok(ops)
}

/// 先写同目录临时文件再改名，避免截断原文件。本机 `run` 才写盘。
#[cfg(not(target_arch = "wasm32"))]
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), Error> {
    use std::fs;
    use std::io;

    use crate::error::io_path;
    if path.is_dir() {
        return Err(io_path(
            io::Error::new(io::ErrorKind::AlreadyExists, "目标路径是目录"),
            path.to_path_buf(),
        ));
    }
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

    fs::write(&tmp, bytes).map_err(|e| io_path(e, tmp.clone()))?;
    if let Err(e) = fs::rename(&tmp, path) {
        let _ = fs::remove_file(&tmp);
        return Err(io_path(e, path.to_path_buf()));
    }
    Ok(())
}

pub fn written_json(
    path: &Path,
    ops_applied: Option<usize>,
    content_type: Option<&str>,
    pretty: bool,
) -> String {
    let mut body = json!({
        "ok": true,
        "path": path.display().to_string(),
    });
    if let Some(n) = ops_applied {
        body["opsApplied"] = json!(n);
    }
    if let Some(ct) = content_type {
        body["contentType"] = json!(ct);
    }
    if pretty {
        serde_json::to_string_pretty(&body).unwrap()
    } else {
        serde_json::to_string(&body).unwrap()
    }
}

pub fn dry_run_value(ops_applied: usize, parts: &[String]) -> Value {
    json!({
        "ok": true,
        "dryRun": true,
        "opsApplied": ops_applied,
        "parts": parts,
    })
}

pub fn dry_run_json(ops_applied: usize, parts: &[String], pretty: bool) -> String {
    let body = dry_run_value(ops_applied, parts);
    if pretty {
        serde_json::to_string_pretty(&body).unwrap()
    } else {
        serde_json::to_string(&body).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::write_atomic;

    #[test]
    fn write_atomic_replaces_and_drops_backup() {
        let dir = std::env::temp_dir().join(format!("ooxp-atomic-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("a.xlsx");
        std::fs::write(&path, b"old").unwrap();
        write_atomic(&path, b"new").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"new");
        assert!(!dir.join("a.xlsx.ooxp.bak").exists());
        assert!(!dir.join("a.xlsx.ooxp.tmp").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn write_atomic_refuses_a_directory() {
        let dir = std::env::temp_dir().join(format!("ooxp-atomic-dir-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let err = write_atomic(&dir, b"nope").unwrap_err();
        assert_eq!(err.code(), crate::error::ErrorCode::Io);
        assert!(dir.is_dir());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
