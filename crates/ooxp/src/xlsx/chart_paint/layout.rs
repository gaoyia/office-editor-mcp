use crate::error::{Error, ErrorKind};
use crate::xml::Element;

use super::*;


/// 图表 PNG 的像素。`twoCell` 按格子累加，`oneCell`/`absolute` 用锚点上的 `xdr:ext`。
/// Excel 16.0 保存后 `graphicFrame/xfrm/ext` 常被写成 0×0，不能拿来当尺寸。缩放来自工作表。
pub(crate) fn bitmap_px(
    sheet: &Element,
    anchor: &Element,
    kind: super::super::picture::AnchorKind,
    geometry: &super::super::geometry::GeometryContext,
) -> Result<(u32, u32), Error> {
    let zoom = sheet_zoom(sheet);
    let (w, h) = match kind {
        super::super::picture::AnchorKind::TwoCell => two_cell_px(sheet, anchor, zoom, geometry)?,
        _ => ext_px(anchor, zoom, geometry.dpi())?,
    };
    Ok((fit_px(w)?, fit_px(h)?))
}

pub(super) fn fit_px(px: i64) -> Result<u32, Error> {
    if !(1..=i64::from(MAX_PX)).contains(&px) {
        return Err(Error::with_path(ErrorKind::ChartImageUnsupported, "id"));
    }
    Ok(px as u32)
}

pub(super) fn sheet_zoom(sheet: &Element) -> u32 {
    sheet
        .child_by_local("sheetViews")
        .and_then(|views| views.child_by_local("sheetView"))
        .and_then(|view| view.attr("zoomScale"))
        .and_then(|s| s.parse::<u32>().ok())
        .filter(|n| (10..=400).contains(n))
        .unwrap_or(100)
}

pub(super) fn zoomed(px: i64, zoom: u32) -> i64 {
    (px as f64 * f64::from(zoom) / 100.0).round() as i64
}

pub(super) fn emu_px(emu: i64, dpi: RenderDpi) -> i64 {
    dpi.emu_to_px_round(emu)
}

pub(super) fn pt_to_px(pt: f64, dpi: RenderDpi) -> i64 {
    let emu = (pt * 12_700.0).round() as i64;
    dpi.emu_to_px_round(emu)
}

pub(super) fn row_px(sheet: &Element, row: u32, dpi: RenderDpi) -> i64 {
    if let Some(sheet_data) = sheet.child_by_local("sheetData") {
        for el in sheet_data.children_by_local("row") {
            let Some(r) = el.attr("r").and_then(|s| s.parse::<u32>().ok()) else {
                continue;
            };
            if r != row {
                continue;
            }
            if let Some(ht) = el.attr("ht").and_then(|s| s.parse::<f64>().ok()) {
                return pt_to_px(ht, dpi).max(1);
            }
        }
    }
    let def = sheet
        .child_by_local("sheetFormatPr")
        .and_then(|el| el.attr_local("defaultRowHeight"))
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(14.25);
    pt_to_px(def, dpi).max(1)
}

pub(super) fn marker_x(
    sheet: &Element,
    col: u32,
    off: i64,
    zoom: u32,
    geometry: &super::super::geometry::GeometryContext,
) -> Result<i64, Error> {
    let mut px = 0i64;
    for c in 1..col {
        px += zoomed(
            i64::from(
                geometry
                    .column_px(sheet, c)
                    .map_err(|_| Error::with_path(ErrorKind::ChartImageUnsupported, "id"))?,
            ),
            zoom,
        );
    }
    Ok(px + zoomed(emu_px(off, geometry.dpi()), zoom))
}

pub(super) fn marker_y(sheet: &Element, row: u32, off: i64, zoom: u32, dpi: RenderDpi) -> i64 {
    let mut px = 0i64;
    for r in 1..row {
        px += zoomed(row_px(sheet, r, dpi), zoom);
    }
    px + zoomed(emu_px(off, dpi), zoom)
}

pub(super) fn two_cell_px(
    sheet: &Element,
    anchor: &Element,
    zoom: u32,
    geometry: &super::super::geometry::GeometryContext,
) -> Result<(i64, i64), Error> {
    let (from, from_x, from_y) = super::super::picture::read_marker(anchor.child_by_local("from"))?;
    let (to, to_x, to_y) = super::super::picture::read_marker(anchor.child_by_local("to"))?;
    let (Some(from), Some(to)) = (from, to) else {
        return ext_px(anchor, zoom, geometry.dpi());
    };
    let w = marker_x(sheet, to.col, to_x, zoom, geometry)?
        - marker_x(sheet, from.col, from_x, zoom, geometry)?;
    let h = marker_y(sheet, to.row, to_y, zoom, geometry.dpi())
        - marker_y(sheet, from.row, from_y, zoom, geometry.dpi());
    Ok((w, h))
}

pub(super) fn ext_px(anchor: &Element, zoom: u32, dpi: RenderDpi) -> Result<(i64, i64), Error> {
    // oneCell/absolute 的合同尺寸在锚点 `xdr:ext`。内层 graphicFrame xfrm 只是占位，
    // Office 保存后经常是 0×0；若优先读它，fit_px 会把整张已会画的图误判为 NI。
    let frame_ext = anchor
        .child_by_local("graphicFrame")
        .and_then(|frame| frame.child_by_local("xfrm"))
        .and_then(|xfrm| xfrm.child_by_local("ext"));
    let ext = first_positive_ext([anchor.child_by_local("ext"), frame_ext])
        .ok_or_else(|| Error::with_path(ErrorKind::ChartImageUnsupported, "id"))?;
    let cx: i64 = ext
        .attr_local("cx")
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| Error::with_path(ErrorKind::ChartImageUnsupported, "id"))?;
    let cy: i64 = ext
        .attr_local("cy")
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| Error::with_path(ErrorKind::ChartImageUnsupported, "id"))?;
    Ok((zoomed(emu_px(cx, dpi), zoom), zoomed(emu_px(cy, dpi), zoom)))
}

pub(super) fn first_positive_ext(cands: [Option<&Element>; 2]) -> Option<&Element> {
    cands.into_iter().flatten().find(|ext| {
        matches!(
            (
                ext.attr_local("cx").and_then(|s| s.parse::<i64>().ok()),
                ext.attr_local("cy").and_then(|s| s.parse::<i64>().ok()),
            ),
            (Some(cx), Some(cy)) if cx > 0 && cy > 0
        )
    })
}
