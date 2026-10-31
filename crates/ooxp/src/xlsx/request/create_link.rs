use serde_json::{Map, Value};

use crate::error::{Error, ErrorKind, TypeName};

use super::super::a1::{self};
use super::InsertHyperlinkOp;

pub(crate) fn parse_hyperlinks(
    v: Option<&Value>,
    sheet_i: usize,
    sheet: &str,
) -> Result<Vec<InsertHyperlinkOp>, Error> {
    let prefix = format!("workbook.sheets[{sheet_i}].hyperlinks");
    let Some(arr) = super::create_util::parse_object_array(v, "hyperlinks", &prefix)? else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for (j, item) in arr.iter().enumerate() {
        out.push(parse_hyperlink(item, &format!("{prefix}[{j}]"), sheet)?);
    }
    Ok(out)
}

pub(crate) fn parse_hyperlink(
    v: &Value,
    prefix: &str,
    sheet: &str,
) -> Result<InsertHyperlinkOp, Error> {
    let obj = super::create_util::require_object(v, "hyperlinks", prefix)?;
    super::create_util::deny_unknown(
        obj,
        prefix,
        &["ref", "url", "location", "tooltip", "display"],
    )?;
    let (refer, start, end) = match obj.get("ref") {
        Some(Value::String(s)) => {
            let (start, end) = a1::parse_range(s).map_err(|e| e.at(format!("{prefix}.ref")))?;
            (a1::format_range(start, end), start, end)
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "ref".into(),
                },
                format!("{prefix}.ref"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "ref".into(),
                    expected: TypeName::String,
                },
                format!("{prefix}.ref"),
            ));
        }
    };
    let url = parse_opt_target(obj, prefix, "url", ErrorKind::EmptyHyperlinkUrl)?;
    let location = parse_opt_target(obj, prefix, "location", ErrorKind::EmptyHyperlinkLocation)?;
    if url.is_none() && location.is_none() {
        return Err(Error::with_path(
            ErrorKind::XlsxHyperlinkNeedTarget,
            prefix.to_string(),
        ));
    }
    Ok(InsertHyperlinkOp {
        sheet: sheet.to_string(),
        refer,
        start,
        end,
        url,
        location,
        tooltip: parse_opt_plain(obj, prefix, "tooltip")?,
        display: parse_opt_plain(obj, prefix, "display")?,
        drawing: None,
    })
}

pub(crate) fn parse_opt_target(
    obj: &Map<String, Value>,
    prefix: &str,
    field: &str,
    empty: ErrorKind,
) -> Result<Option<String>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::String(s)) if s.is_empty() => {
            Err(Error::with_path(empty, format!("{prefix}.{field}")))
        }
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::String,
            },
            format!("{prefix}.{field}"),
        )),
    }
}

pub(crate) fn parse_opt_plain(
    obj: &Map<String, Value>,
    prefix: &str,
    field: &str,
) -> Result<Option<String>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::String,
            },
            format!("{prefix}.{field}"),
        )),
    }
}
