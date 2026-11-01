//! docx request：biblio。

use serde_json::Value;

use crate::error::{Error, ErrorKind, IdReason, TypeName};

use super::super::para::{self, ParaTarget};

use super::common::*;
use super::types::*;

pub(super) fn parse_insert_source(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertSourceOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op",
            "id",
            "tag",
            "sourceType",
            "title",
            "year",
            "last",
            "first",
            "city",
            "publisher",
        ],
    )?;
    let id = require_op_id(obj, i)?;
    if id != "docx:document" {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::InsertSourceNeedsDocument,
            },
            format!("ops[{i}].id"),
        ));
    }
    let tag = require_string_field(obj, i, "insertSource", "tag")?;
    if super::super::biblio::tag_invalid(&tag) {
        return Err(Error::with_path(
            ErrorKind::InvalidSourceTag,
            format!("ops[{i}].tag"),
        ));
    }
    let source_type = match obj.get("sourceType") {
        None => super::super::biblio::DEFAULT_SOURCE_TYPE.to_string(),
        Some(Value::String(s)) => {
            if !super::super::biblio::source_type_ok(s) {
                return Err(Error::with_path(
                    ErrorKind::InvalidSourceType,
                    format!("ops[{i}].sourceType"),
                ));
            }
            s.clone()
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "sourceType".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].sourceType"),
            ));
        }
    };
    let title = require_string_field(obj, i, "insertSource", "title")?;
    if title.is_empty() || title.chars().all(char::is_whitespace) {
        return Err(Error::with_path(
            ErrorKind::EmptySourceTitle,
            format!("ops[{i}].title"),
        ));
    }
    Ok(InsertSourceOp {
        tag,
        source_type,
        title,
        year: optional_string(obj, i, "year")?,
        last: optional_string(obj, i, "last")?,
        first: optional_string(obj, i, "first")?,
        city: optional_string(obj, i, "city")?,
        publisher: optional_string(obj, i, "publisher")?,
    })
}

pub(super) fn parse_delete_source(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteSourceOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_op_id(obj, i)?;
    let n = parse_source_id(&id, i)?;
    Ok(DeleteSourceOp { id, n })
}

pub(super) fn parse_insert_citation(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertCitationOp, Error> {
    deny_keys(obj, i, &["op", "id", "tag"])?;
    let id = require_op_id(obj, i)?;
    let para = para::parse_para_id_at(&id, &format!("ops[{i}].id"))?;
    let tag = require_string_field(obj, i, "insertCitation", "tag")?;
    if super::super::biblio::tag_invalid(&tag) {
        return Err(Error::with_path(
            ErrorKind::InvalidCitationTag,
            format!("ops[{i}].tag"),
        ));
    }
    Ok(InsertCitationOp { id, para, tag })
}

pub(super) fn parse_delete_citation(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteCitationOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_op_id(obj, i)?;
    let (host, n) = super::field::parse_block_id(
        &id,
        "cite",
        i,
        IdReason::CitePrefix,
        IdReason::CiteIndexNotInt,
        IdReason::CiteIndexFromOne,
    )?;
    Ok(DeleteCitationOp { id, host, n })
}

pub(super) fn parse_insert_bibliography(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertBibliographyOp, Error> {
    deny_keys(obj, i, &["op", "id", "instr"])?;
    let id = require_op_id(obj, i)?;
    let instr = match obj.get("instr") {
        None => super::super::field::default_bibliography_instr().to_string(),
        Some(Value::String(s)) => {
            if s.trim().is_empty() {
                return Err(Error::with_path(
                    ErrorKind::EmptyFieldInstr,
                    format!("ops[{i}].instr"),
                ));
            }
            if !super::super::field::instr_is_bibliography(s) {
                return Err(Error::with_path(
                    ErrorKind::InvalidBibliographyInstr,
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
        InsertBibliographyPlace::EndOfDocument
    } else {
        InsertBibliographyPlace::After(para::parse_para_id_at(&id, &path)?)
    };
    Ok(InsertBibliographyOp { id, place, instr })
}

pub(super) fn parse_delete_bibliography(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteBibliographyOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_op_id(obj, i)?;
    let (host, n) = super::field::parse_block_id(
        &id,
        "biblio",
        i,
        IdReason::BiblioPrefix,
        IdReason::BiblioIndexNotInt,
        IdReason::BiblioIndexFromOne,
    )?;
    Ok(DeleteBibliographyOp { id, host, n })
}

pub(super) fn parse_set_bibliography_style(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetBibliographyStyleOp, Error> {
    deny_keys(obj, i, &["op", "id", "styleName"])?;
    let id = require_op_id(obj, i)?;
    require_document_id(&id, i, IdReason::SetBibliographyStyleNeedsDocument)?;
    let style_name = require_string_field(obj, i, "setBibliographyStyle", "styleName")?;
    if !super::super::biblio::style_ok(&style_name) {
        return Err(Error::with_path(
            ErrorKind::InvalidBibliographyStyle,
            format!("ops[{i}].styleName"),
        ));
    }
    Ok(SetBibliographyStyleOp { style_name })
}

pub(super) fn parse_insert_doc_part(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertDocPartOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op", "id", "name", "text", "gallery", "category", "behavior",
        ],
    )?;
    let id = require_op_id(obj, i)?;
    if id != "docx:document" {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::InsertDocPartNeedsDocument,
            },
            format!("ops[{i}].id"),
        ));
    }
    let name = require_string_field(obj, i, "insertDocPart", "name")?;
    if name.is_empty() || name.chars().all(char::is_whitespace) {
        return Err(Error::with_path(
            ErrorKind::EmptyDocPartName,
            format!("ops[{i}].name"),
        ));
    }
    let text = require_string_field(obj, i, "insertDocPart", "text")?;
    if text.is_empty() || text.chars().all(char::is_whitespace) {
        return Err(Error::with_path(
            ErrorKind::EmptyDocPartText,
            format!("ops[{i}].text"),
        ));
    }
    let gallery = match obj.get("gallery") {
        None => super::super::glossary::DEFAULT_GALLERY.to_string(),
        Some(Value::String(s)) => {
            if !super::super::glossary::gallery_ok(s) {
                return Err(Error::with_path(
                    ErrorKind::InvalidDocPartGallery,
                    format!("ops[{i}].gallery"),
                ));
            }
            s.clone()
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "gallery".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].gallery"),
            ));
        }
    };
    let category = match obj.get("category") {
        None => super::super::glossary::DEFAULT_CATEGORY.to_string(),
        Some(Value::String(s)) => {
            if s.is_empty() || s.chars().all(char::is_whitespace) {
                return Err(Error::with_path(
                    ErrorKind::EmptyDocPartName,
                    format!("ops[{i}].category"),
                ));
            }
            s.clone()
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "category".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].category"),
            ));
        }
    };
    let behavior = match obj.get("behavior") {
        None => super::super::glossary::DEFAULT_BEHAVIOR.to_string(),
        Some(Value::String(s)) => {
            if !super::super::glossary::behavior_ok(s) {
                return Err(Error::with_path(
                    ErrorKind::InvalidDocPartBehavior,
                    format!("ops[{i}].behavior"),
                ));
            }
            s.clone()
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "behavior".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].behavior"),
            ));
        }
    };
    Ok(InsertDocPartOp {
        name,
        gallery,
        category,
        behavior,
        text,
    })
}

pub(super) fn parse_delete_doc_part(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteDocPartOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_op_id(obj, i)?;
    let n = parse_doc_part_id(&id, i)?;
    Ok(DeleteDocPartOp { id, n })
}

pub(super) fn parse_insert_doc_part_content(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertDocPartContentOp, Error> {
    deny_keys(obj, i, &["op", "id", "docPart"])?;
    let id = require_op_id(obj, i)?;
    let part_id = match obj.get("docPart") {
        Some(Value::String(s)) => s.clone(),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "docPart".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].docPart"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::OpRequiresField {
                    op: "insertDocPartContent".into(),
                    field: "docPart".into(),
                },
                format!("ops[{i}].docPart"),
            ));
        }
    };
    let n = parse_doc_part_id_at(&part_id, &format!("ops[{i}].docPart"))?;
    let path = format!("ops[{i}].id");
    let place = if id == "docx:document" {
        InsertDocPartContentPlace::EndOfDocument
    } else {
        match para::parse_para_id_at(&id, &path)? {
            ParaTarget::Body(p) => InsertDocPartContentPlace::After(p),
            _ => {
                return Err(Error::with_path(
                    ErrorKind::InvalidId {
                        reason: IdReason::InsertDocPartContentNeedsBodyPara,
                    },
                    path,
                ));
            }
        }
    };
    Ok(InsertDocPartContentOp {
        id,
        place,
        n,
        part_id,
    })
}

pub(super) fn parse_doc_part_id(id: &str, op_index: usize) -> Result<usize, Error> {
    parse_doc_part_id_at(id, &format!("ops[{op_index}].id"))
}

pub(super) fn parse_doc_part_id_at(id: &str, path: &str) -> Result<usize, Error> {
    let rest = id.strip_prefix("docPart:").ok_or_else(|| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::DocPartPrefix,
            },
            path.to_string(),
        )
    })?;
    let n: usize = rest.parse().map_err(|_| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::DocPartIndexNotInt,
            },
            path.to_string(),
        )
    })?;
    if n < 1 {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::DocPartIndexFromOne,
            },
            path.to_string(),
        ));
    }
    Ok(n)
}

pub(super) fn parse_source_id(id: &str, op_index: usize) -> Result<usize, Error> {
    let path = format!("ops[{op_index}].id");
    let rest = id.strip_prefix("source:").ok_or_else(|| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SourcePrefix,
            },
            path.clone(),
        )
    })?;
    let n: usize = rest.parse().map_err(|_| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SourceIndexNotInt,
            },
            path.clone(),
        )
    })?;
    if n < 1 {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SourceIndexFromOne,
            },
            path,
        ));
    }
    Ok(n)
}
