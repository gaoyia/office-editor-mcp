//! 工作表 SpreadsheetDrawing 图表：名单、插删、改系列与锚点。数据写回本工作簿单元格。

use serde_json::{json, Map, Value};

use crate::chart::{self, ChartRestrict, ChartType, CHARTEX_NS, CHART_NS};
use crate::error::{Error, ErrorKind, IdReason};
use crate::opc::{
    rels_part_name, resolve_rel_target, same_part_name, ContentTypes, Package, Relationship,
    Relationships,
};
use crate::xml::{self, Document, Element, Node};

use super::a1::{self, CellRef};
use super::apply::{self, write_json_cell};
use super::inspect;
use super::picture::{self, AnchorKind, EditAs};
use super::request::{
    AddChartSeriesOp, DeleteChartOp, DeleteChartSeriesOp, DeleteErrorBarsOp, DeleteTrendlineOp,
    InsertChartOp, InsertErrorBarsOp, InsertTrendlineOp, SetChartAnchorOp, SetChartAxisTitleOp,
    SetChartDataLabelsOp, SetChartLegendOp, SetChartSeriesAxisOp, SetChartSeriesOp, SetChartSizeOp,
    SetChartTitleOp, SetChartTypeOp, SetErrorBarsOp, SetTrendlineOp,
};
use super::REL_NS;

#[derive(Debug, Clone)]
pub enum ChartKey {
    Cnv(String),
    Index(usize),
}

#[derive(Debug, Clone)]
pub struct SheetChartId {
    pub sheet: String,
    pub key: ChartKey,
}

pub struct ChartInfo {
    pub id: String,
    pub cnv_pr_id: Option<String>,
    pub name: String,
    pub anchor: AnchorKind,
    pub from: Option<String>,
    pub from_off_x: Option<i64>,
    pub from_off_y: Option<i64>,
    pub to: Option<String>,
    pub to_off_x: Option<i64>,
    pub to_off_y: Option<i64>,
    pub pos_x: Option<i64>,
    pub pos_y: Option<i64>,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
    pub edit_as: Option<EditAs>,
    pub chart_type: String,
    pub title: Value,
    pub cat_axis_title: Value,
    pub val_axis_title: Value,
    pub series: Vec<Value>,
    pub legend: Value,
    pub data_labels: Value,
    pub flags: Vec<Value>,
    pub stable: bool,
    pub of_pie_type: Option<String>,
    pub stock_kind: Option<String>,
    pub surface_kind: Option<String>,
    pub layout_id: Option<String>,
}

pub fn parse_insert_sheet_id(id: &str, op_index: usize) -> Result<String, Error> {
    if id.contains("/chart:") || id.contains("/pic:") || id.starts_with("cell:") {
        return Err(invalid_id(IdReason::InsertChartNeedsSheet, op_index));
    }
    let Some(name) = id.strip_prefix("sheet:") else {
        return Err(invalid_id(IdReason::InsertChartNeedsSheet, op_index));
    };
    if name.is_empty() || name.contains('/') {
        return Err(invalid_id(IdReason::InsertChartNeedsSheet, op_index));
    }
    Ok(name.to_string())
}

pub fn parse_chart_id(id: &str, op_index: usize) -> Result<SheetChartId, Error> {
    parse_chart_id_at(id, &format!("ops[{op_index}].id"))
}

pub fn parse_chart_id_at(id: &str, field_path: &str) -> Result<SheetChartId, Error> {
    if let Some(name) = id.strip_prefix("sheet:") {
        if let Some((sheet, rest)) = name.split_once("/chart:") {
            if sheet.is_empty() || rest.is_empty() || rest.contains("/series:") {
                return Err(Error::with_path(
                    ErrorKind::InvalidId {
                        reason: IdReason::SheetChartPrefix,
                    },
                    field_path,
                ));
            }
            let key = if let Some(idx) = rest.strip_prefix("index:") {
                let n: usize = idx.parse().map_err(|_| {
                    Error::with_path(
                        ErrorKind::InvalidId {
                            reason: IdReason::ChartIndexNotInt,
                        },
                        field_path,
                    )
                })?;
                if n == 0 {
                    return Err(Error::with_path(
                        ErrorKind::InvalidId {
                            reason: IdReason::ChartIndexFromOne,
                        },
                        field_path,
                    ));
                }
                ChartKey::Index(n)
            } else {
                ChartKey::Cnv(rest.to_string())
            };
            return Ok(SheetChartId {
                sheet: sheet.to_string(),
                key,
            });
        }
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SheetChartContainer,
            },
            field_path,
        ));
    }
    Err(Error::with_path(
        ErrorKind::InvalidId {
            reason: IdReason::SheetChartPrefix,
        },
        field_path,
    ))
}

pub fn parse_series_id(id: &str, op_index: usize) -> Result<(SheetChartId, usize), Error> {
    let path = format!("ops[{op_index}].id");
    let Some((chart, rest)) = id.rsplit_once("/series:") else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SeriesPrefix,
            },
            path,
        ));
    };
    let index: usize = rest.parse().map_err(|_| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SeriesIndexNotInt,
            },
            path.clone(),
        )
    })?;
    let target = parse_chart_id_at(chart, &path)?;
    Ok((target, index))
}

pub fn parse_trendline_id(
    id: &str,
    op_index: usize,
) -> Result<(SheetChartId, usize, usize), Error> {
    let path = format!("ops[{op_index}].id");
    let (left, k) = crate::chart::remain::parse_member_id(
        id,
        "trendline",
        crate::chart::remain::IdFromOne::Trendline,
    )
    .map_err(|reason| Error::with_path(ErrorKind::InvalidId { reason }, path))?;
    let (target, index) = parse_series_id(left, op_index)?;
    Ok((target, index, k))
}

pub fn parse_error_bars_id(
    id: &str,
    op_index: usize,
) -> Result<(SheetChartId, usize, usize), Error> {
    let path = format!("ops[{op_index}].id");
    let (left, k) = crate::chart::remain::parse_member_id(
        id,
        "errorBars",
        crate::chart::remain::IdFromOne::ErrorBars,
    )
    .map_err(|reason| Error::with_path(ErrorKind::InvalidId { reason }, path))?;
    let (target, index) = parse_series_id(left, op_index)?;
    Ok((target, index, k))
}

fn invalid_id(reason: IdReason, op_index: usize) -> Error {
    Error::with_path(
        ErrorKind::InvalidId { reason },
        format!("ops[{op_index}].id"),
    )
}

pub fn collect_sheet_charts(
    pkg: &Package,
    sheet_part: &str,
    sheet_name: &str,
    sst: &[String],
    sheets: &[(String, String)],
) -> Result<Vec<ChartInfo>, Error> {
    let mut out = Vec::new();
    let mut index = 0usize;
    for drawing_part in picture::drawing_parts(pkg, sheet_part)? {
        let doc = xml::parse(pkg.get(&drawing_part)?)?;
        picture::walk_anchors(&doc.root, &mut |anchor, kind| {
            let Some(frame) = direct_chart_frame(anchor) else {
                return Ok(());
            };
            index += 1;
            out.push(chart_info(
                pkg,
                &drawing_part,
                anchor,
                kind,
                frame,
                sheet_name,
                index,
                sst,
                sheets,
            )?);
            Ok(())
        })?;
    }
    Ok(out)
}

pub fn charts_json(charts: &[ChartInfo]) -> Vec<Value> {
    charts.iter().map(chart_to_json).collect()
}

fn chart_to_json(c: &ChartInfo) -> Value {
    let mut m = Map::new();
    m.insert("id".into(), json!(c.id));
    m.insert("cNvPrId".into(), json!(c.cnv_pr_id));
    m.insert("name".into(), json!(c.name));
    m.insert("anchor".into(), json!(c.anchor.as_str()));
    match c.anchor {
        AnchorKind::TwoCell => {
            m.insert("from".into(), json!(c.from));
            m.insert("fromOffX".into(), json!(c.from_off_x.unwrap_or(0)));
            m.insert("fromOffY".into(), json!(c.from_off_y.unwrap_or(0)));
            m.insert("to".into(), json!(c.to));
            m.insert("toOffX".into(), json!(c.to_off_x.unwrap_or(0)));
            m.insert("toOffY".into(), json!(c.to_off_y.unwrap_or(0)));
            m.insert(
                "editAs".into(),
                json!(c.edit_as.unwrap_or(EditAs::TwoCell).as_str()),
            );
        }
        AnchorKind::OneCell => {
            m.insert("from".into(), json!(c.from));
            m.insert("fromOffX".into(), json!(c.from_off_x.unwrap_or(0)));
            m.insert("fromOffY".into(), json!(c.from_off_y.unwrap_or(0)));
            m.insert("cx".into(), json!(c.cx));
            m.insert("cy".into(), json!(c.cy));
        }
        AnchorKind::Absolute => {
            m.insert("posX".into(), json!(c.pos_x));
            m.insert("posY".into(), json!(c.pos_y));
            m.insert("cx".into(), json!(c.cx));
            m.insert("cy".into(), json!(c.cy));
        }
        AnchorKind::RelSize | AnchorKind::AbsSize => {}
    }
    m.insert("type".into(), json!(c.chart_type));
    if let Some(t) = &c.of_pie_type {
        m.insert("ofPieType".into(), json!(t));
    }
    if let Some(k) = &c.stock_kind {
        m.insert("stockKind".into(), json!(k));
    }
    if let Some(k) = &c.surface_kind {
        m.insert("surfaceKind".into(), json!(k));
    }
    if let Some(id) = &c.layout_id {
        m.insert("layoutId".into(), json!(id));
    }
    m.insert("title".into(), c.title.clone());
    m.insert("catAxisTitle".into(), c.cat_axis_title.clone());
    m.insert("valAxisTitle".into(), c.val_axis_title.clone());
    m.insert("series".into(), Value::Array(c.series.clone()));
    m.insert("legend".into(), c.legend.clone());
    m.insert("dataLabels".into(), c.data_labels.clone());
    m.insert("flags".into(), json!(c.flags.clone()));
    m.insert("stable".into(), json!(c.stable));
    Value::Object(m)
}

fn chart_info(
    pkg: &Package,
    drawing_part: &str,
    anchor: &Element,
    kind: AnchorKind,
    frame: &Element,
    sheet_name: &str,
    index: usize,
    sst: &[String],
    sheets: &[(String, String)],
) -> Result<ChartInfo, Error> {
    let (raw_id, name) = cnv_of_frame(frame);
    let stable = raw_id.is_some();
    let id = match &raw_id {
        Some(v) => format!("sheet:{sheet_name}/chart:{v}"),
        None => format!("sheet:{sheet_name}/chart:index:{index}"),
    };
    let (from, from_off_x, from_off_y) = picture::read_marker(anchor.child_by_local("from"))?;
    let (to, to_off_x, to_off_y) = picture::read_marker(anchor.child_by_local("to"))?;
    let (cx, cy) = picture::read_ext(anchor);
    let (pos_x, pos_y) = picture::read_pos(anchor);
    let chartex = is_chartex(frame);
    let mut ty = if chartex {
        "chartex".to_string()
    } else {
        "other".to_string()
    };
    let mut title = Value::Null;
    let mut cat_axis = Value::Null;
    let mut val_axis = Value::Null;
    let mut series = Vec::new();
    let mut legend = json!({ "show": false, "pos": null });
    let mut data_labels = json!({ "show": false });
    let mut flags = Vec::new();
    let mut of_pie_type = None;
    let mut stock_kind = None;
    let mut surface_kind = None;
    let mut layout_id = None;
    if let Some(rid) = chart_rid(frame) {
        if let Ok(chart_part) = crate::opc::resolve_id(pkg, drawing_part, &rid) {
            if let Ok(bytes) = pkg.get(&chart_part) {
                if let Ok(doc) = xml::parse(bytes) {
                    if !chartex {
                        ty = chart::inspect_type(&doc.root, false);
                    }
                    title = json!(chart::chart_title(&doc.root));
                    let (c, v) = chart::axis_titles(&doc.root, chart::uses_xy_axes(&ty));
                    cat_axis = json!(c);
                    val_axis = json!(v);
                    series = chart::series_json(&id, &doc.root, |f| {
                        read_formula_cells(pkg, f, sheet_name, sst, sheets)
                    })?;
                    legend = chart::legend_json(&doc.root);
                    data_labels = json!({ "show": chart::data_labels_on(&doc.root) });
                    flags = chart::flags_json(&doc.root, &ty);
                    if ty == "ofPie" {
                        of_pie_type = chart::inspect_of_pie_type(&doc.root);
                    }
                    if ty == "stock" {
                        stock_kind = Some(chart::inspect_stock_kind(&doc.root));
                    }
                    if ty == "surface" {
                        surface_kind = Some(chart::inspect_surface_kind(&doc.root));
                    }
                    if ty == "chartex" {
                        layout_id = chart::inspect_layout_id(&doc.root);
                    }
                }
            }
        }
    }
    Ok(ChartInfo {
        id,
        cnv_pr_id: raw_id,
        name,
        anchor: kind,
        from: from.map(|c| a1::format_cell(c.col, c.row)),
        from_off_x: if kind == AnchorKind::Absolute {
            None
        } else {
            Some(from_off_x)
        },
        from_off_y: if kind == AnchorKind::Absolute {
            None
        } else {
            Some(from_off_y)
        },
        to: to.map(|c| a1::format_cell(c.col, c.row)),
        to_off_x: if kind == AnchorKind::TwoCell {
            Some(to_off_x)
        } else {
            None
        },
        to_off_y: if kind == AnchorKind::TwoCell {
            Some(to_off_y)
        } else {
            None
        },
        pos_x,
        pos_y,
        cx: if kind == AnchorKind::TwoCell {
            None
        } else {
            cx
        },
        cy: if kind == AnchorKind::TwoCell {
            None
        } else {
            cy
        },
        edit_as: if kind == AnchorKind::TwoCell {
            Some(picture::parse_edit_as(anchor.attr("editAs")))
        } else {
            None
        },
        chart_type: ty,
        title,
        cat_axis_title: cat_axis,
        val_axis_title: val_axis,
        series,
        legend,
        data_labels,
        flags,
        stable,
        of_pie_type,
        stock_kind,
        surface_kind,
        layout_id,
    })
}

fn read_formula_cells(
    pkg: &Package,
    formula: &str,
    default_sheet: &str,
    sst: &[String],
    sheets: &[(String, String)],
) -> Option<Vec<Value>> {
    let (names, order) = formula_ctx(pkg);
    let target = chart::resolve_chart_formula(formula, &names, &order, default_sheet)?;
    let mut out = Vec::new();
    let mut docs: Vec<(String, Document)> = Vec::new();
    for (sheet_opt, cell) in target.cells() {
        let sheet = sheet_opt.as_deref().unwrap_or(default_sheet);
        let part = sheets
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(sheet))
            .map(|(_, p)| p.as_str())?;
        if !docs.iter().any(|(p, _)| p == part) {
            docs.push((part.to_string(), xml::parse(pkg.get(part).ok()?).ok()?));
        }
        let doc = docs.iter().find(|(p, _)| p == part).map(|(_, d)| d)?;
        let sheet_data = doc.root.child_by_local("sheetData")?;
        out.push(cell_cached(sheet_data, cell, sheet, sst).unwrap_or(Value::Null));
    }
    Some(out)
}

fn formula_ctx(pkg: &Package) -> (Vec<chart::DefinedName>, Vec<String>) {
    let Ok(wb_part) = crate::opc::office_document_part(pkg) else {
        return (Vec::new(), Vec::new());
    };
    let Ok(bytes) = pkg.get(&wb_part) else {
        return (Vec::new(), Vec::new());
    };
    let Ok(wb) = xml::parse(bytes) else {
        return (Vec::new(), Vec::new());
    };
    let names = chart::parse_defined_names(&wb.root);
    let sheets = wb
        .root
        .child_by_local("sheets")
        .map(|s| {
            s.element_children()
                .filter(|c| c.local_name() == "sheet")
                .filter_map(|c| c.attr_local("name").map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    (names, sheets)
}

fn cell_cached(sheet_data: &Element, cell: CellRef, sheet: &str, sst: &[String]) -> Option<Value> {
    let c = apply::find_cell(sheet_data, cell)?;
    let t = c.attr("t").unwrap_or("");
    let (ty, value) = inspect::classify(
        t,
        c.child_by_local("v"),
        c.child_by_local("is"),
        c.child_by_local("f").is_some(),
        sst,
        &format!("{}!{}", sheet, a1::format_cell(cell.col, cell.row)),
    )
    .ok()?;
    if ty == "empty" {
        Some(Value::Null)
    } else {
        Some(value)
    }
}

pub fn insert_chart(
    pkg: &mut Package,
    sheet_part: &str,
    sheet_name: &str,
    dirty_sheets: &mut Vec<(String, Document)>,
    op: &InsertChartOp,
    op_index: usize,
) -> Result<(), Error> {
    let series = chart::normalize_insert_series(op.chart_type, &op.series);
    let cats = chart::shared_categories(&series);
    let cells = chart::data_table_cells(op.chart_type, &series, &cats, op.data_from);
    apply::load_dirty(pkg, dirty_sheets, sheet_part, true)?;
    {
        let doc = apply::dirty_of(dirty_sheets, sheet_part);
        for (cell, value) in &cells {
            write_json_cell(&mut doc.root, *cell, value, op_index)?;
        }
    }
    let xml = chart::build_chart_xml(
        op.chart_type,
        op.title.as_deref(),
        &series,
        &cats,
        sheet_name,
        op.data_from,
        false,
        op.of_pie_type.as_deref(),
        &op.extra,
    );
    let chart_part = next_chart_part_for(pkg, op.chart_type);
    pkg.set(&chart_part, xml.into_bytes())?;
    let drawing_part = {
        let doc = apply::dirty_of(dirty_sheets, sheet_part);
        picture::ensure_drawing_part(pkg, doc, sheet_part)?
    };
    let mut draw_doc = xml::parse(pkg.get(&drawing_part)?)?;
    picture::ensure_xmlns(&mut draw_doc.root, "r", REL_NS);
    picture::ensure_xmlns(&mut draw_doc.root, "a", picture::A_NS);
    let (rel_ty, ct, _) = chart::rel_and_ct(op.chart_type);
    let rid = add_chart_rel(pkg, &drawing_part, &chart_part, rel_ty)?;
    let mut cts = ContentTypes::from_package(pkg)?;
    cts.set_override(&chart_part, ct)?;
    cts.set_override(&drawing_part, picture::CT_DRAWING)?;
    if op.chart_type == ChartType::Chartex {
        let mut chart_rels = Relationships { rels: Vec::new() };
        chart::chartex::attach_style(pkg, &chart_part, &mut chart_rels, &mut cts)?;
        chart_rels.write_to(pkg, &chart_part)?;
    }
    cts.write_to(pkg)?;
    let cnv = {
        let doc = apply::dirty_of(dirty_sheets, sheet_part);
        picture::next_cnv_id(pkg, sheet_part, &doc.root)?
    };
    let (cx, cy) = (
        op.cx.unwrap_or(chart::DEFAULT_CX),
        op.cy.unwrap_or(chart::DEFAULT_CY),
    );
    let frame = build_graphic_frame(
        cnv,
        &format!("图表 {cnv}"),
        &rid,
        cx,
        cy,
        op.chart_type == ChartType::Chartex,
    );
    let anchor = picture::build_anchor(
        op.anchor,
        op.from,
        op.to,
        op.from_off_x,
        op.from_off_y,
        op.to_off_x,
        op.to_off_y,
        None,
        None,
        cx,
        cy,
        EditAs::TwoCell,
        cnv,
        &format!("图表 {cnv}"),
        &rid,
        Some(frame),
    );
    draw_doc.root.push_element(anchor);
    pkg.set(&drawing_part, draw_doc.to_bytes()?)?;
    Ok(())
}

pub fn delete_chart(
    pkg: &mut Package,
    sheet_doc: &mut Document,
    sheet_part: &str,
    sheet_kind: &str,
    op: &DeleteChartOp,
    op_index: usize,
) -> Result<(), Error> {
    let path = format!("ops[{op_index}].id");
    if sheet_kind == "chartsheet" {
        let n = count_charts(pkg, sheet_part)?;
        if n <= 1 {
            return Err(Error::with_path(ErrorKind::DeleteChartsheetLast, path));
        }
    }
    let drawing_part = find_drawing_for(pkg, sheet_part, &op.target, &op.id, &path)?;
    let mut doc = xml::parse(pkg.get(&drawing_part)?)?;
    let rid = find_chart_rid(&doc.root, &op.target.key).ok_or_else(|| {
        Error::with_path(ErrorKind::ChartNotFound { id: op.id.clone() }, path.clone())
    })?;
    let chart_part = crate::opc::resolve_id(pkg, &drawing_part, &rid).ok();
    if !remove_chart_anchor(&mut doc.root, &op.target.key) {
        return Err(Error::with_path(
            ErrorKind::ChartNotFound { id: op.id.clone() },
            path,
        ));
    }
    if picture::drawing_is_empty(&doc.root) {
        pkg.remove(&drawing_part);
        pkg.remove(&rels_part_name(&drawing_part));
        picture::remove_drawing_ref(&mut sheet_doc.root, sheet_part, pkg, &drawing_part)?;
        let mut cts = ContentTypes::from_package(pkg)?;
        cts.remove_override(&drawing_part);
        cts.write_to(pkg)?;
    } else {
        pkg.set(&drawing_part, doc.to_bytes()?)?;
        let mut rels = Relationships::from_package(pkg, &drawing_part)?;
        rels.remove_by_id(&rid);
        if rels.rels.is_empty() {
            pkg.remove(&rels_part_name(&drawing_part));
        } else {
            rels.write_to(pkg, &drawing_part)?;
        }
    }
    if let Some(part) = chart_part {
        if !chart_still_used(pkg, &part)? {
            remove_chart_tree(pkg, &part)?;
        }
    }
    Ok(())
}

pub fn set_chart_title(
    pkg: &mut Package,
    sheet_part: &str,
    op: &SetChartTitleOp,
    op_index: usize,
) -> Result<(), Error> {
    let (mut space, chart_part) = load_space(pkg, sheet_part, &op.target, &op.id, op_index)?;
    chart::reject_restricted(&space.root, &op.id, op_index, ChartRestrict::Decorate)?;
    chart::set_chart_title(&mut space.root, &op.title);
    pkg.set(&chart_part, space.to_bytes()?)?;
    Ok(())
}

pub fn set_chart_series(
    pkg: &mut Package,
    sheet_part: &str,
    sheet_name: &str,
    sheet_index: &[(String, String, &'static str)],
    dirty_sheets: &mut Vec<(String, Document)>,
    op: &SetChartSeriesOp,
    op_index: usize,
) -> Result<bool, Error> {
    let (mut space, chart_part) = load_space(pkg, sheet_part, &op.target, &op.id, op_index)?;
    chart::reject_restricted(&space.root, &op.id, op_index, ChartRestrict::Data)?;
    if op.index >= chart::series_count(&space.root) {
        return Err(Error::with_path(
            ErrorKind::ChartSeriesNotFound { id: op.id.clone() },
            format!("ops[{op_index}].id"),
        ));
    }
    let mut wrote_cells = false;
    let snapshot = chart::with_ser_mut(&mut space.root, op.index, |ser| ser.clone()).unwrap();
    if let Some(name) = &op.name {
        wrote_cells |= write_formula_values(
            pkg,
            sheet_name,
            sheet_index,
            dirty_sheets,
            snapshot
                .child_by_local("tx")
                .and_then(chart::formula_of)
                .as_deref(),
            &[json!(name)],
            true,
            op_index,
        )?;
        chart::with_ser_mut(&mut space.root, op.index, |ser| {
            chart::set_series_name(ser, name);
        });
    }
    if let Some(cats) = &op.categories {
        let local = if snapshot.child_by_local("xVal").is_some() {
            "xVal"
        } else {
            "cat"
        };
        let f = snapshot.child_by_local(local).and_then(chart::formula_of);
        let new_f = write_or_resize(
            pkg,
            sheet_name,
            sheet_index,
            dirty_sheets,
            f.as_deref(),
            cats,
            op_index,
            &mut wrote_cells,
        )?;
        chart::with_ser_mut(&mut space.root, op.index, |ser| {
            chart::set_pts(ser, local, cats, new_f.as_deref());
        });
    }
    if let Some(vals) = &op.values {
        let local = if snapshot.child_by_local("yVal").is_some() {
            "yVal"
        } else {
            "val"
        };
        let f = snapshot.child_by_local(local).and_then(chart::formula_of);
        let new_f = write_or_resize(
            pkg,
            sheet_name,
            sheet_index,
            dirty_sheets,
            f.as_deref(),
            vals,
            op_index,
            &mut wrote_cells,
        )?;
        chart::with_ser_mut(&mut space.root, op.index, |ser| {
            chart::set_pts(ser, local, vals, new_f.as_deref());
        });
    }
    if let Some(sizes) = &op.sizes {
        let f = snapshot
            .child_by_local("bubbleSize")
            .and_then(chart::formula_of);
        let new_f = write_or_resize(
            pkg,
            sheet_name,
            sheet_index,
            dirty_sheets,
            f.as_deref(),
            sizes,
            op_index,
            &mut wrote_cells,
        )?;
        chart::with_ser_mut(&mut space.root, op.index, |ser| {
            chart::set_pts(ser, "bubbleSize", sizes, new_f.as_deref());
        });
    }
    pkg.set(&chart_part, space.to_bytes()?)?;
    Ok(wrote_cells)
}

pub fn add_chart_series(
    pkg: &mut Package,
    sheet_part: &str,
    sheet_name: &str,
    sheet_index: &[(String, String, &'static str)],
    dirty_sheets: &mut Vec<(String, Document)>,
    op: &AddChartSeriesOp,
    op_index: usize,
) -> Result<bool, Error> {
    let (mut space, chart_part) = load_space(pkg, sheet_part, &op.target, &op.id, op_index)?;
    chart::reject_restricted(&space.root, &op.id, op_index, ChartRestrict::Data)?;
    chart::remain::reject_structure_edit(&space.root, &op.id, op_index)?;
    let col = chart::next_val_col_from_formulas(&space.root).ok_or_else(|| {
        Error::with_path(ErrorKind::ChartFormulaNotA1, format!("ops[{op_index}]"))
    })?;
    let first = chart::collect_sers(&space.root)
        .first()
        .cloned()
        .ok_or_else(|| {
            Error::with_path(
                ErrorKind::NeedChartCategories,
                format!("ops[{op_index}].categories"),
            )
        })?;
    let cat_el = first
        .child_by_local("cat")
        .or_else(|| first.child_by_local("xVal"));
    let cat_f = cat_el.and_then(chart::formula_of);
    let cat_ref = chart::parse_a1_formula(cat_f.as_deref().unwrap_or("")).ok_or_else(|| {
        Error::with_path(ErrorKind::ChartFormulaNotA1, format!("ops[{op_index}]"))
    })?;
    let cats = op
        .categories
        .clone()
        .unwrap_or_else(|| cat_el.and_then(chart::cache_values).unwrap_or_default());
    if cats.is_empty() {
        return Err(Error::with_path(
            ErrorKind::NeedChartCategories,
            format!("ops[{op_index}].categories"),
        ));
    }
    let start_row = cat_ref.start.row;
    let sheet_for_write = cat_ref.sheet.as_deref().unwrap_or(sheet_name);
    let target_part = sheet_part_by_name(sheet_index, sheet_for_write, op_index)?;
    apply::load_dirty(pkg, dirty_sheets, &target_part, true)?;
    {
        let doc = apply::dirty_of(dirty_sheets, &target_part);
        let sheet_data = doc.root.child_by_local("sheetData");
        for i in 0..op.values.len() {
            let cell = CellRef {
                col,
                row: start_row + i as u32,
            };
            if let Some(sd) = sheet_data {
                if apply::find_cell(sd, cell).is_some() {
                    return Err(Error::with_path(
                        ErrorKind::ChartFormulaNotA1,
                        format!("ops[{op_index}]"),
                    ));
                }
            }
        }
        let name = op
            .name
            .clone()
            .unwrap_or_else(|| format!("系列{}", chart::series_count(&space.root) + 1));
        write_json_cell(
            &mut doc.root,
            CellRef {
                col,
                row: start_row.saturating_sub(1).max(1),
            },
            &json!(name),
            op_index,
        )?;
        for (i, v) in op.values.iter().enumerate() {
            write_json_cell(
                &mut doc.root,
                CellRef {
                    col,
                    row: start_row + i as u32,
                },
                v,
                op_index,
            )?;
        }
        if let Some(sizes) = &op.sizes {
            for (i, v) in sizes.iter().enumerate() {
                write_json_cell(
                    &mut doc.root,
                    CellRef {
                        col: col + 1,
                        row: start_row + i as u32,
                    },
                    v,
                    op_index,
                )?;
            }
        }
    }
    let name = op
        .name
        .clone()
        .unwrap_or_else(|| format!("系列{}", chart::series_count(&space.root) + 1));
    let name_row = start_row.saturating_sub(1).max(1);
    let name_ref = chart::format_a1_cell(sheet_for_write, CellRef { col, row: name_row });
    let cat_formula = cat_f
        .unwrap_or_else(|| chart::format_a1_formula(sheet_for_write, cat_ref.start, cat_ref.end));
    let val_ref = chart::format_a1_formula(
        sheet_for_write,
        CellRef {
            col,
            row: start_row,
        },
        CellRef {
            col,
            row: start_row + op.values.len() as u32 - 1,
        },
    );
    let ty = chart::inspect_type(&space.root, false);
    let xy = chart::uses_xy_axes(&ty)
        || op.series_type.is_some_and(ChartType::is_xy)
        || chart::last_series_type(&space.root).is_some_and(ChartType::is_xy);
    let xml = chart::ser_xml(
        chart::unused_idx(&space.root),
        &name,
        &name_ref,
        &cat_formula,
        &cats,
        &val_ref,
        &op.values,
        xy,
        None,
    );
    let ser_el = chart::parse_ser_xml(&xml)?;
    chart::insert_ser(&mut space.root, ser_el, op.series_type);
    pkg.set(&chart_part, space.to_bytes()?)?;
    Ok(true)
}

pub fn delete_chart_series(
    pkg: &mut Package,
    sheet_part: &str,
    op: &DeleteChartSeriesOp,
    op_index: usize,
) -> Result<(), Error> {
    let (mut space, chart_part) = load_space(pkg, sheet_part, &op.target, &op.id, op_index)?;
    chart::reject_restricted(&space.root, &op.id, op_index, ChartRestrict::Data)?;
    chart::remain::reject_structure_edit(&space.root, &op.id, op_index)?;
    let n = chart::series_count(&space.root);
    if op.index >= n {
        return Err(Error::with_path(
            ErrorKind::ChartSeriesNotFound { id: op.id.clone() },
            format!("ops[{op_index}].id"),
        ));
    }
    if n <= 1 {
        return Err(Error::with_path(
            ErrorKind::LastChartSeries,
            format!("ops[{op_index}].id"),
        ));
    }
    chart::remove_series_at(&mut space.root, op.index);
    pkg.set(&chart_part, space.to_bytes()?)?;
    Ok(())
}

pub fn set_chart_type(
    pkg: &mut Package,
    sheet_part: &str,
    op: &SetChartTypeOp,
    op_index: usize,
) -> Result<(), Error> {
    let (mut space, chart_part) = load_space(pkg, sheet_part, &op.target, &op.id, op_index)?;
    chart::reject_restricted(&space.root, &op.id, op_index, ChartRestrict::Data)?;
    chart::convert_type(
        &mut space.root,
        op.chart_type,
        op.of_pie_type.as_deref(),
        &op.extra,
        op_index,
    )?;
    pkg.set(&chart_part, space.to_bytes()?)?;
    Ok(())
}

pub fn set_chart_axis_title(
    pkg: &mut Package,
    sheet_part: &str,
    op: &SetChartAxisTitleOp,
    op_index: usize,
) -> Result<(), Error> {
    let (mut space, chart_part) = load_space(pkg, sheet_part, &op.target, &op.id, op_index)?;
    chart::reject_restricted(&space.root, &op.id, op_index, ChartRestrict::Decorate)?;
    chart::set_axis_title(&mut space.root, &op.axis, &op.title, op_index)?;
    pkg.set(&chart_part, space.to_bytes()?)?;
    Ok(())
}

pub fn set_chart_legend(
    pkg: &mut Package,
    sheet_part: &str,
    op: &SetChartLegendOp,
    op_index: usize,
) -> Result<(), Error> {
    let (mut space, chart_part) = load_space(pkg, sheet_part, &op.target, &op.id, op_index)?;
    chart::reject_restricted(&space.root, &op.id, op_index, ChartRestrict::Decorate)?;
    chart::set_legend(&mut space.root, op.show, op.pos.as_deref());
    pkg.set(&chart_part, space.to_bytes()?)?;
    Ok(())
}

pub fn set_chart_data_labels(
    pkg: &mut Package,
    sheet_part: &str,
    op: &SetChartDataLabelsOp,
    op_index: usize,
) -> Result<(), Error> {
    let (mut space, chart_part) = load_space(pkg, sheet_part, &op.target, &op.id, op_index)?;
    chart::reject_restricted(&space.root, &op.id, op_index, ChartRestrict::Decorate)?;
    chart::set_data_labels(&mut space.root, op.show);
    pkg.set(&chart_part, space.to_bytes()?)?;
    Ok(())
}

pub fn insert_trendline(
    pkg: &mut Package,
    sheet_part: &str,
    op: &InsertTrendlineOp,
    op_index: usize,
) -> Result<(), Error> {
    let (mut space, chart_part) = load_space(pkg, sheet_part, &op.target, &op.id, op_index)?;
    chart::reject_restricted(&space.root, &op.id, op_index, ChartRestrict::Decorate)?;
    chart::remain::insert_trendline(&mut space.root, &op.id, op.index, &op.spec, op_index)?;
    pkg.set(&chart_part, space.to_bytes()?)?;
    Ok(())
}

pub fn set_trendline(
    pkg: &mut Package,
    sheet_part: &str,
    op: &SetTrendlineOp,
    op_index: usize,
) -> Result<(), Error> {
    let (mut space, chart_part) = load_space(pkg, sheet_part, &op.target, &op.id, op_index)?;
    chart::reject_restricted(&space.root, &op.id, op_index, ChartRestrict::Decorate)?;
    chart::remain::set_trendline(
        &mut space.root,
        &op.id,
        op.index,
        op.tl_index,
        &op.spec,
        op_index,
    )?;
    pkg.set(&chart_part, space.to_bytes()?)?;
    Ok(())
}

pub fn delete_trendline(
    pkg: &mut Package,
    sheet_part: &str,
    op: &DeleteTrendlineOp,
    op_index: usize,
) -> Result<(), Error> {
    let (mut space, chart_part) = load_space(pkg, sheet_part, &op.target, &op.id, op_index)?;
    chart::reject_restricted(&space.root, &op.id, op_index, ChartRestrict::Decorate)?;
    chart::remain::delete_trendline(&mut space.root, &op.id, op.index, op.tl_index, op_index)?;
    pkg.set(&chart_part, space.to_bytes()?)?;
    Ok(())
}

pub fn insert_error_bars(
    pkg: &mut Package,
    sheet_part: &str,
    op: &InsertErrorBarsOp,
    op_index: usize,
) -> Result<(), Error> {
    let (mut space, chart_part) = load_space(pkg, sheet_part, &op.target, &op.id, op_index)?;
    chart::reject_restricted(&space.root, &op.id, op_index, ChartRestrict::Decorate)?;
    chart::remain::insert_error_bars(&mut space.root, &op.id, op.index, &op.spec, op_index)?;
    pkg.set(&chart_part, space.to_bytes()?)?;
    Ok(())
}

pub fn set_error_bars(
    pkg: &mut Package,
    sheet_part: &str,
    op: &SetErrorBarsOp,
    op_index: usize,
) -> Result<(), Error> {
    let (mut space, chart_part) = load_space(pkg, sheet_part, &op.target, &op.id, op_index)?;
    chart::reject_restricted(&space.root, &op.id, op_index, ChartRestrict::Decorate)?;
    chart::remain::set_error_bars(
        &mut space.root,
        &op.id,
        op.index,
        op.eb_index,
        &op.spec,
        op_index,
    )?;
    pkg.set(&chart_part, space.to_bytes()?)?;
    Ok(())
}

pub fn delete_error_bars(
    pkg: &mut Package,
    sheet_part: &str,
    op: &DeleteErrorBarsOp,
    op_index: usize,
) -> Result<(), Error> {
    let (mut space, chart_part) = load_space(pkg, sheet_part, &op.target, &op.id, op_index)?;
    chart::reject_restricted(&space.root, &op.id, op_index, ChartRestrict::Decorate)?;
    chart::remain::delete_error_bars(&mut space.root, &op.id, op.index, op.eb_index, op_index)?;
    pkg.set(&chart_part, space.to_bytes()?)?;
    Ok(())
}

pub fn set_series_axis(
    pkg: &mut Package,
    sheet_part: &str,
    op: &SetChartSeriesAxisOp,
    op_index: usize,
) -> Result<(), Error> {
    let (mut space, chart_part) = load_space(pkg, sheet_part, &op.target, &op.id, op_index)?;
    chart::reject_restricted(&space.root, &op.id, op_index, ChartRestrict::Decorate)?;
    chart::remain::set_series_axis(&mut space.root, &op.id, op.index, op.secondary, op_index)?;
    pkg.set(&chart_part, space.to_bytes()?)?;
    Ok(())
}

pub fn set_chart_size(
    pkg: &mut Package,
    sheet_part: &str,
    op: &SetChartSizeOp,
    op_index: usize,
) -> Result<(), Error> {
    let path = format!("ops[{op_index}].id");
    let drawing_part = find_drawing_for(pkg, sheet_part, &op.target, &op.id, &path)?;
    let mut doc = xml::parse(pkg.get(&drawing_part)?)?;
    let kind = find_anchor_kind(&doc.root, &op.target.key).ok_or_else(|| {
        Error::with_path(ErrorKind::ChartNotFound { id: op.id.clone() }, path.clone())
    })?;
    let action = picture::plan_size(
        kind,
        &picture::SizePatch {
            cx: op.cx,
            cy: op.cy,
            to: op.to,
            to_off_x: op.to_off_x,
            to_off_y: op.to_off_y,
        },
        ErrorKind::TwoCellChartSize,
        "setChartSize",
        op_index,
    )?;
    let ok = match action {
        picture::SizeAction::Extent { cx, cy } => {
            update_extent(&mut doc.root, &op.target.key, cx, cy)
        }
        picture::SizeAction::TwoCell {
            to,
            to_off_x,
            to_off_y,
        } => update_two_cell_to(&mut doc.root, &op.target.key, to, to_off_x, to_off_y)?,
    };
    if !ok {
        return Err(Error::with_path(
            ErrorKind::ChartNotFound { id: op.id.clone() },
            path,
        ));
    }
    pkg.set(&drawing_part, doc.to_bytes()?)?;
    Ok(())
}

pub fn set_chart_anchor(
    pkg: &mut Package,
    sheet_part: &str,
    op: &SetChartAnchorOp,
    op_index: usize,
) -> Result<(), Error> {
    let path = format!("ops[{op_index}].id");
    let drawing_part = find_drawing_for(pkg, sheet_part, &op.target, &op.id, &path)?;
    let mut doc = xml::parse(pkg.get(&drawing_part)?)?;
    let snapshot = snapshot_anchor(&doc.root, &op.target.key).ok_or_else(|| {
        Error::with_path(ErrorKind::ChartNotFound { id: op.id.clone() }, path.clone())
    })?;
    let planned = picture::plan_anchor(
        &snapshot.geom(),
        &picture::AnchorPatch {
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
        },
        "setChartAnchor",
        op_index,
        chart::DEFAULT_CX,
        chart::DEFAULT_CY,
    )?;
    if planned.pos_only {
        if !update_absolute_pos(&mut doc.root, &op.target.key, planned.pos_x, planned.pos_y) {
            return Err(Error::with_path(
                ErrorKind::ChartNotFound { id: op.id.clone() },
                path,
            ));
        }
        pkg.set(&drawing_part, doc.to_bytes()?)?;
        return Ok(());
    }
    picture::ensure_xmlns(&mut doc.root, "r", REL_NS);
    picture::ensure_xmlns(&mut doc.root, "a", picture::A_NS);
    replace_chart_anchor(
        &mut doc.root,
        &op.target.key,
        picture::build_anchor(
            planned.kind,
            planned.from,
            planned.to,
            planned.from_off_x,
            planned.from_off_y,
            planned.to_off_x,
            planned.to_off_y,
            planned.pos_x,
            planned.pos_y,
            planned.cx,
            planned.cy,
            planned.edit_as,
            snapshot.cnv,
            &snapshot.name,
            &snapshot.rid,
            Some(snapshot.frame),
        ),
    );
    pkg.set(&drawing_part, doc.to_bytes()?)?;
    Ok(())
}

fn write_or_resize(
    pkg: &mut Package,
    default_sheet: &str,
    sheet_index: &[(String, String, &'static str)],
    dirty_sheets: &mut Vec<(String, Document)>,
    formula: Option<&str>,
    values: &[Value],
    op_index: usize,
    wrote: &mut bool,
) -> Result<Option<String>, Error> {
    let Some(raw) = formula else {
        return Ok(None);
    };
    let (names, order) = formula_ctx(pkg);
    let target =
        chart::resolve_chart_formula(raw, &names, &order, default_sheet).ok_or_else(|| {
            Error::with_path(ErrorKind::ChartFormulaNotA1, format!("ops[{op_index}]"))
        })?;
    match target {
        chart::FormulaTarget::DirectA1(refer) => {
            *wrote = true;
            write_resolved_cells(
                pkg,
                default_sheet,
                sheet_index,
                dirty_sheets,
                &chart::FormulaTarget::DirectA1(chart::resize_a1(&refer, values.len())),
                values,
                op_index,
            )?;
            let resized = chart::resize_a1(&refer, values.len());
            let sheet = resized.sheet.as_deref().unwrap_or(default_sheet);
            Ok(Some(chart::format_a1_formula(
                sheet,
                resized.start,
                resized.end,
            )))
        }
        other => {
            let cells = other.cells();
            if cells.len() != values.len() {
                return Err(Error::with_path(
                    ErrorKind::ChartFormulaNotA1,
                    format!("ops[{op_index}]"),
                ));
            }
            *wrote = true;
            write_resolved_cells(
                pkg,
                default_sheet,
                sheet_index,
                dirty_sheets,
                &other,
                values,
                op_index,
            )?;
            Ok(None)
        }
    }
}

fn write_formula_values(
    pkg: &mut Package,
    default_sheet: &str,
    sheet_index: &[(String, String, &'static str)],
    dirty_sheets: &mut Vec<(String, Document)>,
    formula: Option<&str>,
    values: &[Value],
    single: bool,
    op_index: usize,
) -> Result<bool, Error> {
    let Some(raw) = formula else {
        return Ok(false);
    };
    let (names, order) = formula_ctx(pkg);
    let target =
        chart::resolve_chart_formula(raw, &names, &order, default_sheet).ok_or_else(|| {
            Error::with_path(ErrorKind::ChartFormulaNotA1, format!("ops[{op_index}]"))
        })?;
    match target {
        chart::FormulaTarget::DirectA1(refer) => {
            let part_sheet = refer.sheet.as_deref().unwrap_or(default_sheet);
            let part = sheet_part_by_name(sheet_index, part_sheet, op_index)?;
            apply::load_dirty(pkg, dirty_sheets, &part, true)?;
            let doc = apply::dirty_of(dirty_sheets, &part);
            if single {
                write_json_cell(&mut doc.root, refer.start, &values[0], op_index)?;
            } else {
                let resized = chart::resize_a1(&refer, values.len());
                for (cell, value) in chart::walk_a1(&resized).into_iter().zip(values.iter()) {
                    write_json_cell(&mut doc.root, cell, value, op_index)?;
                }
            }
            Ok(true)
        }
        other => {
            let cells = other.cells();
            let expect = if single { 1 } else { values.len() };
            if cells.len() != expect {
                return Err(Error::with_path(
                    ErrorKind::ChartFormulaNotA1,
                    format!("ops[{op_index}]"),
                ));
            }
            write_resolved_cells(
                pkg,
                default_sheet,
                sheet_index,
                dirty_sheets,
                &other,
                if single { &values[..1] } else { values },
                op_index,
            )?;
            Ok(true)
        }
    }
}

fn write_resolved_cells(
    pkg: &mut Package,
    default_sheet: &str,
    sheet_index: &[(String, String, &'static str)],
    dirty_sheets: &mut Vec<(String, Document)>,
    target: &chart::FormulaTarget,
    values: &[Value],
    op_index: usize,
) -> Result<(), Error> {
    for ((sheet_opt, cell), value) in target.cells().into_iter().zip(values.iter()) {
        let sheet = sheet_opt.as_deref().unwrap_or(default_sheet);
        let part = sheet_part_by_name(sheet_index, sheet, op_index)?;
        apply::load_dirty(pkg, dirty_sheets, &part, true)?;
        let doc = apply::dirty_of(dirty_sheets, &part);
        write_json_cell(&mut doc.root, cell, value, op_index)?;
    }
    Ok(())
}

fn sheet_part_by_name(
    index: &[(String, String, &'static str)],
    name: &str,
    op_index: usize,
) -> Result<String, Error> {
    index
        .iter()
        .find(|(n, _, k)| n.eq_ignore_ascii_case(name) && *k == "worksheet")
        .map(|(_, p, _)| p.clone())
        .ok_or_else(|| {
            Error::with_path(
                ErrorKind::SheetNotFound {
                    name: name.to_string(),
                },
                format!("ops[{op_index}]"),
            )
        })
}

fn load_space(
    pkg: &Package,
    sheet_part: &str,
    target: &SheetChartId,
    id: &str,
    op_index: usize,
) -> Result<(Document, String), Error> {
    let path = format!("ops[{op_index}].id");
    let drawing_part = find_drawing_for(pkg, sheet_part, target, id, &path)?;
    let doc = xml::parse(pkg.get(&drawing_part)?)?;
    let rid = find_chart_rid(&doc.root, &target.key).ok_or_else(|| {
        Error::with_path(
            ErrorKind::ChartNotFound { id: id.to_string() },
            path.clone(),
        )
    })?;
    let chart_part = crate::opc::resolve_id(pkg, &drawing_part, &rid)?;
    Ok((xml::parse(pkg.get(&chart_part)?)?, chart_part))
}

fn count_charts(pkg: &Package, sheet_part: &str) -> Result<usize, Error> {
    let mut n = 0usize;
    for part in picture::drawing_parts(pkg, sheet_part)? {
        let doc = xml::parse(pkg.get(&part)?)?;
        picture::walk_anchors(&doc.root, &mut |anchor, _| {
            if direct_chart_frame(anchor).is_some() {
                n += 1;
            }
            Ok(())
        })?;
    }
    Ok(n)
}

fn find_drawing_for(
    pkg: &Package,
    sheet_part: &str,
    target: &SheetChartId,
    id: &str,
    field_path: &str,
) -> Result<String, Error> {
    for part in picture::drawing_parts(pkg, sheet_part)? {
        let doc = xml::parse(pkg.get(&part)?)?;
        if find_chart_rid(&doc.root, &target.key).is_some() {
            return Ok(part);
        }
    }
    Err(Error::with_path(
        ErrorKind::ChartNotFound { id: id.to_string() },
        field_path,
    ))
}

fn find_chart_rid(root: &Element, key: &ChartKey) -> Option<String> {
    let mut index = 0usize;
    let mut rid = None;
    let _ = picture::walk_anchors(root, &mut |anchor, _| {
        if rid.is_some() {
            return Ok(());
        }
        let Some(frame) = direct_chart_frame(anchor) else {
            return Ok(());
        };
        index += 1;
        if key_matches(key, &cnv_of_frame(frame).0, index) {
            rid = chart_rid(frame);
        }
        Ok(())
    });
    rid
}

fn find_anchor_kind(root: &Element, key: &ChartKey) -> Option<AnchorKind> {
    let mut index = 0usize;
    let mut kind = None;
    let _ = picture::walk_anchors(root, &mut |anchor, k| {
        if kind.is_some() {
            return Ok(());
        }
        let Some(frame) = direct_chart_frame(anchor) else {
            return Ok(());
        };
        index += 1;
        if key_matches(key, &cnv_of_frame(frame).0, index) {
            kind = Some(k);
        }
        Ok(())
    });
    kind
}

fn key_matches(key: &ChartKey, raw: &Option<String>, index: usize) -> bool {
    match key {
        ChartKey::Index(n) => index == *n,
        ChartKey::Cnv(want) => {
            let Some(got) = raw else {
                return false;
            };
            if let (Ok(a), Ok(b)) = (want.parse::<u32>(), got.parse::<u32>()) {
                a == b
            } else {
                want == got
            }
        }
    }
}

fn direct_chart_frame(anchor: &Element) -> Option<&Element> {
    anchor
        .element_children()
        .find(|c| c.local_name() == "graphicFrame" && graphic_uri(c).is_some())
}

fn graphic_uri(el: &Element) -> Option<String> {
    let mut found = None;
    picture::walk_all(el, &mut |e| {
        if e.local_name() == "graphicData" && found.is_none() {
            found = e.attr_local("uri").map(str::to_string);
        }
    });
    found.filter(|u| {
        let n = u.trim_end_matches('/');
        n == CHART_NS || n == CHARTEX_NS
    })
}

fn is_chartex(el: &Element) -> bool {
    matches!(graphic_uri(el), Some(u) if u.trim_end_matches('/') == CHARTEX_NS)
}

fn chart_rid(frame: &Element) -> Option<String> {
    let mut found = None;
    picture::walk_all(frame, &mut |e| {
        if found.is_none() && e.local_name() == "chart" && e.attr_local("id").is_some() {
            found = e.attr_local("id").map(str::to_string);
        }
    });
    found
}

fn cnv_of_frame(frame: &Element) -> (Option<String>, String) {
    let mut raw = None;
    let mut name = String::new();
    if let Some(nv) = frame.child_by_local("nvGraphicFramePr") {
        if let Some(pr) = nv.child_by_local("cNvPr") {
            raw = pr.attr_local("id").map(str::to_string);
            name = pr.attr_local("name").unwrap_or("").to_string();
        }
    }
    (raw, name)
}

pub(crate) fn build_graphic_frame(
    cnv: u32,
    name: &str,
    rid: &str,
    cx: i64,
    cy: i64,
    chartex: bool,
) -> Element {
    let mut nv = Element::new("xdr:nvGraphicFramePr");
    nv.push_element(
        Element::new("xdr:cNvPr")
            .with_attr("id", cnv.to_string())
            .with_attr("name", name),
    );
    let mut locks = Element::new("xdr:cNvGraphicFramePr");
    locks.push_element(Element::new("a:graphicFrameLocks").with_attr("noGrp", "1"));
    nv.push_element(locks);
    let mut xfrm = Element::new("xdr:xfrm");
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
    let mut data =
        Element::new("a:graphicData").with_attr("uri", if chartex { CHARTEX_NS } else { CHART_NS });
    data.push_element(chart::graphic_chart_el(rid, chartex));
    let mut graphic = Element::new("a:graphic");
    graphic.push_element(data);
    let mut frame = Element::new("xdr:graphicFrame");
    frame.push_element(nv);
    frame.push_element(xfrm);
    frame.push_element(graphic);
    frame
}

pub(crate) fn next_chart_part_for(pkg: &Package, ty: ChartType) -> String {
    next_chart_part_named(pkg, chart::part_stem(ty))
}

fn next_chart_part_named(pkg: &Package, stem: &str) -> String {
    let mut n = 1u32;
    loop {
        let name = format!("/xl/charts/{stem}{n}.xml");
        if !pkg.contains(&name) {
            return name;
        }
        n += 1;
    }
}

pub(crate) fn add_chart_rel(
    pkg: &mut Package,
    drawing_part: &str,
    chart_part: &str,
    rel_type: &str,
) -> Result<String, Error> {
    let mut rels = Relationships::from_package(pkg, drawing_part)?;
    let rid = rels.next_rid();
    rels.push(Relationship {
        id: rid.clone(),
        rel_type: rel_type.into(),
        target: picture::rel_target_from(drawing_part, chart_part),
        target_mode: None,
        extra: Vec::new(),
    });
    rels.write_to(pkg, drawing_part)?;
    Ok(rid)
}

pub(crate) fn chart_still_used(pkg: &Package, chart_part: &str) -> Result<bool, Error> {
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
            if resolve_rel_target(&source, &r.target)
                .ok()
                .is_some_and(|part| same_part_name(&part, chart_part))
            {
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

pub(crate) fn remove_chart_tree(pkg: &mut Package, chart_part: &str) -> Result<(), Error> {
    if let Ok(rels) = Relationships::from_package(pkg, chart_part) {
        for r in &rels.rels {
            if r.is_external() {
                continue;
            }
            if let Ok(p) = resolve_rel_target(chart_part, &r.target) {
                if pkg.contains(&p) {
                    pkg.remove(&p);
                }
            }
        }
        pkg.remove(&rels_part_name(chart_part));
    }
    if pkg.contains(chart_part) {
        pkg.remove(chart_part);
    }
    let mut cts = ContentTypes::from_package(pkg)?;
    cts.remove_override(chart_part);
    cts.write_to(pkg)?;
    Ok(())
}

fn remove_chart_anchor(root: &mut Element, key: &ChartKey) -> bool {
    remove_chart_anchor_in(root, key, &mut 0)
}

fn remove_chart_anchor_in(el: &mut Element, key: &ChartKey, index: &mut usize) -> bool {
    if el.local_name() == "grpSp" {
        return false;
    }
    let mut drop = None;
    for (i, n) in el.children.iter_mut().enumerate() {
        let Node::Element(child) = n else { continue };
        if child.local_name() == "AlternateContent" {
            let branch = crate::xml::alternate_child(child)
                .unwrap_or(None)
                .and_then(|index| match &child.children[index] {
                    Node::Element(branch) => Some(branch),
                    Node::Text(_) | Node::Raw(_) => None,
                });
            if let Some(branch) = branch {
                if charts_match_in(branch, key, index) {
                    drop = Some(i);
                    break;
                }
            }
            continue;
        }
        if AnchorKind::from_local_name(child.local_name()).is_some() {
            if let Some(frame) = direct_chart_frame(child) {
                *index += 1;
                if key_matches(key, &cnv_of_frame(frame).0, *index) {
                    drop = Some(i);
                    break;
                }
            }
        } else if remove_chart_anchor_in(child, key, index) {
            return true;
        }
    }
    if let Some(i) = drop {
        el.children.remove(i);
        return true;
    }
    false
}

fn charts_match_in(el: &Element, key: &ChartKey, index: &mut usize) -> bool {
    let mut matched = false;
    let _ = picture::walk_anchors(el, &mut |anchor, _| {
        let Some(frame) = direct_chart_frame(anchor) else {
            return Ok(());
        };
        *index += 1;
        if key_matches(key, &cnv_of_frame(frame).0, *index) {
            matched = true;
        }
        Ok(())
    });
    matched
}

fn replace_chart_anchor(root: &mut Element, key: &ChartKey, new_anchor: Element) -> bool {
    replace_chart_anchor_in(root, key, &mut 0, new_anchor)
}

fn replace_chart_anchor_in(
    el: &mut Element,
    key: &ChartKey,
    index: &mut usize,
    new_anchor: Element,
) -> bool {
    for i in 0..el.children.len() {
        let Node::Element(child) = &mut el.children[i] else {
            continue;
        };
        if AnchorKind::from_local_name(child.local_name()).is_some() {
            if let Some(frame) = direct_chart_frame(child) {
                *index += 1;
                if key_matches(key, &cnv_of_frame(frame).0, *index) {
                    el.children[i] = Node::Element(new_anchor);
                    return true;
                }
            }
        } else if replace_chart_anchor_in(child, key, index, new_anchor.clone()) {
            return true;
        }
    }
    false
}

struct AnchorSnap {
    kind: AnchorKind,
    from: Option<CellRef>,
    to: Option<CellRef>,
    from_off_x: i64,
    from_off_y: i64,
    to_off_x: i64,
    to_off_y: i64,
    pos_x: Option<i64>,
    pos_y: Option<i64>,
    cx: Option<i64>,
    cy: Option<i64>,
    edit_as: EditAs,
    cnv: u32,
    name: String,
    rid: String,
    frame: Element,
}

impl AnchorSnap {
    fn geom(&self) -> picture::AnchorGeom {
        picture::AnchorGeom {
            kind: self.kind,
            from: self.from,
            to: self.to,
            from_off_x: self.from_off_x,
            from_off_y: self.from_off_y,
            to_off_x: self.to_off_x,
            to_off_y: self.to_off_y,
            pos_x: self.pos_x,
            pos_y: self.pos_y,
            cx: self.cx,
            cy: self.cy,
            edit_as: self.edit_as,
        }
    }
}

fn snapshot_anchor(root: &Element, key: &ChartKey) -> Option<AnchorSnap> {
    let mut index = 0usize;
    let mut found = None;
    let _ = picture::walk_anchors(root, &mut |anchor, kind| {
        if found.is_some() {
            return Ok(());
        }
        let Some(frame) = direct_chart_frame(anchor) else {
            return Ok(());
        };
        index += 1;
        if !key_matches(key, &cnv_of_frame(frame).0, index) {
            return Ok(());
        }
        let (from, from_off_x, from_off_y) = picture::read_marker(anchor.child_by_local("from"))?;
        let (to, to_off_x, to_off_y) = picture::read_marker(anchor.child_by_local("to"))?;
        let (cx, cy) = picture::read_ext(anchor);
        let (pos_x, pos_y) = picture::read_pos(anchor);
        let (raw, name) = cnv_of_frame(frame);
        found = Some(AnchorSnap {
            kind,
            from,
            to,
            from_off_x,
            from_off_y,
            to_off_x,
            to_off_y,
            pos_x,
            pos_y,
            cx,
            cy,
            edit_as: if kind == AnchorKind::TwoCell {
                picture::parse_edit_as(anchor.attr("editAs"))
            } else {
                EditAs::TwoCell
            },
            cnv: raw.and_then(|s| s.parse().ok()).unwrap_or(1),
            name,
            rid: chart_rid(frame).unwrap_or_default(),
            frame: frame.clone(),
        });
        Ok(())
    });
    found
}

fn update_extent(root: &mut Element, key: &ChartKey, cx: Option<i64>, cy: Option<i64>) -> bool {
    let mut index = 0usize;
    picture::walk_anchors_mut(root, &mut |anchor, _| {
        let Some(frame) = anchor
            .element_children()
            .find(|c| c.local_name() == "graphicFrame" && graphic_uri(c).is_some())
        else {
            return Ok(false);
        };
        // 上面是不可变借用，下面再按 key 改。先记下是否匹配。
        let raw = cnv_of_frame(frame).0;
        index += 1;
        if !key_matches(key, &raw, index) {
            return Ok(false);
        }
        if let Some(ext) = anchor.child_by_local_mut("ext") {
            if let Some(v) = cx {
                picture::set_attr_local(ext, "cx", &v.to_string());
            }
            if let Some(v) = cy {
                picture::set_attr_local(ext, "cy", &v.to_string());
            }
        }
        if let Some(xfrm) = picture::find_local_mut(anchor, "xfrm") {
            if let Some(ext) = xfrm.child_by_local_mut("ext") {
                if let Some(v) = cx {
                    picture::set_attr_local(ext, "cx", &v.to_string());
                }
                if let Some(v) = cy {
                    picture::set_attr_local(ext, "cy", &v.to_string());
                }
            }
        }
        Ok(true)
    })
    .unwrap_or(false)
}

fn update_absolute_pos(
    root: &mut Element,
    key: &ChartKey,
    pos_x: Option<i64>,
    pos_y: Option<i64>,
) -> bool {
    let mut index = 0usize;
    picture::walk_anchors_mut(root, &mut |anchor, _| {
        let Some(frame) = anchor
            .element_children()
            .find(|c| c.local_name() == "graphicFrame" && graphic_uri(c).is_some())
        else {
            return Ok(false);
        };
        let raw = cnv_of_frame(frame).0;
        index += 1;
        if !key_matches(key, &raw, index) {
            return Ok(false);
        }
        if let Some(pos) = anchor.child_by_local_mut("pos") {
            if let Some(v) = pos_x {
                picture::set_attr_local(pos, "x", &v.to_string());
            }
            if let Some(v) = pos_y {
                picture::set_attr_local(pos, "y", &v.to_string());
            }
        }
        Ok(true)
    })
    .unwrap_or(false)
}

fn update_two_cell_to(
    root: &mut Element,
    key: &ChartKey,
    to: CellRef,
    to_off_x: Option<i64>,
    to_off_y: Option<i64>,
) -> Result<bool, Error> {
    let mut index = 0usize;
    picture::walk_anchors_mut(root, &mut |anchor, _| {
        let Some(frame) = anchor
            .element_children()
            .find(|c| c.local_name() == "graphicFrame" && graphic_uri(c).is_some())
        else {
            return Ok(false);
        };
        let raw = cnv_of_frame(frame).0;
        index += 1;
        if !key_matches(key, &raw, index) {
            return Ok(false);
        }
        picture::apply_two_cell_to(anchor, to, to_off_x, to_off_y)?;
        Ok(true)
    })
}

trait FromLocalName {
    fn from_local_name(name: &str) -> Option<Self>
    where
        Self: Sized;
}

impl FromLocalName for AnchorKind {
    fn from_local_name(name: &str) -> Option<Self> {
        match name {
            "twoCellAnchor" => Some(AnchorKind::TwoCell),
            "oneCellAnchor" => Some(AnchorKind::OneCell),
            "absoluteAnchor" => Some(AnchorKind::Absolute),
            _ => None,
        }
    }
}
