use serde_json::{Map, Value};

use crate::error::{Error, ErrorKind, TypeName};

use super::super::a1::{self};
use super::InsertCommentOp;

pub(crate) fn parse_comments(
    v: Option<&Value>,
    sheet_i: usize,
    sheet: &str,
) -> Result<Vec<InsertCommentOp>, Error> {
    let prefix = format!("workbook.sheets[{sheet_i}].comments");
    let Some(arr) = super::create_util::parse_object_array(v, "comments", &prefix)? else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for (j, item) in arr.iter().enumerate() {
        out.push(parse_comment(item, &format!("{prefix}[{j}]"), sheet)?);
    }
    Ok(out)
}

pub(crate) fn parse_comment(
    v: &Value,
    prefix: &str,
    sheet: &str,
) -> Result<InsertCommentOp, Error> {
    let obj = super::create_util::require_object(v, "comments", prefix)?;
    super::create_util::deny_unknown(
        obj,
        prefix,
        &["cell", "text", "author", "initials", "date", "replies"],
    )?;
    let _ = super::create_util::parse_object_array(
        obj.get("replies"),
        "replies",
        &format!("{prefix}.replies"),
    )?;
    let cell = super::create_util::parse_required_a1_field(obj.get("cell"), prefix, "cell")?;
    let text = match obj.get("text") {
        None => String::new(),
        Some(Value::String(s)) => s.clone(),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "text".into(),
                    expected: TypeName::String,
                },
                format!("{prefix}.text"),
            ));
        }
    };
    let author = parse_comment_str(obj, prefix, "author", "ooxp")?;
    let _initials = parse_comment_str(obj, prefix, "initials", "oo")?;
    let date = match obj.get("date") {
        None => None,
        Some(Value::String(s)) if !s.is_empty() => Some(s.clone()),
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "date".into(),
                    expected: TypeName::NonEmptyString,
                },
                format!("{prefix}.date"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "date".into(),
                    expected: TypeName::String,
                },
                format!("{prefix}.date"),
            ));
        }
    };
    Ok(InsertCommentOp {
        sheet: sheet.to_string(),
        a1: a1::format_cell(cell.col, cell.row),
        text,
        author,
        date,
    })
}

pub(crate) fn parse_comment_str(
    obj: &Map<String, Value>,
    prefix: &str,
    field: &str,
    default: &str,
) -> Result<String, Error> {
    match obj.get(field) {
        None => Ok(default.to_string()),
        Some(Value::String(s)) => Ok(s.clone()),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::String,
            },
            format!("{prefix}.{field}"),
        )),
    }
}
