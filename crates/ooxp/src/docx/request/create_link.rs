use serde_json::Value;

use crate::error::{Error, ErrorKind, TypeName};

use super::create_types::*;

pub(crate) fn parse_hyperlinks(
    v: Option<&Value>,
    prefix: &str,
) -> Result<Vec<NewHyperlink>, Error> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    let path = format!("{prefix}.hyperlinks");
    let arr = v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "hyperlinks".into(),
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
            if !matches!(k.as_str(), "text" | "url" | "anchor" | "tooltip") {
                return Err(Error::with_path(
                    ErrorKind::UnknownField { field: k.clone() },
                    format!("{item_path}.{k}"),
                ));
            }
        }
        let text = match obj.get("text") {
            Some(Value::String(s)) => s.clone(),
            Some(_) => {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: "text".into(),
                        expected: TypeName::String,
                    },
                    format!("{item_path}.text"),
                ));
            }
            None => {
                return Err(Error::with_path(
                    ErrorKind::MissingField {
                        field: "text".into(),
                    },
                    format!("{item_path}.text"),
                ));
            }
        };
        let url = super::create_util::parse_optional_str(obj.get("url"), &item_path, "url")?;
        let anchor =
            super::create_util::parse_optional_str(obj.get("anchor"), &item_path, "anchor")?;
        match (&url, &anchor) {
            (Some(_), Some(_)) => {
                return Err(Error::with_path(
                    ErrorKind::MutuallyExclusive {
                        a: "url".into(),
                        b: "anchor".into(),
                    },
                    item_path.clone(),
                ));
            }
            (None, None) => {
                return Err(Error::with_path(ErrorKind::HyperlinkNeedTarget, item_path));
            }
            _ => {}
        }
        if let Some(u) = url.as_deref() {
            if u.is_empty() {
                return Err(Error::with_path(
                    ErrorKind::EmptyHyperlinkUrl,
                    format!("{item_path}.url"),
                ));
            }
        }
        if let Some(a) = anchor.as_deref() {
            if a.is_empty() || a.starts_with('_') || a.chars().any(char::is_whitespace) {
                return Err(Error::with_path(
                    ErrorKind::InvalidBookmarkName,
                    format!("{item_path}.anchor"),
                ));
            }
        }
        let tooltip =
            super::create_util::parse_optional_str(obj.get("tooltip"), &item_path, "tooltip")?;
        out.push(NewHyperlink {
            text,
            url,
            anchor,
            tooltip,
        });
    }
    Ok(out)
}

pub(crate) fn parse_bookmarks(v: Option<&Value>, prefix: &str) -> Result<Vec<NewBookmark>, Error> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    let path = format!("{prefix}.bookmarks");
    let arr = v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "bookmarks".into(),
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
            if !matches!(k.as_str(), "name" | "end") {
                return Err(Error::with_path(
                    ErrorKind::UnknownField { field: k.clone() },
                    format!("{item_path}.{k}"),
                ));
            }
        }
        let name = match obj.get("name") {
            None => None,
            Some(Value::String(s)) => {
                if s.is_empty() || s.starts_with('_') || s.chars().any(char::is_whitespace) {
                    return Err(Error::with_path(
                        ErrorKind::InvalidBookmarkName,
                        format!("{item_path}.name"),
                    ));
                }
                Some(s.clone())
            }
            Some(_) => {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: "name".into(),
                        expected: TypeName::String,
                    },
                    format!("{item_path}.name"),
                ));
            }
        };
        let end = super::create_util::parse_index1(obj.get("end"), &format!("{item_path}.end"))?;
        out.push(NewBookmark { name, end });
    }
    Ok(out)
}
