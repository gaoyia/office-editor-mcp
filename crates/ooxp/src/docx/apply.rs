//! 按段落 id 替换可见文本。

use crate::error::{Error, ErrorCode};
use crate::opc::{self, Package};
use crate::xml;

use super::para;
use super::request::SetParaTextOp;

pub fn apply(bytes: &[u8], ops: &[SetParaTextOp]) -> Result<Vec<u8>, Error> {
    let mut pkg = Package::open_bytes(bytes.to_vec())?;
    let part = opc::office_document_part(&pkg)?;
    let mut doc = xml::parse(pkg.get(&part)?)?;

    for (i, op) in ops.iter().enumerate() {
        let target = para::parse_para_id(&op.id, i)?;
        let p = para::find_paragraph_mut(&mut doc.root, &target).ok_or_else(|| {
            Error::with_path(
                ErrorCode::IdNotFound,
                format!("找不到段落：{}", op.id),
                format!("ops[{i}].id"),
            )
        })?;
        para::set_visible_text(p, &op.text);
    }

    pkg.set(&part, doc.to_bytes()?)?;
    pkg.save_to_vec()
}
