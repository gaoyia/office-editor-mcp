//! 从演示文稿读出页名单和文本形状。

use serde_json::{json, Value};

use crate::error::Error;
use crate::opc::{self, Package};
use crate::xml;

use super::request::InspectScope;
use super::shape;

pub fn inspect(bytes: &[u8], scope: Option<InspectScope>) -> Result<Value, Error> {
    let pkg = Package::open_bytes(bytes.to_vec())?;
    let pres_part = opc::office_document_part(&pkg)?;
    let pres = xml::parse(pkg.get(&pres_part)?)?;
    let slides = shape::list_slides(&pkg, &pres_part, &pres.root)?;
    let slide_count = slides.len();

    let mut slide_json = Vec::new();
    for s in &slides {
        let mut obj = json!({
            "id": format!("slide:{}", s.index),
            "index": s.index,
            "name": s.name,
            "hidden": s.hidden,
        });
        if let Some(scope) = &scope {
            if scope.slide as usize == s.index {
                let doc = xml::parse(pkg.get(&s.part)?)?;
                let all = shape::collect_text_shapes(&doc.root);
                let mut truncated = false;
                let take = if all.len() as u32 > scope.max_shapes {
                    truncated = true;
                    scope.max_shapes as usize
                } else {
                    all.len()
                };
                let mut shapes = Vec::new();
                for (i, sp) in all.iter().take(take).enumerate() {
                    let index = i + 1;
                    let (id, shape_id, stable) = shape::shape_stable_id(sp, s.index, index);
                    let mut sh = json!({
                        "id": id,
                        "shapeId": Value::Null,
                        "name": Value::Null,
                        "text": shape::visible_text(sp),
                        "stable": stable,
                    });
                    if let Some(sid) = shape_id {
                        sh["shapeId"] = json!(sid);
                    }
                    if let Some(n) = shape::shape_name(sp) {
                        sh["name"] = json!(n);
                    }
                    shapes.push(sh);
                }
                obj["truncated"] = json!(truncated);
                obj["shapes"] = Value::Array(shapes);
            }
        }
        slide_json.push(obj);
    }

    Ok(json!({
        "schemaVersion": 1,
        "format": "pptx",
        "id": "pptx:presentation",
        "slideCount": slide_count,
        "slides": slide_json,
    }))
}
