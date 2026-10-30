//! 配色映射与页级 themeOverride：inspect `clrMap` / `clrMapOvr` / `themeOverride`，apply 写入。
//! 不算主题排版引擎，不把映射算成 RGB。

use serde_json::{json, Map, Value};

use crate::error::{Error, ErrorKind};
use crate::opc::{self, ContentTypes, Package, Relationships};
use crate::xml::{self, Element, Node};

use super::handout;
use super::master;
use super::notes;
use super::picture;
use super::request::{
    ClrMapHost, ClrMapOvrHost, ClrMapOvrValue, SetClrMapOp, SetClrMapOvrOp, SetThemeOverrideOp,
};
use super::shape::SlideMeta;
use super::theme::CLR_MAP_KEYS;

const DEFAULT_CLR_MAP: &[(&str, &str)] = &[
    ("bg1", "lt1"),
    ("tx1", "dk1"),
    ("bg2", "lt2"),
    ("tx2", "dk2"),
    ("accent1", "accent1"),
    ("accent2", "accent2"),
    ("accent3", "accent3"),
    ("accent4", "accent4"),
    ("accent5", "accent5"),
    ("accent6", "accent6"),
    ("hlink", "hlink"),
    ("folHlink", "folHlink"),
];

pub fn clr_map_json(root: &Element) -> Value {
    map_attrs_json(root.child_by_local("clrMap"))
}

pub fn clr_map_ovr_json(root: &Element) -> Value {
    let Some(ovr) = root.child_by_local("clrMapOvr") else {
        return Value::Null;
    };
    if ovr.child_by_local("overrideClrMapping").is_some() {
        return map_attrs_json(ovr.child_by_local("overrideClrMapping"));
    }
    if ovr.child_by_local("masterClrMapping").is_some() {
        return json!("master");
    }
    Value::Null
}

pub fn theme_override_json(pkg: &Package, host_part: &str) -> Result<Value, Error> {
    let Some(part) = override_part(pkg, host_part)? else {
        return Ok(Value::Null);
    };
    let doc = xml::parse(pkg.get(&part)?)?;
    Ok(crate::theme::inspect_override_full(&doc.root))
}

fn map_attrs_json(el: Option<&Element>) -> Value {
    let Some(el) = el else {
        return Value::Null;
    };
    let mut obj = Map::new();
    for key in CLR_MAP_KEYS {
        obj.insert(
            (*key).into(),
            json!(el.attr_local(key).filter(|s| !s.is_empty())),
        );
    }
    Value::Object(obj)
}

pub fn set_clr_map(
    pkg: &mut Package,
    pres_part: &str,
    pres_root: &Element,
    op: &SetClrMapOp,
    op_index: usize,
) -> Result<(), Error> {
    let json_path = format!("ops[{op_index}].id");
    let part = match &op.host {
        ClrMapHost::Master(n) => {
            let cat = master::catalog(pkg, pres_part, pres_root)?;
            cat.xml_part(*n, None, &json_path)?
        }
        ClrMapHost::NotesMaster => notes::notes_master_part(pkg, pres_part)?
            .ok_or_else(|| Error::with_path(ErrorKind::NotesMasterNotFound, json_path.clone()))?,
        ClrMapHost::Handout => handout::handout_part(pkg, pres_part)?
            .ok_or_else(|| Error::with_path(ErrorKind::HandoutMasterNotFound, json_path.clone()))?,
    };
    let mut doc = xml::parse(pkg.get(&part)?)?;
    patch_clr_map(&mut doc.root, &op.slots);
    pkg.set(&part, doc.to_bytes()?)?;
    Ok(())
}

pub fn set_clr_map_ovr(
    pkg: &mut Package,
    pres_part: &str,
    pres_root: &Element,
    slides: &[SlideMeta],
    op: &SetClrMapOvrOp,
    op_index: usize,
) -> Result<(), Error> {
    let json_path = format!("ops[{op_index}].id");
    let part = match &op.host {
        ClrMapOvrHost::Slide { slide, on_notes } => {
            host_slide_part(pkg, slides, *slide, *on_notes, &json_path)?
        }
        ClrMapOvrHost::Layout { master, layout } => {
            let cat = master::catalog(pkg, pres_part, pres_root)?;
            cat.xml_part(*master, Some(*layout), &json_path)?
        }
    };
    let mut doc = xml::parse(pkg.get(&part)?)?;
    write_clr_map_ovr(&mut doc.root, &op.value);
    pkg.set(&part, doc.to_bytes()?)?;
    Ok(())
}

pub fn set_theme_override(
    pkg: &mut Package,
    slides: &[SlideMeta],
    op: &SetThemeOverrideOp,
    op_index: usize,
) -> Result<(), Error> {
    let json_path = format!("ops[{op_index}].id");
    let host = host_slide_part(pkg, slides, op.slide, op.on_notes, &json_path)?;
    if !op.enabled {
        remove_override(pkg, &host)?;
        return Ok(());
    }
    let part = match override_part(pkg, &host)? {
        Some(p) => p,
        None => create_override_part(pkg, &host)?,
    };
    let mut doc = xml::parse(pkg.get(&part)?)?;
    crate::theme::patch_override_colors(&mut doc.root, &op.slots);
    pkg.set(&part, doc.to_bytes()?)?;
    Ok(())
}

fn host_slide_part(
    pkg: &Package,
    slides: &[SlideMeta],
    slide: usize,
    on_notes: bool,
    json_path: &str,
) -> Result<String, Error> {
    let meta = slides
        .iter()
        .find(|item| item.index == slide)
        .ok_or_else(|| {
            Error::with_path(
                ErrorKind::SlideNotFound { n: slide as u32 },
                json_path.to_string(),
            )
        })?;
    if !on_notes {
        return Ok(meta.part.clone());
    }
    notes::notes_slide_part(pkg, &meta.part)?.ok_or_else(|| {
        Error::with_path(
            ErrorKind::NotesSlideNotFound { n: slide as u32 },
            json_path.to_string(),
        )
    })
}

fn patch_clr_map(root: &mut Element, slots: &[(String, String)]) {
    if root.child_by_local("clrMap").is_none() {
        insert_after_csld(root, default_clr_map_el("p:clrMap"));
    }
    if let Some(el) = root.child_by_local_mut("clrMap") {
        for (k, v) in slots {
            el.set_attr(k, v);
        }
    }
}

fn write_clr_map_ovr(root: &mut Element, value: &ClrMapOvrValue) {
    if root.child_by_local("clrMapOvr").is_none() {
        insert_after_csld(root, Element::new("p:clrMapOvr"));
    }
    let Some(ovr) = root.child_by_local_mut("clrMapOvr") else {
        return;
    };
    match value {
        ClrMapOvrValue::Master => {
            ovr.children = vec![Node::Element(Element::new("a:masterClrMapping"))];
        }
        ClrMapOvrValue::Slots(slots) => {
            let mut mapping = ovr
                .child_by_local("overrideClrMapping")
                .cloned()
                .unwrap_or_else(|| default_clr_map_el("a:overrideClrMapping"));
            for (k, v) in slots {
                mapping.set_attr(k, v);
            }
            ovr.children = vec![Node::Element(mapping)];
        }
    }
}

fn default_clr_map_el(tag: &str) -> Element {
    let mut el = Element::new(tag);
    for (k, v) in DEFAULT_CLR_MAP {
        el = el.with_attr(*k, *v);
    }
    el
}

fn insert_after_csld(root: &mut Element, el: Element) {
    if let Some(i) = root
        .children
        .iter()
        .position(|n| matches!(n, Node::Element(e) if e.local_name() == "cSld"))
    {
        root.children.insert(i + 1, Node::Element(el));
    } else {
        root.children.insert(0, Node::Element(el));
    }
}

fn override_part(pkg: &Package, host_part: &str) -> Result<Option<String>, Error> {
    let rels = Relationships::from_package(pkg, host_part)?;
    let Some(rel) = rels
        .rels
        .iter()
        .find(|r| r.rel_type.ends_with("/themeOverride") && !r.is_external())
    else {
        return Ok(None);
    };
    let name = opc::resolve_id(pkg, host_part, &rel.id)?;
    if pkg.contains(&name) {
        Ok(Some(name))
    } else {
        Ok(None)
    }
}

fn create_override_part(pkg: &mut Package, host_part: &str) -> Result<String, Error> {
    let dest = next_override_part(pkg);
    let theme = theme_for_host(pkg, host_part)?;
    crate::theme::ops::install_override(
        pkg,
        host_part,
        &dest,
        theme.as_deref(),
        picture::rel_target_from,
        false,
    )?;
    Ok(dest)
}

fn remove_override(pkg: &mut Package, host_part: &str) -> Result<(), Error> {
    let rels = Relationships::from_package(pkg, host_part)?;
    let hits: Vec<(String, String)> = rels
        .rels
        .iter()
        .filter(|r| r.rel_type.ends_with("/themeOverride") && !r.is_external())
        .filter_map(|r| {
            opc::resolve_id(pkg, host_part, &r.id)
                .ok()
                .map(|p| (r.id.clone(), p))
        })
        .collect();
    let mut rels = rels;
    for (id, part) in hits {
        rels.remove_by_id(&id);
        if !part_still_referenced(pkg, &part, host_part) {
            pkg.remove(&part);
            let mut cts = ContentTypes::from_package(pkg)?;
            cts.remove_override(&part);
            cts.write_to(pkg)?;
        }
    }
    rels.write_to(pkg, host_part)?;
    Ok(())
}

fn part_still_referenced(pkg: &Package, part: &str, skip_host: &str) -> bool {
    for name in pkg.part_names() {
        if name == skip_host || !name.starts_with('/') {
            continue;
        }
        if name.contains("/_rels/") {
            continue;
        }
        let Ok(rels) = Relationships::from_package(pkg, &name) else {
            continue;
        };
        for r in &rels.rels {
            if r.is_external() {
                continue;
            }
            if opc::resolve_rel_target(&name, &r.target)
                .ok()
                .is_some_and(|found| opc::same_part_name(&found, part))
            {
                return true;
            }
        }
    }
    false
}

fn theme_for_host(pkg: &Package, host_part: &str) -> Result<Option<String>, Error> {
    if let Some(p) = rel_part_suffix(pkg, host_part, "/theme")? {
        return Ok(Some(p));
    }
    if let Some(nm) = rel_part_suffix(pkg, host_part, "/notesMaster")? {
        return rel_part_suffix(pkg, &nm, "/theme");
    }
    if let Some(layout) = rel_part_suffix(pkg, host_part, "/slideLayout")? {
        return theme_for_host(pkg, &layout);
    }
    if let Some(master) = rel_part_suffix(pkg, host_part, "/slideMaster")? {
        return rel_part_suffix(pkg, &master, "/theme");
    }
    Ok(None)
}

fn rel_part_suffix(pkg: &Package, host_part: &str, suffix: &str) -> Result<Option<String>, Error> {
    let rels = Relationships::from_package(pkg, host_part)?;
    let Some(rel) = rels
        .rels
        .iter()
        .find(|r| r.rel_type.ends_with(suffix) && !r.is_external())
    else {
        return Ok(None);
    };
    let name = opc::resolve_id(pkg, host_part, &rel.id)?;
    if pkg.contains(&name) {
        Ok(Some(name))
    } else {
        Ok(None)
    }
}

fn next_override_part(pkg: &Package) -> String {
    let mut n = 1u32;
    loop {
        let name = format!("/ppt/theme/themeOverride{n}.xml");
        if !pkg.contains(&name) {
            return name;
        }
        n += 1;
    }
}
