//! 按形状 id 替换可见文本，并处理幻灯片图与图表。

use crate::error::{Error, ErrorKind};
use crate::opc::{self, Package};
use crate::xml;

use super::chart;
use super::comments;
use super::connector;
use super::custom_show;
use super::field;
use super::group;
use super::handout;
use super::hyperlink;
use super::ink;
use super::master;
use super::media;
use super::model3d;
use super::notes;
use super::ole;
use super::omml;
use super::picture;
use super::protect;
use super::request::PptxOp;
use super::section;
use super::shape;
use super::show;
use super::slides;
use super::smartart;
use super::table;
use super::theme;
use super::theme_map;
use super::timing;
use super::transition;
use super::zoom;

pub fn apply(bytes: &[u8], ops: &[PptxOp]) -> Result<Vec<u8>, Error> {
    apply_pkg(bytes, ops)?.save_to_vec()
}

pub fn dry_run(bytes: &[u8], ops: &[PptxOp]) -> Result<Vec<String>, Error> {
    Ok(apply_pkg(bytes, ops)?.dirty_part_names())
}

fn apply_pkg(bytes: &[u8], ops: &[PptxOp]) -> Result<crate::opc::Package, Error> {
    let _pins = super::list_pin::PinGuard::enter();
    let mut pkg = Package::open_bytes(bytes.to_vec())?;
    opc::reject_strict(&pkg)?;
    let pres_part = opc::office_document_part(&pkg)?;
    let mut pres = super::list_pin::parse_part(&pkg, &pres_part)?;
    section::pin(&mut pres.root);
    custom_show::pin(&mut pres.root);
    master::capture_freeze(&pkg, &pres_part, &pres.root)?;
    let mut slides = shape::list_slides(&pkg, &pres_part, &pres.root)?;
    let frozen = shape::SlideFreeze::capture(&slides);

    for (i, op) in ops.iter().enumerate() {
        match op {
            PptxOp::SetShapeText(op) => {
                shape::op_set_shape_text(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::SetShapeRuns(op) => {
                shape::op_set_shape_runs(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::SetShapeFormat(op) => {
                shape::op_set_shape_format(&mut pkg, &pres_part, &pres.root, &slides, op, i)?;
            }
            PptxOp::InsertShape(op) => {
                shape::op_insert_shape(&mut pkg, &pres_part, &mut pres, &slides, op, i)?;
            }
            PptxOp::InsertConnector(op) => {
                connector::op_insert_connector(&mut pkg, &pres_part, &mut pres, &slides, op, i)?;
            }
            PptxOp::DeleteConnector(op) => {
                connector::op_delete_connector(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::GroupShapes(op) => {
                group::op_group_shapes(&mut pkg, &slides, op, i)?;
            }
            PptxOp::Ungroup(op) => {
                group::op_ungroup(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::DeleteShape(op) => {
                shape::op_delete_shape(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::SetShapePos(op) => {
                shape::op_set_shape_pos(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::SetShapeSize(op) => {
                shape::op_set_shape_size(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::SetShapeFill(op) => {
                shape::op_set_shape_fill(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::SetShapeLine(op) => {
                shape::op_set_shape_line(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::SetShapeGeom(op) => {
                shape::op_set_shape_geom(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::SetConnector(op) => {
                connector::op_set_connector(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::SetNotesText(op) => {
                notes::op_set_notes_text(&mut pkg, &pres_part, &mut pres, &slides, op, i)?;
            }
            PptxOp::InsertComment(op) => {
                comments::op_insert_comment(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::ReplyComment(op) => {
                comments::op_reply_comment(&mut pkg, &pres_part, &slides, op, i)?;
            }
            PptxOp::DeleteComment(op) => {
                comments::op_delete_comment(&mut pkg, &slides, op, i)?;
            }
            PptxOp::SetCommentText(op) => {
                comments::op_set_comment_text(&mut pkg, &slides, op, i)?;
            }
            PptxOp::SetCommentMeta(op) => {
                comments::op_set_comment_meta(&mut pkg, &pres_part, &slides, op, i)?;
            }
            PptxOp::SetCommentDone(op) => {
                comments::op_set_comment_done(&mut pkg, &slides, op, i)?;
            }
            PptxOp::ReplacePicture(op) => {
                picture::op_replace_picture(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::InsertPicture(op) => {
                picture::op_insert_picture(&mut pkg, &pres_part, &mut pres, &slides, op, i)?;
            }
            PptxOp::DeletePicture(op) => {
                picture::op_delete_picture(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::SetPictureSize(op) => {
                picture::op_set_picture_size(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::SetPicturePos(op) => {
                picture::op_set_picture_pos(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::InsertVideo(op) => {
                media::op_insert_video(&mut pkg, &pres_part, &mut pres, &slides, op, i)?;
            }
            PptxOp::InsertAudio(op) => {
                media::op_insert_audio(&mut pkg, &pres_part, &mut pres, &slides, op, i)?;
            }
            PptxOp::DeleteMedia(op) => {
                media::op_delete_media(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::SetMedia(op) => {
                media::op_set_media(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::InsertSlideSection(op) => {
                section::op_insert_slide_section(&mut pkg, &pres_part, &mut pres, &slides, op, i)?;
            }
            PptxOp::SetSlideSectionName(op) => {
                section::set_name(&mut pkg, &pres_part, &mut pres, op, i)?;
            }
            PptxOp::DeleteSlideSection(op) => {
                section::delete(&mut pkg, &pres_part, &mut pres, op, i)?;
            }
            PptxOp::SetSlideSection(op) => {
                section::op_set_slide_section(
                    &mut pkg,
                    &pres_part,
                    &mut pres,
                    &mut slides,
                    &frozen,
                    op,
                    i,
                )?;
            }
            PptxOp::InsertCustomShow(op) => {
                custom_show::op_insert_custom_show(
                    &mut pkg, &pres_part, &mut pres, &slides, op, i,
                )?;
            }
            PptxOp::SetCustomShow(op) => {
                custom_show::op_set_custom_show(&mut pkg, &pres_part, &mut pres, &slides, op, i)?;
            }
            PptxOp::DeleteCustomShow(op) => {
                custom_show::delete(&mut pkg, &pres_part, &mut pres, op, i)?;
            }
            PptxOp::InsertOle(op) => {
                ole::op_insert_ole(&mut pkg, &pres_part, &mut pres, &slides, op, i)?;
            }
            PptxOp::DeleteOle(op) => {
                ole::op_delete_ole(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::SetOleSize(op) => {
                ole::op_set_ole_size(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::InsertModel3d(op) => {
                model3d::op_insert_model3d(&mut pkg, &pres_part, &mut pres, &slides, op, i)?;
            }
            PptxOp::ReplaceModel3d(op) => {
                model3d::op_replace_model3d(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::DeleteModel3d(op) => {
                model3d::op_delete_model3d(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::SetModel3dSize(op) => {
                model3d::op_set_model3d_size(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::SetModel3dPos(op) => {
                model3d::op_set_model3d_pos(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::InsertInk(op) => {
                ink::op_insert_ink(&mut pkg, &pres_part, &mut pres, &slides, op, i)?;
            }
            PptxOp::ReplaceInk(op) => {
                ink::op_replace_ink(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::DeleteInk(op) => {
                ink::op_delete_ink(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::SetInkSize(op) => {
                ink::op_set_ink_size(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::SetInkPos(op) => {
                ink::op_set_ink_pos(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::SetModel3dCamera(op) => {
                model3d::op_set_model3d_camera(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::InsertMath(op) => {
                omml::op_insert_math(&mut pkg, &pres_part, &mut pres, &slides, op, i)?;
            }
            PptxOp::SetMath(op) => {
                omml::op_set_math(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::DeleteMath(op) => {
                omml::op_delete_math(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::SetMathSize(op) => {
                omml::op_set_math_size(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::SetMathPos(op) => {
                omml::op_set_math_pos(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::InsertZoom(op) => {
                zoom::op_insert_zoom(&mut pkg, &pres_part, &mut pres, &slides, op, i)?;
            }
            PptxOp::DeleteZoom(op) => {
                zoom::op_delete_zoom(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::SetZoom(op) => {
                zoom::op_set_zoom(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::SetShowPr(op) => {
                show::apply(&mut pkg, &pres_part, op, i)?;
            }
            PptxOp::SetProtection(op) => {
                protect::apply(&mut pkg, &pres_part, &mut pres, op, i)?;
            }
            PptxOp::InsertChart(op) => {
                chart::op_insert_chart(&mut pkg, &pres_part, &mut pres, &slides, op, i)?;
            }
            PptxOp::DeleteChart(op) => {
                chart::op_delete_chart(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::SetChartTitle(op) => {
                chart::op_set_chart_title(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::SetChartSeries(op) => {
                chart::op_set_chart_series(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::AddChartSeries(op) => {
                chart::op_add_chart_series(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::DeleteChartSeries(op) => {
                chart::op_delete_chart_series(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::SetChartType(op) => {
                chart::op_set_chart_type(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::SetChartAxisTitle(op) => {
                chart::op_set_chart_axis_title(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::SetChartLegend(op) => {
                chart::op_set_chart_legend(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::SetChartDataLabels(op) => {
                chart::op_set_chart_data_labels(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::SetChartSize(op) => {
                chart::op_set_chart_size(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::SetChartPos(op) => {
                chart::op_set_chart_pos(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::InsertTrendline(op) => {
                chart::op_insert_trendline(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::SetTrendline(op) => {
                chart::op_set_trendline(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::DeleteTrendline(op) => {
                chart::op_delete_trendline(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::InsertErrorBars(op) => {
                chart::op_insert_error_bars(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::SetErrorBars(op) => {
                chart::op_set_error_bars(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::DeleteErrorBars(op) => {
                chart::op_delete_error_bars(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::SetChartSeriesAxis(op) => {
                chart::op_set_chart_series_axis(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::InsertSmartArt(op) => {
                smartart::op_insert_smart_art(&mut pkg, &pres_part, &mut pres, &slides, op, i)?;
            }
            PptxOp::DeleteSmartArt(op) => {
                smartart::op_delete_smart_art(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::SetSmartArtText(op) => {
                smartart::op_set_smart_art_text(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::AddSmartArtPoint(op) => {
                smartart::op_add_smart_art_point(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::DeleteSmartArtPoint(op) => {
                smartart::op_delete_smart_art_point(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::SetSmartArtSize(op) => {
                smartart::op_set_smart_art_size(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::SetSmartArtPos(op) => {
                smartart::op_set_smart_art_pos(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::SetSmartArtColors(op) => {
                smartart::op_set_smart_art_colors(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::SetSmartArtStyle(op) => {
                smartart::op_set_smart_art_style(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::SetSmartArtLayout(op) => {
                smartart::op_set_smart_art_layout(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::InsertTable(op) => {
                table::op_insert_table(&mut pkg, &pres_part, &mut pres, &slides, op, i)?;
            }
            PptxOp::DeleteTable(op) => {
                table::op_delete_table(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::InsertRow(op) => {
                table::op_insert_row(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::DeleteRow(op) => {
                table::op_delete_row(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::InsertCol(op) => {
                table::op_insert_col(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::DeleteCol(op) => {
                table::op_delete_col(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::SetTableCellText(op) => {
                table::op_set_table_cell_text(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::SetTableCellFill(op) => {
                table::op_set_table_cell_fill(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::SetTableCellBorders(op) => {
                table::op_set_table_cell_borders(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::SetTableCellInset(op) => {
                table::op_set_table_cell_inset(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::SetTableCellPr(op) => {
                table::op_set_table_cell_pr(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::SetTableStyleRef(op) => {
                table::op_set_table_style_ref(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::SetTablePos(op) => {
                table::op_set_table_pos(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::SetTableSize(op) => {
                table::op_set_table_size(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::SetTableColWidths(op) => {
                table::op_set_table_col_widths(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::SetTableTrHeight(op) => {
                table::op_set_table_tr_height(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::InsertSlide(op) => {
                slides::op_insert_slide(
                    &mut pkg,
                    &pres_part,
                    &mut pres,
                    &mut slides,
                    &frozen,
                    op,
                    i,
                )?;
            }
            PptxOp::DeleteSlide(op) => {
                slides::op_delete_slide(
                    &mut pkg,
                    &pres_part,
                    &mut pres,
                    &mut slides,
                    &frozen,
                    op,
                    i,
                )?;
            }
            PptxOp::MoveSlide(op) => {
                slides::op_move_slide(
                    &mut pkg,
                    &pres_part,
                    &mut pres,
                    &mut slides,
                    &frozen,
                    op,
                    i,
                )?;
            }
            PptxOp::SetSlideHidden(op) => {
                slides::op_set_slide_hidden(
                    &mut pkg,
                    &pres_part,
                    &pres,
                    &mut slides,
                    &frozen,
                    op,
                    i,
                )?;
            }
            PptxOp::SetTransition(op) => {
                transition::apply(
                    &mut pkg,
                    &slides,
                    &pres_part,
                    &pres.root,
                    transition::SetArgs {
                        slide: op.slide,
                        master: op.master,
                        type_name: op.type_name.as_deref(),
                        dur: op.dur,
                        adv_click: op.adv_click,
                        adv_tm: op.adv_tm,
                        dir: op.dir.as_deref(),
                        inv_x: op.inv_x,
                        inv_y: op.inv_y,
                        sound: op.sound.as_deref(),
                        orient: op.orient.as_deref(),
                        spokes: op.spokes,
                    },
                    i,
                )?;
            }
            PptxOp::InsertAnim(op) => {
                timing::insert(
                    &mut pkg,
                    &slides,
                    &pres_part,
                    &pres.root,
                    &op.id,
                    &op.target,
                    timing::AnimWrite {
                        type_name: &op.type_name,
                        trigger: &op.trigger,
                        delay: op.delay,
                        dur: op.dur,
                        repeat: op.repeat,
                        sound: op.sound.as_deref(),
                        para: op.para,
                        click_on: op.click_on.as_deref(),
                    },
                    i,
                )?;
            }
            PptxOp::SetAnim(op) => {
                timing::set(
                    &mut pkg,
                    &slides,
                    &pres_part,
                    &pres.root,
                    &op.id,
                    &op.target,
                    op.index,
                    op.type_name.as_deref(),
                    op.trigger.as_deref(),
                    op.delay,
                    op.dur,
                    op.repeat,
                    op.sound.as_ref().map(|inner| inner.as_deref()),
                    op.para,
                    op.click_on.as_deref(),
                    i,
                )?;
            }
            PptxOp::DeleteAnim(op) => {
                timing::delete(
                    &mut pkg, &slides, &pres_part, &pres.root, &op.id, &op.target, op.index, i,
                )?;
            }
            PptxOp::MoveAnim(op) => {
                timing::move_anim(
                    &mut pkg, &slides, &pres_part, &pres.root, &op.id, &op.target, op.index,
                    op.dest, i,
                )?;
            }
            PptxOp::SetSlideLayout(op) => {
                master::set_slide_layout(
                    &mut pkg, &slides, &pres_part, &pres.root, op.slide, op.master, op.layout, i,
                )?;
            }
            PptxOp::InsertLayout(op) => {
                master::insert_layout(&mut pkg, &pres_part, &pres.root, op.master, &op.name, i)?;
            }
            PptxOp::DeleteLayout(op) => {
                master::delete_layout(
                    &mut pkg, &slides, &pres_part, &pres.root, op.master, op.layout, i,
                )?;
            }
            PptxOp::InsertPlaceholder(op) => {
                master::op_insert_placeholder(&mut pkg, &pres_part, &mut pres, op, i)?;
            }
            PptxOp::DeletePlaceholder(op) => {
                master::delete_placeholder(
                    &mut pkg,
                    &pres_part,
                    &pres.root,
                    &op.placeholder,
                    &op.id,
                    i,
                )?;
            }
            PptxOp::InsertMaster(op) => {
                master::insert_master(&mut pkg, &pres_part, &mut pres, op.from, i)?;
            }
            PptxOp::DeleteMaster(op) => {
                master::delete_master(&mut pkg, &slides, &pres_part, &mut pres, op.master, i)?;
            }
            PptxOp::SetSlideSize(op) => {
                theme::op_set_slide_size(&mut pkg, &pres_part, &mut pres, op, i)?;
            }
            PptxOp::SetNotesSize(op) => {
                theme::op_set_notes_size(&mut pkg, &pres_part, &mut pres, op, i)?;
            }
            PptxOp::SetThemeColors(op) => {
                theme::set_theme_colors(
                    &mut pkg,
                    &pres_part,
                    &pres.root,
                    op.host,
                    &op.slots,
                    op.name.as_ref().map(|n| n.as_deref()),
                    op.theme_name.as_ref().map(|n| n.as_deref()),
                    i,
                )?;
            }
            PptxOp::SetThemeFonts(op) => {
                theme::set_theme_fonts(
                    &mut pkg,
                    &pres_part,
                    &pres.root,
                    op.host,
                    &op.fonts,
                    op.name.as_ref().map(|n| n.as_deref()),
                    op.major_scripts.as_deref(),
                    op.minor_scripts.as_deref(),
                    i,
                )?;
            }
            PptxOp::SetThemeFmt(op) => {
                theme::set_theme_fmt(
                    &mut pkg,
                    &pres_part,
                    &pres.root,
                    op.host,
                    op.line,
                    op.fill,
                    op.bg_fill,
                    op.name.as_ref().map(|n| n.as_deref()),
                    i,
                )?;
            }
            PptxOp::SetThemeExtraSchemes(op) => {
                theme::set_theme_extra_schemes(
                    &mut pkg,
                    &pres_part,
                    &pres.root,
                    op.host,
                    &op.schemes,
                    i,
                )?;
            }
            PptxOp::SetThemeObjectDefaults(op) => {
                theme::set_theme_object_defaults(
                    &mut pkg,
                    &pres_part,
                    &pres.root,
                    op.host,
                    op.defaults.as_ref(),
                    i,
                )?;
            }
            PptxOp::SetClrMap(op) => {
                theme_map::set_clr_map(&mut pkg, &pres_part, &pres.root, op, i)?;
            }
            PptxOp::SetClrMapOvr(op) => {
                theme_map::set_clr_map_ovr(&mut pkg, &pres_part, &pres.root, &slides, op, i)?;
            }
            PptxOp::SetThemeOverride(op) => {
                theme_map::set_theme_override(&mut pkg, &slides, op, i)?;
            }
            PptxOp::SetHandoutText(op) => {
                handout::set_text(&mut pkg, &pres_part, &mut pres, op, i)?;
            }
            PptxOp::InsertHyperlink(op) => {
                hyperlink::op_insert_hyperlink(&mut pkg, &pres_part, &mut pres, &slides, op, i)?;
            }
            PptxOp::SetHyperlink(op) => {
                hyperlink::op_set_hyperlink(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::DeleteHyperlink(op) => {
                hyperlink::op_delete_hyperlink(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::InsertField(op) => {
                field::op_insert_field(&mut pkg, &pres_part, &mut pres, &slides, op, i)?;
            }
            PptxOp::SetField(op) => {
                field::op_set_field(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::DeleteField(op) => {
                field::op_delete_field(&mut pkg, &pres_part, &pres, &slides, op, i)?;
            }
            PptxOp::SetProps(op) => {
                crate::props::apply_set_props(&mut pkg, op, i)?;
            }
            PptxOp::SetCustomProp(op) => {
                crate::props::apply_set_custom_prop(&mut pkg, op, i)?;
            }
            PptxOp::DeleteCustomProp(op) => {
                crate::props::apply_delete_custom_prop(&mut pkg, op, i)?;
            }
        }
    }

    Ok(pkg)
}

pub(crate) fn open_shape_host(
    pkg: &Package,
    pres_part: &str,
    pres_root: &xml::Element,
    slides: &[shape::SlideMeta],
    target: &shape::ShapeTarget,
    id: &str,
    i: usize,
) -> Result<xml::Document, Error> {
    let part = host_xml_part(
        pkg,
        pres_part,
        pres_root,
        slides,
        target.slide,
        target.on_notes,
        target.master,
        ErrorKind::ShapeNotFound { id: id.to_string() },
        format!("ops[{i}].id"),
    )?;
    xml::parse(pkg.get(&part)?)
}

pub(crate) fn write_shape_host(
    pkg: &mut Package,
    pres_part: &str,
    pres_root: &xml::Element,
    slides: &[shape::SlideMeta],
    target: &shape::ShapeTarget,
    doc: xml::Document,
) -> Result<(), Error> {
    let part = host_xml_part(
        pkg,
        pres_part,
        pres_root,
        slides,
        target.slide,
        target.on_notes,
        target.master,
        ErrorKind::ShapeNotFound {
            id: format!("slide:{}/notes", target.slide),
        },
        "id".into(),
    )?;
    super::list_pin::write_part(pkg, &part, &doc)?;
    Ok(())
}

pub(crate) fn comment_slide_meta<'a>(
    slides: &'a [shape::SlideMeta],
    id: &str,
    i: usize,
) -> Result<&'a shape::SlideMeta, Error> {
    let target = comments::parse_comment_id(id, &format!("ops[{i}].id"))?;
    shape::slide_at_start(slides, target.slide).ok_or_else(|| {
        Error::with_path(
            ErrorKind::SlideNotFound {
                n: target.slide as u32,
            },
            format!("ops[{i}].id"),
        )
    })
}

/// 只认 `sldIdLst` 当前顺序的模块（节、自定义放映）用：apply 开始时的页号换成当前位置。
pub(crate) fn slide_now(
    slides: &[shape::SlideMeta],
    n: usize,
    json_path: String,
) -> Result<usize, Error> {
    shape::pos_at_start(slides, n)
        .map(|pos| pos + 1)
        .ok_or_else(|| Error::with_path(ErrorKind::SlideNotFound { n: n as u32 }, json_path))
}

pub(crate) fn slides_now(
    slides: &[shape::SlideMeta],
    list: &[usize],
    i: usize,
) -> Result<Vec<usize>, Error> {
    list.iter()
        .enumerate()
        .map(|(j, &n)| slide_now(slides, n, format!("ops[{i}].slides[{j}]")))
        .collect()
}

pub(crate) fn notes_or_slide_part(
    pkg: &Package,
    pres_part: &str,
    pres_root: &xml::Element,
    slides: &[shape::SlideMeta],
    slide: usize,
    on_notes: bool,
    master: Option<(usize, Option<usize>)>,
    missing: ErrorKind,
    json_path: String,
) -> Result<String, Error> {
    host_xml_part(
        pkg, pres_part, pres_root, slides, slide, on_notes, master, missing, json_path,
    )
}

pub(crate) fn host_xml_part(
    pkg: &Package,
    pres_part: &str,
    pres_root: &xml::Element,
    slides: &[shape::SlideMeta],
    slide: usize,
    on_notes: bool,
    master: Option<(usize, Option<usize>)>,
    missing: ErrorKind,
    json_path: String,
) -> Result<String, Error> {
    picture::resolve_tree_part(
        pkg,
        pres_part,
        pres_root,
        slides,
        &picture::TreeHost::from_parts(slide, on_notes, master),
        missing,
        &json_path,
    )
}

pub(crate) fn ensure_notes_or_slide(
    pkg: &mut Package,
    pres_part: &str,
    pres: &mut xml::Document,
    slides: &[shape::SlideMeta],
    slide: usize,
    on_notes: bool,
    master: Option<(usize, Option<usize>)>,
    json_path: String,
) -> Result<String, Error> {
    ensure_host_part(
        pkg, pres_part, pres, slides, slide, on_notes, master, json_path,
    )
}

pub(crate) fn ensure_host_part(
    pkg: &mut Package,
    pres_part: &str,
    pres: &mut xml::Document,
    slides: &[shape::SlideMeta],
    slide: usize,
    on_notes: bool,
    master: Option<(usize, Option<usize>)>,
    json_path: String,
) -> Result<String, Error> {
    if master.is_some() {
        return host_xml_part(
            pkg,
            pres_part,
            &pres.root,
            slides,
            slide,
            on_notes,
            master,
            ErrorKind::MasterNotFound { n: 0 },
            json_path,
        );
    }
    let meta = shape::slide_at_start(slides, slide)
        .ok_or_else(|| Error::with_path(ErrorKind::SlideNotFound { n: slide as u32 }, json_path))?;
    if on_notes {
        notes::ensure_notes_slide(pkg, pres_part, pres, &meta.part)
    } else {
        Ok(meta.part.clone())
    }
}
