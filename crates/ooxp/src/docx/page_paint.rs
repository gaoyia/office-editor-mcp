//! 把 docx 里没有可见内容的一页，或单节、单段（可换行）的不透明拉丁 Calibri 正文，画成 PNG。
//! 宽高按节的 `w:pgSz`，96 dpi，twip×96/1440 四舍五入。
//! 有表、图、页眉页脚，或还没对齐的行距/分页控制时不画。单节拉丁 Calibri 多段（可空段、段前段后、左缩进）与空格换行已画；字形灰度仍暂缓。
//! 有字页在 Windows 上走已有公开 GDI+ `AntiAliasGridFit` 路径。字形灰度按 C25 第 0 段暂缓，
//! 不把灰阶差算成整页通过，也不借用幻灯片文字的布局近似许可。

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
    /// 每页要画的行。下标 0 = page:1。
    pages: Vec<Vec<TextLine>>,
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
    let (story, sect) = body_story(body)?;
    let has_text = story.iter().any(|p| matches!(p, StoryPara::Text(_)));
    if !sect_is_plain(sect, has_text) || !uses_default_calibri(pkg, doc_part)? {
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
    pages: &mut Vec<Vec<TextLine>>,
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
    pages.push(Vec::new());
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
) -> Result<Vec<Vec<TextLine>>, Error> {
    // ??????????????floor(body_h * 2048 / 550000) ? ? ???
    // ?? 52 ????????????/???????
    let adv11 = line_advance_twip(DEFAULT_SZ)?;
    let lines_fit = (body_h as u64).saturating_mul(LINE_DEN) / LINE_NUM;
    let usable = (lines_fit as i64).saturating_mul(adv11).min(body_h);
    let page_bottom = margin_top.saturating_add(usable);
    let mut pages: Vec<Vec<TextLine>> = vec![Vec::new()];
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
                    pages.last_mut().ok_or_else(unsupported)?.push(TextLine {
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
}

fn body_story(body: &Element) -> Result<(Vec<StoryPara>, &Element), Error> {
    let mut story: Vec<StoryPara> = Vec::new();
    let mut sect: Option<&Element> = None;
    let children: Vec<&Element> = body.element_children().collect();
    for (index, child) in children.iter().copied().enumerate() {
        match child.local_name() {
            "p" => {
                if sect.is_some() {
                    return Err(unsupported());
                }
                story.push(paragraph_kind(child)?);
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

fn paragraph_kind(p: &Element) -> Result<StoryPara, Error> {
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
    if let Some(lines) = layout.pages.get(idx) {
        for line in lines {
            paint_text(&mut rgb, layout.width, layout.height, line)?;
        }
    }
    encode_png(layout.width, layout.height, &rgb)
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

fn unsupported() -> Error {
    Error::with_path(ErrorKind::PageImageUnsupported, "id")
}
