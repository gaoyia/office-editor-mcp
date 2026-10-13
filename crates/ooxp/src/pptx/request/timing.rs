//! pptx request：timing。

use serde_json::{Map, Value};

use crate::error::{Error, ErrorKind, IdReason, TypeName};

use super::super::timing;

use super::common::*;
use super::slides::*;
use super::types::*;

pub(super) fn parse_insert_anim(obj: &Map<String, Value>, i: usize) -> Result<InsertAnimOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op", "id", "type", "trigger", "delay", "dur", "repeat", "sound", "para", "clickOn",
        ],
    )?;
    let id = require_id(obj, i)?;
    let target = parse_anim_host(&id, i)?;
    let type_name = parse_anim_type(obj.get("type"), i, true)?.ok_or_else(|| {
        Error::with_path(
            ErrorKind::OpRequiresField {
                op: "insertAnim".into(),
                field: "type".into(),
            },
            format!("ops[{i}].type"),
        )
    })?;
    let trigger = parse_anim_trigger(obj.get("trigger"), i, true)?.ok_or_else(|| {
        Error::with_path(ErrorKind::InvalidAnimTrigger, format!("ops[{i}].trigger"))
    })?;
    let delay = match obj.get("delay") {
        None => 0,
        Some(v) => parse_transition_ms(v, i, "delay", ErrorKind::InvalidAnimDelay, 0, 60000)?,
    };
    let dur = match obj.get("dur") {
        None => 500,
        Some(v) => parse_transition_ms(v, i, "dur", ErrorKind::InvalidAnimDur, 1, 60000)?,
    };
    let repeat = parse_anim_repeat(obj.get("repeat"), i)?;
    let sound = parse_anim_sound(obj.get("sound"), i)?;
    let para = parse_anim_para(obj.get("para"), i)?;
    let click_on = match (trigger.as_str(), obj.get("clickOn")) {
        ("clickOn", None) => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "clickOn".into(),
                },
                format!("ops[{i}].clickOn"),
            ));
        }
        ("clickOn", Some(Value::String(s))) if !s.is_empty() => Some(s.clone()),
        ("clickOn", Some(_)) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "clickOn".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].clickOn"),
            ));
        }
        (_, Some(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidAnimTrigger,
                format!("ops[{i}].clickOn"),
            ));
        }
        _ => None,
    };
    Ok(InsertAnimOp {
        id,
        target,
        type_name,
        trigger,
        delay,
        dur,
        repeat,
        sound,
        para,
        click_on,
    })
}

pub(super) fn parse_set_anim(obj: &Map<String, Value>, i: usize) -> Result<SetAnimOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op", "id", "type", "trigger", "delay", "dur", "repeat", "sound", "para", "clickOn",
        ],
    )?;
    let id = require_id(obj, i)?;
    let (target, index) = parse_anim_id(&id, i)?;
    let type_name = parse_anim_type(obj.get("type"), i, false)?;
    let trigger = parse_anim_trigger(obj.get("trigger"), i, false)?;
    let delay = match obj.get("delay") {
        None => None,
        Some(v) => Some(parse_transition_ms(
            v,
            i,
            "delay",
            ErrorKind::InvalidAnimDelay,
            0,
            60000,
        )?),
    };
    let dur = match obj.get("dur") {
        None => None,
        Some(v) => Some(parse_transition_ms(
            v,
            i,
            "dur",
            ErrorKind::InvalidAnimDur,
            1,
            60000,
        )?),
    };
    let repeat = if obj.contains_key("repeat") {
        Some(parse_anim_repeat(obj.get("repeat"), i)?)
    } else {
        None
    };
    let sound = match obj.get("sound") {
        None => None,
        Some(Value::Null) => Some(None),
        Some(v) => Some(Some(parse_anim_sound(Some(v), i)?.ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: "sound".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].sound"),
            )
        })?)),
    };
    let para = match obj.get("para") {
        None => None,
        Some(Value::Null) => Some(None),
        Some(v) => Some(Some(parse_anim_para(Some(v), i)?.ok_or_else(|| {
            Error::with_path(ErrorKind::InvalidAnimPara, format!("ops[{i}].para"))
        })?)),
    };
    let click_on = match obj.get("clickOn") {
        None => None,
        Some(Value::String(s)) if !s.is_empty() => Some(s.clone()),
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidAnimTrigger,
                format!("ops[{i}].clickOn"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "clickOn".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].clickOn"),
            ));
        }
    };
    if type_name.is_none()
        && trigger.is_none()
        && delay.is_none()
        && dur.is_none()
        && repeat.is_none()
        && sound.is_none()
        && para.is_none()
        && click_on.is_none()
    {
        return Err(Error::with_path(
            ErrorKind::NeedAnimSetField,
            format!("ops[{i}]"),
        ));
    }
    if trigger.as_deref() == Some("clickOn") && click_on.is_none() {
        return Err(Error::with_path(
            ErrorKind::MissingField {
                field: "clickOn".into(),
            },
            format!("ops[{i}].clickOn"),
        ));
    }
    Ok(SetAnimOp {
        id,
        target,
        index,
        type_name,
        trigger,
        delay,
        dur,
        repeat,
        sound,
        para,
        click_on,
    })
}

pub(super) fn parse_move_anim(obj: &Map<String, Value>, i: usize) -> Result<MoveAnimOp, Error> {
    deny_keys(obj, i, &["op", "id", "index"])?;
    let id = require_id(obj, i)?;
    let (target, index) = parse_anim_id(&id, i)?;
    let dest = match obj.get("index") {
        None => {
            return Err(Error::with_path(
                ErrorKind::OpRequiresField {
                    op: "moveAnim".into(),
                    field: "index".into(),
                },
                format!("ops[{i}].index"),
            ));
        }
        Some(Value::Number(n)) => {
            let Some(v) = n.as_u64() else {
                return Err(Error::with_path(
                    ErrorKind::InvalidAnimMoveIndex,
                    format!("ops[{i}].index"),
                ));
            };
            if v < 1 {
                return Err(Error::with_path(
                    ErrorKind::InvalidAnimMoveIndex,
                    format!("ops[{i}].index"),
                ));
            }
            v as usize
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "index".into(),
                    expected: TypeName::Integer,
                },
                format!("ops[{i}].index"),
            ));
        }
    };
    Ok(MoveAnimOp {
        id,
        target,
        index,
        dest,
    })
}

pub(super) fn parse_anim_type(
    v: Option<&Value>,
    i: usize,
    required: bool,
) -> Result<Option<String>, Error> {
    match v {
        None if required => Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "insertAnim".into(),
                field: "type".into(),
            },
            format!("ops[{i}].type"),
        )),
        None => Ok(None),
        Some(Value::String(s)) if timing::anim_preset_ok(s) => Ok(Some(s.clone())),
        Some(Value::String(_)) => Err(Error::with_path(
            ErrorKind::InvalidAnimType,
            format!("ops[{i}].type"),
        )),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "type".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].type"),
        )),
    }
}

pub(super) fn parse_anim_repeat(v: Option<&Value>, i: usize) -> Result<timing::RepeatWrite, Error> {
    match v {
        None => Ok(timing::RepeatWrite::Once),
        Some(Value::Null) => Ok(timing::RepeatWrite::Indefinite),
        Some(Value::Number(n)) => {
            let Some(v) = n.as_u64() else {
                return Err(Error::with_path(
                    ErrorKind::InvalidAnimRepeat,
                    format!("ops[{i}].repeat"),
                ));
            };
            if !(1..=99).contains(&v) {
                return Err(Error::with_path(
                    ErrorKind::InvalidAnimRepeat,
                    format!("ops[{i}].repeat"),
                ));
            }
            if v == 1 {
                Ok(timing::RepeatWrite::Once)
            } else {
                Ok(timing::RepeatWrite::Times(v as u32))
            }
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::InvalidAnimRepeat,
            format!("ops[{i}].repeat"),
        )),
    }
}

pub(super) fn parse_anim_para(v: Option<&Value>, i: usize) -> Result<Option<u32>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Number(n)) => {
            let Some(v) = n.as_u64() else {
                return Err(Error::with_path(
                    ErrorKind::InvalidAnimPara,
                    format!("ops[{i}].para"),
                ));
            };
            if v < 1 {
                return Err(Error::with_path(
                    ErrorKind::InvalidAnimPara,
                    format!("ops[{i}].para"),
                ));
            }
            Ok(Some(v as u32))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::InvalidAnimPara,
            format!("ops[{i}].para"),
        )),
    }
}

pub(super) fn parse_anim_trigger(
    v: Option<&Value>,
    i: usize,
    required_default_click: bool,
) -> Result<Option<String>, Error> {
    match v {
        None if required_default_click => Ok(Some("click".to_string())),
        None => Ok(None),
        Some(Value::String(s)) if matches!(s.as_str(), "click" | "with" | "after" | "clickOn") => {
            Ok(Some(s.clone()))
        }
        Some(Value::String(_)) => Err(Error::with_path(
            ErrorKind::InvalidAnimTrigger,
            format!("ops[{i}].trigger"),
        )),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "trigger".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].trigger"),
        )),
    }
}

pub(super) fn parse_delete_anim(obj: &Map<String, Value>, i: usize) -> Result<DeleteAnimOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let (target, index) = parse_anim_id(&id, i)?;
    Ok(DeleteAnimOp { id, target, index })
}

pub(super) fn parse_positive_index(
    raw: &str,
    path: &str,
    not_int: IdReason,
    from_one: IdReason,
) -> Result<usize, Error> {
    let n: usize = raw.parse().map_err(|_| {
        Error::with_path(ErrorKind::InvalidId { reason: not_int }, path.to_string())
    })?;
    if n < 1 {
        return Err(Error::with_path(
            ErrorKind::InvalidId { reason: from_one },
            path.to_string(),
        ));
    }
    Ok(n)
}

pub(super) fn parse_kind_key(after: &str, path: &str) -> Result<(), Error> {
    let Some((kind, key)) = after.split_once(':') else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::AnimForm,
            },
            path.to_string(),
        ));
    };
    if !ANIM_HOST_KINDS.contains(&kind) || key.is_empty() || key.contains('/') {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::AnimForm,
            },
            path.to_string(),
        ));
    }
    Ok(())
}

pub(super) fn parse_anim_host(id: &str, op_index: usize) -> Result<AnimTarget, Error> {
    let path = format!("ops[{op_index}].id");
    if let Some(rest) = id.strip_prefix("notesMaster/") {
        parse_kind_key(rest, &path)?;
        return Ok(AnimTarget::NotesMaster);
    }
    if let Some(rest) = id.strip_prefix("handout/") {
        parse_kind_key(rest, &path)?;
        return Ok(AnimTarget::Handout);
    }
    if let Some(rest) = id.strip_prefix("master:") {
        let Some((n, after)) = rest.split_once('/') else {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::InsertAnimNeedsHost,
                },
                path,
            ));
        };
        let master = parse_positive_index(
            n,
            &path,
            IdReason::SlideIndexNotInt,
            IdReason::SlideIndexFromOne,
        )?;
        if let Some(layout_rest) = after.strip_prefix("layout:") {
            let Some((k, shape)) = layout_rest.split_once('/') else {
                return Err(Error::with_path(
                    ErrorKind::InvalidId {
                        reason: IdReason::InsertAnimNeedsHost,
                    },
                    path,
                ));
            };
            let layout = parse_positive_index(
                k,
                &path,
                IdReason::SlideIndexNotInt,
                IdReason::SlideIndexFromOne,
            )?;
            let Some(key) = shape.strip_prefix("shape:") else {
                return Err(Error::with_path(
                    ErrorKind::InvalidId {
                        reason: IdReason::InsertAnimNeedsHost,
                    },
                    path,
                ));
            };
            if key.is_empty() || key.contains('/') {
                return Err(Error::with_path(
                    ErrorKind::InvalidId {
                        reason: IdReason::InsertAnimNeedsHost,
                    },
                    path,
                ));
            }
            return Ok(AnimTarget::Master {
                master,
                layout: Some(layout),
            });
        }
        if let Some(key) = after.strip_prefix("shape:") {
            if key.is_empty() || key.contains('/') {
                return Err(Error::with_path(
                    ErrorKind::InvalidId {
                        reason: IdReason::InsertAnimNeedsHost,
                    },
                    path,
                ));
            }
            return Ok(AnimTarget::Master {
                master,
                layout: None,
            });
        }
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::InsertAnimNeedsHost,
            },
            path,
        ));
    }
    let Some(rest) = id.strip_prefix("slide:") else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::AnimForm,
            },
            path,
        ));
    };
    let Some((n, after)) = rest.split_once('/') else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::InsertAnimNeedsHost,
            },
            path,
        ));
    };
    let slide = parse_positive_index(
        n,
        &path,
        IdReason::SlideIndexNotInt,
        IdReason::SlideIndexFromOne,
    )?;
    if let Some(notes_after) = after.strip_prefix("notes/") {
        parse_kind_key(notes_after, &path)?;
        return Ok(AnimTarget::Notes { slide });
    }
    parse_kind_key(after, &path)?;
    Ok(AnimTarget::Slide { slide })
}

pub(super) fn parse_anim_id(id: &str, op_index: usize) -> Result<(AnimTarget, usize), Error> {
    let path = format!("ops[{op_index}].id");
    if let Some(rest) = id.strip_prefix("notesMaster/anim:") {
        if rest.is_empty() || rest.contains('/') {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::AnimForm,
                },
                path,
            ));
        }
        let index = parse_positive_index(
            rest,
            &path,
            IdReason::AnimIndexNotInt,
            IdReason::AnimIndexFromOne,
        )?;
        return Ok((AnimTarget::NotesMaster, index));
    }
    if let Some(rest) = id.strip_prefix("handout/anim:") {
        if rest.is_empty() || rest.contains('/') {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::AnimForm,
                },
                path,
            ));
        }
        let index = parse_positive_index(
            rest,
            &path,
            IdReason::AnimIndexNotInt,
            IdReason::AnimIndexFromOne,
        )?;
        return Ok((AnimTarget::Handout, index));
    }
    if let Some(rest) = id.strip_prefix("master:") {
        if let Some((n, after)) = rest.split_once("/layout:") {
            let Some((k, anim)) = after.split_once("/anim:") else {
                return Err(Error::with_path(
                    ErrorKind::InvalidId {
                        reason: IdReason::AnimForm,
                    },
                    path,
                ));
            };
            if anim.is_empty() || anim.contains('/') {
                return Err(Error::with_path(
                    ErrorKind::InvalidId {
                        reason: IdReason::AnimForm,
                    },
                    path,
                ));
            }
            let master = parse_positive_index(
                n,
                &path,
                IdReason::SlideIndexNotInt,
                IdReason::SlideIndexFromOne,
            )?;
            let layout = parse_positive_index(
                k,
                &path,
                IdReason::SlideIndexNotInt,
                IdReason::SlideIndexFromOne,
            )?;
            let index = parse_positive_index(
                anim,
                &path,
                IdReason::AnimIndexNotInt,
                IdReason::AnimIndexFromOne,
            )?;
            return Ok((
                AnimTarget::Master {
                    master,
                    layout: Some(layout),
                },
                index,
            ));
        }
        let Some((n, after)) = rest.split_once("/anim:") else {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::AnimForm,
                },
                path,
            ));
        };
        if after.is_empty() || after.contains('/') {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::AnimForm,
                },
                path,
            ));
        }
        let master = parse_positive_index(
            n,
            &path,
            IdReason::SlideIndexNotInt,
            IdReason::SlideIndexFromOne,
        )?;
        let index = parse_positive_index(
            after,
            &path,
            IdReason::AnimIndexNotInt,
            IdReason::AnimIndexFromOne,
        )?;
        return Ok((
            AnimTarget::Master {
                master,
                layout: None,
            },
            index,
        ));
    }
    let Some(rest) = id.strip_prefix("slide:") else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::AnimForm,
            },
            path,
        ));
    };
    if let Some((n, after)) = rest.split_once("/notes/anim:") {
        if after.is_empty() || after.contains('/') {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::AnimForm,
                },
                path,
            ));
        }
        let slide = parse_positive_index(
            n,
            &path,
            IdReason::SlideIndexNotInt,
            IdReason::SlideIndexFromOne,
        )?;
        let index = parse_positive_index(
            after,
            &path,
            IdReason::AnimIndexNotInt,
            IdReason::AnimIndexFromOne,
        )?;
        return Ok((AnimTarget::Notes { slide }, index));
    }
    let Some((n, after)) = rest.split_once("/anim:") else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::AnimForm,
            },
            path,
        ));
    };
    if after.is_empty() || after.contains('/') {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::AnimForm,
            },
            path,
        ));
    }
    let slide = parse_positive_index(
        n,
        &path,
        IdReason::SlideIndexNotInt,
        IdReason::SlideIndexFromOne,
    )?;
    let index = parse_positive_index(
        after,
        &path,
        IdReason::AnimIndexNotInt,
        IdReason::AnimIndexFromOne,
    )?;
    Ok((AnimTarget::Slide { slide }, index))
}

pub(crate) fn parse_create_anim(v: &Value, prefix: &str) -> Result<CreateAnim, Error> {
    let obj = v.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "anims".into(),
                expected: TypeName::Object,
            },
            prefix.to_string(),
        )
    })?;
    deny_keys_at(
        obj,
        prefix,
        &[
            "type", "trigger", "delay", "dur", "repeat", "sound", "para", "clickOn", "shape",
            "picture", "video", "audio", "table", "chart", "smartArt",
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
        Some(Value::String(s)) if timing::anim_preset_ok(s) => s.clone(),
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidAnimType,
                format!("{prefix}.type"),
            ));
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
    let trigger = match obj.get("trigger") {
        None => "click".to_string(),
        Some(Value::String(s)) if matches!(s.as_str(), "click" | "with" | "after" | "clickOn") => {
            s.clone()
        }
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidAnimTrigger,
                format!("{prefix}.trigger"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "trigger".into(),
                    expected: TypeName::String,
                },
                format!("{prefix}.trigger"),
            ));
        }
    };
    let delay = match obj.get("delay") {
        None => 0,
        Some(v) => parse_transition_ms_at(
            v,
            &format!("{prefix}.delay"),
            "delay",
            ErrorKind::InvalidAnimDelay,
            0,
            60000,
        )?,
    };
    let dur = match obj.get("dur") {
        None => 500,
        Some(v) => parse_transition_ms_at(
            v,
            &format!("{prefix}.dur"),
            "dur",
            ErrorKind::InvalidAnimDur,
            1,
            60000,
        )?,
    };
    let repeat = parse_create_anim_repeat(obj.get("repeat"), &format!("{prefix}.repeat"))?;
    let sound = match obj.get("sound") {
        None => None,
        Some(Value::String(s)) if !s.is_empty() => Some(s.clone()),
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
    let para = match obj.get("para") {
        None => None,
        Some(Value::Number(n)) => {
            let Some(v) = n.as_u64() else {
                return Err(Error::with_path(
                    ErrorKind::InvalidAnimPara,
                    format!("{prefix}.para"),
                ));
            };
            if v < 1 {
                return Err(Error::with_path(
                    ErrorKind::InvalidAnimPara,
                    format!("{prefix}.para"),
                ));
            }
            Some(v as u32)
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::InvalidAnimPara,
                format!("{prefix}.para"),
            ));
        }
    };
    let host = parse_create_anim_host(obj, prefix)?;
    let click_on = match (trigger.as_str(), obj.get("clickOn")) {
        ("clickOn", None) => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "clickOn".into(),
                },
                format!("{prefix}.clickOn"),
            ));
        }
        ("clickOn", Some(v)) => Some(parse_create_click_on(v, &format!("{prefix}.clickOn"))?),
        (_, None) => None,
        (_, Some(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidAnimTrigger,
                format!("{prefix}.clickOn"),
            ));
        }
    };
    Ok(CreateAnim {
        type_name,
        trigger,
        delay,
        dur,
        repeat,
        sound,
        para,
        host,
        click_on,
    })
}

pub(super) fn parse_create_anim_repeat(
    v: Option<&Value>,
    path: &str,
) -> Result<timing::RepeatWrite, Error> {
    match v {
        None => Ok(timing::RepeatWrite::Once),
        Some(Value::Null) => Ok(timing::RepeatWrite::Indefinite),
        Some(Value::Number(n)) => {
            let Some(v) = n.as_u64() else {
                return Err(Error::with_path(
                    ErrorKind::InvalidAnimRepeat,
                    path.to_string(),
                ));
            };
            if !(1..=99).contains(&v) {
                return Err(Error::with_path(
                    ErrorKind::InvalidAnimRepeat,
                    path.to_string(),
                ));
            }
            if v == 1 {
                Ok(timing::RepeatWrite::Once)
            } else {
                Ok(timing::RepeatWrite::Times(v as u32))
            }
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::InvalidAnimRepeat,
            path.to_string(),
        )),
    }
}
