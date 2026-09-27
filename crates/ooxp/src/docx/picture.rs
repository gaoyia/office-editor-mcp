//! 页眉、页脚和正文里的 DrawingML 图片：收集、定位、换媒体、插删、改尺寸与绕排。

use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::error::{io_path, Error, ErrorKind, IdReason};
use crate::opc::{
    rels_part_name, resolve_rel_target, split_dir_file, ContentTypes, Package, Relationship,
    Relationships,
};
use crate::xml::{Element, Node};

use super::hf::{HfKind, HfType};
use super::notes;
use super::para::{self, BodyPara, HostPara, ParaTarget, TxbxKey};
use super::{A_NS, PIC_NS, REL_IMAGE, REL_NS, WP_NS};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PicKind {
    Inline,
    Anchor,
}

impl PicKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Inline => "inline",
            Self::Anchor => "anchor",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "inline" => Some(Self::Inline),
            "anchor" => Some(Self::Anchor),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WrapKind {
    Square,
    Tight,
    Through,
    TopAndBottom,
    None,
}

impl WrapKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Square => "square",
            Self::Tight => "tight",
            Self::Through => "through",
            Self::TopAndBottom => "topAndBottom",
            Self::None => "none",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "square" => Some(Self::Square),
            "tight" => Some(Self::Tight),
            "through" => Some(Self::Through),
            "topAndBottom" => Some(Self::TopAndBottom),
            "none" => Some(Self::None),
            _ => None,
        }
    }

    fn local_name(self) -> &'static str {
        match self {
            Self::Square => "wrapSquare",
            Self::Tight => "wrapTight",
            Self::Through => "wrapThrough",
            Self::TopAndBottom => "wrapTopAndBottom",
            Self::None => "wrapNone",
        }
    }

    pub fn from_local(name: &str) -> Option<Self> {
        match name {
            "wrapSquare" => Some(Self::Square),
            "wrapTight" => Some(Self::Tight),
            "wrapThrough" => Some(Self::Through),
            "wrapTopAndBottom" => Some(Self::TopAndBottom),
            "wrapNone" => Some(Self::None),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlignH {
    Left,
    Center,
    Right,
}

impl AlignH {
    fn as_str(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Center => "center",
            Self::Right => "right",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "left" => Some(Self::Left),
            "center" => Some(Self::Center),
            "right" => Some(Self::Right),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlignV {
    Top,
    Center,
    Bottom,
}

impl AlignV {
    fn as_str(self) -> &'static str {
        match self {
            Self::Top => "top",
            Self::Center => "center",
            Self::Bottom => "bottom",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "top" => Some(Self::Top),
            "center" => Some(Self::Center),
            "bottom" => Some(Self::Bottom),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum PicKey {
    DocPr(String),
    Index(usize),
}

#[derive(Debug, Clone)]
pub enum PicTarget {
    HeaderFooter {
        kind: HfKind,
        hf_type: HfType,
        n: u32,
        key: PicKey,
    },
    Body {
        para: BodyPara,
        key: PicKey,
    },
    InTextBox {
        host: HostPara,
        box_key: TxbxKey,
        para: BodyPara,
        key: PicKey,
    },
    InNote {
        kind: para::NoteKind,
        note_id: String,
        para: BodyPara,
        key: PicKey,
    },
    InComment {
        comment_id: String,
        para: BodyPara,
        key: PicKey,
    },
}

pub struct PicInfo {
    pub id: String,
    pub doc_pr_id: Option<String>,
    pub name: Option<String>,
    pub kind: PicKind,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
    pub wrap: Option<WrapKind>,
    pub stable: bool,
}

pub struct ImageFile {
    pub bytes: Vec<u8>,
    pub ext: String,
    pub content_type: String,
    pub cx: i64,
    pub cy: i64,
}

#[derive(Debug, Clone, Default)]
pub struct WrapPos {
    pub align_h: Option<AlignH>,
    pub align_v: Option<AlignV>,
    pub offset_x: Option<i64>,
    pub offset_y: Option<i64>,
}

/// 把源部件的 `.rels` 原样抄到新部件。媒体 Target 仍指向同一路径。
pub fn copy_part_rels(pkg: &mut Package, src_part: &str, dst_part: &str) -> Result<(), Error> {
    let src_rels = rels_part_name(src_part);
    if !pkg.contains(&src_rels) {
        return Ok(());
    }
    let bytes = pkg.get(&src_rels)?.to_vec();
    pkg.set(&rels_part_name(dst_part), bytes)
}

pub fn collect_pictures(root: &Element, container_id: &str) -> Result<Vec<PicInfo>, Error> {
    let mut out = Vec::new();
    let mut index = 0usize;
    walk_drawings(root, &mut |drawing| {
        if !is_picture(drawing) {
            return Ok(());
        }
        index += 1;
        out.push(pic_info(drawing, container_id, index)?);
        Ok(())
    })?;
    Ok(out)
}

pub fn pictures_json(pics: &[PicInfo]) -> Vec<Value> {
    pics.iter()
        .map(|p| {
            json!({
                "id": p.id,
                "docPrId": p.doc_pr_id,
                "name": p.name,
                "kind": p.kind.as_str(),
                "cx": p.cx,
                "cy": p.cy,
                "wrap": p.wrap.map(WrapKind::as_str),
                "stable": p.stable,
            })
        })
        .collect()
}

pub fn parse_pic_id(id: &str, op_index: usize) -> Result<PicTarget, Error> {
    parse_pic_id_at(id, &format!("ops[{op_index}].id"))
}

pub fn parse_pic_id_at(id: &str, path: &str) -> Result<PicTarget, Error> {
    let path = path.to_string();
    let Some((left, key_s)) = id.rsplit_once("/pic:") else {
        if id.starts_with("para:")
            || id.starts_with("footnote:")
            || id.starts_with("endnote:")
            || id.starts_with("comment:")
            || HfKind::parse(id.split_once(':').map(|(k, _)| k).unwrap_or("")).is_some()
        {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::PicContainer,
                },
                path,
            ));
        }
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::PicPrefix,
            },
            path,
        ));
    };
    let key = parse_pic_key(key_s, &path)?;
    if left.contains("/txbx:") {
        match para::parse_para_id_at(left, &path)? {
            ParaTarget::InTextBox {
                host,
                key: box_key,
                para,
            } => {
                return Ok(PicTarget::InTextBox {
                    host,
                    box_key,
                    para,
                    key,
                });
            }
            _ => {
                return Err(Error::with_path(
                    ErrorKind::InvalidId {
                        reason: IdReason::PicPrefix,
                    },
                    path,
                ));
            }
        }
    }
    if left.starts_with("para:") {
        match para::parse_para_id_at(left, &path)? {
            ParaTarget::Body(p) => return Ok(PicTarget::Body { para: p, key }),
            ParaTarget::HeaderFooter { .. }
            | ParaTarget::InTextBox { .. }
            | ParaTarget::InNote { .. }
            | ParaTarget::InComment { .. } => {
                return Err(Error::with_path(
                    ErrorKind::InvalidId {
                        reason: IdReason::PicPrefix,
                    },
                    path,
                ));
            }
        }
    }
    if let Some((kind_s, rest)) = left.split_once(':') {
        if let Some(kind) = HfKind::parse(kind_s) {
            let Some((type_s, n_s)) = rest.split_once(':') else {
                return Err(Error::with_path(
                    ErrorKind::InvalidId {
                        reason: IdReason::PicPrefix,
                    },
                    path,
                ));
            };
            let hf_type = HfType::parse(type_s).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::InvalidId {
                        reason: IdReason::PicPrefix,
                    },
                    path.clone(),
                )
            })?;
            let n: u32 = n_s.parse().map_err(|_| {
                Error::with_path(
                    ErrorKind::InvalidId {
                        reason: IdReason::PicPrefix,
                    },
                    path.clone(),
                )
            })?;
            if n < 1 {
                return Err(Error::with_path(
                    ErrorKind::InvalidId {
                        reason: IdReason::PicPrefix,
                    },
                    path,
                ));
            }
            return Ok(PicTarget::HeaderFooter {
                kind,
                hf_type,
                n,
                key,
            });
        }
    }
    if left.starts_with("footnote:") || left.starts_with("endnote:") {
        match para::parse_para_id_at(left, &path)? {
            ParaTarget::InNote {
                kind,
                note_id,
                para,
            } => {
                return Ok(PicTarget::InNote {
                    kind,
                    note_id,
                    para,
                    key,
                });
            }
            _ => {
                return Err(Error::with_path(
                    ErrorKind::InvalidId {
                        reason: IdReason::PicPrefix,
                    },
                    path,
                ));
            }
        }
    }
    if left.starts_with("comment:") {
        match para::parse_para_id_at(left, &path)? {
            ParaTarget::InComment { comment_id, para } => {
                return Ok(PicTarget::InComment {
                    comment_id,
                    para,
                    key,
                });
            }
            _ => {
                return Err(Error::with_path(
                    ErrorKind::InvalidId {
                        reason: IdReason::PicPrefix,
                    },
                    path,
                ));
            }
        }
    }
    Err(Error::with_path(
        ErrorKind::InvalidId {
            reason: IdReason::PicPrefix,
        },
        path,
    ))
}

pub fn parse_insert_para_id(id: &str, op_index: usize) -> Result<ParaTarget, Error> {
    let path = format!("ops[{op_index}].id");
    if id.contains("/pic:") {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::InsertNeedsPara,
            },
            path,
        ));
    }
    if let Some((_, after)) = id.split_once("/txbx:") {
        if !after.contains("/para:") {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::InsertNeedsPara,
                },
                path,
            ));
        }
    }
    if let Some(note) = para::parse_note_id_at(id, &path)? {
        match note {
            para::NoteIdTarget::Sep => {
                return Err(Error::with_path(
                    ErrorKind::InvalidId {
                        reason: IdReason::NoteSep,
                    },
                    path,
                ));
            }
            para::NoteIdTarget::Container { .. } => {
                return Err(Error::with_path(
                    ErrorKind::InvalidId {
                        reason: IdReason::InsertNeedsPara,
                    },
                    path,
                ));
            }
            para::NoteIdTarget::Para { .. } => {}
        }
    }
    if let Some(comment) = para::parse_comment_id_at(id, &path)? {
        match comment {
            para::CommentIdTarget::Container { .. } => {
                return Err(Error::with_path(
                    ErrorKind::InvalidId {
                        reason: IdReason::InsertNeedsPara,
                    },
                    path,
                ));
            }
            para::CommentIdTarget::Para { .. } => {}
        }
    }
    para::parse_para_id(id, op_index)
}

fn parse_pic_key(rest: &str, path: &str) -> Result<PicKey, Error> {
    if let Some(n) = rest.strip_prefix("index:") {
        let idx: usize = n.parse().map_err(|_| {
            Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::PicIndexNotInt,
                },
                path.to_string(),
            )
        })?;
        if idx < 1 {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::PicIndexFromOne,
                },
                path.to_string(),
            ));
        }
        return Ok(PicKey::Index(idx));
    }
    if rest.is_empty() {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::PicPrefix,
            },
            path.to_string(),
        ));
    }
    Ok(PicKey::DocPr(rest.to_string()))
}

pub fn find_drawing<'a>(root: &'a Element, key: &PicKey) -> Option<&'a Element> {
    let mut idx = 0usize;
    find_drawing_walk(root, &mut idx, key)
}

pub fn find_drawing_mut<'a>(root: &'a mut Element, key: &PicKey) -> Option<&'a mut Element> {
    let mut idx = 0usize;
    find_drawing_mut_walk(root, &mut idx, key)
}

pub fn find_target_drawing<'a>(root: &'a Element, target: &PicTarget) -> Option<&'a Element> {
    match target {
        PicTarget::HeaderFooter { key, .. } => find_drawing(root, key),
        PicTarget::Body { para, key } => {
            let p = para::find_paragraph(root, para)?;
            find_drawing(p, key)
        }
        PicTarget::InTextBox {
            host,
            box_key,
            para,
            key,
        } => {
            let host_p = para::find_paragraph(root, host.body())?;
            let content = super::textbox::find_txbx_content_by_key(host_p, box_key)?;
            let inner = para::find_paragraph_in_txbx(content, para)?;
            find_drawing(inner, key)
        }
        PicTarget::InNote {
            kind,
            note_id,
            para,
            key,
        } => {
            let note = notes::find_user_note(root, *kind, note_id)?;
            let inner = para::find_paragraph(note, para)?;
            find_drawing(inner, key)
        }
        PicTarget::InComment {
            comment_id,
            para,
            key,
        } => {
            let comment = super::comments::find_comment(root, comment_id)?;
            let inner = para::find_paragraph(comment, para)?;
            find_drawing(inner, key)
        }
    }
}

pub fn part_for_pic(
    pkg: &Package,
    catalog: &[super::hf::HfPart],
    doc_part: &str,
    target: &PicTarget,
    id: &str,
    field_path: &str,
) -> Result<String, Error> {
    match target {
        PicTarget::Body { .. }
        | PicTarget::InTextBox {
            host: HostPara::Body(_),
            ..
        } => Ok(doc_part.to_string()),
        PicTarget::InTextBox {
            host: HostPara::HeaderFooter {
                kind, hf_type, n, ..
            },
            ..
        } => {
            let container_id = format!("{}:{}:{n}", kind.as_str(), hf_type.as_str());
            let item = super::hf::find_by_id(catalog, &container_id).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::PictureNotFound { id: id.to_string() },
                    field_path.to_string(),
                )
            })?;
            Ok(item.part.clone())
        }
        PicTarget::InTextBox {
            host: HostPara::InNote { kind, .. },
            ..
        } => notes::part_name(pkg, doc_part, *kind)?.ok_or_else(|| {
            Error::with_path(
                ErrorKind::PictureNotFound { id: id.to_string() },
                field_path.to_string(),
            )
        }),
        PicTarget::InTextBox {
            host: HostPara::InComment { .. },
            ..
        } => super::comments::part_name(pkg, doc_part)?.ok_or_else(|| {
            Error::with_path(
                ErrorKind::PictureNotFound { id: id.to_string() },
                field_path.to_string(),
            )
        }),
        PicTarget::HeaderFooter {
            kind, hf_type, n, ..
        } => {
            let container_id = format!("{}:{}:{n}", kind.as_str(), hf_type.as_str());
            let item = super::hf::find_by_id(catalog, &container_id).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::PictureNotFound { id: id.to_string() },
                    field_path.to_string(),
                )
            })?;
            Ok(item.part.clone())
        }
        PicTarget::InNote { kind, .. } => {
            notes::part_name(pkg, doc_part, *kind)?.ok_or_else(|| {
                Error::with_path(
                    ErrorKind::PictureNotFound { id: id.to_string() },
                    field_path.to_string(),
                )
            })
        }
        PicTarget::InComment { .. } => {
            super::comments::part_name(pkg, doc_part)?.ok_or_else(|| {
                Error::with_path(
                    ErrorKind::PictureNotFound { id: id.to_string() },
                    field_path.to_string(),
                )
            })
        }
    }
}

pub fn remove_picture(root: &mut Element, key: &PicKey) -> bool {
    let mut idx = 0usize;
    remove_picture_walk(root, &mut idx, key)
}

pub fn read_image(path: &str) -> Result<ImageFile, Error> {
    let p = PathBuf::from(path);
    let bytes = std::fs::read(&p).map_err(|e| io_path(e, p.clone()))?;
    classify_image(bytes, &p)
}

pub fn next_media_part(pkg: &Package, ext: &str) -> String {
    let mut n = 1u32;
    loop {
        let name = format!("/word/media/image{n}.{ext}");
        if !pkg.contains(&name) {
            return name;
        }
        n += 1;
    }
}

/// 从源部件到目标部件的相对关系 Target。
pub fn rel_target_from(source_part: &str, target_opc: &str) -> String {
    let src_dir = split_dir_file(source_part).0;
    let tgt = target_opc.trim_start_matches('/');
    let src = src_dir.trim_start_matches('/');
    if src.is_empty() || src == "/" {
        return tgt.to_string();
    }
    if let Some(rest) = tgt.strip_prefix(&format!("{src}/")) {
        return rest.to_string();
    }
    let up = src.split('/').filter(|s| !s.is_empty()).count();
    format!("{}{tgt}", "../".repeat(up))
}

pub fn next_docpr_id(root: &Element) -> u32 {
    let mut max = 0u32;
    walk_all(root, &mut |e| {
        if e.local_name() == "docPr" {
            if let Some(id) = e.attr_local("id") {
                if let Ok(n) = id.parse::<u32>() {
                    max = max.max(n);
                }
            }
        }
    });
    max + 1
}

pub fn ensure_drawing_ns(root: &mut Element) {
    ensure_xmlns(root, "r", REL_NS);
    ensure_xmlns(root, "wp", WP_NS);
    ensure_xmlns(root, "a", A_NS);
    ensure_xmlns(root, "pic", PIC_NS);
}

pub fn build_inline_drawing(doc_pr: u32, name: &str, rid: &str, cx: i64, cy: i64) -> Element {
    let mut drawing = Element::new("w:drawing");
    let mut inline = Element::new("wp:inline")
        .with_attr("distT", "0")
        .with_attr("distB", "0")
        .with_attr("distL", "0")
        .with_attr("distR", "0");
    inline.push_element(extent_el(cx, cy));
    inline.push_element(
        Element::new("wp:effectExtent")
            .with_attr("l", "0")
            .with_attr("t", "0")
            .with_attr("r", "0")
            .with_attr("b", "0"),
    );
    inline.push_element(doc_pr_el(doc_pr, name));
    inline.push_element(cnv_frame_pr());
    inline.push_element(graphic_el(name, rid, cx, cy));
    drawing.push_element(inline);
    drawing
}

pub fn set_extent(drawing: &mut Element, cx: Option<i64>, cy: Option<i64>) -> bool {
    let Some(frame) = frame_mut(drawing) else {
        return false;
    };
    if let Some(ext) = frame.child_by_local_mut("extent") {
        if let Some(v) = cx {
            set_attr_local(ext, "cx", &v.to_string());
        }
        if let Some(v) = cy {
            set_attr_local(ext, "cy", &v.to_string());
        }
    }
    if let Some(xfrm) = find_local_mut(frame, "xfrm") {
        if let Some(ext) = xfrm.child_by_local_mut("ext") {
            if let Some(v) = cx {
                set_attr_local(ext, "cx", &v.to_string());
            }
            if let Some(v) = cy {
                set_attr_local(ext, "cy", &v.to_string());
            }
        }
    }
    true
}

pub fn apply_wrap(
    drawing: &mut Element,
    kind: PicKind,
    wrap: Option<WrapKind>,
    pos: &WrapPos,
) -> Result<(), Error> {
    match kind {
        PicKind::Inline => {
            if wrap.is_some() {
                return Err(Error::new(ErrorKind::InvalidPicWrap));
            }
            convert_to_inline(drawing);
        }
        PicKind::Anchor => {
            let wrap = wrap.ok_or_else(|| Error::new(ErrorKind::InvalidPicWrap))?;
            convert_to_anchor(drawing, wrap, pos);
        }
    }
    Ok(())
}

pub fn blip_embed(drawing: &Element) -> Option<String> {
    let mut found = None;
    walk_all(drawing, &mut |e| {
        if e.local_name() == "blip" && found.is_none() {
            found = e.attr_local("embed").map(str::to_string);
        }
    });
    found
}

pub fn blip_link(drawing: &Element) -> Option<String> {
    let mut found = None;
    walk_all(drawing, &mut |e| {
        if e.local_name() == "blip" && found.is_none() {
            found = e.attr_local("link").map(str::to_string);
        }
    });
    found
}

/// 优先 embed，没有才用 link。
pub fn blip_rid(drawing: &Element) -> Option<String> {
    blip_embed(drawing).or_else(|| blip_link(drawing))
}

pub fn resolve_embed(pkg: &Package, part: &str, rid: &str) -> Result<String, Error> {
    let rels = Relationships::from_package(pkg, part)?;
    let rel = rels.find_by_id(rid).ok_or_else(|| {
        Error::new(ErrorKind::RelNotFound {
            source: crate::error::RelSource::Part(part.to_string()),
            rid: rid.to_string(),
        })
    })?;
    resolve_rel_target(part, &rel.target)
}

pub fn update_rel_target(
    pkg: &mut Package,
    part: &str,
    rid: &str,
    new_target_opc: &str,
) -> Result<(), Error> {
    let mut rels = Relationships::from_package(pkg, part)?;
    if let Some(rel) = rels.rels.iter_mut().find(|r| r.id == rid) {
        rel.target = rel_target_from(part, new_target_opc);
    }
    rels.write_to(pkg, part)
}

/// 读本地图、写入 `/word/media/…`、挂关系到 `part`、补内容类型。
/// `cx`/`cy` 都给则用调用方的尺寸；都省则按像素 96 dpi。
pub fn add_media(
    pkg: &mut Package,
    part: &str,
    file: &str,
    cx: Option<i64>,
    cy: Option<i64>,
) -> Result<(String, i64, i64), Error> {
    let image = read_image(file)?;
    let (cx, cy) = match (cx, cy) {
        (Some(x), Some(y)) => (x, y),
        _ => (image.cx, image.cy),
    };
    let media = next_media_part(pkg, &image.ext);
    pkg.set(&media, image.bytes)?;
    let rid = add_image_rel(pkg, part, &media)?;
    let mut cts = ContentTypes::from_package(pkg)?;
    cts.ensure_default(&image.ext, &image.content_type);
    cts.write_to(pkg)?;
    Ok((rid, cx, cy))
}

/// 在段末写入一张图。行内只接受 `wrap` 为空；浮动必须带绕排。
pub fn insert_prepared_drawing(
    p: &mut Element,
    doc_pr: u32,
    rid: &str,
    cx: i64,
    cy: i64,
    kind: PicKind,
    wrap: Option<WrapKind>,
    pos: &WrapPos,
) -> Result<(), Error> {
    let name = format!("图片 {doc_pr}");
    let mut drawing = build_inline_drawing(doc_pr, &name, rid, cx, cy);
    apply_wrap(&mut drawing, kind, wrap, pos)?;
    insert_drawing_into_para(p, drawing);
    Ok(())
}

pub fn add_image_rel(pkg: &mut Package, part: &str, media_opc: &str) -> Result<String, Error> {
    let mut rels = Relationships::from_package(pkg, part)?;
    let rid = rels.next_rid();
    rels.push(Relationship {
        id: rid.clone(),
        rel_type: REL_IMAGE.into(),
        target: rel_target_from(part, media_opc),
        target_mode: None,
        extra: Vec::new(),
    });
    rels.write_to(pkg, part)?;
    Ok(rid)
}

pub fn media_still_used(pkg: &Package, media_opc: &str) -> Result<bool, Error> {
    for name in pkg.part_names() {
        if !name.ends_with(".rels") {
            continue;
        }
        let Some(source) = source_from_rels(&name) else {
            continue;
        };
        let rels = Relationships::parse(pkg.get(&name)?)?;
        for r in &rels.rels {
            if r.is_external() {
                continue;
            }
            if resolve_rel_target(&source, &r.target).ok().as_deref() == Some(media_opc) {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

pub fn remove_rel_if_unused(
    pkg: &mut Package,
    part: &str,
    rid: &str,
    root: &Element,
) -> Result<(), Error> {
    if embed_still_in(root, rid) {
        return Ok(());
    }
    let mut rels = Relationships::from_package(pkg, part)?;
    rels.remove_by_id(rid);
    if rels.rels.is_empty() {
        pkg.remove(&rels_part_name(part));
    } else {
        rels.write_to(pkg, part)?;
    }
    Ok(())
}

pub fn parse_align_h(s: &str) -> Option<AlignH> {
    AlignH::parse(s)
}

pub fn parse_align_v(s: &str) -> Option<AlignV> {
    AlignV::parse(s)
}

pub fn parse_wrap(s: &str) -> Option<WrapKind> {
    WrapKind::parse(s)
}

pub fn parse_pic_kind(s: &str) -> Option<PicKind> {
    PicKind::parse(s)
}

pub fn insert_drawing_into_para(p: &mut Element, drawing: Element) {
    let mut run = Element::new(p.prefixed("r"));
    run.push_element(drawing);
    p.children.push(Node::Element(run));
}

fn pic_info(drawing: &Element, container_id: &str, index: usize) -> Result<PicInfo, Error> {
    let frame = frame(drawing);
    let kind = match frame.map(Element::local_name) {
        Some("anchor") => PicKind::Anchor,
        _ => PicKind::Inline,
    };
    let doc_pr = frame.and_then(|f| f.child_by_local("docPr"));
    let raw_id = doc_pr
        .and_then(|d| d.attr_local("id"))
        .filter(|s| !s.is_empty());
    let name = doc_pr
        .and_then(|d| d.attr_local("name"))
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    let (id, stable, doc_pr_id) = match raw_id {
        Some(v) => (format!("{container_id}/pic:{v}"), true, Some(v.to_string())),
        None => (format!("{container_id}/pic:index:{index}"), false, None),
    };
    let extent = frame.and_then(|f| f.child_by_local("extent"));
    let cx = parse_emu_attr(extent, "cx")?;
    let cy = parse_emu_attr(extent, "cy")?;
    let wrap = if kind == PicKind::Anchor {
        frame.and_then(|f| {
            f.element_children()
                .find_map(|c| WrapKind::from_local(c.local_name()))
        })
    } else {
        None
    };
    Ok(PicInfo {
        id,
        doc_pr_id,
        name,
        kind,
        cx,
        cy,
        wrap,
        stable,
    })
}

fn parse_emu_attr(el: Option<&Element>, local: &str) -> Result<Option<i64>, Error> {
    let Some(el) = el else {
        return Ok(None);
    };
    let Some(raw) = el.attr_local(local) else {
        return Ok(None);
    };
    raw.parse::<i64>().map(Some).map_err(|_| {
        Error::xml_detail(
            crate::error::XmlIssue::AttrRead,
            format!("wp:extent @{local}={raw}"),
        )
    })
}

fn is_picture(drawing: &Element) -> bool {
    let mut has_pic = false;
    let mut has_blip = false;
    // 框内图不算外层 drawing 的图：遇 txbxContent 停住，与 collect 边界一致。
    walk_outside_txbx(drawing, &mut |e| match e.local_name() {
        "pic" => has_pic = true,
        "blip" => has_blip = true,
        _ => {}
    });
    has_pic && has_blip
}

fn walk_outside_txbx(el: &Element, f: &mut impl FnMut(&Element)) {
    if el.local_name() == "txbxContent" {
        return;
    }
    f(el);
    for n in &el.children {
        if let Node::Element(e) = n {
            walk_outside_txbx(e, f);
        }
    }
}

fn frame(drawing: &Element) -> Option<&Element> {
    drawing
        .element_children()
        .find(|e| e.local_name() == "inline" || e.local_name() == "anchor")
}

fn frame_mut(drawing: &mut Element) -> Option<&mut Element> {
    drawing
        .element_children_mut()
        .find(|e| e.local_name() == "inline" || e.local_name() == "anchor")
}

fn matches_key(drawing: &Element, key: &PicKey, index: usize) -> bool {
    match key {
        PicKey::Index(n) => index == *n,
        PicKey::DocPr(want) => {
            let Some(got) = frame(drawing)
                .and_then(|f| f.child_by_local("docPr"))
                .and_then(|d| d.attr_local("id"))
            else {
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

pub(crate) fn walk_drawings(
    el: &Element,
    f: &mut impl FnMut(&Element) -> Result<(), Error>,
) -> Result<(), Error> {
    if el.local_name() == "txbxContent" {
        return Ok(());
    }
    if el.local_name() == "AlternateContent" {
        if let Some(i) = first_alt_child_index(el) {
            if let Node::Element(child) = &el.children[i] {
                return walk_drawings(child, f);
            }
        }
        return Ok(());
    }
    if el.local_name() == "drawing" {
        return f(el);
    }
    for n in &el.children {
        if let Node::Element(e) = n {
            walk_drawings(e, f)?;
        }
    }
    Ok(())
}

fn walk_all(el: &Element, f: &mut impl FnMut(&Element)) {
    f(el);
    for n in &el.children {
        if let Node::Element(e) = n {
            walk_all(e, f);
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

fn find_drawing_walk<'a>(el: &'a Element, idx: &mut usize, key: &PicKey) -> Option<&'a Element> {
    if el.local_name() == "txbxContent" {
        return None;
    }
    if el.local_name() == "AlternateContent" {
        if let Some(i) = first_alt_child_index(el) {
            if let Node::Element(e) = &el.children[i] {
                return find_drawing_walk(e, idx, key);
            }
        }
        return None;
    }
    if el.local_name() == "drawing" && is_picture(el) {
        *idx += 1;
        if matches_key(el, key, *idx) {
            return Some(el);
        }
        return None;
    }
    for n in &el.children {
        if let Node::Element(e) = n {
            if let Some(found) = find_drawing_walk(e, idx, key) {
                return Some(found);
            }
        }
    }
    None
}

fn find_drawing_mut_walk<'a>(
    el: &'a mut Element,
    idx: &mut usize,
    key: &PicKey,
) -> Option<&'a mut Element> {
    if el.local_name() == "txbxContent" {
        return None;
    }
    if el.local_name() == "AlternateContent" {
        if let Some(i) = first_alt_child_index(el) {
            if let Node::Element(e) = &mut el.children[i] {
                return find_drawing_mut_walk(e, idx, key);
            }
        }
        return None;
    }
    if el.local_name() == "drawing" && is_picture(el) {
        *idx += 1;
        if matches_key(el, key, *idx) {
            return Some(el);
        }
        return None;
    }
    let children = el.children.as_mut_slice();
    for child in children {
        if let Node::Element(e) = child {
            if let Some(found) = find_drawing_mut_walk(e, idx, key) {
                return Some(found);
            }
        }
    }
    None
}

fn remove_picture_walk(el: &mut Element, idx: &mut usize, key: &PicKey) -> bool {
    if el.local_name() == "txbxContent" {
        return false;
    }
    let mut i = 0;
    while i < el.children.len() {
        let Node::Element(_) = &el.children[i] else {
            i += 1;
            continue;
        };
        let local = match &el.children[i] {
            Node::Element(c) => c.local_name().to_string(),
            Node::Text(_) => {
                i += 1;
                continue;
            }
        };
        if local == "AlternateContent" {
            if ac_has_match(&el.children[i], idx, key) {
                el.children.remove(i);
                return true;
            }
            i += 1;
            continue;
        }
        if local == "drawing" {
            let is_pic = matches!(&el.children[i], Node::Element(c) if is_picture(c));
            if is_pic {
                *idx += 1;
                let hit = match &el.children[i] {
                    Node::Element(c) => matches_key(c, key, *idx),
                    Node::Text(_) => false,
                };
                if hit {
                    el.children.remove(i);
                    return true;
                }
            }
            i += 1;
            continue;
        }
        if local == "txbxContent" {
            i += 1;
            continue;
        }
        if let Node::Element(child) = &mut el.children[i] {
            if remove_picture_walk(child, idx, key) {
                if is_empty_run(child) {
                    el.children.remove(i);
                }
                return true;
            }
        }
        i += 1;
    }
    false
}

fn ac_has_match(node: &Node, idx: &mut usize, key: &PicKey) -> bool {
    let Node::Element(ac) = node else {
        return false;
    };
    let Some(i) = first_alt_child_index(ac) else {
        return false;
    };
    let Node::Element(branch) = &ac.children[i] else {
        return false;
    };
    let mut hit = false;
    let _ = walk_drawings(branch, &mut |d| {
        if is_picture(d) {
            *idx += 1;
            if matches_key(d, key, *idx) {
                hit = true;
            }
        }
        Ok(())
    });
    hit
}

fn is_empty_run(el: &Element) -> bool {
    el.local_name() == "r" && el.element_children().all(|c| c.local_name() == "rPr")
}

fn embed_still_in(root: &Element, rid: &str) -> bool {
    let mut found = false;
    walk_all(root, &mut |e| {
        if e.local_name() == "blip" && e.attr_local("embed") == Some(rid) {
            found = true;
        }
    });
    found
}

fn source_from_rels(rels: &str) -> Option<String> {
    if rels == "/_rels/.rels" {
        return Some("/".into());
    }
    let (dir, file) = split_dir_file(rels);
    let parent = dir.strip_suffix("/_rels")?;
    let stem = file.strip_suffix(".rels")?;
    if parent.is_empty() || parent == "/" {
        Some(format!("/{stem}"))
    } else {
        Some(format!("{parent}/{stem}"))
    }
}

fn ensure_xmlns(el: &mut Element, prefix: &str, uri: &str) {
    let key = format!("xmlns:{prefix}");
    if el.attr(&key).is_none() {
        el.set_attr(&key, uri);
    }
}

fn set_attr_local(el: &mut Element, local: &str, value: &str) {
    if let Some((k, _)) = el
        .attrs
        .iter()
        .find(|(k, _)| k.rsplit(':').next().unwrap_or(k) == local)
    {
        let name = k.clone();
        el.set_attr(&name, value);
        return;
    }
    el.set_attr(local, value);
}

fn find_local_mut<'a>(el: &'a mut Element, local: &str) -> Option<&'a mut Element> {
    if el.local_name() == local {
        return Some(el);
    }
    let children = el.children.as_mut_slice();
    for child in children {
        if let Node::Element(e) = child {
            if let Some(found) = find_local_mut(e, local) {
                return Some(found);
            }
        }
    }
    None
}

fn extent_el(cx: i64, cy: i64) -> Element {
    Element::new("wp:extent")
        .with_attr("cx", cx.to_string())
        .with_attr("cy", cy.to_string())
}

fn doc_pr_el(id: u32, name: &str) -> Element {
    Element::new("wp:docPr")
        .with_attr("id", id.to_string())
        .with_attr("name", name)
}

fn cnv_frame_pr() -> Element {
    let locks = Element::new("a:graphicFrameLocks").with_attr("noChangeAspect", "1");
    let mut pr = Element::new("wp:cNvGraphicFramePr");
    pr.push_element(locks);
    pr
}

fn graphic_el(name: &str, rid: &str, cx: i64, cy: i64) -> Element {
    let c_nv_pr = Element::new("pic:cNvPr")
        .with_attr("id", "0")
        .with_attr("name", name);
    let pic_locks = Element::new("a:picLocks").with_attr("noChangeAspect", "1");
    let mut c_nv_pic = Element::new("pic:cNvPicPr");
    c_nv_pic.push_element(pic_locks);
    let mut nv = Element::new("pic:nvPicPr");
    nv.push_element(c_nv_pr);
    nv.push_element(c_nv_pic);

    let blip = Element::new("a:blip").with_attr("r:embed", rid);
    let mut stretch = Element::new("a:stretch");
    stretch.push_element(Element::new("a:fillRect"));
    let mut fill = Element::new("pic:blipFill");
    fill.push_element(blip);
    fill.push_element(stretch);

    let off = Element::new("a:off")
        .with_attr("x", "0")
        .with_attr("y", "0");
    let ext = Element::new("a:ext")
        .with_attr("cx", cx.to_string())
        .with_attr("cy", cy.to_string());
    let mut xfrm = Element::new("a:xfrm");
    xfrm.push_element(off);
    xfrm.push_element(ext);
    let mut geom = Element::new("a:prstGeom").with_attr("prst", "rect");
    geom.push_element(Element::new("a:avLst"));
    let mut sp = Element::new("pic:spPr");
    sp.push_element(xfrm);
    sp.push_element(geom);

    let mut pic = Element::new("pic:pic");
    pic.push_element(nv);
    pic.push_element(fill);
    pic.push_element(sp);

    let mut data = Element::new("a:graphicData").with_attr(
        "uri",
        "http://schemas.openxmlformats.org/drawingml/2006/picture",
    );
    data.push_element(pic);
    let mut graphic = Element::new("a:graphic");
    graphic.push_element(data);
    graphic
}

fn convert_to_inline(drawing: &mut Element) {
    let Some(old) = take_frame(drawing) else {
        return;
    };
    if old.local_name() == "inline" {
        drawing.children.insert(0, Node::Element(old));
        return;
    }
    let shared = take_shared_children(&old);
    let mut inline = Element::new("wp:inline")
        .with_attr("distT", "0")
        .with_attr("distB", "0")
        .with_attr("distL", "0")
        .with_attr("distR", "0");
    for c in shared {
        inline.children.push(Node::Element(c));
    }
    drawing.children.insert(0, Node::Element(inline));
}

fn convert_to_anchor(drawing: &mut Element, wrap: WrapKind, pos: &WrapPos) {
    let Some(old) = take_frame(drawing) else {
        return;
    };
    let shared = take_shared_children(&old);
    let keep_pos = old.local_name() == "anchor"
        && pos.align_h.is_none()
        && pos.align_v.is_none()
        && pos.offset_x.is_none()
        && pos.offset_y.is_none();
    let mut anchor = Element::new("wp:anchor")
        .with_attr("distT", "0")
        .with_attr("distB", "0")
        .with_attr("distL", "114300")
        .with_attr("distR", "114300")
        .with_attr("simplePos", "0")
        .with_attr("relativeHeight", "251658240")
        .with_attr("behindDoc", if wrap == WrapKind::None { "1" } else { "0" })
        .with_attr("locked", "0")
        .with_attr("layoutInCell", "1")
        .with_attr("allowOverlap", "1");
    if keep_pos {
        for c in old.element_children() {
            let n = c.local_name();
            if n == "simplePos" || n == "positionH" || n == "positionV" {
                anchor.push_element(c.clone());
            }
        }
    } else {
        anchor.push_element(
            Element::new("wp:simplePos")
                .with_attr("x", "0")
                .with_attr("y", "0"),
        );
        anchor.push_element(position_h(pos));
        anchor.push_element(position_v(pos));
    }
    let mut inserted_wrap = false;
    for c in shared {
        if !inserted_wrap && matches!(c.local_name(), "docPr" | "cNvGraphicFramePr" | "graphic") {
            anchor.push_element(wrap_el(wrap));
            inserted_wrap = true;
        }
        if WrapKind::from_local(c.local_name()).is_some() {
            continue;
        }
        anchor.push_element(c);
    }
    if !inserted_wrap {
        anchor.push_element(wrap_el(wrap));
    }
    drawing.children.insert(0, Node::Element(anchor));
}

fn take_frame(drawing: &mut Element) -> Option<Element> {
    let pos = drawing.children.iter().position(|n| match n {
        Node::Element(e) => e.local_name() == "inline" || e.local_name() == "anchor",
        Node::Text(_) => false,
    })?;
    match drawing.children.remove(pos) {
        Node::Element(e) => Some(e),
        Node::Text(_) => None,
    }
}

fn take_shared_children(frame: &Element) -> Vec<Element> {
    frame
        .element_children()
        .filter(|c| {
            matches!(
                c.local_name(),
                "extent" | "effectExtent" | "docPr" | "cNvGraphicFramePr" | "graphic"
            )
        })
        .cloned()
        .collect()
}

fn wrap_el(wrap: WrapKind) -> Element {
    let mut el = Element::new(format!("wp:{}", wrap.local_name()));
    if matches!(wrap, WrapKind::Square | WrapKind::Tight | WrapKind::Through) {
        el.set_attr("wrapText", "bothSides");
    }
    el
}

fn position_h(pos: &WrapPos) -> Element {
    let mut el = Element::new("wp:positionH").with_attr("relativeFrom", "margin");
    if let Some(off) = pos.offset_x {
        el.push_element({
            let mut o = Element::new("wp:posOffset");
            o.set_text(off.to_string());
            o
        });
    } else {
        let align = pos.align_h.unwrap_or(AlignH::Left);
        el.push_element({
            let mut a = Element::new("wp:align");
            a.set_text(align.as_str());
            a
        });
    }
    el
}

fn position_v(pos: &WrapPos) -> Element {
    let mut el = Element::new("wp:positionV").with_attr("relativeFrom", "margin");
    if let Some(off) = pos.offset_y {
        el.push_element({
            let mut o = Element::new("wp:posOffset");
            o.set_text(off.to_string());
            o
        });
    } else {
        let align = pos.align_v.unwrap_or(AlignV::Top);
        el.push_element({
            let mut a = Element::new("wp:align");
            a.set_text(align.as_str());
            a
        });
    }
    el
}

fn classify_image(bytes: Vec<u8>, path: &Path) -> Result<ImageFile, Error> {
    if bytes.len() >= 8 && bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) {
        let (cx, cy) = png_size(&bytes).unwrap_or((914400, 914400));
        return Ok(ImageFile {
            bytes,
            ext: "png".into(),
            content_type: "image/png".into(),
            cx,
            cy,
        });
    }
    if bytes.len() >= 3 && bytes[0] == 0xFF && bytes[1] == 0xD8 && bytes[2] == 0xFF {
        let (cx, cy) = jpeg_size(&bytes).unwrap_or((914400, 914400));
        return Ok(ImageFile {
            bytes,
            ext: "jpeg".into(),
            content_type: "image/jpeg".into(),
            cx,
            cy,
        });
    }
    if bytes.len() >= 6 && (bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a")) {
        let (cx, cy) = gif_size(&bytes).unwrap_or((914400, 914400));
        return Ok(ImageFile {
            bytes,
            ext: "gif".into(),
            content_type: "image/gif".into(),
            cx,
            cy,
        });
    }
    if bytes.len() >= 26 && bytes.starts_with(b"BM") {
        let (cx, cy) = bmp_size(&bytes).unwrap_or((914400, 914400));
        return Ok(ImageFile {
            bytes,
            ext: "bmp".into(),
            content_type: "image/bmp".into(),
            cx,
            cy,
        });
    }
    let _ = path;
    Err(Error::new(ErrorKind::InvalidImageFile))
}

fn px_to_emu(px: u32) -> i64 {
    (px as i64).saturating_mul(914400) / 96
}

fn png_size(bytes: &[u8]) -> Option<(i64, i64)> {
    // 签名 8 + 长度 4 + IHDR 4，宽高在偏移 16。
    if bytes.len() < 24 {
        return None;
    }
    let w = u32::from_be_bytes(bytes[16..20].try_into().ok()?);
    let h = u32::from_be_bytes(bytes[20..24].try_into().ok()?);
    Some((px_to_emu(w), px_to_emu(h)))
}

fn gif_size(bytes: &[u8]) -> Option<(i64, i64)> {
    if bytes.len() < 10 {
        return None;
    }
    let w = u16::from_le_bytes(bytes[6..8].try_into().ok()?) as u32;
    let h = u16::from_le_bytes(bytes[8..10].try_into().ok()?) as u32;
    Some((px_to_emu(w), px_to_emu(h)))
}

fn bmp_size(bytes: &[u8]) -> Option<(i64, i64)> {
    if bytes.len() < 26 {
        return None;
    }
    let w = i32::from_le_bytes(bytes[18..22].try_into().ok()?).unsigned_abs();
    let h = i32::from_le_bytes(bytes[22..26].try_into().ok()?).unsigned_abs();
    Some((px_to_emu(w), px_to_emu(h)))
}

fn jpeg_size(bytes: &[u8]) -> Option<(i64, i64)> {
    let mut i = 2usize;
    while i + 9 < bytes.len() {
        if bytes[i] != 0xFF {
            i += 1;
            continue;
        }
        let marker = bytes[i + 1];
        if marker == 0xD8 || marker == 0xD9 || (0xD0..=0xD7).contains(&marker) {
            i += 2;
            continue;
        }
        if i + 3 >= bytes.len() {
            break;
        }
        let len = u16::from_be_bytes([bytes[i + 2], bytes[i + 3]]) as usize;
        if matches!(marker, 0xC0..=0xC2) && i + 8 < bytes.len() {
            let h = u16::from_be_bytes([bytes[i + 5], bytes[i + 6]]) as u32;
            let w = u16::from_be_bytes([bytes[i + 7], bytes[i + 8]]) as u32;
            return Some((px_to_emu(w), px_to_emu(h)));
        }
        i += 2 + len;
    }
    None
}

#[allow(dead_code)]
pub fn container_id(kind: HfKind, hf_type: HfType, n: u32) -> String {
    format!("{}:{}:{n}", kind.as_str(), hf_type.as_str())
}

#[allow(dead_code)]
pub fn body_para_of(target: &ParaTarget) -> Option<&BodyPara> {
    match target {
        ParaTarget::HeaderFooter { para, .. } => Some(para),
        ParaTarget::Body(p) => Some(p),
        ParaTarget::InTextBox { para, .. } => Some(para),
        ParaTarget::InNote { para, .. } => Some(para),
        ParaTarget::InComment { para, .. } => Some(para),
    }
}
