use std::collections::BTreeMap;

use crate::error::{Error, ErrorKind};
use crate::xml::Element;

use super::*;


/// Excel 导出带圆角外框，绘图区从标题布局顶起算。
/// 幻灯片导出没有这圈外框，有标题时绘图区再低一截。
#[derive(Clone, Copy)]
pub(crate) enum ChartHost {
    Excel,
    Slide,
}

pub(crate) fn render_chart(
    root: &Element,
    width: u32,
    height: u32,
    accents: [[u8; 3]; 6],
    font_bytes: Option<&[u8]>,
    host: ChartHost,
    cells: Option<&super::super::sparkline::CellCache>,
    sheet: &str,
) -> Result<Vec<u8>, Error> {
    render_chart_at(
        root,
        width,
        height,
        accents,
        font_bytes,
        host,
        cells,
        sheet,
        RenderDpi::DEFAULT,
    )
}

pub(crate) fn render_chart_at(
    root: &Element,
    width: u32,
    height: u32,
    accents: [[u8; 3]; 6],
    font_bytes: Option<&[u8]>,
    host: ChartHost,
    cells: Option<&super::super::sparkline::CellCache>,
    sheet: &str,
    dpi: RenderDpi,
) -> Result<Vec<u8>, Error> {
    render_chart_with_theme(
        root, width, height, accents, font_bytes, host, cells, sheet, dpi, None,
    )
}

pub(crate) fn render_chart_with_theme(
    root: &Element,
    width: u32,
    height: u32,
    accents: [[u8; 3]; 6],
    font_bytes: Option<&[u8]>,
    host: ChartHost,
    cells: Option<&super::super::sparkline::CellCache>,
    sheet: &str,
    dpi: RenderDpi,
    theme_xml: Option<&[u8]>,
) -> Result<Vec<u8>, Error> {
    render_chart_with_sys_palette(
        root,
        width,
        height,
        accents,
        font_bytes,
        host,
        cells,
        sheet,
        dpi,
        theme_xml,
        None,
    )
}

pub(crate) fn render_chart_with_sys_palette(
    root: &Element,
    width: u32,
    height: u32,
    accents: [[u8; 3]; 6],
    font_bytes: Option<&[u8]>,
    host: ChartHost,
    cells: Option<&super::super::sparkline::CellCache>,
    sheet: &str,
    dpi: RenderDpi,
    theme_xml: Option<&[u8]>,
    sys: Option<&SystemColorPalette>,
) -> Result<Vec<u8>, Error> {
    render_chart_with_media(
        root, width, height, accents, font_bytes, host, cells, sheet, dpi, theme_xml, sys, None,
    )
}

pub(crate) fn render_chart_with_media(
    root: &Element,
    width: u32,
    height: u32,
    accents: [[u8; 3]; 6],
    font_bytes: Option<&[u8]>,
    host: ChartHost,
    cells: Option<&super::super::sparkline::CellCache>,
    sheet: &str,
    dpi: RenderDpi,
    theme_xml: Option<&[u8]>,
    sys: Option<&SystemColorPalette>,
    blips: Option<&BTreeMap<String, Vec<u8>>>,
) -> Result<Vec<u8>, Error> {
    if width == 0 || height == 0 || width > MAX_PX || height > MAX_PX {
        return Err(Error::with_path(ErrorKind::ChartImageUnsupported, "id"));
    }
    let mut scheme = theme_scheme(theme_xml, accents);
    if let Some(palette) = sys {
        scheme.sys = palette.clone();
    }
    let frame = if matches!(host, ChartHost::Excel) {
        Some(read_frame_with(root, &scheme)?)
    } else {
        None
    };
    let chart = read_chart(root, accents, cells, sheet, dpi)?;
    let font = font_bytes.and_then(|bytes| Font::from_bytes(bytes, FontSettings::default()).ok());
    let mut canvas = Canvas::with_dpi(width, height, dpi);
    if let Some(map) = blips {
        canvas.blips = map.clone();
    }
    if let Some(frame) = frame {
        // Chart area and plot area have independent fills. The default Cartesian
        // plot is white, not the explicitly coloured chart-area background.
        canvas.rounded_frame = frame.rounded;
        canvas.chart_border = frame.border;
        match frame.fill {
            FrameFill::NoFill => canvas.clear_to_transparent(),
            FrameFill::Default => canvas.fill_rect(0, 0, width as i32, height as i32, [255; 3]),
            FrameFill::Solid(color) => canvas.fill_rect(0, 0, width as i32, height as i32, color),
            FrameFill::Translucent(color, alpha) => {
                // Translucent chart fill is straight alpha, not flattened onto white.
                canvas.clear_to_transparent();
                let cov = ooxml_alpha_to_u8(alpha);
                if cov > 0 {
                    canvas.fill_rect_cov(0, 0, width as i32, height as i32, color, cov);
                }
            }
            FrameFill::AlphaProduct(color, product) => {
                canvas.clear_to_transparent();
                let cov = alpha_product_to_u8(product);
                if cov > 0 {
                    canvas.fill_rect_cov(0, 0, width as i32, height as i32, color, cov);
                }
            }
            FrameFill::Linear(grad) => {
                canvas.fill_linear_rect(0, 0, width as i32, height as i32, &grad)
            }
            FrameFill::Pattern(patt) => {
                canvas.fill_pattern_rect(0, 0, width as i32, height as i32, &patt)
            }
            FrameFill::Blip(blip) => {
                canvas.paint_blip_rect(0, 0, width as i32, height as i32, &blip)?
            }
        }
        canvas.plot_fill = match (chart.kind == PlotKind::Pie, frame.plot_fill) {
            // A pie's automatic plot area is transparent. Explicit fills use
            // the same title/legend-aware plot rectangle as the pie, before
            // slices, never the full available host rectangle.
            (true, FrameFill::NoFill | FrameFill::Default) => None,
            (true, fill) => Some(plot_paint(fill)?),
            (false, FrameFill::NoFill) => None,
            (false, FrameFill::Default) => Some(PlotPaint::Solid([255; 3], 255)),
            (false, fill) => Some(plot_paint(fill)?),
        };
    }
    paint(&mut canvas, &chart, font.as_ref(), host)?;
    canvas.encode_png()
}


fn plot_paint(fill: FrameFill) -> Result<PlotPaint, Error> {
    Ok(match fill {
        FrameFill::NoFill | FrameFill::Default => {
            return Err(Error::with_path(ErrorKind::ChartImageUnsupported, "id"))
        }
        FrameFill::Solid(color) => PlotPaint::Solid(color, 255),
        FrameFill::Translucent(color, alpha) => PlotPaint::Solid(color, ooxml_alpha_to_u8(alpha)),
        FrameFill::AlphaProduct(color, product) => {
            PlotPaint::Solid(color, alpha_product_to_u8(product))
        }
        FrameFill::Linear(grad) => PlotPaint::Linear(grad),
        FrameFill::Pattern(patt) => PlotPaint::Pattern(patt),
        FrameFill::Blip(blip) => PlotPaint::Blip(blip),
    })
}
