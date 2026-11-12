//! xlsx 自己的 scope 与 ops 解析。

use serde_json::Value;

use crate::error::{Error, ErrorKind, ScopeNeed, TypeName};
use crate::json;

mod types;
pub use types::*;
mod common;
use common::*;
mod cell;
pub use cell::parse_cell_value;
use cell::*;
mod layout;
use layout::*;
mod table_data;
use table_data::*;
pub(crate) use table_data::{check_cf_fields, check_dv_fields, icon_set_count};
mod sheet;
pub(crate) use sheet::validate_sheet_name;
use sheet::*;
mod picture;
use picture::*;
mod chart;
use chart::*;
mod drawing;
use drawing::*;
mod comments;
use comments::*;
mod hyperlink;
use hyperlink::*;
mod extras;
use extras::*;

pub(crate) mod create_chart;
pub(crate) mod create_comment;
pub(crate) mod create_drawing;
pub(crate) mod create_hf;
pub(crate) mod create_link;
pub(crate) mod create_media;
pub(crate) mod create_picture;
pub(crate) mod create_remain;
pub(crate) mod create_sheet;
pub(crate) mod create_types;
pub(crate) mod create_util;

pub fn parse_scope(v: Option<&Value>) -> Result<Option<InspectScope>, Error> {
    let Some(v) = v else {
        return Ok(None);
    };
    let obj = v.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "scope".into(),
                expected: TypeName::Object,
            },
            "scope",
        )
    })?;
    if obj.is_empty() {
        return Ok(None);
    }
    json::deny_unknown(
        obj,
        &[
            "sheet",
            "range",
            "maxCells",
            "pictures",
            "charts",
            "headerFooter",
            "comments",
            "textBoxes",
            "shapes",
            "connectors",
            "smartArt",
            "hyperlinks",
            "sparklines",
            "oleObjects",
            "model3ds",
            "inks",
            "maths",
            "slicers",
            "protectedRanges",
            "connections",
            "externalLinks",
            "customViews",
            "cellStyles",
        ],
    )?;
    let pictures = parse_bool_field(obj.get("pictures"), "pictures")?;
    let charts = parse_bool_field(obj.get("charts"), "charts")?;
    let header_footer = parse_bool_field(obj.get("headerFooter"), "headerFooter")?;
    let comments = parse_bool_field(obj.get("comments"), "comments")?;
    let text_boxes = parse_bool_field(obj.get("textBoxes"), "textBoxes")?;
    let shapes = parse_bool_field(obj.get("shapes"), "shapes")?;
    let connectors = parse_bool_field(obj.get("connectors"), "connectors")?;
    let smart_art = parse_bool_field(obj.get("smartArt"), "smartArt")?;
    let hyperlinks = parse_bool_field(obj.get("hyperlinks"), "hyperlinks")?;
    let sparklines = parse_bool_field(obj.get("sparklines"), "sparklines")?;
    let ole_objects = parse_bool_field(obj.get("oleObjects"), "oleObjects")?;
    let model3ds = parse_bool_field(obj.get("model3ds"), "model3ds")?;
    let inks = parse_bool_field(obj.get("inks"), "inks")?;
    let maths = parse_bool_field(obj.get("maths"), "maths")?;
    let slicers = parse_bool_field(obj.get("slicers"), "slicers")?;
    let protected_ranges = parse_bool_field(obj.get("protectedRanges"), "protectedRanges")?;
    let connections = parse_bool_field(obj.get("connections"), "connections")?;
    let external_links = parse_bool_field(obj.get("externalLinks"), "externalLinks")?;
    let custom_views = parse_bool_field(obj.get("customViews"), "customViews")?;
    let cell_styles = parse_bool_field(obj.get("cellStyles"), "cellStyles")?;
    let sheet = obj.get("sheet").and_then(Value::as_str);
    let range = obj.get("range").and_then(Value::as_str);
    let max_cells = match obj.get("maxCells") {
        None => None,
        Some(Value::Number(n)) => n.as_u64(),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "maxCells".into(),
                    expected: TypeName::Integer,
                },
                "scope.maxCells",
            ));
        }
    };
    let cells = match (sheet, range, max_cells) {
        (None, None, None) => None,
        (Some(s), Some(r), m) => {
            let max = m.unwrap_or(5000);
            if !(1..=20000).contains(&max) {
                return Err(Error::with_path(
                    ErrorKind::OutOfRange {
                        field: "maxCells".into(),
                        min: 1,
                        max: 20000,
                    },
                    "scope.maxCells",
                ));
            }
            Some(CellScope {
                sheet: s.to_string(),
                range: r.to_string(),
                max_cells: max as u32,
            })
        }
        _ => {
            return Err(Error::with_path(
                ErrorKind::ScopeRequired {
                    need: ScopeNeed::SheetAndRange,
                },
                "scope",
            ));
        }
    };
    if cells.is_none()
        && !pictures
        && !charts
        && !header_footer
        && !comments
        && !text_boxes
        && !shapes
        && !connectors
        && !smart_art
        && !hyperlinks
        && !sparklines
        && !ole_objects
        && !model3ds
        && !inks
        && !maths
        && !slicers
        && !protected_ranges
        && !connections
        && !external_links
        && !custom_views
        && !cell_styles
    {
        return Ok(None);
    }
    Ok(Some(InspectScope {
        cells,
        pictures,
        charts,
        header_footer,
        comments,
        text_boxes,
        shapes,
        connectors,
        smart_art,
        hyperlinks,
        sparklines,
        ole_objects,
        model3ds,
        inks,
        maths,
        slicers,
        protected_ranges,
        connections,
        external_links,
        custom_views,
        cell_styles,
    }))
}

pub fn parse_ops(ops: &[Value]) -> Result<Vec<XlsxOp>, Error> {
    let mut out = Vec::new();
    for (i, op) in ops.iter().enumerate() {
        let obj = op.as_object().ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: format!("ops[{i}]"),
                    expected: TypeName::Object,
                },
                format!("ops[{i}]"),
            )
        })?;
        let op_name = obj.get("op").and_then(Value::as_str).ok_or_else(|| {
            Error::with_path(
                ErrorKind::MissingField { field: "op".into() },
                format!("ops[{i}].op"),
            )
        })?;
        let parsed = match op_name {
            "setCell" => XlsxOp::SetCell(parse_set_cell(obj, i)?),
            "setCellStyle" => XlsxOp::SetCellStyle(Box::new(parse_set_cell_style(obj, i)?)),
            "setHeaderFooterText" => XlsxOp::SetHeaderFooterText(parse_set_hf_text(obj, i)?),
            "replacePicture" => XlsxOp::ReplacePicture(parse_replace_pic(obj, i)?),
            "insertPicture" => XlsxOp::InsertPicture(parse_insert_pic(obj, i)?),
            "deletePicture" => XlsxOp::DeletePicture(parse_delete_pic(obj, i)?),
            "setPictureSize" => XlsxOp::SetPictureSize(parse_set_pic_size(obj, i)?),
            "setPictureAnchor" => XlsxOp::SetPictureAnchor(parse_set_pic_anchor(obj, i)?),
            "insertChart" => XlsxOp::InsertChart(parse_insert_chart(obj, i)?),
            "insertChartSheet" => XlsxOp::InsertChartSheet(parse_insert_chart_sheet(obj, i)?),
            "deleteChartSheet" => XlsxOp::DeleteChartSheet(parse_delete_chart_sheet(obj, i)?),
            "insertSheet" => XlsxOp::InsertSheet(parse_insert_sheet(obj, i)?),
            "deleteSheet" => XlsxOp::DeleteSheet(parse_delete_sheet(obj, i)?),
            "renameSheet" => XlsxOp::RenameSheet(parse_rename_sheet(obj, i)?),
            "moveSheet" => XlsxOp::MoveSheet(parse_move_sheet(obj, i)?),
            "setSheetHidden" => XlsxOp::SetSheetHidden(parse_set_sheet_hidden(obj, i)?),
            "setSheetBackground" => XlsxOp::SetSheetBackground(parse_set_sheet_background(obj, i)?),
            "deleteSheetBackground" => {
                XlsxOp::DeleteSheetBackground(parse_delete_sheet_background(obj, i)?)
            }
            "setColWidth" => XlsxOp::SetColWidth(parse_set_col_width(obj, i)?),
            "setRowHeight" => XlsxOp::SetRowHeight(parse_set_row_height(obj, i)?),
            "mergeRange" => XlsxOp::MergeRange(parse_merge_range(obj, i)?),
            "unmergeRange" => XlsxOp::UnmergeRange(parse_unmerge_range(obj, i)?),
            "insertDataValidation" => {
                XlsxOp::InsertDataValidation(parse_insert_data_validation(obj, i)?)
            }
            "setDataValidation" => XlsxOp::SetDataValidation(parse_set_data_validation(obj, i)?),
            "deleteDataValidation" => {
                XlsxOp::DeleteDataValidation(parse_delete_data_validation(obj, i)?)
            }
            "insertConditionalFormat" => {
                XlsxOp::InsertConditionalFormat(parse_insert_conditional_format(obj, i)?)
            }
            "setConditionalFormat" => {
                XlsxOp::SetConditionalFormat(parse_set_conditional_format(obj, i)?)
            }
            "deleteConditionalFormat" => {
                XlsxOp::DeleteConditionalFormat(parse_delete_conditional_format(obj, i)?)
            }
            "insertSheetTable" => XlsxOp::InsertSheetTable(parse_insert_sheet_table(obj, i)?),
            "setSheetTable" => XlsxOp::SetSheetTable(parse_set_sheet_table(obj, i)?),
            "deleteSheetTable" => XlsxOp::DeleteSheetTable(parse_delete_sheet_table(obj, i)?),
            "insertPivotTable" => XlsxOp::InsertPivotTable(parse_insert_pivot_table(obj, i)?),
            "setPivotTable" => XlsxOp::SetPivotTable(parse_set_pivot_table(obj, i)?),
            "deletePivotTable" => XlsxOp::DeletePivotTable(parse_delete_pivot_table(obj, i)?),
            "insertDefinedName" => XlsxOp::InsertDefinedName(parse_insert_defined_name(obj, i)?),
            "setDefinedName" => XlsxOp::SetDefinedName(parse_set_defined_name(obj, i)?),
            "deleteDefinedName" => XlsxOp::DeleteDefinedName(parse_delete_defined_name(obj, i)?),
            "setPrintSetup" => XlsxOp::SetPrintSetup(parse_set_print_setup(obj, i)?),
            "setSheetView" => XlsxOp::SetSheetView(parse_set_sheet_view(obj, i)?),
            "setAutoFilter" => XlsxOp::SetAutoFilter(parse_set_auto_filter(obj, i)?),
            "setSortState" => XlsxOp::SetSortState(parse_set_sort_state(obj, i)?),
            "insertCustomView" => XlsxOp::InsertCustomView(parse_insert_custom_view(obj, i)?),
            "deleteCustomView" => XlsxOp::DeleteCustomView(parse_delete_custom_view(obj, i)?),
            "showCustomView" => XlsxOp::ShowCustomView(parse_show_custom_view(obj, i)?),
            "insertCellStyle" => XlsxOp::InsertCellStyle(parse_insert_cell_style(obj, i)?),
            "deleteCellStyle" => XlsxOp::DeleteCellStyle(parse_delete_cell_style(obj, i)?),
            "setCellNamedStyle" => XlsxOp::SetCellNamedStyle(parse_set_cell_named_style(obj, i)?),
            "groupRows" => XlsxOp::GroupRows(parse_group_rows(obj, i)?),
            "ungroupRows" => XlsxOp::UngroupRows(parse_ungroup_rows(obj, i)?),
            "groupCols" => XlsxOp::GroupCols(parse_group_cols(obj, i)?),
            "ungroupCols" => XlsxOp::UngroupCols(parse_ungroup_cols(obj, i)?),
            "setOutline" => XlsxOp::SetOutline(parse_set_outline(obj, i)?),
            "setRowHidden" => XlsxOp::SetRowHidden(parse_set_row_hidden(obj, i)?),
            "setColHidden" => XlsxOp::SetColHidden(parse_set_col_hidden(obj, i)?),
            "insertComment" => XlsxOp::InsertComment(parse_insert_comment(obj, i)?),
            "replyComment" => XlsxOp::ReplyComment(parse_reply_comment(obj, i)?),
            "deleteComment" => XlsxOp::DeleteComment(parse_delete_comment(obj, i)?),
            "setCommentText" => XlsxOp::SetCommentText(parse_set_comment_text(obj, i)?),
            "setCommentMeta" => XlsxOp::SetCommentMeta(parse_set_comment_meta(obj, i)?),
            "setCommentDone" => XlsxOp::SetCommentDone(parse_set_comment_done(obj, i)?),
            "insertHyperlink" => XlsxOp::InsertHyperlink(parse_insert_hyperlink(obj, i)?),
            "setHyperlink" => XlsxOp::SetHyperlink(parse_set_hyperlink(obj, i)?),
            "deleteHyperlink" => XlsxOp::DeleteHyperlink(parse_delete_hyperlink(obj, i)?),
            "insertSparkline" => XlsxOp::InsertSparkline(parse_insert_sparkline(obj, i)?),
            "setSparkline" => XlsxOp::SetSparkline(parse_set_sparkline(obj, i)?),
            "deleteSparkline" => XlsxOp::DeleteSparkline(parse_delete_sparkline(obj, i)?),
            "setProtection" => XlsxOp::SetProtection(parse_set_protection(obj, i)?),
            "insertProtectedRange" => {
                XlsxOp::InsertProtectedRange(parse_insert_protected_range(obj, i)?)
            }
            "setProtectedRange" => XlsxOp::SetProtectedRange(parse_set_protected_range(obj, i)?),
            "deleteProtectedRange" => {
                XlsxOp::DeleteProtectedRange(parse_delete_protected_range(obj, i)?)
            }
            "deleteChart" => XlsxOp::DeleteChart(parse_delete_chart(obj, i)?),
            "setChartTitle" => XlsxOp::SetChartTitle(parse_set_chart_title(obj, i)?),
            "setChartSeries" => XlsxOp::SetChartSeries(parse_set_chart_series(obj, i)?),
            "addChartSeries" => XlsxOp::AddChartSeries(parse_add_chart_series(obj, i)?),
            "deleteChartSeries" => XlsxOp::DeleteChartSeries(parse_delete_chart_series(obj, i)?),
            "setChartType" => XlsxOp::SetChartType(parse_set_chart_type(obj, i)?),
            "setChartAxisTitle" => XlsxOp::SetChartAxisTitle(parse_set_chart_axis_title(obj, i)?),
            "setChartLegend" => XlsxOp::SetChartLegend(parse_set_chart_legend(obj, i)?),
            "setChartDataLabels" => {
                XlsxOp::SetChartDataLabels(parse_set_chart_data_labels(obj, i)?)
            }
            "setChartSize" => XlsxOp::SetChartSize(parse_set_chart_size(obj, i)?),
            "setChartAnchor" => XlsxOp::SetChartAnchor(parse_set_chart_anchor(obj, i)?),
            "insertTrendline" => XlsxOp::InsertTrendline(parse_insert_trendline(obj, i)?),
            "setTrendline" => XlsxOp::SetTrendline(parse_set_trendline(obj, i)?),
            "deleteTrendline" => XlsxOp::DeleteTrendline(parse_delete_trendline(obj, i)?),
            "insertErrorBars" => XlsxOp::InsertErrorBars(parse_insert_error_bars(obj, i)?),
            "setErrorBars" => XlsxOp::SetErrorBars(parse_set_error_bars(obj, i)?),
            "deleteErrorBars" => XlsxOp::DeleteErrorBars(parse_delete_error_bars(obj, i)?),
            "setChartSeriesAxis" => XlsxOp::SetChartSeriesAxis(parse_set_series_axis(obj, i)?),
            "insertTextBox" => XlsxOp::InsertTextBox(parse_insert_textbox(obj, i)?),
            "insertShape" => XlsxOp::InsertShape(parse_insert_shape(obj, i)?),
            "insertConnector" => XlsxOp::InsertConnector(parse_insert_connector(obj, i)?),
            "deleteTextBox" => XlsxOp::DeleteTextBox(parse_delete_textbox(obj, i)?),
            "deleteShape" => XlsxOp::DeleteShape(parse_delete_shape(obj, i)?),
            "deleteConnector" => XlsxOp::DeleteConnector(parse_delete_connector(obj, i)?),
            "groupShapes" => XlsxOp::GroupShapes(parse_group_shapes(obj, i)?),
            "ungroup" => XlsxOp::Ungroup(parse_ungroup(obj, i)?),
            "insertOle" => XlsxOp::InsertOle(parse_insert_ole(obj, i)?),
            "deleteOle" => XlsxOp::DeleteOle(parse_delete_ole(obj, i)?),
            "setOleSize" => XlsxOp::SetOleSize(parse_set_ole_size(obj, i)?),
            "insertModel3d" => XlsxOp::InsertModel3d(parse_insert_model3d(obj, i)?),
            "replaceModel3d" => XlsxOp::ReplaceModel3d(parse_replace_model3d(obj, i)?),
            "deleteModel3d" => XlsxOp::DeleteModel3d(parse_delete_model3d(obj, i)?),
            "setModel3dSize" => XlsxOp::SetModel3dSize(parse_set_model3d_size(obj, i)?),
            "setModel3dAnchor" => XlsxOp::SetModel3dAnchor(parse_set_model3d_anchor(obj, i)?),
            "setModel3dCamera" => XlsxOp::SetModel3dCamera(parse_set_model3d_camera(obj, i)?),
            "insertInk" => XlsxOp::InsertInk(parse_insert_ink(obj, i)?),
            "replaceInk" => XlsxOp::ReplaceInk(parse_replace_ink(obj, i)?),
            "deleteInk" => XlsxOp::DeleteInk(parse_delete_ink(obj, i)?),
            "setInkSize" => XlsxOp::SetInkSize(parse_set_ink_size(obj, i)?),
            "setInkAnchor" => XlsxOp::SetInkAnchor(parse_set_ink_anchor(obj, i)?),
            "insertMath" => XlsxOp::InsertMath(parse_insert_math(obj, i)?),
            "setMath" => XlsxOp::SetMath(parse_set_math(obj, i)?),
            "deleteMath" => XlsxOp::DeleteMath(parse_delete_math(obj, i)?),
            "setMathSize" => XlsxOp::SetMathSize(parse_set_math_size(obj, i)?),
            "setMathAnchor" => XlsxOp::SetMathAnchor(parse_set_math_anchor(obj, i)?),
            "insertSlicer" => XlsxOp::InsertSlicer(parse_insert_slicer(obj, i)?),
            "deleteSlicer" => XlsxOp::DeleteSlicer(parse_delete_slicer(obj, i)?),
            "setSlicerSize" => XlsxOp::SetSlicerSize(parse_set_slicer_size(obj, i)?),
            "setSlicerAnchor" => XlsxOp::SetSlicerAnchor(parse_set_slicer_anchor(obj, i)?),
            "setSlicerView" => XlsxOp::SetSlicerView(parse_set_slicer_view(obj, i)?),
            "setSlicerFilter" => XlsxOp::SetSlicerFilter(parse_set_slicer_filter(obj, i)?),
            "setThemeColors" => XlsxOp::SetThemeColors(super::theme::parse_set_colors(obj, i)?),
            "setThemeFonts" => XlsxOp::SetThemeFonts(super::theme::parse_set_fonts(obj, i)?),
            "setThemeFmt" => XlsxOp::SetThemeFmt(super::theme::parse_set_fmt(obj, i)?),
            "setThemeExtraSchemes" => {
                XlsxOp::SetThemeExtraSchemes(super::theme::parse_set_extra_schemes(obj, i)?)
            }
            "setThemeObjectDefaults" => {
                XlsxOp::SetThemeObjectDefaults(super::theme::parse_set_object_defaults(obj, i)?)
            }
            "setThemeOverride" => {
                XlsxOp::SetThemeOverride(super::theme::parse_set_override(obj, i)?)
            }
            "insertConnection" => XlsxOp::InsertConnection(parse_insert_connection(obj, i)?),
            "deleteConnection" => XlsxOp::DeleteConnection(parse_delete_connection(obj, i)?),
            "insertExternalLink" => XlsxOp::InsertExternalLink(parse_insert_external_link(obj, i)?),
            "deleteExternalLink" => XlsxOp::DeleteExternalLink(parse_delete_external_link(obj, i)?),
            "setExternalLink" => XlsxOp::SetExternalLink(parse_set_external_link(obj, i)?),
            "setTextBoxSize" => XlsxOp::SetTextBoxSize(parse_set_textbox_size(obj, i)?),
            "setTextBoxAnchor" => XlsxOp::SetTextBoxAnchor(parse_set_textbox_anchor(obj, i)?),
            "setShapeText" => XlsxOp::SetShapeText(parse_set_shape_text(obj, i)?),
            "setShapeRuns" => XlsxOp::SetShapeRuns(parse_set_shape_runs(obj, i)?),
            "setShapeFormat" => XlsxOp::SetShapeFormat(parse_set_shape_format(obj, i)?),
            "insertSmartArt" => XlsxOp::InsertSmartArt(parse_insert_smartart(obj, i)?),
            "deleteSmartArt" => XlsxOp::DeleteSmartArt(parse_delete_smartart(obj, i)?),
            "setSmartArtText" => XlsxOp::SetSmartArtText(parse_set_smartart_text(obj, i)?),
            "addSmartArtPoint" => XlsxOp::AddSmartArtPoint(parse_add_smartart_point(obj, i)?),
            "deleteSmartArtPoint" => {
                XlsxOp::DeleteSmartArtPoint(parse_delete_smartart_point(obj, i)?)
            }
            "setSmartArtSize" => XlsxOp::SetSmartArtSize(parse_set_smartart_size(obj, i)?),
            "setSmartArtAnchor" => XlsxOp::SetSmartArtAnchor(parse_set_smartart_anchor(obj, i)?),
            "setSmartArtColors" => XlsxOp::SetSmartArtColors(parse_set_smartart_colors(obj, i)?),
            "setSmartArtStyle" => XlsxOp::SetSmartArtStyle(parse_set_smartart_style(obj, i)?),
            "setSmartArtLayout" => XlsxOp::SetSmartArtLayout(parse_set_smartart_layout(obj, i)?),
            "setProps" => XlsxOp::SetProps(crate::props::parse_set_props(obj, i, "xlsx:workbook")?),
            "setCustomProp" => XlsxOp::SetCustomProp(crate::props::parse_set_custom_prop(
                obj,
                i,
                "xlsx:workbook",
            )?),
            "deleteCustomProp" => {
                XlsxOp::DeleteCustomProp(crate::props::parse_delete_custom_prop(obj, i)?)
            }
            "setParaText"
            | "setNotesText"
            | "setPicturePos"
            | "setChartPos"
            | "setSmartArtPos"
            | "setHeaderFooterDistance"
            | "setPageMargin"
            | "setPageSize"
            | "setColumns"
            | "setVerticalAlign"
            | "setLineNumbering"
            | "setDocGrid"
            | "setRtlGutter"
            | "setMirrorMargins"
            | "setGutterAtTop"
            | "setHeaderFooterLink"
            | "insertHeaderFooter"
            | "setTitlePage"
            | "setEvenAndOddHeaders"
            | "setPictureWrap"
            | "setTextBoxWrap"
            | "setTextBoxFill"
            | "setTextBoxLine"
            | "setTextBoxDirection"
            | "setTextBoxTextAnchor"
            | "insertPara"
            | "deletePara"
            | "insertTable"
            | "deleteTable"
            | "insertRow"
            | "deleteRow"
            | "insertCol"
            | "deleteCol"
            | "mergeCells"
            | "splitCells"
            | "acceptRevision"
            | "rejectRevision"
            | "insertBookmark"
            | "setBookmark"
            | "deleteBookmark"
            | "insertField"
            | "setField"
            | "deleteField"
            | "insertNumbering"
            | "setNumbering"
            | "setParaNumbering"
            | "setStyle"
            | "setParaStyle"
            | "insertStyle"
            | "deleteStyle"
            | "setRunStyle"
            | "setTableStyle"
            | "setTableFloat"
            | "setTableWidth"
            | "setTableAlign"
            | "setTableRowHeight"
            | "setTableCellMar"
            | "setTableBorders"
            | "setCellBorders"
            | "setCellFormat"
            | "setParaFormat"
            | "setRunFormat"
            | "insertRuby"
            | "setRuby"
            | "deleteRuby"
            | "setEastAsianSettings"
            | "insertSection"
            | "deleteSection"
            | "insertToc"
            | "deleteToc"
            | "insertCaption"
            | "deleteCaption"
            | "insertCrossRef"
            | "deleteCrossRef"
            | "insertIndex"
            | "deleteIndex"
            | "insertIndexEntry"
            | "deleteIndexEntry"
            | "insertSource"
            | "deleteSource"
            | "insertCitation"
            | "deleteCitation"
            | "insertBibliography"
            | "deleteBibliography"
            | "insertDocPart"
            | "deleteDocPart"
            | "insertDocPartContent"
            | "insertEmbeddedFont"
            | "deleteEmbeddedFont"
            | "setFontEmbed"
            | "setPageBackground"
            | "setPageBorders"
            | "setFormProt"
            | "insertEditRange"
            | "deleteEditRange"
            | "setBibliographyStyle"
            | "insertContentControl"
            | "setContentControl"
            | "deleteContentControl"
            | "setTextBoxLink"
            | "insertFootnote"
            | "insertEndnote"
            | "deleteFootnote"
            | "deleteEndnote"
            | "setNoteNumbering"
            | "setChartWrap"
            | "setSmartArtWrap"
            | "insertWatermark"
            | "deleteWatermark"
            | "insertInkCanvas"
            | "replaceInkCanvas"
            | "deleteInkCanvas"
            | "setInkCanvasSize"
            | "setInkCanvasWrap"
            | "setModel3dWrap"
            | "setModel3dPos"
            | "setMathPos"
            | "insertSlide"
            | "deleteSlide"
            | "moveSlide"
            | "setSlideHidden"
            | "setTransition"
            | "insertAnim"
            | "setAnim"
            | "deleteAnim"
            | "moveAnim"
            | "setSlideLayout"
            | "insertLayout"
            | "deleteLayout"
            | "insertPlaceholder"
            | "deletePlaceholder"
            | "insertMaster"
            | "deleteMaster"
            | "setSlideSize"
            | "setNotesSize"
            | "setClrMap"
            | "setClrMapOvr"
            | "setHandoutText"
            | "setShapePos"
            | "setShapeSize"
            | "setShapeFill"
            | "setShapeLine"
            | "setShapeGeom"
            | "setConnector"
            | "setTableCellText"
            | "setTableCellFill"
            | "setTableCellBorders"
            | "setTableCellInset"
            | "setTableCellPr"
            | "setTableStyleRef"
            | "setTablePos"
            | "setTableSize"
            | "setTableColWidths"
            | "setTableTrHeight"
            | "insertVideo"
            | "insertAudio"
            | "deleteMedia"
            | "setMedia"
            | "insertSlideSection"
            | "setSlideSectionName"
            | "deleteSlideSection"
            | "setSlideSection"
            | "insertCustomShow"
            | "setCustomShow"
            | "deleteCustomShow"
            | "insertZoom"
            | "deleteZoom"
            | "setZoom"
            | "setShowPr" => {
                return Err(Error::with_path(
                    ErrorKind::OpNotForFormat {
                        format: "xlsx".into(),
                        op: op_name.to_string(),
                    },
                    format!("ops[{i}].op"),
                ));
            }
            other => {
                return Err(Error::with_path(
                    ErrorKind::UnsupportedOp {
                        op: other.to_string(),
                    },
                    format!("ops[{i}].op"),
                ));
            }
        };
        out.push(parsed);
    }
    Ok(out)
}
