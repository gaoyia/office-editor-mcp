//! 工作表打印设置：inspect 摘要，apply 写 pageMargins / pageSetup / printOptions。

use serde_json::{json, Map, Value};

use crate::chart;
use crate::error::{Error, ErrorKind};
use crate::xml::{Document, Element, Node};

use super::a1;
use super::layout;
use super::names;
use super::request::SetPrintSetupOp;

const DEF_LEFT: f64 = 0.7;
const DEF_RIGHT: f64 = 0.7;
const DEF_TOP: f64 = 0.75;
const DEF_BOTTOM: f64 = 0.75;
const DEF_HEADER: f64 = 0.3;
const DEF_FOOTER: f64 = 0.3;
const DEF_PAPER: u32 = 1;
const DEF_SCALE: u32 = 100;
const DEF_FIT: u32 = 1;
const ROW_BRK_MAX: &str = "16383";
const COL_BRK_MAX: &str = "1048575";

pub(crate) fn inspect(sheet: &Element, wb: &Element, sheet_id: u32, sheet_name: &str) -> Value {
    let margins = sheet.child_by_local("pageMargins");
    let setup = sheet.child_by_local("pageSetup");
    let opts = sheet.child_by_local("printOptions");
    let mut obj = Map::new();
    obj.insert("left".into(), json!(inch(margins, "left", DEF_LEFT)));
    obj.insert("right".into(), json!(inch(margins, "right", DEF_RIGHT)));
    obj.insert("top".into(), json!(inch(margins, "top", DEF_TOP)));
    obj.insert("bottom".into(), json!(inch(margins, "bottom", DEF_BOTTOM)));
    obj.insert("header".into(), json!(inch(margins, "header", DEF_HEADER)));
    obj.insert("footer".into(), json!(inch(margins, "footer", DEF_FOOTER)));
    obj.insert(
        "paperSize".into(),
        json!(uint(setup, "paperSize", DEF_PAPER)),
    );
    obj.insert("orientation".into(), json!(orientation(setup)));
    obj.insert("scale".into(), json!(uint(setup, "scale", DEF_SCALE)));
    obj.insert("fitToPage".into(), json!(fit_to_page(sheet)));
    obj.insert(
        "fitToWidth".into(),
        json!(uint(setup, "fitToWidth", DEF_FIT)),
    );
    obj.insert(
        "fitToHeight".into(),
        json!(uint(setup, "fitToHeight", DEF_FIT)),
    );
    obj.insert("gridLines".into(), json!(grid_lines(opts)));
    obj.insert(
        "headings".into(),
        json!(opts.is_some_and(|e| attr_on(e, "headings"))),
    );
    obj.insert(
        "hCenter".into(),
        json!(opts.is_some_and(|e| attr_on(e, "horizontalCentered"))),
    );
    obj.insert(
        "vCenter".into(),
        json!(opts.is_some_and(|e| attr_on(e, "verticalCentered"))),
    );
    obj.insert(
        "printArea".into(),
        match names::print_area_formula(wb, sheet_id) {
            Some(raw) => json!(normalize_print_area(&raw, sheet_name)),
            None => Value::Null,
        },
    );
    obj.insert("rowBreaks".into(), json!(manual_breaks(sheet, "rowBreaks")));
    obj.insert("colBreaks".into(), json!(manual_breaks(sheet, "colBreaks")));
    Value::Object(obj)
}

pub(crate) fn apply(
    sheet: &mut Element,
    wb_doc: &mut Document,
    sheet_id: u32,
    sheet_name: &str,
    kind: &str,
    op: &SetPrintSetupOp,
    i: usize,
) -> Result<bool, Error> {
    let chartsheet = kind == "chartsheet";
    if chartsheet {
        reject_chartsheet_fields(op, i)?;
    }
    if op.left.is_some()
        || op.right.is_some()
        || op.top.is_some()
        || op.bottom.is_some()
        || op.header.is_some()
        || op.footer.is_some()
    {
        patch_margins(sheet, op);
    }
    if op.paper_size.is_some()
        || op.orientation.is_some()
        || op.scale.is_some()
        || op.fit_to_width.is_some()
        || op.fit_to_height.is_some()
    {
        patch_page_setup(sheet, op);
    }
    if !chartsheet {
        patch_fit_flag(sheet, op);
        if op.grid_lines.is_some()
            || op.headings.is_some()
            || op.h_center.is_some()
            || op.v_center.is_some()
        {
            patch_print_options(sheet, op);
        }
        if let Some(rows) = op.row_breaks.as_deref() {
            set_breaks(sheet, "rowBreaks", rows, true);
        }
        if let Some(cols) = op.col_breaks.as_deref() {
            set_breaks(sheet, "colBreaks", cols, false);
        }
    }
    let mut wb_dirty = false;
    if let Some(area) = &op.print_area {
        match area {
            Some(raw) => {
                let formula = parse_print_area_formula(raw, sheet_name, i)?;
                names::set_print_area(wb_doc, sheet_id, &formula);
                wb_dirty = true;
            }
            None => {
                names::clear_print_area(wb_doc, sheet_id);
                wb_dirty = true;
            }
        }
    }
    Ok(wb_dirty)
}

fn reject_chartsheet_fields(op: &SetPrintSetupOp, i: usize) -> Result<(), Error> {
    let bad = [
        (op.scale.is_some(), "scale"),
        (op.fit_to_page.is_some(), "fitToPage"),
        (op.fit_to_width.is_some(), "fitToWidth"),
        (op.fit_to_height.is_some(), "fitToHeight"),
        (op.grid_lines.is_some(), "gridLines"),
        (op.headings.is_some(), "headings"),
        (op.h_center.is_some(), "hCenter"),
        (op.v_center.is_some(), "vCenter"),
        (op.row_breaks.is_some(), "rowBreaks"),
        (op.col_breaks.is_some(), "colBreaks"),
    ];
    for (hit, field) in bad {
        if hit {
            return Err(Error::with_path(
                ErrorKind::InvalidPrintSetup {
                    field: field.into(),
                },
                format!("ops[{i}].{field}"),
            ));
        }
    }
    Ok(())
}

fn patch_margins(sheet: &mut Element, op: &SetPrintSetupOp) {
    let cur = sheet.child_by_local("pageMargins");
    let left = op.left.unwrap_or_else(|| inch(cur, "left", DEF_LEFT));
    let right = op.right.unwrap_or_else(|| inch(cur, "right", DEF_RIGHT));
    let top = op.top.unwrap_or_else(|| inch(cur, "top", DEF_TOP));
    let bottom = op.bottom.unwrap_or_else(|| inch(cur, "bottom", DEF_BOTTOM));
    let header = op.header.unwrap_or_else(|| inch(cur, "header", DEF_HEADER));
    let footer = op.footer.unwrap_or_else(|| inch(cur, "footer", DEF_FOOTER));
    let el = ensure_child(sheet, "pageMargins");
    el.set_attr("left", fmt_inch(left));
    el.set_attr("right", fmt_inch(right));
    el.set_attr("top", fmt_inch(top));
    el.set_attr("bottom", fmt_inch(bottom));
    el.set_attr("header", fmt_inch(header));
    el.set_attr("footer", fmt_inch(footer));
}

fn patch_page_setup(sheet: &mut Element, op: &SetPrintSetupOp) {
    let el = ensure_child(sheet, "pageSetup");
    if let Some(n) = op.paper_size {
        el.set_attr("paperSize", n.to_string());
    }
    if let Some(o) = op.orientation.as_deref() {
        el.set_attr("orientation", o);
    }
    if let Some(n) = op.scale {
        el.set_attr("scale", n.to_string());
    }
    if let Some(n) = op.fit_to_width {
        el.set_attr("fitToWidth", n.to_string());
    }
    if let Some(n) = op.fit_to_height {
        el.set_attr("fitToHeight", n.to_string());
    }
}

fn patch_fit_flag(sheet: &mut Element, op: &SetPrintSetupOp) {
    let mut flag = op.fit_to_page;
    if op.fit_to_width.is_some() || op.fit_to_height.is_some() {
        flag = Some(true);
    } else if op.scale.is_some() && op.fit_to_page.is_none() {
        flag = Some(false);
    }
    let Some(on) = flag else {
        return;
    };
    let pr = ensure_page_setup_pr(sheet);
    pr.set_attr("fitToPage", if on { "1" } else { "0" });
}

fn patch_print_options(sheet: &mut Element, op: &SetPrintSetupOp) {
    let el = ensure_child(sheet, "printOptions");
    if let Some(v) = op.grid_lines {
        el.set_attr("gridLines", bool_attr(v));
        if v {
            el.set_attr("gridLinesSet", "1");
        }
    }
    if let Some(v) = op.headings {
        el.set_attr("headings", bool_attr(v));
    }
    if let Some(v) = op.h_center {
        el.set_attr("horizontalCentered", bool_attr(v));
    }
    if let Some(v) = op.v_center {
        el.set_attr("verticalCentered", bool_attr(v));
    }
}

fn set_breaks(sheet: &mut Element, local: &str, values: &[u32], is_row: bool) {
    let mut ids: Vec<u32> = values.to_vec();
    ids.sort_unstable();
    ids.dedup();
    let max = if is_row { ROW_BRK_MAX } else { COL_BRK_MAX };
    let auto: Vec<Element> = sheet
        .child_by_local(local)
        .map(|p| {
            p.children_by_local("brk")
                .into_iter()
                .filter(|b| !attr_on(b, "man"))
                .cloned()
                .collect()
        })
        .unwrap_or_default();
    if ids.is_empty() && auto.is_empty() {
        sheet.children.retain(|n| match n {
            Node::Element(e) => e.local_name() != local,
            _ => true,
        });
        return;
    }
    let parent = ensure_child(sheet, local);
    parent.children.clear();
    for el in auto {
        parent.push_element(el);
    }
    for n in ids {
        let xml_id = n.saturating_sub(1);
        parent.push_element(
            Element::new("brk")
                .with_attr("id", xml_id.to_string())
                .with_attr("max", max)
                .with_attr("man", "1"),
        );
    }
    let count = parent.children_by_local("brk").len();
    let man = parent
        .children_by_local("brk")
        .into_iter()
        .filter(|b| attr_on(b, "man"))
        .count();
    parent.set_attr("count", count.to_string());
    parent.set_attr("manualBreakCount", man.to_string());
}

fn ensure_child<'a>(sheet: &'a mut Element, local: &str) -> &'a mut Element {
    if sheet.child_by_local(local).is_none() {
        layout::insert_child_ordered(sheet, Element::new(local), local);
    }
    sheet.child_by_local_mut(local).unwrap()
}

fn ensure_page_setup_pr(sheet: &mut Element) -> &mut Element {
    if sheet.child_by_local("sheetPr").is_none() {
        layout::insert_child_ordered(sheet, Element::new("sheetPr"), "sheetPr");
    }
    let pr = sheet.child_by_local_mut("sheetPr").unwrap();
    if pr.child_by_local("pageSetUpPr").is_none() {
        pr.push_element(Element::new("pageSetUpPr"));
    }
    pr.child_by_local_mut("pageSetUpPr").unwrap()
}

fn parse_print_area_formula(raw: &str, sheet_name: &str, i: usize) -> Result<String, Error> {
    let t = raw.trim();
    let bad = || {
        Error::with_path(
            ErrorKind::InvalidPrintSetup {
                field: "printArea".into(),
            },
            format!("ops[{i}].printArea"),
        )
    };
    if t.is_empty() {
        return Err(bad());
    }
    if let Some(a1) = chart::parse_a1_formula(t) {
        if let Some(ref sh) = a1.sheet {
            if !sh.eq_ignore_ascii_case(sheet_name) {
                return Err(bad());
            }
        }
        return Ok(chart::format_a1_formula(sheet_name, a1.start, a1.end));
    }
    if let Ok((start, end)) = a1::parse_range(t) {
        return Ok(chart::format_a1_formula(sheet_name, start, end));
    }
    Err(bad())
}

fn normalize_print_area(raw: &str, sheet_name: &str) -> String {
    if let Some(a1) = chart::parse_a1_formula(raw) {
        return chart::format_a1_formula(sheet_name, a1.start, a1.end);
    }
    raw.to_string()
}

fn inch(el: Option<&Element>, name: &str, def: f64) -> f64 {
    el.and_then(|e| e.attr(name))
        .and_then(|s| s.trim().parse().ok())
        .filter(|n: &f64| n.is_finite())
        .unwrap_or(def)
}

fn uint(el: Option<&Element>, name: &str, def: u32) -> u32 {
    el.and_then(|e| e.attr(name))
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(def)
}

fn orientation(setup: Option<&Element>) -> &'static str {
    match setup.and_then(|e| e.attr("orientation")).map(str::trim) {
        Some("landscape") => "landscape",
        _ => "portrait",
    }
}

fn fit_to_page(sheet: &Element) -> bool {
    sheet
        .child_by_local("sheetPr")
        .and_then(|p| p.child_by_local("pageSetUpPr"))
        .is_some_and(|e| attr_on(e, "fitToPage"))
}

fn grid_lines(opts: Option<&Element>) -> bool {
    let Some(e) = opts else {
        return false;
    };
    if !attr_on(e, "gridLines") {
        return false;
    }
    match e.attr("gridLinesSet") {
        None => true,
        Some(_) => attr_on(e, "gridLinesSet"),
    }
}

fn manual_breaks(sheet: &Element, local: &str) -> Vec<u32> {
    let Some(parent) = sheet.child_by_local(local) else {
        return Vec::new();
    };
    let mut out: Vec<u32> = parent
        .children_by_local("brk")
        .into_iter()
        .filter(|b| attr_on(b, "man"))
        .filter_map(|b| b.attr("id")?.parse::<u32>().ok())
        .map(|id| id.saturating_add(1))
        .collect();
    out.sort_unstable();
    out.dedup();
    out
}

fn attr_on(el: &Element, name: &str) -> bool {
    matches!(
        el.attr(name)
            .map(|s| s.trim().to_ascii_lowercase())
            .as_deref(),
        Some("1") | Some("true") | Some("on")
    )
}

fn bool_attr(v: bool) -> &'static str {
    if v {
        "1"
    } else {
        "0"
    }
}

fn fmt_inch(n: f64) -> String {
    let s = format!("{n:.10}");
    let t = s.trim_end_matches('0').trim_end_matches('.');
    if t == "-0" || t.is_empty() {
        "0".into()
    } else {
        t.to_string()
    }
}
