//! docx request：font_embed。

use serde_json::Value;

use crate::error::{Error, ErrorKind, IdReason, TypeName};

use super::common::*;
use super::types::*;

pub(super) fn parse_insert_embedded_font(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertEmbeddedFontOp, Error> {
    deny_keys(obj, i, &["op", "id", "name", "file", "style"])?;
    let id = require_op_id(obj, i)?;
    if id != "docx:document" {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::InsertEmbeddedFontNeedsDocument,
            },
            format!("ops[{i}].id"),
        ));
    }
    let name = require_string_field(obj, i, "insertEmbeddedFont", "name")?;
    if name.is_empty() || name.chars().all(char::is_whitespace) {
        return Err(Error::with_path(
            ErrorKind::EmptyFontName,
            format!("ops[{i}].name"),
        ));
    }
    let file = require_string_field(obj, i, "insertEmbeddedFont", "file")?;
    if file.is_empty() || file.chars().all(char::is_whitespace) {
        return Err(Error::with_path(
            ErrorKind::InvalidFontFile,
            format!("ops[{i}].file"),
        ));
    }
    let style = match obj.get("style") {
        None => super::super::font_embed::FontStyle::Regular,
        Some(Value::String(s)) => {
            super::super::font_embed::FontStyle::parse(s).ok_or_else(|| {
                Error::with_path(ErrorKind::InvalidFontStyle, format!("ops[{i}].style"))
            })?
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "style".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].style"),
            ));
        }
    };
    Ok(InsertEmbeddedFontOp { name, style, file })
}

pub(super) fn parse_delete_embedded_font(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteEmbeddedFontOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_op_id(obj, i)?;
    let n = super::super::font_embed::parse_id(&id, &format!("ops[{i}].id"))?;
    Ok(DeleteEmbeddedFontOp { id, n })
}

pub(super) fn parse_set_font_embed(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetFontEmbedOp, Error> {
    deny_keys(obj, i, &["op", "id", "trueType", "systemFonts", "subset"])?;
    let id = require_op_id(obj, i)?;
    if id != "docx:document" {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SetFontEmbedNeedsDocument,
            },
            format!("ops[{i}].id"),
        ));
    }
    let true_type = parse_optional_bool_field(obj.get("trueType"), i, "trueType")?;
    let system_fonts = parse_optional_bool_field(obj.get("systemFonts"), i, "systemFonts")?;
    let subset = parse_optional_bool_field(obj.get("subset"), i, "subset")?;
    if true_type.is_none() && system_fonts.is_none() && subset.is_none() {
        return Err(Error::with_path(
            ErrorKind::NeedFontEmbedField,
            format!("ops[{i}]"),
        ));
    }
    Ok(SetFontEmbedOp {
        true_type,
        system_fonts,
        subset,
    })
}
