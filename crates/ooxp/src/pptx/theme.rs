//! 幻灯片尺寸与母版 theme：inspect `slideSize` / `theme`，改 `sldSz`、`clrScheme`、`fontScheme`。
//! 不按新画布重排，不重写 `fmtScheme`。

use serde_json::{json, Map, Value};

use crate::error::{Error, ErrorKind};
use crate::opc::{self, Package, Relationships};
use crate::xml::{self, Document, Element, Node};

use super::master;
use super::picture;

pub const SLIDE_SIZE_MIN: i64 = 914_400;
pub const SLIDE_SIZE_MAX: i64 = 51_206_400;

pub const COLOR_SLOTS: &[&str] = &[
    "dk1", "lt1", "dk2", "lt2", "accent1", "accent2", "accent3", "accent4", "accent5", "accent6",
    "hlink", "folHlink",
];

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
    let name = doc
        .root
        .attr_local("name")
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    let colors = color_scheme_json(&doc.root);
    let fonts = font_scheme_json(&doc.root);
    Ok(json!({
        "name": name,
        "colors": colors,
        "fonts": fonts,
    }))
}

pub fn set_theme_colors(
    pkg: &mut Package,
    pres_part: &str,
    pres_root: &Element,
    master: usize,
    slots: &[(String, String)],
    op_index: usize,
) -> Result<(), Error> {
    let json_path = format!("ops[{op_index}].id");
    let part = theme_part_for_master(pkg, pres_part, pres_root, master, &json_path)?;
    let mut doc = xml::parse(pkg.get(&part)?)?;
    picture::ensure_xmlns(&mut doc.root, "a", super::DML_NS);
    let scheme = doc
        .root
        .child_by_local_mut("themeElements")
        .and_then(|e| e.child_by_local_mut("clrScheme"))
        .ok_or_else(|| {
            Error::with_path(
                ErrorKind::ThemeNotFound { n: master as u32 },
                json_path.clone(),
            )
        })?;
    for (slot, hex) in slots {
        upsert_srgb_slot(scheme, slot, hex);
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
    op_index: usize,
) -> Result<(), Error> {
    let json_path = format!("ops[{op_index}].id");
    let part = theme_part_for_master(pkg, pres_part, pres_root, master, &json_path)?;
    let mut doc = xml::parse(pkg.get(&part)?)?;
    picture::ensure_xmlns(&mut doc.root, "a", super::DML_NS);
    if doc.root.child_by_local("themeElements").is_none() {
        doc.root
            .children
            .insert(0, Node::Element(Element::new("a:themeElements")));
    }
    let elements = doc
        .root
        .child_by_local_mut("themeElements")
        .ok_or_else(|| {
            Error::with_path(
                ErrorKind::XmlMissing {
                    parent: "theme".into(),
                    child: "themeElements".into(),
                },
                json_path.clone(),
            )
        })?;
    if elements.child_by_local("fontScheme").is_none() {
        elements.push_element(Element::new("a:fontScheme").with_attr("name", "Office"));
    }
    let scheme = elements.child_by_local_mut("fontScheme").ok_or_else(|| {
        Error::with_path(
            ErrorKind::XmlMissing {
                parent: "themeElements".into(),
                child: "fontScheme".into(),
            },
            json_path,
        )
    })?;
    for (slot, face) in patches {
        set_typeface(scheme, *slot, face);
    }
    pkg.set(&part, doc.to_bytes()?)?;
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontSlot {
    MajorLatin,
    MinorLatin,
    MajorEa,
    MinorEa,
    MajorCs,
    MinorCs,
}

impl FontSlot {
    fn group(self) -> &'static str {
        match self {
            Self::MajorLatin | Self::MajorEa | Self::MajorCs => "majorFont",
            Self::MinorLatin | Self::MinorEa | Self::MinorCs => "minorFont",
        }
    }

    fn face(self) -> &'static str {
        match self {
            Self::MajorLatin | Self::MinorLatin => "latin",
            Self::MajorEa | Self::MinorEa => "ea",
            Self::MajorCs | Self::MinorCs => "cs",
        }
    }
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

fn color_scheme_json(root: &Element) -> Value {
    let mut obj = Map::new();
    let scheme = root
        .child_by_local("themeElements")
        .and_then(|e| e.child_by_local("clrScheme"));
    obj.insert(
        "name".into(),
        json!(scheme
            .and_then(|s| s.attr_local("name"))
            .filter(|s| !s.is_empty())),
    );
    for slot in COLOR_SLOTS {
        obj.insert((*slot).into(), json!(slot_hex(scheme, slot)));
    }
    Value::Object(obj)
}

fn font_scheme_json(root: &Element) -> Value {
    let scheme = root
        .child_by_local("themeElements")
        .and_then(|e| e.child_by_local("fontScheme"));
    let mut obj = Map::new();
    obj.insert(
        "name".into(),
        json!(scheme
            .and_then(|s| s.attr_local("name"))
            .filter(|s| !s.is_empty())),
    );
    obj.insert(
        "majorLatin".into(),
        json!(typeface_of(scheme, "majorFont", "latin")),
    );
    obj.insert(
        "minorLatin".into(),
        json!(typeface_of(scheme, "minorFont", "latin")),
    );
    obj.insert(
        "majorEa".into(),
        json!(typeface_of(scheme, "majorFont", "ea")),
    );
    obj.insert(
        "minorEa".into(),
        json!(typeface_of(scheme, "minorFont", "ea")),
    );
    obj.insert(
        "majorCs".into(),
        json!(typeface_of(scheme, "majorFont", "cs")),
    );
    obj.insert(
        "minorCs".into(),
        json!(typeface_of(scheme, "minorFont", "cs")),
    );
    Value::Object(obj)
}

fn slot_hex(scheme: Option<&Element>, slot: &str) -> Option<String> {
    let el = scheme?.child_by_local(slot)?;
    if let Some(v) = el
        .child_by_local("srgbClr")
        .and_then(|e| e.attr_local("val"))
        .filter(|s| !s.is_empty())
    {
        return Some(v.to_string());
    }
    el.child_by_local("sysClr")
        .and_then(|e| e.attr_local("lastClr"))
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn typeface_of(scheme: Option<&Element>, group: &str, face: &str) -> Option<String> {
    scheme?
        .child_by_local(group)
        .and_then(|g| e_child(g, face))
        .and_then(|e| e.attr_local("typeface"))
        .map(str::to_string)
}

fn e_child<'a>(el: &'a Element, local: &str) -> Option<&'a Element> {
    el.child_by_local(local)
}

fn upsert_srgb_slot(scheme: &mut Element, slot: &str, hex: &str) {
    if let Some(el) = scheme.child_by_local_mut(slot) {
        el.children.clear();
        el.push_element(Element::new("a:srgbClr").with_attr("val", hex));
        return;
    }
    let mut el = Element::new(format!("a:{slot}"));
    el.push_element(Element::new("a:srgbClr").with_attr("val", hex));
    scheme.push_element(el);
}

fn set_typeface(scheme: &mut Element, slot: FontSlot, face: &str) {
    if scheme.child_by_local(slot.group()).is_none() {
        scheme.push_element(Element::new(format!("a:{}", slot.group())));
    }
    let Some(group) = scheme.child_by_local_mut(slot.group()) else {
        return;
    };
    if let Some(el) = group.child_by_local_mut(slot.face()) {
        el.set_attr("typeface", face);
        return;
    }
    group.push_element(Element::new(format!("a:{}", slot.face())).with_attr("typeface", face));
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
