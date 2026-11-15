//! 幻灯片只读栅格化。先铺页底，再按文档顺序画形状、图、表和文字。
//! 页底用本页、版式或母版上没有颜色变换的纯色，没有或画不了时铺白。画不了的对象让整页失败，不交出缺一块的图。
//! 不写回包，像素不进 inspect。

use std::io::Cursor;

use crate::error::{Error, ErrorKind, IdReason};
use crate::opc::{self, Package};
use crate::xml::Element;

use super::picture;
use super::shape;
use super::ExportedMedia;
use crate::xlsx::{fill_device_ellipse, Face};

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
    let bg = page_fill(&pkg, &slide.part, &doc.root)?;
    let mut rgb = vec![0u8; (width as usize) * (height as usize) * 3];
    fill(&mut rgb, bg);
    paint_inherited(&pkg, &slide.part, &doc.root, &mut rgb, width, height)?;
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

const WHITE: [u8; 3] = [255, 255, 255];

/// 本页有 `p:bg` 就用本页，否则用版式，再否则用母版。
/// 这一层写了页底但画不成纯色时铺白，不再往下继承。
fn page_fill(pkg: &Package, slide_part: &str, slide: &Element) -> Result<[u8; 3], Error> {
    let layout_part = linked_part(pkg, slide_part, "/slideLayout")?;
    let layout_doc = match &layout_part {
        Some(part) => Some(super::list_pin::parse_part(pkg, part)?),
        None => None,
    };
    let master_part = match &layout_part {
        Some(part) => linked_part(pkg, part, "/slideMaster")?,
        None => None,
    };
    let master_doc = match &master_part {
        Some(part) => Some(super::list_pin::parse_part(pkg, part)?),
        None => None,
    };
    let theme_part = match &master_part {
        Some(part) => linked_part(pkg, part, "/theme")?,
        None => None,
    };
    let theme_doc = match &theme_part {
        Some(part) => Some(super::list_pin::parse_part(pkg, part)?),
        None => None,
    };
    let map = color_map(
        slide,
        layout_doc.as_ref().map(|doc| &doc.root),
        master_doc.as_ref().map(|doc| &doc.root),
    );
    let theme = theme_doc.as_ref().map(|doc| &doc.root);
    let bg = page_bg(slide)
        .or_else(|| layout_doc.as_ref().and_then(|doc| page_bg(&doc.root)))
        .or_else(|| master_doc.as_ref().and_then(|doc| page_bg(&doc.root)));
    Ok(bg
        .and_then(|el| resolve_bg(el, map, theme))
        .unwrap_or(WHITE))
}

fn page_bg(root: &Element) -> Option<&Element> {
    root.child_by_local("cSld")?.child_by_local("bg")
}

/// 本页有覆盖表就用本页的。否则用版式的覆盖表。都没有时用母版 `clrMap`。
fn color_map<'a>(
    slide: &'a Element,
    layout: Option<&'a Element>,
    master: Option<&'a Element>,
) -> Option<&'a Element> {
    if let Some(map) = override_map(slide) {
        return Some(map);
    }
    if let Some(layout) = layout {
        if let Some(map) = override_map(layout) {
            return Some(map);
        }
    }
    master.and_then(|el| el.child_by_local("clrMap"))
}

fn override_map(root: &Element) -> Option<&Element> {
    root.child_by_local("clrMapOvr")
        .and_then(|el| el.child_by_local("overrideClrMapping"))
}

fn resolve_bg(bg: &Element, map: Option<&Element>, theme: Option<&Element>) -> Option<[u8; 3]> {
    if let Some(pr) = bg.child_by_local("bgPr") {
        return resolve_bg_pr(pr, map, theme);
    }
    if let Some(reference) = bg.child_by_local("bgRef") {
        return resolve_bg_ref(reference, map, theme);
    }
    None
}

fn resolve_bg_pr(pr: &Element, map: Option<&Element>, theme: Option<&Element>) -> Option<[u8; 3]> {
    if pr
        .attr("shadeToTitle")
        .is_some_and(|val| val == "1" || val.eq_ignore_ascii_case("true"))
    {
        return None;
    }
    let mut fill = None;
    for child in pr.element_children() {
        match child.local_name() {
            "solidFill" if fill.is_none() => fill = Some(child),
            "effectLst" if child.element_children().next().is_none() => {}
            _ => return None,
        }
    }
    plain_color(only_child(fill?)?, map, theme)
}

fn resolve_bg_ref(
    reference: &Element,
    map: Option<&Element>,
    theme: Option<&Element>,
) -> Option<[u8; 3]> {
    let idx: u32 = reference.attr("idx")?.parse().ok()?;
    // 0 和 1000 表示没有页底。调用方已经选定这一层，所以铺白，不再继承。
    if idx == 0 || idx == 1000 {
        return Some(WHITE);
    }
    let style = fill_style(theme?, idx)?;
    if !is_plain_ph(style) {
        return None;
    }
    plain_color(only_child(reference)?, map, theme)
}

fn fill_style(theme: &Element, idx: u32) -> Option<&Element> {
    let (list, offset) = if (1..1000).contains(&idx) {
        ("fillStyleLst", idx - 1)
    } else if idx >= 1001 {
        ("bgFillStyleLst", idx - 1001)
    } else {
        return None;
    };
    theme
        .child_by_local("themeElements")?
        .child_by_local("fmtScheme")?
        .child_by_local(list)?
        .element_children()
        .nth(offset as usize)
}

fn is_plain_ph(fill: &Element) -> bool {
    if fill.local_name() != "solidFill" {
        return false;
    }
    let Some(clr) = only_child(fill) else {
        return false;
    };
    clr.local_name() == "schemeClr" && clr.attr("val") == Some("phClr") && color_is_plain(clr)
}

fn plain_color(clr: &Element, map: Option<&Element>, theme: Option<&Element>) -> Option<[u8; 3]> {
    if !color_is_plain(clr) {
        return None;
    }
    match clr.local_name() {
        "srgbClr" => parse_rgb(clr.attr("val")?),
        "schemeClr" => theme_color(theme?, &mapped_name(map, clr.attr("val")?)),
        _ => None,
    }
}

fn color_is_plain(clr: &Element) -> bool {
    clr.element_children()
        .all(|child| child.local_name() == "alpha" && child.attr("val").unwrap_or("") == "100000")
}

fn mapped_name(map: Option<&Element>, name: &str) -> String {
    if let Some(map) = map {
        return map.attr(name).unwrap_or(name).to_string();
    }
    match name {
        "bg1" => "lt1",
        "tx1" => "dk1",
        "bg2" => "lt2",
        "tx2" => "dk2",
        other => other,
    }
    .to_string()
}

fn theme_color(theme: &Element, name: &str) -> Option<[u8; 3]> {
    let slot = theme
        .child_by_local("themeElements")?
        .child_by_local("clrScheme")?
        .child_by_local(name)?;
    if let Some(el) = slot.child_by_local("srgbClr") {
        return parse_rgb(el.attr("val")?);
    }
    let el = slot.child_by_local("sysClr")?;
    el.attr("lastClr")
        .and_then(parse_rgb)
        .or_else(|| match el.attr("val") {
            Some("window") => Some(WHITE),
            Some("windowText") => Some([0, 0, 0]),
            _ => None,
        })
}

fn only_child(el: &Element) -> Option<&Element> {
    let mut kids = el.element_children();
    let first = kids.next()?;
    kids.next().is_none().then_some(first)
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
    visit(pkg, slide_part, tree, &[], rgb, width, height)
}

/// 母版、版式上已经会画的对象先画。页底已经铺好，本页内容还没画。
fn paint_inherited(
    pkg: &Package,
    slide_part: &str,
    slide_root: &Element,
    rgb: &mut [u8],
    width: u32,
    height: u32,
) -> Result<(), Error> {
    if !shows_master_shapes(slide_root) {
        return Ok(());
    }
    let Some(layout_part) = linked_part(pkg, slide_part, "/slideLayout")? else {
        return Ok(());
    };
    let layout = super::list_pin::parse_part(pkg, &layout_part)?;
    if shows_master_shapes(&layout.root) {
        if let Some(master_part) = linked_part(pkg, &layout_part, "/slideMaster")? {
            let master = super::list_pin::parse_part(pkg, &master_part)?;
            paint_tree(pkg, &master_part, &master.root, rgb, width, height)?;
        }
    }
    paint_tree(pkg, &layout_part, &layout.root, rgb, width, height)
}

fn shows_master_shapes(root: &Element) -> bool {
    !matches!(
        root.attr("showMasterSp"),
        Some("0") | Some("false") | Some("FALSE")
    )
}

fn linked_part(pkg: &Package, source: &str, type_suffix: &str) -> Result<Option<String>, Error> {
    let rels = opc::Relationships::from_package(pkg, source)?;
    let Some(rel) = rels
        .rels
        .iter()
        .find(|rel| !rel.is_external() && rel.rel_type.ends_with(type_suffix))
    else {
        return Ok(None);
    };
    Ok(Some(opc::resolve_rel_target(source, &rel.target)?))
}

fn visit(
    pkg: &Package,
    slide_part: &str,
    el: &Element,
    chain: &[super::layout_check::GroupXfrm],
    rgb: &mut [u8],
    width: u32,
    height: u32,
) -> Result<(), Error> {
    for child in el.element_children() {
        match child.local_name() {
            "nvGrpSpPr" | "grpSpPr" | "extLst" => {}
            "pic" => {
                if transformed(child) || is_media(child) {
                    return Err(unsupported());
                }
                blit_pic(pkg, slide_part, child, chain, rgb, width, height)?;
            }
            "sp" => paint_sp(pkg, slide_part, child, chain, rgb, width, height)?,
            "graphicFrame" => paint_frame(pkg, slide_part, child, chain, rgb, width, height)?,
            "grpSp" => {
                if group_turns(child) {
                    return Err(unsupported());
                }
                let mut next = chain.to_vec();
                next.push(super::layout_check::group_xfrm(child));
                visit(pkg, slide_part, child, &next, rgb, width, height)?;
            }
            "AlternateContent" => {
                if let Some(choice) = child.child_by_local("Choice") {
                    visit(pkg, slide_part, choice, chain, rgb, width, height)?;
                }
            }
            _ => return Err(unsupported()),
        }
    }
    Ok(())
}

/// 不是 90 度倍数的旋转，子形状不再是实心外接矩形。翻转和直角倍数可以画。
fn group_turns(grp: &Element) -> bool {
    let Some(xfrm) = grp
        .child_by_local("grpSpPr")
        .and_then(|pr| pr.child_by_local("xfrm"))
    else {
        return false;
    };
    let Some(raw) = xfrm.attr("rot") else {
        return false;
    };
    raw.parse::<i64>()
        .ok()
        .is_none_or(|n| n.rem_euclid(5_400_000) != 0)
}

fn map_box(xfrm: &Element, chain: &[super::layout_check::GroupXfrm]) -> (i64, i64, i64, i64) {
    let (x, y, cx, cy) = xfrm_box(xfrm);
    super::layout_check::map_emu(x, y, cx, cy, chain)
}

/// 矩形、椭圆、圆角矩形和里面的文字。描边、半透明或转角对不上的矩形整块留空，页继续画。
fn paint_sp(
    pkg: &Package,
    part: &str,
    sp: &Element,
    chain: &[super::layout_check::GroupXfrm],
    rgb: &mut [u8],
    width: u32,
    height: u32,
) -> Result<(), Error> {
    let Some(sp_pr) = sp.child_by_local("spPr") else {
        return Ok(());
    };
    let prst = sp_pr
        .child_by_local("prstGeom")
        .and_then(|geom| geom.attr("prst"))
        .unwrap_or("");
    match prst {
        "rect" => paint_rect(pkg, part, sp, sp_pr, chain, rgb, width, height),
        "ellipse" => paint_ellipse(sp, sp_pr, chain, rgb, width, height),
        "roundRect" => paint_round(sp, sp_pr, chain, rgb, width, height),
        _ => Err(unsupported()),
    }
}

fn paint_rect(
    pkg: &Package,
    part: &str,
    sp: &Element,
    sp_pr: &Element,
    chain: &[super::layout_check::GroupXfrm],
    rgb: &mut [u8],
    width: u32,
    height: u32,
) -> Result<(), Error> {
    if let Some((color, x, y, w, h)) = plain_rect(sp, chain) {
        fill_rect(rgb, width, height, x, y, w, h, color);
        paint_tx(sp, x, y, w, h, rgb, width, height);
        return Ok(());
    }
    if let Some((x, y, w, h)) = plain_box(sp, chain) {
        if let Some(color) = scheme_rect_color(pkg, part, sp_pr)? {
            fill_rect(rgb, width, height, x, y, w, h, color);
            paint_tx(sp, x, y, w, h, rgb, width, height);
            return Ok(());
        }
    }
    let Some(xfrm) = sp_pr.child_by_local("xfrm") else {
        return Ok(());
    };
    if xfrm.attr("rot").is_some() {
        return Ok(());
    }
    let (x, y, cx, cy) = map_box(xfrm, chain);
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
    chain: &[super::layout_check::GroupXfrm],
    rgb: &mut [u8],
    width: u32,
    height: u32,
) -> Result<(), Error> {
    let Some(oval) = oval_box(sp_pr, chain)? else {
        return Ok(());
    };
    fill_ellipse(rgb, width, height, &oval);
    paint_tx(
        sp,
        oval.x.round() as i32,
        oval.y.round() as i32,
        oval.w.round() as i32,
        oval.h.round() as i32,
        rgb,
        width,
        height,
    );
    Ok(())
}

fn paint_round(
    sp: &Element,
    sp_pr: &Element,
    chain: &[super::layout_check::GroupXfrm],
    rgb: &mut [u8],
    width: u32,
    height: u32,
) -> Result<(), Error> {
    let Some(round) = round_box(sp_pr, chain)? else {
        return Ok(());
    };
    let inset = round
        .stroke
        .map(|_| {
            stroke_inset(round.radius_f, round.stroke_f)
                .min(round.w / 2)
                .min(round.h / 2)
        })
        .unwrap_or(0);
    if let Some(color) = round.stroke {
        if inset > 0 {
            paint_round_corners(
                rgb,
                width,
                height,
                round.x,
                round.y,
                round.edge_r,
                round.edge_b,
                inset,
                round.radius_f,
                round.stroke_f,
                round.fill,
                color,
            );
        }
    }
    fill_round(
        rgb,
        width,
        height,
        round.x,
        round.y,
        round.w,
        round.h,
        round.edge_r,
        round.edge_b,
        round.radius,
        inset,
        round.fill,
    );
    if let Some(color) = round.stroke {
        stroke_round(
            rgb,
            width,
            height,
            round.x,
            round.y,
            round.w,
            round.h,
            inset,
            round.stroke_px,
            round.edge_r,
            round.edge_b,
            color,
        );
    }
    paint_tx(sp, round.x, round.y, round.w, round.h, rgb, width, height);
    Ok(())
}

struct Oval {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    fill: RgbaFill,
    stroke: Option<[u8; 3]>,
    /// 描边宽度，单位是像素。12700 EMU 是 1.333。
    stroke_f: f64,
}

struct Round {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    fill: RgbaFill,
    radius: i32,
    /// 圆角半径，单位是像素，保留 EMU 的小数。
    radius_f: f64,
    stroke: Option<[u8; 3]>,
    stroke_px: i32,
    /// 描边宽度，单位是像素，不四舍五入。12700 EMU 是 1.333。
    stroke_f: f64,
    edge_r: i32,
    edge_b: i32,
}

fn oval_box(
    sp_pr: &Element,
    chain: &[super::layout_check::GroupXfrm],
) -> Result<Option<Oval>, Error> {
    if rotated(sp_pr) {
        return Err(unsupported());
    }
    let Some(xfrm) = sp_pr.child_by_local("xfrm") else {
        return Ok(None);
    };
    let (ex, ey, ecx, ecy) = map_box(xfrm, chain);
    let (w, h) = (emu_f(ecx), emu_f(ecy));
    if w <= 0.0 || h <= 0.0 || emu_px(ecx) <= 0 || emu_px(ecy) <= 0 {
        return Ok(None);
    }
    Ok(Some(Oval {
        x: emu_f(ex),
        y: emu_f(ey),
        w,
        h,
        fill: shape_fill(sp_pr)?,
        stroke: stroke_color(sp_pr)?,
        stroke_f: stroke_width_f(sp_pr),
    }))
}

fn round_box(
    sp_pr: &Element,
    chain: &[super::layout_check::GroupXfrm],
) -> Result<Option<Round>, Error> {
    if rotated(sp_pr) {
        return Err(unsupported());
    }
    let Some(xfrm) = sp_pr.child_by_local("xfrm") else {
        return Ok(None);
    };
    let (ex, ey, ecx, ecy) = map_box(xfrm, chain);
    let (x, y, w, h) = (emu_px(ex), emu_px(ey), emu_px(ecx), emu_px(ecy));
    if w <= 0 || h <= 0 {
        return Ok(None);
    }
    Ok(Some(Round {
        x,
        y,
        w,
        h,
        fill: shape_fill(sp_pr)?,
        radius: round_radius(sp_pr, w, h),
        radius_f: round_radius_f(sp_pr, emu_f(ecx), emu_f(ecy)),
        stroke: stroke_color(sp_pr)?,
        stroke_px: stroke_px(sp_pr),
        stroke_f: stroke_width_f(sp_pr),
        edge_r: emu_px(ex.saturating_add(ecx)),
        edge_b: emu_px(ey.saturating_add(ecy)),
    }))
}

fn rotated(sp_pr: &Element) -> bool {
    sp_pr
        .child_by_local("xfrm")
        .and_then(|xfrm| xfrm.attr("rot"))
        .is_some_and(|raw| raw.parse::<i64>().ok().unwrap_or(1) != 0)
}

#[derive(Clone, Copy)]
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

fn stroke_px(sp_pr: &Element) -> i32 {
    stroke_width_f(sp_pr).round().max(0.0) as i32
}

fn stroke_width_f(sp_pr: &Element) -> f64 {
    let raw = sp_pr
        .child_by_local("ln")
        .and_then(|ln| ln.attr("w"))
        .unwrap_or("9525");
    raw.parse::<i64>().map(emu_f).unwrap_or(0.0).max(0.0)
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
    let pos = |el: &Element| el.attr("pos").unwrap_or("0").parse::<i64>().ok();
    if pos(stops[0]) != Some(0) || pos(stops[1]) != Some(100_000) {
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

fn round_adj(sp_pr: &Element) -> i64 {
    sp_pr
        .child_by_local("prstGeom")
        .and_then(|geom| geom.child_by_local("avLst"))
        .and_then(|list| {
            list.element_children()
                .find(|el| el.attr("name") == Some("adj"))
        })
        .and_then(|gd| gd.attr("fmla"))
        .and_then(|fmla| fmla.strip_prefix("val "))
        .and_then(|raw| raw.parse::<i64>().ok())
        .unwrap_or(16_667)
}

fn round_radius(sp_pr: &Element, w: i32, h: i32) -> i32 {
    let adj = round_adj(sp_pr);
    let ss = i64::from(w.min(h).max(0));
    ((ss * adj) / 100_000).clamp(0, ss / 2) as i32
}

fn round_radius_f(sp_pr: &Element, w: f64, h: f64) -> f64 {
    let ss = w.min(h).max(0.0);
    (ss * round_adj(sp_pr) as f64 / 100_000.0).clamp(0.0, ss / 2.0)
}

/// 直边实色从这里开始。半径加上线宽后再四舍五入，拐角的过渡留在这一段外面。
fn stroke_inset(radius_f: f64, stroke_f: f64) -> i32 {
    if radius_f <= 0.0 || stroke_f <= 0.0 {
        return 0;
    }
    (radius_f + stroke_f).round() as i32
}

fn paint_frame(
    pkg: &Package,
    slide_part: &str,
    frame: &Element,
    chain: &[super::layout_check::GroupXfrm],
    rgb: &mut [u8],
    width: u32,
    height: u32,
) -> Result<(), Error> {
    let Some(xfrm) = frame.child_by_local("xfrm") else {
        return Err(unsupported());
    };
    let (x, y, cx, cy) = map_box(xfrm, chain);
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
        paint_table(pkg, tbl, rgb, width, height, x, y)?;
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
    paint_image(rgb, width, height, x, y, w as u32, h as u32, &image, None);
    Ok(())
}

fn paint_table(
    pkg: &Package,
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
    let fonts = minor_fonts(pkg);
    let header_style = header_is_bold_light(tbl);
    let mut py = y;
    for (row_index, tr) in tbl
        .element_children()
        .filter(|el| el.local_name() == "tr")
        .enumerate()
    {
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
            paint_cell_text(
                tc,
                &fonts,
                header_style && row_index == 0,
                px,
                py,
                cw,
                rh,
                rgb,
                width,
                height,
            );
            px += cw;
        }
        py += rh;
    }
    Ok(())
}

/// PowerPoint 中等样式 2 - 强调 1。包里通常只有这个 GUID，样式正文不在部件里。
const MEDIUM_STYLE_2_ACCENT_1: &str = "{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}";

struct MinorFonts {
    latin: String,
    ea: String,
}

fn minor_fonts(pkg: &Package) -> MinorFonts {
    let mut fonts = MinorFonts {
        latin: "Calibri".to_string(),
        ea: "微软雅黑".to_string(),
    };
    let Ok(bytes) = pkg.get("/ppt/theme/theme1.xml") else {
        return fonts;
    };
    let Ok(doc) = crate::xml::parse(bytes) else {
        return fonts;
    };
    let Some(minor) = doc
        .root
        .child_by_local("themeElements")
        .and_then(|el| el.child_by_local("fontScheme"))
        .and_then(|el| el.child_by_local("minorFont"))
    else {
        return fonts;
    };
    if let Some(name) = named_typeface(minor, "latin") {
        fonts.latin = name;
    }
    if let Some(name) = named_typeface(minor, "ea") {
        fonts.ea = name;
    }
    fonts
}

fn named_typeface(parent: &Element, slot: &str) -> Option<String> {
    let name = parent
        .child_by_local(slot)?
        .attr("typeface")?
        .trim()
        .to_string();
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

fn header_is_bold_light(tbl: &Element) -> bool {
    let Some(pr) = tbl.child_by_local("tblPr") else {
        return false;
    };
    let first = matches!(pr.attr("firstRow"), Some("1" | "true"));
    if !first {
        return false;
    }
    pr.child_by_local("tableStyleId")
        .map(|el| el.text_content())
        .is_some_and(|id| id.trim().eq_ignore_ascii_case(MEDIUM_STYLE_2_ACCENT_1))
}

fn paint_cell_text(
    tc: &Element,
    fonts: &MinorFonts,
    header: bool,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    rgb: &mut [u8],
    width: u32,
    height: u32,
) {
    let Some(tx) = tc.child_by_local("txBody") else {
        return;
    };
    let pr = tc.child_by_local("tcPr");
    let inset = |name: &str, default: i64| {
        pr.and_then(|el| el.attr(name))
            .and_then(|raw| raw.parse().ok())
            .map(emu_px)
            .unwrap_or_else(|| emu_px(default))
    };
    let left = x + inset("marL", 91_440);
    let top = y + inset("marT", 45_720);
    let right = x + w - inset("marR", 91_440);
    let bottom = y + h - inset("marB", 45_720);
    let anchor = pr.and_then(|el| el.attr("anchor")).unwrap_or("t");
    let mut lines = Vec::new();
    for para in tx.element_children().filter(|el| el.local_name() == "p") {
        if let Some(line) = cell_line(para, fonts, header) {
            lines.push(line);
        }
    }
    paint_text_block(&lines, left, top, right, bottom, anchor, rgb, width, height);
}

fn cell_line(para: &Element, fonts: &MinorFonts, header: bool) -> Option<TextLine> {
    let mut line = para_line(para)?;
    if line.latin.is_empty() {
        line.latin.clone_from(&fonts.latin);
    }
    if line.ea.is_empty() {
        line.ea.clone_from(&fonts.ea);
    }
    let rpr = run_rpr(para);
    if header && rpr.and_then(|el| el.attr("b")).is_none() {
        line.bold = true;
    }
    if header && rpr.and_then(|el| el.child_by_local("solidFill")).is_none() {
        line.color = [255, 255, 255];
    }
    Some(line)
}

fn run_rpr(para: &Element) -> Option<&Element> {
    para.element_children()
        .find(|el| el.local_name() == "r" && el.child_by_local("t").is_some())?
        .child_by_local("rPr")
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
    paint_text_block(&lines, left, top, right, bottom, anchor, rgb, width, height);
}

fn paint_text_block(
    lines: &[TextLine],
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
    anchor: &str,
    rgb: &mut [u8],
    width: u32,
    height: u32,
) {
    if lines.is_empty() {
        return;
    }
    let inner_w = right - left;
    let inner_h = bottom - top;
    if inner_w <= 0 || inner_h <= 0 {
        return;
    }
    let heights: Vec<f64> = lines.iter().map(line_em).collect();
    let block: f64 = heights.iter().sum();
    let mut cursor_y = match anchor {
        "ctr" => f64::from(top) + (f64::from(inner_h) - block) / 2.0,
        "b" => f64::from(bottom) - block,
        _ => f64::from(top),
    };
    for (line, em) in lines.iter().zip(heights) {
        if em <= 0.0 {
            continue;
        }
        let wrapped = wrap_text(&line.text, f64::from(inner_w), |piece| {
            piece_width(line, piece)
        });
        for (index, piece) in wrapped.iter().enumerate() {
            if index > 0 {
                cursor_y += em;
            }
            let tw = piece_width(line, piece);
            let mut cursor_x = match line.align.as_str() {
                "ctr" => f64::from(left) + (f64::from(inner_w) - tw) / 2.0,
                "r" => f64::from(right) - tw,
                _ => f64::from(left),
            };
            let spans = face_spans(line, piece);
            let line_ascent = spans
                .iter()
                .filter_map(|(name, _)| {
                    Face::open_named(line.px, line.bold, name).map(|face| face.line_height())
                })
                .fold(0.0f64, f64::max);
            for (name, span) in spans {
                let Some(face) = Face::open_named(line.px, line.bold, &name) else {
                    continue;
                };
                let draw_top = cursor_y + (line_ascent - face.line_height());
                face.draw(&span, cursor_x, draw_top, line.color, |cx, cy, ink, cov| {
                    blend(rgb, width, height, cx, cy, ink, cov);
                });
                cursor_x += face.width(&span);
            }
        }
        cursor_y += em;
    }
}

struct TextLine {
    text: String,
    px: f32,
    bold: bool,
    color: [u8; 3],
    latin: String,
    ea: String,
    cs: String,
    sym: String,
    lang: String,
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
        latin: typeface(rpr, "latin"),
        ea: typeface(rpr, "ea"),
        cs: typeface(rpr, "cs"),
        sym: typeface(rpr, "sym"),
        lang: rpr.and_then(|el| el.attr("lang")).unwrap_or("").to_string(),
        align,
    })
}

fn typeface(rpr: Option<&Element>, slot: &str) -> String {
    rpr.and_then(|el| el.child_by_local(slot))
        .and_then(|el| el.attr("typeface"))
        .unwrap_or("")
        .to_string()
}

/// ECMA-376 21.1.2.3：每个字符四选一。表里没点名的用东亚字体。
enum FontSlot {
    Latin,
    Ea,
    Cs,
    Sym,
}

fn font_slot(ch: char, lang: &str) -> FontSlot {
    let n = ch as u32;
    if (0x2018..=0x201F).contains(&n) && quote_uses_ea(lang) {
        return FontSlot::Ea;
    }
    if (0xF000..=0xF0FF).contains(&n) {
        return FontSlot::Sym;
    }
    if latin_range(n) {
        return FontSlot::Latin;
    }
    if cs_range(n) {
        return FontSlot::Cs;
    }
    FontSlot::Ea
}

fn quote_uses_ea(lang: &str) -> bool {
    matches!(
        lang,
        "ii-CN" | "ja-JP" | "ko-KR" | "zh-CN" | "zh-HK" | "zh-MO" | "zh-SG" | "zh-TW"
    )
}

fn latin_range(n: u32) -> bool {
    matches!(
        n,
        0x0000..=0x007F
            | 0x0080..=0x00A6
            | 0x00A9..=0x00AF
            | 0x00B2..=0x00B3
            | 0x00B5..=0x00D6
            | 0x00D8..=0x00F6
            | 0x00F8..=0x058F
            | 0x10A0..=0x10FF
            | 0x1200..=0x137F
            | 0x13A0..=0x177F
            | 0x1D00..=0x1D7F
            | 0x1D400..=0x1D7FF
            | 0x1E00..=0x1FFF
            | 0x2000..=0x200B
            | 0x2010..=0x2029
            | 0x2030..=0x2046
            | 0x204A..=0x245F
            | 0x27C0..=0x2BFF
            | 0xFB00..=0xFB17
            | 0xFE50..=0xFE6F
    )
}

fn cs_range(n: u32) -> bool {
    matches!(
        n,
        0x0590..=0x074F
            | 0x0780..=0x07BF
            | 0x0900..=0x109F
            | 0x1780..=0x18AF
            | 0x200C..=0x200F
            | 0x202A..=0x202F
            | 0x2670..=0x2671
            | 0xFB1D..=0xFB4F
    )
}

/// 槽上没写字体时，退回同一 run 里已经写明的拉丁或东亚字体。
fn span_name(line: &TextLine, ch: char) -> String {
    let picked = match font_slot(ch, &line.lang) {
        FontSlot::Latin => line.latin.as_str(),
        FontSlot::Ea => line.ea.as_str(),
        FontSlot::Cs => line.cs.as_str(),
        FontSlot::Sym => line.sym.as_str(),
    };
    if !picked.is_empty() {
        picked.to_string()
    } else if !line.latin.is_empty() {
        line.latin.clone()
    } else if !line.ea.is_empty() {
        line.ea.clone()
    } else {
        "微软雅黑".to_string()
    }
}

fn face_spans(line: &TextLine, text: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    for ch in text.chars() {
        let name = span_name(line, ch);
        if let Some((prev, buf)) = out.last_mut() {
            if prev == &name {
                buf.push(ch);
                continue;
            }
        }
        out.push((name, ch.to_string()));
    }
    out
}

fn piece_width(line: &TextLine, text: &str) -> f64 {
    face_spans(line, text)
        .into_iter()
        .map(|(name, span)| {
            Face::open_named(line.px, line.bold, &name)
                .map(|face| face.width(&span))
                .unwrap_or(0.0)
        })
        .sum()
}

fn line_em(line: &TextLine) -> f64 {
    let mut em = 0.0f64;
    let mut seen: Vec<String> = Vec::new();
    for ch in line.text.chars() {
        let name = span_name(line, ch);
        if seen.iter().any(|item| item == &name) {
            continue;
        }
        seen.push(name.clone());
        if let Some(face) = Face::open_named(line.px, line.bold, &name) {
            em = em.max(face.em_box());
        }
    }
    em
}

fn wrap_text(text: &str, max_w: f64, width: impl Fn(&str) -> f64) -> Vec<String> {
    if max_w <= 0.0 || width(text) <= max_w {
        return vec![text.to_string()];
    }
    let mut lines = Vec::new();
    let mut cur = String::new();
    for ch in text.chars() {
        let mut trial = cur.clone();
        trial.push(ch);
        if width(&trial) > max_w && !cur.is_empty() {
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

/// Office 16.0 两档渐变里，第二档在线性光下的份额。
/// 黑到白、高 720 的导出在四分之一、一半、四分之三处是 105、186、238。
/// 份额由这些字节按伽马 2.2 反推，并收成 `w(1 - t) = 1 - w(t)`。
const GRAD_WEIGHT: [f64; 17] = [
    0.0, 0.0180, 0.0444, 0.0856, 0.1414, 0.2132, 0.2982, 0.3969, 0.5, 0.6031, 0.7018, 0.7868,
    0.8586, 0.9144, 0.9556, 0.9820, 1.0,
];

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
    let quarter = turn / RIGHT_ANGLE;
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
            let along = match quarter {
                1 => (f64::from(row) + 0.5) / f64::from(h),
                2 => (f64::from(w - 1 - col) + 0.5) / f64::from(w),
                3 => (f64::from(h - 1 - row) + 0.5) / f64::from(h),
                _ => (f64::from(col) + 0.5) / f64::from(w),
            };
            let share = grad_weight(along);
            let color = [
                gamma_mix(c0[0], c1[0], share),
                gamma_mix(c0[1], c1[1], share),
                gamma_mix(c0[2], c1[2], share),
            ];
            let i = (py as usize * width as usize + px as usize) * 3;
            rgb[i..i + 3].copy_from_slice(&color);
        }
    }
}

fn grad_weight(t: f64) -> f64 {
    let x = t.clamp(0.0, 1.0) * 16.0;
    let i = (x.floor() as usize).min(GRAD_WEIGHT.len() - 2);
    let f = x - i as f64;
    GRAD_WEIGHT[i] * (1.0 - f) + GRAD_WEIGHT[i + 1] * f
}

fn gamma_mix(start: u8, end: u8, share: f64) -> u8 {
    let from = (f64::from(start) / 255.0).powf(2.2);
    let to = (f64::from(end) / 255.0).powf(2.2);
    let mixed = (from + (to - from) * share).clamp(0.0, 1.0);
    (mixed.powf(1.0 / 2.2) * 255.0).round().clamp(0.0, 255.0) as u8
}

/// 画椭圆。有描边时外接矩形右和下各加 1 像素，和 PowerPoint 16.0 序号圆一致。
/// 边缘交给 GDI+ 的抗锯齿。GDI+ 不可用时退回半像素覆盖。
fn fill_ellipse(rgb: &mut [u8], width: u32, height: u32, oval: &Oval) {
    if fill_ellipse_device(rgb, width, height, oval) {
        return;
    }
    fill_ellipse_ramp(rgb, width, height, oval);
}

/// 左上角和右下角四舍五入。有描边时宽高再加 1 像素。
fn ellipse_device_box(oval: &Oval, grow: bool) -> Option<(i32, i32, i32, i32)> {
    let left = oval.x.round() as i32;
    let top = oval.y.round() as i32;
    let right = (oval.x + oval.w).round() as i32;
    let bottom = (oval.y + oval.h).round() as i32;
    let extra = if grow { 1 } else { 0 };
    let w = right - left + extra;
    let h = bottom - top + extra;
    (w > 0 && h > 0).then_some((left, top, w, h))
}

fn paint_device_ellipse(
    rgb: &mut [u8],
    width: u32,
    height: u32,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    color: [u8; 3],
    alpha: u8,
) -> bool {
    fill_device_ellipse(x, y, w, h, |px, py, cov| {
        let cov = ((u16::from(cov) * u16::from(alpha) + 127) / 255) as u8;
        blend(rgb, width, height, px, py, color, cov);
    })
}

fn fill_ellipse_device(rgb: &mut [u8], width: u32, height: u32, oval: &Oval) -> bool {
    let same = oval
        .stroke
        .is_some_and(|color| oval.fill.on && oval.fill.alpha == 255 && color == oval.fill.color);
    if same || oval.stroke.is_none() {
        if !oval.fill.on {
            return true;
        }
        let Some((x, y, w, h)) = ellipse_device_box(oval, oval.stroke.is_some()) else {
            return true;
        };
        return paint_device_ellipse(
            rgb,
            width,
            height,
            x,
            y,
            w,
            h,
            oval.fill.color,
            oval.fill.alpha,
        );
    }
    let Some(stroke) = oval.stroke else {
        return true;
    };
    // 半透明填充若先铺整圆描边，圆心会变成叠在描边色上的平色。
    if oval.fill.on && oval.fill.alpha < 255 {
        return paint_translucent_stroked_ellipse(rgb, width, height, oval, stroke);
    }
    let Some((x, y, w, h)) = ellipse_device_box(oval, true) else {
        return true;
    };
    if !paint_device_ellipse(rgb, width, height, x, y, w, h, stroke, 255) {
        return false;
    }
    if !oval.fill.on {
        return true;
    }
    let Some((x, y, w, h)) = ellipse_device_box(oval, false) else {
        return true;
    };
    paint_device_ellipse(
        rgb,
        width,
        height,
        x,
        y,
        w,
        h,
        oval.fill.color,
        oval.fill.alpha,
    )
}

/// 半透明填充叠在已画像素上，描边只取外接矩形外侧那一圈的覆盖差。
fn paint_translucent_stroked_ellipse(
    rgb: &mut [u8],
    width: u32,
    height: u32,
    oval: &Oval,
    stroke: [u8; 3],
) -> bool {
    let Some((ox, oy, ow, oh)) = ellipse_device_box(oval, true) else {
        return true;
    };
    let Some((ix, iy, iw, ih)) = ellipse_device_box(oval, false) else {
        return true;
    };
    let Some(outer) = ellipse_coverage(ox, oy, ow, oh) else {
        return false;
    };
    let Some(inner) = ellipse_coverage(ix, iy, iw, ih) else {
        return false;
    };
    for ly in 0..outer.h {
        for lx in 0..outer.w {
            let px = outer.x + lx;
            let py = outer.y + ly;
            let grown = outer.cov[ly as usize * outer.w as usize + lx as usize];
            let fill_cov = cover_at(&inner, px, py);
            if fill_cov > 0 {
                let cov = ((u16::from(fill_cov) * u16::from(oval.fill.alpha) + 127) / 255) as u8;
                blend(rgb, width, height, px, py, oval.fill.color, cov);
            }
            let stroke_cov = grown.saturating_sub(fill_cov);
            if stroke_cov > 0 {
                blend(rgb, width, height, px, py, stroke, stroke_cov);
            }
        }
    }
    true
}

struct EllipseCover {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    cov: Vec<u8>,
}

/// 记下 GDI+ 椭圆的页面覆盖。外扩 2 像素，把抗锯齿边收进来。
fn ellipse_coverage(x: i32, y: i32, w: i32, h: i32) -> Option<EllipseCover> {
    const PAD: i32 = 2;
    let origin_x = x - PAD;
    let origin_y = y - PAD;
    let bw = w + PAD * 2;
    let bh = h + PAD * 2;
    if bw <= 0 || bh <= 0 {
        return None;
    }
    let mut cov = vec![0u8; bw as usize * bh as usize];
    let ok = fill_device_ellipse(x, y, w, h, |px, py, sample| {
        let lx = px - origin_x;
        let ly = py - origin_y;
        if lx < 0 || ly < 0 || lx >= bw || ly >= bh {
            return;
        }
        let i = ly as usize * bw as usize + lx as usize;
        if sample > cov[i] {
            cov[i] = sample;
        }
    });
    ok.then_some(EllipseCover {
        x: origin_x,
        y: origin_y,
        w: bw,
        h: bh,
        cov,
    })
}

fn cover_at(map: &EllipseCover, px: i32, py: i32) -> u8 {
    let lx = px - map.x;
    let ly = py - map.y;
    if lx < 0 || ly < 0 || lx >= map.w || ly >= map.h {
        return 0;
    }
    map.cov[ly as usize * map.w as usize + lx as usize]
}

fn fill_ellipse_ramp(rgb: &mut [u8], width: u32, height: u32, oval: &Oval) {
    let path_rx = oval.w / 2.0;
    let path_ry = oval.h / 2.0;
    if path_rx <= 0.0 || path_ry <= 0.0 {
        return;
    }
    let inset = if oval.stroke.is_some() {
        oval.stroke_f / 2.0
    } else {
        0.0
    };
    let fill_rx = (path_rx - inset).max(0.0);
    let fill_ry = (path_ry - inset).max(0.0);
    let cx = oval.x + path_rx;
    let cy = oval.y + path_ry;
    let same = oval
        .stroke
        .is_some_and(|color| oval.fill.on && color == oval.fill.color);
    let outer_rx = if same { fill_rx } else { path_rx + inset };
    let outer_ry = if same { fill_ry } else { path_ry + inset };
    let x0 = (cx - outer_rx - 1.0).floor() as i32;
    let x1 = (cx + outer_rx + 1.0).ceil() as i32;
    let y0 = (cy - outer_ry - 1.0).floor() as i32;
    let y1 = (cy + outer_ry + 1.0).ceil() as i32;
    for py in y0..y1 {
        for px in x0..x1 {
            let fill_cov = if oval.fill.on && fill_rx > 0.0 && fill_ry > 0.0 {
                ellipse_cover(px, py, cx, cy, fill_rx, fill_ry)
            } else {
                0.0
            };
            if let Some(color) = oval.stroke {
                if !same && outer_rx > 0.0 && outer_ry > 0.0 {
                    let outer = ellipse_cover(px, py, cx, cy, outer_rx, outer_ry);
                    let stroke_cov = (outer - fill_cov).clamp(0.0, 1.0);
                    if oval.fill.on && fill_cov > 0.0 {
                        blend(
                            rgb,
                            width,
                            height,
                            px,
                            py,
                            oval.fill.color,
                            cover_byte(fill_cov, oval.fill.alpha),
                        );
                    }
                    if stroke_cov > 0.0 {
                        blend(
                            rgb,
                            width,
                            height,
                            px,
                            py,
                            color,
                            cover_byte(stroke_cov, 255),
                        );
                    }
                    continue;
                }
            }
            if fill_cov > 0.0 {
                blend(
                    rgb,
                    width,
                    height,
                    px,
                    py,
                    oval.fill.color,
                    cover_byte(fill_cov, oval.fill.alpha),
                );
            }
        }
    }
}

/// 像素中心到椭圆边界的距离，向外为正。正圆时就是半径差。
fn ellipse_signed(px: i32, py: i32, cx: f64, cy: f64, rx: f64, ry: f64) -> f64 {
    let nx = (f64::from(px) + 0.5 - cx) / rx;
    let ny = (f64::from(py) + 0.5 - cy) / ry;
    let unit = (nx * nx + ny * ny).sqrt();
    (unit - 1.0) * rx.min(ry)
}

fn ellipse_cover(px: i32, py: i32, cx: f64, cy: f64, rx: f64, ry: f64) -> f64 {
    (0.5 - ellipse_signed(px, py, cx, cy, rx, ry)).clamp(0.0, 1.0)
}

fn cover_byte(cover: f64, alpha: u8) -> u8 {
    (cover * f64::from(alpha)).round().clamp(0.0, 255.0) as u8
}

fn fill_round(
    rgb: &mut [u8],
    width: u32,
    height: u32,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    edge_r: i32,
    edge_b: i32,
    radius: i32,
    inset: i32,
    fill: RgbaFill,
) {
    if !fill.on {
        return;
    }
    for row in 0..h {
        let py = y + row;
        for col in 0..w {
            let px = x + col;
            if inset > 0 && in_corner_window(px, py, x, y, edge_r, edge_b, inset) {
                continue;
            }
            if inside_round(px, py, x, y, w, h, radius) {
                blend(rgb, width, height, px, py, fill.color, fill.alpha);
            }
        }
    }
}

fn in_corner_window(
    px: i32,
    py: i32,
    x: i32,
    y: i32,
    edge_r: i32,
    edge_b: i32,
    inset: i32,
) -> bool {
    let on_x = (px >= x - 2 && px < x + inset) || (px >= edge_r - inset && px < edge_r + 3);
    let on_y = (py >= y - 2 && py < y + inset) || (py >= edge_b - inset && py < edge_b + 3);
    on_x && on_y
}

/// 四个拐角的圆弧。圆心落在直边描边像素的中心上，半径是 EMU 圆角。
/// 覆盖见 `corner_cover`。直边实色不在这里画。
fn paint_round_corners(
    rgb: &mut [u8],
    width: u32,
    height: u32,
    x: i32,
    y: i32,
    edge_r: i32,
    edge_b: i32,
    inset: i32,
    radius_f: f64,
    stroke_f: f64,
    fill: RgbaFill,
    stroke: [u8; 3],
) {
    if radius_f <= 0.0 || stroke_f <= 0.0 || inset <= 0 {
        return;
    }
    let centers = [
        (f64::from(x) + 0.5 + radius_f, f64::from(y) + 0.5 + radius_f),
        (
            f64::from(edge_r) + 0.5 - radius_f,
            f64::from(y) + 0.5 + radius_f,
        ),
        (
            f64::from(x) + 0.5 + radius_f,
            f64::from(edge_b) + 0.5 - radius_f,
        ),
        (
            f64::from(edge_r) + 0.5 - radius_f,
            f64::from(edge_b) + 0.5 - radius_f,
        ),
    ];
    let windows = [
        (x - 2, y - 2, x + inset, y + inset),
        (edge_r - inset, y - 2, edge_r + 3, y + inset),
        (x - 2, edge_b - inset, x + inset, edge_b + 3),
        (edge_r - inset, edge_b - inset, edge_r + 3, edge_b + 3),
    ];
    for (center, window) in centers.into_iter().zip(windows) {
        for py in window.1..window.3 {
            for px in window.0..window.2 {
                let (fill_cov, stroke_cov) = corner_cover(px, py, center.0, center.1, radius_f);
                if fill_cov <= 0.0 && stroke_cov <= 0.0 {
                    continue;
                }
                let fill_cov = if fill.on {
                    fill_cov * f64::from(fill.alpha) / 255.0
                } else {
                    0.0
                };
                cover_pixel(
                    rgb, width, height, px, py, fill.color, fill_cov, stroke, stroke_cov,
                );
            }
        }
    }
}

/// 拐角描边的覆盖。峰在半径内侧 0.15 像素，向内 0.9 像素、向外 0.65 像素收到 0。
/// 这是 PowerPoint 16.0 在目录页和经营分析页卡片上的读数，不是实心环。
/// 填充只占描边没盖住的内侧，避免把页底混进卡片里面。
fn corner_cover(px: i32, py: i32, cx: f64, cy: f64, radius: f64) -> (f64, f64) {
    let dx = f64::from(px) + 0.5 - cx;
    let dy = f64::from(py) + 0.5 - cy;
    let signed = (dx * dx + dy * dy).sqrt() - radius;
    let off = signed - CORNER_SHIFT;
    let half = if off < 0.0 {
        CORNER_HALF_IN
    } else {
        CORNER_HALF_OUT
    };
    let stroke = (1.0 - off.abs() / half).clamp(0.0, 1.0);
    let shape = ((CORNER_FILL_EDGE - signed) / CORNER_FILL_WIDTH).clamp(0.0, 1.0);
    ((1.0 - stroke) * shape, stroke)
}

/// 覆盖峰相对圆弧半径的偏移。负值表示峰在半径内侧。
const CORNER_SHIFT: f64 = -0.15;
const CORNER_HALF_IN: f64 = 0.90;
const CORNER_HALF_OUT: f64 = 0.65;
/// 填充从这里往里铺满。再往外只剩描边和页底。
const CORNER_FILL_EDGE: f64 = -0.05;
const CORNER_FILL_WIDTH: f64 = 0.8;

fn cover_pixel(
    rgb: &mut [u8],
    width: u32,
    height: u32,
    x: i32,
    y: i32,
    fill: [u8; 3],
    fill_cov: f64,
    stroke: [u8; 3],
    stroke_cov: f64,
) {
    if x < 0 || y < 0 || x >= width as i32 || y >= height as i32 {
        return;
    }
    let stroke_cov = stroke_cov.clamp(0.0, 1.0);
    let fill_cov = fill_cov.clamp(0.0, 1.0 - stroke_cov);
    let rest = 1.0 - fill_cov - stroke_cov;
    if fill_cov + stroke_cov <= 0.0 {
        return;
    }
    let i = (y as usize * width as usize + x as usize) * 3;
    for k in 0..3 {
        let v = f64::from(stroke[k]) * stroke_cov
            + f64::from(fill[k]) * fill_cov
            + f64::from(rgb[i + k]) * rest;
        rgb[i + k] = v.round().clamp(0.0, 255.0) as u8;
    }
}

fn stroke_round(
    rgb: &mut [u8],
    width: u32,
    height: u32,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    inset: i32,
    stroke_px: i32,
    edge_r: i32,
    edge_b: i32,
    color: [u8; 3],
) {
    if stroke_px <= 0 || w <= 0 || h <= 0 {
        return;
    }
    let sw = stroke_px;
    let x1 = (x + w).max(edge_r + sw);
    let y1 = (y + h).max(edge_b + sw);
    for py in y..y1 {
        for px in x..x1 {
            if straight_edge(px, py, x, y, w, h, inset, sw, edge_r, edge_b) {
                blend(rgb, width, height, px, py, color, 255);
            }
        }
    }
}

/// 直边落在 EMU 边线四舍五入的那一像素。右和下用起点加长度之后再取整，
/// 不能把已经取整的宽高再加回去。`inset` 以内是拐角圆弧，这里不画。
fn straight_edge(
    px: i32,
    py: i32,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    inset: i32,
    sw: i32,
    edge_r: i32,
    edge_b: i32,
) -> bool {
    let on_side = py >= y + inset
        && py < y + h - inset
        && ((px >= x && px < x + sw) || (px >= edge_r && px < edge_r + sw));
    let on_bar = px >= x + inset
        && px < x + w - inset
        && ((py >= y && py < y + sw) || (py >= edge_b && py < edge_b + sw));
    on_side || on_bar
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

fn plain_rect(
    sp: &Element,
    chain: &[super::layout_check::GroupXfrm],
) -> Option<([u8; 3], i32, i32, i32, i32)> {
    let sp_pr = sp.child_by_local("spPr")?;
    let color = opaque_rgb(sp_pr.child_by_local("solidFill")?)?;
    let (x, y, w, h) = plain_box(sp, chain)?;
    Some((color, x, y, w, h))
}

/// 没有效果、没有描边的矩形像素框。填充另判。
fn plain_box(
    sp: &Element,
    chain: &[super::layout_check::GroupXfrm],
) -> Option<(i32, i32, i32, i32)> {
    let sp_pr = sp.child_by_local("spPr")?;
    if sp_pr
        .child_by_local("effectLst")
        .is_some_and(|effects| effects.element_children().next().is_some())
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
    let (x, y, cx, cy) = xfrm_box(xfrm);
    let (x, y, cx, cy) = turned_box(x, y, cx, cy, xfrm.attr("rot"))?;
    let (x, y, cx, cy) = super::layout_check::map_emu(x, y, cx, cy, chain);
    let w = emu_px(cx);
    let h = emu_px(cy);
    if w <= 0 || h <= 0 {
        return None;
    }
    Some((emu_px(x), emu_px(y), w, h))
}

/// 实心矩形上没有颜色变换的 `schemeClr`。不是这种填充时返回 `None`，解析不了则整页失败。
fn scheme_rect_color(pkg: &Package, part: &str, sp_pr: &Element) -> Result<Option<[u8; 3]>, Error> {
    let Some(fill) = sp_pr.child_by_local("solidFill") else {
        return Ok(None);
    };
    let Some(clr) = only_child(fill) else {
        return Ok(None);
    };
    if clr.local_name() != "schemeClr" {
        return Ok(None);
    }
    scheme_rgb(pkg, part, clr).ok_or_else(unsupported).map(Some)
}

/// 按这块所在部件的配色表和母版主题取色。规则与页底的 `plain_color` 相同。
fn scheme_rgb(pkg: &Package, part: &str, clr: &Element) -> Option<[u8; 3]> {
    if part.contains("/slideMasters/") {
        let master = super::list_pin::parse_part(pkg, part).ok()?;
        let theme = theme_doc(pkg, part)?;
        return plain_color(clr, master.root.child_by_local("clrMap"), Some(&theme.root));
    }
    if part.contains("/slideLayouts/") {
        let layout = super::list_pin::parse_part(pkg, part).ok()?;
        let master_part = linked_part(pkg, part, "/slideMaster").ok().flatten()?;
        let master = super::list_pin::parse_part(pkg, &master_part).ok()?;
        let theme = theme_doc(pkg, &master_part)?;
        let map = override_map(&layout.root).or_else(|| master.root.child_by_local("clrMap"));
        return plain_color(clr, map, Some(&theme.root));
    }
    let slide = super::list_pin::parse_part(pkg, part).ok()?;
    let layout_part = linked_part(pkg, part, "/slideLayout").ok().flatten();
    let layout = match &layout_part {
        Some(path) => super::list_pin::parse_part(pkg, path).ok(),
        None => None,
    };
    let master_part = match &layout_part {
        Some(path) => linked_part(pkg, path, "/slideMaster").ok().flatten(),
        None => None,
    };
    let master = match &master_part {
        Some(path) => super::list_pin::parse_part(pkg, path).ok(),
        None => None,
    };
    let theme = match &master_part {
        Some(path) => theme_doc(pkg, path),
        None => None,
    };
    let map = color_map(
        &slide.root,
        layout.as_ref().map(|doc| &doc.root),
        master.as_ref().map(|doc| &doc.root),
    );
    plain_color(clr, map, theme.as_ref().map(|doc| &doc.root))
}

fn theme_doc(pkg: &Package, master_part: &str) -> Option<crate::xml::Document> {
    let part = linked_part(pkg, master_part, "/theme").ok().flatten()?;
    super::list_pin::parse_part(pkg, &part).ok()
}

fn line_is_clear(sp_pr: &Element) -> bool {
    let Some(ln) = sp_pr.child_by_local("ln") else {
        return true;
    };
    ln.child_by_local("noFill").is_some()
        && ln.element_children().all(|child| {
            !matches!(
                child.local_name(),
                "solidFill" | "gradFill" | "pattFill" | "blipFill"
            )
        })
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
    chain: &[super::layout_check::GroupXfrm],
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
    let (x, y, cx, cy) = pic_box(pic, chain);
    let dest_w = emu_px(cx);
    let dest_h = emu_px(cy);
    if dest_w <= 0 || dest_h <= 0 {
        return Err(unsupported());
    }
    let dest_x = emu_px(x);
    let dest_y = emu_px(y);
    let dest_w = dest_w as u32;
    let dest_h = dest_h as u32;
    let crop = source_window(pic, image.w, image.h, dest_w, dest_h)?;
    paint_image(
        rgb, width, height, dest_x, dest_y, dest_w, dest_h, &image, crop,
    );
    Ok(())
}

/// 裁剪四边落到整像素，且裁完尺寸等于目标时，返回原图上的这块。
/// 没有裁剪时返回 `None`，沿用原来的取样。对不齐的裁剪让整页失败。
fn source_window(
    pic: &Element,
    image_w: u32,
    image_h: u32,
    dest_w: u32,
    dest_h: u32,
) -> Result<Option<(u32, u32, u32, u32)>, Error> {
    let Some(fill) = pic.child_by_local("blipFill") else {
        return Ok(None);
    };
    let Some(rect) = fill.child_by_local("srcRect") else {
        return Ok(None);
    };
    let (l, t, r, b) = (
        percent_attr(rect, "l")?,
        percent_attr(rect, "t")?,
        percent_attr(rect, "r")?,
        percent_attr(rect, "b")?,
    );
    if l == 0 && t == 0 && r == 0 && b == 0 {
        return Ok(None);
    }
    if fill.child_by_local("tile").is_some() || fill_rect_moves(fill) {
        return Err(unsupported());
    }
    let left = percent_px(l, image_w).ok_or_else(unsupported)?;
    let top = percent_px(t, image_h).ok_or_else(unsupported)?;
    let right = percent_px(r, image_w).ok_or_else(unsupported)?;
    let bottom = percent_px(b, image_h).ok_or_else(unsupported)?;
    let Some(src_w) = image_w.checked_sub(left).and_then(|w| w.checked_sub(right)) else {
        return Err(unsupported());
    };
    let Some(src_h) = image_h.checked_sub(top).and_then(|h| h.checked_sub(bottom)) else {
        return Err(unsupported());
    };
    if src_w == 0 || src_h == 0 || src_w != dest_w || src_h != dest_h {
        return Err(unsupported());
    }
    Ok(Some((left, top, src_w, src_h)))
}

fn percent_attr(el: &Element, name: &str) -> Result<i64, Error> {
    match el.attr(name) {
        None => Ok(0),
        Some(raw) => raw.parse().map_err(|_| unsupported()),
    }
}

/// 百分比换成像素。离整像素超过半个百分比量化步长时不认。
fn percent_px(pct: i64, extent: u32) -> Option<u32> {
    if pct < 0 {
        return None;
    }
    let prod = i128::from(pct) * i128::from(extent);
    let n = (prod + 50_000) / 100_000;
    let err = (prod - n * 100_000).abs();
    if err * 2 > i128::from(extent) || n > i128::from(extent) {
        return None;
    }
    Some(n as u32)
}

fn fill_rect_moves(fill: &Element) -> bool {
    let Some(rect) = fill
        .child_by_local("stretch")
        .and_then(|stretch| stretch.child_by_local("fillRect"))
    else {
        return false;
    };
    ["l", "t", "r", "b"].iter().any(|name| {
        rect.attr(name)
            .is_some_and(|raw| raw.parse::<i64>().ok() != Some(0))
    })
}

fn pic_box(pic: &Element, chain: &[super::layout_check::GroupXfrm]) -> (i64, i64, i64, i64) {
    pic.child_by_local("spPr")
        .and_then(|sp| sp.child_by_local("xfrm"))
        .map(|xfrm| map_box(xfrm, chain))
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
    crop: Option<(u32, u32, u32, u32)>,
) {
    let (sx0, sy0) = crop.map(|(x, y, _, _)| (x, y)).unwrap_or((0, 0));
    for row in 0..dest_h {
        let y = dest_y + row as i32;
        if y < 0 || y >= height as i32 {
            continue;
        }
        let sy = if crop.is_some() {
            sy0 + row
        } else {
            ((row as u64 * image.h as u64) / dest_h as u64).min(image.h as u64 - 1) as u32
        };
        for col in 0..dest_w {
            let x = dest_x + col as i32;
            if x < 0 || x >= width as i32 {
                continue;
            }
            let sx = if crop.is_some() {
                sx0 + col
            } else {
                ((col as u64 * image.w as u64) / dest_w as u64).min(image.w as u64 - 1) as u32
            };
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

fn emu_f(emu: i64) -> f64 {
    (emu as f64) * DPI / EMU_PER_INCH
}

fn emu_px(emu: i64) -> i32 {
    let px = emu_f(emu);
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
        let (color, x, y, w, h) = plain_rect(&doc.root, &[]).unwrap();
        assert_eq!(color, [0xEE, 0x44, 0x55]);
        assert_eq!((x, y, w, h), (192, 96, 96, 48));
    }

    #[test]
    fn grouped_rect_maps_emu_before_pixels() {
        let grp = r#"<p:grpSp xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="15000" cy="15000"/><a:chOff x="0" y="0"/><a:chExt cx="10000" cy="10000"/></a:xfrm></p:grpSpPr></p:grpSp>"#;
        let sp = r#"<p:sp xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><p:spPr><a:xfrm><a:off x="5000" y="0"/><a:ext cx="914400" cy="457200"/></a:xfrm><a:prstGeom prst="rect"/><a:solidFill><a:srgbClr val="EE4455"/></a:solidFill><a:ln><a:noFill/></a:ln></p:spPr></p:sp>"#;
        let group = crate::xml::parse(grp.as_bytes()).unwrap();
        let chain = [super::super::layout_check::group_xfrm(&group.root)];
        let shape = crate::xml::parse(sp.as_bytes()).unwrap();
        let (_, x, y, w, h) = plain_rect(&shape.root, &chain).unwrap();
        // 5000×1.5=7500 EMU，先换算再取整是 1 像素。先取整再乘会变成 2。
        assert_eq!((x, y, w, h), (1, 0, 144, 72));
    }

    #[test]
    fn a_group_off_the_quarter_turn_fails() {
        let xml = r#"<p:grpSp xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><p:grpSpPr><a:xfrm rot="100"><a:off x="0" y="0"/><a:ext cx="914400" cy="914400"/><a:chOff x="0" y="0"/><a:chExt cx="914400" cy="914400"/></a:xfrm></p:grpSpPr></p:grpSp>"#;
        let doc = crate::xml::parse(xml.as_bytes()).unwrap();
        assert!(group_turns(&doc.root));
    }

    #[test]
    fn a_quarter_turn_group_maps_the_solid_rects() {
        let grp = r#"<p:grpSp xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><p:grpSpPr><a:xfrm rot="5400000"><a:off x="762000" y="457200"/><a:ext cx="1524000" cy="609600"/><a:chOff x="762000" y="457200"/><a:chExt cx="1524000" cy="609600"/></a:xfrm></p:grpSpPr></p:grpSp>"#;
        let group = crate::xml::parse(grp.as_bytes()).unwrap();
        assert!(!group_turns(&group.root));
        let chain = [super::super::layout_check::group_xfrm(&group.root)];
        let wide = r#"<p:sp xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><p:spPr><a:xfrm><a:off x="762000" y="762000"/><a:ext cx="1524000" cy="304800"/></a:xfrm><a:prstGeom prst="rect"/><a:solidFill><a:srgbClr val="EE4455"/></a:solidFill><a:ln w="25400"><a:noFill/><a:prstDash val="solid"/></a:ln><a:effectLst/></p:spPr></p:sp>"#;
        let narrow = r#"<p:sp xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><p:spPr><a:xfrm><a:off x="1905000" y="457200"/><a:ext cx="304800" cy="457200"/></a:xfrm><a:prstGeom prst="rect"/><a:solidFill><a:srgbClr val="EE4455"/></a:solidFill><a:ln><a:noFill/></a:ln></p:spPr></p:sp>"#;
        let wide = crate::xml::parse(wide.as_bytes()).unwrap();
        let narrow = crate::xml::parse(narrow.as_bytes()).unwrap();
        let (_, x, y, w, h) = plain_rect(&wide.root, &chain).unwrap();
        assert_eq!((x, y, w, h), (128, 0, 32, 160));
        let (_, x, y, w, h) = plain_rect(&narrow.root, &chain).unwrap();
        assert_eq!((x, y, w, h), (144, 120, 48, 32));
    }

    #[test]
    fn a_child_quarter_turn_maps_before_the_group() {
        let grp = r#"<p:grpSp xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><p:grpSpPr><a:xfrm><a:off x="1371600" y="152400"/><a:ext cx="1752600" cy="1524000"/><a:chOff x="1371600" y="152400"/><a:chExt cx="1752600" cy="1524000"/></a:xfrm></p:grpSpPr></p:grpSp>"#;
        let group = crate::xml::parse(grp.as_bytes()).unwrap();
        let chain = [super::super::layout_check::group_xfrm(&group.root)];
        let turned = r#"<p:sp xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><p:spPr><a:xfrm rot="5400000"><a:off x="762000" y="762000"/><a:ext cx="1524000" cy="304800"/></a:xfrm><a:prstGeom prst="rect"/><a:solidFill><a:srgbClr val="EE4455"/></a:solidFill><a:ln w="25400"><a:noFill/><a:prstDash val="solid"/></a:ln><a:effectLst/></p:spPr></p:sp>"#;
        let plain = r#"<p:sp xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><p:spPr><a:xfrm><a:off x="2667000" y="762000"/><a:ext cx="457200" cy="304800"/></a:xfrm><a:prstGeom prst="rect"/><a:solidFill><a:srgbClr val="EE4455"/></a:solidFill><a:ln><a:noFill/></a:ln></p:spPr></p:sp>"#;
        let turned = crate::xml::parse(turned.as_bytes()).unwrap();
        let plain = crate::xml::parse(plain.as_bytes()).unwrap();
        let (_, x, y, w, h) = plain_rect(&turned.root, &chain).unwrap();
        assert_eq!((x, y, w, h), (144, 16, 32, 160));
        let (_, x, y, w, h) = plain_rect(&plain.root, &chain).unwrap();
        assert_eq!((x, y, w, h), (280, 80, 48, 32));
    }

    #[test]
    fn crop_percent_lands_on_whole_pixels_or_not_at_all() {
        assert_eq!(percent_px(25_000, 4), Some(1));
        assert_eq!(percent_px(33_333, 3), Some(1));
        assert_eq!(percent_px(0, 4), Some(0));
        assert_eq!(percent_px(12_500, 4), None);
        assert_eq!(percent_px(-25_000, 4), None);
    }

    #[test]
    fn one_to_one_crop_keeps_the_source_pixels() {
        let xml = r#"<p:pic xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><p:blipFill><a:blip r:embed="rId2"/><a:srcRect l="25000" t="25000" r="25000" b="25000"/><a:stretch><a:fillRect/></a:stretch></p:blipFill></p:pic>"#;
        let doc = crate::xml::parse(xml.as_bytes()).unwrap();
        assert_eq!(
            source_window(&doc.root, 4, 4, 2, 2).unwrap(),
            Some((1, 1, 2, 2))
        );
        assert!(source_window(&doc.root, 4, 4, 4, 4).is_err());
    }

    #[test]
    fn two_stop_gradient_follows_the_office_curve() {
        let mut rgb = vec![0u8; 720 * 3];
        fill_gradient(
            &mut rgb,
            1,
            720,
            0,
            0,
            1,
            720,
            [0, 0, 0],
            [255, 255, 255],
            RIGHT_ANGLE,
        );
        let at = |y: usize| rgb[y * 3];
        assert_eq!(at(180), 105);
        assert_eq!(at(360), 186);
        assert_eq!(at(540), 238);

        let mut navy = vec![0u8; 720 * 3];
        fill_gradient(
            &mut navy,
            1,
            720,
            0,
            0,
            1,
            720,
            [0x0B, 0x25, 0x45],
            [0x1B, 0x6C, 0xA8],
            RIGHT_ANGLE,
        );
        assert_eq!(&navy[360 * 3..360 * 3 + 3], &[21, 82, 130]);
    }

    #[test]
    fn round_rect_stroke_covers_only_the_outer_pixel() {
        let mut rgb = vec![0u8; 8 * 6 * 3];
        let fill = RgbaFill {
            color: [255, 255, 255],
            alpha: 255,
            on: true,
        };
        fill_round(&mut rgb, 8, 6, 1, 1, 6, 4, 7, 5, 0, 0, fill);
        stroke_round(&mut rgb, 8, 6, 1, 1, 6, 4, 0, 1, 7, 5, [1, 2, 3]);
        assert_eq!(&rgb[(8 + 1) * 3..][..3], &[1, 2, 3]);
        assert_eq!(&rgb[(3 * 8 + 3) * 3..][..3], &[255, 255, 255]);

        // 半径 3、线宽 1 时，实色直边从第 4 像素开始。拐角像素走圆弧。
        let mut curved = vec![255u8; 16 * 24 * 3];
        let white = RgbaFill {
            color: [255, 255, 255],
            alpha: 255,
            on: true,
        };
        let inset = stroke_inset(3.0, 1.0);
        assert_eq!(inset, 4);
        paint_round_corners(
            &mut curved,
            16,
            24,
            1,
            1,
            15,
            21,
            inset,
            3.0,
            1.0,
            white,
            [1, 2, 3],
        );
        fill_round(&mut curved, 16, 24, 1, 1, 14, 20, 15, 21, 3, inset, white);
        stroke_round(
            &mut curved,
            16,
            24,
            1,
            1,
            14,
            20,
            inset,
            1,
            15,
            21,
            [1, 2, 3],
        );
        let at = |buf: &[u8], x: usize, y: usize| {
            let i = (y * 16 + x) * 3;
            [buf[i], buf[i + 1], buf[i + 2]]
        };
        assert_eq!(at(&curved, 1, 1), [255, 255, 255]);
        assert_eq!(at(&curved, 1, 5), [1, 2, 3]);
        let arc = at(&curved, 4, 1);
        assert!(arc[0] < 128, "圆弧没有描边 {arc:?}");
    }

    #[test]
    fn drawingml_font_slot_splits_digits_from_han() {
        assert!(matches!(font_slot('1', "zh-CN"), FontSlot::Latin));
        assert!(matches!(font_slot(' ', "zh-CN"), FontSlot::Latin));
        assert!(matches!(font_slot('万', "zh-CN"), FontSlot::Ea));
        assert!(matches!(font_slot('\u{201C}', "zh-CN"), FontSlot::Ea));
        assert!(matches!(font_slot('\u{201C}', "en-US"), FontSlot::Latin));
        assert!(matches!(font_slot('ا', "ar-SA"), FontSlot::Cs));
    }

    #[test]
    fn stroked_ellipse_uses_the_rounded_rect_plus_one_pixel() {
        let oval = Oval {
            x: 104.987,
            y: 225.722,
            w: 73.491,
            h: 73.491,
            fill: RgbaFill {
                color: [46, 117, 182],
                alpha: 255,
                on: true,
            },
            stroke: Some([46, 117, 182]),
            stroke_f: 1.333,
        };
        assert_eq!(ellipse_device_box(&oval, true), Some((105, 226, 74, 74)));
        assert_eq!(ellipse_device_box(&oval, false), Some((105, 226, 73, 73)));
        let carried = Oval { x: 388.451, ..oval };
        assert_eq!(ellipse_device_box(&carried, true), Some((388, 226, 75, 74)));
    }

    #[test]
    fn table_header_uses_minor_font_and_bold_white() {
        let xml = r#"<a:p xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><a:r><a:t>季度</a:t></a:r></a:p>"#;
        let doc = crate::xml::parse(xml.as_bytes()).unwrap();
        let fonts = MinorFonts {
            latin: "Calibri".to_string(),
            ea: "微软雅黑".to_string(),
        };
        let line = cell_line(&doc.root, &fonts, true).unwrap();
        assert_eq!(line.latin, "Calibri");
        assert_eq!(line.ea, "微软雅黑");
        assert!(line.bold);
        assert_eq!(line.color, [255, 255, 255]);
        assert!((line.px - 24.0).abs() < 0.01, "{}", line.px);
        assert_eq!(line.align, "l");
        let body = cell_line(&doc.root, &fonts, false).unwrap();
        assert!(!body.bold);
        assert_eq!(body.color, [0, 0, 0]);
    }

    #[test]
    fn table_run_properties_override_the_header_style() {
        let xml = r#"<a:p xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><a:r><a:rPr sz="1400" b="0"><a:solidFill><a:srgbClr val="FF0000"/></a:solidFill><a:latin typeface="Arial"/><a:ea typeface="宋体"/></a:rPr><a:t>A</a:t></a:r></a:p>"#;
        let doc = crate::xml::parse(xml.as_bytes()).unwrap();
        let fonts = MinorFonts {
            latin: "Calibri".to_string(),
            ea: "微软雅黑".to_string(),
        };
        let line = cell_line(&doc.root, &fonts, true).unwrap();
        assert_eq!(line.latin, "Arial");
        assert_eq!(line.ea, "宋体");
        assert!(!line.bold);
        assert_eq!(line.color, [255, 0, 0]);
        assert!((line.px - 14.0 * 96.0 / 72.0).abs() < 0.01, "{}", line.px);
    }

    #[test]
    fn medium_style_header_is_only_the_known_guid() {
        let on = r#"<a:tbl xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><a:tblPr firstRow="1"><a:tableStyleId>{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}</a:tableStyleId></a:tblPr></a:tbl>"#;
        let off = r#"<a:tbl xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><a:tblPr firstRow="0"><a:tableStyleId>{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}</a:tableStyleId></a:tblPr></a:tbl>"#;
        let other = r#"<a:tbl xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><a:tblPr firstRow="1"><a:tableStyleId>{00000000-0000-0000-0000-000000000000}</a:tableStyleId></a:tblPr></a:tbl>"#;
        assert!(header_is_bold_light(
            &crate::xml::parse(on.as_bytes()).unwrap().root
        ));
        assert!(!header_is_bold_light(
            &crate::xml::parse(off.as_bytes()).unwrap().root
        ));
        assert!(!header_is_bold_light(
            &crate::xml::parse(other.as_bytes()).unwrap().root
        ));
    }

    #[test]
    fn translucent_ellipse_composites_over_the_page() {
        let mut rgb = vec![0u8; 80 * 80 * 3];
        for px in rgb.chunks_mut(3) {
            px.copy_from_slice(&[25, 100, 156]);
        }
        fill_ellipse(
            &mut rgb,
            80,
            80,
            &Oval {
                x: 10.0,
                y: 10.0,
                w: 40.0,
                h: 40.0,
                fill: RgbaFill {
                    color: [255, 255, 255],
                    alpha: 26,
                    on: true,
                },
                stroke: Some([46, 117, 182]),
                stroke_f: 1.333,
            },
        );
        let at = |x: usize, y: usize| {
            let i = (y * 80 + x) * 3;
            [rgb[i], rgb[i + 1], rgb[i + 2]]
        };
        let center = at(30, 30);
        assert_eq!(center, [48, 116, 166], "圆心应叠在页底上 {center:?}");
        assert_eq!(at(1, 1), [25, 100, 156]);
    }

    #[test]
    fn ellipse_edge_is_partial_not_a_step() {
        let signed = ellipse_signed(10, 15, 10.5, 10.5, 5.0, 5.0);
        assert!(signed.abs() < 1e-9, "{signed}");
        let edge = ellipse_cover(10, 15, 10.5, 10.5, 5.0, 5.0);
        assert!(edge > 0.4 && edge < 0.6, "{edge}");
        assert_eq!(ellipse_cover(10, 10, 10.5, 10.5, 5.0, 5.0), 1.0);
        assert_eq!(ellipse_cover(10, 20, 10.5, 10.5, 5.0, 5.0), 0.0);
    }
}
