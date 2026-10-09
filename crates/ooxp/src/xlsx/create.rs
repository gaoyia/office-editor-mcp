//! 从领域 JSON 生成 Transitional 工作簿。

use std::collections::HashSet;

use serde_json::{Map, Value};

use super::chart;
use super::chartsheet;
use super::comments;
use super::hf;
use super::hyperlink;
use super::picture::{self, AnchorKind};
use super::request::{
    parse_cell_value, HfSlot, InsertChartOp, InsertChartSheetOp, InsertCommentOp,
    InsertConnectorOp, InsertHyperlinkOp, InsertPictureOp, InsertShapeOp, InsertSmartArtOp,
    InsertTextBoxOp, SetCellValue, SetHeaderFooterTextOp,
};
use super::shape;
use super::smartart;
use crate::chart::{ChartType, NewSeries};
use crate::error::{Error, ErrorKind, SheetNameReason, TypeName};
use crate::opc::{
    ContentTypes, Package, Relationship, Relationships, CONTENT_TYPES_PART, ROOT_RELS_PART,
};
use crate::xml;

use super::a1::{self, CellRef};
use super::{
    CT_APP, CT_CORE, CT_SHARED_STRINGS, CT_STYLES, CT_WORKBOOK, CT_WORKSHEET, REL_APP, REL_CORE,
    REL_NS, REL_OFFICE_DOCUMENT, REL_SHARED_STRINGS, REL_STYLES, REL_WORKSHEET, SML_NS,
};

struct NewCell {
    refer: CellRef,
    a1: String,
    value: Option<SetCellValue>,
    formula: Option<String>,
}

struct NewHfSlot {
    slot: HfSlot,
    text: Option<String>,
    pictures: Vec<InsertPictureOp>,
}

struct NewSheet {
    name: String,
    chartsheet: Option<InsertChartSheetOp>,
    cells: Vec<NewCell>,
    pictures: Vec<InsertPictureOp>,
    charts: Vec<InsertChartOp>,
    header_footers: Vec<NewHfSlot>,
    comments: Vec<InsertCommentOp>,
    hyperlinks: Vec<InsertHyperlinkOp>,
    text_boxes: Vec<InsertTextBoxOp>,
    shapes: Vec<InsertShapeOp>,
    connectors: Vec<InsertConnectorOp>,
    smartarts: Vec<InsertSmartArtOp>,
    model3ds: Vec<super::model3d::InsertModel3dOp>,
    maths: Vec<super::omml::InsertMathOp>,
}

pub fn create(workbook: &Value) -> Result<Vec<u8>, Error> {
    let obj = workbook.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "workbook".into(),
                expected: TypeName::Object,
            },
            "workbook",
        )
    })?;
    for k in obj.keys() {
        if k != "sheets" && k != "props" {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("workbook.{k}"),
            ));
        }
    }
    let sheets_v = obj.get("sheets").ok_or_else(|| {
        Error::with_path(
            ErrorKind::MissingField {
                field: "sheets".into(),
            },
            "workbook.sheets",
        )
    })?;
    let sheets_arr = sheets_v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "sheets".into(),
                expected: TypeName::Array,
            },
            "workbook.sheets",
        )
    })?;
    if sheets_arr.is_empty() {
        return Err(Error::with_path(
            ErrorKind::NeedAtLeastOneSheet,
            "workbook.sheets",
        ));
    }

    let mut sheets = Vec::new();
    let mut names_lower = Vec::new();
    for (i, s) in sheets_arr.iter().enumerate() {
        sheets.push(parse_sheet(s, i)?);
        let lower = sheets[i].name.to_ascii_lowercase();
        if names_lower.contains(&lower) {
            return Err(Error::with_path(
                ErrorKind::DuplicateSheetName {
                    name: sheets[i].name.clone(),
                },
                format!("workbook.sheets[{i}].name"),
            ));
        }
        names_lower.push(lower);
    }
    if sheets.iter().any(|s| s.chartsheet.is_some())
        && sheets.iter().all(|s| s.chartsheet.is_some())
    {
        return Err(Error::with_path(
            ErrorKind::NeedAtLeastOneSheet,
            "workbook.sheets",
        ));
    }
    for (i, sh) in sheets.iter().enumerate() {
        let Some(op) = &sh.chartsheet else {
            continue;
        };
        let Some(ds) = &op.data_sheet else {
            continue;
        };
        match sheets.iter().find(|s| s.name.eq_ignore_ascii_case(ds)) {
            Some(found) if found.chartsheet.is_none() => {}
            Some(_) => {
                return Err(Error::with_path(
                    ErrorKind::DataSheetMustBeWorksheet,
                    format!("workbook.sheets[{i}].dataSheet"),
                ));
            }
            None => {
                return Err(Error::with_path(
                    ErrorKind::SheetNotFound { name: ds.clone() },
                    format!("workbook.sheets[{i}].dataSheet"),
                ));
            }
        }
    }

    let mut sst: Vec<String> = Vec::new();
    let mut has_formula = false;
    for sh in &sheets {
        for c in &sh.cells {
            if c.formula.is_some() {
                has_formula = true;
            }
            if let Some(SetCellValue::Text(t)) = &c.value {
                if !sst.iter().any(|s| s == t) {
                    sst.push(t.clone());
                }
            }
        }
    }

    let mut pkg = Package::create();
    let mut types = ContentTypes {
        defaults: vec![
            (
                "rels".into(),
                "application/vnd.openxmlformats-package.relationships+xml".into(),
            ),
            ("xml".into(), "application/xml".into()),
        ],
        overrides: vec![
            ("/xl/workbook.xml".into(), CT_WORKBOOK.into()),
            ("/xl/styles.xml".into(), CT_STYLES.into()),
            ("/docProps/core.xml".into(), CT_CORE.into()),
            ("/docProps/app.xml".into(), CT_APP.into()),
        ],
    };

    let mut root_rels = Relationships { rels: Vec::new() };
    root_rels.push(Relationship {
        id: "rId1".into(),
        rel_type: REL_OFFICE_DOCUMENT.into(),
        target: "xl/workbook.xml".into(),
        target_mode: None,
        extra: Vec::new(),
    });
    root_rels.push(Relationship {
        id: "rId2".into(),
        rel_type: REL_CORE.into(),
        target: "docProps/core.xml".into(),
        target_mode: None,
        extra: Vec::new(),
    });
    root_rels.push(Relationship {
        id: "rId3".into(),
        rel_type: REL_APP.into(),
        target: "docProps/app.xml".into(),
        target_mode: None,
        extra: Vec::new(),
    });

    let mut wb_rels = Relationships { rels: Vec::new() };
    let mut rid = 1u32;
    let mut ws_n = 0u32;
    for sh in &sheets {
        if sh.chartsheet.is_some() {
            continue;
        }
        ws_n += 1;
        types
            .set_override(&format!("/xl/worksheets/sheet{ws_n}.xml"), CT_WORKSHEET)
            .unwrap();
        wb_rels.push(Relationship {
            id: format!("rId{rid}"),
            rel_type: REL_WORKSHEET.into(),
            target: format!("worksheets/sheet{ws_n}.xml"),
            target_mode: None,
            extra: Vec::new(),
        });
        rid += 1;
    }
    let styles_rid = format!("rId{rid}");
    wb_rels.push(Relationship {
        id: styles_rid,
        rel_type: REL_STYLES.into(),
        target: "styles.xml".into(),
        target_mode: None,
        extra: Vec::new(),
    });
    rid += 1;
    if !sst.is_empty() {
        types
            .set_override("/xl/sharedStrings.xml", CT_SHARED_STRINGS)
            .unwrap();
        wb_rels.push(Relationship {
            id: format!("rId{rid}"),
            rel_type: REL_SHARED_STRINGS.into(),
            target: "sharedStrings.xml".into(),
            target_mode: None,
            extra: Vec::new(),
        });
    }

    pkg.set(CONTENT_TYPES_PART, types.to_xml().into_bytes())?;
    pkg.set(ROOT_RELS_PART, root_rels.to_xml().into_bytes())?;
    pkg.set(
        "/xl/workbook.xml",
        workbook_xml(&sheets, has_formula).into_bytes(),
    )?;
    pkg.set("/xl/_rels/workbook.xml.rels", wb_rels.to_xml().into_bytes())?;
    pkg.set("/xl/styles.xml", styles_xml().into_bytes())?;
    pkg.set("/docProps/core.xml", core_xml().into_bytes())?;
    pkg.set("/docProps/app.xml", app_xml(&sheets).into_bytes())?;
    crate::props::apply_create(&mut pkg, obj.get("props"), "workbook.props")?;
    if !sst.is_empty() {
        pkg.set("/xl/sharedStrings.xml", sst_xml(&sst).into_bytes())?;
    }
    let mut ws_n = 0u32;
    for sh in &sheets {
        if sh.chartsheet.is_some() {
            continue;
        }
        ws_n += 1;
        pkg.set(
            &format!("/xl/worksheets/sheet{ws_n}.xml"),
            sheet_xml(sh, &sst)?.into_bytes(),
        )?;
    }
    attach_pictures(&mut pkg, &sheets)?;
    attach_model3ds(&mut pkg, &sheets)?;
    attach_charts(&mut pkg, &sheets)?;
    attach_drawings(&mut pkg, &sheets)?;
    attach_comments(&mut pkg, &sheets)?;
    attach_hyperlinks(&mut pkg, &sheets)?;
    attach_header_footers(&mut pkg, &sheets)?;
    attach_chartsheets(&mut pkg, &sheets)?;
    pkg.save_to_vec()
}

fn parse_sheet(v: &Value, i: usize) -> Result<NewSheet, Error> {
    let obj = v.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: format!("workbook.sheets[{i}]"),
                expected: TypeName::Object,
            },
            format!("workbook.sheets[{i}]"),
        )
    })?;
    let prefix = format!("workbook.sheets[{i}]");
    let kind = parse_create_sheet_kind(obj.get("kind"), &prefix)?;
    let name = obj.get("name").and_then(Value::as_str).ok_or_else(|| {
        Error::with_path(
            ErrorKind::InvalidSheetName {
                reason: SheetNameReason::Missing,
            },
            format!("{prefix}.name"),
        )
    })?;
    super::request::validate_sheet_name(name, format!("{prefix}.name"))?;
    if kind == "chartsheet" {
        return parse_chartsheet_sheet(obj, i, name);
    }
    for k in obj.keys() {
        if !matches!(
            k.as_str(),
            "name"
                | "kind"
                | "cells"
                | "pictures"
                | "charts"
                | "headerFooters"
                | "comments"
                | "hyperlinks"
                | "textBoxes"
                | "shapes"
                | "connectors"
                | "smartArts"
                | "model3ds"
                | "maths"
        ) {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("{prefix}.{k}"),
            ));
        }
    }
    let mut cells = Vec::new();
    if let Some(arr) = obj.get("cells") {
        let arr = arr.as_array().ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: format!("{prefix}.cells"),
                    expected: TypeName::Array,
                },
                format!("{prefix}.cells"),
            )
        })?;
        for (j, c) in arr.iter().enumerate() {
            if let Some(cell) = parse_new_cell(c, i, j)? {
                cells.push(cell);
            }
        }
    }
    cells.sort_by_key(|c| (c.refer.row, c.refer.col));
    let pictures = parse_pictures(obj.get("pictures"), i, name)?;
    let charts = parse_charts(obj.get("charts"), i, name)?;
    let header_footers = parse_header_footers(obj.get("headerFooters"), i, name)?;
    let comments = parse_comments(obj.get("comments"), i, name)?;
    let hyperlinks = parse_hyperlinks(obj.get("hyperlinks"), i, name)?;
    let text_boxes = parse_textboxes(obj.get("textBoxes"), i, name)?;
    let shapes = parse_shapes(obj.get("shapes"), i, name)?;
    let connectors = parse_connectors(obj.get("connectors"), i, name)?;
    let smartarts = parse_smartarts(obj.get("smartArts"), i, name)?;
    let model3ds = parse_model3ds(obj.get("model3ds"), i, name)?;
    let maths = parse_maths(obj.get("maths"), i, name)?;
    Ok(NewSheet {
        name: name.to_string(),
        chartsheet: None,
        cells,
        pictures,
        charts,
        header_footers,
        comments,
        hyperlinks,
        text_boxes,
        shapes,
        connectors,
        smartarts,
        model3ds,
        maths,
    })
}

fn parse_create_sheet_kind(v: Option<&Value>, prefix: &str) -> Result<&'static str, Error> {
    match v {
        None | Some(Value::Null) => Ok("worksheet"),
        Some(Value::String(s)) if s == "worksheet" => Ok("worksheet"),
        Some(Value::String(s)) if s == "chartsheet" => Ok("chartsheet"),
        Some(Value::String(_)) => Err(Error::with_path(
            ErrorKind::UnknownField {
                field: "kind".into(),
            },
            format!("{prefix}.kind"),
        )),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "kind".into(),
                expected: TypeName::String,
            },
            format!("{prefix}.kind"),
        )),
    }
}

fn parse_chartsheet_sheet(
    obj: &Map<String, Value>,
    i: usize,
    name: &str,
) -> Result<NewSheet, Error> {
    let prefix = format!("workbook.sheets[{i}]");
    for k in obj.keys() {
        if !matches!(
            k.as_str(),
            "name"
                | "kind"
                | "type"
                | "ofPieType"
                | "title"
                | "series"
                | "dataFrom"
                | "dataSheet"
                | "cx"
                | "cy"
        ) {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("{prefix}.{k}"),
            ));
        }
    }
    let common = parse_chart_common(obj, &prefix)?;
    let data_from = parse_required_a1_field(obj.get("dataFrom"), &prefix, "dataFrom")?;
    let data_sheet = match obj.get("dataSheet") {
        None | Some(Value::Null) => None,
        Some(Value::String(s)) if !s.is_empty() => Some(s.clone()),
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "dataSheet".into(),
                },
                format!("{prefix}.dataSheet"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "dataSheet".into(),
                    expected: TypeName::String,
                },
                format!("{prefix}.dataSheet"),
            ));
        }
    };
    Ok(NewSheet {
        name: name.to_string(),
        chartsheet: Some(InsertChartSheetOp {
            name: name.to_string(),
            data_sheet,
            chart_type: common.chart_type,
            of_pie_type: common.of_pie_type,
            title: common.title,
            series: common.series,
            cx: common.cx,
            cy: common.cy,
            data_from,
        }),
        cells: Vec::new(),
        pictures: Vec::new(),
        charts: Vec::new(),
        header_footers: Vec::new(),
        comments: Vec::new(),
        hyperlinks: Vec::new(),
        text_boxes: Vec::new(),
        shapes: Vec::new(),
        connectors: Vec::new(),
        smartarts: Vec::new(),
        model3ds: Vec::new(),
        maths: Vec::new(),
    })
}

fn each_worksheet(sheets: &[NewSheet]) -> impl Iterator<Item = (usize, String, &NewSheet)> {
    let mut n = 0u32;
    sheets.iter().enumerate().filter_map(move |(i, sh)| {
        if sh.chartsheet.is_some() {
            None
        } else {
            n += 1;
            Some((i, format!("/xl/worksheets/sheet{n}.xml"), sh))
        }
    })
}

fn attach_chartsheets(pkg: &mut Package, sheets: &[NewSheet]) -> Result<(), Error> {
    if sheets.iter().all(|s| s.chartsheet.is_none()) {
        return Ok(());
    }
    let wb_part = "/xl/workbook.xml";
    let mut wb_doc = xml::parse(pkg.get(wb_part)?)?;
    let mut sheet_index = Vec::new();
    for (_, part, sh) in each_worksheet(sheets) {
        sheet_index.push((sh.name.clone(), part, "worksheet"));
    }
    let mut dirty = Vec::new();
    for (i, sh) in sheets.iter().enumerate() {
        let Some(op) = &sh.chartsheet else {
            continue;
        };
        chartsheet::insert(
            pkg,
            &mut wb_doc,
            wb_part,
            &mut sheet_index,
            &mut dirty,
            op,
            &format!("workbook.sheets[{i}]"),
            i,
        )?;
    }
    for (p, doc) in dirty {
        pkg.set(&p, doc.to_bytes()?)?;
    }
    reorder_workbook_sheets(
        &mut wb_doc,
        &sheets.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(),
    );
    pkg.set(wb_part, wb_doc.to_bytes()?)?;
    Ok(())
}

fn reorder_workbook_sheets(wb: &mut xml::Document, names: &[&str]) {
    let Some(sheets) = wb.root.child_by_local_mut("sheets") else {
        return;
    };
    let mut elems: Vec<xml::Element> = sheets
        .children
        .iter()
        .filter_map(|n| match n {
            xml::Node::Element(e) if e.local_name() == "sheet" => Some(e.clone()),
            _ => None,
        })
        .collect();
    elems.sort_by_key(|e| {
        let name = e.attr("name").unwrap_or("");
        names
            .iter()
            .position(|n| name.eq_ignore_ascii_case(n))
            .unwrap_or(usize::MAX)
    });
    sheets
        .children
        .retain(|n| !matches!(n, xml::Node::Element(e) if e.local_name() == "sheet"));
    for e in elems {
        sheets.push_element(e);
    }
}

fn attach_pictures(pkg: &mut Package, sheets: &[NewSheet]) -> Result<(), Error> {
    for (i, part, sh) in each_worksheet(sheets) {
        if sh.pictures.is_empty() {
            continue;
        }
        let mut doc = xml::parse(pkg.get(&part)?)?;
        for (j, op) in sh.pictures.iter().enumerate() {
            picture::insert_picture(
                pkg,
                &mut doc,
                &part,
                op,
                &format!("workbook.sheets[{i}].pictures[{j}].file"),
            )?;
        }
        pkg.set(&part, doc.to_bytes()?)?;
    }
    Ok(())
}

fn attach_model3ds(pkg: &mut Package, sheets: &[NewSheet]) -> Result<(), Error> {
    for (i, part, sh) in each_worksheet(sheets) {
        if sh.model3ds.is_empty() {
            continue;
        }
        let mut doc = xml::parse(pkg.get(&part)?)?;
        for (j, op) in sh.model3ds.iter().enumerate() {
            super::model3d::insert(
                pkg,
                &mut doc,
                &part,
                op,
                &format!("workbook.sheets[{i}].model3ds[{j}].file"),
            )?;
        }
        pkg.set(&part, doc.to_bytes()?)?;
    }
    Ok(())
}

fn attach_charts(pkg: &mut Package, sheets: &[NewSheet]) -> Result<(), Error> {
    for (i, part, sh) in each_worksheet(sheets) {
        if sh.charts.is_empty() {
            continue;
        }
        let mut dirty = Vec::new();
        for op in &sh.charts {
            chart::insert_chart(pkg, &part, &sh.name, &mut dirty, op, i)?;
        }
        for (p, doc) in dirty {
            pkg.set(&p, doc.to_bytes()?)?;
        }
    }
    Ok(())
}

fn attach_header_footers(pkg: &mut Package, sheets: &[NewSheet]) -> Result<(), Error> {
    for (i, part, sh) in each_worksheet(sheets) {
        if sh.header_footers.is_empty() {
            continue;
        }
        let mut doc = xml::parse(pkg.get(&part)?)?;
        for slot in &sh.header_footers {
            if let Some(text) = &slot.text {
                if !text.is_empty() {
                    hf::set_text(
                        &mut doc.root,
                        &SetHeaderFooterTextOp {
                            id: slot.slot.id(&sh.name),
                            sheet: sh.name.clone(),
                            slot: slot.slot,
                            text: text.clone(),
                        },
                    );
                    continue;
                }
            }
            if matches!(slot.slot.kind_str(), "even" | "first") {
                hf::ensure_create_slot_flags(&mut doc.root, slot.slot);
            }
        }
        for (j, slot) in sh.header_footers.iter().enumerate() {
            for (k, op) in slot.pictures.iter().enumerate() {
                hf::insert_picture(
                    pkg,
                    &mut doc,
                    &part,
                    op,
                    &format!("workbook.sheets[{i}].headerFooters[{j}].pictures[{k}].file"),
                )?;
            }
        }
        pkg.set(&part, doc.to_bytes()?)?;
    }
    Ok(())
}

fn attach_drawings(pkg: &mut Package, sheets: &[NewSheet]) -> Result<(), Error> {
    for (_, part, sh) in each_worksheet(sheets) {
        if sh.text_boxes.is_empty()
            && sh.maths.is_empty()
            && sh.shapes.is_empty()
            && sh.connectors.is_empty()
            && sh.smartarts.is_empty()
        {
            continue;
        }
        let mut doc = xml::parse(pkg.get(&part)?)?;
        for op in &sh.text_boxes {
            shape::insert_textbox(pkg, &mut doc, &part, op)?;
        }
        for op in &sh.maths {
            super::omml::insert(pkg, &mut doc, &part, op)?;
        }
        for op in &sh.shapes {
            shape::insert_shape(pkg, &mut doc, &part, op)?;
        }
        for op in &sh.connectors {
            shape::insert_connector(pkg, &mut doc, &part, op)?;
        }
        for op in &sh.smartarts {
            smartart::insert_smartart(pkg, &mut doc, &part, op)?;
        }
        pkg.set(&part, doc.to_bytes()?)?;
    }
    Ok(())
}

fn attach_comments(pkg: &mut Package, sheets: &[NewSheet]) -> Result<(), Error> {
    const WB_PART: &str = "/xl/workbook.xml";
    for (_, part, sh) in each_worksheet(sheets) {
        if sh.comments.is_empty() {
            continue;
        }
        let mut dirty = Vec::new();
        for op in &sh.comments {
            let cell = a1::parse_cell(&op.a1)?;
            comments::apply_insert(pkg, WB_PART, &mut dirty, &part, cell, op)?;
        }
        for (p, doc) in dirty {
            pkg.set(&p, doc.to_bytes()?)?;
        }
    }
    Ok(())
}

fn attach_hyperlinks(pkg: &mut Package, sheets: &[NewSheet]) -> Result<(), Error> {
    for (i, part, sh) in each_worksheet(sheets) {
        if sh.hyperlinks.is_empty() {
            continue;
        }
        let mut doc = xml::parse(pkg.get(&part)?)?;
        for (j, op) in sh.hyperlinks.iter().enumerate() {
            hyperlink::apply_insert(
                pkg,
                &mut doc.root,
                &part,
                op,
                &format!("workbook.sheets[{i}].hyperlinks[{j}].ref"),
            )?;
        }
        pkg.set(&part, doc.to_bytes()?)?;
    }
    Ok(())
}

fn parse_object_array<'a>(
    v: Option<&'a Value>,
    field: &str,
    prefix: &str,
) -> Result<Option<&'a [Value]>, Error> {
    let Some(v) = v else {
        return Ok(None);
    };
    let arr = v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Array,
            },
            prefix.to_string(),
        )
    })?;
    Ok(Some(arr.as_slice()))
}

fn deny_unknown(obj: &Map<String, Value>, prefix: &str, allowed: &[&str]) -> Result<(), Error> {
    for k in obj.keys() {
        if !allowed.iter().any(|a| *a == k) {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("{prefix}.{k}"),
            ));
        }
    }
    Ok(())
}

fn require_object<'a>(
    v: &'a Value,
    field: &str,
    prefix: &str,
) -> Result<&'a Map<String, Value>, Error> {
    v.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Object,
            },
            prefix.to_string(),
        )
    })
}

struct CreateAnchor {
    from: CellRef,
    to: Option<CellRef>,
    anchor: AnchorKind,
    from_off_x: i64,
    from_off_y: i64,
    to_off_x: i64,
    to_off_y: i64,
    cx: Option<i64>,
    cy: Option<i64>,
}

fn parse_create_anchor(obj: &Map<String, Value>, prefix: &str) -> Result<CreateAnchor, Error> {
    let from = parse_required_a1_field(obj.get("from"), prefix, "from")?;
    let cx = parse_emu(obj.get("cx"), prefix, "cx")?;
    let cy = parse_emu(obj.get("cy"), prefix, "cy")?;
    if cx.is_some() != cy.is_some() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "create".into(),
                field: "cx and cy together".into(),
            },
            prefix.to_string(),
        ));
    }
    let anchor = match obj.get("anchor") {
        None => AnchorKind::OneCell,
        Some(Value::String(s)) => AnchorKind::parse_insert(s).ok_or_else(|| {
            Error::with_path(ErrorKind::InvalidXlsxAnchor, format!("{prefix}.anchor"))
        })?,
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "anchor".into(),
                    expected: TypeName::String,
                },
                format!("{prefix}.anchor"),
            ));
        }
    };
    let to = match obj.get("to") {
        None => None,
        Some(Value::String(s)) => {
            Some(a1::parse_cell(s).map_err(|e| e.at(format!("{prefix}.to")))?)
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "to".into(),
                    expected: TypeName::String,
                },
                format!("{prefix}.to"),
            ));
        }
    };
    if anchor == AnchorKind::TwoCell && to.is_none() {
        return Err(Error::with_path(
            ErrorKind::MissingField { field: "to".into() },
            format!("{prefix}.to"),
        ));
    }
    if anchor == AnchorKind::OneCell && to.is_some() {
        return Err(Error::with_path(
            ErrorKind::UnknownField { field: "to".into() },
            format!("{prefix}.to"),
        ));
    }
    Ok(CreateAnchor {
        from,
        to,
        anchor,
        from_off_x: parse_off(obj.get("fromOffX"), prefix, "fromOffX")?.unwrap_or(0),
        from_off_y: parse_off(obj.get("fromOffY"), prefix, "fromOffY")?.unwrap_or(0),
        to_off_x: parse_off(obj.get("toOffX"), prefix, "toOffX")?.unwrap_or(0),
        to_off_y: parse_off(obj.get("toOffY"), prefix, "toOffY")?.unwrap_or(0),
        cx,
        cy,
    })
}

fn parse_opt_text_field(obj: &Map<String, Value>, prefix: &str) -> Result<Option<String>, Error> {
    match obj.get("text") {
        None => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "text".into(),
                expected: TypeName::String,
            },
            format!("{prefix}.text"),
        )),
    }
}

fn parse_comments(
    v: Option<&Value>,
    sheet_i: usize,
    sheet: &str,
) -> Result<Vec<InsertCommentOp>, Error> {
    let prefix = format!("workbook.sheets[{sheet_i}].comments");
    let Some(arr) = parse_object_array(v, "comments", &prefix)? else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for (j, item) in arr.iter().enumerate() {
        out.push(parse_comment(item, &format!("{prefix}[{j}]"), sheet)?);
    }
    Ok(out)
}

fn parse_comment(v: &Value, prefix: &str, sheet: &str) -> Result<InsertCommentOp, Error> {
    let obj = require_object(v, "comments", prefix)?;
    deny_unknown(obj, prefix, &["cell", "text", "author", "initials", "date"])?;
    let cell = parse_required_a1_field(obj.get("cell"), prefix, "cell")?;
    let text = match obj.get("text") {
        None => String::new(),
        Some(Value::String(s)) => s.clone(),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "text".into(),
                    expected: TypeName::String,
                },
                format!("{prefix}.text"),
            ));
        }
    };
    let author = parse_comment_str(obj, prefix, "author", "ooxp")?;
    let initials = parse_comment_str(obj, prefix, "initials", "oo")?;
    let date = match obj.get("date") {
        None => None,
        Some(Value::String(s)) if !s.is_empty() => Some(s.clone()),
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "date".into(),
                    expected: TypeName::NonEmptyString,
                },
                format!("{prefix}.date"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "date".into(),
                    expected: TypeName::String,
                },
                format!("{prefix}.date"),
            ));
        }
    };
    Ok(InsertCommentOp {
        sheet: sheet.to_string(),
        a1: a1::format_cell(cell.col, cell.row),
        text,
        author,
        initials,
        date,
    })
}

fn parse_comment_str(
    obj: &Map<String, Value>,
    prefix: &str,
    field: &str,
    default: &str,
) -> Result<String, Error> {
    match obj.get(field) {
        None => Ok(default.to_string()),
        Some(Value::String(s)) => Ok(s.clone()),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::String,
            },
            format!("{prefix}.{field}"),
        )),
    }
}

fn parse_hyperlinks(
    v: Option<&Value>,
    sheet_i: usize,
    sheet: &str,
) -> Result<Vec<InsertHyperlinkOp>, Error> {
    let prefix = format!("workbook.sheets[{sheet_i}].hyperlinks");
    let Some(arr) = parse_object_array(v, "hyperlinks", &prefix)? else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for (j, item) in arr.iter().enumerate() {
        out.push(parse_hyperlink(item, &format!("{prefix}[{j}]"), sheet)?);
    }
    Ok(out)
}

fn parse_hyperlink(v: &Value, prefix: &str, sheet: &str) -> Result<InsertHyperlinkOp, Error> {
    let obj = require_object(v, "hyperlinks", prefix)?;
    deny_unknown(
        obj,
        prefix,
        &["ref", "url", "location", "tooltip", "display"],
    )?;
    let (refer, start, end) = match obj.get("ref") {
        Some(Value::String(s)) => {
            let (start, end) = a1::parse_range(s).map_err(|e| e.at(format!("{prefix}.ref")))?;
            (a1::format_range(start, end), start, end)
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "ref".into(),
                },
                format!("{prefix}.ref"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "ref".into(),
                    expected: TypeName::String,
                },
                format!("{prefix}.ref"),
            ));
        }
    };
    let url = parse_opt_target(obj, prefix, "url", ErrorKind::EmptyHyperlinkUrl)?;
    let location = parse_opt_target(obj, prefix, "location", ErrorKind::EmptyHyperlinkLocation)?;
    if url.is_none() && location.is_none() {
        return Err(Error::with_path(
            ErrorKind::XlsxHyperlinkNeedTarget,
            prefix.to_string(),
        ));
    }
    Ok(InsertHyperlinkOp {
        sheet: sheet.to_string(),
        refer,
        start,
        end,
        url,
        location,
        tooltip: parse_opt_plain(obj, prefix, "tooltip")?,
        display: parse_opt_plain(obj, prefix, "display")?,
    })
}

fn parse_opt_target(
    obj: &Map<String, Value>,
    prefix: &str,
    field: &str,
    empty: ErrorKind,
) -> Result<Option<String>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::String(s)) if s.is_empty() => {
            Err(Error::with_path(empty, format!("{prefix}.{field}")))
        }
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::String,
            },
            format!("{prefix}.{field}"),
        )),
    }
}

fn parse_opt_plain(
    obj: &Map<String, Value>,
    prefix: &str,
    field: &str,
) -> Result<Option<String>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::String,
            },
            format!("{prefix}.{field}"),
        )),
    }
}

fn parse_textboxes(
    v: Option<&Value>,
    sheet_i: usize,
    sheet: &str,
) -> Result<Vec<InsertTextBoxOp>, Error> {
    let prefix = format!("workbook.sheets[{sheet_i}].textBoxes");
    let Some(arr) = parse_object_array(v, "textBoxes", &prefix)? else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for (j, item) in arr.iter().enumerate() {
        out.push(parse_textbox(item, &format!("{prefix}[{j}]"), sheet)?);
    }
    Ok(out)
}

fn parse_textbox(v: &Value, prefix: &str, sheet: &str) -> Result<InsertTextBoxOp, Error> {
    let obj = require_object(v, "textBoxes", prefix)?;
    deny_unknown(
        obj,
        prefix,
        &[
            "text",
            "cx",
            "cy",
            "from",
            "to",
            "anchor",
            "fromOffX",
            "fromOffY",
            "toOffX",
            "toOffY",
            "sz",
            "color",
            "textFill",
            "textOutline",
            "bold",
            "italic",
            "underline",
            "font",
            "eastAsiaFont",
            "align",
            "textAnchor",
        ],
    )?;
    let a = parse_create_anchor(obj, prefix)?;
    Ok(InsertTextBoxOp {
        sheet: sheet.to_string(),
        text: parse_opt_text_field(obj, prefix)?,
        cx: a.cx,
        cy: a.cy,
        from: a.from,
        to: a.to,
        anchor: a.anchor,
        from_off_x: a.from_off_x,
        from_off_y: a.from_off_y,
        to_off_x: a.to_off_x,
        to_off_y: a.to_off_y,
        format: crate::drawing_text::parse(obj, prefix, false)?,
    })
}

fn parse_shapes(
    v: Option<&Value>,
    sheet_i: usize,
    sheet: &str,
) -> Result<Vec<InsertShapeOp>, Error> {
    let prefix = format!("workbook.sheets[{sheet_i}].shapes");
    let Some(arr) = parse_object_array(v, "shapes", &prefix)? else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for (j, item) in arr.iter().enumerate() {
        out.push(parse_shape(item, &format!("{prefix}[{j}]"), sheet)?);
    }
    Ok(out)
}

fn parse_shape(v: &Value, prefix: &str, sheet: &str) -> Result<InsertShapeOp, Error> {
    let obj = require_object(v, "shapes", prefix)?;
    deny_unknown(
        obj,
        prefix,
        &[
            "text",
            "cx",
            "cy",
            "from",
            "to",
            "anchor",
            "fromOffX",
            "fromOffY",
            "toOffX",
            "toOffY",
            "prst",
            "sz",
            "color",
            "textFill",
            "textOutline",
            "bold",
            "italic",
            "underline",
            "font",
            "eastAsiaFont",
            "align",
            "textAnchor",
        ],
    )?;
    let a = parse_create_anchor(obj, prefix)?;
    Ok(InsertShapeOp {
        sheet: sheet.to_string(),
        text: parse_opt_text_field(obj, prefix)?,
        cx: a.cx,
        cy: a.cy,
        from: a.from,
        to: a.to,
        anchor: a.anchor,
        from_off_x: a.from_off_x,
        from_off_y: a.from_off_y,
        to_off_x: a.to_off_x,
        to_off_y: a.to_off_y,
        prst: parse_shape_prst(obj, prefix)?,
        format: crate::drawing_text::parse(obj, prefix, false)?,
    })
}

fn parse_connectors(
    v: Option<&Value>,
    sheet_i: usize,
    sheet: &str,
) -> Result<Vec<InsertConnectorOp>, Error> {
    let prefix = format!("workbook.sheets[{sheet_i}].connectors");
    let Some(arr) = parse_object_array(v, "connectors", &prefix)? else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for (j, item) in arr.iter().enumerate() {
        out.push(parse_connector(item, &format!("{prefix}[{j}]"), sheet)?);
    }
    Ok(out)
}

fn parse_connector(v: &Value, prefix: &str, sheet: &str) -> Result<InsertConnectorOp, Error> {
    let obj = require_object(v, "connectors", prefix)?;
    deny_unknown(
        obj,
        prefix,
        &[
            "cx", "cy", "from", "to", "anchor", "fromOffX", "fromOffY", "toOffX", "toOffY", "prst",
        ],
    )?;
    let a = parse_create_anchor(obj, prefix)?;
    Ok(InsertConnectorOp {
        sheet: sheet.to_string(),
        cx: a.cx,
        cy: a.cy,
        from: a.from,
        to: a.to,
        anchor: a.anchor,
        from_off_x: a.from_off_x,
        from_off_y: a.from_off_y,
        to_off_x: a.to_off_x,
        to_off_y: a.to_off_y,
        prst: parse_connector_prst(obj, prefix)?,
    })
}

fn parse_shape_prst(obj: &Map<String, Value>, prefix: &str) -> Result<String, Error> {
    match obj.get("prst") {
        None => Ok("rect".to_string()),
        Some(Value::String(s)) => {
            if crate::pptx::shape::is_writable_shape_prst(s) {
                Ok(s.clone())
            } else {
                Err(Error::with_path(
                    ErrorKind::InvalidShapePrst,
                    format!("{prefix}.prst"),
                ))
            }
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "prst".into(),
                expected: TypeName::String,
            },
            format!("{prefix}.prst"),
        )),
    }
}

fn parse_connector_prst(obj: &Map<String, Value>, prefix: &str) -> Result<String, Error> {
    match obj.get("prst") {
        None => Ok("straightConnector1".to_string()),
        Some(Value::String(s)) => {
            if s == "line" || crate::pptx::connector::is_writable_prst(s) {
                Ok(s.clone())
            } else {
                Err(Error::with_path(
                    ErrorKind::InvalidConnectorPrst,
                    format!("{prefix}.prst"),
                ))
            }
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "prst".into(),
                expected: TypeName::String,
            },
            format!("{prefix}.prst"),
        )),
    }
}

fn parse_smartarts(
    v: Option<&Value>,
    sheet_i: usize,
    sheet: &str,
) -> Result<Vec<InsertSmartArtOp>, Error> {
    let prefix = format!("workbook.sheets[{sheet_i}].smartArts");
    let Some(arr) = parse_object_array(v, "smartArts", &prefix)? else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for (j, item) in arr.iter().enumerate() {
        out.push(parse_smartart(item, &format!("{prefix}[{j}]"), sheet)?);
    }
    Ok(out)
}

fn parse_smartart(v: &Value, prefix: &str, sheet: &str) -> Result<InsertSmartArtOp, Error> {
    let obj = require_object(v, "smartArts", prefix)?;
    deny_unknown(
        obj,
        prefix,
        &[
            "layout", "points", "cx", "cy", "from", "to", "anchor", "fromOffX", "fromOffY",
            "toOffX", "toOffY",
        ],
    )?;
    let (layout, points) = crate::smartart::parse_layout_and_points(obj, prefix)?;
    let a = parse_create_anchor(obj, prefix)?;
    Ok(InsertSmartArtOp {
        sheet: sheet.to_string(),
        layout,
        points,
        cx: a.cx,
        cy: a.cy,
        from: a.from,
        to: a.to,
        anchor: a.anchor,
        from_off_x: a.from_off_x,
        from_off_y: a.from_off_y,
        to_off_x: a.to_off_x,
        to_off_y: a.to_off_y,
    })
}

fn parse_header_footers(
    v: Option<&Value>,
    sheet_i: usize,
    sheet: &str,
) -> Result<Vec<NewHfSlot>, Error> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    let prefix = format!("workbook.sheets[{sheet_i}].headerFooters");
    let arr = v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "headerFooters".into(),
                expected: TypeName::Array,
            },
            prefix.clone(),
        )
    })?;
    let mut out = Vec::new();
    let mut seen = [false; 6];
    for (j, item) in arr.iter().enumerate() {
        let item_path = format!("{prefix}[{j}]");
        let obj = item.as_object().ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: "headerFooters".into(),
                    expected: TypeName::Object,
                },
                item_path.clone(),
            )
        })?;
        for k in obj.keys() {
            if !matches!(k.as_str(), "kind" | "side" | "text" | "pictures") {
                return Err(Error::with_path(
                    ErrorKind::UnknownField { field: k.clone() },
                    format!("{item_path}.{k}"),
                ));
            }
        }
        let kind = match obj.get("kind") {
            Some(Value::String(s)) => s.as_str(),
            None => {
                return Err(Error::with_path(
                    ErrorKind::MissingField {
                        field: "kind".into(),
                    },
                    format!("{item_path}.kind"),
                ));
            }
            Some(_) => {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: "kind".into(),
                        expected: TypeName::String,
                    },
                    format!("{item_path}.kind"),
                ));
            }
        };
        let side = match obj.get("side") {
            Some(Value::String(s)) => s.as_str(),
            None => {
                return Err(Error::with_path(
                    ErrorKind::MissingField {
                        field: "side".into(),
                    },
                    format!("{item_path}.side"),
                ));
            }
            Some(_) => {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: "side".into(),
                        expected: TypeName::String,
                    },
                    format!("{item_path}.side"),
                ));
            }
        };
        if !matches!(kind, "odd" | "even" | "first") {
            return Err(Error::with_path(
                ErrorKind::InvalidXlsxHfKind,
                format!("{item_path}.kind"),
            ));
        }
        if !matches!(side, "header" | "footer") {
            return Err(Error::with_path(
                ErrorKind::InvalidXlsxHfSide,
                format!("{item_path}.side"),
            ));
        }
        let slot = HfSlot::parse(kind, side).expect("kind 与 side 已校验");
        let idx = slot.index();
        if seen[idx] {
            return Err(Error::with_path(
                ErrorKind::CreateXlsxHfDuplicate,
                item_path.clone(),
            ));
        }
        seen[idx] = true;
        let text = match obj.get("text") {
            None | Some(Value::Null) => None,
            Some(Value::String(s)) => Some(s.clone()),
            Some(_) => {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: "text".into(),
                        expected: TypeName::String,
                    },
                    format!("{item_path}.text"),
                ));
            }
        };
        let pictures = parse_hf_pictures(obj.get("pictures"), &item_path, sheet, slot)?;
        let has_text = text.as_ref().is_some_and(|s| !s.is_empty());
        if !has_text && pictures.is_empty() {
            return Err(Error::with_path(ErrorKind::NeedXlsxHfContent, item_path));
        }
        out.push(NewHfSlot {
            slot,
            text,
            pictures,
        });
    }
    Ok(out)
}

fn parse_hf_pictures(
    v: Option<&Value>,
    item_path: &str,
    sheet: &str,
    slot: HfSlot,
) -> Result<Vec<InsertPictureOp>, Error> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    let prefix = format!("{item_path}.pictures");
    let arr = v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "pictures".into(),
                expected: TypeName::Array,
            },
            prefix.clone(),
        )
    })?;
    let mut out = Vec::new();
    for (k, item) in arr.iter().enumerate() {
        out.push(parse_hf_picture(
            item,
            &format!("{prefix}[{k}]"),
            sheet,
            slot,
        )?);
    }
    Ok(out)
}

fn parse_hf_picture(
    v: &Value,
    prefix: &str,
    sheet: &str,
    slot: HfSlot,
) -> Result<InsertPictureOp, Error> {
    let obj = v.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "pictures".into(),
                expected: TypeName::Object,
            },
            prefix.to_string(),
        )
    })?;
    for k in obj.keys() {
        if matches!(
            k.as_str(),
            "from" | "to" | "anchor" | "fromOffX" | "fromOffY" | "toOffX" | "toOffY"
        ) {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("{prefix}.{k}"),
            ));
        }
        if !matches!(k.as_str(), "file" | "cx" | "cy") {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("{prefix}.{k}"),
            ));
        }
    }
    let file = match obj.get("file") {
        Some(Value::String(s)) if !s.is_empty() => s.clone(),
        Some(Value::String(_)) | None => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "file".into(),
                },
                format!("{prefix}.file"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "file".into(),
                    expected: TypeName::NonEmptyString,
                },
                format!("{prefix}.file"),
            ));
        }
    };
    let cx = parse_emu(obj.get("cx"), prefix, "cx")?;
    let cy = parse_emu(obj.get("cy"), prefix, "cy")?;
    if cx.is_some() != cy.is_some() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "create".into(),
                field: "cx and cy together".into(),
            },
            prefix.to_string(),
        ));
    }
    Ok(InsertPictureOp {
        sheet: sheet.to_string(),
        hf: Some(slot),
        file,
        cx,
        cy,
        from: CellRef { col: 1, row: 1 },
        to: None,
        anchor: AnchorKind::OneCell,
        from_off_x: 0,
        from_off_y: 0,
        to_off_x: 0,
        to_off_y: 0,
    })
}

fn parse_pictures(
    v: Option<&Value>,
    sheet_i: usize,
    sheet: &str,
) -> Result<Vec<InsertPictureOp>, Error> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    let prefix = format!("workbook.sheets[{sheet_i}].pictures");
    let arr = v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "pictures".into(),
                expected: TypeName::Array,
            },
            prefix.clone(),
        )
    })?;
    let mut out = Vec::new();
    for (j, item) in arr.iter().enumerate() {
        out.push(parse_picture(item, &format!("{prefix}[{j}]"), sheet)?);
    }
    Ok(out)
}

fn parse_model3ds(
    v: Option<&Value>,
    sheet_i: usize,
    sheet: &str,
) -> Result<Vec<super::model3d::InsertModel3dOp>, Error> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    let prefix = format!("workbook.sheets[{sheet_i}].model3ds");
    let arr = v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "model3ds".into(),
                expected: TypeName::Array,
            },
            prefix.clone(),
        )
    })?;
    let mut out = Vec::new();
    for (j, item) in arr.iter().enumerate() {
        out.push(parse_create_model3d(
            item,
            &format!("{prefix}[{j}]"),
            sheet,
        )?);
    }
    Ok(out)
}

fn parse_maths(
    v: Option<&Value>,
    sheet_i: usize,
    sheet: &str,
) -> Result<Vec<super::omml::InsertMathOp>, Error> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    let prefix = format!("workbook.sheets[{sheet_i}].maths");
    let arr = v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "maths".into(),
                expected: TypeName::Array,
            },
            prefix.clone(),
        )
    })?;
    let mut out = Vec::new();
    for (j, item) in arr.iter().enumerate() {
        out.push(super::omml::parse_create(
            item,
            &format!("{prefix}[{j}]"),
            sheet,
        )?);
    }
    Ok(out)
}

fn parse_create_model3d(
    v: &Value,
    prefix: &str,
    sheet: &str,
) -> Result<super::model3d::InsertModel3dOp, Error> {
    let obj = v.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: prefix.into(),
                expected: TypeName::Object,
            },
            prefix,
        )
    })?;
    crate::json::deny_unknown(
        obj,
        &[
            "file", "cx", "cy", "from", "to", "anchor", "fromOffX", "fromOffY", "toOffX", "toOffY",
        ],
    )?;
    let file = match obj.get("file") {
        Some(Value::String(s)) if !s.is_empty() => s.clone(),
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "file".into(),
                    expected: TypeName::NonEmptyString,
                },
                format!("{prefix}.file"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "file".into(),
                },
                format!("{prefix}.file"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "file".into(),
                    expected: TypeName::String,
                },
                format!("{prefix}.file"),
            ));
        }
    };
    let cx = parse_emu(obj.get("cx"), prefix, "cx")?;
    let cy = parse_emu(obj.get("cy"), prefix, "cy")?;
    if cx.is_some() != cy.is_some() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "insertModel3d".into(),
                field: "cx and cy together".into(),
            },
            prefix.to_string(),
        ));
    }
    let from = match obj.get("from") {
        Some(Value::String(s)) => a1::parse_cell(s).map_err(|e| e.at(format!("{prefix}.from")))?,
        None => a1::CellRef { col: 1, row: 1 },
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "from".into(),
                    expected: TypeName::String,
                },
                format!("{prefix}.from"),
            ));
        }
    };
    let anchor = match obj.get("anchor") {
        None => AnchorKind::OneCell,
        Some(Value::String(s)) => picture::AnchorKind::parse_insert(s).ok_or_else(|| {
            Error::with_path(ErrorKind::InvalidXlsxAnchor, format!("{prefix}.anchor"))
        })?,
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "anchor".into(),
                    expected: TypeName::String,
                },
                format!("{prefix}.anchor"),
            ));
        }
    };
    let to = match obj.get("to") {
        None => None,
        Some(Value::String(s)) => {
            Some(a1::parse_cell(s).map_err(|e| e.at(format!("{prefix}.to")))?)
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "to".into(),
                    expected: TypeName::String,
                },
                format!("{prefix}.to"),
            ));
        }
    };
    if anchor == AnchorKind::TwoCell && to.is_none() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "insertModel3d".into(),
                field: "to".into(),
            },
            format!("{prefix}.to"),
        ));
    }
    if anchor == AnchorKind::OneCell && to.is_some() {
        return Err(Error::with_path(
            ErrorKind::UnknownField { field: "to".into() },
            format!("{prefix}.to"),
        ));
    }
    Ok(super::model3d::InsertModel3dOp {
        sheet: sheet.to_string(),
        file,
        cx,
        cy,
        from,
        to,
        anchor,
        from_off_x: parse_off(obj.get("fromOffX"), prefix, "fromOffX")?.unwrap_or(0),
        from_off_y: parse_off(obj.get("fromOffY"), prefix, "fromOffY")?.unwrap_or(0),
        to_off_x: parse_off(obj.get("toOffX"), prefix, "toOffX")?.unwrap_or(0),
        to_off_y: parse_off(obj.get("toOffY"), prefix, "toOffY")?.unwrap_or(0),
    })
}

fn parse_charts(
    v: Option<&Value>,
    sheet_i: usize,
    sheet: &str,
) -> Result<Vec<InsertChartOp>, Error> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    let prefix = format!("workbook.sheets[{sheet_i}].charts");
    let arr = v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "charts".into(),
                expected: TypeName::Array,
            },
            prefix.clone(),
        )
    })?;
    let mut out = Vec::new();
    for (j, item) in arr.iter().enumerate() {
        out.push(parse_chart(item, &format!("{prefix}[{j}]"), sheet)?);
    }
    Ok(out)
}

struct ChartCommon {
    chart_type: ChartType,
    of_pie_type: Option<String>,
    title: Option<String>,
    series: Vec<NewSeries>,
    cx: Option<i64>,
    cy: Option<i64>,
}

fn parse_chart_common(obj: &Map<String, Value>, prefix: &str) -> Result<ChartCommon, Error> {
    let chart_type = match obj.get("type") {
        Some(Value::String(s)) => ChartType::parse(s).ok_or_else(|| {
            Error::with_path(ErrorKind::InvalidChartType, format!("{prefix}.type"))
        })?,
        None => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "type".into(),
                },
                format!("{prefix}.type"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "type".into(),
                    expected: TypeName::String,
                },
                format!("{prefix}.type"),
            ));
        }
    };
    let title = match obj.get("title") {
        None | Some(Value::Null) => None,
        Some(Value::String(s)) => Some(s.clone()),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "title".into(),
                    expected: TypeName::String,
                },
                format!("{prefix}.title"),
            ));
        }
    };
    let series = parse_chart_series(obj.get("series"), prefix, chart_type)?;
    crate::chart::validate_insert_series_len(
        chart_type,
        series.len(),
        &format!("{prefix}.series"),
    )?;
    let of_pie_type =
        crate::chart::parse_of_pie_type(obj.get("ofPieType"), &format!("{prefix}.ofPieType"))?;
    crate::chart::reject_of_pie_on_other(
        chart_type,
        of_pie_type.as_deref(),
        &format!("{prefix}.ofPieType"),
    )?;
    if chart_type == ChartType::Combo {
        let kinds: HashSet<&str> = series
            .iter()
            .filter_map(|s| s.series_type.map(ChartType::as_str))
            .collect();
        if series.iter().any(|s| s.series_type.is_none()) || kinds.len() < 2 {
            return Err(Error::with_path(
                ErrorKind::InvalidComboSeriesType,
                format!("{prefix}.series"),
            ));
        }
    } else if series.iter().any(|s| s.series_type.is_some()) {
        return Err(Error::with_path(
            ErrorKind::InvalidComboSeriesType,
            format!("{prefix}.series"),
        ));
    }
    if chart_type != ChartType::Bubble && series.iter().any(|s| s.sizes.is_some()) {
        return Err(Error::with_path(
            ErrorKind::SizesOnNonBubble,
            format!("{prefix}.series"),
        ));
    }
    let cx = parse_emu(obj.get("cx"), prefix, "cx")?;
    let cy = parse_emu(obj.get("cy"), prefix, "cy")?;
    if cx.is_some() != cy.is_some() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "create".into(),
                field: "cx and cy together".into(),
            },
            prefix.to_string(),
        ));
    }
    Ok(ChartCommon {
        chart_type,
        of_pie_type,
        title,
        series,
        cx,
        cy,
    })
}

fn parse_chart(v: &Value, prefix: &str, sheet: &str) -> Result<InsertChartOp, Error> {
    let obj = v.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "charts".into(),
                expected: TypeName::Object,
            },
            prefix.to_string(),
        )
    })?;
    for k in obj.keys() {
        if matches!(k.as_str(), "kind" | "wrap") {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("{prefix}.{k}"),
            ));
        }
        if !matches!(
            k.as_str(),
            "type"
                | "ofPieType"
                | "title"
                | "series"
                | "from"
                | "to"
                | "anchor"
                | "cx"
                | "cy"
                | "fromOffX"
                | "fromOffY"
                | "toOffX"
                | "toOffY"
                | "dataFrom"
        ) {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("{prefix}.{k}"),
            ));
        }
    }
    let common = parse_chart_common(obj, prefix)?;
    let from = parse_required_a1_field(obj.get("from"), prefix, "from")?;
    let data_from = parse_required_a1_field(obj.get("dataFrom"), prefix, "dataFrom")?;
    let anchor = match obj.get("anchor") {
        None => AnchorKind::OneCell,
        Some(Value::String(s)) => AnchorKind::parse_insert(s).ok_or_else(|| {
            Error::with_path(ErrorKind::InvalidXlsxAnchor, format!("{prefix}.anchor"))
        })?,
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "anchor".into(),
                    expected: TypeName::String,
                },
                format!("{prefix}.anchor"),
            ));
        }
    };
    let to = match obj.get("to") {
        None => None,
        Some(Value::String(s)) => {
            Some(a1::parse_cell(s).map_err(|e| e.at(format!("{prefix}.to")))?)
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "to".into(),
                    expected: TypeName::String,
                },
                format!("{prefix}.to"),
            ));
        }
    };
    if anchor == AnchorKind::TwoCell && to.is_none() {
        return Err(Error::with_path(
            ErrorKind::MissingField { field: "to".into() },
            format!("{prefix}.to"),
        ));
    }
    if anchor == AnchorKind::OneCell && to.is_some() {
        return Err(Error::with_path(
            ErrorKind::UnknownField { field: "to".into() },
            format!("{prefix}.to"),
        ));
    }
    Ok(InsertChartOp {
        sheet: sheet.to_string(),
        chart_type: common.chart_type,
        of_pie_type: common.of_pie_type,
        title: common.title,
        series: common.series,
        cx: common.cx,
        cy: common.cy,
        from,
        to,
        anchor,
        from_off_x: parse_off(obj.get("fromOffX"), prefix, "fromOffX")?.unwrap_or(0),
        from_off_y: parse_off(obj.get("fromOffY"), prefix, "fromOffY")?.unwrap_or(0),
        to_off_x: parse_off(obj.get("toOffX"), prefix, "toOffX")?.unwrap_or(0),
        to_off_y: parse_off(obj.get("toOffY"), prefix, "toOffY")?.unwrap_or(0),
        data_from,
    })
}

fn parse_chart_series(
    v: Option<&Value>,
    prefix: &str,
    _chart_type: ChartType,
) -> Result<Vec<NewSeries>, Error> {
    let arr = match v {
        Some(Value::Array(a)) if !a.is_empty() => a,
        Some(Value::Array(_)) | None => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "series".into(),
                },
                format!("{prefix}.series"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "series".into(),
                    expected: TypeName::Array,
                },
                format!("{prefix}.series"),
            ));
        }
    };
    let mut out = Vec::new();
    for (j, item) in arr.iter().enumerate() {
        let obj = item.as_object().ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: format!("series[{j}]"),
                    expected: TypeName::Object,
                },
                format!("{prefix}.series[{j}]"),
            )
        })?;
        for k in obj.keys() {
            if !matches!(
                k.as_str(),
                "name" | "categories" | "values" | "sizes" | "type"
            ) {
                return Err(Error::with_path(
                    ErrorKind::UnknownField { field: k.clone() },
                    format!("{prefix}.series[{j}].{k}"),
                ));
            }
        }
        let name = match obj.get("name") {
            None | Some(Value::Null) => None,
            Some(Value::String(s)) => Some(s.clone()),
            Some(_) => {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: format!("series[{j}].name"),
                        expected: TypeName::String,
                    },
                    format!("{prefix}.series[{j}].name"),
                ));
            }
        };
        let categories = parse_chart_datum_list(
            obj.get("categories"),
            &format!("{prefix}.series[{j}].categories"),
        )?;
        let values =
            parse_chart_datum_list(obj.get("values"), &format!("{prefix}.series[{j}].values"))?
                .filter(|v| !v.is_empty())
                .ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::MissingField {
                            field: "series[].values".into(),
                        },
                        format!("{prefix}.series[{j}].values"),
                    )
                })?;
        let sizes =
            parse_chart_datum_list(obj.get("sizes"), &format!("{prefix}.series[{j}].sizes"))?;
        let series_type = match obj.get("type") {
            None | Some(Value::Null) => None,
            Some(Value::String(s)) => Some(ChartType::parse_combo_series(s).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::InvalidComboSeriesType,
                    format!("{prefix}.series[{j}].type"),
                )
            })?),
            Some(_) => {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: format!("series[{j}].type"),
                        expected: TypeName::String,
                    },
                    format!("{prefix}.series[{j}].type"),
                ));
            }
        };
        out.push(NewSeries {
            name,
            categories,
            values,
            sizes,
            series_type,
        });
    }
    Ok(out)
}

fn parse_chart_datum_list(v: Option<&Value>, path: &str) -> Result<Option<Vec<Value>>, Error> {
    match v {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Array(a)) => {
            for (j, item) in a.iter().enumerate() {
                if !matches!(
                    item,
                    Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_)
                ) {
                    return Err(Error::with_path(
                        ErrorKind::InvalidCellValue,
                        format!("{path}[{j}]"),
                    ));
                }
            }
            Ok(Some(a.clone()))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: path.rsplit('.').next().unwrap_or("series").into(),
                expected: TypeName::Array,
            },
            path.to_string(),
        )),
    }
}

fn parse_required_a1_field(v: Option<&Value>, prefix: &str, field: &str) -> Result<CellRef, Error> {
    match v {
        Some(Value::String(s)) => a1::parse_cell(s).map_err(|e| e.at(format!("{prefix}.{field}"))),
        None => Err(Error::with_path(
            ErrorKind::MissingField {
                field: field.into(),
            },
            format!("{prefix}.{field}"),
        )),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::String,
            },
            format!("{prefix}.{field}"),
        )),
    }
}

fn parse_picture(v: &Value, prefix: &str, sheet: &str) -> Result<InsertPictureOp, Error> {
    let obj = v.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "pictures".into(),
                expected: TypeName::Object,
            },
            prefix.to_string(),
        )
    })?;
    for k in obj.keys() {
        if !matches!(
            k.as_str(),
            "file"
                | "from"
                | "to"
                | "anchor"
                | "cx"
                | "cy"
                | "fromOffX"
                | "fromOffY"
                | "toOffX"
                | "toOffY"
        ) {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("{prefix}.{k}"),
            ));
        }
    }
    let file = match obj.get("file") {
        Some(Value::String(s)) if !s.is_empty() => s.clone(),
        Some(Value::String(_)) | None => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "file".into(),
                },
                format!("{prefix}.file"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "file".into(),
                    expected: TypeName::NonEmptyString,
                },
                format!("{prefix}.file"),
            ));
        }
    };
    let from = match obj.get("from") {
        Some(Value::String(s)) => a1::parse_cell(s).map_err(|e| e.at(format!("{prefix}.from")))?,
        None => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "from".into(),
                },
                format!("{prefix}.from"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "from".into(),
                    expected: TypeName::String,
                },
                format!("{prefix}.from"),
            ));
        }
    };
    let cx = parse_emu(obj.get("cx"), prefix, "cx")?;
    let cy = parse_emu(obj.get("cy"), prefix, "cy")?;
    if cx.is_some() != cy.is_some() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "create".into(),
                field: "cx and cy together".into(),
            },
            prefix.to_string(),
        ));
    }
    let anchor = match obj.get("anchor") {
        None => AnchorKind::OneCell,
        Some(Value::String(s)) => AnchorKind::parse_insert(s).ok_or_else(|| {
            Error::with_path(ErrorKind::InvalidXlsxAnchor, format!("{prefix}.anchor"))
        })?,
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "anchor".into(),
                    expected: TypeName::String,
                },
                format!("{prefix}.anchor"),
            ));
        }
    };
    let to = match obj.get("to") {
        None => None,
        Some(Value::String(s)) => {
            Some(a1::parse_cell(s).map_err(|e| e.at(format!("{prefix}.to")))?)
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "to".into(),
                    expected: TypeName::String,
                },
                format!("{prefix}.to"),
            ));
        }
    };
    if anchor == AnchorKind::TwoCell && to.is_none() {
        return Err(Error::with_path(
            ErrorKind::MissingField { field: "to".into() },
            format!("{prefix}.to"),
        ));
    }
    if anchor == AnchorKind::OneCell && to.is_some() {
        return Err(Error::with_path(
            ErrorKind::UnknownField { field: "to".into() },
            format!("{prefix}.to"),
        ));
    }
    Ok(InsertPictureOp {
        sheet: sheet.to_string(),
        hf: None,
        file,
        cx,
        cy,
        from,
        to,
        anchor,
        from_off_x: parse_off(obj.get("fromOffX"), prefix, "fromOffX")?.unwrap_or(0),
        from_off_y: parse_off(obj.get("fromOffY"), prefix, "fromOffY")?.unwrap_or(0),
        to_off_x: parse_off(obj.get("toOffX"), prefix, "toOffX")?.unwrap_or(0),
        to_off_y: parse_off(obj.get("toOffY"), prefix, "toOffY")?.unwrap_or(0),
    })
}

fn parse_emu(v: Option<&Value>, prefix: &str, field: &str) -> Result<Option<i64>, Error> {
    let Some(v) = v else {
        return Ok(None);
    };
    let path = format!("{prefix}.{field}");
    match v {
        Value::Number(n) => {
            if let Some(u) = n.as_u64() {
                if u == 0 || u > i64::MAX as u64 {
                    return Err(Error::with_path(
                        ErrorKind::MinValue {
                            field: field.into(),
                            min: 1,
                        },
                        path,
                    ));
                }
                return Ok(Some(u as i64));
            }
            Err(Error::with_path(
                ErrorKind::WrongType {
                    field: field.into(),
                    expected: TypeName::Integer,
                },
                path,
            ))
        }
        _ => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Integer,
            },
            path,
        )),
    }
}

fn parse_off(v: Option<&Value>, prefix: &str, field: &str) -> Result<Option<i64>, Error> {
    let Some(v) = v else {
        return Ok(None);
    };
    let path = format!("{prefix}.{field}");
    match v {
        Value::Number(n) => {
            if let Some(u) = n.as_u64() {
                if u > i64::MAX as u64 {
                    return Err(Error::with_path(
                        ErrorKind::WrongType {
                            field: field.into(),
                            expected: TypeName::Integer,
                        },
                        path,
                    ));
                }
                return Ok(Some(u as i64));
            }
            if let Some(i64v) = n.as_i64() {
                if i64v < 0 {
                    return Err(Error::with_path(
                        ErrorKind::MinValue {
                            field: field.into(),
                            min: 0,
                        },
                        path,
                    ));
                }
                return Ok(Some(i64v));
            }
            Err(Error::with_path(
                ErrorKind::WrongType {
                    field: field.into(),
                    expected: TypeName::Integer,
                },
                path,
            ))
        }
        _ => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Integer,
            },
            path,
        )),
    }
}

fn parse_new_cell(v: &Value, i: usize, j: usize) -> Result<Option<NewCell>, Error> {
    let obj = v.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: format!("workbook.sheets[{i}].cells[{j}]"),
                expected: TypeName::Object,
            },
            format!("workbook.sheets[{i}].cells[{j}]"),
        )
    })?;
    for k in obj.keys() {
        if !matches!(k.as_str(), "ref" | "value" | "formula") {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("workbook.sheets[{i}].cells[{j}].{k}"),
            ));
        }
    }
    let refer = obj.get("ref").and_then(Value::as_str).ok_or_else(|| {
        Error::with_path(
            ErrorKind::MissingField {
                field: "ref".into(),
            },
            format!("workbook.sheets[{i}].cells[{j}].ref"),
        )
    })?;
    let cell =
        a1::parse_cell(refer).map_err(|e| e.at(format!("workbook.sheets[{i}].cells[{j}].ref")))?;
    let has_v = obj.contains_key("value");
    let has_f = obj.contains_key("formula");
    if has_v && has_f {
        return Err(Error::with_path(
            ErrorKind::MutuallyExclusive {
                a: "value".into(),
                b: "formula".into(),
            },
            format!("workbook.sheets[{i}].cells[{j}]"),
        ));
    }
    if !has_v && !has_f {
        return Ok(None);
    }
    let formula = if has_f {
        let f = obj.get("formula").and_then(Value::as_str).ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: "formula".into(),
                    expected: TypeName::String,
                },
                format!("workbook.sheets[{i}].cells[{j}].formula"),
            )
        })?;
        if !f.starts_with('=') {
            return Err(Error::with_path(
                ErrorKind::FormulaMustStartWithEquals,
                format!("workbook.sheets[{i}].cells[{j}].formula"),
            ));
        }
        Some(f.to_string())
    } else {
        None
    };
    let value = if has_v {
        let parsed = parse_cell_value(
            &obj["value"],
            &format!("workbook.sheets[{i}].cells[{j}].value"),
        )?;
        if matches!(parsed, SetCellValue::Null) {
            return Ok(None);
        }
        Some(parsed)
    } else {
        None
    };
    Ok(Some(NewCell {
        refer: cell,
        a1: a1::format_cell(cell.col, cell.row),
        value,
        formula,
    }))
}

fn workbook_xml(sheets: &[NewSheet], has_formula: bool) -> String {
    let mut body = String::new();
    let mut i = 0u32;
    for sh in sheets {
        if sh.chartsheet.is_some() {
            continue;
        }
        i += 1;
        body.push_str(&format!(
            r#"<sheet name="{}" sheetId="{}" r:id="rId{}"/>"#,
            xml::escape_attr(&sh.name),
            i,
            i
        ));
    }
    let calc = if has_formula {
        r#"<calcPr fullCalcOnLoad="1"/>"#
    } else {
        ""
    };
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><workbook xmlns="{SML_NS}" xmlns:r="{REL_NS}"><sheets>{body}</sheets>{calc}</workbook>"#
    )
}

fn sheet_xml(sheet: &NewSheet, sst: &[String]) -> Result<String, Error> {
    use std::collections::BTreeMap;
    let mut rows: BTreeMap<u32, Vec<&NewCell>> = BTreeMap::new();
    for c in &sheet.cells {
        rows.entry(c.refer.row).or_default().push(c);
    }
    let mut dim = "A1".to_string();
    if !sheet.cells.is_empty() {
        let min_col = sheet.cells.iter().map(|c| c.refer.col).min().unwrap();
        let max_col = sheet.cells.iter().map(|c| c.refer.col).max().unwrap();
        let min_row = sheet.cells.iter().map(|c| c.refer.row).min().unwrap();
        let max_row = sheet.cells.iter().map(|c| c.refer.row).max().unwrap();
        dim = format!(
            "{}:{}",
            a1::format_cell(min_col, min_row),
            a1::format_cell(max_col, max_row)
        );
    }
    let mut data = String::new();
    for (r, cells) in rows {
        let mut inner = String::new();
        for c in cells {
            inner.push_str(&cell_xml(c, sst)?);
        }
        data.push_str(&format!(r#"<row r="{r}">{inner}</row>"#));
    }
    Ok(format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><worksheet xmlns="{SML_NS}" xmlns:r="{REL_NS}"><dimension ref="{dim}"/><sheetData>{data}</sheetData></worksheet>"#
    ))
}

fn cell_xml(c: &NewCell, sst: &[String]) -> Result<String, Error> {
    if let Some(f) = &c.formula {
        let body = xml::escape_text(f.strip_prefix('=').unwrap_or(f));
        return Ok(format!(r#"<c r="{}"><f>{body}</f></c>"#, c.a1));
    }
    match c.value.as_ref() {
        Some(SetCellValue::Number(n)) => {
            let v = if let Some(i) = n.as_i64() {
                i.to_string()
            } else if let Some(u) = n.as_u64() {
                u.to_string()
            } else {
                n.to_string()
            };
            Ok(format!(r#"<c r="{}"><v>{v}</v></c>"#, c.a1))
        }
        Some(SetCellValue::Bool(b)) => Ok(format!(
            r#"<c r="{}" t="b"><v>{}</v></c>"#,
            c.a1,
            if *b { "1" } else { "0" }
        )),
        Some(SetCellValue::Text(s)) => {
            let idx = sst
                .iter()
                .position(|x| x == s)
                .ok_or_else(|| Error::new(ErrorKind::SharedStringNotInTable { text: s.clone() }))?;
            Ok(format!(r#"<c r="{}" t="s"><v>{idx}</v></c>"#, c.a1))
        }
        _ => Ok(format!(r#"<c r="{}"/>"#, c.a1)),
    }
}

fn sst_xml(sst: &[String]) -> String {
    let count = sst.len();
    let mut body = String::new();
    for s in sst {
        let space = if s.starts_with(' ') || s.ends_with(' ') || s.contains('\n') {
            r#" xml:space="preserve""#
        } else {
            ""
        };
        body.push_str(&format!("<si><t{space}>{}</t></si>", xml::escape_text(s)));
    }
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><sst xmlns="{SML_NS}" count="{count}" uniqueCount="{count}">{body}</sst>"#
    )
}

fn styles_xml() -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><styleSheet xmlns="{SML_NS}"><fonts count="1"><font><sz val="11"/><color rgb="FF000000"/><name val="Calibri"/><family val="2"/></font></fonts><fills count="2"><fill><patternFill patternType="none"/></fill><fill><patternFill patternType="gray125"/></fill></fills><borders count="1"><border><left/><right/><top/><bottom/><diagonal/></border></borders><cellStyleXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/></cellStyleXfs><cellXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/></cellXfs><cellStyles count="1"><cellStyle name="Normal" xfId="0" builtinId="0"/></cellStyles></styleSheet>"#
    )
}

fn core_xml() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:dcterms="http://purl.org/dc/terms/" xmlns:dcmitype="http://purl.org/dc/dcmitype/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"><dc:title/><dc:creator>ooxp</dc:creator></cp:coreProperties>"#.to_string()
}

fn app_xml(sheets: &[NewSheet]) -> String {
    let n = sheets.len();
    let n_ws = sheets.iter().filter(|s| s.chartsheet.is_none()).count();
    let n_cs = n - n_ws;
    let mut titles = String::new();
    for sh in sheets {
        titles.push_str(&format!(
            "<vt:lpstr>{}</vt:lpstr>",
            xml::escape_text(&sh.name)
        ));
    }
    let pairs = if n_cs == 0 {
        format!(
            r#"<HeadingPairs><vt:vector size="2" baseType="variant"><vt:variant><vt:lpstr>Worksheets</vt:lpstr></vt:variant><vt:variant><vt:i4>{n_ws}</vt:i4></vt:variant></vt:vector></HeadingPairs>"#
        )
    } else {
        format!(
            r#"<HeadingPairs><vt:vector size="4" baseType="variant"><vt:variant><vt:lpstr>Worksheets</vt:lpstr></vt:variant><vt:variant><vt:i4>{n_ws}</vt:i4></vt:variant><vt:variant><vt:lpstr>Charts</vt:lpstr></vt:variant><vt:variant><vt:i4>{n_cs}</vt:i4></vt:variant></vt:vector></HeadingPairs>"#
        )
    };
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties" xmlns:vt="http://schemas.openxmlformats.org/officeDocument/2006/docPropsVTypes"><Application>ooxp</Application>{pairs}<TitlesOfParts><vt:vector size="{n}" baseType="lpstr">{titles}</vt:vector></TitlesOfParts></Properties>"#
    )
}
