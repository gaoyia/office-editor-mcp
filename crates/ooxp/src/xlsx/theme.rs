//! 工作簿 theme / themeOverride。解析和补丁在 `theme::ops`。

use serde_json::{Map, Value};

use crate::error::{Error, IdReason};
use crate::opc::Package;
use crate::theme::ops::ThemeHost;

use super::picture;

pub use crate::theme::ops::{
    CreateTheme, SetThemeColorsOp, SetThemeExtraSchemesOp, SetThemeFmtOp, SetThemeFontsOp,
    SetThemeObjectDefaultsOp, SetThemeOverrideOp,
};

const HOST: ThemeHost = ThemeHost {
    theme_part: "/xl/theme/theme1.xml",
    override_part: "/xl/theme/themeOverride1.xml",
    host_id: "xlsx:workbook",
    json_root: "workbook",
    id_reason: IdReason::SetThemeNeedsWorkbook,
    rel_target: picture::rel_target_from,
};

pub fn inspect(pkg: &Package, doc_part: &str) -> Result<Value, Error> {
    crate::theme::ops::inspect(pkg, doc_part)
}

pub fn inspect_override(pkg: &Package, doc_part: &str) -> Result<Value, Error> {
    crate::theme::ops::inspect_override(pkg, doc_part)
}

pub fn set_colors(
    pkg: &mut Package,
    doc_part: &str,
    op: &SetThemeColorsOp,
    op_index: usize,
) -> Result<(), Error> {
    crate::theme::ops::set_colors(pkg, doc_part, op, op_index)
}

pub fn set_fonts(
    pkg: &mut Package,
    doc_part: &str,
    op: &SetThemeFontsOp,
    op_index: usize,
) -> Result<(), Error> {
    crate::theme::ops::set_fonts(pkg, doc_part, op, op_index)
}

pub fn set_fmt(
    pkg: &mut Package,
    doc_part: &str,
    op: &SetThemeFmtOp,
    op_index: usize,
) -> Result<(), Error> {
    crate::theme::ops::set_fmt(pkg, doc_part, op, op_index)
}

pub fn set_extra_schemes(
    pkg: &mut Package,
    doc_part: &str,
    op: &SetThemeExtraSchemesOp,
    op_index: usize,
) -> Result<(), Error> {
    crate::theme::ops::set_extra_schemes(pkg, doc_part, op, op_index)
}

pub fn set_object_defaults(
    pkg: &mut Package,
    doc_part: &str,
    op: &SetThemeObjectDefaultsOp,
    op_index: usize,
) -> Result<(), Error> {
    crate::theme::ops::set_object_defaults(pkg, doc_part, op, op_index)
}

pub fn set_override(
    pkg: &mut Package,
    doc_part: &str,
    op: &SetThemeOverrideOp,
    op_index: usize,
) -> Result<(), Error> {
    crate::theme::ops::set_override(&HOST, pkg, doc_part, op, op_index)
}

pub fn parse_set_colors(obj: &Map<String, Value>, i: usize) -> Result<SetThemeColorsOp, Error> {
    crate::theme::ops::parse_set_colors(&HOST, obj, i)
}

pub fn parse_set_fonts(obj: &Map<String, Value>, i: usize) -> Result<SetThemeFontsOp, Error> {
    crate::theme::ops::parse_set_fonts(&HOST, obj, i)
}

pub fn parse_set_fmt(obj: &Map<String, Value>, i: usize) -> Result<SetThemeFmtOp, Error> {
    crate::theme::ops::parse_set_fmt(&HOST, obj, i)
}

pub fn parse_set_extra_schemes(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<SetThemeExtraSchemesOp, Error> {
    crate::theme::ops::parse_set_extra_schemes(&HOST, obj, i)
}

pub fn parse_set_object_defaults(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<SetThemeObjectDefaultsOp, Error> {
    crate::theme::ops::parse_set_object_defaults(&HOST, obj, i)
}

pub fn parse_set_override(obj: &Map<String, Value>, i: usize) -> Result<SetThemeOverrideOp, Error> {
    crate::theme::ops::parse_set_override(&HOST, obj, i)
}

pub fn parse_create(v: &Value) -> Result<CreateTheme, Error> {
    crate::theme::ops::parse_create(&HOST, v)
}

pub fn write_create(pkg: &mut Package, spec: &CreateTheme) -> Result<(), Error> {
    crate::theme::ops::write_create(&HOST, pkg, spec)
}
