//! docx request：notes。

use serde_json::Value;

use crate::error::{Error, ErrorKind, TypeName};

use super::super::para::{self, NoteIdTarget, NoteKind, ParaTarget};

use super::common::*;
use super::sect::*;
use super::types::*;

pub(super) fn parse_insert_note(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    _kind: NoteKind,
) -> Result<InsertNoteOp, Error> {
    deny_keys(obj, i, &["op", "id", "text"])?;
    let id = require_id(obj, i)?;
    let path = format!("ops[{i}].id");
    if para::parse_note_id_at(&id, &path)?.is_some() {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: crate::error::IdReason::NotePrefix,
            },
            path,
        ));
    }
    let host = para::parse_para_id_at(&id, &path)?;
    if matches!(
        host,
        ParaTarget::InNote { .. } | ParaTarget::InComment { .. }
    ) {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: crate::error::IdReason::NotePrefix,
            },
            path,
        ));
    }
    Ok(InsertNoteOp {
        id,
        host,
        text: parse_optional_text(obj, i)?,
    })
}

pub(super) fn parse_delete_note(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    kind: NoteKind,
) -> Result<DeleteNoteOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let path = format!("ops[{i}].id");
    let Some(note) = para::parse_note_id_at(&id, &path)? else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: crate::error::IdReason::NotePrefix,
            },
            path,
        ));
    };
    match note {
        NoteIdTarget::Sep => Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: crate::error::IdReason::NoteSep,
            },
            path,
        )),
        NoteIdTarget::Para { .. } => Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: crate::error::IdReason::NotePrefix,
            },
            path,
        )),
        NoteIdTarget::Container {
            kind: parsed,
            note_id,
        } => {
            if parsed != kind {
                return Err(Error::with_path(
                    ErrorKind::InvalidId {
                        reason: crate::error::IdReason::NotePrefix,
                    },
                    path,
                ));
            }
            Ok(DeleteNoteOp { id, kind, note_id })
        }
    }
}

pub(super) fn parse_set_note_numbering(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetNoteNumberingOp, Error> {
    deny_keys(
        obj,
        i,
        &["op", "id", "kind", "numFmt", "numStart", "numRestart"],
    )?;
    let id = require_id(obj, i)?;
    let target = if id == "docx:document" {
        NoteNumberTarget::Document
    } else {
        NoteNumberTarget::Section(parse_section_id(&id, i)?)
    };
    let kind_s = obj.get("kind").and_then(Value::as_str).ok_or_else(|| {
        Error::with_path(
            ErrorKind::MissingField {
                field: "kind".into(),
            },
            format!("ops[{i}].kind"),
        )
    })?;
    let kind = NoteKind::parse(kind_s)
        .ok_or_else(|| Error::with_path(ErrorKind::InvalidNoteKind, format!("ops[{i}].kind")))?;
    let num_fmt = match obj.get("numFmt") {
        None => None,
        Some(Value::String(s)) => {
            if !matches!(
                s.as_str(),
                "decimal" | "upperRoman" | "lowerRoman" | "upperLetter" | "lowerLetter" | "chicago"
            ) {
                return Err(Error::with_path(
                    ErrorKind::InvalidNoteNumFmt,
                    format!("ops[{i}].numFmt"),
                ));
            }
            Some(s.clone())
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "numFmt".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].numFmt"),
            ));
        }
    };
    let num_start = match obj.get("numStart") {
        None => None,
        Some(Value::Number(n)) => {
            let u = n.as_u64().ok_or_else(|| {
                Error::with_path(
                    ErrorKind::WrongType {
                        field: "numStart".into(),
                        expected: TypeName::Integer,
                    },
                    format!("ops[{i}].numStart"),
                )
            })?;
            if u < 1 {
                return Err(Error::with_path(
                    ErrorKind::MinValue {
                        field: "numStart".into(),
                        min: 1,
                    },
                    format!("ops[{i}].numStart"),
                ));
            }
            if u > u32::MAX as u64 {
                return Err(Error::with_path(
                    ErrorKind::MinValue {
                        field: "numStart".into(),
                        min: 1,
                    },
                    format!("ops[{i}].numStart"),
                ));
            }
            Some(u as u32)
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "numStart".into(),
                    expected: TypeName::Integer,
                },
                format!("ops[{i}].numStart"),
            ));
        }
    };
    let num_restart = match obj.get("numRestart") {
        None => None,
        Some(Value::String(s)) => {
            if !matches!(s.as_str(), "continuous" | "eachSect" | "eachPage") {
                return Err(Error::with_path(
                    ErrorKind::InvalidNoteRestart,
                    format!("ops[{i}].numRestart"),
                ));
            }
            Some(s.clone())
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "numRestart".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].numRestart"),
            ));
        }
    };
    if num_fmt.is_none() && num_start.is_none() && num_restart.is_none() {
        return Err(Error::with_path(
            ErrorKind::NeedNoteNumberingField,
            format!("ops[{i}]"),
        ));
    }
    Ok(SetNoteNumberingOp {
        id,
        target,
        kind,
        num_fmt,
        num_start,
        num_restart,
    })
}
