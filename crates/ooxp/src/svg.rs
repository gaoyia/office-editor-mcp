//! Office 2016 SVG 扩展：xlsx / pptx 共用。
//! docx 44 已过，保持自己的写入路径，不经这里。

use crate::xml::{Element, Node};

/// Word / Excel / PowerPoint 16.0 另存用的 SVG 命名空间。
pub const NS: &str = "http://schemas.microsoft.com/office/drawing/2016/SVG/main";
/// `a:ext/@uri`。
pub const EXT_URI: &str = "{96DAC541-7B7A-43D3-8B79-37D633B846F1}";
/// `[Content_Types].xml` 的 Default `svg`。
pub const CT: &str = "image/svg+xml";

/// 文件头认 SVG：BOM / 空白之后能看到 `<svg` 标签。不认 SVGZ。
pub fn looks_like(bytes: &[u8]) -> bool {
    let mut b = bytes;
    if b.starts_with(&[0xEF, 0xBB, 0xBF]) {
        b = &b[3..];
    }
    while let Some((&c, rest)) = b.split_first() {
        if c.is_ascii_whitespace() {
            b = rest;
        } else {
            break;
        }
    }
    let n = b.len().min(8192);
    let hay = &b[..n];
    let mut i = 0;
    while i + 4 <= hay.len() {
        if hay[i] == b'<' {
            let rest = &hay[i + 1..];
            if rest.len() >= 3 && rest[..3].eq_ignore_ascii_case(b"svg") {
                let next = rest.get(3).copied();
                if matches!(
                    next,
                    None | Some(b' ')
                        | Some(b'\t')
                        | Some(b'\n')
                        | Some(b'\r')
                        | Some(b'>')
                        | Some(b'/')
                ) {
                    return true;
                }
            }
        }
        i += 1;
    }
    false
}

/// 根 `width` / `height`（无单位或 `px` 当 CSS 像素，96 dpi）；没有则 `viewBox`。
pub fn size(bytes: &[u8]) -> Option<(i64, i64)> {
    let text = std::str::from_utf8(bytes).ok()?;
    let tag = svg_open_tag(text)?;
    match (
        parse_css_len(svg_attr(tag, "width")),
        parse_css_len(svg_attr(tag, "height")),
    ) {
        (Some(w), Some(h)) => Some((px_to_emu_f(w), px_to_emu_f(h))),
        _ => viewbox_size(svg_attr(tag, "viewBox")),
    }
}

/// 树上任意节点里 `asvg:svgBlip/@r:embed`。
pub fn blip_rid(el: &Element) -> Option<String> {
    let mut found = None;
    walk(el, &mut |e| {
        if found.is_some() {
            return;
        }
        if e.local_name() == "svgBlip" {
            found = e
                .attr_local("embed")
                .filter(|s| !s.is_empty())
                .map(str::to_string);
        }
    });
    found
}

/// 有 `svgBlip` 则为 `image/svg+xml`。
pub fn content_type(el: &Element) -> Option<&'static str> {
    if blip_rid(el).is_some() {
        Some(CT)
    } else {
        None
    }
}

/// 在 `a:blip` 上写或改 `asvg:svgBlip`。`el` 可以是图，也可以就是 `a:blip`。
pub fn set_svg_blip(el: &mut Element, svg_rid: &str) {
    let Some(blip) = find_local_mut(el, "blip") else {
        return;
    };
    if let Some(lst) = blip.child_by_local_mut("extLst") {
        for child in lst.element_children_mut() {
            if child.local_name() == "ext" && child.attr("uri") == Some(EXT_URI) {
                if let Some(svg) = child.child_by_local_mut("svgBlip") {
                    set_attr_local(svg, "embed", svg_rid);
                    return;
                }
                child.children.clear();
                child.push_element(
                    Element::new("asvg:svgBlip")
                        .with_attr("xmlns:asvg", NS)
                        .with_attr("r:embed", svg_rid),
                );
                return;
            }
        }
        let svg_blip = Element::new("asvg:svgBlip")
            .with_attr("xmlns:asvg", NS)
            .with_attr("r:embed", svg_rid);
        let mut ext = Element::new("a:ext").with_attr("uri", EXT_URI);
        ext.push_element(svg_blip);
        lst.push_element(ext);
        return;
    }
    blip.push_element(svg_ext_lst(svg_rid));
}

/// 去掉 SVG 扩展。空的 `a:extLst` 一并摘掉。
pub fn strip_svg_blip(el: &mut Element) {
    let Some(blip) = find_local_mut(el, "blip") else {
        return;
    };
    let empty = {
        let Some(lst) = blip.child_by_local_mut("extLst") else {
            return;
        };
        lst.children.retain(|n| match n {
            Node::Element(e) => !(e.local_name() == "ext" && e.attr("uri") == Some(EXT_URI)),
            Node::Text(_) => true,
        });
        lst.element_children().next().is_none()
    };
    if empty {
        blip.children
            .retain(|n| !matches!(n, Node::Element(e) if e.local_name() == "extLst"));
    }
}

/// 给没有 `r:embed` 的 `a:blip` 补后备图关系。
pub fn set_blip_embed(el: &mut Element, rid: &str) {
    let Some(blip) = find_local_mut(el, "blip") else {
        return;
    };
    set_attr_local(blip, "embed", rid);
}

fn svg_ext_lst(svg_rid: &str) -> Element {
    let svg_blip = Element::new("asvg:svgBlip")
        .with_attr("xmlns:asvg", NS)
        .with_attr("r:embed", svg_rid);
    let mut ext = Element::new("a:ext").with_attr("uri", EXT_URI);
    ext.push_element(svg_blip);
    let mut lst = Element::new("a:extLst");
    lst.push_element(ext);
    lst
}

fn svg_open_tag(text: &str) -> Option<&str> {
    let lower = text.to_ascii_lowercase();
    let start = lower.find("<svg")?;
    let rest = &text[start..];
    let end = rest.find('>')?;
    Some(&rest[..=end])
}

fn svg_attr<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let lower = tag.to_ascii_lowercase();
    let key = format!("{name}=");
    let i = lower.find(&key)?;
    let rest = tag[i + key.len()..].trim_start();
    let quote = rest.as_bytes().first().copied()?;
    if quote != b'"' && quote != b'\'' {
        return None;
    }
    let inner = &rest[1..];
    let j = inner.as_bytes().iter().position(|&c| c == quote)?;
    Some(&inner[..j])
}

fn viewbox_size(raw: Option<&str>) -> Option<(i64, i64)> {
    let parts: Vec<&str> = raw?.split_whitespace().collect();
    if parts.len() != 4 {
        return None;
    }
    let w: f64 = parts[2].parse().ok()?;
    let h: f64 = parts[3].parse().ok()?;
    if w <= 0.0 || h <= 0.0 {
        return None;
    }
    Some((px_to_emu_f(w), px_to_emu_f(h)))
}

fn parse_css_len(raw: Option<&str>) -> Option<f64> {
    let s = raw?.trim();
    if s.is_empty() || s.ends_with('%') {
        return None;
    }
    let mut i = 0usize;
    let bs = s.as_bytes();
    if i < bs.len() && (bs[i] == b'+' || bs[i] == b'-') {
        i += 1;
    }
    let start_digit = i;
    while i < bs.len() && (bs[i].is_ascii_digit() || bs[i] == b'.') {
        i += 1;
    }
    if i == start_digit {
        return None;
    }
    let n: f64 = s[..i].parse().ok()?;
    if n <= 0.0 {
        return None;
    }
    let unit = s[i..].trim().to_ascii_lowercase();
    let px = match unit.as_str() {
        "" | "px" => n,
        "pt" => n * 96.0 / 72.0,
        "in" => n * 96.0,
        "cm" => n * 96.0 / 2.54,
        "mm" => n * 96.0 / 25.4,
        "pc" => n * 16.0,
        _ => return None,
    };
    Some(px)
}

fn px_to_emu_f(px: f64) -> i64 {
    let v = (px * 914400.0 / 96.0).round();
    if !v.is_finite() || v < 1.0 {
        914400
    } else {
        v.min(i64::MAX as f64) as i64
    }
}

fn walk(el: &Element, f: &mut impl FnMut(&Element)) {
    f(el);
    for c in el.element_children() {
        walk(c, f);
    }
}

fn find_local_mut<'a>(el: &'a mut Element, local: &str) -> Option<&'a mut Element> {
    if el.local_name() == local {
        return Some(el);
    }
    let idx = el.children.iter().position(|n| match n {
        Node::Element(c) => contains_local(c, local),
        _ => false,
    })?;
    let Node::Element(child) = &mut el.children[idx] else {
        return None;
    };
    find_local_mut(child, local)
}

fn contains_local(el: &Element, local: &str) -> bool {
    el.local_name() == local || el.element_children().any(|c| contains_local(c, local))
}

fn set_attr_local(el: &mut Element, local: &str, value: &str) {
    if let Some((k, _)) = el
        .attrs
        .iter()
        .find(|(k, _)| k.rsplit(':').next().unwrap_or(k) == local)
    {
        let name = k.clone();
        el.set_attr(&name, value);
        return;
    }
    let prefixed = if local == "embed" {
        format!("r:{local}")
    } else {
        local.to_string()
    };
    el.set_attr(&prefixed, value);
}
