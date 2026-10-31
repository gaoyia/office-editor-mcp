//! 位置序号：apply 开始时记下，本批不改写。
//! 页、母版、版式和备注页留在这一次 apply 的内存里，避免写回丢掉编号。

use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use crate::error::Error;
use crate::opc::Package;
use crate::xml::{self, Document};

thread_local! {
    static ACTIVE: Cell<bool> = const { Cell::new(false) };
    static CACHE: RefCell<HashMap<String, Document>> = RefCell::new(HashMap::new());
    static MASTERS: RefCell<Vec<MasterSnap>> = const { RefCell::new(Vec::new()) };
}

pub(crate) struct PinGuard;

impl PinGuard {
    pub(crate) fn enter() -> Self {
        ACTIVE.with(|a| a.set(true));
        CACHE.with(|c| c.borrow_mut().clear());
        MASTERS.with(|c| c.borrow_mut().clear());
        Self
    }
}

impl Drop for PinGuard {
    fn drop(&mut self) {
        ACTIVE.with(|a| a.set(false));
        CACHE.with(|c| c.borrow_mut().clear());
        MASTERS.with(|c| c.borrow_mut().clear());
    }
}

fn active() -> bool {
    ACTIVE.with(|a| a.get())
}

#[derive(Clone)]
pub(crate) struct MasterSnap {
    pub id: Option<String>,
    pub layouts: Vec<Option<String>>,
}

pub(crate) fn set_masters(snaps: Vec<MasterSnap>) {
    if active() {
        MASTERS.with(|c| *c.borrow_mut() = snaps);
    }
}

pub(crate) fn masters_frozen() -> bool {
    active() && MASTERS.with(|c| !c.borrow().is_empty())
}

pub(crate) fn master_len() -> usize {
    MASTERS.with(|c| c.borrow().len())
}

pub(crate) fn master_index(id: Option<&str>) -> Option<usize> {
    let id = id?;
    MASTERS.with(|c| {
        c.borrow()
            .iter()
            .position(|m| m.id.as_deref() == Some(id))
            .map(|i| i + 1)
    })
}

pub(crate) fn layout_len(master_id: Option<&str>) -> usize {
    let Some(master_id) = master_id else {
        return 0;
    };
    MASTERS.with(|c| {
        c.borrow()
            .iter()
            .find(|m| m.id.as_deref() == Some(master_id))
            .map(|m| m.layouts.len())
            .unwrap_or(0)
    })
}

pub(crate) fn layout_index(master_id: Option<&str>, layout_id: Option<&str>) -> Option<usize> {
    let master_id = master_id?;
    let layout_id = layout_id?;
    MASTERS.with(|c| {
        let snaps = c.borrow();
        let snap = snaps.iter().find(|m| m.id.as_deref() == Some(master_id))?;
        snap.layouts
            .iter()
            .position(|id| id.as_deref() == Some(layout_id))
            .map(|i| i + 1)
    })
}

fn cacheable(part: &str) -> bool {
    let p = part.replace('\\', "/");
    p.contains("/slides/")
        || p.contains("/notesSlides/")
        || p.contains("/slideMasters/")
        || p.contains("/slideLayouts/")
        || p.contains("/comments/")
        || (p.contains("/charts/") && p.ends_with(".xml") && !p.contains("/_rels/"))
}

pub(crate) fn parse_part(pkg: &Package, part: &str) -> Result<Document, Error> {
    if active() && cacheable(part) {
        if let Some(doc) = CACHE.with(|c| c.borrow().get(part).cloned()) {
            return Ok(doc);
        }
    }
    let mut doc = xml::parse(pkg.get(part)?)?;
    if active() && cacheable(part) {
        let p = part.replace('\\', "/");
        if p.contains("/charts/") && p.ends_with(".xml") && !p.contains("/_rels/") {
            crate::chart::pin_space(&mut doc.root);
        } else {
            pin_part(&mut doc);
        }
        CACHE.with(|c| {
            c.borrow_mut().insert(part.to_string(), doc.clone());
        });
    }
    Ok(doc)
}

pub(crate) fn write_part(pkg: &mut Package, part: &str, doc: &Document) -> Result<(), Error> {
    if active() && cacheable(part) {
        CACHE.with(|c| {
            c.borrow_mut().insert(part.to_string(), doc.clone());
        });
    }
    pkg.set(part, doc.to_bytes()?)
}

fn pin_part(doc: &mut Document) {
    super::shape::pin_shapes(&mut doc.root);
    super::picture::pin_pics(&mut doc.root);
    super::connector::pin_connectors(&mut doc.root);
    super::table::pin_tables(&mut doc.root);
    super::field::pin_fields(&mut doc.root);
    super::hyperlink::pin_links(&mut doc.root);
    super::chart::pin_charts(&mut doc.root);
    super::ole::pin_oles(&mut doc.root);
    super::smartart::pin_smart(&mut doc.root);
    super::model3d::pin_models(&mut doc.root);
    super::omml::pin_maths(&mut doc.root);
    super::zoom::pin_zooms(&mut doc.root);
    super::media::pin_media(&mut doc.root);
    super::ink::pin_inks(&mut doc.root);
    super::timing::pin_anims(&mut doc.root);
}
