use std::collections::HashSet;

use serde_json::Value;

use crate::error::{Error, ErrorKind, TypeName};

use super::super::charts::{self, ChartType, NewSeries};
use super::super::picture::{self, PicKind, WrapKind};
use super::create_types::*;

pub(crate) fn parse_charts(v: Option<&Value>, prefix: &str) -> Result<Vec<CreateChart>, Error> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    let path = format!("{prefix}.charts");
    let arr = v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "charts".into(),
                expected: TypeName::Array,
            },
            path.clone(),
        )
    })?;
    let mut out = Vec::new();
    for (j, item) in arr.iter().enumerate() {
        let item_path = format!("{path}[{j}]");
        out.push(parse_one_chart(item, &item_path)?);
    }
    Ok(out)
}

pub(crate) fn parse_one_chart(v: &Value, path: &str) -> Result<CreateChart, Error> {
    let obj = v.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: path.into(),
                expected: TypeName::Object,
            },
            path,
        )
    })?;
    for k in obj.keys() {
        if !matches!(
            k.as_str(),
            "type"
                | "ofPieType"
                | "stockKind"
                | "surfaceKind"
                | "layoutId"
                | "title"
                | "series"
                | "kind"
                | "wrap"
                | "cx"
                | "cy"
                | "hyperlink"
        ) {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("{path}.{k}"),
            ));
        }
    }
    let chart_type = match obj.get("type") {
        Some(Value::String(s)) => ChartType::parse(s)
            .ok_or_else(|| Error::with_path(ErrorKind::InvalidChartType, format!("{path}.type")))?,
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
    let title = match obj.get("title") {
        None | Some(Value::Null) => None,
        Some(Value::String(s)) => Some(s.clone()),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "title".into(),
                    expected: TypeName::String,
                },
                format!("{path}.title"),
            ));
        }
    };
    let series = parse_create_series(obj.get("series"), path)?;
    let extra = crate::chart::parse_chart_extra(obj, chart_type, series.len(), path, true)?;
    crate::chart::validate_insert_series_len(
        chart_type,
        series.len(),
        &extra,
        &format!("{path}.series"),
    )?;
    let of_pie_type =
        crate::chart::parse_of_pie_type(obj.get("ofPieType"), &format!("{path}.ofPieType"))?;
    crate::chart::reject_of_pie_on_other(
        chart_type,
        of_pie_type.as_deref(),
        &format!("{path}.ofPieType"),
    )?;
    if chart_type == ChartType::Combo {
        let kinds: HashSet<&str> = series
            .iter()
            .filter_map(|s| s.series_type.map(ChartType::as_str))
            .collect();
        if series.iter().any(|s| s.series_type.is_none()) || kinds.len() < 2 {
            return Err(Error::with_path(
                ErrorKind::InvalidComboSeriesType,
                format!("{path}.series"),
            ));
        }
    } else if series.iter().any(|s| s.series_type.is_some()) {
        return Err(Error::with_path(
            ErrorKind::InvalidComboSeriesType,
            format!("{path}.series"),
        ));
    }
    if chart_type != ChartType::Bubble && series.iter().any(|s| s.sizes.is_some()) {
        return Err(Error::with_path(
            ErrorKind::SizesOnNonBubble,
            format!("{path}.series"),
        ));
    }
    let kind = match obj.get("kind") {
        None => PicKind::Anchor,
        Some(Value::String(s)) => picture::parse_pic_kind(s)
            .ok_or_else(|| Error::with_path(ErrorKind::InvalidPicKind, format!("{path}.kind")))?,
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "kind".into(),
                    expected: TypeName::String,
                },
                format!("{path}.kind"),
            ));
        }
    };
    let wrap =
        match obj.get("wrap") {
            None | Some(Value::Null) => {
                if kind == PicKind::Anchor {
                    Some(WrapKind::Square)
                } else {
                    None
                }
            }
            Some(Value::String(s)) => Some(picture::parse_wrap(s).ok_or_else(|| {
                Error::with_path(ErrorKind::InvalidPicWrap, format!("{path}.wrap"))
            })?),
            Some(_) => {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: "wrap".into(),
                        expected: TypeName::String,
                    },
                    format!("{path}.wrap"),
                ));
            }
        };
    if kind == PicKind::Inline && wrap.is_some() {
        return Err(Error::with_path(
            ErrorKind::InvalidPicWrap,
            format!("{path}.wrap"),
        ));
    }
    let cx = super::create_util::parse_emu(obj.get("cx"), path, "cx")?;
    let cy = super::create_util::parse_emu(obj.get("cy"), path, "cy")?;
    if cx.is_some() != cy.is_some() {
        return Err(Error::with_path(
            ErrorKind::MissingField {
                field: "cx and cy".into(),
            },
            path.to_string(),
        ));
    }
    Ok(CreateChart {
        chart_type,
        of_pie_type,
        extra,
        title,
        series,
        kind,
        wrap,
        cx: cx.unwrap_or(charts::default_cx()),
        cy: cy.unwrap_or(charts::default_cy()),
        hyperlink: super::doc_extra::parse_drawing_hlink(
            obj.get("hyperlink"),
            &format!("{path}.hyperlink"),
        )?,
    })
}

pub(crate) fn parse_create_series(v: Option<&Value>, path: &str) -> Result<Vec<NewSeries>, Error> {
    let arr = match v {
        Some(Value::Array(a)) if !a.is_empty() => a,
        Some(Value::Array(_)) | None => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "series".into(),
                },
                format!("{path}.series"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "series".into(),
                    expected: TypeName::Array,
                },
                format!("{path}.series"),
            ));
        }
    };
    let mut out = Vec::new();
    for (j, item) in arr.iter().enumerate() {
        let item_path = format!("{path}.series[{j}]");
        let obj = item.as_object().ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: format!("series[{j}]"),
                    expected: TypeName::Object,
                },
                item_path.clone(),
            )
        })?;
        for k in obj.keys() {
            if !matches!(
                k.as_str(),
                "name" | "categories" | "values" | "sizes" | "type"
            ) {
                return Err(Error::with_path(
                    ErrorKind::UnknownField { field: k.clone() },
                    format!("{item_path}.{k}"),
                ));
            }
        }
        let name = match obj.get("name") {
            None | Some(Value::Null) => None,
            Some(Value::String(s)) => Some(s.clone()),
            Some(_) => {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: "name".into(),
                        expected: TypeName::String,
                    },
                    format!("{item_path}.name"),
                ));
            }
        };
        let categories = super::create_util::parse_datum_list(
            obj.get("categories"),
            &format!("{item_path}.categories"),
        )?;
        let values = super::create_util::parse_datum_list(
            obj.get("values"),
            &format!("{item_path}.values"),
        )?
        .filter(|v| !v.is_empty())
        .ok_or_else(|| {
            Error::with_path(
                ErrorKind::MissingField {
                    field: "values".into(),
                },
                format!("{item_path}.values"),
            )
        })?;
        let sizes =
            super::create_util::parse_datum_list(obj.get("sizes"), &format!("{item_path}.sizes"))?;
        let series_type = match obj.get("type") {
            None | Some(Value::Null) => None,
            Some(Value::String(s)) => Some(ChartType::parse_combo_series(s).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::InvalidComboSeriesType,
                    format!("{item_path}.type"),
                )
            })?),
            Some(_) => {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: "type".into(),
                        expected: TypeName::String,
                    },
                    format!("{item_path}.type"),
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
