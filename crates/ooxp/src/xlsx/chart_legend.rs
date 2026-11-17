//! Legend semantics live separately from chart axes. All dimensions are fixed-96 pixels.
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
}
pub(super) struct Spec {
    pub(super) legacy: bool,
    pos: Option<LegendPos>,
    style: TextStyle,
    overlay: bool,
    manual: Option<Manual>,
    deleted: Vec<usize>,
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
    }
    pub(super) fn read(root: &Element, accents: [[u8; 3]; 6]) -> Result<Self, Error> {
        let chart = root.child_by_local("chart").ok_or_else(unsupported)?;
        let pos = legend_pos(chart);
        let mut result = Self {
            legacy: true,
            pos,
            style: TextStyle::default(),
            overlay: false,
            manual: None,
            deleted: Vec::new(),
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
        result.overlay = legend
            .child_by_local("overlay")
            .is_some_and(|n| bool_attr(n.attr_local("val"), false));
        if let Some(layout) = legend
            .child_by_local("layout")
            .and_then(|n| n.child_by_local("manualLayout"))
            .filter(|n| {
                ["x", "y", "w", "h"].iter().any(|key| {
                    number(n, key).is_some_and(|v| !matches!(*key, "w" | "h") || v > 0.0)
                })
            })
        {
            if layout
                .child_by_local("layoutTarget")
                .and_then(|n| n.attr_local("val"))
                == Some("inner")
            {
                return Err(unsupported());
            }
            for mode in ["wMode", "hMode"] {
                if layout
                    .child_by_local(mode)
                    .and_then(|n| n.attr_local("val"))
                    == Some("edge")
                {
                    return Err(unsupported());
                }
            }
            let size = |key| number(layout, key).filter(|v| *v > 0.0);
            result.manual = Some(Manual {
                x: coord(layout, "x", "xMode"),
                y: coord(layout, "y", "yMode"),
                w: size("w"),
                h: size("h"),
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
            } else if entry.child_by_local("txPr").is_some() {
                return Err(unsupported());
            }
        }
        if let Some(sp) = legend.child_by_local("spPr") {
            for child in sp.element_children() {
                match child.local_name() {
                    "noFill" | "extLst" => (),
                    "effectLst" | "effectDag" if child.element_children().next().is_none() => (),
                    "ln" if child.child_by_local("noFill").is_some() => (),
                    _ => return Err(unsupported()),
                }
            }
        }
        result.legacy = result.default_right_ink() && !result.overlay;
        Ok(result)
    }
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
                "latin" | "ea" | "extLst" => (),
                "effectLst" | "effectDag" if fill.element_children().next().is_none() => (),
                _ => return Err(unsupported()),
            }
        }
        Ok(())
    }
}
struct Item<'a> {
    series: &'a Series,
    width: f64,
}
pub(super) struct Layout<'a> {
    spec: &'a Spec,
    items: Vec<Item<'a>>,
    face: Option<super::super::chart_gdi::Face>,
    px: f32,
    key: f64,
    space: f64,
    line: f64,
    frame: Rect,
    auto: Rect,
    horizontal: bool,
    rows: Vec<Vec<usize>>,
}
impl<'a> Layout<'a> {
    pub(super) fn new(
        canvas: &Canvas,
        chart: &'a ColumnChart,
        font: Option<&Font>,
    ) -> Result<Self, Error> {
        let spec = &chart.legend_spec;
        let px = (spec.style.pt * DPI / 72.0) as f32;
        #[cfg(not(windows))]
        if spec.style.bold || spec.style.italic || !spec.style.name.is_empty() {
            return Err(unsupported());
        }
        let face = if spec.style.name.is_empty() {
            let face = super::super::chart_gdi::Face::open(px, spec.style.bold);
            #[cfg(windows)]
            let face = if spec.style.italic {
                face.and_then(|f| {
                    super::super::chart_gdi::Face::open_cell(
                        px,
                        spec.style.bold,
                        true,
                        f.family_name(),
                    )
                })
            } else {
                face
            };
            face
        } else {
            let face = super::super::chart_gdi::Face::open_cell(
                px,
                spec.style.bold,
                spec.style.italic,
                &spec.style.name,
            );
            #[cfg(windows)]
            if face
                .as_ref()
                .is_none_or(|f| !same_name(f.family_name(), &spec.style.name))
            {
                return Err(unsupported());
            }
            face
        };
        let key = (f64::from(px) * 0.5).round().max(1.0);
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
                let width = face
                    .as_ref()
                    .map(|f| f.layout_width(&series.name))
                    .unwrap_or_else(|| text_width(font, None, &series.name, px));
                items.push(Item {
                    series,
                    width: key + space + width,
                });
            }
        }
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
        let natural_h = if auto_horizontal {
            line + 2.0 * space
        } else {
            line * items.len() as f64
                + 2.0 * space * items.len().saturating_sub(1) as f64
                + 2.0 * space
        };
        let w = f64::from(canvas.w);
        let h = f64::from(canvas.h);
        let mut auto = position(spec.pos, w, h, natural_w, natural_h, space);
        if chart.title.is_some() {
            match spec.pos {
                Some(LegendPos::Top | LegendPos::TopRight) => {
                    auto.y += f64::from(LEGEND_TITLE_BAND)
                }
                Some(LegendPos::Right | LegendPos::Left) => {
                    auto.y += f64::from(LEGEND_TITLE_BAND) / 2.0
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
            frame.w = m.w.map(|v| v * w).unwrap_or(auto.w);
            frame.h = m.h.map(|v| v * h).unwrap_or(auto.h);
            if frame.w <= 0.0 || frame.h <= 0.0 || frame.w > w || frame.h > h {
                return Err(unsupported());
            }
            frame.x = if m.x.edge {
                m.x.value * w
            } else {
                auto.x + auto.w / 2.0 - frame.w / 2.0 + m.x.value * w
            };
            frame.y = if m.y.edge {
                m.y.value * h
            } else {
                auto.y + auto.h / 2.0 - frame.h / 2.0 + m.y.value * h
            };
            frame.x = frame.x.clamp(0.0, w - frame.w);
            frame.y = frame.y.clamp(0.0, h - frame.h);
        }
        let mut result = Self {
            spec,
            items,
            face,
            px,
            key,
            space,
            line,
            frame,
            auto,
            horizontal,
            rows: Vec::new(),
        };
        result.flow()?;
        if !spec.legacy
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
        if self.spec.legacy || self.items.is_empty() {
            return Ok(());
        }
        let mut row = Vec::new();
        let mut used = 0.0;
        for (idx, item) in self.items.iter().enumerate() {
            if item.width + self.space * 2.0 > self.frame.w {
                return Err(unsupported());
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
        let required = self.rows.len() as f64 * self.line
            + self.rows.len().saturating_sub(1) as f64 * self.space * 2.0;
        if required > self.frame.h {
            if self.spec.manual.is_some() {
                return Err(unsupported());
            }
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
        if self.spec.overlay || self.items.is_empty() {
            return r;
        }
        match self.spec.pos {
            Some(LegendPos::Right | LegendPos::TopRight) => {
                r[2] = (self.auto.w + f64::from(LEGEND_PLOT_GAP) - f64::from(INSET_TOP))
                    .ceil()
                    .max(f64::from(INSET_RIGHT - INSET_TOP)) as i32
            }
            Some(LegendPos::Left) => r[0] = (self.auto.w + self.space * 2.0).ceil() as i32,
            Some(LegendPos::Top) => r[1] = (self.auto.h + self.space * 2.0).ceil() as i32,
            Some(LegendPos::Bottom) => r[3] = (self.auto.h + self.space * 2.0).ceil() as i32,
            None => (),
        }
        r
    }
    pub(super) fn draw(&self, canvas: &mut Canvas, chart: &ColumnChart, font: Option<&Font>) {
        let height = self.rows.len() as f64 * self.line
            + self.rows.len().saturating_sub(1) as f64 * self.space * 2.0;
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
            let mut x = self.frame.x + (self.frame.w - width) / 2.0;
            for idx in row {
                let item = &self.items[*idx];
                let ser = item.series;
                let ky = y + (self.line - self.key) / 2.0;
                draw_plot_key(
                    canvas,
                    chart.kind,
                    x.round() as i32,
                    ky.round() as i32,
                    (x + self.key).round() as i32,
                    (ky + self.key).round() as i32,
                    ser.color.unwrap_or(chart.accents[ser.slot % 6]),
                );
                draw_text(
                    canvas,
                    font,
                    self.face.as_ref(),
                    &ser.name,
                    x + self.key + self.space,
                    y,
                    self.px,
                    self.spec.style.color,
                );
                x += item.width + gap;
            }
            y += self.line + self.space * 2.0;
        }
    }
}
fn position(pos: Option<LegendPos>, w: f64, h: f64, width: f64, height: f64, pad: f64) -> Rect {
    let (x, y) = match pos {
        Some(LegendPos::Left) => (pad, (h - height) / 2.0),
        Some(LegendPos::Top) => ((w - width) / 2.0, pad),
        Some(LegendPos::Bottom) => ((w - width) / 2.0, h - pad - height),
        Some(LegendPos::TopRight) => (w - pad - width, pad),
        _ => (w - pad - width, (h - height) / 2.0),
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
        assert!(s.legacy);
        assert!(spec("<legend><layout><manualLayout><x val='NaN'/><w val='-1'/></manualLayout></layout></legend>").unwrap().legacy);
        for pos in ["l", "t", "b", "tr"] {
            assert!(
                !spec(&format!("<legend><legendPos val='{pos}'/></legend>"))
                    .unwrap()
                    .legacy
            );
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
        assert!(spec(
            "<legend><layout><manualLayout><wMode val='edge'/><w val='.4'/></manualLayout></layout></legend>"
        )
        .is_err());
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
