//! 主文档里的段落：定位、可见文本、大纲级别。

use crate::error::{Error, ErrorKind, IdReason};
use crate::xml::{Element, Node};

use super::hf::{HfKind, HfType};

#[derive(Debug, Clone)]
pub enum ParaTarget {
    /// 主文档里的段落。
    Body(BodyPara),
    /// 页眉或页脚部件里的段落。
    HeaderFooter {
        kind: HfKind,
        hf_type: HfType,
        n: u32,
        para: BodyPara,
    },
    /// 文本框里的段落。
    InTextBox {
        host: HostPara,
        key: TxbxKey,
        para: BodyPara,
    },
    /// 脚注或尾注里的段落。
    InNote {
        kind: NoteKind,
        note_id: String,
        para: BodyPara,
    },
    /// 批注里的段落。
    InComment { comment_id: String, para: BodyPara },
}

/// 脚注或尾注。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoteKind {
    Footnote,
    Endnote,
}

impl NoteKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Footnote => "footnote",
            Self::Endnote => "endnote",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "footnote" => Some(Self::Footnote),
            "endnote" => Some(Self::Endnote),
            _ => None,
        }
    }

    pub fn item_local(self) -> &'static str {
        self.as_str()
    }

    pub fn reference_local(self) -> &'static str {
        match self {
            Self::Footnote => "footnoteReference",
            Self::Endnote => "endnoteReference",
        }
    }

    pub fn ref_mark_local(self) -> &'static str {
        match self {
            Self::Footnote => "footnoteRef",
            Self::Endnote => "endnoteRef",
        }
    }

    pub fn pr_local(self) -> &'static str {
        match self {
            Self::Footnote => "footnotePr",
            Self::Endnote => "endnotePr",
        }
    }

    pub fn rel_suffix(self) -> &'static str {
        match self {
            Self::Footnote => "/footnotes",
            Self::Endnote => "/endnotes",
        }
    }

    pub fn file_name(self) -> &'static str {
        match self {
            Self::Footnote => "footnotes.xml",
            Self::Endnote => "endnotes.xml",
        }
    }

    pub fn ref_style(self) -> &'static str {
        match self {
            Self::Footnote => "FootnoteReference",
            Self::Endnote => "EndnoteReference",
        }
    }

    pub fn inspect_id(self, note_id: &str) -> String {
        format!("{}:{note_id}", self.as_str())
    }
}

#[derive(Debug, Clone)]
pub enum NoteIdTarget {
    Container {
        kind: NoteKind,
        note_id: String,
    },
    Para {
        kind: NoteKind,
        note_id: String,
        para: BodyPara,
    },
    Sep,
}

#[derive(Debug, Clone)]
pub enum CommentIdTarget {
    Container { comment_id: String },
    Para { comment_id: String, para: BodyPara },
}

#[derive(Debug, Clone)]
pub enum HostPara {
    Body(BodyPara),
    HeaderFooter {
        kind: HfKind,
        hf_type: HfType,
        n: u32,
        para: BodyPara,
    },
    InNote {
        kind: NoteKind,
        note_id: String,
        para: BodyPara,
    },
    InComment {
        comment_id: String,
        para: BodyPara,
    },
}

impl HostPara {
    pub fn body(&self) -> &BodyPara {
        match self {
            Self::Body(p) => p,
            Self::HeaderFooter { para, .. } => para,
            Self::InNote { para, .. } => para,
            Self::InComment { para, .. } => para,
        }
    }
}

#[derive(Debug, Clone)]
pub enum TxbxKey {
    DocPr(String),
    Index(usize),
}

#[derive(Debug, Clone)]
pub struct TxbxTarget {
    pub host: HostPara,
    pub key: TxbxKey,
}

#[derive(Debug, Clone)]
pub enum BodyPara {
    Index(usize),
    Hex(String),
}

pub fn parse_para_id(id: &str, op_index: usize) -> Result<ParaTarget, Error> {
    parse_para_id_at(id, &format!("ops[{op_index}].id"))
}

pub fn parse_para_id_at(id: &str, path: &str) -> Result<ParaTarget, Error> {
    let path = path.to_string();
    if id.contains("/pic:") {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::ParaPrefix,
            },
            path,
        ));
    }
    if let Some((host, after)) = id.split_once("/txbx:") {
        let (key, tail) = parse_txbx_key(after, &path)?;
        let Some(tail) = tail else {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::SetParaOnBox,
                },
                path,
            ));
        };
        let inner = tail.strip_prefix("para:").ok_or_else(|| {
            Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::ParaPrefix,
                },
                path.clone(),
            )
        })?;
        if inner.contains('/') {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::ParaPrefix,
                },
                path.clone(),
            ));
        }
        let host = parse_host_para_id(host, &path)?;
        let para = parse_body_para(inner, &path)?;
        return Ok(ParaTarget::InTextBox { host, key, para });
    }
    if let Some(note) = parse_note_id_at(id, &path)? {
        return match note {
            NoteIdTarget::Sep => Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::NoteSep,
                },
                path,
            )),
            NoteIdTarget::Container { .. } => Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::NoteContainer,
                },
                path,
            )),
            NoteIdTarget::Para {
                kind,
                note_id,
                para,
            } => Ok(ParaTarget::InNote {
                kind,
                note_id,
                para,
            }),
        };
    }
    if let Some(comment) = parse_comment_id_at(id, &path)? {
        return match comment {
            CommentIdTarget::Container { .. } => Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::CommentContainer,
                },
                path,
            )),
            CommentIdTarget::Para { comment_id, para } => {
                Ok(ParaTarget::InComment { comment_id, para })
            }
        };
    }
    match parse_host_para_id(id, &path)? {
        HostPara::Body(p) => Ok(ParaTarget::Body(p)),
        HostPara::HeaderFooter {
            kind,
            hf_type,
            n,
            para,
        } => Ok(ParaTarget::HeaderFooter {
            kind,
            hf_type,
            n,
            para,
        }),
        HostPara::InNote {
            kind,
            note_id,
            para,
        } => Ok(ParaTarget::InNote {
            kind,
            note_id,
            para,
        }),
        HostPara::InComment { comment_id, para } => Ok(ParaTarget::InComment { comment_id, para }),
    }
}

pub fn parse_host_para_id_at(id: &str, path: &str) -> Result<HostPara, Error> {
    if id.contains("/txbx:") || id.contains("/pic:") {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::InsertNeedsHostPara,
            },
            path.to_string(),
        ));
    }
    parse_host_para_id(id, path)
}

pub fn parse_txbx_id(id: &str, op_index: usize) -> Result<TxbxTarget, Error> {
    parse_txbx_id_at(id, &format!("ops[{op_index}].id"))
}

pub fn parse_txbx_id_at(id: &str, path: &str) -> Result<TxbxTarget, Error> {
    let path = path.to_string();
    if id.contains("/pic:") {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::TxbxPrefix,
            },
            path,
        ));
    }
    let Some((host, after)) = id.split_once("/txbx:") else {
        if looks_like_para_or_hf(id) {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::TxbxContainer,
                },
                path,
            ));
        }
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::TxbxPrefix,
            },
            path,
        ));
    };
    let (key, tail) = parse_txbx_key(after, &path)?;
    if tail.is_some() {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::TxbxPrefix,
            },
            path,
        ));
    }
    let host = parse_host_para_id(host, &path)?;
    Ok(TxbxTarget { host, key })
}

fn looks_like_para_or_hf(id: &str) -> bool {
    id.starts_with("para:")
        || id.starts_with("footnote:")
        || id.starts_with("endnote:")
        || id.starts_with("comment:")
        || HfKind::parse(id.split_once(':').map(|(k, _)| k).unwrap_or("")).is_some()
}

pub fn parse_note_id_at(id: &str, path: &str) -> Result<Option<NoteIdTarget>, Error> {
    let path = path.to_string();
    let (kind, rest) = if let Some(rest) = id.strip_prefix("footnote:") {
        (NoteKind::Footnote, rest)
    } else if let Some(rest) = id.strip_prefix("endnote:") {
        (NoteKind::Endnote, rest)
    } else {
        return Ok(None);
    };
    if rest.starts_with("sep:") {
        let sep_type = rest.strip_prefix("sep:").unwrap_or("");
        if !matches!(
            sep_type,
            "separator" | "continuationSeparator" | "continuationNotice"
        ) {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::NoteSep,
                },
                path,
            ));
        }
        return Ok(Some(NoteIdTarget::Sep));
    }
    if rest.is_empty() {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::NotePrefix,
            },
            path,
        ));
    }
    let (note_id, tail) = match rest.split_once('/') {
        Some((nid, t)) => (nid, Some(t)),
        None => (rest, None),
    };
    if note_id.is_empty() {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::NotePrefix,
            },
            path,
        ));
    }
    match tail {
        None => Ok(Some(NoteIdTarget::Container {
            kind,
            note_id: note_id.to_string(),
        })),
        Some(t) => {
            let inner = t.strip_prefix("para:").ok_or_else(|| {
                Error::with_path(
                    ErrorKind::InvalidId {
                        reason: IdReason::NotePrefix,
                    },
                    path.clone(),
                )
            })?;
            if inner.contains('/') {
                return Err(Error::with_path(
                    ErrorKind::InvalidId {
                        reason: IdReason::ParaPrefix,
                    },
                    path.clone(),
                ));
            }
            Ok(Some(NoteIdTarget::Para {
                kind,
                note_id: note_id.to_string(),
                para: parse_body_para(inner, &path)?,
            }))
        }
    }
}

pub fn parse_comment_id_at(id: &str, path: &str) -> Result<Option<CommentIdTarget>, Error> {
    let path = path.to_string();
    let Some(rest) = id.strip_prefix("comment:") else {
        return Ok(None);
    };
    if rest.is_empty() {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::CommentPrefix,
            },
            path,
        ));
    }
    let (comment_id, tail) = match rest.split_once('/') {
        Some((cid, t)) => (cid, Some(t)),
        None => (rest, None),
    };
    if comment_id.is_empty() {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::CommentPrefix,
            },
            path,
        ));
    }
    match tail {
        None => Ok(Some(CommentIdTarget::Container {
            comment_id: comment_id.to_string(),
        })),
        Some(t) => {
            let inner = t.strip_prefix("para:").ok_or_else(|| {
                Error::with_path(
                    ErrorKind::InvalidId {
                        reason: IdReason::CommentPrefix,
                    },
                    path.clone(),
                )
            })?;
            if inner.contains('/') {
                return Err(Error::with_path(
                    ErrorKind::InvalidId {
                        reason: IdReason::ParaPrefix,
                    },
                    path.clone(),
                ));
            }
            Ok(Some(CommentIdTarget::Para {
                comment_id: comment_id.to_string(),
                para: parse_body_para(inner, &path)?,
            }))
        }
    }
}

fn parse_host_para_id(id: &str, path: &str) -> Result<HostPara, Error> {
    let path = path.to_string();
    if let Some(rest) = id.strip_prefix("para:") {
        return Ok(HostPara::Body(parse_body_para(rest, &path)?));
    }
    if let Some((kind_s, rest)) = id.split_once(':') {
        if let Some(kind) = HfKind::parse(kind_s) {
            let (container, para_part) = match rest.split_once('/') {
                Some((c, p)) => (c, Some(p)),
                None => (rest, None),
            };
            let Some((type_s, n_s)) = container.split_once(':') else {
                return Err(Error::with_path(
                    ErrorKind::InvalidId {
                        reason: IdReason::ParaPrefix,
                    },
                    path,
                ));
            };
            let hf_type = HfType::parse(type_s).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::InvalidId {
                        reason: IdReason::ParaPrefix,
                    },
                    path.clone(),
                )
            })?;
            let n: u32 = n_s.parse().map_err(|_| {
                Error::with_path(
                    ErrorKind::InvalidId {
                        reason: IdReason::ParaPrefix,
                    },
                    path.clone(),
                )
            })?;
            if n < 1 {
                return Err(Error::with_path(
                    ErrorKind::InvalidId {
                        reason: IdReason::ParaPrefix,
                    },
                    path,
                ));
            }
            let Some(para_part) = para_part else {
                return Err(Error::with_path(
                    ErrorKind::InvalidId {
                        reason: IdReason::HdrFtrContainer,
                    },
                    path,
                ));
            };
            let para_rest = para_part.strip_prefix("para:").ok_or_else(|| {
                Error::with_path(
                    ErrorKind::InvalidId {
                        reason: IdReason::ParaPrefix,
                    },
                    path.clone(),
                )
            })?;
            if para_rest.contains('/') {
                return Err(Error::with_path(
                    ErrorKind::InvalidId {
                        reason: IdReason::ParaPrefix,
                    },
                    path,
                ));
            }
            return Ok(HostPara::HeaderFooter {
                kind,
                hf_type,
                n,
                para: parse_body_para(para_rest, &path)?,
            });
        }
    }
    if let Some(note) = parse_note_id_at(id, &path)? {
        return match note {
            NoteIdTarget::Sep => Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::NoteSep,
                },
                path,
            )),
            NoteIdTarget::Container { .. } => Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::NoteContainer,
                },
                path,
            )),
            NoteIdTarget::Para {
                kind,
                note_id,
                para,
            } => Ok(HostPara::InNote {
                kind,
                note_id,
                para,
            }),
        };
    }
    if let Some(comment) = parse_comment_id_at(id, &path)? {
        return match comment {
            CommentIdTarget::Container { .. } => Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::CommentContainer,
                },
                path,
            )),
            CommentIdTarget::Para { comment_id, para } => {
                Ok(HostPara::InComment { comment_id, para })
            }
        };
    }
    Err(Error::with_path(
        ErrorKind::InvalidId {
            reason: IdReason::ParaPrefix,
        },
        path,
    ))
}

/// `header:default:1` 这种容器 id，没有 `/para:`。
pub fn parse_hf_container_id(id: &str) -> Option<(HfKind, HfType, u32)> {
    let (kind_s, rest) = id.split_once(':')?;
    let kind = HfKind::parse(kind_s)?;
    if rest.contains('/') {
        return None;
    }
    let (type_s, n_s) = rest.split_once(':')?;
    let hf_type = HfType::parse(type_s)?;
    let n: u32 = n_s.parse().ok()?;
    if n < 1 {
        return None;
    }
    Some((kind, hf_type, n))
}

fn parse_txbx_key<'a>(s: &'a str, path: &str) -> Result<(TxbxKey, Option<&'a str>), Error> {
    if let Some(rest) = s.strip_prefix("index:") {
        let (num, tail) = split_first_seg(rest);
        let n: usize = num.parse().map_err(|_| {
            Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::TxbxIndexNotInt,
                },
                path.to_string(),
            )
        })?;
        if n < 1 {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::TxbxIndexFromOne,
                },
                path.to_string(),
            ));
        }
        return Ok((TxbxKey::Index(n), tail));
    }
    let (key, tail) = split_first_seg(s);
    if key.is_empty() {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::TxbxPrefix,
            },
            path.to_string(),
        ));
    }
    Ok((TxbxKey::DocPr(key.to_string()), tail))
}

fn split_first_seg(s: &str) -> (&str, Option<&str>) {
    match s.split_once('/') {
        Some((a, b)) => (a, Some(b)),
        None => (s, None),
    }
}

pub fn matches_txbx_key(doc_pr_id: &Option<String>, index: usize, key: &TxbxKey) -> bool {
    match key {
        TxbxKey::DocPr(d) => doc_pr_id.as_deref() == Some(d.as_str()),
        TxbxKey::Index(n) => doc_pr_id.is_none() && *n == index,
    }
}

fn parse_body_para(rest: &str, path: &str) -> Result<BodyPara, Error> {
    if let Some(n) = rest.strip_prefix("index:") {
        let idx: usize = n.parse().map_err(|_| {
            Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::ParaIndexNotInt,
                },
                path.to_string(),
            )
        })?;
        if idx < 1 {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::ParaIndexFromOne,
                },
                path.to_string(),
            ));
        }
        return Ok(BodyPara::Index(idx));
    }
    if rest.is_empty() || rest.chars().any(|c| !c.is_ascii_hexdigit()) {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::ParaIdNotHex,
            },
            path.to_string(),
        ));
    }
    Ok(BodyPara::Hex(rest.to_string()))
}

pub fn collect_paragraphs(root: &Element) -> Vec<&Element> {
    let mut out = Vec::new();
    walk_paras(root, &mut |e| {
        if e.local_name() == "p" {
            out.push(e);
        }
    });
    out
}

/// 从 `w:txbxContent` 里收段落。根就是框内容时，不能走 `collect_paragraphs`（它遇框就停）。
pub fn collect_paragraphs_in_txbx(content: &Element) -> Vec<&Element> {
    let mut out = Vec::new();
    for n in &content.children {
        if let Node::Element(e) = n {
            walk_paras(e, &mut |el| {
                if el.local_name() == "p" {
                    out.push(el);
                }
            });
        }
    }
    out
}

/// 遇到 `w:txbxContent` 停住；`mc:AlternateContent` 只进第一个 `Choice`，没有才进 `Fallback`。
fn walk_paras<'a>(el: &'a Element, f: &mut impl FnMut(&'a Element)) {
    if el.local_name() == "txbxContent" {
        return;
    }
    if el.local_name() == "AlternateContent" {
        if let Some(i) = first_alt_child_index(el) {
            if let Node::Element(child) = &el.children[i] {
                walk_paras(child, f);
            }
        }
        return;
    }
    f(el);
    for n in &el.children {
        if let Node::Element(e) = n {
            walk_paras(e, f);
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

pub fn para_stable_id(p: &Element, index: usize) -> (String, bool) {
    // `w14:paraId`：前缀跟着源文件的命名空间声明走，只能比 local name。
    if let Some(id) = p.attr_local("paraId").filter(|s| !s.is_empty()) {
        (format!("para:{id}"), true)
    } else {
        (format!("para:index:{index}"), false)
    }
}

pub fn para_stable_id_in(container_id: &str, p: &Element, index: usize) -> (String, bool) {
    let (base, stable) = para_stable_id(p, index);
    (format!("{container_id}/{base}"), stable)
}

pub fn heading_level(p: &Element) -> Option<u8> {
    let ppr = p.child_by_local("pPr")?;
    if let Some(ps) = ppr.child_by_local("pStyle") {
        if let Some(val) = ps.attr_local("val") {
            if let Some(n) = parse_heading_style(val) {
                return Some(n);
            }
        }
    }
    if let Some(ol) = ppr.child_by_local("outlineLvl") {
        if let Some(val) = ol.attr_local("val") {
            if let Ok(n) = val.parse::<u8>() {
                if n <= 8 {
                    return Some(n + 1);
                }
            }
        }
    }
    None
}

fn parse_heading_style(s: &str) -> Option<u8> {
    let lower = s.to_ascii_lowercase();
    if let Some(rest) = lower.strip_prefix("heading") {
        let n: u8 = rest.parse().ok()?;
        if (1..=9).contains(&n) {
            return Some(n);
        }
    }
    if let Some(rest) = s.strip_prefix("标题") {
        let n: u8 = rest.parse().ok()?;
        if (1..=9).contains(&n) {
            return Some(n);
        }
    }
    None
}

pub fn p_style(p: &Element) -> Option<String> {
    p.child_by_local("pPr")
        .and_then(|ppr| ppr.child_by_local("pStyle"))
        .and_then(|ps| ps.attr_local("val"))
        .map(str::to_string)
}

pub fn visible_text(p: &Element) -> String {
    let mut out = String::new();
    walk_visible(p, false, &mut out);
    out
}

fn walk_visible(el: &Element, in_del: bool, out: &mut String) {
    if el.local_name() == "txbxContent" {
        return;
    }
    if el.local_name() == "AlternateContent" {
        if let Some(i) = first_alt_child_index(el) {
            if let Node::Element(child) = &el.children[i] {
                walk_visible(child, in_del, out);
            }
        }
        return;
    }
    let in_del = in_del || el.local_name() == "del";
    if el.local_name() == "instrText" {
        return;
    }
    if !in_del {
        match el.local_name() {
            "t" => out.push_str(&el.text_content()),
            "tab" => out.push('\t'),
            "br" => out.push('\n'),
            _ => {}
        }
    }
    for n in &el.children {
        if let Node::Element(e) = n {
            walk_visible(e, in_del, out);
        }
    }
}

pub fn find_paragraph<'a>(root: &'a Element, target: &BodyPara) -> Option<&'a Element> {
    let mut idx = 0usize;
    find_p(root, &mut idx, target)
}

pub fn find_paragraph_mut<'a>(root: &'a mut Element, target: &BodyPara) -> Option<&'a mut Element> {
    let mut idx = 0usize;
    find_p_mut(root, &mut idx, target)
}

pub fn find_paragraph_in_txbx<'a>(content: &'a Element, target: &BodyPara) -> Option<&'a Element> {
    let mut idx = 0usize;
    for n in &content.children {
        if let Node::Element(e) = n {
            if let Some(found) = find_p(e, &mut idx, target) {
                return Some(found);
            }
        }
    }
    None
}

pub fn find_paragraph_in_txbx_mut<'a>(
    content: &'a mut Element,
    target: &BodyPara,
) -> Option<&'a mut Element> {
    let mut idx = 0usize;
    for child in content.children.as_mut_slice() {
        if let Node::Element(e) = child {
            if let Some(found) = find_p_mut(e, &mut idx, target) {
                return Some(found);
            }
        }
    }
    None
}

pub fn next_para_hex(root: &Element) -> String {
    let mut max = 0u32;
    walk_all_paras(root, &mut |e| {
        if let Some(id) = e.attr_local("paraId") {
            if let Ok(n) = u32::from_str_radix(id, 16) {
                max = max.max(n);
            }
        }
    });
    format!("{:08X}", max.saturating_add(1).max(1))
}

fn walk_all_paras(el: &Element, f: &mut impl FnMut(&Element)) {
    if el.local_name() == "p" {
        f(el);
    }
    for n in &el.children {
        if let Node::Element(e) = n {
            walk_all_paras(e, f);
        }
    }
}

fn find_p<'a>(el: &'a Element, idx: &mut usize, target: &BodyPara) -> Option<&'a Element> {
    if el.local_name() == "txbxContent" {
        return None;
    }
    if el.local_name() == "AlternateContent" {
        if let Some(i) = first_alt_child_index(el) {
            if let Node::Element(e) = &el.children[i] {
                return find_p(e, idx, target);
            }
        }
        return None;
    }
    if el.local_name() == "p" {
        *idx += 1;
        let hit = match target {
            BodyPara::Index(n) => *idx == *n,
            BodyPara::Hex(hex) => el
                .attr_local("paraId")
                .is_some_and(|id| id.eq_ignore_ascii_case(hex)),
        };
        if hit {
            return Some(el);
        }
    }
    for n in &el.children {
        if let Node::Element(e) = n {
            if let Some(found) = find_p(e, idx, target) {
                return Some(found);
            }
        }
    }
    None
}

fn find_p_mut<'a>(
    el: &'a mut Element,
    idx: &mut usize,
    target: &BodyPara,
) -> Option<&'a mut Element> {
    if el.local_name() == "txbxContent" {
        return None;
    }
    if el.local_name() == "AlternateContent" {
        if let Some(i) = first_alt_child_index(el) {
            if let Node::Element(e) = &mut el.children[i] {
                return find_p_mut(e, idx, target);
            }
        }
        return None;
    }
    if el.local_name() == "p" {
        *idx += 1;
        let hit = match target {
            BodyPara::Index(n) => *idx == *n,
            BodyPara::Hex(hex) => el
                .attr_local("paraId")
                .is_some_and(|id| id.eq_ignore_ascii_case(hex)),
        };
        if hit {
            return Some(el);
        }
    }
    let children = el.children.as_mut_slice();
    for child in children {
        if let Node::Element(e) = child {
            if let Some(found) = find_p_mut(e, idx, target) {
                return Some(found);
            }
        }
    }
    None
}

pub fn set_visible_text(p: &mut Element, text: &str) {
    let mut first = false;
    rewrite_runs(p, &mut first, text);
    if !first {
        let run = new_text_run(p, text);
        let mut insert_at = 0;
        for (i, n) in p.children.iter().enumerate() {
            if let Node::Element(e) = n {
                if e.local_name() == "pPr" {
                    insert_at = i + 1;
                }
            }
        }
        p.children.insert(insert_at, Node::Element(run));
    }
}

fn rewrite_runs(el: &mut Element, first: &mut bool, text: &str) {
    if el.local_name() == "del" || el.local_name() == "txbxContent" {
        return;
    }
    if el.local_name() == "AlternateContent" {
        if let Some(i) = first_alt_child_index(el) {
            if let Node::Element(e) = &mut el.children[i] {
                rewrite_runs(e, first, text);
            }
        }
        return;
    }
    if el.local_name() == "r" {
        let has_t = el.element_children().any(|c| c.local_name() == "t");
        if has_t {
            if !*first {
                fill_first_run(el, text);
                *first = true;
            } else {
                remove_all_t(el);
            }
        }
        return;
    }
    let mut remove = Vec::new();
    for (i, child) in el.children.iter_mut().enumerate() {
        if let Node::Element(e) = child {
            let was_r = e.local_name() == "r";
            rewrite_runs(e, first, text);
            if was_r && is_empty_run(e) {
                remove.push(i);
            }
        }
    }
    for i in remove.into_iter().rev() {
        el.children.remove(i);
    }
}

fn fill_first_run(r: &mut Element, text: &str) {
    let mut seen_t = false;
    let mut drop_t = Vec::new();
    for (i, child) in r.children.iter_mut().enumerate() {
        if let Node::Element(e) = child {
            if e.local_name() == "t" {
                if !seen_t {
                    apply_t_text(e, text);
                    seen_t = true;
                } else {
                    drop_t.push(i);
                }
            }
        }
    }
    for i in drop_t.into_iter().rev() {
        r.children.remove(i);
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

fn remove_all_t(r: &mut Element) {
    r.children.retain(|n| match n {
        Node::Element(e) => e.local_name() != "t",
        Node::Text(_) => true,
    });
}

fn is_empty_run(r: &Element) -> bool {
    !r.element_children().any(|e| e.local_name() != "rPr")
}

pub fn new_paragraph(parent: &Element, para_id: &str, text: &str) -> Element {
    let mut p = Element::new(parent.prefixed("p"));
    p.set_attr("w14:paraId", para_id);
    p.set_attr("w14:textId", para_id);
    if !text.is_empty() {
        p.push_element(new_text_run(&p, text));
    }
    p
}

pub(crate) fn new_text_run(parent: &Element, text: &str) -> Element {
    let mut t = Element::new(parent.prefixed("t"));
    apply_t_text(&mut t, text);
    let mut r = Element::new(parent.prefixed("r"));
    r.push_element(t);
    r
}

fn needs_xml_space(text: &str) -> bool {
    text.starts_with(' ')
        || text.starts_with('\t')
        || text.ends_with(' ')
        || text.ends_with('\t')
        || text.contains('\n')
}

pub fn story_body_mut(root: &mut Element) -> Option<&mut Element> {
    if root.local_name() == "document" {
        root.child_by_local_mut("body")
    } else {
        Some(root)
    }
}

fn p_matches(el: &Element, idx: usize, target: &BodyPara) -> bool {
    match target {
        BodyPara::Index(n) => idx == *n,
        BodyPara::Hex(hex) => el
            .attr_local("paraId")
            .is_some_and(|id| id.eq_ignore_ascii_case(hex)),
    }
}

/// 从部件根走到目标 `w:p` 的子节点下标路径。
pub fn find_paragraph_path(root: &Element, target: &BodyPara) -> Option<Vec<usize>> {
    let mut idx = 0usize;
    let mut path = Vec::new();
    if find_p_path_from(root, &mut idx, target, &mut path) {
        Some(path)
    } else {
        None
    }
}

fn find_p_path_from(
    el: &Element,
    idx: &mut usize,
    target: &BodyPara,
    path: &mut Vec<usize>,
) -> bool {
    if el.local_name() == "txbxContent" {
        return false;
    }
    if el.local_name() == "AlternateContent" {
        if let Some(i) = first_alt_child_index(el) {
            path.push(i);
            let hit = matches!(
                &el.children[i],
                Node::Element(e) if find_p_path_from(e, idx, target, path)
            );
            if hit {
                return true;
            }
            path.pop();
        }
        return false;
    }
    if el.local_name() == "p" {
        *idx += 1;
        if p_matches(el, *idx, target) {
            return true;
        }
    }
    for (i, n) in el.children.iter().enumerate() {
        if let Node::Element(e) = n {
            path.push(i);
            if find_p_path_from(e, idx, target, path) {
                return true;
            }
            path.pop();
        }
    }
    false
}

pub fn collect_paragraph_paths(root: &Element) -> Vec<Vec<usize>> {
    let mut cur = Vec::new();
    let mut out = Vec::new();
    collect_p_paths_from(root, &mut cur, &mut out);
    out
}

fn collect_p_paths_from(el: &Element, path: &mut Vec<usize>, out: &mut Vec<Vec<usize>>) {
    if el.local_name() == "txbxContent" {
        return;
    }
    if el.local_name() == "AlternateContent" {
        if let Some(i) = first_alt_child_index(el) {
            path.push(i);
            if let Node::Element(e) = &el.children[i] {
                collect_p_paths_from(e, path, out);
            }
            path.pop();
        }
        return;
    }
    if el.local_name() == "p" {
        out.push(path.clone());
    }
    for (i, n) in el.children.iter().enumerate() {
        if let Node::Element(e) = n {
            path.push(i);
            collect_p_paths_from(e, path, out);
            path.pop();
        }
    }
}

pub fn element_at<'a>(root: &'a Element, path: &[usize]) -> Option<&'a Element> {
    let mut cur = root;
    for &i in path {
        cur = match cur.children.get(i)? {
            Node::Element(e) => e,
            _ => return None,
        };
    }
    Some(cur)
}

pub fn element_at_mut<'a>(root: &'a mut Element, path: &[usize]) -> Option<&'a mut Element> {
    let mut cur = root;
    for &i in path {
        cur = match cur.children.get_mut(i)? {
            Node::Element(e) => e,
            _ => return None,
        };
    }
    Some(cur)
}

pub fn parent_and_index_mut<'a>(
    root: &'a mut Element,
    path: &[usize],
) -> Option<(&'a mut Element, usize)> {
    let (last, prefix) = path.split_last()?;
    let mut cur = root;
    for &i in prefix {
        cur = match cur.children.get_mut(i)? {
            Node::Element(e) => e,
            _ => return None,
        };
    }
    Some((cur, *last))
}

pub fn count_direct_p(parent: &Element) -> usize {
    parent
        .element_children()
        .filter(|e| e.local_name() == "p")
        .count()
}

pub fn append_before_body_sect_pr(parent: &mut Element, new_p: Element) {
    let pos = parent
        .children
        .iter()
        .position(|n| matches!(n, Node::Element(e) if e.local_name() == "sectPr"));
    match pos {
        Some(i) => parent.children.insert(i, Node::Element(new_p)),
        None => parent.children.push(Node::Element(new_p)),
    }
}

fn walk_story_descendants(el: &Element, f: &mut impl FnMut(&Element)) {
    if el.local_name() == "txbxContent" {
        return;
    }
    if el.local_name() == "AlternateContent" {
        if let Some(i) = first_alt_child_index(el) {
            if let Node::Element(child) = &el.children[i] {
                walk_story_descendants(child, f);
            }
        }
        return;
    }
    f(el);
    for n in &el.children {
        if let Node::Element(e) = n {
            walk_story_descendants(e, f);
        }
    }
}

pub fn para_has_revision(p: &Element) -> bool {
    let mut hit = false;
    walk_story_descendants(p, &mut |e| {
        if matches!(e.local_name(), "ins" | "del" | "moveFrom" | "moveTo") {
            hit = true;
        }
    });
    hit
}

pub fn para_has_unmatched_range(p: &Element) -> bool {
    unmatched_pair(p, "bookmarkStart", "bookmarkEnd")
        || unmatched_pair(p, "commentRangeStart", "commentRangeEnd")
        || unmatched_pair(p, "permStart", "permEnd")
}

fn unmatched_pair(p: &Element, start: &str, end: &str) -> bool {
    let mut starts = Vec::new();
    let mut ends = Vec::new();
    walk_story_descendants(p, &mut |e| {
        let name = e.local_name();
        if name == start {
            if let Some(id) = e.attr_local("id") {
                starts.push(id.to_string());
            }
        } else if name == end {
            if let Some(id) = e.attr_local("id") {
                ends.push(id.to_string());
            }
        }
    });
    starts.sort();
    ends.sort();
    starts != ends
}

pub fn para_has_sect_pr(p: &Element) -> bool {
    p.child_by_local("pPr")
        .and_then(|ppr| ppr.child_by_local("sectPr"))
        .is_some()
}

pub fn take_ppr_sect_pr(p: &mut Element) -> Option<Element> {
    let taken = {
        let ppr = p.child_by_local_mut("pPr")?;
        let i = ppr
            .children
            .iter()
            .position(|n| matches!(n, Node::Element(e) if e.local_name() == "sectPr"))?;
        match ppr.children.remove(i) {
            Node::Element(e) => e,
            _ => return None,
        }
    };
    let empty_ppr = p
        .child_by_local("pPr")
        .is_some_and(|ppr| ppr.element_children().next().is_none());
    if empty_ppr {
        p.children
            .retain(|n| !matches!(n, Node::Element(e) if e.local_name() == "pPr"));
    }
    Some(taken)
}

pub fn put_ppr_sect_pr(p: &mut Element, sect: Element) {
    if p.child_by_local("pPr").is_none() {
        let ppr = Element::new(p.prefixed("pPr"));
        p.children.insert(0, Node::Element(ppr));
    }
    if let Some(ppr) = p.child_by_local_mut("pPr") {
        ppr.push_element(sect);
    }
}

pub fn is_last_collected(root: &Element, target: &BodyPara) -> bool {
    let paras = collect_paragraphs(root);
    let Some(last) = paras.last() else {
        return false;
    };
    match target {
        BodyPara::Index(n) => *n == paras.len(),
        BodyPara::Hex(hex) => last
            .attr_local("paraId")
            .is_some_and(|id| id.eq_ignore_ascii_case(hex)),
    }
}
