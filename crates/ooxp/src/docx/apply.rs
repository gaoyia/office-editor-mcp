//! 按 id 改段落文本、节上距边界、页眉页脚是否链接前节，或 DrawingML 图。

use std::collections::{HashMap, HashSet};

use crate::error::{Error, ErrorKind};
use crate::opc::{self, rels_part_name, ContentTypes, Package, Relationship, Relationships};
use crate::xml::{self, Document, Element, Node};

use super::biblio;
use super::charts;
use super::comments;
use super::connector;
use super::drawing_hlink;
use super::east_asia;
use super::field;
use super::font_embed;
use super::format;
use super::glossary;
use super::group::{self, GroupHost, GroupMember};
use super::hf::{self, HfKind};
use super::ink;
use super::link;
use super::math;
use super::model3d;
use super::notes;
use super::numbering;
use super::ole;
use super::page_bg;
use super::para::{self, BodyPara, HostPara, NoteKind, ParaTarget};
use super::picture;
use super::protect;
use super::request::{
    DeleteBibliographyOp, DeleteBookmarkOp, DeleteCaptionOp, DeleteCitationOp, DeleteColOp,
    DeleteCommentOp, DeleteConnectorOp, DeleteContentControlOp, DeleteCrossRefOp, DeleteDocPartOp,
    DeleteEditRangeOp, DeleteFieldOp, DeleteHyperlinkOp, DeleteIndexEntryOp, DeleteIndexOp,
    DeleteMathOp, DeleteNoteOp, DeleteParaOp, DeletePictureOp, DeleteRowOp, DeleteRubyOp,
    DeleteSectionOp, DeleteShapeOp, DeleteSourceOp, DeleteStyleOp, DeleteTableOp, DeleteTextBoxOp,
    DeleteTocOp, DocxOp, GroupShapesOp, HyperlinkTarget, InsertBibliographyOp,
    InsertBibliographyPlace, InsertBookmarkOp, InsertCaptionOp, InsertCaptionPlace,
    InsertCitationOp, InsertColOp, InsertCommentOp, InsertConnectorOp, InsertContentControlOp,
    InsertCrossRefOp, InsertDocPartContentOp, InsertDocPartContentPlace, InsertDocPartOp,
    InsertEditRangeOp, InsertFieldOp, InsertHeaderFooterOp, InsertHyperlinkHost, InsertHyperlinkOp,
    InsertIndexEntryOp, InsertIndexOp, InsertIndexPlace, InsertMathOp, InsertMathPlace,
    InsertNoteOp, InsertNumberingOp, InsertParaOp, InsertParaPlace, InsertPictureOp, InsertRowOp,
    InsertRubyOp, InsertRubyPlace, InsertSdtPlace, InsertSectionOp, InsertSectionPlace,
    InsertShapeOp, InsertSourceOp, InsertStyleOp, InsertTableOp, InsertTextBoxOp, InsertTocOp,
    InsertTocPlace, MergeCellsOp, NoteNumberTarget, ReplacePictureOp, ReplyCommentOp, RevisionOp,
    SetBookmarkOp, SetCellBordersOp, SetCellFormatOp, SetColumnsOp, SetCommentDoneOp,
    SetCommentMetaOp, SetCommentTextOp, SetContentControlOp, SetDocGridOp, SetEvenAndOddHeadersOp,
    SetFieldOp, SetFormProtOp, SetGutterAtTopOp, SetHfDistanceOp, SetHfLinkOp, SetHyperlinkOp,
    SetLineNumberingOp, SetMathOp, SetMirrorMarginsOp, SetNoteNumberingOp, SetNumberingOp,
    SetPageBordersOp, SetPageMarginOp, SetPageSizeOp, SetParaFormatOp, SetParaNumberingOp,
    SetParaStyleOp, SetParaTextOp, SetPictureSizeOp, SetPictureWrapOp, SetRtlGutterOp, SetRubyOp,
    SetRunFormatOp, SetRunStyleOp, SetShapeFillOp, SetShapeLineOp, SetShapePosOp, SetShapeSizeOp,
    SetStyleOp, SetTableAlignOp, SetTableBordersOp, SetTableCellMarOp, SetTableFloatOp,
    SetTableRowHeightOp, SetTableStyleOp, SetTableWidthOp, SetTextBoxDirectionOp, SetTextBoxFillOp,
    SetTextBoxLineOp, SetTextBoxLinkOp, SetTextBoxSizeOp, SetTextBoxTextAnchorOp, SetTextBoxWrapOp,
    SetTitlePageOp, SetVerticalAlignOp, SplitCellsOp, UngroupOp,
};
use super::revision;
use super::ruby;
use super::sdt;
use super::sect;
use super::shape;
use super::smartart;
use super::styles;
use super::tbl;
use super::textbox;
use super::watermark;
use super::{CT_FOOTER, CT_HEADER, REL_FOOTER, REL_HEADER, REL_HYPERLINK};

pub fn apply(bytes: &[u8], ops: &[DocxOp]) -> Result<Vec<u8>, Error> {
    let mut pkg = Package::open_bytes(bytes.to_vec())?;
    let doc_part = opc::office_document_part(&pkg)?;
    let doc_xml = xml::parse(pkg.get(&doc_part)?)?;
    let mut catalog = hf::catalog(&pkg, &doc_part, &doc_xml.root)?;

    // 按部件缓存已解析的文档；最后一并写回脏部件。
    let mut docs: HashMap<String, Document> = HashMap::new();
    docs.insert(doc_part.clone(), doc_xml);
    let mut dirty: HashMap<String, ()> = HashMap::new();
    let mut removed: HashSet<String> = HashSet::new();

    for (i, op) in ops.iter().enumerate() {
        match op {
            DocxOp::SetParaText(op) => {
                apply_para_text(&mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::SetHeaderFooterDistance(op) => {
                let doc = docs.get_mut(&doc_part).unwrap();
                apply_hf_distance(&mut doc.root, op, i)?;
                dirty.insert(doc_part.clone(), ());
            }
            DocxOp::SetHeaderFooterLink(op) => {
                apply_hf_link(
                    &mut pkg,
                    &mut docs,
                    &mut dirty,
                    &mut removed,
                    &doc_part,
                    op,
                    i,
                )?;
                refresh_hf_catalog(&pkg, &docs, &doc_part, &mut catalog)?;
            }
            DocxOp::InsertHeaderFooter(op) => {
                apply_insert_hf(&mut pkg, &mut docs, &mut dirty, &doc_part, op, i)?;
                refresh_hf_catalog(&pkg, &docs, &doc_part, &mut catalog)?;
            }
            DocxOp::InsertSection(op) => {
                apply_insert_section(&mut docs, &mut dirty, &doc_part, op, i)?;
                refresh_hf_catalog(&pkg, &docs, &doc_part, &mut catalog)?;
            }
            DocxOp::DeleteSection(op) => {
                apply_delete_section(
                    &mut pkg,
                    &mut docs,
                    &mut dirty,
                    &mut removed,
                    &doc_part,
                    op,
                    i,
                )?;
                refresh_hf_catalog(&pkg, &docs, &doc_part, &mut catalog)?;
            }
            DocxOp::InsertToc(op) => {
                apply_insert_toc(&mut docs, &mut dirty, &doc_part, op, i)?;
            }
            DocxOp::DeleteToc(op) => {
                apply_delete_toc(&mut docs, &mut dirty, &doc_part, op, i)?;
            }
            DocxOp::InsertCaption(op) => {
                apply_insert_caption(&mut docs, &mut dirty, &doc_part, op, i)?;
            }
            DocxOp::DeleteCaption(op) => {
                apply_delete_caption(&mut docs, &mut dirty, &doc_part, op, i)?;
            }
            DocxOp::InsertCrossRef(op) => {
                apply_insert_cross_ref(&pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::DeleteCrossRef(op) => {
                apply_delete_cross_ref(&mut docs, &mut dirty, &doc_part, op, i)?;
            }
            DocxOp::InsertIndex(op) => {
                apply_insert_index(&mut docs, &mut dirty, &doc_part, op, i)?;
            }
            DocxOp::DeleteIndex(op) => {
                apply_delete_index(&mut docs, &mut dirty, &doc_part, op, i)?;
            }
            DocxOp::InsertIndexEntry(op) => {
                apply_insert_index_entry(&pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::DeleteIndexEntry(op) => {
                apply_delete_index_entry(&mut docs, &mut dirty, &doc_part, op, i)?;
            }
            DocxOp::InsertSource(op) => {
                apply_insert_source(&mut pkg, &mut docs, &mut dirty, &doc_part, op, i)?;
            }
            DocxOp::DeleteSource(op) => {
                apply_delete_source(&mut pkg, &mut docs, &mut dirty, &doc_part, op, i)?;
            }
            DocxOp::InsertCitation(op) => {
                apply_insert_citation(&pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::DeleteCitation(op) => {
                apply_delete_citation(&mut docs, &mut dirty, &doc_part, op, i)?;
            }
            DocxOp::InsertBibliography(op) => {
                apply_insert_bibliography(&mut docs, &mut dirty, &doc_part, op, i)?;
            }
            DocxOp::DeleteBibliography(op) => {
                apply_delete_bibliography(&mut docs, &mut dirty, &doc_part, op, i)?;
            }
            DocxOp::SetBibliographyStyle(op) => {
                biblio::set_style(
                    &mut pkg,
                    &mut docs,
                    &mut dirty,
                    &doc_part,
                    &op.style_name,
                    i,
                )?;
            }
            DocxOp::InsertDocPart(op) => {
                apply_insert_doc_part(&mut pkg, &mut docs, &mut dirty, &doc_part, op, i)?;
            }
            DocxOp::DeleteDocPart(op) => {
                apply_delete_doc_part(
                    &mut pkg,
                    &mut docs,
                    &mut dirty,
                    &mut removed,
                    &doc_part,
                    op,
                    i,
                )?;
            }
            DocxOp::InsertDocPartContent(op) => {
                apply_insert_doc_part_content(&pkg, &mut docs, &mut dirty, &doc_part, op, i)?;
            }
            DocxOp::InsertEmbeddedFont(op) => {
                font_embed::insert(
                    &mut pkg, &mut docs, &mut dirty, &doc_part, &op.name, op.style, &op.file, i,
                )?;
            }
            DocxOp::DeleteEmbeddedFont(op) => {
                font_embed::delete(&mut pkg, &mut docs, &mut dirty, &doc_part, op.n, &op.id, i)?;
            }
            DocxOp::SetFontEmbed(op) => {
                font_embed::set_flags(
                    &mut pkg,
                    &mut docs,
                    &mut dirty,
                    &doc_part,
                    op.true_type,
                    op.system_fonts,
                    op.subset,
                )?;
            }
            DocxOp::SetPageBackground(op) => {
                page_bg::set_background(
                    &mut pkg,
                    &mut docs,
                    &mut dirty,
                    &doc_part,
                    op.color.as_deref(),
                    op.fill.as_ref(),
                )?;
            }
            DocxOp::SetPageBorders(op) => {
                apply_page_borders(&mut docs, &mut dirty, &doc_part, op, i)?;
            }
            DocxOp::SetFormProt(op) => {
                apply_set_form_prot(&mut docs, &mut dirty, &doc_part, op, i)?;
            }
            DocxOp::InsertMath(op) => {
                apply_insert_math(&pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::SetMath(op) => {
                apply_set_math(&pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::DeleteMath(op) => {
                apply_delete_math(&pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::InsertRuby(op) => {
                apply_insert_ruby(&pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::SetRuby(op) => {
                apply_set_ruby(&pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::DeleteRuby(op) => {
                apply_delete_ruby(&pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::SetEastAsianSettings(op) => {
                east_asia::apply_settings(
                    &mut pkg,
                    &mut docs,
                    &mut dirty,
                    &doc_part,
                    east_asia::SettingsPatch {
                        character_spacing: op.character_spacing.as_ref().map(|v| v.as_deref()),
                        strict: op.strict,
                        no_before: op.no_before.as_ref().map(|v| v.as_ref()),
                        no_after: op.no_after.as_ref().map(|v| v.as_ref()),
                        theme_font_lang: op.theme_font_lang.as_ref().map(|v| v.as_ref()),
                    },
                )?;
            }
            DocxOp::InsertContentControl(op) => {
                apply_insert_sdt(&mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::SetContentControl(op) => {
                apply_set_sdt(&mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::DeleteContentControl(op) => {
                apply_delete_sdt(&pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::SetTitlePage(op) => {
                apply_title_page(&mut docs, &mut dirty, &doc_part, op, i)?;
                refresh_hf_catalog(&pkg, &docs, &doc_part, &mut catalog)?;
            }
            DocxOp::SetEvenAndOddHeaders(op) => {
                apply_even_and_odd(&mut pkg, &mut docs, &mut dirty, &doc_part, op, i)?;
                refresh_hf_catalog(&pkg, &docs, &doc_part, &mut catalog)?;
            }
            DocxOp::SetPageMargin(op) => {
                apply_page_margin(&mut docs, &mut dirty, &doc_part, op, i)?;
            }
            DocxOp::SetPageSize(op) => {
                apply_page_size(&mut docs, &mut dirty, &doc_part, op, i)?;
            }
            DocxOp::SetColumns(op) => {
                apply_columns(&mut docs, &mut dirty, &doc_part, op, i)?;
            }
            DocxOp::SetVerticalAlign(op) => {
                apply_valign(&mut docs, &mut dirty, &doc_part, op, i)?;
            }
            DocxOp::SetLineNumbering(op) => {
                apply_line_numbering(&mut docs, &mut dirty, &doc_part, op, i)?;
            }
            DocxOp::SetDocGrid(op) => {
                apply_doc_grid(&mut docs, &mut dirty, &doc_part, op, i)?;
            }
            DocxOp::SetRtlGutter(op) => {
                apply_rtl_gutter(&mut docs, &mut dirty, &doc_part, op, i)?;
            }
            DocxOp::SetMirrorMargins(op) => {
                apply_mirror_margins(&mut pkg, &mut docs, &mut dirty, &doc_part, op, i)?;
            }
            DocxOp::SetGutterAtTop(op) => {
                apply_gutter_at_top(&mut pkg, &mut docs, &mut dirty, &doc_part, op, i)?;
            }
            DocxOp::ReplacePicture(op) => {
                apply_replace_pic(&mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::InsertPicture(op) => {
                apply_insert_pic(&mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::DeletePicture(op) => {
                apply_delete_pic(&mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::SetPictureSize(op) => {
                apply_pic_size(&mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::SetPictureWrap(op) => {
                apply_pic_wrap(&mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::InsertTextBox(op) => {
                apply_insert_txbx(&mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::DeleteTextBox(op) => {
                apply_delete_txbx(&mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::SetTextBoxSize(op) => {
                apply_txbx_size(&mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::SetTextBoxWrap(op) => {
                apply_txbx_wrap(&mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::SetTextBoxFill(op) => {
                apply_txbx_fill(&mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::SetTextBoxLine(op) => {
                apply_txbx_line(&mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::SetTextBoxDirection(op) => {
                apply_txbx_dir(&mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::SetTextBoxTextAnchor(op) => {
                apply_txbx_text_anchor(
                    &mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::InsertConnector(op) => {
                apply_insert_connector(
                    &mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::DeleteConnector(op) => {
                apply_delete_connector(
                    &mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::InsertShape(op) => {
                apply_insert_shape(&mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::DeleteShape(op) => {
                apply_delete_shape(&mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::SetShapeSize(op) => {
                apply_shape_size(&mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::SetShapePos(op) => {
                apply_shape_pos(&mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::SetShapeFill(op) => {
                apply_shape_fill(&mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::SetShapeLine(op) => {
                apply_shape_line(&mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::GroupShapes(op) => {
                apply_group_shapes(&mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::Ungroup(op) => {
                apply_ungroup(&mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::InsertPara(op) => {
                apply_insert_para(&mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::DeletePara(op) => {
                apply_delete_para(&mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::SetTextBoxLink(op) => {
                apply_txbx_link(&mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::InsertFootnote(op) => {
                apply_insert_note(
                    &mut pkg,
                    &mut docs,
                    &mut dirty,
                    &doc_part,
                    &catalog,
                    op,
                    NoteKind::Footnote,
                    i,
                )?;
            }
            DocxOp::InsertEndnote(op) => {
                apply_insert_note(
                    &mut pkg,
                    &mut docs,
                    &mut dirty,
                    &doc_part,
                    &catalog,
                    op,
                    NoteKind::Endnote,
                    i,
                )?;
            }
            DocxOp::DeleteFootnote(op) => {
                apply_delete_note(&mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::DeleteEndnote(op) => {
                apply_delete_note(&mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::SetNoteNumbering(op) => {
                apply_note_numbering(&mut pkg, &mut docs, &mut dirty, &doc_part, op, i)?;
            }
            DocxOp::InsertComment(op) => {
                apply_insert_comment(&mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::ReplyComment(op) => {
                apply_reply_comment(&mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::DeleteComment(op) => {
                apply_delete_comment(&mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::SetCommentText(op) => {
                apply_set_comment_text(&mut pkg, &mut docs, &mut dirty, &doc_part, op, i)?;
            }
            DocxOp::SetCommentMeta(op) => {
                apply_set_comment_meta(&mut pkg, &mut docs, &mut dirty, &doc_part, op, i)?;
            }
            DocxOp::SetCommentDone(op) => {
                apply_set_comment_done(&mut pkg, &mut docs, &mut dirty, &doc_part, op, i)?;
            }
            DocxOp::InsertChart(op) => {
                charts::apply_insert(&mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::DeleteChart(op) => {
                charts::apply_delete(&mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::SetChartTitle(op) => {
                charts::apply_title(&mut pkg, &mut docs, &doc_part, &catalog, op, i)?;
            }
            DocxOp::SetChartSeries(op) => {
                charts::apply_series(&mut pkg, &mut docs, &doc_part, &catalog, op, i)?;
            }
            DocxOp::AddChartSeries(op) => {
                charts::apply_add_series(&mut pkg, &mut docs, &doc_part, &catalog, op, i)?;
            }
            DocxOp::DeleteChartSeries(op) => {
                charts::apply_delete_series(&mut pkg, &mut docs, &doc_part, &catalog, op, i)?;
            }
            DocxOp::SetChartType(op) => {
                charts::apply_type(&mut pkg, &mut docs, &doc_part, &catalog, op, i)?;
            }
            DocxOp::SetChartAxisTitle(op) => {
                charts::apply_axis_title(&mut pkg, &mut docs, &doc_part, &catalog, op, i)?;
            }
            DocxOp::SetChartLegend(op) => {
                charts::apply_legend(&mut pkg, &mut docs, &doc_part, &catalog, op, i)?;
            }
            DocxOp::SetChartDataLabels(op) => {
                charts::apply_data_labels(&mut pkg, &mut docs, &doc_part, &catalog, op, i)?;
            }
            DocxOp::SetChartSize(op) => {
                charts::apply_size(&mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::SetChartWrap(op) => {
                charts::apply_wrap(&mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::InsertTrendline(op) => {
                charts::apply_insert_trendline(&mut pkg, &mut docs, &doc_part, &catalog, op, i)?;
            }
            DocxOp::SetTrendline(op) => {
                charts::apply_set_trendline(&mut pkg, &mut docs, &doc_part, &catalog, op, i)?;
            }
            DocxOp::DeleteTrendline(op) => {
                charts::apply_delete_trendline(&mut pkg, &mut docs, &doc_part, &catalog, op, i)?;
            }
            DocxOp::InsertErrorBars(op) => {
                charts::apply_insert_error_bars(&mut pkg, &mut docs, &doc_part, &catalog, op, i)?;
            }
            DocxOp::SetErrorBars(op) => {
                charts::apply_set_error_bars(&mut pkg, &mut docs, &doc_part, &catalog, op, i)?;
            }
            DocxOp::DeleteErrorBars(op) => {
                charts::apply_delete_error_bars(&mut pkg, &mut docs, &doc_part, &catalog, op, i)?;
            }
            DocxOp::SetChartSeriesAxis(op) => {
                charts::apply_series_axis(&mut pkg, &mut docs, &doc_part, &catalog, op, i)?;
            }
            DocxOp::InsertSmartArt(op) => {
                smartart::apply_insert(
                    &mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::DeleteSmartArt(op) => {
                smartart::apply_delete(
                    &mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::SetSmartArtText(op) => {
                smartart::apply_text(&mut pkg, &mut docs, &doc_part, &catalog, op, i)?;
            }
            DocxOp::AddSmartArtPoint(op) => {
                smartart::apply_add_point(&mut pkg, &mut docs, &doc_part, &catalog, op, i)?;
            }
            DocxOp::DeleteSmartArtPoint(op) => {
                smartart::apply_delete_point(&mut pkg, &mut docs, &doc_part, &catalog, op, i)?;
            }
            DocxOp::SetSmartArtSize(op) => {
                smartart::apply_size(&mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::SetSmartArtWrap(op) => {
                smartart::apply_wrap(&mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::SetSmartArtColors(op) => {
                smartart::apply_colors(
                    &mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::SetSmartArtStyle(op) => {
                smartart::apply_style(&mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::SetSmartArtLayout(op) => {
                smartart::apply_layout(
                    &mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::InsertOle(op) => {
                ole::apply_insert(&mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::DeleteOle(op) => {
                ole::apply_delete(&mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::SetOleSize(op) => {
                ole::apply_set_size(&mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::InsertInkCanvas(op) => {
                ink::apply_insert(&mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::ReplaceInkCanvas(op) => {
                ink::apply_replace(&mut pkg, &mut docs, &doc_part, &catalog, op, i)?;
            }
            DocxOp::DeleteInkCanvas(op) => {
                ink::apply_delete(&mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::SetInkCanvasSize(op) => {
                ink::apply_set_size(&mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::SetInkCanvasWrap(op) => {
                ink::apply_set_wrap(&mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::InsertModel3d(op) => {
                super::model3d::apply_insert(
                    &mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::ReplaceModel3d(op) => {
                super::model3d::apply_replace(&mut pkg, &mut docs, &doc_part, &catalog, op, i)?;
            }
            DocxOp::DeleteModel3d(op) => {
                super::model3d::apply_delete(
                    &mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::SetModel3dSize(op) => {
                super::model3d::apply_set_size(
                    &mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::SetModel3dWrap(op) => {
                super::model3d::apply_set_wrap(
                    &mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::SetModel3dCamera(op) => {
                super::model3d::apply_set_camera(
                    &mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::InsertWatermark(op) => {
                watermark::apply_insert(&mut pkg, &mut docs, &mut dirty, &doc_part, op, i)?;
                refresh_hf_catalog(&pkg, &docs, &doc_part, &mut catalog)?;
            }
            DocxOp::DeleteWatermark(op) => {
                watermark::apply_delete(&mut pkg, &mut docs, &mut dirty, &doc_part, op, i)?;
            }
            DocxOp::SetProtection(op) => {
                protect::apply(&mut pkg, &mut docs, &mut dirty, &doc_part, op, i)?;
            }
            DocxOp::SetThemeColors(op) => {
                super::theme::set_colors(&mut pkg, &doc_part, op, i)?;
            }
            DocxOp::SetThemeFonts(op) => {
                super::theme::set_fonts(&mut pkg, &doc_part, op, i)?;
            }
            DocxOp::SetThemeFmt(op) => {
                super::theme::set_fmt(&mut pkg, &doc_part, op, i)?;
            }
            DocxOp::SetThemeExtraSchemes(op) => {
                super::theme::set_extra_schemes(&mut pkg, &doc_part, op, i)?;
            }
            DocxOp::SetThemeObjectDefaults(op) => {
                super::theme::set_object_defaults(&mut pkg, &doc_part, op, i)?;
            }
            DocxOp::SetThemeOverride(op) => {
                super::theme::set_override(&mut pkg, &doc_part, op, i)?;
            }
            DocxOp::SetProps(op) => {
                crate::props::apply_set_props(&mut pkg, op, i)?;
            }
            DocxOp::SetCustomProp(op) => {
                crate::props::apply_set_custom_prop(&mut pkg, op, i)?;
            }
            DocxOp::DeleteCustomProp(op) => {
                crate::props::apply_delete_custom_prop(&mut pkg, op, i)?;
            }
            DocxOp::InsertTable(op) => {
                apply_insert_table(&pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::DeleteTable(op) => {
                apply_delete_table(&pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::InsertRow(op) => {
                apply_insert_row(&pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::DeleteRow(op) => {
                apply_delete_row(&pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::InsertCol(op) => {
                apply_insert_col(&pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::DeleteCol(op) => {
                apply_delete_col(&pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::MergeCells(op) => {
                apply_merge_cells(&pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::SplitCells(op) => {
                apply_split_cells(&pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::AcceptRevision(op) => {
                apply_rev(
                    &pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, true, i,
                )?;
            }
            DocxOp::RejectRevision(op) => {
                apply_rev(
                    &pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, false, i,
                )?;
            }
            DocxOp::InsertHyperlink(op) => {
                apply_insert_hyperlink(
                    &mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::SetHyperlink(op) => {
                apply_set_hyperlink(&mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::DeleteHyperlink(op) => {
                apply_delete_hyperlink(
                    &mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::InsertBookmark(op) => {
                apply_insert_bookmark(&pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::SetBookmark(op) => {
                apply_set_bookmark(&pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::DeleteBookmark(op) => {
                apply_delete_bookmark(&pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::InsertEditRange(op) => {
                apply_insert_edit_range(&pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::DeleteEditRange(op) => {
                apply_delete_edit_range(&pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::InsertField(op) => {
                apply_insert_field(&pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::SetField(op) => {
                apply_set_field(&pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::DeleteField(op) => {
                apply_delete_field(&pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::InsertNumbering(op) => {
                apply_insert_numbering(&mut pkg, &mut docs, &mut dirty, &doc_part, op)?;
            }
            DocxOp::SetNumbering(op) => {
                apply_set_numbering(&pkg, &mut docs, &mut dirty, &doc_part, op, i)?;
            }
            DocxOp::SetParaNumbering(op) => {
                apply_set_para_numbering(&pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::SetStyle(op) => {
                apply_set_style(&pkg, &mut docs, &mut dirty, &doc_part, op, i)?;
            }
            DocxOp::SetParaStyle(op) => {
                apply_set_para_style(&pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::InsertStyle(op) => {
                apply_insert_style(&mut pkg, &mut docs, &mut dirty, &doc_part, op, i)?;
            }
            DocxOp::DeleteStyle(op) => {
                apply_delete_style(&pkg, &mut docs, &mut dirty, &doc_part, op, i)?;
            }
            DocxOp::SetRunStyle(op) => {
                apply_set_run_style(&pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::SetTableStyle(op) => {
                apply_set_table_style(&pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::SetTableFloat(op) => {
                apply_set_table_float(&pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::SetTableWidth(op) => {
                apply_set_table_width(&pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::SetTableAlign(op) => {
                apply_set_table_align(&pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::SetTableRowHeight(op) => {
                apply_set_table_row_height(
                    &pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::SetTableCellMar(op) => {
                apply_set_table_cell_mar(&pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::SetTableBorders(op) => {
                apply_set_table_borders(&pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::SetCellBorders(op) => {
                apply_set_cell_borders(&pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::SetCellFormat(op) => {
                apply_set_cell_format(&pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::SetParaFormat(op) => {
                apply_set_para_format(&pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::SetRunFormat(op) => {
                apply_set_run_format(&pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
        }
    }

    for name in dirty.keys() {
        if removed.contains(name) {
            continue;
        }
        let doc = docs.remove(name).unwrap();
        pkg.set(name, doc.to_bytes()?)?;
    }

    pkg.save_to_vec()
}

fn refuse_set_text_if_revision(el: &Element, i: usize) -> Result<(), Error> {
    if revision::para_blocks_set_text(el) {
        return Err(Error::with_path(
            ErrorKind::SetParaTextHasRevision,
            format!("ops[{i}].id"),
        ));
    }
    Ok(())
}

fn refuse_set_text_if_hyperlink(el: &Element, i: usize) -> Result<(), Error> {
    if link::para_blocks_set_text(el) {
        return Err(Error::with_path(
            ErrorKind::SetParaTextHasHyperlink,
            format!("ops[{i}].id"),
        ));
    }
    Ok(())
}

fn refuse_set_text_if_field(root: &Element, para: &para::BodyPara, i: usize) -> Result<(), Error> {
    if field::para_blocks_set_text(root, para) {
        return Err(Error::with_path(
            ErrorKind::SetParaTextHasField,
            format!("ops[{i}].id"),
        ));
    }
    Ok(())
}

fn refuse_set_text_if_math(root: &Element, para: &para::BodyPara, i: usize) -> Result<(), Error> {
    if math::para_blocks_set_text(root, para) {
        return Err(Error::with_path(
            ErrorKind::SetParaTextHasMath,
            format!("ops[{i}].id"),
        ));
    }
    Ok(())
}

fn refuse_set_text_if_ruby(root: &Element, para: &para::BodyPara, i: usize) -> Result<(), Error> {
    if ruby::para_blocks_set_text(root, para) {
        return Err(Error::with_path(
            ErrorKind::SetParaTextHasRuby,
            format!("ops[{i}].id"),
        ));
    }
    Ok(())
}

fn refuse_set_text_if_sdt(root: &Element, para: &para::BodyPara, i: usize) -> Result<(), Error> {
    if sdt::para_blocks_set_text(root, para) {
        return Err(Error::with_path(
            ErrorKind::SetParaTextHasSdt,
            format!("ops[{i}].id"),
        ));
    }
    Ok(())
}

fn apply_rev(
    pkg: &Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &RevisionOp,
    accept: bool,
    i: usize,
) -> Result<(), Error> {
    let part_name = revision::part_for_story(catalog, doc_part, &op.target.story, &op.id, i)?;
    let doc = load_part(pkg, docs, &part_name)?;
    revision::apply_revision(&mut doc.root, op.target.n, accept, &op.id, i)?;
    dirty.insert(part_name, ());
    Ok(())
}

fn push_hyperlink_rel(rels: &mut Relationships, url: &str) -> String {
    let id = rels.next_rid();
    rels.push(Relationship {
        id: id.clone(),
        rel_type: REL_HYPERLINK.to_string(),
        target: url.to_string(),
        target_mode: Some("External".into()),
        extra: Vec::new(),
    });
    id
}

fn drop_rel_if_unused(
    pkg: &mut Package,
    root: &Element,
    part: &str,
    rid: &str,
) -> Result<(), Error> {
    if link::rid_used(root, rid) {
        return Ok(());
    }
    let mut rels = Relationships::from_package(pkg, part)?;
    rels.remove_by_id(rid);
    rels.write_to(pkg, part)
}

fn apply_insert_hyperlink(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &InsertHyperlinkOp,
    i: usize,
) -> Result<(), Error> {
    match &op.host {
        InsertHyperlinkHost::Para { para, text } => apply_insert_para_hyperlink(
            pkg,
            docs,
            dirty,
            doc_part,
            catalog,
            &op.id,
            para,
            text,
            op.url.as_deref(),
            op.anchor.as_deref(),
            op.tooltip.as_deref(),
            i,
        ),
        InsertHyperlinkHost::Picture(target) => apply_insert_pic_hlink(
            pkg,
            docs,
            dirty,
            doc_part,
            catalog,
            &op.id,
            target,
            op.url.as_deref(),
            op.anchor.as_deref(),
            op.tooltip.as_deref(),
            i,
        ),
        InsertHyperlinkHost::TextBox(target) => apply_insert_txbx_hlink(
            pkg,
            docs,
            dirty,
            doc_part,
            catalog,
            &op.id,
            target,
            op.url.as_deref(),
            op.anchor.as_deref(),
            op.tooltip.as_deref(),
            i,
        ),
        InsertHyperlinkHost::Shape(target) => apply_insert_wsp_hlink(
            pkg,
            docs,
            dirty,
            doc_part,
            catalog,
            &op.id,
            target,
            op.url.as_deref(),
            op.anchor.as_deref(),
            op.tooltip.as_deref(),
            i,
            shape::has_click,
            shape::hlink_parent_mut,
            ErrorKind::ShapeNotFound { id: op.id.clone() },
        ),
        InsertHyperlinkHost::Connector(target) => apply_insert_wsp_hlink(
            pkg,
            docs,
            dirty,
            doc_part,
            catalog,
            &op.id,
            target,
            op.url.as_deref(),
            op.anchor.as_deref(),
            op.tooltip.as_deref(),
            i,
            connector::has_click,
            connector::hlink_parent_mut,
            ErrorKind::ConnectorNotFound { id: op.id.clone() },
        ),
        InsertHyperlinkHost::Ink(target) => apply_insert_ink_hlink(
            pkg,
            docs,
            dirty,
            doc_part,
            catalog,
            &op.id,
            target,
            op.url.as_deref(),
            op.anchor.as_deref(),
            op.tooltip.as_deref(),
            i,
        ),
        InsertHyperlinkHost::Model3d(target) => apply_insert_model3d_hlink(
            pkg,
            docs,
            dirty,
            doc_part,
            catalog,
            &op.id,
            target,
            op.url.as_deref(),
            op.anchor.as_deref(),
            op.tooltip.as_deref(),
            i,
        ),
        InsertHyperlinkHost::Chart(target) => apply_insert_chart_hlink(
            pkg,
            docs,
            dirty,
            doc_part,
            catalog,
            &op.id,
            target,
            op.url.as_deref(),
            op.anchor.as_deref(),
            op.tooltip.as_deref(),
            i,
        ),
        InsertHyperlinkHost::SmartArt(target) => apply_insert_smartart_hlink(
            pkg,
            docs,
            dirty,
            doc_part,
            catalog,
            &op.id,
            target,
            op.url.as_deref(),
            op.anchor.as_deref(),
            op.tooltip.as_deref(),
            i,
        ),
    }
}

fn apply_insert_para_hyperlink(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    id: &str,
    para: &ParaTarget,
    text: &str,
    url: Option<&str>,
    anchor: Option<&str>,
    tooltip: Option<&str>,
    i: usize,
) -> Result<(), Error> {
    let part_name = part_for_para(pkg, catalog, doc_part, para, id, i)?;
    {
        let doc = load_part(pkg, docs, &part_name)?;
        if find_insert_para_mut(&mut doc.root, para).is_none() {
            return Err(Error::with_path(
                ErrorKind::ParaNotFound { id: id.to_string() },
                format!("ops[{i}].id"),
            ));
        }
    }
    let rid = if let Some(url) = url {
        let mut rels = Relationships::from_package(pkg, &part_name)?;
        let rid = push_hyperlink_rel(&mut rels, url);
        rels.write_to(pkg, &part_name)?;
        Some(rid)
    } else {
        None
    };
    let doc = load_part(pkg, docs, &part_name)?;
    let el = find_insert_para_mut(&mut doc.root, para).ok_or_else(|| {
        Error::with_path(
            ErrorKind::ParaNotFound { id: id.to_string() },
            format!("ops[{i}].id"),
        )
    })?;
    link::insert_hyperlink_el(el, text, rid.as_deref(), anchor, tooltip);
    dirty.insert(part_name, ());
    Ok(())
}

fn apply_insert_pic_hlink(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    id: &str,
    target: &picture::PicTarget,
    url: Option<&str>,
    anchor: Option<&str>,
    tooltip: Option<&str>,
    i: usize,
) -> Result<(), Error> {
    let part_name = part_for_pic(pkg, catalog, doc_part, target, id, i)?;
    {
        let doc = load_part(pkg, docs, &part_name)?;
        let drawing = find_target_drawing_mut(&mut doc.root, target).ok_or_else(|| {
            Error::with_path(
                ErrorKind::PictureNotFound { id: id.to_string() },
                format!("ops[{i}].id"),
            )
        })?;
        if picture_hlink_exists(drawing, target) {
            return Err(Error::with_path(
                ErrorKind::ShapeAlreadyHasHyperlink,
                format!("ops[{i}].id"),
            ));
        }
    }
    let rid = {
        let mut rels = Relationships::from_package(pkg, &part_name)?;
        let rid = drawing_hlink::push_rel(&mut rels, url, anchor);
        rels.write_to(pkg, &part_name)?;
        rid
    };
    let doc = load_part(pkg, docs, &part_name)?;
    drawing_hlink::ensure_ns(&mut doc.root);
    let drawing = find_target_drawing_mut(&mut doc.root, target).ok_or_else(|| {
        Error::with_path(
            ErrorKind::PictureNotFound { id: id.to_string() },
            format!("ops[{i}].id"),
        )
    })?;
    let parent = picture_hlink_parent_mut(drawing, target).ok_or_else(|| {
        Error::with_path(
            ErrorKind::PictureNotFound { id: id.to_string() },
            format!("ops[{i}].id"),
        )
    })?;
    drawing_hlink::insert_click(parent, &rid, tooltip);
    dirty.insert(part_name, ());
    Ok(())
}

fn apply_insert_txbx_hlink(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    id: &str,
    target: &para::TxbxTarget,
    url: Option<&str>,
    anchor: Option<&str>,
    tooltip: Option<&str>,
    i: usize,
) -> Result<(), Error> {
    let part_name = part_for_txbx(pkg, catalog, doc_part, target, id, i)?;
    {
        let doc = load_part(pkg, docs, &part_name)?;
        let host = para::find_paragraph_mut(&mut doc.root, target.host.body())
            .ok_or_else(|| txbx_missing(id, i))?;
        let drawing = textbox::find_textbox_drawing_mut(host, &target.key)
            .ok_or_else(|| txbx_missing(id, i))?;
        if drawing_hlink::textbox_has_click(drawing) {
            return Err(Error::with_path(
                ErrorKind::ShapeAlreadyHasHyperlink,
                format!("ops[{i}].id"),
            ));
        }
    }
    let rid = {
        let mut rels = Relationships::from_package(pkg, &part_name)?;
        let rid = drawing_hlink::push_rel(&mut rels, url, anchor);
        rels.write_to(pkg, &part_name)?;
        rid
    };
    let doc = load_part(pkg, docs, &part_name)?;
    drawing_hlink::ensure_ns(&mut doc.root);
    let host = para::find_paragraph_mut(&mut doc.root, target.host.body())
        .ok_or_else(|| txbx_missing(id, i))?;
    let drawing =
        textbox::find_textbox_drawing_mut(host, &target.key).ok_or_else(|| txbx_missing(id, i))?;
    let parent =
        drawing_hlink::textbox_click_parent_mut(drawing).ok_or_else(|| txbx_missing(id, i))?;
    drawing_hlink::insert_click(parent, &rid, tooltip);
    dirty.insert(part_name, ());
    Ok(())
}

fn apply_insert_click_hlink(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    part_name: &str,
    id: &str,
    url: Option<&str>,
    anchor: Option<&str>,
    tooltip: Option<&str>,
    i: usize,
    mut has_click: impl FnMut(&mut Element) -> Option<bool>,
    mut parent_mut: impl FnMut(&mut Element) -> Option<&mut Element>,
    missing: ErrorKind,
) -> Result<(), Error> {
    {
        let doc = load_part(pkg, docs, part_name)?;
        match has_click(&mut doc.root) {
            Some(true) => {
                return Err(Error::with_path(
                    ErrorKind::ShapeAlreadyHasHyperlink,
                    format!("ops[{i}].id"),
                ));
            }
            Some(false) => {}
            None => {
                return Err(Error::with_path(missing, format!("ops[{i}].id")));
            }
        }
    }
    let rid = {
        let mut rels = Relationships::from_package(pkg, part_name)?;
        let rid = drawing_hlink::push_rel(&mut rels, url, anchor);
        rels.write_to(pkg, part_name)?;
        rid
    };
    let doc = load_part(pkg, docs, part_name)?;
    drawing_hlink::ensure_ns(&mut doc.root);
    let parent = parent_mut(&mut doc.root).ok_or_else(|| {
        Error::with_path(
            ErrorKind::HyperlinkNotFound { id: id.to_string() },
            format!("ops[{i}].id"),
        )
    })?;
    drawing_hlink::insert_click(parent, &rid, tooltip);
    dirty.insert(part_name.to_string(), ());
    Ok(())
}

fn apply_insert_wsp_hlink(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    id: &str,
    target: &para::TxbxTarget,
    url: Option<&str>,
    anchor: Option<&str>,
    tooltip: Option<&str>,
    i: usize,
    has_click: fn(&mut Element, &para::TxbxKey) -> Option<bool>,
    parent_mut: for<'a> fn(&'a mut Element, &para::TxbxKey) -> Option<&'a mut Element>,
    missing: ErrorKind,
) -> Result<(), Error> {
    let part_name = part_for_txbx(pkg, catalog, doc_part, target, id, i)?;
    let key = target.key.clone();
    apply_insert_click_hlink(
        pkg,
        docs,
        dirty,
        &part_name,
        id,
        url,
        anchor,
        tooltip,
        i,
        |root| {
            let host = para::find_paragraph_mut(root, target.host.body())?;
            has_click(host, &key)
        },
        |root| {
            let host = para::find_paragraph_mut(root, target.host.body())?;
            parent_mut(host, &key)
        },
        missing,
    )
}

fn apply_insert_ink_hlink(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    id: &str,
    target: &para::TxbxTarget,
    url: Option<&str>,
    anchor: Option<&str>,
    tooltip: Option<&str>,
    i: usize,
) -> Result<(), Error> {
    let part_name = ink::part_for_target(catalog, doc_part, target, id, &format!("ops[{i}].id"))?;
    let key = target.key.clone();
    load_part(pkg, docs, &part_name)?;
    {
        let ctx = ink::HostCtx::load(pkg, &part_name)?;
        let doc = ink_doc_mut(docs, &part_name)?;
        match ink::find_drawing_mut(&mut doc.root, &ctx, &key) {
            None => {
                return Err(Error::with_path(
                    ErrorKind::InkNotFound { id: id.to_string() },
                    format!("ops[{i}].id"),
                ));
            }
            Some(d) if drawing_hlink::picture_has_click(d) => {
                return Err(Error::with_path(
                    ErrorKind::ShapeAlreadyHasHyperlink,
                    format!("ops[{i}].id"),
                ));
            }
            Some(_) => {}
        }
    }
    let rid = {
        let mut rels = Relationships::from_package(pkg, &part_name)?;
        let rid = drawing_hlink::push_rel(&mut rels, url, anchor);
        rels.write_to(pkg, &part_name)?;
        rid
    };
    {
        let ctx = ink::HostCtx::load(pkg, &part_name)?;
        let doc = ink_doc_mut(docs, &part_name)?;
        drawing_hlink::ensure_ns(&mut doc.root);
        let drawing = ink::find_drawing_mut(&mut doc.root, &ctx, &key).ok_or_else(|| {
            Error::with_path(
                ErrorKind::InkNotFound { id: id.to_string() },
                format!("ops[{i}].id"),
            )
        })?;
        let parent = drawing_hlink::picture_click_parent_mut(drawing).ok_or_else(|| {
            Error::with_path(
                ErrorKind::InkNotFound { id: id.to_string() },
                format!("ops[{i}].id"),
            )
        })?;
        drawing_hlink::insert_click(parent, &rid, tooltip);
    }
    dirty.insert(part_name, ());
    Ok(())
}

fn apply_insert_model3d_hlink(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    id: &str,
    target: &para::TxbxTarget,
    url: Option<&str>,
    anchor: Option<&str>,
    tooltip: Option<&str>,
    i: usize,
) -> Result<(), Error> {
    let part_name =
        model3d::part_for_target(catalog, doc_part, target, id, &format!("ops[{i}].id"))?;
    let key = target.key.clone();
    apply_insert_click_hlink(
        pkg,
        docs,
        dirty,
        &part_name,
        id,
        url,
        anchor,
        tooltip,
        i,
        |root| {
            let frame = model3d::find_frame_mut(root, &key)?;
            Some(drawing_hlink::picture_has_click(frame))
        },
        |root| {
            let frame = model3d::find_frame_mut(root, &key)?;
            drawing_hlink::picture_click_parent_mut(frame)
        },
        ErrorKind::Model3dNotFound { id: id.to_string() },
    )
}

fn apply_insert_chart_hlink(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    id: &str,
    target: &charts::ChartTarget,
    url: Option<&str>,
    anchor: Option<&str>,
    tooltip: Option<&str>,
    i: usize,
) -> Result<(), Error> {
    let part_name =
        charts::part_for_target(pkg, catalog, doc_part, target, id, &format!("ops[{i}].id"))?;
    apply_insert_click_hlink(
        pkg,
        docs,
        dirty,
        &part_name,
        id,
        url,
        anchor,
        tooltip,
        i,
        |root| {
            let drawing = charts::find_drawing_mut(root, target)?;
            Some(drawing_hlink::picture_has_click(drawing))
        },
        |root| {
            let drawing = charts::find_drawing_mut(root, target)?;
            drawing_hlink::picture_click_parent_mut(drawing)
        },
        ErrorKind::ChartNotFound { id: id.to_string() },
    )
}

fn apply_insert_smartart_hlink(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    id: &str,
    target: &smartart::DiagramTarget,
    url: Option<&str>,
    anchor: Option<&str>,
    tooltip: Option<&str>,
    i: usize,
) -> Result<(), Error> {
    let part_name =
        smartart::part_for_target(pkg, catalog, doc_part, target, id, &format!("ops[{i}].id"))?;
    apply_insert_click_hlink(
        pkg,
        docs,
        dirty,
        &part_name,
        id,
        url,
        anchor,
        tooltip,
        i,
        |root| {
            let drawing = smartart::find_drawing_mut(root, target)?;
            Some(drawing_hlink::picture_has_click(drawing))
        },
        |root| {
            let drawing = smartart::find_drawing_mut(root, target)?;
            drawing_hlink::picture_click_parent_mut(drawing)
        },
        ErrorKind::SmartArtNotFound { id: id.to_string() },
    )
}

fn apply_set_hyperlink(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &SetHyperlinkOp,
    i: usize,
) -> Result<(), Error> {
    match &op.target {
        HyperlinkTarget::Link(target) => {
            apply_set_para_hyperlink(pkg, docs, dirty, doc_part, catalog, &op.id, target, op, i)
        }
        HyperlinkTarget::Picture(target) => {
            apply_set_pic_hlink(pkg, docs, dirty, doc_part, catalog, &op.id, target, op, i)
        }
        HyperlinkTarget::TextBox(target) => {
            apply_set_txbx_hlink(pkg, docs, dirty, doc_part, catalog, &op.id, target, op, i)
        }
        HyperlinkTarget::Shape(target) => apply_set_wsp_hlink(
            pkg,
            docs,
            dirty,
            doc_part,
            catalog,
            &op.id,
            target,
            op,
            i,
            shape::has_click,
            shape::hlink_parent_mut,
            ErrorKind::ShapeNotFound { id: op.id.clone() },
        ),
        HyperlinkTarget::Connector(target) => apply_set_wsp_hlink(
            pkg,
            docs,
            dirty,
            doc_part,
            catalog,
            &op.id,
            target,
            op,
            i,
            connector::has_click,
            connector::hlink_parent_mut,
            ErrorKind::ConnectorNotFound { id: op.id.clone() },
        ),
        HyperlinkTarget::Ink(target) => {
            apply_set_ink_hlink(pkg, docs, dirty, doc_part, catalog, &op.id, target, op, i)
        }
        HyperlinkTarget::Model3d(target) => {
            apply_set_model3d_hlink(pkg, docs, dirty, doc_part, catalog, &op.id, target, op, i)
        }
        HyperlinkTarget::Chart(target) => {
            apply_set_chart_hlink(pkg, docs, dirty, doc_part, catalog, &op.id, target, op, i)
        }
        HyperlinkTarget::SmartArt(target) => {
            apply_set_smartart_hlink(pkg, docs, dirty, doc_part, catalog, &op.id, target, op, i)
        }
    }
}

fn apply_set_para_hyperlink(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    id: &str,
    target: &link::LinkRef,
    op: &SetHyperlinkOp,
    i: usize,
) -> Result<(), Error> {
    let part_name = link::part_for_story(catalog, doc_part, &target.story, || {
        link::missing_link_err(id, i)
    })?;
    let old_rid = {
        let doc = load_part(pkg, docs, &part_name)?;
        let story = story_xml_mut(&mut doc.root, &target.story, || {
            link::missing_link_err(id, i)
        })?;
        let el = link::hyperlink_at_mut(story, target.n, id, i)?;
        link::r_id(el).map(str::to_string)
    };
    if let Some(url) = op.url.as_deref() {
        let mut rels = Relationships::from_package(pkg, &part_name)?;
        let rid = if let Some(existing) = old_rid.clone() {
            if let Some(rel) = rels.rels.iter_mut().find(|r| r.id == existing) {
                rel.target = url.to_string();
                existing
            } else {
                rels.push(Relationship {
                    id: existing.clone(),
                    rel_type: REL_HYPERLINK.to_string(),
                    target: url.to_string(),
                    target_mode: Some("External".into()),
                    extra: Vec::new(),
                });
                existing
            }
        } else {
            push_hyperlink_rel(&mut rels, url)
        };
        rels.write_to(pkg, &part_name)?;
        let doc = load_part(pkg, docs, &part_name)?;
        let story = story_xml_mut(&mut doc.root, &target.story, || {
            link::missing_link_err(id, i)
        })?;
        let el = link::hyperlink_at_mut(story, target.n, id, i)?;
        link::set_hyperlink_url(el, &rid);
    }
    if let Some(anchor) = op.anchor.as_deref() {
        {
            let doc = load_part(pkg, docs, &part_name)?;
            let story = story_xml_mut(&mut doc.root, &target.story, || {
                link::missing_link_err(id, i)
            })?;
            let el = link::hyperlink_at_mut(story, target.n, id, i)?;
            link::set_hyperlink_anchor(el, anchor);
        }
        if let Some(rid) = old_rid.as_deref() {
            let doc = load_part(pkg, docs, &part_name)?;
            drop_rel_if_unused(pkg, &doc.root, &part_name, rid)?;
        }
    }
    if let Some(text) = op.text.as_deref() {
        let doc = load_part(pkg, docs, &part_name)?;
        let story = story_xml_mut(&mut doc.root, &target.story, || {
            link::missing_link_err(id, i)
        })?;
        let el = link::hyperlink_at_mut(story, target.n, id, i)?;
        para::set_visible_text(el, text);
    }
    if let Some(tooltip) = op.tooltip.as_ref() {
        let doc = load_part(pkg, docs, &part_name)?;
        let story = story_xml_mut(&mut doc.root, &target.story, || {
            link::missing_link_err(id, i)
        })?;
        let el = link::hyperlink_at_mut(story, target.n, id, i)?;
        link::set_hyperlink_tooltip(el, tooltip.as_deref());
    }
    dirty.insert(part_name, ());
    Ok(())
}

fn apply_set_pic_hlink(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    id: &str,
    target: &picture::PicTarget,
    op: &SetHyperlinkOp,
    i: usize,
) -> Result<(), Error> {
    apply_set_drawing_hlink(
        pkg,
        docs,
        dirty,
        &part_for_pic(pkg, catalog, doc_part, target, id, i)?,
        id,
        i,
        op,
        |root| {
            let drawing = find_target_drawing_mut(root, target)?;
            picture_hlink_parent_mut(drawing, target)
        },
        |root| {
            let drawing = find_target_drawing_mut(root, target)?;
            Some(picture_hlink_exists(drawing, target))
        },
        ErrorKind::PictureNotFound { id: id.to_string() },
    )
}

fn apply_set_txbx_hlink(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    id: &str,
    target: &para::TxbxTarget,
    op: &SetHyperlinkOp,
    i: usize,
) -> Result<(), Error> {
    apply_set_drawing_hlink(
        pkg,
        docs,
        dirty,
        &part_for_txbx(pkg, catalog, doc_part, target, id, i)?,
        id,
        i,
        op,
        |root| {
            let host = para::find_paragraph_mut(root, target.host.body())?;
            let drawing = textbox::find_textbox_drawing_mut(host, &target.key)?;
            drawing_hlink::textbox_click_parent_mut(drawing)
        },
        |root| {
            let host = para::find_paragraph_mut(root, target.host.body())?;
            let drawing = textbox::find_textbox_drawing_mut(host, &target.key)?;
            Some(drawing_hlink::textbox_has_click(drawing))
        },
        ErrorKind::TextBoxNotFound { id: id.to_string() },
    )
}

fn apply_set_wsp_hlink(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    id: &str,
    target: &para::TxbxTarget,
    op: &SetHyperlinkOp,
    i: usize,
    has_click: fn(&mut Element, &para::TxbxKey) -> Option<bool>,
    parent_mut: for<'a> fn(&'a mut Element, &para::TxbxKey) -> Option<&'a mut Element>,
    missing: ErrorKind,
) -> Result<(), Error> {
    let part_name = part_for_txbx(pkg, catalog, doc_part, target, id, i)?;
    let key = target.key.clone();
    apply_set_drawing_hlink(
        pkg,
        docs,
        dirty,
        &part_name,
        id,
        i,
        op,
        |root| {
            let host = para::find_paragraph_mut(root, target.host.body())?;
            parent_mut(host, &key)
        },
        |root| {
            let host = para::find_paragraph_mut(root, target.host.body())?;
            has_click(host, &key)
        },
        missing,
    )
}

fn apply_set_ink_hlink(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    id: &str,
    target: &para::TxbxTarget,
    op: &SetHyperlinkOp,
    i: usize,
) -> Result<(), Error> {
    let part_name = ink::part_for_target(catalog, doc_part, target, id, &format!("ops[{i}].id"))?;
    let key = target.key.clone();
    load_part(pkg, docs, &part_name)?;
    {
        let ctx = ink::HostCtx::load(pkg, &part_name)?;
        let doc = ink_doc_mut(docs, &part_name)?;
        match ink::find_drawing_mut(&mut doc.root, &ctx, &key) {
            None => {
                return Err(Error::with_path(
                    ErrorKind::InkNotFound { id: id.to_string() },
                    format!("ops[{i}].id"),
                ));
            }
            Some(d) if drawing_hlink::picture_has_click(d) => {}
            Some(_) => {
                return Err(Error::with_path(
                    ErrorKind::HyperlinkNotFound { id: id.to_string() },
                    format!("ops[{i}].id"),
                ));
            }
        }
    }
    let old_rid = {
        let ctx = ink::HostCtx::load(pkg, &part_name)?;
        let doc = ink_doc_mut(docs, &part_name)?;
        let drawing = ink::find_drawing_mut(&mut doc.root, &ctx, &key).ok_or_else(|| {
            Error::with_path(
                ErrorKind::HyperlinkNotFound { id: id.to_string() },
                format!("ops[{i}].id"),
            )
        })?;
        let parent = drawing_hlink::picture_click_parent_mut(drawing).ok_or_else(|| {
            Error::with_path(
                ErrorKind::HyperlinkNotFound { id: id.to_string() },
                format!("ops[{i}].id"),
            )
        })?;
        drawing_hlink::click_rid(parent)
    };
    let new_rid = if op.url.is_some() || op.anchor.is_some() {
        let mut rels = Relationships::from_package(pkg, &part_name)?;
        let rid = drawing_hlink::set_rel(
            &mut rels,
            old_rid.as_deref(),
            op.url.as_deref(),
            op.anchor.as_deref(),
        );
        rels.write_to(pkg, &part_name)?;
        Some(rid)
    } else {
        None
    };
    {
        let ctx = ink::HostCtx::load(pkg, &part_name)?;
        let doc = ink_doc_mut(docs, &part_name)?;
        drawing_hlink::ensure_ns(&mut doc.root);
        let drawing = ink::find_drawing_mut(&mut doc.root, &ctx, &key).ok_or_else(|| {
            Error::with_path(
                ErrorKind::HyperlinkNotFound { id: id.to_string() },
                format!("ops[{i}].id"),
            )
        })?;
        let parent = drawing_hlink::picture_click_parent_mut(drawing).ok_or_else(|| {
            Error::with_path(
                ErrorKind::HyperlinkNotFound { id: id.to_string() },
                format!("ops[{i}].id"),
            )
        })?;
        let tooltip = op.tooltip.as_ref().map(|t| t.as_deref());
        if !drawing_hlink::set_click(parent, new_rid.as_deref(), tooltip) {
            return Err(Error::with_path(
                ErrorKind::HyperlinkNotFound { id: id.to_string() },
                format!("ops[{i}].id"),
            ));
        }
    }
    if let (Some(old), Some(new)) = (old_rid.as_deref(), new_rid.as_deref()) {
        if old != new {
            let doc = load_part(pkg, docs, &part_name)?;
            drop_rel_if_unused(pkg, &doc.root, &part_name, old)?;
        }
    }
    dirty.insert(part_name, ());
    Ok(())
}

fn apply_set_model3d_hlink(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    id: &str,
    target: &para::TxbxTarget,
    op: &SetHyperlinkOp,
    i: usize,
) -> Result<(), Error> {
    let part_name =
        model3d::part_for_target(catalog, doc_part, target, id, &format!("ops[{i}].id"))?;
    let key = target.key.clone();
    apply_set_drawing_hlink(
        pkg,
        docs,
        dirty,
        &part_name,
        id,
        i,
        op,
        |root| {
            let frame = model3d::find_frame_mut(root, &key)?;
            drawing_hlink::picture_click_parent_mut(frame)
        },
        |root| {
            let frame = model3d::find_frame_mut(root, &key)?;
            Some(drawing_hlink::picture_has_click(frame))
        },
        ErrorKind::Model3dNotFound { id: id.to_string() },
    )
}

fn apply_set_chart_hlink(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    id: &str,
    target: &charts::ChartTarget,
    op: &SetHyperlinkOp,
    i: usize,
) -> Result<(), Error> {
    let part_name =
        charts::part_for_target(pkg, catalog, doc_part, target, id, &format!("ops[{i}].id"))?;
    apply_set_drawing_hlink(
        pkg,
        docs,
        dirty,
        &part_name,
        id,
        i,
        op,
        |root| {
            let drawing = charts::find_drawing_mut(root, target)?;
            drawing_hlink::picture_click_parent_mut(drawing)
        },
        |root| {
            let drawing = charts::find_drawing_mut(root, target)?;
            Some(drawing_hlink::picture_has_click(drawing))
        },
        ErrorKind::ChartNotFound { id: id.to_string() },
    )
}

fn apply_set_smartart_hlink(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    id: &str,
    target: &smartart::DiagramTarget,
    op: &SetHyperlinkOp,
    i: usize,
) -> Result<(), Error> {
    let part_name =
        smartart::part_for_target(pkg, catalog, doc_part, target, id, &format!("ops[{i}].id"))?;
    apply_set_drawing_hlink(
        pkg,
        docs,
        dirty,
        &part_name,
        id,
        i,
        op,
        |root| {
            let drawing = smartart::find_drawing_mut(root, target)?;
            drawing_hlink::picture_click_parent_mut(drawing)
        },
        |root| {
            let drawing = smartart::find_drawing_mut(root, target)?;
            Some(drawing_hlink::picture_has_click(drawing))
        },
        ErrorKind::SmartArtNotFound { id: id.to_string() },
    )
}

fn apply_set_drawing_hlink(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    part_name: &str,
    id: &str,
    i: usize,
    op: &SetHyperlinkOp,
    mut parent_mut: impl FnMut(&mut Element) -> Option<&mut Element>,
    mut exists: impl FnMut(&mut Element) -> Option<bool>,
    missing: ErrorKind,
) -> Result<(), Error> {
    {
        let doc = load_part(pkg, docs, part_name)?;
        match exists(&mut doc.root) {
            Some(true) => {}
            Some(false) => {
                return Err(Error::with_path(
                    ErrorKind::HyperlinkNotFound { id: id.to_string() },
                    format!("ops[{i}].id"),
                ));
            }
            None => {
                return Err(Error::with_path(missing, format!("ops[{i}].id")));
            }
        }
    }
    let old_rid = {
        let doc = load_part(pkg, docs, part_name)?;
        let parent = parent_mut(&mut doc.root).ok_or_else(|| {
            Error::with_path(
                ErrorKind::HyperlinkNotFound { id: id.to_string() },
                format!("ops[{i}].id"),
            )
        })?;
        drawing_hlink::click_rid(parent)
    };
    let new_rid = if op.url.is_some() || op.anchor.is_some() {
        let mut rels = Relationships::from_package(pkg, part_name)?;
        let rid = drawing_hlink::set_rel(
            &mut rels,
            old_rid.as_deref(),
            op.url.as_deref(),
            op.anchor.as_deref(),
        );
        rels.write_to(pkg, part_name)?;
        Some(rid)
    } else {
        None
    };
    {
        let doc = load_part(pkg, docs, part_name)?;
        drawing_hlink::ensure_ns(&mut doc.root);
        let parent = parent_mut(&mut doc.root).ok_or_else(|| {
            Error::with_path(
                ErrorKind::HyperlinkNotFound { id: id.to_string() },
                format!("ops[{i}].id"),
            )
        })?;
        let tooltip = op.tooltip.as_ref().map(|t| t.as_deref());
        if !drawing_hlink::set_click(parent, new_rid.as_deref(), tooltip) {
            return Err(Error::with_path(
                ErrorKind::HyperlinkNotFound { id: id.to_string() },
                format!("ops[{i}].id"),
            ));
        }
    }
    if let (Some(old), Some(new)) = (old_rid.as_deref(), new_rid.as_deref()) {
        if old != new {
            let doc = load_part(pkg, docs, part_name)?;
            drop_rel_if_unused(pkg, &doc.root, part_name, old)?;
        }
    }
    dirty.insert(part_name.to_string(), ());
    Ok(())
}

fn apply_delete_hyperlink(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &DeleteHyperlinkOp,
    i: usize,
) -> Result<(), Error> {
    match &op.target {
        HyperlinkTarget::Link(target) => {
            apply_delete_para_hyperlink(pkg, docs, dirty, doc_part, catalog, &op.id, target, i)
        }
        HyperlinkTarget::Picture(target) => {
            apply_delete_pic_hlink(pkg, docs, dirty, doc_part, catalog, &op.id, target, i)
        }
        HyperlinkTarget::TextBox(target) => {
            apply_delete_txbx_hlink(pkg, docs, dirty, doc_part, catalog, &op.id, target, i)
        }
        HyperlinkTarget::Shape(target) => apply_delete_wsp_hlink(
            pkg,
            docs,
            dirty,
            doc_part,
            catalog,
            &op.id,
            target,
            i,
            shape::hlink_parent_mut,
        ),
        HyperlinkTarget::Connector(target) => apply_delete_wsp_hlink(
            pkg,
            docs,
            dirty,
            doc_part,
            catalog,
            &op.id,
            target,
            i,
            connector::hlink_parent_mut,
        ),
        HyperlinkTarget::Ink(target) => {
            apply_delete_ink_hlink(pkg, docs, dirty, doc_part, catalog, &op.id, target, i)
        }
        HyperlinkTarget::Model3d(target) => {
            apply_delete_model3d_hlink(pkg, docs, dirty, doc_part, catalog, &op.id, target, i)
        }
        HyperlinkTarget::Chart(target) => {
            apply_delete_chart_hlink(pkg, docs, dirty, doc_part, catalog, &op.id, target, i)
        }
        HyperlinkTarget::SmartArt(target) => {
            apply_delete_smartart_hlink(pkg, docs, dirty, doc_part, catalog, &op.id, target, i)
        }
    }
}

fn apply_delete_para_hyperlink(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    id: &str,
    target: &link::LinkRef,
    i: usize,
) -> Result<(), Error> {
    let part_name = link::part_for_story(catalog, doc_part, &target.story, || {
        link::missing_link_err(id, i)
    })?;
    let old_rid = {
        let doc = load_part(pkg, docs, &part_name)?;
        let story = story_xml_mut(&mut doc.root, &target.story, || {
            link::missing_link_err(id, i)
        })?;
        let el = link::hyperlink_at_mut(story, target.n, id, i)?;
        link::r_id(el).map(str::to_string)
    };
    {
        let doc = load_part(pkg, docs, &part_name)?;
        let story = story_xml_mut(&mut doc.root, &target.story, || {
            link::missing_link_err(id, i)
        })?;
        link::unwrap_hyperlink(story, target.n, id, i)?;
    }
    if let Some(rid) = old_rid {
        let doc = load_part(pkg, docs, &part_name)?;
        drop_rel_if_unused(pkg, &doc.root, &part_name, &rid)?;
    }
    dirty.insert(part_name, ());
    Ok(())
}

fn apply_delete_pic_hlink(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    id: &str,
    target: &picture::PicTarget,
    i: usize,
) -> Result<(), Error> {
    let part_name = part_for_pic(pkg, catalog, doc_part, target, id, i)?;
    apply_delete_drawing_hlink(pkg, docs, dirty, &part_name, id, i, |root| {
        let drawing = find_target_drawing_mut(root, target)?;
        picture_hlink_parent_mut(drawing, target)
    })
}

fn apply_delete_txbx_hlink(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    id: &str,
    target: &para::TxbxTarget,
    i: usize,
) -> Result<(), Error> {
    let part_name = part_for_txbx(pkg, catalog, doc_part, target, id, i)?;
    apply_delete_drawing_hlink(pkg, docs, dirty, &part_name, id, i, |root| {
        let host = para::find_paragraph_mut(root, target.host.body())?;
        let drawing = textbox::find_textbox_drawing_mut(host, &target.key)?;
        drawing_hlink::textbox_click_parent_mut(drawing)
    })
}

fn apply_delete_wsp_hlink(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    id: &str,
    target: &para::TxbxTarget,
    i: usize,
    parent_mut: for<'a> fn(&'a mut Element, &para::TxbxKey) -> Option<&'a mut Element>,
) -> Result<(), Error> {
    let part_name = part_for_txbx(pkg, catalog, doc_part, target, id, i)?;
    let key = target.key.clone();
    apply_delete_drawing_hlink(pkg, docs, dirty, &part_name, id, i, |root| {
        let host = para::find_paragraph_mut(root, target.host.body())?;
        parent_mut(host, &key)
    })
}

fn apply_delete_ink_hlink(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    id: &str,
    target: &para::TxbxTarget,
    i: usize,
) -> Result<(), Error> {
    let part_name = ink::part_for_target(catalog, doc_part, target, id, &format!("ops[{i}].id"))?;
    let key = target.key.clone();
    load_part(pkg, docs, &part_name)?;
    let old_rid = {
        let ctx = ink::HostCtx::load(pkg, &part_name)?;
        let doc = ink_doc_mut(docs, &part_name)?;
        let drawing = ink::find_drawing_mut(&mut doc.root, &ctx, &key).ok_or_else(|| {
            Error::with_path(
                ErrorKind::HyperlinkNotFound { id: id.to_string() },
                format!("ops[{i}].id"),
            )
        })?;
        let parent = drawing_hlink::picture_click_parent_mut(drawing).ok_or_else(|| {
            Error::with_path(
                ErrorKind::HyperlinkNotFound { id: id.to_string() },
                format!("ops[{i}].id"),
            )
        })?;
        drawing_hlink::delete_click(parent).ok_or_else(|| {
            Error::with_path(
                ErrorKind::HyperlinkNotFound { id: id.to_string() },
                format!("ops[{i}].id"),
            )
        })?
    };
    if let Some(rid) = old_rid {
        let doc = load_part(pkg, docs, &part_name)?;
        drop_rel_if_unused(pkg, &doc.root, &part_name, &rid)?;
    }
    dirty.insert(part_name, ());
    Ok(())
}

fn apply_delete_model3d_hlink(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    id: &str,
    target: &para::TxbxTarget,
    i: usize,
) -> Result<(), Error> {
    let part_name =
        model3d::part_for_target(catalog, doc_part, target, id, &format!("ops[{i}].id"))?;
    let key = target.key.clone();
    apply_delete_drawing_hlink(pkg, docs, dirty, &part_name, id, i, |root| {
        let frame = model3d::find_frame_mut(root, &key)?;
        drawing_hlink::picture_click_parent_mut(frame)
    })
}

fn apply_delete_chart_hlink(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    id: &str,
    target: &charts::ChartTarget,
    i: usize,
) -> Result<(), Error> {
    let part_name =
        charts::part_for_target(pkg, catalog, doc_part, target, id, &format!("ops[{i}].id"))?;
    apply_delete_drawing_hlink(pkg, docs, dirty, &part_name, id, i, |root| {
        let drawing = charts::find_drawing_mut(root, target)?;
        drawing_hlink::picture_click_parent_mut(drawing)
    })
}

fn apply_delete_smartart_hlink(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    id: &str,
    target: &smartart::DiagramTarget,
    i: usize,
) -> Result<(), Error> {
    let part_name =
        smartart::part_for_target(pkg, catalog, doc_part, target, id, &format!("ops[{i}].id"))?;
    apply_delete_drawing_hlink(pkg, docs, dirty, &part_name, id, i, |root| {
        let drawing = smartart::find_drawing_mut(root, target)?;
        drawing_hlink::picture_click_parent_mut(drawing)
    })
}

fn apply_delete_drawing_hlink(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    part_name: &str,
    id: &str,
    i: usize,
    mut parent_mut: impl FnMut(&mut Element) -> Option<&mut Element>,
) -> Result<(), Error> {
    let old_rid = {
        let doc = load_part(pkg, docs, part_name)?;
        let parent = parent_mut(&mut doc.root).ok_or_else(|| {
            Error::with_path(
                ErrorKind::HyperlinkNotFound { id: id.to_string() },
                format!("ops[{i}].id"),
            )
        })?;
        drawing_hlink::delete_click(parent).ok_or_else(|| {
            Error::with_path(
                ErrorKind::HyperlinkNotFound { id: id.to_string() },
                format!("ops[{i}].id"),
            )
        })?
    };
    if let Some(rid) = old_rid {
        let doc = load_part(pkg, docs, part_name)?;
        drop_rel_if_unused(pkg, &doc.root, part_name, &rid)?;
    }
    dirty.insert(part_name.to_string(), ());
    Ok(())
}

fn picture_hlink_exists(drawing: &Element, target: &picture::PicTarget) -> bool {
    if picture::is_group_drawing(drawing) {
        picture::grouped_pic_has_click(drawing, target.key())
    } else {
        drawing_hlink::picture_has_click(drawing)
    }
}

fn picture_hlink_parent_mut<'a>(
    drawing: &'a mut Element,
    target: &picture::PicTarget,
) -> Option<&'a mut Element> {
    if picture::is_group_drawing(drawing) {
        picture::find_grouped_pic_cnv_pr_mut(drawing, target.key())
    } else {
        drawing_hlink::picture_click_parent_mut(drawing)
    }
}

fn story_body_para(target: &ParaTarget) -> Option<&BodyPara> {
    match target {
        ParaTarget::Body(p) | ParaTarget::HeaderFooter { para: p, .. } => Some(p),
        _ => None,
    }
}

fn para_target_path(root: &Element, target: &ParaTarget) -> Option<Vec<usize>> {
    match target {
        ParaTarget::Body(p) | ParaTarget::HeaderFooter { para: p, .. } => {
            para::find_paragraph_path(root, p)
        }
        ParaTarget::InTextBox { host, key, para } => {
            let hp = para::find_paragraph_path(root, host.body())?;
            let host_el = para::element_at(root, &hp)?;
            let mut path = hp;
            path.extend(textbox::content_path(host_el, key)?);
            let content = para::element_at(root, &path)?;
            path.extend(para::find_paragraph_path_in_txbx(content, para)?);
            Some(path)
        }
        _ => None,
    }
}

fn story_xml_mut<'a>(
    root: &'a mut Element,
    story: &link::LinkStory,
    missing: impl FnOnce() -> Error,
) -> Result<&'a mut Element, Error> {
    match story {
        link::LinkStory::Body | link::LinkStory::HeaderFooter { .. } => Ok(root),
        link::LinkStory::TextBox { host, key } => {
            let Some(host_p) = para::find_paragraph_mut(root, host.body()) else {
                return Err(missing());
            };
            textbox::find_txbx_content_by_key_mut(host_p, key).ok_or_else(missing)
        }
    }
}

fn apply_insert_bookmark(
    pkg: &Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &InsertBookmarkOp,
    i: usize,
) -> Result<(), Error> {
    let part_name = part_for_para(pkg, catalog, doc_part, &op.start, &op.id, i)?;
    let doc = load_part(pkg, docs, &part_name)?;
    let start = para_target_path(&doc.root, &op.start).ok_or_else(|| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: crate::error::IdReason::InsertBookmarkNeedsPara,
            },
            format!("ops[{i}].id"),
        )
    })?;
    let end = para_target_path(&doc.root, &op.end).ok_or_else(|| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: crate::error::IdReason::InsertBookmarkNeedsPara,
            },
            format!("ops[{i}].end"),
        )
    })?;
    link::insert_bookmark(&mut doc.root, &start, &end, op.name.as_deref(), &op.id, i)?;
    dirty.insert(part_name, ());
    Ok(())
}

fn apply_insert_edit_range(
    pkg: &Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &InsertEditRangeOp,
    i: usize,
) -> Result<(), Error> {
    let part_name = part_for_para(pkg, catalog, doc_part, &op.para, &op.id, i)?;
    let para = story_body_para(&op.para).ok_or_else(|| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: crate::error::IdReason::InsertEditRangeNeedsPara,
            },
            format!("ops[{i}].id"),
        )
    })?;
    let doc = load_part(pkg, docs, &part_name)?;
    super::perm::insert_edit_range(
        &mut doc.root,
        para,
        op.ed_grp.as_deref(),
        op.ed.as_deref(),
        &op.id,
        i,
    )?;
    dirty.insert(part_name, ());
    Ok(())
}

fn apply_delete_edit_range(
    pkg: &Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &DeleteEditRangeOp,
    i: usize,
) -> Result<(), Error> {
    let part_name = super::perm::part_for_story(catalog, doc_part, &op.target.story, || {
        super::perm::missing_perm_err(&op.id, i)
    })?;
    let doc = load_part(pkg, docs, &part_name)?;
    super::perm::delete_edit_range(&mut doc.root, op.target.n, &op.id, i)?;
    dirty.insert(part_name, ());
    Ok(())
}

fn apply_set_bookmark(
    pkg: &Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &SetBookmarkOp,
    i: usize,
) -> Result<(), Error> {
    let part_name = link::part_for_story(catalog, doc_part, &op.target.story, || {
        link::missing_bookmark_err(&op.id, i)
    })?;
    let doc = load_part(pkg, docs, &part_name)?;
    let taken = link::all_bookmark_names(&doc.root);
    let renamed = {
        let story = story_xml_mut(&mut doc.root, &op.target.story, || {
            link::missing_bookmark_err(&op.id, i)
        })?;
        link::set_bookmark_name(story, op.target.n, &op.name, &taken, &op.id, i)?
    };
    if let Some((old, new)) = renamed {
        link::rename_hyperlink_anchors(&mut doc.root, &old, &new);
    }
    dirty.insert(part_name, ());
    Ok(())
}

fn apply_delete_bookmark(
    pkg: &Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &DeleteBookmarkOp,
    i: usize,
) -> Result<(), Error> {
    let part_name = link::part_for_story(catalog, doc_part, &op.target.story, || {
        link::missing_bookmark_err(&op.id, i)
    })?;
    let doc = load_part(pkg, docs, &part_name)?;
    let story = story_xml_mut(&mut doc.root, &op.target.story, || {
        link::missing_bookmark_err(&op.id, i)
    })?;
    link::delete_bookmark(story, op.target.n, &op.id, i)?;
    dirty.insert(part_name, ());
    Ok(())
}

fn apply_insert_field(
    pkg: &Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &InsertFieldOp,
    i: usize,
) -> Result<(), Error> {
    let part_name = part_for_para(pkg, catalog, doc_part, &op.para, &op.id, i)?;
    let doc = load_part(pkg, docs, &part_name)?;
    let el = find_insert_para_mut(&mut doc.root, &op.para).ok_or_else(|| {
        Error::with_path(
            ErrorKind::ParaNotFound { id: op.id.clone() },
            format!("ops[{i}].id"),
        )
    })?;
    field::insert_field_form(el, &op.instr, op.result.as_deref(), op.form.as_ref());
    dirty.insert(part_name, ());
    Ok(())
}

fn apply_set_field(
    pkg: &Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &SetFieldOp,
    i: usize,
) -> Result<(), Error> {
    let part_name = field::part_for_story(catalog, doc_part, &op.target.story, || {
        field::missing_field_err(&op.id, i)
    })?;
    let doc = load_part(pkg, docs, &part_name)?;
    let story = story_xml_mut(&mut doc.root, &op.target.story, || {
        field::missing_field_err(&op.id, i)
    })?;
    field::set_field_instr(story, op.target.n, &op.instr, &op.id, i)?;
    dirty.insert(part_name, ());
    Ok(())
}

fn apply_delete_field(
    pkg: &Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &DeleteFieldOp,
    i: usize,
) -> Result<(), Error> {
    let part_name = field::part_for_story(catalog, doc_part, &op.target.story, || {
        field::missing_field_err(&op.id, i)
    })?;
    let doc = load_part(pkg, docs, &part_name)?;
    let story = story_xml_mut(&mut doc.root, &op.target.story, || {
        field::missing_field_err(&op.id, i)
    })?;
    field::delete_field(story, op.target.n, op.keep, &op.id, i)?;
    dirty.insert(part_name, ());
    Ok(())
}

fn apply_insert_numbering(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    op: &InsertNumberingOp,
) -> Result<(), Error> {
    let part = numbering::ensure_part(pkg, docs, dirty, doc_part)?;
    let doc = load_part(pkg, docs, &part)?;
    numbering::insert_definition(&mut doc.root, op.kind);
    dirty.insert(part, ());
    Ok(())
}

fn apply_set_numbering(
    pkg: &Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    op: &SetNumberingOp,
    i: usize,
) -> Result<(), Error> {
    let Some(part) = numbering::numbering_part(pkg, doc_part)? else {
        return Err(numbering::missing_abstract(&op.id, i));
    };
    let doc = load_part(pkg, docs, &part)?;
    numbering::set_level(
        &mut doc.root,
        op.abstract_id,
        op.ilvl,
        op.num_fmt.as_deref(),
        op.start,
        op.lvl_text.as_deref(),
        &op.id,
        i,
    )?;
    dirty.insert(part, ());
    Ok(())
}

fn apply_set_para_numbering(
    pkg: &Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &SetParaNumberingOp,
    i: usize,
) -> Result<(), Error> {
    if let Some(num_id) = op.num {
        let num_part = numbering::numbering_part(pkg, doc_part)?
            .ok_or_else(|| numbering::missing_num(&format!("num:{num_id}"), i))?;
        {
            let num_doc = load_part(pkg, docs, &num_part)?;
            if !numbering::num_exists(&num_doc.root, num_id) {
                return Err(numbering::missing_num(&format!("num:{num_id}"), i));
            }
        }
    }
    let part_name = part_for_para(pkg, catalog, doc_part, &op.para, &op.id, i)?;
    let doc = load_part(pkg, docs, &part_name)?;
    let el = find_insert_para_mut(&mut doc.root, &op.para).ok_or_else(|| {
        Error::with_path(
            ErrorKind::ParaNotFound { id: op.id.clone() },
            format!("ops[{i}].id"),
        )
    })?;
    match op.num {
        Some(n) => numbering::bind_para(el, n, op.ilvl),
        None => numbering::unbind_para(el),
    }
    dirty.insert(part_name, ());
    Ok(())
}

fn apply_set_style(
    pkg: &Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    op: &SetStyleOp,
    i: usize,
) -> Result<(), Error> {
    let Some(part) = styles::styles_part(pkg, doc_part)? else {
        return Err(styles::missing_style(&op.id, i));
    };
    let doc = load_part(pkg, docs, &part)?;
    styles::set_style_props(
        &mut doc.root,
        &op.style_id,
        styles::StylePatch {
            font: op.font.as_deref(),
            east_asia_font: op.east_asia_font.as_deref(),
            sz: op.sz,
            bold: op.bold,
            color: op.color.as_deref(),
            before: op.before,
            after: op.after,
        },
        &op.id,
        i,
    )?;
    dirty.insert(part, ());
    Ok(())
}

fn apply_set_para_style(
    pkg: &Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &SetParaStyleOp,
    i: usize,
) -> Result<(), Error> {
    if let Some(style_id) = op.style.as_deref() {
        let style_part = styles::styles_part(pkg, doc_part)?
            .ok_or_else(|| styles::missing_para_style(&format!("style:{style_id}"), i))?;
        {
            let style_doc = load_part(pkg, docs, &style_part)?;
            let Some(el) = styles::find_style(&style_doc.root, style_id) else {
                return Err(styles::missing_para_style(&format!("style:{style_id}"), i));
            };
            if !styles::is_paragraph_style(el) {
                return Err(Error::with_path(
                    ErrorKind::SetStyleNeedsParagraph,
                    format!("ops[{i}].style"),
                ));
            }
        }
    }
    let part_name = part_for_para(pkg, catalog, doc_part, &op.para, &op.id, i)?;
    let doc = load_part(pkg, docs, &part_name)?;
    let el = find_insert_para_mut(&mut doc.root, &op.para).ok_or_else(|| {
        Error::with_path(
            ErrorKind::ParaNotFound { id: op.id.clone() },
            format!("ops[{i}].id"),
        )
    })?;
    match op.style.as_deref() {
        Some(s) => styles::bind_para(el, s),
        None => styles::unbind_para(el),
    }
    dirty.insert(part_name, ());
    Ok(())
}

fn apply_insert_style(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    op: &InsertStyleOp,
    i: usize,
) -> Result<(), Error> {
    let part = styles::ensure_part(pkg, docs, dirty, doc_part)?;
    let doc = load_part(pkg, docs, &part)?;
    styles::insert_style(
        &mut doc.root,
        &op.style_id,
        &op.ty,
        &op.name,
        op.based_on.as_deref(),
        i,
    )?;
    dirty.insert(part, ());
    Ok(())
}

fn apply_delete_style(
    pkg: &Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    op: &DeleteStyleOp,
    i: usize,
) -> Result<(), Error> {
    let Some(part) = styles::styles_part(pkg, doc_part)? else {
        return Err(styles::missing_style(&op.id, i));
    };
    let doc = load_part(pkg, docs, &part)?;
    styles::delete_style(&mut doc.root, &op.style_id, &op.id, i)?;
    dirty.insert(part, ());
    Ok(())
}

fn apply_set_run_style(
    pkg: &Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &SetRunStyleOp,
    i: usize,
) -> Result<(), Error> {
    if let Some(style_id) = op.style.as_deref() {
        let style_part = styles::styles_part(pkg, doc_part)?
            .ok_or_else(|| styles::missing_para_style(&format!("style:{style_id}"), i))?;
        {
            let style_doc = load_part(pkg, docs, &style_part)?;
            let Some(el) = styles::find_style(&style_doc.root, style_id) else {
                return Err(styles::missing_para_style(&format!("style:{style_id}"), i));
            };
            if !styles::is_character_style(el) {
                return Err(Error::with_path(
                    ErrorKind::SetRunStyleNeedsCharacter,
                    format!("ops[{i}].style"),
                ));
            }
        }
    }
    let part_name = part_for_para(pkg, catalog, doc_part, &op.para, &op.id, i)?;
    let doc = load_part(pkg, docs, &part_name)?;
    let el = find_insert_para_mut(&mut doc.root, &op.para).ok_or_else(|| {
        Error::with_path(
            ErrorKind::ParaNotFound { id: op.id.clone() },
            format!("ops[{i}].id"),
        )
    })?;
    let run = format::find_run_mut(el, op.run).ok_or_else(|| format::missing_run(&op.id, i))?;
    match op.style.as_deref() {
        Some(s) => format::bind_run_style(run, s),
        None => format::unbind_run_style(run),
    }
    dirty.insert(part_name, ());
    Ok(())
}

fn apply_set_table_style(
    pkg: &Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &SetTableStyleOp,
    i: usize,
) -> Result<(), Error> {
    if let Some(Some(style_id)) = op.style.as_ref() {
        let style_part = styles::styles_part(pkg, doc_part)?
            .ok_or_else(|| styles::missing_para_style(&format!("style:{style_id}"), i))?;
        {
            let style_doc = load_part(pkg, docs, &style_part)?;
            let Some(el) = styles::find_style(&style_doc.root, style_id) else {
                return Err(styles::missing_para_style(&format!("style:{style_id}"), i));
            };
            if !styles::is_table_style(el) {
                return Err(Error::with_path(
                    ErrorKind::SetTableStyleNeedsTable,
                    format!("ops[{i}].style"),
                ));
            }
        }
    }
    let part_name = tbl::part_for_story(catalog, doc_part, op.target.story(), &op.id, i)?;
    let doc = load_part(pkg, docs, &part_name)?;
    tbl::apply_set_table_style(
        &mut doc.root,
        &op.target,
        op.style.as_ref().map(|s| s.as_deref()),
        op.look.as_ref().map(|l| l.as_ref()),
        &op.id,
        i,
    )?;
    dirty.insert(part_name, ());
    Ok(())
}

fn apply_set_table_float(
    pkg: &Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &SetTableFloatOp,
    i: usize,
) -> Result<(), Error> {
    let part_name = tbl::part_for_story(catalog, doc_part, op.target.story(), &op.id, i)?;
    let doc = load_part(pkg, docs, &part_name)?;
    tbl::apply_set_table_float(&mut doc.root, &op.target, op.pos.as_ref(), &op.id, i)?;
    dirty.insert(part_name, ());
    Ok(())
}

fn apply_set_table_width(
    pkg: &Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &SetTableWidthOp,
    i: usize,
) -> Result<(), Error> {
    let part_name = tbl::part_for_story(catalog, doc_part, op.target.story(), &op.id, i)?;
    let doc = load_part(pkg, docs, &part_name)?;
    tbl::apply_set_table_width(&mut doc.root, &op.target, &op.patch, &op.id, i)?;
    dirty.insert(part_name, ());
    Ok(())
}

fn apply_set_table_align(
    pkg: &Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &SetTableAlignOp,
    i: usize,
) -> Result<(), Error> {
    let part_name = tbl::part_for_story(catalog, doc_part, op.target.story(), &op.id, i)?;
    let doc = load_part(pkg, docs, &part_name)?;
    tbl::apply_set_table_align(
        &mut doc.root,
        &op.target,
        op.align.as_ref().map(|a| a.as_deref()),
        op.indent,
        &op.id,
        i,
    )?;
    dirty.insert(part_name, ());
    Ok(())
}

fn apply_set_table_row_height(
    pkg: &Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &SetTableRowHeightOp,
    i: usize,
) -> Result<(), Error> {
    let part_name = tbl::part_for_story(catalog, doc_part, op.target.story(), &op.id, i)?;
    let doc = load_part(pkg, docs, &part_name)?;
    tbl::apply_set_table_row_height(
        &mut doc.root,
        &op.target,
        op.h,
        op.rule.as_deref(),
        &op.id,
        i,
    )?;
    dirty.insert(part_name, ());
    Ok(())
}

fn apply_set_table_borders(
    pkg: &Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &SetTableBordersOp,
    i: usize,
) -> Result<(), Error> {
    let part_name = tbl::part_for_story(catalog, doc_part, op.target.story(), &op.id, i)?;
    let doc = load_part(pkg, docs, &part_name)?;
    tbl::apply_set_table_borders(&mut doc.root, &op.target, op.borders.as_ref(), &op.id, i)?;
    dirty.insert(part_name, ());
    Ok(())
}

fn apply_set_cell_borders(
    pkg: &Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &SetCellBordersOp,
    i: usize,
) -> Result<(), Error> {
    let part_name = tbl::part_for_story(catalog, doc_part, op.target.story(), &op.id, i)?;
    let doc = load_part(pkg, docs, &part_name)?;
    tbl::apply_set_cell_borders(&mut doc.root, &op.target, op.borders.as_ref(), &op.id, i)?;
    dirty.insert(part_name, ());
    Ok(())
}

fn apply_set_table_cell_mar(
    pkg: &Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &SetTableCellMarOp,
    i: usize,
) -> Result<(), Error> {
    let part_name = tbl::part_for_story(catalog, doc_part, op.target.story(), &op.id, i)?;
    let doc = load_part(pkg, docs, &part_name)?;
    tbl::apply_set_table_cell_mar(
        &mut doc.root,
        &op.target,
        op.top,
        op.left,
        op.bottom,
        op.right,
        &op.id,
        i,
    )?;
    dirty.insert(part_name, ());
    Ok(())
}

fn apply_set_cell_format(
    pkg: &Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &SetCellFormatOp,
    i: usize,
) -> Result<(), Error> {
    let part_name = tbl::part_for_story(catalog, doc_part, op.target.story(), &op.id, i)?;
    let doc = load_part(pkg, docs, &part_name)?;
    tbl::apply_set_cell_format(
        &mut doc.root,
        &op.target,
        op.fill.as_ref().map(|s| s.as_deref()),
        op.top,
        op.left,
        op.bottom,
        op.right,
        op.tl2br.as_ref().map(|s| s.as_ref()),
        op.tr2bl.as_ref().map(|s| s.as_ref()),
        op.v_align.as_ref().map(|s| s.as_deref()),
        op.no_wrap,
        op.text_direction.as_ref().map(|s| s.as_deref()),
        op.fit_text,
        op.hide_mark,
        op.headers.as_ref().map(|s| s.as_deref()),
        &op.id,
        i,
    )?;
    dirty.insert(part_name, ());
    Ok(())
}

fn apply_insert_table(
    pkg: &Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &InsertTableOp,
    i: usize,
) -> Result<(), Error> {
    let part_name = tbl::part_for_anchor(catalog, doc_part, &op.anchor, &op.id, i)?;
    let doc = load_part(pkg, docs, &part_name)?;
    tbl::apply_insert_table(&mut doc.root, &op.anchor, &op.cells, &op.id, i)?;
    dirty.insert(part_name, ());
    Ok(())
}

fn apply_delete_table(
    pkg: &Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &DeleteTableOp,
    i: usize,
) -> Result<(), Error> {
    let part_name = tbl::part_for_story(catalog, doc_part, op.target.story(), &op.id, i)?;
    let doc = load_part(pkg, docs, &part_name)?;
    tbl::apply_delete_table(&mut doc.root, &op.target, &op.id, i)?;
    dirty.insert(part_name, ());
    Ok(())
}

fn apply_insert_row(
    pkg: &Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &InsertRowOp,
    i: usize,
) -> Result<(), Error> {
    let part_name = tbl::part_for_story(catalog, doc_part, op.target.story(), &op.id, i)?;
    let doc = load_part(pkg, docs, &part_name)?;
    tbl::apply_insert_row(&mut doc.root, &op.target, &op.id, i)?;
    dirty.insert(part_name, ());
    Ok(())
}

fn apply_delete_row(
    pkg: &Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &DeleteRowOp,
    i: usize,
) -> Result<(), Error> {
    let part_name = tbl::part_for_story(catalog, doc_part, op.target.story(), &op.id, i)?;
    let doc = load_part(pkg, docs, &part_name)?;
    tbl::apply_delete_row(&mut doc.root, &op.target, &op.id, i)?;
    dirty.insert(part_name, ());
    Ok(())
}

fn apply_insert_col(
    pkg: &Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &InsertColOp,
    i: usize,
) -> Result<(), Error> {
    let part_name = tbl::part_for_story(catalog, doc_part, op.target.story(), &op.id, i)?;
    let doc = load_part(pkg, docs, &part_name)?;
    tbl::apply_insert_col(&mut doc.root, &op.target, op.after, &op.id, i)?;
    dirty.insert(part_name, ());
    Ok(())
}

fn apply_delete_col(
    pkg: &Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &DeleteColOp,
    i: usize,
) -> Result<(), Error> {
    let part_name = tbl::part_for_story(catalog, doc_part, op.target.story(), &op.id, i)?;
    let doc = load_part(pkg, docs, &part_name)?;
    tbl::apply_delete_col(&mut doc.root, &op.target, op.col, &op.id, i)?;
    dirty.insert(part_name, ());
    Ok(())
}

fn apply_merge_cells(
    pkg: &Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &MergeCellsOp,
    i: usize,
) -> Result<(), Error> {
    let part_name = tbl::part_for_story(catalog, doc_part, op.target.story(), &op.id, i)?;
    let doc = load_part(pkg, docs, &part_name)?;
    tbl::apply_merge(
        &mut doc.root,
        &op.target,
        op.start_row,
        op.start_col,
        op.end_row,
        op.end_col,
        &op.id,
        i,
    )?;
    dirty.insert(part_name, ());
    Ok(())
}

fn apply_split_cells(
    pkg: &Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &SplitCellsOp,
    i: usize,
) -> Result<(), Error> {
    let part_name = tbl::part_for_story(catalog, doc_part, op.target.story(), &op.id, i)?;
    let doc = load_part(pkg, docs, &part_name)?;
    tbl::apply_split(&mut doc.root, &op.target, &op.id, i)?;
    dirty.insert(part_name, ());
    Ok(())
}

fn apply_set_para_format(
    pkg: &Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &SetParaFormatOp,
    i: usize,
) -> Result<(), Error> {
    let part_name = part_for_format_para(pkg, catalog, doc_part, &op.para, &op.id, i)?;
    let doc = load_part(pkg, docs, &part_name)?;
    let el = find_insert_para_mut(&mut doc.root, &op.para).ok_or_else(|| {
        Error::with_path(
            ErrorKind::ParaNotFound { id: op.id.clone() },
            format!("ops[{i}].id"),
        )
    })?;
    format::apply_para(
        el,
        format::ParaFormatPatch {
            align: op.align.as_ref().map(|v| v.as_deref()),
            left: op.left,
            right: op.right,
            first_line: op.first_line,
            hanging: op.hanging,
            before: op.before,
            after: op.after,
            line: op.line,
            line_rule: op.line_rule.as_ref().map(|v| v.as_deref()),
            kinsoku: op.kinsoku,
            overflow_punct: op.overflow_punct,
            top_line_punct: op.top_line_punct,
            auto_space_de: op.auto_space_de,
            auto_space_dn: op.auto_space_dn,
            borders: op.borders.as_ref().map(|v| v.as_ref()),
            shd: op.shd.as_ref().map(|v| v.as_deref()),
            tabs: op.tabs.as_ref().map(|v| v.as_deref()),
        },
    );
    dirty.insert(part_name, ());
    Ok(())
}

fn apply_set_run_format(
    pkg: &Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &SetRunFormatOp,
    i: usize,
) -> Result<(), Error> {
    let part_name = part_for_format_para(pkg, catalog, doc_part, &op.para, &op.id, i)?;
    let doc = load_part(pkg, docs, &part_name)?;
    let el = find_insert_para_mut(&mut doc.root, &op.para).ok_or_else(|| {
        Error::with_path(
            ErrorKind::ParaNotFound { id: op.id.clone() },
            format!("ops[{i}].id"),
        )
    })?;
    let run = format::find_run_mut(el, op.run).ok_or_else(|| format::missing_run(&op.id, i))?;
    format::apply_run(
        run,
        format::RunFormatPatch {
            bold: op.bold,
            italic: op.italic,
            underline: op.underline.as_ref().map(|v| v.as_deref()),
            sz: op.sz,
            color: op.color.as_deref(),
            font: op.font.as_deref(),
            east_asia_font: op.east_asia_font.as_deref(),
            em: op.em.as_ref().map(|v| v.as_deref()),
            east_asian_layout: op.east_asian_layout.as_ref().map(|v| {
                v.as_ref().map(|s| format::EastAsianLayout {
                    id: s.id,
                    combine: s.combine,
                    combine_brackets: s.combine_brackets.clone(),
                    vert: s.vert,
                    vert_compress: s.vert_compress,
                })
            }),
            highlight: op.highlight.as_ref().map(|v| v.as_deref()),
            strike: op.strike,
            dstrike: op.dstrike,
            vert_align: op.vert_align.as_ref().map(|v| v.as_deref()),
            spacing: op.spacing,
            kern: op.kern,
            scale: op.scale,
        },
    );
    dirty.insert(part_name, ());
    Ok(())
}

fn apply_para_text(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &SetParaTextOp,
    i: usize,
) -> Result<(), Error> {
    let target = para::parse_para_id(&op.id, i)?;
    match &target {
        ParaTarget::Body(p) => {
            let doc = load_part(pkg, docs, doc_part)?;
            refuse_set_text_if_field(&doc.root, p, i)?;
            refuse_set_text_if_math(&doc.root, p, i)?;
            refuse_set_text_if_ruby(&doc.root, p, i)?;
            refuse_set_text_if_sdt(&doc.root, p, i)?;
            let el = para::find_paragraph_mut(&mut doc.root, p).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::ParaNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )
            })?;
            refuse_set_text_if_revision(el, i)?;
            refuse_set_text_if_hyperlink(el, i)?;
            para::set_visible_text(el, &op.text);
            dirty.insert(doc_part.to_string(), ());
        }
        ParaTarget::HeaderFooter {
            kind,
            hf_type,
            n,
            para,
        } => {
            let part_name = part_for_hf(catalog, *kind, *hf_type, *n, &op.id, i)?;
            let doc = load_part(pkg, docs, &part_name)?;
            refuse_set_text_if_field(&doc.root, para, i)?;
            refuse_set_text_if_math(&doc.root, para, i)?;
            refuse_set_text_if_ruby(&doc.root, para, i)?;
            refuse_set_text_if_sdt(&doc.root, para, i)?;
            let el = para::find_paragraph_mut(&mut doc.root, para).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::ParaNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )
            })?;
            refuse_set_text_if_revision(el, i)?;
            refuse_set_text_if_hyperlink(el, i)?;
            para::set_visible_text(el, &op.text);
            dirty.insert(part_name, ());
        }
        ParaTarget::InTextBox { host, key, para } => {
            let part_name = part_for_host(pkg, catalog, doc_part, host, &op.id, i)?;
            let doc = load_part(pkg, docs, &part_name)?;
            let host_p = para::find_paragraph_mut(&mut doc.root, host.body()).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::ParaNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )
            })?;
            let content = textbox::find_txbx_content_by_key_mut(host_p, key).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::ParaNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )
            })?;
            refuse_set_text_if_field(content, para, i)?;
            refuse_set_text_if_math(content, para, i)?;
            refuse_set_text_if_ruby(content, para, i)?;
            refuse_set_text_if_sdt(content, para, i)?;
            let el = para::find_paragraph_in_txbx_mut(content, para).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::ParaNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )
            })?;
            refuse_set_text_if_revision(el, i)?;
            refuse_set_text_if_hyperlink(el, i)?;
            para::set_visible_text(el, &op.text);
            dirty.insert(part_name, ());
        }
        ParaTarget::InNote {
            kind,
            note_id,
            para,
        } => {
            let part_name = notes::part_name(pkg, doc_part, *kind)?.ok_or_else(|| {
                Error::with_path(
                    ErrorKind::ParaNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )
            })?;
            let doc = load_part(pkg, docs, &part_name)?;
            let note =
                notes::find_user_note_mut(&mut doc.root, *kind, note_id).ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::ParaNotFound { id: op.id.clone() },
                        format!("ops[{i}].id"),
                    )
                })?;
            let el = para::find_paragraph_mut(note, para).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::ParaNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )
            })?;
            refuse_set_text_if_revision(el, i)?;
            para::set_visible_text(el, &op.text);
            dirty.insert(part_name, ());
        }
        ParaTarget::InComment { comment_id, para } => {
            let part_name = comments::part_name(pkg, doc_part)?.ok_or_else(|| {
                Error::with_path(
                    ErrorKind::ParaNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )
            })?;
            let doc = load_part(pkg, docs, &part_name)?;
            let comment =
                comments::find_comment_mut(&mut doc.root, comment_id).ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::ParaNotFound { id: op.id.clone() },
                        format!("ops[{i}].id"),
                    )
                })?;
            comments::reject_ink(comment, &op.id, i)?;
            let el = para::find_paragraph_mut(comment, para).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::ParaNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )
            })?;
            refuse_set_text_if_revision(el, i)?;
            para::set_visible_text(el, &op.text);
            dirty.insert(part_name, ());
        }
    }
    Ok(())
}

fn apply_hf_distance(root: &mut Element, op: &SetHfDistanceOp, i: usize) -> Result<(), Error> {
    let sect = hf::find_sect_pr_mut(root, op.n).ok_or_else(|| {
        Error::with_path(
            ErrorKind::SectionNotFound { id: op.id.clone() },
            format!("ops[{i}].id"),
        )
    })?;
    let has_pg = sect.child_by_local("pgMar").is_some();
    if !has_pg {
        let pg = hf::ensure_child_in_order(sect, "pgMar", hf::TITLE_PG_ORDER);
        if let Some(h) = op.header_distance {
            hf::set_twip_attr(pg, "header", h);
        }
        if let Some(f) = op.footer_distance {
            hf::set_twip_attr(pg, "footer", f);
        }
        return Ok(());
    }
    let pg = sect.child_by_local_mut("pgMar").unwrap();
    if let Some(h) = op.header_distance {
        hf::set_twip_attr(pg, "header", h);
    }
    if let Some(f) = op.footer_distance {
        hf::set_twip_attr(pg, "footer", f);
    }
    Ok(())
}

fn apply_hf_link(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    removed: &mut HashSet<String>,
    doc_part: &str,
    op: &SetHfLinkOp,
    i: usize,
) -> Result<(), Error> {
    let sect_count = hf::collect_sect_prs(&docs.get(doc_part).unwrap().root).len();
    if op.n > sect_count {
        return Err(Error::with_path(
            ErrorKind::SectionNotFound { id: op.id.clone() },
            format!("ops[{i}].id"),
        ));
    }

    let even_and_odd = hf::even_and_odd_cached(pkg, docs, doc_part)?;
    let title_pg = {
        let sects = hf::collect_sect_prs(&docs.get(doc_part).unwrap().root);
        hf::bool_prop(sects[op.n - 1], "titlePg")
    };
    if !hf::type_is_active(op.hf_type, title_pg, even_and_odd) {
        return Err(Error::with_path(
            ErrorKind::HfTypeInactive {
                kind: op.kind.as_str().into(),
                hf_type: op.hf_type.as_str().into(),
            },
            format!("ops[{i}].type"),
        ));
    }

    if op.n == 1 && op.linked {
        return Err(Error::with_path(
            ErrorKind::CannotLinkFirstSection,
            format!("ops[{i}].linked"),
        ));
    }

    let has_explicit = {
        let sects = hf::collect_sect_prs(&docs.get(doc_part).unwrap().root);
        hf::has_explicit_ref(sects[op.n - 1], op.kind, op.hf_type)
    };
    if (op.linked && !has_explicit) || (!op.linked && has_explicit) {
        return Ok(());
    }

    if op.linked {
        relink_section(pkg, docs, dirty, removed, doc_part, op)?;
    } else {
        unlink_section(pkg, docs, dirty, doc_part, op)?;
    }
    Ok(())
}

fn refresh_hf_catalog(
    pkg: &Package,
    docs: &HashMap<String, Document>,
    doc_part: &str,
    catalog: &mut Vec<hf::HfPart>,
) -> Result<(), Error> {
    *catalog = hf::catalog(pkg, doc_part, &docs.get(doc_part).unwrap().root)?;
    Ok(())
}

fn apply_insert_hf(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    op: &InsertHeaderFooterOp,
    i: usize,
) -> Result<(), Error> {
    let sect_count = hf::collect_sect_prs(&docs.get(doc_part).unwrap().root).len();
    if op.n > sect_count {
        return Err(Error::with_path(
            ErrorKind::SectionNotFound { id: op.id.clone() },
            format!("ops[{i}].id"),
        ));
    }

    let even_and_odd = hf::even_and_odd_cached(pkg, docs, doc_part)?;
    let title_pg = {
        let sects = hf::collect_sect_prs(&docs.get(doc_part).unwrap().root);
        hf::bool_prop(sects[op.n - 1], "titlePg")
    };
    if !hf::type_is_active(op.hf_type, title_pg, even_and_odd) {
        return Err(Error::with_path(
            ErrorKind::HfTypeInactive {
                kind: op.kind.as_str().into(),
                hf_type: op.hf_type.as_str().into(),
            },
            format!("ops[{i}].type"),
        ));
    }

    let has_explicit = {
        let sects = hf::collect_sect_prs(&docs.get(doc_part).unwrap().root);
        hf::has_explicit_ref(sects[op.n - 1], op.kind, op.hf_type)
    };
    if has_explicit {
        return Ok(());
    }
    if op.n > 1 {
        return Err(Error::with_path(
            ErrorKind::HfInsertWhileLinked,
            format!("ops[{i}].id"),
        ));
    }

    let new_part = hf::next_unused_hf_part(pkg, op.kind);
    pkg.set(&new_part, hf::minimal_hf_xml(op.kind))?;

    let mut rels = Relationships::from_package(pkg, doc_part)?;
    let rid = rels.next_rid();
    rels.push(Relationship {
        id: rid.clone(),
        rel_type: match op.kind {
            HfKind::Header => REL_HEADER,
            HfKind::Footer => REL_FOOTER,
        }
        .into(),
        target: hf::same_dir_rel_target(doc_part, &new_part),
        target_mode: None,
        extra: Vec::new(),
    });
    rels.write_to(pkg, doc_part)?;

    let mut cts = ContentTypes::from_package(pkg)?;
    let ct = match op.kind {
        HfKind::Header => CT_HEADER,
        HfKind::Footer => CT_FOOTER,
    };
    cts.set_override(&new_part, ct)?;
    cts.write_to(pkg)?;

    let doc = docs.get_mut(doc_part).unwrap();
    let sect = hf::find_sect_pr_mut(&mut doc.root, op.n).unwrap();
    let rid_attr = hf::rid_attr_name(sect);
    hf::insert_hf_ref(sect, op.kind, op.hf_type, &rid_attr, &rid);
    dirty.insert(doc_part.to_string(), ());
    Ok(())
}

fn apply_title_page(
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    op: &SetTitlePageOp,
    i: usize,
) -> Result<(), Error> {
    let sect_count = hf::collect_sect_prs(&docs.get(doc_part).unwrap().root).len();
    if op.n > sect_count {
        return Err(Error::with_path(
            ErrorKind::SectionNotFound { id: op.id.clone() },
            format!("ops[{i}].id"),
        ));
    }
    let current = {
        let sects = hf::collect_sect_prs(&docs.get(doc_part).unwrap().root);
        hf::bool_prop(sects[op.n - 1], "titlePg")
    };
    if current == op.title_page {
        return Ok(());
    }
    let doc = docs.get_mut(doc_part).unwrap();
    let sect = hf::find_sect_pr_mut(&mut doc.root, op.n).unwrap();
    hf::set_on_off(sect, "titlePg", op.title_page, hf::TITLE_PG_ORDER);
    dirty.insert(doc_part.to_string(), ());
    Ok(())
}

fn find_sect_for_apply<'a>(
    docs: &'a mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    n: usize,
    id: &str,
    i: usize,
) -> Result<&'a mut Element, Error> {
    let doc = docs.get_mut(doc_part).unwrap();
    let sect = hf::find_sect_pr_mut(&mut doc.root, n).ok_or_else(|| {
        Error::with_path(
            ErrorKind::SectionNotFound { id: id.to_string() },
            format!("ops[{i}].id"),
        )
    })?;
    dirty.insert(doc_part.to_string(), ());
    Ok(sect)
}

fn apply_page_borders(
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    op: &SetPageBordersOp,
    i: usize,
) -> Result<(), Error> {
    let sect = find_sect_for_apply(docs, dirty, doc_part, op.n, &op.id, i)?;
    page_bg::set_borders(
        sect,
        op.enabled,
        op.offset_from.as_deref(),
        op.display.as_deref(),
        op.z_order.as_deref(),
        op.top.as_ref().map(|s| s.as_ref()),
        op.left.as_ref().map(|s| s.as_ref()),
        op.bottom.as_ref().map(|s| s.as_ref()),
        op.right.as_ref().map(|s| s.as_ref()),
    );
    Ok(())
}

fn apply_set_form_prot(
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    op: &SetFormProtOp,
    i: usize,
) -> Result<(), Error> {
    let sect = find_sect_for_apply(docs, dirty, doc_part, op.n, &op.id, i)?;
    hf::set_form_prot(sect, op.form_prot);
    Ok(())
}

fn apply_page_margin(
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    op: &SetPageMarginOp,
    i: usize,
) -> Result<(), Error> {
    let sect = find_sect_for_apply(docs, dirty, doc_part, op.n, &op.id, i)?;
    sect::apply_page_margin(sect, op.top, op.bottom, op.left, op.right, op.gutter);
    Ok(())
}

fn apply_page_size(
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    op: &SetPageSizeOp,
    i: usize,
) -> Result<(), Error> {
    let sect = find_sect_for_apply(docs, dirty, doc_part, op.n, &op.id, i)?;
    sect::apply_page_size(sect, op.page_width, op.page_height, op.orient.as_deref());
    Ok(())
}

fn apply_columns(
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    op: &SetColumnsOp,
    i: usize,
) -> Result<(), Error> {
    let items: Option<Vec<sect::ColItem>> = op.cols.as_ref().map(|v| {
        v.iter()
            .map(|c| sect::ColItem {
                width: c.width,
                space: c.space,
            })
            .collect()
    });
    let sect = find_sect_for_apply(docs, dirty, doc_part, op.n, &op.id, i)?;
    sect::apply_columns(
        sect,
        op.num,
        op.space,
        op.equal_width,
        op.sep,
        items.as_deref(),
        op.enabled,
    );
    Ok(())
}

fn apply_valign(
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    op: &SetVerticalAlignOp,
    i: usize,
) -> Result<(), Error> {
    let sect = find_sect_for_apply(docs, dirty, doc_part, op.n, &op.id, i)?;
    sect::apply_valign(sect, op.v_align.as_deref());
    Ok(())
}

fn apply_line_numbering(
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    op: &SetLineNumberingOp,
    i: usize,
) -> Result<(), Error> {
    let sect = find_sect_for_apply(docs, dirty, doc_part, op.n, &op.id, i)?;
    sect::apply_line_numbering(
        sect,
        op.count_by,
        op.start,
        op.distance,
        op.restart.as_deref(),
        op.enabled,
    );
    Ok(())
}

fn apply_doc_grid(
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    op: &SetDocGridOp,
    i: usize,
) -> Result<(), Error> {
    let sect = find_sect_for_apply(docs, dirty, doc_part, op.n, &op.id, i)?;
    sect::apply_doc_grid(
        sect,
        op.grid_type.as_deref(),
        op.line_pitch,
        op.char_space,
        op.enabled,
    );
    Ok(())
}

fn apply_rtl_gutter(
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    op: &SetRtlGutterOp,
    i: usize,
) -> Result<(), Error> {
    let sect = find_sect_for_apply(docs, dirty, doc_part, op.n, &op.id, i)?;
    sect::apply_rtl_gutter(sect, op.rtl_gutter);
    Ok(())
}

fn apply_mirror_margins(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    op: &SetMirrorMarginsOp,
    i: usize,
) -> Result<(), Error> {
    apply_settings_flag(
        pkg,
        docs,
        dirty,
        doc_part,
        "mirrorMargins",
        op.mirror_margins,
        i,
    )
}

fn apply_gutter_at_top(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    op: &SetGutterAtTopOp,
    i: usize,
) -> Result<(), Error> {
    apply_settings_flag(
        pkg,
        docs,
        dirty,
        doc_part,
        "gutterAtTop",
        op.gutter_at_top,
        i,
    )
}

fn apply_settings_flag(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    local: &str,
    on: bool,
    i: usize,
) -> Result<(), Error> {
    let settings = notes::settings_part_name(pkg, doc_part)?.ok_or_else(|| {
        Error::with_path(
            ErrorKind::PartMissing {
                name: "/word/settings.xml".into(),
            },
            format!("ops[{i}].id"),
        )
    })?;
    let doc = load_part(pkg, docs, &settings)?;
    sect::apply_settings_flag(&mut doc.root, local, on);
    dirty.insert(settings, ());
    Ok(())
}

fn apply_even_and_odd(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    op: &SetEvenAndOddHeadersOp,
    i: usize,
) -> Result<(), Error> {
    let settings = notes::settings_part_name(pkg, doc_part)?.ok_or_else(|| {
        Error::with_path(
            ErrorKind::PartMissing {
                name: "/word/settings.xml".into(),
            },
            format!("ops[{i}].id"),
        )
    })?;
    let current = hf::even_and_odd_cached(pkg, docs, doc_part)?;
    if current == op.even_and_odd {
        return Ok(());
    }
    let doc = load_part(pkg, docs, &settings)?;
    hf::set_on_off(
        &mut doc.root,
        "evenAndOddHeaders",
        op.even_and_odd,
        hf::EVEN_ODD_ORDER,
    );
    dirty.insert(settings.clone(), ());
    pkg.set(&settings, docs.get(&settings).unwrap().to_bytes()?)?;
    Ok(())
}

fn unlink_section(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    op: &SetHfLinkOp,
) -> Result<(), Error> {
    let inherited = hf::inherited_part_path(
        pkg,
        doc_part,
        &docs.get(doc_part).unwrap().root,
        op.n,
        op.kind,
        op.hf_type,
    )?;
    let bytes = match inherited {
        Some(ref src) => current_part_bytes(pkg, docs, src)?,
        None => hf::minimal_hf_xml(op.kind),
    };
    let new_part = hf::next_unused_hf_part(pkg, op.kind);
    pkg.set(&new_part, bytes)?;
    if let Some(src) = &inherited {
        picture::copy_part_rels(pkg, src, &new_part)?;
    }

    let mut rels = Relationships::from_package(pkg, doc_part)?;
    let rid = rels.next_rid();
    rels.push(Relationship {
        id: rid.clone(),
        rel_type: match op.kind {
            HfKind::Header => REL_HEADER,
            HfKind::Footer => REL_FOOTER,
        }
        .into(),
        target: hf::same_dir_rel_target(doc_part, &new_part),
        target_mode: None,
        extra: Vec::new(),
    });
    rels.write_to(pkg, doc_part)?;

    let mut cts = ContentTypes::from_package(pkg)?;
    let ct = match op.kind {
        HfKind::Header => CT_HEADER,
        HfKind::Footer => CT_FOOTER,
    };
    cts.set_override(&new_part, ct)?;
    cts.write_to(pkg)?;

    let doc = docs.get_mut(doc_part).unwrap();
    let sect = hf::find_sect_pr_mut(&mut doc.root, op.n).unwrap();
    let rid_attr = hf::rid_attr_name(sect);
    hf::insert_hf_ref(sect, op.kind, op.hf_type, &rid_attr, &rid);
    dirty.insert(doc_part.to_string(), ());
    Ok(())
}

fn relink_section(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    removed: &mut HashSet<String>,
    doc_part: &str,
    op: &SetHfLinkOp,
) -> Result<(), Error> {
    let old_part = hf::explicit_part_path(
        pkg,
        doc_part,
        &docs.get(doc_part).unwrap().root,
        op.n,
        op.kind,
        op.hf_type,
    )?;

    let doc = docs.get_mut(doc_part).unwrap();
    let sect = hf::find_sect_pr_mut(&mut doc.root, op.n).unwrap();
    hf::remove_hf_ref(sect, op.kind, op.hf_type);
    dirty.insert(doc_part.to_string(), ());

    let Some(old) = old_part else {
        return Ok(());
    };
    let still = hf::catalog(pkg, doc_part, &docs.get(doc_part).unwrap().root)?;
    if still.iter().any(|h| h.part == old) {
        return Ok(());
    }

    let mut rels = Relationships::from_package(pkg, doc_part)?;
    rels.remove_by_target_part(doc_part, &old);
    rels.write_to(pkg, doc_part)?;

    let mut cts = ContentTypes::from_package(pkg)?;
    cts.remove_override(&old);
    cts.write_to(pkg)?;

    pkg.remove(&old);
    pkg.remove(&rels_part_name(&old));
    removed.insert(old.clone());
    docs.remove(&old);
    dirty.remove(&old);
    Ok(())
}

fn apply_insert_section(
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    op: &InsertSectionOp,
    i: usize,
) -> Result<(), Error> {
    let doc = docs.get_mut(doc_part).unwrap();
    let after = match &op.place {
        InsertSectionPlace::After(p) => Some(p),
        InsertSectionPlace::EndOfDocument => None,
    };
    sect::insert_section_break(&mut doc.root, after, op.break_type, &op.id, i)?;
    dirty.insert(doc_part.to_string(), ());
    Ok(())
}

fn apply_insert_toc(
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    op: &InsertTocOp,
    i: usize,
) -> Result<(), Error> {
    let doc = docs.get_mut(doc_part).unwrap();
    let after = match &op.place {
        InsertTocPlace::After(p) => Some(p),
        InsertTocPlace::EndOfDocument => None,
    };
    field::insert_toc(&mut doc.root, after, &op.instr, &op.id, i)?;
    dirty.insert(doc_part.to_string(), ());
    Ok(())
}

fn apply_delete_toc(
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    op: &DeleteTocOp,
    i: usize,
) -> Result<(), Error> {
    let doc = docs.get_mut(doc_part).unwrap();
    field::delete_toc(&mut doc.root, op.n, &op.id, i)?;
    dirty.insert(doc_part.to_string(), ());
    Ok(())
}

fn apply_insert_caption(
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    op: &InsertCaptionOp,
    i: usize,
) -> Result<(), Error> {
    let doc = docs.get_mut(doc_part).unwrap();
    let after = match &op.place {
        InsertCaptionPlace::After(p) => Some(p),
        InsertCaptionPlace::EndOfDocument => None,
    };
    field::insert_caption(
        &mut doc.root,
        after,
        &op.label,
        op.title.as_deref(),
        op.bookmark.as_deref(),
        &op.id,
        i,
    )?;
    dirty.insert(doc_part.to_string(), ());
    Ok(())
}

fn apply_delete_caption(
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    op: &DeleteCaptionOp,
    i: usize,
) -> Result<(), Error> {
    let doc = docs.get_mut(doc_part).unwrap();
    field::delete_caption(&mut doc.root, op.n, &op.id, i)?;
    dirty.insert(doc_part.to_string(), ());
    Ok(())
}

fn apply_insert_cross_ref(
    pkg: &Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &InsertCrossRefOp,
    i: usize,
) -> Result<(), Error> {
    {
        let main = docs.get(doc_part).unwrap();
        if !link::all_bookmark_names(&main.root).contains(&op.bookmark) {
            return Err(Error::with_path(
                ErrorKind::BookmarkNotFound {
                    id: op.bookmark.clone(),
                },
                format!("ops[{i}].bookmark"),
            ));
        }
    }
    let part_name = part_for_para(pkg, catalog, doc_part, &op.para, &op.id, i)?;
    let doc = load_part(pkg, docs, &part_name)?;
    let el = find_insert_para_mut(&mut doc.root, &op.para).ok_or_else(|| {
        Error::with_path(
            ErrorKind::ParaNotFound { id: op.id.clone() },
            format!("ops[{i}].id"),
        )
    })?;
    field::insert_cross_ref(el, &op.bookmark, op.kind);
    dirty.insert(part_name, ());
    Ok(())
}

fn apply_delete_cross_ref(
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    op: &DeleteCrossRefOp,
    i: usize,
) -> Result<(), Error> {
    let doc = docs.get_mut(doc_part).unwrap();
    field::delete_cross_ref(&mut doc.root, op.n, &op.id, i)?;
    dirty.insert(doc_part.to_string(), ());
    Ok(())
}

fn apply_insert_index(
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    op: &InsertIndexOp,
    i: usize,
) -> Result<(), Error> {
    let doc = docs.get_mut(doc_part).unwrap();
    let after = match &op.place {
        InsertIndexPlace::After(p) => Some(p),
        InsertIndexPlace::EndOfDocument => None,
    };
    field::insert_index(&mut doc.root, after, &op.instr, &op.id, i)?;
    dirty.insert(doc_part.to_string(), ());
    Ok(())
}

fn apply_delete_index(
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    op: &DeleteIndexOp,
    i: usize,
) -> Result<(), Error> {
    let doc = docs.get_mut(doc_part).unwrap();
    field::delete_index(&mut doc.root, op.n, &op.id, i)?;
    dirty.insert(doc_part.to_string(), ());
    Ok(())
}

fn apply_insert_index_entry(
    pkg: &Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &InsertIndexEntryOp,
    i: usize,
) -> Result<(), Error> {
    let part_name = part_for_para(pkg, catalog, doc_part, &op.para, &op.id, i)?;
    let doc = load_part(pkg, docs, &part_name)?;
    let el = find_insert_para_mut(&mut doc.root, &op.para).ok_or_else(|| {
        Error::with_path(
            ErrorKind::ParaNotFound { id: op.id.clone() },
            format!("ops[{i}].id"),
        )
    })?;
    field::insert_index_entry(el, &op.entry);
    dirty.insert(part_name, ());
    Ok(())
}

fn apply_delete_index_entry(
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    op: &DeleteIndexEntryOp,
    i: usize,
) -> Result<(), Error> {
    let doc = docs.get_mut(doc_part).unwrap();
    field::delete_index_entry(&mut doc.root, op.n, &op.id, i)?;
    dirty.insert(doc_part.to_string(), ());
    Ok(())
}

fn apply_insert_source(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    op: &InsertSourceOp,
    i: usize,
) -> Result<(), Error> {
    biblio::insert_source(
        pkg,
        docs,
        dirty,
        doc_part,
        &biblio::SourceWrite {
            tag: &op.tag,
            source_type: &op.source_type,
            title: &op.title,
            year: op.year.as_deref(),
            last: op.last.as_deref(),
            first: op.first.as_deref(),
            city: op.city.as_deref(),
            publisher: op.publisher.as_deref(),
        },
        i,
    )
}

fn apply_delete_source(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    op: &DeleteSourceOp,
    i: usize,
) -> Result<(), Error> {
    biblio::delete_source(pkg, docs, dirty, doc_part, op.n, &op.id, i)
}

fn apply_insert_citation(
    pkg: &Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &InsertCitationOp,
    i: usize,
) -> Result<(), Error> {
    if !biblio::has_tag(pkg, docs, doc_part, &op.tag)? {
        return Err(Error::with_path(
            ErrorKind::SourceNotFound { id: op.tag.clone() },
            format!("ops[{i}].tag"),
        ));
    }
    let part_name = part_for_para(pkg, catalog, doc_part, &op.para, &op.id, i)?;
    let doc = load_part(pkg, docs, &part_name)?;
    let xml_id = sdt::next_sdt_id(&doc.root);
    let el = find_insert_para_mut(&mut doc.root, &op.para).ok_or_else(|| {
        Error::with_path(
            ErrorKind::ParaNotFound { id: op.id.clone() },
            format!("ops[{i}].id"),
        )
    })?;
    field::insert_citation(el, &op.tag, xml_id);
    dirty.insert(part_name, ());
    Ok(())
}

fn apply_delete_citation(
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    op: &DeleteCitationOp,
    i: usize,
) -> Result<(), Error> {
    let doc = docs.get_mut(doc_part).unwrap();
    field::delete_citation(&mut doc.root, op.n, &op.id, i)?;
    dirty.insert(doc_part.to_string(), ());
    Ok(())
}

fn apply_insert_bibliography(
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    op: &InsertBibliographyOp,
    i: usize,
) -> Result<(), Error> {
    let doc = docs.get_mut(doc_part).unwrap();
    let after = match &op.place {
        InsertBibliographyPlace::After(p) => Some(p),
        InsertBibliographyPlace::EndOfDocument => None,
    };
    field::insert_bibliography(&mut doc.root, after, &op.instr, &op.id, i)?;
    dirty.insert(doc_part.to_string(), ());
    Ok(())
}

fn apply_delete_bibliography(
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    op: &DeleteBibliographyOp,
    i: usize,
) -> Result<(), Error> {
    let doc = docs.get_mut(doc_part).unwrap();
    field::delete_bibliography(&mut doc.root, op.n, &op.id, i)?;
    dirty.insert(doc_part.to_string(), ());
    Ok(())
}

fn apply_insert_doc_part(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    op: &InsertDocPartOp,
    i: usize,
) -> Result<(), Error> {
    glossary::insert_part(
        pkg,
        docs,
        dirty,
        doc_part,
        &glossary::DocPartWrite {
            name: &op.name,
            gallery: &op.gallery,
            category: &op.category,
            behavior: &op.behavior,
            text: &op.text,
        },
        i,
    )
}

fn apply_delete_doc_part(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    removed: &mut HashSet<String>,
    doc_part: &str,
    op: &DeleteDocPartOp,
    i: usize,
) -> Result<(), Error> {
    glossary::delete_part(pkg, docs, dirty, removed, doc_part, op.n, &op.id, i)
}

fn apply_insert_doc_part_content(
    pkg: &Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    op: &InsertDocPartContentOp,
    i: usize,
) -> Result<(), Error> {
    let after = match &op.place {
        InsertDocPartContentPlace::After(p) => Some(p),
        InsertDocPartContentPlace::EndOfDocument => None,
    };
    glossary::insert_content(
        pkg,
        docs,
        dirty,
        doc_part,
        after,
        op.n,
        &op.part_id,
        &op.id,
        i,
    )
}

fn apply_insert_math(
    pkg: &Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &InsertMathOp,
    i: usize,
) -> Result<(), Error> {
    match &op.place {
        InsertMathPlace::EndOfDocument => {
            let doc = docs.get_mut(doc_part).unwrap();
            math::ensure_math_ns(&mut doc.root);
            math::insert_math(
                &mut doc.root,
                None,
                op.kind,
                &op.body,
                op.jc.as_deref(),
                &op.id,
                i,
            )?;
            dirty.insert(doc_part.to_string(), ());
        }
        InsertMathPlace::After(target) => {
            let part_name = part_for_para(pkg, catalog, doc_part, target, &op.id, i)?;
            let doc = load_part(pkg, docs, &part_name)?;
            math::ensure_math_ns(&mut doc.root);
            let path = para_target_path(&doc.root, target).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::InvalidId {
                        reason: crate::error::IdReason::InsertMathNeedsPara,
                    },
                    format!("ops[{i}].id"),
                )
            })?;
            math::insert_math(
                &mut doc.root,
                Some(path.as_slice()),
                op.kind,
                &op.body,
                op.jc.as_deref(),
                &op.id,
                i,
            )?;
            dirty.insert(part_name, ());
        }
    }
    Ok(())
}

fn apply_set_math(
    pkg: &Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &SetMathOp,
    i: usize,
) -> Result<(), Error> {
    let part_name = math::part_for_story(catalog, doc_part, &op.target.story, || {
        math::missing_math_err(&op.id, i)
    })?;
    let doc = load_part(pkg, docs, &part_name)?;
    math::ensure_math_ns(&mut doc.root);
    let story = story_xml_mut(&mut doc.root, &op.target.story, || {
        math::missing_math_err(&op.id, i)
    })?;
    math::set_math(
        story,
        op.target.n,
        op.body.as_ref(),
        op.jc.as_ref().map(|v| v.as_deref()),
        &op.id,
        i,
    )?;
    dirty.insert(part_name, ());
    Ok(())
}

fn apply_delete_math(
    pkg: &Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &DeleteMathOp,
    i: usize,
) -> Result<(), Error> {
    let part_name = math::part_for_story(catalog, doc_part, &op.target.story, || {
        math::missing_math_err(&op.id, i)
    })?;
    let doc = load_part(pkg, docs, &part_name)?;
    let story = story_xml_mut(&mut doc.root, &op.target.story, || {
        math::missing_math_err(&op.id, i)
    })?;
    math::delete_math(story, op.target.n, &op.id, i)?;
    dirty.insert(part_name, ());
    Ok(())
}

fn apply_insert_ruby(
    pkg: &Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &InsertRubyOp,
    i: usize,
) -> Result<(), Error> {
    let spec = ruby::InsertSpec {
        base: &op.base,
        rt: &op.rt,
        align: &op.align,
        hps: op.hps,
        hps_raise: op.hps_raise,
        hps_base_text: op.hps_base_text,
        lid: &op.lid,
    };
    match &op.place {
        InsertRubyPlace::EndOfDocument => {
            let doc = docs.get_mut(doc_part).unwrap();
            ruby::insert_ruby(&mut doc.root, None, &spec, &op.id, i)?;
            dirty.insert(doc_part.to_string(), ());
        }
        InsertRubyPlace::After(target) => {
            let part_name = part_for_para(pkg, catalog, doc_part, target, &op.id, i)?;
            let doc = load_part(pkg, docs, &part_name)?;
            let path = para_target_path(&doc.root, target).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::InvalidId {
                        reason: crate::error::IdReason::InsertRubyNeedsPara,
                    },
                    format!("ops[{i}].id"),
                )
            })?;
            ruby::insert_ruby(&mut doc.root, Some(path.as_slice()), &spec, &op.id, i)?;
            dirty.insert(part_name, ());
        }
    }
    Ok(())
}

fn apply_set_ruby(
    pkg: &Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &SetRubyOp,
    i: usize,
) -> Result<(), Error> {
    let part_name = ruby::part_for_story(catalog, doc_part, &op.target.story, || {
        ruby::missing_ruby_err(&op.id, i)
    })?;
    let doc = load_part(pkg, docs, &part_name)?;
    let story = story_xml_mut(&mut doc.root, &op.target.story, || {
        ruby::missing_ruby_err(&op.id, i)
    })?;
    ruby::set_ruby(
        story,
        op.target.n,
        &ruby::SetPatch {
            base: op.base.as_deref(),
            rt: op.rt.as_deref(),
            align: op.align.as_deref(),
            hps: op.hps,
            hps_raise: op.hps_raise,
            hps_base_text: op.hps_base_text,
            lid: op.lid.as_deref(),
        },
        &op.id,
        i,
    )?;
    dirty.insert(part_name, ());
    Ok(())
}

fn apply_delete_ruby(
    pkg: &Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &DeleteRubyOp,
    i: usize,
) -> Result<(), Error> {
    let part_name = ruby::part_for_story(catalog, doc_part, &op.target.story, || {
        ruby::missing_ruby_err(&op.id, i)
    })?;
    let doc = load_part(pkg, docs, &part_name)?;
    let story = story_xml_mut(&mut doc.root, &op.target.story, || {
        ruby::missing_ruby_err(&op.id, i)
    })?;
    ruby::delete_ruby(story, op.target.n, &op.id, i)?;
    dirty.insert(part_name, ());
    Ok(())
}

fn apply_insert_sdt(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &InsertContentControlOp,
    i: usize,
) -> Result<(), Error> {
    let part_name = match &op.place {
        InsertSdtPlace::EndOfDocument => doc_part.to_string(),
        InsertSdtPlace::After(target) => part_for_para(pkg, catalog, doc_part, target, &op.id, i)?,
    };
    let drawing = if op.kind == sdt::SdtKind::Picture {
        let file = op.file.as_deref().ok_or_else(|| {
            Error::with_path(
                ErrorKind::OpRequiresField {
                    op: "insertContentControl".into(),
                    field: "file".into(),
                },
                format!("ops[{i}].file"),
            )
        })?;
        Some(sdt_inline_drawing(pkg, docs, &part_name, file)?)
    } else {
        None
    };
    let write = sdt::SdtWrite {
        kind: op.kind,
        text: &op.text,
        tag: op.tag.as_deref(),
        alias: op.alias.as_deref(),
        items: &op.items,
        full_date: op.full_date.as_deref(),
        checked: op.checked,
        drawing,
    };
    match &op.place {
        InsertSdtPlace::EndOfDocument => {
            let doc = docs.get_mut(doc_part).unwrap();
            sdt::insert_sdt(&mut doc.root, None, write, &op.id, i)?;
            dirty.insert(doc_part.to_string(), ());
        }
        InsertSdtPlace::After(target) => {
            let doc = load_part(pkg, docs, &part_name)?;
            let path = para_target_path(&doc.root, target).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::InvalidId {
                        reason: crate::error::IdReason::InsertSdtNeedsPara,
                    },
                    format!("ops[{i}].id"),
                )
            })?;
            sdt::insert_sdt(&mut doc.root, Some(path.as_slice()), write, &op.id, i)?;
            dirty.insert(part_name, ());
        }
    }
    Ok(())
}

fn apply_set_sdt(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &SetContentControlOp,
    i: usize,
) -> Result<(), Error> {
    let part_name = sdt::part_for_story(catalog, doc_part, &op.target.story, || {
        sdt::missing_sdt_err(&op.id, i)
    })?;
    if let Some(file) = &op.file {
        if op.text.is_some() {
            return Err(Error::with_path(
                ErrorKind::ContentControlFieldMismatch,
                format!("ops[{i}].text"),
            ));
        }
        if op.checked.is_some() {
            return Err(Error::with_path(
                ErrorKind::ContentControlFieldMismatch,
                format!("ops[{i}].checked"),
            ));
        }
        if op.items.is_some() {
            return Err(Error::with_path(
                ErrorKind::ContentControlFieldMismatch,
                format!("ops[{i}].items"),
            ));
        }
        if op.full_date.is_some() {
            return Err(Error::with_path(
                ErrorKind::ContentControlFieldMismatch,
                format!("ops[{i}].fullDate"),
            ));
        }
        let old_rids = {
            let doc = load_part(pkg, docs, &part_name)?;
            let story = story_xml_mut(&mut doc.root, &op.target.story, || {
                sdt::missing_sdt_err(&op.id, i)
            })?;
            sdt::sdt_picture_rids(story, op.target.n, &op.id, i)?
        };
        let old_media: Vec<String> = old_rids
            .iter()
            .filter_map(|r| picture::resolve_embed(pkg, &part_name, r).ok())
            .collect();
        let drawing = sdt_inline_drawing(pkg, docs, &part_name, file)?;
        let doc = load_part(pkg, docs, &part_name)?;
        let story = story_xml_mut(&mut doc.root, &op.target.story, || {
            sdt::missing_sdt_err(&op.id, i)
        })?;
        sdt::set_sdt_picture(
            story,
            op.target.n,
            drawing,
            op.tag.as_deref(),
            op.alias.as_deref(),
            &op.id,
            i,
        )?;
        dirty.insert(part_name.clone(), ());
        for r in &old_rids {
            picture::remove_rel_if_unused(pkg, &part_name, r, &docs.get(&part_name).unwrap().root)?;
        }
        for m in old_media {
            if !picture::media_still_used(pkg, &m)? {
                pkg.remove(&m);
            }
        }
        return Ok(());
    }
    let doc = load_part(pkg, docs, &part_name)?;
    let story = story_xml_mut(&mut doc.root, &op.target.story, || {
        sdt::missing_sdt_err(&op.id, i)
    })?;
    sdt::set_sdt(
        story,
        &sdt::SdtSet {
            n: op.target.n,
            text: op.text.as_deref(),
            tag: op.tag.as_deref(),
            alias: op.alias.as_deref(),
            items: op.items.as_deref(),
            full_date: op.full_date.as_deref(),
            checked: op.checked,
            id: &op.id,
            i,
        },
    )?;
    dirty.insert(part_name, ());
    Ok(())
}

/// 读本地图、挂关系到当前故事部件，并生成一张行内 DrawingML。
fn sdt_inline_drawing(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    part_name: &str,
    file: &str,
) -> Result<Element, Error> {
    let added = picture::add_media(pkg, part_name, file, None, None)?;
    let doc = load_part(pkg, docs, part_name)?;
    picture::ensure_drawing_ns(&mut doc.root);
    let doc_pr = picture::next_docpr_id(&doc.root);
    let name = format!("图片 {doc_pr}");
    Ok(picture::build_inline_drawing(
        doc_pr,
        &name,
        &added.blip_rid,
        added.svg_rid.as_deref(),
        added.cx,
        added.cy,
    ))
}

fn apply_delete_sdt(
    pkg: &Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &DeleteContentControlOp,
    i: usize,
) -> Result<(), Error> {
    let part_name = sdt::part_for_story(catalog, doc_part, &op.target.story, || {
        sdt::missing_sdt_err(&op.id, i)
    })?;
    let doc = load_part(pkg, docs, &part_name)?;
    let story = story_xml_mut(&mut doc.root, &op.target.story, || {
        sdt::missing_sdt_err(&op.id, i)
    })?;
    sdt::delete_sdt(story, op.target.n, op.keep, &op.id, i)?;
    dirty.insert(part_name, ());
    Ok(())
}

fn apply_delete_section(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    removed: &mut HashSet<String>,
    doc_part: &str,
    op: &DeleteSectionOp,
    i: usize,
) -> Result<(), Error> {
    let old_parts: Vec<String> = {
        let catalog = hf::catalog(pkg, doc_part, &docs.get(doc_part).unwrap().root)?;
        catalog.into_iter().map(|h| h.part).collect()
    };
    {
        let doc = docs.get_mut(doc_part).unwrap();
        sect::delete_section_break(&mut doc.root, op.n, &op.id, i)?;
    }
    dirty.insert(doc_part.to_string(), ());
    drop_hf_parts_not_in_catalog(pkg, docs, dirty, removed, doc_part, &old_parts)
}

fn drop_hf_parts_not_in_catalog(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    removed: &mut HashSet<String>,
    doc_part: &str,
    old_parts: &[String],
) -> Result<(), Error> {
    let still = hf::catalog(pkg, doc_part, &docs.get(doc_part).unwrap().root)?;
    for old in old_parts {
        if still.iter().any(|h| h.part == *old) {
            continue;
        }
        let mut rels = Relationships::from_package(pkg, doc_part)?;
        rels.remove_by_target_part(doc_part, old);
        rels.write_to(pkg, doc_part)?;

        let mut cts = ContentTypes::from_package(pkg)?;
        cts.remove_override(old);
        cts.write_to(pkg)?;

        pkg.remove(old);
        pkg.remove(&rels_part_name(old));
        removed.insert(old.to_string());
        docs.remove(old);
        dirty.remove(old);
    }
    Ok(())
}

fn current_part_bytes(
    pkg: &Package,
    docs: &HashMap<String, Document>,
    part: &str,
) -> Result<Vec<u8>, Error> {
    if let Some(doc) = docs.get(part) {
        return doc.to_bytes();
    }
    Ok(pkg.get(part)?.to_vec())
}

fn load_part<'a>(
    pkg: &Package,
    docs: &'a mut HashMap<String, Document>,
    part_name: &str,
) -> Result<&'a mut Document, Error> {
    if !docs.contains_key(part_name) {
        let parsed = xml::parse(pkg.get(part_name)?)?;
        docs.insert(part_name.to_string(), parsed);
    }
    Ok(docs.get_mut(part_name).unwrap())
}

fn ink_doc_mut<'a>(
    docs: &'a mut HashMap<String, Document>,
    part_name: &str,
) -> Result<&'a mut Document, Error> {
    docs.get_mut(part_name).ok_or_else(|| {
        Error::xml_detail(
            crate::error::XmlIssue::Parse,
            format!("{part_name} missing from apply cache"),
        )
    })
}

fn part_for_pic(
    pkg: &Package,
    catalog: &[hf::HfPart],
    doc_part: &str,
    target: &picture::PicTarget,
    id: &str,
    i: usize,
) -> Result<String, Error> {
    match target {
        picture::PicTarget::Body { .. } => Ok(doc_part.to_string()),
        picture::PicTarget::InTextBox { host, .. } => {
            part_for_host(pkg, catalog, doc_part, host, id, i)
        }
        picture::PicTarget::HeaderFooter {
            kind, hf_type, n, ..
        } => {
            let container_id = format!("{}:{}:{n}", kind.as_str(), hf_type.as_str());
            let item = hf::find_by_id(catalog, &container_id).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::PictureNotFound { id: id.to_string() },
                    format!("ops[{i}].id"),
                )
            })?;
            Ok(item.part.clone())
        }
        picture::PicTarget::InNote { kind, .. } => notes::part_name(pkg, doc_part, *kind)?
            .ok_or_else(|| {
                Error::with_path(
                    ErrorKind::PictureNotFound { id: id.to_string() },
                    format!("ops[{i}].id"),
                )
            }),
        picture::PicTarget::InComment { .. } => {
            comments::part_name(pkg, doc_part)?.ok_or_else(|| {
                Error::with_path(
                    ErrorKind::PictureNotFound { id: id.to_string() },
                    format!("ops[{i}].id"),
                )
            })
        }
    }
}

fn find_target_drawing_mut<'a>(
    root: &'a mut Element,
    target: &picture::PicTarget,
) -> Option<&'a mut Element> {
    match target {
        picture::PicTarget::HeaderFooter { key, .. } => picture::find_drawing_mut(root, key),
        picture::PicTarget::Body { para, key } => {
            let p = para::find_paragraph_mut(root, para)?;
            picture::find_drawing_mut(p, key)
        }
        picture::PicTarget::InTextBox {
            host,
            box_key,
            para,
            key,
        } => {
            let host_p = para::find_paragraph_mut(root, host.body())?;
            let content = textbox::find_txbx_content_by_key_mut(host_p, box_key)?;
            let inner = para::find_paragraph_in_txbx_mut(content, para)?;
            picture::find_drawing_mut(inner, key)
        }
        picture::PicTarget::InNote {
            kind,
            note_id,
            para,
            key,
        } => {
            let note = notes::find_user_note_mut(root, *kind, note_id)?;
            let inner = para::find_paragraph_mut(note, para)?;
            picture::find_drawing_mut(inner, key)
        }
        picture::PicTarget::InComment {
            comment_id,
            para,
            key,
        } => {
            let comment = comments::find_comment_mut(root, comment_id)?;
            let inner = para::find_paragraph_mut(comment, para)?;
            picture::find_drawing_mut(inner, key)
        }
    }
}

fn remove_target_picture(root: &mut Element, target: &picture::PicTarget) -> bool {
    match target {
        picture::PicTarget::HeaderFooter { key, .. } => picture::remove_picture(root, key),
        picture::PicTarget::Body { para, key } => match para::find_paragraph_mut(root, para) {
            Some(p) => picture::remove_picture(p, key),
            None => false,
        },
        picture::PicTarget::InTextBox {
            host,
            box_key,
            para,
            key,
        } => {
            let Some(host_p) = para::find_paragraph_mut(root, host.body()) else {
                return false;
            };
            let Some(content) = textbox::find_txbx_content_by_key_mut(host_p, box_key) else {
                return false;
            };
            match para::find_paragraph_in_txbx_mut(content, para) {
                Some(inner) => picture::remove_picture(inner, key),
                None => false,
            }
        }
        picture::PicTarget::InNote {
            kind,
            note_id,
            para,
            key,
        } => match notes::find_user_note_mut(root, *kind, note_id) {
            Some(note) => match para::find_paragraph_mut(note, para) {
                Some(inner) => picture::remove_picture(inner, key),
                None => false,
            },
            None => false,
        },
        picture::PicTarget::InComment {
            comment_id,
            para,
            key,
        } => match comments::find_comment_mut(root, comment_id) {
            Some(comment) => match para::find_paragraph_mut(comment, para) {
                Some(inner) => picture::remove_picture(inner, key),
                None => false,
            },
            None => false,
        },
    }
}

fn apply_replace_pic(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &ReplacePictureOp,
    i: usize,
) -> Result<(), Error> {
    reject_ink_pic(pkg, docs, doc_part, &op.target, &op.id, i)?;
    let part_name = part_for_pic(pkg, catalog, doc_part, &op.target, &op.id, i)?;
    let doc = load_part(pkg, docs, &part_name)?;
    let (xml_dirty, drop_rid) = {
        let drawing = find_target_drawing_mut(&mut doc.root, &op.target).ok_or_else(|| {
            Error::with_path(
                ErrorKind::PictureNotFound { id: op.id.clone() },
                format!("ops[{i}].id"),
            )
        })?;
        picture::replace_media(pkg, &part_name, drawing, &op.file)?
    };
    if xml_dirty {
        dirty.insert(part_name.clone(), ());
    }
    if let Some(rid) = drop_rid {
        let media = picture::resolve_embed(pkg, &part_name, &rid).ok();
        picture::remove_rel_if_unused(pkg, &part_name, &rid, &doc.root)?;
        if let Some(m) = media {
            if !picture::media_still_used(pkg, &m)? {
                pkg.remove(&m);
            }
        }
    }
    Ok(())
}

fn apply_insert_pic(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &InsertPictureOp,
    i: usize,
) -> Result<(), Error> {
    let part_name = match &op.para {
        ParaTarget::Body(_) => doc_part.to_string(),
        ParaTarget::HeaderFooter {
            kind, hf_type, n, ..
        } => part_for_hf(catalog, *kind, *hf_type, *n, &op.id, i)?,
        ParaTarget::InTextBox { host, .. } => {
            part_for_host(pkg, catalog, doc_part, host, &op.id, i)?
        }
        ParaTarget::InNote { kind, .. } => {
            notes::part_name(pkg, doc_part, *kind)?.ok_or_else(|| {
                Error::with_path(
                    ErrorKind::ParaNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )
            })?
        }
        ParaTarget::InComment { .. } => comments::part_name(pkg, doc_part)?.ok_or_else(|| {
            Error::with_path(
                ErrorKind::ParaNotFound { id: op.id.clone() },
                format!("ops[{i}].id"),
            )
        })?,
    };
    if let ParaTarget::InComment { comment_id, .. } = &op.para {
        let doc = load_part(pkg, docs, &part_name)?;
        let comment = comments::find_comment(&doc.root, comment_id).ok_or_else(|| {
            Error::with_path(
                ErrorKind::ParaNotFound { id: op.id.clone() },
                format!("ops[{i}].id"),
            )
        })?;
        comments::reject_ink(comment, &op.id, i)?;
    }
    {
        let doc = load_part(pkg, docs, &part_name)?;
        if find_insert_para_mut(&mut doc.root, &op.para).is_none() {
            return Err(Error::with_path(
                ErrorKind::ParaNotFound { id: op.id.clone() },
                format!("ops[{i}].id"),
            ));
        }
    }
    let added = picture::add_media(pkg, &part_name, &op.file, op.cx, op.cy)?;

    let doc = load_part(pkg, docs, &part_name)?;
    picture::ensure_drawing_ns(&mut doc.root);
    let doc_pr = picture::next_docpr_id(&doc.root);
    let p = find_insert_para_mut(&mut doc.root, &op.para).ok_or_else(|| {
        Error::with_path(
            ErrorKind::ParaNotFound { id: op.id.clone() },
            format!("ops[{i}].id"),
        )
    })?;
    picture::insert_prepared_drawing(
        p,
        doc_pr,
        &added.blip_rid,
        added.svg_rid.as_deref(),
        added.cx,
        added.cy,
        picture::PicKind::Inline,
        None,
        &picture::WrapPos::default(),
    )?;
    dirty.insert(part_name, ());
    Ok(())
}

fn apply_delete_pic(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &DeletePictureOp,
    i: usize,
) -> Result<(), Error> {
    reject_ink_pic(pkg, docs, doc_part, &op.target, &op.id, i)?;
    let part_name = part_for_pic(pkg, catalog, doc_part, &op.target, &op.id, i)?;
    let (rids, medias) = {
        let doc = load_part(pkg, docs, &part_name)?;
        let drawing = find_target_drawing_mut(&mut doc.root, &op.target).ok_or_else(|| {
            Error::with_path(
                ErrorKind::PictureNotFound { id: op.id.clone() },
                format!("ops[{i}].id"),
            )
        })?;
        let rids = picture::embed_rids(drawing);
        let medias: Vec<String> = rids
            .iter()
            .filter_map(|r| picture::resolve_embed(pkg, &part_name, r).ok())
            .collect();
        if !remove_target_picture(&mut doc.root, &op.target) {
            return Err(Error::with_path(
                ErrorKind::PictureNotFound { id: op.id.clone() },
                format!("ops[{i}].id"),
            ));
        }
        (rids, medias)
    };
    dirty.insert(part_name.clone(), ());
    for r in rids {
        picture::remove_rel_if_unused(pkg, &part_name, &r, &docs.get(&part_name).unwrap().root)?;
    }
    for m in medias {
        if !picture::media_still_used(pkg, &m)? {
            pkg.remove(&m);
        }
    }
    Ok(())
}

fn apply_pic_size(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &SetPictureSizeOp,
    i: usize,
) -> Result<(), Error> {
    reject_ink_pic(pkg, docs, doc_part, &op.target, &op.id, i)?;
    let part_name = part_for_pic(pkg, catalog, doc_part, &op.target, &op.id, i)?;
    let doc = load_part(pkg, docs, &part_name)?;
    let drawing = find_target_drawing_mut(&mut doc.root, &op.target).ok_or_else(|| {
        Error::with_path(
            ErrorKind::PictureNotFound { id: op.id.clone() },
            format!("ops[{i}].id"),
        )
    })?;
    if !picture::set_extent(drawing, op.cx, op.cy) {
        return Err(Error::with_path(
            ErrorKind::PictureNotFound { id: op.id.clone() },
            format!("ops[{i}].id"),
        ));
    }
    dirty.insert(part_name, ());
    Ok(())
}

fn apply_pic_wrap(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &SetPictureWrapOp,
    i: usize,
) -> Result<(), Error> {
    reject_ink_pic(pkg, docs, doc_part, &op.target, &op.id, i)?;
    let part_name = part_for_pic(pkg, catalog, doc_part, &op.target, &op.id, i)?;
    let doc = load_part(pkg, docs, &part_name)?;
    picture::ensure_drawing_ns(&mut doc.root);
    let drawing = find_target_drawing_mut(&mut doc.root, &op.target).ok_or_else(|| {
        Error::with_path(
            ErrorKind::PictureNotFound { id: op.id.clone() },
            format!("ops[{i}].id"),
        )
    })?;
    picture::apply_wrap(drawing, op.kind, op.wrap, &op.pos)
        .map_err(|e| e.at(format!("ops[{i}]")))?;
    dirty.insert(part_name, ());
    Ok(())
}

fn part_for_hf(
    catalog: &[hf::HfPart],
    kind: hf::HfKind,
    hf_type: hf::HfType,
    n: u32,
    id: &str,
    i: usize,
) -> Result<String, Error> {
    let container_id = format!("{}:{}:{n}", kind.as_str(), hf_type.as_str());
    let item = hf::find_by_id(catalog, &container_id).ok_or_else(|| {
        Error::with_path(
            ErrorKind::ParaNotFound { id: id.to_string() },
            format!("ops[{i}].id"),
        )
    })?;
    Ok(item.part.clone())
}

fn part_for_host(
    pkg: &Package,
    catalog: &[hf::HfPart],
    doc_part: &str,
    host: &HostPara,
    id: &str,
    i: usize,
) -> Result<String, Error> {
    match host {
        HostPara::Body(_) => Ok(doc_part.to_string()),
        HostPara::HeaderFooter {
            kind, hf_type, n, ..
        } => part_for_hf(catalog, *kind, *hf_type, *n, id, i),
        HostPara::InNote { kind, .. } => notes::part_name(pkg, doc_part, *kind)?.ok_or_else(|| {
            Error::with_path(
                ErrorKind::ParaNotFound { id: id.to_string() },
                format!("ops[{i}].id"),
            )
        }),
        HostPara::InComment { .. } => comments::part_name(pkg, doc_part)?.ok_or_else(|| {
            Error::with_path(
                ErrorKind::ParaNotFound { id: id.to_string() },
                format!("ops[{i}].id"),
            )
        }),
    }
}

fn part_for_txbx(
    pkg: &Package,
    catalog: &[hf::HfPart],
    doc_part: &str,
    target: &para::TxbxTarget,
    id: &str,
    i: usize,
) -> Result<String, Error> {
    part_for_host(pkg, catalog, doc_part, &target.host, id, i)
}

fn find_insert_para_mut<'a>(root: &'a mut Element, target: &ParaTarget) -> Option<&'a mut Element> {
    match target {
        ParaTarget::Body(p) => para::find_paragraph_mut(root, p),
        ParaTarget::HeaderFooter { para, .. } => para::find_paragraph_mut(root, para),
        ParaTarget::InTextBox { host, key, para } => {
            let host_p = para::find_paragraph_mut(root, host.body())?;
            let content = textbox::find_txbx_content_by_key_mut(host_p, key)?;
            para::find_paragraph_in_txbx_mut(content, para)
        }
        ParaTarget::InNote {
            kind,
            note_id,
            para,
        } => {
            let note = notes::find_user_note_mut(root, *kind, note_id)?;
            para::find_paragraph_mut(note, para)
        }
        ParaTarget::InComment { comment_id, para } => {
            let comment = comments::find_comment_mut(root, comment_id)?;
            para::find_paragraph_mut(comment, para)
        }
    }
}

fn txbx_missing(id: &str, i: usize) -> Error {
    Error::with_path(
        ErrorKind::TextBoxNotFound { id: id.to_string() },
        format!("ops[{i}].id"),
    )
}

fn apply_insert_txbx(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &InsertTextBoxOp,
    i: usize,
) -> Result<(), Error> {
    if matches!(
        &op.host,
        HostPara::InNote { .. } | HostPara::InComment { .. }
    ) {
        return Err(Error::with_path(
            ErrorKind::GraphicInRestrictedStory,
            format!("ops[{i}].id"),
        ));
    }
    let part_name = part_for_host(pkg, catalog, doc_part, &op.host, &op.id, i)?;
    let doc = load_part(pkg, docs, &part_name)?;
    textbox::ensure_textbox_ns(&mut doc.root);
    let doc_pr = picture::next_docpr_id(&doc.root);
    let para_id = para::next_para_hex(&doc.root);
    let (cx, cy) = match (op.cx, op.cy) {
        (Some(x), Some(y)) => (x, y),
        _ => (textbox::DEFAULT_CX, textbox::DEFAULT_CY),
    };
    let p = para::find_paragraph_mut(&mut doc.root, op.host.body()).ok_or_else(|| {
        Error::with_path(
            ErrorKind::ParaNotFound { id: op.id.clone() },
            format!("ops[{i}].id"),
        )
    })?;
    textbox::insert_textbox(p, doc_pr, cx, cy, &op.text, &para_id);
    dirty.insert(part_name, ());
    Ok(())
}

fn apply_delete_txbx(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &DeleteTextBoxOp,
    i: usize,
) -> Result<(), Error> {
    let part_name = part_for_txbx(pkg, catalog, doc_part, &op.target, &op.id, i)?;
    let doc = load_part(pkg, docs, &part_name)?;
    let story = {
        let host = para::find_paragraph(&doc.root, op.target.host.body())
            .ok_or_else(|| txbx_missing(&op.id, i))?;
        if textbox::find_txbx_content_by_key(host, &op.target.key).is_none() {
            return Err(txbx_missing(&op.id, i));
        }
        textbox::story_id_at(&doc.root, &op.target)
    };
    let host = para::find_paragraph_mut(&mut doc.root, op.target.host.body())
        .ok_or_else(|| txbx_missing(&op.id, i))?;
    if !textbox::remove_textbox(host, &op.target.key) {
        return Err(txbx_missing(&op.id, i));
    }
    if let Some(sid) = story {
        textbox::resequence_story(&mut doc.root, &sid);
    }
    dirty.insert(part_name, ());
    Ok(())
}

fn apply_txbx_size(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &SetTextBoxSizeOp,
    i: usize,
) -> Result<(), Error> {
    let part_name = part_for_txbx(pkg, catalog, doc_part, &op.target, &op.id, i)?;
    let doc = load_part(pkg, docs, &part_name)?;
    let host = para::find_paragraph_mut(&mut doc.root, op.target.host.body())
        .ok_or_else(|| txbx_missing(&op.id, i))?;
    let drawing = textbox::find_textbox_drawing_mut(host, &op.target.key)
        .ok_or_else(|| txbx_missing(&op.id, i))?;
    if !picture::set_extent(drawing, op.cx, op.cy) {
        return Err(txbx_missing(&op.id, i));
    }
    dirty.insert(part_name, ());
    Ok(())
}

fn apply_txbx_wrap(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &SetTextBoxWrapOp,
    i: usize,
) -> Result<(), Error> {
    let part_name = part_for_txbx(pkg, catalog, doc_part, &op.target, &op.id, i)?;
    let doc = load_part(pkg, docs, &part_name)?;
    textbox::ensure_textbox_ns(&mut doc.root);
    let host = para::find_paragraph_mut(&mut doc.root, op.target.host.body())
        .ok_or_else(|| txbx_missing(&op.id, i))?;
    let drawing = textbox::find_textbox_drawing_mut(host, &op.target.key)
        .ok_or_else(|| txbx_missing(&op.id, i))?;
    picture::apply_wrap(drawing, op.kind, op.wrap, &op.pos)
        .map_err(|e| e.at(format!("ops[{i}]")))?;
    dirty.insert(part_name, ());
    Ok(())
}

fn apply_txbx_fill(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &SetTextBoxFillOp,
    i: usize,
) -> Result<(), Error> {
    let part_name = part_for_txbx(pkg, catalog, doc_part, &op.target, &op.id, i)?;
    let doc = load_part(pkg, docs, &part_name)?;
    textbox::ensure_textbox_ns(&mut doc.root);
    let host = para::find_paragraph_mut(&mut doc.root, op.target.host.body())
        .ok_or_else(|| txbx_missing(&op.id, i))?;
    textbox::with_wsp_mut(host, &op.target.key, |wsp| {
        textbox::set_fill(wsp, op.color.as_deref())
    })
    .ok_or_else(|| txbx_missing(&op.id, i))??;
    dirty.insert(part_name, ());
    Ok(())
}

fn apply_txbx_line(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &SetTextBoxLineOp,
    i: usize,
) -> Result<(), Error> {
    let part_name = part_for_txbx(pkg, catalog, doc_part, &op.target, &op.id, i)?;
    let doc = load_part(pkg, docs, &part_name)?;
    textbox::ensure_textbox_ns(&mut doc.root);
    let host = para::find_paragraph_mut(&mut doc.root, op.target.host.body())
        .ok_or_else(|| txbx_missing(&op.id, i))?;
    textbox::with_wsp_mut(host, &op.target.key, |wsp| {
        textbox::set_line(wsp, op.color.as_deref(), op.width)
    })
    .ok_or_else(|| txbx_missing(&op.id, i))??;
    dirty.insert(part_name, ());
    Ok(())
}

fn apply_txbx_dir(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &SetTextBoxDirectionOp,
    i: usize,
) -> Result<(), Error> {
    let part_name = part_for_txbx(pkg, catalog, doc_part, &op.target, &op.id, i)?;
    let doc = load_part(pkg, docs, &part_name)?;
    textbox::ensure_textbox_ns(&mut doc.root);
    let host = para::find_paragraph_mut(&mut doc.root, op.target.host.body())
        .ok_or_else(|| txbx_missing(&op.id, i))?;
    textbox::with_wsp_mut(host, &op.target.key, |wsp| {
        textbox::set_direction(wsp, op.vertical);
    })
    .ok_or_else(|| txbx_missing(&op.id, i))?;
    dirty.insert(part_name, ());
    Ok(())
}

fn apply_txbx_text_anchor(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &SetTextBoxTextAnchorOp,
    i: usize,
) -> Result<(), Error> {
    let part_name = part_for_txbx(pkg, catalog, doc_part, &op.target, &op.id, i)?;
    let doc = load_part(pkg, docs, &part_name)?;
    textbox::ensure_textbox_ns(&mut doc.root);
    let host = para::find_paragraph_mut(&mut doc.root, op.target.host.body())
        .ok_or_else(|| txbx_missing(&op.id, i))?;
    textbox::with_wsp_mut(host, &op.target.key, |wsp| {
        textbox::set_text_anchor(wsp, op.value.as_deref());
    })
    .ok_or_else(|| txbx_missing(&op.id, i))?;
    dirty.insert(part_name, ());
    Ok(())
}

fn cxn_missing(id: &str, i: usize) -> Error {
    Error::with_path(
        ErrorKind::ConnectorNotFound { id: id.to_string() },
        format!("ops[{i}].id"),
    )
}

fn apply_insert_connector(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &InsertConnectorOp,
    i: usize,
) -> Result<(), Error> {
    if matches!(
        &op.host,
        HostPara::InNote { .. } | HostPara::InComment { .. }
    ) {
        return Err(Error::with_path(
            ErrorKind::GraphicInRestrictedStory,
            format!("ops[{i}].id"),
        ));
    }
    let part_name = part_for_host(pkg, catalog, doc_part, &op.host, &op.id, i)?;
    let doc = load_part(pkg, docs, &part_name)?;
    textbox::ensure_textbox_ns(&mut doc.root);
    let doc_pr = picture::next_docpr_id(&doc.root);
    let (cx, cy) = match (op.cx, op.cy) {
        (Some(x), Some(y)) => (x, y),
        _ => (connector::DEFAULT_CX, connector::DEFAULT_CY),
    };
    let p = para::find_paragraph_mut(&mut doc.root, op.host.body()).ok_or_else(|| {
        Error::with_path(
            ErrorKind::ParaNotFound { id: op.id.clone() },
            format!("ops[{i}].id"),
        )
    })?;
    connector::insert_connector(p, doc_pr, cx, cy, &op.prst);
    dirty.insert(part_name, ());
    Ok(())
}

fn apply_delete_connector(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &DeleteConnectorOp,
    i: usize,
) -> Result<(), Error> {
    let part_name = part_for_txbx(pkg, catalog, doc_part, &op.target, &op.id, i)?;
    let doc = load_part(pkg, docs, &part_name)?;
    let host = para::find_paragraph_mut(&mut doc.root, op.target.host.body())
        .ok_or_else(|| cxn_missing(&op.id, i))?;
    if !connector::remove_connector(host, &op.target.key) {
        return Err(cxn_missing(&op.id, i));
    }
    dirty.insert(part_name, ());
    Ok(())
}

fn shape_missing(id: &str, i: usize) -> Error {
    Error::with_path(
        ErrorKind::ShapeNotFound { id: id.to_string() },
        format!("ops[{i}].id"),
    )
}

fn apply_insert_shape(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &InsertShapeOp,
    i: usize,
) -> Result<(), Error> {
    if matches!(
        &op.host,
        HostPara::InNote { .. } | HostPara::InComment { .. }
    ) {
        return Err(Error::with_path(
            ErrorKind::GraphicInRestrictedStory,
            format!("ops[{i}].id"),
        ));
    }
    let part_name = part_for_host(pkg, catalog, doc_part, &op.host, &op.id, i)?;
    let doc = load_part(pkg, docs, &part_name)?;
    textbox::ensure_textbox_ns(&mut doc.root);
    let doc_pr = picture::next_docpr_id(&doc.root);
    let (cx, cy) = match (op.cx, op.cy) {
        (Some(x), Some(y)) => (x, y),
        _ => (shape::DEFAULT_CX, shape::DEFAULT_CY),
    };
    let p = para::find_paragraph_mut(&mut doc.root, op.host.body()).ok_or_else(|| {
        Error::with_path(
            ErrorKind::ParaNotFound { id: op.id.clone() },
            format!("ops[{i}].id"),
        )
    })?;
    shape::insert_shape(p, doc_pr, cx, cy, &op.prst);
    dirty.insert(part_name, ());
    Ok(())
}

fn apply_delete_shape(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &DeleteShapeOp,
    i: usize,
) -> Result<(), Error> {
    let part_name = part_for_txbx(pkg, catalog, doc_part, &op.target, &op.id, i)?;
    let doc = load_part(pkg, docs, &part_name)?;
    let host = para::find_paragraph_mut(&mut doc.root, op.target.host.body())
        .ok_or_else(|| shape_missing(&op.id, i))?;
    if !shape::remove_shape(host, &op.target.key) {
        return Err(shape_missing(&op.id, i));
    }
    dirty.insert(part_name, ());
    Ok(())
}

fn apply_shape_size(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &SetShapeSizeOp,
    i: usize,
) -> Result<(), Error> {
    let part_name = part_for_txbx(pkg, catalog, doc_part, &op.target, &op.id, i)?;
    let doc = load_part(pkg, docs, &part_name)?;
    let host = para::find_paragraph_mut(&mut doc.root, op.target.host.body())
        .ok_or_else(|| shape_missing(&op.id, i))?;
    let drawing = shape::find_shape_drawing_mut(host, &op.target.key)
        .ok_or_else(|| shape_missing(&op.id, i))?;
    if !picture::set_extent(drawing, op.cx, op.cy) {
        return Err(shape_missing(&op.id, i));
    }
    dirty.insert(part_name, ());
    Ok(())
}

fn apply_shape_pos(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &SetShapePosOp,
    i: usize,
) -> Result<(), Error> {
    let part_name = part_for_txbx(pkg, catalog, doc_part, &op.target, &op.id, i)?;
    let doc = load_part(pkg, docs, &part_name)?;
    let host = para::find_paragraph_mut(&mut doc.root, op.target.host.body())
        .ok_or_else(|| shape_missing(&op.id, i))?;
    let drawing = shape::find_shape_drawing_mut(host, &op.target.key)
        .ok_or_else(|| shape_missing(&op.id, i))?;
    if !shape::set_pos(drawing, op.x, op.y) {
        return Err(Error::with_path(
            ErrorKind::DocxShapeNeedsAnchor,
            format!("ops[{i}].id"),
        ));
    }
    dirty.insert(part_name, ());
    Ok(())
}

fn apply_shape_fill(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &SetShapeFillOp,
    i: usize,
) -> Result<(), Error> {
    let part_name = part_for_txbx(pkg, catalog, doc_part, &op.target, &op.id, i)?;
    let doc = load_part(pkg, docs, &part_name)?;
    textbox::ensure_textbox_ns(&mut doc.root);
    let host = para::find_paragraph_mut(&mut doc.root, op.target.host.body())
        .ok_or_else(|| shape_missing(&op.id, i))?;
    shape::with_wsp_mut(host, &op.target.key, |wsp| {
        textbox::set_fill(wsp, op.color.as_deref())
    })
    .ok_or_else(|| shape_missing(&op.id, i))??;
    dirty.insert(part_name, ());
    Ok(())
}

fn apply_shape_line(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &SetShapeLineOp,
    i: usize,
) -> Result<(), Error> {
    let part_name = part_for_txbx(pkg, catalog, doc_part, &op.target, &op.id, i)?;
    let doc = load_part(pkg, docs, &part_name)?;
    textbox::ensure_textbox_ns(&mut doc.root);
    let host = para::find_paragraph_mut(&mut doc.root, op.target.host.body())
        .ok_or_else(|| shape_missing(&op.id, i))?;
    shape::with_wsp_mut(host, &op.target.key, |wsp| {
        textbox::set_line(wsp, op.color.as_deref(), op.width)
    })
    .ok_or_else(|| shape_missing(&op.id, i))??;
    dirty.insert(part_name, ());
    Ok(())
}

fn apply_group_shapes(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &GroupShapesOp,
    i: usize,
) -> Result<(), Error> {
    let part_name = part_for_group_host(catalog, doc_part, &op.host, &op.id, i)?;
    let doc = load_part(pkg, docs, &part_name)?;
    textbox::ensure_textbox_ns(&mut doc.root);
    group::group_shapes(&mut doc.root, &op.host, &op.members, i)?;
    dirty.insert(part_name, ());
    Ok(())
}

fn apply_ungroup(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &UngroupOp,
    i: usize,
) -> Result<(), Error> {
    let part_name = part_for_group_member(pkg, catalog, doc_part, &op.member, &op.id, i)?;
    let doc = load_part(pkg, docs, &part_name)?;
    textbox::ensure_textbox_ns(&mut doc.root);
    group::ungroup(&mut doc.root, &op.member, i)?;
    dirty.insert(part_name, ());
    Ok(())
}

fn part_for_group_host(
    catalog: &[hf::HfPart],
    doc_part: &str,
    host: &GroupHost,
    id: &str,
    i: usize,
) -> Result<String, Error> {
    match host {
        GroupHost::Document => Ok(doc_part.to_string()),
        GroupHost::HeaderFooter { kind, hf_type, n } => {
            part_for_hf(catalog, *kind, *hf_type, *n, id, i)
        }
    }
}

fn part_for_group_member(
    pkg: &Package,
    catalog: &[hf::HfPart],
    doc_part: &str,
    member: &GroupMember,
    id: &str,
    i: usize,
) -> Result<String, Error> {
    match member {
        GroupMember::Picture(target) => part_for_pic(pkg, catalog, doc_part, target, id, i),
        GroupMember::TextBox(t) | GroupMember::Connector(t) | GroupMember::Shape(t) => {
            part_for_txbx(pkg, catalog, doc_part, t, id, i)
        }
    }
}

fn apply_insert_para(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &InsertParaOp,
    i: usize,
) -> Result<(), Error> {
    match &op.place {
        InsertParaPlace::TextBox { box_target, after } => {
            let part_name = part_for_txbx(pkg, catalog, doc_part, box_target, &op.id, i)?;
            let doc = load_part(pkg, docs, &part_name)?;
            let new_id = para::next_para_hex(&doc.root);
            let host = para::find_paragraph_mut(&mut doc.root, box_target.host.body())
                .ok_or_else(|| txbx_missing(&op.id, i))?;
            let content = textbox::find_txbx_content_by_key_mut(host, &box_target.key)
                .ok_or_else(|| txbx_missing(&op.id, i))?;
            let new_p = para::new_paragraph(content, &new_id, &op.text);
            if let Some(ParaTarget::InTextBox { para, .. }) = after {
                insert_p_after(content, para, new_p).map_err(|_| {
                    Error::with_path(
                        ErrorKind::ParaNotFound { id: op.id.clone() },
                        format!("ops[{i}].id"),
                    )
                })?;
            } else {
                content.children.push(Node::Element(new_p));
            }
            dirty.insert(part_name, ());
            Ok(())
        }
        InsertParaPlace::Note {
            kind,
            note_id,
            after,
        } => {
            let part_name = notes::part_name(pkg, doc_part, *kind)?.ok_or_else(|| {
                Error::with_path(
                    ErrorKind::NoteNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )
            })?;
            let doc = load_part(pkg, docs, &part_name)?;
            let new_id = para::next_para_hex(&doc.root);
            let note =
                notes::find_user_note_mut(&mut doc.root, *kind, note_id).ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::NoteNotFound { id: op.id.clone() },
                        format!("ops[{i}].id"),
                    )
                })?;
            let new_p = para::new_paragraph(note, &new_id, &op.text);
            if let Some(para) = after {
                insert_p_after(note, para, new_p).map_err(|_| {
                    Error::with_path(
                        ErrorKind::ParaNotFound { id: op.id.clone() },
                        format!("ops[{i}].id"),
                    )
                })?;
            } else {
                note.children.push(Node::Element(new_p));
            }
            dirty.insert(part_name, ());
            Ok(())
        }
        InsertParaPlace::Comment { comment_id, after } => {
            let part_name = comments::part_name(pkg, doc_part)?
                .ok_or_else(|| comments::comment_missing(&op.id, i))?;
            let old_last = {
                let doc = load_part(pkg, docs, &part_name)?;
                let comment = comments::find_comment(&doc.root, comment_id)
                    .ok_or_else(|| comments::comment_missing(&op.id, i))?;
                comments::reject_ink(comment, &op.id, i)?;
                comments::last_para_id(comment)
            };
            let doc = load_part(pkg, docs, &part_name)?;
            let new_id = para::next_para_hex(&doc.root);
            let comment = comments::find_comment_mut(&mut doc.root, comment_id)
                .ok_or_else(|| comments::comment_missing(&op.id, i))?;
            let new_p = para::new_paragraph(comment, &new_id, &op.text);
            if let Some(para) = after {
                insert_p_after(comment, para, new_p).map_err(|_| {
                    Error::with_path(
                        ErrorKind::ParaNotFound { id: op.id.clone() },
                        format!("ops[{i}].id"),
                    )
                })?;
            } else {
                comment.children.push(Node::Element(new_p));
            }
            dirty.insert(part_name.clone(), ());
            retarget_comment_last(pkg, docs, dirty, doc_part, comment_id, old_last.as_deref())?;
            Ok(())
        }
        InsertParaPlace::Body { after } => apply_insert_story_para(
            pkg,
            docs,
            dirty,
            doc_part,
            after.as_ref(),
            &op.id,
            &op.text,
            i,
        ),
        InsertParaPlace::HeaderFooter {
            kind,
            hf_type,
            n,
            after,
        } => {
            let part_name = part_for_hf(catalog, *kind, *hf_type, *n, &op.id, i)?;
            apply_insert_story_para(
                pkg,
                docs,
                dirty,
                &part_name,
                after.as_ref(),
                &op.id,
                &op.text,
                i,
            )
        }
    }
}

fn apply_insert_story_para(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    part_name: &str,
    after: Option<&para::BodyPara>,
    id: &str,
    text: &str,
    i: usize,
) -> Result<(), Error> {
    let doc = load_part(pkg, docs, part_name)?;
    let new_id = para::next_para_hex(&doc.root);
    if let Some(after) = after {
        let path = para::find_paragraph_path(&doc.root, after).ok_or_else(|| {
            Error::with_path(
                ErrorKind::ParaNotFound { id: id.to_string() },
                format!("ops[{i}].id"),
            )
        })?;
        let new_p = {
            let host = para::element_at(&doc.root, &path).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::ParaNotFound { id: id.to_string() },
                    format!("ops[{i}].id"),
                )
            })?;
            para::new_paragraph(host, &new_id, text)
        };
        let (parent, idx) = para::parent_and_index_mut(&mut doc.root, &path).ok_or_else(|| {
            Error::with_path(
                ErrorKind::ParaNotFound { id: id.to_string() },
                format!("ops[{i}].id"),
            )
        })?;
        parent.children.insert(idx + 1, Node::Element(new_p));
    } else {
        let body = para::story_body_mut(&mut doc.root).ok_or_else(|| {
            Error::with_path(
                ErrorKind::XmlMissing {
                    parent: part_name.to_string(),
                    child: "body".into(),
                },
                format!("ops[{i}].id"),
            )
        })?;
        let new_p = para::new_paragraph(body, &new_id, text);
        para::append_before_body_sect_pr(body, new_p);
    }
    dirty.insert(part_name.to_string(), ());
    Ok(())
}

fn insert_p_after(parent: &mut Element, para: &para::BodyPara, new_p: Element) -> Result<(), ()> {
    let mut idx = 0usize;
    let mut insert_at = None;
    for (j, n) in parent.children.iter().enumerate() {
        if let Node::Element(e) = n {
            if e.local_name() == "p" {
                idx += 1;
                let hit = match para {
                    para::BodyPara::Index(n) => idx == *n,
                    para::BodyPara::Hex(hex) => e
                        .attr_local("paraId")
                        .is_some_and(|id| id.eq_ignore_ascii_case(hex)),
                };
                if hit {
                    insert_at = Some(j + 1);
                    break;
                }
            }
        }
    }
    let at = insert_at.ok_or(())?;
    parent.children.insert(at, Node::Element(new_p));
    Ok(())
}

fn apply_delete_para(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &DeleteParaOp,
    i: usize,
) -> Result<(), Error> {
    match &op.inner {
        ParaTarget::InTextBox { host, key, para } => {
            let part_name = part_for_host(pkg, catalog, doc_part, host, &op.id, i)?;
            let doc = load_part(pkg, docs, &part_name)?;
            let host_p = para::find_paragraph_mut(&mut doc.root, host.body()).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::ParaNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )
            })?;
            let content = textbox::find_txbx_content_by_key_mut(host_p, key).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::ParaNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )
            })?;
            let count = para::collect_paragraphs_in_txbx(content).len();
            if count <= 1 {
                return Err(Error::with_path(
                    ErrorKind::LastTextBoxPara,
                    format!("ops[{i}].id"),
                ));
            }
            remove_p_at(content, para).map_err(|_| {
                Error::with_path(
                    ErrorKind::ParaNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )
            })?;
            dirty.insert(part_name, ());
            Ok(())
        }
        ParaTarget::InNote {
            kind,
            note_id,
            para,
        } => {
            let part_name = notes::part_name(pkg, doc_part, *kind)?.ok_or_else(|| {
                Error::with_path(
                    ErrorKind::NoteNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )
            })?;
            let doc = load_part(pkg, docs, &part_name)?;
            let note =
                notes::find_user_note_mut(&mut doc.root, *kind, note_id).ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::NoteNotFound { id: op.id.clone() },
                        format!("ops[{i}].id"),
                    )
                })?;
            let count = para::collect_paragraphs(note).len();
            if count <= 1 {
                return Err(Error::with_path(
                    ErrorKind::LastNotePara,
                    format!("ops[{i}].id"),
                ));
            }
            remove_p_at(note, para).map_err(|_| {
                Error::with_path(
                    ErrorKind::ParaNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )
            })?;
            dirty.insert(part_name, ());
            Ok(())
        }
        ParaTarget::InComment { comment_id, para } => {
            let part_name = comments::part_name(pkg, doc_part)?
                .ok_or_else(|| comments::comment_missing(&op.id, i))?;
            let old_last = {
                let doc = load_part(pkg, docs, &part_name)?;
                let comment = comments::find_comment(&doc.root, comment_id)
                    .ok_or_else(|| comments::comment_missing(&op.id, i))?;
                comments::reject_ink(comment, &op.id, i)?;
                comments::last_para_id(comment)
            };
            let doc = load_part(pkg, docs, &part_name)?;
            let comment = comments::find_comment_mut(&mut doc.root, comment_id)
                .ok_or_else(|| comments::comment_missing(&op.id, i))?;
            let count = para::collect_paragraphs(comment).len();
            if count <= 1 {
                return Err(Error::with_path(
                    ErrorKind::LastCommentPara,
                    format!("ops[{i}].id"),
                ));
            }
            remove_p_at(comment, para).map_err(|_| {
                Error::with_path(
                    ErrorKind::ParaNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )
            })?;
            dirty.insert(part_name.clone(), ());
            retarget_comment_last(pkg, docs, dirty, doc_part, comment_id, old_last.as_deref())?;
            Ok(())
        }
        ParaTarget::Body(para) => {
            apply_delete_story_para(pkg, docs, dirty, doc_part, para, &op.id, i)
        }
        ParaTarget::HeaderFooter {
            kind,
            hf_type,
            n,
            para,
        } => {
            let part_name = part_for_hf(catalog, *kind, *hf_type, *n, &op.id, i)?;
            apply_delete_story_para(pkg, docs, dirty, &part_name, para, &op.id, i)
        }
    }
}

fn apply_delete_story_para(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    part_name: &str,
    para: &para::BodyPara,
    id: &str,
    i: usize,
) -> Result<(), Error> {
    let doc = load_part(pkg, docs, part_name)?;
    let path = para::find_paragraph_path(&doc.root, para).ok_or_else(|| {
        Error::with_path(
            ErrorKind::ParaNotFound { id: id.to_string() },
            format!("ops[{i}].id"),
        )
    })?;
    let parent_count = if path.len() <= 1 {
        para::count_direct_p(&doc.root)
    } else {
        let parent = para::element_at(&doc.root, &path[..path.len() - 1]).ok_or_else(|| {
            Error::with_path(
                ErrorKind::ParaNotFound { id: id.to_string() },
                format!("ops[{i}].id"),
            )
        })?;
        para::count_direct_p(parent)
    };
    if parent_count <= 1 {
        return Err(Error::with_path(
            ErrorKind::LastStoryPara,
            format!("ops[{i}].id"),
        ));
    }
    let (has_rev, unmatched, has_sect, is_last) = {
        let p_el = para::element_at(&doc.root, &path).ok_or_else(|| {
            Error::with_path(
                ErrorKind::ParaNotFound { id: id.to_string() },
                format!("ops[{i}].id"),
            )
        })?;
        (
            para::para_has_revision(p_el),
            para::para_has_unmatched_range(p_el),
            para::para_has_sect_pr(p_el),
            para::is_last_collected(&doc.root, para),
        )
    };
    if has_rev {
        return Err(Error::with_path(
            ErrorKind::ParaHasRevision,
            format!("ops[{i}].id"),
        ));
    }
    if unmatched {
        return Err(Error::with_path(
            ErrorKind::CrossParaMarker,
            format!("ops[{i}].id"),
        ));
    }
    if has_sect && !is_last {
        return Err(Error::with_path(
            ErrorKind::SectionBreakPara,
            format!("ops[{i}].id"),
        ));
    }
    if has_sect && is_last {
        let sect = {
            let p_mut = para::element_at_mut(&mut doc.root, &path).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::ParaNotFound { id: id.to_string() },
                    format!("ops[{i}].id"),
                )
            })?;
            para::take_ppr_sect_pr(p_mut)
        };
        if let Some(sect) = sect {
            let paths = para::collect_paragraph_paths(&doc.root);
            if let Some(dest_path) = paths.iter().rev().find(|pp| *pp != &path).cloned() {
                let dest = para::element_at_mut(&mut doc.root, &dest_path).ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::ParaNotFound { id: id.to_string() },
                        format!("ops[{i}].id"),
                    )
                })?;
                para::put_ppr_sect_pr(dest, sect);
            }
        }
    }
    let (parent, idx) = para::parent_and_index_mut(&mut doc.root, &path).ok_or_else(|| {
        Error::with_path(
            ErrorKind::ParaNotFound { id: id.to_string() },
            format!("ops[{i}].id"),
        )
    })?;
    parent.children.remove(idx);
    dirty.insert(part_name.to_string(), ());
    Ok(())
}

fn remove_p_at(parent: &mut Element, para: &para::BodyPara) -> Result<(), ()> {
    let mut idx = 0usize;
    let mut remove_at = None;
    for (j, n) in parent.children.iter().enumerate() {
        if let Node::Element(e) = n {
            if e.local_name() == "p" {
                idx += 1;
                let hit = match para {
                    para::BodyPara::Index(n) => idx == *n,
                    para::BodyPara::Hex(hex) => e
                        .attr_local("paraId")
                        .is_some_and(|id| id.eq_ignore_ascii_case(hex)),
                };
                if hit {
                    remove_at = Some(j);
                    break;
                }
            }
        }
    }
    let at = remove_at.ok_or(())?;
    parent.children.remove(at);
    Ok(())
}

fn apply_txbx_link(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &SetTextBoxLinkOp,
    i: usize,
) -> Result<(), Error> {
    let part_a = part_for_txbx(pkg, catalog, doc_part, &op.target, &op.id, i)?;
    if let Some(next) = &op.next {
        let part_b = part_for_txbx(
            pkg,
            catalog,
            doc_part,
            next,
            op.next_id.as_deref().unwrap_or(""),
            i,
        )?;
        if part_a != part_b {
            return Err(Error::with_path(
                ErrorKind::TextBoxLinkCrossPart,
                format!("ops[{i}].next"),
            ));
        }
    }
    let doc = load_part(pkg, docs, &part_a)?;
    if para::find_paragraph(&doc.root, op.target.host.body()).is_none()
        || !textbox::textbox_exists(
            para::find_paragraph(&doc.root, op.target.host.body()).unwrap(),
            &op.target.key,
        )
    {
        return Err(txbx_missing(&op.id, i));
    }
    if let Some(next) = &op.next {
        let host = para::find_paragraph(&doc.root, next.host.body()).ok_or_else(|| {
            Error::with_path(
                ErrorKind::TextBoxNotFound {
                    id: op.next_id.clone().unwrap_or_default(),
                },
                format!("ops[{i}].next"),
            )
        })?;
        if !textbox::textbox_exists(host, &next.key) {
            return Err(Error::with_path(
                ErrorKind::TextBoxNotFound {
                    id: op.next_id.clone().unwrap_or_default(),
                },
                format!("ops[{i}].next"),
            ));
        }
    }
    textbox::apply_link(&mut doc.root, &op.target, op.next.as_ref())?;
    dirty.insert(part_a, ());
    Ok(())
}

fn apply_insert_note(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &InsertNoteOp,
    kind: NoteKind,
    i: usize,
) -> Result<(), Error> {
    let host_part = match &op.host {
        ParaTarget::Body(_) => doc_part.to_string(),
        ParaTarget::HeaderFooter {
            kind: hf_kind,
            hf_type,
            n,
            ..
        } => part_for_hf(catalog, *hf_kind, *hf_type, *n, &op.id, i)?,
        ParaTarget::InTextBox { host, .. } => {
            part_for_host(pkg, catalog, doc_part, host, &op.id, i)?
        }
        ParaTarget::InNote { .. } | ParaTarget::InComment { .. } => {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: crate::error::IdReason::NotePrefix,
                },
                format!("ops[{i}].id"),
            ));
        }
    };
    {
        let doc = load_part(pkg, docs, &host_part)?;
        if find_insert_para_mut(&mut doc.root, &op.host).is_none() {
            return Err(Error::with_path(
                ErrorKind::ParaNotFound { id: op.id.clone() },
                format!("ops[{i}].id"),
            ));
        }
    }
    let notes_part = notes::ensure_part(pkg, docs, dirty, doc_part, kind)?;
    let (w_id, para_hex) = {
        let doc = load_part(pkg, docs, &notes_part)?;
        let w_id = notes::next_user_id(&doc.root, kind);
        let para_hex = para::next_para_hex(&doc.root);
        notes::append_user_note(&mut doc.root, kind, &w_id, &para_hex, &op.text);
        dirty.insert(notes_part.clone(), ());
        (w_id, para_hex)
    };
    let _ = para_hex;
    let doc = load_part(pkg, docs, &host_part)?;
    let p = find_insert_para_mut(&mut doc.root, &op.host).ok_or_else(|| {
        Error::with_path(
            ErrorKind::ParaNotFound { id: op.id.clone() },
            format!("ops[{i}].id"),
        )
    })?;
    notes::append_reference(p, kind, &w_id);
    dirty.insert(host_part, ());
    Ok(())
}

fn apply_delete_note(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &DeleteNoteOp,
    i: usize,
) -> Result<(), Error> {
    let notes_part = notes::part_name(pkg, doc_part, op.kind)?.ok_or_else(|| {
        Error::with_path(
            ErrorKind::NoteNotFound { id: op.id.clone() },
            format!("ops[{i}].id"),
        )
    })?;
    let (embeds, media) = {
        let doc = load_part(pkg, docs, &notes_part)?;
        if notes::find_user_note(&doc.root, op.kind, &op.note_id).is_none() {
            let is_sep = notes::seps(&doc.root, op.kind)
                .iter()
                .any(|e| notes::ids_match(e.attr_local("id").unwrap_or(""), &op.note_id));
            return Err(Error::with_path(
                if is_sep {
                    ErrorKind::InvalidId {
                        reason: crate::error::IdReason::NoteSep,
                    }
                } else {
                    ErrorKind::NoteNotFound { id: op.id.clone() }
                },
                format!("ops[{i}].id"),
            ));
        }
        let note = notes::find_user_note(&doc.root, op.kind, &op.note_id).unwrap();
        let embeds = notes::collect_blip_embeds(note);
        let mut media = Vec::new();
        for rid in &embeds {
            if let Ok(m) = picture::resolve_embed(pkg, &notes_part, rid) {
                media.push(m);
            }
        }
        (embeds, media)
    };
    {
        let doc = load_part(pkg, docs, &notes_part)?;
        notes::remove_user_note(&mut doc.root, op.kind, &op.note_id);
        dirty.insert(notes_part.clone(), ());
    }
    {
        let doc = load_part(pkg, docs, doc_part)?;
        notes::remove_references(&mut doc.root, op.kind, &op.note_id);
        dirty.insert(doc_part.to_string(), ());
    }
    for item in catalog {
        let doc = load_part(pkg, docs, &item.part)?;
        notes::remove_references(&mut doc.root, op.kind, &op.note_id);
        dirty.insert(item.part.clone(), ());
    }
    let notes_root = &docs.get(&notes_part).unwrap().root;
    for rid in embeds {
        picture::remove_rel_if_unused(pkg, &notes_part, &rid, notes_root)?;
    }
    for m in media {
        if !picture::media_still_used(pkg, &m)? {
            pkg.remove(&m);
        }
    }
    Ok(())
}

fn apply_note_numbering(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    op: &SetNoteNumberingOp,
    i: usize,
) -> Result<(), Error> {
    match op.target {
        NoteNumberTarget::Document => {
            let settings = notes::settings_part_name(pkg, doc_part)?.ok_or_else(|| {
                Error::with_path(
                    ErrorKind::PartMissing {
                        name: "/word/settings.xml".into(),
                    },
                    format!("ops[{i}].id"),
                )
            })?;
            let doc = load_part(pkg, docs, &settings)?;
            notes::apply_numbering(
                &mut doc.root,
                op.kind,
                op.num_fmt.as_deref(),
                op.num_start,
                op.num_restart.as_deref(),
            );
            dirty.insert(settings, ());
            // Word 16.0 读的是节上的 numFmt；只写 settings 时 COM NumberStyle 仍是阿拉伯。
            let n_sects = hf::collect_sect_prs(&load_part(pkg, docs, doc_part)?.root).len();
            for n in 1..=n_sects {
                let doc = load_part(pkg, docs, doc_part)?;
                let Some(sect) = hf::find_sect_pr_mut(&mut doc.root, n) else {
                    continue;
                };
                notes::apply_numbering(
                    sect,
                    op.kind,
                    op.num_fmt.as_deref(),
                    op.num_start,
                    op.num_restart.as_deref(),
                );
            }
            if n_sects > 0 {
                dirty.insert(doc_part.to_string(), ());
            }
        }
        NoteNumberTarget::Section(n) => {
            let doc = load_part(pkg, docs, doc_part)?;
            let sect = hf::find_sect_pr_mut(&mut doc.root, n).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::SectionNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )
            })?;
            notes::apply_numbering(
                sect,
                op.kind,
                op.num_fmt.as_deref(),
                op.num_start,
                op.num_restart.as_deref(),
            );
            dirty.insert(doc_part.to_string(), ());
        }
    }
    Ok(())
}

fn part_for_para(
    pkg: &Package,
    catalog: &[hf::HfPart],
    doc_part: &str,
    target: &ParaTarget,
    id: &str,
    i: usize,
) -> Result<String, Error> {
    match target {
        ParaTarget::Body(_) => Ok(doc_part.to_string()),
        ParaTarget::HeaderFooter {
            kind, hf_type, n, ..
        } => part_for_hf(catalog, *kind, *hf_type, *n, id, i),
        ParaTarget::InTextBox { host, .. } => part_for_host(pkg, catalog, doc_part, host, id, i),
        ParaTarget::InNote { kind, .. } => {
            notes::part_name(pkg, doc_part, *kind)?.ok_or_else(|| {
                Error::with_path(
                    ErrorKind::ParaNotFound { id: id.to_string() },
                    format!("ops[{i}].id"),
                )
            })
        }
        ParaTarget::InComment { .. } => Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: crate::error::IdReason::CommentPrefix,
            },
            format!("ops[{i}].id"),
        )),
    }
}

fn part_for_format_para(
    pkg: &Package,
    catalog: &[hf::HfPart],
    doc_part: &str,
    target: &ParaTarget,
    id: &str,
    i: usize,
) -> Result<String, Error> {
    match target {
        ParaTarget::InComment { .. } => comments::part_name(pkg, doc_part)?.ok_or_else(|| {
            Error::with_path(
                ErrorKind::ParaNotFound { id: id.to_string() },
                format!("ops[{i}].id"),
            )
        }),
        _ => part_for_para(pkg, catalog, doc_part, target, id, i),
    }
}

fn para_index_of(root: &Element, target: &ParaTarget) -> Option<usize> {
    match target {
        ParaTarget::Body(p) | ParaTarget::HeaderFooter { para: p, .. } => {
            comments::para_index_in_parent(root, p, false)
        }
        ParaTarget::InTextBox { host, key, para } => {
            let host_p = para::find_paragraph(root, host.body())?;
            let content = textbox::find_txbx_content_by_key(host_p, key)?;
            comments::para_index_in_parent(content, para, true)
        }
        ParaTarget::InNote {
            kind,
            note_id,
            para,
        } => {
            let note = notes::find_user_note(root, *kind, note_id)?;
            comments::para_index_in_parent(note, para, false)
        }
        ParaTarget::InComment { .. } => None,
    }
}

fn retarget_comment_last(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    comment_id: &str,
    old_last: Option<&str>,
) -> Result<(), Error> {
    let Some(part) = comments::part_name(pkg, doc_part)? else {
        return Ok(());
    };
    let new_last = {
        let doc = load_part(pkg, docs, &part)?;
        comments::find_comment(&doc.root, comment_id).and_then(comments::last_para_id)
    };
    let (Some(old), Some(new)) = (old_last, new_last.as_deref()) else {
        return Ok(());
    };
    if comments::para_ids_match(old, new) {
        return Ok(());
    }
    if let Some(ex) = comments::extended_part_name(pkg, doc_part)? {
        let doc = load_part(pkg, docs, &ex)?;
        comments::retarget_para_id(&mut doc.root, old, new);
        dirty.insert(ex, ());
    }
    comments::retarget_ids_parts(pkg, docs, dirty, doc_part, old, new)?;
    Ok(())
}

fn apply_insert_comment(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &InsertCommentOp,
    i: usize,
) -> Result<(), Error> {
    let start_key = comments::story_key(doc_part, &op.start_target).map_err(|_| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: crate::error::IdReason::CommentPrefix,
            },
            format!("ops[{i}].start"),
        )
    })?;
    let end_key = comments::story_key(doc_part, &op.end_target).map_err(|_| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: crate::error::IdReason::CommentPrefix,
            },
            format!("ops[{i}].end"),
        )
    })?;
    if start_key != end_key {
        return Err(Error::with_path(
            ErrorKind::CommentRangeCrossPart,
            format!("ops[{i}]"),
        ));
    }
    let host_part = part_for_para(pkg, catalog, doc_part, &op.start_target, &op.start, i)?;
    {
        let doc = load_part(pkg, docs, &host_part)?;
        let start_idx = para_index_of(&doc.root, &op.start_target).ok_or_else(|| {
            Error::with_path(
                ErrorKind::ParaNotFound {
                    id: op.start.clone(),
                },
                format!("ops[{i}].start"),
            )
        })?;
        let end_idx = para_index_of(&doc.root, &op.end_target).ok_or_else(|| {
            Error::with_path(
                ErrorKind::ParaNotFound { id: op.end.clone() },
                format!("ops[{i}].end"),
            )
        })?;
        if start_idx > end_idx {
            return Err(Error::with_path(
                ErrorKind::CommentRangeReversed,
                format!("ops[{i}]"),
            ));
        }
    }
    let comments_part = comments::ensure_comments_part(pkg, docs, dirty, doc_part)?;
    let extended_part = comments::ensure_extended_part(pkg, docs, dirty, doc_part)?;
    let people_part = comments::ensure_people_part(pkg, docs, dirty, doc_part)?;
    let (w_id, para_hex) = {
        let doc = load_part(pkg, docs, &comments_part)?;
        let w_id = comments::next_comment_id(&doc.root);
        let para_hex = para::next_para_hex(&doc.root);
        let (p_style, r_style) = comments::first_comment_styles(&doc.root);
        comments::append_comment(
            &mut doc.root,
            &w_id,
            &para_hex,
            &op.text,
            &op.author,
            &op.initials,
            op.date.as_deref(),
            &p_style,
            &r_style,
        );
        dirty.insert(comments_part.clone(), ());
        (w_id, para_hex)
    };
    {
        let doc = load_part(pkg, docs, &extended_part)?;
        comments::upsert_comment_ex(&mut doc.root, &para_hex, None, Some(false));
        dirty.insert(extended_part, ());
    }
    {
        let doc = load_part(pkg, docs, &people_part)?;
        comments::ensure_person(&mut doc.root, &op.author);
        dirty.insert(people_part, ());
    }
    comments::patch_ids_parts(pkg, docs, dirty, doc_part, &para_hex)?;
    {
        let doc = load_part(pkg, docs, &host_part)?;
        let p = find_insert_para_mut(&mut doc.root, &op.start_target).ok_or_else(|| {
            Error::with_path(
                ErrorKind::ParaNotFound {
                    id: op.start.clone(),
                },
                format!("ops[{i}].start"),
            )
        })?;
        comments::insert_range_start(p, &w_id);
    }
    {
        let doc = load_part(pkg, docs, &host_part)?;
        let p = find_insert_para_mut(&mut doc.root, &op.end_target).ok_or_else(|| {
            Error::with_path(
                ErrorKind::ParaNotFound { id: op.end.clone() },
                format!("ops[{i}].end"),
            )
        })?;
        comments::insert_range_end(p, &w_id);
        dirty.insert(host_part, ());
    }
    Ok(())
}

fn apply_reply_comment(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &ReplyCommentOp,
    i: usize,
) -> Result<(), Error> {
    let comments_part =
        comments::part_name(pkg, doc_part)?.ok_or_else(|| comments::comment_missing(&op.id, i))?;
    let (parent_last, p_style, r_style) = {
        let doc = load_part(pkg, docs, &comments_part)?;
        let parent = comments::find_comment(&doc.root, &op.comment_id)
            .ok_or_else(|| comments::comment_missing(&op.id, i))?;
        comments::reject_ink(parent, &op.id, i)?;
        let last = comments::last_para_id(parent).ok_or_else(|| comments::need_para_id(i))?;
        let (p_style, r_style) = comments::styles_of(parent);
        (last, p_style, r_style)
    };
    let comments_part = comments::ensure_comments_part(pkg, docs, dirty, doc_part)?;
    let extended_part = comments::ensure_extended_part(pkg, docs, dirty, doc_part)?;
    let people_part = comments::ensure_people_part(pkg, docs, dirty, doc_part)?;
    let (w_id, para_hex) = {
        let doc = load_part(pkg, docs, &comments_part)?;
        let w_id = comments::next_comment_id(&doc.root);
        let para_hex = para::next_para_hex(&doc.root);
        comments::append_comment(
            &mut doc.root,
            &w_id,
            &para_hex,
            &op.text,
            &op.author,
            &op.initials,
            op.date.as_deref(),
            &p_style,
            &r_style,
        );
        dirty.insert(comments_part, ());
        (w_id, para_hex)
    };
    comments::insert_reply_markers_in_hosts(
        pkg,
        docs,
        dirty,
        doc_part,
        catalog,
        &op.comment_id,
        &w_id,
    )?;
    {
        let doc = load_part(pkg, docs, &extended_part)?;
        comments::upsert_comment_ex(&mut doc.root, &para_hex, Some(&parent_last), Some(false));
        dirty.insert(extended_part, ());
    }
    {
        let doc = load_part(pkg, docs, &people_part)?;
        comments::ensure_person(&mut doc.root, &op.author);
        dirty.insert(people_part, ());
    }
    comments::patch_ids_parts(pkg, docs, dirty, doc_part, &para_hex)?;
    Ok(())
}

fn apply_delete_comment(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &DeleteCommentOp,
    i: usize,
) -> Result<(), Error> {
    let comments_part =
        comments::part_name(pkg, doc_part)?.ok_or_else(|| comments::comment_missing(&op.id, i))?;
    {
        let comments_doc = load_part(pkg, docs, &comments_part)?;
        if comments::find_comment(&comments_doc.root, &op.comment_id).is_none() {
            return Err(comments::comment_missing(&op.id, i));
        }
    }
    let extended_part = comments::extended_part_name(pkg, doc_part)?;
    let extended_root = if let Some(ex) = &extended_part {
        Some(load_part(pkg, docs, ex)?.root.clone())
    } else {
        None
    };
    let (ids, last_paras, embeds) = {
        let comments_doc = load_part(pkg, docs, &comments_part)?;
        let ids = comments::cascade_ids(&comments_doc.root, extended_root.as_ref(), &op.comment_id);
        let mut last_paras = Vec::new();
        let mut embeds = Vec::new();
        for id in &ids {
            if let Some(c) = comments::find_comment(&comments_doc.root, id) {
                if let Some(p) = comments::last_para_id(c) {
                    last_paras.push(p);
                }
                embeds.extend(comments::collect_blip_embeds(c));
            }
        }
        (ids, last_paras, embeds)
    };
    let mut media = Vec::new();
    for rid in &embeds {
        if let Ok(m) = picture::resolve_embed(pkg, &comments_part, rid) {
            media.push(m);
        }
    }
    let id_set: HashSet<String> = ids.iter().cloned().collect();
    comments::strip_markers_in_hosts(pkg, docs, dirty, doc_part, catalog, &id_set)?;
    {
        let doc = load_part(pkg, docs, &comments_part)?;
        for id in &ids {
            comments::remove_comment(&mut doc.root, id);
        }
        dirty.insert(comments_part.clone(), ());
    }
    if let Some(ex) = &extended_part {
        let doc = load_part(pkg, docs, ex)?;
        for pid in &last_paras {
            comments::remove_comment_ex_for_para(&mut doc.root, pid);
        }
        dirty.insert(ex.clone(), ());
    }
    for pid in &last_paras {
        comments::remove_ids_for_para(pkg, docs, dirty, doc_part, pid)?;
    }
    let comments_root = docs.get(&comments_part).unwrap().root.clone();
    for rid in embeds {
        picture::remove_rel_if_unused(pkg, &comments_part, &rid, &comments_root)?;
    }
    for m in media {
        if !picture::media_still_used(pkg, &m)? {
            pkg.remove(&m);
        }
    }
    Ok(())
}

fn apply_set_comment_text(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    op: &SetCommentTextOp,
    i: usize,
) -> Result<(), Error> {
    let comments_part =
        comments::part_name(pkg, doc_part)?.ok_or_else(|| comments::comment_missing(&op.id, i))?;
    let old_last = {
        let doc = load_part(pkg, docs, &comments_part)?;
        let comment = comments::find_comment(&doc.root, &op.comment_id)
            .ok_or_else(|| comments::comment_missing(&op.id, i))?;
        comments::reject_ink(comment, &op.id, i)?;
        comments::last_para_id(comment)
    };
    {
        let doc = load_part(pkg, docs, &comments_part)?;
        let comment = comments::find_comment_mut(&mut doc.root, &op.comment_id)
            .ok_or_else(|| comments::comment_missing(&op.id, i))?;
        comments::set_visible_keep_first(comment, &op.text);
        dirty.insert(comments_part, ());
    }
    retarget_comment_last(
        pkg,
        docs,
        dirty,
        doc_part,
        &op.comment_id,
        old_last.as_deref(),
    )?;
    Ok(())
}

fn apply_set_comment_meta(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    op: &SetCommentMetaOp,
    i: usize,
) -> Result<(), Error> {
    let comments_part =
        comments::part_name(pkg, doc_part)?.ok_or_else(|| comments::comment_missing(&op.id, i))?;
    {
        let doc = load_part(pkg, docs, &comments_part)?;
        let comment = comments::find_comment_mut(&mut doc.root, &op.comment_id)
            .ok_or_else(|| comments::comment_missing(&op.id, i))?;
        if let Some(a) = &op.author {
            set_w_attr(comment, "author", a);
        }
        if let Some(ini) = &op.initials {
            set_w_attr(comment, "initials", ini);
        }
        if let Some(d) = &op.date {
            set_w_attr(comment, "date", d);
        }
        dirty.insert(comments_part, ());
    }
    if let Some(author) = &op.author {
        let people_part = comments::ensure_people_part(pkg, docs, dirty, doc_part)?;
        let doc = load_part(pkg, docs, &people_part)?;
        comments::ensure_person(&mut doc.root, author);
        dirty.insert(people_part, ());
    }
    Ok(())
}

fn apply_set_comment_done(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    op: &SetCommentDoneOp,
    i: usize,
) -> Result<(), Error> {
    let comments_part =
        comments::part_name(pkg, doc_part)?.ok_or_else(|| comments::comment_missing(&op.id, i))?;
    let last = {
        let doc = load_part(pkg, docs, &comments_part)?;
        let comment = comments::find_comment(&doc.root, &op.comment_id)
            .ok_or_else(|| comments::comment_missing(&op.id, i))?;
        comments::last_para_id(comment).ok_or_else(|| comments::need_para_id(i))?
    };
    let extended_part = comments::ensure_extended_part(pkg, docs, dirty, doc_part)?;
    {
        let comments_doc = load_part(pkg, docs, &comments_part)?;
        let comments_root = comments_doc.root.clone();
        let ex = load_part(pkg, docs, &extended_part)?;
        if comments::fill_missing_comment_ex(&comments_root, &mut ex.root).is_err() {
            return Err(comments::need_para_id(i));
        }
        if !comments::set_done_on_para(&mut ex.root, &last, op.done) {
            comments::upsert_comment_ex(&mut ex.root, &last, None, Some(op.done));
        }
        dirty.insert(extended_part, ());
    }
    Ok(())
}

fn set_w_attr(el: &mut Element, local: &str, val: &str) {
    if let Some((_, v)) = el
        .attrs
        .iter_mut()
        .find(|(k, _)| k.rsplit(':').next() == Some(local))
    {
        *v = val.to_string();
        return;
    }
    el.set_attr(&el.prefixed(local), val);
}

fn reject_ink_pic(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    doc_part: &str,
    target: &picture::PicTarget,
    id: &str,
    i: usize,
) -> Result<(), Error> {
    let picture::PicTarget::InComment { comment_id, .. } = target else {
        return Ok(());
    };
    let part = comments::part_name(pkg, doc_part)?.ok_or_else(|| {
        Error::with_path(
            ErrorKind::PictureNotFound { id: id.to_string() },
            format!("ops[{i}].id"),
        )
    })?;
    let doc = load_part(pkg, docs, &part)?;
    let comment = comments::find_comment(&doc.root, comment_id).ok_or_else(|| {
        Error::with_path(
            ErrorKind::PictureNotFound { id: id.to_string() },
            format!("ops[{i}].id"),
        )
    })?;
    comments::reject_ink(comment, id, i)
}
