//! 书目源：bibliography customXml 的 inspect 与插删。
//! 不按 APA / MLA 等样式表排版。CITATION / BIBLIOGRAPHY 域在 `field`。

use std::collections::HashMap;

use serde_json::{json, Value};

use crate::error::{Error, ErrorKind};
use crate::opc::{self, ContentTypes, Package, Relationship, Relationships};
use crate::xml::{self, Document, Element, Node};

use super::picture;

pub const REL_CUSTOM_XML: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/customXml";
pub const REL_CUSTOM_XML_PROPS: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/customXmlProps";
pub const CT_CUSTOM_XML_PROPS: &str =
    "application/vnd.openxmlformats-officedocument.customXmlProperties+xml";
pub const BIBLIO_NS: &str = "http://schemas.openxmlformats.org/officeDocument/2006/bibliography";
const DS_NS: &str = "http://schemas.openxmlformats.org/officeDocument/2006/customXml";

const SOURCE_TYPES: &[&str] = &[
    "Book",
    "BookSection",
    "JournalArticle",
    "ArticleInAPeriodical",
    "ConferenceProceedings",
    "Report",
    "InternetSite",
    "DocumentFromInternetSite",
    "ElectronicSource",
    "Art",
    "SoundRecording",
    "Performance",
    "Film",
    "Interview",
    "Patent",
    "Case",
    "Misc",
];

pub const DEFAULT_SOURCE_TYPE: &str = "Book";

const BIBLIO_STYLES: &[(&str, &str, &str)] = &[
    ("APA", r"\APASixthEditionOfficeOnline.xsl", "6"),
    ("MLA", r"\MLASeventhEditionOfficeOnline.xsl", "7"),
    ("Chicago", r"\CHICAGO.XSL", "15"),
    ("IEEE", r"\IEEE2006OfficeOnline.xsl", "2006"),
    ("GB7714", r"\GB.XSL", "2005"),
];

pub fn style_ok(s: &str) -> bool {
    BIBLIO_STYLES.iter().any(|(n, _, _)| *n == s)
}

pub fn style_attrs(name: &str) -> Option<(&'static str, &'static str, &'static str)> {
    BIBLIO_STYLES
        .iter()
        .find(|(n, _, _)| *n == name)
        .map(|(n, sel, ver)| (*n, *sel, *ver))
}

pub struct SourceWrite<'a> {
    pub tag: &'a str,
    pub source_type: &'a str,
    pub title: &'a str,
    pub year: Option<&'a str>,
    pub last: Option<&'a str>,
    pub first: Option<&'a str>,
    pub city: Option<&'a str>,
    pub publisher: Option<&'a str>,
}

/// tag 空、只含空白、含空白或英文双引号时非法。
pub fn tag_invalid(s: &str) -> bool {
    s.is_empty() || s.chars().any(|c| c.is_whitespace() || c == '"')
}

pub fn source_type_ok(s: &str) -> bool {
    SOURCE_TYPES.contains(&s)
}

pub fn count_sources(pkg: &Package, doc_part: &str) -> Result<usize, Error> {
    let Some(part) = find_biblio_part(pkg, doc_part)? else {
        return Ok(0);
    };
    let doc = xml::parse(pkg.get(&part)?)?;
    Ok(source_elements(&doc.root).len())
}

pub fn sources_json(pkg: &Package, doc_part: &str) -> Result<Vec<Value>, Error> {
    let Some(part) = find_biblio_part(pkg, doc_part)? else {
        return Ok(Vec::new());
    };
    let doc = xml::parse(pkg.get(&part)?)?;
    let mut out = Vec::new();
    for (i, src) in source_elements(&doc.root).into_iter().enumerate() {
        let n = i + 1;
        out.push(json!({
            "id": format!("source:{n}"),
            "index": n,
            "tag": child_text(src, "Tag"),
            "sourceType": child_text(src, "SourceType"),
            "title": child_text(src, "Title"),
            "year": child_text(src, "Year"),
            "author": first_person_author(src),
        }));
    }
    Ok(out)
}

pub fn style_json(pkg: &Package, doc_part: &str) -> Result<Value, Error> {
    let Some(part) = find_biblio_part(pkg, doc_part)? else {
        return Ok(Value::Null);
    };
    let doc = xml::parse(pkg.get(&part)?)?;
    Ok(json!({
        "selectedStyle": attr_or_null(&doc.root, "SelectedStyle"),
        "styleName": attr_or_null(&doc.root, "StyleName"),
        "version": attr_or_null(&doc.root, "Version"),
    }))
}

fn attr_or_null(el: &Element, local: &str) -> Value {
    el.attr_local(local)
        .map(|s| json!(s))
        .unwrap_or(Value::Null)
}

pub fn set_style(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    style_name: &str,
    op_i: usize,
) -> Result<(), Error> {
    let Some((_, selected, version)) = style_attrs(style_name) else {
        return Err(Error::with_path(
            ErrorKind::InvalidBibliographyStyle,
            format!("ops[{op_i}].styleName"),
        ));
    };
    let part = ensure_biblio_part(pkg, docs, dirty, doc_part)?;
    let doc = docs.get_mut(&part).expect("ensure 已载入书目部件");
    set_local_attr(&mut doc.root, "SelectedStyle", selected);
    set_local_attr(&mut doc.root, "StyleName", style_name);
    set_local_attr(&mut doc.root, "Version", version);
    dirty.insert(part, ());
    Ok(())
}

fn set_local_attr(el: &mut Element, local: &str, val: &str) {
    if let Some((k, v)) = el
        .attrs
        .iter_mut()
        .find(|(k, _)| k.rsplit(':').next() == Some(local))
    {
        let _ = k;
        *v = val.to_string();
        return;
    }
    el.set_attr(local, val);
}

pub fn has_tag(
    pkg: &Package,
    docs: &HashMap<String, Document>,
    doc_part: &str,
    tag: &str,
) -> Result<bool, Error> {
    Ok(source_tags(pkg, docs, doc_part)?.iter().any(|t| t == tag))
}

pub fn insert_source(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    write: &SourceWrite<'_>,
    op_i: usize,
) -> Result<(), Error> {
    let part = ensure_biblio_part(pkg, docs, dirty, doc_part)?;
    let tags = source_tags(pkg, docs, doc_part)?;
    if tags.iter().any(|t| t == write.tag) {
        return Err(Error::with_path(
            ErrorKind::DuplicateSourceTag,
            format!("ops[{op_i}].tag"),
        ));
    }
    let doc = docs.get_mut(&part).expect("ensure 已载入书目部件");
    let order = source_elements(&doc.root).len() + 1;
    append_source(&mut doc.root, write, order);
    dirty.insert(part, ());
    Ok(())
}

pub fn delete_source(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    n: usize,
    err_id: &str,
    op_i: usize,
) -> Result<(), Error> {
    let Some(part) = find_biblio_part(pkg, doc_part)? else {
        return Err(missing_source(err_id, op_i));
    };
    if !docs.contains_key(&part) {
        let parsed = xml::parse(pkg.get(&part)?)?;
        docs.insert(part.clone(), parsed);
    }
    let doc = docs.get_mut(&part).expect("已载入书目部件");
    let idx = match source_child_index(&doc.root, n) {
        Some(i) => i,
        None => return Err(missing_source(err_id, op_i)),
    };
    doc.root.children.remove(idx);
    rewrite_ref_order(&mut doc.root);
    dirty.insert(part, ());
    Ok(())
}

fn missing_source(id: &str, op_i: usize) -> Error {
    Error::with_path(
        ErrorKind::SourceNotFound { id: id.to_string() },
        format!("ops[{op_i}].id"),
    )
}

fn find_biblio_part(pkg: &Package, doc_part: &str) -> Result<Option<String>, Error> {
    let rels = Relationships::from_package(pkg, doc_part)?;
    for r in &rels.rels {
        if r.rel_type != REL_CUSTOM_XML {
            continue;
        }
        let item = match opc::resolve_rel_target(doc_part, &r.target) {
            Ok(p) => p,
            Err(_) => continue,
        };
        if !pkg.contains(&item) {
            continue;
        }
        let item_rels = Relationships::from_package(pkg, &item)?;
        let Some(pr) = item_rels
            .rels
            .iter()
            .find(|x| x.rel_type == REL_CUSTOM_XML_PROPS)
        else {
            continue;
        };
        let props = match opc::resolve_rel_target(&item, &pr.target) {
            Ok(p) => p,
            Err(_) => continue,
        };
        if !pkg.contains(&props) {
            continue;
        }
        let parsed = xml::parse(pkg.get(&props)?)?;
        if is_biblio_props(&parsed.root) {
            return Ok(Some(item));
        }
    }
    Ok(None)
}

fn is_biblio_props(root: &Element) -> bool {
    let Some(refs) = root.child_by_local("schemaRefs") else {
        return false;
    };
    refs.element_children()
        .any(|r| r.local_name() == "schemaRef" && r.attr_local("uri") == Some(BIBLIO_NS))
}

fn ensure_biblio_part(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
) -> Result<String, Error> {
    if let Some(existing) = find_biblio_part(pkg, doc_part)? {
        if !docs.contains_key(&existing) {
            let parsed = xml::parse(pkg.get(&existing)?)?;
            docs.insert(existing.clone(), parsed);
        }
        return Ok(existing);
    }
    let n = next_item_n(pkg);
    let item = format!("/customXml/item{n}.xml");
    let props = format!("/customXml/itemProps{n}.xml");
    pkg.set(&item, empty_sources_xml().into_bytes())?;
    pkg.set(&props, item_props_xml(&new_guid()).into_bytes())?;
    let parsed = xml::parse(pkg.get(&item)?)?;
    docs.insert(item.clone(), parsed);
    dirty.insert(item.clone(), ());

    let mut item_rels = Relationships { rels: Vec::new() };
    item_rels.push(Relationship {
        id: item_rels.next_rid(),
        rel_type: REL_CUSTOM_XML_PROPS.into(),
        target: picture::rel_target_from(&item, &props),
        target_mode: None,
        extra: Vec::new(),
    });
    item_rels.write_to(pkg, &item)?;

    let mut doc_rels = Relationships::from_package(pkg, doc_part)?;
    doc_rels.push(Relationship {
        id: doc_rels.next_rid(),
        rel_type: REL_CUSTOM_XML.into(),
        target: picture::rel_target_from(doc_part, &item),
        target_mode: None,
        extra: Vec::new(),
    });
    doc_rels.write_to(pkg, doc_part)?;

    let mut cts = ContentTypes::from_package(pkg)?;
    cts.set_override(&props, CT_CUSTOM_XML_PROPS)?;
    cts.write_to(pkg)?;
    Ok(item)
}

fn next_item_n(pkg: &Package) -> u32 {
    let mut max = 0u32;
    for name in pkg.part_names() {
        let Some(file) = name.strip_prefix("/customXml/item") else {
            continue;
        };
        let Some(n) = file.strip_suffix(".xml") else {
            continue;
        };
        if n.chars().all(|c| c.is_ascii_digit()) {
            if let Ok(v) = n.parse::<u32>() {
                max = max.max(v);
            }
        }
    }
    max + 1
}

fn empty_sources_xml() -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="no"?><b:Sources SelectedStyle="\APASixthEditionOfficeOnline.xsl" StyleName="APA" Version="6" xmlns:b="{BIBLIO_NS}"></b:Sources>"#
    )
}

fn item_props_xml(guid: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="no"?><ds:datastoreItem ds:itemID="{guid}" xmlns:ds="{DS_NS}"><ds:schemaRefs><ds:schemaRef ds:uri="{BIBLIO_NS}"/></ds:schemaRefs></ds:datastoreItem>"#
    )
}

fn new_guid() -> String {
    let mut b = [0u8; 16];
    getrandom::getrandom(&mut b).expect("系统随机数不可用");
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    format!(
        "{{{:02X}{:02X}{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}}}",
        b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7], b[8], b[9], b[10], b[11], b[12], b[13],
        b[14], b[15]
    )
}

fn source_tags(
    pkg: &Package,
    docs: &HashMap<String, Document>,
    doc_part: &str,
) -> Result<Vec<String>, Error> {
    let Some(part) = find_biblio_part(pkg, doc_part)? else {
        return Ok(Vec::new());
    };
    if let Some(doc) = docs.get(&part) {
        return Ok(source_elements(&doc.root)
            .into_iter()
            .map(|s| child_text(s, "Tag"))
            .collect());
    }
    let doc = xml::parse(pkg.get(&part)?)?;
    Ok(source_elements(&doc.root)
        .into_iter()
        .map(|s| child_text(s, "Tag"))
        .collect())
}

fn source_elements(root: &Element) -> Vec<&Element> {
    root.element_children()
        .filter(|e| e.local_name() == "Source")
        .collect()
}

fn source_child_index(root: &Element, n: usize) -> Option<usize> {
    if n < 1 {
        return None;
    }
    let mut seen = 0usize;
    for (i, child) in root.children.iter().enumerate() {
        if let Node::Element(e) = child {
            if e.local_name() == "Source" {
                seen += 1;
                if seen == n {
                    return Some(i);
                }
            }
        }
    }
    None
}

fn rewrite_ref_order(root: &mut Element) {
    let mut n = 0u32;
    for child in root.element_children_mut() {
        if child.local_name() != "Source" {
            continue;
        }
        n += 1;
        if let Some(order) = child.child_by_local_mut("RefOrder") {
            order.set_text(n.to_string());
        } else {
            push_text_child(child, "RefOrder", &n.to_string());
        }
    }
}

fn append_source(root: &mut Element, write: &SourceWrite<'_>, order: usize) {
    let mut src = Element::new(root.prefixed("Source"));
    push_text_child(&mut src, "Tag", write.tag);
    push_text_child(&mut src, "SourceType", write.source_type);
    push_text_child(&mut src, "Guid", &new_guid());
    push_author(&mut src, write.last, write.first);
    push_text_child(&mut src, "Title", write.title);
    if let Some(y) = write.year {
        push_text_child(&mut src, "Year", y);
    }
    if let Some(c) = write.city {
        push_text_child(&mut src, "City", c);
    }
    if let Some(p) = write.publisher {
        push_text_child(&mut src, "Publisher", p);
    }
    push_text_child(&mut src, "RefOrder", &order.to_string());
    root.push_element(src);
}

fn push_author(source: &mut Element, last: Option<&str>, first: Option<&str>) {
    if last.is_none() && first.is_none() {
        return;
    }
    let mut author = Element::new(source.prefixed("Author"));
    let mut inner = Element::new(author.prefixed("Author"));
    let mut list = Element::new(inner.prefixed("NameList"));
    let mut person = Element::new(list.prefixed("Person"));
    if let Some(s) = last {
        push_text_child(&mut person, "Last", s);
    }
    if let Some(s) = first {
        push_text_child(&mut person, "First", s);
    }
    list.push_element(person);
    inner.push_element(list);
    author.push_element(inner);
    source.push_element(author);
}

fn push_text_child(parent: &mut Element, local: &str, text: &str) {
    let mut e = Element::new(parent.prefixed(local));
    e.set_text(text.to_string());
    parent.push_element(e);
}

fn child_text(el: &Element, local: &str) -> String {
    el.child_by_local(local)
        .map(|c| c.text_content())
        .unwrap_or_default()
}

fn first_person_author(src: &Element) -> String {
    match first_person(src) {
        Some(p) => {
            let last = child_text(p, "Last");
            let first = child_text(p, "First");
            match (last.is_empty(), first.is_empty()) {
                (true, true) => String::new(),
                (false, true) => last,
                (true, false) => first,
                (false, false) => format!("{last}, {first}"),
            }
        }
        None => String::new(),
    }
}

fn first_person(el: &Element) -> Option<&Element> {
    if el.local_name() == "Person" {
        return Some(el);
    }
    for c in el.element_children() {
        if let Some(p) = first_person(c) {
            return Some(p);
        }
    }
    None
}
