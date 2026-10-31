use serde_json::{json, Map, Value};

use crate::error::{Error, ErrorKind, IdReason};
use crate::opc::{
    office_document_part, rels_part_name, resolve_id, resolve_rel_target, ContentTypes, Package,
    Relationship, Relationships,
};
use crate::xml::{self, Document, Element, Node, DEFAULT_DECL};

use super::super::a1;
use super::super::layout;
use super::super::names;
use super::super::picture::{self, AnchorKind, EditAs};
use super::super::shape;
use super::super::REL_NS;
use super::{InsertSlicerOp, SlicerKey, SlicerViewFields};

pub(crate) const MC_NS: &str = "http://schemas.openxmlformats.org/markup-compatibility/2006";
pub(crate) const X14_NS: &str = "http://schemas.microsoft.com/office/spreadsheetml/2009/9/main";
pub(crate) const X15_NS: &str = "http://schemas.microsoft.com/office/spreadsheetml/2010/11/main";
pub(crate) const SLE_NS: &str = "http://schemas.microsoft.com/office/drawing/2010/slicer";
pub(crate) const SLE15_NS: &str = "http://schemas.microsoft.com/office/drawing/2012/slicer";
pub(crate) const TSLE_NS: &str = "http://schemas.microsoft.com/office/drawing/2012/timeslicer";
pub(crate) const A14_NS: &str = "http://schemas.microsoft.com/office/drawing/2010/main";
pub(crate) const SML_NS: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";

pub(crate) const REL_SLICER: &str = "http://schemas.microsoft.com/office/2007/relationships/slicer";
pub(crate) const REL_SLICER_CACHE: &str =
    "http://schemas.microsoft.com/office/2007/relationships/slicerCache";
pub(crate) const REL_TIMELINE: &str =
    "http://schemas.microsoft.com/office/2011/relationships/timeline";
pub(crate) const REL_TIMELINE_CACHE: &str =
    "http://schemas.microsoft.com/office/2011/relationships/timelineCache";
pub(crate) const CT_SLICER: &str = "application/vnd.ms-excel.slicer+xml";
pub(crate) const CT_SLICER_CACHE: &str = "application/vnd.ms-excel.slicerCache+xml";
pub(crate) const CT_TIMELINE: &str = "application/vnd.ms-excel.timeline+xml";
pub(crate) const CT_TIMELINE_CACHE: &str = "application/vnd.ms-excel.timelineCache+xml";

pub(crate) const SHEET_SLICER_LIST_URI: &str = "{3A4CF648-6AED-40f4-86FF-DC5316D8AED3}";
pub(crate) const WB_X15_SLICER_CACHES_URI: &str = "{46BE6895-7355-4a93-B00E-2C351335B9C9}";
pub(crate) const WB_X14_SLICER_CACHES_URI: &str = "{BBE1A952-AA13-448e-AADC-164F8A28A991}";
pub(crate) const TABLE_SLICER_CACHE_URI: &str = "{2F2917AC-EB37-4324-AD4E-5DD8C200BD13}";
pub(crate) const PIVOT_CACHE_DEF_EXT_URI: &str = "{725AE2AE-9491-48be-B2B4-4EB974FC3084}";
pub(crate) const SHEET_TIMELINE_REFS_URI: &str = "{7E03D99C-DC04-49d9-9315-930204A7B6E9}";
pub(crate) const WB_TIMELINE_CACHE_REFS_URI: &str = "{D0CA8CA8-9F24-4464-BF8E-62219DCF47F9}";

pub(crate) const DEFAULT_CX: i64 = 1_905_000;
pub(crate) const DEFAULT_CY: i64 = 2_540_000;
pub(crate) const DEFAULT_TIMELINE_CX: i64 = 5_080_000;
pub(crate) const DEFAULT_TIMELINE_CY: i64 = 1_016_000;
pub(crate) const DEFAULT_ROW_HEIGHT: i64 = 381_000;
pub(crate) const DEFAULT_COLUMN_COUNT: u32 = 1;
pub(crate) const DEFAULT_TIMELINE_LEVEL: u32 = 2;

#[derive(Clone)]
pub(crate) struct Hit {
    pub(crate) cnv: Option<String>,
    pub(crate) cnv_name: String,
    pub(crate) sle_name: String,
    pub(crate) kind: AnchorKind,
    pub(crate) timeline: bool,
    pub(crate) anchor: Element,
}

pub(crate) fn draw_key(key: &SlicerKey) -> shape::DrawKey {
    match key {
        SlicerKey::Id(s) => shape::DrawKey::Cnv(s.clone()),
        SlicerKey::Index(n) => shape::DrawKey::Index(*n),
    }
}

pub(crate) fn write_drawing(
    pkg: &mut Package,
    sheet_doc: &mut Document,
    sheet_part: &str,
    op: &InsertSlicerOp,
    view_name: &str,
    frame: SlicerFrameKind,
    cx: i64,
    cy: i64,
    to: a1::CellRef,
    to_off_x: i64,
    to_off_y: i64,
) -> Result<(), Error> {
    let drawing_part = picture::ensure_drawing_part(pkg, sheet_doc, sheet_part)?;
    let mut draw_doc = super::super::list_pin::parse_part(pkg, &drawing_part)?;
    picture::ensure_xmlns(&mut draw_doc.root, "r", REL_NS);
    picture::ensure_xmlns(&mut draw_doc.root, "a", picture::A_NS);
    picture::ensure_xmlns(&mut draw_doc.root, "mc", MC_NS);
    match frame {
        SlicerFrameKind::Table | SlicerFrameKind::Pivot => {
            picture::ensure_xmlns(&mut draw_doc.root, "sle", SLE_NS);
        }
        SlicerFrameKind::Timeline => {
            picture::ensure_xmlns(&mut draw_doc.root, "tsle", TSLE_NS);
        }
    }
    let cnv = picture::next_cnv_id(pkg, sheet_part, &sheet_doc.root)?;
    let ac = build_slicer_ac(
        frame,
        cnv,
        view_name,
        op.from,
        op.from_off_x,
        op.from_off_y,
        to,
        to_off_x,
        to_off_y,
        cx,
        cy,
        if op.anchor == AnchorKind::TwoCell {
            EditAs::TwoCell
        } else {
            EditAs::OneCell
        },
    );
    draw_doc.root.push_element(ac);
    super::super::list_pin::write_part(pkg, &drawing_part, &draw_doc)?;
    Ok(())
}

pub(crate) fn dest_to(
    sheet: &Element,
    op: &InsertSlicerOp,
    cx: i64,
    cy: i64,
) -> (a1::CellRef, i64, i64) {
    if op.anchor == AnchorKind::TwoCell {
        return (op.to.unwrap_or(op.from), op.to_off_x, op.to_off_y);
    }
    let x =
        layout::col_origin_emu(sheet, op.from.col).saturating_add(op.from_off_x.saturating_add(cx));
    let y =
        layout::row_origin_emu(sheet, op.from.row).saturating_add(op.from_off_y.saturating_add(cy));
    let (cell, ox, oy) = layout::emu_to_marker(sheet, x, y);
    (cell, ox, oy)
}

pub(crate) fn collect_hits(pkg: &Package, sheet_part: &str) -> Result<Vec<Hit>, Error> {
    let views = load_all_views(pkg, sheet_part)?;
    let caches = load_caches(pkg)?;
    let names: Vec<&str> = views
        .iter()
        .filter(|v| caches.iter().any(|c| c.name == v.cache))
        .map(|v| v.name.as_str())
        .collect();
    let sheet = super::super::list_pin::parse_part(pkg, sheet_part)?;
    let mut out = Vec::new();
    for drawing_part in picture::drawing_parts_from(pkg, sheet_part, &sheet.root)? {
        let doc = super::super::list_pin::parse_part(pkg, &drawing_part)?;
        picture::walk_anchors(&doc.root, &mut |anchor, kind| {
            shape::visit_payloads(anchor, &mut |el| {
                if super::is_slicer_or_timeline_frame(el) {
                    if let Some(sle) = sle_name(el) {
                        if names.iter().any(|n| *n == sle) {
                            let (cnv, cnv_name) = shape::cnv_of_object(el);
                            out.push(Hit {
                                cnv,
                                cnv_name,
                                sle_name: sle,
                                kind,
                                timeline: frame_is_timeline(el),
                                anchor: anchor.clone(),
                            });
                        }
                    }
                }
                Ok(())
            })
        })?;
    }
    Ok(out)
}

pub(crate) fn hit_json(
    pkg: &Package,
    sheet_part: &str,
    sheet_name: &str,
    hit: &Hit,
    index: usize,
    views: &[SlicerView],
    caches: &[CacheMeta],
) -> Result<Value, Error> {
    let _ = sheet_part;
    let stable = hit.cnv.is_some();
    let id = match &hit.cnv {
        Some(v) => format!("sheet:{sheet_name}/slicer:{v}"),
        None => format!("sheet:{sheet_name}/slicer:index:{index}"),
    };
    let view = views.iter().find(|v| v.name == hit.sle_name);
    let name = view
        .map(|v| v.name.clone())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| {
            if hit.cnv_name.is_empty() {
                String::new()
            } else {
                hit.cnv_name.clone()
            }
        });
    let caption = view.and_then(|v| v.caption.clone());
    let cache_name = view.map(|v| v.cache.as_str()).unwrap_or("");
    let cache = caches.iter().find(|c| c.name == cache_name);
    let (source_kind, source_name, field) = match cache {
        Some(c) => (Some(c.kind), c.source_name.clone(), Some(c.field.clone())),
        None => (None, None, None),
    };
    let mut m = shape::geom_map(&hit.anchor, hit.kind, id, hit.cnv.clone(), name, stable)?;
    let stable_v = m.remove("stable").unwrap_or(json!(stable));
    let mut out = Map::new();
    for (k, v) in m {
        if k == "anchor" {
            out.insert("caption".into(), json!(caption));
            insert_view_fields(&mut out, view, hit.timeline);
            insert_filter_fields(&mut out, pkg, cache, hit.timeline);
            out.insert(
                "sourceKind".into(),
                match source_kind {
                    Some(k) => json!(k),
                    None => Value::Null,
                },
            );
            out.insert("sourceName".into(), json!(source_name));
            out.insert("field".into(), json!(field));
        }
        out.insert(k, v);
    }
    out.insert("stable".into(), stable_v);
    Ok(Value::Object(out))
}

pub(crate) fn insert_view_fields(
    out: &mut Map<String, Value>,
    view: Option<&SlicerView>,
    timeline: bool,
) {
    if timeline {
        out.insert(
            "showHeader".into(),
            json!(view.and_then(|v| v.show_header).unwrap_or(true)),
        );
        out.insert(
            "showSelectionLabel".into(),
            json!(view.and_then(|v| v.show_selection_label).unwrap_or(true)),
        );
        out.insert(
            "showTimeLevel".into(),
            json!(view.and_then(|v| v.show_time_level).unwrap_or(true)),
        );
        out.insert(
            "showHorizontalScrollbar".into(),
            json!(view
                .and_then(|v| v.show_horizontal_scrollbar)
                .unwrap_or(true)),
        );
        out.insert(
            "level".into(),
            json!(view.and_then(|v| v.level).unwrap_or(DEFAULT_TIMELINE_LEVEL)),
        );
        out.insert(
            "selectionLevel".into(),
            json!(view
                .and_then(|v| v.selection_level)
                .unwrap_or(DEFAULT_TIMELINE_LEVEL)),
        );
        out.insert(
            "scrollPosition".into(),
            json!(view.and_then(|v| v.scroll_position.clone())),
        );
        out.insert("style".into(), json!(view.and_then(|v| v.style.clone())));
        return;
    }
    out.insert(
        "rowHeight".into(),
        json!(view
            .and_then(|v| v.row_height)
            .unwrap_or(DEFAULT_ROW_HEIGHT)),
    );
    out.insert(
        "columnCount".into(),
        json!(view
            .and_then(|v| v.column_count)
            .unwrap_or(DEFAULT_COLUMN_COUNT)),
    );
    out.insert(
        "showCaption".into(),
        json!(view.and_then(|v| v.show_caption).unwrap_or(true)),
    );
    out.insert(
        "startItem".into(),
        json!(view.and_then(|v| v.start_item).unwrap_or(0)),
    );
    out.insert(
        "lockedPosition".into(),
        json!(view.and_then(|v| v.locked_position).unwrap_or(false)),
    );
    out.insert("style".into(), json!(view.and_then(|v| v.style.clone())));
}

pub(crate) fn insert_filter_fields(
    out: &mut Map<String, Value>,
    pkg: &Package,
    cache: Option<&CacheMeta>,
    timeline: bool,
) {
    if timeline {
        out.insert(
            "filter".into(),
            super::timeline::timeline_filter_json(pkg, cache),
        );
        return;
    }
    out.insert("selected".into(), json!(slicer_selected(pkg, cache)));
}

pub(crate) fn is_slicer_uri(uri: &str) -> bool {
    let t = uri.trim_end_matches('/');
    t == SLE_NS
}

pub(crate) fn is_timeslicer_uri(uri: &str) -> bool {
    let t = uri.trim_end_matches('/');
    t == TSLE_NS
}

pub(crate) fn graphic_uri(el: &Element) -> Option<String> {
    let mut found = None;
    picture::walk_all(el, &mut |e| {
        if e.local_name() == "graphicData" && found.is_none() {
            found = e.attr_local("uri").map(str::to_string);
        }
    });
    found
}

pub(crate) fn frame_is_timeline(el: &Element) -> bool {
    graphic_uri(el).is_some_and(|u| is_timeslicer_uri(&u))
}

pub(crate) fn sle_name(el: &Element) -> Option<String> {
    let mut found = None;
    picture::walk_all(el, &mut |e| {
        if found.is_none() && (e.local_name() == "slicer" || e.local_name() == "timeslicer") {
            found = e.attr("name").filter(|s| !s.is_empty()).map(str::to_string);
        }
    });
    found
}

pub(crate) fn key_matches(key: &SlicerKey, hit: &Hit, index: usize) -> bool {
    match key {
        SlicerKey::Id(id) => hit.cnv.as_deref() == Some(id.as_str()),
        SlicerKey::Index(k) => {
            let frame = hit
                .anchor
                .element_children()
                .find(|el| super::is_slicer_or_timeline_frame(el));
            match frame {
                Some(frame) => super::super::list_pin::index_matches(frame, *k, index),
                None => *k == index,
            }
        }
    }
}

pub(crate) struct SlicerView {
    pub(crate) name: String,
    pub(crate) cache: String,
    pub(crate) caption: Option<String>,
    pub(crate) row_height: Option<i64>,
    pub(crate) column_count: Option<u32>,
    pub(crate) show_caption: Option<bool>,
    pub(crate) start_item: Option<u32>,
    pub(crate) locked_position: Option<bool>,
    pub(crate) style: Option<String>,
    pub(crate) show_header: Option<bool>,
    pub(crate) show_selection_label: Option<bool>,
    pub(crate) show_time_level: Option<bool>,
    pub(crate) show_horizontal_scrollbar: Option<bool>,
    pub(crate) level: Option<u32>,
    pub(crate) selection_level: Option<u32>,
    pub(crate) scroll_position: Option<String>,
}

pub(crate) fn view_from_slicer_el(el: &Element) -> Option<SlicerView> {
    let name = el.attr("name").unwrap_or("").to_string();
    if name.is_empty() {
        return None;
    }
    Some(SlicerView {
        name,
        cache: el.attr("cache").unwrap_or("").to_string(),
        caption: el.attr("caption").map(str::to_string),
        row_height: parse_attr_i64(el, "rowHeight"),
        column_count: parse_attr_u32(el, "columnCount"),
        show_caption: parse_attr_bool(el, "showCaption"),
        start_item: parse_attr_u32(el, "startItem"),
        locked_position: parse_attr_bool(el, "lockedPosition"),
        style: el.attr("style").map(str::to_string),
        show_header: None,
        show_selection_label: None,
        show_time_level: None,
        show_horizontal_scrollbar: None,
        level: None,
        selection_level: None,
        scroll_position: None,
    })
}

pub(crate) fn view_from_timeline_el(el: &Element) -> Option<SlicerView> {
    let name = el.attr("name").unwrap_or("").to_string();
    if name.is_empty() {
        return None;
    }
    Some(SlicerView {
        name,
        cache: el.attr("cache").unwrap_or("").to_string(),
        caption: el.attr("caption").map(str::to_string),
        row_height: None,
        column_count: None,
        show_caption: None,
        start_item: None,
        locked_position: None,
        style: el.attr("style").map(str::to_string),
        show_header: parse_attr_bool(el, "showHeader"),
        show_selection_label: parse_attr_bool(el, "showSelectionLabel"),
        show_time_level: parse_attr_bool(el, "showTimeLevel"),
        show_horizontal_scrollbar: parse_attr_bool(el, "showHorizontalScrollbar"),
        level: parse_attr_u32(el, "level"),
        selection_level: parse_attr_u32(el, "selectionLevel"),
        scroll_position: el.attr("scrollPosition").map(str::to_string),
    })
}

pub(crate) fn parse_attr_u32(el: &Element, name: &str) -> Option<u32> {
    el.attr(name)?.parse().ok()
}

pub(crate) fn parse_attr_i64(el: &Element, name: &str) -> Option<i64> {
    el.attr(name)?.parse().ok()
}

pub(crate) fn parse_attr_bool(el: &Element, name: &str) -> Option<bool> {
    match el.attr(name)? {
        "1" | "true" | "TRUE" => Some(true),
        "0" | "false" | "FALSE" => Some(false),
        _ => None,
    }
}

pub(crate) fn xml_bool(v: bool) -> &'static str {
    if v {
        "1"
    } else {
        "0"
    }
}

pub(crate) fn set_opt_bool(el: &mut Element, name: &str, v: Option<bool>) {
    if let Some(b) = v {
        el.set_attr(name, xml_bool(b));
    }
}

pub(crate) fn set_opt_u32(el: &mut Element, name: &str, v: Option<u32>) {
    if let Some(n) = v {
        el.set_attr(name, n.to_string());
    }
}

pub(crate) fn set_opt_style(el: &mut Element, v: &Option<Option<String>>) {
    match v {
        None => {}
        Some(None) => el.remove_attr("style"),
        Some(Some(s)) => el.set_attr("style", s),
    }
}

pub(crate) fn set_opt_scroll(el: &mut Element, v: &Option<Option<String>>) {
    match v {
        None => {}
        Some(None) => el.remove_attr("scrollPosition"),
        Some(Some(s)) => el.set_attr("scrollPosition", s),
    }
}

pub(crate) fn apply_view_attrs(
    el: &mut Element,
    view: &SlicerViewFields,
    level: Option<u32>,
    timeline: bool,
) {
    if let Some(ref c) = view.caption {
        el.set_attr("caption", c);
    }
    set_opt_style(el, &view.style);
    if timeline {
        set_opt_bool(el, "showHeader", view.show_header);
        set_opt_bool(el, "showSelectionLabel", view.show_selection_label);
        set_opt_bool(el, "showTimeLevel", view.show_time_level);
        set_opt_bool(
            el,
            "showHorizontalScrollbar",
            view.show_horizontal_scrollbar,
        );
        set_opt_u32(el, "level", level);
        set_opt_u32(el, "selectionLevel", view.selection_level);
        set_opt_scroll(el, &view.scroll_position);
        return;
    }
    if let Some(h) = view.row_height {
        el.set_attr("rowHeight", h.to_string());
    }
    set_opt_u32(el, "columnCount", view.column_count);
    set_opt_bool(el, "showCaption", view.show_caption);
    set_opt_u32(el, "startItem", view.start_item);
    set_opt_bool(el, "lockedPosition", view.locked_position);
}

pub(crate) fn patch_named_view(
    pkg: &mut Package,
    sheet_part: &str,
    timeline: bool,
    name: &str,
    op_id: &str,
    op_index: usize,
    patch: impl FnOnce(&mut Element) -> Result<(), Error>,
) -> Result<(), Error> {
    let rel_suffix = if timeline { "/timeline" } else { "/slicer" };
    let local = if timeline { "timeline" } else { "slicer" };
    let rels = Relationships::from_package(pkg, sheet_part)?;
    let mut found: Option<(String, Document)> = None;
    for rel in &rels.rels {
        if !rel.rel_type.ends_with(rel_suffix) {
            continue;
        }
        let part = resolve_rel_target(sheet_part, &rel.target)?;
        let doc = super::super::list_pin::parse_part(pkg, &part)?;
        let hit = doc
            .root
            .element_children()
            .any(|el| el.local_name() == local && el.attr("name") == Some(name));
        if hit {
            found = Some((part, doc));
            break;
        }
    }
    let Some((part, mut doc)) = found else {
        return Err(Error::with_path(
            ErrorKind::SlicerNotFound {
                id: op_id.to_string(),
            },
            format!("ops[{op_index}].id"),
        ));
    };
    let Some(el) = doc
        .root
        .element_children_mut()
        .find(|el| el.local_name() == local && el.attr("name") == Some(name))
    else {
        return Err(Error::with_path(
            ErrorKind::SlicerNotFound {
                id: op_id.to_string(),
            },
            format!("ops[{op_index}].id"),
        ));
    };
    patch(el)?;
    super::super::list_pin::write_part(pkg, &part, &doc)?;
    Ok(())
}

pub(crate) fn load_slicer_views(pkg: &Package, sheet_part: &str) -> Result<Vec<SlicerView>, Error> {
    let rels = Relationships::from_package(pkg, sheet_part)?;
    let mut out = Vec::new();
    for rel in &rels.rels {
        if !rel.rel_type.ends_with("/slicer") {
            continue;
        }
        let Ok(part) = resolve_rel_target(sheet_part, &rel.target) else {
            continue;
        };
        let Ok(bytes) = pkg.get(&part) else {
            continue;
        };
        let Ok(doc) = xml::parse(bytes) else {
            continue;
        };
        for el in doc.root.children_by_local("slicer") {
            if let Some(v) = view_from_slicer_el(el) {
                out.push(v);
            }
        }
    }
    Ok(out)
}

pub(crate) fn load_timeline_views(
    pkg: &Package,
    sheet_part: &str,
) -> Result<Vec<SlicerView>, Error> {
    let rels = Relationships::from_package(pkg, sheet_part)?;
    let mut out = Vec::new();
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
        let Ok(doc) = xml::parse(bytes) else {
            continue;
        };
        for el in doc.root.children_by_local("timeline") {
            if let Some(v) = view_from_timeline_el(el) {
                out.push(v);
            }
        }
    }
    Ok(out)
}

pub(crate) fn load_all_views(pkg: &Package, sheet_part: &str) -> Result<Vec<SlicerView>, Error> {
    let mut out = load_slicer_views(pkg, sheet_part)?;
    out.extend(load_timeline_views(pkg, sheet_part)?);
    Ok(out)
}

pub(crate) struct CacheMeta {
    pub(crate) name: String,
    pub(crate) field: String,
    pub(crate) kind: &'static str,
    pub(crate) source_name: Option<String>,
    pub(crate) part: String,
}

pub(crate) fn load_caches(pkg: &Package) -> Result<Vec<CacheMeta>, Error> {
    let wb_part = office_document_part(pkg)?;
    let rels = Relationships::from_package(pkg, &wb_part)?;
    let mut out = Vec::new();
    for rel in &rels.rels {
        if !rel.rel_type.ends_with("/slicerCache") {
            continue;
        }
        let Ok(part) = resolve_rel_target(&wb_part, &rel.target) else {
            continue;
        };
        let Ok(bytes) = pkg.get(&part) else {
            continue;
        };
        let Ok(doc) = xml::parse(bytes) else {
            continue;
        };
        let name = doc.root.attr("name").unwrap_or("").to_string();
        if name.is_empty() {
            continue;
        }
        let field = doc.root.attr("sourceName").unwrap_or("").to_string();
        let table = has_local(&doc.root, "tableSlicerCache");
        let pivot = has_local(&doc.root, "tabular") || has_local(&doc.root, "pivotTables");
        let olap = has_local(&doc.root, "olap");
        if olap && !table && !pivot {
            continue;
        }
        let kind = if table {
            "table"
        } else if pivot {
            "pivot"
        } else {
            continue;
        };
        let source_name = if table {
            super::table::table_display_by_id(pkg, &wb_part, super::table::table_id_of(&doc.root))
        } else {
            super::pivot::pivot_name_of(&doc.root)
        };
        out.push(CacheMeta {
            name,
            field,
            kind,
            source_name,
            part,
        });
    }
    for rel in &rels.rels {
        if !rel.rel_type.ends_with("/timelineCache") {
            continue;
        }
        let Ok(part) = resolve_rel_target(&wb_part, &rel.target) else {
            continue;
        };
        let Ok(bytes) = pkg.get(&part) else {
            continue;
        };
        let Ok(doc) = xml::parse(bytes) else {
            continue;
        };
        let name = doc.root.attr("name").unwrap_or("").to_string();
        if name.is_empty() {
            continue;
        }
        if !has_local(&doc.root, "pivotTables") {
            continue;
        }
        out.push(CacheMeta {
            field: doc.root.attr("sourceName").unwrap_or("").to_string(),
            name,
            kind: "timeline",
            source_name: super::pivot::pivot_name_of(&doc.root),
            part,
        });
    }
    Ok(out)
}

pub(crate) fn has_local(root: &Element, local: &str) -> bool {
    let mut hit = false;
    picture::walk_all(root, &mut |e| {
        if e.local_name() == local {
            hit = true;
        }
    });
    hit
}

pub(crate) fn sheet_id(wb: &Document, name: &str) -> Option<u32> {
    let sheets = wb.root.child_by_local("sheets")?;
    for sh in sheets.children_by_local("sheet") {
        if sh
            .attr("name")
            .is_some_and(|n| n.eq_ignore_ascii_case(name))
        {
            return sh.attr("sheetId").and_then(|s| s.parse().ok());
        }
    }
    None
}

pub(crate) fn unique_cache_name(pkg: &Package, wb: &Document, stem: &str) -> String {
    let mut taken = defined_names(wb);
    if let Ok(caches) = load_caches(pkg) {
        for c in caches {
            taken.push(c.name);
        }
    }
    unique_name(stem, &taken)
}

pub(crate) fn unique_view_name(pkg: &Package, field: &str) -> String {
    let mut taken = Vec::new();
    if let Ok(wb_part) = office_document_part(pkg) {
        if let Ok(bytes) = pkg.get(&wb_part) {
            if let Ok(wb) = xml::parse(bytes) {
                if let Some(sheets) = wb.root.child_by_local("sheets") {
                    for sh in sheets.children_by_local("sheet") {
                        if let Some(rid) = rid_attr(sh) {
                            if let Ok(part) = resolve_id(pkg, &wb_part, rid) {
                                if let Ok(views) = load_all_views(pkg, &part) {
                                    for v in views {
                                        taken.push(v.name);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    unique_name(field, &taken)
}

pub(crate) fn defined_names(wb: &Document) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(p) = wb.root.child_by_local("definedNames") {
        for el in p.children_by_local("definedName") {
            if let Some(n) = el.attr("name") {
                out.push(n.to_string());
            }
        }
    }
    out
}

pub(crate) fn unique_name(base: &str, taken: &[String]) -> String {
    let stem = base.replace(' ', "_");
    let mut n = 1u32;
    loop {
        let cand = if n == 1 {
            stem.clone()
        } else {
            format!("{stem}_{n}")
        };
        if !taken.iter().any(|t| t.eq_ignore_ascii_case(&cand)) {
            return cand;
        }
        n += 1;
    }
}

pub(crate) fn ensure_slicers_part(
    pkg: &mut Package,
    sheet_doc: &mut Document,
    sheet_part: &str,
) -> Result<(String, String, bool), Error> {
    let rels = Relationships::from_package(pkg, sheet_part)?;
    for rel in &rels.rels {
        if rel.rel_type.ends_with("/slicer") {
            let part = resolve_rel_target(sheet_part, &rel.target)?;
            return Ok((part, rel.id.clone(), false));
        }
    }
    let part = next_part(pkg, "/xl/slicers/slicer", "xml");
    let root = Element::new("slicers").with_attr("xmlns", X14_NS);
    pkg.set(
        &part,
        Document::new(Some(DEFAULT_DECL.to_string()), root).to_bytes()?,
    )?;
    let rid = add_rel(pkg, sheet_part, REL_SLICER, &part)?;
    picture::ensure_xmlns(&mut sheet_doc.root, "r", REL_NS);
    picture::ensure_xmlns(&mut sheet_doc.root, "mc", MC_NS);
    picture::ensure_xmlns(&mut sheet_doc.root, "x14", X14_NS);
    picture::ensure_xmlns(&mut sheet_doc.root, "x15", X15_NS);
    ensure_ignorable(&mut sheet_doc.root, &["x14", "x15"]);
    Ok((part, rid, true))
}

pub(crate) fn append_slicer_view(
    pkg: &mut Package,
    slicer_part: &str,
    name: &str,
    cache: &str,
    field: &str,
    view: &SlicerViewFields,
) -> Result<(), Error> {
    let caption = view.caption.as_deref().unwrap_or(field);
    let row_h = view.row_height.unwrap_or(DEFAULT_ROW_HEIGHT);
    let mut el = Element::new("slicer")
        .with_attr("name", name)
        .with_attr("cache", cache)
        .with_attr("caption", caption)
        .with_attr("rowHeight", row_h.to_string());
    apply_view_attrs(&mut el, view, None, false);
    let mut doc = super::super::list_pin::parse_part(pkg, slicer_part)?;
    doc.root.push_element(el);
    super::super::list_pin::write_part(pkg, slicer_part, &doc)?;
    Ok(())
}

pub(crate) fn push_sheet_list(sheet: &mut Element, rid: &str) {
    picture::ensure_xmlns(sheet, "r", REL_NS);
    picture::ensure_xmlns(sheet, "x14", X14_NS);
    picture::ensure_xmlns(sheet, "x15", X15_NS);
    picture::ensure_xmlns(sheet, "mc", MC_NS);
    ensure_ignorable(sheet, &["x14", "x15"]);
    if sheet.child_by_local("extLst").is_none() {
        layout::insert_child_ordered(sheet, Element::new("extLst"), "extLst");
    }
    let ext_lst = sheet.child_by_local_mut("extLst").unwrap();
    for n in ext_lst.children.iter_mut() {
        let Node::Element(ext) = n else { continue };
        if uri_eq(ext.attr("uri").unwrap_or(""), SHEET_SLICER_LIST_URI) {
            if let Some(list) = ext.child_by_local_mut("slicerList") {
                if list.element_children().any(|e| rid_attr(e) == Some(rid)) {
                    return;
                }
                list.push_element(Element::new("x14:slicer").with_attr("r:id", rid));
                return;
            }
        }
    }
    let mut list = Element::new("x14:slicerList").with_attr("xmlns:x14", X14_NS);
    list.push_element(Element::new("x14:slicer").with_attr("r:id", rid));
    let mut ext = Element::new("ext")
        .with_attr("uri", SHEET_SLICER_LIST_URI)
        .with_attr("xmlns:x15", X15_NS);
    ext.push_element(list);
    ext_lst.push_element(ext);
}

pub(crate) fn push_wb_cache(wb: &mut Document, table: bool, rid: &str) {
    picture::ensure_xmlns(&mut wb.root, "r", REL_NS);
    picture::ensure_xmlns(&mut wb.root, "mc", MC_NS);
    picture::ensure_xmlns(&mut wb.root, "x14", X14_NS);
    picture::ensure_xmlns(&mut wb.root, "x15", X15_NS);
    ensure_ignorable(&mut wb.root, &["x14", "x15"]);
    if wb.root.child_by_local("extLst").is_none() {
        names::insert_workbook_child(&mut wb.root, Element::new("extLst"), "extLst");
    }
    let uri = if table {
        WB_X15_SLICER_CACHES_URI
    } else {
        WB_X14_SLICER_CACHES_URI
    };
    let list_name = if table {
        "x15:slicerCaches"
    } else {
        "x14:slicerCaches"
    };
    let ext_lst = wb.root.child_by_local_mut("extLst").unwrap();
    for n in ext_lst.children.iter_mut() {
        let Node::Element(ext) = n else { continue };
        if !uri_eq(ext.attr("uri").unwrap_or(""), uri) {
            continue;
        }
        for list in ext.element_children_mut() {
            if list.local_name() == "slicerCaches" {
                list.push_element(Element::new("x14:slicerCache").with_attr("r:id", rid));
                return;
            }
        }
    }
    let mut list = Element::new(list_name);
    if table {
        list.set_attr("xmlns:x14", X14_NS);
    }
    list.push_element(Element::new("x14:slicerCache").with_attr("r:id", rid));
    let mut ext = Element::new("ext").with_attr("uri", uri);
    if table {
        ext.set_attr("xmlns:x15", X15_NS);
    } else {
        ext.set_attr("xmlns:x14", X14_NS);
    }
    ext.push_element(list);
    ext_lst.push_element(ext);
}

pub(crate) fn build_slicer_ac(
    frame: SlicerFrameKind,
    cnv: u32,
    name: &str,
    from: a1::CellRef,
    from_off_x: i64,
    from_off_y: i64,
    to: a1::CellRef,
    to_off_x: i64,
    to_off_y: i64,
    cx: i64,
    cy: i64,
    edit_as: EditAs,
) -> Element {
    // Excel 16.0：twoCellAnchor 是 wsDr 的子元素，Choice 只包 graphicFrame。
    let graphic = build_frame(frame, cnv, name, cx, cy);
    let mut choice = match frame {
        SlicerFrameKind::Table => Element::new("mc:Choice")
            .with_attr("xmlns:sle15", SLE15_NS)
            .with_attr("Requires", "sle15"),
        SlicerFrameKind::Pivot => Element::new("mc:Choice")
            .with_attr("xmlns:a14", A14_NS)
            .with_attr("Requires", "a14"),
        SlicerFrameKind::Timeline => Element::new("mc:Choice")
            .with_attr("xmlns:tsle", TSLE_NS)
            .with_attr("Requires", "tsle"),
    };
    choice.push_element(graphic);
    let mut fallback = Element::new("mc:Fallback");
    let fallback_text = match frame {
        SlicerFrameKind::Timeline => "此形状表示时间线。",
        _ => "此形状表示切片器。",
    };
    fallback.push_element(fallback_sp(name, cx, cy, fallback_text));
    let mut ac = Element::new("mc:AlternateContent").with_attr("xmlns:mc", MC_NS);
    ac.push_element(choice);
    ac.push_element(fallback);
    let mut anchor = two_cell(
        from, from_off_x, from_off_y, to, to_off_x, to_off_y, edit_as,
    );
    anchor.push_element(ac);
    anchor.push_element(Element::new("xdr:clientData"));
    anchor
}

pub(crate) fn two_cell(
    from: a1::CellRef,
    from_off_x: i64,
    from_off_y: i64,
    to: a1::CellRef,
    to_off_x: i64,
    to_off_y: i64,
    edit_as: EditAs,
) -> Element {
    let mut anchor = Element::new("xdr:twoCellAnchor").with_attr("editAs", edit_as.as_str());
    anchor.push_element(picture::marker_el("xdr:from", from, from_off_x, from_off_y));
    anchor.push_element(picture::marker_el("xdr:to", to, to_off_x, to_off_y));
    anchor
}

#[derive(Clone, Copy)]
pub(crate) enum SlicerFrameKind {
    Table,
    Pivot,
    Timeline,
}

pub(crate) fn build_frame(
    frame: SlicerFrameKind,
    cnv: u32,
    name: &str,
    cx: i64,
    cy: i64,
) -> Element {
    let mut nv = Element::new("xdr:nvGraphicFramePr");
    nv.push_element(
        Element::new("xdr:cNvPr")
            .with_attr("id", cnv.to_string())
            .with_attr("name", name),
    );
    let mut locks = Element::new("xdr:cNvGraphicFramePr");
    if !matches!(frame, SlicerFrameKind::Timeline) {
        locks.push_element(Element::new("a:graphicFrameLocks").with_attr("noGrp", "1"));
    }
    nv.push_element(locks);
    let (ext_cx, ext_cy) = match frame {
        SlicerFrameKind::Timeline => ("0".to_string(), "0".to_string()),
        _ => (cx.to_string(), cy.to_string()),
    };
    let mut xfrm = Element::new("xdr:xfrm");
    xfrm.push_element(
        Element::new("a:off")
            .with_attr("x", "0")
            .with_attr("y", "0"),
    );
    xfrm.push_element(
        Element::new("a:ext")
            .with_attr("cx", ext_cx)
            .with_attr("cy", ext_cy),
    );
    let data = match frame {
        SlicerFrameKind::Timeline => {
            let mut data = Element::new("a:graphicData").with_attr("uri", TSLE_NS);
            data.push_element(
                Element::new("tsle:timeslicer")
                    .with_attr("xmlns:tsle", TSLE_NS)
                    .with_attr("name", name),
            );
            data
        }
        _ => {
            let mut data = Element::new("a:graphicData").with_attr("uri", SLE_NS);
            data.push_element(
                Element::new("sle:slicer")
                    .with_attr("xmlns:sle", SLE_NS)
                    .with_attr("name", name),
            );
            data
        }
    };
    let mut graphic = Element::new("a:graphic");
    graphic.push_element(data);
    let mut frame = Element::new("xdr:graphicFrame").with_attr("macro", "");
    frame.push_element(nv);
    frame.push_element(xfrm);
    frame.push_element(graphic);
    frame
}

pub(crate) fn fallback_sp(name: &str, cx: i64, cy: i64, text: &str) -> Element {
    let mut nv = Element::new("xdr:nvSpPr");
    nv.push_element(
        Element::new("xdr:cNvPr")
            .with_attr("id", "0")
            .with_attr("name", name),
    );
    nv.push_element(Element::new("xdr:cNvSpPr"));
    let mut xfrm = Element::new("a:xfrm");
    xfrm.push_element(
        Element::new("a:off")
            .with_attr("x", "0")
            .with_attr("y", "0"),
    );
    xfrm.push_element(
        Element::new("a:ext")
            .with_attr("cx", cx.to_string())
            .with_attr("cy", cy.to_string()),
    );
    let mut geom = Element::new("a:prstGeom").with_attr("prst", "rect");
    geom.push_element(Element::new("a:avLst"));
    let mut sp_pr = Element::new("xdr:spPr");
    sp_pr.push_element(xfrm);
    sp_pr.push_element(geom);
    let mut p = Element::new("a:p");
    let mut r = Element::new("a:r");
    r.push_element(Element::new("a:t").with_text(text));
    p.push_element(r);
    let mut body = Element::new("xdr:txBody");
    body.push_element(Element::new("a:bodyPr"));
    body.push_element(Element::new("a:lstStyle"));
    body.push_element(p);
    let mut sp = Element::new("xdr:sp")
        .with_attr("macro", "")
        .with_attr("textlink", "");
    sp.push_element(nv);
    sp.push_element(sp_pr);
    sp.push_element(body);
    sp
}

pub(crate) fn remove_slicer_anchor(root: &mut Element, sle_name: &str, cnv: Option<&str>) -> bool {
    let mut i = 0;
    while i < root.children.len() {
        let hit = match &root.children[i] {
            Node::Element(e)
                if e.local_name() == "AlternateContent"
                    || AnchorKind::from_local(e.local_name()).is_some() =>
            {
                slicer_in(e, sle_name, cnv)
            }
            _ => false,
        };
        if hit {
            root.children.remove(i);
            return true;
        }
        if let Node::Element(e) = &mut root.children[i] {
            if remove_slicer_anchor(e, sle_name, cnv) {
                return true;
            }
        }
        i += 1;
    }
    false
}

pub(crate) fn slicer_in(el: &Element, want_name: &str, cnv: Option<&str>) -> bool {
    let mut hit = false;
    let _ = picture::walk_anchors(el, &mut |anchor, _| {
        shape::visit_payloads(anchor, &mut |e| {
            if super::is_slicer_or_timeline_frame(e) {
                let name = sle_name(e);
                let (id, _) = shape::cnv_of_object(e);
                if name.as_deref() == Some(want_name) || (cnv.is_some() && id.as_deref() == cnv) {
                    hit = true;
                }
            }
            Ok(())
        })
    });
    hit
}

pub(crate) fn remove_slicer_view(
    pkg: &mut Package,
    sheet_doc: &mut Document,
    sheet_part: &str,
    name: &str,
) -> Result<(), Error> {
    let mut rels = Relationships::from_package(pkg, sheet_part)?;
    let mut drop_rid = None;
    let mut drop_part = None;
    for rel in &rels.rels {
        if !rel.rel_type.ends_with("/slicer") {
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
        let before = doc.root.children_by_local("slicer").len();
        doc.root.children.retain(|n| match n {
            Node::Element(e) if e.local_name() == "slicer" => e.attr("name") != Some(name),
            _ => true,
        });
        if doc.root.children_by_local("slicer").len() == before {
            continue;
        }
        if doc.root.children_by_local("slicer").is_empty() {
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
        drop_sheet_list_rid(&mut sheet_doc.root, &rid);
    }
    Ok(())
}

pub(crate) fn drop_sheet_list_rid(sheet: &mut Element, rid: &str) {
    let Some(ext_lst) = sheet.child_by_local_mut("extLst") else {
        return;
    };
    for n in ext_lst.children.iter_mut() {
        let Node::Element(ext) = n else { continue };
        if !uri_eq(ext.attr("uri").unwrap_or(""), SHEET_SLICER_LIST_URI) {
            continue;
        }
        if let Some(list) = ext.child_by_local_mut("slicerList") {
            list.children.retain(|n| match n {
                Node::Element(e) => rid_attr(e) != Some(rid),
                _ => true,
            });
        }
    }
    ext_lst.children.retain(|n| match n {
        Node::Element(ext) if uri_eq(ext.attr("uri").unwrap_or(""), SHEET_SLICER_LIST_URI) => ext
            .child_by_local("slicerList")
            .is_some_and(|l| l.element_children().next().is_some()),
        _ => true,
    });
    if ext_lst.element_children().next().is_none() {
        sheet
            .children
            .retain(|n| !matches!(n, Node::Element(e) if e.local_name() == "extLst"));
    }
}

pub(crate) fn cache_users(pkg: &Package, cache_name: &str) -> Result<usize, Error> {
    let wb_part = office_document_part(pkg)?;
    let wb = super::super::list_pin::parse_part(pkg, &wb_part)?;
    let mut n = 0usize;
    if let Some(sheets) = wb.root.child_by_local("sheets") {
        for sh in sheets.children_by_local("sheet") {
            if let Some(rid) = rid_attr(sh) {
                if let Ok(part) = resolve_id(pkg, &wb_part, rid) {
                    if let Ok(views) = load_all_views(pkg, &part) {
                        n += views.iter().filter(|v| v.cache == cache_name).count();
                    }
                }
            }
        }
    }
    Ok(n)
}

pub(crate) fn drop_cache(
    pkg: &mut Package,
    wb_doc: &mut Document,
    wb_part: &str,
    cache_name: &str,
) -> Result<(), Error> {
    let mut rels = Relationships::from_package(pkg, wb_part)?;
    let mut drop_rid = None;
    let mut drop_part = None;
    for rel in &rels.rels {
        if !(rel.rel_type.ends_with("/slicerCache") || rel.rel_type.ends_with("/timelineCache")) {
            continue;
        }
        let Ok(part) = resolve_rel_target(wb_part, &rel.target) else {
            continue;
        };
        let Ok(bytes) = pkg.get(&part) else {
            continue;
        };
        let Ok(doc) = xml::parse(bytes) else {
            continue;
        };
        if doc.root.attr("name") == Some(cache_name) {
            drop_rid = Some(rel.id.clone());
            drop_part = Some(part);
            break;
        }
    }
    if let (Some(rid), Some(part)) = (drop_rid, drop_part) {
        rels.remove_by_id(&rid);
        rels.write_to(pkg, wb_part)?;
        pkg.remove(&part);
        pkg.remove(&rels_part_name(&part));
        let mut cts = ContentTypes::from_package(pkg)?;
        cts.remove_override(&part);
        cts.write_to(pkg)?;
        drop_wb_cache_rid(&mut wb_doc.root, &rid);
        drop_cache_defined_name(&mut wb_doc.root, cache_name);
    }
    Ok(())
}

pub(crate) fn drop_wb_cache_rid(wb: &mut Element, rid: &str) {
    let Some(ext_lst) = wb.child_by_local_mut("extLst") else {
        return;
    };
    for n in ext_lst.children.iter_mut() {
        let Node::Element(ext) = n else { continue };
        let uri = ext.attr("uri").unwrap_or("");
        if !(uri_eq(uri, WB_X14_SLICER_CACHES_URI)
            || uri_eq(uri, WB_X15_SLICER_CACHES_URI)
            || uri_eq(uri, WB_TIMELINE_CACHE_REFS_URI))
        {
            continue;
        }
        for list in ext.element_children_mut() {
            if list.local_name() == "slicerCaches" || list.local_name() == "timelineCacheRefs" {
                list.children.retain(|n| match n {
                    Node::Element(e) => rid_attr(e) != Some(rid),
                    _ => true,
                });
            }
        }
    }
    ext_lst.children.retain(|n| match n {
        Node::Element(ext) => {
            let uri = ext.attr("uri").unwrap_or("");
            if uri_eq(uri, WB_X14_SLICER_CACHES_URI) || uri_eq(uri, WB_X15_SLICER_CACHES_URI) {
                ext.element_children().any(|list| {
                    list.local_name() == "slicerCaches" && list.element_children().next().is_some()
                })
            } else if uri_eq(uri, WB_TIMELINE_CACHE_REFS_URI) {
                ext.element_children().any(|list| {
                    list.local_name() == "timelineCacheRefs"
                        && list.element_children().next().is_some()
                })
            } else {
                true
            }
        }
        _ => true,
    });
    if ext_lst.element_children().next().is_none() {
        wb.children
            .retain(|n| !matches!(n, Node::Element(e) if e.local_name() == "extLst"));
    }
}

pub(crate) fn push_cache_defined_name(wb: &mut Document, name: &str) {
    if let Some(parent) = wb.root.child_by_local("definedNames") {
        if parent
            .children_by_local("definedName")
            .iter()
            .any(|e| e.attr("name").is_some_and(|n| n.eq_ignore_ascii_case(name)))
        {
            return;
        }
    }
    if wb.root.child_by_local("definedNames").is_none() {
        names::insert_workbook_child(&mut wb.root, Element::new("definedNames"), "definedNames");
    }
    let parent = wb.root.child_by_local_mut("definedNames").unwrap();
    parent.push_element(
        Element::new("definedName")
            .with_attr("name", name)
            .with_text("#N/A"),
    );
}

pub(crate) fn drop_cache_defined_name(wb: &mut Element, name: &str) {
    let empty = {
        let Some(parent) = wb.child_by_local_mut("definedNames") else {
            return;
        };
        parent.children.retain(|n| match n {
            Node::Element(e) if e.local_name() == "definedName" => {
                !e.attr("name").is_some_and(|n| n.eq_ignore_ascii_case(name))
            }
            _ => true,
        });
        let n = parent.children_by_local("definedName").len();
        n == 0
    };
    if empty {
        wb.children
            .retain(|n| !matches!(n, Node::Element(e) if e.local_name() == "definedNames"));
    }
}

pub(crate) fn add_rel(
    pkg: &mut Package,
    source: &str,
    rel_type: &str,
    target: &str,
) -> Result<String, Error> {
    let mut rels = Relationships::from_package(pkg, source)?;
    let rid = rels.next_rid();
    rels.push(Relationship {
        id: rid.clone(),
        rel_type: rel_type.into(),
        target: picture::rel_target_from(source, target),
        target_mode: None,
        extra: Vec::new(),
    });
    rels.write_to(pkg, source)?;
    Ok(rid)
}

pub(crate) fn next_part(pkg: &Package, prefix: &str, ext: &str) -> String {
    let mut n = 1u32;
    loop {
        let name = format!("{prefix}{n}.{ext}");
        if !pkg.contains(&name) {
            return name;
        }
        n += 1;
    }
}

pub(crate) fn rid_attr(el: &Element) -> Option<&str> {
    el.attrs.iter().find_map(|(k, v)| {
        if k.contains(':') && k.rsplit(':').next() == Some("id") {
            Some(v.as_str())
        } else {
            None
        }
    })
}

pub(crate) fn uri_eq(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
}

pub(crate) fn ensure_ignorable(el: &mut Element, tokens: &[&str]) {
    picture::ensure_xmlns(el, "mc", MC_NS);
    let mut cur: Vec<String> = el
        .attr("mc:Ignorable")
        .unwrap_or("")
        .split_whitespace()
        .map(str::to_string)
        .collect();
    for t in tokens {
        if !cur.iter().any(|x| x == t) {
            cur.push((*t).to_string());
        }
    }
    if !cur.is_empty() {
        el.set_attr("mc:Ignorable", cur.join(" "));
    }
}

pub(crate) fn field_err(field: &str, op_index: usize) -> Error {
    Error::with_path(
        ErrorKind::SlicerFieldNotFound {
            name: field.to_string(),
        },
        format!("ops[{op_index}].field"),
    )
}

pub(crate) fn source_id_err(op_index: usize) -> Error {
    Error::with_path(
        ErrorKind::InvalidId {
            reason: IdReason::InsertSlicerNeedsTableOrPivot,
        },
        format!("ops[{op_index}].source"),
    )
}

pub(crate) fn slicer_selected(pkg: &Package, cache: Option<&CacheMeta>) -> Vec<String> {
    let Some(cache) = cache else {
        return Vec::new();
    };
    let Ok(bytes) = pkg.get(&cache.part) else {
        return Vec::new();
    };
    let Ok(doc) = xml::parse(bytes) else {
        return Vec::new();
    };
    if cache.kind == "table" {
        return super::table::table_selected(pkg, &doc.root);
    }
    super::pivot::pivot_selected(pkg, &doc.root, &cache.field)
}

pub(crate) fn unknown_selected(op_index: usize) -> Error {
    Error::with_path(
        ErrorKind::InvalidSlicer {
            field: "selected".into(),
        },
        format!("ops[{op_index}].selected"),
    )
}

pub(crate) fn attr_is_true(el: &Element, name: &str) -> bool {
    matches!(el.attr(name), Some("1" | "true" | "TRUE"))
}
