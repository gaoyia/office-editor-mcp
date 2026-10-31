use serde_json::Value;

use crate::error::{Error, ErrorKind, TypeName};

use super::super::a1::{self};
use super::super::ink;
use super::super::model3d;
use super::super::omml;
use super::super::picture::{self, AnchorKind};

pub(crate) fn parse_model3ds(
    v: Option<&Value>,
    sheet_i: usize,
    sheet: &str,
) -> Result<Vec<model3d::InsertModel3dOp>, Error> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    let prefix = format!("workbook.sheets[{sheet_i}].model3ds");
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
        out.push(parse_create_model3d(
            item,
            &format!("{prefix}[{j}]"),
            sheet,
        )?);
    }
    Ok(out)
}

pub(crate) fn parse_inks(
    v: Option<&Value>,
    sheet_i: usize,
    sheet: &str,
) -> Result<Vec<ink::InsertInkOp>, Error> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    let prefix = format!("workbook.sheets[{sheet_i}].inks");
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
        out.push(ink::parse_create(item, &format!("{prefix}[{j}]"), sheet)?);
    }
    Ok(out)
}

pub(crate) fn parse_maths(
    v: Option<&Value>,
    sheet_i: usize,
    sheet: &str,
) -> Result<Vec<omml::InsertMathOp>, Error> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    let prefix = format!("workbook.sheets[{sheet_i}].maths");
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
        out.push(omml::parse_create(item, &format!("{prefix}[{j}]"), sheet)?);
    }
    Ok(out)
}

pub(crate) fn parse_create_model3d(
    v: &Value,
    prefix: &str,
    sheet: &str,
) -> Result<model3d::InsertModel3dOp, Error> {
    let obj = v.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: prefix.into(),
                expected: TypeName::Object,
            },
            prefix,
        )
    })?;
    crate::json::deny_unknown(
        obj,
        &[
            "file", "cx", "cy", "from", "to", "anchor", "fromOffX", "fromOffY", "toOffX", "toOffY",
        ],
    )?;
    let file = match obj.get("file") {
        Some(Value::String(s)) if !s.is_empty() => s.clone(),
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "file".into(),
                    expected: TypeName::NonEmptyString,
                },
                format!("{prefix}.file"),
            ));
        }
        None => {
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
                    expected: TypeName::String,
                },
                format!("{prefix}.file"),
            ));
        }
    };
    let cx = super::create_util::parse_emu(obj.get("cx"), prefix, "cx")?;
    let cy = super::create_util::parse_emu(obj.get("cy"), prefix, "cy")?;
    if cx.is_some() != cy.is_some() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "insertModel3d".into(),
                field: "cx and cy together".into(),
            },
            prefix.to_string(),
        ));
    }
    let from = match obj.get("from") {
        Some(Value::String(s)) => a1::parse_cell(s).map_err(|e| e.at(format!("{prefix}.from")))?,
        None => a1::CellRef { col: 1, row: 1 },
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
    let anchor = match obj.get("anchor") {
        None => AnchorKind::OneCell,
        Some(Value::String(s)) => picture::AnchorKind::parse_insert(s).ok_or_else(|| {
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
            ErrorKind::OpRequiresField {
                op: "insertModel3d".into(),
                field: "to".into(),
            },
            format!("{prefix}.to"),
        ));
    }
    if anchor == AnchorKind::OneCell && to.is_some() {
        return Err(Error::with_path(
            ErrorKind::UnknownField { field: "to".into() },
            format!("{prefix}.to"),
        ));
    }
    Ok(model3d::InsertModel3dOp {
        sheet: sheet.to_string(),
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
    })
}
