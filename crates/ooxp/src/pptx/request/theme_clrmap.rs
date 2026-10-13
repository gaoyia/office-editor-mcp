//! pptx request：`setClrMap` / `setClrMapOvr` / 页级 `setThemeOverride`。

use serde_json::{Map, Value};

use crate::error::{Error, ErrorKind, IdReason, TypeName};
use crate::theme::COLOR_SLOTS;

use super::super::picture;
use super::super::theme::CLR_MAP_KEYS;
use super::common::*;
use super::master::parse_hex_slot;
use super::types::*;

pub(super) fn parse_set_clr_map(obj: &Map<String, Value>, i: usize) -> Result<SetClrMapOp, Error> {
    let mut allowed = vec!["op", "id"];
    allowed.extend(CLR_MAP_KEYS.iter().copied());
    deny_keys(obj, i, &allowed)?;
    let id = require_id(obj, i)?;
    let host = parse_clr_map_host(&id, i)?;
    let slots = parse_map_slots(obj, i, "")?;
    if slots.is_empty() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setClrMap".into(),
                field: "a clrMap slot".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(SetClrMapOp { host, slots })
}

pub(super) fn parse_set_clr_map_ovr(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<SetClrMapOvrOp, Error> {
    deny_keys(obj, i, &["op", "id", "clrMapOvr"])?;
    let id = require_id(obj, i)?;
    let host = parse_clr_map_ovr_host(&id, i)?;
    let value = match obj.get("clrMapOvr") {
        None => {
            return Err(Error::with_path(
                ErrorKind::OpRequiresField {
                    op: "setClrMapOvr".into(),
                    field: "clrMapOvr".into(),
                },
                format!("ops[{i}].clrMapOvr"),
            ));
        }
        Some(Value::String(s)) if s == "master" => ClrMapOvrValue::Master,
        Some(Value::Object(map)) => {
            deny_keys_at(map, &format!("ops[{i}].clrMapOvr"), CLR_MAP_KEYS)?;
            let slots = parse_map_slots(map, i, "clrMapOvr.")?;
            if slots.is_empty() {
                return Err(Error::with_path(
                    ErrorKind::OpRequiresField {
                        op: "setClrMapOvr".into(),
                        field: "a clrMap slot".into(),
                    },
                    format!("ops[{i}].clrMapOvr"),
                ));
            }
            ClrMapOvrValue::Slots(slots)
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "clrMapOvr".into(),
                    expected: TypeName::Object,
                },
                format!("ops[{i}].clrMapOvr"),
            ));
        }
    };
    Ok(SetClrMapOvrOp { host, value })
}

pub(super) fn parse_set_theme_override(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<SetThemeOverrideOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op", "id", "enabled", "dk1", "lt1", "dk2", "lt2", "accent1", "accent2", "accent3",
            "accent4", "accent5", "accent6", "hlink", "folHlink",
        ],
    )?;
    let id = require_id(obj, i)?;
    let (slide, on_notes) =
        picture::parse_insert_slide_or_notes(&id, i, IdReason::SetThemeOverrideNeedsSlide)?;
    let mut slots = Vec::new();
    for slot in COLOR_SLOTS {
        if let Some(v) = obj.get(*slot) {
            slots.push(((*slot).to_string(), parse_hex_slot(v, i, slot)?));
        }
    }
    let enabled = match obj.get("enabled") {
        None => true,
        Some(Value::Bool(false)) => false,
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::NeedThemeOverrideField,
                format!("ops[{i}].enabled"),
            ));
        }
    };
    if !enabled && !slots.is_empty() {
        return Err(Error::with_path(
            ErrorKind::MutuallyExclusive {
                a: "enabled".into(),
                b: "color slots".into(),
            },
            format!("ops[{i}].enabled"),
        ));
    }
    if enabled && slots.is_empty() {
        return Err(Error::with_path(
            ErrorKind::NeedThemeOverrideField,
            format!("ops[{i}]"),
        ));
    }
    Ok(SetThemeOverrideOp {
        slide,
        on_notes,
        enabled,
        slots,
    })
}

fn parse_clr_map_host(id: &str, i: usize) -> Result<ClrMapHost, Error> {
    if id == "notesMaster" {
        return Ok(ClrMapHost::NotesMaster);
    }
    if id == "handout" {
        return Ok(ClrMapHost::Handout);
    }
    let master = super::super::master::parse_master_only_id(id, i, IdReason::SetClrMapNeedsHost)?;
    Ok(ClrMapHost::Master(master))
}

fn parse_clr_map_ovr_host(id: &str, i: usize) -> Result<ClrMapOvrHost, Error> {
    let path = format!("ops[{i}].id");
    if id.starts_with("master:") {
        let rest = id.strip_prefix("master:").unwrap_or("");
        let Some((n_str, after)) = rest.split_once('/') else {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::SetClrMapOvrNeedsHost,
                },
                path,
            ));
        };
        let Some(k_str) = after.strip_prefix("layout:") else {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::SetClrMapOvrNeedsHost,
                },
                path,
            ));
        };
        if n_str.is_empty() || k_str.is_empty() || k_str.contains('/') {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::SetClrMapOvrNeedsHost,
                },
                path,
            ));
        }
        let master: usize = n_str.parse().map_err(|_| {
            Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::SetClrMapOvrNeedsHost,
                },
                path.clone(),
            )
        })?;
        let layout: usize = k_str.parse().map_err(|_| {
            Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::SetClrMapOvrNeedsHost,
                },
                path.clone(),
            )
        })?;
        if master < 1 || layout < 1 {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::SetClrMapOvrNeedsHost,
                },
                path,
            ));
        }
        return Ok(ClrMapOvrHost::Layout { master, layout });
    }
    let (slide, on_notes) =
        picture::parse_insert_slide_or_notes(id, i, IdReason::SetClrMapOvrNeedsHost)?;
    Ok(ClrMapOvrHost::Slide { slide, on_notes })
}

fn parse_map_slots(
    obj: &Map<String, Value>,
    i: usize,
    prefix: &str,
) -> Result<Vec<(String, String)>, Error> {
    let mut slots = Vec::new();
    for key in CLR_MAP_KEYS {
        if let Some(v) = obj.get(*key) {
            slots.push(((*key).to_string(), parse_scheme_index(v, i, prefix, key)?));
        }
    }
    Ok(slots)
}

fn parse_scheme_index(v: &Value, i: usize, prefix: &str, field: &str) -> Result<String, Error> {
    let path = format!("ops[{i}].{prefix}{field}");
    match v {
        Value::String(s) if COLOR_SLOTS.iter().any(|slot| *slot == s) => Ok(s.clone()),
        Value::String(_) => Err(Error::with_path(ErrorKind::InvalidColorSchemeIndex, path)),
        _ => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::String,
            },
            path,
        )),
    }
}
