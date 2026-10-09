//! 备注页 `notesSlide`：名单、改演讲者备注、新建最小 Transitional 部件。

use serde_json::{json, Map, Value};

use crate::error::{Error, ErrorKind};
use crate::opc::{self, ContentTypes, Package, Relationship, Relationships};
use crate::xml::{self, Document, Element, Node};

use super::picture;
use super::request::SetNotesTextOp;
use super::shape::{self, ShapeKey};
use super::{CT_THEME, DML_NS, PML_NS, REL_NS, REL_THEME};

pub const REL_NOTES_SLIDE: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/notesSlide";
pub const REL_NOTES_MASTER: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/notesMaster";
pub const CT_NOTES_SLIDE: &str =
    "application/vnd.openxmlformats-officedocument.presentationml.notesSlide+xml";
pub const CT_NOTES_MASTER: &str =
    "application/vnd.openxmlformats-officedocument.presentationml.notesMaster+xml";

const DEFAULT_NOTES_CX: i64 = 6_858_000;
const DEFAULT_NOTES_CY: i64 = 9_144_000;

/// 对照 PowerPoint 16.0 另存的 notesMaster，去掉 `p14:creationId`。
const NOTES_MASTER_XML: &str = include_str!("notes_master.xml");

pub fn notes_slide_part(pkg: &Package, slide_part: &str) -> Result<Option<String>, Error> {
    let rels = Relationships::from_package(pkg, slide_part)?;
    let Some(rel) = rels
        .rels
        .iter()
        .find(|r| r.rel_type.ends_with("/notesSlide"))
    else {
        return Ok(None);
    };
    Ok(Some(opc::resolve_id(pkg, slide_part, &rel.id)?))
}

pub fn has_notes(pkg: &Package, slide_part: &str) -> Result<bool, Error> {
    Ok(notes_slide_part(pkg, slide_part)?.is_some())
}

pub fn notes_size(pres_root: &Element) -> (i64, i64) {
    let Some(sz) = pres_root.child_by_local("notesSz") else {
        return (DEFAULT_NOTES_CX, DEFAULT_NOTES_CY);
    };
    let cx = sz
        .attr("cx")
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_NOTES_CX);
    let cy = sz
        .attr("cy")
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_NOTES_CY);
    (cx, cy)
}

pub fn placeholder_type(sp: &Element) -> Option<&str> {
    sp.child_by_local("nvSpPr")
        .and_then(|nv| nv.child_by_local("nvPr"))
        .and_then(|nv| nv.child_by_local("ph"))
        .and_then(|ph| ph.attr("type"))
}

pub fn is_slide_image(sp: &Element) -> bool {
    placeholder_type(sp) == Some("sldImg")
}

pub fn notes_json(
    pkg: &Package,
    slide_part: &str,
    slide: usize,
    want_pictures: bool,
    want_charts: bool,
    want_smartarts: bool,
    want_tables: bool,
    want_model3ds: bool,
    want_maths: bool,
    want_timing: bool,
) -> Result<Value, Error> {
    let Some(part) = notes_slide_part(pkg, slide_part)? else {
        return Ok(Value::Null);
    };
    let doc = xml::parse(pkg.get(&part)?)?;
    let host = format!("slide:{slide}/notes");
    let rels = crate::opc::Relationships::from_package(pkg, &part)?;
    let pics = picture::collect_pictures_with(&doc.root, &host, Some(&rels));
    let charts_list = if want_charts {
        Some(super::chart::collect_charts(pkg, &doc.root, &host, &part)?)
    } else {
        None
    };
    let n_charts = charts_list
        .as_ref()
        .map(|c| c.len())
        .unwrap_or_else(|| super::chart::count_charts(&doc.root));
    let smartarts_list = if want_smartarts {
        Some(super::smartart::collect_smartarts(
            pkg, &doc.root, &host, &part,
        )?)
    } else {
        None
    };
    let n_smartarts = smartarts_list
        .as_ref()
        .map(|c| c.len())
        .unwrap_or_else(|| super::smartart::count_smartarts(&doc.root));
    let tables_list = if want_tables {
        Some(super::table::collect_tables(&doc.root, &host))
    } else {
        None
    };
    let n_tables = tables_list
        .as_ref()
        .map(|c| c.len())
        .unwrap_or_else(|| super::table::count_tables(&doc.root));
    let all = shape::collect_text_shapes(&doc.root);
    let mut shapes = Vec::new();
    for (i, sp) in all.iter().enumerate() {
        let index = i + 1;
        shapes.push(shape::shape_node_json(sp, &host, index));
    }
    let mut obj = Map::new();
    obj.insert("id".into(), json!(host));
    obj.insert("pictureCount".into(), json!(pics.len()));
    let maths = super::omml::collect(&doc.root, &host);
    obj.insert("mathCount".into(), json!(maths.len()));
    obj.insert("chartCount".into(), json!(n_charts));
    obj.insert("smartArtCount".into(), json!(n_smartarts));
    obj.insert("tableCount".into(), json!(n_tables));
    let m3ds = super::model3d::collect(&doc.root, &host);
    obj.insert("model3dCount".into(), json!(m3ds.len()));
    let anims = super::timing::list(&doc.root, &host, false);
    obj.insert("animCount".into(), json!(anims.len()));
    if want_pictures {
        obj.insert(
            "pictures".into(),
            Value::Array(picture::pictures_json(&pics)),
        );
    }
    if want_maths {
        obj.insert("maths".into(), Value::Array(super::omml::list_json(&maths)));
    }
    if let Some(charts) = charts_list {
        obj.insert("charts".into(), Value::Array(charts));
    }
    if let Some(smartarts) = smartarts_list {
        obj.insert("smartArts".into(), Value::Array(smartarts));
    }
    if let Some(tables) = tables_list {
        obj.insert("tables".into(), Value::Array(tables));
    }
    if want_model3ds {
        obj.insert(
            "model3ds".into(),
            Value::Array(super::model3d::list_json(&m3ds)),
        );
    }
    if want_timing {
        obj.insert("timing".into(), Value::Array(anims));
    }
    obj.insert("shapes".into(), Value::Array(shapes));
    Ok(Value::Object(obj))
}

pub fn ensure_notes_slide(
    pkg: &mut Package,
    pres_part: &str,
    pres: &mut Document,
    slide_part: &str,
) -> Result<String, Error> {
    if let Some(part) = notes_slide_part(pkg, slide_part)? {
        return Ok(part);
    }
    let master = ensure_notes_master(pkg, pres_part, pres)?;
    let notes_part = next_part(pkg, "/ppt/notesSlides", "notesSlide");
    pkg.set(&notes_part, notes_slide_xml().into_bytes())?;
    let mut notes_rels = Relationships { rels: Vec::new() };
    notes_rels.push(Relationship {
        id: "rId1".into(),
        rel_type: REL_NOTES_MASTER.into(),
        target: picture::rel_target_from(&notes_part, &master),
        target_mode: None,
        extra: Vec::new(),
    });
    notes_rels.push(Relationship {
        id: "rId2".into(),
        rel_type: super::REL_SLIDE.into(),
        target: picture::rel_target_from(&notes_part, slide_part),
        target_mode: None,
        extra: Vec::new(),
    });
    notes_rels.write_to(pkg, &notes_part)?;
    let mut slide_rels = Relationships::from_package(pkg, slide_part)?;
    let rid = slide_rels.next_rid();
    slide_rels.push(Relationship {
        id: rid,
        rel_type: REL_NOTES_SLIDE.into(),
        target: picture::rel_target_from(slide_part, &notes_part),
        target_mode: None,
        extra: Vec::new(),
    });
    slide_rels.write_to(pkg, slide_part)?;
    let mut cts = ContentTypes::from_package(pkg)?;
    cts.set_override(&notes_part, CT_NOTES_SLIDE)?;
    cts.write_to(pkg)?;
    Ok(notes_part)
}

pub fn apply_set_notes_text(
    pkg: &mut Package,
    pres_part: &str,
    pres: &mut Document,
    slide_part: &str,
    op: &SetNotesTextOp,
    op_index: usize,
) -> Result<(), Error> {
    let notes_part = ensure_notes_slide(pkg, pres_part, pres, slide_part)?;
    let mut doc = xml::parse(pkg.get(&notes_part)?)?;
    if find_body_mut(&mut doc.root).is_none() {
        append_body_placeholder(&mut doc.root)?;
    }
    let body = find_body_mut(&mut doc.root).ok_or_else(|| {
        Error::with_path(
            ErrorKind::ShapeNotFound { id: op.id.clone() },
            format!("ops[{op_index}].id"),
        )
    })?;
    let tx = body.child_by_local_mut("txBody").ok_or_else(|| {
        Error::with_path(
            ErrorKind::ShapeHasNoText { id: op.id.clone() },
            format!("ops[{op_index}].id"),
        )
    })?;
    shape::set_txbody_text(tx, &op.text);
    pkg.set(&notes_part, doc.to_bytes()?)?;
    Ok(())
}

fn ensure_notes_master(
    pkg: &mut Package,
    pres_part: &str,
    pres: &mut Document,
) -> Result<String, Error> {
    let mut pres_rels = Relationships::from_package(pkg, pres_part)?;
    if let Some(rel) = pres_rels
        .rels
        .iter()
        .find(|r| r.rel_type.ends_with("/notesMaster"))
    {
        return opc::resolve_id(pkg, pres_part, &rel.id);
    }
    let master = next_part(pkg, "/ppt/notesMasters", "notesMaster");
    pkg.set(&master, NOTES_MASTER_XML.as_bytes().to_vec())?;
    // create 最小包的 theme 挂在 slideMaster 上，presentation 没有 theme 关系。
    let cloned_theme = attach_cloned_theme(pkg, pres_part, &pres_rels, &master)?;
    let rid = pres_rels.next_rid();
    pres_rels.push(Relationship {
        id: rid.clone(),
        rel_type: REL_NOTES_MASTER.into(),
        target: picture::rel_target_from(pres_part, &master),
        target_mode: None,
        extra: Vec::new(),
    });
    pres_rels.write_to(pkg, pres_part)?;
    insert_notes_master_id(&mut pres.root, &rid);
    pkg.set(pres_part, pres.to_bytes()?)?;
    let mut cts = ContentTypes::from_package(pkg)?;
    cts.set_override(&master, CT_NOTES_MASTER)?;
    if let Some(theme) = cloned_theme.as_deref() {
        cts.set_override(theme, CT_THEME)?;
    }
    cts.write_to(pkg)?;
    Ok(master)
}

/// PowerPoint 16.0 拒开与 slideMaster 共用同一 theme 部件的备注母版或讲义母版；复制一份独立 theme。
pub(crate) fn attach_cloned_theme(
    pkg: &mut Package,
    pres_part: &str,
    pres_rels: &Relationships,
    master: &str,
) -> Result<Option<String>, Error> {
    let Some(src) = theme_part(pkg, pres_part, pres_rels)? else {
        return Ok(None);
    };
    let cloned = next_part(pkg, "/ppt/theme", "theme");
    pkg.set(&cloned, pkg.get(&src)?.to_vec())?;
    let mut master_rels = Relationships { rels: Vec::new() };
    master_rels.push(Relationship {
        id: "rId1".into(),
        rel_type: REL_THEME.into(),
        target: picture::rel_target_from(master, &cloned),
        target_mode: None,
        extra: Vec::new(),
    });
    master_rels.write_to(pkg, master)?;
    Ok(Some(cloned))
}

pub(crate) fn theme_part(
    pkg: &Package,
    pres_part: &str,
    pres_rels: &Relationships,
) -> Result<Option<String>, Error> {
    if let Some(theme) = pres_rels
        .rels
        .iter()
        .find(|r| r.rel_type.ends_with("/theme"))
    {
        return Ok(Some(opc::resolve_id(pkg, pres_part, &theme.id)?));
    }
    for rel in pres_rels
        .rels
        .iter()
        .filter(|r| r.rel_type.ends_with("/slideMaster"))
    {
        let master = opc::resolve_id(pkg, pres_part, &rel.id)?;
        let master_rels = Relationships::from_package(pkg, &master)?;
        if let Some(theme) = master_rels
            .rels
            .iter()
            .find(|r| r.rel_type.ends_with("/theme"))
        {
            return Ok(Some(opc::resolve_id(pkg, &master, &theme.id)?));
        }
    }
    Ok(None)
}

fn insert_notes_master_id(pres_root: &mut Element, rid: &str) {
    if pres_root.child_by_local("notesMasterIdLst").is_some() {
        return;
    }
    let mut item = Element::new(pres_root.prefixed("notesMasterId"));
    item.attrs.push(("r:id".into(), rid.to_string()));
    let mut lst = Element::new(pres_root.prefixed("notesMasterIdLst"));
    lst.push_element(item);
    let after = pres_root
        .children
        .iter()
        .position(|n| matches!(n, Node::Element(e) if e.local_name() == "sldMasterIdLst"));
    if let Some(i) = after {
        pres_root.children.insert(i + 1, Node::Element(lst));
        return;
    }
    let before = pres_root
        .children
        .iter()
        .position(|n| matches!(n, Node::Element(e) if e.local_name() == "sldIdLst"));
    if let Some(i) = before {
        pres_root.children.insert(i, Node::Element(lst));
    } else {
        pres_root.push_element(lst);
    }
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

fn notes_slide_xml() -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><p:notes xmlns:a="{DML_NS}" xmlns:r="{REL_NS}" xmlns:p="{PML_NS}"><p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/><a:chOff x="0" y="0"/><a:chExt cx="0" cy="0"/></a:xfrm></p:grpSpPr><p:sp><p:nvSpPr><p:cNvPr id="2" name="幻灯片图像占位符 1"/><p:cNvSpPr><a:spLocks noGrp="1" noRot="1" noChangeAspect="1"/></p:cNvSpPr><p:nvPr><p:ph type="sldImg"/></p:nvPr></p:nvSpPr><p:spPr/></p:sp><p:sp><p:nvSpPr><p:cNvPr id="3" name="备注占位符 2"/><p:cNvSpPr><a:spLocks noGrp="1"/></p:cNvSpPr><p:nvPr><p:ph type="body" idx="1"/></p:nvPr></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:endParaRPr lang="zh-CN" altLang="en-US"/></a:p></p:txBody></p:sp><p:sp><p:nvSpPr><p:cNvPr id="4" name="灯片编号占位符 3"/><p:cNvSpPr><a:spLocks noGrp="1"/></p:cNvSpPr><p:nvPr><p:ph type="sldNum" sz="quarter" idx="5"/></p:nvPr></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:fld id="{{68E3DCAC-8797-4710-9773-FBA65FDE8677}}" type="slidenum"><a:rPr lang="zh-CN" altLang="en-US" smtClean="0"/><a:t>‹#›</a:t></a:fld><a:endParaRPr lang="zh-CN" altLang="en-US"/></a:p></p:txBody></p:sp></p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:notes>"#
    )
}

fn find_body_mut(root: &mut Element) -> Option<&mut Element> {
    find_sp_ph_mut(root, "body")
}

fn find_sp_ph_mut<'a>(el: &'a mut Element, ph: &str) -> Option<&'a mut Element> {
    if el.local_name() == "AlternateContent" {
        if let Some(i) = shape::first_alt_child_index(el) {
            if let Node::Element(child) = &mut el.children[i] {
                return find_sp_ph_mut(child, ph);
            }
        }
        return None;
    }
    if el.local_name() == "sp" && placeholder_type(el) == Some(ph) {
        return Some(el);
    }
    for child in el.children.as_mut_slice() {
        if let Node::Element(e) = child {
            if let Some(found) = find_sp_ph_mut(e, ph) {
                return Some(found);
            }
        }
    }
    None
}

fn append_body_placeholder(root: &mut Element) -> Result<(), Error> {
    let cnv = picture::next_cnv_id(root);
    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?><p:sp xmlns:a="{DML_NS}" xmlns:p="{PML_NS}"><p:nvSpPr><p:cNvPr id="{cnv}" name="备注占位符"/><p:cNvSpPr><a:spLocks noGrp="1"/></p:cNvSpPr><p:nvPr><p:ph type="body" idx="1"/></p:nvPr></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:endParaRPr lang="zh-CN"/></a:p></p:txBody></p:sp>"#
    );
    let frag = xml::parse(xml.as_bytes())?;
    picture::insert_into_sp_tree(root, frag.root)
}

pub fn reject_slide_image(sp: &Element, op_index: usize) -> Result<(), Error> {
    if is_slide_image(sp) {
        return Err(Error::with_path(
            ErrorKind::NotesSlideImage,
            format!("ops[{op_index}].id"),
        ));
    }
    Ok(())
}

pub fn find_any_sp_by_key<'a>(root: &'a Element, key: &ShapeKey) -> Option<&'a Element> {
    let mut found = None;
    shape::walk_choice(root, &mut |e| {
        if found.is_some() || e.local_name() != "sp" {
            return;
        }
        let hit = match key {
            ShapeKey::Index(_) => false,
            ShapeKey::Id(req) => e
                .child_by_local("nvSpPr")
                .and_then(|nv| nv.child_by_local("cNvPr"))
                .and_then(|c| c.attr("id"))
                .is_some_and(|id| ids_match(id, req)),
        };
        if hit {
            found = Some(e);
        }
    });
    found
}

fn ids_match(file: &str, req: &str) -> bool {
    match (file.parse::<u64>(), req.parse::<u64>()) {
        (Ok(a), Ok(b)) => a == b,
        _ => file == req,
    }
}
