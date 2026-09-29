//! 工作簿外部数据连接：inspect 名单，apply 写 `/xl/connections.xml`。
//! 不向数据源发请求，不把查询结果写进单元格。

use serde_json::{json, Map, Value};

use crate::error::{Error, ErrorKind, IdReason};
use crate::opc::{resolve_rel_target, ContentTypes, Package, Relationship, Relationships};
use crate::xml::{self, Document, Element, Node, DEFAULT_DECL};

use super::picture;
use super::SML_NS;

const REL_CONNECTIONS: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/connections";
const CT_CONNECTIONS: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.connections+xml";
const DEFAULT_PART: &str = "/xl/connections.xml";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionKind {
    Web,
    Text,
}

#[derive(Debug, Clone)]
pub enum ConnectionKey {
    Id(String),
    Index(usize),
}

#[derive(Debug, Clone)]
pub struct InsertConnectionOp {
    pub name: String,
    pub kind: ConnectionKind,
    pub url: Option<String>,
    pub source_file: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DeleteConnectionOp {
    pub id: String,
    pub key: ConnectionKey,
}

pub fn parse_connection_id(id: &str, json_path: &str) -> Result<ConnectionKey, Error> {
    let rest = id.strip_prefix("connection:").ok_or_else(|| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::ConnectionPrefix,
            },
            json_path.to_string(),
        )
    })?;
    if rest.is_empty() {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::ConnectionPrefix,
            },
            json_path.to_string(),
        ));
    }
    if let Some(idx) = rest.strip_prefix("index:") {
        let k: usize = idx.parse().map_err(|_| {
            Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::ConnectionIndexNotInt,
                },
                json_path.to_string(),
            )
        })?;
        if k < 1 {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::ConnectionIndexFromOne,
                },
                json_path.to_string(),
            ));
        }
        return Ok(ConnectionKey::Index(k));
    }
    Ok(ConnectionKey::Id(rest.to_string()))
}

pub(crate) fn count(pkg: &Package, wb_part: &str) -> Result<usize, Error> {
    Ok(load_included(pkg, wb_part)?.len())
}

pub(crate) fn inspect(pkg: &Package, wb_part: &str) -> Result<Value, Error> {
    let items = load_included(pkg, wb_part)?;
    let mut out = Vec::with_capacity(items.len());
    for (k, el) in items {
        out.push(inspect_one(&el, k));
    }
    Ok(Value::Array(out))
}

fn inspect_one(el: &Element, k: usize) -> Value {
    let xml_id = el.attr("id").map(str::to_string);
    let stable = xml_id.is_some();
    let id = match xml_id.as_deref() {
        Some(raw) => format!("connection:{raw}"),
        None => format!("connection:index:{k}"),
    };
    let ty = el.attr("type").unwrap_or("");
    let kind = match ty {
        "4" => "web",
        "6" => "text",
        _ => "other",
    };
    let url = el
        .child_by_local("webPr")
        .and_then(|p| p.attr("url"))
        .map(str::to_string);
    let source_file = el
        .child_by_local("textPr")
        .and_then(|p| p.attr("sourceFile"))
        .map(str::to_string);
    let mut obj = Map::new();
    obj.insert("id".into(), json!(id));
    obj.insert("xmlId".into(), json!(xml_id));
    obj.insert("name".into(), json!(el.attr("name").unwrap_or("")));
    obj.insert("kind".into(), json!(kind));
    obj.insert("url".into(), json!(url));
    obj.insert("sourceFile".into(), json!(source_file));
    obj.insert(
        "description".into(),
        json!(el.attr("description").map(str::to_string)),
    );
    obj.insert("stable".into(), json!(stable));
    Value::Object(obj)
}

pub(crate) fn insert(
    pkg: &mut Package,
    wb_part: &str,
    op: &InsertConnectionOp,
    op_index: usize,
) -> Result<(), Error> {
    let part = ensure_part(pkg, wb_part)?;
    let mut doc = xml::parse(pkg.get(&part)?)?;
    if name_taken(&doc.root, &op.name) {
        return Err(Error::with_path(
            ErrorKind::InvalidConnection {
                field: "name".into(),
            },
            format!("ops[{op_index}].name"),
        ));
    }
    let id = next_xml_id(&doc.root);
    let conn = build_connection(id, op);
    doc.root.push_element(conn);
    pkg.set(&part, doc.to_bytes()?)?;
    Ok(())
}

pub(crate) fn delete(
    pkg: &mut Package,
    wb_part: &str,
    op: &DeleteConnectionOp,
    op_index: usize,
) -> Result<(), Error> {
    let Some(part) = find_part(pkg, wb_part)? else {
        return Err(not_found(&op.id, op_index));
    };
    let mut doc = xml::parse(pkg.get(&part)?)?;
    let pos = find_included_index(&doc.root, &op.key).ok_or_else(|| not_found(&op.id, op_index))?;
    let mut seen = 0usize;
    doc.root.children.retain(|n| match n {
        Node::Element(e) if e.local_name() == "connection" && !is_deleted(e) => {
            seen += 1;
            seen != pos
        }
        _ => true,
    });
    let remain = doc
        .root
        .children_by_local("connection")
        .into_iter()
        .filter(|e| !is_deleted(e))
        .count();
    if remain == 0 && doc.root.children_by_local("connection").is_empty() {
        drop_part(pkg, wb_part, &part)?;
        return Ok(());
    }
    pkg.set(&part, doc.to_bytes()?)?;
    Ok(())
}

fn load_included(pkg: &Package, wb_part: &str) -> Result<Vec<(usize, Element)>, Error> {
    let Some(part) = find_part(pkg, wb_part)? else {
        return Ok(Vec::new());
    };
    let doc = xml::parse(pkg.get(&part)?)?;
    Ok(doc
        .root
        .children_by_local("connection")
        .into_iter()
        .filter(|e| !is_deleted(e))
        .enumerate()
        .map(|(i, e)| (i + 1, e.clone()))
        .collect())
}

fn find_part(pkg: &Package, wb_part: &str) -> Result<Option<String>, Error> {
    let rels = Relationships::from_package(pkg, wb_part)?;
    for rel in &rels.rels {
        if rel.rel_type == REL_CONNECTIONS || rel.rel_type.ends_with("/connections") {
            let part = resolve_rel_target(wb_part, &rel.target)?;
            if !pkg.contains(&part) {
                return Err(Error::new(ErrorKind::PartMissing { name: part }));
            }
            return Ok(Some(part));
        }
    }
    if pkg.contains(DEFAULT_PART) {
        return Ok(Some(DEFAULT_PART.to_string()));
    }
    Ok(None)
}

fn ensure_part(pkg: &mut Package, wb_part: &str) -> Result<String, Error> {
    if let Some(part) = find_part(pkg, wb_part)? {
        ensure_rel_and_ct(pkg, wb_part, &part)?;
        return Ok(part);
    }
    let root = Element::new("connections").with_attr("xmlns", SML_NS);
    pkg.set(
        DEFAULT_PART,
        Document {
            decl: Some(DEFAULT_DECL.to_string()),
            root,
        }
        .to_bytes()?,
    )?;
    ensure_rel_and_ct(pkg, wb_part, DEFAULT_PART)?;
    Ok(DEFAULT_PART.to_string())
}

fn ensure_rel_and_ct(pkg: &mut Package, wb_part: &str, part: &str) -> Result<(), Error> {
    let mut rels = Relationships::from_package(pkg, wb_part)?;
    let has_rel = rels.rels.iter().any(|r| {
        if r.rel_type != REL_CONNECTIONS && !r.rel_type.ends_with("/connections") {
            return false;
        }
        resolve_rel_target(wb_part, &r.target)
            .ok()
            .is_some_and(|p| p == part)
    });
    if !has_rel {
        let rid = rels.next_rid();
        rels.push(Relationship {
            id: rid,
            rel_type: REL_CONNECTIONS.into(),
            target: picture::rel_target_from(wb_part, part),
            target_mode: None,
            extra: Vec::new(),
        });
        rels.write_to(pkg, wb_part)?;
    }
    let mut cts = ContentTypes::from_package(pkg)?;
    cts.set_override(part, CT_CONNECTIONS)?;
    cts.write_to(pkg)?;
    Ok(())
}

fn drop_part(pkg: &mut Package, wb_part: &str, part: &str) -> Result<(), Error> {
    pkg.remove(part);
    let mut rels = Relationships::from_package(pkg, wb_part)?;
    rels.remove_by_target_part(wb_part, part);
    rels.write_to(pkg, wb_part)?;
    let mut cts = ContentTypes::from_package(pkg)?;
    cts.remove_override(part);
    cts.write_to(pkg)?;
    Ok(())
}

fn next_xml_id(root: &Element) -> u32 {
    let mut max = 0u32;
    for el in root.children_by_local("connection") {
        if let Some(Ok(n)) = el.attr("id").map(str::parse::<u32>) {
            max = max.max(n);
        }
    }
    max.saturating_add(1).max(1)
}

fn name_taken(root: &Element, name: &str) -> bool {
    root.children_by_local("connection")
        .into_iter()
        .filter(|e| !is_deleted(e))
        .any(|e| e.attr("name").is_some_and(|n| n.eq_ignore_ascii_case(name)))
}

fn build_connection(id: u32, op: &InsertConnectionOp) -> Element {
    let mut el = Element::new("connection")
        .with_attr("id", id.to_string())
        .with_attr("name", &op.name);
    if let Some(desc) = op.description.as_deref().filter(|s| !s.is_empty()) {
        el.set_attr("description", desc);
    }
    match op.kind {
        ConnectionKind::Web => {
            el.set_attr("type", "4");
            el.set_attr("refreshedVersion", "0");
            el.set_attr("background", "1");
            let url = op.url.as_deref().unwrap_or("");
            el.push_element(
                Element::new("webPr")
                    .with_attr("url", url)
                    .with_attr("htmlTables", "1")
                    .with_attr("htmlFormat", "all"),
            );
        }
        ConnectionKind::Text => {
            el.set_attr("type", "6");
            el.set_attr("refreshedVersion", "8");
            el.set_attr("background", "1");
            el.set_attr("saveData", "1");
            let src = op.source_file.as_deref().unwrap_or("");
            let mut text_pr = Element::new("textPr")
                .with_attr("prompt", "0")
                .with_attr("codePage", "65001")
                .with_attr("sourceFile", src)
                .with_attr("comma", "1");
            let mut fields = Element::new("textFields");
            fields.push_element(Element::new("textField"));
            text_pr.push_element(fields);
            el.push_element(text_pr);
        }
    }
    el
}

fn find_included_index(root: &Element, key: &ConnectionKey) -> Option<usize> {
    let items: Vec<&Element> = root
        .children_by_local("connection")
        .into_iter()
        .filter(|e| !is_deleted(e))
        .collect();
    match key {
        ConnectionKey::Id(xml_id) => items
            .iter()
            .position(|e| e.attr("id") == Some(xml_id.as_str()))
            .map(|i| i + 1),
        ConnectionKey::Index(k) => {
            if *k == 0 || *k > items.len() {
                return None;
            }
            let el = items[*k - 1];
            if el.attr("id").is_some() {
                return None;
            }
            Some(*k)
        }
    }
}

fn is_deleted(el: &Element) -> bool {
    matches!(
        el.attr("deleted")
            .map(|s| s.to_ascii_lowercase())
            .as_deref(),
        Some("1") | Some("true") | Some("on")
    )
}

fn not_found(id: &str, op_index: usize) -> Error {
    Error::with_path(
        ErrorKind::ConnectionNotFound { id: id.to_string() },
        format!("ops[{op_index}].id"),
    )
}
