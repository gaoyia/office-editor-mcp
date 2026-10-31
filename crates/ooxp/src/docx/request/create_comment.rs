use serde_json::Value;

use crate::error::{Error, ErrorKind, TypeName};

use super::create_types::*;

pub(crate) fn parse_comments(
    v: Option<&Value>,
    path: &str,
    reply: bool,
) -> Result<Vec<NewComment>, Error> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    let field = if reply { "replies" } else { "comments" };
    let arr = v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Array,
            },
            path,
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
            if reply {
                if !matches!(k.as_str(), "text" | "author" | "initials" | "date") {
                    return Err(Error::with_path(
                        ErrorKind::UnknownField { field: k.clone() },
                        format!("{item_path}.{k}"),
                    ));
                }
            } else if !matches!(
                k.as_str(),
                "text" | "author" | "initials" | "date" | "replies"
            ) {
                return Err(Error::with_path(
                    ErrorKind::UnknownField { field: k.clone() },
                    format!("{item_path}.{k}"),
                ));
            }
        }
        let text = match obj.get("text") {
            None => String::new(),
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
        };
        let author =
            super::create_util::parse_comment_attr(obj.get("author"), &item_path, "author")?
                .unwrap_or_else(|| "ooxp".into());
        let initials =
            super::create_util::parse_comment_attr(obj.get("initials"), &item_path, "initials")?
                .unwrap_or_else(|| "oo".into());
        let date = super::create_util::parse_comment_attr(obj.get("date"), &item_path, "date")?;
        let replies = if reply {
            Vec::new()
        } else {
            parse_comments(obj.get("replies"), &format!("{item_path}.replies"), true)?
        };
        out.push(NewComment {
            text,
            author,
            initials,
            date,
            replies,
        });
    }
    Ok(out)
}
