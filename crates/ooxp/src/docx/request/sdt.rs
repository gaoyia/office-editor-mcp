//! docx request：sdt。

use serde_json::Value;

use crate::error::{Error, ErrorKind, IdReason, TypeName};

use super::common::*;
use super::types::*;

pub(super) fn parse_insert_sdt(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertContentControlOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op", "id", "kind", "text", "tag", "alias", "items", "fullDate", "file", "checked",
        ],
    )?;
    let id = require_op_id(obj, i)?;
    let kind = match obj.get("kind") {
        None => super::super::sdt::SdtKind::PlainText,
        Some(Value::String(s)) => super::super::sdt::SdtKind::parse(s).ok_or_else(|| {
            Error::with_path(
                ErrorKind::InvalidContentControlKind,
                format!("ops[{i}].kind"),
            )
        })?,
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "kind".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].kind"),
            ));
        }
    };
    let text_opt = parse_sdt_text_opt(obj, i)?;
    let tag = optional_op_string(obj, i, "tag")?;
    let alias = optional_op_string(obj, i, "alias")?;
    let items = parse_sdt_items(obj, i)?;
    if !items.is_empty()
        && !matches!(
            kind,
            super::super::sdt::SdtKind::DropDown | super::super::sdt::SdtKind::ComboBox
        )
    {
        return Err(Error::with_path(
            ErrorKind::ContentControlFieldMismatch,
            format!("ops[{i}].items"),
        ));
    }
    let full_date = optional_op_string(obj, i, "fullDate")?;
    if full_date.is_some() && kind != super::super::sdt::SdtKind::Date {
        return Err(Error::with_path(
            ErrorKind::ContentControlFieldMismatch,
            format!("ops[{i}].fullDate"),
        ));
    }
    if let Some(fd) = &full_date {
        if fd.is_empty() {
            return Err(Error::with_path(
                ErrorKind::EmptyContentControlText,
                format!("ops[{i}].fullDate"),
            ));
        }
    }
    let file = match obj.get("file") {
        None => None,
        Some(_) => Some(require_file(obj, i, "insertContentControl")?),
    };
    if file.is_some() && kind != super::super::sdt::SdtKind::Picture {
        return Err(Error::with_path(
            ErrorKind::ContentControlFieldMismatch,
            format!("ops[{i}].file"),
        ));
    }
    let checked = parse_optional_bool_field(obj.get("checked"), i, "checked")?;
    if checked.is_some() && kind != super::super::sdt::SdtKind::Checkbox {
        return Err(Error::with_path(
            ErrorKind::ContentControlFieldMismatch,
            format!("ops[{i}].checked"),
        ));
    }
    if kind.needs_text() {
        parse_sdt_text(obj, i, "insertContentControl")?;
    } else if text_opt.is_some() {
        return Err(Error::with_path(
            ErrorKind::ContentControlFieldMismatch,
            format!("ops[{i}].text"),
        ));
    }
    if kind == super::super::sdt::SdtKind::Picture && file.is_none() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "insertContentControl".into(),
                field: "file".into(),
            },
            format!("ops[{i}].file"),
        ));
    }
    let text = text_opt.unwrap_or_default();
    let path = format!("ops[{i}].id");
    let place = if id == "docx:document" {
        InsertSdtPlace::EndOfDocument
    } else {
        InsertSdtPlace::After(parse_story_or_box_para_at(
            &id,
            &path,
            IdReason::InsertSdtNeedsPara,
        )?)
    };
    Ok(InsertContentControlOp {
        id,
        place,
        kind,
        text,
        tag,
        alias,
        items,
        full_date,
        file,
        checked: checked.unwrap_or(false),
    })
}

pub(super) fn parse_set_sdt(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetContentControlOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op", "id", "text", "tag", "alias", "items", "fullDate", "file", "checked",
        ],
    )?;
    let id = require_op_id(obj, i)?;
    let target = super::super::sdt::parse_sdt_ref(&id, &format!("ops[{i}].id"))?;
    let text = parse_sdt_text_opt(obj, i)?;
    let tag = optional_op_string(obj, i, "tag")?;
    let alias = optional_op_string(obj, i, "alias")?;
    let items = if obj.contains_key("items") {
        Some(parse_sdt_items(obj, i)?)
    } else {
        None
    };
    let full_date = optional_op_string(obj, i, "fullDate")?;
    let file = match obj.get("file") {
        None => None,
        Some(_) => Some(require_file(obj, i, "setContentControl")?),
    };
    let checked = parse_optional_bool_field(obj.get("checked"), i, "checked")?;
    if file.is_some() && text.is_some() {
        return Err(Error::with_path(
            ErrorKind::ContentControlFieldMismatch,
            format!("ops[{i}].file"),
        ));
    }
    if checked.is_some() && (text.is_some() || file.is_some()) {
        return Err(Error::with_path(
            ErrorKind::ContentControlFieldMismatch,
            format!("ops[{i}].checked"),
        ));
    }
    Ok(SetContentControlOp {
        id,
        target,
        text,
        tag,
        alias,
        items,
        full_date,
        file,
        checked,
    })
}

pub(super) fn parse_delete_sdt(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteContentControlOp, Error> {
    deny_keys(obj, i, &["op", "id", "keep"])?;
    let id = require_op_id(obj, i)?;
    let target = super::super::sdt::parse_sdt_ref(&id, &format!("ops[{i}].id"))?;
    let keep = match obj.get("keep") {
        None => super::super::sdt::SdtKeep::Content,
        Some(Value::String(s)) => super::super::sdt::SdtKeep::parse(s).ok_or_else(|| {
            Error::with_path(
                ErrorKind::InvalidContentControlKeep,
                format!("ops[{i}].keep"),
            )
        })?,
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "keep".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].keep"),
            ));
        }
    };
    Ok(DeleteContentControlOp { id, target, keep })
}

pub(super) fn parse_sdt_text(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    op: &str,
) -> Result<String, Error> {
    match parse_sdt_text_opt(obj, i)? {
        Some(s) => Ok(s),
        None => Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: op.into(),
                field: "text".into(),
            },
            format!("ops[{i}].text"),
        )),
    }
}

pub(super) fn parse_sdt_text_opt(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<Option<String>, Error> {
    match obj.get("text") {
        Some(Value::String(s)) => {
            if s.is_empty() {
                return Err(Error::with_path(
                    ErrorKind::EmptyContentControlText,
                    format!("ops[{i}].text"),
                ));
            }
            Ok(Some(s.clone()))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "text".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].text"),
        )),
        None => Ok(None),
    }
}

pub(super) fn parse_sdt_items(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<Vec<String>, Error> {
    match obj.get("items") {
        None => Ok(Vec::new()),
        Some(Value::Array(arr)) => {
            if arr.is_empty() {
                return Err(Error::with_path(
                    ErrorKind::InvalidContentControlItems,
                    format!("ops[{i}].items"),
                ));
            }
            let mut out = Vec::new();
            for (j, v) in arr.iter().enumerate() {
                let s = v.as_str().ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::WrongType {
                            field: "items".into(),
                            expected: TypeName::String,
                        },
                        format!("ops[{i}].items[{j}]"),
                    )
                })?;
                if s.is_empty() {
                    return Err(Error::with_path(
                        ErrorKind::InvalidContentControlItems,
                        format!("ops[{i}].items[{j}]"),
                    ));
                }
                out.push(s.to_string());
            }
            Ok(out)
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "items".into(),
                expected: TypeName::Array,
            },
            format!("ops[{i}].items"),
        )),
    }
}
