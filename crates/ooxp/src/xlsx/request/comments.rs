//! xlsx request：comments。

use serde_json::Value;

use crate::error::{Error, ErrorKind, IdReason, TypeName};

use super::super::comments;

use super::common::*;
use super::types::*;

pub(super) fn parse_insert_comment(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertCommentOp, Error> {
    deny_keys(obj, i, &["op", "id", "text", "author", "initials", "date"])?;
    let id = require_id(obj, i)?;
    let (sheet, a1) = comments::parse_insert_cell(&id, i)?;
    let _initials = parse_comment_str(obj, i, "initials", "oo")?;
    Ok(InsertCommentOp {
        sheet,
        a1,
        text: parse_optional_text(obj, i)?,
        author: parse_comment_str(obj, i, "author", "ooxp")?,
        date: parse_optional_date(obj, i)?,
    })
}

pub(super) fn parse_reply_comment(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<ReplyCommentOp, Error> {
    deny_keys(obj, i, &["op", "id", "text", "author", "initials", "date"])?;
    let id = require_id(obj, i)?;
    let target = comments::parse_comment_id(&id, &format!("ops[{i}].id"))?;
    if matches!(target.key, comments::CommentKey::Reply { .. }) {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::XlsxCommentForm,
            },
            format!("ops[{i}].id"),
        ));
    }
    let _initials = parse_comment_str(obj, i, "initials", "oo")?;
    Ok(ReplyCommentOp {
        id,
        text: parse_optional_text(obj, i)?,
        author: parse_comment_str(obj, i, "author", "ooxp")?,
        date: parse_optional_date(obj, i)?,
    })
}

pub(super) fn parse_delete_comment(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteCommentOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let _ = comments::parse_comment_id(&id, &format!("ops[{i}].id"))?;
    Ok(DeleteCommentOp { id })
}

pub(super) fn parse_set_comment_text(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetCommentTextOp, Error> {
    deny_keys(obj, i, &["op", "id", "text"])?;
    let id = require_id(obj, i)?;
    let _ = comments::parse_comment_id(&id, &format!("ops[{i}].id"))?;
    let text = match obj.get("text") {
        Some(Value::String(s)) => s.clone(),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "text".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].text"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::OpRequiresField {
                    op: "setCommentText".into(),
                    field: "text".into(),
                },
                format!("ops[{i}].text"),
            ));
        }
    };
    Ok(SetCommentTextOp { id, text })
}

pub(super) fn parse_set_comment_meta(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetCommentMetaOp, Error> {
    deny_keys(obj, i, &["op", "id", "author", "initials", "date"])?;
    let id = require_id(obj, i)?;
    let _ = comments::parse_comment_id(&id, &format!("ops[{i}].id"))?;
    let author = match obj.get("author") {
        None => None,
        Some(Value::String(s)) => Some(s.clone()),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "author".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].author"),
            ));
        }
    };
    let initials = match obj.get("initials") {
        None => None,
        Some(Value::String(s)) => Some(s.clone()),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "initials".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].initials"),
            ));
        }
    };
    let date = parse_optional_date(obj, i)?;
    if author.is_none() && initials.is_none() && date.is_none() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setCommentMeta".into(),
                field: "author, initials, or date".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(SetCommentMetaOp { id, author, date })
}

pub(super) fn parse_set_comment_done(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetCommentDoneOp, Error> {
    deny_keys(obj, i, &["op", "id", "done"])?;
    let id = require_id(obj, i)?;
    let _ = comments::parse_comment_id(&id, &format!("ops[{i}].id"))?;
    let done = match obj.get("done") {
        Some(Value::Bool(b)) => *b,
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "done".into(),
                    expected: TypeName::Boolean,
                },
                format!("ops[{i}].done"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::OpRequiresField {
                    op: "setCommentDone".into(),
                    field: "done".into(),
                },
                format!("ops[{i}].done"),
            ));
        }
    };
    Ok(SetCommentDoneOp { id, done })
}

pub(super) fn parse_comment_str(
    obj: &serde_json::Map<String, Value>,
    i: usize,
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
            format!("ops[{i}].{field}"),
        )),
    }
}
