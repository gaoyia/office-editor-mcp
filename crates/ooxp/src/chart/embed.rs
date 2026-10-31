//! 图表 embedding 工作簿：docx / pptx 共用。xlsx 图表不走这里。

use std::collections::HashMap;

use serde_json::{json, Value};

use crate::chart::{self, ChartType, NewSeries};
use crate::error::Error;
use crate::opc::Package;
use crate::xlsx;
use crate::xlsx::a1::{self, CellRef};
use crate::xml::Element;

pub const REL_PACKAGE: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/package";
pub const CT_EMBED_XLSX: &str = "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet";

pub const SHEET: &str = "Sheet1";

pub fn origin() -> CellRef {
    CellRef { col: 1, row: 1 }
}

pub fn next_part(pkg: &Package, dir: &str, stem: &str, ext: &str) -> String {
    pkg.first_free(|n| format!("{dir}/{stem}{n}{ext}"))
}

pub fn build(ty: ChartType, series: &[NewSeries], cats: &[Value]) -> Result<Vec<u8>, Error> {
    let cells: Vec<Value> = chart::data_table_cells(ty, series, cats, origin())
        .into_iter()
        .map(|(cell, value)| {
            json!({
                "ref": a1::format_cell(cell.col, cell.row),
                "value": value
            })
        })
        .collect();
    xlsx::create(&json!({ "sheets": [{ "name": SHEET, "cells": cells }] }))
}

pub fn read_formula(bytes: &[u8], formula: &str) -> Option<Vec<Value>> {
    let refer = chart::parse_a1_formula(formula)?;
    let sheet = refer.sheet.clone().unwrap_or_else(|| SHEET.to_string());
    let range = format!(
        "{}:{}",
        a1::format_cell(refer.start.col, refer.start.row),
        a1::format_cell(refer.end.col, refer.end.row)
    );
    let scope = xlsx::parse_scope(Some(&json!({
        "sheet": sheet,
        "range": range,
        "maxCells": 20000
    })))
    .ok()??;
    let tree = xlsx::inspect(bytes, Some(scope)).ok()?;
    let mut by_ref = HashMap::new();
    if let Some(cells) = tree["sheets"][0]["cells"].as_array() {
        for c in cells {
            if let Some(r) = c["ref"].as_str() {
                by_ref.insert(r.to_string(), c["value"].clone());
            }
        }
    }
    let mut out = Vec::new();
    let mut texts = Vec::new();
    let mut all_num = true;
    for cell in chart::walk_a1(&refer) {
        let a1 = a1::format_cell(cell.col, cell.row);
        match by_ref.get(&a1) {
            Some(Value::Number(n)) => {
                out.push(Value::Number(n.clone()));
                texts.push(n.to_string());
            }
            Some(Value::String(s)) => {
                all_num = false;
                out.push(Value::String(s.clone()));
                texts.push(s.clone());
            }
            Some(Value::Bool(b)) => {
                all_num = false;
                out.push(json!(b));
                texts.push(b.to_string());
            }
            Some(Value::Null) | None => {
                all_num = false;
                out.push(json!(""));
                texts.push(String::new());
            }
            Some(other) => {
                all_num = false;
                out.push(other.clone());
                texts.push(other.to_string());
            }
        }
    }
    if all_num {
        Some(out)
    } else {
        Some(texts.into_iter().map(Value::String).collect())
    }
}

pub fn patch_series(
    bytes: &[u8],
    ser: &Element,
    name: Option<&str>,
    cats: Option<&[Value]>,
    vals: Option<&[Value]>,
    sizes: Option<&[Value]>,
) -> Result<Vec<u8>, Error> {
    let mut ops = Vec::new();
    if let Some(n) = name {
        if let Some(f) = ser.child_by_local("tx").and_then(chart::formula_of) {
            push_formula_ops(&mut ops, &f, &[json!(n)]);
        }
    }
    if let Some(c) = cats {
        push_pts_ops(&mut ops, ser, "cat", "xVal", c);
    }
    if let Some(v) = vals {
        push_pts_ops(&mut ops, ser, "val", "yVal", v);
    }
    if let Some(s) = sizes {
        push_pts_ops(&mut ops, ser, "bubbleSize", "bubbleSize", s);
    }
    apply_ops(bytes, ops)
}

pub fn patch_formulas(bytes: &[u8], items: &[(&str, &[Value])]) -> Result<Vec<u8>, Error> {
    let mut ops = Vec::new();
    for (formula, values) in items {
        push_formula_ops(&mut ops, formula, values);
    }
    apply_ops(bytes, ops)
}

pub fn apply_ops(bytes: &[u8], ops: Vec<Value>) -> Result<Vec<u8>, Error> {
    if ops.is_empty() {
        return Ok(bytes.to_vec());
    }
    let parsed = xlsx::parse_ops(&ops)?;
    xlsx::apply(bytes, &parsed)
}

pub fn resized_formula(f: Option<&str>, n: usize) -> Option<String> {
    let f = f?;
    let refer = chart::parse_a1_formula(f)?;
    let refer = chart::resize_a1(&refer, n);
    let sheet = refer.sheet.as_deref().unwrap_or(SHEET);
    Some(chart::format_a1_formula(sheet, refer.start, refer.end))
}

fn push_pts_ops(ops: &mut Vec<Value>, ser: &Element, a: &str, b: &str, values: &[Value]) {
    let el = ser.child_by_local(a).or_else(|| ser.child_by_local(b));
    let Some(f) = el.and_then(chart::formula_of) else {
        return;
    };
    push_formula_ops(ops, &f, values);
}

fn push_formula_ops(ops: &mut Vec<Value>, formula: &str, values: &[Value]) {
    let Some(refer) = chart::parse_a1_formula(formula) else {
        return;
    };
    let sheet = refer.sheet.as_deref().unwrap_or(SHEET);
    let start = refer.start;
    for (i, v) in values.iter().enumerate() {
        let cell = if start.col == refer.end.col {
            CellRef {
                col: start.col,
                row: start.row + i as u32,
            }
        } else {
            CellRef {
                col: start.col + i as u32,
                row: start.row,
            }
        };
        let a1 = a1::format_cell(cell.col, cell.row);
        ops.push(json!({
            "op": "setCell",
            "id": format!("cell:{sheet}!{a1}"),
            "value": v
        }));
    }
}
