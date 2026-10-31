//! 位置序号：apply 开始时记在元素上，本批不改写。
//! drawing 与连接部件在这一次 apply 里留在内存，避免写回丢掉编号。

use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use crate::error::Error;
use crate::opc::{Package, Relationships};
use crate::xml::{self, Document, Element, Node};

thread_local! {
    static ACTIVE: Cell<bool> = const { Cell::new(false) };
    static CACHE: RefCell<HashMap<String, Document>> = RefCell::new(HashMap::new());
    static PIVOTS: RefCell<HashMap<String, Vec<String>>> = RefCell::new(HashMap::new());
}

pub(crate) struct PinGuard;

impl PinGuard {
    pub(crate) fn enter() -> Self {
        ACTIVE.with(|a| a.set(true));
        CACHE.with(|c| c.borrow_mut().clear());
        PIVOTS.with(|c| c.borrow_mut().clear());
        Self
    }
}

impl Drop for PinGuard {
    fn drop(&mut self) {
        ACTIVE.with(|a| a.set(false));
        CACHE.with(|c| c.borrow_mut().clear());
        PIVOTS.with(|c| c.borrow_mut().clear());
    }
}

fn active() -> bool {
    ACTIVE.with(|a| a.get())
}

#[derive(Default)]
pub(crate) struct DrawCount {
    pub pic: usize,
    pub chart: usize,
    pub sp: usize,
    pub shape: usize,
    pub cxn: usize,
    pub math: usize,
    pub smart: usize,
    pub model: usize,
    pub ink: usize,
    pub slicer: usize,
}

pub(crate) fn stamp(el: &mut Element, family: &str, n: usize) {
    let attr = xml::pin_attr(family);
    if el.attr(&attr).is_none() {
        el.set_attr(&attr, n.to_string());
    }
}

/// 有编号时按编号认。没有编号时退回当时数到的位置。
pub(crate) fn index_matches(el: &Element, wanted: usize, live: usize) -> bool {
    const FAMILIES: &[&str] = &[
        "pic",
        "pic-odd-header",
        "pic-odd-footer",
        "pic-even-header",
        "pic-even-footer",
        "pic-first-header",
        "pic-first-footer",
        "chart",
        "sp",
        "shape",
        "cxn",
        "math",
        "smartdiagram",
        "ole",
        "model3d",
        "slicer",
        "ink",
        "dv",
        "cf",
        "link",
        "spark",
        "editRange",
        "table",
        "name",
        "customView",
        "cellStyle",
        "extlink",
        "connection",
    ];
    for family in FAMILIES {
        let attr = xml::pin_attr(family);
        if el.attr(&attr).is_some() {
            return xml::pin_matches(el, &attr, wanted, live);
        }
    }
    live == wanted
}

pub(crate) fn pin_children(parent: &mut Element, local: &str, family: &str) {
    let attr = xml::pin_attr(family);
    let mut n = 0usize;
    for child in parent.element_children_mut() {
        if child.local_name() != local {
            continue;
        }
        n += 1;
        if child.attr(&attr).is_none() {
            child.set_attr(&attr, n.to_string());
        }
    }
}

pub(crate) fn find_child<'a>(
    parent: &'a Element,
    local: &str,
    family: &str,
    wanted: usize,
) -> Option<&'a Element> {
    let attr = xml::pin_attr(family);
    let mut live = 0usize;
    for child in parent.element_children() {
        if child.local_name() != local {
            continue;
        }
        live += 1;
        if xml::pin_matches(child, &attr, wanted, live) {
            return Some(child);
        }
    }
    None
}

pub(crate) fn find_child_mut<'a>(
    parent: &'a mut Element,
    local: &str,
    family: &str,
    wanted: usize,
) -> Option<&'a mut Element> {
    let attr = xml::pin_attr(family);
    let mut live = 0usize;
    let mut found = None;
    for (i, node) in parent.children.iter().enumerate() {
        let Node::Element(el) = node else {
            continue;
        };
        if el.local_name() != local {
            continue;
        }
        live += 1;
        if xml::pin_matches(el, &attr, wanted, live) {
            found = Some(i);
            break;
        }
    }
    let i = found?;
    match &mut parent.children[i] {
        Node::Element(el) => Some(el),
        _ => None,
    }
}

pub(crate) fn pin_sheet(root: &mut Element) {
    if let Some(parent) = root.child_by_local_mut("dataValidations") {
        pin_children(parent, "dataValidation", "dv");
    }
    pin_cf(root);
    if let Some(parent) = root.child_by_local_mut("hyperlinks") {
        pin_children(parent, "hyperlink", "link");
    }
    pin_sparklines(root);
    if let Some(parent) = root.child_by_local_mut("protectedRanges") {
        pin_children(parent, "protectedRange", "editRange");
    }
    if let Some(parent) = root.child_by_local_mut("tableParts") {
        pin_children(parent, "tablePart", "table");
    }
}

pub(crate) fn pin_workbook(root: &mut Element) {
    if let Some(parent) = root.child_by_local_mut("definedNames") {
        pin_children(parent, "definedName", "name");
    }
    if let Some(parent) = root.child_by_local_mut("customWorkbookViews") {
        pin_children(parent, "customWorkbookView", "customView");
    }
    if let Some(parent) = root.child_by_local_mut("externalReferences") {
        pin_children(parent, "externalReference", "extlink");
    }
}

pub(crate) fn pin_cell_styles(root: &mut Element) {
    if let Some(parent) = root.child_by_local_mut("cellStyles") {
        pin_children(parent, "cellStyle", "cellStyle");
    }
}

fn pin_cf(root: &mut Element) {
    let attr = xml::pin_attr("cf");
    let mut n = 0usize;
    for block in root.element_children_mut() {
        if block.local_name() != "conditionalFormatting" {
            continue;
        }
        for rule in block.element_children_mut() {
            if rule.local_name() != "cfRule" {
                continue;
            }
            n += 1;
            if rule.attr(&attr).is_none() {
                rule.set_attr(&attr, n.to_string());
            }
        }
    }
}

fn pin_sparklines(root: &mut Element) {
    let Some(ext_lst) = root.child_by_local_mut("extLst") else {
        return;
    };
    let attr = xml::pin_attr("spark");
    let mut n = 0usize;
    for ext in ext_lst.element_children_mut() {
        if ext.local_name() != "ext" {
            continue;
        }
        let uri = ext.attr("uri").unwrap_or("");
        if !uri.eq_ignore_ascii_case("{05C60535-1F16-4fd2-B633-F4F36F0B64E0}") {
            continue;
        }
        for groups in ext.element_children_mut() {
            if groups.local_name() != "sparklineGroups" {
                continue;
            }
            for group in groups.element_children_mut() {
                if group.local_name() != "sparklineGroup" {
                    continue;
                }
                n += 1;
                if group.attr(&attr).is_none() {
                    group.set_attr(&attr, n.to_string());
                }
            }
        }
    }
}

pub(crate) fn find_cf(root: &Element, wanted: usize) -> Option<(usize, usize)> {
    let attr = xml::pin_attr("cf");
    let mut n = 0usize;
    for (ci, node) in root.children.iter().enumerate() {
        let Node::Element(block) = node else {
            continue;
        };
        if block.local_name() != "conditionalFormatting" {
            continue;
        }
        for (ri, rn) in block.children.iter().enumerate() {
            let Node::Element(rule) = rn else {
                continue;
            };
            if rule.local_name() != "cfRule" {
                continue;
            }
            n += 1;
            if xml::pin_matches(rule, &attr, wanted, n) {
                return Some((ci, ri));
            }
        }
    }
    None
}

pub(crate) fn capture_pivots(
    pkg: &Package,
    sheets: &[(String, String, &'static str)],
) -> Result<(), Error> {
    let mut map = HashMap::new();
    for (_, part, kind) in sheets {
        if *kind != "worksheet" && *kind != "chartsheet" {
            continue;
        }
        let rels = Relationships::from_package(pkg, part)?;
        let ids = rels
            .rels
            .iter()
            .filter(|rel| rel.rel_type.ends_with("/pivotTable"))
            .map(|rel| rel.id.clone())
            .collect();
        map.insert((*part).to_string(), ids);
    }
    PIVOTS.with(|c| *c.borrow_mut() = map);
    Ok(())
}

/// apply 里按开始时的关系 id 认。守卫没开时返回 `None`，调用方按当时的顺序。
pub(crate) fn pivot_rid(sheet_part: &str, index: usize) -> Option<String> {
    if !active() || index == 0 {
        return None;
    }
    PIVOTS.with(|c| {
        c.borrow()
            .get(sheet_part)
            .and_then(|ids| ids.get(index - 1).cloned())
    })
}

pub(crate) fn pivots_frozen(sheet_part: &str) -> bool {
    active() && PIVOTS.with(|c| c.borrow().contains_key(sheet_part))
}

fn cacheable(part: &str) -> bool {
    let p = part.replace('\\', "/");
    is_drawing_part(&p) || p.rsplit('/').next() == Some("connections.xml")
}

fn is_drawing_part(part: &str) -> bool {
    part.contains("/drawings/")
        || part.contains("/diagrams/")
        || part.contains("vmlDrawing")
        || part.ends_with(".vml")
}

pub(crate) fn cache_put(part: &str, doc: &Document) {
    if !active() {
        return;
    }
    CACHE.with(|c| {
        c.borrow_mut().insert(part.to_string(), doc.clone());
    });
}

pub(crate) fn parse_part(pkg: &Package, part: &str) -> Result<Document, Error> {
    if active() && cacheable(part) {
        if let Some(doc) = CACHE.with(|c| c.borrow().get(part).cloned()) {
            return Ok(doc);
        }
    }
    let mut doc = xml::parse(pkg.get(part)?)?;
    if active() && part.replace('\\', "/").rsplit('/').next() == Some("connections.xml") {
        pin_connections(&mut doc.root);
    }
    if active() && cacheable(part) {
        cache_put(part, &doc);
    }
    Ok(doc)
}

pub(crate) fn write_part(pkg: &mut Package, part: &str, doc: &Document) -> Result<(), Error> {
    if active() && cacheable(part) {
        cache_put(part, doc);
    }
    pkg.set(part, doc.to_bytes()?)
}

fn pin_connections(root: &mut Element) {
    let attr = xml::pin_attr("connection");
    let mut n = 0usize;
    for child in root.element_children_mut() {
        if child.local_name() != "connection" || connection_deleted(child) {
            continue;
        }
        n += 1;
        if child.attr(&attr).is_none() {
            child.set_attr(&attr, n.to_string());
        }
    }
}

fn connection_deleted(el: &Element) -> bool {
    matches!(
        el.attr("deleted")
            .map(|s| s.to_ascii_lowercase())
            .as_deref(),
        Some("1") | Some("true") | Some("on")
    )
}
