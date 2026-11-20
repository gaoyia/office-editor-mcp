//! Linear value-axis scalars. Nice-number search is bounded by label line
//! capacity, not a universal 5..10 interval count. Office's general automatic
//! algorithm is undocumented; nonzero automatic minima are explicitly out of scope.
use super::{Element, Error, ErrorKind, RenderDpi, Series, INSET_BOTTOM, INSET_TOP};
#[derive(Clone, Copy, Default)]
pub(super) struct Spec {
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub major: Option<f64>,
    pub minor: Option<f64>,
    pub log_base: Option<f64>,
    /// Category-axis crossesAt on the value scale. Official Excel 16 writes
    /// this on catAx when the value-axis CrossesAt is set; valAx stays autoZero.
    pub crosses_at: Option<f64>,
}
pub(super) struct Scale {
    pub min: f64,
    pub max: f64,
    pub major: f64,
    pub minor: f64,
    pub log_base: Option<f64>,
    pub crosses_at: Option<f64>,
}
impl Scale {
    /// autoZero crosses at zero, or the nearer bound when zero is outside.
    /// A finite catAx crossesAt uses that value, clamped to the visible scale.
    pub fn crossing(&self) -> f64 {
        self.crosses_at.unwrap_or(0.0).clamp(self.min, self.max)
    }
}

fn unsupported() -> Error {
    Error::with_path(ErrorKind::ChartImageUnsupported, "id")
}

fn read_cat_crosses_at(plot: &Element) -> Result<Option<f64>, Error> {
    let cats: Vec<_> = plot
        .element_children()
        .filter(|e| e.local_name() == "catAx")
        .collect();
    if cats.len() > 1 {
        return Err(unsupported());
    }
    let Some(cat) = cats.first() else {
        return Ok(None);
    };
    let crosses = cat
        .child_by_local("crosses")
        .and_then(|e| e.attr_local("val"));
    if cat.child_by_local("crossesAt").is_some() && crosses.is_some() {
        return Err(unsupported());
    }
    if let Some(el) = cat.child_by_local("crossesAt") {
        let value = el
            .attr_local("val")
            .and_then(|v| v.parse::<f64>().ok())
            .filter(|v| v.is_finite())
            .ok_or_else(unsupported)?;
        return Ok(Some(value));
    }
    if crosses.is_some_and(|v| v != "autoZero") {
        return Err(unsupported());
    }
    Ok(None)
}

fn values_out_of_log_range(raw_min: f64, raw_max: f64, min: f64, max: f64) -> bool {
    !raw_min.is_finite() || !raw_max.is_finite() || raw_min < min - 1e-12 || raw_max > max + 1e-12
}

fn is_integer_power(value: f64, base: f64) -> bool {
    if value <= 0.0 || base <= 1.0 || !value.is_finite() || !base.is_finite() {
        return false;
    }
    let exp = value.log(base);
    if !exp.is_finite() || exp < -1e-9 {
        return false;
    }
    let rounded = exp.round();
    (value - base.powf(rounded)).abs() <= 1e-9 * value.max(1.0)
}
impl Spec {
    pub fn read(plot: &Element) -> Result<Self, Error> {
        let axes: Vec<_> = plot
            .element_children()
            .filter(|e| e.local_name() == "valAx")
            .collect();
        if axes.len() > 1 {
            return Err(unsupported());
        }
        let Some(axis) = axes.first() else {
            return Ok(Self::default());
        };
        let mut spec = Self::read_axis(axis)?;
        spec.crosses_at = read_cat_crosses_at(plot)?;
        Ok(spec)
    }

    pub fn read_axis(axis: &Element) -> Result<Self, Error> {
        if axis
            .child_by_local("delete")
            .and_then(|e| e.attr_local("val"))
            .is_some_and(|v| !matches!(v, "0" | "false" | "off"))
        {
            return Err(unsupported());
        }
        let scaling = axis.child_by_local("scaling");
        if scaling.is_some_and(|s| {
            s.child_by_local("orientation")
                .and_then(|e| e.attr_local("val"))
                .is_some_and(|v| v != "minMax")
        }) {
            return Err(unsupported());
        }
        let log_base = match scaling.and_then(|s| s.child_by_local("logBase")) {
            Some(el) => {
                let value = el
                    .attr_local("val")
                    .and_then(|v| v.parse::<f64>().ok())
                    .filter(|v| v.is_finite())
                    .ok_or_else(unsupported)?;
                if !(2.0..=1000.0).contains(&value) {
                    return Err(unsupported());
                }
                Some(value)
            }
            None => None,
        };
        if axis.child_by_local("crossesAt").is_some()
            || axis
                .child_by_local("crosses")
                .and_then(|e| e.attr_local("val"))
                .is_some_and(|v| v != "autoZero")
        {
            return Err(unsupported());
        }
        if axis
            .child_by_local("numFmt")
            .and_then(|e| e.attr_local("formatCode"))
            .is_some_and(|v| v != "General")
        {
            return Err(unsupported());
        }
        let read = |node: Option<&Element>, name: &str| -> Result<Option<f64>, Error> {
            let Some(el) = node.and_then(|n| n.child_by_local(name)) else {
                return Ok(None);
            };
            let value = el
                .attr_local("val")
                .and_then(|v| v.parse::<f64>().ok())
                .filter(|v| v.is_finite())
                .ok_or_else(unsupported)?;
            Ok(Some(value))
        };
        let spec = Self {
            min: read(scaling, "min")?,
            max: read(scaling, "max")?,
            major: read(Some(axis), "majorUnit")?,
            minor: read(Some(axis), "minorUnit")?,
            log_base,
            crosses_at: None,
        };
        if spec.major.is_some_and(|v| v <= 0.0)
            || spec.minor.is_some_and(|v| v <= 0.0)
            || matches!((spec.min,spec.max),(Some(a),Some(b)) if a>=b)
        {
            return Err(unsupported());
        }
        if let Some(base) = spec.log_base {
            // Official Excel 16 log axes write no majorUnit/minorUnit; custom
            // steps are not uniquely locked. Auto min/max (omitted in XML) stay
            // NI. Only explicit positive power-of-base ends are accepted here.
            if spec.major.is_some() || spec.minor.is_some() {
                return Err(unsupported());
            }
            let (Some(min), Some(max)) = (spec.min, spec.max) else {
                return Err(unsupported());
            };
            if min < 1.0
                || max <= min
                || !is_integer_power(min, base)
                || !is_integer_power(max, base)
            {
                return Err(unsupported());
            }
        }
        Ok(spec)
    }
}
/// A full label line plus one raster-grid row, capped at the historic ten slots.
/// Face metrics come from the already-dpi-aware label size. Insets follow the
/// request export DPI, never the Windows desktop DPI.
#[cfg(test)]
pub(super) fn interval_budget(height: u32, label_em: f64) -> i32 {
    interval_budget_dpi(height, label_em, RenderDpi::DEFAULT)
}

pub(super) fn interval_budget_dpi(height: u32, label_em: f64, dpi: RenderDpi) -> i32 {
    interval_budget_span(
        f64::from(height) - f64::from(dpi.scale_px(INSET_TOP) + dpi.scale_px(INSET_BOTTOM)),
        label_em,
        1.0,
    )
}

pub(super) fn interval_budget_span(span: f64, label_em: f64, occupancy: f64) -> i32 {
    let room = span.max(1.0);
    (room / (label_em.max(1.0) * occupancy.max(1.0) + 1.0))
        .floor()
        .clamp(1.0, 10.0) as i32
}

pub(super) fn scatter_axes(plot: &Element) -> Result<(Spec, Spec), Error> {
    if plot
        .element_children()
        .any(|e| matches!(e.local_name(), "catAx" | "dateAx" | "serAx"))
    {
        return Err(unsupported());
    }
    let axes: Vec<_> = plot
        .element_children()
        .filter(|e| e.local_name() == "valAx")
        .collect();
    if axes.len() != 2 {
        return Err(unsupported());
    }
    let mut x = None;
    let mut y = None;
    for axis in axes {
        let pos = axis
            .child_by_local("axPos")
            .and_then(|e| e.attr_local("val"))
            .unwrap_or("");
        let spec = Spec::read_axis(axis)?;
        match pos {
            "b" | "t" => {
                if x.is_some() {
                    return Err(unsupported());
                }
                x = Some(spec);
            }
            "l" | "r" => {
                if y.is_some() {
                    return Err(unsupported());
                }
                y = Some(spec);
            }
            _ => return Err(unsupported()),
        }
    }
    Ok((x.ok_or_else(unsupported)?, y.ok_or_else(unsupported)?))
}

/// 簇状条形图：类目轴在左（axPos=l），包内数值轴 axPos 仍为 b；绘制时 maxMin（第一类在上）把数值轴放在绘图区顶，minMax 放在底。
/// 类目 scaling 为 minMax 或省略时，第一类在下；maxMin 时第一类在上（Excel 默认条形图常见）。
/// 返回 (数值轴, 第一类是否在下)。
pub(super) fn bar_axes(plot: &Element) -> Result<(Spec, bool), Error> {
    if plot
        .element_children()
        .any(|e| matches!(e.local_name(), "dateAx" | "serAx"))
    {
        return Err(unsupported());
    }
    let cats: Vec<_> = plot
        .element_children()
        .filter(|e| e.local_name() == "catAx")
        .collect();
    let vals: Vec<_> = plot
        .element_children()
        .filter(|e| e.local_name() == "valAx")
        .collect();
    if cats.len() != 1 || vals.len() != 1 {
        return Err(unsupported());
    }
    let cat = cats[0];
    let val = vals[0];
    let cat_pos = cat
        .child_by_local("axPos")
        .and_then(|e| e.attr_local("val"))
        .unwrap_or("");
    let val_pos = val
        .child_by_local("axPos")
        .and_then(|e| e.attr_local("val"))
        .unwrap_or("");
    if cat_pos != "l" || val_pos != "b" {
        return Err(unsupported());
    }
    if cat
        .child_by_local("delete")
        .and_then(|e| e.attr_local("val"))
        .is_some_and(|v| !matches!(v, "0" | "false" | "off"))
    {
        return Err(unsupported());
    }
    if cat.child_by_local("majorGridlines").is_some() {
        return Err(unsupported());
    }
    let cat_orient = cat
        .child_by_local("scaling")
        .and_then(|s| s.child_by_local("orientation"))
        .and_then(|e| e.attr_local("val"))
        .unwrap_or("minMax");
    if cat
        .child_by_local("scaling")
        .is_some_and(|s| s.child_by_local("logBase").is_some())
    {
        return Err(unsupported());
    }
    // Horizontal bar crossesAt is not in the F16 official column/line/area set.
    if cat.child_by_local("crossesAt").is_some()
        || cat
            .child_by_local("crosses")
            .and_then(|e| e.attr_local("val"))
            .is_some_and(|v| v != "autoZero")
    {
        return Err(unsupported());
    }
    let from_bottom = match cat_orient {
        "minMax" => true,
        "maxMin" => false,
        _ => return Err(unsupported()),
    };
    Ok((Spec::read_axis(val)?, from_bottom))
}

pub(super) fn scale(series: &[Series], spec: Spec, budget: i32) -> Result<Scale, Error> {
    scale_values(
        series.iter().flat_map(|s| s.values.iter().copied()),
        spec,
        budget,
    )
}

pub(super) fn scale_values(
    values: impl Iterator<Item = Option<f64>>,
    spec: Spec,
    budget: i32,
) -> Result<Scale, Error> {
    let mut raw_min = f64::INFINITY;
    let mut raw_max = f64::NEG_INFINITY;
    for v in values.flatten().filter(|v| v.is_finite()) {
        raw_min = raw_min.min(v);
        raw_max = raw_max.max(v);
    }
    if !raw_min.is_finite() {
        raw_min = 0.0;
        raw_max = 0.0;
    }
    if let Some(base) = spec.log_base {
        let min = spec.min.ok_or_else(unsupported)?;
        let max = spec.max.ok_or_else(unsupported)?;
        // Official F16 log evidence had autoZero only; custom crosses stay NI.
        if spec.crosses_at.is_some() {
            return Err(unsupported());
        }
        // Out-of-range values (official clip stubs) and non-positive points stay NI.
        if values_out_of_log_range(raw_min, raw_max, min, max) {
            return Err(unsupported());
        }
        return Ok(Scale {
            min,
            max,
            major: base,
            minor: base,
            log_base: Some(base),
            crosses_at: None,
        });
    }
    // Excel may choose a nonzero minimum for tightly clustered one-sided data.
    // Its general layout-dependent algorithm is not implemented; don't force zero.
    if spec.min.is_none() && raw_min > 0.0 && raw_min < raw_max && raw_min >= raw_max * 5.0 / 6.0 {
        return Err(unsupported());
    }
    if spec.max.is_none() && raw_max < 0.0 && raw_min < raw_max && raw_max <= raw_min * 5.0 / 6.0 {
        return Err(unsupported());
    }
    let lo = raw_min.min(0.0);
    let hi = raw_max.max(0.0);
    let ends = |step: f64| {
        let mut a = spec.min.unwrap_or_else(|| (lo / step).floor() * step);
        let mut b = spec.max.unwrap_or_else(|| (hi / step).ceil() * step);
        if spec.max.is_none() && hi > 1e-12 && hi > b * 0.95 + b.abs() * 1e-9 {
            b += step;
        }
        if spec.min.is_none() && lo < -1e-12 && lo < a * 0.95 - a.abs() * 1e-9 {
            a -= step;
        }
        (a, b)
    };
    let result = if let Some(step) = spec.major {
        let (a, b) = ends(step);
        (a, b, step)
    } else if raw_min.abs() < 1e-12
        && raw_max.abs() < 1e-12
        && spec.min.is_none()
        && spec.max.is_none()
    {
        (0.0, 1.0, 0.2)
    } else {
        let span = (spec.max.unwrap_or(hi) - spec.min.unwrap_or(lo))
            .abs()
            .max(1e-9);
        let exp = span.log10().floor() as i32;
        let mut best: Option<(f64, f64, f64)> = None;
        for mag in (exp - 2)..=(exp + 1) {
            for nice in [1.0, 2.0, 5.0] {
                let step = nice * 10f64.powi(mag);
                let (a, b) = ends(step);
                let intervals = ((b - a) / step - 1e-9).ceil();
                if b <= a || !(1.0..=f64::from(budget)).contains(&intervals) {
                    continue;
                }
                if best.is_none_or(|(oa, ob, os)| {
                    b - a < ob - oa - 1e-9 || ((b - a - (ob - oa)).abs() < 1e-9 && step < os)
                }) {
                    best = Some((a, b, step));
                }
            }
        }
        best.ok_or_else(unsupported)?
    };
    let (min, max, major) = result;
    let minor = spec.minor.unwrap_or(major / 5.0);
    if max <= min
        || !max.is_finite()
        || !min.is_finite()
        || !major.is_finite()
        || !minor.is_finite()
        || (max - min) / major > 1000.0
        || (max - min) / minor > 5000.0
    {
        return Err(unsupported());
    }
    Ok(Scale {
        min,
        max,
        major,
        minor,
        log_base: None,
        crosses_at: spec.crosses_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn check(label_em: f64, emit: bool) {
        let rows: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/data/chart_axis_oracle.json")).unwrap();
        for row in rows.as_array().unwrap() {
            let series = Series {
                name: "S".into(),
                values: row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_f64())
                    .collect(),
                color: None,
                slot: 0,
                ..Series::default()
            };
            let e = &row["explicit"];
            let spec = Spec {
                min: e["min"].as_f64(),
                max: e["max"].as_f64(),
                major: e["majorUnit"].as_f64(),
                minor: e["minorUnit"].as_f64(),
                log_base: None,
                crosses_at: None,
            };
            let budget = interval_budget(row["height96"].as_u64().unwrap() as u32, label_em);
            let result = scale(&[series], spec, budget);
            if row["unsupported"] == true {
                assert!(result.is_err(), "{} must be honest NI", row["case"]);
                continue;
            }
            let got = result.unwrap();
            let expected = &row["expected"];
            for (key, value) in [
                ("min", got.min),
                ("max", got.max),
                ("major", got.major),
                ("minor", got.minor),
                ("crossing", got.crossing()),
            ] {
                let want = expected[key].as_f64().unwrap();
                assert!(
                    (want - value).abs() <= want.abs().max(1.0) * 1e-9,
                    "{} {key} got {value}, want {want}, budget {budget}, em {label_em}",
                    row["case"]
                );
            }
            if emit {
                println!(
                    "AXIS_ROW|{}",
                    serde_json::json!({"case":row["case"],"min":got.min,"max":got.max,"major":got.major,"minor":got.minor,"crossing":got.crossing(),"budget":budget,"labelEm96":label_em})
                );
            }
        }
    }
    #[test]
    fn independent_scalar_oracle_fixed96() {
        check(17.0, false);
    }
    #[test]
    #[cfg(windows)]
    fn native_scalar_oracle_fixed96() {
        let f =
            super::super::super::chart_gdi::Face::open((10.0 * 96.0 / 72.0) as f32, false).unwrap();
        let path =
            std::path::PathBuf::from(std::env::var_os("WINDIR").unwrap()).join("Fonts/calibri.ttf");
        let font = fontdue::Font::from_bytes(
            std::fs::read(path).unwrap(),
            fontdue::FontSettings::default(),
        )
        .unwrap();
        let line = font
            .horizontal_line_metrics((10.0 * 96.0 / 72.0) as f32)
            .unwrap()
            .new_line_size;
        check(f.em_box().max(f64::from(line)), true);
    }
    #[test]
    fn budget_is_line_capacity_not_current_desktop_dpi() {
        assert_eq!(interval_budget(200, 17.0), 8);
        assert_eq!(interval_budget(336, 17.0), 10);
        assert_eq!(interval_budget(96, 17.0), 2);
        assert!(interval_budget(200, 25.0) < interval_budget(200, 17.0));
    }

    #[test]
    fn auto_scale_minus_eight_to_eight_uses_major_two_when_budget_allows() {
        // Office automatic scaling of -8..8 is -10..10, majorUnit=2 (10 intervals).
        // This is the shared nice-number search, not a one-chart special case.
        let values = [Some(-8.0), Some(0.0), Some(8.0)];
        let got = scale_values(values.into_iter(), Spec::default(), 10).unwrap();
        assert_eq!(
            (got.min, got.max, got.major),
            (-10.0, 10.0, 2.0),
            "budget 10 must accept Office's 10-interval major=2"
        );
    }

    #[test]
    fn horizontal_numeric_budget_reaches_ten_on_typical_bar_width() {
        // Measured plot width of the 636-wide official bar charts is 537px.
        // occupancy=3 drops below 10 once em exceeds ~17.6, which is why those
        // signed bars used major=5. Numeric occupancy 1 must still reach the cap.
        let em = 20.0;
        assert_eq!(interval_budget_span(537.0, em, 1.0), 10);
        assert!(
            interval_budget_span(537.0, em, 3.0) < 10,
            "occupancy 3 is what used to reject major=2 on this width"
        );
    }
    #[test]
    fn explicit_xml_is_not_silently_ignored() {
        let doc=crate::xml::parse(b"<plotArea><valAx><scaling><min val='-2'/><max val='8'/></scaling><majorUnit val='2'/><minorUnit val='0.5'/></valAx></plotArea>").unwrap();
        let s = Spec::read(&doc.root).unwrap();
        assert_eq!(
            (s.min, s.max, s.major, s.minor),
            (Some(-2.0), Some(8.0), Some(2.0), Some(0.5))
        );
        for bad in ["<logBase val='10'/>", "<orientation val='maxMin'/>"] {
            let xml = format!("<plotArea><valAx><scaling>{bad}</scaling></valAx></plotArea>");
            let d = crate::xml::parse(xml.as_bytes()).unwrap();
            assert!(Spec::read(&d.root).is_err());
        }
        let log = crate::xml::parse(
            b"<plotArea><valAx><scaling><logBase val='10'/><min val='1'/><max val='1000'/></scaling></valAx></plotArea>",
        )
        .unwrap();
        let s = Spec::read(&log.root).unwrap();
        assert_eq!(s.log_base, Some(10.0));
        assert_eq!((s.min, s.max), (Some(1.0), Some(1000.0)));
        for bad in [
            "<logBase val='10'/><min val='1'/>",
            "<logBase val='10'/><min val='3'/><max val='300'/>",
            "<logBase val='10'/><min val='0.1'/><max val='1000'/>",
            "<logBase val='1'/><min val='1'/><max val='1000'/>",
        ] {
            let xml = format!("<plotArea><valAx><scaling>{bad}</scaling></valAx></plotArea>");
            let d = crate::xml::parse(xml.as_bytes()).unwrap();
            assert!(Spec::read(&d.root).is_err(), "unverified log scale {bad}");
        }
    }

    #[test]
    fn log_explicit_power_bounds_scale_and_reject_out_of_range() {
        let spec = Spec {
            min: Some(1.0),
            max: Some(1000.0),
            major: None,
            minor: None,
            log_base: Some(10.0),
            crosses_at: None,
        };
        let ok = scale_values(
            [Some(1.0), Some(10.0), Some(100.0), Some(1000.0)].into_iter(),
            spec,
            10,
        )
        .unwrap();
        assert_eq!(
            (ok.min, ok.max, ok.major, ok.log_base),
            (1.0, 1000.0, 10.0, Some(10.0))
        );
        assert!(
            scale_values([Some(1.0), Some(10.0), Some(0.5)].into_iter(), spec, 10).is_err(),
            "values below the explicit log min must stay NI"
        );
    }

    #[test]
    fn cat_axis_crosses_at_sets_crossing_and_clamps() {
        let doc = crate::xml::parse(
            b"<plotArea><valAx><scaling><min val='-8'/><max val='8'/></scaling><majorUnit val='2'/></valAx><catAx><crossesAt val='2'/></catAx></plotArea>",
        )
        .unwrap();
        let s = Spec::read(&doc.root).unwrap();
        assert_eq!(s.crosses_at, Some(2.0));
        let got = scale_values(
            [Some(-4.0), Some(-1.0), Some(3.0), Some(6.0)].into_iter(),
            s,
            10,
        )
        .unwrap();
        assert_eq!(got.crossing(), 2.0);
        let outside = crate::xml::parse(
            b"<plotArea><valAx><scaling><min val='-8'/><max val='8'/></scaling></valAx><catAx><crossesAt val='10'/></catAx></plotArea>",
        )
        .unwrap();
        let s = Spec::read(&outside.root).unwrap();
        let got = scale_values([Some(-4.0), Some(6.0)].into_iter(), s, 10).unwrap();
        assert_eq!(
            got.crossing(),
            8.0,
            "official F16-6 clamps crossesAt to max"
        );
        let val_ax = crate::xml::parse(
            b"<plotArea><valAx><scaling><min val='-8'/><max val='8'/></scaling><crossesAt val='2'/></valAx></plotArea>",
        )
        .unwrap();
        assert!(
            Spec::read(&val_ax.root).is_err(),
            "valAx crossesAt is not the official Excel 16 encoding"
        );
        let crosses_min = crate::xml::parse(
            b"<plotArea><valAx><scaling><min val='-8'/><max val='8'/></scaling></valAx><catAx><crosses val='min'/></catAx></plotArea>",
        )
        .unwrap();
        assert!(
            Spec::read(&crosses_min.root).is_err(),
            "catAx crosses=min/max stay NI until F17"
        );
        let log = crate::xml::parse(
            b"<plotArea><valAx><scaling><logBase val='10'/><min val='1'/><max val='1000'/></scaling></valAx><catAx><crossesAt val='10'/></catAx></plotArea>",
        )
        .unwrap();
        let s = Spec::read(&log.root).unwrap();
        assert!(
            scale_values([Some(1.0), Some(10.0), Some(100.0)].into_iter(), s, 10).is_err(),
            "log plus custom crossesAt stays NI"
        );
    }
}
