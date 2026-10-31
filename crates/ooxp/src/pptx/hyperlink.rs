//! 幻灯片对象上的超链接：框级 `hlinkClick` / `hlinkHover`，run 级 `hlinkClick` / `hlinkMouseOver`。
//! 不引用 xlsx / docx。

use serde_json::{json, Map, Value};

use crate::chart::{CHARTEX_NS, CHART_NS};
use crate::error::{Error, ErrorKind, IdReason};
use crate::opc::{resolve_rel_target, Package, Relationship, Relationships};
use crate::smartart;
use crate::xml::{Document, Element, Node};

use super::picture;
use super::request::{
    DeleteHyperlinkOp, InsertHyperlinkOp, LinkInsertHost, PptxLinkKind, SetHyperlinkOp,
};
use super::shape::{self, SlideMeta};
use super::table;
use super::zoom;
use super::{REL_HYPERLINK, REL_NS, REL_SLIDE};

const ACTION_SLIDE: &str = "ppaction://hlinksldjump";
const ACTION_FILE: &str = "ppaction://hlinkfile";

struct LinkSite {
    path: Vec<usize>,
    on: &'static str,
    event: &'static str,
    host: String,
    text: Option<String>,
}

struct Counters {
    shapes: usize,
    pics: usize,
    tables: usize,
    charts: usize,
    smart: usize,
}

pub fn count(root: &Element) -> usize {
    collect_sites(root, "").len()
}

pub fn list_json(
    root: &Element,
    host_prefix: &str,
    part: &str,
    rels: &Relationships,
    slides: &[SlideMeta],
) -> Value {
    let sites = collect_sites(root, host_prefix);
    let mut out = Vec::new();
    for (i, site) in sites.iter().enumerate() {
        let Some(el) = el_at(root, &site.path) else {
            continue;
        };
        out.push(item_json(
            el,
            i + 1,
            host_prefix,
            &site.host,
            part,
            rels,
            slides,
            site.on,
            site.event,
            site.text.as_deref(),
        ));
    }
    Value::Array(out)
}

pub fn apply_insert(
    pkg: &mut Package,
    doc: &mut Document,
    part: &str,
    slides: &[SlideMeta],
    op: &InsertHyperlinkOp,
    i: usize,
) -> Result<(), Error> {
    reject_self_jump(op.host.parts().0, &op.target, i)?;
    ensure_jump_exists(slides, &op.target, i)?;
    picture::ensure_xmlns(&mut doc.root, "r", REL_NS);
    picture::ensure_xmlns(&mut doc.root, "a", super::DML_NS);
    if op.on.is_run_like() {
        apply_insert_run(pkg, doc, part, slides, op, i)
    } else {
        apply_insert_box(pkg, doc, part, slides, op, i)
    }
}

fn apply_insert_box(
    pkg: &mut Package,
    doc: &mut Document,
    part: &str,
    slides: &[SlideMeta],
    op: &InsertHyperlinkOp,
    i: usize,
) -> Result<(), Error> {
    let tag = op.event.cnv_tag();
    let cnv = box_cnv_mut(&mut doc.root, &op.host, &op.id, i)?;
    if cnv.child_by_local(tag).is_some() {
        return Err(Error::with_path(
            ErrorKind::ShapeAlreadyHasHyperlink,
            format!("ops[{i}].id"),
        ));
    }
    let rid = push_rel(pkg, part, slides, &op.target)?;
    insert_hlink(cnv, tag, &rid, action_of(&op.target), op.tooltip.as_deref());
    Ok(())
}

fn apply_insert_run(
    pkg: &mut Package,
    doc: &mut Document,
    part: &str,
    slides: &[SlideMeta],
    op: &InsertHyperlinkOp,
    i: usize,
) -> Result<(), Error> {
    let tx = run_tx_mut(&mut doc.root, &op.host, &op.id, i)?;
    if last_para_index(tx).is_none() {
        tx.push_element(Element::new(tx.prefixed("p")));
    }
    let para_i = last_para_index(tx).ok_or_else(|| missing_host(&op.host, &op.id, i))?;
    let Node::Element(p) = &mut tx.children[para_i] else {
        return Err(missing_host(&op.host, &op.id, i));
    };
    let rid = push_rel(pkg, part, slides, &op.target)?;
    let tag = op.event.run_tag();
    match &op.text {
        Some(text) => append_linked_run(
            p,
            tag,
            &rid,
            action_of(&op.target),
            op.tooltip.as_deref(),
            text,
        ),
        None => wrap_last_run(
            p,
            tag,
            &rid,
            action_of(&op.target),
            op.tooltip.as_deref(),
            i,
        )?,
    }
    Ok(())
}

pub fn apply_set(
    pkg: &mut Package,
    doc: &mut Document,
    part: &str,
    slides: &[SlideMeta],
    op: &SetHyperlinkOp,
    i: usize,
) -> Result<(), Error> {
    if let Some(target) = &op.target {
        reject_self_jump(op.slide, target, i)?;
        ensure_jump_exists(slides, target, i)?;
    }
    picture::ensure_xmlns(&mut doc.root, "r", REL_NS);
    let path = site_path(&doc.root, op.index).ok_or_else(|| not_found(&op.id, i))?;
    let old_rid = {
        let click = el_at(&doc.root, &path).ok_or_else(|| not_found(&op.id, i))?;
        shape::rid_attr(click).map(str::to_string)
    };
    let new_rid = match &op.target {
        Some(target) => Some(set_rel(pkg, part, slides, old_rid.as_deref(), target)?),
        None => None,
    };
    {
        let click = el_at_mut(&mut doc.root, &path).ok_or_else(|| not_found(&op.id, i))?;
        if let (Some(rid), Some(target)) = (new_rid.as_deref(), op.target.as_ref()) {
            set_rid(click, rid);
            match action_of(target) {
                Some(a) => click.set_attr("action", a),
                None => click.remove_attr("action"),
            }
        }
        if let Some(tip) = &op.tooltip {
            match tip {
                None => click.remove_attr("tooltip"),
                Some(s) => click.set_attr("tooltip", s),
            }
        }
    }
    if let (Some(old), Some(new)) = (old_rid.as_deref(), new_rid.as_deref()) {
        if old != new {
            drop_rel_if_unused(pkg, part, &doc.root, old)?;
        }
    }
    Ok(())
}

pub fn apply_delete(
    pkg: &mut Package,
    root: &mut Element,
    part: &str,
    op: &DeleteHyperlinkOp,
    i: usize,
) -> Result<(), Error> {
    let path = site_path(root, op.index).ok_or_else(|| not_found(&op.id, i))?;
    let rid = {
        let click = el_at(root, &path).ok_or_else(|| not_found(&op.id, i))?;
        shape::rid_attr(click).map(str::to_string)
    };
    if !remove_at(root, &path) {
        return Err(not_found(&op.id, i));
    }
    if let Some(rid) = rid.as_deref() {
        drop_rel_if_unused(pkg, part, root, rid)?;
    }
    Ok(())
}

pub fn parse_link_id(id: &str, op_index: usize) -> Result<(picture::TreeHost, usize), Error> {
    let path = format!("ops[{op_index}].id");
    let (host, after) = picture::parse_object_id(id, &path, "link", IdReason::SlideLinkPrefix)?;
    if after.contains('/') {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SlideLinkPrefix,
            },
            path,
        ));
    }
    let index: usize = after.parse().map_err(|_| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::LinkIndexNotInt,
            },
            path.clone(),
        )
    })?;
    if index < 1 {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::LinkIndexFromOne,
            },
            path,
        ));
    }
    Ok((host, index))
}

pub fn parse_jump_slide(value: &str, i: usize, field: &str) -> Result<usize, Error> {
    let path = format!("ops[{i}].{field}");
    let rest = value.strip_prefix("slide:").ok_or_else(|| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::PptxJumpNeedsSlide,
            },
            path.clone(),
        )
    })?;
    if rest.is_empty() || rest.contains('/') {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::PptxJumpNeedsSlide,
            },
            path,
        ));
    }
    let n: usize = rest.parse().map_err(|_| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SlideIndexNotInt,
            },
            path.clone(),
        )
    })?;
    if n < 1 {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SlideIndexFromOne,
            },
            path,
        ));
    }
    Ok(n)
}

fn collect_sites(root: &Element, host_prefix: &str) -> Vec<LinkSite> {
    let mut out = Vec::new();
    let mut counters = Counters {
        shapes: 0,
        pics: 0,
        tables: 0,
        charts: 0,
        smart: 0,
    };
    walk_links(root, &[], host_prefix, &mut counters, &mut out);
    out
}

fn walk_links(
    root: &Element,
    path: &[usize],
    host_prefix: &str,
    counters: &mut Counters,
    out: &mut Vec<LinkSite>,
) {
    let mut stack = vec![(root, path.to_vec())];
    while let Some((el, path)) = stack.pop() {
        if el.local_name() == "AlternateContent" {
            if crate::drawing_math::is_math_ac(el) {
                continue;
            }
            if let Some(i) = shape::first_alt_child_index(el) {
                if let Node::Element(child) = &el.children[i] {
                    let mut next = path;
                    next.push(i);
                    stack.push((child, next));
                }
            }
            continue;
        }
        if el.local_name() == "oleObj" {
            continue;
        }
        if el.local_name() == "sp" && el.child_by_local("txBody").is_some() {
            counters.shapes += 1;
            let (host, _, _) = shape::shape_stable_id(el, host_prefix, counters.shapes);
            push_cnv_links(el, &path, "shape", &host, out);
            if let Some(i) = child_index(el, "txBody") {
                if let Some(tx) = el.child_by_local("txBody") {
                    let mut next = path;
                    next.push(i);
                    collect_runs(tx, &next, "run", &host, out);
                }
            }
            continue;
        }
        if el.local_name() == "sp" {
            continue;
        }
        if picture::is_counted_pic(el) {
            counters.pics += 1;
            let host = box_id(host_prefix, "pic", el, counters.pics);
            push_cnv_links(el, &path, "pic", &host, out);
            continue;
        }
        if el.local_name() == "pic" {
            continue;
        }
        if el.local_name() == "graphicFrame" {
            if is_zoom_frame(el) {
                continue;
            }
            if is_table_frame(el) {
                counters.tables += 1;
                let host = box_id(host_prefix, "table", el, counters.tables);
                push_cnv_links(el, &path, "table", &host, out);
                collect_cell_runs(el, &path, &host, out);
                continue;
            }
            if is_chart_frame(el) {
                counters.charts += 1;
                let host = box_id(host_prefix, "chart", el, counters.charts);
                push_cnv_links(el, &path, "chart", &host, out);
                continue;
            }
            if is_diagram_frame(el) {
                counters.smart += 1;
                let host = box_id(host_prefix, "smartdiagram", el, counters.smart);
                push_cnv_links(el, &path, "smartArt", &host, out);
                continue;
            }
            continue;
        }
        let mut kids = Vec::new();
        for (i, n) in el.children.iter().enumerate() {
            if let Node::Element(child) = n {
                let mut next = path.clone();
                next.push(i);
                kids.push((child, next));
            }
        }
        for item in kids.into_iter().rev() {
            stack.push(item);
        }
    }
}

fn push_cnv_links(
    el: &Element,
    path: &[usize],
    on: &'static str,
    host: &str,
    out: &mut Vec<LinkSite>,
) {
    let Some((cnv, cnv_path)) = cnv_with_path(el, path) else {
        return;
    };
    for (local, event) in [("hlinkClick", "click"), ("hlinkHover", "hover")] {
        if let Some(i) = child_index(cnv, local) {
            if let Node::Element(h) = &cnv.children[i] {
                if is_media_action(h) {
                    continue;
                }
                let mut p = cnv_path.clone();
                p.push(i);
                out.push(LinkSite {
                    path: p,
                    on,
                    event,
                    host: host.to_string(),
                    text: None,
                });
            }
        }
    }
}

fn collect_runs(
    el: &Element,
    path: &[usize],
    on: &'static str,
    host: &str,
    out: &mut Vec<LinkSite>,
) {
    if el.local_name() == "AlternateContent" {
        if let Some(i) = shape::first_alt_child_index(el) {
            if let Node::Element(child) = &el.children[i] {
                let mut next = path.to_vec();
                next.push(i);
                collect_runs(child, &next, on, host, out);
            }
        }
        return;
    }
    if el.local_name() == "fld" {
        return;
    }
    if el.local_name() == "r" {
        if let Some(rpr_i) = child_index(el, "rPr") {
            if let Some(rpr) = el.child_by_local("rPr") {
                let text = el
                    .child_by_local("t")
                    .map(|t| t.text_content())
                    .unwrap_or_default();
                for (local, event) in [("hlinkClick", "click"), ("hlinkMouseOver", "hover")] {
                    if let Some(j) = child_index(rpr, local) {
                        if let Node::Element(h) = &rpr.children[j] {
                            if is_media_action(h) {
                                continue;
                            }
                            let mut p = path.to_vec();
                            p.push(rpr_i);
                            p.push(j);
                            out.push(LinkSite {
                                path: p,
                                on,
                                event,
                                host: host.to_string(),
                                text: Some(text.clone()),
                            });
                        }
                    }
                }
            }
        }
        return;
    }
    for (i, n) in el.children.iter().enumerate() {
        if let Node::Element(e) = n {
            let mut next = path.to_vec();
            next.push(i);
            collect_runs(e, &next, on, host, out);
        }
    }
}

fn collect_cell_runs(frame: &Element, path: &[usize], table_id: &str, out: &mut Vec<LinkSite>) {
    let Some((tbl, tbl_path)) = tbl_with_path(frame, path) else {
        return;
    };
    let mut row = 0usize;
    for (ti, n) in tbl.children.iter().enumerate() {
        let Node::Element(tr) = n else {
            continue;
        };
        if tr.local_name() != "tr" {
            continue;
        }
        row += 1;
        let mut col = 0usize;
        let mut tr_path = tbl_path.clone();
        tr_path.push(ti);
        for (ci, cn) in tr.children.iter().enumerate() {
            let Node::Element(tc) = cn else {
                continue;
            };
            if tc.local_name() != "tc" {
                continue;
            }
            col += 1;
            let host = format!("{table_id}/r:{row}/c:{col}");
            if let Some(tx_i) = child_index(tc, "txBody") {
                if let Some(tx) = tc.child_by_local("txBody") {
                    let mut tx_path = tr_path.clone();
                    tx_path.push(ci);
                    tx_path.push(tx_i);
                    collect_runs(tx, &tx_path, "cell", &host, out);
                }
            }
        }
    }
}

fn item_json(
    click: &Element,
    index: usize,
    host_prefix: &str,
    host: &str,
    part: &str,
    rels: &Relationships,
    slides: &[SlideMeta],
    on: &str,
    event: &str,
    text: Option<&str>,
) -> Value {
    let rid = shape::rid_attr(click).map(str::to_string);
    let rel = rid.as_deref().and_then(|id| rels.find_by_id(id));
    let action = click.attr_local("action").unwrap_or("");
    let kind = classify(action, rel);
    let mut url = Value::Null;
    let mut file = Value::Null;
    let mut jump = Value::Null;
    match kind {
        "url" => {
            if let Some(r) = rel {
                url = json!(r.target);
            }
        }
        "file" => {
            if let Some(r) = rel {
                file = json!(r.target);
            }
        }
        "slide" => {
            if let Some(n) = resolve_jump(part, rel, slides) {
                jump = json!(format!("slide:{n}"));
            }
        }
        _ => {}
    }
    let mut obj = Map::new();
    obj.insert("id".into(), json!(format!("{host_prefix}/link:{index}")));
    obj.insert("index".into(), json!(index));
    obj.insert("kind".into(), json!(kind));
    obj.insert("on".into(), json!(on));
    obj.insert("event".into(), json!(event));
    obj.insert("host".into(), json!(host));
    obj.insert("rId".into(), rid.map(|s| json!(s)).unwrap_or(Value::Null));
    obj.insert("url".into(), url);
    obj.insert("file".into(), file);
    obj.insert("slide".into(), jump);
    obj.insert(
        "tooltip".into(),
        click
            .attr_local("tooltip")
            .map(|s| json!(s))
            .unwrap_or(Value::Null),
    );
    obj.insert("text".into(), text.map(|s| json!(s)).unwrap_or(Value::Null));
    Value::Object(obj)
}

fn last_para_index(tx: &Element) -> Option<usize> {
    tx.children
        .iter()
        .rposition(|n| matches!(n, Node::Element(e) if e.local_name() == "p"))
}

fn last_run_index(p: &Element) -> Option<usize> {
    p.children
        .iter()
        .rposition(|n| matches!(n, Node::Element(e) if e.local_name() == "r"))
}

fn last_rpr(p: &Element) -> Option<Element> {
    for n in p.children.iter().rev() {
        let Node::Element(e) = n else {
            continue;
        };
        if matches!(e.local_name(), "r" | "fld") {
            if let Some(rpr) = e.child_by_local("rPr") {
                return Some(rpr.clone());
            }
        }
    }
    p.child_by_local("endParaRPr").cloned()
}

fn strip_hlink(el: &mut Element, local: &str) {
    el.children
        .retain(|n| !matches!(n, Node::Element(e) if e.local_name() == local));
}

fn needs_xml_space(text: &str) -> bool {
    text.starts_with(' ') || text.starts_with('\t') || text.ends_with(' ') || text.ends_with('\t')
}

fn append_linked_run(
    p: &mut Element,
    tag: &str,
    rid: &str,
    action: Option<&str>,
    tooltip: Option<&str>,
    text: &str,
) {
    let mut r = Element::new(p.prefixed("r"));
    let mut rpr = last_rpr(p).unwrap_or_else(|| Element::new(p.prefixed("rPr")));
    rpr.name = p.prefixed("rPr");
    strip_hlink(&mut rpr, tag);
    insert_hlink(&mut rpr, tag, rid, action, tooltip);
    r.push_element(rpr);
    let mut t = Element::new(r.prefixed("t"));
    if !text.is_empty() {
        t.set_text(text);
        if needs_xml_space(text) {
            t.set_attr("xml:space", "preserve");
        }
    }
    r.push_element(t);
    let at = p
        .children
        .iter()
        .position(|n| matches!(n, Node::Element(e) if e.local_name() == "endParaRPr"))
        .unwrap_or(p.children.len());
    p.children.insert(at, Node::Element(r));
}

fn wrap_last_run(
    p: &mut Element,
    tag: &str,
    rid: &str,
    action: Option<&str>,
    tooltip: Option<&str>,
    i: usize,
) -> Result<(), Error> {
    let run_i = last_run_index(p)
        .ok_or_else(|| Error::with_path(ErrorKind::PptxRunLinkNeedsRun, format!("ops[{i}].id")))?;
    let Node::Element(r) = &mut p.children[run_i] else {
        return Err(Error::with_path(
            ErrorKind::PptxRunLinkNeedsRun,
            format!("ops[{i}].id"),
        ));
    };
    if r.child_by_local("rPr")
        .and_then(|rpr| rpr.child_by_local(tag))
        .is_some()
    {
        return Err(Error::with_path(
            ErrorKind::RunAlreadyHasHyperlink,
            format!("ops[{i}].id"),
        ));
    }
    if r.child_by_local("rPr").is_none() {
        r.children
            .insert(0, Node::Element(Element::new(r.prefixed("rPr"))));
    }
    let rpr = r
        .child_by_local_mut("rPr")
        .ok_or_else(|| Error::with_path(ErrorKind::PptxRunLinkNeedsRun, format!("ops[{i}].id")))?;
    insert_hlink(rpr, tag, rid, action, tooltip);
    Ok(())
}

fn classify(action: &str, rel: Option<&Relationship>) -> &'static str {
    if action.contains("hlinksldjump")
        || rel.is_some_and(|r| r.rel_type.ends_with("/slide") && !r.is_external())
    {
        "slide"
    } else if action.contains("hlinkfile") {
        "file"
    } else {
        "url"
    }
}

fn resolve_jump(part: &str, rel: Option<&Relationship>, slides: &[SlideMeta]) -> Option<usize> {
    let rel = rel?;
    if rel.is_external() {
        return None;
    }
    let target = resolve_rel_target(part, &rel.target).ok()?;
    let norm = target.trim_start_matches('/');
    slides
        .iter()
        .find_map(|s| (s.part.trim_start_matches('/') == norm).then_some(s.index))
}

fn box_cnv_mut<'a>(
    root: &'a mut Element,
    host: &LinkInsertHost,
    id: &str,
    i: usize,
) -> Result<&'a mut Element, Error> {
    match host {
        LinkInsertHost::Shape(t) => {
            let sp =
                shape::find_shape_mut(root, &t.key).ok_or_else(|| missing_host(host, id, i))?;
            cnv_pr_mut(sp).ok_or_else(|| missing_host(host, id, i))
        }
        LinkInsertHost::Pic(t) => {
            let pic =
                picture::find_pic_mut(root, &t.key).ok_or_else(|| missing_host(host, id, i))?;
            pic_cnv_mut(pic).ok_or_else(|| missing_host(host, id, i))
        }
        LinkInsertHost::Table(t) => {
            let frame =
                table::find_frame_mut(root, &t.key).ok_or_else(|| missing_host(host, id, i))?;
            graphic_cnv_mut(frame).ok_or_else(|| missing_host(host, id, i))
        }
        LinkInsertHost::Chart(t) => {
            let frame = super::chart::find_frame_mut(root, &t.key)
                .ok_or_else(|| missing_host(host, id, i))?;
            graphic_cnv_mut(frame).ok_or_else(|| missing_host(host, id, i))
        }
        LinkInsertHost::SmartArt(t) => {
            let frame = super::smartart::find_frame_mut(root, &t.key)
                .ok_or_else(|| missing_host(host, id, i))?;
            graphic_cnv_mut(frame).ok_or_else(|| missing_host(host, id, i))
        }
        LinkInsertHost::Cell(_) => Err(missing_host(host, id, i)),
    }
}

fn run_tx_mut<'a>(
    root: &'a mut Element,
    host: &LinkInsertHost,
    id: &str,
    i: usize,
) -> Result<&'a mut Element, Error> {
    match host {
        LinkInsertHost::Shape(t) => {
            let sp =
                shape::find_shape_mut(root, &t.key).ok_or_else(|| missing_host(host, id, i))?;
            sp.child_by_local_mut("txBody")
                .ok_or_else(|| missing_host(host, id, i))
        }
        LinkInsertHost::Cell(c) => {
            table::cell_tx_mut(root, c).ok_or_else(|| missing_host(host, id, i))
        }
        _ => Err(Error::with_path(
            ErrorKind::InvalidHyperlinkOn,
            format!("ops[{i}].on"),
        )),
    }
}

fn cnv_pr_mut(sp: &mut Element) -> Option<&mut Element> {
    sp.child_by_local_mut("nvSpPr")
        .and_then(|nv| nv.child_by_local_mut("cNvPr"))
}

fn pic_cnv_mut(pic: &mut Element) -> Option<&mut Element> {
    pic.child_by_local_mut("nvPicPr")
        .and_then(|nv| nv.child_by_local_mut("cNvPr"))
}

fn graphic_cnv_mut(frame: &mut Element) -> Option<&mut Element> {
    frame
        .child_by_local_mut("nvGraphicFramePr")
        .and_then(|nv| nv.child_by_local_mut("cNvPr"))
}

pub(crate) fn pin_links(root: &mut Element) {
    let paths: Vec<Vec<usize>> = collect_sites(root, "")
        .into_iter()
        .map(|site| site.path)
        .collect();
    let attr = crate::xml::pin_attr("link");
    for (i, path) in paths.iter().enumerate() {
        let Some(el) = el_at_mut(root, path) else {
            continue;
        };
        if el.attr(&attr).is_none() {
            el.set_attr(&attr, (i + 1).to_string());
        }
    }
}

fn site_path(root: &Element, index: usize) -> Option<Vec<usize>> {
    let attr = crate::xml::pin_attr("link");
    let mut live = 0usize;
    for site in collect_sites(root, "") {
        live += 1;
        let el = el_at(root, &site.path)?;
        if crate::xml::pin_matches(el, &attr, index, live) {
            return Some(site.path);
        }
    }
    None
}

fn el_at<'a>(root: &'a Element, path: &[usize]) -> Option<&'a Element> {
    let mut cur = root;
    for &i in path {
        let Node::Element(e) = cur.children.get(i)? else {
            return None;
        };
        cur = e;
    }
    Some(cur)
}

fn el_at_mut<'a>(root: &'a mut Element, path: &[usize]) -> Option<&'a mut Element> {
    let mut cur = root;
    for &i in path {
        let child = cur.children.get_mut(i)?;
        let Node::Element(e) = child else {
            return None;
        };
        cur = e;
    }
    Some(cur)
}

fn remove_at(root: &mut Element, path: &[usize]) -> bool {
    if path.is_empty() {
        return false;
    }
    let (parent_path, last) = path.split_at(path.len() - 1);
    let Some(parent) = el_at_mut(root, parent_path) else {
        return false;
    };
    let i = last[0];
    if i >= parent.children.len() {
        return false;
    }
    parent.children.remove(i);
    true
}

fn insert_hlink(
    parent: &mut Element,
    local: &str,
    rid: &str,
    action: Option<&str>,
    tooltip: Option<&str>,
) {
    let mut el = Element::new(format!("a:{local}"));
    el.set_attr("r:id", rid);
    if let Some(a) = action {
        el.set_attr("action", a);
    }
    if let Some(t) = tooltip {
        el.set_attr("tooltip", t);
    }
    let pos = insert_pos(parent, local);
    match pos {
        Some(i) => parent.children.insert(i, Node::Element(el)),
        None => parent.push_element(el),
    }
}

fn insert_pos(parent: &Element, local: &str) -> Option<usize> {
    match local {
        "hlinkClick" => parent.children.iter().position(|n| {
            matches!(
                n,
                Node::Element(e)
                    if matches!(e.local_name(), "hlinkHover" | "hlinkMouseOver" | "extLst")
            )
        }),
        "hlinkHover" | "hlinkMouseOver" => {
            let mut after_click = None;
            let mut ext = None;
            for (i, n) in parent.children.iter().enumerate() {
                if let Node::Element(e) = n {
                    if e.local_name() == "hlinkClick" {
                        after_click = Some(i + 1);
                    }
                    if e.local_name() == "extLst" && ext.is_none() {
                        ext = Some(i);
                    }
                }
            }
            after_click.or(ext)
        }
        _ => None,
    }
}

fn set_rid(click: &mut Element, rid: &str) {
    if let Some((k, _)) = click
        .attrs
        .iter()
        .find(|(k, _)| k.contains(':') && k.rsplit(':').next() == Some("id"))
    {
        let name = k.clone();
        click.set_attr(&name, rid);
        return;
    }
    click.set_attr("r:id", rid);
}

fn action_of(target: &PptxLinkKind) -> Option<&'static str> {
    match target {
        PptxLinkKind::Url(_) => None,
        PptxLinkKind::File(_) => Some(ACTION_FILE),
        PptxLinkKind::Slide(_) => Some(ACTION_SLIDE),
    }
}

fn push_rel(
    pkg: &mut Package,
    part: &str,
    slides: &[SlideMeta],
    target: &PptxLinkKind,
) -> Result<String, Error> {
    let mut rels = Relationships::from_package(pkg, part)?;
    let rid = rels.next_rid();
    rels.push(make_rel(&rid, part, slides, target));
    rels.write_to(pkg, part)?;
    Ok(rid)
}

fn set_rel(
    pkg: &mut Package,
    part: &str,
    slides: &[SlideMeta],
    old_rid: Option<&str>,
    target: &PptxLinkKind,
) -> Result<String, Error> {
    let mut rels = Relationships::from_package(pkg, part)?;
    if let Some(old) = old_rid {
        if let Some(r) = rels.rels.iter_mut().find(|r| r.id == old) {
            *r = make_rel(old, part, slides, target);
            rels.write_to(pkg, part)?;
            return Ok(old.to_string());
        }
    }
    let rid = rels.next_rid();
    rels.push(make_rel(&rid, part, slides, target));
    rels.write_to(pkg, part)?;
    Ok(rid)
}

fn make_rel(rid: &str, part: &str, slides: &[SlideMeta], target: &PptxLinkKind) -> Relationship {
    match target {
        PptxLinkKind::Url(url) | PptxLinkKind::File(url) => Relationship {
            id: rid.to_string(),
            rel_type: REL_HYPERLINK.into(),
            target: url.clone(),
            target_mode: Some("External".into()),
            extra: Vec::new(),
        },
        PptxLinkKind::Slide(n) => {
            let dest = slides
                .iter()
                .find(|s| s.start == Some(*n))
                .map(|s| s.part.as_str())
                .unwrap_or("");
            Relationship {
                id: rid.to_string(),
                rel_type: REL_SLIDE.into(),
                target: picture::rel_target_from(part, dest),
                target_mode: None,
                extra: Vec::new(),
            }
        }
    }
}

fn drop_rel_if_unused(
    pkg: &mut Package,
    part: &str,
    root: &Element,
    rid: &str,
) -> Result<(), Error> {
    if rid_used(root, rid) {
        return Ok(());
    }
    let mut rels = Relationships::from_package(pkg, part)?;
    if rels.find_by_id(rid).is_some() {
        rels.remove_by_id(rid);
        rels.write_to(pkg, part)?;
    }
    Ok(())
}

fn rid_used(el: &Element, rid: &str) -> bool {
    if shape::rid_attr(el) == Some(rid) {
        return true;
    }
    el.element_children().any(|c| rid_used(c, rid))
}

fn reject_self_jump(host: usize, target: &PptxLinkKind, i: usize) -> Result<(), Error> {
    if let PptxLinkKind::Slide(n) = target {
        if *n == host {
            return Err(Error::with_path(
                ErrorKind::PptxHyperlinkSelfSlide,
                format!("ops[{i}].slide"),
            ));
        }
    }
    Ok(())
}

fn ensure_jump_exists(slides: &[SlideMeta], target: &PptxLinkKind, i: usize) -> Result<(), Error> {
    if let PptxLinkKind::Slide(n) = target {
        if !slides.iter().any(|s| s.start == Some(*n)) {
            return Err(Error::with_path(
                ErrorKind::SlideNotFound { n: *n as u32 },
                format!("ops[{i}].slide"),
            ));
        }
    }
    Ok(())
}

fn not_found(id: &str, i: usize) -> Error {
    Error::with_path(
        ErrorKind::HyperlinkNotFound { id: id.to_string() },
        format!("ops[{i}].id"),
    )
}

fn missing_host(host: &LinkInsertHost, id: &str, i: usize) -> Error {
    let kind = match host {
        LinkInsertHost::Shape(_) => ErrorKind::ShapeNotFound { id: id.to_string() },
        LinkInsertHost::Pic(_) => ErrorKind::PictureNotFound { id: id.to_string() },
        LinkInsertHost::Table(_) | LinkInsertHost::Cell(_) => {
            ErrorKind::TableNotFound { id: id.to_string() }
        }
        LinkInsertHost::Chart(_) => ErrorKind::ChartNotFound { id: id.to_string() },
        LinkInsertHost::SmartArt(_) => ErrorKind::SmartArtNotFound { id: id.to_string() },
    };
    Error::with_path(kind, format!("ops[{i}].id"))
}

fn child_index(el: &Element, local: &str) -> Option<usize> {
    el.children
        .iter()
        .position(|n| matches!(n, Node::Element(e) if e.local_name() == local))
}

fn cnv_with_path<'a>(el: &'a Element, path: &[usize]) -> Option<(&'a Element, Vec<usize>)> {
    for nv_name in ["nvSpPr", "nvPicPr", "nvGraphicFramePr"] {
        if let Some(i) = child_index(el, nv_name) {
            let nv = el.child_by_local(nv_name)?;
            if let Some(j) = child_index(nv, "cNvPr") {
                let cnv = nv.child_by_local("cNvPr")?;
                let mut p = path.to_vec();
                p.push(i);
                p.push(j);
                return Some((cnv, p));
            }
        }
    }
    None
}

fn tbl_with_path<'a>(el: &'a Element, path: &[usize]) -> Option<(&'a Element, Vec<usize>)> {
    if el.local_name() == "AlternateContent" {
        if let Some(i) = shape::first_alt_child_index(el) {
            if let Node::Element(child) = &el.children[i] {
                let mut next = path.to_vec();
                next.push(i);
                return tbl_with_path(child, &next);
            }
        }
        return None;
    }
    if el.local_name() == "tbl" {
        return Some((el, path.to_vec()));
    }
    for (i, n) in el.children.iter().enumerate() {
        if let Node::Element(e) = n {
            let mut next = path.to_vec();
            next.push(i);
            if let Some(found) = tbl_with_path(e, &next) {
                return Some(found);
            }
        }
    }
    None
}

fn box_id(prefix: &str, kind: &str, el: &Element, index: usize) -> String {
    match cnv_raw_id(el) {
        Some(id) => format!("{prefix}/{kind}:{id}"),
        None => format!("{prefix}/{kind}:index:{index}"),
    }
}

fn cnv_raw_id(el: &Element) -> Option<String> {
    let cnv = el
        .child_by_local("nvSpPr")
        .or_else(|| el.child_by_local("nvPicPr"))
        .or_else(|| el.child_by_local("nvGraphicFramePr"))
        .and_then(|nv| nv.child_by_local("cNvPr"))?;
    cnv.attr_local("id")
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn graphic_uri(el: &Element) -> Option<String> {
    let mut found = None;
    shape::walk_choice(el, &mut |e| {
        if e.local_name() == "graphicData" && found.is_none() {
            found = e.attr_local("uri").map(str::to_string);
        }
    });
    found
}

fn is_table_frame(el: &Element) -> bool {
    el.local_name() == "graphicFrame"
        && graphic_uri(el).is_some_and(|u| u.trim_end_matches('/') == table::TABLE_URI)
}

fn is_chart_frame(el: &Element) -> bool {
    el.local_name() == "graphicFrame"
        && graphic_uri(el).is_some_and(|u| {
            let n = u.trim_end_matches('/');
            n == CHART_NS || n == CHARTEX_NS
        })
}

fn is_diagram_frame(el: &Element) -> bool {
    el.local_name() == "graphicFrame"
        && graphic_uri(el).is_some_and(|u| smartart::is_diagram_uri(&u))
}

fn is_zoom_frame(el: &Element) -> bool {
    el.local_name() == "graphicFrame"
        && graphic_uri(el).is_some_and(|u| {
            let n = u.trim_end_matches('/');
            n.eq_ignore_ascii_case(zoom::SLIDE_ZOOM_URI.trim_end_matches('/'))
                || n.eq_ignore_ascii_case(zoom::SECTION_ZOOM_URI.trim_end_matches('/'))
                || n.eq_ignore_ascii_case(zoom::SUMMARY_ZOOM_URI.trim_end_matches('/'))
        })
}

fn is_media_action(el: &Element) -> bool {
    el.attr_local("action").unwrap_or("").contains("media")
}

pub(crate) fn op_insert_hyperlink(
    pkg: &mut crate::opc::Package,
    pres_part: &str,
    pres: &mut crate::xml::Document,
    slides: &[super::shape::SlideMeta],
    op: &super::request::InsertHyperlinkOp,
    i: usize,
) -> Result<(), crate::error::Error> {
    let (slide, on_notes, master) = op.host.parts();
    let part = super::apply::ensure_host_part(
        pkg,
        pres_part,
        pres,
        slides,
        slide,
        on_notes,
        master,
        format!("ops[{i}].id"),
    )?;
    let mut doc = super::list_pin::parse_part(pkg, &part)?;
    apply_insert(pkg, &mut doc, &part, slides, op, i)?;
    super::list_pin::write_part(pkg, &part, &doc)?;
    Ok(())
}

pub(crate) fn op_set_hyperlink(
    pkg: &mut crate::opc::Package,
    pres_part: &str,
    pres: &crate::xml::Document,
    slides: &[super::shape::SlideMeta],
    op: &super::request::SetHyperlinkOp,
    i: usize,
) -> Result<(), crate::error::Error> {
    let part = super::apply::host_xml_part(
        pkg,
        pres_part,
        &pres.root,
        slides,
        op.slide,
        op.on_notes,
        op.master,
        ErrorKind::HyperlinkNotFound { id: op.id.clone() },
        format!("ops[{i}].id"),
    )?;
    let mut doc = super::list_pin::parse_part(pkg, &part)?;
    apply_set(pkg, &mut doc, &part, slides, op, i)?;
    super::list_pin::write_part(pkg, &part, &doc)?;
    Ok(())
}

pub(crate) fn op_delete_hyperlink(
    pkg: &mut crate::opc::Package,
    pres_part: &str,
    pres: &crate::xml::Document,
    slides: &[super::shape::SlideMeta],
    op: &super::request::DeleteHyperlinkOp,
    i: usize,
) -> Result<(), crate::error::Error> {
    let part = super::apply::host_xml_part(
        pkg,
        pres_part,
        &pres.root,
        slides,
        op.slide,
        op.on_notes,
        op.master,
        ErrorKind::HyperlinkNotFound { id: op.id.clone() },
        format!("ops[{i}].id"),
    )?;
    let mut doc = super::list_pin::parse_part(pkg, &part)?;
    apply_delete(pkg, &mut doc.root, &part, op, i)?;
    super::list_pin::write_part(pkg, &part, &doc)?;
    Ok(())
}
