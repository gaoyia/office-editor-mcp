//! docx request：comments。

use serde_json::Value;

use crate::error::{Error, ErrorKind, TypeName};

use super::super::para::{self, CommentIdTarget, ParaTarget};

use super::common::*;
use super::types::*;

pub(super) fn parse_comment_anchor(id: &str, path: &str) -> Result<ParaTarget, Error> {
    if para::parse_comment_id_at(id, path)?.is_some() {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: crate::error::IdReason::CommentPrefix,
            },
            path.to_string(),
        ));
    }
    para::parse_para_id_at(id, path)
}

pub(super) fn parse_comment_container(id: &str, path: &str) -> Result<String, Error> {
    let Some(comment) = para::parse_comment_id_at(id, path)? else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: crate::error::IdReason::CommentPrefix,
            },
            path.to_string(),
        ));
    };
    match comment {
        CommentIdTarget::Container { comment_id } => Ok(comment_id),
        CommentIdTarget::Para { .. } => Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: crate::error::IdReason::CommentPrefix,
            },
            path.to_string(),
        )),
    }
}

pub(super) fn parse_insert_comment(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertCommentOp, Error> {
    deny_keys(
        obj,
        i,
        &["op", "start", "end", "text", "author", "initials", "date"],
    )?;
    let start = match obj.get("start") {
        Some(Value::String(s)) if !s.is_empty() => s.clone(),
        Some(Value::String(_)) | None => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "start".into(),
                },
                format!("ops[{i}].start"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "start".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].start"),
            ));
        }
    };
    let end = match obj.get("end") {
        Some(Value::String(s)) if !s.is_empty() => s.clone(),
        Some(Value::String(_)) | None => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "end".into(),
                },
                format!("ops[{i}].end"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "end".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].end"),
            ));
        }
    };
    let start_target = parse_comment_anchor(&start, &format!("ops[{i}].start"))?;
    let end_target = parse_comment_anchor(&end, &format!("ops[{i}].end"))?;
    Ok(InsertCommentOp {
        start,
        end,
        start_target,
        end_target,
        text: parse_optional_text(obj, i)?,
        author: parse_optional_attr(obj, i, "author")?.unwrap_or_else(|| "ooxp".into()),
        initials: parse_optional_attr(obj, i, "initials")?.unwrap_or_else(|| "oo".into()),
        date: parse_optional_attr(obj, i, "date")?,
    })
}

pub(super) fn parse_reply_comment(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<ReplyCommentOp, Error> {
    deny_keys(obj, i, &["op", "id", "text", "author", "initials", "date"])?;
    let id = require_id(obj, i)?;
    let comment_id = parse_comment_container(&id, &format!("ops[{i}].id"))?;
    Ok(ReplyCommentOp {
        id,
        comment_id,
        text: parse_optional_text(obj, i)?,
        author: parse_optional_attr(obj, i, "author")?.unwrap_or_else(|| "ooxp".into()),
        initials: parse_optional_attr(obj, i, "initials")?.unwrap_or_else(|| "oo".into()),
        date: parse_optional_attr(obj, i, "date")?,
    })
}

pub(super) fn parse_delete_comment(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteCommentOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let comment_id = parse_comment_container(&id, &format!("ops[{i}].id"))?;
    Ok(DeleteCommentOp { id, comment_id })
}

pub(super) fn parse_set_comment_text(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetCommentTextOp, Error> {
    deny_keys(obj, i, &["op", "id", "text"])?;
    let id = require_id(obj, i)?;
    let comment_id = parse_comment_container(&id, &format!("ops[{i}].id"))?;
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
    Ok(SetCommentTextOp {
        id,
        comment_id,
        text,
    })
}

pub(super) fn parse_set_comment_meta(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetCommentMetaOp, Error> {
    deny_keys(obj, i, &["op", "id", "author", "initials", "date"])?;
    let id = require_id(obj, i)?;
    let comment_id = parse_comment_container(&id, &format!("ops[{i}].id"))?;
    let author = parse_optional_attr(obj, i, "author")?;
    let initials = parse_optional_attr(obj, i, "initials")?;
    let date = parse_optional_attr(obj, i, "date")?;
    if author.is_none() && initials.is_none() && date.is_none() {
        return Err(Error::with_path(
            ErrorKind::NeedCommentMetaField,
            format!("ops[{i}]"),
        ));
    }
    Ok(SetCommentMetaOp {
        id,
        comment_id,
        author,
        initials,
        date,
    })
}

pub(super) fn parse_set_comment_done(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetCommentDoneOp, Error> {
    deny_keys(obj, i, &["op", "id", "done"])?;
    let id = require_id(obj, i)?;
    let comment_id = parse_comment_container(&id, &format!("ops[{i}].id"))?;
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
    Ok(SetCommentDoneOp {
        id,
        comment_id,
        done,
    })
}
