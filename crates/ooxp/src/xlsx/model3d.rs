//! 工作表 3D 模型：`xdr:graphicFrame` 里的 `am3d:model3d`。
//! 不进 `pictures`。Fallback 海报不进 `pictures`。

use serde_json::{json, Map, Value};

use crate::error::{Error, ErrorKind, IdReason};
use crate::opc::{resolve_id, ContentTypes, Package, Relationship, Relationships};
use crate::xml::{self, Document, Element, Node};

use super::a1;
use super::picture::{self, AnchorKind, EditAs};
use super::shape;
use super::{ExportedMedia, REL_NS};

const MC_NS: &str = "http://schemas.openxmlformats.org/markup-compatibility/2006";

#[derive(Debug, Clone)]
pub enum Model3dKey {
    Id(String),
    Index(usize),
}

#[derive(Debug, Clone)]
pub struct Model3dTarget {
    pub sheet: String,
    pub key: Model3dKey,
}

#[derive(Debug, Clone)]
pub struct InsertModel3dOp {
    pub sheet: String,
    pub file: String,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
    pub from: a1::CellRef,
    pub to: Option<a1::CellRef>,
    pub anchor: AnchorKind,
    pub from_off_x: i64,
    pub from_off_y: i64,
    pub to_off_x: i64,
    pub to_off_y: i64,
}

#[derive(Debug, Clone)]
pub struct ReplaceModel3dOp {
    pub id: String,
    pub target: Model3dTarget,
    pub file: String,
}

#[derive(Debug, Clone)]
pub struct DeleteModel3dOp {
    pub id: String,
    pub target: Model3dTarget,
}

#[derive(Debug, Clone)]
pub struct SetModel3dSizeOp {
    pub id: String,
    pub target: Model3dTarget,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
    pub to: Option<a1::CellRef>,
    pub to_off_x: Option<i64>,
    pub to_off_y: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct SetModel3dAnchorOp {
    pub id: String,
    pub target: Model3dTarget,
    pub anchor: Option<AnchorKind>,
    pub from: Option<a1::CellRef>,
    pub to: Option<a1::CellRef>,
    pub from_off_x: Option<i64>,
    pub from_off_y: Option<i64>,
    pub to_off_x: Option<i64>,
    pub to_off_y: Option<i64>,
    pub pos_x: Option<i64>,
    pub pos_y: Option<i64>,
    pub edit_as: Option<EditAs>,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct SetModel3dCameraOp {
    pub id: String,
    pub target: Model3dTarget,
    pub patch: crate::model3d::Model3dCameraPatch,
}

pub struct Model3dInfo {
    pub id: String,
    pub cnv_pr_id: Option<String>,
    pub name: String,
    pub anchor: AnchorKind,
    pub from: Option<String>,
    pub from_off_x: Option<i64>,
    pub from_off_y: Option<i64>,
    pub to: Option<String>,
    pub to_off_x: Option<i64>,
    pub to_off_y: Option<i64>,
    pub edit_as: Option<EditAs>,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
    pub pos_x: Option<i64>,
    pub pos_y: Option<i64>,
    pub stable: bool,
    pub camera: Value,
    pub rot: Value,
    pub light_rig: Value,
}

pub fn looks_like_id(id: &str) -> bool {
    id.contains("/model3d:") && id.starts_with("sheet:")
}

pub fn parse_id(id: &str, json_path: &str) -> Result<Model3dTarget, Error> {
    let rest = id.strip_prefix("sheet:").ok_or_else(|| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SheetModel3dPrefix,
            },
            json_path.to_string(),
        )
    })?;
    let Some((name, after)) = rest.split_once("/model3d:") else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: if rest.contains('/') {
                    IdReason::SheetModel3dPrefix
                } else {
                    IdReason::SheetModel3dContainer
                },
            },
            json_path.to_string(),
        ));
    };
    if name.is_empty() || after.is_empty() {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SheetModel3dPrefix,
            },
            json_path.to_string(),
        ));
    }
    let key = parse_key(after, json_path)?;
    Ok(Model3dTarget {
        sheet: name.to_string(),
        key,
    })
}

fn parse_key(s: &str, path: &str) -> Result<Model3dKey, Error> {
    if let Some(rest) = s.strip_prefix("index:") {
        let n: usize = rest.parse().map_err(|_| {
            Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::SheetModel3dPrefix,
                },
                path.to_string(),
            )
        })?;
        if n < 1 {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::SheetModel3dPrefix,
                },
                path.to_string(),
            ));
        }
        return Ok(Model3dKey::Index(n));
    }
    if s.is_empty() || s.contains('/') {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SheetModel3dPrefix,
            },
            path.to_string(),
        ));
    }
    Ok(Model3dKey::Id(s.to_string()))
}

pub fn count(pkg: &Package, sheet_part: &str, sheet_root: &Element) -> Result<usize, Error> {
    Ok(collect(pkg, sheet_part, sheet_root, "")?.len())
}

pub fn collect(
    pkg: &Package,
    sheet_part: &str,
    sheet_root: &Element,
    sheet_name: &str,
) -> Result<Vec<Model3dInfo>, Error> {
    let mut out = Vec::new();
    let mut index = 0usize;
    for part in picture::drawing_parts_from(pkg, sheet_part, sheet_root)? {
        let doc = xml::parse(pkg.get(&part)?)?;
        picture::walk_anchors(&doc.root, &mut |anchor, kind| {
            let mut frames = Vec::new();
            collect_frames(anchor, &mut frames);
            for frame in frames {
                index += 1;
                out.push(info_of(
                    anchor,
                    kind,
                    frame,
                    &format!("sheet:{sheet_name}"),
                    index,
                )?);
            }
            Ok(())
        })?;
    }
    Ok(out)
}

fn collect_frames<'a>(el: &'a Element, out: &mut Vec<&'a Element>) {
    if el.local_name() == "AlternateContent" {
        if let Some(choice) = el.child_by_local("Choice") {
            if crate::model3d::choice_requires_am3d(choice) {
                collect_frames(choice, out);
            }
        }
        return;
    }
    if el.local_name() == "graphicFrame" && crate::model3d::is_model3d_graphic(el) {
        out.push(el);
        return;
    }
    for c in el.element_children() {
        collect_frames(c, out);
    }
}

fn info_of(
    anchor: &Element,
    kind: AnchorKind,
    frame: &Element,
    id_base: &str,
    index: usize,
) -> Result<Model3dInfo, Error> {
    let (raw, name) = cnv_of(frame);
    let stable = raw.is_some();
    let id = match &raw {
        Some(v) => format!("{id_base}/model3d:{v}"),
        None => format!("{id_base}/model3d:index:{index}"),
    };
    let (from, from_off_x, from_off_y) = picture::read_marker(anchor.child_by_local("from"))?;
    let (to, to_off_x, to_off_y) = picture::read_marker(anchor.child_by_local("to"))?;
    let (cx, cy) = picture::read_ext(anchor);
    let (pos_x, pos_y) = picture::read_pos(anchor);
    let edit_as = if kind == AnchorKind::TwoCell {
        Some(picture::parse_edit_as(anchor.attr_local("editAs")))
    } else {
        None
    };
    let (camera, rot, light_rig) = crate::model3d::inspect_view(frame);
    Ok(Model3dInfo {
        id,
        cnv_pr_id: raw,
        name,
        anchor: kind,
        from: from.map(|c| a1::format_cell(c.col, c.row)),
        from_off_x: if kind == AnchorKind::Absolute {
            None
        } else {
            Some(from_off_x)
        },
        from_off_y: if kind == AnchorKind::Absolute {
            None
        } else {
            Some(from_off_y)
        },
        to: to.map(|c| a1::format_cell(c.col, c.row)),
        to_off_x: if kind == AnchorKind::TwoCell {
            Some(to_off_x)
        } else {
            None
        },
        to_off_y: if kind == AnchorKind::TwoCell {
            Some(to_off_y)
        } else {
            None
        },
        edit_as,
        cx,
        cy,
        pos_x,
        pos_y,
        stable,
        camera,
        rot,
        light_rig,
    })
}

fn cnv_of(frame: &Element) -> (Option<String>, String) {
    let mut raw = None;
    let mut name = String::new();
    if let Some(nv) = frame.child_by_local("nvGraphicFramePr") {
        if let Some(pr) = nv.child_by_local("cNvPr") {
            raw = pr.attr_local("id").map(str::to_string);
            name = pr.attr_local("name").unwrap_or("").to_string();
        }
    }
    (raw, name)
}

pub fn list_json(items: &[Model3dInfo]) -> Vec<Value> {
    items.iter().map(info_json).collect()
}

fn info_json(p: &Model3dInfo) -> Value {
    let mut m = Map::new();
    m.insert("id".into(), json!(p.id));
    m.insert("cNvPrId".into(), json!(p.cnv_pr_id));
    m.insert("name".into(), json!(p.name));
    m.insert("anchor".into(), json!(p.anchor.as_str()));
    match p.anchor {
        AnchorKind::TwoCell => {
            m.insert("from".into(), json!(p.from));
            m.insert("fromOffX".into(), json!(p.from_off_x.unwrap_or(0)));
            m.insert("fromOffY".into(), json!(p.from_off_y.unwrap_or(0)));
            m.insert("to".into(), json!(p.to));
            m.insert("toOffX".into(), json!(p.to_off_x.unwrap_or(0)));
            m.insert("toOffY".into(), json!(p.to_off_y.unwrap_or(0)));
            m.insert(
                "editAs".into(),
                json!(p.edit_as.unwrap_or(EditAs::TwoCell).as_str()),
            );
        }
        AnchorKind::OneCell => {
            m.insert("from".into(), json!(p.from));
            m.insert("fromOffX".into(), json!(p.from_off_x.unwrap_or(0)));
            m.insert("fromOffY".into(), json!(p.from_off_y.unwrap_or(0)));
            m.insert("cx".into(), json!(p.cx));
            m.insert("cy".into(), json!(p.cy));
        }
        AnchorKind::Absolute => {
            m.insert("posX".into(), json!(p.pos_x));
            m.insert("posY".into(), json!(p.pos_y));
            m.insert("cx".into(), json!(p.cx));
            m.insert("cy".into(), json!(p.cy));
        }
        AnchorKind::RelSize | AnchorKind::AbsSize => {}
    }
    m.insert("contentType".into(), json!(crate::model3d::CT_GLB));
    m.insert("stable".into(), json!(p.stable));
    m.insert("camera".into(), p.camera.clone());
    m.insert("rot".into(), p.rot.clone());
    m.insert("lightRig".into(), p.light_rig.clone());
    Value::Object(m)
}

pub fn export(bytes: &[u8], id: &str) -> Result<ExportedMedia, Error> {
    let pkg = Package::open_bytes(bytes.to_vec())?;
    let target = parse_id(id, "id")?;
    let sheet_part = picture::lookup_sheet_part(&pkg, &target.sheet, "id")?;
    let sheet_doc = xml::parse(pkg.get(&sheet_part)?)?;
    let drawing_part = drawing_for(&pkg, &sheet_part, &sheet_doc.root, &target.key, id, "id")?;
    let doc = xml::parse(pkg.get(&drawing_part)?)?;
    let frame = find_frame(&doc.root, &target.key)
        .ok_or_else(|| Error::with_path(ErrorKind::Model3dNotFound { id: id.to_string() }, "id"))?;
    let rid = crate::model3d::glb_embed_rid(frame)
        .ok_or_else(|| Error::with_path(ErrorKind::Model3dNotFound { id: id.to_string() }, "id"))?;
    let media = resolve_id(&pkg, &drawing_part, &rid)?;
    let media_bytes = pkg.get(&media)?.to_vec();
    let cts = ContentTypes::from_package(&pkg)?;
    let content_type = cts.content_type_of(&media).map(str::to_string);
    Ok(ExportedMedia {
        bytes: media_bytes,
        content_type,
    })
}

fn drawing_for(
    pkg: &Package,
    sheet_part: &str,
    sheet_root: &Element,
    key: &Model3dKey,
    id: &str,
    path: &str,
) -> Result<String, Error> {
    for part in picture::drawing_parts_from(pkg, sheet_part, sheet_root)? {
        let doc = xml::parse(pkg.get(&part)?)?;
        if find_frame(&doc.root, key).is_some() {
            return Ok(part);
        }
    }
    Err(Error::with_path(
        ErrorKind::Model3dNotFound { id: id.to_string() },
        path.to_string(),
    ))
}

fn find_frame<'a>(root: &'a Element, key: &Model3dKey) -> Option<&'a Element> {
    let mut frames = Vec::new();
    collect_frames(root, &mut frames);
    for (i, frame) in frames.into_iter().enumerate() {
        if key_matches(key, &cnv_of(frame).0, i + 1) {
            return Some(frame);
        }
    }
    None
}

fn key_matches(key: &Model3dKey, raw: &Option<String>, index: usize) -> bool {
    match key {
        Model3dKey::Index(n) => index == *n,
        Model3dKey::Id(want) => {
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

pub fn insert(
    pkg: &mut Package,
    sheet_doc: &mut Document,
    sheet_part: &str,
    op: &InsertModel3dOp,
    file_field: &str,
) -> Result<(), Error> {
    let glb = crate::model3d::read_glb(&op.file).map_err(|e| e.at(file_field))?;
    let (cx, cy) = match (op.cx, op.cy) {
        (Some(x), Some(y)) => (x, y),
        _ => (crate::model3d::DEFAULT_CX, crate::model3d::DEFAULT_CY),
    };
    let drawing_part = picture::ensure_drawing_part(pkg, sheet_doc, sheet_part)?;
    let mut draw_doc = xml::parse(pkg.get(&drawing_part)?)?;
    picture::ensure_xmlns(&mut draw_doc.root, "r", REL_NS);
    picture::ensure_xmlns(&mut draw_doc.root, "a", picture::A_NS);
    picture::ensure_xmlns(&mut draw_doc.root, "mc", MC_NS);
    let cnv = picture::next_cnv_id(pkg, sheet_part, &sheet_doc.root)?;
    let (glb_rid, png_rid) = write_media(pkg, &drawing_part, glb)?;
    let name = format!("3D 模型 {cnv}");
    let ac = build_xlsx_ac(
        cnv,
        &name,
        &glb_rid,
        &png_rid,
        cx,
        cy,
        op.from_off_x,
        op.from_off_y,
    );
    let mut anchor = Element::new(format!("xdr:{}", op.anchor.local_name()));
    if op.anchor == AnchorKind::TwoCell {
        anchor.set_attr("editAs", EditAs::OneCell.as_str());
    }
    match op.anchor {
        AnchorKind::TwoCell => {
            anchor.push_element(picture::marker_el(
                "xdr:from",
                op.from,
                op.from_off_x,
                op.from_off_y,
            ));
            anchor.push_element(picture::marker_el(
                "xdr:to",
                op.to.unwrap_or(op.from),
                op.to_off_x,
                op.to_off_y,
            ));
        }
        AnchorKind::OneCell => {
            anchor.push_element(picture::marker_el(
                "xdr:from",
                op.from,
                op.from_off_x,
                op.from_off_y,
            ));
            anchor.push_element(
                Element::new("xdr:ext")
                    .with_attr("cx", cx.to_string())
                    .with_attr("cy", cy.to_string()),
            );
        }
        AnchorKind::Absolute => {
            return Err(Error::with_path(ErrorKind::InvalidXlsxAnchor, "ops.anchor"));
        }
        AnchorKind::RelSize | AnchorKind::AbsSize => {
            return Err(Error::with_path(ErrorKind::InvalidXlsxAnchor, "ops.anchor"));
        }
    }
    anchor.push_element(ac);
    anchor.push_element(Element::new("xdr:clientData"));
    draw_doc.root.push_element(anchor);
    pkg.set(&drawing_part, draw_doc.to_bytes()?)?;
    Ok(())
}

fn write_media(
    pkg: &mut Package,
    drawing_part: &str,
    glb: Vec<u8>,
) -> Result<(String, String), Error> {
    let glb_part = crate::ole::next_numbered(pkg, "/xl/media", "model3d", "glb");
    pkg.set(&glb_part, glb)?;
    let png_part = picture::next_media_part(pkg, "png");
    pkg.set(&png_part, crate::ole::PREVIEW_PNG.to_vec())?;
    let mut rels = Relationships::from_package(pkg, drawing_part)?;
    let glb_rid = rels.next_rid();
    rels.push(Relationship {
        id: glb_rid.clone(),
        rel_type: crate::model3d::REL_MODEL3D.into(),
        target: picture::rel_target_from(drawing_part, &glb_part),
        target_mode: None,
        extra: Vec::new(),
    });
    let png_rid = rels.next_rid();
    rels.push(Relationship {
        id: png_rid.clone(),
        rel_type: picture::REL_IMAGE.into(),
        target: picture::rel_target_from(drawing_part, &png_part),
        target_mode: None,
        extra: Vec::new(),
    });
    rels.write_to(pkg, drawing_part)?;
    let mut cts = ContentTypes::from_package(pkg)?;
    cts.ensure_default("glb", crate::model3d::CT_GLB);
    cts.ensure_default("png", "image/png");
    cts.write_to(pkg)?;
    Ok((glb_rid, png_rid))
}

fn build_xlsx_ac(
    cnv: u32,
    name: &str,
    glb_rid: &str,
    png_rid: &str,
    cx: i64,
    cy: i64,
    off_x: i64,
    off_y: i64,
) -> Element {
    let mut ac = Element::new("mc:AlternateContent").with_attr("xmlns:mc", MC_NS);
    let mut choice = Element::new("mc:Choice")
        .with_attr("xmlns:am3d", crate::model3d::AM3D_NS)
        .with_attr("Requires", "am3d");
    let mut frame = Element::new("xdr:graphicFrame").with_attr("macro", "");
    let mut nv = Element::new("xdr:nvGraphicFramePr");
    nv.push_element(
        Element::new("xdr:cNvPr")
            .with_attr("id", cnv.to_string())
            .with_attr("name", name),
    );
    let mut locks = Element::new("xdr:cNvGraphicFramePr");
    locks.push_element(Element::new("a:graphicFrameLocks"));
    nv.push_element(locks);
    frame.push_element(nv);
    let mut xfrm = Element::new("xdr:xfrm");
    xfrm.push_element(
        Element::new("a:off")
            .with_attr("x", "0")
            .with_attr("y", "0"),
    );
    xfrm.push_element(
        Element::new("a:ext")
            .with_attr("cx", "0")
            .with_attr("cy", "0"),
    );
    frame.push_element(xfrm);
    let mut graphic = Element::new("a:graphic");
    let mut data = Element::new("a:graphicData").with_attr("uri", crate::model3d::AM3D_NS);
    data.push_element(crate::model3d::build_model3d_el(glb_rid, png_rid, cx, cy));
    graphic.push_element(data);
    frame.push_element(graphic);
    choice.push_element(frame);
    ac.push_element(choice);

    let mut fallback = Element::new("mc:Fallback");
    fallback.push_element(build_fallback_pic(cnv, name, png_rid, cx, cy, off_x, off_y));
    ac.push_element(fallback);
    ac
}

fn build_fallback_pic(
    cnv: u32,
    name: &str,
    rid: &str,
    cx: i64,
    cy: i64,
    off_x: i64,
    off_y: i64,
) -> Element {
    let mut pic = Element::new("xdr:pic");
    let mut nv = Element::new("xdr:nvPicPr");
    nv.push_element(
        Element::new("xdr:cNvPr")
            .with_attr("id", cnv.to_string())
            .with_attr("name", name),
    );
    let mut cnv_pic = Element::new("xdr:cNvPicPr");
    cnv_pic.push_element(
        Element::new("a:picLocks")
            .with_attr("noGrp", "1")
            .with_attr("noRot", "1")
            .with_attr("noChangeAspect", "1")
            .with_attr("noMove", "1")
            .with_attr("noResize", "1")
            .with_attr("noEditPoints", "1")
            .with_attr("noAdjustHandles", "1")
            .with_attr("noChangeArrowheads", "1")
            .with_attr("noChangeShapeType", "1")
            .with_attr("noCrop", "1"),
    );
    nv.push_element(cnv_pic);
    pic.push_element(nv);
    let mut blip_fill = Element::new("xdr:blipFill");
    blip_fill.push_element(
        Element::new("a:blip")
            .with_attr("xmlns:r", REL_NS)
            .with_attr("r:embed", rid),
    );
    let mut stretch = Element::new("a:stretch");
    stretch.push_element(Element::new("a:fillRect"));
    blip_fill.push_element(stretch);
    pic.push_element(blip_fill);
    let mut sp_pr = Element::new("xdr:spPr");
    let mut xfrm = Element::new("a:xfrm");
    xfrm.push_element(
        Element::new("a:off")
            .with_attr("x", off_x.to_string())
            .with_attr("y", off_y.to_string()),
    );
    xfrm.push_element(
        Element::new("a:ext")
            .with_attr("cx", cx.to_string())
            .with_attr("cy", cy.to_string()),
    );
    sp_pr.push_element(xfrm);
    let mut geom = Element::new("a:prstGeom").with_attr("prst", "rect");
    geom.push_element(Element::new("a:avLst"));
    sp_pr.push_element(geom);
    pic.push_element(sp_pr);
    pic
}

pub fn replace(
    pkg: &mut Package,
    sheet_part: &str,
    op: &ReplaceModel3dOp,
    op_index: usize,
) -> Result<(), Error> {
    let path = format!("ops[{op_index}].id");
    let glb =
        crate::model3d::read_glb(&op.file).map_err(|e| e.at(format!("ops[{op_index}].file")))?;
    let sheet_doc = xml::parse(pkg.get(sheet_part)?)?;
    let drawing_part = drawing_for(
        pkg,
        sheet_part,
        &sheet_doc.root,
        &op.target.key,
        &op.id,
        &path,
    )?;
    let doc = xml::parse(pkg.get(&drawing_part)?)?;
    let frame = find_frame(&doc.root, &op.target.key).ok_or_else(|| {
        Error::with_path(
            ErrorKind::Model3dNotFound { id: op.id.clone() },
            path.clone(),
        )
    })?;
    let glb_rid = crate::model3d::glb_embed_rid(frame).ok_or_else(|| {
        Error::with_path(
            ErrorKind::Model3dNotFound { id: op.id.clone() },
            path.clone(),
        )
    })?;
    let png_rid = crate::model3d::raster_embed_rid(frame);
    let glb_part = resolve_id(pkg, &drawing_part, &glb_rid)?;
    pkg.set(&glb_part, glb)?;
    if let Some(rid) = png_rid {
        if let Ok(png_part) = resolve_id(pkg, &drawing_part, &rid) {
            pkg.set(&png_part, crate::ole::PREVIEW_PNG.to_vec())?;
        }
    }
    Ok(())
}

pub fn delete(
    pkg: &mut Package,
    sheet_doc: &mut Document,
    sheet_part: &str,
    op: &DeleteModel3dOp,
    op_index: usize,
) -> Result<(), Error> {
    let path = format!("ops[{op_index}].id");
    let drawing_part = drawing_for(
        pkg,
        sheet_part,
        &sheet_doc.root,
        &op.target.key,
        &op.id,
        &path,
    )?;
    let mut doc = xml::parse(pkg.get(&drawing_part)?)?;
    let rids = find_frame(&doc.root, &op.target.key)
        .map(crate::model3d::collect_embed_rids)
        .unwrap_or_default();
    if !remove_model3d(&mut doc.root, &op.target.key) {
        return Err(Error::with_path(
            ErrorKind::Model3dNotFound { id: op.id.clone() },
            path,
        ));
    }
    remove_empty_anchors(&mut doc.root);
    pkg.set(&drawing_part, doc.to_bytes()?)?;
    drop_rids(pkg, &drawing_part, &doc.root, &rids)?;
    Ok(())
}

fn draw_key(key: &Model3dKey) -> shape::DrawKey {
    match key {
        Model3dKey::Id(s) => shape::DrawKey::Cnv(s.clone()),
        Model3dKey::Index(n) => shape::DrawKey::Index(*n),
    }
}

pub fn set_size(
    pkg: &mut Package,
    sheet_root: &Element,
    sheet_part: &str,
    op: &SetModel3dSizeOp,
    op_index: usize,
) -> Result<(), Error> {
    shape::set_object_size(
        pkg,
        sheet_root,
        sheet_part,
        &draw_key(&op.target.key),
        crate::model3d::is_model3d_graphic,
        picture::SizePatch {
            cx: op.cx,
            cy: op.cy,
            to: op.to,
            to_off_x: op.to_off_x,
            to_off_y: op.to_off_y,
        },
        ErrorKind::TwoCellModel3dSize,
        ErrorKind::Model3dNotFound { id: op.id.clone() },
        "setModel3dSize",
        op_index,
    )
}

pub fn set_anchor(
    pkg: &mut Package,
    sheet_root: &Element,
    sheet_part: &str,
    op: &SetModel3dAnchorOp,
    op_index: usize,
) -> Result<(), Error> {
    shape::set_object_anchor(
        pkg,
        sheet_root,
        sheet_part,
        &draw_key(&op.target.key),
        crate::model3d::is_model3d_graphic,
        shape::AnchorWrite {
            op: "setModel3dAnchor",
            anchor: op.anchor,
            from: op.from,
            to: op.to,
            from_off_x: op.from_off_x,
            from_off_y: op.from_off_y,
            to_off_x: op.to_off_x,
            to_off_y: op.to_off_y,
            pos_x: op.pos_x,
            pos_y: op.pos_y,
            edit_as: op.edit_as,
            cx: op.cx,
            cy: op.cy,
            default_cx: crate::model3d::DEFAULT_CX,
            default_cy: crate::model3d::DEFAULT_CY,
            rel: None,
        },
        op_index,
        ErrorKind::Model3dNotFound { id: op.id.clone() },
    )
}

pub fn set_camera(
    pkg: &mut Package,
    sheet_root: &Element,
    sheet_part: &str,
    op: &SetModel3dCameraOp,
    op_index: usize,
) -> Result<(), Error> {
    let path = format!("ops[{op_index}].id");
    let drawing_part = drawing_for(pkg, sheet_part, sheet_root, &op.target.key, &op.id, &path)?;
    let mut doc = xml::parse(pkg.get(&drawing_part)?)?;
    let frame = find_frame_mut(&mut doc.root, &op.target.key).ok_or_else(|| {
        Error::with_path(
            ErrorKind::Model3dNotFound { id: op.id.clone() },
            path.clone(),
        )
    })?;
    let model = crate::model3d::model3d_in_mut(frame).ok_or_else(|| {
        Error::with_path(
            ErrorKind::Model3dNotFound { id: op.id.clone() },
            path.clone(),
        )
    })?;
    crate::model3d::apply_camera_patch(model, &op.patch)?;
    pkg.set(&drawing_part, doc.to_bytes()?)?;
    Ok(())
}

fn find_frame_mut<'a>(root: &'a mut Element, key: &Model3dKey) -> Option<&'a mut Element> {
    find_frame_walk_mut(root, key, &mut 0)
}

fn find_frame_walk_mut<'a>(
    el: &'a mut Element,
    key: &Model3dKey,
    index: &mut usize,
) -> Option<&'a mut Element> {
    crate::xml::search_mut(el, |el| {
        if el.local_name() == "AlternateContent" {
            let indexes: Vec<usize> = el
                .children
                .iter()
                .enumerate()
                .filter_map(|(i, n)| match n {
                    Node::Element(e)
                        if e.local_name() == "Choice"
                            && crate::model3d::choice_requires_am3d(e) =>
                    {
                        Some(i)
                    }
                    _ => None,
                })
                .collect();
            return crate::xml::SearchMut::Kids(indexes);
        }
        if el.local_name() == "graphicFrame" && crate::model3d::is_model3d_graphic(el) {
            *index += 1;
            if key_matches(key, &cnv_of(el).0, *index) {
                return crate::xml::SearchMut::Hit;
            }
            return crate::xml::SearchMut::Skip;
        }
        crate::xml::SearchMut::down(el)
    })
}

fn remove_model3d(el: &mut Element, key: &Model3dKey) -> bool {
    remove_in(el, key, &mut 0)
}

fn remove_in(el: &mut Element, key: &Model3dKey, index: &mut usize) -> bool {
    let mut drop = None;
    for (i, n) in el.children.iter_mut().enumerate() {
        let Node::Element(child) = n else { continue };
        if child.local_name() == "AlternateContent" {
            let mut frames = Vec::new();
            collect_frames(child, &mut frames);
            let mut hit = false;
            for frame in frames {
                *index += 1;
                if key_matches(key, &cnv_of(frame).0, *index) {
                    hit = true;
                }
            }
            if hit {
                drop = Some(i);
                break;
            }
        } else if child.local_name() == "graphicFrame" && crate::model3d::is_model3d_graphic(child)
        {
            *index += 1;
            if key_matches(key, &cnv_of(child).0, *index) {
                drop = Some(i);
                break;
            }
        } else if remove_in(child, key, index) {
            if AnchorKind::from_local(el.local_name()).is_some() && anchor_empty(el) {
                return true;
            }
            return true;
        }
    }
    if let Some(i) = drop {
        el.children.remove(i);
        if AnchorKind::from_local(el.local_name()).is_some() && anchor_empty(el) {
            // 调用方继续往上看，把空锚点删掉。
        }
        return true;
    }
    false
}

fn remove_empty_anchors(el: &mut Element) {
    el.children.retain(|n| {
        let Node::Element(child) = n else {
            return true;
        };
        !(AnchorKind::from_local(child.local_name()).is_some() && anchor_empty(child))
    });
    for n in el.children.iter_mut() {
        if let Node::Element(child) = n {
            remove_empty_anchors(child);
        }
    }
}

fn anchor_empty(el: &Element) -> bool {
    !el.element_children().any(|c| {
        let n = c.local_name();
        !matches!(n, "from" | "to" | "ext" | "pos" | "clientData")
    })
}

fn drop_rids(pkg: &mut Package, part: &str, root: &Element, rids: &[String]) -> Result<(), Error> {
    let mut rels = Relationships::from_package(pkg, part)?;
    let mut dirty = false;
    let mut media_parts = Vec::new();
    for rid in rids {
        if rid_still_used(root, rid) {
            continue;
        }
        if let Ok(m) = resolve_id(pkg, part, rid) {
            if !media_parts.iter().any(|x| x == &m) {
                media_parts.push(m);
            }
        }
        rels.remove_by_id(rid);
        dirty = true;
    }
    if dirty {
        if rels.rels.is_empty() {
            pkg.remove(&crate::opc::rels_part_name(part));
        } else {
            rels.write_to(pkg, part)?;
        }
    }
    for m in media_parts {
        if !picture::media_still_used(pkg, &m)? {
            pkg.remove(&m);
        }
    }
    Ok(())
}

fn rid_still_used(root: &Element, rid: &str) -> bool {
    let mut used = false;
    crate::model3d::walk(root, &mut |e| {
        if e.attr_local("embed") == Some(rid)
            || e.attr_local("id") == Some(rid) && e.local_name() != "cNvPr"
        {
            used = true;
        }
    });
    used
}
