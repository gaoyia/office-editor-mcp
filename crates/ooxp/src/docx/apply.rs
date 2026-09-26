//! 按段落 id 替换可见文本（主文档或页眉页脚）。

use std::collections::HashMap;

use crate::error::{Error, ErrorKind};
use crate::opc::{self, Package};
use crate::xml::{self, Document};

use super::hf;
use super::para::{self, ParaTarget};
use super::request::SetParaTextOp;

pub fn apply(bytes: &[u8], ops: &[SetParaTextOp]) -> Result<Vec<u8>, Error> {
    let mut pkg = Package::open_bytes(bytes.to_vec())?;
    let doc_part = opc::office_document_part(&pkg)?;
    let doc_xml = xml::parse(pkg.get(&doc_part)?)?;
    let catalog = hf::catalog(&pkg, &doc_part, &doc_xml.root)?;

    // 按部件缓存已解析的文档；最后一并写回脏部件。
    let mut docs: HashMap<String, Document> = HashMap::new();
    docs.insert(doc_part.clone(), doc_xml);
    let mut dirty: HashMap<String, ()> = HashMap::new();

    for (i, op) in ops.iter().enumerate() {
        let target = para::parse_para_id(&op.id, i)?;
        let (part_name, body) = match &target {
            ParaTarget::Body(p) => (doc_part.clone(), p.clone()),
            ParaTarget::HeaderFooter {
                kind,
                hf_type,
                n,
                para,
            } => {
                let container_id = format!("{}:{}:{}", kind.as_str(), hf_type.as_str(), n);
                let item = hf::find_by_id(&catalog, &container_id).ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::ParaNotFound {
                            id: op.id.clone(),
                        },
                        format!("ops[{i}].id"),
                    )
                })?;
                (item.part.clone(), para.clone())
            }
        };

        if !docs.contains_key(&part_name) {
            let parsed = xml::parse(pkg.get(&part_name)?)?;
            docs.insert(part_name.clone(), parsed);
        }
        let doc = docs.get_mut(&part_name).unwrap();
        let p = para::find_paragraph_mut(&mut doc.root, &body).ok_or_else(|| {
            Error::with_path(
                ErrorKind::ParaNotFound {
                    id: op.id.clone(),
                },
                format!("ops[{i}].id"),
            )
        })?;
        para::set_visible_text(p, &op.text);
        dirty.insert(part_name, ());
    }

    for name in dirty.keys() {
        let doc = docs.remove(name).unwrap();
        pkg.set(name, doc.to_bytes()?)?;
    }

    pkg.save_to_vec()
}
