//! docx request：style_fmt。

use serde_json::Value;

use crate::error::{Error, ErrorKind, IdReason, TypeName};

use super::super::para::{self, ParaTarget};

use super::common::*;
use super::math_ruby::*;
use super::page::*;
use super::para::*;
use super::types::*;

pub(super) fn parse_insert_numbering(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertNumberingOp, Error> {
    deny_keys(obj, i, &["op", "id", "kind"])?;
    let id = require_op_id(obj, i)?;
    if id != "docx:document" {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::InsertNumberingNeedsDocument,
            },
            format!("ops[{i}].id"),
        ));
    }
    let kind = match obj.get("kind").and_then(Value::as_str) {
        Some(s) => super::super::numbering::parse_kind(s).ok_or_else(|| {
            Error::with_path(ErrorKind::InvalidNumberingKind, format!("ops[{i}].kind"))
        })?,
        None => {
            return Err(Error::with_path(
                ErrorKind::OpRequiresField {
                    op: "insertNumbering".into(),
                    field: "kind".into(),
                },
                format!("ops[{i}].kind"),
            ));
        }
    };
    Ok(InsertNumberingOp { kind })
}

pub(super) fn parse_set_numbering(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetNumberingOp, Error> {
    deny_keys(obj, i, &["op", "id", "ilvl", "numFmt", "start", "lvlText"])?;
    let id = require_op_id(obj, i)?;
    let abstract_id = super::super::numbering::parse_abstract_ref(&id, &format!("ops[{i}].id"))?;
    let ilvl = match obj.get("ilvl") {
        Some(Value::Number(n)) => n.as_u64().ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: "ilvl".into(),
                    expected: TypeName::Integer,
                },
                format!("ops[{i}].ilvl"),
            )
        })? as u32,
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "ilvl".into(),
                    expected: TypeName::Integer,
                },
                format!("ops[{i}].ilvl"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::OpRequiresField {
                    op: "setNumbering".into(),
                    field: "ilvl".into(),
                },
                format!("ops[{i}].ilvl"),
            ));
        }
    };
    if ilvl > 8 {
        return Err(Error::with_path(
            ErrorKind::InvalidIlvl,
            format!("ops[{i}].ilvl"),
        ));
    }
    let num_fmt = match obj.get("numFmt") {
        None => None,
        Some(Value::String(s)) => {
            if !super::super::numbering::NUM_FMT_SET.contains(&s.as_str()) {
                return Err(Error::with_path(
                    ErrorKind::InvalidListNumFmt,
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
    let start = match obj.get("start") {
        None => None,
        Some(Value::Number(n)) => {
            let u = n.as_u64().ok_or_else(|| {
                Error::with_path(
                    ErrorKind::WrongType {
                        field: "start".into(),
                        expected: TypeName::Integer,
                    },
                    format!("ops[{i}].start"),
                )
            })?;
            if u < 1 {
                return Err(Error::with_path(
                    ErrorKind::MinValue {
                        field: "start".into(),
                        min: 1,
                    },
                    format!("ops[{i}].start"),
                ));
            }
            Some(u as u32)
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "start".into(),
                    expected: TypeName::Integer,
                },
                format!("ops[{i}].start"),
            ));
        }
    };
    let lvl_text = optional_op_string(obj, i, "lvlText")?;
    if num_fmt.is_none() && start.is_none() && lvl_text.is_none() {
        return Err(Error::with_path(
            ErrorKind::NeedSetNumberingField,
            format!("ops[{i}]"),
        ));
    }
    Ok(SetNumberingOp {
        id,
        abstract_id,
        ilvl,
        num_fmt,
        start,
        lvl_text,
    })
}

pub(super) fn parse_set_para_numbering(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetParaNumberingOp, Error> {
    deny_keys(obj, i, &["op", "id", "num", "ilvl"])?;
    let id = require_op_id(obj, i)?;
    let para = parse_story_para_at(
        &id,
        &format!("ops[{i}].id"),
        IdReason::SetParaNumberingNeedsPara,
    )?;
    let num = match obj.get("num") {
        Some(Value::Null) => None,
        Some(Value::String(s)) => Some(super::super::numbering::parse_num_ref(
            s,
            &format!("ops[{i}].num"),
        )?),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "num".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].num"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::NeedSetParaNumberingNum,
                format!("ops[{i}].num"),
            ));
        }
    };
    let ilvl = match obj.get("ilvl") {
        None => 0,
        Some(Value::Number(n)) => n.as_u64().ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: "ilvl".into(),
                    expected: TypeName::Integer,
                },
                format!("ops[{i}].ilvl"),
            )
        })? as u32,
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "ilvl".into(),
                    expected: TypeName::Integer,
                },
                format!("ops[{i}].ilvl"),
            ));
        }
    };
    if ilvl > 8 {
        return Err(Error::with_path(
            ErrorKind::InvalidIlvl,
            format!("ops[{i}].ilvl"),
        ));
    }
    Ok(SetParaNumberingOp {
        id,
        para,
        num,
        ilvl,
    })
}

pub(super) fn parse_set_style(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetStyleOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op",
            "id",
            "font",
            "eastAsiaFont",
            "sz",
            "bold",
            "color",
            "before",
            "after",
        ],
    )?;
    let id = require_op_id(obj, i)?;
    let style_id = super::super::styles::parse_style_ref(&id, &format!("ops[{i}].id"))?;
    let font = parse_nonempty_opt_string(obj, i, "font")?;
    let east_asia_font = parse_nonempty_opt_string(obj, i, "eastAsiaFont")?;
    let sz = parse_opt_u32_range(obj, i, "sz", 2, 3276)?;
    let bold = match obj.get("bold") {
        None => None,
        Some(Value::Bool(b)) => Some(*b),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "bold".into(),
                    expected: TypeName::Boolean,
                },
                format!("ops[{i}].bold"),
            ));
        }
    };
    let color = match obj.get("color") {
        None => None,
        Some(Value::String(s)) => {
            if !is_rgb_hex(s) {
                return Err(Error::with_path(
                    ErrorKind::InvalidStyleColor,
                    format!("ops[{i}].color"),
                ));
            }
            Some(s.clone())
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "color".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].color"),
            ));
        }
    };
    let before = parse_opt_u32_min(obj, i, "before", 0)?;
    let after = parse_opt_u32_min(obj, i, "after", 0)?;
    if font.is_none()
        && east_asia_font.is_none()
        && sz.is_none()
        && bold.is_none()
        && color.is_none()
        && before.is_none()
        && after.is_none()
    {
        return Err(Error::with_path(
            ErrorKind::NeedSetStyleField,
            format!("ops[{i}]"),
        ));
    }
    Ok(SetStyleOp {
        id,
        style_id,
        font,
        east_asia_font,
        sz,
        bold,
        color,
        before,
        after,
    })
}

pub(super) fn parse_set_para_style(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetParaStyleOp, Error> {
    deny_keys(obj, i, &["op", "id", "style"])?;
    let id = require_op_id(obj, i)?;
    let para = parse_story_para_at(
        &id,
        &format!("ops[{i}].id"),
        IdReason::SetParaStyleNeedsPara,
    )?;
    let style = match obj.get("style") {
        Some(Value::Null) => None,
        Some(Value::String(s)) => Some(super::super::styles::parse_style_ref(
            s,
            &format!("ops[{i}].style"),
        )?),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "style".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].style"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::NeedSetParaStyleStyle,
                format!("ops[{i}].style"),
            ));
        }
    };
    Ok(SetParaStyleOp { id, para, style })
}

pub(super) fn parse_insert_style(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertStyleOp, Error> {
    deny_keys(obj, i, &["op", "id", "styleId", "type", "name", "basedOn"])?;
    let id = require_op_id(obj, i)?;
    if id != "docx:document" {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::InsertStyleNeedsDocument,
            },
            format!("ops[{i}].id"),
        ));
    }
    let style_id = match obj.get("styleId") {
        Some(Value::String(s)) => {
            super::super::styles::validate_style_id_token(s, &format!("ops[{i}].styleId"))?;
            s.clone()
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "styleId".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].styleId"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::NeedInsertStyleField,
                format!("ops[{i}].styleId"),
            ));
        }
    };
    let ty = match obj.get("type").and_then(Value::as_str) {
        Some(t @ ("paragraph" | "character" | "table")) => t.to_string(),
        Some("numbering") | Some(_) => {
            return Err(Error::with_path(
                ErrorKind::InvalidStyleType,
                format!("ops[{i}].type"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::NeedInsertStyleField,
                format!("ops[{i}].type"),
            ));
        }
    };
    let name = match obj.get("name") {
        None => style_id.clone(),
        Some(Value::String(s)) if !s.is_empty() => s.clone(),
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::NeedInsertStyleField,
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
    let based_on = match obj.get("basedOn") {
        None | Some(Value::Null) => None,
        Some(Value::String(s)) => Some(super::super::styles::parse_style_ref(
            s,
            &format!("ops[{i}].basedOn"),
        )?),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "basedOn".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].basedOn"),
            ));
        }
    };
    Ok(InsertStyleOp {
        style_id,
        ty,
        name,
        based_on,
    })
}

pub(super) fn parse_delete_style(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteStyleOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_op_id(obj, i)?;
    let style_id = super::super::styles::parse_style_ref(&id, &format!("ops[{i}].id"))?;
    Ok(DeleteStyleOp { id, style_id })
}

pub(super) fn parse_set_run_style(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetRunStyleOp, Error> {
    deny_keys(obj, i, &["op", "id", "style"])?;
    let id = require_op_id(obj, i)?;
    let (para, run) = super::super::format::parse_run_id(&id, &format!("ops[{i}].id"))?;
    match &para {
        ParaTarget::Body(_) | ParaTarget::HeaderFooter { .. } => {}
        _ => {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::SetRunStyleNeedsStoryRun,
                },
                format!("ops[{i}].id"),
            ));
        }
    }
    let style = match obj.get("style") {
        Some(Value::Null) => None,
        Some(Value::String(s)) => Some(super::super::styles::parse_style_ref(
            s,
            &format!("ops[{i}].style"),
        )?),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "style".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].style"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::NeedSetParaStyleStyle,
                format!("ops[{i}].style"),
            ));
        }
    };
    Ok(SetRunStyleOp {
        id,
        para,
        run,
        style,
    })
}

pub(super) fn parse_set_para_format(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetParaFormatOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op",
            "id",
            "align",
            "left",
            "right",
            "firstLine",
            "hanging",
            "before",
            "after",
            "line",
            "lineRule",
            "kinsoku",
            "overflowPunct",
            "topLinePunct",
            "autoSpaceDE",
            "autoSpaceDN",
            "borders",
            "shd",
            "tabs",
        ],
    )?;
    let id = require_op_id(obj, i)?;
    let para = para::parse_para_id(&id, i)?;
    let align = parse_opt_null_enum(
        obj,
        i,
        "align",
        super::super::format::ALIGN_SET,
        ErrorKind::InvalidAlign,
    )?;
    let left = parse_opt_null_u32(obj, i, "left")?;
    let right = parse_opt_null_u32(obj, i, "right")?;
    let first_line = parse_opt_null_u32(obj, i, "firstLine")?;
    let hanging = parse_opt_null_u32(obj, i, "hanging")?;
    if matches!((&first_line, &hanging), (Some(Some(_)), Some(Some(_)))) {
        return Err(Error::with_path(
            ErrorKind::MutuallyExclusive {
                a: "firstLine".into(),
                b: "hanging".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    let before = parse_opt_null_u32(obj, i, "before")?;
    let after = parse_opt_null_u32(obj, i, "after")?;
    let line = parse_opt_null_u32(obj, i, "line")?;
    let line_rule = parse_opt_null_enum(
        obj,
        i,
        "lineRule",
        super::super::format::LINE_RULE_SET,
        ErrorKind::InvalidLineRule,
    )?;
    let kinsoku = parse_opt_null_bool(obj, i, "kinsoku")?;
    let overflow_punct = parse_opt_null_bool(obj, i, "overflowPunct")?;
    let top_line_punct = parse_opt_null_bool(obj, i, "topLinePunct")?;
    let auto_space_de = parse_opt_null_bool(obj, i, "autoSpaceDE")?;
    let auto_space_dn = parse_opt_null_bool(obj, i, "autoSpaceDN")?;
    let borders = parse_para_borders(obj, i)?;
    let shd = parse_para_shd(obj, i)?;
    let tabs = parse_para_tabs(obj, i)?;
    if align.is_none()
        && left.is_none()
        && right.is_none()
        && first_line.is_none()
        && hanging.is_none()
        && before.is_none()
        && after.is_none()
        && line.is_none()
        && line_rule.is_none()
        && kinsoku.is_none()
        && overflow_punct.is_none()
        && top_line_punct.is_none()
        && auto_space_de.is_none()
        && auto_space_dn.is_none()
        && borders.is_none()
        && shd.is_none()
        && tabs.is_none()
    {
        return Err(Error::with_path(
            ErrorKind::NeedSetParaFormatField,
            format!("ops[{i}]"),
        ));
    }
    Ok(SetParaFormatOp {
        id,
        para,
        align,
        left,
        right,
        first_line,
        hanging,
        before,
        after,
        line,
        line_rule,
        kinsoku,
        overflow_punct,
        top_line_punct,
        auto_space_de,
        auto_space_dn,
        borders,
        shd,
        tabs,
    })
}

pub(super) fn parse_set_run_format(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetRunFormatOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op",
            "id",
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
        ],
    )?;
    let id = require_op_id(obj, i)?;
    let (para, run) = super::super::format::parse_run_id(&id, &format!("ops[{i}].id"))?;
    let bold = parse_opt_bool(obj, i, "bold")?;
    let italic = parse_opt_bool(obj, i, "italic")?;
    let underline = parse_opt_null_enum(
        obj,
        i,
        "underline",
        super::super::format::UNDERLINE_SET,
        ErrorKind::InvalidUnderline,
    )?;
    let sz = parse_opt_u32_range(obj, i, "sz", 2, 3276)?;
    let color = match obj.get("color") {
        None => None,
        Some(Value::String(s)) => {
            if !is_rgb_hex(s) {
                return Err(Error::with_path(
                    ErrorKind::InvalidStyleColor,
                    format!("ops[{i}].color"),
                ));
            }
            Some(s.clone())
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "color".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].color"),
            ));
        }
    };
    let font = parse_nonempty_opt_string(obj, i, "font")?;
    let east_asia_font = parse_nonempty_opt_string(obj, i, "eastAsiaFont")?;
    let em = parse_opt_null_enum(
        obj,
        i,
        "em",
        super::super::format::EM_SET,
        ErrorKind::InvalidEm,
    )?;
    let east_asian_layout = parse_opt_east_asian_layout(obj, i)?;
    let highlight = parse_opt_null_enum(
        obj,
        i,
        "highlight",
        super::super::format::HIGHLIGHT_SET,
        ErrorKind::InvalidHighlight,
    )?;
    let strike = parse_opt_null_bool(obj, i, "strike")?;
    let dstrike = parse_opt_null_bool(obj, i, "dstrike")?;
    let vert_align = parse_opt_null_enum(
        obj,
        i,
        "vertAlign",
        super::super::format::VERT_ALIGN_SET,
        ErrorKind::InvalidVertAlign,
    )?;
    let spacing = parse_opt_null_i32(obj, i, "spacing")?;
    let kern = parse_opt_null_u32(obj, i, "kern")?;
    let scale = parse_opt_null_u32_range(obj, i, "scale", 1, 600)?;
    if bold.is_none()
        && italic.is_none()
        && underline.is_none()
        && sz.is_none()
        && color.is_none()
        && font.is_none()
        && east_asia_font.is_none()
        && em.is_none()
        && east_asian_layout.is_none()
        && highlight.is_none()
        && strike.is_none()
        && dstrike.is_none()
        && vert_align.is_none()
        && spacing.is_none()
        && kern.is_none()
        && scale.is_none()
    {
        return Err(Error::with_path(
            ErrorKind::NeedSetRunFormatField,
            format!("ops[{i}]"),
        ));
    }
    Ok(SetRunFormatOp {
        id,
        para,
        run,
        bold,
        italic,
        underline,
        sz,
        color,
        font,
        east_asia_font,
        em,
        east_asian_layout,
        highlight,
        strike,
        dstrike,
        vert_align,
        spacing,
        kern,
        scale,
    })
}

pub(super) fn parse_para_borders(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<Option<Option<super::super::format::ParaBordersPatch>>, Error> {
    match obj.get("borders") {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(None)),
        Some(Value::Object(map)) => {
            for k in map.keys() {
                if !["top", "left", "bottom", "right"].contains(&k.as_str()) {
                    return Err(Error::with_path(
                        ErrorKind::UnknownField { field: k.clone() },
                        format!("ops[{i}].borders.{k}"),
                    ));
                }
            }
            let top = parse_border_side(map.get("top"), i, "borders.top")?;
            let left = parse_border_side(map.get("left"), i, "borders.left")?;
            let bottom = parse_border_side(map.get("bottom"), i, "borders.bottom")?;
            let right = parse_border_side(map.get("right"), i, "borders.right")?;
            if top.is_none() && left.is_none() && bottom.is_none() && right.is_none() {
                return Err(Error::with_path(
                    ErrorKind::NeedParaBordersField,
                    format!("ops[{i}].borders"),
                ));
            }
            Ok(Some(Some(super::super::format::ParaBordersPatch {
                top,
                left,
                bottom,
                right,
            })))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "borders".into(),
                expected: TypeName::Object,
            },
            format!("ops[{i}].borders"),
        )),
    }
}

pub(super) fn parse_para_shd(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<Option<Option<String>>, Error> {
    match obj.get("shd") {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(None)),
        Some(Value::Object(map)) => {
            for k in map.keys() {
                if k != "fill" {
                    return Err(Error::with_path(
                        ErrorKind::UnknownField { field: k.clone() },
                        format!("ops[{i}].shd.{k}"),
                    ));
                }
            }
            let fill = match map.get("fill") {
                Some(Value::String(s)) if is_rgb_hex(s) => s.to_ascii_uppercase(),
                Some(Value::String(_)) => {
                    return Err(Error::with_path(
                        ErrorKind::InvalidStyleColor,
                        format!("ops[{i}].shd.fill"),
                    ));
                }
                Some(_) => {
                    return Err(Error::with_path(
                        ErrorKind::WrongType {
                            field: "fill".into(),
                            expected: TypeName::String,
                        },
                        format!("ops[{i}].shd.fill"),
                    ));
                }
                None => {
                    return Err(Error::with_path(
                        ErrorKind::OpRequiresField {
                            op: "setParaFormat".into(),
                            field: "shd.fill".into(),
                        },
                        format!("ops[{i}].shd"),
                    ));
                }
            };
            Ok(Some(Some(fill)))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "shd".into(),
                expected: TypeName::Object,
            },
            format!("ops[{i}].shd"),
        )),
    }
}

pub(super) fn parse_para_tabs(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<Option<Option<Vec<super::super::format::TabStop>>>, Error> {
    match obj.get("tabs") {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(None)),
        Some(Value::Array(arr)) => {
            if arr.is_empty() {
                return Ok(Some(None));
            }
            let mut out = Vec::new();
            for (j, item) in arr.iter().enumerate() {
                let Value::Object(map) = item else {
                    return Err(Error::with_path(
                        ErrorKind::WrongType {
                            field: "tabs".into(),
                            expected: TypeName::Object,
                        },
                        format!("ops[{i}].tabs[{j}]"),
                    ));
                };
                for k in map.keys() {
                    if !["val", "pos"].contains(&k.as_str()) {
                        return Err(Error::with_path(
                            ErrorKind::UnknownField { field: k.clone() },
                            format!("ops[{i}].tabs[{j}].{k}"),
                        ));
                    }
                }
                let val = match map.get("val") {
                    Some(Value::String(s))
                        if super::super::format::TAB_VAL_SET.contains(&s.as_str()) =>
                    {
                        s.clone()
                    }
                    Some(Value::String(_)) => {
                        return Err(Error::with_path(
                            ErrorKind::InvalidTabVal,
                            format!("ops[{i}].tabs[{j}].val"),
                        ));
                    }
                    Some(_) => {
                        return Err(Error::with_path(
                            ErrorKind::WrongType {
                                field: "val".into(),
                                expected: TypeName::String,
                            },
                            format!("ops[{i}].tabs[{j}].val"),
                        ));
                    }
                    None => {
                        return Err(Error::with_path(
                            ErrorKind::OpRequiresField {
                                op: "setParaFormat".into(),
                                field: "tabs.val".into(),
                            },
                            format!("ops[{i}].tabs[{j}]"),
                        ));
                    }
                };
                let pos = match map.get("pos") {
                    Some(Value::Number(n)) => {
                        let Some(v) = n.as_u64() else {
                            return Err(Error::with_path(
                                ErrorKind::WrongType {
                                    field: "pos".into(),
                                    expected: TypeName::Integer,
                                },
                                format!("ops[{i}].tabs[{j}].pos"),
                            ));
                        };
                        if v > u32::MAX as u64 {
                            return Err(Error::with_path(
                                ErrorKind::WrongType {
                                    field: "pos".into(),
                                    expected: TypeName::Integer,
                                },
                                format!("ops[{i}].tabs[{j}].pos"),
                            ));
                        }
                        v as u32
                    }
                    Some(_) => {
                        return Err(Error::with_path(
                            ErrorKind::WrongType {
                                field: "pos".into(),
                                expected: TypeName::Integer,
                            },
                            format!("ops[{i}].tabs[{j}].pos"),
                        ));
                    }
                    None => {
                        return Err(Error::with_path(
                            ErrorKind::OpRequiresField {
                                op: "setParaFormat".into(),
                                field: "tabs.pos".into(),
                            },
                            format!("ops[{i}].tabs[{j}]"),
                        ));
                    }
                };
                out.push(super::super::format::TabStop { val, pos });
            }
            Ok(Some(Some(out)))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "tabs".into(),
                expected: TypeName::Array,
            },
            format!("ops[{i}].tabs"),
        )),
    }
}

pub(super) fn parse_opt_null_u32_range(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    field: &str,
    min: u64,
    max: u64,
) -> Result<Option<Option<u32>>, Error> {
    let Some(slot) = parse_opt_null_u32(obj, i, field)? else {
        return Ok(None);
    };
    let Some(u) = slot else {
        return Ok(Some(None));
    };
    let n = u as u64;
    if n < min {
        return Err(Error::with_path(
            ErrorKind::MinValue {
                field: field.into(),
                min: min as i64,
            },
            format!("ops[{i}].{field}"),
        ));
    }
    if n > max {
        return Err(Error::with_path(
            ErrorKind::CreateIndexOutOfRange {
                field: field.into(),
                max: max as i64,
            },
            format!("ops[{i}].{field}"),
        ));
    }
    Ok(Some(Some(u)))
}

pub(super) fn parse_opt_null_i32(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    field: &str,
) -> Result<Option<Option<i32>>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(None)),
        Some(Value::Number(n)) => {
            let v = n.as_i64().ok_or_else(|| {
                Error::with_path(
                    ErrorKind::WrongType {
                        field: field.into(),
                        expected: TypeName::Integer,
                    },
                    format!("ops[{i}].{field}"),
                )
            })?;
            i32::try_from(v).map(|n| Some(Some(n))).map_err(|_| {
                Error::with_path(
                    ErrorKind::WrongType {
                        field: field.into(),
                        expected: TypeName::Integer,
                    },
                    format!("ops[{i}].{field}"),
                )
            })
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Integer,
            },
            format!("ops[{i}].{field}"),
        )),
    }
}

pub(super) fn parse_nonempty_opt_string(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    field: &str,
) -> Result<Option<String>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::String(s)) => {
            if s.is_empty() {
                return Err(Error::with_path(
                    ErrorKind::EmptyStyleFont,
                    format!("ops[{i}].{field}"),
                ));
            }
            Ok(Some(s.clone()))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].{field}"),
        )),
    }
}
