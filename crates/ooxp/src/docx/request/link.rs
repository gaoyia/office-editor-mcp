//! docx request：link。

use serde_json::Value;

use crate::error::{Error, ErrorKind, IdReason, TypeName};

use super::super::charts::{self};
use super::super::connector;
use super::super::ink;
use super::super::model3d;
use super::super::para::{self};
use super::super::picture::{self};
use super::super::shape;
use super::super::smartart;

use super::common::*;
use super::drawing::*;
use super::para::*;
use super::types::*;

pub(super) fn parse_insert_hyperlink(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertHyperlinkOp, Error> {
    let id = require_op_id(obj, i)?;
    let path = format!("ops[{i}].id");
    let host = if let Some(h) = parse_drawing_insert_host(&id, &path)? {
        deny_keys(obj, i, &["op", "id", "url", "anchor", "tooltip"])?;
        h
    } else if id.contains("/ole:") {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::InsertLinkNeedsPara,
            },
            path,
        ));
    } else {
        deny_keys(obj, i, &["op", "id", "text", "url", "anchor", "tooltip"])?;
        let para = parse_story_or_box_para_at(&id, &path, IdReason::InsertLinkNeedsPara)?;
        let text = match obj.get("text") {
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
            None => {
                return Err(Error::with_path(
                    ErrorKind::OpRequiresField {
                        op: "insertHyperlink".into(),
                        field: "text".into(),
                    },
                    format!("ops[{i}].text"),
                ));
            }
        };
        InsertHyperlinkHost::Para { para, text }
    };
    let url = optional_op_string(obj, i, "url")?;
    let anchor = optional_op_string(obj, i, "anchor")?;
    match (&url, &anchor) {
        (Some(_), Some(_)) => {
            return Err(Error::with_path(
                ErrorKind::MutuallyExclusive {
                    a: "url".into(),
                    b: "anchor".into(),
                },
                format!("ops[{i}]"),
            ));
        }
        (None, None) => {
            return Err(Error::with_path(
                ErrorKind::HyperlinkNeedTarget,
                format!("ops[{i}]"),
            ));
        }
        _ => {}
    }
    if let Some(u) = url.as_deref() {
        if u.is_empty() {
            return Err(Error::with_path(
                ErrorKind::EmptyHyperlinkUrl,
                format!("ops[{i}].url"),
            ));
        }
    }
    if let Some(a) = anchor.as_deref() {
        super::super::link::check_bookmark_name(a, i, "anchor")?;
    }
    let tooltip = optional_op_string(obj, i, "tooltip")?;
    Ok(InsertHyperlinkOp {
        id,
        host,
        url,
        anchor,
        tooltip,
    })
}

pub(super) fn parse_set_hyperlink(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetHyperlinkOp, Error> {
    let id = require_op_id(obj, i)?;
    let path = format!("ops[{i}].id");
    let drawing = looks_like_drawing_hlink_id(&id);
    if drawing {
        deny_keys(obj, i, &["op", "id", "url", "anchor", "tooltip"])?;
    } else {
        deny_keys(obj, i, &["op", "id", "url", "anchor", "text", "tooltip"])?;
    }
    let target = parse_hyperlink_target(&id, &path)?;
    let url = optional_op_string(obj, i, "url")?;
    let anchor = optional_op_string(obj, i, "anchor")?;
    let text = if drawing {
        None
    } else {
        optional_op_string(obj, i, "text")?
    };
    let tooltip = match obj.get("tooltip") {
        None => None,
        Some(Value::Null) => Some(None),
        Some(Value::String(s)) => Some(Some(s.clone())),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "tooltip".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].tooltip"),
            ));
        }
    };
    if url.is_some() && anchor.is_some() {
        return Err(Error::with_path(
            ErrorKind::MutuallyExclusive {
                a: "url".into(),
                b: "anchor".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    if url.is_none() && anchor.is_none() && text.is_none() && tooltip.is_none() {
        return Err(Error::with_path(
            ErrorKind::NeedSetHyperlinkField,
            format!("ops[{i}]"),
        ));
    }
    if let Some(u) = url.as_deref() {
        if u.is_empty() {
            return Err(Error::with_path(
                ErrorKind::EmptyHyperlinkUrl,
                format!("ops[{i}].url"),
            ));
        }
    }
    if let Some(a) = anchor.as_deref() {
        super::super::link::check_bookmark_name(a, i, "anchor")?;
    }
    Ok(SetHyperlinkOp {
        id,
        target,
        url,
        anchor,
        text,
        tooltip,
    })
}

pub(super) fn parse_delete_hyperlink(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteHyperlinkOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_op_id(obj, i)?;
    let target = parse_hyperlink_target(&id, &format!("ops[{i}].id"))?;
    Ok(DeleteHyperlinkOp { id, target })
}

pub(super) fn looks_like_pic_hlink_id(id: &str) -> bool {
    id.contains("/pic:")
}

pub(super) fn looks_like_chart_hlink_id(id: &str) -> bool {
    let Some(rest) = id.rsplit_once("/chart:").map(|(_, r)| r) else {
        return false;
    };
    !id.contains("/series:") && !rest.contains('/')
}

pub(super) fn looks_like_drawing_hlink_id(id: &str) -> bool {
    looks_like_pic_hlink_id(id)
        || looks_like_txbx_hlink_id(id)
        || shape::looks_like_shape_id(id)
        || connector::looks_like_cxn_id(id)
        || ink::looks_like_ink_id(id)
        || model3d::looks_like_id(id)
        || looks_like_chart_hlink_id(id)
        || looks_like_smartart_hlink_id(id)
}

pub(super) fn parse_drawing_insert_host(
    id: &str,
    path: &str,
) -> Result<Option<InsertHyperlinkHost>, Error> {
    Ok(parse_drawing_hlink_target(id, path)?.map(|t| match t {
        HyperlinkTarget::Picture(p) => InsertHyperlinkHost::Picture(p),
        HyperlinkTarget::TextBox(t) => InsertHyperlinkHost::TextBox(t),
        HyperlinkTarget::Shape(t) => InsertHyperlinkHost::Shape(t),
        HyperlinkTarget::Connector(t) => InsertHyperlinkHost::Connector(t),
        HyperlinkTarget::Ink(t) => InsertHyperlinkHost::Ink(t),
        HyperlinkTarget::Model3d(t) => InsertHyperlinkHost::Model3d(t),
        HyperlinkTarget::Chart(t) => InsertHyperlinkHost::Chart(t),
        HyperlinkTarget::SmartArt(t) => InsertHyperlinkHost::SmartArt(t),
        HyperlinkTarget::Link(_) => unreachable!(),
    }))
}

pub(super) fn parse_drawing_hlink_target(
    id: &str,
    path: &str,
) -> Result<Option<HyperlinkTarget>, Error> {
    if looks_like_pic_hlink_id(id) {
        return Ok(Some(HyperlinkTarget::Picture(picture::parse_pic_id_at(
            id, path,
        )?)));
    }
    if looks_like_txbx_hlink_id(id) {
        return Ok(Some(HyperlinkTarget::TextBox(para::parse_txbx_id_at(
            id, path,
        )?)));
    }
    if shape::looks_like_shape_id(id) {
        return Ok(Some(HyperlinkTarget::Shape(shape::parse_shape_id_at(
            id, path,
        )?)));
    }
    if connector::looks_like_cxn_id(id) {
        return Ok(Some(HyperlinkTarget::Connector(
            connector::parse_cxn_id_at(id, path)?,
        )));
    }
    if ink::looks_like_ink_id(id) {
        return Ok(Some(HyperlinkTarget::Ink(ink::parse_ink_id_at(id, path)?)));
    }
    if model3d::looks_like_id(id) {
        return Ok(Some(HyperlinkTarget::Model3d(model3d::parse_id_at(
            id, path,
        )?)));
    }
    if looks_like_chart_hlink_id(id) {
        return Ok(Some(HyperlinkTarget::Chart(charts::parse_chart_id_at(
            id, path,
        )?)));
    }
    if looks_like_smartart_hlink_id(id) {
        return Ok(Some(HyperlinkTarget::SmartArt(
            smartart::parse_diagram_id_at(id, path)?,
        )));
    }
    Ok(None)
}

pub(super) fn parse_hyperlink_target(id: &str, path: &str) -> Result<HyperlinkTarget, Error> {
    if id.contains("/ole:") {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::InsertLinkNeedsPara,
            },
            path.to_string(),
        ));
    }
    if let Some(t) = parse_drawing_hlink_target(id, path)? {
        return Ok(t);
    }
    let link = super::super::link::parse_link_ref(id, path)?;
    Ok(HyperlinkTarget::Link(link))
}

pub(super) fn parse_insert_bookmark(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertBookmarkOp, Error> {
    deny_keys(obj, i, &["op", "id", "name", "end"])?;
    let id = require_op_id(obj, i)?;
    let start = parse_story_or_box_para_at(
        &id,
        &format!("ops[{i}].id"),
        IdReason::InsertBookmarkNeedsPara,
    )?;
    let end = match obj.get("end") {
        None => start.clone(),
        Some(Value::String(s)) if !s.is_empty() => parse_story_or_box_para_at(
            s,
            &format!("ops[{i}].end"),
            IdReason::InsertBookmarkNeedsPara,
        )?,
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::InsertBookmarkNeedsPara,
                },
                format!("ops[{i}].end"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "end".into(),
                    expected: TypeName::NonEmptyString,
                },
                format!("ops[{i}].end"),
            ));
        }
    };
    if !same_story(&start, &end) {
        return Err(Error::with_path(
            ErrorKind::BookmarkRangeCrossStory,
            format!("ops[{i}].end"),
        ));
    }
    let name = optional_op_string(obj, i, "name")?;
    if let Some(n) = name.as_deref() {
        super::super::link::check_bookmark_name(n, i, "name")?;
    }
    Ok(InsertBookmarkOp {
        id,
        start,
        end,
        name,
    })
}

pub(super) fn parse_set_bookmark(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetBookmarkOp, Error> {
    deny_keys(obj, i, &["op", "id", "name"])?;
    let id = require_op_id(obj, i)?;
    let target = super::super::link::parse_bookmark_ref(&id, &format!("ops[{i}].id"))?;
    let name = match obj.get("name") {
        Some(Value::String(s)) => {
            super::super::link::check_bookmark_name(s, i, "name")?;
            s.clone()
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "name".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].name"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::OpRequiresField {
                    op: "setBookmark".into(),
                    field: "name".into(),
                },
                format!("ops[{i}].name"),
            ));
        }
    };
    Ok(SetBookmarkOp { id, target, name })
}

pub(super) fn parse_delete_bookmark(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteBookmarkOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_op_id(obj, i)?;
    let target = super::super::link::parse_bookmark_ref(&id, &format!("ops[{i}].id"))?;
    Ok(DeleteBookmarkOp { id, target })
}

pub(super) fn parse_insert_edit_range(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertEditRangeOp, Error> {
    deny_keys(obj, i, &["op", "id", "edGrp", "ed"])?;
    let id = require_op_id(obj, i)?;
    let path = format!("ops[{i}].id");
    let para = super::super::para::parse_para_id_at(&id, &path).map_err(|_| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::InsertEditRangeNeedsPara,
            },
            path.clone(),
        )
    })?;
    if matches!(
        para,
        super::super::para::ParaTarget::InNote { .. }
            | super::super::para::ParaTarget::InComment { .. }
    ) {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::InsertEditRangeNeedsPara,
            },
            path,
        ));
    }
    let ed_grp = match obj.get("edGrp") {
        None => None,
        Some(Value::String(s)) if super::super::perm::ed_grp_ok(s) => Some(s.clone()),
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidEdGrp,
                format!("ops[{i}].edGrp"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "edGrp".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].edGrp"),
            ));
        }
    };
    let ed = match obj.get("ed") {
        None => None,
        Some(Value::String(s)) if !s.is_empty() => Some(s.clone()),
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::EmptyEditRangeEd,
                format!("ops[{i}].ed"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "ed".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].ed"),
            ));
        }
    };
    if ed_grp.is_some() && ed.is_some() {
        return Err(Error::with_path(
            ErrorKind::MutuallyExclusive {
                a: "edGrp".into(),
                b: "ed".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    let ed_grp = if ed_grp.is_none() && ed.is_none() {
        Some("everyone".into())
    } else {
        ed_grp
    };
    Ok(InsertEditRangeOp {
        id,
        para,
        ed_grp,
        ed,
    })
}

pub(super) fn parse_delete_edit_range(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteEditRangeOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_op_id(obj, i)?;
    let target = super::super::perm::parse_perm_ref(&id, &format!("ops[{i}].id"))?;
    Ok(DeleteEditRangeOp { id, target })
}
