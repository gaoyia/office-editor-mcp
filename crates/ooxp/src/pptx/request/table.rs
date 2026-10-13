//! pptx request：table。

use serde_json::{Map, Value};

use crate::error::{Error, ErrorKind, IdReason, TypeName};

use super::super::table;

use super::common::*;
use super::link::*;
use super::types::*;

pub(super) fn parse_insert_table(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<InsertTableOp, Error> {
    deny_keys(
        obj,
        i,
        &["op", "id", "rows", "cols", "cells", "x", "y", "cx", "cy"],
    )?;
    let id = require_id(obj, i)?;
    let (slide, on_notes, master) = table::parse_insert_slide_id(&id, i)?.parts();
    let rows = table::parse_dim(
        obj.get("rows"),
        &format!("ops[{i}].rows"),
        "rows",
        1,
        table::max_rows(),
    )?;
    let cols = table::parse_dim(
        obj.get("cols"),
        &format!("ops[{i}].cols"),
        "cols",
        1,
        table::max_cols(),
    )?;
    let cells = table::parse_cells(obj.get("cells"), &format!("ops[{i}].cells"), rows, cols)?;
    let x = parse_off_field(obj.get("x"), i, "x")?;
    let y = parse_off_field(obj.get("y"), i, "y")?;
    if x.is_some() != y.is_some() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "insertTable".into(),
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
                op: "insertTable".into(),
                field: "cx and cy together".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(InsertTableOp {
        slide,
        on_notes,
        master,
        rows,
        cols,
        cells,
        x,
        y,
        cx,
        cy,
        col_widths: None,
        row_heights: None,
    })
}

pub(super) fn parse_delete_table(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<DeleteTableOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let target = table::parse_table_id(
        &id,
        &format!("ops[{i}].id"),
        IdReason::DeleteTableNeedsTable,
    )?;
    Ok(DeleteTableOp { id, target })
}

pub(super) fn parse_insert_row(obj: &Map<String, Value>, i: usize) -> Result<InsertRowOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let target = table::parse_table_or_row(&id, &format!("ops[{i}].id"))?;
    Ok(InsertRowOp { id, target })
}

pub(super) fn parse_delete_row(obj: &Map<String, Value>, i: usize) -> Result<DeleteRowOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let (table, row) = table::parse_row_id(&id, &format!("ops[{i}].id"))?;
    Ok(DeleteRowOp { id, table, row })
}

pub(super) fn parse_insert_col(obj: &Map<String, Value>, i: usize) -> Result<InsertColOp, Error> {
    deny_keys(obj, i, &["op", "id", "after"])?;
    let id = require_id(obj, i)?;
    let target = table::parse_table_id(&id, &format!("ops[{i}].id"), IdReason::NeedTableId)?;
    let after = table::parse_after(obj.get("after"), i)?;
    Ok(InsertColOp { id, target, after })
}

pub(super) fn parse_delete_col(obj: &Map<String, Value>, i: usize) -> Result<DeleteColOp, Error> {
    deny_keys(obj, i, &["op", "id", "col"])?;
    let id = require_id(obj, i)?;
    let target = table::parse_table_id(&id, &format!("ops[{i}].id"), IdReason::NeedTableId)?;
    let col = table::parse_dim(
        obj.get("col"),
        &format!("ops[{i}].col"),
        "col",
        1,
        i64::MAX as u64,
    )? as u32;
    Ok(DeleteColOp { id, target, col })
}

pub(super) fn parse_set_table_cell_text(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<SetTableCellTextOp, Error> {
    deny_keys(obj, i, &["op", "id", "text"])?;
    let id = require_id(obj, i)?;
    let target = table::parse_cell_id(&id, &format!("ops[{i}].id"))?;
    let text = match obj.get("text") {
        Some(Value::String(s)) => s.clone(),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "text".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].text"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::OpRequiresField {
                    op: "setTableCellText".into(),
                    field: "text".into(),
                },
                format!("ops[{i}].text"),
            ));
        }
    };
    Ok(SetTableCellTextOp { id, target, text })
}

pub(super) fn parse_set_table_pos(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<SetTablePosOp, Error> {
    deny_keys(obj, i, &["op", "id", "x", "y"])?;
    let id = require_id(obj, i)?;
    let target = table::parse_table_id(&id, &format!("ops[{i}].id"), IdReason::NeedTableId)?;
    let x = parse_off_field(obj.get("x"), i, "x")?;
    let y = parse_off_field(obj.get("y"), i, "y")?;
    if x.is_none() && y.is_none() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setTablePos".into(),
                field: "x or y".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(SetTablePosOp { id, target, x, y })
}

pub(super) fn parse_set_table_size(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<SetTableSizeOp, Error> {
    deny_keys(obj, i, &["op", "id", "cx", "cy"])?;
    let id = require_id(obj, i)?;
    let target = table::parse_table_id(&id, &format!("ops[{i}].id"), IdReason::NeedTableId)?;
    let cx = parse_emu_field(obj.get("cx"), i, "cx")?;
    let cy = parse_emu_field(obj.get("cy"), i, "cy")?;
    if cx.is_none() && cy.is_none() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setTableSize".into(),
                field: "cx or cy".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(SetTableSizeOp { id, target, cx, cy })
}

fn parse_pos_emu_array(
    v: Option<&Value>,
    path: &str,
    field: &str,
) -> Result<Option<Vec<i64>>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Array(arr)) => {
            let mut out = Vec::with_capacity(arr.len());
            for (j, item) in arr.iter().enumerate() {
                let n =
                    parse_emu_at(Some(item), &format!("{path}[{j}]"), field)?.ok_or_else(|| {
                        Error::with_path(
                            ErrorKind::WrongType {
                                field: field.into(),
                                expected: TypeName::Integer,
                            },
                            format!("{path}[{j}]"),
                        )
                    })?;
                out.push(n);
            }
            Ok(Some(out))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Array,
            },
            path.to_string(),
        )),
    }
}

fn parse_row_heights(v: Option<&Value>, path: &str) -> Result<Option<Vec<i64>>, Error> {
    parse_pos_emu_array(v, path, "rowHeights")
}

pub(super) fn parse_set_table_col_widths(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<SetTableColWidthsOp, Error> {
    deny_keys(obj, i, &["op", "id", "cols"])?;
    let id = require_id(obj, i)?;
    let target = table::parse_table_id(&id, &format!("ops[{i}].id"), IdReason::NeedTableId)?;
    let cols = parse_pos_emu_array(obj.get("cols"), &format!("ops[{i}].cols"), "cols")?
        .ok_or_else(|| {
            Error::with_path(
                ErrorKind::OpRequiresField {
                    op: "setTableColWidths".into(),
                    field: "cols".into(),
                },
                format!("ops[{i}].cols"),
            )
        })?;
    Ok(SetTableColWidthsOp { id, target, cols })
}

pub(super) fn parse_set_table_tr_height(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<SetTableTrHeightOp, Error> {
    deny_keys(obj, i, &["op", "id", "h"])?;
    let id = require_id(obj, i)?;
    let (table, row) = table::parse_row_id_with(
        &id,
        &format!("ops[{i}].id"),
        IdReason::SetTableTrHeightNeedsRow,
    )?;
    let h = parse_emu_field(obj.get("h"), i, "h")?.ok_or_else(|| {
        Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setTableTrHeight".into(),
                field: "h".into(),
            },
            format!("ops[{i}].h"),
        )
    })?;
    Ok(SetTableTrHeightOp { id, table, row, h })
}

pub(crate) fn parse_create_table(
    v: &Value,
    prefix: &str,
    slide: usize,
) -> Result<CreateTable, Error> {
    let obj = v.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "tables".into(),
                expected: TypeName::Object,
            },
            prefix.to_string(),
        )
    })?;
    deny_keys_at(
        obj,
        prefix,
        &[
            "rows",
            "cols",
            "cells",
            "x",
            "y",
            "cx",
            "cy",
            "hyperlink",
            "colWidths",
            "rowHeights",
        ],
    )?;
    let rows = table::parse_dim(
        obj.get("rows"),
        &format!("{prefix}.rows"),
        "rows",
        1,
        table::max_rows(),
    )?;
    let cols = table::parse_dim(
        obj.get("cols"),
        &format!("{prefix}.cols"),
        "cols",
        1,
        table::max_cols(),
    )?;
    let (cells, cell_links) =
        parse_create_table_cells(obj.get("cells"), &format!("{prefix}.cells"), rows, cols)?;
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
    let col_widths = parse_pos_emu_array(
        obj.get("colWidths"),
        &format!("{prefix}.colWidths"),
        "colWidths",
    )?;
    if let Some(ws) = &col_widths {
        if ws.len() != cols {
            return Err(Error::with_path(
                ErrorKind::NeedTableColWidthsMatch,
                format!("{prefix}.colWidths"),
            ));
        }
    }
    let row_heights = parse_row_heights(obj.get("rowHeights"), &format!("{prefix}.rowHeights"))?;
    if let Some(hs) = &row_heights {
        if hs.len() != rows {
            return Err(Error::with_path(
                ErrorKind::NeedTableRowHeightsMatch,
                format!("{prefix}.rowHeights"),
            ));
        }
    }
    let op = InsertTableOp {
        slide,
        on_notes: false,
        master: None,
        rows,
        cols,
        cells,
        x,
        y,
        cx,
        cy,
        col_widths,
        row_heights,
    };
    let link = parse_create_opt_hyperlink(obj, prefix)?;
    Ok(CreateTable {
        op,
        link,
        cell_links,
    })
}

pub(super) fn parse_create_table_cells(
    v: Option<&Value>,
    path: &str,
    rows: usize,
    cols: usize,
) -> Result<(Vec<Vec<String>>, Vec<CreateTableCellLink>), Error> {
    match v {
        None => Ok((vec![vec![String::new(); cols]; rows], Vec::new())),
        Some(Value::Array(arr)) => {
            if arr.len() != rows {
                return Err(Error::with_path(
                    ErrorKind::NeedTableCellsMatch,
                    path.to_string(),
                ));
            }
            let mut texts = Vec::new();
            let mut links = Vec::new();
            for (ri, row) in arr.iter().enumerate() {
                let Some(cells) = row.as_array() else {
                    return Err(Error::with_path(
                        ErrorKind::WrongType {
                            field: "cells".into(),
                            expected: TypeName::Array,
                        },
                        format!("{path}[{ri}]"),
                    ));
                };
                if cells.len() != cols {
                    return Err(Error::with_path(
                        ErrorKind::NeedTableCellsMatch,
                        format!("{path}[{ri}]"),
                    ));
                }
                let mut line = Vec::new();
                for (ci, c) in cells.iter().enumerate() {
                    let cell_path = format!("{path}[{ri}][{ci}]");
                    match c {
                        Value::String(s) => line.push(s.clone()),
                        Value::Object(obj) => {
                            deny_keys_at(
                                obj,
                                &cell_path,
                                &["text", "url", "file", "slide", "tooltip", "event"],
                            )?;
                            let text = match obj.get("text") {
                                None => String::new(),
                                Some(Value::String(s)) => s.clone(),
                                Some(_) => {
                                    return Err(Error::with_path(
                                        ErrorKind::WrongType {
                                            field: "text".into(),
                                            expected: TypeName::String,
                                        },
                                        format!("{cell_path}.text"),
                                    ));
                                }
                            };
                            if let Some(link) = parse_create_link_fields(obj, &cell_path, false)? {
                                links.push(CreateTableCellLink {
                                    row: ri + 1,
                                    col: ci + 1,
                                    link,
                                });
                            }
                            line.push(text);
                        }
                        _ => {
                            return Err(Error::with_path(
                                ErrorKind::WrongType {
                                    field: "cells".into(),
                                    expected: TypeName::String,
                                },
                                cell_path,
                            ));
                        }
                    }
                }
                texts.push(line);
            }
            Ok((texts, links))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "cells".into(),
                expected: TypeName::Array,
            },
            path.to_string(),
        )),
    }
}
