use crate::error::{Error, ErrorKind};
use crate::xml::Element;

use super::*;


#[cfg(test)]
pub(super) fn read_frame(root: &Element) -> Result<FrameStyle, Error> {
    read_frame_with(root, &theme_scheme(None, FALLBACK_ACCENTS))
}

pub(super) fn read_frame_with(root: &Element, scheme: &ThemeScheme) -> Result<FrameStyle, Error> {
    let plot = root
        .child_by_local("chart")
        .and_then(|c| c.child_by_local("plotArea"));
    let rounded = flag(root, "roundedCorners").unwrap_or(true);
    let border = root
        .child_by_local("spPr")
        .and_then(|p| p.child_by_local("ln"))
        .is_none();
    // Visible chart/plot lines, rounded no-line, and outerShdw are parsed
    // here so Office-exportable frames produce an image. The locked default
    // 1px gray border (no `a:ln`) and rectangular no-line path are unchanged.
    Ok(FrameStyle {
        rounded,
        border,
        fill: read_frame_fill(root, scheme)?,
        plot_fill: plot
            .map(|node| read_frame_fill(node, scheme))
            .transpose()?
            .unwrap_or(FrameFill::Default),
        chart_line: read_visible_frame_line(root, scheme)?,
        plot_line: plot
            .map(|node| read_visible_frame_line(node, scheme))
            .transpose()?
            .flatten(),
        chart_shadow: read_outer_shadow(root, scheme)?,
    })
}

pub(super) fn read_frame_fill(node: &Element, scheme: &ThemeScheme) -> Result<FrameFill, Error> {
    let unsupported = || Error::with_path(ErrorKind::ChartImageUnsupported, "id");
    let Some(props) = node.child_by_local("spPr") else {
        return Ok(FrameFill::Default);
    };
    let mut fill = FrameFill::Default;
    let mut saw_fill = false;
    let mut saw_hidden_ext = false;
    let mut saw_line = false;
    let mut saw_hidden_line = false;
    for prop in props.element_children() {
        match prop.local_name() {
            "noFill" => {
                if saw_fill {
                    return Err(unsupported());
                }
                saw_fill = true;
                fill = FrameFill::NoFill;
            }
            "gradFill" => {
                if saw_fill {
                    return Err(unsupported());
                }
                saw_fill = true;
                fill = read_linear_frame_grad(prop, scheme)?;
            }
            "pattFill" => {
                if saw_fill {
                    return Err(unsupported());
                }
                saw_fill = true;
                fill = FrameFill::Pattern(read_pattern_fill(prop, scheme)?);
            }
            "blipFill" => {
                if saw_fill {
                    return Err(unsupported());
                }
                saw_fill = true;
                fill = FrameFill::Blip(read_blip_fill(prop)?);
            }
            "solidFill" => {
                if saw_fill {
                    return Err(unsupported());
                }
                saw_fill = true;
                let mut colors = prop.element_children();
                let color = colors.next().ok_or_else(unsupported)?;
                if colors.next().is_some() {
                    return Err(unsupported());
                }
                fill = resolve_frame_solid(color, scheme)?;
            }
            "ln" => saw_line = true,
            "effectLst" => (),
            // Excel 16 writes a:noFill plus an Office 2010 hiddenFill ext that
            // only remembers the previous fill; it is not a visible fill.
            "extLst" if is_office_hidden_fill_ext(prop) => saw_hidden_ext = true,
            "extLst" if is_office_hidden_line_ext(prop) => saw_hidden_line = true,
            _ => return Err(unsupported()),
        }
    }
    if (saw_hidden_ext && fill != FrameFill::NoFill) || (saw_hidden_line && !saw_line) {
        return Err(unsupported());
    }
    Ok(fill)
}

/// Excel's invisible line retains the old line in a14:hiddenLine. That
/// extension is editing memory, not a visible stroke or effect.
pub(super) fn is_office_hidden_line_ext(ext_lst: &Element) -> bool {
    let mut exts = ext_lst.element_children();
    let Some(ext) = exts.next() else {
        return false;
    };
    if exts.next().is_some()
        || ext.local_name() != "ext"
        || ext.attr_local("uri") != Some("{91240B29-F687-4F45-9708-019B960494DF}")
    {
        return false;
    }
    let mut kids = ext.element_children();
    matches!(kids.next(), Some(node) if node.local_name() == "hiddenLine") && kids.next().is_none()
}

pub(super) fn is_invisible_frame_line(line: &Element) -> bool {
    for (name, value) in &line.attrs {
        match name.as_str() {
            "w" if value.parse::<u32>().is_ok() => (),
            "cap" if value == "flat" => (),
            "cmpd" if value == "sng" => (),
            "algn" if value == "ctr" => (),
            _ => return false,
        }
    }
    let mut no_fill = false;
    let mut dash = false;
    let mut join = false;
    for child in line.element_children() {
        if child.element_children().next().is_some() {
            return false;
        }
        match child.local_name() {
            "noFill" if !no_fill && child.attrs.is_empty() => no_fill = true,
            "prstDash" if !dash && child.attr_local("val") == Some("solid") => dash = true,
            "round" if !join && child.attrs.is_empty() => join = true,
            _ => return false,
        }
    }
    no_fill
}

/// Close-enough visible `a:ln` on a chart or plot frame. `a:noFill` stays
/// invisible (the locked rectangular no-line path). Omitted `w` uses 1pt
/// (12700 EMU). Omitted fill uses opaque black so a bare `<a:ln w="..."/>`
/// still exports. Unresolved or non-solid fills also fall back to black.
/// This is not a unique official-matrix lock.
pub(super) fn read_visible_frame_line(
    node: &Element,
    scheme: &ThemeScheme,
) -> Result<Option<FrameLine>, Error> {
    let Some(line) = node
        .child_by_local("spPr")
        .and_then(|props| props.child_by_local("ln"))
    else {
        return Ok(None);
    };
    if is_invisible_frame_line(line)
        || line
            .element_children()
            .any(|child| child.local_name() == "noFill")
    {
        return Ok(None);
    }
    let width_emu = line
        .attr_local("w")
        .and_then(|value| value.parse::<f64>().ok())
        .filter(|width| width.is_finite() && *width > 0.0)
        .unwrap_or(12_700.0);
    let mut color = [0_u8, 0, 0];
    let mut alpha = 255_u8;
    for child in line.element_children() {
        if child.local_name() == "solidFill" {
            let (rgb, a) = frame_line_color(child, scheme);
            color = rgb;
            alpha = a;
        }
    }
    Ok(Some(FrameLine {
        color,
        alpha,
        width_emu,
    }))
}

fn frame_line_color(fill: &Element, scheme: &ThemeScheme) -> ([u8; 3], u8) {
    let Some(color) = fill.element_children().next() else {
        return ([0, 0, 0], 255);
    };
    match resolve_frame_solid(color, scheme) {
        Ok(FrameFill::Solid(rgb)) => (rgb, 255),
        Ok(FrameFill::Translucent(rgb, a)) => (rgb, ooxml_alpha_to_u8(a)),
        Ok(FrameFill::AlphaProduct(rgb, product)) => (rgb, alpha_product_to_u8(product)),
        _ => ([0, 0, 0], 255),
    }
}

fn parse_emu_attr(node: &Element, name: &str) -> f64 {
    node.attr_local(name)
        .and_then(|value| value.parse::<f64>().ok())
        .filter(|value| value.is_finite() && *value >= 0.0)
        .unwrap_or(0.0)
}

/// Close-enough chart-area `a:outerShdw`. Missing color is opaque black;
/// missing blur/dist/dir are zero. Other `effectLst` children are ignored
/// so the chart still exports. Not a unique official lock.
pub(super) fn read_outer_shadow(
    node: &Element,
    scheme: &ThemeScheme,
) -> Result<Option<OuterShadow>, Error> {
    let Some(shadow) = node
        .child_by_local("spPr")
        .and_then(|props| props.child_by_local("effectLst"))
        .and_then(|list| {
            list.element_children()
                .find(|child| child.local_name() == "outerShdw")
        })
    else {
        return Ok(None);
    };
    let mut color = [0_u8, 0, 0];
    let mut alpha = 255_u8;
    for child in shadow.element_children() {
        match resolve_frame_solid(child, scheme) {
            Ok(FrameFill::Solid(rgb)) => {
                color = rgb;
                alpha = 255;
            }
            Ok(FrameFill::Translucent(rgb, a)) => {
                color = rgb;
                alpha = ooxml_alpha_to_u8(a);
            }
            Ok(FrameFill::AlphaProduct(rgb, product)) => {
                color = rgb;
                alpha = alpha_product_to_u8(product);
            }
            _ => {}
        }
    }
    let dir = shadow
        .attr_local("dir")
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(0);
    Ok(Some(OuterShadow {
        color,
        alpha,
        blur_emu: parse_emu_attr(shadow, "blurRad"),
        dist_emu: parse_emu_attr(shadow, "dist"),
        dir,
    }))
}

pub(super) fn is_office_hidden_fill_ext(ext_lst: &Element) -> bool {
    let mut exts = ext_lst.element_children();
    let Some(ext) = exts.next() else {
        return false;
    };
    if exts.next().is_some() || ext.local_name() != "ext" {
        return false;
    }
    if ext.attr_local("uri") != Some("{909E8E84-426E-40DD-AFC4-6F175D3DCCD1}") {
        return false;
    }
    let mut kids = ext.element_children();
    let Some(hidden) = kids.next() else {
        return false;
    };
    if kids.next().is_some() || hidden.local_name() != "hiddenFill" {
        return false;
    }
    let mut fills = hidden.element_children();
    let Some(remembered) = fills.next() else {
        return false;
    };
    fills.next().is_none() && matches!(remembered.local_name(), "solidFill" | "noFill")
}

/// DrawingML `a:alpha` is opacity in thousandths of a percent, 100000 = opaque.
pub(super) fn parse_ooxml_alpha(val: Option<&str>) -> Option<u32> {
    let raw = val?;
    if raw.is_empty() || !raw.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let alpha: u32 = raw.parse().ok()?;
    (alpha <= 100_000).then_some(alpha)
}

pub(super) fn ooxml_alpha_to_u8(alpha: u32) -> u8 {
    ((u64::from(alpha) * 255 + 50_000) / 100_000) as u8
}

/// Pattern, blip, and gradient frame fills. Locked 0/90 opaque sRGB tables
/// and `ltHorz` / stretch-blip foundations are unchanged. Other Office-exportable
/// variants use close-enough rules so they produce an image instead of rejecting.

pub(super) fn read_pattern_fill(
    patt: &Element,
    scheme: &ThemeScheme,
) -> Result<pattern::PatternFill, Error> {
    let unsupported = || Error::with_path(ErrorKind::ChartImageUnsupported, "id");
    let prst = pattern::PatternPreset::parse(patt.attr_local("prst").ok_or_else(unsupported)?)
        .ok_or_else(unsupported)?;
    let mut fg = None;
    let mut bg = None;
    for child in patt.element_children() {
        match child.local_name() {
            "fgClr" if fg.is_none() => fg = Some(read_pattern_color(child, scheme)?),
            "bgClr" if bg.is_none() => bg = Some(read_pattern_color(child, scheme)?),
            _ => return Err(unsupported()),
        }
    }
    Ok(pattern::PatternFill {
        prst,
        fg: fg.ok_or_else(unsupported)?,
        bg: bg.ok_or_else(unsupported)?,
    })
}

fn read_pattern_color(slot: &Element, scheme: &ThemeScheme) -> Result<[u8; 3], Error> {
    let unsupported = || Error::with_path(ErrorKind::ChartImageUnsupported, "id");
    let mut colors = slot.element_children();
    let color = colors.next().ok_or_else(unsupported)?;
    if colors.next().is_some() {
        return Err(unsupported());
    }
    match resolve_frame_solid(color, scheme) {
        Ok(FrameFill::Solid(rgb)) => Ok(rgb),
        Ok(FrameFill::Translucent(rgb, _)) | Ok(FrameFill::AlphaProduct(rgb, _)) => Ok(rgb),
        _ => Ok(resolve_stop_color(color, scheme).0),
    }
}

/// Picture fills. Stretch with empty/zero srcRect/fillRect and a no-op effect
/// list keeps the locked foundation. Tile, crop, non-zero fillRect, a non-zero
/// `dpi`, and remaining blip effects are stored and painted close-enough so
/// the chart exports when media is present.
pub(super) fn read_blip_fill(fill: &Element) -> Result<pattern::BlipFill, Error> {
    let unsupported = || Error::with_path(ErrorKind::ChartImageUnsupported, "id");
    let mut dpi = 0u32;
    for (name, value) in &fill.attrs {
        match name.as_str() {
            "rotWithShape" => {}
            "dpi" => {
                dpi = value.parse().unwrap_or(0);
            }
            _ => {}
        }
    }
    let mut embed = None;
    let mut src_rect = None;
    let mut fill_rect = None;
    let mut tile = None;
    let mut saw_stretch = false;
    for child in fill.element_children() {
        match child.local_name() {
            "blip" if embed.is_none() => {
                let id = child
                    .attr_local("embed")
                    .filter(|s| !s.is_empty())
                    .ok_or_else(unsupported)?;
                embed = Some(id.to_string());
            }
            "srcRect" if src_rect.is_none() => {
                src_rect = Some(read_percent_rect(child));
            }
            "stretch" if !saw_stretch && tile.is_none() => {
                saw_stretch = true;
                if let Some(rect) = child.element_children().find(|el| el.local_name() == "fillRect")
                {
                    fill_rect = Some(read_percent_rect(rect));
                }
            }
            "tile" if tile.is_none() && !saw_stretch => {
                tile = Some(read_blip_tile(child));
            }
            _ => {}
        }
    }
    Ok(pattern::BlipFill {
        embed: embed.ok_or_else(unsupported)?,
        src_rect: src_rect.filter(|r| *r != [0, 0, 0, 0]),
        fill_rect: fill_rect.filter(|r| *r != [0, 0, 0, 0]),
        tile,
        dpi,
    })
}

fn read_percent_rect(node: &Element) -> [i64; 4] {
    let one = |name: &str| {
        node.attr_local(name)
            .or_else(|| node.attr(name))
            .and_then(|raw| raw.parse().ok())
            .unwrap_or(0)
    };
    [one("l"), one("t"), one("r"), one("b")]
}

fn read_blip_tile(node: &Element) -> pattern::BlipTile {
    let num = |name: &str, default: i64| {
        node.attr_local(name)
            .or_else(|| node.attr(name))
            .and_then(|raw| raw.parse().ok())
            .unwrap_or(default)
    };
    pattern::BlipTile {
        tx: num("tx", 0),
        ty: num("ty", 0),
        sx: num("sx", 100_000),
        sy: num("sy", 100_000),
        flip: pattern::BlipFlip::parse(node.attr_local("flip").or_else(|| node.attr("flip"))),
        algn: pattern::BlipAlign::parse(node.attr_local("algn").or_else(|| node.attr("algn"))),
    }
}

pub(super) fn read_linear_frame_grad(
    grad: &Element,
    scheme: &ThemeScheme,
) -> Result<FrameFill, Error> {
    let unsupported = || Error::with_path(ErrorKind::ChartImageUnsupported, "id");
    let mut gs_lst = None;
    let mut lin = None;
    let mut path = None;
    for child in grad.element_children() {
        match child.local_name() {
            "gsLst" if gs_lst.is_none() => gs_lst = Some(child),
            "lin" if lin.is_none() => lin = Some(child),
            "path" if path.is_none() => {
                path = Some(match child.attr_local("path").unwrap_or("rect") {
                    "circle" => PathShade::Circle,
                    "shape" => PathShade::Shape,
                    _ => PathShade::Rect,
                });
            }
            "tileRect" => {}
            _ => return Err(unsupported()),
        }
    }
    let gs_lst = gs_lst.ok_or_else(unsupported)?;
    let mut ang = LINEAR_ANG_HORIZONTAL;
    if let Some(lin) = lin {
        if let Some(raw) = lin.attr_local("ang") {
            ang = raw.parse().map_err(|_| unsupported())?;
        }
    }
    let mut raw_stops = Vec::new();
    let mut any_alpha = false;
    let mut any_non_srgb = false;
    for gs in gs_lst.element_children() {
        if gs.local_name() != "gs" {
            return Err(unsupported());
        }
        let pos: u32 = gs
            .attr_local("pos")
            .ok_or_else(unsupported)?
            .parse()
            .map_err(|_| unsupported())?;
        let mut colors = gs.element_children();
        let color = colors.next().ok_or_else(unsupported)?;
        if colors.next().is_some() {
            return Err(unsupported());
        }
        if color.local_name() != "srgbClr" || color.element_children().next().is_some() {
            any_non_srgb = true;
        }
        let (rgb, alpha) = resolve_stop_color(color, scheme);
        if alpha != 255 {
            any_alpha = true;
        }
        raw_stops.push((pos, rgb, alpha));
    }
    let use_close = any_alpha
        || any_non_srgb
        || raw_stops.len() < 2
        || raw_stops.first().is_some_and(|s| s.0 != 0)
        || raw_stops.last().is_some_and(|s| s.0 != 100_000)
        || {
            let mut ordered = raw_stops.clone();
            ordered.sort_by_key(|s| s.0);
            ordered.windows(2).any(|pair| pair[0].0 == pair[1].0)
        };
    let stops = if use_close {
        gradient::StopTable::close_enough(raw_stops).ok_or_else(unsupported)?
    } else {
        gradient::StopTable::new(raw_stops.into_iter().map(|(p, c, _)| (p, c)).collect())
            .ok_or_else(unsupported)?
    };
    Ok(FrameFill::Linear(LinearFrameGrad {
        stops,
        ang,
        path,
    }))
}

pub(super) fn read_chart(
    root: &Element,
    accents: [[u8; 3]; 6],
    cells: Option<&super::super::sparkline::CellCache>,
    sheet: &str,
    dpi: RenderDpi,
) -> Result<ColumnChart, Error> {
    let unsupported = || Error::with_path(ErrorKind::ChartImageUnsupported, "id");
    let chart = root.child_by_local("chart").ok_or_else(unsupported)?;
    let plot = chart.child_by_local("plotArea").ok_or_else(unsupported)?;
    let wrappers: Vec<&Element> = plot
        .element_children()
        .filter(|child| is_chart_wrapper(child.local_name()))
        .collect();
    if wrappers.is_empty() || wrappers.len() > 2 {
        return Err(unsupported());
    }
    let wrapper = wrappers[0];
    let locked_secondary = if wrappers.len() == 2 {
        Some(axis::read_locked_secondary_axes(plot)?)
    } else {
        None
    };
    let mut area_missing_as_zero = false;
    let mut stacking = Stacking::Clustered;
    let kind = match wrapper.local_name() {
        "barChart" => {
            let dir = wrapper
                .child_by_local("barDir")
                .and_then(|el| el.attr_local("val"))
                .unwrap_or("bar");
            let grouping = wrapper
                .child_by_local("grouping")
                .and_then(|el| el.attr_local("val"))
                .unwrap_or("clustered");
            stacking = match grouping {
                "clustered" => Stacking::Clustered,
                "stacked" => Stacking::Stacked,
                "percentStacked" => Stacking::PercentStacked,
                _ => return Err(unsupported()),
            };
            if dir == "bar" {
                let blanks = chart
                    .child_by_local("dispBlanksAs")
                    .and_then(|el| el.attr_local("val"))
                    .unwrap_or("gap");
                if blanks != "gap" {
                    return Err(unsupported());
                }
                if flag(wrapper, "varyColors").unwrap_or(false)
                    && wrapper
                        .element_children()
                        .filter(|el| el.local_name() == "ser")
                        .count()
                        != 1
                {
                    return Err(unsupported());
                }
            }
            match dir {
                "col" => PlotKind::ClusteredColumn,
                "bar" => PlotKind::ClusteredBar,
                _ => return Err(unsupported()),
            }
        }
        "lineChart" => {
            let grouping = wrapper
                .child_by_local("grouping")
                .and_then(|el| el.attr_local("val"))
                .unwrap_or("standard");
            stacking = match grouping {
                "standard" => Stacking::Clustered,
                "stacked" => Stacking::Stacked,
                "percentStacked" => Stacking::PercentStacked,
                _ => return Err(unsupported()),
            };
            if flag(wrapper, "varyColors").unwrap_or(false) {
                return Err(unsupported());
            }
            if flag(wrapper, "smooth").unwrap_or(false) {
                return Err(unsupported());
            }
            for child in wrapper.element_children() {
                if matches!(
                    child.local_name(),
                    "dropLines" | "hiLowLines" | "upDownBars"
                ) {
                    return Err(unsupported());
                }
            }
            let blanks = chart
                .child_by_local("dispBlanksAs")
                .and_then(|el| el.attr_local("val"))
                .unwrap_or("gap");
            if !matches!(blanks, "gap" | "zero" | "span") {
                return Err(unsupported());
            }
            PlotKind::StandardLine
        }
        "scatterChart" => {
            if flag(wrapper, "varyColors").unwrap_or(false) {
                return Err(unsupported());
            }
            data_labels_hidden(wrapper.child_by_local("dLbls"))?;
            let blanks = chart
                .child_by_local("dispBlanksAs")
                .and_then(|el| el.attr_local("val"))
                .unwrap_or("gap");
            if blanks != "gap" {
                return Err(unsupported());
            }
            for child in wrapper.element_children() {
                if matches!(
                    child.local_name(),
                    "dropLines" | "hiLowLines" | "upDownBars"
                ) {
                    return Err(unsupported());
                }
            }
            let style = wrapper
                .child_by_local("scatterStyle")
                .and_then(|el| el.attr_local("val"))
                .unwrap_or("lineMarker");
            // Official Excel 16.0 maps scatterStyle=marker (no series line hide,
            // smooth=0) to xlXYScatterLines (74) and draws connecting strokes.
            // Markers-only remains available via an explicit series a:ln/a:noFill.
            let connect = match style {
                "marker" | "lineMarker" => true,
                _ => return Err(unsupported()),
            };
            PlotKind::Scatter { connect }
        }
        "areaChart" => {
            let grouping = wrapper
                .child_by_local("grouping")
                .and_then(|el| el.attr_local("val"))
                .unwrap_or("standard");
            stacking = match grouping {
                "standard" => Stacking::Clustered,
                "stacked" => Stacking::Stacked,
                "percentStacked" => Stacking::PercentStacked,
                _ => return Err(unsupported()),
            };
            if flag(wrapper, "varyColors").unwrap_or(false) {
                return Err(unsupported());
            }
            for child in wrapper.element_children() {
                if matches!(child.local_name(), "dropLines") {
                    return Err(unsupported());
                }
            }
            data_labels_hidden(wrapper.child_by_local("dLbls"))?;
            let blanks_el = chart.child_by_local("dispBlanksAs");
            let blanks = blanks_el
                .and_then(|el| el.attr_local("val"))
                .unwrap_or("gap");
            // Omitted dispBlanksAs is official area default (missing as 0).
            // Excel 16 public saves of that default write explicit val=zero,
            // which fills the same V-to-baseline. Explicit gap stays a hole.
            // Official Excel 16 area span paints the same V-to-baseline as
            // zero, including #N/A; it is not a 4-to-9 chord. Other errors
            // and non-area types remain NI.
            area_missing_as_zero = blanks_el.is_none() || blanks == "zero" || blanks == "span";
            if blanks != "gap" && !area_missing_as_zero {
                return Err(unsupported());
            }
            PlotKind::StandardArea
        }
        "pieChart" => {
            if wrapper
                .element_children()
                .filter(|el| el.local_name() == "ser")
                .count()
                != 1
            {
                return Err(unsupported());
            }
            data_labels_hidden(wrapper.child_by_local("dLbls"))?;
            let blanks = chart
                .child_by_local("dispBlanksAs")
                .and_then(|el| el.attr_local("val"))
                .unwrap_or("gap");
            // Official pies often write zero (F09 gold). Unknown values stay NI.
            // zero/span with a blank point are rejected after the category
            // domain is applied, because pie blank geometry is not locked.
            if !matches!(blanks, "gap" | "zero" | "span") {
                return Err(unsupported());
            }
            if plot
                .element_children()
                .any(|el| matches!(el.local_name(), "valAx" | "catAx" | "dateAx" | "serAx"))
            {
                return Err(unsupported());
            }
            PlotKind::Pie
        }
        _ => return Err(unsupported()),
    };
    if let Some(sec) = &locked_secondary {
        if !matches!(kind, PlotKind::ClusteredColumn | PlotKind::StandardLine) {
            return Err(unsupported());
        }
        if wrappers[1].local_name() != wrapper.local_name() {
            return Err(unsupported());
        }
        if kind == PlotKind::ClusteredColumn {
            let dir = wrappers[1]
                .child_by_local("barDir")
                .and_then(|el| el.attr_local("val"))
                .unwrap_or("bar");
            let grouping = wrappers[1]
                .child_by_local("grouping")
                .and_then(|el| el.attr_local("val"))
                .unwrap_or("clustered");
            if dir != "col" || grouping != "clustered" {
                return Err(unsupported());
            }
        }
        if kind == PlotKind::StandardLine {
            let grouping = wrappers[1]
                .child_by_local("grouping")
                .and_then(|el| el.attr_local("val"))
                .unwrap_or("standard");
            if grouping != "standard"
                || flag(wrappers[1], "varyColors").unwrap_or(false)
                || flag(wrappers[1], "smooth").unwrap_or(false)
            {
                return Err(unsupported());
            }
        }
        let gap1 = number_val(wrappers[1], "gapWidth").unwrap_or(150.0);
        let over1 = number_val(wrappers[1], "overlap").unwrap_or(0.0);
        if (gap1 - number_val(wrapper, "gapWidth").unwrap_or(150.0)).abs() > 1e-9
            || (over1 - number_val(wrapper, "overlap").unwrap_or(0.0)).abs() > 1e-9
        {
            return Err(unsupported());
        }
        for wrap in &wrappers {
            let nser = wrap
                .element_children()
                .filter(|el| el.local_name() == "ser")
                .count();
            if nser != 1 {
                return Err(unsupported());
            }
            let ids: Vec<_> = wrap
                .element_children()
                .filter(|el| el.local_name() == "axId")
                .filter_map(|el| el.attr_local("val"))
                .collect();
            let uses_primary = ids.iter().any(|id| *id == sec.primary_val_id.as_str());
            let uses_secondary = ids.iter().any(|id| *id == sec.secondary_val_id.as_str());
            if uses_primary == uses_secondary {
                return Err(unsupported());
            }
        }
        let _ = sec;
    }
    let vary = if kind == PlotKind::Pie {
        flag(wrapper, "varyColors").unwrap_or(true)
    } else {
        flag(wrapper, "varyColors").unwrap_or(false)
    };
    let gap_width = number_val(wrapper, "gapWidth").unwrap_or(150.0);
    let overlap = number_val(wrapper, "overlap").unwrap_or(0.0);
    // Official Excel 16 stacked / percentStacked barChart writes overlap=100.
    // lineChart / areaChart have no overlap element.
    if stacking != Stacking::Clustered
        && matches!(kind, PlotKind::ClusteredColumn | PlotKind::ClusteredBar)
    {
        match number_val(wrapper, "overlap") {
            Some(v) if (v - 100.0).abs() <= 1e-9 => {}
            _ => return Err(unsupported()),
        }
    }
    let mut series = Vec::new();
    let ser_wrappers: Vec<&Element> = if locked_secondary.is_some() {
        wrappers.clone()
    } else {
        vec![wrapper]
    };
    for wrap in &ser_wrappers {
    let on_secondary = locked_secondary.as_ref().is_some_and(|sec| {
        wrap
            .element_children()
            .filter(|el| el.local_name() == "axId")
            .filter_map(|el| el.attr_local("val"))
            .any(|id| id == sec.secondary_val_id)
    });
    for ser in wrap
        .element_children()
        .filter(|el| el.local_name() == "ser")
    {
        let order = number_val(ser, "order").unwrap_or(series.len() as f64) as usize;
        let (values, x_values) = if matches!(kind, PlotKind::Scatter { .. }) {
            let ys = series_numbers(ser.child_by_local("yVal"), sheet, cells);
            let xs = series_numbers(ser.child_by_local("xVal"), sheet, cells);
            if xs.is_empty() && ys.is_empty() {
                continue;
            }
            if !xs.iter().any(|v| v.is_some_and(|n| n.is_finite()))
                || !ys.iter().any(|v| v.is_some_and(|n| n.is_finite()))
            {
                return Err(unsupported());
            }
            (ys, xs)
        } else {
            let values = series_numbers(ser.child_by_local("val"), sheet, cells);
            // Official all-empty pies omit every val cache pt while keeping
            // category pts. Do not drop that series here; the category domain
            // is applied below. Do not size the domain from untrusted ptCount
            // alone.
            if values.is_empty() && kind != PlotKind::Pie {
                continue;
            }
            (values, Vec::new())
        };
        let value_label = if matches!(
            kind,
            PlotKind::ClusteredColumn | PlotKind::ClusteredBar | PlotKind::StandardLine
        ) {
            read_value_label(wrap, ser, kind)?
        } else {
            data_labels_hidden(ser.child_by_local("dLbls"))?;
            None
        };
        if on_secondary && value_label.is_some() {
            return Err(unsupported());
        }
        if value_label == Some(ValueLabel::InsideEnd) && values.iter().flatten().any(|v| *v == 0.0)
        {
            return Err(unsupported());
        }
        if value_label.is_some()
            && values
                .iter()
                .flatten()
                .any(|v| value_label_text(*v).is_none())
        {
            return Err(unsupported());
        }
        let (error_points, unavailable_points) = if kind == PlotKind::StandardLine {
            series_error_points(ser.child_by_local("val"), sheet, cells, values.len())
        } else {
            (Vec::new(), Vec::new())
        };
        // Explicit area zero is locked only for true blanks / omitted cache
        // points. Official area span also locks #N/A as a baseline V, but
        // other errors stay NI under both modes.
        if kind == PlotKind::StandardArea
            && area_missing_as_zero
            && chart.child_by_local("dispBlanksAs").is_some()
        {
            let (errors, unavailable) =
                series_error_points(ser.child_by_local("val"), sheet, cells, values.len());
            let blanks = chart
                .child_by_local("dispBlanksAs")
                .and_then(|el| el.attr_local("val"))
                .unwrap_or("gap");
            let unverified = errors
                .iter()
                .zip(&unavailable)
                .any(|(error, na)| *error && (blanks != "span" || !*na));
            if unverified {
                return Err(unsupported());
            }
        }
        // Like the bubble-only flag, NA-as-blank is inert without NA points.
        // Do not reject a fully numeric ordinary line for an inactive option.
        if kind == PlotKind::StandardLine
            && unavailable_points.iter().any(|na| *na)
            && find_deep(root, "dispNaAsBlank")
                .is_some_and(|e| !matches!(e.attr_local("val"), Some("0" | "false" | "off")))
        {
            return Err(unsupported());
        }
        if value_label.is_some()
            && error_points
                .iter()
                .zip(&unavailable_points)
                .any(|(error, na)| *error && !na)
        {
            return Err(unsupported());
        }
        let name = series_name(ser, sheet, cells).unwrap_or_else(|| format!("系列{}", order + 1));
        let mut line_color = None;
        let style = match kind {
            PlotKind::ClusteredColumn | PlotKind::ClusteredBar => LineSeriesStyle {
        join: stroke::Join::Round,
        dash: None,
                cap: stroke::Cap::Round,
                color: series_color(ser, accents),
                visible: false,
                width: 0.0,
                marker: MarkerKind::None,
                marker_color: None,
            },
            PlotKind::StandardLine => read_line_series(ser, accents, dpi, order, true)?,
            PlotKind::StandardArea => {
                let area = read_area_series(ser, accents, dpi)?;
                line_color = area.line_color;
                LineSeriesStyle {
        join: stroke::Join::Round,
        dash: None,
                cap: stroke::Cap::Round,
                    color: area.fill,
                    visible: area.line_visible,
                    width: area.line_width,
                    marker: MarkerKind::None,
                    marker_color: None,
                }
            }
            PlotKind::Pie => read_pie_series_style(ser, accents, dpi)?,
            PlotKind::Scatter { connect } => {
                let mut style = read_line_series(ser, accents, dpi, order, false)?;
                if style.cap != stroke::Cap::Round || style.dash.is_some() { return Err(unsupported()); }
                if !connect {
                    style.visible = false;
                }
                style
            }
        };
        if series.len() <= order {
            series.resize_with(order + 1, Series::default);
        }
        let is_bar = matches!(kind, PlotKind::ClusteredBar | PlotKind::ClusteredColumn);
        let mut point_inversion = vec![None; values.len()];
        if is_bar {
            for point in ser.element_children().filter(|el| el.local_name() == "dPt") {
                if let Some(invert) = flag(point, "invertIfNegative") {
                    let index = number_val(point, "idx").ok_or_else(unsupported)?;
                    if !index.is_finite()
                        || index < 0.0
                        || index.fract() != 0.0
                        || index as usize >= point_inversion.len()
                        || point_inversion[index as usize].is_some()
                    {
                        return Err(unsupported());
                    }
                    point_inversion[index as usize] = Some(invert);
                }
            }
        }
        let negative_outline = if is_bar {
            ser.child_by_local("spPr")
                .and_then(|sp| sp.child_by_local("ln"))
                .map(|ln| read_solid_ln(ln, accents, dpi.pt_to_px_font(0.75), dpi))
                .transpose()?
        } else {
            None
        };
        let values_len = values.len();
        series[order] = Series {
            name,
            values,
            color: style.color,
            slot: order,
            line_visible: style.visible,
            line_width: style.width,
            line_cap: style.cap,
            line_join: style.join,
            line_dash: style.dash,
            marker: style.marker,
            marker_color: style.marker_color,
            line_color,
            x_values,
            invert_if_negative: is_bar && flag(ser, "invertIfNegative").unwrap_or(true),
            point_inversion,
            negative_outline,
            value_label,
            deleted_label_points: if matches!(
                kind,
                PlotKind::ClusteredColumn | PlotKind::ClusteredBar | PlotKind::StandardLine
            ) {
                let visibility =
                    read_deleted_label_points(ser.child_by_local("dLbls"), values_len)?;
                if value_label.is_none() && !visibility.shown.is_empty() {
                    // Point showVal=1 cannot silently omit labels when the
                    // series itself is hidden; that combination stays NI
                    // until per-point show is painted.
                    return Err(unsupported());
                }
                visibility.deleted
            } else {
                Vec::new()
            },
            error_points,
            unavailable_points,
            span_blanks: kind == PlotKind::StandardLine
                && chart
                    .child_by_local("dispBlanksAs")
                    .and_then(|el| el.attr_local("val"))
                    == Some("span"),
            on_secondary,
        };
    }
    }
    // Official all-empty pies keep category cache points but omit every val
    // pt. Do not drop that series before the category domain is applied.
    if kind != PlotKind::Pie {
        series.retain(|ser| !ser.values.is_empty() || !ser.x_values.is_empty());
    }
    if series.is_empty() {
        return Err(unsupported());
    }
    let mut n = series
        .iter()
        .map(|ser| ser.values.len().max(ser.x_values.len()))
        .max()
        .unwrap_or(0);
    if n == 0 && kind != PlotKind::Pie {
        return Err(unsupported());
    }
    let mut categories = if matches!(kind, PlotKind::Scatter { .. }) {
        Vec::new()
    } else {
        cache_labels(series_cat(wrapper))
    };
    if categories.is_empty() && !matches!(kind, PlotKind::Scatter { .. }) {
        if let Some(text) = formula_text(series_cat(wrapper)) {
            if let Some(labels) = labels_from_cells(&text, sheet, cells) {
                categories = labels;
            }
        }
    }
    if kind == PlotKind::StandardLine || kind == PlotKind::Pie {
        n = n.max(categories.len());
    }
    if n == 0 {
        return Err(unsupported());
    }
    if !matches!(kind, PlotKind::Scatter { .. }) {
        if categories.len() < n {
            let start = categories.len();
            categories.extend((start..n).map(|i| (i + 1).to_string()));
        }
        categories.truncate(n);
    }
    if kind == PlotKind::StandardLine || series.iter().any(|s| s.value_label.is_some()) {
        // Do not silently shrink the category domain when both caches omit
        // trailing points. Sparse-domain/resource-budget support is separate;
        // this batch requires a complete domain from parsed values/categories.
        for wrap in &ser_wrappers {
        for ser in wrap
            .element_children()
            .filter(|e| e.local_name() == "ser")
        {
            if let Some(count) = ser
                .child_by_local("val")
                .and_then(|e| find_deep(e, "numCache").or_else(|| find_deep(e, "numLit")))
                .and_then(|e| e.child_by_local("ptCount"))
            {
                let count = count
                    .attr_local("val")
                    .and_then(|v| v.parse::<u64>().ok())
                    .ok_or_else(unsupported)?;
                if count > n as u64 {
                    return Err(unsupported());
                }
            }
        }
        }
    }
    for ser in &mut series {
        ser.values.resize(n, None);
        // Column blank/error label text has not yet been locked by a public
        // probe. Never report successful showVal while silently omitting it.
        if matches!(kind, PlotKind::ClusteredColumn | PlotKind::ClusteredBar)
            && ser.value_label.is_some()
            && ser.values.iter().any(Option::is_none)
        {
            return Err(unsupported());
        }
        if kind == PlotKind::StandardLine {
            ser.error_points.resize(n, false);
            ser.unavailable_points.resize(n, false);
            let mode = chart
                .child_by_local("dispBlanksAs")
                .and_then(|el| el.attr_local("val"))
                .unwrap_or("gap");
            if mode != "gap"
                && ser
                    .error_points
                    .iter()
                    .zip(&ser.unavailable_points)
                    .any(|(error, na)| *error && !na)
            {
                return Err(unsupported());
            }
            if chart
                .child_by_local("dispBlanksAs")
                .and_then(|el| el.attr_local("val"))
                == Some("zero")
            {
                for (value, error) in ser.values.iter_mut().zip(&ser.error_points) {
                    if value.is_none() && !error {
                        *value = Some(0.0);
                    }
                }
            }
        }
        if matches!(kind, PlotKind::Scatter { .. }) {
            ser.x_values.resize(n, None);
        }
        if kind == PlotKind::Pie {
            let mode = chart
                .child_by_local("dispBlanksAs")
                .and_then(|el| el.attr_local("val"))
                .unwrap_or("gap");
            // A complete numeric cache makes zero/span inert. Leave residual
            // NI when a blank or non-finite point exists, until official pie
            // blank/zero/span geometry is uniquely locked.
            if mode != "gap"
                && ser
                    .values
                    .iter()
                    .any(|v| v.is_none() || v.is_some_and(|n| !n.is_finite()))
            {
                return Err(unsupported());
            }
        }
    }
    let grid = plot.element_children().any(|el| {
        matches!(el.local_name(), "valAx" | "dateAx")
            && el.child_by_local("majorGridlines").is_some()
    });
    let between = plot
        .element_children()
        .filter(|el| el.local_name() == "valAx")
        .all(|el| {
            el.child_by_local("crossBetween")
                .and_then(|c| c.attr_local("val"))
                .unwrap_or("between")
                == "between"
        });
    // Official Excel 16 areaChart stacked / percentStacked writes valAx
    // crossBetween=midCat (first/last categories on the plot edges). Standard
    // area and all line charts stay on the existing between path.
    let area_midcat = matches!(kind, PlotKind::StandardArea)
        && stacking != Stacking::Clustered
        && plot.element_children().filter(|el| el.local_name() == "valAx").all(|el| {
            el.child_by_local("crossBetween")
                .and_then(|c| c.attr_local("val"))
                == Some("midCat")
        });
    // Official Excel 16 scatter ChartTypes write crossBetween=midCat on both
    // numeric valAx. This is not the category-slot midCat rule; scatter x/y
    // still map linearly over the axis min/max.
    let scatter_midcat = matches!(kind, PlotKind::Scatter { .. })
        && plot.element_children().filter(|el| el.local_name() == "valAx").all(|el| {
            el.child_by_local("crossBetween")
                .and_then(|c| c.attr_local("val"))
                == Some("midCat")
        });
    if !between && !area_midcat && !scatter_midcat {
        return Err(unsupported());
    }
    let vary_points = (matches!(kind, PlotKind::ClusteredColumn | PlotKind::ClusteredBar)
        && vary
        && series.len() == 1)
        || (kind == PlotKind::Pie && vary);
    let first_slice_ang = if kind == PlotKind::Pie {
        pie_first_slice_ang(wrapper)?
    } else {
        0.0
    };
    let (point_colors, pie_explosion, point_explosions, series_fill, point_fills) =
        if kind == PlotKind::Pie {
            if series.len() != 1 {
                return Err(unsupported());
            }
            let ser = wrapper
                .element_children()
                .find(|el| el.local_name() == "ser")
                .ok_or_else(unsupported)?;
            read_pie_point_colors(ser, accents, n, dpi)?
        } else {
            (Vec::new(), 0.0, Vec::new(), None, Vec::new())
        };
    if kind == PlotKind::Pie {
        // Official Excel 16 public pies paint finite non-zero values at |v|
        // even when every value is negative. All-zero and all-empty pies keep
        // the frame and legend but take no slice. The previous "at least one
        // finite positive" gate is therefore not required.
        if legend_pos(chart).is_some() && !legend::Spec::read(root, accents)?.default_right_ink() {
            return Err(unsupported());
        }
    }
    let (axis, axis_x, grid_x, cats_from_bottom) = if matches!(kind, PlotKind::Pie) {
        (axis::Spec::default(), None, false, true)
    } else if matches!(kind, PlotKind::Scatter { .. }) {
        let (x, y) = axis::scatter_axes(plot)?;
        let grid_x = plot.element_children().any(|el| {
            el.local_name() == "valAx"
                && matches!(
                    el.child_by_local("axPos")
                        .and_then(|p| p.attr_local("val"))
                        .unwrap_or(""),
                    "b" | "t"
                )
                && el.child_by_local("majorGridlines").is_some()
        });
        (y, Some(x), grid_x, true)
    } else if kind == PlotKind::ClusteredBar {
        let (spec, from_bottom) = if stacking == Stacking::PercentStacked {
            axis::bar_axes_percent_stacked(plot)?
        } else {
            axis::bar_axes(plot)?
        };
        (spec, None, false, from_bottom)
    } else if locked_secondary.is_some() {
        let sec = locked_secondary.as_ref().unwrap();
        (sec.primary, None, false, true)
    } else if stacking == Stacking::PercentStacked {
        (axis::Spec::read_percent_stacked(plot)?, None, false, true)
    } else {
        (axis::Spec::read(plot)?, None, false, true)
    };
    let axis_secondary = locked_secondary.as_ref().map(|sec| sec.secondary);
    if axis_secondary.is_some() {
        if series.iter().any(|ser| ser.value_label.is_some())
            || !series.iter().any(|ser| ser.on_secondary)
            || !series.iter().any(|ser| !ser.on_secondary)
        {
            return Err(unsupported());
        }
    }
    if axis.log_base.is_some()
        && !matches!(kind, PlotKind::ClusteredColumn | PlotKind::StandardLine)
    {
        return Err(unsupported());
    }
    if axis_x.as_ref().is_some_and(|spec| spec.log_base.is_some()) {
        return Err(unsupported());
    }
    if axis.log_base.is_some() && series.iter().any(|ser| ser.value_label.is_some()) {
        return Err(unsupported());
    }
    if (axis.crosses_at.is_some() || !matches!(axis.cat_crosses, axis::CatCrosses::AutoZero))
        && !matches!(
            kind,
            PlotKind::ClusteredColumn | PlotKind::StandardLine | PlotKind::StandardArea
        )
    {
        return Err(unsupported());
    }
    // Official F20 evidence is clustered column, standard line, and clustered bar.
    if axis.deleted
        && !matches!(
            kind,
            PlotKind::ClusteredColumn | PlotKind::StandardLine | PlotKind::ClusteredBar
        )
    {
        return Err(unsupported());
    }
    if stacking != Stacking::Clustered {
        // Official F22 evidence is linear autoZero minMax, explicit min/max,
        // no value labels, no inversion, no log / maxMin / delete / custom crosses.
        if axis.min.is_none()
            || axis.max.is_none()
            || axis.log_base.is_some()
            || axis.reversed
            || axis.deleted
            || axis.crosses_at.is_some()
            || !matches!(axis.cat_crosses, axis::CatCrosses::AutoZero)
            || series.iter().any(|ser| ser.value_label.is_some())
            || series.iter().any(|ser| {
                ser.invert_if_negative || ser.point_inversion.iter().any(|p| p.is_some())
            })
        {
            return Err(unsupported());
        }
        if stacking == Stacking::PercentStacked {
            if axis.min != Some(0.0)
                || axis.max != Some(1.0)
                || axis.tick_format != axis::TickFormat::Percent
            {
                return Err(unsupported());
            }
            for ser in &series {
                if ser.values.iter().any(|v| {
                    !v.is_some_and(|n| n.is_finite() && n > 0.0)
                }) {
                    return Err(unsupported());
                }
            }
        }
        // Official F23 line/area evidence has every cache point finite.
        // Blanks, packing, and percentStacked mixed/missing stay NI.
        if matches!(kind, PlotKind::StandardLine | PlotKind::StandardArea) {
            if series.iter().any(|ser| {
                ser.values.iter().any(|v| !v.is_some_and(|n| n.is_finite()))
            }) {
                return Err(unsupported());
            }
        }
        // Official xlLineStacked writes c:marker/symbol=none. Auto or shaped
        // markers on stacked lines are not uniquely locked.
        if kind == PlotKind::StandardLine
            && series
                .iter()
                .any(|ser| !matches!(ser.marker, MarkerKind::None))
        {
            return Err(unsupported());
        }
        if kind == PlotKind::StandardArea && !area_midcat {
            return Err(unsupported());
        }
        if !matches!(
            kind,
            PlotKind::ClusteredColumn
                | PlotKind::ClusteredBar
                | PlotKind::StandardLine
                | PlotKind::StandardArea
        ) {
            return Err(unsupported());
        }
    }
    Ok(ColumnChart {
        categories,
        series,
        gap_width,
        overlap,
        stacking,
        vary_points,
        grid,
        grid_x,
        legend: legend_pos(chart),
        legend_spec: legend::Spec::read(root, accents)?,
        axis,
        axis_x,
        axis_secondary,
        cats_from_bottom,
        title: title_text(chart),
        title_typo: title_typography(chart)?,
        accents,
        kind,
        first_slice_ang,
        point_colors,
        series_fill,
        point_fills,
        pie_explosion,
        point_explosions,
        area_missing_as_zero,
    })
}

pub(super) fn series_cat(bar: &Element) -> Option<&Element> {
    bar.element_children()
        .find(|el| el.local_name() == "ser")
        .and_then(|ser| ser.child_by_local("cat"))
}

pub(super) fn is_chart_wrapper(name: &str) -> bool {
    matches!(
        name,
        "barChart"
            | "bar3DChart"
            | "lineChart"
            | "line3DChart"
            | "pieChart"
            | "pie3DChart"
            | "doughnutChart"
            | "ofPieChart"
            | "areaChart"
            | "area3DChart"
            | "scatterChart"
            | "radarChart"
            | "bubbleChart"
            | "stockChart"
            | "surfaceChart"
            | "surface3DChart"
    )
}


pub(super) fn series_numbers(
    parent: Option<&Element>,
    sheet: &str,
    cells: Option<&super::super::sparkline::CellCache>,
) -> Vec<Option<f64>> {
    let cached = cache_numbers(parent);
    if !cached.is_empty() {
        return cached;
    }
    let Some(text) = formula_text(parent) else {
        return Vec::new();
    };
    numbers_from_cells(&text, sheet, cells).unwrap_or_default()
}

pub(super) fn formula_text(parent: Option<&Element>) -> Option<String> {
    let text = find_deep(parent?, "f")?.text_content();
    let text = text.trim();
    if text.is_empty() {
        None
    } else {
        Some(text.to_string())
    }
}

pub(super) fn numbers_from_cells(
    formula: &str,
    sheet: &str,
    cells: Option<&super::super::sparkline::CellCache>,
) -> Option<Vec<Option<f64>>> {
    let values = super::super::sparkline::range_values(formula, sheet, cells?)?;
    Some(values.into_iter().map(json_number).collect())
}

pub(super) fn labels_from_cells(
    formula: &str,
    sheet: &str,
    cells: Option<&super::super::sparkline::CellCache>,
) -> Option<Vec<String>> {
    let values = super::super::sparkline::range_values(formula, sheet, cells?)?;
    Some(values.into_iter().map(json_label).collect())
}

pub(super) fn json_number(value: serde_json::Value) -> Option<f64> {
    match value {
        serde_json::Value::Number(n) => n.as_f64().filter(|n| n.is_finite()),
        serde_json::Value::Bool(b) => Some(if b { 1.0 } else { 0.0 }),
        serde_json::Value::String(s) => parse_chart_number(&s),
        _ => None,
    }
}

/// Excel charts treat an empty string (empty `inlineStr`, empty cached `<c:v>`) as 0
/// and keep the stroke connected. A missing cell, an omitted cache point, or an
/// unparsable value such as `#N/A` stays `None` and breaks the stroke.
pub(super) fn parse_chart_number(text: &str) -> Option<f64> {
    let text = text.trim();
    if text.is_empty() {
        Some(0.0)
    } else {
        text.parse().ok().filter(|n: &f64| n.is_finite())
    }
}

pub(super) fn json_label(value: serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s,
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(true) => "TRUE".into(),
        serde_json::Value::Bool(false) => "FALSE".into(),
        _ => String::new(),
    }
}

pub(super) fn series_name(
    ser: &Element,
    sheet: &str,
    cells: Option<&super::super::sparkline::CellCache>,
) -> Option<String> {
    let tx = ser.child_by_local("tx")?;
    if let Some(v) = tx.child_by_local("v") {
        let text = v.text_content();
        if !text.is_empty() {
            return Some(text);
        }
    }
    if let Some(cache) = find_deep(tx, "strCache").or_else(|| find_deep(tx, "numCache")) {
        let text = cache_labels_of(cache).into_iter().next()?;
        if !text.is_empty() {
            return Some(text);
        }
    }
    let text = formula_text(Some(tx))?;
    labels_from_cells(&text, sheet, cells)?
        .into_iter()
        .find(|label| !label.is_empty())
}

/// Preserved inherited chart pen / legacy scatter normalization. Native
/// explicit width controls have a distinct nearest-pixel rule (chart_stroke).
/// In particular omitted w remains the accepted 2.25pt/default 4px at 106dpi;
/// an explicit 2.25pt resolves to 3px. Do not globally replace this old rule.
pub(super) fn excel_chart_line_width_px(px: f64) -> f64 {
    if !px.is_finite() || px <= 0.0 {
        return px;
    }
    let rounded = px.round();
    if (px - rounded).abs() <= 1e-6 {
        rounded
    } else {
        px.ceil()
    }
}

pub(super) fn read_line_series(
    ser: &Element,
    accents: [[u8; 3]; 6],
    dpi: RenderDpi,
    slot: usize,
    explicit_pens: bool,
) -> Result<LineSeriesStyle, Error> {
    let unsupported = || Error::with_path(ErrorKind::ChartImageUnsupported, "id");
    if flag(ser, "smooth").unwrap_or(false) {
        return Err(unsupported());
    }
    for name in ["dPt", "trendline", "errBars"] {
        if ser.child_by_local(name).is_some() {
            return Err(unsupported());
        }
    }
    let mut dash = None;
    let mut cap = stroke::Cap::Round;
    let mut join = stroke::Join::Round;
    let mut visible = true;
    let mut width = excel_chart_line_width_px(dpi.pt_to_px_font(2.25));
    let mut color = None;
    if let Some(sp) = ser.child_by_local("spPr") {
        let style = read_line_sp_pr(sp, accents, dpi, explicit_pens)?;
        dash = style.dash;
        cap = style.cap;
        join = style.join;
        visible = style.visible;
        width = excel_chart_line_width_px(style.width);
        color = style.color;
    }
    if color.is_none() {
        color = series_color(ser, accents);
    }
    let (marker, marker_color) = read_line_marker(ser, accents, dpi, slot)?;
    Ok(LineSeriesStyle {
        dash,
        cap,
        join,
        color,
        visible,
        width,
        marker,
        marker_color,
    })
}

pub(super) fn read_line_sp_pr(
    sp: &Element,
    accents: [[u8; 3]; 6],
    dpi: RenderDpi,
    explicit_pens: bool,
) -> Result<LineSeriesStyle, Error> {
    let unsupported = || Error::with_path(ErrorKind::ChartImageUnsupported, "id");
    let mut dash = None;
    let mut cap = stroke::Cap::Round;
    let mut join = stroke::Join::Round;
    let mut visible = true;
    let mut width = excel_chart_line_width_px(dpi.pt_to_px_font(2.25));
    let mut color = None;
    for child in sp.element_children() {
        match child.local_name() {
            "noFill" | "solidFill" => {}
            "effectLst" => {
                if child.element_children().next().is_some() {
                    return Err(unsupported());
                }
            }
            "ln" => {
                let line = read_ln_with_caps(child, accents, width, dpi, explicit_pens)?;
                dash = line.dash;
                cap = line.cap;
                join = line.join;
                visible = line.visible;
                width = excel_chart_line_width_px(line.width);
                if line.color.is_some() {
                    color = line.color;
                }
            }
            _ => return Err(unsupported()),
        }
    }
    if color.is_none() {
        if let Some(fill) = sp.child_by_local("solidFill") {
            color = color_of_slot(fill, &accents);
        }
    }
    Ok(LineSeriesStyle {
        dash,
        cap,
        join,
        color,
        visible,
        width,
        marker: MarkerKind::None,
        marker_color: None,
    })
}

pub(super) fn read_solid_ln(
    ln: &Element,
    accents: [[u8; 3]; 6],
    default_width: f64,
    dpi: RenderDpi,
) -> Result<LineSeriesStyle, Error> {
    read_ln_with_caps(ln, accents, default_width, dpi, false)
}

pub(super) fn read_ln_with_caps(ln: &Element, accents: [[u8;3];6], default_width: f64, dpi: RenderDpi, explicit_caps: bool) -> Result<LineSeriesStyle, Error> {
    let unsupported = || Error::with_path(ErrorKind::ChartImageUnsupported, "id");
    // Omitted cap deliberately retains the accepted chart baseline, not a new
    // guessed global default. Only explicit cap values are expanded here.
    let cap = match ln.attr_local("cap") { Some(v) => stroke::Cap::from_ooxml(v).ok_or_else(unsupported)?, None => stroke::Cap::Round };
    if !explicit_caps && cap != stroke::Cap::Round { return Err(unsupported()); }
    for (attr, allowed) in [
        ("cmpd", &["sng"][..]),
        ("algn", &["ctr"][..]),
    ] {
        if let Some(val) = ln.attr_local(attr) {
            if !allowed.contains(&val) {
                return Err(unsupported());
            }
        }
    }
    for (name, _) in &ln.attrs {
        let local = name.rsplit(':').next().unwrap_or(name.as_str());
        if local == "xmlns" || name.starts_with("xmlns:") {
            continue;
        }
        if !matches!(local, "w" | "cap" | "cmpd" | "algn") {
            return Err(unsupported());
        }
    }
    let mut width = ln
        .attr_local("w")
        .and_then(|raw| raw.parse::<f64>().ok())
        .map(|emu| dpi.emu_to_px_f(emu))
        .unwrap_or(default_width);
    if !width.is_finite() || width < 0.0 {
        return Err(unsupported());
    }
    if explicit_caps && ln.attr_local("w").is_some() {
        width = stroke::explicit_pen_width_px(width).ok_or_else(unsupported)?;
    }
    let mut join = stroke::Join::Round;
    let mut dash = None;
    let mut dash_declared = false;
    let mut visible = true;
    let mut color = None;
    for child in ln.element_children() {
        match child.local_name() {
            "noFill" => visible = false,
            "solidFill" => {
                if cap != stroke::Cap::Round && child.element_children().any(|node| node.element_children().next().is_some()) {
                    // New cap coverage is an opaque-color baseline, not permission
                    // to flatten alpha or invent additional color transforms.
                    return Err(unsupported());
                }
                color = Some(color_of_slot(child, &accents).ok_or_else(unsupported)?);
            }
            "custDash" => {
                // Explicit opaque custom dashes: flat and square positive stops;
                // round includes its caps and clamps short positive d to a dot.
                // Omitted caps and zero dash/space remain separately gated.
                if dash_declared || !explicit_caps || ln.attr_local("cap").is_none()
                    || child.attrs.iter().any(|(key,_)| !key.starts_with("xmlns"))
                    || ln.attr_local("w").and_then(|v| v.parse::<f64>().ok())
                        .is_none_or(|w| !w.is_finite() || w <= 0.0)
                { return Err(unsupported()); }
                let mut stops = Vec::new();
                for stop in child.element_children() {
                    if stop.local_name() != "ds" || stop.element_children().next().is_some()
                        || stop.attrs.iter().any(|(k,_)| !matches!(k.as_str(), "d"|"sp") && !k.starts_with("xmlns"))
                    { return Err(unsupported()); }
                    let percentage = |key| stop.attr_local(key).and_then(|v| v.parse::<i32>().ok())
                        .filter(|v| *v > 0).map(f64::from).ok_or_else(unsupported);
                    let d=percentage("d")?;
                    stops.push((d,percentage("sp")?));
                }
                dash = Some(std::sync::Arc::new(stroke::DashPattern::from_width_percent(&stops,width).ok_or_else(unsupported)?));
                dash_declared = true;
            }
            "prstDash" => {
                if dash_declared { return Err(unsupported()); }
                dash_declared = true;
                let value = child.attr_local("val").unwrap_or("solid");
                if value != "solid" {
                    // Presets reuse the same run/cap/join sampler as custom
                    // dashes. Explicit flat pens are native-verified for all
                    // ten tokens; round dash/dot are separately discriminated.
                    // No inferred default cap/width, square terminal-gap rule,
                    // other host expansion, or legend/plot compensation.
                    if !explicit_caps || ln.attr_local("cap").is_none()
                        || !(cap == stroke::Cap::Flat || (cap == stroke::Cap::Round && matches!(value,"dash"|"dot")))
                        || ln.attr_local("w").and_then(|v|v.parse::<f64>().ok()).is_none_or(|w|!w.is_finite() || w<=0.0)
                        || child.element_children().next().is_some()
                        || child.attrs.iter().any(|(k,_)| k!="val" && !k.starts_with("xmlns"))
                    { return Err(unsupported()); }
                    dash=Some(std::sync::Arc::new(stroke::DashPattern::from_preset_width(value,width).ok_or_else(unsupported)?));
                }
            }
            "round" => {},
            "bevel" | "miter" if explicit_caps => {},
            "miter" | "bevel" if cap == stroke::Cap::Round => {},
            "effectLst" => {
                if child.element_children().next().is_some() {
                    return Err(unsupported());
                }
            }
            _ => return Err(unsupported()),
        }
    }
    let joins:Vec<_>=ln.element_children().filter(|c|matches!(c.local_name(),"round"|"bevel"|"miter")).collect();
    // New noncustom joins require an explicitly resolved simple pen. Omitted
    // width/cap, default/round joins and other stroke readers keep their exact
    // legacy path; a styled join must not silently become a round join.
    let explicit_solid_join = dash.is_none() && explicit_caps
        && ln.attr_local("cap").is_some()
        && ln.attr_local("w").and_then(|v|v.parse::<f64>().ok()).is_some_and(|w|w.is_finite() && w>0.0)
        && joins.iter().any(|c|matches!(c.local_name(),"bevel"|"miter"));
    if (dash.is_some() || explicit_solid_join) && (!visible || ln.element_children().any(|child|
        child.local_name()=="solidFill" && child.element_children()
            .any(|color| color.element_children().next().is_some()))) {
        return Err(unsupported());
    }
    if dash.is_some() || explicit_solid_join {
        if joins.len()>1 { return Err(unsupported()); }
        if let Some(c)=joins.first() {
            if c.element_children().next().is_some() || c.attrs.iter().any(|(k,_)|
                !k.starts_with("xmlns") && !(c.local_name()=="miter" && k=="lim")) { return Err(unsupported()); }
            join=match c.local_name() {
                "bevel" => stroke::Join::Bevel,
                "miter" => {
                    // Office DrawingML default: 800%; verified against omitted
                    // lim native controls, not an invented renderer constant.
                    let limit=match c.attr_local("lim") {
                        Some(v)=>v.parse::<i32>().ok().filter(|v| *v>=0).ok_or_else(unsupported)? as u32,
                        None=>800000,
                    };
                    stroke::Join::Miter { limit }
                },
                _ => stroke::Join::Round,
            };
        }
    } else if cap!=stroke::Cap::Round && joins.iter().any(|c|matches!(c.local_name(),"bevel"|"miter")) {
        // Unresolved/non-explicit solid pens remain outside this new path.
        return Err(unsupported());
    }
    if !visible {
        color = None;
    }
    Ok(LineSeriesStyle {
        dash,
        cap,
        join,
        color,
        visible,
        width,
        marker: MarkerKind::None,
        marker_color: None,
    })
}

pub(super) fn read_line_marker(
    ser: &Element,
    accents: [[u8; 3]; 6],
    dpi: RenderDpi,
    slot: usize,
) -> Result<(MarkerKind, Option<[u8; 3]>), Error> {
    let unsupported = || Error::with_path(ErrorKind::ChartImageUnsupported, "id");
    let Some(marker) = ser.child_by_local("marker") else {
        return Ok((
            shaped_marker(default_marker_symbol(slot), OMITTED_MARKER_SIZE_PT, dpi),
            None,
        ));
    };
    let symbol = marker
        .child_by_local("symbol")
        .and_then(|el| el.attr_local("val"))
        .unwrap_or("auto");
    if symbol == "none" {
        return Ok((MarkerKind::None, None));
    }
    let symbol = if symbol == "auto" {
        default_marker_symbol(slot)
    } else {
        marker_symbol_from_name(symbol).ok_or_else(unsupported)?
    };
    let size_pt = number_val(marker, "size").unwrap_or(EXPLICIT_MARKER_DEFAULT_SIZE_PT);
    if !size_pt.is_finite() || size_pt <= 0.0 {
        return Err(unsupported());
    }
    let mut marker_color = None;
    if let Some(sp) = marker.child_by_local("spPr") {
        for child in sp.element_children() {
            match child.local_name() {
                "solidFill" => {
                    marker_color = Some(color_of_slot(child, &accents).ok_or_else(unsupported)?);
                }
                "noFill" => return Err(unsupported()),
                "ln" => {
                    let _ = read_solid_ln(child, accents, 1.0, dpi)?;
                }
                "effectLst" => {
                    if child.element_children().next().is_some() {
                        return Err(unsupported());
                    }
                }
                _ => return Err(unsupported()),
            }
        }
    }
    Ok((shaped_marker(symbol, size_pt, dpi), marker_color))
}

pub(super) fn pie_first_slice_ang(wrapper: &Element) -> Result<f64, Error> {
    let unsupported = || Error::with_path(ErrorKind::ChartImageUnsupported, "id");
    let Some(el) = wrapper.child_by_local("firstSliceAng") else {
        return Ok(0.0);
    };
    let angle = el
        .attr_local("val")
        .unwrap_or("0")
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite())
        .ok_or_else(unsupported)?;
    if !(0.0..=360.0).contains(&angle) {
        return Err(unsupported());
    }
    if (angle - 360.0).abs() < 1e-9 {
        Ok(0.0)
    } else {
        Ok(angle)
    }
}

pub(super) fn read_pie_series_style(
    ser: &Element,
    accents: [[u8; 3]; 6],
    dpi: RenderDpi,
) -> Result<LineSeriesStyle, Error> {
    let unsupported = || Error::with_path(ErrorKind::ChartImageUnsupported, "id");
    data_labels_hidden(ser.child_by_local("dLbls"))?;
    for name in ["trendline", "errBars"] {
        if ser.child_by_local(name).is_some() {
            return Err(unsupported());
        }
    }
    // Series explosion is applied in draw_pie_slices after the category
    // domain is known. A non-zero value is accepted here and validated
    // together with dPt explosion in read_pie_point_colors.
    if let Some(explosion) = number_val(ser, "explosion") {
        if !explosion.is_finite() || explosion < 0.0 {
            return Err(unsupported());
        }
    }
    if let Some(sp) = ser.child_by_local("spPr") {
        let _ = read_pie_sp_pr(sp, accents, dpi)?;
    }
    Ok(LineSeriesStyle {
        join: stroke::Join::Round,
        dash: None,
                cap: stroke::Cap::Round,
        color: series_color(ser, accents),
        visible: false,
        width: 0.0,
        marker: MarkerKind::None,
        marker_color: None,
    })
}

pub(super) fn pie_color_node(slot: &Element) -> Option<&Element> {
    slot.child_by_local("srgbClr")
        .or_else(|| slot.child_by_local("sysClr"))
        .or_else(|| slot.child_by_local("schemeClr"))
        .or_else(|| {
            if matches!(slot.local_name(), "srgbClr" | "sysClr" | "schemeClr") {
                Some(slot)
            } else {
                None
            }
        })
}

pub(super) fn pie_solid_fill(slot: &Element, accents: &[[u8; 3]; 6]) -> Result<SliceFill, Error> {
    let unsupported = || Error::with_path(ErrorKind::ChartImageUnsupported, "id");
    let color = color_of_slot(slot, accents).ok_or_else(unsupported)?;
    let Some(node) = pie_color_node(slot) else {
        return Err(unsupported());
    };
    let mut alpha = 255u8;
    let mut saw_alpha = false;
    for child in node.element_children() {
        match child.local_name() {
            "alpha" => {
                if saw_alpha {
                    return Err(unsupported());
                }
                saw_alpha = true;
                let raw = child.attr_local("val").ok_or_else(unsupported)?;
                let val = raw
                    .parse::<f64>()
                    .ok()
                    .filter(|v| v.is_finite() && *v >= 0.0 && *v <= 100_000.0)
                    .ok_or_else(unsupported)?;
                alpha = ooxml_alpha_to_u8(val.round() as u32);
            }
            "alphaMod" | "alphaOff" => return Err(unsupported()),
            "lumMod" | "lumOff" | "shade" | "tint" => {
                // Opaque F07 transforms already resolve through color_of_slot.
                // Official pie evidence did not lock those transforms with a
                // non-opaque alpha, so that combination stays NI.
                if saw_alpha || color_has_non_opaque_alpha(slot) {
                    return Err(unsupported());
                }
            }
            _ => {}
        }
    }
    if alpha < 255 && node.local_name() != "srgbClr" {
        return Err(unsupported());
    }
    Ok(SliceFill { color, alpha })
}

pub(super) fn pie_hidden_fill_memory(ext_lst: &Element) -> bool {
    let mut count = 0;
    for ext in ext_lst.element_children() {
        if ext.local_name() != "ext" {
            return false;
        }
        count += 1;
        let uri = ext.attr_local("uri").unwrap_or("");
        if uri != PIE_HIDDEN_FILL_URI {
            return false;
        }
        let mut kids = ext.element_children();
        let Some(hidden) = kids.next() else {
            return false;
        };
        if kids.next().is_some() || hidden.local_name() != "hiddenFill" {
            return false;
        }
    }
    count == 1
}

pub(super) fn read_pie_sp_pr(
    sp: &Element,
    accents: [[u8; 3]; 6],
    dpi: RenderDpi,
) -> Result<Option<SliceFill>, Error> {
    let unsupported = || Error::with_path(ErrorKind::ChartImageUnsupported, "id");
    let mut fill = None;
    let mut no_fill = false;
    for child in sp.element_children() {
        match child.local_name() {
            "solidFill" => {
                if no_fill || fill.is_some() {
                    return Err(unsupported());
                }
                fill = Some(pie_solid_fill(child, &accents)?);
            }
            "noFill" => {
                if no_fill || fill.is_some() {
                    return Err(unsupported());
                }
                no_fill = true;
                fill = Some(SliceFill {
                    color: [0, 0, 0],
                    alpha: 0,
                });
            }
            "gradFill" | "pattFill" | "blipFill" => return Err(unsupported()),
            "ln" => {
                let line = read_solid_ln(child, accents, 0.0, dpi)?;
                if line.visible {
                    return Err(unsupported());
                }
            }
            "effectLst" => {
                if child.element_children().next().is_some() {
                    return Err(unsupported());
                }
            }
            "extLst" => {
                if !no_fill || !pie_hidden_fill_memory(child) {
                    return Err(unsupported());
                }
            }
            _ => return Err(unsupported()),
        }
    }
    Ok(fill)
}

pub(super) fn read_pie_point_colors(
    ser: &Element,
    accents: [[u8; 3]; 6],
    n: usize,
    dpi: RenderDpi,
) -> Result<
    (
        Vec<Option<[u8; 3]>>,
        f64,
        Vec<f64>,
        Option<SliceFill>,
        Vec<Option<SliceFill>>,
    ),
    Error,
> {
    let unsupported = || Error::with_path(ErrorKind::ChartImageUnsupported, "id");
    let pie_explosion = match number_val(ser, "explosion") {
        Some(v) if v.is_finite() && v >= 0.0 => v,
        None => 0.0,
        _ => return Err(unsupported()),
    };
    let series_fill = match ser.child_by_local("spPr") {
        Some(sp) => read_pie_sp_pr(sp, accents, dpi)?,
        None => None,
    };
    let mut colors = vec![None; n];
    let mut point_fills = vec![None; n];
    let mut point_explosions = vec![0.0; n];
    for dpt in ser.element_children().filter(|el| el.local_name() == "dPt") {
        let idx = number_val(dpt, "idx").ok_or_else(unsupported)?;
        if !idx.is_finite() || idx < 0.0 || idx.fract() != 0.0 {
            return Err(unsupported());
        }
        let idx = idx as usize;
        if idx >= n
            || colors[idx].is_some()
            || point_fills[idx].is_some()
            || point_explosions[idx] != 0.0
        {
            return Err(unsupported());
        }
        if let Some(explosion) = number_val(dpt, "explosion") {
            if !explosion.is_finite() || explosion < 0.0 {
                return Err(unsupported());
            }
            // Official point explosion and series explosion were not written
            // together; that combination stays NI.
            if pie_explosion != 0.0 && explosion != 0.0 {
                return Err(unsupported());
            }
            point_explosions[idx] = explosion;
        }
        if let Some(bubble) = dpt.child_by_local("bubble3D") {
            // Official exploded dPt writes bubble3D=0 on a 2D pie; treat that
            // as a no-op. Other values stay NI.
            let val = bubble
                .attr_local("val")
                .unwrap_or("1")
                .parse::<f64>()
                .ok()
                .filter(|v| v.is_finite())
                .ok_or_else(unsupported)?;
            if val != 0.0 {
                return Err(unsupported());
            }
        }
        for name in ["invertIfNegative", "marker", "pictureOptions"] {
            if dpt.child_by_local(name).is_some() {
                return Err(unsupported());
            }
        }
        if let Some(sp) = dpt.child_by_local("spPr") {
            if let Some(fill) = read_pie_sp_pr(sp, accents, dpi)? {
                point_fills[idx] = Some(fill);
                if fill.alpha > 0 {
                    colors[idx] = Some(fill.color);
                }
            }
        }
    }
    Ok((colors, pie_explosion, point_explosions, series_fill, point_fills))
}

pub(super) fn read_area_series(
    ser: &Element,
    accents: [[u8; 3]; 6],
    dpi: RenderDpi,
) -> Result<AreaSeriesStyle, Error> {
    let unsupported = || Error::with_path(ErrorKind::ChartImageUnsupported, "id");
    if flag(ser, "smooth").unwrap_or(false) {
        return Err(unsupported());
    }
    data_labels_hidden(ser.child_by_local("dLbls"))?;
    for name in ["dPt", "trendline", "errBars"] {
        if ser.child_by_local(name).is_some() {
            return Err(unsupported());
        }
    }
    if let Some(marker) = ser.child_by_local("marker") {
        let symbol = marker
            .child_by_local("symbol")
            .and_then(|el| el.attr_local("val"))
            .unwrap_or("auto");
        if symbol != "none" {
            return Err(unsupported());
        }
    }
    let mut fill = None;
    let mut line_visible = false;
    let mut line_width = dpi.pt_to_px_font(2.25);
    let mut line_color = None;
    if let Some(sp) = ser.child_by_local("spPr") {
        for child in sp.element_children() {
            match child.local_name() {
                "solidFill" => {
                    if color_has_non_opaque_alpha(child) {
                        return Err(unsupported());
                    }
                    fill = Some(color_of_slot(child, &accents).ok_or_else(unsupported)?);
                }
                "noFill" => return Err(unsupported()),
                "ln" => {
                    if let Some(solid) = child.child_by_local("solidFill") {
                        if color_has_non_opaque_alpha(solid) {
                            return Err(unsupported());
                        }
                    }
                    let line = read_solid_ln(child, accents, line_width, dpi)?;
                    line_visible = line.visible;
                    line_width = line.width;
                    line_color = line.color;
                }
                "effectLst" => {
                    if child.element_children().next().is_some() {
                        return Err(unsupported());
                    }
                }
                _ => return Err(unsupported()),
            }
        }
    }
    Ok(AreaSeriesStyle {
        fill,
        line_visible,
        line_width,
        line_color,
    })
}

pub(super) fn cache_numbers(parent: Option<&Element>) -> Vec<Option<f64>> {
    let Some(parent) = parent else {
        return Vec::new();
    };
    let Some(cache) = find_deep(parent, "numCache").or_else(|| find_deep(parent, "numLit")) else {
        return Vec::new();
    };
    points_to_values(&read_pts(cache), parse_chart_number)
}

pub(super) fn cache_labels(parent: Option<&Element>) -> Vec<String> {
    let Some(parent) = parent else {
        return Vec::new();
    };
    let Some(cache) = find_cache(parent) else {
        return Vec::new();
    };
    cache_labels_of(cache)
}

pub(super) fn cache_labels_of(cache: &Element) -> Vec<String> {
    points_to_values(&read_pts(cache), |text| Some(text.to_string()))
        .into_iter()
        .map(|item| item.unwrap_or_default())
        .collect()
}

pub(super) fn find_cache(el: &Element) -> Option<&Element> {
    find_deep(el, "strCache")
        .or_else(|| find_deep(el, "numCache"))
        .or_else(|| find_deep(el, "strLit"))
        .or_else(|| find_deep(el, "numLit"))
}

pub(super) fn read_pts(cache: &Element) -> Vec<(usize, String)> {
    let mut pts = Vec::new();
    for pt in cache.element_children() {
        if pt.local_name() != "pt" {
            continue;
        }
        let Some(idx) = pt.attr_local("idx").and_then(|s| s.parse().ok()) else {
            continue;
        };
        let text = pt
            .child_by_local("v")
            .map(|el| el.text_content())
            .unwrap_or_default();
        pts.push((idx, text));
    }
    pts
}

pub(super) fn points_to_values<T>(pts: &[(usize, String)], map: impl Fn(&str) -> Option<T>) -> Vec<Option<T>> {
    let len = pts.iter().map(|(idx, _)| *idx + 1).max().unwrap_or(0);
    let mut out = Vec::new();
    out.resize_with(len, || None);
    for (idx, text) in pts {
        if let Some(value) = map(text.trim()) {
            out[*idx] = Some(value);
        }
    }
    out
}

/// Use the renderer that will actually draw the label. On Windows chart text
/// is GDI; fontdue's fallback line leading is not its line box and would shift
/// outside-end labels upward, even when the visible glyph itself is correct.
pub(super) fn value_label_height(font: Option<&Font>, gdi: Option<&super::super::chart_gdi::Face>, px: f32) -> f64 {
    gdi.map_or_else(|| text_height(font, px), |face| face.em_box())
}

/// Locked General integers are unchanged (no scientific notation or locale
/// separators). Finite non-integers use a close-enough General decimal: 10
/// fraction digits with trailing zeros trimmed, so official 9.1999… becomes
/// "9.2". This is not a unique official precision lock. Non-finite values
/// and magnitudes >= 1e11 still stay unsupported.
pub(super) fn value_label_text(value: f64) -> Option<String> {
    if !value.is_finite() || value.abs() >= 100_000_000_000.0 {
        return None;
    }
    if value.fract() == 0.0 {
        return Some(if value == 0.0 {
            "0".to_owned()
        } else {
            format!("{value:.0}")
        });
    }
    let mut text = format!("{value:.10}");
    if let Some(dot) = text.find('.') {
        while text.ends_with('0') && text.len() > dot + 1 {
            text.pop();
        }
        if text.ends_with('.') {
            text.pop();
        }
    }
    Some(text)
}

/// Series-level point visibility is a general rule, not a unique pixel lock.
/// `delete=true` or `showVal=false` hides that label; `delete=false` or
/// `showVal=true` keeps it. Known Excel editing identity extensions stay
/// inert; unknown extensions, conflicting flags, and visible position/style
/// or custom text remain NI.
#[derive(Debug)]
pub(super) struct PointLabelVisibility {
    pub deleted: Vec<usize>,
    pub shown: Vec<usize>,
}

fn parse_onoff_attr(e: &Element) -> Option<bool> {
    match e.attr_local("val") {
        None | Some("1") | Some("true") => Some(true),
        Some("0") | Some("false") | Some("off") => Some(false),
        _ => None,
    }
}

pub(super) fn read_deleted_label_points(
    labels: Option<&Element>,
    len: usize,
) -> Result<PointLabelVisibility, Error> {
    let unsupported = || Error::with_path(ErrorKind::ChartImageUnsupported, "id");
    let empty = || PointLabelVisibility {
        deleted: Vec::new(),
        shown: Vec::new(),
    };
    let Some(labels) = labels else {
        return Ok(empty());
    };
    let scalar = |e: &Element| {
        e.element_children().next().is_none()
            && e.attrs
                .iter()
                .filter(|(k, _)| !k.starts_with("xmlns"))
                .all(|(k, _)| k == "val")
            && e.attrs.iter().filter(|(k, _)| k == "val").count() <= 1
    };
    let mut deleted = Vec::new();
    let mut shown = Vec::new();
    for point in labels
        .element_children()
        .filter(|e| e.local_name() == "dLbl")
    {
        if point.attrs.iter().any(|(k, _)| !k.starts_with("xmlns")) {
            return Err(unsupported());
        }
        let mut idx = None;
        let mut delete = None;
        let mut show_val = None;
        let mut ext_seen = false;
        for child in point.element_children() {
            match child.local_name() {
                "idx" if idx.is_none() && scalar(child) => {
                    let raw = child.attr_local("val").ok_or_else(unsupported)?;
                    if raw.is_empty() || !raw.bytes().all(|b| b.is_ascii_digit()) {
                        return Err(unsupported());
                    }
                    idx = Some(raw.parse::<usize>().map_err(|_| unsupported())?);
                }
                "delete" if delete.is_none() && scalar(child) => {
                    delete = Some(parse_onoff_attr(child).ok_or_else(unsupported)?);
                }
                "showVal" if show_val.is_none() && scalar(child) => {
                    show_val = Some(parse_onoff_attr(child).ok_or_else(unsupported)?);
                }
                "extLst" if !ext_seen => {
                    ext_seen = true;
                    for ext in child.element_children() {
                        if ext.local_name() != "ext" {
                            return Err(unsupported());
                        }
                        let mut settings = ext.element_children();
                        let inert = match ext.attr_local("uri") {
                            Some("{CE6537A1-D6FC-4f65-9D91-7224C49458BB}") => {
                                settings.next().is_none()
                            }
                            Some("{C3380CC4-5D6E-409C-BE32-E72D297353CC}") => {
                                settings.next().is_some_and(|e| {
                                    e.local_name() == "uniqueId"
                                        && scalar(e)
                                        && e.attr_local("val").is_some()
                                }) && settings.next().is_none()
                            }
                            _ => false,
                        };
                        if !inert {
                            return Err(unsupported());
                        }
                    }
                }
                _ => return Err(unsupported()),
            }
        }
        let idx = idx.ok_or_else(unsupported)?;
        if idx >= len {
            return Err(unsupported());
        }
        let hide = match (delete, show_val) {
            (Some(true), None | Some(false)) => true,
            (Some(false), None | Some(true)) => false,
            (None, Some(false)) => true,
            (None, Some(true)) => false,
            (None, None) | (Some(true), Some(true)) | (Some(false), Some(false)) => {
                return Err(unsupported());
            }
        };
        if hide {
            deleted.push(idx);
        } else {
            shown.push(idx);
        }
    }
    // Keep input-sized work O(n log n), including duplicate validation and
    // point lookup during painting; do not add a quadratic resource path.
    deleted.sort_unstable();
    shown.sort_unstable();
    if deleted.windows(2).any(|pair| pair[0] == pair[1])
        || shown.windows(2).any(|pair| pair[0] == pair[1])
    {
        return Err(unsupported());
    }
    Ok(PointLabelVisibility { deleted, shown })
}

/// Only these non-bubble chart kinds reach this parser. Excel writes
/// showBubbleSize=1 on ordinary line/column wrappers; it has no visible meaning
/// there. Real label flags, custom formatting and visible point overrides remain guarded.
pub(super) fn read_value_label(
    wrapper: &Element,
    ser: &Element,
    kind: PlotKind,
) -> Result<Option<ValueLabel>, Error> {
    let unsupported = || Error::with_path(ErrorKind::ChartImageUnsupported, "id");
    let mut visible = false;
    let mut position = if kind == PlotKind::StandardLine {
        "r"
    } else {
        "outEnd"
    };
    let configs: Vec<_> = [
        (false, wrapper.child_by_local("dLbls")),
        (true, ser.child_by_local("dLbls")),
    ]
    .into_iter()
    .filter_map(|(series_level, labels)| labels.map(|labels| (series_level, labels)))
    .collect();
    for (series_level, labels) in &configs {
        if flag(labels, "delete").unwrap_or(false) {
            visible = false;
            continue;
        }
        if labels.element_children().any(|e| e.local_name() == "dLbl") {
            if !series_level {
                return Err(unsupported());
            }
            // Shape is checked here even for this parser's direct callers;
            // read_chart additionally checks indices against real values.
            read_deleted_label_points(Some(labels), usize::MAX)?;
        }
        for name in [
            "showLegendKey",
            "showCatName",
            "showSerName",
            "showPercent",
            "showLeaderLines",
        ] {
            if flag(labels, name).unwrap_or(false) {
                return Err(unsupported());
            }
        }
        if let Some(v) = flag(labels, "showVal") {
            visible = v;
        }
        if let Some(p) = labels
            .child_by_local("dLblPos")
            .and_then(|e| e.attr_local("val"))
        {
            position = p;
        }
    }
    if !visible {
        return Ok(None);
    }
    for (_, labels) in configs {
        // Hidden label styling cannot affect output. Visible custom styling is
        // not approximated by the default font/background.

        for child in labels.element_children() {
            match child.local_name() {
                "spPr" => {
                    for style in child.element_children() {
                        match style.local_name() {
                            "noFill" => {}
                            "ln" if style
                                .element_children()
                                .all(|e| e.local_name() == "noFill") => {}
                            "effectLst" if style.element_children().next().is_none() => {}
                            _ => return Err(unsupported()),
                        }
                    }
                }
                "numFmt" if child.attr_local("formatCode").unwrap_or("General") != "General" => {
                    return Err(unsupported())
                }
                "txPr" | "layout" => return Err(unsupported()),
                "extLst" => {
                    for ext in child.element_children() {
                        for setting in ext.element_children() {
                            if setting.local_name() != "showLeaderLines"
                                || !matches!(setting.attr_local("val"), Some("0" | "false"))
                            {
                                return Err(unsupported());
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
    if let Some(cache) = ser
        .child_by_local("val")
        .and_then(|v| find_deep(v, "numCache").or_else(|| find_deep(v, "numLit")))
    {
        if cache
            .child_by_local("formatCode")
            .is_some_and(|e| e.text_content() != "General")
        {
            return Err(unsupported());
        }
    }
    match (kind, position) {
        (PlotKind::ClusteredColumn | PlotKind::ClusteredBar, "outEnd") => {
            Ok(Some(ValueLabel::OutsideEnd))
        }
        (PlotKind::ClusteredColumn | PlotKind::ClusteredBar, "inEnd") => {
            Ok(Some(ValueLabel::InsideEnd))
        }
        (PlotKind::StandardLine, "r") => Ok(Some(ValueLabel::Right)),
        _ => Err(unsupported()),
    }
}

/// Excel16 public probes show #N/A is not a zero point and the line connects
/// across it (including zero/span); an actual blank follows dispBlanksAs.
/// Other explicit non-numeric errors remain distinguishable and are not silently
/// treated as blanks when opting into the newly supported zero/span modes.
pub(super) fn series_error_points(
    parent: Option<&Element>,
    sheet: &str,
    cells: Option<&super::super::sparkline::CellCache>,
    len: usize,
) -> (Vec<bool>, Vec<bool>) {
    let mut errors = vec![false; len];
    let mut unavailable = vec![false; len];
    let cache = parent
        .and_then(|p| find_deep(p, "numCache").or_else(|| find_deep(p, "numLit")))
        .filter(|_| !cache_numbers(parent).is_empty());
    if let Some(cache) = cache {
        for (idx, text) in read_pts(cache) {
            if let Some(error) = errors.get_mut(idx) {
                *error = parse_chart_number(text.trim()).is_none();
                unavailable[idx] = text.trim() == "#N/A";
            }
        }
    } else if let Some(values) = cells.and_then(|cells| {
        formula_text(parent).and_then(|f| super::super::sparkline::range_values(&f, sheet, cells))
    }) {
        for (idx, value) in values.into_iter().take(len).enumerate() {
            unavailable[idx] = value.as_str() == Some("#N/A");
            errors[idx] = !value.is_null() && json_number(value).is_none();
        }
    }
    (errors, unavailable)
}

pub(super) fn data_labels_hidden(dlbls: Option<&Element>) -> Result<(), Error> {
    let Some(dlbls) = dlbls else {
        return Ok(());
    };
    let unsupported = || Error::with_path(ErrorKind::ChartImageUnsupported, "id");
    if dlbls
        .element_children()
        .any(|child| child.local_name() == "dLbl")
    {
        return Err(unsupported());
    }
    for name in [
        "showLegendKey",
        "showVal",
        "showCatName",
        "showSerName",
        "showPercent",
        "showLeaderLines",
    ] {
        if flag(dlbls, name).unwrap_or(false) {
            return Err(unsupported());
        }
    }
    Ok(())
}

pub(super) fn flag(el: &Element, name: &str) -> Option<bool> {
    let child = el.child_by_local(name)?;
    Some(!matches!(
        child.attr_local("val"),
        Some("0" | "false" | "off")
    ))
}

pub(super) fn number_val(el: &Element, name: &str) -> Option<f64> {
    el.child_by_local(name)?.attr_local("val")?.parse().ok()
}

pub(super) fn find_deep<'a>(el: &'a Element, name: &str) -> Option<&'a Element> {
    if el.local_name() == name {
        return Some(el);
    }
    for child in el.element_children() {
        if let Some(found) = find_deep(child, name) {
            return Some(found);
        }
    }
    None
}

pub(super) fn join_text(el: &Element) -> String {
    let mut text = String::new();
    if el.local_name() == "t" {
        text.push_str(&el.text_content());
    }
    for child in el.element_children() {
        text.push_str(&join_text(child));
    }
    text
}
