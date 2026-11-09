//! pptx request：shape。

use serde_json::{Map, Value};

use crate::error::{Error, ErrorKind, TypeName};

use super::super::connector;
use super::super::group;
use super::super::picture;

use super::common::*;
use super::types::*;

pub(super) fn parse_set_shape_text(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<SetShapeTextOp, Error> {
    deny_keys(obj, i, &["op", "id", "text"])?;
    let id = require_id(obj, i)?;
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
                    op: "setShapeText".into(),
                    field: "text".into(),
                },
                format!("ops[{i}].text"),
            ));
        }
    };
    Ok(SetShapeTextOp { id, text })
}

pub(super) fn parse_set_shape_runs(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<SetShapeRunsOp, Error> {
    deny_keys(obj, i, &["op", "id", "runs"])?;
    let id = require_id(obj, i)?;
    let runs = crate::shape_runs::parse_shape_runs(obj, i)?;
    Ok(SetShapeRunsOp { id, runs })
}

pub(super) fn parse_insert_shape(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<InsertShapeOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op",
            "id",
            "text",
            "x",
            "y",
            "cx",
            "cy",
            "prst",
            "sz",
            "color",
            "textFill",
            "textOutline",
            "bold",
            "italic",
            "underline",
            "font",
            "eastAsiaFont",
            "align",
            "textAnchor",
            "spcBef",
            "spcAft",
            "lnSpc",
            "spc",
            "kern",
            "geom",
            "fill",
            "line",
            "index",
            "before",
            "after",
        ],
    )?;
    let id = require_id(obj, i)?;
    let host = parse_insert_shape_host(&id, i)?;
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
    if obj.contains_key("geom") && obj.contains_key("prst") {
        return Err(Error::with_path(
            ErrorKind::MutuallyExclusive {
                a: "prst".into(),
                b: "geom".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    let geom = match obj.get("geom") {
        None => None,
        Some(v) => Some(parse_geom_object(v, i, "geom")?),
    };
    let prst = if geom.is_some() {
        String::new()
    } else {
        match obj.get("prst") {
            None => "rect".to_string(),
            Some(Value::String(s)) => {
                if crate::drawing_geom::is_writable_shape_prst(s) {
                    s.clone()
                } else {
                    return Err(Error::with_path(
                        ErrorKind::InvalidShapePrst,
                        format!("ops[{i}].prst"),
                    ));
                }
            }
            Some(_) => {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: "prst".into(),
                        expected: TypeName::String,
                    },
                    format!("ops[{i}].prst"),
                ));
            }
        }
    };
    let x = parse_signed_off_field(obj.get("x"), i, "x")?;
    let y = parse_signed_off_field(obj.get("y"), i, "y")?;
    if x.is_some() != y.is_some() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "insertShape".into(),
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
                op: "insertShape".into(),
                field: "cx and cy together".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    let fill = match obj.get("fill") {
        None => None,
        Some(v) => Some(parse_paint_value(v, &format!("ops[{i}].fill"), None)?),
    };
    let line = match obj.get("line") {
        None => None,
        Some(v) => Some(parse_shape_line(v, &format!("ops[{i}].line"))?),
    };
    let place = parse_sp_tree_place(obj, &format!("ops[{i}]"))?;
    Ok(InsertShapeOp {
        host,
        text,
        x,
        y,
        cx,
        cy,
        prst,
        geom,
        format: crate::drawing_text::parse(obj, &format!("ops[{i}]"), false)?,
        fill,
        line,
        place,
    })
}

pub(super) fn parse_set_shape_format(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<SetShapeFormatOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op",
            "id",
            "sz",
            "color",
            "textFill",
            "textOutline",
            "bold",
            "italic",
            "underline",
            "font",
            "eastAsiaFont",
            "align",
            "textAnchor",
            "spcBef",
            "spcAft",
            "lnSpc",
            "spc",
            "kern",
        ],
    )?;
    let id = require_id(obj, i)?;
    let format = crate::drawing_text::parse(obj, &format!("ops[{i}]"), true)?
        .expect("require_one 已保证有补丁");
    Ok(SetShapeFormatOp { id, format })
}

pub(super) fn parse_insert_connector(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<InsertConnectorOp, Error> {
    deny_keys(
        obj,
        i,
        &["op", "id", "prst", "x", "y", "cx", "cy", "start", "end"],
    )?;
    let id = require_id(obj, i)?;
    let (slide, on_notes, master) = connector::parse_insert_slide_id(&id, i)?.parts();
    let prst = match obj.get("prst") {
        None => "straightConnector1".to_string(),
        Some(Value::String(s)) => {
            if crate::drawing_geom::is_writable_connector_prst(s) {
                s.clone()
            } else {
                return Err(Error::with_path(
                    ErrorKind::InvalidConnectorPrst,
                    format!("ops[{i}].prst"),
                ));
            }
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "prst".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].prst"),
            ));
        }
    };
    let x = parse_off_field(obj.get("x"), i, "x")?;
    let y = parse_off_field(obj.get("y"), i, "y")?;
    if x.is_some() != y.is_some() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "insertConnector".into(),
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
                op: "insertConnector".into(),
                field: "cx and cy together".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(InsertConnectorOp {
        slide,
        on_notes,
        master,
        prst,
        x,
        y,
        cx,
        cy,
        start: parse_opt_cxn_bind(obj, i, "start")?,
        end: parse_opt_cxn_bind(obj, i, "end")?,
    })
}

pub(super) fn parse_delete_connector(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<DeleteConnectorOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let target = connector::parse_cxn_id(&id, i)?;
    Ok(DeleteConnectorOp { id, target })
}

pub(super) fn parse_group_shapes(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<GroupShapesOp, Error> {
    deny_keys(obj, i, &["op", "id", "ids"])?;
    let id = require_id(obj, i)?;
    let slide = connector::parse_slide_only(&id, i, crate::error::IdReason::SlideOpNeedsSlide)?;
    let ids = match obj.get("ids") {
        Some(Value::Array(arr)) => {
            if arr.len() < 2 {
                return Err(Error::with_path(
                    ErrorKind::GroupNeedsTwo,
                    format!("ops[{i}].ids"),
                ));
            }
            let mut out = Vec::new();
            for (k, v) in arr.iter().enumerate() {
                let s = v.as_str().ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::WrongType {
                            field: format!("ids[{k}]"),
                            expected: TypeName::String,
                        },
                        format!("ops[{i}].ids[{k}]"),
                    )
                })?;
                if s.is_empty() {
                    return Err(Error::with_path(
                        ErrorKind::WrongType {
                            field: format!("ids[{k}]"),
                            expected: TypeName::NonEmptyString,
                        },
                        format!("ops[{i}].ids[{k}]"),
                    ));
                }
                out.push(s.to_string());
            }
            out
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "ids".into(),
                    expected: TypeName::Array,
                },
                format!("ops[{i}].ids"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::OpRequiresField {
                    op: "groupShapes".into(),
                    field: "ids".into(),
                },
                format!("ops[{i}].ids"),
            ));
        }
    };
    let mut members = Vec::new();
    for (k, mid) in ids.iter().enumerate() {
        let m = group::parse_member_id(mid, i).map_err(|e| e.at(format!("ops[{i}].ids[{k}]")))?;
        members.push(m);
    }
    Ok(GroupShapesOp {
        slide,
        ids,
        members,
    })
}

pub(super) fn parse_ungroup(obj: &Map<String, Value>, i: usize) -> Result<UngroupOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let member = group::parse_member_id(&id, i)?;
    Ok(UngroupOp { id, member })
}

pub(super) fn parse_delete_shape(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<DeleteShapeOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let (target, placeholder) = parse_slide_or_master_shape(&id, i)?;
    Ok(DeleteShapeOp {
        id,
        target,
        placeholder,
    })
}

pub(super) fn parse_set_shape_pos(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<SetShapePosOp, Error> {
    deny_keys(obj, i, &["op", "id", "x", "y"])?;
    let id = require_id(obj, i)?;
    let (target, placeholder, cxn) = parse_shape_pos_host(&id, i)?;
    let (x, y) = if cxn.is_some() {
        (
            parse_off_field(obj.get("x"), i, "x")?,
            parse_off_field(obj.get("y"), i, "y")?,
        )
    } else {
        (
            parse_signed_off_field(obj.get("x"), i, "x")?,
            parse_signed_off_field(obj.get("y"), i, "y")?,
        )
    };
    if x.is_none() && y.is_none() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setShapePos".into(),
                field: "x or y".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(SetShapePosOp {
        id,
        target,
        placeholder,
        cxn,
        x,
        y,
    })
}

pub(super) fn parse_set_shape_size(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<SetShapeSizeOp, Error> {
    deny_keys(obj, i, &["op", "id", "cx", "cy"])?;
    let id = require_id(obj, i)?;
    let (target, placeholder, cxn) = parse_shape_pos_host(&id, i)?;
    let cx = parse_emu_field(obj.get("cx"), i, "cx")?;
    let cy = parse_emu_field(obj.get("cy"), i, "cy")?;
    if cx.is_none() && cy.is_none() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setShapeSize".into(),
                field: "cx or cy".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(SetShapeSizeOp {
        id,
        target,
        placeholder,
        cxn,
        cx,
        cy,
    })
}

pub(super) fn parse_set_shape_fill(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<SetShapeFillOp, Error> {
    deny_keys(obj, i, &["op", "id", "color", "fill", "alpha"])?;
    let id = require_id(obj, i)?;
    let (target, placeholder) = parse_slide_or_master_shape(&id, i)?;
    let has_color = obj.contains_key("color");
    let has_fill = obj.contains_key("fill");
    if has_color == has_fill {
        if has_color {
            return Err(Error::with_path(
                ErrorKind::MutuallyExclusive {
                    a: "color".into(),
                    b: "fill".into(),
                },
                format!("ops[{i}]"),
            ));
        }
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setShapeFill".into(),
                field: "color".into(),
            },
            format!("ops[{i}].color"),
        ));
    }
    let alpha = parse_alpha_at(obj.get("alpha"), &format!("ops[{i}].alpha"))?;
    let paint = if has_color {
        let color = parse_color_field(obj, i, true, "setShapeFill")?;
        paint_from_color(color, alpha, &format!("ops[{i}].alpha"))?
    } else {
        parse_paint_value(
            obj.get("fill").expect("已检查"),
            &format!("ops[{i}].fill"),
            alpha,
        )?
    };
    Ok(SetShapeFillOp {
        id,
        target,
        placeholder,
        paint,
    })
}

pub(super) fn parse_set_page_background(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<SetPageBackgroundOp, Error> {
    deny_keys(obj, i, &["op", "id", "color", "fill", "alpha"])?;
    let id = require_id(obj, i)?;
    let host = picture::parse_insert_host(&id, i, crate::error::IdReason::PageBackgroundHost)?;
    if !obj.contains_key("color") {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setPageBackground".into(),
                field: "color".into(),
            },
            format!("ops[{i}].color"),
        ));
    }
    let color = parse_rgb_value(obj.get("color"), &format!("ops[{i}].color"), true)?;
    let alpha = parse_alpha_at(obj.get("alpha"), &format!("ops[{i}].alpha"))?;
    let fill = match obj.get("fill") {
        None => None,
        Some(v) => Some(parse_page_fill(v, &format!("ops[{i}].fill"))?),
    };
    if color.is_none() && (alpha.is_some() || fill.is_some()) {
        return Err(Error::with_path(
            ErrorKind::MutuallyExclusive {
                a: "color".into(),
                b: "fill/alpha".into(),
            },
            format!("ops[{i}].color"),
        ));
    }
    if alpha.is_some() && matches!(fill, Some(PageFill::Picture { .. })) {
        return Err(Error::with_path(
            ErrorKind::MutuallyExclusive {
                a: "alpha".into(),
                b: "picture".into(),
            },
            format!("ops[{i}].alpha"),
        ));
    }
    Ok(SetPageBackgroundOp {
        host,
        color,
        alpha,
        fill,
    })
}

pub(super) fn parse_set_shape_line(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<SetShapeLineOp, Error> {
    deny_keys(obj, i, &["op", "id", "color", "width"])?;
    let id = require_id(obj, i)?;
    let (target, placeholder, cxn) = if id.starts_with("master:")
        || id.starts_with("notesMaster/")
        || id.starts_with("handout/")
    {
        (
            None,
            Some(super::super::master::parse_placeholder_id(&id, i)?),
            None,
        )
    } else if connector::looks_like_cxn_id(&id) {
        (None, None, Some(connector::parse_cxn_id(&id, i)?))
    } else {
        (
            Some(super::super::shape::parse_shape_id(&id, i)?),
            None,
            None,
        )
    };
    if !obj.contains_key("color") {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setShapeLine".into(),
                field: "color".into(),
            },
            format!("ops[{i}].color"),
        ));
    }
    let color = parse_color_field(obj, i, true, "setShapeLine")?;
    let width = parse_emu_field(obj.get("width"), i, "width")?;
    if color.is_none() && width.is_some() {
        return Err(Error::with_path(
            ErrorKind::UnknownField {
                field: "width".into(),
            },
            format!("ops[{i}].width"),
        ));
    }
    Ok(SetShapeLineOp {
        id,
        target,
        placeholder,
        cxn,
        color,
        width,
    })
}

pub(super) fn parse_set_shape_geom(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<SetShapeGeomOp, Error> {
    deny_keys(obj, i, &["op", "id", "paths"])?;
    let id = require_id(obj, i)?;
    let (target, placeholder) = parse_slide_or_master_shape(&id, i)?;
    let paths = match obj.get("paths") {
        Some(v) => parse_geom_paths(v, i, "paths")?,
        None => {
            return Err(Error::with_path(
                ErrorKind::OpRequiresField {
                    op: "setShapeGeom".into(),
                    field: "paths".into(),
                },
                format!("ops[{i}].paths"),
            ));
        }
    };
    Ok(SetShapeGeomOp {
        id,
        target,
        placeholder,
        paths,
    })
}

pub(super) fn parse_set_connector(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<SetConnectorOp, Error> {
    deny_keys(obj, i, &["op", "id", "start", "end"])?;
    let id = require_id(obj, i)?;
    let target = connector::parse_cxn_id(&id, i)?;
    let start = parse_opt_cxn_bind(obj, i, "start")?;
    let end = parse_opt_cxn_bind(obj, i, "end")?;
    if start.is_none() && end.is_none() {
        return Err(Error::with_path(
            ErrorKind::NeedConnectorBind,
            format!("ops[{i}]"),
        ));
    }
    Ok(SetConnectorOp {
        id,
        target,
        start,
        end,
    })
}

pub(super) fn parse_geom_object(
    v: &Value,
    i: usize,
    field: &str,
) -> Result<Vec<super::super::geom::Path>, Error> {
    parse_geom_object_at(v, &format!("ops[{i}].{field}"), "insertShape")
}

pub(crate) fn parse_geom_object_at(
    v: &Value,
    prefix: &str,
    op: &str,
) -> Result<Vec<super::super::geom::Path>, Error> {
    let obj = match v {
        Value::Object(m) => m,
        _ => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "geom".into(),
                    expected: TypeName::Object,
                },
                prefix.to_string(),
            ));
        }
    };
    deny_keys_at(obj, prefix, &["paths"])?;
    match obj.get("paths") {
        Some(v) => parse_geom_paths_at(v, &format!("{prefix}.paths")),
        None => Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: op.into(),
                field: "geom.paths".into(),
            },
            format!("{prefix}.paths"),
        )),
    }
}

pub(super) fn parse_geom_paths(
    v: &Value,
    i: usize,
    field: &str,
) -> Result<Vec<super::super::geom::Path>, Error> {
    parse_geom_paths_at(v, &format!("ops[{i}].{field}"))
}

pub(super) fn parse_geom_paths_at(
    v: &Value,
    prefix: &str,
) -> Result<Vec<super::super::geom::Path>, Error> {
    let arr = match v {
        Value::Array(a) => a,
        _ => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "paths".into(),
                    expected: TypeName::Array,
                },
                prefix.to_string(),
            ));
        }
    };
    if arr.is_empty() {
        return Err(Error::with_path(
            ErrorKind::EmptyArray {
                field: "paths".into(),
            },
            prefix.to_string(),
        ));
    }
    let mut out = Vec::with_capacity(arr.len());
    for (j, item) in arr.iter().enumerate() {
        out.push(parse_geom_path_at(item, &format!("{prefix}[{j}]"))?);
    }
    Ok(out)
}

pub(super) fn parse_geom_path_at(
    v: &Value,
    prefix: &str,
) -> Result<super::super::geom::Path, Error> {
    let obj = match v {
        Value::Object(m) => m,
        _ => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "paths".into(),
                    expected: TypeName::Object,
                },
                prefix.to_string(),
            ));
        }
    };
    deny_keys_at(obj, prefix, &["w", "h", "ops"])?;
    let w = parse_emu_at(obj.get("w"), &format!("{prefix}.w"), "w")?.ok_or_else(|| {
        Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setShapeGeom".into(),
                field: "w".into(),
            },
            format!("{prefix}.w"),
        )
    })?;
    let h = parse_emu_at(obj.get("h"), &format!("{prefix}.h"), "h")?.ok_or_else(|| {
        Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setShapeGeom".into(),
                field: "h".into(),
            },
            format!("{prefix}.h"),
        )
    })?;
    let ops_v = obj.get("ops").ok_or_else(|| {
        Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setShapeGeom".into(),
                field: "ops".into(),
            },
            format!("{prefix}.ops"),
        )
    })?;
    let ops_arr = match ops_v {
        Value::Array(a) => a,
        _ => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "ops".into(),
                    expected: TypeName::Array,
                },
                format!("{prefix}.ops"),
            ));
        }
    };
    if ops_arr.is_empty() {
        return Err(Error::with_path(
            ErrorKind::EmptyArray {
                field: "ops".into(),
            },
            format!("{prefix}.ops"),
        ));
    }
    let mut ops = Vec::with_capacity(ops_arr.len());
    for (j, item) in ops_arr.iter().enumerate() {
        ops.push(parse_geom_op_at(item, &format!("{prefix}.ops[{j}]"))?);
    }
    Ok(super::super::geom::Path { w, h, ops })
}

pub(super) fn parse_geom_op_at(v: &Value, prefix: &str) -> Result<super::super::geom::Op, Error> {
    let obj = match v {
        Value::Object(m) => m,
        _ => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "ops".into(),
                    expected: TypeName::Object,
                },
                prefix.to_string(),
            ));
        }
    };
    let ty = match obj.get("type") {
        Some(Value::String(s)) => s.as_str(),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "type".into(),
                    expected: TypeName::String,
                },
                format!("{prefix}.type"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::OpRequiresField {
                    op: "setShapeGeom".into(),
                    field: "type".into(),
                },
                format!("{prefix}.type"),
            ));
        }
    };
    match ty {
        "moveTo" | "lnTo" => {
            deny_keys_at(obj, prefix, &["type", "x", "y"])?;
            let x = parse_off_at(obj.get("x"), &format!("{prefix}.x"), "x")?.ok_or_else(|| {
                Error::with_path(
                    ErrorKind::OpRequiresField {
                        op: "setShapeGeom".into(),
                        field: "x".into(),
                    },
                    format!("{prefix}.x"),
                )
            })?;
            let y = parse_off_at(obj.get("y"), &format!("{prefix}.y"), "y")?.ok_or_else(|| {
                Error::with_path(
                    ErrorKind::OpRequiresField {
                        op: "setShapeGeom".into(),
                        field: "y".into(),
                    },
                    format!("{prefix}.y"),
                )
            })?;
            if ty == "moveTo" {
                Ok(super::super::geom::Op::MoveTo { x, y })
            } else {
                Ok(super::super::geom::Op::LnTo { x, y })
            }
        }
        "close" => {
            deny_keys_at(obj, prefix, &["type"])?;
            Ok(super::super::geom::Op::Close)
        }
        _ => Err(Error::with_path(
            ErrorKind::InvalidGeomOp,
            format!("{prefix}.type"),
        )),
    }
}

pub(super) fn parse_opt_cxn_bind(
    obj: &Map<String, Value>,
    i: usize,
    field: &str,
) -> Result<Option<super::super::connector::CxnBind>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(v) => Ok(Some(parse_cxn_bind(v, i, field)?)),
    }
}

pub(super) fn parse_cxn_bind(
    v: &Value,
    i: usize,
    field: &str,
) -> Result<super::super::connector::CxnBind, Error> {
    let prefix = format!("ops[{i}].{field}");
    let obj = match v {
        Value::Object(m) => m,
        _ => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: field.into(),
                    expected: TypeName::Object,
                },
                prefix,
            ));
        }
    };
    deny_keys_at(obj, &prefix, &["id", "site"])?;
    let id = obj
        .get("id")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .ok_or_else(|| {
            Error::with_path(
                ErrorKind::MissingField { field: "id".into() },
                format!("{prefix}.id"),
            )
        })?;
    let site = parse_emu_at(obj.get("site"), &format!("{prefix}.site"), "site")?.unwrap_or(1);
    let site = u32::try_from(site).map_err(|_| {
        Error::with_path(
            ErrorKind::MinValue {
                field: "site".into(),
                min: 1,
            },
            format!("{prefix}.site"),
        )
    })?;
    if connector::looks_like_cxn_id(&id) {
        let t = connector::parse_cxn_id(&id, i)?;
        Ok(super::super::connector::CxnBind {
            id,
            slide: t.slide,
            on_notes: t.on_notes,
            master: t.master,
            key: t.key,
            is_cxn: true,
            site,
        })
    } else {
        let t = super::super::shape::parse_shape_id(&id, i)?;
        Ok(super::super::connector::CxnBind {
            id,
            slide: t.slide,
            on_notes: t.on_notes,
            master: t.master,
            key: t.key,
            is_cxn: false,
            site,
        })
    }
}

pub(super) fn parse_shape_pos_host(id: &str, i: usize) -> Result<ShapePosHost, Error> {
    if id.starts_with("master:") || id.starts_with("notesMaster/") || id.starts_with("handout/") {
        Ok((
            None,
            Some(super::super::master::parse_placeholder_id(id, i)?),
            None,
        ))
    } else if connector::looks_like_cxn_id(id) {
        Ok((None, None, Some(connector::parse_cxn_id(id, i)?)))
    } else {
        Ok((
            Some(super::super::shape::parse_shape_id(id, i)?),
            None,
            None,
        ))
    }
}

pub(super) fn parse_slide_or_master_shape(
    id: &str,
    i: usize,
) -> Result<
    (
        Option<super::super::shape::ShapeTarget>,
        Option<super::super::master::PlaceholderId>,
    ),
    Error,
> {
    if id.starts_with("master:") || id.starts_with("notesMaster/") || id.starts_with("handout/") {
        Ok((
            None,
            Some(super::super::master::parse_placeholder_id(id, i)?),
        ))
    } else {
        Ok((Some(super::super::shape::parse_shape_id(id, i)?), None))
    }
}

pub(super) fn parse_insert_shape_host(id: &str, i: usize) -> Result<InsertShapeHost, Error> {
    if id.starts_with("master:") {
        if id.contains("/layout:") {
            let (master, layout) = super::super::master::parse_layout_ref(id, i)?;
            return Ok(InsertShapeHost::Master {
                master,
                layout: Some(layout),
            });
        }
        let master = super::super::master::parse_master_only_id(
            id,
            i,
            crate::error::IdReason::InsertShapeHost,
        )?;
        return Ok(InsertShapeHost::Master {
            master,
            layout: None,
        });
    }
    let (slide, on_notes) =
        picture::parse_insert_slide_or_notes(id, i, crate::error::IdReason::InsertShapeHost)?;
    Ok(InsertShapeHost::Slide { slide, on_notes })
}

pub(crate) fn parse_create_connector(v: &Value, prefix: &str) -> Result<CreateConnector, Error> {
    let obj = v.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "connectors".into(),
                expected: TypeName::Object,
            },
            prefix.to_string(),
        )
    })?;
    deny_keys_at(obj, prefix, &["prst", "x", "y", "cx", "cy", "start", "end"])?;
    let prst = match obj.get("prst") {
        None => "straightConnector1".to_string(),
        Some(Value::String(s)) => {
            if crate::drawing_geom::is_writable_connector_prst(s) {
                s.clone()
            } else {
                return Err(Error::with_path(
                    ErrorKind::InvalidConnectorPrst,
                    format!("{prefix}.prst"),
                ));
            }
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "prst".into(),
                    expected: TypeName::String,
                },
                format!("{prefix}.prst"),
            ));
        }
    };
    let x = parse_off_at(obj.get("x"), &format!("{prefix}.x"), "x")?;
    let y = parse_off_at(obj.get("y"), &format!("{prefix}.y"), "y")?;
    if x.is_some() != y.is_some() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "create".into(),
                field: "x and y together".into(),
            },
            prefix.to_string(),
        ));
    }
    let cx = parse_emu_at(obj.get("cx"), &format!("{prefix}.cx"), "cx")?;
    let cy = parse_emu_at(obj.get("cy"), &format!("{prefix}.cy"), "cy")?;
    if cx.is_some() != cy.is_some() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "create".into(),
                field: "cx and cy together".into(),
            },
            prefix.to_string(),
        ));
    }
    Ok(CreateConnector {
        prst,
        x,
        y,
        cx,
        cy,
        start: parse_create_cxn_bind(obj.get("start"), &format!("{prefix}.start"))?,
        end: parse_create_cxn_bind(obj.get("end"), &format!("{prefix}.end"))?,
    })
}

pub(super) fn parse_create_cxn_bind(
    v: Option<&Value>,
    prefix: &str,
) -> Result<Option<CreateCxnBind>, Error> {
    let Some(v) = v else {
        return Ok(None);
    };
    let obj = v.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "start".into(),
                expected: TypeName::Object,
            },
            prefix.to_string(),
        )
    })?;
    deny_keys_at(obj, prefix, &["shape", "site"])?;
    let shape = parse_create_index(obj.get("shape"), &format!("{prefix}.shape"), "shape")?;
    let site = match obj.get("site") {
        None => 1,
        Some(v) => {
            let n = parse_create_index(Some(v), &format!("{prefix}.site"), "site")?;
            u32::try_from(n).map_err(|_| {
                Error::with_path(
                    ErrorKind::MinValue {
                        field: "site".into(),
                        min: 1,
                    },
                    format!("{prefix}.site"),
                )
            })?
        }
    };
    Ok(Some(CreateCxnBind { shape, site }))
}

const ALPHA_MAX: i64 = 100_000;

pub(super) fn parse_sp_tree_place(
    obj: &Map<String, Value>,
    path: &str,
) -> Result<SpTreePlace, Error> {
    let has_index = obj.contains_key("index");
    let has_before = obj.contains_key("before");
    let has_after = obj.contains_key("after");
    if has_before && has_after {
        return Err(Error::with_path(
            ErrorKind::MutuallyExclusive {
                a: "before".into(),
                b: "after".into(),
            },
            path.to_string(),
        ));
    }
    if has_index && (has_before || has_after) {
        return Err(Error::with_path(
            ErrorKind::MutuallyExclusive {
                a: "index".into(),
                b: "before/after".into(),
            },
            path.to_string(),
        ));
    }
    if has_index {
        let n = parse_layer_index(obj.get("index").expect("已检查"), &format!("{path}.index"))?;
        return Ok(SpTreePlace::Index(n));
    }
    if has_before {
        let id = parse_anchor_id(
            obj.get("before").expect("已检查"),
            &format!("{path}.before"),
        )?;
        return Ok(SpTreePlace::Before(id));
    }
    if has_after {
        let id = parse_anchor_id(obj.get("after").expect("已检查"), &format!("{path}.after"))?;
        return Ok(SpTreePlace::After(id));
    }
    Ok(SpTreePlace::End)
}

pub(super) fn parse_layer_index(v: &Value, path: &str) -> Result<usize, Error> {
    let Value::Number(n) = v else {
        return Err(Error::with_path(
            ErrorKind::WrongType {
                field: "index".into(),
                expected: TypeName::Integer,
            },
            path.to_string(),
        ));
    };
    if let Some(i) = n.as_i64() {
        if i < 0 {
            return Err(Error::with_path(
                ErrorKind::MinValue {
                    field: "index".into(),
                    min: 0,
                },
                path.to_string(),
            ));
        }
        return Ok(i as usize);
    }
    if let Some(u) = n.as_u64() {
        if u > usize::MAX as u64 {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "index".into(),
                    expected: TypeName::Integer,
                },
                path.to_string(),
            ));
        }
        return Ok(u as usize);
    }
    Err(Error::with_path(
        ErrorKind::WrongType {
            field: "index".into(),
            expected: TypeName::Integer,
        },
        path.to_string(),
    ))
}

fn parse_anchor_id(v: &Value, path: &str) -> Result<String, Error> {
    match v {
        Value::String(s) if !s.is_empty() => Ok(s.clone()),
        _ => Err(Error::with_path(
            ErrorKind::WrongType {
                field: path.rsplit('.').next().unwrap_or("id").into(),
                expected: TypeName::NonEmptyString,
            },
            path.to_string(),
        )),
    }
}

pub(super) fn parse_alpha_at(v: Option<&Value>, path: &str) -> Result<Option<u32>, Error> {
    let Some(v) = v else {
        return Ok(None);
    };
    let Value::Number(n) = v else {
        return Err(Error::with_path(
            ErrorKind::WrongType {
                field: "alpha".into(),
                expected: TypeName::Integer,
            },
            path.to_string(),
        ));
    };
    let Some(i) = n
        .as_i64()
        .or_else(|| n.as_u64().and_then(|u| i64::try_from(u).ok()))
    else {
        return Err(Error::with_path(
            ErrorKind::WrongType {
                field: "alpha".into(),
                expected: TypeName::Integer,
            },
            path.to_string(),
        ));
    };
    if i < 0 {
        return Err(Error::with_path(
            ErrorKind::MinValue {
                field: "alpha".into(),
                min: 0,
            },
            path.to_string(),
        ));
    }
    if i > ALPHA_MAX {
        return Err(Error::with_path(
            ErrorKind::CreateIndexOutOfRange {
                field: "alpha".into(),
                max: ALPHA_MAX,
            },
            path.to_string(),
        ));
    }
    Ok(Some(i as u32))
}

fn paint_from_color(
    color: Option<String>,
    alpha: Option<u32>,
    alpha_path: &str,
) -> Result<ShapePaint, Error> {
    match color {
        None => {
            if alpha.is_some() {
                return Err(Error::with_path(
                    ErrorKind::MutuallyExclusive {
                        a: "alpha".into(),
                        b: "color".into(),
                    },
                    alpha_path.to_string(),
                ));
            }
            Ok(ShapePaint::None)
        }
        Some(c) => Ok(ShapePaint::Solid { color: c, alpha }),
    }
}

/// `fill` 可以是六位色、`null`，或 `{type, ...}`。`outer_alpha` 来自与 `fill` 同级的 `alpha`。
pub(super) fn parse_paint_value(
    v: &Value,
    path: &str,
    outer_alpha: Option<u32>,
) -> Result<ShapePaint, Error> {
    match v {
        Value::Null => paint_from_color(None, outer_alpha, path),
        Value::String(s) => {
            let color = parse_rgb_str(s, path)?;
            Ok(ShapePaint::Solid {
                color,
                alpha: outer_alpha,
            })
        }
        Value::Object(obj) => parse_paint_object(obj, path, outer_alpha),
        _ => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "fill".into(),
                expected: TypeName::Object,
            },
            path.to_string(),
        )),
    }
}

fn parse_paint_object(
    obj: &Map<String, Value>,
    path: &str,
    outer_alpha: Option<u32>,
) -> Result<ShapePaint, Error> {
    deny_keys_at(
        obj,
        path,
        &["type", "color", "alpha", "stops", "ang", "file"],
    )?;
    let kind = match obj.get("type") {
        Some(Value::String(s)) => s.as_str(),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "type".into(),
                    expected: TypeName::String,
                },
                format!("{path}.type"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "type".into(),
                },
                format!("{path}.type"),
            ));
        }
    };
    let inner = parse_alpha_at(obj.get("alpha"), &format!("{path}.alpha"))?;
    if inner.is_some() && outer_alpha.is_some() {
        return Err(Error::with_path(
            ErrorKind::MutuallyExclusive {
                a: "alpha".into(),
                b: "fill.alpha".into(),
            },
            format!("{path}.alpha"),
        ));
    }
    let alpha = inner.or(outer_alpha);
    match kind {
        "solid" => {
            deny_unused(obj, path, &["type", "color", "alpha"])?;
            let color = require_rgb(obj.get("color"), &format!("{path}.color"))?;
            Ok(ShapePaint::Solid { color, alpha })
        }
        "grad" => {
            deny_unused(obj, path, &["type", "stops", "ang", "alpha"])?;
            let stops = parse_grad_stops(obj.get("stops"), &format!("{path}.stops"))?;
            let ang = parse_ang_at(obj.get("ang"), &format!("{path}.ang"))?;
            Ok(ShapePaint::Grad { ang, stops, alpha })
        }
        "blip" => {
            if alpha.is_some() {
                return Err(Error::with_path(
                    ErrorKind::MutuallyExclusive {
                        a: "alpha".into(),
                        b: "blip".into(),
                    },
                    format!("{path}.alpha"),
                ));
            }
            deny_unused(obj, path, &["type", "file"])?;
            let file = match obj.get("file") {
                Some(Value::String(s)) if !s.is_empty() => s.clone(),
                Some(Value::String(_)) | None => {
                    return Err(Error::with_path(
                        ErrorKind::MissingField {
                            field: "file".into(),
                        },
                        format!("{path}.file"),
                    ));
                }
                Some(_) => {
                    return Err(Error::with_path(
                        ErrorKind::WrongType {
                            field: "file".into(),
                            expected: TypeName::NonEmptyString,
                        },
                        format!("{path}.file"),
                    ));
                }
            };
            Ok(ShapePaint::Blip { file })
        }
        _ => Err(Error::with_path(
            ErrorKind::InvalidShapeFillType,
            format!("{path}.type"),
        )),
    }
}

fn deny_unused(obj: &Map<String, Value>, path: &str, allow: &[&str]) -> Result<(), Error> {
    for k in obj.keys() {
        if !allow.contains(&k.as_str()) {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("{path}.{k}"),
            ));
        }
    }
    Ok(())
}

fn parse_grad_stops(v: Option<&Value>, path: &str) -> Result<Vec<(i64, String)>, Error> {
    let Some(Value::Array(items)) = v else {
        return Err(Error::with_path(
            if v.is_none() {
                ErrorKind::NeedGradientStops
            } else {
                ErrorKind::WrongType {
                    field: "stops".into(),
                    expected: TypeName::Array,
                }
            },
            path.to_string(),
        ));
    };
    if items.len() < 2 {
        return Err(Error::with_path(
            ErrorKind::NeedGradientStops,
            path.to_string(),
        ));
    }
    let mut out = Vec::new();
    let mut seen = Vec::new();
    for (i, item) in items.iter().enumerate() {
        let prefix = format!("{path}[{i}]");
        let obj = item.as_object().ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: "stops".into(),
                    expected: TypeName::Object,
                },
                prefix.clone(),
            )
        })?;
        deny_keys_at(obj, &prefix, &["pos", "color"])?;
        let pos = match obj.get("pos") {
            Some(Value::Number(n)) => n
                .as_i64()
                .or_else(|| n.as_u64().and_then(|u| i64::try_from(u).ok()))
                .ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::WrongType {
                            field: "pos".into(),
                            expected: TypeName::Integer,
                        },
                        format!("{prefix}.pos"),
                    )
                })?,
            Some(_) => {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: "pos".into(),
                        expected: TypeName::Integer,
                    },
                    format!("{prefix}.pos"),
                ));
            }
            None => {
                return Err(Error::with_path(
                    ErrorKind::MissingField {
                        field: "pos".into(),
                    },
                    format!("{prefix}.pos"),
                ));
            }
        };
        if pos < 0 {
            return Err(Error::with_path(
                ErrorKind::MinValue {
                    field: "pos".into(),
                    min: 0,
                },
                format!("{prefix}.pos"),
            ));
        }
        if pos > crate::drawing_text::POS_MAX {
            return Err(Error::with_path(
                ErrorKind::CreateIndexOutOfRange {
                    field: "pos".into(),
                    max: crate::drawing_text::POS_MAX,
                },
                format!("{prefix}.pos"),
            ));
        }
        if seen.contains(&pos) {
            return Err(Error::with_path(
                ErrorKind::DuplicateGradientPos,
                format!("{prefix}.pos"),
            ));
        }
        seen.push(pos);
        let color = require_rgb(obj.get("color"), &format!("{prefix}.color"))?;
        out.push((pos, color));
    }
    Ok(out)
}

fn parse_ang_at(v: Option<&Value>, path: &str) -> Result<i64, Error> {
    let Some(v) = v else {
        return Ok(crate::drawing_text::ANG_DEFAULT);
    };
    let Value::Number(n) = v else {
        return Err(Error::with_path(
            ErrorKind::WrongType {
                field: "ang".into(),
                expected: TypeName::Integer,
            },
            path.to_string(),
        ));
    };
    let Some(i) = n
        .as_i64()
        .or_else(|| n.as_u64().and_then(|u| i64::try_from(u).ok()))
    else {
        return Err(Error::with_path(
            ErrorKind::WrongType {
                field: "ang".into(),
                expected: TypeName::Integer,
            },
            path.to_string(),
        ));
    };
    if i < 0 {
        return Err(Error::with_path(
            ErrorKind::MinValue {
                field: "ang".into(),
                min: 0,
            },
            path.to_string(),
        ));
    }
    if i > crate::drawing_text::ANG_MAX {
        return Err(Error::with_path(
            ErrorKind::CreateIndexOutOfRange {
                field: "ang".into(),
                max: crate::drawing_text::ANG_MAX,
            },
            path.to_string(),
        ));
    }
    Ok(i)
}

pub(super) fn parse_shape_line(v: &Value, path: &str) -> Result<ShapeLine, Error> {
    let obj = v.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "line".into(),
                expected: TypeName::Object,
            },
            path.to_string(),
        )
    })?;
    deny_keys_at(obj, path, &["color", "width"])?;
    if !obj.contains_key("color") {
        return Err(Error::with_path(
            ErrorKind::MissingField {
                field: "color".into(),
            },
            format!("{path}.color"),
        ));
    }
    let color = parse_rgb_value(obj.get("color"), &format!("{path}.color"), true)?;
    let width = parse_emu_at(obj.get("width"), &format!("{path}.width"), "width")?;
    Ok(ShapeLine { color, width })
}

fn parse_page_fill(v: &Value, path: &str) -> Result<PageFill, Error> {
    let obj = v.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "fill".into(),
                expected: TypeName::Object,
            },
            path.to_string(),
        )
    })?;
    deny_keys_at(obj, path, &["kind", "color", "file"])?;
    let kind = match obj.get("kind") {
        Some(Value::String(s)) => s.clone(),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "kind".into(),
                    expected: TypeName::String,
                },
                format!("{path}.kind"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "kind".into(),
                },
                format!("{path}.kind"),
            ));
        }
    };
    match kind.as_str() {
        "gradient" => {
            let color2 = match obj.get("color") {
                None => None,
                Some(v) => Some(
                    parse_rgb_value(Some(v), &format!("{path}.color"), false)?.expect("非空色"),
                ),
            };
            if obj.contains_key("file") {
                return Err(Error::with_path(
                    ErrorKind::UnknownField {
                        field: "file".into(),
                    },
                    format!("{path}.file"),
                ));
            }
            Ok(PageFill::Gradient { color2 })
        }
        "picture" => {
            if obj.contains_key("color") {
                return Err(Error::with_path(
                    ErrorKind::UnknownField {
                        field: "color".into(),
                    },
                    format!("{path}.color"),
                ));
            }
            let file = match obj.get("file") {
                Some(Value::String(s)) if !s.is_empty() => s.clone(),
                Some(Value::String(_)) | None => {
                    return Err(Error::with_path(
                        ErrorKind::MissingField {
                            field: "file".into(),
                        },
                        format!("{path}.file"),
                    ));
                }
                Some(_) => {
                    return Err(Error::with_path(
                        ErrorKind::WrongType {
                            field: "file".into(),
                            expected: TypeName::NonEmptyString,
                        },
                        format!("{path}.file"),
                    ));
                }
            };
            Ok(PageFill::Picture { file })
        }
        _ => Err(Error::with_path(
            ErrorKind::InvalidPageBackgroundFill,
            format!("{path}.kind"),
        )),
    }
}

fn require_rgb(v: Option<&Value>, path: &str) -> Result<String, Error> {
    match parse_rgb_value(v, path, false)? {
        Some(c) => Ok(c),
        None => Err(Error::with_path(
            ErrorKind::MissingField {
                field: "color".into(),
            },
            path.to_string(),
        )),
    }
}

/// `allow_null` 为真时，JSON `null` 得到 `None`。缺字段也是 `None`。
fn parse_rgb_value(
    v: Option<&Value>,
    path: &str,
    allow_null: bool,
) -> Result<Option<String>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Null) if allow_null => Ok(None),
        Some(Value::String(s)) => Ok(Some(parse_rgb_str(s, path)?)),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "color".into(),
                expected: TypeName::String,
            },
            path.to_string(),
        )),
    }
}

fn parse_rgb_str(s: &str, path: &str) -> Result<String, Error> {
    if s.len() == 6 && s.chars().all(|c| c.is_ascii_hexdigit()) {
        Ok(s.to_ascii_uppercase())
    } else {
        Err(Error::with_path(
            ErrorKind::InvalidHexColor,
            path.to_string(),
        ))
    }
}
