//! pptx request：chart。

use serde_json::{Map, Value};

use crate::chart::{ChartExtra, ChartType, NewSeries};
use crate::error::{Error, ErrorKind, TypeName};

use super::super::chart;

use super::common::*;
use super::link::*;
use super::types::*;

pub(super) fn parse_insert_chart(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<InsertChartOp, Error> {
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
            "x",
            "y",
            "cx",
            "cy",
        ],
    )?;
    let id = require_id(obj, i)?;
    let (slide, on_notes, master) = chart::parse_insert_slide_id(&id, i)?.parts();
    let chart_type = parse_chart_type_field(obj.get("type"), &format!("ops[{i}].type"))?;
    let of_pie_type =
        crate::chart::parse_of_pie_type(obj.get("ofPieType"), &format!("ops[{i}].ofPieType"))?;
    crate::chart::reject_of_pie_on_other(
        chart_type,
        of_pie_type.as_deref(),
        &format!("ops[{i}].ofPieType"),
    )?;
    let title = parse_chart_title_field(obj.get("title"), &format!("ops[{i}].title"))?;
    let series = parse_insert_series(obj.get("series"), &format!("ops[{i}]"), "insertChart")?;
    let extra =
        crate::chart::parse_chart_extra(obj, chart_type, series.len(), &format!("ops[{i}]"), true)?;
    validate_insert_series(chart_type, &series, &extra, &format!("ops[{i}].series"))?;
    let x = parse_off_field(obj.get("x"), i, "x")?;
    let y = parse_off_field(obj.get("y"), i, "y")?;
    if x.is_some() != y.is_some() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "insertChart".into(),
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
                op: "insertChart".into(),
                field: "cx and cy together".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(InsertChartOp {
        slide,
        on_notes,
        master,
        chart_type,
        of_pie_type,
        extra,
        title,
        series,
        x,
        y,
        cx,
        cy,
    })
}

pub(super) fn parse_delete_chart(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<DeleteChartOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let target = chart::parse_chart_id(&id, &format!("ops[{i}].id"))?;
    Ok(DeleteChartOp { id, target })
}

pub(super) fn parse_set_chart_title(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<SetChartTitleOp, Error> {
    deny_keys(obj, i, &["op", "id", "title"])?;
    let id = require_id(obj, i)?;
    let target = chart::parse_chart_id(&id, &format!("ops[{i}].id"))?;
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
    Ok(SetChartTitleOp { id, target, title })
}

pub(super) fn parse_set_chart_series(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<SetChartSeriesOp, Error> {
    deny_keys(
        obj,
        i,
        &["op", "id", "name", "categories", "values", "sizes"],
    )?;
    let id = require_id(obj, i)?;
    let (target, index) = chart::parse_series_id(&id, &format!("ops[{i}].id"))?;
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
    let categories = parse_datum_list(obj.get("categories"), &format!("ops[{i}].categories"))?;
    let values = parse_datum_list(obj.get("values"), &format!("ops[{i}].values"))?;
    let sizes = parse_datum_list(obj.get("sizes"), &format!("ops[{i}].sizes"))?;
    if name.is_none() && categories.is_none() && values.is_none() && sizes.is_none() {
        return Err(Error::with_path(
            ErrorKind::NeedChartSeriesField,
            format!("ops[{i}]"),
        ));
    }
    Ok(SetChartSeriesOp {
        id,
        target,
        index,
        name,
        categories,
        values,
        sizes,
    })
}

pub(super) fn parse_add_chart_series(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<AddChartSeriesOp, Error> {
    deny_keys(
        obj,
        i,
        &["op", "id", "name", "categories", "values", "sizes", "type"],
    )?;
    let id = require_id(obj, i)?;
    let target = chart::parse_chart_id(&id, &format!("ops[{i}].id"))?;
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
    let categories = parse_datum_list(obj.get("categories"), &format!("ops[{i}].categories"))?;
    let values = parse_datum_list(obj.get("values"), &format!("ops[{i}].values"))?
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
    let sizes = parse_datum_list(obj.get("sizes"), &format!("ops[{i}].sizes"))?;
    let series_type = parse_optional_combo_series_type(obj.get("type"), &format!("ops[{i}].type"))?;
    Ok(AddChartSeriesOp {
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
    obj: &Map<String, Value>,
    i: usize,
) -> Result<DeleteChartSeriesOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let (target, index) = chart::parse_series_id(&id, &format!("ops[{i}].id"))?;
    Ok(DeleteChartSeriesOp { id, target, index })
}

pub(super) fn parse_set_chart_type(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<SetChartTypeOp, Error> {
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
    let target = chart::parse_chart_id(&id, &format!("ops[{i}].id"))?;
    let chart_type = parse_chart_type_field(obj.get("type"), &format!("ops[{i}].type"))?;
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
    Ok(SetChartTypeOp {
        id,
        target,
        chart_type,
        of_pie_type,
        extra,
    })
}

pub(super) fn parse_set_chart_axis_title(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<SetChartAxisTitleOp, Error> {
    deny_keys(obj, i, &["op", "id", "axis", "title"])?;
    let id = require_id(obj, i)?;
    let target = chart::parse_chart_id(&id, &format!("ops[{i}].id"))?;
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
    Ok(SetChartAxisTitleOp {
        id,
        target,
        axis,
        title,
    })
}

pub(super) fn parse_set_chart_legend(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<SetChartLegendOp, Error> {
    deny_keys(obj, i, &["op", "id", "show", "pos"])?;
    let id = require_id(obj, i)?;
    let target = chart::parse_chart_id(&id, &format!("ops[{i}].id"))?;
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
    Ok(SetChartLegendOp {
        id,
        target,
        show,
        pos,
    })
}

pub(super) fn parse_set_chart_data_labels(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<SetChartDataLabelsOp, Error> {
    deny_keys(obj, i, &["op", "id", "show"])?;
    let id = require_id(obj, i)?;
    let target = chart::parse_chart_id(&id, &format!("ops[{i}].id"))?;
    let show = parse_required_bool(obj.get("show"), i, "setChartDataLabels", "show")?;
    Ok(SetChartDataLabelsOp { id, target, show })
}

pub(super) fn parse_set_chart_size(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<SetChartSizeOp, Error> {
    deny_keys(obj, i, &["op", "id", "cx", "cy"])?;
    let id = require_id(obj, i)?;
    let target = chart::parse_chart_id(&id, &format!("ops[{i}].id"))?;
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
    Ok(SetChartSizeOp { id, target, cx, cy })
}

pub(super) fn parse_set_chart_pos(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<SetChartPosOp, Error> {
    deny_keys(obj, i, &["op", "id", "x", "y"])?;
    let id = require_id(obj, i)?;
    let target = chart::parse_chart_id(&id, &format!("ops[{i}].id"))?;
    let x = parse_off_field(obj.get("x"), i, "x")?;
    let y = parse_off_field(obj.get("y"), i, "y")?;
    if x.is_none() && y.is_none() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setChartPos".into(),
                field: "x or y".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(SetChartPosOp { id, target, x, y })
}

pub(super) fn parse_insert_trendline(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<InsertTrendlineOp, Error> {
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
    let (target, index) = chart::parse_series_id(&id, &format!("ops[{i}].id"))?;
    let spec = crate::chart::remain::parse_trendline_write(obj, i, true)?;
    Ok(InsertTrendlineOp {
        id,
        target,
        index,
        spec,
    })
}

pub(super) fn parse_set_trendline(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<SetTrendlineOp, Error> {
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
    let (target, index, tl_index) = chart::parse_trendline_id(&id, &format!("ops[{i}].id"))?;
    let spec = crate::chart::remain::parse_trendline_write(obj, i, false)?;
    Ok(SetTrendlineOp {
        id,
        target,
        index,
        tl_index,
        spec,
    })
}

pub(super) fn parse_delete_trendline(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<DeleteTrendlineOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let (target, index, tl_index) = chart::parse_trendline_id(&id, &format!("ops[{i}].id"))?;
    Ok(DeleteTrendlineOp {
        id,
        target,
        index,
        tl_index,
    })
}

pub(super) fn parse_insert_error_bars(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<InsertErrorBarsOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op", "id", "dir", "barType", "valType", "value", "plus", "minus",
        ],
    )?;
    let id = require_id(obj, i)?;
    let (target, index) = chart::parse_series_id(&id, &format!("ops[{i}].id"))?;
    let spec = crate::chart::remain::parse_error_bars_write(obj, i, true)?;
    Ok(InsertErrorBarsOp {
        id,
        target,
        index,
        spec,
    })
}

pub(super) fn parse_set_error_bars(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<SetErrorBarsOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op", "id", "dir", "barType", "valType", "value", "plus", "minus",
        ],
    )?;
    let id = require_id(obj, i)?;
    let (target, index, eb_index) = chart::parse_error_bars_id(&id, &format!("ops[{i}].id"))?;
    let spec = crate::chart::remain::parse_error_bars_write(obj, i, false)?;
    Ok(SetErrorBarsOp {
        id,
        target,
        index,
        eb_index,
        spec,
    })
}

pub(super) fn parse_delete_error_bars(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<DeleteErrorBarsOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let (target, index, eb_index) = chart::parse_error_bars_id(&id, &format!("ops[{i}].id"))?;
    Ok(DeleteErrorBarsOp {
        id,
        target,
        index,
        eb_index,
    })
}

pub(super) fn parse_set_series_axis(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<SetChartSeriesAxisOp, Error> {
    deny_keys(obj, i, &["op", "id", "axis"])?;
    let id = require_id(obj, i)?;
    let (target, index) = chart::parse_series_id(&id, &format!("ops[{i}].id"))?;
    let secondary = crate::chart::remain::parse_series_axis(obj, i)?;
    Ok(SetChartSeriesAxisOp {
        id,
        target,
        index,
        secondary,
    })
}

pub(super) fn parse_chart_type_field(v: Option<&Value>, path: &str) -> Result<ChartType, Error> {
    match v {
        Some(Value::String(s)) => ChartType::parse(s)
            .ok_or_else(|| Error::with_path(ErrorKind::InvalidChartType, path.to_string())),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "type".into(),
                expected: TypeName::String,
            },
            path,
        )),
        None => Err(Error::with_path(
            ErrorKind::MissingField {
                field: "type".into(),
            },
            path,
        )),
    }
}

pub(super) fn parse_chart_title_field(
    v: Option<&Value>,
    path: &str,
) -> Result<Option<String>, Error> {
    match v {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "title".into(),
                expected: TypeName::String,
            },
            path,
        )),
    }
}

pub(super) fn validate_insert_series(
    chart_type: ChartType,
    series: &[NewSeries],
    extra: &ChartExtra,
    path: &str,
) -> Result<(), Error> {
    if chart_type.needs_one_series() && series.len() != 1 {
        return Err(Error::with_path(ErrorKind::PieNeedsOneSeries, path));
    }
    crate::chart::validate_insert_series_len(chart_type, series.len(), extra, path)?;
    if chart_type == ChartType::Combo {
        let kinds: std::collections::HashSet<&str> = series
            .iter()
            .filter_map(|s| s.series_type.map(ChartType::as_str))
            .collect();
        if series.iter().any(|s| s.series_type.is_none()) || kinds.len() < 2 {
            return Err(Error::with_path(ErrorKind::InvalidComboSeriesType, path));
        }
    } else if series.iter().any(|s| s.series_type.is_some()) {
        return Err(Error::with_path(ErrorKind::InvalidComboSeriesType, path));
    }
    if chart_type != ChartType::Bubble && series.iter().any(|s| s.sizes.is_some()) {
        return Err(Error::with_path(ErrorKind::SizesOnNonBubble, path));
    }
    Ok(())
}

pub(super) fn parse_insert_series(
    v: Option<&Value>,
    base: &str,
    op: &str,
) -> Result<Vec<NewSeries>, Error> {
    let arr = match v {
        Some(Value::Array(a)) if !a.is_empty() => a,
        Some(Value::Array(_)) | None => {
            return Err(Error::with_path(
                ErrorKind::OpRequiresField {
                    op: op.into(),
                    field: "series".into(),
                },
                format!("{base}.series"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "series".into(),
                    expected: TypeName::Array,
                },
                format!("{base}.series"),
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
                format!("{base}.series[{j}]"),
            )
        })?;
        deny_keys_at(
            obj,
            &format!("{base}.series[{j}]"),
            &["name", "categories", "values", "sizes", "type"],
        )?;
        let name = match obj.get("name") {
            None | Some(Value::Null) => None,
            Some(Value::String(s)) => Some(s.clone()),
            Some(_) => {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: format!("series[{j}].name"),
                        expected: TypeName::String,
                    },
                    format!("{base}.series[{j}].name"),
                ));
            }
        };
        let categories = parse_datum_list(
            obj.get("categories"),
            &format!("{base}.series[{j}].categories"),
        )?;
        let values = parse_datum_list(obj.get("values"), &format!("{base}.series[{j}].values"))?
            .filter(|v| !v.is_empty())
            .ok_or_else(|| {
                Error::with_path(
                    ErrorKind::OpRequiresField {
                        op: op.into(),
                        field: "series[].values".into(),
                    },
                    format!("{base}.series[{j}].values"),
                )
            })?;
        let sizes = parse_datum_list(obj.get("sizes"), &format!("{base}.series[{j}].sizes"))?;
        let series_type =
            parse_optional_combo_series_type(obj.get("type"), &format!("{base}.series[{j}].type"))?;
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

pub(super) fn parse_optional_combo_series_type(
    v: Option<&Value>,
    path: &str,
) -> Result<Option<ChartType>, Error> {
    match v {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(ChartType::parse_combo_series(s).ok_or_else(|| {
            Error::with_path(ErrorKind::InvalidComboSeriesType, path.to_string())
        })?)),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "type".into(),
                expected: TypeName::String,
            },
            path,
        )),
    }
}

pub(super) fn parse_datum_list(v: Option<&Value>, path: &str) -> Result<Option<Vec<Value>>, Error> {
    match v {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Array(a)) => {
            for (j, item) in a.iter().enumerate() {
                if !matches!(
                    item,
                    Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_)
                ) {
                    return Err(Error::with_path(
                        ErrorKind::InvalidCellValue,
                        format!("{path}[{j}]"),
                    ));
                }
            }
            Ok(Some(a.clone()))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "array".into(),
                expected: TypeName::Array,
            },
            path,
        )),
    }
}

pub(crate) fn parse_create_chart(
    v: &Value,
    prefix: &str,
    slide: usize,
) -> Result<CreateChart, Error> {
    let obj = v.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "charts".into(),
                expected: TypeName::Object,
            },
            prefix.to_string(),
        )
    })?;
    deny_keys_at(
        obj,
        prefix,
        &[
            "type",
            "ofPieType",
            "stockKind",
            "surfaceKind",
            "layoutId",
            "title",
            "series",
            "x",
            "y",
            "cx",
            "cy",
            "hyperlink",
        ],
    )?;
    let chart_type = parse_chart_type_field(obj.get("type"), &format!("{prefix}.type"))?;
    let of_pie_type =
        crate::chart::parse_of_pie_type(obj.get("ofPieType"), &format!("{prefix}.ofPieType"))?;
    crate::chart::reject_of_pie_on_other(
        chart_type,
        of_pie_type.as_deref(),
        &format!("{prefix}.ofPieType"),
    )?;
    let title = parse_chart_title_field(obj.get("title"), &format!("{prefix}.title"))?;
    let series = parse_insert_series(obj.get("series"), prefix, "create")?;
    let extra = crate::chart::parse_chart_extra(obj, chart_type, series.len(), prefix, true)?;
    validate_insert_series(chart_type, &series, &extra, &format!("{prefix}.series"))?;
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
    let op = InsertChartOp {
        slide,
        on_notes: false,
        master: None,
        chart_type,
        of_pie_type,
        extra,
        title,
        series,
        x,
        y,
        cx,
        cy,
    };
    let link = parse_create_opt_hyperlink(obj, prefix)?;
    Ok(CreateChart { op, link })
}
