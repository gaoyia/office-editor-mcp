//! 工作表切片器与时间线：inspect 名单，apply 写 slicer / timeline 部件与 drawing graphicFrame。
//! 不在库里执行筛选，不刷新透视合计。

use serde_json::{json, Map, Value};

use crate::error::{Error, ErrorKind, IdReason, RelSource};
use crate::opc::{
    office_document_part, rels_part_name, resolve_id, resolve_rel_target, ContentTypes, Package,
    Relationship, Relationships,
};
use crate::xml::{self, Document, Element, Node, DEFAULT_DECL};

use super::a1;
use super::layout;
use super::names;
use super::picture::{self, AnchorKind, EditAs};
use super::shape;
use super::REL_NS;

const MC_NS: &str = "http://schemas.openxmlformats.org/markup-compatibility/2006";
const X14_NS: &str = "http://schemas.microsoft.com/office/spreadsheetml/2009/9/main";
const X15_NS: &str = "http://schemas.microsoft.com/office/spreadsheetml/2010/11/main";
const SLE_NS: &str = "http://schemas.microsoft.com/office/drawing/2010/slicer";
const SLE15_NS: &str = "http://schemas.microsoft.com/office/drawing/2012/slicer";
const TSLE_NS: &str = "http://schemas.microsoft.com/office/drawing/2012/timeslicer";
const A14_NS: &str = "http://schemas.microsoft.com/office/drawing/2010/main";
const SML_NS: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";

const REL_SLICER: &str = "http://schemas.microsoft.com/office/2007/relationships/slicer";
const REL_SLICER_CACHE: &str = "http://schemas.microsoft.com/office/2007/relationships/slicerCache";
const REL_TIMELINE: &str = "http://schemas.microsoft.com/office/2011/relationships/timeline";
const REL_TIMELINE_CACHE: &str =
    "http://schemas.microsoft.com/office/2011/relationships/timelineCache";
const CT_SLICER: &str = "application/vnd.ms-excel.slicer+xml";
const CT_SLICER_CACHE: &str = "application/vnd.ms-excel.slicerCache+xml";
const CT_TIMELINE: &str = "application/vnd.ms-excel.timeline+xml";
const CT_TIMELINE_CACHE: &str = "application/vnd.ms-excel.timelineCache+xml";

/// Excel 16.0 表格切片器工作表 `slicerList`。
const SHEET_SLICER_LIST_URI: &str = "{3A4CF648-6AED-40f4-86FF-DC5316D8AED3}";
/// 工作簿表格 `x15:slicerCaches`。
const WB_X15_SLICER_CACHES_URI: &str = "{46BE6895-7355-4a93-B00E-2C351335B9C9}";
/// 工作簿透视 `x14:slicerCaches`。
const WB_X14_SLICER_CACHES_URI: &str = "{BBE1A952-AA13-448e-AADC-164F8A28A991}";
/// slicerCache 上的 `x15:tableSlicerCache`。
const TABLE_SLICER_CACHE_URI: &str = "{2F2917AC-EB37-4324-AD4E-5DD8C200BD13}";
/// 透视缓存定义上的 `x14:pivotCacheDefinition`（MS-XLSX 2.2.4.6）。
const PIVOT_CACHE_DEF_EXT_URI: &str = "{725AE2AE-9491-48be-B2B4-4EB974FC3084}";
/// 工作表 `x15:timelineRefs`。Excel 16.0 另存 URI 含小写 hex。
const SHEET_TIMELINE_REFS_URI: &str = "{7E03D99C-DC04-49d9-9315-930204A7B6E9}";
/// 工作簿 `x15:timelineCacheRefs`。
const WB_TIMELINE_CACHE_REFS_URI: &str = "{D0CA8CA8-9F24-4464-BF8E-62219DCF47F9}";

const DEFAULT_CX: i64 = 1_905_000;
const DEFAULT_CY: i64 = 2_540_000;
/// Excel 16.0 时间线约 400×80 磅。
const DEFAULT_TIMELINE_CX: i64 = 5_080_000;
const DEFAULT_TIMELINE_CY: i64 = 1_016_000;
const DEFAULT_ROW_HEIGHT: i64 = 381_000;
const DEFAULT_COLUMN_COUNT: u32 = 1;
const DEFAULT_TIMELINE_LEVEL: u32 = 2;

#[derive(Debug, Clone)]
pub enum SlicerKey {
    Id(String),
    Index(usize),
}

#[derive(Debug, Clone)]
pub struct SlicerTarget {
    pub sheet: String,
    pub key: SlicerKey,
}

#[derive(Debug, Clone)]
pub enum SlicerSource {
    Table { sheet: String, index: usize },
    Pivot { sheet: String, index: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlicerKind {
    Slicer,
    Timeline,
}

#[derive(Debug, Clone, Default)]
pub struct SlicerViewFields {
    pub caption: Option<String>,
    pub row_height: Option<i64>,
    pub column_count: Option<u32>,
    pub show_caption: Option<bool>,
    pub start_item: Option<u32>,
    pub locked_position: Option<bool>,
    pub style: Option<Option<String>>,
    pub show_header: Option<bool>,
    pub show_selection_label: Option<bool>,
    pub show_time_level: Option<bool>,
    pub show_horizontal_scrollbar: Option<bool>,
    pub selection_level: Option<u32>,
    pub scroll_position: Option<Option<String>>,
}

impl SlicerViewFields {
    pub(crate) fn slicer_only_field(&self) -> Option<&'static str> {
        if self.row_height.is_some() {
            Some("rowHeight")
        } else if self.column_count.is_some() {
            Some("columnCount")
        } else if self.show_caption.is_some() {
            Some("showCaption")
        } else if self.start_item.is_some() {
            Some("startItem")
        } else if self.locked_position.is_some() {
            Some("lockedPosition")
        } else {
            None
        }
    }

    pub(crate) fn timeline_only_field(&self) -> Option<&'static str> {
        if self.show_header.is_some() {
            Some("showHeader")
        } else if self.show_selection_label.is_some() {
            Some("showSelectionLabel")
        } else if self.show_time_level.is_some() {
            Some("showTimeLevel")
        } else if self.show_horizontal_scrollbar.is_some() {
            Some("showHorizontalScrollbar")
        } else if self.selection_level.is_some() {
            Some("selectionLevel")
        } else if self.scroll_position.is_some() {
            Some("scrollPosition")
        } else {
            None
        }
    }

    pub(crate) fn any_set(&self) -> bool {
        self.caption.is_some()
            || self.style.is_some()
            || self.slicer_only_field().is_some()
            || self.timeline_only_field().is_some()
    }
}

#[derive(Debug, Clone)]
pub struct InsertSlicerOp {
    pub sheet: String,
    pub source: SlicerSource,
    pub field: String,
    pub kind: SlicerKind,
    pub level: u32,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
    pub from: a1::CellRef,
    pub to: Option<a1::CellRef>,
    pub anchor: AnchorKind,
    pub from_off_x: i64,
    pub from_off_y: i64,
    pub to_off_x: i64,
    pub to_off_y: i64,
    pub view: SlicerViewFields,
}

#[derive(Debug, Clone)]
pub struct DeleteSlicerOp {
    pub id: String,
    pub target: SlicerTarget,
}

#[derive(Debug, Clone)]
pub struct SetSlicerSizeOp {
    pub id: String,
    pub target: SlicerTarget,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
    pub to: Option<a1::CellRef>,
    pub to_off_x: Option<i64>,
    pub to_off_y: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct SetSlicerAnchorOp {
    pub id: String,
    pub target: SlicerTarget,
    pub anchor: Option<AnchorKind>,
    pub from: Option<a1::CellRef>,
    pub to: Option<a1::CellRef>,
    pub from_off_x: Option<i64>,
    pub from_off_y: Option<i64>,
    pub to_off_x: Option<i64>,
    pub to_off_y: Option<i64>,
    pub pos_x: Option<i64>,
    pub pos_y: Option<i64>,
    pub edit_as: Option<EditAs>,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
    pub rel: Option<super::request::RelBoxPatch>,
}

#[derive(Debug, Clone)]
pub struct SetSlicerViewOp {
    pub id: String,
    pub target: SlicerTarget,
    pub view: SlicerViewFields,
    pub level: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct TimelineFilter {
    pub filter_type: String,
    pub start_date: String,
    pub end_date: String,
}

#[derive(Debug, Clone)]
pub struct SetSlicerFilterOp {
    pub id: String,
    pub target: SlicerTarget,
    pub selected: Option<Vec<String>>,
    pub filter: Option<Option<TimelineFilter>>,
}

#[derive(Clone)]
struct Hit {
    cnv: Option<String>,
    cnv_name: String,
    sle_name: String,
    kind: AnchorKind,
    timeline: bool,
    anchor: Element,
}

pub fn parse_slicer_id(id: &str, json_path: &str) -> Result<SlicerTarget, Error> {
    let rest = id.strip_prefix("sheet:").ok_or_else(|| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SheetSlicerPrefix,
            },
            json_path.to_string(),
        )
    })?;
    let Some((name, after)) = rest.split_once("/slicer:") else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SheetSlicerContainer,
            },
            json_path.to_string(),
        ));
    };
    if name.is_empty() || after.is_empty() {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SheetSlicerPrefix,
            },
            json_path.to_string(),
        ));
    }
    let key = if let Some(rest) = after.strip_prefix("index:") {
        let k: usize = rest.parse().map_err(|_| {
            Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::SlicerIndexNotInt,
                },
                json_path.to_string(),
            )
        })?;
        if k < 1 {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::SlicerIndexFromOne,
                },
                json_path.to_string(),
            ));
        }
        SlicerKey::Index(k)
    } else {
        SlicerKey::Id(after.to_string())
    };
    Ok(SlicerTarget {
        sheet: name.to_string(),
        key,
    })
}

pub fn count(pkg: &Package, sheet_part: &str) -> Result<usize, Error> {
    Ok(collect_hits(pkg, sheet_part)?.len())
}

pub fn collect(pkg: &Package, sheet_part: &str, sheet_name: &str) -> Result<Vec<Value>, Error> {
    let hits = collect_hits(pkg, sheet_part)?;
    let views = load_all_views(pkg, sheet_part)?;
    let caches = load_caches(pkg)?;
    let mut out = Vec::new();
    for (i, hit) in hits.iter().enumerate() {
        out.push(hit_json(
            pkg,
            sheet_part,
            sheet_name,
            hit,
            i + 1,
            &views,
            &caches,
        )?);
    }
    Ok(out)
}

pub fn insert(
    pkg: &mut Package,
    wb_doc: &mut Document,
    wb_part: &str,
    sheet_doc: &mut Document,
    sheet_part: &str,
    op: &InsertSlicerOp,
    op_index: usize,
) -> Result<(), Error> {
    match (&op.kind, &op.source) {
        (SlicerKind::Timeline, SlicerSource::Table { .. }) => Err(Error::with_path(
            ErrorKind::InvalidSlicer {
                field: "source".into(),
            },
            format!("ops[{op_index}].source"),
        )),
        (SlicerKind::Timeline, SlicerSource::Pivot { sheet, index }) => {
            if !sheet.eq_ignore_ascii_case(&op.sheet) {
                return Err(source_id_err(op_index));
            }
            insert_timeline(
                pkg, wb_doc, wb_part, sheet_doc, sheet_part, op, *index, op_index,
            )
        }
        (SlicerKind::Slicer, SlicerSource::Table { sheet, index }) => {
            if !sheet.eq_ignore_ascii_case(&op.sheet) {
                return Err(source_id_err(op_index));
            }
            insert_table(
                pkg, wb_doc, wb_part, sheet_doc, sheet_part, op, *index, op_index,
            )
        }
        (SlicerKind::Slicer, SlicerSource::Pivot { sheet, index }) => {
            if !sheet.eq_ignore_ascii_case(&op.sheet) {
                return Err(source_id_err(op_index));
            }
            insert_pivot(
                pkg, wb_doc, wb_part, sheet_doc, sheet_part, op, *index, op_index,
            )
        }
    }
}

pub fn delete(
    pkg: &mut Package,
    wb_doc: &mut Document,
    wb_part: &str,
    sheet_doc: &mut Document,
    sheet_part: &str,
    op: &DeleteSlicerOp,
    op_index: usize,
) -> Result<(), Error> {
    let path = format!("ops[{op_index}].id");
    let hits = collect_hits(pkg, sheet_part)?;
    let Some((idx, hit)) = hits
        .iter()
        .enumerate()
        .find(|(i, h)| key_matches(&op.target.key, h, *i + 1))
    else {
        return Err(Error::with_path(
            ErrorKind::SlicerNotFound { id: op.id.clone() },
            path,
        ));
    };
    let _ = idx;
    let view_name = hit.sle_name.clone();
    let timeline = hit.timeline;
    let views = if timeline {
        load_timeline_views(pkg, sheet_part)?
    } else {
        load_slicer_views(pkg, sheet_part)?
    };
    let cache_name = views
        .iter()
        .find(|v| v.name == view_name)
        .map(|v| v.cache.clone())
        .unwrap_or_default();

    if let Some(drawing_part) = picture::drawing_parts_from(pkg, sheet_part, &sheet_doc.root)?
        .into_iter()
        .next()
    {
        let mut draw = xml::parse(pkg.get(&drawing_part)?)?;
        remove_slicer_anchor(&mut draw.root, &hit.sle_name, hit.cnv.as_deref());
        shape::write_or_drop_drawing(pkg, sheet_doc, sheet_part, &drawing_part, draw)?;
    }

    if timeline {
        remove_timeline_view(pkg, sheet_doc, sheet_part, &view_name)?;
    } else {
        remove_slicer_view(pkg, sheet_doc, sheet_part, &view_name)?;
    }
    if !cache_name.is_empty() && cache_users(pkg, &cache_name)? == 0 {
        drop_cache(pkg, wb_doc, wb_part, &cache_name)?;
    }
    Ok(())
}

fn draw_key(key: &SlicerKey) -> shape::DrawKey {
    match key {
        SlicerKey::Id(s) => shape::DrawKey::Cnv(s.clone()),
        SlicerKey::Index(n) => shape::DrawKey::Index(*n),
    }
}

/// 改外层 `xdr:ext` 或 `twoCell` 的 `to`。不改 cache。
pub fn set_size(
    pkg: &mut Package,
    sheet_root: &Element,
    sheet_part: &str,
    op: &SetSlicerSizeOp,
    op_index: usize,
) -> Result<(), Error> {
    shape::set_object_size(
        pkg,
        sheet_root,
        sheet_part,
        &draw_key(&op.target.key),
        is_slicer_or_timeline_frame,
        picture::SizePatch {
            cx: op.cx,
            cy: op.cy,
            to: op.to,
            to_off_x: op.to_off_x,
            to_off_y: op.to_off_y,
        },
        ErrorKind::TwoCellSlicerSize,
        ErrorKind::SlicerNotFound { id: op.id.clone() },
        "setSlicerSize",
        op_index,
    )
}

/// 改包含 graphicFrame 的外层锚点。留下 AlternateContent 与 clientData。
pub fn set_anchor(
    pkg: &mut Package,
    sheet_root: &Element,
    sheet_part: &str,
    op: &SetSlicerAnchorOp,
    op_index: usize,
) -> Result<(), Error> {
    shape::set_object_anchor(
        pkg,
        sheet_root,
        sheet_part,
        &draw_key(&op.target.key),
        is_slicer_or_timeline_frame,
        shape::AnchorWrite {
            op: "setSlicerAnchor",
            anchor: op.anchor,
            from: op.from,
            to: op.to,
            from_off_x: op.from_off_x,
            from_off_y: op.from_off_y,
            to_off_x: op.to_off_x,
            to_off_y: op.to_off_y,
            pos_x: op.pos_x,
            pos_y: op.pos_y,
            edit_as: op.edit_as,
            cx: op.cx,
            cy: op.cy,
            default_cx: DEFAULT_CX,
            default_cy: DEFAULT_CY,
            rel: op.rel,
        },
        op_index,
        ErrorKind::SlicerNotFound { id: op.id.clone() },
    )
}

/// 改 slicer / timeline 视图部件属性。不改 drawing，不改 cache。
pub fn set_view(
    pkg: &mut Package,
    sheet_part: &str,
    op: &SetSlicerViewOp,
    op_index: usize,
) -> Result<(), Error> {
    let path = format!("ops[{op_index}].id");
    let hits = collect_hits(pkg, sheet_part)?;
    let Some(hit) = hits
        .iter()
        .enumerate()
        .find(|(i, h)| key_matches(&op.target.key, h, *i + 1))
        .map(|(_, h)| h)
    else {
        return Err(Error::with_path(
            ErrorKind::SlicerNotFound { id: op.id.clone() },
            path,
        ));
    };
    if hit.timeline {
        if let Some(field) = op.view.slicer_only_field() {
            return Err(Error::with_path(
                ErrorKind::InvalidSlicer {
                    field: field.into(),
                },
                format!("ops[{op_index}].{field}"),
            ));
        }
    } else if let Some(field) = op.view.timeline_only_field().or(op.level.map(|_| "level")) {
        return Err(Error::with_path(
            ErrorKind::InvalidSlicer {
                field: field.into(),
            },
            format!("ops[{op_index}].{field}"),
        ));
    }
    patch_named_view(
        pkg,
        sheet_part,
        hit.timeline,
        &hit.sle_name,
        &op.id,
        op_index,
        |el| {
            apply_view_attrs(el, &op.view, op.level, hit.timeline);
            Ok(())
        },
    )
}

/// 改 slicerCache 勾选或 timelineCache 筛选区间。不改 drawing，不改视图标题。
pub fn set_filter(
    pkg: &mut Package,
    sheet_part: &str,
    op: &SetSlicerFilterOp,
    op_index: usize,
) -> Result<(), Error> {
    let path = format!("ops[{op_index}].id");
    let hits = collect_hits(pkg, sheet_part)?;
    let Some(hit) = hits
        .iter()
        .enumerate()
        .find(|(i, h)| key_matches(&op.target.key, h, *i + 1))
        .map(|(_, h)| h)
    else {
        return Err(Error::with_path(
            ErrorKind::SlicerNotFound { id: op.id.clone() },
            path,
        ));
    };
    if hit.timeline {
        if op.selected.is_some() {
            return Err(Error::with_path(
                ErrorKind::InvalidSlicer {
                    field: "selected".into(),
                },
                format!("ops[{op_index}].selected"),
            ));
        }
        let Some(filter) = op.filter.as_ref() else {
            return Err(Error::with_path(
                ErrorKind::NeedSetSlicerFilterField,
                format!("ops[{op_index}]"),
            ));
        };
        return set_timeline_filter(pkg, sheet_part, &hit.sle_name, filter.as_ref(), op_index);
    }
    if op.filter.is_some() {
        return Err(Error::with_path(
            ErrorKind::InvalidSlicer {
                field: "filter".into(),
            },
            format!("ops[{op_index}].filter"),
        ));
    }
    let Some(selected) = op.selected.as_ref() else {
        return Err(Error::with_path(
            ErrorKind::NeedSetSlicerFilterField,
            format!("ops[{op_index}]"),
        ));
    };
    set_slicer_selected(pkg, sheet_part, &hit.sle_name, selected, op_index)
}

fn insert_table(
    pkg: &mut Package,
    wb_doc: &mut Document,
    wb_part: &str,
    sheet_doc: &mut Document,
    sheet_part: &str,
    op: &InsertSlicerOp,
    index: usize,
    op_index: usize,
) -> Result<(), Error> {
    let (table_id, display, col_id) = table_column(
        pkg,
        &sheet_doc.root,
        sheet_part,
        &op.sheet,
        index,
        &op.field,
        op_index,
    )?;
    let cx = op.cx.unwrap_or(DEFAULT_CX);
    let cy = op.cy.unwrap_or(DEFAULT_CY);
    let (to, to_off_x, to_off_y) = dest_to(&sheet_doc.root, op, cx, cy);
    let cache_name = unique_cache_name(pkg, wb_doc, &format!("Slicer_{}", op.field));
    let view_name = unique_view_name(pkg, &op.field);
    let cache_part = next_part(pkg, "/xl/slicerCaches/slicerCache", "xml");
    pkg.set(
        &cache_part,
        table_cache_xml(&cache_name, &op.field, table_id, col_id),
    )?;
    let cache_rid = add_rel(pkg, wb_part, REL_SLICER_CACHE, &cache_part)?;
    push_wb_cache(wb_doc, true, &cache_rid);
    push_cache_defined_name(wb_doc, &cache_name);
    let (slicer_part, slicer_rid, is_new) = ensure_slicers_part(pkg, sheet_doc, sheet_part)?;
    append_slicer_view(
        pkg,
        &slicer_part,
        &view_name,
        &cache_name,
        &op.field,
        &op.view,
    )?;
    if is_new {
        push_sheet_list(&mut sheet_doc.root, &slicer_rid);
    }
    let mut cts = ContentTypes::from_package(pkg)?;
    cts.set_override(&cache_part, CT_SLICER_CACHE)?;
    cts.set_override(&slicer_part, CT_SLICER)?;
    cts.write_to(pkg)?;
    write_drawing(
        pkg,
        sheet_doc,
        sheet_part,
        op,
        &view_name,
        SlicerFrameKind::Table,
        cx,
        cy,
        to,
        to_off_x,
        to_off_y,
    )?;
    let _ = display;
    Ok(())
}

fn insert_pivot(
    pkg: &mut Package,
    wb_doc: &mut Document,
    wb_part: &str,
    sheet_doc: &mut Document,
    sheet_part: &str,
    op: &InsertSlicerOp,
    index: usize,
    op_index: usize,
) -> Result<(), Error> {
    let meta = pivot_field(
        pkg, wb_part, sheet_part, &op.sheet, index, &op.field, op_index,
    )?;
    let tabular_id = ensure_x14_pivot_cache_id(pkg, wb_part, &meta.cache_id)?;
    let tab_id = sheet_id(wb_doc, &op.sheet).unwrap_or(1);
    let cx = op.cx.unwrap_or(DEFAULT_CX);
    let cy = op.cy.unwrap_or(DEFAULT_CY);
    let (to, to_off_x, to_off_y) = dest_to(&sheet_doc.root, op, cx, cy);
    let cache_name = unique_cache_name(pkg, wb_doc, &format!("Slicer_{}", op.field));
    let view_name = unique_view_name(pkg, &op.field);
    let cache_part = next_part(pkg, "/xl/slicerCaches/slicerCache", "xml");
    pkg.set(
        &cache_part,
        pivot_cache_xml(
            &cache_name,
            &op.field,
            tab_id,
            &meta.name,
            &tabular_id,
            meta.item_count,
        ),
    )?;
    let cache_rid = add_rel(pkg, wb_part, REL_SLICER_CACHE, &cache_part)?;
    push_wb_cache(wb_doc, false, &cache_rid);
    push_cache_defined_name(wb_doc, &cache_name);
    let (slicer_part, slicer_rid, is_new) = ensure_slicers_part(pkg, sheet_doc, sheet_part)?;
    append_slicer_view(
        pkg,
        &slicer_part,
        &view_name,
        &cache_name,
        &op.field,
        &op.view,
    )?;
    if is_new {
        push_sheet_list(&mut sheet_doc.root, &slicer_rid);
    }
    let mut cts = ContentTypes::from_package(pkg)?;
    cts.set_override(&cache_part, CT_SLICER_CACHE)?;
    cts.set_override(&slicer_part, CT_SLICER)?;
    cts.write_to(pkg)?;
    write_drawing(
        pkg,
        sheet_doc,
        sheet_part,
        op,
        &view_name,
        SlicerFrameKind::Pivot,
        cx,
        cy,
        to,
        to_off_x,
        to_off_y,
    )?;
    Ok(())
}

fn insert_timeline(
    pkg: &mut Package,
    wb_doc: &mut Document,
    wb_part: &str,
    sheet_doc: &mut Document,
    sheet_part: &str,
    op: &InsertSlicerOp,
    index: usize,
    op_index: usize,
) -> Result<(), Error> {
    let meta = pivot_field(
        pkg, wb_part, sheet_part, &op.sheet, index, &op.field, op_index,
    )?;
    // Excel 16.0 只在透视表 updatedVersion≥8 时加载时间线；库插入的透视默认是 4。
    bump_pivot_updated_version(pkg, &meta.part)?;
    let (start, end) = cache_date_bounds(pkg, wb_part, &meta.cache_id, &op.field, op_index)?;
    mark_cache_field_dates(pkg, wb_part, &meta.cache_id, &op.field, &start, &end)?;
    let tabular_id = ensure_x14_pivot_cache_id(pkg, wb_part, &meta.cache_id)?;
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

fn write_drawing(
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
    let mut draw_doc = xml::parse(pkg.get(&drawing_part)?)?;
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
    pkg.set(&drawing_part, draw_doc.to_bytes()?)?;
    Ok(())
}

fn dest_to(sheet: &Element, op: &InsertSlicerOp, cx: i64, cy: i64) -> (a1::CellRef, i64, i64) {
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

fn collect_hits(pkg: &Package, sheet_part: &str) -> Result<Vec<Hit>, Error> {
    let views = load_all_views(pkg, sheet_part)?;
    let caches = load_caches(pkg)?;
    let names: Vec<&str> = views
        .iter()
        .filter(|v| caches.iter().any(|c| c.name == v.cache))
        .map(|v| v.name.as_str())
        .collect();
    let sheet = xml::parse(pkg.get(sheet_part)?)?;
    let mut out = Vec::new();
    for drawing_part in picture::drawing_parts_from(pkg, sheet_part, &sheet.root)? {
        let doc = xml::parse(pkg.get(&drawing_part)?)?;
        picture::walk_anchors(&doc.root, &mut |anchor, kind| {
            shape::visit_payloads(anchor, &mut |el| {
                if is_slicer_or_timeline_frame(el) {
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

fn hit_json(
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
    let mut m = shape::geom_map(&hit.anchor, hit.kind, id, hit.cnv.clone(), name, stable);
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

fn insert_view_fields(out: &mut Map<String, Value>, view: Option<&SlicerView>, timeline: bool) {
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

fn insert_filter_fields(
    out: &mut Map<String, Value>,
    pkg: &Package,
    cache: Option<&CacheMeta>,
    timeline: bool,
) {
    if timeline {
        out.insert("filter".into(), timeline_filter_json(pkg, cache));
        return;
    }
    out.insert("selected".into(), json!(slicer_selected(pkg, cache)));
}

fn is_slicer_uri(uri: &str) -> bool {
    let t = uri.trim_end_matches('/');
    t == SLE_NS
}

fn is_timeslicer_uri(uri: &str) -> bool {
    let t = uri.trim_end_matches('/');
    t == TSLE_NS
}

fn graphic_uri(el: &Element) -> Option<String> {
    let mut found = None;
    picture::walk_all(el, &mut |e| {
        if e.local_name() == "graphicData" && found.is_none() {
            found = e.attr_local("uri").map(str::to_string);
        }
    });
    found
}

pub(crate) fn is_slicer_or_timeline_frame(el: &Element) -> bool {
    if el.local_name() != "graphicFrame" {
        return false;
    }
    graphic_uri(el).is_some_and(|u| is_slicer_uri(&u) || is_timeslicer_uri(&u))
}

fn frame_is_timeline(el: &Element) -> bool {
    graphic_uri(el).is_some_and(|u| is_timeslicer_uri(&u))
}

fn sle_name(el: &Element) -> Option<String> {
    let mut found = None;
    picture::walk_all(el, &mut |e| {
        if found.is_none() && (e.local_name() == "slicer" || e.local_name() == "timeslicer") {
            found = e.attr("name").filter(|s| !s.is_empty()).map(str::to_string);
        }
    });
    found
}

fn key_matches(key: &SlicerKey, hit: &Hit, index: usize) -> bool {
    match key {
        SlicerKey::Id(id) => hit.cnv.as_deref() == Some(id.as_str()),
        SlicerKey::Index(k) => *k == index,
    }
}

struct SlicerView {
    name: String,
    cache: String,
    caption: Option<String>,
    row_height: Option<i64>,
    column_count: Option<u32>,
    show_caption: Option<bool>,
    start_item: Option<u32>,
    locked_position: Option<bool>,
    style: Option<String>,
    show_header: Option<bool>,
    show_selection_label: Option<bool>,
    show_time_level: Option<bool>,
    show_horizontal_scrollbar: Option<bool>,
    level: Option<u32>,
    selection_level: Option<u32>,
    scroll_position: Option<String>,
}

fn view_from_slicer_el(el: &Element) -> Option<SlicerView> {
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

fn view_from_timeline_el(el: &Element) -> Option<SlicerView> {
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

fn parse_attr_u32(el: &Element, name: &str) -> Option<u32> {
    el.attr(name)?.parse().ok()
}

fn parse_attr_i64(el: &Element, name: &str) -> Option<i64> {
    el.attr(name)?.parse().ok()
}

fn parse_attr_bool(el: &Element, name: &str) -> Option<bool> {
    match el.attr(name)? {
        "1" | "true" | "TRUE" => Some(true),
        "0" | "false" | "FALSE" => Some(false),
        _ => None,
    }
}

fn xml_bool(v: bool) -> &'static str {
    if v {
        "1"
    } else {
        "0"
    }
}

fn set_opt_bool(el: &mut Element, name: &str, v: Option<bool>) {
    if let Some(b) = v {
        el.set_attr(name, xml_bool(b));
    }
}

fn set_opt_u32(el: &mut Element, name: &str, v: Option<u32>) {
    if let Some(n) = v {
        el.set_attr(name, n.to_string());
    }
}

fn set_opt_style(el: &mut Element, v: &Option<Option<String>>) {
    match v {
        None => {}
        Some(None) => el.remove_attr("style"),
        Some(Some(s)) => el.set_attr("style", s),
    }
}

fn set_opt_scroll(el: &mut Element, v: &Option<Option<String>>) {
    match v {
        None => {}
        Some(None) => el.remove_attr("scrollPosition"),
        Some(Some(s)) => el.set_attr("scrollPosition", s),
    }
}

fn apply_view_attrs(el: &mut Element, view: &SlicerViewFields, level: Option<u32>, timeline: bool) {
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

fn patch_named_view(
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
        let doc = xml::parse(pkg.get(&part)?)?;
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
    pkg.set(&part, doc.to_bytes()?)?;
    Ok(())
}

fn load_slicer_views(pkg: &Package, sheet_part: &str) -> Result<Vec<SlicerView>, Error> {
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

fn load_timeline_views(pkg: &Package, sheet_part: &str) -> Result<Vec<SlicerView>, Error> {
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

fn load_all_views(pkg: &Package, sheet_part: &str) -> Result<Vec<SlicerView>, Error> {
    let mut out = load_slicer_views(pkg, sheet_part)?;
    out.extend(load_timeline_views(pkg, sheet_part)?);
    Ok(out)
}

struct CacheMeta {
    name: String,
    field: String,
    kind: &'static str,
    source_name: Option<String>,
    part: String,
}

fn load_caches(pkg: &Package) -> Result<Vec<CacheMeta>, Error> {
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
            table_display_by_id(pkg, &wb_part, table_id_of(&doc.root))
        } else {
            pivot_name_of(&doc.root)
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
            source_name: pivot_name_of(&doc.root),
            part,
        });
    }
    Ok(out)
}

fn has_local(root: &Element, local: &str) -> bool {
    let mut hit = false;
    picture::walk_all(root, &mut |e| {
        if e.local_name() == local {
            hit = true;
        }
    });
    hit
}

fn table_id_of(root: &Element) -> Option<u32> {
    let mut id = None;
    picture::walk_all(root, &mut |e| {
        if e.local_name() == "tableSlicerCache" && id.is_none() {
            id = e.attr("tableId").and_then(|s| s.parse().ok());
        }
    });
    id
}

fn pivot_name_of(root: &Element) -> Option<String> {
    let mut name = None;
    picture::walk_all(root, &mut |e| {
        if e.local_name() == "pivotTable" && name.is_none() {
            name = e.attr("name").filter(|s| !s.is_empty()).map(str::to_string);
        }
    });
    name
}

fn table_display_by_id(pkg: &Package, wb_part: &str, table_id: Option<u32>) -> Option<String> {
    let want = table_id?;
    let wb = xml::parse(pkg.get(wb_part).ok()?).ok()?;
    let rels = Relationships::from_package(pkg, wb_part).ok()?;
    let sheets = wb.root.child_by_local("sheets")?;
    for sh in sheets.children_by_local("sheet") {
        let rid = rid_attr(sh)?;
        let Ok(sheet_part) = resolve_id(pkg, wb_part, rid) else {
            continue;
        };
        let Ok(sheet) = xml::parse(pkg.get(&sheet_part).ok()?) else {
            continue;
        };
        let Some(parts) = sheet.root.child_by_local("tableParts") else {
            continue;
        };
        let sheet_rels = Relationships::from_package(pkg, &sheet_part).ok()?;
        for tp in parts.children_by_local("tablePart") {
            let Some(rid) = tp.attr_local("id") else {
                continue;
            };
            let Some(rel) = sheet_rels.find_by_id(rid) else {
                continue;
            };
            let Ok(part) = resolve_rel_target(&sheet_part, &rel.target) else {
                continue;
            };
            let Ok(doc) = xml::parse(pkg.get(&part).ok()?) else {
                continue;
            };
            let got = doc.root.attr("id").and_then(|s| s.parse::<u32>().ok());
            if got == Some(want) {
                return Some(
                    doc.root
                        .attr("displayName")
                        .or_else(|| doc.root.attr("name"))
                        .unwrap_or("")
                        .to_string(),
                );
            }
        }
        let _ = rels;
    }
    None
}

/// 读当前工作表文档上的 `tableParts`，不要从包里重解析。
/// 同一批 apply 刚插入的表还只写在 dirty 文档里。
fn table_column(
    pkg: &Package,
    sheet: &Element,
    sheet_part: &str,
    sheet_name: &str,
    index: usize,
    field: &str,
    op_index: usize,
) -> Result<(u32, String, u32), Error> {
    let table_id = format!("sheet:{sheet_name}/table:{index}");
    let parent = sheet.child_by_local("tableParts").ok_or_else(|| {
        Error::with_path(
            ErrorKind::SheetTableNotFound {
                id: table_id.clone(),
            },
            format!("ops[{op_index}].source"),
        )
    })?;
    let parts = parent.children_by_local("tablePart");
    if index == 0 || index > parts.len() {
        return Err(Error::with_path(
            ErrorKind::SheetTableNotFound { id: table_id },
            format!("ops[{op_index}].source"),
        ));
    }
    let rid = parts[index - 1].attr_local("id").ok_or_else(|| {
        Error::new(ErrorKind::XmlMissing {
            parent: "tablePart".into(),
            child: "r:id".into(),
        })
    })?;
    let rels = Relationships::from_package(pkg, sheet_part)?;
    let rel = rels.find_by_id(rid).ok_or_else(|| {
        Error::new(ErrorKind::RelNotFound {
            source: RelSource::Part(sheet_part.to_string()),
            rid: rid.to_string(),
        })
    })?;
    let part = resolve_rel_target(sheet_part, &rel.target)?;
    let doc = xml::parse(pkg.get(&part)?)?;
    let xml_id = doc
        .root
        .attr("id")
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(index as u32);
    let display = doc
        .root
        .attr("displayName")
        .or_else(|| doc.root.attr("name"))
        .unwrap_or("")
        .to_string();
    let Some(cols) = doc.root.child_by_local("tableColumns") else {
        return Err(field_err(field, op_index));
    };
    for col in cols.children_by_local("tableColumn") {
        if col.attr("name").is_some_and(|n| n == field) {
            let col_id = col
                .attr("id")
                .and_then(|s| s.parse::<u32>().ok())
                .unwrap_or(1);
            return Ok((xml_id, display, col_id));
        }
    }
    Err(field_err(field, op_index))
}

struct PivotMeta {
    name: String,
    cache_id: String,
    item_count: usize,
    part: String,
}

fn pivot_field(
    pkg: &Package,
    wb_part: &str,
    sheet_part: &str,
    sheet_name: &str,
    index: usize,
    field: &str,
    op_index: usize,
) -> Result<PivotMeta, Error> {
    let rels = Relationships::from_package(pkg, sheet_part)?;
    let mut n = 0usize;
    for rel in &rels.rels {
        if !rel.rel_type.ends_with("/pivotTable") {
            continue;
        }
        n += 1;
        if n != index {
            continue;
        }
        let part = resolve_rel_target(sheet_part, &rel.target)?;
        let doc = xml::parse(pkg.get(&part)?)?;
        let name = doc.root.attr("name").unwrap_or("").to_string();
        let cache_id = doc.root.attr("cacheId").unwrap_or("").to_string();
        let item_count = cache_item_count(pkg, wb_part, &cache_id, field);
        if !cache_has_field(pkg, wb_part, &cache_id, field) {
            return Err(field_err(field, op_index));
        }
        let _ = sheet_name;
        return Ok(PivotMeta {
            name,
            cache_id,
            item_count,
            part,
        });
    }
    Err(Error::with_path(
        ErrorKind::PivotTableNotFound {
            id: format!("sheet:{sheet_name}/pivot:{index}"),
        },
        format!("ops[{op_index}].source"),
    ))
}

fn cache_has_field(pkg: &Package, wb_part: &str, cache_id: &str, field: &str) -> bool {
    cache_field_el(pkg, wb_part, cache_id, field).is_some()
}

fn cache_item_count(pkg: &Package, wb_part: &str, cache_id: &str, field: &str) -> usize {
    let Some(cf) = cache_field_el(pkg, wb_part, cache_id, field) else {
        return 0;
    };
    cf.child_by_local("sharedItems")
        .map(|s| {
            s.element_children()
                .filter(|c| c.local_name() != "extLst")
                .count()
        })
        .unwrap_or(0)
}

/// Excel 16.0 打开时间线时，透视表 `updatedVersion` 必须至少为 8，否则会静默丢掉时间线部件。
fn bump_pivot_updated_version(pkg: &mut Package, part: &str) -> Result<(), Error> {
    const MIN: u32 = 8;
    let mut doc = xml::parse(pkg.get(part)?)?;
    let cur = doc
        .root
        .attr("updatedVersion")
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(0);
    if cur >= MIN {
        return Ok(());
    }
    doc.root.set_attr("updatedVersion", MIN.to_string());
    pkg.set(part, doc.to_bytes()?)?;
    Ok(())
}

/// 透视切片器的 `tabular/@pivotCacheId` 必须指向缓存定义上的 x14 扩展，不能只写 workbook `cacheId`。
fn ensure_x14_pivot_cache_id(
    pkg: &mut Package,
    wb_part: &str,
    workbook_cache_id: &str,
) -> Result<String, Error> {
    let part = pivot_cache_def_part(pkg, wb_part, workbook_cache_id)?;
    let mut doc = xml::parse(pkg.get(&part)?)?;
    let fallback = if workbook_cache_id.is_empty() || workbook_cache_id == "0" {
        "1".to_string()
    } else {
        workbook_cache_id.to_string()
    };
    let id = ensure_pcd_ext(&mut doc.root, &fallback);
    pkg.set(&part, doc.to_bytes()?)?;
    Ok(id)
}

fn pivot_cache_def_part(pkg: &Package, wb_part: &str, cache_id: &str) -> Result<String, Error> {
    let wb = xml::parse(pkg.get(wb_part)?)?;
    let parent = wb.root.child_by_local("pivotCaches").ok_or_else(|| {
        Error::new(ErrorKind::XmlMissing {
            parent: "workbook".into(),
            child: "pivotCaches".into(),
        })
    })?;
    let rels = Relationships::from_package(pkg, wb_part)?;
    for el in parent.children_by_local("pivotCache") {
        if el.attr("cacheId") != Some(cache_id) {
            continue;
        }
        let rid = el.attr_local("id").ok_or_else(|| {
            Error::new(ErrorKind::XmlMissing {
                parent: "pivotCache".into(),
                child: "r:id".into(),
            })
        })?;
        let rel = rels.find_by_id(rid).ok_or_else(|| {
            Error::new(ErrorKind::RelNotFound {
                source: RelSource::Part(wb_part.to_string()),
                rid: rid.to_string(),
            })
        })?;
        return resolve_rel_target(wb_part, &rel.target);
    }
    Err(Error::new(ErrorKind::XmlMissing {
        parent: "pivotCaches".into(),
        child: "pivotCache".into(),
    }))
}

fn ensure_pcd_ext(root: &mut Element, fallback: &str) -> String {
    if root.child_by_local("extLst").is_none() {
        root.push_element(Element::new("extLst"));
    }
    let ext_lst = root.child_by_local_mut("extLst").unwrap();
    let found = ext_lst.children.iter().position(|n| match n {
        Node::Element(ext) => uri_eq(ext.attr("uri").unwrap_or(""), PIVOT_CACHE_DEF_EXT_URI),
        _ => false,
    });
    if found.is_none() {
        let mut ext = Element::new("ext")
            .with_attr("uri", PIVOT_CACHE_DEF_EXT_URI)
            .with_attr("xmlns:x14", X14_NS);
        ext.push_element(
            Element::new("x14:pivotCacheDefinition").with_attr("pivotCacheId", fallback),
        );
        ext_lst.push_element(ext);
        return fallback.to_string();
    }
    let idx = found.unwrap();
    let Node::Element(ext) = &mut ext_lst.children[idx] else {
        return fallback.to_string();
    };
    picture::ensure_xmlns(ext, "x14", X14_NS);
    let has_pcd = ext
        .element_children()
        .any(|e| e.local_name() == "pivotCacheDefinition");
    if !has_pcd {
        ext.push_element(
            Element::new("x14:pivotCacheDefinition").with_attr("pivotCacheId", fallback),
        );
        return fallback.to_string();
    }
    let pcd = ext
        .element_children_mut()
        .find(|e| e.local_name() == "pivotCacheDefinition")
        .unwrap();
    let cur = pcd.attr("pivotCacheId").unwrap_or("").to_string();
    if cur.is_empty() || cur == "0" {
        pcd.set_attr("pivotCacheId", fallback);
        return fallback.to_string();
    }
    cur
}

fn cache_field_el(pkg: &Package, wb_part: &str, cache_id: &str, field: &str) -> Option<Element> {
    let wb = xml::parse(pkg.get(wb_part).ok()?).ok()?;
    let parent = wb.root.child_by_local("pivotCaches")?;
    let rels = Relationships::from_package(pkg, wb_part).ok()?;
    for el in parent.children_by_local("pivotCache") {
        if el.attr("cacheId") != Some(cache_id) {
            continue;
        }
        let rid = el.attr_local("id")?;
        let rel = rels.find_by_id(rid)?;
        let part = resolve_rel_target(wb_part, &rel.target).ok()?;
        let doc = xml::parse(pkg.get(&part).ok()?).ok()?;
        let fields = doc.root.child_by_local("cacheFields")?;
        for cf in fields.children_by_local("cacheField") {
            if cf.attr("name") == Some(field) {
                return Some(cf.clone());
            }
        }
    }
    None
}

fn sheet_id(wb: &Document, name: &str) -> Option<u32> {
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

fn unique_cache_name(pkg: &Package, wb: &Document, stem: &str) -> String {
    let mut taken = defined_names(wb);
    if let Ok(caches) = load_caches(pkg) {
        for c in caches {
            taken.push(c.name);
        }
    }
    unique_name(stem, &taken)
}

fn unique_view_name(pkg: &Package, field: &str) -> String {
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

fn defined_names(wb: &Document) -> Vec<String> {
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

fn unique_name(base: &str, taken: &[String]) -> String {
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

fn table_cache_xml(name: &str, field: &str, table_id: u32, column: u32) -> Vec<u8> {
    let mut root = Element::new("slicerCacheDefinition")
        .with_attr("xmlns", X14_NS)
        .with_attr("xmlns:mc", MC_NS)
        .with_attr("mc:Ignorable", "x")
        .with_attr("xmlns:x", SML_NS)
        .with_attr("name", name)
        .with_attr("sourceName", field);
    let mut ext = Element::new("x:ext")
        .with_attr("uri", TABLE_SLICER_CACHE_URI)
        .with_attr("xmlns:x15", X15_NS);
    ext.push_element(
        Element::new("x15:tableSlicerCache")
            .with_attr("tableId", table_id.to_string())
            .with_attr("column", column.to_string()),
    );
    let mut lst = Element::new("extLst");
    lst.push_element(ext);
    root.push_element(lst);
    Document {
        decl: Some(DEFAULT_DECL.to_string()),
        root,
    }
    .to_bytes()
    .unwrap_or_default()
}

fn pivot_cache_xml(
    name: &str,
    field: &str,
    tab_id: u32,
    pivot_name: &str,
    cache_id: &str,
    item_count: usize,
) -> Vec<u8> {
    let mut root = Element::new("slicerCacheDefinition")
        .with_attr("xmlns", X14_NS)
        .with_attr("xmlns:mc", MC_NS)
        .with_attr("mc:Ignorable", "x")
        .with_attr("xmlns:x", SML_NS)
        .with_attr("name", name)
        .with_attr("sourceName", field);
    let mut pts = Element::new("pivotTables");
    pts.push_element(
        Element::new("pivotTable")
            .with_attr("tabId", tab_id.to_string())
            .with_attr("name", pivot_name),
    );
    root.push_element(pts);
    let mut items = Element::new("items").with_attr("count", item_count.to_string());
    for i in 0..item_count {
        items.push_element(
            Element::new("i")
                .with_attr("x", i.to_string())
                .with_attr("s", "1"),
        );
    }
    let mut tabular = Element::new("tabular").with_attr("pivotCacheId", cache_id);
    tabular.push_element(items);
    let mut data = Element::new("data");
    data.push_element(tabular);
    root.push_element(data);
    Document {
        decl: Some(DEFAULT_DECL.to_string()),
        root,
    }
    .to_bytes()
    .unwrap_or_default()
}

fn timeline_cache_xml(
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
    Document {
        decl: Some(DEFAULT_DECL.to_string()),
        root,
    }
    .to_bytes()
    .unwrap_or_default()
}

fn ensure_timelines_part(
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
        Document {
            decl: Some(DEFAULT_DECL.to_string()),
            root,
        }
        .to_bytes()?,
    )?;
    let rid = add_rel(pkg, sheet_part, REL_TIMELINE, &part)?;
    picture::ensure_xmlns(&mut sheet_doc.root, "r", REL_NS);
    picture::ensure_xmlns(&mut sheet_doc.root, "mc", MC_NS);
    picture::ensure_xmlns(&mut sheet_doc.root, "x15", X15_NS);
    ensure_ignorable(&mut sheet_doc.root, &["x15"]);
    Ok((part, rid, true))
}

fn append_timeline_view(
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
    let mut doc = xml::parse(pkg.get(timeline_part)?)?;
    doc.root.push_element(el);
    pkg.set(timeline_part, doc.to_bytes()?)?;
    Ok(())
}

fn push_sheet_timeline_refs(sheet: &mut Element, rid: &str) {
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

fn push_wb_timeline_cache(wb: &mut Document, rid: &str) {
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

fn ensure_slicers_part(
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
        Document {
            decl: Some(DEFAULT_DECL.to_string()),
            root,
        }
        .to_bytes()?,
    )?;
    let rid = add_rel(pkg, sheet_part, REL_SLICER, &part)?;
    picture::ensure_xmlns(&mut sheet_doc.root, "r", REL_NS);
    picture::ensure_xmlns(&mut sheet_doc.root, "mc", MC_NS);
    picture::ensure_xmlns(&mut sheet_doc.root, "x14", X14_NS);
    picture::ensure_xmlns(&mut sheet_doc.root, "x15", X15_NS);
    ensure_ignorable(&mut sheet_doc.root, &["x14", "x15"]);
    Ok((part, rid, true))
}

fn append_slicer_view(
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
    let mut doc = xml::parse(pkg.get(slicer_part)?)?;
    doc.root.push_element(el);
    pkg.set(slicer_part, doc.to_bytes()?)?;
    Ok(())
}

fn push_sheet_list(sheet: &mut Element, rid: &str) {
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

fn push_wb_cache(wb: &mut Document, table: bool, rid: &str) {
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

fn build_slicer_ac(
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

fn two_cell(
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
enum SlicerFrameKind {
    Table,
    Pivot,
    Timeline,
}

fn build_frame(frame: SlicerFrameKind, cnv: u32, name: &str, cx: i64, cy: i64) -> Element {
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

fn fallback_sp(name: &str, cx: i64, cy: i64, text: &str) -> Element {
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

fn remove_slicer_anchor(root: &mut Element, sle_name: &str, cnv: Option<&str>) -> bool {
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

fn slicer_in(el: &Element, want_name: &str, cnv: Option<&str>) -> bool {
    let mut hit = false;
    let _ = picture::walk_anchors(el, &mut |anchor, _| {
        shape::visit_payloads(anchor, &mut |e| {
            if is_slicer_or_timeline_frame(e) {
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

fn remove_slicer_view(
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
            pkg.set(&part, doc.to_bytes()?)?;
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

fn remove_timeline_view(
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
            pkg.set(&part, doc.to_bytes()?)?;
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

fn drop_sheet_timeline_rid(sheet: &mut Element, rid: &str) {
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

fn drop_sheet_list_rid(sheet: &mut Element, rid: &str) {
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

fn cache_users(pkg: &Package, cache_name: &str) -> Result<usize, Error> {
    let wb_part = office_document_part(pkg)?;
    let wb = xml::parse(pkg.get(&wb_part)?)?;
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

fn drop_cache(
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

fn drop_wb_cache_rid(wb: &mut Element, rid: &str) {
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

/// 切片器缓存必须有同名定义名，公式为 `#N/A`（MS-XLSX 2.3.2.3）。
/// Excel 16.0 另存不写 `hidden`，也不给 `definedNames` 加 `count`；写了会打不开。
fn push_cache_defined_name(wb: &mut Document, name: &str) {
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

fn drop_cache_defined_name(wb: &mut Element, name: &str) {
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

fn add_rel(pkg: &mut Package, source: &str, rel_type: &str, target: &str) -> Result<String, Error> {
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

fn next_part(pkg: &Package, prefix: &str, ext: &str) -> String {
    let mut n = 1u32;
    loop {
        let name = format!("{prefix}{n}.{ext}");
        if !pkg.contains(&name) {
            return name;
        }
        n += 1;
    }
}

fn rid_attr(el: &Element) -> Option<&str> {
    el.attrs.iter().find_map(|(k, v)| {
        if k.contains(':') && k.rsplit(':').next() == Some("id") {
            Some(v.as_str())
        } else {
            None
        }
    })
}

fn uri_eq(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
}

fn ensure_ignorable(el: &mut Element, tokens: &[&str]) {
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

fn cache_date_bounds(
    pkg: &Package,
    wb_part: &str,
    cache_id: &str,
    field: &str,
    op_index: usize,
) -> Result<(String, String), Error> {
    let Some(cf) = cache_field_el(pkg, wb_part, cache_id, field) else {
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

fn mark_cache_field_dates(
    pkg: &mut Package,
    wb_part: &str,
    cache_id: &str,
    field: &str,
    start: &str,
    end: &str,
) -> Result<(), Error> {
    let part = pivot_cache_def_part(pkg, wb_part, cache_id)?;
    let mut doc = xml::parse(pkg.get(&part)?)?;
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
    pkg.set(&part, doc.to_bytes()?)?;
    rewrite_cache_records_dates(pkg, &part, field_index, &order)?;
    Ok(())
}

fn rewrite_cache_records_dates(
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
    let mut doc = xml::parse(pkg.get(&rec_part)?)?;
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
    pkg.set(&rec_part, doc.to_bytes()?)?;
    Ok(())
}

fn normalize_iso(raw: &str) -> String {
    let t = raw.trim();
    if t.contains('T') {
        t.to_string()
    } else if t.len() >= 10 {
        format!("{}T00:00:00", &t[..10])
    } else {
        t.to_string()
    }
}

/// Excel 1900 日期系统：序列 1 = 1900-01-01。序列 ≥ 60 时跳过伪造的 1900-02-29。
fn excel_serial_to_iso(serial: f64) -> Option<String> {
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

fn days_from_civil(y: i32, m: u32, d: u32) -> i64 {
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

fn civil_from_days(z: i64) -> (i32, u32, u32) {
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

fn field_err(field: &str, op_index: usize) -> Error {
    Error::with_path(
        ErrorKind::SlicerFieldNotFound {
            name: field.to_string(),
        },
        format!("ops[{op_index}].field"),
    )
}

fn source_id_err(op_index: usize) -> Error {
    Error::with_path(
        ErrorKind::InvalidId {
            reason: IdReason::InsertSlicerNeedsTableOrPivot,
        },
        format!("ops[{op_index}].source"),
    )
}

fn slicer_selected(pkg: &Package, cache: Option<&CacheMeta>) -> Vec<String> {
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
        return table_selected(pkg, &doc.root);
    }
    pivot_selected(pkg, &doc.root, &cache.field)
}

fn table_selected(pkg: &Package, cache_root: &Element) -> Vec<String> {
    let Some(tid) = table_id_of(cache_root) else {
        return Vec::new();
    };
    let col = table_column_id_of(cache_root);
    let Some(loc) = table_loc(pkg, tid, col) else {
        return Vec::new();
    };
    let Ok(bytes) = pkg.get(&loc.table_part) else {
        return Vec::new();
    };
    let Ok(doc) = xml::parse(bytes) else {
        return Vec::new();
    };
    filter_vals(&doc.root, loc.col_id)
}

fn pivot_selected(pkg: &Package, cache_root: &Element, field: &str) -> Vec<String> {
    let names = pivot_item_names(pkg, cache_root, field);
    let mut out = Vec::new();
    let Some(items) = find_items(cache_root) else {
        return out;
    };
    for i in items.children_by_local("i") {
        if !attr_is_true(i, "s") {
            continue;
        }
        let Some(x) = i.attr("x").and_then(|s| s.parse::<usize>().ok()) else {
            continue;
        };
        if let Some(n) = names.get(x) {
            out.push(n.clone());
        }
    }
    out
}

fn timeline_filter_json(pkg: &Package, cache: Option<&CacheMeta>) -> Value {
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

fn custom_filter_dates(root: &Element) -> (String, String) {
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

fn set_slicer_selected(
    pkg: &mut Package,
    sheet_part: &str,
    view_name: &str,
    selected: &[String],
    op_index: usize,
) -> Result<(), Error> {
    let views = load_slicer_views(pkg, sheet_part)?;
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
    if cache.kind == "table" {
        return set_table_selected(pkg, cache, selected, op_index);
    }
    set_pivot_selected(pkg, cache, selected, op_index)
}

fn set_table_selected(
    pkg: &mut Package,
    cache: &CacheMeta,
    selected: &[String],
    op_index: usize,
) -> Result<(), Error> {
    let bytes = pkg.get(&cache.part)?;
    let doc = xml::parse(bytes)?;
    let Some(tid) = table_id_of(&doc.root) else {
        return Err(unknown_selected(op_index));
    };
    let col = table_column_id_of(&doc.root);
    let Some(loc) = table_loc(pkg, tid, col) else {
        return Err(unknown_selected(op_index));
    };
    let allowed = table_column_values(pkg, &loc);
    for (k, name) in selected.iter().enumerate() {
        if !allowed.iter().any(|a| a == name) {
            return Err(Error::with_path(
                ErrorKind::InvalidSlicer {
                    field: "selected".into(),
                },
                format!("ops[{op_index}].selected[{k}]"),
            ));
        }
    }
    let mut table = xml::parse(pkg.get(&loc.table_part)?)?;
    write_table_filters(&mut table.root, loc.col_id, selected);
    pkg.set(&loc.table_part, table.to_bytes()?)?;
    Ok(())
}

fn set_pivot_selected(
    pkg: &mut Package,
    cache: &CacheMeta,
    selected: &[String],
    op_index: usize,
) -> Result<(), Error> {
    let mut doc = xml::parse(pkg.get(&cache.part)?)?;
    let names = pivot_item_names(pkg, &doc.root, &cache.field);
    let mut want = Vec::new();
    for (k, name) in selected.iter().enumerate() {
        let Some(x) = names.iter().position(|n| n == name) else {
            return Err(Error::with_path(
                ErrorKind::InvalidSlicer {
                    field: "selected".into(),
                },
                format!("ops[{op_index}].selected[{k}]"),
            ));
        };
        if !want.contains(&x) {
            want.push(x);
        }
    }
    let Some(items) = find_items_mut(&mut doc.root) else {
        if selected.is_empty() {
            return Ok(());
        }
        return Err(unknown_selected(op_index));
    };
    for el in items.element_children_mut() {
        if el.local_name() != "i" {
            continue;
        }
        let Some(x) = el.attr("x").and_then(|s| s.parse::<usize>().ok()) else {
            continue;
        };
        if want.contains(&x) {
            el.set_attr("s", "1");
        } else {
            el.remove_attr("s");
        }
    }
    for x in &want {
        let exists = items.element_children().any(|el| {
            el.local_name() == "i" && el.attr("x").and_then(|s| s.parse::<usize>().ok()) == Some(*x)
        });
        if !exists {
            items.push_element(
                Element::new("i")
                    .with_attr("x", x.to_string())
                    .with_attr("s", "1"),
            );
        }
    }
    let n = items.children_by_local("i").len();
    items.set_attr("count", n.to_string());
    pkg.set(&cache.part, doc.to_bytes()?)?;
    Ok(())
}

fn set_timeline_filter(
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
    let mut doc = xml::parse(pkg.get(&cache.part)?)?;
    match filter {
        None => clear_timeline_filter(&mut doc.root),
        Some(f) => write_timeline_filter(&mut doc.root, f),
    }
    pkg.set(&cache.part, doc.to_bytes()?)?;
    Ok(())
}

fn clear_timeline_filter(root: &mut Element) {
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

fn write_timeline_filter(root: &mut Element, filter: &TimelineFilter) {
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

fn unknown_selected(op_index: usize) -> Error {
    Error::with_path(
        ErrorKind::InvalidSlicer {
            field: "selected".into(),
        },
        format!("ops[{op_index}].selected"),
    )
}

fn table_column_id_of(root: &Element) -> Option<u32> {
    let mut id = None;
    picture::walk_all(root, &mut |e| {
        if e.local_name() == "tableSlicerCache" && id.is_none() {
            id = e.attr("column").and_then(|s| s.parse().ok());
        }
    });
    id
}

struct TableLoc {
    table_part: String,
    sheet_part: String,
    col_id: u32,
    start: a1::CellRef,
    end: a1::CellRef,
}

fn table_loc(pkg: &Package, table_id: u32, column: Option<u32>) -> Option<TableLoc> {
    let wb_part = office_document_part(pkg).ok()?;
    let wb = xml::parse(pkg.get(&wb_part).ok()?).ok()?;
    let sheets = wb.root.child_by_local("sheets")?;
    for sh in sheets.children_by_local("sheet") {
        let rid = rid_attr(sh)?;
        let Ok(sheet_part) = resolve_id(pkg, &wb_part, rid) else {
            continue;
        };
        let Ok(sheet) = xml::parse(pkg.get(&sheet_part).ok()?) else {
            continue;
        };
        let Some(parts) = sheet.root.child_by_local("tableParts") else {
            continue;
        };
        let sheet_rels = Relationships::from_package(pkg, &sheet_part).ok()?;
        for tp in parts.children_by_local("tablePart") {
            let Some(rid) = tp.attr_local("id") else {
                continue;
            };
            let Some(rel) = sheet_rels.find_by_id(rid) else {
                continue;
            };
            let Ok(part) = resolve_rel_target(&sheet_part, &rel.target) else {
                continue;
            };
            let Ok(doc) = xml::parse(pkg.get(&part).ok()?) else {
                continue;
            };
            let got = doc.root.attr("id").and_then(|s| s.parse::<u32>().ok());
            if got != Some(table_id) {
                continue;
            }
            let refer = doc.root.attr("ref")?;
            let (start, end) = a1::parse_range(refer).ok()?;
            let cols = doc.root.child_by_local("tableColumns")?;
            let list = cols.children_by_local("tableColumn");
            let col_id = if let Some(want) = column {
                list.iter()
                    .position(|c| c.attr("id").and_then(|s| s.parse::<u32>().ok()) == Some(want))
                    .unwrap_or(0) as u32
            } else {
                0
            };
            return Some(TableLoc {
                table_part: part,
                sheet_part,
                col_id,
                start,
                end,
            });
        }
    }
    None
}

fn table_column_values(pkg: &Package, loc: &TableLoc) -> Vec<String> {
    let Ok(wb_part) = office_document_part(pkg) else {
        return Vec::new();
    };
    let Ok(rels) = Relationships::from_package(pkg, &wb_part) else {
        return Vec::new();
    };
    let sst = super::inspect::load_shared_strings(pkg, &wb_part, &rels).unwrap_or_default();
    let Ok(bytes) = pkg.get(&loc.sheet_part) else {
        return Vec::new();
    };
    let Ok(sheet) = xml::parse(bytes) else {
        return Vec::new();
    };
    let col = loc.start.col + loc.col_id;
    let mut out = Vec::new();
    for row in (loc.start.row + 1)..=loc.end.row {
        let raw = sheet_cell_text(&sheet.root, a1::CellRef { col, row }, &sst);
        if raw.is_empty() {
            continue;
        }
        if !out.iter().any(|s| s == &raw) {
            out.push(raw);
        }
    }
    out
}

fn sheet_cell_text(root: &Element, cell: a1::CellRef, sst: &[String]) -> String {
    let Some(sd) = root.child_by_local("sheetData") else {
        return String::new();
    };
    let want = a1::format_cell(cell.col, cell.row);
    for row in sd.children_by_local("row") {
        for c in row.children_by_local("c") {
            if c.attr("r") != Some(want.as_str()) {
                continue;
            }
            return match c.attr("t").unwrap_or("n") {
                "s" => c
                    .child_by_local("v")
                    .and_then(|v| v.text_content().parse::<usize>().ok())
                    .and_then(|i| sst.get(i).cloned())
                    .unwrap_or_default(),
                "inlineStr" => c
                    .child_by_local("is")
                    .map(|is| is.text_content())
                    .unwrap_or_default(),
                _ => c
                    .child_by_local("v")
                    .map(|v| v.text_content())
                    .unwrap_or_default(),
            };
        }
    }
    String::new()
}

fn filter_vals(table_root: &Element, col_id: u32) -> Vec<String> {
    let Some(af) = table_root.child_by_local("autoFilter") else {
        return Vec::new();
    };
    let want = col_id.to_string();
    for fc in af.children_by_local("filterColumn") {
        if fc.attr("colId") != Some(want.as_str()) {
            continue;
        }
        let mut out = Vec::new();
        if let Some(filters) = fc.child_by_local("filters") {
            for f in filters.children_by_local("filter") {
                if let Some(v) = f.attr("val") {
                    out.push(v.to_string());
                }
            }
        }
        return out;
    }
    Vec::new()
}

fn write_table_filters(table_root: &mut Element, col_id: u32, selected: &[String]) {
    let refer = table_root.attr("ref").unwrap_or("A1").to_string();
    if table_root.child_by_local("autoFilter").is_none() {
        table_root.push_element(Element::new("autoFilter").with_attr("ref", refer));
    }
    let Some(af) = table_root.child_by_local_mut("autoFilter") else {
        return;
    };
    let want = col_id.to_string();
    af.children.retain(|n| match n {
        Node::Element(e) => {
            !(e.local_name() == "filterColumn" && e.attr("colId") == Some(want.as_str()))
        }
        _ => true,
    });
    if selected.is_empty() {
        return;
    }
    let mut filters = Element::new("filters");
    for v in selected {
        filters.push_element(Element::new("filter").with_attr("val", v.clone()));
    }
    let mut col = Element::new("filterColumn").with_attr("colId", want);
    col.push_element(filters);
    af.push_element(col);
}

fn pivot_item_names(pkg: &Package, cache_root: &Element, field: &str) -> Vec<String> {
    let Ok(wb_part) = office_document_part(pkg) else {
        return Vec::new();
    };
    let mut cache_id = String::new();
    picture::walk_all(cache_root, &mut |e| {
        if e.local_name() == "tabular" && cache_id.is_empty() {
            if let Some(v) = e.attr("pivotCacheId") {
                cache_id = v.to_string();
            }
        }
    });
    let Some(cf) = cache_field_by_x14(pkg, &wb_part, &cache_id, field)
        .or_else(|| cache_field_el(pkg, &wb_part, &cache_id, field))
    else {
        return Vec::new();
    };
    let Some(shared) = cf.child_by_local("sharedItems") else {
        return Vec::new();
    };
    shared
        .element_children()
        .filter(|c| c.local_name() != "extLst")
        .map(shared_item_text)
        .collect()
}

fn shared_item_text(el: &Element) -> String {
    match el.local_name() {
        "m" => String::new(),
        _ => el.attr("v").unwrap_or("").to_string(),
    }
}

fn cache_field_by_x14(pkg: &Package, wb_part: &str, x14_id: &str, field: &str) -> Option<Element> {
    if x14_id.is_empty() {
        return None;
    }
    let wb = xml::parse(pkg.get(wb_part).ok()?).ok()?;
    let parent = wb.root.child_by_local("pivotCaches")?;
    let rels = Relationships::from_package(pkg, wb_part).ok()?;
    for el in parent.children_by_local("pivotCache") {
        let rid = el.attr_local("id")?;
        let rel = rels.find_by_id(rid)?;
        let part = resolve_rel_target(wb_part, &rel.target).ok()?;
        let doc = xml::parse(pkg.get(&part).ok()?).ok()?;
        let mut found = false;
        picture::walk_all(&doc.root, &mut |e| {
            if e.local_name() == "pivotCacheDefinition" && e.attr("pivotCacheId") == Some(x14_id) {
                found = true;
            }
        });
        if !found {
            continue;
        }
        let fields = doc.root.child_by_local("cacheFields")?;
        for cf in fields.children_by_local("cacheField") {
            if cf.attr("name") == Some(field) {
                return Some(cf.clone());
            }
        }
    }
    None
}

fn find_items(root: &Element) -> Option<&Element> {
    let data = root.child_by_local("data")?;
    let tabular = data.child_by_local("tabular")?;
    tabular.child_by_local("items")
}

fn find_items_mut(root: &mut Element) -> Option<&mut Element> {
    let data = root.child_by_local_mut("data")?;
    let tabular = data.child_by_local_mut("tabular")?;
    tabular.child_by_local_mut("items")
}

fn attr_is_true(el: &Element, name: &str) -> bool {
    matches!(el.attr(name), Some("1" | "true" | "TRUE"))
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
