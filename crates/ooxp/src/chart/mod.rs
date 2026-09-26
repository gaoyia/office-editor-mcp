//! `c:chart` 部件（§21.2）：类型、标题、系列 cache、图例与标签。不含 Word embedding，也不含 xlsx 锚点。

use serde_json::{json, Map, Number, Value};

use crate::error::{Error, ErrorKind};
use crate::xml::{self, Element, Node};
use crate::xlsx::a1::{self, CellRef};

pub const CHART_NS: &str = "http://schemas.openxmlformats.org/drawingml/2006/chart";
pub const CHARTEX_NS: &str = "http://schemas.microsoft.com/office/drawing/2014/chartex";
pub const A_NS: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";
pub const REL_NS: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
pub const REL_CHART: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart";
pub const CT_CHART: &str = "application/vnd.openxmlformats-officedocument.drawingml.chart+xml";

pub const DEFAULT_CX: i64 = 5_486_400;
pub const DEFAULT_CY: i64 = 3_200_400;

pub const WRAPPERS: &[&str] = &[
    "barChart",
    "bar3DChart",
    "lineChart",
    "line3DChart",
    "pieChart",
    "pie3DChart",
    "ofPieChart",
    "doughnutChart",
    "areaChart",
    "area3DChart",
    "scatterChart",
    "bubbleChart",
    "radarChart",
    "stockChart",
    "surfaceChart",
    "surface3DChart",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChartType {
    Bar,
    Col,
    Line,
    Pie,
    Area,
    Scatter,
    Doughnut,
    Radar,
    Bubble,
    Combo,
}

impl ChartType {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "bar" => Some(Self::Bar),
            "col" => Some(Self::Col),
            "line" => Some(Self::Line),
            "pie" => Some(Self::Pie),
            "area" => Some(Self::Area),
            "scatter" => Some(Self::Scatter),
            "doughnut" => Some(Self::Doughnut),
            "radar" => Some(Self::Radar),
            "bubble" => Some(Self::Bubble),
            "combo" => Some(Self::Combo),
            _ => None,
        }
    }

    pub fn parse_combo_series(s: &str) -> Option<Self> {
        match s {
            "bar" => Some(Self::Bar),
            "col" => Some(Self::Col),
            "line" => Some(Self::Line),
            "area" => Some(Self::Area),
            "scatter" => Some(Self::Scatter),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Bar => "bar",
            Self::Col => "col",
            Self::Line => "line",
            Self::Pie => "pie",
            Self::Area => "area",
            Self::Scatter => "scatter",
            Self::Doughnut => "doughnut",
            Self::Radar => "radar",
            Self::Bubble => "bubble",
            Self::Combo => "combo",
        }
    }

    pub fn wrapper(self) -> &'static str {
        match self {
            Self::Bar | Self::Col => "barChart",
            Self::Line => "lineChart",
            Self::Pie => "pieChart",
            Self::Area => "areaChart",
            Self::Scatter => "scatterChart",
            Self::Doughnut => "doughnutChart",
            Self::Radar => "radarChart",
            Self::Bubble => "bubbleChart",
            Self::Combo => "barChart",
        }
    }

    pub fn is_xy(self) -> bool {
        matches!(self, Self::Scatter | Self::Bubble)
    }

    pub fn no_axes(self) -> bool {
        matches!(self, Self::Pie | Self::Doughnut)
    }

    pub fn needs_one_series(self) -> bool {
        matches!(self, Self::Pie | Self::Doughnut)
    }
}

/// 哪一类图表 op，用来套分层限制。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChartRestrict {
    Decorate,
    Data,
}

#[derive(Debug, Clone)]
pub struct NewSeries {
    pub name: Option<String>,
    pub categories: Option<Vec<Value>>,
    pub values: Vec<Value>,
    pub sizes: Option<Vec<Value>>,
    pub series_type: Option<ChartType>,
}

#[derive(Debug, Clone)]
pub struct A1Ref {
    pub sheet: Option<String>,
    pub start: CellRef,
    pub end: CellRef,
}

pub fn inspect_type(space: &Element, _chartex: bool) -> String {
    let Some(plot) = plot_area(space) else {
        return "other".into();
    };
    let mut kinds = Vec::new();
    for c in plot.element_children() {
        if let Some(k) = wrapper_kind(c) {
            if !kinds.contains(&k) {
                kinds.push(k);
            }
        }
    }
    if kinds.len() > 1 {
        return "combo".into();
    }
    match kinds.first() {
        Some(k) => (*k).into(),
        None => "other".into(),
    }
}

pub fn wrapper_kind(el: &Element) -> Option<&'static str> {
    match el.local_name() {
        "barChart" | "bar3DChart" => {
            let dir = el
                .child_by_local("barDir")
                .and_then(|d| d.attr_local("val"))
                .unwrap_or("col");
            Some(if dir == "bar" { "bar" } else { "col" })
        }
        "lineChart" | "line3DChart" => Some("line"),
        "pieChart" | "pie3DChart" => Some("pie"),
        "areaChart" | "area3DChart" => Some("area"),
        "scatterChart" => Some("scatter"),
        "doughnutChart" => Some("doughnut"),
        "radarChart" => Some("radar"),
        "bubbleChart" => Some("bubble"),
        "ofPieChart" | "stockChart" | "surfaceChart" | "surface3DChart" => Some("other"),
        _ => None,
    }
}

pub fn chart_title(space: &Element) -> Option<String> {
    let chart = inner_chart(space)?;
    if bool_val(chart.child_by_local("autoTitleDeleted")) {
        return None;
    }
    title_text(chart.child_by_local("title"))
}

pub fn axis_titles(space: &Element, scatter: bool) -> (Option<String>, Option<String>) {
    let Some(plot) = plot_area(space) else {
        return (None, None);
    };
    if scatter {
        let vals: Vec<_> = plot
            .element_children()
            .filter(|c| c.local_name() == "valAx")
            .collect();
        let cat = vals
            .first()
            .and_then(|a| title_text(a.child_by_local("title")));
        let val = vals
            .get(1)
            .and_then(|a| title_text(a.child_by_local("title")));
        (cat, val)
    } else {
        let cat = plot
            .element_children()
            .find(|c| c.local_name() == "catAx")
            .and_then(|a| title_text(a.child_by_local("title")));
        let val = plot
            .element_children()
            .find(|c| c.local_name() == "valAx")
            .and_then(|a| title_text(a.child_by_local("title")));
        (cat, val)
    }
}

pub fn uses_xy_axes(ty: &str) -> bool {
    matches!(ty, "scatter" | "bubble")
}

pub fn flags_json(space: &Element, ty: &str) -> Vec<Value> {
    let mut flags = Vec::new();
    if has_local(space, "trendline") {
        flags.push(json!("trendline"));
    }
    if has_local(space, "errBars") {
        flags.push(json!("errorBars"));
    }
    if has_local(space, "pivotSource") {
        flags.push(json!("pivot"));
    }
    if secondary_axis(space, ty) {
        flags.push(json!("secondaryAxis"));
    }
    flags
}

pub fn legend_json(space: &Element) -> Value {
    let Some(chart) = inner_chart(space) else {
        return json!({ "show": false, "pos": null });
    };
    let Some(leg) = chart.child_by_local("legend") else {
        return json!({ "show": false, "pos": null });
    };
    let pos = leg
        .child_by_local("legendPos")
        .and_then(|p| p.attr_local("val"))
        .filter(|s| matches!(*s, "b" | "t" | "l" | "r" | "tr"));
    json!({ "show": true, "pos": pos })
}

pub fn data_labels_on(space: &Element) -> bool {
    let Some(plot) = plot_area(space) else {
        return false;
    };
    dlbls_on(plot)
}

pub fn reject_restricted(
    space: &Element,
    id: &str,
    i: usize,
    class: ChartRestrict,
) -> Result<(), Error> {
    let ty = inspect_type(space, false);
    let flags = flags_json(space, &ty);
    let hard = matches!(ty.as_str(), "other" | "chartex")
        || flags.iter().any(|f| {
            matches!(
                f.as_str(),
                Some("secondaryAxis" | "trendline" | "errorBars")
            )
        });
    if hard {
        return Err(Error::with_path(
            ErrorKind::RestrictedChart { id: id.to_string() },
            format!("ops[{i}].id"),
        ));
    }
    if class == ChartRestrict::Data && flags.iter().any(|f| f.as_str() == Some("pivot")) {
        return Err(Error::with_path(
            ErrorKind::RestrictedChart { id: id.to_string() },
            format!("ops[{i}].id"),
        ));
    }
    Ok(())
}

/// 系列名单。`read_cells` 把 `c:f` 原文读成单元格值；读不到则退回 cache / lit。
pub fn series_json<F>(chart_id: &str, space: &Element, mut read_cells: F) -> Result<Vec<Value>, Error>
where
    F: FnMut(&str) -> Option<Vec<Value>>,
{
    let mut out = Vec::new();
    let mut index = 0usize;
    let Some(plot) = plot_area(space) else {
        return Ok(out);
    };
    for wrap in plot.element_children() {
        if !WRAPPERS.contains(&wrap.local_name()) {
            continue;
        }
        let bubble = wrap.local_name() == "bubbleChart";
        let xy = bubble || wrap.local_name() == "scatterChart";
        let (cat_local, val_local) = if xy {
            ("xVal", "yVal")
        } else {
            ("cat", "val")
        };
        for ser in wrap.element_children() {
            if ser.local_name() != "ser" {
                continue;
            }
            let name_ref = ser.child_by_local("tx").and_then(formula_of);
            let name = match name_ref.as_deref().and_then(|f| read_cells(f)) {
                Some(v) if !v.is_empty() => value_as_name(&v[0]),
                _ => series_name(ser),
            };
            let (cats, cat_ref, cat_stale) = read_pts(ser, cat_local, &mut read_cells);
            let (vals, val_ref, val_stale) = read_pts(ser, val_local, &mut read_cells);
            let mut node = Map::new();
            node.insert("id".into(), json!(format!("{chart_id}/series:{index}")));
            node.insert("index".into(), json!(index));
            node.insert("name".into(), json!(name));
            node.insert("nameRef".into(), json!(name_ref));
            node.insert("categories".into(), json!(cats));
            node.insert("catRef".into(), json!(cat_ref));
            node.insert("values".into(), json!(vals));
            node.insert("valRef".into(), json!(val_ref));
            let mut stale = cat_stale || val_stale;
            if bubble {
                let (sizes, size_ref, size_stale) = read_pts(ser, "bubbleSize", &mut read_cells);
                node.insert("sizes".into(), json!(sizes));
                node.insert("sizeRef".into(), json!(size_ref));
                stale |= size_stale;
            }
            if stale {
                node.insert("cacheStale".into(), json!(true));
            }
            out.push(Value::Object(node));
            index += 1;
        }
    }
    Ok(out)
}

fn read_pts<F>(ser: &Element, local: &str, read_cells: &mut F) -> (Vec<Value>, Option<String>, bool)
where
    F: FnMut(&str) -> Option<Vec<Value>>,
{
    let Some(el) = find_child(ser, local) else {
        return (Vec::new(), None, false);
    };
    let cache = cache_values(el);
    let formula = formula_of(el);
    let cells = formula.as_deref().and_then(|f| read_cells(f));
    let (vals, stale) = match (cache, cells) {
        (Some(c), Some(e)) => {
            let stale = !values_equal(&c, &e);
            (e, stale)
        }
        (None, Some(e)) => (e, false),
        (Some(c), None) => (c, false),
        (None, None) => (Vec::new(), false),
    };
    (vals, formula, stale)
}

pub fn cache_values(el: &Element) -> Option<Vec<Value>> {
    let cache = find_desc(el, "numCache")
        .or_else(|| find_desc(el, "strCache"))
        .or_else(|| find_desc(el, "numLit"))
        .or_else(|| find_desc(el, "strLit"))?;
    let mut pts: Vec<(u32, String)> = Vec::new();
    for pt in descendants_local(cache, "pt") {
        let idx = pt
            .attr_local("idx")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let v = pt
            .child_by_local("v")
            .map(|e| e.text_content())
            .unwrap_or_default();
        pts.push((idx, v));
    }
    if pts.is_empty() {
        return Some(Vec::new());
    }
    pts.sort_by_key(|(i, _)| *i);
    Some(to_json_array(
        pts.into_iter().map(|(_, v)| v).collect::<Vec<_>>(),
    ))
}

pub fn formula_of(el: &Element) -> Option<String> {
    find_desc(el, "f").map(|f| f.text_content())
}

pub fn series_name(ser: &Element) -> Option<String> {
    let tx = ser.child_by_local("tx")?;
    if let Some(v) = tx.child_by_local("v") {
        let s = v.text_content();
        return if s.is_empty() { None } else { Some(s) };
    }
    if let Some(cache) = find_desc(tx, "strCache").or_else(|| find_desc(tx, "numCache")) {
        if let Some(pt) = descendants_local(cache, "pt").next() {
            let s = pt
                .child_by_local("v")
                .map(|e| e.text_content())
                .unwrap_or_default();
            return if s.is_empty() { None } else { Some(s) };
        }
    }
    None
}

/// 只认连续 A1。命名区域、三维引用、不连续区域返回 `None`。
pub fn parse_a1_formula(f: &str) -> Option<A1Ref> {
    let mut s = f.trim().trim_start_matches('=').trim();
    if s.is_empty() {
        return None;
    }
    if let Some(rest) = s.strip_prefix('[') {
        s = rest.split_once(']')?.1.trim_start_matches('!').trim();
    }
    if has_unquoted(s, ',') {
        return None;
    }
    let (sheet, range) = if let Some(rest) = s.strip_prefix('\'') {
        let end = rest.find('\'')?;
        let sheet = rest[..end].replace("''", "'");
        let after = rest[end + 1..].trim();
        if after.starts_with(':') {
            return None;
        }
        let range = after.trim_start_matches('!').trim();
        (Some(sheet), range)
    } else if let Some((a, b)) = s.split_once('!') {
        if a.contains(':') {
            return None;
        }
        (Some(a.to_string()), b.trim())
    } else {
        (None, s)
    };
    if range.contains('[') || range.contains(' ') {
        return None;
    }
    let (start, end) = a1::parse_range(&range.replace('$', "")).ok()?;
    Some(A1Ref { sheet, start, end })
}

pub fn format_a1_formula(sheet: &str, start: CellRef, end: CellRef) -> String {
    let a = format_abs(start);
    let b = format_abs(end);
    format!("{}!{a}:{b}", quote_sheet(sheet))
}

pub fn format_a1_cell(sheet: &str, cell: CellRef) -> String {
    format!("{}!{}", quote_sheet(sheet), format_abs(cell))
}

pub fn quote_sheet(name: &str) -> String {
    let simple = name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        && name
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_');
    if simple {
        name.to_string()
    } else {
        format!("'{}'", name.replace('\'', "''"))
    }
}

fn format_abs(c: CellRef) -> String {
    format!("${}${}", a1::col_to_letters(c.col), c.row)
}

pub fn resize_a1(refer: &A1Ref, n: usize) -> A1Ref {
    let len = n.max(1) as u32;
    let mut end = refer.end;
    if refer.start.col == refer.end.col {
        end.row = refer.start.row + len - 1;
    } else if refer.start.row == refer.end.row {
        end.col = refer.start.col + len - 1;
    }
    A1Ref {
        sheet: refer.sheet.clone(),
        start: refer.start,
        end,
    }
}

pub fn walk_a1(refer: &A1Ref) -> Vec<CellRef> {
    let mut out = Vec::new();
    if refer.start.col == refer.end.col {
        for row in refer.start.row..=refer.end.row {
            out.push(CellRef {
                col: refer.start.col,
                row,
            });
        }
    } else if refer.start.row == refer.end.row {
        for col in refer.start.col..=refer.end.col {
            out.push(CellRef {
                col,
                row: refer.start.row,
            });
        }
    } else {
        for row in refer.start.row..=refer.end.row {
            for col in refer.start.col..=refer.end.col {
                out.push(CellRef { col, row });
            }
        }
    }
    out
}

pub fn inner_chart(space: &Element) -> Option<&Element> {
    space.child_by_local("chart")
}

pub fn inner_chart_mut(space: &mut Element) -> Option<&mut Element> {
    space.child_by_local_mut("chart")
}

pub fn plot_area(space: &Element) -> Option<&Element> {
    inner_chart(space)?.child_by_local("plotArea")
}

pub fn plot_area_mut(space: &mut Element) -> Option<&mut Element> {
    inner_chart_mut(space)?.child_by_local_mut("plotArea")
}

pub fn series_count(space: &Element) -> usize {
    collect_sers(space).len()
}

pub fn collect_sers(space: &Element) -> Vec<&Element> {
    let mut out = Vec::new();
    if let Some(plot) = plot_area(space) {
        for w in plot.element_children() {
            if WRAPPERS.contains(&w.local_name()) {
                for c in w.element_children() {
                    if c.local_name() == "ser" {
                        out.push(c);
                    }
                }
            }
        }
    }
    out
}

pub fn with_ser_mut<R>(
    space: &mut Element,
    index: usize,
    f: impl FnOnce(&mut Element) -> R,
) -> Option<R> {
    let mut seen = 0usize;
    let plot = plot_area_mut(space)?;
    for w in plot.element_children_mut() {
        if !WRAPPERS.contains(&w.local_name()) {
            continue;
        }
        for c in w.element_children_mut() {
            if c.local_name() == "ser" {
                if seen == index {
                    return Some(f(c));
                }
                seen += 1;
            }
        }
    }
    None
}

pub fn set_series_name(ser: &mut Element, name: &str) {
    if ser.child_by_local("tx").is_none() {
        ser.children.insert(
            2.min(ser.children.len()),
            Node::Element(Element::new("c:tx")),
        );
    }
    let Some(tx) = ser.child_by_local_mut("tx") else {
        return;
    };
    if let Some(f) = formula_of(tx) {
        let xml = format!(
            r#"<c:tx xmlns:c="{CHART_NS}">{}</c:tx>"#,
            str_ref(&f, &[json!(name)])
        );
        if let Ok(doc) = xml::parse(xml.as_bytes()) {
            *tx = doc.root;
        }
        return;
    }
    tx.children.clear();
    tx.push_element(Element::new("c:v").with_text(name));
}

pub fn set_pts(ser: &mut Element, local: &str, values: &[Value], formula: Option<&str>) {
    let tag = format!("c:{local}");
    let existing_f = ser.child_by_local(local).and_then(formula_of);
    let f = formula
        .map(str::to_string)
        .or(existing_f)
        .unwrap_or_else(|| default_formula(local, values.len()));
    let xml = format!(
        r#"<{tag} xmlns:c="{CHART_NS}">{}</{tag}>"#,
        str_or_num_ref(&f, values)
    );
    if let Ok(doc) = xml::parse(xml.as_bytes()) {
        if let Some(old) = ser
            .children
            .iter()
            .position(|n| matches!(n, Node::Element(e) if e.local_name() == local))
        {
            ser.children[old] = Node::Element(doc.root);
        } else {
            ser.push_element(doc.root);
        }
    }
}

pub fn remove_series_at(space: &mut Element, index: usize) {
    let mut seen = 0usize;
    if let Some(plot) = plot_area_mut(space) {
        for w in plot.element_children_mut() {
            if !WRAPPERS.contains(&w.local_name()) {
                continue;
            }
            let mut i = 0;
            while i < w.children.len() {
                if matches!(&w.children[i], Node::Element(e) if e.local_name() == "ser") {
                    if seen == index {
                        w.children.remove(i);
                        return;
                    }
                    seen += 1;
                }
                i += 1;
            }
        }
    }
}

pub fn set_chart_title(space: &mut Element, title: &str) {
    let Some(chart) = inner_chart_mut(space) else {
        return;
    };
    chart.children.retain(|n| match n {
        Node::Element(e) => e.local_name() != "title" && e.local_name() != "autoTitleDeleted",
        _ => true,
    });
    if title.is_empty() {
        chart.children.insert(
            0,
            Node::Element(Element::new("c:autoTitleDeleted").with_attr("val", "1")),
        );
        return;
    }
    let xml = format!(
        r#"<c:chart xmlns:c="{CHART_NS}" xmlns:a="{A_NS}">{}<c:autoTitleDeleted val="0"/></c:chart>"#,
        title_el(title)
    );
    if let Ok(doc) = xml::parse(xml.as_bytes()) {
        let nodes: Vec<_> = doc.root.children.into_iter().collect();
        for n in nodes.into_iter().rev() {
            chart.children.insert(0, n);
        }
    }
}

pub fn set_axis_title(space: &mut Element, axis: &str, title: &str, i: usize) -> Result<(), Error> {
    let scatter = uses_xy_axes(&inspect_type(space, false));
    let plot = plot_area_mut(space).ok_or_else(|| {
        Error::with_path(ErrorKind::InvalidChartAxis, format!("ops[{i}].axis"))
    })?;
    let mut idxs = Vec::new();
    for (i, n) in plot.children.iter().enumerate() {
        if let Node::Element(e) = n {
            idxs.push((i, e.local_name().to_string()));
        }
    }
    let target_i = if scatter {
        let vals: Vec<_> = idxs
            .iter()
            .filter(|(_, n)| n == "valAx")
            .map(|(i, _)| *i)
            .collect();
        match axis {
            "cat" => vals.first().copied(),
            "val" => vals.get(1).copied(),
            _ => None,
        }
    } else {
        match axis {
            "cat" => idxs.iter().find(|(_, n)| n == "catAx").map(|(i, _)| *i),
            "val" => idxs.iter().find(|(_, n)| n == "valAx").map(|(i, _)| *i),
            _ => None,
        }
    };
    let Some(idx) = target_i else {
        return Err(Error::with_path(
            ErrorKind::InvalidChartAxis,
            format!("ops[{i}].axis"),
        ));
    };
    let Node::Element(ax) = &mut plot.children[idx] else {
        return Err(Error::with_path(
            ErrorKind::InvalidChartAxis,
            format!("ops[{i}].axis"),
        ));
    };
    ax.children.retain(|n| match n {
        Node::Element(e) => e.local_name() != "title",
        _ => true,
    });
    if !title.is_empty() {
        let xml = format!(
            r#"<c:ax xmlns:c="{CHART_NS}" xmlns:a="{A_NS}">{}</c:ax>"#,
            title_el(title)
        );
        if let Ok(doc) = xml::parse(xml.as_bytes()) {
            if let Some(Node::Element(t)) = doc.root.children.into_iter().next() {
                ax.children
                    .insert(1.min(ax.children.len()), Node::Element(t));
            }
        }
    }
    Ok(())
}

pub fn set_legend(space: &mut Element, show: bool, pos: Option<&str>) {
    let Some(chart) = inner_chart_mut(space) else {
        return;
    };
    if !show {
        chart.children.retain(|n| match n {
            Node::Element(e) => e.local_name() != "legend",
            _ => true,
        });
        return;
    }
    let pos = pos.map(str::to_string).unwrap_or_else(|| {
        chart
            .child_by_local("legend")
            .and_then(|l| l.child_by_local("legendPos"))
            .and_then(|p| p.attr_local("val"))
            .unwrap_or("r")
            .to_string()
    });
    chart.children.retain(|n| match n {
        Node::Element(e) => e.local_name() != "legend",
        _ => true,
    });
    let mut legend = Element::new("c:legend");
    legend.push_element(Element::new("c:legendPos").with_attr("val", &pos));
    legend.push_element(Element::new("c:overlay").with_attr("val", "0"));
    let insert_at = chart
        .children
        .iter()
        .position(|n| matches!(n, Node::Element(e) if e.local_name() == "plotVisOnly"))
        .unwrap_or(chart.children.len());
    chart.children.insert(insert_at, Node::Element(legend));
}

pub fn set_data_labels(space: &mut Element, show: bool) {
    let Some(plot) = plot_area_mut(space) else {
        return;
    };
    if show {
        for w in plot.element_children_mut() {
            if WRAPPERS.contains(&w.local_name()) {
                w.children.retain(|n| match n {
                    Node::Element(e) => e.local_name() != "dLbls",
                    _ => true,
                });
                let mut d = Element::new("c:dLbls");
                d.push_element(Element::new("c:showLegendKey").with_attr("val", "0"));
                d.push_element(Element::new("c:showVal").with_attr("val", "1"));
                d.push_element(Element::new("c:showCatName").with_attr("val", "0"));
                d.push_element(Element::new("c:showSerName").with_attr("val", "0"));
                d.push_element(Element::new("c:showPercent").with_attr("val", "0"));
                let at = w
                    .children
                    .iter()
                    .position(|n| matches!(n, Node::Element(e) if e.local_name() == "axId"))
                    .unwrap_or(w.children.len());
                w.children.insert(at, Node::Element(d));
            }
        }
    } else {
        strip_dlbls(plot);
    }
}

pub fn convert_type(space: &mut Element, target: ChartType, i: usize) -> Result<(), Error> {
    if target == ChartType::Combo {
        return Err(Error::with_path(
            ErrorKind::InvalidChartType,
            format!("ops[{i}].type"),
        ));
    }
    let current = inspect_type(space, false);
    if current == target.as_str() {
        if matches!(target, ChartType::Bar | ChartType::Col) {
            set_bar_dir(space, target);
        }
        return Ok(());
    }
    if matches!(current.as_str(), "bar" | "col") && matches!(target, ChartType::Bar | ChartType::Col)
    {
        set_bar_dir(space, target);
        return Ok(());
    }
    if target.needs_one_series() && series_count(space) != 1 {
        return Err(Error::with_path(
            ErrorKind::ChartTypeNotConvertible,
            format!("ops[{i}].type"),
        ));
    }
    let from_xy = uses_xy_axes(&current) || (current == "combo" && any_ser_xy(space));
    let to_xy = target.is_xy();
    if to_xy && !from_xy && !all_sers_have_cat_val(space) {
        return Err(Error::with_path(
            ErrorKind::ChartTypeNotConvertible,
            format!("ops[{i}].type"),
        ));
    }
    if from_xy && !to_xy && current != "combo" && !all_sers_have_xy(space) {
        return Err(Error::with_path(
            ErrorKind::ChartTypeNotConvertible,
            format!("ops[{i}].type"),
        ));
    }
    if current == "combo" && to_xy && !all_sers_have_cat_or_xy(space) {
        return Err(Error::with_path(
            ErrorKind::ChartTypeNotConvertible,
            format!("ops[{i}].type"),
        ));
    }
    {
        let Some(plot) = plot_area_mut(space) else {
            return Err(Error::with_path(
                ErrorKind::ChartTypeNotConvertible,
                format!("ops[{i}].type"),
            ));
        };
        let (insert_at, mut sers, ax_ids) = extract_wrapper_sers(plot);
        for ser in &mut sers {
            reshape_ser(ser, target);
        }
        let wrap = build_type_wrapper(target, sers, &ax_ids);
        let at = insert_at.unwrap_or(plot.children.len());
        plot.children.insert(at, Node::Element(wrap));
    }
    retarget_axes(space, target, from_xy);
    Ok(())
}

pub fn last_series_type(space: &Element) -> Option<ChartType> {
    let plot = plot_area(space)?;
    let mut last = None;
    for wrap in plot.element_children() {
        if !WRAPPERS.contains(&wrap.local_name()) {
            continue;
        }
        let ty = wrapper_kind(wrap).and_then(ChartType::parse);
        for c in wrap.element_children() {
            if c.local_name() == "ser" {
                last = ty;
            }
        }
    }
    last
}

pub fn unused_idx(space: &Element) -> u32 {
    let mut used: std::collections::HashSet<u32> = std::collections::HashSet::new();
    for ser in collect_sers(space) {
        if let Some(v) = ser
            .child_by_local("idx")
            .and_then(|e| e.attr_local("val"))
            .and_then(|s| s.parse().ok())
        {
            used.insert(v);
        }
    }
    (0..).find(|n| !used.contains(n)).unwrap_or(0)
}

pub fn insert_ser(space: &mut Element, ser_el: Element, series_type: Option<ChartType>) {
    let want = series_type.or_else(|| last_series_type(space));
    let Some(plot) = plot_area_mut(space) else {
        return;
    };
    let target_i = want.and_then(|ty| {
        plot.element_children().enumerate().find_map(|(i, w)| {
            if !WRAPPERS.contains(&w.local_name()) {
                return None;
            }
            if wrapper_kind(w) == Some(ty.as_str()) {
                Some(i)
            } else {
                None
            }
        })
    });
    if let Some(i) = target_i {
        if let Node::Element(w) = &mut plot.children[i] {
            push_ser_into_wrapper(w, ser_el);
            return;
        }
    }
    if let Some(ty) = series_type {
        let ax_ids: Vec<String> = plot
            .element_children()
            .find(|c| WRAPPERS.contains(&c.local_name()))
            .map(|w| {
                w.element_children()
                    .filter(|c| c.local_name() == "axId")
                    .filter_map(|c| c.attr_local("val").map(str::to_string))
                    .take(2)
                    .collect()
            })
            .unwrap_or_default();
        let mut w = Element::new(format!("c:{}", ty.wrapper()));
        for p in wrapper_prefix_els(ty) {
            w.push_element(p);
        }
        w.push_element(ser_el);
        if !ty.no_axes() {
            let a = ax_ids.first().map(String::as_str).unwrap_or("1");
            let b = ax_ids.get(1).map(String::as_str).unwrap_or("2");
            w.push_element(Element::new("c:axId").with_attr("val", a));
            w.push_element(Element::new("c:axId").with_attr("val", b));
        }
        for s in wrapper_suffix_els(ty) {
            w.push_element(s);
        }
        let at = plot
            .children
            .iter()
            .position(|n| {
                matches!(n, Node::Element(e) if matches!(e.local_name(), "catAx" | "valAx" | "dateAx" | "serAx"))
            })
            .unwrap_or(plot.children.len());
        plot.children.insert(at, Node::Element(w));
        return;
    }
    if let Some(w) = plot
        .element_children_mut()
        .find(|c| WRAPPERS.contains(&c.local_name()))
    {
        push_ser_into_wrapper(w, ser_el);
    }
}

pub fn parse_ser_xml(xml: &str) -> Result<Element, Error> {
    let wrapped = format!(r#"<c:root xmlns:c="{CHART_NS}">{xml}</c:root>"#);
    let doc = xml::parse(wrapped.as_bytes())?;
    doc.root
        .children
        .into_iter()
        .find_map(|n| match n {
            Node::Element(e) if e.local_name() == "ser" => Some(e),
            _ => None,
        })
        .ok_or_else(|| {
            Error::new(ErrorKind::XmlMissing {
                parent: "addChartSeries".into(),
                child: "c:ser".into(),
            })
        })
}

pub fn shared_categories(series: &[NewSeries]) -> Vec<Value> {
    if let Some(s) = series.iter().find_map(|s| s.categories.as_ref()) {
        return s.clone();
    }
    let n = series.first().map(|s| s.values.len()).unwrap_or(0);
    (1..=n).map(|i| json!(i.to_string())).collect()
}

pub fn normalize_insert_series(ty: ChartType, series: &[NewSeries]) -> Vec<NewSeries> {
    series
        .iter()
        .map(|s| {
            let mut s = s.clone();
            if ty == ChartType::Bubble && s.sizes.is_none() {
                s.sizes = Some(ones(s.values.len()));
            }
            s
        })
        .collect()
}

pub fn ones(n: usize) -> Vec<Value> {
    (0..n).map(|_| json!(1)).collect()
}

pub fn build_chart_xml(
    ty: ChartType,
    title: Option<&str>,
    series: &[NewSeries],
    cats: &[Value],
    sheet: &str,
    origin: CellRef,
    include_external_data: bool,
) -> String {
    if ty == ChartType::Combo {
        return build_combo_chart_xml(title, series, cats, sheet, origin, include_external_data);
    }
    let xy = ty.is_xy();
    let mut sers = String::new();
    for (i, ser) in series.iter().enumerate() {
        let (val_col, size_col) = if ty == ChartType::Bubble {
            (
                origin.col + (i as u32) * 2 + 1,
                Some(origin.col + (i as u32) * 2 + 2),
            )
        } else {
            (origin.col + (i as u32) + 1, None)
        };
        let name = ser
            .name
            .clone()
            .unwrap_or_else(|| format!("系列{}", i + 1));
        let cat_end = CellRef {
            col: origin.col,
            row: origin.row + cats.len() as u32,
        };
        let cat_start = CellRef {
            col: origin.col,
            row: origin.row + 1,
        };
        let val_start = CellRef {
            col: val_col,
            row: origin.row + 1,
        };
        let val_end = CellRef {
            col: val_col,
            row: origin.row + ser.values.len() as u32,
        };
        let name_cell = CellRef {
            col: val_col,
            row: origin.row,
        };
        let cat_ref = format_a1_formula(sheet, cat_start, cat_end);
        let val_ref = format_a1_formula(sheet, val_start, val_end);
        let name_ref = format_a1_cell(sheet, name_cell);
        let sizes = ser.sizes.as_deref();
        let size_ref = size_col.map(|c| {
            let n = sizes.map(|s| s.len()).unwrap_or(ser.values.len()) as u32;
            format_a1_formula(
                sheet,
                CellRef {
                    col: c,
                    row: origin.row + 1,
                },
                CellRef {
                    col: c,
                    row: origin.row + n,
                },
            )
        });
        let size_pair = match (sizes, size_ref.as_deref()) {
            (Some(sz), Some(r)) => Some((sz, r)),
            _ => None,
        };
        sers.push_str(&ser_xml(
            i as u32,
            &name,
            &name_ref,
            &cat_ref,
            cats,
            &val_ref,
            &ser.values,
            xy,
            size_pair,
        ));
    }
    let wrapper = ty.wrapper();
    let extra = wrapper_prefix_xml(ty);
    let suffix = wrapper_suffix_xml(ty);
    let axes = if ty.no_axes() {
        String::new()
    } else if xy {
        ax_pair_scatter()
    } else {
        ax_pair()
    };
    let ax_ids = if ty.no_axes() {
        String::new()
    } else {
        r#"<c:axId val="1"/><c:axId val="2"/>"#.into()
    };
    chart_space_xml(
        title,
        &format!("<c:{wrapper}>{extra}{sers}<c:dLbls><c:showLegendKey val=\"0\"/><c:showVal val=\"0\"/><c:showCatName val=\"0\"/><c:showSerName val=\"0\"/><c:showPercent val=\"0\"/></c:dLbls>{suffix}{ax_ids}</c:{wrapper}>"),
        &axes,
        include_external_data,
    )
}

pub fn ser_xml(
    idx: u32,
    name: &str,
    name_ref: &str,
    cat_ref: &str,
    cats: &[Value],
    val_ref: &str,
    vals: &[Value],
    xy: bool,
    sizes: Option<(&[Value], &str)>,
) -> String {
    let (cat_tag, val_tag) = if xy {
        ("c:xVal", "c:yVal")
    } else {
        ("c:cat", "c:val")
    };
    let mut body = format!(
        r#"<c:ser><c:idx val="{idx}"/><c:order val="{idx}"/><c:tx>{}</c:tx><{cat_tag}>{}</{cat_tag}><{val_tag}>{}</{val_tag}>"#,
        str_ref(name_ref, &[json!(name)]),
        str_or_num_ref(cat_ref, cats),
        str_or_num_ref(val_ref, vals),
    );
    if let Some((sz, sz_ref)) = sizes {
        body.push_str(&format!(
            "<c:bubbleSize>{}</c:bubbleSize><c:bubble3D val=\"0\"/>",
            str_or_num_ref(sz_ref, sz)
        ));
    }
    body.push_str("</c:ser>");
    body
}

pub fn data_table_cells(
    ty: ChartType,
    series: &[NewSeries],
    cats: &[Value],
    origin: CellRef,
) -> Vec<(CellRef, Value)> {
    let mut out = Vec::new();
    for (i, cat) in cats.iter().enumerate() {
        out.push((
            CellRef {
                col: origin.col,
                row: origin.row + 1 + i as u32,
            },
            cat.clone(),
        ));
    }
    if ty == ChartType::Bubble {
        for (si, s) in series.iter().enumerate() {
            let y_col = origin.col + (si as u32) * 2 + 1;
            let sz_col = origin.col + (si as u32) * 2 + 2;
            let name = s
                .name
                .clone()
                .unwrap_or_else(|| format!("系列{}", si + 1));
            out.push((
                CellRef {
                    col: y_col,
                    row: origin.row,
                },
                json!(name),
            ));
            for (ri, v) in s.values.iter().enumerate() {
                out.push((
                    CellRef {
                        col: y_col,
                        row: origin.row + 1 + ri as u32,
                    },
                    v.clone(),
                ));
            }
            let sizes = s.sizes.clone().unwrap_or_else(|| ones(s.values.len()));
            for (ri, v) in sizes.iter().enumerate() {
                out.push((
                    CellRef {
                        col: sz_col,
                        row: origin.row + 1 + ri as u32,
                    },
                    v.clone(),
                ));
            }
        }
    } else {
        for (si, s) in series.iter().enumerate() {
            let col = origin.col + (si as u32) + 1;
            let name = s
                .name
                .clone()
                .unwrap_or_else(|| format!("系列{}", si + 1));
            out.push((
                CellRef {
                    col,
                    row: origin.row,
                },
                json!(name),
            ));
            for (ri, v) in s.values.iter().enumerate() {
                out.push((
                    CellRef {
                        col,
                        row: origin.row + 1 + ri as u32,
                    },
                    v.clone(),
                ));
            }
        }
    }
    out
}

pub fn next_val_col_from_formulas(space: &Element) -> Option<u32> {
    let mut max = 0u32;
    let mut found = false;
    for ser in collect_sers(space) {
        for local in ["tx", "val", "yVal", "bubbleSize"] {
            if let Some(f) = ser.child_by_local(local).and_then(formula_of) {
                if let Some(r) = parse_a1_formula(&f) {
                    found = true;
                    max = max.max(r.end.col);
                }
            }
        }
    }
    if found {
        Some(max + 1)
    } else {
        None
    }
}

fn build_combo_chart_xml(
    title: Option<&str>,
    series: &[NewSeries],
    cats: &[Value],
    sheet: &str,
    origin: CellRef,
    include_external_data: bool,
) -> String {
    let mut groups: Vec<(ChartType, Vec<(usize, &NewSeries)>)> = Vec::new();
    for (i, s) in series.iter().enumerate() {
        let ty = s.series_type.unwrap_or(ChartType::Col);
        if let Some((_, items)) = groups.iter_mut().find(|(t, _)| *t == ty) {
            items.push((i, s));
        } else {
            groups.push((ty, vec![(i, s)]));
        }
    }
    let mut wrappers = String::new();
    for (ty, items) in &groups {
        let mut sers = String::new();
        for (i, ser) in items {
            let col = origin.col + (*i as u32) + 1;
            let name = ser
                .name
                .clone()
                .unwrap_or_else(|| format!("系列{}", *i + 1));
            let cat_ref = format_a1_formula(
                sheet,
                CellRef {
                    col: origin.col,
                    row: origin.row + 1,
                },
                CellRef {
                    col: origin.col,
                    row: origin.row + cats.len() as u32,
                },
            );
            let val_ref = format_a1_formula(
                sheet,
                CellRef {
                    col,
                    row: origin.row + 1,
                },
                CellRef {
                    col,
                    row: origin.row + ser.values.len() as u32,
                },
            );
            let name_ref = format_a1_cell(
                sheet,
                CellRef {
                    col,
                    row: origin.row,
                },
            );
            sers.push_str(&ser_xml(
                *i as u32,
                &name,
                &name_ref,
                &cat_ref,
                cats,
                &val_ref,
                &ser.values,
                ty.is_xy(),
                None,
            ));
        }
        let extra = wrapper_prefix_xml(*ty);
        wrappers.push_str(&format!(
            "<c:{w}>{extra}{sers}<c:axId val=\"1\"/><c:axId val=\"2\"/></c:{w}>",
            w = ty.wrapper()
        ));
    }
    let axes = if groups.iter().all(|(t, _)| t.is_xy()) {
        ax_pair_scatter()
    } else {
        ax_pair()
    };
    chart_space_xml(title, &wrappers, &axes, include_external_data)
}

fn chart_space_xml(title: Option<&str>, plot_body: &str, axes: &str, embed: bool) -> String {
    let title_xml = match title {
        Some(t) if !t.is_empty() => format!(r#"<c:autoTitleDeleted val="0"/>{}"#, title_el(t)),
        _ => r#"<c:autoTitleDeleted val="1"/>"#.into(),
    };
    let ext = if embed {
        r#"<c:externalData r:id="rId1"><c:autoUpdate val="0"/></c:externalData>"#
    } else {
        ""
    };
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><c:chartSpace xmlns:c="{CHART_NS}" xmlns:a="{A_NS}" xmlns:r="{REL_NS}"><c:chart>{title_xml}<c:plotArea><c:layout/>{plot_body}{axes}</c:plotArea><c:legend><c:legendPos val="r"/><c:overlay val="0"/></c:legend><c:plotVisOnly val="1"/></c:chart>{ext}</c:chartSpace>"#
    )
}

fn wrapper_prefix_xml(ty: ChartType) -> &'static str {
    match ty {
        ChartType::Bar => {
            r#"<c:barDir val="bar"/><c:grouping val="clustered"/><c:varyColors val="0"/>"#
        }
        ChartType::Col => {
            r#"<c:barDir val="col"/><c:grouping val="clustered"/><c:varyColors val="0"/>"#
        }
        ChartType::Line => r#"<c:grouping val="standard"/><c:varyColors val="0"/>"#,
        ChartType::Area => r#"<c:grouping val="standard"/><c:varyColors val="0"/>"#,
        ChartType::Pie | ChartType::Doughnut => r#"<c:varyColors val="1"/>"#,
        ChartType::Scatter => r#"<c:scatterStyle val="lineMarker"/><c:varyColors val="0"/>"#,
        ChartType::Radar => r#"<c:radarStyle val="marker"/><c:varyColors val="0"/>"#,
        ChartType::Bubble => r#"<c:varyColors val="0"/>"#,
        ChartType::Combo => {
            r#"<c:barDir val="col"/><c:grouping val="clustered"/><c:varyColors val="0"/>"#
        }
    }
}

fn wrapper_suffix_xml(ty: ChartType) -> &'static str {
    match ty {
        ChartType::Doughnut => r#"<c:holeSize val="50"/>"#,
        ChartType::Bubble => r#"<c:bubbleScale val="100"/><c:showNegBubbles val="0"/>"#,
        _ => "",
    }
}

fn wrapper_prefix_els(ty: ChartType) -> Vec<Element> {
    match ty {
        ChartType::Bar => vec![
            Element::new("c:barDir").with_attr("val", "bar"),
            Element::new("c:grouping").with_attr("val", "clustered"),
            Element::new("c:varyColors").with_attr("val", "0"),
        ],
        ChartType::Col => vec![
            Element::new("c:barDir").with_attr("val", "col"),
            Element::new("c:grouping").with_attr("val", "clustered"),
            Element::new("c:varyColors").with_attr("val", "0"),
        ],
        ChartType::Line | ChartType::Area => vec![
            Element::new("c:grouping").with_attr("val", "standard"),
            Element::new("c:varyColors").with_attr("val", "0"),
        ],
        ChartType::Pie | ChartType::Doughnut => {
            vec![Element::new("c:varyColors").with_attr("val", "1")]
        }
        ChartType::Scatter => vec![
            Element::new("c:scatterStyle").with_attr("val", "lineMarker"),
            Element::new("c:varyColors").with_attr("val", "0"),
        ],
        ChartType::Radar => vec![
            Element::new("c:radarStyle").with_attr("val", "marker"),
            Element::new("c:varyColors").with_attr("val", "0"),
        ],
        ChartType::Bubble => vec![Element::new("c:varyColors").with_attr("val", "0")],
        ChartType::Combo => vec![
            Element::new("c:barDir").with_attr("val", "col"),
            Element::new("c:grouping").with_attr("val", "clustered"),
            Element::new("c:varyColors").with_attr("val", "0"),
        ],
    }
}

fn wrapper_suffix_els(ty: ChartType) -> Vec<Element> {
    match ty {
        ChartType::Doughnut => vec![Element::new("c:holeSize").with_attr("val", "50")],
        ChartType::Bubble => vec![
            Element::new("c:bubbleScale").with_attr("val", "100"),
            Element::new("c:showNegBubbles").with_attr("val", "0"),
        ],
        _ => Vec::new(),
    }
}

fn str_ref(f: &str, pts: &[Value]) -> String {
    let mut body = format!(
        r#"<c:strRef><c:f>{}</c:f><c:strCache><c:ptCount val="{}" />"#,
        xml::escape_text(f),
        pts.len()
    );
    for (i, p) in pts.iter().enumerate() {
        body.push_str(&format!(
            r#"<c:pt idx="{i}"><c:v>{}</c:v></c:pt>"#,
            xml::escape_text(&value_text(p))
        ));
    }
    body.push_str("</c:strCache></c:strRef>");
    body
}

fn num_ref(f: &str, pts: &[Value]) -> String {
    let mut body = format!(
        r#"<c:numRef><c:f>{}</c:f><c:numCache><c:formatCode>General</c:formatCode><c:ptCount val="{}" />"#,
        xml::escape_text(f),
        pts.len()
    );
    for (i, p) in pts.iter().enumerate() {
        body.push_str(&format!(
            r#"<c:pt idx="{i}"><c:v>{}</c:v></c:pt>"#,
            xml::escape_text(&value_text(p))
        ));
    }
    body.push_str("</c:numCache></c:numRef>");
    body
}

fn str_or_num_ref(f: &str, pts: &[Value]) -> String {
    if !pts.is_empty() && pts.iter().all(|v| v.is_number()) {
        num_ref(f, pts)
    } else {
        str_ref(f, pts)
    }
}

fn value_text(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => String::new(),
        _ => v.to_string(),
    }
}

fn value_as_name(v: &Value) -> Option<String> {
    match v {
        Value::Null => None,
        Value::String(s) if s.is_empty() => None,
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => Some(v.to_string()),
    }
}

fn title_el(text: &str) -> String {
    format!(
        r#"<c:title><c:tx><c:rich><a:bodyPr/><a:lstStyle/><a:p><a:pPr><a:defRPr/></a:pPr><a:r><a:t>{}</a:t></a:r></a:p></c:rich></c:tx><c:overlay val="0"/></c:title>"#,
        xml::escape_text(text)
    )
}

fn ax_pair() -> String {
    r#"<c:catAx><c:axId val="1"/><c:scaling><c:orientation val="minMax"/></c:scaling><c:delete val="0"/><c:axPos val="b"/><c:crossAx val="2"/><c:crosses val="autoZero"/><c:auto val="1"/><c:lblAlgn val="ctr"/><c:lblOffset val="100"/><c:noMultiLvlLbl val="0"/></c:catAx><c:valAx><c:axId val="2"/><c:scaling><c:orientation val="minMax"/></c:scaling><c:delete val="0"/><c:axPos val="l"/><c:majorGridlines/><c:numFmt formatCode="General" sourceLinked="1"/><c:crossAx val="1"/><c:crosses val="autoZero"/><c:crossBetween val="between"/></c:valAx>"#.into()
}

fn ax_pair_scatter() -> String {
    r#"<c:valAx><c:axId val="1"/><c:scaling><c:orientation val="minMax"/></c:scaling><c:delete val="0"/><c:axPos val="b"/><c:crossAx val="2"/><c:crosses val="autoZero"/></c:valAx><c:valAx><c:axId val="2"/><c:scaling><c:orientation val="minMax"/></c:scaling><c:delete val="0"/><c:axPos val="l"/><c:majorGridlines/><c:numFmt formatCode="General" sourceLinked="1"/><c:crossAx val="1"/><c:crosses val="autoZero"/></c:valAx>"#.into()
}

fn default_formula(local: &str, n: usize) -> String {
    let end = n.max(1) + 1;
    match local {
        "cat" | "xVal" => format!("Sheet1!$A$2:$A${end}"),
        "bubbleSize" => format!("Sheet1!$C$2:$C${end}"),
        _ => format!("Sheet1!$B$2:$B${end}"),
    }
}

fn title_text(title: Option<&Element>) -> Option<String> {
    let title = title?;
    let tx = title.child_by_local("tx")?;
    if let Some(v) = tx.child_by_local("v") {
        let s = v.text_content();
        return if s.is_empty() { None } else { Some(s) };
    }
    if let Some(rich) = find_desc(tx, "rich") {
        let s = join_t(rich);
        return if s.is_empty() { None } else { Some(s) };
    }
    if let Some(cache) = find_desc(tx, "strCache").or_else(|| find_desc(tx, "numCache")) {
        let mut s = String::new();
        for pt in descendants_local(cache, "pt") {
            if let Some(v) = pt.child_by_local("v") {
                s.push_str(&v.text_content());
            }
        }
        return if s.is_empty() { None } else { Some(s) };
    }
    None
}

fn secondary_axis(space: &Element, ty: &str) -> bool {
    let Some(plot) = plot_area(space) else {
        return false;
    };
    if plot.element_children().any(|c| {
        matches!(c.local_name(), "catAx" | "valAx" | "dateAx" | "serAx")
            && bool_val(c.child_by_local("overlay"))
    }) {
        return true;
    }
    if matches!(ty, "pie" | "doughnut" | "other" | "chartex") {
        return false;
    }
    let val_n = plot
        .element_children()
        .filter(|c| matches!(c.local_name(), "valAx" | "dateAx"))
        .count();
    if uses_xy_axes(ty) {
        val_n > 2
    } else {
        val_n > 1
    }
}

fn dlbls_on(el: &Element) -> bool {
    if el.local_name() == "dLbls" {
        return ["showVal", "showCatName", "showSerName", "showPercent"]
            .iter()
            .any(|n| bool_val(el.child_by_local(n)));
    }
    el.element_children().any(dlbls_on)
}

fn strip_dlbls(el: &mut Element) {
    el.children.retain(|n| match n {
        Node::Element(e) => e.local_name() != "dLbls",
        _ => true,
    });
    for c in el.element_children_mut() {
        strip_dlbls(c);
    }
}

fn extract_wrapper_sers(plot: &mut Element) -> (Option<usize>, Vec<Element>, Vec<String>) {
    let mut first = None;
    let mut sers = Vec::new();
    let mut ax_ids = Vec::new();
    let mut i = 0;
    while i < plot.children.len() {
        let is_wrap = matches!(
            &plot.children[i],
            Node::Element(e) if WRAPPERS.contains(&e.local_name())
        );
        if !is_wrap {
            i += 1;
            continue;
        }
        if first.is_none() {
            first = Some(i);
        }
        if let Node::Element(w) = plot.children.remove(i) {
            for n in w.children {
                if let Node::Element(c) = n {
                    if c.local_name() == "ser" {
                        sers.push(c);
                    } else if c.local_name() == "axId" && ax_ids.len() < 2 {
                        if let Some(v) = c.attr_local("val") {
                            ax_ids.push(v.to_string());
                        }
                    }
                }
            }
        }
    }
    (first, sers, ax_ids)
}

fn build_type_wrapper(ty: ChartType, sers: Vec<Element>, ax_ids: &[String]) -> Element {
    let mut w = Element::new(format!("c:{}", ty.wrapper()));
    for p in wrapper_prefix_els(ty) {
        w.push_element(p);
    }
    for s in sers {
        w.push_element(s);
    }
    for s in wrapper_suffix_els(ty) {
        w.push_element(s);
    }
    if !ty.no_axes() {
        let a = ax_ids.first().map(String::as_str).unwrap_or("1");
        let b = ax_ids.get(1).map(String::as_str).unwrap_or("2");
        w.push_element(Element::new("c:axId").with_attr("val", a));
        w.push_element(Element::new("c:axId").with_attr("val", b));
    }
    w
}

fn reshape_ser(ser: &mut Element, target: ChartType) {
    let is_xy = ser.child_by_local("xVal").is_some() || ser.child_by_local("yVal").is_some();
    let is_bubble = ser.child_by_local("bubbleSize").is_some();
    let want_xy = target.is_xy();
    let want_bubble = target == ChartType::Bubble;
    if want_xy && !is_xy {
        rename_local(ser, "cat", "c:xVal");
        rename_local(ser, "val", "c:yVal");
    }
    if !want_xy && is_xy {
        rename_local(ser, "xVal", "c:cat");
        rename_local(ser, "yVal", "c:val");
    }
    if want_bubble && !is_bubble {
        append_bubble_size_ones(ser);
    }
    if !want_bubble && is_bubble {
        ser.children.retain(|n| match n {
            Node::Element(e) => e.local_name() != "bubbleSize" && e.local_name() != "bubble3D",
            _ => true,
        });
    }
}

fn rename_local(ser: &mut Element, from: &str, to: &str) {
    for c in ser.element_children_mut() {
        if c.local_name() == from {
            c.name = to.into();
        }
    }
}

fn append_bubble_size_ones(ser: &mut Element) {
    if ser.child_by_local("bubbleSize").is_some() {
        return;
    }
    let n = ser
        .child_by_local("yVal")
        .or_else(|| ser.child_by_local("val"))
        .and_then(cache_values)
        .map(|v| v.len())
        .unwrap_or(1)
        .max(1);
    let xml = format!(
        r#"<c:bubbleSize xmlns:c="{CHART_NS}"><c:numLit><c:formatCode>General</c:formatCode><c:ptCount val="{n}"/>{}</c:numLit></c:bubbleSize>"#,
        (0..n)
            .map(|i| format!(r#"<c:pt idx="{i}"><c:v>1</c:v></c:pt>"#))
            .collect::<String>()
    );
    if let Ok(doc) = xml::parse(xml.as_bytes()) {
        let at = ser
            .children
            .iter()
            .position(|n| {
                matches!(n, Node::Element(e) if e.local_name() == "yVal" || e.local_name() == "val")
            })
            .map(|i| i + 1)
            .unwrap_or(ser.children.len());
        ser.children.insert(at, Node::Element(doc.root));
        ser.push_element(Element::new("c:bubble3D").with_attr("val", "0"));
    }
}

fn any_ser_xy(space: &Element) -> bool {
    collect_sers(space)
        .iter()
        .any(|s| s.child_by_local("xVal").is_some() || s.child_by_local("yVal").is_some())
}

fn all_sers_have_cat_val(space: &Element) -> bool {
    collect_sers(space)
        .iter()
        .all(|s| s.child_by_local("cat").is_some() && s.child_by_local("val").is_some())
}

fn all_sers_have_xy(space: &Element) -> bool {
    collect_sers(space)
        .iter()
        .all(|s| s.child_by_local("xVal").is_some() && s.child_by_local("yVal").is_some())
}

fn all_sers_have_cat_or_xy(space: &Element) -> bool {
    collect_sers(space).iter().all(|s| {
        (s.child_by_local("cat").is_some() && s.child_by_local("val").is_some())
            || (s.child_by_local("xVal").is_some() && s.child_by_local("yVal").is_some())
    })
}

fn retarget_axes(space: &mut Element, target: ChartType, from_xy: bool) {
    let Some(plot) = plot_area_mut(space) else {
        return;
    };
    if target.no_axes() {
        plot.children.retain(|n| match n {
            Node::Element(e) => !matches!(e.local_name(), "catAx" | "valAx" | "dateAx" | "serAx"),
            _ => true,
        });
        return;
    }
    let has_cat = plot.element_children().any(|c| c.local_name() == "catAx");
    let val_n = plot
        .element_children()
        .filter(|c| c.local_name() == "valAx")
        .count();
    if target.is_xy() {
        for c in plot.element_children_mut() {
            if c.local_name() == "catAx" {
                c.name = "c:valAx".into();
            }
        }
        let val_n = plot
            .element_children()
            .filter(|c| c.local_name() == "valAx")
            .count();
        if val_n == 0 {
            append_axes(plot, &ax_pair_scatter());
        } else if val_n == 1 {
            append_axes(plot, r#"<c:valAx><c:axId val="2"/><c:scaling><c:orientation val="minMax"/></c:scaling><c:delete val="0"/><c:axPos val="l"/><c:majorGridlines/><c:numFmt formatCode="General" sourceLinked="1"/><c:crossAx val="1"/><c:crosses val="autoZero"/></c:valAx>"#);
        }
    } else if from_xy {
        let mut first = true;
        for c in plot.element_children_mut() {
            if c.local_name() == "valAx" && first {
                c.name = "c:catAx".into();
                first = false;
            }
        }
        if plot.element_children().all(|c| c.local_name() != "valAx") {
            append_axes(plot, r#"<c:valAx><c:axId val="2"/><c:scaling><c:orientation val="minMax"/></c:scaling><c:delete val="0"/><c:axPos val="l"/><c:majorGridlines/><c:numFmt formatCode="General" sourceLinked="1"/><c:crossAx val="1"/><c:crosses val="autoZero"/><c:crossBetween val="between"/></c:valAx>"#);
        }
    } else if !has_cat && val_n == 0 {
        append_axes(plot, &ax_pair());
    }
}

fn append_axes(plot: &mut Element, xml: &str) {
    let wrapped = format!(r#"<c:plotArea xmlns:c="{CHART_NS}">{xml}</c:plotArea>"#);
    if let Ok(doc) = xml::parse(wrapped.as_bytes()) {
        for n in doc.root.children {
            plot.children.push(n);
        }
    }
}

fn set_bar_dir(space: &mut Element, target: ChartType) {
    let Some(plot) = plot_area_mut(space) else {
        return;
    };
    for w in plot.element_children_mut() {
        if matches!(w.local_name(), "barChart" | "bar3DChart") {
            let val = if target == ChartType::Bar { "bar" } else { "col" };
            if let Some(d) = w.child_by_local_mut("barDir") {
                let name = d
                    .attrs
                    .iter()
                    .find(|(k, _)| k.rsplit(':').next().unwrap_or(k) == "val")
                    .map(|(k, _)| k.clone())
                    .unwrap_or_else(|| "val".into());
                d.set_attr(&name, val);
            } else {
                w.children.insert(
                    0,
                    Node::Element(Element::new("c:barDir").with_attr("val", val)),
                );
            }
        }
    }
}

fn push_ser_into_wrapper(w: &mut Element, ser_el: Element) {
    let at = w
        .children
        .iter()
        .position(|n| {
            matches!(n, Node::Element(e) if matches!(e.local_name(), "dLbls" | "axId" | "holeSize" | "bubbleScale" | "showNegBubbles"))
        })
        .unwrap_or(w.children.len());
    w.children.insert(at, Node::Element(ser_el));
}

fn to_json_array(raw: Vec<String>) -> Vec<Value> {
    let nums: Option<Vec<Number>> = raw.iter().map(|s| parse_num(s)).collect();
    if let Some(n) = nums {
        n.into_iter().map(Value::Number).collect()
    } else {
        raw.into_iter().map(Value::String).collect()
    }
}

fn parse_num(s: &str) -> Option<Number> {
    let t = s.trim();
    if t.is_empty() {
        return None;
    }
    if let Ok(i) = t.parse::<i64>() {
        return Some(Number::from(i));
    }
    t.parse::<f64>().ok().and_then(Number::from_f64)
}

fn values_equal(a: &[Value], b: &[Value]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b.iter()).all(|(x, y)| value_text(x) == value_text(y))
}

fn bool_val(el: Option<&Element>) -> bool {
    matches!(
        el.and_then(|e| e.attr_local("val")).unwrap_or("0"),
        "1" | "true" | "on" | "True"
    )
}

fn find_child<'a>(el: &'a Element, local: &str) -> Option<&'a Element> {
    el.element_children().find(|c| c.local_name() == local)
}

fn find_desc<'a>(el: &'a Element, local: &str) -> Option<&'a Element> {
    if el.local_name() == local {
        return Some(el);
    }
    for c in el.element_children() {
        if let Some(f) = find_desc(c, local) {
            return Some(f);
        }
    }
    None
}

fn descendants_local<'a>(el: &'a Element, local: &'a str) -> impl Iterator<Item = &'a Element> {
    fn rec<'a>(el: &'a Element, local: &str, out: &mut Vec<&'a Element>) {
        for c in el.element_children() {
            if c.local_name() == local {
                out.push(c);
            }
            rec(c, local, out);
        }
    }
    let mut out = Vec::new();
    rec(el, local, &mut out);
    out.into_iter()
}

fn has_local(el: &Element, local: &str) -> bool {
    if el.local_name() == local {
        return true;
    }
    el.element_children().any(|c| has_local(c, local))
}

fn join_t(el: &Element) -> String {
    let mut s = String::new();
    fn rec(el: &Element, s: &mut String) {
        if el.local_name() == "t" {
            s.push_str(&el.text_content());
        }
        for c in el.element_children() {
            rec(c, s);
        }
    }
    rec(el, &mut s);
    s
}

fn has_unquoted(s: &str, ch: char) -> bool {
    let mut in_q = false;
    for c in s.chars() {
        if c == '\'' {
            in_q = !in_q;
        } else if c == ch && !in_q {
            return true;
        }
    }
    false
}
