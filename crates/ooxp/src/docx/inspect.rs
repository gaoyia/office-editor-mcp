//! 从主文档读出大纲和段落。

use serde_json::{json, Value};

use crate::error::Error;
use crate::opc::{self, Package};
use crate::xml;

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
            let mut item = json!({
                "id": id,
                "index": index,
                "level": level,
                "text": para::visible_text(p),
            });
            match para::p_style(p) {
                Some(s) => item["style"] = json!(s),
                None => item["style"] = json!(null),
            }
            outline.push(item);
        }
    }

    let mut tree = json!({
        "schemaVersion": 1,
        "format": "docx",
        "id": "docx:document",
        "paraCount": para_count,
        "outline": outline,
    });

    if let Some(scope) = scope {
        let start = scope.from as usize;
        let mut paragraphs = Vec::new();
        let mut truncated = false;
        if start <= para_count {
            for (i, p) in paras.iter().enumerate().skip(start.saturating_sub(1)) {
                if paragraphs.len() as u32 >= scope.max_paras {
                    truncated = true;
                    break;
                }
                let index = i + 1;
                paragraphs.push(para_json(p, index));
            }
        }
        tree["from"] = json!(scope.from);
        tree["truncated"] = json!(truncated);
        tree["paragraphs"] = Value::Array(paragraphs);
    }

    Ok(tree)
}

fn para_json(p: &crate::xml::Element, index: usize) -> Value {
    let (id, stable) = para::para_stable_id(p, index);
    let mut obj = json!({
        "id": id,
        "index": index,
        "text": para::visible_text(p),
        "stable": stable,
    });
    match para::p_style(p) {
        Some(s) => obj["style"] = json!(s),
        None => obj["style"] = json!(null),
    }
    if let Some(level) = para::heading_level(p) {
        obj["level"] = json!(level);
    }
    obj
}
