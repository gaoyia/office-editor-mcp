//! 幻灯片里的文本形状：定位、可见文字、AlternateContent 只走一支。

use crate::error::{Error, ErrorKind, IdReason};
use crate::opc::{self, Package};
use crate::xml::{Element, Node};

pub struct SlideMeta {
    pub index: usize,
    pub part: String,
    pub name: Option<String>,
    pub hidden: bool,
}

pub enum ShapeKey {
    Id(String),
    Index(usize),
}

pub struct ShapeTarget {
    pub slide: usize,
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
    let (n, after) = rest.split_once("/shape:").ok_or_else(|| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::ShapeForm,
            },
            path.clone(),
        )
    })?;
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
            key: ShapeKey::Index(idx),
        });
    }
    Ok(ShapeTarget {
        slide,
        key: ShapeKey::Id(after.to_string()),
    })
}

/// `r:id` / `rel:id`，不要和 `p:sldId/@id` 弄混。后者的本地名也是 id。
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
        let rid = rid_attr(sld_id).ok_or_else(|| {
            Error::new(ErrorKind::SlideRelMissing { index: i + 1 })
        })?;
        let part = opc::resolve_id(pkg, pres_part, rid)?;
        let doc = crate::xml::parse(pkg.get(&part)?)?;
        let name = doc
            .root
            .child_by_local("cSld")
            .and_then(|c| c.attr("name"))
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        let hidden = matches!(doc.root.attr("show"), Some("0") | Some("false") | Some("FALSE"));
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

pub fn shape_stable_id(sp: &Element, slide: usize, index: usize) -> (String, Option<String>, bool) {
    if let Some(id) = cnv_pr(sp)
        .and_then(|c| c.attr("id"))
        .filter(|s| !s.is_empty())
    {
        (
            format!("slide:{slide}/shape:{id}"),
            Some(id.to_string()),
            true,
        )
    } else {
        (format!("slide:{slide}/shape:index:{index}"), None, false)
    }
}

pub fn shape_name(sp: &Element) -> Option<String> {
    cnv_pr(sp)
        .and_then(|c| c.attr("name"))
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

pub fn visible_text(sp: &Element) -> String {
    let Some(tx) = sp.child_by_local("txBody") else {
        return String::new();
    };
    let mut out = String::new();
    walk_choice(tx, &mut |e| match e.local_name() {
        "t" => out.push_str(&e.text_content()),
        "br" => out.push('\n'),
        _ => {}
    });
    out
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

fn ids_match(file: &str, req: &str) -> bool {
    match (file.parse::<u64>(), req.parse::<u64>()) {
        (Ok(a), Ok(b)) => a == b,
        _ => file == req,
    }
}

/// 按 `docs/json-contract.md` 的替换规则原地改：新文本写进第一个含 `a:t` 的 `a:r`，
/// 其余这类 `a:t` 删掉，因此变空的 `a:r` 删掉。
///
/// 段落本身、`a:pPr`、`a:bodyPr`、`a:lstStyle` 一概不动。早先的写法是把所有 `a:p`
/// 删掉重建一个，还拿「整个 txBody 里第一个 pPr」当新段落的属性，结果多段形状被压成
/// 一段，而且第一段会白捡后面某段的对齐方式。
pub fn set_txbody_text(tx: &mut Element, text: &str) {
    let mut filled = false;
    rewrite_runs(tx, &mut filled, text);
    if !filled {
        append_text_run(tx, text);
    }
}

fn rewrite_runs(el: &mut Element, filled: &mut bool, text: &str) {
    if el.local_name() == "AlternateContent" {
        if let Some(i) = first_alt_child_index(el) {
            if let Node::Element(child) = &mut el.children[i] {
                rewrite_runs(child, filled, text);
            }
        }
        return;
    }
    // 只认 `a:r`。`a:fld` 里的 `a:t` 不动，契约只让改 run。
    if el.local_name() == "r" {
        if el.element_children().any(|e| e.local_name() == "t") {
            if *filled {
                remove_all_t(el);
            } else {
                fill_first_run(el, text);
                *filled = true;
            }
        }
        return;
    }
    let mut drop = Vec::new();
    for (i, child) in el.children.iter_mut().enumerate() {
        if let Node::Element(e) = child {
            let was_run = e.local_name() == "r";
            rewrite_runs(e, filled, text);
            if was_run && is_empty_run(e) {
                drop.push(i);
            }
        }
    }
    for i in drop.into_iter().rev() {
        el.children.remove(i);
    }
}

/// 文本里的 `\n` 拆成 `a:t` 加 `a:br`，都放在这一个 run 里。原有的 `a:rPr` 留着。
fn fill_first_run(r: &mut Element, text: &str) {
    let Some(pos) = r
        .children
        .iter()
        .position(|n| matches!(n, Node::Element(e) if e.local_name() == "t"))
    else {
        return;
    };
    let t_name = match &r.children[pos] {
        Node::Element(e) => e.name.clone(),
        Node::Text(_) => return,
    };
    let proto = Element::new(t_name.clone());
    // 从后往前删，pos 之前的下标不受影响。
    let mut i = r.children.len();
    while i > 0 {
        i -= 1;
        if let Node::Element(e) = &r.children[i] {
            if e.local_name() == "t" {
                r.children.remove(i);
            }
        }
    }
    let mut at = pos;
    for (n, line) in text.split('\n').enumerate() {
        if n > 0 {
            r.children
                .insert(at, Node::Element(Element::new(proto.prefixed("br"))));
            at += 1;
        }
        let mut t = Element::new(t_name.clone());
        apply_t_text(&mut t, line);
        r.children.insert(at, Node::Element(t));
        at += 1;
    }
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

/// 一个含 `a:t` 的 run 都没有时才走这里：补一个 run。
/// 已有段落就挂在第一段末尾，没有段落才新建一段，免得凭空多出一段。
fn append_text_run(tx: &mut Element, text: &str) {
    // txBody 自己是 `p:` 前缀，段落和 run 是 `a:` 前缀，前缀只能从里面的元素借。
    let sample = tx
        .element_children()
        .find(|e| e.local_name() == "p")
        .or_else(|| tx.element_children().next())
        .map(|e| e.name.clone())
        .unwrap_or_else(|| "a:bodyPr".to_string());
    let proto = Element::new(sample);
    let mut run = Element::new(proto.prefixed("r"));
    for (n, line) in text.split('\n').enumerate() {
        if n > 0 {
            run.push_element(Element::new(proto.prefixed("br")));
        }
        let mut t = Element::new(proto.prefixed("t"));
        apply_t_text(&mut t, line);
        run.push_element(t);
    }
    let existing = tx.children.iter_mut().find_map(|n| match n {
        Node::Element(e) if e.local_name() == "p" => Some(e),
        _ => None,
    });
    match existing {
        Some(p) => p.push_element(run),
        None => {
            let mut p = Element::new(proto.prefixed("p"));
            p.push_element(run);
            tx.push_element(p);
        }
    }
}

fn apply_t_text(t: &mut Element, text: &str) {
    t.set_text(text.to_string());
    if needs_xml_space(text) {
        t.set_attr("xml:space", "preserve");
    } else {
        t.remove_attr("space");
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

fn first_alt_child_index(el: &Element) -> Option<usize> {
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
