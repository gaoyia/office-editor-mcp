//! 从领域 JSON 生成 Transitional 工作簿。

use std::collections::HashSet;

use serde_json::Value;

use super::chart;
use super::picture::{self, AnchorKind};
use super::request::{parse_cell_value, InsertChartOp, InsertPictureOp, SetCellValue};
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

struct NewSheet {
    name: String,
    cells: Vec<NewCell>,
    pictures: Vec<InsertPictureOp>,
    charts: Vec<InsertChartOp>,
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
        if k != "sheets" {
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
    for (i, _) in sheets.iter().enumerate() {
        types
            .set_override(&format!("/xl/worksheets/sheet{}.xml", i + 1), CT_WORKSHEET)
            .unwrap();
        wb_rels.push(Relationship {
            id: format!("rId{rid}"),
            rel_type: REL_WORKSHEET.into(),
            target: format!("worksheets/sheet{}.xml", i + 1),
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
    if !sst.is_empty() {
        pkg.set("/xl/sharedStrings.xml", sst_xml(&sst).into_bytes())?;
    }
    for (i, sh) in sheets.iter().enumerate() {
        pkg.set(
            &format!("/xl/worksheets/sheet{}.xml", i + 1),
            sheet_xml(sh, &sst)?.into_bytes(),
        )?;
    }
    attach_pictures(&mut pkg, &sheets)?;
    attach_charts(&mut pkg, &sheets)?;
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
    for k in obj.keys() {
        if !matches!(k.as_str(), "name" | "cells" | "pictures" | "charts") {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("workbook.sheets[{i}].{k}"),
            ));
        }
    }
    let name = obj.get("name").and_then(Value::as_str).ok_or_else(|| {
        Error::with_path(
            ErrorKind::InvalidSheetName {
                reason: SheetNameReason::Missing,
            },
            format!("workbook.sheets[{i}].name"),
        )
    })?;
    validate_sheet_name(name, i)?;
    let mut cells = Vec::new();
    if let Some(arr) = obj.get("cells") {
        let arr = arr.as_array().ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: format!("workbook.sheets[{i}].cells"),
                    expected: TypeName::Array,
                },
                format!("workbook.sheets[{i}].cells"),
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
    Ok(NewSheet {
        name: name.to_string(),
        cells,
        pictures,
        charts,
    })
}

fn attach_pictures(pkg: &mut Package, sheets: &[NewSheet]) -> Result<(), Error> {
    for (i, sh) in sheets.iter().enumerate() {
        if sh.pictures.is_empty() {
            continue;
        }
        let part = format!("/xl/worksheets/sheet{}.xml", i + 1);
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

fn attach_charts(pkg: &mut Package, sheets: &[NewSheet]) -> Result<(), Error> {
    for (i, sh) in sheets.iter().enumerate() {
        if sh.charts.is_empty() {
            continue;
        }
        let part = format!("/xl/worksheets/sheet{}.xml", i + 1);
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
    if chart_type.needs_one_series() && series.len() != 1 {
        return Err(Error::with_path(
            ErrorKind::PieNeedsOneSeries,
            format!("{prefix}.series"),
        ));
    }
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
        chart_type,
        title,
        series,
        cx,
        cy,
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

fn validate_sheet_name(name: &str, i: usize) -> Result<(), Error> {
    let path = format!("workbook.sheets[{i}].name");
    if name.is_empty() || name.len() > 31 {
        return Err(Error::with_path(
            ErrorKind::InvalidSheetName {
                reason: SheetNameReason::Length,
            },
            path,
        ));
    }
    if name.eq_ignore_ascii_case("History") {
        return Err(Error::with_path(
            ErrorKind::InvalidSheetName {
                reason: SheetNameReason::History,
            },
            path,
        ));
    }
    if name.contains([':', '*', '?', '/', '[', ']', '\\']) {
        return Err(Error::with_path(
            ErrorKind::InvalidSheetName {
                reason: SheetNameReason::IllegalChars,
            },
            path,
        ));
    }
    Ok(())
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
    for (i, sh) in sheets.iter().enumerate() {
        body.push_str(&format!(
            r#"<sheet name="{}" sheetId="{}" r:id="rId{}"/>"#,
            xml::escape_attr(&sh.name),
            i + 1,
            i + 1
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
    let mut titles = String::new();
    for sh in sheets {
        titles.push_str(&format!(
            "<vt:lpstr>{}</vt:lpstr>",
            xml::escape_text(&sh.name)
        ));
    }
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties" xmlns:vt="http://schemas.openxmlformats.org/officeDocument/2006/docPropsVTypes"><Application>ooxp</Application><HeadingPairs><vt:vector size="2" baseType="variant"><vt:variant><vt:lpstr>Worksheets</vt:lpstr></vt:variant><vt:variant><vt:i4>{n}</vt:i4></vt:variant></vt:vector></HeadingPairs><TitlesOfParts><vt:vector size="{n}" baseType="lpstr">{titles}</vt:vector></TitlesOfParts></Properties>"#
    )
}
