//! create 第四轮与第五轮：把骨架之外的字段解析成已有 apply op。

use std::collections::HashMap;

use serde_json::{json, Map, Value};

use super::super::a1;
use super::super::apply;
use super::super::inspect;
use super::XlsxOp;
use crate::error::{Error, ErrorKind, TypeName};

pub(crate) const WORKBOOK_REMAIN_KEYS: &[&str] = &[
    "definedNames",
    "connections",
    "externalLinks",
    "customViews",
    "cellStyles",
    "workbookProtection",
];

pub(crate) const SHEET_REMAIN_KEYS: &[&str] = &[
    "cols",
    "rows",
    "merges",
    "dataValidations",
    "conditionalFormats",
    "tables",
    "pivots",
    "print",
    "view",
    "autoFilter",
    "sortState",
    "groupRows",
    "groupCols",
    "outline",
    "sheetProtection",
    "protectedRanges",
    "sparklines",
    "oleObjects",
    "slicers",
    "groups",
];

pub(crate) const CHARTSHEET_REMAIN_KEYS: &[&str] = &["print", "view", "sheetProtection"];

pub(crate) const CELL_REMAIN_KEYS: &[&str] = &[
    "shared",
    "sharedRef",
    "array",
    "arrayRef",
    "numFmt",
    "font",
    "fill",
    "border",
    "align",
    "namedStyle",
];

const CELL_STYLE_KEYS: &[&str] = &["numFmt", "font", "fill", "border", "align"];
const CELL_SET_KEYS: &[&str] = &[
    "formula",
    "value",
    "shared",
    "sharedRef",
    "array",
    "arrayRef",
];

const SKELETON_SHEET_KEYS: &[&str] = &[
    "name",
    "kind",
    "cells",
    "pictures",
    "charts",
    "headerFooters",
    "comments",
    "hyperlinks",
    "textBoxes",
    "shapes",
    "connectors",
    "smartArts",
    "model3ds",
    "inks",
    "maths",
];

const SKELETON_CHARTSHEET_KEYS: &[&str] = &[
    "name",
    "kind",
    "type",
    "ofPieType",
    "stockKind",
    "surfaceKind",
    "layoutId",
    "title",
    "series",
    "dataFrom",
    "dataSheet",
    "cx",
    "cy",
];

pub(crate) fn is_workbook_key(k: &str) -> bool {
    matches!(k, "sheets" | "props" | "theme") || WORKBOOK_REMAIN_KEYS.contains(&k)
}

pub(crate) fn is_sheet_key(k: &str) -> bool {
    SKELETON_SHEET_KEYS.contains(&k) || SHEET_REMAIN_KEYS.contains(&k)
}

pub(crate) fn is_chartsheet_key(k: &str) -> bool {
    SKELETON_CHARTSHEET_KEYS.contains(&k) || CHARTSHEET_REMAIN_KEYS.contains(&k)
}

pub(crate) fn is_cell_key(k: &str) -> bool {
    matches!(k, "ref" | "value" | "formula") || CELL_REMAIN_KEYS.contains(&k)
}

/// 骨架写完后按契约顺序复用 apply。
pub(crate) fn apply_remain(
    bytes: Vec<u8>,
    workbook: &Map<String, Value>,
) -> Result<Vec<u8>, Error> {
    let mut ops = Vec::new();
    let mut paths = Vec::new();
    collect_all(workbook, &mut ops, &mut paths)?;
    if has_groups(workbook) {
        append_group_ops(&bytes, workbook, &mut ops, &mut paths)?;
    }
    if has_comment_replies(workbook) {
        append_reply_ops(&bytes, workbook, &mut ops, &mut paths)?;
    }
    if ops.is_empty() {
        return Ok(bytes);
    }
    apply::apply(&bytes, &ops).map_err(|e| remap_ops_error(e, &paths))
}

fn collect_all(
    workbook: &Map<String, Value>,
    ops: &mut Vec<XlsxOp>,
    paths: &mut Vec<String>,
) -> Result<(), Error> {
    let style_map = collect_cell_styles(workbook, ops, paths)?;
    let sheets = workbook["sheets"].as_array().expect("create 已校验 sheets");
    for (i, sheet) in sheets.iter().enumerate() {
        let Some(obj) = sheet.as_object() else {
            continue;
        };
        let prefix = format!("workbook.sheets[{i}]");
        let name = sheet_name(obj);
        let id = format!("sheet:{name}");
        collect_cells(obj, name, &prefix, &style_map, ops, paths)?;
        if is_chartsheet(obj) {
            continue;
        }
        collect_cols(obj, &id, &prefix, ops, paths)?;
        collect_rows(obj, &id, &prefix, ops, paths)?;
        collect_string_ops(
            obj,
            "merges",
            "range",
            "mergeRange",
            &id,
            &prefix,
            ops,
            paths,
        )?;
    }
    for (i, sheet) in sheets.iter().enumerate() {
        let Some(obj) = sheet.as_object() else {
            continue;
        };
        let prefix = format!("workbook.sheets[{i}]");
        let id = format!("sheet:{}", sheet_name(obj));
        collect_object_op(obj, "print", "setPrintSetup", &id, &prefix, ops, paths)?;
        collect_object_op(obj, "view", "setSheetView", &id, &prefix, ops, paths)?;
        if is_chartsheet(obj) {
            continue;
        }
        collect_object_op(obj, "autoFilter", "setAutoFilter", &id, &prefix, ops, paths)?;
        collect_object_op(obj, "sortState", "setSortState", &id, &prefix, ops, paths)?;
        collect_string_ops(
            obj,
            "groupRows",
            "rows",
            "groupRows",
            &id,
            &prefix,
            ops,
            paths,
        )?;
        collect_string_ops(
            obj,
            "groupCols",
            "col",
            "groupCols",
            &id,
            &prefix,
            ops,
            paths,
        )?;
        collect_object_op(obj, "outline", "setOutline", &id, &prefix, ops, paths)?;
    }
    for (i, sheet) in sheets.iter().enumerate() {
        let Some(obj) = sheet.as_object() else {
            continue;
        };
        if is_chartsheet(obj) {
            continue;
        }
        let prefix = format!("workbook.sheets[{i}]");
        let id = format!("sheet:{}", sheet_name(obj));
        collect_insert_array(
            obj,
            "dataValidations",
            "insertDataValidation",
            &id,
            &prefix,
            ops,
            paths,
        )?;
        collect_insert_array(
            obj,
            "conditionalFormats",
            "insertConditionalFormat",
            &id,
            &prefix,
            ops,
            paths,
        )?;
        collect_insert_array(obj, "tables", "insertSheetTable", &id, &prefix, ops, paths)?;
        collect_insert_array(obj, "pivots", "insertPivotTable", &id, &prefix, ops, paths)?;
    }
    collect_insert_array(
        workbook,
        "definedNames",
        "insertDefinedName",
        "xlsx:workbook",
        "workbook",
        ops,
        paths,
    )?;
    collect_insert_array(
        workbook,
        "connections",
        "insertConnection",
        "xlsx:workbook",
        "workbook",
        ops,
        paths,
    )?;
    collect_insert_array(
        workbook,
        "externalLinks",
        "insertExternalLink",
        "xlsx:workbook",
        "workbook",
        ops,
        paths,
    )?;
    for (i, sheet) in sheets.iter().enumerate() {
        let Some(obj) = sheet.as_object() else {
            continue;
        };
        let prefix = format!("workbook.sheets[{i}]");
        let id = format!("sheet:{}", sheet_name(obj));
        collect_object_op(
            obj,
            "sheetProtection",
            "setProtection",
            &id,
            &prefix,
            ops,
            paths,
        )?;
        if is_chartsheet(obj) {
            continue;
        }
        collect_insert_array(
            obj,
            "protectedRanges",
            "insertProtectedRange",
            &id,
            &prefix,
            ops,
            paths,
        )?;
    }
    collect_object_op(
        workbook,
        "workbookProtection",
        "setProtection",
        "xlsx:workbook",
        "workbook",
        ops,
        paths,
    )?;
    for (i, sheet) in sheets.iter().enumerate() {
        let Some(obj) = sheet.as_object() else {
            continue;
        };
        if is_chartsheet(obj) {
            continue;
        }
        let prefix = format!("workbook.sheets[{i}]");
        let id = format!("sheet:{}", sheet_name(obj));
        collect_insert_array(
            obj,
            "sparklines",
            "insertSparkline",
            &id,
            &prefix,
            ops,
            paths,
        )?;
        collect_insert_array(obj, "oleObjects", "insertOle", &id, &prefix, ops, paths)?;
        collect_insert_array(obj, "slicers", "insertSlicer", &id, &prefix, ops, paths)?;
    }
    collect_insert_array(
        workbook,
        "customViews",
        "insertCustomView",
        "xlsx:workbook",
        "workbook",
        ops,
        paths,
    )?;
    if let Some(arr) = optional_array(workbook, "sheets", "workbook")? {
        for (i, sheet) in arr.iter().enumerate() {
            let Some(obj) = sheet.as_object() else {
                continue;
            };
            if let Some(v) = obj.get("groups") {
                require_array(v, &format!("workbook.sheets[{i}].groups"))?;
            }
        }
    }
    Ok(())
}

fn collect_cell_styles(
    workbook: &Map<String, Value>,
    ops: &mut Vec<XlsxOp>,
    paths: &mut Vec<String>,
) -> Result<HashMap<String, usize>, Error> {
    let mut map = HashMap::new();
    map.insert("normal".into(), 1usize);
    collect_insert_array(
        workbook,
        "cellStyles",
        "insertCellStyle",
        "xlsx:workbook",
        "workbook",
        ops,
        paths,
    )?;
    let Some(arr) = optional_array(workbook, "cellStyles", "workbook")? else {
        return Ok(map);
    };
    for (j, item) in arr.iter().enumerate() {
        let cp = format!("workbook.cellStyles[{j}]");
        let obj = require_object(item, &cp)?;
        if let Some(Value::String(name)) = obj.get("name") {
            if !name.is_empty() {
                map.insert(name.to_ascii_lowercase(), j + 2);
            }
        }
    }
    Ok(map)
}

fn collect_cells(
    sheet: &Map<String, Value>,
    name: &str,
    prefix: &str,
    style_map: &HashMap<String, usize>,
    ops: &mut Vec<XlsxOp>,
    paths: &mut Vec<String>,
) -> Result<(), Error> {
    let Some(arr) = optional_array(sheet, "cells", prefix)? else {
        return Ok(());
    };
    for (j, cell) in arr.iter().enumerate() {
        let cp = format!("{prefix}.cells[{j}]");
        let obj = require_object(cell, &cp)?;
        let refer = match obj.get("ref").and_then(Value::as_str) {
            Some(s) => s,
            None => continue,
        };
        let parsed = a1::parse_cell(refer).map_err(|e| e.at(format!("{cp}.ref")))?;
        let id = format!("cell:{name}!{}", a1::format_cell(parsed.col, parsed.row));
        if cell_has_set_fields(obj) {
            let mut m = Map::new();
            for k in CELL_SET_KEYS {
                if let Some(v) = obj.get(*k) {
                    m.insert((*k).to_string(), v.clone());
                }
            }
            push_op(ops, paths, m, "setCell", &id, &cp)?;
        }
        if cell_has_style(obj) {
            let mut m = Map::new();
            for k in CELL_STYLE_KEYS {
                if let Some(v) = obj.get(*k) {
                    m.insert((*k).to_string(), v.clone());
                }
            }
            push_op(ops, paths, m, "setCellStyle", &id, &cp)?;
        }
        if let Some(v) = obj.get("namedStyle") {
            if cell_has_style(obj) {
                let other = CELL_STYLE_KEYS
                    .iter()
                    .copied()
                    .find(|k| obj.contains_key(*k))
                    .unwrap_or("numFmt");
                return Err(Error::with_path(
                    ErrorKind::MutuallyExclusive {
                        a: "namedStyle".into(),
                        b: other.into(),
                    },
                    cp,
                ));
            }
            let style_name = v.as_str().ok_or_else(|| {
                Error::with_path(
                    ErrorKind::WrongType {
                        field: "namedStyle".into(),
                        expected: TypeName::String,
                    },
                    format!("{cp}.namedStyle"),
                )
            })?;
            let Some(&index) = style_map.get(&style_name.to_ascii_lowercase()) else {
                return Err(Error::with_path(
                    ErrorKind::InvalidNamedCellStyle {
                        field: "namedStyle".into(),
                    },
                    format!("{cp}.namedStyle"),
                ));
            };
            let mut m = Map::new();
            m.insert("style".into(), json!(format!("cellStyle:{index}")));
            push_op(ops, paths, m, "setCellNamedStyle", &id, &cp)?;
        }
    }
    Ok(())
}

fn cell_has_set_fields(obj: &Map<String, Value>) -> bool {
    obj.contains_key("shared")
        || obj.contains_key("sharedRef")
        || obj.contains_key("array")
        || obj.contains_key("arrayRef")
}

fn cell_has_style(obj: &Map<String, Value>) -> bool {
    CELL_STYLE_KEYS.iter().any(|k| obj.contains_key(*k))
}

fn collect_cols(
    sheet: &Map<String, Value>,
    sheet_id: &str,
    prefix: &str,
    ops: &mut Vec<XlsxOp>,
    paths: &mut Vec<String>,
) -> Result<(), Error> {
    let Some(arr) = optional_array(sheet, "cols", prefix)? else {
        return Ok(());
    };
    for (j, item) in arr.iter().enumerate() {
        let cp = format!("{prefix}.cols[{j}]");
        let obj = require_object(item, &cp)?;
        deny_create_meta(obj, &cp)?;
        deny_extra(obj, &["col", "width", "bestFit", "hidden"], &cp)?;
        if !obj.contains_key("col") {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "col".into(),
                },
                format!("{cp}.col"),
            ));
        }
        let has_w = obj.contains_key("width") || obj.contains_key("bestFit");
        let has_h = obj.contains_key("hidden");
        if !has_w && !has_h {
            return Err(Error::with_path(ErrorKind::NeedSetColWidthField, cp));
        }
        if has_w {
            let mut m = Map::new();
            m.insert("col".into(), obj["col"].clone());
            if let Some(v) = obj.get("width") {
                m.insert("width".into(), v.clone());
            }
            if let Some(v) = obj.get("bestFit") {
                m.insert("bestFit".into(), v.clone());
            }
            push_op(ops, paths, m, "setColWidth", sheet_id, &cp)?;
        }
        if has_h {
            let mut m = Map::new();
            m.insert("col".into(), obj["col"].clone());
            m.insert("hidden".into(), obj["hidden"].clone());
            push_op(ops, paths, m, "setColHidden", sheet_id, &cp)?;
        }
    }
    Ok(())
}

fn collect_rows(
    sheet: &Map<String, Value>,
    sheet_id: &str,
    prefix: &str,
    ops: &mut Vec<XlsxOp>,
    paths: &mut Vec<String>,
) -> Result<(), Error> {
    let Some(arr) = optional_array(sheet, "rows", prefix)? else {
        return Ok(());
    };
    for (j, item) in arr.iter().enumerate() {
        let cp = format!("{prefix}.rows[{j}]");
        let obj = require_object(item, &cp)?;
        deny_create_meta(obj, &cp)?;
        deny_extra(obj, &["row", "toRow", "height", "hidden"], &cp)?;
        if !obj.contains_key("row") {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "row".into(),
                },
                format!("{cp}.row"),
            ));
        }
        let has_h = obj.contains_key("height");
        let has_hid = obj.contains_key("hidden");
        if !has_h && !has_hid {
            return Err(Error::with_path(ErrorKind::NeedSetRowHeightField, cp));
        }
        if has_h {
            let mut m = Map::new();
            m.insert("row".into(), obj["row"].clone());
            if let Some(v) = obj.get("toRow") {
                m.insert("toRow".into(), v.clone());
            }
            m.insert("height".into(), obj["height"].clone());
            push_op(ops, paths, m, "setRowHeight", sheet_id, &cp)?;
        }
        if has_hid {
            let mut m = Map::new();
            m.insert("rows".into(), json!(row_span_string(obj, &cp)?));
            m.insert("hidden".into(), obj["hidden"].clone());
            push_op(ops, paths, m, "setRowHidden", sheet_id, &cp)?;
        }
    }
    Ok(())
}

fn collect_string_ops(
    sheet: &Map<String, Value>,
    key: &str,
    field: &str,
    op: &str,
    sheet_id: &str,
    prefix: &str,
    ops: &mut Vec<XlsxOp>,
    paths: &mut Vec<String>,
) -> Result<(), Error> {
    let Some(arr) = optional_array(sheet, key, prefix)? else {
        return Ok(());
    };
    for (j, item) in arr.iter().enumerate() {
        let cp = format!("{prefix}.{key}[{j}]");
        let s = match item {
            Value::String(s) => s,
            _ => {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: format!("{key}[{j}]"),
                        expected: TypeName::String,
                    },
                    cp,
                ));
            }
        };
        let mut m = Map::new();
        m.insert(field.to_string(), json!(s));
        push_op(ops, paths, m, op, sheet_id, &cp)?;
    }
    Ok(())
}

fn collect_insert_array(
    host: &Map<String, Value>,
    key: &str,
    op: &str,
    id: &str,
    prefix: &str,
    ops: &mut Vec<XlsxOp>,
    paths: &mut Vec<String>,
) -> Result<(), Error> {
    let Some(arr) = optional_array(host, key, prefix)? else {
        return Ok(());
    };
    for (j, item) in arr.iter().enumerate() {
        let cp = format!("{prefix}.{key}[{j}]");
        let obj = require_object(item, &cp)?;
        deny_create_meta(obj, &cp)?;
        push_op(ops, paths, obj.clone(), op, id, &cp)?;
    }
    Ok(())
}

fn collect_object_op(
    host: &Map<String, Value>,
    key: &str,
    op: &str,
    id: &str,
    prefix: &str,
    ops: &mut Vec<XlsxOp>,
    paths: &mut Vec<String>,
) -> Result<(), Error> {
    let Some(v) = host.get(key) else {
        return Ok(());
    };
    let cp = format!("{prefix}.{key}");
    let obj = require_object(v, &cp)?;
    deny_create_meta(obj, &cp)?;
    push_op(ops, paths, obj.clone(), op, id, &cp)?;
    Ok(())
}

fn has_groups(workbook: &Map<String, Value>) -> bool {
    let Some(sheets) = workbook.get("sheets").and_then(Value::as_array) else {
        return false;
    };
    sheets.iter().any(|s| {
        s.get("groups")
            .and_then(Value::as_array)
            .is_some_and(|a| !a.is_empty())
    })
}

fn has_comment_replies(workbook: &Map<String, Value>) -> bool {
    let Some(sheets) = workbook.get("sheets").and_then(Value::as_array) else {
        return false;
    };
    sheets.iter().any(|s| {
        s.get("comments")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .any(|c| {
                c.get("replies")
                    .and_then(Value::as_array)
                    .is_some_and(|a| !a.is_empty())
            })
    })
}

fn append_reply_ops(
    bytes: &[u8],
    workbook: &Map<String, Value>,
    ops: &mut Vec<XlsxOp>,
    paths: &mut Vec<String>,
) -> Result<(), Error> {
    let scope =
        super::parse_scope(Some(&json!({ "comments": true })))?.expect("批注 inspect 范围非空");
    let tree = inspect::inspect(bytes, Some(scope))?;
    let sheets = workbook["sheets"].as_array().expect("create 已校验 sheets");
    for (i, sheet) in sheets.iter().enumerate() {
        let Some(obj) = sheet.as_object() else {
            continue;
        };
        let prefix = format!("workbook.sheets[{i}]");
        let Some(arr) = optional_array(obj, "comments", &prefix)? else {
            continue;
        };
        let name = sheet_name(obj);
        let Some(ins) = tree["sheets"]
            .as_array()
            .and_then(|a| a.iter().find(|s| s["name"].as_str() == Some(name)))
        else {
            continue;
        };
        let listed = ins["comments"].as_array();
        let mut used = Vec::new();
        for (j, item) in arr.iter().enumerate() {
            let cp = format!("{prefix}.comments[{j}]");
            let cobj = require_object(item, &cp)?;
            let Some(replies) = optional_array(cobj, "replies", &cp)? else {
                continue;
            };
            if replies.is_empty() {
                continue;
            }
            let cell = cobj.get("cell").and_then(Value::as_str).unwrap_or("");
            let want = a1::parse_cell(cell)
                .ok()
                .map(|c| a1::format_cell(c.col, c.row));
            let host_id = listed.and_then(|cs| {
                cs.iter().find_map(|c| {
                    let id = c["id"].as_str()?;
                    if used.iter().any(|u: &String| u == id) {
                        return None;
                    }
                    let got = c["ref"].as_str().map(|s| s.to_string());
                    if got == want {
                        Some(id.to_string())
                    } else {
                        None
                    }
                })
            });
            let Some(host_id) = host_id else {
                return Err(Error::with_path(
                    ErrorKind::CommentNotFound {
                        id: format!("cell:{name}!{}", want.as_deref().unwrap_or(cell)),
                    },
                    format!("{cp}.replies"),
                ));
            };
            used.push(host_id.clone());
            for (k, reply) in replies.iter().enumerate() {
                let rp = format!("{cp}.replies[{k}]");
                let robj = require_object(reply, &rp)?;
                deny_create_meta(robj, &rp)?;
                if robj.contains_key("replies") {
                    return Err(Error::with_path(
                        ErrorKind::UnknownField {
                            field: "replies".into(),
                        },
                        format!("{rp}.replies"),
                    ));
                }
                push_op(ops, paths, robj.clone(), "replyComment", &host_id, &rp)?;
            }
        }
    }
    Ok(())
}

fn append_group_ops(
    bytes: &[u8],
    workbook: &Map<String, Value>,
    ops: &mut Vec<XlsxOp>,
    paths: &mut Vec<String>,
) -> Result<(), Error> {
    let scope = super::parse_scope(Some(&json!({
        "shapes": true,
        "textBoxes": true,
        "connectors": true
    })))?
    .expect("组合 inspect 范围非空");
    let tree = inspect::inspect(bytes, Some(scope))?;
    let sheets = workbook["sheets"].as_array().expect("create 已校验 sheets");
    for (i, sheet) in sheets.iter().enumerate() {
        let Some(obj) = sheet.as_object() else {
            continue;
        };
        let Some(arr) = optional_array(obj, "groups", &format!("workbook.sheets[{i}]"))? else {
            continue;
        };
        let name = sheet_name(obj);
        let sheet_id = format!("sheet:{name}");
        let Some(ins) = tree["sheets"]
            .as_array()
            .and_then(|a| a.iter().find(|s| s["name"].as_str() == Some(name)))
        else {
            continue;
        };
        for (j, item) in arr.iter().enumerate() {
            let cp = format!("workbook.sheets[{i}].groups[{j}]");
            let g = require_object(item, &cp)?;
            deny_create_meta(g, &cp)?;
            deny_extra(g, &["textBoxes", "shapes", "connectors"], &cp)?;
            let mut ids = Vec::new();
            push_member_ids(&mut ids, g, "textBoxes", ins, &cp)?;
            push_member_ids(&mut ids, g, "shapes", ins, &cp)?;
            push_member_ids(&mut ids, g, "connectors", ins, &cp)?;
            if ids.len() < 2 {
                return Err(Error::with_path(ErrorKind::GroupNeedsTwo, cp));
            }
            let mut m = Map::new();
            m.insert("ids".into(), json!(ids));
            push_op(ops, paths, m, "groupShapes", &sheet_id, &cp)?;
        }
    }
    Ok(())
}

fn push_member_ids(
    ids: &mut Vec<String>,
    group: &Map<String, Value>,
    field: &str,
    inspect_sheet: &Value,
    prefix: &str,
) -> Result<(), Error> {
    let Some(arr) = group.get(field) else {
        return Ok(());
    };
    let items = require_array(arr, &format!("{prefix}.{field}"))?;
    let listed = inspect_sheet
        .get(field)
        .and_then(Value::as_array)
        .map(|a| a.as_slice())
        .unwrap_or(&[]);
    for (k, v) in items.iter().enumerate() {
        let ip = format!("{prefix}.{field}[{k}]");
        let idx = index1(v, field, &ip, listed.len())?;
        let id = listed[idx]["id"].as_str().ok_or_else(|| {
            Error::with_path(
                ErrorKind::CreateIndexOutOfRange {
                    field: field.into(),
                    max: listed.len() as i64,
                },
                ip.clone(),
            )
        })?;
        ids.push(id.to_string());
    }
    Ok(())
}

fn index1(v: &Value, field: &str, path: &str, max: usize) -> Result<usize, Error> {
    let Some(n) = v.as_u64() else {
        if v.as_i64().is_some_and(|x| x < 1) {
            return Err(Error::with_path(
                ErrorKind::MinValue {
                    field: field.into(),
                    min: 1,
                },
                path,
            ));
        }
        return Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Integer,
            },
            path,
        ));
    };
    if n == 0 {
        return Err(Error::with_path(
            ErrorKind::MinValue {
                field: field.into(),
                min: 1,
            },
            path,
        ));
    }
    if n as usize > max {
        return Err(Error::with_path(
            ErrorKind::CreateIndexOutOfRange {
                field: field.into(),
                max: max as i64,
            },
            path,
        ));
    }
    Ok((n as usize) - 1)
}

fn push_op(
    ops: &mut Vec<XlsxOp>,
    paths: &mut Vec<String>,
    mut item: Map<String, Value>,
    op: &str,
    id: &str,
    prefix: &str,
) -> Result<(), Error> {
    item.insert("op".into(), json!(op));
    item.insert("id".into(), json!(id));
    let prefix_s = prefix.to_string();
    let parsed = super::parse_ops(&[Value::Object(item)])
        .map_err(|e| remap_ops_error(e, std::slice::from_ref(&prefix_s)))?;
    ops.push(parsed.into_iter().next().expect("parse_ops 非空"));
    paths.push(prefix_s);
    Ok(())
}

fn remap_ops_error(err: Error, paths: &[String]) -> Error {
    let Some(p) = err.field_path() else {
        return err;
    };
    let Some(rest) = p.strip_prefix("ops[") else {
        return err;
    };
    let Some((idx_s, after)) = rest.split_once(']') else {
        return err;
    };
    let Ok(idx) = idx_s.parse::<usize>() else {
        return err;
    };
    let Some(prefix) = paths.get(idx) else {
        return err;
    };
    let mapped = if after.is_empty() {
        prefix.clone()
    } else if let Some(field) = after.strip_prefix('.') {
        format!("{prefix}.{field}")
    } else {
        format!("{prefix}{after}")
    };
    err.at(mapped)
}

fn deny_create_meta(obj: &Map<String, Value>, prefix: &str) -> Result<(), Error> {
    for k in ["op", "id"] {
        if obj.contains_key(k) {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.into() },
                format!("{prefix}.{k}"),
            ));
        }
    }
    Ok(())
}

fn deny_extra(obj: &Map<String, Value>, allowed: &[&str], prefix: &str) -> Result<(), Error> {
    for k in obj.keys() {
        if !allowed.contains(&k.as_str()) {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("{prefix}.{k}"),
            ));
        }
    }
    Ok(())
}

fn optional_array<'a>(
    obj: &'a Map<String, Value>,
    key: &str,
    prefix: &str,
) -> Result<Option<&'a Vec<Value>>, Error> {
    match obj.get(key) {
        None => Ok(None),
        Some(v) => Ok(Some(require_array(v, &format!("{prefix}.{key}"))?)),
    }
}

fn require_array<'a>(v: &'a Value, path: &str) -> Result<&'a Vec<Value>, Error> {
    v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: path.rsplit('.').next().unwrap_or("array").into(),
                expected: TypeName::Array,
            },
            path,
        )
    })
}

fn require_object<'a>(v: &'a Value, path: &str) -> Result<&'a Map<String, Value>, Error> {
    v.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: path.rsplit('.').next().unwrap_or("object").into(),
                expected: TypeName::Object,
            },
            path,
        )
    })
}

fn sheet_name(obj: &Map<String, Value>) -> &str {
    obj.get("name").and_then(Value::as_str).unwrap_or("")
}

fn is_chartsheet(obj: &Map<String, Value>) -> bool {
    obj.get("kind").and_then(Value::as_str) == Some("chartsheet")
}

fn row_span_string(obj: &Map<String, Value>, prefix: &str) -> Result<String, Error> {
    let row = json_row(obj.get("row"), "row", prefix)?;
    let to = match obj.get("toRow") {
        None => row,
        Some(_) => json_row(obj.get("toRow"), "toRow", prefix)?,
    };
    if to < row {
        return Err(Error::with_path(
            ErrorKind::RowRangeReversed,
            format!("{prefix}.toRow"),
        ));
    }
    if to == row {
        Ok(row.to_string())
    } else {
        Ok(format!("{row}:{to}"))
    }
}

fn json_row(v: Option<&Value>, field: &str, prefix: &str) -> Result<u32, Error> {
    match v {
        Some(Value::Number(n)) => {
            let Some(u) = n.as_u64() else {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: field.into(),
                        expected: TypeName::Integer,
                    },
                    format!("{prefix}.{field}"),
                ));
            };
            if u < 1 {
                return Err(Error::with_path(
                    ErrorKind::MinValue {
                        field: field.into(),
                        min: 1,
                    },
                    format!("{prefix}.{field}"),
                ));
            }
            if u > 1_048_576 {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: field.into(),
                        expected: TypeName::Integer,
                    },
                    format!("{prefix}.{field}"),
                ));
            }
            Ok(u as u32)
        }
        None => Err(Error::with_path(
            ErrorKind::MissingField {
                field: field.into(),
            },
            format!("{prefix}.{field}"),
        )),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Integer,
            },
            format!("{prefix}.{field}"),
        )),
    }
}
