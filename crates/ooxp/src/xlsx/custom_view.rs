//! 工作簿自定义视图：inspect 名单，apply 写 `customWorkbookViews` / `customSheetViews`。
//! 不按视图恢复隐藏行 / 列，不实现放映引擎。

use serde_json::{json, Map, Value};
use std::sync::atomic::{AtomicU32, Ordering};

use crate::error::{Error, ErrorKind, IdReason};
use crate::xml::{Document, Element, Node};

use super::layout;
use super::names;
use super::view;

static GUID_SEQ: AtomicU32 = AtomicU32::new(1);

const DEF_WIN_W: i64 = 1456;
const DEF_WIN_H: i64 = 1007;
const DEF_MARGIN_LEFT: &str = "0.7";
const DEF_MARGIN_RIGHT: &str = "0.7";
const DEF_MARGIN_TOP: &str = "0.75";
const DEF_MARGIN_BOTTOM: &str = "0.75";
const DEF_MARGIN_HEADER: &str = "0.3";
const DEF_MARGIN_FOOTER: &str = "0.3";

#[derive(Debug, Clone)]
pub struct InsertCustomViewOp {
    pub name: String,
    pub include_print_settings: bool,
    pub include_hidden_row_col: bool,
}

#[derive(Debug, Clone)]
pub struct DeleteCustomViewOp {
    #[allow(dead_code)]
    pub id: String,
    pub index: usize,
}

#[derive(Debug, Clone)]
pub struct ShowCustomViewOp {
    #[allow(dead_code)]
    pub id: String,
    pub index: usize,
}

pub fn parse_custom_view_id(id: &str, json_path: &str) -> Result<usize, Error> {
    let rest = id.strip_prefix("customView:").ok_or_else(|| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::CustomViewPrefix,
            },
            json_path.to_string(),
        )
    })?;
    if rest.is_empty() {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::CustomViewPrefix,
            },
            json_path.to_string(),
        ));
    }
    let n: usize = rest.parse().map_err(|_| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::CustomViewIndexNotInt,
            },
            json_path.to_string(),
        )
    })?;
    if n < 1 {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::CustomViewIndexFromOne,
            },
            json_path.to_string(),
        ));
    }
    Ok(n)
}

pub(crate) fn count(wb: &Element) -> usize {
    workbook_views(wb).len()
}

pub(crate) fn inspect(wb: &Element) -> Value {
    let items: Vec<Value> = workbook_views(wb)
        .into_iter()
        .enumerate()
        .map(|(i, el)| inspect_one(el, i + 1, wb))
        .collect();
    Value::Array(items)
}

fn inspect_one(el: &Element, n: usize, wb: &Element) -> Value {
    let mut obj = Map::new();
    obj.insert("id".into(), json!(format!("customView:{n}")));
    obj.insert("name".into(), json!(el.attr("name").unwrap_or("")));
    obj.insert(
        "includePrintSettings".into(),
        json!(attr_on_default_true(el, "includePrintSettings")),
    );
    obj.insert(
        "includeHiddenRowCol".into(),
        json!(attr_on_default_true(el, "includeHiddenRowCol")),
    );
    obj.insert("activeSheet".into(), json!(active_sheet_name(el, wb)));
    Value::Object(obj)
}

pub fn insert(
    wb: &mut Element,
    sheet_index: &[(String, String, &'static str)],
    dirty: &mut [(String, Document)],
    op: &InsertCustomViewOp,
    i: usize,
) -> Result<(), Error> {
    if name_taken(wb, &op.name) {
        return Err(Error::with_path(
            ErrorKind::InvalidCustomView {
                field: "name".into(),
            },
            format!("ops[{i}].name"),
        ));
    }
    let guid = unique_guid(wb);
    let (x, y, w, h) = window_metrics(wb);
    let active = selected_sheet_id(wb, sheet_index, dirty);
    let mut view_el = Element::new("customWorkbookView")
        .with_attr("name", &op.name)
        .with_attr("guid", &guid);
    view_el.set_attr("xWindow", x.to_string());
    view_el.set_attr("yWindow", y.to_string());
    view_el.set_attr("windowWidth", w.to_string());
    view_el.set_attr("windowHeight", h.to_string());
    view_el.set_attr("activeSheetId", active);
    if !op.include_print_settings {
        view_el.set_attr("includePrintSettings", "0");
    }
    if !op.include_hidden_row_col {
        view_el.set_attr("includeHiddenRowCol", "0");
    }
    if let Some(parent) = wb.child_by_local_mut("customWorkbookViews") {
        parent.push_element(view_el);
    } else {
        let mut parent = Element::new("customWorkbookViews");
        parent.push_element(view_el);
        names::insert_workbook_child(wb, parent, "customWorkbookViews");
    }
    let targets: Vec<String> = sheet_index
        .iter()
        .filter(|(_, _, kind)| *kind == "worksheet" || *kind == "chartsheet")
        .map(|(_, part, _)| part.clone())
        .collect();
    for part in targets {
        let doc = dirty
            .iter_mut()
            .find(|(p, _)| p == &part)
            .map(|(_, d)| d)
            .expect("insert 前已 load_dirty");
        write_custom_sheet_view(&mut doc.root, &guid, op.include_print_settings);
    }
    Ok(())
}

pub fn delete(
    wb: &mut Element,
    sheet_index: &[(String, String, &'static str)],
    dirty: &mut [(String, Document)],
    op: &DeleteCustomViewOp,
    i: usize,
) -> Result<(), Error> {
    let guid = nth_guid(wb, op.index, &format!("ops[{i}].id"))?;
    remove_workbook_view(wb, op.index);
    let targets: Vec<String> = sheet_index
        .iter()
        .filter(|(_, _, kind)| *kind == "worksheet" || *kind == "chartsheet")
        .map(|(_, part, _)| part.clone())
        .collect();
    for part in targets {
        let doc = dirty
            .iter_mut()
            .find(|(p, _)| p == &part)
            .map(|(_, d)| d)
            .expect("delete 前已 load_dirty");
        strip_custom_sheet_views(&mut doc.root, &guid);
    }
    Ok(())
}

pub fn show(
    wb: &Element,
    sheet_index: &[(String, String, &'static str)],
    dirty: &mut [(String, Document)],
    op: &ShowCustomViewOp,
    i: usize,
) -> Result<(), Error> {
    let (guid, include_print) = nth_show(wb, op.index, &format!("ops[{i}].id"))?;
    let targets: Vec<String> = sheet_index
        .iter()
        .filter(|(_, _, kind)| *kind == "worksheet" || *kind == "chartsheet")
        .map(|(_, part, _)| part.clone())
        .collect();
    for part in targets {
        let doc = dirty
            .iter_mut()
            .find(|(p, _)| p == &part)
            .map(|(_, d)| d)
            .expect("show 前已 load_dirty");
        apply_snapshot(&mut doc.root, &guid, include_print);
    }
    Ok(())
}

fn workbook_views(wb: &Element) -> Vec<&Element> {
    wb.child_by_local("customWorkbookViews")
        .map(|p| p.children_by_local("customWorkbookView"))
        .unwrap_or_default()
}

fn attr_on_default_true(el: &Element, name: &str) -> bool {
    !matches!(
        el.attr(name).map(|s| s.to_ascii_lowercase()).as_deref(),
        Some("0") | Some("false") | Some("off")
    )
}

fn active_sheet_name(el: &Element, wb: &Element) -> Option<String> {
    let id = el.attr("activeSheetId")?;
    let sheets = wb.child_by_local("sheets")?;
    for sh in sheets.children_by_local("sheet") {
        if sh.attr("sheetId") == Some(id) {
            return sh.attr("name").map(str::to_string);
        }
    }
    None
}

fn name_taken(wb: &Element, name: &str) -> bool {
    workbook_views(wb).iter().any(|el| {
        el.attr("name")
            .is_some_and(|n| n.eq_ignore_ascii_case(name))
    })
}

fn unique_guid(wb: &Element) -> String {
    let used: Vec<String> = workbook_views(wb)
        .iter()
        .filter_map(|el| el.attr("guid").map(str::to_string))
        .collect();
    loop {
        let g = new_guid();
        if used.iter().all(|u| !guid_eq(Some(u), &g)) {
            return g;
        }
    }
}

fn window_metrics(wb: &Element) -> (i64, i64, i64, i64) {
    let view = wb
        .child_by_local("bookViews")
        .and_then(|p| p.child_by_local("workbookView"));
    let Some(view) = view else {
        return (0, 0, DEF_WIN_W, DEF_WIN_H);
    };
    let x = parse_i64(view, "xWindow") / 15;
    let y = parse_i64(view, "yWindow") / 15;
    let w = scale_size(view, "windowWidth", DEF_WIN_W);
    let h = scale_size(view, "windowHeight", DEF_WIN_H);
    (x, y, w, h)
}

fn parse_i64(el: &Element, name: &str) -> i64 {
    el.attr(name)
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(0)
}

fn scale_size(el: &Element, name: &str, fallback: i64) -> i64 {
    let n = parse_i64(el, name) / 15;
    if n <= 0 {
        fallback
    } else {
        n
    }
}

fn selected_sheet_id(
    wb: &Element,
    sheet_index: &[(String, String, &'static str)],
    dirty: &[(String, Document)],
) -> String {
    for (name, part, kind) in sheet_index {
        if *kind != "worksheet" && *kind != "chartsheet" {
            continue;
        }
        let Some((_, doc)) = dirty.iter().find(|(p, _)| p == part) else {
            continue;
        };
        if view::first_sheet_view(&doc.root)
            .and_then(|v| v.attr("tabSelected"))
            .is_some_and(attr_true)
        {
            if let Some(id) = sheet_id_of(wb, name) {
                return id;
            }
        }
    }
    first_sheet_id(wb)
}

fn sheet_id_of(wb: &Element, name: &str) -> Option<String> {
    let sheets = wb.child_by_local("sheets")?;
    for sh in sheets.children_by_local("sheet") {
        if sh
            .attr("name")
            .is_some_and(|n| n.eq_ignore_ascii_case(name))
        {
            return sh.attr("sheetId").map(str::to_string);
        }
    }
    None
}

fn first_sheet_id(wb: &Element) -> String {
    wb.child_by_local("sheets")
        .and_then(|p| p.child_by_local("sheet"))
        .and_then(|s| s.attr("sheetId"))
        .unwrap_or("1")
        .to_string()
}

fn attr_true(raw: &str) -> bool {
    matches!(raw.to_ascii_lowercase().as_str(), "1" | "true" | "on")
}

fn write_custom_sheet_view(sheet: &mut Element, guid: &str, include_print: bool) {
    let copy_pane = view::has_split_or_freeze(sheet);
    let (pane, sels) = if copy_pane {
        view::snapshot_pane(sheet)
    } else {
        (None, Vec::new())
    };
    let margins = if include_print {
        Some(page_margins_of(sheet))
    } else {
        None
    };
    let mut csv = Element::new("customSheetView").with_attr("guid", guid);
    if let Some(p) = pane {
        csv.push_element(p);
    }
    for sel in sels {
        csv.push_element(sel);
    }
    if let Some(m) = margins {
        csv.push_element(m);
    }
    if let Some(parent) = sheet.child_by_local_mut("customSheetViews") {
        parent.push_element(csv);
        return;
    }
    let mut parent = Element::new("customSheetViews");
    parent.push_element(csv);
    layout::insert_child_ordered(sheet, parent, "customSheetViews");
}

fn page_margins_of(sheet: &Element) -> Element {
    if let Some(m) = sheet.child_by_local("pageMargins").cloned() {
        return m;
    }
    Element::new("pageMargins")
        .with_attr("left", DEF_MARGIN_LEFT)
        .with_attr("right", DEF_MARGIN_RIGHT)
        .with_attr("top", DEF_MARGIN_TOP)
        .with_attr("bottom", DEF_MARGIN_BOTTOM)
        .with_attr("header", DEF_MARGIN_HEADER)
        .with_attr("footer", DEF_MARGIN_FOOTER)
}

fn nth_guid(wb: &Element, index: usize, json_path: &str) -> Result<String, Error> {
    let list = workbook_views(wb);
    let Some(el) = list.get(index.wrapping_sub(1)) else {
        return Err(Error::with_path(
            ErrorKind::CustomViewNotFound {
                id: format!("customView:{index}"),
            },
            json_path.to_string(),
        ));
    };
    Ok(el.attr("guid").unwrap_or("").to_string())
}

fn nth_show(wb: &Element, index: usize, json_path: &str) -> Result<(String, bool), Error> {
    let list = workbook_views(wb);
    let Some(el) = list.get(index.wrapping_sub(1)) else {
        return Err(Error::with_path(
            ErrorKind::CustomViewNotFound {
                id: format!("customView:{index}"),
            },
            json_path.to_string(),
        ));
    };
    Ok((
        el.attr("guid").unwrap_or("").to_string(),
        attr_on_default_true(el, "includePrintSettings"),
    ))
}

fn remove_workbook_view(wb: &mut Element, index: usize) {
    let Some(parent) = wb.child_by_local_mut("customWorkbookViews") else {
        return;
    };
    let mut seen = 0usize;
    parent.children.retain(|n| match n {
        Node::Element(e) if e.local_name() == "customWorkbookView" => {
            seen += 1;
            seen != index
        }
        _ => true,
    });
    let empty = parent
        .element_children()
        .all(|e| e.local_name() != "customWorkbookView");
    if empty {
        wb.children
            .retain(|n| !matches!(n, Node::Element(e) if e.local_name() == "customWorkbookViews"));
    }
}

fn strip_custom_sheet_views(sheet: &mut Element, guid: &str) {
    let Some(parent) = sheet.child_by_local_mut("customSheetViews") else {
        return;
    };
    parent.children.retain(|n| match n {
        Node::Element(e) if e.local_name() == "customSheetView" => !guid_eq(e.attr("guid"), guid),
        _ => true,
    });
    let empty = parent
        .element_children()
        .all(|e| e.local_name() != "customSheetView");
    if empty {
        sheet
            .children
            .retain(|n| !matches!(n, Node::Element(e) if e.local_name() == "customSheetViews"));
    }
}

fn apply_snapshot(sheet: &mut Element, guid: &str, include_print: bool) {
    let csv = sheet.child_by_local("customSheetViews").and_then(|p| {
        p.children_by_local("customSheetView")
            .into_iter()
            .find(|e| guid_eq(e.attr("guid"), guid))
            .cloned()
    });
    let Some(csv) = csv else {
        return;
    };
    let pane = csv.child_by_local("pane").cloned();
    let sels: Vec<Element> = csv
        .children_by_local("selection")
        .into_iter()
        .cloned()
        .collect();
    {
        let view_el = view::ensure_default_view(sheet);
        let rest = view::strip_pane_and_selection(view_el);
        if let Some(p) = pane {
            view_el.push_element(p);
        }
        for sel in sels {
            view_el.push_element(sel);
        }
        if view_el.child_by_local("selection").is_none() {
            view_el.push_element(
                Element::new("selection")
                    .with_attr("activeCell", "A1")
                    .with_attr("sqref", "A1"),
            );
        }
        view_el.children.extend(rest);
    }
    if include_print {
        if let Some(m) = csv.child_by_local("pageMargins").cloned() {
            sheet
                .children
                .retain(|n| !matches!(n, Node::Element(e) if e.local_name() == "pageMargins"));
            layout::insert_child_ordered(sheet, m, "pageMargins");
        }
    }
}

fn guid_eq(have: Option<&str>, want: &str) -> bool {
    let Some(have) = have else {
        return want.is_empty();
    };
    have.eq_ignore_ascii_case(want)
}

fn new_guid() -> String {
    let t = crate::host_env::now_nanos();
    let n = GUID_SEQ.fetch_add(1, Ordering::Relaxed) as u128;
    let pid = crate::host_env::pid();
    let mix = t ^ (n << 48) ^ (pid << 16);
    let mut b = [0u8; 16];
    for (i, byte) in b.iter_mut().enumerate() {
        *byte = (mix >> ((i % 16) * 8)) as u8 ^ (i as u8).wrapping_mul(31);
    }
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    format!(
        "{{{:02X}{:02X}{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}}}",
        b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7], b[8], b[9], b[10], b[11], b[12], b[13],
        b[14], b[15]
    )
}
