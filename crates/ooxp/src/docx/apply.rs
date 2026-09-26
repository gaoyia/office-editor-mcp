//! 按 id 改段落文本、节上距边界、页眉页脚是否链接前节，或 DrawingML 图。

use std::collections::{HashMap, HashSet};

use crate::error::{Error, ErrorKind};
use crate::opc::{self, rels_part_name, ContentTypes, Package, Relationship, Relationships};
use crate::xml::{self, Document, Element, Node};

use super::hf::{self, HfKind};
use super::para::{self, HostPara, ParaTarget};
use super::picture;
use super::request::{
    DeleteParaOp, DeletePictureOp, DeleteTextBoxOp, DocxOp, InsertParaOp, InsertPictureOp,
    InsertTextBoxOp, ReplacePictureOp, SetHfDistanceOp, SetHfLinkOp, SetParaTextOp,
    SetPictureSizeOp, SetPictureWrapOp, SetTextBoxDirectionOp, SetTextBoxFillOp, SetTextBoxLineOp,
    SetTextBoxLinkOp, SetTextBoxSizeOp, SetTextBoxWrapOp,
};
use super::textbox;
use super::{CT_FOOTER, CT_HEADER, REL_FOOTER, REL_HEADER};

pub fn apply(bytes: &[u8], ops: &[DocxOp]) -> Result<Vec<u8>, Error> {
    let mut pkg = Package::open_bytes(bytes.to_vec())?;
    let doc_part = opc::office_document_part(&pkg)?;
    let doc_xml = xml::parse(pkg.get(&doc_part)?)?;
    let catalog = hf::catalog(&pkg, &doc_part, &doc_xml.root)?;

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
            DocxOp::InsertPara(op) => {
                apply_insert_para(&mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::DeletePara(op) => {
                apply_delete_para(&mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::SetTextBoxLink(op) => {
                apply_txbx_link(&mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
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
            let el = para::find_paragraph_mut(&mut doc.root, p).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::ParaNotFound {
                        id: op.id.clone(),
                    },
                    format!("ops[{i}].id"),
                )
            })?;
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
            let el = para::find_paragraph_mut(&mut doc.root, para).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::ParaNotFound {
                        id: op.id.clone(),
                    },
                    format!("ops[{i}].id"),
                )
            })?;
            para::set_visible_text(el, &op.text);
            dirty.insert(part_name, ());
        }
        ParaTarget::InTextBox { host, key, para } => {
            let part_name = part_for_host(catalog, doc_part, host, &op.id, i)?;
            let doc = load_part(pkg, docs, &part_name)?;
            let host_p = para::find_paragraph_mut(&mut doc.root, host.body()).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::ParaNotFound {
                        id: op.id.clone(),
                    },
                    format!("ops[{i}].id"),
                )
            })?;
            let content = textbox::find_txbx_content_by_key_mut(host_p, key).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::ParaNotFound {
                        id: op.id.clone(),
                    },
                    format!("ops[{i}].id"),
                )
            })?;
            let el = para::find_paragraph_in_txbx_mut(content, para).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::ParaNotFound {
                        id: op.id.clone(),
                    },
                    format!("ops[{i}].id"),
                )
            })?;
            para::set_visible_text(el, &op.text);
            dirty.insert(part_name, ());
        }
    }
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
            ErrorKind::SectionNotFound {
                id: op.id.clone(),
            },
            format!("ops[{i}].id"),
        ));
    }

    let even_and_odd = hf::even_and_odd_headers(pkg, doc_part)?;
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

fn part_for_pic(
    catalog: &[hf::HfPart],
    doc_part: &str,
    target: &picture::PicTarget,
    id: &str,
    i: usize,
) -> Result<String, Error> {
    match target {
        picture::PicTarget::Body { .. } => Ok(doc_part.to_string()),
        picture::PicTarget::InTextBox { host, .. } => {
            part_for_host(catalog, doc_part, host, id, i)
        }
        picture::PicTarget::HeaderFooter {
            kind,
            hf_type,
            n,
            ..
        } => {
            let container_id = format!("{}:{}:{n}", kind.as_str(), hf_type.as_str());
            let item = hf::find_by_id(catalog, &container_id).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::PictureNotFound {
                        id: id.to_string(),
                    },
                    format!("ops[{i}].id"),
                )
            })?;
            Ok(item.part.clone())
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
    let part_name = part_for_pic(catalog, doc_part, &op.target, &op.id, i)?;
    let doc = load_part(pkg, docs, &part_name)?;
    let drawing = find_target_drawing_mut(&mut doc.root, &op.target).ok_or_else(|| {
        Error::with_path(
            ErrorKind::PictureNotFound {
                id: op.id.clone(),
            },
            format!("ops[{i}].id"),
        )
    })?;
    let rid = picture::blip_embed(drawing).ok_or_else(|| {
        Error::xml_detail(
            crate::error::XmlIssue::Parse,
            format!("{} missing a:blip/@r:embed", op.id),
        )
    })?;
    let old_media = picture::resolve_embed(pkg, &part_name, &rid)?;
    let image = picture::read_image(&op.file)?;
    let old_ext = old_media.rsplit('.').next().unwrap_or("");
    if old_ext.eq_ignore_ascii_case(&image.ext) {
        pkg.set(&old_media, image.bytes)?;
        let mut cts = ContentTypes::from_package(pkg)?;
        cts.ensure_default(&image.ext, &image.content_type);
        cts.write_to(pkg)?;
        return Ok(());
    }
    let new_media = picture::next_media_part(pkg, &image.ext);
    pkg.set(&new_media, image.bytes)?;
    picture::update_rel_target(pkg, &part_name, &rid, &new_media)?;
    let mut cts = ContentTypes::from_package(pkg)?;
    cts.ensure_default(&image.ext, &image.content_type);
    cts.write_to(pkg)?;
    if !picture::media_still_used(pkg, &old_media)? {
        pkg.remove(&old_media);
    }
    let _ = dirty;
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
            kind,
            hf_type,
            n,
            ..
        } => part_for_hf(catalog, *kind, *hf_type, *n, &op.id, i)?,
        ParaTarget::InTextBox { host, .. } => {
            part_for_host(catalog, doc_part, host, &op.id, i)?
        }
    };
    {
        let doc = load_part(pkg, docs, &part_name)?;
        if find_insert_para_mut(&mut doc.root, &op.para).is_none() {
            return Err(Error::with_path(
                ErrorKind::ParaNotFound {
                    id: op.id.clone(),
                },
                format!("ops[{i}].id"),
            ));
        }
    }
    let image = picture::read_image(&op.file)?;
    let (cx, cy) = match (op.cx, op.cy) {
        (Some(x), Some(y)) => (x, y),
        _ => (image.cx, image.cy),
    };
    let media = picture::next_media_part(pkg, &image.ext);
    pkg.set(&media, image.bytes)?;
    let rid = picture::add_image_rel(pkg, &part_name, &media)?;
    let mut cts = ContentTypes::from_package(pkg)?;
    cts.ensure_default(&image.ext, &image.content_type);
    cts.write_to(pkg)?;

    let doc = load_part(pkg, docs, &part_name)?;
    picture::ensure_drawing_ns(&mut doc.root);
    let doc_pr = picture::next_docpr_id(&doc.root);
    let name = format!("图片 {doc_pr}");
    let p = find_insert_para_mut(&mut doc.root, &op.para).ok_or_else(|| {
        Error::with_path(
            ErrorKind::ParaNotFound {
                id: op.id.clone(),
            },
            format!("ops[{i}].id"),
        )
    })?;
    let drawing = picture::build_inline_drawing(doc_pr, &name, &rid, cx, cy);
    picture::insert_drawing_into_para(p, drawing);
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
    let part_name = part_for_pic(catalog, doc_part, &op.target, &op.id, i)?;
    let (rid, media) = {
        let doc = load_part(pkg, docs, &part_name)?;
        let rid = {
            let drawing = find_target_drawing_mut(&mut doc.root, &op.target).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::PictureNotFound {
                        id: op.id.clone(),
                    },
                    format!("ops[{i}].id"),
                )
            })?;
            picture::blip_embed(drawing)
        };
        let media = match rid.as_deref() {
            Some(r) => picture::resolve_embed(pkg, &part_name, r).ok(),
            None => None,
        };
        if !remove_target_picture(&mut doc.root, &op.target) {
            return Err(Error::with_path(
                ErrorKind::PictureNotFound {
                    id: op.id.clone(),
                },
                format!("ops[{i}].id"),
            ));
        }
        (rid, media)
    };
    dirty.insert(part_name.clone(), ());
    if let Some(r) = rid {
        picture::remove_rel_if_unused(pkg, &part_name, &r, &docs.get(&part_name).unwrap().root)?;
    }
    if let Some(m) = media {
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
    let part_name = part_for_pic(catalog, doc_part, &op.target, &op.id, i)?;
    let doc = load_part(pkg, docs, &part_name)?;
    let drawing = find_target_drawing_mut(&mut doc.root, &op.target).ok_or_else(|| {
        Error::with_path(
            ErrorKind::PictureNotFound {
                id: op.id.clone(),
            },
            format!("ops[{i}].id"),
        )
    })?;
    if !picture::set_extent(drawing, op.cx, op.cy) {
        return Err(Error::with_path(
            ErrorKind::PictureNotFound {
                id: op.id.clone(),
            },
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
    let part_name = part_for_pic(catalog, doc_part, &op.target, &op.id, i)?;
    let doc = load_part(pkg, docs, &part_name)?;
    picture::ensure_drawing_ns(&mut doc.root);
    let drawing = find_target_drawing_mut(&mut doc.root, &op.target).ok_or_else(|| {
        Error::with_path(
            ErrorKind::PictureNotFound {
                id: op.id.clone(),
            },
            format!("ops[{i}].id"),
        )
    })?;
    picture::apply_wrap(drawing, op.kind, op.wrap, &op.pos).map_err(|e| e.at(format!("ops[{i}]")))?;
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
            ErrorKind::ParaNotFound {
                id: id.to_string(),
            },
            format!("ops[{i}].id"),
        )
    })?;
    Ok(item.part.clone())
}

fn part_for_host(
    catalog: &[hf::HfPart],
    doc_part: &str,
    host: &HostPara,
    id: &str,
    i: usize,
) -> Result<String, Error> {
    match host {
        HostPara::Body(_) => Ok(doc_part.to_string()),
        HostPara::HeaderFooter {
            kind,
            hf_type,
            n,
            ..
        } => part_for_hf(catalog, *kind, *hf_type, *n, id, i),
    }
}

fn part_for_txbx(
    catalog: &[hf::HfPart],
    doc_part: &str,
    target: &para::TxbxTarget,
    id: &str,
    i: usize,
) -> Result<String, Error> {
    part_for_host(catalog, doc_part, &target.host, id, i)
}

fn find_insert_para_mut<'a>(
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
    }
}

fn txbx_missing(id: &str, i: usize) -> Error {
    Error::with_path(
        ErrorKind::TextBoxNotFound {
            id: id.to_string(),
        },
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
    let part_name = part_for_host(catalog, doc_part, &op.host, &op.id, i)?;
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
            ErrorKind::ParaNotFound {
                id: op.id.clone(),
            },
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
    let part_name = part_for_txbx(catalog, doc_part, &op.target, &op.id, i)?;
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
    let part_name = part_for_txbx(catalog, doc_part, &op.target, &op.id, i)?;
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
    let part_name = part_for_txbx(catalog, doc_part, &op.target, &op.id, i)?;
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
    let part_name = part_for_txbx(catalog, doc_part, &op.target, &op.id, i)?;
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
    let part_name = part_for_txbx(catalog, doc_part, &op.target, &op.id, i)?;
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
    let part_name = part_for_txbx(catalog, doc_part, &op.target, &op.id, i)?;
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

fn apply_insert_para(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &InsertParaOp,
    i: usize,
) -> Result<(), Error> {
    let part_name = part_for_txbx(catalog, doc_part, &op.box_target, &op.id, i)?;
    let doc = load_part(pkg, docs, &part_name)?;
    let new_id = para::next_para_hex(&doc.root);
    let host = para::find_paragraph_mut(&mut doc.root, op.box_target.host.body())
        .ok_or_else(|| txbx_missing(&op.id, i))?;
    let content = textbox::find_txbx_content_by_key_mut(host, &op.box_target.key)
        .ok_or_else(|| txbx_missing(&op.id, i))?;
    let new_p = para::new_paragraph(content, &new_id, &op.text);
    if let Some(ParaTarget::InTextBox { para, .. }) = &op.after {
        let mut idx = 0usize;
        let mut insert_at = None;
        for (j, n) in content.children.iter().enumerate() {
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
        let at = insert_at.ok_or_else(|| {
            Error::with_path(
                ErrorKind::ParaNotFound {
                    id: op.id.clone(),
                },
                format!("ops[{i}].id"),
            )
        })?;
        content.children.insert(at, Node::Element(new_p));
    } else {
        content.children.push(Node::Element(new_p));
    }
    dirty.insert(part_name, ());
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
    let ParaTarget::InTextBox { host, key, para } = &op.inner else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: crate::error::IdReason::InsertParaNeedsBox,
            },
            format!("ops[{i}].id"),
        ));
    };
    let part_name = part_for_host(catalog, doc_part, host, &op.id, i)?;
    let doc = load_part(pkg, docs, &part_name)?;
    let host_p = para::find_paragraph_mut(&mut doc.root, host.body()).ok_or_else(|| {
        Error::with_path(
            ErrorKind::ParaNotFound {
                id: op.id.clone(),
            },
            format!("ops[{i}].id"),
        )
    })?;
    let content = textbox::find_txbx_content_by_key_mut(host_p, key).ok_or_else(|| {
        Error::with_path(
            ErrorKind::ParaNotFound {
                id: op.id.clone(),
            },
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
    let mut idx = 0usize;
    let mut remove_at = None;
    for (j, n) in content.children.iter().enumerate() {
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
    let at = remove_at.ok_or_else(|| {
        Error::with_path(
            ErrorKind::ParaNotFound {
                id: op.id.clone(),
            },
            format!("ops[{i}].id"),
        )
    })?;
    content.children.remove(at);
    dirty.insert(part_name, ());
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
    let part_a = part_for_txbx(catalog, doc_part, &op.target, &op.id, i)?;
    if let Some(next) = &op.next {
        let part_b = part_for_txbx(
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
