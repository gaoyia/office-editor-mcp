//! 幻灯片 drawing 公式框：`a14:m` AlternateContent。
//! 不进 `shapes`。备注页可以插。批注内本期不插。

use serde_json::{json, Map, Value};

use crate::error::{Error, ErrorKind, IdReason, TypeName};
use crate::opc::{resolve_id, Package, Relationships};
use crate::xml::{Document, Element, Node};

use super::picture;
use super::shape;
use super::{DML_NS, REL_NS};

#[derive(Debug, Clone)]
pub enum MathKey {
    Id(String),
    Index(usize),
}

#[derive(Debug, Clone)]
pub struct MathTarget {
    pub slide: usize,
    pub on_notes: bool,
    pub master: Option<(usize, Option<usize>)>,
    pub key: MathKey,
}

#[derive(Debug, Clone)]
pub struct InsertMathOp {
    pub slide: usize,
    pub on_notes: bool,
    pub master: Option<(usize, Option<usize>)>,
    pub text: String,
    pub x: Option<i64>,
    pub y: Option<i64>,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct SetMathOp {
    pub id: String,
    pub target: MathTarget,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct DeleteMathOp {
    pub id: String,
    pub target: MathTarget,
}

#[derive(Debug, Clone)]
pub struct SetMathSizeOp {
    pub id: String,
    pub target: MathTarget,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct SetMathPosOp {
    pub id: String,
    pub target: MathTarget,
    pub x: Option<i64>,
    pub y: Option<i64>,
}

pub struct MathInfo {
    pub id: String,
    pub cnv_pr_id: Option<String>,
    pub name: String,
    pub x: Option<i64>,
    pub y: Option<i64>,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
    pub text: String,
    pub stable: bool,
}

pub fn looks_like_id(id: &str) -> bool {
    id.contains("/math:") && (id.starts_with("slide:") || id.starts_with("master:"))
}

pub fn parse_id(id: &str, json_path: &str) -> Result<MathTarget, Error> {
    if picture::is_bare_host(id) {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SlideMathContainer,
            },
            json_path.to_string(),
        ));
    }
    let (host, after) = picture::parse_object_id(id, json_path, "math", IdReason::SlideMathPrefix)?;
    if after.is_empty() {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SlideMathPrefix,
            },
            json_path.to_string(),
        ));
    }
    let (slide, on_notes, master) = host.parts();
    Ok(MathTarget {
        slide,
        on_notes,
        master,
        key: parse_key(after, json_path)?,
    })
}

fn parse_key(s: &str, path: &str) -> Result<MathKey, Error> {
    if let Some(rest) = s.strip_prefix("index:") {
        let n: usize = rest.parse().map_err(|_| {
            Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::SlideMathPrefix,
                },
                path.to_string(),
            )
        })?;
        if n < 1 {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::SlideMathPrefix,
                },
                path.to_string(),
            ));
        }
        return Ok(MathKey::Index(n));
    }
    if s.is_empty() || s.contains('/') {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SlideMathPrefix,
            },
            path.to_string(),
        ));
    }
    Ok(MathKey::Id(s.to_string()))
}

pub fn count(root: &Element) -> usize {
    collect(root, "").len()
}

pub fn collect(root: &Element, host: &str) -> Vec<MathInfo> {
    let mut out = Vec::new();
    let mut index = 0usize;
    let mut acs = Vec::new();
    collect_acs(root, &mut acs);
    for ac in acs {
        index += 1;
        out.push(info_of(ac, host, index));
    }
    out
}

fn collect_acs<'a>(el: &'a Element, out: &mut Vec<&'a Element>) {
    if el.local_name() == "AlternateContent" {
        if crate::drawing_math::is_math_ac(el) {
            out.push(el);
        }
        return;
    }
    for c in el.element_children() {
        collect_acs(c, out);
    }
}

fn info_of(ac: &Element, host: &str, index: usize) -> MathInfo {
    let (raw, name) = crate::drawing_math::cnv_of_ac(ac);
    let stable = raw.as_ref().is_some_and(|s| !s.is_empty());
    let id = match &raw {
        Some(v) if !v.is_empty() => format!("{host}/math:{v}"),
        _ => format!("{host}/math:index:{index}"),
    };
    let sp = ac
        .child_by_local("Choice")
        .and_then(crate::drawing_math::first_sp);
    let xfrm = sp.and_then(|s| {
        s.child_by_local("spPr")
            .and_then(|p| p.child_by_local("xfrm"))
    });
    let (x, y) = match xfrm.and_then(|x| x.child_by_local("off")) {
        Some(off) => (parse_i64(off, "x"), parse_i64(off, "y")),
        None => (None, None),
    };
    let (cx, cy) = match xfrm.and_then(|x| x.child_by_local("ext")) {
        Some(ext) => (parse_i64(ext, "cx"), parse_i64(ext, "cy")),
        None => (None, None),
    };
    MathInfo {
        id,
        cnv_pr_id: raw.filter(|s| !s.is_empty()),
        name,
        x,
        y,
        cx,
        cy,
        text: crate::drawing_math::choice_omml_text(ac),
        stable,
    }
}

fn parse_i64(el: &Element, name: &str) -> Option<i64> {
    el.attr(name)?.parse().ok()
}

pub fn list_json(items: &[MathInfo]) -> Vec<Value> {
    items.iter().map(info_json).collect()
}

fn info_json(p: &MathInfo) -> Value {
    let mut m = Map::new();
    m.insert("id".into(), json!(p.id));
    m.insert("cNvPrId".into(), json!(p.cnv_pr_id));
    m.insert("name".into(), json!(p.name));
    m.insert("x".into(), json!(p.x));
    m.insert("y".into(), json!(p.y));
    m.insert("cx".into(), json!(p.cx));
    m.insert("cy".into(), json!(p.cy));
    m.insert("text".into(), json!(p.text));
    m.insert("stable".into(), json!(p.stable));
    Value::Object(m)
}

pub fn insert(
    _pkg: &mut Package,
    slide_doc: &mut Document,
    _slide_part: &str,
    op: &InsertMathOp,
) -> Result<(), Error> {
    let (cx, cy) = match (op.cx, op.cy) {
        (Some(x), Some(y)) => (x, y),
        _ => (
            crate::drawing_math::DEFAULT_CX,
            crate::drawing_math::DEFAULT_CY,
        ),
    };
    let x = op.x.unwrap_or(0);
    let y = op.y.unwrap_or(0);
    picture::ensure_xmlns(&mut slide_doc.root, "r", REL_NS);
    picture::ensure_xmlns(&mut slide_doc.root, "a", DML_NS);
    picture::ensure_xmlns(&mut slide_doc.root, "mc", crate::drawing_math::MC_NS);
    picture::ensure_xmlns(&mut slide_doc.root, "a14", crate::drawing_math::A14_NS);
    picture::ensure_xmlns(&mut slide_doc.root, "m", crate::drawing_math::MATH_NS);
    crate::drawing_math::ensure_ignorable(&mut slide_doc.root, "a14");
    let cnv = picture::next_cnv_id(&slide_doc.root);
    let name = format!("公式 {cnv}");
    let ac = build_pptx_ac(cnv, &name, &op.text, x, y, cx, cy);
    picture::insert_into_sp_tree(&mut slide_doc.root, ac)?;
    Ok(())
}

fn build_pptx_ac(cnv: u32, name: &str, text: &str, x: i64, y: i64, cx: i64, cy: i64) -> Element {
    let mut ac = Element::new("mc:AlternateContent");
    let mut choice = Element::new("mc:Choice").with_attr("Requires", "a14");
    choice.push_element(build_pptx_sp(cnv, name, text, x, y, cx, cy, true));
    ac.push_element(choice);
    let mut fallback = Element::new("mc:Fallback");
    fallback.push_element(build_pptx_sp(cnv, name, text, x, y, cx, cy, false));
    ac.push_element(fallback);
    ac
}

fn build_pptx_sp(
    cnv: u32,
    name: &str,
    text: &str,
    x: i64,
    y: i64,
    cx: i64,
    cy: i64,
    math: bool,
) -> Element {
    let mut sp = Element::new("p:sp");
    let mut nv = Element::new("p:nvSpPr");
    nv.push_element(
        Element::new("p:cNvPr")
            .with_attr("id", cnv.to_string())
            .with_attr("name", name),
    );
    nv.push_element(Element::new("p:cNvSpPr").with_attr("txBox", "1"));
    nv.push_element(Element::new("p:nvPr"));
    sp.push_element(nv);

    let mut sp_pr = Element::new("p:spPr");
    let mut xfrm = Element::new("a:xfrm");
    xfrm.push_element(
        Element::new("a:off")
            .with_attr("x", x.to_string())
            .with_attr("y", y.to_string()),
    );
    xfrm.push_element(
        Element::new("a:ext")
            .with_attr("cx", cx.to_string())
            .with_attr("cy", cy.to_string()),
    );
    sp_pr.push_element(xfrm);
    let mut geom = Element::new("a:prstGeom").with_attr("prst", "rect");
    geom.push_element(Element::new("a:avLst"));
    sp_pr.push_element(geom);
    sp_pr.push_element(Element::new("a:noFill"));
    let mut ln = Element::new("a:ln");
    ln.push_element(Element::new("a:noFill"));
    sp_pr.push_element(ln);
    sp.push_element(sp_pr);

    let mut tx = Element::new("p:txBody");
    tx.push_element(
        Element::new("a:bodyPr")
            .with_attr("wrap", "square")
            .with_attr("lIns", "91440")
            .with_attr("tIns", "45720")
            .with_attr("rIns", "91440")
            .with_attr("bIns", "45720"),
    );
    tx.push_element(Element::new("a:lstStyle"));
    if math {
        tx.push_element(crate::drawing_math::math_p(text));
    } else {
        tx.push_element(crate::drawing_math::fallback_p(text));
    }
    sp.push_element(tx);
    sp
}

pub fn set(slide_doc: &mut Document, op: &SetMathOp, op_index: usize) -> Result<(), Error> {
    let path = format!("ops[{op_index}].id");
    if !set_in(&mut slide_doc.root, &op.target.key, &op.text) {
        return Err(Error::with_path(
            ErrorKind::MathDrawingNotFound { id: op.id.clone() },
            path,
        ));
    }
    Ok(())
}

fn find_ac_mut<'a>(
    el: &'a mut Element,
    key: &MathKey,
    index: &mut usize,
) -> Option<&'a mut Element> {
    crate::xml::search_mut(el, |el| {
        if el.local_name() == "AlternateContent" && crate::drawing_math::is_math_ac(el) {
            *index += 1;
            if key_matches(key, &crate::drawing_math::cnv_of_ac(el).0, *index) {
                return crate::xml::SearchMut::Hit;
            }
            return crate::xml::SearchMut::Skip;
        }
        crate::xml::SearchMut::down(el)
    })
}

fn choice_sp_mut(ac: &mut Element) -> Option<&mut Element> {
    let choice = ac.child_by_local_mut("Choice")?;
    crate::drawing_math::first_sp_mut(choice)
}

/// 写入 Choice 里该 `p:sp` 的 `a:xfrm/a:ext`。不改 Fallback。
pub fn set_size(
    slide_doc: &mut Document,
    op: &SetMathSizeOp,
    op_index: usize,
) -> Result<(), Error> {
    let path = format!("ops[{op_index}].id");
    let ac = find_ac_mut(&mut slide_doc.root, &op.target.key, &mut 0).ok_or_else(|| {
        Error::with_path(
            ErrorKind::MathDrawingNotFound { id: op.id.clone() },
            path.clone(),
        )
    })?;
    let sp = choice_sp_mut(ac).ok_or_else(|| {
        Error::with_path(ErrorKind::MathDrawingNotFound { id: op.id.clone() }, path)
    })?;
    shape::set_sp_size(sp, op.cx, op.cy);
    Ok(())
}

/// 写入 Choice 里该 `p:sp` 的 `a:xfrm/a:off`。不改 Fallback。
pub fn set_pos(slide_doc: &mut Document, op: &SetMathPosOp, op_index: usize) -> Result<(), Error> {
    let path = format!("ops[{op_index}].id");
    let ac = find_ac_mut(&mut slide_doc.root, &op.target.key, &mut 0).ok_or_else(|| {
        Error::with_path(
            ErrorKind::MathDrawingNotFound { id: op.id.clone() },
            path.clone(),
        )
    })?;
    let sp = choice_sp_mut(ac).ok_or_else(|| {
        Error::with_path(ErrorKind::MathDrawingNotFound { id: op.id.clone() }, path)
    })?;
    shape::set_sp_pos(sp, op.x, op.y);
    Ok(())
}

fn set_in(el: &mut Element, key: &MathKey, text: &str) -> bool {
    set_in_index(el, key, text, &mut 0)
}

fn set_in_index(el: &mut Element, key: &MathKey, text: &str, index: &mut usize) -> bool {
    if el.local_name() == "AlternateContent" && crate::drawing_math::is_math_ac(el) {
        *index += 1;
        if key_matches(key, &crate::drawing_math::cnv_of_ac(el).0, *index) {
            return crate::drawing_math::set_ac_text(el, text);
        }
        return false;
    }
    for n in el.children.iter_mut() {
        if let Node::Element(child) = n {
            if set_in_index(child, key, text, index) {
                return true;
            }
        }
    }
    false
}

fn key_matches(key: &MathKey, raw: &Option<String>, index: usize) -> bool {
    match key {
        MathKey::Index(n) => index == *n,
        MathKey::Id(want) => {
            let Some(got) = raw else {
                return false;
            };
            if let (Ok(a), Ok(b)) = (want.parse::<u32>(), got.parse::<u32>()) {
                a == b
            } else {
                want == got
            }
        }
    }
}

pub fn delete(
    pkg: &mut Package,
    slide_doc: &mut Document,
    slide_part: &str,
    op: &DeleteMathOp,
    op_index: usize,
) -> Result<(), Error> {
    let path = format!("ops[{op_index}].id");
    let ac = find_ac(&slide_doc.root, &op.target.key).ok_or_else(|| {
        Error::with_path(
            ErrorKind::MathDrawingNotFound { id: op.id.clone() },
            path.clone(),
        )
    })?;
    let spid = crate::drawing_math::cnv_of_ac(ac)
        .0
        .filter(|s| !s.is_empty());
    let rids = crate::drawing_math::collect_embed_rids(ac);
    let mut old_parts = Vec::new();
    for rid in &rids {
        if let Ok(p) = resolve_id(pkg, slide_part, rid) {
            if !old_parts.iter().any(|x| x == &p) {
                old_parts.push(p);
            }
        }
    }
    if !remove_math(&mut slide_doc.root, &op.target.key) {
        return Err(Error::with_path(
            ErrorKind::MathDrawingNotFound { id: op.id.clone() },
            path,
        ));
    }
    if let Some(id) = spid {
        shape::strip_timing_for_spid(&mut slide_doc.root, &id);
    }
    let mut rels = Relationships::from_package(pkg, slide_part)?;
    let mut dirty = false;
    for rid in &rids {
        if !rid_still_used(&slide_doc.root, rid) {
            rels.remove_by_id(rid);
            dirty = true;
        }
    }
    if dirty {
        rels.write_to(pkg, slide_part)?;
    }
    for part in old_parts {
        if !picture::media_still_used(pkg, &part)? {
            pkg.remove(&part);
        }
    }
    Ok(())
}

fn find_ac<'a>(root: &'a Element, key: &MathKey) -> Option<&'a Element> {
    let mut acs = Vec::new();
    collect_acs(root, &mut acs);
    for (i, ac) in acs.into_iter().enumerate() {
        if key_matches(key, &crate::drawing_math::cnv_of_ac(ac).0, i + 1) {
            return Some(ac);
        }
    }
    None
}

fn remove_math(el: &mut Element, key: &MathKey) -> bool {
    remove_in(el, key, &mut 0)
}

fn remove_in(el: &mut Element, key: &MathKey, index: &mut usize) -> bool {
    let mut drop = None;
    for (i, n) in el.children.iter_mut().enumerate() {
        let Node::Element(child) = n else { continue };
        if child.local_name() == "AlternateContent" && crate::drawing_math::is_math_ac(child) {
            *index += 1;
            if key_matches(key, &crate::drawing_math::cnv_of_ac(child).0, *index) {
                drop = Some(i);
                break;
            }
        } else if remove_in(child, key, index) {
            return true;
        }
    }
    if let Some(i) = drop {
        el.children.remove(i);
        return true;
    }
    false
}

fn rid_still_used(root: &Element, rid: &str) -> bool {
    let mut used = false;
    crate::model3d::walk(root, &mut |e| {
        if e.attr_local("embed") == Some(rid) {
            used = true;
        }
    });
    used
}

pub(crate) fn parse_create(
    v: &Value,
    prefix: &str,
    slide: usize,
    on_notes: bool,
) -> Result<InsertMathOp, Error> {
    let obj = v.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: prefix.into(),
                expected: TypeName::Object,
            },
            prefix,
        )
    })?;
    crate::json::deny_unknown(obj, &["text", "x", "y", "cx", "cy", "expr", "kind"])?;
    crate::drawing_math::reject_expr_kind(obj, prefix)?;
    let text = crate::drawing_math::require_text(obj, prefix)?;
    let cx = parse_emu(obj.get("cx"), prefix, "cx")?;
    let cy = parse_emu(obj.get("cy"), prefix, "cy")?;
    if cx.is_some() != cy.is_some() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "insertMath".into(),
                field: "cx and cy together".into(),
            },
            prefix.to_string(),
        ));
    }
    Ok(InsertMathOp {
        slide,
        on_notes,
        master: None,
        text,
        x: parse_emu(obj.get("x"), prefix, "x")?,
        y: parse_emu(obj.get("y"), prefix, "y")?,
        cx,
        cy,
    })
}

fn parse_emu(v: Option<&Value>, prefix: &str, field: &str) -> Result<Option<i64>, Error> {
    match v {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(n)) => n
            .as_i64()
            .ok_or_else(|| {
                Error::with_path(
                    ErrorKind::WrongType {
                        field: field.into(),
                        expected: TypeName::Integer,
                    },
                    format!("{prefix}.{field}"),
                )
            })
            .map(Some),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Integer,
            },
            format!("{prefix}.{field}"),
        )),
    }
}
