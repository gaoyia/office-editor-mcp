//! 从工作簿读出带 id 的精简树。

use serde_json::{json, Map, Value};

use super::request::{CellScope, InspectScope};
use crate::error::{Error, ErrorKind, RelSource};
use crate::opc::{self, Package};
use crate::xml::{self, Element};

use super::a1;
use super::background;
use super::cf;
use super::chart;
use super::comments;
use super::connection;
use super::custom_view;
use super::extlink;
use super::form_control;
use super::formula;
use super::hf;
use super::ink;
use super::model3d;
use super::names;
use super::ole;
use super::omml;
use super::picture;
use super::pivot;
use super::shape;
use super::slicer;
use super::smartart;
use super::style;
use super::table;
use super::validation;
use super::{
    REL_CHARTSHEET, REL_DIALOGSHEET, REL_INTL_MACROSHEET, REL_MACROSHEET, REL_SHARED_STRINGS,
};

/// 按 workbook 关系 Type 判定表种类。`/xlMacrosheet` 与 `/xlIntlMacrosheet` 都算宏表。
pub(crate) fn sheet_kind(rel_type: &str) -> &'static str {
    if rel_type == REL_CHARTSHEET || rel_type.ends_with("/chartsheet") {
        "chartsheet"
    } else if rel_type == REL_DIALOGSHEET || rel_type.ends_with("/dialogsheet") {
        "dialogsheet"
    } else if rel_type == REL_MACROSHEET
        || rel_type == REL_INTL_MACROSHEET
        || rel_type.ends_with("/macrosheet")
        || rel_type.ends_with("/xlMacrosheet")
        || rel_type.ends_with("/xlIntlMacrosheet")
    {
        "macrosheet"
    } else {
        "worksheet"
    }
}

pub(crate) fn sheet_state_hidden(state: Option<&str>) -> bool {
    matches!(state, Some("hidden") | Some("veryHidden"))
}

pub(crate) fn sheet_state_visible(state: Option<&str>) -> bool {
    !sheet_state_hidden(state)
}

pub(crate) struct SheetMeta {
    pub(crate) name: String,
    pub(crate) kind: &'static str,
    pub(crate) part: String,
    pub(crate) hidden: bool,
}

pub fn inspect(bytes: &[u8], scope: Option<InspectScope>) -> Result<Value, Error> {
    let pkg = Package::open_bytes(bytes.to_vec())?;
    let wb_part = opc::office_document_part(&pkg)?;
    let wb = xml::parse(pkg.get(&wb_part)?)?;
    let rels = opc::Relationships::from_package(&pkg, &wb_part)?;

    let sheets = list_sheets(&wb.root, &rels, &wb_part, &pkg)?;
    let sst = load_shared_strings(&pkg, &wb_part, &rels)?;
    let styles = style::load_for_inspect(&pkg, &wb_part, &rels)?;
    let want_pictures = scope.as_ref().is_some_and(|s| s.pictures);
    let want_charts = scope.as_ref().is_some_and(|s| s.charts);
    let want_header_footer = scope.as_ref().is_some_and(|s| s.header_footer);
    let want_comments = scope.as_ref().is_some_and(|s| s.comments);
    let want_text_boxes = scope.as_ref().is_some_and(|s| s.text_boxes);
    let want_shapes = scope.as_ref().is_some_and(|s| s.shapes);
    let want_connectors = scope.as_ref().is_some_and(|s| s.connectors);
    let want_smart_art = scope.as_ref().is_some_and(|s| s.smart_art);
    let want_hyperlinks = scope.as_ref().is_some_and(|s| s.hyperlinks);
    let want_sparklines = scope.as_ref().is_some_and(|s| s.sparklines);
    let want_ole = scope.as_ref().is_some_and(|s| s.ole_objects);
    let want_form_controls = scope.as_ref().is_some_and(|s| s.form_controls);
    let want_model3ds = scope.as_ref().is_some_and(|s| s.model3ds);
    let want_inks = scope.as_ref().is_some_and(|s| s.inks);
    let want_maths = scope.as_ref().is_some_and(|s| s.maths);
    let want_slicers = scope.as_ref().is_some_and(|s| s.slicers);
    let want_protected_ranges = scope.as_ref().is_some_and(|s| s.protected_ranges);
    let want_connections = scope.as_ref().is_some_and(|s| s.connections);
    let want_external_links = scope.as_ref().is_some_and(|s| s.external_links);
    let want_custom_views = scope.as_ref().is_some_and(|s| s.custom_views);
    let want_cell_styles = scope.as_ref().is_some_and(|s| s.cell_styles);
    let cells_scope = scope.as_ref().and_then(|s| s.cells.as_ref());
    let sheet_parts: Vec<(String, String)> = sheets
        .iter()
        .map(|s| (s.name.clone(), s.part.clone()))
        .collect();

    let mut sheet_json = Vec::new();
    let mut picture_count = 0usize;
    let mut chart_count = 0usize;
    let mut text_box_count = 0usize;
    let mut math_count = 0usize;
    let mut shape_count = 0usize;
    let mut connector_count = 0usize;
    let mut smart_art_count = 0usize;
    let mut comment_count = 0usize;
    let mut hyperlink_count = 0usize;
    let mut sparkline_count = 0usize;
    let mut ole_object_count = 0usize;
    let mut form_control_count = 0usize;
    let mut model3d_count = 0usize;
    let mut ink_count = 0usize;
    let mut slicer_count = 0usize;
    for (sheet_id, sh) in sheets.iter().enumerate() {
        let mut obj = Map::new();
        obj.insert("id".into(), json!(format!("sheet:{}", sh.name)));
        obj.insert("name".into(), json!(sh.name));
        obj.insert("kind".into(), json!(sh.kind));
        obj.insert("hidden".into(), json!(sh.hidden));
        let sheet_doc = xml::parse(pkg.get(&sh.part)?)?;
        if sh.kind == "worksheet" {
            let pics = picture::collect_sheet_pictures(&pkg, &sh.part, &sh.name)?;
            picture_count += pics.len();
            obj.insert("pictureCount".into(), json!(pics.len()));
            obj.insert(
                "background".into(),
                background::inspect_value(&pkg, &sh.part, &sheet_doc.root, &sh.name)?,
            );
            let charts = chart::collect_sheet_charts(&pkg, &sh.part, &sh.name, &sst, &sheet_parts)?;
            chart_count += charts.len();
            obj.insert("chartCount".into(), json!(charts.len()));
            let boxes = shape::collect_textboxes(&pkg, &sh.part, &sh.name)?;
            text_box_count += boxes.len();
            obj.insert("textBoxCount".into(), json!(boxes.len()));
            let maths = omml::collect(&pkg, &sh.part, &sheet_doc.root, &sh.name)?;
            math_count += maths.len();
            obj.insert("mathCount".into(), json!(maths.len()));
            let shapes = shape::collect_shapes(&pkg, &sh.part, &sh.name)?;
            shape_count += shapes.len();
            obj.insert("shapeCount".into(), json!(shapes.len()));
            let connectors = shape::collect_connectors(&pkg, &sh.part, &sh.name)?;
            connector_count += connectors.len();
            obj.insert("connectorCount".into(), json!(connectors.len()));
            let arts = smartart::collect_smartarts(&pkg, &sh.part, &sh.name)?;
            smart_art_count += arts.len();
            obj.insert("smartArtCount".into(), json!(arts.len()));
            let dv_count = validation::count(&sheet_doc.root);
            obj.insert("dataValidationCount".into(), json!(dv_count));
            let cf_count = cf::count(&sheet_doc.root);
            obj.insert("conditionalFormatCount".into(), json!(cf_count));
            let table_count = table::count(&sheet_doc.root);
            obj.insert("tableCount".into(), json!(table_count));
            let pivot_count = pivot::count(&pkg, &sh.part)?;
            obj.insert("pivotCount".into(), json!(pivot_count));
            let n_comments = comments::count_on_sheet(&pkg, &sh.part)?;
            comment_count += n_comments;
            obj.insert("commentCount".into(), json!(n_comments));
            let n_links = super::hyperlink::count(&sheet_doc.root);
            hyperlink_count += n_links;
            obj.insert("hyperlinkCount".into(), json!(n_links));
            let n_sparks = super::sparkline::count(&sheet_doc.root);
            sparkline_count += n_sparks;
            obj.insert("sparklineCount".into(), json!(n_sparks));
            let n_ole = ole::count(&pkg, &sh.part, &sheet_doc.root)?;
            ole_object_count += n_ole;
            obj.insert("oleObjectCount".into(), json!(n_ole));
            let forms = form_control::collect(&pkg, &sh.part, &sheet_doc.root, &sh.name)?;
            form_control_count += forms.len();
            obj.insert("formControlCount".into(), json!(forms.len()));
            let n_m3d = model3d::count(&pkg, &sh.part, &sheet_doc.root)?;
            model3d_count += n_m3d;
            obj.insert("model3dCount".into(), json!(n_m3d));
            let n_ink = ink::count(&pkg, &sh.part, &sheet_doc.root)?;
            ink_count += n_ink;
            obj.insert("inkCount".into(), json!(n_ink));
            let n_slicer = slicer::count(&pkg, &sh.part)?;
            slicer_count += n_slicer;
            obj.insert("slicerCount".into(), json!(n_slicer));
            let n_pr = super::protect::count_ranges(&sheet_doc.root);
            obj.insert("protectedRangeCount".into(), json!(n_pr));
            let has_hf = hf::has_header_footer(&sheet_doc.root);
            obj.insert("hasHeaderFooter".into(), json!(has_hf));
            obj.insert(
                "print".into(),
                super::print::inspect(&sheet_doc.root, &wb.root, sheet_id as u32, &sh.name),
            );
            obj.insert("view".into(), super::view::inspect_view(&sheet_doc.root));
            obj.insert(
                "autoFilter".into(),
                super::view::inspect_filter(&sheet_doc.root),
            );
            obj.insert(
                "sortState".into(),
                super::view::inspect_sort(&sheet_doc.root, &styles),
            );
            obj.insert("outline".into(), super::outline::inspect(&sheet_doc.root));
            obj.insert(
                "sheetProtection".into(),
                super::protect::inspect_sheet(&sheet_doc.root, sh.kind),
            );
            if want_header_footer {
                obj.insert(
                    "headerFooters".into(),
                    Value::Array(hf::inspect_slots(
                        &pkg,
                        &sh.part,
                        &sh.name,
                        &sheet_doc.root,
                        sh.kind,
                        want_pictures,
                        want_text_boxes,
                        want_shapes,
                        want_connectors,
                    )?),
                );
            }
            if want_pictures {
                obj.insert(
                    "pictures".into(),
                    Value::Array(picture::pictures_json(&pics)),
                );
            }
            if want_charts {
                obj.insert("charts".into(), Value::Array(chart::charts_json(&charts)));
            }
            if want_text_boxes {
                obj.insert("textBoxes".into(), Value::Array(boxes));
            }
            if want_maths {
                obj.insert("maths".into(), Value::Array(omml::list_json(&maths)));
            }
            if want_shapes {
                obj.insert("shapes".into(), Value::Array(shapes));
            }
            if want_connectors {
                obj.insert("connectors".into(), Value::Array(connectors));
            }
            if want_smart_art {
                obj.insert("smartArts".into(), Value::Array(arts));
            }
            if want_comments {
                obj.insert(
                    "comments".into(),
                    Value::Array(comments::collect_on_sheet(&pkg, &sh.part, &sh.name)?),
                );
            }
            if want_hyperlinks {
                let sheet_rels = opc::Relationships::from_package(&pkg, &sh.part)?;
                obj.insert(
                    "hyperlinks".into(),
                    super::hyperlink::inspect(&sheet_doc.root, &sh.name, &sheet_rels),
                );
            }
            if want_sparklines {
                obj.insert(
                    "sparklines".into(),
                    super::sparkline::inspect(&sheet_doc.root, &sh.name),
                );
            }
            if want_ole {
                obj.insert(
                    "oleObjects".into(),
                    Value::Array(ole::collect(&pkg, &sh.part, &sheet_doc.root, &sh.name)?),
                );
            }
            if want_form_controls {
                obj.insert("formControls".into(), Value::Array(forms));
            }
            if want_model3ds {
                obj.insert(
                    "model3ds".into(),
                    Value::Array(model3d::list_json(&model3d::collect(
                        &pkg,
                        &sh.part,
                        &sheet_doc.root,
                        &sh.name,
                    )?)),
                );
            }
            if want_inks {
                obj.insert(
                    "inks".into(),
                    Value::Array(ink::collect(&pkg, &sh.part, &sheet_doc.root, &sh.name)?),
                );
            }
            if want_slicers {
                obj.insert(
                    "slicers".into(),
                    Value::Array(slicer::collect(&pkg, &sh.part, &sh.name)?),
                );
            }
            if want_protected_ranges {
                obj.insert(
                    "protectedRanges".into(),
                    super::protect::inspect_ranges(&sheet_doc.root, &sh.name),
                );
            }
            if let Some(cs) = cells_scope {
                if sh.name.eq_ignore_ascii_case(&cs.sheet) {
                    let (cells, truncated, range) =
                        inspect_cells(&sheet_doc.root, &sh.name, cs, &sst, &styles)?;
                    let (start, end) = a1::parse_range(&cs.range)?;
                    let (cols, rows, merges) = super::layout::inspect(&sheet_doc.root, start, end);
                    obj.insert("truncated".into(), json!(truncated));
                    obj.insert("range".into(), json!(range));
                    obj.insert("cells".into(), Value::Array(cells));
                    obj.insert("cols".into(), cols);
                    obj.insert("rows".into(), rows);
                    obj.insert("merges".into(), merges);
                    obj.insert(
                        "dataValidations".into(),
                        validation::inspect(&sheet_doc.root, &sh.name),
                    );
                    obj.insert(
                        "conditionalFormats".into(),
                        cf::inspect(&sheet_doc.root, &sh.name, &styles),
                    );
                    obj.insert(
                        "tables".into(),
                        table::inspect(&pkg, &sh.part, &sheet_doc.root, &sh.name)?,
                    );
                    obj.insert(
                        "pivots".into(),
                        pivot::inspect(&pkg, &wb_part, &sh.part, &sh.name)?,
                    );
                }
            }
        } else if sh.kind == "chartsheet" {
            let pics = picture::collect_sheet_pictures(&pkg, &sh.part, &sh.name)?;
            picture_count += pics.len();
            obj.insert("pictureCount".into(), json!(pics.len()));
            let charts = chart::collect_sheet_charts(&pkg, &sh.part, &sh.name, &sst, &sheet_parts)?;
            chart_count += charts.len();
            obj.insert("chartCount".into(), json!(charts.len()));
            let boxes = shape::collect_textboxes(&pkg, &sh.part, &sh.name)?;
            text_box_count += boxes.len();
            obj.insert("textBoxCount".into(), json!(boxes.len()));
            let maths = omml::collect(&pkg, &sh.part, &sheet_doc.root, &sh.name)?;
            math_count += maths.len();
            obj.insert("mathCount".into(), json!(maths.len()));
            let shapes = shape::collect_shapes(&pkg, &sh.part, &sh.name)?;
            shape_count += shapes.len();
            obj.insert("shapeCount".into(), json!(shapes.len()));
            let connectors = shape::collect_connectors(&pkg, &sh.part, &sh.name)?;
            connector_count += connectors.len();
            obj.insert("connectorCount".into(), json!(connectors.len()));
            obj.insert(
                "hasHeaderFooter".into(),
                json!(hf::has_header_footer(&sheet_doc.root)),
            );
            obj.insert(
                "print".into(),
                super::print::inspect(&sheet_doc.root, &wb.root, sheet_id as u32, &sh.name),
            );
            obj.insert("view".into(), super::view::inspect_view(&sheet_doc.root));
            obj.insert(
                "sheetProtection".into(),
                super::protect::inspect_sheet(&sheet_doc.root, sh.kind),
            );
            if want_header_footer {
                obj.insert(
                    "headerFooters".into(),
                    Value::Array(hf::inspect_slots(
                        &pkg,
                        &sh.part,
                        &sh.name,
                        &sheet_doc.root,
                        sh.kind,
                        want_pictures,
                        want_text_boxes,
                        want_shapes,
                        want_connectors,
                    )?),
                );
            }
            if want_pictures {
                obj.insert(
                    "pictures".into(),
                    Value::Array(picture::pictures_json(&pics)),
                );
            }
            if want_charts {
                obj.insert("charts".into(), Value::Array(chart::charts_json(&charts)));
            }
            if want_text_boxes {
                obj.insert("textBoxes".into(), Value::Array(boxes));
            }
            if want_maths {
                obj.insert("maths".into(), Value::Array(omml::list_json(&maths)));
            }
            if want_shapes {
                obj.insert("shapes".into(), Value::Array(shapes));
            }
            if want_connectors {
                obj.insert("connectors".into(), Value::Array(connectors));
            }
            if let Some(cs) = cells_scope {
                if sh.name.eq_ignore_ascii_case(&cs.sheet) {
                    return Err(Error::with_path(
                        ErrorKind::InspectWorksheetOnly,
                        "scope.sheet",
                    ));
                }
            }
        } else {
            obj.insert(
                "hasHeaderFooter".into(),
                json!(hf::has_header_footer(&sheet_doc.root)),
            );
            if want_header_footer {
                obj.insert(
                    "headerFooters".into(),
                    Value::Array(hf::inspect_slots(
                        &pkg,
                        &sh.part,
                        &sh.name,
                        &sheet_doc.root,
                        sh.kind,
                        false,
                        false,
                        false,
                        false,
                    )?),
                );
            }
            if let Some(cs) = cells_scope {
                if sh.name.eq_ignore_ascii_case(&cs.sheet) {
                    return Err(Error::with_path(
                        ErrorKind::InspectWorksheetOnly,
                        "scope.sheet",
                    ));
                }
            }
        }
        sheet_json.push(Value::Object(obj));
    }

    if let Some(cs) = cells_scope {
        if !sheets
            .iter()
            .any(|s| s.name.eq_ignore_ascii_case(&cs.sheet))
        {
            return Err(Error::with_path(
                ErrorKind::SheetNotFound {
                    name: cs.sheet.clone(),
                },
                "scope.sheet",
            ));
        }
    }

    let mut root = Map::new();
    root.insert("schemaVersion".into(), json!(1));
    root.insert("format".into(), json!("xlsx"));
    root.insert("id".into(), json!("xlsx:workbook"));
    root.insert("pictureCount".into(), json!(picture_count));
    root.insert("chartCount".into(), json!(chart_count));
    root.insert("textBoxCount".into(), json!(text_box_count));
    root.insert("mathCount".into(), json!(math_count));
    root.insert("shapeCount".into(), json!(shape_count));
    root.insert("connectorCount".into(), json!(connector_count));
    root.insert("smartArtCount".into(), json!(smart_art_count));
    root.insert("commentCount".into(), json!(comment_count));
    root.insert("hyperlinkCount".into(), json!(hyperlink_count));
    root.insert("sparklineCount".into(), json!(sparkline_count));
    root.insert("oleObjectCount".into(), json!(ole_object_count));
    root.insert("formControlCount".into(), json!(form_control_count));
    root.insert("model3dCount".into(), json!(model3d_count));
    root.insert("inkCount".into(), json!(ink_count));
    root.insert("slicerCount".into(), json!(slicer_count));
    root.insert(
        "connectionCount".into(),
        json!(connection::count(&pkg, &wb_part)?),
    );
    root.insert("externalLinkCount".into(), json!(extlink::count(&wb.root)));
    root.insert(
        "customViewCount".into(),
        json!(custom_view::count(&wb.root)),
    );
    root.insert("definedNameCount".into(), json!(names::count(&wb.root)));
    root.insert("cellStyleCount".into(), json!(styles.named_count()));
    root.insert("props".into(), crate::props::inspect(&pkg)?);
    root.insert(
        "workbookProtection".into(),
        super::protect::inspect_workbook(&wb.root),
    );
    root.insert("theme".into(), super::theme::inspect(&pkg, &wb_part)?);
    root.insert(
        "themeOverride".into(),
        super::theme::inspect_override(&pkg, &wb_part)?,
    );
    root.insert("sheets".into(), Value::Array(sheet_json));
    let sheet_names: Vec<String> = sheets.iter().map(|s| s.name.clone()).collect();
    root.insert(
        "definedNames".into(),
        names::inspect(&wb.root, &sheet_names),
    );
    if want_connections {
        root.insert("connections".into(), connection::inspect(&pkg, &wb_part)?);
    }
    if want_external_links {
        root.insert(
            "externalLinks".into(),
            extlink::inspect(&pkg, &wb_part, &wb.root)?,
        );
    }
    if want_custom_views {
        root.insert("customViews".into(), custom_view::inspect(&wb.root));
    }
    if want_cell_styles {
        root.insert("cellStyles".into(), styles.named_json());
    }
    Ok(Value::Object(root))
}

pub(crate) fn list_sheets(
    workbook: &Element,
    rels: &opc::Relationships,
    wb_part: &str,
    pkg: &Package,
) -> Result<Vec<SheetMeta>, Error> {
    let sheets_el = workbook.child_by_local("sheets").ok_or_else(|| {
        Error::new(ErrorKind::XmlMissing {
            parent: "workbook.xml".into(),
            child: "sheets".into(),
        })
    })?;
    let mut out = Vec::new();
    for sh in sheets_el.children_by_local("sheet") {
        let name = sh.attr("name").unwrap_or("").to_string();
        // `r:id`：前缀只是习惯，任何映射到 relationships 命名空间的前缀都合法。
        let rid = sh
            .attr_local("id")
            .ok_or_else(|| {
                Error::new(ErrorKind::XmlMissing {
                    parent: "sheet".into(),
                    child: "r:id".into(),
                })
            })?
            .to_string();
        let rel = rels.find_by_id(&rid).ok_or_else(|| {
            Error::new(ErrorKind::RelNotFound {
                source: RelSource::Workbook,
                rid: rid.clone(),
            })
        })?;
        let kind = sheet_kind(&rel.rel_type);
        let part = opc::resolve_rel_target(wb_part, &rel.target)?;
        if !pkg.contains(&part) {
            return Err(Error::new(ErrorKind::PartMissing { name: part }));
        }
        let hidden = sheet_state_hidden(sh.attr("state"));
        out.push(SheetMeta {
            name,
            kind,
            part,
            hidden,
        });
    }
    Ok(out)
}

pub(crate) fn load_shared_strings(
    pkg: &Package,
    wb_part: &str,
    rels: &opc::Relationships,
) -> Result<Vec<String>, Error> {
    let Some(rel) = rels
        .find_by_type_suffix("/sharedStrings")
        .into_iter()
        .next()
        .or_else(|| rels.rels.iter().find(|r| r.rel_type == REL_SHARED_STRINGS))
    else {
        return Ok(Vec::new());
    };
    if rel.is_external() {
        return Ok(Vec::new());
    }
    let part = opc::resolve_rel_target(wb_part, &rel.target)?;
    let Ok(bytes) = pkg.get(&part) else {
        return Ok(Vec::new());
    };
    let doc = xml::parse(bytes)?;
    let mut out = Vec::new();
    for si in doc.root.children_by_local("si") {
        out.push(si.text_content());
    }
    Ok(out)
}

fn inspect_cells(
    root: &Element,
    sheet_name: &str,
    scope: &CellScope,
    sst: &[String],
    styles: &style::Stylebook,
) -> Result<(Vec<Value>, bool, String), Error> {
    let (start, end) = a1::parse_range(&scope.range)?;
    let range_norm = format!(
        "{}:{}",
        a1::format_cell(start.col, start.row),
        a1::format_cell(end.col, end.row)
    );
    let Some(sheet_data) = root.child_by_local("sheetData") else {
        return Ok((Vec::new(), false, range_norm));
    };
    let index = formula::index_sheet(sheet_data);

    let mut cells = Vec::new();
    let mut truncated = false;
    let mut failed: Option<Error> = None;
    formula::for_each_cell(sheet_data, |pref, c| {
        if failed.is_some() || truncated {
            return;
        }
        if !a1::in_range(pref, start, end) {
            return;
        }
        if cells.len() as u32 >= scope.max_cells {
            truncated = true;
            return;
        }
        match cell_to_json(c, sheet_name, sst, styles, &index, pref) {
            Ok(cell) => cells.push(cell),
            Err(e) => failed = Some(e),
        }
    });
    if let Some(e) = failed {
        return Err(e);
    }
    Ok((cells, truncated, range_norm))
}

fn cell_to_json(
    c: &Element,
    sheet_name: &str,
    sst: &[String],
    styles: &style::Stylebook,
    index: &formula::FormulaIndex,
    pref: a1::CellRef,
) -> Result<Value, Error> {
    let refer = a1::format_cell(pref.col, pref.row);
    let t = c.attr("t").unwrap_or("");
    let f_el = c.child_by_local("f");
    let v_el = c.child_by_local("v");
    let is_el = c.child_by_local("is");

    let mut formula = json!(null);
    let mut shared = false;
    let mut shared_ref: Option<String> = None;
    let mut array = false;
    let mut array_ref: Option<String> = None;
    let mut data_table = false;
    let mut data_table_ref: Option<String> = None;
    if let Some(f) = f_el {
        let ft = f.attr("t").unwrap_or("");
        let text = f.text_content();
        match ft {
            "shared" => {
                shared = true;
                if let Some(g) = f.attr("si").and_then(|si| index.shared.get(si)) {
                    if g.master == pref {
                        shared_ref = Some(a1::format_range(g.start, g.end));
                        if !g.formula.trim().is_empty() {
                            formula = json!(formula::shown_formula(&g.formula));
                        } else if !text.trim().is_empty() {
                            formula = json!(formula::shown_formula(&text));
                        }
                    } else if !g.formula.trim().is_empty() {
                        let shifted = formula::shift_a1_formula(&g.formula, g.master, pref);
                        formula = json!(formula::shown_formula(&shifted));
                    }
                } else if !text.trim().is_empty() {
                    formula = json!(formula::shown_formula(&text));
                }
            }
            "array" => {
                array = true;
                if let Some(g) = index.arrays.iter().find(|g| g.master == pref) {
                    array_ref = Some(a1::format_range(g.start, g.end));
                    if !g.formula.trim().is_empty() {
                        formula = json!(formula::shown_formula(&g.formula));
                    }
                } else if !text.trim().is_empty() {
                    formula = json!(formula::shown_formula(&text));
                    array_ref = f
                        .attr("ref")
                        .and_then(|r| a1::parse_range(r).ok())
                        .map(|(a, b)| a1::format_range(a, b));
                }
            }
            "dataTable" => {
                data_table = true;
                if let Some(g) = index.tables.iter().find(|g| g.master == pref) {
                    data_table_ref = Some(a1::format_range(g.start, g.end));
                } else {
                    data_table_ref = f
                        .attr("ref")
                        .and_then(|r| a1::parse_range(r).ok())
                        .map(|(a, b)| a1::format_range(a, b));
                }
                if !text.trim().is_empty() {
                    formula = json!(formula::shown_formula(&text));
                } else if let Some(s) = formula::shown_table_from_elem(f) {
                    formula = json!(s);
                }
            }
            _ if !text.trim().is_empty() => {
                formula = json!(formula::shown_formula(&text));
            }
            _ => {}
        }
    } else {
        match formula::occupy_at(index, pref) {
            Some(formula::Occupied::Array { master }) if master != pref => {
                array = true;
            }
            Some(formula::Occupied::DataTable { master }) if master != pref => {
                data_table = true;
            }
            _ => {}
        }
    }

    let (ty, value) = classify(
        t,
        v_el,
        is_el,
        f_el.is_some(),
        sst,
        &format!("{sheet_name}!{refer}"),
    )?;

    let id = format!("cell:{sheet_name}!{refer}");
    let mut obj = json!({
        "id": id,
        "ref": refer,
        "type": ty,
        "value": value,
        "formula": formula,
    });
    style::attach_style(&mut obj, c, styles);
    if array {
        obj["array"] = json!(true);
    }
    if let Some(r) = array_ref {
        obj["arrayRef"] = json!(r);
    }
    if shared {
        obj["shared"] = json!(true);
    }
    if let Some(r) = shared_ref {
        obj["sharedRef"] = json!(r);
    }
    if data_table {
        obj["dataTable"] = json!(true);
    }
    if let Some(r) = data_table_ref {
        obj["dataTableRef"] = json!(r);
    }
    Ok(obj)
}

pub(crate) fn classify(
    t: &str,
    v_el: Option<&Element>,
    is_el: Option<&Element>,
    has_f: bool,
    sst: &[String],
    cell: &str,
) -> Result<(&'static str, Value), Error> {
    match t {
        "b" => {
            let raw = v_el.map(|e| e.text_content()).unwrap_or_default();
            let b = raw == "1" || raw.eq_ignore_ascii_case("true");
            Ok(("boolean", json!(b)))
        }
        "s" => {
            let raw = v_el.map(|e| e.text_content()).unwrap_or_default();
            let idx: usize = raw.trim().parse().map_err(|_| {
                Error::new(ErrorKind::SharedStringIndex {
                    cell: cell.to_string(),
                    index: raw.trim().to_string(),
                })
            })?;
            let text = sst.get(idx).cloned().ok_or_else(|| {
                Error::new(ErrorKind::SharedStringIndex {
                    cell: cell.to_string(),
                    index: idx.to_string(),
                })
            })?;
            Ok(("text", json!(text)))
        }
        "inlineStr" => {
            let text = is_el.map(|e| e.text_content()).unwrap_or_default();
            Ok(("text", json!(text)))
        }
        "str" => {
            let text = v_el.map(|e| e.text_content()).unwrap_or_default();
            Ok(("text", json!(text)))
        }
        "e" => {
            let text = v_el.map(|e| e.text_content()).unwrap_or_default();
            Ok(("error", json!(text)))
        }
        "d" => {
            let text = v_el.map(|e| e.text_content()).unwrap_or_default();
            Ok(("date", json!(text)))
        }
        "n" | "" => {
            // 省略 t 却给了 <is> 的写法少见但合法，按文本读，别当成没有值的数字。
            if let (None, Some(is)) = (v_el, is_el) {
                return Ok(("text", json!(is.text_content())));
            }
            if v_el.is_none() && is_el.is_none() && !has_f {
                return Ok(("empty", json!(null)));
            }
            if v_el.is_none() {
                return Ok(("number", json!(null)));
            }
            let raw = v_el.unwrap().text_content();
            if let Ok(n) = raw.trim().parse::<i64>() {
                return Ok(("number", json!(n)));
            }
            if let Ok(n) = raw.trim().parse::<f64>() {
                return Ok(("number", json!(n)));
            }
            Ok(("text", json!(raw)))
        }
        _ => {
            let raw = v_el.or(is_el).map(|e| e.text_content()).unwrap_or_default();
            Ok(("text", json!(raw)))
        }
    }
}
