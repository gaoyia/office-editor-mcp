use serde_json::Value;

use crate::error::{Error, ErrorKind, TypeName};

use super::super::a1::{self};
use super::super::picture::AnchorKind;
use super::InsertPictureOp;

pub(crate) fn parse_pictures(
    v: Option<&Value>,
    sheet_i: usize,
    sheet: &str,
) -> Result<Vec<InsertPictureOp>, Error> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    let prefix = format!("workbook.sheets[{sheet_i}].pictures");
    let arr = v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "pictures".into(),
                expected: TypeName::Array,
            },
            prefix.clone(),
        )
    })?;
    let mut out = Vec::new();
    for (j, item) in arr.iter().enumerate() {
        out.push(parse_picture(item, &format!("{prefix}[{j}]"), sheet)?);
    }
    Ok(out)
}

pub(crate) fn parse_picture(
    v: &Value,
    prefix: &str,
    sheet: &str,
) -> Result<InsertPictureOp, Error> {
    let obj = v.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "pictures".into(),
                expected: TypeName::Object,
            },
            prefix.to_string(),
        )
    })?;
    for k in obj.keys() {
        if !matches!(
            k.as_str(),
            "file"
                | "from"
                | "to"
                | "anchor"
                | "cx"
                | "cy"
                | "fromOffX"
                | "fromOffY"
                | "toOffX"
                | "toOffY"
        ) {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("{prefix}.{k}"),
            ));
        }
    }
    let file = match obj.get("file") {
        Some(Value::String(s)) if !s.is_empty() => s.clone(),
        Some(Value::String(_)) | None => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "file".into(),
                },
                format!("{prefix}.file"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "file".into(),
                    expected: TypeName::NonEmptyString,
                },
                format!("{prefix}.file"),
            ));
        }
    };
    let from = match obj.get("from") {
        Some(Value::String(s)) => a1::parse_cell(s).map_err(|e| e.at(format!("{prefix}.from")))?,
        None => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "from".into(),
                },
                format!("{prefix}.from"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "from".into(),
                    expected: TypeName::String,
                },
                format!("{prefix}.from"),
            ));
        }
    };
    let cx = super::create_util::parse_emu(obj.get("cx"), prefix, "cx")?;
    let cy = super::create_util::parse_emu(obj.get("cy"), prefix, "cy")?;
    if cx.is_some() != cy.is_some() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "create".into(),
                field: "cx and cy together".into(),
            },
            prefix.to_string(),
        ));
    }
    let anchor = match obj.get("anchor") {
        None => AnchorKind::OneCell,
        Some(Value::String(s)) => AnchorKind::parse_insert(s).ok_or_else(|| {
            Error::with_path(ErrorKind::InvalidXlsxAnchor, format!("{prefix}.anchor"))
        })?,
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "anchor".into(),
                    expected: TypeName::String,
                },
                format!("{prefix}.anchor"),
            ));
        }
    };
    let to = match obj.get("to") {
        None => None,
        Some(Value::String(s)) => {
            Some(a1::parse_cell(s).map_err(|e| e.at(format!("{prefix}.to")))?)
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "to".into(),
                    expected: TypeName::String,
                },
                format!("{prefix}.to"),
            ));
        }
    };
    if anchor == AnchorKind::TwoCell && to.is_none() {
        return Err(Error::with_path(
            ErrorKind::MissingField { field: "to".into() },
            format!("{prefix}.to"),
        ));
    }
    if anchor == AnchorKind::OneCell && to.is_some() {
        return Err(Error::with_path(
            ErrorKind::UnknownField { field: "to".into() },
            format!("{prefix}.to"),
        ));
    }
    Ok(InsertPictureOp {
        sheet: sheet.to_string(),
        hf: None,
        file,
        cx,
        cy,
        from,
        to,
        anchor,
        from_off_x: super::create_util::parse_off(obj.get("fromOffX"), prefix, "fromOffX")?
            .unwrap_or(0),
        from_off_y: super::create_util::parse_off(obj.get("fromOffY"), prefix, "fromOffY")?
            .unwrap_or(0),
        to_off_x: super::create_util::parse_off(obj.get("toOffX"), prefix, "toOffX")?.unwrap_or(0),
        to_off_y: super::create_util::parse_off(obj.get("toOffY"), prefix, "toOffY")?.unwrap_or(0),
        rel: None,
    })
}
