//! 工作表视图与工作表级筛选：inspect 摘要，apply 写 sheetViews / autoFilter。

use serde_json::{json, Map, Value};

use crate::error::{Error, ErrorKind};
use crate::xml::{Element, Node};

use super::a1::{self, CellRef};
use super::layout;
use super::request::{SetAutoFilterOp, SetSheetViewOp};

const DEF_ZOOM: u32 = 100;
const DEF_CELL: &str = "A1";

pub(crate) fn inspect_view(sheet: &Element) -> Value {
    let view = first_sheet_view(sheet);
    let (frozen_rows, frozen_cols) = view.map(frozen_of).unwrap_or((0, 0));
    let (split_rows, split_cols) = view.map(split_of).unwrap_or((0, 0));
    let mut obj = Map::new();
    obj.insert("zoom".into(), json!(zoom_of(view)));
    obj.insert("frozenRows".into(), json!(frozen_rows));
    obj.insert("frozenCols".into(), json!(frozen_cols));
    obj.insert("splitRows".into(), json!(split_rows));
    obj.insert("splitCols".into(), json!(split_cols));
    obj.insert(
        "activeCell".into(),
        json!(view
            .map(active_cell_of)
            .unwrap_or_else(|| DEF_CELL.to_string())),
    );
    Value::Object(obj)
}

pub(crate) fn inspect_filter(sheet: &Element) -> Value {
    match sheet
        .child_by_local("autoFilter")
        .and_then(|e| e.attr("ref"))
    {
        Some(raw) if !raw.trim().is_empty() => match a1::parse_range(raw) {
            Ok((s, e)) => json!(a1::format_range(s, e)),
            Err(_) => json!(raw),
        },
        _ => Value::Null,
    }
}

pub(crate) fn apply_view(
    sheet: &mut Element,
    kind: &str,
    op: &SetSheetViewOp,
    i: usize,
) -> Result<(), Error> {
    let chartsheet = kind == "chartsheet";
    if chartsheet {
        reject_chartsheet_view_fields(op, i)?;
    }
    {
        let view = ensure_default_view(sheet);
        if let Some(zoom) = op.zoom {
            view.set_attr("zoomScale", zoom.to_string());
            if chartsheet {
                view.set_attr("zoomToFit", "0");
            }
        }
    }
    if op.has_freeze() {
        let (cur_rows, cur_cols) = frozen_of(ensure_default_view(sheet));
        let rows = op.frozen_rows.unwrap_or(cur_rows);
        let cols = op.frozen_cols.unwrap_or(cur_cols);
        set_freeze(ensure_default_view(sheet), rows, cols);
    }
    if op.has_split() {
        let (cur_rows, cur_cols) = split_of(ensure_default_view(sheet));
        let rows = op.split_rows.unwrap_or(cur_rows);
        let cols = op.split_cols.unwrap_or(cur_cols);
        set_split(ensure_default_view(sheet), rows, cols);
    }
    if let Some(cell) = op.active_cell.as_deref() {
        set_active_cell(ensure_default_view(sheet), cell);
    }
    Ok(())
}

pub(crate) fn apply_filter(
    sheet: &mut Element,
    kind: &str,
    op: &SetAutoFilterOp,
    table_overlap: bool,
    i: usize,
) -> Result<(), Error> {
    if kind != "worksheet" {
        return Err(Error::with_path(
            ErrorKind::InvalidAutoFilter {
                field: "ref".into(),
            },
            format!("ops[{i}].ref"),
        ));
    }
    match op.refer.as_deref() {
        None => {
            sheet.children.retain(|n| match n {
                Node::Element(e) => e.local_name() != "autoFilter",
                _ => true,
            });
        }
        Some(raw) => {
            let (start, end) = parse_filter_range(raw, i)?;
            if table_overlap {
                return Err(Error::with_path(
                    ErrorKind::InvalidAutoFilter {
                        field: "ref".into(),
                    },
                    format!("ops[{i}].ref"),
                ));
            }
            let formatted = a1::format_range(start, end);
            if let Some(af) = sheet.child_by_local_mut("autoFilter") {
                af.set_attr("ref", formatted);
            } else {
                layout::insert_child_ordered(
                    sheet,
                    Element::new("autoFilter").with_attr("ref", formatted),
                    "autoFilter",
                );
            }
        }
    }
    Ok(())
}

fn reject_chartsheet_view_fields(op: &SetSheetViewOp, i: usize) -> Result<(), Error> {
    for (name, hit) in [
        ("frozenRows", op.frozen_rows.is_some()),
        ("frozenCols", op.frozen_cols.is_some()),
        ("splitRows", op.split_rows.is_some()),
        ("splitCols", op.split_cols.is_some()),
        ("activeCell", op.active_cell.is_some()),
    ] {
        if hit {
            return Err(Error::with_path(
                ErrorKind::InvalidSheetView { field: name.into() },
                format!("ops[{i}].{name}"),
            ));
        }
    }
    Ok(())
}

pub(crate) fn first_sheet_view(sheet: &Element) -> Option<&Element> {
    sheet
        .child_by_local("sheetViews")?
        .element_children()
        .find(|e| e.local_name() == "sheetView")
}

pub(crate) fn ensure_default_view(sheet: &mut Element) -> &mut Element {
    if sheet.child_by_local("sheetViews").is_none() {
        layout::insert_child_ordered(sheet, Element::new("sheetViews"), "sheetViews");
    }
    let views = sheet.child_by_local_mut("sheetViews").unwrap();
    if !views
        .element_children()
        .any(|e| e.local_name() == "sheetView")
    {
        views.push_element(Element::new("sheetView").with_attr("workbookViewId", "0"));
    }
    views
        .element_children_mut()
        .find(|e| e.local_name() == "sheetView")
        .expect("刚写入 sheetView")
}

fn zoom_of(view: Option<&Element>) -> u32 {
    view.and_then(|v| v.attr("zoomScale"))
        .and_then(|s| s.parse::<u32>().ok())
        .filter(|n| (10..=400).contains(n))
        .unwrap_or(DEF_ZOOM)
}

fn frozen_of(view: &Element) -> (u32, u32) {
    let Some(pane) = view.child_by_local("pane") else {
        return (0, 0);
    };
    let state = pane.attr("state").unwrap_or("split");
    if state != "frozen" && state != "frozenSplit" {
        return (0, 0);
    }
    (split_count(pane, "ySplit"), split_count(pane, "xSplit"))
}

fn split_of(view: &Element) -> (u32, u32) {
    let Some(pane) = view.child_by_local("pane") else {
        return (0, 0);
    };
    let state = pane.attr("state").unwrap_or("split");
    if state != "split" {
        return (0, 0);
    }
    let x = split_count(pane, "xSplit");
    let y = split_count(pane, "ySplit");
    if x == 0 && y == 0 {
        return (0, 0);
    }
    match pane
        .attr("topLeftCell")
        .and_then(|s| a1::parse_cell(s).ok())
    {
        Some(c) => (c.row.saturating_sub(1), c.col.saturating_sub(1)),
        None => (0, 0),
    }
}

pub(crate) fn has_split_or_freeze(sheet: &Element) -> bool {
    let Some(view) = first_sheet_view(sheet) else {
        return false;
    };
    let (fr, fc) = frozen_of(view);
    let (sr, sc) = split_of(view);
    fr > 0 || fc > 0 || sr > 0 || sc > 0
}

pub(crate) fn snapshot_pane(sheet: &Element) -> (Option<Element>, Vec<Element>) {
    let Some(view) = first_sheet_view(sheet) else {
        return (None, Vec::new());
    };
    let pane = view.child_by_local("pane").cloned();
    let sels = view
        .children_by_local("selection")
        .into_iter()
        .cloned()
        .collect();
    (pane, sels)
}

pub(crate) fn strip_pane_and_selection(view: &mut Element) -> Vec<Node> {
    let mut rest = Vec::new();
    for n in std::mem::take(&mut view.children) {
        match &n {
            Node::Element(e) if e.local_name() == "pane" || e.local_name() == "selection" => {}
            _ => rest.push(n),
        }
    }
    rest
}

fn split_count(pane: &Element, attr: &str) -> u32 {
    pane.attr(attr)
        .and_then(|s| s.parse::<f64>().ok())
        .filter(|n| n.is_finite() && *n > 0.0)
        .map(|n| n.round() as u32)
        .unwrap_or(0)
}

fn active_cell_of(view: &Element) -> String {
    let pane_name = view
        .child_by_local("pane")
        .and_then(|p| p.attr("activePane"));
    let selections: Vec<&Element> = view.children_by_local("selection");
    let pick = match pane_name {
        Some(name) => selections
            .iter()
            .copied()
            .find(|s| s.attr("pane") == Some(name))
            .or_else(|| selections.first().copied()),
        None => selections
            .iter()
            .copied()
            .find(|s| s.attr("pane").is_none())
            .or_else(|| selections.first().copied()),
    };
    if let Some(sel) = pick {
        if let Some(ac) = sel.attr("activeCell") {
            if let Ok(c) = a1::parse_cell(ac) {
                return a1::format_cell(c.col, c.row);
            }
        }
        if let Some(sq) = sel.attr("sqref") {
            let first = sq.split_whitespace().next().unwrap_or("");
            if let Ok((s, _)) = a1::parse_range(first) {
                return a1::format_cell(s.col, s.row);
            }
        }
    }
    DEF_CELL.to_string()
}

fn set_freeze(view: &mut Element, rows: u32, cols: u32) {
    let current_ac = active_cell_of(view);
    let rest = strip_pane_and_selection(view);
    if rows == 0 && cols == 0 {
        view.push_element(
            Element::new("selection")
                .with_attr("activeCell", current_ac.clone())
                .with_attr("sqref", current_ac),
        );
        view.children.extend(rest);
        return;
    }
    let top_left = a1::format_cell(cols + 1, rows + 1);
    let active_pane = active_pane_name(rows, cols);
    let mut pane = Element::new("pane");
    if cols > 0 {
        pane.set_attr("xSplit", cols.to_string());
    }
    if rows > 0 {
        pane.set_attr("ySplit", rows.to_string());
    }
    pane.set_attr("topLeftCell", &top_left);
    pane.set_attr("activePane", active_pane);
    pane.set_attr("state", "frozen");
    view.push_element(pane);
    view.push_element(
        Element::new("selection")
            .with_attr("pane", active_pane)
            .with_attr("activeCell", top_left.clone())
            .with_attr("sqref", top_left),
    );
    view.children.extend(rest);
}

fn set_split(view: &mut Element, rows: u32, cols: u32) {
    let current_ac = active_cell_of(view);
    let rest = strip_pane_and_selection(view);
    if rows == 0 && cols == 0 {
        view.push_element(
            Element::new("selection")
                .with_attr("activeCell", current_ac.clone())
                .with_attr("sqref", current_ac),
        );
        view.children.extend(rest);
        return;
    }
    let top_left = a1::format_cell(cols + 1, rows + 1);
    let active_pane = active_pane_name(rows, cols);
    let mut pane = Element::new("pane");
    if cols > 0 {
        pane.set_attr("xSplit", layout::default_col_split_twentieths(cols));
    }
    if rows > 0 {
        pane.set_attr("ySplit", layout::default_row_split_twentieths(rows));
    }
    pane.set_attr("topLeftCell", &top_left);
    pane.set_attr("activePane", active_pane);
    view.push_element(pane);
    view.push_element(
        Element::new("selection")
            .with_attr("pane", active_pane)
            .with_attr("activeCell", top_left.clone())
            .with_attr("sqref", top_left),
    );
    view.children.extend(rest);
}

fn active_pane_name(rows: u32, cols: u32) -> &'static str {
    if rows > 0 && cols > 0 {
        "bottomRight"
    } else if rows > 0 {
        "bottomLeft"
    } else {
        "topRight"
    }
}

fn set_active_cell(view: &mut Element, cell: &str) {
    let pane_name = view
        .child_by_local("pane")
        .and_then(|p| p.attr("activePane"))
        .map(str::to_string);
    let mut found = false;
    for n in view.children.iter_mut() {
        let Node::Element(e) = n else { continue };
        if e.local_name() != "selection" {
            continue;
        }
        let matches = match (pane_name.as_deref(), e.attr("pane")) {
            (Some(want), Some(have)) => want == have,
            (Some(_), None) => false,
            (None, None) => true,
            (None, Some(_)) => false,
        };
        if matches {
            e.set_attr("activeCell", cell);
            e.set_attr("sqref", cell);
            found = true;
            break;
        }
    }
    if found {
        return;
    }
    let mut sel = Element::new("selection")
        .with_attr("activeCell", cell)
        .with_attr("sqref", cell);
    if let Some(p) = pane_name.as_deref() {
        sel.set_attr("pane", p);
    }
    let at = view
        .children
        .iter()
        .position(|n| matches!(n, Node::Element(e) if e.local_name() == "pane"))
        .map(|i| i + 1)
        .unwrap_or(0);
    view.children.insert(at, Node::Element(sel));
}

fn parse_filter_range(raw: &str, i: usize) -> Result<(CellRef, CellRef), Error> {
    a1::parse_range(raw).map_err(|e| e.at(format!("ops[{i}].ref")))
}
