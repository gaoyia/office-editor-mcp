use serde_json::{Map, Value};

use crate::error::{Error, ErrorKind, TypeName};

use super::{InsertConnectorOp, InsertShapeOp, InsertSmartArtOp, InsertTextBoxOp};

pub(crate) fn parse_textboxes(
    v: Option<&Value>,
    sheet_i: usize,
    sheet: &str,
) -> Result<Vec<InsertTextBoxOp>, Error> {
    let prefix = format!("workbook.sheets[{sheet_i}].textBoxes");
    let Some(arr) = super::create_util::parse_object_array(v, "textBoxes", &prefix)? else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for (j, item) in arr.iter().enumerate() {
        out.push(parse_textbox(item, &format!("{prefix}[{j}]"), sheet)?);
    }
    Ok(out)
}

pub(crate) fn parse_textbox(
    v: &Value,
    prefix: &str,
    sheet: &str,
) -> Result<InsertTextBoxOp, Error> {
    let obj = super::create_util::require_object(v, "textBoxes", prefix)?;
    super::create_util::deny_unknown(
        obj,
        prefix,
        &[
            "text",
            "cx",
            "cy",
            "from",
            "to",
            "anchor",
            "fromOffX",
            "fromOffY",
            "toOffX",
            "toOffY",
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
        ],
    )?;
    let a = super::create_util::parse_create_anchor(obj, prefix)?;
    Ok(InsertTextBoxOp {
        sheet: sheet.to_string(),
        text: super::create_util::parse_opt_text_field(obj, prefix)?,
        cx: a.cx,
        cy: a.cy,
        from: a.from,
        to: a.to,
        anchor: a.anchor,
        from_off_x: a.from_off_x,
        from_off_y: a.from_off_y,
        to_off_x: a.to_off_x,
        to_off_y: a.to_off_y,
        format: crate::drawing_text::parse(obj, prefix, false)?,
        rel: None,
    })
}

pub(crate) fn parse_shapes(
    v: Option<&Value>,
    sheet_i: usize,
    sheet: &str,
) -> Result<Vec<InsertShapeOp>, Error> {
    let prefix = format!("workbook.sheets[{sheet_i}].shapes");
    let Some(arr) = super::create_util::parse_object_array(v, "shapes", &prefix)? else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for (j, item) in arr.iter().enumerate() {
        out.push(parse_shape(item, &format!("{prefix}[{j}]"), sheet)?);
    }
    Ok(out)
}

pub(crate) fn parse_shape(v: &Value, prefix: &str, sheet: &str) -> Result<InsertShapeOp, Error> {
    let obj = super::create_util::require_object(v, "shapes", prefix)?;
    super::create_util::deny_unknown(
        obj,
        prefix,
        &[
            "text",
            "cx",
            "cy",
            "from",
            "to",
            "anchor",
            "fromOffX",
            "fromOffY",
            "toOffX",
            "toOffY",
            "prst",
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
        ],
    )?;
    let a = super::create_util::parse_create_anchor(obj, prefix)?;
    Ok(InsertShapeOp {
        sheet: sheet.to_string(),
        text: super::create_util::parse_opt_text_field(obj, prefix)?,
        cx: a.cx,
        cy: a.cy,
        from: a.from,
        to: a.to,
        anchor: a.anchor,
        from_off_x: a.from_off_x,
        from_off_y: a.from_off_y,
        to_off_x: a.to_off_x,
        to_off_y: a.to_off_y,
        prst: parse_shape_prst(obj, prefix)?,
        format: crate::drawing_text::parse(obj, prefix, false)?,
        rel: None,
    })
}

pub(crate) fn parse_connectors(
    v: Option<&Value>,
    sheet_i: usize,
    sheet: &str,
) -> Result<Vec<InsertConnectorOp>, Error> {
    let prefix = format!("workbook.sheets[{sheet_i}].connectors");
    let Some(arr) = super::create_util::parse_object_array(v, "connectors", &prefix)? else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for (j, item) in arr.iter().enumerate() {
        out.push(parse_connector(item, &format!("{prefix}[{j}]"), sheet)?);
    }
    Ok(out)
}

pub(crate) fn parse_connector(
    v: &Value,
    prefix: &str,
    sheet: &str,
) -> Result<InsertConnectorOp, Error> {
    let obj = super::create_util::require_object(v, "connectors", prefix)?;
    super::create_util::deny_unknown(
        obj,
        prefix,
        &[
            "cx", "cy", "from", "to", "anchor", "fromOffX", "fromOffY", "toOffX", "toOffY", "prst",
        ],
    )?;
    let a = super::create_util::parse_create_anchor(obj, prefix)?;
    Ok(InsertConnectorOp {
        sheet: sheet.to_string(),
        cx: a.cx,
        cy: a.cy,
        from: a.from,
        to: a.to,
        anchor: a.anchor,
        from_off_x: a.from_off_x,
        from_off_y: a.from_off_y,
        to_off_x: a.to_off_x,
        to_off_y: a.to_off_y,
        prst: parse_connector_prst(obj, prefix)?,
        rel: None,
    })
}

pub(crate) fn parse_shape_prst(obj: &Map<String, Value>, prefix: &str) -> Result<String, Error> {
    match obj.get("prst") {
        None => Ok("rect".to_string()),
        Some(Value::String(s)) => {
            if crate::drawing_geom::is_writable_shape_prst(s) {
                Ok(s.clone())
            } else {
                Err(Error::with_path(
                    ErrorKind::InvalidShapePrst,
                    format!("{prefix}.prst"),
                ))
            }
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "prst".into(),
                expected: TypeName::String,
            },
            format!("{prefix}.prst"),
        )),
    }
}

pub(crate) fn parse_connector_prst(
    obj: &Map<String, Value>,
    prefix: &str,
) -> Result<String, Error> {
    match obj.get("prst") {
        None => Ok("straightConnector1".to_string()),
        Some(Value::String(s)) => {
            if crate::drawing_geom::is_writable_connector_prst(s) {
                Ok(s.clone())
            } else {
                Err(Error::with_path(
                    ErrorKind::InvalidConnectorPrst,
                    format!("{prefix}.prst"),
                ))
            }
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "prst".into(),
                expected: TypeName::String,
            },
            format!("{prefix}.prst"),
        )),
    }
}

pub(crate) fn parse_smartarts(
    v: Option<&Value>,
    sheet_i: usize,
    sheet: &str,
) -> Result<Vec<InsertSmartArtOp>, Error> {
    let prefix = format!("workbook.sheets[{sheet_i}].smartArts");
    let Some(arr) = super::create_util::parse_object_array(v, "smartArts", &prefix)? else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for (j, item) in arr.iter().enumerate() {
        out.push(parse_smartart(item, &format!("{prefix}[{j}]"), sheet)?);
    }
    Ok(out)
}

pub(crate) fn parse_smartart(
    v: &Value,
    prefix: &str,
    sheet: &str,
) -> Result<InsertSmartArtOp, Error> {
    let obj = super::create_util::require_object(v, "smartArts", prefix)?;
    super::create_util::deny_unknown(
        obj,
        prefix,
        &[
            "layout", "points", "cx", "cy", "from", "to", "anchor", "fromOffX", "fromOffY",
            "toOffX", "toOffY",
        ],
    )?;
    let (layout, points) = crate::smartart::parse_layout_and_points(obj, prefix)?;
    let a = super::create_util::parse_create_anchor(obj, prefix)?;
    Ok(InsertSmartArtOp {
        sheet: sheet.to_string(),
        layout,
        points,
        cx: a.cx,
        cy: a.cy,
        from: a.from,
        to: a.to,
        anchor: a.anchor,
        from_off_x: a.from_off_x,
        from_off_y: a.from_off_y,
        to_off_x: a.to_off_x,
        to_off_y: a.to_off_y,
    })
}
