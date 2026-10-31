use serde_json::Value;

use crate::error::{Error, ErrorKind, TypeName};

use super::{CreateSection, InsertCustomShowOp};

pub(crate) fn parse_sections(
    v: Option<&Value>,
    slide_count: usize,
) -> Result<Vec<CreateSection>, Error> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    let arr = v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "sections".into(),
                expected: TypeName::Array,
            },
            "presentation.sections",
        )
    })?;
    let mut out = Vec::new();
    let mut prev = 0usize;
    for (j, item) in arr.iter().enumerate() {
        let sec = super::parse_create_section(item, &format!("presentation.sections[{j}]"))?;
        if sec.start <= prev || sec.start > slide_count {
            return Err(Error::with_path(
                ErrorKind::OpRequiresField {
                    op: "create".into(),
                    field: "start".into(),
                },
                format!("presentation.sections[{j}].start"),
            ));
        }
        prev = sec.start;
        out.push(sec);
    }
    Ok(out)
}

pub(crate) fn parse_custom_shows(
    v: Option<&Value>,
    slide_count: usize,
) -> Result<Vec<InsertCustomShowOp>, Error> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    let arr = v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "customShows".into(),
                expected: TypeName::Array,
            },
            "presentation.customShows",
        )
    })?;
    let mut out = Vec::new();
    for (j, item) in arr.iter().enumerate() {
        let show =
            super::parse_create_custom_show(item, &format!("presentation.customShows[{j}]"))?;
        for (k, n) in show.slides.iter().enumerate() {
            if *n > slide_count {
                return Err(Error::with_path(
                    ErrorKind::OpRequiresField {
                        op: "create".into(),
                        field: "slides".into(),
                    },
                    format!("presentation.customShows[{j}].slides[{k}]"),
                ));
            }
        }
        out.push(show);
    }
    Ok(out)
}
