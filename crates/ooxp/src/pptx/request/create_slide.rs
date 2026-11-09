use serde_json::Value;

use crate::error::{Error, ErrorKind, TypeName};

use super::create_types::*;

pub(crate) fn parse_slide(v: &Value, i: usize) -> Result<NewSlide, Error> {
    let obj = v.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: format!("presentation.slides[{i}]"),
                expected: TypeName::Object,
            },
            format!("presentation.slides[{i}]"),
        )
    })?;
    for k in obj.keys() {
        if !matches!(
            k.as_str(),
            "shapes"
                | "pictures"
                | "videos"
                | "audios"
                | "charts"
                | "smartArts"
                | "tables"
                | "notes"
                | "comments"
                | "transition"
                | "anims"
                | "model3ds"
                | "inks"
                | "maths"
                | "oleObjects"
                | "connectors"
                | "zooms"
        ) {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("presentation.slides[{i}].{k}"),
            ));
        }
    }
    let mut shapes = Vec::new();
    if let Some(arr) = obj.get("shapes") {
        let arr = arr.as_array().ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: format!("presentation.slides[{i}].shapes"),
                    expected: TypeName::Array,
                },
                format!("presentation.slides[{i}].shapes"),
            )
        })?;
        for (j, sh) in arr.iter().enumerate() {
            shapes.push(parse_shape(sh, i, j)?);
        }
    }
    let pictures = super::create_picture::parse_pictures(obj.get("pictures"), i)?;
    let videos = super::create_media::parse_media(obj.get("videos"), i, "videos")?;
    let audios = super::create_media::parse_media(obj.get("audios"), i, "audios")?;
    let charts = super::create_chart::parse_charts(obj.get("charts"), i)?;
    let smart_arts = super::create_drawing::parse_smartarts(obj.get("smartArts"), i)?;
    let tables = super::create_drawing::parse_tables(obj.get("tables"), i)?;
    let notes = super::create_notes::parse_notes(obj.get("notes"), i)?;
    let comments = super::create_comment::parse_comments(obj.get("comments"), i)?;
    let transition = super::create_timing::parse_transition(obj.get("transition"), i)?;
    let anims = super::create_timing::parse_anims(obj.get("anims"), i)?;
    let model3ds = super::create_media::parse_model3ds(
        obj.get("model3ds"),
        i,
        format!("presentation.slides[{i}].model3ds"),
        false,
    )?;
    let inks = super::create_media::parse_inks(
        obj.get("inks"),
        i,
        format!("presentation.slides[{i}].inks"),
        false,
    )?;
    let maths = super::create_media::parse_maths(
        obj.get("maths"),
        i,
        format!("presentation.slides[{i}].maths"),
        false,
    )?;
    let ole_objects = super::create_drawing::parse_ole_objects(obj.get("oleObjects"), i)?;
    let connectors = super::create_drawing::parse_connectors(obj.get("connectors"), i)?;
    let zooms = super::create_drawing::parse_zooms(obj.get("zooms"), i)?;
    Ok(NewSlide {
        shapes,
        pictures,
        videos,
        audios,
        charts,
        smart_arts,
        tables,
        notes,
        comments,
        transition,
        anims,
        model3ds,
        inks,
        maths,
        ole_objects,
        connectors,
        zooms,
    })
}

pub(crate) fn empty_slide() -> NewSlide {
    NewSlide {
        shapes: vec![NewShape {
            text: String::new(),
            link: None,
            format: None,
            geom: None,
            fields: Vec::new(),
            prst: None,
            x: None,
            y: None,
            cx: None,
            cy: None,
            fill: None,
            line: None,
            index: None,
        }],
        pictures: Vec::new(),
        videos: Vec::new(),
        audios: Vec::new(),
        charts: Vec::new(),
        smart_arts: Vec::new(),
        tables: Vec::new(),
        notes: None,
        comments: Vec::new(),
        transition: None,
        anims: Vec::new(),
        model3ds: Vec::new(),
        inks: Vec::new(),
        maths: Vec::new(),
        ole_objects: Vec::new(),
        connectors: Vec::new(),
        zooms: Vec::new(),
    }
}

pub(crate) fn parse_shape(v: &Value, slide: usize, i: usize) -> Result<NewShape, Error> {
    let sh = super::parse_create_shape(v, &format!("presentation.slides[{slide}].shapes[{i}]"))?;
    Ok(NewShape {
        text: sh.text,
        link: sh.link,
        format: sh.format,
        geom: sh.geom,
        fields: sh.fields,
        prst: sh.prst,
        x: sh.x,
        y: sh.y,
        cx: sh.cx,
        cy: sh.cy,
        fill: sh.fill,
        line: sh.line,
        index: sh.index,
    })
}
