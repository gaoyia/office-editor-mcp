//! 从演示文稿读出页名单、文本形状、图和图表。

use serde_json::{json, Map, Value};

use crate::error::Error;
use crate::opc::{self, Package};
use crate::xml;

use super::chart;
use super::comments;
use super::handout;
use super::hyperlink;
use super::master;
use super::notes;
use super::picture;
use super::request::InspectScope;
use super::shape;
use super::smartart;
use super::table;
use super::timing;
use super::transition;

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
    let want_hyperlinks = scope.as_ref().is_some_and(|s| s.hyperlinks);
    let want_tables = scope.as_ref().is_some_and(|s| s.tables);
    let want_timing = scope.as_ref().is_some_and(|s| s.timing);
    let want_masters = scope.as_ref().is_some_and(|s| s.masters);
    let want_handout = scope.as_ref().is_some_and(|s| s.handout);
    let catalog = master::catalog(&pkg, &pres_part, &pres.root)?;

    let mut slide_json = Vec::new();
    let mut picture_count = 0usize;
    let mut chart_count = 0usize;
    let mut smart_art_count = 0usize;
    let mut comment_count = 0usize;
    let mut hyperlink_count = 0usize;
    let mut table_count = 0usize;
    let mut anim_count = 0usize;
    for s in &slides {
        let doc = xml::parse(pkg.get(&s.part)?)?;
        let host = format!("slide:{}", s.index);
        let pics = picture::collect_pictures(&doc.root, &host);
        picture_count += pics.len();
        let charts_list = if want_charts {
            Some(chart::collect_charts(&pkg, &doc.root, &host, &s.part)?)
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
                &pkg, &doc.root, &host, &s.part,
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
            Some(comments::collect_on_slide(
                &pkg,
                &s.part,
                s.index,
                want_pictures,
            )?)
        } else {
            None
        };
        let n_comments = match &comments_list {
            Some(list) => list.len(),
            None => comments::count_on_slide(&pkg, &s.part)?,
        };
        comment_count += n_comments;
        let rels = opc::Relationships::from_package(&pkg, &s.part)?;
        let n_links = hyperlink::count(&doc.root);
        hyperlink_count += n_links;
        let tables_list = if want_tables {
            Some(table::collect_tables(&doc.root, s.index))
        } else {
            None
        };
        let n_tables = tables_list
            .as_ref()
            .map(|c| c.len())
            .unwrap_or_else(|| table::count_tables(&doc.root));
        table_count += n_tables;
        let anims = timing::list(&doc.root, s.index);
        let n_anims = anims.len();
        anim_count += n_anims;
        let mut obj = Map::new();
        obj.insert("id".into(), json!(host));
        obj.insert("index".into(), json!(s.index));
        obj.insert("name".into(), json!(s.name));
        obj.insert(
            "layoutId".into(),
            master::layout_id_json(&pkg, &catalog, &s.part)?,
        );
        obj.insert("hidden".into(), json!(s.hidden));
        obj.insert("transition".into(), transition::inspect(&doc.root));
        obj.insert("hasNotes".into(), json!(has));
        obj.insert("pictureCount".into(), json!(pics.len()));
        obj.insert("chartCount".into(), json!(n_charts));
        obj.insert("smartArtCount".into(), json!(n_smartarts));
        obj.insert("commentCount".into(), json!(n_comments));
        obj.insert("hyperlinkCount".into(), json!(n_links));
        obj.insert("tableCount".into(), json!(n_tables));
        obj.insert("animCount".into(), json!(n_anims));
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
        if let Some(tables) = tables_list {
            obj.insert("tables".into(), Value::Array(tables));
        }
        if want_notes {
            obj.insert(
                "notes".into(),
                notes::notes_json(
                    &pkg,
                    &s.part,
                    s.index,
                    want_pictures,
                    want_charts,
                    want_smartarts,
                )?,
            );
        }
        if let Some(list) = comments_list {
            obj.insert("comments".into(), Value::Array(list));
        }
        if want_hyperlinks {
            obj.insert(
                "hyperlinks".into(),
                hyperlink::list_json(&doc.root, s.index, &s.part, &rels, &slides),
            );
        }
        if want_timing {
            obj.insert("timing".into(), Value::Array(anims));
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
                    shapes.push(shape::shape_node_json(sp, &host, index));
                }
                obj.insert("truncated".into(), json!(truncated));
                obj.insert("shapes".into(), Value::Array(shapes));
            }
        }
        slide_json.push(Value::Object(obj));
    }

    let mut root = Map::new();
    root.insert("schemaVersion".into(), json!(1));
    root.insert("format".into(), json!("pptx"));
    root.insert("id".into(), json!("pptx:presentation"));
    root.insert("slideCount".into(), json!(slide_count));
    root.insert("pictureCount".into(), json!(picture_count));
    root.insert("chartCount".into(), json!(chart_count));
    root.insert("smartArtCount".into(), json!(smart_art_count));
    root.insert("commentCount".into(), json!(comment_count));
    root.insert("hyperlinkCount".into(), json!(hyperlink_count));
    root.insert("tableCount".into(), json!(table_count));
    root.insert("animCount".into(), json!(anim_count));
    root.insert("masterCount".into(), json!(catalog.count()));
    root.insert(
        "hasHandout".into(),
        json!(handout::has_handout(&pkg, &pres_part)?),
    );
    root.insert("props".into(), crate::props::inspect(&pkg)?);
    if want_masters {
        root.insert("masters".into(), master::masters_json(&pkg, &catalog)?);
    }
    if want_handout {
        root.insert("handout".into(), handout::handout_json(&pkg, &pres_part)?);
    }
    root.insert("slides".into(), Value::Array(slide_json));
    Ok(Value::Object(root))
}
