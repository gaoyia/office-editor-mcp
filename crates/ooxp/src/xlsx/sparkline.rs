//! 工作表 `x14:sparklineGroups`：inspect 名单，apply 写 `extLst`。

use serde_json::{json, Map, Value};

use super::a1::{self, CellRef};
use super::layout;
use super::request::{
    ColorSpec, DeleteSparklineOp, InsertSparklineOp, Patch, SetSparklineOp, SparkAxisType,
    SparkColors, SparkEmpty, SparklineType,
};
use super::style;
use crate::chart;
use crate::error::{Error, ErrorKind};
use crate::xml::{Element, Node};

/// MS-XLSX §2.2.4.12。Excel 16.0 另存中间一段为小写 `4fd2`。
const SPARK_URI: &str = "{05C60535-1F16-4fd2-B633-F4F36F0B64E0}";
const X14_NS: &str = "http://schemas.microsoft.com/office/spreadsheetml/2009/9/main";
const XM_NS: &str = "http://schemas.microsoft.com/office/excel/2006/main";

const COLOR_SERIES: &str = "FF376092";
const COLOR_NEG: &str = "FFD00000";
const COLOR_AXIS: &str = "FF000000";

pub(crate) fn count(root: &Element) -> usize {
    groups(root).len()
}

pub(crate) fn inspect(root: &Element, sheet: &str) -> Value {
    let mut out = Vec::new();
    for (i, g) in groups(root).into_iter().enumerate() {
        out.push(inspect_one(g, sheet, i + 1));
    }
    Value::Array(out)
}

fn inspect_one(g: &Element, sheet: &str, n: usize) -> Value {
    let mut obj = Map::new();
    obj.insert("id".into(), json!(format!("sheet:{sheet}/spark:{n}")));
    obj.insert("index".into(), json!(n));
    obj.insert("type".into(), json!(spark_type_str(g)));
    obj.insert(
        "displayEmptyCellsAs".into(),
        json!(display_empty(g).as_str()),
    );
    for name in [
        "markers",
        "high",
        "low",
        "first",
        "last",
        "negative",
        "displayXAxis",
    ] {
        if crate::xml::attr_is_true(g, name) {
            obj.insert(name.into(), json!(true));
        }
    }
    if let Some(w) = g.attr("lineWeight").and_then(|s| s.parse::<f64>().ok()) {
        obj.insert("lineWeight".into(), json_num(w));
    }
    if let Some(t) = g.attr("minAxisType") {
        if t != "individual" {
            obj.insert("minAxisType".into(), json!(t));
        }
    }
    if let Some(t) = g.attr("maxAxisType") {
        if t != "individual" {
            obj.insert("maxAxisType".into(), json!(t));
        }
    }
    if let Some(v) = g.attr("manualMin").and_then(|s| s.parse::<f64>().ok()) {
        obj.insert("manualMin".into(), json_num(v));
    }
    if let Some(v) = g.attr("manualMax").and_then(|s| s.parse::<f64>().ok()) {
        obj.insert("manualMax".into(), json_num(v));
    }
    if let Some(colors) = inspect_colors(g) {
        obj.insert("colors".into(), colors);
    }
    let mut items = Vec::new();
    if let Some(list) = g.child_by_local("sparklines") {
        for el in list.children_by_local("sparkline") {
            items.push(inspect_sparkline(el));
        }
    }
    obj.insert("sparklines".into(), Value::Array(items));
    Value::Object(obj)
}

fn inspect_colors(g: &Element) -> Option<Value> {
    let mut m = Map::new();
    for (key, local) in [
        ("series", "colorSeries"),
        ("negative", "colorNegative"),
        ("axis", "colorAxis"),
        ("markers", "colorMarkers"),
        ("first", "colorFirst"),
        ("last", "colorLast"),
        ("high", "colorHigh"),
        ("low", "colorLow"),
    ] {
        if let Some(c) = g.child_by_local(local).and_then(style::color_json) {
            m.insert(key.into(), c);
        }
    }
    if m.is_empty() {
        None
    } else {
        Some(Value::Object(m))
    }
}

fn inspect_sparkline(el: &Element) -> Value {
    let refer = match child_text(el, "sqref") {
        Some(raw) => match a1::parse_range(&raw) {
            Ok((a, b)) if a == b => json!(a1::format_range(a, b)),
            Ok(_) => json!(raw),
            Err(_) => Value::Null,
        },
        None => Value::Null,
    };
    let formula = child_text(el, "f")
        .map(Value::String)
        .unwrap_or(Value::Null);
    let mut obj = Map::new();
    obj.insert("ref".into(), refer);
    obj.insert("formula".into(), formula);
    Value::Object(obj)
}

pub(crate) fn apply_insert(
    worksheet: &mut Element,
    sheet: &str,
    op: &InsertSparklineOp,
    i: usize,
) -> Result<(), Error> {
    if overlaps(worksheet, op.host, None) {
        return Err(Error::with_path(
            ErrorKind::SparklineOverlap,
            format!("ops[{i}].ref"),
        ));
    }
    let formula = write_formula(sheet, &op.formula);
    let mut group = new_group(op);
    let mut list = Element::new("x14:sparklines");
    list.push_element(new_sparkline(&op.refer, &formula));
    group.push_element(list);
    ensure_groups(worksheet).push_element(group);
    Ok(())
}

pub(crate) fn apply_set(
    worksheet: &mut Element,
    sheet: &str,
    op: &SetSparklineOp,
    i: usize,
) -> Result<(), Error> {
    let n_children = nth(worksheet, op.index, i, &op.id)?
        .child_by_local("sparklines")
        .map(|p| p.children_by_local("sparkline").len())
        .unwrap_or(0);
    if n_children != 1 && (op.refer.is_some() || matches!(op.formula, Patch::Set(_))) {
        return Err(Error::with_path(
            ErrorKind::SparklineGroupHasMany,
            format!("ops[{i}]"),
        ));
    }
    if let Some((_, host)) = &op.refer {
        if overlaps(worksheet, *host, Some(op.index)) {
            return Err(Error::with_path(
                ErrorKind::SparklineOverlap,
                format!("ops[{i}].ref"),
            ));
        }
    }
    let el = nth_mut(worksheet, op.index, i, &op.id)?;
    if let Some(kind) = op.kind {
        match kind {
            SparklineType::Line => {
                el.remove_attr("type");
            }
            SparklineType::Column | SparklineType::Stacked => {
                el.set_attr("type", kind.as_str());
            }
        }
    }
    if let Some(v) = op.display_empty {
        el.set_attr("displayEmptyCellsAs", v.as_str());
    }
    patch_flag(el, "markers", op.markers);
    patch_flag(el, "high", op.high);
    patch_flag(el, "low", op.low);
    patch_flag(el, "first", op.first);
    patch_flag(el, "last", op.last);
    patch_flag(el, "negative", op.negative);
    patch_flag(el, "displayXAxis", op.display_x_axis);
    match &op.line_weight {
        Patch::Absent => {}
        Patch::Clear => el.remove_attr("lineWeight"),
        Patch::Set(v) => el.set_attr("lineWeight", fmt_spark_num(*v)),
    }
    apply_axis(el, "minAxisType", "manualMin", op.min_axis, &op.manual_min);
    apply_axis(el, "maxAxisType", "manualMax", op.max_axis, &op.manual_max);
    if let Some(colors) = &op.colors {
        overlay_colors(el, colors);
    }
    if op.refer.is_none() && !matches!(op.formula, Patch::Set(_)) {
        return Ok(());
    }
    let spark = el.child_by_local_mut("sparklines").and_then(|p| {
        p.children.iter_mut().find_map(|n| match n {
            Node::Element(e) if e.local_name() == "sparkline" => Some(e),
            _ => None,
        })
    });
    let Some(spark) = spark else {
        return Err(not_found(&op.id, i));
    };
    if let Some((refer, _)) = &op.refer {
        set_child_text(spark, "sqref", "xm:sqref", refer);
    }
    if let Patch::Set(raw) = &op.formula {
        set_child_text(spark, "f", "xm:f", &write_formula(sheet, raw));
    }
    Ok(())
}

pub(crate) fn apply_delete(
    worksheet: &mut Element,
    op: &DeleteSparklineOp,
    i: usize,
) -> Result<(), Error> {
    let Some((ext_i, groups_i, group_i)) = find_path(worksheet, op.index) else {
        return Err(not_found(&op.id, i));
    };
    {
        let Some(ext_lst) = worksheet.child_by_local_mut("extLst") else {
            return Err(not_found(&op.id, i));
        };
        let ext_empty = {
            let Node::Element(ext) = &mut ext_lst.children[ext_i] else {
                return Err(not_found(&op.id, i));
            };
            let groups_empty = {
                let Node::Element(groups) = &mut ext.children[groups_i] else {
                    return Err(not_found(&op.id, i));
                };
                groups.children.remove(group_i);
                groups.children_by_local("sparklineGroup").is_empty()
            };
            if groups_empty {
                ext.children.remove(groups_i);
            }
            !ext.element_children().any(|_| true)
        };
        if ext_empty {
            ext_lst.children.remove(ext_i);
        }
    }
    if worksheet
        .child_by_local("extLst")
        .is_some_and(|e| e.element_children().next().is_none())
    {
        worksheet
            .children
            .retain(|n| !matches!(n, Node::Element(e) if e.local_name() == "extLst"));
    }
    Ok(())
}

fn new_group(op: &InsertSparklineOp) -> Element {
    let mut g = Element::new("x14:sparklineGroup");
    g.set_attr("displayEmptyCellsAs", op.display_empty.as_str());
    if op.kind != SparklineType::Line {
        g.set_attr("type", op.kind.as_str());
    }
    set_flag(&mut g, "markers", op.markers);
    set_flag(&mut g, "high", op.high);
    set_flag(&mut g, "low", op.low);
    set_flag(&mut g, "first", op.first);
    set_flag(&mut g, "last", op.last);
    set_flag(&mut g, "negative", op.negative);
    set_flag(&mut g, "displayXAxis", op.display_x_axis);
    if let Some(w) = op.line_weight {
        g.set_attr("lineWeight", fmt_spark_num(w));
    }
    write_axis_type(&mut g, "minAxisType", op.min_axis);
    write_axis_type(&mut g, "maxAxisType", op.max_axis);
    if let Some(v) = op.manual_min {
        g.set_attr("manualMin", fmt_spark_num(v));
    }
    if let Some(v) = op.manual_max {
        g.set_attr("manualMax", fmt_spark_num(v));
    }
    for (name, rgb, spec) in [
        ("x14:colorSeries", COLOR_SERIES, op.colors.series.as_ref()),
        ("x14:colorNegative", COLOR_NEG, op.colors.negative.as_ref()),
        ("x14:colorAxis", COLOR_AXIS, op.colors.axis.as_ref()),
        ("x14:colorMarkers", COLOR_NEG, op.colors.markers.as_ref()),
        ("x14:colorFirst", COLOR_NEG, op.colors.first.as_ref()),
        ("x14:colorLast", COLOR_NEG, op.colors.last.as_ref()),
        ("x14:colorHigh", COLOR_NEG, op.colors.high.as_ref()),
        ("x14:colorLow", COLOR_NEG, op.colors.low.as_ref()),
    ] {
        g.push_element(spark_color_el(name, spec, rgb));
    }
    g
}

fn new_sparkline(refer: &str, formula: &str) -> Element {
    let mut el = Element::new("x14:sparkline");
    el.push_element(Element::new("xm:f").with_text(formula));
    el.push_element(Element::new("xm:sqref").with_text(refer));
    el
}

fn ensure_groups(worksheet: &mut Element) -> &mut Element {
    if let Some(idx) = find_groups_slot(worksheet) {
        return groups_at_mut(worksheet, idx);
    }
    if worksheet.child_by_local("extLst").is_none() {
        layout::insert_child_ordered(worksheet, Element::new("extLst"), "extLst");
    }
    let ext_lst = worksheet.child_by_local_mut("extLst").unwrap();
    let mut ext = Element::new("ext");
    ext.set_attr("uri", SPARK_URI);
    ext.set_attr("xmlns:x14", X14_NS);
    let mut groups = Element::new("x14:sparklineGroups");
    groups.set_attr("xmlns:xm", XM_NS);
    ext.push_element(groups);
    ext_lst.push_element(ext);
    let idx = find_groups_slot(worksheet).expect("刚写入的 sparklineGroups");
    groups_at_mut(worksheet, idx)
}

fn find_groups_slot(root: &Element) -> Option<(usize, usize)> {
    let ext_lst = root.child_by_local("extLst")?;
    for (ei, n) in ext_lst.children.iter().enumerate() {
        let Node::Element(ext) = n else { continue };
        if ext.local_name() != "ext" {
            continue;
        }
        if !uri_is_sparkline(ext.attr("uri").unwrap_or("")) {
            continue;
        }
        for (gi, gn) in ext.children.iter().enumerate() {
            if matches!(gn, Node::Element(e) if e.local_name() == "sparklineGroups") {
                return Some((ei, gi));
            }
        }
    }
    None
}

fn groups_at_mut(root: &mut Element, idx: (usize, usize)) -> &mut Element {
    let ext_lst = root.child_by_local_mut("extLst").unwrap();
    let Node::Element(ext) = &mut ext_lst.children[idx.0] else {
        panic!("ext 槽位");
    };
    let Node::Element(groups) = &mut ext.children[idx.1] else {
        panic!("sparklineGroups 槽位");
    };
    groups
}

fn groups(root: &Element) -> Vec<&Element> {
    let mut out = Vec::new();
    let Some(ext_lst) = root.child_by_local("extLst") else {
        return out;
    };
    for ext in ext_lst.children_by_local("ext") {
        if !uri_is_sparkline(ext.attr("uri").unwrap_or("")) {
            continue;
        }
        for parent in ext.children_by_local("sparklineGroups") {
            out.extend(parent.children_by_local("sparklineGroup"));
        }
    }
    out
}

fn nth<'a>(root: &'a Element, index: usize, i: usize, id: &str) -> Result<&'a Element, Error> {
    let attr = crate::xml::pin_attr("spark");
    let mut live = 0usize;
    for group in groups(root) {
        live += 1;
        if crate::xml::pin_matches(group, &attr, index, live) {
            return Ok(group);
        }
    }
    Err(not_found(id, i))
}

fn nth_mut<'a>(
    root: &'a mut Element,
    index: usize,
    i: usize,
    id: &str,
) -> Result<&'a mut Element, Error> {
    let Some((ext_i, groups_i, group_i)) = find_path(root, index) else {
        return Err(not_found(id, i));
    };
    let ext_lst = root.child_by_local_mut("extLst").unwrap();
    let Node::Element(ext) = &mut ext_lst.children[ext_i] else {
        return Err(not_found(id, i));
    };
    let Node::Element(groups) = &mut ext.children[groups_i] else {
        return Err(not_found(id, i));
    };
    let Node::Element(g) = &mut groups.children[group_i] else {
        return Err(not_found(id, i));
    };
    Ok(g)
}

fn find_path(root: &Element, index: usize) -> Option<(usize, usize, usize)> {
    let ext_lst = root.child_by_local("extLst")?;
    let mut n = 0usize;
    for (ei, en) in ext_lst.children.iter().enumerate() {
        let Node::Element(ext) = en else { continue };
        if ext.local_name() != "ext" {
            continue;
        }
        if !uri_is_sparkline(ext.attr("uri").unwrap_or("")) {
            continue;
        }
        for (gi, gn) in ext.children.iter().enumerate() {
            let Node::Element(groups) = gn else { continue };
            if groups.local_name() != "sparklineGroups" {
                continue;
            }
            for (si, sn) in groups.children.iter().enumerate() {
                if let Node::Element(e) = sn {
                    if e.local_name() != "sparklineGroup" {
                        continue;
                    }
                    n += 1;
                    if crate::xml::pin_matches(e, &crate::xml::pin_attr("spark"), index, n) {
                        return Some((ei, gi, si));
                    }
                }
            }
        }
    }
    None
}

fn overlaps(root: &Element, host: CellRef, skip: Option<usize>) -> bool {
    let attr = crate::xml::pin_attr("spark");
    for (i, g) in groups(root).into_iter().enumerate() {
        if skip.is_some_and(|wanted| crate::xml::pin_matches(g, &attr, wanted, i + 1)) {
            continue;
        }
        let Some(list) = g.child_by_local("sparklines") else {
            continue;
        };
        for el in list.children_by_local("sparkline") {
            let Some(raw) = child_text(el, "sqref") else {
                continue;
            };
            let Ok((a, b)) = a1::parse_range(&raw) else {
                continue;
            };
            if a.row <= host.row && host.row <= b.row && a.col <= host.col && host.col <= b.col {
                return true;
            }
        }
    }
    false
}

fn uri_is_sparkline(uri: &str) -> bool {
    uri.eq_ignore_ascii_case(SPARK_URI)
}

fn spark_type_str(g: &Element) -> String {
    match g.attr("type").map(str::trim) {
        None | Some("") | Some("line") => "line".into(),
        Some("column") => "column".into(),
        Some("stacked") => "stacked".into(),
        Some(other) => other.to_string(),
    }
}

fn display_empty(g: &Element) -> SparkEmpty {
    match g.attr("displayEmptyCellsAs").map(str::trim) {
        Some("gap") => SparkEmpty::Gap,
        Some("span") => SparkEmpty::Span,
        _ => SparkEmpty::Zero,
    }
}

fn write_formula(sheet: &str, raw: &str) -> String {
    let t = raw.trim();
    if t.contains('!') {
        return t.to_string();
    }
    if let Ok((a, b)) = a1::parse_range(t) {
        return format!("{}!{}", chart::quote_sheet(sheet), a1::format_range(a, b));
    }
    t.to_string()
}

fn child_text(el: &Element, local: &str) -> Option<String> {
    el.child_by_local(local)
        .map(|c| c.text_content())
        .filter(|s| !s.is_empty())
}

fn set_child_text(parent: &mut Element, local: &str, prefixed: &str, text: &str) {
    if let Some(el) = parent.child_by_local_mut(local) {
        el.set_text(text);
        return;
    }
    parent.push_element(Element::new(prefixed).with_text(text));
}

fn not_found(id: &str, i: usize) -> Error {
    Error::with_path(
        ErrorKind::SparklineNotFound { id: id.to_string() },
        format!("ops[{i}].id"),
    )
}

fn set_flag(el: &mut Element, name: &str, on: bool) {
    if on {
        el.set_attr(name, "1");
    }
}

fn patch_flag(el: &mut Element, name: &str, v: Option<bool>) {
    match v {
        None => {}
        Some(true) => el.set_attr(name, "1"),
        Some(false) => el.remove_attr(name),
    }
}

fn json_num(v: f64) -> Value {
    if v.fract() == 0.0 && v.abs() <= (i64::MAX as f64) {
        json!(v as i64)
    } else {
        json!(v)
    }
}

fn fmt_spark_num(v: f64) -> String {
    if v.fract() == 0.0 && v.abs() < 1e15 {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}

fn write_axis_type(el: &mut Element, attr: &str, kind: Option<SparkAxisType>) {
    match kind {
        None | Some(SparkAxisType::Individual) => {
            if kind.is_some() {
                el.remove_attr(attr);
            }
        }
        Some(other) => el.set_attr(attr, other.as_str()),
    }
}

fn apply_axis(
    el: &mut Element,
    type_attr: &str,
    manual_attr: &str,
    kind: Option<SparkAxisType>,
    manual: &Patch<f64>,
) {
    if let Some(kind) = kind {
        match kind {
            SparkAxisType::Individual => el.remove_attr(type_attr),
            SparkAxisType::Group | SparkAxisType::Custom => {
                el.set_attr(type_attr, kind.as_str());
            }
        }
        if kind != SparkAxisType::Custom && matches!(manual, Patch::Absent) {
            el.remove_attr(manual_attr);
        }
    }
    match manual {
        Patch::Absent => {}
        Patch::Clear => el.remove_attr(manual_attr),
        Patch::Set(v) => el.set_attr(manual_attr, fmt_spark_num(*v)),
    }
}

fn spark_color_el(name: &str, spec: Option<&ColorSpec>, default: &str) -> Element {
    match spec {
        Some(spec) => style::color_el(name, spec),
        None => Element::new(name).with_attr("rgb", default),
    }
}

fn overlay_colors(g: &mut Element, colors: &SparkColors) {
    let items = [
        ("colorSeries", "x14:colorSeries", colors.series.as_ref()),
        (
            "colorNegative",
            "x14:colorNegative",
            colors.negative.as_ref(),
        ),
        ("colorAxis", "x14:colorAxis", colors.axis.as_ref()),
        ("colorMarkers", "x14:colorMarkers", colors.markers.as_ref()),
        ("colorFirst", "x14:colorFirst", colors.first.as_ref()),
        ("colorLast", "x14:colorLast", colors.last.as_ref()),
        ("colorHigh", "x14:colorHigh", colors.high.as_ref()),
        ("colorLow", "x14:colorLow", colors.low.as_ref()),
    ];
    for (local, prefixed, spec) in items {
        let Some(spec) = spec else { continue };
        set_named_el(g, local, style::color_el(prefixed, spec));
    }
}

fn set_named_el(parent: &mut Element, local: &str, el: Element) {
    if let Some(i) = parent
        .children
        .iter()
        .position(|n| matches!(n, Node::Element(e) if e.local_name() == local))
    {
        parent.children[i] = Node::Element(el);
    } else {
        parent.push_element(el);
    }
}
