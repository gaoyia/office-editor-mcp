//! 从领域 JSON 生成 Transitional 工作簿。

use serde_json::Value;

use super::chart;
use super::chartsheet;
use super::comments;
use super::hf;
use super::hyperlink;
use super::picture::{self};
use super::request::create_types::*;
use super::request::{SetCellValue, SetHeaderFooterTextOp};
use super::shape;
use super::smartart;
use crate::error::{Error, ErrorKind, TypeName};
use crate::opc::{
    ContentTypes, Package, Relationship, Relationships, CONTENT_TYPES_PART, ROOT_RELS_PART,
};
use crate::xml;

use super::a1::{self};
use super::{
    CT_APP, CT_CORE, CT_SHARED_STRINGS, CT_STYLES, CT_WORKBOOK, CT_WORKSHEET, REL_APP, REL_CORE,
    REL_OFFICE_DOCUMENT, REL_SHARED_STRINGS, REL_STYLES, REL_WORKSHEET,
};

pub fn create(workbook: &Value) -> Result<Vec<u8>, Error> {
    create_with_workbook_ct(workbook, CT_WORKBOOK)
}

pub fn create_with_workbook_ct(workbook: &Value, workbook_ct: &str) -> Result<Vec<u8>, Error> {
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
        if !super::request::create_remain::is_workbook_key(k) {
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
        sheets.push(super::request::create_sheet::parse_sheet(s, i)?);
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
            ("/xl/workbook.xml".into(), workbook_ct.into()),
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
        super::create_xml::workbook_xml(&sheets, has_formula).into_bytes(),
    )?;
    pkg.set("/xl/_rels/workbook.xml.rels", wb_rels.to_xml().into_bytes())?;
    pkg.set(
        "/xl/styles.xml",
        super::create_xml::styles_xml().into_bytes(),
    )?;
    pkg.set(
        "/docProps/core.xml",
        super::create_xml::core_xml().into_bytes(),
    )?;
    pkg.set(
        "/docProps/app.xml",
        super::create_xml::app_xml(&sheets).into_bytes(),
    )?;
    crate::props::apply_create(&mut pkg, obj.get("props"), "workbook.props")?;
    if !sst.is_empty() {
        pkg.set(
            "/xl/sharedStrings.xml",
            super::create_xml::sst_xml(&sst).into_bytes(),
        )?;
    }
    let mut ws_n = 0u32;
    for sh in &sheets {
        if sh.chartsheet.is_some() {
            continue;
        }
        ws_n += 1;
        pkg.set(
            &format!("/xl/worksheets/sheet{ws_n}.xml"),
            super::create_xml::sheet_xml(sh, &sst)?.into_bytes(),
        )?;
    }
    attach_pictures(&mut pkg, &sheets)?;
    attach_model3ds(&mut pkg, &sheets)?;
    attach_inks(&mut pkg, &sheets)?;
    attach_charts(&mut pkg, &sheets)?;
    attach_drawings(&mut pkg, &sheets)?;
    attach_comments(&mut pkg, &sheets)?;
    attach_hyperlinks(&mut pkg, &sheets)?;
    attach_header_footers(&mut pkg, &sheets)?;
    attach_chartsheets(&mut pkg, &sheets)?;
    if let Some(v) = obj.get("theme") {
        super::theme::write_create(&mut pkg, &super::theme::parse_create(v)?)?;
    }
    let bytes = pkg.save_to_vec()?;
    let bytes = super::request::create_remain::apply_remain(bytes, obj)?;
    super::eval::fill_saved(bytes)
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

fn attach_inks(pkg: &mut Package, sheets: &[NewSheet]) -> Result<(), Error> {
    for (i, part, sh) in each_worksheet(sheets) {
        if sh.inks.is_empty() {
            continue;
        }
        let mut doc = xml::parse(pkg.get(&part)?)?;
        for (j, op) in sh.inks.iter().enumerate() {
            super::ink::insert(
                pkg,
                &mut doc,
                &part,
                op,
                &format!("workbook.sheets[{i}].inks[{j}].file"),
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
