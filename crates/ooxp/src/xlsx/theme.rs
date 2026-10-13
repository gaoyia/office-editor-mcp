//! 工作簿 theme / themeOverride：inspect 与 apply。
//! 不算主题排版引擎。`setThemeColors` / `setThemeFonts` / `setThemeFmt` 不新建 theme。

use serde_json::{Map, Value};

use crate::error::{Error, ErrorKind, IdReason, TypeName};
use crate::opc::{self, ContentTypes, Package, Relationship, Relationships};
use crate::theme::{ExtraScheme, FmtFillKind, FontSlot, ScriptFont};
use crate::xml;

use super::picture;
use super::{CT_THEME, CT_THEME_OVERRIDE, REL_THEME, REL_THEME_OVERRIDE};

const THEME_PART: &str = "/xl/theme/theme1.xml";
const OVERRIDE_PART: &str = "/xl/theme/themeOverride1.xml";

#[derive(Debug, Clone)]
pub struct SetThemeColorsOp {
    pub slots: Vec<(String, String)>,
    pub name: Option<Option<String>>,
    pub theme_name: Option<Option<String>>,
}

#[derive(Debug, Clone)]
pub struct SetThemeFontsOp {
    pub fonts: Vec<(FontSlot, String)>,
    pub name: Option<Option<String>>,
    pub major_scripts: Option<Vec<ScriptFont>>,
    pub minor_scripts: Option<Vec<ScriptFont>>,
}

#[derive(Debug, Clone)]
pub struct SetThemeFmtOp {
    pub line: Option<(usize, u32)>,
    pub fill: Option<(usize, FmtFillKind)>,
    pub bg_fill: Option<(usize, FmtFillKind)>,
    pub name: Option<Option<String>>,
}

#[derive(Debug, Clone)]
pub struct SetThemeExtraSchemesOp {
    pub schemes: Vec<ExtraScheme>,
}

#[derive(Debug, Clone)]
pub struct SetThemeOverrideOp {
    pub enabled: bool,
    pub slots: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub struct CreateTheme {
    pub name: String,
    pub slots: Vec<(String, String)>,
    pub fonts: Vec<(FontSlot, String)>,
    pub colors_name: Option<String>,
    pub fonts_name: Option<String>,
    pub fmt_name: Option<String>,
    pub major_scripts: Option<Vec<ScriptFont>>,
    pub minor_scripts: Option<Vec<ScriptFont>>,
    pub extra_schemes: Option<Vec<ExtraScheme>>,
}

struct NamedColorSlots {
    name: Option<String>,
    slots: Vec<(String, String)>,
}

struct CreateFonts {
    name: Option<String>,
    fonts: Vec<(FontSlot, String)>,
    major_scripts: Option<Vec<ScriptFont>>,
    minor_scripts: Option<Vec<ScriptFont>>,
}

pub fn inspect(pkg: &Package, doc_part: &str) -> Result<Value, Error> {
    let Some(part) = theme_part(pkg, doc_part)? else {
        return Ok(Value::Null);
    };
    let doc = xml::parse(pkg.get(&part)?)?;
    Ok(crate::theme::inspect_full(&doc.root))
}

pub fn inspect_override(pkg: &Package, doc_part: &str) -> Result<Value, Error> {
    let Some(part) = override_part(pkg, doc_part)? else {
        return Ok(Value::Null);
    };
    let doc = xml::parse(pkg.get(&part)?)?;
    Ok(crate::theme::inspect_override_full(&doc.root))
}

pub fn set_colors(
    pkg: &mut Package,
    doc_part: &str,
    op: &SetThemeColorsOp,
    op_index: usize,
) -> Result<(), Error> {
    let json_path = format!("ops[{op_index}].id");
    let part = theme_part(pkg, doc_part)?
        .ok_or_else(|| Error::with_path(ErrorKind::DocumentThemeNotFound, json_path.clone()))?;
    let mut doc = xml::parse(pkg.get(&part)?)?;
    if !op.slots.is_empty() && !crate::theme::patch_colors(&mut doc.root, &op.slots) {
        return Err(Error::with_path(
            ErrorKind::DocumentThemeNotFound,
            json_path.clone(),
        ));
    }
    if let Some(name) = &op.name {
        if !crate::theme::patch_clr_scheme_name(&mut doc.root, name.as_deref()) {
            return Err(Error::with_path(
                ErrorKind::DocumentThemeNotFound,
                json_path.clone(),
            ));
        }
    }
    if let Some(name) = &op.theme_name {
        crate::theme::patch_theme_name(&mut doc.root, name.as_deref());
    }
    pkg.set(&part, doc.to_bytes()?)?;
    Ok(())
}

pub fn set_fonts(
    pkg: &mut Package,
    doc_part: &str,
    op: &SetThemeFontsOp,
    op_index: usize,
) -> Result<(), Error> {
    let json_path = format!("ops[{op_index}].id");
    let part = theme_part(pkg, doc_part)?
        .ok_or_else(|| Error::with_path(ErrorKind::DocumentThemeNotFound, json_path.clone()))?;
    let mut doc = xml::parse(pkg.get(&part)?)?;
    crate::theme::patch_fonts(&mut doc.root, &op.fonts, &json_path)?;
    if let Some(name) = &op.name {
        if !crate::theme::patch_font_scheme_name(&mut doc.root, name.as_deref()) {
            return Err(Error::with_path(
                ErrorKind::DocumentThemeNotFound,
                json_path.clone(),
            ));
        }
    }
    if let Some(fonts) = &op.major_scripts {
        crate::theme::replace_script_fonts(&mut doc.root, true, fonts, &json_path)?;
    }
    if let Some(fonts) = &op.minor_scripts {
        crate::theme::replace_script_fonts(&mut doc.root, false, fonts, &json_path)?;
    }
    pkg.set(&part, doc.to_bytes()?)?;
    Ok(())
}

pub fn set_fmt(
    pkg: &mut Package,
    doc_part: &str,
    op: &SetThemeFmtOp,
    op_index: usize,
) -> Result<(), Error> {
    let json_path = format!("ops[{op_index}].id");
    let part = theme_part(pkg, doc_part)?
        .ok_or_else(|| Error::with_path(ErrorKind::DocumentThemeNotFound, json_path.clone()))?;
    let mut doc = xml::parse(pkg.get(&part)?)?;
    if let Some((index, w)) = op.line {
        if !crate::theme::patch_line_width(&mut doc.root, index, w) {
            return Err(Error::with_path(
                ErrorKind::InvalidThemeFmtIndex,
                format!("ops[{op_index}].line.index"),
            ));
        }
    }
    if let Some((index, kind)) = op.fill {
        if !crate::theme::patch_fill_slot(&mut doc.root, false, index, kind) {
            return Err(Error::with_path(
                ErrorKind::InvalidThemeFmtIndex,
                format!("ops[{op_index}].fill.index"),
            ));
        }
    }
    if let Some((index, kind)) = op.bg_fill {
        if !crate::theme::patch_fill_slot(&mut doc.root, true, index, kind) {
            return Err(Error::with_path(
                ErrorKind::InvalidThemeFmtIndex,
                format!("ops[{op_index}].bgFill.index"),
            ));
        }
    }
    if let Some(name) = &op.name {
        if !crate::theme::patch_fmt_scheme_name(&mut doc.root, name.as_deref()) {
            return Err(Error::with_path(
                ErrorKind::InvalidThemeFmtIndex,
                format!("ops[{op_index}].name"),
            ));
        }
    }
    pkg.set(&part, doc.to_bytes()?)?;
    Ok(())
}

pub fn set_extra_schemes(
    pkg: &mut Package,
    doc_part: &str,
    op: &SetThemeExtraSchemesOp,
    op_index: usize,
) -> Result<(), Error> {
    let json_path = format!("ops[{op_index}].id");
    let part = theme_part(pkg, doc_part)?
        .ok_or_else(|| Error::with_path(ErrorKind::DocumentThemeNotFound, json_path))?;
    let mut doc = xml::parse(pkg.get(&part)?)?;
    crate::theme::replace_extra_schemes(&mut doc.root, &op.schemes);
    pkg.set(&part, doc.to_bytes()?)?;
    Ok(())
}

pub fn set_override(
    pkg: &mut Package,
    doc_part: &str,
    op: &SetThemeOverrideOp,
    _op_index: usize,
) -> Result<(), Error> {
    if !op.enabled {
        remove_override(pkg, doc_part)?;
        return Ok(());
    }
    let part = match override_part(pkg, doc_part)? {
        Some(p) => p,
        None => create_override_part(pkg, doc_part)?,
    };
    let mut doc = xml::parse(pkg.get(&part)?)?;
    crate::theme::patch_override_colors(&mut doc.root, &op.slots);
    pkg.set(&part, doc.to_bytes()?)?;
    Ok(())
}

pub fn parse_set_colors(obj: &Map<String, Value>, i: usize) -> Result<SetThemeColorsOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op",
            "id",
            "name",
            "themeName",
            "dk1",
            "lt1",
            "dk2",
            "lt2",
            "accent1",
            "accent2",
            "accent3",
            "accent4",
            "accent5",
            "accent6",
            "hlink",
            "folHlink",
        ],
    )?;
    require_workbook_id(obj, i)?;
    let slots = parse_color_slots(obj, i)?;
    let name = parse_opt_null_name(obj, i, "name")?;
    let theme_name = parse_opt_null_name(obj, i, "themeName")?;
    if slots.is_empty() && name.is_none() && theme_name.is_none() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setThemeColors".into(),
                field: "a color slot, name, or themeName".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(SetThemeColorsOp {
        slots,
        name,
        theme_name,
    })
}

pub fn parse_set_fonts(obj: &Map<String, Value>, i: usize) -> Result<SetThemeFontsOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op",
            "id",
            "name",
            "majorLatin",
            "minorLatin",
            "majorEa",
            "minorEa",
            "majorCs",
            "minorCs",
            "majorScripts",
            "minorScripts",
        ],
    )?;
    require_workbook_id(obj, i)?;
    let fonts = parse_font_slots(obj, i)?;
    let name = parse_opt_null_name(obj, i, "name")?;
    let major_scripts = parse_opt_scripts(obj, i, "majorScripts")?;
    let minor_scripts = parse_opt_scripts(obj, i, "minorScripts")?;
    if fonts.is_empty() && name.is_none() && major_scripts.is_none() && minor_scripts.is_none() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setThemeFonts".into(),
                field: "a font slot, name, majorScripts, or minorScripts".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(SetThemeFontsOp {
        fonts,
        name,
        major_scripts,
        minor_scripts,
    })
}

pub fn parse_set_fmt(obj: &Map<String, Value>, i: usize) -> Result<SetThemeFmtOp, Error> {
    deny_keys(obj, i, &["op", "id", "line", "fill", "bgFill", "name"])?;
    require_workbook_id(obj, i)?;
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
        line,
        fill,
        bg_fill,
        name,
    })
}

pub fn parse_set_extra_schemes(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<SetThemeExtraSchemesOp, Error> {
    deny_keys(obj, i, &["op", "id", "extraSchemes"])?;
    require_workbook_id(obj, i)?;
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
    Ok(SetThemeExtraSchemesOp { schemes })
}

pub fn parse_set_override(obj: &Map<String, Value>, i: usize) -> Result<SetThemeOverrideOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op", "id", "enabled", "dk1", "lt1", "dk2", "lt2", "accent1", "accent2", "accent3",
            "accent4", "accent5", "accent6", "hlink", "folHlink",
        ],
    )?;
    require_workbook_id(obj, i)?;
    let slots = parse_color_slots(obj, i)?;
    let enabled = match obj.get("enabled") {
        None => true,
        Some(Value::Bool(false)) => false,
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::NeedThemeOverrideField,
                format!("ops[{i}].enabled"),
            ));
        }
    };
    if !enabled && !slots.is_empty() {
        return Err(Error::with_path(
            ErrorKind::MutuallyExclusive {
                a: "enabled".into(),
                b: "color slots".into(),
            },
            format!("ops[{i}].enabled"),
        ));
    }
    if enabled && slots.is_empty() {
        return Err(Error::with_path(
            ErrorKind::NeedThemeOverrideField,
            format!("ops[{i}]"),
        ));
    }
    Ok(SetThemeOverrideOp { enabled, slots })
}

pub fn parse_create(v: &Value) -> Result<CreateTheme, Error> {
    let obj = v.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "theme".into(),
                expected: TypeName::Object,
            },
            "workbook.theme",
        )
    })?;
    for k in obj.keys() {
        if !matches!(
            k.as_str(),
            "name" | "colors" | "fonts" | "fmt" | "extraSchemes"
        ) {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("workbook.theme.{k}"),
            ));
        }
    }
    let name = match obj.get("name") {
        None => "Office Theme".into(),
        Some(Value::String(s)) if !s.is_empty() => s.clone(),
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "name".into(),
                    expected: TypeName::NonEmptyString,
                },
                "workbook.theme.name",
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "name".into(),
                    expected: TypeName::String,
                },
                "workbook.theme.name",
            ));
        }
    };
    let (colors_name, slots) = match obj.get("colors") {
        None => (None, Vec::new()),
        Some(Value::Object(m)) => {
            let parsed = parse_create_colors(m)?;
            (parsed.name, parsed.slots)
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "colors".into(),
                    expected: TypeName::Object,
                },
                "workbook.theme.colors",
            ));
        }
    };
    let (fonts_name, fonts, major_scripts, minor_scripts) = match obj.get("fonts") {
        None => (None, Vec::new(), None, None),
        Some(Value::Object(m)) => {
            let parsed = parse_create_fonts(m)?;
            (
                parsed.name,
                parsed.fonts,
                parsed.major_scripts,
                parsed.minor_scripts,
            )
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "fonts".into(),
                    expected: TypeName::Object,
                },
                "workbook.theme.fonts",
            ));
        }
    };
    let fmt_name = match obj.get("fmt") {
        None => None,
        Some(Value::Object(m)) => parse_create_fmt_name(m)?,
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "fmt".into(),
                    expected: TypeName::Object,
                },
                "workbook.theme.fmt",
            ));
        }
    };
    let extra_schemes = match obj.get("extraSchemes") {
        None => None,
        Some(Value::Null) => Some(Vec::new()),
        Some(Value::Array(arr)) => Some(parse_extra_scheme_array(
            arr,
            "workbook.theme.extraSchemes",
        )?),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "extraSchemes".into(),
                    expected: TypeName::Array,
                },
                "workbook.theme.extraSchemes",
            ));
        }
    };
    Ok(CreateTheme {
        name,
        slots,
        fonts,
        colors_name,
        fonts_name,
        fmt_name,
        major_scripts,
        minor_scripts,
        extra_schemes,
    })
}

pub fn write_create(pkg: &mut Package, spec: &CreateTheme) -> Result<(), Error> {
    let doc_part = opc::office_document_part(pkg)?;
    pkg.set(
        THEME_PART,
        crate::theme::default_theme_xml(&spec.name).into_bytes(),
    )?;
    let mut doc = xml::parse(pkg.get(THEME_PART)?)?;
    if !spec.slots.is_empty() {
        crate::theme::patch_colors(&mut doc.root, &spec.slots);
    }
    if let Some(name) = &spec.colors_name {
        crate::theme::patch_clr_scheme_name(&mut doc.root, Some(name));
    }
    if !spec.fonts.is_empty() {
        crate::theme::patch_fonts(&mut doc.root, &spec.fonts, "workbook.theme.fonts")?;
    }
    if let Some(name) = &spec.fonts_name {
        crate::theme::patch_fonts(&mut doc.root, &[], "workbook.theme.fonts")?;
        crate::theme::patch_font_scheme_name(&mut doc.root, Some(name));
    }
    if let Some(fonts) = &spec.major_scripts {
        crate::theme::replace_script_fonts(
            &mut doc.root,
            true,
            fonts,
            "workbook.theme.fonts.majorScripts",
        )?;
    }
    if let Some(fonts) = &spec.minor_scripts {
        crate::theme::replace_script_fonts(
            &mut doc.root,
            false,
            fonts,
            "workbook.theme.fonts.minorScripts",
        )?;
    }
    if let Some(name) = &spec.fmt_name {
        crate::theme::patch_fmt_scheme_name(&mut doc.root, Some(name));
    }
    if let Some(schemes) = &spec.extra_schemes {
        crate::theme::replace_extra_schemes(&mut doc.root, schemes);
    }
    pkg.set(THEME_PART, doc.to_bytes()?)?;
    let mut rels = Relationships::from_package(pkg, &doc_part)?;
    if rels.find_by_type_suffix("/theme").is_empty() {
        rels.push(Relationship {
            id: rels.next_rid(),
            rel_type: REL_THEME.into(),
            target: picture::rel_target_from(&doc_part, THEME_PART),
            target_mode: None,
            extra: Vec::new(),
        });
        rels.write_to(pkg, &doc_part)?;
    }
    let mut cts = ContentTypes::from_package(pkg)?;
    cts.set_override(THEME_PART, CT_THEME)?;
    cts.write_to(pkg)?;
    Ok(())
}

fn parse_create_colors(obj: &Map<String, Value>) -> Result<NamedColorSlots, Error> {
    for k in obj.keys() {
        if k != "name" && !crate::theme::COLOR_SLOTS.contains(&k.as_str()) {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("workbook.theme.colors.{k}"),
            ));
        }
    }
    let colors_name = match obj.get("name") {
        None | Some(Value::Null) => None,
        Some(Value::String(s)) if !s.is_empty() => Some(s.clone()),
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "name".into(),
                    expected: TypeName::NonEmptyString,
                },
                "workbook.theme.colors.name",
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "name".into(),
                    expected: TypeName::String,
                },
                "workbook.theme.colors.name",
            ));
        }
    };
    let mut slots = Vec::new();
    for slot in crate::theme::COLOR_SLOTS {
        if let Some(v) = obj.get(*slot) {
            slots.push((
                (*slot).to_string(),
                parse_hex_at(v, &format!("workbook.theme.colors.{slot}"))?,
            ));
        }
    }
    Ok(NamedColorSlots {
        name: colors_name,
        slots,
    })
}

fn parse_create_fonts(obj: &Map<String, Value>) -> Result<CreateFonts, Error> {
    for k in obj.keys() {
        if !matches!(
            k.as_str(),
            "name"
                | "majorLatin"
                | "minorLatin"
                | "majorEa"
                | "minorEa"
                | "majorCs"
                | "minorCs"
                | "majorScripts"
                | "minorScripts"
        ) {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("workbook.theme.fonts.{k}"),
            ));
        }
    }
    let fonts_name = match obj.get("name") {
        None | Some(Value::Null) => None,
        Some(Value::String(s)) if !s.is_empty() => Some(s.clone()),
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "name".into(),
                    expected: TypeName::NonEmptyString,
                },
                "workbook.theme.fonts.name",
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "name".into(),
                    expected: TypeName::String,
                },
                "workbook.theme.fonts.name",
            ));
        }
    };
    let mut fonts = Vec::new();
    if let Some(s) = parse_face_at(obj, "majorLatin", false, "workbook.theme.fonts")? {
        fonts.push((FontSlot::MajorLatin, s));
    }
    if let Some(s) = parse_face_at(obj, "minorLatin", false, "workbook.theme.fonts")? {
        fonts.push((FontSlot::MinorLatin, s));
    }
    if let Some(s) = parse_face_at(obj, "majorEa", true, "workbook.theme.fonts")? {
        fonts.push((FontSlot::MajorEa, s));
    }
    if let Some(s) = parse_face_at(obj, "minorEa", true, "workbook.theme.fonts")? {
        fonts.push((FontSlot::MinorEa, s));
    }
    if let Some(s) = parse_face_at(obj, "majorCs", true, "workbook.theme.fonts")? {
        fonts.push((FontSlot::MajorCs, s));
    }
    if let Some(s) = parse_face_at(obj, "minorCs", true, "workbook.theme.fonts")? {
        fonts.push((FontSlot::MinorCs, s));
    }
    let major_scripts =
        parse_scripts_at(obj.get("majorScripts"), "workbook.theme.fonts.majorScripts")?;
    let minor_scripts =
        parse_scripts_at(obj.get("minorScripts"), "workbook.theme.fonts.minorScripts")?;
    Ok(CreateFonts {
        name: fonts_name,
        fonts,
        major_scripts,
        minor_scripts,
    })
}

fn parse_create_fmt_name(obj: &Map<String, Value>) -> Result<Option<String>, Error> {
    for k in obj.keys() {
        if k != "name" {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("workbook.theme.fmt.{k}"),
            ));
        }
    }
    match obj.get("name") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) if !s.is_empty() => Ok(Some(s.clone())),
        Some(Value::String(_)) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "name".into(),
                expected: TypeName::NonEmptyString,
            },
            "workbook.theme.fmt.name",
        )),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "name".into(),
                expected: TypeName::String,
            },
            "workbook.theme.fmt.name",
        )),
    }
}

fn parse_face_at(
    obj: &Map<String, Value>,
    field: &str,
    allow_empty: bool,
    base: &str,
) -> Result<Option<String>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::String(s)) if !s.is_empty() || allow_empty => Ok(Some(s.clone())),
        Some(Value::String(_)) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::NonEmptyString,
            },
            format!("{base}.{field}"),
        )),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::String,
            },
            format!("{base}.{field}"),
        )),
    }
}

fn parse_color_slots(obj: &Map<String, Value>, i: usize) -> Result<Vec<(String, String)>, Error> {
    let mut slots = Vec::new();
    for slot in crate::theme::COLOR_SLOTS {
        if let Some(v) = obj.get(*slot) {
            slots.push(((*slot).to_string(), parse_hex_slot(v, i, slot)?));
        }
    }
    Ok(slots)
}

fn parse_font_slots(obj: &Map<String, Value>, i: usize) -> Result<Vec<(FontSlot, String)>, Error> {
    let mut fonts = Vec::new();
    if let Some(s) = parse_theme_face(obj, i, "majorLatin", false)? {
        fonts.push((FontSlot::MajorLatin, s));
    }
    if let Some(s) = parse_theme_face(obj, i, "minorLatin", false)? {
        fonts.push((FontSlot::MinorLatin, s));
    }
    if let Some(s) = parse_theme_face(obj, i, "majorEa", true)? {
        fonts.push((FontSlot::MajorEa, s));
    }
    if let Some(s) = parse_theme_face(obj, i, "minorEa", true)? {
        fonts.push((FontSlot::MinorEa, s));
    }
    if let Some(s) = parse_theme_face(obj, i, "majorCs", true)? {
        fonts.push((FontSlot::MajorCs, s));
    }
    if let Some(s) = parse_theme_face(obj, i, "minorCs", true)? {
        fonts.push((FontSlot::MinorCs, s));
    }
    Ok(fonts)
}

fn parse_fmt_line(v: Option<&Value>, i: usize, field: &str) -> Result<Option<(usize, u32)>, Error> {
    let Some(v) = v else {
        return Ok(None);
    };
    let obj = v.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Object,
            },
            format!("ops[{i}].{field}"),
        )
    })?;
    for k in obj.keys() {
        if !matches!(k.as_str(), "index" | "w") {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("ops[{i}].{field}.{k}"),
            ));
        }
    }
    let index = parse_fmt_index(obj.get("index"), i, &format!("{field}.index"))?;
    let w = parse_fmt_w(obj.get("w"), i, &format!("{field}.w"))?;
    Ok(Some((index, w)))
}

fn parse_fmt_fill(
    v: Option<&Value>,
    i: usize,
    field: &str,
) -> Result<Option<(usize, FmtFillKind)>, Error> {
    let Some(v) = v else {
        return Ok(None);
    };
    let obj = v.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Object,
            },
            format!("ops[{i}].{field}"),
        )
    })?;
    for k in obj.keys() {
        if !matches!(k.as_str(), "index" | "kind") {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("ops[{i}].{field}.{k}"),
            ));
        }
    }
    let index = parse_fmt_index(obj.get("index"), i, &format!("{field}.index"))?;
    let kind = match obj.get("kind") {
        Some(Value::String(s)) if s == "solid" => FmtFillKind::Solid,
        Some(Value::String(s)) if s == "grad" => FmtFillKind::Grad,
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidThemeFmtKind,
                format!("ops[{i}].{field}.kind"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: format!("{field}.kind"),
                    expected: TypeName::String,
                },
                format!("ops[{i}].{field}.kind"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::OpRequiresField {
                    op: "setThemeFmt".into(),
                    field: "kind".into(),
                },
                format!("ops[{i}].{field}.kind"),
            ));
        }
    };
    Ok(Some((index, kind)))
}

fn parse_fmt_index(v: Option<&Value>, i: usize, field: &str) -> Result<usize, Error> {
    match v {
        Some(Value::Number(n)) => {
            if let Some(u) = n.as_u64() {
                if u <= 2 {
                    return Ok(u as usize);
                }
                return Err(Error::with_path(
                    ErrorKind::InvalidThemeFmtIndex,
                    format!("ops[{i}].{field}"),
                ));
            }
            Err(Error::with_path(
                ErrorKind::WrongType {
                    field: field.into(),
                    expected: TypeName::Integer,
                },
                format!("ops[{i}].{field}"),
            ))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Integer,
            },
            format!("ops[{i}].{field}"),
        )),
        None => Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setThemeFmt".into(),
                field: "index".into(),
            },
            format!("ops[{i}].{field}"),
        )),
    }
}

fn parse_fmt_w(v: Option<&Value>, i: usize, field: &str) -> Result<u32, Error> {
    match v {
        Some(Value::Number(n)) => {
            if let Some(u) = n.as_u64() {
                if u > u32::MAX as u64 {
                    return Err(Error::with_path(
                        ErrorKind::MinValue {
                            field: field.into(),
                            min: 0,
                        },
                        format!("ops[{i}].{field}"),
                    ));
                }
                return Ok(u as u32);
            }
            if n.as_i64().is_some_and(|x| x < 0) {
                return Err(Error::with_path(
                    ErrorKind::MinValue {
                        field: field.into(),
                        min: 0,
                    },
                    format!("ops[{i}].{field}"),
                ));
            }
            Err(Error::with_path(
                ErrorKind::WrongType {
                    field: field.into(),
                    expected: TypeName::Integer,
                },
                format!("ops[{i}].{field}"),
            ))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Integer,
            },
            format!("ops[{i}].{field}"),
        )),
        None => Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setThemeFmt".into(),
                field: "w".into(),
            },
            format!("ops[{i}].{field}"),
        )),
    }
}

fn require_workbook_id(obj: &Map<String, Value>, i: usize) -> Result<(), Error> {
    let id = match obj.get("id") {
        Some(Value::String(s)) if !s.is_empty() => s.as_str(),
        Some(Value::String(_)) | None => {
            return Err(Error::with_path(
                ErrorKind::MissingField { field: "id".into() },
                format!("ops[{i}].id"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "id".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].id"),
            ));
        }
    };
    if id != "xlsx:workbook" {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SetThemeNeedsWorkbook,
            },
            format!("ops[{i}].id"),
        ));
    }
    Ok(())
}

fn parse_hex_slot(v: &Value, i: usize, field: &str) -> Result<String, Error> {
    parse_hex_at(v, &format!("ops[{i}].{field}"))
}

fn parse_hex_at(v: &Value, path: &str) -> Result<String, Error> {
    match v {
        Value::String(s) if s.len() == 6 && s.chars().all(|c| c.is_ascii_hexdigit()) => {
            Ok(s.clone())
        }
        Value::String(_) => Err(Error::with_path(
            ErrorKind::InvalidHexColor,
            path.to_string(),
        )),
        _ => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "color".into(),
                expected: TypeName::String,
            },
            path.to_string(),
        )),
    }
}

fn parse_theme_face(
    obj: &Map<String, Value>,
    i: usize,
    field: &str,
    allow_empty: bool,
) -> Result<Option<String>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::String(s)) if !s.is_empty() || allow_empty => Ok(Some(s.clone())),
        Some(Value::String(_)) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::NonEmptyString,
            },
            format!("ops[{i}].{field}"),
        )),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].{field}"),
        )),
    }
}

fn parse_opt_null_name(
    obj: &Map<String, Value>,
    i: usize,
    field: &str,
) -> Result<Option<Option<String>>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(None)),
        Some(Value::String(s)) if !s.is_empty() => Ok(Some(Some(s.clone()))),
        Some(Value::String(_)) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::NonEmptyString,
            },
            format!("ops[{i}].{field}"),
        )),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].{field}"),
        )),
    }
}

fn parse_opt_scripts(
    obj: &Map<String, Value>,
    i: usize,
    field: &str,
) -> Result<Option<Vec<ScriptFont>>, Error> {
    parse_scripts_at(obj.get(field), &format!("ops[{i}].{field}"))
}

fn parse_scripts_at(v: Option<&Value>, path: &str) -> Result<Option<Vec<ScriptFont>>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(Vec::new())),
        Some(Value::Array(arr)) => {
            let mut out = Vec::with_capacity(arr.len());
            for (j, item) in arr.iter().enumerate() {
                out.push(parse_script_item(item, &format!("{path}[{j}]"))?);
            }
            Ok(Some(out))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "scripts".into(),
                expected: TypeName::Array,
            },
            path.to_string(),
        )),
    }
}

fn parse_script_item(v: &Value, path: &str) -> Result<ScriptFont, Error> {
    let obj = v.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "script font".into(),
                expected: TypeName::Object,
            },
            path.to_string(),
        )
    })?;
    for k in obj.keys() {
        if !matches!(k.as_str(), "script" | "typeface") {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("{path}.{k}"),
            ));
        }
    }
    let script = match obj.get("script") {
        Some(Value::String(s)) if !s.is_empty() => s.clone(),
        Some(Value::String(_)) | None => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "script".into(),
                    expected: TypeName::NonEmptyString,
                },
                format!("{path}.script"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "script".into(),
                    expected: TypeName::String,
                },
                format!("{path}.script"),
            ));
        }
    };
    let typeface = match obj.get("typeface") {
        Some(Value::String(s)) => s.clone(),
        None => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "typeface".into(),
                },
                format!("{path}.typeface"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "typeface".into(),
                    expected: TypeName::String,
                },
                format!("{path}.typeface"),
            ));
        }
    };
    Ok(ScriptFont { script, typeface })
}

fn parse_extra_scheme_array(arr: &[Value], path: &str) -> Result<Vec<ExtraScheme>, Error> {
    let mut out = Vec::with_capacity(arr.len());
    for (j, item) in arr.iter().enumerate() {
        out.push(parse_extra_scheme_item(item, &format!("{path}[{j}]"))?);
    }
    Ok(out)
}

fn parse_extra_scheme_item(v: &Value, path: &str) -> Result<ExtraScheme, Error> {
    let obj = v.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "extraSchemes[]".into(),
                expected: TypeName::Object,
            },
            path.to_string(),
        )
    })?;
    for k in obj.keys() {
        if !matches!(k.as_str(), "name" | "colors") {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("{path}.{k}"),
            ));
        }
    }
    let item_name = parse_scheme_name(obj.get("name"), &format!("{path}.name"))?;
    let (colors_name, slots) = match obj.get("colors") {
        None => (None, Vec::new()),
        Some(Value::Object(m)) => {
            let parsed = parse_extra_colors(m, &format!("{path}.colors"))?;
            (parsed.name, parsed.slots)
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "colors".into(),
                    expected: TypeName::Object,
                },
                format!("{path}.colors"),
            ));
        }
    };
    let name = match (item_name, colors_name) {
        (Some(a), Some(b)) if a != b => {
            return Err(Error::with_path(
                ErrorKind::MutuallyExclusive {
                    a: "name".into(),
                    b: "colors.name".into(),
                },
                format!("{path}.name"),
            ));
        }
        (Some(a), _) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    };
    Ok(ExtraScheme { name, slots })
}

fn parse_scheme_name(v: Option<&Value>, path: &str) -> Result<Option<String>, Error> {
    match v {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) if !s.is_empty() => Ok(Some(s.clone())),
        Some(Value::String(_)) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "name".into(),
                expected: TypeName::NonEmptyString,
            },
            path.to_string(),
        )),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "name".into(),
                expected: TypeName::String,
            },
            path.to_string(),
        )),
    }
}

fn parse_extra_colors(obj: &Map<String, Value>, path: &str) -> Result<NamedColorSlots, Error> {
    for k in obj.keys() {
        if k != "name" && !crate::theme::COLOR_SLOTS.contains(&k.as_str()) {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("{path}.{k}"),
            ));
        }
    }
    let name = parse_scheme_name(obj.get("name"), &format!("{path}.name"))?;
    let mut slots = Vec::new();
    for slot in crate::theme::COLOR_SLOTS {
        if let Some(v) = obj.get(*slot) {
            slots.push((
                (*slot).to_string(),
                parse_hex_at(v, &format!("{path}.{slot}"))?,
            ));
        }
    }
    Ok(NamedColorSlots { name, slots })
}

fn deny_keys(obj: &Map<String, Value>, i: usize, allowed: &[&str]) -> Result<(), Error> {
    for k in obj.keys() {
        if !allowed.contains(&k.as_str()) {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("ops[{i}].{k}"),
            ));
        }
    }
    Ok(())
}

fn theme_part(pkg: &Package, doc_part: &str) -> Result<Option<String>, Error> {
    part_by_suffix(pkg, doc_part, "/theme")
}

fn override_part(pkg: &Package, doc_part: &str) -> Result<Option<String>, Error> {
    part_by_suffix(pkg, doc_part, "/themeOverride")
}

fn part_by_suffix(pkg: &Package, doc_part: &str, suffix: &str) -> Result<Option<String>, Error> {
    let rels = Relationships::from_package(pkg, doc_part)?;
    let Some(rel) = rels.rels.iter().find(|r| r.rel_type.ends_with(suffix)) else {
        return Ok(None);
    };
    if rel.is_external() {
        return Ok(None);
    }
    let name = opc::resolve_rel_target(doc_part, &rel.target)?;
    if pkg.contains(&name) {
        Ok(Some(name))
    } else {
        Ok(None)
    }
}

fn create_override_part(pkg: &mut Package, doc_part: &str) -> Result<String, Error> {
    let mut xml_bytes = crate::theme::default_override_xml().into_bytes();
    if let Some(theme) = theme_part(pkg, doc_part)? {
        let theme_doc = xml::parse(pkg.get(&theme)?)?;
        if let Some(scheme) = crate::theme::clone_clr_scheme(&theme_doc.root) {
            let mut ov = xml::parse(&xml_bytes)?;
            ov.root.children.retain(|n| match n {
                crate::xml::Node::Element(e) => e.local_name() != "clrScheme",
                _ => true,
            });
            ov.root
                .children
                .insert(0, crate::xml::Node::Element(scheme));
            xml_bytes = ov.to_bytes()?;
        }
    }
    pkg.set(OVERRIDE_PART, xml_bytes)?;
    let mut rels = Relationships::from_package(pkg, doc_part)?;
    if rels.find_by_type_suffix("/themeOverride").is_empty() {
        rels.push(Relationship {
            id: rels.next_rid(),
            rel_type: REL_THEME_OVERRIDE.into(),
            target: picture::rel_target_from(doc_part, OVERRIDE_PART),
            target_mode: None,
            extra: Vec::new(),
        });
        rels.write_to(pkg, doc_part)?;
    }
    let mut cts = ContentTypes::from_package(pkg)?;
    cts.set_override(OVERRIDE_PART, CT_THEME_OVERRIDE)?;
    cts.write_to(pkg)?;
    Ok(OVERRIDE_PART.into())
}

fn remove_override(pkg: &mut Package, doc_part: &str) -> Result<(), Error> {
    let rels = Relationships::from_package(pkg, doc_part)?;
    let hits: Vec<(String, String)> = rels
        .rels
        .iter()
        .filter(|r| r.rel_type.ends_with("/themeOverride") && !r.is_external())
        .filter_map(|r| {
            opc::resolve_rel_target(doc_part, &r.target)
                .ok()
                .map(|p| (r.id.clone(), p))
        })
        .collect();
    let mut rels = rels;
    for (id, part) in hits {
        rels.remove_by_id(&id);
        pkg.remove(&part);
        let mut cts = ContentTypes::from_package(pkg)?;
        cts.remove_override(&part);
        cts.write_to(pkg)?;
    }
    rels.write_to(pkg, doc_part)?;
    Ok(())
}
