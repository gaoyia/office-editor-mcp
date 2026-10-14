//! pptx request：extras。

use serde_json::{Map, Value};

use crate::error::{Error, ErrorKind, IdReason, TypeName};

use super::super::custom_show;
use super::super::field;
use super::super::ole;
use super::super::picture;
use super::super::section;
use super::super::zoom;

use super::common::*;
use super::media::*;
use super::shape::*;
use super::slides::*;
use super::types::*;

pub(super) fn parse_set_slide_section_name(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<SetSlideSectionNameOp, Error> {
    deny_keys(obj, i, &["op", "id", "name"])?;
    let id = require_id(obj, i)?;
    Ok(SetSlideSectionNameOp {
        section: section::parse_section_id(&id, &format!("ops[{i}].id"))?,
        name: require_name(obj, i)?,
    })
}

pub(super) fn parse_set_slide_section(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<SetSlideSectionOp, Error> {
    deny_keys(obj, i, &["op", "id", "section"])?;
    let id = require_id(obj, i)?;
    let slide = if id
        .strip_prefix("slide:")
        .is_some_and(|rest| !rest.is_empty() && !rest.contains('/'))
    {
        parse_slide_only_id(&id, i, "id")?
    } else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SetSlideSectionNeedsSlide,
            },
            format!("ops[{i}].id"),
        ));
    };
    let section = match obj.get("section") {
        Some(Value::String(s)) => section::parse_section_id(s, &format!("ops[{i}].section"))?,
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "section".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].section"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "section".into(),
                },
                format!("ops[{i}].section"),
            ));
        }
    };
    Ok(SetSlideSectionOp { slide, section })
}

pub(super) fn parse_insert_custom_show(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<InsertCustomShowOp, Error> {
    deny_keys(obj, i, &["op", "id", "name", "slides", "after"])?;
    let id = require_id(obj, i)?;
    if id != "pptx:presentation" {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::InsertCustomShowNeedsPresentation,
            },
            format!("ops[{i}].id"),
        ));
    }
    let name = require_name(obj, i)?;
    custom_show::check_name(&name, &format!("ops[{i}].name"))?;
    Ok(InsertCustomShowOp {
        name,
        slides: parse_slides_array(obj, i, false)?.unwrap_or_default(),
        after: parse_optional_show_id_field(obj, i, "after")?,
    })
}

pub(super) fn parse_set_custom_show(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<SetCustomShowOp, Error> {
    deny_keys(obj, i, &["op", "id", "name", "slides"])?;
    let id = require_id(obj, i)?;
    let show = custom_show::parse_show_id(&id, &format!("ops[{i}].id"))?;
    let name = match obj.get("name") {
        None => None,
        Some(Value::String(s)) => {
            custom_show::check_name(s, &format!("ops[{i}].name"))?;
            Some(s.clone())
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
    };
    let slides = parse_slides_array(obj, i, true)?;
    if name.is_none() && slides.is_none() {
        return Err(Error::with_path(
            ErrorKind::NeedCustomShowField,
            format!("ops[{i}]"),
        ));
    }
    Ok(SetCustomShowOp { show, name, slides })
}

pub(super) fn parse_delete_custom_show(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<DeleteCustomShowOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    Ok(DeleteCustomShowOp {
        show: custom_show::parse_show_id(&id, &format!("ops[{i}].id"))?,
    })
}

pub(super) fn parse_insert_ole(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<ole::InsertOleOp, Error> {
    deny_keys(obj, i, &["op", "id", "file"])?;
    let id = require_id(obj, i)?;
    let (slide, on_notes, master) = ole::parse_insert_slide_id(&id, i)?.parts();
    let file = require_file(obj, i, "insertOle")?;
    Ok(ole::InsertOleOp {
        slide,
        on_notes,
        master,
        file,
    })
}

pub(super) fn parse_delete_ole(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<ole::DeleteOleOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let target = ole::parse_ole_id(&id, &format!("ops[{i}].id"))?;
    Ok(ole::DeleteOleOp { id, target })
}

pub(super) fn parse_insert_model3d(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<super::super::model3d::InsertModel3dOp, Error> {
    deny_keys(obj, i, &["op", "id", "file", "x", "y", "cx", "cy"])?;
    let id = require_id(obj, i)?;
    let (slide, on_notes, master) =
        picture::parse_insert_host(&id, i, IdReason::InsertModel3dNeedsSlide)?.parts();
    let file = require_file(obj, i, "insertModel3d")?;
    let cx = parse_emu_field(obj.get("cx"), i, "cx")?;
    let cy = parse_emu_field(obj.get("cy"), i, "cy")?;
    if cx.is_some() != cy.is_some() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "insertModel3d".into(),
                field: "cx and cy together".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(super::super::model3d::InsertModel3dOp {
        slide,
        on_notes,
        master,
        file,
        x: parse_emu_field(obj.get("x"), i, "x")?,
        y: parse_emu_field(obj.get("y"), i, "y")?,
        cx,
        cy,
    })
}

pub(super) fn parse_replace_model3d(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<super::super::model3d::ReplaceModel3dOp, Error> {
    deny_keys(obj, i, &["op", "id", "file"])?;
    let id = require_id(obj, i)?;
    let target = super::super::model3d::parse_id(&id, &format!("ops[{i}].id"))?;
    let file = require_file(obj, i, "replaceModel3d")?;
    Ok(super::super::model3d::ReplaceModel3dOp { id, target, file })
}

pub(super) fn parse_delete_model3d(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<super::super::model3d::DeleteModel3dOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let target = super::super::model3d::parse_id(&id, &format!("ops[{i}].id"))?;
    Ok(super::super::model3d::DeleteModel3dOp { id, target })
}

pub(super) fn parse_set_model3d_size(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<super::super::model3d::SetModel3dSizeOp, Error> {
    deny_keys(obj, i, &["op", "id", "cx", "cy"])?;
    let id = require_id(obj, i)?;
    let target = super::super::model3d::parse_id(&id, &format!("ops[{i}].id"))?;
    let cx = parse_emu_field(obj.get("cx"), i, "cx")?;
    let cy = parse_emu_field(obj.get("cy"), i, "cy")?;
    if cx.is_none() && cy.is_none() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setModel3dSize".into(),
                field: "cx or cy".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(super::super::model3d::SetModel3dSizeOp { id, target, cx, cy })
}

pub(super) fn parse_set_model3d_pos(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<super::super::model3d::SetModel3dPosOp, Error> {
    deny_keys(obj, i, &["op", "id", "x", "y"])?;
    let id = require_id(obj, i)?;
    let target = super::super::model3d::parse_id(&id, &format!("ops[{i}].id"))?;
    let x = parse_off_field(obj.get("x"), i, "x")?;
    let y = parse_off_field(obj.get("y"), i, "y")?;
    if x.is_none() && y.is_none() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setModel3dPos".into(),
                field: "x or y".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(super::super::model3d::SetModel3dPosOp { id, target, x, y })
}

pub(super) fn parse_set_model3d_camera(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<super::super::model3d::SetModel3dCameraOp, Error> {
    deny_keys(obj, i, &["op", "id", "camera", "rot", "lightRig"])?;
    let id = require_id(obj, i)?;
    let target = super::super::model3d::parse_id(&id, &format!("ops[{i}].id"))?;
    let patch = crate::model3d::parse_camera_patch(obj, i)?;
    Ok(super::super::model3d::SetModel3dCameraOp { id, target, patch })
}

pub(super) fn parse_insert_math(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<super::super::omml::InsertMathOp, Error> {
    deny_keys(
        obj,
        i,
        &["op", "id", "text", "x", "y", "cx", "cy", "expr", "kind"],
    )?;
    crate::drawing_math::reject_expr_kind(obj, &format!("ops[{i}]"))?;
    let id = require_id(obj, i)?;
    let (slide, on_notes, master) =
        picture::parse_insert_host(&id, i, IdReason::InsertMathNeedsSlide)?.parts();
    let text = crate::drawing_math::require_text(obj, &format!("ops[{i}]"))?;
    let cx = parse_emu_field(obj.get("cx"), i, "cx")?;
    let cy = parse_emu_field(obj.get("cy"), i, "cy")?;
    if cx.is_some() != cy.is_some() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "insertMath".into(),
                field: "cx and cy together".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(super::super::omml::InsertMathOp {
        slide,
        on_notes,
        master,
        text,
        x: parse_emu_field(obj.get("x"), i, "x")?,
        y: parse_emu_field(obj.get("y"), i, "y")?,
        cx,
        cy,
    })
}

pub(super) fn parse_set_math(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<super::super::omml::SetMathOp, Error> {
    deny_keys(obj, i, &["op", "id", "text", "expr", "kind"])?;
    crate::drawing_math::reject_expr_kind(obj, &format!("ops[{i}]"))?;
    let id = require_id(obj, i)?;
    let target = super::super::omml::parse_id(&id, &format!("ops[{i}].id"))?;
    let text = crate::drawing_math::require_text(obj, &format!("ops[{i}]"))?;
    Ok(super::super::omml::SetMathOp { id, target, text })
}

pub(super) fn parse_delete_math(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<super::super::omml::DeleteMathOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let target = super::super::omml::parse_id(&id, &format!("ops[{i}].id"))?;
    Ok(super::super::omml::DeleteMathOp { id, target })
}

pub(super) fn parse_set_math_size(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<super::super::omml::SetMathSizeOp, Error> {
    deny_keys(obj, i, &["op", "id", "cx", "cy"])?;
    let id = require_id(obj, i)?;
    let target = super::super::omml::parse_id(&id, &format!("ops[{i}].id"))?;
    let cx = parse_emu_field(obj.get("cx"), i, "cx")?;
    let cy = parse_emu_field(obj.get("cy"), i, "cy")?;
    if cx.is_none() && cy.is_none() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setMathSize".into(),
                field: "cx or cy".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(super::super::omml::SetMathSizeOp { id, target, cx, cy })
}

pub(super) fn parse_set_math_pos(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<super::super::omml::SetMathPosOp, Error> {
    deny_keys(obj, i, &["op", "id", "x", "y"])?;
    let id = require_id(obj, i)?;
    let target = super::super::omml::parse_id(&id, &format!("ops[{i}].id"))?;
    let x = parse_off_field(obj.get("x"), i, "x")?;
    let y = parse_off_field(obj.get("y"), i, "y")?;
    if x.is_none() && y.is_none() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setMathPos".into(),
                field: "x or y".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(super::super::omml::SetMathPosOp { id, target, x, y })
}

pub(super) fn parse_insert_zoom(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<zoom::InsertZoomOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op", "id", "type", "target", "targets", "x", "y", "cx", "cy",
        ],
    )?;
    let id = require_id(obj, i)?;
    let (slide, on_notes, master) = zoom::parse_insert_slide_id(&id, i)?.parts();
    let kind_s = match obj.get("type") {
        None => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "type".into(),
                },
                format!("ops[{i}].type"),
            ));
        }
        Some(Value::String(s)) if s == "slide" || s == "section" || s == "summary" => s.clone(),
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidZoomType,
                format!("ops[{i}].type"),
            ));
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
    if obj.contains_key("target") && obj.contains_key("targets") {
        return Err(Error::with_path(
            ErrorKind::MutuallyExclusive {
                a: "target".into(),
                b: "targets".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    let kind = if kind_s == "summary" {
        if obj.contains_key("target") {
            return Err(Error::with_path(
                ErrorKind::InvalidZoomTargets,
                format!("ops[{i}].target"),
            ));
        }
        let targets = parse_zoom_targets(obj, i)?;
        zoom::InsertKind::Summary { targets }
    } else if obj.contains_key("targets") {
        return Err(Error::with_path(
            ErrorKind::InvalidZoomTargets,
            format!("ops[{i}].targets"),
        ));
    } else {
        let target_s = match obj.get("target") {
            None => {
                return Err(Error::with_path(
                    ErrorKind::MissingField {
                        field: "target".into(),
                    },
                    format!("ops[{i}].target"),
                ));
            }
            Some(Value::String(s)) if !s.is_empty() => s.as_str(),
            Some(Value::String(_)) => {
                return Err(Error::with_path(
                    ErrorKind::MissingField {
                        field: "target".into(),
                    },
                    format!("ops[{i}].target"),
                ));
            }
            Some(_) => {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: "target".into(),
                        expected: TypeName::String,
                    },
                    format!("ops[{i}].target"),
                ));
            }
        };
        if kind_s == "slide" {
            let target = zoom::parse_slide_target(target_s, &format!("ops[{i}].target"))?;
            if target == slide {
                return Err(Error::with_path(
                    ErrorKind::ZoomTargetSelf,
                    format!("ops[{i}].target"),
                ));
            }
            zoom::InsertKind::Slide { target }
        } else {
            let target = section::parse_section_id(target_s, &format!("ops[{i}].target"))?;
            zoom::InsertKind::Section { target }
        }
    };
    let x = parse_off_field(obj.get("x"), i, "x")?;
    let y = parse_off_field(obj.get("y"), i, "y")?;
    if x.is_some() != y.is_some() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "insertZoom".into(),
                field: "x and y together".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    let cx = parse_emu_field(obj.get("cx"), i, "cx")?;
    let cy = parse_emu_field(obj.get("cy"), i, "cy")?;
    if cx.is_some() != cy.is_some() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "insertZoom".into(),
                field: "cx and cy together".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(zoom::InsertZoomOp {
        slide,
        on_notes,
        master,
        kind,
        x,
        y,
        cx,
        cy,
    })
}

pub(super) fn parse_zoom_targets(obj: &Map<String, Value>, i: usize) -> Result<Vec<usize>, Error> {
    match obj.get("targets") {
        None => Err(Error::with_path(
            ErrorKind::MissingField {
                field: "targets".into(),
            },
            format!("ops[{i}].targets"),
        )),
        Some(Value::Array(arr)) if arr.is_empty() => Err(Error::with_path(
            ErrorKind::InvalidZoomTargets,
            format!("ops[{i}].targets"),
        )),
        Some(Value::Array(arr)) => {
            let mut out = Vec::new();
            for (j, v) in arr.iter().enumerate() {
                let s = v.as_str().ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::WrongType {
                            field: format!("targets[{j}]"),
                            expected: TypeName::String,
                        },
                        format!("ops[{i}].targets[{j}]"),
                    )
                })?;
                let n = section::parse_section_id(s, &format!("ops[{i}].targets[{j}]"))?;
                if out.contains(&n) {
                    return Err(Error::with_path(
                        ErrorKind::InvalidZoomTargets,
                        format!("ops[{i}].targets[{j}]"),
                    ));
                }
                out.push(n);
            }
            Ok(out)
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "targets".into(),
                expected: TypeName::Array,
            },
            format!("ops[{i}].targets"),
        )),
    }
}

pub(super) fn parse_delete_zoom(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<zoom::DeleteZoomOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let target = zoom::parse_zoom_id(&id, &format!("ops[{i}].id"))?;
    Ok(zoom::DeleteZoomOp { id, target })
}

pub(super) fn parse_set_zoom(obj: &Map<String, Value>, i: usize) -> Result<zoom::SetZoomOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op", "id", "target", "targets", "file", "x", "y", "cx", "cy",
        ],
    )?;
    let id = require_id(obj, i)?;
    let target = zoom::parse_zoom_id(&id, &format!("ops[{i}].id"))?;
    if obj.contains_key("target") && obj.contains_key("targets") {
        return Err(Error::with_path(
            ErrorKind::MutuallyExclusive {
                a: "target".into(),
                b: "targets".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    let dest = if obj.contains_key("targets") {
        Some(zoom::SetZoomDest::Summary {
            targets: parse_zoom_targets(obj, i)?,
        })
    } else {
        match obj.get("target") {
            None => None,
            Some(Value::String(s)) if s.starts_with("slide:") => {
                let n = zoom::parse_slide_target(s, &format!("ops[{i}].target"))?;
                Some(zoom::SetZoomDest::Slide { target: n })
            }
            Some(Value::String(s)) if s.starts_with("section:") => {
                let n = section::parse_section_id(s, &format!("ops[{i}].target"))?;
                Some(zoom::SetZoomDest::Section { target: n })
            }
            Some(Value::String(_)) => {
                return Err(Error::with_path(
                    ErrorKind::InvalidId {
                        reason: IdReason::ZoomTargetSlide,
                    },
                    format!("ops[{i}].target"),
                ));
            }
            Some(_) => {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: "target".into(),
                        expected: TypeName::String,
                    },
                    format!("ops[{i}].target"),
                ));
            }
        }
    };
    let file = match obj.get("file") {
        None => None,
        Some(Value::String(s)) if !s.is_empty() => Some(s.clone()),
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "file".into(),
                },
                format!("ops[{i}].file"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "file".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].file"),
            ));
        }
    };
    let x = parse_off_field(obj.get("x"), i, "x")?;
    let y = parse_off_field(obj.get("y"), i, "y")?;
    let cx = parse_emu_field(obj.get("cx"), i, "cx")?;
    let cy = parse_emu_field(obj.get("cy"), i, "cy")?;
    if cx.is_some() != cy.is_some() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setZoom".into(),
                field: "cx and cy".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    if dest.is_none()
        && file.is_none()
        && x.is_none()
        && y.is_none()
        && cx.is_none()
        && cy.is_none()
    {
        return Err(Error::with_path(
            ErrorKind::NeedZoomSetField,
            format!("ops[{i}]"),
        ));
    }
    Ok(zoom::SetZoomOp {
        id,
        target,
        dest,
        file,
        x,
        y,
        cx,
        cy,
    })
}

pub(super) fn parse_set_show_pr(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<super::super::show::SetShowPrOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op",
            "id",
            "type",
            "loop",
            "showNarration",
            "showAnimation",
            "useTimings",
        ],
    )?;
    let id = require_id(obj, i)?;
    if id != "pptx:presentation" {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SetShowPrNeedsPresentation,
            },
            format!("ops[{i}].id"),
        ));
    }
    let type_name = match obj.get("type") {
        None => None,
        Some(Value::String(s)) if matches!(s.as_str(), "present" | "browse" | "kiosk") => {
            Some(s.clone())
        }
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidShowPrType,
                format!("ops[{i}].type"),
            ));
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
    let loop_ = parse_opt_bool(obj, i, "loop")?;
    let show_narration = parse_opt_bool(obj, i, "showNarration")?;
    let show_animation = parse_opt_bool(obj, i, "showAnimation")?;
    let use_timings = parse_opt_bool(obj, i, "useTimings")?;
    if type_name.is_none()
        && loop_.is_none()
        && show_narration.is_none()
        && show_animation.is_none()
        && use_timings.is_none()
    {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setShowPr".into(),
                field: "type, loop, showNarration, showAnimation, or useTimings".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(super::super::show::SetShowPrOp {
        type_name,
        loop_,
        show_narration,
        show_animation,
        use_timings,
    })
}

pub(super) fn parse_opt_password(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<Option<String>, Error> {
    match obj.get("password") {
        None => Ok(None),
        Some(Value::String(s)) if !s.is_empty() => Ok(Some(s.clone())),
        Some(Value::String(_)) => Err(Error::with_path(
            ErrorKind::ProtectionPasswordBlank,
            format!("ops[{i}].password"),
        )),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "password".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].password"),
        )),
    }
}

pub(super) fn parse_set_protection(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<super::super::protect::SetProtectionOp, Error> {
    deny_keys(obj, i, &["op", "id", "enforced", "password"])?;
    let id = require_id(obj, i)?;
    if id != "pptx:presentation" {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SetProtectionNeedsPresentation,
            },
            format!("ops[{i}].id"),
        ));
    }
    let enforced = match obj.get("enforced") {
        Some(Value::Bool(b)) => *b,
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "enforced".into(),
                    expected: TypeName::Boolean,
                },
                format!("ops[{i}].enforced"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::NeedProtectionField,
                format!("ops[{i}]"),
            ));
        }
    };
    Ok(super::super::protect::SetProtectionOp {
        enforced,
        password: parse_opt_password(obj, i)?,
    })
}

pub(super) fn parse_slides_array(
    obj: &Map<String, Value>,
    i: usize,
    optional: bool,
) -> Result<Option<Vec<usize>>, Error> {
    match obj.get("slides") {
        None => {
            if optional {
                Ok(None)
            } else {
                Ok(Some(Vec::new()))
            }
        }
        Some(Value::Array(arr)) => {
            let mut out = Vec::new();
            for (j, v) in arr.iter().enumerate() {
                let s = v.as_str().ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::WrongType {
                            field: format!("slides[{j}]"),
                            expected: TypeName::String,
                        },
                        format!("ops[{i}].slides[{j}]"),
                    )
                })?;
                out.push(parse_slide_only_id(s, i, &format!("slides[{j}]"))?);
            }
            Ok(Some(out))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "slides".into(),
                expected: TypeName::Array,
            },
            format!("ops[{i}].slides"),
        )),
    }
}

pub(super) fn parse_optional_show_id_field(
    obj: &Map<String, Value>,
    i: usize,
    field: &str,
) -> Result<Option<usize>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::String(s)) => Ok(Some(custom_show::parse_show_id(
            s,
            &format!("ops[{i}].{field}"),
        )?)),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].{field}"),
        )),
    }
}

pub(crate) fn parse_create_media(
    v: &Value,
    prefix: &str,
    slide: usize,
) -> Result<InsertMediaOp, Error> {
    let obj = v.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "file".into(),
                expected: TypeName::Object,
            },
            prefix.to_string(),
        )
    })?;
    deny_keys_at(
        obj,
        prefix,
        &[
            "file",
            "x",
            "y",
            "cx",
            "cy",
            "st",
            "end",
            "bookmarks",
            "tracks",
            "trackDisplayLoc",
        ],
    )?;
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
    let x = parse_off_at(obj.get("x"), &format!("{prefix}.x"), "x")?;
    let y = parse_off_at(obj.get("y"), &format!("{prefix}.y"), "y")?;
    if x.is_some() != y.is_some() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "create".into(),
                field: "x and y together".into(),
            },
            prefix.to_string(),
        ));
    }
    let cx = parse_emu_at(obj.get("cx"), &format!("{prefix}.cx"), "cx")?;
    let cy = parse_emu_at(obj.get("cy"), &format!("{prefix}.cy"), "cy")?;
    if cx.is_some() != cy.is_some() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "create".into(),
                field: "cx and cy together".into(),
            },
            prefix.to_string(),
        ));
    }
    let extras = parse_media_clip_fields(obj, prefix, false)?;
    Ok(InsertMediaOp {
        slide,
        on_notes: false,
        master: None,
        file,
        x,
        y,
        cx,
        cy,
        st: extras.st,
        end: extras.end,
        bookmarks: extras.bookmarks,
        tracks: extras.tracks,
        track_display_loc: extras.track_display_loc,
    })
}

pub(crate) fn parse_create_shape(v: &Value, prefix: &str) -> Result<CreateShape, Error> {
    let obj = v.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "shapes".into(),
                expected: TypeName::Object,
            },
            prefix.to_string(),
        )
    })?;
    deny_keys_at(
        obj,
        prefix,
        &[
            "text",
            "url",
            "file",
            "slide",
            "tooltip",
            "on",
            "event",
            "sz",
            "color",
            "textFill",
            "textOutline",
            "bold",
            "italic",
            "underline",
            "font",
            "eastAsiaFont",
            "align",
            "textAnchor",
            "spcBef",
            "spcAft",
            "lnSpc",
            "spc",
            "kern",
            "geom",
            "fields",
        ],
    )?;
    let text = match obj.get("text") {
        None => String::new(),
        Some(Value::String(s)) => s.clone(),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "text".into(),
                    expected: TypeName::String,
                },
                format!("{prefix}.text"),
            ));
        }
    };
    let url = parse_create_link_string(obj, prefix, "url", ErrorKind::EmptyHyperlinkUrl)?;
    let file = parse_create_link_string(obj, prefix, "file", ErrorKind::EmptyHyperlinkFile)?;
    let slide = match obj.get("slide") {
        None => None,
        Some(Value::Null) => {
            return Err(Error::with_path(
                ErrorKind::PptxHyperlinkNeedTarget,
                format!("{prefix}.slide"),
            ));
        }
        Some(Value::Number(n)) => {
            let Some(u) = n.as_u64() else {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: "slide".into(),
                        expected: TypeName::Integer,
                    },
                    format!("{prefix}.slide"),
                ));
            };
            if u < 1 || u > usize::MAX as u64 {
                return Err(Error::with_path(
                    ErrorKind::MinValue {
                        field: "slide".into(),
                        min: 1,
                    },
                    format!("{prefix}.slide"),
                ));
            }
            Some(u as usize)
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "slide".into(),
                    expected: TypeName::Integer,
                },
                format!("{prefix}.slide"),
            ));
        }
    };
    let tooltip = match obj.get("tooltip") {
        None => None,
        Some(Value::String(s)) => Some(s.clone()),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "tooltip".into(),
                    expected: TypeName::String,
                },
                format!("{prefix}.tooltip"),
            ));
        }
    };
    let target = match (url, file, slide) {
        (Some(u), None, None) => Some(PptxLinkKind::Url(u)),
        (None, Some(f), None) => Some(PptxLinkKind::File(f)),
        (None, None, Some(n)) => Some(PptxLinkKind::Slide(n)),
        (None, None, None) => None,
        _ => {
            return Err(Error::with_path(
                ErrorKind::PptxHyperlinkNeedTarget,
                prefix.to_string(),
            ));
        }
    };
    if target.is_none() && tooltip.is_some() {
        return Err(Error::with_path(
            ErrorKind::PptxHyperlinkNeedTarget,
            format!("{prefix}.tooltip"),
        ));
    }
    let on_run = match obj.get("on") {
        None => false,
        Some(Value::String(s)) if s == "shape" => false,
        Some(Value::String(s)) if s == "run" => true,
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidHyperlinkOn,
                format!("{prefix}.on"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "on".into(),
                    expected: TypeName::String,
                },
                format!("{prefix}.on"),
            ));
        }
    };
    let event = match obj.get("event") {
        None => LinkEvent::Click,
        Some(Value::String(s)) if s == "click" => LinkEvent::Click,
        Some(Value::String(s)) if s == "hover" => LinkEvent::Hover,
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidHyperlinkEvent,
                format!("{prefix}.event"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "event".into(),
                    expected: TypeName::String,
                },
                format!("{prefix}.event"),
            ));
        }
    };
    if target.is_none() && (on_run || obj.get("on").is_some() || obj.get("event").is_some()) {
        return Err(Error::with_path(
            ErrorKind::PptxHyperlinkNeedTarget,
            prefix.to_string(),
        ));
    }
    let link = target.map(|target| CreateHyperlink {
        target,
        tooltip,
        on_run,
        event,
    });
    let format = crate::drawing_text::parse(obj, prefix, false)?;
    let geom = match obj.get("geom") {
        None => None,
        Some(v) => Some(parse_geom_object_at(
            v,
            &format!("{prefix}.geom"),
            "create",
        )?),
    };
    let fields = parse_create_fields(obj.get("fields"), &format!("{prefix}.fields"))?;
    Ok(CreateShape {
        text,
        link,
        format,
        geom,
        fields,
    })
}

pub(crate) fn parse_create_section(v: &Value, prefix: &str) -> Result<CreateSection, Error> {
    let obj = v.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "sections".into(),
                expected: TypeName::Object,
            },
            prefix.to_string(),
        )
    })?;
    deny_keys_at(obj, prefix, &["name", "start"])?;
    let name = match obj.get("name") {
        Some(Value::String(s)) => s.clone(),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "name".into(),
                    expected: TypeName::String,
                },
                format!("{prefix}.name"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "name".into(),
                },
                format!("{prefix}.name"),
            ));
        }
    };
    let start = match obj.get("start") {
        None => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "start".into(),
                },
                format!("{prefix}.start"),
            ));
        }
        Some(v) => parse_create_index(Some(v), &format!("{prefix}.start"), "start")?,
    };
    Ok(CreateSection { name, start })
}

pub(super) fn parse_create_fields(
    v: Option<&Value>,
    prefix: &str,
) -> Result<Vec<CreateField>, Error> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    let arr = v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "fields".into(),
                expected: TypeName::Array,
            },
            prefix.to_string(),
        )
    })?;
    let mut out = Vec::new();
    for (j, item) in arr.iter().enumerate() {
        out.push(parse_create_field(item, &format!("{prefix}[{j}]"))?);
    }
    Ok(out)
}

pub(super) fn parse_create_field(v: &Value, prefix: &str) -> Result<CreateField, Error> {
    let obj = v.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "fields".into(),
                expected: TypeName::Object,
            },
            prefix.to_string(),
        )
    })?;
    deny_keys_at(obj, prefix, &["type", "text"])?;
    let type_name = match obj.get("type") {
        Some(Value::String(s)) if field::is_reserved_type(s) => s.clone(),
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidFldType,
                format!("{prefix}.type"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "type".into(),
                    expected: TypeName::String,
                },
                format!("{prefix}.type"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "type".into(),
                },
                format!("{prefix}.type"),
            ));
        }
    };
    let text = match obj.get("text") {
        None => String::new(),
        Some(Value::String(s)) => s.clone(),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "text".into(),
                    expected: TypeName::String,
                },
                format!("{prefix}.text"),
            ));
        }
    };
    Ok(CreateField { type_name, text })
}

pub(crate) fn parse_create_ole(
    v: &Value,
    prefix: &str,
    slide: usize,
) -> Result<ole::InsertOleOp, Error> {
    let obj = v.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "oleObjects".into(),
                expected: TypeName::Object,
            },
            prefix.to_string(),
        )
    })?;
    deny_keys_at(obj, prefix, &["file"])?;
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
    Ok(ole::InsertOleOp {
        slide,
        on_notes: false,
        master: None,
        file,
    })
}

pub(crate) fn parse_create_zoom(
    v: &Value,
    prefix: &str,
    slide: usize,
) -> Result<zoom::InsertZoomOp, Error> {
    let obj = v.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "zooms".into(),
                expected: TypeName::Object,
            },
            prefix.to_string(),
        )
    })?;
    deny_keys_at(
        obj,
        prefix,
        &["type", "target", "targets", "x", "y", "cx", "cy"],
    )?;
    let kind_s = match obj.get("type") {
        None => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "type".into(),
                },
                format!("{prefix}.type"),
            ));
        }
        Some(Value::String(s)) if s == "slide" || s == "section" || s == "summary" => s.clone(),
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidZoomType,
                format!("{prefix}.type"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "type".into(),
                    expected: TypeName::String,
                },
                format!("{prefix}.type"),
            ));
        }
    };
    if obj.contains_key("target") && obj.contains_key("targets") {
        return Err(Error::with_path(
            ErrorKind::MutuallyExclusive {
                a: "target".into(),
                b: "targets".into(),
            },
            prefix.to_string(),
        ));
    }
    let kind = if kind_s == "summary" {
        if obj.contains_key("target") {
            return Err(Error::with_path(
                ErrorKind::InvalidZoomTargets,
                format!("{prefix}.target"),
            ));
        }
        let targets = parse_create_zoom_targets(obj, prefix)?;
        zoom::InsertKind::Summary { targets }
    } else if obj.contains_key("targets") {
        return Err(Error::with_path(
            ErrorKind::InvalidZoomTargets,
            format!("{prefix}.targets"),
        ));
    } else {
        let target = parse_create_index(obj.get("target"), &format!("{prefix}.target"), "target")?;
        if kind_s == "slide" {
            if target == slide {
                return Err(Error::with_path(
                    ErrorKind::ZoomTargetSelf,
                    format!("{prefix}.target"),
                ));
            }
            zoom::InsertKind::Slide { target }
        } else {
            zoom::InsertKind::Section { target }
        }
    };
    let x = parse_off_at(obj.get("x"), &format!("{prefix}.x"), "x")?;
    let y = parse_off_at(obj.get("y"), &format!("{prefix}.y"), "y")?;
    if x.is_some() != y.is_some() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "create".into(),
                field: "x and y together".into(),
            },
            prefix.to_string(),
        ));
    }
    let cx = parse_emu_at(obj.get("cx"), &format!("{prefix}.cx"), "cx")?;
    let cy = parse_emu_at(obj.get("cy"), &format!("{prefix}.cy"), "cy")?;
    if cx.is_some() != cy.is_some() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "create".into(),
                field: "cx and cy together".into(),
            },
            prefix.to_string(),
        ));
    }
    Ok(zoom::InsertZoomOp {
        slide,
        on_notes: false,
        master: None,
        kind,
        x,
        y,
        cx,
        cy,
    })
}

pub(super) fn parse_create_zoom_targets(
    obj: &Map<String, Value>,
    prefix: &str,
) -> Result<Vec<usize>, Error> {
    match obj.get("targets") {
        None => Err(Error::with_path(
            ErrorKind::MissingField {
                field: "targets".into(),
            },
            format!("{prefix}.targets"),
        )),
        Some(Value::Array(arr)) if arr.is_empty() => Err(Error::with_path(
            ErrorKind::InvalidZoomTargets,
            format!("{prefix}.targets"),
        )),
        Some(Value::Array(arr)) => {
            let mut out = Vec::new();
            for (j, v) in arr.iter().enumerate() {
                let n = parse_create_index(Some(v), &format!("{prefix}.targets[{j}]"), "targets")?;
                if out.contains(&n) {
                    return Err(Error::with_path(
                        ErrorKind::InvalidZoomTargets,
                        format!("{prefix}.targets[{j}]"),
                    ));
                }
                out.push(n);
            }
            Ok(out)
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "targets".into(),
                expected: TypeName::Array,
            },
            format!("{prefix}.targets"),
        )),
    }
}

pub(crate) fn parse_create_custom_show(
    v: &Value,
    prefix: &str,
) -> Result<InsertCustomShowOp, Error> {
    let obj = v.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "customShows".into(),
                expected: TypeName::Object,
            },
            prefix.to_string(),
        )
    })?;
    deny_keys_at(obj, prefix, &["name", "slides"])?;
    let name = match obj.get("name") {
        Some(Value::String(s)) => {
            custom_show::check_name(s, &format!("{prefix}.name"))?;
            s.clone()
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "name".into(),
                    expected: TypeName::String,
                },
                format!("{prefix}.name"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "name".into(),
                },
                format!("{prefix}.name"),
            ));
        }
    };
    let slides = match obj.get("slides") {
        None => Vec::new(),
        Some(Value::Array(arr)) => {
            let mut out = Vec::new();
            for (j, item) in arr.iter().enumerate() {
                out.push(parse_create_index(
                    Some(item),
                    &format!("{prefix}.slides[{j}]"),
                    "slides",
                )?);
            }
            out
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "slides".into(),
                    expected: TypeName::Array,
                },
                format!("{prefix}.slides"),
            ));
        }
    };
    Ok(InsertCustomShowOp {
        name,
        slides,
        after: None,
    })
}

pub(crate) fn parse_create_handout(v: &Value, prefix: &str) -> Result<SetHandoutTextOp, Error> {
    let obj = v.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "handout".into(),
                expected: TypeName::Object,
            },
            prefix.to_string(),
        )
    })?;
    deny_keys_at(obj, prefix, &["header", "footer", "date", "sldNum"])?;
    let header = parse_create_opt_string(obj, prefix, "header")?;
    let footer = parse_create_opt_string(obj, prefix, "footer")?;
    let date = parse_create_opt_string(obj, prefix, "date")?;
    let sld_num = parse_create_opt_string(obj, prefix, "sldNum")?;
    if header.is_none() && footer.is_none() && date.is_none() && sld_num.is_none() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "create".into(),
                field: "header, footer, date, or sldNum".into(),
            },
            prefix.to_string(),
        ));
    }
    Ok(SetHandoutTextOp {
        header,
        footer,
        date,
        sld_num,
    })
}

pub(crate) fn parse_create_show_pr(
    v: &Value,
    prefix: &str,
) -> Result<super::super::show::SetShowPrOp, Error> {
    let obj = v.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "showPr".into(),
                expected: TypeName::Object,
            },
            prefix.to_string(),
        )
    })?;
    deny_keys_at(
        obj,
        prefix,
        &[
            "type",
            "loop",
            "showNarration",
            "showAnimation",
            "useTimings",
        ],
    )?;
    let type_name = match obj.get("type") {
        None => None,
        Some(Value::String(s)) if matches!(s.as_str(), "present" | "browse" | "kiosk") => {
            Some(s.clone())
        }
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidShowPrType,
                format!("{prefix}.type"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "type".into(),
                    expected: TypeName::String,
                },
                format!("{prefix}.type"),
            ));
        }
    };
    let loop_ = parse_create_opt_bool(obj, prefix, "loop")?;
    let show_narration = parse_create_opt_bool(obj, prefix, "showNarration")?;
    let show_animation = parse_create_opt_bool(obj, prefix, "showAnimation")?;
    let use_timings = parse_create_opt_bool(obj, prefix, "useTimings")?;
    if type_name.is_none()
        && loop_.is_none()
        && show_narration.is_none()
        && show_animation.is_none()
        && use_timings.is_none()
    {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "create".into(),
                field: "type".into(),
            },
            prefix.to_string(),
        ));
    }
    Ok(super::super::show::SetShowPrOp {
        type_name,
        loop_,
        show_narration,
        show_animation,
        use_timings,
    })
}

pub(crate) fn parse_create_protection(
    v: &Value,
    prefix: &str,
) -> Result<super::super::protect::SetProtectionOp, Error> {
    let obj = v.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "modifyProtection".into(),
                expected: TypeName::Object,
            },
            prefix.to_string(),
        )
    })?;
    deny_keys_at(obj, prefix, &["enforced", "password"])?;
    let enforced = match obj.get("enforced") {
        Some(Value::Bool(b)) => *b,
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "enforced".into(),
                    expected: TypeName::Boolean,
                },
                format!("{prefix}.enforced"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::NeedProtectionField,
                format!("{prefix}.enforced"),
            ));
        }
    };
    let password = match obj.get("password") {
        None => None,
        Some(Value::String(s)) if !s.is_empty() => Some(s.clone()),
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::ProtectionPasswordBlank,
                format!("{prefix}.password"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "password".into(),
                    expected: TypeName::String,
                },
                format!("{prefix}.password"),
            ));
        }
    };
    Ok(super::super::protect::SetProtectionOp { enforced, password })
}

pub(super) fn parse_create_opt_string(
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

pub(super) fn parse_create_opt_bool(
    obj: &Map<String, Value>,
    prefix: &str,
    field: &str,
) -> Result<Option<bool>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::Bool(b)) => Ok(Some(*b)),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Boolean,
            },
            format!("{prefix}.{field}"),
        )),
    }
}
