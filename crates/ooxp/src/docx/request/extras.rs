//! docx request：extras。

use std::collections::HashSet;

use serde_json::Value;

use crate::error::{Error, ErrorKind, TypeName};

use super::super::charts::{self, ChartType, NewSeries};
use super::super::para::{self};
use super::super::picture::{self, PicKind, WrapKind, WrapPos};

use super::common::*;
use super::picture::*;

pub(super) fn parse_insert_chart(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<charts::InsertChartOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op",
            "id",
            "type",
            "ofPieType",
            "stockKind",
            "surfaceKind",
            "layoutId",
            "title",
            "series",
            "kind",
            "wrap",
            "cx",
            "cy",
        ],
    )?;
    let id = require_id(obj, i)?;
    let para = charts::parse_insert_para_id(&id, i)?;
    let chart_type = parse_chart_type_field(obj.get("type"), i, true)?;
    let title = match obj.get("title") {
        None | Some(Value::Null) => None,
        Some(Value::String(s)) => Some(s.clone()),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "title".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].title"),
            ));
        }
    };
    let of_pie_type =
        crate::chart::parse_of_pie_type(obj.get("ofPieType"), &format!("ops[{i}].ofPieType"))?;
    crate::chart::reject_of_pie_on_other(
        chart_type,
        of_pie_type.as_deref(),
        &format!("ops[{i}].ofPieType"),
    )?;
    let series = parse_insert_series(obj.get("series"), i, chart_type)?;
    let extra =
        crate::chart::parse_chart_extra(obj, chart_type, series.len(), &format!("ops[{i}]"), true)?;
    crate::chart::validate_insert_series_len(
        chart_type,
        series.len(),
        &extra,
        &format!("ops[{i}].series"),
    )?;
    if chart_type == ChartType::Combo {
        let kinds: HashSet<&str> = series
            .iter()
            .filter_map(|s| s.series_type.map(ChartType::as_str))
            .collect();
        if series.iter().any(|s| s.series_type.is_none()) || kinds.len() < 2 {
            return Err(Error::with_path(
                ErrorKind::InvalidComboSeriesType,
                format!("ops[{i}].series"),
            ));
        }
    } else if series.iter().any(|s| s.series_type.is_some()) {
        return Err(Error::with_path(
            ErrorKind::InvalidComboSeriesType,
            format!("ops[{i}].series"),
        ));
    }
    if chart_type != ChartType::Bubble && series.iter().any(|s| s.sizes.is_some()) {
        return Err(Error::with_path(
            ErrorKind::SizesOnNonBubble,
            format!("ops[{i}].series"),
        ));
    }
    let kind = match obj.get("kind") {
        None => PicKind::Anchor,
        Some(Value::String(s)) => picture::parse_pic_kind(s)
            .ok_or_else(|| Error::with_path(ErrorKind::InvalidPicKind, format!("ops[{i}].kind")))?,
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
    let wrap = match obj.get("wrap") {
        None | Some(Value::Null) => {
            if kind == PicKind::Anchor {
                Some(WrapKind::Square)
            } else {
                None
            }
        }
        Some(Value::String(s)) => Some(picture::parse_wrap(s).ok_or_else(|| {
            Error::with_path(ErrorKind::InvalidPicWrap, format!("ops[{i}].wrap"))
        })?),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "wrap".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].wrap"),
            ));
        }
    };
    if kind == PicKind::Inline && wrap.is_some() {
        return Err(Error::with_path(
            ErrorKind::InvalidPicWrap,
            format!("ops[{i}].wrap"),
        ));
    }
    let cx = parse_emu_field(obj.get("cx"), i, "cx")?;
    let cy = parse_emu_field(obj.get("cy"), i, "cy")?;
    if cx.is_some() != cy.is_some() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "insertChart".into(),
                field: "cx and cy together".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(charts::InsertChartOp {
        id,
        para,
        chart_type,
        of_pie_type,
        extra,
        title,
        series,
        kind,
        wrap,
        pos: WrapPos::default(),
        cx: cx.unwrap_or(charts::default_cx()),
        cy: cy.unwrap_or(charts::default_cy()),
    })
}

pub(super) fn parse_insert_series(
    v: Option<&Value>,
    i: usize,
    _chart_type: ChartType,
) -> Result<Vec<NewSeries>, Error> {
    let arr = match v {
        Some(Value::Array(a)) if !a.is_empty() => a,
        Some(Value::Array(_)) | None => {
            return Err(Error::with_path(
                ErrorKind::OpRequiresField {
                    op: "insertChart".into(),
                    field: "series".into(),
                },
                format!("ops[{i}].series"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "series".into(),
                    expected: TypeName::Array,
                },
                format!("ops[{i}].series"),
            ));
        }
    };
    let mut out = Vec::new();
    for (j, item) in arr.iter().enumerate() {
        let obj = item.as_object().ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: format!("series[{j}]"),
                    expected: TypeName::Object,
                },
                format!("ops[{i}].series[{j}]"),
            )
        })?;
        deny_keys(obj, i, &["name", "categories", "values", "sizes", "type"])?;
        let name = match obj.get("name") {
            None | Some(Value::Null) => None,
            Some(Value::String(s)) => Some(s.clone()),
            Some(_) => {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: format!("series[{j}].name"),
                        expected: TypeName::String,
                    },
                    format!("ops[{i}].series[{j}].name"),
                ));
            }
        };
        let categories =
            parse_datum_list(obj.get("categories"), i, &format!("series[{j}].categories"))?;
        let values = parse_datum_list(obj.get("values"), i, &format!("series[{j}].values"))?
            .filter(|v| !v.is_empty())
            .ok_or_else(|| {
                Error::with_path(
                    ErrorKind::OpRequiresField {
                        op: "insertChart".into(),
                        field: "series[].values".into(),
                    },
                    format!("ops[{i}].series[{j}].values"),
                )
            })?;
        let sizes = parse_datum_list(obj.get("sizes"), i, &format!("series[{j}].sizes"))?;
        let series_type =
            parse_optional_combo_series_type(obj.get("type"), i, &format!("series[{j}].type"))?;
        out.push(NewSeries {
            name,
            categories,
            values,
            sizes,
            series_type,
        });
    }
    Ok(out)
}

pub(super) fn parse_delete_chart(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<charts::DeleteChartOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let target = charts::parse_chart_id(&id, i)?;
    Ok(charts::DeleteChartOp { id, target })
}

pub(super) fn parse_insert_ole(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<super::super::ole::InsertOleOp, Error> {
    deny_keys(obj, i, &["op", "id", "file"])?;
    let id = require_id(obj, i)?;
    let para = super::super::ole::parse_insert_para_id(&id, i)?;
    let file = require_file(obj, i, "insertOle")?;
    Ok(super::super::ole::InsertOleOp { id, para, file })
}

pub(super) fn parse_delete_ole(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<super::super::ole::DeleteOleOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let target = super::super::ole::parse_ole_id(&id, i)?;
    Ok(super::super::ole::DeleteOleOp { id, target })
}

pub(super) fn parse_set_ole_size(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<super::super::ole::SetOleSizeOp, Error> {
    deny_keys(obj, i, &["op", "id", "cx", "cy"])?;
    let id = require_id(obj, i)?;
    let target = super::super::ole::parse_ole_id(&id, i)?;
    let cx = parse_emu_field(obj.get("cx"), i, "cx")?;
    let cy = parse_emu_field(obj.get("cy"), i, "cy")?;
    if cx.is_none() && cy.is_none() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setOleSize".into(),
                field: "cx or cy".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(super::super::ole::SetOleSizeOp { id, target, cx, cy })
}

pub(super) fn parse_insert_ink_canvas(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<super::super::ink::InsertInkCanvasOp, Error> {
    deny_keys(obj, i, &["op", "id", "file", "cx", "cy"])?;
    let id = require_id(obj, i)?;
    let host = para::parse_host_para_id_at(&id, &format!("ops[{i}].id"))?;
    let file = require_file(obj, i, "insertInkCanvas")?;
    let cx = parse_emu_field(obj.get("cx"), i, "cx")?;
    let cy = parse_emu_field(obj.get("cy"), i, "cy")?;
    if cx.is_some() != cy.is_some() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "insertInkCanvas".into(),
                field: "cx and cy together".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(super::super::ink::InsertInkCanvasOp {
        id,
        host,
        file,
        cx,
        cy,
    })
}

pub(super) fn parse_replace_ink_canvas(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<super::super::ink::ReplaceInkCanvasOp, Error> {
    deny_keys(obj, i, &["op", "id", "file"])?;
    let id = require_id(obj, i)?;
    let target = super::super::ink::parse_ink_id(&id, i)?;
    let file = require_file(obj, i, "replaceInkCanvas")?;
    Ok(super::super::ink::ReplaceInkCanvasOp { id, target, file })
}

pub(super) fn parse_delete_ink_canvas(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<super::super::ink::DeleteInkCanvasOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let target = super::super::ink::parse_ink_id(&id, i)?;
    Ok(super::super::ink::DeleteInkCanvasOp { id, target })
}

pub(super) fn parse_set_ink_canvas_size(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<super::super::ink::SetInkCanvasSizeOp, Error> {
    deny_keys(obj, i, &["op", "id", "cx", "cy"])?;
    let id = require_id(obj, i)?;
    let target = super::super::ink::parse_ink_id(&id, i)?;
    let cx = parse_emu_field(obj.get("cx"), i, "cx")?;
    let cy = parse_emu_field(obj.get("cy"), i, "cy")?;
    if cx.is_none() && cy.is_none() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setInkCanvasSize".into(),
                field: "cx or cy".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(super::super::ink::SetInkCanvasSizeOp { id, target, cx, cy })
}

pub(super) fn parse_set_ink_canvas_wrap(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<super::super::ink::SetInkCanvasWrapOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op", "id", "kind", "wrap", "alignH", "alignV", "offsetX", "offsetY",
        ],
    )?;
    let id = require_id(obj, i)?;
    let target = super::super::ink::parse_ink_id(&id, i)?;
    let kind = match obj.get("kind") {
        Some(Value::String(s)) => picture::parse_pic_kind(s)
            .ok_or_else(|| Error::with_path(ErrorKind::InvalidPicKind, format!("ops[{i}].kind")))?,
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
                    op: "setInkCanvasWrap".into(),
                    field: "kind".into(),
                },
                format!("ops[{i}].kind"),
            ));
        }
    };
    let wrap = match obj.get("wrap") {
        None | Some(Value::Null) => None,
        Some(Value::String(s)) => Some(picture::parse_wrap(s).ok_or_else(|| {
            Error::with_path(ErrorKind::InvalidPicWrap, format!("ops[{i}].wrap"))
        })?),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "wrap".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].wrap"),
            ));
        }
    };
    if kind == PicKind::Inline && wrap.is_some() {
        return Err(Error::with_path(
            ErrorKind::InvalidPicWrap,
            format!("ops[{i}].wrap"),
        ));
    }
    if kind == PicKind::Anchor && wrap.is_none() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setInkCanvasWrap".into(),
                field: "wrap".into(),
            },
            format!("ops[{i}].wrap"),
        ));
    }
    let align_h = parse_align_h_field(obj.get("alignH"), i)?;
    let align_v = parse_align_v_field(obj.get("alignV"), i)?;
    let offset_x = parse_i64_field(obj.get("offsetX"), i, "offsetX")?;
    let offset_y = parse_i64_field(obj.get("offsetY"), i, "offsetY")?;
    if align_h.is_some() && offset_x.is_some() {
        return Err(Error::with_path(
            ErrorKind::MutuallyExclusive {
                a: "alignH".into(),
                b: "offsetX".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    if align_v.is_some() && offset_y.is_some() {
        return Err(Error::with_path(
            ErrorKind::MutuallyExclusive {
                a: "alignV".into(),
                b: "offsetY".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(super::super::ink::SetInkCanvasWrapOp {
        id,
        target,
        kind,
        wrap,
        pos: WrapPos {
            align_h,
            align_v,
            offset_x,
            offset_y,
        },
    })
}

pub(super) fn parse_insert_model3d(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<super::super::model3d::InsertModel3dOp, Error> {
    deny_keys(obj, i, &["op", "id", "file", "cx", "cy"])?;
    let id = require_id(obj, i)?;
    let host = para::parse_host_para_id_at(&id, &format!("ops[{i}].id"))?;
    let file = require_file(obj, i, "insertModel3d")?;
    let cx = parse_emu_field(obj.get("cx"), i, "cx")?;
    let cy = parse_emu_field(obj.get("cy"), i, "cy")?;
    if cx.is_some() != cy.is_some() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "insertModel3d".into(),
                field: "cx and cy together".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(super::super::model3d::InsertModel3dOp {
        id,
        host,
        file,
        cx,
        cy,
    })
}

pub(super) fn parse_replace_model3d(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<super::super::model3d::ReplaceModel3dOp, Error> {
    deny_keys(obj, i, &["op", "id", "file"])?;
    let id = require_id(obj, i)?;
    let target = super::super::model3d::parse_id(&id, i)?;
    let file = require_file(obj, i, "replaceModel3d")?;
    Ok(super::super::model3d::ReplaceModel3dOp { id, target, file })
}

pub(super) fn parse_delete_model3d(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<super::super::model3d::DeleteModel3dOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let target = super::super::model3d::parse_id(&id, i)?;
    Ok(super::super::model3d::DeleteModel3dOp { id, target })
}

pub(super) fn parse_set_model3d_size(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<super::super::model3d::SetModel3dSizeOp, Error> {
    deny_keys(obj, i, &["op", "id", "cx", "cy"])?;
    let id = require_id(obj, i)?;
    let target = super::super::model3d::parse_id(&id, i)?;
    let cx = parse_emu_field(obj.get("cx"), i, "cx")?;
    let cy = parse_emu_field(obj.get("cy"), i, "cy")?;
    if cx.is_none() && cy.is_none() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setModel3dSize".into(),
                field: "cx or cy".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(super::super::model3d::SetModel3dSizeOp { id, target, cx, cy })
}

pub(super) fn parse_set_model3d_wrap(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<super::super::model3d::SetModel3dWrapOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op", "id", "kind", "wrap", "alignH", "alignV", "offsetX", "offsetY",
        ],
    )?;
    let id = require_id(obj, i)?;
    let target = super::super::model3d::parse_id(&id, i)?;
    let kind = match obj.get("kind") {
        Some(Value::String(s)) => picture::parse_pic_kind(s)
            .ok_or_else(|| Error::with_path(ErrorKind::InvalidPicKind, format!("ops[{i}].kind")))?,
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
                    op: "setModel3dWrap".into(),
                    field: "kind".into(),
                },
                format!("ops[{i}].kind"),
            ));
        }
    };
    let wrap = match obj.get("wrap") {
        None | Some(Value::Null) => None,
        Some(Value::String(s)) => Some(picture::parse_wrap(s).ok_or_else(|| {
            Error::with_path(ErrorKind::InvalidPicWrap, format!("ops[{i}].wrap"))
        })?),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "wrap".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].wrap"),
            ));
        }
    };
    if kind == PicKind::Inline && wrap.is_some() {
        return Err(Error::with_path(
            ErrorKind::InvalidPicWrap,
            format!("ops[{i}].wrap"),
        ));
    }
    if kind == PicKind::Anchor && wrap.is_none() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setModel3dWrap".into(),
                field: "wrap".into(),
            },
            format!("ops[{i}].wrap"),
        ));
    }
    let align_h = parse_align_h_field(obj.get("alignH"), i)?;
    let align_v = parse_align_v_field(obj.get("alignV"), i)?;
    let offset_x = parse_i64_field(obj.get("offsetX"), i, "offsetX")?;
    let offset_y = parse_i64_field(obj.get("offsetY"), i, "offsetY")?;
    if align_h.is_some() && offset_x.is_some() {
        return Err(Error::with_path(
            ErrorKind::MutuallyExclusive {
                a: "alignH".into(),
                b: "offsetX".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    if align_v.is_some() && offset_y.is_some() {
        return Err(Error::with_path(
            ErrorKind::MutuallyExclusive {
                a: "alignV".into(),
                b: "offsetY".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(super::super::model3d::SetModel3dWrapOp {
        id,
        target,
        kind,
        wrap,
        pos: WrapPos {
            align_h,
            align_v,
            offset_x,
            offset_y,
        },
    })
}

pub(super) fn parse_set_model3d_camera(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<super::super::model3d::SetModel3dCameraOp, Error> {
    deny_keys(obj, i, &["op", "id", "camera", "rot", "lightRig"])?;
    let id = require_id(obj, i)?;
    let target = super::super::model3d::parse_id(&id, i)?;
    let patch = crate::model3d::parse_camera_patch(obj, i)?;
    Ok(super::super::model3d::SetModel3dCameraOp { id, target, patch })
}

pub(super) fn parse_set_chart_title(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<charts::SetChartTitleOp, Error> {
    deny_keys(obj, i, &["op", "id", "title"])?;
    let id = require_id(obj, i)?;
    let target = charts::parse_chart_id(&id, i)?;
    let title = match obj.get("title") {
        Some(Value::String(s)) => s.clone(),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "title".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].title"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::OpRequiresField {
                    op: "setChartTitle".into(),
                    field: "title".into(),
                },
                format!("ops[{i}].title"),
            ));
        }
    };
    Ok(charts::SetChartTitleOp { id, target, title })
}

pub(super) fn parse_set_chart_series(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<charts::SetChartSeriesOp, Error> {
    deny_keys(
        obj,
        i,
        &["op", "id", "name", "categories", "values", "sizes"],
    )?;
    let id = require_id(obj, i)?;
    let target = charts::parse_series_id(&id, i)?;
    let name = match obj.get("name") {
        None | Some(Value::Null) => None,
        Some(Value::String(s)) => Some(s.clone()),
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
    let categories = parse_datum_list(obj.get("categories"), i, "categories")?;
    let values = parse_datum_list(obj.get("values"), i, "values")?;
    let sizes = parse_datum_list(obj.get("sizes"), i, "sizes")?;
    if name.is_none() && categories.is_none() && values.is_none() && sizes.is_none() {
        return Err(Error::with_path(
            ErrorKind::NeedChartSeriesField,
            format!("ops[{i}]"),
        ));
    }
    Ok(charts::SetChartSeriesOp {
        id,
        target,
        name,
        categories,
        values,
        sizes,
    })
}

pub(super) fn parse_add_chart_series(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<charts::AddChartSeriesOp, Error> {
    deny_keys(
        obj,
        i,
        &["op", "id", "name", "categories", "values", "sizes", "type"],
    )?;
    let id = require_id(obj, i)?;
    let target = charts::parse_chart_id(&id, i)?;
    let name = match obj.get("name") {
        None | Some(Value::Null) => None,
        Some(Value::String(s)) => Some(s.clone()),
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
    let categories = parse_datum_list(obj.get("categories"), i, "categories")?;
    let values = parse_datum_list(obj.get("values"), i, "values")?
        .filter(|v| !v.is_empty())
        .ok_or_else(|| {
            Error::with_path(
                ErrorKind::OpRequiresField {
                    op: "addChartSeries".into(),
                    field: "values".into(),
                },
                format!("ops[{i}].values"),
            )
        })?;
    let sizes = parse_datum_list(obj.get("sizes"), i, "sizes")?;
    let series_type = parse_optional_combo_series_type(obj.get("type"), i, "type")?;
    Ok(charts::AddChartSeriesOp {
        id,
        target,
        name,
        categories,
        values,
        sizes,
        series_type,
    })
}

pub(super) fn parse_delete_chart_series(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<charts::DeleteChartSeriesOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let target = charts::parse_series_id(&id, i)?;
    Ok(charts::DeleteChartSeriesOp { id, target })
}

pub(super) fn parse_set_chart_type(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<charts::SetChartTypeOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op",
            "id",
            "type",
            "ofPieType",
            "stockKind",
            "surfaceKind",
            "layoutId",
        ],
    )?;
    let id = require_id(obj, i)?;
    let target = charts::parse_chart_id(&id, i)?;
    let chart_type = parse_chart_type_field(obj.get("type"), i, true)?;
    if chart_type == ChartType::Combo {
        return Err(Error::with_path(
            ErrorKind::InvalidChartType,
            format!("ops[{i}].type"),
        ));
    }
    let of_pie_type =
        crate::chart::parse_of_pie_type(obj.get("ofPieType"), &format!("ops[{i}].ofPieType"))?;
    crate::chart::reject_of_pie_on_other(
        chart_type,
        of_pie_type.as_deref(),
        &format!("ops[{i}].ofPieType"),
    )?;
    let extra = crate::chart::parse_chart_extra(obj, chart_type, 0, &format!("ops[{i}]"), false)?;
    Ok(charts::SetChartTypeOp {
        id,
        target,
        chart_type,
        of_pie_type,
        extra,
    })
}

pub(super) fn parse_set_chart_axis_title(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<charts::SetChartAxisTitleOp, Error> {
    deny_keys(obj, i, &["op", "id", "axis", "title"])?;
    let id = require_id(obj, i)?;
    let target = charts::parse_chart_id(&id, i)?;
    let axis = match obj.get("axis") {
        Some(Value::String(s)) if s == "cat" || s == "val" => s.clone(),
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidChartAxis,
                format!("ops[{i}].axis"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "axis".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].axis"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::OpRequiresField {
                    op: "setChartAxisTitle".into(),
                    field: "axis".into(),
                },
                format!("ops[{i}].axis"),
            ));
        }
    };
    let title = match obj.get("title") {
        Some(Value::String(s)) => s.clone(),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "title".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].title"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::OpRequiresField {
                    op: "setChartAxisTitle".into(),
                    field: "title".into(),
                },
                format!("ops[{i}].title"),
            ));
        }
    };
    Ok(charts::SetChartAxisTitleOp {
        id,
        target,
        axis,
        title,
    })
}

pub(super) fn parse_set_chart_legend(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<charts::SetChartLegendOp, Error> {
    deny_keys(obj, i, &["op", "id", "show", "pos"])?;
    let id = require_id(obj, i)?;
    let target = charts::parse_chart_id(&id, i)?;
    let show = parse_required_bool(obj.get("show"), i, "setChartLegend", "show")?;
    let pos = match obj.get("pos") {
        None | Some(Value::Null) => None,
        Some(Value::String(s)) if matches!(s.as_str(), "b" | "t" | "l" | "r" | "tr") => {
            Some(s.clone())
        }
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidLegendPos,
                format!("ops[{i}].pos"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "pos".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].pos"),
            ));
        }
    };
    Ok(charts::SetChartLegendOp {
        id,
        target,
        show,
        pos,
    })
}

pub(super) fn parse_set_chart_data_labels(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<charts::SetChartDataLabelsOp, Error> {
    deny_keys(obj, i, &["op", "id", "show"])?;
    let id = require_id(obj, i)?;
    let target = charts::parse_chart_id(&id, i)?;
    let show = parse_required_bool(obj.get("show"), i, "setChartDataLabels", "show")?;
    Ok(charts::SetChartDataLabelsOp { id, target, show })
}

pub(super) fn parse_set_chart_size(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<charts::SetChartSizeOp, Error> {
    deny_keys(obj, i, &["op", "id", "cx", "cy"])?;
    let id = require_id(obj, i)?;
    let target = charts::parse_chart_id(&id, i)?;
    let cx = parse_emu_field(obj.get("cx"), i, "cx")?;
    let cy = parse_emu_field(obj.get("cy"), i, "cy")?;
    if cx.is_none() && cy.is_none() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setChartSize".into(),
                field: "cx or cy".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(charts::SetChartSizeOp { id, target, cx, cy })
}

pub(super) fn parse_insert_trendline(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<charts::InsertTrendlineOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op",
            "id",
            "type",
            "order",
            "period",
            "displayRSquared",
            "displayEquation",
            "name",
        ],
    )?;
    let id = require_id(obj, i)?;
    let target = charts::parse_series_id(&id, i)?;
    let spec = crate::chart::remain::parse_trendline_write(obj, i, true)?;
    Ok(charts::InsertTrendlineOp { id, target, spec })
}

pub(super) fn parse_set_trendline(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<charts::SetTrendlineOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op",
            "id",
            "type",
            "order",
            "period",
            "displayRSquared",
            "displayEquation",
            "name",
        ],
    )?;
    let id = require_id(obj, i)?;
    let (target, tl_index) = charts::parse_trendline_id(&id, i)?;
    let spec = crate::chart::remain::parse_trendline_write(obj, i, false)?;
    Ok(charts::SetTrendlineOp {
        id,
        target,
        tl_index,
        spec,
    })
}

pub(super) fn parse_delete_trendline(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<charts::DeleteTrendlineOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let (target, tl_index) = charts::parse_trendline_id(&id, i)?;
    Ok(charts::DeleteTrendlineOp {
        id,
        target,
        tl_index,
    })
}

pub(super) fn parse_insert_error_bars(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<charts::InsertErrorBarsOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op", "id", "dir", "barType", "valType", "value", "plus", "minus",
        ],
    )?;
    let id = require_id(obj, i)?;
    let target = charts::parse_series_id(&id, i)?;
    let spec = crate::chart::remain::parse_error_bars_write(obj, i, true)?;
    Ok(charts::InsertErrorBarsOp { id, target, spec })
}

pub(super) fn parse_set_error_bars(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<charts::SetErrorBarsOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op", "id", "dir", "barType", "valType", "value", "plus", "minus",
        ],
    )?;
    let id = require_id(obj, i)?;
    let (target, eb_index) = charts::parse_error_bars_id(&id, i)?;
    let spec = crate::chart::remain::parse_error_bars_write(obj, i, false)?;
    Ok(charts::SetErrorBarsOp {
        id,
        target,
        eb_index,
        spec,
    })
}

pub(super) fn parse_delete_error_bars(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<charts::DeleteErrorBarsOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let (target, eb_index) = charts::parse_error_bars_id(&id, i)?;
    Ok(charts::DeleteErrorBarsOp {
        id,
        target,
        eb_index,
    })
}

pub(super) fn parse_set_series_axis(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<charts::SetChartSeriesAxisOp, Error> {
    deny_keys(obj, i, &["op", "id", "axis"])?;
    let id = require_id(obj, i)?;
    let target = charts::parse_series_id(&id, i)?;
    let secondary = crate::chart::remain::parse_series_axis(obj, i)?;
    Ok(charts::SetChartSeriesAxisOp {
        id,
        target,
        secondary,
    })
}

pub(super) fn parse_set_chart_wrap(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<charts::SetChartWrapOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op", "id", "kind", "wrap", "alignH", "alignV", "offsetX", "offsetY",
        ],
    )?;
    let id = require_id(obj, i)?;
    let target = charts::parse_chart_id(&id, i)?;
    let kind = match obj.get("kind") {
        Some(Value::String(s)) => picture::parse_pic_kind(s)
            .ok_or_else(|| Error::with_path(ErrorKind::InvalidPicKind, format!("ops[{i}].kind")))?,
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
                    op: "setChartWrap".into(),
                    field: "kind".into(),
                },
                format!("ops[{i}].kind"),
            ));
        }
    };
    let wrap = match obj.get("wrap") {
        None | Some(Value::Null) => None,
        Some(Value::String(s)) => Some(picture::parse_wrap(s).ok_or_else(|| {
            Error::with_path(ErrorKind::InvalidPicWrap, format!("ops[{i}].wrap"))
        })?),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "wrap".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].wrap"),
            ));
        }
    };
    if kind == PicKind::Inline && wrap.is_some() {
        return Err(Error::with_path(
            ErrorKind::InvalidPicWrap,
            format!("ops[{i}].wrap"),
        ));
    }
    if kind == PicKind::Anchor && wrap.is_none() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setChartWrap".into(),
                field: "wrap".into(),
            },
            format!("ops[{i}].wrap"),
        ));
    }
    let align_h = parse_align_h_field(obj.get("alignH"), i)?;
    let align_v = parse_align_v_field(obj.get("alignV"), i)?;
    let offset_x = parse_i64_field(obj.get("offsetX"), i, "offsetX")?;
    let offset_y = parse_i64_field(obj.get("offsetY"), i, "offsetY")?;
    if align_h.is_some() && offset_x.is_some() {
        return Err(Error::with_path(
            ErrorKind::MutuallyExclusive {
                a: "alignH".into(),
                b: "offsetX".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    if align_v.is_some() && offset_y.is_some() {
        return Err(Error::with_path(
            ErrorKind::MutuallyExclusive {
                a: "alignV".into(),
                b: "offsetY".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(charts::SetChartWrapOp {
        id,
        target,
        kind,
        wrap,
        pos: WrapPos {
            align_h,
            align_v,
            offset_x,
            offset_y,
        },
    })
}

pub(super) fn parse_optional_combo_series_type(
    v: Option<&Value>,
    i: usize,
    field: &str,
) -> Result<Option<ChartType>, Error> {
    match v {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => ChartType::parse_combo_series(s).map(Some).ok_or_else(|| {
            Error::with_path(
                ErrorKind::InvalidComboSeriesType,
                format!("ops[{i}].{field}"),
            )
        }),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].{field}"),
        )),
    }
}

pub(super) fn parse_chart_type_field(
    v: Option<&Value>,
    i: usize,
    required: bool,
) -> Result<ChartType, Error> {
    match v {
        Some(Value::String(s)) => ChartType::parse(s)
            .ok_or_else(|| Error::with_path(ErrorKind::InvalidChartType, format!("ops[{i}].type"))),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "type".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].type"),
        )),
        None if required => Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "insertChart".into(),
                field: "type".into(),
            },
            format!("ops[{i}].type"),
        )),
        None => Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setChartType".into(),
                field: "type".into(),
            },
            format!("ops[{i}].type"),
        )),
    }
}

pub(super) fn parse_datum_list(
    v: Option<&Value>,
    i: usize,
    field: &str,
) -> Result<Option<Vec<Value>>, Error> {
    match v {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Array(a)) => {
            let mut out = Vec::new();
            for (j, item) in a.iter().enumerate() {
                match item {
                    Value::String(_) | Value::Number(_) => out.push(item.clone()),
                    _ => {
                        return Err(Error::with_path(
                            ErrorKind::WrongType {
                                field: format!("{field}[{j}]"),
                                expected: TypeName::String,
                            },
                            format!("ops[{i}].{field}[{j}]"),
                        ));
                    }
                }
            }
            Ok(Some(out))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Array,
            },
            format!("ops[{i}].{field}"),
        )),
    }
}
