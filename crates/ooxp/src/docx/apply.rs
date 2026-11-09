//! 按 id 改段落文本、节上距边界、页眉页脚是否链接前节，或 DrawingML 图。

use std::collections::{HashMap, HashSet};

use crate::error::{Error, ErrorKind};
use crate::opc::{self, Package};
use crate::xml::{self, Document, Element, Node};

use super::biblio;
use super::charts;
use super::comments;
use super::east_asia;
use super::font_embed;
use super::group::{GroupHost, GroupMember};
use super::hf;
use super::ink;
use super::link;
use super::notes;
use super::ole;
use super::page_bg;
use super::para::{self, HostPara, NoteKind, ParaTarget};
use super::picture;
use super::protect;
use super::request::DocxOp;
use super::smartart;
use super::textbox;
use super::watermark;

pub fn apply(bytes: &[u8], ops: &[DocxOp]) -> Result<Vec<u8>, Error> {
    apply_pkg(bytes, ops)?.save_to_vec()
}

pub fn dry_run(bytes: &[u8], ops: &[DocxOp]) -> Result<Vec<String>, Error> {
    Ok(apply_pkg(bytes, ops)?.dirty_part_names())
}

fn apply_pkg(bytes: &[u8], ops: &[DocxOp]) -> Result<crate::opc::Package, Error> {
    let mut pkg = Package::open_bytes(bytes.to_vec())?;
    opc::reject_strict(&pkg)?;
    let doc_part = opc::office_document_part(&pkg)?;
    let mut doc_xml = xml::parse(pkg.get(&doc_part)?)?;
    super::para::pin_paragraph_indexes(&mut doc_xml.root);
    super::tbl::pin_table_indexes(&mut doc_xml.root);
    super::inline_pin::pin_inline_indexes(&pkg, &doc_part, &mut doc_xml.root);
    let mut catalog = hf::catalog(&pkg, &doc_part, &doc_xml.root)?;
    let hf_freeze = hf::HfFreeze::capture(&catalog);

    // 按部件缓存已解析的文档；最后一并写回脏部件。
    let mut docs: HashMap<String, Document> = HashMap::new();
    docs.insert(doc_part.clone(), doc_xml);
    let mut dirty: HashMap<String, ()> = HashMap::new();
    let mut removed: HashSet<String> = HashSet::new();

    for (i, op) in ops.iter().enumerate() {
        match op {
            DocxOp::SetParaText(op) => {
                super::para::apply_para_text(
                    &mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::SetParaRuns(op) => {
                super::para::apply_para_runs(
                    &mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::SetHeaderFooterDistance(op) => {
                let doc = docs.get_mut(&doc_part).unwrap();
                super::hf::apply_hf_distance(&mut doc.root, op, i)?;
                dirty.insert(doc_part.clone(), ());
            }
            DocxOp::SetHeaderFooterLink(op) => {
                super::hf::apply_hf_link(
                    &mut pkg,
                    &mut docs,
                    &mut dirty,
                    &mut removed,
                    &doc_part,
                    op,
                    i,
                )?;
                super::hf::refresh_hf_catalog(&pkg, &docs, &doc_part, &hf_freeze, &mut catalog)?;
            }
            DocxOp::InsertHeaderFooter(op) => {
                super::hf::apply_insert_hf(&mut pkg, &mut docs, &mut dirty, &doc_part, op, i)?;
                super::hf::refresh_hf_catalog(&pkg, &docs, &doc_part, &hf_freeze, &mut catalog)?;
            }
            DocxOp::InsertSection(op) => {
                super::sect::apply_insert_section(&mut docs, &mut dirty, &doc_part, op, i)?;
                super::hf::refresh_hf_catalog(&pkg, &docs, &doc_part, &hf_freeze, &mut catalog)?;
            }
            DocxOp::DeleteSection(op) => {
                super::sect::apply_delete_section(
                    &mut pkg,
                    &mut docs,
                    &mut dirty,
                    &mut removed,
                    &doc_part,
                    op,
                    i,
                )?;
                super::hf::refresh_hf_catalog(&pkg, &docs, &doc_part, &hf_freeze, &mut catalog)?;
            }
            DocxOp::InsertToc(op) => {
                super::field::apply_insert_toc(
                    &pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::DeleteToc(op) => {
                super::field::apply_delete_toc(
                    &pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::InsertCaption(op) => {
                super::field::apply_insert_caption(
                    &pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::DeleteCaption(op) => {
                super::field::apply_delete_caption(
                    &pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::InsertCrossRef(op) => {
                super::field::apply_insert_cross_ref(
                    &pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::DeleteCrossRef(op) => {
                super::field::apply_delete_cross_ref(
                    &pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::InsertIndex(op) => {
                super::field::apply_insert_index(
                    &pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::DeleteIndex(op) => {
                super::field::apply_delete_index(
                    &pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::InsertIndexEntry(op) => {
                super::field::apply_insert_index_entry(
                    &pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::DeleteIndexEntry(op) => {
                super::field::apply_delete_index_entry(
                    &pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::InsertSource(op) => {
                super::biblio::apply_insert_source(
                    &mut pkg, &mut docs, &mut dirty, &doc_part, op, i,
                )?;
            }
            DocxOp::DeleteSource(op) => {
                super::biblio::apply_delete_source(
                    &mut pkg, &mut docs, &mut dirty, &doc_part, op, i,
                )?;
            }
            DocxOp::InsertCitation(op) => {
                super::biblio::apply_insert_citation(
                    &pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::DeleteCitation(op) => {
                super::biblio::apply_delete_citation(
                    &pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::InsertBibliography(op) => {
                super::biblio::apply_insert_bibliography(
                    &pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::DeleteBibliography(op) => {
                super::biblio::apply_delete_bibliography(
                    &pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
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
                super::glossary::apply_insert_doc_part(
                    &mut pkg, &mut docs, &mut dirty, &doc_part, op, i,
                )?;
            }
            DocxOp::DeleteDocPart(op) => {
                super::glossary::apply_delete_doc_part(
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
                super::glossary::apply_insert_doc_part_content(
                    &pkg, &mut docs, &mut dirty, &doc_part, op, i,
                )?;
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
                super::sect::apply_page_borders(&mut docs, &mut dirty, &doc_part, op, i)?;
            }
            DocxOp::SetFormProt(op) => {
                super::sect::apply_set_form_prot(&mut docs, &mut dirty, &doc_part, op, i)?;
            }
            DocxOp::InsertMath(op) => {
                super::math::apply_insert_math(
                    &pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::SetMath(op) => {
                super::math::apply_set_math(
                    &pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::DeleteMath(op) => {
                super::math::apply_delete_math(
                    &pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::InsertRuby(op) => {
                super::ruby::apply_insert_ruby(
                    &pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::SetRuby(op) => {
                super::ruby::apply_set_ruby(
                    &pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::DeleteRuby(op) => {
                super::ruby::apply_delete_ruby(
                    &pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
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
                super::sdt::apply_insert_sdt(
                    &mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::SetContentControl(op) => {
                super::sdt::apply_set_sdt(
                    &mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::InsertRepeatingItem(op) => {
                super::sdt::apply_insert_repeating_item(
                    &mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::DeleteRepeatingItem(op) => {
                super::sdt::apply_delete_repeating_item(
                    &mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::DeleteContentControl(op) => {
                super::sdt::apply_delete_sdt(
                    &pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::SetTitlePage(op) => {
                super::hf::apply_title_page(&mut docs, &mut dirty, &doc_part, op, i)?;
                super::hf::refresh_hf_catalog(&pkg, &docs, &doc_part, &hf_freeze, &mut catalog)?;
            }
            DocxOp::SetEvenAndOddHeaders(op) => {
                super::hf::apply_even_and_odd(&mut pkg, &mut docs, &mut dirty, &doc_part, op, i)?;
                super::hf::refresh_hf_catalog(&pkg, &docs, &doc_part, &hf_freeze, &mut catalog)?;
            }
            DocxOp::SetPageMargin(op) => {
                super::sect::apply_page_margin(&mut docs, &mut dirty, &doc_part, op, i)?;
            }
            DocxOp::SetPageSize(op) => {
                super::sect::apply_page_size(&mut docs, &mut dirty, &doc_part, op, i)?;
            }
            DocxOp::SetColumns(op) => {
                super::sect::apply_columns(&mut docs, &mut dirty, &doc_part, op, i)?;
            }
            DocxOp::SetVerticalAlign(op) => {
                super::sect::apply_valign(&mut docs, &mut dirty, &doc_part, op, i)?;
            }
            DocxOp::SetLineNumbering(op) => {
                super::sect::apply_line_numbering(&mut docs, &mut dirty, &doc_part, op, i)?;
            }
            DocxOp::SetDocGrid(op) => {
                super::sect::apply_doc_grid(&mut docs, &mut dirty, &doc_part, op, i)?;
            }
            DocxOp::SetRtlGutter(op) => {
                super::sect::apply_rtl_gutter(&mut docs, &mut dirty, &doc_part, op, i)?;
            }
            DocxOp::SetMirrorMargins(op) => {
                super::sect::apply_mirror_margins(
                    &mut pkg, &mut docs, &mut dirty, &doc_part, op, i,
                )?;
            }
            DocxOp::SetGutterAtTop(op) => {
                super::sect::apply_gutter_at_top(
                    &mut pkg, &mut docs, &mut dirty, &doc_part, op, i,
                )?;
            }
            DocxOp::ReplacePicture(op) => {
                super::picture::apply_replace_pic(
                    &mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::InsertPicture(op) => {
                super::picture::apply_insert_pic(
                    &mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::DeletePicture(op) => {
                super::picture::apply_delete_pic(
                    &mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::SetPictureSize(op) => {
                super::picture::apply_pic_size(
                    &mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::SetPictureWrap(op) => {
                super::picture::apply_pic_wrap(
                    &mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::InsertTextBox(op) => {
                super::textbox::apply_insert_txbx(
                    &mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::DeleteTextBox(op) => {
                super::textbox::apply_delete_txbx(
                    &mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::SetTextBoxSize(op) => {
                super::textbox::apply_txbx_size(
                    &mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::SetTextBoxWrap(op) => {
                super::textbox::apply_txbx_wrap(
                    &mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::SetTextBoxFill(op) => {
                super::textbox::apply_txbx_fill(
                    &mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::SetTextBoxLine(op) => {
                super::textbox::apply_txbx_line(
                    &mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::SetTextBoxDirection(op) => {
                super::textbox::apply_txbx_dir(
                    &mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::SetTextBoxTextAnchor(op) => {
                super::textbox::apply_txbx_text_anchor(
                    &mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::InsertConnector(op) => {
                super::connector::apply_insert_connector(
                    &mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::DeleteConnector(op) => {
                super::connector::apply_delete_connector(
                    &mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::InsertShape(op) => {
                super::shape::apply_insert_shape(
                    &mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::DeleteShape(op) => {
                super::shape::apply_delete_shape(
                    &mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::SetShapeSize(op) => {
                super::shape::apply_shape_size(
                    &mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::SetShapePos(op) => {
                super::shape::apply_shape_pos(
                    &mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::SetShapeFill(op) => {
                super::shape::apply_shape_fill(
                    &mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::SetShapeLine(op) => {
                super::shape::apply_shape_line(
                    &mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::GroupShapes(op) => {
                super::group::apply_group_shapes(
                    &mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::Ungroup(op) => {
                super::group::apply_ungroup(
                    &mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::InsertPara(op) => {
                super::para::apply_insert_para(
                    &mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::DeletePara(op) => {
                super::para::apply_delete_para(
                    &mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::SetTextBoxLink(op) => {
                super::textbox::apply_txbx_link(
                    &mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::InsertFootnote(op) => {
                super::notes::apply_insert_note(
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
                super::notes::apply_insert_note(
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
                super::notes::apply_delete_note(
                    &mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::DeleteEndnote(op) => {
                super::notes::apply_delete_note(
                    &mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::SetNoteNumbering(op) => {
                super::notes::apply_note_numbering(
                    &mut pkg, &mut docs, &mut dirty, &doc_part, op, i,
                )?;
            }
            DocxOp::InsertComment(op) => {
                super::comments::apply_insert_comment(
                    &mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::ReplyComment(op) => {
                super::comments::apply_reply_comment(
                    &mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::DeleteComment(op) => {
                super::comments::apply_delete_comment(
                    &mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::SetCommentText(op) => {
                super::comments::apply_set_comment_text(
                    &mut pkg, &mut docs, &mut dirty, &doc_part, op, i,
                )?;
            }
            DocxOp::SetCommentMeta(op) => {
                super::comments::apply_set_comment_meta(
                    &mut pkg, &mut docs, &mut dirty, &doc_part, op, i,
                )?;
            }
            DocxOp::SetCommentDone(op) => {
                super::comments::apply_set_comment_done(
                    &mut pkg, &mut docs, &mut dirty, &doc_part, op, i,
                )?;
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
                super::hf::refresh_hf_catalog(&pkg, &docs, &doc_part, &hf_freeze, &mut catalog)?;
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
                super::tbl::apply_insert_table(
                    &pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::DeleteTable(op) => {
                super::tbl::apply_delete_table(
                    &pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::InsertRow(op) => {
                super::tbl::apply_insert_row(
                    &pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::DeleteRow(op) => {
                super::tbl::apply_delete_row(
                    &pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::InsertCol(op) => {
                super::tbl::apply_insert_col(
                    &pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::DeleteCol(op) => {
                super::tbl::apply_delete_col(
                    &pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::MergeCells(op) => {
                super::tbl::apply_merge_cells(
                    &pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::SplitCells(op) => {
                super::tbl::apply_split_cells(
                    &pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::AcceptRevision(op) => {
                super::revision::apply_rev(
                    &pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, true, i,
                )?;
            }
            DocxOp::RejectRevision(op) => {
                super::revision::apply_rev(
                    &pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, false, i,
                )?;
            }
            DocxOp::InsertHyperlink(op) => {
                super::link::apply_insert_hyperlink(
                    &mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::SetHyperlink(op) => {
                super::link::apply_set_hyperlink(
                    &mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::DeleteHyperlink(op) => {
                super::link::apply_delete_hyperlink(
                    &mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::InsertBookmark(op) => {
                super::link::apply_insert_bookmark(
                    &pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::SetBookmark(op) => {
                super::link::apply_set_bookmark(
                    &pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::DeleteBookmark(op) => {
                super::link::apply_delete_bookmark(
                    &pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::InsertEditRange(op) => {
                super::link::apply_insert_edit_range(
                    &pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::DeleteEditRange(op) => {
                super::link::apply_delete_edit_range(
                    &pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::InsertField(op) => {
                super::field::apply_insert_field(
                    &pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::SetField(op) => {
                super::field::apply_set_field(
                    &pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::DeleteField(op) => {
                super::field::apply_delete_field(
                    &pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::InsertNumbering(op) => {
                super::numbering::apply_insert_numbering(
                    &mut pkg, &mut docs, &mut dirty, &doc_part, op,
                )?;
            }
            DocxOp::SetNumbering(op) => {
                super::numbering::apply_set_numbering(
                    &pkg, &mut docs, &mut dirty, &doc_part, op, i,
                )?;
            }
            DocxOp::SetParaNumbering(op) => {
                super::numbering::apply_set_para_numbering(
                    &pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::SetStyle(op) => {
                super::styles::apply_set_style(&pkg, &mut docs, &mut dirty, &doc_part, op, i)?;
            }
            DocxOp::SetParaStyle(op) => {
                super::styles::apply_set_para_style(
                    &pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::InsertStyle(op) => {
                super::styles::apply_insert_style(
                    &mut pkg, &mut docs, &mut dirty, &doc_part, op, i,
                )?;
            }
            DocxOp::DeleteStyle(op) => {
                super::styles::apply_delete_style(&pkg, &mut docs, &mut dirty, &doc_part, op, i)?;
            }
            DocxOp::SetRunStyle(op) => {
                super::styles::apply_set_run_style(
                    &pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::SetTableStyle(op) => {
                super::styles::apply_set_table_style(
                    &pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::SetTableFloat(op) => {
                super::tbl::apply_set_table_float(
                    &pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::SetTableWidth(op) => {
                super::tbl::apply_set_table_width(
                    &pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::SetTableAlign(op) => {
                super::tbl::apply_set_table_align(
                    &pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::SetTableRowHeight(op) => {
                super::tbl::apply_set_table_row_height(
                    &pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::SetTableCellMar(op) => {
                super::tbl::apply_set_table_cell_mar(
                    &pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::SetTableBorders(op) => {
                super::tbl::apply_set_table_borders(
                    &pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::SetCellBorders(op) => {
                super::tbl::apply_set_cell_borders(
                    &pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::SetCellFormat(op) => {
                super::tbl::apply_set_cell_format(
                    &pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::SetParaFormat(op) => {
                super::format::apply_set_para_format(
                    &pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
            }
            DocxOp::SetRunFormat(op) => {
                super::format::apply_set_run_format(
                    &pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i,
                )?;
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

    Ok(pkg)
}

pub(crate) fn para_target_path(root: &Element, target: &ParaTarget) -> Option<Vec<usize>> {
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
        ParaTarget::InNote {
            kind,
            note_id,
            para,
        } => {
            let note_path = note_child_path(root, *kind, note_id)?;
            let note = para::element_at(root, &note_path)?;
            let mut path = note_path;
            path.extend(para::find_paragraph_path(note, para)?);
            Some(path)
        }
        ParaTarget::InComment { comment_id, para } => {
            let comment_path = comment_child_path(root, comment_id)?;
            let comment = para::element_at(root, &comment_path)?;
            let mut path = comment_path;
            path.extend(para::find_paragraph_path(comment, para)?);
            Some(path)
        }
    }
}

fn note_child_path(root: &Element, kind: para::NoteKind, note_id: &str) -> Option<Vec<usize>> {
    root.children.iter().enumerate().find_map(|(i, n)| match n {
        Node::Element(e)
            if e.local_name() == kind.item_local()
                && notes::ids_match(e.attr_local("id").unwrap_or(""), note_id)
                && notes::is_user_note(e) =>
        {
            Some(vec![i])
        }
        _ => None,
    })
}

fn comment_child_path(root: &Element, comment_id: &str) -> Option<Vec<usize>> {
    root.children.iter().enumerate().find_map(|(i, n)| match n {
        Node::Element(e)
            if e.local_name() == "comment" && e.attr_local("id") == Some(comment_id) =>
        {
            Some(vec![i])
        }
        _ => None,
    })
}
pub(crate) fn story_xml_mut<'a>(
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
pub(crate) fn current_part_bytes(
    pkg: &Package,
    docs: &HashMap<String, Document>,
    part: &str,
) -> Result<Vec<u8>, Error> {
    if let Some(doc) = docs.get(part) {
        return doc.to_bytes();
    }
    Ok(pkg.get(part)?.to_vec())
}
pub(crate) fn load_part<'a>(
    pkg: &Package,
    docs: &'a mut HashMap<String, Document>,
    part_name: &str,
) -> Result<&'a mut Document, Error> {
    if !docs.contains_key(part_name) {
        let mut parsed = xml::parse(pkg.get(part_name)?)?;
        super::para::pin_paragraph_indexes(&mut parsed.root);
        super::tbl::pin_table_indexes(&mut parsed.root);
        super::inline_pin::pin_inline_indexes(pkg, part_name, &mut parsed.root);
        docs.insert(part_name.to_string(), parsed);
    }
    Ok(docs.get_mut(part_name).unwrap())
}
pub(crate) fn ink_doc_mut<'a>(
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
pub(crate) fn part_for_pic(
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
pub(crate) fn part_for_hf(
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
pub(crate) fn part_for_host(
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
pub(crate) fn part_for_txbx(
    pkg: &Package,
    catalog: &[hf::HfPart],
    doc_part: &str,
    target: &para::TxbxTarget,
    id: &str,
    i: usize,
) -> Result<String, Error> {
    part_for_host(pkg, catalog, doc_part, &target.host, id, i)
}
pub(crate) fn find_insert_para_mut<'a>(
    root: &'a mut Element,
    target: &ParaTarget,
) -> Option<&'a mut Element> {
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
pub(crate) fn part_for_group_host(
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
pub(crate) fn part_for_group_member(
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
pub(crate) fn part_for_para(
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
pub(crate) fn part_for_format_para(
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
pub(crate) fn para_index_of(root: &Element, target: &ParaTarget) -> Option<usize> {
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
pub(crate) fn set_w_attr(el: &mut Element, local: &str, val: &str) {
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
