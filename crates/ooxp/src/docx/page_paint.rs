//! 把 docx 里没有可见内容的一页，或单节、单段（可换行）的正文（拉丁或东亚文字，系统字体），画成 PNG。
//! 宽高按节的 `w:pgSz`，按请求 DPI 将 twip×dpi/1440 四舍五入。省略或 96 时与历史 `twip×96/1440` 像素一致。
//! 磅与 EMU 也按请求 DPI 原生换算（磅×dpi/72；嵌入图 extent 为 EMU×dpi/914400 截断，与历史 96 dpi 截断一致）。
//! 不要套用 Excel 先按 96 量化再放大的行高规则。只影响本次 PNG，不改源包，也不改系统 DPI。
//! 单节拉丁或东亚文字多段（可空段、段前段后、左缩进；空格换行，东亚按字换行）已画；字形灰度仍暂缓。
//! 不透明 `wp:inline` PNG（extent 像素=源）与表（填色 + single/nil 边、tblGrid/tblInd/trHeight、默认单元格边距、表内拉丁或东亚文字）已画。
//! default 页眉/页脚拉丁或东亚文字（按 pgMar/@header 顶距、@footer 底距到内容底，正文不位移）已画；奇偶/首页/页眉页脚内图/表/域仍 NI。
//! 合并/表样式、浮动绕排、显式行距/分页控制、缩放图与文图混排仍不画。
//! 文字走已有 `Face`：Windows 上按系统字体名打开 GDI+ `AntiAliasGridFit`；非 Windows 先找系统字体，找不到再回退江城黑体子集。
//! 字形灰度按 C25 第 0 段暂缓，不把灰阶差算成整页通过，也不借用幻灯片文字的布局近似许可。
use std::io::Cursor;

use crate::error::{Error, ErrorKind, IdReason};
use crate::opc::{self, Package};
use crate::render_dpi::RenderDpi;
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

#[derive(Clone, Debug)]
struct RunFonts {
    ascii: String,
    east_asia: String,
}

impl Default for RunFonts {
    fn default() -> Self {
        Self {
            ascii: "Calibri".into(),
            east_asia: "宋体".into(),
        }
    }
}

fn is_east_asia_char(ch: char) -> bool {
    let u = ch as u32;
    matches!(
        u,
        0x1100..=0x11FF
            | 0x2E80..=0x9FFF
            | 0xA960..=0xA97F
            | 0xAC00..=0xD7AF
            | 0xF900..=0xFAFF
            | 0xFE10..=0xFE1F
            | 0xFE30..=0xFE4F
            | 0xFF00..=0xFFEF
            | 0x20000..=0x2FA1F
    )
}

fn reject_theme_font(name: &str) -> Result<(), Error> {
    if name.is_empty() || name.starts_with('+') {
        return Err(unsupported());
    }
    Ok(())
}

#[allow(dead_code)]
pub fn export_page_image(bytes: &[u8], id: &str) -> Result<ExportedMedia, Error> {
    export_page_image_at(bytes, id, RenderDpi::DEFAULT)
}

pub fn export_page_image_at(
    bytes: &[u8],
    id: &str,
    dpi: RenderDpi,
) -> Result<ExportedMedia, Error> {
    let page = parse_page_id(id)?;
    let pkg = Package::open_bytes(bytes.to_vec())?;
    let doc_part = opc::office_document_part(&pkg)?;
    let doc = xml::parse(pkg.get(&doc_part)?)?;
    let layout = layout_of(&pkg, &doc_part, &doc.root, dpi)?;
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
    dpi: RenderDpi,
    /// 下标 0 = page:1。
    pages: Vec<Page>,
    /// 每页重复绘制的 default 页眉文字（无则空）。
    header_texts: Vec<TextLine>,
    /// 每页重复绘制的 default 页脚文字（无则空）。
    footer_texts: Vec<TextLine>,
}

struct Page {
    texts: Vec<TextLine>,
    images: Vec<ImageBlit>,
    tables: Vec<TableBlit>,
}

struct TableBlit {
    left_twip: i64,
    top_twip: i64,
    col_widths: Vec<i64>,
    row_heights: Vec<i64>,
    /// (row, col, rgb)
    fills: Vec<(usize, usize, [u8; 3])>,
    /// 四边+内外：true=画 1px 黑单线（本切片只认 single）。
    border_top: bool,
    border_left: bool,
    border_bottom: bool,
    border_right: bool,
    border_inside_h: bool,
    border_inside_v: bool,
}

struct ImageBlit {
    left_twip: i64,
    top_twip: i64,
    width_px: u32,
    height_px: u32,
    /// 不透明 RGB，长度 width*height*3。
    rgb: Vec<u8>,
}

#[derive(Clone)]
struct TextLine {
    text: String,
    sz: u32,
    left_twip: i64,
    top_twip: i64,
    body_w_twip: i64,
    fonts: RunFonts,
}

fn layout_of(
    pkg: &Package,
    doc_part: &str,
    root: &Element,
    dpi: RenderDpi,
) -> Result<Layout, Error> {
    if root.element_children().any(|el| el.local_name() != "body") {
        return Err(unsupported());
    }
    let body = root.child_by_local("body").ok_or_else(unsupported)?;
    let defaults = parse_plain_defaults(pkg, doc_part)?;
    let (story, sect) = body_story(pkg, doc_part, body, dpi, &defaults)?;
    let has_text = story.iter().any(|p| matches!(p, StoryPara::Text(_)));
    let has_image = story.iter().any(|p| matches!(p, StoryPara::Image(_)));
    let has_table = story.iter().any(|p| matches!(p, StoryPara::Table(_)));
    if !sect_is_plain(sect, has_text || has_image || has_table) {
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
    let width = twip_px(page_w, dpi)?;
    let height = twip_px(page_h, dpi)?;
    let pages = layout_story(&story, left, top, body_w, body_h, dpi)?;
    if pages.is_empty() {
        return Err(unsupported());
    }
    // 页眉/页脚在边距区域内绘制，不把正文 y 位移（oracle：blank 页眉时正文 A 仍在 101）。
    let header_texts =
        layout_default_header(pkg, doc_part, sect, left, body_w, page_h, dpi, &defaults)?;
    let footer_texts =
        layout_default_footer(pkg, doc_part, sect, left, body_w, page_h, dpi, &defaults)?;
    Ok(Layout {
        width,
        height,
        dpi,
        pages,
        header_texts,
        footer_texts,
    })
}

/// `w:pgMar/@w:header`：页顶到页眉内容顶。缺省 708（Word 默认半英寸），不是正文 top 的 1440。
fn header_distance_twip(sect: &Element) -> Result<i64, Error> {
    hf_distance_twip(sect, "header")
}

/// `w:pgMar/@w:footer`：页底到页脚内容底（ECMA 17.6.11）。缺省 708，不是正文 bottom 的 1440。
fn footer_distance_twip(sect: &Element) -> Result<i64, Error> {
    hf_distance_twip(sect, "footer")
}

fn hf_distance_twip(sect: &Element, local: &str) -> Result<i64, Error> {
    let Some(mar) = sect.child_by_local("pgMar") else {
        return Ok(708);
    };
    match mar.attr_local(local) {
        None => Ok(708),
        Some(raw) => {
            let n = parse_twip(raw)?;
            if n < 0 {
                return Err(unsupported());
            }
            Ok(n)
        }
    }
}

/// 解析并排版唯一的 default 页眉。无引用 → 空；有图/表/域/奇偶首页 → NI。
fn layout_default_header(
    pkg: &Package,
    doc_part: &str,
    sect: &Element,
    margin_left: i64,
    body_w: i64,
    page_h: i64,
    dpi: RenderDpi,
    defaults: &RunFonts,
) -> Result<Vec<TextLine>, Error> {
    if super::hf::even_and_odd_headers(pkg, doc_part)? {
        return Err(unsupported());
    }
    let rels = crate::opc::Relationships::from_package(pkg, doc_part)?;
    let map = super::hf::explicit_refs(
        sect,
        "headerReference",
        super::hf::HfKind::Header,
        doc_part,
        &rels,
    )?;
    if map.contains_key(&super::hf::HfType::Even) || map.contains_key(&super::hf::HfType::First) {
        return Err(unsupported());
    }
    let Some(part) = map.get(&super::hf::HfType::Default).cloned() else {
        return Ok(Vec::new());
    };
    let header_top = header_distance_twip(sect)?;
    if header_top >= page_h {
        return Err(unsupported());
    }
    let bytes = pkg.get(&part)?;
    let doc = xml::parse(bytes)?;
    if doc.root.local_name() != "hdr" {
        return Err(unsupported());
    }
    let story = hf_plain_story(pkg, &part, &doc.root, dpi, defaults)?;
    if story.is_empty() {
        return Ok(Vec::new());
    }
    // 页眉不跨页；可用高度为页高减去页眉顶距。
    let avail = page_h.saturating_sub(header_top);
    let pages = layout_story(&story, margin_left, header_top, body_w, avail, dpi)?;
    if pages.len() != 1 {
        return Err(unsupported());
    }
    let page = &pages[0];
    if !page.images.is_empty() || !page.tables.is_empty() {
        return Err(unsupported());
    }
    Ok(page.texts.clone())
}

fn layout_default_footer(
    pkg: &Package,
    doc_part: &str,
    sect: &Element,
    margin_left: i64,
    body_w: i64,
    page_h: i64,
    dpi: RenderDpi,
    defaults: &RunFonts,
) -> Result<Vec<TextLine>, Error> {
    if super::hf::even_and_odd_headers(pkg, doc_part)? {
        return Err(unsupported());
    }
    let rels = crate::opc::Relationships::from_package(pkg, doc_part)?;
    let map = super::hf::explicit_refs(
        sect,
        "footerReference",
        super::hf::HfKind::Footer,
        doc_part,
        &rels,
    )?;
    if map.contains_key(&super::hf::HfType::Even) || map.contains_key(&super::hf::HfType::First) {
        return Err(unsupported());
    }
    let Some(part) = map.get(&super::hf::HfType::Default).cloned() else {
        return Ok(Vec::new());
    };
    let footer_dist = footer_distance_twip(sect)?;
    if footer_dist <= 0 || footer_dist >= page_h {
        return Err(unsupported());
    }
    let bytes = pkg.get(&part)?;
    let doc = xml::parse(bytes)?;
    if doc.root.local_name() != "ftr" {
        return Err(unsupported());
    }
    let story = hf_plain_story(pkg, &part, &doc.root, dpi, defaults)?;
    if story.is_empty() {
        return Ok(Vec::new());
    }
    // 先在页顶排版量高度，再把内容底对齐到 page_h - @footer；向上长，不把正文原点上推。
    let pages = layout_story(&story, margin_left, 0, body_w, page_h, dpi)?;
    if pages.len() != 1 {
        return Err(unsupported());
    }
    let page = &pages[0];
    if !page.images.is_empty() || !page.tables.is_empty() {
        return Err(unsupported());
    }
    if page.texts.is_empty() {
        return Ok(Vec::new());
    }
    let mut content_bottom = 0i64;
    for line in &page.texts {
        let bottom = line.top_twip.saturating_add(line_advance_twip(line.sz)?);
        if bottom > content_bottom {
            content_bottom = bottom;
        }
    }
    let target_bottom = page_h - footer_dist;
    let shift = target_bottom - content_bottom;
    let mut texts = page.texts.clone();
    for line in &mut texts {
        let top = line.top_twip.checked_add(shift).ok_or_else(unsupported)?;
        if top < 0 {
            return Err(unsupported());
        }
        line.top_twip = top;
    }
    Ok(texts)
}

fn hf_plain_story(
    pkg: &Package,
    part: &str,
    root: &Element,
    dpi: RenderDpi,
    defaults: &RunFonts,
) -> Result<Vec<StoryPara>, Error> {
    let mut story = Vec::new();
    for child in root.element_children() {
        match child.local_name() {
            "p" => {
                let para = paragraph_kind(pkg, part, child, dpi, defaults)?;
                match para {
                    StoryPara::Image(_) | StoryPara::Table(_) => return Err(unsupported()),
                    other => story.push(other),
                }
            }
            // 表/图/内容控件/域等：明确 NI，不静默丢。
            _ => return Err(unsupported()),
        }
    }
    Ok(story)
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
        tables: Vec::new(),
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
    dpi: RenderDpi,
) -> Result<Vec<Page>, Error> {
    // ??????????????floor(body_h * 2048 / 550000) ? ? ???
    let adv11 = line_advance_twip(DEFAULT_SZ)?;
    let lines_fit = (body_h as u64).saturating_mul(LINE_DEN) / LINE_NUM;
    let usable = (lines_fit as i64).saturating_mul(adv11).min(body_h);
    let page_bottom = margin_top.saturating_add(usable);
    let mut pages: Vec<Page> = vec![Page {
        texts: Vec::new(),
        images: Vec::new(),
        tables: Vec::new(),
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
            StoryPara::Table(table) => {
                let row_heights = table_row_heights(table, dpi)?;
                let table_h: i64 = row_heights.iter().copied().sum();
                if table_h <= 0 || table.col_widths.is_empty() || row_heights.is_empty() {
                    return Err(unsupported());
                }
                ensure_space(
                    &mut pages,
                    &mut y,
                    &mut at_page_top,
                    margin_top,
                    page_bottom,
                    table_h,
                )?;
                let left_twip = margin_left.saturating_add(table.indent_twip.max(0));
                let table_w: i64 = table.col_widths.iter().copied().sum();
                if table_w > body_w.saturating_sub(table.indent_twip.max(0)) {
                    return Err(unsupported());
                }
                let texts = table_text_lines(table, left_twip, y, &row_heights, dpi)?;
                let page = pages.last_mut().ok_or_else(unsupported)?;
                page.texts.extend(texts);
                page.tables.push(TableBlit {
                    left_twip,
                    top_twip: y,
                    col_widths: table.col_widths.clone(),
                    row_heights,
                    fills: table.fills.clone(),
                    border_top: table.border_top,
                    border_left: table.border_left,
                    border_bottom: table.border_bottom,
                    border_right: table.border_right,
                    border_inside_h: table.border_inside_h,
                    border_inside_v: table.border_inside_v,
                });
                y = y.saturating_add(table_h);
                at_page_top = false;
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
                let rows = wrap_text_lines(&line.text, line.sz, content_w, dpi, &line.fonts)?;
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
                            fonts: line.fonts.clone(),
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

fn wrap_text_lines(
    text: &str,
    sz: u32,
    body_w_twip: i64,
    dpi: RenderDpi,
    fonts: &RunFonts,
) -> Result<Vec<String>, Error> {
    let ascii = open_run_face(sz, dpi, &fonts.ascii)?;
    let east = if fonts.east_asia == fonts.ascii {
        None
    } else {
        Some(open_run_face(sz, dpi, &fonts.east_asia)?)
    };
    let max_w = f64::from(twip_px_unclamped(body_w_twip, dpi)?);
    if typo_width_mixed(text, &ascii, east.as_ref()) <= max_w {
        return Ok(vec![text.to_string()]);
    }
    // ASCII 空格贪心：断在最后一个仍放得下的空格；断点空格留在上行（与 Word 16.0 一致）。
    // 东亚文字在没有空格时按字断开。不 collapse 多空格。
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
        if typo_width_mixed(&candidate, &ascii, east.as_ref()) <= max_w {
            i += 1;
            continue;
        }
        if let Some(sp) = last_space.filter(|&sp| sp >= start) {
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
            if chars[start] != ' ' {
                let mut j = start;
                while j < chars.len() && chars[j] != ' ' {
                    j += 1;
                }
                let word: String = chars[start..j].iter().collect();
                if typo_width_mixed(&word, &ascii, east.as_ref()) > max_w {
                    // 整词超宽：若是东亚则按字切，拉丁词仍 NI（与历史一致）。
                    if !word.chars().any(is_east_asia_char) {
                        return Err(unsupported());
                    }
                }
            }
            continue;
        }
        if i > start && (is_east_asia_char(chars[i]) || is_east_asia_char(chars[i - 1])) {
            let line: String = chars[start..i].iter().collect();
            lines.push(line);
            start = i;
            last_space = None;
            continue;
        }
        return Err(unsupported());
    }
    if start < chars.len() {
        lines.push(chars[start..].iter().collect());
    }
    if lines.is_empty() {
        return Err(unsupported());
    }
    Ok(lines)
}

fn typo_width_mixed(text: &str, ascii: &Face, east: Option<&Face>) -> f64 {
    text.chars()
        .map(|ch| face_for_char(ch, ascii, east).typo_width(&ch.to_string()))
        .sum()
}

fn face_for_char<'a>(ch: char, ascii: &'a Face, east: Option<&'a Face>) -> &'a Face {
    if is_east_asia_char(ch) {
        east.unwrap_or(ascii)
    } else {
        ascii
    }
}

fn open_run_face(sz: u32, dpi: RenderDpi, name: &str) -> Result<Face, Error> {
    reject_theme_font(name)?;
    let pt = (sz as f32) / 2.0;
    let px = dpi.pt_to_px_font(f64::from(pt)) as f32;
    Face::open_named(px, false, name).ok_or_else(unsupported)
}

struct PendingText {
    text: String,
    sz: u32,
    indent_twip: i64,
    before_twip: i64,
    after_twip: i64,
    fonts: RunFonts,
}

enum StoryPara {
    Empty { before: i64, after: i64 },
    Text(PendingText),
    Image(PendingImage),
    Table(PendingTable),
}

struct PendingTable {
    indent_twip: i64,
    col_widths: Vec<i64>,
    margin_top: i64,
    margin_left: i64,
    margin_bottom: i64,
    margin_right: i64,
    rows: Vec<PendingRowBuild>,
    fills: Vec<(usize, usize, [u8; 3])>,
    border_top: bool,
    border_left: bool,
    border_bottom: bool,
    border_right: bool,
    border_inside_h: bool,
    border_inside_v: bool,
}

enum RowHeight {
    Auto,
    AtLeast(i64),
    Exact(i64),
}

enum CellPara {
    Empty { before: i64, after: i64 },
    Text(PendingText),
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
    dpi: RenderDpi,
    defaults: &RunFonts,
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
                story.push(paragraph_kind(pkg, doc_part, child, dpi, defaults)?);
            }
            "tbl" => {
                if sect.is_some() {
                    return Err(unsupported());
                }
                story.push(StoryPara::Table(parse_plain_table(
                    pkg, doc_part, child, dpi, defaults,
                )?));
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

fn paragraph_kind(
    pkg: &Package,
    doc_part: &str,
    p: &Element,
    dpi: RenderDpi,
    defaults: &RunFonts,
) -> Result<StoryPara, Error> {
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
        let img = plain_inline_image(pkg, doc_part, drawing, dpi)?;
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
    let (text, sz, fonts) = plain_run(run, defaults)?;
    if text.is_empty() {
        return Ok(StoryPara::Empty { before, after });
    }
    Ok(StoryPara::Text(PendingText {
        text,
        sz,
        indent_twip: indent,
        before_twip: before,
        after_twip: after,
        fonts,
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
fn plain_run(run: &Element, defaults: &RunFonts) -> Result<(String, u32, RunFonts), Error> {
    let mut sz = DEFAULT_SZ;
    let mut fonts = defaults.clone();
    let mut text = String::new();
    let mut saw_text = false;
    for child in run.element_children() {
        match child.local_name() {
            "rPr" => parse_plain_rpr(child, &mut sz, &mut fonts)?,
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
    Ok((text, sz, fonts))
}

fn parse_plain_rpr(rpr: &Element, sz: &mut u32, fonts: &mut RunFonts) -> Result<(), Error> {
    let mut sz_cs: Option<u32> = None;
    for child in rpr.element_children() {
        match child.local_name() {
            "sz" => *sz = parse_sz(child)?,
            "szCs" => sz_cs = Some(parse_sz(child)?),
            "rFonts" => {
                if child.element_children().next().is_some() {
                    return Err(unsupported());
                }
                apply_rfonts_attrs(child, fonts)?;
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
            // 本切片只认 default 页眉/页脚；奇偶/首页仍 NI。
            "headerReference" | "footerReference" => {
                child.attr_local("type").unwrap_or("default") == "default"
            }
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

fn parse_plain_defaults(pkg: &Package, doc_part: &str) -> Result<RunFonts, Error> {
    let Some(part) = styles::styles_part(pkg, doc_part)? else {
        return Err(unsupported());
    };
    let styles = xml::parse(pkg.get(&part)?)?;
    let root = &styles.root;
    let Some(defaults) = root.child_by_local("docDefaults") else {
        return Err(unsupported());
    };
    if let Some(ppr_default) = defaults.child_by_local("pPrDefault") {
        if let Some(ppr) = ppr_default.child_by_local("pPr") {
            if ppr.element_children().next().is_some() {
                return Err(unsupported());
            }
        }
        if ppr_default
            .element_children()
            .any(|el| el.local_name() != "pPr")
        {
            return Err(unsupported());
        }
    }
    let Some(rpr) = defaults
        .child_by_local("rPrDefault")
        .and_then(|el| el.child_by_local("rPr"))
    else {
        return Err(unsupported());
    };
    if rpr.child_by_local("sz").and_then(|el| el.attr_local("val")) != Some("22") {
        return Err(unsupported());
    }
    let mut fonts = RunFonts::default();
    if let Some(rfonts) = rpr.child_by_local("rFonts") {
        // 只读属性，复用 run 解析；缺省值已在 fonts 里。
        parse_plain_rpr_fonts_only(rfonts, &mut fonts)?;
    }
    let Some(normal) = root.element_children().find(|el| {
        el.local_name() == "style"
            && el.attr_local("type") == Some("paragraph")
            && el.attr_local("default") == Some("1")
    }) else {
        return Err(unsupported());
    };
    if let Some(ppr) = normal.child_by_local("pPr") {
        if ppr.element_children().next().is_some() {
            return Err(unsupported());
        }
    }
    Ok(fonts)
}

fn parse_plain_rpr_fonts_only(rfonts: &Element, fonts: &mut RunFonts) -> Result<(), Error> {
    if rfonts.element_children().next().is_some() {
        return Err(unsupported());
    }
    apply_rfonts_attrs(rfonts, fonts)
}

fn apply_rfonts_attrs(rfonts: &Element, fonts: &mut RunFonts) -> Result<(), Error> {
    let mut ascii: Option<String> = None;
    let mut h_ansi: Option<String> = None;
    for (name, value) in rfonts.attrs.iter() {
        let local = local_of(name);
        match local {
            "ascii" => {
                reject_theme_font(value)?;
                ascii = Some(value.to_string());
            }
            "hAnsi" => {
                reject_theme_font(value)?;
                h_ansi = Some(value.to_string());
            }
            "eastAsia" => {
                reject_theme_font(value)?;
                fonts.east_asia = value.to_string();
            }
            "cs" | "hint" => {}
            _ => return Err(unsupported()),
        }
    }
    match (ascii, h_ansi) {
        (Some(a), Some(h)) if a != h => return Err(unsupported()),
        (Some(a), _) => fonts.ascii = a,
        (None, Some(h)) => fonts.ascii = h,
        (None, None) => {}
    }
    Ok(())
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

fn twip_px(twip: i64, dpi: RenderDpi) -> Result<u32, Error> {
    let px = dpi.twip_to_px_round(twip);
    let px = u64::try_from(px).map_err(|_| unsupported())?;
    if !(1..=MAX_PX).contains(&px) {
        return Err(unsupported());
    }
    Ok(px as u32)
}

fn paint_page(layout: &Layout, page: u32) -> Result<Vec<u8>, Error> {
    let mut rgb = vec![255u8; (layout.width as usize) * (layout.height as usize) * 3];
    let idx = (page as usize).saturating_sub(1);
    if let Some(page_items) = layout.pages.get(idx) {
        for tbl in &page_items.tables {
            paint_table(&mut rgb, layout.width, layout.height, tbl, layout.dpi)?;
        }
        for img in &page_items.images {
            paint_image(&mut rgb, layout.width, layout.height, img, layout.dpi)?;
        }
        for line in &layout.header_texts {
            paint_text(&mut rgb, layout.width, layout.height, line, layout.dpi)?;
        }
        for line in &layout.footer_texts {
            paint_text(&mut rgb, layout.width, layout.height, line, layout.dpi)?;
        }
        for line in &page_items.texts {
            paint_text(&mut rgb, layout.width, layout.height, line, layout.dpi)?;
        }
    }
    encode_png(layout.width, layout.height, &rgb, layout.dpi)
}

fn paint_image(
    rgb: &mut [u8],
    page_w: u32,
    page_h: u32,
    img: &ImageBlit,
    dpi: RenderDpi,
) -> Result<(), Error> {
    let x0 = twip_px_unclamped(img.left_twip, dpi)? as i32;
    let y0 = twip_px_unclamped(img.top_twip, dpi)? as i32;
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

fn paint_text(
    rgb: &mut [u8],
    width: u32,
    height: u32,
    line: &TextLine,
    dpi: RenderDpi,
) -> Result<(), Error> {
    let ascii = open_run_face(line.sz, dpi, &line.fonts.ascii)?;
    let east = if line.fonts.east_asia == line.fonts.ascii {
        None
    } else {
        Some(open_run_face(line.sz, dpi, &line.fonts.east_asia)?)
    };
    let body_w = f64::from(twip_px_unclamped(line.body_w_twip, dpi)?);
    if typo_width_mixed(&line.text, &ascii, east.as_ref()) > body_w {
        return Err(unsupported());
    }
    let mut x = f64::from(twip_px_unclamped(line.left_twip, dpi)?);
    let y = f64::from(twip_px_unclamped(line.top_twip, dpi)?);
    let mut buf = String::new();
    let mut current_is_ea: Option<bool> = None;
    for ch in line.text.chars() {
        let is_ea = is_east_asia_char(ch);
        if current_is_ea != Some(is_ea) && !buf.is_empty() {
            let face = if current_is_ea == Some(true) {
                east.as_ref().unwrap_or(&ascii)
            } else {
                &ascii
            };
            let adv = face.typo_width(&buf);
            face.draw(&buf, x, y, [0, 0, 0], |px, py, color, cov| {
                blend(rgb, width, height, px, py, color, cov);
            });
            x += adv;
            buf.clear();
        }
        current_is_ea = Some(is_ea);
        buf.push(ch);
    }
    if !buf.is_empty() {
        let face = if current_is_ea == Some(true) {
            east.as_ref().unwrap_or(&ascii)
        } else {
            &ascii
        };
        face.draw(&buf, x, y, [0, 0, 0], |px, py, color, cov| {
            blend(rgb, width, height, px, py, color, cov);
        });
    }
    Ok(())
}

fn twip_px_unclamped(twip: i64, dpi: RenderDpi) -> Result<u32, Error> {
    let px = dpi.twip_to_px_round(twip);
    u32::try_from(px).map_err(|_| unsupported())
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

fn encode_png(w: u32, h: u32, rgb: &[u8], dpi: RenderDpi) -> Result<Vec<u8>, Error> {
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, w, h);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    // 仅非默认 DPI 写入 pHYs，使省略 dpi / dpi=96 的既有 PNG 字节保持不变。
    if !dpi.is_default() {
        let ppm = (dpi.get() / 0.0254).round() as u32;
        encoder.set_pixel_dims(Some(png::PixelDimensions {
            xppu: ppm,
            yppu: ppm,
            unit: png::Unit::Meter,
        }));
    }
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

fn emu_px(emu: i64, dpi: RenderDpi) -> Result<u32, Error> {
    if emu <= 0 {
        return Err(unsupported());
    }
    // Word 嵌入图 extent：按请求 DPI 整数截断，不是四舍五入。96 dpi 时与历史 `emu*96/914400` 一致。
    let px = (emu as i128) * (dpi.get() as i128) / 914_400;
    if px < 1 || px > MAX_PX as i128 {
        return Err(unsupported());
    }
    Ok(px as u32)
}

/// ?? `wp:inline` + ??? PNG?? extent ?????????????????
fn plain_inline_image(
    pkg: &Package,
    doc_part: &str,
    drawing: &Element,
    dpi: RenderDpi,
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
    let width_px = emu_px(cx, dpi)?;
    let height_px = emu_px(cy, dpi)?;
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

/// 空单元格自动行高（仅 DEFAULT_SZ / 空单元格路径）。
/// Word COM：默认表 Top/Bottom padding=0、Left/Right=5.4pt(=108twip)；空单元格段落
/// Font=11、SpaceBefore=0，LineSpacing≈13.9pt（≈278–279 twip）。
/// TTF 单倍 `line_advance(22)=268` twip；与 Word 单元格行框差 **11 twip（0.55pt）**。
/// 96dpi PDF 横线累计验证 2–5 行：114/132/151/170/188/207。非 Calibri11 空行不走此常数。
fn empty_auto_row_twip() -> Result<i64, Error> {
    Ok(line_advance_twip(DEFAULT_SZ)?.saturating_add(11))
}

/// 单元格行框：默认 11 磅走已对照过的空单元格自动行高（TTF 单倍 + 11 twip）；其它字号用 TTF 单倍。
fn cell_line_box_twip(sz: u32) -> Result<i64, Error> {
    if sz == DEFAULT_SZ {
        empty_auto_row_twip()
    } else {
        line_advance_twip(sz)
    }
}

fn parse_tbl_cell_mar(
    el: &Element,
    top: &mut i64,
    left: &mut i64,
    bottom: &mut i64,
    right: &mut i64,
) -> Result<(), Error> {
    for side in el.element_children() {
        let raw = side.attr_local("w").ok_or_else(unsupported)?;
        let n = parse_twip(raw)?;
        if n < 0 {
            return Err(unsupported());
        }
        let ty = side.attr_local("type").unwrap_or("dxa");
        if ty != "dxa" {
            return Err(unsupported());
        }
        match side.local_name() {
            "top" => *top = n,
            "left" | "start" => *left = n,
            "bottom" => *bottom = n,
            "right" | "end" => *right = n,
            _ => return Err(unsupported()),
        }
    }
    Ok(())
}

fn inner_cell_width(col_w: i64, table: &PendingTable, indent: i64) -> Result<i64, Error> {
    let inner = col_w
        .saturating_sub(table.margin_left.max(0))
        .saturating_sub(table.margin_right.max(0))
        .saturating_sub(indent.max(0));
    if inner <= 0 {
        return Err(unsupported());
    }
    Ok(inner)
}

fn cell_paras_height(
    paras: &[CellPara],
    col_w: i64,
    table: &PendingTable,
    dpi: RenderDpi,
) -> Result<i64, Error> {
    let mut height = 0i64;
    for para in paras {
        match para {
            CellPara::Empty { before, after } => {
                height = height
                    .saturating_add((*before).max(0))
                    .saturating_add(cell_line_box_twip(DEFAULT_SZ)?)
                    .saturating_add((*after).max(0));
            }
            CellPara::Text(line) => {
                let content_w = inner_cell_width(col_w, table, line.indent_twip)?;
                let rows = wrap_text_lines(&line.text, line.sz, content_w, dpi, &line.fonts)?;
                let n = i64::try_from(rows.len()).map_err(|_| unsupported())?;
                if n <= 0 {
                    return Err(unsupported());
                }
                height = height
                    .saturating_add(line.before_twip.max(0))
                    .saturating_add(cell_line_box_twip(line.sz)?.saturating_mul(n))
                    .saturating_add(line.after_twip.max(0));
            }
        }
    }
    Ok(height)
}

fn table_row_heights(table: &PendingTable, dpi: RenderDpi) -> Result<Vec<i64>, Error> {
    if table.rows.is_empty() || table.col_widths.is_empty() {
        return Err(unsupported());
    }
    let mut heights = Vec::with_capacity(table.rows.len());
    for row in &table.rows {
        if row.cells.len() != table.col_widths.len() {
            return Err(unsupported());
        }
        let mut content = 0i64;
        for (ci, cell) in row.cells.iter().enumerate() {
            let h = cell_paras_height(&cell.paras, table.col_widths[ci], table, dpi)?;
            content = content.max(h);
        }
        let with_mar = content
            .saturating_add(table.margin_top.max(0))
            .saturating_add(table.margin_bottom.max(0));
        let height = match row.height {
            RowHeight::Auto => with_mar,
            RowHeight::AtLeast(n) => with_mar.max(n),
            RowHeight::Exact(n) => n,
        };
        if height <= 0 {
            return Err(unsupported());
        }
        heights.push(height);
    }
    Ok(heights)
}

fn table_text_lines(
    table: &PendingTable,
    table_left: i64,
    table_top: i64,
    row_heights: &[i64],
    dpi: RenderDpi,
) -> Result<Vec<TextLine>, Error> {
    let mut texts = Vec::new();
    let mut row_top = table_top;
    for (ri, row) in table.rows.iter().enumerate() {
        let row_h = *row_heights.get(ri).ok_or_else(unsupported)?;
        let row_bottom = row_top.saturating_add(row_h);
        let mut cell_left = table_left;
        for (ci, cell) in row.cells.iter().enumerate() {
            let col_w = *table.col_widths.get(ci).ok_or_else(unsupported)?;
            let mut y = row_top.saturating_add(table.margin_top.max(0));
            for para in &cell.paras {
                match para {
                    CellPara::Empty { before, after } => {
                        y = y
                            .saturating_add((*before).max(0))
                            .saturating_add(cell_line_box_twip(DEFAULT_SZ)?)
                            .saturating_add((*after).max(0));
                    }
                    CellPara::Text(line) => {
                        let content_w = inner_cell_width(col_w, table, line.indent_twip)?;
                        let left_twip = cell_left
                            .saturating_add(table.margin_left.max(0))
                            .saturating_add(line.indent_twip.max(0));
                        let rows =
                            wrap_text_lines(&line.text, line.sz, content_w, dpi, &line.fonts)?;
                        let adv = cell_line_box_twip(line.sz)?;
                        y = y.saturating_add(line.before_twip.max(0));
                        for row_text in rows {
                            // exact 行高时裁掉超出行底的行，避免盖住下一行。
                            if y.saturating_add(adv) > row_bottom {
                                break;
                            }
                            texts.push(TextLine {
                                text: row_text,
                                sz: line.sz,
                                left_twip,
                                top_twip: y,
                                body_w_twip: content_w,
                                fonts: line.fonts.clone(),
                            });
                            y = y.saturating_add(adv);
                        }
                        y = y.saturating_add(line.after_twip.max(0));
                    }
                }
            }
            cell_left = cell_left.saturating_add(col_w);
        }
        row_top = row_bottom;
    }
    Ok(texts)
}

fn parse_plain_table(
    pkg: &Package,
    doc_part: &str,
    tbl: &Element,
    dpi: RenderDpi,
    defaults: &RunFonts,
) -> Result<PendingTable, Error> {
    let mut indent = 0i64;
    let mut margin_top = 0i64;
    let mut margin_left = 108i64;
    let mut margin_bottom = 0i64;
    let mut margin_right = 108i64;
    let mut tbl_w_auto = true;
    let mut col_widths: Vec<i64> = Vec::new();
    let mut border_top = false;
    let mut border_left = false;
    let mut border_bottom = false;
    let mut border_right = false;
    let mut border_inside_h = false;
    let mut border_inside_v = false;
    let mut saw_borders = false;
    let mut rows: Vec<PendingRowBuild> = Vec::new();

    for child in tbl.element_children() {
        match child.local_name() {
            "tblPr" => {
                for p in child.element_children() {
                    match p.local_name() {
                        "tblLook" | "jc" => {}
                        "tblW" => {
                            // 缺省或 type=auto：列宽认各列一致的 dxa tcW（Word 会忽略过期 tblGrid）。
                            // 官方 wide PDF：grid 2880+720，但 tcW 均为 2880，竖线在 96/288/480。
                            let ty = p.attr_local("type").unwrap_or("auto");
                            match ty {
                                "auto" => tbl_w_auto = true,
                                "dxa" => tbl_w_auto = false,
                                _ => return Err(unsupported()),
                            }
                        }
                        "tblInd" => {
                            let w = p.attr_local("w").ok_or_else(unsupported)?;
                            indent = parse_twip(w)?;
                            let ty = p.attr_local("type").unwrap_or("dxa");
                            if ty != "dxa" {
                                return Err(unsupported());
                            }
                        }
                        "tblBorders" => {
                            saw_borders = true;
                            for b in p.element_children() {
                                let on = border_is_single_black(b)?;
                                match b.local_name() {
                                    "top" => border_top = on,
                                    "left" | "start" => border_left = on,
                                    "bottom" => border_bottom = on,
                                    "right" | "end" => border_right = on,
                                    "insideH" => border_inside_h = on,
                                    "insideV" => border_inside_v = on,
                                    "tl2br" | "tr2bl" => {
                                        if on {
                                            return Err(unsupported());
                                        }
                                    }
                                    _ => return Err(unsupported()),
                                }
                            }
                        }
                        "tblCellMar" => parse_tbl_cell_mar(
                            p,
                            &mut margin_top,
                            &mut margin_left,
                            &mut margin_bottom,
                            &mut margin_right,
                        )?,
                        "tblLayout" => {
                            let ty = p.attr_local("type").unwrap_or("autofit");
                            if ty != "autofit" && ty != "fixed" {
                                return Err(unsupported());
                            }
                        }
                        "tblStyle" | "tblOverlap" | "shd" | "tblCaption" | "tblDescription" => {
                            return Err(unsupported());
                        }
                        _ => return Err(unsupported()),
                    }
                }
            }
            "tblGrid" => {
                for g in child.element_children() {
                    if g.local_name() != "gridCol" {
                        return Err(unsupported());
                    }
                    let w = g.attr_local("w").ok_or_else(unsupported)?;
                    col_widths.push(parse_twip(w)?);
                }
            }
            "tr" => rows.push(parse_plain_row(
                pkg,
                doc_part,
                child,
                col_widths.len(),
                dpi,
                defaults,
            )?),
            "bookmarkStart" | "bookmarkEnd" => {}
            _ => return Err(unsupported()),
        }
    }
    if col_widths.is_empty() || rows.is_empty() {
        return Err(unsupported());
    }
    for row in &rows {
        if row.cells.len() != col_widths.len() {
            return Err(unsupported());
        }
    }
    if tbl_w_auto {
        if let Some(preferred) = preferred_auto_col_widths(&rows, col_widths.len()) {
            col_widths = preferred;
        }
    }
    // 缺省建表骨架带 single 边；若没有任何 tblBorders，按无边处理。
    if !saw_borders {
        border_top = false;
        border_left = false;
        border_bottom = false;
        border_right = false;
        border_inside_h = false;
        border_inside_v = false;
    }
    let mut fills = Vec::new();
    for (ri, row) in rows.iter().enumerate() {
        for (ci, cell) in row.cells.iter().enumerate() {
            if let Some(rgb) = cell.fill {
                fills.push((ri, ci, rgb));
            }
        }
    }
    Ok(PendingTable {
        indent_twip: indent,
        col_widths,
        margin_top,
        margin_left,
        margin_bottom,
        margin_right,
        rows,
        fills,
        border_top,
        border_left,
        border_bottom,
        border_right,
        border_inside_h,
        border_inside_v,
    })
}

struct PendingRowBuild {
    height: RowHeight,
    cells: Vec<PendingCellBuild>,
}

struct PendingCellBuild {
    fill: Option<[u8; 3]>,
    tc_w: Option<i64>,
    paras: Vec<CellPara>,
}

fn parse_plain_row(
    pkg: &Package,
    doc_part: &str,
    tr: &Element,
    expect_cols: usize,
    dpi: RenderDpi,
    defaults: &RunFonts,
) -> Result<PendingRowBuild, Error> {
    let mut height = RowHeight::Auto;
    let mut cells: Vec<PendingCellBuild> = Vec::new();
    for child in tr.element_children() {
        match child.local_name() {
            "trPr" => {
                for p in child.element_children() {
                    match p.local_name() {
                        "trHeight" => {
                            let h = p
                                .attr_local("val")
                                .or_else(|| p.attr_local("h"))
                                .ok_or_else(unsupported)?;
                            let n = parse_twip(h)?;
                            let rule = p.attr_local("hRule").unwrap_or("atLeast");
                            match rule {
                                "exact" => height = RowHeight::Exact(n),
                                "atLeast" => height = RowHeight::AtLeast(n),
                                "auto" => {
                                    if n != 0 {
                                        return Err(unsupported());
                                    }
                                    height = RowHeight::Auto;
                                }
                                _ => return Err(unsupported()),
                            }
                        }
                        "gridAfter" | "gridBefore" | "wAfter" | "wBefore" | "tblHeader"
                        | "cantSplit" => return Err(unsupported()),
                        _ => return Err(unsupported()),
                    }
                }
            }
            "tc" => cells.push(parse_plain_cell(pkg, doc_part, child, dpi, defaults)?),
            "tblPrEx" => return Err(unsupported()),
            _ => return Err(unsupported()),
        }
    }
    if expect_cols != 0 && cells.len() != expect_cols {
        return Err(unsupported());
    }
    Ok(PendingRowBuild { height, cells })
}

fn parse_plain_cell(
    pkg: &Package,
    doc_part: &str,
    tc: &Element,
    dpi: RenderDpi,
    defaults: &RunFonts,
) -> Result<PendingCellBuild, Error> {
    let mut fill = None;
    let mut tc_w = None;
    let mut paras: Vec<CellPara> = Vec::new();
    for child in tc.element_children() {
        match child.local_name() {
            "tcPr" => {
                for p in child.element_children() {
                    match p.local_name() {
                        "tcW" => {
                            let w = p.attr_local("w").ok_or_else(unsupported)?;
                            let ty = p.attr_local("type").unwrap_or("dxa");
                            match ty {
                                "dxa" => {
                                    let n = parse_twip(w)?;
                                    if n <= 0 {
                                        return Err(unsupported());
                                    }
                                    tc_w = Some(n);
                                }
                                "auto" | "nil" => {}
                                _ => return Err(unsupported()),
                            }
                        }
                        "shd" => {
                            fill = Some(parse_shd_fill(p)?);
                        }
                        "gridSpan" | "hMerge" | "vMerge" | "tcBorders" | "tcMar" | "vAlign"
                        | "noWrap" | "textDirection" | "tcFitText" | "hideMark" => {
                            return Err(unsupported());
                        }
                        _ => return Err(unsupported()),
                    }
                }
            }
            "p" => match paragraph_kind(pkg, doc_part, child, dpi, defaults)? {
                StoryPara::Image(_) | StoryPara::Table(_) => return Err(unsupported()),
                StoryPara::Empty { before, after } => paras.push(CellPara::Empty { before, after }),
                StoryPara::Text(text) => paras.push(CellPara::Text(text)),
            },
            "tbl" => return Err(unsupported()),
            _ => return Err(unsupported()),
        }
    }
    if paras.is_empty() {
        return Err(unsupported());
    }
    Ok(PendingCellBuild { fill, tc_w, paras })
}

/// 	blW=auto 且每一列的 dxa 	cW 在各行都一致时，用它作列宽。
/// Word 16.0 对空表走首选单元格宽，不沿用可能过期的 	blGrid。
fn preferred_auto_col_widths(rows: &[PendingRowBuild], n: usize) -> Option<Vec<i64>> {
    if n == 0 || rows.is_empty() {
        return None;
    }
    let mut widths = Vec::with_capacity(n);
    for ci in 0..n {
        let w = rows[0].cells[ci].tc_w?;
        if w <= 0 || !rows.iter().all(|row| row.cells[ci].tc_w == Some(w)) {
            return None;
        }
        widths.push(w);
    }
    Some(widths)
}

fn parse_shd_fill(shd: &Element) -> Result<[u8; 3], Error> {
    let fill = shd.attr_local("fill").ok_or_else(unsupported)?;
    if fill.eq_ignore_ascii_case("auto") {
        return Err(unsupported());
    }
    parse_rgb_hex(fill)
}

fn parse_rgb_hex(raw: &str) -> Result<[u8; 3], Error> {
    let s = raw.trim();
    if s.len() != 6 {
        return Err(unsupported());
    }
    let n = u32::from_str_radix(s, 16).map_err(|_| unsupported())?;
    Ok([
        ((n >> 16) & 0xff) as u8,
        ((n >> 8) & 0xff) as u8,
        (n & 0xff) as u8,
    ])
}

/// 本切片只画 `single` + `auto`/`000000`；`nil`/`none` 为无边；其它 NI。
fn border_is_single_black(el: &Element) -> Result<bool, Error> {
    let val = el.attr_local("val").unwrap_or("nil");
    match val {
        "nil" | "none" => Ok(false),
        "single" => {
            let color = el.attr_local("color").unwrap_or("auto");
            if !(color.eq_ignore_ascii_case("auto") || color.eq_ignore_ascii_case("000000")) {
                return Err(unsupported());
            }
            // sz 缺省或 4（建表骨架）；其它粗细本切片 NI，避免假称通用。
            let sz = el.attr_local("sz").unwrap_or("4");
            if sz != "4" && sz != "2" && sz != "8" {
                // 允许常见细线 2/4/8（¼/½/1 pt）；都画成 1px 黑（oracle 半磅=1px）。
                return Err(unsupported());
            }
            Ok(true)
        }
        _ => Err(unsupported()),
    }
}

fn paint_table(
    rgb: &mut [u8],
    page_w: u32,
    page_h: u32,
    tbl: &TableBlit,
    dpi: RenderDpi,
) -> Result<(), Error> {
    let x0 = twip_px_unclamped(tbl.left_twip, dpi)? as i32;
    let y0 = twip_px_unclamped(tbl.top_twip, dpi)? as i32;
    if x0 < 0 || y0 < 0 {
        return Err(unsupported());
    }
    // 累计网格线像素（与 Word PDF 累计取整一致）。
    let mut xs = vec![x0];
    let mut acc = tbl.left_twip;
    for w in &tbl.col_widths {
        acc = acc.saturating_add(*w);
        xs.push(twip_px_unclamped(acc, dpi)? as i32);
    }
    let mut ys = vec![y0];
    acc = tbl.top_twip;
    for h in &tbl.row_heights {
        acc = acc.saturating_add(*h);
        ys.push(twip_px_unclamped(acc, dpi)? as i32);
    }
    let right = *xs.last().unwrap();
    let bottom = *ys.last().unwrap();
    if right > page_w as i32 || bottom > page_h as i32 {
        return Err(unsupported());
    }

    // 填色：有边时内缩 1px，避免盖住网格线。
    let inset = if tbl.border_top
        || tbl.border_left
        || tbl.border_bottom
        || tbl.border_right
        || tbl.border_inside_h
        || tbl.border_inside_v
    {
        1
    } else {
        0
    };
    for &(r, c, color) in &tbl.fills {
        if r + 1 >= ys.len() || c + 1 >= xs.len() {
            return Err(unsupported());
        }
        let x1 = xs[c] + inset;
        let y1 = ys[r] + inset;
        let x2 = xs[c + 1] - inset;
        let y2 = ys[r + 1] - inset;
        fill_rect_rgb(rgb, page_w, page_h, x1, y1, x2, y2, color)?;
    }

    let black = [0u8, 0u8, 0u8];
    // 横线
    for (i, &y) in ys.iter().enumerate() {
        let draw = if i == 0 {
            tbl.border_top
        } else if i + 1 == ys.len() {
            tbl.border_bottom
        } else {
            tbl.border_inside_h
        };
        if draw {
            hline_rgb(rgb, page_w, page_h, xs[0], *xs.last().unwrap(), y, black)?;
        }
    }
    // 竖线
    for (i, &x) in xs.iter().enumerate() {
        let draw = if i == 0 {
            tbl.border_left
        } else if i + 1 == xs.len() {
            tbl.border_right
        } else {
            tbl.border_inside_v
        };
        if draw {
            vline_rgb(rgb, page_w, page_h, ys[0], *ys.last().unwrap(), x, black)?;
        }
    }
    Ok(())
}

fn fill_rect_rgb(
    rgb: &mut [u8],
    page_w: u32,
    page_h: u32,
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
    color: [u8; 3],
) -> Result<(), Error> {
    if x2 <= x1 || y2 <= y1 {
        return Ok(());
    }
    for y in y1..y2 {
        if y < 0 || y >= page_h as i32 {
            continue;
        }
        for x in x1..x2 {
            if x < 0 || x >= page_w as i32 {
                continue;
            }
            let i = (y as usize * page_w as usize + x as usize) * 3;
            rgb[i] = color[0];
            rgb[i + 1] = color[1];
            rgb[i + 2] = color[2];
        }
    }
    Ok(())
}

fn hline_rgb(
    rgb: &mut [u8],
    page_w: u32,
    page_h: u32,
    x1: i32,
    x2: i32,
    y: i32,
    color: [u8; 3],
) -> Result<(), Error> {
    if y < 0 || y >= page_h as i32 {
        return Ok(());
    }
    let (a, b) = if x1 <= x2 { (x1, x2) } else { (x2, x1) };
    for x in a..=b {
        if x < 0 || x >= page_w as i32 {
            continue;
        }
        let i = (y as usize * page_w as usize + x as usize) * 3;
        rgb[i] = color[0];
        rgb[i + 1] = color[1];
        rgb[i + 2] = color[2];
    }
    Ok(())
}

fn vline_rgb(
    rgb: &mut [u8],
    page_w: u32,
    page_h: u32,
    y1: i32,
    y2: i32,
    x: i32,
    color: [u8; 3],
) -> Result<(), Error> {
    if x < 0 || x >= page_w as i32 {
        return Ok(());
    }
    let (a, b) = if y1 <= y2 { (y1, y2) } else { (y2, y1) };
    for y in a..=b {
        if y < 0 || y >= page_h as i32 {
            continue;
        }
        let i = (y as usize * page_w as usize + x as usize) * 3;
        rgb[i] = color[0];
        rgb[i + 1] = color[1];
        rgb[i + 2] = color[2];
    }
    Ok(())
}

fn unsupported() -> Error {
    Error::with_path(ErrorKind::PageImageUnsupported, "id")
}
