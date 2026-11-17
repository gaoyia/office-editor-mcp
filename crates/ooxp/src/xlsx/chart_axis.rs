//! Linear value-axis scalars. Nice-number search is bounded by label line
//! capacity, not a universal 5..10 interval count. Office's general automatic
//! algorithm is undocumented; nonzero automatic minima are explicitly out of scope.
use super::{Element, Error, ErrorKind, Series, INSET_BOTTOM, INSET_TOP};
#[derive(Clone, Copy, Default)]
pub(super) struct Spec {
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub major: Option<f64>,
    pub minor: Option<f64>,
}
pub(super) struct Scale {
    pub min: f64,
    pub max: f64,
    pub major: f64,
    pub minor: f64,
}
impl Scale {
    /// Default autoZero crosses at zero, or the nearer bound when zero is outside.
    pub fn crossing(&self) -> f64 {
        0.0_f64.clamp(self.min, self.max)
    }
}

fn unsupported() -> Error {
    Error::with_path(ErrorKind::ChartImageUnsupported, "id")
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
        Self::read_axis(axis)
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
            s.child_by_local("logBase").is_some()
                || s.child_by_local("orientation")
                    .and_then(|e| e.attr_local("val"))
                    .is_some_and(|v| v != "minMax")
        }) {
            return Err(unsupported());
        }
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
        };
        if spec.major.is_some_and(|v| v <= 0.0)
            || spec.minor.is_some_and(|v| v <= 0.0)
            || matches!((spec.min,spec.max),(Some(a),Some(b)) if a>=b)
        {
            return Err(unsupported());
        }
        Ok(spec)
    }
}
/// A full label line plus one raster-grid row, capped at the historic ten slots.
/// Uses the existing fixed96 face metrics; never the current desktop DPI.
pub(super) fn interval_budget(height: u32, label_em: f64) -> i32 {
    interval_budget_span(
        f64::from(height) - f64::from(INSET_TOP + INSET_BOTTOM),
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
    }
}
