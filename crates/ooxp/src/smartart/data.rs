//! data 部件：可见点、连线、改字。不含宿主 graphicFrame。

use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::error::Error;
use crate::opc::Package;
use crate::xml::{self, Element, Node};

use super::{find_local, find_local_mut, LayoutKind, NewPoint, Role, A_NS, DGM_NS, REL_NS};

static GUID_SEQ: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Default)]
pub struct RelIds {
    pub dm: Option<String>,
    pub lo: Option<String>,
    pub qs: Option<String>,
    pub cs: Option<String>,
}

impl RelIds {
    pub fn get(&self, local: &str) -> Option<&String> {
        match local {
            "dm" => self.dm.as_ref(),
            "lo" => self.lo.as_ref(),
            "qs" => self.qs.as_ref(),
            "cs" => self.cs.as_ref(),
            _ => None,
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = Option<&String>> {
        [
            self.dm.as_ref(),
            self.lo.as_ref(),
            self.qs.as_ref(),
            self.cs.as_ref(),
        ]
        .into_iter()
    }
}

struct ResolvedPt {
    model_id: String,
    parent: Option<String>,
    role: Role,
    text: String,
}

fn resolve_points(layout: LayoutKind, points: Option<&[NewPoint]>) -> Vec<ResolvedPt> {
    if let Some(pts) = points {
        return pts
            .iter()
            .enumerate()
            .map(|(i, p)| ResolvedPt {
                model_id: (i + 1).to_string(),
                parent: p.parent.map(|j| (j + 1).to_string()),
                role: p.role,
                text: p.text.clone(),
            })
            .collect();
    }
    match layout {
        LayoutKind::List | LayoutKind::Cycle => (1..=3)
            .map(|i| ResolvedPt {
                model_id: i.to_string(),
                parent: None,
                role: Role::Node,
                text: format!("文本 {i}"),
            })
            .collect(),
        LayoutKind::Org => vec![
            ResolvedPt {
                model_id: "1".into(),
                parent: None,
                role: Role::Node,
                text: "文本 1".into(),
            },
            ResolvedPt {
                model_id: "2".into(),
                parent: Some("1".into()),
                role: Role::Node,
                text: "文本 2".into(),
            },
            ResolvedPt {
                model_id: "3".into(),
                parent: Some("1".into()),
                role: Role::Node,
                text: "文本 3".into(),
            },
        ],
    }
}

pub fn build_data_xml(layout: LayoutKind, points: Option<&[NewPoint]>) -> String {
    let points = resolve_points(layout, points);
    let mut pts = String::from(r#"<dgm:pt type="doc" modelId="0"/>"#);
    for p in &points {
        let ty = match p.role {
            Role::Node => "",
            Role::Asst => r#" type="asst""#,
        };
        pts.push_str(&format!(
            r#"<dgm:pt{ty} modelId="{id}"><dgm:prSet/><dgm:t><a:bodyPr/><a:lstStyle/><a:p><a:r><a:rPr lang="zh-CN" sz="1100"/><a:t{space}>{text}</a:t></a:r></a:p></dgm:t></dgm:pt>"#,
            id = xml::escape_attr(&p.model_id),
            space = if needs_xml_space(&p.text) {
                r#" xml:space="preserve""#
            } else {
                ""
            },
            text = xml::escape_text(&p.text),
        ));
    }
    let mut cxns = String::new();
    let mut src_ord: HashMap<String, u32> = HashMap::new();
    for (cxn_id, p) in ((points.len() as u64 + 1)..).zip(&points) {
        let src = p.parent.clone().unwrap_or_else(|| "0".into());
        let ord = src_ord.entry(src.clone()).or_insert(0);
        cxns.push_str(&format!(
            r#"<dgm:cxn modelId="{cxn}" srcId="{src}" destId="{dest}" srcOrd="{ord}" destOrd="0"/>"#,
            cxn = cxn_id,
            src = xml::escape_attr(&src),
            dest = xml::escape_attr(&p.model_id),
            ord = *ord,
        ));
        *ord += 1;
    }
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><dgm:dataModel xmlns:dgm="{DGM_NS}" xmlns:a="{A_NS}" xmlns:r="{REL_NS}"><dgm:ptLst>{pts}</dgm:ptLst><dgm:cxnLst>{cxns}</dgm:cxnLst><dgm:bg/><dgm:whole/></dgm:dataModel>"#
    )
}

pub fn points_json(diagram_id: &str, data: &Element) -> Vec<Value> {
    let parents = parent_map(data);
    let visible = visible_pts(data);
    let vis_ids: HashSet<&str> = visible
        .iter()
        .filter_map(|p| p.attr_local("modelId"))
        .collect();
    visible
        .into_iter()
        .filter_map(|pt| {
            let model_id = pt.attr_local("modelId")?;
            let role = match pt.attr_local("type").unwrap_or("node") {
                "asst" => "asst",
                _ => "node",
            };
            let text = pt
                .child_by_local("t")
                .map(collect_dgm_text)
                .unwrap_or_default();
            let parent = parents.get(model_id).and_then(|src| {
                if vis_ids.contains(src.as_str()) {
                    Some(src.clone())
                } else {
                    None
                }
            });
            Some(json!({
                "id": format!("{diagram_id}/pt:{model_id}"),
                "modelId": model_id,
                "role": role,
                "text": text,
                "parent": parent,
            }))
        })
        .collect()
}

fn collect_dgm_text(t: &Element) -> String {
    let mut out = String::new();
    // dgm:t 的 local name 也是 t，不能当可见文本收；只看子孙里的 a:t / a:br。
    for n in &t.children {
        if let Node::Element(e) = n {
            collect_t_br(e, &mut out);
        }
    }
    out
}

fn collect_t_br(el: &Element, out: &mut String) {
    match el.local_name() {
        "t" => {
            out.push_str(&el.text_content());
            return;
        }
        "br" => out.push('\n'),
        _ => {}
    }
    for n in &el.children {
        if let Node::Element(e) = n {
            collect_t_br(e, out);
        }
    }
}

fn parent_map(data: &Element) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let Some(lst) = find_local(data, "cxnLst") else {
        return map;
    };
    for cxn in lst.element_children() {
        if cxn.local_name() != "cxn" {
            continue;
        }
        let ty = cxn.attr_local("type").unwrap_or("");
        if !ty.is_empty() && ty != "parOf" {
            continue;
        }
        let Some(src) = cxn.attr_local("srcId") else {
            continue;
        };
        let Some(dest) = cxn.attr_local("destId") else {
            continue;
        };
        map.entry(dest.to_string())
            .or_insert_with(|| src.to_string());
    }
    map
}

fn visible_pts(data: &Element) -> Vec<&Element> {
    let Some(lst) = find_local(data, "ptLst") else {
        return Vec::new();
    };
    lst.element_children()
        .filter(|e| e.local_name() == "pt" && is_visible_pt(e))
        .collect()
}

fn is_visible_pt(pt: &Element) -> bool {
    matches!(pt.attr_local("type").unwrap_or("node"), "node" | "asst")
}

pub fn unique_id_of(pkg: &Package, host: &str, rid: &str) -> Option<String> {
    let part = crate::opc::resolve_id(pkg, host, rid).ok()?;
    let doc = xml::parse(pkg.get(&part).ok()?).ok()?;
    doc.root.attr_local("uniqueId").map(str::to_string)
}

pub fn find_visible_pt<'a>(root: &'a Element, model_id: &str) -> Option<&'a Element> {
    visible_pts(root)
        .into_iter()
        .find(|p| p.attr_local("modelId") == Some(model_id))
}

pub fn find_visible_pt_mut<'a>(root: &'a mut Element, model_id: &str) -> Option<&'a mut Element> {
    let lst = find_local_mut(root, "ptLst")?;
    for child in lst.children.as_mut_slice() {
        if let Node::Element(e) = child {
            if e.local_name() == "pt"
                && is_visible_pt(e)
                && e.attr_local("modelId") == Some(model_id)
            {
                return Some(e);
            }
        }
    }
    None
}

/// 列表与循环版式不展示助手点，跨族时改成普通节点以保住文字。
pub fn demote_asst(data: &mut Element) {
    let Some(lst) = find_local_mut(data, "ptLst") else {
        return;
    };
    for child in lst.children.as_mut_slice() {
        if let Node::Element(e) = child {
            if e.local_name() == "pt" && e.attr_local("type") == Some("asst") {
                e.set_attr("type", "node");
            }
        }
    }
}

pub fn set_point_text(pt: &mut Element, text: &str) {
    if let Some(t) = pt.child_by_local_mut("t") {
        set_tx_text(t, text);
        return;
    }
    pt.push_element(minimal_t(text));
}

fn minimal_t(text: &str) -> Element {
    let mut t = Element::new("dgm:t");
    t.push_element(Element::new("a:bodyPr"));
    t.push_element(Element::new("a:lstStyle"));
    let mut p = Element::new("a:p");
    let mut r = Element::new("a:r");
    r.push_element(
        Element::new("a:rPr")
            .with_attr("lang", "zh-CN")
            .with_attr("sz", "1100"),
    );
    let mut at = Element::new("a:t");
    apply_t_text(&mut at, text);
    r.push_element(at);
    p.push_element(r);
    t.push_element(p);
    t
}

fn set_tx_text(tx: &mut Element, text: &str) {
    let lines: Vec<&str> = text.split('\n').collect();
    let mut filled = false;
    rewrite_paras(tx, &mut filled, &lines);
    if !filled {
        append_text_para(tx, &lines);
    }
    prune_empty_paras(tx);
}

fn rewrite_paras(el: &mut Element, filled: &mut bool, lines: &[&str]) {
    if el.local_name() == "p" {
        if !*filled && el.element_children().any(is_text_run) {
            fill_para(el, lines);
            *filled = true;
        } else {
            strip_para(el);
        }
        return;
    }
    for child in el.children.as_mut_slice() {
        if let Node::Element(e) = child {
            rewrite_paras(e, filled, lines);
        }
    }
}

fn is_text_run(e: &Element) -> bool {
    e.local_name() == "r" && e.element_children().any(|c| c.local_name() == "t")
}

fn strip_para(p: &mut Element) {
    let mut i = p.children.len();
    while i > 0 {
        i -= 1;
        let Node::Element(e) = &mut p.children[i] else {
            continue;
        };
        match e.local_name() {
            "br" => {
                p.children.remove(i);
            }
            "r" => {
                e.children.retain(|n| match n {
                    Node::Element(c) => c.local_name() != "t",
                    Node::Text(_) | Node::Raw(_) => true,
                });
                if !e.element_children().any(|c| c.local_name() != "rPr") {
                    p.children.remove(i);
                }
            }
            _ => {}
        }
    }
}

fn prune_empty_paras(el: &mut Element) {
    for child in el.children.as_mut_slice() {
        if let Node::Element(e) = child {
            if e.local_name() != "p" {
                prune_empty_paras(e);
            }
        }
    }
    let mut remaining = el
        .children
        .iter()
        .filter(|n| matches!(n, Node::Element(e) if e.local_name() == "p"))
        .count();
    if remaining <= 1 {
        return;
    }
    let mut i = el.children.len();
    while i > 0 {
        i -= 1;
        let Node::Element(e) = &el.children[i] else {
            continue;
        };
        if e.local_name() != "p" {
            continue;
        }
        if remaining <= 1 {
            break;
        }
        if !e
            .element_children()
            .any(|c| !matches!(c.local_name(), "pPr" | "endParaRPr"))
        {
            el.children.remove(i);
            remaining -= 1;
        }
    }
}

fn fill_para(p: &mut Element, lines: &[&str]) {
    let mut seen = false;
    let mut i = 0;
    while i < p.children.len() {
        let Node::Element(e) = &mut p.children[i] else {
            i += 1;
            continue;
        };
        match e.local_name() {
            "r" if is_text_run(e) && !seen => {
                seen = true;
                i += 1;
            }
            "br" => {
                p.children.remove(i);
            }
            "r" => {
                e.children.retain(|n| match n {
                    Node::Element(c) => c.local_name() != "t",
                    Node::Text(_) | Node::Raw(_) => true,
                });
                if !e.element_children().any(|c| c.local_name() != "rPr") {
                    p.children.remove(i);
                } else {
                    i += 1;
                }
            }
            _ => i += 1,
        }
    }
    let Some(at) = p
        .children
        .iter()
        .position(|n| matches!(n, Node::Element(e) if is_text_run(e)))
    else {
        return;
    };
    let (rpr, t_name) = {
        let Node::Element(run) = &mut p.children[at] else {
            return;
        };
        let t_name = match run.element_children().find(|c| c.local_name() == "t") {
            Some(t) => t.name.clone(),
            None => return,
        };
        let rpr = fill_single_line(run, lines[0]);
        (rpr, t_name)
    };
    let proto = Element::new(t_name.clone());
    let mut ins = at + 1;
    for line in &lines[1..] {
        p.children
            .insert(ins, Node::Element(Element::new(proto.prefixed("br"))));
        ins += 1;
        let mut run = Element::new(proto.prefixed("r"));
        if let Some(pr) = &rpr {
            run.push_element(pr.clone());
        }
        let mut t = Element::new(t_name.clone());
        apply_t_text(&mut t, line);
        run.push_element(t);
        p.children.insert(ins, Node::Element(run));
        ins += 1;
    }
}

fn fill_single_line(run: &mut Element, line: &str) -> Option<Element> {
    let mut seen = false;
    let mut drop = Vec::new();
    for (i, child) in run.children.iter_mut().enumerate() {
        if let Node::Element(e) = child {
            if e.local_name() == "t" {
                if seen {
                    drop.push(i);
                } else {
                    apply_t_text(e, line);
                    seen = true;
                }
            }
        }
    }
    for i in drop.into_iter().rev() {
        run.children.remove(i);
    }
    run.element_children()
        .find(|e| e.local_name() == "rPr")
        .cloned()
}

fn append_text_para(tx: &mut Element, lines: &[&str]) {
    let sample = tx
        .element_children()
        .find(|e| e.local_name() == "p")
        .or_else(|| tx.element_children().next())
        .map(|e| e.name.clone())
        .unwrap_or_else(|| "a:p".to_string());
    let proto = Element::new(sample);
    if let Some(p) = tx.child_by_local_mut("p") {
        append_lines_to_para(p, lines);
        return;
    }
    let mut p = Element::new(proto.prefixed("p"));
    append_lines_to_para(&mut p, lines);
    tx.push_element(p);
}

fn append_lines_to_para(p: &mut Element, lines: &[&str]) {
    let proto = Element::new("a:t");
    for (i, line) in lines.iter().enumerate() {
        if i > 0 {
            p.push_element(Element::new(proto.prefixed("br")));
        }
        let mut run = Element::new(proto.prefixed("r"));
        let mut t = Element::new(proto.name.clone());
        apply_t_text(&mut t, line);
        run.push_element(t);
        p.push_element(run);
    }
}

fn apply_t_text(t: &mut Element, text: &str) {
    t.set_text(text.to_string());
    if needs_xml_space(text) {
        t.set_attr("xml:space", "preserve");
    } else {
        t.remove_attr("xml:space");
    }
}

fn needs_xml_space(text: &str) -> bool {
    text.starts_with(' ') || text.starts_with('\t') || text.ends_with(' ') || text.ends_with('\t')
}

pub fn sync_drawing_text(
    pkg: &mut Package,
    host: &str,
    data_part: &str,
    model_id: &str,
    text: &str,
) -> Result<bool, Error> {
    let caches = super::collect_drawing_caches(pkg, host, data_part)?;
    if caches.is_empty() {
        return Ok(true);
    }
    let mut all_ok = true;
    for part in caches {
        let Ok(bytes) = pkg.get(&part) else {
            all_ok = false;
            continue;
        };
        let Ok(mut doc) = xml::parse(bytes) else {
            all_ok = false;
            continue;
        };
        if let Some(sp) = find_by_model_id_mut(&mut doc.root, model_id) {
            if let Some(tx) = find_local_mut(sp, "txBody") {
                set_tx_text(tx, text);
                pkg.set(&part, doc.to_bytes()?)?;
            } else {
                all_ok = false;
            }
        } else {
            all_ok = false;
        }
    }
    Ok(all_ok)
}

fn find_by_model_id_mut<'a>(el: &'a mut Element, model_id: &str) -> Option<&'a mut Element> {
    crate::xml::search_mut(el, |el| {
        if el.attr_local("modelId") == Some(model_id) {
            return crate::xml::SearchMut::Hit;
        }
        crate::xml::SearchMut::down(el)
    })
}

pub fn ensure_doc_pt(root: &mut Element) -> String {
    if let Some(lst) = find_local(root, "ptLst") {
        for pt in lst.element_children() {
            if pt.local_name() == "pt" && pt.attr_local("type") == Some("doc") {
                if let Some(id) = pt.attr_local("modelId") {
                    return id.to_string();
                }
            }
        }
    }
    let id = unused_model_id(root);
    if let Some(lst) = find_local_mut(root, "ptLst") {
        lst.children.insert(
            0,
            Node::Element(
                Element::new("dgm:pt")
                    .with_attr("type", "doc")
                    .with_attr("modelId", &id),
            ),
        );
    }
    id
}

fn all_model_ids(root: &Element) -> Vec<String> {
    let mut out = Vec::new();
    collect_model_ids(root, &mut out);
    out
}

fn collect_model_ids(el: &Element, out: &mut Vec<String>) {
    if matches!(el.local_name(), "pt" | "cxn") {
        if let Some(id) = el.attr_local("modelId") {
            out.push(id.to_string());
        }
    }
    for n in &el.children {
        if let Node::Element(e) = n {
            collect_model_ids(e, out);
        }
    }
}

fn unused_model_id(root: &Element) -> String {
    next_model_ids(root).0
}

pub fn next_model_ids(root: &Element) -> (String, String) {
    let ids = all_model_ids(root);
    if ids.iter().all(|s| s.parse::<u64>().is_ok()) {
        let used: HashSet<u64> = ids.iter().filter_map(|s| s.parse().ok()).collect();
        let mut a = 1u64;
        while used.contains(&a) {
            a += 1;
        }
        let mut b = a + 1;
        while used.contains(&b) {
            b += 1;
        }
        (a.to_string(), b.to_string())
    } else {
        (new_guid(), new_guid())
    }
}

fn new_guid() -> String {
    let n = GUID_SEQ.fetch_add(1, Ordering::Relaxed);
    let (secs, nanos) = crate::host_env::unix_secs_nanos();
    let a = (secs as u32).wrapping_mul(0x9E37_79B9) ^ (n as u32);
    let b = nanos;
    format!(
        "{{{:08X}-{:04X}-4{:03X}-A{:03X}-{:012X}}}",
        a,
        (b >> 16) as u16,
        ((b as u16) & 0x0fff),
        ((n as u16) & 0x0fff),
        n.wrapping_shl(16) | (b as u64 & 0xffff)
    )
}

pub fn append_visible_pt(root: &mut Element, model_id: &str, role: Role, text: &str) {
    let mut pt = Element::new("dgm:pt").with_attr("modelId", model_id);
    if role == Role::Asst {
        pt.set_attr("type", "asst");
    }
    pt.push_element(Element::new("dgm:prSet"));
    pt.push_element(minimal_t(text));
    if let Some(lst) = find_local_mut(root, "ptLst") {
        lst.push_element(pt);
    }
}

pub fn append_par_of(root: &mut Element, cxn_id: &str, src: &str, dest: &str) {
    let src_ord = count_children(root, src);
    let cxn = Element::new("dgm:cxn")
        .with_attr("modelId", cxn_id)
        .with_attr("srcId", src)
        .with_attr("destId", dest)
        .with_attr("srcOrd", src_ord.to_string())
        .with_attr("destOrd", "0");
    if let Some(lst) = find_local_mut(root, "cxnLst") {
        lst.push_element(cxn);
        return;
    }
    if let Some(model) = find_local_mut(root, "dataModel") {
        let mut lst = Element::new("dgm:cxnLst");
        lst.push_element(cxn);
        model.push_element(lst);
    }
}

fn count_children(root: &Element, src: &str) -> u32 {
    let Some(lst) = find_local(root, "cxnLst") else {
        return 0;
    };
    lst.element_children()
        .filter(|c| {
            c.local_name() == "cxn"
                && c.attr_local("srcId") == Some(src)
                && matches!(c.attr_local("type").unwrap_or("parOf"), "" | "parOf")
        })
        .count() as u32
}

pub fn visible_model_ids(root: &Element) -> HashSet<String> {
    visible_pts(root)
        .into_iter()
        .filter_map(|p| p.attr_local("modelId").map(str::to_string))
        .collect()
}

pub fn descendants_of(root: &Element, model_id: &str) -> HashSet<String> {
    let mut kids: HashMap<String, Vec<String>> = HashMap::new();
    if let Some(lst) = find_local(root, "cxnLst") {
        for cxn in lst.element_children() {
            if cxn.local_name() != "cxn" {
                continue;
            }
            let ty = cxn.attr_local("type").unwrap_or("");
            if !ty.is_empty() && ty != "parOf" {
                continue;
            }
            if let (Some(src), Some(dest)) = (cxn.attr_local("srcId"), cxn.attr_local("destId")) {
                kids.entry(src.to_string())
                    .or_default()
                    .push(dest.to_string());
            }
        }
    }
    let mut out = HashSet::new();
    let mut stack = vec![model_id.to_string()];
    while let Some(id) = stack.pop() {
        if !out.insert(id.clone()) {
            continue;
        }
        if let Some(ch) = kids.get(&id) {
            stack.extend(ch.iter().cloned());
        }
    }
    out
}

pub fn delete_points_and_cxns(root: &mut Element, doomed: &HashSet<String>) {
    let mut doomed_cxn = HashSet::new();
    if let Some(lst) = find_local(root, "cxnLst") {
        for cxn in lst.element_children() {
            if cxn.local_name() != "cxn" {
                continue;
            }
            let src = cxn.attr_local("srcId").unwrap_or("");
            let dest = cxn.attr_local("destId").unwrap_or("");
            if doomed.contains(src) || doomed.contains(dest) {
                if let Some(id) = cxn.attr_local("modelId") {
                    doomed_cxn.insert(id.to_string());
                }
            }
        }
    }
    let mut trans = HashSet::new();
    if let Some(lst) = find_local(root, "ptLst") {
        for pt in lst.element_children() {
            if pt.local_name() != "pt" {
                continue;
            }
            let ty = pt.attr_local("type").unwrap_or("");
            if ty != "parTrans" && ty != "sibTrans" {
                continue;
            }
            if let Some(cxn_id) = pt.attr_local("cxnId") {
                if doomed_cxn.contains(cxn_id) {
                    if let Some(id) = pt.attr_local("modelId") {
                        trans.insert(id.to_string());
                    }
                }
            }
        }
    }
    if let Some(lst) = find_local_mut(root, "ptLst") {
        lst.children.retain(|n| match n {
            Node::Element(e) if e.local_name() == "pt" => {
                let Some(id) = e.attr_local("modelId") else {
                    return true;
                };
                if e.attr_local("type") == Some("doc") {
                    return true;
                }
                !doomed.contains(id) && !trans.contains(id)
            }
            _ => true,
        });
    }
    if let Some(lst) = find_local_mut(root, "cxnLst") {
        lst.children.retain(|n| match n {
            Node::Element(e) if e.local_name() == "cxn" => {
                let src = e.attr_local("srcId").unwrap_or("");
                let dest = e.attr_local("destId").unwrap_or("");
                let id = e.attr_local("modelId").unwrap_or("");
                !doomed.contains(src)
                    && !doomed.contains(dest)
                    && !trans.contains(src)
                    && !trans.contains(dest)
                    && !doomed_cxn.contains(id)
            }
            _ => true,
        });
    }
}
