//! InkML 校验、常量和写媒体。AlternateContent 外壳留在各格式。

use crate::error::{Error, ErrorKind};
use crate::host_files;
use crate::ole;
use crate::opc::{self, ContentTypes, Package, Relationship, Relationships};
use crate::xml::{self, Element};

pub const INKML_NS: &str = "http://www.w3.org/2003/InkML";
pub const CT_INKML: &str = "application/inkml+xml";
pub const REL_CUSTOM_XML: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/customXml";

#[derive(Debug, Clone)]
pub enum InkKey {
    Id(String),
    Index(usize),
}

pub fn key_matches(key: &InkKey, raw: &Option<String>, index: usize) -> bool {
    match key {
        InkKey::Index(n) => index == *n,
        InkKey::Id(want) => {
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

pub fn read_inkml(path: &str, json_path: &str) -> Result<Vec<u8>, Error> {
    let bytes = host_files::read_user_file(path)?;
    validate_inkml(&bytes, json_path)?;
    Ok(bytes)
}

pub fn validate_inkml(bytes: &[u8], json_path: &str) -> Result<(), Error> {
    let err = || Error::with_path(ErrorKind::InvalidInkMl, json_path.to_string());
    let doc = xml::parse(bytes).map_err(|_| err())?;
    if !is_inkml_root(&doc.root) {
        return Err(err());
    }
    Ok(())
}

pub fn is_inkml_root(el: &Element) -> bool {
    if el.local_name() != "ink" {
        return false;
    }
    el.attrs
        .iter()
        .any(|(k, v)| (k == "xmlns" || k.starts_with("xmlns:")) && v == INKML_NS)
}

/// 写出 InkML，并在宿主上挂 customXml 关系。`png_part` 有值时再挂一张后备 PNG。
pub fn write_media(
    pkg: &mut Package,
    host_part: &str,
    ink_dir: &str,
    inkml: Vec<u8>,
    png_part: Option<String>,
    rel_target: fn(&str, &str) -> String,
    image_rel: &str,
) -> Result<(String, Option<String>), Error> {
    let ink_part = ole::next_numbered(pkg, ink_dir, "ink", "xml");
    pkg.set(&ink_part, inkml)?;
    let mut rels = Relationships::from_package(pkg, host_part)?;
    let ink_rid = rels.next_rid();
    rels.push(Relationship {
        id: ink_rid.clone(),
        rel_type: REL_CUSTOM_XML.into(),
        target: rel_target(host_part, &ink_part),
        target_mode: None,
        extra: Vec::new(),
    });
    let png_rid = if let Some(png_part) = png_part {
        pkg.set(&png_part, ole::PREVIEW_PNG.to_vec())?;
        let png_rid = rels.next_rid();
        rels.push(Relationship {
            id: png_rid.clone(),
            rel_type: image_rel.into(),
            target: rel_target(host_part, &png_part),
            target_mode: None,
            extra: Vec::new(),
        });
        Some(png_rid)
    } else {
        None
    };
    rels.write_to(pkg, host_part)?;
    let mut cts = ContentTypes::from_package(pkg)?;
    cts.set_override(&ink_part, CT_INKML)?;
    if png_rid.is_some() {
        cts.ensure_default("png", "image/png");
    }
    cts.write_to(pkg)?;
    Ok((ink_rid, png_rid))
}

pub fn drop_rids(
    pkg: &mut Package,
    part: &str,
    root: &Element,
    rids: &[String],
    media_still_used: fn(&Package, &str) -> Result<bool, Error>,
) -> Result<(), Error> {
    let mut rels = Relationships::from_package(pkg, part)?;
    let mut dirty = false;
    let mut media_parts = Vec::new();
    for rid in rids {
        if rid_still_used(root, rid) {
            continue;
        }
        if let Ok(m) = opc::resolve_id(pkg, part, rid) {
            if !media_parts.iter().any(|x| x == &m) {
                media_parts.push(m);
            }
        }
        rels.remove_by_id(rid);
        dirty = true;
    }
    if dirty {
        if rels.rels.is_empty() {
            pkg.remove(&opc::rels_part_name(part));
        } else {
            rels.write_to(pkg, part)?;
        }
    }
    let mut cts = ContentTypes::from_package(pkg)?;
    let mut cts_dirty = false;
    for m in media_parts {
        if !media_still_used(pkg, &m)? {
            pkg.remove(&m);
            if m.contains("/ink/") {
                cts.remove_override(&m);
                cts_dirty = true;
            }
        }
    }
    if cts_dirty {
        cts.write_to(pkg)?;
    }
    Ok(())
}

fn rid_still_used(root: &Element, rid: &str) -> bool {
    let mut used = false;
    xml::walk_elements(root, &mut |e| {
        if e.local_name() == "contentPart" && e.attr_local("id") == Some(rid) {
            used = true;
        }
        if e.local_name() == "blip" && e.attr_local("embed") == Some(rid) {
            used = true;
        }
    });
    used
}
