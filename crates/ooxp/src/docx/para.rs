//! 主文档里的段落：定位、可见文本、大纲级别。

use crate::error::{Error, ErrorKind, IdReason};
use crate::xml::{Element, Node};

use super::hf::{self, HfKind, HfType};

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

#[derive(Debug, Clone, PartialEq, Eq)]
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

/// 目录、题注、交叉引用、索引、索引项、引文、书目块所在的故事。
#[derive(Debug, Clone)]
pub enum FieldBlockHost {
    Body,
    HeaderFooter {
        kind: HfKind,
        hf_type: HfType,
        n: u32,
    },
    TextBox {
        host: HostPara,
        key: TxbxKey,
    },
    Note {
        kind: NoteKind,
        note_id: String,
    },
    Comment {
        comment_id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TxbxKey {
    DocPr(String),
    Index(usize),
}

#[derive(Debug, Clone)]
pub struct TxbxTarget {
    pub host: HostPara,
    pub key: TxbxKey,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BodyPara {
    Index(usize),
    Hex(String),
}

pub fn parse_para_id(id: &str, op_index: usize) -> Result<ParaTarget, Error> {
    parse_para_id_at(id, &format!("ops[{op_index}].id"))
}

pub fn parse_para_id_at(id: &str, path: &str) -> Result<ParaTarget, Error> {
    let path = path.to_string();
    if id.contains("/pic:")
        || id.contains("/cxn:")
        || id.contains("/shape:")
        || id.contains("/ink:")
        || id.contains("/model3d:")
        || id.contains("/table:")
    {
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
    if id.contains("/txbx:")
        || id.contains("/pic:")
        || id.contains("/cxn:")
        || id.contains("/shape:")
        || id.contains("/ink:")
        || id.contains("/model3d:")
    {
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
    if id.contains("/pic:")
        || id.contains("/cxn:")
        || id.contains("/shape:")
        || id.contains("/ink:")
        || id.contains("/model3d:")
        || id.contains("/table:")
    {
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

pub fn matches_txbx_key(
    el: &Element,
    doc_pr_id: &Option<String>,
    index: usize,
    key: &TxbxKey,
) -> bool {
    match key {
        TxbxKey::DocPr(d) => doc_pr_id.as_deref() == Some(d.as_str()),
        TxbxKey::Index(n) => {
            doc_pr_id.is_none()
                && crate::xml::pin_matches(el, &crate::xml::pin_attr("txbx"), *n, index)
        }
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

/// 根是 `txbxContent` 时收框内段；否则与 `collect_paragraphs` 相同。
pub fn collect_story_paragraphs(root: &Element) -> Vec<&Element> {
    if root.local_name() == "txbxContent" {
        collect_paragraphs_in_txbx(root)
    } else {
        collect_paragraphs(root)
    }
}

/// 若 `root` 是 `txbxContent`，对每个子元素调用 `walk`；否则对 `root` 自身调用。
pub fn walk_from_story_root(
    root: &Element,
    path: &mut Vec<usize>,
    mut walk: impl FnMut(&Element, &mut Vec<usize>),
) {
    if root.local_name() == "txbxContent" {
        for (i, n) in root.children.iter().enumerate() {
            if let Node::Element(e) = n {
                path.push(i);
                walk(e, path);
                path.pop();
            }
        }
    } else {
        walk(root, path);
    }
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

/// 遇到 `w:txbxContent` 停住。`mc:AlternateContent` 进 `alternate_child` 选出的那一支。
fn walk_paras<'a>(root: &'a Element, f: &mut impl FnMut(&'a Element)) {
    let mut stack = vec![root];
    while let Some(el) = stack.pop() {
        if el.local_name() == "txbxContent" {
            continue;
        }
        if el.local_name() == "AlternateContent" {
            if let Some(i) = first_alt_child_index(el) {
                if let Node::Element(child) = &el.children[i] {
                    stack.push(child);
                }
            }
            continue;
        }
        f(el);
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

fn walk_visible(root: &Element, in_del: bool, out: &mut String) {
    let mut stack = vec![(root, in_del)];
    while let Some((el, in_del)) = stack.pop() {
        if el.local_name() == "txbxContent" {
            continue;
        }
        if el.local_name() == "AlternateContent" {
            if let Some(i) = first_alt_child_index(el) {
                if let Node::Element(child) = &el.children[i] {
                    stack.push((child, in_del));
                }
            }
            continue;
        }
        let in_del = in_del || el.local_name() == "del";
        if el.local_name() == "instrText" {
            continue;
        }
        if !in_del {
            match el.local_name() {
                "t" => out.push_str(&el.text_content()),
                "tab" => out.push('\t'),
                "br" => out.push('\n'),
                _ => {}
            }
        }
        for node in el.children.iter().rev() {
            if let Node::Element(child) = node {
                stack.push((child, in_del));
            }
        }
    }
}

pub fn find_paragraph<'a>(root: &'a Element, target: &BodyPara) -> Option<&'a Element> {
    if root.local_name() == "txbxContent" {
        return find_paragraph_in_txbx(root, target);
    }
    let mut idx = 0usize;
    find_p(root, &mut idx, target)
}

pub fn find_paragraph_mut<'a>(root: &'a mut Element, target: &BodyPara) -> Option<&'a mut Element> {
    if root.local_name() == "txbxContent" {
        return find_paragraph_in_txbx_mut(root, target);
    }
    let mut idx = 0usize;
    find_p_mut(root, &mut idx, target)
}

pub fn same_para(p: &Element, target: &BodyPara, index: usize) -> bool {
    match target {
        BodyPara::Index(n) => index_matches(p, *n, index),
        BodyPara::Hex(hex) => p
            .attr_local("paraId")
            .is_some_and(|id| id.eq_ignore_ascii_case(hex)),
    }
}

/// 必须以 `xml::TRANSIENT_ATTR_PREFIX` 开头，写回时才会被丢掉。
const PIN_ATTR: &str = "ooxp:pin";

/// 有 `ooxp:pin` 时按 apply 开始时的序号认。没有这个标记的新段落不占用旧序号。
fn index_matches(p: &Element, wanted: usize, live: usize) -> bool {
    crate::xml::pin_matches(p, PIN_ATTR, wanted, live)
}

fn paragraph_pin(el: &Element) -> crate::xml::PinNode {
    if matches!(
        el.local_name(),
        "txbxContent" | "footnote" | "endnote" | "comment"
    ) {
        crate::xml::PinNode::Restart
    } else if el.local_name() == "AlternateContent" {
        crate::xml::PinNode::Only(first_alt_child_index(el))
    } else if el.local_name() == "p" {
        crate::xml::PinNode::Member
    } else {
        crate::xml::PinNode::Descend
    }
}

/// 按 `find_p` 的同一套顺序给已有段落记下序号。文本框、注、批注各自从 1 起。
pub(crate) fn pin_paragraph_indexes(root: &mut Element) {
    let attr = crate::xml::pin_attr("");
    crate::xml::pin_tree(root, &attr, &paragraph_pin);
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
    crate::xml::walk_elements(el, &mut |node| {
        if node.local_name() == "p" {
            f(node);
        }
    });
}

fn find_p<'a>(el: &'a Element, idx: &mut usize, target: &BodyPara) -> Option<&'a Element> {
    crate::xml::search(el, |el| {
        if el.local_name() == "txbxContent" {
            return crate::xml::Search::Skip;
        }
        if el.local_name() == "AlternateContent" {
            if let Some(i) = first_alt_child_index(el) {
                if let Node::Element(e) = &el.children[i] {
                    return crate::xml::Search::Only(e);
                }
            }
            return crate::xml::Search::Skip;
        }
        if el.local_name() == "p" {
            *idx += 1;
            let hit = match target {
                BodyPara::Index(n) => index_matches(el, *n, *idx),
                BodyPara::Hex(hex) => el
                    .attr_local("paraId")
                    .is_some_and(|id| id.eq_ignore_ascii_case(hex)),
            };
            if hit {
                return crate::xml::Search::Hit(el);
            }
        }
        crate::xml::Search::down(el)
    })
}

fn find_p_mut<'a>(
    el: &'a mut Element,
    idx: &mut usize,
    target: &BodyPara,
) -> Option<&'a mut Element> {
    crate::xml::search_mut(el, |el| {
        if el.local_name() == "txbxContent" {
            return crate::xml::SearchMut::Skip;
        }
        if el.local_name() == "AlternateContent" {
            if let Some(i) = first_alt_child_index(el) {
                if matches!(&el.children[i], Node::Element(_)) {
                    return crate::xml::SearchMut::Only(i);
                }
            }
            return crate::xml::SearchMut::Skip;
        }
        if el.local_name() == "p" {
            *idx += 1;
            let hit = match target {
                BodyPara::Index(n) => index_matches(el, *n, *idx),
                BodyPara::Hex(hex) => el
                    .attr_local("paraId")
                    .is_some_and(|id| id.eq_ignore_ascii_case(hex)),
            };
            if hit {
                return crate::xml::SearchMut::Hit;
            }
        }
        crate::xml::SearchMut::down(el)
    })
}

pub struct WrittenRun {
    pub text: String,
    pub rpr: Option<Element>,
}

pub fn first_text_rpr(p: &Element) -> Option<Element> {
    find_first_text_run(p).and_then(|r| r.child_by_local("rPr").cloned())
}

pub fn rpr_at_char(p: &Element, at: usize) -> Option<Element> {
    let mut seen = 0usize;
    rpr_covering(p, at, &mut seen)
}

pub fn replace_visible_runs(p: &mut Element, runs: &[WrittenRun]) {
    let insert_at = first_direct_text_run_index(p).unwrap_or_else(|| after_ppr(p));
    strip_visible_text_runs(p);
    let insert_at = insert_at.min(p.children.len());
    for (i, run) in runs.iter().enumerate() {
        let mut el = new_text_run(p, &run.text);
        if let Some(rpr) = &run.rpr {
            el.children.insert(0, Node::Element(rpr.clone()));
        }
        p.children.insert(insert_at + i, Node::Element(el));
    }
}

fn after_ppr(p: &Element) -> usize {
    let mut insert_at = 0;
    for (i, n) in p.children.iter().enumerate() {
        if let Node::Element(e) = n {
            if e.local_name() == "pPr" {
                insert_at = i + 1;
            }
        }
    }
    insert_at
}

fn first_direct_text_run_index(p: &Element) -> Option<usize> {
    p.children.iter().position(|n| {
        let Node::Element(e) = n else {
            return false;
        };
        e.local_name() == "r" && e.element_children().any(|c| c.local_name() == "t")
    })
}

fn find_first_text_run(el: &Element) -> Option<&Element> {
    crate::xml::search(el, |el| {
        if el.local_name() == "del" || el.local_name() == "txbxContent" {
            return crate::xml::Search::Skip;
        }
        if el.local_name() == "AlternateContent" {
            if let Some(i) = first_alt_child_index(el) {
                if let Node::Element(e) = &el.children[i] {
                    return crate::xml::Search::Only(e);
                }
            }
            return crate::xml::Search::Skip;
        }
        if el.local_name() == "r" && el.element_children().any(|c| c.local_name() == "t") {
            return crate::xml::Search::Hit(el);
        }
        crate::xml::Search::down(el)
    })
}

fn rpr_covering(el: &Element, at: usize, seen: &mut usize) -> Option<Element> {
    if el.local_name() == "del" || el.local_name() == "txbxContent" {
        return None;
    }
    if el.local_name() == "AlternateContent" {
        if let Some(i) = first_alt_child_index(el) {
            if let Node::Element(e) = &el.children[i] {
                return rpr_covering(e, at, seen);
            }
        }
        return None;
    }
    if el.local_name() == "r" {
        let rpr = el.child_by_local("rPr").cloned();
        for child in el.element_children() {
            if child.local_name() == "t" {
                let n = child.text_content().chars().count();
                if at < *seen + n {
                    return rpr;
                }
                *seen += n;
            }
        }
        return None;
    }
    for child in el.element_children() {
        if let Some(found) = rpr_covering(child, at, seen) {
            return Some(found);
        }
    }
    None
}

fn strip_visible_text_runs(el: &mut Element) {
    if el.local_name() == "del" || el.local_name() == "txbxContent" {
        return;
    }
    if el.local_name() == "AlternateContent" {
        if let Some(i) = first_alt_child_index(el) {
            if let Node::Element(e) = &mut el.children[i] {
                strip_visible_text_runs(e);
            }
        }
        return;
    }
    if el.local_name() == "r" {
        if el.element_children().any(is_visible_text_el) {
            remove_visible_text(el);
        }
        return;
    }
    let mut remove = Vec::new();
    for (i, child) in el.children.iter_mut().enumerate() {
        if let Node::Element(e) = child {
            let was_r = e.local_name() == "r";
            strip_visible_text_runs(e);
            if was_r && is_empty_run(e) {
                remove.push(i);
            }
        }
    }
    for i in remove.into_iter().rev() {
        el.children.remove(i);
    }
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
        let has_visible = el.element_children().any(is_visible_text_el);
        if has_visible {
            if !*first {
                fill_first_run(el, text);
                *first = true;
            } else {
                remove_visible_text(el);
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
    r.children.retain(|n| match n {
        Node::Element(e) => !is_visible_text_el(e),
        Node::Text(_) | Node::Raw(_) => true,
    });
    let insert_at = r
        .children
        .iter()
        .position(|n| matches!(n, Node::Element(e) if e.local_name() != "rPr"))
        .unwrap_or(r.children.len());
    let pieces = visible_children(r, text);
    for (offset, piece) in pieces.into_iter().enumerate() {
        r.children.insert(insert_at + offset, Node::Element(piece));
    }
}

fn is_visible_text_el(el: &Element) -> bool {
    matches!(el.local_name(), "t" | "tab" | "br")
}

pub(crate) enum VisiblePiece {
    Text(String),
    Tab,
    Break,
}

pub(crate) fn split_visible_text(text: &str) -> Vec<VisiblePiece> {
    if text.is_empty() {
        return vec![VisiblePiece::Text(String::new())];
    }
    let mut out = Vec::new();
    let mut buf = String::new();
    for c in text.chars() {
        match c {
            '\t' => {
                if !buf.is_empty() {
                    out.push(VisiblePiece::Text(std::mem::take(&mut buf)));
                }
                out.push(VisiblePiece::Tab);
            }
            '\n' => {
                if !buf.is_empty() {
                    out.push(VisiblePiece::Text(std::mem::take(&mut buf)));
                }
                out.push(VisiblePiece::Break);
            }
            other => buf.push(other),
        }
    }
    if !buf.is_empty() {
        out.push(VisiblePiece::Text(buf));
    }
    out
}

fn visible_children(host: &Element, text: &str) -> Vec<Element> {
    split_visible_text(text)
        .into_iter()
        .map(|piece| match piece {
            VisiblePiece::Text(s) => {
                let mut t = Element::new(host.prefixed("t"));
                apply_t_text(&mut t, &s);
                t
            }
            VisiblePiece::Tab => Element::new(host.prefixed("tab")),
            VisiblePiece::Break => Element::new(host.prefixed("br")),
        })
        .collect()
}

pub(crate) fn visible_run_xml(text: &str) -> String {
    if text.is_empty() {
        return "<w:r><w:t/></w:r>".into();
    }
    let mut inner = String::new();
    for piece in split_visible_text(text) {
        match piece {
            VisiblePiece::Text(s) => {
                let space = if needs_xml_space(&s) {
                    r#" xml:space="preserve""#
                } else {
                    ""
                };
                inner.push_str(&format!(
                    "<w:t{space}>{}</w:t>",
                    crate::xml::escape_text(&s)
                ));
            }
            VisiblePiece::Tab => inner.push_str("<w:tab/>"),
            VisiblePiece::Break => inner.push_str("<w:br/>"),
        }
    }
    format!("<w:r>{inner}</w:r>")
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

fn remove_visible_text(r: &mut Element) {
    r.children.retain(|n| match n {
        Node::Element(e) => !is_visible_text_el(e),
        Node::Text(_) | Node::Raw(_) => true,
    });
}

fn is_empty_run(r: &Element) -> bool {
    !r.element_children().any(|e| e.local_name() != "rPr")
}

pub fn new_paragraph(parent: &Element, para_id: &str, text: &str) -> Element {
    let mut p = Element::new(parent.prefixed("p"));
    // 空标记：本批新建的段落不占用 apply 开始时的序号。写出时会丢掉。
    p.set_attr(PIN_ATTR, "");
    p.set_attr("w14:paraId", para_id);
    p.set_attr("w14:textId", para_id);
    if !text.is_empty() {
        p.push_element(new_text_run(&p, text));
    }
    p
}

pub(crate) fn new_text_run(parent: &Element, text: &str) -> Element {
    let mut r = Element::new(parent.prefixed("r"));
    crate::xml::mark_new(&mut r, "run");
    for piece in visible_children(&r, text) {
        r.push_element(piece);
    }
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
    same_para(el, target, idx)
}

/// 从部件根走到目标 `w:p` 的子节点下标路径。
pub fn find_paragraph_path(root: &Element, target: &BodyPara) -> Option<Vec<usize>> {
    if root.local_name() == "txbxContent" {
        return find_paragraph_path_in_txbx(root, target);
    }
    let mut idx = 0usize;
    let mut path = Vec::new();
    if find_p_path_from(root, &mut idx, target, &mut path) {
        Some(path)
    } else {
        None
    }
}

/// 从 `w:txbxContent` 走到目标 `w:p`。根就是框内容时不能走 `find_paragraph_path`。
pub fn find_paragraph_path_in_txbx(content: &Element, target: &BodyPara) -> Option<Vec<usize>> {
    let mut idx = 0usize;
    let mut path = Vec::new();
    for (i, n) in content.children.iter().enumerate() {
        if let Node::Element(e) = n {
            path.push(i);
            if find_p_path_from(e, &mut idx, target, &mut path) {
                return Some(path);
            }
            path.pop();
        }
    }
    None
}

fn find_p_path_from(
    el: &Element,
    idx: &mut usize,
    target: &BodyPara,
    path: &mut Vec<usize>,
) -> bool {
    let mut stack: Vec<Vec<usize>> = vec![Vec::new()];
    while let Some(rel) = stack.pop() {
        let Some(node) = element_at(el, &rel) else {
            continue;
        };
        if node.local_name() == "txbxContent" {
            continue;
        }
        if node.local_name() == "AlternateContent" {
            if let Some(i) = first_alt_child_index(node) {
                if matches!(&node.children[i], Node::Element(_)) {
                    let mut child = rel.clone();
                    child.push(i);
                    stack.push(child);
                }
            }
            continue;
        }
        if node.local_name() == "p" {
            *idx += 1;
            if p_matches(node, *idx, target) {
                path.extend_from_slice(&rel);
                return true;
            }
        }
        let indexes: Vec<usize> = node
            .children
            .iter()
            .enumerate()
            .filter_map(|(i, n)| matches!(n, Node::Element(_)).then_some(i))
            .collect();
        for i in indexes.into_iter().rev() {
            let mut child = rel.clone();
            child.push(i);
            stack.push(child);
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

/// 按收集顺序给还没有编号的元素记下序号。编号相对 `root`。
pub(crate) fn pin_collected(root: &mut Element, paths: &[Vec<usize>], family: &str) {
    let attr = crate::xml::pin_attr(family);
    for (i, path) in paths.iter().enumerate() {
        if let Some(el) = element_at_mut(root, path) {
            if el.attr(&attr).is_none() {
                el.set_attr(&attr, (i + 1).to_string());
            }
        }
    }
}

/// 在收集结果里按编号找出第 `n` 个。空编号是本批新建的，不占旧序号。
pub(crate) fn pick_collected<T>(
    root: &Element,
    items: Vec<T>,
    n: usize,
    family: &str,
    path_of: impl Fn(&T) -> &[usize],
) -> Option<T> {
    let attr = crate::xml::pin_attr(family);
    let mut live = 0usize;
    for item in items {
        live += 1;
        let Some(el) = element_at(root, path_of(&item)) else {
            continue;
        };
        if crate::xml::pin_matches(el, &attr, n, live) {
            return Some(item);
        }
    }
    None
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

fn walk_story_descendants(root: &Element, f: &mut impl FnMut(&Element)) {
    let mut stack = vec![root];
    while let Some(el) = stack.pop() {
        if el.local_name() == "txbxContent" {
            continue;
        }
        if el.local_name() == "AlternateContent" {
            if let Some(i) = first_alt_child_index(el) {
                if let Node::Element(child) = &el.children[i] {
                    stack.push(child);
                }
            }
            continue;
        }
        f(el);
        for node in el.children.iter().rev() {
            if let Node::Element(child) = node {
                stack.push(child);
            }
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
    same_para(last, target, paras.len())
}

use super::request::DeleteParaOp;
use super::request::InsertParaOp;
use super::request::InsertParaPlace;
use super::request::ParaRunSource;
use crate::opc::Package;
use crate::xml::Document;
use std::collections::HashMap;
type RunSlice<'a> = (
    String,
    Option<Element>,
    Option<&'a super::request::para_remain::RunFormatOwned>,
);
use super::comments;
use super::format;
use super::notes;
use super::request::SetParaRunsOp;
use super::request::SetParaTextOp;
use super::textbox;
// 从 apply 搬来的操作体。
pub(crate) fn apply_para_text(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &SetParaTextOp,
    i: usize,
) -> Result<(), Error> {
    let target = super::para::parse_para_id(&op.id, i)?;
    match &target {
        ParaTarget::Body(p) => {
            let doc = super::apply::load_part(pkg, docs, doc_part)?;
            super::field::refuse_set_text_if_field(&doc.root, p, i)?;
            super::math::refuse_set_text_if_math(&doc.root, p, i)?;
            super::ruby::refuse_set_text_if_ruby(&doc.root, p, i)?;
            super::sdt::refuse_set_text_if_sdt(&doc.root, p, i)?;
            let el = super::para::find_paragraph_mut(&mut doc.root, p).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::ParaNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )
            })?;
            super::revision::refuse_set_text_if_revision(el, i)?;
            super::link::refuse_set_text_if_hyperlink(el, i)?;
            super::para::set_visible_text(el, &op.text);
            dirty.insert(doc_part.to_string(), ());
        }
        ParaTarget::HeaderFooter {
            kind,
            hf_type,
            n,
            para,
        } => {
            let part_name = super::apply::part_for_hf(catalog, *kind, *hf_type, *n, &op.id, i)?;
            let doc = super::apply::load_part(pkg, docs, &part_name)?;
            super::field::refuse_set_text_if_field(&doc.root, para, i)?;
            super::math::refuse_set_text_if_math(&doc.root, para, i)?;
            super::ruby::refuse_set_text_if_ruby(&doc.root, para, i)?;
            super::sdt::refuse_set_text_if_sdt(&doc.root, para, i)?;
            let el = super::para::find_paragraph_mut(&mut doc.root, para).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::ParaNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )
            })?;
            super::revision::refuse_set_text_if_revision(el, i)?;
            super::link::refuse_set_text_if_hyperlink(el, i)?;
            super::para::set_visible_text(el, &op.text);
            dirty.insert(part_name, ());
        }
        ParaTarget::InTextBox { host, key, para } => {
            let part_name = super::apply::part_for_host(pkg, catalog, doc_part, host, &op.id, i)?;
            let doc = super::apply::load_part(pkg, docs, &part_name)?;
            let host_p =
                super::para::find_paragraph_mut(&mut doc.root, host.body()).ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::ParaNotFound { id: op.id.clone() },
                        format!("ops[{i}].id"),
                    )
                })?;
            let content = textbox::find_txbx_content_by_key_mut(host_p, key).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::ParaNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )
            })?;
            super::field::refuse_set_text_if_field(content, para, i)?;
            super::math::refuse_set_text_if_math(content, para, i)?;
            super::ruby::refuse_set_text_if_ruby(content, para, i)?;
            super::sdt::refuse_set_text_if_sdt(content, para, i)?;
            let el = super::para::find_paragraph_in_txbx_mut(content, para).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::ParaNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )
            })?;
            super::revision::refuse_set_text_if_revision(el, i)?;
            super::link::refuse_set_text_if_hyperlink(el, i)?;
            super::para::set_visible_text(el, &op.text);
            dirty.insert(part_name, ());
        }
        ParaTarget::InNote {
            kind,
            note_id,
            para,
        } => {
            let part_name = notes::part_name(pkg, doc_part, *kind)?.ok_or_else(|| {
                Error::with_path(
                    ErrorKind::ParaNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )
            })?;
            let doc = super::apply::load_part(pkg, docs, &part_name)?;
            let note =
                notes::find_user_note_mut(&mut doc.root, *kind, note_id).ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::ParaNotFound { id: op.id.clone() },
                        format!("ops[{i}].id"),
                    )
                })?;
            let el = super::para::find_paragraph_mut(note, para).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::ParaNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )
            })?;
            super::revision::refuse_set_text_if_revision(el, i)?;
            super::para::set_visible_text(el, &op.text);
            dirty.insert(part_name, ());
        }
        ParaTarget::InComment { comment_id, para } => {
            let part_name = comments::part_name(pkg, doc_part)?.ok_or_else(|| {
                Error::with_path(
                    ErrorKind::ParaNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )
            })?;
            let doc = super::apply::load_part(pkg, docs, &part_name)?;
            let comment =
                comments::find_comment_mut(&mut doc.root, comment_id).ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::ParaNotFound { id: op.id.clone() },
                        format!("ops[{i}].id"),
                    )
                })?;
            comments::reject_ink(comment, &op.id, i)?;
            let el = super::para::find_paragraph_mut(comment, para).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::ParaNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )
            })?;
            super::revision::refuse_set_text_if_revision(el, i)?;
            super::para::set_visible_text(el, &op.text);
            dirty.insert(part_name, ());
        }
    }
    Ok(())
}
pub(crate) fn apply_para_runs(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &SetParaRunsOp,
    i: usize,
) -> Result<(), Error> {
    let target = super::para::parse_para_id(&op.id, i)?;
    match &target {
        ParaTarget::Body(p) => {
            let doc = super::apply::load_part(pkg, docs, doc_part)?;
            super::field::refuse_set_text_if_field(&doc.root, p, i)?;
            super::math::refuse_set_text_if_math(&doc.root, p, i)?;
            super::ruby::refuse_set_text_if_ruby(&doc.root, p, i)?;
            super::sdt::refuse_set_text_if_sdt(&doc.root, p, i)?;
            let el = super::para::find_paragraph_mut(&mut doc.root, p).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::ParaNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )
            })?;
            super::revision::refuse_set_text_if_revision(el, i)?;
            super::link::refuse_set_text_if_hyperlink(el, i)?;
            write_para_runs(el, op, i)?;
            dirty.insert(doc_part.to_string(), ());
        }
        ParaTarget::HeaderFooter {
            kind,
            hf_type,
            n,
            para,
        } => {
            let part_name = super::apply::part_for_hf(catalog, *kind, *hf_type, *n, &op.id, i)?;
            let doc = super::apply::load_part(pkg, docs, &part_name)?;
            super::field::refuse_set_text_if_field(&doc.root, para, i)?;
            super::math::refuse_set_text_if_math(&doc.root, para, i)?;
            super::ruby::refuse_set_text_if_ruby(&doc.root, para, i)?;
            super::sdt::refuse_set_text_if_sdt(&doc.root, para, i)?;
            let el = super::para::find_paragraph_mut(&mut doc.root, para).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::ParaNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )
            })?;
            super::revision::refuse_set_text_if_revision(el, i)?;
            super::link::refuse_set_text_if_hyperlink(el, i)?;
            write_para_runs(el, op, i)?;
            dirty.insert(part_name, ());
        }
        ParaTarget::InTextBox { host, key, para } => {
            let part_name = super::apply::part_for_host(pkg, catalog, doc_part, host, &op.id, i)?;
            let doc = super::apply::load_part(pkg, docs, &part_name)?;
            let host_p =
                super::para::find_paragraph_mut(&mut doc.root, host.body()).ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::ParaNotFound { id: op.id.clone() },
                        format!("ops[{i}].id"),
                    )
                })?;
            let content = textbox::find_txbx_content_by_key_mut(host_p, key).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::ParaNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )
            })?;
            super::field::refuse_set_text_if_field(content, para, i)?;
            super::math::refuse_set_text_if_math(content, para, i)?;
            super::ruby::refuse_set_text_if_ruby(content, para, i)?;
            super::sdt::refuse_set_text_if_sdt(content, para, i)?;
            let el = super::para::find_paragraph_in_txbx_mut(content, para).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::ParaNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )
            })?;
            super::revision::refuse_set_text_if_revision(el, i)?;
            super::link::refuse_set_text_if_hyperlink(el, i)?;
            write_para_runs(el, op, i)?;
            dirty.insert(part_name, ());
        }
        ParaTarget::InNote {
            kind,
            note_id,
            para,
        } => {
            let part_name = notes::part_name(pkg, doc_part, *kind)?.ok_or_else(|| {
                Error::with_path(
                    ErrorKind::ParaNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )
            })?;
            let doc = super::apply::load_part(pkg, docs, &part_name)?;
            let note =
                notes::find_user_note_mut(&mut doc.root, *kind, note_id).ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::ParaNotFound { id: op.id.clone() },
                        format!("ops[{i}].id"),
                    )
                })?;
            let el = super::para::find_paragraph_mut(note, para).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::ParaNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )
            })?;
            super::revision::refuse_set_text_if_revision(el, i)?;
            write_para_runs(el, op, i)?;
            dirty.insert(part_name, ());
        }
        ParaTarget::InComment { comment_id, para } => {
            let part_name = comments::part_name(pkg, doc_part)?.ok_or_else(|| {
                Error::with_path(
                    ErrorKind::ParaNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )
            })?;
            let doc = super::apply::load_part(pkg, docs, &part_name)?;
            let comment =
                comments::find_comment_mut(&mut doc.root, comment_id).ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::ParaNotFound { id: op.id.clone() },
                        format!("ops[{i}].id"),
                    )
                })?;
            comments::reject_ink(comment, &op.id, i)?;
            let el = super::para::find_paragraph_mut(comment, para).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::ParaNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )
            })?;
            super::revision::refuse_set_text_if_revision(el, i)?;
            write_para_runs(el, op, i)?;
            dirty.insert(part_name, ());
        }
    }
    Ok(())
}
pub(crate) fn write_para_runs(el: &mut Element, op: &SetParaRunsOp, i: usize) -> Result<(), Error> {
    let visible = super::para::visible_text(el);
    let chars: Vec<char> = visible.chars().collect();
    let slices = match op.runs.first().map(|run| &run.source) {
        Some(ParaRunSource::Match { .. }) => match_slices(el, &visible, &chars, op, i)?,
        Some(ParaRunSource::Text(_)) => {
            let joined: String = op
                .runs
                .iter()
                .map(|run| match &run.source {
                    ParaRunSource::Text(text) => text.as_str(),
                    _ => "",
                })
                .collect();
            if joined != visible {
                return Err(Error::with_path(
                    ErrorKind::ParaRunMismatch { text: joined },
                    format!("ops[{i}].runs"),
                ));
            }
            let mut cursor = 0usize;
            let mut slices = Vec::new();
            for run in &op.runs {
                let ParaRunSource::Text(text) = &run.source else {
                    continue;
                };
                let end = cursor + text.chars().count();
                slices.push(slice_at(el, &chars, cursor, end, run.format.as_ref()));
                cursor = end;
            }
            slices
        }
        _ => {
            let mut cursor = 0usize;
            let mut slices = Vec::new();
            for (j, run) in op.runs.iter().enumerate() {
                let ParaRunSource::Range { start, end } = run.source else {
                    continue;
                };
                if start != cursor || end > chars.len() {
                    return Err(Error::with_path(
                        ErrorKind::OutOfRange {
                            field: "start".into(),
                            min: cursor as i64,
                            max: chars.len() as i64,
                        },
                        format!("ops[{i}].runs[{j}].start"),
                    ));
                }
                slices.push(slice_at(el, &chars, start, end, run.format.as_ref()));
                cursor = end;
            }
            if cursor != chars.len() {
                return Err(Error::with_path(
                    ErrorKind::OutOfRange {
                        field: "end".into(),
                        min: 0,
                        max: chars.len() as i64,
                    },
                    format!("ops[{i}].runs"),
                ));
            }
            slices
        }
    };
    let pieces: Vec<super::para::WrittenRun> = slices
        .iter()
        .map(|(text, rpr, _)| super::para::WrittenRun {
            text: text.clone(),
            rpr: rpr.clone(),
        })
        .collect();
    super::para::replace_visible_runs(el, &pieces);
    let mut n = 0usize;
    for child in el.children.iter_mut() {
        let crate::xml::Node::Element(run) = child else {
            continue;
        };
        if run.local_name() == "r" && run.element_children().any(|c| c.local_name() == "t") {
            if let Some((_, _, Some(fmt))) = slices.get(n) {
                format::apply_run(run, super::format::run_patch(fmt));
            }
            n += 1;
        }
    }
    Ok(())
}
pub(crate) fn slice_at<'a>(
    el: &Element,
    chars: &[char],
    start: usize,
    end: usize,
    format: Option<&'a super::request::para_remain::RunFormatOwned>,
) -> RunSlice<'a> {
    let text: String = chars[start..end].iter().collect();
    let rpr = if start < chars.len() {
        super::para::rpr_at_char(el, start)
    } else {
        super::para::first_text_rpr(el)
    };
    (text, rpr, format)
}
pub(crate) fn match_slices<'a>(
    el: &Element,
    visible: &str,
    chars: &[char],
    op: &'a SetParaRunsOp,
    i: usize,
) -> Result<Vec<RunSlice<'a>>, Error> {
    let mut cursor_chars = 0usize;
    let mut cursor_bytes = 0usize;
    let mut slices = Vec::new();
    for (j, run) in op.runs.iter().enumerate() {
        let ParaRunSource::Match {
            text: needle,
            after,
            before,
            nth,
        } = &run.source
        else {
            continue;
        };
        let Some((abs_bytes, end_bytes)) = crate::text_span::locate_span(
            visible,
            cursor_bytes,
            needle.as_deref(),
            after.as_deref(),
            before.as_deref(),
            *nth,
        ) else {
            let (field, text) = match needle {
                Some(text) => ("match", text.clone()),
                None => ("before", before.clone().unwrap_or_default()),
            };
            return Err(Error::with_path(
                ErrorKind::ParaRunMismatch { text },
                format!("ops[{i}].runs[{j}].{field}"),
            ));
        };
        let gap = visible[cursor_bytes..abs_bytes].chars().count();
        let start = cursor_chars + gap;
        let end = start + visible[abs_bytes..end_bytes].chars().count();
        if gap > 0 {
            slices.push(slice_at(el, chars, cursor_chars, start, None));
        }
        slices.push(slice_at(el, chars, start, end, run.format.as_ref()));
        cursor_chars = end;
        cursor_bytes = visible
            .chars()
            .take(cursor_chars)
            .map(|ch| ch.len_utf8())
            .sum();
    }
    if cursor_chars < chars.len() {
        slices.push(slice_at(el, chars, cursor_chars, chars.len(), None));
    }
    Ok(slices)
}
pub(crate) fn apply_insert_para(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &InsertParaOp,
    i: usize,
) -> Result<(), Error> {
    match &op.place {
        InsertParaPlace::TextBox { box_target, after } => {
            let part_name =
                super::apply::part_for_txbx(pkg, catalog, doc_part, box_target, &op.id, i)?;
            let doc = super::apply::load_part(pkg, docs, &part_name)?;
            let new_id = super::para::next_para_hex(&doc.root);
            let host = super::para::find_paragraph_mut(&mut doc.root, box_target.host.body())
                .ok_or_else(|| super::textbox::txbx_missing(&op.id, i))?;
            let content = textbox::find_txbx_content_by_key_mut(host, &box_target.key)
                .ok_or_else(|| super::textbox::txbx_missing(&op.id, i))?;
            let new_p = super::para::new_paragraph(content, &new_id, &op.text);
            if let Some(ParaTarget::InTextBox { para, .. }) = after {
                insert_p_after(content, para, new_p).map_err(|_| {
                    Error::with_path(
                        ErrorKind::ParaNotFound { id: op.id.clone() },
                        format!("ops[{i}].id"),
                    )
                })?;
            } else {
                content.children.push(Node::Element(new_p));
            }
            dirty.insert(part_name, ());
            Ok(())
        }
        InsertParaPlace::Note {
            kind,
            note_id,
            after,
        } => {
            let part_name = notes::part_name(pkg, doc_part, *kind)?.ok_or_else(|| {
                Error::with_path(
                    ErrorKind::NoteNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )
            })?;
            let doc = super::apply::load_part(pkg, docs, &part_name)?;
            let new_id = super::para::next_para_hex(&doc.root);
            let note =
                notes::find_user_note_mut(&mut doc.root, *kind, note_id).ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::NoteNotFound { id: op.id.clone() },
                        format!("ops[{i}].id"),
                    )
                })?;
            let new_p = super::para::new_paragraph(note, &new_id, &op.text);
            if let Some(para) = after {
                insert_p_after(note, para, new_p).map_err(|_| {
                    Error::with_path(
                        ErrorKind::ParaNotFound { id: op.id.clone() },
                        format!("ops[{i}].id"),
                    )
                })?;
            } else {
                note.children.push(Node::Element(new_p));
            }
            dirty.insert(part_name, ());
            Ok(())
        }
        InsertParaPlace::Comment { comment_id, after } => {
            let part_name = comments::part_name(pkg, doc_part)?
                .ok_or_else(|| comments::comment_missing(&op.id, i))?;
            let old_last = {
                let doc = super::apply::load_part(pkg, docs, &part_name)?;
                let comment = comments::find_comment(&doc.root, comment_id)
                    .ok_or_else(|| comments::comment_missing(&op.id, i))?;
                comments::reject_ink(comment, &op.id, i)?;
                comments::last_para_id(comment)
            };
            let doc = super::apply::load_part(pkg, docs, &part_name)?;
            let new_id = super::para::next_para_hex(&doc.root);
            let comment = comments::find_comment_mut(&mut doc.root, comment_id)
                .ok_or_else(|| comments::comment_missing(&op.id, i))?;
            let new_p = super::para::new_paragraph(comment, &new_id, &op.text);
            if let Some(para) = after {
                insert_p_after(comment, para, new_p).map_err(|_| {
                    Error::with_path(
                        ErrorKind::ParaNotFound { id: op.id.clone() },
                        format!("ops[{i}].id"),
                    )
                })?;
            } else {
                comment.children.push(Node::Element(new_p));
            }
            dirty.insert(part_name.clone(), ());
            super::comments::retarget_comment_last(
                pkg,
                docs,
                dirty,
                doc_part,
                comment_id,
                old_last.as_deref(),
            )?;
            Ok(())
        }
        InsertParaPlace::Body { after } => apply_insert_story_para(
            pkg,
            docs,
            dirty,
            doc_part,
            after.as_ref(),
            &op.id,
            &op.text,
            i,
        ),
        InsertParaPlace::HeaderFooter {
            kind,
            hf_type,
            n,
            after,
        } => {
            let part_name = super::apply::part_for_hf(catalog, *kind, *hf_type, *n, &op.id, i)?;
            apply_insert_story_para(
                pkg,
                docs,
                dirty,
                &part_name,
                after.as_ref(),
                &op.id,
                &op.text,
                i,
            )
        }
    }
}
pub(crate) fn apply_insert_story_para(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    part_name: &str,
    after: Option<&super::para::BodyPara>,
    id: &str,
    text: &str,
    i: usize,
) -> Result<(), Error> {
    let doc = super::apply::load_part(pkg, docs, part_name)?;
    let new_id = super::para::next_para_hex(&doc.root);
    if let Some(after) = after {
        let path = super::para::find_paragraph_path(&doc.root, after).ok_or_else(|| {
            Error::with_path(
                ErrorKind::ParaNotFound { id: id.to_string() },
                format!("ops[{i}].id"),
            )
        })?;
        let new_p = {
            let host = super::para::element_at(&doc.root, &path).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::ParaNotFound { id: id.to_string() },
                    format!("ops[{i}].id"),
                )
            })?;
            super::para::new_paragraph(host, &new_id, text)
        };
        let (parent, idx) =
            super::para::parent_and_index_mut(&mut doc.root, &path).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::ParaNotFound { id: id.to_string() },
                    format!("ops[{i}].id"),
                )
            })?;
        parent.children.insert(idx + 1, Node::Element(new_p));
    } else {
        let body = super::para::story_body_mut(&mut doc.root).ok_or_else(|| {
            Error::with_path(
                ErrorKind::XmlMissing {
                    parent: part_name.to_string(),
                    child: "body".into(),
                },
                format!("ops[{i}].id"),
            )
        })?;
        let new_p = super::para::new_paragraph(body, &new_id, text);
        super::para::append_before_body_sect_pr(body, new_p);
    }
    dirty.insert(part_name.to_string(), ());
    Ok(())
}
pub(crate) fn insert_p_after(
    parent: &mut Element,
    para: &super::para::BodyPara,
    new_p: Element,
) -> Result<(), ()> {
    let mut idx = 0usize;
    let mut insert_at = None;
    for (j, n) in parent.children.iter().enumerate() {
        if let Node::Element(e) = n {
            if e.local_name() == "p" {
                idx += 1;
                if same_para(e, para, idx) {
                    insert_at = Some(j + 1);
                    break;
                }
            }
        }
    }
    let at = insert_at.ok_or(())?;
    parent.children.insert(at, Node::Element(new_p));
    Ok(())
}
pub(crate) fn apply_delete_para(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &DeleteParaOp,
    i: usize,
) -> Result<(), Error> {
    match &op.inner {
        ParaTarget::InTextBox { host, key, para } => {
            let part_name = super::apply::part_for_host(pkg, catalog, doc_part, host, &op.id, i)?;
            let doc = super::apply::load_part(pkg, docs, &part_name)?;
            let host_p =
                super::para::find_paragraph_mut(&mut doc.root, host.body()).ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::ParaNotFound { id: op.id.clone() },
                        format!("ops[{i}].id"),
                    )
                })?;
            let content = textbox::find_txbx_content_by_key_mut(host_p, key).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::ParaNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )
            })?;
            let count = super::para::collect_paragraphs_in_txbx(content).len();
            if count <= 1 {
                return Err(Error::with_path(
                    ErrorKind::LastTextBoxPara,
                    format!("ops[{i}].id"),
                ));
            }
            remove_p_at(content, para).map_err(|_| {
                Error::with_path(
                    ErrorKind::ParaNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )
            })?;
            dirty.insert(part_name, ());
            Ok(())
        }
        ParaTarget::InNote {
            kind,
            note_id,
            para,
        } => {
            let part_name = notes::part_name(pkg, doc_part, *kind)?.ok_or_else(|| {
                Error::with_path(
                    ErrorKind::NoteNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )
            })?;
            let doc = super::apply::load_part(pkg, docs, &part_name)?;
            let note =
                notes::find_user_note_mut(&mut doc.root, *kind, note_id).ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::NoteNotFound { id: op.id.clone() },
                        format!("ops[{i}].id"),
                    )
                })?;
            let count = super::para::collect_paragraphs(note).len();
            if count <= 1 {
                return Err(Error::with_path(
                    ErrorKind::LastNotePara,
                    format!("ops[{i}].id"),
                ));
            }
            remove_p_at(note, para).map_err(|_| {
                Error::with_path(
                    ErrorKind::ParaNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )
            })?;
            dirty.insert(part_name, ());
            Ok(())
        }
        ParaTarget::InComment { comment_id, para } => {
            let part_name = comments::part_name(pkg, doc_part)?
                .ok_or_else(|| comments::comment_missing(&op.id, i))?;
            let old_last = {
                let doc = super::apply::load_part(pkg, docs, &part_name)?;
                let comment = comments::find_comment(&doc.root, comment_id)
                    .ok_or_else(|| comments::comment_missing(&op.id, i))?;
                comments::reject_ink(comment, &op.id, i)?;
                comments::last_para_id(comment)
            };
            let doc = super::apply::load_part(pkg, docs, &part_name)?;
            let comment = comments::find_comment_mut(&mut doc.root, comment_id)
                .ok_or_else(|| comments::comment_missing(&op.id, i))?;
            let count = super::para::collect_paragraphs(comment).len();
            if count <= 1 {
                return Err(Error::with_path(
                    ErrorKind::LastCommentPara,
                    format!("ops[{i}].id"),
                ));
            }
            remove_p_at(comment, para).map_err(|_| {
                Error::with_path(
                    ErrorKind::ParaNotFound { id: op.id.clone() },
                    format!("ops[{i}].id"),
                )
            })?;
            dirty.insert(part_name.clone(), ());
            super::comments::retarget_comment_last(
                pkg,
                docs,
                dirty,
                doc_part,
                comment_id,
                old_last.as_deref(),
            )?;
            Ok(())
        }
        ParaTarget::Body(para) => {
            apply_delete_story_para(pkg, docs, dirty, doc_part, para, &op.id, i)
        }
        ParaTarget::HeaderFooter {
            kind,
            hf_type,
            n,
            para,
        } => {
            let part_name = super::apply::part_for_hf(catalog, *kind, *hf_type, *n, &op.id, i)?;
            apply_delete_story_para(pkg, docs, dirty, &part_name, para, &op.id, i)
        }
    }
}
pub(crate) fn apply_delete_story_para(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    part_name: &str,
    para: &super::para::BodyPara,
    id: &str,
    i: usize,
) -> Result<(), Error> {
    let doc = super::apply::load_part(pkg, docs, part_name)?;
    let path = super::para::find_paragraph_path(&doc.root, para).ok_or_else(|| {
        Error::with_path(
            ErrorKind::ParaNotFound { id: id.to_string() },
            format!("ops[{i}].id"),
        )
    })?;
    let parent_count = if path.len() <= 1 {
        super::para::count_direct_p(&doc.root)
    } else {
        let parent =
            super::para::element_at(&doc.root, &path[..path.len() - 1]).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::ParaNotFound { id: id.to_string() },
                    format!("ops[{i}].id"),
                )
            })?;
        super::para::count_direct_p(parent)
    };
    if parent_count <= 1 {
        return Err(Error::with_path(
            ErrorKind::LastStoryPara,
            format!("ops[{i}].id"),
        ));
    }
    let (has_rev, unmatched, has_sect, is_last) = {
        let p_el = super::para::element_at(&doc.root, &path).ok_or_else(|| {
            Error::with_path(
                ErrorKind::ParaNotFound { id: id.to_string() },
                format!("ops[{i}].id"),
            )
        })?;
        (
            super::para::para_has_revision(p_el),
            super::para::para_has_unmatched_range(p_el),
            super::para::para_has_sect_pr(p_el),
            super::para::is_last_collected(&doc.root, para),
        )
    };
    if has_rev {
        return Err(Error::with_path(
            ErrorKind::ParaHasRevision,
            format!("ops[{i}].id"),
        ));
    }
    if unmatched {
        return Err(Error::with_path(
            ErrorKind::CrossParaMarker,
            format!("ops[{i}].id"),
        ));
    }
    if has_sect && !is_last {
        return Err(Error::with_path(
            ErrorKind::SectionBreakPara,
            format!("ops[{i}].id"),
        ));
    }
    if has_sect && is_last {
        let sect = {
            let p_mut = super::para::element_at_mut(&mut doc.root, &path).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::ParaNotFound { id: id.to_string() },
                    format!("ops[{i}].id"),
                )
            })?;
            super::para::take_ppr_sect_pr(p_mut)
        };
        if let Some(sect) = sect {
            let paths = super::para::collect_paragraph_paths(&doc.root);
            if let Some(dest_path) = paths.iter().rev().find(|pp| *pp != &path).cloned() {
                let dest =
                    super::para::element_at_mut(&mut doc.root, &dest_path).ok_or_else(|| {
                        Error::with_path(
                            ErrorKind::ParaNotFound { id: id.to_string() },
                            format!("ops[{i}].id"),
                        )
                    })?;
                super::para::put_ppr_sect_pr(dest, sect);
            }
        }
    }
    let (parent, idx) =
        super::para::parent_and_index_mut(&mut doc.root, &path).ok_or_else(|| {
            Error::with_path(
                ErrorKind::ParaNotFound { id: id.to_string() },
                format!("ops[{i}].id"),
            )
        })?;
    parent.children.remove(idx);
    dirty.insert(part_name.to_string(), ());
    Ok(())
}
pub(crate) fn remove_p_at(parent: &mut Element, para: &super::para::BodyPara) -> Result<(), ()> {
    let mut idx = 0usize;
    let mut remove_at = None;
    for (j, n) in parent.children.iter().enumerate() {
        if let Node::Element(e) = n {
            if e.local_name() == "p" {
                idx += 1;
                if same_para(e, para, idx) {
                    remove_at = Some(j);
                    break;
                }
            }
        }
    }
    let at = remove_at.ok_or(())?;
    parent.children.remove(at);
    Ok(())
}
