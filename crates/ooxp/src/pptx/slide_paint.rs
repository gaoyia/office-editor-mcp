//! 幻灯片只读栅格化。先铺页底，再按文档顺序画形状、图、表和文字。
//! 没有本页纯色页底时铺白。画不了的对象让整页失败，不交出缺一块的图。
//! 不写回包，像素不进 inspect。

use std::io::Cursor;

use crate::error::{Error, ErrorKind, IdReason};
use crate::opc::{self, Package};
use crate::xml::Element;

use super::picture;
use super::shape;
use super::ExportedMedia;
use crate::xlsx::Face;

const EMU_PER_INCH: f64 = 914_400.0;
const DPI: f64 = 96.0;
const MAX_PX: u32 = 4096;

pub fn export_slide_image(bytes: &[u8], id: &str) -> Result<ExportedMedia, Error> {
    let n = parse_slide_id(id)?;
    let pkg = Package::open_bytes(bytes.to_vec())?;
    let pres_part = opc::office_document_part(&pkg)?;
    let pres = super::list_pin::parse_part(&pkg, &pres_part)?;
    let slides = shape::list_slides(&pkg, &pres_part, &pres.root)?;
    let slide = slides
        .iter()
        .find(|s| s.index == n)
        .ok_or_else(|| Error::with_path(ErrorKind::SlideNotFound { n: n as u32 }, "id"))?;
    let doc = super::list_pin::parse_part(&pkg, &slide.part)?;
    let (cx, cy) = picture::canvas_size(&pres.root);
    let width = emu_span(cx)?;
    let height = emu_span(cy)?;
    let bg = page_color(&doc.root).unwrap_or([255, 255, 255]);
    let mut rgb = vec![0u8; (width as usize) * (height as usize) * 3];
    fill(&mut rgb, bg);
    paint_tree(&pkg, &slide.part, &doc.root, &mut rgb, width, height)?;
    let png = encode_png(width, height, &rgb)?;
    Ok(ExportedMedia {
        bytes: png,
        content_type: Some("image/png".to_string()),
    })
}

fn parse_slide_id(id: &str) -> Result<usize, Error> {
    let Some(rest) = id.strip_prefix("slide:") else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::ExportSlideNeedsSlide,
            },
            "id",
        ));
    };
    if rest.contains('/') || rest.is_empty() {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::ExportSlideNeedsSlide,
            },
            "id",
        ));
    }
    let n: u32 = rest.parse().map_err(|_| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SlideIndexNotInt,
            },
            "id",
        )
    })?;
    if n == 0 {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SlideIndexFromOne,
            },
            "id",
        ));
    }
    Ok(n as usize)
}

fn page_color(root: &Element) -> Option<[u8; 3]> {
    let clr = root
        .child_by_local("cSld")?
        .child_by_local("bg")?
        .child_by_local("bgPr")?
        .child_by_local("solidFill")?
        .child_by_local("srgbClr")?;
    for child in clr.element_children() {
        if child.local_name() != "alpha" {
            return None;
        }
        let val = child.attr("val").unwrap_or("");
        if val != "100000" {
            return None;
        }
    }
    parse_rgb(clr.attr("val")?)
}

fn paint_tree(
    pkg: &Package,
    slide_part: &str,
    root: &Element,
    rgb: &mut [u8],
    width: u32,
    height: u32,
) -> Result<(), Error> {
    let Some(tree) = root
        .child_by_local("cSld")
        .and_then(|c| c.child_by_local("spTree"))
    else {
        return Ok(());
    };
    visit(pkg, slide_part, tree, false, rgb, width, height)
}

fn visit(
    pkg: &Package,
    slide_part: &str,
    el: &Element,
    grouped: bool,
    rgb: &mut [u8],
    width: u32,
    height: u32,
) -> Result<(), Error> {
    for child in el.element_children() {
        match child.local_name() {
            "nvGrpSpPr" | "grpSpPr" | "extLst" => {}
            "pic" => {
                if grouped || transformed(child) || is_media(child) {
                    return Err(unsupported());
                }
                blit_pic(pkg, slide_part, child, rgb, width, height)?;
            }
            "sp" if !grouped => paint_sp(child, rgb, width, height)?,
            "graphicFrame" if !grouped => paint_frame(pkg, slide_part, child, rgb, width, height)?,
            "grpSp" => visit(pkg, slide_part, child, true, rgb, width, height)?,
            "AlternateContent" => {
                if let Some(choice) = child.child_by_local("Choice") {
                    visit(pkg, slide_part, choice, grouped, rgb, width, height)?;
                }
            }
            _ => return Err(unsupported()),
        }
    }
    Ok(())
}

/// 矩形、椭圆、圆角矩形和里面的文字。描边、半透明或转角对不上的矩形整块留空，页继续画。
fn paint_sp(sp: &Element, rgb: &mut [u8], width: u32, height: u32) -> Result<(), Error> {
    let Some(sp_pr) = sp.child_by_local("spPr") else {
        return Ok(());
    };
    let prst = sp_pr
        .child_by_local("prstGeom")
        .and_then(|geom| geom.attr("prst"))
        .unwrap_or("");
    match prst {
        "rect" => paint_rect(sp, sp_pr, rgb, width, height),
        "ellipse" => paint_ellipse(sp, sp_pr, rgb, width, height),
        "roundRect" => paint_round(sp, sp_pr, rgb, width, height),
        _ => Err(unsupported()),
    }
}

fn paint_rect(
    sp: &Element,
    sp_pr: &Element,
    rgb: &mut [u8],
    width: u32,
    height: u32,
) -> Result<(), Error> {
    if let Some((color, x, y, w, h)) = plain_rect(sp) {
        fill_rect(rgb, width, height, x, y, w, h, color);
        paint_tx(sp, x, y, w, h, rgb, width, height);
        return Ok(());
    }
    let Some(xfrm) = sp_pr.child_by_local("xfrm") else {
        return Ok(());
    };
    if xfrm.attr("rot").is_some() {
        return Ok(());
    }
    let (x, y, cx, cy) = xfrm_box(xfrm);
    let (x, y, w, h) = (emu_px(x), emu_px(y), emu_px(cx), emu_px(cy));
    if w <= 0 || h <= 0 {
        return Ok(());
    }
    if sp_pr.child_by_local("noFill").is_some() && line_is_clear(sp_pr) {
        paint_tx(sp, x, y, w, h, rgb, width, height);
        return Ok(());
    }
    if sp_pr.child_by_local("gradFill").is_some() {
        let (c0, c1, ang) = linear_gradient(sp_pr).ok_or_else(unsupported)?;
        if !line_is_clear(sp_pr) {
            return Err(unsupported());
        }
        fill_gradient(rgb, width, height, x, y, w, h, c0, c1, ang);
        paint_tx(sp, x, y, w, h, rgb, width, height);
    }
    Ok(())
}

fn paint_ellipse(
    sp: &Element,
    sp_pr: &Element,
    rgb: &mut [u8],
    width: u32,
    height: u32,
) -> Result<(), Error> {
    let Some(oval) = oval_box(sp_pr)? else {
        return Ok(());
    };
    fill_ellipse(
        rgb,
        width,
        height,
        oval.x,
        oval.y,
        oval.w,
        oval.h,
        oval.fill,
        oval.stroke,
    );
    paint_tx(sp, oval.x, oval.y, oval.w, oval.h, rgb, width, height);
    Ok(())
}

fn paint_round(
    sp: &Element,
    sp_pr: &Element,
    rgb: &mut [u8],
    width: u32,
    height: u32,
) -> Result<(), Error> {
    let Some(round) = round_box(sp_pr)? else {
        return Ok(());
    };
    fill_round(
        rgb,
        width,
        height,
        round.x,
        round.y,
        round.w,
        round.h,
        round.radius,
        round.fill,
    );
    paint_tx(sp, round.x, round.y, round.w, round.h, rgb, width, height);
    Ok(())
}

struct Oval {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    fill: RgbaFill,
    stroke: Option<[u8; 3]>,
}

struct Round {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    fill: RgbaFill,
    radius: i32,
}

fn oval_box(sp_pr: &Element) -> Result<Option<Oval>, Error> {
    if rotated(sp_pr) {
        return Err(unsupported());
    }
    let Some((x, y, w, h)) = px_box(sp_pr) else {
        return Ok(None);
    };
    Ok(Some(Oval {
        x,
        y,
        w,
        h,
        fill: shape_fill(sp_pr)?,
        stroke: stroke_color(sp_pr)?,
    }))
}

fn round_box(sp_pr: &Element) -> Result<Option<Round>, Error> {
    if rotated(sp_pr) {
        return Err(unsupported());
    }
    let Some((x, y, w, h)) = px_box(sp_pr) else {
        return Ok(None);
    };
    Ok(Some(Round {
        x,
        y,
        w,
        h,
        fill: shape_fill(sp_pr)?,
        radius: round_radius(sp_pr, w, h),
    }))
}

fn rotated(sp_pr: &Element) -> bool {
    sp_pr
        .child_by_local("xfrm")
        .and_then(|xfrm| xfrm.attr("rot"))
        .is_some_and(|raw| raw.parse::<i64>().ok().unwrap_or(1) != 0)
}

fn px_box(sp_pr: &Element) -> Option<(i32, i32, i32, i32)> {
    let xfrm = sp_pr.child_by_local("xfrm")?;
    let (x, y, cx, cy) = xfrm_box(xfrm);
    let (w, h) = (emu_px(cx), emu_px(cy));
    if w <= 0 || h <= 0 {
        return None;
    }
    Some((emu_px(x), emu_px(y), w, h))
}

struct RgbaFill {
    color: [u8; 3],
    alpha: u8,
    on: bool,
}

fn shape_fill(sp_pr: &Element) -> Result<RgbaFill, Error> {
    if sp_pr.child_by_local("noFill").is_some() {
        return Ok(RgbaFill {
            color: [0, 0, 0],
            alpha: 0,
            on: false,
        });
    }
    if let Some(fill) = sp_pr.child_by_local("solidFill") {
        let (color, alpha) = rgba_of(fill).ok_or_else(unsupported)?;
        return Ok(RgbaFill {
            color,
            alpha,
            on: true,
        });
    }
    if sp_pr.child_by_local("gradFill").is_some() {
        return Err(unsupported());
    }
    Ok(RgbaFill {
        color: [0, 0, 0],
        alpha: 0,
        on: false,
    })
}

fn stroke_color(sp_pr: &Element) -> Result<Option<[u8; 3]>, Error> {
    if line_is_clear(sp_pr) {
        return Ok(None);
    }
    let Some(fill) = sp_pr
        .child_by_local("ln")
        .and_then(|ln| ln.child_by_local("solidFill"))
    else {
        return Err(unsupported());
    };
    let (color, alpha) = rgba_of(fill).ok_or_else(unsupported)?;
    if alpha != 255 {
        return Err(unsupported());
    }
    Ok(Some(color))
}

fn rgba_of(fill: &Element) -> Option<([u8; 3], u8)> {
    let clr = fill.child_by_local("srgbClr")?;
    let mut alpha = 255u8;
    for child in clr.element_children() {
        if child.local_name() != "alpha" {
            return None;
        }
        let val: u32 = child.attr("val")?.parse().ok()?;
        alpha = ((u64::from(val.min(100_000)) * 255 + 50_000) / 100_000) as u8;
    }
    Some((parse_rgb(clr.attr("val")?)?, alpha))
}

fn linear_gradient(sp_pr: &Element) -> Option<([u8; 3], [u8; 3], i64)> {
    let grad = sp_pr.child_by_local("gradFill")?;
    let stops: Vec<&Element> = grad
        .child_by_local("gsLst")?
        .element_children()
        .filter(|el| el.local_name() == "gs")
        .collect();
    if stops.len() != 2 {
        return None;
    }
    let c0 = opaque_rgb(stops[0])?;
    let c1 = opaque_rgb(stops[1])?;
    let ang: i64 = grad
        .child_by_local("lin")?
        .attr("ang")
        .unwrap_or("0")
        .parse()
        .ok()?;
    if ang.rem_euclid(FULL_TURN) % RIGHT_ANGLE != 0 {
        return None;
    }
    Some((c0, c1, ang))
}

fn round_radius(sp_pr: &Element, w: i32, h: i32) -> i32 {
    let adj = sp_pr
        .child_by_local("prstGeom")
        .and_then(|geom| geom.child_by_local("avLst"))
        .and_then(|list| {
            list.element_children()
                .find(|el| el.attr("name") == Some("adj"))
        })
        .and_then(|gd| gd.attr("fmla"))
        .and_then(|fmla| fmla.strip_prefix("val "))
        .and_then(|raw| raw.parse::<i64>().ok())
        .unwrap_or(16_667);
    let ss = i64::from(w.min(h).max(0));
    ((ss * adj) / 100_000).clamp(0, ss / 2) as i32
}

fn paint_frame(
    pkg: &Package,
    slide_part: &str,
    frame: &Element,
    rgb: &mut [u8],
    width: u32,
    height: u32,
) -> Result<(), Error> {
    let Some(xfrm) = frame.child_by_local("xfrm") else {
        return Err(unsupported());
    };
    let (x, y, cx, cy) = xfrm_box(xfrm);
    let (x, y, w, h) = (emu_px(x), emu_px(y), emu_px(cx), emu_px(cy));
    if w <= 0 || h <= 0 {
        return Err(unsupported());
    }
    let Some(data) = frame
        .child_by_local("graphic")
        .and_then(|graphic| graphic.child_by_local("graphicData"))
    else {
        return Err(unsupported());
    };
    if let Some(chart) = data.child_by_local("chart") {
        return paint_chart(pkg, slide_part, chart, rgb, width, height, x, y, w, h);
    }
    if let Some(tbl) = data.child_by_local("tbl") {
        paint_table(tbl, rgb, width, height, x, y)?;
        return Ok(());
    }
    Err(unsupported())
}

fn paint_chart(
    pkg: &Package,
    slide_part: &str,
    chart: &Element,
    rgb: &mut [u8],
    width: u32,
    height: u32,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
) -> Result<(), Error> {
    let Some(rid) = chart.attr_local("id").filter(|s| !s.is_empty()) else {
        return Err(unsupported());
    };
    let part = opc::resolve_id(pkg, slide_part, rid).map_err(|_| unsupported())?;
    let bytes = pkg.get(&part).map_err(|_| unsupported())?;
    let doc = crate::xml::parse(bytes).map_err(|_| unsupported())?;
    let theme = pkg.get("/ppt/theme/theme1.xml").ok();
    let png = crate::xlsx::render_clustered_chart(&doc.root, w as u32, h as u32, theme)
        .map_err(|_| unsupported())?;
    let image = decode_png(&png)?;
    paint_image(rgb, width, height, x, y, w as u32, h as u32, &image);
    Ok(())
}

fn paint_table(
    tbl: &Element,
    rgb: &mut [u8],
    width: u32,
    height: u32,
    x: i32,
    y: i32,
) -> Result<(), Error> {
    let cols: Vec<i32> = tbl
        .child_by_local("tblGrid")
        .map(|grid| {
            grid.element_children()
                .filter(|el| el.local_name() == "gridCol")
                .map(|col| emu_px(col.attr("w").and_then(|raw| raw.parse().ok()).unwrap_or(0)))
                .collect()
        })
        .unwrap_or_default();
    let mut py = y;
    for tr in tbl.element_children().filter(|el| el.local_name() == "tr") {
        let rh = emu_px(tr.attr("h").and_then(|raw| raw.parse().ok()).unwrap_or(0));
        let mut px = x;
        for (i, tc) in tr
            .element_children()
            .filter(|el| el.local_name() == "tc")
            .enumerate()
        {
            let cw = cols.get(i).copied().unwrap_or(0);
            if tc.attr_local("gridSpan").is_some()
                || tc.attr_local("rowSpan").is_some_and(|v| v != "1")
                || tc.attr_local("hMerge").is_some()
                || tc.attr_local("vMerge").is_some()
            {
                return Err(unsupported());
            }
            let fill = tc
                .child_by_local("tcPr")
                .and_then(|pr| pr.child_by_local("solidFill"))
                .and_then(opaque_rgb);
            if let Some(color) = fill {
                fill_rect(rgb, width, height, px, py, cw, rh, color);
            }
            let ink = fill
                .map(|color| {
                    let lum = u16::from(color[0]) + u16::from(color[1]) + u16::from(color[2]);
                    if lum < 380 {
                        [255, 255, 255]
                    } else {
                        [0x1F, 0x4E, 0x79]
                    }
                })
                .unwrap_or([0, 0, 0]);
            paint_cell_text(tc, px, py, cw, rh, ink, rgb, width, height);
            px += cw;
        }
        py += rh;
    }
    Ok(())
}

fn paint_cell_text(
    tc: &Element,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    color: [u8; 3],
    rgb: &mut [u8],
    width: u32,
    height: u32,
) {
    let Some(tx) = tc.child_by_local("txBody") else {
        return;
    };
    let text = tx_string(tx);
    if text.is_empty() {
        return;
    }
    let px = 16.0 * 96.0 / 72.0;
    let Some(face) = Face::open_named(px, false, "微软雅黑") else {
        return;
    };
    let tw = face.width(&text);
    let left = f64::from(x) + (f64::from(w) - tw) / 2.0;
    let top = f64::from(y) + (f64::from(h) - face.em_box()) / 2.0;
    face.draw(&text, left, top, color, |cx, cy, ink, cov| {
        blend(rgb, width, height, cx, cy, ink, cov);
    });
}

fn tx_string(tx: &Element) -> String {
    let mut out = String::new();
    for para in tx.element_children().filter(|el| el.local_name() == "p") {
        for child in para.element_children() {
            if child.local_name() == "r" {
                if let Some(text) = child.child_by_local("t") {
                    out.push_str(&text.text_content());
                }
            }
        }
    }
    out
}

fn paint_tx(sp: &Element, x: i32, y: i32, w: i32, h: i32, rgb: &mut [u8], width: u32, height: u32) {
    let Some(tx) = sp.child_by_local("txBody") else {
        return;
    };
    let body = tx.child_by_local("bodyPr");
    let inset = |name: &str, default: i64| {
        body.and_then(|el| el.attr(name))
            .and_then(|raw| raw.parse().ok())
            .map(emu_px)
            .unwrap_or_else(|| emu_px(default))
    };
    let left = x + inset("lIns", 91_440);
    let top = y + inset("tIns", 45_720);
    let right = x + w - inset("rIns", 91_440);
    let bottom = y + h - inset("bIns", 45_720);
    let inner_w = right - left;
    let inner_h = bottom - top;
    if inner_w <= 0 || inner_h <= 0 {
        return;
    }
    let anchor = body.and_then(|el| el.attr("anchor")).unwrap_or("t");
    let mut lines: Vec<TextLine> = Vec::new();
    for para in tx.element_children().filter(|el| el.local_name() == "p") {
        let Some(line) = para_line(para) else {
            continue;
        };
        lines.push(line);
    }
    if lines.is_empty() {
        return;
    }
    let Some(probe) = Face::open_named(lines[0].px, lines[0].bold, &lines[0].font) else {
        return;
    };
    let line_h = probe.em_box();
    let block = line_h * lines.len() as f64;
    let mut cursor_y = match anchor {
        "ctr" => f64::from(top) + (f64::from(inner_h) - block) / 2.0,
        "b" => f64::from(bottom) - block,
        _ => f64::from(top),
    };
    for line in lines {
        let Some(face) = Face::open_named(line.px, line.bold, &line.font) else {
            cursor_y += line_h;
            continue;
        };
        let wrapped = wrap_text(&face, &line.text, f64::from(inner_w));
        for (index, piece) in wrapped.iter().enumerate() {
            if index > 0 {
                cursor_y += face.em_box();
            }
            let tw = face.width(piece);
            let cursor_x = match line.align.as_str() {
                "ctr" => f64::from(left) + (f64::from(inner_w) - tw) / 2.0,
                "r" => f64::from(right) - tw,
                _ => f64::from(left),
            };
            face.draw(piece, cursor_x, cursor_y, line.color, |cx, cy, ink, cov| {
                blend(rgb, width, height, cx, cy, ink, cov);
            });
        }
        cursor_y += face.em_box();
    }
}

struct TextLine {
    text: String,
    px: f32,
    bold: bool,
    color: [u8; 3],
    font: String,
    align: String,
}

fn para_line(para: &Element) -> Option<TextLine> {
    let run = para
        .element_children()
        .find(|el| el.local_name() == "r" && el.child_by_local("t").is_some())?;
    let text = run.child_by_local("t")?.text_content();
    if text.trim().is_empty() {
        return None;
    }
    let rpr = run.child_by_local("rPr");
    let sz = rpr
        .and_then(|el| el.attr("sz"))
        .and_then(|raw| raw.parse::<f32>().ok())
        .unwrap_or(1800.0);
    let bold = rpr
        .and_then(|el| el.attr("b"))
        .is_some_and(|v| v == "1" || v == "true");
    let color = rpr
        .and_then(|el| el.child_by_local("solidFill"))
        .and_then(opaque_rgb)
        .unwrap_or([0, 0, 0]);
    let latin = rpr
        .and_then(|el| el.child_by_local("latin"))
        .and_then(|el| el.attr("typeface"))
        .unwrap_or("");
    let ea = rpr
        .and_then(|el| el.child_by_local("ea"))
        .and_then(|el| el.attr("typeface"))
        .unwrap_or("");
    let font = if text.chars().any(|ch| ch > '\u{7f}') && !ea.is_empty() {
        ea.to_string()
    } else if !latin.is_empty() {
        latin.to_string()
    } else if !ea.is_empty() {
        ea.to_string()
    } else {
        "微软雅黑".to_string()
    };
    let align = para
        .child_by_local("pPr")
        .and_then(|el| el.attr("algn"))
        .unwrap_or("l")
        .to_string();
    Some(TextLine {
        text,
        px: sz / 100.0 * 96.0 / 72.0,
        bold,
        color,
        font,
        align,
    })
}

fn wrap_text(face: &Face, text: &str, max_w: f64) -> Vec<String> {
    if max_w <= 0.0 || face.width(text) <= max_w {
        return vec![text.to_string()];
    }
    let mut lines = Vec::new();
    let mut cur = String::new();
    for ch in text.chars() {
        let mut trial = cur.clone();
        trial.push(ch);
        if face.width(&trial) > max_w && !cur.is_empty() {
            lines.push(std::mem::take(&mut cur));
            cur.push(ch);
        } else {
            cur = trial;
        }
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    if lines.is_empty() {
        vec![text.to_string()]
    } else {
        lines
    }
}

fn fill_gradient(
    rgb: &mut [u8],
    width: u32,
    height: u32,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    c0: [u8; 3],
    c1: [u8; 3],
    ang: i64,
) {
    let turn = ang.rem_euclid(FULL_TURN);
    for row in 0..h {
        let py = y + row;
        if py < 0 || py >= height as i32 {
            continue;
        }
        for col in 0..w {
            let px = x + col;
            if px < 0 || px >= width as i32 {
                continue;
            }
            let quarter = turn / RIGHT_ANGLE;
            let (num, den) = match quarter {
                1 => (i64::from(row), i64::from(h - 1)),
                2 => (i64::from(w - 1 - col), i64::from(w - 1)),
                3 => (i64::from(h - 1 - row), i64::from(h - 1)),
                _ => (i64::from(col), i64::from(w - 1)),
            };
            let color = [
                lerp(c0[0], c1[0], num, den),
                lerp(c0[1], c1[1], num, den),
                lerp(c0[2], c1[2], num, den),
            ];
            let i = (py as usize * width as usize + px as usize) * 3;
            rgb[i..i + 3].copy_from_slice(&color);
        }
    }
}

fn lerp(a: u8, b: u8, num: i64, den: i64) -> u8 {
    if den <= 0 {
        return a;
    }
    let v = i64::from(a) + (i64::from(b) - i64::from(a)) * num / den;
    v.clamp(0, 255) as u8
}

fn fill_ellipse(
    rgb: &mut [u8],
    width: u32,
    height: u32,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    fill: RgbaFill,
    stroke: Option<[u8; 3]>,
) {
    let sw = if stroke.is_some() { 1 } else { 0 };
    for row in 0..h {
        let py = y + row;
        for col in 0..w {
            let px = x + col;
            if !inside_ellipse(px, py, x, y, w, h) {
                continue;
            }
            let inner = sw > 0
                && w > sw * 2
                && h > sw * 2
                && inside_ellipse(px, py, x + sw, y + sw, w - sw * 2, h - sw * 2);
            if inner || stroke.is_none() {
                if fill.on {
                    blend(rgb, width, height, px, py, fill.color, fill.alpha);
                }
            } else if let Some(color) = stroke {
                blend(rgb, width, height, px, py, color, 255);
            }
        }
    }
}

fn inside_ellipse(px: i32, py: i32, x: i32, y: i32, w: i32, h: i32) -> bool {
    if w <= 0 || h <= 0 {
        return false;
    }
    let cx = f64::from(x) + f64::from(w) / 2.0;
    let cy = f64::from(y) + f64::from(h) / 2.0;
    let dx = (f64::from(px) + 0.5 - cx) / (f64::from(w) / 2.0);
    let dy = (f64::from(py) + 0.5 - cy) / (f64::from(h) / 2.0);
    dx * dx + dy * dy <= 1.0
}

fn fill_round(
    rgb: &mut [u8],
    width: u32,
    height: u32,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    radius: i32,
    fill: RgbaFill,
) {
    if !fill.on {
        return;
    }
    for row in 0..h {
        let py = y + row;
        for col in 0..w {
            let px = x + col;
            if inside_round(px, py, x, y, w, h, radius) {
                blend(rgb, width, height, px, py, fill.color, fill.alpha);
            }
        }
    }
}

fn inside_round(px: i32, py: i32, x: i32, y: i32, w: i32, h: i32, radius: i32) -> bool {
    if px < x || py < y || px >= x + w || py >= y + h {
        return false;
    }
    let r = radius.max(0).min(w / 2).min(h / 2);
    if r == 0 {
        return true;
    }
    let (cx, cy) = if px < x + r && py < y + r {
        (x + r, y + r)
    } else if px >= x + w - r && py < y + r {
        (x + w - r, y + r)
    } else if px < x + r && py >= y + h - r {
        (x + r, y + h - r)
    } else if px >= x + w - r && py >= y + h - r {
        (x + w - r, y + h - r)
    } else {
        return true;
    };
    let dx = f64::from(px) + 0.5 - f64::from(cx);
    let dy = f64::from(py) + 0.5 - f64::from(cy);
    dx * dx + dy * dy <= f64::from(r) * f64::from(r)
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
    for (k, src) in color.into_iter().enumerate() {
        let dst = u16::from(rgb[i + k]);
        rgb[i + k] = ((u16::from(src) * c + dst * (255 - c) + 127) / 255) as u8;
    }
}

fn plain_rect(sp: &Element) -> Option<([u8; 3], i32, i32, i32, i32)> {
    let sp_pr = sp.child_by_local("spPr")?;
    if sp_pr.child_by_local("effectLst").is_some()
        || sp_pr.child_by_local("scene3d").is_some()
        || sp_pr.child_by_local("sp3d").is_some()
    {
        return None;
    }
    let xfrm = sp_pr.child_by_local("xfrm")?;
    let prst = sp_pr.child_by_local("prstGeom")?.attr("prst")?;
    if prst != "rect" {
        return None;
    }
    if !line_is_clear(sp_pr) {
        return None;
    }
    let color = opaque_rgb(sp_pr.child_by_local("solidFill")?)?;
    let (x, y, cx, cy) = xfrm_box(xfrm);
    let (x, y, cx, cy) = turned_box(x, y, cx, cy, xfrm.attr("rot"))?;
    let w = emu_px(cx);
    let h = emu_px(cy);
    if w <= 0 || h <= 0 {
        return None;
    }
    Some((color, emu_px(x), emu_px(y), w, h))
}

fn line_is_clear(sp_pr: &Element) -> bool {
    let Some(ln) = sp_pr.child_by_local("ln") else {
        return true;
    };
    ln.element_children()
        .all(|child| child.local_name() == "noFill")
        && ln.child_by_local("noFill").is_some()
}

fn opaque_rgb(fill: &Element) -> Option<[u8; 3]> {
    let clr = fill.child_by_local("srgbClr")?;
    for child in clr.element_children() {
        if child.local_name() != "alpha" {
            return None;
        }
        if child.attr("val").unwrap_or("") != "100000" {
            return None;
        }
    }
    parse_rgb(clr.attr("val")?)
}

fn xfrm_box(xfrm: &Element) -> (i64, i64, i64, i64) {
    let off = xfrm.child_by_local("off");
    let ext = xfrm.child_by_local("ext");
    let num = |el: Option<&Element>, name: &str| {
        el.and_then(|node| node.attr(name))
            .and_then(|raw| raw.parse().ok())
            .unwrap_or(0)
    };
    (num(off, "x"), num(off, "y"), num(ext, "cx"), num(ext, "cy"))
}

const RIGHT_ANGLE: i64 = 5_400_000;
const FULL_TURN: i64 = 21_600_000;

/// 绕矩形中心转 90 度的倍数。90 和 270 换成宽高对调后的外接矩形。
/// 转完落不到整数 EMU，或角度不是直角，就跳过。
fn turned_box(x: i64, y: i64, cx: i64, cy: i64, rot: Option<&str>) -> Option<(i64, i64, i64, i64)> {
    let Some(raw) = rot else {
        return Some((x, y, cx, cy));
    };
    let n: i64 = raw.parse().ok()?;
    let wrapped = n.rem_euclid(FULL_TURN);
    if wrapped % RIGHT_ANGLE != 0 {
        return None;
    }
    if (wrapped / RIGHT_ANGLE) % 2 == 0 {
        return Some((x, y, cx, cy));
    }
    let dx = cx - cy;
    let dy = cy - cx;
    if dx % 2 != 0 || dy % 2 != 0 {
        return None;
    }
    Some((x + dx / 2, y + dy / 2, cy, cx))
}

fn transformed(pic: &Element) -> bool {
    if pic
        .child_by_local("blipFill")
        .and_then(|fill| fill.child_by_local("srcRect"))
        .is_some()
    {
        return true;
    }
    let Some(xfrm) = pic
        .child_by_local("spPr")
        .and_then(|sp| sp.child_by_local("xfrm"))
    else {
        return false;
    };
    if crate::xml::attr_is_true(xfrm, "flipH") || crate::xml::attr_is_true(xfrm, "flipV") {
        return true;
    }
    match xfrm.attr("rot") {
        Some(raw) => raw.parse::<i64>().ok().unwrap_or(1) != 0,
        None => false,
    }
}

fn is_media(pic: &Element) -> bool {
    pic.child_by_local("nvPicPr")
        .and_then(|nv| nv.child_by_local("nvPr"))
        .is_some_and(|pr| {
            pr.child_by_local("videoFile").is_some() || pr.child_by_local("audioFile").is_some()
        })
}

fn blit_pic(
    pkg: &Package,
    slide_part: &str,
    pic: &Element,
    rgb: &mut [u8],
    width: u32,
    height: u32,
) -> Result<(), Error> {
    let Some(rid) = pic
        .child_by_local("blipFill")
        .and_then(|fill| fill.child_by_local("blip"))
        .and_then(|blip| blip.attr_local("embed"))
        .filter(|s| !s.is_empty())
    else {
        return Err(unsupported());
    };
    let part = opc::resolve_id(pkg, slide_part, rid)?;
    let bytes = pkg.get(&part)?;
    if !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Err(unsupported());
    }
    let image = decode_png(bytes)?;
    let (x, y, cx, cy) = pic_box(pic);
    let dest_w = emu_px(cx);
    let dest_h = emu_px(cy);
    if dest_w <= 0 || dest_h <= 0 {
        return Err(unsupported());
    }
    let dest_x = emu_px(x);
    let dest_y = emu_px(y);
    paint_image(
        rgb,
        width,
        height,
        dest_x,
        dest_y,
        dest_w as u32,
        dest_h as u32,
        &image,
    );
    Ok(())
}

fn pic_box(pic: &Element) -> (i64, i64, i64, i64) {
    pic.child_by_local("spPr")
        .and_then(|sp| sp.child_by_local("xfrm"))
        .map(xfrm_box)
        .unwrap_or((0, 0, 0, 0))
}

struct Rgba {
    w: u32,
    h: u32,
    px: Vec<u8>,
}

fn decode_png(bytes: &[u8]) -> Result<Rgba, Error> {
    let decoder = png::Decoder::new(Cursor::new(bytes));
    let mut reader = decoder.read_info().map_err(|_| unsupported())?;
    if reader.info().bit_depth != png::BitDepth::Eight {
        return Err(unsupported());
    }
    let mut buf = vec![0; reader.output_buffer_size()];
    let frame = reader.next_frame(&mut buf).map_err(|_| unsupported())?;
    if frame.width == 0 || frame.height == 0 || frame.width > MAX_PX || frame.height > MAX_PX {
        return Err(unsupported());
    }
    let raw = &buf[..frame.buffer_size()];
    let px = match frame.color_type {
        png::ColorType::Rgb => {
            let mut out = Vec::with_capacity(raw.len() / 3 * 4);
            for chunk in raw.chunks_exact(3) {
                out.extend_from_slice(&[chunk[0], chunk[1], chunk[2], 255]);
            }
            out
        }
        png::ColorType::Rgba => raw.to_vec(),
        _ => return Err(unsupported()),
    };
    if px.len() != (frame.width as usize) * (frame.height as usize) * 4 {
        return Err(unsupported());
    }
    Ok(Rgba {
        w: frame.width,
        h: frame.height,
        px,
    })
}

fn paint_image(
    rgb: &mut [u8],
    width: u32,
    height: u32,
    dest_x: i32,
    dest_y: i32,
    dest_w: u32,
    dest_h: u32,
    image: &Rgba,
) {
    for row in 0..dest_h {
        let y = dest_y + row as i32;
        if y < 0 || y >= height as i32 {
            continue;
        }
        let sy = ((row as u64 * image.h as u64) / dest_h as u64).min(image.h as u64 - 1) as u32;
        for col in 0..dest_w {
            let x = dest_x + col as i32;
            if x < 0 || x >= width as i32 {
                continue;
            }
            let sx = ((col as u64 * image.w as u64) / dest_w as u64).min(image.w as u64 - 1) as u32;
            let si = ((sy as usize) * (image.w as usize) + sx as usize) * 4;
            let src = [
                image.px[si],
                image.px[si + 1],
                image.px[si + 2],
                image.px[si + 3],
            ];
            let di = ((y as usize) * (width as usize) + x as usize) * 3;
            let dst = [rgb[di], rgb[di + 1], rgb[di + 2]];
            let mixed = over(dst, src);
            rgb[di..di + 3].copy_from_slice(&mixed);
        }
    }
}

fn over(dst: [u8; 3], src: [u8; 4]) -> [u8; 3] {
    let a = u16::from(src[3]);
    if a == 255 {
        return [src[0], src[1], src[2]];
    }
    if a == 0 {
        return dst;
    }
    let inv = 255 - a;
    [
        ((u16::from(src[0]) * a + u16::from(dst[0]) * inv) / 255) as u8,
        ((u16::from(src[1]) * a + u16::from(dst[1]) * inv) / 255) as u8,
        ((u16::from(src[2]) * a + u16::from(dst[2]) * inv) / 255) as u8,
    ]
}

fn fill(rgb: &mut [u8], color: [u8; 3]) {
    for px in rgb.chunks_exact_mut(3) {
        px.copy_from_slice(&color);
    }
}

fn fill_rect(
    rgb: &mut [u8],
    width: u32,
    height: u32,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    color: [u8; 3],
) {
    for row in 0..h {
        let py = y + row;
        if py < 0 || py >= height as i32 {
            continue;
        }
        for col in 0..w {
            let px = x + col;
            if px < 0 || px >= width as i32 {
                continue;
            }
            let i = (py as usize * width as usize + px as usize) * 3;
            rgb[i..i + 3].copy_from_slice(&color);
        }
    }
}

fn emu_span(emu: i64) -> Result<u32, Error> {
    let px = emu_px(emu);
    if px < 1 || px > MAX_PX as i32 {
        return Err(unsupported());
    }
    Ok(px as u32)
}

fn emu_px(emu: i64) -> i32 {
    let px = (emu as f64) * DPI / EMU_PER_INCH;
    if !px.is_finite() {
        return 0;
    }
    px.round().clamp(-100_000.0, 100_000.0) as i32
}

fn parse_rgb(raw: &str) -> Option<[u8; 3]> {
    if raw.len() != 6 {
        return None;
    }
    let n = u32::from_str_radix(raw, 16).ok()?;
    Some([
        ((n >> 16) & 0xff) as u8,
        ((n >> 8) & 0xff) as u8,
        (n & 0xff) as u8,
    ])
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

fn unsupported() -> Error {
    Error::with_path(ErrorKind::SlideImageUnsupported, "id")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flipped_solid_rect_keeps_the_same_box() {
        let xml = r#"<p:sp xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><p:spPr><a:xfrm flipH="1" flipV="1"><a:off x="1828800" y="914400"/><a:ext cx="914400" cy="457200"/></a:xfrm><a:prstGeom prst="rect"/><a:solidFill><a:srgbClr val="EE4455"/></a:solidFill><a:ln><a:noFill/></a:ln></p:spPr></p:sp>"#;
        let doc = crate::xml::parse(xml.as_bytes()).unwrap();
        let (color, x, y, w, h) = plain_rect(&doc.root).unwrap();
        assert_eq!(color, [0xEE, 0x44, 0x55]);
        assert_eq!((x, y, w, h), (192, 96, 96, 48));
    }
}
