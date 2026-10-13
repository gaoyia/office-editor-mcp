//! pptx 自己的 scope 与 ops 解析。

use serde_json::Value;

use crate::error::{Error, ErrorKind, ScopeNeed, TypeName};
use crate::json;

mod types;
pub use types::*;
mod common;
use common::*;
mod shape;
pub(crate) use shape::parse_create_connector;
use shape::*;
mod slides;
pub(crate) use slides::parse_create_transition;
use slides::*;
mod timing;
pub(crate) use timing::parse_create_anim;
use timing::*;
mod master;
use master::*;
mod theme_remain;
use theme_remain::*;
mod theme_clrmap;
use theme_clrmap::*;
mod link;
use link::*;
mod comments;
pub(crate) use comments::parse_create_comment;
use comments::*;
mod picture;
pub(crate) use picture::parse_create_picture;
use picture::*;
mod media;
use media::*;
mod chart;
pub(crate) use chart::parse_create_chart;
use chart::*;
mod smartart;
pub(crate) use smartart::parse_create_smartart;
use smartart::*;
mod table;
pub(crate) use table::parse_create_table;
use table::*;
mod extras;
use extras::*;
pub(crate) use extras::{
    parse_create_custom_show, parse_create_handout, parse_create_media, parse_create_ole,
    parse_create_protection, parse_create_section, parse_create_shape, parse_create_show_pr,
    parse_create_zoom,
};

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
            "slide",
            "maxShapes",
            "pictures",
            "media",
            "charts",
            "smartArt",
            "notes",
            "comments",
            "hyperlinks",
            "fields",
            "tables",
            "connectors",
            "timing",
            "masters",
            "notesMaster",
            "handout",
            "sections",
            "customShows",
            "oleObjects",
            "model3ds",
            "maths",
            "zooms",
        ],
    )?;
    let pictures = parse_bool_field(obj.get("pictures"), "pictures")?;
    let media = parse_bool_field(obj.get("media"), "media")?;
    let charts = parse_bool_field(obj.get("charts"), "charts")?;
    let smart_art = parse_bool_field(obj.get("smartArt"), "smartArt")?;
    let notes = parse_bool_field(obj.get("notes"), "notes")?;
    let comments = parse_bool_field(obj.get("comments"), "comments")?;
    let hyperlinks = parse_bool_field(obj.get("hyperlinks"), "hyperlinks")?;
    let fields = parse_bool_field(obj.get("fields"), "fields")?;
    let tables = parse_bool_field(obj.get("tables"), "tables")?;
    let connectors = parse_bool_field(obj.get("connectors"), "connectors")?;
    let timing = parse_bool_field(obj.get("timing"), "timing")?;
    let masters = parse_bool_field(obj.get("masters"), "masters")?;
    let notes_master = parse_bool_field(obj.get("notesMaster"), "notesMaster")?;
    let handout = parse_bool_field(obj.get("handout"), "handout")?;
    let sections = parse_bool_field(obj.get("sections"), "sections")?;
    let custom_shows = parse_bool_field(obj.get("customShows"), "customShows")?;
    let ole_objects = parse_bool_field(obj.get("oleObjects"), "oleObjects")?;
    let model3ds = parse_bool_field(obj.get("model3ds"), "model3ds")?;
    let maths = parse_bool_field(obj.get("maths"), "maths")?;
    let zooms = parse_bool_field(obj.get("zooms"), "zooms")?;
    let slide = match obj.get("slide") {
        None => None,
        Some(s) => {
            let n = s.as_u64().ok_or_else(|| {
                Error::with_path(
                    ErrorKind::WrongType {
                        field: "slide".into(),
                        expected: TypeName::Integer,
                    },
                    "scope.slide",
                )
            })?;
            if n < 1 {
                return Err(Error::with_path(
                    ErrorKind::MinValue {
                        field: "slide".into(),
                        min: 1,
                    },
                    "scope.slide",
                ));
            }
            Some(n as u32)
        }
    };
    if obj.contains_key("maxShapes") && slide.is_none() {
        return Err(Error::with_path(
            ErrorKind::ScopeRequired {
                need: ScopeNeed::Slide,
            },
            "scope.slide",
        ));
    }
    let max_shapes = match obj.get("maxShapes") {
        None => 2000,
        Some(v) => {
            let n = v.as_u64().ok_or_else(|| {
                Error::with_path(
                    ErrorKind::WrongType {
                        field: "maxShapes".into(),
                        expected: TypeName::Integer,
                    },
                    "scope.maxShapes",
                )
            })?;
            if !(1..=5000).contains(&n) {
                return Err(Error::with_path(
                    ErrorKind::OutOfRange {
                        field: "maxShapes".into(),
                        min: 1,
                        max: 5000,
                    },
                    "scope.maxShapes",
                ));
            }
            n
        }
    };
    Ok(Some(InspectScope {
        slide,
        max_shapes: max_shapes as u32,
        pictures,
        media,
        charts,
        smart_art,
        notes,
        comments,
        hyperlinks,
        fields,
        tables,
        connectors,
        timing,
        masters,
        notes_master,
        handout,
        sections,
        custom_shows,
        ole_objects,
        model3ds,
        maths,
        zooms,
    }))
}

pub fn parse_ops(ops: &[Value]) -> Result<Vec<PptxOp>, Error> {
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
        if is_foreign_op(op_name) {
            return Err(Error::with_path(
                ErrorKind::OpNotForFormat {
                    format: "pptx".into(),
                    op: op_name.to_string(),
                },
                format!("ops[{i}].op"),
            ));
        }
        let parsed = match op_name {
            "setShapeText" => PptxOp::SetShapeText(parse_set_shape_text(obj, i)?),
            "insertShape" => PptxOp::InsertShape(parse_insert_shape(obj, i)?),
            "insertConnector" => PptxOp::InsertConnector(parse_insert_connector(obj, i)?),
            "deleteConnector" => PptxOp::DeleteConnector(parse_delete_connector(obj, i)?),
            "groupShapes" => PptxOp::GroupShapes(parse_group_shapes(obj, i)?),
            "ungroup" => PptxOp::Ungroup(parse_ungroup(obj, i)?),
            "deleteShape" => PptxOp::DeleteShape(parse_delete_shape(obj, i)?),
            "setShapePos" => PptxOp::SetShapePos(parse_set_shape_pos(obj, i)?),
            "setShapeSize" => PptxOp::SetShapeSize(parse_set_shape_size(obj, i)?),
            "setShapeFill" => PptxOp::SetShapeFill(parse_set_shape_fill(obj, i)?),
            "setShapeFormat" => PptxOp::SetShapeFormat(parse_set_shape_format(obj, i)?),
            "setShapeLine" => PptxOp::SetShapeLine(parse_set_shape_line(obj, i)?),
            "setShapeGeom" => PptxOp::SetShapeGeom(parse_set_shape_geom(obj, i)?),
            "setConnector" => PptxOp::SetConnector(parse_set_connector(obj, i)?),
            "setNotesText" => PptxOp::SetNotesText(parse_set_notes_text(obj, i)?),
            "insertComment" => PptxOp::InsertComment(parse_insert_comment(obj, i)?),
            "replyComment" => PptxOp::ReplyComment(parse_reply_comment(obj, i)?),
            "deleteComment" => PptxOp::DeleteComment(parse_delete_comment(obj, i)?),
            "setCommentText" => PptxOp::SetCommentText(parse_set_comment_text(obj, i)?),
            "setCommentMeta" => PptxOp::SetCommentMeta(parse_set_comment_meta(obj, i)?),
            "setCommentDone" => PptxOp::SetCommentDone(parse_set_comment_done(obj, i)?),
            "replacePicture" => PptxOp::ReplacePicture(parse_replace_pic(obj, i)?),
            "insertPicture" => PptxOp::InsertPicture(parse_insert_pic(obj, i)?),
            "deletePicture" => PptxOp::DeletePicture(parse_delete_pic(obj, i)?),
            "setPictureSize" => PptxOp::SetPictureSize(parse_set_pic_size(obj, i)?),
            "setPicturePos" => PptxOp::SetPicturePos(parse_set_pic_pos(obj, i)?),
            "insertVideo" => PptxOp::InsertVideo(parse_insert_media(obj, i, "insertVideo")?),
            "insertAudio" => PptxOp::InsertAudio(parse_insert_media(obj, i, "insertAudio")?),
            "deleteMedia" => PptxOp::DeleteMedia(parse_delete_media(obj, i)?),
            "setMedia" => PptxOp::SetMedia(parse_set_media(obj, i)?),
            "insertSlideSection" => PptxOp::InsertSlideSection(parse_insert_slide_section(obj, i)?),
            "setSlideSectionName" => {
                PptxOp::SetSlideSectionName(parse_set_slide_section_name(obj, i)?)
            }
            "deleteSlideSection" => PptxOp::DeleteSlideSection(parse_delete_slide_section(obj, i)?),
            "setSlideSection" => PptxOp::SetSlideSection(parse_set_slide_section(obj, i)?),
            "insertCustomShow" => PptxOp::InsertCustomShow(parse_insert_custom_show(obj, i)?),
            "setCustomShow" => PptxOp::SetCustomShow(parse_set_custom_show(obj, i)?),
            "deleteCustomShow" => PptxOp::DeleteCustomShow(parse_delete_custom_show(obj, i)?),
            "insertOle" => PptxOp::InsertOle(parse_insert_ole(obj, i)?),
            "deleteOle" => PptxOp::DeleteOle(parse_delete_ole(obj, i)?),
            "insertModel3d" => PptxOp::InsertModel3d(parse_insert_model3d(obj, i)?),
            "replaceModel3d" => PptxOp::ReplaceModel3d(parse_replace_model3d(obj, i)?),
            "deleteModel3d" => PptxOp::DeleteModel3d(parse_delete_model3d(obj, i)?),
            "setModel3dSize" => PptxOp::SetModel3dSize(parse_set_model3d_size(obj, i)?),
            "setModel3dPos" => PptxOp::SetModel3dPos(parse_set_model3d_pos(obj, i)?),
            "insertMath" => PptxOp::InsertMath(parse_insert_math(obj, i)?),
            "setMath" => PptxOp::SetMath(parse_set_math(obj, i)?),
            "deleteMath" => PptxOp::DeleteMath(parse_delete_math(obj, i)?),
            "setMathSize" => PptxOp::SetMathSize(parse_set_math_size(obj, i)?),
            "setMathPos" => PptxOp::SetMathPos(parse_set_math_pos(obj, i)?),
            "insertZoom" => PptxOp::InsertZoom(parse_insert_zoom(obj, i)?),
            "deleteZoom" => PptxOp::DeleteZoom(parse_delete_zoom(obj, i)?),
            "setZoom" => PptxOp::SetZoom(parse_set_zoom(obj, i)?),
            "setShowPr" => PptxOp::SetShowPr(parse_set_show_pr(obj, i)?),
            "setProtection" => PptxOp::SetProtection(parse_set_protection(obj, i)?),
            "insertChart" => PptxOp::InsertChart(parse_insert_chart(obj, i)?),
            "deleteChart" => PptxOp::DeleteChart(parse_delete_chart(obj, i)?),
            "setChartTitle" => PptxOp::SetChartTitle(parse_set_chart_title(obj, i)?),
            "setChartSeries" => PptxOp::SetChartSeries(parse_set_chart_series(obj, i)?),
            "addChartSeries" => PptxOp::AddChartSeries(parse_add_chart_series(obj, i)?),
            "deleteChartSeries" => PptxOp::DeleteChartSeries(parse_delete_chart_series(obj, i)?),
            "setChartType" => PptxOp::SetChartType(parse_set_chart_type(obj, i)?),
            "setChartAxisTitle" => PptxOp::SetChartAxisTitle(parse_set_chart_axis_title(obj, i)?),
            "setChartLegend" => PptxOp::SetChartLegend(parse_set_chart_legend(obj, i)?),
            "setChartDataLabels" => {
                PptxOp::SetChartDataLabels(parse_set_chart_data_labels(obj, i)?)
            }
            "setChartSize" => PptxOp::SetChartSize(parse_set_chart_size(obj, i)?),
            "setChartPos" => PptxOp::SetChartPos(parse_set_chart_pos(obj, i)?),
            "insertTrendline" => PptxOp::InsertTrendline(parse_insert_trendline(obj, i)?),
            "setTrendline" => PptxOp::SetTrendline(parse_set_trendline(obj, i)?),
            "deleteTrendline" => PptxOp::DeleteTrendline(parse_delete_trendline(obj, i)?),
            "insertErrorBars" => PptxOp::InsertErrorBars(parse_insert_error_bars(obj, i)?),
            "setErrorBars" => PptxOp::SetErrorBars(parse_set_error_bars(obj, i)?),
            "deleteErrorBars" => PptxOp::DeleteErrorBars(parse_delete_error_bars(obj, i)?),
            "setChartSeriesAxis" => PptxOp::SetChartSeriesAxis(parse_set_series_axis(obj, i)?),
            "insertSmartArt" => PptxOp::InsertSmartArt(parse_insert_smartart(obj, i)?),
            "deleteSmartArt" => PptxOp::DeleteSmartArt(parse_delete_smartart(obj, i)?),
            "setSmartArtText" => PptxOp::SetSmartArtText(parse_set_smartart_text(obj, i)?),
            "addSmartArtPoint" => PptxOp::AddSmartArtPoint(parse_add_smartart_point(obj, i)?),
            "deleteSmartArtPoint" => {
                PptxOp::DeleteSmartArtPoint(parse_delete_smartart_point(obj, i)?)
            }
            "setSmartArtSize" => PptxOp::SetSmartArtSize(parse_set_smartart_size(obj, i)?),
            "setSmartArtPos" => PptxOp::SetSmartArtPos(parse_set_smartart_pos(obj, i)?),
            "setSmartArtColors" => PptxOp::SetSmartArtColors(parse_set_smartart_colors(obj, i)?),
            "setSmartArtStyle" => PptxOp::SetSmartArtStyle(parse_set_smartart_style(obj, i)?),
            "setSmartArtLayout" => PptxOp::SetSmartArtLayout(parse_set_smartart_layout(obj, i)?),
            "insertTable" => PptxOp::InsertTable(parse_insert_table(obj, i)?),
            "deleteTable" => PptxOp::DeleteTable(parse_delete_table(obj, i)?),
            "insertRow" => PptxOp::InsertRow(parse_insert_row(obj, i)?),
            "deleteRow" => PptxOp::DeleteRow(parse_delete_row(obj, i)?),
            "insertCol" => PptxOp::InsertCol(parse_insert_col(obj, i)?),
            "deleteCol" => PptxOp::DeleteCol(parse_delete_col(obj, i)?),
            "setTableCellText" => PptxOp::SetTableCellText(parse_set_table_cell_text(obj, i)?),
            "setTableCellFill" => PptxOp::SetTableCellFill(parse_set_table_cell_fill(obj, i)?),
            "setTableCellBorders" => {
                PptxOp::SetTableCellBorders(parse_set_table_cell_borders(obj, i)?)
            }
            "setTableCellInset" => PptxOp::SetTableCellInset(parse_set_table_cell_inset(obj, i)?),
            "setTablePos" => PptxOp::SetTablePos(parse_set_table_pos(obj, i)?),
            "setTableSize" => PptxOp::SetTableSize(parse_set_table_size(obj, i)?),
            "setTableColWidths" => PptxOp::SetTableColWidths(parse_set_table_col_widths(obj, i)?),
            "setTableTrHeight" => PptxOp::SetTableTrHeight(parse_set_table_tr_height(obj, i)?),
            "insertSlide" => PptxOp::InsertSlide(parse_insert_slide(obj, i)?),
            "deleteSlide" => PptxOp::DeleteSlide(parse_delete_slide(obj, i)?),
            "moveSlide" => PptxOp::MoveSlide(parse_move_slide(obj, i)?),
            "setSlideHidden" => PptxOp::SetSlideHidden(parse_set_slide_hidden(obj, i)?),
            "setTransition" => PptxOp::SetTransition(parse_set_transition(obj, i)?),
            "insertAnim" => PptxOp::InsertAnim(parse_insert_anim(obj, i)?),
            "setAnim" => PptxOp::SetAnim(parse_set_anim(obj, i)?),
            "deleteAnim" => PptxOp::DeleteAnim(parse_delete_anim(obj, i)?),
            "moveAnim" => PptxOp::MoveAnim(parse_move_anim(obj, i)?),
            "setSlideLayout" => PptxOp::SetSlideLayout(parse_set_slide_layout(obj, i)?),
            "insertLayout" => PptxOp::InsertLayout(parse_insert_layout(obj, i)?),
            "deleteLayout" => PptxOp::DeleteLayout(parse_delete_layout(obj, i)?),
            "insertPlaceholder" => PptxOp::InsertPlaceholder(parse_insert_placeholder(obj, i)?),
            "deletePlaceholder" => PptxOp::DeletePlaceholder(parse_delete_placeholder(obj, i)?),
            "insertMaster" => PptxOp::InsertMaster(parse_insert_master(obj, i)?),
            "deleteMaster" => PptxOp::DeleteMaster(parse_delete_master(obj, i)?),
            "setSlideSize" => PptxOp::SetSlideSize(parse_set_slide_size(obj, i)?),
            "setThemeColors" => PptxOp::SetThemeColors(parse_set_theme_colors(obj, i)?),
            "setThemeFonts" => PptxOp::SetThemeFonts(parse_set_theme_fonts(obj, i)?),
            "setThemeFmt" => PptxOp::SetThemeFmt(parse_set_theme_fmt(obj, i)?),
            "setThemeExtraSchemes" => {
                PptxOp::SetThemeExtraSchemes(parse_set_theme_extra_schemes(obj, i)?)
            }
            "setClrMap" => PptxOp::SetClrMap(parse_set_clr_map(obj, i)?),
            "setClrMapOvr" => PptxOp::SetClrMapOvr(parse_set_clr_map_ovr(obj, i)?),
            "setThemeOverride" => PptxOp::SetThemeOverride(parse_set_theme_override(obj, i)?),
            "setHandoutText" => PptxOp::SetHandoutText(parse_set_handout_text(obj, i)?),
            "insertHyperlink" => PptxOp::InsertHyperlink(parse_insert_hyperlink(obj, i)?),
            "setHyperlink" => PptxOp::SetHyperlink(parse_set_hyperlink(obj, i)?),
            "deleteHyperlink" => PptxOp::DeleteHyperlink(parse_delete_hyperlink(obj, i)?),
            "insertField" => PptxOp::InsertField(parse_insert_field(obj, i)?),
            "setField" => PptxOp::SetField(parse_set_field(obj, i)?),
            "deleteField" => PptxOp::DeleteField(parse_delete_field(obj, i)?),
            "setProps" => {
                PptxOp::SetProps(crate::props::parse_set_props(obj, i, "pptx:presentation")?)
            }
            "setCustomProp" => PptxOp::SetCustomProp(crate::props::parse_set_custom_prop(
                obj,
                i,
                "pptx:presentation",
            )?),
            "deleteCustomProp" => {
                PptxOp::DeleteCustomProp(crate::props::parse_delete_custom_prop(obj, i)?)
            }
            _ => {
                return Err(Error::with_path(
                    ErrorKind::UnsupportedOp {
                        op: op_name.to_string(),
                    },
                    format!("ops[{i}].op"),
                ));
            }
        };
        out.push(parsed);
    }
    Ok(out)
}

fn is_foreign_op(op_name: &str) -> bool {
    matches!(
        op_name,
        "setCell"
            | "setParaText"
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
            | "setHeaderFooterText"
            | "setPictureWrap"
            | "setPictureAnchor"
            | "setModel3dWrap"
            | "setModel3dAnchor"
            | "setMathAnchor"
            | "insertTextBox"
            | "deleteTextBox"
            | "setTextBoxSize"
            | "setTextBoxWrap"
            | "setTextBoxFill"
            | "setTextBoxLine"
            | "setTextBoxDirection"
            | "setTextBoxTextAnchor"
            | "insertPara"
            | "deletePara"
            | "mergeCells"
            | "splitCells"
            | "acceptRevision"
            | "rejectRevision"
            | "insertSparkline"
            | "setSparkline"
            | "deleteSparkline"
            | "insertSlicer"
            | "deleteSlicer"
            | "setSlicerSize"
            | "setSlicerAnchor"
            | "setSlicerView"
            | "setSlicerFilter"
            | "insertConnection"
            | "deleteConnection"
            | "insertExternalLink"
            | "setExternalLink"
            | "deleteExternalLink"
            | "insertCustomView"
            | "deleteCustomView"
            | "showCustomView"
            | "insertBookmark"
            | "setBookmark"
            | "deleteBookmark"
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
            | "setChartAnchor"
            | "insertChartSheet"
            | "deleteChartSheet"
            | "insertSheet"
            | "deleteSheet"
            | "renameSheet"
            | "moveSheet"
            | "setSheetHidden"
            | "setCellStyle"
            | "insertCellStyle"
            | "deleteCellStyle"
            | "setCellNamedStyle"
            | "setColWidth"
            | "setRowHeight"
            | "mergeRange"
            | "unmergeRange"
            | "insertDataValidation"
            | "setDataValidation"
            | "deleteDataValidation"
            | "insertConditionalFormat"
            | "setConditionalFormat"
            | "deleteConditionalFormat"
            | "insertSheetTable"
            | "setSheetTable"
            | "deleteSheetTable"
            | "insertPivotTable"
            | "setPivotTable"
            | "deletePivotTable"
            | "insertDefinedName"
            | "setDefinedName"
            | "deleteDefinedName"
            | "insertProtectedRange"
            | "setProtectedRange"
            | "deleteProtectedRange"
            | "setPrintSetup"
            | "setSheetView"
            | "setAutoFilter"
            | "setSortState"
            | "groupRows"
            | "ungroupRows"
            | "groupCols"
            | "ungroupCols"
            | "setOutline"
            | "setRowHidden"
            | "setColHidden"
            | "setSmartArtWrap"
            | "insertWatermark"
            | "deleteWatermark"
            | "insertInkCanvas"
            | "replaceInkCanvas"
            | "deleteInkCanvas"
            | "setInkCanvasSize"
            | "setInkCanvasWrap"
    )
}
