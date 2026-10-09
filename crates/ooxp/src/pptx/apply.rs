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
use super::master;
use super::media;
use super::notes;
use super::ole;
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
use super::timing;
use super::transition;
use super::zoom;

pub fn apply(bytes: &[u8], ops: &[PptxOp]) -> Result<Vec<u8>, Error> {
    let mut pkg = Package::open_bytes(bytes.to_vec())?;
    let pres_part = opc::office_document_part(&pkg)?;
    let mut pres = xml::parse(pkg.get(&pres_part)?)?;
    let mut slides = shape::list_slides(&pkg, &pres_part, &pres.root)?;

    for (i, op) in ops.iter().enumerate() {
        match op {
            PptxOp::SetShapeText(op) => {
                if op.id.starts_with("master:") {
                    let ph = master::parse_placeholder_id(&op.id, i)?;
                    master::set_text(&mut pkg, &pres_part, &pres.root, &ph, &op.id, &op.text, i)?;
                    continue;
                }
                let target = shape::parse_shape_id(&op.id, i)?;
                let meta = slides.get(target.slide - 1).ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::SlideNotFound {
                            n: target.slide as u32,
                        },
                        format!("ops[{i}].id"),
                    )
                })?;
                let part = if target.on_notes {
                    notes::notes_slide_part(&pkg, &meta.part)?.ok_or_else(|| {
                        Error::with_path(
                            ErrorKind::ShapeNotFound { id: op.id.clone() },
                            format!("ops[{i}].id"),
                        )
                    })?
                } else {
                    meta.part.clone()
                };
                let mut doc = xml::parse(pkg.get(&part)?)?;
                if target.on_notes {
                    if let Some(sp) = notes::find_any_sp_by_key(&doc.root, &target.key) {
                        notes::reject_slide_image(sp, i)?;
                    }
                }
                let sp = shape::find_shape_mut(&mut doc.root, &target.key).ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::ShapeNotFound { id: op.id.clone() },
                        format!("ops[{i}].id"),
                    )
                })?;
                if target.on_notes {
                    notes::reject_slide_image(sp, i)?;
                }
                let tx = sp.child_by_local_mut("txBody").ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::ShapeHasNoText { id: op.id.clone() },
                        format!("ops[{i}].id"),
                    )
                })?;
                shape::set_txbody_text(tx, &op.text);
                pkg.set(&part, doc.to_bytes()?)?;
            }
            PptxOp::InsertShape(op) => {
                let meta = slides.get(op.slide - 1).ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::SlideNotFound { n: op.slide as u32 },
                        format!("ops[{i}].id"),
                    )
                })?;
                let part = if op.on_notes {
                    notes::ensure_notes_slide(&mut pkg, &pres_part, &mut pres, &meta.part)?
                } else {
                    meta.part.clone()
                };
                let mut doc = xml::parse(pkg.get(&part)?)?;
                let canvas = if op.on_notes {
                    notes::notes_size(&pres.root)
                } else {
                    picture::canvas_size(&pres.root)
                };
                shape::insert_shape(&mut doc, canvas, op)?;
                pkg.set(&part, doc.to_bytes()?)?;
            }
            PptxOp::InsertConnector(op) => {
                let meta = slides.get(op.slide - 1).ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::SlideNotFound { n: op.slide as u32 },
                        format!("ops[{i}].id"),
                    )
                })?;
                let part = meta.part.clone();
                let mut doc = xml::parse(pkg.get(&part)?)?;
                connector::insert(&mut doc, picture::canvas_size(&pres.root), op)?;
                pkg.set(&part, doc.to_bytes()?)?;
            }
            PptxOp::DeleteConnector(op) => {
                let meta = slides.get(op.target.slide - 1).ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::SlideNotFound {
                            n: op.target.slide as u32,
                        },
                        format!("ops[{i}].id"),
                    )
                })?;
                let part = meta.part.clone();
                let mut doc = xml::parse(pkg.get(&part)?)?;
                let Some(spid) = connector::try_remove(&mut doc.root, &op.target.key) else {
                    return Err(Error::with_path(
                        ErrorKind::ConnectorNotFound { id: op.id.clone() },
                        format!("ops[{i}].id"),
                    ));
                };
                if let Some(id) = spid {
                    shape::strip_timing_for_spid(&mut doc.root, &id);
                }
                pkg.set(&part, doc.to_bytes()?)?;
            }
            PptxOp::GroupShapes(op) => {
                let meta = slides.get(op.slide - 1).ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::SlideNotFound { n: op.slide as u32 },
                        format!("ops[{i}].id"),
                    )
                })?;
                let part = meta.part.clone();
                let mut doc = xml::parse(pkg.get(&part)?)?;
                group::group(&mut doc, op, i)?;
                pkg.set(&part, doc.to_bytes()?)?;
            }
            PptxOp::Ungroup(op) => {
                let slide = match &op.member {
                    group::MemberId::Shape(t) => t.slide,
                    group::MemberId::Cxn(t) => t.slide,
                };
                let meta = slides.get(slide - 1).ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::SlideNotFound { n: slide as u32 },
                        format!("ops[{i}].id"),
                    )
                })?;
                let part = meta.part.clone();
                let mut doc = xml::parse(pkg.get(&part)?)?;
                group::ungroup(&mut doc, op, i)?;
                pkg.set(&part, doc.to_bytes()?)?;
            }
            PptxOp::DeleteShape(op) => {
                let mut doc = open_shape_host(&pkg, &slides, &op.target, &op.id, i)?;
                if op.target.on_notes {
                    reject_notes_image(&doc.root, &op.target.key, i)?;
                }
                let Some(spid) = shape::try_remove_shape(&mut doc.root, &op.target.key) else {
                    return Err(Error::with_path(
                        ErrorKind::ShapeNotFound { id: op.id.clone() },
                        format!("ops[{i}].id"),
                    ));
                };
                if let Some(id) = spid {
                    shape::strip_timing_for_spid(&mut doc.root, &id);
                }
                write_shape_host(&mut pkg, &slides, &op.target, doc)?;
            }
            PptxOp::SetShapePos(op) => {
                if let Some(ph) = &op.placeholder {
                    master::set_pos(&mut pkg, &pres_part, &pres.root, ph, &op.id, op.x, op.y, i)?;
                    continue;
                }
                if let Some(cxn) = &op.cxn {
                    let slide = cxn.slide;
                    let meta = slides.get(slide - 1).ok_or_else(|| {
                        Error::with_path(
                            ErrorKind::SlideNotFound { n: slide as u32 },
                            format!("ops[{i}].id"),
                        )
                    })?;
                    let part = meta.part.clone();
                    let mut doc = xml::parse(pkg.get(&part)?)?;
                    connector::set_pos(&mut doc, op, i)?;
                    pkg.set(&part, doc.to_bytes()?)?;
                    continue;
                }
                let target = op.target.as_ref().expect("幻灯片形状");
                let mut doc = open_shape_host(&pkg, &slides, target, &op.id, i)?;
                if target.on_notes {
                    reject_notes_image(&doc.root, &target.key, i)?;
                }
                shape::set_shape_pos(&mut doc, op, i)?;
                write_shape_host(&mut pkg, &slides, target, doc)?;
            }
            PptxOp::SetShapeSize(op) => {
                if let Some(ph) = &op.placeholder {
                    master::set_size(
                        &mut pkg, &pres_part, &pres.root, ph, &op.id, op.cx, op.cy, i,
                    )?;
                    continue;
                }
                if let Some(cxn) = &op.cxn {
                    let slide = cxn.slide;
                    let meta = slides.get(slide - 1).ok_or_else(|| {
                        Error::with_path(
                            ErrorKind::SlideNotFound { n: slide as u32 },
                            format!("ops[{i}].id"),
                        )
                    })?;
                    let part = meta.part.clone();
                    let mut doc = xml::parse(pkg.get(&part)?)?;
                    connector::set_size(&mut doc, op, i)?;
                    pkg.set(&part, doc.to_bytes()?)?;
                    continue;
                }
                let target = op.target.as_ref().expect("幻灯片形状");
                let mut doc = open_shape_host(&pkg, &slides, target, &op.id, i)?;
                if target.on_notes {
                    reject_notes_image(&doc.root, &target.key, i)?;
                }
                shape::set_shape_size(&mut doc, op, i)?;
                write_shape_host(&mut pkg, &slides, target, doc)?;
            }
            PptxOp::SetShapeFill(op) => {
                let mut doc = open_shape_host(&pkg, &slides, &op.target, &op.id, i)?;
                if op.target.on_notes {
                    reject_notes_image(&doc.root, &op.target.key, i)?;
                }
                shape::set_shape_fill(&mut doc, op, i)?;
                write_shape_host(&mut pkg, &slides, &op.target, doc)?;
            }
            PptxOp::SetShapeLine(op) => {
                if let Some(cxn) = &op.cxn {
                    let slide = cxn.slide;
                    let meta = slides.get(slide - 1).ok_or_else(|| {
                        Error::with_path(
                            ErrorKind::SlideNotFound { n: slide as u32 },
                            format!("ops[{i}].id"),
                        )
                    })?;
                    let part = meta.part.clone();
                    let mut doc = xml::parse(pkg.get(&part)?)?;
                    connector::set_line(&mut doc, op, i)?;
                    pkg.set(&part, doc.to_bytes()?)?;
                    continue;
                }
                let target = op.target.as_ref().expect("形状");
                let mut doc = open_shape_host(&pkg, &slides, target, &op.id, i)?;
                if target.on_notes {
                    reject_notes_image(&doc.root, &target.key, i)?;
                }
                shape::set_shape_line(&mut doc, op, i)?;
                write_shape_host(&mut pkg, &slides, target, doc)?;
            }
            PptxOp::SetNotesText(op) => {
                let meta = slides.get(op.slide - 1).ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::SlideNotFound { n: op.slide as u32 },
                        format!("ops[{i}].id"),
                    )
                })?;
                notes::apply_set_notes_text(&mut pkg, &pres_part, &mut pres, &meta.part, op, i)?;
            }
            PptxOp::InsertComment(op) => {
                let meta = slides.get(op.slide - 1).ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::SlideNotFound { n: op.slide as u32 },
                        format!("ops[{i}].id"),
                    )
                })?;
                comments::apply_insert(&mut pkg, &pres_part, &pres, meta, op, i)?;
            }
            PptxOp::ReplyComment(op) => {
                let meta = comment_slide_meta(&slides, &op.id, i)?;
                comments::apply_reply(&mut pkg, &pres_part, meta, op, i)?;
            }
            PptxOp::DeleteComment(op) => {
                let meta = comment_slide_meta(&slides, &op.id, i)?;
                comments::apply_delete(&mut pkg, meta, op, i)?;
            }
            PptxOp::SetCommentText(op) => {
                let meta = comment_slide_meta(&slides, &op.id, i)?;
                comments::apply_set_text(&mut pkg, meta, op, i)?;
            }
            PptxOp::SetCommentMeta(op) => {
                let meta = comment_slide_meta(&slides, &op.id, i)?;
                comments::apply_set_meta(&mut pkg, &pres_part, meta, op, i)?;
            }
            PptxOp::SetCommentDone(op) => {
                let meta = comment_slide_meta(&slides, &op.id, i)?;
                comments::apply_set_done(&mut pkg, meta, op, i)?;
            }
            PptxOp::ReplacePicture(op) => {
                let meta = slides.get(op.target.slide - 1).ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::SlideNotFound {
                            n: op.target.slide as u32,
                        },
                        format!("ops[{i}].id"),
                    )
                })?;
                if op.target.comment.is_some() {
                    comments::replace_picture(&mut pkg, meta, op, i)?;
                    continue;
                }
                let part = pic_host_part(
                    &pkg,
                    meta,
                    op.target.on_notes,
                    &op.id,
                    format!("ops[{i}].id"),
                )?;
                let mut doc = xml::parse(pkg.get(&part)?)?;
                let dirty = picture::replace_picture(&mut pkg, &part, &mut doc.root, op, i)?;
                if dirty {
                    pkg.set(&part, doc.to_bytes()?)?;
                }
            }
            PptxOp::InsertPicture(op) => {
                let meta = slides.get(op.slide - 1).ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::SlideNotFound { n: op.slide as u32 },
                        format!("ops[{i}].id"),
                    )
                })?;
                if op.comment.is_some() {
                    comments::insert_picture(&mut pkg, meta, op, i)?;
                    continue;
                }
                let part = if op.on_notes {
                    notes::ensure_notes_slide(&mut pkg, &pres_part, &mut pres, &meta.part)?
                } else {
                    meta.part.clone()
                };
                let mut doc = xml::parse(pkg.get(&part)?)?;
                picture::insert_picture(
                    &mut pkg,
                    &mut doc,
                    &part,
                    &pres.root,
                    op,
                    &format!("ops[{i}].file"),
                )?;
                pkg.set(&part, doc.to_bytes()?)?;
            }
            PptxOp::DeletePicture(op) => {
                let meta = slides.get(op.target.slide - 1).ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::SlideNotFound {
                            n: op.target.slide as u32,
                        },
                        format!("ops[{i}].id"),
                    )
                })?;
                if op.target.comment.is_some() {
                    comments::delete_picture(&mut pkg, meta, op, i)?;
                    continue;
                }
                let part = pic_host_part(
                    &pkg,
                    meta,
                    op.target.on_notes,
                    &op.id,
                    format!("ops[{i}].id"),
                )?;
                let mut doc = xml::parse(pkg.get(&part)?)?;
                picture::delete_picture(&mut pkg, &mut doc, &part, op, i)?;
                pkg.set(&part, doc.to_bytes()?)?;
            }
            PptxOp::SetPictureSize(op) => {
                let meta = slides.get(op.target.slide - 1).ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::SlideNotFound {
                            n: op.target.slide as u32,
                        },
                        format!("ops[{i}].id"),
                    )
                })?;
                let part = pic_host_part(
                    &pkg,
                    meta,
                    op.target.on_notes,
                    &op.id,
                    format!("ops[{i}].id"),
                )?;
                let mut doc = xml::parse(pkg.get(&part)?)?;
                picture::set_picture_size(&mut doc, op, i)?;
                pkg.set(&part, doc.to_bytes()?)?;
            }
            PptxOp::SetPicturePos(op) => {
                let meta = slides.get(op.target.slide - 1).ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::SlideNotFound {
                            n: op.target.slide as u32,
                        },
                        format!("ops[{i}].id"),
                    )
                })?;
                let part = pic_host_part(
                    &pkg,
                    meta,
                    op.target.on_notes,
                    &op.id,
                    format!("ops[{i}].id"),
                )?;
                let mut doc = xml::parse(pkg.get(&part)?)?;
                picture::set_picture_pos(&mut doc, op, i)?;
                pkg.set(&part, doc.to_bytes()?)?;
            }
            PptxOp::InsertVideo(op) => {
                let meta = slides.get(op.slide - 1).ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::SlideNotFound { n: op.slide as u32 },
                        format!("ops[{i}].id"),
                    )
                })?;
                let part = meta.part.clone();
                let mut doc = xml::parse(pkg.get(&part)?)?;
                media::insert(
                    &mut pkg,
                    &mut doc,
                    &part,
                    &pres.root,
                    op,
                    &format!("ops[{i}].file"),
                    media::MediaKind::Video,
                )?;
                pkg.set(&part, doc.to_bytes()?)?;
            }
            PptxOp::InsertAudio(op) => {
                let meta = slides.get(op.slide - 1).ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::SlideNotFound { n: op.slide as u32 },
                        format!("ops[{i}].id"),
                    )
                })?;
                let part = meta.part.clone();
                let mut doc = xml::parse(pkg.get(&part)?)?;
                media::insert(
                    &mut pkg,
                    &mut doc,
                    &part,
                    &pres.root,
                    op,
                    &format!("ops[{i}].file"),
                    media::MediaKind::Audio,
                )?;
                pkg.set(&part, doc.to_bytes()?)?;
            }
            PptxOp::DeleteMedia(op) => {
                let meta = slides.get(op.target.slide - 1).ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::SlideNotFound {
                            n: op.target.slide as u32,
                        },
                        format!("ops[{i}].id"),
                    )
                })?;
                let part = meta.part.clone();
                let mut doc = xml::parse(pkg.get(&part)?)?;
                media::delete(&mut pkg, &mut doc, &part, op, i)?;
                pkg.set(&part, doc.to_bytes()?)?;
            }
            PptxOp::InsertSlideSection(op) => {
                section::insert(&mut pkg, &pres_part, &mut pres, op, i)?;
            }
            PptxOp::SetSlideSectionName(op) => {
                section::set_name(&mut pkg, &pres_part, &mut pres, op, i)?;
            }
            PptxOp::DeleteSlideSection(op) => {
                section::delete(&mut pkg, &pres_part, &mut pres, op, i)?;
            }
            PptxOp::SetSlideSection(op) => {
                section::set_slide(&mut pkg, &pres_part, &mut pres, op, i)?;
                slides = shape::list_slides(&pkg, &pres_part, &pres.root)?;
            }
            PptxOp::InsertCustomShow(op) => {
                custom_show::insert(&mut pkg, &pres_part, &mut pres, op, i)?;
            }
            PptxOp::SetCustomShow(op) => {
                custom_show::set(&mut pkg, &pres_part, &mut pres, op, i)?;
            }
            PptxOp::DeleteCustomShow(op) => {
                custom_show::delete(&mut pkg, &pres_part, &mut pres, op, i)?;
            }
            PptxOp::InsertOle(op) => {
                let meta = slides.get(op.slide - 1).ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::SlideNotFound { n: op.slide as u32 },
                        format!("ops[{i}].id"),
                    )
                })?;
                let part = meta.part.clone();
                let mut doc = xml::parse(pkg.get(&part)?)?;
                ole::insert(&mut pkg, &mut doc, &part, &pres.root, op, i)?;
                pkg.set(&part, doc.to_bytes()?)?;
            }
            PptxOp::DeleteOle(op) => {
                let meta = slides.get(op.target.slide - 1).ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::SlideNotFound {
                            n: op.target.slide as u32,
                        },
                        format!("ops[{i}].id"),
                    )
                })?;
                let part = meta.part.clone();
                let mut doc = xml::parse(pkg.get(&part)?)?;
                ole::delete(&mut pkg, &mut doc, &part, op, i)?;
                pkg.set(&part, doc.to_bytes()?)?;
            }
            PptxOp::InsertZoom(op) => {
                let meta = slides.get(op.slide - 1).ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::SlideNotFound { n: op.slide as u32 },
                        format!("ops[{i}].id"),
                    )
                })?;
                let part = meta.part.clone();
                let mut doc = xml::parse(pkg.get(&part)?)?;
                zoom::insert(&mut pkg, &mut doc, &part, &pres.root, &slides, op, i)?;
                pkg.set(&part, doc.to_bytes()?)?;
            }
            PptxOp::DeleteZoom(op) => {
                let meta = slides.get(op.target.slide - 1).ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::SlideNotFound {
                            n: op.target.slide as u32,
                        },
                        format!("ops[{i}].id"),
                    )
                })?;
                let part = meta.part.clone();
                let mut doc = xml::parse(pkg.get(&part)?)?;
                zoom::delete(&mut pkg, &mut doc, &part, op, i)?;
                pkg.set(&part, doc.to_bytes()?)?;
            }
            PptxOp::SetShowPr(op) => {
                show::apply(&mut pkg, &pres_part, op, i)?;
            }
            PptxOp::SetProtection(op) => {
                protect::apply(&mut pkg, &pres_part, &mut pres, op, i)?;
            }
            PptxOp::InsertChart(op) => {
                let part = ensure_notes_or_slide(
                    &mut pkg,
                    &pres_part,
                    &mut pres,
                    &slides,
                    op.slide,
                    op.on_notes,
                    format!("ops[{i}].id"),
                )?;
                let mut doc = xml::parse(pkg.get(&part)?)?;
                chart::insert_chart(&mut pkg, &mut doc, &part, &pres.root, op, i)?;
                pkg.set(&part, doc.to_bytes()?)?;
            }
            PptxOp::DeleteChart(op) => {
                let part = notes_or_slide_part(
                    &pkg,
                    &slides,
                    op.target.slide,
                    op.target.on_notes,
                    ErrorKind::ChartNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )?;
                let mut doc = xml::parse(pkg.get(&part)?)?;
                chart::delete_chart(&mut pkg, &mut doc, &part, op, i)?;
                pkg.set(&part, doc.to_bytes()?)?;
            }
            PptxOp::SetChartTitle(op) => {
                let part = notes_or_slide_part(
                    &pkg,
                    &slides,
                    op.target.slide,
                    op.target.on_notes,
                    ErrorKind::ChartNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )?;
                let doc = xml::parse(pkg.get(&part)?)?;
                chart::apply_title(&mut pkg, &doc.root, &part, op, i)?;
            }
            PptxOp::SetChartSeries(op) => {
                let part = notes_or_slide_part(
                    &pkg,
                    &slides,
                    op.target.slide,
                    op.target.on_notes,
                    ErrorKind::ChartNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )?;
                let doc = xml::parse(pkg.get(&part)?)?;
                chart::apply_series(&mut pkg, &doc.root, &part, op, i)?;
            }
            PptxOp::AddChartSeries(op) => {
                let part = notes_or_slide_part(
                    &pkg,
                    &slides,
                    op.target.slide,
                    op.target.on_notes,
                    ErrorKind::ChartNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )?;
                let doc = xml::parse(pkg.get(&part)?)?;
                chart::apply_add_series(&mut pkg, &doc.root, &part, op, i)?;
            }
            PptxOp::DeleteChartSeries(op) => {
                let part = notes_or_slide_part(
                    &pkg,
                    &slides,
                    op.target.slide,
                    op.target.on_notes,
                    ErrorKind::ChartNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )?;
                let doc = xml::parse(pkg.get(&part)?)?;
                chart::apply_delete_series(&mut pkg, &doc.root, &part, op, i)?;
            }
            PptxOp::SetChartType(op) => {
                let part = notes_or_slide_part(
                    &pkg,
                    &slides,
                    op.target.slide,
                    op.target.on_notes,
                    ErrorKind::ChartNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )?;
                let doc = xml::parse(pkg.get(&part)?)?;
                chart::apply_type(&mut pkg, &doc.root, &part, op, i)?;
            }
            PptxOp::SetChartAxisTitle(op) => {
                let part = notes_or_slide_part(
                    &pkg,
                    &slides,
                    op.target.slide,
                    op.target.on_notes,
                    ErrorKind::ChartNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )?;
                let doc = xml::parse(pkg.get(&part)?)?;
                chart::apply_axis_title(&mut pkg, &doc.root, &part, op, i)?;
            }
            PptxOp::SetChartLegend(op) => {
                let part = notes_or_slide_part(
                    &pkg,
                    &slides,
                    op.target.slide,
                    op.target.on_notes,
                    ErrorKind::ChartNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )?;
                let doc = xml::parse(pkg.get(&part)?)?;
                chart::apply_legend(&mut pkg, &doc.root, &part, op, i)?;
            }
            PptxOp::SetChartDataLabels(op) => {
                let part = notes_or_slide_part(
                    &pkg,
                    &slides,
                    op.target.slide,
                    op.target.on_notes,
                    ErrorKind::ChartNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )?;
                let doc = xml::parse(pkg.get(&part)?)?;
                chart::apply_data_labels(&mut pkg, &doc.root, &part, op, i)?;
            }
            PptxOp::SetChartSize(op) => {
                let part = notes_or_slide_part(
                    &pkg,
                    &slides,
                    op.target.slide,
                    op.target.on_notes,
                    ErrorKind::ChartNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )?;
                let mut doc = xml::parse(pkg.get(&part)?)?;
                chart::apply_size(&mut doc, op, i)?;
                pkg.set(&part, doc.to_bytes()?)?;
            }
            PptxOp::SetChartPos(op) => {
                let part = notes_or_slide_part(
                    &pkg,
                    &slides,
                    op.target.slide,
                    op.target.on_notes,
                    ErrorKind::ChartNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )?;
                let mut doc = xml::parse(pkg.get(&part)?)?;
                chart::apply_pos(&mut doc, op, i)?;
                pkg.set(&part, doc.to_bytes()?)?;
            }
            PptxOp::InsertTrendline(op) => {
                let part = notes_or_slide_part(
                    &pkg,
                    &slides,
                    op.target.slide,
                    op.target.on_notes,
                    ErrorKind::ChartNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )?;
                let doc = xml::parse(pkg.get(&part)?)?;
                chart::apply_insert_trendline(&mut pkg, &doc.root, &part, op, i)?;
            }
            PptxOp::SetTrendline(op) => {
                let part = notes_or_slide_part(
                    &pkg,
                    &slides,
                    op.target.slide,
                    op.target.on_notes,
                    ErrorKind::ChartNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )?;
                let doc = xml::parse(pkg.get(&part)?)?;
                chart::apply_set_trendline(&mut pkg, &doc.root, &part, op, i)?;
            }
            PptxOp::DeleteTrendline(op) => {
                let part = notes_or_slide_part(
                    &pkg,
                    &slides,
                    op.target.slide,
                    op.target.on_notes,
                    ErrorKind::ChartNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )?;
                let doc = xml::parse(pkg.get(&part)?)?;
                chart::apply_delete_trendline(&mut pkg, &doc.root, &part, op, i)?;
            }
            PptxOp::InsertErrorBars(op) => {
                let part = notes_or_slide_part(
                    &pkg,
                    &slides,
                    op.target.slide,
                    op.target.on_notes,
                    ErrorKind::ChartNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )?;
                let doc = xml::parse(pkg.get(&part)?)?;
                chart::apply_insert_error_bars(&mut pkg, &doc.root, &part, op, i)?;
            }
            PptxOp::SetErrorBars(op) => {
                let part = notes_or_slide_part(
                    &pkg,
                    &slides,
                    op.target.slide,
                    op.target.on_notes,
                    ErrorKind::ChartNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )?;
                let doc = xml::parse(pkg.get(&part)?)?;
                chart::apply_set_error_bars(&mut pkg, &doc.root, &part, op, i)?;
            }
            PptxOp::DeleteErrorBars(op) => {
                let part = notes_or_slide_part(
                    &pkg,
                    &slides,
                    op.target.slide,
                    op.target.on_notes,
                    ErrorKind::ChartNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )?;
                let doc = xml::parse(pkg.get(&part)?)?;
                chart::apply_delete_error_bars(&mut pkg, &doc.root, &part, op, i)?;
            }
            PptxOp::SetChartSeriesAxis(op) => {
                let part = notes_or_slide_part(
                    &pkg,
                    &slides,
                    op.target.slide,
                    op.target.on_notes,
                    ErrorKind::ChartNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )?;
                let doc = xml::parse(pkg.get(&part)?)?;
                chart::apply_series_axis(&mut pkg, &doc.root, &part, op, i)?;
            }
            PptxOp::InsertSmartArt(op) => {
                let part = ensure_notes_or_slide(
                    &mut pkg,
                    &pres_part,
                    &mut pres,
                    &slides,
                    op.slide,
                    op.on_notes,
                    format!("ops[{i}].id"),
                )?;
                let mut doc = xml::parse(pkg.get(&part)?)?;
                smartart::insert_smartart(&mut pkg, &mut doc, &part, &pres.root, op, i)?;
                pkg.set(&part, doc.to_bytes()?)?;
            }
            PptxOp::DeleteSmartArt(op) => {
                let part = notes_or_slide_part(
                    &pkg,
                    &slides,
                    op.target.slide,
                    op.target.on_notes,
                    ErrorKind::SmartArtNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )?;
                let mut doc = xml::parse(pkg.get(&part)?)?;
                smartart::delete_smartart(&mut pkg, &mut doc, &part, op, i)?;
                pkg.set(&part, doc.to_bytes()?)?;
            }
            PptxOp::SetSmartArtText(op) => {
                let part = notes_or_slide_part(
                    &pkg,
                    &slides,
                    op.target.diagram.slide,
                    op.target.diagram.on_notes,
                    ErrorKind::SmartArtNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )?;
                let doc = xml::parse(pkg.get(&part)?)?;
                smartart::apply_text(&mut pkg, &doc.root, &part, op, i)?;
            }
            PptxOp::AddSmartArtPoint(op) => {
                let (slide, on_notes) = match &op.parent {
                    smartart::AddParent::Diagram(t) => (t.slide, t.on_notes),
                    smartart::AddParent::Point(p) => (p.diagram.slide, p.diagram.on_notes),
                };
                let part = notes_or_slide_part(
                    &pkg,
                    &slides,
                    slide,
                    on_notes,
                    ErrorKind::SmartArtNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )?;
                let doc = xml::parse(pkg.get(&part)?)?;
                smartart::apply_add_point(&mut pkg, &doc.root, &part, op, i)?;
            }
            PptxOp::DeleteSmartArtPoint(op) => {
                let part = notes_or_slide_part(
                    &pkg,
                    &slides,
                    op.target.diagram.slide,
                    op.target.diagram.on_notes,
                    ErrorKind::SmartArtNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )?;
                let doc = xml::parse(pkg.get(&part)?)?;
                smartart::apply_delete_point(&mut pkg, &doc.root, &part, op, i)?;
            }
            PptxOp::SetSmartArtSize(op) => {
                let part = notes_or_slide_part(
                    &pkg,
                    &slides,
                    op.target.slide,
                    op.target.on_notes,
                    ErrorKind::SmartArtNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )?;
                let mut doc = xml::parse(pkg.get(&part)?)?;
                smartart::apply_size(&mut doc, op, i)?;
                pkg.set(&part, doc.to_bytes()?)?;
            }
            PptxOp::SetSmartArtPos(op) => {
                let part = notes_or_slide_part(
                    &pkg,
                    &slides,
                    op.target.slide,
                    op.target.on_notes,
                    ErrorKind::SmartArtNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )?;
                let mut doc = xml::parse(pkg.get(&part)?)?;
                smartart::apply_pos(&mut doc, op, i)?;
                pkg.set(&part, doc.to_bytes()?)?;
            }
            PptxOp::SetSmartArtColors(op) => {
                let part = notes_or_slide_part(
                    &pkg,
                    &slides,
                    op.target.slide,
                    op.target.on_notes,
                    ErrorKind::SmartArtNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )?;
                let mut doc = xml::parse(pkg.get(&part)?)?;
                smartart::apply_colors(&mut pkg, &mut doc, &part, op, i)?;
                pkg.set(&part, doc.to_bytes()?)?;
            }
            PptxOp::SetSmartArtStyle(op) => {
                let part = notes_or_slide_part(
                    &pkg,
                    &slides,
                    op.target.slide,
                    op.target.on_notes,
                    ErrorKind::SmartArtNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )?;
                let mut doc = xml::parse(pkg.get(&part)?)?;
                smartart::apply_style(&mut pkg, &mut doc, &part, op, i)?;
                pkg.set(&part, doc.to_bytes()?)?;
            }
            PptxOp::SetSmartArtLayout(op) => {
                let part = notes_or_slide_part(
                    &pkg,
                    &slides,
                    op.target.slide,
                    op.target.on_notes,
                    ErrorKind::SmartArtNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )?;
                let mut doc = xml::parse(pkg.get(&part)?)?;
                smartart::apply_layout(&mut pkg, &mut doc, &part, op, i)?;
                pkg.set(&part, doc.to_bytes()?)?;
            }
            PptxOp::InsertTable(op) => {
                let part = ensure_notes_or_slide(
                    &mut pkg,
                    &pres_part,
                    &mut pres,
                    &slides,
                    op.slide,
                    op.on_notes,
                    format!("ops[{i}].id"),
                )?;
                let mut doc = xml::parse(pkg.get(&part)?)?;
                table::insert_table(&mut doc, &pres.root, op, i)?;
                pkg.set(&part, doc.to_bytes()?)?;
            }
            PptxOp::DeleteTable(op) => {
                let part = notes_or_slide_part(
                    &pkg,
                    &slides,
                    op.target.slide,
                    op.target.on_notes,
                    ErrorKind::TableNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )?;
                let mut doc = xml::parse(pkg.get(&part)?)?;
                table::delete_table(&mut doc, op, i)?;
                pkg.set(&part, doc.to_bytes()?)?;
            }
            PptxOp::InsertRow(op) => {
                let (slide, on_notes) = match &op.target {
                    table::TableOrRow::Table(t) => (t.slide, t.on_notes),
                    table::TableOrRow::Row { table, .. } => (table.slide, table.on_notes),
                };
                let part = notes_or_slide_part(
                    &pkg,
                    &slides,
                    slide,
                    on_notes,
                    ErrorKind::TableNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )?;
                let mut doc = xml::parse(pkg.get(&part)?)?;
                table::insert_row(&mut doc, op, i)?;
                pkg.set(&part, doc.to_bytes()?)?;
            }
            PptxOp::DeleteRow(op) => {
                let part = notes_or_slide_part(
                    &pkg,
                    &slides,
                    op.table.slide,
                    op.table.on_notes,
                    ErrorKind::TableNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )?;
                let mut doc = xml::parse(pkg.get(&part)?)?;
                table::delete_row(&mut doc, op, i)?;
                pkg.set(&part, doc.to_bytes()?)?;
            }
            PptxOp::InsertCol(op) => {
                let part = notes_or_slide_part(
                    &pkg,
                    &slides,
                    op.target.slide,
                    op.target.on_notes,
                    ErrorKind::TableNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )?;
                let mut doc = xml::parse(pkg.get(&part)?)?;
                table::insert_col(&mut doc, op, i)?;
                pkg.set(&part, doc.to_bytes()?)?;
            }
            PptxOp::DeleteCol(op) => {
                let part = notes_or_slide_part(
                    &pkg,
                    &slides,
                    op.target.slide,
                    op.target.on_notes,
                    ErrorKind::TableNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )?;
                let mut doc = xml::parse(pkg.get(&part)?)?;
                table::delete_col(&mut doc, op, i)?;
                pkg.set(&part, doc.to_bytes()?)?;
            }
            PptxOp::SetTableCellText(op) => {
                let part = notes_or_slide_part(
                    &pkg,
                    &slides,
                    op.target.table.slide,
                    op.target.table.on_notes,
                    ErrorKind::TableNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )?;
                let mut doc = xml::parse(pkg.get(&part)?)?;
                table::set_cell_text(&mut doc, op, i)?;
                pkg.set(&part, doc.to_bytes()?)?;
            }
            PptxOp::SetTablePos(op) => {
                let part = notes_or_slide_part(
                    &pkg,
                    &slides,
                    op.target.slide,
                    op.target.on_notes,
                    ErrorKind::TableNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )?;
                let mut doc = xml::parse(pkg.get(&part)?)?;
                table::apply_pos(&mut doc, op, i)?;
                pkg.set(&part, doc.to_bytes()?)?;
            }
            PptxOp::SetTableSize(op) => {
                let part = notes_or_slide_part(
                    &pkg,
                    &slides,
                    op.target.slide,
                    op.target.on_notes,
                    ErrorKind::TableNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )?;
                let mut doc = xml::parse(pkg.get(&part)?)?;
                table::apply_size(&mut doc, op, i)?;
                pkg.set(&part, doc.to_bytes()?)?;
            }
            PptxOp::InsertSlide(op) => {
                slides::insert(&mut pkg, &pres_part, &mut pres, &slides, op, i)?;
                slides = shape::list_slides(&pkg, &pres_part, &pres.root)?;
            }
            PptxOp::DeleteSlide(op) => {
                slides::delete(&mut pkg, &pres_part, &mut pres, &slides, op, i)?;
                slides = shape::list_slides(&pkg, &pres_part, &pres.root)?;
            }
            PptxOp::MoveSlide(op) => {
                slides::move_slide(&pres_part, &mut pres, &mut pkg, &slides, op, i)?;
                slides = shape::list_slides(&pkg, &pres_part, &pres.root)?;
            }
            PptxOp::SetSlideHidden(op) => {
                slides::set_hidden(&mut pkg, &slides, op, i)?;
                slides = shape::list_slides(&pkg, &pres_part, &pres.root)?;
            }
            PptxOp::SetTransition(op) => {
                transition::apply(
                    &mut pkg,
                    &slides,
                    transition::SetArgs {
                        slide: op.slide,
                        type_name: op.type_name.as_deref(),
                        dur: op.dur,
                        adv_click: op.adv_click,
                        adv_tm: op.adv_tm,
                        dir: op.dir.as_deref(),
                        inv_x: op.inv_x,
                        inv_y: op.inv_y,
                    },
                    i,
                )?;
            }
            PptxOp::InsertAnim(op) => {
                timing::insert(&mut pkg, &slides, &op.id, op.slide, &op.trigger, i)?;
            }
            PptxOp::DeleteAnim(op) => {
                timing::delete(&mut pkg, &slides, &op.id, op.slide, op.index, i)?;
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
                master::insert_placeholder(
                    &mut pkg,
                    &pres_part,
                    &pres.root,
                    op.master,
                    op.layout,
                    &op.type_name,
                    &op.text,
                    op.x,
                    op.y,
                    op.cx,
                    op.cy,
                    i,
                )?;
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
                theme::set_slide_size(&mut pres, op.cx, op.cy, op.type_name.as_deref(), i)?;
                pkg.set(&pres_part, pres.to_bytes()?)?;
            }
            PptxOp::SetThemeColors(op) => {
                theme::set_theme_colors(&mut pkg, &pres_part, &pres.root, op.master, &op.slots, i)?;
            }
            PptxOp::SetThemeFonts(op) => {
                theme::set_theme_fonts(&mut pkg, &pres_part, &pres.root, op.master, &op.fonts, i)?;
            }
            PptxOp::SetHandoutText(op) => {
                handout::set_text(&mut pkg, &pres_part, &mut pres, op, i)?;
            }
            PptxOp::InsertHyperlink(op) => {
                let meta = slides.get(op.shape.slide - 1).ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::SlideNotFound {
                            n: op.shape.slide as u32,
                        },
                        format!("ops[{i}].id"),
                    )
                })?;
                let part = meta.part.clone();
                let mut doc = xml::parse(pkg.get(&part)?)?;
                hyperlink::apply_insert(&mut pkg, &mut doc, &part, &slides, op, i)?;
                pkg.set(&part, doc.to_bytes()?)?;
            }
            PptxOp::SetHyperlink(op) => {
                let meta = slides.get(op.slide - 1).ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::SlideNotFound { n: op.slide as u32 },
                        format!("ops[{i}].id"),
                    )
                })?;
                let part = meta.part.clone();
                let mut doc = xml::parse(pkg.get(&part)?)?;
                hyperlink::apply_set(&mut pkg, &mut doc, &part, &slides, op, i)?;
                pkg.set(&part, doc.to_bytes()?)?;
            }
            PptxOp::DeleteHyperlink(op) => {
                let meta = slides.get(op.slide - 1).ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::SlideNotFound { n: op.slide as u32 },
                        format!("ops[{i}].id"),
                    )
                })?;
                let part = meta.part.clone();
                let mut doc = xml::parse(pkg.get(&part)?)?;
                hyperlink::apply_delete(&mut pkg, &mut doc.root, &part, op, i)?;
                pkg.set(&part, doc.to_bytes()?)?;
            }
            PptxOp::InsertField(op) => {
                let meta = slides.get(op.shape.slide - 1).ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::SlideNotFound {
                            n: op.shape.slide as u32,
                        },
                        format!("ops[{i}].id"),
                    )
                })?;
                let part = meta.part.clone();
                let mut doc = xml::parse(pkg.get(&part)?)?;
                field::apply_insert(&mut doc.root, op, i)?;
                pkg.set(&part, doc.to_bytes()?)?;
            }
            PptxOp::SetField(op) => {
                let meta = slides.get(op.slide - 1).ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::SlideNotFound { n: op.slide as u32 },
                        format!("ops[{i}].id"),
                    )
                })?;
                let part = meta.part.clone();
                let mut doc = xml::parse(pkg.get(&part)?)?;
                field::apply_set(&mut doc.root, op, i)?;
                pkg.set(&part, doc.to_bytes()?)?;
            }
            PptxOp::DeleteField(op) => {
                let meta = slides.get(op.slide - 1).ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::SlideNotFound { n: op.slide as u32 },
                        format!("ops[{i}].id"),
                    )
                })?;
                let part = meta.part.clone();
                let mut doc = xml::parse(pkg.get(&part)?)?;
                field::apply_delete(&mut doc.root, op, i)?;
                pkg.set(&part, doc.to_bytes()?)?;
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

    pkg.save_to_vec()
}

fn reject_notes_image(
    root: &crate::xml::Element,
    key: &shape::ShapeKey,
    i: usize,
) -> Result<(), Error> {
    if let Some(sp) = notes::find_any_sp_by_key(root, key) {
        notes::reject_slide_image(sp, i)?;
    }
    Ok(())
}

fn open_shape_host(
    pkg: &Package,
    slides: &[shape::SlideMeta],
    target: &shape::ShapeTarget,
    id: &str,
    i: usize,
) -> Result<xml::Document, Error> {
    let meta = slides.get(target.slide - 1).ok_or_else(|| {
        Error::with_path(
            ErrorKind::SlideNotFound {
                n: target.slide as u32,
            },
            format!("ops[{i}].id"),
        )
    })?;
    let part = if target.on_notes {
        notes::notes_slide_part(pkg, &meta.part)?.ok_or_else(|| {
            Error::with_path(
                ErrorKind::ShapeNotFound { id: id.to_string() },
                format!("ops[{i}].id"),
            )
        })?
    } else {
        meta.part.clone()
    };
    xml::parse(pkg.get(&part)?)
}

fn write_shape_host(
    pkg: &mut Package,
    slides: &[shape::SlideMeta],
    target: &shape::ShapeTarget,
    doc: xml::Document,
) -> Result<(), Error> {
    let meta = slides.get(target.slide - 1).ok_or_else(|| {
        Error::with_path(
            ErrorKind::SlideNotFound {
                n: target.slide as u32,
            },
            "id",
        )
    })?;
    let part = if target.on_notes {
        notes::notes_slide_part(pkg, &meta.part)?.ok_or_else(|| {
            Error::new(ErrorKind::ShapeNotFound {
                id: format!("slide:{}/notes", target.slide),
            })
        })?
    } else {
        meta.part.clone()
    };
    pkg.set(&part, doc.to_bytes()?)?;
    Ok(())
}

fn comment_slide_meta<'a>(
    slides: &'a [shape::SlideMeta],
    id: &str,
    i: usize,
) -> Result<&'a shape::SlideMeta, Error> {
    let target = comments::parse_comment_id(id, &format!("ops[{i}].id"))?;
    slides.get(target.slide - 1).ok_or_else(|| {
        Error::with_path(
            ErrorKind::SlideNotFound {
                n: target.slide as u32,
            },
            format!("ops[{i}].id"),
        )
    })
}

fn notes_or_slide_part(
    pkg: &Package,
    slides: &[shape::SlideMeta],
    slide: usize,
    on_notes: bool,
    missing: ErrorKind,
    json_path: String,
) -> Result<String, Error> {
    let meta = slides.get(slide - 1).ok_or_else(|| {
        Error::with_path(
            ErrorKind::SlideNotFound { n: slide as u32 },
            json_path.clone(),
        )
    })?;
    if on_notes {
        notes::notes_slide_part(pkg, &meta.part)?
            .ok_or_else(|| Error::with_path(missing, json_path))
    } else {
        Ok(meta.part.clone())
    }
}

fn ensure_notes_or_slide(
    pkg: &mut Package,
    pres_part: &str,
    pres: &mut xml::Document,
    slides: &[shape::SlideMeta],
    slide: usize,
    on_notes: bool,
    json_path: String,
) -> Result<String, Error> {
    let meta = slides
        .get(slide - 1)
        .ok_or_else(|| Error::with_path(ErrorKind::SlideNotFound { n: slide as u32 }, json_path))?;
    if on_notes {
        notes::ensure_notes_slide(pkg, pres_part, pres, &meta.part)
    } else {
        Ok(meta.part.clone())
    }
}

fn pic_host_part(
    pkg: &Package,
    meta: &shape::SlideMeta,
    on_notes: bool,
    id: &str,
    json_path: String,
) -> Result<String, Error> {
    if on_notes {
        notes::notes_slide_part(pkg, &meta.part)?.ok_or_else(|| {
            Error::with_path(ErrorKind::PictureNotFound { id: id.to_string() }, json_path)
        })
    } else {
        Ok(meta.part.clone())
    }
}
