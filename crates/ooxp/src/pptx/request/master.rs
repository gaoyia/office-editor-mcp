//! pptx request：master。

use serde_json::{Map, Value};

use crate::error::{Error, ErrorKind, IdReason, TypeName};

use super::common::*;
use super::slides::*;
use super::types::*;

pub(super) fn parse_set_slide_layout(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<SetSlideLayoutOp, Error> {
    deny_keys(obj, i, &["op", "id", "layout"])?;
    let id = require_id(obj, i)?;
    let layout = match obj.get("layout") {
        Some(Value::String(s)) if !s.is_empty() => s.clone(),
        Some(Value::String(_)) | None => {
            return Err(Error::with_path(
                ErrorKind::OpRequiresField {
                    op: "setSlideLayout".into(),
                    field: "layout".into(),
                },
                format!("ops[{i}].layout"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "layout".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].layout"),
            ));
        }
    };
    let (master, layout_n) = super::super::master::parse_layout_ref(&layout, i)?;
    Ok(SetSlideLayoutOp {
        slide: parse_slide_only_id(&id, i, "id")?,
        master,
        layout: layout_n,
    })
}

pub(super) fn parse_insert_layout(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<InsertLayoutOp, Error> {
    deny_keys(obj, i, &["op", "id", "name"])?;
    let id = require_id(obj, i)?;
    let master =
        super::super::master::parse_master_only_id(&id, i, IdReason::InsertLayoutNeedsMaster)?;
    let name = match obj.get("name") {
        None => "空白".to_string(),
        Some(Value::String(s)) if !s.is_empty() => s.clone(),
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "name".into(),
                    expected: TypeName::NonEmptyString,
                },
                format!("ops[{i}].name"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "name".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].name"),
            ));
        }
    };
    Ok(InsertLayoutOp { master, name })
}

pub(super) fn parse_delete_layout(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<DeleteLayoutOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let (master, layout) = super::super::master::parse_layout_ref_at(&id, &format!("ops[{i}].id"))?;
    Ok(DeleteLayoutOp { master, layout })
}

pub(super) fn parse_insert_placeholder(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<InsertPlaceholderOp, Error> {
    deny_keys(obj, i, &["op", "id", "type", "text", "x", "y", "cx", "cy"])?;
    let id = require_id(obj, i)?;
    let host = super::super::master::parse_insert_placeholder_host(&id, i)?;
    let type_name = match obj.get("type") {
        None => {
            return Err(Error::with_path(
                ErrorKind::OpRequiresField {
                    op: "insertPlaceholder".into(),
                    field: "type".into(),
                },
                format!("ops[{i}].type"),
            ));
        }
        Some(Value::String(s)) if super::super::master::is_writable_placeholder_type(s) => {
            s.clone()
        }
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidPlaceholderType,
                format!("ops[{i}].type"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "type".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].type"),
            ));
        }
    };
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
        None => String::new(),
    };
    let x = parse_off_field(obj.get("x"), i, "x")?;
    let y = parse_off_field(obj.get("y"), i, "y")?;
    if x.is_some() != y.is_some() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "insertPlaceholder".into(),
                field: "x and y together".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    let cx = parse_emu_field(obj.get("cx"), i, "cx")?;
    let cy = parse_emu_field(obj.get("cy"), i, "cy")?;
    if cx.is_some() != cy.is_some() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "insertPlaceholder".into(),
                field: "cx and cy together".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(InsertPlaceholderOp {
        host,
        type_name,
        text,
        x,
        y,
        cx,
        cy,
    })
}

pub(super) fn parse_delete_placeholder(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<DeletePlaceholderOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let placeholder = super::super::master::parse_placeholder_id(&id, i)?;
    Ok(DeletePlaceholderOp { id, placeholder })
}

pub(super) fn parse_insert_master(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<InsertMasterOp, Error> {
    deny_keys(obj, i, &["op", "id", "from"])?;
    let id = require_id(obj, i)?;
    if id != "pptx:presentation" {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::InsertMasterNeedsPresentation,
            },
            format!("ops[{i}].id"),
        ));
    }
    let from = match obj.get("from") {
        None => 1usize,
        Some(Value::String(s)) => super::super::master::parse_master_only_at(
            s,
            &format!("ops[{i}].from"),
            IdReason::MasterForm,
        )?,
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "from".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].from"),
            ));
        }
    };
    Ok(InsertMasterOp { from })
}

pub(super) fn parse_delete_master(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<DeleteMasterOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    Ok(DeleteMasterOp {
        master: super::super::master::parse_master_only_id(
            &id,
            i,
            IdReason::DeleteMasterNeedsMaster,
        )?,
    })
}

pub(super) fn parse_hex_slot(v: &Value, i: usize, field: &str) -> Result<String, Error> {
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

pub(super) fn parse_theme_host(
    id: &str,
    i: usize,
) -> Result<super::super::theme::ThemeHost, Error> {
    if id == "notesMaster" {
        return Ok(super::super::theme::ThemeHost::NotesMaster);
    }
    if id == "handout" {
        return Ok(super::super::theme::ThemeHost::Handout);
    }
    let master = super::super::master::parse_master_only_id(id, i, IdReason::SetThemeNeedsMaster)?;
    Ok(super::super::theme::ThemeHost::Master(master))
}

pub(super) fn parse_set_theme_colors(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<SetThemeColorsOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op",
            "id",
            "name",
            "themeName",
            "dk1",
            "lt1",
            "dk2",
            "lt2",
            "accent1",
            "accent2",
            "accent3",
            "accent4",
            "accent5",
            "accent6",
            "hlink",
            "folHlink",
        ],
    )?;
    let id = require_id(obj, i)?;
    let host = parse_theme_host(&id, i)?;
    let mut slots = Vec::new();
    for slot in super::super::theme::COLOR_SLOTS {
        if let Some(v) = obj.get(*slot) {
            slots.push(((*slot).to_string(), parse_hex_slot(v, i, slot)?));
        }
    }
    let name = crate::theme::ops::parse_opt_null_name(obj, i, "name")?;
    let theme_name = crate::theme::ops::parse_opt_null_name(obj, i, "themeName")?;
    if slots.is_empty() && name.is_none() && theme_name.is_none() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setThemeColors".into(),
                field: "a color slot, name, or themeName".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(SetThemeColorsOp {
        host,
        slots,
        name,
        theme_name,
    })
}

pub(super) fn parse_theme_face(
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

pub(super) fn parse_set_theme_fonts(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<SetThemeFontsOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op",
            "id",
            "name",
            "majorLatin",
            "minorLatin",
            "majorEa",
            "minorEa",
            "majorCs",
            "minorCs",
            "majorScripts",
            "minorScripts",
        ],
    )?;
    let id = require_id(obj, i)?;
    let host = parse_theme_host(&id, i)?;
    let mut fonts = Vec::new();
    if let Some(s) = parse_theme_face(obj, i, "majorLatin", false)? {
        fonts.push((super::super::theme::FontSlot::MajorLatin, s));
    }
    if let Some(s) = parse_theme_face(obj, i, "minorLatin", false)? {
        fonts.push((super::super::theme::FontSlot::MinorLatin, s));
    }
    if let Some(s) = parse_theme_face(obj, i, "majorEa", true)? {
        fonts.push((super::super::theme::FontSlot::MajorEa, s));
    }
    if let Some(s) = parse_theme_face(obj, i, "minorEa", true)? {
        fonts.push((super::super::theme::FontSlot::MinorEa, s));
    }
    if let Some(s) = parse_theme_face(obj, i, "majorCs", true)? {
        fonts.push((super::super::theme::FontSlot::MajorCs, s));
    }
    if let Some(s) = parse_theme_face(obj, i, "minorCs", true)? {
        fonts.push((super::super::theme::FontSlot::MinorCs, s));
    }
    let name = crate::theme::ops::parse_opt_null_name(obj, i, "name")?;
    let major_scripts = crate::theme::ops::parse_opt_scripts(obj, i, "majorScripts")?;
    let minor_scripts = crate::theme::ops::parse_opt_scripts(obj, i, "minorScripts")?;
    if fonts.is_empty() && name.is_none() && major_scripts.is_none() && minor_scripts.is_none() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setThemeFonts".into(),
                field: "a font slot, name, majorScripts, or minorScripts".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(SetThemeFontsOp {
        host,
        fonts,
        name,
        major_scripts,
        minor_scripts,
    })
}

pub(super) fn parse_set_handout_text(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<SetHandoutTextOp, Error> {
    deny_keys(obj, i, &["op", "id", "header", "footer", "date", "sldNum"])?;
    let id = require_id(obj, i)?;
    if id != "handout" {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: crate::error::IdReason::SetHandoutTextNeedsHandout,
            },
            format!("ops[{i}].id"),
        ));
    }
    let header = parse_optional_slot(obj, i, "header")?;
    let footer = parse_optional_slot(obj, i, "footer")?;
    let date = parse_optional_slot(obj, i, "date")?;
    let sld_num = parse_optional_slot(obj, i, "sldNum")?;
    if header.is_none() && footer.is_none() && date.is_none() && sld_num.is_none() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setHandoutText".into(),
                field: "header, footer, date, or sldNum".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(SetHandoutTextOp {
        header,
        footer,
        date,
        sld_num,
    })
}
