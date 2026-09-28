//! 幻灯片里的文本形状：定位、可见文字、AlternateContent 只走一支。

use serde_json::{json, Value};

use crate::error::{Error, ErrorKind, IdReason};
use crate::opc::{self, Package};
use crate::xml::{Document, Element, Node};

use super::picture;
use super::request::{
    InsertShapeOp, SetShapeFillOp, SetShapeLineOp, SetShapePosOp, SetShapeSizeOp,
};
use super::DML_NS;

pub struct SlideMeta {
    pub index: usize,
    pub part: String,
    pub name: Option<String>,
    pub hidden: bool,
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
            key: ShapeKey::Index(idx),
        });
    }
    Ok(ShapeTarget {
        slide,
        on_notes,
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
        let doc = crate::xml::parse(pkg.get(&part)?)?;
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
        out.push(SlideMeta {
            index: i + 1,
            part,
            name,
            hidden,
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
    sh.insert("x".into(), x);
    sh.insert("y".into(), y);
    sh.insert("cx".into(), cx);
    sh.insert("cy".into(), cy);
    sh.insert("text".into(), json!(visible_text(sp)));
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

pub fn visible_text(sp: &Element) -> String {
    let Some(tx) = sp.child_by_local("txBody") else {
        return String::new();
    };
    txbody_visible_text(tx)
}

pub fn txbody_visible_text(tx: &Element) -> String {
    let mut out = String::new();
    collect_visible_text(tx, &mut out);
    out
}

/// 只收 `a:r` 里的 `a:t` 和段落级 `a:br`。
/// `a:fld` 是生成文本（§21.1.2.2.4），缓存的 `a:t` 不进 inspect 的 `text`。
fn collect_visible_text(el: &Element, out: &mut String) {
    if el.local_name() == "AlternateContent" {
        if let Some(i) = first_alt_child_index(el) {
            if let Node::Element(child) = &el.children[i] {
                collect_visible_text(child, out);
            }
        }
        return;
    }
    if el.local_name() == "fld" {
        return;
    }
    match el.local_name() {
        "t" => out.push_str(&el.text_content()),
        "br" => out.push('\n'),
        _ => {}
    }
    for n in &el.children {
        if let Node::Element(e) = n {
            collect_visible_text(e, out);
        }
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
    if el.local_name() == "AlternateContent" {
        if let Some(i) = first_alt_child_index(el) {
            if let Node::Element(child) = &mut el.children[i] {
                return find_sp_mut(child, idx, target);
            }
        }
        return None;
    }
    if el.local_name() == "sp" && el.child_by_local("txBody").is_some() {
        *idx += 1;
        let hit = match target {
            ShapeKey::Index(n) => *idx == *n,
            ShapeKey::Id(req) => cnv_pr(el)
                .and_then(|c| c.attr("id"))
                .is_some_and(|id| ids_match(id, req)),
        };
        if hit {
            return Some(el);
        }
        return None;
    }
    for child in el.children.as_mut_slice() {
        if let Node::Element(e) = child {
            if let Some(found) = find_sp_mut(e, idx, target) {
                return Some(found);
            }
        }
    }
    None
}

pub(crate) fn ids_match(file: &str, req: &str) -> bool {
    match (file.parse::<u64>(), req.parse::<u64>()) {
        (Ok(a), Ok(b)) => a == b,
        _ => file == req,
    }
}

/// 按 `docs/json-contract.md` 的替换规则原地改：第一个有文本 run 的段落收下全部新文本，
/// 其余段落清掉可见内容；清完后只剩 `a:pPr` / `a:endParaRPr` 的段删掉。
///
/// 换行必须在段落这一级处理。`a:br` 与 `a:r` 平级（`EG_TextRun`，ECMA-376 §A.4），
/// 而 `CT_RegularTextRun` 只允许一个可选 `a:rPr` 加恰好一个 `a:t`，
/// 所以既不能把 `a:br` 塞进 run 里，也不能让一个 run 带多个 `a:t`。
pub fn set_txbody_text(tx: &mut Element, text: &str) {
    let lines: Vec<&str> = text.split('\n').collect();
    let mut filled = false;
    rewrite_paras(tx, &mut filled, &lines);
    if !filled {
        append_text_para(tx, &lines);
    }
    prune_empty_paras(tx);
}

fn rewrite_paras(el: &mut Element, filled: &mut bool, lines: &[&str]) {
    if el.local_name() == "AlternateContent" {
        if let Some(i) = first_alt_child_index(el) {
            if let Node::Element(child) = &mut el.children[i] {
                rewrite_paras(child, filled, lines);
            }
        }
        return;
    }
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

/// 清掉本段的可见内容：`a:t` 和 `a:br` 都删，因此变空的 `a:r` 删。
/// `a:br` 一并删是因为 inspect 把它读成 `\n`，留一个下来读回值就和写入值差一个换行。
/// `a:pPr`、`a:endParaRPr`、`a:fld` 留着；随后由 `prune_empty_paras` 决定空段是否删除。
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
                remove_all_t(e);
                if is_empty_run(e) {
                    p.children.remove(i);
                }
            }
            _ => {}
        }
    }
}

/// 清完之后只剩 `a:pPr` / `a:endParaRPr` 的段删掉。
/// 含 `a:r`、`a:br`、`a:fld` 或其它不认识的子元素的段留下（域按契约第 8 条不清）。
/// `CT_TextBody` 至少留一段（§A.4，`minOccurs="1"`）；从后往前删，优先保住前面的写入段。
fn prune_empty_paras(el: &mut Element) {
    if el.local_name() == "AlternateContent" {
        if let Some(i) = first_alt_child_index(el) {
            if let Node::Element(child) = &mut el.children[i] {
                prune_empty_paras(child);
            }
        }
        return;
    }
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
        if is_prunable_para(e) {
            el.children.remove(i);
            remaining -= 1;
        }
    }
}

fn is_prunable_para(p: &Element) -> bool {
    !p.element_children()
        .any(|e| !matches!(e.local_name(), "pPr" | "endParaRPr"))
}

/// 第一行写进第一个文本 run 的 `a:t`，余下每行在段落里插一个 `a:br` 加一个新 `a:r`。
fn fill_para(p: &mut Element, lines: &[&str]) {
    // 先把除第一个文本 run 以外的可见内容清干净，免得回填时和旧内容混在一起。
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
                remove_all_t(e);
                if is_empty_run(e) {
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
        (fill_single_line(run, lines[0]), t_name)
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

/// 文本写进第一个 `a:t`，多余的 `a:t` 删掉。返回 `a:rPr` 的副本，给后续行的新 run 用。
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

fn remove_all_t(r: &mut Element) {
    r.children.retain(|n| match n {
        Node::Element(e) => e.local_name() != "t",
        Node::Text(_) => true,
    });
}

fn is_empty_run(r: &Element) -> bool {
    !r.element_children().any(|e| e.local_name() != "rPr")
}

/// 一个文本 run 都没有时才走这里。已有段落就挂在第一段末尾，没有段落才新建一段，
/// 免得凭空多出一段。
fn append_text_para(tx: &mut Element, lines: &[&str]) {
    // txBody 自己是 `p:` 前缀，段落和 run 是 `a:` 前缀，前缀只能从里面的元素借。
    let sample = tx
        .element_children()
        .find(|e| e.local_name() == "p")
        .or_else(|| tx.element_children().next())
        .map(|e| e.name.clone())
        .unwrap_or_else(|| "a:bodyPr".to_string());
    let proto = Element::new(sample);
    let mut built: Vec<Element> = Vec::new();
    for (n, line) in lines.iter().enumerate() {
        if n > 0 {
            built.push(Element::new(proto.prefixed("br")));
        }
        let mut run = Element::new(proto.prefixed("r"));
        let mut t = Element::new(proto.prefixed("t"));
        apply_t_text(&mut t, line);
        run.push_element(t);
        built.push(run);
    }
    let existing = tx.children.iter_mut().find_map(|n| match n {
        Node::Element(e) if e.local_name() == "p" => Some(e),
        _ => None,
    });
    match existing {
        Some(p) => {
            // `a:endParaRPr` 必须在段末。空段常只有这一项，往后面塞 run 会被 PowerPoint 当成没字。
            let insert_at = p
                .children
                .iter()
                .position(|n| matches!(n, Node::Element(e) if e.local_name() == "endParaRPr"))
                .unwrap_or(p.children.len());
            for (k, el) in built.into_iter().enumerate() {
                p.children.insert(insert_at + k, Node::Element(el));
            }
        }
        None => {
            let mut p = Element::new(proto.prefixed("p"));
            for el in built {
                p.push_element(el);
            }
            tx.push_element(p);
        }
    }
}

fn apply_t_text(t: &mut Element, text: &str) {
    t.set_text(text.to_string());
    if needs_xml_space(text) {
        t.set_attr("xml:space", "preserve");
    } else {
        // `xml` 是 XML 规范的保留前缀，不能重新映射，所以精确名就够了。
        t.remove_attr("xml:space");
    }
}

fn needs_xml_space(text: &str) -> bool {
    text.starts_with(' ') || text.starts_with('\t') || text.ends_with(' ') || text.ends_with('\t')
}

/// 遇到 `mc:AlternateContent` 只进第一个 `Choice`，没有才进 `Fallback`。
pub fn walk_choice<'a>(el: &'a Element, f: &mut impl FnMut(&'a Element)) {
    if el.local_name() == "AlternateContent" {
        if let Some(i) = first_alt_child_index(el) {
            if let Node::Element(child) = &el.children[i] {
                walk_choice(child, f);
            }
        }
        return;
    }
    f(el);
    for n in &el.children {
        if let Node::Element(e) = n {
            walk_choice(e, f);
        }
    }
}

pub(crate) fn first_alt_child_index(el: &Element) -> Option<usize> {
    let mut fallback = None;
    for (i, n) in el.children.iter().enumerate() {
        if let Node::Element(e) = n {
            if e.local_name() == "Choice" {
                return Some(i);
            }
            if e.local_name() == "Fallback" && fallback.is_none() {
                fallback = Some(i);
            }
        }
    }
    fallback
}

const DEFAULT_SHAPE_CX: i64 = 1_828_800;
const DEFAULT_SHAPE_CY: i64 = 914_400;

fn is_counted_shape(el: &Element) -> bool {
    el.local_name() == "sp" && el.child_by_local("txBody").is_some()
}

fn shape_matches(el: &Element, index: usize, key: &ShapeKey) -> bool {
    match key {
        ShapeKey::Index(n) => index == *n,
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

pub fn is_writable_shape_prst(s: &str) -> bool {
    matches!(
        s,
        "rect"
            | "roundRect"
            | "round1Rect"
            | "round2SameRect"
            | "round2DiagRect"
            | "snipRoundRect"
            | "snip1Rect"
            | "snip2SameRect"
            | "snip2DiagRect"
            | "ellipse"
            | "triangle"
            | "rtTriangle"
            | "parallelogram"
            | "trapezoid"
            | "diamond"
            | "pentagon"
            | "hexagon"
            | "heptagon"
            | "octagon"
            | "decagon"
            | "dodecagon"
            | "star4"
            | "star5"
            | "star6"
            | "star8"
            | "star16"
            | "star24"
            | "star32"
            | "plus"
            | "plaque"
            | "teardrop"
            | "homePlate"
            | "chevron"
            | "pie"
            | "donut"
            | "noSmoking"
            | "blockArc"
            | "rightArrow"
            | "leftArrow"
            | "upArrow"
            | "downArrow"
            | "leftRightArrow"
            | "upDownArrow"
            | "quadArrow"
            | "stripedRightArrow"
            | "notchedRightArrow"
            | "bentUpArrow"
            | "cube"
            | "can"
            | "lightningBolt"
            | "heart"
            | "sun"
            | "moon"
            | "smileyFace"
            | "foldedCorner"
            | "bevel"
            | "frame"
            | "cloud"
            | "ribbon"
            | "wave"
    )
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
    let is_box = op.prst == "rect";
    let name = if is_box {
        format!("文本框 {cnv}")
    } else {
        format!("形状 {cnv}")
    };
    let mut sp = build_sp(cnv, &name, &op.prst, is_box, x, y, cx, cy);
    if let Some(tx) = sp.child_by_local_mut("txBody") {
        set_txbody_text(tx, &op.text);
    }
    picture::insert_into_sp_tree(&mut doc.root, sp)?;
    Ok(())
}

fn build_sp(
    cnv: u32,
    name: &str,
    prst: &str,
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
    let mut geom = Element::new("a:prstGeom").with_attr("prst", prst);
    geom.push_element(Element::new("a:avLst"));
    sp_pr.push_element(geom);
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
    let sp = find_shape_mut(&mut doc.root, &op.target.key).ok_or_else(|| {
        Error::with_path(
            ErrorKind::ShapeNotFound { id: op.id.clone() },
            format!("ops[{op_index}].id"),
        )
    })?;
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
    match op.color.as_deref() {
        Some(c) => {
            let mut fill = Element::new("a:solidFill");
            fill.push_element(Element::new("a:srgbClr").with_attr("val", c.to_ascii_uppercase()));
            sp_pr.push_element(fill);
        }
        None => sp_pr.push_element(Element::new("a:noFill")),
    }
    Ok(())
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
