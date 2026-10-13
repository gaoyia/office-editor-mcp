//! 从演示文稿读出页名单、文本形状、图和图表。

use serde_json::{json, Map, Value};

use crate::error::Error;
use crate::opc::{self, Package};
use crate::xml;

use super::chart;
use super::comments;
use super::connector;
use super::custom_show;
use super::field;
use super::handout;
use super::hyperlink;
use super::master;
use super::media;
use super::model3d;
use super::notes;
use super::ole;
use super::omml;
use super::picture;
use super::protect;
use super::request::InspectScope;
use super::section;
use super::shape;
use super::show;
use super::smartart;
use super::table;
use super::theme;
use super::theme_map;
use super::timing;
use super::transition;
use super::zoom;

pub fn inspect(bytes: &[u8], scope: Option<InspectScope>) -> Result<Value, Error> {
    let pkg = Package::open_bytes(bytes.to_vec())?;
    let pres_part = opc::office_document_part(&pkg)?;
    let pres = xml::parse(pkg.get(&pres_part)?)?;
    let slides = shape::list_slides(&pkg, &pres_part, &pres.root)?;
    let slide_count = slides.len();
    let want_pictures = scope.as_ref().is_some_and(|s| s.pictures);
    let want_media = scope.as_ref().is_some_and(|s| s.media);
    let want_charts = scope.as_ref().is_some_and(|s| s.charts);
    let want_smartarts = scope.as_ref().is_some_and(|s| s.smart_art);
    let want_notes = scope.as_ref().is_some_and(|s| s.notes);
    let want_comments = scope.as_ref().is_some_and(|s| s.comments);
    let want_hyperlinks = scope.as_ref().is_some_and(|s| s.hyperlinks);
    let want_fields = scope.as_ref().is_some_and(|s| s.fields);
    let want_tables = scope.as_ref().is_some_and(|s| s.tables);
    let want_connectors = scope.as_ref().is_some_and(|s| s.connectors);
    let want_timing = scope.as_ref().is_some_and(|s| s.timing);
    let want_masters = scope.as_ref().is_some_and(|s| s.masters);
    let want_notes_master = scope.as_ref().is_some_and(|s| s.notes_master);
    let want_handout = scope.as_ref().is_some_and(|s| s.handout);
    let want_sections = scope.as_ref().is_some_and(|s| s.sections);
    let want_custom_shows = scope.as_ref().is_some_and(|s| s.custom_shows);
    let want_ole_objects = scope.as_ref().is_some_and(|s| s.ole_objects);
    let want_model3ds = scope.as_ref().is_some_and(|s| s.model3ds);
    let want_maths = scope.as_ref().is_some_and(|s| s.maths);
    let want_zooms = scope.as_ref().is_some_and(|s| s.zooms);
    let catalog = master::catalog(&pkg, &pres_part, &pres.root)?;

    let mut slide_json = Vec::new();
    let mut picture_count = 0usize;
    let mut math_count = 0usize;
    let mut video_count = 0usize;
    let mut audio_count = 0usize;
    let mut chart_count = 0usize;
    let mut smart_art_count = 0usize;
    let mut comment_count = 0usize;
    let mut hyperlink_count = 0usize;
    let mut field_count = 0usize;
    let mut table_count = 0usize;
    let mut ole_object_count = 0usize;
    let mut model3d_count = 0usize;
    let mut zoom_count = 0usize;
    let mut connector_count = 0usize;
    let mut anim_count = 0usize;
    for s in &slides {
        let doc = xml::parse(pkg.get(&s.part)?)?;
        let host = format!("slide:{}", s.index);
        let rels = opc::Relationships::from_package(&pkg, &s.part)?;
        let pics = picture::collect_pictures_with(&doc.root, &host, Some(&rels));
        picture_count += pics.len();
        let maths_list = if want_maths {
            Some(omml::collect(&doc.root, &host))
        } else {
            None
        };
        let n_math = maths_list
            .as_ref()
            .map(|c| c.len())
            .unwrap_or_else(|| omml::count(&doc.root));
        math_count += n_math;
        let media_lists = if want_media {
            Some(media::collect(&pkg, &s.part, &doc.root, &host)?)
        } else {
            None
        };
        let (n_videos, n_audios) = match &media_lists {
            Some((v, a)) => (v.len(), a.len()),
            None => media::count(&doc.root),
        };
        video_count += n_videos;
        audio_count += n_audios;
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
        let n_fields = field::count(&doc.root);
        field_count += n_fields;
        let tables_list = if want_tables {
            Some(table::collect_tables(&doc.root, &host))
        } else {
            None
        };
        let n_tables = tables_list
            .as_ref()
            .map(|c| c.len())
            .unwrap_or_else(|| table::count_tables(&doc.root));
        table_count += n_tables;
        let ole_list = if want_ole_objects {
            Some(ole::collect(&pkg, &s.part, &doc.root, &host)?)
        } else {
            None
        };
        let n_ole = match &ole_list {
            Some(list) => list.len(),
            None => ole::count(&pkg, &s.part, &doc.root)?,
        };
        ole_object_count += n_ole;
        let model3d_list = if want_model3ds {
            Some(model3d::collect(&doc.root, &host))
        } else {
            None
        };
        let n_m3d = model3d_list
            .as_ref()
            .map(|c| c.len())
            .unwrap_or_else(|| model3d::count(&doc.root));
        model3d_count += n_m3d;
        let zooms_list = if want_zooms {
            Some(zoom::collect(&doc.root, &host, &slides, &pres.root))
        } else {
            None
        };
        let n_zoom = zooms_list
            .as_ref()
            .map(|c| c.len())
            .unwrap_or_else(|| zoom::count(&doc.root));
        zoom_count += n_zoom;
        let connectors_list = if want_connectors {
            Some(connector::list_json(&doc.root, &host))
        } else {
            None
        };
        let n_cxn = connectors_list
            .as_ref()
            .map(|c| c.len())
            .unwrap_or_else(|| connector::count(&doc.root));
        connector_count += n_cxn;
        let anims = timing::list(&doc.root, &host, false);
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
        obj.insert("clrMapOvr".into(), theme_map::clr_map_ovr_json(&doc.root));
        obj.insert(
            "themeOverride".into(),
            theme_map::theme_override_json(&pkg, &s.part)?,
        );
        obj.insert("hasNotes".into(), json!(has));
        obj.insert("pictureCount".into(), json!(pics.len()));
        obj.insert("mathCount".into(), json!(n_math));
        obj.insert("videoCount".into(), json!(n_videos));
        obj.insert("audioCount".into(), json!(n_audios));
        obj.insert("chartCount".into(), json!(n_charts));
        obj.insert("smartArtCount".into(), json!(n_smartarts));
        obj.insert("commentCount".into(), json!(n_comments));
        obj.insert("hyperlinkCount".into(), json!(n_links));
        obj.insert("fieldCount".into(), json!(n_fields));
        obj.insert("tableCount".into(), json!(n_tables));
        obj.insert("oleObjectCount".into(), json!(n_ole));
        obj.insert("model3dCount".into(), json!(n_m3d));
        obj.insert("zoomCount".into(), json!(n_zoom));
        obj.insert("connectorCount".into(), json!(n_cxn));
        obj.insert("animCount".into(), json!(n_anims));
        if want_pictures {
            obj.insert(
                "pictures".into(),
                Value::Array(picture::pictures_json(&pics)),
            );
        }
        if let Some(items) = maths_list {
            obj.insert("maths".into(), Value::Array(omml::list_json(&items)));
        }
        if let Some((videos, audios)) = media_lists {
            obj.insert("videos".into(), Value::Array(media::list_json(&videos)));
            obj.insert("audios".into(), Value::Array(media::list_json(&audios)));
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
        if let Some(oles) = ole_list {
            obj.insert("oleObjects".into(), Value::Array(oles));
        }
        if let Some(items) = model3d_list {
            obj.insert("model3ds".into(), Value::Array(model3d::list_json(&items)));
        }
        if let Some(zooms) = zooms_list {
            obj.insert("zooms".into(), Value::Array(zooms));
        }
        if let Some(connectors) = connectors_list {
            obj.insert("connectors".into(), Value::Array(connectors));
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
                    want_tables,
                    want_model3ds,
                    want_maths,
                    want_timing,
                    want_media,
                    want_ole_objects,
                    want_zooms,
                    want_connectors,
                    want_hyperlinks,
                    want_fields,
                    &slides,
                    &pres.root,
                )?,
            );
        }
        if let Some(list) = comments_list {
            obj.insert("comments".into(), Value::Array(list));
        }
        if want_hyperlinks {
            obj.insert(
                "hyperlinks".into(),
                hyperlink::list_json(&doc.root, &host, &s.part, &rels, &slides),
            );
        }
        if want_fields {
            obj.insert("fields".into(), field::list_json(&doc.root, &host));
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
    root.insert("mathCount".into(), json!(math_count));
    root.insert("videoCount".into(), json!(video_count));
    root.insert("audioCount".into(), json!(audio_count));
    root.insert("chartCount".into(), json!(chart_count));
    root.insert("smartArtCount".into(), json!(smart_art_count));
    root.insert("commentCount".into(), json!(comment_count));
    root.insert("hyperlinkCount".into(), json!(hyperlink_count));
    root.insert("fieldCount".into(), json!(field_count));
    root.insert("tableCount".into(), json!(table_count));
    root.insert("oleObjectCount".into(), json!(ole_object_count));
    root.insert("model3dCount".into(), json!(model3d_count));
    root.insert("zoomCount".into(), json!(zoom_count));
    root.insert("connectorCount".into(), json!(connector_count));
    root.insert("animCount".into(), json!(anim_count));
    root.insert("masterCount".into(), json!(catalog.count()));
    root.insert(
        "hasHandout".into(),
        json!(handout::has_handout(&pkg, &pres_part)?),
    );
    root.insert("sectionCount".into(), json!(section::count(&pres.root)));
    root.insert("slideSize".into(), theme::slide_size_json(&pres.root));
    root.insert("notesSize".into(), theme::notes_size_json(&pres.root));
    root.insert(
        "customShowCount".into(),
        json!(custom_show::count(&pres.root)),
    );
    root.insert("showPr".into(), show::inspect(&pkg, &pres_part)?);
    root.insert("modifyProtection".into(), protect::inspect(&pres.root));
    root.insert("props".into(), crate::props::inspect(&pkg)?);
    if want_masters {
        root.insert(
            "masters".into(),
            master::masters_json(
                &pkg,
                &catalog,
                want_timing,
                want_pictures,
                want_media,
                want_charts,
                want_smartarts,
                want_tables,
                want_ole_objects,
                want_model3ds,
                want_maths,
                want_zooms,
                want_connectors,
                want_hyperlinks,
                want_fields,
                &slides,
                &pres.root,
            )?,
        );
    }
    if want_notes_master {
        root.insert(
            "notesMaster".into(),
            notes_master_tree(&pkg, &pres_part, want_timing)?,
        );
    }
    if want_handout {
        root.insert(
            "handout".into(),
            handout_tree(&pkg, &pres_part, want_timing)?,
        );
    }
    if want_sections {
        root.insert(
            "sections".into(),
            Value::Array(section::list_json(&pres.root)),
        );
    }
    if want_custom_shows {
        root.insert(
            "customShows".into(),
            Value::Array(custom_show::list_json(&pres.root)),
        );
    }
    root.insert("slides".into(), Value::Array(slide_json));
    Ok(Value::Object(root))
}

fn notes_master_tree(pkg: &Package, pres_part: &str, want_timing: bool) -> Result<Value, Error> {
    let Some(part) = notes::notes_master_part(pkg, pres_part)? else {
        return Ok(Value::Null);
    };
    let doc = xml::parse(pkg.get(&part)?)?;
    let host = "notesMaster";
    let anims = timing::list(&doc.root, host, true);
    let mut obj = Map::new();
    obj.insert("id".into(), json!("notesMaster"));
    obj.insert("theme".into(), theme::theme_json(pkg, &part)?);
    obj.insert("clrMap".into(), super::theme_map::clr_map_json(&doc.root));
    obj.insert("animCount".into(), json!(anims.len()));
    if want_timing {
        obj.insert("timing".into(), Value::Array(anims));
    }
    obj.insert(
        "placeholders".into(),
        json!(master::placeholders_json(&doc.root, host)),
    );
    Ok(Value::Object(obj))
}

fn handout_tree(pkg: &Package, pres_part: &str, want_timing: bool) -> Result<Value, Error> {
    let mut v = handout::handout_json(pkg, pres_part)?;
    let Some(obj) = v.as_object_mut() else {
        return Ok(v);
    };
    let Some(part) = handout::handout_part(pkg, pres_part)? else {
        return Ok(v);
    };
    let doc = xml::parse(pkg.get(&part)?)?;
    let host = "handout";
    let anims = timing::list(&doc.root, host, true);
    obj.insert("animCount".into(), json!(anims.len()));
    if want_timing {
        obj.insert("timing".into(), Value::Array(anims));
    }
    obj.insert(
        "placeholders".into(),
        json!(master::placeholders_json(&doc.root, host)),
    );
    Ok(v)
}
