//! docx request：sect。

use serde_json::Value;

use crate::error::{Error, ErrorKind, IdReason, TypeName};

use super::super::hf::{HfKind, HfType};
use super::super::para::{self, ParaTarget};

use super::common::*;
use super::types::*;

pub(super) fn parse_set_hf_distance(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetHfDistanceOp, Error> {
    for k in obj.keys() {
        if !matches!(
            k.as_str(),
            "op" | "id" | "headerDistance" | "footerDistance"
        ) {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("ops[{i}].{k}"),
            ));
        }
    }
    let id = obj
        .get("id")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            Error::with_path(
                ErrorKind::MissingField { field: "id".into() },
                format!("ops[{i}].id"),
            )
        })?;
    let n = parse_section_id(id, i)?;
    let header_distance = parse_twip_field(obj.get("headerDistance"), i, "headerDistance")?;
    let footer_distance = parse_twip_field(obj.get("footerDistance"), i, "footerDistance")?;
    if header_distance.is_none() && footer_distance.is_none() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setHeaderFooterDistance".into(),
                field: "headerDistance or footerDistance".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(SetHfDistanceOp {
        id: id.to_string(),
        n,
        header_distance,
        footer_distance,
    })
}

pub(super) fn parse_set_hf_link(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetHfLinkOp, Error> {
    for k in obj.keys() {
        if !matches!(k.as_str(), "op" | "id" | "kind" | "type" | "linked") {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("ops[{i}].{k}"),
            ));
        }
    }
    let id = obj
        .get("id")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            Error::with_path(
                ErrorKind::MissingField { field: "id".into() },
                format!("ops[{i}].id"),
            )
        })?;
    let n = parse_section_id(id, i)?;
    let kind = match obj.get("kind") {
        Some(Value::String(s)) => HfKind::parse(s)
            .ok_or_else(|| Error::with_path(ErrorKind::InvalidHfKind, format!("ops[{i}].kind")))?,
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "kind".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].kind"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::OpRequiresField {
                    op: "setHeaderFooterLink".into(),
                    field: "kind".into(),
                },
                format!("ops[{i}].kind"),
            ));
        }
    };
    let hf_type = match obj.get("type") {
        Some(Value::String(s)) => HfType::parse(s)
            .ok_or_else(|| Error::with_path(ErrorKind::InvalidHfType, format!("ops[{i}].type")))?,
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "type".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].type"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::OpRequiresField {
                    op: "setHeaderFooterLink".into(),
                    field: "type".into(),
                },
                format!("ops[{i}].type"),
            ));
        }
    };
    let linked = match obj.get("linked") {
        Some(Value::Bool(b)) => *b,
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "linked".into(),
                    expected: TypeName::Boolean,
                },
                format!("ops[{i}].linked"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::OpRequiresField {
                    op: "setHeaderFooterLink".into(),
                    field: "linked".into(),
                },
                format!("ops[{i}].linked"),
            ));
        }
    };
    Ok(SetHfLinkOp {
        id: id.to_string(),
        n,
        kind,
        hf_type,
        linked,
    })
}

pub(super) fn parse_insert_hf(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertHeaderFooterOp, Error> {
    for k in obj.keys() {
        if !matches!(k.as_str(), "op" | "id" | "kind" | "type") {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("ops[{i}].{k}"),
            ));
        }
    }
    let id = require_op_id(obj, i)?;
    let n = parse_section_id(&id, i)?;
    let kind = parse_hf_kind(obj, i, "insertHeaderFooter")?;
    let hf_type = parse_hf_type(obj, i, "insertHeaderFooter")?;
    Ok(InsertHeaderFooterOp {
        id,
        n,
        kind,
        hf_type,
    })
}

pub(super) fn parse_insert_section(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertSectionOp, Error> {
    deny_keys(obj, i, &["op", "id", "type"])?;
    let id = require_op_id(obj, i)?;
    let break_type = match obj.get("type") {
        None => super::super::sect::SectionBreakType::NextPage,
        Some(Value::String(s)) => super::super::sect::parse_section_break_type(s)
            .map_err(|kind| Error::with_path(kind, format!("ops[{i}].type")))?,
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
    let path = format!("ops[{i}].id");
    let place = if id == "docx:document" {
        InsertSectionPlace::EndOfDocument
    } else {
        match para::parse_para_id_at(&id, &path)? {
            ParaTarget::Body(p) => InsertSectionPlace::After(p),
            _ => {
                return Err(Error::with_path(
                    ErrorKind::InvalidId {
                        reason: IdReason::InsertSectionNeedsBodyPara,
                    },
                    path,
                ));
            }
        }
    };
    Ok(InsertSectionOp {
        id,
        place,
        break_type,
    })
}

pub(super) fn parse_delete_section(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteSectionOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_op_id(obj, i)?;
    let n = parse_section_id(&id, i)?;
    Ok(DeleteSectionOp { id, n })
}

pub(super) fn parse_set_title_page(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetTitlePageOp, Error> {
    for k in obj.keys() {
        if !matches!(k.as_str(), "op" | "id" | "titlePage") {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("ops[{i}].{k}"),
            ));
        }
    }
    let id = require_op_id(obj, i)?;
    let n = parse_section_id(&id, i)?;
    let title_page = parse_required_bool(obj.get("titlePage"), i, "setTitlePage", "titlePage")?;
    Ok(SetTitlePageOp { id, n, title_page })
}

pub(super) fn parse_set_even_and_odd(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetEvenAndOddHeadersOp, Error> {
    for k in obj.keys() {
        if !matches!(k.as_str(), "op" | "id" | "evenAndOdd") {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("ops[{i}].{k}"),
            ));
        }
    }
    let id = require_op_id(obj, i)?;
    if id != "docx:document" {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::EvenOddNeedsDocument,
            },
            format!("ops[{i}].id"),
        ));
    }
    let even_and_odd = parse_required_bool(
        obj.get("evenAndOdd"),
        i,
        "setEvenAndOddHeaders",
        "evenAndOdd",
    )?;
    Ok(SetEvenAndOddHeadersOp { id, even_and_odd })
}

pub(super) fn parse_enabled_false(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    other: bool,
) -> Result<Option<bool>, Error> {
    match obj.get("enabled") {
        None => Ok(None),
        Some(Value::Bool(false)) => {
            if other {
                return Err(Error::with_path(
                    ErrorKind::MutuallyExclusive {
                        a: "enabled".into(),
                        b: "other fields".into(),
                    },
                    format!("ops[{i}].enabled"),
                ));
            }
            Ok(Some(false))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::InvalidSectValue {
                field: "enabled".into(),
            },
            format!("ops[{i}].enabled"),
        )),
    }
}

pub(super) fn parse_signed_twip_field(
    v: Option<&Value>,
    i: usize,
    field: &str,
) -> Result<Option<i64>, Error> {
    let Some(v) = v else {
        return Ok(None);
    };
    let path = format!("ops[{i}].{field}");
    match v {
        Value::Number(n) => n.as_i64().map(Some).ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: field.into(),
                    expected: TypeName::Integer,
                },
                path,
            )
        }),
        _ => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Integer,
            },
            path,
        )),
    }
}

pub(super) fn parse_set_page_margin(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetPageMarginOp, Error> {
    deny_keys(
        obj,
        i,
        &["op", "id", "top", "bottom", "left", "right", "gutter"],
    )?;
    let id = require_op_id(obj, i)?;
    let n = parse_section_id(&id, i)?;
    let top = parse_signed_twip_field(obj.get("top"), i, "top")?;
    let bottom = parse_signed_twip_field(obj.get("bottom"), i, "bottom")?;
    let left = parse_twip_field(obj.get("left"), i, "left")?;
    let right = parse_twip_field(obj.get("right"), i, "right")?;
    let gutter = parse_twip_field(obj.get("gutter"), i, "gutter")?;
    if top.is_none() && bottom.is_none() && left.is_none() && right.is_none() && gutter.is_none() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setPageMargin".into(),
                field: "top, bottom, left, right, or gutter".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(SetPageMarginOp {
        id,
        n,
        top,
        bottom,
        left,
        right,
        gutter,
    })
}

pub(super) fn parse_set_page_size(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetPageSizeOp, Error> {
    deny_keys(obj, i, &["op", "id", "pageWidth", "pageHeight", "orient"])?;
    let id = require_op_id(obj, i)?;
    let n = parse_section_id(&id, i)?;
    let page_width = parse_twip_field(obj.get("pageWidth"), i, "pageWidth")?;
    let page_height = parse_twip_field(obj.get("pageHeight"), i, "pageHeight")?;
    let orient = parse_enum_field(obj.get("orient"), i, "orient", &["portrait", "landscape"])?;
    if page_width.is_none() && page_height.is_none() && orient.is_none() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setPageSize".into(),
                field: "pageWidth, pageHeight, or orient".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(SetPageSizeOp {
        id,
        n,
        page_width,
        page_height,
        orient,
    })
}

pub(super) fn parse_col_items(v: Option<&Value>, i: usize) -> Result<Option<Vec<ColWidth>>, Error> {
    let Some(v) = v else {
        return Ok(None);
    };
    let Value::Array(arr) = v else {
        return Err(Error::with_path(
            ErrorKind::WrongType {
                field: "cols".into(),
                expected: TypeName::Array,
            },
            format!("ops[{i}].cols"),
        ));
    };
    let mut out = Vec::new();
    for (j, item) in arr.iter().enumerate() {
        let Value::Object(m) = item else {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "cols".into(),
                    expected: TypeName::Object,
                },
                format!("ops[{i}].cols[{j}]"),
            ));
        };
        for k in m.keys() {
            if !matches!(k.as_str(), "width" | "space") {
                return Err(Error::with_path(
                    ErrorKind::UnknownField { field: k.clone() },
                    format!("ops[{i}].cols[{j}].{k}"),
                ));
            }
        }
        out.push(ColWidth {
            width: parse_twip_field(m.get("width"), i, "width")?,
            space: parse_twip_field(m.get("space"), i, "space")?,
        });
    }
    Ok(Some(out))
}

pub(super) fn parse_set_columns(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetColumnsOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op",
            "id",
            "num",
            "space",
            "equalWidth",
            "sep",
            "cols",
            "enabled",
        ],
    )?;
    let id = require_op_id(obj, i)?;
    let n = parse_section_id(&id, i)?;
    let num = parse_twip_field(obj.get("num"), i, "num")?;
    if let Some(0) = num {
        return Err(Error::with_path(
            ErrorKind::MinValue {
                field: "num".into(),
                min: 1,
            },
            format!("ops[{i}].num"),
        ));
    }
    let space = parse_twip_field(obj.get("space"), i, "space")?;
    let equal_width = parse_optional_bool_field(obj.get("equalWidth"), i, "equalWidth")?;
    let sep = parse_optional_bool_field(obj.get("sep"), i, "sep")?;
    let cols = parse_col_items(obj.get("cols"), i)?;
    if equal_width == Some(true) && cols.is_some() {
        return Err(Error::with_path(
            ErrorKind::MutuallyExclusive {
                a: "equalWidth".into(),
                b: "cols".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    let other = num.is_some()
        || space.is_some()
        || equal_width.is_some()
        || sep.is_some()
        || cols.is_some();
    let enabled = parse_enabled_false(obj, i, other)?;
    if enabled.is_none() && !other {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setColumns".into(),
                field: "num, space, equalWidth, sep, cols, or enabled".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(SetColumnsOp {
        id,
        n,
        num,
        space,
        equal_width,
        sep,
        cols,
        enabled,
    })
}

pub(super) fn parse_set_vertical_align(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetVerticalAlignOp, Error> {
    deny_keys(obj, i, &["op", "id", "vAlign"])?;
    let id = require_op_id(obj, i)?;
    let n = parse_section_id(&id, i)?;
    let v_align = match obj.get("vAlign") {
        None => {
            return Err(Error::with_path(
                ErrorKind::OpRequiresField {
                    op: "setVerticalAlign".into(),
                    field: "vAlign".into(),
                },
                format!("ops[{i}].vAlign"),
            ));
        }
        Some(Value::Null) => None,
        Some(Value::String(s)) if matches!(s.as_str(), "top" | "center" | "both" | "bottom") => {
            Some(s.clone())
        }
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidSectValue {
                    field: "vAlign".into(),
                },
                format!("ops[{i}].vAlign"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "vAlign".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].vAlign"),
            ));
        }
    };
    Ok(SetVerticalAlignOp { id, n, v_align })
}

pub(super) fn parse_set_line_numbering(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetLineNumberingOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op", "id", "countBy", "start", "distance", "restart", "enabled",
        ],
    )?;
    let id = require_op_id(obj, i)?;
    let n = parse_section_id(&id, i)?;
    let count_by = parse_twip_field(obj.get("countBy"), i, "countBy")?;
    let start = parse_twip_field(obj.get("start"), i, "start")?;
    let distance = parse_twip_field(obj.get("distance"), i, "distance")?;
    let restart = parse_enum_field(
        obj.get("restart"),
        i,
        "restart",
        &["newPage", "newSection", "continuous"],
    )?;
    let other = count_by.is_some() || start.is_some() || distance.is_some() || restart.is_some();
    let enabled = parse_enabled_false(obj, i, other)?;
    if enabled.is_none() && !other {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setLineNumbering".into(),
                field: "countBy, start, distance, restart, or enabled".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(SetLineNumberingOp {
        id,
        n,
        count_by,
        start,
        distance,
        restart,
        enabled,
    })
}

pub(super) fn parse_set_doc_grid(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetDocGridOp, Error> {
    deny_keys(
        obj,
        i,
        &["op", "id", "type", "linePitch", "charSpace", "enabled"],
    )?;
    let id = require_op_id(obj, i)?;
    let n = parse_section_id(&id, i)?;
    let grid_type = parse_enum_field(
        obj.get("type"),
        i,
        "type",
        &["default", "lines", "linesAndChars", "snapToChars"],
    )?;
    let line_pitch = parse_i64_field(obj.get("linePitch"), i, "linePitch")?;
    let char_space = parse_i64_field(obj.get("charSpace"), i, "charSpace")?;
    let other = grid_type.is_some() || line_pitch.is_some() || char_space.is_some();
    let enabled = parse_enabled_false(obj, i, other)?;
    if enabled.is_none() && !other {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setDocGrid".into(),
                field: "type, linePitch, charSpace, or enabled".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(SetDocGridOp {
        id,
        n,
        grid_type,
        line_pitch,
        char_space,
        enabled,
    })
}

pub(super) fn parse_set_rtl_gutter(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetRtlGutterOp, Error> {
    deny_keys(obj, i, &["op", "id", "rtlGutter"])?;
    let id = require_op_id(obj, i)?;
    let n = parse_section_id(&id, i)?;
    let rtl_gutter = parse_required_bool(obj.get("rtlGutter"), i, "setRtlGutter", "rtlGutter")?;
    Ok(SetRtlGutterOp { id, n, rtl_gutter })
}

pub(super) fn parse_set_mirror_margins(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetMirrorMarginsOp, Error> {
    deny_keys(obj, i, &["op", "id", "mirrorMargins"])?;
    let id = require_op_id(obj, i)?;
    require_document_id(&id, i, IdReason::SettingsNeedDocument)?;
    let mirror_margins = parse_required_bool(
        obj.get("mirrorMargins"),
        i,
        "setMirrorMargins",
        "mirrorMargins",
    )?;
    Ok(SetMirrorMarginsOp { id, mirror_margins })
}

pub(super) fn parse_set_gutter_at_top(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetGutterAtTopOp, Error> {
    deny_keys(obj, i, &["op", "id", "gutterAtTop"])?;
    let id = require_op_id(obj, i)?;
    require_document_id(&id, i, IdReason::SettingsNeedDocument)?;
    let gutter_at_top =
        parse_required_bool(obj.get("gutterAtTop"), i, "setGutterAtTop", "gutterAtTop")?;
    Ok(SetGutterAtTopOp { id, gutter_at_top })
}

pub(super) fn parse_hf_kind(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    op: &str,
) -> Result<HfKind, Error> {
    match obj.get("kind") {
        Some(Value::String(s)) => HfKind::parse(s)
            .ok_or_else(|| Error::with_path(ErrorKind::InvalidHfKind, format!("ops[{i}].kind"))),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "kind".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].kind"),
        )),
        None => Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: op.into(),
                field: "kind".into(),
            },
            format!("ops[{i}].kind"),
        )),
    }
}

pub(super) fn parse_hf_type(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    op: &str,
) -> Result<HfType, Error> {
    match obj.get("type") {
        Some(Value::String(s)) => HfType::parse(s)
            .ok_or_else(|| Error::with_path(ErrorKind::InvalidHfType, format!("ops[{i}].type"))),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "type".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].type"),
        )),
        None => Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: op.into(),
                field: "type".into(),
            },
            format!("ops[{i}].type"),
        )),
    }
}

pub(super) fn parse_section_id(id: &str, op_index: usize) -> Result<usize, Error> {
    let path = format!("ops[{op_index}].id");
    let rest = id.strip_prefix("section:").ok_or_else(|| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SectionPrefix,
            },
            path.clone(),
        )
    })?;
    let n: usize = rest.parse().map_err(|_| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SectionIndexNotInt,
            },
            path.clone(),
        )
    })?;
    if n < 1 {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SectionIndexFromOne,
            },
            path,
        ));
    }
    Ok(n)
}
