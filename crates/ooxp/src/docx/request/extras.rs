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
    let chart_type = crate::chart::fields::parse_chart_type(
        obj.get("type"),
        &format!("ops[{i}].type"),
        Some("insertChart"),
    )?;
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
        let categories = crate::chart::fields::parse_datum_list(
            obj.get("categories"),
            i,
            &format!("series[{j}].categories"),
        )?;
        let values = crate::chart::fields::parse_datum_list(
            obj.get("values"),
            i,
            &format!("series[{j}].values"),
        )?
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
        let sizes = crate::chart::fields::parse_datum_list(
            obj.get("sizes"),
            i,
            &format!("series[{j}].sizes"),
        )?;
        let series_type = crate::chart::fields::parse_combo_type(
            obj.get("type"),
            &format!("ops[{i}].series[{j}].type"),
        )?;
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
    let (id, target) = crate::chart::fields::bind_chart_id(obj, i, &["op", "id"], |id| {
        charts::parse_chart_id(id, i)
    })?;
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
    let (id, target, title) =
        crate::chart::fields::bind_title(obj, i, |id| charts::parse_chart_id(id, i))?;
    Ok(charts::SetChartTitleOp { id, target, title })
}

pub(super) fn parse_set_chart_series(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<charts::SetChartSeriesOp, Error> {
    let (id, target, fields) =
        crate::chart::fields::bind_set_series(obj, i, |id| charts::parse_series_id(id, i))?;
    Ok(charts::SetChartSeriesOp {
        id,
        target,
        name: fields.name,
        categories: fields.categories,
        values: fields.values,
        sizes: fields.sizes,
    })
}

pub(super) fn parse_add_chart_series(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<charts::AddChartSeriesOp, Error> {
    let (id, target, fields) =
        crate::chart::fields::bind_add_series(obj, i, |id| charts::parse_chart_id(id, i))?;
    Ok(charts::AddChartSeriesOp {
        id,
        target,
        name: fields.name,
        categories: fields.categories,
        values: fields.values,
        sizes: fields.sizes,
        series_type: fields.series_type,
    })
}

pub(super) fn parse_delete_chart_series(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<charts::DeleteChartSeriesOp, Error> {
    let (id, target) = crate::chart::fields::bind_chart_id(obj, i, &["op", "id"], |id| {
        charts::parse_series_id(id, i)
    })?;
    Ok(charts::DeleteChartSeriesOp { id, target })
}

pub(super) fn parse_set_chart_type(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<charts::SetChartTypeOp, Error> {
    let (id, target, fields) =
        crate::chart::fields::bind_set_type(obj, i, Some("insertChart"), |id| {
            charts::parse_chart_id(id, i)
        })?;
    Ok(charts::SetChartTypeOp {
        id,
        target,
        chart_type: fields.chart_type,
        of_pie_type: fields.of_pie_type,
        extra: fields.extra,
    })
}

pub(super) fn parse_set_chart_axis_title(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<charts::SetChartAxisTitleOp, Error> {
    let (id, target, fields) =
        crate::chart::fields::bind_axis_title(obj, i, |id| charts::parse_chart_id(id, i))?;
    Ok(charts::SetChartAxisTitleOp {
        id,
        target,
        axis: fields.axis,
        title: fields.title,
    })
}

pub(super) fn parse_set_chart_legend(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<charts::SetChartLegendOp, Error> {
    let (id, target, fields) =
        crate::chart::fields::bind_legend(obj, i, |id| charts::parse_chart_id(id, i))?;
    Ok(charts::SetChartLegendOp {
        id,
        target,
        show: fields.show,
        pos: fields.pos,
    })
}

pub(super) fn parse_set_chart_data_labels(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<charts::SetChartDataLabelsOp, Error> {
    let (id, target, show) =
        crate::chart::fields::bind_data_labels(obj, i, |id| charts::parse_chart_id(id, i))?;
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
    let (id, target, spec) =
        crate::chart::fields::bind_trendline(obj, i, true, |id| charts::parse_series_id(id, i))?;
    Ok(charts::InsertTrendlineOp { id, target, spec })
}

pub(super) fn parse_set_trendline(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<charts::SetTrendlineOp, Error> {
    let (id, (target, tl_index), spec) =
        crate::chart::fields::bind_trendline(obj, i, false, |id| {
            charts::parse_trendline_id(id, i)
        })?;
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
    let (id, (target, tl_index)) =
        crate::chart::fields::bind_chart_id(obj, i, &["op", "id"], |id| {
            charts::parse_trendline_id(id, i)
        })?;
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
    let (id, target, spec) =
        crate::chart::fields::bind_error_bars(obj, i, true, |id| charts::parse_series_id(id, i))?;
    Ok(charts::InsertErrorBarsOp { id, target, spec })
}

pub(super) fn parse_set_error_bars(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<charts::SetErrorBarsOp, Error> {
    let (id, (target, eb_index), spec) =
        crate::chart::fields::bind_error_bars(obj, i, false, |id| {
            charts::parse_error_bars_id(id, i)
        })?;
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
    let (id, (target, eb_index)) =
        crate::chart::fields::bind_chart_id(obj, i, &["op", "id"], |id| {
            charts::parse_error_bars_id(id, i)
        })?;
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
    let (id, target, secondary) =
        crate::chart::fields::bind_series_axis(obj, i, |id| charts::parse_series_id(id, i))?;
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
