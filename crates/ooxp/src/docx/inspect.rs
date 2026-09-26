//! 从主文档与页眉页脚读出大纲和段落。

use serde_json::{json, Value};

use crate::error::Error;
use crate::opc::{self, Package};
use crate::xml;

use super::hf::{self, HfKind, HfPart};
use super::para;
use super::request::InspectScope;

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
    let (headers, footers) = hf_arrays(&pkg, &catalog, expand_hf)?;

    let mut tree = json!({
        "schemaVersion": 1,
        "format": "docx",
        "id": "docx:document",
        "paraCount": para_count,
        "outline": outline,
        "headers": headers,
        "footers": footers,
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
                    paragraphs.push(para_json(p, index, None));
                }
            }
            tree["from"] = json!(from);
            tree["truncated"] = json!(truncated);
            tree["paragraphs"] = Value::Array(paragraphs);
        }
    }

    Ok(tree)
}

fn hf_arrays(
    pkg: &Package,
    catalog: &[HfPart],
    expand: bool,
) -> Result<(Vec<Value>, Vec<Value>), Error> {
    let mut headers = Vec::new();
    let mut footers = Vec::new();
    for item in catalog {
        let node = hf_node(pkg, item, expand)?;
        match item.kind {
            HfKind::Header => headers.push(node),
            HfKind::Footer => footers.push(node),
        }
    }
    Ok((headers, footers))
}

fn hf_node(pkg: &Package, item: &HfPart, expand: bool) -> Result<Value, Error> {
    let bytes = pkg.get(&item.part)?;
    let doc = xml::parse(bytes)?;
    let paras = para::collect_paragraphs(&doc.root);
    let mut node = json!({
        "id": item.id,
        "type": item.hf_type.as_str(),
        "sections": item.sections,
        "paraCount": paras.len(),
    });
    if expand {
        let mut paragraphs = Vec::new();
        for (i, p) in paras.iter().enumerate() {
            let index = i + 1;
            paragraphs.push(para_json(p, index, Some(&item.id)));
        }
        node["paragraphs"] = Value::Array(paragraphs);
    }
    Ok(node)
}

fn para_json(p: &crate::xml::Element, index: usize, container: Option<&str>) -> Value {
    let (id, stable) = match container {
        Some(c) => para::para_stable_id_in(c, p, index),
        None => para::para_stable_id(p, index),
    };
    let style = para::p_style(p);
    let text = para::visible_text(p);
    if let Some(level) = para::heading_level(p) {
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
    }
}
