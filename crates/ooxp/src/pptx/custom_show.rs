//! 自定义放映：`p:custShowLst`。不算放映引擎，不写时间轴，不改各页 XML。

use serde_json::{json, Map, Value};

use crate::error::{Error, ErrorKind, IdReason, TypeName};
use crate::opc::Package;
use crate::xml::{Document, Element, Node};

use super::picture;
use super::request::{DeleteCustomShowOp, InsertCustomShowOp, SetCustomShowOp};
use super::shape;
use super::REL_NS;

pub const NAME_MAX_CHARS: usize = 31;

pub struct ShowInfo {
    pub index: usize,
    pub name: String,
    pub rids: Vec<String>,
}

pub fn count(root: &Element) -> usize {
    cust_show_lst(root)
        .map(|lst| {
            lst.element_children()
                .filter(|e| e.local_name() == "custShow")
                .count()
        })
        .unwrap_or(0)
}

pub fn collect(root: &Element) -> Vec<ShowInfo> {
    let Some(lst) = cust_show_lst(root) else {
        return Vec::new();
    };
    lst.element_children()
        .filter(|e| e.local_name() == "custShow")
        .enumerate()
        .map(|(i, show)| ShowInfo {
            index: i + 1,
            name: show.attr("name").unwrap_or("").to_string(),
            rids: show_rids(show),
        })
        .collect()
}

pub fn list_json(root: &Element) -> Vec<Value> {
    collect(root)
        .into_iter()
        .map(|s| {
            let slides: Vec<Value> = s
                .rids
                .iter()
                .filter_map(|rid| slide_n_of_rid(root, rid))
                .map(|n| json!(format!("slide:{n}")))
                .collect();
            let mut obj = Map::new();
            obj.insert("id".into(), json!(format!("show:{}", s.index)));
            obj.insert("name".into(), json!(s.name));
            obj.insert("slides".into(), Value::Array(slides));
            Value::Object(obj)
        })
        .collect()
}

pub fn parse_show_id(id: &str, json_path: &str) -> Result<usize, Error> {
    let Some(rest) = id.strip_prefix("show:") else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::CustomShowPrefix,
            },
            json_path,
        ));
    };
    if rest.is_empty() || rest.contains('/') {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::CustomShowPrefix,
            },
            json_path,
        ));
    }
    let n: usize = rest.parse().map_err(|_| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::CustomShowIndexNotInt,
            },
            json_path,
        )
    })?;
    if n < 1 {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::CustomShowIndexFromOne,
            },
            json_path,
        ));
    }
    Ok(n)
}

pub fn check_name(name: &str, json_path: &str) -> Result<(), Error> {
    if name.is_empty() {
        return Err(Error::with_path(
            ErrorKind::WrongType {
                field: "name".into(),
                expected: TypeName::NonEmptyString,
            },
            json_path,
        ));
    }
    if name.chars().count() > NAME_MAX_CHARS {
        return Err(Error::with_path(
            ErrorKind::CustomShowNameTooLong,
            json_path,
        ));
    }
    Ok(())
}

pub fn insert(
    pkg: &mut Package,
    pres_part: &str,
    pres: &mut Document,
    op: &InsertCustomShowOp,
    op_index: usize,
) -> Result<(), Error> {
    picture::ensure_xmlns(&mut pres.root, "r", REL_NS);
    if let Some(after) = op.after {
        ensure_show_index(&pres.root, after, op_index, "after")?;
    }
    let rids = resolve_slide_rids(&pres.root, &op.slides, op_index, "slides")?;
    reject_taken_name(&pres.root, &op.name, None, op_index, "name")?;
    let xml_id = next_xml_id(&pres.root);
    let el = new_show(&op.name, xml_id, &rids);
    let insert_at = op.after.unwrap_or_else(|| count(&pres.root));
    let lst = ensure_cust_show_lst(&mut pres.root);
    insert_show_at(lst, insert_at, el);
    pkg.set(pres_part, pres.to_bytes()?)?;
    Ok(())
}

pub fn set(
    pkg: &mut Package,
    pres_part: &str,
    pres: &mut Document,
    op: &SetCustomShowOp,
    op_index: usize,
) -> Result<(), Error> {
    picture::ensure_xmlns(&mut pres.root, "r", REL_NS);
    let idx = ensure_show_index(&pres.root, op.show, op_index, "id")?;
    if let Some(name) = &op.name {
        reject_taken_name(&pres.root, name, Some(op.show), op_index, "name")?;
    }
    let rids = if let Some(slides) = &op.slides {
        Some(resolve_slide_rids(&pres.root, slides, op_index, "slides")?)
    } else {
        None
    };
    let lst = cust_show_lst_mut(&mut pres.root).expect("刚确认有自定义放映名单");
    let show = show_at_mut(lst, idx).expect("刚确认自定义放映下标");
    if let Some(name) = &op.name {
        show.set_attr("name", name.clone());
    }
    if let Some(rids) = rids {
        replace_sld_lst(show, &rids);
    }
    pkg.set(pres_part, pres.to_bytes()?)?;
    Ok(())
}

pub fn delete(
    pkg: &mut Package,
    pres_part: &str,
    pres: &mut Document,
    op: &DeleteCustomShowOp,
    op_index: usize,
) -> Result<(), Error> {
    let idx = ensure_show_index(&pres.root, op.show, op_index, "id")?;
    let lst = cust_show_lst_mut(&mut pres.root).expect("刚确认有自定义放映名单");
    remove_show_at(lst, idx);
    if count(&pres.root) == 0 {
        drop_cust_show_lst(&mut pres.root);
    }
    pkg.set(pres_part, pres.to_bytes()?)?;
    Ok(())
}

/// 删页时按被删 `sldId` 的关系 id 摘掉自定义放映里对应 `p:sld`。
pub fn remove_rid(root: &mut Element, rid: &str) {
    let Some(lst) = cust_show_lst_mut(root) else {
        return;
    };
    for child in lst.element_children_mut() {
        if child.local_name() != "custShow" {
            continue;
        }
        let Some(sld_lst) = child.child_by_local_mut("sldLst") else {
            continue;
        };
        sld_lst.children.retain(|n| match n {
            Node::Element(e) => !(e.local_name() == "sld" && shape::rid_attr(e) == Some(rid)),
            _ => true,
        });
    }
}

fn reject_taken_name(
    root: &Element,
    name: &str,
    skip: Option<usize>,
    op_index: usize,
    field: &str,
) -> Result<(), Error> {
    for s in collect(root) {
        if skip == Some(s.index) {
            continue;
        }
        if s.name == name {
            return Err(Error::with_path(
                ErrorKind::CustomShowNameTaken,
                format!("ops[{op_index}].{field}"),
            ));
        }
    }
    Ok(())
}

fn resolve_slide_rids(
    root: &Element,
    slides: &[usize],
    op_index: usize,
    field: &str,
) -> Result<Vec<String>, Error> {
    let known = presentation_rids(root);
    let mut out = Vec::new();
    for (j, &n) in slides.iter().enumerate() {
        let rid = known.get(n - 1).ok_or_else(|| {
            Error::with_path(
                ErrorKind::SlideNotFound { n: n as u32 },
                format!("ops[{op_index}].{field}[{j}]"),
            )
        })?;
        out.push(rid.clone());
    }
    Ok(out)
}

fn ensure_show_index(
    root: &Element,
    n: usize,
    op_index: usize,
    field: &str,
) -> Result<usize, Error> {
    let total = count(root);
    if n < 1 || n > total {
        return Err(Error::with_path(
            ErrorKind::CustomShowNotFound {
                id: format!("show:{n}"),
            },
            format!("ops[{op_index}].{field}"),
        ));
    }
    Ok(n - 1)
}

fn next_xml_id(root: &Element) -> u32 {
    let Some(lst) = cust_show_lst(root) else {
        return 0;
    };
    lst.element_children()
        .filter(|e| e.local_name() == "custShow")
        .filter_map(|e| e.attr("id")?.parse::<u32>().ok())
        .max()
        .map(|n| n.saturating_add(1))
        .unwrap_or(0)
}

fn new_show(name: &str, xml_id: u32, rids: &[String]) -> Element {
    let mut sld_lst = Element::new("p:sldLst");
    for rid in rids {
        sld_lst.push_element(sld_el(rid));
    }
    let mut show = Element::new("p:custShow")
        .with_attr("name", name)
        .with_attr("id", xml_id.to_string());
    show.push_element(sld_lst);
    show
}

fn sld_el(rid: &str) -> Element {
    Element::new("p:sld").with_attr("r:id", rid)
}

fn replace_sld_lst(show: &mut Element, rids: &[String]) {
    let tag = show
        .child_by_local("sldLst")
        .map(|e| e.name.clone())
        .unwrap_or_else(|| "p:sldLst".into());
    let sld_tag = show
        .child_by_local("sldLst")
        .and_then(|lst| {
            lst.element_children()
                .find(|e| e.local_name() == "sld")
                .map(|e| e.name.clone())
        })
        .unwrap_or_else(|| "p:sld".into());
    let rid_key = show
        .child_by_local("sldLst")
        .and_then(|lst| {
            lst.element_children()
                .find(|e| e.local_name() == "sld")
                .and_then(|e| {
                    e.attrs
                        .iter()
                        .find(|(k, _)| k.contains(':') && k.rsplit(':').next() == Some("id"))
                        .map(|(k, _)| k.clone())
                })
        })
        .unwrap_or_else(|| "r:id".into());
    show.children.retain(|n| match n {
        Node::Element(e) => e.local_name() != "sldLst",
        _ => true,
    });
    let mut lst = Element::new(tag);
    for rid in rids {
        lst.push_element(Element::new(sld_tag.clone()).with_attr(rid_key.clone(), rid));
    }
    let ext_i = show
        .children
        .iter()
        .position(|n| matches!(n, Node::Element(e) if e.local_name() == "extLst"));
    if let Some(i) = ext_i {
        show.children.insert(i, Node::Element(lst));
    } else {
        show.children.push(Node::Element(lst));
    }
}

fn show_rids(show: &Element) -> Vec<String> {
    let Some(lst) = show.child_by_local("sldLst") else {
        return Vec::new();
    };
    lst.element_children()
        .filter(|e| e.local_name() == "sld")
        .filter_map(|e| shape::rid_attr(e).map(str::to_string))
        .collect()
}

fn presentation_rids(root: &Element) -> Vec<String> {
    let Some(lst) = root.child_by_local("sldIdLst") else {
        return Vec::new();
    };
    lst.element_children()
        .filter(|e| e.local_name() == "sldId")
        .filter_map(|e| shape::rid_attr(e).map(str::to_string))
        .collect()
}

fn slide_n_of_rid(root: &Element, rid: &str) -> Option<usize> {
    presentation_rids(root)
        .iter()
        .position(|r| r == rid)
        .map(|i| i + 1)
}

fn cust_show_lst(root: &Element) -> Option<&Element> {
    root.child_by_local("custShowLst")
}

fn cust_show_lst_mut(root: &mut Element) -> Option<&mut Element> {
    root.child_by_local_mut("custShowLst")
}

fn ensure_cust_show_lst(root: &mut Element) -> &mut Element {
    if cust_show_lst(root).is_some() {
        return cust_show_lst_mut(root).expect("刚确认有自定义放映名单");
    }
    let at = cust_show_lst_insert_index(root);
    root.children
        .insert(at, Node::Element(Element::new("p:custShowLst")));
    cust_show_lst_mut(root).expect("刚插入 custShowLst")
}

fn cust_show_lst_insert_index(root: &Element) -> usize {
    const AFTER: &[&str] = &["notesSz", "smartTags", "embeddedFontLst"];
    const BEFORE: &[&str] = &[
        "photoAlbum",
        "custDataLst",
        "kinsoku",
        "defaultTextStyle",
        "modifyVerifier",
        "extLst",
    ];
    let mut last_after = None;
    let mut first_before = None;
    for (i, n) in root.children.iter().enumerate() {
        let Node::Element(e) = n else {
            continue;
        };
        let name = e.local_name();
        if AFTER.contains(&name) {
            last_after = Some(i);
        }
        if first_before.is_none() && BEFORE.contains(&name) {
            first_before = Some(i);
        }
    }
    if let Some(i) = last_after {
        return i + 1;
    }
    if let Some(i) = first_before {
        return i;
    }
    root.children.len()
}

fn drop_cust_show_lst(root: &mut Element) {
    root.children
        .retain(|n| !matches!(n, Node::Element(e) if e.local_name() == "custShowLst"));
}

fn show_indices(lst: &Element) -> Vec<usize> {
    lst.children
        .iter()
        .enumerate()
        .filter_map(|(i, n)| match n {
            Node::Element(e) if e.local_name() == "custShow" => Some(i),
            _ => None,
        })
        .collect()
}

fn show_at_mut(lst: &mut Element, nth: usize) -> Option<&mut Element> {
    let i = *show_indices(lst).get(nth)?;
    match &mut lst.children[i] {
        Node::Element(e) => Some(e),
        _ => None,
    }
}

fn insert_show_at(lst: &mut Element, nth: usize, el: Element) {
    let mut seen = 0usize;
    for (i, n) in lst.children.iter().enumerate() {
        let Node::Element(e) = n else {
            continue;
        };
        if e.local_name() != "custShow" {
            continue;
        }
        if seen == nth {
            lst.children.insert(i, Node::Element(el));
            return;
        }
        seen += 1;
    }
    lst.children.push(Node::Element(el));
}

fn remove_show_at(lst: &mut Element, nth: usize) {
    if let Some(&i) = show_indices(lst).get(nth) {
        lst.children.remove(i);
    }
}
