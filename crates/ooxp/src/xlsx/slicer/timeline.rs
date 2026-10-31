use serde_json::{json, Value};

use crate::error::{Error, ErrorKind};
use crate::opc::{rels_part_name, resolve_rel_target, ContentTypes, Package, Relationships};
use crate::xml::{self, Document, Element, Node, DEFAULT_DECL};

use super::super::layout;
use super::super::names;
use super::super::picture::{self};
use super::super::REL_NS;
use super::common::*;
use super::{InsertSlicerOp, SlicerViewFields, TimelineFilter};

pub(crate) fn insert_timeline(
    pkg: &mut Package,
    wb_doc: &mut Document,
    wb_part: &str,
    sheet_doc: &mut Document,
    sheet_part: &str,
    op: &InsertSlicerOp,
    index: usize,
    op_index: usize,
) -> Result<(), Error> {
    let meta = super::pivot::pivot_field(
        pkg, wb_part, sheet_part, &op.sheet, index, &op.field, op_index,
    )?;
    // Excel 16.0 只在透视表 updatedVersion≥8 时加载时间线；库插入的透视默认是 4。
    super::pivot::bump_pivot_updated_version(pkg, &meta.part)?;
    let (start, end) = cache_date_bounds(pkg, wb_part, &meta.cache_id, &op.field, op_index)?;
    mark_cache_field_dates(pkg, wb_part, &meta.cache_id, &op.field, &start, &end)?;
    let tabular_id = super::pivot::ensure_x14_pivot_cache_id(pkg, wb_part, &meta.cache_id)?;
    let tab_id = sheet_id(wb_doc, &op.sheet).unwrap_or(1);
    let cx = op.cx.unwrap_or(DEFAULT_TIMELINE_CX);
    let cy = op.cy.unwrap_or(DEFAULT_TIMELINE_CY);
    let (to, to_off_x, to_off_y) = dest_to(&sheet_doc.root, op, cx, cy);
    let cache_name = unique_cache_name(pkg, wb_doc, &format!("Timeline_{}", op.field));
    let view_name = unique_view_name(pkg, &op.field);
    let cache_part = next_part(pkg, "/xl/timelineCaches/timelineCache", "xml");
    pkg.set(
        &cache_part,
        timeline_cache_xml(
            &cache_name,
            &op.field,
            tab_id,
            &meta.name,
            &tabular_id,
            &start,
            &end,
        ),
    )?;
    let cache_rid = add_rel(pkg, wb_part, REL_TIMELINE_CACHE, &cache_part)?;
    push_wb_timeline_cache(wb_doc, &cache_rid);
    push_cache_defined_name(wb_doc, &cache_name);
    let (timeline_part, timeline_rid, is_new) = ensure_timelines_part(pkg, sheet_doc, sheet_part)?;
    append_timeline_view(
        pkg,
        &timeline_part,
        &view_name,
        &cache_name,
        &op.field,
        op.level,
        &start,
        &op.view,
    )?;
    if is_new {
        push_sheet_timeline_refs(&mut sheet_doc.root, &timeline_rid);
    }
    let mut cts = ContentTypes::from_package(pkg)?;
    cts.set_override(&cache_part, CT_TIMELINE_CACHE)?;
    cts.set_override(&timeline_part, CT_TIMELINE)?;
    cts.write_to(pkg)?;
    write_drawing(
        pkg,
        sheet_doc,
        sheet_part,
        op,
        &view_name,
        SlicerFrameKind::Timeline,
        cx,
        cy,
        to,
        to_off_x,
        to_off_y,
    )?;
    Ok(())
}

pub(crate) fn timeline_cache_xml(
    name: &str,
    field: &str,
    tab_id: u32,
    pivot_name: &str,
    cache_id: &str,
    start: &str,
    end: &str,
) -> Vec<u8> {
    let mut pts = Element::new("pivotTables");
    pts.push_element(
        Element::new("pivotTable")
            .with_attr("tabId", tab_id.to_string())
            .with_attr("name", pivot_name),
    );
    let bounds = Element::new("bounds")
        .with_attr("startDate", start)
        .with_attr("endDate", end);
    let mut state = Element::new("state")
        .with_attr("minimalRefreshVersion", "6")
        .with_attr("lastRefreshVersion", "6")
        .with_attr("pivotCacheId", cache_id)
        .with_attr("filterType", "unknown");
    state.push_element(bounds);
    let mut root = Element::new("timelineCacheDefinition")
        .with_attr("xmlns", X15_NS)
        .with_attr("name", name)
        .with_attr("sourceName", field);
    root.push_element(pts);
    root.push_element(state);
    Document::new(Some(DEFAULT_DECL.to_string()), root)
        .to_bytes()
        .unwrap_or_default()
}

pub(crate) fn ensure_timelines_part(
    pkg: &mut Package,
    sheet_doc: &mut Document,
    sheet_part: &str,
) -> Result<(String, String, bool), Error> {
    let rels = Relationships::from_package(pkg, sheet_part)?;
    for rel in &rels.rels {
        if rel.rel_type.ends_with("/timeline") {
            let part = resolve_rel_target(sheet_part, &rel.target)?;
            return Ok((part, rel.id.clone(), false));
        }
    }
    let part = next_part(pkg, "/xl/timelines/timeline", "xml");
    let root = Element::new("timelines").with_attr("xmlns", X15_NS);
    pkg.set(
        &part,
        Document::new(Some(DEFAULT_DECL.to_string()), root).to_bytes()?,
    )?;
    let rid = add_rel(pkg, sheet_part, REL_TIMELINE, &part)?;
    picture::ensure_xmlns(&mut sheet_doc.root, "r", REL_NS);
    picture::ensure_xmlns(&mut sheet_doc.root, "mc", MC_NS);
    picture::ensure_xmlns(&mut sheet_doc.root, "x15", X15_NS);
    ensure_ignorable(&mut sheet_doc.root, &["x15"]);
    Ok((part, rid, true))
}

pub(crate) fn append_timeline_view(
    pkg: &mut Package,
    timeline_part: &str,
    name: &str,
    cache: &str,
    field: &str,
    level: u32,
    scroll: &str,
    view: &SlicerViewFields,
) -> Result<(), Error> {
    let caption = view.caption.as_deref().unwrap_or(field);
    let sel = view.selection_level.unwrap_or(level);
    let mut el = Element::new("timeline")
        .with_attr("name", name)
        .with_attr("cache", cache)
        .with_attr("caption", caption)
        .with_attr("level", level.to_string())
        .with_attr("selectionLevel", sel.to_string());
    match &view.scroll_position {
        None => el.set_attr("scrollPosition", scroll),
        Some(None) => {}
        Some(Some(s)) => el.set_attr("scrollPosition", s),
    }
    apply_view_attrs(&mut el, view, None, true);
    let mut doc = super::super::list_pin::parse_part(pkg, timeline_part)?;
    doc.root.push_element(el);
    super::super::list_pin::write_part(pkg, timeline_part, &doc)?;
    Ok(())
}

pub(crate) fn push_sheet_timeline_refs(sheet: &mut Element, rid: &str) {
    picture::ensure_xmlns(sheet, "r", REL_NS);
    picture::ensure_xmlns(sheet, "x15", X15_NS);
    picture::ensure_xmlns(sheet, "mc", MC_NS);
    ensure_ignorable(sheet, &["x15"]);
    if sheet.child_by_local("extLst").is_none() {
        layout::insert_child_ordered(sheet, Element::new("extLst"), "extLst");
    }
    let ext_lst = sheet.child_by_local_mut("extLst").unwrap();
    for n in ext_lst.children.iter_mut() {
        let Node::Element(ext) = n else { continue };
        if uri_eq(ext.attr("uri").unwrap_or(""), SHEET_TIMELINE_REFS_URI) {
            if let Some(list) = ext.child_by_local_mut("timelineRefs") {
                if list.element_children().any(|e| rid_attr(e) == Some(rid)) {
                    return;
                }
                list.push_element(Element::new("x15:timelineRef").with_attr("r:id", rid));
                return;
            }
        }
    }
    let mut list = Element::new("x15:timelineRefs");
    list.push_element(Element::new("x15:timelineRef").with_attr("r:id", rid));
    let mut ext = Element::new("ext")
        .with_attr("uri", SHEET_TIMELINE_REFS_URI)
        .with_attr("xmlns:x15", X15_NS);
    ext.push_element(list);
    ext_lst.push_element(ext);
}

pub(crate) fn push_wb_timeline_cache(wb: &mut Document, rid: &str) {
    picture::ensure_xmlns(&mut wb.root, "r", REL_NS);
    picture::ensure_xmlns(&mut wb.root, "mc", MC_NS);
    picture::ensure_xmlns(&mut wb.root, "x15", X15_NS);
    ensure_ignorable(&mut wb.root, &["x15"]);
    if wb.root.child_by_local("extLst").is_none() {
        names::insert_workbook_child(&mut wb.root, Element::new("extLst"), "extLst");
    }
    let ext_lst = wb.root.child_by_local_mut("extLst").unwrap();
    for n in ext_lst.children.iter_mut() {
        let Node::Element(ext) = n else { continue };
        if !uri_eq(ext.attr("uri").unwrap_or(""), WB_TIMELINE_CACHE_REFS_URI) {
            continue;
        }
        for list in ext.element_children_mut() {
            if list.local_name() == "timelineCacheRefs" {
                list.push_element(Element::new("x15:timelineCacheRef").with_attr("r:id", rid));
                return;
            }
        }
    }
    let mut list = Element::new("x15:timelineCacheRefs");
    list.push_element(Element::new("x15:timelineCacheRef").with_attr("r:id", rid));
    let mut ext = Element::new("ext")
        .with_attr("uri", WB_TIMELINE_CACHE_REFS_URI)
        .with_attr("xmlns:x15", X15_NS);
    ext.push_element(list);
    ext_lst.push_element(ext);
}

pub(crate) fn remove_timeline_view(
    pkg: &mut Package,
    sheet_doc: &mut Document,
    sheet_part: &str,
    name: &str,
) -> Result<(), Error> {
    let mut rels = Relationships::from_package(pkg, sheet_part)?;
    let mut drop_rid = None;
    let mut drop_part = None;
    for rel in &rels.rels {
        if !rel.rel_type.ends_with("/timeline") {
            continue;
        }
        let Ok(part) = resolve_rel_target(sheet_part, &rel.target) else {
            continue;
        };
        let Ok(bytes) = pkg.get(&part) else {
            continue;
        };
        let Ok(mut doc) = xml::parse(bytes) else {
            continue;
        };
        let before = doc.root.children_by_local("timeline").len();
        doc.root.children.retain(|n| match n {
            Node::Element(e) if e.local_name() == "timeline" => e.attr("name") != Some(name),
            _ => true,
        });
        if doc.root.children_by_local("timeline").len() == before {
            continue;
        }
        if doc.root.children_by_local("timeline").is_empty() {
            drop_rid = Some(rel.id.clone());
            drop_part = Some(part);
        } else {
            super::super::list_pin::write_part(pkg, &part, &doc)?;
        }
        break;
    }
    if let (Some(rid), Some(part)) = (drop_rid, drop_part) {
        rels.remove_by_id(&rid);
        rels.write_to(pkg, sheet_part)?;
        pkg.remove(&part);
        pkg.remove(&rels_part_name(&part));
        let mut cts = ContentTypes::from_package(pkg)?;
        cts.remove_override(&part);
        cts.write_to(pkg)?;
        drop_sheet_timeline_rid(&mut sheet_doc.root, &rid);
    }
    Ok(())
}

pub(crate) fn drop_sheet_timeline_rid(sheet: &mut Element, rid: &str) {
    let Some(ext_lst) = sheet.child_by_local_mut("extLst") else {
        return;
    };
    for n in ext_lst.children.iter_mut() {
        let Node::Element(ext) = n else { continue };
        if !uri_eq(ext.attr("uri").unwrap_or(""), SHEET_TIMELINE_REFS_URI) {
            continue;
        }
        if let Some(list) = ext.child_by_local_mut("timelineRefs") {
            list.children.retain(|n| match n {
                Node::Element(e) => rid_attr(e) != Some(rid),
                _ => true,
            });
        }
    }
    ext_lst.children.retain(|n| match n {
        Node::Element(ext) if uri_eq(ext.attr("uri").unwrap_or(""), SHEET_TIMELINE_REFS_URI) => ext
            .child_by_local("timelineRefs")
            .is_some_and(|l| l.element_children().next().is_some()),
        _ => true,
    });
    if ext_lst.element_children().next().is_none() {
        sheet
            .children
            .retain(|n| !matches!(n, Node::Element(e) if e.local_name() == "extLst"));
    }
}

pub(crate) fn cache_date_bounds(
    pkg: &Package,
    wb_part: &str,
    cache_id: &str,
    field: &str,
    op_index: usize,
) -> Result<(String, String), Error> {
    let Some(cf) = super::pivot::cache_field_el(pkg, wb_part, cache_id, field) else {
        return Err(field_err(field, op_index));
    };
    let mut dates = Vec::new();
    if let Some(shared) = cf.child_by_local("sharedItems") {
        if let Some(v) = shared.attr("minDate") {
            dates.push(normalize_iso(v));
        }
        if let Some(v) = shared.attr("maxDate") {
            dates.push(normalize_iso(v));
        }
        for child in shared.element_children() {
            match child.local_name() {
                "d" => {
                    if let Some(v) = child.attr("v") {
                        dates.push(normalize_iso(v));
                    }
                }
                "n" => {
                    if let Some(iso) = child
                        .attr("v")
                        .and_then(|s| s.parse::<f64>().ok())
                        .and_then(excel_serial_to_iso)
                    {
                        dates.push(iso);
                    }
                }
                _ => {}
            }
        }
    }
    dates.retain(|s| !s.is_empty());
    if dates.is_empty() {
        return Err(Error::with_path(
            ErrorKind::InvalidSlicer {
                field: "field".into(),
            },
            format!("ops[{op_index}].field"),
        ));
    }
    dates.sort();
    let start = dates.first().cloned().unwrap();
    let end = dates.last().cloned().unwrap();
    Ok((start, end))
}

pub(crate) fn mark_cache_field_dates(
    pkg: &mut Package,
    wb_part: &str,
    cache_id: &str,
    field: &str,
    start: &str,
    end: &str,
) -> Result<(), Error> {
    let part = super::pivot::pivot_cache_def_part(pkg, wb_part, cache_id)?;
    let mut doc = super::super::list_pin::parse_part(pkg, &part)?;
    let field_index = {
        let Some(fields) = doc.root.child_by_local("cacheFields") else {
            return Ok(());
        };
        match fields
            .children_by_local("cacheField")
            .iter()
            .position(|cf| cf.attr("name") == Some(field))
        {
            Some(i) => i,
            None => return Ok(()),
        }
    };
    let mut order = Vec::new();
    if let Some(fields) = doc.root.child_by_local_mut("cacheFields") {
        if let Some(cf) = fields.element_children_mut().nth(field_index) {
            cf.set_attr("numFmtId", "14");
            if let Some(shared) = cf.child_by_local_mut("sharedItems") {
                for child in shared.element_children() {
                    match child.local_name() {
                        "d" => {
                            if let Some(v) = child.attr("v") {
                                order.push(normalize_iso(v));
                            }
                        }
                        "n" => {
                            if let Some(iso) = child
                                .attr("v")
                                .and_then(|s| s.parse::<f64>().ok())
                                .and_then(excel_serial_to_iso)
                            {
                                order.push(iso);
                            }
                        }
                        _ => {}
                    }
                }
                if order.is_empty() {
                    order.push(start.to_string());
                    if end != start {
                        order.push(end.to_string());
                    }
                }
                shared.attrs.clear();
                shared.set_attr("containsSemiMixedTypes", "0");
                shared.set_attr("containsNonDate", "0");
                shared.set_attr("containsDate", "1");
                shared.set_attr("containsString", "0");
                shared.set_attr("minDate", start);
                shared.set_attr("maxDate", end);
                shared.set_attr("count", order.len().to_string());
                shared.children = order
                    .iter()
                    .map(|v| Node::Element(Element::new("d").with_attr("v", v.clone())))
                    .collect();
            }
        }
    }
    super::super::list_pin::write_part(pkg, &part, &doc)?;
    rewrite_cache_records_dates(pkg, &part, field_index, &order)?;
    Ok(())
}

pub(crate) fn rewrite_cache_records_dates(
    pkg: &mut Package,
    def_part: &str,
    field_index: usize,
    order: &[String],
) -> Result<(), Error> {
    let rels = Relationships::from_package(pkg, def_part)?;
    let Some(rel) = rels
        .rels
        .iter()
        .find(|r| r.rel_type.ends_with("/pivotCacheRecords"))
    else {
        return Ok(());
    };
    let rec_part = resolve_rel_target(def_part, &rel.target)?;
    let mut doc = super::super::list_pin::parse_part(pkg, &rec_part)?;
    for rec in doc.root.element_children_mut() {
        let mut i = 0usize;
        for n in rec.children.iter_mut() {
            let Node::Element(el) = n else { continue };
            if i == field_index && el.local_name() == "n" {
                if let Some(iso) = el
                    .attr("v")
                    .and_then(|s| s.parse::<f64>().ok())
                    .and_then(excel_serial_to_iso)
                {
                    if let Some(idx) = order.iter().position(|d| d == &iso) {
                        el.name = "x".into();
                        el.set_attr("v", idx.to_string());
                    } else {
                        el.name = "d".into();
                        el.set_attr("v", iso);
                    }
                }
            }
            i += 1;
        }
    }
    super::super::list_pin::write_part(pkg, &rec_part, &doc)?;
    Ok(())
}

pub(crate) fn normalize_iso(raw: &str) -> String {
    let t = raw.trim();
    if t.contains('T') {
        t.to_string()
    } else if t.len() >= 10 {
        format!("{}T00:00:00", &t[..10])
    } else {
        t.to_string()
    }
}

pub(crate) fn excel_serial_to_iso(serial: f64) -> Option<String> {
    if !serial.is_finite() || !(0.0..2_958_466.0).contains(&serial) {
        return None;
    }
    let day_f = serial.floor();
    let frac = serial - day_f;
    let mut day = day_f as i64;
    if day >= 60 {
        day -= 1;
    }
    let (y, m, d) = civil_from_days(days_from_civil(1899, 12, 31) + day);
    let secs = (frac * 86400.0).round() as i64;
    let h = ((secs / 3600) % 24) as u32;
    let mi = ((secs / 60) % 60) as u32;
    let s = (secs % 60) as u32;
    Some(format!("{y:04}-{m:02}-{d:02}T{h:02}:{mi:02}:{s:02}"))
}

pub(crate) fn days_from_civil(y: i32, m: u32, d: u32) -> i64 {
    let mut y = y;
    let m = m as i32;
    if m <= 2 {
        y -= 1;
    }
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u32;
    let mp = if m > 2 {
        (m - 3) as u32
    } else {
        (m + 9) as u32
    };
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era as i64 * 146097 + doe as i64 - 719468
}

pub(crate) fn civil_from_days(z: i64) -> (i32, u32, u32) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i32 + era as i32 * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

pub(crate) fn timeline_filter_json(pkg: &Package, cache: Option<&CacheMeta>) -> Value {
    let Some(cache) = cache else {
        return Value::Null;
    };
    let Ok(bytes) = pkg.get(&cache.part) else {
        return Value::Null;
    };
    let Ok(doc) = xml::parse(bytes) else {
        return Value::Null;
    };
    let Some(state) = doc.root.child_by_local("state") else {
        return Value::Null;
    };
    let filter_type = state.attr("filterType").unwrap_or("").to_string();
    if !filter_type.starts_with("date") {
        return Value::Null;
    }
    let (start, end) = if let Some(sel) = state.child_by_local("selection") {
        (
            sel.attr("startDate").unwrap_or("").to_string(),
            sel.attr("endDate").unwrap_or("").to_string(),
        )
    } else {
        custom_filter_dates(&doc.root)
    };
    json!({
        "filterType": filter_type,
        "startDate": start,
        "endDate": end,
    })
}

pub(crate) fn custom_filter_dates(root: &Element) -> (String, String) {
    let mut vals = Vec::new();
    picture::walk_all(root, &mut |e| {
        if e.local_name() == "customFilter" {
            if let Some(v) = e.attr("val") {
                vals.push(v.to_string());
            }
        }
    });
    match vals.as_slice() {
        [] => (String::new(), String::new()),
        [a] => (a.clone(), a.clone()),
        [a, b, ..] => (a.clone(), b.clone()),
    }
}

pub(crate) fn set_timeline_filter(
    pkg: &mut Package,
    sheet_part: &str,
    view_name: &str,
    filter: Option<&TimelineFilter>,
    op_index: usize,
) -> Result<(), Error> {
    let views = load_timeline_views(pkg, sheet_part)?;
    let Some(view) = views.iter().find(|v| v.name == view_name) else {
        return Err(Error::with_path(
            ErrorKind::SlicerNotFound {
                id: format!("slicer:{view_name}"),
            },
            format!("ops[{op_index}].id"),
        ));
    };
    let caches = load_caches(pkg)?;
    let Some(cache) = caches.iter().find(|c| c.name == view.cache) else {
        return Err(Error::with_path(
            ErrorKind::SlicerNotFound {
                id: format!("slicer:{view_name}"),
            },
            format!("ops[{op_index}].id"),
        ));
    };
    let mut doc = super::super::list_pin::parse_part(pkg, &cache.part)?;
    match filter {
        None => clear_timeline_filter(&mut doc.root),
        Some(f) => write_timeline_filter(&mut doc.root, f),
    }
    super::super::list_pin::write_part(pkg, &cache.part, &doc)?;
    Ok(())
}

pub(crate) fn clear_timeline_filter(root: &mut Element) {
    if let Some(state) = root.child_by_local_mut("state") {
        state.set_attr("filterType", "unknown");
        state.children.retain(|n| match n {
            Node::Element(e) => e.local_name() != "selection",
            _ => true,
        });
    }
    root.children.retain(|n| match n {
        Node::Element(e) => e.local_name() != "timelinePivotFilter",
        _ => true,
    });
}

pub(crate) fn write_timeline_filter(root: &mut Element, filter: &TimelineFilter) {
    if let Some(state) = root.child_by_local_mut("state") {
        state.set_attr("filterType", &filter.filter_type);
        let sel_at = state
            .children
            .iter()
            .position(|n| matches!(n, Node::Element(e) if e.local_name() == "selection"));
        if let Some(i) = sel_at {
            if let Node::Element(sel) = &mut state.children[i] {
                sel.set_attr("startDate", &filter.start_date);
                sel.set_attr("endDate", &filter.end_date);
            }
        } else {
            let after = state
                .children
                .iter()
                .position(|n| matches!(n, Node::Element(e) if e.local_name() == "bounds"));
            let sel = Element::new("selection")
                .with_attr("startDate", filter.start_date.clone())
                .with_attr("endDate", filter.end_date.clone());
            match after {
                Some(i) => state.children.insert(i, Node::Element(sel)),
                None => state.push_element(sel),
            }
        }
    }
    // Excel 16.0 另存日期筛选不写 timelinePivotFilter；写了会打不开。
    root.children.retain(|n| match n {
        Node::Element(e) => e.local_name() != "timelinePivotFilter",
        _ => true,
    });
}

#[cfg(test)]
mod excel_date_tests {
    use super::excel_serial_to_iso;

    #[test]
    fn serial_44927_is_2023_01_01() {
        assert_eq!(
            excel_serial_to_iso(44927.0).as_deref(),
            Some("2023-01-01T00:00:00")
        );
        assert_eq!(
            excel_serial_to_iso(44986.0).as_deref(),
            Some("2023-03-01T00:00:00")
        );
        assert_eq!(
            excel_serial_to_iso(1.0).as_deref(),
            Some("1900-01-01T00:00:00")
        );
    }
}
