//! 按 id 改段落文本、节上距边界、页眉页脚是否链接前节，或 DrawingML 图。

use std::collections::{HashMap, HashSet};

use crate::error::{Error, ErrorKind};
use crate::opc::{self, rels_part_name, ContentTypes, Package, Relationship, Relationships};
use crate::xml::{self, Document, Element, Node};

use super::charts;
use super::smartart;
use super::comments;
use super::hf::{self, HfKind};
use super::notes;
use super::para::{self, HostPara, NoteKind, ParaTarget};
use super::picture;
use super::request::{
    DeleteCommentOp, DeleteNoteOp, DeleteParaOp, DeletePictureOp, DeleteTextBoxOp, DocxOp,
    InsertCommentOp, InsertNoteOp, InsertParaOp, InsertParaPlace, InsertPictureOp, InsertTextBoxOp,
    NoteNumberTarget, ReplyCommentOp, ReplacePictureOp, SetCommentDoneOp, SetCommentMetaOp,
    SetCommentTextOp, SetHfDistanceOp, SetHfLinkOp, SetNoteNumberingOp, SetParaTextOp,
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
            DocxOp::InsertFootnote(op) => {
                apply_insert_note(
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
                apply_insert_note(
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
                apply_delete_note(&mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::DeleteEndnote(op) => {
                apply_delete_note(&mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::SetNoteNumbering(op) => {
                apply_note_numbering(&mut pkg, &mut docs, &mut dirty, &doc_part, op, i)?;
            }
            DocxOp::InsertComment(op) => {
                apply_insert_comment(&mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::ReplyComment(op) => {
                apply_reply_comment(&mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::DeleteComment(op) => {
                apply_delete_comment(&mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::SetCommentText(op) => {
                apply_set_comment_text(&mut pkg, &mut docs, &mut dirty, &doc_part, op, i)?;
            }
            DocxOp::SetCommentMeta(op) => {
                apply_set_comment_meta(&mut pkg, &mut docs, &mut dirty, &doc_part, op, i)?;
            }
            DocxOp::SetCommentDone(op) => {
                apply_set_comment_done(&mut pkg, &mut docs, &mut dirty, &doc_part, op, i)?;
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
            DocxOp::InsertSmartArt(op) => {
                smartart::apply_insert(&mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::DeleteSmartArt(op) => {
                smartart::apply_delete(&mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
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
                smartart::apply_colors(&mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::SetSmartArtStyle(op) => {
                smartart::apply_style(&mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
            }
            DocxOp::SetSmartArtLayout(op) => {
                smartart::apply_layout(&mut pkg, &mut docs, &mut dirty, &doc_part, &catalog, op, i)?;
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
                    ErrorKind::ParaNotFound { id: op.id.clone() },
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
                    ErrorKind::ParaNotFound { id: op.id.clone() },
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
                    ErrorKind::ParaNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )
            })?;
            let content = textbox::find_txbx_content_by_key_mut(host_p, key).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::ParaNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )
            })?;
            let el = para::find_paragraph_in_txbx_mut(content, para).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::ParaNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )
            })?;
            para::set_visible_text(el, &op.text);
            dirty.insert(part_name, ());
        }
        ParaTarget::InNote {
            kind,
            note_id,
            para,
        } => {
            let part_name = notes::part_name(pkg, doc_part, *kind)?.ok_or_else(|| {
                Error::with_path(
                    ErrorKind::ParaNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )
            })?;
            let doc = load_part(pkg, docs, &part_name)?;
            let note = notes::find_user_note_mut(&mut doc.root, *kind, note_id).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::ParaNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )
            })?;
            let el = para::find_paragraph_mut(note, para).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::ParaNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )
            })?;
            para::set_visible_text(el, &op.text);
            dirty.insert(part_name, ());
        }
        ParaTarget::InComment { comment_id, para } => {
            let part_name = comments::part_name(pkg, doc_part)?.ok_or_else(|| {
                Error::with_path(
                    ErrorKind::ParaNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )
            })?;
            let doc = load_part(pkg, docs, &part_name)?;
            let comment = comments::find_comment_mut(&mut doc.root, comment_id).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::ParaNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )
            })?;
            comments::reject_ink(comment, &op.id, i)?;
            let el = para::find_paragraph_mut(comment, para).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::ParaNotFound { id: op.id.clone() },
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
            ErrorKind::SectionNotFound { id: op.id.clone() },
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
            ErrorKind::SectionNotFound { id: op.id.clone() },
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
    pkg: &Package,
    catalog: &[hf::HfPart],
    doc_part: &str,
    target: &picture::PicTarget,
    id: &str,
    i: usize,
) -> Result<String, Error> {
    match target {
        picture::PicTarget::Body { .. } => Ok(doc_part.to_string()),
        picture::PicTarget::InTextBox { host, .. } => part_for_host(catalog, doc_part, host, id, i),
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
        picture::PicTarget::InNote { kind, .. } => {
            notes::part_name(pkg, doc_part, *kind)?.ok_or_else(|| {
                Error::with_path(
                    ErrorKind::PictureNotFound { id: id.to_string() },
                    format!("ops[{i}].id"),
                )
            })
        }
        picture::PicTarget::InComment { .. } => comments::part_name(pkg, doc_part)?.ok_or_else(|| {
            Error::with_path(
                ErrorKind::PictureNotFound { id: id.to_string() },
                format!("ops[{i}].id"),
            )
        }),
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
        picture::PicTarget::InNote {
            kind,
            note_id,
            para,
            key,
        } => {
            let note = notes::find_user_note_mut(root, *kind, note_id)?;
            let inner = para::find_paragraph_mut(note, para)?;
            picture::find_drawing_mut(inner, key)
        }
        picture::PicTarget::InComment {
            comment_id,
            para,
            key,
        } => {
            let comment = comments::find_comment_mut(root, comment_id)?;
            let inner = para::find_paragraph_mut(comment, para)?;
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
        picture::PicTarget::InNote {
            kind,
            note_id,
            para,
            key,
        } => match notes::find_user_note_mut(root, *kind, note_id) {
            Some(note) => match para::find_paragraph_mut(note, para) {
                Some(inner) => picture::remove_picture(inner, key),
                None => false,
            },
            None => false,
        },
        picture::PicTarget::InComment {
            comment_id,
            para,
            key,
        } => match comments::find_comment_mut(root, comment_id) {
            Some(comment) => match para::find_paragraph_mut(comment, para) {
                Some(inner) => picture::remove_picture(inner, key),
                None => false,
            },
            None => false,
        },
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
    reject_ink_pic(pkg, docs, doc_part, &op.target, &op.id, i)?;
    let part_name = part_for_pic(pkg, catalog, doc_part, &op.target, &op.id, i)?;
    let doc = load_part(pkg, docs, &part_name)?;
    let drawing = find_target_drawing_mut(&mut doc.root, &op.target).ok_or_else(|| {
        Error::with_path(
            ErrorKind::PictureNotFound { id: op.id.clone() },
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
            kind, hf_type, n, ..
        } => part_for_hf(catalog, *kind, *hf_type, *n, &op.id, i)?,
        ParaTarget::InTextBox { host, .. } => part_for_host(catalog, doc_part, host, &op.id, i)?,
        ParaTarget::InNote { kind, .. } => notes::part_name(pkg, doc_part, *kind)?.ok_or_else(|| {
            Error::with_path(
                ErrorKind::ParaNotFound { id: op.id.clone() },
                format!("ops[{i}].id"),
            )
        })?,
        ParaTarget::InComment { .. } => comments::part_name(pkg, doc_part)?.ok_or_else(|| {
            Error::with_path(
                ErrorKind::ParaNotFound { id: op.id.clone() },
                format!("ops[{i}].id"),
            )
        })?,
    };
    if let ParaTarget::InComment { comment_id, .. } = &op.para {
        let doc = load_part(pkg, docs, &part_name)?;
        let comment = comments::find_comment(&doc.root, comment_id).ok_or_else(|| {
            Error::with_path(
                ErrorKind::ParaNotFound { id: op.id.clone() },
                format!("ops[{i}].id"),
            )
        })?;
        comments::reject_ink(comment, &op.id, i)?;
    }
    {
        let doc = load_part(pkg, docs, &part_name)?;
        if find_insert_para_mut(&mut doc.root, &op.para).is_none() {
            return Err(Error::with_path(
                ErrorKind::ParaNotFound { id: op.id.clone() },
                format!("ops[{i}].id"),
            ));
        }
    }
    let (rid, cx, cy) = picture::add_media(pkg, &part_name, &op.file, op.cx, op.cy)?;

    let doc = load_part(pkg, docs, &part_name)?;
    picture::ensure_drawing_ns(&mut doc.root);
    let doc_pr = picture::next_docpr_id(&doc.root);
    let p = find_insert_para_mut(&mut doc.root, &op.para).ok_or_else(|| {
        Error::with_path(
            ErrorKind::ParaNotFound { id: op.id.clone() },
            format!("ops[{i}].id"),
        )
    })?;
    picture::insert_prepared_drawing(
        p,
        doc_pr,
        &rid,
        cx,
        cy,
        picture::PicKind::Inline,
        None,
        &picture::WrapPos::default(),
    )?;
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
    reject_ink_pic(pkg, docs, doc_part, &op.target, &op.id, i)?;
    let part_name = part_for_pic(pkg, catalog, doc_part, &op.target, &op.id, i)?;
    let (rid, media) = {
        let doc = load_part(pkg, docs, &part_name)?;
        let rid = {
            let drawing = find_target_drawing_mut(&mut doc.root, &op.target).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::PictureNotFound { id: op.id.clone() },
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
                ErrorKind::PictureNotFound { id: op.id.clone() },
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
    reject_ink_pic(pkg, docs, doc_part, &op.target, &op.id, i)?;
    let part_name = part_for_pic(pkg, catalog, doc_part, &op.target, &op.id, i)?;
    let doc = load_part(pkg, docs, &part_name)?;
    let drawing = find_target_drawing_mut(&mut doc.root, &op.target).ok_or_else(|| {
        Error::with_path(
            ErrorKind::PictureNotFound { id: op.id.clone() },
            format!("ops[{i}].id"),
        )
    })?;
    if !picture::set_extent(drawing, op.cx, op.cy) {
        return Err(Error::with_path(
            ErrorKind::PictureNotFound { id: op.id.clone() },
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
    reject_ink_pic(pkg, docs, doc_part, &op.target, &op.id, i)?;
    let part_name = part_for_pic(pkg, catalog, doc_part, &op.target, &op.id, i)?;
    let doc = load_part(pkg, docs, &part_name)?;
    picture::ensure_drawing_ns(&mut doc.root);
    let drawing = find_target_drawing_mut(&mut doc.root, &op.target).ok_or_else(|| {
        Error::with_path(
            ErrorKind::PictureNotFound { id: op.id.clone() },
            format!("ops[{i}].id"),
        )
    })?;
    picture::apply_wrap(drawing, op.kind, op.wrap, &op.pos)
        .map_err(|e| e.at(format!("ops[{i}]")))?;
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
            ErrorKind::ParaNotFound { id: id.to_string() },
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
            kind, hf_type, n, ..
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

fn find_insert_para_mut<'a>(root: &'a mut Element, target: &ParaTarget) -> Option<&'a mut Element> {
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

fn txbx_missing(id: &str, i: usize) -> Error {
    Error::with_path(
        ErrorKind::TextBoxNotFound { id: id.to_string() },
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
            ErrorKind::ParaNotFound { id: op.id.clone() },
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
    match &op.place {
        InsertParaPlace::TextBox { box_target, after } => {
            let part_name = part_for_txbx(catalog, doc_part, box_target, &op.id, i)?;
            let doc = load_part(pkg, docs, &part_name)?;
            let new_id = para::next_para_hex(&doc.root);
            let host = para::find_paragraph_mut(&mut doc.root, box_target.host.body())
                .ok_or_else(|| txbx_missing(&op.id, i))?;
            let content = textbox::find_txbx_content_by_key_mut(host, &box_target.key)
                .ok_or_else(|| txbx_missing(&op.id, i))?;
            let new_p = para::new_paragraph(content, &new_id, &op.text);
            if let Some(ParaTarget::InTextBox { para, .. }) = after {
                insert_p_after(content, para, new_p).map_err(|_| {
                    Error::with_path(
                        ErrorKind::ParaNotFound { id: op.id.clone() },
                        format!("ops[{i}].id"),
                    )
                })?;
            } else {
                content.children.push(Node::Element(new_p));
            }
            dirty.insert(part_name, ());
            Ok(())
        }
        InsertParaPlace::Note {
            kind,
            note_id,
            after,
        } => {
            let part_name = notes::part_name(pkg, doc_part, *kind)?.ok_or_else(|| {
                Error::with_path(
                    ErrorKind::NoteNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )
            })?;
            let doc = load_part(pkg, docs, &part_name)?;
            let new_id = para::next_para_hex(&doc.root);
            let note = notes::find_user_note_mut(&mut doc.root, *kind, note_id).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::NoteNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )
            })?;
            let new_p = para::new_paragraph(note, &new_id, &op.text);
            if let Some(para) = after {
                insert_p_after(note, para, new_p).map_err(|_| {
                    Error::with_path(
                        ErrorKind::ParaNotFound { id: op.id.clone() },
                        format!("ops[{i}].id"),
                    )
                })?;
            } else {
                note.children.push(Node::Element(new_p));
            }
            dirty.insert(part_name, ());
            Ok(())
        }
        InsertParaPlace::Comment {
            comment_id,
            after,
        } => {
            let part_name = comments::part_name(pkg, doc_part)?.ok_or_else(|| {
                comments::comment_missing(&op.id, i)
            })?;
            let old_last = {
                let doc = load_part(pkg, docs, &part_name)?;
                let comment = comments::find_comment(&doc.root, comment_id)
                    .ok_or_else(|| comments::comment_missing(&op.id, i))?;
                comments::reject_ink(comment, &op.id, i)?;
                comments::last_para_id(comment)
            };
            let doc = load_part(pkg, docs, &part_name)?;
            let new_id = para::next_para_hex(&doc.root);
            let comment = comments::find_comment_mut(&mut doc.root, comment_id)
                .ok_or_else(|| comments::comment_missing(&op.id, i))?;
            let new_p = para::new_paragraph(comment, &new_id, &op.text);
            if let Some(para) = after {
                insert_p_after(comment, para, new_p).map_err(|_| {
                    Error::with_path(
                        ErrorKind::ParaNotFound { id: op.id.clone() },
                        format!("ops[{i}].id"),
                    )
                })?;
            } else {
                comment.children.push(Node::Element(new_p));
            }
            dirty.insert(part_name.clone(), ());
            retarget_comment_last(pkg, docs, dirty, doc_part, comment_id, old_last.as_deref())?;
            Ok(())
        }
    }
}

fn insert_p_after(
    parent: &mut Element,
    para: &para::BodyPara,
    new_p: Element,
) -> Result<(), ()> {
    let mut idx = 0usize;
    let mut insert_at = None;
    for (j, n) in parent.children.iter().enumerate() {
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
    let at = insert_at.ok_or(())?;
    parent.children.insert(at, Node::Element(new_p));
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
    match &op.inner {
        ParaTarget::InTextBox { host, key, para } => {
            let part_name = part_for_host(catalog, doc_part, host, &op.id, i)?;
            let doc = load_part(pkg, docs, &part_name)?;
            let host_p = para::find_paragraph_mut(&mut doc.root, host.body()).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::ParaNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )
            })?;
            let content = textbox::find_txbx_content_by_key_mut(host_p, key).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::ParaNotFound { id: op.id.clone() },
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
            remove_p_at(content, para).map_err(|_| {
                Error::with_path(
                    ErrorKind::ParaNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )
            })?;
            dirty.insert(part_name, ());
            Ok(())
        }
        ParaTarget::InNote {
            kind,
            note_id,
            para,
        } => {
            let part_name = notes::part_name(pkg, doc_part, *kind)?.ok_or_else(|| {
                Error::with_path(
                    ErrorKind::NoteNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )
            })?;
            let doc = load_part(pkg, docs, &part_name)?;
            let note = notes::find_user_note_mut(&mut doc.root, *kind, note_id).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::NoteNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )
            })?;
            let count = para::collect_paragraphs(note).len();
            if count <= 1 {
                return Err(Error::with_path(
                    ErrorKind::LastNotePara,
                    format!("ops[{i}].id"),
                ));
            }
            remove_p_at(note, para).map_err(|_| {
                Error::with_path(
                    ErrorKind::ParaNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )
            })?;
            dirty.insert(part_name, ());
            Ok(())
        }
        ParaTarget::InComment { comment_id, para } => {
            let part_name = comments::part_name(pkg, doc_part)?.ok_or_else(|| {
                comments::comment_missing(&op.id, i)
            })?;
            let old_last = {
                let doc = load_part(pkg, docs, &part_name)?;
                let comment = comments::find_comment(&doc.root, comment_id)
                    .ok_or_else(|| comments::comment_missing(&op.id, i))?;
                comments::reject_ink(comment, &op.id, i)?;
                comments::last_para_id(comment)
            };
            let doc = load_part(pkg, docs, &part_name)?;
            let comment = comments::find_comment_mut(&mut doc.root, comment_id)
                .ok_or_else(|| comments::comment_missing(&op.id, i))?;
            let count = para::collect_paragraphs(comment).len();
            if count <= 1 {
                return Err(Error::with_path(
                    ErrorKind::LastCommentPara,
                    format!("ops[{i}].id"),
                ));
            }
            remove_p_at(comment, para).map_err(|_| {
                Error::with_path(
                    ErrorKind::ParaNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )
            })?;
            dirty.insert(part_name.clone(), ());
            retarget_comment_last(pkg, docs, dirty, doc_part, comment_id, old_last.as_deref())?;
            Ok(())
        }
        _ => Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: crate::error::IdReason::InsertParaNeedsBox,
            },
            format!("ops[{i}].id"),
        )),
    }
}

fn remove_p_at(parent: &mut Element, para: &para::BodyPara) -> Result<(), ()> {
    let mut idx = 0usize;
    let mut remove_at = None;
    for (j, n) in parent.children.iter().enumerate() {
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
    let at = remove_at.ok_or(())?;
    parent.children.remove(at);
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

fn apply_insert_note(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &InsertNoteOp,
    kind: NoteKind,
    i: usize,
) -> Result<(), Error> {
    let host_part = match &op.host {
        ParaTarget::Body(_) => doc_part.to_string(),
        ParaTarget::HeaderFooter {
            kind: hf_kind,
            hf_type,
            n,
            ..
        } => part_for_hf(catalog, *hf_kind, *hf_type, *n, &op.id, i)?,
        ParaTarget::InTextBox { host, .. } => part_for_host(catalog, doc_part, host, &op.id, i)?,
        ParaTarget::InNote { .. } | ParaTarget::InComment { .. } => {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: crate::error::IdReason::NotePrefix,
                },
                format!("ops[{i}].id"),
            ));
        }
    };
    {
        let doc = load_part(pkg, docs, &host_part)?;
        if find_insert_para_mut(&mut doc.root, &op.host).is_none() {
            return Err(Error::with_path(
                ErrorKind::ParaNotFound { id: op.id.clone() },
                format!("ops[{i}].id"),
            ));
        }
    }
    let notes_part = notes::ensure_part(pkg, docs, dirty, doc_part, kind)?;
    let (w_id, para_hex) = {
        let doc = load_part(pkg, docs, &notes_part)?;
        let w_id = notes::next_user_id(&doc.root, kind);
        let para_hex = para::next_para_hex(&doc.root);
        notes::append_user_note(&mut doc.root, kind, &w_id, &para_hex, &op.text);
        dirty.insert(notes_part.clone(), ());
        (w_id, para_hex)
    };
    let _ = para_hex;
    let doc = load_part(pkg, docs, &host_part)?;
    let p = find_insert_para_mut(&mut doc.root, &op.host).ok_or_else(|| {
        Error::with_path(
            ErrorKind::ParaNotFound { id: op.id.clone() },
            format!("ops[{i}].id"),
        )
    })?;
    notes::append_reference(p, kind, &w_id);
    dirty.insert(host_part, ());
    Ok(())
}

fn apply_delete_note(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &DeleteNoteOp,
    i: usize,
) -> Result<(), Error> {
    let notes_part = notes::part_name(pkg, doc_part, op.kind)?.ok_or_else(|| {
        Error::with_path(
            ErrorKind::NoteNotFound { id: op.id.clone() },
            format!("ops[{i}].id"),
        )
    })?;
    let (embeds, media) = {
        let doc = load_part(pkg, docs, &notes_part)?;
        if notes::find_user_note(&doc.root, op.kind, &op.note_id).is_none() {
            let is_sep = notes::seps(&doc.root, op.kind).iter().any(|e| {
                notes::ids_match(e.attr_local("id").unwrap_or(""), &op.note_id)
            });
            return Err(Error::with_path(
                if is_sep {
                    ErrorKind::InvalidId {
                        reason: crate::error::IdReason::NoteSep,
                    }
                } else {
                    ErrorKind::NoteNotFound { id: op.id.clone() }
                },
                format!("ops[{i}].id"),
            ));
        }
        let note = notes::find_user_note(&doc.root, op.kind, &op.note_id).unwrap();
        let embeds = notes::collect_blip_embeds(note);
        let mut media = Vec::new();
        for rid in &embeds {
            if let Ok(m) = picture::resolve_embed(pkg, &notes_part, rid) {
                media.push(m);
            }
        }
        (embeds, media)
    };
    {
        let doc = load_part(pkg, docs, &notes_part)?;
        notes::remove_user_note(&mut doc.root, op.kind, &op.note_id);
        dirty.insert(notes_part.clone(), ());
    }
    {
        let doc = load_part(pkg, docs, doc_part)?;
        notes::remove_references(&mut doc.root, op.kind, &op.note_id);
        dirty.insert(doc_part.to_string(), ());
    }
    for item in catalog {
        let doc = load_part(pkg, docs, &item.part)?;
        notes::remove_references(&mut doc.root, op.kind, &op.note_id);
        dirty.insert(item.part.clone(), ());
    }
    let notes_root = &docs.get(&notes_part).unwrap().root;
    for rid in embeds {
        picture::remove_rel_if_unused(pkg, &notes_part, &rid, notes_root)?;
    }
    for m in media {
        if !picture::media_still_used(pkg, &m)? {
            pkg.remove(&m);
        }
    }
    Ok(())
}

fn apply_note_numbering(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    op: &SetNoteNumberingOp,
    i: usize,
) -> Result<(), Error> {
    match op.target {
        NoteNumberTarget::Document => {
            let settings = notes::settings_part_name(pkg, doc_part)?.ok_or_else(|| {
                Error::with_path(
                    ErrorKind::PartMissing {
                        name: "/word/settings.xml".into(),
                    },
                    format!("ops[{i}].id"),
                )
            })?;
            let doc = load_part(pkg, docs, &settings)?;
            notes::apply_numbering(
                &mut doc.root,
                op.kind,
                op.num_fmt.as_deref(),
                op.num_start,
                op.num_restart.as_deref(),
            );
            dirty.insert(settings, ());
            // Word 16.0 读的是节上的 numFmt；只写 settings 时 COM NumberStyle 仍是阿拉伯。
            let n_sects = hf::collect_sect_prs(&load_part(pkg, docs, doc_part)?.root).len();
            for n in 1..=n_sects {
                let doc = load_part(pkg, docs, doc_part)?;
                let Some(sect) = hf::find_sect_pr_mut(&mut doc.root, n) else {
                    continue;
                };
                notes::apply_numbering(
                    sect,
                    op.kind,
                    op.num_fmt.as_deref(),
                    op.num_start,
                    op.num_restart.as_deref(),
                );
            }
            if n_sects > 0 {
                dirty.insert(doc_part.to_string(), ());
            }
        }
        NoteNumberTarget::Section(n) => {
            let doc = load_part(pkg, docs, doc_part)?;
            let sect = hf::find_sect_pr_mut(&mut doc.root, n).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::SectionNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )
            })?;
            notes::apply_numbering(
                sect,
                op.kind,
                op.num_fmt.as_deref(),
                op.num_start,
                op.num_restart.as_deref(),
            );
            dirty.insert(doc_part.to_string(), ());
        }
    }
    Ok(())
}

fn part_for_para(
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
        ParaTarget::InTextBox { host, .. } => part_for_host(catalog, doc_part, host, id, i),
        ParaTarget::InNote { kind, .. } => notes::part_name(pkg, doc_part, *kind)?.ok_or_else(|| {
            Error::with_path(
                ErrorKind::ParaNotFound { id: id.to_string() },
                format!("ops[{i}].id"),
            )
        }),
        ParaTarget::InComment { .. } => Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: crate::error::IdReason::CommentPrefix,
            },
            format!("ops[{i}].id"),
        )),
    }
}

fn para_index_of(root: &Element, target: &ParaTarget) -> Option<usize> {
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

fn retarget_comment_last(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    comment_id: &str,
    old_last: Option<&str>,
) -> Result<(), Error> {
    let Some(part) = comments::part_name(pkg, doc_part)? else {
        return Ok(());
    };
    let new_last = {
        let doc = load_part(pkg, docs, &part)?;
        comments::find_comment(&doc.root, comment_id).and_then(comments::last_para_id)
    };
    let (Some(old), Some(new)) = (old_last, new_last.as_deref()) else {
        return Ok(());
    };
    if comments::para_ids_match(old, new) {
        return Ok(());
    }
    if let Some(ex) = comments::extended_part_name(pkg, doc_part)? {
        let doc = load_part(pkg, docs, &ex)?;
        comments::retarget_para_id(&mut doc.root, old, new);
        dirty.insert(ex, ());
    }
    comments::retarget_ids_parts(pkg, docs, dirty, doc_part, old, new)?;
    Ok(())
}

fn apply_insert_comment(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &InsertCommentOp,
    i: usize,
) -> Result<(), Error> {
    let start_key = comments::story_key(doc_part, &op.start_target).map_err(|_| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: crate::error::IdReason::CommentPrefix,
            },
            format!("ops[{i}].start"),
        )
    })?;
    let end_key = comments::story_key(doc_part, &op.end_target).map_err(|_| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: crate::error::IdReason::CommentPrefix,
            },
            format!("ops[{i}].end"),
        )
    })?;
    if start_key != end_key {
        return Err(Error::with_path(
            ErrorKind::CommentRangeCrossPart,
            format!("ops[{i}]"),
        ));
    }
    let host_part = part_for_para(pkg, catalog, doc_part, &op.start_target, &op.start, i)?;
    {
        let doc = load_part(pkg, docs, &host_part)?;
        let start_idx = para_index_of(&doc.root, &op.start_target).ok_or_else(|| {
            Error::with_path(
                ErrorKind::ParaNotFound {
                    id: op.start.clone(),
                },
                format!("ops[{i}].start"),
            )
        })?;
        let end_idx = para_index_of(&doc.root, &op.end_target).ok_or_else(|| {
            Error::with_path(
                ErrorKind::ParaNotFound { id: op.end.clone() },
                format!("ops[{i}].end"),
            )
        })?;
        if start_idx > end_idx {
            return Err(Error::with_path(
                ErrorKind::CommentRangeReversed,
                format!("ops[{i}]"),
            ));
        }
    }
    let comments_part = comments::ensure_comments_part(pkg, docs, dirty, doc_part)?;
    let extended_part = comments::ensure_extended_part(pkg, docs, dirty, doc_part)?;
    let people_part = comments::ensure_people_part(pkg, docs, dirty, doc_part)?;
    let (w_id, para_hex) = {
        let doc = load_part(pkg, docs, &comments_part)?;
        let w_id = comments::next_comment_id(&doc.root);
        let para_hex = para::next_para_hex(&doc.root);
        let (p_style, r_style) = comments::first_comment_styles(&doc.root);
        comments::append_comment(
            &mut doc.root,
            &w_id,
            &para_hex,
            &op.text,
            &op.author,
            &op.initials,
            op.date.as_deref(),
            &p_style,
            &r_style,
        );
        dirty.insert(comments_part.clone(), ());
        (w_id, para_hex)
    };
    {
        let doc = load_part(pkg, docs, &extended_part)?;
        comments::upsert_comment_ex(&mut doc.root, &para_hex, None, Some(false));
        dirty.insert(extended_part, ());
    }
    {
        let doc = load_part(pkg, docs, &people_part)?;
        comments::ensure_person(&mut doc.root, &op.author);
        dirty.insert(people_part, ());
    }
    comments::patch_ids_parts(pkg, docs, dirty, doc_part, &para_hex)?;
    {
        let doc = load_part(pkg, docs, &host_part)?;
        let p = find_insert_para_mut(&mut doc.root, &op.start_target).ok_or_else(|| {
            Error::with_path(
                ErrorKind::ParaNotFound {
                    id: op.start.clone(),
                },
                format!("ops[{i}].start"),
            )
        })?;
        comments::insert_range_start(p, &w_id);
    }
    {
        let doc = load_part(pkg, docs, &host_part)?;
        let p = find_insert_para_mut(&mut doc.root, &op.end_target).ok_or_else(|| {
            Error::with_path(
                ErrorKind::ParaNotFound { id: op.end.clone() },
                format!("ops[{i}].end"),
            )
        })?;
        comments::insert_range_end(p, &w_id);
        dirty.insert(host_part, ());
    }
    Ok(())
}

fn apply_reply_comment(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &ReplyCommentOp,
    i: usize,
) -> Result<(), Error> {
    let comments_part = comments::part_name(pkg, doc_part)?
        .ok_or_else(|| comments::comment_missing(&op.id, i))?;
    let (parent_last, p_style, r_style) = {
        let doc = load_part(pkg, docs, &comments_part)?;
        let parent = comments::find_comment(&doc.root, &op.comment_id)
            .ok_or_else(|| comments::comment_missing(&op.id, i))?;
        comments::reject_ink(parent, &op.id, i)?;
        let last = comments::last_para_id(parent).ok_or_else(|| comments::need_para_id(i))?;
        let (p_style, r_style) = comments::styles_of(parent);
        (last, p_style, r_style)
    };
    let comments_part = comments::ensure_comments_part(pkg, docs, dirty, doc_part)?;
    let extended_part = comments::ensure_extended_part(pkg, docs, dirty, doc_part)?;
    let people_part = comments::ensure_people_part(pkg, docs, dirty, doc_part)?;
    let (w_id, para_hex) = {
        let doc = load_part(pkg, docs, &comments_part)?;
        let w_id = comments::next_comment_id(&doc.root);
        let para_hex = para::next_para_hex(&doc.root);
        comments::append_comment(
            &mut doc.root,
            &w_id,
            &para_hex,
            &op.text,
            &op.author,
            &op.initials,
            op.date.as_deref(),
            &p_style,
            &r_style,
        );
        dirty.insert(comments_part, ());
        (w_id, para_hex)
    };
    comments::insert_reply_markers_in_hosts(
        pkg,
        docs,
        dirty,
        doc_part,
        catalog,
        &op.comment_id,
        &w_id,
    )?;
    {
        let doc = load_part(pkg, docs, &extended_part)?;
        comments::upsert_comment_ex(&mut doc.root, &para_hex, Some(&parent_last), Some(false));
        dirty.insert(extended_part, ());
    }
    {
        let doc = load_part(pkg, docs, &people_part)?;
        comments::ensure_person(&mut doc.root, &op.author);
        dirty.insert(people_part, ());
    }
    comments::patch_ids_parts(pkg, docs, dirty, doc_part, &para_hex)?;
    Ok(())
}

fn apply_delete_comment(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &DeleteCommentOp,
    i: usize,
) -> Result<(), Error> {
    let comments_part = comments::part_name(pkg, doc_part)?
        .ok_or_else(|| comments::comment_missing(&op.id, i))?;
    {
        let comments_doc = load_part(pkg, docs, &comments_part)?;
        if comments::find_comment(&comments_doc.root, &op.comment_id).is_none() {
            return Err(comments::comment_missing(&op.id, i));
        }
    }
    let extended_part = comments::extended_part_name(pkg, doc_part)?;
    let extended_root = if let Some(ex) = &extended_part {
        Some(load_part(pkg, docs, ex)?.root.clone())
    } else {
        None
    };
    let (ids, last_paras, embeds) = {
        let comments_doc = load_part(pkg, docs, &comments_part)?;
        let ids = comments::cascade_ids(
            &comments_doc.root,
            extended_root.as_ref(),
            &op.comment_id,
        );
        let mut last_paras = Vec::new();
        let mut embeds = Vec::new();
        for id in &ids {
            if let Some(c) = comments::find_comment(&comments_doc.root, id) {
                if let Some(p) = comments::last_para_id(c) {
                    last_paras.push(p);
                }
                embeds.extend(comments::collect_blip_embeds(c));
            }
        }
        (ids, last_paras, embeds)
    };
    let mut media = Vec::new();
    for rid in &embeds {
        if let Ok(m) = picture::resolve_embed(pkg, &comments_part, rid) {
            media.push(m);
        }
    };
    let id_set: HashSet<String> = ids.iter().cloned().collect();
    comments::strip_markers_in_hosts(pkg, docs, dirty, doc_part, catalog, &id_set)?;
    {
        let doc = load_part(pkg, docs, &comments_part)?;
        for id in &ids {
            comments::remove_comment(&mut doc.root, id);
        }
        dirty.insert(comments_part.clone(), ());
    }
    if let Some(ex) = &extended_part {
        let doc = load_part(pkg, docs, ex)?;
        for pid in &last_paras {
            comments::remove_comment_ex_for_para(&mut doc.root, pid);
        }
        dirty.insert(ex.clone(), ());
    }
    for pid in &last_paras {
        comments::remove_ids_for_para(pkg, docs, dirty, doc_part, pid)?;
    }
    let comments_root = docs.get(&comments_part).unwrap().root.clone();
    for rid in embeds {
        picture::remove_rel_if_unused(pkg, &comments_part, &rid, &comments_root)?;
    }
    for m in media {
        if !picture::media_still_used(pkg, &m)? {
            pkg.remove(&m);
        }
    }
    Ok(())
}

fn apply_set_comment_text(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    op: &SetCommentTextOp,
    i: usize,
) -> Result<(), Error> {
    let comments_part = comments::part_name(pkg, doc_part)?
        .ok_or_else(|| comments::comment_missing(&op.id, i))?;
    let old_last = {
        let doc = load_part(pkg, docs, &comments_part)?;
        let comment = comments::find_comment(&doc.root, &op.comment_id)
            .ok_or_else(|| comments::comment_missing(&op.id, i))?;
        comments::reject_ink(comment, &op.id, i)?;
        comments::last_para_id(comment)
    };
    {
        let doc = load_part(pkg, docs, &comments_part)?;
        let comment = comments::find_comment_mut(&mut doc.root, &op.comment_id)
            .ok_or_else(|| comments::comment_missing(&op.id, i))?;
        comments::set_visible_keep_first(comment, &op.text);
        dirty.insert(comments_part, ());
    }
    retarget_comment_last(
        pkg,
        docs,
        dirty,
        doc_part,
        &op.comment_id,
        old_last.as_deref(),
    )?;
    Ok(())
}

fn apply_set_comment_meta(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    op: &SetCommentMetaOp,
    i: usize,
) -> Result<(), Error> {
    let comments_part = comments::part_name(pkg, doc_part)?
        .ok_or_else(|| comments::comment_missing(&op.id, i))?;
    {
        let doc = load_part(pkg, docs, &comments_part)?;
        let comment = comments::find_comment_mut(&mut doc.root, &op.comment_id)
            .ok_or_else(|| comments::comment_missing(&op.id, i))?;
        comments::reject_ink(comment, &op.id, i)?;
        if let Some(a) = &op.author {
            set_w_attr(comment, "author", a);
        }
        if let Some(ini) = &op.initials {
            set_w_attr(comment, "initials", ini);
        }
        if let Some(d) = &op.date {
            set_w_attr(comment, "date", d);
        }
        dirty.insert(comments_part, ());
    }
    if let Some(author) = &op.author {
        let people_part = comments::ensure_people_part(pkg, docs, dirty, doc_part)?;
        let doc = load_part(pkg, docs, &people_part)?;
        comments::ensure_person(&mut doc.root, author);
        dirty.insert(people_part, ());
    }
    Ok(())
}

fn apply_set_comment_done(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    op: &SetCommentDoneOp,
    i: usize,
) -> Result<(), Error> {
    let comments_part = comments::part_name(pkg, doc_part)?
        .ok_or_else(|| comments::comment_missing(&op.id, i))?;
    let last = {
        let doc = load_part(pkg, docs, &comments_part)?;
        let comment = comments::find_comment(&doc.root, &op.comment_id)
            .ok_or_else(|| comments::comment_missing(&op.id, i))?;
        comments::reject_ink(comment, &op.id, i)?;
        comments::last_para_id(comment).ok_or_else(|| comments::need_para_id(i))?
    };
    let extended_part = comments::ensure_extended_part(pkg, docs, dirty, doc_part)?;
    {
        let comments_doc = load_part(pkg, docs, &comments_part)?;
        let comments_root = comments_doc.root.clone();
        let ex = load_part(pkg, docs, &extended_part)?;
        if comments::fill_missing_comment_ex(&comments_root, &mut ex.root).is_err() {
            return Err(comments::need_para_id(i));
        }
        if !comments::set_done_on_para(&mut ex.root, &last, op.done) {
            comments::upsert_comment_ex(&mut ex.root, &last, None, Some(op.done));
        }
        dirty.insert(extended_part, ());
    }
    Ok(())
}

fn set_w_attr(el: &mut Element, local: &str, val: &str) {
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

fn reject_ink_pic(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    doc_part: &str,
    target: &picture::PicTarget,
    id: &str,
    i: usize,
) -> Result<(), Error> {
    let picture::PicTarget::InComment { comment_id, .. } = target else {
        return Ok(());
    };
    let part = comments::part_name(pkg, doc_part)?.ok_or_else(|| {
        Error::with_path(
            ErrorKind::PictureNotFound { id: id.to_string() },
            format!("ops[{i}].id"),
        )
    })?;
    let doc = load_part(pkg, docs, &part)?;
    let comment = comments::find_comment(&doc.root, comment_id).ok_or_else(|| {
        Error::with_path(
            ErrorKind::PictureNotFound { id: id.to_string() },
            format!("ops[{i}].id"),
        )
    })?;
    comments::reject_ink(comment, id, i)
}
