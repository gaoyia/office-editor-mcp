use crate::error::{Error, ErrorKind, RelSource};
use crate::opc::{office_document_part, resolve_rel_target, ContentTypes, Package, Relationships};
use crate::xml::{self, Document, Element, Node, DEFAULT_DECL};

use super::super::picture::{self};
use super::common::*;
use super::InsertSlicerOp;

pub(crate) fn insert_pivot(
    pkg: &mut Package,
    wb_doc: &mut Document,
    wb_part: &str,
    sheet_doc: &mut Document,
    sheet_part: &str,
    op: &InsertSlicerOp,
    index: usize,
    op_index: usize,
) -> Result<(), Error> {
    let meta = pivot_field(
        pkg, wb_part, sheet_part, &op.sheet, index, &op.field, op_index,
    )?;
    let tabular_id = ensure_x14_pivot_cache_id(pkg, wb_part, &meta.cache_id)?;
    let tab_id = sheet_id(wb_doc, &op.sheet).unwrap_or(1);
    let cx = op.cx.unwrap_or(DEFAULT_CX);
    let cy = op.cy.unwrap_or(DEFAULT_CY);
    let (to, to_off_x, to_off_y) = dest_to(&sheet_doc.root, op, cx, cy);
    let cache_name = unique_cache_name(pkg, wb_doc, &format!("Slicer_{}", op.field));
    let view_name = unique_view_name(pkg, &op.field);
    let cache_part = next_part(pkg, "/xl/slicerCaches/slicerCache", "xml");
    pkg.set(
        &cache_part,
        pivot_cache_xml(
            &cache_name,
            &op.field,
            tab_id,
            &meta.name,
            &tabular_id,
            meta.item_count,
        ),
    )?;
    let cache_rid = add_rel(pkg, wb_part, REL_SLICER_CACHE, &cache_part)?;
    push_wb_cache(wb_doc, false, &cache_rid);
    push_cache_defined_name(wb_doc, &cache_name);
    let (slicer_part, slicer_rid, is_new) = ensure_slicers_part(pkg, sheet_doc, sheet_part)?;
    append_slicer_view(
        pkg,
        &slicer_part,
        &view_name,
        &cache_name,
        &op.field,
        &op.view,
    )?;
    if is_new {
        push_sheet_list(&mut sheet_doc.root, &slicer_rid);
    }
    let mut cts = ContentTypes::from_package(pkg)?;
    cts.set_override(&cache_part, CT_SLICER_CACHE)?;
    cts.set_override(&slicer_part, CT_SLICER)?;
    cts.write_to(pkg)?;
    write_drawing(
        pkg,
        sheet_doc,
        sheet_part,
        op,
        &view_name,
        SlicerFrameKind::Pivot,
        cx,
        cy,
        to,
        to_off_x,
        to_off_y,
    )?;
    Ok(())
}

pub(crate) fn pivot_name_of(root: &Element) -> Option<String> {
    let mut name = None;
    picture::walk_all(root, &mut |e| {
        if e.local_name() == "pivotTable" && name.is_none() {
            name = e.attr("name").filter(|s| !s.is_empty()).map(str::to_string);
        }
    });
    name
}

pub(crate) struct PivotMeta {
    pub(crate) name: String,
    pub(crate) cache_id: String,
    pub(crate) item_count: usize,
    pub(crate) part: String,
}

pub(crate) fn pivot_field(
    pkg: &Package,
    wb_part: &str,
    sheet_part: &str,
    sheet_name: &str,
    index: usize,
    field: &str,
    op_index: usize,
) -> Result<PivotMeta, Error> {
    let rels = Relationships::from_package(pkg, sheet_part)?;
    let mut n = 0usize;
    for rel in &rels.rels {
        if !rel.rel_type.ends_with("/pivotTable") {
            continue;
        }
        n += 1;
        if n != index {
            continue;
        }
        let part = resolve_rel_target(sheet_part, &rel.target)?;
        let doc = super::super::list_pin::parse_part(pkg, &part)?;
        let name = doc.root.attr("name").unwrap_or("").to_string();
        let cache_id = doc.root.attr("cacheId").unwrap_or("").to_string();
        let item_count = cache_item_count(pkg, wb_part, &cache_id, field);
        if !cache_has_field(pkg, wb_part, &cache_id, field) {
            return Err(field_err(field, op_index));
        }
        let _ = sheet_name;
        return Ok(PivotMeta {
            name,
            cache_id,
            item_count,
            part,
        });
    }
    Err(Error::with_path(
        ErrorKind::PivotTableNotFound {
            id: format!("sheet:{sheet_name}/pivot:{index}"),
        },
        format!("ops[{op_index}].source"),
    ))
}

pub(crate) fn cache_has_field(pkg: &Package, wb_part: &str, cache_id: &str, field: &str) -> bool {
    cache_field_el(pkg, wb_part, cache_id, field).is_some()
}

pub(crate) fn cache_item_count(pkg: &Package, wb_part: &str, cache_id: &str, field: &str) -> usize {
    let Some(cf) = cache_field_el(pkg, wb_part, cache_id, field) else {
        return 0;
    };
    cf.child_by_local("sharedItems")
        .map(|s| {
            s.element_children()
                .filter(|c| c.local_name() != "extLst")
                .count()
        })
        .unwrap_or(0)
}

pub(crate) fn bump_pivot_updated_version(pkg: &mut Package, part: &str) -> Result<(), Error> {
    const MIN: u32 = 8;
    let mut doc = super::super::list_pin::parse_part(pkg, part)?;
    let cur = doc
        .root
        .attr("updatedVersion")
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(0);
    if cur >= MIN {
        return Ok(());
    }
    doc.root.set_attr("updatedVersion", MIN.to_string());
    super::super::list_pin::write_part(pkg, part, &doc)?;
    Ok(())
}

pub(crate) fn ensure_x14_pivot_cache_id(
    pkg: &mut Package,
    wb_part: &str,
    workbook_cache_id: &str,
) -> Result<String, Error> {
    let part = pivot_cache_def_part(pkg, wb_part, workbook_cache_id)?;
    let mut doc = super::super::list_pin::parse_part(pkg, &part)?;
    let fallback = if workbook_cache_id.is_empty() || workbook_cache_id == "0" {
        "1".to_string()
    } else {
        workbook_cache_id.to_string()
    };
    let id = ensure_pcd_ext(&mut doc.root, &fallback);
    super::super::list_pin::write_part(pkg, &part, &doc)?;
    Ok(id)
}

pub(crate) fn pivot_cache_def_part(
    pkg: &Package,
    wb_part: &str,
    cache_id: &str,
) -> Result<String, Error> {
    let wb = super::super::list_pin::parse_part(pkg, wb_part)?;
    let parent = wb.root.child_by_local("pivotCaches").ok_or_else(|| {
        Error::new(ErrorKind::XmlMissing {
            parent: "workbook".into(),
            child: "pivotCaches".into(),
        })
    })?;
    let rels = Relationships::from_package(pkg, wb_part)?;
    for el in parent.children_by_local("pivotCache") {
        if el.attr("cacheId") != Some(cache_id) {
            continue;
        }
        let rid = el.attr_local("id").ok_or_else(|| {
            Error::new(ErrorKind::XmlMissing {
                parent: "pivotCache".into(),
                child: "r:id".into(),
            })
        })?;
        let rel = rels.find_by_id(rid).ok_or_else(|| {
            Error::new(ErrorKind::RelNotFound {
                source: RelSource::Part(wb_part.to_string()),
                rid: rid.to_string(),
            })
        })?;
        return resolve_rel_target(wb_part, &rel.target);
    }
    Err(Error::new(ErrorKind::XmlMissing {
        parent: "pivotCaches".into(),
        child: "pivotCache".into(),
    }))
}

pub(crate) fn ensure_pcd_ext(root: &mut Element, fallback: &str) -> String {
    if root.child_by_local("extLst").is_none() {
        root.push_element(Element::new("extLst"));
    }
    let ext_lst = root.child_by_local_mut("extLst").unwrap();
    let found = ext_lst.children.iter().position(|n| match n {
        Node::Element(ext) => uri_eq(ext.attr("uri").unwrap_or(""), PIVOT_CACHE_DEF_EXT_URI),
        _ => false,
    });
    if found.is_none() {
        let mut ext = Element::new("ext")
            .with_attr("uri", PIVOT_CACHE_DEF_EXT_URI)
            .with_attr("xmlns:x14", X14_NS);
        ext.push_element(
            Element::new("x14:pivotCacheDefinition").with_attr("pivotCacheId", fallback),
        );
        ext_lst.push_element(ext);
        return fallback.to_string();
    }
    let idx = found.unwrap();
    let Node::Element(ext) = &mut ext_lst.children[idx] else {
        return fallback.to_string();
    };
    picture::ensure_xmlns(ext, "x14", X14_NS);
    let has_pcd = ext
        .element_children()
        .any(|e| e.local_name() == "pivotCacheDefinition");
    if !has_pcd {
        ext.push_element(
            Element::new("x14:pivotCacheDefinition").with_attr("pivotCacheId", fallback),
        );
        return fallback.to_string();
    }
    let pcd = ext
        .element_children_mut()
        .find(|e| e.local_name() == "pivotCacheDefinition")
        .unwrap();
    let cur = pcd.attr("pivotCacheId").unwrap_or("").to_string();
    if cur.is_empty() || cur == "0" {
        pcd.set_attr("pivotCacheId", fallback);
        return fallback.to_string();
    }
    cur
}

pub(crate) fn cache_field_el(
    pkg: &Package,
    wb_part: &str,
    cache_id: &str,
    field: &str,
) -> Option<Element> {
    let wb = xml::parse(pkg.get(wb_part).ok()?).ok()?;
    let parent = wb.root.child_by_local("pivotCaches")?;
    let rels = Relationships::from_package(pkg, wb_part).ok()?;
    for el in parent.children_by_local("pivotCache") {
        if el.attr("cacheId") != Some(cache_id) {
            continue;
        }
        let rid = el.attr_local("id")?;
        let rel = rels.find_by_id(rid)?;
        let part = resolve_rel_target(wb_part, &rel.target).ok()?;
        let doc = xml::parse(pkg.get(&part).ok()?).ok()?;
        let fields = doc.root.child_by_local("cacheFields")?;
        for cf in fields.children_by_local("cacheField") {
            if cf.attr("name") == Some(field) {
                return Some(cf.clone());
            }
        }
    }
    None
}

pub(crate) fn pivot_cache_xml(
    name: &str,
    field: &str,
    tab_id: u32,
    pivot_name: &str,
    cache_id: &str,
    item_count: usize,
) -> Vec<u8> {
    let mut root = Element::new("slicerCacheDefinition")
        .with_attr("xmlns", X14_NS)
        .with_attr("xmlns:mc", MC_NS)
        .with_attr("mc:Ignorable", "x")
        .with_attr("xmlns:x", SML_NS)
        .with_attr("name", name)
        .with_attr("sourceName", field);
    let mut pts = Element::new("pivotTables");
    pts.push_element(
        Element::new("pivotTable")
            .with_attr("tabId", tab_id.to_string())
            .with_attr("name", pivot_name),
    );
    root.push_element(pts);
    let mut items = Element::new("items").with_attr("count", item_count.to_string());
    for i in 0..item_count {
        items.push_element(
            Element::new("i")
                .with_attr("x", i.to_string())
                .with_attr("s", "1"),
        );
    }
    let mut tabular = Element::new("tabular").with_attr("pivotCacheId", cache_id);
    tabular.push_element(items);
    let mut data = Element::new("data");
    data.push_element(tabular);
    root.push_element(data);
    Document::new(Some(DEFAULT_DECL.to_string()), root)
        .to_bytes()
        .unwrap_or_default()
}

pub(crate) fn pivot_selected(pkg: &Package, cache_root: &Element, field: &str) -> Vec<String> {
    let names = pivot_item_names(pkg, cache_root, field);
    let mut out = Vec::new();
    let Some(items) = find_items(cache_root) else {
        return out;
    };
    for i in items.children_by_local("i") {
        if !attr_is_true(i, "s") {
            continue;
        }
        let Some(x) = i.attr("x").and_then(|s| s.parse::<usize>().ok()) else {
            continue;
        };
        if let Some(n) = names.get(x) {
            out.push(n.clone());
        }
    }
    out
}

pub(crate) fn set_pivot_selected(
    pkg: &mut Package,
    cache: &CacheMeta,
    selected: &[String],
    op_index: usize,
) -> Result<(), Error> {
    let mut doc = super::super::list_pin::parse_part(pkg, &cache.part)?;
    let names = pivot_item_names(pkg, &doc.root, &cache.field);
    let mut want = Vec::new();
    for (k, name) in selected.iter().enumerate() {
        let Some(x) = names.iter().position(|n| n == name) else {
            return Err(Error::with_path(
                ErrorKind::InvalidSlicer {
                    field: "selected".into(),
                },
                format!("ops[{op_index}].selected[{k}]"),
            ));
        };
        if !want.contains(&x) {
            want.push(x);
        }
    }
    let Some(items) = find_items_mut(&mut doc.root) else {
        if selected.is_empty() {
            return Ok(());
        }
        return Err(unknown_selected(op_index));
    };
    for el in items.element_children_mut() {
        if el.local_name() != "i" {
            continue;
        }
        let Some(x) = el.attr("x").and_then(|s| s.parse::<usize>().ok()) else {
            continue;
        };
        if want.contains(&x) {
            el.set_attr("s", "1");
        } else {
            el.remove_attr("s");
        }
    }
    for x in &want {
        let exists = items.element_children().any(|el| {
            el.local_name() == "i" && el.attr("x").and_then(|s| s.parse::<usize>().ok()) == Some(*x)
        });
        if !exists {
            items.push_element(
                Element::new("i")
                    .with_attr("x", x.to_string())
                    .with_attr("s", "1"),
            );
        }
    }
    let n = items.children_by_local("i").len();
    items.set_attr("count", n.to_string());
    super::super::list_pin::write_part(pkg, &cache.part, &doc)?;
    Ok(())
}

pub(crate) fn pivot_item_names(pkg: &Package, cache_root: &Element, field: &str) -> Vec<String> {
    let Ok(wb_part) = office_document_part(pkg) else {
        return Vec::new();
    };
    let mut cache_id = String::new();
    picture::walk_all(cache_root, &mut |e| {
        if e.local_name() == "tabular" && cache_id.is_empty() {
            if let Some(v) = e.attr("pivotCacheId") {
                cache_id = v.to_string();
            }
        }
    });
    let Some(cf) = cache_field_by_x14(pkg, &wb_part, &cache_id, field)
        .or_else(|| cache_field_el(pkg, &wb_part, &cache_id, field))
    else {
        return Vec::new();
    };
    let Some(shared) = cf.child_by_local("sharedItems") else {
        return Vec::new();
    };
    shared
        .element_children()
        .filter(|c| c.local_name() != "extLst")
        .map(shared_item_text)
        .collect()
}

pub(crate) fn shared_item_text(el: &Element) -> String {
    match el.local_name() {
        "m" => String::new(),
        _ => el.attr("v").unwrap_or("").to_string(),
    }
}

pub(crate) fn cache_field_by_x14(
    pkg: &Package,
    wb_part: &str,
    x14_id: &str,
    field: &str,
) -> Option<Element> {
    if x14_id.is_empty() {
        return None;
    }
    let wb = xml::parse(pkg.get(wb_part).ok()?).ok()?;
    let parent = wb.root.child_by_local("pivotCaches")?;
    let rels = Relationships::from_package(pkg, wb_part).ok()?;
    for el in parent.children_by_local("pivotCache") {
        let rid = el.attr_local("id")?;
        let rel = rels.find_by_id(rid)?;
        let part = resolve_rel_target(wb_part, &rel.target).ok()?;
        let doc = xml::parse(pkg.get(&part).ok()?).ok()?;
        let mut found = false;
        picture::walk_all(&doc.root, &mut |e| {
            if e.local_name() == "pivotCacheDefinition" && e.attr("pivotCacheId") == Some(x14_id) {
                found = true;
            }
        });
        if !found {
            continue;
        }
        let fields = doc.root.child_by_local("cacheFields")?;
        for cf in fields.children_by_local("cacheField") {
            if cf.attr("name") == Some(field) {
                return Some(cf.clone());
            }
        }
    }
    None
}

pub(crate) fn find_items(root: &Element) -> Option<&Element> {
    let data = root.child_by_local("data")?;
    let tabular = data.child_by_local("tabular")?;
    tabular.child_by_local("items")
}

pub(crate) fn find_items_mut(root: &mut Element) -> Option<&mut Element> {
    let data = root.child_by_local_mut("data")?;
    let tabular = data.child_by_local_mut("tabular")?;
    tabular.child_by_local_mut("items")
}
