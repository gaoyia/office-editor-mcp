//! 工作表 `headerFooter` 与 `drawingHF`：inspect、改字、往槽位插 DrawingML 图。

use serde_json::{json, Map, Value};

use crate::error::{Error, ErrorKind, IdReason};
use crate::opc::{ContentTypes, Package, Relationship, Relationships};
use crate::xml::{self, Document, Element, Node};

use super::a1;
use super::picture::{
    self, build_anchor, drawing_hf_parts, drawing_hf_parts_from, ensure_xmlns, next_cnv_id,
    next_drawing_part, pictures_json, read_image, rel_target_from, walk_anchors, AnchorKind,
    EditAs, PicInfo, A_NS, CT_DRAWING, REL_DRAWING, XDR_NS,
};
use super::request::{HfSlot, InsertPictureOp, SetHeaderFooterTextOp};
use super::REL_NS;

const SLOT_XML: [(&str, HfSlot); 6] = [
    ("oddHeader", HfSlot::ODD_HEADER),
    ("oddFooter", HfSlot::ODD_FOOTER),
    ("evenHeader", HfSlot::EVEN_HEADER),
    ("evenFooter", HfSlot::EVEN_FOOTER),
    ("firstHeader", HfSlot::FIRST_HEADER),
    ("firstFooter", HfSlot::FIRST_FOOTER),
];

const ATTR_TO_SLOT: [(&str, HfSlot); 18] = [
    ("cho", HfSlot::ODD_HEADER),
    ("lho", HfSlot::ODD_HEADER),
    ("rho", HfSlot::ODD_HEADER),
    ("che", HfSlot::EVEN_HEADER),
    ("lhe", HfSlot::EVEN_HEADER),
    ("rhe", HfSlot::EVEN_HEADER),
    ("chf", HfSlot::FIRST_HEADER),
    ("lhf", HfSlot::FIRST_HEADER),
    ("rhf", HfSlot::FIRST_HEADER),
    ("cfo", HfSlot::ODD_FOOTER),
    ("lfo", HfSlot::ODD_FOOTER),
    ("rfo", HfSlot::ODD_FOOTER),
    ("cfe", HfSlot::EVEN_FOOTER),
    ("lfe", HfSlot::EVEN_FOOTER),
    ("rfe", HfSlot::EVEN_FOOTER),
    ("cff", HfSlot::FIRST_FOOTER),
    ("lff", HfSlot::FIRST_FOOTER),
    ("rff", HfSlot::FIRST_FOOTER),
];

/// 解析 `odd:header` 这类槽位片段。必须恰好两段、全小写。
pub(crate) fn parse_slot(s: &str) -> Option<HfSlot> {
    let (kind, side) = s.split_once(':')?;
    if kind.contains('/') || side.contains('/') || side.contains(':') {
        return None;
    }
    HfSlot::parse(kind, side)
}

pub(crate) fn parse_slot_id(id: &str, op_index: usize) -> Result<(String, HfSlot), Error> {
    let path = format!("ops[{op_index}].id");
    if id.contains("/pic:") {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::HfSlotPrefix,
            },
            path,
        ));
    }
    let Some(rest) = id.strip_prefix("sheet:") else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::HfSlotPrefix,
            },
            path,
        ));
    };
    let Some((sheet, slot_s)) = rest.split_once("/hf:") else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::HfSlotPrefix,
            },
            path,
        ));
    };
    if sheet.is_empty() {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::HfSlotPrefix,
            },
            path,
        ));
    }
    let Some(slot) = parse_slot(slot_s) else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::HfSlotPrefix,
            },
            path,
        ));
    };
    Ok((sheet.to_string(), slot))
}

/// 按 `drawingHF` 属性把 `cNvPr` id 分到槽；对不上归奇数页页眉。
pub(crate) fn slot_for_cnv(hf_el: Option<&Element>, cnv: Option<&str>) -> HfSlot {
    let Some(el) = hf_el else {
        return HfSlot::ODD_HEADER;
    };
    let Some(cnv) = cnv else {
        return HfSlot::ODD_HEADER;
    };
    for (attr, slot) in ATTR_TO_SLOT {
        if let Some(v) = el.attr(attr) {
            if cnv_eq(v, cnv) {
                return slot;
            }
        }
    }
    HfSlot::ODD_HEADER
}

fn cnv_eq(a: &str, b: &str) -> bool {
    match (a.parse::<u32>(), b.parse::<u32>()) {
        (Ok(x), Ok(y)) => x == y,
        _ => a == b,
    }
}

pub(crate) fn has_header_footer(root: &Element) -> bool {
    let Some(hf) = root.child_by_local("headerFooter") else {
        return false;
    };
    SLOT_XML.iter().any(|(local, _)| {
        hf.child_by_local(local)
            .is_some_and(|e| !e.text_content().is_empty())
    })
}

pub(crate) fn inspect_slots(
    pkg: &Package,
    sheet_part: &str,
    sheet_name: &str,
    root: &Element,
    sheet_kind: &str,
    want_pictures: bool,
) -> Result<Vec<Value>, Error> {
    let hf = root.child_by_local("headerFooter");
    let hf_el = root.child_by_local("drawingHF");
    let mut grouped: Vec<(HfSlot, Vec<PicInfo>)> =
        HfSlot::ALL.iter().map(|s| (*s, Vec::new())).collect();
    if sheet_kind == "worksheet" {
        for pic in collect_hf_pictures(pkg, sheet_part, sheet_name, hf_el)? {
            if let Some((_, bucket)) = grouped.iter_mut().find(|(s, _)| *s == pic.0) {
                bucket.push(pic.1);
            }
        }
    }
    let mut out = Vec::new();
    for (i, slot) in HfSlot::ALL.iter().enumerate() {
        let local = SLOT_XML[i].0;
        let text = hf
            .and_then(|h| h.child_by_local(local))
            .map(|e| json!(e.text_content()))
            .unwrap_or(Value::Null);
        let mut m = Map::new();
        m.insert("id".into(), json!(slot.id(sheet_name)));
        m.insert("kind".into(), json!(slot.kind_str()));
        m.insert("side".into(), json!(slot.side_str()));
        m.insert("text".into(), text);
        if sheet_kind == "worksheet" {
            let pics = &grouped[i].1;
            m.insert("pictureCount".into(), json!(pics.len()));
            if want_pictures {
                m.insert("pictures".into(), Value::Array(pictures_json(pics)));
            }
        }
        out.push(Value::Object(m));
    }
    Ok(out)
}

fn collect_hf_pictures(
    pkg: &Package,
    sheet_part: &str,
    sheet_name: &str,
    hf_el: Option<&Element>,
) -> Result<Vec<(HfSlot, PicInfo)>, Error> {
    let mut raw = Vec::new();
    for drawing_part in drawing_hf_parts(pkg, sheet_part)? {
        let doc = xml::parse(pkg.get(&drawing_part)?)?;
        walk_anchors(&doc.root, &mut |anchor, kind| {
            let Some(pic) = picture::direct_pic(anchor) else {
                return Ok(());
            };
            if !picture::has_blip(pic) {
                return Ok(());
            }
            let (cnv, _) = picture::cnv_of(pic);
            let slot = slot_for_cnv(hf_el, cnv.as_deref());
            raw.push((slot, anchor.clone(), kind, pic.clone()));
            Ok(())
        })?;
    }
    let mut counts = [0usize; 6];
    let mut out = Vec::new();
    for (slot, anchor, kind, pic) in raw {
        let i = slot.index();
        counts[i] += 1;
        let id_base = slot.id(sheet_name);
        let info = picture::pic_info_at(&anchor, kind, &pic, &id_base, counts[i])?;
        out.push((slot, info));
    }
    Ok(out)
}

/// create 写 even / first 槽位时补属性；没有原文时也要有空的 `headerFooter`。
pub(crate) fn ensure_create_slot_flags(sheet: &mut Element, slot: HfSlot) {
    let hf = ensure_header_footer(sheet);
    match slot.kind_str() {
        "even" => hf.set_attr("differentOddEven", "1"),
        "first" => hf.set_attr("differentFirst", "1"),
        _ => {}
    }
}

pub(crate) fn set_text(sheet: &mut Element, op: &SetHeaderFooterTextOp) {
    let hf = ensure_header_footer(sheet);
    match op.slot.kind_str() {
        "even" => hf.set_attr("differentOddEven", "1"),
        "first" => hf.set_attr("differentFirst", "1"),
        _ => {}
    }
    let local = op.slot.xml_local();
    if op.text.is_empty() {
        hf.children.retain(|n| match n {
            Node::Element(e) => e.local_name() != local,
            _ => true,
        });
        return;
    }
    if let Some(el) = hf.child_by_local_mut(local) {
        write_slot_text(el, &op.text);
        return;
    }
    let mut el = Element::new(local);
    write_slot_text(&mut el, &op.text);
    insert_slot_child(hf, el, op.slot);
}

fn write_slot_text(el: &mut Element, text: &str) {
    el.set_text(text);
    el.set_attr("xml:space", "preserve");
}

fn ensure_header_footer(sheet: &mut Element) -> &mut Element {
    if sheet.child_by_local("headerFooter").is_none() {
        insert_sheet_el(
            sheet,
            Element::new("headerFooter"),
            &[
                "rowBreaks",
                "colBreaks",
                "customProperties",
                "drawing",
                "drawingHF",
                "legacyDrawing",
                "legacyDrawingHF",
                "picture",
                "oleObjects",
                "controls",
                "webPublishItems",
                "tableParts",
                "extLst",
            ],
        );
    }
    sheet.child_by_local_mut("headerFooter").unwrap()
}

fn insert_slot_child(hf: &mut Element, el: Element, slot: HfSlot) {
    let order: [&str; 6] = SLOT_XML.map(|(n, _)| n);
    let want = slot.xml_local();
    let want_i = order.iter().position(|n| *n == want).unwrap();
    let mut at = hf.children.len();
    for (i, n) in hf.children.iter().enumerate() {
        let Node::Element(e) = n else { continue };
        if let Some(pos) = order.iter().position(|n| *n == e.local_name()) {
            if pos > want_i {
                at = i;
                break;
            }
        }
    }
    hf.children.insert(at, Node::Element(el));
}

fn insert_sheet_el(root: &mut Element, el: Element, after: &[&str]) {
    let mut at = root.children.len();
    for (i, n) in root.children.iter().enumerate() {
        let Node::Element(e) = n else { continue };
        if after.contains(&e.local_name()) {
            at = i;
            break;
        }
    }
    root.children.insert(at, Node::Element(el));
}

pub(crate) fn insert_picture(
    pkg: &mut Package,
    sheet_doc: &mut Document,
    sheet_part: &str,
    op: &InsertPictureOp,
    file_field: &str,
) -> Result<(), Error> {
    let slot = op.hf.expect("insert_picture 只处理页眉槽位");
    let image = read_image(&op.file).map_err(|e| e.at(file_field))?;
    let (cx, cy) = match (op.cx, op.cy) {
        (Some(x), Some(y)) => (x, y),
        _ => (image.cx, image.cy),
    };
    let drawing_part = ensure_drawing_hf_part(pkg, sheet_doc, sheet_part)?;
    let mut draw_doc = xml::parse(pkg.get(&drawing_part)?)?;
    ensure_xmlns(&mut draw_doc.root, "r", REL_NS);
    ensure_xmlns(&mut draw_doc.root, "a", A_NS);
    // Excel 16.0：drawingHF 的 cho 等属性若写成 1，打开工作簿会被拒绝。
    let cnv = next_cnv_id(pkg, sheet_part, &sheet_doc.root)?.max(2);
    let (rid, svg_rid) = picture::write_picture_media(pkg, &drawing_part, &image)?;
    let mut cts = ContentTypes::from_package(pkg)?;
    cts.set_override(&drawing_part, CT_DRAWING)?;
    cts.write_to(pkg)?;
    let name = format!("图片 {cnv}");
    let from = a1::CellRef { col: 1, row: 1 };
    let mut anchor = build_anchor(
        AnchorKind::OneCell,
        from,
        None,
        0,
        0,
        0,
        0,
        None,
        None,
        cx,
        cy,
        EditAs::TwoCell,
        cnv,
        &name,
        &rid,
        None,
    );
    if let Some(svg) = svg_rid {
        crate::svg::set_svg_blip(&mut anchor, &svg);
    }
    draw_doc.root.push_element(anchor);
    pkg.set(&drawing_part, draw_doc.to_bytes()?)?;
    assign_hf_attr(&mut sheet_doc.root, slot, cnv);
    Ok(())
}

fn assign_hf_attr(sheet: &mut Element, slot: HfSlot, cnv: u32) {
    let Some(el) = sheet.child_by_local_mut("drawingHF") else {
        return;
    };
    let id = cnv.to_string();
    for attr in slot.section_attrs() {
        if el.attr(attr).is_none() {
            el.set_attr(attr, id);
            return;
        }
    }
}

fn ensure_drawing_hf_part(
    pkg: &mut Package,
    sheet_doc: &mut Document,
    sheet_part: &str,
) -> Result<String, Error> {
    if let Some(part) = drawing_hf_parts_from(pkg, sheet_part, &sheet_doc.root)?
        .into_iter()
        .next()
    {
        return Ok(part);
    }
    let part = next_drawing_part(pkg);
    let root = Element::new("xdr:wsDr")
        .with_attr("xmlns:xdr", XDR_NS)
        .with_attr("xmlns:a", A_NS)
        .with_attr("xmlns:r", REL_NS);
    let doc = Document {
        decl: Some(crate::xml::DEFAULT_DECL.to_string()),
        root,
    };
    pkg.set(&part, doc.to_bytes()?)?;
    let mut rels = Relationships::from_package(pkg, sheet_part)?;
    let rid = rels.next_rid();
    rels.push(Relationship {
        id: rid.clone(),
        rel_type: REL_DRAWING.into(),
        target: rel_target_from(sheet_part, &part),
        target_mode: None,
        extra: Vec::new(),
    });
    rels.write_to(pkg, sheet_part)?;
    ensure_xmlns(&mut sheet_doc.root, "r", REL_NS);
    insert_drawing_hf_el(&mut sheet_doc.root, &rid);
    let mut cts = ContentTypes::from_package(pkg)?;
    cts.set_override(&part, CT_DRAWING)?;
    cts.write_to(pkg)?;
    Ok(part)
}

fn insert_drawing_hf_el(worksheet: &mut Element, rid: &str) {
    if worksheet
        .element_children()
        .any(|e| e.local_name() == "drawingHF")
    {
        return;
    }
    insert_sheet_el(
        worksheet,
        Element::new("drawingHF").with_attr("r:id", rid),
        &[
            "legacyDrawing",
            "legacyDrawingHF",
            "picture",
            "oleObjects",
            "controls",
            "webPublishItems",
            "tableParts",
            "extLst",
        ],
    );
}

pub(crate) fn clear_cnv_attrs(sheet: &mut Element, cnv: &str) {
    let Some(el) = sheet.child_by_local_mut("drawingHF") else {
        return;
    };
    let drop: Vec<String> = el
        .attrs
        .iter()
        .filter(|(k, v)| ATTR_TO_SLOT.iter().any(|(a, _)| *a == k.as_str()) && cnv_eq(v, cnv))
        .map(|(k, _)| k.clone())
        .collect();
    for k in drop {
        el.remove_attr(&k);
    }
}

pub(crate) fn remove_drawing_hf_ref(
    worksheet: &mut Element,
    sheet_part: &str,
    pkg: &mut Package,
    drawing_part: &str,
) -> Result<(), Error> {
    worksheet.children.retain(|n| match n {
        Node::Element(e) => e.local_name() != "drawingHF",
        _ => true,
    });
    let mut rels = Relationships::from_package(pkg, sheet_part)?;
    rels.remove_by_target_part(sheet_part, drawing_part);
    rels.write_to(pkg, sheet_part)?;
    Ok(())
}
