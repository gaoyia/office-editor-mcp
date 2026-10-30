//! pptx 主题 op 里和宿主绑在一起的解析。配色、字体、fmt 字段走 `theme::ops`。

use serde_json::{Map, Value};

use crate::error::{Error, ErrorKind, TypeName};
use crate::theme::ops::{
    parse_extra_scheme_array, parse_fmt_fill, parse_fmt_line, parse_opt_null_name,
};

use super::common::*;
use super::types::*;

pub(super) fn parse_set_theme_fmt(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<SetThemeFmtOp, Error> {
    deny_keys(obj, i, &["op", "id", "line", "fill", "bgFill", "name"])?;
    let id = require_id(obj, i)?;
    let host = super::master::parse_theme_host(&id, i)?;
    let line = parse_fmt_line(obj.get("line"), i, "line")?;
    let fill = parse_fmt_fill(obj.get("fill"), i, "fill")?;
    let bg_fill = parse_fmt_fill(obj.get("bgFill"), i, "bgFill")?;
    let name = parse_opt_null_name(obj, i, "name")?;
    if line.is_none() && fill.is_none() && bg_fill.is_none() && name.is_none() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setThemeFmt".into(),
                field: "line, fill, bgFill, or name".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(SetThemeFmtOp {
        host,
        line,
        fill,
        bg_fill,
        name,
    })
}

pub(super) fn parse_set_theme_extra_schemes(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<SetThemeExtraSchemesOp, Error> {
    deny_keys(obj, i, &["op", "id", "extraSchemes"])?;
    let id = require_id(obj, i)?;
    let host = super::master::parse_theme_host(&id, i)?;
    let schemes = match obj.get("extraSchemes") {
        None => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "extraSchemes".into(),
                },
                format!("ops[{i}].extraSchemes"),
            ));
        }
        Some(Value::Null) => Vec::new(),
        Some(Value::Array(arr)) => {
            parse_extra_scheme_array(arr, &format!("ops[{i}].extraSchemes"))?
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "extraSchemes".into(),
                    expected: TypeName::Array,
                },
                format!("ops[{i}].extraSchemes"),
            ));
        }
    };
    Ok(SetThemeExtraSchemesOp { host, schemes })
}

pub(super) fn parse_set_theme_object_defaults(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<SetThemeObjectDefaultsOp, Error> {
    deny_keys(obj, i, &["op", "id", "objectDefaults"])?;
    let id = require_id(obj, i)?;
    let host = super::master::parse_theme_host(&id, i)?;
    let defaults = crate::theme::parse_object_defaults_field(obj.get("objectDefaults"), i)?;
    Ok(SetThemeObjectDefaultsOp { host, defaults })
}
