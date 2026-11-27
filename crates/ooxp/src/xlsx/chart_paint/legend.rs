//! Legend semantics live separately from chart axes. Dimensions follow the export DPI; 96 is the historical default.
use super::*;

#[derive(Clone, PartialEq)]
struct TextStyle {
    name: String,
    pt: f64,
    bold: bool,
    italic: bool,
    color: [u8; 3],
}
impl Default for TextStyle {
    fn default() -> Self {
        Self {
            name: String::new(),
            pt: LABEL_PT,
            bold: false,
            italic: false,
            color: INK,
        }
    }
}
#[derive(Clone, Copy)]
struct Coordinate {
    value: f64,
    edge: bool,
}
#[derive(Clone, Copy)]
struct Manual {
    x: Coordinate,
    y: Coordinate,
    w: Option<f64>,
    h: Option<f64>,
    w_edge: bool,
    h_edge: bool,
    /// `layoutTarget=inner` keeps chart-space manual coordinates but does
    /// not apply the automatic pos reservation to the plot. Not an outer no-op.
    inner: bool,
}
pub(super) struct Spec {
    pub(super) legacy: bool,
    pos: Option<LegendPos>,
    style: TextStyle,
    overlay: bool,
    manual: Option<Manual>,
    deleted: Vec<usize>,
    entries: Vec<(usize, TextStyle)>,
    background: FrameFill,
    border: Option<([u8; 3], f64)>,
    shadow: Option<OuterShadow>,
}
pub(super) fn unsupported() -> Error {
    Error::with_path(ErrorKind::ChartImageUnsupported, "id")
}
fn bool_attr(v: Option<&str>, default: bool) -> bool {
    match v.map(str::trim) {
        Some("1" | "true" | "on") => true,
        Some("0" | "false" | "off") => false,
        _ => default,
    }
}
fn number(el: &Element, name: &str) -> Option<f64> {
    el.child_by_local(name)?
        .attr_local("val")?
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
}
fn unhandled_attrs(el: &Element, allowed: &[&str]) -> bool {
    el.attrs.iter().any(|(key, _)| {
        !key.starts_with("xmlns") && !allowed.contains(&key.rsplit(':').next().unwrap_or(key))
    })
}
fn coord(el: &Element, name: &str, mode: &str) -> Coordinate {
    Coordinate {
        value: number(el, name).unwrap_or(0.0),
        edge: number(el, name).is_some()
            && el.child_by_local(mode).and_then(|n| n.attr_local("val")) == Some("edge"),
    }
}
impl Spec {
    // Overlay changes plot reservation, not default legend keys or text packing.
    // Keep the locked default-right ink independent of that layout-only flag.
    pub(super) fn default_right_ink(&self) -> bool {
        self.pos == Some(LegendPos::Right)
            && self.style == TextStyle::default()
            && self.manual.is_none()
            && self.deleted.is_empty()
            && self.entries.is_empty()
            && self.background == FrameFill::NoFill
            && self.border.is_none()
            && self.shadow.is_none()
    }

    /// Official-like automatic pie legend: overlay 0, no manual layout, and
    /// the same default typography/frame subset as `default_right_ink`.
    /// Right stays on the existing path; l/t/b/tr are the F39 close-enough
    /// positions. Not a unique official-matrix lock of every layout.
    pub(super) fn official_like_pie_auto_legend(&self) -> bool {
        !self.overlay
            && self.style == TextStyle::default()
            && self.manual.is_none()
            && self.deleted.is_empty()
            && self.entries.is_empty()
            && self.background == FrameFill::NoFill
            && self.border.is_none()
            && self.shadow.is_none()
            && matches!(
                self.pos,
                Some(
                    LegendPos::Right
                        | LegendPos::Left
                        | LegendPos::Top
                        | LegendPos::Bottom
                        | LegendPos::TopRight
                )
            )
    }
    // Entry-only paint changes do not change the inherited font metrics.
    // Controlled Excel exports confirm that the existing default-right geometry
    // stays fixed. Keep default_right_ink strict for the separate pie boundary.
    pub(super) fn default_right_geometry(&self) -> bool {
        self.pos == Some(LegendPos::Right)
            && self.style == TextStyle::default()
            && self.manual.is_none()
            && self.deleted.is_empty()
            && self.background == FrameFill::NoFill
            && self.border.is_none()
            && self.shadow.is_none()
            && self.entries.iter().all(|(_, style)| {
                let mut metrics = style.clone();
                metrics.color = self.style.color;
                metrics == self.style
            })
    }
    pub(super) fn entry_text_color(&self, index: usize) -> [u8; 3] {
        self.entries
            .iter()
            .find(|(i, _)| *i == index)
            .map(|(_, style)| style.color)
            .unwrap_or(self.style.color)
    }
    pub(super) fn read(root: &Element, accents: [[u8; 3]; 6]) -> Result<Self, Error> {
        let chart = root.child_by_local("chart").ok_or_else(unsupported)?;
        let pos = legend_pos(chart);
        let mut result = Self {
            legacy: true,
            pos,
            style: TextStyle::default(),
            overlay: true,
            manual: None,
            deleted: Vec::new(),
            entries: Vec::new(),
            background: FrameFill::NoFill,
            border: None,
            shadow: None,
        };
        let Some(legend) = chart.child_by_local("legend").filter(|_| pos.is_some()) else {
            return Ok(result);
        };
        for tx in [root.child_by_local("txPr"), legend.child_by_local("txPr")]
            .into_iter()
            .flatten()
        {
            result.style.read(tx, accents)?;
        }
        // Excel 16 public-API matrix (all five legendPos values): absence,
        // <overlay/>, and explicit 1 are equivalent. Only explicit 0 reserves.
        // This is an element default, not just CT_Boolean's attribute default.
        result.overlay = legend
            .child_by_local("overlay")
            .is_none_or(|n| bool_attr(n.attr_local("val"), true));
        if let Some(layout) = legend
            .child_by_local("layout")
            .and_then(|n| n.child_by_local("manualLayout"))
            .filter(|n| {
                ["x", "y", "w", "h"].iter().any(|key| {
                    number(n, key).is_some_and(|v| !matches!(*key, "w" | "h") || v > 0.0)
                })
            })
        {
            let target = layout
                .child_by_local("layoutTarget")
                .and_then(|n| n.attr_local("val"));
            if let Some(val) = target {
                if val != "inner" && val != "outer" {
                    return Err(unsupported());
                }
            }
            let mode_edge = |name: &str| -> Result<bool, Error> {
                match layout
                    .child_by_local(name)
                    .and_then(|n| n.attr_local("val"))
                {
                    None | Some("factor") => Ok(false),
                    Some("edge") => Ok(true),
                    Some(_) => Err(unsupported()),
                }
            };
            let size = |key| number(layout, key).filter(|v| *v > 0.0);
            result.manual = Some(Manual {
                x: coord(layout, "x", "xMode"),
                y: coord(layout, "y", "yMode"),
                w: size("w"),
                h: size("h"),
                w_edge: mode_edge("wMode")?,
                h_edge: mode_edge("hMode")?,
                inner: target == Some("inner"),
            });
        }
        for entry in legend.children_by_local("legendEntry") {
            let Some(index) = entry
                .child_by_local("idx")
                .and_then(|n| n.attr_local("val"))
                .and_then(|v| v.parse().ok())
            else {
                continue;
            };
            if entry
                .child_by_local("delete")
                .is_some_and(|n| bool_attr(n.attr_local("val"), false))
            {
                result.deleted.push(index);
            } else if let Some(tx) = entry.child_by_local("txPr") {
                if result.entries.iter().any(|(idx, _)| *idx == index) {
                    return Err(unsupported());
                }
                let mut style = result.style.clone();
                style.read(tx, accents)?;
                result.entries.push((index, style));
            }
        }
        if let Some(sp) = legend.child_by_local("spPr") {
            let mut saw_fill = false;
            let mut saw_line = false;
            for child in sp.element_children() {
                match child.local_name() {
                    "noFill" | "solidFill" if !saw_fill => {
                        saw_fill = true;
                        result.background = if child.local_name() == "noFill" {
                            FrameFill::NoFill
                        } else {
                            legend_solid(child, accents)?
                        };
                    }
                    "gradFill" if !saw_fill => {
                        saw_fill = true;
                        result.background =
                            read_linear_frame_grad(child, &theme_scheme(None, accents))?;
                    }
                    "extLst" => (),
                    "effectDag" if child.element_children().next().is_none() => (),
                    "effectLst" => {
                        let mut saw_shadow = false;
                        for effect in child.element_children() {
                            if effect.local_name() != "outerShdw" || saw_shadow {
                                return Err(unsupported());
                            }
                            saw_shadow = true;
                            let shadow = read_outer_shadow_effect(
                                effect,
                                &theme_scheme(None, accents),
                            );
                            // Zero dist and blur have no visible offset or
                            // softness; keep the default-right ink path.
                            if shadow.blur_emu > 0.0 || shadow.dist_emu > 0.0 {
                                result.shadow = Some(shadow);
                            }
                        }
                    }
                    "ln" if !saw_line => {
                        saw_line = true;
                        if child.child_by_local("noFill").is_some() {
                            continue;
                        }
                        if unhandled_attrs(child, &["w"]) {
                            return Err(unsupported());
                        }
                        let width = child
                            .attr_local("w")
                            .and_then(|v| v.parse::<u64>().ok())
                            .filter(|v| *v > 0)
                            .ok_or_else(unsupported)?;
                        let mut color = None;
                        for part in child.element_children() {
                            match part.local_name() {
                                "solidFill" if color.is_none() => {
                                    let FrameFill::Solid(rgb) = legend_solid(part, accents)? else {
                                        return Err(unsupported());
                                    };
                                    color = Some(rgb);
                                }
                                "prstDash" if part.attr_local("val") == Some("solid") => (),
                                _ => return Err(unsupported()),
                            }
                        }
                        result.border =
                            Some((color.ok_or_else(unsupported)?, width as f64 / 12_700.0));
                    }
                    _ => return Err(unsupported()),
                }
            }
        }
        result.legacy = result.default_right_geometry() && !result.overlay;
        Ok(result)
    }
}
// Reuse the already tested sRGB/straight-alpha conversion for legend solid
// fills. Official-like linear gradFill and outerShdw use the shared frame
// helpers. Other fills and effects remain NI.
fn legend_solid(solid: &Element, accents: [[u8; 3]; 6]) -> Result<FrameFill, Error> {
    let colors: Vec<_> = solid.element_children().collect();
    if colors.len() != 1 {
        return Err(unsupported());
    }
    let color = colors[0];
    if color.local_name() != "srgbClr"
        || unhandled_attrs(color, &["val"])
        || color.element_children().any(|n| n.local_name() != "alpha")
    {
        return Err(unsupported());
    }
    resolve_frame_solid(color, &theme_scheme(None, accents))
}

impl TextStyle {
    fn read(&mut self, tx: &Element, accents: [[u8; 3]; 6]) -> Result<(), Error> {
        if let Some(body) = tx.child_by_local("bodyPr") {
            if unhandled_attrs(body, &["rot", "vert"])
                || body
                    .element_children()
                    .any(|n| !matches!(n.local_name(), "noAutofit" | "extLst"))
            {
                return Err(unsupported());
            }
            if body
                .attr_local("rot")
                .and_then(|n| n.parse::<i64>().ok())
                .is_some_and(|n| n != 0)
                || body.attr_local("vert").is_some_and(|v| v != "horz")
            {
                return Err(unsupported());
            }
        }
        if tx
            .child_by_local("lstStyle")
            .is_some_and(|n| n.element_children().next().is_some())
        {
            return Err(unsupported());
        }
        let paragraphs = tx.children_by_local("p");
        if paragraphs.len() > 1 {
            return Err(unsupported());
        }
        for p in paragraphs {
            if p.element_children()
                .any(|n| !matches!(n.local_name(), "pPr" | "endParaRPr" | "extLst"))
            {
                return Err(unsupported());
            }
            if let Some(pp) = p.child_by_local("pPr") {
                if unhandled_attrs(pp, &["algn", "rtl", "lvl", "fontAlgn"])
                    || pp.attr_local("algn").is_some_and(|v| v != "l")
                    || bool_attr(pp.attr_local("rtl"), false)
                    || pp
                        .attr_local("lvl")
                        .and_then(|v| v.parse::<u32>().ok())
                        .is_some_and(|v| v != 0)
                    || pp.attr_local("fontAlgn").is_some_and(|v| v != "base")
                    || pp
                        .element_children()
                        .any(|n| !matches!(n.local_name(), "defRPr" | "extLst"))
                {
                    return Err(unsupported());
                }
            }
            if let Some(props) = p
                .child_by_local("pPr")
                .and_then(|n| n.child_by_local("defRPr"))
                .or_else(|| p.child_by_local("endParaRPr"))
            {
                self.props(props, accents)?;
            }
        }
        Ok(())
    }
    fn props(&mut self, p: &Element, accents: [[u8; 3]; 6]) -> Result<(), Error> {
        if unhandled_attrs(
            p,
            &[
                "sz",
                "b",
                "i",
                "u",
                "strike",
                "baseline",
                "spc",
                "cap",
                "normalizeH",
                "kumimoji",
                "lang",
                "altLang",
                "dirty",
                "err",
                "smtClean",
                "smtId",
                "noProof",
                "bmk",
            ],
        ) {
            return Err(unsupported());
        }
        if let Some(pt) = p
            .attr_local("sz")
            .and_then(|v| v.parse::<f64>().ok())
            .filter(|v| v.is_finite() && (100.0..=400000.0).contains(v))
        {
            if pt > 40900.0 {
                return Err(unsupported());
            }
            self.pt = pt / 100.0;
        }
        if p.attr_local("cap").is_some_and(|v| v != "none")
            || p.attr_local("spc")
                .and_then(|v| v.parse::<i32>().ok())
                .is_some_and(|v| v != 0)
            || bool_attr(p.attr_local("normalizeH"), false)
            || bool_attr(p.attr_local("kumimoji"), false)
        {
            return Err(unsupported());
        }
        self.bold = bool_attr(p.attr_local("b"), self.bold);
        self.italic = bool_attr(p.attr_local("i"), self.italic);
        if p.attr_local("u").is_some_and(|v| v != "none")
            || p.attr_local("strike").is_some_and(|v| v != "noStrike")
            || p.attr_local("baseline")
                .and_then(|v| v.parse::<i32>().ok())
                .is_some_and(|v| v != 0)
        {
            return Err(unsupported());
        }
        let latin = p
            .child_by_local("latin")
            .and_then(|n| n.attr_local("typeface"))
            .filter(|n| !n.trim().is_empty());
        let ea = p
            .child_by_local("ea")
            .and_then(|n| n.attr_local("typeface"))
            .filter(|n| !n.trim().is_empty());
        if matches!((latin, ea), (Some(a), Some(b)) if a != b) {
            return Err(unsupported());
        }
        if let Some(name) = latin.or(ea) {
            if name.starts_with('+') {
                return Err(unsupported());
            }
            self.name = name.to_string();
        }
        for fill in p.element_children() {
            match fill.local_name() {
                "gradFill" | "pattFill" | "blipFill" | "noFill" => return Err(unsupported()),
                "effectLst" | "effectDag" if fill.element_children().next().is_some() => {
                    return Err(unsupported())
                }
                "solidFill" => {
                    let Some(color) = fill.element_children().next() else {
                        continue;
                    };
                    if color.local_name() == "srgbClr"
                        && color.attr_local("val").is_none_or(|v| {
                            v.len() != 6 || !v.bytes().all(|b| b.is_ascii_hexdigit())
                        })
                    {
                        continue;
                    }
                    if color.local_name() == "schemeClr"
                        && !matches!(
                            color.attr_local("val"),
                            Some(
                                "accent1"
                                    | "accent2"
                                    | "accent3"
                                    | "accent4"
                                    | "accent5"
                                    | "accent6"
                            )
                        )
                    {
                        return Err(unsupported());
                    }
                    if color.element_children().any(|n| {
                        !matches!(
                            n.local_name(),
                            "lumMod" | "lumOff" | "shade" | "tint" | "alpha"
                        )
                    }) {
                        return Err(unsupported());
                    }
                    if find_deep(fill, "alpha")
                        .and_then(|n| n.attr_local("val"))
                        .is_some_and(|v| v != "100000")
                    {
                        return Err(unsupported());
                    }
                    if let Some(rgb) = color_of_slot(fill, &accents) {
                        self.color = rgb;
                    } else if fill.child_by_local("srgbClr").is_none() {
                        return Err(unsupported());
                    }
                }
                "cs" if fill
                    .attr_local("typeface")
                    .is_some_and(|name| Some(name) == latin.or(ea))
                    && !unhandled_attrs(fill, &["typeface"]) =>
                {
                    ()
                }
                "latin" | "ea" | "extLst" => (),
                "effectLst" | "effectDag" if fill.element_children().next().is_none() => (),
                _ => return Err(unsupported()),
            }
        }
        Ok(())
    }
}
fn style_face(
    style: &TextStyle,
    px: f32,
    dpi: RenderDpi,
) -> Result<Option<super::super::chart_gdi::Face>, Error> {
    #[cfg(not(windows))]
    if style.bold || style.italic || !style.name.is_empty() {
        return Err(unsupported());
    }
    let face = if style.name.is_empty() {
        let face = super::super::chart_gdi::Face::open_dpi(px, style.bold, dpi.get());
        #[cfg(windows)]
        let face = if style.italic {
            face.and_then(|f| {
                super::super::chart_gdi::Face::open_cell_dpi(
                    px,
                    style.bold,
                    true,
                    f.family_name(),
                    dpi.get(),
                )
            })
        } else {
            face
        };
        face
    } else {
        let face = super::super::chart_gdi::Face::open_cell_dpi(
            px,
            style.bold,
            style.italic,
            &style.name,
            dpi.get(),
        );
        #[cfg(windows)]
        if face
            .as_ref()
            .is_none_or(|f| !same_name(f.family_name(), &style.name))
        {
            return Err(unsupported());
        }
        face
    };
    Ok(face)
}

struct Item<'a> {
    series: &'a Series,
    width: f64,
    style: TextStyle,
    face: Option<super::super::chart_gdi::Face>,
    px: f32,
    line: f64,
    space: f64,
    cell: f64,
}
pub(super) struct Layout<'a> {
    spec: &'a Spec,
    items: Vec<Item<'a>>,
    key: f64,
    space: f64,
    line: f64,
    pitch: f64,
    reservation_width: f64,
    frame: Rect,
    auto: Rect,
    horizontal: bool,
    rows: Vec<Vec<usize>>,
    dpi: RenderDpi,
}
impl<'a> Layout<'a> {
    pub(super) fn new(
        canvas: &Canvas,
        chart: &'a ColumnChart,
        font: Option<&Font>,
    ) -> Result<Self, Error> {
        let spec = &chart.legend_spec;
        let px = canvas.dpi.pt_to_px_font(spec.style.pt) as f32;
        if spec
            .entries
            .iter()
            .any(|(idx, _)| *idx >= chart.series.len())
        {
            return Err(unsupported());
        }
        let face = style_face(&spec.style, px, canvas.dpi)?;
        let space = text_width(font, face.as_ref(), " ", px).ceil().max(1.0);
        let line = face
            .as_ref()
            .map(|f| f.em_box())
            .filter(|v| *v > 0.0)
            .unwrap_or_else(|| text_height(font, px))
            .max(f64::from(px));
        let mut items = Vec::new();
        if spec.pos.is_some() {
            for (idx, series) in chart.series.iter().enumerate() {
                if spec.deleted.contains(&idx) {
                    continue;
                }
                let style = spec
                    .entries
                    .iter()
                    .find(|(i, _)| *i == idx)
                    .map(|(_, style)| style)
                    .unwrap_or(&spec.style)
                    .clone();
                let item_px = canvas.dpi.pt_to_px_font(style.pt) as f32;
                let item_face = style_face(&style, item_px, canvas.dpi)?;
                let item_space = text_width(font, item_face.as_ref(), " ", item_px)
                    .ceil()
                    .max(1.0);
                let item_line = item_face
                    .as_ref()
                    .map(|f| f.em_box())
                    .filter(|v| *v > 0.0)
                    .unwrap_or_else(|| text_height(font, item_px))
                    .max(f64::from(item_px));
                let width = item_face
                    .as_ref()
                    .map(|f| f.typo_width(&series.name))
                    .unwrap_or_else(|| text_width(font, None, &series.name, item_px));
                let item_face_cell = item_face
                    .as_ref()
                    .and_then(|f| f.cell_height())
                    .filter(|v| v.is_finite() && *v > 0.0)
                    .unwrap_or(item_line);
                items.push(Item {
                    series,
                    width,
                    style,
                    face: item_face,
                    px: item_px,
                    line: item_line,
                    space: item_space,
                    cell: item_face_cell,
                });
            }
        }
        // Controlled family/point matrices: key size follows 45% of the
        // design cell, and rows add a 6pt gutter to that cell. Do not use a
        // family lookup, COM bounds, or a fitted device-coordinate correction.
        let base_cell = face
            .as_ref()
            .and_then(|f| f.cell_height())
            .filter(|v| v.is_finite() && *v > 0.0)
            .unwrap_or(line);
        let cell = items.iter().map(|i| i.cell).fold(base_cell, f64::max);
        // An omitted family still depends on unresolved chart/theme defaults;
        // preserve its established point-based model rather than silently
        // substituting the locally selected fallback face for native Excel.
        let (key, pitch) = if !spec.style.name.is_empty() {
            ((cell * 0.45).max(1.0), cell + canvas.dpi.pt_to_px_font(6.0))
        } else {
            let max_pt = items
                .iter()
                .map(|i| i.style.pt)
                .fold(spec.style.pt, f64::max);
            let row_px = canvas.dpi.pt_to_px_font(max_pt);
            (
                (row_px * 0.5).ceil().max(1.0),
                row_px + canvas.dpi.pt_to_px_font(8.0),
            )
        };
        for item in &mut items {
            item.width +=
                super::legend_key_width(chart.kind, key.round() as i32, canvas.dpi) as f64;
        }
        let line = items.iter().map(|i| i.line).fold(line, f64::max);
        let space = items.iter().map(|i| i.space).fold(space, f64::max);
        if !spec.legacy && !items.is_empty() && face.is_none() && font.is_none() {
            return Err(unsupported());
        }
        let horizontal =
            spec.manual.is_some() || matches!(spec.pos, Some(LegendPos::Top | LegendPos::Bottom));
        let max_w = f64::from(canvas.w) - 2.0 * space;
        let max_item = items.iter().map(|i| i.width).fold(0.0, f64::max);
        if !spec.legacy && max_item > max_w {
            return Err(unsupported());
        }
        let auto_horizontal = matches!(spec.pos, Some(LegendPos::Top | LegendPos::Bottom));
        let natural_w = if auto_horizontal {
            (items.iter().map(|i| i.width).sum::<f64>()
                + space * 2.0 * items.len().saturating_sub(1) as f64)
                .min(max_w)
        } else {
            max_item
        } + space * 2.0;
        // Plot reservation uses the padded font layout box. Ink packing above
        // uses typographic advances; conflating the two shrinks a left legend's
        // reserved band when its text has GDI overhang padding.
        let reservation_width = items
            .iter()
            .map(|item| {
                item.face
                    .as_ref()
                    .map(|f| f.layout_width(&item.series.name))
                    .unwrap_or_else(|| text_width(font, None, &item.series.name, item.px))
            })
            .fold(0.0, f64::max)
            + key
            + 3.0 * space;
        let natural_h = pitch
            * if auto_horizontal {
                1.0
            } else {
                items.len() as f64
            };
        let w = f64::from(canvas.w);
        let h = f64::from(canvas.h);
        let mut auto = position(
            spec.pos,
            w,
            h,
            natural_w,
            natural_h,
            canvas.dpi.pt_to_px_font(8.0),
            canvas.dpi.pt_to_px_font(6.0),
        );
        if chart.title.is_some() {
            match spec.pos {
                Some(LegendPos::Top | LegendPos::TopRight) => {
                    auto.y += f64::from(canvas.sx(LEGEND_TITLE_BAND))
                }
                Some(LegendPos::Right | LegendPos::Left) => {
                    auto.y += f64::from(canvas.sx(LEGEND_TITLE_BAND)) / 2.0
                }
                _ => (),
            }
        }
        let mut frame = Rect {
            x: auto.x,
            y: auto.y,
            w: auto.w,
            h: auto.h,
        };
        if let Some(m) = spec.manual {
            // Chart-space coordinates for outer and inner. inner does not
            // re-originate into the plot rectangle. wMode/hMode=edge are the
            // right/bottom edge in that same space, applied after left/top.
            // Degenerate edge sizes (right<=left or bottom<=top) stay
            // exportable with no painted legend; they are not a factor fallback
            // and not a guessed minimum box.
            let factor_w = m.w.map(|v| v * w).unwrap_or(auto.w);
            let factor_h = m.h.map(|v| v * h).unwrap_or(auto.h);
            frame.x = if m.x.edge {
                m.x.value * w
            } else {
                auto.x + auto.w / 2.0 - factor_w / 2.0 + m.x.value * w
            };
            frame.y = if m.y.edge {
                m.y.value * h
            } else {
                auto.y + auto.h / 2.0 - factor_h / 2.0 + m.y.value * h
            };
            frame.w = if m.w_edge {
                m.w.map(|v| v * w - frame.x).unwrap_or(auto.w)
            } else {
                factor_w
            };
            frame.h = if m.h_edge {
                m.h.map(|v| v * h - frame.y).unwrap_or(auto.h)
            } else {
                factor_h
            };
            let degenerate = frame.w <= 0.0 || frame.h <= 0.0;
            if degenerate {
                frame.w = 0.0;
                frame.h = 0.0;
            } else if !m.w_edge && !m.h_edge && (frame.w > w || frame.h > h) {
                return Err(unsupported());
            }
            // Manual frame uses chart-space coordinates, not the COM
            // Legend.Left/Top rectangle (whose public bounds omit 4pt padding).
            // Edge can deliberately clip outside; factor is automatically bounded.
            if !degenerate && !m.x.edge {
                frame.x = frame.x.min(w - frame.w);
            }
            if !degenerate && !m.y.edge {
                frame.y = frame.y.min(h - frame.h);
            }
        }
        let mut result = Self {
            spec,
            items,
            key,
            space,
            line,
            pitch,
            reservation_width,
            frame,
            auto,
            horizontal,
            rows: Vec::new(),
            dpi: canvas.dpi,
        };
        result.flow()?;
        if !spec.legacy
            && spec.manual.is_none()
            && (result.frame.w > w
                || result.frame.x < 0.0
                || result.frame.x + result.frame.w > w + 1e-6
                || result.frame.h > h
                || result.frame.y < 0.0
                || result.frame.y + result.frame.h > h + 1e-6)
        {
            return Err(unsupported());
        }
        Ok(result)
    }
    fn flow(&mut self) -> Result<(), Error> {
        if self.spec.legacy || self.items.is_empty() || self.frame.w <= 0.0 || self.frame.h <= 0.0
        {
            return Ok(());
        }
        let mut row = Vec::new();
        let mut used = 0.0;
        for (idx, item) in self.items.iter().enumerate() {
            if item.width + self.space * 2.0 > self.frame.w {
                if self.spec.manual.is_none() {
                    return Err(unsupported());
                }
            }
            if !row.is_empty()
                && (!self.horizontal
                    || used + self.space * 2.0 + item.width > self.frame.w - self.space * 2.0)
            {
                self.rows.push(std::mem::take(&mut row));
                used = 0.0;
            }
            if !row.is_empty() {
                used += self.space * 2.0;
            }
            row.push(idx);
            used += item.width;
        }
        if !row.is_empty() {
            self.rows.push(row);
        }
        let required = self.rows.len() as f64 * self.pitch;
        if required > self.frame.h && self.spec.manual.is_none() {
            let delta = required + self.space * 2.0 - self.frame.h;
            self.frame.h += delta;
            self.auto.h = self.frame.h;
            if self.spec.pos == Some(LegendPos::Bottom) {
                self.frame.y -= delta;
                self.auto.y = self.frame.y;
            }
        }
        Ok(())
    }
    pub(super) fn reserve(&self) -> [i32; 4] {
        let mut r = [0; 4];
        if self.spec.overlay
            || self.items.is_empty()
            || self.spec.manual.as_ref().is_some_and(|m| m.inner)
        {
            return r;
        }
        match self.spec.pos {
            Some(LegendPos::Right | LegendPos::TopRight) => {
                r[2] = (self.auto.w + f64::from(self.dpi.scale_px(LEGEND_PLOT_GAP))
                    - f64::from(self.dpi.scale_px(INSET_TOP)))
                .ceil()
                .max(f64::from(
                    self.dpi.scale_px(INSET_RIGHT) - self.dpi.scale_px(INSET_TOP),
                )) as i32
            }
            Some(LegendPos::Left) => {
                r[0] = (self.reservation_width + self.space * 2.0).ceil() as i32
            }
            Some(LegendPos::Top) => r[1] = (self.auto.h + self.space * 2.0).ceil() as i32,
            Some(LegendPos::Bottom) => r[3] = (self.auto.h + self.space * 2.0).ceil() as i32 + 1,
            None => (),
        }
        if self.spec.pos == Some(LegendPos::TopRight) {
            // A non-overlay corner legend is a two-dimensional reservation,
            // not a synonym for right. Native Excel reserves its full top row
            // block and a 6pt trailing clearance below the plot.
            r[1] = self.auto.h.round() as i32;
            r[3] = self.dpi.pt_to_px_font(6.0).round() as i32;
        }
        r
    }
    pub(super) fn draw(&self, canvas: &mut Canvas, chart: &ColumnChart, font: Option<&Font>) -> Result<(),Error> {
        if self.items.is_empty() {
            return Ok(());
        }
        // The existing layout frame is the semantic frame, independent of glyph
        // ink bounds. Paint before keys/text, with no COM-origin compensation.
        let (x0, y0, x1, y1) = (
            self.frame.x.round() as i32,
            self.frame.y.round() as i32,
            (self.frame.x + self.frame.w).round() as i32,
            (self.frame.y + self.frame.h).round() as i32,
        );
        if let Some(shadow) = &self.spec.shadow {
            canvas.fill_outer_shadow_rect(x0, y0, x1, y1, shadow);
        }
        match self.spec.background {
            FrameFill::Solid(rgb) => canvas.fill_rect(x0, y0, x1, y1, rgb),
            FrameFill::Translucent(rgb, alpha) => {
                canvas.fill_rect_cov(x0, y0, x1, y1, rgb, ooxml_alpha_to_u8(alpha))
            }
            FrameFill::Linear(ref grad) => canvas.fill_linear_rect(x0, y0, x1, y1, grad),
            _ => (),
        }
        if let Some((rgb, pt)) = self.spec.border {
            let width = canvas.dpi.pt_to_px_font(pt);
            let (x, y, r, b) = (
                self.frame.x,
                self.frame.y,
                self.frame.x + self.frame.w,
                self.frame.y + self.frame.h,
            );
            for (xa, ya, xb, yb) in [(x, y, r, y), (r, y, r, b), (r, b, x, b), (x, b, x, y)] {
                canvas.stroke_segment(xa, ya, xb, yb, width, rgb);
            }
        }
        let height = self.rows.len() as f64 * self.line
            + self.rows.len().saturating_sub(1) as f64 * (self.pitch - self.line).max(0.0);
        let mut y = self.frame.y + (self.frame.h - height) / 2.0;
        for row in &self.rows {
            let items_width = row.iter().map(|i| self.items[*i].width).sum::<f64>();
            // A wide manual frame distributes spare space across the outer and
            // inter-entry gaps; packing the same narrow group in its centre
            // ignores the effective legend width (native 10/18pt probes).
            let gap = if self.spec.manual.is_some() && self.horizontal {
                ((self.frame.w - items_width) / (row.len() + 1) as f64).max(self.space * 2.0)
            } else {
                self.space * 2.0
            };
            let width = items_width + row.len().saturating_sub(1) as f64 * gap;
            // A vertical legend is one shared key/text column, not separately
            // centred rows. Mixed font widths must not stagger its keys.
            let column_width = self.items.iter().map(|item| item.width).fold(0.0, f64::max);
            let packed_width = if self.horizontal { width } else { column_width };
            let mut x = self.frame.x + (self.frame.w - packed_width) / 2.0;
            for idx in row {
                let item = &self.items[*idx];
                let ser = item.series;
                let ky = y + (self.line - self.key) / 2.0;
                super::draw_plot_key_styled(
                    canvas,
                    chart.kind,
                    x.round() as i32,
                    ky.round() as i32,
                    x.round() as i32
                        + super::legend_key_width(chart.kind, self.key.round() as i32, canvas.dpi),
                    (ky + self.key).round() as i32,
                    ser.color.unwrap_or(chart.accents[ser.slot % 6]),
                    ser.marker,
                    if ser.line_visible { super::stroke::Style::new(ser.line_width,ser.line_cap) } else { None },
                    ser.line_dash.as_deref(),
                )?;
                draw_text(
                    canvas,
                    font,
                    item.face.as_ref(),
                    &ser.name,
                    x + f64::from(super::legend_key_width(
                        chart.kind,
                        self.key.round() as i32,
                        canvas.dpi,
                    )) + if self.horizontal {
                        item.space
                    } else {
                        self.space
                    },
                    y + (self.line - item.line) / 2.0,
                    item.px,
                    item.style.color,
                );
                x += item.width + gap;
            }
            y += self.pitch;
        }
        Ok(())
    }
}
fn position(
    pos: Option<LegendPos>,
    w: f64,
    h: f64,
    width: f64,
    height: f64,
    side: f64,
    pad: f64,
) -> Rect {
    let (x, y) = match pos {
        Some(LegendPos::Left) => (side, (h - height) / 2.0),
        Some(LegendPos::Top) => ((w - width) / 2.0, pad),
        Some(LegendPos::Bottom) => ((w - width) / 2.0, h - pad - height),
        Some(LegendPos::TopRight) => (w - side - width, pad),
        _ => (w - side - width, (h - height) / 2.0),
    };
    Rect {
        x,
        y,
        w: width,
        h: height,
    }
}
#[cfg(windows)]
fn same_name(a: &str, b: &str) -> bool {
    let norm = |s: &str| {
        if s.eq_ignore_ascii_case("DengXian") || s == "等线" {
            "dengxian".to_string()
        } else {
            s.to_lowercase()
        }
    };
    norm(a) == norm(b)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn spec(legend: &str) -> Result<Spec, Error> {
        let doc = crate::xml::parse(
            format!("<chartSpace><chart>{legend}</chart></chartSpace>").as_bytes(),
        )
        .unwrap();
        Spec::read(&doc.root, FALLBACK_ACCENTS)
    }
    #[test]
    fn absent_off_and_malformed_optional_values_are_distinct() {
        assert!(spec("").unwrap().pos.is_none());
        assert!(spec("<legend><delete val='1'/></legend>")
            .unwrap()
            .pos
            .is_none());
        let s = spec("<legend><legendPos val='garbage'/><txPr><p><pPr><defRPr sz='NaN'/></pPr></p></txPr></legend>").unwrap();
        assert!(!s.legacy);
        assert!(spec("<legend><overlay val='0'/><layout><manualLayout><x val='NaN'/><w val='-1'/></manualLayout></layout></legend>").unwrap().legacy);
        for pos in ["l", "t", "b", "tr"] {
            assert!(
                !spec(&format!("<legend><legendPos val='{pos}'/></legend>"))
                    .unwrap()
                    .legacy
            );
        }
    }
    #[test]
    fn automatic_positions_cover_five_edges_without_inventing_a_float_enum() {
        let left = position(Some(LegendPos::Left), 600.0, 360.0, 40.0, 50.0, 12.0, 9.0);
        let right = position(Some(LegendPos::Right), 600.0, 360.0, 40.0, 50.0, 12.0, 9.0);
        let top = position(Some(LegendPos::Top), 600.0, 360.0, 80.0, 25.0, 12.0, 9.0);
        let bottom = position(Some(LegendPos::Bottom), 600.0, 360.0, 80.0, 25.0, 12.0, 9.0);
        let corner = position(
            Some(LegendPos::TopRight),
            600.0,
            360.0,
            40.0,
            50.0,
            12.0,
            9.0,
        );
        assert_eq!((left.x, left.y), (12.0, 155.0));
        assert_eq!((right.x, right.y), (548.0, 155.0));
        assert_eq!((top.x, top.y), (260.0, 9.0));
        assert_eq!((bottom.x, bottom.y), (260.0, 326.0));
        assert_eq!((corner.x, corner.y), (548.0, 9.0));
    }
    #[test]
    fn excel_overlay_matrix_absent_empty_and_true_match_but_false_reserves() {
        for pos in ["l", "r", "t", "b", "tr"] {
            for overlay in [
                "",
                "<overlay/>",
                "<overlay val='1'/>",
                "<overlay val='true'/>",
            ] {
                let s = spec(&format!(
                    "<legend><legendPos val='{pos}'/>{overlay}</legend>"
                ))
                .unwrap();
                assert!(s.overlay, "{pos} {overlay}");
                assert!(!s.legacy);
            }
            let s = spec(&format!(
                "<legend><legendPos val='{pos}'/><overlay val='0'/></legend>"
            ))
            .unwrap();
            assert!(!s.overlay);
            assert_eq!(s.legacy, pos == "r");
        }
    }
    #[test]
    fn styles_and_deleted_entries_are_not_lost() {
        let s = spec("<legend><txPr><p><pPr><defRPr sz='1800' b='1' i='1'><solidFill><srgbClr val='FF0000'/></solidFill><latin typeface='Arial'/></defRPr></pPr></p></txPr><legendEntry><idx val='0'/><delete val='1'/></legendEntry></legend>").unwrap();
        assert_eq!(s.style.pt, 18.0);
        assert!(s.style.bold && s.style.italic);
        assert_eq!(s.style.color, [255, 0, 0]);
        assert_eq!(s.deleted, [0]);
        assert!(spec("<legend><txPr><p><pPr><defRPr u='sng'/></pPr></p></txPr></legend>").is_err());
    }
    #[test]
    fn manual_modes_are_not_the_same_coordinate_system() {
        let s = spec("<legend><layout><manualLayout><xMode val='factor'/><yMode val='edge'/><x val='.5'/><y val='.1'/><w val='.4'/><h val='.3'/></manualLayout></layout></legend>").unwrap();
        let m = s.manual.unwrap();
        assert!(!m.x.edge && m.y.edge);
        assert_eq!(m.w, Some(0.4));
        assert_eq!(m.h, Some(0.3));
        let s = spec("<legend><layout><manualLayout><xMode val='edge'/><x val='NaN'/><yMode val='edge'/><w val='.4'/></manualLayout></layout></legend>").unwrap();
        let m = s.manual.unwrap();
        assert!(
            !m.x.edge && !m.y.edge,
            "missing or malformed coordinates retain automatic positioning"
        );
        let edge = spec(
            "<legend><layout><manualLayout><wMode val='edge'/><w val='.4'/></manualLayout></layout></legend>",
        )
        .unwrap();
        let m = edge.manual.unwrap();
        assert!(m.w_edge && !m.h_edge && !m.inner);
        assert_eq!(m.w, Some(0.4));
        let inner = spec(
            "<legend><layout><manualLayout><layoutTarget val='inner'/><xMode val='edge'/><yMode val='edge'/><x val='.1'/><y val='.2'/><w val='.4'/><h val='.3'/></manualLayout></layout></legend>",
        )
        .unwrap();
        let m = inner.manual.unwrap();
        assert!(m.inner && m.x.edge && m.y.edge && !m.w_edge && !m.h_edge);
        assert!(spec(
            "<legend><layout><manualLayout><layoutTarget val='other'/><w val='.4'/></manualLayout></layout></legend>"
        )
        .is_err());
    }

    #[test]
    fn official_like_legend_grad_fill_and_outer_shadow_parse() {
        let grad = spec(
            "<legend><spPr><gradFill flip='none' rotWithShape='1'><gsLst><gs pos='0'><srgbClr val='FF0000'/></gs><gs pos='100000'><srgbClr val='FFFFFF'/></gs></gsLst><lin ang='5400000' scaled='1'/><tileRect/></gradFill></spPr></legend>",
        )
        .unwrap();
        assert!(matches!(grad.background, FrameFill::Linear(ref g) if g.ang == 5_400_000));
        assert!(grad.shadow.is_none());
        let shadow = spec(
            "<legend><spPr><effectLst><outerShdw blurRad='50800' dist='71842' dir='2700013' rotWithShape='0'><srgbClr val='000000'><alpha val='60000'/></srgbClr></outerShdw></effectLst></spPr></legend>",
        )
        .unwrap();
        let sh = shadow.shadow.unwrap();
        assert_eq!(sh.color, [0, 0, 0]);
        assert_eq!(sh.blur_emu, 50_800.0);
        assert_eq!(sh.dist_emu, 71_842.0);
        assert_eq!(sh.dir, 2_700_013);
        assert!(spec("<legend><spPr><effectLst><innerShdw/></effectLst></spPr></legend>").is_err());
        assert!(spec("<legend><spPr><gradFill/></spPr></legend>").is_err());
    }
}

#[cfg(test)]
mod overlay_ink_tests {
    use super::*;
    #[test]
    fn overlay_is_independent_of_locked_default_right_ink() {
        let mut spec = Spec {
            legacy: true,
            pos: Some(LegendPos::Right),
            style: TextStyle::default(),
            overlay: false,
            manual: None,
            deleted: Vec::new(),
            entries: Vec::new(),
            background: FrameFill::NoFill,
            border: None,
            shadow: None,
        };
        assert!(spec.default_right_ink());
        spec.overlay = true;
        assert!(spec.default_right_ink());
        spec.style.bold = true;
        assert!(!spec.default_right_ink());
        spec.style = TextStyle::default();
        spec.deleted.push(0);
        assert!(!spec.default_right_ink());
        spec.deleted.clear();
        spec.pos = Some(LegendPos::Bottom);
        assert!(!spec.default_right_ink());
    }
}
