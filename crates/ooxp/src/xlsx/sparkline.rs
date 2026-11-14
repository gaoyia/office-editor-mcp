//! 工作表 `x14:sparklineGroups`：inspect 名单，apply 写 `extLst`。

use std::collections::HashMap;

use serde_json::{json, Map, Value};

use super::a1::{self, CellRef};
use super::layout;
use super::request::{
    ColorSpec, DeleteSparklineOp, InsertSparklineOp, Patch, SetSparklineOp, SparkAxisType,
    SparkColors, SparkEmpty, SparklineType,
};
use super::style;
use crate::chart;
use crate::error::{Error, ErrorKind, IdReason};
use crate::opc::Package;
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

pub(crate) struct CellCache {
    sheets: HashMap<String, HashMap<(u32, u32), Value>>,
}

impl CellCache {
    pub(crate) fn load(
        pkg: &Package,
        sheets: &[(&str, &str, &str)],
        sst: &[String],
    ) -> Result<Self, Error> {
        let mut out = HashMap::new();
        for (name, part, kind) in sheets {
            if *kind != "worksheet" {
                continue;
            }
            let doc = crate::xml::parse(pkg.get(part)?)?;
            let mut cells = HashMap::new();
            if let Some(data) = doc.root.child_by_local("sheetData") {
                let mut err = None;
                super::formula::for_each_cell(data, |here, el| {
                    if err.is_some() {
                        return;
                    }
                    let t = el.attr("t").unwrap_or("");
                    match super::inspect::classify(
                        t,
                        el.child_by_local("v"),
                        el.child_by_local("is"),
                        el.child_by_local("f").is_some(),
                        sst,
                        &format!("{name}!{}", a1::format_cell(here.col, here.row)),
                    ) {
                        Ok((_, value)) => {
                            cells.insert((here.col, here.row), value);
                        }
                        Err(e) => err = Some(e),
                    }
                });
                if let Some(e) = err {
                    return Err(e);
                }
            }
            out.insert(name.to_ascii_lowercase(), cells);
        }
        Ok(Self { sheets: out })
    }
}

pub(crate) fn inspect(root: &Element, sheet: &str, cells: &CellCache) -> Value {
    let mut out = Vec::new();
    for (i, g) in groups(root).into_iter().enumerate() {
        out.push(inspect_one(g, sheet, i + 1, cells));
    }
    Value::Array(out)
}

fn inspect_one(g: &Element, sheet: &str, n: usize, cells: &CellCache) -> Value {
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
            items.push(inspect_sparkline(el, sheet, cells));
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

fn inspect_sparkline(el: &Element, sheet: &str, cells: &CellCache) -> Value {
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
    let values = if let Value::String(raw) = &formula {
        range_values(raw, sheet, cells)
    } else {
        None
    };
    obj.insert("ref".into(), refer);
    obj.insert("formula".into(), formula);
    if let Some(values) = values {
        obj.insert("values".into(), Value::Array(values));
    }
    Value::Object(obj)
}

/// 公式必须是一张已知表上的连续区域。先按行，再从左到右。没有缓存的格子是 null。
pub(crate) fn range_values(formula: &str, here: &str, cells: &CellCache) -> Option<Vec<Value>> {
    let (named, body) = split_formula(formula)?;
    let sheet = named.unwrap_or_else(|| here.to_string());
    let map = cells.sheets.get(&sheet.to_ascii_lowercase())?;
    let (start, end) = a1::parse_range(body).ok()?;
    let mut out = Vec::new();
    for row in start.row..=end.row {
        for col in start.col..=end.col {
            out.push(map.get(&(col, row)).cloned().unwrap_or(Value::Null));
        }
    }
    Some(out)
}

fn split_formula(raw: &str) -> Option<(Option<String>, &str)> {
    let text = raw.trim().trim_start_matches('=');
    if text.is_empty() {
        return None;
    }
    if let Some(rest) = text.strip_prefix('\'') {
        let mut name = String::new();
        let mut chars = rest.chars();
        while let Some(ch) = chars.next() {
            if ch == '\'' {
                if chars.as_str().starts_with('\'') {
                    name.push('\'');
                    chars.next();
                    continue;
                }
                let body = chars.as_str().strip_prefix('!')?;
                return Some((Some(name), body));
            }
            name.push(ch);
        }
        return None;
    }
    if let Some((name, body)) = text.split_once('!') {
        if name.is_empty() || body.is_empty() || name.contains(':') {
            return None;
        }
        return Some((Some(name.to_string()), body));
    }
    Some((None, text))
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

pub fn export_sparkline_image(bytes: &[u8], id: &str) -> Result<super::ExportedMedia, Error> {
    let (sheet_name, index) = parse_export_id(id)?;
    let pkg = Package::open_bytes(bytes.to_vec())?;
    let wb_part = crate::opc::office_document_part(&pkg)?;
    let wb = crate::xml::parse(pkg.get(&wb_part)?)?;
    let rels = crate::opc::Relationships::from_package(&pkg, &wb_part)?;
    let sheets = super::inspect::list_sheets(&wb.root, &rels, &wb_part, &pkg)?;
    let sst = super::inspect::load_shared_strings(&pkg, &wb_part, &rels)?;
    let refs: Vec<(&str, &str, &str)> = sheets
        .iter()
        .map(|s| (s.name.as_str(), s.part.as_str(), s.kind))
        .collect();
    let cells = CellCache::load(&pkg, &refs, &sst)?;
    let sheet_part = super::picture::lookup_sheet_part(&pkg, &sheet_name, "id")?;
    let doc = crate::xml::parse(pkg.get(&sheet_part)?)?;
    let group = group_at(&doc.root, index, id)?;
    let png = paint_group(&pkg, &sheets, &sheet_name, &doc.root, group, &cells)?;
    Ok(super::ExportedMedia {
        bytes: png,
        content_type: Some("image/png".to_string()),
    })
}

fn parse_export_id(id: &str) -> Result<(String, usize), Error> {
    let bad = |reason| Error::with_path(ErrorKind::InvalidId { reason }, "id");
    let Some(rest) = id.strip_prefix("sheet:") else {
        return Err(bad(IdReason::SheetSparkPrefix));
    };
    let Some((name, nstr)) = rest.split_once("/spark:") else {
        return Err(bad(IdReason::SheetSparkPrefix));
    };
    if name.is_empty() || name.contains('/') {
        return Err(bad(IdReason::SheetSparkPrefix));
    }
    if nstr.is_empty() || !nstr.bytes().all(|b| b.is_ascii_digit()) {
        return Err(bad(IdReason::SparkIndexNotInt));
    }
    let n: usize = nstr.parse().map_err(|_| bad(IdReason::SparkIndexNotInt))?;
    if n == 0 {
        return Err(bad(IdReason::SparkIndexFromOne));
    }
    Ok((name.to_string(), n))
}

fn group_at<'a>(root: &'a Element, index: usize, id: &str) -> Result<&'a Element, Error> {
    let attr = crate::xml::pin_attr("spark");
    let mut live = 0usize;
    for group in groups(root) {
        live += 1;
        if crate::xml::pin_matches(group, &attr, index, live) {
            return Ok(group);
        }
    }
    Err(Error::with_path(
        ErrorKind::SparklineNotFound { id: id.to_string() },
        "id",
    ))
}

fn paint_group(
    pkg: &Package,
    sheets: &[super::inspect::SheetMeta],
    sheet_name: &str,
    sheet: &Element,
    group: &Element,
    cells: &CellCache,
) -> Result<Vec<u8>, Error> {
    if !is_plain_line(group) {
        return Err(image_unsupported());
    }
    let item = only_sparkline(group)?;
    let refer = child_text(item, "sqref").ok_or_else(image_unsupported)?;
    let (host, end) = a1::parse_range(refer.trim()).map_err(|_| image_unsupported())?;
    if host != end || in_merge(sheet, host) {
        return Err(image_unsupported());
    }
    let formula = child_text(item, "f").ok_or_else(image_unsupported)?;
    let cached = range_values(&formula, sheet_name, cells).ok_or_else(image_unsupported)?;
    if !attr_on(group, "displayHidden")
        && formula_touches_hidden(pkg, sheets, sheet_name, sheet, &formula)?
    {
        return Err(image_unsupported());
    }
    let width = layout::col_width_px(sheet, host.col);
    let height = layout::row_height_px(sheet, host.row);
    let values = cached.iter().map(point_of).collect();
    let spec = super::sparkline_paint::LineSpec {
        values,
        empty: empty_mode(group),
        axis_min: axis_end(group, "minAxisType", "manualMin")?,
        axis_max: axis_end(group, "maxAxisType", "manualMax")?,
        color: series_rgb(group)?,
        weight_px: line_weight_px(group)?,
    };
    super::sparkline_paint::render(width, height, &spec)
}

fn is_plain_line(group: &Element) -> bool {
    matches!(
        group.attr("type").map(str::trim),
        None | Some("") | Some("line")
    ) && !attr_on(group, "markers")
        && !attr_on(group, "high")
        && !attr_on(group, "low")
        && !attr_on(group, "first")
        && !attr_on(group, "last")
        && !attr_on(group, "negative")
        && !attr_on(group, "displayXAxis")
        && !attr_on(group, "dateAxis")
        && !attr_on(group, "rightToLeft")
}

fn only_sparkline(group: &Element) -> Result<&Element, Error> {
    let Some(list) = group.child_by_local("sparklines") else {
        return Err(image_unsupported());
    };
    let mut found = None;
    for el in list.children_by_local("sparkline") {
        if found.is_some() {
            return Err(image_unsupported());
        }
        found = Some(el);
    }
    found.ok_or_else(image_unsupported)
}

fn attr_on(el: &Element, name: &str) -> bool {
    el.attr(name).is_some_and(|raw| {
        let text = raw.trim();
        text == "1" || text.eq_ignore_ascii_case("true")
    })
}

fn empty_mode(group: &Element) -> super::sparkline_paint::EmptyCells {
    match display_empty(group) {
        SparkEmpty::Gap => super::sparkline_paint::EmptyCells::Gap,
        SparkEmpty::Span => super::sparkline_paint::EmptyCells::Span,
        SparkEmpty::Zero => super::sparkline_paint::EmptyCells::Zero,
    }
}

fn axis_end(group: &Element, type_attr: &str, manual_attr: &str) -> Result<Option<f64>, Error> {
    match group.attr(type_attr).map(str::trim) {
        None | Some("") | Some("individual") | Some("group") => Ok(None),
        Some("custom") => {
            let raw = group.attr(manual_attr).ok_or_else(image_unsupported)?;
            let n = raw.trim().parse::<f64>().map_err(|_| image_unsupported())?;
            if n.is_finite() {
                Ok(Some(n))
            } else {
                Err(image_unsupported())
            }
        }
        Some(_) => Err(image_unsupported()),
    }
}

fn series_rgb(group: &Element) -> Result<[u8; 3], Error> {
    let Some(el) = group.child_by_local("colorSeries") else {
        return Ok([0x37, 0x60, 0x92]);
    };
    let Some(raw) = el.attr("rgb") else {
        return Err(image_unsupported());
    };
    parse_series_rgb(raw).ok_or_else(image_unsupported)
}

fn parse_series_rgb(raw: &str) -> Option<[u8; 3]> {
    let hex = raw.trim();
    let body = match hex.len() {
        6 => hex,
        8 => {
            let alpha = u8::from_str_radix(&hex[..2], 16).ok()?;
            if alpha != 0xff {
                return None;
            }
            &hex[2..]
        }
        _ => return None,
    };
    let n = u32::from_str_radix(body, 16).ok()?;
    Some([
        ((n >> 16) & 0xff) as u8,
        ((n >> 8) & 0xff) as u8,
        (n & 0xff) as u8,
    ])
}

fn line_weight_px(group: &Element) -> Result<f64, Error> {
    let pt = match group.attr("lineWeight").map(str::trim) {
        None | Some("") => 0.75,
        Some(raw) => raw.parse::<f64>().map_err(|_| image_unsupported())?,
    };
    if pt.is_finite() && pt > 0.0 && pt <= 72.0 {
        Ok(pt * 96.0 / 72.0)
    } else {
        Err(image_unsupported())
    }
}

fn point_of(value: &Value) -> Option<f64> {
    value.as_f64().filter(|n| n.is_finite())
}

fn in_merge(sheet: &Element, cell: CellRef) -> bool {
    let Some(merges) = sheet.child_by_local("mergeCells") else {
        return false;
    };
    for el in merges.children_by_local("mergeCell") {
        let Some(raw) = el.attr("ref") else {
            continue;
        };
        let Ok((start, end)) = a1::parse_range(raw) else {
            continue;
        };
        if start == end {
            continue;
        }
        if cell.col >= start.col
            && cell.col <= end.col
            && cell.row >= start.row
            && cell.row <= end.row
        {
            return true;
        }
    }
    false
}

fn formula_touches_hidden(
    pkg: &Package,
    sheets: &[super::inspect::SheetMeta],
    host: &str,
    host_root: &Element,
    formula: &str,
) -> Result<bool, Error> {
    let Some((named, body)) = split_formula(formula) else {
        return Ok(false);
    };
    let Ok((start, end)) = a1::parse_range(body) else {
        return Ok(false);
    };
    let name = named.unwrap_or_else(|| host.to_string());
    if name.eq_ignore_ascii_case(host) {
        return Ok(range_hidden(host_root, start, end));
    }
    let Some(meta) = sheets.iter().find(|s| s.name.eq_ignore_ascii_case(&name)) else {
        return Ok(false);
    };
    let doc = crate::xml::parse(pkg.get(&meta.part)?)?;
    Ok(range_hidden(&doc.root, start, end))
}

fn range_hidden(sheet: &Element, start: CellRef, end: CellRef) -> bool {
    if let Some(data) = sheet.child_by_local("sheetData") {
        for row in data.children_by_local("row") {
            let Some(n) = row.attr("r").and_then(|s| s.parse::<u32>().ok()) else {
                continue;
            };
            if n < start.row || n > end.row {
                continue;
            }
            if attr_on(row, "hidden") {
                return true;
            }
        }
    }
    for cols in sheet.children_by_local("cols") {
        for col in cols.children_by_local("col") {
            let Some(min) = col.attr("min").and_then(|s| s.parse::<u32>().ok()) else {
                continue;
            };
            let max = col
                .attr("max")
                .and_then(|s| s.parse::<u32>().ok())
                .unwrap_or(min);
            if max < start.col || min > end.col {
                continue;
            }
            if attr_on(col, "hidden") {
                return true;
            }
        }
    }
    false
}

fn image_unsupported() -> Error {
    Error::with_path(ErrorKind::SparklineImageUnsupported, "id")
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
