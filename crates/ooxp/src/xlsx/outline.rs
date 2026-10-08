//! 工作表行 / 列大纲：inspect 摘要，apply 分组、取消与折叠。

use serde_json::{json, Map, Value};

use super::a1;
use super::apply;
use super::layout;
use super::request::{GroupColsOp, GroupRowsOp, SetOutlineOp, UngroupColsOp, UngroupRowsOp};
use crate::error::{Error, ErrorKind};
use crate::xml::{Element, Node};

const MAX_OUTLINE: u32 = 7;
const MAX_ROW: u32 = 1_048_576;
const MAX_COL: u32 = 16_384;

const SHEET_PR_CHILD_ORDER: &[&str] = &["tabColor", "outlinePr", "pageSetUpPr"];

pub(crate) fn inspect(sheet: &Element) -> Value {
    let outline_pr = sheet
        .child_by_local("sheetPr")
        .and_then(|p| p.child_by_local("outlinePr"));
    let mut obj = Map::new();
    obj.insert(
        "summaryBelow".into(),
        json!(attr_default_true(outline_pr, "summaryBelow")),
    );
    obj.insert(
        "summaryRight".into(),
        json!(attr_default_true(outline_pr, "summaryRight")),
    );
    obj.insert("rows".into(), inspect_rows(sheet));
    obj.insert("cols".into(), inspect_cols(sheet));
    Value::Object(obj)
}

fn inspect_rows(sheet: &Element) -> Value {
    let mut items = Vec::new();
    let Some(sheet_data) = sheet.child_by_local("sheetData") else {
        return json!([]);
    };
    for row in sheet_data.children_by_local("row") {
        let Some(r) = row.attr("r").and_then(|s| s.parse::<u32>().ok()) else {
            continue;
        };
        let level = parse_level(row.attr("outlineLevel"));
        let collapsed = attr_true(row, "collapsed");
        if level == 0 && !collapsed {
            continue;
        }
        items.push((
            r,
            json!({
                "r": r,
                "level": level,
                "hidden": attr_true(row, "hidden"),
                "collapsed": collapsed,
            }),
        ));
    }
    items.sort_by_key(|(r, _)| *r);
    Value::Array(items.into_iter().map(|(_, v)| v).collect())
}

fn inspect_cols(sheet: &Element) -> Value {
    let mut items = Vec::new();
    for cols in sheet.children_by_local("cols") {
        for col in cols.children_by_local("col") {
            let Some(min) = col.attr("min").and_then(s_to_u32) else {
                continue;
            };
            let Some(max) = col.attr("max").and_then(s_to_u32) else {
                continue;
            };
            if min == 0 || max == 0 || min > max {
                continue;
            }
            let level = parse_level(col.attr("outlineLevel"));
            let collapsed = attr_true(col, "collapsed");
            if level == 0 && !collapsed {
                continue;
            }
            items.push(json!({
                "min": a1::col_to_letters(min),
                "max": a1::col_to_letters(max),
                "level": level,
                "hidden": attr_true(col, "hidden"),
                "collapsed": collapsed,
            }));
        }
    }
    Value::Array(items)
}

fn s_to_u32(s: &str) -> Option<u32> {
    s.trim().parse().ok()
}

pub(crate) fn apply_group_rows(
    sheet: &mut Element,
    op: &GroupRowsOp,
    op_index: usize,
) -> Result<(), Error> {
    bump_rows(sheet, op.row, op.to_row, 1, op_index)?;
    sync_sheet_format_outline(sheet);
    Ok(())
}

pub(crate) fn apply_ungroup_rows(
    sheet: &mut Element,
    op: &UngroupRowsOp,
    _op_index: usize,
) -> Result<(), Error> {
    bump_rows(sheet, op.row, op.to_row, -1, 0)?;
    sync_sheet_format_outline(sheet);
    Ok(())
}

pub(crate) fn apply_group_cols(
    sheet: &mut Element,
    op: &GroupColsOp,
    op_index: usize,
) -> Result<(), Error> {
    layout::apply_col_outline_delta(sheet, op.min_col, op.max_col, 1, op_index)?;
    sync_sheet_format_outline(sheet);
    Ok(())
}

pub(crate) fn apply_ungroup_cols(
    sheet: &mut Element,
    op: &UngroupColsOp,
    op_index: usize,
) -> Result<(), Error> {
    layout::apply_col_outline_delta(sheet, op.min_col, op.max_col, -1, op_index)?;
    sync_sheet_format_outline(sheet);
    Ok(())
}

pub(crate) fn apply_set_outline(
    sheet: &mut Element,
    op: &SetOutlineOp,
    op_index: usize,
) -> Result<(), Error> {
    if let Some(v) = op.summary_below {
        set_summary_flag(sheet, "summaryBelow", v);
    }
    if let Some(v) = op.summary_right {
        set_summary_flag(sheet, "summaryRight", v);
    }
    let below = summary_below(sheet);
    let right = summary_right(sheet);
    if let Some((from, to)) = op.collapse_rows {
        apply_collapse_rows(sheet, from, to, below, op.collapsed.unwrap(), op_index)?;
    }
    if let Some((from, to)) = op.collapse_cols {
        apply_collapse_cols(sheet, from, to, right, op.collapsed.unwrap(), op_index)?;
    }
    Ok(())
}

fn bump_rows(
    sheet: &mut Element,
    from: u32,
    to: u32,
    delta: i32,
    op_index: usize,
) -> Result<(), Error> {
    let sheet_data = apply::ensure_sheet_data(sheet);
    if delta > 0 {
        for r in from..=to {
            let lvl = row_outline_level(sheet_data, r);
            if lvl.saturating_add(delta as u32) > MAX_OUTLINE {
                return Err(Error::with_path(
                    ErrorKind::OutlineLevelMax,
                    format!("ops[{op_index}].rows"),
                ));
            }
        }
        for r in from..=to {
            let next = row_outline_level(sheet_data, r).saturating_add(delta as u32);
            set_row_outline(sheet_data, r, next);
        }
    } else {
        for r in from..=to {
            let next = row_outline_level(sheet_data, r).saturating_sub((-delta) as u32);
            set_row_outline(sheet_data, r, next);
        }
    }
    Ok(())
}

fn apply_collapse_rows(
    sheet: &mut Element,
    from: u32,
    to: u32,
    summary_below: bool,
    collapsed: bool,
    op_index: usize,
) -> Result<(), Error> {
    let summary = if summary_below {
        to.checked_add(1).filter(|n| *n <= MAX_ROW)
    } else {
        from.checked_sub(1).filter(|n| *n >= 1)
    };
    let Some(summary) = summary else {
        return Err(Error::with_path(
            ErrorKind::InvalidOutline {
                field: "collapseRows".into(),
            },
            format!("ops[{op_index}].collapseRows"),
        ));
    };
    let sheet_data = apply::ensure_sheet_data(sheet);
    for r in from..=to {
        set_row_flag(sheet_data, r, "hidden", collapsed);
    }
    set_row_flag(sheet_data, summary, "collapsed", collapsed);
    Ok(())
}

fn apply_collapse_cols(
    sheet: &mut Element,
    from: u32,
    to: u32,
    summary_right: bool,
    collapsed: bool,
    op_index: usize,
) -> Result<(), Error> {
    let summary = if summary_right {
        to.checked_add(1).filter(|n| *n <= MAX_COL)
    } else {
        from.checked_sub(1).filter(|n| *n >= 1)
    };
    let Some(summary) = summary else {
        return Err(Error::with_path(
            ErrorKind::InvalidOutline {
                field: "collapseCols".into(),
            },
            format!("ops[{op_index}].collapseCols"),
        ));
    };
    layout::apply_col_flag(sheet, from, to, "hidden", collapsed);
    layout::apply_col_flag(sheet, summary, summary, "collapsed", collapsed);
    Ok(())
}

fn set_summary_flag(sheet: &mut Element, name: &str, value: bool) {
    if value {
        if let Some(pr) = sheet.child_by_local_mut("sheetPr") {
            if let Some(op) = pr.child_by_local_mut("outlinePr") {
                op.remove_attr(name);
            }
        }
        drop_empty_outline_pr(sheet);
        return;
    }
    let op = ensure_outline_pr(sheet);
    op.set_attr(name, "0");
}

fn summary_below(sheet: &Element) -> bool {
    attr_default_true(
        sheet
            .child_by_local("sheetPr")
            .and_then(|p| p.child_by_local("outlinePr")),
        "summaryBelow",
    )
}

fn summary_right(sheet: &Element) -> bool {
    attr_default_true(
        sheet
            .child_by_local("sheetPr")
            .and_then(|p| p.child_by_local("outlinePr")),
        "summaryRight",
    )
}

fn ensure_outline_pr(sheet: &mut Element) -> &mut Element {
    if sheet.child_by_local("sheetPr").is_none() {
        layout::insert_child_ordered(sheet, Element::new("sheetPr"), "sheetPr");
    }
    let pr = sheet.child_by_local_mut("sheetPr").unwrap();
    if pr.child_by_local("outlinePr").is_none() {
        insert_sheet_pr_child(pr, Element::new("outlinePr"), "outlinePr");
    }
    pr.child_by_local_mut("outlinePr").unwrap()
}

fn insert_sheet_pr_child(pr: &mut Element, el: Element, local: &str) {
    let want = SHEET_PR_CHILD_ORDER
        .iter()
        .position(|n| *n == local)
        .unwrap_or(SHEET_PR_CHILD_ORDER.len());
    let mut at = pr.children.len();
    for (i, n) in pr.children.iter().enumerate() {
        let Node::Element(e) = n else { continue };
        let pos = SHEET_PR_CHILD_ORDER
            .iter()
            .position(|n| *n == e.local_name())
            .unwrap_or(SHEET_PR_CHILD_ORDER.len());
        if pos > want {
            at = i;
            break;
        }
    }
    pr.children.insert(at, Node::Element(el));
}

fn drop_empty_outline_pr(sheet: &mut Element) {
    let Some(pr) = sheet.child_by_local_mut("sheetPr") else {
        return;
    };
    let Some(pos) = pr
        .children
        .iter()
        .position(|n| matches!(n, Node::Element(e) if e.local_name() == "outlinePr"))
    else {
        return;
    };
    let empty = match &pr.children[pos] {
        Node::Element(e) => e.attrs.is_empty() && e.element_children().next().is_none(),
        _ => false,
    };
    if empty {
        pr.children.remove(pos);
    }
}

fn row_outline_level(sheet_data: &Element, r: u32) -> u32 {
    let Some(idx) = apply::find_row_index(sheet_data, r) else {
        return 0;
    };
    let Node::Element(row) = &sheet_data.children[idx] else {
        return 0;
    };
    parse_level(row.attr("outlineLevel"))
}

fn set_row_outline(sheet_data: &mut Element, r: u32, level: u32) {
    if level_is_zero_and_missing(sheet_data, r, level) {
        return;
    }
    if level == 0 {
        let Some(idx) = apply::find_row_index(sheet_data, r) else {
            return;
        };
        let Node::Element(row) = &mut sheet_data.children[idx] else {
            return;
        };
        row.remove_attr("outlineLevel");
        drop_row_if_shell(sheet_data, r);
        return;
    }
    let row = apply::ensure_row(sheet_data, r);
    row.set_attr("outlineLevel", level.to_string());
}

fn level_is_zero_and_missing(sheet_data: &Element, r: u32, level: u32) -> bool {
    level == 0 && apply::find_row_index(sheet_data, r).is_none()
}

fn set_row_flag(sheet_data: &mut Element, r: u32, name: &str, on: bool) {
    if on {
        let row = apply::ensure_row(sheet_data, r);
        row.set_attr(name, "1");
        return;
    }
    let Some(idx) = apply::find_row_index(sheet_data, r) else {
        return;
    };
    let Node::Element(row) = &mut sheet_data.children[idx] else {
        return;
    };
    row.remove_attr(name);
    drop_row_if_shell(sheet_data, r);
}

fn drop_row_if_shell(sheet_data: &mut Element, r: u32) {
    let Some(idx) = apply::find_row_index(sheet_data, r) else {
        return;
    };
    let Node::Element(row) = &sheet_data.children[idx] else {
        return;
    };
    let only_r = row.attrs.iter().all(|(k, _)| k == "r")
        && row.element_children().all(|e| e.local_name() != "c");
    if only_r {
        sheet_data.children.remove(idx);
    }
}

fn sync_sheet_format_outline(sheet: &mut Element) {
    let max_row = max_row_level(sheet);
    let max_col = max_col_level(sheet);
    if max_row == 0 && max_col == 0 {
        if let Some(pr) = sheet.child_by_local_mut("sheetFormatPr") {
            pr.remove_attr("outlineLevelRow");
            pr.remove_attr("outlineLevelCol");
            let empty = pr.attrs.is_empty() && pr.element_children().next().is_none();
            if empty {
                sheet.children.retain(|n| match n {
                    Node::Element(e) => e.local_name() != "sheetFormatPr",
                    _ => true,
                });
            }
        }
        return;
    }
    let pr = layout::ensure_sheet_format_pr(sheet);
    if max_row == 0 {
        pr.remove_attr("outlineLevelRow");
    } else {
        pr.set_attr("outlineLevelRow", max_row.to_string());
    }
    if max_col == 0 {
        pr.remove_attr("outlineLevelCol");
    } else {
        pr.set_attr("outlineLevelCol", max_col.to_string());
    }
}

fn max_row_level(sheet: &Element) -> u32 {
    let Some(sheet_data) = sheet.child_by_local("sheetData") else {
        return 0;
    };
    sheet_data
        .children_by_local("row")
        .into_iter()
        .map(|row| parse_level(row.attr("outlineLevel")))
        .max()
        .unwrap_or(0)
}

fn max_col_level(sheet: &Element) -> u32 {
    let mut max = 0u32;
    for cols in sheet.children_by_local("cols") {
        for col in cols.children_by_local("col") {
            max = max.max(parse_level(col.attr("outlineLevel")));
        }
    }
    max
}

fn parse_level(raw: Option<&str>) -> u32 {
    raw.and_then(|s| s.trim().parse().ok()).unwrap_or(0)
}

fn attr_true(el: &Element, name: &str) -> bool {
    matches!(
        el.attr(name).map(str::trim),
        Some("1" | "true" | "TRUE" | "on")
    )
}

fn attr_default_true(el: Option<&Element>, name: &str) -> bool {
    match el.and_then(|e| e.attr(name)).map(str::trim) {
        None => true,
        Some("0" | "false" | "FALSE" | "off") => false,
        _ => true,
    }
}
