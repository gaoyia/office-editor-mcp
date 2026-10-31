//! 段内位置 id 在第一次装载部件时编号。
//! 故事级名单跳过文本框，文本框里再从 1 起。
//! 页眉页脚里的图、SmartArt、OLE 和页眉水印在整份部件里从 1 起，其余绘图退路在各段里从 1 起。

use crate::opc::Package;
use crate::xml::{Element, Node};

use super::para;

pub(crate) enum PinStep {
    Skip,
    Member,
    Descend,
}

pub(crate) fn pin_walk(
    el: &mut Element,
    n: &mut usize,
    family: &str,
    step: &impl Fn(&Element) -> PinStep,
) {
    if el.local_name() == "AlternateContent" {
        if let Some(i) = para::first_alt_child_index(el) {
            if let Node::Element(child) = &mut el.children[i] {
                pin_walk(child, n, family, step);
            }
        }
        return;
    }
    match step(el) {
        PinStep::Skip => {}
        PinStep::Member => {
            *n += 1;
            let attr = crate::xml::pin_attr(family);
            if el.attr(&attr).is_none() {
                el.set_attr(&attr, n.to_string());
            }
        }
        PinStep::Descend => {
            let len = el.children.len();
            for i in 0..len {
                if let Node::Element(child) = &mut el.children[i] {
                    pin_walk(child, n, family, step);
                }
            }
        }
    }
}

pub(crate) fn pin_inline_indexes(pkg: &Package, part: &str, root: &mut Element) {
    pin_stories(root);
    pin_sections(root);
    pin_runs(root);
    if matches!(root.local_name(), "hdr" | "ftr") {
        super::picture::pin_scope(root);
        super::smartart::pin_scope(root);
        super::ole::pin_scope(pkg, part, root);
    }
    pin_per_paragraph(root, super::picture::pin_scope);
    pin_per_paragraph(root, super::smartart::pin_scope);
    super::ole::pin_paragraphs(pkg, part, root);
    pin_per_paragraph(root, super::charts::pin_scope);
    pin_per_paragraph(root, super::shape::pin_scope);
    pin_per_paragraph(root, super::connector::pin_scope);
    pin_per_paragraph(root, super::textbox::pin_scope);
    super::ink::pin_paragraphs(pkg, part, root);
    pin_per_paragraph(root, super::model3d::pin_scope);
    super::watermark::pin_scope(root);
}

fn pin_stories(root: &mut Element) {
    pin_one_story(root);
    let boxes = txbx_paths(root);
    for path in boxes {
        if let Some(el) = para::element_at_mut(root, &path) {
            pin_one_story(el);
        }
    }
}

fn pin_one_story(root: &mut Element) {
    super::link::pin_bookmarks(root);
    super::link::pin_hyperlinks(root);
    super::field::pin_fields(root);
    super::math::pin_maths(root);
    super::ruby::pin_rubies(root);
    super::sdt::pin_sdts(root);
    super::perm::pin_perms(root);
    super::revision::pin_revs(root);
    super::field::pin_doc_lists(root);
}

pub(crate) fn pin_sections(root: &mut Element) {
    super::hf::pin_sections(root);
}

fn pin_runs(root: &mut Element) {
    let paths = paragraph_paths(root);
    for path in paths {
        if let Some(p) = para::element_at_mut(root, &path) {
            super::format::pin_runs_in_paragraph(p);
        }
    }
}

fn pin_per_paragraph(root: &mut Element, pin_scope: fn(&mut Element)) {
    for_each_paragraph(root, &mut |p| pin_scope(p));
}

pub(crate) fn for_each_paragraph(root: &mut Element, f: &mut impl FnMut(&mut Element)) {
    let paths = paragraph_paths(root);
    for path in paths {
        if let Some(p) = para::element_at_mut(root, &path) {
            f(p);
        }
    }
}

fn paragraph_paths(root: &Element) -> Vec<Vec<usize>> {
    let mut out = Vec::new();
    walk_paths(root, &[], &mut out, "p");
    out
}

fn txbx_paths(root: &Element) -> Vec<Vec<usize>> {
    let mut out = Vec::new();
    walk_paths(root, &[], &mut out, "txbxContent");
    out
}

fn walk_paths(el: &Element, path: &[usize], out: &mut Vec<Vec<usize>>, local: &str) {
    for (i, node) in el.children.iter().enumerate() {
        let Node::Element(child) = node else {
            continue;
        };
        let mut next = path.to_vec();
        next.push(i);
        if child.local_name() == local {
            out.push(next.clone());
        }
        walk_paths(child, &next, out, local);
    }
}
