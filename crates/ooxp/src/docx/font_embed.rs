//! fontTable 嵌入字体：inspect、插删与 exportMedia。
//! 不实现字体渲染、PANOSE 替换或 TTF 子集化引擎，不把 odttf 字节塞进树。

use std::collections::HashMap;

use serde_json::{json, Value};

use crate::error::{Error, ErrorKind, IdReason};
use crate::opc::{self, ContentTypes, Package, Relationship, Relationships};
use crate::xml::{self, Document, Element, Node};

use super::hf;
use super::notes;
use super::picture;
use super::{
    CT_FONT_TABLE, CT_OBFUSCATED_FONT, CT_SETTINGS, REL_FONT, REL_FONT_TABLE, REL_NS, REL_SETTINGS,
    WML_NS,
};

const FONT_TABLE_PART: &str = "/word/fontTable.xml";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontStyle {
    Regular,
    Bold,
    Italic,
    BoldItalic,
}

impl FontStyle {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "regular" => Some(Self::Regular),
            "bold" => Some(Self::Bold),
            "italic" => Some(Self::Italic),
            "boldItalic" => Some(Self::BoldItalic),
            _ => None,
        }
    }

    pub fn as_json(self) -> &'static str {
        match self {
            Self::Regular => "regular",
            Self::Bold => "bold",
            Self::Italic => "italic",
            Self::BoldItalic => "boldItalic",
        }
    }

    fn local(self) -> &'static str {
        match self {
            Self::Regular => "embedRegular",
            Self::Bold => "embedBold",
            Self::Italic => "embedItalic",
            Self::BoldItalic => "embedBoldItalic",
        }
    }

    fn from_local(local: &str) -> Option<Self> {
        match local {
            "embedRegular" => Some(Self::Regular),
            "embedBold" => Some(Self::Bold),
            "embedItalic" => Some(Self::Italic),
            "embedBoldItalic" => Some(Self::BoldItalic),
            _ => None,
        }
    }
}

struct EmbedRef {
    font_name: String,
    style: FontStyle,
    subsetted: bool,
    font_key: Option<String>,
    rid: Option<String>,
}

pub fn count(pkg: &Package, doc_part: &str) -> Result<usize, Error> {
    Ok(collect(pkg, doc_part)?.len())
}

pub fn flags_json(pkg: &Package, doc_part: &str) -> Result<Value, Error> {
    let flags = read_flags(pkg, doc_part)?;
    Ok(json!({
        "trueType": flags.0,
        "systemFonts": flags.1,
        "subset": flags.2,
    }))
}

pub fn fonts_json(pkg: &Package, doc_part: &str) -> Result<Vec<Value>, Error> {
    let embeds = collect(pkg, doc_part)?;
    Ok(embeds
        .into_iter()
        .enumerate()
        .map(|(i, e)| {
            let n = i + 1;
            json!({
                "id": format!("font:{n}"),
                "index": n,
                "name": e.font_name,
                "style": e.style.as_json(),
                "subsetted": e.subsetted,
                "fontKey": e.font_key,
            })
        })
        .collect())
}

pub fn parse_id(id: &str, path: &str) -> Result<usize, Error> {
    let rest = id.strip_prefix("font:").ok_or_else(|| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::FontPrefix,
            },
            path.to_string(),
        )
    })?;
    let n: usize = rest.parse().map_err(|_| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::FontIndexNotInt,
            },
            path.to_string(),
        )
    })?;
    if n < 1 {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::FontIndexFromOne,
            },
            path.to_string(),
        ));
    }
    Ok(n)
}

pub fn export_media(bytes: &[u8], id: &str) -> Result<super::export::ExportedMedia, Error> {
    let n = parse_id(id, "id")?;
    let pkg = Package::open_bytes(bytes.to_vec())?;
    let doc_part = opc::office_document_part(&pkg)?;
    let part = font_part_for(&pkg, &doc_part, n, id, "id")?;
    let media_bytes = pkg.get(&part)?.to_vec();
    let cts = ContentTypes::from_package(&pkg)?;
    let content_type = cts.content_type_of(&part).map(str::to_string);
    Ok(super::export::ExportedMedia {
        bytes: media_bytes,
        content_type,
    })
}

pub fn insert(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    name: &str,
    style: FontStyle,
    file: &str,
    op_i: usize,
) -> Result<(), Error> {
    let ttf = read_font_file(file).map_err(|e| e.at(format!("ops[{op_i}].file")))?;
    let part = ensure_font_table(pkg, docs, dirty, doc_part)?;
    {
        let doc = docs.get(&part).expect("ensure 已载入 fontTable");
        if find_embed(&doc.root, name, style).is_some() {
            return Err(Error::with_path(
                ErrorKind::DuplicateEmbeddedFont,
                format!("ops[{op_i}].name"),
            ));
        }
    }
    let (guid_bytes, font_key) = new_font_key();
    let odttf = obfuscate(&ttf, &guid_bytes);
    let font_part = next_font_part(pkg);
    pkg.set(&font_part, odttf)?;

    let mut ft_rels = Relationships::from_package(pkg, &part)?;
    let rid = ft_rels.next_rid();
    ft_rels.push(Relationship {
        id: rid.clone(),
        rel_type: REL_FONT.into(),
        target: picture::rel_target_from(&part, &font_part),
        target_mode: None,
        extra: Vec::new(),
    });
    ft_rels.write_to(pkg, &part)?;

    let mut cts = ContentTypes::from_package(pkg)?;
    cts.ensure_default("odttf", CT_OBFUSCATED_FONT);
    cts.write_to(pkg)?;

    {
        let doc = docs.get_mut(&part).expect("ensure 已载入 fontTable");
        ensure_xmlns(&mut doc.root, "r", REL_NS);
        append_embed(&mut doc.root, name, style, &rid, &font_key);
        dirty.insert(part, ());
    }
    set_true_type_on(pkg, docs, dirty, doc_part)?;
    Ok(())
}

pub fn delete(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    n: usize,
    err_id: &str,
    op_i: usize,
) -> Result<(), Error> {
    let Some(part) = find_font_table_part(pkg, doc_part)? else {
        return Err(missing(err_id, op_i));
    };
    if !docs.contains_key(&part) {
        let parsed = xml::parse(pkg.get(&part)?)?;
        docs.insert(part.clone(), parsed);
    }
    let total = {
        let doc = docs.get(&part).expect("已载入 fontTable");
        collect_from_root(&doc.root).len()
    };
    if n < 1 || n > total {
        return Err(missing(err_id, op_i));
    }
    let rid = {
        let doc = docs.get(&part).expect("已载入 fontTable");
        collect_from_root(&doc.root)[n - 1].rid.clone()
    };
    if let Some(rid) = rid.as_deref() {
        let still_used = {
            let doc = docs.get(&part).expect("已载入 fontTable");
            collect_from_root(&doc.root)
                .iter()
                .enumerate()
                .any(|(i, e)| i != n - 1 && e.rid.as_deref() == Some(rid))
        };
        if !still_used {
            if let Ok(target) = opc::resolve_id(pkg, &part, rid) {
                pkg.remove(&target);
            }
            let mut ft_rels = Relationships::from_package(pkg, &part)?;
            ft_rels.remove_by_id(rid);
            ft_rels.write_to(pkg, &part)?;
        }
    }
    {
        let doc = docs.get_mut(&part).expect("已载入 fontTable");
        remove_embed_at(&mut doc.root, n);
        dirty.insert(part, ());
    }
    Ok(())
}

pub fn set_flags(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    true_type: Option<bool>,
    system_fonts: Option<bool>,
    subset: Option<bool>,
) -> Result<(), Error> {
    let need_on = true_type == Some(true) || system_fonts == Some(true) || subset == Some(true);
    let settings = match notes::settings_part_name(pkg, doc_part)? {
        Some(p) => p,
        None if need_on => ensure_settings(pkg, docs, dirty, doc_part)?,
        None => return Ok(()),
    };
    if !docs.contains_key(&settings) {
        let parsed = xml::parse(pkg.get(&settings)?)?;
        docs.insert(settings.clone(), parsed);
    }
    let doc = docs.get_mut(&settings).expect("已载入 settings");
    if let Some(on) = true_type {
        hf::set_on_off(&mut doc.root, "embedTrueTypeFonts", on, hf::SETTINGS_ORDER);
    }
    if let Some(on) = system_fonts {
        hf::set_on_off(&mut doc.root, "embedSystemFonts", on, hf::SETTINGS_ORDER);
    }
    if let Some(on) = subset {
        hf::set_on_off(&mut doc.root, "saveSubsetFonts", on, hf::SETTINGS_ORDER);
    }
    dirty.insert(settings, ());
    Ok(())
}

fn missing(id: &str, op_i: usize) -> Error {
    Error::with_path(
        ErrorKind::FontNotFound { id: id.to_string() },
        format!("ops[{op_i}].id"),
    )
}

fn collect(pkg: &Package, doc_part: &str) -> Result<Vec<EmbedRef>, Error> {
    let Some(part) = find_font_table_part(pkg, doc_part)? else {
        return Ok(Vec::new());
    };
    let Ok(bytes) = pkg.get(&part) else {
        return Ok(Vec::new());
    };
    let doc = xml::parse(bytes)?;
    Ok(collect_from_root(&doc.root))
}

fn collect_from_root(root: &Element) -> Vec<EmbedRef> {
    let mut out = Vec::new();
    for font in root.element_children() {
        if font.local_name() != "font" {
            continue;
        }
        let name = font.attr_local("name").unwrap_or("").to_string();
        for child in font.element_children() {
            let Some(style) = FontStyle::from_local(child.local_name()) else {
                continue;
            };
            out.push(EmbedRef {
                font_name: name.clone(),
                style,
                subsetted: attr_on_off(child, "subsetted"),
                font_key: child.attr_local("fontKey").map(str::to_string),
                rid: child.attr_local("id").map(str::to_string),
            });
        }
    }
    out
}

fn find_embed(root: &Element, name: &str, style: FontStyle) -> Option<usize> {
    collect_from_root(root)
        .into_iter()
        .position(|e| e.font_name == name && e.style == style)
}

fn append_embed(root: &mut Element, name: &str, style: FontStyle, rid: &str, font_key: &str) {
    let embed = Element::new(root.prefixed(style.local()))
        .with_attr("r:id", rid)
        .with_attr("w:fontKey", font_key);
    if let Some(font) = root
        .element_children_mut()
        .find(|e| e.local_name() == "font" && e.attr_local("name") == Some(name))
    {
        font.push_element(embed);
        return;
    }
    let mut font = Element::new(root.prefixed("font")).with_attr("w:name", name);
    font.push_element(embed);
    root.push_element(font);
}

fn remove_embed_at(root: &mut Element, n: usize) {
    let mut seen = 0usize;
    for font in root.element_children_mut() {
        if font.local_name() != "font" {
            continue;
        }
        let mut drop_at = None;
        for (i, child) in font.children.iter().enumerate() {
            let Node::Element(el) = child else {
                continue;
            };
            if FontStyle::from_local(el.local_name()).is_none() {
                continue;
            }
            seen += 1;
            if seen == n {
                drop_at = Some(i);
                break;
            }
        }
        if let Some(i) = drop_at {
            font.children.remove(i);
            return;
        }
    }
}

fn attr_on_off(el: &Element, local: &str) -> bool {
    match el.attr_local(local) {
        None => false,
        Some(v) => {
            let lower = v.to_ascii_lowercase();
            matches!(lower.as_str(), "1" | "true" | "on")
        }
    }
}

fn read_flags(pkg: &Package, doc_part: &str) -> Result<(bool, bool, bool), Error> {
    let Some(settings) = notes::settings_part_name(pkg, doc_part)? else {
        return Ok((false, false, false));
    };
    let Ok(bytes) = pkg.get(&settings) else {
        return Ok((false, false, false));
    };
    let doc = xml::parse(bytes)?;
    Ok((
        hf::bool_prop(&doc.root, "embedTrueTypeFonts"),
        hf::bool_prop(&doc.root, "embedSystemFonts"),
        hf::bool_prop(&doc.root, "saveSubsetFonts"),
    ))
}

fn find_font_table_part(pkg: &Package, doc_part: &str) -> Result<Option<String>, Error> {
    let rels = Relationships::from_package(pkg, doc_part)?;
    for r in rels.find_by_type_suffix("/fontTable") {
        if r.is_external() {
            continue;
        }
        let Ok(p) = opc::resolve_rel_target(doc_part, &r.target) else {
            continue;
        };
        if pkg.contains(&p) {
            return Ok(Some(p));
        }
    }
    Ok(None)
}

fn ensure_font_table(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
) -> Result<String, Error> {
    if let Some(existing) = find_font_table_part(pkg, doc_part)? {
        if !docs.contains_key(&existing) {
            let parsed = xml::parse(pkg.get(&existing)?)?;
            docs.insert(existing.clone(), parsed);
        }
        return Ok(existing);
    }
    pkg.set(FONT_TABLE_PART, empty_font_table().into_bytes())?;
    let parsed = xml::parse(pkg.get(FONT_TABLE_PART)?)?;
    docs.insert(FONT_TABLE_PART.to_string(), parsed);
    dirty.insert(FONT_TABLE_PART.to_string(), ());

    let mut doc_rels = Relationships::from_package(pkg, doc_part)?;
    doc_rels.push(Relationship {
        id: doc_rels.next_rid(),
        rel_type: REL_FONT_TABLE.into(),
        target: picture::rel_target_from(doc_part, FONT_TABLE_PART),
        target_mode: None,
        extra: Vec::new(),
    });
    doc_rels.write_to(pkg, doc_part)?;

    let mut cts = ContentTypes::from_package(pkg)?;
    cts.set_override(FONT_TABLE_PART, CT_FONT_TABLE)?;
    cts.ensure_default("odttf", CT_OBFUSCATED_FONT);
    cts.write_to(pkg)?;
    Ok(FONT_TABLE_PART.to_string())
}

fn empty_font_table() -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:fonts xmlns:w="{WML_NS}" xmlns:r="{REL_NS}"/>"#
    )
}

fn next_font_part(pkg: &Package) -> String {
    let mut n = 1u32;
    loop {
        let name = format!("/word/fonts/font{n}.odttf");
        if !pkg.contains(&name) {
            return name;
        }
        n += 1;
    }
}

fn font_part_for(
    pkg: &Package,
    doc_part: &str,
    n: usize,
    id: &str,
    path: &str,
) -> Result<String, Error> {
    let embeds = collect(pkg, doc_part)?;
    if n < 1 || n > embeds.len() {
        return Err(Error::with_path(
            ErrorKind::FontNotFound { id: id.to_string() },
            path.to_string(),
        ));
    }
    let Some(rid) = embeds[n - 1].rid.as_deref() else {
        return Err(Error::with_path(
            ErrorKind::FontNotFound { id: id.to_string() },
            path.to_string(),
        ));
    };
    let table = find_font_table_part(pkg, doc_part)?.ok_or_else(|| {
        Error::with_path(
            ErrorKind::FontNotFound { id: id.to_string() },
            path.to_string(),
        )
    })?;
    opc::resolve_id(pkg, &table, rid)
}

fn set_true_type_on(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
) -> Result<(), Error> {
    let settings = match notes::settings_part_name(pkg, doc_part)? {
        Some(p) => p,
        None => ensure_settings(pkg, docs, dirty, doc_part)?,
    };
    if !docs.contains_key(&settings) {
        let parsed = xml::parse(pkg.get(&settings)?)?;
        docs.insert(settings.clone(), parsed);
    }
    let (already, _, _) = {
        let doc = docs.get(&settings).expect("已载入 settings");
        (hf::bool_prop(&doc.root, "embedTrueTypeFonts"), false, false)
    };
    if already {
        return Ok(());
    }
    let doc = docs.get_mut(&settings).expect("已载入 settings");
    hf::set_on_off(
        &mut doc.root,
        "embedTrueTypeFonts",
        true,
        hf::SETTINGS_ORDER,
    );
    dirty.insert(settings, ());
    Ok(())
}

fn ensure_settings(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
) -> Result<String, Error> {
    let part = "/word/settings.xml".to_string();
    if !pkg.contains(&part) {
        pkg.set(&part, minimal_settings().into_bytes())?;
    }
    if !docs.contains_key(&part) {
        let parsed = xml::parse(pkg.get(&part)?)?;
        docs.insert(part.clone(), parsed);
    }
    dirty.insert(part.clone(), ());
    let mut doc_rels = Relationships::from_package(pkg, doc_part)?;
    if doc_rels.find_by_type_suffix("/settings").is_empty() {
        doc_rels.push(Relationship {
            id: doc_rels.next_rid(),
            rel_type: REL_SETTINGS.into(),
            target: picture::rel_target_from(doc_part, &part),
            target_mode: None,
            extra: Vec::new(),
        });
        doc_rels.write_to(pkg, doc_part)?;
    }
    let mut cts = ContentTypes::from_package(pkg)?;
    cts.set_override(&part, CT_SETTINGS)?;
    cts.write_to(pkg)?;
    Ok(part)
}

fn minimal_settings() -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:settings xmlns:w="{WML_NS}"><w:compat><w:compatSetting w:name="compatibilityMode" w:uri="http://schemas.microsoft.com/office/word" w:val="15"/></w:compat></w:settings>"#
    )
}

fn read_font_file(path: &str) -> Result<Vec<u8>, Error> {
    let bytes = crate::host_files::read_user_file(path)?;
    validate_font(&bytes)?;
    Ok(bytes)
}

fn validate_font(bytes: &[u8]) -> Result<(), Error> {
    if bytes.len() < 32 {
        return Err(Error::new(ErrorKind::InvalidFontFile));
    }
    if bytes.starts_with(b"ttcf") {
        return Err(Error::new(ErrorKind::InvalidFontFile));
    }
    let magic = &bytes[0..4];
    if magic == [0, 1, 0, 0] || magic == b"OTTO" || magic == b"true" {
        return Ok(());
    }
    Err(Error::new(ErrorKind::InvalidFontFile))
}

fn new_font_key() -> ([u8; 16], String) {
    let mut b = [0u8; 16];
    getrandom::getrandom(&mut b).expect("系统随机数不可用");
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    let guid = format!(
        "{{{:02X}{:02X}{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}}}",
        b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7], b[8], b[9], b[10], b[11], b[12], b[13],
        b[14], b[15]
    );
    (b, guid)
}

fn obfuscate(ttf: &[u8], guid: &[u8; 16]) -> Vec<u8> {
    let mut out = ttf.to_vec();
    let mut key = *guid;
    key.reverse();
    for i in 0..16 {
        out[i] ^= key[i];
        out[i + 16] ^= key[i];
    }
    out
}

fn ensure_xmlns(el: &mut Element, prefix: &str, uri: &str) {
    let key = format!("xmlns:{prefix}");
    if el.attr(&key).is_none() {
        el.set_attr(&key, uri);
    }
}
