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
        }
        canvas.plot_fill = if chart.kind == PlotKind::Pie {
            // A pie's automatic plot area is transparent. Explicit opaque solid
            // fill uses the same title/legend-aware plot rectangle as the pie,
            // before slices, never the full available host rectangle.
            match frame.plot_fill {
                FrameFill::NoFill | FrameFill::Default => None,
                FrameFill::Solid(color) => Some(PlotPaint::Solid(color, 255)),
                // Translucent and gradient pie plot fills need their own
                // fractional-edge compositing locks, not Cartesian assumptions.
                _ => return Err(Error::with_path(ErrorKind::ChartImageUnsupported, "id")),
            }
        } else {
            match frame.plot_fill {
                FrameFill::NoFill => None,
                FrameFill::Default => Some(PlotPaint::Solid([255; 3], 255)),
                FrameFill::Solid(color) => Some(PlotPaint::Solid(color, 255)),
                FrameFill::Translucent(color, alpha) => {
                    Some(PlotPaint::Solid(color, ooxml_alpha_to_u8(alpha)))
                }
                FrameFill::AlphaProduct(color, product) => {
                    Some(PlotPaint::Solid(color, alpha_product_to_u8(product)))
                }
                FrameFill::Linear(grad) => Some(PlotPaint::Linear(grad)),
            }
        };
    }
    paint(&mut canvas, &chart, font.as_ref(), host)?;
    canvas.encode_png()
}
