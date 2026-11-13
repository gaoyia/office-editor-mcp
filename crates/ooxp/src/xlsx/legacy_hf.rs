//! 只有 `legacyDrawingHF`、没有 `drawingHF` 的旧式页眉图。
//! 六个形状 id 对奇数页页眉页脚的左、中、右。两套都在时不进这份名单。

use serde_json::{json, Map, Value};

use crate::error::{Error, ErrorKind, IdReason};
use crate::image::ImageFile;
use crate::opc::{ContentTypes, Relationships};
use crate::xml::{Element, Node};

use super::list_pin;
use super::picture;
use super::request::HfSlot;

const SHAPE_SLOTS: &[(&str, HfSlot)] = &[
    ("LH", HfSlot::ODD_HEADER),
    ("CH", HfSlot::ODD_HEADER),
    ("RH", HfSlot::ODD_HEADER),
    ("LF", HfSlot::ODD_FOOTER),
    ("CF", HfSlot::ODD_FOOTER),
    ("RF", HfSlot::ODD_FOOTER),
];

pub(crate) struct Listed {
    pub slot: HfSlot,
    pub value: Value,
}

struct Hit {
    slot: HfSlot,
    shape_id: String,
    content_type: Option<String>,
}

/// id 里带 `/legacyPic:` 就是旧式页眉图，要在普通页眉图之前分流。
pub(crate) fn looks_like_id(id: &str) -> bool {
    id.contains("/legacyPic:")
}

pub(crate) fn parse_id(id: &str, field_path: &str) -> Result<(String, HfSlot, String), Error> {
    let invalid = || {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SheetLegacyPicturePrefix,
            },
            field_path,
        )
    };
    let Some(rest) = id.strip_prefix("sheet:") else {
        return Err(invalid());
    };
    let Some((head, shape_id)) = rest.rsplit_once("/legacyPic:") else {
        return Err(invalid());
    };
    if shape_id.is_empty() || head.is_empty() {
        return Err(invalid());
    }
    let Some((sheet, slot_s)) = head.split_once("/hf:") else {
        return Err(invalid());
    };
    if sheet.is_empty() {
        return Err(invalid());
    }
    let Some(slot) = super::hf::parse_slot(slot_s) else {
        return Err(invalid());
    };
    let Some(expected) = slot_of(shape_id) else {
        return Err(invalid());
    };
    if expected != slot {
        return Err(invalid());
    }
    Ok((sheet.to_string(), slot, shape_id.to_string()))
}

pub(crate) fn count(
    pkg: &crate::opc::Package,
    sheet_part: &str,
    root: &Element,
    kind: &str,
) -> Result<usize, Error> {
    Ok(hits(pkg, sheet_part, root, kind)?.len())
}

pub(crate) fn collect(
    pkg: &crate::opc::Package,
    sheet_part: &str,
    sheet_name: &str,
    root: &Element,
    kind: &str,
) -> Result<Vec<Listed>, Error> {
    let mut out = Vec::new();
    for hit in hits(pkg, sheet_part, root, kind)? {
        let mut value = Map::new();
        value.insert(
            "id".into(),
            json!(node_id(sheet_name, hit.slot, &hit.shape_id)),
        );
        value.insert("shapeId".into(), json!(hit.shape_id));
        if let Some(ct) = hit.content_type {
            value.insert("contentType".into(), json!(ct));
        }
        out.push(Listed {
            slot: hit.slot,
            value: Value::Object(value),
        });
    }
    Ok(out)
}

pub(crate) fn export(bytes: &[u8], id: &str) -> Result<super::ExportedMedia, Error> {
    let (sheet, _slot, shape_id) = parse_id(id, "id")?;
    let pkg = crate::opc::Package::open_bytes(bytes.to_vec())?;
    let sheet_part = picture::lookup_sheet_part(&pkg, &sheet, "id")?;
    let sheet_doc = list_pin::parse_part(&pkg, &sheet_part)?;
    let Some(found) = locate(&pkg, &sheet_doc.root, &sheet_part, &shape_id)? else {
        return Err(Error::with_path(
            ErrorKind::LegacyPictureNotFound { id: id.to_string() },
            "id",
        ));
    };
    let media_bytes = pkg.get(&found.media)?.to_vec();
    let cts = ContentTypes::from_package(&pkg)?;
    let content_type = cts.content_type_of(&found.media).map(str::to_string);
    Ok(super::ExportedMedia {
        bytes: media_bytes,
        content_type,
    })
}

pub(crate) fn replace(
    pkg: &mut crate::opc::Package,
    sheet: &Element,
    sheet_part: &str,
    id: &str,
    shape_id: &str,
    file: &str,
    op_index: usize,
) -> Result<(), Error> {
    let path = format!("ops[{op_index}].id");
    let file_path = format!("ops[{op_index}].file");
    let image = crate::image::read_image(file).map_err(|e| e.at(file_path))?;
    if image.svg.is_some() {
        return Err(Error::with_path(
            ErrorKind::LegacyPictureRasterOnly,
            format!("ops[{op_index}].file"),
        ));
    }
    let Some(found) = locate(pkg, sheet, sheet_part, shape_id)? else {
        return Err(Error::with_path(
            ErrorKind::LegacyPictureNotFound { id: id.to_string() },
            path,
        ));
    };
    write_media(pkg, &found, shape_id, &image)?;
    Ok(())
}

pub(crate) fn delete(
    pkg: &mut crate::opc::Package,
    sheet: &mut Element,
    sheet_part: &str,
    id: &str,
    shape_id: &str,
    op_index: usize,
) -> Result<(), Error> {
    let path = format!("ops[{op_index}].id");
    let Some(found) = locate(pkg, sheet, sheet_part, shape_id)? else {
        return Err(Error::with_path(
            ErrorKind::LegacyPictureNotFound { id: id.to_string() },
            path,
        ));
    };
    let mut doc = list_pin::parse_part(pkg, &found.vml_part)?;
    let users = count_rid(&doc.root, &found.image_rid);
    if !remove_shape(&mut doc.root, shape_id) {
        return Err(Error::with_path(
            ErrorKind::LegacyPictureNotFound { id: id.to_string() },
            path,
        ));
    }
    if users <= 1 {
        let mut rels = Relationships::from_package(pkg, &found.vml_part)?;
        rels.remove_by_id(&found.image_rid);
        rels.write_to(pkg, &found.vml_part)?;
    }
    list_pin::write_part(pkg, &found.vml_part, &doc)?;
    if !any_shape(&doc.root) {
        pkg.remove(&found.vml_part);
        pkg.remove(&crate::opc::rels_part_name(&found.vml_part));
        let mut cts = ContentTypes::from_package(pkg)?;
        cts.remove_override(&found.vml_part);
        cts.write_to(pkg)?;
        let mut sheet_rels = Relationships::from_package(pkg, sheet_part)?;
        sheet_rels.remove_by_id(&found.hf_rid);
        if sheet_rels.rels.is_empty() {
            pkg.remove(&crate::opc::rels_part_name(sheet_part));
        } else {
            sheet_rels.write_to(pkg, sheet_part)?;
        }
        sheet.children.retain(|n| match n {
            Node::Element(e) => e.local_name() != "legacyDrawingHF",
            _ => true,
        });
    }
    if !picture::media_still_used(pkg, &found.media)? {
        pkg.remove(&found.media);
    }
    Ok(())
}

struct Found {
    vml_part: String,
    hf_rid: String,
    image_rid: String,
    media: String,
}

fn hits(
    pkg: &crate::opc::Package,
    sheet_part: &str,
    root: &Element,
    kind: &str,
) -> Result<Vec<Hit>, Error> {
    if kind != "worksheet" {
        return Ok(Vec::new());
    }
    let Some((_hf_rid, vml_part)) = legacy_vml(pkg, sheet_part, root)? else {
        return Ok(Vec::new());
    };
    let doc = list_pin::parse_part(pkg, &vml_part)?;
    let cts = ContentTypes::from_package(pkg)?;
    let mut shapes = Vec::new();
    collect_shapes(&doc.root, &mut shapes);
    let mut out = Vec::new();
    for shape in shapes {
        let Some(shape_id) = shape.attr_local("id") else {
            continue;
        };
        let Some(slot) = slot_of(shape_id) else {
            continue;
        };
        let Some(image_rid) = shape_image_rid(shape) else {
            continue;
        };
        let media = crate::opc::resolve_id(pkg, &vml_part, image_rid)?;
        let content_type = cts.content_type_of(&media).map(str::to_string);
        out.push(Hit {
            slot,
            shape_id: shape_id.to_string(),
            content_type,
        });
    }
    Ok(out)
}

fn legacy_vml(
    pkg: &crate::opc::Package,
    sheet_part: &str,
    root: &Element,
) -> Result<Option<(String, String)>, Error> {
    if root.child_by_local("drawingHF").is_some() {
        return Ok(None);
    }
    let Some(el) = root.child_by_local("legacyDrawingHF") else {
        return Ok(None);
    };
    let Some(rid) = el.attr_local("id") else {
        return Ok(None);
    };
    let rels = Relationships::from_package(pkg, sheet_part)?;
    let Some(rel) = rels.find_by_id(rid) else {
        return Err(Error::new(ErrorKind::RelNotFound {
            source: crate::error::RelSource::Part(sheet_part.to_string()),
            rid: rid.to_string(),
        }));
    };
    if !rel.rel_type.ends_with("/vmlDrawing") {
        return Ok(None);
    }
    let part = crate::opc::resolve_id(pkg, sheet_part, rid)?;
    Ok(Some((rid.to_string(), part)))
}

fn locate(
    pkg: &crate::opc::Package,
    root: &Element,
    sheet_part: &str,
    shape_id: &str,
) -> Result<Option<Found>, Error> {
    let Some((hf_rid, vml_part)) = legacy_vml(pkg, sheet_part, root)? else {
        return Ok(None);
    };
    let doc = list_pin::parse_part(pkg, &vml_part)?;
    let Some(shape) = find_shape(&doc.root, shape_id) else {
        return Ok(None);
    };
    let Some(image_rid) = shape_image_rid(shape) else {
        return Ok(None);
    };
    let media = crate::opc::resolve_id(pkg, &vml_part, image_rid)?;
    Ok(Some(Found {
        vml_part,
        hf_rid,
        image_rid: image_rid.to_string(),
        media,
    }))
}

fn write_media(
    pkg: &mut crate::opc::Package,
    found: &Found,
    shape_id: &str,
    image: &ImageFile,
) -> Result<(), Error> {
    let doc = list_pin::parse_part(pkg, &found.vml_part)?;
    let users = count_rid(&doc.root, &found.image_rid);
    let old_ext = found.media.rsplit('.').next().unwrap_or("");
    let same_ext = old_ext.eq_ignore_ascii_case(&image.ext);
    if users <= 1 && same_ext {
        pkg.set(&found.media, image.bytes.clone())?;
        return Ok(());
    }
    let media = picture::next_media_part(pkg, &image.ext);
    pkg.set(&media, image.bytes.clone())?;
    let mut cts = ContentTypes::from_package(pkg)?;
    cts.ensure_default(&image.ext, &image.content_type);
    cts.write_to(pkg)?;
    if users <= 1 {
        let mut rels = Relationships::from_package(pkg, &found.vml_part)?;
        if let Some(rel) = rels.find_by_id_mut(&found.image_rid) {
            rel.target = picture::rel_target_from(&found.vml_part, &media);
        }
        rels.write_to(pkg, &found.vml_part)?;
    } else {
        let rid = picture::add_image_rel(pkg, &found.vml_part, &media)?;
        let mut doc = doc;
        set_shape_rid(&mut doc.root, shape_id, &rid);
        list_pin::write_part(pkg, &found.vml_part, &doc)?;
    }
    if !picture::media_still_used(pkg, &found.media)? {
        pkg.remove(&found.media);
    }
    Ok(())
}

fn node_id(sheet: &str, slot: HfSlot, shape_id: &str) -> String {
    format!("{}/legacyPic:{shape_id}", slot.id(sheet))
}

fn slot_of(shape_id: &str) -> Option<HfSlot> {
    SHAPE_SLOTS
        .iter()
        .find(|(id, _)| *id == shape_id)
        .map(|(_, slot)| *slot)
}

fn local_name(key: &str) -> &str {
    key.rsplit(':').next().unwrap_or(key)
}

fn shape_image_rid(shape: &Element) -> Option<&str> {
    let image = find_imagedata(shape)?;
    image
        .attrs
        .iter()
        .find(|(k, _)| local_name(k) == "relid")
        .map(|(_, v)| v.as_str())
        .or_else(|| {
            image
                .attrs
                .iter()
                .find(|(k, _)| local_name(k) == "id")
                .map(|(_, v)| v.as_str())
        })
}

fn find_imagedata(el: &Element) -> Option<&Element> {
    if el.local_name() == "imagedata" {
        return Some(el);
    }
    for child in el.element_children() {
        if let Some(found) = find_imagedata(child) {
            return Some(found);
        }
    }
    None
}

fn find_shape<'a>(el: &'a Element, shape_id: &str) -> Option<&'a Element> {
    if el.local_name() == "shape" && el.attr_local("id") == Some(shape_id) {
        return Some(el);
    }
    for child in el.element_children() {
        if let Some(found) = find_shape(child, shape_id) {
            return Some(found);
        }
    }
    None
}

fn collect_shapes<'a>(el: &'a Element, out: &mut Vec<&'a Element>) {
    if el.local_name() == "shape" {
        out.push(el);
    }
    for child in el.element_children() {
        collect_shapes(child, out);
    }
}

fn count_rid(el: &Element, rid: &str) -> usize {
    let mut shapes = Vec::new();
    collect_shapes(el, &mut shapes);
    shapes
        .into_iter()
        .filter(|shape| shape_image_rid(shape) == Some(rid))
        .count()
}

fn any_shape(el: &Element) -> bool {
    if el.local_name() == "shape" {
        return true;
    }
    el.element_children().any(any_shape)
}

fn remove_shape(el: &mut Element, shape_id: &str) -> bool {
    let mut i = 0;
    while i < el.children.len() {
        let is_target = matches!(
            &el.children[i],
            Node::Element(child)
                if child.local_name() == "shape" && child.attr_local("id") == Some(shape_id)
        );
        if is_target {
            el.children.remove(i);
            return true;
        }
        if let Node::Element(child) = &mut el.children[i] {
            if remove_shape(child, shape_id) {
                return true;
            }
        }
        i += 1;
    }
    false
}

fn set_shape_rid(el: &mut Element, shape_id: &str, rid: &str) -> bool {
    if el.local_name() == "shape" && el.attr_local("id") == Some(shape_id) {
        return set_first_imagedata(el, rid);
    }
    for child in el.element_children_mut() {
        if set_shape_rid(child, shape_id, rid) {
            return true;
        }
    }
    false
}

fn set_first_imagedata(el: &mut Element, rid: &str) -> bool {
    if el.local_name() == "imagedata" {
        set_rid_attr(el, rid);
        return true;
    }
    for child in el.element_children_mut() {
        if set_first_imagedata(child, rid) {
            return true;
        }
    }
    false
}

fn set_rid_attr(el: &mut Element, rid: &str) {
    let key = el
        .attrs
        .iter()
        .find(|(k, _)| local_name(k) == "relid")
        .map(|(k, _)| k.clone())
        .or_else(|| {
            el.attrs
                .iter()
                .find(|(k, _)| local_name(k) == "id")
                .map(|(k, _)| k.clone())
        });
    match key {
        Some(k) => el.set_attr(&k, rid),
        None => el.set_attr("o:relid", rid),
    }
}
