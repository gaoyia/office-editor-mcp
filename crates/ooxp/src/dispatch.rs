//! 按 format 把信封交给 xlsx / docx / pptx。

use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{io_path, Error, ErrorCode};
use crate::json::{
    self, deny_unknown, infer_format, require_ops, str_field, write_atomic, Action, Format,
    Request, RunOutput,
};
use crate::{docx, pptx, xlsx};

pub fn run(input_json: &str, cli_output: Option<&Path>) -> Result<RunOutput, Error> {
    let req = json::parse_request(input_json)?;
    match req.action {
        Action::Inspect => run_inspect(&req),
        Action::Create => run_create(&req, cli_output),
        Action::Apply => run_apply(&req, cli_output),
    }
}

fn run_inspect(req: &Request) -> Result<RunOutput, Error> {
    deny_unknown(
        &req.obj,
        &["schemaVersion", "action", "path", "format", "scope"],
    )?;
    let path = str_field(&req.obj, "path")?;
    let format = req.obj.get("format").and_then(serde_json::Value::as_str);
    let fmt = infer_format(path, format)?;
    let bytes = fs::read(path).map_err(|e| io_path(e, PathBuf::from(path)))?;
    let tree = match fmt {
        Format::Xlsx => {
            let scope = xlsx::parse_scope(req.obj.get("scope"))?;
            xlsx::inspect(&bytes, scope)?
        }
        Format::Docx => {
            let scope = docx::parse_scope(req.obj.get("scope"))?;
            docx::inspect(&bytes, scope)?
        }
        Format::Pptx => {
            let scope = pptx::parse_scope(req.obj.get("scope"))?;
            pptx::inspect(&bytes, scope)?
        }
    };
    Ok(RunOutput::Inspect(tree))
}

fn run_create(req: &Request, cli_output: Option<&Path>) -> Result<RunOutput, Error> {
    deny_unknown(
        &req.obj,
        &[
            "schemaVersion",
            "action",
            "path",
            "format",
            "workbook",
            "document",
            "presentation",
        ],
    )?;
    let path = req
        .obj
        .get("path")
        .and_then(serde_json::Value::as_str)
        .map(PathBuf::from)
        .or_else(|| cli_output.map(Path::to_path_buf))
        .ok_or_else(|| Error::with_path(ErrorCode::InvalidField, "缺少 path", "path"))?;
    let format = req.obj.get("format").and_then(serde_json::Value::as_str);
    let fmt = infer_format(&path.to_string_lossy(), format)?;
    let has_wb = req.obj.contains_key("workbook");
    let has_doc = req.obj.contains_key("document");
    let has_pres = req.obj.contains_key("presentation");
    let content_count = [has_wb, has_doc, has_pres]
        .into_iter()
        .filter(|b| *b)
        .count();
    if content_count > 1 {
        return Err(Error::with_path(
            ErrorCode::InvalidField,
            "workbook、document、presentation 不能同时出现",
            "presentation",
        ));
    }
    let bytes = match fmt {
        Format::Xlsx => {
            if has_doc || has_pres {
                return Err(Error::with_path(
                    ErrorCode::InvalidField,
                    "xlsx 的 create 必须用 workbook",
                    if has_doc { "document" } else { "presentation" },
                ));
            }
            let workbook = req.obj.get("workbook").ok_or_else(|| {
                Error::with_path(
                    ErrorCode::InvalidField,
                    "create 必须有 workbook",
                    "workbook",
                )
            })?;
            xlsx::create(workbook)?
        }
        Format::Docx => {
            if has_wb || has_pres {
                return Err(Error::with_path(
                    ErrorCode::InvalidField,
                    "docx 的 create 必须用 document",
                    if has_wb { "workbook" } else { "presentation" },
                ));
            }
            let document = req.obj.get("document").ok_or_else(|| {
                Error::with_path(
                    ErrorCode::InvalidField,
                    "create 必须有 document",
                    "document",
                )
            })?;
            docx::create(document)?
        }
        Format::Pptx => {
            if has_wb || has_doc {
                return Err(Error::with_path(
                    ErrorCode::InvalidField,
                    "pptx 的 create 必须用 presentation",
                    if has_wb { "workbook" } else { "document" },
                ));
            }
            let presentation = req.obj.get("presentation").ok_or_else(|| {
                Error::with_path(
                    ErrorCode::InvalidField,
                    "create 必须有 presentation",
                    "presentation",
                )
            })?;
            pptx::create(presentation)?
        }
    };
    write_atomic(&path, &bytes)?;
    Ok(RunOutput::Written {
        path,
        ops_applied: None,
    })
}

fn run_apply(req: &Request, cli_output: Option<&Path>) -> Result<RunOutput, Error> {
    deny_unknown(
        &req.obj,
        &["schemaVersion", "action", "path", "format", "output", "ops"],
    )?;
    let path = str_field(&req.obj, "path")?;
    let format = req.obj.get("format").and_then(serde_json::Value::as_str);
    let fmt = infer_format(path, format)?;
    let out = req
        .obj
        .get("output")
        .and_then(serde_json::Value::as_str)
        .map(PathBuf::from)
        .or_else(|| cli_output.map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from(path));
    let ops = require_ops(&req.obj)?;
    let bytes = fs::read(path).map_err(|e| io_path(e, PathBuf::from(path)))?;
    let (out_bytes, n) = match fmt {
        Format::Xlsx => {
            let parsed = xlsx::parse_ops(ops)?;
            let n = parsed.len();
            (xlsx::apply(&bytes, &parsed)?, n)
        }
        Format::Docx => {
            let parsed = docx::parse_ops(ops)?;
            let n = parsed.len();
            (docx::apply(&bytes, &parsed)?, n)
        }
        Format::Pptx => {
            let parsed = pptx::parse_ops(ops)?;
            let n = parsed.len();
            (pptx::apply(&bytes, &parsed)?, n)
        }
    };
    write_atomic(&out, &out_bytes)?;
    Ok(RunOutput::Written {
        path: out,
        ops_applied: Some(n),
    })
}
