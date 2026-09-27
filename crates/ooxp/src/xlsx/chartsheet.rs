//! 新建、删除图表工作表。数据写在点名的普通工作表上，不写 embedding。

use crate::chart;
use crate::error::{Error, ErrorKind, IdReason};
use crate::opc::{
    rels_part_name, resolve_rel_target, ContentTypes, Package, Relationship, Relationships,
};
use crate::xml::{Document, Element, DEFAULT_DECL};

use super::a1;
use super::apply::{self, write_json_cell};
use super::chart as xlsx_chart;
use super::picture::{self, AnchorKind, EditAs};
use super::request::{DeleteChartSheetOp, InsertChartSheetOp};
use super::sheets;
use super::{CT_CHARTSHEET, REL_CHARTSHEET, REL_NS, SML_NS};

/// 图表工作表上那一张框的 `cNvPr/@id`。Excel 另存从 2 起；id 1 在部分 drawing 场景会拒开。
const CHARTSHEET_CNV: u32 = 2;

pub fn insert(
    pkg: &mut Package,
    wb_doc: &mut Document,
    wb_part: &str,
    sheet_index: &mut Vec<(String, String, &'static str)>,
    dirty_sheets: &mut Vec<(String, Document)>,
    op: &InsertChartSheetOp,
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
    let (data_name, data_part) = resolve_data_sheet(sheet_index, op, op_index)?;
    let series = chart::normalize_insert_series(op.chart_type, &op.series);
    let cats = chart::shared_categories(&series);
    let cells = chart::data_table_cells(op.chart_type, &series, &cats, op.data_from);
    apply::load_dirty(pkg, dirty_sheets, &data_part, true)?;
    {
        let doc = apply::dirty_of(dirty_sheets, &data_part);
        for (cell, value) in &cells {
            write_json_cell(&mut doc.root, *cell, value, op_index)?;
        }
    }
    let xml = chart::build_chart_xml(
        op.chart_type,
        op.title.as_deref(),
        &series,
        &cats,
        &data_name,
        op.data_from,
        false,
    );
    let chart_part = xlsx_chart::next_chart_part(pkg);
    pkg.set(&chart_part, xml.into_bytes())?;
    let drawing_part = picture::next_drawing_part(pkg);
    let cs_part = next_chartsheet_part(pkg);
    let rid = xlsx_chart::add_chart_rel(pkg, &drawing_part, &chart_part)?;
    let (cx, cy) = (
        op.cx.unwrap_or(chart::DEFAULT_CX),
        op.cy.unwrap_or(chart::DEFAULT_CY),
    );
    let cnv = CHARTSHEET_CNV;
    let frame = xlsx_chart::build_graphic_frame(cnv, &format!("图表 {cnv}"), &rid, cx, cy);
    let dummy = a1::CellRef { col: 1, row: 1 };
    let mut draw_root = Element::new("xdr:wsDr")
        .with_attr("xmlns:xdr", picture::XDR_NS)
        .with_attr("xmlns:a", picture::A_NS)
        .with_attr("xmlns:r", REL_NS);
    draw_root.push_element(picture::build_anchor(
        AnchorKind::Absolute,
        dummy,
        None,
        0,
        0,
        0,
        0,
        Some(0),
        Some(0),
        cx,
        cy,
        EditAs::TwoCell,
        cnv,
        &format!("图表 {cnv}"),
        &rid,
        Some(frame),
    ));
    pkg.set(
        &drawing_part,
        Document {
            decl: Some(DEFAULT_DECL.to_string()),
            root: draw_root,
        }
        .to_bytes()?,
    )?;

    let mut cs_rels = Relationships { rels: Vec::new() };
    let drawing_rid = cs_rels.next_rid();
    cs_rels.push(Relationship {
        id: drawing_rid.clone(),
        rel_type: picture::REL_DRAWING.into(),
        target: picture::rel_target_from(&cs_part, &drawing_part),
        target_mode: None,
        extra: Vec::new(),
    });
    cs_rels.write_to(pkg, &cs_part)?;
    pkg.set(&cs_part, chartsheet_xml(&drawing_rid).to_bytes()?)?;

    let mut wb_rels = Relationships::from_package(pkg, wb_part)?;
    let sheet_rid = wb_rels.next_rid();
    wb_rels.push(Relationship {
        id: sheet_rid.clone(),
        rel_type: REL_CHARTSHEET.into(),
        target: picture::rel_target_from(wb_part, &cs_part),
        target_mode: None,
        extra: Vec::new(),
    });
    wb_rels.write_to(pkg, wb_part)?;
    picture::ensure_xmlns(&mut wb_doc.root, "r", REL_NS);
    let sheets = wb_doc.root.child_by_local_mut("sheets").ok_or_else(|| {
        Error::new(ErrorKind::XmlMissing {
            parent: "workbook.xml".into(),
            child: "sheets".into(),
        })
    })?;
    let sheet_id = sheets::next_sheet_id(sheets);
    sheets.push_element(
        Element::new("sheet")
            .with_attr("name", op.name.clone())
            .with_attr("sheetId", sheet_id.to_string())
            .with_attr("r:id", sheet_rid),
    );
    let mut cts = ContentTypes::from_package(pkg)?;
    cts.set_override(&chart_part, chart::CT_CHART)?;
    cts.set_override(&drawing_part, picture::CT_DRAWING)?;
    cts.set_override(&cs_part, CT_CHARTSHEET)?;
    cts.write_to(pkg)?;
    sheet_index.push((op.name.clone(), cs_part, "chartsheet"));
    Ok(())
}

pub fn delete(
    pkg: &mut Package,
    wb_doc: &mut Document,
    wb_part: &str,
    sheet_index: &mut Vec<(String, String, &'static str)>,
    dirty_sheets: &mut Vec<(String, Document)>,
    op: &DeleteChartSheetOp,
    op_index: usize,
) -> Result<(), Error> {
    let meta = sheet_index
        .iter()
        .find(|(n, _, _)| n.eq_ignore_ascii_case(&op.sheet))
        .cloned();
    let Some((_, cs_part, kind)) = meta else {
        return Err(Error::with_path(
            ErrorKind::SheetNotFound {
                name: op.sheet.clone(),
            },
            format!("ops[{op_index}].id"),
        ));
    };
    if kind != "chartsheet" {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::DeleteChartSheetNeedsChartsheet,
            },
            format!("ops[{op_index}].id"),
        ));
    }
    sheets::reject_if_last_sheet(sheet_index.len(), op_index)?;
    sheets::reject_if_last_visible(&wb_doc.root, &op.sheet, op_index)?;
    apply::load_dirty(pkg, dirty_sheets, &cs_part, false)?;
    let drawing_parts = {
        let doc = apply::dirty_of(dirty_sheets, &cs_part);
        picture::drawing_parts_from(pkg, &cs_part, &doc.root)?
    };
    let mut charts = Vec::new();
    for drawing_part in &drawing_parts {
        charts.extend(chart_parts_of_drawing(pkg, drawing_part)?);
    }
    let rid = sheets::remove_workbook_sheet(&mut wb_doc.root, &op.sheet).ok_or_else(|| {
        Error::new(ErrorKind::XmlMissing {
            parent: "sheets".into(),
            child: "sheet".into(),
        })
    })?;
    let mut wb_rels = Relationships::from_package(pkg, wb_part)?;
    wb_rels.remove_by_id(&rid);
    wb_rels.write_to(pkg, wb_part)?;
    dirty_sheets.retain(|(p, _)| p != &cs_part);
    pkg.remove(&cs_part);
    pkg.remove(&rels_part_name(&cs_part));
    let mut cts = ContentTypes::from_package(pkg)?;
    cts.remove_override(&cs_part);
    for drawing_part in &drawing_parts {
        pkg.remove(drawing_part);
        pkg.remove(&rels_part_name(drawing_part));
        cts.remove_override(drawing_part);
    }
    cts.write_to(pkg)?;
    for chart_part in charts {
        if !xlsx_chart::chart_still_used(pkg, &chart_part)? {
            xlsx_chart::remove_chart_tree(pkg, &chart_part)?;
        }
    }
    sheet_index.retain(|(n, _, _)| !n.eq_ignore_ascii_case(&op.sheet));
    sheets::clamp_active_tab(&mut wb_doc.root, sheet_index.len());
    Ok(())
}

fn resolve_data_sheet(
    sheet_index: &[(String, String, &'static str)],
    op: &InsertChartSheetOp,
    op_index: usize,
) -> Result<(String, String), Error> {
    if let Some(want) = &op.data_sheet {
        let found = sheet_index
            .iter()
            .find(|(n, _, _)| n.eq_ignore_ascii_case(want));
        match found {
            Some((name, part, kind)) if *kind == "worksheet" => Ok((name.clone(), part.clone())),
            Some(_) => Err(Error::with_path(
                ErrorKind::DataSheetMustBeWorksheet,
                format!("ops[{op_index}].dataSheet"),
            )),
            None => Err(Error::with_path(
                ErrorKind::SheetNotFound { name: want.clone() },
                format!("ops[{op_index}].dataSheet"),
            )),
        }
    } else {
        sheet_index
            .iter()
            .find(|(_, _, k)| *k == "worksheet")
            .map(|(name, part, _)| (name.clone(), part.clone()))
            .ok_or_else(|| {
                Error::with_path(
                    ErrorKind::DataSheetMustBeWorksheet,
                    format!("ops[{op_index}].dataSheet"),
                )
            })
    }
}

fn next_chartsheet_part(pkg: &Package) -> String {
    let mut n = 1u32;
    loop {
        let name = format!("/xl/chartsheets/sheet{n}.xml");
        if !pkg.contains(&name) {
            return name;
        }
        n += 1;
    }
}

fn chartsheet_xml(drawing_rid: &str) -> Document {
    let view = Element::new("sheetView")
        .with_attr("workbookViewId", "0")
        .with_attr("zoomToFit", "1");
    let mut views = Element::new("sheetViews");
    views.push_element(view);
    let margins = Element::new("pageMargins")
        .with_attr("left", "0.7")
        .with_attr("right", "0.7")
        .with_attr("top", "0.75")
        .with_attr("bottom", "0.75")
        .with_attr("header", "0.3")
        .with_attr("footer", "0.3");
    let drawing = Element::new("drawing").with_attr("r:id", drawing_rid);
    let mut root = Element::new("chartsheet")
        .with_attr("xmlns", SML_NS)
        .with_attr("xmlns:r", REL_NS);
    root.push_element(views);
    root.push_element(margins);
    root.push_element(drawing);
    Document {
        decl: Some(DEFAULT_DECL.to_string()),
        root,
    }
}

fn chart_parts_of_drawing(pkg: &Package, drawing_part: &str) -> Result<Vec<String>, Error> {
    let rels = Relationships::from_package(pkg, drawing_part)?;
    let mut out = Vec::new();
    for r in &rels.rels {
        if r.is_external() {
            continue;
        }
        if !(r.rel_type.ends_with("/chart") || r.rel_type.ends_with("/chartEx")) {
            continue;
        }
        if let Ok(p) = resolve_rel_target(drawing_part, &r.target) {
            if pkg.contains(&p) {
                out.push(p);
            }
        }
    }
    Ok(out)
}
