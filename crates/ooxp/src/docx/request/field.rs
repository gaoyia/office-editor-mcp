//! docx request：field。

use serde_json::Value;

use crate::error::{Error, ErrorKind, IdReason, TypeName};

use super::super::para::{self, ParaTarget};

use super::common::*;
use super::para::*;
use super::types::*;

pub(super) fn parse_insert_toc(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertTocOp, Error> {
    deny_keys(obj, i, &["op", "id", "kind", "instr"])?;
    let id = require_op_id(obj, i)?;
    let kind = match obj.get("kind") {
        None => None,
        Some(Value::String(s)) => Some(
            super::super::field::parse_toc_kind(s)
                .map_err(|kind| Error::with_path(kind, format!("ops[{i}].kind")))?,
        ),
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
    let instr = match obj.get("instr") {
        None => None,
        Some(Value::String(s)) => {
            if s.trim().is_empty() {
                return Err(Error::with_path(
                    ErrorKind::EmptyFieldInstr,
                    format!("ops[{i}].instr"),
                ));
            }
            if !super::super::field::instr_is_toc_or_toa(s) {
                return Err(Error::with_path(
                    ErrorKind::InvalidTocKind,
                    format!("ops[{i}].instr"),
                ));
            }
            Some(s.clone())
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "instr".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].instr"),
            ));
        }
    };
    let instr = match instr {
        Some(s) => s,
        None => kind
            .unwrap_or(super::super::field::TocKind::Toc)
            .default_instr()
            .to_string(),
    };
    let path = format!("ops[{i}].id");
    let place = if id == "docx:document" {
        InsertTocPlace::EndOfDocument
    } else {
        match para::parse_para_id_at(&id, &path)? {
            ParaTarget::Body(p) => InsertTocPlace::After(p),
            _ => {
                return Err(Error::with_path(
                    ErrorKind::InvalidId {
                        reason: IdReason::InsertTocNeedsBodyPara,
                    },
                    path,
                ));
            }
        }
    };
    Ok(InsertTocOp { id, place, instr })
}

pub(super) fn parse_delete_toc(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteTocOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_op_id(obj, i)?;
    let n = parse_toc_id(&id, i)?;
    Ok(DeleteTocOp { id, n })
}

pub(super) fn parse_insert_caption(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertCaptionOp, Error> {
    deny_keys(obj, i, &["op", "id", "label", "title", "bookmark"])?;
    let id = require_op_id(obj, i)?;
    let label = match obj.get("label") {
        None => "Figure".to_string(),
        Some(Value::String(s)) => {
            super::super::field::parse_seq_identifier(s)
                .map_err(|kind| Error::with_path(kind, format!("ops[{i}].label")))?;
            s.clone()
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "label".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].label"),
            ));
        }
    };
    let title = optional_op_string(obj, i, "title")?;
    let bookmark = match obj.get("bookmark") {
        None => None,
        Some(Value::String(s)) => {
            if s.is_empty() || s.chars().any(char::is_whitespace) {
                return Err(Error::with_path(
                    ErrorKind::InvalidBookmarkName,
                    format!("ops[{i}].bookmark"),
                ));
            }
            Some(s.clone())
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "bookmark".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].bookmark"),
            ));
        }
    };
    let path = format!("ops[{i}].id");
    let place = if id == "docx:document" {
        InsertCaptionPlace::EndOfDocument
    } else {
        match para::parse_para_id_at(&id, &path)? {
            ParaTarget::Body(p) => InsertCaptionPlace::After(p),
            _ => {
                return Err(Error::with_path(
                    ErrorKind::InvalidId {
                        reason: IdReason::InsertCaptionNeedsBodyPara,
                    },
                    path,
                ));
            }
        }
    };
    Ok(InsertCaptionOp {
        id,
        place,
        label,
        title,
        bookmark,
    })
}

pub(super) fn parse_delete_caption(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteCaptionOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_op_id(obj, i)?;
    let n = parse_caption_id(&id, i)?;
    Ok(DeleteCaptionOp { id, n })
}

pub(super) fn parse_insert_cross_ref(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertCrossRefOp, Error> {
    deny_keys(obj, i, &["op", "id", "bookmark", "kind"])?;
    let id = require_op_id(obj, i)?;
    let para = parse_story_para_at(
        &id,
        &format!("ops[{i}].id"),
        IdReason::InsertCrossRefNeedsPara,
    )?;
    let bookmark = match obj.get("bookmark") {
        Some(Value::String(s)) => {
            if s.is_empty() || s.chars().any(char::is_whitespace) {
                return Err(Error::with_path(
                    ErrorKind::InvalidBookmarkName,
                    format!("ops[{i}].bookmark"),
                ));
            }
            s.clone()
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "bookmark".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].bookmark"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::OpRequiresField {
                    op: "insertCrossRef".into(),
                    field: "bookmark".into(),
                },
                format!("ops[{i}].bookmark"),
            ));
        }
    };
    let kind = match obj.get("kind") {
        None => super::super::field::CrossRefKind::Ref,
        Some(Value::String(s)) => super::super::field::parse_cross_ref_kind(s)
            .map_err(|kind| Error::with_path(kind, format!("ops[{i}].kind")))?,
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
    Ok(InsertCrossRefOp {
        id,
        para,
        bookmark,
        kind,
    })
}

pub(super) fn parse_delete_cross_ref(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteCrossRefOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_op_id(obj, i)?;
    let n = parse_xref_id(&id, i)?;
    Ok(DeleteCrossRefOp { id, n })
}

pub(super) fn parse_insert_index(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertIndexOp, Error> {
    deny_keys(obj, i, &["op", "id", "instr"])?;
    let id = require_op_id(obj, i)?;
    let instr = match obj.get("instr") {
        None => super::super::field::default_index_instr().to_string(),
        Some(Value::String(s)) => {
            if s.trim().is_empty() {
                return Err(Error::with_path(
                    ErrorKind::EmptyFieldInstr,
                    format!("ops[{i}].instr"),
                ));
            }
            if !super::super::field::instr_is_index(s) {
                return Err(Error::with_path(
                    ErrorKind::InvalidIndexInstr,
                    format!("ops[{i}].instr"),
                ));
            }
            s.clone()
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "instr".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].instr"),
            ));
        }
    };
    let path = format!("ops[{i}].id");
    let place = if id == "docx:document" {
        InsertIndexPlace::EndOfDocument
    } else {
        match para::parse_para_id_at(&id, &path)? {
            ParaTarget::Body(p) => InsertIndexPlace::After(p),
            _ => {
                return Err(Error::with_path(
                    ErrorKind::InvalidId {
                        reason: IdReason::InsertIndexNeedsBodyPara,
                    },
                    path,
                ));
            }
        }
    };
    Ok(InsertIndexOp { id, place, instr })
}

pub(super) fn parse_delete_index(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteIndexOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_op_id(obj, i)?;
    let n = parse_index_id(&id, i)?;
    Ok(DeleteIndexOp { id, n })
}

pub(super) fn parse_insert_index_entry(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertIndexEntryOp, Error> {
    deny_keys(obj, i, &["op", "id", "entry"])?;
    let id = require_op_id(obj, i)?;
    let para = parse_story_para_at(
        &id,
        &format!("ops[{i}].id"),
        IdReason::InsertIndexEntryNeedsPara,
    )?;
    let entry = match obj.get("entry") {
        Some(Value::String(s)) => {
            super::super::field::parse_index_entry(s)
                .map_err(|kind| Error::with_path(kind, format!("ops[{i}].entry")))?;
            s.clone()
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "entry".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].entry"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::OpRequiresField {
                    op: "insertIndexEntry".into(),
                    field: "entry".into(),
                },
                format!("ops[{i}].entry"),
            ));
        }
    };
    Ok(InsertIndexEntryOp { id, para, entry })
}

pub(super) fn parse_delete_index_entry(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteIndexEntryOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_op_id(obj, i)?;
    let n = parse_xe_id(&id, i)?;
    Ok(DeleteIndexEntryOp { id, n })
}

pub(super) fn parse_toc_id(id: &str, op_index: usize) -> Result<usize, Error> {
    let path = format!("ops[{op_index}].id");
    let rest = id.strip_prefix("toc:").ok_or_else(|| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::TocPrefix,
            },
            path.clone(),
        )
    })?;
    let n: usize = rest.parse().map_err(|_| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::TocIndexNotInt,
            },
            path.clone(),
        )
    })?;
    if n < 1 {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::TocIndexFromOne,
            },
            path,
        ));
    }
    Ok(n)
}

pub(super) fn parse_caption_id(id: &str, op_index: usize) -> Result<usize, Error> {
    let path = format!("ops[{op_index}].id");
    let rest = id.strip_prefix("caption:").ok_or_else(|| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::CaptionPrefix,
            },
            path.clone(),
        )
    })?;
    let n: usize = rest.parse().map_err(|_| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::CaptionIndexNotInt,
            },
            path.clone(),
        )
    })?;
    if n < 1 {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::CaptionIndexFromOne,
            },
            path,
        ));
    }
    Ok(n)
}

pub(super) fn parse_xref_id(id: &str, op_index: usize) -> Result<usize, Error> {
    let path = format!("ops[{op_index}].id");
    let rest = id.strip_prefix("xref:").ok_or_else(|| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::XrefPrefix,
            },
            path.clone(),
        )
    })?;
    let n: usize = rest.parse().map_err(|_| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::XrefIndexNotInt,
            },
            path.clone(),
        )
    })?;
    if n < 1 {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::XrefIndexFromOne,
            },
            path,
        ));
    }
    Ok(n)
}

pub(super) fn parse_index_id(id: &str, op_index: usize) -> Result<usize, Error> {
    let path = format!("ops[{op_index}].id");
    let rest = id.strip_prefix("index:").ok_or_else(|| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::IndexPrefix,
            },
            path.clone(),
        )
    })?;
    let n: usize = rest.parse().map_err(|_| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::IndexIndexNotInt,
            },
            path.clone(),
        )
    })?;
    if n < 1 {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::IndexIndexFromOne,
            },
            path,
        ));
    }
    Ok(n)
}

pub(super) fn parse_xe_id(id: &str, op_index: usize) -> Result<usize, Error> {
    let path = format!("ops[{op_index}].id");
    let rest = id.strip_prefix("xe:").ok_or_else(|| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::XePrefix,
            },
            path.clone(),
        )
    })?;
    let n: usize = rest.parse().map_err(|_| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::XeIndexNotInt,
            },
            path.clone(),
        )
    })?;
    if n < 1 {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::XeIndexFromOne,
            },
            path,
        ));
    }
    Ok(n)
}

pub(super) fn require_instr(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    missing: ErrorKind,
) -> Result<String, Error> {
    match obj.get("instr") {
        Some(Value::String(s)) => {
            if s.trim().is_empty() {
                return Err(Error::with_path(
                    ErrorKind::EmptyFieldInstr,
                    format!("ops[{i}].instr"),
                ));
            }
            Ok(s.clone())
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "instr".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].instr"),
        )),
        None => Err(Error::with_path(missing, format!("ops[{i}].instr"))),
    }
}

pub(super) fn parse_insert_field(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertFieldOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op", "id", "instr", "result", "code", "arg", "name", "items", "checked",
        ],
    )?;
    let id = require_op_id(obj, i)?;
    let para =
        parse_story_or_box_para_at(&id, &format!("ops[{i}].id"), IdReason::InsertFieldNeedsPara)?;
    let body = super::super::field::parse_field_body(obj, &format!("ops[{i}]"))?;
    Ok(InsertFieldOp {
        id,
        para,
        instr: body.instr,
        result: body.result,
        form: body.form,
    })
}

pub(super) fn parse_set_field(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetFieldOp, Error> {
    deny_keys(obj, i, &["op", "id", "instr"])?;
    let id = require_op_id(obj, i)?;
    let target = super::super::field::parse_field_ref(&id, &format!("ops[{i}].id"))?;
    let instr = require_instr(obj, i, ErrorKind::NeedSetFieldInstr)?;
    Ok(SetFieldOp { id, target, instr })
}

pub(super) fn parse_delete_field(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteFieldOp, Error> {
    deny_keys(obj, i, &["op", "id", "keep"])?;
    let id = require_op_id(obj, i)?;
    let target = super::super::field::parse_field_ref(&id, &format!("ops[{i}].id"))?;
    let keep = match obj.get("keep") {
        None => super::super::field::FieldKeep::Result,
        Some(Value::String(s)) if s == "result" => super::super::field::FieldKeep::Result,
        Some(Value::String(s)) if s == "instr" => super::super::field::FieldKeep::Instr,
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidFieldKeep,
                format!("ops[{i}].keep"),
            ));
        }
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
    Ok(DeleteFieldOp { id, target, keep })
}
