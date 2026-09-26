//! 按形状 id 替换可见文本，并处理幻灯片图。

use crate::error::{Error, ErrorKind};
use crate::opc::{self, Package};
use crate::xml;

use super::picture;
use super::request::PptxOp;
use super::shape;

pub fn apply(bytes: &[u8], ops: &[PptxOp]) -> Result<Vec<u8>, Error> {
    let mut pkg = Package::open_bytes(bytes.to_vec())?;
    let pres_part = opc::office_document_part(&pkg)?;
    let pres = xml::parse(pkg.get(&pres_part)?)?;
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
                let mut doc = xml::parse(pkg.get(&meta.part)?)?;
                let sp = shape::find_shape_mut(&mut doc.root, &target.key).ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::ShapeNotFound { id: op.id.clone() },
                        format!("ops[{i}].id"),
                    )
                })?;
                let tx = sp.child_by_local_mut("txBody").ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::ShapeHasNoText { id: op.id.clone() },
                        format!("ops[{i}].id"),
                    )
                })?;
                shape::set_txbody_text(tx, &op.text);
                pkg.set(&meta.part, doc.to_bytes()?)?;
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
                let doc = xml::parse(pkg.get(&meta.part)?)?;
                picture::replace_picture(&mut pkg, &meta.part, &doc.root, op, i)?;
            }
            PptxOp::InsertPicture(op) => {
                let meta = slides.get(op.slide - 1).ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::SlideNotFound { n: op.slide as u32 },
                        format!("ops[{i}].id"),
                    )
                })?;
                let mut doc = xml::parse(pkg.get(&meta.part)?)?;
                picture::insert_picture(
                    &mut pkg,
                    &mut doc,
                    &meta.part,
                    &pres.root,
                    op,
                    &format!("ops[{i}].file"),
                )?;
                pkg.set(&meta.part, doc.to_bytes()?)?;
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
                let mut doc = xml::parse(pkg.get(&meta.part)?)?;
                picture::delete_picture(&mut pkg, &mut doc, &meta.part, op, i)?;
                pkg.set(&meta.part, doc.to_bytes()?)?;
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
                let mut doc = xml::parse(pkg.get(&meta.part)?)?;
                picture::set_picture_size(&mut doc, op, i)?;
                pkg.set(&meta.part, doc.to_bytes()?)?;
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
                let mut doc = xml::parse(pkg.get(&meta.part)?)?;
                picture::set_picture_pos(&mut doc, op, i)?;
                pkg.set(&meta.part, doc.to_bytes()?)?;
            }
        }
    }

    pkg.save_to_vec()
}
