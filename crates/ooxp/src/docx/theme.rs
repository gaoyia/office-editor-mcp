//! 主文档 theme：inspect 根上 `theme`，apply `setThemeColors` / `setThemeFonts`。
//! 不新建部件，不算主题排版引擎，不重写 `fmtScheme`。

use serde_json::{Map, Value};

use crate::error::{Error, ErrorKind, IdReason, TypeName};
use crate::opc::{self, Package, Relationships};
use crate::theme::FontSlot;
use crate::xml;

#[derive(Debug, Clone)]
pub struct SetThemeColorsOp {
    pub slots: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub struct SetThemeFontsOp {
    pub fonts: Vec<(FontSlot, String)>,
}

pub fn inspect(pkg: &Package, doc_part: &str) -> Result<Value, Error> {
    let Some(part) = theme_part(pkg, doc_part)? else {
        return Ok(Value::Null);
    };
    let doc = xml::parse(pkg.get(&part)?)?;
    Ok(crate::theme::inspect(&doc.root))
}

pub fn set_colors(
    pkg: &mut Package,
    doc_part: &str,
    op: &SetThemeColorsOp,
    op_index: usize,
) -> Result<(), Error> {
    let json_path = format!("ops[{op_index}].id");
    let part = theme_part(pkg, doc_part)?
        .ok_or_else(|| Error::with_path(ErrorKind::DocumentThemeNotFound, json_path.clone()))?;
    let mut doc = xml::parse(pkg.get(&part)?)?;
    if !crate::theme::patch_colors(&mut doc.root, &op.slots) {
        return Err(Error::with_path(
            ErrorKind::DocumentThemeNotFound,
            json_path,
        ));
    }
    pkg.set(&part, doc.to_bytes()?)?;
    Ok(())
}

pub fn set_fonts(
    pkg: &mut Package,
    doc_part: &str,
    op: &SetThemeFontsOp,
    op_index: usize,
) -> Result<(), Error> {
    let json_path = format!("ops[{op_index}].id");
    let part = theme_part(pkg, doc_part)?
        .ok_or_else(|| Error::with_path(ErrorKind::DocumentThemeNotFound, json_path.clone()))?;
    let mut doc = xml::parse(pkg.get(&part)?)?;
    crate::theme::patch_fonts(&mut doc.root, &op.fonts, &json_path)?;
    pkg.set(&part, doc.to_bytes()?)?;
    Ok(())
}

pub fn parse_set_colors(obj: &Map<String, Value>, i: usize) -> Result<SetThemeColorsOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op", "id", "dk1", "lt1", "dk2", "lt2", "accent1", "accent2", "accent3", "accent4",
            "accent5", "accent6", "hlink", "folHlink",
        ],
    )?;
    require_document_id(obj, i)?;
    let mut slots = Vec::new();
    for slot in crate::theme::COLOR_SLOTS {
        if let Some(v) = obj.get(*slot) {
            slots.push(((*slot).to_string(), parse_hex_slot(v, i, slot)?));
        }
    }
    if slots.is_empty() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setThemeColors".into(),
                field: "a color slot".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(SetThemeColorsOp { slots })
}

pub fn parse_set_fonts(obj: &Map<String, Value>, i: usize) -> Result<SetThemeFontsOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op",
            "id",
            "majorLatin",
            "minorLatin",
            "majorEa",
            "minorEa",
            "majorCs",
            "minorCs",
        ],
    )?;
    require_document_id(obj, i)?;
    let mut fonts = Vec::new();
    if let Some(s) = parse_theme_face(obj, i, "majorLatin", false)? {
        fonts.push((FontSlot::MajorLatin, s));
    }
    if let Some(s) = parse_theme_face(obj, i, "minorLatin", false)? {
        fonts.push((FontSlot::MinorLatin, s));
    }
    if let Some(s) = parse_theme_face(obj, i, "majorEa", true)? {
        fonts.push((FontSlot::MajorEa, s));
    }
    if let Some(s) = parse_theme_face(obj, i, "minorEa", true)? {
        fonts.push((FontSlot::MinorEa, s));
    }
    if let Some(s) = parse_theme_face(obj, i, "majorCs", true)? {
        fonts.push((FontSlot::MajorCs, s));
    }
    if let Some(s) = parse_theme_face(obj, i, "minorCs", true)? {
        fonts.push((FontSlot::MinorCs, s));
    }
    if fonts.is_empty() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setThemeFonts".into(),
                field: "a font slot".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(SetThemeFontsOp { fonts })
}

fn require_document_id(obj: &Map<String, Value>, i: usize) -> Result<(), Error> {
    let id = match obj.get("id") {
        Some(Value::String(s)) if !s.is_empty() => s.as_str(),
        Some(Value::String(_)) | None => {
            return Err(Error::with_path(
                ErrorKind::MissingField { field: "id".into() },
                format!("ops[{i}].id"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "id".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].id"),
            ));
        }
    };
    if id != "docx:document" {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SetThemeNeedsDocument,
            },
            format!("ops[{i}].id"),
        ));
    }
    Ok(())
}

fn parse_hex_slot(v: &Value, i: usize, field: &str) -> Result<String, Error> {
    match v {
        Value::String(s) if s.len() == 6 && s.chars().all(|c| c.is_ascii_hexdigit()) => {
            Ok(s.clone())
        }
        Value::String(_) => Err(Error::with_path(
            ErrorKind::InvalidHexColor,
            format!("ops[{i}].{field}"),
        )),
        _ => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].{field}"),
        )),
    }
}

fn parse_theme_face(
    obj: &Map<String, Value>,
    i: usize,
    field: &str,
    allow_empty: bool,
) -> Result<Option<String>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::String(s)) if !s.is_empty() || allow_empty => Ok(Some(s.clone())),
        Some(Value::String(_)) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::NonEmptyString,
            },
            format!("ops[{i}].{field}"),
        )),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].{field}"),
        )),
    }
}

fn deny_keys(obj: &Map<String, Value>, i: usize, allowed: &[&str]) -> Result<(), Error> {
    for k in obj.keys() {
        if !allowed.contains(&k.as_str()) {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("ops[{i}].{k}"),
            ));
        }
    }
    Ok(())
}

fn theme_part(pkg: &Package, doc_part: &str) -> Result<Option<String>, Error> {
    let rels = Relationships::from_package(pkg, doc_part)?;
    let Some(rel) = rels.rels.iter().find(|r| r.rel_type.ends_with("/theme")) else {
        return Ok(None);
    };
    Ok(Some(opc::resolve_id(pkg, doc_part, &rel.id)?))
}
