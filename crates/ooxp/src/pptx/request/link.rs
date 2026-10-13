//! pptx request：link。

use serde_json::{Map, Value};

use crate::error::{Error, ErrorKind, IdReason, TypeName};

use super::super::chart;
use super::super::field;
use super::super::hyperlink;
use super::super::picture;
use super::super::shape;
use super::super::smartart;
use super::super::table;

use super::common::*;
use super::types::*;

pub(super) fn parse_insert_hyperlink(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<InsertHyperlinkOp, Error> {
    let on = parse_pptx_link_on(obj, i)?;
    let event = parse_pptx_link_event(obj, i)?;
    let run_like = on.is_some_and(LinkOn::is_run_like);
    let mut allowed = vec!["op", "id", "url", "file", "slide", "tooltip", "on", "event"];
    if run_like || on.is_none() {
        allowed.push("text");
    }
    deny_keys(obj, i, &allowed)?;
    let id = require_id(obj, i)?;
    let host = parse_insert_hyperlink_host(&id, i)?;
    let on = match on {
        Some(v) => v,
        None => host.default_on(),
    };
    if !host.on_ok(on) {
        return Err(Error::with_path(
            ErrorKind::InvalidHyperlinkOn,
            format!("ops[{i}].on"),
        ));
    }
    if obj.get("text").is_some() && !on.is_run_like() {
        return Err(Error::with_path(
            ErrorKind::UnknownField {
                field: "text".into(),
            },
            format!("ops[{i}].text"),
        ));
    }
    let target = parse_pptx_link_target(obj, i)?
        .ok_or_else(|| Error::with_path(ErrorKind::PptxHyperlinkNeedTarget, format!("ops[{i}]")))?;
    Ok(InsertHyperlinkOp {
        id,
        host,
        target,
        tooltip: parse_optional_tooltip(obj, i)?,
        on,
        event,
        text: if on.is_run_like() {
            parse_optional_run_text(obj, i)?
        } else {
            None
        },
    })
}

pub(super) fn parse_pptx_link_on(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<Option<LinkOn>, Error> {
    match obj.get("on") {
        None => Ok(None),
        Some(Value::String(s)) if s == "shape" => Ok(Some(LinkOn::Shape)),
        Some(Value::String(s)) if s == "run" => Ok(Some(LinkOn::Run)),
        Some(Value::String(s)) if s == "pic" => Ok(Some(LinkOn::Pic)),
        Some(Value::String(s)) if s == "table" => Ok(Some(LinkOn::Table)),
        Some(Value::String(s)) if s == "chart" => Ok(Some(LinkOn::Chart)),
        Some(Value::String(s)) if s == "smartArt" => Ok(Some(LinkOn::SmartArt)),
        Some(Value::String(s)) if s == "cell" => Ok(Some(LinkOn::Cell)),
        Some(Value::String(_)) => Err(Error::with_path(
            ErrorKind::InvalidHyperlinkOn,
            format!("ops[{i}].on"),
        )),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "on".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].on"),
        )),
    }
}

pub(super) fn parse_pptx_link_event(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<LinkEvent, Error> {
    match obj.get("event") {
        None => Ok(LinkEvent::Click),
        Some(Value::String(s)) if s == "click" => Ok(LinkEvent::Click),
        Some(Value::String(s)) if s == "hover" => Ok(LinkEvent::Hover),
        Some(Value::String(_)) => Err(Error::with_path(
            ErrorKind::InvalidHyperlinkEvent,
            format!("ops[{i}].event"),
        )),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "event".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].event"),
        )),
    }
}

pub(super) fn parse_optional_run_text(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<Option<String>, Error> {
    match obj.get("text") {
        None => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "text".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].text"),
        )),
    }
}

pub(super) fn parse_set_hyperlink(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<SetHyperlinkOp, Error> {
    deny_keys(obj, i, &["op", "id", "url", "file", "slide", "tooltip"])?;
    let id = require_id(obj, i)?;
    let (host, index) = hyperlink::parse_link_id(&id, i)?;
    let (slide, on_notes, master) = host.parts();
    let target = parse_pptx_link_target(obj, i)?;
    let tooltip = parse_set_tooltip(obj, i)?;
    if target.is_none() && tooltip.is_none() {
        return Err(Error::with_path(
            ErrorKind::PptxNeedSetHyperlinkField,
            format!("ops[{i}]"),
        ));
    }
    Ok(SetHyperlinkOp {
        id,
        slide,
        on_notes,
        master,
        index,
        target,
        tooltip,
    })
}

pub(super) fn parse_delete_hyperlink(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<DeleteHyperlinkOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let (host, index) = hyperlink::parse_link_id(&id, i)?;
    let (slide, on_notes, master) = host.parts();
    Ok(DeleteHyperlinkOp {
        id,
        slide,
        on_notes,
        master,
        index,
    })
}

pub(super) fn parse_insert_field(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<field::InsertFieldOp, Error> {
    deny_keys(obj, i, &["op", "id", "type", "text"])?;
    let id = require_id(obj, i)?;
    let shape = parse_insert_field_shape(&id, i)?;
    let type_name = require_fld_type(obj, i)?;
    let text = match obj.get("text") {
        None => String::new(),
        Some(Value::String(s)) => s.clone(),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "text".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].text"),
            ));
        }
    };
    Ok(field::InsertFieldOp {
        id,
        shape,
        type_name,
        text,
    })
}

pub(super) fn parse_set_field(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<field::SetFieldOp, Error> {
    deny_keys(obj, i, &["op", "id", "type", "text"])?;
    let id = require_id(obj, i)?;
    let (host, key) = field::parse_field_id(&id, i)?;
    let (slide, on_notes, master) = host.parts();
    let type_name = match obj.get("type") {
        None => None,
        Some(Value::String(s)) => {
            if !field::is_reserved_type(s) {
                return Err(Error::with_path(
                    ErrorKind::InvalidFldType,
                    format!("ops[{i}].type"),
                ));
            }
            Some(s.clone())
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "type".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].type"),
            ));
        }
    };
    let text = match obj.get("text") {
        None => None,
        Some(Value::String(s)) => Some(s.clone()),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "text".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].text"),
            ));
        }
    };
    if type_name.is_none() && text.is_none() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setField".into(),
                field: "type".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(field::SetFieldOp {
        id,
        slide,
        on_notes,
        master,
        key,
        type_name,
        text,
    })
}

pub(super) fn parse_delete_field(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<field::DeleteFieldOp, Error> {
    deny_keys(obj, i, &["op", "id", "keep"])?;
    let id = require_id(obj, i)?;
    let (host, key) = field::parse_field_id(&id, i)?;
    let (slide, on_notes, master) = host.parts();
    let keep = match obj.get("keep") {
        None => field::FieldKeep::Result,
        Some(Value::String(s)) if s == "result" => field::FieldKeep::Result,
        Some(Value::String(s)) if s == "instr" => field::FieldKeep::Instr,
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidFieldKeep,
                format!("ops[{i}].keep"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "keep".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].keep"),
            ));
        }
    };
    Ok(field::DeleteFieldOp {
        id,
        slide,
        on_notes,
        master,
        key,
        keep,
    })
}

pub(super) fn parse_insert_hyperlink_host(id: &str, i: usize) -> Result<LinkInsertHost, Error> {
    let path = format!("ops[{i}].id");
    if id.starts_with("notesMaster") || id.starts_with("handout") {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::InsertHyperlinkNeedsShape,
            },
            path,
        ));
    }
    if id.contains("/r:") {
        let cell = table::parse_cell_id(id, &path).map_err(|_| {
            Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::InsertHyperlinkNeedsShape,
                },
                path.clone(),
            )
        })?;
        return Ok(LinkInsertHost::Cell(cell));
    }
    if id.contains("/pic:") {
        let pic = picture::parse_pic_id(id, &path).map_err(|_| {
            Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::InsertHyperlinkNeedsShape,
                },
                path.clone(),
            )
        })?;
        if pic.comment.is_some() {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::InsertHyperlinkNeedsShape,
                },
                path,
            ));
        }
        return Ok(LinkInsertHost::Pic(pic));
    }
    if id.contains("/table:") {
        let table = table::parse_table_id(id, &path, IdReason::InsertHyperlinkNeedsShape)?;
        return Ok(LinkInsertHost::Table(table));
    }
    if id.contains("/chart:") {
        let chart = chart::parse_chart_id(id, &path).map_err(|_| {
            Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::InsertHyperlinkNeedsShape,
                },
                path,
            )
        })?;
        return Ok(LinkInsertHost::Chart(chart));
    }
    if id.contains("/smartdiagram:") {
        let art = smartart::parse_diagram_id(id, &path).map_err(|_| {
            Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::InsertHyperlinkNeedsShape,
                },
                path,
            )
        })?;
        return Ok(LinkInsertHost::SmartArt(art));
    }
    Ok(LinkInsertHost::Shape(parse_insert_hyperlink_shape(id, i)?))
}

pub(super) fn parse_insert_field_shape(id: &str, i: usize) -> Result<shape::ShapeTarget, Error> {
    parse_link_or_field_shape(id, i, IdReason::InsertFieldNeedsShape)
}

pub(super) fn parse_insert_hyperlink_shape(
    id: &str,
    i: usize,
) -> Result<shape::ShapeTarget, Error> {
    parse_link_or_field_shape(id, i, IdReason::InsertHyperlinkNeedsShape)
}

pub(super) fn parse_link_or_field_shape(
    id: &str,
    i: usize,
    bad: IdReason,
) -> Result<shape::ShapeTarget, Error> {
    if id.starts_with("notesMaster") || id.starts_with("handout") {
        return Err(Error::with_path(
            ErrorKind::InvalidId { reason: bad },
            format!("ops[{i}].id"),
        ));
    }
    if id.starts_with("master:") {
        let ph = super::super::master::parse_placeholder_id(id, i).map_err(|_| {
            Error::with_path(
                ErrorKind::InvalidId {
                    reason: bad.clone(),
                },
                format!("ops[{i}].id"),
            )
        })?;
        let super::super::master::PlaceholderHost::Master { master, layout } = ph.host else {
            return Err(Error::with_path(
                ErrorKind::InvalidId { reason: bad },
                format!("ops[{i}].id"),
            ));
        };
        return Ok(shape::ShapeTarget {
            slide: 0,
            on_notes: false,
            master: Some((master, layout)),
            key: ph.key,
        });
    }
    shape::parse_shape_id(id, i)
        .map_err(|_| Error::with_path(ErrorKind::InvalidId { reason: bad }, format!("ops[{i}].id")))
}

pub(super) fn require_fld_type(obj: &Map<String, Value>, i: usize) -> Result<String, Error> {
    match obj.get("type") {
        Some(Value::String(s)) if field::is_reserved_type(s) => Ok(s.clone()),
        Some(Value::String(_)) => Err(Error::with_path(
            ErrorKind::InvalidFldType,
            format!("ops[{i}].type"),
        )),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "type".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].type"),
        )),
        None => Err(Error::with_path(
            ErrorKind::MissingField {
                field: "type".into(),
            },
            format!("ops[{i}].type"),
        )),
    }
}

pub(super) fn parse_pptx_link_target(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<Option<PptxLinkKind>, Error> {
    let url = parse_pptx_target_string(obj, i, "url", ErrorKind::EmptyHyperlinkUrl)?;
    let file = parse_pptx_target_string(obj, i, "file", ErrorKind::EmptyHyperlinkFile)?;
    let slide = match obj.get("slide") {
        None => None,
        Some(Value::Null) => {
            return Err(Error::with_path(
                ErrorKind::PptxHyperlinkNeedTarget,
                format!("ops[{i}].slide"),
            ));
        }
        Some(Value::String(s)) if s.is_empty() => {
            return Err(Error::with_path(
                ErrorKind::PptxHyperlinkNeedTarget,
                format!("ops[{i}].slide"),
            ));
        }
        Some(Value::String(s)) => Some(hyperlink::parse_jump_slide(s, i, "slide")?),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "slide".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].slide"),
            ));
        }
    };
    match (url, file, slide) {
        (Some(u), None, None) => Ok(Some(PptxLinkKind::Url(u))),
        (None, Some(f), None) => Ok(Some(PptxLinkKind::File(f))),
        (None, None, Some(n)) => Ok(Some(PptxLinkKind::Slide(n))),
        (None, None, None) => Ok(None),
        _ => Err(Error::with_path(
            ErrorKind::PptxHyperlinkNeedTarget,
            format!("ops[{i}]"),
        )),
    }
}

pub(super) fn parse_pptx_target_string(
    obj: &Map<String, Value>,
    i: usize,
    field: &str,
    empty: ErrorKind,
) -> Result<Option<String>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::Null) => Err(Error::with_path(empty, format!("ops[{i}].{field}"))),
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

pub(super) fn parse_optional_tooltip(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<Option<String>, Error> {
    match obj.get("tooltip") {
        None => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "tooltip".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].tooltip"),
        )),
    }
}

pub(super) fn parse_set_tooltip(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<Option<Option<String>>, Error> {
    match obj.get("tooltip") {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(None)),
        Some(Value::String(s)) => Ok(Some(Some(s.clone()))),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "tooltip".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].tooltip"),
        )),
    }
}

pub(super) fn parse_create_opt_hyperlink(
    obj: &Map<String, Value>,
    prefix: &str,
) -> Result<Option<CreateHyperlink>, Error> {
    match obj.get("hyperlink") {
        None => Ok(None),
        Some(v) => {
            let inner = v.as_object().ok_or_else(|| {
                Error::with_path(
                    ErrorKind::WrongType {
                        field: "hyperlink".into(),
                        expected: TypeName::Object,
                    },
                    format!("{prefix}.hyperlink"),
                )
            })?;
            let path = format!("{prefix}.hyperlink");
            deny_keys_at(inner, &path, &["url", "file", "slide", "tooltip", "event"])?;
            parse_create_link_fields(inner, &path, true)
        }
    }
}
