//! 正文与页眉页脚里的修订痕迹：inspect 名单与接受 / 拒绝。

use serde_json::{json, Value};

use crate::error::{Error, ErrorKind, IdReason, XmlIssue};
use crate::xml::{Element, Node};

use super::hf::{self, HfKind, HfPart, HfType};
use super::para;

#[derive(Debug, Clone)]
pub enum RevStory {
    Body,
    HeaderFooter {
        kind: HfKind,
        hf_type: HfType,
        n: u32,
    },
    Note {
        kind: para::NoteKind,
        note_id: String,
    },
    Comment {
        comment_id: String,
    },
}

#[derive(Debug, Clone)]
pub struct RevRef {
    pub story: RevStory,
    pub n: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RevKind {
    Ins,
    Del,
    ParaMarkIns,
    ParaMarkDel,
    RowIns,
    RowDel,
    RPrChange,
    PPrChange,
    TblPrChange,
    TblGridChange,
    TblPrExChange,
    TcPrChange,
    TrPrChange,
    SectPrChange,
    MoveFrom,
    MoveTo,
}

#[derive(Clone, Copy)]
struct WalkCtx {
    in_prop: bool,
    in_tr_pr: bool,
    in_p_pr: bool,
    in_run: bool,
    row_tracked: bool,
}

impl RevKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Ins => "ins",
            Self::Del => "del",
            Self::ParaMarkIns => "paraMarkIns",
            Self::ParaMarkDel => "paraMarkDel",
            Self::RowIns => "rowIns",
            Self::RowDel => "rowDel",
            Self::RPrChange => "rPrChange",
            Self::PPrChange => "pPrChange",
            Self::TblPrChange => "tblPrChange",
            Self::TblGridChange => "tblGridChange",
            Self::TblPrExChange => "tblPrExChange",
            Self::TcPrChange => "tcPrChange",
            Self::TrPrChange => "trPrChange",
            Self::SectPrChange => "sectPrChange",
            Self::MoveFrom => "moveFrom",
            Self::MoveTo => "moveTo",
        }
    }

    fn change_inner(self) -> Option<&'static str> {
        match self {
            Self::RPrChange => Some("rPr"),
            Self::PPrChange => Some("pPr"),
            Self::TblPrChange => Some("tblPr"),
            Self::TblGridChange => Some("tblGrid"),
            Self::TblPrExChange => Some("tblPrEx"),
            Self::TcPrChange => Some("tcPr"),
            Self::TrPrChange => Some("trPr"),
            Self::SectPrChange => Some("sectPr"),
            _ => None,
        }
    }

    fn is_change(self) -> bool {
        self.change_inner().is_some()
    }
}

pub fn count_revisions(root: &Element) -> usize {
    collect_rev_hits(root).len()
}

pub fn revisions_json(root: &Element, container: Option<&str>) -> Result<Vec<Value>, Error> {
    let paras = para::collect_paragraphs(root);
    let mut index_of: Vec<(*const Element, usize)> = Vec::new();
    for (i, p) in paras.iter().enumerate() {
        index_of.push((*p as *const Element, i + 1));
    }
    let hits = collect_rev_hits(root);
    let mut out = Vec::new();
    for (i, (path, kind)) in hits.iter().enumerate() {
        let el = para::element_at(root, path).ok_or_else(|| xml_broken("revision"))?;
        let n = i + 1;
        let id = match container {
            Some(c) => format!("{c}/rev:{n}"),
            None => format!("rev:{n}"),
        };
        let xml_id = el.attr_local("id");
        let author = el.attr_local("author");
        let date = el.attr_local("date");
        let host = host_para_id(root, path, container, &index_of)?;
        let text = match kind {
            RevKind::Ins | RevKind::MoveFrom | RevKind::MoveTo => json!(markup_text(el, false)),
            RevKind::Del => json!(markup_text(el, true)),
            _ => Value::Null,
        };
        out.push(json!({
            "id": id,
            "index": n,
            "kind": kind.as_str(),
            "xmlId": xml_id,
            "author": author,
            "date": date,
            "host": host,
            "text": text,
        }));
    }
    Ok(out)
}

pub fn parse_rev_ref(id: &str, path: &str) -> Result<RevRef, Error> {
    let (story, rest) = split_story(id, path)?;
    if rest.contains('/') {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::RevPrefix,
            },
            path.to_string(),
        ));
    }
    let n = parse_n(rest, path)?;
    Ok(RevRef { story, n })
}

pub fn part_for_story(
    pkg: &crate::opc::Package,
    catalog: &[HfPart],
    doc_part: &str,
    story: &RevStory,
    id: &str,
    i: usize,
) -> Result<String, Error> {
    match story {
        RevStory::Body => Ok(doc_part.to_string()),
        RevStory::HeaderFooter { kind, hf_type, n } => {
            let cid = format!("{}:{}:{n}", kind.as_str(), hf_type.as_str());
            let item = hf::find_by_id(catalog, &cid).ok_or_else(|| missing(id, i))?;
            Ok(item.part.clone())
        }
        RevStory::Note { kind, .. } => {
            super::notes::part_name(pkg, doc_part, *kind)?.ok_or_else(|| missing(id, i))
        }
        RevStory::Comment { .. } => {
            super::comments::part_name(pkg, doc_part)?.ok_or_else(|| missing(id, i))
        }
    }
}

/// 段内是否有本期收入的修订节点。框内段落同样按本段子孙判定。
pub fn para_blocks_set_text(p: &Element) -> bool {
    !collect_rev_hits(p).is_empty()
}

pub fn apply_revision(
    root: &mut Element,
    n: usize,
    accept: bool,
    id: &str,
    i: usize,
) -> Result<(), Error> {
    let hits = collect_rev_hits(root);
    let Some((path, kind)) = para::pick_collected(root, hits, n, "rev", |hit| hit.0.as_slice())
    else {
        return Err(missing(id, i));
    };
    apply_kind(root, &path, kind, accept, i)
}

pub fn apply_revision_under(
    root: &mut Element,
    host_path: &[usize],
    n: usize,
    accept: bool,
    id: &str,
    i: usize,
) -> Result<(), Error> {
    let host = para::element_at(root, host_path).ok_or_else(|| missing(id, i))?;
    let hits = collect_rev_hits(host);
    let Some((rel, kind)) = para::pick_collected(host, hits, n, "rev", |hit| hit.0.as_slice())
    else {
        return Err(missing(id, i));
    };
    let mut path = host_path.to_vec();
    path.extend(rel);
    apply_kind(root, &path, kind, accept, i)
}

fn apply_kind(
    root: &mut Element,
    path: &[usize],
    kind: RevKind,
    accept: bool,
    i: usize,
) -> Result<(), Error> {
    match (kind, accept) {
        (RevKind::Ins, true) | (RevKind::Del, false) => {
            unwrap_at(root, path, kind == RevKind::Del)?
        }
        (RevKind::Ins, false) | (RevKind::Del, true) => remove_at(root, path)?,
        (k, true) if k.is_change() => remove_at(root, path)?,
        (k, false) if k.is_change() => {
            let inner = k.change_inner().ok_or_else(|| xml_broken("revision"))?;
            restore_prev(root, path, inner)?;
        }
        (RevKind::ParaMarkIns, true) | (RevKind::ParaMarkDel, false) => remove_at(root, path)?,
        (RevKind::ParaMarkIns, false) | (RevKind::ParaMarkDel, true) => {
            delete_host_para(root, path, i)?;
        }
        (RevKind::RowDel, true) | (RevKind::RowIns, false) => {
            delete_host_row(root, path, i)?;
        }
        (RevKind::RowIns, true) => settle_row(root, path, true)?,
        (RevKind::RowDel, false) => settle_row(root, path, false)?,
        (RevKind::MoveFrom, true) | (RevKind::MoveTo, false) => apply_move(root, path, kind, true)?,
        (RevKind::MoveFrom, false) | (RevKind::MoveTo, true) => {
            apply_move(root, path, kind, false)?
        }
        _ => return Err(xml_broken("revision")),
    }
    Ok(())
}

fn collect_rev_hits(root: &Element) -> Vec<(Vec<usize>, RevKind)> {
    let path = Vec::new();
    let mut out = Vec::new();
    walk_rev(root, &path, WalkCtx::root(), &mut out);
    out
}

pub(crate) fn pin_revs(root: &mut Element) {
    let paths: Vec<_> = collect_rev_hits(root)
        .into_iter()
        .map(|(path, _)| path)
        .collect();
    para::pin_collected(root, &paths, "rev");
}

impl WalkCtx {
    fn root() -> Self {
        Self {
            in_prop: false,
            in_tr_pr: false,
            in_p_pr: false,
            in_run: false,
            row_tracked: false,
        }
    }

    fn enter(self, el: &Element) -> Self {
        let name = el.local_name();
        Self {
            in_prop: self.in_prop || is_prop_container(name),
            in_tr_pr: self.in_tr_pr || name == "trPr",
            in_p_pr: self.in_p_pr || name == "pPr",
            in_run: self.in_run || name == "r",
            row_tracked: self.row_tracked || (name == "tr" && tr_is_tracked(el)),
        }
    }
}

fn walk_rev(root: &Element, path: &[usize], ctx: WalkCtx, out: &mut Vec<(Vec<usize>, RevKind)>) {
    let mut stack = vec![(root, path.to_vec(), ctx)];
    while let Some((el, path, ctx)) = stack.pop() {
        if el.local_name() == "AlternateContent" {
            if let Some(i) = first_alt(el) {
                if let Node::Element(c) = &el.children[i] {
                    let mut next = path;
                    next.push(i);
                    stack.push((c, next, ctx));
                }
            }
            continue;
        }
        if let Some(kind) = classify_rev(el, ctx) {
            out.push((path.clone(), kind));
            if kind.is_change() {
                continue;
            }
        }
        let next = ctx.enter(el);
        let mut kids = Vec::new();
        for (i, n) in el.children.iter().enumerate() {
            if let Node::Element(child) = n {
                let mut child_path = path.clone();
                child_path.push(i);
                kids.push((child, child_path, next));
            }
        }
        for item in kids.into_iter().rev() {
            stack.push(item);
        }
    }
}

fn classify_rev(el: &Element, ctx: WalkCtx) -> Option<RevKind> {
    let name = el.local_name();
    match name {
        "rPrChange" => Some(RevKind::RPrChange),
        "pPrChange" => Some(RevKind::PPrChange),
        "tblPrChange" => Some(RevKind::TblPrChange),
        "tblGridChange" => Some(RevKind::TblGridChange),
        "tblPrExChange" => Some(RevKind::TblPrExChange),
        "tcPrChange" => Some(RevKind::TcPrChange),
        "trPrChange" => Some(RevKind::TrPrChange),
        "sectPrChange" => Some(RevKind::SectPrChange),
        "ins" | "del" => {
            let ins = name == "ins";
            if ctx.in_tr_pr {
                return Some(if ins {
                    RevKind::RowIns
                } else {
                    RevKind::RowDel
                });
            }
            if ctx.in_p_pr && !ctx.in_run {
                if ctx.row_tracked {
                    return None;
                }
                return Some(if ins {
                    RevKind::ParaMarkIns
                } else {
                    RevKind::ParaMarkDel
                });
            }
            if is_run_ins_del(el, ctx.in_prop) {
                return Some(if ins { RevKind::Ins } else { RevKind::Del });
            }
            None
        }
        "moveFrom" | "moveTo" => {
            if is_run_ins_del(el, ctx.in_prop) {
                return Some(if name == "moveFrom" {
                    RevKind::MoveFrom
                } else {
                    RevKind::MoveTo
                });
            }
            None
        }
        _ => None,
    }
}

fn is_prop_container(name: &str) -> bool {
    matches!(
        name,
        "pPr" | "rPr" | "trPr" | "tblPr" | "tcPr" | "tblPrEx" | "sectPr" | "numPr"
    )
}

fn is_run_ins_del(el: &Element, in_prop: bool) -> bool {
    if in_prop {
        return false;
    }
    !el.element_children()
        .any(|c| matches!(c.local_name(), "p" | "tbl"))
}

fn tr_is_tracked(tr: &Element) -> bool {
    tr.child_by_local("trPr").is_some_and(|pr| {
        pr.element_children()
            .any(|c| matches!(c.local_name(), "ins" | "del"))
    })
}

fn host_para_id(
    root: &Element,
    path: &[usize],
    container: Option<&str>,
    index_of: &[(*const Element, usize)],
) -> Result<Value, Error> {
    let mut cur = root;
    let mut outer: Option<&Element> = None;
    let mut txbx: Option<&Element> = None;
    let mut inner: Option<&Element> = None;
    for &i in path {
        match cur.children.get(i) {
            Some(Node::Element(e)) => {
                if e.local_name() == "txbxContent" {
                    txbx = Some(e);
                }
                if e.local_name() == "p" {
                    if txbx.is_some() {
                        inner = Some(e);
                    } else {
                        outer = Some(e);
                    }
                }
                cur = e;
            }
            _ => return Ok(Value::Null),
        }
    }
    if let Some(content) = txbx {
        let Some(shell) = outer else {
            return Ok(Value::Null);
        };
        let Some(p) = inner else {
            return Ok(Value::Null);
        };
        let Some(index) = index_of
            .iter()
            .find(|(ptr, _)| std::ptr::eq(*ptr, shell))
            .map(|(_, i)| *i)
        else {
            return Ok(Value::Null);
        };
        let (shell_id, _) = match container {
            Some(c) => para::para_stable_id_in(c, shell, index),
            None => para::para_stable_id(shell, index),
        };
        let boxes = super::textbox::collect_textboxes(shell, &shell_id)?;
        for b in &boxes {
            let key = if let Some(d) = &b.doc_pr_id {
                para::TxbxKey::DocPr(d.clone())
            } else if let Some((_, rest)) = b.id.rsplit_once("/txbx:index:") {
                let Ok(n) = rest.parse::<usize>() else {
                    continue;
                };
                para::TxbxKey::Index(n)
            } else {
                continue;
            };
            let Some(found) = super::textbox::find_txbx_content_by_key(shell, &key) else {
                continue;
            };
            if !std::ptr::eq(found, content) {
                continue;
            }
            let paras = para::collect_paragraphs_in_txbx(found);
            let Some(pos) = paras.iter().position(|el| std::ptr::eq(*el, p)) else {
                continue;
            };
            let (id, _) = para::para_stable_id_in(&b.id, p, pos + 1);
            return Ok(json!(id));
        }
        return Ok(Value::Null);
    }
    let Some(p) = outer else {
        return Ok(Value::Null);
    };
    let Some(index) = index_of
        .iter()
        .find(|(ptr, _)| std::ptr::eq(*ptr, p))
        .map(|(_, i)| *i)
    else {
        return Ok(Value::Null);
    };
    let (id, _) = match container {
        Some(c) => para::para_stable_id_in(c, p, index),
        None => para::para_stable_id(p, index),
    };
    Ok(json!(id))
}

fn markup_text(el: &Element, deleted: bool) -> String {
    let mut out = String::new();
    collect_markup_text(el, deleted, &mut out);
    out
}

fn collect_markup_text(el: &Element, deleted: bool, out: &mut String) {
    let name = el.local_name();
    if deleted {
        if name == "delText" || name == "t" {
            out.push_str(&el.text_content());
            return;
        }
    } else if name == "t" {
        out.push_str(&el.text_content());
        return;
    }
    if name == "txbxContent" {
        return;
    }
    if name == "AlternateContent" {
        if let Some(i) = first_alt(el) {
            if let Node::Element(c) = &el.children[i] {
                collect_markup_text(c, deleted, out);
            }
        }
        return;
    }
    for n in &el.children {
        if let Node::Element(e) = n {
            collect_markup_text(e, deleted, out);
        }
    }
}

fn split_story<'a>(id: &'a str, path: &str) -> Result<(RevStory, &'a str), Error> {
    if let Some(rest) = id.strip_prefix("rev:") {
        return Ok((RevStory::Body, rest));
    }
    if let Some((pre, rest)) = id.split_once("/rev:") {
        if let Some((kind, hf_type, n)) = para::parse_hf_container_id(pre) {
            return Ok((RevStory::HeaderFooter { kind, hf_type, n }, rest));
        }
        if let Some(note_id) = pre.strip_prefix("footnote:") {
            return Ok((
                RevStory::Note {
                    kind: para::NoteKind::Footnote,
                    note_id: note_id.to_string(),
                },
                rest,
            ));
        }
        if let Some(note_id) = pre.strip_prefix("endnote:") {
            return Ok((
                RevStory::Note {
                    kind: para::NoteKind::Endnote,
                    note_id: note_id.to_string(),
                },
                rest,
            ));
        }
        if let Some(comment_id) = pre.strip_prefix("comment:") {
            return Ok((
                RevStory::Comment {
                    comment_id: comment_id.to_string(),
                },
                rest,
            ));
        }
    }
    Err(Error::with_path(
        ErrorKind::InvalidId {
            reason: IdReason::RevPrefix,
        },
        path.to_string(),
    ))
}

fn parse_n(s: &str, path: &str) -> Result<usize, Error> {
    let n: usize = s.parse().map_err(|_| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::RevIndexNotInt,
            },
            path.to_string(),
        )
    })?;
    if n < 1 {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::RevIndexFromOne,
            },
            path.to_string(),
        ));
    }
    Ok(n)
}

fn ancestor_path(root: &Element, path: &[usize], local: &str) -> Option<Vec<usize>> {
    let mut cur = root;
    let mut found = None;
    for (i, &idx) in path.iter().enumerate() {
        match cur.children.get(idx) {
            Some(Node::Element(e)) => {
                if e.local_name() == local {
                    found = Some(path[..=i].to_vec());
                }
                cur = e;
            }
            _ => return None,
        }
    }
    found
}

fn delete_host_para(root: &mut Element, path: &[usize], i: usize) -> Result<(), Error> {
    let p_path = ancestor_path(root, path, "p").ok_or_else(|| xml_broken("revision"))?;
    let parent_count = {
        let (parent_path, _) = split_parent(&p_path)?;
        let parent = para::element_at(root, parent_path).ok_or_else(|| xml_broken("revision"))?;
        para::count_direct_p(parent)
    };
    if parent_count <= 1 {
        return Err(Error::with_path(
            ErrorKind::LastStoryPara,
            format!("ops[{i}].id"),
        ));
    }
    {
        let p = para::element_at(root, &p_path).ok_or_else(|| xml_broken("revision"))?;
        if para::para_has_unmatched_range(p) {
            return Err(Error::with_path(
                ErrorKind::CrossParaMarker,
                format!("ops[{i}].id"),
            ));
        }
        if para::para_has_sect_pr(p) {
            return Err(Error::with_path(
                ErrorKind::SectionBreakPara,
                format!("ops[{i}].id"),
            ));
        }
    }
    remove_at(root, &p_path)
}

fn delete_host_row(root: &mut Element, path: &[usize], i: usize) -> Result<(), Error> {
    let tr_path = ancestor_path(root, path, "tr").ok_or_else(|| xml_broken("revision"))?;
    let row_count = {
        let (parent_path, _) = split_parent(&tr_path)?;
        let parent = para::element_at(root, parent_path).ok_or_else(|| xml_broken("revision"))?;
        parent
            .element_children()
            .filter(|c| c.local_name() == "tr")
            .count()
    };
    if row_count <= 1 {
        return Err(Error::with_path(
            ErrorKind::LastTableRow,
            format!("ops[{i}].id"),
        ));
    }
    remove_at(root, &tr_path)
}

fn settle_row(root: &mut Element, path: &[usize], unwrap_ins: bool) -> Result<(), Error> {
    let tr_path = ancestor_path(root, path, "tr").ok_or_else(|| xml_broken("revision"))?;
    let tr = para::element_at_mut(root, &tr_path).ok_or_else(|| xml_broken("revision"))?;
    loop {
        let hits = collect_rev_hits(tr);
        let run = hits.iter().find(|(_, k)| {
            matches!(
                (k, unwrap_ins),
                (RevKind::Ins, true) | (RevKind::Del, false)
            )
        });
        let Some((rel, kind)) = run.cloned() else {
            break;
        };
        if kind == RevKind::Ins {
            unwrap_at(tr, &rel, false)?;
        } else {
            unwrap_at(tr, &rel, true)?;
        }
    }
    strip_prop_ins_del(tr);
    Ok(())
}

fn apply_move(
    root: &mut Element,
    path: &[usize],
    kind: RevKind,
    drop_content: bool,
) -> Result<(), Error> {
    let xml_id = para::element_at(root, path)
        .and_then(|el| el.attr_local("id"))
        .map(str::to_string);
    if drop_content {
        remove_at(root, path)?;
    } else {
        unwrap_at(root, path, false)?;
    }
    let Some(id) = xml_id else {
        return Ok(());
    };
    let (start, end) = match kind {
        RevKind::MoveFrom => ("moveFromRangeStart", "moveFromRangeEnd"),
        RevKind::MoveTo => ("moveToRangeStart", "moveToRangeEnd"),
        _ => return Ok(()),
    };
    remove_named_markers(root, start, end, &id)
}

fn remove_named_markers(
    root: &mut Element,
    start: &str,
    end: &str,
    xml_id: &str,
) -> Result<(), Error> {
    let mut paths = Vec::new();
    let mut path = Vec::new();
    collect_named_markers(root, &mut path, start, end, xml_id, &mut paths);
    paths.sort_by(|a, b| cmp_path_rev(a, b));
    for p in paths {
        remove_at(root, &p)?;
    }
    Ok(())
}

fn collect_named_markers(
    el: &Element,
    path: &mut Vec<usize>,
    start: &str,
    end: &str,
    xml_id: &str,
    out: &mut Vec<Vec<usize>>,
) {
    if el.local_name() == "txbxContent" {
        return;
    }
    if el.local_name() == "AlternateContent" {
        if let Some(i) = first_alt(el) {
            path.push(i);
            if let Node::Element(c) = &el.children[i] {
                collect_named_markers(c, path, start, end, xml_id, out);
            }
            path.pop();
        }
        return;
    }
    let name = el.local_name();
    if (name == start || name == end) && el.attr_local("id") == Some(xml_id) {
        out.push(path.clone());
    }
    for (i, n) in el.children.iter().enumerate() {
        if let Node::Element(e) = n {
            path.push(i);
            collect_named_markers(e, path, start, end, xml_id, out);
            path.pop();
        }
    }
}

fn cmp_path_rev(a: &[usize], b: &[usize]) -> std::cmp::Ordering {
    for (x, y) in a.iter().zip(b.iter()) {
        match x.cmp(y) {
            std::cmp::Ordering::Equal => continue,
            o => return o.reverse(),
        }
    }
    b.len().cmp(&a.len())
}

fn remove_at(root: &mut Element, path: &[usize]) -> Result<(), Error> {
    let (parent_path, idx) = split_parent(path)?;
    let parent = para::element_at_mut(root, parent_path).ok_or_else(|| xml_broken("revision"))?;
    if idx >= parent.children.len() {
        return Err(xml_broken("revision"));
    }
    parent.children.remove(idx);
    Ok(())
}

fn unwrap_at(root: &mut Element, path: &[usize], restore_del: bool) -> Result<(), Error> {
    let (parent_path, idx) = split_parent(path)?;
    let parent = para::element_at_mut(root, parent_path).ok_or_else(|| xml_broken("revision"))?;
    if idx >= parent.children.len() {
        return Err(xml_broken("revision"));
    }
    let Node::Element(mut el) = parent.children.remove(idx) else {
        return Err(xml_broken("revision"));
    };
    if restore_del {
        restore_del_text(&mut el);
    }
    // Word 会把同一条修订再写进 rPr 的 ins/del；拆外壳后必须去掉，否则仍显示为修订。
    strip_prop_ins_del(&mut el);
    parent.children.splice(idx..idx, el.take_children());
    Ok(())
}

fn restore_prev(root: &mut Element, path: &[usize], inner: &str) -> Result<(), Error> {
    let prev_children = {
        let change = para::element_at(root, path).ok_or_else(|| xml_broken("revision"))?;
        let prev = change
            .child_by_local(inner)
            .ok_or_else(|| xml_broken(inner))?;
        prev.children.clone()
    };
    let (parent_path, _) = split_parent(path)?;
    let parent = para::element_at_mut(root, parent_path).ok_or_else(|| xml_broken("revision"))?;
    parent.children = prev_children;
    Ok(())
}

fn restore_del_text(el: &mut Element) {
    if el.local_name() == "delText" {
        el.name = el.prefixed("t");
    }
    for n in &mut el.children {
        if let Node::Element(e) = n {
            restore_del_text(e);
        }
    }
}

fn strip_prop_ins_del(el: &mut Element) {
    if is_prop_container(el.local_name()) {
        el.children.retain(|n| match n {
            Node::Element(e) => !matches!(e.local_name(), "ins" | "del"),
            _ => true,
        });
    }
    for n in &mut el.children {
        if let Node::Element(e) = n {
            strip_prop_ins_del(e);
        }
    }
}

fn split_parent(path: &[usize]) -> Result<(&[usize], usize), Error> {
    if path.is_empty() {
        return Err(xml_broken("revision"));
    }
    Ok((&path[..path.len() - 1], path[path.len() - 1]))
}

fn first_alt(el: &Element) -> Option<usize> {
    crate::xml::alternate_child(el).unwrap_or(None)
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

fn direct_child_path(root: &Element, pred: impl Fn(&Element) -> bool) -> Option<Vec<usize>> {
    root.children.iter().enumerate().find_map(|(i, n)| match n {
        Node::Element(e) if pred(e) => Some(vec![i]),
        _ => None,
    })
}

fn missing(id: &str, i: usize) -> Error {
    Error::with_path(
        ErrorKind::RevisionNotFound { id: id.to_string() },
        format!("ops[{i}].id"),
    )
}

use super::request::RevisionOp;
use crate::opc::Package;
use crate::xml::Document;
use std::collections::HashMap;
// 从 apply 搬来的操作体。
pub(crate) fn refuse_set_text_if_revision(el: &Element, i: usize) -> Result<(), Error> {
    if super::revision::para_blocks_set_text(el) {
        return Err(Error::with_path(
            ErrorKind::SetParaTextHasRevision,
            format!("ops[{i}].id"),
        ));
    }
    Ok(())
}
pub(crate) fn apply_rev(
    pkg: &Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &RevisionOp,
    accept: bool,
    i: usize,
) -> Result<(), Error> {
    let part_name =
        super::revision::part_for_story(pkg, catalog, doc_part, &op.target.story, &op.id, i)?;
    let doc = super::apply::load_part(pkg, docs, &part_name)?;
    match &op.target.story {
        super::revision::RevStory::Note { kind, note_id } => {
            let host_path = direct_child_path(&doc.root, |e| {
                e.local_name() == kind.item_local()
                    && super::notes::ids_match(e.attr_local("id").unwrap_or(""), note_id)
                    && super::notes::is_user_note(e)
            })
            .ok_or_else(|| missing(&op.id, i))?;
            super::revision::apply_revision_under(
                &mut doc.root,
                &host_path,
                op.target.n,
                accept,
                &op.id,
                i,
            )?;
        }
        super::revision::RevStory::Comment { comment_id } => {
            let host_path = direct_child_path(&doc.root, |e| {
                e.local_name() == "comment" && e.attr_local("id") == Some(comment_id)
            })
            .ok_or_else(|| missing(&op.id, i))?;
            super::revision::apply_revision_under(
                &mut doc.root,
                &host_path,
                op.target.n,
                accept,
                &op.id,
                i,
            )?;
        }
        _ => super::revision::apply_revision(&mut doc.root, op.target.n, accept, &op.id, i)?,
    }
    dirty.insert(part_name, ());
    Ok(())
}
