//! docx request：table。

use serde_json::Value;

use crate::error::{Error, ErrorKind, IdReason, TypeName};

use super::super::tbl::{self};

use super::common::*;
use super::types::*;

pub(super) fn parse_insert_table(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertTableOp, Error> {
    deny_keys(obj, i, &["op", "id", "rows", "cols", "cells"])?;
    let id = require_op_id(obj, i)?;
    let anchor = tbl::parse_insert_anchor(&id, i)?;
    let rows = tbl::parse_dim(obj.get("rows"), i, "rows", tbl::max_rows())?;
    let cols = tbl::parse_dim(obj.get("cols"), i, "cols", tbl::max_cols())?;
    let cells = tbl::parse_cells_field(obj.get("cells"), i, rows, cols)?;
    Ok(InsertTableOp { id, anchor, cells })
}

pub(super) fn parse_delete_table(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteTableOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_op_id(obj, i)?;
    let target = tbl::parse_table_only(&id, i, IdReason::DeleteTableNeedsTable)?;
    Ok(DeleteTableOp { id, target })
}

pub(super) fn parse_insert_row(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertRowOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_op_id(obj, i)?;
    let target = tbl::parse_table_or_row(&id, i)?;
    Ok(InsertRowOp { id, target })
}

pub(super) fn parse_delete_row(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteRowOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_op_id(obj, i)?;
    let target = tbl::parse_row_ref(&id, i, IdReason::DeleteRowNeedsRow)?;
    Ok(DeleteRowOp { id, target })
}

pub(super) fn parse_insert_col(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertColOp, Error> {
    deny_keys(obj, i, &["op", "id", "after"])?;
    let id = require_op_id(obj, i)?;
    let target = tbl::parse_table_only(&id, i, IdReason::NeedTableId)?;
    let after = parse_col_after(obj.get("after"), i)?;
    Ok(InsertColOp { id, target, after })
}

pub(super) fn parse_delete_col(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteColOp, Error> {
    deny_keys(obj, i, &["op", "id", "col"])?;
    let id = require_op_id(obj, i)?;
    let target = tbl::parse_table_only(&id, i, IdReason::NeedTableId)?;
    let col = tbl::parse_dim(obj.get("col"), i, "col", tbl::max_cols())? as u32;
    Ok(DeleteColOp { id, target, col })
}

pub(super) fn parse_merge_cells(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<MergeCellsOp, Error> {
    deny_keys(
        obj,
        i,
        &["op", "id", "startRow", "startCol", "endRow", "endCol"],
    )?;
    let id = require_op_id(obj, i)?;
    let target = tbl::parse_table_only(&id, i, IdReason::NeedTableId)?;
    let start_row = tbl::parse_dim(obj.get("startRow"), i, "startRow", tbl::max_rows())?;
    let start_col = tbl::parse_dim(obj.get("startCol"), i, "startCol", tbl::max_cols())?;
    let end_row = tbl::parse_dim(obj.get("endRow"), i, "endRow", tbl::max_rows())?;
    let end_col = tbl::parse_dim(obj.get("endCol"), i, "endCol", tbl::max_cols())?;
    Ok(MergeCellsOp {
        id,
        target,
        start_row,
        start_col,
        end_row,
        end_col,
    })
}

pub(super) fn parse_split_cells(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SplitCellsOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_op_id(obj, i)?;
    let target = tbl::parse_cell_ref(&id, i)?;
    Ok(SplitCellsOp { id, target })
}

pub(super) fn parse_set_table_style(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetTableStyleOp, Error> {
    deny_keys(obj, i, &["op", "id", "style", "look"])?;
    let id = require_op_id(obj, i)?;
    let target = tbl::parse_table_only(&id, i, IdReason::NeedTableId)?;
    let style = match obj.get("style") {
        None => None,
        Some(Value::Null) => Some(None),
        Some(Value::String(s)) => Some(Some(super::super::styles::parse_style_ref(
            s,
            &format!("ops[{i}].style"),
        )?)),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "style".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].style"),
            ));
        }
    };
    let look = match obj.get("look") {
        None => None,
        Some(Value::Null) => Some(None),
        Some(Value::Object(m)) => {
            deny_keys(
                m,
                i,
                &[
                    "firstRow",
                    "lastRow",
                    "firstColumn",
                    "lastColumn",
                    "bandedRows",
                    "bandedColumns",
                ],
            )?;
            Some(Some(super::super::tbl::TableLookPatch {
                first_row: parse_opt_bool(m, i, "firstRow")?,
                last_row: parse_opt_bool(m, i, "lastRow")?,
                first_column: parse_opt_bool(m, i, "firstColumn")?,
                last_column: parse_opt_bool(m, i, "lastColumn")?,
                banded_rows: parse_opt_bool(m, i, "bandedRows")?,
                banded_columns: parse_opt_bool(m, i, "bandedColumns")?,
            }))
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "look".into(),
                    expected: TypeName::Object,
                },
                format!("ops[{i}].look"),
            ));
        }
    };
    if style.is_none() && look.is_none() {
        return Err(Error::with_path(
            ErrorKind::NeedSetTableStyleField,
            format!("ops[{i}]"),
        ));
    }
    Ok(SetTableStyleOp {
        id,
        target,
        style,
        look,
    })
}

pub(super) fn parse_set_table_float(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetTableFloatOp, Error> {
    deny_keys(obj, i, &["op", "id", "pos"])?;
    let id = require_op_id(obj, i)?;
    let target = tbl::parse_table_only(&id, i, IdReason::NeedTableId)?;
    let pos = match obj.get("pos") {
        Some(Value::Null) => None,
        Some(Value::Object(m)) => {
            deny_keys(
                m,
                i,
                &[
                    "horzAnchor",
                    "vertAnchor",
                    "x",
                    "y",
                    "leftFromText",
                    "rightFromText",
                    "topFromText",
                    "bottomFromText",
                ],
            )?;
            let horz = parse_opt_tbl_anchor(m, i, "horzAnchor")?;
            let vert = parse_opt_tbl_anchor(m, i, "vertAnchor")?;
            Some(super::super::tbl::TableFloat {
                horz_anchor: horz,
                vert_anchor: vert,
                x: parse_opt_i32(m, i, "x")?,
                y: parse_opt_i32(m, i, "y")?,
                left_from_text: parse_opt_u32_min(m, i, "leftFromText", 0)?,
                right_from_text: parse_opt_u32_min(m, i, "rightFromText", 0)?,
                top_from_text: parse_opt_u32_min(m, i, "topFromText", 0)?,
                bottom_from_text: parse_opt_u32_min(m, i, "bottomFromText", 0)?,
            })
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "pos".into(),
                    expected: TypeName::Object,
                },
                format!("ops[{i}].pos"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::NeedSetTableFloatField,
                format!("ops[{i}].pos"),
            ));
        }
    };
    Ok(SetTableFloatOp { id, target, pos })
}

pub(super) fn parse_set_table_width(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetTableWidthOp, Error> {
    deny_keys(obj, i, &["op", "id", "type", "w", "cols"])?;
    let id = require_op_id(obj, i)?;
    let target = tbl::parse_table_only(&id, i, IdReason::NeedTableId)?;
    let path = format!("ops[{i}]");
    let type_ = tbl::parse_opt_width_type(obj, &path)?;
    let w = tbl::parse_opt_twip(obj, &path, "w")?;
    let cols = tbl::parse_cols_array(obj.get("cols"), &format!("{path}.cols"))?;
    let patch = tbl::finish_width_patch(type_, w, cols, &path)?;
    Ok(SetTableWidthOp { id, target, patch })
}

pub(super) fn parse_set_table_align(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetTableAlignOp, Error> {
    deny_keys(obj, i, &["op", "id", "align", "indent"])?;
    let id = require_op_id(obj, i)?;
    let target = tbl::parse_table_only(&id, i, IdReason::NeedTableId)?;
    let align = parse_opt_null_enum(
        obj,
        i,
        "align",
        tbl::TBL_ALIGN_VALS,
        ErrorKind::InvalidTblAlign,
    )?;
    let indent = parse_opt_null_u32(obj, i, "indent")?;
    if align.is_none() && indent.is_none() {
        return Err(Error::with_path(
            ErrorKind::NeedSetTableAlignField,
            format!("ops[{i}]"),
        ));
    }
    Ok(SetTableAlignOp {
        id,
        target,
        align,
        indent,
    })
}

pub(super) fn parse_set_table_row_height(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetTableRowHeightOp, Error> {
    deny_keys(obj, i, &["op", "id", "h", "rule"])?;
    let id = require_op_id(obj, i)?;
    let target = tbl::parse_row_ref(&id, i, IdReason::SetTableRowHeightNeedsRow)?;
    let path = format!("ops[{i}]");
    let spec = tbl::parse_row_height_spec(obj, &path)?;
    Ok(SetTableRowHeightOp {
        id,
        target,
        h: spec.h,
        rule: spec.rule,
    })
}

pub(super) fn parse_set_table_cell_mar(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetTableCellMarOp, Error> {
    deny_keys(obj, i, &["op", "id", "top", "left", "bottom", "right"])?;
    let id = require_op_id(obj, i)?;
    let target = tbl::parse_table_only(&id, i, IdReason::NeedTableId)?;
    let top = parse_opt_null_u32(obj, i, "top")?;
    let left = parse_opt_null_u32(obj, i, "left")?;
    let bottom = parse_opt_null_u32(obj, i, "bottom")?;
    let right = parse_opt_null_u32(obj, i, "right")?;
    if top.is_none() && left.is_none() && bottom.is_none() && right.is_none() {
        return Err(Error::with_path(
            ErrorKind::NeedSetTableCellMarField,
            format!("ops[{i}]"),
        ));
    }
    Ok(SetTableCellMarOp {
        id,
        target,
        top,
        left,
        bottom,
        right,
    })
}

pub(super) fn parse_set_table_borders(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetTableBordersOp, Error> {
    deny_keys(obj, i, &["op", "id", "borders"])?;
    let id = require_op_id(obj, i)?;
    let target = tbl::parse_table_only(&id, i, IdReason::NeedTableId)?;
    let borders = match obj.get("borders") {
        None => {
            return Err(Error::with_path(
                ErrorKind::OpRequiresField {
                    op: "setTableBorders".into(),
                    field: "borders".into(),
                },
                format!("ops[{i}].borders"),
            ));
        }
        Some(v) => tbl::parse_table_borders_value(v, &format!("ops[{i}].borders"))?,
    };
    Ok(SetTableBordersOp {
        id,
        target,
        borders,
    })
}

pub(super) fn parse_set_cell_borders(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetCellBordersOp, Error> {
    deny_keys(obj, i, &["op", "id", "borders"])?;
    let id = require_op_id(obj, i)?;
    let target = tbl::parse_cell_ref(&id, i)?;
    let borders = match obj.get("borders") {
        None => {
            return Err(Error::with_path(
                ErrorKind::OpRequiresField {
                    op: "setCellBorders".into(),
                    field: "borders".into(),
                },
                format!("ops[{i}].borders"),
            ));
        }
        Some(v) => tbl::parse_cell_borders_value(v, &format!("ops[{i}].borders"))?,
    };
    Ok(SetCellBordersOp {
        id,
        target,
        borders,
    })
}

pub(super) fn parse_set_cell_format(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetCellFormatOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op",
            "id",
            "fill",
            "top",
            "left",
            "bottom",
            "right",
            "tl2br",
            "tr2bl",
            "vAlign",
            "noWrap",
            "textDirection",
            "fitText",
            "hideMark",
            "headers",
        ],
    )?;
    let id = require_op_id(obj, i)?;
    let target = tbl::parse_cell_ref(&id, i)?;
    let fill = parse_opt_null_rgb(obj, i, "fill")?;
    let top = parse_opt_null_u32(obj, i, "top")?;
    let left = parse_opt_null_u32(obj, i, "left")?;
    let bottom = parse_opt_null_u32(obj, i, "bottom")?;
    let right = parse_opt_null_u32(obj, i, "right")?;
    let tl2br = parse_opt_null_diagonal(obj, i, "tl2br")?;
    let tr2bl = parse_opt_null_diagonal(obj, i, "tr2bl")?;
    let v_align = parse_opt_null_enum(
        obj,
        i,
        "vAlign",
        tbl::CELL_VALIGN,
        ErrorKind::InvalidSectValue {
            field: "vAlign".into(),
        },
    )?;
    let no_wrap = parse_opt_null_bool(obj, i, "noWrap")?;
    let text_direction = parse_opt_null_enum(
        obj,
        i,
        "textDirection",
        tbl::CELL_TEXT_DIR,
        ErrorKind::InvalidSectValue {
            field: "textDirection".into(),
        },
    )?;
    let fit_text = parse_opt_null_bool(obj, i, "fitText")?;
    let hide_mark = parse_opt_null_bool(obj, i, "hideMark")?;
    let headers = parse_opt_null_headers(obj, i)?;
    if fill.is_none()
        && top.is_none()
        && left.is_none()
        && bottom.is_none()
        && right.is_none()
        && tl2br.is_none()
        && tr2bl.is_none()
        && v_align.is_none()
        && no_wrap.is_none()
        && text_direction.is_none()
        && fit_text.is_none()
        && hide_mark.is_none()
        && headers.is_none()
    {
        return Err(Error::with_path(
            ErrorKind::NeedSetCellFormatField,
            format!("ops[{i}]"),
        ));
    }
    Ok(SetCellFormatOp {
        id,
        target,
        fill,
        top,
        left,
        bottom,
        right,
        tl2br,
        tr2bl,
        v_align,
        no_wrap,
        text_direction,
        fit_text,
        hide_mark,
        headers,
    })
}

pub(super) fn parse_opt_null_headers(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<Option<Option<Vec<String>>>, Error> {
    match obj.get("headers") {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(None)),
        Some(Value::Array(arr)) => {
            let mut out = Vec::with_capacity(arr.len());
            for (j, item) in arr.iter().enumerate() {
                match item {
                    Value::String(s) if !s.is_empty() => out.push(s.clone()),
                    Value::String(_) => {
                        return Err(Error::with_path(
                            ErrorKind::WrongType {
                                field: "headers".into(),
                                expected: TypeName::NonEmptyString,
                            },
                            format!("ops[{i}].headers[{j}]"),
                        ));
                    }
                    _ => {
                        return Err(Error::with_path(
                            ErrorKind::WrongType {
                                field: "headers".into(),
                                expected: TypeName::NonEmptyString,
                            },
                            format!("ops[{i}].headers[{j}]"),
                        ));
                    }
                }
            }
            Ok(Some(Some(out)))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "headers".into(),
                expected: TypeName::Array,
            },
            format!("ops[{i}].headers"),
        )),
    }
}

pub(super) fn parse_opt_tbl_anchor(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    field: &str,
) -> Result<Option<String>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::String(s)) if super::super::tbl::TBL_ANCHORS.contains(&s.as_str()) => {
            Ok(Some(s.clone()))
        }
        Some(Value::String(_)) => Err(Error::with_path(
            ErrorKind::InvalidTblAnchor,
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

pub(super) fn parse_opt_i32(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    field: &str,
) -> Result<Option<i32>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::Number(n)) => {
            let v = n.as_i64().ok_or_else(|| {
                Error::with_path(
                    ErrorKind::WrongType {
                        field: field.into(),
                        expected: TypeName::Integer,
                    },
                    format!("ops[{i}].{field}"),
                )
            })?;
            i32::try_from(v).map(Some).map_err(|_| {
                Error::with_path(
                    ErrorKind::WrongType {
                        field: field.into(),
                        expected: TypeName::Integer,
                    },
                    format!("ops[{i}].{field}"),
                )
            })
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

pub(super) fn parse_opt_null_rgb(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    field: &str,
) -> Result<Option<Option<String>>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(None)),
        Some(Value::String(s)) if is_rgb_hex(s) => Ok(Some(Some(s.to_ascii_uppercase()))),
        Some(Value::String(_)) => Err(Error::with_path(
            ErrorKind::InvalidStyleColor,
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

pub(super) fn parse_opt_null_diagonal(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    field: &str,
) -> Result<Option<Option<super::super::tbl::DiagonalBorder>>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(None)),
        Some(Value::Object(m)) => {
            deny_keys(m, i, &["val", "sz", "color"])?;
            let val = match m.get("val") {
                None => "single".to_string(),
                Some(Value::String(s)) if super::super::tbl::is_diagonal_val(s) => s.clone(),
                Some(Value::String(_)) => {
                    return Err(Error::with_path(
                        ErrorKind::InvalidPageBorderVal,
                        format!("ops[{i}].{field}.val"),
                    ));
                }
                Some(_) => {
                    return Err(Error::with_path(
                        ErrorKind::WrongType {
                            field: "val".into(),
                            expected: TypeName::String,
                        },
                        format!("ops[{i}].{field}.val"),
                    ));
                }
            };
            let sz = parse_opt_u32_min(m, i, "sz", 0)?;
            let color = match m.get("color") {
                None => None,
                Some(Value::String(s)) if s.eq_ignore_ascii_case("auto") => Some("auto".into()),
                Some(Value::String(s)) if is_rgb_hex(s) => Some(s.to_ascii_uppercase()),
                Some(Value::String(_)) => {
                    return Err(Error::with_path(
                        ErrorKind::InvalidPageBorderColor,
                        format!("ops[{i}].{field}.color"),
                    ));
                }
                Some(_) => {
                    return Err(Error::with_path(
                        ErrorKind::WrongType {
                            field: "color".into(),
                            expected: TypeName::String,
                        },
                        format!("ops[{i}].{field}.color"),
                    ));
                }
            };
            Ok(Some(Some(super::super::tbl::DiagonalBorder {
                val,
                sz,
                color,
            })))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Object,
            },
            format!("ops[{i}].{field}"),
        )),
    }
}

pub(super) fn parse_col_after(v: Option<&Value>, i: usize) -> Result<u32, Error> {
    let path = format!("ops[{i}].after");
    match v {
        Some(Value::Number(n)) => {
            let Some(u) = n.as_u64() else {
                if n.as_i64().is_some_and(|x| x < 0) {
                    return Err(Error::with_path(
                        ErrorKind::MinValue {
                            field: "after".into(),
                            min: 0,
                        },
                        path,
                    ));
                }
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: "after".into(),
                        expected: TypeName::Integer,
                    },
                    path,
                ));
            };
            Ok(u as u32)
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "after".into(),
                expected: TypeName::Integer,
            },
            path,
        )),
        None => Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "insertCol".into(),
                field: "after".into(),
            },
            path,
        )),
    }
}
