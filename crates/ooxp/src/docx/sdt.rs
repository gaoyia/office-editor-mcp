//! 正文、页眉页脚与文本框里被收入的 `w:sdt`：inspect 名单与插改删。

use serde_json::{json, Value};

use crate::error::{Error, ErrorKind, IdReason, XmlIssue};
use crate::xml::{self, Element, Node};

use super::comments::W15_NS;
use super::hf::{self, HfPart};
use super::link::{self, LinkStory};
use super::para::{self, BodyPara};
use super::{MC_NS, W14_NS};

const CHECKED_MARK: &str = "\u{2612}";
const UNCHECKED_MARK: &str = "\u{2610}";
const CHECKBOX_FONT: &str = "MS Gothic";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SdtKind {
    PlainText,
    RichText,
    DropDown,
    ComboBox,
    Date,
    RepeatingSection,
    Checkbox,
    Picture,
    Group,
    Equation,
}

impl SdtKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PlainText => "plainText",
            Self::RichText => "richText",
            Self::DropDown => "dropDown",
            Self::ComboBox => "comboBox",
            Self::Date => "date",
            Self::RepeatingSection => "repeatingSection",
            Self::Checkbox => "checkbox",
            Self::Picture => "picture",
            Self::Group => "group",
            Self::Equation => "equation",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "plainText" => Some(Self::PlainText),
            "richText" => Some(Self::RichText),
            "dropDown" => Some(Self::DropDown),
            "comboBox" => Some(Self::ComboBox),
            "date" => Some(Self::Date),
            "checkbox" => Some(Self::Checkbox),
            "picture" => Some(Self::Picture),
            "group" => Some(Self::Group),
            "equation" => Some(Self::Equation),
            "repeatingSection" => Some(Self::RepeatingSection),
            _ => None,
        }
    }

    fn type_local(self) -> &'static str {
        match self {
            Self::PlainText => "text",
            Self::RichText => "richText",
            Self::DropDown => "dropDownList",
            Self::ComboBox => "comboBox",
            Self::Date => "date",
            Self::RepeatingSection => "repeatingSection",
            Self::Checkbox => "checkbox",
            Self::Picture => "picture",
            Self::Group => "group",
            Self::Equation => "equation",
        }
    }

    pub fn needs_text(self) -> bool {
        !matches!(self, Self::Checkbox | Self::Picture)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SdtLevel {
    Inline,
    Block,
}

impl SdtLevel {
    fn as_str(self) -> &'static str {
        match self {
            Self::Inline => "inline",
            Self::Block => "block",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SdtKeep {
    Content,
    None,
}

impl SdtKeep {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "content" => Some(Self::Content),
            "none" => Some(Self::None),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SdtRef {
    pub story: LinkStory,
    pub n: usize,
}

struct SdtRec {
    path: Vec<usize>,
    kind: SdtKind,
    level: SdtLevel,
    parent: Option<usize>,
}

pub fn count_sdts(root: &Element) -> usize {
    collect_sdts(root).len()
}

pub fn sdts_json(root: &Element, container: Option<&str>) -> Result<Vec<Value>, Error> {
    let index_of = para_index_map(root);
    let recs = collect_sdts(root);
    let mut out = Vec::new();
    for (i, rec) in recs.iter().enumerate() {
        let n = i + 1;
        let id = match container {
            Some(c) => format!("{c}/sdt:{n}"),
            None => format!("sdt:{n}"),
        };
        let sdt = para::element_at(root, &rec.path).ok_or_else(|| xml_broken("sdt"))?;
        let pr = sdt.child_by_local("sdtPr");
        let parent = rec.parent.map(|p| match container {
            Some(c) => json!(format!("{c}/sdt:{}", p + 1)),
            None => json!(format!("sdt:{}", p + 1)),
        });
        let mut node = json!({
            "id": id,
            "index": n,
            "kind": rec.kind.as_str(),
            "level": rec.level.as_str(),
            "host": host_para_id(root, rec, container, &index_of),
            "tag": pr_val(pr, "tag"),
            "alias": pr_val(pr, "alias"),
            "xmlId": pr_val(pr, "id"),
            "bound": pr.is_some_and(|p| p.child_by_local("dataBinding").is_some()),
            "parent": parent.unwrap_or(Value::Null),
            "text": sdt_text(sdt),
        });
        if matches!(rec.kind, SdtKind::DropDown | SdtKind::ComboBox) {
            node["items"] = Value::Array(list_items(pr));
        }
        if rec.kind == SdtKind::Date {
            node["fullDate"] = date_full(pr);
        }
        if rec.kind == SdtKind::Checkbox {
            node["checked"] = json!(checkbox_checked(pr));
        }
        out.push(node);
    }
    Ok(out)
}

pub fn parse_sdt_ref(id: &str, path: &str) -> Result<SdtRef, Error> {
    let (story, rest) = link::split_story(id, path, "sdt", IdReason::SdtPrefix)?;
    if rest.contains('/') {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SdtPrefix,
            },
            path.to_string(),
        ));
    }
    let n = link::parse_n(
        rest,
        path,
        IdReason::SdtIndexNotInt,
        IdReason::SdtIndexFromOne,
    )?;
    Ok(SdtRef { story, n })
}

pub fn part_for_story(
    catalog: &[HfPart],
    doc_part: &str,
    story: &LinkStory,
    missing: impl FnOnce() -> Error,
) -> Result<String, Error> {
    link::part_for_story(catalog, doc_part, story, missing)
}

pub fn missing_sdt_err(id: &str, i: usize) -> Error {
    Error::with_path(
        ErrorKind::ContentControlNotFound { id: id.to_string() },
        format!("ops[{i}].id"),
    )
}

/// 目标段落是否含本期收入的行内控件。
pub fn para_blocks_set_text(root: &Element, target: &BodyPara) -> bool {
    let Some(p_path) = para::find_paragraph_path(root, target) else {
        return false;
    };
    collect_sdts(root)
        .iter()
        .any(|rec| rec.level == SdtLevel::Inline && path_starts_with(&rec.path, &p_path))
}

pub struct SdtWrite<'a> {
    pub kind: SdtKind,
    pub text: &'a str,
    pub tag: Option<&'a str>,
    pub alias: Option<&'a str>,
    pub items: &'a [String],
    pub full_date: Option<&'a str>,
    pub checked: bool,
    pub drawing: Option<Element>,
    pub binding: Option<(&'a str, &'a str, &'a str)>,
}

pub fn insert_sdt(
    root: &mut Element,
    after: Option<&[usize]>,
    mut write: SdtWrite<'_>,
    err_id: &str,
    op_i: usize,
) -> Result<(), Error> {
    if write.kind == SdtKind::Checkbox {
        ensure_w14(root);
    }
    let drawing = write.drawing.take();
    match after {
        Some(path) => {
            if write.kind == SdtKind::RepeatingSection {
                return wrap_repeating(root, path, &write, err_id, op_i);
            }
            let xml_id = next_sdt_id(root);
            let el = para::element_at_mut(root, path).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::ParaNotFound {
                        id: err_id.to_string(),
                    },
                    format!("ops[{op_i}].id"),
                )
            })?;
            el.push_element(new_inline_sdt(el, &write, xml_id, drawing));
            Ok(())
        }
        None => {
            if write.kind == SdtKind::RepeatingSection {
                return insert_repeating_at_body_end(root, &write, op_i);
            }
            insert_at_body_end(root, &write, drawing, op_i)
        }
    }
}

pub struct SdtSet<'a> {
    pub n: usize,
    pub text: Option<&'a str>,
    pub tag: Option<&'a str>,
    pub alias: Option<&'a str>,
    pub items: Option<&'a [String]>,
    pub full_date: Option<&'a str>,
    pub checked: Option<bool>,
    pub id: &'a str,
    pub i: usize,
}

pub struct BoundWrite {
    pub store_item_id: String,
    pub xpath: String,
    pub prefix_mappings: String,
    pub text: String,
}

pub fn set_sdt(root: &mut Element, set: &SdtSet<'_>) -> Result<Option<BoundWrite>, Error> {
    let rec = sdt_at(root, set.n, set.id, set.i)?;
    if rec.kind == SdtKind::RepeatingSection {
        return Err(Error::with_path(
            ErrorKind::ContentControlRepeating,
            format!("ops[{}].id", set.i),
        ));
    }
    if rec.kind == SdtKind::Group {
        return Err(Error::with_path(
            ErrorKind::ContentControlGroup,
            format!("ops[{}].id", set.i),
        ));
    }
    if set.items.is_some() && !matches!(rec.kind, SdtKind::DropDown | SdtKind::ComboBox) {
        return Err(Error::with_path(
            ErrorKind::ContentControlFieldMismatch,
            format!("ops[{}].items", set.i),
        ));
    }
    if set.full_date.is_some() && rec.kind != SdtKind::Date {
        return Err(Error::with_path(
            ErrorKind::ContentControlFieldMismatch,
            format!("ops[{}].fullDate", set.i),
        ));
    }
    if set.checked.is_some() && rec.kind != SdtKind::Checkbox {
        return Err(Error::with_path(
            ErrorKind::ContentControlFieldMismatch,
            format!("ops[{}].checked", set.i),
        ));
    }
    if rec.kind == SdtKind::Checkbox {
        if set.checked.is_none() {
            return Err(Error::with_path(
                ErrorKind::OpRequiresField {
                    op: "setContentControl".into(),
                    field: "checked".into(),
                },
                format!("ops[{}].checked", set.i),
            ));
        }
        if set.text.is_some() {
            return Err(Error::with_path(
                ErrorKind::ContentControlFieldMismatch,
                format!("ops[{}].text", set.i),
            ));
        }
        ensure_w14(root);
    }
    if rec.kind == SdtKind::Picture {
        if set.text.is_some() {
            return Err(Error::with_path(
                ErrorKind::ContentControlFieldMismatch,
                format!("ops[{}].text", set.i),
            ));
        }
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setContentControl".into(),
                field: "file".into(),
            },
            format!("ops[{}].file", set.i),
        ));
    }
    if rec.kind != SdtKind::Checkbox && set.text.is_none() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setContentControl".into(),
                field: "text".into(),
            },
            format!("ops[{}].text", set.i),
        ));
    }
    let path = rec.path.clone();
    let kind = rec.kind;
    let level = rec.level;
    let bound = {
        let sdt = para::element_at(root, &path).ok_or_else(|| missing_sdt(set.id, set.i))?;
        if let Some((store, xpath, mappings)) = simple_binding(sdt) {
            let Some(text) = set.text else {
                return Err(Error::with_path(
                    ErrorKind::ContentControlBound,
                    format!("ops[{}].id", set.i),
                ));
            };
            Some(BoundWrite {
                store_item_id: store,
                xpath,
                prefix_mappings: mappings,
                text: text.to_string(),
            })
        } else if pr_has_binding(sdt) {
            return Err(Error::with_path(
                ErrorKind::ContentControlBound,
                format!("ops[{}].id", set.i),
            ));
        } else {
            None
        }
    };
    let sdt = para::element_at_mut(root, &path).ok_or_else(|| missing_sdt(set.id, set.i))?;
    clear_showing_plchdr(sdt);
    apply_pr_meta(sdt, set.tag, set.alias);
    if let Some(list) = set.items {
        replace_items(sdt, kind, list);
    }
    if let Some(fd) = set.full_date {
        set_full_date(sdt, fd);
    }
    if kind == SdtKind::Checkbox {
        set_checkbox(sdt, set.checked.unwrap_or(false), level);
    } else if let Some(text) = set.text {
        replace_content(sdt, level, text);
    }
    Ok(bound)
}

pub fn sdt_picture_rids(
    root: &Element,
    n: usize,
    id: &str,
    i: usize,
) -> Result<Vec<String>, Error> {
    let rec = sdt_at(root, n, id, i)?;
    let sdt = para::element_at(root, &rec.path).ok_or_else(|| missing_sdt(id, i))?;
    Ok(collect_blip_rids(sdt))
}

pub fn set_sdt_picture(
    root: &mut Element,
    n: usize,
    drawing: Element,
    tag: Option<&str>,
    alias: Option<&str>,
    id: &str,
    i: usize,
) -> Result<(), Error> {
    let rec = sdt_at(root, n, id, i)?;
    if rec.kind != SdtKind::Picture {
        return Err(Error::with_path(
            ErrorKind::ContentControlFieldMismatch,
            format!("ops[{i}].file"),
        ));
    }
    let path = rec.path.clone();
    let level = rec.level;
    {
        let sdt = para::element_at(root, &path).ok_or_else(|| missing_sdt(id, i))?;
        if pr_has_binding(sdt) {
            return Err(Error::with_path(
                ErrorKind::ContentControlBound,
                format!("ops[{i}].id"),
            ));
        }
        if level == SdtLevel::Block && content_has_tbl(sdt) {
            return Err(Error::with_path(
                ErrorKind::ContentControlHasTable,
                format!("ops[{i}].id"),
            ));
        }
    }
    let sdt = para::element_at_mut(root, &path).ok_or_else(|| missing_sdt(id, i))?;
    clear_showing_plchdr(sdt);
    apply_pr_meta(sdt, tag, alias);
    replace_picture_content(sdt, level, drawing);
    Ok(())
}

pub fn delete_sdt(
    root: &mut Element,
    n: usize,
    keep: SdtKeep,
    id: &str,
    i: usize,
) -> Result<(), Error> {
    let rec = sdt_at(root, n, id, i)?;
    let path = rec.path.clone();
    if keep == SdtKeep::None && blocked_by_sect_pr(root, &path) {
        return Err(Error::with_path(
            ErrorKind::SectionBreakPara,
            format!("ops[{i}].id"),
        ));
    }
    match keep {
        SdtKeep::Content => unwrap_sdt(root, &path)?,
        SdtKeep::None => {
            let host_p = enclosing_para(root, &path);
            remove_at(root, &path)?;
            if let Some(pp) = host_p {
                drop_empty_host_para(root, &pp)?;
            }
        }
    }
    Ok(())
}

fn insert_at_body_end(
    root: &mut Element,
    write: &SdtWrite<'_>,
    drawing: Option<Element>,
    op_i: usize,
) -> Result<(), Error> {
    let xml_id = next_sdt_id(root);
    let new_id = para::next_para_hex(root);
    let body = para::story_body_mut(root).ok_or_else(|| {
        Error::with_path(
            ErrorKind::XmlMissing {
                parent: "document".into(),
                child: "body".into(),
            },
            format!("ops[{op_i}].id"),
        )
    })?;
    let mut new_p = para::new_paragraph(body, &new_id, "");
    new_p.push_element(new_inline_sdt(&new_p, write, xml_id, drawing));
    para::append_before_body_sect_pr(body, new_p);
    Ok(())
}

fn insert_repeating_at_body_end(
    root: &mut Element,
    write: &SdtWrite<'_>,
    op_i: usize,
) -> Result<(), Error> {
    ensure_w15(root);
    let (outer_id, inner_id) = next_two_sdt_ids(root);
    let new_id = para::next_para_hex(root);
    let body = para::story_body_mut(root).ok_or_else(|| {
        Error::with_path(
            ErrorKind::XmlMissing {
                parent: "document".into(),
                child: "body".into(),
            },
            format!("ops[{op_i}].id"),
        )
    })?;
    let para = para::new_paragraph(body, &new_id, write.text);
    let shell = repeating_shell(body, Node::Element(para), outer_id, inner_id, write);
    para::append_before_body_sect_pr(body, shell);
    Ok(())
}

pub(crate) fn wrap_repeating(
    root: &mut Element,
    path: &[usize],
    write: &SdtWrite<'_>,
    err_id: &str,
    op_i: usize,
) -> Result<(), Error> {
    ensure_w15(root);
    let (outer_id, inner_id) = next_two_sdt_ids(root);
    let (parent, idx) = para::parent_and_index_mut(root, path).ok_or_else(|| {
        Error::with_path(
            ErrorKind::ParaNotFound {
                id: err_id.to_string(),
            },
            format!("ops[{op_i}].id"),
        )
    })?;
    let node = parent.children.remove(idx);
    let shell = repeating_shell(parent, node, outer_id, inner_id, write);
    parent.children.insert(idx, Node::Element(shell));
    Ok(())
}

fn repeating_shell(
    parent: &Element,
    inner: Node,
    outer_id: i64,
    inner_id: i64,
    write: &SdtWrite<'_>,
) -> Element {
    let mut item_pr = Element::new(parent.prefixed("sdtPr"));
    item_pr.push_element(val_el(&item_pr, "id", &inner_id.to_string()));
    item_pr.push_element(Element::new("w15:repeatingSectionItem"));
    let mut item_content = Element::new(parent.prefixed("sdtContent"));
    item_content.children.push(inner);
    let mut item = Element::new(parent.prefixed("sdt"));
    crate::xml::mark_new(&mut item, "sdt");
    item.push_element(item_pr);
    item.push_element(item_content);

    let mut pr = Element::new(parent.prefixed("sdtPr"));
    if let Some(a) = write.alias.filter(|s| !s.is_empty()) {
        pr.push_element(val_el(&pr, "alias", a));
    }
    if let Some(t) = write.tag.filter(|s| !s.is_empty()) {
        pr.push_element(val_el(&pr, "tag", t));
    }
    pr.push_element(val_el(&pr, "id", &outer_id.to_string()));
    push_data_binding(&mut pr, write.binding);
    pr.push_element(Element::new("w15:repeatingSection"));
    let mut content = Element::new(parent.prefixed("sdtContent"));
    content.push_element(item);
    let mut sdt = Element::new(parent.prefixed("sdt"));
    crate::xml::mark_new(&mut sdt, "sdt");
    sdt.push_element(pr);
    sdt.push_element(content);
    sdt
}

fn next_two_sdt_ids(root: &Element) -> (i64, i64) {
    let first = next_sdt_id(root);
    let mut used = Vec::new();
    collect_xml_ids(root, &mut used);
    used.push(first);
    let mut second = 1i64;
    while used.contains(&second) {
        second += 1;
        if second > i32::MAX as i64 {
            second = first.saturating_add(1);
            break;
        }
    }
    (first, second)
}

fn ensure_w15(root: &mut Element) {
    let key = "xmlns:w15";
    if root.attr(key).is_none() {
        root.set_attr(key, W15_NS);
    }
    let mc = "xmlns:mc";
    if root.attr(mc).is_none() {
        root.set_attr(mc, MC_NS);
    }
    let cur = root.attr_local("Ignorable").unwrap_or("").to_string();
    if cur.split_whitespace().any(|t| t == "w15") {
        return;
    }
    let next = if cur.is_empty() {
        "w15".into()
    } else {
        format!("{cur} w15")
    };
    if let Some((_, v)) = root
        .attrs
        .iter_mut()
        .find(|(k, _)| k.rsplit(':').next() == Some("Ignorable"))
    {
        *v = next;
        return;
    }
    root.set_attr("mc:Ignorable", next);
}

fn collect_sdts(root: &Element) -> Vec<SdtRec> {
    let mut path = Vec::new();
    let mut out = Vec::new();
    para::walk_from_story_root(root, &mut path, |el, path| {
        walk_sdts(el, path, false, None, &mut out);
    });
    out
}

fn walk_sdts(
    root: &Element,
    path: &[usize],
    in_para: bool,
    parent: Option<usize>,
    out: &mut Vec<SdtRec>,
) {
    let mut stack = vec![(root, path.to_vec(), in_para, parent)];
    while let Some((el, path, in_para, parent)) = stack.pop() {
        if el.local_name() == "txbxContent" {
            continue;
        }
        if el.local_name() == "AlternateContent" {
            if let Some(i) = first_alt(el) {
                if let Node::Element(c) = &el.children[i] {
                    let mut next = path;
                    next.push(i);
                    stack.push((c, next, in_para, parent));
                }
            }
            continue;
        }
        if el.local_name() == "sdt" {
            let nested_parent = if let Some(kind) = classify(el) {
                let idx = out.len();
                out.push(SdtRec {
                    path: path.clone(),
                    kind,
                    level: if in_para {
                        SdtLevel::Inline
                    } else {
                        SdtLevel::Block
                    },
                    parent,
                });
                Some(idx)
            } else {
                parent
            };
            let mut kids = Vec::new();
            for (i, n) in el.children.iter().enumerate() {
                if let Node::Element(child) = n {
                    let mut next = path.clone();
                    next.push(i);
                    kids.push((child, next, in_para, nested_parent));
                }
            }
            for item in kids.into_iter().rev() {
                stack.push(item);
            }
            continue;
        }
        let now_para = in_para || el.local_name() == "p";
        let mut kids = Vec::new();
        for (i, n) in el.children.iter().enumerate() {
            if let Node::Element(child) = n {
                let mut next = path.clone();
                next.push(i);
                kids.push((child, next, now_para, parent));
            }
        }
        for item in kids.into_iter().rev() {
            stack.push(item);
        }
    }
}

fn classify(sdt: &Element) -> Option<SdtKind> {
    let Some(pr) = sdt.child_by_local("sdtPr") else {
        return Some(SdtKind::RichText);
    };
    let mut found = None;
    for c in pr.element_children() {
        match c.local_name() {
            "text" => found = Some(SdtKind::PlainText),
            "richText" => found = Some(SdtKind::RichText),
            "dropDownList" => found = Some(SdtKind::DropDown),
            "comboBox" => found = Some(SdtKind::ComboBox),
            "date" => found = Some(SdtKind::Date),
            "repeatingSection" | "repeatingSectionItem" => found = Some(SdtKind::RepeatingSection),
            "docPartObj" | "bibliography" | "citation" => return None,
            "picture" => found = Some(SdtKind::Picture),
            "group" => found = Some(SdtKind::Group),
            "equation" => found = Some(SdtKind::Equation),
            "checkbox" => found = Some(SdtKind::Checkbox),
            _ => {}
        }
    }
    Some(found.unwrap_or(SdtKind::RichText))
}

fn sdt_at(root: &Element, n: usize, id: &str, i: usize) -> Result<SdtRec, Error> {
    let recs = collect_sdts(root);
    para::pick_collected(root, recs, n, "sdt", |rec| rec.path.as_slice())
        .ok_or_else(|| missing_sdt(id, i))
}

pub(crate) fn pin_sdts(root: &mut Element) {
    let paths: Vec<_> = collect_sdts(root).into_iter().map(|rec| rec.path).collect();
    para::pin_collected(root, &paths, "sdt");
}

fn new_inline_sdt(
    parent: &Element,
    write: &SdtWrite<'_>,
    xml_id: i64,
    drawing: Option<Element>,
) -> Element {
    let mut sdt = Element::new(parent.prefixed("sdt"));
    crate::xml::mark_new(&mut sdt, "sdt");
    let mut pr = Element::new(sdt.prefixed("sdtPr"));
    if let Some(a) = write.alias.filter(|s| !s.is_empty()) {
        pr.push_element(val_el(&pr, "alias", a));
    }
    if let Some(t) = write.tag.filter(|s| !s.is_empty()) {
        pr.push_element(val_el(&pr, "tag", t));
    }
    pr.push_element(val_el(&pr, "id", &xml_id.to_string()));
    push_data_binding(&mut pr, write.binding);
    if write.kind == SdtKind::Checkbox {
        pr.push_element(checkbox_pr(write.checked));
    } else {
        let mut ty = Element::new(pr.prefixed(write.kind.type_local()));
        if write.kind == SdtKind::Date {
            if let Some(fd) = write.full_date {
                ty.set_attr("w:fullDate", fd);
            }
        }
        if matches!(write.kind, SdtKind::DropDown | SdtKind::ComboBox) {
            let list = if write.items.is_empty() {
                vec![write.text.to_string()]
            } else {
                write.items.to_vec()
            };
            for it in &list {
                ty.push_element(list_item_el(&ty, it));
            }
        }
        pr.push_element(ty);
    }
    sdt.push_element(pr);
    let mut content = Element::new(sdt.prefixed("sdtContent"));
    if let Some(drawing) = drawing {
        let mut run = Element::new(content.prefixed("r"));
        crate::xml::mark_new(&mut run, "run");
        let mut rpr = Element::new(run.prefixed("rPr"));
        rpr.push_element(Element::new(rpr.prefixed("noProof")));
        run.push_element(rpr);
        run.push_element(drawing);
        content.push_element(run);
    } else if write.kind == SdtKind::Checkbox {
        content.push_element(checkbox_run(&content, write.checked));
    } else {
        content.push_element(para::new_text_run(&content, write.text));
    }
    sdt.push_element(content);
    sdt
}

fn val_el(parent: &Element, local: &str, val: &str) -> Element {
    let mut e = Element::new(parent.prefixed(local));
    e.set_attr("w:val", val);
    e
}

fn list_item_el(parent: &Element, text: &str) -> Element {
    let mut e = Element::new(parent.prefixed("listItem"));
    e.set_attr("w:displayText", text);
    e.set_attr("w:value", text);
    e
}

pub(crate) fn next_sdt_id(root: &Element) -> i64 {
    let mut used = Vec::new();
    collect_xml_ids(root, &mut used);
    let mut n = 1i64;
    while used.contains(&n) {
        n += 1;
        if n > i32::MAX as i64 {
            n = 1;
            break;
        }
    }
    n
}

fn collect_xml_ids(el: &Element, used: &mut Vec<i64>) {
    if el.local_name() == "sdt" {
        if let Some(pr) = el.child_by_local("sdtPr") {
            if let Some(id) = pr.child_by_local("id") {
                if let Some(v) = id.attr_local("val") {
                    if let Ok(n) = v.parse::<i64>() {
                        used.push(n);
                    }
                }
            }
        }
    }
    for c in el.element_children() {
        collect_xml_ids(c, used);
    }
}

fn pr_val(pr: Option<&Element>, local: &str) -> Value {
    let Some(pr) = pr else {
        return Value::Null;
    };
    let Some(el) = pr.child_by_local(local) else {
        return Value::Null;
    };
    match el.attr_local("val") {
        Some(v) => json!(v),
        None => Value::Null,
    }
}

fn list_items(pr: Option<&Element>) -> Vec<Value> {
    let Some(pr) = pr else {
        return Vec::new();
    };
    let list = pr
        .child_by_local("dropDownList")
        .or_else(|| pr.child_by_local("comboBox"));
    let Some(list) = list else {
        return Vec::new();
    };
    list.children_by_local("listItem")
        .into_iter()
        .map(|it| {
            json!({
                "display": it.attr_local("displayText"),
                "value": it.attr_local("value"),
            })
        })
        .collect()
}

fn date_full(pr: Option<&Element>) -> Value {
    let Some(pr) = pr else {
        return Value::Null;
    };
    let Some(date) = pr.child_by_local("date") else {
        return Value::Null;
    };
    match date.attr_local("fullDate") {
        Some(v) => json!(v),
        None => Value::Null,
    }
}

fn sdt_text(sdt: &Element) -> String {
    let Some(content) = sdt.child_by_local("sdtContent") else {
        return String::new();
    };
    let mut out = String::new();
    collect_wt(content, &mut out);
    out
}

fn collect_wt(el: &Element, out: &mut String) {
    if el.local_name() == "t" {
        out.push_str(&el.text_content());
        return;
    }
    for c in el.element_children() {
        collect_wt(c, out);
    }
}

fn push_data_binding(pr: &mut Element, binding: Option<(&str, &str, &str)>) {
    let Some((xpath, mappings, store)) = binding else {
        return;
    };
    pr.push_element(
        Element::new(pr.prefixed("dataBinding"))
            .with_attr("w:prefixMappings", mappings)
            .with_attr("w:xpath", xpath)
            .with_attr("w:storeItemID", store),
    );
}

fn simple_binding(sdt: &Element) -> Option<(String, String, String)> {
    let binding = sdt.child_by_local("sdtPr")?.child_by_local("dataBinding")?;
    let store = binding.attr_local("storeItemID")?.to_string();
    let xpath = binding.attr_local("xpath")?.to_string();
    let mappings = binding
        .attr_local("prefixMappings")
        .unwrap_or("")
        .to_string();
    if store.is_empty() || !xpath_is_simple(&xpath) {
        return None;
    }
    Some((store, xpath, mappings))
}

fn xpath_is_simple(xpath: &str) -> bool {
    xpath.starts_with('/')
        && !xpath.contains("//")
        && !xpath.contains('[')
        && !xpath.contains('@')
        && !xpath.contains('(')
}

fn pr_has_binding(sdt: &Element) -> bool {
    sdt.child_by_local("sdtPr")
        .is_some_and(|p| p.child_by_local("dataBinding").is_some())
}

fn content_has_tbl(sdt: &Element) -> bool {
    sdt.child_by_local("sdtContent").is_some_and(walk_has_tbl)
}

fn walk_has_tbl(root: &Element) -> bool {
    let mut stack = vec![root];
    while let Some(el) = stack.pop() {
        if el.local_name() == "tbl" {
            return true;
        }
        for node in el.children.iter().rev() {
            if let Node::Element(child) = node {
                stack.push(child);
            }
        }
    }
    false
}

fn clear_showing_plchdr(sdt: &mut Element) {
    let Some(pr) = sdt.child_by_local_mut("sdtPr") else {
        return;
    };
    pr.children.retain(|n| match n {
        Node::Element(e) => e.local_name() != "showingPlcHdr",
        Node::Text(_) | Node::Raw(_) => true,
    });
}

fn apply_pr_meta(sdt: &mut Element, tag: Option<&str>, alias: Option<&str>) {
    ensure_pr(sdt);
    let Some(pr) = sdt.child_by_local_mut("sdtPr") else {
        return;
    };
    if let Some(v) = tag {
        set_or_remove_val(pr, "tag", v);
    }
    if let Some(v) = alias {
        set_or_remove_val(pr, "alias", v);
    }
}

fn ensure_pr(sdt: &mut Element) {
    if sdt.child_by_local("sdtPr").is_some() {
        return;
    }
    let pr = Element::new(sdt.prefixed("sdtPr"));
    sdt.children.insert(0, Node::Element(pr));
}

fn set_or_remove_val(pr: &mut Element, local: &str, val: &str) {
    if val.is_empty() {
        pr.children.retain(|n| match n {
            Node::Element(e) => e.local_name() != local,
            Node::Text(_) | Node::Raw(_) => true,
        });
        return;
    }
    if let Some(el) = pr.child_by_local_mut(local) {
        el.set_attr("w:val", val);
        return;
    }
    pr.push_element(val_el(pr, local, val));
}

fn replace_items(sdt: &mut Element, kind: SdtKind, items: &[String]) {
    let Some(pr) = sdt.child_by_local_mut("sdtPr") else {
        return;
    };
    let local = kind.type_local();
    if let Some(list) = pr.child_by_local_mut(local) {
        list.children.retain(|n| match n {
            Node::Element(e) => e.local_name() != "listItem",
            Node::Text(_) | Node::Raw(_) => true,
        });
        for it in items {
            let item = list_item_el(list, it);
            list.push_element(item);
        }
    }
}

fn set_full_date(sdt: &mut Element, full_date: &str) {
    let Some(pr) = sdt.child_by_local_mut("sdtPr") else {
        return;
    };
    if let Some(date) = pr.child_by_local_mut("date") {
        date.set_attr("w:fullDate", full_date);
    }
}

fn replace_content(sdt: &mut Element, level: SdtLevel, text: &str) {
    ensure_content(sdt);
    match level {
        SdtLevel::Inline => {
            let Some(content) = sdt.child_by_local_mut("sdtContent") else {
                return;
            };
            let run = para::new_text_run(content, text);
            content.children.clear();
            content.push_element(run);
        }
        SdtLevel::Block => {
            let keep_rest = content_has_tbl(sdt);
            let first_p = first_content_para_index(sdt);
            {
                let Some(content) = sdt.child_by_local_mut("sdtContent") else {
                    return;
                };
                if let Some(idx) = first_p {
                    if let Some(Node::Element(p)) = content.children.get_mut(idx) {
                        para::set_visible_text(p, text);
                    }
                    if !keep_rest {
                        let mut drop = Vec::new();
                        for (i, n) in content.children.iter().enumerate() {
                            if i == idx {
                                continue;
                            }
                            if let Node::Element(e) = n {
                                if e.local_name() == "p" {
                                    drop.push(i);
                                }
                            }
                        }
                        for i in drop.into_iter().rev() {
                            content.children.remove(i);
                        }
                    }
                } else {
                    let p = para::new_paragraph(content, "00CC0001", text);
                    content.children.clear();
                    content.push_element(p);
                }
            }
        }
    }
}

fn first_content_para_index(sdt: &Element) -> Option<usize> {
    let content = sdt.child_by_local("sdtContent")?;
    content.children.iter().position(|n| match n {
        Node::Element(e) => e.local_name() == "p",
        Node::Text(_) | Node::Raw(_) => false,
    })
}

fn ensure_content(sdt: &mut Element) {
    if sdt.child_by_local("sdtContent").is_some() {
        return;
    }
    sdt.push_element(Element::new(sdt.prefixed("sdtContent")));
}

fn unwrap_sdt(root: &mut Element, path: &[usize]) -> Result<(), Error> {
    let kids = {
        let sdt = para::element_at_mut(root, path).ok_or_else(|| xml_broken("sdt"))?;
        match sdt.child_by_local_mut("sdtContent") {
            Some(c) => std::mem::take(&mut c.children),
            None => Vec::new(),
        }
    };
    let (parent_path, idx) = split_parent(path)?;
    let parent = para::element_at_mut(root, parent_path).ok_or_else(|| xml_broken("sdt"))?;
    if idx >= parent.children.len() {
        return Err(xml_broken("sdt"));
    }
    parent.children.remove(idx);
    for (off, kid) in kids.into_iter().enumerate() {
        parent.children.insert(idx + off, kid);
    }
    Ok(())
}

fn blocked_by_sect_pr(root: &Element, drop: &[usize]) -> bool {
    let Some(para_path) = enclosing_para(root, drop) else {
        return false;
    };
    let Some(p) = para::element_at(root, &para_path) else {
        return false;
    };
    if !para::para_has_sect_pr(p) {
        return false;
    }
    let leftover = p.children.iter().enumerate().any(|(i, n)| {
        let Node::Element(e) = n else {
            return false;
        };
        if e.local_name() == "pPr" {
            return false;
        }
        let mut cpath = para_path.clone();
        cpath.push(i);
        !path_starts_with(drop, &cpath)
    });
    !leftover
}

fn drop_empty_host_para(root: &mut Element, para_path: &[usize]) -> Result<(), Error> {
    if para_path.is_empty() {
        return Ok(());
    }
    let parent_path = &para_path[..para_path.len() - 1];
    let parent_name = match para::element_at(root, parent_path) {
        Some(p) => p.local_name().to_string(),
        None => return Ok(()),
    };
    if parent_name == "tc" {
        return Ok(());
    }
    {
        let Some(p) = para::element_at(root, para_path) else {
            return Ok(());
        };
        if p.local_name() != "p" {
            return Ok(());
        }
        if para::para_has_sect_pr(p) || para_has_block_content(p) {
            return Ok(());
        }
    }
    remove_at(root, para_path)?;
    if parent_name == "body" {
        ensure_body_has_para(root)?;
    }
    Ok(())
}

fn ensure_body_has_para(root: &mut Element) -> Result<(), Error> {
    let hex = para::next_para_hex(root);
    let body = para::story_body_mut(root).ok_or_else(|| {
        Error::with_path(
            ErrorKind::XmlMissing {
                parent: "document".into(),
                child: "body".into(),
            },
            "document",
        )
    })?;
    if body.element_children().any(|e| e.local_name() == "p") {
        return Ok(());
    }
    let new_p = para::new_paragraph(body, &hex, "");
    para::append_before_body_sect_pr(body, new_p);
    Ok(())
}

fn para_has_block_content(p: &Element) -> bool {
    p.element_children().any(|e| e.local_name() != "pPr")
}

fn enclosing_para(root: &Element, path: &[usize]) -> Option<Vec<usize>> {
    let mut cur = root;
    let mut acc = Vec::new();
    let mut found = None;
    for &i in path {
        acc.push(i);
        match cur.children.get(i) {
            Some(Node::Element(e)) => {
                if e.local_name() == "p" {
                    found = Some(acc.clone());
                }
                cur = e;
            }
            _ => break,
        }
    }
    found
}

fn para_index_map(root: &Element) -> Vec<(*const Element, usize)> {
    para::collect_story_paragraphs(root)
        .into_iter()
        .enumerate()
        .map(|(i, p)| (p as *const Element, i + 1))
        .collect()
}

fn host_para_id(
    root: &Element,
    rec: &SdtRec,
    container: Option<&str>,
    index_of: &[(*const Element, usize)],
) -> Value {
    let p = match rec.level {
        SdtLevel::Inline => enclosing_para_el(root, &rec.path),
        SdtLevel::Block => first_desc_p(root, &rec.path),
    };
    let Some(p) = p else {
        return Value::Null;
    };
    let idx = index_of
        .iter()
        .find(|(ptr, _)| std::ptr::eq(*ptr, p))
        .map(|(_, i)| *i);
    let Some(index) = idx else {
        return Value::Null;
    };
    let (id, _) = match container {
        Some(c) => para::para_stable_id_in(c, p, index),
        None => para::para_stable_id(p, index),
    };
    json!(id)
}

fn enclosing_para_el<'a>(root: &'a Element, path: &[usize]) -> Option<&'a Element> {
    let mut cur = root;
    let mut found = None;
    for &i in path {
        match cur.children.get(i) {
            Some(Node::Element(e)) => {
                if e.local_name() == "p" {
                    found = Some(e);
                }
                cur = e;
            }
            _ => return None,
        }
    }
    found
}

fn first_desc_p<'a>(root: &'a Element, path: &[usize]) -> Option<&'a Element> {
    let sdt = para::element_at(root, path)?;
    let content = sdt.child_by_local("sdtContent")?;
    find_p(content)
}

fn find_p(el: &Element) -> Option<&Element> {
    crate::xml::find_by_local(el, "p")
}

fn first_alt(el: &Element) -> Option<usize> {
    crate::xml::alternate_child(el).unwrap_or(None)
}

fn path_starts_with(path: &[usize], prefix: &[usize]) -> bool {
    path.len() >= prefix.len() && path[..prefix.len()] == *prefix
}

fn remove_at(root: &mut Element, path: &[usize]) -> Result<(), Error> {
    let (parent_path, idx) = split_parent(path)?;
    let parent = para::element_at_mut(root, parent_path).ok_or_else(|| xml_broken("sdt"))?;
    if idx >= parent.children.len() {
        return Err(xml_broken("sdt"));
    }
    parent.children.remove(idx);
    Ok(())
}

fn split_parent(path: &[usize]) -> Result<(&[usize], usize), Error> {
    let (last, prefix) = path.split_last().ok_or_else(|| xml_broken("sdt"))?;
    Ok((prefix, *last))
}

fn xml_broken(detail: &str) -> Error {
    Error::with_path(
        ErrorKind::XmlBroken {
            issue: XmlIssue::Parse,
            detail: Some(detail.into()),
        },
        "document",
    )
}

fn missing_sdt(id: &str, i: usize) -> Error {
    Error::with_path(
        ErrorKind::ContentControlNotFound { id: id.to_string() },
        format!("ops[{i}].id"),
    )
}

fn checkbox_pr(checked: bool) -> Element {
    let mut box_el = Element::new("w14:checkbox");
    let mut ch = Element::new("w14:checked");
    ch.set_attr("w14:val", if checked { "1" } else { "0" });
    box_el.push_element(ch);
    box_el.push_element(checkbox_state("checkedState", "2612"));
    box_el.push_element(checkbox_state("uncheckedState", "2610"));
    box_el
}

fn checkbox_state(local: &str, val: &str) -> Element {
    let mut el = Element::new(format!("w14:{local}"));
    el.set_attr("w14:val", val);
    el.set_attr("w14:font", CHECKBOX_FONT);
    el
}

fn checkbox_run(parent: &Element, checked: bool) -> Element {
    let mut rpr = Element::new(parent.prefixed("rPr"));
    let mut fonts = Element::new(rpr.prefixed("rFonts"));
    fonts.set_attr("w:ascii", CHECKBOX_FONT);
    fonts.set_attr("w:eastAsia", CHECKBOX_FONT);
    fonts.set_attr("w:hAnsi", CHECKBOX_FONT);
    fonts.set_attr("w:hint", "eastAsia");
    rpr.push_element(fonts);
    let mut t = Element::new(parent.prefixed("t"));
    t.children.push(Node::Text(
        if checked {
            CHECKED_MARK
        } else {
            UNCHECKED_MARK
        }
        .into(),
    ));
    let mut r = Element::new(parent.prefixed("r"));
    crate::xml::mark_new(&mut r, "run");
    r.push_element(rpr);
    r.push_element(t);
    r
}

fn checkbox_checked(pr: Option<&Element>) -> bool {
    let Some(pr) = pr else {
        return false;
    };
    let Some(box_el) = pr.child_by_local("checkbox") else {
        return false;
    };
    let Some(ch) = box_el.child_by_local("checked") else {
        return false;
    };
    match ch.attr_local("val") {
        None => true,
        Some(v) => matches!(v, "1" | "true" | "on" | "True" | "TRUE"),
    }
}

fn set_checkbox(sdt: &mut Element, checked: bool, level: SdtLevel) {
    ensure_pr(sdt);
    if let Some(pr) = sdt.child_by_local_mut("sdtPr") {
        if let Some(box_el) = pr.child_by_local_mut("checkbox") {
            if let Some(ch) = box_el.child_by_local_mut("checked") {
                ch.set_attr("w14:val", if checked { "1" } else { "0" });
            } else {
                let mut ch = Element::new("w14:checked");
                ch.set_attr("w14:val", if checked { "1" } else { "0" });
                box_el.children.insert(0, Node::Element(ch));
            }
        } else {
            pr.push_element(checkbox_pr(checked));
        }
    }
    ensure_content(sdt);
    let Some(content) = sdt.child_by_local_mut("sdtContent") else {
        return;
    };
    let run = checkbox_run(content, checked);
    match level {
        SdtLevel::Inline => {
            content.children.clear();
            content.push_element(run);
        }
        SdtLevel::Block => {
            let mut p = Element::new(content.prefixed("p"));
            p.push_element(run);
            content.children.clear();
            content.push_element(p);
        }
    }
}

fn replace_picture_content(sdt: &mut Element, level: SdtLevel, drawing: Element) {
    ensure_content(sdt);
    let Some(content) = sdt.child_by_local_mut("sdtContent") else {
        return;
    };
    let mut run = Element::new(content.prefixed("r"));
    crate::xml::mark_new(&mut run, "run");
    let mut rpr = Element::new(run.prefixed("rPr"));
    rpr.push_element(Element::new(rpr.prefixed("noProof")));
    run.push_element(rpr);
    run.push_element(drawing);
    match level {
        SdtLevel::Inline => {
            content.children.clear();
            content.push_element(run);
        }
        SdtLevel::Block => {
            let mut p = Element::new(content.prefixed("p"));
            p.push_element(run);
            content.children.clear();
            content.push_element(p);
        }
    }
}

fn collect_blip_rids(el: &Element) -> Vec<String> {
    let mut out = Vec::new();
    walk_blips(el, &mut out);
    out
}

fn walk_blips(el: &Element, out: &mut Vec<String>) {
    crate::xml::walk_elements(el, &mut |node| {
        if node.local_name() == "blip" || node.local_name() == "svgBlip" {
            if let Some(v) = node.attr_local("embed") {
                out.push(v.to_string());
            }
        }
    });
}

fn ensure_w14(root: &mut Element) {
    let key = "xmlns:w14";
    if root.attr(key).is_none() {
        root.set_attr(key, W14_NS);
    }
    let mc = "xmlns:mc";
    if root.attr(mc).is_none() {
        root.set_attr(mc, MC_NS);
    }
    let cur = root.attr_local("Ignorable").unwrap_or("").to_string();
    if cur.split_whitespace().any(|t| t == "w14") {
        return;
    }
    let next = if cur.is_empty() {
        "w14".into()
    } else {
        format!("{cur} w14")
    };
    if let Some((_, v)) = root
        .attrs
        .iter_mut()
        .find(|(k, _)| k.rsplit(':').next() == Some("Ignorable"))
    {
        *v = next;
        return;
    }
    root.set_attr("mc:Ignorable", next);
}

use super::picture;
use super::request::DeleteContentControlOp;
use super::request::DeleteRepeatingItemOp;
use super::request::InsertContentControlOp;
use super::request::InsertRepeatingItemOp;
use super::request::InsertSdtPlace;
use super::request::SetContentControlOp;
use crate::opc::{ContentTypes, Package, Relationship, Relationships};
use crate::xml::Document;
use std::collections::HashMap;
// 从 apply 搬来的操作体。
pub(crate) fn refuse_set_text_if_sdt(
    root: &Element,
    para: &para::BodyPara,
    i: usize,
) -> Result<(), Error> {
    if super::sdt::para_blocks_set_text(root, para) {
        return Err(Error::with_path(
            ErrorKind::SetParaTextHasSdt,
            format!("ops[{i}].id"),
        ));
    }
    Ok(())
}
pub(crate) fn apply_insert_sdt(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &InsertContentControlOp,
    i: usize,
) -> Result<(), Error> {
    let part_name = match &op.place {
        InsertSdtPlace::EndOfDocument => doc_part.to_string(),
        InsertSdtPlace::After(target) => {
            super::apply::part_for_para(pkg, catalog, doc_part, target, &op.id, i)?
        }
        InsertSdtPlace::WrapRow(row) => {
            super::tbl::part_for_story(catalog, doc_part, row.story(), &op.id, i)?
        }
    };
    let store_owned = match &op.binding {
        Some(binding) => Some(prepare_binding(
            pkg, docs, dirty, doc_part, binding, &op.text, i,
        )?),
        None => None,
    };
    let binding_ref = match (&op.binding, store_owned.as_deref()) {
        (Some(binding), Some(store)) => Some((
            binding.xpath.as_str(),
            binding.prefix_mappings.as_str(),
            store,
        )),
        _ => None,
    };
    let drawing = if op.kind == super::sdt::SdtKind::Picture {
        let file = op.file.as_deref().ok_or_else(|| {
            Error::with_path(
                ErrorKind::OpRequiresField {
                    op: "insertContentControl".into(),
                    field: "file".into(),
                },
                format!("ops[{i}].file"),
            )
        })?;
        Some(sdt_inline_drawing(pkg, docs, &part_name, file)?)
    } else {
        None
    };
    let write = super::sdt::SdtWrite {
        kind: op.kind,
        text: &op.text,
        tag: op.tag.as_deref(),
        alias: op.alias.as_deref(),
        items: &op.items,
        full_date: op.full_date.as_deref(),
        checked: op.checked,
        drawing,
        binding: binding_ref,
    };
    match &op.place {
        InsertSdtPlace::EndOfDocument => {
            let doc = docs.get_mut(doc_part).unwrap();
            super::sdt::insert_sdt(&mut doc.root, None, write, &op.id, i)?;
            dirty.insert(doc_part.to_string(), ());
        }
        InsertSdtPlace::After(target) => {
            let doc = super::apply::load_part(pkg, docs, &part_name)?;
            let path = super::apply::para_target_path(&doc.root, target).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::InvalidId {
                        reason: crate::error::IdReason::InsertSdtNeedsPara,
                    },
                    format!("ops[{i}].id"),
                )
            })?;
            super::sdt::insert_sdt(&mut doc.root, Some(path.as_slice()), write, &op.id, i)?;
            dirty.insert(part_name, ());
        }
        InsertSdtPlace::WrapRow(row) => {
            let doc = super::apply::load_part(pkg, docs, &part_name)?;
            let path = super::tbl::row_element_path(&doc.root, row, &op.id, i)?;
            super::sdt::wrap_repeating(&mut doc.root, &path, &write, &op.id, i)?;
            dirty.insert(part_name, ());
        }
    }
    Ok(())
}
pub(crate) fn apply_set_sdt(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &SetContentControlOp,
    i: usize,
) -> Result<(), Error> {
    let part_name = super::sdt::part_for_story(catalog, doc_part, &op.target.story, || {
        super::sdt::missing_sdt_err(&op.id, i)
    })?;
    if let Some(file) = &op.file {
        if op.text.is_some() {
            return Err(Error::with_path(
                ErrorKind::ContentControlFieldMismatch,
                format!("ops[{i}].text"),
            ));
        }
        if op.checked.is_some() {
            return Err(Error::with_path(
                ErrorKind::ContentControlFieldMismatch,
                format!("ops[{i}].checked"),
            ));
        }
        if op.items.is_some() {
            return Err(Error::with_path(
                ErrorKind::ContentControlFieldMismatch,
                format!("ops[{i}].items"),
            ));
        }
        if op.full_date.is_some() {
            return Err(Error::with_path(
                ErrorKind::ContentControlFieldMismatch,
                format!("ops[{i}].fullDate"),
            ));
        }
        let old_rids = {
            let doc = super::apply::load_part(pkg, docs, &part_name)?;
            let story = super::apply::story_xml_mut(&mut doc.root, &op.target.story, || {
                super::sdt::missing_sdt_err(&op.id, i)
            })?;
            super::sdt::sdt_picture_rids(story, op.target.n, &op.id, i)?
        };
        let old_media: Vec<String> = old_rids
            .iter()
            .filter_map(|r| picture::resolve_embed(pkg, &part_name, r).ok())
            .collect();
        let drawing = sdt_inline_drawing(pkg, docs, &part_name, file)?;
        let doc = super::apply::load_part(pkg, docs, &part_name)?;
        let story = super::apply::story_xml_mut(&mut doc.root, &op.target.story, || {
            super::sdt::missing_sdt_err(&op.id, i)
        })?;
        super::sdt::set_sdt_picture(
            story,
            op.target.n,
            drawing,
            op.tag.as_deref(),
            op.alias.as_deref(),
            &op.id,
            i,
        )?;
        dirty.insert(part_name.clone(), ());
        for r in &old_rids {
            picture::remove_rel_if_unused(pkg, &part_name, r, &docs.get(&part_name).unwrap().root)?;
        }
        for m in old_media {
            if !picture::media_still_used(pkg, &m)? {
                pkg.remove(&m);
            }
        }
        return Ok(());
    }
    let doc = super::apply::load_part(pkg, docs, &part_name)?;
    let story = super::apply::story_xml_mut(&mut doc.root, &op.target.story, || {
        super::sdt::missing_sdt_err(&op.id, i)
    })?;
    let bound = super::sdt::set_sdt(
        story,
        &super::sdt::SdtSet {
            n: op.target.n,
            text: op.text.as_deref(),
            tag: op.tag.as_deref(),
            alias: op.alias.as_deref(),
            items: op.items.as_deref(),
            full_date: op.full_date.as_deref(),
            checked: op.checked,
            id: &op.id,
            i,
        },
    )?;
    if let Some(bound) = bound {
        write_bound_text(pkg, docs, dirty, &bound, i)?;
    }
    dirty.insert(part_name, ());
    Ok(())
}

pub(crate) fn apply_insert_repeating_item(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &InsertRepeatingItemOp,
    i: usize,
) -> Result<(), Error> {
    let part_name = super::sdt::part_for_story(catalog, doc_part, &op.target.story, || {
        super::sdt::missing_sdt_err(&op.id, i)
    })?;
    let doc = super::apply::load_part(pkg, docs, &part_name)?;
    let story = super::apply::story_xml_mut(&mut doc.root, &op.target.story, || {
        super::sdt::missing_sdt_err(&op.id, i)
    })?;
    super::sdt::insert_repeating_item(story, op.target.n, &op.id, i)?;
    dirty.insert(part_name, ());
    Ok(())
}

pub(crate) fn apply_delete_repeating_item(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &DeleteRepeatingItemOp,
    i: usize,
) -> Result<(), Error> {
    let part_name = super::sdt::part_for_story(catalog, doc_part, &op.target.story, || {
        super::sdt::missing_sdt_err(&op.id, i)
    })?;
    let doc = super::apply::load_part(pkg, docs, &part_name)?;
    let story = super::apply::story_xml_mut(&mut doc.root, &op.target.story, || {
        super::sdt::missing_sdt_err(&op.id, i)
    })?;
    super::sdt::delete_repeating_item(story, op.target.n, &op.id, i)?;
    dirty.insert(part_name, ());
    Ok(())
}
fn prepare_binding(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    binding: &super::request::SdtBinding,
    text: &str,
    i: usize,
) -> Result<String, Error> {
    let store = if let Some(id) = &binding.store_item_id {
        let part = find_store_item(pkg, id).ok_or_else(|| {
            Error::with_path(
                ErrorKind::XmlMissing {
                    parent: "customXml".into(),
                    child: id.clone(),
                },
                format!("ops[{i}].binding.storeItemId"),
            )
        })?;
        load_item(pkg, docs, &part)?;
        id.clone()
    } else {
        let xml = binding.xml.as_deref().ok_or_else(|| {
            Error::with_path(
                ErrorKind::OpRequiresField {
                    op: "insertContentControl".into(),
                    field: "xml".into(),
                },
                format!("ops[{i}].binding.xml"),
            )
        })?;
        create_store(pkg, docs, dirty, doc_part, xml, &binding.prefix_mappings)?
    };
    if !text.is_empty() {
        write_bound_text(
            pkg,
            docs,
            dirty,
            &BoundWrite {
                store_item_id: store.clone(),
                xpath: binding.xpath.clone(),
                prefix_mappings: binding.prefix_mappings.clone(),
                text: text.to_string(),
            },
            i,
        )?;
    }
    Ok(store)
}

fn write_bound_text(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    bound: &BoundWrite,
    i: usize,
) -> Result<(), Error> {
    let part = find_store_item(pkg, &bound.store_item_id).ok_or_else(|| {
        Error::with_path(
            ErrorKind::XmlMissing {
                parent: "customXml".into(),
                child: bound.store_item_id.clone(),
            },
            format!("ops[{i}].id"),
        )
    })?;
    load_item(pkg, docs, &part)?;
    let doc = docs.get_mut(&part).unwrap();
    set_xpath_text(
        &mut doc.root,
        &bound.xpath,
        &bound.prefix_mappings,
        &bound.text,
    )
    .map_err(|e| e.at(format!("ops[{i}].text")))?;
    dirty.insert(part, ());
    Ok(())
}

fn load_item(pkg: &Package, docs: &mut HashMap<String, Document>, part: &str) -> Result<(), Error> {
    if docs.contains_key(part) {
        return Ok(());
    }
    let parsed = xml::parse(pkg.get(part)?)?;
    docs.insert(part.to_string(), parsed);
    Ok(())
}

fn find_store_item(pkg: &Package, store_id: &str) -> Option<String> {
    let want = store_id.trim().to_ascii_lowercase();
    for name in pkg.part_names() {
        let Some(file) = name.strip_prefix("/customXml/itemProps") else {
            continue;
        };
        let Some(n) = file.strip_suffix(".xml") else {
            continue;
        };
        if !n.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        let Ok(bytes) = pkg.get(&name) else {
            continue;
        };
        let Ok(doc) = xml::parse(bytes) else {
            continue;
        };
        let id = doc.root.attr_local("itemID").unwrap_or("");
        if id.trim().eq_ignore_ascii_case(&want) {
            return Some(format!("/customXml/item{n}.xml"));
        }
    }
    None
}

fn create_store(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    xml_text: &str,
    mappings: &str,
) -> Result<String, Error> {
    let n = next_custom_item(pkg);
    let item = format!("/customXml/item{n}.xml");
    let props = format!("/customXml/itemProps{n}.xml");
    let store = crate::host_env::random_guid();
    let parsed = xml::parse(xml_text.as_bytes())?;
    pkg.set(&item, parsed.to_bytes()?)?;
    docs.insert(item.clone(), parsed);
    dirty.insert(item.clone(), ());
    let uri = schema_uri(mappings);
    pkg.set(&props, item_props_xml(&store, &uri).into_bytes())?;
    let mut item_rels = Relationships { rels: Vec::new() };
    item_rels.push(Relationship {
        id: item_rels.next_rid(),
        rel_type: super::biblio::REL_CUSTOM_XML_PROPS.into(),
        target: picture::rel_target_from(&item, &props),
        target_mode: None,
        extra: Vec::new(),
    });
    item_rels.write_to(pkg, &item)?;
    let mut doc_rels = Relationships::from_package(pkg, doc_part)?;
    doc_rels.push(Relationship {
        id: doc_rels.next_rid(),
        rel_type: super::biblio::REL_CUSTOM_XML.into(),
        target: picture::rel_target_from(doc_part, &item),
        target_mode: None,
        extra: Vec::new(),
    });
    doc_rels.write_to(pkg, doc_part)?;
    let mut cts = ContentTypes::from_package(pkg)?;
    cts.set_override(&props, super::biblio::CT_CUSTOM_XML_PROPS)?;
    cts.write_to(pkg)?;
    Ok(store)
}

fn next_custom_item(pkg: &Package) -> u32 {
    let mut max = 0u32;
    for name in pkg.part_names() {
        let Some(file) = name.strip_prefix("/customXml/item") else {
            continue;
        };
        if file.starts_with("Props") {
            continue;
        }
        let Some(n) = file.strip_suffix(".xml") else {
            continue;
        };
        if let Ok(v) = n.parse::<u32>() {
            max = max.max(v);
        }
    }
    max + 1
}

fn schema_uri(mappings: &str) -> String {
    parse_prefix_mappings(mappings)
        .into_iter()
        .next()
        .map(|(_, uri)| uri)
        .unwrap_or_default()
}

fn item_props_xml(store: &str, uri: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="no"?><ds:datastoreItem ds:itemID="{store}" xmlns:ds="http://schemas.openxmlformats.org/officeDocument/2006/customXml"><ds:schemaRefs><ds:schemaRef ds:uri="{uri}"/></ds:schemaRefs></ds:datastoreItem>"#
    )
}

fn set_xpath_text(
    root: &mut Element,
    xpath: &str,
    mappings: &str,
    text: &str,
) -> Result<(), Error> {
    let prefixes = parse_prefix_mappings(mappings);
    let steps = xpath_steps(xpath)?;
    let mut path = Vec::new();
    {
        let mut current = &*root;
        let mut inherited = String::new();
        for (i, (prefix, local)) in steps.iter().enumerate() {
            let want_ns = prefix.as_ref().and_then(|p| {
                prefixes
                    .iter()
                    .find(|(name, _)| name == p)
                    .map(|(_, uri)| uri.as_str())
            });
            if i == 0 {
                if !element_matches(current, local, want_ns, &inherited) {
                    return Err(Error::new(ErrorKind::XmlMissing {
                        parent: "customXml".into(),
                        child: xpath.to_string(),
                    }));
                }
                inherited = element_default_ns(current, &inherited);
                continue;
            }
            let idx = current.children.iter().position(
                |n| matches!(n, Node::Element(e) if element_matches(e, local, want_ns, &inherited)),
            );
            let Some(idx) = idx else {
                return Err(Error::new(ErrorKind::XmlMissing {
                    parent: "customXml".into(),
                    child: xpath.to_string(),
                }));
            };
            let Node::Element(child) = &current.children[idx] else {
                return Err(Error::new(ErrorKind::XmlMissing {
                    parent: "customXml".into(),
                    child: xpath.to_string(),
                }));
            };
            inherited = element_default_ns(child, &inherited);
            current = child;
            path.push(idx);
        }
        if current.element_children().next().is_some() {
            return Err(Error::new(ErrorKind::ContentControlBound));
        }
    }
    let mut current = root;
    for idx in path {
        let Node::Element(child) = &mut current.children[idx] else {
            return Err(Error::new(ErrorKind::XmlMissing {
                parent: "customXml".into(),
                child: xpath.to_string(),
            }));
        };
        current = child;
    }
    current.set_text(text);
    Ok(())
}

fn xpath_steps(xpath: &str) -> Result<Vec<(Option<String>, String)>, Error> {
    if !xpath_is_simple(xpath) {
        return Err(Error::new(ErrorKind::ContentControlBound));
    }
    let mut out = Vec::new();
    for step in xpath.split('/').filter(|s| !s.is_empty()) {
        if let Some((prefix, local)) = step.split_once(':') {
            out.push((Some(prefix.to_string()), local.to_string()));
        } else {
            out.push((None, step.to_string()));
        }
    }
    if out.is_empty() {
        return Err(Error::new(ErrorKind::ContentControlBound));
    }
    Ok(out)
}

fn parse_prefix_mappings(mappings: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut rest = mappings.trim();
    while let Some(body) = rest.strip_prefix("xmlns:") {
        let Some((prefix, after)) = body.split_once('=') else {
            break;
        };
        let after = after.trim_start();
        let Some(quote) = after.chars().next() else {
            break;
        };
        if quote != '\'' && quote != '"' {
            break;
        }
        let Some((uri, tail)) = after[quote.len_utf8()..].split_once(quote) else {
            break;
        };
        out.push((prefix.trim().to_string(), uri.to_string()));
        rest = tail.trim_start();
    }
    out
}

fn element_default_ns(el: &Element, inherited: &str) -> String {
    if let Some(v) = el.attr("xmlns") {
        return v.to_string();
    }
    inherited.to_string()
}

fn element_matches(el: &Element, local: &str, want_ns: Option<&str>, inherited: &str) -> bool {
    if el.local_name() != local {
        return false;
    }
    let Some(want) = want_ns else {
        return true;
    };
    let got = if let Some((prefix, _)) = el.name.split_once(':') {
        el.attr(&format!("xmlns:{prefix}")).unwrap_or(inherited)
    } else {
        el.attr("xmlns").unwrap_or(inherited)
    };
    got == want
}

pub fn insert_repeating_item(
    root: &mut Element,
    n: usize,
    id: &str,
    i: usize,
) -> Result<(), Error> {
    let rec = sdt_at(root, n, id, i)?;
    let path = rec.path.clone();
    let role = {
        let sdt = para::element_at(root, &path).ok_or_else(|| missing_sdt(id, i))?;
        repeating_role(sdt)
    };
    let source_path = match role {
        Some(RepeatingRole::Item) => path,
        Some(RepeatingRole::Section) => {
            let sdt = para::element_at(root, &path).ok_or_else(|| missing_sdt(id, i))?;
            last_item_path(sdt, &path).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::XmlMissing {
                        parent: "sdt".into(),
                        child: "repeatingSectionItem".into(),
                    },
                    format!("ops[{i}].id"),
                )
            })?
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::ContentControlRepeating,
                format!("ops[{i}].id"),
            ));
        }
    };
    let mut clone = {
        let sdt = para::element_at(root, &source_path).ok_or_else(|| missing_sdt(id, i))?;
        sdt.clone()
    };
    freshen_clone(root, &mut clone);
    let (parent, idx) =
        para::parent_and_index_mut(root, &source_path).ok_or_else(|| missing_sdt(id, i))?;
    parent.children.insert(idx + 1, Node::Element(clone));
    Ok(())
}

pub fn delete_repeating_item(
    root: &mut Element,
    n: usize,
    id: &str,
    i: usize,
) -> Result<(), Error> {
    let rec = sdt_at(root, n, id, i)?;
    let path = rec.path;
    let sdt = para::element_at(root, &path).ok_or_else(|| missing_sdt(id, i))?;
    if repeating_role(sdt) != Some(RepeatingRole::Item) {
        return Err(Error::with_path(
            ErrorKind::ContentControlRepeating,
            format!("ops[{i}].id"),
        ));
    }
    let count = {
        let (parent, _) =
            para::parent_and_index_mut(root, &path).ok_or_else(|| missing_sdt(id, i))?;
        parent
            .element_children()
            .filter(|e| repeating_role(e) == Some(RepeatingRole::Item))
            .count()
    };
    if count <= 1 {
        return Err(Error::with_path(
            ErrorKind::LastRepeatingItem,
            format!("ops[{i}].id"),
        ));
    }
    let (parent, idx) =
        para::parent_and_index_mut(root, &path).ok_or_else(|| missing_sdt(id, i))?;
    parent.children.remove(idx);
    Ok(())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RepeatingRole {
    Section,
    Item,
}

fn repeating_role(sdt: &Element) -> Option<RepeatingRole> {
    let pr = sdt.child_by_local("sdtPr")?;
    for child in pr.element_children() {
        match child.local_name() {
            "repeatingSection" => return Some(RepeatingRole::Section),
            "repeatingSectionItem" => return Some(RepeatingRole::Item),
            _ => {}
        }
    }
    None
}

fn last_item_path(section: &Element, section_path: &[usize]) -> Option<Vec<usize>> {
    let content = section.child_by_local("sdtContent")?;
    let content_idx = section
        .children
        .iter()
        .position(|n| matches!(n, Node::Element(e) if e.local_name() == "sdtContent"))?;
    let mut last = None;
    for (i, child) in content.children.iter().enumerate() {
        if let Node::Element(e) = child {
            if repeating_role(e) == Some(RepeatingRole::Item) {
                last = Some(i);
            }
        }
    }
    let i = last?;
    let mut path = section_path.to_vec();
    path.push(content_idx);
    path.push(i);
    Some(path)
}

fn freshen_clone(root: &Element, clone: &mut Element) {
    let mut used = Vec::new();
    collect_xml_ids(root, &mut used);
    freshen_walk(clone, &mut used);
}

fn freshen_walk(el: &mut Element, used: &mut Vec<i64>) {
    if el.local_name() == "sdt" {
        crate::xml::mark_new(el, "sdt");
        if let Some(pr) = el.child_by_local_mut("sdtPr") {
            if let Some(id) = pr.child_by_local_mut("id") {
                let next = next_free_id(used);
                id.set_attr("w:val", next.to_string());
                used.push(next);
            }
        }
    }
    if el.local_name() == "p" && el.attr_local("paraId").is_some() {
        let hex = format!("{:08X}", next_free_id(used) as u32);
        if let Some((_, v)) = el
            .attrs
            .iter_mut()
            .find(|(k, _)| k.rsplit(':').next() == Some("paraId"))
        {
            *v = hex;
        }
    }
    for child in &mut el.children {
        if let Node::Element(inner) = child {
            freshen_walk(inner, used);
        }
    }
}

fn next_free_id(used: &[i64]) -> i64 {
    let mut n = 1i64;
    while used.contains(&n) {
        n += 1;
    }
    n
}

pub(crate) fn sdt_inline_drawing(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    part_name: &str,
    file: &str,
) -> Result<Element, Error> {
    let added = picture::add_media(pkg, part_name, file, None, None)?;
    let doc = super::apply::load_part(pkg, docs, part_name)?;
    picture::ensure_drawing_ns(&mut doc.root);
    let doc_pr = picture::next_docpr_id(&doc.root);
    let name = format!("图片 {doc_pr}");
    Ok(picture::build_inline_drawing(
        doc_pr,
        &name,
        &added.blip_rid,
        added.svg_rid.as_deref(),
        added.cx,
        added.cy,
    ))
}
pub(crate) fn apply_delete_sdt(
    pkg: &Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &DeleteContentControlOp,
    i: usize,
) -> Result<(), Error> {
    let part_name = super::sdt::part_for_story(catalog, doc_part, &op.target.story, || {
        super::sdt::missing_sdt_err(&op.id, i)
    })?;
    let doc = super::apply::load_part(pkg, docs, &part_name)?;
    let story = super::apply::story_xml_mut(&mut doc.root, &op.target.story, || {
        super::sdt::missing_sdt_err(&op.id, i)
    })?;
    super::sdt::delete_sdt(story, op.target.n, op.keep, &op.id, i)?;
    dirty.insert(part_name, ());
    Ok(())
}
