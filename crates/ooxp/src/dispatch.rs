//! 按 format 把信封交给 xlsx / docx / pptx。
//! `run_bytes` 不碰磁盘。本机的 `run` 才按路径读写。

use std::io;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::error::{io_path, Error, ErrorKind};
use crate::json::{
    self, deny_unknown, infer_format, require_ops, str_field, Action, Format, Request, RunOutput,
};
use crate::{docx, pptx, xlsx};

/// 库门口的结果。字节由调用方自己落盘。
#[derive(Debug)]
pub enum ByteOutput {
    Inspect(Value),
    Help(Value),
    Bytes {
        bytes: Vec<u8>,
        ops_applied: Option<usize>,
        content_type: Option<String>,
    },
}

/// 不打开、不保存文件。`inspect`、`apply`、`exportMedia`、`exportChartData` 必须带上包字节。
pub fn run_bytes(
    input_json: &str,
    package: Option<&[u8]>,
    files: &[(&str, &[u8])],
) -> Result<ByteOutput, Error> {
    let map = files
        .iter()
        .map(|(path, bytes)| crate::host_files::success(path, bytes.to_vec()))
        .collect();
    let _guard = crate::host_files::install(map);
    run_bytes_inner(input_json, package)
}

fn run_bytes_inner(input_json: &str, package: Option<&[u8]>) -> Result<ByteOutput, Error> {
    let req = json::parse_request(input_json)?;
    screen(&req)?;
    match req.action {
        Action::Inspect => {
            let bytes = require_package(package)?;
            Ok(ByteOutput::Inspect(inspect_bytes(&req, bytes)?))
        }
        Action::Create => Ok(ByteOutput::Bytes {
            bytes: create_bytes(&req)?,
            ops_applied: None,
            content_type: None,
        }),
        Action::Apply => {
            let bytes = require_package(package)?;
            let (out, n) = apply_bytes(&req, bytes)?;
            Ok(ByteOutput::Bytes {
                bytes: out,
                ops_applied: Some(n),
                content_type: None,
            })
        }
        Action::ExportMedia => {
            let bytes = require_package(package)?;
            let exported = export_media_bytes(&req, bytes)?;
            Ok(ByteOutput::Bytes {
                bytes: exported.bytes,
                ops_applied: None,
                content_type: exported.content_type,
            })
        }
        Action::ExportChartData => {
            let bytes = require_package(package)?;
            let exported = export_chart_bytes(&req, bytes)?;
            Ok(ByteOutput::Bytes {
                bytes: exported.bytes,
                ops_applied: None,
                content_type: exported.content_type,
            })
        }
        Action::Help => Ok(ByteOutput::Help(help_value(&req)?)),
    }
}

/// 本机入口：按 JSON 里的路径读包、写结果。不编进 wasm。
#[cfg(not(target_arch = "wasm32"))]
pub fn run(input_json: &str, cli_output: Option<&Path>) -> Result<RunOutput, Error> {
    use std::fs;

    use crate::json::write_atomic;

    let req = json::parse_request(input_json)?;
    // 先拒掉多余字段，再读盘。缺文件不能盖过 InvalidField。
    screen(&req)?;
    let _clock = crate::host_env::install_native();
    let _files = crate::host_files::load_json(&Value::Object(req.obj.clone()));
    let package = match req.action {
        Action::Help | Action::Create => None,
        Action::Inspect | Action::Apply | Action::ExportMedia | Action::ExportChartData => {
            let path = str_field(&req.obj, "path")?;
            Some(fs::read(path).map_err(|e| io_path(e, PathBuf::from(path)))?)
        }
    };
    let produced = run_bytes_inner(input_json, package.as_deref())?;
    match produced {
        ByteOutput::Inspect(tree) => Ok(RunOutput::Inspect(tree)),
        ByteOutput::Help(tree) => Ok(RunOutput::Help(tree)),
        ByteOutput::Bytes {
            bytes,
            ops_applied,
            content_type,
        } => {
            let path = output_path(&req, cli_output)?;
            write_atomic(&path, &bytes)?;
            Ok(RunOutput::Written {
                path,
                ops_applied,
                content_type,
            })
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn output_path(req: &Request, cli_output: Option<&Path>) -> Result<PathBuf, Error> {
    match req.action {
        Action::Create => req
            .obj
            .get("path")
            .and_then(Value::as_str)
            .map(PathBuf::from)
            .or_else(|| cli_output.map(Path::to_path_buf))
            .ok_or_else(|| {
                Error::with_path(
                    ErrorKind::MissingField {
                        field: "path".into(),
                    },
                    "path",
                )
            }),
        Action::Apply => {
            let path = str_field(&req.obj, "path")?;
            Ok(req
                .obj
                .get("output")
                .and_then(Value::as_str)
                .map(PathBuf::from)
                .or_else(|| cli_output.map(Path::to_path_buf))
                .unwrap_or_else(|| PathBuf::from(path)))
        }
        Action::ExportMedia | Action::ExportChartData => req
            .obj
            .get("output")
            .and_then(Value::as_str)
            .map(PathBuf::from)
            .or_else(|| cli_output.map(Path::to_path_buf))
            .ok_or_else(|| {
                Error::with_path(
                    ErrorKind::MissingField {
                        field: "output".into(),
                    },
                    "output",
                )
            }),
        Action::Inspect | Action::Help => Err(Error::new(ErrorKind::UnsupportedAction {
            action: "bytes".into(),
        })),
    }
}

fn screen(req: &Request) -> Result<(), Error> {
    match req.action {
        Action::Help => deny_unknown(
            &req.obj,
            &[
                "schemaVersion",
                "action",
                "format",
                "q",
                "op",
                "name",
                "locale",
            ],
        ),
        Action::Inspect => deny_unknown(
            &req.obj,
            &[
                "schemaVersion",
                "action",
                "path",
                "format",
                "scope",
                "locale",
            ],
        ),
        Action::Create => deny_unknown(
            &req.obj,
            &[
                "schemaVersion",
                "action",
                "path",
                "format",
                "workbook",
                "document",
                "presentation",
                "locale",
            ],
        ),
        Action::Apply => deny_unknown(
            &req.obj,
            &[
                "schemaVersion",
                "action",
                "path",
                "format",
                "output",
                "ops",
                "locale",
            ],
        ),
        Action::ExportMedia | Action::ExportChartData => deny_unknown(
            &req.obj,
            &[
                "schemaVersion",
                "action",
                "path",
                "format",
                "id",
                "output",
                "locale",
            ],
        ),
    }
}

fn require_package(package: Option<&[u8]>) -> Result<&[u8], Error> {
    package.ok_or_else(|| {
        Error::from(io::Error::new(
            io::ErrorKind::InvalidInput,
            "package bytes are required",
        ))
    })
}

fn help_value(req: &Request) -> Result<Value, Error> {
    json::deny_unknown(
        &req.obj,
        &[
            "schemaVersion",
            "action",
            "format",
            "q",
            "op",
            "name",
            "locale",
        ],
    )?;
    crate::help::help(&req.obj)
}

fn inspect_bytes(req: &Request, bytes: &[u8]) -> Result<Value, Error> {
    deny_unknown(
        &req.obj,
        &[
            "schemaVersion",
            "action",
            "path",
            "format",
            "scope",
            "locale",
        ],
    )?;
    let path = str_field(&req.obj, "path")?;
    let format = req.obj.get("format").and_then(Value::as_str);
    let fmt = infer_format(path, format)?;
    let tree = match fmt {
        Format::Xlsx => {
            let scope = xlsx::parse_scope(req.obj.get("scope"))?;
            xlsx::inspect(bytes, scope)?
        }
        Format::Docx => {
            let scope = docx::parse_scope(req.obj.get("scope"))?;
            docx::inspect(bytes, scope)?
        }
        Format::Pptx => {
            let scope = pptx::parse_scope(req.obj.get("scope"))?;
            pptx::inspect(bytes, scope)?
        }
    };
    Ok(tree)
}

fn create_bytes(req: &Request) -> Result<Vec<u8>, Error> {
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
            "locale",
        ],
    )?;
    let path = str_field(&req.obj, "path")?;
    let format = req.obj.get("format").and_then(Value::as_str);
    let fmt = infer_format(path, format)?;
    let has_wb = req.obj.contains_key("workbook");
    let has_doc = req.obj.contains_key("document");
    let has_pres = req.obj.contains_key("presentation");
    let content_count = [has_wb, has_doc, has_pres]
        .into_iter()
        .filter(|b| *b)
        .count();
    if content_count > 1 {
        return Err(Error::with_path(
            ErrorKind::ContentObjectsExclusive,
            "presentation",
        ));
    }
    let bytes = match fmt {
        Format::Xlsx => {
            if has_doc || has_pres {
                return Err(Error::with_path(
                    ErrorKind::CreateNeedsObject {
                        format: "xlsx".into(),
                        field: "workbook".into(),
                    },
                    if has_doc { "document" } else { "presentation" },
                ));
            }
            let workbook = req.obj.get("workbook").ok_or_else(|| {
                Error::with_path(
                    ErrorKind::CreateMissingObject {
                        field: "workbook".into(),
                    },
                    "workbook",
                )
            })?;
            let xlsm = std::path::Path::new(path)
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| e.eq_ignore_ascii_case("xlsm"));
            if xlsm {
                xlsx::create_with_workbook_ct(workbook, xlsx::CT_WORKBOOK_MACRO)?
            } else {
                xlsx::create(workbook)?
            }
        }
        Format::Docx => {
            if has_wb || has_pres {
                return Err(Error::with_path(
                    ErrorKind::CreateNeedsObject {
                        format: "docx".into(),
                        field: "document".into(),
                    },
                    if has_wb { "workbook" } else { "presentation" },
                ));
            }
            let document = req.obj.get("document").ok_or_else(|| {
                Error::with_path(
                    ErrorKind::CreateMissingObject {
                        field: "document".into(),
                    },
                    "document",
                )
            })?;
            docx::create(document)?
        }
        Format::Pptx => {
            if has_wb || has_doc {
                return Err(Error::with_path(
                    ErrorKind::CreateNeedsObject {
                        format: "pptx".into(),
                        field: "presentation".into(),
                    },
                    if has_wb { "workbook" } else { "document" },
                ));
            }
            let presentation = req.obj.get("presentation").ok_or_else(|| {
                Error::with_path(
                    ErrorKind::CreateMissingObject {
                        field: "presentation".into(),
                    },
                    "presentation",
                )
            })?;
            pptx::create(presentation)?
        }
    };
    Ok(bytes)
}

fn apply_bytes(req: &Request, bytes: &[u8]) -> Result<(Vec<u8>, usize), Error> {
    deny_unknown(
        &req.obj,
        &[
            "schemaVersion",
            "action",
            "path",
            "format",
            "output",
            "ops",
            "locale",
        ],
    )?;
    let path = str_field(&req.obj, "path")?;
    let format = req.obj.get("format").and_then(Value::as_str);
    let fmt = infer_format(path, format)?;
    let ops = require_ops(&req.obj)?;
    let (out_bytes, n) = match fmt {
        Format::Xlsx => {
            let parsed = xlsx::parse_ops(ops)?;
            let n = parsed.len();
            (xlsx::apply(bytes, &parsed)?, n)
        }
        Format::Docx => {
            let parsed = docx::parse_ops(ops)?;
            let n = parsed.len();
            (docx::apply(bytes, &parsed)?, n)
        }
        Format::Pptx => {
            let parsed = pptx::parse_ops(ops)?;
            let n = parsed.len();
            (pptx::apply(bytes, &parsed)?, n)
        }
    };
    Ok((out_bytes, n))
}

struct Exported {
    bytes: Vec<u8>,
    content_type: Option<String>,
}

fn export_media_bytes(req: &Request, bytes: &[u8]) -> Result<Exported, Error> {
    deny_unknown(
        &req.obj,
        &[
            "schemaVersion",
            "action",
            "path",
            "format",
            "id",
            "output",
            "locale",
        ],
    )?;
    let path = str_field(&req.obj, "path")?;
    let format = req.obj.get("format").and_then(Value::as_str);
    let fmt = infer_format(path, format)?;
    let id = str_field(&req.obj, "id")?;
    let (raw, content_type) = match fmt {
        Format::Docx => {
            let exported = docx::export_media(bytes, id)?;
            (exported.bytes, exported.content_type)
        }
        Format::Xlsx => {
            let exported = xlsx::export_media(bytes, id)?;
            (exported.bytes, exported.content_type)
        }
        Format::Pptx => {
            let exported = pptx::export_media(bytes, id)?;
            (exported.bytes, exported.content_type)
        }
    };
    Ok(Exported {
        bytes: raw,
        content_type,
    })
}

fn export_chart_bytes(req: &Request, bytes: &[u8]) -> Result<Exported, Error> {
    deny_unknown(
        &req.obj,
        &[
            "schemaVersion",
            "action",
            "path",
            "format",
            "id",
            "output",
            "locale",
        ],
    )?;
    let path = str_field(&req.obj, "path")?;
    let format = req.obj.get("format").and_then(Value::as_str);
    let fmt = infer_format(path, format)?;
    let id = str_field(&req.obj, "id")?;
    match fmt {
        Format::Docx => {
            let exported = docx::export_chart_data(bytes, id)?;
            Ok(Exported {
                bytes: exported.bytes,
                content_type: exported.content_type,
            })
        }
        Format::Xlsx => Err(Error::with_path(
            ErrorKind::ActionNotForFormat {
                format: "xlsx".into(),
                action: "exportChartData".into(),
            },
            "action",
        )),
        Format::Pptx => {
            let exported = pptx::export_chart_data(bytes, id)?;
            Ok(Exported {
                bytes: exported.bytes,
                content_type: exported.content_type,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inspect_reads_given_bytes_not_the_path() {
        let workbook = serde_json::json!({
            "sheets": [{ "name": "Sheet1", "cells": [{ "ref": "A1", "value": 1 }] }]
        });
        let package = xlsx::create(&workbook).unwrap();
        let request = serde_json::json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": "missing-on-purpose.xlsx"
        })
        .to_string();
        let ByteOutput::Inspect(tree) = run_bytes(&request, Some(&package), &[]).unwrap() else {
            panic!("inspect 应返回树");
        };
        assert_eq!(tree["sheets"][0]["name"], "Sheet1");
    }
}
