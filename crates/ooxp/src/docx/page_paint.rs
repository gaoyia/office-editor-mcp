//! 把 docx 里没有可见内容的一页，或单节、单段（可换行）的不透明拉丁 Calibri 正文，画成 PNG。
//! 宽高按节的 `w:pgSz`，96 dpi，twip×96/1440 四舍五入。
//! 有表、图、页眉页脚，或还没对齐的行距/分页控制时不画。单节拉丁 Calibri 多段（可空段、段前段后、左缩进）与空格换行已画；字形灰度仍暂缓。
//! 有字页在 Windows 上走已有公开 GDI+ `AntiAliasGridFit` 路径。字形灰度按 C25 第 0 段暂缓，
//! 不把灰阶差算成整页通过，也不借用幻灯片文字的布局近似许可。

use std::io::Cursor;

use crate::error::{Error, ErrorKind, IdReason};
use crate::opc::{self, Package};
use crate::xlsx::Face;
use crate::xml::{self, Element, Node};

use super::export::ExportedMedia;
use super::styles;

/// Calibri 单倍行距倍数：本机 calibri.ttf 的 hhea / OS/2 typo / win 行框之和均为 **2500**，
/// unitsPerEm = **2048**（字体指标，非样本拟合）。
/// 半磅字号 sz 下行高 twip = sz * 20/2 * 2500 / 2048 = sz * 25000 / 2048。
/// LINE_NUM = 默认 11 磅（sz=22）空段分页分子：22 * 25000。
const LINE_NUM: u64 = 550_000;
const LINE_DEN: u64 = 2048;
const MAX_PX: u64 = 4096;
const DEFAULT_SZ: u32 = 22;

pub fn export_page_image(bytes: &[u8], id: &str) -> Result<ExportedMedia, Error> {
    let page = parse_page_id(id)?;
    let pkg = Package::open_bytes(bytes.to_vec())?;
    let doc_part = opc::office_document_part(&pkg)?;
    let doc = xml::parse(pkg.get(&doc_part)?)?;
    let layout = layout_of(&pkg, &doc_part, &doc.root)?;
    if page == 0 || page as usize > layout.pages.len() {
        return Err(Error::with_path(ErrorKind::PageNotFound { n: page }, "id"));
    }
    let png = paint_page(&layout, page)?;
    Ok(ExportedMedia {
        bytes: png,
        content_type: Some("image/png".to_string()),
    })
}

struct Layout {
    width: u32,
    height: u32,
    /// 下标 0 = page:1。
    pages: Vec<Page>,
}

struct Page {
    texts: Vec<TextLine>,
    images: Vec<ImageBlit>,
}

struct ImageBlit {
    left_twip: i64,
    top_twip: i64,
    width_px: u32,
    height_px: u32,
    /// 不透明 RGB，长度 width*height*3。
    rgb: Vec<u8>,
}

struct TextLine {
    text: String,
    sz: u32,
    left_twip: i64,
    top_twip: i64,
    body_w_twip: i64,
}

fn layout_of(pkg: &Package, doc_part: &str, root: &Element) -> Result<Layout, Error> {
    if root.element_children().any(|el| el.local_name() != "body") {
        return Err(unsupported());
    }
    let body = root.child_by_local("body").ok_or_else(unsupported)?;
    let (story, sect) = body_story(pkg, doc_part, body)?;
    let has_text = story.iter().any(|p| matches!(p, StoryPara::Text(_)));
    let has_image = story.iter().any(|p| matches!(p, StoryPara::Image(_)));
    if !sect_is_plain(sect, has_text || has_image) || !uses_default_calibri(pkg, doc_part)? {
        return Err(unsupported());
    }
    let page_w = page_twip(sect, "w")?;
    let page_h = page_twip(sect, "h")?;
    let top = margin(sect, "top")?;
    let bottom = margin(sect, "bottom")?;
    let left = margin(sect, "left")?;
    let right = margin(sect, "right")?;
    let body_h = page_h - top - bottom;
    let body_w = page_w - left - right;
    if body_h <= 0 || body_w <= 0 {
        return Err(unsupported());
    }
    if let Some(mar) = sect.child_by_local("pgMar") {
        if let Some(gutter) = mar.attr_local("gutter") {
            if parse_twip(gutter)? != 0 {
                return Err(unsupported());
            }
        }
    }
    let width = twip_px(page_w)?;
    let height = twip_px(page_h)?;
    let pages = layout_story(&story, left, top, body_w, body_h)?;
    if pages.is_empty() {
        return Err(unsupported());
    }
    Ok(Layout {
        width,
        height,
        pages,
    })
}

fn ensure_space(
    pages: &mut Vec<Page>,
    y: &mut i64,
    at_page_top: &mut bool,
    margin_top: i64,
    page_bottom: i64,
    need: i64,
) -> Result<(), Error> {
    if need <= 0 {
        return Ok(());
    }
    if *y + need <= page_bottom {
        return Ok(());
    }
    if *at_page_top {
        return Err(unsupported());
    }
    pages.push(Page {
        texts: Vec::new(),
        images: Vec::new(),
    });
    *y = margin_top;
    *at_page_top = true;
    if *y + need > page_bottom {
        return Err(unsupported());
    }
    Ok(())
}

/// 多段流式排版。行距见 `line_advance_twip`（Calibri TTF 2500/2048）。
/// 段前在段起始计入一次；换到新页顶后不再补段前。空段占一行默认 11 磅行高。
fn layout_story(
    story: &[StoryPara],
    margin_left: i64,
    margin_top: i64,
    body_w: i64,
    body_h: i64,
) -> Result<Vec<Page>, Error> {
    // ??????????????floor(body_h * 2048 / 550000) ? ? ???
    let adv11 = line_advance_twip(DEFAULT_SZ)?;
    let lines_fit = (body_h as u64).saturating_mul(LINE_DEN) / LINE_NUM;
    let usable = (lines_fit as i64).saturating_mul(adv11).min(body_h);
    let page_bottom = margin_top.saturating_add(usable);
    let mut pages: Vec<Page> = vec![Page {
        texts: Vec::new(),
        images: Vec::new(),
    }];
    let mut y = margin_top;
    let mut at_page_top = true;

    for para in story {
        match para {
            StoryPara::Empty { before, after } => {
                let adv = adv11;
                let before_use = (*before).max(0);
                ensure_space(
                    &mut pages,
                    &mut y,
                    &mut at_page_top,
                    margin_top,
                    page_bottom,
                    before_use.saturating_add(adv),
                )?;
                // 换页后段前仍计（文档首页/段起始与 Word COM 一致；见 first-before 探针）。
                let before_use = (*before).max(0);
                y = y.saturating_add(before_use).saturating_add(adv);
                at_page_top = false;
                y = y.saturating_add((*after).max(0));
            }
            StoryPara::Image(img) => {
                let before_use = img.before_twip.max(0);
                let h_twip = emu_twip(img.cy_emu)?;
                ensure_space(
                    &mut pages,
                    &mut y,
                    &mut at_page_top,
                    margin_top,
                    page_bottom,
                    before_use.saturating_add(h_twip),
                )?;
                let before_use = img.before_twip.max(0);
                y = y.saturating_add(before_use);
                let left_twip = margin_left.saturating_add(img.indent_twip.max(0));
                if left_twip < 0 {
                    return Err(unsupported());
                }
                let w_twip = emu_twip(img.cx_emu)?;
                if w_twip > body_w.saturating_sub(img.indent_twip.max(0)) {
                    return Err(unsupported());
                }
                pages
                    .last_mut()
                    .ok_or_else(unsupported)?
                    .images
                    .push(ImageBlit {
                        left_twip,
                        top_twip: y,
                        width_px: img.width_px,
                        height_px: img.height_px,
                        rgb: img.rgb.clone(),
                    });
                y = y.saturating_add(h_twip);
                at_page_top = false;
                y = y.saturating_add(img.after_twip.max(0));
            }
            StoryPara::Text(line) => {
                let content_w = body_w.saturating_sub(line.indent_twip.max(0));
                if content_w <= 0 {
                    return Err(unsupported());
                }
                let left_twip = margin_left.saturating_add(line.indent_twip);
                if left_twip < 0 {
                    return Err(unsupported());
                }
                let rows = wrap_latin_lines(&line.text, line.sz, content_w)?;
                let adv = line_advance_twip(line.sz)?;
                for (i, row) in rows.into_iter().enumerate() {
                    let lead = if i == 0 { line.before_twip.max(0) } else { 0 };
                    ensure_space(
                        &mut pages,
                        &mut y,
                        &mut at_page_top,
                        margin_top,
                        page_bottom,
                        lead.saturating_add(adv),
                    )?;
                    y = y.saturating_add(lead);
                    pages
                        .last_mut()
                        .ok_or_else(unsupported)?
                        .texts
                        .push(TextLine {
                            text: row,
                            sz: line.sz,
                            left_twip,
                            top_twip: y,
                            body_w_twip: content_w,
                        });
                    y = y.saturating_add(adv);
                    at_page_top = false;
                }
                y = y.saturating_add(line.after_twip.max(0));
            }
        }
    }
    Ok(pages)
}

fn line_advance_twip(sz: u32) -> Result<i64, Error> {
    line_offset_twip(1, sz)
}

/// Calibri 单倍：`sz` 半磅 × 25000 / 2048 twip（见文件头 Calibri TTF 2500/2048）。
fn line_offset_twip(line_index: u64, sz: u32) -> Result<i64, Error> {
    // DEFAULT_SZ 走 LINE_NUM，与历史空段分页同一整数除法路径。
    let n = if sz == DEFAULT_SZ {
        line_index.saturating_mul(LINE_NUM) / LINE_DEN
    } else {
        line_index
            .saturating_mul(u64::from(sz))
            .saturating_mul(25_000)
            / LINE_DEN
    };
    i64::try_from(n).map_err(|_| unsupported())
}

fn wrap_latin_lines(text: &str, sz: u32, body_w_twip: i64) -> Result<Vec<String>, Error> {
    #[cfg(not(windows))]
    {
        let _ = (text, sz, body_w_twip);
        return Err(unsupported());
    }
    #[cfg(windows)]
    {
        let pt = (sz as f32) / 2.0;
        let px = pt * 96.0 / 72.0;
        let face = Face::open_named(px, false, "Calibri").ok_or_else(unsupported)?;
        let max_w = f64::from(twip_px_unclamped(body_w_twip)?);
        if face.typo_width(text) <= max_w {
            return Ok(vec![text.to_string()]);
        }
        // ASCII 空格贪心：断在最后一个仍放得下的空格；断点空格留在上行（与 Word 16.0 一致）。
        // 不 collapse 多空格。
        let mut lines: Vec<String> = Vec::new();
        let mut start = 0usize;
        let chars: Vec<char> = text.chars().collect();
        let mut i = 0usize;
        let mut last_space: Option<usize> = None;
        while i < chars.len() {
            if chars[i] == ' ' {
                last_space = Some(i);
            }
            let end = i + 1;
            let candidate: String = chars[start..end].iter().collect();
            if face.typo_width(&candidate) <= max_w {
                i += 1;
                continue;
            }
            let Some(sp) = last_space.filter(|&sp| sp >= start) else {
                return Err(unsupported());
            };
            // 上行含断点空格。
            let line: String = chars[start..=sp].iter().collect();
            if line.trim().is_empty() {
                return Err(unsupported());
            }
            lines.push(line);
            start = sp + 1;
            last_space = None;
            i = start;
            if start >= chars.len() {
                break;
            }
            // 单个词本身超宽
            if chars[start] != ' ' {
                let mut j = start;
                while j < chars.len() && chars[j] != ' ' {
                    j += 1;
                }
                let word: String = chars[start..j].iter().collect();
                if face.typo_width(&word) > max_w {
                    return Err(unsupported());
                }
            }
        }
        if start < chars.len() {
            let rest: String = chars[start..].iter().collect();
            if !rest.is_empty() {
                lines.push(rest);
            }
        }
        if lines.is_empty() {
            return Err(unsupported());
        }
        Ok(lines)
    }
}

struct PendingText {
    text: String,
    sz: u32,
    indent_twip: i64,
    before_twip: i64,
    after_twip: i64,
}

enum StoryPara {
    Empty { before: i64, after: i64 },
    Text(PendingText),
    Image(PendingImage),
}

struct PendingImage {
    indent_twip: i64,
    before_twip: i64,
    after_twip: i64,
    cx_emu: i64,
    cy_emu: i64,
    width_px: u32,
    height_px: u32,
    rgb: Vec<u8>,
}

fn body_story<'a>(
    pkg: &Package,
    doc_part: &str,
    body: &'a Element,
) -> Result<(Vec<StoryPara>, &'a Element), Error> {
    let mut story: Vec<StoryPara> = Vec::new();
    let mut sect: Option<&Element> = None;
    let children: Vec<&Element> = body.element_children().collect();
    for (index, child) in children.iter().copied().enumerate() {
        match child.local_name() {
            "p" => {
                if sect.is_some() {
                    return Err(unsupported());
                }
                story.push(paragraph_kind(pkg, doc_part, child)?);
            }
            "sectPr" => {
                if sect.is_some() || index + 1 != children.len() {
                    return Err(unsupported());
                }
                sect = Some(child);
            }
            _ => return Err(unsupported()),
        }
    }
    let Some(sect) = sect else {
        return Err(unsupported());
    };
    if story.is_empty() {
        return Err(unsupported());
    }
    Ok((story, sect))
}

fn paragraph_kind(pkg: &Package, doc_part: &str, p: &Element) -> Result<StoryPara, Error> {
    let mut indent = 0i64;
    let mut before = 0i64;
    let mut after = 0i64;
    let mut run: Option<&Element> = None;
    for child in p.element_children() {
        match child.local_name() {
            "pPr" => {
                parse_plain_ppr(child, &mut indent, &mut before, &mut after)?;
            }
            "r" if run.is_none() => run = Some(child),
            _ => return Err(unsupported()),
        }
    }
    let Some(run) = run else {
        if p.children.iter().any(|node| match node {
            Node::Element(_) => false,
            Node::Text(text) => !text.trim().is_empty(),
            Node::Raw(_) => false,
        }) {
            return Err(unsupported());
        }
        return Ok(StoryPara::Empty { before, after });
    };
    // ? run?????? inline ???? NI?
    let mut drawing: Option<&Element> = None;
    let mut has_text = false;
    for child in run.element_children() {
        match child.local_name() {
            "drawing" => {
                if drawing.is_some() {
                    return Err(unsupported());
                }
                drawing = Some(child);
            }
            "t" => {
                if !element_text(child).is_empty() {
                    has_text = true;
                }
            }
            "rPr" => {}
            "tab" | "br" | "cr" | "noBreakHyphen" | "softHyphen" | "sym" | "object" | "pict"
            | "fldChar" | "instrText" => return Err(unsupported()),
            _ => return Err(unsupported()),
        }
    }
    if drawing.is_some() && has_text {
        return Err(unsupported());
    }
    if let Some(drawing) = drawing {
        let img = plain_inline_image(pkg, doc_part, drawing)?;
        return Ok(StoryPara::Image(PendingImage {
            indent_twip: indent,
            before_twip: before,
            after_twip: after,
            cx_emu: img.0,
            cy_emu: img.1,
            width_px: img.2,
            height_px: img.3,
            rgb: img.4,
        }));
    }
    let (text, sz) = plain_run(run)?;
    if text.is_empty() {
        return Ok(StoryPara::Empty { before, after });
    }
    Ok(StoryPara::Text(PendingText {
        text,
        sz,
        indent_twip: indent,
        before_twip: before,
        after_twip: after,
    }))
}

fn parse_plain_ppr(
    ppr: &Element,
    indent: &mut i64,
    before: &mut i64,
    after: &mut i64,
) -> Result<(), Error> {
    for child in ppr.element_children() {
        match child.local_name() {
            "ind" => {
                if child.element_children().next().is_some() {
                    return Err(unsupported());
                }
                for (name, value) in child.attrs.iter() {
                    let local = local_of(name);
                    match local {
                        "left" | "start" => *indent = parse_twip(value)?,
                        "right" | "end" | "hanging" | "firstLine" => {
                            if parse_twip(value)? != 0 {
                                return Err(unsupported());
                            }
                        }
                        "leftChars" | "rightChars" | "startChars" | "endChars" | "hangingChars"
                        | "firstLineChars" => {
                            if value != "0" {
                                return Err(unsupported());
                            }
                        }
                        _ => return Err(unsupported()),
                    }
                }
            }
            "spacing" => {
                if child.element_children().next().is_some() {
                    return Err(unsupported());
                }
                for (name, value) in child.attrs.iter() {
                    let local = local_of(name);
                    match local {
                        "before" => *before = parse_twip(value)?,
                        "after" => *after = parse_twip(value)?,
                        "beforeAutospacing" | "afterAutospacing" => {
                            if value != "0" {
                                return Err(unsupported());
                            }
                        }
                        // 显式 line/lineRule：本切片只接受缺省单倍（Calibri 2500/2048）。
                        "line" | "lineRule" => return Err(unsupported()),
                        _ => return Err(unsupported()),
                    }
                }
            }
            "jc" => {
                let val = child.attr_local("val").unwrap_or("left");
                if val != "left" && val != "start" {
                    return Err(unsupported());
                }
            }
            _ => return Err(unsupported()),
        }
    }
    Ok(())
}
fn plain_run(run: &Element) -> Result<(String, u32), Error> {
    let mut sz = DEFAULT_SZ;
    let mut text = String::new();
    let mut saw_text = false;
    for child in run.element_children() {
        match child.local_name() {
            "rPr" => parse_plain_rpr(child, &mut sz)?,
            "t" => {
                if saw_text {
                    return Err(unsupported());
                }
                saw_text = true;
                text.push_str(&element_text(child));
            }
            _ => return Err(unsupported()),
        }
    }
    if !saw_text || text.chars().any(|ch| ch.is_control() || ch == '\t') {
        return Err(unsupported());
    }
    if !text.is_ascii() {
        return Err(unsupported());
    }
    Ok((text, sz))
}

fn parse_plain_rpr(rpr: &Element, sz: &mut u32) -> Result<(), Error> {
    let mut sz_cs: Option<u32> = None;
    for child in rpr.element_children() {
        match child.local_name() {
            "sz" => *sz = parse_sz(child)?,
            "szCs" => sz_cs = Some(parse_sz(child)?),
            "rFonts" => {
                if child.element_children().next().is_some() {
                    return Err(unsupported());
                }
                for (name, value) in child.attrs.iter() {
                    let local = local_of(name);
                    match local {
                        "ascii" | "hAnsi" => {
                            if value != "Calibri" {
                                return Err(unsupported());
                            }
                        }
                        "eastAsia" | "cs" | "hint" => {}
                        _ => return Err(unsupported()),
                    }
                }
            }
            "color" => {
                let val = child.attr_local("val").unwrap_or("auto");
                if val != "auto" && !val.eq_ignore_ascii_case("000000") {
                    return Err(unsupported());
                }
                if child.attrs.iter().any(|(name, _)| local_of(name) != "val") {
                    return Err(unsupported());
                }
            }
            "lang" => {}
            _ => return Err(unsupported()),
        }
    }
    if let Some(other) = sz_cs {
        if other != *sz {
            return Err(unsupported());
        }
    }
    Ok(())
}

fn parse_sz(el: &Element) -> Result<u32, Error> {
    let raw = el.attr_local("val").ok_or_else(unsupported)?;
    let n: u32 = raw.parse().map_err(|_| unsupported())?;
    if !(2..=3276).contains(&n) {
        return Err(unsupported());
    }
    Ok(n)
}

fn parse_twip(raw: &str) -> Result<i64, Error> {
    raw.parse().map_err(|_| unsupported())
}

fn element_text(el: &Element) -> String {
    let mut out = String::new();
    for node in &el.children {
        if let Node::Text(text) = node {
            out.push_str(text);
        }
    }
    out
}

fn local_of(name: &str) -> &str {
    name.rsplit_once(':')
        .map(|(_, local)| local)
        .unwrap_or(name)
}

fn sect_is_plain(sect: &Element, has_text: bool) -> bool {
    sect.element_children()
        .all(|child| match child.local_name() {
            "pgSz" | "pgMar" | "type" | "formProt" | "paperSrc" | "textDirection" | "bidi"
            | "rtlGutter" | "pgNumType" => true,
            "titlePg" => !has_text,
            "vAlign" => {
                let val = child.attr_local("val").unwrap_or("top");
                val == "top"
            }
            "cols" => {
                child.attr_local("num").unwrap_or("1") == "1"
                    && child.element_children().next().is_none()
            }
            _ => false,
        })
}

fn uses_default_calibri(pkg: &Package, doc_part: &str) -> Result<bool, Error> {
    let Some(part) = styles::styles_part(pkg, doc_part)? else {
        return Ok(false);
    };
    let styles = xml::parse(pkg.get(&part)?)?;
    let root = &styles.root;
    let Some(defaults) = root.child_by_local("docDefaults") else {
        return Ok(false);
    };
    if let Some(ppr_default) = defaults.child_by_local("pPrDefault") {
        if let Some(ppr) = ppr_default.child_by_local("pPr") {
            if ppr.element_children().next().is_some() {
                return Ok(false);
            }
        }
        if ppr_default
            .element_children()
            .any(|el| el.local_name() != "pPr")
        {
            return Ok(false);
        }
    }
    let Some(rpr) = defaults
        .child_by_local("rPrDefault")
        .and_then(|el| el.child_by_local("rPr"))
    else {
        return Ok(false);
    };
    if rpr.child_by_local("sz").and_then(|el| el.attr_local("val")) != Some("22") {
        return Ok(false);
    }
    if rpr
        .child_by_local("rFonts")
        .and_then(|el| el.attr_local("ascii"))
        != Some("Calibri")
    {
        return Ok(false);
    }
    let Some(normal) = root.element_children().find(|el| {
        el.local_name() == "style"
            && el.attr_local("type") == Some("paragraph")
            && el.attr_local("default") == Some("1")
    }) else {
        return Ok(false);
    };
    if let Some(ppr) = normal.child_by_local("pPr") {
        if ppr.element_children().next().is_some() {
            return Ok(false);
        }
    }
    Ok(true)
}

fn page_twip(sect: &Element, local: &str) -> Result<i64, Error> {
    let raw = sect
        .child_by_local("pgSz")
        .and_then(|el| el.attr_local(local))
        .ok_or_else(unsupported)?;
    let n: i64 = raw.parse().map_err(|_| unsupported())?;
    if n <= 0 {
        return Err(unsupported());
    }
    Ok(n)
}

fn margin(sect: &Element, local: &str) -> Result<i64, Error> {
    let Some(mar) = sect.child_by_local("pgMar") else {
        return Ok(1440);
    };
    match mar.attr_local(local) {
        None => Ok(1440),
        Some(raw) => raw.parse().map_err(|_| unsupported()),
    }
}

fn twip_px(twip: i64) -> Result<u32, Error> {
    let n = u64::try_from(twip).map_err(|_| unsupported())?;
    let px = (n + 7) / 15;
    if !(1..=MAX_PX).contains(&px) {
        return Err(unsupported());
    }
    Ok(px as u32)
}

fn paint_page(layout: &Layout, page: u32) -> Result<Vec<u8>, Error> {
    let mut rgb = vec![255u8; (layout.width as usize) * (layout.height as usize) * 3];
    let idx = (page as usize).saturating_sub(1);
    if let Some(page_items) = layout.pages.get(idx) {
        for img in &page_items.images {
            paint_image(&mut rgb, layout.width, layout.height, img)?;
        }
        for line in &page_items.texts {
            paint_text(&mut rgb, layout.width, layout.height, line)?;
        }
    }
    encode_png(layout.width, layout.height, &rgb)
}

fn paint_image(rgb: &mut [u8], page_w: u32, page_h: u32, img: &ImageBlit) -> Result<(), Error> {
    let x0 = twip_px_unclamped(img.left_twip)? as i32;
    let y0 = twip_px_unclamped(img.top_twip)? as i32;
    if x0 < 0 || y0 < 0 {
        return Err(unsupported());
    }
    let w = img.width_px as i32;
    let h = img.height_px as i32;
    if x0 + w > page_w as i32 || y0 + h > page_h as i32 {
        return Err(unsupported());
    }
    // 1:1 ??????????? PPT HighQualityBicubic?
    for row in 0..img.height_px {
        let src_off = (row as usize) * (img.width_px as usize) * 3;
        let dst_y = (y0 as u32).saturating_add(row);
        let dst_off = (dst_y as usize * page_w as usize + x0 as usize) * 3;
        let n = (img.width_px as usize) * 3;
        rgb[dst_off..dst_off + n].copy_from_slice(&img.rgb[src_off..src_off + n]);
    }
    Ok(())
}

fn paint_text(rgb: &mut [u8], width: u32, height: u32, line: &TextLine) -> Result<(), Error> {
    #[cfg(not(windows))]
    {
        let _ = (rgb, width, height, line);
        return Err(unsupported());
    }
    #[cfg(windows)]
    {
        let pt = (line.sz as f32) / 2.0;
        let px = pt * 96.0 / 72.0;
        let face = Face::open_named(px, false, "Calibri").ok_or_else(unsupported)?;
        let body_w = f64::from(twip_px_unclamped(line.body_w_twip)?);
        if face.typo_width(&line.text) > body_w {
            return Err(unsupported());
        }
        let x = f64::from(twip_px_unclamped(line.left_twip)?);
        let y = f64::from(twip_px_unclamped(line.top_twip)?);
        face.draw(&line.text, x, y, [0, 0, 0], |px, py, color, cov| {
            blend(rgb, width, height, px, py, color, cov);
        });
        Ok(())
    }
}

fn twip_px_unclamped(twip: i64) -> Result<u32, Error> {
    let n = u64::try_from(twip).map_err(|_| unsupported())?;
    Ok(((n + 7) / 15) as u32)
}

fn blend(rgb: &mut [u8], width: u32, height: u32, x: i32, y: i32, color: [u8; 3], cov: u8) {
    if cov == 0 || x < 0 || y < 0 || x >= width as i32 || y >= height as i32 {
        return;
    }
    let i = (y as usize * width as usize + x as usize) * 3;
    if cov == 255 {
        rgb[i..i + 3].copy_from_slice(&color);
        return;
    }
    let c = u16::from(cov);
    let inv = 255 - c;
    for (k, src) in color.into_iter().enumerate() {
        let dst = u16::from(rgb[i + k]);
        rgb[i + k] = ((u16::from(src) * c + 127) / 255 + (dst * inv + 127) / 255) as u8;
    }
}

fn encode_png(w: u32, h: u32, rgb: &[u8]) -> Result<Vec<u8>, Error> {
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, w, h);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(|_| unsupported())?;
    writer.write_image_data(rgb).map_err(|_| unsupported())?;
    drop(writer);
    Ok(out)
}

fn parse_page_id(id: &str) -> Result<u32, Error> {
    let Some(rest) = id.strip_prefix("page:") else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::ExportPageNeedsPage,
            },
            "id",
        ));
    };
    if rest.contains('/') || rest.is_empty() {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::ExportPageNeedsPage,
            },
            "id",
        ));
    }
    let n: u32 = rest.parse().map_err(|_| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::ExportPageNeedsPage,
            },
            "id",
        )
    })?;
    if n == 0 {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::ExportPageNeedsPage,
            },
            "id",
        ));
    }
    Ok(n)
}

fn emu_twip(emu: i64) -> Result<i64, Error> {
    if emu <= 0 {
        return Err(unsupported());
    }
    // 914400 EMU = 1440 twip ? /635?
    Ok(emu / 635)
}

fn emu_px(emu: i64) -> Result<u32, Error> {
    if emu <= 0 {
        return Err(unsupported());
    }
    let px = emu.saturating_mul(96) / 914_400;
    if !(1..=MAX_PX).contains(&(px as u64)) {
        return Err(unsupported());
    }
    Ok(px as u32)
}

/// ?? `wp:inline` + ??? PNG?? extent ?????????????????
fn plain_inline_image(
    pkg: &Package,
    doc_part: &str,
    drawing: &Element,
) -> Result<(i64, i64, u32, u32, Vec<u8>), Error> {
    let children: Vec<&Element> = drawing.element_children().collect();
    if children.len() != 1 || children[0].local_name() != "inline" {
        return Err(unsupported());
    }
    let inline = children[0];
    let extent = inline.child_by_local("extent").ok_or_else(unsupported)?;
    let cx: i64 = extent
        .attr_local("cx")
        .ok_or_else(unsupported)?
        .parse()
        .map_err(|_| unsupported())?;
    let cy: i64 = extent
        .attr_local("cy")
        .ok_or_else(unsupported)?
        .parse()
        .map_err(|_| unsupported())?;
    let width_px = emu_px(cx)?;
    let height_px = emu_px(cy)?;
    // ??????inline ??? dist ????? 0?? 0 ? NI?
    for key in ["distT", "distB", "distL", "distR"] {
        if let Some(v) = inline.attr_local(key) {
            if v != "0" {
                return Err(unsupported());
            }
        }
    }
    if inline.child_by_local("anchor").is_some() {
        return Err(unsupported());
    }
    // blipFill??? srcRect??? blip ? alpha?
    let pic = find_local_desc(inline, "pic").ok_or_else(unsupported)?;
    let blip_fill = pic.child_by_local("blipFill").ok_or_else(unsupported)?;
    if blip_fill.child_by_local("srcRect").is_some() {
        return Err(unsupported());
    }
    let blip = blip_fill.child_by_local("blip").ok_or_else(unsupported)?;
    if blip.element_children().any(|el| {
        let n = el.local_name();
        n == "alphaModFix" || n == "alpha" || n.contains("alpha")
    }) {
        return Err(unsupported());
    }
    let rid = super::picture::blip_embed(drawing).ok_or_else(unsupported)?;
    if super::picture::blip_link(drawing).is_some() {
        // ??? link ??????
        if super::picture::blip_embed(drawing).is_none() {
            return Err(unsupported());
        }
    }
    // ?? embed ???? link NI?
    let media = super::picture::resolve_embed(pkg, doc_part, &rid)?;
    let bytes = pkg.get(&media)?;
    let (sw, sh, rgb) = decode_opaque_png(bytes)?;
    if sw != width_px || sh != height_px {
        // ??????????? oracle 1:1??
        return Err(unsupported());
    }
    Ok((cx, cy, width_px, height_px, rgb))
}

fn find_local_desc<'a>(root: &'a Element, local: &str) -> Option<&'a Element> {
    if root.local_name() == local {
        return Some(root);
    }
    for child in root.element_children() {
        if let Some(found) = find_local_desc(child, local) {
            return Some(found);
        }
    }
    None
}

fn decode_opaque_png(bytes: &[u8]) -> Result<(u32, u32, Vec<u8>), Error> {
    let decoder = png::Decoder::new(Cursor::new(bytes));
    let mut reader = decoder.read_info().map_err(|_| unsupported())?;
    let info = reader.info();
    let w = info.width;
    let h = info.height;
    if !(1..=MAX_PX).contains(&(w as u64)) || !(1..=MAX_PX).contains(&(h as u64)) {
        return Err(unsupported());
    }
    let mut buf = vec![0u8; reader.output_buffer_size()];
    let frame = reader.next_frame(&mut buf).map_err(|_| unsupported())?;
    let data = &buf[..frame.buffer_size()];
    let mut rgb = Vec::with_capacity((w as usize) * (h as usize) * 3);
    match frame.color_type {
        png::ColorType::Rgb => {
            if frame.bit_depth != png::BitDepth::Eight {
                return Err(unsupported());
            }
            rgb.extend_from_slice(data);
        }
        png::ColorType::Rgba => {
            if frame.bit_depth != png::BitDepth::Eight {
                return Err(unsupported());
            }
            for chunk in data.as_chunks::<4>().0 {
                if chunk[3] != 255 {
                    return Err(unsupported());
                }
                rgb.extend_from_slice(&chunk[..3]);
            }
        }
        png::ColorType::Grayscale => {
            if frame.bit_depth != png::BitDepth::Eight {
                return Err(unsupported());
            }
            for &g in data {
                rgb.extend_from_slice(&[g, g, g]);
            }
        }
        png::ColorType::GrayscaleAlpha => {
            if frame.bit_depth != png::BitDepth::Eight {
                return Err(unsupported());
            }
            for chunk in data.as_chunks::<2>().0 {
                if chunk[1] != 255 {
                    return Err(unsupported());
                }
                rgb.extend_from_slice(&[chunk[0], chunk[0], chunk[0]]);
            }
        }
        _ => return Err(unsupported()),
    }
    if rgb.len() != (w as usize) * (h as usize) * 3 {
        return Err(unsupported());
    }
    Ok((w, h, rgb))
}

fn unsupported() -> Error {
    Error::with_path(ErrorKind::PageImageUnsupported, "id")
}
