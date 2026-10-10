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
    dirty_sheets: &[(String, Document)],
    op: &DeleteExternalLinkOp,
    op_index: usize,
) -> Result<(), Error> {
    let n = count(&wb.root);
    if op.index == 0 || op.index > n {
        return Err(not_found(&op.id, op_index));
    }
    if in_use(pkg, &wb.root, sheet_index, dirty_sheets, op.index as u32)? {
        return Err(Error::with_path(
            ErrorKind::ExternalLinkInUse,
            format!("ops[{op_index}].id"),
        ));
    }
    let parent = wb
        .root
        .child_by_local("externalReferences")
        .ok_or_else(|| not_found(&op.id, op_index))?;
    let el = parent
        .children_by_local("externalReference")
        .into_iter()
        .nth(op.index - 1)
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
    let n = count(&wb.root);
    if op.index == 0 || op.index > n {
        return Err(not_found(&op.id, op_index));
    }
    let parent = wb
        .root
        .child_by_local("externalReferences")
        .ok_or_else(|| not_found(&op.id, op_index))?;
    let el = parent
        .children_by_local("externalReference")
        .into_iter()
        .nth(op.index - 1)
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
        Document {
            decl: Some(DEFAULT_DECL.to_string()),
            root,
        }
        .to_bytes()?,
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
        let mut seen = 0usize;
        parent.children.retain(|n| match n {
            Node::Element(e) if e.local_name() == "externalReference" => {
                seen += 1;
                seen != index
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

fn in_use(
    pkg: &Package,
    wb: &Element,
    sheet_index: &[(String, String, &'static str)],
    dirty_sheets: &[(String, Document)],
    min: u32,
) -> Result<bool, Error> {
    if let Some(parent) = wb.child_by_local("definedNames") {
        for el in parent.children_by_local("definedName") {
            if text_has_ext_index_at_least(&el.text_content(), min) {
                return Ok(true);
            }
        }
    }
    for (_, part, _) in sheet_index {
        if let Some((_, doc)) = dirty_sheets.iter().find(|(p, _)| p == part) {
            if formulas_use(&doc.root, min) {
                return Ok(true);
            }
            continue;
        }
        let bytes = pkg.get(part)?;
        let doc = xml::parse(bytes)?;
        if formulas_use(&doc.root, min) {
            return Ok(true);
        }
    }
    Ok(false)
}

fn formulas_use(root: &Element, min: u32) -> bool {
    let mut found = false;
    walk_formulas(root, &mut |text| {
        if text_has_ext_index_at_least(text, min) {
            found = true;
        }
    });
    found
}

fn walk_formulas(el: &Element, visit: &mut impl FnMut(&str)) {
    if el.local_name() == "f" {
        visit(&el.text_content());
    }
    for c in el.element_children() {
        walk_formulas(c, visit);
    }
}

/// `[10]` 解析成 10，不会当成 `[1]` 的前缀。
pub(crate) fn text_has_ext_index_at_least(text: &str, min: u32) -> bool {
    let b = text.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'[' {
            let start = i + 1;
            let mut j = start;
            while j < b.len() && b[j].is_ascii_digit() {
                j += 1;
            }
            if j > start && j < b.len() && b[j] == b']' {
                if let Ok(n) = text[start..j].parse::<u32>() {
                    if n >= min && n >= 1 {
                        return true;
                    }
                }
                i = j + 1;
                continue;
            }
        }
        i += 1;
    }
    false
}

fn not_found(id: &str, op_index: usize) -> Error {
    Error::with_path(
        ErrorKind::ExternalLinkNotFound { id: id.to_string() },
        format!("ops[{op_index}].id"),
    )
}
