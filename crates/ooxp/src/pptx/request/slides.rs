//! pptx request：slides。

use serde_json::{Map, Value};

use crate::error::{Error, ErrorKind, IdReason, TypeName};

use super::super::picture;
use super::super::section;
use super::super::transition;

use super::common::*;
use super::types::*;

pub(super) fn parse_set_notes_text(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<SetNotesTextOp, Error> {
    deny_keys(obj, i, &["op", "id", "text"])?;
    let id = require_id(obj, i)?;
    let (slide, _) = picture::parse_insert_slide_or_notes(
        &id,
        i,
        crate::error::IdReason::SetNotesTextNeedsSlide,
    )?;
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
                    op: "setNotesText".into(),
                    field: "text".into(),
                },
                format!("ops[{i}].text"),
            ));
        }
    };
    Ok(SetNotesTextOp { slide, id, text })
}

pub(super) fn parse_insert_slide(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<InsertSlideOp, Error> {
    deny_keys(obj, i, &["op", "id", "after"])?;
    let id = require_id(obj, i)?;
    if id != "pptx:presentation" {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::InsertSlideNeedsPresentation,
            },
            format!("ops[{i}].id"),
        ));
    }
    Ok(InsertSlideOp {
        after: parse_optional_slide_after(obj, i, false)?,
    })
}

pub(super) fn parse_delete_slide(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<DeleteSlideOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    Ok(DeleteSlideOp {
        slide: parse_slide_only_id(&id, i, "id")?,
    })
}

pub(super) fn parse_move_slide(obj: &Map<String, Value>, i: usize) -> Result<MoveSlideOp, Error> {
    deny_keys(obj, i, &["op", "id", "after"])?;
    let id = require_id(obj, i)?;
    Ok(MoveSlideOp {
        slide: parse_slide_only_id(&id, i, "id")?,
        after: parse_optional_slide_after(obj, i, true)?,
    })
}

pub(super) fn parse_set_slide_hidden(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<SetSlideHiddenOp, Error> {
    deny_keys(obj, i, &["op", "id", "hidden"])?;
    let id = require_id(obj, i)?;
    Ok(SetSlideHiddenOp {
        slide: parse_slide_only_id(&id, i, "id")?,
        hidden: parse_required_bool(obj.get("hidden"), i, "setSlideHidden", "hidden")?,
    })
}

pub(super) fn parse_set_transition(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<SetTransitionOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op", "id", "type", "dur", "advClick", "advTm", "dir", "invX", "invY", "sound",
            "orient", "spokes",
        ],
    )?;
    let id = require_id(obj, i)?;
    let host = picture::parse_insert_host(&id, i, IdReason::SetTransitionNeedsHost)?;
    if host.on_notes() {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SetTransitionNeedsHost,
            },
            format!("ops[{i}].id"),
        ));
    }
    let (slide, _, master) = host.parts();
    let type_name = match obj.get("type") {
        None => {
            return Err(Error::with_path(
                ErrorKind::OpRequiresField {
                    op: "setTransition".into(),
                    field: "type".into(),
                },
                format!("ops[{i}].type"),
            ));
        }
        Some(Value::Null) => None,
        Some(Value::String(s)) => {
            if s.is_empty() || s == "other" || !transition::is_writable_type(s) {
                return Err(Error::with_path(
                    ErrorKind::InvalidTransitionType,
                    format!("ops[{i}].type"),
                ));
            }
            Some(s.clone())
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
    let dur = match obj.get("dur") {
        None => None,
        Some(_) if type_name.is_none() => None,
        Some(v) => Some(parse_transition_dur(v, i)?),
    };
    let adv_click = match obj.get("advClick") {
        None => true,
        Some(Value::Bool(b)) => *b,
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "advClick".into(),
                    expected: TypeName::Boolean,
                },
                format!("ops[{i}].advClick"),
            ));
        }
    };
    let adv_tm = match obj.get("advTm") {
        None | Some(Value::Null) => None,
        Some(_) if type_name.is_none() => None,
        Some(v) => Some(parse_transition_adv_tm(v, i)?),
    };
    let dir = match obj.get("dir") {
        None => None,
        Some(_) if type_name.is_none() => None,
        Some(Value::String(s)) if !s.is_empty() => Some(s.clone()),
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidTransitionDir,
                format!("ops[{i}].dir"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "dir".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].dir"),
            ));
        }
    };
    let inv_x = parse_optional_flag(obj.get("invX"), i, "invX")?;
    let inv_y = parse_optional_flag(obj.get("invY"), i, "invY")?;
    let sound = match obj.get("sound") {
        None => None,
        Some(_) if type_name.is_none() => None,
        Some(_) => parse_anim_sound(obj.get("sound"), i)?,
    };
    let orient = match obj.get("orient") {
        None => None,
        Some(_) if type_name.is_none() => None,
        Some(Value::String(s)) if !s.is_empty() => Some(s.clone()),
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidTransitionOrient,
                format!("ops[{i}].orient"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "orient".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].orient"),
            ));
        }
    };
    let spokes = match obj.get("spokes") {
        None => None,
        Some(_) if type_name.is_none() => None,
        Some(v) => Some(parse_transition_spokes(v, &format!("ops[{i}].spokes"))?),
    };
    Ok(SetTransitionOp {
        slide,
        master,
        type_name,
        dur,
        adv_click,
        adv_tm,
        dir,
        inv_x,
        inv_y,
        sound,
        orient,
        spokes,
    })
}

pub(super) fn parse_set_slide_size(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<SetSlideSizeOp, Error> {
    deny_keys(obj, i, &["op", "id", "cx", "cy", "type"])?;
    let id = require_id(obj, i)?;
    if id != "pptx:presentation" {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SetSlideSizeNeedsPresentation,
            },
            format!("ops[{i}].id"),
        ));
    }
    let cx = parse_emu_field(obj.get("cx"), i, "cx")?.ok_or_else(|| {
        Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setSlideSize".into(),
                field: "cx".into(),
            },
            format!("ops[{i}].cx"),
        )
    })?;
    let cy = parse_emu_field(obj.get("cy"), i, "cy")?.ok_or_else(|| {
        Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setSlideSize".into(),
                field: "cy".into(),
            },
            format!("ops[{i}].cy"),
        )
    })?;
    let type_name = match obj.get("type") {
        None => None,
        Some(Value::String(s)) if super::super::theme::is_slide_size_type(s) => Some(s.clone()),
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidSlideSizeType,
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
    Ok(SetSlideSizeOp { cx, cy, type_name })
}

pub(super) fn parse_set_notes_size(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<SetNotesSizeOp, Error> {
    deny_keys(obj, i, &["op", "id", "cx", "cy"])?;
    let id = require_id(obj, i)?;
    if id != "pptx:presentation" {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SetNotesSizeNeedsPresentation,
            },
            format!("ops[{i}].id"),
        ));
    }
    let (cx, cy) = parse_notes_size_pair(obj.get("cx"), obj.get("cy"), i, "ops")?;
    Ok(SetNotesSizeOp { cx, cy })
}

pub(crate) fn parse_create_notes_size(v: &Value) -> Result<(i64, i64), Error> {
    let obj = v.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "notesSize".into(),
                expected: TypeName::Object,
            },
            "presentation.notesSize",
        )
    })?;
    for k in obj.keys() {
        if !matches!(k.as_str(), "cx" | "cy") {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("presentation.notesSize.{k}"),
            ));
        }
    }
    parse_notes_size_pair(obj.get("cx"), obj.get("cy"), 0, "presentation.notesSize")
}

fn parse_notes_size_pair(
    cx: Option<&Value>,
    cy: Option<&Value>,
    i: usize,
    loc: &str,
) -> Result<(i64, i64), Error> {
    let (cx_path, cy_path, op_index) = if loc == "ops" {
        (format!("ops[{i}].cx"), format!("ops[{i}].cy"), i)
    } else {
        (format!("{loc}.cx"), format!("{loc}.cy"), 0)
    };
    let cx = parse_emu_at(cx, &cx_path, "cx")?.ok_or_else(|| {
        Error::with_path(
            ErrorKind::OpRequiresField {
                op: if loc == "ops" {
                    "setNotesSize".into()
                } else {
                    "notesSize".into()
                },
                field: "cx".into(),
            },
            cx_path.clone(),
        )
    })?;
    let cy = parse_emu_at(cy, &cy_path, "cy")?.ok_or_else(|| {
        Error::with_path(
            ErrorKind::OpRequiresField {
                op: if loc == "ops" {
                    "setNotesSize".into()
                } else {
                    "notesSize".into()
                },
                field: "cy".into(),
            },
            cy_path.clone(),
        )
    })?;
    super::super::theme::check_size(cx, "cx", op_index).map_err(|e| {
        if loc == "ops" {
            e
        } else {
            Error::with_path(ErrorKind::SlideSizeOutOfRange, cx_path.clone())
        }
    })?;
    super::super::theme::check_size(cy, "cy", op_index).map_err(|e| {
        if loc == "ops" {
            e
        } else {
            Error::with_path(ErrorKind::SlideSizeOutOfRange, cy_path.clone())
        }
    })?;
    Ok((cx, cy))
}

pub(super) fn parse_transition_dur(v: &Value, i: usize) -> Result<u32, Error> {
    parse_transition_ms(v, i, "dur", ErrorKind::InvalidTransitionDur, 1, 60000)
}

pub(super) fn parse_transition_adv_tm(v: &Value, i: usize) -> Result<u32, Error> {
    parse_transition_ms(
        v,
        i,
        "advTm",
        ErrorKind::InvalidTransitionAdvTm,
        1,
        86400000,
    )
}

pub(super) fn parse_transition_spokes(v: &Value, path: &str) -> Result<u32, Error> {
    let n = match v {
        Value::Number(num) => num.as_u64().ok_or_else(|| {
            Error::with_path(ErrorKind::InvalidTransitionSpokes, path.to_string())
        })?,
        _ => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "spokes".into(),
                    expected: TypeName::Integer,
                },
                path.to_string(),
            ));
        }
    };
    if !(1..=8).contains(&n) {
        return Err(Error::with_path(
            ErrorKind::InvalidTransitionSpokes,
            path.to_string(),
        ));
    }
    Ok(n as u32)
}

pub(super) fn parse_transition_ms(
    v: &Value,
    i: usize,
    field: &str,
    invalid: ErrorKind,
    min: u64,
    max: u64,
) -> Result<u32, Error> {
    let path = format!("ops[{i}].{field}");
    let n = match v {
        Value::Number(num) => num.as_u64().ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: field.into(),
                    expected: TypeName::Integer,
                },
                path.clone(),
            )
        })?,
        _ => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: field.into(),
                    expected: TypeName::Integer,
                },
                path,
            ));
        }
    };
    if !(min..=max).contains(&n) {
        return Err(Error::with_path(invalid, path));
    }
    Ok(n as u32)
}

pub(super) fn parse_optional_flag(v: Option<&Value>, i: usize, field: &str) -> Result<bool, Error> {
    match v {
        None => Ok(false),
        Some(Value::Bool(b)) => Ok(*b),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Boolean,
            },
            format!("ops[{i}].{field}"),
        )),
    }
}

pub(super) fn parse_optional_slide_after(
    obj: &Map<String, Value>,
    i: usize,
    allow_null: bool,
) -> Result<Option<usize>, Error> {
    match obj.get("after") {
        None => Ok(None),
        Some(Value::Null) if allow_null => Ok(None),
        Some(Value::String(s)) => Ok(Some(parse_slide_only_id(s, i, "after")?)),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "after".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].after"),
        )),
    }
}

pub(super) fn parse_slide_only_id(id: &str, op_index: usize, field: &str) -> Result<usize, Error> {
    let path = format!("ops[{op_index}].{field}");
    let Some(rest) = id.strip_prefix("slide:") else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SlideOpNeedsSlide,
            },
            path,
        ));
    };
    if rest.is_empty() || rest.contains('/') {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SlideOpNeedsSlide,
            },
            path,
        ));
    }
    let slide: usize = rest.parse().map_err(|_| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SlideIndexNotInt,
            },
            path.clone(),
        )
    })?;
    if slide < 1 {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SlideIndexFromOne,
            },
            path,
        ));
    }
    Ok(slide)
}

pub(super) fn parse_optional_str(
    obj: &Map<String, Value>,
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

pub(super) fn parse_insert_slide_section(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<InsertSlideSectionOp, Error> {
    deny_keys(obj, i, &["op", "id", "name", "start", "after"])?;
    let id = require_id(obj, i)?;
    if id != "pptx:presentation" {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::InsertSlideSectionNeedsPresentation,
            },
            format!("ops[{i}].id"),
        ));
    }
    if obj.contains_key("start") && obj.contains_key("after") {
        return Err(Error::with_path(
            ErrorKind::MutuallyExclusive {
                a: "start".into(),
                b: "after".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(InsertSlideSectionOp {
        name: require_name(obj, i)?,
        start: parse_optional_slide_id_field(obj, i, "start")?,
        after: parse_optional_section_id_field(obj, i, "after")?,
    })
}

pub(super) fn parse_delete_slide_section(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<DeleteSlideSectionOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    Ok(DeleteSlideSectionOp {
        section: section::parse_section_id(&id, &format!("ops[{i}].id"))?,
    })
}

pub(super) fn parse_optional_slide_id_field(
    obj: &Map<String, Value>,
    i: usize,
    field: &str,
) -> Result<Option<usize>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::String(s)) => Ok(Some(parse_slide_only_id(s, i, field)?)),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].{field}"),
        )),
    }
}

pub(super) fn parse_optional_section_id_field(
    obj: &Map<String, Value>,
    i: usize,
    field: &str,
) -> Result<Option<usize>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::String(s)) => Ok(Some(section::parse_section_id(
            s,
            &format!("ops[{i}].{field}"),
        )?)),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].{field}"),
        )),
    }
}

pub(crate) fn parse_create_transition(
    v: &Value,
    prefix: &str,
    slide: usize,
) -> Result<SetTransitionOp, Error> {
    let obj = v.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "transition".into(),
                expected: TypeName::Object,
            },
            prefix.to_string(),
        )
    })?;
    deny_keys_at(
        obj,
        prefix,
        &[
            "type", "dur", "advClick", "advTm", "dir", "invX", "invY", "sound", "orient", "spokes",
        ],
    )?;
    let type_name = match obj.get("type") {
        None => {
            return Err(Error::with_path(
                ErrorKind::OpRequiresField {
                    op: "create".into(),
                    field: "type".into(),
                },
                format!("{prefix}.type"),
            ));
        }
        Some(Value::Null) => {
            return Err(Error::with_path(
                ErrorKind::InvalidTransitionType,
                format!("{prefix}.type"),
            ));
        }
        Some(Value::String(s)) => {
            if s.is_empty() || s == "other" || !transition::is_writable_type(s) {
                return Err(Error::with_path(
                    ErrorKind::InvalidTransitionType,
                    format!("{prefix}.type"),
                ));
            }
            s.clone()
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "type".into(),
                    expected: TypeName::String,
                },
                format!("{prefix}.type"),
            ));
        }
    };
    let dur = match obj.get("dur") {
        None => None,
        Some(v) => Some(parse_transition_ms_at(
            v,
            &format!("{prefix}.dur"),
            "dur",
            ErrorKind::InvalidTransitionDur,
            1,
            60000,
        )?),
    };
    let adv_click = match obj.get("advClick") {
        None => true,
        Some(Value::Bool(b)) => *b,
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "advClick".into(),
                    expected: TypeName::Boolean,
                },
                format!("{prefix}.advClick"),
            ));
        }
    };
    let adv_tm = match obj.get("advTm") {
        None | Some(Value::Null) => None,
        Some(v) => Some(parse_transition_ms_at(
            v,
            &format!("{prefix}.advTm"),
            "advTm",
            ErrorKind::InvalidTransitionAdvTm,
            1,
            86400000,
        )?),
    };
    let dir = match obj.get("dir") {
        None => None,
        Some(Value::String(s)) if !s.is_empty() => Some(s.clone()),
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidTransitionDir,
                format!("{prefix}.dir"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "dir".into(),
                    expected: TypeName::String,
                },
                format!("{prefix}.dir"),
            ));
        }
    };
    let inv_x = parse_optional_flag_at(obj.get("invX"), &format!("{prefix}.invX"), "invX")?;
    let inv_y = parse_optional_flag_at(obj.get("invY"), &format!("{prefix}.invY"), "invY")?;
    let sound = match obj.get("sound") {
        None => None,
        Some(Value::String(s)) if !s.is_empty() => Some(s.clone()),
        Some(Value::String(_)) | Some(Value::Null) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "sound".into(),
                    expected: TypeName::String,
                },
                format!("{prefix}.sound"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "sound".into(),
                    expected: TypeName::String,
                },
                format!("{prefix}.sound"),
            ));
        }
    };
    let orient = match obj.get("orient") {
        None => None,
        Some(Value::String(s)) if !s.is_empty() => Some(s.clone()),
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidTransitionOrient,
                format!("{prefix}.orient"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "orient".into(),
                    expected: TypeName::String,
                },
                format!("{prefix}.orient"),
            ));
        }
    };
    let spokes = match obj.get("spokes") {
        None => None,
        Some(v) => Some(parse_transition_spokes(v, &format!("{prefix}.spokes"))?),
    };
    Ok(SetTransitionOp {
        slide,
        master: None,
        type_name: Some(type_name),
        dur,
        adv_click,
        adv_tm,
        dir,
        inv_x,
        inv_y,
        sound,
        orient,
        spokes,
    })
}

pub(super) fn parse_transition_ms_at(
    v: &Value,
    path: &str,
    field: &str,
    invalid: ErrorKind,
    min: u64,
    max: u64,
) -> Result<u32, Error> {
    let n = match v {
        Value::Number(num) => num.as_u64().ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: field.into(),
                    expected: TypeName::Integer,
                },
                path.to_string(),
            )
        })?,
        _ => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: field.into(),
                    expected: TypeName::Integer,
                },
                path.to_string(),
            ));
        }
    };
    if !(min..=max).contains(&n) {
        return Err(Error::with_path(invalid, path.to_string()));
    }
    Ok(n as u32)
}

pub(super) fn parse_optional_flag_at(
    v: Option<&Value>,
    path: &str,
    field: &str,
) -> Result<bool, Error> {
    match v {
        None => Ok(false),
        Some(Value::Bool(b)) => Ok(*b),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Boolean,
            },
            path.to_string(),
        )),
    }
}
