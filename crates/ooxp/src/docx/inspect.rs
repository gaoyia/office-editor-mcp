//! 从主文档与页眉页脚读出大纲和段落。

use serde_json::{json, Value};

use crate::error::Error;
use crate::opc::{self, Package};
use crate::xml;

use super::hf::{self, HfKind, HfPart};
use super::para;
use super::picture;
use super::request::InspectScope;
use super::textbox;

pub fn inspect(bytes: &[u8], scope: Option<InspectScope>) -> Result<Value, Error> {
    let pkg = Package::open_bytes(bytes.to_vec())?;
    let part = opc::office_document_part(&pkg)?;
    let doc = xml::parse(pkg.get(&part)?)?;
    let paras = para::collect_paragraphs(&doc.root);
    let para_count = paras.len();

    let mut outline = Vec::new();
    for (i, p) in paras.iter().enumerate() {
        let index = i + 1;
        if let Some(level) = para::heading_level(p) {
            let (id, _) = para::para_stable_id(p, index);
            outline.push(json!({
                "id": id,
                "index": index,
                "level": level,
                "style": para::p_style(p),
                "text": para::visible_text(p),
            }));
        }
    }

    let catalog = hf::catalog(&pkg, &part, &doc.root)?;
    let expand_hf = scope.as_ref().is_some_and(|s| s.header_footer);
    let expand_pics = scope.as_ref().is_some_and(|s| s.pictures);
    let expand_txbx = scope.as_ref().is_some_and(|s| s.text_boxes);
    let expand_txbx_content = scope.as_ref().is_some_and(|s| s.text_box_content);
    let (headers, footers) = hf_arrays(
        &pkg,
        &catalog,
        expand_hf,
        expand_pics,
        expand_txbx,
        expand_txbx_content,
    )?;
    let sections = section_nodes(&pkg, &part, &doc.root)?;
    let picture_count = body_picture_count(&paras)?;
    let body_boxes = collect_body_boxes(&paras)?;
    let text_box_count = body_boxes.len();

    let mut tree = json!({
        "schemaVersion": 1,
        "format": "docx",
        "id": "docx:document",
        "paraCount": para_count,
        "pictureCount": picture_count,
        "textBoxCount": text_box_count,
        "outline": outline,
        "headers": headers,
        "footers": footers,
        "sections": sections,
    });

    if let Some(scope) = scope {
        if let (Some(from), Some(max_paras)) = (scope.from, scope.max_paras) {
            let start = from as usize;
            let mut paragraphs = Vec::new();
            let mut truncated = false;
            if start <= para_count {
                for (i, p) in paras.iter().enumerate().skip(start.saturating_sub(1)) {
                    if paragraphs.len() as u32 >= max_paras {
                        truncated = true;
                        break;
                    }
                    let index = i + 1;
                    paragraphs.push(body_para_json(
                        p,
                        index,
                        expand_pics,
                        expand_txbx,
                        expand_txbx_content,
                        &body_boxes,
                    )?);
                }
            }
            tree["from"] = json!(from);
            tree["truncated"] = json!(truncated);
            tree["paragraphs"] = Value::Array(paragraphs);
        }
    }

    Ok(tree)
}

fn body_picture_count(paras: &[&crate::xml::Element]) -> Result<usize, Error> {
    let mut n = 0usize;
    for (i, p) in paras.iter().enumerate() {
        let (id, _) = para::para_stable_id(p, i + 1);
        n += picture::collect_pictures(p, &id)?.len();
    }
    Ok(n)
}

fn section_nodes(
    pkg: &Package,
    doc_part: &str,
    root: &crate::xml::Element,
) -> Result<Vec<Value>, Error> {
    let sects = hf::collect_sect_prs(root);
    let links = hf::section_link_states(pkg, doc_part, root)?;
    let mut out = Vec::new();
    for (i, sect) in sects.iter().enumerate() {
        let index = i + 1;
        let pg = sect.child_by_local("pgMar");
        let link = links.get(i).copied().unwrap_or(hf::SectionLinkState {
            header: hf::HfLinkTriple {
                default: Some(false),
                even: None,
                first: None,
            },
            footer: hf::HfLinkTriple {
                default: Some(false),
                even: None,
                first: None,
            },
        });
        out.push(json!({
            "id": format!("section:{index}"),
            "index": index,
            "headerDistance": hf::twip_attr(pg, "header")?,
            "footerDistance": hf::twip_attr(pg, "footer")?,
            "headerLink": {
                "default": link.header.default,
                "even": link.header.even,
                "first": link.header.first,
            },
            "footerLink": {
                "default": link.footer.default,
                "even": link.footer.even,
                "first": link.footer.first,
            },
        }));
    }
    Ok(out)
}

fn collect_body_boxes(paras: &[&crate::xml::Element]) -> Result<Vec<textbox::TxbxInfo>, Error> {
    let mut all = Vec::new();
    for (i, p) in paras.iter().enumerate() {
        let (id, _) = para::para_stable_id(p, i + 1);
        all.extend(textbox::collect_textboxes(p, &id)?);
    }
    textbox::resolve_links(&mut all);
    Ok(all)
}

fn boxes_for_para<'a>(all: &'a [textbox::TxbxInfo], para_id: &str) -> Vec<&'a textbox::TxbxInfo> {
    let prefix = format!("{para_id}/txbx:");
    all.iter().filter(|b| b.id.starts_with(&prefix)).collect()
}

fn hf_arrays(
    pkg: &Package,
    catalog: &[HfPart],
    expand: bool,
    pictures: bool,
    text_boxes: bool,
    text_box_content: bool,
) -> Result<(Vec<Value>, Vec<Value>), Error> {
    let mut headers = Vec::new();
    let mut footers = Vec::new();
    for item in catalog {
        let node = hf_node(pkg, item, expand, pictures, text_boxes, text_box_content)?;
        match item.kind {
            HfKind::Header => headers.push(node),
            HfKind::Footer => footers.push(node),
        }
    }
    Ok((headers, footers))
}

fn hf_node(
    pkg: &Package,
    item: &HfPart,
    expand: bool,
    pictures: bool,
    text_boxes: bool,
    text_box_content: bool,
) -> Result<Value, Error> {
    let bytes = pkg.get(&item.part)?;
    let doc = xml::parse(bytes)?;
    let paras = para::collect_paragraphs(&doc.root);
    let pics = picture::collect_pictures(&doc.root, &item.id)?;
    let mut boxes = Vec::new();
    for (i, p) in paras.iter().enumerate() {
        let (id, _) = para::para_stable_id_in(&item.id, p, i + 1);
        boxes.extend(textbox::collect_textboxes(p, &id)?);
    }
    textbox::resolve_links(&mut boxes);
    let mut node = json!({
        "id": item.id,
        "type": item.hf_type.as_str(),
        "sections": item.sections,
        "paraCount": paras.len(),
        "pictureCount": pics.len(),
        "textBoxCount": boxes.len(),
    });
    if pictures {
        node["pictures"] = Value::Array(picture::pictures_json(&pics));
    }
    if text_boxes {
        let mut listed = Vec::new();
        for (i, p) in paras.iter().enumerate() {
            let (id, _) = para::para_stable_id_in(&item.id, p, i + 1);
            let mine: Vec<_> = boxes_for_para(&boxes, &id).into_iter().cloned().collect();
            listed.extend(textbox::textboxes_json(
                p,
                &id,
                &mine,
                text_box_content,
                pictures,
            )?);
        }
        node["textBoxes"] = Value::Array(listed);
    }
    if expand {
        let mut paragraphs = Vec::new();
        for (i, p) in paras.iter().enumerate() {
            let index = i + 1;
            paragraphs.push(para_json(p, index, Some(&item.id), None, None));
        }
        node["paragraphs"] = Value::Array(paragraphs);
    }
    Ok(node)
}

fn body_para_json(
    p: &crate::xml::Element,
    index: usize,
    expand_pics: bool,
    expand_txbx: bool,
    expand_txbx_content: bool,
    all_boxes: &[textbox::TxbxInfo],
) -> Result<Value, Error> {
    let (id, _) = para::para_stable_id(p, index);
    let pics = picture::collect_pictures(p, &id)?;
    let mine: Vec<_> = boxes_for_para(all_boxes, &id)
        .into_iter()
        .cloned()
        .collect();
    let mut node = para_json(p, index, None, Some(pics.len()), Some(mine.len()));
    if expand_pics {
        node["pictures"] = Value::Array(picture::pictures_json(&pics));
    }
    if expand_txbx {
        node["textBoxes"] = Value::Array(textbox::textboxes_json(
            p,
            &id,
            &mine,
            expand_txbx_content,
            expand_pics,
        )?);
    }
    Ok(node)
}

fn para_json(
    p: &crate::xml::Element,
    index: usize,
    container: Option<&str>,
    picture_count: Option<usize>,
    text_box_count: Option<usize>,
) -> Value {
    let (id, stable) = match container {
        Some(c) => para::para_stable_id_in(c, p, index),
        None => para::para_stable_id(p, index),
    };
    let style = para::p_style(p);
    let text = para::visible_text(p);
    let mut node = if let Some(level) = para::heading_level(p) {
        json!({
            "id": id,
            "index": index,
            "style": style,
            "level": level,
            "text": text,
            "stable": stable,
        })
    } else {
        json!({
            "id": id,
            "index": index,
            "style": style,
            "text": text,
            "stable": stable,
        })
    };
    if let Some(n) = picture_count {
        node["pictureCount"] = json!(n);
    }
    if let Some(n) = text_box_count {
        node["textBoxCount"] = json!(n);
    }
    node
}
