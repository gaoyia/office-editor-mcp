//! 工作表背景图：`<picture r:id>` 挂在 worksheet 上，不进 drawing。

use serde_json::{json, Value};

use super::picture;
use super::{ExportedMedia, REL_NS};
use crate::error::{Error, ErrorKind};
use crate::image::read_image;
use crate::opc::{self, rels_part_name, ContentTypes, Package, Relationship, Relationships};
use crate::xml::{Element, Node};

pub(crate) fn looks_like_id(id: &str) -> bool {
    let Some(rest) = id.strip_prefix("sheet:") else {
        return false;
    };
    let Some(name) = rest.strip_suffix("/background") else {
        return false;
    };
    !name.is_empty() && !name.contains('/')
}

pub(crate) fn require_worksheet(
    index: &[(String, String, &'static str)],
    name: &str,
    op_index: usize,
) -> Result<String, Error> {
    if let Some((_, part, _)) = index
        .iter()
        .find(|(n, _, k)| n.eq_ignore_ascii_case(name) && *k == "worksheet")
    {
        return Ok(part.clone());
    }
    if index.iter().any(|(n, _, _)| n.eq_ignore_ascii_case(name)) {
        return Err(Error::with_path(
            ErrorKind::SheetBackgroundWorksheetOnly,
            format!("ops[{op_index}].id"),
        ));
    }
    Err(Error::with_path(
        ErrorKind::SheetNotFound {
            name: name.to_string(),
        },
        format!("ops[{op_index}].id"),
    ))
}

pub(crate) fn inspect_value(
    pkg: &Package,
    sheet_part: &str,
    root: &Element,
    name: &str,
) -> Result<Value, Error> {
    let Some(pic) = root.child_by_local("picture") else {
        return Ok(Value::Null);
    };
    let content_type = picture_content_type(pkg, sheet_part, pic)?;
    Ok(json!({
        "id": format!("sheet:{name}/background"),
        "contentType": content_type,
    }))
}

pub(crate) fn set(
    pkg: &mut Package,
    sheet_part: &str,
    root: &mut Element,
    file: &str,
    field: &str,
) -> Result<(), Error> {
    let image = read_image(file).map_err(|e| e.at(field))?;
    let (bytes, ext, content_type) = media_payload(&image);
    let media = picture::next_media_part(pkg, &ext);
    pkg.set(&media, bytes)?;
    let mut cts = ContentTypes::from_package(pkg)?;
    cts.ensure_default(&ext, &content_type);
    cts.write_to(pkg)?;

    picture::ensure_xmlns(root, "r", REL_NS);
    let mut rels = Relationships::from_package(pkg, sheet_part)?;
    let old_media = existing_media(pkg, sheet_part, root);
    if let Some(pic) = picture_mut(root) {
        let rid = match pic.attr_local("id") {
            Some(id) if rels.find_by_id(id).is_some() => id.to_string(),
            _ => {
                let rid = rels.next_rid();
                rels.push(image_rel(&rid, sheet_part, &media));
                set_rid(pic, &rid);
                rels.write_to(pkg, sheet_part)?;
                drop_unused(pkg, old_media.as_deref(), &media)?;
                return Ok(());
            }
        };
        if let Some(rel) = rels.find_by_id_mut(&rid) {
            rel.rel_type = picture::REL_IMAGE.into();
            rel.target = picture::rel_target_from(sheet_part, &media);
            rel.target_mode = None;
        }
    } else {
        let rid = rels.next_rid();
        rels.push(image_rel(&rid, sheet_part, &media));
        let pic = Element::new("picture").with_attr("r:id", &rid);
        super::layout::insert_child_ordered(root, pic, "picture");
    }
    rels.write_to(pkg, sheet_part)?;
    drop_unused(pkg, old_media.as_deref(), &media)?;
    Ok(())
}

pub(crate) fn delete(
    pkg: &mut Package,
    sheet_part: &str,
    root: &mut Element,
    op_index: usize,
) -> Result<(), Error> {
    let Some(idx) = picture_index(root) else {
        return Err(Error::with_path(
            ErrorKind::SheetBackgroundNotFound,
            format!("ops[{op_index}].id"),
        ));
    };
    let Node::Element(pic) = root.children.remove(idx) else {
        return Err(Error::with_path(
            ErrorKind::SheetBackgroundNotFound,
            format!("ops[{op_index}].id"),
        ));
    };
    let rid = pic.attr_local("id").unwrap_or("").to_string();
    let old_media = if rid.is_empty() {
        None
    } else {
        opc::resolve_id(pkg, sheet_part, &rid).ok()
    };
    if !rid.is_empty() {
        let mut rels = Relationships::from_package(pkg, sheet_part)?;
        rels.remove_by_id(&rid);
        if rels.rels.is_empty() {
            pkg.remove(&rels_part_name(sheet_part));
        } else {
            rels.write_to(pkg, sheet_part)?;
        }
    }
    if let Some(media) = old_media {
        if !picture::media_still_used(pkg, &media)? {
            pkg.remove(&media);
        }
    }
    Ok(())
}

pub fn export(bytes: &[u8], id: &str) -> Result<ExportedMedia, Error> {
    let name = id
        .strip_prefix("sheet:")
        .and_then(|rest| rest.strip_suffix("/background"))
        .unwrap_or("");
    let pkg = Package::open_bytes(bytes.to_vec())?;
    let (part, kind) = find_sheet(&pkg, name)?;
    if kind != "worksheet" {
        return Err(Error::with_path(
            ErrorKind::SheetBackgroundWorksheetOnly,
            "id",
        ));
    }
    let doc = super::list_pin::parse_part(&pkg, &part)?;
    let pic = doc
        .root
        .child_by_local("picture")
        .ok_or_else(|| Error::with_path(ErrorKind::SheetBackgroundNotFound, "id"))?;
    let rid = pic.attr_local("id").unwrap_or("");
    let media = opc::resolve_id(&pkg, &part, rid)?;
    let media_bytes = pkg.get(&media)?.to_vec();
    let cts = ContentTypes::from_package(&pkg)?;
    Ok(ExportedMedia {
        bytes: media_bytes,
        content_type: cts.content_type_of(&media).map(str::to_string),
    })
}

fn media_payload(image: &crate::image::ImageFile) -> (Vec<u8>, String, String) {
    if let Some(svg) = &image.svg {
        return (svg.clone(), "svg".into(), crate::svg::CT.into());
    }
    (
        image.bytes.clone(),
        image.ext.clone(),
        image.content_type.clone(),
    )
}

fn image_rel(rid: &str, sheet_part: &str, media: &str) -> Relationship {
    Relationship {
        id: rid.to_string(),
        rel_type: picture::REL_IMAGE.into(),
        target: picture::rel_target_from(sheet_part, media),
        target_mode: None,
        extra: Vec::new(),
    }
}

fn set_rid(pic: &mut Element, rid: &str) {
    if pic.attr_local("id").is_some() {
        picture::set_attr_local(pic, "id", rid);
    } else {
        pic.set_attr("r:id", rid);
    }
}

fn picture_index(root: &Element) -> Option<usize> {
    root.children
        .iter()
        .position(|n| matches!(n, Node::Element(e) if e.local_name() == "picture"))
}

fn picture_mut(root: &mut Element) -> Option<&mut Element> {
    let idx = picture_index(root)?;
    match &mut root.children[idx] {
        Node::Element(el) => Some(el),
        Node::Text(_) | Node::Raw(_) => None,
    }
}

fn existing_media(pkg: &Package, sheet_part: &str, root: &Element) -> Option<String> {
    let rid = root.child_by_local("picture")?.attr_local("id")?;
    opc::resolve_id(pkg, sheet_part, rid).ok()
}

fn drop_unused(pkg: &mut Package, old: Option<&str>, media: &str) -> Result<(), Error> {
    let Some(old) = old else {
        return Ok(());
    };
    if old != media && !picture::media_still_used(pkg, old)? {
        pkg.remove(old);
    }
    Ok(())
}

fn picture_content_type(
    pkg: &Package,
    sheet_part: &str,
    pic: &Element,
) -> Result<Option<String>, Error> {
    let Some(rid) = pic.attr_local("id") else {
        return Ok(None);
    };
    let Ok(media) = opc::resolve_id(pkg, sheet_part, rid) else {
        return Ok(None);
    };
    let cts = ContentTypes::from_package(pkg)?;
    Ok(cts.content_type_of(&media).map(str::to_string))
}

fn find_sheet(pkg: &Package, name: &str) -> Result<(String, &'static str), Error> {
    let wb_part = opc::office_document_part(pkg)?;
    let wb = super::list_pin::parse_part(pkg, &wb_part)?;
    let rels = Relationships::from_package(pkg, &wb_part)?;
    let sheets = super::inspect::list_sheets(&wb.root, &rels, &wb_part, pkg)?;
    sheets
        .iter()
        .find(|s| s.name.eq_ignore_ascii_case(name))
        .map(|s| (s.part.clone(), s.kind))
        .ok_or_else(|| {
            Error::with_path(
                ErrorKind::SheetNotFound {
                    name: name.to_string(),
                },
                "id",
            )
        })
}
