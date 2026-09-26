//! 按 id 改段落文本或节上的页眉页脚距边界。

use std::collections::HashMap;

use crate::error::{Error, ErrorKind};
use crate::opc::{self, Package};
use crate::xml::{self, Document, Element, Node};

use super::hf;
use super::para::{self, ParaTarget};
use super::request::{DocxOp, SetHfDistanceOp, SetParaTextOp};

pub fn apply(bytes: &[u8], ops: &[DocxOp]) -> Result<Vec<u8>, Error> {
    let mut pkg = Package::open_bytes(bytes.to_vec())?;
    let doc_part = opc::office_document_part(&pkg)?;
    let doc_xml = xml::parse(pkg.get(&doc_part)?)?;
    let catalog = hf::catalog(&pkg, &doc_part, &doc_xml.root)?;

    // 按部件缓存已解析的文档；最后一并写回脏部件。
    let mut docs: HashMap<String, Document> = HashMap::new();
    docs.insert(doc_part.clone(), doc_xml);
    let mut dirty: HashMap<String, ()> = HashMap::new();

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
        }
    }

    for name in dirty.keys() {
        let doc = docs.remove(name).unwrap();
        pkg.set(name, doc.to_bytes()?)?;
    }

    pkg.save_to_vec()
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
    let (part_name, body) = match &target {
        ParaTarget::Body(p) => (doc_part.to_string(), p.clone()),
        ParaTarget::HeaderFooter {
            kind,
            hf_type,
            n,
            para,
        } => {
            let container_id = format!("{}:{}:{}", kind.as_str(), hf_type.as_str(), n);
            let item = hf::find_by_id(catalog, &container_id).ok_or_else(|| {
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
    Ok(())
}

fn apply_hf_distance(root: &mut Element, op: &SetHfDistanceOp, i: usize) -> Result<(), Error> {
    let sect = hf::find_sect_pr_mut(root, op.n).ok_or_else(|| {
        Error::with_path(
            ErrorKind::SectionNotFound {
                id: op.id.clone(),
            },
            format!("ops[{i}].id"),
        )
    })?;
    let has_pg = sect.child_by_local("pgMar").is_some();
    if !has_pg {
        let mut pg = Element::new(sect.prefixed("pgMar"));
        if let Some(h) = op.header_distance {
            hf::set_twip_attr(&mut pg, "header", h);
        }
        if let Some(f) = op.footer_distance {
            hf::set_twip_attr(&mut pg, "footer", f);
        }
        sect.children.push(Node::Element(pg));
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
