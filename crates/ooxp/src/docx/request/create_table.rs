use serde_json::Value;

use crate::error::{Error, ErrorKind, TypeName};

use super::create_types::*;

pub(crate) fn parse_tables(
    v: Option<&Value>,
    path: &str,
    para_n: usize,
) -> Result<Vec<NewTable>, Error> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    let arr = v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "tables".into(),
                expected: TypeName::Array,
            },
            path,
        )
    })?;
    let mut out = Vec::new();
    for (i, item) in arr.iter().enumerate() {
        let item_path = format!("{path}[{i}]");
        let obj = item.as_object().ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: item_path.clone(),
                    expected: TypeName::Object,
                },
                item_path.clone(),
            )
        })?;
        for k in obj.keys() {
            if !matches!(
                k.as_str(),
                "cells"
                    | "after"
                    | "style"
                    | "look"
                    | "pos"
                    | "align"
                    | "indent"
                    | "width"
                    | "cols"
                    | "rowHeights"
                    | "cellMar"
                    | "borders"
                    | "cellFormats"
            ) {
                return Err(Error::with_path(
                    ErrorKind::UnknownField { field: k.clone() },
                    format!("{item_path}.{k}"),
                ));
            }
        }
        let cells_v = obj.get("cells").ok_or_else(|| {
            Error::with_path(
                ErrorKind::MissingField {
                    field: "cells".into(),
                },
                format!("{item_path}.cells"),
            )
        })?;
        let cells = super::super::tbl::parse_create_cells(cells_v, &format!("{item_path}.cells"))?;
        let after = parse_table_after(obj.get("after"), &format!("{item_path}.after"), para_n)?;
        let rows = cells.len();
        let cols = cells.first().map(|r| r.len()).unwrap_or(0);
        let extra = super::doc_extra::parse_table_extra(obj, &item_path, rows, cols)?;
        out.push(NewTable {
            after,
            cells,
            extra,
        });
    }
    Ok(out)
}

pub(crate) fn parse_table_after(
    v: Option<&Value>,
    path: &str,
    para_n: usize,
) -> Result<u32, Error> {
    match v {
        None => Ok(para_n as u32),
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
            if u > para_n as u64 {
                return Err(Error::with_path(
                    ErrorKind::OutOfRange {
                        field: "after".into(),
                        min: 0,
                        max: para_n as i64,
                    },
                    path,
                ));
            }
            Ok(u as u32)
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "after".into(),
                expected: TypeName::Integer,
            },
            path,
        )),
    }
}
