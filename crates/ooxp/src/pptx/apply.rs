//! 按形状 id 替换可见文本，并处理幻灯片图与图表。

use crate::error::{Error, ErrorKind};
use crate::opc::{self, Package};
use crate::xml;

use super::chart;
use super::comments;
use super::notes;
use super::picture;
use super::request::PptxOp;
use super::shape;
use super::smartart;

pub fn apply(bytes: &[u8], ops: &[PptxOp]) -> Result<Vec<u8>, Error> {
    let mut pkg = Package::open_bytes(bytes.to_vec())?;
    let pres_part = opc::office_document_part(&pkg)?;
    let mut pres = xml::parse(pkg.get(&pres_part)?)?;
    let slides = shape::list_slides(&pkg, &pres_part, &pres.root)?;

    for (i, op) in ops.iter().enumerate() {
        match op {
            PptxOp::SetShapeText(op) => {
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
                let part = pic_host_part(
                    &pkg,
                    meta,
                    op.target.on_notes,
                    &op.id,
                    format!("ops[{i}].id"),
                )?;
                let doc = xml::parse(pkg.get(&part)?)?;
                picture::replace_picture(&mut pkg, &part, &doc.root, op, i)?;
            }
            PptxOp::InsertPicture(op) => {
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
            PptxOp::InsertChart(op) => {
                let meta = slides.get(op.slide - 1).ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::SlideNotFound { n: op.slide as u32 },
                        format!("ops[{i}].id"),
                    )
                })?;
                let mut doc = xml::parse(pkg.get(&meta.part)?)?;
                chart::insert_chart(&mut pkg, &mut doc, &meta.part, &pres.root, op, i)?;
                pkg.set(&meta.part, doc.to_bytes()?)?;
            }
            PptxOp::DeleteChart(op) => {
                let meta = slides.get(op.target.slide - 1).ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::SlideNotFound {
                            n: op.target.slide as u32,
                        },
                        format!("ops[{i}].id"),
                    )
                })?;
                let mut doc = xml::parse(pkg.get(&meta.part)?)?;
                chart::delete_chart(&mut pkg, &mut doc, &meta.part, op, i)?;
                pkg.set(&meta.part, doc.to_bytes()?)?;
            }
            PptxOp::SetChartTitle(op) => {
                let meta = slides.get(op.target.slide - 1).ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::SlideNotFound {
                            n: op.target.slide as u32,
                        },
                        format!("ops[{i}].id"),
                    )
                })?;
                let doc = xml::parse(pkg.get(&meta.part)?)?;
                chart::apply_title(&mut pkg, &doc.root, &meta.part, op, i)?;
            }
            PptxOp::SetChartSeries(op) => {
                let meta = slides.get(op.target.slide - 1).ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::SlideNotFound {
                            n: op.target.slide as u32,
                        },
                        format!("ops[{i}].id"),
                    )
                })?;
                let doc = xml::parse(pkg.get(&meta.part)?)?;
                chart::apply_series(&mut pkg, &doc.root, &meta.part, op, i)?;
            }
            PptxOp::AddChartSeries(op) => {
                let meta = slides.get(op.target.slide - 1).ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::SlideNotFound {
                            n: op.target.slide as u32,
                        },
                        format!("ops[{i}].id"),
                    )
                })?;
                let doc = xml::parse(pkg.get(&meta.part)?)?;
                chart::apply_add_series(&mut pkg, &doc.root, &meta.part, op, i)?;
            }
            PptxOp::DeleteChartSeries(op) => {
                let meta = slides.get(op.target.slide - 1).ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::SlideNotFound {
                            n: op.target.slide as u32,
                        },
                        format!("ops[{i}].id"),
                    )
                })?;
                let doc = xml::parse(pkg.get(&meta.part)?)?;
                chart::apply_delete_series(&mut pkg, &doc.root, &meta.part, op, i)?;
            }
            PptxOp::SetChartType(op) => {
                let meta = slides.get(op.target.slide - 1).ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::SlideNotFound {
                            n: op.target.slide as u32,
                        },
                        format!("ops[{i}].id"),
                    )
                })?;
                let doc = xml::parse(pkg.get(&meta.part)?)?;
                chart::apply_type(&mut pkg, &doc.root, &meta.part, op, i)?;
            }
            PptxOp::SetChartAxisTitle(op) => {
                let meta = slides.get(op.target.slide - 1).ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::SlideNotFound {
                            n: op.target.slide as u32,
                        },
                        format!("ops[{i}].id"),
                    )
                })?;
                let doc = xml::parse(pkg.get(&meta.part)?)?;
                chart::apply_axis_title(&mut pkg, &doc.root, &meta.part, op, i)?;
            }
            PptxOp::SetChartLegend(op) => {
                let meta = slides.get(op.target.slide - 1).ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::SlideNotFound {
                            n: op.target.slide as u32,
                        },
                        format!("ops[{i}].id"),
                    )
                })?;
                let doc = xml::parse(pkg.get(&meta.part)?)?;
                chart::apply_legend(&mut pkg, &doc.root, &meta.part, op, i)?;
            }
            PptxOp::SetChartDataLabels(op) => {
                let meta = slides.get(op.target.slide - 1).ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::SlideNotFound {
                            n: op.target.slide as u32,
                        },
                        format!("ops[{i}].id"),
                    )
                })?;
                let doc = xml::parse(pkg.get(&meta.part)?)?;
                chart::apply_data_labels(&mut pkg, &doc.root, &meta.part, op, i)?;
            }
            PptxOp::SetChartSize(op) => {
                let meta = slides.get(op.target.slide - 1).ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::SlideNotFound {
                            n: op.target.slide as u32,
                        },
                        format!("ops[{i}].id"),
                    )
                })?;
                let mut doc = xml::parse(pkg.get(&meta.part)?)?;
                chart::apply_size(&mut doc, op, i)?;
                pkg.set(&meta.part, doc.to_bytes()?)?;
            }
            PptxOp::SetChartPos(op) => {
                let meta = slides.get(op.target.slide - 1).ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::SlideNotFound {
                            n: op.target.slide as u32,
                        },
                        format!("ops[{i}].id"),
                    )
                })?;
                let mut doc = xml::parse(pkg.get(&meta.part)?)?;
                chart::apply_pos(&mut doc, op, i)?;
                pkg.set(&meta.part, doc.to_bytes()?)?;
            }
            PptxOp::InsertSmartArt(op) => {
                let meta = slides.get(op.slide - 1).ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::SlideNotFound { n: op.slide as u32 },
                        format!("ops[{i}].id"),
                    )
                })?;
                let mut doc = xml::parse(pkg.get(&meta.part)?)?;
                smartart::insert_smartart(&mut pkg, &mut doc, &meta.part, &pres.root, op, i)?;
                pkg.set(&meta.part, doc.to_bytes()?)?;
            }
            PptxOp::DeleteSmartArt(op) => {
                let meta = slides.get(op.target.slide - 1).ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::SlideNotFound {
                            n: op.target.slide as u32,
                        },
                        format!("ops[{i}].id"),
                    )
                })?;
                let mut doc = xml::parse(pkg.get(&meta.part)?)?;
                smartart::delete_smartart(&mut pkg, &mut doc, &meta.part, op, i)?;
                pkg.set(&meta.part, doc.to_bytes()?)?;
            }
            PptxOp::SetSmartArtText(op) => {
                let meta = slides.get(op.target.diagram.slide - 1).ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::SlideNotFound {
                            n: op.target.diagram.slide as u32,
                        },
                        format!("ops[{i}].id"),
                    )
                })?;
                let doc = xml::parse(pkg.get(&meta.part)?)?;
                smartart::apply_text(&mut pkg, &doc.root, &meta.part, op, i)?;
            }
            PptxOp::AddSmartArtPoint(op) => {
                let slide = match &op.parent {
                    smartart::AddParent::Diagram(t) => t.slide,
                    smartart::AddParent::Point(p) => p.diagram.slide,
                };
                let meta = slides.get(slide - 1).ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::SlideNotFound { n: slide as u32 },
                        format!("ops[{i}].id"),
                    )
                })?;
                let doc = xml::parse(pkg.get(&meta.part)?)?;
                smartart::apply_add_point(&mut pkg, &doc.root, &meta.part, op, i)?;
            }
            PptxOp::DeleteSmartArtPoint(op) => {
                let meta = slides.get(op.target.diagram.slide - 1).ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::SlideNotFound {
                            n: op.target.diagram.slide as u32,
                        },
                        format!("ops[{i}].id"),
                    )
                })?;
                let doc = xml::parse(pkg.get(&meta.part)?)?;
                smartart::apply_delete_point(&mut pkg, &doc.root, &meta.part, op, i)?;
            }
            PptxOp::SetSmartArtSize(op) => {
                let meta = slides.get(op.target.slide - 1).ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::SlideNotFound {
                            n: op.target.slide as u32,
                        },
                        format!("ops[{i}].id"),
                    )
                })?;
                let mut doc = xml::parse(pkg.get(&meta.part)?)?;
                smartart::apply_size(&mut doc, op, i)?;
                pkg.set(&meta.part, doc.to_bytes()?)?;
            }
            PptxOp::SetSmartArtPos(op) => {
                let meta = slides.get(op.target.slide - 1).ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::SlideNotFound {
                            n: op.target.slide as u32,
                        },
                        format!("ops[{i}].id"),
                    )
                })?;
                let mut doc = xml::parse(pkg.get(&meta.part)?)?;
                smartart::apply_pos(&mut doc, op, i)?;
                pkg.set(&meta.part, doc.to_bytes()?)?;
            }
            PptxOp::SetSmartArtColors(op) => {
                let meta = slides.get(op.target.slide - 1).ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::SlideNotFound {
                            n: op.target.slide as u32,
                        },
                        format!("ops[{i}].id"),
                    )
                })?;
                let mut doc = xml::parse(pkg.get(&meta.part)?)?;
                smartart::apply_colors(&mut pkg, &mut doc, &meta.part, op, i)?;
                pkg.set(&meta.part, doc.to_bytes()?)?;
            }
            PptxOp::SetSmartArtStyle(op) => {
                let meta = slides.get(op.target.slide - 1).ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::SlideNotFound {
                            n: op.target.slide as u32,
                        },
                        format!("ops[{i}].id"),
                    )
                })?;
                let mut doc = xml::parse(pkg.get(&meta.part)?)?;
                smartart::apply_style(&mut pkg, &mut doc, &meta.part, op, i)?;
                pkg.set(&meta.part, doc.to_bytes()?)?;
            }
            PptxOp::SetSmartArtLayout(op) => {
                let meta = slides.get(op.target.slide - 1).ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::SlideNotFound {
                            n: op.target.slide as u32,
                        },
                        format!("ops[{i}].id"),
                    )
                })?;
                let mut doc = xml::parse(pkg.get(&meta.part)?)?;
                smartart::apply_layout(&mut pkg, &mut doc, &meta.part, op, i)?;
                pkg.set(&meta.part, doc.to_bytes()?)?;
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
