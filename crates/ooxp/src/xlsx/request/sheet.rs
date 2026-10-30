//! xlsx request：sheet。

use serde_json::Value;

use crate::error::{Error, ErrorKind, IdReason, SheetNameReason, TypeName};

use super::super::a1;
use super::super::names;

use super::cell::*;
use super::common::*;
use super::layout::*;
use super::types::*;

pub(super) fn parse_insert_sheet(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertSheetOp, Error> {
    deny_keys(obj, i, &["op", "id", "name", "after"])?;
    let id = require_id(obj, i)?;
    if id != "xlsx:workbook" {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::InsertSheetNeedsWorkbook,
            },
            format!("ops[{i}].id"),
        ));
    }
    Ok(InsertSheetOp {
        name: parse_new_sheet_name(obj, i)?,
        after: parse_optional_after(obj, i, false)?,
    })
}

pub(super) fn parse_delete_sheet(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteSheetOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    Ok(DeleteSheetOp {
        sheet: parse_sheet_only_id(&id, i, "id", IdReason::DeleteSheetNeedsWorksheet)?,
    })
}

pub(super) fn parse_rename_sheet(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<RenameSheetOp, Error> {
    deny_keys(obj, i, &["op", "id", "name"])?;
    let id = require_id(obj, i)?;
    Ok(RenameSheetOp {
        sheet: parse_sheet_only_id(&id, i, "id", IdReason::SheetOpNeedsSheet)?,
        name: parse_new_sheet_name(obj, i)?,
    })
}

pub(super) fn parse_move_sheet(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<MoveSheetOp, Error> {
    deny_keys(obj, i, &["op", "id", "after"])?;
    let id = require_id(obj, i)?;
    Ok(MoveSheetOp {
        sheet: parse_sheet_only_id(&id, i, "id", IdReason::SheetOpNeedsSheet)?,
        after: parse_optional_after(obj, i, true)?,
    })
}

pub(super) fn parse_set_sheet_hidden(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetSheetHiddenOp, Error> {
    deny_keys(obj, i, &["op", "id", "hidden"])?;
    let id = require_id(obj, i)?;
    Ok(SetSheetHiddenOp {
        sheet: parse_sheet_only_id(&id, i, "id", IdReason::SheetOpNeedsSheet)?,
        hidden: parse_required_bool(obj.get("hidden"), i, "setSheetHidden", "hidden")?,
    })
}

pub(super) fn parse_insert_defined_name(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertDefinedNameOp, Error> {
    deny_keys(
        obj,
        i,
        &["op", "id", "name", "formula", "hidden", "localSheet"],
    )?;
    let id = require_id(obj, i)?;
    if id != "xlsx:workbook" {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::InsertDefinedNameNeedsWorkbook,
            },
            format!("ops[{i}].id"),
        ));
    }
    let name = parse_defined_name_name(obj.get("name"), i)?.ok_or_else(|| {
        Error::with_path(
            ErrorKind::MissingField {
                field: "name".into(),
            },
            format!("ops[{i}].name"),
        )
    })?;
    let formula = parse_defined_name_formula(obj.get("formula"), i)?.ok_or_else(|| {
        Error::with_path(
            ErrorKind::MissingField {
                field: "formula".into(),
            },
            format!("ops[{i}].formula"),
        )
    })?;
    let local_sheet = match obj.get("localSheet") {
        None => None,
        Some(Value::Null) => {
            return Err(Error::with_path(
                ErrorKind::InvalidDefinedName {
                    field: "localSheet".into(),
                },
                format!("ops[{i}].localSheet"),
            ));
        }
        Some(Value::String(s)) => {
            if s.is_empty() {
                return Err(Error::with_path(
                    ErrorKind::InvalidDefinedName {
                        field: "localSheet".into(),
                    },
                    format!("ops[{i}].localSheet"),
                ));
            }
            Some(s.clone())
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "localSheet".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].localSheet"),
            ));
        }
    };
    Ok(InsertDefinedNameOp {
        name,
        formula,
        hidden: parse_opt_bool(obj, i, "hidden")?,
        local_sheet,
    })
}

pub(super) fn parse_set_defined_name(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetDefinedNameOp, Error> {
    deny_keys(
        obj,
        i,
        &["op", "id", "name", "formula", "hidden", "localSheet"],
    )?;
    let id = require_id(obj, i)?;
    let index = parse_defined_name_id(&id, i)?;
    if let Some(Value::Null) = obj.get("hidden") {
        return Err(Error::with_path(
            ErrorKind::InvalidDefinedName {
                field: "hidden".into(),
            },
            format!("ops[{i}].hidden"),
        ));
    }
    let op = SetDefinedNameOp {
        index,
        name: parse_defined_name_name(obj.get("name"), i)?,
        formula: parse_defined_name_formula(obj.get("formula"), i)?,
        hidden: parse_opt_bool(obj, i, "hidden")?,
        local_sheet: parse_set_local_sheet(obj, i)?,
    };
    if !op.has_patch() {
        return Err(Error::with_path(
            ErrorKind::NeedSetDefinedNameField,
            format!("ops[{i}]"),
        ));
    }
    Ok(op)
}

pub(super) fn parse_delete_defined_name(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteDefinedNameOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    Ok(DeleteDefinedNameOp {
        index: parse_defined_name_id(&id, i)?,
    })
}

pub(super) fn parse_set_print_setup(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetPrintSetupOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op",
            "id",
            "left",
            "right",
            "top",
            "bottom",
            "header",
            "footer",
            "paperSize",
            "orientation",
            "scale",
            "fitToPage",
            "fitToWidth",
            "fitToHeight",
            "gridLines",
            "headings",
            "hCenter",
            "vCenter",
            "printArea",
            "rowBreaks",
            "colBreaks",
        ],
    )?;
    let id = require_id(obj, i)?;
    let sheet = parse_sheet_only_id(&id, i, "id", IdReason::SheetOpNeedsSheet)?;
    let op = SetPrintSetupOp {
        sheet,
        left: parse_print_inch(obj, i, "left")?,
        right: parse_print_inch(obj, i, "right")?,
        top: parse_print_inch(obj, i, "top")?,
        bottom: parse_print_inch(obj, i, "bottom")?,
        header: parse_print_inch(obj, i, "header")?,
        footer: parse_print_inch(obj, i, "footer")?,
        paper_size: parse_print_u32(obj, i, "paperSize", 1, u32::MAX)?,
        orientation: parse_print_orientation(obj, i)?,
        scale: parse_print_u32(obj, i, "scale", 10, 400)?,
        fit_to_page: parse_opt_bool(obj, i, "fitToPage")?,
        fit_to_width: parse_print_u32(obj, i, "fitToWidth", 0, u32::MAX)?,
        fit_to_height: parse_print_u32(obj, i, "fitToHeight", 0, u32::MAX)?,
        grid_lines: parse_opt_bool(obj, i, "gridLines")?,
        headings: parse_opt_bool(obj, i, "headings")?,
        h_center: parse_opt_bool(obj, i, "hCenter")?,
        v_center: parse_opt_bool(obj, i, "vCenter")?,
        print_area: parse_print_area(obj, i)?,
        row_breaks: parse_print_breaks(obj, i, "rowBreaks", 1_048_576)?,
        col_breaks: parse_print_breaks(obj, i, "colBreaks", 16_384)?,
    };
    if !op.has_patch() {
        return Err(Error::with_path(
            ErrorKind::NeedSetPrintSetupField,
            format!("ops[{i}]"),
        ));
    }
    Ok(op)
}

pub(super) fn parse_set_sheet_view(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetSheetViewOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op",
            "id",
            "zoom",
            "frozenRows",
            "frozenCols",
            "splitRows",
            "splitCols",
            "activeCell",
        ],
    )?;
    let id = require_id(obj, i)?;
    let sheet = parse_sheet_only_id(&id, i, "id", IdReason::SheetOpNeedsSheet)?;
    let op = SetSheetViewOp {
        sheet,
        zoom: parse_view_u32(obj, i, "zoom", 10, 400)?,
        frozen_rows: parse_view_u32(obj, i, "frozenRows", 0, 1_048_575)?,
        frozen_cols: parse_view_u32(obj, i, "frozenCols", 0, 16_383)?,
        split_rows: parse_view_u32(obj, i, "splitRows", 0, 1_048_575)?,
        split_cols: parse_view_u32(obj, i, "splitCols", 0, 16_383)?,
        active_cell: parse_view_cell(obj, i)?,
    };
    if op.has_freeze() && op.has_split() {
        let field = if op.split_rows.is_some() {
            "splitRows"
        } else {
            "splitCols"
        };
        return Err(Error::with_path(
            ErrorKind::InvalidSheetView {
                field: field.into(),
            },
            format!("ops[{i}].{field}"),
        ));
    }
    if !op.has_patch() {
        return Err(Error::with_path(
            ErrorKind::NeedSetSheetViewField,
            format!("ops[{i}]"),
        ));
    }
    Ok(op)
}

pub(super) fn parse_set_auto_filter(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetAutoFilterOp, Error> {
    deny_keys(obj, i, &["op", "id", "ref"])?;
    let id = require_id(obj, i)?;
    let sheet = parse_sheet_only_id(&id, i, "id", IdReason::SheetOpNeedsSheet)?;
    let refer = match obj.get("ref") {
        None => {
            return Err(Error::with_path(
                ErrorKind::NeedSetAutoFilterField,
                format!("ops[{i}]"),
            ));
        }
        Some(Value::Null) => None,
        Some(Value::String(s)) => {
            if s.trim().is_empty() {
                return Err(Error::with_path(
                    ErrorKind::InvalidAutoFilter {
                        field: "ref".into(),
                    },
                    format!("ops[{i}].ref"),
                ));
            }
            Some(s.clone())
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "ref".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].ref"),
            ));
        }
    };
    Ok(SetAutoFilterOp { sheet, refer })
}

pub(super) fn parse_set_sort_state(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetSortStateOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op",
            "id",
            "ref",
            "caseSensitive",
            "columnSort",
            "sortMethod",
            "columns",
        ],
    )?;
    let id = require_id(obj, i)?;
    let sheet = parse_sheet_only_id(&id, i, "id", IdReason::SheetOpNeedsSheet)?;
    let refer = match obj.get("ref") {
        None => {
            return Err(Error::with_path(
                ErrorKind::NeedSetSortStateField,
                format!("ops[{i}]"),
            ));
        }
        Some(Value::Null) => None,
        Some(Value::String(s)) => {
            if s.trim().is_empty() {
                return Err(Error::with_path(
                    ErrorKind::InvalidSortState {
                        field: "ref".into(),
                    },
                    format!("ops[{i}].ref"),
                ));
            }
            Some(s.clone())
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "ref".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].ref"),
            ));
        }
    };
    let Some(raw_ref) = refer.as_deref() else {
        for name in ["caseSensitive", "columnSort", "sortMethod", "columns"] {
            if obj.contains_key(name) {
                return Err(Error::with_path(
                    ErrorKind::InvalidSortState { field: name.into() },
                    format!("ops[{i}].{name}"),
                ));
            }
        }
        return Ok(SetSortStateOp {
            sheet,
            refer: None,
            case_sensitive: false,
            column_sort: false,
            sort_method: None,
            columns: Vec::new(),
        });
    };
    let (outer_start, outer_end) =
        a1::parse_range(raw_ref).map_err(|e| e.at(format!("ops[{i}].ref")))?;
    let formatted = a1::format_range(outer_start, outer_end);
    let case_sensitive = parse_opt_bool(obj, i, "caseSensitive")?.unwrap_or(false);
    let column_sort = parse_opt_bool(obj, i, "columnSort")?.unwrap_or(false);
    let sort_method = parse_sort_method(obj, i)?;
    let columns_v = obj
        .get("columns")
        .ok_or_else(|| Error::with_path(ErrorKind::NeedSetSortStateField, format!("ops[{i}]")))?;
    let arr = columns_v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "columns".into(),
                expected: TypeName::Array,
            },
            format!("ops[{i}].columns"),
        )
    })?;
    if arr.is_empty() {
        return Err(Error::with_path(
            ErrorKind::EmptyArray {
                field: "columns".into(),
            },
            format!("ops[{i}].columns"),
        ));
    }
    let mut columns = Vec::new();
    for (j, item) in arr.iter().enumerate() {
        let item_obj = item.as_object().ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: format!("columns[{j}]"),
                    expected: TypeName::Object,
                },
                format!("ops[{i}].columns[{j}]"),
            )
        })?;
        deny_keys_at(
            item_obj,
            &format!("ops[{i}].columns[{j}]"),
            &[
                "ref",
                "descending",
                "sortBy",
                "customList",
                "dxfId",
                "color",
                "iconSet",
                "iconId",
            ],
        )?;
        let raw = item_obj
            .get("ref")
            .and_then(Value::as_str)
            .filter(|s| !s.trim().is_empty())
            .ok_or_else(|| {
                Error::with_path(
                    ErrorKind::InvalidSortState {
                        field: "ref".into(),
                    },
                    format!("ops[{i}].columns[{j}].ref"),
                )
            })?;
        let (start, end) =
            a1::parse_range(raw).map_err(|e| e.at(format!("ops[{i}].columns[{j}].ref")))?;
        if !a1::range_contains((outer_start, outer_end), (start, end)) {
            return Err(Error::with_path(
                ErrorKind::InvalidSortState {
                    field: "ref".into(),
                },
                format!("ops[{i}].columns[{j}].ref"),
            ));
        }
        let descending = match item_obj.get("descending") {
            None => false,
            Some(Value::Bool(b)) => *b,
            Some(_) => {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: "descending".into(),
                        expected: TypeName::Boolean,
                    },
                    format!("ops[{i}].columns[{j}].descending"),
                ));
            }
        };
        columns.push(parse_sort_condition(
            item_obj,
            i,
            j,
            a1::format_range(start, end),
            descending,
        )?);
    }
    Ok(SetSortStateOp {
        sheet,
        refer: Some(formatted),
        case_sensitive,
        column_sort,
        sort_method,
        columns,
    })
}

pub(super) fn parse_sort_method(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<Option<String>, Error> {
    match obj.get("sortMethod") {
        None => Ok(None),
        Some(Value::String(s)) if s == "none" => Ok(None),
        Some(Value::String(s)) if s == "pinYin" || s == "stroke" => Ok(Some(s.clone())),
        Some(Value::String(_)) => Err(Error::with_path(
            ErrorKind::InvalidSortState {
                field: "sortMethod".into(),
            },
            format!("ops[{i}].sortMethod"),
        )),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "sortMethod".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].sortMethod"),
        )),
    }
}

pub(super) fn parse_sort_condition(
    item_obj: &serde_json::Map<String, Value>,
    i: usize,
    j: usize,
    refer: String,
    descending: bool,
) -> Result<SortConditionOp, Error> {
    let loc = format!("columns[{j}]");
    let sort_by = match item_obj.get("sortBy") {
        None => None,
        Some(Value::String(s)) if s == "value" => None,
        Some(Value::String(s)) if s == "cellColor" || s == "fontColor" || s == "icon" => {
            Some(s.clone())
        }
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidSortState {
                    field: "sortBy".into(),
                },
                format!("ops[{i}].{loc}.sortBy"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "sortBy".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].{loc}.sortBy"),
            ));
        }
    };
    let custom_list = match item_obj.get("customList") {
        None => None,
        Some(Value::String(s)) if !s.is_empty() => Some(s.clone()),
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidSortState {
                    field: "customList".into(),
                },
                format!("ops[{i}].{loc}.customList"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "customList".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].{loc}.customList"),
            ));
        }
    };
    let color = match item_obj.get("color") {
        None => None,
        Some(Value::Object(m)) => Some(parse_color_spec(m, i, &format!("{loc}.color"))?),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "color".into(),
                    expected: TypeName::Object,
                },
                format!("ops[{i}].{loc}.color"),
            ));
        }
    };
    let dxf_id = parse_opt_sort_u32(item_obj, i, &loc, "dxfId")?;
    let icon_set = match item_obj.get("iconSet") {
        None => None,
        Some(Value::String(s)) if ICON_SETS.contains(&s.as_str()) => Some(s.clone()),
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidSortState {
                    field: "iconSet".into(),
                },
                format!("ops[{i}].{loc}.iconSet"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "iconSet".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].{loc}.iconSet"),
            ));
        }
    };
    let icon_id = parse_opt_sort_u32(item_obj, i, &loc, "iconId")?;
    match sort_by.as_deref() {
        None => {
            reject_sort_fields(
                i,
                &loc,
                &[
                    ("color", color.is_some()),
                    ("dxfId", dxf_id.is_some()),
                    ("iconSet", icon_set.is_some()),
                    ("iconId", icon_id.is_some()),
                ],
            )?;
        }
        Some("cellColor") | Some("fontColor") => {
            reject_sort_fields(
                i,
                &loc,
                &[
                    ("customList", custom_list.is_some()),
                    ("iconSet", icon_set.is_some()),
                    ("iconId", icon_id.is_some()),
                ],
            )?;
            if color.is_some() == dxf_id.is_some() {
                return Err(Error::with_path(
                    ErrorKind::InvalidSortState {
                        field: "color".into(),
                    },
                    format!("ops[{i}].{loc}.color"),
                ));
            }
        }
        Some("icon") => {
            reject_sort_fields(
                i,
                &loc,
                &[
                    ("customList", custom_list.is_some()),
                    ("color", color.is_some()),
                    ("dxfId", dxf_id.is_some()),
                ],
            )?;
        }
        Some(_) => unreachable!("sortBy 已在上面收窄"),
    }
    Ok(SortConditionOp {
        refer,
        descending,
        sort_by,
        custom_list,
        dxf_id,
        color,
        icon_set,
        icon_id,
    })
}

pub(super) fn reject_sort_fields(
    i: usize,
    loc: &str,
    fields: &[(&str, bool)],
) -> Result<(), Error> {
    for (name, hit) in fields {
        if *hit {
            return Err(Error::with_path(
                ErrorKind::InvalidSortState {
                    field: (*name).into(),
                },
                format!("ops[{i}].{loc}.{name}"),
            ));
        }
    }
    Ok(())
}

pub(super) fn parse_opt_sort_u32(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    loc: &str,
    field: &str,
) -> Result<Option<u32>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::Number(n)) => {
            if let Some(u) = n.as_u64().filter(|v| *v <= u32::MAX as u64) {
                Ok(Some(u as u32))
            } else {
                Err(Error::with_path(
                    ErrorKind::InvalidSortState {
                        field: field.into(),
                    },
                    format!("ops[{i}].{loc}.{field}"),
                ))
            }
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Integer,
            },
            format!("ops[{i}].{loc}.{field}"),
        )),
    }
}

pub(super) fn parse_group_rows(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<GroupRowsOp, Error> {
    deny_keys(obj, i, &["op", "id", "rows"])?;
    let id = require_id(obj, i)?;
    let sheet = parse_sheet_only_id(&id, i, "id", IdReason::SheetOpNeedsSheet)?;
    let (row, to_row) = parse_required_row_span(obj, i, "rows")?;
    Ok(GroupRowsOp { sheet, row, to_row })
}

pub(super) fn parse_ungroup_rows(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<UngroupRowsOp, Error> {
    deny_keys(obj, i, &["op", "id", "rows"])?;
    let id = require_id(obj, i)?;
    let sheet = parse_sheet_only_id(&id, i, "id", IdReason::SheetOpNeedsSheet)?;
    let (row, to_row) = parse_required_row_span(obj, i, "rows")?;
    Ok(UngroupRowsOp { sheet, row, to_row })
}

pub(super) fn parse_group_cols(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<GroupColsOp, Error> {
    deny_keys(obj, i, &["op", "id", "col"])?;
    let id = require_id(obj, i)?;
    let sheet = parse_sheet_only_id(&id, i, "id", IdReason::SheetOpNeedsSheet)?;
    let (min_col, max_col) = parse_required_col_span(obj, i, "col")?;
    Ok(GroupColsOp {
        sheet,
        min_col,
        max_col,
    })
}

pub(super) fn parse_ungroup_cols(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<UngroupColsOp, Error> {
    deny_keys(obj, i, &["op", "id", "col"])?;
    let id = require_id(obj, i)?;
    let sheet = parse_sheet_only_id(&id, i, "id", IdReason::SheetOpNeedsSheet)?;
    let (min_col, max_col) = parse_required_col_span(obj, i, "col")?;
    Ok(UngroupColsOp {
        sheet,
        min_col,
        max_col,
    })
}

pub(super) fn parse_set_outline(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetOutlineOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op",
            "id",
            "summaryBelow",
            "summaryRight",
            "collapseRows",
            "collapseCols",
            "collapsed",
        ],
    )?;
    let id = require_id(obj, i)?;
    let sheet = parse_sheet_only_id(&id, i, "id", IdReason::SheetOpNeedsSheet)?;
    let summary_below = parse_opt_bool(obj, i, "summaryBelow")?;
    let summary_right = parse_opt_bool(obj, i, "summaryRight")?;
    let collapse_rows = parse_opt_row_span(obj, i, "collapseRows")?;
    let collapse_cols = parse_opt_col_span(obj, i, "collapseCols")?;
    let collapsed = parse_opt_bool(obj, i, "collapsed")?;
    if summary_below.is_none()
        && summary_right.is_none()
        && collapse_rows.is_none()
        && collapse_cols.is_none()
    {
        return Err(Error::with_path(
            ErrorKind::NeedSetOutlineField,
            format!("ops[{i}]"),
        ));
    }
    if (collapse_rows.is_some() || collapse_cols.is_some()) && collapsed.is_none() {
        return Err(Error::with_path(
            ErrorKind::InvalidOutline {
                field: "collapsed".into(),
            },
            format!("ops[{i}].collapsed"),
        ));
    }
    if collapsed.is_some() && collapse_rows.is_none() && collapse_cols.is_none() {
        return Err(Error::with_path(
            ErrorKind::InvalidOutline {
                field: "collapsed".into(),
            },
            format!("ops[{i}].collapsed"),
        ));
    }
    Ok(SetOutlineOp {
        sheet,
        summary_below,
        summary_right,
        collapse_rows,
        collapse_cols,
        collapsed,
    })
}

pub(super) fn parse_set_row_hidden(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetRowHiddenOp, Error> {
    deny_keys(obj, i, &["op", "id", "rows", "hidden"])?;
    let id = require_id(obj, i)?;
    let sheet = parse_sheet_only_id(&id, i, "id", IdReason::SheetOpNeedsSheet)?;
    let (row, to_row) = parse_required_row_span(obj, i, "rows")?;
    let hidden = parse_required_bool(obj.get("hidden"), i, "setRowHidden", "hidden")?;
    Ok(SetRowHiddenOp {
        sheet,
        row,
        to_row,
        hidden,
    })
}

pub(super) fn parse_set_col_hidden(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetColHiddenOp, Error> {
    deny_keys(obj, i, &["op", "id", "col", "hidden"])?;
    let id = require_id(obj, i)?;
    let sheet = parse_sheet_only_id(&id, i, "id", IdReason::SheetOpNeedsSheet)?;
    let (min_col, max_col) = parse_required_col_span(obj, i, "col")?;
    let hidden = parse_required_bool(obj.get("hidden"), i, "setColHidden", "hidden")?;
    Ok(SetColHiddenOp {
        sheet,
        min_col,
        max_col,
        hidden,
    })
}

pub(super) fn parse_required_col_span(
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
    a1::parse_col_span(raw).map_err(|e| e.at(format!("ops[{i}].{field}")))
}

pub(super) fn parse_opt_row_span(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    field: &str,
) -> Result<Option<(u32, u32)>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::String(s)) => a1::parse_row_span(s)
            .map(Some)
            .map_err(|e| e.at(format!("ops[{i}].{field}"))),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].{field}"),
        )),
    }
}

pub(super) fn parse_opt_col_span(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    field: &str,
) -> Result<Option<(u32, u32)>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::String(s)) => a1::parse_col_span(s)
            .map(Some)
            .map_err(|e| e.at(format!("ops[{i}].{field}"))),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].{field}"),
        )),
    }
}

pub(super) fn parse_view_u32(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    field: &str,
    min: u32,
    max: u32,
) -> Result<Option<u32>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::Number(n)) => {
            let v = n.as_u64().and_then(|x| u32::try_from(x).ok());
            let Some(v) = v.filter(|x| *x >= min && *x <= max) else {
                return Err(Error::with_path(
                    ErrorKind::InvalidSheetView {
                        field: field.into(),
                    },
                    format!("ops[{i}].{field}"),
                ));
            };
            Ok(Some(v))
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

pub(super) fn parse_view_cell(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<Option<String>, Error> {
    match obj.get("activeCell") {
        None => Ok(None),
        Some(Value::String(s)) => {
            let cell = a1::parse_cell(s).map_err(|_| {
                Error::with_path(
                    ErrorKind::InvalidSheetView {
                        field: "activeCell".into(),
                    },
                    format!("ops[{i}].activeCell"),
                )
            })?;
            Ok(Some(a1::format_cell(cell.col, cell.row)))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "activeCell".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].activeCell"),
        )),
    }
}

pub(super) fn parse_print_inch(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    field: &str,
) -> Result<Option<f64>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::Number(n)) => {
            let Some(v) = n.as_f64().filter(|x| x.is_finite() && *x >= 0.0) else {
                return Err(Error::with_path(
                    ErrorKind::InvalidPrintSetup {
                        field: field.into(),
                    },
                    format!("ops[{i}].{field}"),
                ));
            };
            Ok(Some(v))
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

pub(super) fn parse_print_u32(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    field: &str,
    min: u32,
    max: u32,
) -> Result<Option<u32>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::Number(n)) => {
            let v = n.as_u64().and_then(|x| u32::try_from(x).ok());
            let Some(v) = v.filter(|x| *x >= min && *x <= max) else {
                return Err(Error::with_path(
                    ErrorKind::InvalidPrintSetup {
                        field: field.into(),
                    },
                    format!("ops[{i}].{field}"),
                ));
            };
            Ok(Some(v))
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

pub(super) fn parse_print_orientation(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<Option<String>, Error> {
    match obj.get("orientation") {
        None => Ok(None),
        Some(Value::String(s)) if s == "portrait" || s == "landscape" => Ok(Some(s.clone())),
        Some(Value::String(_)) => Err(Error::with_path(
            ErrorKind::InvalidPrintSetup {
                field: "orientation".into(),
            },
            format!("ops[{i}].orientation"),
        )),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "orientation".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].orientation"),
        )),
    }
}

pub(super) fn parse_print_area(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<Option<Option<String>>, Error> {
    match obj.get("printArea") {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(None)),
        Some(Value::String(s)) => {
            if s.trim().is_empty() {
                return Err(Error::with_path(
                    ErrorKind::InvalidPrintSetup {
                        field: "printArea".into(),
                    },
                    format!("ops[{i}].printArea"),
                ));
            }
            Ok(Some(Some(s.clone())))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "printArea".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].printArea"),
        )),
    }
}

pub(super) fn parse_print_breaks(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    field: &str,
    max: u32,
) -> Result<Option<Vec<u32>>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::Array(arr)) => {
            let mut out = Vec::with_capacity(arr.len());
            for (j, v) in arr.iter().enumerate() {
                let Some(n) = v.as_u64().and_then(|x| u32::try_from(x).ok()) else {
                    return Err(Error::with_path(
                        ErrorKind::WrongType {
                            field: format!("{field}[{j}]"),
                            expected: TypeName::Integer,
                        },
                        format!("ops[{i}].{field}[{j}]"),
                    ));
                };
                if n < 1 || n > max {
                    return Err(Error::with_path(
                        ErrorKind::InvalidPrintSetup {
                            field: field.into(),
                        },
                        format!("ops[{i}].{field}[{j}]"),
                    ));
                }
                out.push(n);
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

pub(super) fn parse_defined_name_name(
    v: Option<&Value>,
    i: usize,
) -> Result<Option<String>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Null) => Err(Error::with_path(
            ErrorKind::InvalidDefinedName {
                field: "name".into(),
            },
            format!("ops[{i}].name"),
        )),
        Some(Value::String(s)) => {
            names::validate_name(s, i)?;
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

pub(super) fn parse_defined_name_formula(
    v: Option<&Value>,
    i: usize,
) -> Result<Option<String>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Null) => Err(Error::with_path(
            ErrorKind::InvalidDefinedName {
                field: "formula".into(),
            },
            format!("ops[{i}].formula"),
        )),
        Some(Value::String(s)) => {
            names::validate_formula(s, i)?;
            Ok(Some(s.clone()))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "formula".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].formula"),
        )),
    }
}

pub(super) fn parse_set_local_sheet(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<Option<Option<String>>, Error> {
    match obj.get("localSheet") {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(None)),
        Some(Value::String(s)) => {
            if s.is_empty() {
                return Err(Error::with_path(
                    ErrorKind::InvalidDefinedName {
                        field: "localSheet".into(),
                    },
                    format!("ops[{i}].localSheet"),
                ));
            }
            Ok(Some(Some(s.clone())))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "localSheet".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].localSheet"),
        )),
    }
}

pub(super) fn parse_defined_name_id(id: &str, op_index: usize) -> Result<usize, Error> {
    let Some(nstr) = id.strip_prefix("name:") else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::DefinedNamePrefix,
            },
            format!("ops[{op_index}].id"),
        ));
    };
    if nstr.is_empty() || nstr.contains('/') || !nstr.chars().all(|c| c.is_ascii_digit()) {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::NameIndexNotInt,
            },
            format!("ops[{op_index}].id"),
        ));
    }
    let n: usize = nstr.parse().map_err(|_| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::NameIndexNotInt,
            },
            format!("ops[{op_index}].id"),
        )
    })?;
    if n == 0 {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::NameIndexFromOne,
            },
            format!("ops[{op_index}].id"),
        ));
    }
    Ok(n)
}

pub(super) fn parse_new_sheet_name(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<String, Error> {
    match obj.get("name") {
        Some(Value::String(s)) => {
            validate_sheet_name(s, format!("ops[{i}].name"))?;
            Ok(s.clone())
        }
        None => Err(Error::with_path(
            ErrorKind::InvalidSheetName {
                reason: SheetNameReason::Missing,
            },
            format!("ops[{i}].name"),
        )),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "name".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].name"),
        )),
    }
}

pub(super) fn parse_optional_after(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    allow_null: bool,
) -> Result<Option<String>, Error> {
    match obj.get("after") {
        None => Ok(None),
        Some(Value::Null) if allow_null => Ok(None),
        Some(Value::String(s)) => Ok(Some(parse_sheet_only_id(
            s,
            i,
            "after",
            IdReason::SheetOpNeedsSheet,
        )?)),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "after".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].after"),
        )),
    }
}

pub(crate) fn validate_sheet_name(name: &str, path: impl Into<String>) -> Result<(), Error> {
    let path = path.into();
    if name.is_empty() || name.len() > 31 {
        return Err(Error::with_path(
            ErrorKind::InvalidSheetName {
                reason: SheetNameReason::Length,
            },
            path,
        ));
    }
    if name.eq_ignore_ascii_case("History") {
        return Err(Error::with_path(
            ErrorKind::InvalidSheetName {
                reason: SheetNameReason::History,
            },
            path,
        ));
    }
    if name.contains([':', '*', '?', '/', '[', ']', '\\']) {
        return Err(Error::with_path(
            ErrorKind::InvalidSheetName {
                reason: SheetNameReason::IllegalChars,
            },
            path,
        ));
    }
    Ok(())
}
