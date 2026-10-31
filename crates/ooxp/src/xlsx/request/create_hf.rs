use serde_json::Value;

use crate::error::{Error, ErrorKind, TypeName};

use super::super::a1::CellRef;
use super::super::picture::AnchorKind;
use super::create_types::*;
use super::{
    HfSlot, InsertPictureOp,
};

pub(crate) fn parse_header_footers(
    v: Option<&Value>,
    sheet_i: usize,
    sheet: &str,
) -> Result<Vec<NewHfSlot>, Error> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    let prefix = format!("workbook.sheets[{sheet_i}].headerFooters");
    let arr = v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "headerFooters".into(),
                expected: TypeName::Array,
            },
            prefix.clone(),
        )
    })?;
    let mut out = Vec::new();
    let mut seen = [false; 6];
    for (j, item) in arr.iter().enumerate() {
        let item_path = format!("{prefix}[{j}]");
        let obj = item.as_object().ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: "headerFooters".into(),
                    expected: TypeName::Object,
                },
                item_path.clone(),
            )
        })?;
        for k in obj.keys() {
            if !matches!(k.as_str(), "kind" | "side" | "text" | "pictures") {
                return Err(Error::with_path(
                    ErrorKind::UnknownField { field: k.clone() },
                    format!("{item_path}.{k}"),
                ));
            }
        }
        let kind = match obj.get("kind") {
            Some(Value::String(s)) => s.as_str(),
            None => {
                return Err(Error::with_path(
                    ErrorKind::MissingField {
                        field: "kind".into(),
                    },
                    format!("{item_path}.kind"),
                ));
            }
            Some(_) => {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: "kind".into(),
                        expected: TypeName::String,
                    },
                    format!("{item_path}.kind"),
                ));
            }
        };
        let side = match obj.get("side") {
            Some(Value::String(s)) => s.as_str(),
            None => {
                return Err(Error::with_path(
                    ErrorKind::MissingField {
                        field: "side".into(),
                    },
                    format!("{item_path}.side"),
                ));
            }
            Some(_) => {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: "side".into(),
                        expected: TypeName::String,
                    },
                    format!("{item_path}.side"),
                ));
            }
        };
        if !matches!(kind, "odd" | "even" | "first") {
            return Err(Error::with_path(
                ErrorKind::InvalidXlsxHfKind,
                format!("{item_path}.kind"),
            ));
        }
        if !matches!(side, "header" | "footer") {
            return Err(Error::with_path(
                ErrorKind::InvalidXlsxHfSide,
                format!("{item_path}.side"),
            ));
        }
        let slot = HfSlot::parse(kind, side).expect("kind 与 side 已校验");
        let idx = slot.index();
        if seen[idx] {
            return Err(Error::with_path(
                ErrorKind::CreateXlsxHfDuplicate,
                item_path.clone(),
            ));
        }
        seen[idx] = true;
        let text = match obj.get("text") {
            None | Some(Value::Null) => None,
            Some(Value::String(s)) => Some(s.clone()),
            Some(_) => {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: "text".into(),
                        expected: TypeName::String,
                    },
                    format!("{item_path}.text"),
                ));
            }
        };
        let pictures = parse_hf_pictures(obj.get("pictures"), &item_path, sheet, slot)?;
        let has_text = text.as_ref().is_some_and(|s| !s.is_empty());
        if !has_text && pictures.is_empty() {
            return Err(Error::with_path(ErrorKind::NeedXlsxHfContent, item_path));
        }
        out.push(NewHfSlot {
            slot,
            text,
            pictures,
        });
    }
    Ok(out)
}

pub(crate) fn parse_hf_pictures(
    v: Option<&Value>,
    item_path: &str,
    sheet: &str,
    slot: HfSlot,
) -> Result<Vec<InsertPictureOp>, Error> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    let prefix = format!("{item_path}.pictures");
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
    for (k, item) in arr.iter().enumerate() {
        out.push(parse_hf_picture(
            item,
            &format!("{prefix}[{k}]"),
            sheet,
            slot,
        )?);
    }
    Ok(out)
}

pub(crate) fn parse_hf_picture(
    v: &Value,
    prefix: &str,
    sheet: &str,
    slot: HfSlot,
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
        if matches!(
            k.as_str(),
            "from" | "to" | "anchor" | "fromOffX" | "fromOffY" | "toOffX" | "toOffY"
        ) {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("{prefix}.{k}"),
            ));
        }
        if !matches!(k.as_str(), "file" | "cx" | "cy") {
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
    Ok(InsertPictureOp {
        sheet: sheet.to_string(),
        hf: Some(slot),
        file,
        cx,
        cy,
        from: CellRef { col: 1, row: 1 },
        to: None,
        anchor: AnchorKind::OneCell,
        from_off_x: 0,
        from_off_y: 0,
        to_off_x: 0,
        to_off_y: 0,
        rel: None,
    })
}
