//! 幻灯片尺寸与母版 theme：inspect `slideSize` / `theme`，改 `sldSz`、配色、字体、`fmtScheme` 与补充色板。
//! 不按新画布重排，不算主题排版引擎。

use serde_json::{json, Value};

use crate::error::{Error, ErrorKind};
use crate::opc::{self, Package, Relationships};
use crate::xml::{Document, Element, Node};

use super::handout;
use super::master;
use super::notes;

pub use crate::theme::{FontSlot, COLOR_SLOTS};

/// `setThemeColors` 等 op 的宿主：slideMaster、备注母版或讲义母版。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeHost {
    Master(usize),
    NotesMaster,
    Handout,
}

/// `p:clrMap` / `a:overrideClrMapping` 的十二个键，与 ECMA-376 属性同名。
pub const CLR_MAP_KEYS: &[&str] = &[
    "bg1", "tx1", "bg2", "tx2", "accent1", "accent2", "accent3", "accent4", "accent5", "accent6",
    "hlink", "folHlink",
];

pub const SLIDE_SIZE_MIN: i64 = 914_400;
pub const SLIDE_SIZE_MAX: i64 = 51_206_400;

pub fn is_slide_size_type(ty: &str) -> bool {
    matches!(
        ty,
        "screen4x3"
            | "letter"
            | "A4"
            | "35mm"
            | "overhead"
            | "banner"
            | "custom"
            | "ledger"
            | "A3"
            | "B4ISO"
            | "B5ISO"
            | "B4JIS"
            | "B5JIS"
            | "hagakiCard"
            | "screen16x9"
            | "screen16x10"
    )
}

pub fn slide_size_json(pres_root: &Element) -> Value {
    let Some(sz) = pres_root.child_by_local("sldSz") else {
        return Value::Null;
    };
    let ty = sz.attr_local("type").filter(|s| !s.is_empty());
    json!({
        "cx": parse_emu(sz.attr_local("cx")),
        "cy": parse_emu(sz.attr_local("cy")),
        "type": ty,
    })
}

pub fn set_slide_size(
    pres: &mut Document,
    cx: i64,
    cy: i64,
    type_name: Option<&str>,
    op_index: usize,
) -> Result<(), Error> {
    check_size(cx, "cx", op_index)?;
    check_size(cy, "cy", op_index)?;
    if let Some(sz) = pres.root.child_by_local_mut("sldSz") {
        sz.set_attr("cx", cx.to_string());
        sz.set_attr("cy", cy.to_string());
        if let Some(ty) = type_name {
            sz.set_attr("type", ty);
        }
        return Ok(());
    }
    let mut el = Element::new("p:sldSz")
        .with_attr("cx", cx.to_string())
        .with_attr("cy", cy.to_string());
    if let Some(ty) = type_name {
        el = el.with_attr("type", ty);
    }
    let at = pres
        .root
        .children
        .iter()
        .position(|n| matches!(n, Node::Element(e) if e.local_name() == "sldIdLst"))
        .map(|i| i + 1)
        .or_else(|| {
            pres.root
                .children
                .iter()
                .position(|n| matches!(n, Node::Element(e) if e.local_name() == "notesSz"))
        })
        .unwrap_or(pres.root.children.len());
    pres.root.children.insert(at, Node::Element(el));
    Ok(())
}

pub fn theme_json(pkg: &Package, master_part: &str) -> Result<Value, Error> {
    let Some(part) = theme_part(pkg, master_part)? else {
        return Ok(Value::Null);
    };
    let doc = super::list_pin::parse_part(pkg, &part)?;
    Ok(crate::theme::inspect_full(&doc.root))
}

pub fn notes_size_json(pres_root: &Element) -> Value {
    let Some(sz) = pres_root.child_by_local("notesSz") else {
        return Value::Null;
    };
    json!({
        "cx": parse_emu(sz.attr_local("cx")),
        "cy": parse_emu(sz.attr_local("cy")),
    })
}

pub fn set_notes_size(pres: &mut Document, cx: i64, cy: i64, op_index: usize) -> Result<(), Error> {
    check_size(cx, "cx", op_index)?;
    check_size(cy, "cy", op_index)?;
    if let Some(sz) = pres.root.child_by_local_mut("notesSz") {
        sz.set_attr("cx", cx.to_string());
        sz.set_attr("cy", cy.to_string());
        return Ok(());
    }
    let el = Element::new("p:notesSz")
        .with_attr("cx", cx.to_string())
        .with_attr("cy", cy.to_string());
    let at = pres
        .root
        .children
        .iter()
        .position(|n| matches!(n, Node::Element(e) if e.local_name() == "sldSz"))
        .map(|i| i + 1)
        .or_else(|| {
            pres.root
                .children
                .iter()
                .position(|n| matches!(n, Node::Element(e) if e.local_name() == "sldIdLst"))
                .map(|i| i + 1)
        })
        .unwrap_or(pres.root.children.len());
    pres.root.children.insert(at, Node::Element(el));
    Ok(())
}

pub fn set_theme_colors(
    pkg: &mut Package,
    pres_part: &str,
    pres_root: &Element,
    host: ThemeHost,
    slots: &[(String, String)],
    name: Option<Option<&str>>,
    theme_name: Option<Option<&str>>,
    op_index: usize,
) -> Result<(), Error> {
    let json_path = format!("ops[{op_index}].id");
    let part = theme_part_for_host(pkg, pres_part, pres_root, host, &json_path)?;
    let mut doc = super::list_pin::parse_part(pkg, &part)?;
    if !slots.is_empty() && !crate::theme::patch_colors(&mut doc.root, slots) {
        return Err(missing_theme(host, &json_path));
    }
    if let Some(name) = name {
        if !crate::theme::patch_clr_scheme_name(&mut doc.root, name) {
            return Err(missing_theme(host, &format!("ops[{op_index}].id")));
        }
    }
    if let Some(name) = theme_name {
        crate::theme::patch_theme_name(&mut doc.root, name);
    }
    super::list_pin::write_part(pkg, &part, &doc)?;
    Ok(())
}

pub fn set_theme_fonts(
    pkg: &mut Package,
    pres_part: &str,
    pres_root: &Element,
    host: ThemeHost,
    patches: &[(FontSlot, String)],
    name: Option<Option<&str>>,
    major_scripts: Option<&[crate::theme::ScriptFont]>,
    minor_scripts: Option<&[crate::theme::ScriptFont]>,
    op_index: usize,
) -> Result<(), Error> {
    let json_path = format!("ops[{op_index}].id");
    let part = theme_part_for_host(pkg, pres_part, pres_root, host, &json_path)?;
    let mut doc = super::list_pin::parse_part(pkg, &part)?;
    crate::theme::patch_fonts(&mut doc.root, patches, &json_path)?;
    if let Some(name) = name {
        if !crate::theme::patch_font_scheme_name(&mut doc.root, name) {
            return Err(missing_theme(host, &json_path));
        }
    }
    if let Some(fonts) = major_scripts {
        crate::theme::replace_script_fonts(&mut doc.root, true, fonts, &json_path)?;
    }
    if let Some(fonts) = minor_scripts {
        crate::theme::replace_script_fonts(&mut doc.root, false, fonts, &json_path)?;
    }
    super::list_pin::write_part(pkg, &part, &doc)?;
    Ok(())
}

pub fn set_theme_fmt(
    pkg: &mut Package,
    pres_part: &str,
    pres_root: &Element,
    host: ThemeHost,
    line: Option<(usize, u32)>,
    fill: Option<(usize, crate::theme::FmtFillKind)>,
    bg_fill: Option<(usize, crate::theme::FmtFillKind)>,
    name: Option<Option<&str>>,
    op_index: usize,
) -> Result<(), Error> {
    let json_path = format!("ops[{op_index}].id");
    let part = theme_part_for_host(pkg, pres_part, pres_root, host, &json_path)?;
    let mut doc = super::list_pin::parse_part(pkg, &part)?;
    if let Some((index, w)) = line {
        if !crate::theme::patch_line_width(&mut doc.root, index, w) {
            return Err(Error::with_path(
                ErrorKind::InvalidThemeFmtIndex,
                format!("ops[{op_index}].line.index"),
            ));
        }
    }
    if let Some((index, kind)) = fill {
        if !crate::theme::patch_fill_slot(&mut doc.root, false, index, kind) {
            return Err(Error::with_path(
                ErrorKind::InvalidThemeFmtIndex,
                format!("ops[{op_index}].fill.index"),
            ));
        }
    }
    if let Some((index, kind)) = bg_fill {
        if !crate::theme::patch_fill_slot(&mut doc.root, true, index, kind) {
            return Err(Error::with_path(
                ErrorKind::InvalidThemeFmtIndex,
                format!("ops[{op_index}].bgFill.index"),
            ));
        }
    }
    if let Some(name) = name {
        if !crate::theme::patch_fmt_scheme_name(&mut doc.root, name) {
            return Err(Error::with_path(
                ErrorKind::InvalidThemeFmtIndex,
                format!("ops[{op_index}].name"),
            ));
        }
    }
    super::list_pin::write_part(pkg, &part, &doc)?;
    Ok(())
}

pub fn set_theme_extra_schemes(
    pkg: &mut Package,
    pres_part: &str,
    pres_root: &Element,
    host: ThemeHost,
    schemes: &[crate::theme::ExtraScheme],
    op_index: usize,
) -> Result<(), Error> {
    let json_path = format!("ops[{op_index}].id");
    let part = theme_part_for_host(pkg, pres_part, pres_root, host, &json_path)?;
    let mut doc = super::list_pin::parse_part(pkg, &part)?;
    crate::theme::replace_extra_schemes(&mut doc.root, schemes);
    super::list_pin::write_part(pkg, &part, &doc)?;
    Ok(())
}

pub fn set_theme_object_defaults(
    pkg: &mut Package,
    pres_part: &str,
    pres_root: &Element,
    host: ThemeHost,
    defaults: Option<&crate::theme::ObjectDefaultsPatch>,
    op_index: usize,
) -> Result<(), Error> {
    let json_path = format!("ops[{op_index}].id");
    let part = theme_part_for_host(pkg, pres_part, pres_root, host, &json_path)?;
    let mut doc = super::list_pin::parse_part(pkg, &part)?;
    crate::theme::patch_object_defaults(&mut doc.root, defaults);
    super::list_pin::write_part(pkg, &part, &doc)?;
    Ok(())
}

fn theme_part_for_host(
    pkg: &Package,
    pres_part: &str,
    pres_root: &Element,
    host: ThemeHost,
    json_path: &str,
) -> Result<String, Error> {
    let host_part = match host {
        ThemeHost::Master(n) => {
            let cat = master::catalog(pkg, pres_part, pres_root)?;
            cat.master_part(n)
                .ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::MasterNotFound { n: n as u32 },
                        json_path.to_string(),
                    )
                })?
                .to_string()
        }
        ThemeHost::NotesMaster => notes::notes_master_part(pkg, pres_part)?.ok_or_else(|| {
            Error::with_path(ErrorKind::NotesMasterNotFound, json_path.to_string())
        })?,
        ThemeHost::Handout => handout::handout_part(pkg, pres_part)?.ok_or_else(|| {
            Error::with_path(ErrorKind::HandoutMasterNotFound, json_path.to_string())
        })?,
    };
    theme_part(pkg, &host_part)?.ok_or_else(|| missing_theme(host, json_path))
}

fn missing_theme(host: ThemeHost, json_path: &str) -> Error {
    let kind = match host {
        ThemeHost::Master(n) => ErrorKind::ThemeNotFound { n: n as u32 },
        ThemeHost::NotesMaster => ErrorKind::NotesThemeNotFound,
        ThemeHost::Handout => ErrorKind::HandoutThemeNotFound,
    };
    Error::with_path(kind, json_path.to_string())
}

fn theme_part(pkg: &Package, master_part: &str) -> Result<Option<String>, Error> {
    let rels = Relationships::from_package(pkg, master_part)?;
    let Some(rel) = rels.rels.iter().find(|r| r.rel_type.ends_with("/theme")) else {
        return Ok(None);
    };
    Ok(Some(opc::resolve_id(pkg, master_part, &rel.id)?))
}

fn parse_emu(raw: Option<&str>) -> Value {
    raw.and_then(|s| s.parse::<i64>().ok())
        .map(Value::from)
        .unwrap_or(Value::Null)
}

pub(crate) fn check_size(v: i64, field: &str, op_index: usize) -> Result<(), Error> {
    if (SLIDE_SIZE_MIN..=SLIDE_SIZE_MAX).contains(&v) {
        return Ok(());
    }
    Err(Error::with_path(
        ErrorKind::SlideSizeOutOfRange,
        format!("ops[{op_index}].{field}"),
    ))
}
