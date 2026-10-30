//! 图表 op 的字段体。格式模块只解析 id 和宿主。
//! `categories` / `values` / `sizes` 按契约只收字符串和数字。

use serde_json::{Map, Value};

use crate::error::{Error, ErrorKind, TypeName};
use crate::op_fields::{self, deny_keys, require_id};

use super::{ChartExtra, ChartType};

pub struct SeriesFields {
    pub name: Option<String>,
    pub categories: Option<Vec<Value>>,
    pub values: Option<Vec<Value>>,
    pub sizes: Option<Vec<Value>>,
}

pub struct AddSeriesFields {
    pub name: Option<String>,
    pub categories: Option<Vec<Value>>,
    pub values: Vec<Value>,
    pub sizes: Option<Vec<Value>>,
    pub series_type: Option<ChartType>,
}

pub struct TypeFields {
    pub chart_type: ChartType,
    pub of_pie_type: Option<String>,
    pub extra: ChartExtra,
}

pub struct LegendFields {
    pub show: bool,
    pub pos: Option<String>,
}

pub struct AxisTitleFields {
    pub axis: String,
    pub title: String,
}

pub fn bind_set_series<T>(
    obj: &Map<String, Value>,
    i: usize,
    host: impl FnOnce(&str) -> Result<T, Error>,
) -> Result<(String, T, SeriesFields), Error> {
    deny_keys(
        obj,
        i,
        &["op", "id", "name", "categories", "values", "sizes"],
    )?;
    let id = require_id(obj, i)?;
    let host_val = host(&id)?;
    let fields = SeriesFields {
        name: parse_opt_name(obj, i)?,
        categories: parse_datum_list(obj.get("categories"), i, "categories")?,
        values: parse_datum_list(obj.get("values"), i, "values")?,
        sizes: parse_datum_list(obj.get("sizes"), i, "sizes")?,
    };
    if fields.name.is_none()
        && fields.categories.is_none()
        && fields.values.is_none()
        && fields.sizes.is_none()
    {
        return Err(Error::with_path(
            ErrorKind::NeedChartSeriesField,
            format!("ops[{i}]"),
        ));
    }
    Ok((id, host_val, fields))
}

pub fn bind_add_series<T>(
    obj: &Map<String, Value>,
    i: usize,
    host: impl FnOnce(&str) -> Result<T, Error>,
) -> Result<(String, T, AddSeriesFields), Error> {
    deny_keys(
        obj,
        i,
        &["op", "id", "name", "categories", "values", "sizes", "type"],
    )?;
    let id = require_id(obj, i)?;
    let host_val = host(&id)?;
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
    Ok((
        id,
        host_val,
        AddSeriesFields {
            name: parse_opt_name(obj, i)?,
            categories: parse_datum_list(obj.get("categories"), i, "categories")?,
            values,
            sizes: parse_datum_list(obj.get("sizes"), i, "sizes")?,
            series_type: parse_combo_type(obj.get("type"), &format!("ops[{i}].type"))?,
        },
    ))
}

pub fn bind_chart_id<T>(
    obj: &Map<String, Value>,
    i: usize,
    allowed: &[&str],
    host: impl FnOnce(&str) -> Result<T, Error>,
) -> Result<(String, T), Error> {
    deny_keys(obj, i, allowed)?;
    let id = require_id(obj, i)?;
    let host_val = host(&id)?;
    Ok((id, host_val))
}

pub fn bind_set_type<T>(
    obj: &Map<String, Value>,
    i: usize,
    missing_op: Option<&str>,
    host: impl FnOnce(&str) -> Result<T, Error>,
) -> Result<(String, T, TypeFields), Error> {
    let (id, host_val) = bind_chart_id(
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
        host,
    )?;
    let chart_type = parse_chart_type(obj.get("type"), &format!("ops[{i}].type"), missing_op)?;
    if chart_type == ChartType::Combo {
        return Err(Error::with_path(
            ErrorKind::InvalidChartType,
            format!("ops[{i}].type"),
        ));
    }
    let of_pie_type =
        super::parse_of_pie_type(obj.get("ofPieType"), &format!("ops[{i}].ofPieType"))?;
    super::reject_of_pie_on_other(
        chart_type,
        of_pie_type.as_deref(),
        &format!("ops[{i}].ofPieType"),
    )?;
    let extra = super::parse_chart_extra(obj, chart_type, 0, &format!("ops[{i}]"), false)?;
    Ok((
        id,
        host_val,
        TypeFields {
            chart_type,
            of_pie_type,
            extra,
        },
    ))
}

pub fn bind_legend<T>(
    obj: &Map<String, Value>,
    i: usize,
    host: impl FnOnce(&str) -> Result<T, Error>,
) -> Result<(String, T, LegendFields), Error> {
    let (id, host_val) = bind_chart_id(obj, i, &["op", "id", "show", "pos"], host)?;
    let show = op_fields::parse_required_bool(obj.get("show"), i, "setChartLegend", "show")?;
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
    Ok((id, host_val, LegendFields { show, pos }))
}

pub fn bind_axis_title<T>(
    obj: &Map<String, Value>,
    i: usize,
    host: impl FnOnce(&str) -> Result<T, Error>,
) -> Result<(String, T, AxisTitleFields), Error> {
    let (id, host_val) = bind_chart_id(obj, i, &["op", "id", "axis", "title"], host)?;
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
    Ok((id, host_val, AxisTitleFields { axis, title }))
}

pub fn bind_title<T>(
    obj: &Map<String, Value>,
    i: usize,
    host: impl FnOnce(&str) -> Result<T, Error>,
) -> Result<(String, T, String), Error> {
    let (id, host_val) = bind_chart_id(obj, i, &["op", "id", "title"], host)?;
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
    Ok((id, host_val, title))
}

pub fn bind_data_labels<T>(
    obj: &Map<String, Value>,
    i: usize,
    host: impl FnOnce(&str) -> Result<T, Error>,
) -> Result<(String, T, bool), Error> {
    let (id, host_val) = bind_chart_id(obj, i, &["op", "id", "show"], host)?;
    let show = op_fields::parse_required_bool(obj.get("show"), i, "setChartDataLabels", "show")?;
    Ok((id, host_val, show))
}

pub fn bind_series_axis<T>(
    obj: &Map<String, Value>,
    i: usize,
    host: impl FnOnce(&str) -> Result<T, Error>,
) -> Result<(String, T, bool), Error> {
    let (id, host_val) = bind_chart_id(obj, i, &["op", "id", "axis"], host)?;
    let secondary = super::remain::parse_series_axis(obj, i)?;
    Ok((id, host_val, secondary))
}

pub fn bind_trendline<T>(
    obj: &Map<String, Value>,
    i: usize,
    insert: bool,
    host: impl FnOnce(&str) -> Result<T, Error>,
) -> Result<(String, T, super::remain::TrendlineWrite), Error> {
    let (id, host_val) = bind_chart_id(
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
        host,
    )?;
    let spec = super::remain::parse_trendline_write(obj, i, insert)?;
    Ok((id, host_val, spec))
}

pub fn bind_error_bars<T>(
    obj: &Map<String, Value>,
    i: usize,
    insert: bool,
    host: impl FnOnce(&str) -> Result<T, Error>,
) -> Result<(String, T, super::remain::ErrorBarsWrite), Error> {
    let (id, host_val) = bind_chart_id(
        obj,
        i,
        &[
            "op", "id", "dir", "barType", "valType", "value", "plus", "minus",
        ],
        host,
    )?;
    let spec = super::remain::parse_error_bars_write(obj, i, insert)?;
    Ok((id, host_val, spec))
}

/// `missing_op` 为 `Some` 时缺 `type` 报 `OpRequiresField`；为 `None` 时报 `MissingField`。
pub fn parse_chart_type(
    v: Option<&Value>,
    path: &str,
    missing_op: Option<&str>,
) -> Result<ChartType, Error> {
    match v {
        Some(Value::String(s)) => {
            ChartType::parse(s).ok_or_else(|| Error::with_path(ErrorKind::InvalidChartType, path))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "type".into(),
                expected: TypeName::String,
            },
            path,
        )),
        None => {
            if let Some(op) = missing_op {
                Err(Error::with_path(
                    ErrorKind::OpRequiresField {
                        op: op.into(),
                        field: "type".into(),
                    },
                    path,
                ))
            } else {
                Err(Error::with_path(
                    ErrorKind::MissingField {
                        field: "type".into(),
                    },
                    path,
                ))
            }
        }
    }
}

pub fn parse_combo_type(v: Option<&Value>, path: &str) -> Result<Option<ChartType>, Error> {
    match v {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => {
            Ok(Some(ChartType::parse_combo_series(s).ok_or_else(|| {
                Error::with_path(ErrorKind::InvalidComboSeriesType, path)
            })?))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "type".into(),
                expected: TypeName::String,
            },
            path,
        )),
    }
}

pub fn parse_datum_list(
    v: Option<&Value>,
    i: usize,
    field: &str,
) -> Result<Option<Vec<Value>>, Error> {
    parse_datum_list_at(v, &format!("ops[{i}].{field}"), field)
}

pub fn parse_datum_list_at(
    v: Option<&Value>,
    path: &str,
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
                            format!("{path}[{j}]"),
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
            path,
        )),
    }
}

fn parse_opt_name(obj: &Map<String, Value>, i: usize) -> Result<Option<String>, Error> {
    match obj.get("name") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "name".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].name"),
        )),
    }
}
