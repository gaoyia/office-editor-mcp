//! xlsx request：layout。

use serde_json::Value;

use crate::error::{Error, ErrorKind, IdReason, TypeName};

use super::super::a1;

use super::common::*;
use super::types::*;

pub(super) fn parse_set_col_width(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetColWidthOp, Error> {
    deny_keys(obj, i, &["op", "id", "col", "width", "bestFit"])?;
    let id = require_id(obj, i)?;
    let sheet = parse_sheet_only_id(&id, i, "id", IdReason::SheetOpNeedsSheet)?;
    let col = match obj.get("col") {
        Some(Value::String(s)) => s,
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "col".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].col"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "col".into(),
                },
                format!("ops[{i}].col"),
            ));
        }
    };
    let (min_col, max_col) = a1::parse_col_span(col).map_err(|e| e.at(format!("ops[{i}].col")))?;
    let width = parse_patch_dim(obj, i, "width", 255.0, ErrorKind::InvalidColWidth)?;
    let best_fit = match obj.get("bestFit") {
        None => None,
        Some(Value::Bool(b)) => Some(*b),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "bestFit".into(),
                    expected: TypeName::Boolean,
                },
                format!("ops[{i}].bestFit"),
            ));
        }
    };
    if width.is_absent() && best_fit.is_none() {
        return Err(Error::with_path(
            ErrorKind::NeedSetColWidthField,
            format!("ops[{i}]"),
        ));
    }
    Ok(SetColWidthOp {
        sheet,
        min_col,
        max_col,
        width,
        best_fit,
    })
}

pub(super) fn parse_set_row_height(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetRowHeightOp, Error> {
    deny_keys(obj, i, &["op", "id", "row", "toRow", "height"])?;
    let id = require_id(obj, i)?;
    let sheet = parse_sheet_only_id(&id, i, "id", IdReason::SheetOpNeedsSheet)?;
    let row = parse_required_row(obj, i, "row")?;
    let to_row = match obj.get("toRow") {
        None => row,
        Some(_) => parse_required_row(obj, i, "toRow")?,
    };
    if to_row < row {
        return Err(Error::with_path(
            ErrorKind::RowRangeReversed,
            format!("ops[{i}].toRow"),
        ));
    }
    let height = match obj.get("height") {
        None => {
            return Err(Error::with_path(
                ErrorKind::NeedSetRowHeightField,
                format!("ops[{i}].height"),
            ));
        }
        Some(Value::Null) => Patch::Clear,
        Some(Value::Number(n)) => {
            let Some(v) = n.as_f64() else {
                return Err(Error::with_path(
                    ErrorKind::InvalidRowHeight,
                    format!("ops[{i}].height"),
                ));
            };
            if !(v > 0.0 && v <= 409.0) {
                return Err(Error::with_path(
                    ErrorKind::InvalidRowHeight,
                    format!("ops[{i}].height"),
                ));
            }
            Patch::Set(v)
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "height".into(),
                    expected: TypeName::Integer,
                },
                format!("ops[{i}].height"),
            ));
        }
    };
    Ok(SetRowHeightOp {
        sheet,
        row,
        to_row,
        height,
    })
}

pub(super) fn parse_merge_range(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<MergeRangeOp, Error> {
    deny_keys(obj, i, &["op", "id", "range"])?;
    let id = require_id(obj, i)?;
    let sheet = parse_sheet_only_id(&id, i, "id", IdReason::SheetOpNeedsSheet)?;
    let (start, end) = parse_op_range(obj, i)?;
    if start == end {
        return Err(Error::with_path(
            ErrorKind::MergeNeedsMultipleCells,
            format!("ops[{i}].range"),
        ));
    }
    Ok(MergeRangeOp { sheet, start, end })
}

pub(super) fn parse_unmerge_range(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<UnmergeRangeOp, Error> {
    deny_keys(obj, i, &["op", "id", "range"])?;
    let id = require_id(obj, i)?;
    let sheet = parse_sheet_only_id(&id, i, "id", IdReason::SheetOpNeedsSheet)?;
    let (start, end) = parse_op_range(obj, i)?;
    Ok(UnmergeRangeOp { sheet, start, end })
}

pub(super) fn parse_required_row_span(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    field: &str,
) -> Result<(u32, u32), Error> {
    let raw = match obj.get(field) {
        Some(Value::String(s)) => s,
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: field.into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].{field}"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: field.into(),
                },
                format!("ops[{i}].{field}"),
            ));
        }
    };
    a1::parse_row_span(raw).map_err(|e| e.at(format!("ops[{i}].{field}")))
}

pub(super) fn parse_op_range(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<(a1::CellRef, a1::CellRef), Error> {
    let raw = match obj.get("range") {
        Some(Value::String(s)) => s,
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "range".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].range"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "range".into(),
                },
                format!("ops[{i}].range"),
            ));
        }
    };
    a1::parse_range(raw).map_err(|e| e.at(format!("ops[{i}].range")))
}

pub(super) fn parse_required_row(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    field: &str,
) -> Result<u32, Error> {
    match obj.get(field) {
        Some(Value::Number(n)) => {
            let v = n
                .as_u64()
                .or_else(|| n.as_i64().filter(|x| *x >= 0).map(|x| x as u64));
            let Some(v) = v else {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: field.into(),
                        expected: TypeName::Integer,
                    },
                    format!("ops[{i}].{field}"),
                ));
            };
            if !(1..=1_048_576).contains(&v) {
                return Err(Error::with_path(
                    ErrorKind::InvalidRange {
                        reason: crate::error::RangeReason::RowOutOfRange { row: v as u32 },
                    },
                    format!("ops[{i}].{field}"),
                ));
            }
            Ok(v as u32)
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Integer,
            },
            format!("ops[{i}].{field}"),
        )),
        None => Err(Error::with_path(
            ErrorKind::MissingField {
                field: field.into(),
            },
            format!("ops[{i}].{field}"),
        )),
    }
}

pub(super) fn parse_patch_dim(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    field: &str,
    max: f64,
    invalid: ErrorKind,
) -> Result<Patch<f64>, Error> {
    match obj.get(field) {
        None => Ok(Patch::Absent),
        Some(Value::Null) => Ok(Patch::Clear),
        Some(Value::Number(n)) => {
            let Some(v) = n.as_f64() else {
                return Err(Error::with_path(invalid, format!("ops[{i}].{field}")));
            };
            if !(v > 0.0 && v <= max) {
                return Err(Error::with_path(invalid, format!("ops[{i}].{field}")));
            }
            Ok(Patch::Set(v))
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
