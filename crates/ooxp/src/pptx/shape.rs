//! 幻灯片里的文本形状：定位、可见文字、AlternateContent 只走一支。

use serde_json::{json, Value};

use crate::error::{Error, ErrorKind, IdReason};
use crate::opc::{self, Package};
use crate::xml::{Document, Element, Node};

use super::picture;
use super::request::{
    InsertShapeOp, SetShapeFillOp, SetShapeGeomOp, SetShapeLineOp, SetShapePosOp, SetShapeSizeOp,
};
use super::DML_NS;

pub struct SlideMeta {
    /// 当前在 `sldIdLst` 里的位置，从 1 起。
    pub index: usize,
    /// apply 开始时的页号。本批新建的页为 `None`。请求里的 `slide:{n}` 按它找页。
    pub start: Option<usize>,
    pub part: String,
    pub name: Option<String>,
    pub hidden: bool,
    /// `p:sldId` 的无前缀 `id`（`ST_SlideId`），不是 `r:id`。
    pub sld_id: Option<u32>,
}

#[derive(Debug, Clone)]
pub enum ShapeKey {
    Id(String),
    Index(usize),
}

#[derive(Debug, Clone)]
pub struct ShapeTarget {
    pub slide: usize,
    pub on_notes: bool,
    pub master: Option<(usize, Option<usize>)>,
    pub key: ShapeKey,
}

pub fn parse_shape_id(id: &str, op_index: usize) -> Result<ShapeTarget, Error> {
    let path = format!("ops[{op_index}].id");
    let rest = id.strip_prefix("slide:").ok_or_else(|| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::ShapePrefix,
            },
            path.clone(),
        )
    })?;
    let (n, after, on_notes) = if let Some((n, after)) = rest.split_once("/notes/shape:") {
        (n, after, true)
    } else if let Some((n, after)) = rest.split_once("/shape:") {
        (n, after, false)
    } else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::ShapeForm,
            },
            path,
        ));
    };
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
    if after.is_empty() {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::ShapeEmpty,
            },
            path,
        ));
    }
    if let Some(k) = after.strip_prefix("index:") {
        let idx: usize = k.parse().map_err(|_| {
            Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::ShapeIndexNotInt,
                },
                path.clone(),
            )
        })?;
        if idx < 1 {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::ShapeIndexFromOne,
                },
                path,
            ));
        }
        return Ok(ShapeTarget {
            slide,
            on_notes,
            master: None,
            key: ShapeKey::Index(idx),
        });
    }
    Ok(ShapeTarget {
        slide,
        on_notes,
        master: None,
        key: ShapeKey::Id(after.to_string()),
    })
}

/// `r:id` / `rel:id`：要的是**带前缀**的那个 id。
/// 这里不能用 `attr_local("id")`：`p:sldId` 同时带无前缀的 `id`（幻灯片编号）
/// 和 `r:id`（关系），local name 都是 `id`，只比 local name 会命中前者。
pub fn rid_attr(el: &Element) -> Option<&str> {
    el.attrs.iter().find_map(|(k, v)| {
        if k.contains(':') && k.rsplit(':').next() == Some("id") {
            Some(v.as_str())
        } else {
            None
        }
    })
}

/// apply 开始时各页的 `p:sldId/@id`。增删重排后重新列页，用它把 `start` 标回去。
#[derive(Clone, Debug)]
pub struct SlideFreeze {
    ids: Vec<Option<u32>>,
}

impl SlideFreeze {
    pub fn capture(slides: &[SlideMeta]) -> Self {
        Self {
            ids: slides.iter().map(|slide| slide.sld_id).collect(),
        }
    }

    pub fn mark_start(&self, slides: &mut [SlideMeta]) {
        for slide in slides.iter_mut() {
            slide.start = slide.sld_id.and_then(|id| {
                self.ids
                    .iter()
                    .position(|saved| *saved == Some(id))
                    .map(|pos| pos + 1)
            });
        }
    }
}

/// 按 apply 开始时的页号取页。
pub fn slide_at_start(slides: &[SlideMeta], n: usize) -> Option<&SlideMeta> {
    slides.iter().find(|slide| slide.start == Some(n))
}

/// apply 开始时第 `n` 页现在排在第几位，从 0 起。
pub fn pos_at_start(slides: &[SlideMeta], n: usize) -> Option<usize> {
    slides.iter().position(|slide| slide.start == Some(n))
}

pub fn list_slides(
    pkg: &Package,
    pres_part: &str,
    pres_root: &Element,
) -> Result<Vec<SlideMeta>, Error> {
    let Some(lst) = pres_root.child_by_local("sldIdLst") else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for (i, sld_id) in lst
        .element_children()
        .filter(|e| e.local_name() == "sldId")
        .enumerate()
    {
        let rid = rid_attr(sld_id)
            .ok_or_else(|| Error::new(ErrorKind::SlideRelMissing { index: i + 1 }))?;
        let part = opc::resolve_id(pkg, pres_part, rid)?;
        let doc = super::list_pin::parse_part(pkg, &part)?;
        let name = doc
            .root
            .child_by_local("cSld")
            .and_then(|c| c.attr("name"))
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        let hidden = matches!(
            doc.root.attr("show"),
            Some("0") | Some("false") | Some("FALSE")
        );
        let numeric_id = sld_id.attr("id").and_then(|s| s.parse().ok());
        out.push(SlideMeta {
            index: i + 1,
            start: Some(i + 1),
            part,
            name,
            hidden,
            sld_id: numeric_id,
        });
    }
    Ok(out)
}

pub fn collect_text_shapes(root: &Element) -> Vec<&Element> {
    let mut out = Vec::new();
    walk_choice(root, &mut |e| {
        if e.local_name() == "sp" && e.child_by_local("txBody").is_some() {
            out.push(e);
        }
    });
    out
}

pub fn cnv_pr(sp: &Element) -> Option<&Element> {
    sp.child_by_local("nvSpPr")
        .and_then(|nv| nv.child_by_local("cNvPr"))
}

pub fn shape_stable_id(sp: &Element, host: &str, index: usize) -> (String, Option<String>, bool) {
    if let Some(id) = cnv_pr(sp)
        .and_then(|c| c.attr("id"))
        .filter(|s| !s.is_empty())
    {
        (format!("{host}/shape:{id}"), Some(id.to_string()), true)
    } else {
        (format!("{host}/shape:index:{index}"), None, false)
    }
}

pub fn shape_name(sp: &Element) -> Option<String> {
    cnv_pr(sp)
        .and_then(|c| c.attr("name"))
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

pub fn shape_node_json(sp: &Element, host: &str, index: usize) -> Value {
    let (id, shape_id, stable) = shape_stable_id(sp, host, index);
    let (x, y, cx, cy) = geometry_json(sp);
    let mut sh = serde_json::Map::new();
    sh.insert("id".into(), json!(id));
    sh.insert("shapeId".into(), shape_id.map_or(Value::Null, |s| json!(s)));
    sh.insert(
        "name".into(),
        shape_name(sp).map_or(Value::Null, |n| json!(n)),
    );
    sh.insert("prst".into(), prst_json(sp));
    if let Some(geom) = super::geom::inspect_json(sp) {
        sh.insert("geom".into(), geom);
    }
    sh.insert("x".into(), x);
    sh.insert("y".into(), y);
    sh.insert("cx".into(), cx);
    sh.insert("cy".into(), cy);
    sh.insert("text".into(), json!(crate::drawing_text::visible_text(sp)));
    crate::drawing_text::insert_inspect(&mut sh, sp.child_by_local("txBody"));
    sh.insert("stable".into(), json!(stable));
    Value::Object(sh)
}

pub(crate) fn prst_json(el: &Element) -> Value {
    el.child_by_local("spPr")
        .and_then(|p| p.child_by_local("prstGeom"))
        .and_then(|g| g.attr_local("prst"))
        .filter(|s| !s.is_empty())
        .map_or(Value::Null, |s| json!(s))
}

pub(crate) fn geometry_json(sp: &Element) -> (Value, Value, Value, Value) {
    let xfrm = sp
        .child_by_local("spPr")
        .and_then(|p| p.child_by_local("xfrm"));
    let off = xfrm.and_then(|x| x.child_by_local("off"));
    let ext = xfrm.and_then(|x| x.child_by_local("ext"));
    (
        json_i64_attr(off, "x"),
        json_i64_attr(off, "y"),
        json_i64_attr(ext, "cx"),
        json_i64_attr(ext, "cy"),
    )
}

fn json_i64_attr(el: Option<&Element>, name: &str) -> Value {
    match el
        .and_then(|e| e.attr(name))
        .and_then(|s| s.parse::<i64>().ok())
    {
        Some(n) => json!(n),
        None => Value::Null,
    }
}

pub fn find_shape_mut<'a>(root: &'a mut Element, target: &ShapeKey) -> Option<&'a mut Element> {
    let mut idx = 0usize;
    find_sp_mut(root, &mut idx, target)
}

fn find_sp_mut<'a>(
    el: &'a mut Element,
    idx: &mut usize,
    target: &ShapeKey,
) -> Option<&'a mut Element> {
    crate::xml::search_mut(el, |el| {
        if el.local_name() == "AlternateContent" {
            if let Some(i) = first_alt_child_index(el) {
                if matches!(&el.children[i], Node::Element(_)) {
                    return crate::xml::SearchMut::Only(i);
                }
            }
            return crate::xml::SearchMut::Skip;
        }
        if el.local_name() == "sp" && el.child_by_local("txBody").is_some() {
            *idx += 1;
            if match target {
                ShapeKey::Index(n) => {
                    crate::xml::pin_matches(el, &crate::xml::pin_attr("shape"), *n, *idx)
                }
                ShapeKey::Id(req) => cnv_pr(el)
                    .and_then(|c| c.attr("id"))
                    .is_some_and(|id| ids_match(id, req)),
            } {
                return crate::xml::SearchMut::Hit;
            }
            return crate::xml::SearchMut::Skip;
        }
        crate::xml::SearchMut::down(el)
    })
}

pub(crate) fn ids_match(file: &str, req: &str) -> bool {
    match (file.parse::<u64>(), req.parse::<u64>()) {
        (Ok(a), Ok(b)) => a == b,
        _ => file == req,
    }
}

/// 遇到 `mc:AlternateContent` 进 `alternate_child` 选出的那一支。
/// 公式框整段不进 `shapes`。
pub fn walk_choice<'a>(root: &'a Element, f: &mut impl FnMut(&'a Element)) {
    let mut stack = vec![root];
    while let Some(el) = stack.pop() {
        if el.local_name() == "AlternateContent" {
            if crate::drawing_math::is_math_ac(el) {
                continue;
            }
            if let Some(i) = first_alt_child_index(el) {
                if let Node::Element(child) = &el.children[i] {
                    stack.push(child);
                }
            }
            continue;
        }
        f(el);
        if el.local_name() == "oleObj" {
            continue;
        }
        for node in el.children.iter().rev() {
            if let Node::Element(child) = node {
                stack.push(child);
            }
        }
    }
}

pub(crate) fn first_alt_child_index(el: &Element) -> Option<usize> {
    crate::xml::alternate_child(el).unwrap_or(None)
}

const DEFAULT_SHAPE_CX: i64 = 1_828_800;
const DEFAULT_SHAPE_CY: i64 = 914_400;

fn is_counted_shape(el: &Element) -> bool {
    el.local_name() == "sp" && el.child_by_local("txBody").is_some()
}

pub(crate) fn pin_shapes(root: &mut Element) {
    crate::xml::pin_counted(root, "shape", is_counted_shape, |_| false);
}

fn shape_matches(el: &Element, index: usize, key: &ShapeKey) -> bool {
    match key {
        ShapeKey::Index(n) => {
            crate::xml::pin_matches(el, &crate::xml::pin_attr("shape"), *n, index)
        }
        ShapeKey::Id(req) => cnv_pr(el)
            .and_then(|c| c.attr("id"))
            .is_some_and(|id| ids_match(id, req)),
    }
}

fn counted_spid(el: &Element) -> Option<String> {
    cnv_pr(el)
        .and_then(|c| c.attr("id"))
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

/// 删掉命中的文本形状。返回 `None` 表示没找到；`Some(spid)` 里 `spid` 可能仍是 `None`（没有 `cNvPr/@id`）。
pub fn try_remove_shape(root: &mut Element, key: &ShapeKey) -> Option<Option<String>> {
    let mut idx = 0usize;
    let mut spid = None;
    if !remove_shape_in(root, &mut idx, key, &mut spid) {
        return None;
    }
    prune_empty_groups(root);
    Some(spid)
}

fn remove_shape_in(
    el: &mut Element,
    idx: &mut usize,
    key: &ShapeKey,
    spid: &mut Option<String>,
) -> bool {
    let mut i = 0;
    while i < el.children.len() {
        let hit_ac = match &el.children[i] {
            Node::Element(e) if e.local_name() == "AlternateContent" => {
                scan_alt_for_hit(e, idx, key)
            }
            _ => None,
        };
        if let Some(hit) = hit_ac {
            *spid = hit;
            el.children.remove(i);
            return true;
        }
        let is_hit = match &el.children[i] {
            Node::Element(e) if is_counted_shape(e) => {
                *idx += 1;
                if shape_matches(e, *idx, key) {
                    *spid = counted_spid(e);
                    true
                } else {
                    false
                }
            }
            _ => false,
        };
        if is_hit {
            el.children.remove(i);
            return true;
        }
        if let Node::Element(e) = &mut el.children[i] {
            if e.local_name() != "AlternateContent" && remove_shape_in(e, idx, key, spid) {
                return true;
            }
        }
        i += 1;
    }
    false
}

/// 扫描 Choice 里的文本形状：命中则返回该形状的 `cNvPr/@id`，并把 `idx` 计到命中处（后续形状不再计，因为整段 AC 会删掉）。
fn scan_alt_for_hit(ac: &Element, idx: &mut usize, key: &ShapeKey) -> Option<Option<String>> {
    let mut hit: Option<Option<String>> = None;
    walk_choice(ac, &mut |e| {
        if hit.is_some() || !is_counted_shape(e) {
            return;
        }
        *idx += 1;
        if shape_matches(e, *idx, key) {
            hit = Some(counted_spid(e));
        }
    });
    hit
}

pub(crate) fn prune_empty_groups(el: &mut Element) {
    for n in &mut el.children {
        if let Node::Element(e) = n {
            prune_empty_groups(e);
        }
    }
    el.children.retain(
        |n| !matches!(n, Node::Element(e) if e.local_name() == "grpSp" && !grp_has_member(e)),
    );
}

fn grp_has_member(grp: &Element) -> bool {
    grp.element_children().any(|c| {
        matches!(
            c.local_name(),
            "sp" | "pic" | "cxnSp" | "graphicFrame" | "grpSp" | "AlternateContent"
        )
    })
}

pub fn insert_shape(
    doc: &mut Document,
    canvas: (i64, i64),
    op: &InsertShapeOp,
) -> Result<(), Error> {
    let cx = op.cx.unwrap_or(DEFAULT_SHAPE_CX);
    let cy = op.cy.unwrap_or(DEFAULT_SHAPE_CY);
    let (x, y) = match (op.x, op.y) {
        (Some(px), Some(py)) => (px, py),
        _ => picture::vacant_pos_canvas(canvas, &doc.root, cx, cy),
    };
    picture::ensure_xmlns(&mut doc.root, "a", DML_NS);
    let cnv = picture::next_cnv_id(&doc.root);
    let is_box = op.geom.is_none() && op.prst == "rect";
    let name = if is_box {
        format!("文本框 {cnv}")
    } else {
        format!("形状 {cnv}")
    };
    let mut sp = build_sp(
        cnv,
        &name,
        &op.prst,
        op.geom.as_deref(),
        is_box,
        x,
        y,
        cx,
        cy,
    );
    if let Some(tx) = sp.child_by_local_mut("txBody") {
        crate::drawing_text::set_txbody_text(tx, &op.text);
        if let Some(fmt) = &op.format {
            crate::drawing_text::apply(tx, fmt);
        }
    }
    picture::insert_into_sp_tree(&mut doc.root, sp)?;
    Ok(())
}

fn build_sp(
    cnv: u32,
    name: &str,
    prst: &str,
    geom: Option<&[super::geom::Path]>,
    is_box: bool,
    x: i64,
    y: i64,
    cx: i64,
    cy: i64,
) -> Element {
    let mut sp = Element::new("p:sp");
    let mut nv = Element::new("p:nvSpPr");
    nv.push_element(
        Element::new("p:cNvPr")
            .with_attr("id", cnv.to_string())
            .with_attr("name", name),
    );
    if is_box {
        nv.push_element(Element::new("p:cNvSpPr").with_attr("txBox", "1"));
    } else {
        nv.push_element(Element::new("p:cNvSpPr"));
    }
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
    if let Some(paths) = geom {
        sp_pr.push_element(super::geom::build_cust_geom(paths));
    } else {
        let mut geom_el = Element::new("a:prstGeom").with_attr("prst", prst);
        geom_el.push_element(Element::new("a:avLst"));
        sp_pr.push_element(geom_el);
    }
    if is_box {
        sp_pr.push_element(Element::new("a:noFill"));
        let mut ln = Element::new("a:ln");
        ln.push_element(Element::new("a:noFill"));
        sp_pr.push_element(ln);
    } else {
        let mut fill = Element::new("a:solidFill");
        fill.push_element(Element::new("a:srgbClr").with_attr("val", "5B9BD5"));
        sp_pr.push_element(fill);
        let mut ln = Element::new("a:ln").with_attr("w", "12700");
        let mut ln_fill = Element::new("a:solidFill");
        ln_fill.push_element(Element::new("a:srgbClr").with_attr("val", "2E75B6"));
        ln.push_element(ln_fill);
        sp_pr.push_element(ln);
    }
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
    sp.push_element(tx);
    sp
}

pub fn set_shape_pos(doc: &mut Document, op: &SetShapePosOp, op_index: usize) -> Result<(), Error> {
    let target = op.target.as_ref().expect("幻灯片形状");
    let sp = find_shape_mut(&mut doc.root, &target.key).ok_or_else(|| {
        Error::with_path(
            ErrorKind::ShapeNotFound { id: op.id.clone() },
            format!("ops[{op_index}].id"),
        )
    })?;
    set_sp_pos(sp, op.x, op.y);
    Ok(())
}

pub fn set_shape_size(
    doc: &mut Document,
    op: &SetShapeSizeOp,
    op_index: usize,
) -> Result<(), Error> {
    let target = op.target.as_ref().expect("幻灯片形状");
    let sp = find_shape_mut(&mut doc.root, &target.key).ok_or_else(|| {
        Error::with_path(
            ErrorKind::ShapeNotFound { id: op.id.clone() },
            format!("ops[{op_index}].id"),
        )
    })?;
    set_sp_size(sp, op.cx, op.cy);
    Ok(())
}

pub(crate) fn set_sp_pos(sp: &mut Element, x: Option<i64>, y: Option<i64>) {
    let xfrm = ensure_xfrm(sp);
    let off = ensure_a_child(xfrm, "off");
    if let Some(x) = x {
        picture::set_attr_local(off, "x", &x.to_string());
    }
    if let Some(y) = y {
        picture::set_attr_local(off, "y", &y.to_string());
    }
}

pub(crate) fn set_sp_size(sp: &mut Element, cx: Option<i64>, cy: Option<i64>) {
    let xfrm = ensure_xfrm(sp);
    let ext = ensure_a_child(xfrm, "ext");
    if let Some(cx) = cx {
        picture::set_attr_local(ext, "cx", &cx.to_string());
    }
    if let Some(cy) = cy {
        picture::set_attr_local(ext, "cy", &cy.to_string());
    }
}

pub fn set_shape_fill(
    doc: &mut Document,
    op: &SetShapeFillOp,
    op_index: usize,
) -> Result<(), Error> {
    let target = op.target.as_ref().expect("幻灯片形状");
    let sp = find_shape_mut(&mut doc.root, &target.key).ok_or_else(|| {
        Error::with_path(
            ErrorKind::ShapeNotFound { id: op.id.clone() },
            format!("ops[{op_index}].id"),
        )
    })?;
    apply_fill(sp, op.color.as_deref());
    Ok(())
}

pub(crate) fn apply_fill(sp: &mut Element, color: Option<&str>) {
    let sp_pr = picture::ensure_child(sp, "spPr");
    remove_named(
        sp_pr,
        &[
            "noFill",
            "solidFill",
            "gradFill",
            "blipFill",
            "pattFill",
            "grpFill",
        ],
    );
    match color {
        Some(c) => {
            let mut fill = Element::new("a:solidFill");
            fill.push_element(Element::new("a:srgbClr").with_attr("val", c.to_ascii_uppercase()));
            sp_pr.push_element(fill);
        }
        None => sp_pr.push_element(Element::new("a:noFill")),
    }
}

pub fn set_shape_line(
    doc: &mut Document,
    op: &SetShapeLineOp,
    op_index: usize,
) -> Result<(), Error> {
    let target = op.target.as_ref().expect("形状");
    let sp = find_shape_mut(&mut doc.root, &target.key).ok_or_else(|| {
        Error::with_path(
            ErrorKind::ShapeNotFound { id: op.id.clone() },
            format!("ops[{op_index}].id"),
        )
    })?;
    apply_line(sp, op.color.as_deref(), op.width);
    Ok(())
}

pub fn set_shape_geom(
    doc: &mut Document,
    op: &SetShapeGeomOp,
    op_index: usize,
) -> Result<(), Error> {
    let target = op.target.as_ref().expect("幻灯片形状");
    let sp = find_shape_mut(&mut doc.root, &target.key).ok_or_else(|| {
        Error::with_path(
            ErrorKind::ShapeNotFound { id: op.id.clone() },
            format!("ops[{op_index}].id"),
        )
    })?;
    apply_geom(sp, &op.paths);
    Ok(())
}

pub(crate) fn apply_geom(sp: &mut Element, paths: &[super::geom::Path]) {
    super::geom::apply_to_sp(sp, paths);
}

pub(crate) fn apply_line(el: &mut Element, color: Option<&str>, width: Option<i64>) {
    let sp_pr = picture::ensure_child(el, "spPr");
    remove_named(sp_pr, &["ln"]);
    let mut ln = Element::new("a:ln");
    if let Some(w) = width {
        ln.set_attr("w", w.to_string());
    }
    match color {
        Some(c) => {
            let mut fill = Element::new("a:solidFill");
            fill.push_element(Element::new("a:srgbClr").with_attr("val", c.to_ascii_uppercase()));
            ln.push_element(fill);
        }
        None => ln.push_element(Element::new("a:noFill")),
    }
    sp_pr.push_element(ln);
}

fn ensure_xfrm(sp: &mut Element) -> &mut Element {
    let sp_pr = picture::ensure_child(sp, "spPr");
    ensure_a_child(sp_pr, "xfrm")
}

fn ensure_a_child<'a>(parent: &'a mut Element, local: &str) -> &'a mut Element {
    if parent.child_by_local(local).is_none() {
        parent.push_element(Element::new(format!("a:{local}")));
    }
    parent.child_by_local_mut(local).expect("刚插入的子元素")
}

fn remove_named(parent: &mut Element, locals: &[&str]) {
    parent.children.retain(|n| match n {
        Node::Element(e) => !locals.contains(&e.local_name()),
        _ => true,
    });
}

pub fn strip_timing_for_spid(slide_root: &mut Element, spid: &str) {
    let mut i = 0;
    while i < slide_root.children.len() {
        let drop = match &mut slide_root.children[i] {
            Node::Element(e) if e.local_name() == "timing" => {
                remove_matching_tgt_el(e, spid);
                prune_anim(e);
                e.element_children().next().is_none()
            }
            _ => false,
        };
        if drop {
            slide_root.children.remove(i);
            continue;
        }
        i += 1;
    }
}

fn remove_matching_tgt_el(el: &mut Element, spid: &str) {
    el.children.retain(
        |n| !matches!(n, Node::Element(e) if e.local_name() == "tgtEl" && contains_sp_tgt(e, spid)),
    );
    for n in &mut el.children {
        if let Node::Element(e) = n {
            remove_matching_tgt_el(e, spid);
        }
    }
}

fn contains_sp_tgt(el: &Element, spid: &str) -> bool {
    if el.local_name() == "spTgt" {
        if let Some(v) = el.attr("spid") {
            return ids_match(v, spid);
        }
    }
    el.element_children().any(|c| contains_sp_tgt(c, spid))
}

pub(crate) fn prune_anim(el: &mut Element) {
    for _ in 0..8 {
        prune_anim_pass(el);
    }
}

fn prune_anim_pass(el: &mut Element) {
    for n in &mut el.children {
        if let Node::Element(e) = n {
            prune_anim_pass(e);
        }
    }
    let parent = el.local_name().to_string();
    el.children.retain(|n| {
        let Node::Element(e) = n else {
            return true;
        };
        match e.local_name() {
            "cBhvr" => e.child_by_local("tgtEl").is_some(),
            "anim" | "animEffect" | "animMotion" | "animRot" | "animScale" | "animClr" | "set" => {
                e.child_by_local("cBhvr").is_some() || e.child_by_local("childTnLst").is_some()
            }
            "childTnLst" | "tnLst" | "seq" | "par" | "bldLst" => {
                e.element_children().next().is_some()
            }
            "cTn" => {
                if parent == "cBhvr" {
                    true
                } else {
                    e.element_children().next().is_some()
                }
            }
            _ => true,
        }
    });
}
