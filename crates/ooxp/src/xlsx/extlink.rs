//! 工作簿外部引用：inspect 名单，apply 写 `/xl/externalLinks/…`。
//! 不打开目标文件，不求值，不把缓存值写进 `sheetDataSet`。

use serde_json::{json, Map, Value};

use crate::error::{Error, ErrorKind, IdReason, RelSource};
use crate::opc::{
    rels_part_name, resolve_rel_target, ContentTypes, Package, Relationship, Relationships,
};
use crate::xml::{self, Document, Element, Node, DEFAULT_DECL};

use super::names;
use super::picture;
use super::{REL_NS, SML_NS};

const REL_EXTERNAL_LINK: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/externalLink";
const REL_EXTERNAL_LINK_PATH: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/externalLinkPath";
const CT_EXTERNAL_LINK: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.externalLink+xml";

#[derive(Debug, Clone)]
pub struct InsertExternalLinkOp {
    pub path: String,
    pub sheets: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct DeleteExternalLinkOp {
    pub id: String,
    pub index: usize,
}

#[derive(Debug, Clone)]
pub struct SetExternalLinkOp {
    pub id: String,
    pub index: usize,
    pub path: String,
}

pub fn parse_external_link_id(id: &str, json_path: &str) -> Result<usize, Error> {
    let rest = id.strip_prefix("extlink:").ok_or_else(|| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::ExternalLinkPrefix,
            },
            json_path.to_string(),
        )
    })?;
    if rest.is_empty() {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::ExternalLinkPrefix,
            },
            json_path.to_string(),
        ));
    }
    let n: usize = rest.parse().map_err(|_| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::ExternalLinkIndexNotInt,
            },
            json_path.to_string(),
        )
    })?;
    if n < 1 {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::ExternalLinkIndexFromOne,
            },
            json_path.to_string(),
        ));
    }
    Ok(n)
}

pub(crate) fn count(wb: &Element) -> usize {
    wb.child_by_local("externalReferences")
        .map(|p| p.children_by_local("externalReference").len())
        .unwrap_or(0)
}

pub(crate) fn inspect(pkg: &Package, wb_part: &str, wb: &Element) -> Result<Value, Error> {
    let Some(parent) = wb.child_by_local("externalReferences") else {
        return Ok(json!([]));
    };
    let wb_rels = Relationships::from_package(pkg, wb_part)?;
    let mut out = Vec::new();
    for (i, el) in parent
        .children_by_local("externalReference")
        .into_iter()
        .enumerate()
    {
        out.push(inspect_one(pkg, wb_part, &wb_rels, el, i + 1)?);
    }
    Ok(Value::Array(out))
}

fn inspect_one(
    pkg: &Package,
    wb_part: &str,
    wb_rels: &Relationships,
    el: &Element,
    index: usize,
) -> Result<Value, Error> {
    let rid = el.attr_local("id").ok_or_else(|| {
        Error::new(ErrorKind::XmlMissing {
            parent: "externalReference".into(),
            child: "r:id".into(),
        })
    })?;
    let rel = wb_rels.find_by_id(rid).ok_or_else(|| {
        Error::new(ErrorKind::RelNotFound {
            source: RelSource::Workbook,
            rid: rid.to_string(),
        })
    })?;
    if rel.is_external() {
        return Err(Error::new(ErrorKind::ExternalRel {
            rid: rid.to_string(),
        }));
    }
    let part = resolve_rel_target(wb_part, &rel.target)?;
    if !pkg.contains(&part) {
        return Err(Error::new(ErrorKind::PartMissing { name: part }));
    }
    let doc = xml::parse(pkg.get(&part)?)?;
    let book = doc.root.child_by_local("externalBook");
    let kind = if book.is_some() { "book" } else { "other" };
    let path = match book {
        Some(b) => path_of(pkg, &part, b)?,
        None => None,
    };
    let sheets = match book {
        Some(b) => b
            .child_by_local("sheetNames")
            .map(|p| {
                p.children_by_local("sheetName")
                    .into_iter()
                    .map(|s| json!(s.attr("val").unwrap_or("")))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default(),
        None => Vec::new(),
    };
    let mut obj = Map::new();
    obj.insert("id".into(), json!(format!("extlink:{index}")));
    obj.insert("index".into(), json!(index));
    obj.insert("kind".into(), json!(kind));
    obj.insert("path".into(), json!(path));
    obj.insert("sheets".into(), Value::Array(sheets));
    Ok(Value::Object(obj))
}

fn path_of(pkg: &Package, part: &str, book: &Element) -> Result<Option<String>, Error> {
    let Some(rid) = book.attr_local("id") else {
        return Ok(None);
    };
    let rels = Relationships::from_package(pkg, part)?;
    Ok(rels.find_by_id(rid).map(|r| r.target.clone()))
}

pub(crate) fn insert(
    pkg: &mut Package,
    wb: &mut Document,
    wb_part: &str,
    op: &InsertExternalLinkOp,
) -> Result<(), Error> {
    let part = next_part(pkg);
    let target = target_from_path(&op.path);
    write_link_part(pkg, &part, &op.sheets, &target)?;
    picture::ensure_xmlns(&mut wb.root, "r", REL_NS);
    let rid = add_workbook_rel(pkg, wb_part, &part)?;
    let parent = ensure_external_references(&mut wb.root);
    parent.push_element(Element::new("externalReference").with_attr("r:id", rid));
    Ok(())
}

pub(crate) fn delete(
    pkg: &mut Package,
    wb: &mut Document,
    wb_part: &str,
    sheet_index: &[(String, String, &'static str)],
    dirty_sheets: &mut Vec<(String, Document)>,
    op: &DeleteExternalLinkOp,
    op_index: usize,
) -> Result<(), Error> {
    if op.index == 0 {
        return Err(not_found(&op.id, op_index));
    }
    break_link_refs(
        pkg,
        &mut wb.root,
        wb_part,
        sheet_index,
        dirty_sheets,
        op.index as u32,
        op_index,
    )?;
    let parent = wb
        .root
        .child_by_local("externalReferences")
        .ok_or_else(|| not_found(&op.id, op_index))?;
    let el = super::list_pin::find_child(parent, "externalReference", "extlink", op.index)
        .ok_or_else(|| not_found(&op.id, op_index))?;
    let rid = el.attr_local("id").map(str::to_string);
    let part = match rid.as_deref() {
        Some(rid) => resolve_internal_part(pkg, wb_part, rid)?,
        None => None,
    };
    drop_reference(&mut wb.root, op.index);
    if let Some(part) = part {
        drop_part(pkg, wb_part, &part)?;
    } else if let Some(rid) = rid {
        let mut rels = Relationships::from_package(pkg, wb_part)?;
        rels.remove_by_id(&rid);
        rels.write_to(pkg, wb_part)?;
    }
    Ok(())
}

pub(crate) fn set_path(
    pkg: &mut Package,
    wb: &Document,
    wb_part: &str,
    op: &SetExternalLinkOp,
    op_index: usize,
) -> Result<(), Error> {
    if op.index == 0 {
        return Err(not_found(&op.id, op_index));
    }
    let parent = wb
        .root
        .child_by_local("externalReferences")
        .ok_or_else(|| not_found(&op.id, op_index))?;
    let el = super::list_pin::find_child(parent, "externalReference", "extlink", op.index)
        .ok_or_else(|| not_found(&op.id, op_index))?;
    let rid = el.attr_local("id").ok_or_else(|| {
        Error::new(ErrorKind::XmlMissing {
            parent: "externalReference".into(),
            child: "r:id".into(),
        })
    })?;
    let Some(part) = resolve_internal_part(pkg, wb_part, rid)? else {
        return Err(not_found(&op.id, op_index));
    };
    let doc = xml::parse(pkg.get(&part)?)?;
    let Some(book) = doc.root.child_by_local("externalBook") else {
        return Err(Error::with_path(
            ErrorKind::InvalidExternalLink {
                field: "kind".into(),
            },
            format!("ops[{op_index}].id"),
        ));
    };
    let book_rid = book.attr_local("id").ok_or_else(|| {
        Error::new(ErrorKind::XmlMissing {
            parent: "externalBook".into(),
            child: "r:id".into(),
        })
    })?;
    let target = target_from_path(&op.path);
    let mut rels = Relationships::from_package(pkg, &part)?;
    let rel = rels.find_by_id_mut(book_rid).ok_or_else(|| {
        Error::new(ErrorKind::RelNotFound {
            source: RelSource::Part(part.clone()),
            rid: book_rid.to_string(),
        })
    })?;
    rel.target = target;
    rels.write_to(pkg, &part)?;
    Ok(())
}

fn write_link_part(
    pkg: &mut Package,
    part: &str,
    sheets: &[String],
    target: &str,
) -> Result<(), Error> {
    let mut names_el = Element::new("sheetNames");
    let mut data = Element::new("sheetDataSet");
    for (i, name) in sheets.iter().enumerate() {
        names_el.push_element(Element::new("sheetName").with_attr("val", name));
        data.push_element(Element::new("sheetData").with_attr("sheetId", i.to_string()));
    }
    let book = Element::new("externalBook")
        .with_attr("r:id", "rId1")
        .push_then(names_el)
        .push_then(data);
    let root = Element::new("externalLink")
        .with_attr("xmlns", SML_NS)
        .with_attr("xmlns:r", REL_NS)
        .push_then(book);
    pkg.set(
        part,
        Document::new(Some(DEFAULT_DECL.to_string()), root).to_bytes()?,
    )?;
    let mut rels = Relationships { rels: Vec::new() };
    rels.push(Relationship {
        id: "rId1".into(),
        rel_type: REL_EXTERNAL_LINK_PATH.into(),
        target: target.to_string(),
        target_mode: Some("External".into()),
        extra: Vec::new(),
    });
    rels.write_to(pkg, part)?;
    let mut cts = ContentTypes::from_package(pkg)?;
    cts.set_override(part, CT_EXTERNAL_LINK)?;
    cts.write_to(pkg)?;
    Ok(())
}

trait PushThen {
    fn push_then(self, child: Element) -> Self;
}

impl PushThen for Element {
    fn push_then(mut self, child: Element) -> Self {
        self.push_element(child);
        self
    }
}

fn add_workbook_rel(pkg: &mut Package, wb_part: &str, part: &str) -> Result<String, Error> {
    let mut rels = Relationships::from_package(pkg, wb_part)?;
    let rid = rels.next_rid();
    rels.push(Relationship {
        id: rid.clone(),
        rel_type: REL_EXTERNAL_LINK.into(),
        target: picture::rel_target_from(wb_part, part),
        target_mode: None,
        extra: Vec::new(),
    });
    rels.write_to(pkg, wb_part)?;
    Ok(rid)
}

fn ensure_external_references(root: &mut Element) -> &mut Element {
    if root.child_by_local("externalReferences").is_none() {
        names::insert_workbook_child(
            root,
            Element::new("externalReferences"),
            "externalReferences",
        );
    }
    root.child_by_local_mut("externalReferences")
        .expect("刚插入的 externalReferences")
}

fn drop_reference(root: &mut Element, index: usize) {
    let remain = {
        let Some(parent) = root.child_by_local_mut("externalReferences") else {
            return;
        };
        let attr = crate::xml::pin_attr("extlink");
        let mut live = 0usize;
        parent.children.retain(|n| match n {
            Node::Element(e) if e.local_name() == "externalReference" => {
                live += 1;
                !crate::xml::pin_matches(e, &attr, index, live)
            }
            _ => true,
        });
        parent.children_by_local("externalReference").len()
    };
    if remain == 0 {
        root.children.retain(|n| match n {
            Node::Element(e) => e.local_name() != "externalReferences",
            _ => true,
        });
    }
}

fn resolve_internal_part(pkg: &Package, wb_part: &str, rid: &str) -> Result<Option<String>, Error> {
    let rels = Relationships::from_package(pkg, wb_part)?;
    let Some(rel) = rels.find_by_id(rid) else {
        return Ok(None);
    };
    if rel.is_external() {
        return Ok(None);
    }
    let part = resolve_rel_target(wb_part, &rel.target)?;
    Ok(Some(part))
}

fn drop_part(pkg: &mut Package, wb_part: &str, part: &str) -> Result<(), Error> {
    pkg.remove(part);
    pkg.remove(&rels_part_name(part));
    let mut rels = Relationships::from_package(pkg, wb_part)?;
    rels.remove_by_target_part(wb_part, part);
    rels.write_to(pkg, wb_part)?;
    let mut cts = ContentTypes::from_package(pkg)?;
    cts.remove_override(part);
    cts.write_to(pkg)?;
    Ok(())
}

fn next_part(pkg: &Package) -> String {
    let mut n = 1u32;
    loop {
        let name = format!("/xl/externalLinks/externalLink{n}.xml");
        if !pkg.contains(&name) {
            return name;
        }
        n += 1;
    }
}

pub(crate) fn target_from_path(path: &str) -> String {
    let p = path.trim();
    let lower = p.to_ascii_lowercase();
    if lower.starts_with("file:") || lower.starts_with("http:") || lower.starts_with("https:") {
        return p.to_string();
    }
    if p.starts_with("\\\\") {
        return format!("file:///{p}");
    }
    let bytes = p.as_bytes();
    if bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'\\' || bytes[2] == b'/')
    {
        return format!("file:///{p}");
    }
    p.to_string()
}

fn break_link_refs(
    pkg: &mut Package,
    wb: &mut Element,
    wb_part: &str,
    sheet_index: &[(String, String, &'static str)],
    dirty: &mut Vec<(String, Document)>,
    index: u32,
    op_index: usize,
) -> Result<(), Error> {
    let mut changed = false;
    for (_, part, kind) in sheet_index {
        if *kind != "worksheet" {
            continue;
        }
        if break_sheet(pkg, dirty, part, index, op_index)? {
            changed = true;
        }
    }
    if break_defined_names(wb, index, op_index)? {
        changed = true;
    }
    if break_charts(pkg, index, op_index)? {
        changed = true;
    }
    if changed {
        super::cell::mark_full_calc(wb);
        super::cell::remove_calc_chain(pkg, wb_part)?;
    }
    Ok(())
}

fn break_sheet(
    pkg: &mut Package,
    dirty: &mut Vec<(String, Document)>,
    part: &str,
    index: u32,
    op_index: usize,
) -> Result<bool, Error> {
    let rewrite = |root: &mut Element| break_sheet_root(root, index, op_index);
    if let Some(pos) = dirty.iter().position(|(name, _)| name == part) {
        return rewrite(&mut dirty[pos].1.root);
    }
    if !pkg.contains(part) {
        return Ok(false);
    }
    let mut doc = xml::parse(pkg.get(part)?)?;
    let changed = rewrite(&mut doc.root)?;
    if changed {
        super::list_pin::pin_sheet(&mut doc.root);
        let _ = super::ole::pin_objects(pkg, part, &mut doc.root);
        dirty.push((part.to_string(), doc));
    }
    Ok(changed)
}

fn break_sheet_root(root: &mut Element, index: u32, op_index: usize) -> Result<bool, Error> {
    let mut changed = false;
    let mut drop_si = std::collections::HashSet::new();
    crate::xml::walk_elements(root, &mut |el| {
        if el.local_name() != "f" || el.attr("t") != Some("shared") {
            return;
        }
        let text = el.text_content();
        if text.is_empty() {
            return;
        }
        if formula_uses(&text, index, op_index).unwrap_or(false) {
            if let Some(si) = el.attr("si") {
                drop_si.insert(si.to_string());
            }
        }
    });
    changed |= break_cells(root, index, &drop_si, op_index)?;
    changed |= rewrite_named_formulas(root, index, op_index)?;
    Ok(changed)
}

fn break_cells(
    el: &mut Element,
    index: u32,
    drop_si: &std::collections::HashSet<String>,
    op_index: usize,
) -> Result<bool, Error> {
    let mut changed = false;
    if el.local_name() == "c" {
        let drop = cell_should_drop_formula(el, index, drop_si, op_index)?;
        if drop {
            el.children.retain(|node| match node {
                Node::Element(child) => child.local_name() != "f",
                _ => true,
            });
            if el.child_by_local("v").is_none() {
                el.push_element(Element::new("v").with_text("0"));
            }
            changed = true;
        } else if let Some(f) = el.child_by_local_mut("f") {
            let text = f.text_content();
            if !text.is_empty() {
                let next = rewrite_formula(&text, index, op_index)?;
                if next != text {
                    f.set_text(next);
                    changed = true;
                }
            }
        }
    }
    for child in el.element_children_mut() {
        if break_cells(child, index, drop_si, op_index)? {
            changed = true;
        }
    }
    Ok(changed)
}

fn cell_should_drop_formula(
    cell: &Element,
    index: u32,
    drop_si: &std::collections::HashSet<String>,
    op_index: usize,
) -> Result<bool, Error> {
    let Some(f) = cell.child_by_local("f") else {
        return Ok(false);
    };
    if f.attr("t") == Some("shared") {
        if let Some(si) = f.attr("si") {
            if drop_si.contains(si) {
                return Ok(true);
            }
        }
    }
    let text = f.text_content();
    if text.is_empty() {
        return Ok(false);
    }
    formula_uses(&text, index, op_index)
}

fn rewrite_named_formulas(el: &mut Element, index: u32, op_index: usize) -> Result<bool, Error> {
    let mut changed = false;
    if matches!(el.local_name(), "formula" | "formula1" | "formula2")
        && el.element_children().next().is_none()
    {
        let text = el.text_content();
        if !text.is_empty() {
            let next = rewrite_formula(&text, index, op_index)?;
            if next != text {
                el.set_text(next);
                changed = true;
            }
        }
    }
    for child in el.element_children_mut() {
        if rewrite_named_formulas(child, index, op_index)? {
            changed = true;
        }
    }
    Ok(changed)
}

fn break_defined_names(wb: &mut Element, index: u32, op_index: usize) -> Result<bool, Error> {
    let Some(parent) = wb.child_by_local_mut("definedNames") else {
        return Ok(false);
    };
    let mut changed = false;
    for el in parent.element_children_mut() {
        if el.local_name() != "definedName" {
            continue;
        }
        let text = el.text_content();
        let next = rewrite_formula(&text, index, op_index)?;
        if next != text {
            el.set_text(next);
            changed = true;
        }
    }
    Ok(changed)
}

fn break_charts(pkg: &mut Package, index: u32, op_index: usize) -> Result<bool, Error> {
    let mut changed = false;
    for part in pkg.part_names() {
        if !part.contains("/charts/")
            || !part.ends_with(".xml")
            || part.contains("/_rels/")
            || part.contains("colors")
            || part.contains("style")
        {
            continue;
        }
        let mut doc = xml::parse(pkg.get(&part)?)?;
        if break_chart_tree(&mut doc.root, index, op_index)? {
            pkg.set(&part, doc.to_bytes()?)?;
            changed = true;
        }
    }
    Ok(changed)
}

fn break_chart_tree(el: &mut Element, index: u32, op_index: usize) -> Result<bool, Error> {
    let mut changed = false;
    let mut kept = Vec::new();
    for node in el.take_children() {
        match node {
            Node::Element(mut child) if child.local_name() == "f" => {
                let text = child.text_content();
                if formula_uses(&text, index, op_index)? {
                    changed = true;
                    continue;
                }
                if !text.is_empty() {
                    let next = rewrite_formula(&text, index, op_index)?;
                    if next != text {
                        child.set_text(next);
                        changed = true;
                    }
                }
                kept.push(Node::Element(child));
            }
            Node::Element(mut child) => {
                if break_chart_tree(&mut child, index, op_index)? {
                    changed = true;
                }
                kept.push(Node::Element(child));
            }
            other => kept.push(other),
        }
    }
    el.children = kept;
    Ok(changed)
}

fn formula_uses(text: &str, index: u32, op_index: usize) -> Result<bool, Error> {
    super::formula::formula_uses_external_index(text, index).map_err(|_| bad_formula(op_index))
}

fn rewrite_formula(text: &str, index: u32, op_index: usize) -> Result<String, Error> {
    super::formula::rewrite_external_link_formula(text, index).map_err(|_| bad_formula(op_index))
}

fn bad_formula(op_index: usize) -> Error {
    Error::with_path(ErrorKind::UnreadableFormula, format!("ops[{op_index}].id"))
}

fn not_found(id: &str, op_index: usize) -> Error {
    Error::with_path(
        ErrorKind::ExternalLinkNotFound { id: id.to_string() },
        format!("ops[{op_index}].id"),
    )
}
