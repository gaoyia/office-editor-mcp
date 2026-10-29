//! 普通工作表的插、删、改名、重排与隐藏。

use crate::error::{Error, ErrorKind, IdReason};
use crate::opc::{
    rels_part_name, resolve_rel_target, ContentTypes, Package, Relationship, Relationships,
};
use crate::xml::{self, Document, Element, Node, DEFAULT_DECL};

use super::apply;
use super::chart as xlsx_chart;
use super::inspect;
use super::picture;
use super::request::{DeleteSheetOp, InsertSheetOp, MoveSheetOp, RenameSheetOp, SetSheetHiddenOp};
use super::{CT_WORKSHEET, REL_NS, REL_WORKSHEET, SML_NS};

pub fn insert(
    pkg: &mut Package,
    wb_doc: &mut Document,
    wb_part: &str,
    sheet_index: &mut Vec<(String, String, &'static str)>,
    op: &InsertSheetOp,
    op_index: usize,
) -> Result<(), Error> {
    if sheet_index
        .iter()
        .any(|(n, _, _)| n.eq_ignore_ascii_case(&op.name))
    {
        return Err(Error::with_path(
            ErrorKind::DuplicateSheetName {
                name: op.name.clone(),
            },
            format!("ops[{op_index}].name"),
        ));
    }
    let insert_at = match &op.after {
        None => sheet_index.len(),
        Some(after) => {
            let pos = sheet_index
                .iter()
                .position(|(n, _, _)| n.eq_ignore_ascii_case(after))
                .ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::SheetNotFound {
                            name: after.clone(),
                        },
                        format!("ops[{op_index}].after"),
                    )
                })?;
            pos + 1
        }
    };
    let ws_part = next_worksheet_part(pkg);
    pkg.set(&ws_part, empty_worksheet().to_bytes()?)?;
    let mut wb_rels = Relationships::from_package(pkg, wb_part)?;
    let sheet_rid = wb_rels.next_rid();
    wb_rels.push(Relationship {
        id: sheet_rid.clone(),
        rel_type: REL_WORKSHEET.into(),
        target: picture::rel_target_from(wb_part, &ws_part),
        target_mode: None,
        extra: Vec::new(),
    });
    wb_rels.write_to(pkg, wb_part)?;
    picture::ensure_xmlns(&mut wb_doc.root, "r", REL_NS);
    let sheets = workbook_sheets_mut(&mut wb_doc.root)?;
    let sheet_id = next_sheet_id(sheets);
    let el = Element::new("sheet")
        .with_attr("name", op.name.clone())
        .with_attr("sheetId", sheet_id.to_string())
        .with_attr("r:id", sheet_rid);
    insert_sheet_element(sheets, insert_at, el);
    let mut cts = ContentTypes::from_package(pkg)?;
    cts.set_override(&ws_part, CT_WORKSHEET)?;
    cts.write_to(pkg)?;
    sheet_index.insert(insert_at, (op.name.clone(), ws_part, "worksheet"));
    Ok(())
}

pub fn delete(
    pkg: &mut Package,
    wb_doc: &mut Document,
    wb_part: &str,
    sheet_index: &mut Vec<(String, String, &'static str)>,
    dirty_sheets: &mut Vec<(String, Document)>,
    op: &DeleteSheetOp,
    op_index: usize,
) -> Result<(), Error> {
    let meta = sheet_index
        .iter()
        .find(|(n, _, _)| n.eq_ignore_ascii_case(&op.sheet))
        .cloned();
    let Some((exact, ws_part, kind)) = meta else {
        return Err(Error::with_path(
            ErrorKind::SheetNotFound {
                name: op.sheet.clone(),
            },
            format!("ops[{op_index}].id"),
        ));
    };
    match kind {
        "chartsheet" => {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::DeleteSheetNeedsWorksheet,
                },
                format!("ops[{op_index}].id"),
            ));
        }
        "dialogsheet" | "macrosheet" => {
            return Err(Error::with_path(
                ErrorKind::DialogOrMacroSheet,
                format!("ops[{op_index}].id"),
            ));
        }
        _ => {}
    }
    reject_if_last_sheet(sheet_index.len(), op_index)?;
    reject_if_last_visible(&wb_doc.root, &exact, op_index)?;
    let skip_charts = chart_parts_on_sheet(pkg, dirty_sheets, &ws_part)?;
    if sheet_name_still_referenced(
        pkg,
        &wb_doc.root,
        sheet_index,
        dirty_sheets,
        &exact,
        Some(&ws_part),
        &skip_charts,
    )? {
        return Err(Error::with_path(
            ErrorKind::SheetStillReferenced {
                name: exact.clone(),
            },
            format!("ops[{op_index}].id"),
        ));
    }
    apply::load_dirty(pkg, dirty_sheets, &ws_part, false)?;
    let drawing_parts = {
        let doc = apply::dirty_of(dirty_sheets, &ws_part);
        let mut parts = picture::drawing_parts_from(pkg, &ws_part, &doc.root)?;
        parts.extend(picture::drawing_hf_parts_from(pkg, &ws_part, &doc.root)?);
        parts
    };
    let mut charts = Vec::new();
    let mut media = Vec::new();
    for drawing_part in &drawing_parts {
        charts.extend(chart_parts_of_drawing(pkg, drawing_part)?);
        media.extend(image_parts_of_drawing(pkg, drawing_part)?);
    }
    let extra_parts = internal_rel_targets(pkg, &ws_part)?;
    let rid = remove_workbook_sheet(&mut wb_doc.root, &exact).ok_or_else(|| {
        Error::new(ErrorKind::XmlMissing {
            parent: "sheets".into(),
            child: "sheet".into(),
        })
    })?;
    let mut wb_rels = Relationships::from_package(pkg, wb_part)?;
    wb_rels.remove_by_id(&rid);
    wb_rels.write_to(pkg, wb_part)?;
    dirty_sheets.retain(|(p, _)| p != &ws_part);
    pkg.remove(&ws_part);
    pkg.remove(&rels_part_name(&ws_part));
    let mut cts = ContentTypes::from_package(pkg)?;
    cts.remove_override(&ws_part);
    for drawing_part in &drawing_parts {
        pkg.remove(drawing_part);
        pkg.remove(&rels_part_name(drawing_part));
        cts.remove_override(drawing_part);
    }
    for part in extra_parts {
        if part == ws_part || drawing_parts.iter().any(|d| d == &part) {
            continue;
        }
        if !part_still_used(pkg, &part)? {
            pkg.remove(&part);
            pkg.remove(&rels_part_name(&part));
            cts.remove_override(&part);
        }
    }
    cts.write_to(pkg)?;
    for chart_part in charts {
        if !xlsx_chart::chart_still_used(pkg, &chart_part)? {
            xlsx_chart::remove_chart_tree(pkg, &chart_part)?;
        }
    }
    for media_part in media {
        if !picture::media_still_used(pkg, &media_part)? {
            pkg.remove(&media_part);
        }
    }
    sheet_index.retain(|(n, _, _)| !n.eq_ignore_ascii_case(&op.sheet));
    clamp_active_tab(&mut wb_doc.root, sheet_index.len());
    Ok(())
}

pub fn rename(
    pkg: &Package,
    wb_doc: &mut Document,
    sheet_index: &mut Vec<(String, String, &'static str)>,
    dirty_sheets: &[(String, Document)],
    op: &RenameSheetOp,
    op_index: usize,
) -> Result<(), Error> {
    let meta = sheet_index
        .iter()
        .find(|(n, _, _)| n.eq_ignore_ascii_case(&op.sheet))
        .cloned();
    let Some((exact, part, kind)) = meta else {
        return Err(Error::with_path(
            ErrorKind::SheetNotFound {
                name: op.sheet.clone(),
            },
            format!("ops[{op_index}].id"),
        ));
    };
    reject_dialog_or_macro(kind, op_index)?;
    if exact == op.name {
        return Ok(());
    }
    if sheet_index
        .iter()
        .any(|(n, _, _)| n.eq_ignore_ascii_case(&op.name) && !n.eq_ignore_ascii_case(&exact))
    {
        return Err(Error::with_path(
            ErrorKind::DuplicateSheetName {
                name: op.name.clone(),
            },
            format!("ops[{op_index}].name"),
        ));
    }
    if sheet_name_still_referenced(
        pkg,
        &wb_doc.root,
        sheet_index,
        dirty_sheets,
        &exact,
        Some(&part),
        &[],
    )? {
        return Err(Error::with_path(
            ErrorKind::SheetStillReferenced {
                name: exact.clone(),
            },
            format!("ops[{op_index}].id"),
        ));
    }
    let sheets = workbook_sheets_mut(&mut wb_doc.root)?;
    let el = find_sheet_el_mut(sheets, &exact).ok_or_else(|| {
        Error::new(ErrorKind::XmlMissing {
            parent: "sheets".into(),
            child: "sheet".into(),
        })
    })?;
    el.set_attr("name", op.name.clone());
    if let Some((n, _, _)) = sheet_index
        .iter_mut()
        .find(|(n, _, _)| n.eq_ignore_ascii_case(&op.sheet))
    {
        *n = op.name.clone();
    }
    Ok(())
}

pub fn move_sheet(
    wb_doc: &mut Document,
    sheet_index: &mut Vec<(String, String, &'static str)>,
    op: &MoveSheetOp,
    op_index: usize,
) -> Result<(), Error> {
    let from = sheet_index
        .iter()
        .position(|(n, _, _)| n.eq_ignore_ascii_case(&op.sheet))
        .ok_or_else(|| {
            Error::with_path(
                ErrorKind::SheetNotFound {
                    name: op.sheet.clone(),
                },
                format!("ops[{op_index}].id"),
            )
        })?;
    reject_dialog_or_macro(sheet_index[from].2, op_index)?;
    if let Some(after) = &op.after {
        if after.eq_ignore_ascii_case(&op.sheet) {
            return Err(Error::with_path(
                ErrorKind::MoveSheetAfterSelf,
                format!("ops[{op_index}].after"),
            ));
        }
    }
    let dest = match &op.after {
        None => 0usize,
        Some(after) => {
            let pos = sheet_index
                .iter()
                .position(|(n, _, _)| n.eq_ignore_ascii_case(after))
                .ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::SheetNotFound {
                            name: after.clone(),
                        },
                        format!("ops[{op_index}].after"),
                    )
                })?;
            pos + 1
        }
    };
    if from == dest {
        return Ok(());
    }
    let sheets = workbook_sheets_mut(&mut wb_doc.root)?;
    let from_node = find_sheet_child_index(sheets, &sheet_index[from].0).ok_or_else(|| {
        Error::new(ErrorKind::XmlMissing {
            parent: "sheets".into(),
            child: "sheet".into(),
        })
    })?;
    let dest_node = if dest == 0 {
        first_sheet_child_index(sheets).unwrap_or(0)
    } else {
        let after_name = &sheet_index[dest - 1].0;
        let after_idx = find_sheet_child_index(sheets, after_name).ok_or_else(|| {
            Error::new(ErrorKind::XmlMissing {
                parent: "sheets".into(),
                child: "sheet".into(),
            })
        })?;
        after_idx + 1
    };
    if from_node != dest_node {
        let node = sheets.children.remove(from_node);
        let insert_at = if dest_node > from_node {
            dest_node - 1
        } else {
            dest_node
        };
        sheets.children.insert(insert_at, node);
    }
    let item = sheet_index.remove(from);
    let insert_at = if dest > from { dest - 1 } else { dest };
    sheet_index.insert(insert_at, item);
    Ok(())
}

pub fn set_hidden(
    wb_doc: &mut Document,
    sheet_index: &[(String, String, &'static str)],
    op: &SetSheetHiddenOp,
    op_index: usize,
) -> Result<(), Error> {
    let meta = sheet_index
        .iter()
        .find(|(n, _, _)| n.eq_ignore_ascii_case(&op.sheet))
        .cloned();
    let Some((exact, _, kind)) = meta else {
        return Err(Error::with_path(
            ErrorKind::SheetNotFound {
                name: op.sheet.clone(),
            },
            format!("ops[{op_index}].id"),
        ));
    };
    reject_dialog_or_macro(kind, op_index)?;
    let state = {
        let sheets = wb_doc.root.child_by_local("sheets").ok_or_else(|| {
            Error::new(ErrorKind::XmlMissing {
                parent: "workbook.xml".into(),
                child: "sheets".into(),
            })
        })?;
        let el = sheets
            .children_by_local("sheet")
            .into_iter()
            .find(|s| {
                s.attr("name")
                    .is_some_and(|n| n.eq_ignore_ascii_case(&exact))
            })
            .ok_or_else(|| {
                Error::new(ErrorKind::XmlMissing {
                    parent: "sheets".into(),
                    child: "sheet".into(),
                })
            })?;
        el.attr("state").map(str::to_string)
    };
    let currently_hidden = inspect::sheet_state_hidden(state.as_deref());
    if op.hidden {
        if state.as_deref() == Some("hidden") {
            return Ok(());
        }
        if !currently_hidden {
            reject_if_last_visible(&wb_doc.root, &exact, op_index)?;
        }
    } else if matches!(state.as_deref(), None | Some("visible")) {
        return Ok(());
    }
    let sheets = workbook_sheets_mut(&mut wb_doc.root)?;
    let el = find_sheet_el_mut(sheets, &exact).ok_or_else(|| {
        Error::new(ErrorKind::XmlMissing {
            parent: "sheets".into(),
            child: "sheet".into(),
        })
    })?;
    if op.hidden {
        el.set_attr("state", "hidden");
    } else {
        el.remove_attr("state");
    }
    Ok(())
}

pub(crate) fn next_sheet_id(sheets: &Element) -> u32 {
    let mut max = 0u32;
    for sh in sheets.children_by_local("sheet") {
        if let Some(id) = sh.attr("sheetId").and_then(|s| s.parse().ok()) {
            max = max.max(id);
        }
    }
    max + 1
}

pub(crate) fn remove_workbook_sheet(wb: &mut Element, name: &str) -> Option<String> {
    let sheets = wb.child_by_local_mut("sheets")?;
    let idx = find_sheet_child_index(sheets, name)?;
    let Node::Element(el) = sheets.children.remove(idx) else {
        return None;
    };
    el.attr_local("id").map(str::to_string)
}

pub(crate) fn clamp_active_tab(wb: &mut Element, sheet_count: usize) {
    if sheet_count == 0 {
        return;
    }
    let Some(views) = wb.child_by_local_mut("bookViews") else {
        return;
    };
    for view in views.element_children_mut() {
        if view.local_name() != "workbookView" {
            continue;
        }
        let Some(tab) = view.attr("activeTab").and_then(|s| s.parse::<usize>().ok()) else {
            continue;
        };
        if tab >= sheet_count {
            view.set_attr("activeTab", (sheet_count - 1).to_string());
        }
    }
}

pub(crate) fn reject_if_last_sheet(count: usize, op_index: usize) -> Result<(), Error> {
    if count <= 1 {
        return Err(Error::with_path(
            ErrorKind::NeedAtLeastOneSheet,
            format!("ops[{op_index}].id"),
        ));
    }
    Ok(())
}

pub(crate) fn reject_if_last_visible(
    wb: &Element,
    name: &str,
    op_index: usize,
) -> Result<(), Error> {
    let sheets = wb.child_by_local("sheets").ok_or_else(|| {
        Error::new(ErrorKind::XmlMissing {
            parent: "workbook.xml".into(),
            child: "sheets".into(),
        })
    })?;
    reject_if_last_visible_el(sheets, name, op_index)
}

fn reject_if_last_visible_el(sheets: &Element, name: &str, op_index: usize) -> Result<(), Error> {
    let Some(target) = sheets
        .children_by_local("sheet")
        .into_iter()
        .find(|s| s.attr("name").is_some_and(|n| n.eq_ignore_ascii_case(name)))
    else {
        return Ok(());
    };
    if !inspect::sheet_state_visible(target.attr("state")) {
        return Ok(());
    }
    let visible = sheets
        .children_by_local("sheet")
        .into_iter()
        .filter(|s| inspect::sheet_state_visible(s.attr("state")))
        .count();
    if visible <= 1 {
        return Err(Error::with_path(
            ErrorKind::LastVisibleSheet,
            format!("ops[{op_index}].id"),
        ));
    }
    Ok(())
}

fn reject_dialog_or_macro(kind: &str, op_index: usize) -> Result<(), Error> {
    if kind == "dialogsheet" || kind == "macrosheet" {
        return Err(Error::with_path(
            ErrorKind::DialogOrMacroSheet,
            format!("ops[{op_index}].id"),
        ));
    }
    Ok(())
}

fn workbook_sheets_mut(wb: &mut Element) -> Result<&mut Element, Error> {
    wb.child_by_local_mut("sheets").ok_or_else(|| {
        Error::new(ErrorKind::XmlMissing {
            parent: "workbook.xml".into(),
            child: "sheets".into(),
        })
    })
}

fn find_sheet_el_mut<'a>(sheets: &'a mut Element, name: &str) -> Option<&'a mut Element> {
    sheets.element_children_mut().find(|e| {
        e.local_name() == "sheet" && e.attr("name").is_some_and(|n| n.eq_ignore_ascii_case(name))
    })
}

fn find_sheet_child_index(sheets: &Element, name: &str) -> Option<usize> {
    sheets.children.iter().position(|n| match n {
        Node::Element(e) => {
            e.local_name() == "sheet"
                && e.attr("name").is_some_and(|n| n.eq_ignore_ascii_case(name))
        }
        Node::Text(_) => false,
    })
}

fn first_sheet_child_index(sheets: &Element) -> Option<usize> {
    sheets.children.iter().position(|n| match n {
        Node::Element(e) => e.local_name() == "sheet",
        Node::Text(_) => false,
    })
}

fn insert_sheet_element(sheets: &mut Element, sheet_pos: usize, el: Element) {
    let mut seen = 0usize;
    for (i, n) in sheets.children.iter().enumerate() {
        let Node::Element(e) = n else {
            continue;
        };
        if e.local_name() != "sheet" {
            continue;
        }
        if seen == sheet_pos {
            sheets.children.insert(i, Node::Element(el));
            return;
        }
        seen += 1;
    }
    sheets.children.push(Node::Element(el));
}

fn next_worksheet_part(pkg: &Package) -> String {
    let mut n = 1u32;
    loop {
        let name = format!("/xl/worksheets/sheet{n}.xml");
        if !pkg.contains(&name) {
            return name;
        }
        n += 1;
    }
}

fn empty_worksheet() -> Document {
    let mut root = Element::new("worksheet")
        .with_attr("xmlns", SML_NS)
        .with_attr("xmlns:r", REL_NS);
    root.push_element(Element::new("sheetData"));
    Document::new(Some(DEFAULT_DECL.to_string()), root)
}

fn sheet_name_still_referenced(
    pkg: &Package,
    wb: &Element,
    sheet_index: &[(String, String, &'static str)],
    dirty_sheets: &[(String, Document)],
    name: &str,
    skip_part: Option<&str>,
    skip_charts: &[String],
) -> Result<bool, Error> {
    if let Some(dns) = wb.child_by_local("definedNames") {
        for dn in dns.children_by_local("definedName") {
            if formula_refers_to_sheet(&dn.text_content(), name) {
                return Ok(true);
            }
        }
    }
    for (sheet_name, part, _) in sheet_index {
        if skip_part.is_some_and(|p| p == part) {
            continue;
        }
        if sheet_name.eq_ignore_ascii_case(name) {
            continue;
        }
        let texts = formula_texts_of_part(pkg, dirty_sheets, part)?;
        if texts.iter().any(|t| formula_refers_to_sheet(t, name)) {
            return Ok(true);
        }
    }
    for chart_part in chart_parts_in_package(pkg) {
        if skip_charts.iter().any(|p| p == &chart_part) {
            continue;
        }
        let Ok(bytes) = pkg.get(&chart_part) else {
            continue;
        };
        let Ok(doc) = xml::parse(bytes) else {
            continue;
        };
        if collect_local_texts(&doc.root, "f")
            .iter()
            .any(|t| formula_refers_to_sheet(t, name))
        {
            return Ok(true);
        }
    }
    Ok(false)
}

fn formula_texts_of_part(
    pkg: &Package,
    dirty: &[(String, Document)],
    part: &str,
) -> Result<Vec<String>, Error> {
    if let Some((_, doc)) = dirty.iter().find(|(p, _)| p == part) {
        return Ok(collect_local_texts(&doc.root, "f"));
    }
    let Ok(bytes) = pkg.get(part) else {
        return Ok(Vec::new());
    };
    let doc = xml::parse(bytes)?;
    Ok(collect_local_texts(&doc.root, "f"))
}

fn collect_local_texts(el: &Element, local: &str) -> Vec<String> {
    let mut out = Vec::new();
    collect_local_texts_into(el, local, &mut out);
    out
}

fn collect_local_texts_into(el: &Element, local: &str, out: &mut Vec<String>) {
    if el.local_name() == local {
        out.push(el.text_content());
    }
    for c in el.element_children() {
        collect_local_texts_into(c, local, out);
    }
}

fn chart_parts_on_sheet(
    pkg: &Package,
    dirty: &mut Vec<(String, Document)>,
    sheet_part: &str,
) -> Result<Vec<String>, Error> {
    apply::load_dirty(pkg, dirty, sheet_part, false)?;
    let doc = apply::dirty_of(dirty, sheet_part);
    let drawings = picture::drawing_parts_from(pkg, sheet_part, &doc.root)?;
    let mut out = Vec::new();
    for d in drawings {
        out.extend(chart_parts_of_drawing(pkg, &d)?);
    }
    Ok(out)
}

fn chart_parts_in_package(pkg: &Package) -> Vec<String> {
    pkg.part_names()
        .into_iter()
        .filter(|n| {
            n.contains("/charts/")
                && n.ends_with(".xml")
                && !n.contains("/_rels/")
                && !n.contains("colors")
                && !n.contains("style")
        })
        .collect()
}

fn chart_parts_of_drawing(pkg: &Package, drawing_part: &str) -> Result<Vec<String>, Error> {
    rel_targets_ending(pkg, drawing_part, &["/chart", "/chartEx"])
}

fn image_parts_of_drawing(pkg: &Package, drawing_part: &str) -> Result<Vec<String>, Error> {
    rel_targets_ending(pkg, drawing_part, &["/image"])
}

fn rel_targets_ending(pkg: &Package, part: &str, suffixes: &[&str]) -> Result<Vec<String>, Error> {
    let rels = Relationships::from_package(pkg, part)?;
    let mut out = Vec::new();
    for r in &rels.rels {
        if r.is_external() {
            continue;
        }
        if !suffixes.iter().any(|s| r.rel_type.ends_with(s)) {
            continue;
        }
        if let Ok(p) = resolve_rel_target(part, &r.target) {
            if pkg.contains(&p) {
                out.push(p);
            }
        }
    }
    Ok(out)
}

fn internal_rel_targets(pkg: &Package, part: &str) -> Result<Vec<String>, Error> {
    let rels = Relationships::from_package(pkg, part)?;
    let mut out = Vec::new();
    for r in &rels.rels {
        if r.is_external() {
            continue;
        }
        if let Ok(p) = resolve_rel_target(part, &r.target) {
            if pkg.contains(&p) {
                out.push(p);
            }
        }
    }
    Ok(out)
}

fn part_still_used(pkg: &Package, part: &str) -> Result<bool, Error> {
    for name in pkg.part_names() {
        if !name.ends_with(".rels") {
            continue;
        }
        let Some(source) = source_from_rels(&name) else {
            continue;
        };
        let Ok(rels) = Relationships::parse(pkg.get(&name)?) else {
            continue;
        };
        for r in &rels.rels {
            if r.is_external() {
                continue;
            }
            if resolve_rel_target(&source, &r.target).ok().as_deref() == Some(part) {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn source_from_rels(rels_part: &str) -> Option<String> {
    let trimmed = rels_part.strip_suffix(".rels")?;
    let (dir, file) = crate::opc::split_dir_file(trimmed);
    let parent = dir.trim_end_matches("/_rels").trim_end_matches("_rels");
    if parent.is_empty() || parent == "/" {
        Some(format!("/{file}"))
    } else {
        Some(format!("{parent}/{file}"))
    }
}

/// 公式或定义名是否引用指定工作表（大小写不敏感）。
fn formula_refers_to_sheet(text: &str, sheet: &str) -> bool {
    let stripped = strip_excel_strings(text);
    quoted_sheet_ref(&stripped, sheet) || unquoted_sheet_ref(&stripped, sheet)
}

fn strip_excel_strings(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '"' {
            out.push(' ');
            i += 1;
            while i < chars.len() {
                if chars[i] == '"' {
                    if i + 1 < chars.len() && chars[i + 1] == '"' {
                        i += 2;
                        continue;
                    }
                    out.push(' ');
                    i += 1;
                    break;
                }
                i += 1;
            }
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

fn quoted_sheet_ref(text: &str, sheet: &str) -> bool {
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '\'' {
            i += 1;
            let mut name = String::new();
            while i < chars.len() {
                if chars[i] == '\'' {
                    if i + 1 < chars.len() && chars[i + 1] == '\'' {
                        name.push('\'');
                        i += 2;
                        continue;
                    }
                    i += 1;
                    break;
                }
                name.push(chars[i]);
                i += 1;
            }
            if i < chars.len() && chars[i] == '!' && name_token_matches(&name, sheet) {
                return true;
            }
        } else {
            i += 1;
        }
    }
    false
}

fn unquoted_sheet_ref(text: &str, sheet: &str) -> bool {
    if needs_quote(sheet) {
        return false;
    }
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '!' && i > 0 {
            let mut j = i;
            while j > 0 {
                let c = chars[j - 1];
                if is_unquoted_name_char(c) || c == ':' || c == ']' {
                    j -= 1;
                } else {
                    break;
                }
            }
            let token: String = chars[j..i].iter().collect();
            if name_token_matches(&token, sheet) {
                return true;
            }
        }
        i += 1;
    }
    false
}

fn name_token_matches(token: &str, sheet: &str) -> bool {
    let rest = if let Some(idx) = token.rfind(']') {
        &token[idx + 1..]
    } else {
        token
    };
    rest.split(':').any(|p| p.eq_ignore_ascii_case(sheet))
}

fn needs_quote(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return true;
    };
    if first.is_ascii_digit() {
        return true;
    }
    !name.chars().all(is_unquoted_name_char)
}

fn is_unquoted_name_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '.'
}

#[cfg(test)]
mod formula_tests {
    use super::formula_refers_to_sheet;

    #[test]
    fn unquoted_and_quoted_refs() {
        assert!(formula_refers_to_sheet("Sheet1!A1", "Sheet1"));
        assert!(formula_refers_to_sheet("sheet1!$A$1", "Sheet1"));
        assert!(!formula_refers_to_sheet("Sheet10!A1", "Sheet1"));
        assert!(formula_refers_to_sheet("'销售 2026'!A1", "销售 2026"));
        assert!(formula_refers_to_sheet("Sheet1:Sheet3!A1", "Sheet3"));
        assert!(!formula_refers_to_sheet("\"Sheet1!A1\"", "Sheet1"));
        assert!(formula_refers_to_sheet("'O''Brien'!A1", "O'Brien"));
    }
}
