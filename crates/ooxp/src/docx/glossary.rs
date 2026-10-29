//! glossary 构建基块：inspect 与插删。不实现 Word 库面板，不改 Normal.dotm。

use std::collections::{HashMap, HashSet};

use serde_json::{json, Value};

use crate::error::{Error, ErrorKind, IdReason};
use crate::opc::{self, rels_part_name, ContentTypes, Package, Relationship, Relationships};
use crate::xml::{self, Document, Element, Node};

use super::para::{self, BodyPara};
use super::picture;
use super::{
    CT_FONT_TABLE, CT_GLOSSARY, CT_SETTINGS, CT_STYLES, CT_WEB_SETTINGS, MC_NS, REL_FONT_TABLE,
    REL_GLOSSARY, REL_NS, REL_SETTINGS, REL_STYLES, REL_WEB_SETTINGS, W14_NS, WML_NS,
};

pub const DEFAULT_GALLERY: &str = "custQuickParts";
pub const DEFAULT_CATEGORY: &str = "General";
pub const DEFAULT_BEHAVIOR: &str = "content";

const GALLERIES: &[&str] = &[
    "placeholder",
    "any",
    "default",
    "docParts",
    "coverPg",
    "eq",
    "ftrs",
    "hdrs",
    "pgNum",
    "tbls",
    "watermarks",
    "autoTxt",
    "txtBox",
    "pgNumT",
    "pgNumB",
    "pgNumMargins",
    "tblOfContents",
    "bib",
    "custQuickParts",
    "custCoverPg",
    "custEq",
    "custFtrs",
    "custHdrs",
    "custPgNum",
    "custTbls",
    "custWatermarks",
    "custAutoTxt",
    "custTxtBox",
    "custPgNumT",
    "custPgNumB",
    "custPgNumMargins",
    "custTblOfContents",
    "custBib",
    "custom1",
    "custom2",
    "custom3",
    "custom4",
    "custom5",
];

const BEHAVIORS: &[&str] = &["content", "p", "pg"];

const GLOSSARY_PART: &str = "/word/glossary/document.xml";

pub struct DocPartWrite<'a> {
    pub name: &'a str,
    pub gallery: &'a str,
    pub category: &'a str,
    pub behavior: &'a str,
    pub text: &'a str,
}

pub fn gallery_ok(s: &str) -> bool {
    GALLERIES.contains(&s)
}

pub fn behavior_ok(s: &str) -> bool {
    BEHAVIORS.contains(&s)
}

pub fn count_parts(pkg: &Package, doc_part: &str) -> Result<usize, Error> {
    let Some(part) = find_glossary_part(pkg, doc_part)? else {
        return Ok(0);
    };
    let doc = xml::parse(pkg.get(&part)?)?;
    Ok(doc_part_elements(&doc.root).len())
}

pub fn parts_json(pkg: &Package, doc_part: &str) -> Result<Vec<Value>, Error> {
    let Some(part) = find_glossary_part(pkg, doc_part)? else {
        return Ok(Vec::new());
    };
    let doc = xml::parse(pkg.get(&part)?)?;
    let mut out = Vec::new();
    for (i, el) in doc_part_elements(&doc.root).into_iter().enumerate() {
        let n = i + 1;
        let pr = el.child_by_local("docPartPr");
        let name = pr
            .and_then(|p| p.child_by_local("name"))
            .and_then(|e| e.attr_local("val"))
            .unwrap_or("")
            .to_string();
        let cat = pr.and_then(|p| p.child_by_local("category"));
        let gallery = cat
            .and_then(|c| c.child_by_local("gallery"))
            .and_then(|g| g.attr_local("val"))
            .map(str::to_string);
        let category = cat
            .and_then(|c| c.child_by_local("name"))
            .and_then(|g| g.attr_local("val"))
            .map(str::to_string);
        let typ = pr
            .and_then(|p| p.child_by_local("types"))
            .and_then(|t| t.child_by_local("type"))
            .and_then(|t| t.attr_local("val"))
            .map(str::to_string);
        let behavior = pr
            .and_then(|p| p.child_by_local("behaviors"))
            .and_then(|b| b.child_by_local("behavior"))
            .and_then(|b| b.attr_local("val"))
            .map(str::to_string);
        let guid = pr
            .and_then(|p| p.child_by_local("guid"))
            .and_then(|g| g.attr_local("val"))
            .map(str::to_string);
        let text = pr
            .map(|_| collect_t(el.child_by_local("docPartBody").unwrap_or(el)))
            .unwrap_or_else(|| collect_t(el));
        out.push(json!({
            "id": format!("docPart:{n}"),
            "index": n,
            "name": name,
            "gallery": gallery,
            "category": category,
            "type": typ,
            "behavior": behavior,
            "guid": guid,
            "text": text,
        }));
    }
    Ok(out)
}

pub fn insert_part(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    write: &DocPartWrite<'_>,
    op_i: usize,
) -> Result<(), Error> {
    let part = ensure_glossary_part(pkg, docs, dirty, doc_part)?;
    let existing = identities(pkg, docs, doc_part)?;
    let key = (
        write.name.to_string(),
        write.gallery.to_string(),
        write.category.to_string(),
    );
    if existing.iter().any(|k| k == &key) {
        return Err(Error::with_path(
            ErrorKind::DuplicateDocPartName,
            format!("ops[{op_i}].name"),
        ));
    }
    let doc = docs.get_mut(&part).expect("ensure 已载入 glossary");
    append_doc_part(&mut doc.root, write);
    dirty.insert(part, ());
    Ok(())
}

pub fn delete_part(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    removed: &mut HashSet<String>,
    doc_part: &str,
    n: usize,
    err_id: &str,
    op_i: usize,
) -> Result<(), Error> {
    let Some(part) = find_glossary_part(pkg, doc_part)? else {
        return Err(missing_part(err_id, op_i));
    };
    if !docs.contains_key(&part) {
        let parsed = xml::parse(pkg.get(&part)?)?;
        docs.insert(part.clone(), parsed);
    }
    let total = {
        let doc = docs.get(&part).expect("已载入 glossary");
        doc_part_elements(&doc.root).len()
    };
    if n < 1 || n > total {
        return Err(missing_part(err_id, op_i));
    }
    if total == 1 {
        drop_glossary(pkg, docs, dirty, removed, doc_part, &part)?;
        return Ok(());
    }
    let doc = docs.get_mut(&part).expect("已载入 glossary");
    let idx = match doc_part_child_index(&doc.root, n) {
        Some(i) => i,
        None => return Err(missing_part(err_id, op_i)),
    };
    let parts = doc
        .root
        .child_by_local_mut("docParts")
        .expect("有 docPart 则有 docParts");
    parts.children.remove(idx);
    dirty.insert(part, ());
    Ok(())
}

pub fn insert_content(
    pkg: &Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    after: Option<&BodyPara>,
    n: usize,
    err_id: &str,
    host_id: &str,
    op_i: usize,
) -> Result<(), Error> {
    let Some(g_part) = find_glossary_part(pkg, doc_part)? else {
        return Err(missing_part_at(err_id, op_i, "docPart"));
    };
    if !docs.contains_key(&g_part) {
        let parsed = xml::parse(pkg.get(&g_part)?)?;
        docs.insert(g_part.clone(), parsed);
    }
    let (mut blocks, behavior) = {
        let gdoc = docs.get(&g_part).expect("已载入 glossary");
        let parts = doc_part_elements(&gdoc.root);
        if n < 1 || n > parts.len() {
            return Err(missing_part_at(err_id, op_i, "docPart"));
        }
        let el = parts[n - 1];
        let behavior = el
            .child_by_local("docPartPr")
            .and_then(|p| p.child_by_local("behaviors"))
            .and_then(|b| b.child_by_local("behavior"))
            .and_then(|b| b.attr_local("val"))
            .unwrap_or(DEFAULT_BEHAVIOR)
            .to_string();
        let body = el.child_by_local("docPartBody");
        let blocks = body.map(body_blocks).unwrap_or_default();
        (blocks, behavior)
    };
    let main = docs.get_mut(doc_part).expect("主文档已载入");
    let hex = para::next_para_hex(&main.root);
    let mut next = u32::from_str_radix(&hex, 16).unwrap_or(1);
    if behavior == "pg" {
        let body = para::story_body_mut(&mut main.root).ok_or_else(|| {
            Error::with_path(
                ErrorKind::XmlMissing {
                    parent: "document".into(),
                    child: "body".into(),
                },
                format!("ops[{op_i}].id"),
            )
        })?;
        let id = format!("{:08X}", next.max(1));
        next = next.max(1).saturating_add(1);
        let pg = page_break_para(body, &id);
        blocks.insert(0, pg);
    }
    for b in &mut blocks {
        rewrite_para_ids(b, &mut next);
    }
    insert_blocks(&mut main.root, after, blocks, host_id, op_i)?;
    dirty.insert(doc_part.to_string(), ());
    Ok(())
}

fn missing_part(id: &str, op_i: usize) -> Error {
    missing_part_at(id, op_i, "id")
}

fn missing_part_at(id: &str, op_i: usize, field: &str) -> Error {
    Error::with_path(
        ErrorKind::DocPartNotFound { id: id.to_string() },
        format!("ops[{op_i}].{field}"),
    )
}

fn find_glossary_part(pkg: &Package, doc_part: &str) -> Result<Option<String>, Error> {
    let rels = Relationships::from_package(pkg, doc_part)?;
    for r in rels.find_by_type_suffix("/glossaryDocument") {
        if r.is_external() {
            continue;
        }
        let Ok(p) = opc::resolve_rel_target(doc_part, &r.target) else {
            continue;
        };
        if pkg.contains(&p) {
            return Ok(Some(p));
        }
    }
    Ok(None)
}

fn ensure_glossary_part(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
) -> Result<String, Error> {
    if let Some(existing) = find_glossary_part(pkg, doc_part)? {
        if !docs.contains_key(&existing) {
            let parsed = xml::parse(pkg.get(&existing)?)?;
            docs.insert(existing.clone(), parsed);
        }
        return Ok(existing);
    }
    pkg.set(GLOSSARY_PART, empty_glossary_xml().into_bytes())?;
    let parsed = xml::parse(pkg.get(GLOSSARY_PART)?)?;
    docs.insert(GLOSSARY_PART.to_string(), parsed);
    dirty.insert(GLOSSARY_PART.to_string(), ());

    write_satellites(pkg, docs, doc_part)?;

    let mut g_rels = Relationships { rels: Vec::new() };
    push_rel(
        &mut g_rels,
        GLOSSARY_PART,
        REL_STYLES,
        sibling(GLOSSARY_PART, "styles.xml"),
    );
    push_rel(
        &mut g_rels,
        GLOSSARY_PART,
        REL_SETTINGS,
        sibling(GLOSSARY_PART, "settings.xml"),
    );
    push_rel(
        &mut g_rels,
        GLOSSARY_PART,
        REL_WEB_SETTINGS,
        sibling(GLOSSARY_PART, "webSettings.xml"),
    );
    push_rel(
        &mut g_rels,
        GLOSSARY_PART,
        REL_FONT_TABLE,
        sibling(GLOSSARY_PART, "fontTable.xml"),
    );
    g_rels.write_to(pkg, GLOSSARY_PART)?;

    let mut doc_rels = Relationships::from_package(pkg, doc_part)?;
    doc_rels.push(Relationship {
        id: doc_rels.next_rid(),
        rel_type: REL_GLOSSARY.into(),
        target: picture::rel_target_from(doc_part, GLOSSARY_PART),
        target_mode: None,
        extra: Vec::new(),
    });
    doc_rels.write_to(pkg, doc_part)?;

    let mut cts = ContentTypes::from_package(pkg)?;
    cts.set_override(GLOSSARY_PART, CT_GLOSSARY)?;
    cts.set_override(&sibling(GLOSSARY_PART, "styles.xml"), CT_STYLES)?;
    cts.set_override(&sibling(GLOSSARY_PART, "settings.xml"), CT_SETTINGS)?;
    cts.set_override(&sibling(GLOSSARY_PART, "webSettings.xml"), CT_WEB_SETTINGS)?;
    cts.set_override(&sibling(GLOSSARY_PART, "fontTable.xml"), CT_FONT_TABLE)?;
    cts.write_to(pkg)?;
    Ok(GLOSSARY_PART.to_string())
}

fn write_satellites(
    pkg: &mut Package,
    docs: &HashMap<String, Document>,
    doc_part: &str,
) -> Result<(), Error> {
    let rels = Relationships::from_package(pkg, doc_part)?;
    let styles_src = first_internal_target(pkg, doc_part, &rels, "/styles")?;
    let settings_src = first_internal_target(pkg, doc_part, &rels, "/settings")?;
    let styles = match styles_src {
        Some(src) => part_bytes(pkg, docs, &src)?.unwrap_or_else(|| minimal_styles().into_bytes()),
        None => minimal_styles().into_bytes(),
    };
    let settings = match settings_src {
        Some(src) => {
            part_bytes(pkg, docs, &src)?.unwrap_or_else(|| minimal_settings().into_bytes())
        }
        None => minimal_settings().into_bytes(),
    };
    pkg.set(&sibling(GLOSSARY_PART, "styles.xml"), styles)?;
    pkg.set(&sibling(GLOSSARY_PART, "settings.xml"), settings)?;
    pkg.set(
        &sibling(GLOSSARY_PART, "webSettings.xml"),
        minimal_web_settings().into_bytes(),
    )?;
    pkg.set(
        &sibling(GLOSSARY_PART, "fontTable.xml"),
        minimal_font_table().into_bytes(),
    )?;
    Ok(())
}

fn first_internal_target(
    pkg: &Package,
    source: &str,
    rels: &Relationships,
    suffix: &str,
) -> Result<Option<String>, Error> {
    for r in rels.find_by_type_suffix(suffix) {
        if r.is_external() {
            continue;
        }
        if let Ok(p) = opc::resolve_rel_target(source, &r.target) {
            if pkg.contains(&p) {
                return Ok(Some(p));
            }
        }
    }
    Ok(None)
}

fn part_bytes(
    pkg: &Package,
    docs: &HashMap<String, Document>,
    name: &str,
) -> Result<Option<Vec<u8>>, Error> {
    if let Some(d) = docs.get(name) {
        return Ok(Some(d.to_bytes()?));
    }
    if pkg.contains(name) {
        return Ok(Some(pkg.get(name)?.to_vec()));
    }
    Ok(None)
}

fn drop_glossary(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    removed: &mut HashSet<String>,
    doc_part: &str,
    glossary: &str,
) -> Result<(), Error> {
    let mut rels = Relationships::from_package(pkg, doc_part)?;
    rels.remove_by_target_part(doc_part, glossary);
    rels.write_to(pkg, doc_part)?;

    let satellites = [
        sibling(glossary, "styles.xml"),
        sibling(glossary, "settings.xml"),
        sibling(glossary, "webSettings.xml"),
        sibling(glossary, "fontTable.xml"),
        rels_part_name(glossary),
    ];
    let mut cts = ContentTypes::from_package(pkg)?;
    cts.remove_override(glossary);
    for s in &satellites {
        cts.remove_override(s);
    }
    cts.write_to(pkg)?;

    pkg.remove(glossary);
    removed.insert(glossary.to_string());
    docs.remove(glossary);
    dirty.remove(glossary);
    for s in &satellites {
        pkg.remove(s);
        removed.insert(s.clone());
        docs.remove(s);
        dirty.remove(s);
    }
    Ok(())
}

fn push_rel(rels: &mut Relationships, source: &str, rel_type: &str, target: String) {
    rels.push(Relationship {
        id: rels.next_rid(),
        rel_type: rel_type.into(),
        target: picture::rel_target_from(source, &target),
        target_mode: None,
        extra: Vec::new(),
    });
}

fn sibling(part: &str, file: &str) -> String {
    match part.rsplit_once('/') {
        Some((dir, _)) if !dir.is_empty() => format!("{dir}/{file}"),
        _ => format!("/{file}"),
    }
}

fn empty_glossary_xml() -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:glossaryDocument xmlns:w="{WML_NS}" xmlns:r="{REL_NS}" xmlns:w14="{W14_NS}" xmlns:mc="{MC_NS}" mc:Ignorable="w14"><w:docParts></w:docParts></w:glossaryDocument>"#
    )
}

fn minimal_styles() -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:styles xmlns:w="{WML_NS}"><w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/><w:qFormat/></w:style></w:styles>"#
    )
}

fn minimal_settings() -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:settings xmlns:w="{WML_NS}"><w:compat><w:compatSetting w:name="compatibilityMode" w:uri="http://schemas.microsoft.com/office/word" w:val="15"/></w:compat></w:settings>"#
    )
}

fn minimal_web_settings() -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:webSettings xmlns:w="{WML_NS}"/>"#
    )
}

fn minimal_font_table() -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:fonts xmlns:w="{WML_NS}"><w:font w:name="Calibri"><w:charset w:val="00"/><w:family w:val="swiss"/><w:pitch w:val="variable"/></w:font></w:fonts>"#
    )
}

fn new_guid() -> String {
    let mut b = [0u8; 16];
    crate::host_env::fill_random(&mut b);
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    format!(
        "{{{:02X}{:02X}{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}}}",
        b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7], b[8], b[9], b[10], b[11], b[12], b[13],
        b[14], b[15]
    )
}

fn identities(
    pkg: &Package,
    docs: &HashMap<String, Document>,
    doc_part: &str,
) -> Result<Vec<(String, String, String)>, Error> {
    let Some(part) = find_glossary_part(pkg, doc_part)? else {
        return Ok(Vec::new());
    };
    let owned;
    let root = if let Some(doc) = docs.get(&part) {
        &doc.root
    } else {
        owned = xml::parse(pkg.get(&part)?)?;
        &owned.root
    };
    Ok(doc_part_elements(root).into_iter().map(identity).collect())
}

fn identity(el: &Element) -> (String, String, String) {
    let pr = el.child_by_local("docPartPr");
    let name = pr
        .and_then(|p| p.child_by_local("name"))
        .and_then(|e| e.attr_local("val"))
        .unwrap_or("")
        .to_string();
    let cat = pr.and_then(|p| p.child_by_local("category"));
    let gallery = cat
        .and_then(|c| c.child_by_local("gallery"))
        .and_then(|g| g.attr_local("val"))
        .unwrap_or("")
        .to_string();
    let category = cat
        .and_then(|c| c.child_by_local("name"))
        .and_then(|g| g.attr_local("val"))
        .unwrap_or("")
        .to_string();
    (name, gallery, category)
}

fn doc_part_elements(root: &Element) -> Vec<&Element> {
    let Some(parts) = root.child_by_local("docParts") else {
        return Vec::new();
    };
    parts
        .element_children()
        .filter(|e| e.local_name() == "docPart")
        .collect()
}

fn doc_part_child_index(root: &Element, n: usize) -> Option<usize> {
    let parts = root.child_by_local("docParts")?;
    let mut seen = 0usize;
    for (i, child) in parts.children.iter().enumerate() {
        if let Node::Element(e) = child {
            if e.local_name() == "docPart" {
                seen += 1;
                if seen == n {
                    return Some(i);
                }
            }
        }
    }
    None
}

fn append_doc_part(root: &mut Element, write: &DocPartWrite<'_>) {
    if root.child_by_local("docParts").is_none() {
        let el = Element::new(root.prefixed("docParts"));
        root.push_element(el);
    }
    let parts = root.child_by_local_mut("docParts").expect("刚补上");
    let part = build_doc_part(parts, write);
    parts.push_element(part);
}

fn build_doc_part(host: &Element, write: &DocPartWrite<'_>) -> Element {
    let mut part = Element::new(host.prefixed("docPart"));
    let mut pr = Element::new(part.prefixed("docPartPr"));
    pr.push_element(val_el(&pr, "name", write.name));
    let mut cat = Element::new(pr.prefixed("category"));
    cat.push_element(val_el(&cat, "name", write.category));
    cat.push_element(val_el(&cat, "gallery", write.gallery));
    pr.push_element(cat);
    let mut behaviors = Element::new(pr.prefixed("behaviors"));
    behaviors.push_element(val_el(&behaviors, "behavior", write.behavior));
    pr.push_element(behaviors);
    pr.push_element(val_el(&pr, "guid", &new_guid()));
    part.push_element(pr);
    let mut body = Element::new(part.prefixed("docPartBody"));
    body.push_element(para::new_paragraph(&body, "00000001", write.text));
    part.push_element(body);
    part
}

fn val_el(parent: &Element, local: &str, val: &str) -> Element {
    Element::new(parent.prefixed(local)).with_attr(prefixed_attr(parent, "val"), val)
}

fn prefixed_attr(el: &Element, local: &str) -> String {
    match el.name.split_once(':') {
        Some((pfx, _)) => format!("{pfx}:{local}"),
        None => local.to_string(),
    }
}

fn collect_t(el: &Element) -> String {
    let mut out = String::new();
    walk_t(el, &mut out);
    out
}

fn walk_t(el: &Element, out: &mut String) {
    crate::xml::walk_elements(el, &mut |node| {
        if node.local_name() == "t" {
            out.push_str(&node.text_content());
        }
    });
}

fn body_blocks(body: &Element) -> Vec<Element> {
    let mut blocks: Vec<Element> = body
        .element_children()
        .filter(|e| e.local_name() != "sectPr")
        .cloned()
        .collect();
    while blocks.last().is_some_and(is_empty_p) {
        blocks.pop();
    }
    blocks
}

fn is_empty_p(el: &Element) -> bool {
    el.local_name() == "p" && !has_payload(el)
}

fn has_payload(el: &Element) -> bool {
    let n = el.local_name();
    if matches!(
        n,
        "tbl" | "drawing" | "pict" | "sdt" | "object" | "oMath" | "oMathPara"
    ) {
        return true;
    }
    if n == "t" && !el.text_content().is_empty() {
        return true;
    }
    if n == "br" {
        return true;
    }
    el.element_children().any(has_payload)
}

fn rewrite_para_ids(el: &mut Element, next: &mut u32) {
    if el.local_name() == "p" {
        let n = (*next).max(1);
        let id = format!("{n:08X}");
        *next = n.saturating_add(1);
        el.set_attr("w14:paraId", &id);
        el.set_attr("w14:textId", &id);
    }
    for child in el.element_children_mut() {
        rewrite_para_ids(child, next);
    }
}

fn page_break_para(body: &Element, para_id: &str) -> Element {
    let mut p = para::new_paragraph(body, para_id, "");
    let mut br = Element::new(p.prefixed("br"));
    br.set_attr(&prefixed_attr(&p, "type"), "page");
    let mut r = Element::new(p.prefixed("r"));
    r.push_element(br);
    p.push_element(r);
    p
}

fn insert_blocks(
    root: &mut Element,
    after: Option<&BodyPara>,
    blocks: Vec<Element>,
    err_id: &str,
    op_i: usize,
) -> Result<(), Error> {
    if blocks.is_empty() {
        return Ok(());
    }
    if let Some(para) = after {
        let path = para::find_paragraph_path(root, para).ok_or_else(|| {
            Error::with_path(
                ErrorKind::ParaNotFound {
                    id: err_id.to_string(),
                },
                format!("ops[{op_i}].id"),
            )
        })?;
        let parent_path = path[..path.len().saturating_sub(1)].to_vec();
        let parent = para::element_at(root, &parent_path).ok_or_else(|| {
            Error::with_path(
                ErrorKind::ParaNotFound {
                    id: err_id.to_string(),
                },
                format!("ops[{op_i}].id"),
            )
        })?;
        if parent.local_name() != "body" {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::InsertDocPartContentNeedsBodyPara,
                },
                format!("ops[{op_i}].id"),
            ));
        }
        let (host, idx) = para::parent_and_index_mut(root, &path).ok_or_else(|| {
            Error::with_path(
                ErrorKind::ParaNotFound {
                    id: err_id.to_string(),
                },
                format!("ops[{op_i}].id"),
            )
        })?;
        for (i, block) in blocks.into_iter().enumerate() {
            host.children.insert(idx + 1 + i, Node::Element(block));
        }
    } else {
        let body = para::story_body_mut(root).ok_or_else(|| {
            Error::with_path(
                ErrorKind::XmlMissing {
                    parent: "document".into(),
                    child: "body".into(),
                },
                format!("ops[{op_i}].id"),
            )
        })?;
        for block in blocks {
            para::append_before_body_sect_pr(body, block);
        }
    }
    Ok(())
}
