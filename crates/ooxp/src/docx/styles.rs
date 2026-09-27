//! `styles.xml` 与段落 `w:pStyle`：inspect 名单与换绑、改定义。

use serde_json::{json, Value};

use crate::error::{Error, ErrorKind, IdReason};
use crate::opc::{self, Package, Relationships};
use crate::xml::{Element, Node};

use super::REL_STYLES;

pub fn styles_part(pkg: &Package, doc_part: &str) -> Result<Option<String>, Error> {
    let rels = Relationships::from_package(pkg, doc_part)?;
    let Some(rel) = rels
        .rels
        .iter()
        .find(|r| r.rel_type == REL_STYLES && !r.is_external())
    else {
        return Ok(None);
    };
    Ok(Some(opc::resolve_rel_target(doc_part, &rel.target)?))
}

pub fn count_styles(pkg: &Package, doc_part: &str) -> Result<usize, Error> {
    let Some(part) = styles_part(pkg, doc_part)? else {
        return Ok(0);
    };
    let doc = crate::xml::parse(pkg.get(&part)?)?;
    Ok(collect_style_els(&doc.root).len())
}

pub fn styles_json(pkg: &Package, doc_part: &str) -> Result<Vec<Value>, Error> {
    let Some(part) = styles_part(pkg, doc_part)? else {
        return Ok(Vec::new());
    };
    let doc = crate::xml::parse(pkg.get(&part)?)?;
    let mut out = Vec::new();
    for el in collect_style_els(&doc.root) {
        let Some(style_id) = el.attr_local("styleId").filter(|s| !s.is_empty()) else {
            continue;
        };
        let type_val = el.attr_local("type").map(str::to_string);
        let name = el
            .child_by_local("name")
            .and_then(|e| e.attr_local("val"))
            .map(str::to_string);
        let based_on = el
            .child_by_local("basedOn")
            .and_then(|e| e.attr_local("val"))
            .map(str::to_string);
        let default = matches!(el.attr_local("default").unwrap_or(""), "1" | "true" | "on");
        out.push(json!({
            "id": format!("style:{style_id}"),
            "styleId": style_id,
            "type": type_val,
            "name": name,
            "basedOn": based_on,
            "default": default,
        }));
    }
    Ok(out)
}

pub fn parse_style_ref(id: &str, path: &str) -> Result<String, Error> {
    let rest = id.strip_prefix("style:").ok_or_else(|| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::StylePrefix,
            },
            path.to_string(),
        )
    })?;
    if rest.is_empty() {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::StylePrefix,
            },
            path.to_string(),
        ));
    }
    Ok(rest.to_string())
}

pub fn is_paragraph_style(el: &Element) -> bool {
    matches!(el.attr_local("type"), None | Some("paragraph"))
}

pub fn find_style<'a>(root: &'a Element, style_id: &str) -> Option<&'a Element> {
    collect_style_els(root)
        .into_iter()
        .find(|s| s.attr_local("styleId") == Some(style_id))
}

pub fn find_style_mut<'a>(root: &'a mut Element, style_id: &str) -> Option<&'a mut Element> {
    root.children.iter_mut().find_map(|n| match n {
        Node::Element(e)
            if e.local_name() == "style" && e.attr_local("styleId") == Some(style_id) =>
        {
            Some(e)
        }
        _ => None,
    })
}

pub fn missing_style(id: &str, i: usize) -> Error {
    Error::with_path(
        ErrorKind::StyleNotFound { id: id.to_string() },
        format!("ops[{i}].id"),
    )
}

pub fn missing_para_style(id: &str, i: usize) -> Error {
    Error::with_path(
        ErrorKind::StyleNotFound { id: id.to_string() },
        format!("ops[{i}].style"),
    )
}

pub fn bind_para(p: &mut Element, style_id: &str) {
    ensure_ppr(p);
    let ppr = p.child_by_local_mut("pPr").expect("刚写入 pPr");
    if let Some(ps) = ppr.child_by_local_mut("pStyle") {
        let key = ps.prefixed("val");
        ps.set_attr(&key, style_id);
        return;
    }
    let mut ps = Element::new(ppr.prefixed("pStyle"));
    let key = ps.prefixed("val");
    ps.set_attr(&key, style_id);
    ppr.children.insert(0, Node::Element(ps));
}

pub fn unbind_para(p: &mut Element) {
    let Some(ppr) = p.child_by_local_mut("pPr") else {
        return;
    };
    ppr.children
        .retain(|n| !matches!(n, Node::Element(e) if e.local_name() == "pStyle"));
}

pub struct StylePatch<'a> {
    pub font: Option<&'a str>,
    pub east_asia_font: Option<&'a str>,
    pub sz: Option<u32>,
    pub bold: Option<bool>,
    pub color: Option<&'a str>,
    pub before: Option<u32>,
    pub after: Option<u32>,
}

pub fn set_style_props(
    root: &mut Element,
    style_id: &str,
    patch: StylePatch<'_>,
    id: &str,
    i: usize,
) -> Result<(), Error> {
    let Some(style) = find_style_mut(root, style_id) else {
        return Err(missing_style(id, i));
    };
    if !is_paragraph_style(style) {
        return Err(Error::with_path(
            ErrorKind::SetStyleNeedsParagraph,
            format!("ops[{i}].id"),
        ));
    }
    if patch.font.is_some()
        || patch.east_asia_font.is_some()
        || patch.sz.is_some()
        || patch.bold.is_some()
        || patch.color.is_some()
    {
        ensure_style_rpr(style);
        let rpr = style.child_by_local_mut("rPr").expect("刚写入 rPr");
        if patch.font.is_some() || patch.east_asia_font.is_some() {
            apply_fonts(rpr, patch.font, patch.east_asia_font);
        }
        if let Some(sz) = patch.sz {
            set_sz(rpr, sz);
        }
        if let Some(bold) = patch.bold {
            set_bold(rpr, bold);
        }
        if let Some(color) = patch.color {
            set_color(rpr, color);
        }
    }
    if patch.before.is_some() || patch.after.is_some() {
        ensure_style_ppr(style);
        let ppr = style.child_by_local_mut("pPr").expect("刚写入 pPr");
        ensure_child(ppr, "spacing");
        let spacing = ppr.child_by_local_mut("spacing").expect("刚写入 spacing");
        if let Some(n) = patch.before {
            let key = spacing.prefixed("before");
            spacing.set_attr(&key, n.to_string());
        }
        if let Some(n) = patch.after {
            let key = spacing.prefixed("after");
            spacing.set_attr(&key, n.to_string());
        }
    }
    Ok(())
}

fn collect_style_els(root: &Element) -> Vec<&Element> {
    root.children_by_local("style")
        .into_iter()
        .filter(|s| s.attr_local("styleId").is_some_and(|id| !id.is_empty()))
        .collect()
}

fn ensure_ppr(p: &mut Element) {
    if p.child_by_local("pPr").is_some() {
        return;
    }
    let ppr = Element::new(p.prefixed("pPr"));
    p.children.insert(0, Node::Element(ppr));
}

fn ensure_style_ppr(style: &mut Element) {
    if style.child_by_local("pPr").is_some() {
        return;
    }
    let ppr = Element::new(style.prefixed("pPr"));
    let at = style
        .children
        .iter()
        .position(|n| matches!(n, Node::Element(e) if e.local_name() == "rPr"));
    if let Some(i) = at {
        style.children.insert(i, Node::Element(ppr));
    } else {
        style.push_element(ppr);
    }
}

fn ensure_style_rpr(style: &mut Element) {
    if style.child_by_local("rPr").is_some() {
        return;
    }
    let rpr = Element::new(style.prefixed("rPr"));
    let at = style
        .children
        .iter()
        .position(|n| matches!(n, Node::Element(e) if e.local_name() == "pPr"));
    if let Some(i) = at {
        style.children.insert(i + 1, Node::Element(rpr));
    } else {
        style.push_element(rpr);
    }
}

fn ensure_child(parent: &mut Element, local: &str) {
    if parent.child_by_local(local).is_some() {
        return;
    }
    parent.push_element(Element::new(parent.prefixed(local)));
}

fn apply_fonts(rpr: &mut Element, font: Option<&str>, east_asia: Option<&str>) {
    ensure_child(rpr, "rFonts");
    let fonts = rpr.child_by_local_mut("rFonts").expect("刚写入 rFonts");
    if let Some(name) = font {
        let ascii = fonts.prefixed("ascii");
        let h_ansi = fonts.prefixed("hAnsi");
        let cs = fonts.prefixed("cs");
        fonts.set_attr(&ascii, name);
        fonts.set_attr(&h_ansi, name);
        fonts.set_attr(&cs, name);
    }
    if let Some(name) = east_asia {
        let key = fonts.prefixed("eastAsia");
        fonts.set_attr(&key, name);
    }
}

fn set_sz(rpr: &mut Element, sz: u32) {
    for local in ["sz", "szCs"] {
        ensure_child(rpr, local);
        let el = rpr.child_by_local_mut(local).expect("刚写入 sz");
        let key = el.prefixed("val");
        el.set_attr(&key, sz.to_string());
    }
}

fn set_bold(rpr: &mut Element, bold: bool) {
    for local in ["b", "bCs"] {
        ensure_child(rpr, local);
        let el = rpr.child_by_local_mut(local).expect("刚写入 b");
        let key = el.prefixed("val");
        if bold {
            el.remove_attr(&key);
        } else {
            el.set_attr(&key, "0");
        }
    }
}

fn set_color(rpr: &mut Element, hex: &str) {
    ensure_child(rpr, "color");
    let el = rpr.child_by_local_mut("color").expect("刚写入 color");
    el.attrs.retain(|(k, _)| {
        let local = k.rsplit(':').next().unwrap_or(k);
        !matches!(local, "themeColor" | "themeTint" | "themeShade" | "theme")
    });
    let key = el.prefixed("val");
    el.set_attr(&key, hex.to_ascii_uppercase());
}
