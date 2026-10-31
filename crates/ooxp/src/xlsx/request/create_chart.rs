use std::collections::HashSet;

use serde_json::{Map, Value};

use crate::chart::{ChartExtra, ChartType, NewSeries};
use crate::error::{Error, ErrorKind, TypeName};

use super::super::a1::{self};
use super::super::picture::AnchorKind;
use super::InsertChartOp;

pub(crate) fn parse_charts(
    v: Option<&Value>,
    sheet_i: usize,
    sheet: &str,
) -> Result<Vec<InsertChartOp>, Error> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    let prefix = format!("workbook.sheets[{sheet_i}].charts");
    let arr = v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "charts".into(),
                expected: TypeName::Array,
            },
            prefix.clone(),
        )
    })?;
    let mut out = Vec::new();
    for (j, item) in arr.iter().enumerate() {
        out.push(parse_chart(item, &format!("{prefix}[{j}]"), sheet)?);
    }
    Ok(out)
}

pub(crate) struct ChartCommon {
    pub(crate) chart_type: ChartType,
    pub(crate) of_pie_type: Option<String>,
    pub(crate) extra: ChartExtra,
    pub(crate) title: Option<String>,
    pub(crate) series: Vec<NewSeries>,
    pub(crate) cx: Option<i64>,
    pub(crate) cy: Option<i64>,
}

pub(crate) fn parse_chart_common(
    obj: &Map<String, Value>,
    prefix: &str,
) -> Result<ChartCommon, Error> {
    let chart_type = match obj.get("type") {
        Some(Value::String(s)) => ChartType::parse(s).ok_or_else(|| {
            Error::with_path(ErrorKind::InvalidChartType, format!("{prefix}.type"))
        })?,
        None => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "type".into(),
                },
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
    let title = match obj.get("title") {
        None | Some(Value::Null) => None,
        Some(Value::String(s)) => Some(s.clone()),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "title".into(),
                    expected: TypeName::String,
                },
                format!("{prefix}.title"),
            ));
        }
    };
    let series = parse_chart_series(obj.get("series"), prefix, chart_type)?;
    let extra = crate::chart::parse_chart_extra(obj, chart_type, series.len(), prefix, true)?;
    crate::chart::validate_insert_series_len(
        chart_type,
        series.len(),
        &extra,
        &format!("{prefix}.series"),
    )?;
    let of_pie_type =
        crate::chart::parse_of_pie_type(obj.get("ofPieType"), &format!("{prefix}.ofPieType"))?;
    crate::chart::reject_of_pie_on_other(
        chart_type,
        of_pie_type.as_deref(),
        &format!("{prefix}.ofPieType"),
    )?;
    if chart_type == ChartType::Combo {
        let kinds: HashSet<&str> = series
            .iter()
            .filter_map(|s| s.series_type.map(ChartType::as_str))
            .collect();
        if series.iter().any(|s| s.series_type.is_none()) || kinds.len() < 2 {
            return Err(Error::with_path(
                ErrorKind::InvalidComboSeriesType,
                format!("{prefix}.series"),
            ));
        }
    } else if series.iter().any(|s| s.series_type.is_some()) {
        return Err(Error::with_path(
            ErrorKind::InvalidComboSeriesType,
            format!("{prefix}.series"),
        ));
    }
    if chart_type != ChartType::Bubble && series.iter().any(|s| s.sizes.is_some()) {
        return Err(Error::with_path(
            ErrorKind::SizesOnNonBubble,
            format!("{prefix}.series"),
        ));
    }
    let cx = super::create_util::parse_emu(obj.get("cx"), prefix, "cx")?;
    let cy = super::create_util::parse_emu(obj.get("cy"), prefix, "cy")?;
    if cx.is_some() != cy.is_some() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "create".into(),
                field: "cx and cy together".into(),
            },
            prefix.to_string(),
        ));
    }
    Ok(ChartCommon {
        chart_type,
        of_pie_type,
        extra,
        title,
        series,
        cx,
        cy,
    })
}

pub(crate) fn parse_chart(v: &Value, prefix: &str, sheet: &str) -> Result<InsertChartOp, Error> {
    let obj = v.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "charts".into(),
                expected: TypeName::Object,
            },
            prefix.to_string(),
        )
    })?;
    for k in obj.keys() {
        if matches!(k.as_str(), "kind" | "wrap") {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("{prefix}.{k}"),
            ));
        }
        if !matches!(
            k.as_str(),
            "type"
                | "ofPieType"
                | "stockKind"
                | "surfaceKind"
                | "layoutId"
                | "title"
                | "series"
                | "from"
                | "to"
                | "anchor"
                | "cx"
                | "cy"
                | "fromOffX"
                | "fromOffY"
                | "toOffX"
                | "toOffY"
                | "dataFrom"
        ) {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("{prefix}.{k}"),
            ));
        }
    }
    let common = parse_chart_common(obj, prefix)?;
    let from = super::create_util::parse_required_a1_field(obj.get("from"), prefix, "from")?;
    let data_from =
        super::create_util::parse_required_a1_field(obj.get("dataFrom"), prefix, "dataFrom")?;
    let anchor = match obj.get("anchor") {
        None => AnchorKind::OneCell,
        Some(Value::String(s)) => AnchorKind::parse_insert(s).ok_or_else(|| {
            Error::with_path(ErrorKind::InvalidXlsxAnchor, format!("{prefix}.anchor"))
        })?,
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "anchor".into(),
                    expected: TypeName::String,
                },
                format!("{prefix}.anchor"),
            ));
        }
    };
    let to = match obj.get("to") {
        None => None,
        Some(Value::String(s)) => {
            Some(a1::parse_cell(s).map_err(|e| e.at(format!("{prefix}.to")))?)
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "to".into(),
                    expected: TypeName::String,
                },
                format!("{prefix}.to"),
            ));
        }
    };
    if anchor == AnchorKind::TwoCell && to.is_none() {
        return Err(Error::with_path(
            ErrorKind::MissingField { field: "to".into() },
            format!("{prefix}.to"),
        ));
    }
    if anchor == AnchorKind::OneCell && to.is_some() {
        return Err(Error::with_path(
            ErrorKind::UnknownField { field: "to".into() },
            format!("{prefix}.to"),
        ));
    }
    Ok(InsertChartOp {
        sheet: sheet.to_string(),
        chart_type: common.chart_type,
        of_pie_type: common.of_pie_type,
        extra: common.extra,
        title: common.title,
        series: common.series,
        cx: common.cx,
        cy: common.cy,
        from,
        to,
        anchor,
        from_off_x: super::create_util::parse_off(obj.get("fromOffX"), prefix, "fromOffX")?
            .unwrap_or(0),
        from_off_y: super::create_util::parse_off(obj.get("fromOffY"), prefix, "fromOffY")?
            .unwrap_or(0),
        to_off_x: super::create_util::parse_off(obj.get("toOffX"), prefix, "toOffX")?.unwrap_or(0),
        to_off_y: super::create_util::parse_off(obj.get("toOffY"), prefix, "toOffY")?.unwrap_or(0),
        data_from,
    })
}

pub(crate) fn parse_chart_series(
    v: Option<&Value>,
    prefix: &str,
    _chart_type: ChartType,
) -> Result<Vec<NewSeries>, Error> {
    let arr = match v {
        Some(Value::Array(a)) if !a.is_empty() => a,
        Some(Value::Array(_)) | None => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "series".into(),
                },
                format!("{prefix}.series"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "series".into(),
                    expected: TypeName::Array,
                },
                format!("{prefix}.series"),
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
                format!("{prefix}.series[{j}]"),
            )
        })?;
        for k in obj.keys() {
            if !matches!(
                k.as_str(),
                "name" | "categories" | "values" | "sizes" | "type"
            ) {
                return Err(Error::with_path(
                    ErrorKind::UnknownField { field: k.clone() },
                    format!("{prefix}.series[{j}].{k}"),
                ));
            }
        }
        let name = match obj.get("name") {
            None | Some(Value::Null) => None,
            Some(Value::String(s)) => Some(s.clone()),
            Some(_) => {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: format!("series[{j}].name"),
                        expected: TypeName::String,
                    },
                    format!("{prefix}.series[{j}].name"),
                ));
            }
        };
        let categories = parse_chart_datum_list(
            obj.get("categories"),
            &format!("{prefix}.series[{j}].categories"),
        )?;
        let values =
            parse_chart_datum_list(obj.get("values"), &format!("{prefix}.series[{j}].values"))?
                .filter(|v| !v.is_empty())
                .ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::MissingField {
                            field: "series[].values".into(),
                        },
                        format!("{prefix}.series[{j}].values"),
                    )
                })?;
        let sizes =
            parse_chart_datum_list(obj.get("sizes"), &format!("{prefix}.series[{j}].sizes"))?;
        let series_type = match obj.get("type") {
            None | Some(Value::Null) => None,
            Some(Value::String(s)) => Some(ChartType::parse_combo_series(s).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::InvalidComboSeriesType,
                    format!("{prefix}.series[{j}].type"),
                )
            })?),
            Some(_) => {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: format!("series[{j}].type"),
                        expected: TypeName::String,
                    },
                    format!("{prefix}.series[{j}].type"),
                ));
            }
        };
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

pub(crate) fn parse_chart_datum_list(
    v: Option<&Value>,
    path: &str,
) -> Result<Option<Vec<Value>>, Error> {
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
                field: path.rsplit('.').next().unwrap_or("series").into(),
                expected: TypeName::Array,
            },
            path.to_string(),
        )),
    }
}
