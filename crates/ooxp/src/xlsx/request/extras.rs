//! xlsx request：extras。

use serde_json::Value;

use crate::error::{Error, ErrorKind, IdReason, TypeName};

use super::super::a1;
use super::super::connection;
use super::super::custom_view;
use super::super::extlink;
use super::super::ole;
use super::super::picture::{self};
use super::super::slicer;

use super::cell::*;
use super::common::*;
use super::hyperlink::*;
use super::table_data::*;
use super::types::*;

pub(super) fn parse_insert_sparkline(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertSparklineOp, Error> {
    deny_keys(obj, i, SPARK_KEYS)?;
    let id = require_id(obj, i)?;
    let sheet = parse_sheet_only_id(&id, i, "id", IdReason::InsertSparklineNeedsSheet)?;
    let (refer, host) = parse_required_spark_host(obj, i)?;
    let formula = parse_required_spark_formula(obj, i)?;
    let min_axis = parse_spark_axis(obj, i, "minAxisType")?;
    let max_axis = parse_spark_axis(obj, i, "maxAxisType")?;
    let manual_min = parse_opt_f64_top(obj, i, "manualMin")?;
    let manual_max = parse_opt_f64_top(obj, i, "manualMax")?;
    check_spark_axis(min_axis, max_axis, manual_min, manual_max, i)?;
    Ok(InsertSparklineOp {
        sheet,
        refer,
        host,
        formula,
        kind: parse_spark_type(obj, i)?.unwrap_or(SparklineType::Line),
        display_empty: parse_spark_empty(obj, i)?.unwrap_or(SparkEmpty::Gap),
        markers: parse_opt_bool(obj, i, "markers")?.unwrap_or(false),
        high: parse_opt_bool(obj, i, "high")?.unwrap_or(false),
        low: parse_opt_bool(obj, i, "low")?.unwrap_or(false),
        first: parse_opt_bool(obj, i, "first")?.unwrap_or(false),
        last: parse_opt_bool(obj, i, "last")?.unwrap_or(false),
        negative: parse_opt_bool(obj, i, "negative")?.unwrap_or(false),
        display_x_axis: parse_opt_bool(obj, i, "displayXAxis")?.unwrap_or(false),
        line_weight: parse_spark_line_weight(obj, i)?,
        min_axis,
        max_axis,
        manual_min,
        manual_max,
        colors: parse_spark_colors(obj.get("colors"), i)?.unwrap_or_default(),
    })
}

pub(super) fn parse_set_sparkline(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetSparklineOp, Error> {
    deny_keys(obj, i, SPARK_KEYS)?;
    let id = require_id(obj, i)?;
    let (sheet, index) = parse_spark_id(&id, i)?;
    let kind = parse_spark_type(obj, i)?;
    let refer = parse_opt_spark_host(obj, i)?;
    let formula = parse_opt_spark_formula(obj, i)?;
    let display_empty = parse_spark_empty(obj, i)?;
    let markers = parse_opt_bool(obj, i, "markers")?;
    let high = parse_opt_bool(obj, i, "high")?;
    let low = parse_opt_bool(obj, i, "low")?;
    let first = parse_opt_bool(obj, i, "first")?;
    let last = parse_opt_bool(obj, i, "last")?;
    let negative = parse_opt_bool(obj, i, "negative")?;
    let display_x_axis = parse_opt_bool(obj, i, "displayXAxis")?;
    let line_weight = parse_spark_line_weight_patch(obj, i)?;
    let min_axis = parse_spark_axis(obj, i, "minAxisType")?;
    let max_axis = parse_spark_axis(obj, i, "maxAxisType")?;
    let manual_min = parse_f64_patch(obj, i, "manualMin")?;
    let manual_max = parse_f64_patch(obj, i, "manualMax")?;
    let colors = parse_spark_colors(obj.get("colors"), i)?;
    let min_check = match &manual_min {
        Patch::Set(v) => Some(*v),
        _ => None,
    };
    let max_check = match &manual_max {
        Patch::Set(v) => Some(*v),
        _ => None,
    };
    if min_axis == Some(SparkAxisType::Custom) && min_check.is_none() {
        return Err(Error::with_path(
            ErrorKind::InvalidSparkline {
                field: "manualMin".into(),
            },
            format!("ops[{i}].manualMin"),
        ));
    }
    if max_axis == Some(SparkAxisType::Custom) && max_check.is_none() {
        return Err(Error::with_path(
            ErrorKind::InvalidSparkline {
                field: "manualMax".into(),
            },
            format!("ops[{i}].manualMax"),
        ));
    }
    if min_axis.is_some() && min_axis != Some(SparkAxisType::Custom) && min_check.is_some() {
        return Err(Error::with_path(
            ErrorKind::InvalidSparkline {
                field: "manualMin".into(),
            },
            format!("ops[{i}].manualMin"),
        ));
    }
    if max_axis.is_some() && max_axis != Some(SparkAxisType::Custom) && max_check.is_some() {
        return Err(Error::with_path(
            ErrorKind::InvalidSparkline {
                field: "manualMax".into(),
            },
            format!("ops[{i}].manualMax"),
        ));
    }
    if kind.is_none()
        && refer.is_none()
        && formula.is_absent()
        && display_empty.is_none()
        && markers.is_none()
        && high.is_none()
        && low.is_none()
        && first.is_none()
        && last.is_none()
        && negative.is_none()
        && display_x_axis.is_none()
        && line_weight.is_absent()
        && min_axis.is_none()
        && max_axis.is_none()
        && manual_min.is_absent()
        && manual_max.is_absent()
        && colors.is_none()
    {
        return Err(Error::with_path(
            ErrorKind::NeedSetSparklineField,
            format!("ops[{i}]"),
        ));
    }
    Ok(SetSparklineOp {
        id,
        sheet,
        index,
        kind,
        refer,
        formula,
        display_empty,
        markers,
        high,
        low,
        first,
        last,
        negative,
        display_x_axis,
        line_weight,
        min_axis,
        max_axis,
        manual_min,
        manual_max,
        colors,
    })
}

pub(super) fn parse_delete_sparkline(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteSparklineOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let (sheet, index) = parse_spark_id(&id, i)?;
    Ok(DeleteSparklineOp { id, sheet, index })
}

pub(super) fn parse_opt_password(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<Option<String>, Error> {
    match obj.get("password") {
        None => Ok(None),
        Some(Value::String(s)) if !s.is_empty() => Ok(Some(s.clone())),
        Some(Value::String(_)) => Err(Error::with_path(
            ErrorKind::ProtectionPasswordBlank,
            format!("ops[{i}].password"),
        )),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "password".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].password"),
        )),
    }
}

pub(super) fn parse_set_protection(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetProtectionOp, Error> {
    let id = require_id(obj, i)?;
    if id == "xlsx:workbook" {
        deny_keys(
            obj,
            i,
            &[
                "op",
                "id",
                "lockStructure",
                "lockWindows",
                "lockRevision",
                "password",
            ],
        )?;
        let op = SetWorkbookProtectionOp {
            lock_structure: parse_opt_bool(obj, i, "lockStructure")?,
            lock_windows: parse_opt_bool(obj, i, "lockWindows")?,
            lock_revision: parse_opt_bool(obj, i, "lockRevision")?,
            password: parse_opt_password(obj, i)?,
        };
        if op.lock_structure.is_none() && op.lock_windows.is_none() && op.lock_revision.is_none() {
            return Err(Error::with_path(
                ErrorKind::NeedProtectionField,
                format!("ops[{i}]"),
            ));
        }
        return Ok(SetProtectionOp::Workbook(op));
    }
    deny_keys(
        obj,
        i,
        &[
            "op",
            "id",
            "enforced",
            "objects",
            "scenarios",
            "formatCells",
            "formatColumns",
            "formatRows",
            "insertColumns",
            "insertRows",
            "insertHyperlinks",
            "deleteColumns",
            "deleteRows",
            "selectLockedCells",
            "sort",
            "autoFilter",
            "pivotTables",
            "selectUnlockedCells",
            "password",
        ],
    )?;
    let sheet = parse_sheet_only_id(&id, i, "id", IdReason::SetProtectionNeedsSheetOrWorkbook)?;
    let op = SetSheetProtectionOp {
        sheet,
        enforced: parse_opt_bool(obj, i, "enforced")?,
        objects: parse_opt_bool(obj, i, "objects")?,
        scenarios: parse_opt_bool(obj, i, "scenarios")?,
        format_cells: parse_opt_bool(obj, i, "formatCells")?,
        format_columns: parse_opt_bool(obj, i, "formatColumns")?,
        format_rows: parse_opt_bool(obj, i, "formatRows")?,
        insert_columns: parse_opt_bool(obj, i, "insertColumns")?,
        insert_rows: parse_opt_bool(obj, i, "insertRows")?,
        insert_hyperlinks: parse_opt_bool(obj, i, "insertHyperlinks")?,
        delete_columns: parse_opt_bool(obj, i, "deleteColumns")?,
        delete_rows: parse_opt_bool(obj, i, "deleteRows")?,
        select_locked_cells: parse_opt_bool(obj, i, "selectLockedCells")?,
        sort: parse_opt_bool(obj, i, "sort")?,
        auto_filter: parse_opt_bool(obj, i, "autoFilter")?,
        pivot_tables: parse_opt_bool(obj, i, "pivotTables")?,
        select_unlocked_cells: parse_opt_bool(obj, i, "selectUnlockedCells")?,
        password: parse_opt_password(obj, i)?,
    };
    if op.enforced.is_none() && !op.has_flag() {
        return Err(Error::with_path(
            ErrorKind::NeedProtectionField,
            format!("ops[{i}]"),
        ));
    }
    Ok(SetProtectionOp::Sheet(op))
}

pub(super) fn parse_insert_protected_range(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertProtectedRangeOp, Error> {
    deny_keys(obj, i, &["op", "id", "name", "sqref", "password"])?;
    let id = require_id(obj, i)?;
    let sheet = parse_sheet_only_id(&id, i, "id", IdReason::InsertProtectedRangeNeedsSheet)?;
    let name = parse_protected_range_name(obj.get("name"), i, true)?.ok_or_else(|| {
        Error::with_path(
            ErrorKind::MissingField {
                field: "name".into(),
            },
            format!("ops[{i}].name"),
        )
    })?;
    let sqref = parse_sqref_field(obj.get("sqref"), i, true)?;
    Ok(InsertProtectedRangeOp {
        sheet,
        name,
        sqref,
        password: parse_opt_password(obj, i)?,
    })
}

pub(super) fn parse_set_protected_range(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetProtectedRangeOp, Error> {
    deny_keys(obj, i, &["op", "id", "name", "sqref", "password"])?;
    let id = require_id(obj, i)?;
    let (sheet, index) = parse_edit_range_id(&id, i)?;
    let sqref = if obj.contains_key("sqref") {
        Some(parse_sqref_field(obj.get("sqref"), i, true)?)
    } else {
        None
    };
    let op = SetProtectedRangeOp {
        sheet,
        index,
        name: parse_protected_range_name(obj.get("name"), i, false)?,
        sqref,
        password: parse_opt_password(obj, i)?,
    };
    if !op.has_patch() {
        return Err(Error::with_path(
            ErrorKind::NeedSetProtectedRangeField,
            format!("ops[{i}]"),
        ));
    }
    Ok(op)
}

pub(super) fn parse_delete_protected_range(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteProtectedRangeOp, Error> {
    deny_keys(obj, i, &["op", "id", "password"])?;
    let id = require_id(obj, i)?;
    let (sheet, index) = parse_edit_range_id(&id, i)?;
    Ok(DeleteProtectedRangeOp {
        sheet,
        index,
        password: parse_opt_password(obj, i)?,
    })
}

pub(super) fn parse_protected_range_name(
    v: Option<&Value>,
    i: usize,
    required: bool,
) -> Result<Option<String>, Error> {
    match v {
        None if required => Err(Error::with_path(
            ErrorKind::MissingField {
                field: "name".into(),
            },
            format!("ops[{i}].name"),
        )),
        None => Ok(None),
        Some(Value::Null) => Err(Error::with_path(
            ErrorKind::InvalidProtectedRangeName,
            format!("ops[{i}].name"),
        )),
        Some(Value::String(s)) => {
            let t = s.trim();
            if t.is_empty() || t.chars().count() > 255 {
                return Err(Error::with_path(
                    ErrorKind::InvalidProtectedRangeName,
                    format!("ops[{i}].name"),
                ));
            }
            Ok(Some(t.to_string()))
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

pub(super) fn parse_edit_range_id(id: &str, op_index: usize) -> Result<(String, usize), Error> {
    let Some(rest) = id.strip_prefix("sheet:") else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SheetEditRangePrefix,
            },
            format!("ops[{op_index}].id"),
        ));
    };
    let Some((name, nstr)) = rest.split_once("/editRange:") else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SheetEditRangePrefix,
            },
            format!("ops[{op_index}].id"),
        ));
    };
    if name.is_empty() || name.contains('/') {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SheetEditRangePrefix,
            },
            format!("ops[{op_index}].id"),
        ));
    }
    if nstr.is_empty() || !nstr.bytes().all(|b| b.is_ascii_digit()) {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::EditRangeIndexNotInt,
            },
            format!("ops[{op_index}].id"),
        ));
    }
    let n: usize = nstr.parse().map_err(|_| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::EditRangeIndexNotInt,
            },
            format!("ops[{op_index}].id"),
        )
    })?;
    if n == 0 {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::EditRangeIndexFromOne,
            },
            format!("ops[{op_index}].id"),
        ));
    }
    Ok((name.to_string(), n))
}

pub(super) fn parse_spark_id(id: &str, op_index: usize) -> Result<(String, usize), Error> {
    let Some(rest) = id.strip_prefix("sheet:") else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SheetSparkPrefix,
            },
            format!("ops[{op_index}].id"),
        ));
    };
    let Some((name, nstr)) = rest.split_once("/spark:") else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SheetSparkPrefix,
            },
            format!("ops[{op_index}].id"),
        ));
    };
    if name.is_empty() || name.contains('/') {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SheetSparkPrefix,
            },
            format!("ops[{op_index}].id"),
        ));
    }
    if nstr.is_empty() || !nstr.bytes().all(|b| b.is_ascii_digit()) {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SparkIndexNotInt,
            },
            format!("ops[{op_index}].id"),
        ));
    }
    let n: usize = nstr.parse().map_err(|_| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SparkIndexNotInt,
            },
            format!("ops[{op_index}].id"),
        )
    })?;
    if n == 0 {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SparkIndexFromOne,
            },
            format!("ops[{op_index}].id"),
        ));
    }
    Ok((name.to_string(), n))
}

pub(super) fn parse_required_spark_host(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<(String, a1::CellRef), Error> {
    parse_opt_spark_host(obj, i)?.ok_or_else(|| {
        Error::with_path(
            ErrorKind::MissingField {
                field: "ref".into(),
            },
            format!("ops[{i}].ref"),
        )
    })
}

pub(super) fn parse_opt_spark_host(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<Option<(String, a1::CellRef)>, Error> {
    match obj.get("ref") {
        None => Ok(None),
        Some(Value::String(s)) => {
            let (start, end) = a1::parse_range(s).map_err(|e| e.at(format!("ops[{i}].ref")))?;
            if start != end {
                return Err(Error::with_path(
                    ErrorKind::SparklineHostNotCell,
                    format!("ops[{i}].ref"),
                ));
            }
            Ok(Some((a1::format_range(start, end), start)))
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

pub(super) fn parse_required_spark_formula(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<String, Error> {
    match obj.get("formula") {
        Some(Value::String(s)) => parse_spark_formula_text(s, i),
        None => Err(Error::with_path(
            ErrorKind::MissingField {
                field: "formula".into(),
            },
            format!("ops[{i}].formula"),
        )),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "formula".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].formula"),
        )),
    }
}

pub(super) fn parse_opt_spark_formula(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<Patch<String>, Error> {
    match obj.get("formula") {
        None => Ok(Patch::Absent),
        Some(Value::String(s)) => Ok(Patch::Set(parse_spark_formula_text(s, i)?)),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "formula".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].formula"),
        )),
    }
}

pub(super) fn parse_spark_formula_text(s: &str, i: usize) -> Result<String, Error> {
    if s.trim().is_empty() {
        return Err(Error::with_path(
            ErrorKind::EmptySparklineFormula,
            format!("ops[{i}].formula"),
        ));
    }
    if formula_is_block(s) {
        return Err(Error::with_path(
            ErrorKind::SparklineFormulaNotLine,
            format!("ops[{i}].formula"),
        ));
    }
    Ok(s.to_string())
}

pub(super) fn parse_spark_type(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<Option<SparklineType>, Error> {
    match obj.get("type") {
        None => Ok(None),
        Some(Value::String(s)) => match s.as_str() {
            "line" => Ok(Some(SparklineType::Line)),
            "column" => Ok(Some(SparklineType::Column)),
            "stacked" => Ok(Some(SparklineType::Stacked)),
            _ => Err(Error::with_path(
                ErrorKind::InvalidSparklineType,
                format!("ops[{i}].type"),
            )),
        },
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "type".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].type"),
        )),
    }
}

pub(super) fn parse_spark_empty(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<Option<SparkEmpty>, Error> {
    match obj.get("displayEmptyCellsAs") {
        None => Ok(None),
        Some(Value::String(s)) => SparkEmpty::parse(s).map(Some).ok_or_else(|| {
            Error::with_path(
                ErrorKind::InvalidSparkline {
                    field: "displayEmptyCellsAs".into(),
                },
                format!("ops[{i}].displayEmptyCellsAs"),
            )
        }),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "displayEmptyCellsAs".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].displayEmptyCellsAs"),
        )),
    }
}

pub(super) fn parse_spark_axis(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    field: &str,
) -> Result<Option<SparkAxisType>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::String(s)) => SparkAxisType::parse(s).map(Some).ok_or_else(|| {
            Error::with_path(
                ErrorKind::InvalidSparkline {
                    field: field.into(),
                },
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

pub(super) fn parse_spark_line_weight(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<Option<f64>, Error> {
    match obj.get("lineWeight") {
        None => Ok(None),
        Some(Value::Number(n)) => {
            let Some(v) = n.as_f64().filter(|x| x.is_finite()) else {
                return Err(Error::with_path(
                    ErrorKind::InvalidSparkline {
                        field: "lineWeight".into(),
                    },
                    format!("ops[{i}].lineWeight"),
                ));
            };
            if !(0.0..=1584.0).contains(&v) {
                return Err(Error::with_path(
                    ErrorKind::InvalidSparkline {
                        field: "lineWeight".into(),
                    },
                    format!("ops[{i}].lineWeight"),
                ));
            }
            Ok(Some(v))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "lineWeight".into(),
                expected: TypeName::Integer,
            },
            format!("ops[{i}].lineWeight"),
        )),
    }
}

pub(super) fn parse_spark_line_weight_patch(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<Patch<f64>, Error> {
    match obj.get("lineWeight") {
        None => Ok(Patch::Absent),
        Some(Value::Null) => Ok(Patch::Clear),
        Some(Value::Number(n)) => {
            let Some(v) = n.as_f64().filter(|x| x.is_finite()) else {
                return Err(Error::with_path(
                    ErrorKind::InvalidSparkline {
                        field: "lineWeight".into(),
                    },
                    format!("ops[{i}].lineWeight"),
                ));
            };
            if !(0.0..=1584.0).contains(&v) {
                return Err(Error::with_path(
                    ErrorKind::InvalidSparkline {
                        field: "lineWeight".into(),
                    },
                    format!("ops[{i}].lineWeight"),
                ));
            }
            Ok(Patch::Set(v))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "lineWeight".into(),
                expected: TypeName::Integer,
            },
            format!("ops[{i}].lineWeight"),
        )),
    }
}

pub(super) fn parse_spark_colors(
    v: Option<&Value>,
    i: usize,
) -> Result<Option<SparkColors>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Object(m)) => {
            deny_sub_keys(
                m,
                i,
                "colors",
                &[
                    "series", "negative", "axis", "markers", "first", "last", "high", "low",
                ],
            )?;
            Ok(Some(SparkColors {
                series: parse_opt_color(m, i, "colors", "series")?,
                negative: parse_opt_color(m, i, "colors", "negative")?,
                axis: parse_opt_color(m, i, "colors", "axis")?,
                markers: parse_opt_color(m, i, "colors", "markers")?,
                first: parse_opt_color(m, i, "colors", "first")?,
                last: parse_opt_color(m, i, "colors", "last")?,
                high: parse_opt_color(m, i, "colors", "high")?,
                low: parse_opt_color(m, i, "colors", "low")?,
            }))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "colors".into(),
                expected: TypeName::Object,
            },
            format!("ops[{i}].colors"),
        )),
    }
}

pub(super) fn check_spark_axis(
    min_axis: Option<SparkAxisType>,
    max_axis: Option<SparkAxisType>,
    manual_min: Option<f64>,
    manual_max: Option<f64>,
    i: usize,
) -> Result<(), Error> {
    let bad = |field: &str| {
        Error::with_path(
            ErrorKind::InvalidSparkline {
                field: field.into(),
            },
            format!("ops[{i}].{field}"),
        )
    };
    match min_axis {
        Some(SparkAxisType::Custom) if manual_min.is_none() => return Err(bad("manualMin")),
        Some(kind) if kind != SparkAxisType::Custom && manual_min.is_some() => {
            return Err(bad("manualMin"));
        }
        None if manual_min.is_some() => return Err(bad("manualMin")),
        _ => {}
    }
    match max_axis {
        Some(SparkAxisType::Custom) if manual_max.is_none() => return Err(bad("manualMax")),
        Some(kind) if kind != SparkAxisType::Custom && manual_max.is_some() => {
            return Err(bad("manualMax"));
        }
        None if manual_max.is_some() => return Err(bad("manualMax")),
        _ => {}
    }
    Ok(())
}

pub(super) fn parse_insert_ole(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<ole::InsertOleOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op", "id", "file", "cx", "cy", "from", "to", "anchor", "fromOffX", "fromOffY",
            "toOffX", "toOffY",
        ],
    )?;
    let id = require_id(obj, i)?;
    let sheet = parse_sheet_only_id(&id, i, "id", IdReason::InsertOleNeedsSheet)?;
    let file = require_file(obj, i, "insertOle")?;
    let cx = parse_emu_field(obj.get("cx"), i, "cx")?;
    let cy = parse_emu_field(obj.get("cy"), i, "cy")?;
    if cx.is_some() != cy.is_some() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "insertOle".into(),
                field: "cx and cy together".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    let from = match obj.get("from") {
        Some(Value::String(s)) => a1::parse_cell(s).map_err(|e| e.at(format!("ops[{i}].from")))?,
        None => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "from".into(),
                },
                format!("ops[{i}].from"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "from".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].from"),
            ));
        }
    };
    let anchor = match obj.get("anchor") {
        None => picture::AnchorKind::OneCell,
        Some(Value::String(s)) => picture::AnchorKind::parse_insert(s).ok_or_else(|| {
            Error::with_path(ErrorKind::InvalidXlsxAnchor, format!("ops[{i}].anchor"))
        })?,
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "anchor".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].anchor"),
            ));
        }
    };
    let to = match obj.get("to") {
        None => None,
        Some(Value::String(s)) => {
            Some(a1::parse_cell(s).map_err(|e| e.at(format!("ops[{i}].to")))?)
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "to".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].to"),
            ));
        }
    };
    if anchor == picture::AnchorKind::TwoCell && to.is_none() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "insertOle".into(),
                field: "to".into(),
            },
            format!("ops[{i}].to"),
        ));
    }
    if anchor == picture::AnchorKind::OneCell && to.is_some() {
        return Err(Error::with_path(
            ErrorKind::UnknownField { field: "to".into() },
            format!("ops[{i}].to"),
        ));
    }
    Ok(ole::InsertOleOp {
        sheet,
        file,
        cx,
        cy,
        from,
        to,
        anchor,
        from_off_x: parse_off_field(obj.get("fromOffX"), i, "fromOffX")?.unwrap_or(0),
        from_off_y: parse_off_field(obj.get("fromOffY"), i, "fromOffY")?.unwrap_or(0),
        to_off_x: parse_off_field(obj.get("toOffX"), i, "toOffX")?.unwrap_or(0),
        to_off_y: parse_off_field(obj.get("toOffY"), i, "toOffY")?.unwrap_or(0),
    })
}

pub(super) fn parse_delete_ole(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<ole::DeleteOleOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let target = ole::parse_ole_id(&id, &format!("ops[{i}].id"))?;
    Ok(ole::DeleteOleOp { id, target })
}

pub(super) fn parse_set_ole_size(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<ole::SetOleSizeOp, Error> {
    deny_keys(obj, i, &["op", "id", "cx", "cy", "to", "toOffX", "toOffY"])?;
    let id = require_id(obj, i)?;
    let target = ole::parse_ole_id(&id, &format!("ops[{i}].id"))?;
    let s = parse_drawing_size(obj, i, "setOleSize")?;
    Ok(ole::SetOleSizeOp {
        id,
        target,
        cx: s.cx,
        cy: s.cy,
        to: s.to,
        to_off_x: s.to_off_x,
        to_off_y: s.to_off_y,
    })
}

pub(super) fn parse_insert_model3d(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<super::super::model3d::InsertModel3dOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op", "id", "file", "cx", "cy", "from", "to", "anchor", "fromOffX", "fromOffY",
            "toOffX", "toOffY",
        ],
    )?;
    let id = require_id(obj, i)?;
    let sheet = parse_sheet_only_id(&id, i, "id", IdReason::InsertModel3dNeedsSheet)?;
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
    let from = match obj.get("from") {
        Some(Value::String(s)) => a1::parse_cell(s).map_err(|e| e.at(format!("ops[{i}].from")))?,
        None => a1::CellRef { col: 1, row: 1 },
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "from".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].from"),
            ));
        }
    };
    let anchor = match obj.get("anchor") {
        None => picture::AnchorKind::OneCell,
        Some(Value::String(s)) => picture::AnchorKind::parse_insert(s).ok_or_else(|| {
            Error::with_path(ErrorKind::InvalidXlsxAnchor, format!("ops[{i}].anchor"))
        })?,
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "anchor".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].anchor"),
            ));
        }
    };
    let to = match obj.get("to") {
        None => None,
        Some(Value::String(s)) => {
            Some(a1::parse_cell(s).map_err(|e| e.at(format!("ops[{i}].to")))?)
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "to".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].to"),
            ));
        }
    };
    if anchor == picture::AnchorKind::TwoCell && to.is_none() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "insertModel3d".into(),
                field: "to".into(),
            },
            format!("ops[{i}].to"),
        ));
    }
    if anchor == picture::AnchorKind::OneCell && to.is_some() {
        return Err(Error::with_path(
            ErrorKind::UnknownField { field: "to".into() },
            format!("ops[{i}].to"),
        ));
    }
    Ok(super::super::model3d::InsertModel3dOp {
        sheet,
        file,
        cx,
        cy,
        from,
        to,
        anchor,
        from_off_x: parse_off_field(obj.get("fromOffX"), i, "fromOffX")?.unwrap_or(0),
        from_off_y: parse_off_field(obj.get("fromOffY"), i, "fromOffY")?.unwrap_or(0),
        to_off_x: parse_off_field(obj.get("toOffX"), i, "toOffX")?.unwrap_or(0),
        to_off_y: parse_off_field(obj.get("toOffY"), i, "toOffY")?.unwrap_or(0),
    })
}

pub(super) fn parse_replace_model3d(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<super::super::model3d::ReplaceModel3dOp, Error> {
    deny_keys(obj, i, &["op", "id", "file"])?;
    let id = require_id(obj, i)?;
    let target = super::super::model3d::parse_id(&id, &format!("ops[{i}].id"))?;
    let file = require_file(obj, i, "replaceModel3d")?;
    Ok(super::super::model3d::ReplaceModel3dOp { id, target, file })
}

pub(super) fn parse_delete_model3d(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<super::super::model3d::DeleteModel3dOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let target = super::super::model3d::parse_id(&id, &format!("ops[{i}].id"))?;
    Ok(super::super::model3d::DeleteModel3dOp { id, target })
}

pub(super) fn parse_set_model3d_size(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<super::super::model3d::SetModel3dSizeOp, Error> {
    deny_keys(obj, i, &["op", "id", "cx", "cy", "to", "toOffX", "toOffY"])?;
    let id = require_id(obj, i)?;
    let target = super::super::model3d::parse_id(&id, &format!("ops[{i}].id"))?;
    let s = parse_drawing_size(obj, i, "setModel3dSize")?;
    Ok(super::super::model3d::SetModel3dSizeOp {
        id,
        target,
        cx: s.cx,
        cy: s.cy,
        to: s.to,
        to_off_x: s.to_off_x,
        to_off_y: s.to_off_y,
    })
}

pub(super) fn parse_set_model3d_anchor(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<super::super::model3d::SetModel3dAnchorOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op", "id", "anchor", "from", "to", "fromOffX", "fromOffY", "toOffX", "toOffY", "posX",
            "posY", "editAs", "cx", "cy",
        ],
    )?;
    let id = require_id(obj, i)?;
    let target = super::super::model3d::parse_id(&id, &format!("ops[{i}].id"))?;
    let p = parse_drawing_anchor_patch(obj, i, "setModel3dAnchor")?;
    Ok(super::super::model3d::SetModel3dAnchorOp {
        id,
        target,
        anchor: p.anchor,
        from: p.from,
        to: p.to,
        from_off_x: p.from_off_x,
        from_off_y: p.from_off_y,
        to_off_x: p.to_off_x,
        to_off_y: p.to_off_y,
        pos_x: p.pos_x,
        pos_y: p.pos_y,
        edit_as: p.edit_as,
        cx: p.cx,
        cy: p.cy,
    })
}

pub(super) fn parse_set_model3d_camera(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<super::super::model3d::SetModel3dCameraOp, Error> {
    deny_keys(obj, i, &["op", "id", "camera", "rot", "lightRig"])?;
    let id = require_id(obj, i)?;
    let target = super::super::model3d::parse_id(&id, &format!("ops[{i}].id"))?;
    let patch = crate::model3d::parse_camera_patch(obj, i)?;
    Ok(super::super::model3d::SetModel3dCameraOp { id, target, patch })
}

pub(super) fn parse_insert_ink(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<super::super::ink::InsertInkOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op", "id", "file", "cx", "cy", "from", "to", "anchor", "fromOffX", "fromOffY",
            "toOffX", "toOffY",
        ],
    )?;
    let id = require_id(obj, i)?;
    let sheet = parse_sheet_only_id(&id, i, "id", IdReason::InsertInkNeedsSheet)?;
    let file = require_file(obj, i, "insertInk")?;
    let cx = parse_emu_field(obj.get("cx"), i, "cx")?;
    let cy = parse_emu_field(obj.get("cy"), i, "cy")?;
    if cx.is_some() != cy.is_some() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "insertInk".into(),
                field: "cx and cy together".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    let from = match obj.get("from") {
        Some(Value::String(s)) => a1::parse_cell(s).map_err(|e| e.at(format!("ops[{i}].from")))?,
        None => a1::CellRef { col: 1, row: 1 },
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "from".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].from"),
            ));
        }
    };
    let anchor = match obj.get("anchor") {
        None => picture::AnchorKind::OneCell,
        Some(Value::String(s)) => picture::AnchorKind::parse_insert(s).ok_or_else(|| {
            Error::with_path(ErrorKind::InvalidXlsxAnchor, format!("ops[{i}].anchor"))
        })?,
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "anchor".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].anchor"),
            ));
        }
    };
    let to = match obj.get("to") {
        None => None,
        Some(Value::String(s)) => {
            Some(a1::parse_cell(s).map_err(|e| e.at(format!("ops[{i}].to")))?)
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "to".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].to"),
            ));
        }
    };
    if anchor == picture::AnchorKind::TwoCell && to.is_none() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "insertInk".into(),
                field: "to".into(),
            },
            format!("ops[{i}].to"),
        ));
    }
    if anchor == picture::AnchorKind::OneCell && to.is_some() {
        return Err(Error::with_path(
            ErrorKind::UnknownField { field: "to".into() },
            format!("ops[{i}].to"),
        ));
    }
    Ok(super::super::ink::InsertInkOp {
        sheet,
        file,
        cx,
        cy,
        from,
        to,
        anchor,
        from_off_x: parse_off_field(obj.get("fromOffX"), i, "fromOffX")?.unwrap_or(0),
        from_off_y: parse_off_field(obj.get("fromOffY"), i, "fromOffY")?.unwrap_or(0),
        to_off_x: parse_off_field(obj.get("toOffX"), i, "toOffX")?.unwrap_or(0),
        to_off_y: parse_off_field(obj.get("toOffY"), i, "toOffY")?.unwrap_or(0),
    })
}

pub(super) fn parse_replace_ink(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<super::super::ink::ReplaceInkOp, Error> {
    deny_keys(obj, i, &["op", "id", "file"])?;
    let id = require_id(obj, i)?;
    let target = super::super::ink::parse_id(&id, &format!("ops[{i}].id"))?;
    let file = require_file(obj, i, "replaceInk")?;
    Ok(super::super::ink::ReplaceInkOp { id, target, file })
}

pub(super) fn parse_delete_ink(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<super::super::ink::DeleteInkOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let target = super::super::ink::parse_id(&id, &format!("ops[{i}].id"))?;
    Ok(super::super::ink::DeleteInkOp { id, target })
}

pub(super) fn parse_set_ink_size(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<super::super::ink::SetInkSizeOp, Error> {
    deny_keys(obj, i, &["op", "id", "cx", "cy", "to", "toOffX", "toOffY"])?;
    let id = require_id(obj, i)?;
    let target = super::super::ink::parse_id(&id, &format!("ops[{i}].id"))?;
    let s = parse_drawing_size(obj, i, "setInkSize")?;
    Ok(super::super::ink::SetInkSizeOp {
        id,
        target,
        cx: s.cx,
        cy: s.cy,
        to: s.to,
        to_off_x: s.to_off_x,
        to_off_y: s.to_off_y,
    })
}

pub(super) fn parse_set_ink_anchor(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<super::super::ink::SetInkAnchorOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op", "id", "anchor", "from", "to", "fromOffX", "fromOffY", "toOffX", "toOffY", "posX",
            "posY", "editAs", "cx", "cy",
        ],
    )?;
    let id = require_id(obj, i)?;
    let target = super::super::ink::parse_id(&id, &format!("ops[{i}].id"))?;
    let p = parse_drawing_anchor_patch(obj, i, "setInkAnchor")?;
    Ok(super::super::ink::SetInkAnchorOp {
        id,
        target,
        anchor: p.anchor,
        from: p.from,
        to: p.to,
        from_off_x: p.from_off_x,
        from_off_y: p.from_off_y,
        to_off_x: p.to_off_x,
        to_off_y: p.to_off_y,
        pos_x: p.pos_x,
        pos_y: p.pos_y,
        edit_as: p.edit_as,
        cx: p.cx,
        cy: p.cy,
    })
}

pub(super) fn parse_insert_math(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<super::super::omml::InsertMathOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op", "id", "text", "cx", "cy", "from", "to", "anchor", "fromOffX", "fromOffY",
            "toOffX", "toOffY", "expr", "kind",
        ],
    )?;
    crate::drawing_math::reject_expr_kind(obj, &format!("ops[{i}]"))?;
    let id = require_id(obj, i)?;
    let sheet = parse_sheet_only_id(&id, i, "id", IdReason::InsertMathNeedsSheet)?;
    let text = crate::drawing_math::require_text(obj, &format!("ops[{i}]"))?;
    let cx = parse_emu_field(obj.get("cx"), i, "cx")?;
    let cy = parse_emu_field(obj.get("cy"), i, "cy")?;
    if cx.is_some() != cy.is_some() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "insertMath".into(),
                field: "cx and cy together".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    let from = match obj.get("from") {
        Some(Value::String(s)) => a1::parse_cell(s).map_err(|e| e.at(format!("ops[{i}].from")))?,
        None => a1::CellRef { col: 1, row: 1 },
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "from".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].from"),
            ));
        }
    };
    let anchor = match obj.get("anchor") {
        None => picture::AnchorKind::OneCell,
        Some(Value::String(s)) => picture::AnchorKind::parse_insert(s).ok_or_else(|| {
            Error::with_path(ErrorKind::InvalidXlsxAnchor, format!("ops[{i}].anchor"))
        })?,
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "anchor".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].anchor"),
            ));
        }
    };
    let to = match obj.get("to") {
        None => None,
        Some(Value::String(s)) => {
            Some(a1::parse_cell(s).map_err(|e| e.at(format!("ops[{i}].to")))?)
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "to".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].to"),
            ));
        }
    };
    if anchor == picture::AnchorKind::TwoCell && to.is_none() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "insertMath".into(),
                field: "to".into(),
            },
            format!("ops[{i}].to"),
        ));
    }
    if anchor == picture::AnchorKind::OneCell && to.is_some() {
        return Err(Error::with_path(
            ErrorKind::UnknownField { field: "to".into() },
            format!("ops[{i}].to"),
        ));
    }
    Ok(super::super::omml::InsertMathOp {
        sheet,
        text,
        cx,
        cy,
        from,
        to,
        anchor,
        from_off_x: parse_off_field(obj.get("fromOffX"), i, "fromOffX")?.unwrap_or(0),
        from_off_y: parse_off_field(obj.get("fromOffY"), i, "fromOffY")?.unwrap_or(0),
        to_off_x: parse_off_field(obj.get("toOffX"), i, "toOffX")?.unwrap_or(0),
        to_off_y: parse_off_field(obj.get("toOffY"), i, "toOffY")?.unwrap_or(0),
    })
}

pub(super) fn parse_set_math(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<super::super::omml::SetMathOp, Error> {
    deny_keys(obj, i, &["op", "id", "text", "expr", "kind"])?;
    crate::drawing_math::reject_expr_kind(obj, &format!("ops[{i}]"))?;
    let id = require_id(obj, i)?;
    let target = super::super::omml::parse_id(&id, &format!("ops[{i}].id"))?;
    let text = crate::drawing_math::require_text(obj, &format!("ops[{i}]"))?;
    Ok(super::super::omml::SetMathOp { id, target, text })
}

pub(super) fn parse_delete_math(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<super::super::omml::DeleteMathOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let target = super::super::omml::parse_id(&id, &format!("ops[{i}].id"))?;
    Ok(super::super::omml::DeleteMathOp { id, target })
}

pub(super) fn parse_set_math_size(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<super::super::omml::SetMathSizeOp, Error> {
    deny_keys(obj, i, &["op", "id", "cx", "cy", "to", "toOffX", "toOffY"])?;
    let id = require_id(obj, i)?;
    let target = super::super::omml::parse_id(&id, &format!("ops[{i}].id"))?;
    let s = parse_drawing_size(obj, i, "setMathSize")?;
    Ok(super::super::omml::SetMathSizeOp {
        id,
        target,
        cx: s.cx,
        cy: s.cy,
        to: s.to,
        to_off_x: s.to_off_x,
        to_off_y: s.to_off_y,
    })
}

pub(super) fn parse_set_math_anchor(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<super::super::omml::SetMathAnchorOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op", "id", "anchor", "from", "to", "fromOffX", "fromOffY", "toOffX", "toOffY", "posX",
            "posY", "editAs", "cx", "cy", "fromX", "fromY", "toX", "toY",
        ],
    )?;
    let id = require_id(obj, i)?;
    let target = super::super::omml::parse_id(&id, &format!("ops[{i}].id"))?;
    let p = parse_drawing_anchor_patch(obj, i, "setMathAnchor")?;
    Ok(super::super::omml::SetMathAnchorOp {
        id,
        target,
        anchor: p.anchor,
        from: p.from,
        to: p.to,
        from_off_x: p.from_off_x,
        from_off_y: p.from_off_y,
        to_off_x: p.to_off_x,
        to_off_y: p.to_off_y,
        pos_x: p.pos_x,
        pos_y: p.pos_y,
        edit_as: p.edit_as,
        cx: p.cx,
        cy: p.cy,
        rel: p.rel,
    })
}

pub(super) fn parse_insert_slicer(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<slicer::InsertSlicerOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op",
            "id",
            "source",
            "field",
            "kind",
            "level",
            "cx",
            "cy",
            "from",
            "to",
            "anchor",
            "fromOffX",
            "fromOffY",
            "toOffX",
            "toOffY",
            "caption",
            "rowHeight",
            "columnCount",
            "showCaption",
            "startItem",
            "lockedPosition",
            "style",
            "showHeader",
            "showSelectionLabel",
            "showTimeLevel",
            "showHorizontalScrollbar",
            "selectionLevel",
            "scrollPosition",
        ],
    )?;
    let id = require_id(obj, i)?;
    let sheet = parse_sheet_only_id(&id, i, "id", IdReason::InsertSlicerNeedsSheet)?;
    let source_raw = match obj.get("source") {
        Some(Value::String(s)) if !s.is_empty() => s.clone(),
        None => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "source".into(),
                },
                format!("ops[{i}].source"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "source".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].source"),
            ));
        }
    };
    let source = parse_slicer_source(&source_raw, i)?;
    let field = match obj.get("field") {
        Some(Value::String(s)) if !s.is_empty() => s.clone(),
        None => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "field".into(),
                },
                format!("ops[{i}].field"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "field".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].field"),
            ));
        }
    };
    let cx = parse_emu_field(obj.get("cx"), i, "cx")?;
    let cy = parse_emu_field(obj.get("cy"), i, "cy")?;
    if cx.is_some() != cy.is_some() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "insertSlicer".into(),
                field: "cx and cy together".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    let from = match obj.get("from") {
        Some(Value::String(s)) => a1::parse_cell(s).map_err(|e| e.at(format!("ops[{i}].from")))?,
        None => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "from".into(),
                },
                format!("ops[{i}].from"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "from".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].from"),
            ));
        }
    };
    let anchor = match obj.get("anchor") {
        None => picture::AnchorKind::OneCell,
        Some(Value::String(s)) => picture::AnchorKind::parse_insert(s).ok_or_else(|| {
            Error::with_path(ErrorKind::InvalidXlsxAnchor, format!("ops[{i}].anchor"))
        })?,
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "anchor".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].anchor"),
            ));
        }
    };
    let to = match obj.get("to") {
        None => None,
        Some(Value::String(s)) => {
            Some(a1::parse_cell(s).map_err(|e| e.at(format!("ops[{i}].to")))?)
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "to".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].to"),
            ));
        }
    };
    if anchor == picture::AnchorKind::TwoCell && to.is_none() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "insertSlicer".into(),
                field: "to".into(),
            },
            format!("ops[{i}].to"),
        ));
    }
    if anchor == picture::AnchorKind::OneCell && to.is_some() {
        return Err(Error::with_path(
            ErrorKind::UnknownField { field: "to".into() },
            format!("ops[{i}].to"),
        ));
    }
    let kind = match obj.get("kind") {
        None => slicer::SlicerKind::Slicer,
        Some(Value::String(s)) if s == "slicer" => slicer::SlicerKind::Slicer,
        Some(Value::String(s)) if s == "timeline" => slicer::SlicerKind::Timeline,
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidSlicer {
                    field: "kind".into(),
                },
                format!("ops[{i}].kind"),
            ));
        }
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
    let level = match obj.get("level") {
        None => 2,
        Some(_) if kind != slicer::SlicerKind::Timeline => {
            return Err(Error::with_path(
                ErrorKind::InvalidSlicer {
                    field: "level".into(),
                },
                format!("ops[{i}].level"),
            ));
        }
        Some(Value::Number(n)) => {
            let v = n.as_u64().and_then(|x| u32::try_from(x).ok());
            match v {
                Some(v) if v <= 3 => v,
                _ => {
                    return Err(Error::with_path(
                        ErrorKind::InvalidSlicer {
                            field: "level".into(),
                        },
                        format!("ops[{i}].level"),
                    ));
                }
            }
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "level".into(),
                    expected: TypeName::Integer,
                },
                format!("ops[{i}].level"),
            ));
        }
    };
    let view = parse_slicer_view_fields(obj, i)?;
    if kind == slicer::SlicerKind::Slicer {
        if let Some(field) = view.timeline_only_field() {
            return Err(Error::with_path(
                ErrorKind::InvalidSlicer {
                    field: field.into(),
                },
                format!("ops[{i}].{field}"),
            ));
        }
    } else if let Some(field) = view.slicer_only_field() {
        return Err(Error::with_path(
            ErrorKind::InvalidSlicer {
                field: field.into(),
            },
            format!("ops[{i}].{field}"),
        ));
    }
    Ok(slicer::InsertSlicerOp {
        sheet,
        source,
        field,
        kind,
        level,
        cx,
        cy,
        from,
        to,
        anchor,
        from_off_x: parse_off_field(obj.get("fromOffX"), i, "fromOffX")?.unwrap_or(0),
        from_off_y: parse_off_field(obj.get("fromOffY"), i, "fromOffY")?.unwrap_or(0),
        to_off_x: parse_off_field(obj.get("toOffX"), i, "toOffX")?.unwrap_or(0),
        to_off_y: parse_off_field(obj.get("toOffY"), i, "toOffY")?.unwrap_or(0),
        view,
    })
}

pub(super) fn parse_slicer_source(raw: &str, i: usize) -> Result<slicer::SlicerSource, Error> {
    if raw.contains("/table:") {
        let (sheet, index) =
            parse_table_id(raw, i).map_err(|e| e.at(format!("ops[{i}].source")))?;
        return Ok(slicer::SlicerSource::Table { sheet, index });
    }
    if raw.contains("/pivot:") {
        let (sheet, index) =
            parse_pivot_id(raw, i).map_err(|e| e.at(format!("ops[{i}].source")))?;
        return Ok(slicer::SlicerSource::Pivot { sheet, index });
    }
    Err(Error::with_path(
        ErrorKind::InvalidId {
            reason: IdReason::InsertSlicerNeedsTableOrPivot,
        },
        format!("ops[{i}].source"),
    ))
}

pub(super) fn parse_delete_slicer(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<slicer::DeleteSlicerOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let target = slicer::parse_slicer_id(&id, &format!("ops[{i}].id"))?;
    Ok(slicer::DeleteSlicerOp { id, target })
}

pub(super) fn parse_set_slicer_size(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<slicer::SetSlicerSizeOp, Error> {
    deny_keys(obj, i, &["op", "id", "cx", "cy", "to", "toOffX", "toOffY"])?;
    let id = require_id(obj, i)?;
    let target = slicer::parse_slicer_id(&id, &format!("ops[{i}].id"))?;
    let s = parse_drawing_size(obj, i, "setSlicerSize")?;
    Ok(slicer::SetSlicerSizeOp {
        id,
        target,
        cx: s.cx,
        cy: s.cy,
        to: s.to,
        to_off_x: s.to_off_x,
        to_off_y: s.to_off_y,
    })
}

pub(super) fn parse_set_slicer_anchor(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<slicer::SetSlicerAnchorOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op", "id", "anchor", "from", "to", "fromOffX", "fromOffY", "toOffX", "toOffY", "posX",
            "posY", "editAs", "cx", "cy", "fromX", "fromY", "toX", "toY",
        ],
    )?;
    let id = require_id(obj, i)?;
    let target = slicer::parse_slicer_id(&id, &format!("ops[{i}].id"))?;
    let p = parse_drawing_anchor_patch(obj, i, "setSlicerAnchor")?;
    Ok(slicer::SetSlicerAnchorOp {
        id,
        target,
        anchor: p.anchor,
        from: p.from,
        to: p.to,
        from_off_x: p.from_off_x,
        from_off_y: p.from_off_y,
        to_off_x: p.to_off_x,
        to_off_y: p.to_off_y,
        pos_x: p.pos_x,
        pos_y: p.pos_y,
        edit_as: p.edit_as,
        cx: p.cx,
        cy: p.cy,
        rel: p.rel,
    })
}

pub(super) fn parse_set_slicer_view(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<slicer::SetSlicerViewOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op",
            "id",
            "caption",
            "rowHeight",
            "columnCount",
            "showCaption",
            "startItem",
            "lockedPosition",
            "style",
            "showHeader",
            "showSelectionLabel",
            "showTimeLevel",
            "showHorizontalScrollbar",
            "level",
            "selectionLevel",
            "scrollPosition",
        ],
    )?;
    let id = require_id(obj, i)?;
    let target = slicer::parse_slicer_id(&id, &format!("ops[{i}].id"))?;
    let view = parse_slicer_view_fields(obj, i)?;
    let level = parse_slicer_opt_u32(obj, i, "level", 0, Some(3))?;
    if !view.any_set() && level.is_none() {
        return Err(Error::with_path(
            ErrorKind::NeedSetSlicerViewField,
            format!("ops[{i}]"),
        ));
    }
    Ok(slicer::SetSlicerViewOp {
        id,
        target,
        view,
        level,
    })
}

pub(super) fn parse_set_slicer_filter(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<slicer::SetSlicerFilterOp, Error> {
    deny_keys(obj, i, &["op", "id", "selected", "filter"])?;
    let id = require_id(obj, i)?;
    let target = slicer::parse_slicer_id(&id, &format!("ops[{i}].id"))?;
    let selected = parse_slicer_selected(obj.get("selected"), i)?;
    let filter = parse_slicer_timeline_filter(obj.get("filter"), i)?;
    if selected.is_none() && filter.is_none() {
        return Err(Error::with_path(
            ErrorKind::NeedSetSlicerFilterField,
            format!("ops[{i}]"),
        ));
    }
    if selected.is_some() && filter.is_some() {
        return Err(Error::with_path(
            ErrorKind::InvalidSlicer {
                field: "filter".into(),
            },
            format!("ops[{i}].filter"),
        ));
    }
    Ok(slicer::SetSlicerFilterOp {
        id,
        target,
        selected,
        filter,
    })
}

fn parse_slicer_selected(v: Option<&Value>, i: usize) -> Result<Option<Vec<String>>, Error> {
    let Some(v) = v else {
        return Ok(None);
    };
    let Value::Array(items) = v else {
        return Err(Error::with_path(
            ErrorKind::WrongType {
                field: "selected".into(),
                expected: TypeName::Array,
            },
            format!("ops[{i}].selected"),
        ));
    };
    let mut out = Vec::with_capacity(items.len());
    for (k, item) in items.iter().enumerate() {
        let path = format!("ops[{i}].selected[{k}]");
        match item {
            Value::String(s) if !s.is_empty() => out.push(s.clone()),
            Value::String(_) => {
                return Err(Error::with_path(
                    ErrorKind::InvalidSlicer {
                        field: "selected".into(),
                    },
                    path,
                ))
            }
            _ => {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: "selected".into(),
                        expected: TypeName::NonEmptyString,
                    },
                    path,
                ))
            }
        }
    }
    Ok(Some(out))
}

fn parse_slicer_timeline_filter(
    v: Option<&Value>,
    i: usize,
) -> Result<Option<Option<slicer::TimelineFilter>>, Error> {
    let Some(v) = v else {
        return Ok(None);
    };
    if v.is_null() {
        return Ok(Some(None));
    }
    let Value::Object(obj) = v else {
        return Err(Error::with_path(
            ErrorKind::WrongType {
                field: "filter".into(),
                expected: TypeName::Object,
            },
            format!("ops[{i}].filter"),
        ));
    };
    deny_keys_at(
        obj,
        &format!("ops[{i}].filter"),
        &["filterType", "startDate", "endDate"],
    )?;
    let filter_type = require_filter_text(obj, i, "filterType")?;
    if !filter_type.starts_with("date") {
        return Err(Error::with_path(
            ErrorKind::InvalidSlicer {
                field: "filterType".into(),
            },
            format!("ops[{i}].filter.filterType"),
        ));
    }
    Ok(Some(Some(slicer::TimelineFilter {
        filter_type,
        start_date: require_filter_text(obj, i, "startDate")?,
        end_date: require_filter_text(obj, i, "endDate")?,
    })))
}

fn require_filter_text(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    field: &str,
) -> Result<String, Error> {
    match obj.get(field) {
        Some(Value::String(s)) if !s.is_empty() => Ok(s.clone()),
        Some(Value::String(_)) | None => Err(Error::with_path(
            ErrorKind::InvalidSlicer {
                field: field.into(),
            },
            format!("ops[{i}].filter.{field}"),
        )),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::NonEmptyString,
            },
            format!("ops[{i}].filter.{field}"),
        )),
    }
}

fn parse_slicer_view_fields(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<slicer::SlicerViewFields, Error> {
    Ok(slicer::SlicerViewFields {
        caption: parse_slicer_opt_text(obj, i, "caption")?,
        row_height: parse_emu_field(obj.get("rowHeight"), i, "rowHeight")?,
        column_count: parse_slicer_opt_u32(obj, i, "columnCount", 1, Some(20000))?,
        show_caption: parse_opt_bool(obj, i, "showCaption")?,
        start_item: parse_slicer_opt_u32(obj, i, "startItem", 0, None)?,
        locked_position: parse_opt_bool(obj, i, "lockedPosition")?,
        style: parse_slicer_opt_style(obj, i, "style")?,
        show_header: parse_opt_bool(obj, i, "showHeader")?,
        show_selection_label: parse_opt_bool(obj, i, "showSelectionLabel")?,
        show_time_level: parse_opt_bool(obj, i, "showTimeLevel")?,
        show_horizontal_scrollbar: parse_opt_bool(obj, i, "showHorizontalScrollbar")?,
        selection_level: parse_slicer_opt_u32(obj, i, "selectionLevel", 0, Some(3))?,
        scroll_position: parse_slicer_opt_style(obj, i, "scrollPosition")?,
    })
}

fn parse_slicer_opt_text(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    field: &str,
) -> Result<Option<String>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::String(s)) if !s.is_empty() => Ok(Some(s.clone())),
        Some(Value::String(_)) => Err(Error::with_path(
            ErrorKind::InvalidSlicer {
                field: field.into(),
            },
            format!("ops[{i}].{field}"),
        )),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::NonEmptyString,
            },
            format!("ops[{i}].{field}"),
        )),
    }
}

fn parse_slicer_opt_style(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    field: &str,
) -> Result<Option<Option<String>>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(None)),
        Some(Value::String(s)) if !s.is_empty() => Ok(Some(Some(s.clone()))),
        Some(Value::String(_)) => Err(Error::with_path(
            ErrorKind::InvalidSlicer {
                field: field.into(),
            },
            format!("ops[{i}].{field}"),
        )),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].{field}"),
        )),
    }
}

fn parse_slicer_opt_u32(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    field: &str,
    min: u32,
    max: Option<u32>,
) -> Result<Option<u32>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::Number(n)) => {
            let Some(u) = n.as_u64().and_then(|x| u32::try_from(x).ok()) else {
                return Err(Error::with_path(
                    ErrorKind::InvalidSlicer {
                        field: field.into(),
                    },
                    format!("ops[{i}].{field}"),
                ));
            };
            if u < min || max.is_some_and(|m| u > m) {
                return Err(Error::with_path(
                    ErrorKind::InvalidSlicer {
                        field: field.into(),
                    },
                    format!("ops[{i}].{field}"),
                ));
            }
            Ok(Some(u))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Integer,
            },
            format!("ops[{i}].{field}"),
        )),
    }
}

pub(super) fn parse_insert_custom_view(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<custom_view::InsertCustomViewOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op",
            "id",
            "name",
            "includePrintSettings",
            "includeHiddenRowCol",
        ],
    )?;
    let id = require_id(obj, i)?;
    if id != "xlsx:workbook" {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::InsertCustomViewNeedsWorkbook,
            },
            format!("ops[{i}].id"),
        ));
    }
    let name = match obj.get("name") {
        Some(Value::String(s)) if !s.is_empty() => s.clone(),
        None => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "name".into(),
                },
                format!("ops[{i}].name"),
            ));
        }
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidCustomView {
                    field: "name".into(),
                },
                format!("ops[{i}].name"),
            ));
        }
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
    Ok(custom_view::InsertCustomViewOp {
        name,
        include_print_settings: parse_opt_bool(obj, i, "includePrintSettings")?.unwrap_or(true),
        include_hidden_row_col: parse_opt_bool(obj, i, "includeHiddenRowCol")?.unwrap_or(true),
    })
}

pub(super) fn parse_delete_custom_view(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<custom_view::DeleteCustomViewOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let index = custom_view::parse_custom_view_id(&id, &format!("ops[{i}].id"))?;
    Ok(custom_view::DeleteCustomViewOp { index })
}

pub(super) fn parse_show_custom_view(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<custom_view::ShowCustomViewOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let index = custom_view::parse_custom_view_id(&id, &format!("ops[{i}].id"))?;
    Ok(custom_view::ShowCustomViewOp { index })
}

pub(super) fn parse_insert_connection(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<connection::InsertConnectionOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op",
            "id",
            "name",
            "type",
            "url",
            "sourceFile",
            "description",
        ],
    )?;
    let id = require_id(obj, i)?;
    if id != "xlsx:workbook" {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::InsertConnectionNeedsWorkbook,
            },
            format!("ops[{i}].id"),
        ));
    }
    let name = match obj.get("name") {
        Some(Value::String(s)) if !s.is_empty() => s.clone(),
        None => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "name".into(),
                },
                format!("ops[{i}].name"),
            ));
        }
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidConnection {
                    field: "name".into(),
                },
                format!("ops[{i}].name"),
            ));
        }
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
    let kind = match obj.get("type") {
        Some(Value::String(s)) if s == "web" => connection::ConnectionKind::Web,
        Some(Value::String(s)) if s == "text" => connection::ConnectionKind::Text,
        None => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "type".into(),
                },
                format!("ops[{i}].type"),
            ));
        }
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidConnection {
                    field: "type".into(),
                },
                format!("ops[{i}].type"),
            ));
        }
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
    let url = parse_opt_nonempty_string(obj, i, "url")?;
    let source_file = parse_opt_nonempty_string(obj, i, "sourceFile")?;
    let description = match obj.get("description") {
        None => None,
        Some(Value::String(s)) => Some(s.clone()),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "description".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].description"),
            ));
        }
    };
    match kind {
        connection::ConnectionKind::Web => {
            if source_file.is_some() {
                return Err(Error::with_path(
                    ErrorKind::InvalidConnection {
                        field: "sourceFile".into(),
                    },
                    format!("ops[{i}].sourceFile"),
                ));
            }
            if url.is_none() {
                return Err(Error::with_path(
                    ErrorKind::MissingField {
                        field: "url".into(),
                    },
                    format!("ops[{i}].url"),
                ));
            }
        }
        connection::ConnectionKind::Text => {
            if url.is_some() {
                return Err(Error::with_path(
                    ErrorKind::InvalidConnection {
                        field: "url".into(),
                    },
                    format!("ops[{i}].url"),
                ));
            }
            if source_file.is_none() {
                return Err(Error::with_path(
                    ErrorKind::MissingField {
                        field: "sourceFile".into(),
                    },
                    format!("ops[{i}].sourceFile"),
                ));
            }
        }
    }
    Ok(connection::InsertConnectionOp {
        name,
        kind,
        url,
        source_file,
        description,
    })
}

pub(super) fn parse_delete_connection(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<connection::DeleteConnectionOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let key = connection::parse_connection_id(&id, &format!("ops[{i}].id"))?;
    Ok(connection::DeleteConnectionOp { id, key })
}

pub(super) fn parse_insert_external_link(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<extlink::InsertExternalLinkOp, Error> {
    deny_keys(obj, i, &["op", "id", "path", "sheets"])?;
    let id = require_id(obj, i)?;
    if id != "xlsx:workbook" {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::InsertExternalLinkNeedsWorkbook,
            },
            format!("ops[{i}].id"),
        ));
    }
    let path = match obj.get("path") {
        Some(Value::String(s)) if !s.is_empty() => s.clone(),
        None => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "path".into(),
                },
                format!("ops[{i}].path"),
            ));
        }
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidExternalLink {
                    field: "path".into(),
                },
                format!("ops[{i}].path"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "path".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].path"),
            ));
        }
    };
    let sheets = match obj.get("sheets") {
        None => vec!["Sheet1".into()],
        Some(Value::Array(arr)) => {
            if arr.is_empty() {
                return Err(Error::with_path(
                    ErrorKind::InvalidExternalLink {
                        field: "sheets".into(),
                    },
                    format!("ops[{i}].sheets"),
                ));
            }
            let mut out = Vec::with_capacity(arr.len());
            for (j, v) in arr.iter().enumerate() {
                match v {
                    Value::String(s) if !s.is_empty() => out.push(s.clone()),
                    Value::String(_) => {
                        return Err(Error::with_path(
                            ErrorKind::InvalidExternalLink {
                                field: "sheets".into(),
                            },
                            format!("ops[{i}].sheets[{j}]"),
                        ));
                    }
                    _ => {
                        return Err(Error::with_path(
                            ErrorKind::WrongType {
                                field: "sheets".into(),
                                expected: TypeName::String,
                            },
                            format!("ops[{i}].sheets[{j}]"),
                        ));
                    }
                }
            }
            out
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "sheets".into(),
                    expected: TypeName::Array,
                },
                format!("ops[{i}].sheets"),
            ));
        }
    };
    Ok(extlink::InsertExternalLinkOp { path, sheets })
}

pub(super) fn parse_delete_external_link(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<extlink::DeleteExternalLinkOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let index = extlink::parse_external_link_id(&id, &format!("ops[{i}].id"))?;
    Ok(extlink::DeleteExternalLinkOp { id, index })
}

pub(super) fn parse_set_external_link(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<extlink::SetExternalLinkOp, Error> {
    deny_keys(obj, i, &["op", "id", "path"])?;
    let id = require_id(obj, i)?;
    let index = extlink::parse_external_link_id(&id, &format!("ops[{i}].id"))?;
    let path = match obj.get("path") {
        Some(Value::String(s)) if !s.is_empty() => s.clone(),
        None => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "path".into(),
                },
                format!("ops[{i}].path"),
            ));
        }
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidExternalLink {
                    field: "path".into(),
                },
                format!("ops[{i}].path"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "path".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].path"),
            ));
        }
    };
    Ok(extlink::SetExternalLinkOp { id, index, path })
}
