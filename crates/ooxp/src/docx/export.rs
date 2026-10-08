//! 按图 id 导出媒体部件原字节。

use crate::error::{Error, ErrorKind, XmlIssue};
use crate::opc::{self, ContentTypes, Package};
use crate::xml;

use super::hf;
use super::picture;

pub struct ExportedMedia {
    pub bytes: Vec<u8>,
    pub content_type: Option<String>,
}

pub fn export_media(bytes: &[u8], id: &str) -> Result<ExportedMedia, Error> {
    if id.starts_with("font:") {
        return super::font_embed::export_media(bytes, id);
    }
    if id.contains("/ole:") {
        return super::ole::export_media(bytes, id);
    }
    let pkg = Package::open_bytes(bytes.to_vec())?;
    let target = picture::parse_pic_id_at(id, "id")?;
    let doc_part = opc::office_document_part(&pkg)?;
    let doc_xml = xml::parse(pkg.get(&doc_part)?)?;
    let catalog = hf::catalog(&pkg, &doc_part, &doc_xml.root)?;
    let part_name = picture::part_for_pic(&pkg, &catalog, &doc_part, &target, id, "id")?;
    let xml_doc = if part_name == doc_part {
        doc_xml
    } else {
        xml::parse(pkg.get(&part_name)?)?
    };
    let drawing = picture::find_target_drawing(&xml_doc.root, &target)
        .ok_or_else(|| Error::with_path(ErrorKind::PictureNotFound { id: id.to_string() }, "id"))?;
    let rid = picture::blip_rid(drawing).ok_or_else(|| {
        Error::xml_detail(
            XmlIssue::Parse,
            format!("{id} missing a:blip/@r:embed or @r:link"),
        )
    })?;
    let media = opc::resolve_id(&pkg, &part_name, &rid)?;
    let media_bytes = pkg.get(&media)?.to_vec();
    let cts = ContentTypes::from_package(&pkg)?;
    let content_type = cts.content_type_of(&media).map(str::to_string);
    Ok(ExportedMedia {
        bytes: media_bytes,
        content_type,
    })
}
