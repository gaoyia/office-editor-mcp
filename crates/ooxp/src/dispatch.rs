//! 按 format 把信封交给 xlsx / docx / pptx。
//! `run_bytes` 不碰磁盘。本机的 `run` 才按路径读写。

use serde_json::Value;

use crate::error::{Error, ErrorKind};
use crate::json::{
    self, deny_unknown, infer_format, require_ops, str_field, Action, Format, Request,
};
use crate::{docx, pptx, xlsx};

/// 库门口的结果。字节由调用方自己落盘。
#[derive(Debug)]
pub enum ByteOutput {
    Inspect(Value),
    Help(Value),
    DryRun {
        ops_applied: usize,
        parts: Vec<String>,
    },
    Bytes {
        bytes: Vec<u8>,
        ops_applied: Option<usize>,
        content_type: Option<String>,
    },
}

/// 不打开、不保存文件。`inspect`、`apply`、`exportMedia`、`exportChartData`、`exportChartImage`、`exportSparklineImage`、`exportSlideImage`、`exportPageImage` 必须带上包字节。
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
        Action::LayoutCheck => {
            let bytes = require_package(package)?;
            Ok(ByteOutput::Inspect(layout_check_bytes(&req, bytes)?))
        }
        Action::Create => Ok(ByteOutput::Bytes {
            bytes: create_bytes(&req)?,
            ops_applied: None,
            content_type: None,
        }),
        Action::Apply => {
            let bytes = require_package(package)?;
            if req.obj.get("dryRun").and_then(Value::as_bool) == Some(true) {
                let (parts, n) = apply_dry_run(&req, bytes)?;
                Ok(ByteOutput::DryRun {
                    ops_applied: n,
                    parts,
                })
            } else {
                let (out, n) = apply_bytes(&req, bytes)?;
                Ok(ByteOutput::Bytes {
                    bytes: out,
                    ops_applied: Some(n),
                    content_type: None,
                })
            }
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
        Action::ExportChartImage => {
            let bytes = require_package(package)?;
            let exported = export_chart_image_bytes(&req, bytes)?;
            Ok(ByteOutput::Bytes {
                bytes: exported.bytes,
                ops_applied: None,
                content_type: exported.content_type,
            })
        }
        Action::ExportSparklineImage => {
            let bytes = require_package(package)?;
            let exported = export_sparkline_image_bytes(&req, bytes)?;
            Ok(ByteOutput::Bytes {
                bytes: exported.bytes,
                ops_applied: None,
                content_type: exported.content_type,
            })
        }
        Action::ExportSlideImage => {
            let bytes = require_package(package)?;
            let exported = export_slide_image_bytes(&req, bytes)?;
            Ok(ByteOutput::Bytes {
                bytes: exported.bytes,
                ops_applied: None,
                content_type: exported.content_type,
            })
        }
        Action::ExportPageImage => {
            let bytes = require_package(package)?;
            let exported = export_page_image_bytes(&req, bytes)?;
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
pub fn run(input_json: &str) -> Result<crate::json::RunOutput, Error> {
    use std::fs;
    use std::path::PathBuf;

    use crate::error::io_path;
    use crate::json::{write_atomic, RunOutput};

    let req = json::parse_request(input_json)?;
    // 先拒掉多余字段，再读盘。缺文件不能盖过 InvalidField。
    screen(&req)?;
    let _clock = crate::host_env::install_native();
    let _files = crate::host_files::install_reader(crate::host_files::cache_read);
    let package = match req.action {
        Action::Help | Action::Create => None,
        Action::Inspect
        | Action::Apply
        | Action::ExportMedia
        | Action::ExportChartData
        | Action::ExportChartImage
        | Action::ExportSparklineImage
        | Action::ExportSlideImage
        | Action::ExportPageImage
        | Action::LayoutCheck => {
            let path = str_field(&req.obj, "path")?;
            Some(fs::read(path).map_err(|e| io_path(e, PathBuf::from(path)))?)
        }
    };
    let produced = run_bytes_inner(input_json, package.as_deref())?;
    match produced {
        ByteOutput::Inspect(tree) => Ok(RunOutput::Inspect(tree)),
        ByteOutput::Help(tree) => Ok(RunOutput::Help(tree)),
        ByteOutput::DryRun { ops_applied, parts } => Ok(RunOutput::DryRun { ops_applied, parts }),
        ByteOutput::Bytes {
            bytes,
            ops_applied,
            content_type,
        } => {
            let path = output_path(&req)?;
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
fn output_path(req: &Request) -> Result<std::path::PathBuf, Error> {
    use std::path::PathBuf;
    match req.action {
        Action::Create => req
            .obj
            .get("path")
            .and_then(Value::as_str)
            .map(PathBuf::from)
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
                .unwrap_or_else(|| PathBuf::from(path)))
        }
        Action::ExportMedia
        | Action::ExportChartData
        | Action::ExportChartImage
        | Action::ExportSparklineImage
        | Action::ExportSlideImage
        | Action::ExportPageImage => req
            .obj
            .get("output")
            .and_then(Value::as_str)
            .map(PathBuf::from)
            .ok_or_else(|| {
                Error::with_path(
                    ErrorKind::MissingField {
                        field: "output".into(),
                    },
                    "output",
                )
            }),
        Action::Inspect | Action::LayoutCheck | Action::Help => {
            Err(Error::new(ErrorKind::UnsupportedAction {
                action: "bytes".into(),
            }))
        }
    }
}

fn screen(req: &Request) -> Result<(), Error> {
    deny_unknown(&req.obj, json::envelope_fields(req.action))?;
    if matches!(
        req.action,
        Action::ExportMedia
            | Action::ExportChartData
            | Action::ExportChartImage
            | Action::ExportSparklineImage
            | Action::ExportSlideImage
            | Action::ExportPageImage
    ) {
        str_field(&req.obj, "output")?;
    }
    Ok(())
}

fn format_of(req: &Request) -> Result<Format, Error> {
    let path = str_field(&req.obj, "path")?;
    let format = req.obj.get("format").and_then(Value::as_str);
    infer_format(path, format)
}

fn for_format<T>(
    fmt: Format,
    xlsx: impl FnOnce() -> Result<T, Error>,
    docx: impl FnOnce() -> Result<T, Error>,
    pptx: impl FnOnce() -> Result<T, Error>,
) -> Result<T, Error> {
    match fmt {
        Format::Xlsx => xlsx(),
        Format::Docx => docx(),
        Format::Pptx => pptx(),
    }
}

fn require_package(package: Option<&[u8]>) -> Result<&[u8], Error> {
    package.ok_or_else(|| Error::new(ErrorKind::PackageRequired))
}

fn help_value(req: &Request) -> Result<Value, Error> {
    crate::help::help(&req.obj)
}

fn inspect_bytes(req: &Request, bytes: &[u8]) -> Result<Value, Error> {
    let fmt = format_of(req)?;
    for_format(
        fmt,
        || {
            let scope = xlsx::parse_scope(req.obj.get("scope"))?;
            xlsx::inspect(bytes, scope)
        },
        || {
            let scope = docx::parse_scope(req.obj.get("scope"))?;
            docx::inspect(bytes, scope)
        },
        || {
            let scope = pptx::parse_scope(req.obj.get("scope"))?;
            pptx::inspect(bytes, scope)
        },
    )
}

fn create_bytes(req: &Request) -> Result<Vec<u8>, Error> {
    let path = str_field(&req.obj, "path")?;
    let fmt = format_of(req)?;
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
    let bytes = for_format(
        fmt,
        || {
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
            let base = std::path::Path::new(path).parent();
            if xlsm {
                xlsx::create_in(workbook, xlsx::CT_WORKBOOK_MACRO, base)
            } else {
                xlsx::create_in(workbook, xlsx::CT_WORKBOOK, base)
            }
        },
        || {
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
            docx::create(document)
        },
        || {
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
            pptx::create(presentation)
        },
    )?;
    Ok(bytes)
}

fn apply_bytes(req: &Request, bytes: &[u8]) -> Result<(Vec<u8>, usize), Error> {
    let fmt = format_of(req)?;
    let ops = require_ops(&req.obj)?;
    for_format(
        fmt,
        || {
            let parsed = xlsx::parse_ops(ops)?;
            let base = req
                .obj
                .get("output")
                .and_then(Value::as_str)
                .or_else(|| req.obj.get("path").and_then(Value::as_str))
                .and_then(|p| std::path::Path::new(p).parent());
            Ok((xlsx::apply_in(bytes, &parsed, base)?, parsed.len()))
        },
        || {
            let parsed = docx::parse_ops(ops)?;
            Ok((docx::apply(bytes, &parsed)?, parsed.len()))
        },
        || {
            let parsed = pptx::parse_ops(ops)?;
            Ok((pptx::apply(bytes, &parsed)?, parsed.len()))
        },
    )
    .map_err(|err| with_op_object(err, ops))
}

fn apply_dry_run(req: &Request, bytes: &[u8]) -> Result<(Vec<String>, usize), Error> {
    let fmt = format_of(req)?;
    let ops = require_ops(&req.obj)?;
    for_format(
        fmt,
        || {
            let parsed = xlsx::parse_ops(ops)?;
            Ok((xlsx::dry_run(bytes, &parsed)?, parsed.len()))
        },
        || {
            let parsed = docx::parse_ops(ops)?;
            Ok((docx::dry_run(bytes, &parsed)?, parsed.len()))
        },
        || {
            let parsed = pptx::parse_ops(ops)?;
            Ok((pptx::dry_run(bytes, &parsed)?, parsed.len()))
        },
    )
    .map_err(|err| with_op_object(err, ops))
}

/// apply（含 dryRun）失败时，从出错那条 op 的顶层 JSON 读 `id` / `name` 挂到错误上。
/// 只认字符串，不读包，不解释成 `p:cNvPr/@name`；定位不到就保持缺省。
fn with_op_object(err: Error, ops: &[Value]) -> Error {
    let Some(i) = err.field_path().and_then(op_index) else {
        return err;
    };
    let Some(obj) = ops.get(i).and_then(Value::as_object) else {
        return err;
    };
    let object_id = obj.get("id").and_then(Value::as_str).map(str::to_string);
    let object_name = obj.get("name").and_then(Value::as_str).map(str::to_string);
    err.with_object(object_id, object_name)
}

fn op_index(path: &str) -> Option<usize> {
    let (digits, _) = path.strip_prefix("ops[")?.split_once(']')?;
    digits.parse().ok()
}

fn layout_check_bytes(req: &Request, bytes: &[u8]) -> Result<Value, Error> {
    match format_of(req)? {
        Format::Pptx => pptx::layout_check(bytes),
        Format::Xlsx => Err(Error::with_path(
            ErrorKind::ActionNotForFormat {
                format: "xlsx".into(),
                action: "layoutCheck".into(),
            },
            "format",
        )),
        Format::Docx => Err(Error::with_path(
            ErrorKind::ActionNotForFormat {
                format: "docx".into(),
                action: "layoutCheck".into(),
            },
            "format",
        )),
    }
}

struct Exported {
    bytes: Vec<u8>,
    content_type: Option<String>,
}

fn export_media_bytes(req: &Request, bytes: &[u8]) -> Result<Exported, Error> {
    let fmt = format_of(req)?;
    let id = str_field(&req.obj, "id")?;
    for_format(
        fmt,
        || {
            let exported = xlsx::export_media(bytes, id)?;
            Ok(Exported {
                bytes: exported.bytes,
                content_type: exported.content_type,
            })
        },
        || {
            let exported = docx::export_media(bytes, id)?;
            Ok(Exported {
                bytes: exported.bytes,
                content_type: exported.content_type,
            })
        },
        || {
            let exported = pptx::export_media(bytes, id)?;
            Ok(Exported {
                bytes: exported.bytes,
                content_type: exported.content_type,
            })
        },
    )
}

fn export_chart_image_bytes(req: &Request, bytes: &[u8]) -> Result<Exported, Error> {
    let fmt = format_of(req)?;
    let id = str_field(&req.obj, "id")?;
    for_format(
        fmt,
        || {
            let exported = xlsx::export_chart_image(bytes, id)?;
            Ok(Exported {
                bytes: exported.bytes,
                content_type: exported.content_type,
            })
        },
        || {
            Err(Error::with_path(
                ErrorKind::ActionNotForFormat {
                    format: "docx".into(),
                    action: "exportChartImage".into(),
                },
                "action",
            ))
        },
        || {
            Err(Error::with_path(
                ErrorKind::ActionNotForFormat {
                    format: "pptx".into(),
                    action: "exportChartImage".into(),
                },
                "action",
            ))
        },
    )
}

fn export_sparkline_image_bytes(req: &Request, bytes: &[u8]) -> Result<Exported, Error> {
    let fmt = format_of(req)?;
    let id = str_field(&req.obj, "id")?;
    for_format(
        fmt,
        || {
            let exported = xlsx::export_sparkline_image(bytes, id)?;
            Ok(Exported {
                bytes: exported.bytes,
                content_type: exported.content_type,
            })
        },
        || {
            Err(Error::with_path(
                ErrorKind::ActionNotForFormat {
                    format: "docx".into(),
                    action: "exportSparklineImage".into(),
                },
                "action",
            ))
        },
        || {
            Err(Error::with_path(
                ErrorKind::ActionNotForFormat {
                    format: "pptx".into(),
                    action: "exportSparklineImage".into(),
                },
                "action",
            ))
        },
    )
}

fn export_slide_image_bytes(req: &Request, bytes: &[u8]) -> Result<Exported, Error> {
    let fmt = format_of(req)?;
    let id = str_field(&req.obj, "id")?;
    for_format(
        fmt,
        || {
            Err(Error::with_path(
                ErrorKind::ActionNotForFormat {
                    format: "xlsx".into(),
                    action: "exportSlideImage".into(),
                },
                "action",
            ))
        },
        || {
            Err(Error::with_path(
                ErrorKind::ActionNotForFormat {
                    format: "docx".into(),
                    action: "exportSlideImage".into(),
                },
                "action",
            ))
        },
        || {
            let exported = pptx::export_slide_image(bytes, id)?;
            Ok(Exported {
                bytes: exported.bytes,
                content_type: exported.content_type,
            })
        },
    )
}

fn export_page_image_bytes(req: &Request, bytes: &[u8]) -> Result<Exported, Error> {
    let fmt = format_of(req)?;
    let id = str_field(&req.obj, "id")?;
    for_format(
        fmt,
        || {
            Err(Error::with_path(
                ErrorKind::ActionNotForFormat {
                    format: "xlsx".into(),
                    action: "exportPageImage".into(),
                },
                "action",
            ))
        },
        || {
            let exported = docx::export_page_image(bytes, id)?;
            Ok(Exported {
                bytes: exported.bytes,
                content_type: exported.content_type,
            })
        },
        || {
            Err(Error::with_path(
                ErrorKind::ActionNotForFormat {
                    format: "pptx".into(),
                    action: "exportPageImage".into(),
                },
                "action",
            ))
        },
    )
}

fn export_chart_bytes(req: &Request, bytes: &[u8]) -> Result<Exported, Error> {
    let fmt = format_of(req)?;
    let id = str_field(&req.obj, "id")?;
    for_format(
        fmt,
        || {
            Err(Error::with_path(
                ErrorKind::ActionNotForFormat {
                    format: "xlsx".into(),
                    action: "exportChartData".into(),
                },
                "action",
            ))
        },
        || {
            let exported = docx::export_chart_data(bytes, id)?;
            Ok(Exported {
                bytes: exported.bytes,
                content_type: exported.content_type,
            })
        },
        || {
            let exported = pptx::export_chart_data(bytes, id)?;
            Ok(Exported {
                bytes: exported.bytes,
                content_type: exported.content_type,
            })
        },
    )
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

    #[test]
    fn inspect_without_package_bytes_is_invalid_field() {
        let request = r#"{"schemaVersion":1,"action":"inspect","path":"a.xlsx"}"#;
        let err = run_bytes(request, None, &[]).unwrap_err();
        assert_eq!(err.code(), crate::error::ErrorCode::InvalidField);
        let body = err.to_json(crate::Lang::En);
        assert_eq!(body["error"]["message"], "Package bytes are required");
        assert!(body["error"].get("path").is_none());
    }
}
