//! xlsx request：table_data。

use serde_json::Value;

use crate::error::{Error, ErrorKind, IdReason, TypeName};

use super::super::a1;

use super::cell::*;
use super::common::*;
use super::types::*;

pub(super) fn parse_insert_data_validation(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertDataValidationOp, Error> {
    deny_keys(obj, i, DV_KEYS)?;
    let id = require_id(obj, i)?;
    let sheet = parse_sheet_only_id(&id, i, "id", IdReason::SheetOpNeedsSheet)?;
    let sqref = parse_sqref_field(obj.get("sqref"), i, true)?;
    let type_ = parse_dv_type_field(obj.get("type"), i, true)?.ok_or_else(|| {
        Error::with_path(
            ErrorKind::MissingField {
                field: "type".into(),
            },
            format!("ops[{i}].type"),
        )
    })?;
    let operator = parse_dv_operator_field(obj.get("operator"), i, false)?;
    let formula1 = parse_dv_formula_opt(obj.get("formula1"), i, "formula1")?;
    let formula2 = parse_dv_formula_opt(obj.get("formula2"), i, "formula2")?;
    let dropdown = parse_opt_bool(obj, i, "dropdown")?;
    check_dv_fields(
        type_,
        operator,
        formula1.as_deref(),
        formula2.as_deref(),
        dropdown,
        i,
    )?;
    Ok(InsertDataValidationOp {
        sheet,
        sqref,
        type_,
        operator,
        formula1,
        formula2,
        allow_blank: parse_opt_bool(obj, i, "allowBlank")?.unwrap_or(true),
        dropdown,
        show_input_message: parse_opt_bool(obj, i, "showInputMessage")?.unwrap_or(true),
        show_error_message: parse_opt_bool(obj, i, "showErrorMessage")?.unwrap_or(true),
        error_style: parse_dv_error_style_opt(obj.get("errorStyle"), i)?,
        error_title: parse_dv_string_opt(obj.get("errorTitle"), i, "errorTitle")?,
        error: parse_dv_string_opt(obj.get("error"), i, "error")?,
        prompt_title: parse_dv_string_opt(obj.get("promptTitle"), i, "promptTitle")?,
        prompt: parse_dv_string_opt(obj.get("prompt"), i, "prompt")?,
    })
}

pub(super) fn parse_set_data_validation(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetDataValidationOp, Error> {
    deny_keys(obj, i, DV_KEYS)?;
    let id = require_id(obj, i)?;
    let (sheet, index) = parse_dv_id(&id, i)?;
    let sqref = if obj.contains_key("sqref") {
        Some(parse_sqref_field(obj.get("sqref"), i, true)?)
    } else {
        None
    };
    let type_ = parse_dv_type_field(obj.get("type"), i, false)?;
    let operator = parse_dv_operator_patch(obj.get("operator"), i)?;
    let formula1 = parse_dv_formula_patch(obj.get("formula1"), i, "formula1")?;
    let formula2 = parse_dv_formula_patch(obj.get("formula2"), i, "formula2")?;
    let op = SetDataValidationOp {
        sheet,
        index,
        sqref,
        type_,
        operator,
        formula1,
        formula2,
        allow_blank: parse_opt_bool(obj, i, "allowBlank")?,
        dropdown: parse_opt_bool(obj, i, "dropdown")?,
        show_input_message: parse_opt_bool(obj, i, "showInputMessage")?,
        show_error_message: parse_opt_bool(obj, i, "showErrorMessage")?,
        error_style: parse_dv_error_style_patch(obj.get("errorStyle"), i)?,
        error_title: parse_dv_string_patch(obj.get("errorTitle"), i, "errorTitle")?,
        error: parse_dv_string_patch(obj.get("error"), i, "error")?,
        prompt_title: parse_dv_string_patch(obj.get("promptTitle"), i, "promptTitle")?,
        prompt: parse_dv_string_patch(obj.get("prompt"), i, "prompt")?,
    };
    if !op.has_patch() {
        return Err(Error::with_path(
            ErrorKind::NeedSetDataValidationField,
            format!("ops[{i}]"),
        ));
    }
    Ok(op)
}

pub(super) fn parse_delete_data_validation(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteDataValidationOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let (sheet, index) = parse_dv_id(&id, i)?;
    Ok(DeleteDataValidationOp { sheet, index })
}

pub(crate) fn icon_set_count(set: &str) -> usize {
    if set.starts_with('5') {
        5
    } else if set.starts_with('4') {
        4
    } else {
        3
    }
}

pub(super) fn parse_insert_conditional_format(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertConditionalFormatOp, Error> {
    deny_keys(obj, i, CF_KEYS)?;
    let id = require_id(obj, i)?;
    let sheet = parse_sheet_only_id(&id, i, "id", IdReason::SheetOpNeedsSheet)?;
    let sqref = parse_sqref_field(obj.get("sqref"), i, true)?;
    let type_ = parse_cf_type_field(obj.get("type"), i, true)?.ok_or_else(|| {
        Error::with_path(
            ErrorKind::MissingField {
                field: "type".into(),
            },
            format!("ops[{i}].type"),
        )
    })?;
    let operator = parse_cf_operator_field(obj.get("operator"), i, false)?;
    let formulas = parse_cf_formulas_opt(obj.get("formulas"), i)?;
    let dxf = parse_dxf_opt(obj.get("dxf"), i)?;
    let color_scale = parse_color_scale_opt(obj.get("colorScale"), i)?;
    let data_bar = parse_data_bar_opt(obj.get("dataBar"), i)?;
    let icon_set = parse_icon_set_opt(obj.get("iconSet"), i)?;
    let priority = parse_cf_priority_opt(obj.get("priority"), i)?;
    let stop_if_true = parse_opt_bool(obj, i, "stopIfTrue")?.unwrap_or(false);
    let text = parse_cf_text_opt(obj.get("text"), i)?;
    let rank = parse_opt_u32_top(obj, i, "rank", "rank", 1, u32::MAX)?;
    let percent = parse_opt_bool(obj, i, "percent")?;
    let bottom = parse_opt_bool(obj, i, "bottom")?;
    let above_average = parse_opt_bool(obj, i, "aboveAverage")?;
    let equal_average = parse_opt_bool(obj, i, "equalAverage")?;
    let std_dev = parse_cf_std_dev_opt(obj.get("stdDev"), i)?;
    check_cf_fields(
        type_,
        operator,
        formulas.as_deref(),
        dxf.as_ref(),
        color_scale.as_ref(),
        data_bar.as_ref(),
        icon_set.as_ref(),
        CfExtraCheck {
            text: text.as_deref(),
            rank,
            percent,
            bottom,
            above_average,
            equal_average,
            std_dev,
        },
        i,
        true,
    )?;
    Ok(InsertConditionalFormatOp {
        sheet,
        sqref,
        type_,
        operator,
        formulas,
        dxf,
        color_scale,
        data_bar,
        icon_set,
        priority,
        stop_if_true,
        text,
        rank,
        percent,
        bottom,
        above_average,
        equal_average,
        std_dev,
    })
}

pub(super) fn parse_set_conditional_format(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetConditionalFormatOp, Error> {
    deny_keys(obj, i, CF_KEYS)?;
    let id = require_id(obj, i)?;
    let (sheet, index) = parse_cf_id(&id, i)?;
    let sqref = if obj.contains_key("sqref") {
        Some(parse_sqref_field(obj.get("sqref"), i, true)?)
    } else {
        None
    };
    let op = SetConditionalFormatOp {
        sheet,
        index,
        sqref,
        type_: parse_cf_type_field(obj.get("type"), i, false)?,
        operator: parse_cf_operator_patch(obj.get("operator"), i)?,
        formulas: parse_cf_formulas_patch(obj.get("formulas"), i)?,
        dxf: parse_dxf_patch_field(obj.get("dxf"), i)?,
        color_scale: parse_color_scale_patch(obj.get("colorScale"), i)?,
        data_bar: parse_data_bar_patch(obj.get("dataBar"), i)?,
        icon_set: parse_icon_set_patch(obj.get("iconSet"), i)?,
        priority: parse_cf_priority_opt(obj.get("priority"), i)?,
        stop_if_true: parse_opt_bool(obj, i, "stopIfTrue")?,
        text: parse_cf_text_patch(obj.get("text"), i)?,
        rank: parse_cf_rank_patch(obj.get("rank"), i)?,
        percent: parse_opt_bool(obj, i, "percent")?,
        bottom: parse_opt_bool(obj, i, "bottom")?,
        above_average: parse_cf_bool_patch(obj.get("aboveAverage"), i, "aboveAverage")?,
        equal_average: parse_opt_bool(obj, i, "equalAverage")?,
        std_dev: parse_cf_std_dev_patch(obj.get("stdDev"), i)?,
    };
    if !op.has_patch() {
        return Err(Error::with_path(
            ErrorKind::NeedSetConditionalFormatField,
            format!("ops[{i}]"),
        ));
    }
    Ok(op)
}

pub(super) fn parse_delete_conditional_format(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteConditionalFormatOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let (sheet, index) = parse_cf_id(&id, i)?;
    Ok(DeleteConditionalFormatOp { sheet, index })
}

pub(super) fn parse_insert_sheet_table(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertSheetTableOp, Error> {
    deny_keys(obj, i, &["op", "id", "ref", "name", "columns"])?;
    let id = require_id(obj, i)?;
    let sheet = parse_sheet_only_id(&id, i, "id", IdReason::SheetOpNeedsSheet)?;
    let range = parse_table_ref(obj.get("ref"), i, true)?.ok_or_else(|| {
        Error::with_path(
            ErrorKind::MissingField {
                field: "ref".into(),
            },
            format!("ops[{i}].ref"),
        )
    })?;
    Ok(InsertSheetTableOp {
        sheet,
        range,
        name: parse_table_name(obj.get("name"), i)?,
        columns: parse_table_columns(obj.get("columns"), i)?,
    })
}

pub(super) fn parse_set_sheet_table(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetSheetTableOp, Error> {
    deny_keys(obj, i, &["op", "id", "ref", "name", "columns"])?;
    let id = require_id(obj, i)?;
    let (sheet, index) = parse_table_id(&id, i)?;
    let op = SetSheetTableOp {
        sheet,
        index,
        range: parse_table_ref(obj.get("ref"), i, false)?,
        name: parse_table_name(obj.get("name"), i)?,
        columns: parse_table_columns(obj.get("columns"), i)?,
    };
    if !op.has_patch() {
        return Err(Error::with_path(
            ErrorKind::NeedSetSheetTableField,
            format!("ops[{i}]"),
        ));
    }
    Ok(op)
}

pub(super) fn parse_delete_sheet_table(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteSheetTableOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let (sheet, index) = parse_table_id(&id, i)?;
    Ok(DeleteSheetTableOp { sheet, index })
}

pub(super) fn parse_insert_pivot_table(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertPivotTableOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op", "id", "source", "location", "name", "rows", "cols", "filters", "values",
        ],
    )?;
    let id = require_id(obj, i)?;
    let sheet = parse_sheet_only_id(&id, i, "id", IdReason::SheetOpNeedsSheet)?;
    let source = parse_pivot_source(obj.get("source"), i, true)?.ok_or_else(|| {
        Error::with_path(
            ErrorKind::MissingField {
                field: "source".into(),
            },
            format!("ops[{i}].source"),
        )
    })?;
    let location = parse_pivot_location(obj.get("location"), i, true)?.ok_or_else(|| {
        Error::with_path(
            ErrorKind::MissingField {
                field: "location".into(),
            },
            format!("ops[{i}].location"),
        )
    })?;
    let values = parse_pivot_fields(obj.get("values"), i, "values", true)?.ok_or_else(|| {
        Error::with_path(
            ErrorKind::MissingField {
                field: "values".into(),
            },
            format!("ops[{i}].values"),
        )
    })?;
    if values.is_empty() {
        return Err(Error::with_path(
            ErrorKind::InvalidPivotTable {
                field: "values".into(),
            },
            format!("ops[{i}].values"),
        ));
    }
    Ok(InsertPivotTableOp {
        sheet,
        source,
        location,
        name: parse_pivot_name(obj.get("name"), i)?,
        rows: parse_pivot_fields(obj.get("rows"), i, "rows", false)?.unwrap_or_default(),
        cols: parse_pivot_fields(obj.get("cols"), i, "cols", false)?.unwrap_or_default(),
        filters: parse_pivot_fields(obj.get("filters"), i, "filters", false)?.unwrap_or_default(),
        values,
    })
}

pub(super) fn parse_set_pivot_table(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetPivotTableOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op", "id", "source", "location", "name", "rows", "cols", "filters", "values",
        ],
    )?;
    let id = require_id(obj, i)?;
    let (sheet, index) = parse_pivot_id(&id, i)?;
    let values = parse_pivot_fields(obj.get("values"), i, "values", false)?;
    if values.as_ref().is_some_and(|v| v.is_empty()) {
        return Err(Error::with_path(
            ErrorKind::InvalidPivotTable {
                field: "values".into(),
            },
            format!("ops[{i}].values"),
        ));
    }
    let op = SetPivotTableOp {
        sheet,
        index,
        source: parse_pivot_source(obj.get("source"), i, false)?,
        location: parse_pivot_location(obj.get("location"), i, false)?,
        name: parse_pivot_name(obj.get("name"), i)?,
        rows: parse_pivot_fields(obj.get("rows"), i, "rows", false)?,
        cols: parse_pivot_fields(obj.get("cols"), i, "cols", false)?,
        filters: parse_pivot_fields(obj.get("filters"), i, "filters", false)?,
        values,
    };
    if !op.has_patch() {
        return Err(Error::with_path(
            ErrorKind::NeedSetPivotTableField,
            format!("ops[{i}]"),
        ));
    }
    Ok(op)
}

pub(super) fn parse_delete_pivot_table(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeletePivotTableOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let (sheet, index) = parse_pivot_id(&id, i)?;
    Ok(DeletePivotTableOp { sheet, index })
}

pub(super) fn parse_pivot_name(v: Option<&Value>, i: usize) -> Result<Option<String>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Null) => Err(Error::with_path(
            ErrorKind::InvalidPivotTable {
                field: "name".into(),
            },
            format!("ops[{i}].name"),
        )),
        Some(Value::String(s)) => {
            if s.is_empty() || s.chars().any(char::is_whitespace) {
                return Err(Error::with_path(
                    ErrorKind::InvalidPivotTable {
                        field: "name".into(),
                    },
                    format!("ops[{i}].name"),
                ));
            }
            Ok(Some(s.clone()))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "name".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].name"),
        )),
    }
}

pub(super) fn parse_pivot_id(id: &str, op_index: usize) -> Result<(String, usize), Error> {
    let Some(rest) = id.strip_prefix("sheet:") else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SheetPivotPrefix,
            },
            format!("ops[{op_index}].id"),
        ));
    };
    let Some((name, nstr)) = rest.split_once("/pivot:") else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SheetPivotPrefix,
            },
            format!("ops[{op_index}].id"),
        ));
    };
    if name.is_empty() || name.contains('/') {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SheetPivotPrefix,
            },
            format!("ops[{op_index}].id"),
        ));
    }
    if nstr.is_empty() || !nstr.chars().all(|c| c.is_ascii_digit()) {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::PivotIndexNotInt,
            },
            format!("ops[{op_index}].id"),
        ));
    }
    let n: usize = nstr.parse().map_err(|_| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::PivotIndexNotInt,
            },
            format!("ops[{op_index}].id"),
        )
    })?;
    if n == 0 {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::PivotIndexFromOne,
            },
            format!("ops[{op_index}].id"),
        ));
    }
    Ok((name.to_string(), n))
}

pub(super) fn parse_pivot_source(
    v: Option<&Value>,
    i: usize,
    required: bool,
) -> Result<Option<PivotSource>, Error> {
    match v {
        None if required => Err(Error::with_path(
            ErrorKind::MissingField {
                field: "source".into(),
            },
            format!("ops[{i}].source"),
        )),
        None => Ok(None),
        Some(Value::Null) => Err(Error::with_path(
            ErrorKind::InvalidPivotTable {
                field: "source".into(),
            },
            format!("ops[{i}].source"),
        )),
        Some(Value::Object(obj)) => {
            deny_keys_at(obj, i, "source", &["sheet", "ref"])?;
            let sheet = match obj.get("sheet") {
                None => None,
                Some(Value::Null) => {
                    return Err(Error::with_path(
                        ErrorKind::InvalidPivotTable {
                            field: "source".into(),
                        },
                        format!("ops[{i}].source.sheet"),
                    ));
                }
                Some(Value::String(s)) if s.is_empty() => {
                    return Err(Error::with_path(
                        ErrorKind::InvalidPivotTable {
                            field: "source".into(),
                        },
                        format!("ops[{i}].source.sheet"),
                    ));
                }
                Some(Value::String(s)) => Some(s.clone()),
                Some(_) => {
                    return Err(Error::with_path(
                        ErrorKind::WrongType {
                            field: "sheet".into(),
                            expected: TypeName::String,
                        },
                        format!("ops[{i}].source.sheet"),
                    ));
                }
            };
            let refer = match obj.get("ref") {
                Some(Value::String(s)) => s.as_str(),
                None => {
                    return Err(Error::with_path(
                        ErrorKind::MissingField {
                            field: "ref".into(),
                        },
                        format!("ops[{i}].source.ref"),
                    ));
                }
                Some(_) => {
                    return Err(Error::with_path(
                        ErrorKind::WrongType {
                            field: "ref".into(),
                            expected: TypeName::String,
                        },
                        format!("ops[{i}].source.ref"),
                    ));
                }
            };
            let (start, end) =
                a1::parse_range(refer).map_err(|e| e.at(format!("ops[{i}].source.ref")))?;
            if start.row == end.row {
                return Err(Error::with_path(
                    ErrorKind::InvalidPivotTable {
                        field: "source".into(),
                    },
                    format!("ops[{i}].source.ref"),
                ));
            }
            Ok(Some(PivotSource {
                sheet,
                range: (start, end),
            }))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "source".into(),
                expected: TypeName::Object,
            },
            format!("ops[{i}].source"),
        )),
    }
}

pub(super) fn parse_pivot_location(
    v: Option<&Value>,
    i: usize,
    required: bool,
) -> Result<Option<a1::CellRef>, Error> {
    match v {
        None if required => Err(Error::with_path(
            ErrorKind::MissingField {
                field: "location".into(),
            },
            format!("ops[{i}].location"),
        )),
        None => Ok(None),
        Some(Value::Null) => Err(Error::with_path(
            ErrorKind::InvalidPivotTable {
                field: "location".into(),
            },
            format!("ops[{i}].location"),
        )),
        Some(Value::String(s)) => {
            let (start, _) = a1::parse_range(s).map_err(|e| e.at(format!("ops[{i}].location")))?;
            Ok(Some(start))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "location".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].location"),
        )),
    }
}

pub(super) fn parse_pivot_fields(
    v: Option<&Value>,
    i: usize,
    field: &str,
    required: bool,
) -> Result<Option<Vec<String>>, Error> {
    match v {
        None if required => Err(Error::with_path(
            ErrorKind::MissingField {
                field: field.into(),
            },
            format!("ops[{i}].{field}"),
        )),
        None => Ok(None),
        Some(Value::Null) => Err(Error::with_path(
            ErrorKind::InvalidPivotTable {
                field: field.into(),
            },
            format!("ops[{i}].{field}"),
        )),
        Some(Value::Array(arr)) => {
            let mut out = Vec::new();
            for (j, item) in arr.iter().enumerate() {
                let Some(s) = item.as_str() else {
                    return Err(Error::with_path(
                        ErrorKind::WrongType {
                            field: field.into(),
                            expected: TypeName::String,
                        },
                        format!("ops[{i}].{field}[{j}]"),
                    ));
                };
                if s.is_empty() {
                    return Err(Error::with_path(
                        ErrorKind::InvalidPivotTable {
                            field: field.into(),
                        },
                        format!("ops[{i}].{field}[{j}]"),
                    ));
                }
                out.push(s.to_string());
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

pub(super) fn parse_table_id(id: &str, op_index: usize) -> Result<(String, usize), Error> {
    let Some(rest) = id.strip_prefix("sheet:") else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SheetTablePrefix,
            },
            format!("ops[{op_index}].id"),
        ));
    };
    let Some((name, nstr)) = rest.split_once("/table:") else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SheetTablePrefix,
            },
            format!("ops[{op_index}].id"),
        ));
    };
    if name.is_empty() || name.contains('/') {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SheetTablePrefix,
            },
            format!("ops[{op_index}].id"),
        ));
    }
    if nstr.is_empty() || !nstr.chars().all(|c| c.is_ascii_digit()) {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::TableIndexNotInt,
            },
            format!("ops[{op_index}].id"),
        ));
    }
    let n: usize = nstr.parse().map_err(|_| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::TableIndexNotInt,
            },
            format!("ops[{op_index}].id"),
        )
    })?;
    if n == 0 {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::TableIndexFromOne,
            },
            format!("ops[{op_index}].id"),
        ));
    }
    Ok((name.to_string(), n))
}

pub(super) fn parse_table_ref(
    v: Option<&Value>,
    i: usize,
    required: bool,
) -> Result<Option<(a1::CellRef, a1::CellRef)>, Error> {
    match v {
        None if required => Err(Error::with_path(
            ErrorKind::MissingField {
                field: "ref".into(),
            },
            format!("ops[{i}].ref"),
        )),
        None => Ok(None),
        Some(Value::Null) => Err(Error::with_path(
            ErrorKind::InvalidSheetTable {
                field: "ref".into(),
            },
            format!("ops[{i}].ref"),
        )),
        Some(Value::String(s)) => {
            let (start, end) = a1::parse_range(s).map_err(|e| e.at(format!("ops[{i}].ref")))?;
            if start.row == end.row {
                return Err(Error::with_path(
                    ErrorKind::InvalidSheetTable {
                        field: "ref".into(),
                    },
                    format!("ops[{i}].ref"),
                ));
            }
            Ok(Some((start, end)))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "ref".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].ref"),
        )),
    }
}

pub(super) fn parse_table_name(v: Option<&Value>, i: usize) -> Result<Option<String>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Null) => Err(Error::with_path(
            ErrorKind::InvalidSheetTable {
                field: "name".into(),
            },
            format!("ops[{i}].name"),
        )),
        Some(Value::String(s)) => {
            if s.is_empty() || s.chars().any(char::is_whitespace) {
                return Err(Error::with_path(
                    ErrorKind::InvalidSheetTable {
                        field: "name".into(),
                    },
                    format!("ops[{i}].name"),
                ));
            }
            Ok(Some(s.clone()))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "name".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].name"),
        )),
    }
}

pub(super) fn parse_table_columns(
    v: Option<&Value>,
    i: usize,
) -> Result<Option<Vec<String>>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Null) => Err(Error::with_path(
            ErrorKind::InvalidSheetTable {
                field: "columns".into(),
            },
            format!("ops[{i}].columns"),
        )),
        Some(Value::Array(arr)) if arr.is_empty() => Err(Error::with_path(
            ErrorKind::InvalidSheetTable {
                field: "columns".into(),
            },
            format!("ops[{i}].columns"),
        )),
        Some(Value::Array(arr)) => {
            let mut out = Vec::new();
            for (j, item) in arr.iter().enumerate() {
                let Some(s) = item.as_str() else {
                    return Err(Error::with_path(
                        ErrorKind::WrongType {
                            field: "columns".into(),
                            expected: TypeName::String,
                        },
                        format!("ops[{i}].columns[{j}]"),
                    ));
                };
                if s.is_empty() {
                    return Err(Error::with_path(
                        ErrorKind::InvalidSheetTable {
                            field: "columns".into(),
                        },
                        format!("ops[{i}].columns[{j}]"),
                    ));
                }
                out.push(s.to_string());
            }
            if has_table_col_dup(&out) {
                return Err(Error::with_path(
                    ErrorKind::InvalidSheetTable {
                        field: "columns".into(),
                    },
                    format!("ops[{i}].columns"),
                ));
            }
            Ok(Some(out))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "columns".into(),
                expected: TypeName::Array,
            },
            format!("ops[{i}].columns"),
        )),
    }
}

pub(super) fn has_table_col_dup(cols: &[String]) -> bool {
    let mut seen = Vec::new();
    for c in cols {
        let k = c.to_ascii_lowercase();
        if seen.iter().any(|s: &String| s == &k) {
            return true;
        }
        seen.push(k);
    }
    false
}

pub(super) fn parse_cf_id(id: &str, op_index: usize) -> Result<(String, usize), Error> {
    let Some(rest) = id.strip_prefix("sheet:") else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SheetCfPrefix,
            },
            format!("ops[{op_index}].id"),
        ));
    };
    let Some((name, nstr)) = rest.split_once("/cf:") else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SheetCfPrefix,
            },
            format!("ops[{op_index}].id"),
        ));
    };
    if name.is_empty() || name.contains('/') {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SheetCfPrefix,
            },
            format!("ops[{op_index}].id"),
        ));
    }
    if !nstr.chars().all(|c| c.is_ascii_digit()) {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::CfIndexNotInt,
            },
            format!("ops[{op_index}].id"),
        ));
    }
    let n: usize = nstr.parse().map_err(|_| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::CfIndexNotInt,
            },
            format!("ops[{op_index}].id"),
        )
    })?;
    if n == 0 {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::CfIndexFromOne,
            },
            format!("ops[{op_index}].id"),
        ));
    }
    Ok((name.to_string(), n))
}

pub(super) fn parse_cf_type_field(
    v: Option<&Value>,
    i: usize,
    required: bool,
) -> Result<Option<CfType>, Error> {
    match v {
        None if required => Err(Error::with_path(
            ErrorKind::MissingField {
                field: "type".into(),
            },
            format!("ops[{i}].type"),
        )),
        None => Ok(None),
        Some(Value::String(s)) => CfType::parse(s).map(Some).ok_or_else(|| {
            Error::with_path(
                ErrorKind::InvalidConditionalFormat {
                    field: "type".into(),
                },
                format!("ops[{i}].type"),
            )
        }),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "type".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].type"),
        )),
    }
}

pub(super) fn parse_cf_operator_field(
    v: Option<&Value>,
    i: usize,
    required: bool,
) -> Result<Option<CfOperator>, Error> {
    match v {
        None if required => Err(Error::with_path(
            ErrorKind::MissingField {
                field: "operator".into(),
            },
            format!("ops[{i}].operator"),
        )),
        None => Ok(None),
        Some(Value::String(s)) => CfOperator::parse(s).map(Some).ok_or_else(|| {
            Error::with_path(
                ErrorKind::InvalidConditionalFormat {
                    field: "operator".into(),
                },
                format!("ops[{i}].operator"),
            )
        }),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "operator".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].operator"),
        )),
    }
}

pub(super) fn parse_cf_operator_patch(
    v: Option<&Value>,
    i: usize,
) -> Result<Patch<CfOperator>, Error> {
    match v {
        None => Ok(Patch::Absent),
        Some(Value::Null) => Ok(Patch::Clear),
        Some(Value::String(s)) => CfOperator::parse(s).map(Patch::Set).ok_or_else(|| {
            Error::with_path(
                ErrorKind::InvalidConditionalFormat {
                    field: "operator".into(),
                },
                format!("ops[{i}].operator"),
            )
        }),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "operator".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].operator"),
        )),
    }
}

pub(super) fn parse_cf_formulas_opt(
    v: Option<&Value>,
    i: usize,
) -> Result<Option<Vec<String>>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Array(arr)) if arr.is_empty() => Err(Error::with_path(
            ErrorKind::InvalidConditionalFormat {
                field: "formulas".into(),
            },
            format!("ops[{i}].formulas"),
        )),
        Some(Value::Array(arr)) => {
            let mut out = Vec::new();
            for (j, item) in arr.iter().enumerate() {
                let Some(s) = item.as_str() else {
                    return Err(Error::with_path(
                        ErrorKind::WrongType {
                            field: "formulas".into(),
                            expected: TypeName::String,
                        },
                        format!("ops[{i}].formulas[{j}]"),
                    ));
                };
                out.push(strip_formula_eq(s));
            }
            if out.len() > 3 {
                return Err(Error::with_path(
                    ErrorKind::InvalidConditionalFormat {
                        field: "formulas".into(),
                    },
                    format!("ops[{i}].formulas"),
                ));
            }
            Ok(Some(out))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "formulas".into(),
                expected: TypeName::Array,
            },
            format!("ops[{i}].formulas"),
        )),
    }
}

pub(super) fn parse_cf_formulas_patch(
    v: Option<&Value>,
    i: usize,
) -> Result<Patch<Vec<String>>, Error> {
    match v {
        None => Ok(Patch::Absent),
        Some(Value::Null) => Ok(Patch::Clear),
        Some(other) => Ok(Patch::Set(
            parse_cf_formulas_opt(Some(other), i)?.ok_or_else(|| {
                Error::with_path(
                    ErrorKind::InvalidConditionalFormat {
                        field: "formulas".into(),
                    },
                    format!("ops[{i}].formulas"),
                )
            })?,
        )),
    }
}

pub(super) fn parse_cf_text_opt(v: Option<&Value>, i: usize) -> Result<Option<String>, Error> {
    match v {
        None => Ok(None),
        Some(Value::String(s)) if !s.is_empty() => Ok(Some(s.clone())),
        Some(Value::String(_)) => Err(Error::with_path(
            ErrorKind::InvalidConditionalFormat {
                field: "text".into(),
            },
            format!("ops[{i}].text"),
        )),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "text".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].text"),
        )),
    }
}

pub(super) fn parse_cf_text_patch(v: Option<&Value>, i: usize) -> Result<Patch<String>, Error> {
    match v {
        None => Ok(Patch::Absent),
        Some(Value::Null) => Ok(Patch::Clear),
        Some(other) => Ok(Patch::Set(parse_cf_text_opt(Some(other), i)?.ok_or_else(
            || {
                Error::with_path(
                    ErrorKind::InvalidConditionalFormat {
                        field: "text".into(),
                    },
                    format!("ops[{i}].text"),
                )
            },
        )?)),
    }
}

pub(super) fn parse_cf_rank_patch(v: Option<&Value>, i: usize) -> Result<Patch<u32>, Error> {
    match v {
        None => Ok(Patch::Absent),
        Some(Value::Null) => Err(Error::with_path(
            ErrorKind::InvalidConditionalFormat {
                field: "rank".into(),
            },
            format!("ops[{i}].rank"),
        )),
        Some(Value::Number(n)) => {
            let Some(v) = n.as_u64().and_then(|x| u32::try_from(x).ok()) else {
                return Err(Error::with_path(
                    ErrorKind::InvalidConditionalFormat {
                        field: "rank".into(),
                    },
                    format!("ops[{i}].rank"),
                ));
            };
            if v < 1 {
                return Err(Error::with_path(
                    ErrorKind::InvalidConditionalFormat {
                        field: "rank".into(),
                    },
                    format!("ops[{i}].rank"),
                ));
            }
            Ok(Patch::Set(v))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "rank".into(),
                expected: TypeName::Integer,
            },
            format!("ops[{i}].rank"),
        )),
    }
}

pub(super) fn parse_cf_std_dev_opt(v: Option<&Value>, i: usize) -> Result<Option<i32>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Number(n)) => {
            let Some(v) = n.as_i64().and_then(|x| i32::try_from(x).ok()) else {
                return Err(Error::with_path(
                    ErrorKind::InvalidConditionalFormat {
                        field: "stdDev".into(),
                    },
                    format!("ops[{i}].stdDev"),
                ));
            };
            if v < 1 {
                return Err(Error::with_path(
                    ErrorKind::InvalidConditionalFormat {
                        field: "stdDev".into(),
                    },
                    format!("ops[{i}].stdDev"),
                ));
            }
            Ok(Some(v))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "stdDev".into(),
                expected: TypeName::Integer,
            },
            format!("ops[{i}].stdDev"),
        )),
    }
}

pub(super) fn parse_cf_std_dev_patch(v: Option<&Value>, i: usize) -> Result<Patch<i32>, Error> {
    match v {
        None => Ok(Patch::Absent),
        Some(Value::Null) => Ok(Patch::Clear),
        Some(other) => Ok(Patch::Set(
            parse_cf_std_dev_opt(Some(other), i)?.ok_or_else(|| {
                Error::with_path(
                    ErrorKind::InvalidConditionalFormat {
                        field: "stdDev".into(),
                    },
                    format!("ops[{i}].stdDev"),
                )
            })?,
        )),
    }
}

pub(super) fn parse_cf_bool_patch(
    v: Option<&Value>,
    i: usize,
    field: &str,
) -> Result<Patch<bool>, Error> {
    match v {
        None => Ok(Patch::Absent),
        Some(Value::Null) => Ok(Patch::Clear),
        Some(Value::Bool(b)) => Ok(Patch::Set(*b)),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Boolean,
            },
            format!("ops[{i}].{field}"),
        )),
    }
}

pub(super) fn parse_dxf_opt(v: Option<&Value>, i: usize) -> Result<Option<DxfPatch>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Object(m)) => Ok(Some(parse_dxf_obj(m, i)?)),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "dxf".into(),
                expected: TypeName::Object,
            },
            format!("ops[{i}].dxf"),
        )),
    }
}

pub(super) fn parse_dxf_patch_field(v: Option<&Value>, i: usize) -> Result<Patch<DxfPatch>, Error> {
    match v {
        None => Ok(Patch::Absent),
        Some(Value::Null) => Ok(Patch::Clear),
        Some(Value::Object(m)) => Ok(Patch::Set(parse_dxf_obj(m, i)?)),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "dxf".into(),
                expected: TypeName::Object,
            },
            format!("ops[{i}].dxf"),
        )),
    }
}

pub(super) fn parse_dxf_obj(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DxfPatch, Error> {
    deny_sub_keys(obj, i, "dxf", &["font", "fill", "border"])?;
    let font = match obj.get("font") {
        None => None,
        Some(Value::Object(m)) => Some(parse_font_patch(m, i, "dxf.font")?),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "font".into(),
                    expected: TypeName::Object,
                },
                format!("ops[{i}].dxf.font"),
            ));
        }
    };
    let fill = match obj.get("fill") {
        None => None,
        Some(Value::Object(m)) => Some(parse_fill_patch(m, i, "dxf.fill")?),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "fill".into(),
                    expected: TypeName::Object,
                },
                format!("ops[{i}].dxf.fill"),
            ));
        }
    };
    let border = match obj.get("border") {
        None => None,
        Some(Value::Object(m)) => Some(parse_border_patch(m, i, "dxf.border")?),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "border".into(),
                    expected: TypeName::Object,
                },
                format!("ops[{i}].dxf.border"),
            ));
        }
    };
    Ok(DxfPatch { font, fill, border })
}

pub(super) fn parse_color_scale_opt(
    v: Option<&Value>,
    i: usize,
) -> Result<Option<ColorScaleSpec>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Object(m)) => Ok(Some(parse_color_scale_obj(m, i)?)),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "colorScale".into(),
                expected: TypeName::Object,
            },
            format!("ops[{i}].colorScale"),
        )),
    }
}

pub(super) fn parse_color_scale_patch(
    v: Option<&Value>,
    i: usize,
) -> Result<Patch<ColorScaleSpec>, Error> {
    match v {
        None => Ok(Patch::Absent),
        Some(Value::Null) => Ok(Patch::Clear),
        Some(Value::Object(m)) => Ok(Patch::Set(parse_color_scale_obj(m, i)?)),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "colorScale".into(),
                expected: TypeName::Object,
            },
            format!("ops[{i}].colorScale"),
        )),
    }
}

pub(super) fn parse_color_scale_obj(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<ColorScaleSpec, Error> {
    deny_sub_keys(obj, i, "colorScale", &["cfvo", "colors"])?;
    let colors = parse_color_array(obj.get("colors"), i, "colorScale.colors", 2, 3)?;
    let cfvo = parse_cfvo_array_opt(obj.get("cfvo"), i, "colorScale.cfvo", 2, 3)?;
    if let Some(v) = &cfvo {
        if v.len() != colors.len() {
            return Err(Error::with_path(
                ErrorKind::InvalidConditionalFormat {
                    field: "colorScale.cfvo".into(),
                },
                format!("ops[{i}].colorScale.cfvo"),
            ));
        }
    }
    Ok(ColorScaleSpec { cfvo, colors })
}

pub(super) fn parse_data_bar_opt(
    v: Option<&Value>,
    i: usize,
) -> Result<Option<DataBarSpec>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Object(m)) => Ok(Some(parse_data_bar_obj(m, i)?)),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "dataBar".into(),
                expected: TypeName::Object,
            },
            format!("ops[{i}].dataBar"),
        )),
    }
}

pub(super) fn parse_data_bar_patch(
    v: Option<&Value>,
    i: usize,
) -> Result<Patch<DataBarSpec>, Error> {
    match v {
        None => Ok(Patch::Absent),
        Some(Value::Null) => Ok(Patch::Clear),
        Some(Value::Object(m)) => Ok(Patch::Set(parse_data_bar_obj(m, i)?)),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "dataBar".into(),
                expected: TypeName::Object,
            },
            format!("ops[{i}].dataBar"),
        )),
    }
}

pub(super) fn parse_data_bar_obj(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DataBarSpec, Error> {
    deny_sub_keys(
        obj,
        i,
        "dataBar",
        &["cfvo", "color", "minLength", "maxLength", "showValue"],
    )?;
    let cfvo = parse_cfvo_array_opt(obj.get("cfvo"), i, "dataBar.cfvo", 2, 2)?;
    let color = match obj.get("color") {
        None => None,
        Some(Value::Object(m)) => Some(parse_color_spec(m, i, "dataBar.color")?),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "color".into(),
                    expected: TypeName::Object,
                },
                format!("ops[{i}].dataBar.color"),
            ));
        }
    };
    Ok(DataBarSpec {
        cfvo,
        color,
        min_length: parse_opt_u32_top(obj, i, "minLength", "dataBar.minLength", 0, 100)?,
        max_length: parse_opt_u32_top(obj, i, "maxLength", "dataBar.maxLength", 0, 100)?,
        show_value: parse_sub_bool(obj, i, "dataBar", "showValue")?,
    })
}

pub(super) fn parse_icon_set_opt(
    v: Option<&Value>,
    i: usize,
) -> Result<Option<IconSetSpec>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Object(m)) => Ok(Some(parse_icon_set_obj(m, i)?)),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "iconSet".into(),
                expected: TypeName::Object,
            },
            format!("ops[{i}].iconSet"),
        )),
    }
}

pub(super) fn parse_icon_set_patch(
    v: Option<&Value>,
    i: usize,
) -> Result<Patch<IconSetSpec>, Error> {
    match v {
        None => Ok(Patch::Absent),
        Some(Value::Null) => Ok(Patch::Clear),
        Some(Value::Object(m)) => Ok(Patch::Set(parse_icon_set_obj(m, i)?)),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "iconSet".into(),
                expected: TypeName::Object,
            },
            format!("ops[{i}].iconSet"),
        )),
    }
}

pub(super) fn parse_icon_set_obj(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<IconSetSpec, Error> {
    deny_sub_keys(
        obj,
        i,
        "iconSet",
        &["set", "cfvo", "showValue", "reverse", "percent"],
    )?;
    let set = match obj.get("set") {
        None => None,
        Some(Value::String(s)) if ICON_SETS.contains(&s.as_str()) => Some(s.clone()),
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidConditionalFormat {
                    field: "iconSet.set".into(),
                },
                format!("ops[{i}].iconSet.set"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "set".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].iconSet.set"),
            ));
        }
    };
    let n = icon_set_count(set.as_deref().unwrap_or("3TrafficLights1"));
    let cfvo = parse_cfvo_array_opt(obj.get("cfvo"), i, "iconSet.cfvo", n, n)?;
    Ok(IconSetSpec {
        set,
        cfvo,
        show_value: parse_sub_bool(obj, i, "iconSet", "showValue")?,
        reverse: parse_sub_bool(obj, i, "iconSet", "reverse")?,
        percent: parse_sub_bool(obj, i, "iconSet", "percent")?,
    })
}

pub(super) fn parse_cfvo_array_opt(
    v: Option<&Value>,
    i: usize,
    path: &str,
    min: usize,
    max: usize,
) -> Result<Option<Vec<CfvoSpec>>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Array(arr)) => {
            if arr.len() < min || arr.len() > max {
                return Err(Error::with_path(
                    ErrorKind::InvalidConditionalFormat { field: path.into() },
                    format!("ops[{i}].{path}"),
                ));
            }
            let mut out = Vec::new();
            for (j, item) in arr.iter().enumerate() {
                let Some(obj) = item.as_object() else {
                    return Err(Error::with_path(
                        ErrorKind::WrongType {
                            field: "cfvo".into(),
                            expected: TypeName::Object,
                        },
                        format!("ops[{i}].{path}[{j}]"),
                    ));
                };
                out.push(parse_cfvo_obj(obj, i, &format!("{path}[{j}]"))?);
            }
            Ok(Some(out))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "cfvo".into(),
                expected: TypeName::Array,
            },
            format!("ops[{i}].{path}"),
        )),
    }
}

pub(super) fn parse_cfvo_obj(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    prefix: &str,
) -> Result<CfvoSpec, Error> {
    deny_sub_keys(obj, i, prefix, &["type", "val", "gte"])?;
    let type_ = match obj.get("type") {
        Some(Value::String(s)) if CFVO_TYPES.contains(&s.as_str()) => s.clone(),
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidConditionalFormat {
                    field: "cfvo.type".into(),
                },
                format!("ops[{i}].{prefix}.type"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "type".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].{prefix}.type"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "type".into(),
                },
                format!("ops[{i}].{prefix}.type"),
            ));
        }
    };
    let val = match obj.get("val") {
        None => None,
        Some(Value::String(s)) => Some(s.clone()),
        Some(Value::Number(n)) => Some(n.to_string()),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "val".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].{prefix}.val"),
            ));
        }
    };
    Ok(CfvoSpec {
        type_,
        val,
        gte: parse_sub_bool(obj, i, prefix, "gte")?,
    })
}

pub(super) fn parse_color_array(
    v: Option<&Value>,
    i: usize,
    path: &str,
    min: usize,
    max: usize,
) -> Result<Vec<ColorSpec>, Error> {
    let Some(Value::Array(arr)) = v else {
        return Err(Error::with_path(
            if v.is_none() {
                ErrorKind::MissingField {
                    field: "colors".into(),
                }
            } else {
                ErrorKind::WrongType {
                    field: "colors".into(),
                    expected: TypeName::Array,
                }
            },
            format!("ops[{i}].{path}"),
        ));
    };
    if arr.len() < min || arr.len() > max {
        return Err(Error::with_path(
            ErrorKind::InvalidConditionalFormat {
                field: "colors".into(),
            },
            format!("ops[{i}].{path}"),
        ));
    }
    let mut out = Vec::new();
    for (j, item) in arr.iter().enumerate() {
        let Some(obj) = item.as_object() else {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "colors".into(),
                    expected: TypeName::Object,
                },
                format!("ops[{i}].{path}[{j}]"),
            ));
        };
        out.push(parse_color_spec(obj, i, &format!("{path}[{j}]"))?);
    }
    Ok(out)
}

pub(super) fn parse_cf_priority_opt(v: Option<&Value>, i: usize) -> Result<Option<i32>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Null) => Err(Error::with_path(
            ErrorKind::InvalidConditionalFormat {
                field: "priority".into(),
            },
            format!("ops[{i}].priority"),
        )),
        Some(Value::Number(n)) => {
            let Some(v) = n.as_i64().filter(|x| *x >= 1 && *x <= i32::MAX as i64) else {
                return Err(Error::with_path(
                    ErrorKind::InvalidConditionalFormat {
                        field: "priority".into(),
                    },
                    format!("ops[{i}].priority"),
                ));
            };
            Ok(Some(v as i32))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "priority".into(),
                expected: TypeName::Integer,
            },
            format!("ops[{i}].priority"),
        )),
    }
}

pub(super) fn parse_opt_u32_top(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    field: &str,
    path: &str,
    min: u32,
    max: u32,
) -> Result<Option<u32>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::Number(n)) => {
            let Some(v) = n.as_u64().and_then(|x| u32::try_from(x).ok()) else {
                return Err(Error::with_path(
                    ErrorKind::InvalidConditionalFormat {
                        field: field.into(),
                    },
                    format!("ops[{i}].{path}"),
                ));
            };
            if v < min || v > max {
                return Err(Error::with_path(
                    ErrorKind::InvalidConditionalFormat {
                        field: field.into(),
                    },
                    format!("ops[{i}].{path}"),
                ));
            }
            Ok(Some(v))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Integer,
            },
            format!("ops[{i}].{path}"),
        )),
    }
}

pub(crate) fn check_cf_fields(
    type_: CfType,
    operator: Option<CfOperator>,
    formulas: Option<&[String]>,
    dxf: Option<&DxfPatch>,
    color_scale: Option<&ColorScaleSpec>,
    data_bar: Option<&DataBarSpec>,
    icon_set: Option<&IconSetSpec>,
    extra: CfExtraCheck<'_>,
    i: usize,
    insert: bool,
) -> Result<(), Error> {
    let bad = |field: &str| {
        Error::with_path(
            ErrorKind::InvalidConditionalFormat {
                field: field.into(),
            },
            format!("ops[{i}].{field}"),
        )
    };
    let reject_visual = || {
        if color_scale.is_some() {
            Err(bad("colorScale"))
        } else if data_bar.is_some() {
            Err(bad("dataBar"))
        } else if icon_set.is_some() {
            Err(bad("iconSet"))
        } else {
            Ok(())
        }
    };
    match type_ {
        CfType::CellIs => {
            reject_visual()?;
            reject_text_extra(&extra, i)?;
            reject_top10_extra(&extra, i)?;
            reject_avg_extra(&extra, i)?;
            let Some(op) = operator else {
                return Err(Error::with_path(
                    ErrorKind::MissingField {
                        field: "operator".into(),
                    },
                    format!("ops[{i}].operator"),
                ));
            };
            let need = op.formula_count();
            let got = formulas.map(|f| f.len()).unwrap_or(0);
            if got != need {
                return Err(Error::with_path(
                    ErrorKind::NeedConditionalFormatFormula {
                        field: "formulas".into(),
                    },
                    format!("ops[{i}].formulas"),
                ));
            }
        }
        CfType::Expression => {
            if operator.is_some() {
                return Err(bad("operator"));
            }
            reject_visual()?;
            reject_text_extra(&extra, i)?;
            reject_top10_extra(&extra, i)?;
            reject_avg_extra(&extra, i)?;
            if formulas.map(|f| f.len()).unwrap_or(0) != 1 {
                return Err(Error::with_path(
                    ErrorKind::NeedConditionalFormatFormula {
                        field: "formulas".into(),
                    },
                    format!("ops[{i}].formulas"),
                ));
            }
        }
        CfType::ColorScale => {
            if operator.is_some() {
                return Err(bad("operator"));
            }
            if formulas.is_some() {
                return Err(bad("formulas"));
            }
            if dxf.is_some() {
                return Err(bad("dxf"));
            }
            if data_bar.is_some() {
                return Err(bad("dataBar"));
            }
            if icon_set.is_some() {
                return Err(bad("iconSet"));
            }
            reject_text_extra(&extra, i)?;
            reject_top10_extra(&extra, i)?;
            reject_avg_extra(&extra, i)?;
            if insert && color_scale.is_none() {
                return Err(Error::with_path(
                    ErrorKind::MissingField {
                        field: "colorScale".into(),
                    },
                    format!("ops[{i}].colorScale"),
                ));
            }
        }
        CfType::DataBar => {
            if operator.is_some() {
                return Err(bad("operator"));
            }
            if formulas.is_some() {
                return Err(bad("formulas"));
            }
            if dxf.is_some() {
                return Err(bad("dxf"));
            }
            if color_scale.is_some() {
                return Err(bad("colorScale"));
            }
            if icon_set.is_some() {
                return Err(bad("iconSet"));
            }
            reject_text_extra(&extra, i)?;
            reject_top10_extra(&extra, i)?;
            reject_avg_extra(&extra, i)?;
        }
        CfType::IconSet => {
            if operator.is_some() {
                return Err(bad("operator"));
            }
            if formulas.is_some() {
                return Err(bad("formulas"));
            }
            if dxf.is_some() {
                return Err(bad("dxf"));
            }
            if color_scale.is_some() {
                return Err(bad("colorScale"));
            }
            if data_bar.is_some() {
                return Err(bad("dataBar"));
            }
            reject_text_extra(&extra, i)?;
            reject_top10_extra(&extra, i)?;
            reject_avg_extra(&extra, i)?;
        }
        CfType::Top10 => {
            if operator.is_some() {
                return Err(bad("operator"));
            }
            if formulas.is_some() {
                return Err(bad("formulas"));
            }
            reject_visual()?;
            reject_text_extra(&extra, i)?;
            reject_avg_extra(&extra, i)?;
        }
        CfType::AboveAverage => {
            if operator.is_some() {
                return Err(bad("operator"));
            }
            if formulas.is_some() {
                return Err(bad("formulas"));
            }
            reject_visual()?;
            reject_text_extra(&extra, i)?;
            reject_top10_extra(&extra, i)?;
        }
        CfType::ContainsText | CfType::NotContainsText | CfType::BeginsWith | CfType::EndsWith => {
            if operator.is_some() {
                return Err(bad("operator"));
            }
            reject_visual()?;
            reject_top10_extra(&extra, i)?;
            reject_avg_extra(&extra, i)?;
            if insert && extra.text.is_none() {
                return Err(Error::with_path(
                    ErrorKind::MissingField {
                        field: "text".into(),
                    },
                    format!("ops[{i}].text"),
                ));
            }
            if let Some(fs) = formulas {
                if fs.len() != 1 {
                    return Err(Error::with_path(
                        ErrorKind::NeedConditionalFormatFormula {
                            field: "formulas".into(),
                        },
                        format!("ops[{i}].formulas"),
                    ));
                }
            }
        }
    }
    Ok(())
}

pub(super) fn reject_text_extra(extra: &CfExtraCheck<'_>, i: usize) -> Result<(), Error> {
    if extra.text.is_some() {
        return Err(Error::with_path(
            ErrorKind::InvalidConditionalFormat {
                field: "text".into(),
            },
            format!("ops[{i}].text"),
        ));
    }
    Ok(())
}

pub(super) fn reject_top10_extra(extra: &CfExtraCheck<'_>, i: usize) -> Result<(), Error> {
    if extra.rank.is_some() {
        return Err(Error::with_path(
            ErrorKind::InvalidConditionalFormat {
                field: "rank".into(),
            },
            format!("ops[{i}].rank"),
        ));
    }
    if extra.percent.is_some() {
        return Err(Error::with_path(
            ErrorKind::InvalidConditionalFormat {
                field: "percent".into(),
            },
            format!("ops[{i}].percent"),
        ));
    }
    if extra.bottom.is_some() {
        return Err(Error::with_path(
            ErrorKind::InvalidConditionalFormat {
                field: "bottom".into(),
            },
            format!("ops[{i}].bottom"),
        ));
    }
    Ok(())
}

pub(super) fn reject_avg_extra(extra: &CfExtraCheck<'_>, i: usize) -> Result<(), Error> {
    if extra.above_average.is_some() {
        return Err(Error::with_path(
            ErrorKind::InvalidConditionalFormat {
                field: "aboveAverage".into(),
            },
            format!("ops[{i}].aboveAverage"),
        ));
    }
    if extra.equal_average.is_some() {
        return Err(Error::with_path(
            ErrorKind::InvalidConditionalFormat {
                field: "equalAverage".into(),
            },
            format!("ops[{i}].equalAverage"),
        ));
    }
    if extra.std_dev.is_some() {
        return Err(Error::with_path(
            ErrorKind::InvalidConditionalFormat {
                field: "stdDev".into(),
            },
            format!("ops[{i}].stdDev"),
        ));
    }
    Ok(())
}

pub(super) fn parse_dv_id(id: &str, op_index: usize) -> Result<(String, usize), Error> {
    let Some(rest) = id.strip_prefix("sheet:") else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SheetDvPrefix,
            },
            format!("ops[{op_index}].id"),
        ));
    };
    let Some((name, nstr)) = rest.split_once("/dv:") else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SheetDvPrefix,
            },
            format!("ops[{op_index}].id"),
        ));
    };
    if name.is_empty() || name.contains('/') {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SheetDvPrefix,
            },
            format!("ops[{op_index}].id"),
        ));
    }
    if nstr.is_empty() || !nstr.bytes().all(|b| b.is_ascii_digit()) {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::DvIndexNotInt,
            },
            format!("ops[{op_index}].id"),
        ));
    }
    let n: usize = nstr.parse().map_err(|_| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::DvIndexNotInt,
            },
            format!("ops[{op_index}].id"),
        )
    })?;
    if n == 0 {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::DvIndexFromOne,
            },
            format!("ops[{op_index}].id"),
        ));
    }
    Ok((name.to_string(), n))
}

pub(super) fn parse_sqref_field(
    v: Option<&Value>,
    i: usize,
    required: bool,
) -> Result<Vec<(a1::CellRef, a1::CellRef)>, Error> {
    let Some(v) = v else {
        if required {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "sqref".into(),
                },
                format!("ops[{i}].sqref"),
            ));
        }
        return Ok(Vec::new());
    };
    let parts: Vec<&str> = match v {
        Value::String(s) => s.split_whitespace().collect(),
        Value::Array(arr) => {
            let mut out = Vec::new();
            for (j, item) in arr.iter().enumerate() {
                let Some(s) = item.as_str() else {
                    return Err(Error::with_path(
                        ErrorKind::WrongType {
                            field: "sqref".into(),
                            expected: TypeName::String,
                        },
                        format!("ops[{i}].sqref[{j}]"),
                    ));
                };
                if s.trim().is_empty() {
                    return Err(Error::with_path(
                        ErrorKind::InvalidRange {
                            reason: crate::error::RangeReason::Empty,
                        },
                        format!("ops[{i}].sqref[{j}]"),
                    ));
                }
                out.push(s);
            }
            return parse_sqref_parts(&out, i);
        }
        _ => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "sqref".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].sqref"),
            ));
        }
    };
    if parts.is_empty() {
        return Err(Error::with_path(
            ErrorKind::InvalidRange {
                reason: crate::error::RangeReason::Empty,
            },
            format!("ops[{i}].sqref"),
        ));
    }
    parse_sqref_parts(&parts, i)
}

pub(super) fn parse_sqref_parts(
    parts: &[&str],
    i: usize,
) -> Result<Vec<(a1::CellRef, a1::CellRef)>, Error> {
    let mut out = Vec::new();
    for p in parts {
        let range = a1::parse_range(p).map_err(|e| e.at(format!("ops[{i}].sqref")))?;
        out.push(range);
    }
    Ok(out)
}

pub(super) fn parse_dv_type_field(
    v: Option<&Value>,
    i: usize,
    required: bool,
) -> Result<Option<DvType>, Error> {
    match v {
        None if required => Err(Error::with_path(
            ErrorKind::MissingField {
                field: "type".into(),
            },
            format!("ops[{i}].type"),
        )),
        None => Ok(None),
        Some(Value::String(s)) => DvType::parse(s).map(Some).ok_or_else(|| {
            Error::with_path(
                ErrorKind::InvalidDataValidation {
                    field: "type".into(),
                },
                format!("ops[{i}].type"),
            )
        }),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "type".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].type"),
        )),
    }
}

pub(super) fn parse_dv_operator_field(
    v: Option<&Value>,
    i: usize,
    _required: bool,
) -> Result<Option<DvOperator>, Error> {
    match v {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => DvOperator::parse(s).map(Some).ok_or_else(|| {
            Error::with_path(
                ErrorKind::InvalidDataValidation {
                    field: "operator".into(),
                },
                format!("ops[{i}].operator"),
            )
        }),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "operator".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].operator"),
        )),
    }
}

pub(super) fn parse_dv_operator_patch(
    v: Option<&Value>,
    i: usize,
) -> Result<Patch<DvOperator>, Error> {
    match v {
        None => Ok(Patch::Absent),
        Some(Value::Null) => Ok(Patch::Clear),
        Some(Value::String(s)) => {
            let op = DvOperator::parse(s).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::InvalidDataValidation {
                        field: "operator".into(),
                    },
                    format!("ops[{i}].operator"),
                )
            })?;
            Ok(Patch::Set(op))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "operator".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].operator"),
        )),
    }
}

pub(super) fn strip_formula_eq(s: &str) -> String {
    let t = s.trim();
    t.strip_prefix('=')
        .map(|r| r.trim_start().to_string())
        .unwrap_or_else(|| t.to_string())
}

pub(super) fn parse_dv_formula_opt(
    v: Option<&Value>,
    i: usize,
    field: &str,
) -> Result<Option<String>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => {
            let t = strip_formula_eq(s);
            if t.is_empty() {
                Ok(None)
            } else {
                Ok(Some(t))
            }
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].{field}"),
        )),
    }
}

pub(super) fn parse_dv_formula_patch(
    v: Option<&Value>,
    i: usize,
    field: &str,
) -> Result<Patch<String>, Error> {
    match v {
        None => Ok(Patch::Absent),
        Some(Value::Null) => Ok(Patch::Clear),
        Some(Value::String(s)) => {
            let t = strip_formula_eq(s);
            if t.is_empty() {
                Ok(Patch::Clear)
            } else {
                Ok(Patch::Set(t))
            }
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].{field}"),
        )),
    }
}

pub(super) fn parse_dv_string_opt(
    v: Option<&Value>,
    i: usize,
    field: &str,
) -> Result<Option<String>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].{field}"),
        )),
    }
}

pub(super) fn parse_dv_string_patch(
    v: Option<&Value>,
    i: usize,
    field: &str,
) -> Result<Patch<String>, Error> {
    match v {
        None => Ok(Patch::Absent),
        Some(Value::Null) => Ok(Patch::Clear),
        Some(Value::String(s)) => Ok(Patch::Set(s.clone())),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].{field}"),
        )),
    }
}

pub(super) fn parse_dv_error_style_opt(
    v: Option<&Value>,
    i: usize,
) -> Result<Option<String>, Error> {
    match v {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) if matches!(s.as_str(), "stop" | "warning" | "information") => {
            Ok(Some(s.clone()))
        }
        Some(Value::String(_)) => Err(Error::with_path(
            ErrorKind::InvalidDataValidation {
                field: "errorStyle".into(),
            },
            format!("ops[{i}].errorStyle"),
        )),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "errorStyle".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].errorStyle"),
        )),
    }
}

pub(super) fn parse_dv_error_style_patch(
    v: Option<&Value>,
    i: usize,
) -> Result<Patch<String>, Error> {
    match v {
        None => Ok(Patch::Absent),
        Some(Value::Null) => Ok(Patch::Clear),
        Some(Value::String(s)) if matches!(s.as_str(), "stop" | "warning" | "information") => {
            Ok(Patch::Set(s.clone()))
        }
        Some(Value::String(_)) => Err(Error::with_path(
            ErrorKind::InvalidDataValidation {
                field: "errorStyle".into(),
            },
            format!("ops[{i}].errorStyle"),
        )),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "errorStyle".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].errorStyle"),
        )),
    }
}

pub(crate) fn check_dv_fields(
    type_: DvType,
    operator: Option<DvOperator>,
    formula1: Option<&str>,
    formula2: Option<&str>,
    dropdown: Option<bool>,
    i: usize,
) -> Result<(), Error> {
    if dropdown.is_some() && type_ != DvType::List {
        return Err(Error::with_path(
            ErrorKind::InvalidDataValidation {
                field: "dropdown".into(),
            },
            format!("ops[{i}].dropdown"),
        ));
    }
    if operator.is_some() && !type_.allows_operator() {
        return Err(Error::with_path(
            ErrorKind::InvalidDataValidation {
                field: "operator".into(),
            },
            format!("ops[{i}].operator"),
        ));
    }
    let f1_ok = formula1.is_some_and(|s| !s.is_empty());
    if type_.needs_formula1() && !f1_ok {
        return Err(Error::with_path(
            ErrorKind::NeedDataValidationFormula {
                field: "formula1".into(),
            },
            format!("ops[{i}].formula1"),
        ));
    }
    let needs_f2 =
        type_.allows_operator() && operator.unwrap_or(DvOperator::Between).needs_formula2();
    let f2_ok = formula2.is_some_and(|s| !s.is_empty());
    if needs_f2 && !f2_ok {
        return Err(Error::with_path(
            ErrorKind::NeedDataValidationFormula {
                field: "formula2".into(),
            },
            format!("ops[{i}].formula2"),
        ));
    }
    if !needs_f2 && f2_ok {
        return Err(Error::with_path(
            ErrorKind::InvalidDataValidation {
                field: "formula2".into(),
            },
            format!("ops[{i}].formula2"),
        ));
    }
    Ok(())
}
