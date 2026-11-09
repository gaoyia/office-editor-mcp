//! 幻灯片、备注页、母版和版式的 `p:bg`。不引用 docx。

use crate::error::{Error, ErrorKind};
use crate::opc::Package;
use crate::xml::{Document, Element, Node};

use super::picture;
use super::request::{PageFill, SetPageBackgroundOp, ShapePaint};
use super::shape::SlideMeta;
use super::{DML_NS, REL_NS};

pub(crate) fn op_set_page_background(
    pkg: &mut Package,
    pres_part: &str,
    pres: &mut Document,
    slides: &[SlideMeta],
    op: &SetPageBackgroundOp,
    i: usize,
) -> Result<(), Error> {
    let (slide, on_notes, master) = op.host.parts();
    let part = super::apply::ensure_host_part(
        pkg,
        pres_part,
        pres,
        slides,
        slide,
        on_notes,
        master,
        format!("ops[{i}].id"),
    )?;
    let mut doc = super::list_pin::parse_part(pkg, &part)?;
    picture::ensure_xmlns(&mut doc.root, "a", DML_NS);
    if op.color.is_none() {
        remove_bg(&mut doc.root);
    } else {
        if matches!(op.fill, Some(PageFill::Picture { .. })) {
            picture::ensure_xmlns(&mut doc.root, "r", REL_NS);
        }
        let bg = build_bg(pkg, &part, op)?;
        install_bg(&mut doc.root, bg);
    }
    super::list_pin::write_part(pkg, &part, &doc)
}

fn remove_bg(root: &mut Element) {
    let Some(c_sld) = root.child_by_local_mut("cSld") else {
        return;
    };
    c_sld
        .children
        .retain(|n| !matches!(n, Node::Element(e) if e.local_name() == "bg"));
}

fn install_bg(root: &mut Element, bg: Element) {
    let Some(c_sld) = root.child_by_local_mut("cSld") else {
        return;
    };
    c_sld
        .children
        .retain(|n| !matches!(n, Node::Element(e) if e.local_name() == "bg"));
    let at = c_sld
        .children
        .iter()
        .position(|n| matches!(n, Node::Element(e) if e.local_name() == "spTree"))
        .unwrap_or(c_sld.children.len());
    c_sld.children.insert(at, Node::Element(bg));
}

fn build_bg(pkg: &mut Package, part: &str, op: &SetPageBackgroundOp) -> Result<Element, Error> {
    let color = op
        .color
        .clone()
        .ok_or_else(|| Error::new(ErrorKind::InvalidPageBackgroundColor))?;
    let paint = match &op.fill {
        None => ShapePaint::Solid {
            color,
            alpha: op.alpha,
        },
        Some(PageFill::Gradient { color2 }) => ShapePaint::Grad {
            ang: crate::drawing_text::ANG_DEFAULT,
            stops: vec![
                (0, color),
                (
                    crate::drawing_text::POS_MAX,
                    color2.clone().unwrap_or_else(|| "FFFFFF".into()),
                ),
            ],
            alpha: op.alpha,
        },
        Some(PageFill::Picture { file }) => ShapePaint::Blip { file: file.clone() },
    };
    let mut bg = Element::new("p:bg");
    let mut pr = Element::new("p:bgPr");
    pr.push_element(super::shape::paint_for_page(pkg, part, &paint)?);
    pr.push_element(Element::new("a:effectLst"));
    bg.push_element(pr);
    Ok(bg)
}
