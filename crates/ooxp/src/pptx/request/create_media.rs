use serde_json::Value;

use crate::error::{Error, ErrorKind, TypeName};

use super::InsertMediaOp;

pub(crate) fn parse_model3ds(
    v: Option<&Value>,
    slide_i: usize,
    prefix: String,
    on_notes: bool,
) -> Result<Vec<super::super::model3d::InsertModel3dOp>, Error> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    let arr = v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "model3ds".into(),
                expected: TypeName::Array,
            },
            prefix.clone(),
        )
    })?;
    let mut out = Vec::new();
    for (j, item) in arr.iter().enumerate() {
        out.push(super::super::model3d::parse_create(
            item,
            &format!("{prefix}[{j}]"),
            slide_i + 1,
            on_notes,
        )?);
    }
    Ok(out)
}

pub(crate) fn parse_inks(
    v: Option<&Value>,
    slide_i: usize,
    prefix: String,
    on_notes: bool,
) -> Result<Vec<super::super::ink::InsertInkOp>, Error> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    let arr = v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "inks".into(),
                expected: TypeName::Array,
            },
            prefix.clone(),
        )
    })?;
    let mut out = Vec::new();
    for (j, item) in arr.iter().enumerate() {
        out.push(super::super::ink::parse_create(
            item,
            &format!("{prefix}[{j}]"),
            slide_i + 1,
            on_notes,
        )?);
    }
    Ok(out)
}

pub(crate) fn parse_maths(
    v: Option<&Value>,
    slide_i: usize,
    prefix: String,
    on_notes: bool,
) -> Result<Vec<super::super::omml::InsertMathOp>, Error> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    let arr = v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "maths".into(),
                expected: TypeName::Array,
            },
            prefix.clone(),
        )
    })?;
    let mut out = Vec::new();
    for (j, item) in arr.iter().enumerate() {
        out.push(super::super::omml::parse_create(
            item,
            &format!("{prefix}[{j}]"),
            slide_i + 1,
            on_notes,
        )?);
    }
    Ok(out)
}

pub(crate) fn parse_media(
    v: Option<&Value>,
    slide_i: usize,
    field: &str,
) -> Result<Vec<InsertMediaOp>, Error> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    let prefix = format!("presentation.slides[{slide_i}].{field}");
    let arr = v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Array,
            },
            prefix.clone(),
        )
    })?;
    let mut out = Vec::new();
    for (j, item) in arr.iter().enumerate() {
        out.push(super::parse_create_media(
            item,
            &format!("{prefix}[{j}]"),
            slide_i + 1,
        )?);
    }
    Ok(out)
}
