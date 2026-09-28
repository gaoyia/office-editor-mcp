//! 幻灯片母版与版式：inspect 名单、占位符几何与文字、换页的版式关系。

use serde_json::{json, Map, Value};

use crate::error::{Error, ErrorKind, IdReason};
use crate::opc::{self, rels_part_name, ContentTypes, Package, Relationship, Relationships};
use crate::xml::{self, Document, Element, Node};

use super::notes;
use super::picture;
use super::shape::{self, ShapeKey, SlideMeta};
use super::slides;
use super::{
    CT_SLIDE_LAYOUT, CT_SLIDE_MASTER, CT_THEME, DML_NS, PML_NS, REL_NS, REL_SLIDE_LAYOUT,
    REL_SLIDE_MASTER, REL_THEME,
};

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
    parse_layout_ref_at(id, &format!("ops[{op_index}].layout"))
}

pub fn parse_layout_ref_at(id: &str, path: &str) -> Result<(usize, usize), Error> {
    let rest = id.strip_prefix("master:").ok_or_else(|| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::LayoutForm,
            },
            path,
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
        parse_from_one(n_str, path, IdReason::MasterForm)?,
        parse_from_one(k_str, path, IdReason::LayoutForm)?,
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

pub fn parse_master_only_id(id: &str, op_index: usize, reason: IdReason) -> Result<usize, Error> {
    parse_master_only_at(id, &format!("ops[{op_index}].id"), reason)
}

pub fn parse_master_only_at(id: &str, path: &str, reason: IdReason) -> Result<usize, Error> {
    let rest = id.strip_prefix("master:").ok_or_else(|| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: reason.clone(),
            },
            path.to_string(),
        )
    })?;
    if rest.is_empty() || rest.contains('/') {
        return Err(Error::with_path(
            ErrorKind::InvalidId { reason },
            path.to_string(),
        ));
    }
    parse_from_one(rest, path, reason)
}

pub fn parse_master_or_layout_host(
    id: &str,
    op_index: usize,
) -> Result<(usize, Option<usize>), Error> {
    let path = format!("ops[{op_index}].id");
    let rest = id.strip_prefix("master:").ok_or_else(|| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::InsertPlaceholderNeedsHost,
            },
            path.clone(),
        )
    })?;
    if let Some((n_str, after)) = rest.split_once('/') {
        let Some(k_str) = after.strip_prefix("layout:") else {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::InsertPlaceholderNeedsHost,
                },
                path,
            ));
        };
        if k_str.is_empty() || k_str.contains('/') {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::InsertPlaceholderNeedsHost,
                },
                path,
            ));
        }
        return Ok((
            parse_from_one(n_str, &path, IdReason::InsertPlaceholderNeedsHost)?,
            Some(parse_from_one(
                k_str,
                &path,
                IdReason::InsertPlaceholderNeedsHost,
            )?),
        ));
    }
    Ok((
        parse_from_one(rest, &path, IdReason::InsertPlaceholderNeedsHost)?,
        None,
    ))
}

pub fn is_writable_placeholder_type(ty: &str) -> bool {
    matches!(
        ty,
        "title" | "body" | "ctrTitle" | "subTitle" | "dt" | "ftr" | "sldNum"
    )
}

const PH_CX: i64 = 5_080_000;
const PH_CY: i64 = 2_540_000;
const EMPTY_SP_TREE: &str = r#"<p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/><a:chOff x="0" y="0"/><a:chExt cx="0" cy="0"/></a:xfrm></p:grpSpPr>"#;

pub fn insert_layout(
    pkg: &mut Package,
    pres_part: &str,
    pres_root: &Element,
    master: usize,
    name: &str,
    op_index: usize,
) -> Result<(), Error> {
    let master_part = {
        let cat = catalog(pkg, pres_part, pres_root)?;
        cat.master(master)
            .ok_or_else(|| {
                Error::with_path(
                    ErrorKind::MasterNotFound { n: master as u32 },
                    format!("ops[{op_index}].id"),
                )
            })?
            .part
            .clone()
    };
    let layout_part = next_part(pkg, "/ppt/slideLayouts", "slideLayout");
    let xml = blank_layout_xml(name);
    pkg.set(&layout_part, xml.into_bytes())?;
    let mut layout_rels = Relationships { rels: Vec::new() };
    layout_rels.push(Relationship {
        id: "rId1".into(),
        rel_type: REL_SLIDE_MASTER.into(),
        target: picture::rel_target_from(&layout_part, &master_part),
        target_mode: None,
        extra: Vec::new(),
    });
    layout_rels.write_to(pkg, &layout_part)?;
    let mut master_rels = Relationships::from_package(pkg, &master_part)?;
    let rid = master_rels.next_rid();
    master_rels.push(Relationship {
        id: rid.clone(),
        rel_type: REL_SLIDE_LAYOUT.into(),
        target: picture::rel_target_from(&master_part, &layout_part),
        target_mode: None,
        extra: Vec::new(),
    });
    master_rels.write_to(pkg, &master_part)?;
    let mut doc = xml::parse(pkg.get(&master_part)?)?;
    picture::ensure_xmlns(&mut doc.root, "r", REL_NS);
    let next_id = next_unique_sld_id(pkg, pres_part, pres_root)?;
    append_sld_layout_id(&mut doc.root, next_id, &rid);
    pkg.set(&master_part, doc.to_bytes()?)?;
    let mut cts = ContentTypes::from_package(pkg)?;
    cts.set_override(&layout_part, CT_SLIDE_LAYOUT)?;
    cts.write_to(pkg)?;
    Ok(())
}

pub fn delete_layout(
    pkg: &mut Package,
    slides: &[SlideMeta],
    pres_part: &str,
    pres_root: &Element,
    master: usize,
    layout: usize,
    op_index: usize,
) -> Result<(), Error> {
    let cat = catalog(pkg, pres_part, pres_root)?;
    let rec = cat.master(master).ok_or_else(|| {
        Error::with_path(
            ErrorKind::MasterNotFound { n: master as u32 },
            format!("ops[{op_index}].id"),
        )
    })?;
    if rec.layouts.len() <= 1 {
        return Err(Error::with_path(
            ErrorKind::LastLayout,
            format!("ops[{op_index}].id"),
        ));
    }
    let dest = cat.layout(master, layout).ok_or_else(|| {
        Error::with_path(
            ErrorKind::LayoutNotFound {
                master: master as u32,
                layout: layout as u32,
            },
            format!("ops[{op_index}].id"),
        )
    })?;
    let want = format!("master:{master}/layout:{layout}");
    for meta in slides {
        let Some(part) = slides::slide_layout_of(pkg, &meta.part)? else {
            continue;
        };
        if cat.layout_id_of(&part).as_deref() == Some(want.as_str()) {
            return Err(Error::with_path(
                ErrorKind::LayoutInUse,
                format!("ops[{op_index}].id"),
            ));
        }
    }
    let layout_part = dest.part.clone();
    let master_part = rec.part.clone();
    drop(cat);
    let mut master_rels = Relationships::from_package(pkg, &master_part)?;
    let rid = master_rels
        .rels
        .iter()
        .find(|r| {
            r.rel_type.ends_with("/slideLayout")
                && opc::resolve_id(pkg, &master_part, &r.id)
                    .ok()
                    .is_some_and(|p| same_part(&p, &layout_part))
        })
        .map(|r| r.id.clone());
    let Some(rid) = rid else {
        return Err(Error::with_path(
            ErrorKind::LayoutNotFound {
                master: master as u32,
                layout: layout as u32,
            },
            format!("ops[{op_index}].id"),
        ));
    };
    let mut doc = xml::parse(pkg.get(&master_part)?)?;
    remove_sld_layout_id(&mut doc.root, &rid);
    pkg.set(&master_part, doc.to_bytes()?)?;
    master_rels.remove_by_id(&rid);
    master_rels.write_to(pkg, &master_part)?;
    let mut cts = ContentTypes::from_package(pkg)?;
    pkg.remove(&layout_part);
    pkg.remove(&rels_part_name(&layout_part));
    cts.remove_override(&layout_part);
    cts.write_to(pkg)?;
    Ok(())
}

pub fn insert_placeholder(
    pkg: &mut Package,
    pres_part: &str,
    pres_root: &Element,
    master: usize,
    layout: Option<usize>,
    type_name: &str,
    text: &str,
    x: Option<i64>,
    y: Option<i64>,
    cx: Option<i64>,
    cy: Option<i64>,
    op_index: usize,
) -> Result<(), Error> {
    let json_path = format!("ops[{op_index}].id");
    let part = {
        let cat = catalog(pkg, pres_part, pres_root)?;
        match layout {
            Some(k) => cat
                .layout(master, k)
                .map(|l| l.part.clone())
                .ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::LayoutNotFound {
                            master: master as u32,
                            layout: k as u32,
                        },
                        json_path.clone(),
                    )
                })?,
            None => cat.master(master).map(|m| m.part.clone()).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::MasterNotFound { n: master as u32 },
                    json_path.clone(),
                )
            })?,
        }
    };
    let mut doc = xml::parse(pkg.get(&part)?)?;
    picture::ensure_xmlns(&mut doc.root, "a", DML_NS);
    let cx = cx.unwrap_or(PH_CX);
    let cy = cy.unwrap_or(PH_CY);
    let (x, y) = match (x, y) {
        (Some(px), Some(py)) => (px, py),
        _ => picture::vacant_pos_canvas(canvas_sz(pres_root), &doc.root, cx, cy),
    };
    let idx = next_ph_idx(&doc.root, type_name);
    let cnv = picture::next_cnv_id(&doc.root);
    let name = placeholder_name(type_name, cnv);
    let mut sp = build_placeholder(cnv, &name, type_name, idx, x, y, cx, cy);
    if let Some(tx) = sp.child_by_local_mut("txBody") {
        shape::set_txbody_text(tx, text);
    }
    picture::insert_into_sp_tree(&mut doc.root, sp)?;
    pkg.set(&part, doc.to_bytes()?)?;
    Ok(())
}

pub fn delete_placeholder(
    pkg: &mut Package,
    pres_part: &str,
    pres_root: &Element,
    ph: &PlaceholderId,
    id: &str,
    op_index: usize,
) -> Result<(), Error> {
    let json_path = format!("ops[{op_index}].id");
    let cat = catalog(pkg, pres_part, pres_root)?;
    let part = cat.part_for(ph, &json_path)?.to_string();
    let mut doc = xml::parse(pkg.get(&part)?)?;
    if !remove_placeholder(&mut doc.root, &ph.key) {
        return Err(Error::with_path(
            ErrorKind::ShapeNotFound { id: id.to_string() },
            json_path,
        ));
    }
    shape::prune_empty_groups(&mut doc.root);
    pkg.set(&part, doc.to_bytes()?)?;
    Ok(())
}

pub fn insert_master(
    pkg: &mut Package,
    pres_part: &str,
    pres: &mut Document,
    from: usize,
    op_index: usize,
) -> Result<(), Error> {
    let (src_master, src_layouts) = {
        let cat = catalog(pkg, pres_part, &pres.root)?;
        let src = cat.master(from).ok_or_else(|| {
            Error::with_path(
                ErrorKind::MasterNotFound { n: from as u32 },
                format!("ops[{op_index}].id"),
            )
        })?;
        (
            src.part.clone(),
            src.layouts
                .iter()
                .map(|l| l.part.clone())
                .collect::<Vec<_>>(),
        )
    };
    let src_rels = Relationships::from_package(pkg, &src_master)?;
    let src_theme = src_rels
        .rels
        .iter()
        .find(|r| r.rel_type.ends_with("/theme"))
        .and_then(|r| opc::resolve_id(pkg, &src_master, &r.id).ok());
    let new_master = next_part(pkg, "/ppt/slideMasters", "slideMaster");
    let new_theme = if src_theme.is_some() {
        Some(next_part(pkg, "/ppt/theme", "theme"))
    } else {
        None
    };
    let mut new_layouts = Vec::new();
    for src_layout in &src_layouts {
        let dest = next_part(pkg, "/ppt/slideLayouts", "slideLayout");
        pkg.set(&dest, pkg.get(src_layout)?.to_vec())?;
        new_layouts.push((src_layout.clone(), dest));
    }
    pkg.set(&new_master, pkg.get(&src_master)?.to_vec())?;
    if let (Some(src_th), Some(dest_th)) = (src_theme.as_ref(), new_theme.as_ref()) {
        pkg.set(dest_th, pkg.get(src_th)?.to_vec())?;
    }
    for (src_layout, dest) in &new_layouts {
        let mut rels = Relationships::from_package(pkg, src_layout)?;
        let mut saw_master = false;
        for rel in &mut rels.rels {
            if rel.rel_type.ends_with("/slideMaster") {
                rel.target = picture::rel_target_from(dest, &new_master);
                saw_master = true;
            }
        }
        if !saw_master {
            rels.push(Relationship {
                id: rels.next_rid(),
                rel_type: REL_SLIDE_MASTER.into(),
                target: picture::rel_target_from(dest, &new_master),
                target_mode: None,
                extra: Vec::new(),
            });
        }
        rels.write_to(pkg, dest)?;
    }
    let mut master_rels = Relationships { rels: Vec::new() };
    let mut layout_rids = Vec::new();
    for (_, dest) in &new_layouts {
        let rid = master_rels.next_rid();
        master_rels.push(Relationship {
            id: rid.clone(),
            rel_type: REL_SLIDE_LAYOUT.into(),
            target: picture::rel_target_from(&new_master, dest),
            target_mode: None,
            extra: Vec::new(),
        });
        layout_rids.push(rid);
    }
    if let Some(dest_th) = &new_theme {
        master_rels.push(Relationship {
            id: master_rels.next_rid(),
            rel_type: REL_THEME.into(),
            target: picture::rel_target_from(&new_master, dest_th),
            target_mode: None,
            extra: Vec::new(),
        });
    }
    for rel in &src_rels.rels {
        if rel.rel_type.ends_with("/slideLayout") || rel.rel_type.ends_with("/theme") {
            continue;
        }
        let mut copied = rel.clone();
        copied.id = master_rels.next_rid();
        master_rels.push(copied);
    }
    master_rels.write_to(pkg, &new_master)?;
    let mut master_doc = xml::parse(pkg.get(&new_master)?)?;
    picture::ensure_xmlns(&mut master_doc.root, "r", REL_NS);
    let mut next_id = next_unique_sld_id(pkg, pres_part, &pres.root)?;
    rewrite_sld_layout_id_lst(&mut next_id, &mut master_doc.root, &layout_rids);
    pkg.set(&new_master, master_doc.to_bytes()?)?;
    let mut pres_rels = Relationships::from_package(pkg, pres_part)?;
    let rid = pres_rels.next_rid();
    pres_rels.push(Relationship {
        id: rid.clone(),
        rel_type: REL_SLIDE_MASTER.into(),
        target: picture::rel_target_from(pres_part, &new_master),
        target_mode: None,
        extra: Vec::new(),
    });
    pres_rels.write_to(pkg, pres_part)?;
    picture::ensure_xmlns(&mut pres.root, "r", REL_NS);
    append_sld_master_id(&mut pres.root, next_id, &rid);
    pkg.set(pres_part, pres.to_bytes()?)?;
    let mut cts = ContentTypes::from_package(pkg)?;
    cts.set_override(&new_master, CT_SLIDE_MASTER)?;
    for (_, dest) in &new_layouts {
        cts.set_override(dest, CT_SLIDE_LAYOUT)?;
    }
    if let Some(dest_th) = &new_theme {
        cts.set_override(dest_th, CT_THEME)?;
    }
    cts.write_to(pkg)?;
    Ok(())
}

pub fn delete_master(
    pkg: &mut Package,
    slides: &[SlideMeta],
    pres_part: &str,
    pres: &mut Document,
    master: usize,
    op_index: usize,
) -> Result<(), Error> {
    let cat = catalog(pkg, pres_part, &pres.root)?;
    if cat.count() <= 1 {
        return Err(Error::with_path(
            ErrorKind::LastMaster,
            format!("ops[{op_index}].id"),
        ));
    }
    let rec = cat.master(master).ok_or_else(|| {
        Error::with_path(
            ErrorKind::MasterNotFound { n: master as u32 },
            format!("ops[{op_index}].id"),
        )
    })?;
    for meta in slides {
        let Some(part) = slides::slide_layout_of(pkg, &meta.part)? else {
            continue;
        };
        if let Some(id) = cat.layout_id_of(&part) {
            if id.starts_with(&format!("master:{master}/")) {
                return Err(Error::with_path(
                    ErrorKind::MasterInUse,
                    format!("ops[{op_index}].id"),
                ));
            }
        }
    }
    let master_part = rec.part.clone();
    let layout_parts: Vec<String> = rec.layouts.iter().map(|l| l.part.clone()).collect();
    drop(cat);
    let master_rels = Relationships::from_package(pkg, &master_part)?;
    let theme = master_rels
        .rels
        .iter()
        .find(|r| r.rel_type.ends_with("/theme"))
        .and_then(|r| opc::resolve_id(pkg, &master_part, &r.id).ok());
    let mut pres_rels = Relationships::from_package(pkg, pres_part)?;
    let rid = pres_rels
        .rels
        .iter()
        .find(|r| {
            r.rel_type.ends_with("/slideMaster")
                && opc::resolve_id(pkg, pres_part, &r.id)
                    .ok()
                    .is_some_and(|p| same_part(&p, &master_part))
        })
        .map(|r| r.id.clone());
    let Some(rid) = rid else {
        return Err(Error::with_path(
            ErrorKind::MasterNotFound { n: master as u32 },
            format!("ops[{op_index}].id"),
        ));
    };
    remove_sld_master_id(&mut pres.root, &rid);
    pres_rels.remove_by_id(&rid);
    pres_rels.write_to(pkg, pres_part)?;
    pkg.set(pres_part, pres.to_bytes()?)?;
    let mut cts = ContentTypes::from_package(pkg)?;
    pkg.remove(&master_part);
    pkg.remove(&rels_part_name(&master_part));
    cts.remove_override(&master_part);
    for part in &layout_parts {
        pkg.remove(part);
        pkg.remove(&rels_part_name(part));
        cts.remove_override(part);
    }
    if let Some(th) = theme {
        if !picture::media_still_used(pkg, &th)? {
            pkg.remove(&th);
            pkg.remove(&rels_part_name(&th));
            cts.remove_override(&th);
        }
    }
    cts.write_to(pkg)?;
    Ok(())
}

fn blank_layout_xml(name: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><p:sldLayout xmlns:a="{DML_NS}" xmlns:r="{REL_NS}" xmlns:p="{PML_NS}" type="blank" preserve="1"><p:cSld name="{}"><p:spTree>{EMPTY_SP_TREE}</p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:sldLayout>"#,
        xml::escape_attr(name),
    )
}

fn next_part(pkg: &Package, dir: &str, stem: &str) -> String {
    let mut n = 1u32;
    loop {
        let name = format!("{dir}/{stem}{n}.xml");
        if !pkg.contains(&name) {
            return name;
        }
        n += 1;
    }
}

fn canvas_sz(pres_root: &Element) -> (i64, i64) {
    pres_root
        .child_by_local("sldSz")
        .and_then(|e| {
            let cx = e.attr("cx")?.parse().ok()?;
            let cy = e.attr("cy")?.parse().ok()?;
            Some((cx, cy))
        })
        .unwrap_or((12_192_000, 6_858_000))
}

fn next_unique_sld_id(pkg: &Package, pres_part: &str, pres_root: &Element) -> Result<u32, Error> {
    let mut max = 2_147_483_647u32;
    if let Some(lst) = pres_root.child_by_local("sldMasterIdLst") {
        for el in lst
            .element_children()
            .filter(|e| e.local_name() == "sldMasterId")
        {
            if let Some(v) = el.attr("id").and_then(|s| s.parse().ok()) {
                max = max.max(v);
            }
        }
    }
    for part in master_parts(pkg, pres_part, pres_root)? {
        let doc = xml::parse(pkg.get(&part)?)?;
        if let Some(lst) = doc.root.child_by_local("sldLayoutIdLst") {
            for el in lst
                .element_children()
                .filter(|e| e.local_name() == "sldLayoutId")
            {
                if let Some(v) = el.attr("id").and_then(|s| s.parse().ok()) {
                    max = max.max(v);
                }
            }
        }
    }
    Ok(max.saturating_add(1))
}

fn append_sld_layout_id(root: &mut Element, id: u32, rid: &str) {
    let lst = if let Some(i) = root
        .children
        .iter()
        .position(|n| matches!(n, Node::Element(e) if e.local_name() == "sldLayoutIdLst"))
    {
        match &mut root.children[i] {
            Node::Element(e) => e,
            Node::Text(_) => unreachable!(),
        }
    } else {
        let lst = Element::new("p:sldLayoutIdLst");
        if let Some(i) = root
            .children
            .iter()
            .position(|n| matches!(n, Node::Element(e) if e.local_name() == "clrMap"))
        {
            root.children.insert(i + 1, Node::Element(lst));
        } else {
            root.push_element(lst);
        }
        match root.child_by_local_mut("sldLayoutIdLst") {
            Some(e) => e,
            None => return,
        }
    };
    let tag = lst_child_tag(lst, "sldLayoutId");
    lst.push_element(
        Element::new(tag)
            .with_attr("id", id.to_string())
            .with_attr("r:id", rid),
    );
}

fn append_sld_master_id(root: &mut Element, id: u32, rid: &str) {
    if root.child_by_local("sldMasterIdLst").is_none() {
        let lst = Element::new("p:sldMasterIdLst");
        if let Some(i) = root
            .children
            .iter()
            .position(|n| matches!(n, Node::Element(e) if e.local_name() == "sldIdLst"))
        {
            root.children.insert(i, Node::Element(lst));
        } else {
            root.children.insert(0, Node::Element(lst));
        }
    }
    let lst = match root.child_by_local_mut("sldMasterIdLst") {
        Some(e) => e,
        None => return,
    };
    let tag = lst_child_tag(lst, "sldMasterId");
    lst.push_element(
        Element::new(tag)
            .with_attr("id", id.to_string())
            .with_attr("r:id", rid),
    );
}

fn rewrite_sld_layout_id_lst(next_id: &mut u32, master_root: &mut Element, rids: &[String]) {
    if let Some(lst) = master_root.child_by_local_mut("sldLayoutIdLst") {
        lst.children
            .retain(|n| !matches!(n, Node::Element(e) if e.local_name() == "sldLayoutId"));
        let tag = lst_child_tag(lst, "sldLayoutId");
        for rid in rids {
            lst.push_element(
                Element::new(tag.clone())
                    .with_attr("id", next_id.to_string())
                    .with_attr("r:id", rid),
            );
            *next_id = next_id.saturating_add(1);
        }
        return;
    }
    let mut lst = Element::new("p:sldLayoutIdLst");
    for rid in rids {
        lst.push_element(
            Element::new("p:sldLayoutId")
                .with_attr("id", next_id.to_string())
                .with_attr("r:id", rid),
        );
        *next_id = next_id.saturating_add(1);
    }
    master_root.push_element(lst);
}

fn remove_sld_layout_id(root: &mut Element, rid: &str) {
    if let Some(lst) = root.child_by_local_mut("sldLayoutIdLst") {
        lst.children.retain(|n| match n {
            Node::Element(e) if e.local_name() == "sldLayoutId" => shape::rid_attr(e) != Some(rid),
            _ => true,
        });
    }
}

fn remove_sld_master_id(root: &mut Element, rid: &str) {
    if let Some(lst) = root.child_by_local_mut("sldMasterIdLst") {
        lst.children.retain(|n| match n {
            Node::Element(e) if e.local_name() == "sldMasterId" => shape::rid_attr(e) != Some(rid),
            _ => true,
        });
    }
}

fn lst_child_tag(lst: &Element, local: &str) -> String {
    if let Some(e) = lst.element_children().find(|e| e.local_name() == local) {
        return e.name.clone();
    }
    if let Some((prefix, _)) = lst.name.split_once(':') {
        format!("{prefix}:{local}")
    } else {
        format!("p:{local}")
    }
}

fn next_ph_idx(root: &Element, type_name: &str) -> Option<u32> {
    if matches!(type_name, "title" | "ctrTitle") {
        return None;
    }
    let mut max = None;
    shape::walk_choice(root, &mut |e| {
        if !is_placeholder(e) {
            return;
        }
        if let Some(v) = e
            .child_by_local("nvSpPr")
            .and_then(|nv| nv.child_by_local("nvPr"))
            .and_then(|nv| nv.child_by_local("ph"))
            .and_then(|ph| ph.attr("idx"))
            .and_then(|s| s.parse::<u32>().ok())
        {
            max = Some(max.unwrap_or(0).max(v));
        }
    });
    Some(max.map(|v| v + 1).unwrap_or(10))
}

fn placeholder_name(ty: &str, cnv: u32) -> String {
    match ty {
        "title" => format!("标题占位符 {cnv}"),
        "ctrTitle" => format!("居中标题占位符 {cnv}"),
        "subTitle" => format!("副标题占位符 {cnv}"),
        "body" => format!("文本占位符 {cnv}"),
        "dt" => format!("日期占位符 {cnv}"),
        "ftr" => format!("页脚占位符 {cnv}"),
        "sldNum" => format!("幻灯片编号占位符 {cnv}"),
        _ => format!("占位符 {cnv}"),
    }
}

fn build_placeholder(
    cnv: u32,
    name: &str,
    type_name: &str,
    idx: Option<u32>,
    x: i64,
    y: i64,
    cx: i64,
    cy: i64,
) -> Element {
    let mut sp = Element::new("p:sp");
    let mut nv = Element::new("p:nvSpPr");
    nv.push_element(
        Element::new("p:cNvPr")
            .with_attr("id", cnv.to_string())
            .with_attr("name", name),
    );
    let mut cnv_sp = Element::new("p:cNvSpPr");
    cnv_sp.push_element(Element::new("a:spLocks").with_attr("noGrp", "1"));
    nv.push_element(cnv_sp);
    let mut ph = Element::new("p:ph").with_attr("type", type_name);
    if let Some(idx) = idx {
        ph = ph.with_attr("idx", idx.to_string());
    }
    if type_name == "body" {
        ph = ph.with_attr("sz", "quarter");
    }
    let mut nv_pr = Element::new("p:nvPr");
    nv_pr.push_element(ph);
    nv.push_element(nv_pr);
    sp.push_element(nv);

    let mut sp_pr = Element::new("p:spPr");
    let mut xfrm = Element::new("a:xfrm");
    xfrm.push_element(
        Element::new("a:off")
            .with_attr("x", x.to_string())
            .with_attr("y", y.to_string()),
    );
    xfrm.push_element(
        Element::new("a:ext")
            .with_attr("cx", cx.to_string())
            .with_attr("cy", cy.to_string()),
    );
    sp_pr.push_element(xfrm);
    let mut geom = Element::new("a:prstGeom").with_attr("prst", "rect");
    geom.push_element(Element::new("a:avLst"));
    sp_pr.push_element(geom);
    sp_pr.push_element(Element::new("a:noFill"));
    let mut ln = Element::new("a:ln");
    ln.push_element(Element::new("a:noFill"));
    sp_pr.push_element(ln);
    sp.push_element(sp_pr);

    let mut tx = Element::new("p:txBody");
    tx.push_element(Element::new("a:bodyPr"));
    tx.push_element(Element::new("a:lstStyle"));
    tx.push_element(Element::new("a:p"));
    sp.push_element(tx);
    sp
}

fn remove_placeholder(root: &mut Element, key: &ShapeKey) -> bool {
    let mut idx = 0usize;
    remove_placeholder_in(root, &mut idx, key)
}

fn remove_placeholder_in(el: &mut Element, idx: &mut usize, key: &ShapeKey) -> bool {
    let mut i = 0;
    while i < el.children.len() {
        let hit_ac = match &el.children[i] {
            Node::Element(e) if e.local_name() == "AlternateContent" => {
                let mut hit = false;
                let mut n = *idx;
                shape::walk_choice(e, &mut |sp| {
                    if hit || !is_placeholder(sp) {
                        return;
                    }
                    n += 1;
                    let ok = match key {
                        ShapeKey::Index(req) => n == *req,
                        ShapeKey::Id(req) => shape::cnv_pr(sp)
                            .and_then(|c| c.attr("id"))
                            .is_some_and(|id| shape::ids_match(id, req)),
                    };
                    if ok {
                        hit = true;
                    }
                });
                if hit {
                    *idx = n;
                    true
                } else {
                    *idx = n;
                    false
                }
            }
            _ => false,
        };
        if hit_ac {
            el.children.remove(i);
            return true;
        }
        let is_hit = match &el.children[i] {
            Node::Element(e) if is_placeholder(e) => {
                *idx += 1;
                match key {
                    ShapeKey::Index(n) => *idx == *n,
                    ShapeKey::Id(req) => shape::cnv_pr(e)
                        .and_then(|c| c.attr("id"))
                        .is_some_and(|id| shape::ids_match(id, req)),
                }
            }
            _ => false,
        };
        if is_hit {
            el.children.remove(i);
            return true;
        }
        if let Node::Element(e) = &mut el.children[i] {
            if e.local_name() != "AlternateContent" && remove_placeholder_in(e, idx, key) {
                return true;
            }
        }
        i += 1;
    }
    false
}
