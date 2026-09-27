//! 从演示文稿读出页名单、文本形状、图和图表。

use serde_json::{json, Map, Value};

use crate::error::Error;
use crate::opc::{self, Package};
use crate::xml;

use super::chart;
use super::comments;
use super::notes;
use super::picture;
use super::request::InspectScope;
use super::shape;
use super::smartart;

pub fn inspect(bytes: &[u8], scope: Option<InspectScope>) -> Result<Value, Error> {
    let pkg = Package::open_bytes(bytes.to_vec())?;
    let pres_part = opc::office_document_part(&pkg)?;
    let pres = xml::parse(pkg.get(&pres_part)?)?;
    let slides = shape::list_slides(&pkg, &pres_part, &pres.root)?;
    let slide_count = slides.len();
    let want_pictures = scope.as_ref().is_some_and(|s| s.pictures);
    let want_charts = scope.as_ref().is_some_and(|s| s.charts);
    let want_smartarts = scope.as_ref().is_some_and(|s| s.smart_art);
    let want_notes = scope.as_ref().is_some_and(|s| s.notes);
    let want_comments = scope.as_ref().is_some_and(|s| s.comments);

    let mut slide_json = Vec::new();
    let mut picture_count = 0usize;
    let mut chart_count = 0usize;
    let mut smart_art_count = 0usize;
    let mut comment_count = 0usize;
    for s in &slides {
        let doc = xml::parse(pkg.get(&s.part)?)?;
        let host = format!("slide:{}", s.index);
        let pics = picture::collect_pictures(&doc.root, &host);
        picture_count += pics.len();
        let charts_list = if want_charts {
            Some(chart::collect_charts(&pkg, &doc.root, s.index, &s.part)?)
        } else {
            None
        };
        let n_charts = charts_list
            .as_ref()
            .map(|c| c.len())
            .unwrap_or_else(|| chart::count_charts(&doc.root));
        chart_count += n_charts;
        let smartarts_list = if want_smartarts {
            Some(smartart::collect_smartarts(
                &pkg, &doc.root, s.index, &s.part,
            )?)
        } else {
            None
        };
        let n_smartarts = smartarts_list
            .as_ref()
            .map(|c| c.len())
            .unwrap_or_else(|| smartart::count_smartarts(&doc.root));
        smart_art_count += n_smartarts;
        let has = notes::has_notes(&pkg, &s.part)?;
        let comments_list = if want_comments {
            Some(comments::collect_on_slide(&pkg, &s.part, s.index)?)
        } else {
            None
        };
        let n_comments = match &comments_list {
            Some(list) => list.len(),
            None => comments::count_on_slide(&pkg, &s.part)?,
        };
        comment_count += n_comments;
        let mut obj = Map::new();
        obj.insert("id".into(), json!(host));
        obj.insert("index".into(), json!(s.index));
        obj.insert("name".into(), json!(s.name));
        obj.insert("hidden".into(), json!(s.hidden));
        obj.insert("hasNotes".into(), json!(has));
        obj.insert("pictureCount".into(), json!(pics.len()));
        obj.insert("chartCount".into(), json!(n_charts));
        obj.insert("smartArtCount".into(), json!(n_smartarts));
        obj.insert("commentCount".into(), json!(n_comments));
        if want_pictures {
            obj.insert(
                "pictures".into(),
                Value::Array(picture::pictures_json(&pics)),
            );
        }
        if let Some(charts) = charts_list {
            obj.insert("charts".into(), Value::Array(charts));
        }
        if let Some(smartarts) = smartarts_list {
            obj.insert("smartArts".into(), Value::Array(smartarts));
        }
        if want_notes {
            obj.insert(
                "notes".into(),
                notes::notes_json(&pkg, &s.part, s.index, want_pictures)?,
            );
        }
        if let Some(list) = comments_list {
            obj.insert("comments".into(), Value::Array(list));
        }
        if let Some(scope) = &scope {
            if scope.slide == Some(s.index as u32) {
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
                    let (id, shape_id, stable) = shape::shape_stable_id(sp, &host, index);
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
                obj.insert("truncated".into(), json!(truncated));
                obj.insert("shapes".into(), Value::Array(shapes));
            }
        }
        slide_json.push(Value::Object(obj));
    }

    Ok(json!({
        "schemaVersion": 1,
        "format": "pptx",
        "id": "pptx:presentation",
        "slideCount": slide_count,
        "pictureCount": picture_count,
        "chartCount": chart_count,
        "smartArtCount": smart_art_count,
        "commentCount": comment_count,
        "props": crate::props::inspect(&pkg)?,
        "slides": slide_json,
    }))
}
