//! 幻灯片母版与版式：inspect 名单、占位符几何与文字、换页的版式关系。

use serde_json::{json, Map, Value};

use crate::error::{Error, ErrorKind, IdReason};
use crate::opc::{self, Package, Relationship, Relationships};
use crate::xml::{self, Element, Node};

use super::notes;
use super::picture;
use super::shape::{self, ShapeKey, SlideMeta};
use super::slides;
use super::REL_SLIDE_LAYOUT;

#[derive(Debug, Clone)]
pub struct PlaceholderId {
    pub master: usize,
    pub layout: Option<usize>,
    pub key: ShapeKey,
}

pub struct Catalog {
    masters: Vec<MasterRec>,
}

struct MasterRec {
    index: usize,
    part: String,
    name: Option<String>,
    layouts: Vec<LayoutRec>,
}

struct LayoutRec {
    index: usize,
    part: String,
    name: Option<String>,
    kind: Option<String>,
}

pub fn catalog(pkg: &Package, pres_part: &str, pres_root: &Element) -> Result<Catalog, Error> {
    let mut masters = Vec::new();
    for (i, part) in master_parts(pkg, pres_part, pres_root)?
        .into_iter()
        .enumerate()
    {
        let doc = xml::parse(pkg.get(&part)?)?;
        let name = csld_name(&doc.root);
        let mut layouts = Vec::new();
        for (k, layout_part) in layout_parts(pkg, &part, &doc.root)?.into_iter().enumerate() {
            let layout_doc = xml::parse(pkg.get(&layout_part)?)?;
            layouts.push(LayoutRec {
                index: k + 1,
                part: layout_part,
                name: csld_name(&layout_doc.root),
                kind: layout_doc.root.attr("type").map(str::to_string),
            });
        }
        masters.push(MasterRec {
            index: i + 1,
            part,
            name,
            layouts,
        });
    }
    Ok(Catalog { masters })
}

impl Catalog {
    pub fn count(&self) -> usize {
        self.masters.len()
    }

    pub fn layout_id_of(&self, layout_part: &str) -> Option<String> {
        for m in &self.masters {
            for l in &m.layouts {
                if same_part(&l.part, layout_part) {
                    return Some(format!("master:{}/layout:{}", m.index, l.index));
                }
            }
        }
        None
    }

    fn master(&self, n: usize) -> Option<&MasterRec> {
        self.masters.iter().find(|m| m.index == n)
    }

    fn layout(&self, master: usize, layout: usize) -> Option<&LayoutRec> {
        self.master(master)?
            .layouts
            .iter()
            .find(|l| l.index == layout)
    }

    fn part_for(&self, ph: &PlaceholderId, json_path: &str) -> Result<&str, Error> {
        match ph.layout {
            Some(k) => self
                .layout(ph.master, k)
                .map(|l| l.part.as_str())
                .ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::LayoutNotFound {
                            master: ph.master as u32,
                            layout: k as u32,
                        },
                        json_path,
                    )
                }),
            None => self
                .master(ph.master)
                .map(|m| m.part.as_str())
                .ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::MasterNotFound {
                            n: ph.master as u32,
                        },
                        json_path,
                    )
                }),
        }
    }
}

pub fn layout_id_json(pkg: &Package, catalog: &Catalog, slide_part: &str) -> Result<Value, Error> {
    let Some(part) = slides::slide_layout_of(pkg, slide_part)? else {
        return Ok(Value::Null);
    };
    Ok(json!(catalog.layout_id_of(&part)))
}

pub fn masters_json(pkg: &Package, catalog: &Catalog) -> Result<Value, Error> {
    let mut out = Vec::new();
    for m in &catalog.masters {
        let doc = xml::parse(pkg.get(&m.part)?)?;
        let host = format!("master:{}", m.index);
        let mut layouts = Vec::new();
        for l in &m.layouts {
            let layout_doc = xml::parse(pkg.get(&l.part)?)?;
            let layout_host = format!("{host}/layout:{}", l.index);
            layouts.push(json!({
                "id": layout_host,
                "index": l.index,
                "name": l.name,
                "kind": l.kind,
                "placeholders": placeholders_json(&layout_doc.root, &layout_host),
            }));
        }
        out.push(json!({
            "id": host,
            "index": m.index,
            "name": m.name,
            "placeholders": placeholders_json(&doc.root, &host),
            "layouts": layouts,
        }));
    }
    Ok(Value::Array(out))
}

pub fn parse_placeholder_id(id: &str, op_index: usize) -> Result<PlaceholderId, Error> {
    parse_placeholder_id_at(id, &format!("ops[{op_index}].id"))
}

pub fn parse_layout_ref(id: &str, op_index: usize) -> Result<(usize, usize), Error> {
    let path = format!("ops[{op_index}].layout");
    let rest = id.strip_prefix("master:").ok_or_else(|| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::LayoutForm,
            },
            path.clone(),
        )
    })?;
    let Some((n_str, after)) = rest.split_once('/') else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::LayoutForm,
            },
            path,
        ));
    };
    let Some(k_str) = after.strip_prefix("layout:") else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::LayoutForm,
            },
            path,
        ));
    };
    if k_str.is_empty() || k_str.contains('/') {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::LayoutForm,
            },
            path,
        ));
    }
    Ok((
        parse_from_one(n_str, &path, IdReason::MasterForm)?,
        parse_from_one(k_str, &path, IdReason::LayoutForm)?,
    ))
}

pub fn set_text(
    pkg: &mut Package,
    pres_part: &str,
    pres_root: &Element,
    ph: &PlaceholderId,
    id: &str,
    text: &str,
    op_index: usize,
) -> Result<(), Error> {
    apply_on_placeholder(pkg, pres_part, pres_root, ph, id, op_index, |sp| {
        notes::reject_slide_image(sp, op_index)?;
        let tx = sp.child_by_local_mut("txBody").ok_or_else(|| {
            Error::with_path(
                ErrorKind::ShapeNotFound { id: id.to_string() },
                format!("ops[{op_index}].id"),
            )
        })?;
        shape::set_txbody_text(tx, text);
        Ok(())
    })
}

pub fn set_pos(
    pkg: &mut Package,
    pres_part: &str,
    pres_root: &Element,
    ph: &PlaceholderId,
    id: &str,
    x: Option<i64>,
    y: Option<i64>,
    op_index: usize,
) -> Result<(), Error> {
    apply_on_placeholder(pkg, pres_part, pres_root, ph, id, op_index, |sp| {
        notes::reject_slide_image(sp, op_index)?;
        shape::set_sp_pos(sp, x, y);
        Ok(())
    })
}

pub fn set_size(
    pkg: &mut Package,
    pres_part: &str,
    pres_root: &Element,
    ph: &PlaceholderId,
    id: &str,
    cx: Option<i64>,
    cy: Option<i64>,
    op_index: usize,
) -> Result<(), Error> {
    apply_on_placeholder(pkg, pres_part, pres_root, ph, id, op_index, |sp| {
        notes::reject_slide_image(sp, op_index)?;
        shape::set_sp_size(sp, cx, cy);
        Ok(())
    })
}

pub fn set_slide_layout(
    pkg: &mut Package,
    slides: &[SlideMeta],
    pres_part: &str,
    pres_root: &Element,
    slide: usize,
    master: usize,
    layout: usize,
    op_index: usize,
) -> Result<(), Error> {
    let meta = slides.get(slide - 1).ok_or_else(|| {
        Error::with_path(
            ErrorKind::SlideNotFound { n: slide as u32 },
            format!("ops[{op_index}].id"),
        )
    })?;
    let catalog = catalog(pkg, pres_part, pres_root)?;
    let dest = catalog
        .layout(master, layout)
        .ok_or_else(|| {
            Error::with_path(
                ErrorKind::LayoutNotFound {
                    master: master as u32,
                    layout: layout as u32,
                },
                format!("ops[{op_index}].layout"),
            )
        })?
        .part
        .clone();
    let mut rels = Relationships::from_package(pkg, &meta.part)?;
    if let Some(rel) = rels
        .rels
        .iter_mut()
        .find(|r| r.rel_type.ends_with("/slideLayout"))
    {
        let current = opc::resolve_id(pkg, &meta.part, &rel.id)?;
        if same_part(&current, &dest) {
            return Ok(());
        }
        rel.target = picture::rel_target_from(&meta.part, &dest);
    } else {
        rels.push(Relationship {
            id: rels.next_rid(),
            rel_type: REL_SLIDE_LAYOUT.into(),
            target: picture::rel_target_from(&meta.part, &dest),
            target_mode: None,
            extra: Vec::new(),
        });
    }
    rels.write_to(pkg, &meta.part)?;
    Ok(())
}

fn apply_on_placeholder(
    pkg: &mut Package,
    pres_part: &str,
    pres_root: &Element,
    ph: &PlaceholderId,
    id: &str,
    op_index: usize,
    f: impl FnOnce(&mut Element) -> Result<(), Error>,
) -> Result<(), Error> {
    let json_path = format!("ops[{op_index}].id");
    let catalog = catalog(pkg, pres_part, pres_root)?;
    let part = catalog.part_for(ph, &json_path)?.to_string();
    let mut doc = xml::parse(pkg.get(&part)?)?;
    let sp = find_placeholder_mut(&mut doc.root, &ph.key).ok_or_else(|| {
        Error::with_path(ErrorKind::ShapeNotFound { id: id.to_string() }, json_path)
    })?;
    f(sp)?;
    pkg.set(&part, doc.to_bytes()?)?;
    Ok(())
}

fn parse_placeholder_id_at(id: &str, path: &str) -> Result<PlaceholderId, Error> {
    let rest = id.strip_prefix("master:").ok_or_else(|| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::MasterForm,
            },
            path.to_string(),
        )
    })?;
    let (n_str, after) = match rest.split_once('/') {
        Some((n, a)) => (n, Some(a)),
        None => (rest, None),
    };
    let master = parse_from_one(n_str, path, IdReason::MasterForm)?;
    let Some(after) = after else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::MasterForm,
            },
            path.to_string(),
        ));
    };
    if let Some(layout_rest) = after.strip_prefix("layout:") {
        let (k_str, shape_after) = match layout_rest.split_once('/') {
            Some((k, s)) => (k, Some(s)),
            None => (layout_rest, None),
        };
        let layout = parse_from_one(k_str, path, IdReason::LayoutForm)?;
        let Some(shape_after) = shape_after else {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::MasterForm,
                },
                path.to_string(),
            ));
        };
        let key = parse_shape_after(shape_after, path)?;
        return Ok(PlaceholderId {
            master,
            layout: Some(layout),
            key,
        });
    }
    let key = parse_shape_after(after, path)?;
    Ok(PlaceholderId {
        master,
        layout: None,
        key,
    })
}

fn parse_shape_after(after: &str, path: &str) -> Result<ShapeKey, Error> {
    let Some(rest) = after.strip_prefix("shape:") else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::MasterForm,
            },
            path.to_string(),
        ));
    };
    if rest.is_empty() {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::ShapeEmpty,
            },
            path.to_string(),
        ));
    }
    if let Some(k) = rest.strip_prefix("index:") {
        let idx: usize = k.parse().map_err(|_| {
            Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::ShapeIndexNotInt,
                },
                path.to_string(),
            )
        })?;
        if idx < 1 {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::ShapeIndexFromOne,
                },
                path.to_string(),
            ));
        }
        return Ok(ShapeKey::Index(idx));
    }
    Ok(ShapeKey::Id(rest.to_string()))
}

fn parse_from_one(s: &str, path: &str, reason: IdReason) -> Result<usize, Error> {
    let n: usize = s.parse().map_err(|_| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: reason.clone(),
            },
            path.to_string(),
        )
    })?;
    if n < 1 {
        return Err(Error::with_path(
            ErrorKind::InvalidId { reason },
            path.to_string(),
        ));
    }
    Ok(n)
}

fn master_parts(pkg: &Package, pres_part: &str, pres_root: &Element) -> Result<Vec<String>, Error> {
    let mut out = Vec::new();
    if let Some(lst) = pres_root.child_by_local("sldMasterIdLst") {
        for el in lst
            .element_children()
            .filter(|e| e.local_name() == "sldMasterId")
        {
            if let Some(rid) = shape::rid_attr(el) {
                out.push(opc::resolve_id(pkg, pres_part, rid)?);
            }
        }
    }
    Ok(out)
}

fn layout_parts(
    pkg: &Package,
    master_part: &str,
    master_root: &Element,
) -> Result<Vec<String>, Error> {
    let mut out = Vec::new();
    if let Some(lst) = master_root.child_by_local("sldLayoutIdLst") {
        for el in lst
            .element_children()
            .filter(|e| e.local_name() == "sldLayoutId")
        {
            if let Some(rid) = shape::rid_attr(el) {
                out.push(opc::resolve_id(pkg, master_part, rid)?);
            }
        }
    }
    Ok(out)
}

fn same_part(a: &str, b: &str) -> bool {
    a.trim_start_matches('/') == b.trim_start_matches('/')
}

fn csld_name(root: &Element) -> Option<String> {
    root.child_by_local("cSld")
        .and_then(|c| c.attr("name"))
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn placeholders_json(root: &Element, host: &str) -> Vec<Value> {
    let mut out = Vec::new();
    let mut i = 0usize;
    shape::walk_choice(root, &mut |e| {
        if !is_placeholder(e) {
            return;
        }
        i += 1;
        out.push(placeholder_json(e, host, i));
    });
    out
}

fn placeholder_json(sp: &Element, host: &str, index: usize) -> Value {
    let (id, shape_id, stable) = shape::shape_stable_id(sp, host, index);
    let (x, y, cx, cy) = shape::geometry_json(sp);
    let ph = sp
        .child_by_local("nvSpPr")
        .and_then(|nv| nv.child_by_local("nvPr"))
        .and_then(|nv| nv.child_by_local("ph"));
    let mut obj = Map::new();
    obj.insert("id".into(), json!(id));
    obj.insert("shapeId".into(), json!(shape_id));
    obj.insert("name".into(), json!(shape::shape_name(sp)));
    obj.insert(
        "type".into(),
        json!(ph.and_then(|p| p.attr("type")).filter(|s| !s.is_empty())),
    );
    obj.insert(
        "idx".into(),
        json!(ph.and_then(|p| p.attr("idx")).filter(|s| !s.is_empty())),
    );
    obj.insert("x".into(), x);
    obj.insert("y".into(), y);
    obj.insert("cx".into(), cx);
    obj.insert("cy".into(), cy);
    obj.insert("text".into(), json!(shape::visible_text(sp)));
    obj.insert("stable".into(), json!(stable));
    Value::Object(obj)
}

fn is_placeholder(el: &Element) -> bool {
    el.local_name() == "sp"
        && el
            .child_by_local("nvSpPr")
            .and_then(|nv| nv.child_by_local("nvPr"))
            .and_then(|nv| nv.child_by_local("ph"))
            .is_some()
}

fn find_placeholder_mut<'a>(root: &'a mut Element, key: &ShapeKey) -> Option<&'a mut Element> {
    let mut idx = 0usize;
    find_ph_mut(root, &mut idx, key)
}

fn find_ph_mut<'a>(
    el: &'a mut Element,
    idx: &mut usize,
    key: &ShapeKey,
) -> Option<&'a mut Element> {
    if el.local_name() == "AlternateContent" {
        if let Some(i) = shape::first_alt_child_index(el) {
            if let Node::Element(child) = &mut el.children[i] {
                return find_ph_mut(child, idx, key);
            }
        }
        return None;
    }
    if is_placeholder(el) {
        *idx += 1;
        let hit = match key {
            ShapeKey::Index(n) => *idx == *n,
            ShapeKey::Id(req) => shape::cnv_pr(el)
                .and_then(|c| c.attr("id"))
                .is_some_and(|id| shape::ids_match(id, req)),
        };
        if hit {
            return Some(el);
        }
        return None;
    }
    for child in el.children.as_mut_slice() {
        if let Node::Element(e) = child {
            if let Some(found) = find_ph_mut(e, idx, key) {
                return Some(found);
            }
        }
    }
    None
}
