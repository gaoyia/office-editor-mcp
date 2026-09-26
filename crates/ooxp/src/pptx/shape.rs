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

fn ids_match(file: &str, req: &str) -> bool {
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
            for el in built {
                p.push_element(el);
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
