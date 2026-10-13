//! docx request：math_ruby。

use serde_json::Value;

use crate::error::{Error, ErrorKind, IdReason, TypeName};

use super::common::*;
use super::types::*;

pub(super) fn parse_insert_math(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertMathOp, Error> {
    deny_keys(obj, i, &["op", "id", "kind", "text", "expr", "jc"])?;
    let id = require_op_id(obj, i)?;
    let kind = match obj.get("kind") {
        None => super::super::math::MathKind::Inline,
        Some(Value::String(s)) if s == "inline" => super::super::math::MathKind::Inline,
        Some(Value::String(s)) if s == "display" => super::super::math::MathKind::Display,
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidMathKind,
                format!("ops[{i}].kind"),
            ));
        }
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
    let jc = match obj.get("jc") {
        None => None,
        Some(Value::String(s)) if super::super::math::MATH_JC.contains(&s.as_str()) => {
            Some(s.clone())
        }
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidMathJc,
                format!("ops[{i}].jc"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "jc".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].jc"),
            ));
        }
    };
    if jc.is_some() && kind == super::super::math::MathKind::Inline {
        return Err(Error::with_path(
            ErrorKind::MathJcOnInline,
            format!("ops[{i}].jc"),
        ));
    }
    let body = super::super::math_expr::parse_math_body(obj, &format!("ops[{i}]"))?;
    let path = format!("ops[{i}].id");
    let place = if id == "docx:document" {
        InsertMathPlace::EndOfDocument
    } else {
        InsertMathPlace::After(parse_story_or_box_para_at(
            &id,
            &path,
            IdReason::InsertMathNeedsPara,
        )?)
    };
    Ok(InsertMathOp {
        id,
        place,
        kind,
        body,
        jc,
    })
}

pub(super) fn parse_set_math(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetMathOp, Error> {
    deny_keys(obj, i, &["op", "id", "text", "expr", "jc"])?;
    let id = require_op_id(obj, i)?;
    let target = super::super::math::parse_math_ref(&id, &format!("ops[{i}].id"))?;
    let jc = parse_opt_null_enum(
        obj,
        i,
        "jc",
        super::super::math::MATH_JC,
        ErrorKind::InvalidMathJc,
    )?;
    let has_text = obj.contains_key("text");
    let has_expr = obj.contains_key("expr");
    let body = if has_text || has_expr {
        Some(super::super::math_expr::parse_math_body(
            obj,
            &format!("ops[{i}]"),
        )?)
    } else if jc.is_some() {
        None
    } else {
        return Err(Error::with_path(
            ErrorKind::MathNeedTextOrExpr,
            format!("ops[{i}]"),
        ));
    };
    Ok(SetMathOp {
        id,
        target,
        body,
        jc,
    })
}

pub(super) fn parse_delete_math(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteMathOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_op_id(obj, i)?;
    let target = super::super::math::parse_math_ref(&id, &format!("ops[{i}].id"))?;
    Ok(DeleteMathOp { id, target })
}

pub(super) fn parse_insert_ruby(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertRubyOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op",
            "id",
            "base",
            "rt",
            "align",
            "hps",
            "hpsRaise",
            "hpsBaseText",
            "lid",
        ],
    )?;
    let id = require_op_id(obj, i)?;
    let base = require_nonempty(obj, i, "insertRuby", "base", ErrorKind::EmptyRubyBase)?;
    let rt = require_nonempty(obj, i, "insertRuby", "rt", ErrorKind::EmptyRubyRt)?;
    let align = match obj.get("align") {
        None => "distributeSpace".to_string(),
        Some(Value::String(s)) => {
            if !super::super::ruby::ALIGN_SET.contains(&s.as_str()) {
                return Err(Error::with_path(
                    ErrorKind::InvalidRubyAlign,
                    format!("ops[{i}].align"),
                ));
            }
            s.clone()
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "align".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].align"),
            ));
        }
    };
    let hps = parse_opt_u32_range(obj, i, "hps", 2, 3276)?.unwrap_or(12);
    let hps_raise = parse_opt_u32_range(obj, i, "hpsRaise", 2, 3276)?.unwrap_or(hps);
    let hps_base_text = parse_opt_u32_range(obj, i, "hpsBaseText", 2, 3276)?.unwrap_or(24);
    let lid = match obj.get("lid") {
        None => "zh-CN".to_string(),
        Some(Value::String(s)) => {
            if s.is_empty() {
                return Err(Error::with_path(
                    ErrorKind::EmptyRubyLid,
                    format!("ops[{i}].lid"),
                ));
            }
            s.clone()
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "lid".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].lid"),
            ));
        }
    };
    let path = format!("ops[{i}].id");
    let place = if id == "docx:document" {
        InsertRubyPlace::EndOfDocument
    } else {
        InsertRubyPlace::After(parse_story_or_box_para_at(
            &id,
            &path,
            IdReason::InsertRubyNeedsPara,
        )?)
    };
    Ok(InsertRubyOp {
        id,
        place,
        base,
        rt,
        align,
        hps,
        hps_raise,
        hps_base_text,
        lid,
    })
}

pub(super) fn parse_set_ruby(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetRubyOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op",
            "id",
            "base",
            "rt",
            "align",
            "hps",
            "hpsRaise",
            "hpsBaseText",
            "lid",
        ],
    )?;
    let id = require_op_id(obj, i)?;
    let target = super::super::ruby::parse_ruby_ref(&id, &format!("ops[{i}].id"))?;
    let base = parse_opt_nonempty(obj, i, "base", ErrorKind::EmptyRubyBase)?;
    let rt = parse_opt_nonempty(obj, i, "rt", ErrorKind::EmptyRubyRt)?;
    let align = parse_opt_null_enum(
        obj,
        i,
        "align",
        super::super::ruby::ALIGN_SET,
        ErrorKind::InvalidRubyAlign,
    )?;
    let align = match align {
        Some(Some(s)) => Some(s),
        Some(None) => {
            return Err(Error::with_path(
                ErrorKind::InvalidRubyAlign,
                format!("ops[{i}].align"),
            ));
        }
        None => None,
    };
    let hps = parse_opt_u32_range(obj, i, "hps", 2, 3276)?;
    let hps_raise = parse_opt_u32_range(obj, i, "hpsRaise", 2, 3276)?;
    let hps_base_text = parse_opt_u32_range(obj, i, "hpsBaseText", 2, 3276)?;
    let lid = parse_opt_nonempty(obj, i, "lid", ErrorKind::EmptyRubyLid)?;
    if base.is_none()
        && rt.is_none()
        && align.is_none()
        && hps.is_none()
        && hps_raise.is_none()
        && hps_base_text.is_none()
        && lid.is_none()
    {
        return Err(Error::with_path(
            ErrorKind::NeedSetRubyField,
            format!("ops[{i}]"),
        ));
    }
    Ok(SetRubyOp {
        id,
        target,
        base,
        rt,
        align,
        hps,
        hps_raise,
        hps_base_text,
        lid,
    })
}

pub(super) fn parse_delete_ruby(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteRubyOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_op_id(obj, i)?;
    let target = super::super::ruby::parse_ruby_ref(&id, &format!("ops[{i}].id"))?;
    Ok(DeleteRubyOp { id, target })
}

pub(super) fn parse_set_east_asian_settings(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetEastAsianSettingsOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op",
            "id",
            "characterSpacingControl",
            "strictFirstAndLastChars",
            "noLineBreaksBefore",
            "noLineBreaksAfter",
            "themeFontLang",
        ],
    )?;
    let id = require_op_id(obj, i)?;
    require_document_id(&id, i, IdReason::SetEastAsianSettingsNeedsDocument)?;
    let character_spacing = parse_opt_null_enum(
        obj,
        i,
        "characterSpacingControl",
        super::super::east_asia::CHARACTER_SPACING_SET,
        ErrorKind::InvalidCharacterSpacing,
    )?;
    let strict = parse_opt_null_bool(obj, i, "strictFirstAndLastChars")?;
    let no_before = parse_opt_kinsoku(obj, i, "noLineBreaksBefore")?;
    let no_after = parse_opt_kinsoku(obj, i, "noLineBreaksAfter")?;
    let theme_font_lang = parse_opt_theme_font_lang(obj, i)?;
    if character_spacing.is_none()
        && strict.is_none()
        && no_before.is_none()
        && no_after.is_none()
        && theme_font_lang.is_none()
    {
        return Err(Error::with_path(
            ErrorKind::NeedSetEastAsianSettingsField,
            format!("ops[{i}]"),
        ));
    }
    Ok(SetEastAsianSettingsOp {
        character_spacing,
        strict,
        no_before,
        no_after,
        theme_font_lang,
    })
}

pub(super) fn parse_opt_theme_font_lang(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<Option<Option<super::super::east_asia::ThemeFontLang>>, Error> {
    parse_theme_font_lang_at(obj.get("themeFontLang"), &format!("ops[{i}].themeFontLang"))
}

pub(super) fn parse_theme_font_lang_at(
    v: Option<&Value>,
    path: &str,
) -> Result<Option<Option<super::super::east_asia::ThemeFontLang>>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(None)),
        Some(Value::Object(inner)) => {
            for k in inner.keys() {
                if !matches!(k.as_str(), "val" | "eastAsia" | "bidi") {
                    return Err(Error::with_path(
                        ErrorKind::UnknownField { field: k.clone() },
                        format!("{path}.{k}"),
                    ));
                }
            }
            Ok(Some(Some(super::super::east_asia::ThemeFontLang {
                val: parse_opt_lang_attr(inner.get("val"), &format!("{path}.val"))?,
                east_asia: parse_opt_lang_attr(inner.get("eastAsia"), &format!("{path}.eastAsia"))?,
                bidi: parse_opt_lang_attr(inner.get("bidi"), &format!("{path}.bidi"))?,
            })))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "themeFontLang".into(),
                expected: TypeName::Object,
            },
            path.to_string(),
        )),
    }
}

pub(super) fn parse_opt_lang_attr(v: Option<&Value>, path: &str) -> Result<Option<String>, Error> {
    match v {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) if !s.is_empty() => Ok(Some(s.clone())),
        Some(Value::String(_)) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "themeFontLang".into(),
                expected: TypeName::NonEmptyString,
            },
            path.to_string(),
        )),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "themeFontLang".into(),
                expected: TypeName::String,
            },
            path.to_string(),
        )),
    }
}

pub(super) fn parse_opt_kinsoku(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    field: &str,
) -> Result<Option<Option<super::super::east_asia::KinsokuChars>>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(None)),
        Some(Value::Object(inner)) => {
            for k in inner.keys() {
                if !matches!(k.as_str(), "lang" | "chars") {
                    return Err(Error::with_path(
                        ErrorKind::UnknownField { field: k.clone() },
                        format!("ops[{i}].{field}.{k}"),
                    ));
                }
            }
            let lang = match inner.get("lang") {
                Some(Value::String(s)) if !s.is_empty() => s.clone(),
                Some(Value::String(_)) => {
                    return Err(Error::with_path(
                        ErrorKind::EmptyKinsokuLang,
                        format!("ops[{i}].{field}.lang"),
                    ));
                }
                Some(_) => {
                    return Err(Error::with_path(
                        ErrorKind::WrongType {
                            field: "lang".into(),
                            expected: TypeName::String,
                        },
                        format!("ops[{i}].{field}.lang"),
                    ));
                }
                None => {
                    return Err(Error::with_path(
                        ErrorKind::EmptyKinsokuLang,
                        format!("ops[{i}].{field}.lang"),
                    ));
                }
            };
            let chars = match inner.get("chars") {
                Some(Value::String(s)) if !s.is_empty() => s.clone(),
                Some(Value::String(_)) => {
                    return Err(Error::with_path(
                        ErrorKind::EmptyKinsokuChars,
                        format!("ops[{i}].{field}.chars"),
                    ));
                }
                Some(_) => {
                    return Err(Error::with_path(
                        ErrorKind::WrongType {
                            field: "chars".into(),
                            expected: TypeName::String,
                        },
                        format!("ops[{i}].{field}.chars"),
                    ));
                }
                None => {
                    return Err(Error::with_path(
                        ErrorKind::EmptyKinsokuChars,
                        format!("ops[{i}].{field}.chars"),
                    ));
                }
            };
            Ok(Some(Some(super::super::east_asia::KinsokuChars {
                lang,
                chars,
            })))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Object,
            },
            format!("ops[{i}].{field}"),
        )),
    }
}

pub(super) fn parse_opt_east_asian_layout(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<Option<Option<super::super::format::EastAsianLayout>>, Error> {
    match obj.get("eastAsianLayout") {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(None)),
        Some(Value::Object(inner)) => {
            for k in inner.keys() {
                if !matches!(
                    k.as_str(),
                    "id" | "combine" | "combineBrackets" | "vert" | "vertCompress"
                ) {
                    return Err(Error::with_path(
                        ErrorKind::UnknownField { field: k.clone() },
                        format!("ops[{i}].eastAsianLayout.{k}"),
                    ));
                }
            }
            let id = parse_layout_id(inner, i)?;
            let combine = parse_inner_opt_bool(inner, i, "combine")?;
            let vert = parse_inner_opt_bool(inner, i, "vert")?;
            let vert_compress = parse_inner_opt_bool(inner, i, "vertCompress")?;
            let combine_brackets = match inner.get("combineBrackets") {
                None => None,
                Some(Value::Null) => None,
                Some(Value::String(s)) => {
                    if !super::super::format::COMBINE_BRACKETS_SET.contains(&s.as_str()) {
                        return Err(Error::with_path(
                            ErrorKind::InvalidCombineBrackets,
                            format!("ops[{i}].eastAsianLayout.combineBrackets"),
                        ));
                    }
                    Some(s.clone())
                }
                Some(_) => {
                    return Err(Error::with_path(
                        ErrorKind::WrongType {
                            field: "combineBrackets".into(),
                            expected: TypeName::String,
                        },
                        format!("ops[{i}].eastAsianLayout.combineBrackets"),
                    ));
                }
            };
            if combine.is_none() && vert.is_none() {
                return Err(Error::with_path(
                    ErrorKind::NeedEastAsianLayoutField,
                    format!("ops[{i}].eastAsianLayout"),
                ));
            }
            if combine_brackets.is_some() && combine != Some(true) {
                return Err(Error::with_path(
                    ErrorKind::InvalidCombineBrackets,
                    format!("ops[{i}].eastAsianLayout.combineBrackets"),
                ));
            }
            Ok(Some(Some(super::super::format::EastAsianLayout {
                id,
                combine,
                combine_brackets,
                vert,
                vert_compress,
            })))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "eastAsianLayout".into(),
                expected: TypeName::Object,
            },
            format!("ops[{i}].eastAsianLayout"),
        )),
    }
}

pub(super) fn parse_inner_opt_bool(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    field: &str,
) -> Result<Option<bool>, Error> {
    match obj.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Bool(b)) => Ok(Some(*b)),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Boolean,
            },
            format!("ops[{i}].eastAsianLayout.{field}"),
        )),
    }
}
