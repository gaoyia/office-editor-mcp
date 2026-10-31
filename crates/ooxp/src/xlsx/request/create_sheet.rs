use serde_json::{Map, Value};

use crate::error::{Error, ErrorKind, SheetNameReason, TypeName};

use super::super::a1::{self};
use super::create_types::*;
use super::{
    parse_cell_value, InsertChartSheetOp, SetCellValue,
};

pub(crate) fn parse_sheet(v: &Value, i: usize) -> Result<NewSheet, Error> {
    let obj = v.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: format!("workbook.sheets[{i}]"),
                expected: TypeName::Object,
            },
            format!("workbook.sheets[{i}]"),
        )
    })?;
    let prefix = format!("workbook.sheets[{i}]");
    let kind = parse_create_sheet_kind(obj.get("kind"), &prefix)?;
    let name = obj.get("name").and_then(Value::as_str).ok_or_else(|| {
        Error::with_path(
            ErrorKind::InvalidSheetName {
                reason: SheetNameReason::Missing,
            },
            format!("{prefix}.name"),
        )
    })?;
    super::validate_sheet_name(name, format!("{prefix}.name"))?;
    if kind == "chartsheet" {
        return parse_chartsheet_sheet(obj, i, name);
    }
    for k in obj.keys() {
        if !super::create_remain::is_sheet_key(k) {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("{prefix}.{k}"),
            ));
        }
    }
    let mut cells = Vec::new();
    if let Some(arr) = obj.get("cells") {
        let arr = arr.as_array().ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: format!("{prefix}.cells"),
                    expected: TypeName::Array,
                },
                format!("{prefix}.cells"),
            )
        })?;
        for (j, c) in arr.iter().enumerate() {
            if let Some(cell) = parse_new_cell(c, i, j)? {
                cells.push(cell);
            }
        }
    }
    cells.sort_by_key(|c| (c.refer.row, c.refer.col));
    let pictures = super::create_picture::parse_pictures(obj.get("pictures"), i, name)?;
    let charts = super::create_chart::parse_charts(obj.get("charts"), i, name)?;
    let header_footers = super::create_hf::parse_header_footers(obj.get("headerFooters"), i, name)?;
    let comments = super::create_comment::parse_comments(obj.get("comments"), i, name)?;
    let hyperlinks = super::create_link::parse_hyperlinks(obj.get("hyperlinks"), i, name)?;
    let text_boxes = super::create_drawing::parse_textboxes(obj.get("textBoxes"), i, name)?;
    let shapes = super::create_drawing::parse_shapes(obj.get("shapes"), i, name)?;
    let connectors = super::create_drawing::parse_connectors(obj.get("connectors"), i, name)?;
    let smartarts = super::create_drawing::parse_smartarts(obj.get("smartArts"), i, name)?;
    let model3ds = super::create_media::parse_model3ds(obj.get("model3ds"), i, name)?;
    let inks = super::create_media::parse_inks(obj.get("inks"), i, name)?;
    let maths = super::create_media::parse_maths(obj.get("maths"), i, name)?;
    Ok(NewSheet {
        name: name.to_string(),
        chartsheet: None,
        cells,
        pictures,
        charts,
        header_footers,
        comments,
        hyperlinks,
        text_boxes,
        shapes,
        connectors,
        smartarts,
        model3ds,
        inks,
        maths,
    })
}

pub(crate) fn parse_create_sheet_kind(
    v: Option<&Value>,
    prefix: &str,
) -> Result<&'static str, Error> {
    match v {
        None | Some(Value::Null) => Ok("worksheet"),
        Some(Value::String(s)) if s == "worksheet" => Ok("worksheet"),
        Some(Value::String(s)) if s == "chartsheet" => Ok("chartsheet"),
        Some(Value::String(_)) => Err(Error::with_path(
            ErrorKind::UnknownField {
                field: "kind".into(),
            },
            format!("{prefix}.kind"),
        )),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "kind".into(),
                expected: TypeName::String,
            },
            format!("{prefix}.kind"),
        )),
    }
}

pub(crate) fn parse_chartsheet_sheet(
    obj: &Map<String, Value>,
    i: usize,
    name: &str,
) -> Result<NewSheet, Error> {
    let prefix = format!("workbook.sheets[{i}]");
    for k in obj.keys() {
        if !super::create_remain::is_chartsheet_key(k) {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("{prefix}.{k}"),
            ));
        }
    }
    let common = super::create_chart::parse_chart_common(obj, &prefix)?;
    let data_from =
        super::create_util::parse_required_a1_field(obj.get("dataFrom"), &prefix, "dataFrom")?;
    let data_sheet = match obj.get("dataSheet") {
        None | Some(Value::Null) => None,
        Some(Value::String(s)) if !s.is_empty() => Some(s.clone()),
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "dataSheet".into(),
                },
                format!("{prefix}.dataSheet"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "dataSheet".into(),
                    expected: TypeName::String,
                },
                format!("{prefix}.dataSheet"),
            ));
        }
    };
    Ok(NewSheet {
        name: name.to_string(),
        chartsheet: Some(InsertChartSheetOp {
            name: name.to_string(),
            data_sheet,
            chart_type: common.chart_type,
            of_pie_type: common.of_pie_type,
            extra: common.extra,
            title: common.title,
            series: common.series,
            cx: common.cx,
            cy: common.cy,
            data_from,
        }),
        cells: Vec::new(),
        pictures: Vec::new(),
        charts: Vec::new(),
        header_footers: Vec::new(),
        comments: Vec::new(),
        hyperlinks: Vec::new(),
        text_boxes: Vec::new(),
        shapes: Vec::new(),
        connectors: Vec::new(),
        smartarts: Vec::new(),
        model3ds: Vec::new(),
        inks: Vec::new(),
        maths: Vec::new(),
    })
}

pub(crate) fn parse_new_cell(v: &Value, i: usize, j: usize) -> Result<Option<NewCell>, Error> {
    let obj = v.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: format!("workbook.sheets[{i}].cells[{j}]"),
                expected: TypeName::Object,
            },
            format!("workbook.sheets[{i}].cells[{j}]"),
        )
    })?;
    for k in obj.keys() {
        if !super::create_remain::is_cell_key(k) {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("workbook.sheets[{i}].cells[{j}].{k}"),
            ));
        }
    }
    let refer = obj.get("ref").and_then(Value::as_str).ok_or_else(|| {
        Error::with_path(
            ErrorKind::MissingField {
                field: "ref".into(),
            },
            format!("workbook.sheets[{i}].cells[{j}].ref"),
        )
    })?;
    let cell =
        a1::parse_cell(refer).map_err(|e| e.at(format!("workbook.sheets[{i}].cells[{j}].ref")))?;
    let has_v = obj.contains_key("value");
    let has_f = obj.contains_key("formula");
    if has_v && has_f {
        return Err(Error::with_path(
            ErrorKind::MutuallyExclusive {
                a: "value".into(),
                b: "formula".into(),
            },
            format!("workbook.sheets[{i}].cells[{j}]"),
        ));
    }
    if !has_v && !has_f {
        return Ok(None);
    }
    let formula = if has_f {
        let f = obj.get("formula").and_then(Value::as_str).ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: "formula".into(),
                    expected: TypeName::String,
                },
                format!("workbook.sheets[{i}].cells[{j}].formula"),
            )
        })?;
        if !f.starts_with('=') {
            return Err(Error::with_path(
                ErrorKind::FormulaMustStartWithEquals,
                format!("workbook.sheets[{i}].cells[{j}].formula"),
            ));
        }
        Some(f.to_string())
    } else {
        None
    };
    let value = if has_v {
        let parsed = parse_cell_value(
            &obj["value"],
            &format!("workbook.sheets[{i}].cells[{j}].value"),
        )?;
        if matches!(parsed, SetCellValue::Null) {
            return Ok(None);
        }
        Some(parsed)
    } else {
        None
    };
    Ok(Some(NewCell {
        refer: cell,
        a1: a1::format_cell(cell.col, cell.row),
        value,
        formula,
    }))
}
