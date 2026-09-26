//! 按形状 id 替换可见文本。

use crate::error::{Error, ErrorKind};
use crate::opc::{self, Package};
use crate::xml;

use super::request::SetShapeTextOp;
use super::shape;

pub fn apply(bytes: &[u8], ops: &[SetShapeTextOp]) -> Result<Vec<u8>, Error> {
    let mut pkg = Package::open_bytes(bytes.to_vec())?;
    let pres_part = opc::office_document_part(&pkg)?;
    let pres = xml::parse(pkg.get(&pres_part)?)?;
    let slides = shape::list_slides(&pkg, &pres_part, &pres.root)?;

    for (i, op) in ops.iter().enumerate() {
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

    pkg.save_to_vec()
}
