//! 工作表切片器与时间线：inspect 名单，apply 写 slicer / timeline 部件与 drawing graphicFrame。
//! 不在库里执行筛选，不刷新透视合计。

use serde_json::Value;

use crate::error::{Error, ErrorKind, IdReason};
use crate::opc::Package;
use crate::xml::{Document, Element};

use super::a1;
use super::picture::{self, AnchorKind, EditAs};
use super::shape;

mod common;
mod pivot;
mod table;
mod timeline;
use common::*;

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
            timeline::insert_timeline(
                pkg, wb_doc, wb_part, sheet_doc, sheet_part, op, *index, op_index,
            )
        }
        (SlicerKind::Slicer, SlicerSource::Table { sheet, index }) => {
            if !sheet.eq_ignore_ascii_case(&op.sheet) {
                return Err(source_id_err(op_index));
            }
            table::insert_table(
                pkg, wb_doc, wb_part, sheet_doc, sheet_part, op, *index, op_index,
            )
        }
        (SlicerKind::Slicer, SlicerSource::Pivot { sheet, index }) => {
            if !sheet.eq_ignore_ascii_case(&op.sheet) {
                return Err(source_id_err(op_index));
            }
            pivot::insert_pivot(
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
        let mut draw = super::list_pin::parse_part(pkg, &drawing_part)?;
        remove_slicer_anchor(&mut draw.root, &hit.sle_name, hit.cnv.as_deref());
        shape::write_or_drop_drawing(pkg, sheet_doc, sheet_part, &drawing_part, draw)?;
    }

    if timeline {
        timeline::remove_timeline_view(pkg, sheet_doc, sheet_part, &view_name)?;
    } else {
        remove_slicer_view(pkg, sheet_doc, sheet_part, &view_name)?;
    }
    if !cache_name.is_empty() && cache_users(pkg, &cache_name)? == 0 {
        drop_cache(pkg, wb_doc, wb_part, &cache_name)?;
    }
    Ok(())
}

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
        return timeline::set_timeline_filter(
            pkg,
            sheet_part,
            &hit.sle_name,
            filter.as_ref(),
            op_index,
        );
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

pub(crate) fn is_slicer_or_timeline_frame(el: &Element) -> bool {
    if el.local_name() != "graphicFrame" {
        return false;
    }
    graphic_uri(el).is_some_and(|u| is_slicer_uri(&u) || is_timeslicer_uri(&u))
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
        return table::set_table_selected(pkg, cache, selected, op_index);
    }
    pivot::set_pivot_selected(pkg, cache, selected, op_index)
}
