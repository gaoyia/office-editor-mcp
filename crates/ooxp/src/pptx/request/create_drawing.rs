use serde_json::Value;

use crate::error::{Error, ErrorKind, TypeName};

use super::super::ole;
use super::super::zoom;
use super::create_types::*;
use super::{
    CreateConnector,
    CreateSmartArt, CreateTable,
};

pub(crate) fn parse_smartarts(
    v: Option<&Value>,
    slide_i: usize,
) -> Result<Vec<CreateSmartArt>, Error> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    let prefix = format!("presentation.slides[{slide_i}].smartArts");
    let arr = v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "smartArts".into(),
                expected: TypeName::Array,
            },
            prefix.clone(),
        )
    })?;
    let mut out = Vec::new();
    for (j, item) in arr.iter().enumerate() {
        out.push(super::parse_create_smartart(
            item,
            &format!("{prefix}[{j}]"),
            slide_i + 1,
        )?);
    }
    Ok(out)
}

pub(crate) fn parse_tables(v: Option<&Value>, slide_i: usize) -> Result<Vec<CreateTable>, Error> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    let prefix = format!("presentation.slides[{slide_i}].tables");
    let arr = v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "tables".into(),
                expected: TypeName::Array,
            },
            prefix.clone(),
        )
    })?;
    let mut out = Vec::new();
    for (j, item) in arr.iter().enumerate() {
        out.push(super::parse_create_table(
            item,
            &format!("{prefix}[{j}]"),
            slide_i + 1,
        )?);
    }
    Ok(out)
}

pub(crate) fn parse_ole_objects(
    v: Option<&Value>,
    slide_i: usize,
) -> Result<Vec<ole::InsertOleOp>, Error> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    let prefix = format!("presentation.slides[{slide_i}].oleObjects");
    let arr = v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "oleObjects".into(),
                expected: TypeName::Array,
            },
            prefix.clone(),
        )
    })?;
    let mut out = Vec::new();
    for (j, item) in arr.iter().enumerate() {
        out.push(super::parse_create_ole(
            item,
            &format!("{prefix}[{j}]"),
            slide_i + 1,
        )?);
    }
    Ok(out)
}

pub(crate) fn parse_connectors(
    v: Option<&Value>,
    slide_i: usize,
) -> Result<Vec<CreateConnector>, Error> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    let prefix = format!("presentation.slides[{slide_i}].connectors");
    let arr = v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "connectors".into(),
                expected: TypeName::Array,
            },
            prefix.clone(),
        )
    })?;
    let mut out = Vec::new();
    for (j, item) in arr.iter().enumerate() {
        out.push(super::parse_create_connector(
            item,
            &format!("{prefix}[{j}]"),
        )?);
    }
    Ok(out)
}

pub(crate) fn parse_zooms(
    v: Option<&Value>,
    slide_i: usize,
) -> Result<Vec<zoom::InsertZoomOp>, Error> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    let prefix = format!("presentation.slides[{slide_i}].zooms");
    let arr = v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "zooms".into(),
                expected: TypeName::Array,
            },
            prefix.clone(),
        )
    })?;
    let mut out = Vec::new();
    for (j, item) in arr.iter().enumerate() {
        out.push(super::parse_create_zoom(
            item,
            &format!("{prefix}[{j}]"),
            slide_i + 1,
        )?);
    }
    Ok(out)
}

pub(crate) fn validate_zooms(slides: &[NewSlide], section_count: usize) -> Result<(), Error> {
    for (i, slide) in slides.iter().enumerate() {
        for (j, op) in slide.zooms.iter().enumerate() {
            let prefix = format!("presentation.slides[{i}].zooms[{j}]");
            match &op.kind {
                zoom::InsertKind::Slide { target } => {
                    if *target < 1 || *target > slides.len() {
                        return Err(Error::with_path(
                            ErrorKind::OpRequiresField {
                                op: "create".into(),
                                field: "target".into(),
                            },
                            format!("{prefix}.target"),
                        ));
                    }
                }
                zoom::InsertKind::Section { target } => {
                    if *target < 1 || *target > section_count {
                        return Err(Error::with_path(
                            ErrorKind::OpRequiresField {
                                op: "create".into(),
                                field: "target".into(),
                            },
                            format!("{prefix}.target"),
                        ));
                    }
                }
                zoom::InsertKind::Summary { targets } => {
                    for (k, n) in targets.iter().enumerate() {
                        if *n < 1 || *n > section_count {
                            return Err(Error::with_path(
                                ErrorKind::OpRequiresField {
                                    op: "create".into(),
                                    field: "targets".into(),
                                },
                                format!("{prefix}.targets[{k}]"),
                            ));
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
