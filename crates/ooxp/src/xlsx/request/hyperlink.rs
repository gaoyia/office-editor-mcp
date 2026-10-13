//! xlsx request：hyperlink。

use serde_json::Value;

use crate::error::{Error, ErrorKind, IdReason, TypeName};

use super::super::a1;
use super::super::picture::{self};
use super::super::shape::{self};
use super::super::smartart as xlsx_smartart;

use super::common::*;
use super::types::*;

pub(super) fn parse_insert_hyperlink(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertHyperlinkOp, Error> {
    deny_keys(
        obj,
        i,
        &["op", "id", "ref", "url", "location", "tooltip", "display"],
    )?;
    let id = require_id(obj, i)?;
    let url = parse_opt_target(obj, i, "url", ErrorKind::EmptyHyperlinkUrl)?;
    let location = parse_opt_target(obj, i, "location", ErrorKind::EmptyHyperlinkLocation)?;
    let tooltip = parse_opt_plain(obj, i, "tooltip")?;
    let display = parse_opt_plain(obj, i, "display")?;
    if let Some(drawing) = parse_drawing_hlink_host(&id, i)? {
        if obj.contains_key("ref") {
            return Err(Error::with_path(
                ErrorKind::UnknownField {
                    field: "ref".into(),
                },
                format!("ops[{i}].ref"),
            ));
        }
        if display.is_some() {
            return Err(Error::with_path(
                ErrorKind::UnknownField {
                    field: "display".into(),
                },
                format!("ops[{i}].display"),
            ));
        }
        match (url.is_some(), location.is_some()) {
            (true, false) | (false, true) => {}
            (true, true) => {
                return Err(Error::with_path(
                    ErrorKind::MutuallyExclusive {
                        a: "url".into(),
                        b: "location".into(),
                    },
                    format!("ops[{i}]"),
                ));
            }
            (false, false) => {
                return Err(Error::with_path(
                    ErrorKind::XlsxHyperlinkNeedTarget,
                    format!("ops[{i}]"),
                ));
            }
        }
        let sheet = drawing.sheet().to_string();
        return Ok(InsertHyperlinkOp {
            sheet,
            refer: String::new(),
            start: a1::CellRef { col: 1, row: 1 },
            end: a1::CellRef { col: 1, row: 1 },
            url,
            location,
            tooltip,
            display: None,
            drawing: Some(drawing),
        });
    }
    let sheet = parse_sheet_only_id(&id, i, "id", IdReason::InsertHyperlinkNeedsSheet)?;
    let (refer, start, end) = parse_required_range(obj, i)?;
    if url.is_none() && location.is_none() {
        return Err(Error::with_path(
            ErrorKind::XlsxHyperlinkNeedTarget,
            format!("ops[{i}]"),
        ));
    }
    Ok(InsertHyperlinkOp {
        sheet,
        refer,
        start,
        end,
        url,
        location,
        tooltip,
        display,
        drawing: None,
    })
}

pub(super) fn parse_drawing_hlink_host(
    id: &str,
    i: usize,
) -> Result<Option<DrawingHlinkHost>, Error> {
    if id.contains("/pic:") {
        return Ok(Some(DrawingHlinkHost::Picture(picture::parse_pic_id(
            id, i,
        )?)));
    }
    if id.contains("/smartdiagram:") {
        return Ok(Some(DrawingHlinkHost::SmartArt(
            xlsx_smartart::parse_diagram_id(id, i)?,
        )));
    }
    if id.contains("/shape:") {
        return Ok(Some(DrawingHlinkHost::Shape(shape::parse_shape_id(id, i)?)));
    }
    if id.contains("/sp:") {
        return Ok(Some(DrawingHlinkHost::TextBox(shape::parse_sp_id(id, i)?)));
    }
    Ok(None)
}

pub(super) fn parse_set_hyperlink(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetHyperlinkOp, Error> {
    deny_keys(
        obj,
        i,
        &["op", "id", "url", "location", "tooltip", "display", "ref"],
    )?;
    let id = require_id(obj, i)?;
    let url = parse_hyperlink_patch(obj, i, "url", Some(ErrorKind::EmptyHyperlinkUrl))?;
    let location =
        parse_hyperlink_patch(obj, i, "location", Some(ErrorKind::EmptyHyperlinkLocation))?;
    let tooltip = parse_hyperlink_patch(obj, i, "tooltip", None)?;
    let display = parse_hyperlink_patch(obj, i, "display", None)?;
    if let Some(drawing) = parse_drawing_hlink_host(&id, i)? {
        if obj.contains_key("ref") {
            return Err(Error::with_path(
                ErrorKind::UnknownField {
                    field: "ref".into(),
                },
                format!("ops[{i}].ref"),
            ));
        }
        if !matches!(display, Patch::Absent) {
            return Err(Error::with_path(
                ErrorKind::UnknownField {
                    field: "display".into(),
                },
                format!("ops[{i}].display"),
            ));
        }
        if matches!(url, Patch::Set(_)) && matches!(location, Patch::Set(_)) {
            return Err(Error::with_path(
                ErrorKind::MutuallyExclusive {
                    a: "url".into(),
                    b: "location".into(),
                },
                format!("ops[{i}]"),
            ));
        }
        if url.is_absent() && location.is_absent() && tooltip.is_absent() {
            return Err(Error::with_path(
                ErrorKind::NeedXlsxSetHyperlinkField,
                format!("ops[{i}]"),
            ));
        }
        let sheet = drawing.sheet().to_string();
        return Ok(SetHyperlinkOp {
            id,
            sheet,
            index: 0,
            url,
            location,
            tooltip,
            display: Patch::Absent,
            refer: None,
            drawing: Some(drawing),
        });
    }
    let (sheet, index) = parse_link_id(&id, i)?;
    let refer = parse_opt_range(obj, i)?;
    if url.is_absent()
        && location.is_absent()
        && tooltip.is_absent()
        && display.is_absent()
        && refer.is_none()
    {
        return Err(Error::with_path(
            ErrorKind::NeedXlsxSetHyperlinkField,
            format!("ops[{i}]"),
        ));
    }
    Ok(SetHyperlinkOp {
        id,
        sheet,
        index,
        url,
        location,
        tooltip,
        display,
        refer,
        drawing: None,
    })
}

pub(super) fn parse_delete_hyperlink(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteHyperlinkOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    if let Some(drawing) = parse_drawing_hlink_host(&id, i)? {
        let sheet = drawing.sheet().to_string();
        return Ok(DeleteHyperlinkOp {
            id,
            sheet,
            index: 0,
            drawing: Some(drawing),
        });
    }
    let (sheet, index) = parse_link_id(&id, i)?;
    Ok(DeleteHyperlinkOp {
        id,
        sheet,
        index,
        drawing: None,
    })
}

pub(super) fn parse_link_id(id: &str, op_index: usize) -> Result<(String, usize), Error> {
    let Some(rest) = id.strip_prefix("sheet:") else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SheetLinkPrefix,
            },
            format!("ops[{op_index}].id"),
        ));
    };
    let Some((name, nstr)) = rest.split_once("/link:") else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SheetLinkPrefix,
            },
            format!("ops[{op_index}].id"),
        ));
    };
    if name.is_empty() || name.contains('/') {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SheetLinkPrefix,
            },
            format!("ops[{op_index}].id"),
        ));
    }
    if nstr.is_empty() || !nstr.bytes().all(|b| b.is_ascii_digit()) {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::LinkIndexNotInt,
            },
            format!("ops[{op_index}].id"),
        ));
    }
    let n: usize = nstr.parse().map_err(|_| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::LinkIndexNotInt,
            },
            format!("ops[{op_index}].id"),
        )
    })?;
    if n == 0 {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::LinkIndexFromOne,
            },
            format!("ops[{op_index}].id"),
        ));
    }
    Ok((name.to_string(), n))
}

pub(super) fn formula_is_block(raw: &str) -> bool {
    let t = raw.trim();
    let a1 = match t.rfind('!') {
        Some(i) => &t[i + 1..],
        None => t,
    };
    match a1::parse_range(a1) {
        Ok((a, b)) => a.row != b.row && a.col != b.col,
        Err(_) => false,
    }
}

pub(super) fn parse_opt_f64_top(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    field: &str,
) -> Result<Option<f64>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::Number(n)) => {
            let Some(v) = n.as_f64().filter(|x| x.is_finite()) else {
                return Err(Error::with_path(
                    ErrorKind::InvalidSparkline {
                        field: field.into(),
                    },
                    format!("ops[{i}].{field}"),
                ));
            };
            Ok(Some(v))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Integer,
            },
            format!("ops[{i}].{field}"),
        )),
    }
}

pub(super) fn parse_f64_patch(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    field: &str,
) -> Result<Patch<f64>, Error> {
    match obj.get(field) {
        None => Ok(Patch::Absent),
        Some(Value::Null) => Ok(Patch::Clear),
        Some(Value::Number(n)) => {
            let Some(v) = n.as_f64().filter(|x| x.is_finite()) else {
                return Err(Error::with_path(
                    ErrorKind::InvalidSparkline {
                        field: field.into(),
                    },
                    format!("ops[{i}].{field}"),
                ));
            };
            Ok(Patch::Set(v))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Integer,
            },
            format!("ops[{i}].{field}"),
        )),
    }
}

pub(super) fn parse_required_range(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<(String, a1::CellRef, a1::CellRef), Error> {
    match obj.get("ref") {
        Some(Value::String(s)) => {
            let (start, end) = a1::parse_range(s).map_err(|e| e.at(format!("ops[{i}].ref")))?;
            Ok((a1::format_range(start, end), start, end))
        }
        None => Err(Error::with_path(
            ErrorKind::MissingField {
                field: "ref".into(),
            },
            format!("ops[{i}].ref"),
        )),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "ref".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].ref"),
        )),
    }
}

pub(super) fn parse_opt_range(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<Option<(String, a1::CellRef, a1::CellRef)>, Error> {
    match obj.get("ref") {
        None => Ok(None),
        Some(Value::String(s)) => {
            let (start, end) = a1::parse_range(s).map_err(|e| e.at(format!("ops[{i}].ref")))?;
            Ok(Some((a1::format_range(start, end), start, end)))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "ref".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].ref"),
        )),
    }
}

pub(super) fn parse_opt_target(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    field: &str,
    empty: ErrorKind,
) -> Result<Option<String>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::String(s)) if s.is_empty() => {
            Err(Error::with_path(empty, format!("ops[{i}].{field}")))
        }
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].{field}"),
        )),
    }
}

pub(super) fn parse_opt_plain(
    obj: &serde_json::Map<String, Value>,
    i: usize,
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
            format!("ops[{i}].{field}"),
        )),
    }
}

pub(super) fn parse_hyperlink_patch(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    field: &str,
    empty: Option<ErrorKind>,
) -> Result<Patch<String>, Error> {
    match obj.get(field) {
        None => Ok(Patch::Absent),
        Some(Value::Null) => Ok(Patch::Clear),
        Some(Value::String(s)) if s.is_empty() => {
            if let Some(kind) = empty {
                Err(Error::with_path(kind, format!("ops[{i}].{field}")))
            } else {
                Ok(Patch::Set(String::new()))
            }
        }
        Some(Value::String(s)) => Ok(Patch::Set(s.clone())),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].{field}"),
        )),
    }
}
