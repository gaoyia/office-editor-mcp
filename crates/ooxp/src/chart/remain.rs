//! 趋势线、误差线、次轴，以及系列上的额外 inspect 字段。

use serde_json::{json, Map, Value};

use crate::error::{Error, ErrorKind, IdReason, TypeName};
use crate::xml::{Element, Node};

use super::{
    bool_val, collect_sers, inspect_type, plot_area, plot_area_mut, series_count, with_ser_mut,
    WRAPPERS,
};

const TREND_TYPES: &[&str] = &["linear", "exp", "log", "poly", "power", "movingAvg"];

#[derive(Debug, Clone)]
pub struct TrendlineWrite {
    pub kind: Option<String>,
    pub order: Option<u32>,
    pub period: Option<u32>,
    pub display_r_squared: Option<bool>,
    pub display_equation: Option<bool>,
    pub name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ErrorBarsWrite {
    pub dir: Option<String>,
    pub bar_type: Option<String>,
    pub val_type: Option<String>,
    pub value: Option<f64>,
}

pub fn split_member<'a>(id: &'a str, member: &str) -> Option<(&'a str, &'a str)> {
    let pat = match member {
        "trendline" => "/trendline:",
        "errorBars" => "/errorBars:",
        _ => return None,
    };
    id.rsplit_once(pat)
}

pub fn parse_member_index(n: &str, from_one: IdFromOne) -> Result<usize, MemberIndexErr> {
    let v: usize = n.parse().map_err(|_| MemberIndexErr::NotInt(from_one))?;
    if v < 1 {
        return Err(MemberIndexErr::FromOne(from_one));
    }
    Ok(v)
}

#[derive(Clone, Copy)]
pub enum IdFromOne {
    Trendline,
    ErrorBars,
}

pub enum MemberIndexErr {
    NotInt(IdFromOne),
    FromOne(IdFromOne),
}

impl MemberIndexErr {
    pub fn reason(self) -> IdReason {
        match self {
            Self::NotInt(IdFromOne::Trendline) => IdReason::TrendlineIndexNotInt,
            Self::FromOne(IdFromOne::Trendline) => IdReason::TrendlineIndexFromOne,
            Self::NotInt(IdFromOne::ErrorBars) => IdReason::ErrorBarsIndexNotInt,
            Self::FromOne(IdFromOne::ErrorBars) => IdReason::ErrorBarsIndexFromOne,
        }
    }
}

pub fn parse_member_id<'a>(
    id: &'a str,
    member: &str,
    from_one: IdFromOne,
) -> Result<(&'a str, usize), IdReason> {
    let prefix = match member {
        "errorBars" => IdReason::ErrorBarsPrefix,
        _ => IdReason::TrendlinePrefix,
    };
    let Some((left, n)) = split_member(id, member) else {
        return Err(prefix);
    };
    let k = parse_member_index(n, from_one).map_err(MemberIndexErr::reason)?;
    Ok((left, k))
}

pub fn supports_trendline(ty: &str) -> bool {
    matches!(
        ty,
        "bar" | "col" | "line" | "area" | "scatter" | "bubble" | "radar" | "combo" | "stock"
    )
}

pub fn supports_error_bars(ty: &str) -> bool {
    matches!(
        ty,
        "bar" | "col" | "line" | "area" | "scatter" | "bubble" | "radar" | "combo"
    )
}

pub fn supports_secondary_axis(ty: &str) -> bool {
    matches!(
        ty,
        "bar" | "col" | "line" | "area" | "scatter" | "bubble" | "radar" | "combo"
    )
}

pub fn reject_structure_edit(space: &Element, id: &str, i: usize) -> Result<(), Error> {
    if matches!(
        inspect_type(space, false).as_str(),
        "stock" | "surface" | "ofPie"
    ) {
        return Err(Error::with_path(
            ErrorKind::RestrictedChart { id: id.to_string() },
            format!("ops[{i}].id"),
        ));
    }
    Ok(())
}

pub fn attach_series_extras(
    node: &mut Map<String, Value>,
    space: &Element,
    ser: &Element,
    chart_id: &str,
    index: usize,
) {
    let axis = if series_on_secondary(space, index) {
        "secondary"
    } else {
        "primary"
    };
    node.insert("axis".into(), json!(axis));
    let tls = trendlines_json(chart_id, index, ser);
    if !tls.is_empty() {
        node.insert("trendlines".into(), json!(tls));
    }
    let ebs = error_bars_json(chart_id, index, ser);
    if !ebs.is_empty() {
        node.insert("errorBars".into(), json!(ebs));
    }
}

pub fn series_on_secondary(space: &Element, index: usize) -> bool {
    let Some(plot) = plot_area(space) else {
        return false;
    };
    let Some(wrap) = wrap_of_series(plot, index) else {
        return false;
    };
    let scatter = wrap.local_name() == "scatterChart" || wrap.local_name() == "bubbleChart";
    if let (Some(wrap_val), Some(primary_val)) = (
        wrap_value_axid(wrap, scatter),
        primary_val_id(plot, scatter),
    ) {
        if wrap_val != primary_val {
            return true;
        }
    }
    for id in wrapper_ax_ids(wrap) {
        if let Some(ax) = axis_by_id(plot, &id) {
            if bool_val(ax.child_by_local("overlay")) {
                return true;
            }
        }
    }
    false
}

fn trendlines_json(chart_id: &str, ser_index: usize, ser: &Element) -> Vec<Value> {
    let mut out = Vec::new();
    let mut k = 0usize;
    for c in ser.element_children() {
        if c.local_name() != "trendline" {
            continue;
        }
        k += 1;
        let ty = c
            .child_by_local("trendlineType")
            .and_then(|e| e.attr_local("val"))
            .unwrap_or("linear");
        let mut m = Map::new();
        m.insert(
            "id".into(),
            json!(format!("{chart_id}/series:{ser_index}/trendline:{k}")),
        );
        m.insert("index".into(), json!(k));
        m.insert("type".into(), json!(ty));
        if let Some(order) = child_u32(c, "order") {
            m.insert("order".into(), json!(order));
        }
        if let Some(period) = child_u32(c, "period") {
            m.insert("period".into(), json!(period));
        }
        m.insert(
            "displayRSquared".into(),
            json!(bool_val(c.child_by_local("dispRSqr"))),
        );
        m.insert(
            "displayEquation".into(),
            json!(bool_val(c.child_by_local("dispEq"))),
        );
        out.push(Value::Object(m));
    }
    out
}

fn error_bars_json(chart_id: &str, ser_index: usize, ser: &Element) -> Vec<Value> {
    let mut out = Vec::new();
    let mut k = 0usize;
    for c in ser.element_children() {
        if c.local_name() != "errBars" {
            continue;
        }
        k += 1;
        let dir = c
            .child_by_local("errDir")
            .and_then(|e| e.attr_local("val"))
            .unwrap_or("y");
        let bar_type = c
            .child_by_local("errBarType")
            .and_then(|e| e.attr_local("val"))
            .unwrap_or("both");
        let val_type = c
            .child_by_local("errValType")
            .and_then(|e| e.attr_local("val"))
            .unwrap_or("stdErr");
        let mut m = Map::new();
        m.insert(
            "id".into(),
            json!(format!("{chart_id}/series:{ser_index}/errorBars:{k}")),
        );
        m.insert("index".into(), json!(k));
        m.insert("dir".into(), json!(dir));
        m.insert("barType".into(), json!(bar_type));
        m.insert("valType".into(), json!(val_type));
        if let Some(v) = c
            .child_by_local("val")
            .and_then(|e| e.attr_local("val"))
            .and_then(|s| s.parse::<f64>().ok())
        {
            m.insert("value".into(), json!(v));
        }
        out.push(Value::Object(m));
    }
    out
}

pub fn insert_trendline(
    space: &mut Element,
    id: &str,
    ser_index: usize,
    spec: &TrendlineWrite,
    i: usize,
) -> Result<(), Error> {
    let ty = inspect_type(space, false);
    if !supports_trendline(&ty) {
        return Err(Error::with_path(
            ErrorKind::TrendlineNotSupported,
            format!("ops[{i}].id"),
        ));
    }
    let kind = spec.kind.as_deref().ok_or_else(|| {
        Error::with_path(ErrorKind::InvalidTrendlineType, format!("ops[{i}].type"))
    })?;
    check_trendline_fields(kind, spec.order, spec.period, i)?;
    let n = series_count(space);
    if ser_index >= n {
        return Err(Error::with_path(
            ErrorKind::ChartSeriesNotFound { id: id.to_string() },
            format!("ops[{i}].id"),
        ));
    }
    let el = build_trendline(spec, kind);
    with_ser_mut(space, ser_index, |ser| {
        insert_before_data(
            ser,
            el,
            &[
                "errBars",
                "cat",
                "val",
                "xVal",
                "yVal",
                "bubbleSize",
                "extLst",
            ],
        );
    });
    Ok(())
}

pub fn set_trendline(
    space: &mut Element,
    id: &str,
    ser_index: usize,
    tl_index: usize,
    spec: &TrendlineWrite,
    i: usize,
) -> Result<(), Error> {
    let sers = collect_sers(space);
    let Some(ser) = sers.get(ser_index) else {
        return Err(Error::with_path(
            ErrorKind::ChartSeriesNotFound { id: id.to_string() },
            format!("ops[{i}].id"),
        ));
    };
    let tls: Vec<&Element> = ser
        .element_children()
        .filter(|c| c.local_name() == "trendline")
        .collect();
    let Some(el) = tls.get(tl_index.saturating_sub(1)).copied() else {
        return Err(Error::with_path(
            ErrorKind::TrendlineNotFound { id: id.to_string() },
            format!("ops[{i}].id"),
        ));
    };
    let existing = el
        .child_by_local("trendlineType")
        .and_then(|e| e.attr_local("val"))
        .unwrap_or("linear")
        .to_string();
    drop(tls);
    drop(sers);
    let kind = spec.kind.as_deref().unwrap_or(existing.as_str());
    check_trendline_fields(kind, spec.order, spec.period, i)?;
    let found = with_ser_mut(space, ser_index, |ser| {
        let Some(el) = nth_child_local_mut(ser, "trendline", tl_index) else {
            return false;
        };
        if let Some(kind) = spec.kind.as_deref() {
            set_val_child(el, "trendlineType", kind);
            if kind != "poly" {
                remove_local(el, "order");
            }
            if kind != "movingAvg" {
                remove_local(el, "period");
            }
            if kind == "poly" && el.child_by_local("order").is_none() {
                el.push_element(val_el("c:order", spec.order.unwrap_or(2).to_string()));
            }
            if kind == "movingAvg" && el.child_by_local("period").is_none() {
                el.push_element(val_el("c:period", spec.period.unwrap_or(2).to_string()));
            }
        }
        if let Some(order) = spec.order {
            set_val_child(el, "order", order.to_string());
        }
        if let Some(period) = spec.period {
            set_val_child(el, "period", period.to_string());
        }
        if let Some(v) = spec.display_r_squared {
            set_val_child(el, "dispRSqr", if v { "1" } else { "0" });
        }
        if let Some(v) = spec.display_equation {
            set_val_child(el, "dispEq", if v { "1" } else { "0" });
        }
        if let Some(name) = &spec.name {
            set_text_child(el, "name", name);
        }
        true
    })
    .unwrap_or(false);
    if !found {
        return Err(Error::with_path(
            ErrorKind::TrendlineNotFound { id: id.to_string() },
            format!("ops[{i}].id"),
        ));
    }
    Ok(())
}

pub fn delete_trendline(
    space: &mut Element,
    id: &str,
    ser_index: usize,
    tl_index: usize,
    i: usize,
) -> Result<(), Error> {
    if ser_index >= series_count(space) {
        return Err(Error::with_path(
            ErrorKind::ChartSeriesNotFound { id: id.to_string() },
            format!("ops[{i}].id"),
        ));
    }
    let found = with_ser_mut(space, ser_index, |ser| {
        remove_nth_local(ser, "trendline", tl_index)
    })
    .unwrap_or(false);
    if !found {
        return Err(Error::with_path(
            ErrorKind::TrendlineNotFound { id: id.to_string() },
            format!("ops[{i}].id"),
        ));
    }
    Ok(())
}

pub fn insert_error_bars(
    space: &mut Element,
    id: &str,
    ser_index: usize,
    spec: &ErrorBarsWrite,
    i: usize,
) -> Result<(), Error> {
    let ty = inspect_type(space, false);
    if !supports_error_bars(&ty) {
        return Err(Error::with_path(
            ErrorKind::ErrorBarsNotSupported,
            format!("ops[{i}].id"),
        ));
    }
    if ser_index >= series_count(space) {
        return Err(Error::with_path(
            ErrorKind::ChartSeriesNotFound { id: id.to_string() },
            format!("ops[{i}].id"),
        ));
    }
    let dir = spec.dir.as_deref().unwrap_or("y");
    let bar_type = spec.bar_type.as_deref().unwrap_or("both");
    let val_type = spec.val_type.as_deref().unwrap_or("stdErr");
    check_error_bars_fields(dir, bar_type, val_type, spec.value, true, i)?;
    let dup = with_ser_mut(space, ser_index, |ser| {
        ser.element_children().any(|c| {
            c.local_name() == "errBars"
                && c.child_by_local("errDir")
                    .and_then(|e| e.attr_local("val"))
                    .unwrap_or("y")
                    == dir
        })
    })
    .unwrap_or(false);
    if dup {
        return Err(Error::with_path(
            ErrorKind::ErrorBarsDuplicateDir,
            format!("ops[{i}].dir"),
        ));
    }
    let el = build_error_bars(dir, bar_type, val_type, spec.value);
    with_ser_mut(space, ser_index, |ser| {
        insert_before_data(
            ser,
            el,
            &["cat", "val", "xVal", "yVal", "bubbleSize", "extLst"],
        );
    });
    Ok(())
}

pub fn set_error_bars(
    space: &mut Element,
    id: &str,
    ser_index: usize,
    eb_index: usize,
    spec: &ErrorBarsWrite,
    i: usize,
) -> Result<(), Error> {
    let sers = collect_sers(space);
    let Some(ser) = sers.get(ser_index) else {
        return Err(Error::with_path(
            ErrorKind::ChartSeriesNotFound { id: id.to_string() },
            format!("ops[{i}].id"),
        ));
    };
    let bars: Vec<&Element> = ser
        .element_children()
        .filter(|c| c.local_name() == "errBars")
        .collect();
    let Some(el) = bars.get(eb_index.saturating_sub(1)).copied() else {
        return Err(Error::with_path(
            ErrorKind::ErrorBarsNotFound { id: id.to_string() },
            format!("ops[{i}].id"),
        ));
    };
    let dir = spec.dir.clone().unwrap_or_else(|| {
        el.child_by_local("errDir")
            .and_then(|e| e.attr_local("val"))
            .unwrap_or("y")
            .to_string()
    });
    let bar_type = spec.bar_type.clone().unwrap_or_else(|| {
        el.child_by_local("errBarType")
            .and_then(|e| e.attr_local("val"))
            .unwrap_or("both")
            .to_string()
    });
    let val_type = spec.val_type.clone().unwrap_or_else(|| {
        el.child_by_local("errValType")
            .and_then(|e| e.attr_local("val"))
            .unwrap_or("stdErr")
            .to_string()
    });
    let value = spec.value.or_else(|| {
        el.child_by_local("val")
            .and_then(|e| e.attr_local("val"))
            .and_then(|s| s.parse().ok())
    });
    drop(bars);
    drop(sers);
    let require_value_rules = spec.val_type.is_some() || spec.value.is_some();
    check_error_bars_fields(
        &dir,
        &bar_type,
        &val_type,
        if spec.val_type.is_some() || spec.value.is_some() {
            spec.value.or(value)
        } else {
            spec.value
        },
        require_value_rules,
        i,
    )?;
    let found = with_ser_mut(space, ser_index, |ser| {
        let Some(el) = nth_child_local_mut(ser, "errBars", eb_index) else {
            return false;
        };
        if let Some(d) = spec.dir.as_deref() {
            set_val_child(el, "errDir", d);
        }
        if let Some(b) = spec.bar_type.as_deref() {
            set_val_child(el, "errBarType", b);
        }
        if let Some(v) = spec.val_type.as_deref() {
            set_val_child(el, "errValType", v);
            if v == "stdErr" {
                remove_local(el, "val");
            }
        }
        if let Some(v) = spec.value {
            set_val_child(el, "val", format_num(v));
        }
        true
    })
    .unwrap_or(false);
    if !found {
        return Err(Error::with_path(
            ErrorKind::ErrorBarsNotFound { id: id.to_string() },
            format!("ops[{i}].id"),
        ));
    }
    Ok(())
}

pub fn delete_error_bars(
    space: &mut Element,
    id: &str,
    ser_index: usize,
    eb_index: usize,
    i: usize,
) -> Result<(), Error> {
    if ser_index >= series_count(space) {
        return Err(Error::with_path(
            ErrorKind::ChartSeriesNotFound { id: id.to_string() },
            format!("ops[{i}].id"),
        ));
    }
    let found = with_ser_mut(space, ser_index, |ser| {
        remove_nth_local(ser, "errBars", eb_index)
    })
    .unwrap_or(false);
    if !found {
        return Err(Error::with_path(
            ErrorKind::ErrorBarsNotFound { id: id.to_string() },
            format!("ops[{i}].id"),
        ));
    }
    Ok(())
}

pub fn set_series_axis(
    space: &mut Element,
    id: &str,
    ser_index: usize,
    secondary: bool,
    i: usize,
) -> Result<(), Error> {
    let ty = inspect_type(space, false);
    if !supports_secondary_axis(&ty) {
        return Err(Error::with_path(
            ErrorKind::SecondaryAxisNotSupported,
            format!("ops[{i}].id"),
        ));
    }
    if series_count(space) <= ser_index {
        return Err(Error::with_path(
            ErrorKind::ChartSeriesNotFound { id: id.to_string() },
            format!("ops[{i}].id"),
        ));
    }
    if series_on_secondary(space, ser_index) == secondary {
        return Ok(());
    }
    let scatter = super::uses_xy_axes(&ty);
    ensure_axes_and_move(space, ser_index, secondary, scatter)?;
    Ok(())
}

fn ensure_axes_and_move(
    space: &mut Element,
    ser_index: usize,
    secondary: bool,
    scatter: bool,
) -> Result<(), Error> {
    let (cat_id, primary_val, wrap_name) = {
        let Some(plot) = plot_area(space) else {
            return Ok(());
        };
        let cat_id = primary_cat_id(plot, scatter).unwrap_or_else(|| "1".into());
        let primary_val = primary_val_id(plot, scatter).unwrap_or_else(|| "2".into());
        let loc = ser_location(plot, ser_index);
        let Some((wrap_i, _)) = loc else {
            return Ok(());
        };
        let wrap_name = match &plot.children.get(wrap_i) {
            Some(Node::Element(e)) => e.local_name().to_string(),
            _ => return Ok(()),
        };
        (cat_id, primary_val, wrap_name)
    };

    let (target_cat, target_val) = if secondary {
        if scatter {
            (
                cat_id.clone(),
                ensure_overlay_val(space, &cat_id, &primary_val, scatter),
            )
        } else {
            ensure_secondary_pair(space, &cat_id, &primary_val)
        }
    } else {
        (cat_id.clone(), primary_val.clone())
    };

    let Some(plot) = plot_area_mut(space) else {
        return Ok(());
    };
    let Some((wrap_i, local_i)) = ser_location(plot, ser_index) else {
        return Ok(());
    };
    let ser_count_in_wrap = wrap_ser_count(plot, wrap_i);
    if ser_count_in_wrap == 1 {
        if let Node::Element(w) = &mut plot.children[wrap_i] {
            retarget_wrapper_axes(w, &target_cat, &target_val, scatter);
        }
        return Ok(());
    }
    let ser = take_ser_from_wrap(plot, wrap_i, local_i)?;
    if let Some(dest) = find_wrap_with_val(plot, &wrap_name, &target_val, scatter) {
        if let Node::Element(w) = &mut plot.children[dest] {
            push_ser_before_axid(w, ser);
        }
        return Ok(());
    }
    let mut nw = match &plot.children[wrap_i] {
        Node::Element(w) => {
            let mut c = w.clone();
            c.children.retain(|n| match n {
                Node::Element(e) => e.local_name() != "ser",
                _ => true,
            });
            retarget_wrapper_axes(&mut c, &target_cat, &target_val, scatter);
            push_ser_before_axid(&mut c, ser);
            c
        }
        _ => return Ok(()),
    };
    let insert_at = wrap_i + 1;
    let _ = &mut nw;
    plot.children.insert(insert_at, Node::Element(nw));
    Ok(())
}

fn ensure_overlay_val(
    space: &mut Element,
    cat_id: &str,
    primary_val: &str,
    scatter: bool,
) -> String {
    let (next, template) = {
        let Some(plot) = plot_area(space) else {
            return primary_val.to_string();
        };
        for c in plot.element_children() {
            if !matches!(c.local_name(), "valAx" | "dateAx") {
                continue;
            }
            if bool_val(c.child_by_local("overlay")) {
                if let Some(id) = c.child_by_local("axId").and_then(|e| e.attr_local("val")) {
                    return id.to_string();
                }
            }
        }
        let next = next_ax_id(plot);
        let template = plot
            .element_children()
            .find(|c| {
                matches!(c.local_name(), "valAx" | "dateAx")
                    && c.child_by_local("axId").and_then(|e| e.attr_local("val"))
                        == Some(primary_val)
            })
            .cloned();
        (next, template)
    };
    let Some(mut ax) = template else {
        return primary_val.to_string();
    };
    if let Some(id_el) = ax.child_by_local_mut("axId") {
        id_el.set_attr("val", next.to_string());
    }
    set_val_child(&mut ax, "overlay", "1");
    set_val_child(&mut ax, "crosses", "max");
    set_val_child(&mut ax, "crossAx", cat_id);
    if let Some(pos) = ax.child_by_local_mut("axPos") {
        let cur = pos.attr_local("val").unwrap_or("l");
        let flipped = match cur {
            "l" => "r",
            "r" => "l",
            "b" => "t",
            "t" => "b",
            _ => "r",
        };
        pos.set_attr("val", flipped);
    }
    remove_local(&mut ax, "title");
    remove_local(&mut ax, "majorGridlines");
    let _ = scatter;
    if let Some(plot) = plot_area_mut(space) {
        plot.push_element(ax);
    }
    next.to_string()
}

/// Excel / Word / PowerPoint 的次轴柱形是「隐藏分类轴 + 右侧数值轴」一对，
/// 不能只给第二包裹共用主分类轴。
fn ensure_secondary_pair(
    space: &mut Element,
    primary_cat: &str,
    primary_val: &str,
) -> (String, String) {
    let found = {
        let Some(plot) = plot_area(space) else {
            return (primary_cat.to_string(), primary_val.to_string());
        };
        existing_secondary_pair(plot, primary_cat, primary_val)
    };
    if let Some(pair) = found {
        return pair;
    }
    let (cat_new, val_new, cat_tpl, val_tpl) = {
        let Some(plot) = plot_area(space) else {
            return (primary_cat.to_string(), primary_val.to_string());
        };
        let base = next_ax_id(plot);
        let cat_new = base.to_string();
        let val_new = (base + 1).to_string();
        let cat_tpl = plot
            .element_children()
            .find(|c| {
                matches!(c.local_name(), "catAx" | "dateAx")
                    && c.child_by_local("axId").and_then(|e| e.attr_local("val"))
                        == Some(primary_cat)
            })
            .cloned();
        let val_tpl = plot
            .element_children()
            .find(|c| {
                matches!(c.local_name(), "valAx" | "dateAx")
                    && c.child_by_local("axId").and_then(|e| e.attr_local("val"))
                        == Some(primary_val)
            })
            .cloned();
        (cat_new, val_new, cat_tpl, val_tpl)
    };
    let Some(mut cat_ax) = cat_tpl else {
        return (
            primary_cat.to_string(),
            ensure_overlay_val(space, primary_cat, primary_val, false),
        );
    };
    let Some(mut val_ax) = val_tpl else {
        return (
            primary_cat.to_string(),
            ensure_overlay_val(space, primary_cat, primary_val, false),
        );
    };
    if let Some(id_el) = cat_ax.child_by_local_mut("axId") {
        id_el.set_attr("val", &cat_new);
    }
    set_val_child(&mut cat_ax, "delete", "1");
    set_val_child(&mut cat_ax, "crossAx", &val_new);
    remove_local(&mut cat_ax, "crosses");
    remove_local(&mut cat_ax, "title");
    if let Some(id_el) = val_ax.child_by_local_mut("axId") {
        id_el.set_attr("val", &val_new);
    }
    set_val_child(&mut val_ax, "crosses", "max");
    set_val_child(&mut val_ax, "crossAx", &cat_new);
    remove_local(&mut val_ax, "overlay");
    if let Some(pos) = val_ax.child_by_local_mut("axPos") {
        pos.set_attr("val", "r");
    }
    remove_local(&mut val_ax, "title");
    remove_local(&mut val_ax, "majorGridlines");
    if let Some(plot) = plot_area_mut(space) {
        plot.push_element(val_ax);
        plot.push_element(cat_ax);
    }
    (cat_new, val_new)
}

fn existing_secondary_pair(
    plot: &Element,
    primary_cat: &str,
    primary_val: &str,
) -> Option<(String, String)> {
    for c in plot.element_children() {
        if !matches!(c.local_name(), "valAx" | "dateAx") {
            continue;
        }
        let id = c.child_by_local("axId").and_then(|e| e.attr_local("val"))?;
        if id == primary_val {
            continue;
        }
        let pos = c.child_by_local("axPos").and_then(|e| e.attr_local("val"));
        let crosses = c
            .child_by_local("crosses")
            .and_then(|e| e.attr_local("val"));
        let overlay = bool_val(c.child_by_local("overlay"));
        if pos != Some("r") && crosses != Some("max") && !overlay {
            continue;
        }
        let cat = c
            .child_by_local("crossAx")
            .and_then(|e| e.attr_local("val"))
            .unwrap_or(primary_cat);
        if cat == primary_cat {
            continue;
        }
        return Some((cat.to_string(), id.to_string()));
    }
    None
}

fn check_trendline_fields(
    kind: &str,
    order: Option<u32>,
    period: Option<u32>,
    i: usize,
) -> Result<(), Error> {
    if !TREND_TYPES.contains(&kind) {
        return Err(Error::with_path(
            ErrorKind::InvalidTrendlineType,
            format!("ops[{i}].type"),
        ));
    }
    if order.is_some() && kind != "poly" {
        return Err(Error::with_path(
            ErrorKind::TrendlineOrderNotPoly,
            format!("ops[{i}].order"),
        ));
    }
    if period.is_some() && kind != "movingAvg" {
        return Err(Error::with_path(
            ErrorKind::TrendlinePeriodNotMovingAvg,
            format!("ops[{i}].period"),
        ));
    }
    Ok(())
}

fn check_error_bars_fields(
    dir: &str,
    bar_type: &str,
    val_type: &str,
    value: Option<f64>,
    require_value_rules: bool,
    i: usize,
) -> Result<(), Error> {
    if !matches!(dir, "x" | "y") {
        return Err(Error::with_path(
            ErrorKind::InvalidErrorBars,
            format!("ops[{i}].dir"),
        ));
    }
    if !matches!(bar_type, "both" | "plus" | "minus") {
        return Err(Error::with_path(
            ErrorKind::InvalidErrorBars,
            format!("ops[{i}].barType"),
        ));
    }
    if matches!(val_type, "cust" | "lCaps") {
        return Err(Error::with_path(
            ErrorKind::CustomErrorBars,
            format!("ops[{i}].valType"),
        ));
    }
    if !matches!(val_type, "stdErr" | "stdDev" | "fixedVal" | "percentage") {
        return Err(Error::with_path(
            ErrorKind::InvalidErrorBars,
            format!("ops[{i}].valType"),
        ));
    }
    if require_value_rules {
        if matches!(val_type, "stdDev" | "fixedVal" | "percentage") && value.is_none() {
            return Err(Error::with_path(
                ErrorKind::NeedErrorBarsValue,
                format!("ops[{i}].value"),
            ));
        }
        if val_type == "stdErr" && value.is_some() {
            return Err(Error::with_path(
                ErrorKind::ErrorBarsValueOnStdErr,
                format!("ops[{i}].value"),
            ));
        }
    }
    Ok(())
}

fn build_trendline(spec: &TrendlineWrite, kind: &str) -> Element {
    let mut el = Element::new("c:trendline");
    if let Some(name) = &spec.name {
        let mut n = Element::new("c:name");
        n.set_text(name);
        el.push_element(n);
    }
    el.push_element(val_el("c:trendlineType", kind));
    if kind == "poly" {
        el.push_element(val_el("c:order", spec.order.unwrap_or(2).to_string()));
    }
    if kind == "movingAvg" {
        el.push_element(val_el("c:period", spec.period.unwrap_or(2).to_string()));
    }
    el.push_element(val_el(
        "c:dispRSqr",
        if spec.display_r_squared.unwrap_or(false) {
            "1"
        } else {
            "0"
        },
    ));
    el.push_element(val_el(
        "c:dispEq",
        if spec.display_equation.unwrap_or(false) {
            "1"
        } else {
            "0"
        },
    ));
    el
}

fn build_error_bars(dir: &str, bar_type: &str, val_type: &str, value: Option<f64>) -> Element {
    let mut el = Element::new("c:errBars");
    el.push_element(val_el("c:errDir", dir));
    el.push_element(val_el("c:errBarType", bar_type));
    el.push_element(val_el("c:errValType", val_type));
    el.push_element(val_el("c:noEndCap", "0"));
    if let Some(v) = value {
        el.push_element(val_el("c:val", format_num(v)));
    }
    el
}

fn insert_before_data(ser: &mut Element, el: Element, stops: &[&str]) {
    let at = ser
        .children
        .iter()
        .position(|n| matches!(n, Node::Element(e) if stops.contains(&e.local_name())));
    match at {
        Some(i) => ser.children.insert(i, Node::Element(el)),
        None => ser.push_element(el),
    }
}

fn nth_child_local_mut<'a>(
    ser: &'a mut Element,
    local: &str,
    index_from_one: usize,
) -> Option<&'a mut Element> {
    let mut k = 0usize;
    for c in ser.element_children_mut() {
        if c.local_name() == local {
            k += 1;
            if k == index_from_one {
                return Some(c);
            }
        }
    }
    None
}

fn remove_nth_local(ser: &mut Element, local: &str, index_from_one: usize) -> bool {
    let mut k = 0usize;
    let mut idx = None;
    for (i, n) in ser.children.iter().enumerate() {
        if let Node::Element(e) = n {
            if e.local_name() == local {
                k += 1;
                if k == index_from_one {
                    idx = Some(i);
                    break;
                }
            }
        }
    }
    if let Some(i) = idx {
        ser.children.remove(i);
        true
    } else {
        false
    }
}

fn set_val_child(parent: &mut Element, local: &str, val: impl Into<String>) {
    let val = val.into();
    if let Some(c) = parent.child_by_local_mut(local) {
        c.set_attr("val", val);
        return;
    }
    parent.push_element(val_el(format!("c:{local}"), val));
}

fn set_text_child(parent: &mut Element, local: &str, text: &str) {
    if let Some(c) = parent.child_by_local_mut(local) {
        c.set_text(text);
        return;
    }
    let mut n = Element::new(format!("c:{local}"));
    n.set_text(text);
    parent.push_element(n);
}

fn remove_local(parent: &mut Element, local: &str) {
    parent.children.retain(|n| match n {
        Node::Element(e) => e.local_name() != local,
        _ => true,
    });
}

fn val_el(name: impl Into<String>, val: impl Into<String>) -> Element {
    Element::new(name).with_attr("val", val)
}

fn child_u32(el: &Element, local: &str) -> Option<u32> {
    el.child_by_local(local)
        .and_then(|e| e.attr_local("val"))
        .and_then(|s| s.parse().ok())
}

fn format_num(v: f64) -> String {
    if v.fract() == 0.0 && v.abs() < 1e15 {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}

fn wrap_of_series(plot: &Element, index: usize) -> Option<&Element> {
    let mut seen = 0usize;
    for w in plot.element_children() {
        if !WRAPPERS.contains(&w.local_name()) {
            continue;
        }
        for c in w.element_children() {
            if c.local_name() == "ser" {
                if seen == index {
                    return Some(w);
                }
                seen += 1;
            }
        }
    }
    None
}

fn wrapper_ax_ids(wrap: &Element) -> Vec<String> {
    wrap.element_children()
        .filter(|c| c.local_name() == "axId")
        .filter_map(|c| c.attr_local("val").map(str::to_string))
        .collect()
}

fn axis_by_id<'a>(plot: &'a Element, id: &str) -> Option<&'a Element> {
    plot.element_children().find(|c| {
        matches!(c.local_name(), "catAx" | "valAx" | "dateAx" | "serAx")
            && c.child_by_local("axId").and_then(|e| e.attr_local("val")) == Some(id)
    })
}

fn primary_cat_id(plot: &Element, scatter: bool) -> Option<String> {
    if scatter {
        return plot
            .element_children()
            .find(|c| c.local_name() == "valAx" && !bool_val(c.child_by_local("overlay")))
            .and_then(|a| {
                a.child_by_local("axId")
                    .and_then(|e| e.attr_local("val").map(str::to_string))
            });
    }
    plot.element_children()
        .find(|c| matches!(c.local_name(), "catAx" | "dateAx"))
        .and_then(|a| {
            a.child_by_local("axId")
                .and_then(|e| e.attr_local("val").map(str::to_string))
        })
}

fn primary_val_id(plot: &Element, scatter: bool) -> Option<String> {
    let mut vals = plot.element_children().filter(|c| {
        matches!(c.local_name(), "valAx" | "dateAx") && !bool_val(c.child_by_local("overlay"))
    });
    let first = vals.next()?;
    let ax = if scatter {
        vals.next().unwrap_or(first)
    } else {
        first
    };
    ax.child_by_local("axId")
        .and_then(|e| e.attr_local("val").map(str::to_string))
}

fn next_ax_id(plot: &Element) -> i64 {
    let mut max = 2i64;
    for c in plot.element_children() {
        if let Some(v) = c
            .child_by_local("axId")
            .and_then(|e| e.attr_local("val"))
            .and_then(|s| s.parse::<i64>().ok())
        {
            max = max.max(v);
        }
        for a in c.element_children() {
            if a.local_name() == "axId" {
                if let Some(v) = a.attr_local("val").and_then(|s| s.parse::<i64>().ok()) {
                    max = max.max(v);
                }
            }
        }
    }
    max + 1
}

fn ser_location(plot: &Element, index: usize) -> Option<(usize, usize)> {
    let mut seen = 0usize;
    for (wi, n) in plot.children.iter().enumerate() {
        let Node::Element(w) = n else { continue };
        if !WRAPPERS.contains(&w.local_name()) {
            continue;
        }
        let mut local = 0usize;
        for c in w.element_children() {
            if c.local_name() == "ser" {
                if seen == index {
                    return Some((wi, local));
                }
                seen += 1;
                local += 1;
            }
        }
    }
    None
}

fn wrap_ser_count(plot: &Element, wrap_i: usize) -> usize {
    match &plot.children.get(wrap_i) {
        Some(Node::Element(w)) => w
            .element_children()
            .filter(|c| c.local_name() == "ser")
            .count(),
        _ => 0,
    }
}

fn take_ser_from_wrap(plot: &mut Element, wrap_i: usize, local_i: usize) -> Result<Element, Error> {
    let Node::Element(w) = &mut plot.children[wrap_i] else {
        return Err(Error::new(ErrorKind::XmlMissing {
            parent: "plotArea".into(),
            child: "ser".into(),
        }));
    };
    let mut k = 0usize;
    let mut idx = None;
    for (i, n) in w.children.iter().enumerate() {
        if let Node::Element(e) = n {
            if e.local_name() == "ser" {
                if k == local_i {
                    idx = Some(i);
                    break;
                }
                k += 1;
            }
        }
    }
    let i = idx.ok_or_else(|| {
        Error::new(ErrorKind::XmlMissing {
            parent: "plotArea".into(),
            child: "ser".into(),
        })
    })?;
    match w.children.remove(i) {
        Node::Element(e) => Ok(e),
        _ => Err(Error::new(ErrorKind::XmlMissing {
            parent: "plotArea".into(),
            child: "ser".into(),
        })),
    }
}

fn find_wrap_with_val(
    plot: &Element,
    wrap_name: &str,
    val_id: &str,
    scatter: bool,
) -> Option<usize> {
    for (i, n) in plot.children.iter().enumerate() {
        let Node::Element(w) = n else { continue };
        if w.local_name() != wrap_name {
            continue;
        }
        if wrap_value_axid(w, scatter).as_deref() == Some(val_id) {
            return Some(i);
        }
    }
    None
}

fn wrap_value_axid(wrap: &Element, scatter: bool) -> Option<String> {
    let ids: Vec<String> = wrapper_ax_ids(wrap);
    if scatter {
        ids.get(1).cloned().or_else(|| ids.last().cloned())
    } else {
        ids.last().cloned()
    }
}

fn retarget_value_axid(wrap: &mut Element, new_val: &str, scatter: bool) {
    let ids = wrapper_ax_ids(wrap);
    let target_old = if scatter {
        ids.get(1).cloned().or_else(|| ids.last().cloned())
    } else {
        ids.last().cloned()
    };
    let Some(old) = target_old else {
        return;
    };
    for c in wrap.element_children_mut() {
        if c.local_name() == "axId" && c.attr_local("val") == Some(old.as_str()) {
            c.set_attr("val", new_val);
            return;
        }
    }
}

fn retarget_wrapper_axes(wrap: &mut Element, cat: &str, val: &str, scatter: bool) {
    if scatter {
        retarget_value_axid(wrap, val, true);
        return;
    }
    let positions: Vec<usize> = wrap
        .children
        .iter()
        .enumerate()
        .filter_map(|(i, n)| match n {
            Node::Element(e) if e.local_name() == "axId" => Some(i),
            _ => None,
        })
        .collect();
    if positions.len() < 2 {
        retarget_value_axid(wrap, val, false);
        return;
    }
    let first = positions[0];
    let last = positions[positions.len() - 1];
    if let Node::Element(e) = &mut wrap.children[first] {
        e.set_attr("val", cat);
    }
    if let Node::Element(e) = &mut wrap.children[last] {
        e.set_attr("val", val);
    }
}

/// 系列必须排在 dLbls / axId 等后随元素之前，否则 Office 不认次轴。
const AFTER_SER: &[&str] = &[
    "dLbls",
    "gapWidth",
    "overlap",
    "serLines",
    "dropLines",
    "hiLowLines",
    "upDownBars",
    "marker",
    "smooth",
    "bubble3D",
    "bubbleScale",
    "showNegBubbles",
    "sizeRepresents",
    "axId",
    "extLst",
];

fn push_ser_before_axid(wrap: &mut Element, ser: Element) {
    let at = wrap
        .children
        .iter()
        .position(|n| matches!(n, Node::Element(e) if AFTER_SER.contains(&e.local_name())));
    match at {
        Some(i) => wrap.children.insert(i, Node::Element(ser)),
        None => wrap.push_element(ser),
    }
}

pub fn parse_trendline_write(
    obj: &Map<String, Value>,
    i: usize,
    insert: bool,
) -> Result<TrendlineWrite, Error> {
    let kind = match obj.get("type") {
        Some(Value::String(s)) => {
            if TREND_TYPES.contains(&s.as_str()) {
                Some(s.clone())
            } else {
                return Err(Error::with_path(
                    ErrorKind::InvalidTrendlineType,
                    format!("ops[{i}].type"),
                ));
            }
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "type".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].type"),
            ));
        }
        None if insert => {
            return Err(Error::with_path(
                ErrorKind::OpRequiresField {
                    op: "insertTrendline".into(),
                    field: "type".into(),
                },
                format!("ops[{i}].type"),
            ));
        }
        None => None,
    };
    let order = parse_opt_u32(obj, i, "order", 2, Some(6))?;
    let period = parse_opt_u32(obj, i, "period", 2, None)?;
    let display_r_squared = parse_opt_bool(obj, i, "displayRSquared")?;
    let display_equation = parse_opt_bool(obj, i, "displayEquation")?;
    let name = parse_opt_string(obj, i, "name")?;
    if insert {
        if let Some(k) = kind.as_deref() {
            check_trendline_fields(k, order, period, i)?;
        }
    } else if kind.is_none()
        && order.is_none()
        && period.is_none()
        && display_r_squared.is_none()
        && display_equation.is_none()
        && name.is_none()
    {
        return Err(Error::with_path(
            ErrorKind::NeedTrendlineField,
            format!("ops[{i}]"),
        ));
    }
    Ok(TrendlineWrite {
        kind,
        order,
        period,
        display_r_squared,
        display_equation,
        name,
    })
}

pub fn parse_error_bars_write(
    obj: &Map<String, Value>,
    i: usize,
    insert: bool,
) -> Result<ErrorBarsWrite, Error> {
    let dir = parse_opt_enum(obj, i, "dir", &["x", "y"], ErrorKind::InvalidErrorBars)?;
    let bar_type = parse_opt_enum(
        obj,
        i,
        "barType",
        &["both", "plus", "minus"],
        ErrorKind::InvalidErrorBars,
    )?;
    let val_type = match obj.get("valType") {
        Some(Value::String(s)) if matches!(s.as_str(), "cust" | "lCaps") => {
            return Err(Error::with_path(
                ErrorKind::CustomErrorBars,
                format!("ops[{i}].valType"),
            ));
        }
        Some(Value::String(s))
            if matches!(s.as_str(), "stdErr" | "stdDev" | "fixedVal" | "percentage") =>
        {
            Some(s.clone())
        }
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidErrorBars,
                format!("ops[{i}].valType"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "valType".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].valType"),
            ));
        }
        None => None,
    };
    let value = parse_opt_f64(obj, i, "value")?;
    if !insert && dir.is_none() && bar_type.is_none() && val_type.is_none() && value.is_none() {
        return Err(Error::with_path(
            ErrorKind::NeedErrorBarsField,
            format!("ops[{i}]"),
        ));
    }
    if insert {
        let d = dir.as_deref().unwrap_or("y");
        let b = bar_type.as_deref().unwrap_or("both");
        let v = val_type.as_deref().unwrap_or("stdErr");
        check_error_bars_fields(d, b, v, value, true, i)?;
    }
    Ok(ErrorBarsWrite {
        dir,
        bar_type,
        val_type,
        value,
    })
}

pub fn parse_series_axis(obj: &Map<String, Value>, i: usize) -> Result<bool, Error> {
    match obj.get("axis") {
        Some(Value::String(s)) if s == "primary" => Ok(false),
        Some(Value::String(s)) if s == "secondary" => Ok(true),
        Some(Value::String(_)) => Err(Error::with_path(
            ErrorKind::InvalidSeriesAxis,
            format!("ops[{i}].axis"),
        )),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "axis".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].axis"),
        )),
        None => Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setChartSeriesAxis".into(),
                field: "axis".into(),
            },
            format!("ops[{i}].axis"),
        )),
    }
}

fn parse_opt_enum(
    obj: &Map<String, Value>,
    i: usize,
    field: &str,
    allowed: &[&str],
    invalid: ErrorKind,
) -> Result<Option<String>, Error> {
    match obj.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) if allowed.contains(&s.as_str()) => Ok(Some(s.clone())),
        Some(Value::String(_)) => Err(Error::with_path(invalid, format!("ops[{i}].{field}"))),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].{field}"),
        )),
    }
}

fn parse_opt_bool(obj: &Map<String, Value>, i: usize, field: &str) -> Result<Option<bool>, Error> {
    match obj.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Bool(b)) => Ok(Some(*b)),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Boolean,
            },
            format!("ops[{i}].{field}"),
        )),
    }
}

fn parse_opt_string(
    obj: &Map<String, Value>,
    i: usize,
    field: &str,
) -> Result<Option<String>, Error> {
    match obj.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].{field}"),
        )),
    }
}

fn parse_opt_u32(
    obj: &Map<String, Value>,
    i: usize,
    field: &str,
    min: u32,
    max: Option<u32>,
) -> Result<Option<u32>, Error> {
    match obj.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(n)) => {
            let Some(v) = n.as_u64() else {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: field.into(),
                        expected: TypeName::Integer,
                    },
                    format!("ops[{i}].{field}"),
                ));
            };
            if v < u64::from(min) {
                return Err(Error::with_path(
                    if let Some(max) = max {
                        ErrorKind::OutOfRange {
                            field: field.into(),
                            min: i64::from(min),
                            max: i64::from(max),
                        }
                    } else {
                        ErrorKind::MinValue {
                            field: field.into(),
                            min: i64::from(min),
                        }
                    },
                    format!("ops[{i}].{field}"),
                ));
            }
            if let Some(max) = max {
                if v > u64::from(max) {
                    return Err(Error::with_path(
                        ErrorKind::OutOfRange {
                            field: field.into(),
                            min: i64::from(min),
                            max: i64::from(max),
                        },
                        format!("ops[{i}].{field}"),
                    ));
                }
            }
            Ok(Some(v as u32))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Integer,
            },
            format!("ops[{i}].{field}"),
        )),
    }
}

fn parse_opt_f64(obj: &Map<String, Value>, i: usize, field: &str) -> Result<Option<f64>, Error> {
    match obj.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(n)) => n.as_f64().map(Some).ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: field.into(),
                    expected: TypeName::Integer,
                },
                format!("ops[{i}].{field}"),
            )
        }),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Integer,
            },
            format!("ops[{i}].{field}"),
        )),
    }
}
