//! 幻灯片文本形状上的 `a:hlinkClick`：形状级与 run 级。不引用 xlsx / docx。

use serde_json::{json, Map, Value};

use crate::error::{Error, ErrorKind, IdReason};
use crate::opc::{resolve_rel_target, Package, Relationship, Relationships};
use crate::xml::{Document, Element, Node};

use super::picture;
use super::request::{DeleteHyperlinkOp, InsertHyperlinkOp, PptxLinkKind, SetHyperlinkOp};
use super::shape::{self, SlideMeta};
use super::{REL_HYPERLINK, REL_NS, REL_SLIDE};

const ACTION_SLIDE: &str = "ppaction://hlinksldjump";
const ACTION_FILE: &str = "ppaction://hlinkfile";

struct ClickView<'a> {
    click: &'a Element,
    on: &'static str,
    text: Option<String>,
}

pub fn count(root: &Element) -> usize {
    let mut n = 0usize;
    for sp in shape::collect_text_shapes(root) {
        n += clicks_in_shape(sp).len();
    }
    n
}

pub fn list_json(
    root: &Element,
    slide: usize,
    part: &str,
    rels: &Relationships,
    slides: &[SlideMeta],
) -> Value {
    let host_prefix = format!("slide:{slide}");
    let mut out = Vec::new();
    let mut index = 0usize;
    for (shape_index, sp) in shape::collect_text_shapes(root).into_iter().enumerate() {
        let (host, _, _) = shape::shape_stable_id(sp, &host_prefix, shape_index + 1);
        for view in clicks_in_shape(sp) {
            index += 1;
            out.push(item_json(
                view.click,
                index,
                slide,
                &host,
                part,
                rels,
                slides,
                view.on,
                view.text.as_deref(),
            ));
        }
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
    reject_self_jump(op.shape.slide, &op.target, i)?;
    ensure_jump_exists(slides, &op.target, i)?;
    picture::ensure_xmlns(&mut doc.root, "r", REL_NS);
    picture::ensure_xmlns(&mut doc.root, "a", super::DML_NS);
    if op.on_run {
        apply_insert_run(pkg, doc, part, slides, op, i)
    } else {
        apply_insert_shape(pkg, doc, part, slides, op, i)
    }
}

fn apply_insert_shape(
    pkg: &mut Package,
    doc: &mut Document,
    part: &str,
    slides: &[SlideMeta],
    op: &InsertHyperlinkOp,
    i: usize,
) -> Result<(), Error> {
    let sp = shape::find_shape_mut(&mut doc.root, &op.shape.key).ok_or_else(|| {
        Error::with_path(
            ErrorKind::ShapeNotFound { id: op.id.clone() },
            format!("ops[{i}].id"),
        )
    })?;
    let cnv = cnv_pr_mut(sp).ok_or_else(|| {
        Error::with_path(
            ErrorKind::ShapeNotFound { id: op.id.clone() },
            format!("ops[{i}].id"),
        )
    })?;
    if cnv.child_by_local("hlinkClick").is_some() {
        return Err(Error::with_path(
            ErrorKind::ShapeAlreadyHasHyperlink,
            format!("ops[{i}].id"),
        ));
    }
    let rid = push_rel(pkg, part, slides, &op.target)?;
    insert_click(cnv, &rid, action_of(&op.target), op.tooltip.as_deref());
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
    let sp = shape::find_shape_mut(&mut doc.root, &op.shape.key).ok_or_else(|| {
        Error::with_path(
            ErrorKind::ShapeNotFound { id: op.id.clone() },
            format!("ops[{i}].id"),
        )
    })?;
    let tx = sp.child_by_local_mut("txBody").ok_or_else(|| {
        Error::with_path(
            ErrorKind::ShapeNotFound { id: op.id.clone() },
            format!("ops[{i}].id"),
        )
    })?;
    if last_para_index(tx).is_none() {
        tx.push_element(Element::new(tx.prefixed("p")));
    }
    let para_i = last_para_index(tx).ok_or_else(|| {
        Error::with_path(
            ErrorKind::ShapeNotFound { id: op.id.clone() },
            format!("ops[{i}].id"),
        )
    })?;
    let Node::Element(p) = &mut tx.children[para_i] else {
        return Err(Error::with_path(
            ErrorKind::ShapeNotFound { id: op.id.clone() },
            format!("ops[{i}].id"),
        ));
    };
    let rid = push_rel(pkg, part, slides, &op.target)?;
    match &op.text {
        Some(text) => {
            append_linked_run(p, &rid, action_of(&op.target), op.tooltip.as_deref(), text)
        }
        None => wrap_last_run(p, &rid, action_of(&op.target), op.tooltip.as_deref(), i)?,
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
    let old_rid = {
        let click = find_click(&doc.root, op.index).ok_or_else(|| not_found(&op.id, i))?;
        shape::rid_attr(click).map(str::to_string)
    };
    let new_rid = match &op.target {
        Some(target) => Some(set_rel(pkg, part, slides, old_rid.as_deref(), target)?),
        None => None,
    };
    {
        let click = find_click_mut(&mut doc.root, op.index).ok_or_else(|| not_found(&op.id, i))?;
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
    let rid = {
        let click = find_click(root, op.index).ok_or_else(|| not_found(&op.id, i))?;
        shape::rid_attr(click).map(str::to_string)
    };
    if !remove_click(root, op.index) {
        return Err(not_found(&op.id, i));
    }
    if let Some(rid) = rid.as_deref() {
        drop_rel_if_unused(pkg, part, root, rid)?;
    }
    Ok(())
}

pub fn parse_link_id(id: &str, op_index: usize) -> Result<(usize, usize), Error> {
    let path = format!("ops[{op_index}].id");
    let rest = id.strip_prefix("slide:").ok_or_else(|| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SlideLinkPrefix,
            },
            path.clone(),
        )
    })?;
    let Some((n, after)) = rest.split_once("/link:") else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SlideLinkPrefix,
            },
            path,
        ));
    };
    if n.contains('/') || after.contains('/') {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SlideLinkPrefix,
            },
            path,
        ));
    }
    let slide: usize = n.parse().map_err(|_| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SlideIndexNotInt,
            },
            path.clone(),
        )
    })?;
    if slide < 1 {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SlideIndexFromOne,
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
    Ok((slide, index))
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

fn item_json(
    click: &Element,
    index: usize,
    slide: usize,
    host: &str,
    part: &str,
    rels: &Relationships,
    slides: &[SlideMeta],
    on: &str,
    text: Option<&str>,
) -> Value {
    let rid = shape::rid_attr(click).map(str::to_string);
    let rel = rid.as_deref().and_then(|id| rels.find_by_id(id));
    let action = click.attr("action").unwrap_or("");
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
    obj.insert("id".into(), json!(format!("slide:{slide}/link:{index}")));
    obj.insert("index".into(), json!(index));
    obj.insert("kind".into(), json!(kind));
    obj.insert("on".into(), json!(on));
    obj.insert("host".into(), json!(host));
    obj.insert("rId".into(), rid.map(|s| json!(s)).unwrap_or(Value::Null));
    obj.insert("url".into(), url);
    obj.insert("file".into(), file);
    obj.insert("slide".into(), jump);
    obj.insert(
        "tooltip".into(),
        click
            .attr("tooltip")
            .map(|s| json!(s))
            .unwrap_or(Value::Null),
    );
    obj.insert("text".into(), text.map(|s| json!(s)).unwrap_or(Value::Null));
    Value::Object(obj)
}

fn clicks_in_shape(sp: &Element) -> Vec<ClickView<'_>> {
    let mut out = Vec::new();
    if let Some(click) = hlink_click(sp) {
        out.push(ClickView {
            click,
            on: "shape",
            text: None,
        });
    }
    if let Some(tx) = sp.child_by_local("txBody") {
        collect_run_clicks(tx, &mut out);
    }
    out
}

fn collect_run_clicks<'a>(el: &'a Element, out: &mut Vec<ClickView<'a>>) {
    if el.local_name() == "AlternateContent" {
        if let Some(i) = shape::first_alt_child_index(el) {
            if let Node::Element(child) = &el.children[i] {
                collect_run_clicks(child, out);
            }
        }
        return;
    }
    if el.local_name() == "fld" {
        return;
    }
    if el.local_name() == "r" {
        if let Some(click) = el
            .child_by_local("rPr")
            .and_then(|rpr| rpr.child_by_local("hlinkClick"))
        {
            let text = el
                .child_by_local("t")
                .map(|t| t.text_content())
                .unwrap_or_default();
            out.push(ClickView {
                click,
                on: "run",
                text: Some(text),
            });
        }
        return;
    }
    for n in &el.children {
        if let Node::Element(e) = n {
            collect_run_clicks(e, out);
        }
    }
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

fn strip_hlink_click(el: &mut Element) {
    el.children
        .retain(|n| !matches!(n, Node::Element(e) if e.local_name() == "hlinkClick"));
}

fn needs_xml_space(text: &str) -> bool {
    text.starts_with(' ') || text.starts_with('\t') || text.ends_with(' ') || text.ends_with('\t')
}

fn append_linked_run(
    p: &mut Element,
    rid: &str,
    action: Option<&str>,
    tooltip: Option<&str>,
    text: &str,
) {
    let mut r = Element::new(p.prefixed("r"));
    let mut rpr = last_rpr(p).unwrap_or_else(|| Element::new(p.prefixed("rPr")));
    rpr.name = p.prefixed("rPr");
    strip_hlink_click(&mut rpr);
    insert_click(&mut rpr, rid, action, tooltip);
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
        .and_then(|rpr| rpr.child_by_local("hlinkClick"))
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
    insert_click(rpr, rid, action, tooltip);
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

fn hlink_click(sp: &Element) -> Option<&Element> {
    shape::cnv_pr(sp)?.child_by_local("hlinkClick")
}

fn cnv_pr_mut(sp: &mut Element) -> Option<&mut Element> {
    sp.child_by_local_mut("nvSpPr")
        .and_then(|nv| nv.child_by_local_mut("cNvPr"))
}

fn find_click(root: &Element, index: usize) -> Option<&Element> {
    let mut seen = 0usize;
    for sp in shape::collect_text_shapes(root) {
        for view in clicks_in_shape(sp) {
            seen += 1;
            if seen == index {
                return Some(view.click);
            }
        }
    }
    None
}

fn find_click_mut(root: &mut Element, index: usize) -> Option<&mut Element> {
    let mut seen = 0usize;
    find_click_in(root, index, &mut seen)
}

fn find_click_in<'a>(
    el: &'a mut Element,
    want: usize,
    seen: &mut usize,
) -> Option<&'a mut Element> {
    if el.local_name() == "AlternateContent" {
        if let Some(i) = shape::first_alt_child_index(el) {
            if let Node::Element(child) = &mut el.children[i] {
                return find_click_in(child, want, seen);
            }
        }
        return None;
    }
    let is_text_sp = el.local_name() == "sp" && el.child_by_local("txBody").is_some();
    if is_text_sp {
        if hlink_click(el).is_some() {
            *seen += 1;
            if *seen == want {
                return cnv_pr_mut(el).and_then(|c| c.child_by_local_mut("hlinkClick"));
            }
        }
        if let Some(tx) = el.child_by_local_mut("txBody") {
            return find_run_click_in(tx, want, seen);
        }
        return None;
    }
    for child in el.children.as_mut_slice() {
        if let Node::Element(e) = child {
            if let Some(found) = find_click_in(e, want, seen) {
                return Some(found);
            }
        }
    }
    None
}

fn find_run_click_in<'a>(
    el: &'a mut Element,
    want: usize,
    seen: &mut usize,
) -> Option<&'a mut Element> {
    if el.local_name() == "AlternateContent" {
        if let Some(i) = shape::first_alt_child_index(el) {
            if let Node::Element(child) = &mut el.children[i] {
                return find_run_click_in(child, want, seen);
            }
        }
        return None;
    }
    if el.local_name() == "fld" {
        return None;
    }
    if el.local_name() == "r" {
        if el
            .child_by_local("rPr")
            .and_then(|rpr| rpr.child_by_local("hlinkClick"))
            .is_some()
        {
            *seen += 1;
            if *seen == want {
                return el
                    .child_by_local_mut("rPr")
                    .and_then(|rpr| rpr.child_by_local_mut("hlinkClick"));
            }
        }
        return None;
    }
    for child in el.children.as_mut_slice() {
        if let Node::Element(e) = child {
            if let Some(found) = find_run_click_in(e, want, seen) {
                return Some(found);
            }
        }
    }
    None
}

fn remove_click(root: &mut Element, index: usize) -> bool {
    let mut seen = 0usize;
    remove_click_in(root, index, &mut seen)
}

fn remove_click_in(el: &mut Element, want: usize, seen: &mut usize) -> bool {
    if el.local_name() == "AlternateContent" {
        if let Some(i) = shape::first_alt_child_index(el) {
            if let Node::Element(child) = &mut el.children[i] {
                return remove_click_in(child, want, seen);
            }
        }
        return false;
    }
    let is_text_sp = el.local_name() == "sp" && el.child_by_local("txBody").is_some();
    if is_text_sp {
        if hlink_click(el).is_some() {
            *seen += 1;
            if *seen == want {
                if let Some(cnv) = cnv_pr_mut(el) {
                    strip_hlink_click(cnv);
                    return true;
                }
                return false;
            }
        }
        if let Some(tx) = el.child_by_local_mut("txBody") {
            return remove_run_click_in(tx, want, seen);
        }
        return false;
    }
    for child in el.children.as_mut_slice() {
        if let Node::Element(e) = child {
            if remove_click_in(e, want, seen) {
                return true;
            }
        }
    }
    false
}

fn remove_run_click_in(el: &mut Element, want: usize, seen: &mut usize) -> bool {
    if el.local_name() == "AlternateContent" {
        if let Some(i) = shape::first_alt_child_index(el) {
            if let Node::Element(child) = &mut el.children[i] {
                return remove_run_click_in(child, want, seen);
            }
        }
        return false;
    }
    if el.local_name() == "fld" {
        return false;
    }
    if el.local_name() == "r" {
        if el
            .child_by_local("rPr")
            .and_then(|rpr| rpr.child_by_local("hlinkClick"))
            .is_some()
        {
            *seen += 1;
            if *seen == want {
                if let Some(rpr) = el.child_by_local_mut("rPr") {
                    strip_hlink_click(rpr);
                    return true;
                }
                return false;
            }
        }
        return false;
    }
    for child in el.children.as_mut_slice() {
        if let Node::Element(e) = child {
            if remove_run_click_in(e, want, seen) {
                return true;
            }
        }
    }
    false
}

fn insert_click(cnv: &mut Element, rid: &str, action: Option<&str>, tooltip: Option<&str>) {
    let mut click = Element::new("a:hlinkClick");
    click.set_attr("r:id", rid);
    if let Some(a) = action {
        click.set_attr("action", a);
    }
    if let Some(t) = tooltip {
        click.set_attr("tooltip", t);
    }
    let pos = cnv.children.iter().position(
        |n| matches!(n, Node::Element(e) if matches!(e.local_name(), "hlinkHover" | "extLst")),
    );
    match pos {
        Some(i) => cnv.children.insert(i, Node::Element(click)),
        None => cnv.push_element(click),
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
                .find(|s| s.index == *n)
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
        if !slides.iter().any(|s| s.index == *n) {
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
