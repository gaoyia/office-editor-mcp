//! 工作表线程化批注（MS-XLSX）。不把旧式 VML 批注当树节点。

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{json, Map, Value};

use crate::error::{Error, ErrorKind, IdReason};
use crate::opc::{self, ContentTypes, Package, Relationship, Relationships};
use crate::xml::{self, Document, Element, Node};

use super::a1::{self, CellRef};
use super::apply;
use super::layout;
use super::picture;
use super::request::{
    DeleteCommentOp, InsertCommentOp, ReplyCommentOp, SetCommentDoneOp, SetCommentMetaOp,
    SetCommentTextOp,
};
use super::REL_NS;

pub const REL_THREADED: &str =
    "http://schemas.microsoft.com/office/2017/10/relationships/threadedComment";
pub const REL_PERSON: &str = "http://schemas.microsoft.com/office/2017/10/relationships/person";
pub const REL_COMMENTS: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/comments";
pub const REL_VML: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/vmlDrawing";

pub const CT_THREADED: &str = "application/vnd.ms-excel.threadedcomments+xml";
pub const CT_PERSON: &str = "application/vnd.ms-excel.person+xml";
pub const CT_COMMENTS: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.comments+xml";
pub const CT_VML: &str = "application/vnd.openxmlformats-officedocument.vmlDrawing";

pub const TC_NS: &str = "http://schemas.microsoft.com/office/spreadsheetml/2018/threadedcomments";
const XR_NS: &str = "http://schemas.microsoft.com/office/spreadsheetml/2014/revision";
const MC_NS: &str = "http://schemas.openxmlformats.org/markup-compatibility/2006";
const PERSON_PART: &str = "/xl/persons/person.xml";

static GUID_SEQ: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone)]
pub enum CommentKey {
    Top(String),
    Reply { parent: String, reply: String },
}

#[derive(Debug, Clone)]
pub struct CommentTarget {
    pub sheet: String,
    pub a1: String,
    pub key: CommentKey,
}

struct Person {
    id: String,
    name: String,
}

pub fn parse_comment_id(id: &str, json_path: &str) -> Result<CommentTarget, Error> {
    let path = json_path.to_string();
    let rest = id
        .strip_prefix("cell:")
        .ok_or_else(|| invalid_form(&path))?;
    let bang = rest.rfind('!').ok_or_else(|| invalid_form(&path))?;
    let sheet = rest[..bang].to_string();
    if sheet.is_empty() {
        return Err(invalid_form(&path));
    }
    let after_bang = &rest[bang + 1..];
    let (a1raw, after_comment) = after_bang
        .split_once("/comment:")
        .ok_or_else(|| invalid_form(&path))?;
    let a1 = a1::normalize_a1(a1raw).map_err(|_| invalid_form(&path))?;
    if after_comment.is_empty() {
        return Err(invalid_form(&path));
    }
    if let Some((parent, reply)) = after_comment.split_once("/reply:") {
        if !looks_like_guid(parent) || !looks_like_guid(reply) || reply.contains('/') {
            return Err(invalid_form(&path));
        }
        return Ok(CommentTarget {
            sheet,
            a1,
            key: CommentKey::Reply {
                parent: parent.to_string(),
                reply: reply.to_string(),
            },
        });
    }
    if after_comment.contains('/') || !looks_like_guid(after_comment) {
        return Err(invalid_form(&path));
    }
    Ok(CommentTarget {
        sheet,
        a1,
        key: CommentKey::Top(after_comment.to_string()),
    })
}

pub fn parse_insert_cell(id: &str, op_index: usize) -> Result<(String, String), Error> {
    let path = format!("ops[{op_index}].id");
    if id.contains("/comment:") {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::InsertCommentNeedsCell,
            },
            path,
        ));
    }
    let rest = id.strip_prefix("cell:").ok_or_else(|| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::InsertCommentNeedsCell,
            },
            path.clone(),
        )
    })?;
    let Some((name, a1raw)) = rest.split_once('!') else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::InsertCommentNeedsCell,
            },
            path,
        ));
    };
    if name.is_empty() {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::InsertCommentNeedsCell,
            },
            path,
        ));
    }
    let a1 = a1::normalize_a1(a1raw).map_err(|_| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::InsertCommentNeedsCell,
            },
            path,
        )
    })?;
    Ok((name.to_string(), a1))
}

fn invalid_form(path: &str) -> Error {
    Error::with_path(
        ErrorKind::InvalidId {
            reason: IdReason::XlsxCommentForm,
        },
        path.to_string(),
    )
}

fn looks_like_guid(s: &str) -> bool {
    let inner = s
        .strip_prefix('{')
        .and_then(|t| t.strip_suffix('}'))
        .unwrap_or("");
    inner.len() == 36 && inner.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
}

pub fn count_on_sheet(pkg: &Package, sheet_part: &str) -> Result<usize, Error> {
    let Some(part) = threaded_part(pkg, sheet_part)? else {
        return Ok(0);
    };
    if !pkg.contains(&part) {
        return Ok(0);
    }
    let doc = xml::parse(pkg.get(&part)?)?;
    Ok(threaded_elements(&doc.root).filter(|e| is_top(e)).count())
}

pub fn collect_on_sheet(
    pkg: &Package,
    sheet_part: &str,
    sheet_name: &str,
) -> Result<Vec<Value>, Error> {
    let Some(part) = threaded_part(pkg, sheet_part)? else {
        return Ok(Vec::new());
    };
    if !pkg.contains(&part) {
        return Ok(Vec::new());
    }
    let doc = xml::parse(pkg.get(&part)?)?;
    let people = load_people(pkg)?;
    let tops: Vec<&Element> = threaded_elements(&doc.root).filter(|e| is_top(e)).collect();
    let mut out = Vec::new();
    for top in tops {
        if let Some(v) = comment_json(top, &doc.root, sheet_name, &people) {
            out.push(v);
        }
    }
    Ok(out)
}

fn comment_json(
    top: &Element,
    root: &Element,
    sheet_name: &str,
    people: &[Person],
) -> Option<Value> {
    let guid = top.attr("id")?.to_string();
    let a1 = normalize_ref(top.attr("ref"));
    let a1_id = match &a1 {
        Value::String(s) => s.clone(),
        _ => top.attr("ref").unwrap_or("A1").to_string(),
    };
    let host = format!("cell:{sheet_name}!{a1_id}/comment:{guid}");
    let author = person_name(people, top.attr("personId").unwrap_or(""));
    let mut replies = Vec::new();
    for reply in threaded_elements(root).filter(|e| e.attr("parentId") == Some(guid.as_str())) {
        let Some(rg) = reply.attr("id") else { continue };
        replies.push(json!({
            "id": format!("{host}/reply:{rg}"),
            "commentId": rg,
            "author": person_name(people, reply.attr("personId").unwrap_or("")),
            "initials": Value::Null,
            "date": opt_attr(reply, "dT"),
            "done": attr_done(reply),
            "kind": "normal",
            "text": comment_text(reply),
        }));
    }
    let mut obj = Map::new();
    obj.insert("id".into(), json!(host));
    obj.insert("commentId".into(), json!(guid));
    obj.insert("author".into(), author);
    obj.insert("initials".into(), Value::Null);
    obj.insert("date".into(), opt_attr(top, "dT"));
    obj.insert("done".into(), json!(attr_done(top)));
    obj.insert("kind".into(), json!("normal"));
    obj.insert("ref".into(), a1);
    obj.insert("text".into(), json!(comment_text(top)));
    obj.insert("replies".into(), Value::Array(replies));
    Some(Value::Object(obj))
}

fn opt_attr(el: &Element, name: &str) -> Value {
    match el.attr(name) {
        Some(s) => json!(s),
        None => Value::Null,
    }
}

fn normalize_ref(raw: Option<&str>) -> Value {
    let Some(s) = raw else {
        return Value::Null;
    };
    match a1::parse_cell(s) {
        Ok(c) => json!(a1::format_cell(c.col, c.row)),
        Err(_) => Value::Null,
    }
}

fn comment_text(el: &Element) -> String {
    el.child_by_local("text")
        .map(|t| t.text_content())
        .unwrap_or_default()
}

fn person_name(people: &[Person], id: &str) -> Value {
    match people.iter().find(|p| p.id == id) {
        Some(p) => json!(p.name),
        None => Value::Null,
    }
}

fn attr_done(el: &Element) -> bool {
    matches!(
        el.attr("done").map(str::trim),
        Some("1" | "true" | "TRUE" | "on")
    )
}

fn is_top(el: &Element) -> bool {
    matches!(el.attr("parentId"), None | Some(""))
}

fn threaded_elements(root: &Element) -> impl Iterator<Item = &Element> {
    root.element_children()
        .filter(|e| e.local_name() == "threadedComment")
}

fn threaded_part(pkg: &Package, sheet_part: &str) -> Result<Option<String>, Error> {
    let rels = Relationships::from_package(pkg, sheet_part)?;
    let Some(rel) = rels.rels.iter().find(|r| r.rel_type == REL_THREADED) else {
        return Ok(None);
    };
    Ok(Some(opc::resolve_id(pkg, sheet_part, &rel.id)?))
}

fn comments_part(pkg: &Package, sheet_part: &str) -> Result<Option<String>, Error> {
    let rels = Relationships::from_package(pkg, sheet_part)?;
    let Some(rel) = rels.rels.iter().find(|r| r.rel_type == REL_COMMENTS) else {
        return Ok(None);
    };
    Ok(Some(opc::resolve_id(pkg, sheet_part, &rel.id)?))
}

fn vml_part(pkg: &Package, sheet_part: &str) -> Result<Option<String>, Error> {
    let rels = Relationships::from_package(pkg, sheet_part)?;
    let Some(rel) = rels.rels.iter().find(|r| r.rel_type == REL_VML) else {
        return Ok(None);
    };
    Ok(Some(opc::resolve_id(pkg, sheet_part, &rel.id)?))
}

fn load_people(pkg: &Package) -> Result<Vec<Person>, Error> {
    let wb = opc::office_document_part(pkg)?;
    let rels = Relationships::from_package(pkg, &wb)?;
    let Some(rel) = rels.rels.iter().find(|r| r.rel_type == REL_PERSON) else {
        return Ok(Vec::new());
    };
    let part = opc::resolve_id(pkg, &wb, &rel.id)?;
    if !pkg.contains(&part) {
        return Ok(Vec::new());
    }
    let doc = xml::parse(pkg.get(&part)?)?;
    Ok(doc
        .root
        .element_children()
        .filter(|e| e.local_name() == "person")
        .map(|e| Person {
            id: e.attr("id").unwrap_or("").to_string(),
            name: e.attr("displayName").unwrap_or("").to_string(),
        })
        .collect())
}

pub fn apply_insert(
    pkg: &mut Package,
    wb_part: &str,
    dirty_sheets: &mut Vec<(String, Document)>,
    sheet_part: &str,
    cell: CellRef,
    op: &InsertCommentOp,
) -> Result<(), Error> {
    let person_id = ensure_person(pkg, wb_part, &op.author)?;
    let created = op.date.clone().unwrap_or_else(utc_now);
    let guid = new_guid();
    let refer = a1::format_cell(cell.col, cell.row);
    let tc_part = ensure_threaded_part(pkg, sheet_part)?;
    let mut tc = xml::parse(pkg.get(&tc_part)?)?;
    tc.root.push_element(build_threaded(
        &refer, &created, &person_id, &guid, None, &op.text, None,
    ));
    pkg.set(&tc_part, tc.to_bytes()?)?;
    refresh_placeholder(pkg, sheet_part, dirty_sheets, cell, &guid)?;
    Ok(())
}

pub fn apply_reply(
    pkg: &mut Package,
    wb_part: &str,
    dirty_sheets: &mut Vec<(String, Document)>,
    sheet_part: &str,
    op: &ReplyCommentOp,
    op_index: usize,
) -> Result<(), Error> {
    let target = parse_comment_id(&op.id, &format!("ops[{op_index}].id"))?;
    let CommentKey::Top(parent) = &target.key else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::XlsxCommentForm,
            },
            format!("ops[{op_index}].id"),
        ));
    };
    let tc_part = threaded_part(pkg, sheet_part)?.ok_or_else(|| not_found(&op.id, op_index))?;
    let mut tc = xml::parse(pkg.get(&tc_part)?)?;
    let refer = {
        let parent_el =
            find_threaded_mut(&mut tc.root, parent).ok_or_else(|| not_found(&op.id, op_index))?;
        parent_el
            .attr("ref")
            .map(|s| s.to_string())
            .unwrap_or_else(|| target.a1.clone())
    };
    let cell = a1::parse_cell(&refer)
        .unwrap_or_else(|_| a1::parse_cell(&target.a1).unwrap_or(CellRef { col: 1, row: 1 }));
    let person_id = ensure_person(pkg, wb_part, &op.author)?;
    let created = op.date.clone().unwrap_or_else(utc_now);
    let reply_id = new_guid();
    tc.root.push_element(build_threaded(
        &refer,
        &created,
        &person_id,
        &reply_id,
        Some(parent),
        &op.text,
        None,
    ));
    pkg.set(&tc_part, tc.to_bytes()?)?;
    refresh_placeholder(pkg, sheet_part, dirty_sheets, cell, parent)?;
    Ok(())
}

pub fn apply_delete(
    pkg: &mut Package,
    dirty_sheets: &mut Vec<(String, Document)>,
    sheet_part: &str,
    op: &DeleteCommentOp,
    op_index: usize,
) -> Result<(), Error> {
    let target = parse_comment_id(&op.id, &format!("ops[{op_index}].id"))?;
    let tc_part = threaded_part(pkg, sheet_part)?.ok_or_else(|| not_found(&op.id, op_index))?;
    let mut tc = xml::parse(pkg.get(&tc_part)?)?;
    match &target.key {
        CommentKey::Top(guid) => {
            if !remove_threaded(&mut tc.root, guid) {
                return Err(not_found(&op.id, op_index));
            }
            remove_children_of(&mut tc.root, guid);
            pkg.set(&tc_part, tc.to_bytes()?)?;
            let cell = a1::parse_cell(&target.a1).map_err(|_| not_found(&op.id, op_index))?;
            remove_placeholder_comment(pkg, sheet_part, guid)?;
            remove_note_shape(pkg, sheet_part, dirty_sheets, cell)?;
        }
        CommentKey::Reply { parent, reply } => {
            if find_threaded_mut(&mut tc.root, parent).is_none() {
                return Err(not_found(&op.id, op_index));
            }
            if !remove_threaded(&mut tc.root, reply) {
                return Err(not_found(&op.id, op_index));
            }
            pkg.set(&tc_part, tc.to_bytes()?)?;
            let cell = a1::parse_cell(&target.a1).unwrap_or(CellRef { col: 1, row: 1 });
            refresh_placeholder(pkg, sheet_part, dirty_sheets, cell, parent)?;
        }
    }
    Ok(())
}

pub fn apply_set_text(
    pkg: &mut Package,
    dirty_sheets: &mut Vec<(String, Document)>,
    sheet_part: &str,
    op: &SetCommentTextOp,
    op_index: usize,
) -> Result<(), Error> {
    let target = parse_comment_id(&op.id, &format!("ops[{op_index}].id"))?;
    let guid = match &target.key {
        CommentKey::Top(g) => g.clone(),
        CommentKey::Reply { reply, .. } => reply.clone(),
    };
    let parent = match &target.key {
        CommentKey::Top(g) => g.clone(),
        CommentKey::Reply { parent, .. } => parent.clone(),
    };
    let tc_part = threaded_part(pkg, sheet_part)?.ok_or_else(|| not_found(&op.id, op_index))?;
    let mut tc = xml::parse(pkg.get(&tc_part)?)?;
    let host = find_threaded_mut(&mut tc.root, &guid).ok_or_else(|| not_found(&op.id, op_index))?;
    set_comment_text_el(host, &op.text);
    pkg.set(&tc_part, tc.to_bytes()?)?;
    let cell = a1::parse_cell(&target.a1).unwrap_or(CellRef { col: 1, row: 1 });
    refresh_placeholder(pkg, sheet_part, dirty_sheets, cell, &parent)?;
    Ok(())
}

pub fn apply_set_meta(
    pkg: &mut Package,
    wb_part: &str,
    dirty_sheets: &mut Vec<(String, Document)>,
    sheet_part: &str,
    op: &SetCommentMetaOp,
    op_index: usize,
) -> Result<(), Error> {
    let target = parse_comment_id(&op.id, &format!("ops[{op_index}].id"))?;
    let guid = match &target.key {
        CommentKey::Top(g) => g.clone(),
        CommentKey::Reply { reply, .. } => reply.clone(),
    };
    let parent = match &target.key {
        CommentKey::Top(g) => g.clone(),
        CommentKey::Reply { parent, .. } => parent.clone(),
    };
    let tc_part = threaded_part(pkg, sheet_part)?.ok_or_else(|| not_found(&op.id, op_index))?;
    let mut tc = xml::parse(pkg.get(&tc_part)?)?;
    let host = find_threaded_mut(&mut tc.root, &guid).ok_or_else(|| not_found(&op.id, op_index))?;
    if let Some(date) = &op.date {
        host.set_attr("dT", date);
    }
    if let Some(author) = &op.author {
        let pid = ensure_person(pkg, wb_part, author)?;
        host.set_attr("personId", &pid);
    }
    pkg.set(&tc_part, tc.to_bytes()?)?;
    let cell = a1::parse_cell(&target.a1).unwrap_or(CellRef { col: 1, row: 1 });
    refresh_placeholder(pkg, sheet_part, dirty_sheets, cell, &parent)?;
    Ok(())
}

pub fn apply_set_done(
    pkg: &mut Package,
    sheet_part: &str,
    op: &SetCommentDoneOp,
    op_index: usize,
) -> Result<(), Error> {
    let target = parse_comment_id(&op.id, &format!("ops[{op_index}].id"))?;
    let guid = match &target.key {
        CommentKey::Top(g) => g.as_str(),
        CommentKey::Reply { reply, .. } => reply.as_str(),
    };
    let tc_part = threaded_part(pkg, sheet_part)?.ok_or_else(|| not_found(&op.id, op_index))?;
    let mut tc = xml::parse(pkg.get(&tc_part)?)?;
    let host = find_threaded_mut(&mut tc.root, guid).ok_or_else(|| not_found(&op.id, op_index))?;
    if op.done {
        host.set_attr("done", "1");
    } else {
        host.remove_attr("done");
    }
    pkg.set(&tc_part, tc.to_bytes()?)?;
    Ok(())
}

fn not_found(id: &str, op_index: usize) -> Error {
    Error::with_path(
        ErrorKind::CommentNotFound { id: id.to_string() },
        format!("ops[{op_index}].id"),
    )
}

fn find_threaded_mut<'a>(root: &'a mut Element, guid: &str) -> Option<&'a mut Element> {
    root.children.iter_mut().find_map(|n| match n {
        Node::Element(e) if e.local_name() == "threadedComment" && e.attr("id") == Some(guid) => {
            Some(e)
        }
        _ => None,
    })
}

fn remove_threaded(root: &mut Element, guid: &str) -> bool {
    let Some(i) = root.children.iter().position(|n| {
        matches!(
            n,
            Node::Element(e)
                if e.local_name() == "threadedComment" && e.attr("id") == Some(guid)
        )
    }) else {
        return false;
    };
    root.children.remove(i);
    true
}

fn remove_children_of(root: &mut Element, parent: &str) {
    root.children.retain(|n| {
        !matches!(
            n,
            Node::Element(e)
                if e.local_name() == "threadedComment" && e.attr("parentId") == Some(parent)
        )
    });
}

fn set_comment_text_el(host: &mut Element, text: &str) {
    if let Some(t) = host.child_by_local_mut("text") {
        t.set_text(text);
        return;
    }
    host.children.insert(
        0,
        Node::Element(Element::new("text").with_text(text.to_string())),
    );
}

fn build_threaded(
    refer: &str,
    dt: &str,
    person_id: &str,
    id: &str,
    parent: Option<&str>,
    text: &str,
    done: Option<bool>,
) -> Element {
    let mut xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?><threadedComment xmlns="{TC_NS}" ref="{}" dT="{}" personId="{}" id="{}""#,
        xml::escape_attr(refer),
        xml::escape_attr(dt),
        xml::escape_attr(person_id),
        xml::escape_attr(id),
    );
    if let Some(p) = parent {
        xml.push_str(&format!(r#" parentId="{}""#, xml::escape_attr(p)));
    }
    if done == Some(true) {
        xml.push_str(r#" done="1""#);
    }
    xml.push_str(&format!(
        "><text>{}</text></threadedComment>",
        xml::escape_text(text)
    ));
    xml::parse(xml.as_bytes())
        .map(|d| d.root)
        .unwrap_or_else(|_| Element::new("threadedComment"))
}

fn ensure_threaded_part(pkg: &mut Package, sheet_part: &str) -> Result<String, Error> {
    if let Some(part) = threaded_part(pkg, sheet_part)? {
        if !pkg.contains(&part) {
            pkg.set(&part, empty_threaded())?;
        }
        return Ok(part);
    }
    let part = next_part(pkg, "/xl/threadedComments/threadedComment", ".xml");
    pkg.set(&part, empty_threaded())?;
    add_rel(pkg, sheet_part, REL_THREADED, &part)?;
    let mut cts = ContentTypes::from_package(pkg)?;
    cts.set_override(&part, CT_THREADED)?;
    cts.write_to(pkg)?;
    Ok(part)
}

fn empty_threaded() -> Vec<u8> {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><ThreadedComments xmlns="{TC_NS}" xmlns:x="http://schemas.openxmlformats.org/spreadsheetml/2006/main"/>"#
    )
    .into_bytes()
}

fn ensure_person(pkg: &mut Package, wb_part: &str, name: &str) -> Result<String, Error> {
    let part = ensure_person_part(pkg, wb_part)?;
    let mut doc = xml::parse(pkg.get(&part)?)?;
    if let Some(p) = doc
        .root
        .element_children()
        .find(|e| e.local_name() == "person" && e.attr("displayName") == Some(name))
    {
        return Ok(p.attr("id").unwrap_or("").to_string());
    }
    let id = new_guid();
    let person = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?><person xmlns="{TC_NS}" displayName="{}" id="{}" userId="Name" providerId="None"/>"#,
        xml::escape_attr(name),
        xml::escape_attr(&id)
    );
    let el = xml::parse(person.as_bytes())
        .map(|d| d.root)
        .unwrap_or_else(|_| Element::new("person"));
    doc.root.push_element(el);
    pkg.set(&part, doc.to_bytes()?)?;
    Ok(id)
}

fn ensure_person_part(pkg: &mut Package, wb_part: &str) -> Result<String, Error> {
    let rels = Relationships::from_package(pkg, wb_part)?;
    if let Some(rel) = rels.rels.iter().find(|r| r.rel_type == REL_PERSON) {
        let part = opc::resolve_id(pkg, wb_part, &rel.id)?;
        if !pkg.contains(&part) {
            pkg.set(&part, empty_people())?;
        }
        return Ok(part);
    }
    pkg.set(PERSON_PART, empty_people())?;
    add_rel(pkg, wb_part, REL_PERSON, PERSON_PART)?;
    let mut cts = ContentTypes::from_package(pkg)?;
    cts.set_override(PERSON_PART, CT_PERSON)?;
    cts.write_to(pkg)?;
    Ok(PERSON_PART.to_string())
}

fn empty_people() -> Vec<u8> {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><personList xmlns="{TC_NS}" xmlns:x="http://schemas.openxmlformats.org/spreadsheetml/2006/main"/>"#
    )
    .into_bytes()
}

fn ensure_comments_part(pkg: &mut Package, sheet_part: &str) -> Result<String, Error> {
    if let Some(part) = comments_part(pkg, sheet_part)? {
        if !pkg.contains(&part) {
            pkg.set(&part, empty_comments())?;
        }
        return Ok(part);
    }
    let part = next_part(pkg, "/xl/comments", ".xml");
    pkg.set(&part, empty_comments())?;
    add_rel(pkg, sheet_part, REL_COMMENTS, &part)?;
    let mut cts = ContentTypes::from_package(pkg)?;
    cts.set_override(&part, CT_COMMENTS)?;
    cts.write_to(pkg)?;
    Ok(part)
}

fn empty_comments() -> Vec<u8> {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><comments xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:mc="{MC_NS}" xmlns:xr="{XR_NS}" mc:Ignorable="xr"><authors/><commentList/></comments>"#
    )
    .into_bytes()
}

fn refresh_placeholder(
    pkg: &mut Package,
    sheet_part: &str,
    dirty_sheets: &mut Vec<(String, Document)>,
    cell: CellRef,
    top_guid: &str,
) -> Result<(), Error> {
    let tc_part = match threaded_part(pkg, sheet_part)? {
        Some(p) => p,
        None => return Ok(()),
    };
    let tc = xml::parse(pkg.get(&tc_part)?)?;
    let Some(top) = threaded_elements(&tc.root).find(|e| e.attr("id") == Some(top_guid)) else {
        return Ok(());
    };
    let people = load_people(pkg)?;
    let mut lines = vec![(
        display_name(&people, top.attr("personId").unwrap_or("")),
        comment_text(top),
    )];
    for reply in threaded_elements(&tc.root).filter(|e| e.attr("parentId") == Some(top_guid)) {
        lines.push((
            display_name(&people, reply.attr("personId").unwrap_or("")),
            comment_text(reply),
        ));
    }
    let body = placeholder_body(&lines);
    let refer = a1::format_cell(cell.col, cell.row);
    let cpart = ensure_comments_part(pkg, sheet_part)?;
    let mut cdoc = xml::parse(pkg.get(&cpart)?)?;
    if let Some(c) = find_placeholder_mut(&mut cdoc.root, top_guid) {
        set_placeholder_text(c, &body);
        if c.attr("ref") != Some(refer.as_str()) {
            c.set_attr("ref", &refer);
        }
    } else {
        append_placeholder(&mut cdoc.root, top_guid, &refer, &body);
    }
    pkg.set(&cpart, cdoc.to_bytes()?)?;
    ensure_note_shape(pkg, sheet_part, dirty_sheets, cell)?;
    Ok(())
}

fn display_name(people: &[Person], id: &str) -> String {
    people
        .iter()
        .find(|p| p.id == id)
        .map(|p| p.name.clone())
        .unwrap_or_else(|| "ooxp".into())
}

fn placeholder_body(lines: &[(String, String)]) -> String {
    let mut s = String::from("[线程批注]");
    for (author, text) in lines {
        s.push('\n');
        s.push_str(author);
        s.push_str(":\n");
        s.push_str(text);
    }
    s
}

fn find_placeholder_mut<'a>(root: &'a mut Element, guid: &str) -> Option<&'a mut Element> {
    let list = root.child_by_local_mut("commentList")?;
    list.children.iter_mut().find_map(|n| match n {
        Node::Element(e) if e.local_name() == "comment" && comment_uid(e) == Some(guid) => Some(e),
        _ => None,
    })
}

fn comment_uid(el: &Element) -> Option<&str> {
    el.attr("xr:uid").or_else(|| el.attr_local("uid"))
}

fn set_placeholder_text(comment: &mut Element, body: &str) {
    if let Some(text) = comment.child_by_local_mut("text") {
        text.children.clear();
        text.push_element(placeholder_run(body));
    } else {
        comment.push_element(placeholder_text_el(body));
    }
}

fn placeholder_text_el(body: &str) -> Element {
    let mut text = Element::new("text");
    text.push_element(placeholder_run(body));
    text
}

fn placeholder_run(body: &str) -> Element {
    let mut rpr = Element::new("rPr");
    rpr.push_element(Element::new("b"));
    rpr.push_element(Element::new("sz").with_attr("val", "9"));
    rpr.push_element(Element::new("color").with_attr("indexed", "81"));
    rpr.push_element(Element::new("rFont").with_attr("val", "Tahoma"));
    rpr.push_element(Element::new("charset").with_attr("val", "134"));
    let t = Element::new("t")
        .with_attr("xml:space", "preserve")
        .with_text(body.to_string());
    let mut r = Element::new("r");
    r.push_element(rpr);
    r.push_element(t);
    r
}

fn append_placeholder(root: &mut Element, guid: &str, refer: &str, body: &str) {
    if root.child_by_local("authors").is_none() {
        root.children
            .insert(0, Node::Element(Element::new("authors")));
    }
    let author_id = {
        let authors = root.child_by_local_mut("authors").unwrap();
        let n = authors
            .element_children()
            .filter(|e| e.local_name() == "author")
            .count();
        authors.push_element(Element::new("author").with_text(format!("tc={guid}")));
        n
    };
    if root.child_by_local("commentList").is_none() {
        root.push_element(Element::new("commentList"));
    }
    let list = root.child_by_local_mut("commentList").unwrap();
    let mut comment = Element::new("comment")
        .with_attr("ref", refer)
        .with_attr("authorId", author_id.to_string())
        .with_attr("shapeId", "0")
        .with_attr("xr:uid", guid);
    comment.push_element(placeholder_text_el(body));
    list.push_element(comment);
}

fn remove_placeholder_comment(
    pkg: &mut Package,
    sheet_part: &str,
    guid: &str,
) -> Result<(), Error> {
    let Some(part) = comments_part(pkg, sheet_part)? else {
        return Ok(());
    };
    if !pkg.contains(&part) {
        return Ok(());
    }
    let mut doc = xml::parse(pkg.get(&part)?)?;
    if let Some(list) = doc.root.child_by_local_mut("commentList") {
        list.children.retain(|n| {
            !matches!(
                n,
                Node::Element(e) if e.local_name() == "comment" && comment_uid(e) == Some(guid)
            )
        });
    }
    pkg.set(&part, doc.to_bytes()?)?;
    Ok(())
}

fn ensure_note_shape(
    pkg: &mut Package,
    sheet_part: &str,
    dirty_sheets: &mut Vec<(String, Document)>,
    cell: CellRef,
) -> Result<(), Error> {
    apply::load_dirty(pkg, dirty_sheets, sheet_part, false)?;
    let sheet = apply::dirty_of(dirty_sheets, sheet_part);
    picture::ensure_xmlns(&mut sheet.root, "r", REL_NS);
    let part = ensure_vml_part(pkg, sheet_part, &mut sheet.root)?;
    let mut vml = String::from_utf8_lossy(pkg.get(&part)?).into_owned();
    if note_on_cell(&vml, cell).is_some() {
        return Ok(());
    }
    if !vml.contains("_x0000_t202") {
        vml = insert_before_xml_end(&vml, SHAPE_TYPE);
    }
    let sid = next_shape_id(&vml);
    let shape = note_shape_xml(sid, cell);
    vml = insert_before_xml_end(&vml, &shape);
    pkg.set(&part, vml.into_bytes())?;
    Ok(())
}

fn ensure_vml_part(
    pkg: &mut Package,
    sheet_part: &str,
    sheet: &mut Element,
) -> Result<String, Error> {
    if let Some(el) = sheet
        .element_children()
        .find(|e| e.local_name() == "legacyDrawing")
    {
        if let Some(rid) = el.attr_local("id") {
            if let Ok(part) = opc::resolve_id(pkg, sheet_part, rid) {
                if !pkg.contains(&part) {
                    pkg.set(&part, empty_vml())?;
                }
                return Ok(part);
            }
        }
    }
    if let Some(part) = vml_part(pkg, sheet_part)? {
        if !pkg.contains(&part) {
            pkg.set(&part, empty_vml())?;
        }
        ensure_legacy_drawing(sheet, pkg, sheet_part, &part)?;
        return Ok(part);
    }
    let part = next_part(pkg, "/xl/drawings/vmlDrawing", ".vml");
    pkg.set(&part, empty_vml())?;
    let rid = add_rel(pkg, sheet_part, REL_VML, &part)?;
    if sheet
        .element_children()
        .all(|e| e.local_name() != "legacyDrawing")
    {
        layout::insert_child_ordered(
            sheet,
            Element::new("legacyDrawing").with_attr("r:id", &rid),
            "legacyDrawing",
        );
    }
    let mut cts = ContentTypes::from_package(pkg)?;
    cts.ensure_default("vml", CT_VML);
    cts.write_to(pkg)?;
    Ok(part)
}

fn ensure_legacy_drawing(
    sheet: &mut Element,
    pkg: &Package,
    sheet_part: &str,
    vml: &str,
) -> Result<(), Error> {
    if sheet
        .element_children()
        .any(|e| e.local_name() == "legacyDrawing")
    {
        return Ok(());
    }
    let rels = Relationships::from_package(pkg, sheet_part)?;
    let rid = rels
        .rels
        .iter()
        .find(|r| r.rel_type == REL_VML)
        .map(|r| r.id.clone())
        .unwrap_or_else(|| "rId1".into());
    let _ = vml;
    layout::insert_child_ordered(
        sheet,
        Element::new("legacyDrawing").with_attr("r:id", rid),
        "legacyDrawing",
    );
    Ok(())
}

fn empty_vml() -> Vec<u8> {
    format!(
        r#"<xml xmlns:v="urn:schemas-microsoft-com:vml" xmlns:o="urn:schemas-microsoft-com:office:office" xmlns:x="urn:schemas-microsoft-com:office:excel">{SHAPE_TYPE}</xml>"#
    )
    .into_bytes()
}

const SHAPE_TYPE: &str = r#"<o:shapelayout v:ext="edit"><o:idmap v:ext="edit" data="1"/></o:shapelayout><v:shapetype id="_x0000_t202" coordsize="21600,21600" o:spt="202" path="m,l,21600r21600,l21600,xe"><v:stroke joinstyle="miter"/><v:path gradientshapeok="t" o:connecttype="rect"/></v:shapetype>"#;

fn note_shape_xml(sid: u32, cell: CellRef) -> String {
    let row0 = cell.row.saturating_sub(1);
    let col0 = cell.col.saturating_sub(1);
    format!(
        r##"<v:shape id="_x0000_s{sid}" type="#_x0000_t202" style="position:absolute;margin-left:59.25pt;margin-top:1.5pt;width:108pt;height:59.25pt;z-index:1;visibility:hidden" fillcolor="#ffffe1" o:insetmode="auto"><v:fill color2="#ffffe1"/><v:shadow color="black" obscured="t"/><v:path o:connecttype="none"/><v:textbox style="mso-direction-alt:auto"><div style="text-align:left"></div></v:textbox><x:ClientData ObjectType="Note"><x:MoveWithCells/><x:SizeWithCells/><x:AutoFill>False</x:AutoFill><x:Row>{row0}</x:Row><x:Column>{col0}</x:Column></x:ClientData></v:shape>"##
    )
}

fn remove_note_shape(
    pkg: &mut Package,
    sheet_part: &str,
    dirty_sheets: &mut Vec<(String, Document)>,
    cell: CellRef,
) -> Result<(), Error> {
    apply::load_dirty(pkg, dirty_sheets, sheet_part, false)?;
    let Some(part) = vml_part(pkg, sheet_part)? else {
        return Ok(());
    };
    if !pkg.contains(&part) {
        return Ok(());
    }
    let vml = String::from_utf8_lossy(pkg.get(&part)?).into_owned();
    let Some((_, start, end)) = note_on_cell(&vml, cell) else {
        return Ok(());
    };
    let mut out = String::with_capacity(vml.len());
    out.push_str(&vml[..start]);
    out.push_str(&vml[end..]);
    pkg.set(&part, out.into_bytes())?;
    Ok(())
}

fn note_on_cell(vml: &str, cell: CellRef) -> Option<(u32, usize, usize)> {
    let row0 = cell.row.saturating_sub(1);
    let col0 = cell.col.saturating_sub(1);
    let mut best: Option<(u32, usize, usize)> = None;
    for (start, end) in shape_spans(vml) {
        let body = &vml[start..end];
        if !is_note(body) {
            continue;
        }
        if xml_u32_tag(body, "x:Row") != Some(row0) || xml_u32_tag(body, "x:Column") != Some(col0) {
            continue;
        }
        let sid = shape_s_num(body);
        if best.map(|(b, _, _)| sid >= b).unwrap_or(true) {
            best = Some((sid, start, end));
        }
    }
    best
}

fn is_note(shape: &str) -> bool {
    shape.contains(r#"ObjectType="Note""#) || shape.contains("ObjectType='Note'")
}

fn shape_s_num(shape: &str) -> u32 {
    let Some(i) = shape.find("_x0000_s") else {
        return 0;
    };
    let rest = &shape[i + 8..];
    let n: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    n.parse().unwrap_or(0)
}

fn next_shape_id(vml: &str) -> u32 {
    let mut max = 1024u32;
    let mut from = 0usize;
    while let Some(i) = vml[from..].find("_x0000_s") {
        let abs = from + i + 8;
        let n: String = vml[abs..]
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        if let Ok(v) = n.parse::<u32>() {
            max = max.max(v);
        }
        from = abs;
    }
    max + 1
}

fn shape_spans(vml: &str) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut from = 0usize;
    while let Some(rel) = vml[from..].find("<v:shape") {
        let start = from + rel;
        let Some(rel_end) = vml[start..].find("</v:shape>") else {
            break;
        };
        let end = start + rel_end + "</v:shape>".len();
        out.push((start, end));
        from = end;
    }
    out
}

fn xml_u32_tag(s: &str, tag: &str) -> Option<u32> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let a = s.find(&open)?;
    let rest = &s[a + open.len()..];
    let b = rest.find(&close)?;
    rest[..b].trim().parse().ok()
}

fn insert_before_xml_end(vml: &str, chunk: &str) -> String {
    if let Some(i) = vml.rfind("</xml>") {
        let mut s = String::with_capacity(vml.len() + chunk.len());
        s.push_str(&vml[..i]);
        s.push_str(chunk);
        s.push_str(&vml[i..]);
        s
    } else {
        format!("{vml}{chunk}")
    }
}

fn add_rel(pkg: &mut Package, source: &str, rel_type: &str, target: &str) -> Result<String, Error> {
    let mut rels = Relationships::from_package(pkg, source)?;
    let rid = rels.next_rid();
    rels.push(Relationship {
        id: rid.clone(),
        rel_type: rel_type.into(),
        target: picture::rel_target_from(source, target),
        target_mode: None,
        extra: Vec::new(),
    });
    rels.write_to(pkg, source)?;
    Ok(rid)
}

fn next_part(pkg: &Package, prefix: &str, suffix: &str) -> String {
    let mut n = 1u32;
    loop {
        let name = format!("{prefix}{n}{suffix}");
        if !pkg.contains(&name) {
            return name;
        }
        n += 1;
    }
}

fn new_guid() -> String {
    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let n = GUID_SEQ.fetch_add(1, Ordering::Relaxed) as u128;
    let pid = std::process::id() as u128;
    let mix = t ^ (n << 48) ^ (pid << 16);
    let mut b = [0u8; 16];
    for (i, byte) in b.iter_mut().enumerate() {
        *byte = (mix >> ((i % 16) * 8)) as u8 ^ (i as u8).wrapping_mul(31);
    }
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    format!(
        "{{{:02X}{:02X}{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}}}",
        b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7], b[8], b[9], b[10], b[11], b[12], b[13], b[14],
        b[15]
    )
}

fn utc_now() -> String {
    let dur = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    unix_to_dt(dur.as_secs(), dur.subsec_millis())
}

fn unix_to_dt(secs: u64, millis: u32) -> String {
    let mut days = secs / 86_400;
    let rem = secs % 86_400;
    let hour = rem / 3600;
    let min = (rem % 3600) / 60;
    let sec = rem % 60;
    let mut year = 1970u64;
    loop {
        let len = if is_leap(year) { 366 } else { 365 };
        if days < len {
            break;
        }
        days -= len;
        year += 1;
    }
    let dim = [
        31u64,
        if is_leap(year) { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut month = 1u64;
    for (i, d) in dim.iter().enumerate() {
        if days < *d {
            month = i as u64 + 1;
            break;
        }
        days -= *d;
    }
    let day = days + 1;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{min:02}:{sec:02}.{millis:03}")
}

fn is_leap(y: u64) -> bool {
    y.is_multiple_of(4) && !y.is_multiple_of(100) || y.is_multiple_of(400)
}
