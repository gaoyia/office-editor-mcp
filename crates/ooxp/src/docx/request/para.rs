//! docx request：para。

use serde_json::Value;

use crate::error::{Error, ErrorKind, IdReason, TypeName};

use super::super::para::{self, CommentIdTarget, NoteIdTarget, ParaTarget, TxbxTarget};

use super::common::*;
use super::types::*;

pub(super) fn parse_set_para_text(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetParaTextOp, Error> {
    for k in obj.keys() {
        if !matches!(k.as_str(), "op" | "id" | "text") {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("ops[{i}].{k}"),
            ));
        }
    }
    let id = obj
        .get("id")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            Error::with_path(
                ErrorKind::MissingField { field: "id".into() },
                format!("ops[{i}].id"),
            )
        })?;
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
                    op: "setParaText".into(),
                    field: "text".into(),
                },
                format!("ops[{i}].text"),
            ));
        }
    };
    Ok(SetParaTextOp {
        id: id.to_string(),
        text,
    })
}

pub(super) fn parse_set_para_runs(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetParaRunsOp, Error> {
    for k in obj.keys() {
        if !matches!(k.as_str(), "op" | "id" | "runs") {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("ops[{i}].{k}"),
            ));
        }
    }
    let id = obj
        .get("id")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            Error::with_path(
                ErrorKind::MissingField { field: "id".into() },
                format!("ops[{i}].id"),
            )
        })?;
    let runs_v = obj.get("runs").ok_or_else(|| {
        Error::with_path(
            ErrorKind::MissingField {
                field: "runs".into(),
            },
            format!("ops[{i}].runs"),
        )
    })?;
    let arr = runs_v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "runs".into(),
                expected: TypeName::Array,
            },
            format!("ops[{i}].runs"),
        )
    })?;
    if arr.is_empty() {
        return Err(Error::with_path(
            ErrorKind::EmptyArray {
                field: "runs".into(),
            },
            format!("ops[{i}].runs"),
        ));
    }
    let mut runs = Vec::new();
    let mut mode: Option<u8> = None;
    for (j, item) in arr.iter().enumerate() {
        let item_path = format!("ops[{i}].runs[{j}]");
        let item = item.as_object().ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: item_path.clone(),
                    expected: TypeName::Object,
                },
                item_path.clone(),
            )
        })?;
        for k in item.keys() {
            if k != "text"
                && k != "match"
                && k != "after"
                && k != "before"
                && k != "nth"
                && k != "start"
                && k != "end"
                && !RUN_FORMAT_KEYS.contains(&k.as_str())
            {
                return Err(Error::with_path(
                    ErrorKind::UnknownField { field: k.clone() },
                    format!("{item_path}.{k}"),
                ));
            }
        }
        let has_text = item.contains_key("text");
        let has_match = item.contains_key("match");
        let has_after = item.contains_key("after");
        let has_before = item.contains_key("before");
        let has_nth = item.contains_key("nth");
        let has_range = item.contains_key("start") || item.contains_key("end");
        let has_anchor = has_match || has_after || has_before || has_nth;
        let chosen = [has_text, has_anchor, has_range]
            .into_iter()
            .filter(|on| *on)
            .count();
        if chosen != 1 {
            return Err(Error::with_path(
                ErrorKind::MutuallyExclusive {
                    a: "text".into(),
                    b: "match".into(),
                },
                item_path,
            ));
        }
        let this_mode = if has_text {
            0
        } else if has_anchor {
            1
        } else {
            2
        };
        if let Some(prev) = mode {
            if prev != this_mode {
                return Err(Error::with_path(
                    ErrorKind::MutuallyExclusive {
                        a: "text".into(),
                        b: "match".into(),
                    },
                    item_path,
                ));
            }
        }
        mode = Some(this_mode);
        let source = if has_text {
            let text = match item.get("text") {
                Some(Value::String(s)) => s.clone(),
                _ => {
                    return Err(Error::with_path(
                        ErrorKind::WrongType {
                            field: "text".into(),
                            expected: TypeName::String,
                        },
                        format!("{item_path}.text"),
                    ));
                }
            };
            ParaRunSource::Text(text)
        } else if has_anchor {
            let matched = if has_match {
                Some(req_nonempty(item, &item_path, "match")?)
            } else {
                None
            };
            let after = opt_nonempty(item, &item_path, "after")?;
            let before = opt_nonempty(item, &item_path, "before")?;
            if matched.is_none() && (after.is_none() || before.is_none()) {
                let field = if after.is_none() { "after" } else { "before" };
                return Err(Error::with_path(
                    ErrorKind::MissingField {
                        field: field.into(),
                    },
                    format!("{item_path}.{field}"),
                ));
            }
            let nth = match item.get("nth") {
                None => 1,
                Some(Value::Number(n)) => match n.as_u64() {
                    Some(n) if n >= 1 => n as usize,
                    _ => {
                        return Err(Error::with_path(
                            ErrorKind::OutOfRange {
                                field: "nth".into(),
                                min: 1,
                                max: i64::MAX,
                            },
                            format!("{item_path}.nth"),
                        ));
                    }
                },
                Some(_) => {
                    return Err(Error::with_path(
                        ErrorKind::WrongType {
                            field: "nth".into(),
                            expected: TypeName::Integer,
                        },
                        format!("{item_path}.nth"),
                    ));
                }
            };
            ParaRunSource::Match {
                text: matched,
                after,
                before,
                nth,
            }
        } else {
            let start = req_usize(item, &item_path, "start")?;
            let end = req_usize(item, &item_path, "end")?;
            if end < start {
                return Err(Error::with_path(
                    ErrorKind::OutOfRange {
                        field: "end".into(),
                        min: start as i64,
                        max: i64::MAX,
                    },
                    format!("{item_path}.end"),
                ));
            }
            ParaRunSource::Range { start, end }
        };
        let format = super::super::create_round5::parse_run_format(item, &item_path)?;
        runs.push(ParaRunPiece { source, format });
    }
    Ok(SetParaRunsOp {
        id: id.to_string(),
        runs,
    })
}

const RUN_FORMAT_KEYS: &[&str] = &[
    "bold",
    "italic",
    "underline",
    "sz",
    "color",
    "font",
    "eastAsiaFont",
    "em",
    "eastAsianLayout",
    "highlight",
    "strike",
    "dstrike",
    "vertAlign",
    "spacing",
    "kern",
    "scale",
];

fn req_nonempty(
    obj: &serde_json::Map<String, Value>,
    path: &str,
    key: &str,
) -> Result<String, Error> {
    match obj.get(key) {
        Some(Value::String(s)) if !s.is_empty() => Ok(s.clone()),
        Some(Value::String(_)) => Err(Error::with_path(
            ErrorKind::MissingField { field: key.into() },
            format!("{path}.{key}"),
        )),
        _ => Err(Error::with_path(
            ErrorKind::WrongType {
                field: key.into(),
                expected: TypeName::NonEmptyString,
            },
            format!("{path}.{key}"),
        )),
    }
}

fn opt_nonempty(
    obj: &serde_json::Map<String, Value>,
    path: &str,
    key: &str,
) -> Result<Option<String>, Error> {
    match obj.get(key) {
        None => Ok(None),
        Some(_) => req_nonempty(obj, path, key).map(Some),
    }
}

fn req_usize(
    obj: &serde_json::Map<String, Value>,
    path: &str,
    key: &str,
) -> Result<usize, Error> {
    match obj.get(key) {
        Some(Value::Number(n)) => n.as_u64().map(|n| n as usize).ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: key.into(),
                    expected: TypeName::Integer,
                },
                format!("{path}.{key}"),
            )
        }),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: key.into(),
                expected: TypeName::Integer,
            },
            format!("{path}.{key}"),
        )),
        None => Err(Error::with_path(
            ErrorKind::MissingField { field: key.into() },
            format!("{path}.{key}"),
        )),
    }
}

pub(super) fn parse_story_para_at(
    id: &str,
    path: &str,
    reason: IdReason,
) -> Result<ParaTarget, Error> {
    let target = para::parse_para_id_at(id, path)?;
    match target {
        ParaTarget::Body(_) | ParaTarget::HeaderFooter { .. } => Ok(target),
        _ => Err(Error::with_path(
            ErrorKind::InvalidId { reason },
            path.to_string(),
        )),
    }
}

pub(super) fn same_story(a: &ParaTarget, b: &ParaTarget) -> bool {
    match (a, b) {
        (ParaTarget::Body(_), ParaTarget::Body(_)) => true,
        (
            ParaTarget::HeaderFooter {
                kind: k1,
                hf_type: t1,
                n: n1,
                ..
            },
            ParaTarget::HeaderFooter {
                kind: k2,
                hf_type: t2,
                n: n2,
                ..
            },
        ) => k1 == k2 && t1 == t2 && n1 == n2,
        (
            ParaTarget::InTextBox {
                host: h1, key: k1, ..
            },
            ParaTarget::InTextBox {
                host: h2, key: k2, ..
            },
        ) => h1 == h2 && k1 == k2,
        _ => false,
    }
}

pub(super) fn parse_insert_para(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertParaOp, Error> {
    deny_keys(obj, i, &["op", "id", "text"])?;
    let id = require_id(obj, i)?;
    let path = format!("ops[{i}].id");
    let text = parse_optional_text(obj, i)?;
    if let Some(note) = para::parse_note_id_at(&id, &path)? {
        let place = match note {
            NoteIdTarget::Sep => {
                return Err(Error::with_path(
                    ErrorKind::InvalidId {
                        reason: crate::error::IdReason::NoteSep,
                    },
                    path,
                ));
            }
            NoteIdTarget::Container { kind, note_id } => InsertParaPlace::Note {
                kind,
                note_id,
                after: None,
            },
            NoteIdTarget::Para {
                kind,
                note_id,
                para,
            } => InsertParaPlace::Note {
                kind,
                note_id,
                after: Some(para),
            },
        };
        return Ok(InsertParaOp { id, place, text });
    }
    if let Some(comment) = para::parse_comment_id_at(&id, &path)? {
        let place = match comment {
            CommentIdTarget::Container { comment_id } => InsertParaPlace::Comment {
                comment_id,
                after: None,
            },
            CommentIdTarget::Para { comment_id, para } => InsertParaPlace::Comment {
                comment_id,
                after: Some(para),
            },
        };
        return Ok(InsertParaOp { id, place, text });
    }
    if id == "docx:document" {
        return Ok(InsertParaOp {
            id,
            place: InsertParaPlace::Body { after: None },
            text,
        });
    }
    if let Some((kind, hf_type, n)) = para::parse_hf_container_id(&id) {
        return Ok(InsertParaOp {
            id,
            place: InsertParaPlace::HeaderFooter {
                kind,
                hf_type,
                n,
                after: None,
            },
            text,
        });
    }
    if let Ok(box_target) = para::parse_txbx_id_at(&id, &path) {
        return Ok(InsertParaOp {
            id,
            place: InsertParaPlace::TextBox {
                box_target,
                after: None,
            },
            text,
        });
    }
    match para::parse_para_id_at(&id, &path)? {
        ParaTarget::InTextBox { host, key, para } => Ok(InsertParaOp {
            id,
            place: InsertParaPlace::TextBox {
                box_target: TxbxTarget {
                    host: host.clone(),
                    key: key.clone(),
                },
                after: Some(ParaTarget::InTextBox { host, key, para }),
            },
            text,
        }),
        ParaTarget::Body(after) => Ok(InsertParaOp {
            id,
            place: InsertParaPlace::Body { after: Some(after) },
            text,
        }),
        ParaTarget::HeaderFooter {
            kind,
            hf_type,
            n,
            para,
        } => Ok(InsertParaOp {
            id,
            place: InsertParaPlace::HeaderFooter {
                kind,
                hf_type,
                n,
                after: Some(para),
            },
            text,
        }),
        ParaTarget::InNote { .. } | ParaTarget::InComment { .. } => Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: crate::error::IdReason::InsertParaNeedsBox,
            },
            path,
        )),
    }
}

pub(super) fn parse_delete_para(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteParaOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let path = format!("ops[{i}].id");
    if let Some(note) = para::parse_note_id_at(&id, &path)? {
        return match note {
            NoteIdTarget::Sep => Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: crate::error::IdReason::NoteSep,
                },
                path,
            )),
            NoteIdTarget::Container { .. } => Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: crate::error::IdReason::InsertParaNeedsBox,
                },
                path,
            )),
            NoteIdTarget::Para {
                kind,
                note_id,
                para,
            } => Ok(DeleteParaOp {
                id,
                inner: ParaTarget::InNote {
                    kind,
                    note_id,
                    para,
                },
            }),
        };
    }
    if let Some(comment) = para::parse_comment_id_at(&id, &path)? {
        return match comment {
            CommentIdTarget::Container { .. } => Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: crate::error::IdReason::InsertParaNeedsBox,
                },
                path,
            )),
            CommentIdTarget::Para { comment_id, para } => Ok(DeleteParaOp {
                id,
                inner: ParaTarget::InComment { comment_id, para },
            }),
        };
    }
    match para::parse_para_id_at(&id, &path) {
        Ok(inner @ ParaTarget::InTextBox { .. })
        | Ok(inner @ ParaTarget::Body(_))
        | Ok(inner @ ParaTarget::HeaderFooter { .. }) => Ok(DeleteParaOp { id, inner }),
        Ok(_) => Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: crate::error::IdReason::InsertParaNeedsBox,
            },
            path,
        )),
        Err(e) => Err(e),
    }
}
