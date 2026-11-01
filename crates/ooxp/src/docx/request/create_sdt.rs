use serde_json::Value;

use crate::error::{Error, ErrorKind, TypeName};

use super::super::sdt::SdtKind;
use super::create_types::*;

pub(crate) fn parse_sdts(v: Option<&Value>, prefix: &str) -> Result<Vec<NewSdt>, Error> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    let path = format!("{prefix}.contentControls");
    let arr = v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "contentControls".into(),
                expected: TypeName::Array,
            },
            path.clone(),
        )
    })?;
    let mut out = Vec::new();
    for (j, item) in arr.iter().enumerate() {
        let item_path = format!("{path}[{j}]");
        let obj = item.as_object().ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: item_path.clone(),
                    expected: TypeName::Object,
                },
                item_path.clone(),
            )
        })?;
        for k in obj.keys() {
            if !matches!(
                k.as_str(),
                "text" | "kind" | "tag" | "alias" | "items" | "fullDate" | "file" | "checked"
            ) {
                return Err(Error::with_path(
                    ErrorKind::UnknownField { field: k.clone() },
                    format!("{item_path}.{k}"),
                ));
            }
        }
        let kind = match obj.get("kind") {
            None => SdtKind::PlainText,
            Some(Value::String(s)) => SdtKind::parse(s).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::InvalidContentControlKind,
                    format!("{item_path}.kind"),
                )
            })?,
            Some(_) => {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: "kind".into(),
                        expected: TypeName::String,
                    },
                    format!("{item_path}.kind"),
                ));
            }
        };
        let text_opt = match obj.get("text") {
            Some(Value::String(s)) if !s.is_empty() => Some(s.clone()),
            Some(Value::String(_)) => {
                return Err(Error::with_path(
                    ErrorKind::EmptyContentControlText,
                    format!("{item_path}.text"),
                ));
            }
            Some(_) => {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: "text".into(),
                        expected: TypeName::String,
                    },
                    format!("{item_path}.text"),
                ));
            }
            None => None,
        };
        let tag = super::create_util::parse_optional_str(obj.get("tag"), &item_path, "tag")?;
        let alias = super::create_util::parse_optional_str(obj.get("alias"), &item_path, "alias")?;
        let items = parse_sdt_items(obj.get("items"), &item_path)?;
        if !items.is_empty() && !matches!(kind, SdtKind::DropDown | SdtKind::ComboBox) {
            return Err(Error::with_path(
                ErrorKind::ContentControlFieldMismatch,
                format!("{item_path}.items"),
            ));
        }
        let full_date =
            super::create_util::parse_optional_str(obj.get("fullDate"), &item_path, "fullDate")?;
        if full_date.is_some() && kind != SdtKind::Date {
            return Err(Error::with_path(
                ErrorKind::ContentControlFieldMismatch,
                format!("{item_path}.fullDate"),
            ));
        }
        if let Some(fd) = &full_date {
            if fd.is_empty() {
                return Err(Error::with_path(
                    ErrorKind::EmptyContentControlText,
                    format!("{item_path}.fullDate"),
                ));
            }
        }
        let file = match obj.get("file") {
            None => None,
            Some(Value::String(s)) if !s.is_empty() => Some(s.clone()),
            Some(Value::String(_)) => {
                return Err(Error::with_path(
                    ErrorKind::MissingField {
                        field: "file".into(),
                    },
                    format!("{item_path}.file"),
                ));
            }
            Some(_) => {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: "file".into(),
                        expected: TypeName::NonEmptyString,
                    },
                    format!("{item_path}.file"),
                ));
            }
        };
        if file.is_some() && kind != SdtKind::Picture {
            return Err(Error::with_path(
                ErrorKind::ContentControlFieldMismatch,
                format!("{item_path}.file"),
            ));
        }
        let checked =
            super::create_util::parse_optional_bool(obj.get("checked"), &item_path, "checked")?;
        if checked.is_some() && kind != SdtKind::Checkbox {
            return Err(Error::with_path(
                ErrorKind::ContentControlFieldMismatch,
                format!("{item_path}.checked"),
            ));
        }
        match kind {
            SdtKind::Picture => {
                if file.is_none() {
                    return Err(Error::with_path(
                        ErrorKind::MissingField {
                            field: "file".into(),
                        },
                        format!("{item_path}.file"),
                    ));
                }
                if text_opt.is_some() {
                    return Err(Error::with_path(
                        ErrorKind::ContentControlFieldMismatch,
                        format!("{item_path}.text"),
                    ));
                }
            }
            SdtKind::RepeatingSection | SdtKind::Checkbox => {
                if text_opt.is_some() {
                    return Err(Error::with_path(
                        ErrorKind::ContentControlFieldMismatch,
                        format!("{item_path}.text"),
                    ));
                }
            }
            _ => {
                if text_opt.is_none() {
                    return Err(Error::with_path(
                        ErrorKind::MissingField {
                            field: "text".into(),
                        },
                        format!("{item_path}.text"),
                    ));
                }
            }
        }
        out.push(NewSdt {
            text: text_opt.unwrap_or_default(),
            kind,
            tag,
            alias,
            items,
            full_date,
            file,
            checked: checked.unwrap_or(false),
        });
    }
    Ok(out)
}

pub(crate) fn parse_sdt_items(v: Option<&Value>, item_path: &str) -> Result<Vec<String>, Error> {
    match v {
        None => Ok(Vec::new()),
        Some(Value::Array(arr)) => {
            if arr.is_empty() {
                return Err(Error::with_path(
                    ErrorKind::InvalidContentControlItems,
                    format!("{item_path}.items"),
                ));
            }
            let mut out = Vec::new();
            for (j, item) in arr.iter().enumerate() {
                let s = item.as_str().ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::WrongType {
                            field: "items".into(),
                            expected: TypeName::String,
                        },
                        format!("{item_path}.items[{j}]"),
                    )
                })?;
                if s.is_empty() {
                    return Err(Error::with_path(
                        ErrorKind::InvalidContentControlItems,
                        format!("{item_path}.items[{j}]"),
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
            format!("{item_path}.items"),
        )),
    }
}
