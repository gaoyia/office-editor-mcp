//! 幻灯片尺寸与母版 theme：inspect `slideSize` / `theme`，改 `sldSz`、配色、字体、`fmtScheme` 与补充色板。
//! 不按新画布重排，不算主题排版引擎。

use serde_json::{json, Value};

use crate::error::{Error, ErrorKind};
use crate::opc::{self, Package, Relationships};
use crate::xml::{self, Document, Element, Node};

use super::master;

pub use crate::theme::{FontSlot, COLOR_SLOTS};

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
    let doc = xml::parse(pkg.get(&part)?)?;
    Ok(crate::theme::inspect_full(&doc.root))
}

pub fn set_theme_colors(
    pkg: &mut Package,
    pres_part: &str,
    pres_root: &Element,
    master: usize,
    slots: &[(String, String)],
    name: Option<Option<&str>>,
    theme_name: Option<Option<&str>>,
    op_index: usize,
) -> Result<(), Error> {
    let json_path = format!("ops[{op_index}].id");
    let part = theme_part_for_master(pkg, pres_part, pres_root, master, &json_path)?;
    let mut doc = xml::parse(pkg.get(&part)?)?;
    if !slots.is_empty() && !crate::theme::patch_colors(&mut doc.root, slots) {
        return Err(Error::with_path(
            ErrorKind::ThemeNotFound { n: master as u32 },
            json_path,
        ));
    }
    if let Some(name) = name {
        if !crate::theme::patch_clr_scheme_name(&mut doc.root, name) {
            return Err(Error::with_path(
                ErrorKind::ThemeNotFound { n: master as u32 },
                format!("ops[{op_index}].id"),
            ));
        }
    }
    if let Some(name) = theme_name {
        crate::theme::patch_theme_name(&mut doc.root, name);
    }
    pkg.set(&part, doc.to_bytes()?)?;
    Ok(())
}

pub fn set_theme_fonts(
    pkg: &mut Package,
    pres_part: &str,
    pres_root: &Element,
    master: usize,
    patches: &[(FontSlot, String)],
    name: Option<Option<&str>>,
    major_scripts: Option<&[crate::theme::ScriptFont]>,
    minor_scripts: Option<&[crate::theme::ScriptFont]>,
    op_index: usize,
) -> Result<(), Error> {
    let json_path = format!("ops[{op_index}].id");
    let part = theme_part_for_master(pkg, pres_part, pres_root, master, &json_path)?;
    let mut doc = xml::parse(pkg.get(&part)?)?;
    crate::theme::patch_fonts(&mut doc.root, patches, &json_path)?;
    if let Some(name) = name {
        if !crate::theme::patch_font_scheme_name(&mut doc.root, name) {
            return Err(Error::with_path(
                ErrorKind::ThemeNotFound { n: master as u32 },
                json_path.clone(),
            ));
        }
    }
    if let Some(fonts) = major_scripts {
        crate::theme::replace_script_fonts(&mut doc.root, true, fonts, &json_path)?;
    }
    if let Some(fonts) = minor_scripts {
        crate::theme::replace_script_fonts(&mut doc.root, false, fonts, &json_path)?;
    }
    pkg.set(&part, doc.to_bytes()?)?;
    Ok(())
}

pub fn set_theme_fmt(
    pkg: &mut Package,
    pres_part: &str,
    pres_root: &Element,
    master: usize,
    line: Option<(usize, u32)>,
    fill: Option<(usize, crate::theme::FmtFillKind)>,
    bg_fill: Option<(usize, crate::theme::FmtFillKind)>,
    name: Option<Option<&str>>,
    op_index: usize,
) -> Result<(), Error> {
    let json_path = format!("ops[{op_index}].id");
    let part = theme_part_for_master(pkg, pres_part, pres_root, master, &json_path)?;
    let mut doc = xml::parse(pkg.get(&part)?)?;
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
    pkg.set(&part, doc.to_bytes()?)?;
    Ok(())
}

pub fn set_theme_extra_schemes(
    pkg: &mut Package,
    pres_part: &str,
    pres_root: &Element,
    master: usize,
    schemes: &[crate::theme::ExtraScheme],
    op_index: usize,
) -> Result<(), Error> {
    let json_path = format!("ops[{op_index}].id");
    let part = theme_part_for_master(pkg, pres_part, pres_root, master, &json_path)?;
    let mut doc = xml::parse(pkg.get(&part)?)?;
    crate::theme::replace_extra_schemes(&mut doc.root, schemes);
    pkg.set(&part, doc.to_bytes()?)?;
    Ok(())
}

fn theme_part_for_master(
    pkg: &Package,
    pres_part: &str,
    pres_root: &Element,
    master: usize,
    json_path: &str,
) -> Result<String, Error> {
    let cat = master::catalog(pkg, pres_part, pres_root)?;
    let master_part = cat
        .master_part(master)
        .ok_or_else(|| {
            Error::with_path(
                ErrorKind::MasterNotFound { n: master as u32 },
                json_path.to_string(),
            )
        })?
        .to_string();
    drop(cat);
    theme_part(pkg, &master_part)?.ok_or_else(|| {
        Error::with_path(
            ErrorKind::ThemeNotFound { n: master as u32 },
            json_path.to_string(),
        )
    })
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

fn check_size(v: i64, field: &str, op_index: usize) -> Result<(), Error> {
    if (SLIDE_SIZE_MIN..=SLIDE_SIZE_MAX).contains(&v) {
        return Ok(());
    }
    Err(Error::with_path(
        ErrorKind::SlideSizeOutOfRange,
        format!("ops[{op_index}].{field}"),
    ))
}
