use serde_json::Value;

use crate::error::{Error, ErrorKind, TypeName};

use super::create_types::*;
use super::{CreateAnim, CreateAnimHost, SetTransitionOp};

pub(crate) fn parse_transition(
    v: Option<&Value>,
    slide_i: usize,
) -> Result<Option<SetTransitionOp>, Error> {
    let Some(v) = v else {
        return Ok(None);
    };
    Ok(Some(super::parse_create_transition(
        v,
        &format!("presentation.slides[{slide_i}].transition"),
        slide_i + 1,
    )?))
}

pub(crate) fn parse_anims(v: Option<&Value>, slide_i: usize) -> Result<Vec<CreateAnim>, Error> {
    parse_anims_at(v, &format!("presentation.slides[{slide_i}].anims"))
}

pub(crate) fn parse_anims_at(v: Option<&Value>, prefix: &str) -> Result<Vec<CreateAnim>, Error> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    let arr = v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "anims".into(),
                expected: TypeName::Array,
            },
            prefix.to_string(),
        )
    })?;
    let mut out = Vec::new();
    for (j, item) in arr.iter().enumerate() {
        out.push(super::parse_create_anim(item, &format!("{prefix}[{j}]"))?);
    }
    Ok(out)
}

pub(crate) fn validate_anims(slides: &[NewSlide]) -> Result<(), Error> {
    for (i, slide) in slides.iter().enumerate() {
        for (j, anim) in slide.anims.iter().enumerate() {
            let (n, len, field) = match anim.host {
                CreateAnimHost::Shape(n) => (n, slide.shapes.len(), "shape"),
                CreateAnimHost::Picture(n) => (n, slide.pictures.len(), "picture"),
                CreateAnimHost::Video(n) => (n, slide.videos.len(), "video"),
                CreateAnimHost::Audio(n) => (n, slide.audios.len(), "audio"),
                CreateAnimHost::Table(n) => (n, slide.tables.len(), "table"),
                CreateAnimHost::Chart(n) => (n, slide.charts.len(), "chart"),
                CreateAnimHost::SmartArt(n) => (n, slide.smart_arts.len(), "smartArt"),
            };
            if n < 1 || n > len {
                return Err(Error::with_path(
                    ErrorKind::OpRequiresField {
                        op: "create".into(),
                        field: field.into(),
                    },
                    format!("presentation.slides[{i}].anims[{j}].{field}"),
                ));
            }
        }
    }
    Ok(())
}
