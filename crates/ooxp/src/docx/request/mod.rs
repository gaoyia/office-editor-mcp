//! docx 自己的 scope 与 ops 解析。

use serde_json::Value;

use super::para::NoteKind;
use crate::error::{Error, ErrorKind, ScopeNeed, TypeName};
use crate::json;

mod types;
pub use types::*;
mod common;
use common::*;
mod para;
use para::*;
mod sect;
use sect::*;
mod picture;
use picture::*;
mod drawing;
use drawing::*;
mod notes;
use notes::*;
mod comments;
use comments::*;
mod table;
use table::*;
mod revision;
use revision::*;
mod link;
use link::*;
mod field;
use field::*;
mod biblio;
use biblio::*;
mod font_embed;
use font_embed::*;
mod page;
use page::*;
mod math_ruby;
use math_ruby::*;
mod sdt;
use sdt::*;
mod style_fmt;
use style_fmt::*;
mod extras;
use extras::*;

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
            "from",
            "maxParas",
            "headerFooter",
            "pictures",
            "textBoxes",
            "connectors",
            "shapes",
            "inkCanvases",
            "textBoxContent",
            "notes",
            "comments",
            "charts",
            "smartArt",
            "oleObjects",
            "model3ds",
            "watermarks",
            "tables",
            "revisions",
            "hyperlinks",
            "bookmarks",
            "editRanges",
            "fields",
            "tocs",
            "captions",
            "crossRefs",
            "indexes",
            "indexEntries",
            "sources",
            "citations",
            "bibliographies",
            "docParts",
            "embeddedFonts",
            "maths",
            "rubies",
            "contentControls",
            "numbering",
            "styles",
            "runs",
        ],
    )?;

    let header_footer = parse_header_footer(obj.get("headerFooter"))?;
    let pictures = parse_bool_field(obj.get("pictures"), "pictures")?;
    let text_boxes = parse_bool_field(obj.get("textBoxes"), "textBoxes")?;
    let connectors = parse_bool_field(obj.get("connectors"), "connectors")?;
    let shapes = parse_bool_field(obj.get("shapes"), "shapes")?;
    let ink_canvases = parse_bool_field(obj.get("inkCanvases"), "inkCanvases")?;
    let text_box_content = parse_bool_field(obj.get("textBoxContent"), "textBoxContent")?;
    let notes = parse_bool_field(obj.get("notes"), "notes")?;
    let comments = parse_bool_field(obj.get("comments"), "comments")?;
    let charts = parse_bool_field(obj.get("charts"), "charts")?;
    let smart_art = parse_bool_field(obj.get("smartArt"), "smartArt")?;
    let ole_objects = parse_bool_field(obj.get("oleObjects"), "oleObjects")?;
    let model3ds = parse_bool_field(obj.get("model3ds"), "model3ds")?;
    let watermarks = parse_bool_field(obj.get("watermarks"), "watermarks")?;
    let tables = parse_bool_field(obj.get("tables"), "tables")?;
    let revisions = parse_bool_field(obj.get("revisions"), "revisions")?;
    let hyperlinks = parse_bool_field(obj.get("hyperlinks"), "hyperlinks")?;
    let bookmarks = parse_bool_field(obj.get("bookmarks"), "bookmarks")?;
    let edit_ranges = parse_bool_field(obj.get("editRanges"), "editRanges")?;
    let fields = parse_bool_field(obj.get("fields"), "fields")?;
    let tocs = parse_bool_field(obj.get("tocs"), "tocs")?;
    let captions = parse_bool_field(obj.get("captions"), "captions")?;
    let cross_refs = parse_bool_field(obj.get("crossRefs"), "crossRefs")?;
    let indexes = parse_bool_field(obj.get("indexes"), "indexes")?;
    let index_entries = parse_bool_field(obj.get("indexEntries"), "indexEntries")?;
    let sources = parse_bool_field(obj.get("sources"), "sources")?;
    let citations = parse_bool_field(obj.get("citations"), "citations")?;
    let bibliographies = parse_bool_field(obj.get("bibliographies"), "bibliographies")?;
    let doc_parts = parse_bool_field(obj.get("docParts"), "docParts")?;
    let embedded_fonts = parse_bool_field(obj.get("embeddedFonts"), "embeddedFonts")?;
    let maths = parse_bool_field(obj.get("maths"), "maths")?;
    let rubies = parse_bool_field(obj.get("rubies"), "rubies")?;
    let content_controls = parse_bool_field(obj.get("contentControls"), "contentControls")?;
    let numbering = parse_bool_field(obj.get("numbering"), "numbering")?;
    let styles = parse_bool_field(obj.get("styles"), "styles")?;
    let runs = parse_bool_field(obj.get("runs"), "runs")?;
    if text_box_content && !text_boxes {
        return Err(Error::with_path(
            ErrorKind::ScopeFieldRequires {
                field: "textBoxContent".into(),
                required: "textBoxes".into(),
            },
            "scope.textBoxContent",
        ));
    }
    let from = obj.get("from");
    let max = obj.get("maxParas");
    match (from, max) {
        (None, None) => {
            if !obj.contains_key("headerFooter")
                && !obj.contains_key("pictures")
                && !obj.contains_key("textBoxes")
                && !obj.contains_key("connectors")
                && !obj.contains_key("shapes")
                && !obj.contains_key("inkCanvases")
                && !obj.contains_key("textBoxContent")
                && !obj.contains_key("notes")
                && !obj.contains_key("comments")
                && !obj.contains_key("charts")
                && !obj.contains_key("smartArt")
                && !obj.contains_key("oleObjects")
                && !obj.contains_key("model3ds")
                && !obj.contains_key("watermarks")
                && !obj.contains_key("tables")
                && !obj.contains_key("revisions")
                && !obj.contains_key("hyperlinks")
                && !obj.contains_key("bookmarks")
                && !obj.contains_key("editRanges")
                && !obj.contains_key("fields")
                && !obj.contains_key("tocs")
                && !obj.contains_key("captions")
                && !obj.contains_key("crossRefs")
                && !obj.contains_key("indexes")
                && !obj.contains_key("indexEntries")
                && !obj.contains_key("sources")
                && !obj.contains_key("citations")
                && !obj.contains_key("bibliographies")
                && !obj.contains_key("docParts")
                && !obj.contains_key("embeddedFonts")
                && !obj.contains_key("maths")
                && !obj.contains_key("rubies")
                && !obj.contains_key("contentControls")
                && !obj.contains_key("numbering")
                && !obj.contains_key("styles")
                && !obj.contains_key("runs")
            {
                return Ok(None);
            }
            Ok(Some(InspectScope {
                from: None,
                max_paras: None,
                header_footer,
                pictures,
                text_boxes,
                connectors,
                shapes,
                ink_canvases,
                text_box_content,
                notes,
                comments,
                charts,
                smart_art,
                ole_objects,
                model3ds,
                watermarks,
                tables,
                revisions,
                hyperlinks,
                bookmarks,
                edit_ranges,
                fields,
                tocs,
                captions,
                cross_refs,
                indexes,
                index_entries,
                sources,
                citations,
                bibliographies,
                doc_parts,
                embedded_fonts,
                maths,
                rubies,
                content_controls,
                numbering,
                styles,
                runs,
            }))
        }
        (Some(f), Some(m)) => {
            let from = f.as_u64().ok_or_else(|| {
                Error::with_path(
                    ErrorKind::WrongType {
                        field: "from".into(),
                        expected: TypeName::Integer,
                    },
                    "scope.from",
                )
            })?;
            let max = m.as_u64().ok_or_else(|| {
                Error::with_path(
                    ErrorKind::WrongType {
                        field: "maxParas".into(),
                        expected: TypeName::Integer,
                    },
                    "scope.maxParas",
                )
            })?;
            if from < 1 {
                return Err(Error::with_path(
                    ErrorKind::MinValue {
                        field: "from".into(),
                        min: 1,
                    },
                    "scope.from",
                ));
            }
            if !(1..=5000).contains(&max) {
                return Err(Error::with_path(
                    ErrorKind::OutOfRange {
                        field: "maxParas".into(),
                        min: 1,
                        max: 5000,
                    },
                    "scope.maxParas",
                ));
            }
            Ok(Some(InspectScope {
                from: Some(from as u32),
                max_paras: Some(max as u32),
                header_footer,
                pictures,
                text_boxes,
                connectors,
                shapes,
                ink_canvases,
                text_box_content,
                notes,
                comments,
                charts,
                smart_art,
                ole_objects,
                model3ds,
                watermarks,
                tables,
                revisions,
                hyperlinks,
                bookmarks,
                edit_ranges,
                fields,
                tocs,
                captions,
                cross_refs,
                indexes,
                index_entries,
                sources,
                citations,
                bibliographies,
                doc_parts,
                embedded_fonts,
                maths,
                rubies,
                content_controls,
                numbering,
                styles,
                runs,
            }))
        }
        _ => Err(Error::with_path(
            ErrorKind::ScopeRequired {
                need: ScopeNeed::FromAndMaxParas,
            },
            "scope",
        )),
    }
}

pub fn parse_ops(ops: &[Value]) -> Result<Vec<DocxOp>, Error> {
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
        if op_name == "setCell"
            || op_name == "setShapeText"
            || op_name == "setNotesText"
            || op_name == "setPictureAnchor"
            || op_name == "setChartAnchor"
            || op_name == "setModel3dAnchor"
            || op_name == "setHeaderFooterText"
            || op_name == "insertChartSheet"
            || op_name == "deleteChartSheet"
            || op_name == "insertSheet"
            || op_name == "deleteSheet"
            || op_name == "renameSheet"
            || op_name == "moveSheet"
            || op_name == "setSheetHidden"
            || op_name == "setCellStyle"
            || op_name == "insertCellStyle"
            || op_name == "deleteCellStyle"
            || op_name == "setCellNamedStyle"
            || op_name == "setColWidth"
            || op_name == "setRowHeight"
            || op_name == "mergeRange"
            || op_name == "unmergeRange"
            || op_name == "insertDataValidation"
            || op_name == "setDataValidation"
            || op_name == "deleteDataValidation"
            || op_name == "insertConditionalFormat"
            || op_name == "setConditionalFormat"
            || op_name == "deleteConditionalFormat"
            || op_name == "insertSheetTable"
            || op_name == "setSheetTable"
            || op_name == "deleteSheetTable"
            || op_name == "insertPivotTable"
            || op_name == "setPivotTable"
            || op_name == "deletePivotTable"
            || op_name == "insertDefinedName"
            || op_name == "setDefinedName"
            || op_name == "deleteDefinedName"
            || op_name == "insertProtectedRange"
            || op_name == "setProtectedRange"
            || op_name == "deleteProtectedRange"
            || op_name == "setPrintSetup"
            || op_name == "setSheetView"
            || op_name == "setAutoFilter"
            || op_name == "setSortState"
            || op_name == "insertCustomView"
            || op_name == "deleteCustomView"
            || op_name == "showCustomView"
            || op_name == "groupRows"
            || op_name == "ungroupRows"
            || op_name == "groupCols"
            || op_name == "ungroupCols"
            || op_name == "setOutline"
            || op_name == "setRowHidden"
            || op_name == "setColHidden"
            || op_name == "setPicturePos"
            || op_name == "setChartPos"
            || op_name == "setSmartArtPos"
            || op_name == "setModel3dPos"
            || op_name == "setMathPos"
            || op_name == "setMathAnchor"
            || op_name == "setMathSize"
            || op_name == "insertSparkline"
            || op_name == "setSparkline"
            || op_name == "deleteSparkline"
            || op_name == "insertSlicer"
            || op_name == "deleteSlicer"
            || op_name == "setSlicerSize"
            || op_name == "setSlicerAnchor"
            || op_name == "setSlicerView"
            || op_name == "setSlicerFilter"
            || op_name == "insertConnection"
            || op_name == "deleteConnection"
            || op_name == "insertExternalLink"
            || op_name == "setExternalLink"
            || op_name == "deleteExternalLink"
            || op_name == "insertSlide"
            || op_name == "deleteSlide"
            || op_name == "moveSlide"
            || op_name == "setSlideHidden"
            || op_name == "setTransition"
            || op_name == "insertAnim"
            || op_name == "setAnim"
            || op_name == "deleteAnim"
            || op_name == "moveAnim"
            || op_name == "setSlideLayout"
            || op_name == "insertLayout"
            || op_name == "deleteLayout"
            || op_name == "insertPlaceholder"
            || op_name == "deletePlaceholder"
            || op_name == "insertMaster"
            || op_name == "deleteMaster"
            || op_name == "setSlideSize"
            || op_name == "setNotesSize"
            || op_name == "setClrMap"
            || op_name == "setClrMapOvr"
            || op_name == "setHandoutText"
            || op_name == "setShapeFormat"
            || op_name == "setShapeGeom"
            || op_name == "setConnector"
            || op_name == "setTableCellText"
            || op_name == "setTableCellFill"
            || op_name == "setTableCellBorders"
            || op_name == "setTableCellInset"
            || op_name == "setTableCellPr"
            || op_name == "setTableStyleRef"
            || op_name == "setTablePos"
            || op_name == "setTableSize"
            || op_name == "setTableColWidths"
            || op_name == "setTableTrHeight"
            || op_name == "insertVideo"
            || op_name == "insertAudio"
            || op_name == "deleteMedia"
            || op_name == "setMedia"
            || op_name == "insertSlideSection"
            || op_name == "setSlideSectionName"
            || op_name == "deleteSlideSection"
            || op_name == "setSlideSection"
            || op_name == "insertCustomShow"
            || op_name == "setCustomShow"
            || op_name == "deleteCustomShow"
            || op_name == "insertZoom"
            || op_name == "deleteZoom"
            || op_name == "setZoom"
            || op_name == "setShowPr"
        {
            return Err(Error::with_path(
                ErrorKind::OpNotForFormat {
                    format: "docx".into(),
                    op: op_name.to_string(),
                },
                format!("ops[{i}].op"),
            ));
        }
        match op_name {
            "setParaText" => out.push(DocxOp::SetParaText(parse_set_para_text(obj, i)?)),
            "setHeaderFooterDistance" => out.push(DocxOp::SetHeaderFooterDistance(
                parse_set_hf_distance(obj, i)?,
            )),
            "setHeaderFooterLink" => {
                out.push(DocxOp::SetHeaderFooterLink(parse_set_hf_link(obj, i)?))
            }
            "insertHeaderFooter" => out.push(DocxOp::InsertHeaderFooter(parse_insert_hf(obj, i)?)),
            "insertSection" => out.push(DocxOp::InsertSection(parse_insert_section(obj, i)?)),
            "deleteSection" => out.push(DocxOp::DeleteSection(parse_delete_section(obj, i)?)),
            "insertToc" => out.push(DocxOp::InsertToc(parse_insert_toc(obj, i)?)),
            "deleteToc" => out.push(DocxOp::DeleteToc(parse_delete_toc(obj, i)?)),
            "insertCaption" => out.push(DocxOp::InsertCaption(parse_insert_caption(obj, i)?)),
            "deleteCaption" => out.push(DocxOp::DeleteCaption(parse_delete_caption(obj, i)?)),
            "insertCrossRef" => out.push(DocxOp::InsertCrossRef(parse_insert_cross_ref(obj, i)?)),
            "deleteCrossRef" => out.push(DocxOp::DeleteCrossRef(parse_delete_cross_ref(obj, i)?)),
            "insertIndex" => out.push(DocxOp::InsertIndex(parse_insert_index(obj, i)?)),
            "deleteIndex" => out.push(DocxOp::DeleteIndex(parse_delete_index(obj, i)?)),
            "insertIndexEntry" => {
                out.push(DocxOp::InsertIndexEntry(parse_insert_index_entry(obj, i)?))
            }
            "deleteIndexEntry" => {
                out.push(DocxOp::DeleteIndexEntry(parse_delete_index_entry(obj, i)?))
            }
            "insertSource" => out.push(DocxOp::InsertSource(parse_insert_source(obj, i)?)),
            "deleteSource" => out.push(DocxOp::DeleteSource(parse_delete_source(obj, i)?)),
            "insertCitation" => out.push(DocxOp::InsertCitation(parse_insert_citation(obj, i)?)),
            "deleteCitation" => out.push(DocxOp::DeleteCitation(parse_delete_citation(obj, i)?)),
            "insertBibliography" => out.push(DocxOp::InsertBibliography(
                parse_insert_bibliography(obj, i)?,
            )),
            "deleteBibliography" => out.push(DocxOp::DeleteBibliography(
                parse_delete_bibliography(obj, i)?,
            )),
            "setBibliographyStyle" => out.push(DocxOp::SetBibliographyStyle(
                parse_set_bibliography_style(obj, i)?,
            )),
            "insertDocPart" => out.push(DocxOp::InsertDocPart(parse_insert_doc_part(obj, i)?)),
            "deleteDocPart" => out.push(DocxOp::DeleteDocPart(parse_delete_doc_part(obj, i)?)),
            "insertDocPartContent" => out.push(DocxOp::InsertDocPartContent(
                parse_insert_doc_part_content(obj, i)?,
            )),
            "insertEmbeddedFont" => out.push(DocxOp::InsertEmbeddedFont(
                parse_insert_embedded_font(obj, i)?,
            )),
            "deleteEmbeddedFont" => out.push(DocxOp::DeleteEmbeddedFont(
                parse_delete_embedded_font(obj, i)?,
            )),
            "setFontEmbed" => out.push(DocxOp::SetFontEmbed(parse_set_font_embed(obj, i)?)),
            "setPageBackground" => out.push(DocxOp::SetPageBackground(parse_set_page_background(
                obj, i,
            )?)),
            "setPageBorders" => out.push(DocxOp::SetPageBorders(parse_set_page_borders(obj, i)?)),
            "setFormProt" => out.push(DocxOp::SetFormProt(parse_set_form_prot(obj, i)?)),
            "insertMath" => out.push(DocxOp::InsertMath(parse_insert_math(obj, i)?)),
            "setMath" => out.push(DocxOp::SetMath(parse_set_math(obj, i)?)),
            "deleteMath" => out.push(DocxOp::DeleteMath(parse_delete_math(obj, i)?)),
            "insertRuby" => out.push(DocxOp::InsertRuby(parse_insert_ruby(obj, i)?)),
            "setRuby" => out.push(DocxOp::SetRuby(parse_set_ruby(obj, i)?)),
            "deleteRuby" => out.push(DocxOp::DeleteRuby(parse_delete_ruby(obj, i)?)),
            "setEastAsianSettings" => out.push(DocxOp::SetEastAsianSettings(
                parse_set_east_asian_settings(obj, i)?,
            )),
            "insertContentControl" => {
                out.push(DocxOp::InsertContentControl(parse_insert_sdt(obj, i)?))
            }
            "setContentControl" => out.push(DocxOp::SetContentControl(parse_set_sdt(obj, i)?)),
            "deleteContentControl" => {
                out.push(DocxOp::DeleteContentControl(parse_delete_sdt(obj, i)?))
            }
            "setTitlePage" => out.push(DocxOp::SetTitlePage(parse_set_title_page(obj, i)?)),
            "setEvenAndOddHeaders" => out.push(DocxOp::SetEvenAndOddHeaders(
                parse_set_even_and_odd(obj, i)?,
            )),
            "setPageMargin" => out.push(DocxOp::SetPageMargin(parse_set_page_margin(obj, i)?)),
            "setPageSize" => out.push(DocxOp::SetPageSize(parse_set_page_size(obj, i)?)),
            "setColumns" => out.push(DocxOp::SetColumns(parse_set_columns(obj, i)?)),
            "setVerticalAlign" => {
                out.push(DocxOp::SetVerticalAlign(parse_set_vertical_align(obj, i)?))
            }
            "setLineNumbering" => {
                out.push(DocxOp::SetLineNumbering(parse_set_line_numbering(obj, i)?))
            }
            "setDocGrid" => out.push(DocxOp::SetDocGrid(parse_set_doc_grid(obj, i)?)),
            "setRtlGutter" => out.push(DocxOp::SetRtlGutter(parse_set_rtl_gutter(obj, i)?)),
            "setMirrorMargins" => {
                out.push(DocxOp::SetMirrorMargins(parse_set_mirror_margins(obj, i)?))
            }
            "setGutterAtTop" => out.push(DocxOp::SetGutterAtTop(parse_set_gutter_at_top(obj, i)?)),
            "replacePicture" => out.push(DocxOp::ReplacePicture(parse_replace_pic(obj, i)?)),
            "insertPicture" => out.push(DocxOp::InsertPicture(parse_insert_pic(obj, i)?)),
            "deletePicture" => out.push(DocxOp::DeletePicture(parse_delete_pic(obj, i)?)),
            "setPictureSize" => out.push(DocxOp::SetPictureSize(parse_set_pic_size(obj, i)?)),
            "setPictureWrap" => out.push(DocxOp::SetPictureWrap(parse_set_pic_wrap(obj, i)?)),
            "insertTextBox" => out.push(DocxOp::InsertTextBox(parse_insert_txbx(obj, i)?)),
            "deleteTextBox" => out.push(DocxOp::DeleteTextBox(parse_delete_txbx(obj, i)?)),
            "setTextBoxSize" => out.push(DocxOp::SetTextBoxSize(parse_set_txbx_size(obj, i)?)),
            "setTextBoxWrap" => out.push(DocxOp::SetTextBoxWrap(parse_set_txbx_wrap(obj, i)?)),
            "setTextBoxFill" => out.push(DocxOp::SetTextBoxFill(parse_set_txbx_fill(obj, i)?)),
            "setTextBoxLine" => out.push(DocxOp::SetTextBoxLine(parse_set_txbx_line(obj, i)?)),
            "setTextBoxDirection" => {
                out.push(DocxOp::SetTextBoxDirection(parse_set_txbx_dir(obj, i)?))
            }
            "setTextBoxTextAnchor" => out.push(DocxOp::SetTextBoxTextAnchor(
                parse_set_txbx_text_anchor(obj, i)?,
            )),
            "insertConnector" => out.push(DocxOp::InsertConnector(parse_insert_connector(obj, i)?)),
            "deleteConnector" => out.push(DocxOp::DeleteConnector(parse_delete_connector(obj, i)?)),
            "insertShape" => out.push(DocxOp::InsertShape(parse_insert_shape(obj, i)?)),
            "deleteShape" => out.push(DocxOp::DeleteShape(parse_delete_shape(obj, i)?)),
            "setShapeSize" => out.push(DocxOp::SetShapeSize(parse_set_shape_size(obj, i)?)),
            "setShapePos" => out.push(DocxOp::SetShapePos(parse_set_shape_pos(obj, i)?)),
            "setShapeFill" => out.push(DocxOp::SetShapeFill(parse_set_shape_fill(obj, i)?)),
            "setShapeLine" => out.push(DocxOp::SetShapeLine(parse_set_shape_line(obj, i)?)),
            "groupShapes" => out.push(DocxOp::GroupShapes(parse_group_shapes(obj, i)?)),
            "ungroup" => out.push(DocxOp::Ungroup(parse_ungroup(obj, i)?)),
            "insertPara" => out.push(DocxOp::InsertPara(parse_insert_para(obj, i)?)),
            "deletePara" => out.push(DocxOp::DeletePara(parse_delete_para(obj, i)?)),
            "insertTable" => out.push(DocxOp::InsertTable(parse_insert_table(obj, i)?)),
            "deleteTable" => out.push(DocxOp::DeleteTable(parse_delete_table(obj, i)?)),
            "insertRow" => out.push(DocxOp::InsertRow(parse_insert_row(obj, i)?)),
            "deleteRow" => out.push(DocxOp::DeleteRow(parse_delete_row(obj, i)?)),
            "insertCol" => out.push(DocxOp::InsertCol(parse_insert_col(obj, i)?)),
            "deleteCol" => out.push(DocxOp::DeleteCol(parse_delete_col(obj, i)?)),
            "mergeCells" => out.push(DocxOp::MergeCells(parse_merge_cells(obj, i)?)),
            "splitCells" => out.push(DocxOp::SplitCells(parse_split_cells(obj, i)?)),
            "acceptRevision" => out.push(DocxOp::AcceptRevision(parse_revision_op(obj, i)?)),
            "rejectRevision" => out.push(DocxOp::RejectRevision(parse_revision_op(obj, i)?)),
            "insertHyperlink" => out.push(DocxOp::InsertHyperlink(parse_insert_hyperlink(obj, i)?)),
            "setHyperlink" => out.push(DocxOp::SetHyperlink(parse_set_hyperlink(obj, i)?)),
            "deleteHyperlink" => out.push(DocxOp::DeleteHyperlink(parse_delete_hyperlink(obj, i)?)),
            "insertBookmark" => out.push(DocxOp::InsertBookmark(parse_insert_bookmark(obj, i)?)),
            "setBookmark" => out.push(DocxOp::SetBookmark(parse_set_bookmark(obj, i)?)),
            "deleteBookmark" => out.push(DocxOp::DeleteBookmark(parse_delete_bookmark(obj, i)?)),
            "insertEditRange" => {
                out.push(DocxOp::InsertEditRange(parse_insert_edit_range(obj, i)?))
            }
            "deleteEditRange" => {
                out.push(DocxOp::DeleteEditRange(parse_delete_edit_range(obj, i)?))
            }
            "insertField" => out.push(DocxOp::InsertField(parse_insert_field(obj, i)?)),
            "setField" => out.push(DocxOp::SetField(parse_set_field(obj, i)?)),
            "deleteField" => out.push(DocxOp::DeleteField(parse_delete_field(obj, i)?)),
            "insertNumbering" => out.push(DocxOp::InsertNumbering(parse_insert_numbering(obj, i)?)),
            "setNumbering" => out.push(DocxOp::SetNumbering(parse_set_numbering(obj, i)?)),
            "setParaNumbering" => {
                out.push(DocxOp::SetParaNumbering(parse_set_para_numbering(obj, i)?))
            }
            "setStyle" => out.push(DocxOp::SetStyle(parse_set_style(obj, i)?)),
            "setParaStyle" => out.push(DocxOp::SetParaStyle(parse_set_para_style(obj, i)?)),
            "insertStyle" => out.push(DocxOp::InsertStyle(parse_insert_style(obj, i)?)),
            "deleteStyle" => out.push(DocxOp::DeleteStyle(parse_delete_style(obj, i)?)),
            "setRunStyle" => out.push(DocxOp::SetRunStyle(parse_set_run_style(obj, i)?)),
            "setTableStyle" => out.push(DocxOp::SetTableStyle(parse_set_table_style(obj, i)?)),
            "setTableFloat" => out.push(DocxOp::SetTableFloat(parse_set_table_float(obj, i)?)),
            "setTableWidth" => out.push(DocxOp::SetTableWidth(parse_set_table_width(obj, i)?)),
            "setTableAlign" => out.push(DocxOp::SetTableAlign(parse_set_table_align(obj, i)?)),
            "setTableRowHeight" => out.push(DocxOp::SetTableRowHeight(parse_set_table_row_height(
                obj, i,
            )?)),
            "setTableCellMar" => {
                out.push(DocxOp::SetTableCellMar(parse_set_table_cell_mar(obj, i)?))
            }
            "setTableBorders" => {
                out.push(DocxOp::SetTableBorders(parse_set_table_borders(obj, i)?))
            }
            "setCellBorders" => out.push(DocxOp::SetCellBorders(parse_set_cell_borders(obj, i)?)),
            "setCellFormat" => out.push(DocxOp::SetCellFormat(parse_set_cell_format(obj, i)?)),
            "setParaFormat" => out.push(DocxOp::SetParaFormat(parse_set_para_format(obj, i)?)),
            "setRunFormat" => out.push(DocxOp::SetRunFormat(parse_set_run_format(obj, i)?)),
            "setTextBoxLink" => out.push(DocxOp::SetTextBoxLink(parse_set_txbx_link(obj, i)?)),
            "insertFootnote" => out.push(DocxOp::InsertFootnote(parse_insert_note(
                obj,
                i,
                NoteKind::Footnote,
            )?)),
            "insertEndnote" => out.push(DocxOp::InsertEndnote(parse_insert_note(
                obj,
                i,
                NoteKind::Endnote,
            )?)),
            "deleteFootnote" => out.push(DocxOp::DeleteFootnote(parse_delete_note(
                obj,
                i,
                NoteKind::Footnote,
            )?)),
            "deleteEndnote" => out.push(DocxOp::DeleteEndnote(parse_delete_note(
                obj,
                i,
                NoteKind::Endnote,
            )?)),
            "setNoteNumbering" => {
                out.push(DocxOp::SetNoteNumbering(parse_set_note_numbering(obj, i)?))
            }
            "insertComment" => out.push(DocxOp::InsertComment(parse_insert_comment(obj, i)?)),
            "replyComment" => out.push(DocxOp::ReplyComment(parse_reply_comment(obj, i)?)),
            "deleteComment" => out.push(DocxOp::DeleteComment(parse_delete_comment(obj, i)?)),
            "setCommentText" => out.push(DocxOp::SetCommentText(parse_set_comment_text(obj, i)?)),
            "setCommentMeta" => out.push(DocxOp::SetCommentMeta(parse_set_comment_meta(obj, i)?)),
            "setCommentDone" => out.push(DocxOp::SetCommentDone(parse_set_comment_done(obj, i)?)),
            "insertChart" => out.push(DocxOp::InsertChart(parse_insert_chart(obj, i)?)),
            "deleteChart" => out.push(DocxOp::DeleteChart(parse_delete_chart(obj, i)?)),
            "setChartTitle" => out.push(DocxOp::SetChartTitle(parse_set_chart_title(obj, i)?)),
            "setChartSeries" => out.push(DocxOp::SetChartSeries(parse_set_chart_series(obj, i)?)),
            "addChartSeries" => out.push(DocxOp::AddChartSeries(parse_add_chart_series(obj, i)?)),
            "deleteChartSeries" => out.push(DocxOp::DeleteChartSeries(parse_delete_chart_series(
                obj, i,
            )?)),
            "setChartType" => out.push(DocxOp::SetChartType(parse_set_chart_type(obj, i)?)),
            "setChartAxisTitle" => out.push(DocxOp::SetChartAxisTitle(parse_set_chart_axis_title(
                obj, i,
            )?)),
            "setChartLegend" => out.push(DocxOp::SetChartLegend(parse_set_chart_legend(obj, i)?)),
            "setChartDataLabels" => out.push(DocxOp::SetChartDataLabels(
                parse_set_chart_data_labels(obj, i)?,
            )),
            "setChartSize" => out.push(DocxOp::SetChartSize(parse_set_chart_size(obj, i)?)),
            "setChartWrap" => out.push(DocxOp::SetChartWrap(parse_set_chart_wrap(obj, i)?)),
            "insertTrendline" => out.push(DocxOp::InsertTrendline(parse_insert_trendline(obj, i)?)),
            "setTrendline" => out.push(DocxOp::SetTrendline(parse_set_trendline(obj, i)?)),
            "deleteTrendline" => out.push(DocxOp::DeleteTrendline(parse_delete_trendline(obj, i)?)),
            "insertErrorBars" => {
                out.push(DocxOp::InsertErrorBars(parse_insert_error_bars(obj, i)?))
            }
            "setErrorBars" => out.push(DocxOp::SetErrorBars(parse_set_error_bars(obj, i)?)),
            "deleteErrorBars" => {
                out.push(DocxOp::DeleteErrorBars(parse_delete_error_bars(obj, i)?))
            }
            "setChartSeriesAxis" => {
                out.push(DocxOp::SetChartSeriesAxis(parse_set_series_axis(obj, i)?))
            }
            "insertSmartArt" => out.push(DocxOp::InsertSmartArt(parse_insert_smartart(obj, i)?)),
            "deleteSmartArt" => out.push(DocxOp::DeleteSmartArt(parse_delete_smartart(obj, i)?)),
            "setSmartArtText" => {
                out.push(DocxOp::SetSmartArtText(parse_set_smartart_text(obj, i)?))
            }
            "addSmartArtPoint" => {
                out.push(DocxOp::AddSmartArtPoint(parse_add_smartart_point(obj, i)?))
            }
            "deleteSmartArtPoint" => out.push(DocxOp::DeleteSmartArtPoint(
                parse_delete_smartart_point(obj, i)?,
            )),
            "setSmartArtSize" => {
                out.push(DocxOp::SetSmartArtSize(parse_set_smartart_size(obj, i)?))
            }
            "setSmartArtWrap" => {
                out.push(DocxOp::SetSmartArtWrap(parse_set_smartart_wrap(obj, i)?))
            }
            "setSmartArtColors" => out.push(DocxOp::SetSmartArtColors(parse_set_smartart_colors(
                obj, i,
            )?)),
            "setSmartArtStyle" => {
                out.push(DocxOp::SetSmartArtStyle(parse_set_smartart_style(obj, i)?))
            }
            "setSmartArtLayout" => out.push(DocxOp::SetSmartArtLayout(parse_set_smartart_layout(
                obj, i,
            )?)),
            "insertOle" => out.push(DocxOp::InsertOle(parse_insert_ole(obj, i)?)),
            "deleteOle" => out.push(DocxOp::DeleteOle(parse_delete_ole(obj, i)?)),
            "insertInkCanvas" => {
                out.push(DocxOp::InsertInkCanvas(parse_insert_ink_canvas(obj, i)?))
            }
            "replaceInkCanvas" => {
                out.push(DocxOp::ReplaceInkCanvas(parse_replace_ink_canvas(obj, i)?))
            }
            "deleteInkCanvas" => {
                out.push(DocxOp::DeleteInkCanvas(parse_delete_ink_canvas(obj, i)?))
            }
            "setInkCanvasSize" => {
                out.push(DocxOp::SetInkCanvasSize(parse_set_ink_canvas_size(obj, i)?))
            }
            "setInkCanvasWrap" => {
                out.push(DocxOp::SetInkCanvasWrap(parse_set_ink_canvas_wrap(obj, i)?))
            }
            "insertModel3d" => out.push(DocxOp::InsertModel3d(parse_insert_model3d(obj, i)?)),
            "replaceModel3d" => out.push(DocxOp::ReplaceModel3d(parse_replace_model3d(obj, i)?)),
            "deleteModel3d" => out.push(DocxOp::DeleteModel3d(parse_delete_model3d(obj, i)?)),
            "setModel3dSize" => out.push(DocxOp::SetModel3dSize(parse_set_model3d_size(obj, i)?)),
            "setModel3dWrap" => out.push(DocxOp::SetModel3dWrap(parse_set_model3d_wrap(obj, i)?)),
            "insertWatermark" => out.push(DocxOp::InsertWatermark(super::watermark::parse_insert(
                obj, i,
            )?)),
            "deleteWatermark" => out.push(DocxOp::DeleteWatermark(super::watermark::parse_delete(
                obj, i,
            )?)),
            "setProtection" => out.push(DocxOp::SetProtection(super::protect::parse_op(obj, i)?)),
            "setThemeColors" => out.push(DocxOp::SetThemeColors(super::theme::parse_set_colors(
                obj, i,
            )?)),
            "setThemeFonts" => out.push(DocxOp::SetThemeFonts(super::theme::parse_set_fonts(
                obj, i,
            )?)),
            "setThemeFmt" => out.push(DocxOp::SetThemeFmt(super::theme::parse_set_fmt(obj, i)?)),
            "setThemeExtraSchemes" => out.push(DocxOp::SetThemeExtraSchemes(
                super::theme::parse_set_extra_schemes(obj, i)?,
            )),
            "setThemeObjectDefaults" => out.push(DocxOp::SetThemeObjectDefaults(
                super::theme::parse_set_object_defaults(obj, i)?,
            )),
            "setThemeOverride" => out.push(DocxOp::SetThemeOverride(
                super::theme::parse_set_override(obj, i)?,
            )),
            "setProps" => out.push(DocxOp::SetProps(crate::props::parse_set_props(
                obj,
                i,
                "docx:document",
            )?)),
            "setCustomProp" => out.push(DocxOp::SetCustomProp(
                crate::props::parse_set_custom_prop(obj, i, "docx:document")?,
            )),
            "deleteCustomProp" => out.push(DocxOp::DeleteCustomProp(
                crate::props::parse_delete_custom_prop(obj, i)?,
            )),
            _ => {
                return Err(Error::with_path(
                    ErrorKind::UnsupportedOp {
                        op: op_name.to_string(),
                    },
                    format!("ops[{i}].op"),
                ));
            }
        }
    }
    Ok(out)
}
