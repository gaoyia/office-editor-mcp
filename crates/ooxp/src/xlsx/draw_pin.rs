//! apply 开始时给 drawing 里没有 `cNvPr/@id` 的对象编号。
//! 序号按 inspect 的顺序跨部件累加，写在元素上，本批不改写。

use crate::error::Error;
use crate::model3d;
use crate::opc::Package;
use crate::xml::{self, Document, Element, Node};

use super::chart;
use super::hf;
use super::list_pin::{self, DrawCount};
use super::picture::{self, has_blip};
use super::request::HfSlot;
use super::shape;
use super::slicer;
use super::smartart;

pub(crate) fn preload(
    pkg: &Package,
    sheets: &[(String, String, &'static str)],
) -> Result<(), Error> {
    for (_, part, kind) in sheets {
        if *kind != "worksheet" && *kind != "chartsheet" {
            continue;
        }
        let Ok(bytes) = pkg.get(part) else {
            continue;
        };
        let Ok(sheet) = xml::parse(bytes) else {
            continue;
        };
        let mut count = DrawCount::default();
        let parts = picture::drawing_parts_from(pkg, part, &sheet.root).unwrap_or_default();
        let user = super::chart_drawing::object_drawing_parts_from(pkg, part, &sheet.root)
            .unwrap_or_default();
        let mut ordered = parts;
        for extra in user {
            if !ordered.contains(&extra) {
                ordered.push(extra);
            }
        }
        for drawing in ordered {
            let Ok(bytes) = pkg.get(&drawing) else {
                continue;
            };
            let mut doc = xml::parse(bytes)?;
            pin_part(&mut doc, &mut count);
            list_pin::cache_put(&drawing, &doc);
        }
        let hf_el = sheet.root.child_by_local("headerFooter").cloned();
        let hf_parts = picture::drawing_hf_parts_from(pkg, part, &sheet.root).unwrap_or_default();
        let mut slots = [0usize; 6];
        for drawing in hf_parts {
            let Ok(bytes) = pkg.get(&drawing) else {
                continue;
            };
            let mut doc = xml::parse(bytes)?;
            pin_hf(&mut doc.root, hf_el.as_ref(), &mut slots);
            list_pin::cache_put(&drawing, &doc);
        }
        let _ = count.pic
            + count.chart
            + count.sp
            + count.shape
            + count.cxn
            + count.math
            + count.smart
            + count.model
            + count.ink
            + count.slicer;
    }
    Ok(())
}

fn pin_part(doc: &mut Document, count: &mut DrawCount) {
    let _ = picture::walk_anchors_mut(&mut doc.root, &mut |anchor, _| {
        if let Some(i) = direct_child(anchor, |el| el.local_name() == "pic" && has_blip(el)) {
            if let Node::Element(pic) = &mut anchor.children[i] {
                count.pic += 1;
                list_pin::stamp(pic, "pic", count.pic);
            }
        }
        if let Some(i) = direct_child(anchor, chart::is_chart_frame) {
            if let Node::Element(frame) = &mut anchor.children[i] {
                count.chart += 1;
                list_pin::stamp(frame, "chart", count.chart);
            }
        }
        pin_payloads(anchor, count);
        Ok(false)
    });
    pin_model3d(&mut doc.root, &mut count.model);
    pin_ink(&mut doc.root, &mut count.ink);
}

fn pin_hf(root: &mut Element, hf_el: Option<&Element>, slots: &mut [usize; 6]) {
    let _ = picture::walk_anchors_mut(root, &mut |anchor, _| {
        let Some(i) = direct_child(anchor, |el| el.local_name() == "pic" && has_blip(el)) else {
            return Ok(false);
        };
        let cnv = match &anchor.children[i] {
            Node::Element(pic) => picture::cnv_of(pic).0,
            _ => None,
        };
        let slot = hf::slot_for_cnv(hf_el, cnv.as_deref());
        let idx = slot_index(slot);
        slots[idx] += 1;
        let n = slots[idx];
        let family = slot_family(slot);
        if let Node::Element(pic) = &mut anchor.children[i] {
            list_pin::stamp(pic, family, n);
        }
        Ok(false)
    });
}

fn slot_index(slot: HfSlot) -> usize {
    if slot == HfSlot::ODD_HEADER {
        0
    } else if slot == HfSlot::ODD_FOOTER {
        1
    } else if slot == HfSlot::EVEN_HEADER {
        2
    } else if slot == HfSlot::EVEN_FOOTER {
        3
    } else if slot == HfSlot::FIRST_HEADER {
        4
    } else {
        5
    }
}

fn slot_family(slot: HfSlot) -> &'static str {
    if slot == HfSlot::ODD_HEADER {
        "pic-odd-header"
    } else if slot == HfSlot::ODD_FOOTER {
        "pic-odd-footer"
    } else if slot == HfSlot::EVEN_HEADER {
        "pic-even-header"
    } else if slot == HfSlot::EVEN_FOOTER {
        "pic-even-footer"
    } else if slot == HfSlot::FIRST_HEADER {
        "pic-first-header"
    } else {
        "pic-first-footer"
    }
}

fn direct_child(anchor: &Element, pred: impl Fn(&Element) -> bool) -> Option<usize> {
    anchor.children.iter().position(|n| match n {
        Node::Element(el) => pred(el),
        _ => false,
    })
}

fn pin_payloads(anchor: &mut Element, count: &mut DrawCount) {
    let kids = child_indexes(anchor);
    for i in kids {
        let local = match &anchor.children[i] {
            Node::Element(el) => el.local_name().to_string(),
            _ => continue,
        };
        if matches!(local.as_str(), "from" | "to" | "ext" | "pos" | "clientData") {
            continue;
        }
        pin_one(anchor, i, count);
    }
}

fn pin_one(parent: &mut Element, index: usize, count: &mut DrawCount) {
    let local = match &parent.children[index] {
        Node::Element(el) => el.local_name().to_string(),
        _ => return,
    };
    if local == "AlternateContent" {
        let math = matches!(&parent.children[index], Node::Element(el) if crate::drawing_math::is_math_ac(el));
        if math {
            if let Node::Element(el) = &mut parent.children[index] {
                count.math += 1;
                list_pin::stamp(el, "math", count.math);
            }
            return;
        }
        let choice = match &parent.children[index] {
            Node::Element(el) => xml::alternate_child(el).unwrap_or(None),
            _ => None,
        };
        let Some(choice) = choice else {
            return;
        };
        let kids = match &parent.children[index] {
            Node::Element(el) => match el.children.get(choice) {
                Some(Node::Element(child)) => child_indexes(child),
                _ => Vec::new(),
            },
            _ => Vec::new(),
        };
        if let Node::Element(ac) = &mut parent.children[index] {
            if let Node::Element(choice_el) = &mut ac.children[choice] {
                for k in kids {
                    pin_one(choice_el, k, count);
                }
            }
        }
        return;
    }
    if local == "grpSp" {
        let kids = match &parent.children[index] {
            Node::Element(el) => child_indexes(el),
            _ => Vec::new(),
        };
        if let Node::Element(group) = &mut parent.children[index] {
            for k in kids {
                let child_local = match &group.children[k] {
                    Node::Element(el) => el.local_name().to_string(),
                    _ => continue,
                };
                if matches!(child_local.as_str(), "nvGrpSpPr" | "grpSpPr") {
                    continue;
                }
                pin_one(group, k, count);
            }
        }
        return;
    }
    if let Node::Element(el) = &mut parent.children[index] {
        stamp_payload(el, count);
    }
}

fn stamp_payload(el: &mut Element, count: &mut DrawCount) {
    if smartart::is_diagram_frame(el) {
        count.smart += 1;
        list_pin::stamp(el, "smartdiagram", count.smart);
    } else if slicer::is_slicer_or_timeline_frame(el) {
        count.slicer += 1;
        list_pin::stamp(el, "slicer", count.slicer);
    } else if shape::is_connector(el) {
        count.cxn += 1;
        list_pin::stamp(el, "cxn", count.cxn);
    } else if shape::is_textbox(el) {
        count.sp += 1;
        list_pin::stamp(el, "sp", count.sp);
    } else if shape::is_shape(el) {
        count.shape += 1;
        list_pin::stamp(el, "shape", count.shape);
    }
}

fn pin_model3d(el: &mut Element, n: &mut usize) {
    if el.local_name() == "AlternateContent" {
        let ok = el
            .child_by_local("Choice")
            .is_some_and(model3d::choice_requires_am3d);
        if ok {
            if let Some(choice) = el.child_by_local_mut("Choice") {
                pin_model3d(choice, n);
            }
        }
        return;
    }
    if el.local_name() == "graphicFrame" && model3d::is_model3d_graphic(el) {
        *n += 1;
        list_pin::stamp(el, "model3d", *n);
        return;
    }
    let kids = child_indexes(el);
    for i in kids {
        if let Node::Element(child) = &mut el.children[i] {
            pin_model3d(child, n);
        }
    }
}

fn pin_ink(el: &mut Element, n: &mut usize) {
    if el.local_name() == "AlternateContent" {
        let choice_i = el.children.iter().position(|node| {
            matches!(node, Node::Element(c) if c.local_name() == "Choice" && choice_is_ink(c))
        });
        if let Some(choice_i) = choice_i {
            let parts = match &el.children[choice_i] {
                Node::Element(choice) => choice
                    .children
                    .iter()
                    .enumerate()
                    .filter_map(|(i, node)| {
                        matches!(node, Node::Element(c) if c.local_name() == "contentPart")
                            .then_some(i)
                    })
                    .collect::<Vec<_>>(),
                _ => Vec::new(),
            };
            if let Node::Element(choice) = &mut el.children[choice_i] {
                for i in parts {
                    if let Node::Element(part) = &mut choice.children[i] {
                        *n += 1;
                        list_pin::stamp(part, "ink", *n);
                    }
                }
            }
        }
        return;
    }
    let kids = child_indexes(el);
    for i in kids {
        if let Node::Element(child) = &mut el.children[i] {
            pin_ink(child, n);
        }
    }
}

fn choice_is_ink(choice: &Element) -> bool {
    let req = choice.attr_local("Requires").unwrap_or("");
    req.split_whitespace().any(|t| t == "xdr14")
        && choice
            .element_children()
            .any(|c| c.local_name() == "contentPart")
}

fn child_indexes(el: &Element) -> Vec<usize> {
    el.children
        .iter()
        .enumerate()
        .filter_map(|(i, node)| matches!(node, Node::Element(_)).then_some(i))
        .collect()
}
