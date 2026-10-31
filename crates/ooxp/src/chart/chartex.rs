//! Office 2016+ `cx:chartSpace`（chartex）。按 16.0 另存骨架写入，不渲染。

use serde_json::{json, Map, Value};

use super::embed;
use crate::chart::{
    format_a1_cell, format_a1_formula, parse_a1_formula, resize_a1, NewSeries, CHARTEX_NS, REL_NS,
};
use crate::opc::{ContentTypes, Package, Relationship, Relationships};
use crate::xlsx::a1::CellRef;
use crate::xml::{self, Element, Node};

pub const REL_CHARTEX: &str = "http://schemas.microsoft.com/office/2014/relationships/chartEx";
pub const CT_CHARTEX: &str = "application/vnd.ms-office.chartex+xml";
pub const REL_CHART_STYLE: &str =
    "http://schemas.microsoft.com/office/2011/relationships/chartStyle";
pub const REL_CHART_COLOR_STYLE: &str =
    "http://schemas.microsoft.com/office/2011/relationships/chartColorStyle";
pub const CT_CHART_STYLE: &str = "application/vnd.ms-office.chartstyle+xml";
pub const CT_CHART_COLOR_STYLE: &str = "application/vnd.ms-office.chartcolorstyle+xml";

const STYLE_XML: &str = include_str!("chartex_style.xml");
const COLORS_XML: &str = include_str!("chartex_colors.xml");

/// Office 16.0 打开 chartex 必须有 chartStyle / chartColorStyle 卫星件。
pub fn attach_style(
    pkg: &mut Package,
    chart_part: &str,
    rels: &mut Relationships,
    cts: &mut ContentTypes,
) -> Result<(), crate::error::Error> {
    let dir = chart_part
        .rsplit_once('/')
        .map(|(d, _)| d)
        .unwrap_or("/xl/charts");
    let style_part = embed::next_part(pkg, dir, "style", ".xml");
    let colors_part = embed::next_part(pkg, dir, "colors", ".xml");
    pkg.set(&style_part, STYLE_XML.as_bytes().to_vec())?;
    pkg.set(&colors_part, COLORS_XML.as_bytes().to_vec())?;
    let style_name = style_part.rsplit('/').next().unwrap_or("style1.xml");
    let colors_name = colors_part.rsplit('/').next().unwrap_or("colors1.xml");
    let style_rid = rels.next_rid();
    rels.push(Relationship {
        id: style_rid,
        rel_type: REL_CHART_STYLE.into(),
        target: style_name.into(),
        target_mode: None,
        extra: Vec::new(),
    });
    let colors_rid = rels.next_rid();
    rels.push(Relationship {
        id: colors_rid,
        rel_type: REL_CHART_COLOR_STYLE.into(),
        target: colors_name.into(),
        target_mode: None,
        extra: Vec::new(),
    });
    cts.set_override(&style_part, CT_CHART_STYLE)?;
    cts.set_override(&colors_part, CT_CHART_COLOR_STYLE)?;
    Ok(())
}

/// 可写的 `cx:series/@layoutId`。`regionMap` 不写：Excel 16.0 不另存地理缓存。
pub fn parse_layout_id(s: &str) -> Option<&'static str> {
    match s {
        "waterfall" => Some("waterfall"),
        "treemap" => Some("treemap"),
        "sunburst" => Some("sunburst"),
        "funnel" => Some("funnel"),
        "clusteredColumn" => Some("clusteredColumn"),
        "boxWhisker" => Some("boxWhisker"),
        "pareto" => Some("pareto"),
        _ => None,
    }
}

pub fn series_len_for(layout: &str) -> usize {
    match layout {
        "boxWhisker" => 3,
        "pareto" => 2,
        _ => 1,
    }
}

pub fn is_region_map(s: &str) -> bool {
    s == "regionMap"
}

fn escape(s: &str) -> String {
    xml::escape_text(s)
}

fn pts(vals: &[Value]) -> String {
    let mut body = String::new();
    for (i, v) in vals.iter().enumerate() {
        let t = match v {
            Value::Number(n) => n.to_string(),
            Value::String(s) => s.clone(),
            Value::Bool(b) => {
                if *b {
                    "TRUE".into()
                } else {
                    "FALSE".into()
                }
            }
            _ => v.to_string(),
        };
        body.push_str(&format!(r#"<cx:pt idx="{i}">{}</cx:pt>"#, escape(&t)));
    }
    body
}

fn unique_id(i: usize) -> String {
    // 不要用全零 GUID，Office 16.0 会当成损坏。
    format!("{{00000000-c11c-4000-8000-{i:012x}}}")
}

fn num_dim_type(layout: &str) -> &'static str {
    match layout {
        "treemap" | "sunburst" => "size",
        _ => "val",
    }
}

fn series_xml(
    layout: &str,
    idx: usize,
    name: &str,
    name_ref: Option<&str>,
    data_id: usize,
) -> String {
    let uid = unique_id(idx);
    let tx = if let Some(r) = name_ref {
        format!(
            r#"<cx:tx><cx:txData><cx:f>{}</cx:f><cx:v>{}</cx:v></cx:txData></cx:tx>"#,
            escape(r),
            escape(name)
        )
    } else {
        format!(
            r#"<cx:tx><cx:txData><cx:v>{}</cx:v></cx:txData></cx:tx>"#,
            escape(name)
        )
    };
    let labels = match layout {
        "funnel" => {
            r#"<cx:dataLabels><cx:visibility seriesName="0" categoryName="0" value="1"/></cx:dataLabels>"#
        }
        "waterfall" | "clusteredColumn" | "pareto" | "boxWhisker" => {
            r#"<cx:dataLabels pos="outEnd"><cx:visibility seriesName="0" categoryName="0" value="1"/></cx:dataLabels>"#
        }
        "treemap" => {
            r#"<cx:dataLabels pos="inEnd"><cx:visibility seriesName="0" categoryName="1" value="0"/></cx:dataLabels>"#
        }
        "sunburst" => {
            r#"<cx:dataLabels pos="ctr"><cx:visibility seriesName="0" categoryName="1" value="0"/></cx:dataLabels>"#
        }
        _ => "",
    };
    let layout_pr = match layout {
        "waterfall" => r#"<cx:layoutPr><cx:subtotals/></cx:layoutPr>"#,
        "treemap" => r#"<cx:layoutPr><cx:parentLabelLayout val="overlapping"/></cx:layoutPr>"#,
        _ => "",
    };
    let written = if layout == "pareto" && idx == 1 {
        "paretoLine"
    } else if layout == "pareto" {
        "clusteredColumn"
    } else {
        layout
    };
    format!(
        r#"<cx:series layoutId="{written}" uniqueId="{uid}">{tx}{labels}<cx:dataId val="{data_id}"/>{layout_pr}</cx:series>"#
    )
}

fn axes_xml(layout: &str) -> String {
    match layout {
        "funnel" => {
            r#"<cx:axis id="1"><cx:catScaling gapWidth="0.0599999987"/><cx:tickLabels/></cx:axis>"#
                .into()
        }
        "treemap" | "sunburst" => String::new(),
        _ => r#"<cx:axis id="0"><cx:catScaling gapWidth="0.5"/><cx:tickLabels/></cx:axis><cx:axis id="1"><cx:valScaling/><cx:majorGridlines/><cx:tickLabels/></cx:axis>"#.into(),
    }
}

fn legend_xml(layout: &str) -> String {
    match layout {
        "sunburst" | "funnel" => String::new(),
        _ => r#"<cx:legend pos="t" align="ctr" overlay="0"/>"#.into(),
    }
}

/// 写出一份最小 `cx:chartSpace`。`formulas` 为真时写 `cx:f` 与可选 `cx:externalData`。
pub fn build(
    layout: &str,
    title: Option<&str>,
    series: &[NewSeries],
    cats: &[Value],
    sheet: &str,
    origin: CellRef,
    formulas: bool,
    external: bool,
) -> String {
    let nd = num_dim_type(layout);
    let cat_start = CellRef {
        col: origin.col,
        row: origin.row + 1,
    };
    let cat_end = CellRef {
        col: origin.col,
        row: origin.row + cats.len() as u32,
    };
    let cat_ref = format_a1_formula(sheet, cat_start, cat_end);
    let mut data = String::new();
    let n_data = if layout == "pareto" {
        2
    } else {
        series.len().max(1)
    };
    for di in 0..n_data {
        let ser = series.get(di).or_else(|| series.first());
        let vals: &[Value] = ser.map(|s| s.values.as_slice()).unwrap_or(&[]);
        let val_col = origin.col + di as u32 + 1;
        let val_start = CellRef {
            col: val_col,
            row: origin.row + 1,
        };
        let val_end = CellRef {
            col: val_col,
            row: origin.row + vals.len() as u32,
        };
        let val_ref = format_a1_formula(sheet, val_start, val_end);
        let cat_f = if formulas {
            format!(r#"<cx:f>{}</cx:f>"#, escape(&cat_ref))
        } else {
            String::new()
        };
        let val_f = if formulas {
            format!(r#"<cx:f>{}</cx:f>"#, escape(&val_ref))
        } else {
            String::new()
        };
        data.push_str(&format!(
            r#"<cx:data id="{di}"><cx:strDim type="cat">{cat_f}<cx:lvl ptCount="{}">{}</cx:lvl></cx:strDim><cx:numDim type="{nd}">{val_f}<cx:lvl ptCount="{}" formatCode="General">{}</cx:lvl></cx:numDim></cx:data>"#,
            cats.len(),
            pts(cats),
            vals.len(),
            pts(vals)
        ));
    }
    let ext = if external {
        r#"<cx:externalData r:id="rId1" cx:autoUpdate="0"/>"#
    } else {
        ""
    };
    let title_el = match title {
        Some(t) if !t.is_empty() => format!(
            r#"<cx:title pos="t" align="ctr" overlay="0"><cx:tx><cx:txData><cx:v>{}</cx:v></cx:txData></cx:tx></cx:title>"#,
            escape(t)
        ),
        _ => r#"<cx:title pos="t" align="ctr" overlay="0"/>"#.into(),
    };
    let mut sers = String::new();
    if layout == "pareto" {
        let name0 = series
            .first()
            .and_then(|s| s.name.clone())
            .unwrap_or_else(|| "系列1".into());
        let name1 = series
            .get(1)
            .and_then(|s| s.name.clone())
            .unwrap_or_else(|| "系列2".into());
        let n0 = if formulas {
            Some(format_a1_cell(
                sheet,
                CellRef {
                    col: origin.col + 1,
                    row: origin.row,
                },
            ))
        } else {
            None
        };
        let n1 = if formulas {
            Some(format_a1_cell(
                sheet,
                CellRef {
                    col: origin.col + 2,
                    row: origin.row,
                },
            ))
        } else {
            None
        };
        sers.push_str(&series_xml(layout, 0, &name0, n0.as_deref(), 0));
        sers.push_str(&series_xml(layout, 1, &name1, n1.as_deref(), 1));
    } else {
        for (i, ser) in series.iter().enumerate() {
            let name = ser.name.clone().unwrap_or_else(|| format!("系列{}", i + 1));
            let nref = if formulas {
                Some(format_a1_cell(
                    sheet,
                    CellRef {
                        col: origin.col + i as u32 + 1,
                        row: origin.row,
                    },
                ))
            } else {
                None
            };
            sers.push_str(&series_xml(layout, i, &name, nref.as_deref(), i));
        }
    }
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><cx:chartSpace xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="{REL_NS}" xmlns:cx="{CHARTEX_NS}"><cx:chartData>{ext}{data}</cx:chartData><cx:chart>{title_el}<cx:plotArea><cx:plotAreaRegion>{sers}</cx:plotAreaRegion>{}</cx:plotArea>{}</cx:chart></cx:chartSpace>"#,
        axes_xml(layout),
        legend_xml(layout)
    )
}

fn lvl_pts(lvl: &Element) -> Vec<Value> {
    let mut pts: Vec<(u32, String)> = Vec::new();
    for pt in lvl.element_children() {
        if pt.local_name() != "pt" {
            continue;
        }
        let idx = pt
            .attr_local("idx")
            .and_then(|s| s.parse().ok())
            .unwrap_or(pts.len() as u32);
        pts.push((idx, pt.text_content()));
    }
    pts.sort_by_key(|(i, _)| *i);
    pts.into_iter()
        .map(|(_, t)| {
            if let Ok(i) = t.parse::<i64>() {
                json!(i)
            } else if let Ok(f) = t.parse::<f64>() {
                json!(f)
            } else {
                json!(t)
            }
        })
        .collect()
}

pub fn series_json(chart_id: &str, space: &Element) -> Vec<Value> {
    let mut data = std::collections::HashMap::new();
    if let Some(cd) = space.child_by_local("chartData") {
        for d in cd.element_children() {
            if d.local_name() != "data" {
                continue;
            }
            let id = d.attr_local("id").unwrap_or("0").to_string();
            let cats = d
                .element_children()
                .find(|c| c.local_name() == "strDim")
                .and_then(|s| s.child_by_local("lvl"))
                .map(lvl_pts)
                .unwrap_or_default();
            let vals = d
                .element_children()
                .find(|c| c.local_name() == "numDim")
                .and_then(|s| s.child_by_local("lvl"))
                .map(lvl_pts)
                .unwrap_or_default();
            data.insert(id, (cats, vals));
        }
    }
    let mut out = Vec::new();
    let Some(region) = space
        .child_by_local("chart")
        .and_then(|c| c.child_by_local("plotArea"))
        .and_then(|p| p.child_by_local("plotAreaRegion"))
    else {
        return out;
    };
    let mut index = 0usize;
    for ser in region.element_children() {
        if ser.local_name() != "series" {
            continue;
        }
        let name = ser
            .child_by_local("tx")
            .and_then(|tx| tx.child_by_local("txData"))
            .and_then(|tx| tx.child_by_local("v"))
            .map(|v| v.text_content())
            .filter(|s| !s.is_empty());
        let did = ser
            .child_by_local("dataId")
            .and_then(|e| e.attr_local("val"))
            .unwrap_or("0");
        let (cats, vals) = data.get(did).cloned().unwrap_or_default();
        let mut m = Map::new();
        m.insert("id".into(), json!(format!("{chart_id}/series:{index}")));
        m.insert("index".into(), json!(index));
        m.insert("name".into(), json!(name));
        m.insert("categories".into(), json!(cats));
        m.insert("values".into(), json!(vals));
        out.push(Value::Object(m));
        index += 1;
    }
    out
}

pub fn is_space(space: &Element) -> bool {
    space.child_by_local("chartData").is_some()
}

pub struct FormulaWrite {
    pub formula: String,
    pub values: Vec<Value>,
}

pub fn set_title(space: &mut Element, title: &str) {
    let Some(chart) = space.child_by_local_mut("chart") else {
        return;
    };
    if title.is_empty() {
        chart.children.retain(|n| match n {
            Node::Element(e) => e.local_name() != "title",
            _ => true,
        });
        return;
    }
    if chart.child_by_local("title").is_none() {
        let el = Element::new(chart.prefixed("title"))
            .with_attr("pos", "t")
            .with_attr("align", "ctr")
            .with_attr("overlay", "0");
        chart.children.insert(0, Node::Element(el));
    }
    let Some(title_el) = chart.child_by_local_mut("title") else {
        return;
    };
    let tx_name = title_el.prefixed("tx");
    let data_name = title_el.prefixed("txData");
    let v_name = title_el.prefixed("v");
    title_el.children.retain(|n| match n {
        Node::Element(e) => e.local_name() != "tx",
        _ => true,
    });
    let mut data = Element::new(data_name);
    data.push_element(Element::new(v_name).with_text(title));
    let mut tx = Element::new(tx_name);
    tx.push_element(data);
    title_el.push_element(tx);
}

pub fn set_legend(space: &mut Element, show: bool, pos: Option<&str>) {
    let Some(chart) = space.child_by_local_mut("chart") else {
        return;
    };
    let kept = chart
        .child_by_local("legend")
        .and_then(|l| l.attr("pos"))
        .unwrap_or("t")
        .to_string();
    chart.children.retain(|n| match n {
        Node::Element(e) => e.local_name() != "legend",
        _ => true,
    });
    if !show {
        return;
    }
    let pos = pos.unwrap_or(&kept);
    chart.push_element(
        Element::new(chart.prefixed("legend"))
            .with_attr("pos", pos)
            .with_attr("align", "ctr")
            .with_attr("overlay", "0"),
    );
}

pub fn edit_series(
    space: &mut Element,
    index: usize,
    name: Option<&str>,
    cats: Option<&[Value]>,
    vals: Option<&[Value]>,
) -> Option<Vec<FormulaWrite>> {
    let did = {
        let region = region_mut(space)?;
        let ser = nth_series_mut(region, index)?;
        if let Some(n) = name {
            set_tx_v(ser, n);
        }
        ser.child_by_local("dataId")
            .and_then(|e| e.attr_local("val"))
            .unwrap_or("0")
            .to_string()
    };
    let mut writes = Vec::new();
    if let Some(n) = name {
        if let Some(f) = series_name_formula(space, index) {
            writes.push(FormulaWrite {
                formula: f,
                values: vec![json!(n)],
            });
        }
    }
    let data = data_mut(space, &did)?;
    if let Some(c) = cats {
        if let Some(f) = set_dim_pts(data, "strDim", c) {
            writes.push(FormulaWrite {
                formula: f,
                values: c.to_vec(),
            });
        }
    }
    if let Some(v) = vals {
        if let Some(f) = set_dim_pts(data, "numDim", v) {
            writes.push(FormulaWrite {
                formula: f,
                values: v.to_vec(),
            });
        }
    }
    Some(writes)
}

fn region_mut(space: &mut Element) -> Option<&mut Element> {
    space
        .child_by_local_mut("chart")?
        .child_by_local_mut("plotArea")?
        .child_by_local_mut("plotAreaRegion")
}

fn nth_series_mut(region: &mut Element, index: usize) -> Option<&mut Element> {
    region
        .element_children_mut()
        .filter(|e| e.local_name() == "series")
        .nth(index)
}

fn data_mut<'a>(space: &'a mut Element, id: &str) -> Option<&'a mut Element> {
    space
        .child_by_local_mut("chartData")?
        .element_children_mut()
        .find(|e| e.local_name() == "data" && e.attr_local("id").unwrap_or("0") == id)
}

fn set_tx_v(ser: &mut Element, name: &str) {
    let Some(tx) = ser.child_by_local_mut("tx") else {
        return;
    };
    let Some(data) = tx.child_by_local_mut("txData") else {
        return;
    };
    if let Some(v) = data.child_by_local_mut("v") {
        v.set_text(name);
        return;
    }
    data.push_element(Element::new(data.prefixed("v")).with_text(name));
}

fn series_name_formula(space: &Element, index: usize) -> Option<String> {
    let region = space
        .child_by_local("chart")?
        .child_by_local("plotArea")?
        .child_by_local("plotAreaRegion")?;
    let ser = region
        .element_children()
        .filter(|e| e.local_name() == "series")
        .nth(index)?;
    ser.child_by_local("tx")
        .and_then(|tx| tx.child_by_local("txData"))
        .and_then(|d| d.child_by_local("f"))
        .map(|f| f.text_content())
        .filter(|s| !s.is_empty())
}

fn set_dim_pts(data: &mut Element, local: &str, values: &[Value]) -> Option<String> {
    let dim = data
        .element_children_mut()
        .find(|e| e.local_name() == local)?;
    let formula = resize_dim_formula(dim, values.len());
    let Some(lvl) = dim.child_by_local_mut("lvl") else {
        return formula;
    };
    lvl.set_attr("ptCount", values.len().to_string());
    lvl.children.retain(|n| match n {
        Node::Element(e) => e.local_name() != "pt",
        _ => true,
    });
    let pt_name = lvl.prefixed("pt");
    for (i, v) in values.iter().enumerate() {
        lvl.push_element(
            Element::new(&pt_name)
                .with_attr("idx", i.to_string())
                .with_text(value_text(v)),
        );
    }
    formula
}

fn resize_dim_formula(dim: &mut Element, n: usize) -> Option<String> {
    let f = dim.child_by_local_mut("f")?;
    let text = f.text_content();
    if text.is_empty() {
        return None;
    }
    if let Some(refer) = parse_a1_formula(&text) {
        if n == 0 {
            return Some(text);
        }
        let resized = resize_a1(&refer, n);
        let sheet = resized.sheet.clone().unwrap_or_else(|| "Sheet1".into());
        let next = format_a1_formula(&sheet, resized.start, resized.end);
        f.set_text(&next);
        return Some(next);
    }
    Some(text)
}

pub fn append_series(
    space: &mut Element,
    chart_id: &str,
    name: &str,
    cats: Option<&[Value]>,
    vals: &[Value],
    op_index: usize,
) -> Result<Vec<FormulaWrite>, crate::error::Error> {
    use crate::error::{Error, ErrorKind};
    if layout_is_fixed(space) {
        return Err(Error::with_path(
            ErrorKind::RestrictedChart {
                id: chart_id.to_string(),
            },
            format!("ops[{op_index}].id"),
        ));
    }
    let count = series_count(space);
    let Some(refer) = first_val_ref(space) else {
        return Err(Error::with_path(
            ErrorKind::ChartFormulaNotA1,
            format!("ops[{op_index}]"),
        ));
    };
    let cat_len = first_cat_len(space);
    if let Some(c) = cats {
        if cat_len.is_some_and(|n| n != c.len()) {
            return Err(Error::with_path(
                ErrorKind::ChartFormulaNotA1,
                format!("ops[{op_index}].categories"),
            ));
        }
    }
    let sheet = refer.sheet.clone().unwrap_or_else(|| "Sheet1".into());
    let col = refer.start.col + count as u32;
    let start = CellRef {
        col,
        row: refer.start.row,
    };
    let end = CellRef {
        col,
        row: refer.start.row + vals.len().saturating_sub(1) as u32,
    };
    let val_f = format_a1_formula(&sheet, start, end);
    let name_f = if refer.start.row > 1 {
        Some(format_a1_cell(
            &sheet,
            CellRef {
                col,
                row: refer.start.row - 1,
            },
        ))
    } else {
        None
    };
    let data_id = next_data_id(space);
    let layout = first_layout(space).unwrap_or_else(|| "funnel".into());
    let cat_f = first_cat_formula(space);
    let cat_vals = cats
        .map(|c| c.to_vec())
        .unwrap_or_else(|| first_cat_values(space));
    push_data(
        space,
        &data_id,
        &layout,
        cat_f.as_deref(),
        &cat_vals,
        &val_f,
        vals,
    );
    push_series(space, &layout, count, name, name_f.as_deref(), &data_id);
    let mut writes = vec![FormulaWrite {
        formula: val_f,
        values: vals.to_vec(),
    }];
    if let Some(f) = name_f {
        writes.push(FormulaWrite {
            formula: f,
            values: vec![json!(name)],
        });
    }
    Ok(writes)
}

pub fn remove_series(
    space: &mut Element,
    index: usize,
    id: &str,
    op_index: usize,
) -> Result<(), crate::error::Error> {
    use crate::error::{Error, ErrorKind};
    if layout_is_fixed(space) {
        return Err(Error::with_path(
            ErrorKind::RestrictedChart { id: id.to_string() },
            format!("ops[{op_index}].id"),
        ));
    }
    let n = series_count(space);
    if n <= 1 {
        return Err(Error::with_path(
            ErrorKind::LastChartSeries,
            format!("ops[{op_index}].id"),
        ));
    }
    let did = {
        let region = region_mut(space).ok_or_else(|| {
            Error::with_path(
                ErrorKind::ChartSeriesNotFound { id: id.to_string() },
                format!("ops[{op_index}].id"),
            )
        })?;
        let ser = nth_series_mut(region, index).ok_or_else(|| {
            Error::with_path(
                ErrorKind::ChartSeriesNotFound { id: id.to_string() },
                format!("ops[{op_index}].id"),
            )
        })?;
        ser.child_by_local("dataId")
            .and_then(|e| e.attr_local("val"))
            .unwrap_or("0")
            .to_string()
    };
    if let Some(region) = region_mut(space) {
        let mut seen = 0usize;
        region.children.retain(|n| match n {
            Node::Element(e) if e.local_name() == "series" => {
                let keep = seen != index;
                seen += 1;
                keep
            }
            _ => true,
        });
    }
    if let Some(parent) = space.child_by_local_mut("chartData") {
        parent.children.retain(|n| match n {
            Node::Element(e) if e.local_name() == "data" => {
                e.attr_local("id").unwrap_or("0") != did
            }
            _ => true,
        });
    }
    Ok(())
}

fn layout_is_fixed(space: &Element) -> bool {
    let Some(region) = space
        .child_by_local("chart")
        .and_then(|c| c.child_by_local("plotArea"))
        .and_then(|p| p.child_by_local("plotAreaRegion"))
    else {
        return false;
    };
    region.element_children().any(|e| {
        e.local_name() == "series"
            && matches!(e.attr("layoutId"), Some("paretoLine" | "boxWhisker"))
    })
}

fn series_count(space: &Element) -> usize {
    space
        .child_by_local("chart")
        .and_then(|c| c.child_by_local("plotArea"))
        .and_then(|p| p.child_by_local("plotAreaRegion"))
        .map(|r| {
            r.element_children()
                .filter(|e| e.local_name() == "series")
                .count()
        })
        .unwrap_or(0)
}

fn first_layout(space: &Element) -> Option<String> {
    space
        .child_by_local("chart")
        .and_then(|c| c.child_by_local("plotArea"))
        .and_then(|p| p.child_by_local("plotAreaRegion"))
        .and_then(|r| {
            r.element_children()
                .find(|e| e.local_name() == "series")
                .and_then(|s| s.attr("layoutId").map(str::to_string))
        })
}

fn next_data_id(space: &Element) -> String {
    let max = space
        .child_by_local("chartData")
        .map(|d| {
            d.element_children()
                .filter(|e| e.local_name() == "data")
                .filter_map(|e| e.attr_local("id").and_then(|s| s.parse::<u32>().ok()))
                .max()
                .unwrap_or(0)
        })
        .unwrap_or(0);
    (max + 1).to_string()
}

fn first_val_ref(space: &Element) -> Option<crate::chart::A1Ref> {
    let data = space.child_by_local("chartData")?;
    let first = data.element_children().find(|e| e.local_name() == "data")?;
    let f = first
        .element_children()
        .find(|e| e.local_name() == "numDim")?
        .child_by_local("f")?
        .text_content();
    parse_a1_formula(&f)
}

fn first_cat_formula(space: &Element) -> Option<String> {
    let data = space.child_by_local("chartData")?;
    let first = data.element_children().find(|e| e.local_name() == "data")?;
    Some(
        first
            .element_children()
            .find(|e| e.local_name() == "strDim")?
            .child_by_local("f")?
            .text_content(),
    )
    .filter(|s| !s.is_empty())
}

fn first_cat_len(space: &Element) -> Option<usize> {
    Some(first_cat_values(space).len()).filter(|n| *n > 0)
}

fn first_cat_values(space: &Element) -> Vec<Value> {
    let Some(data) = space.child_by_local("chartData") else {
        return Vec::new();
    };
    let Some(first) = data.element_children().find(|e| e.local_name() == "data") else {
        return Vec::new();
    };
    first
        .element_children()
        .find(|e| e.local_name() == "strDim")
        .and_then(|s| s.child_by_local("lvl"))
        .map(lvl_pts)
        .unwrap_or_default()
}

fn push_data(
    space: &mut Element,
    id: &str,
    layout: &str,
    cat_f: Option<&str>,
    cats: &[Value],
    val_f: &str,
    vals: &[Value],
) {
    let Some(parent) = space.child_by_local_mut("chartData") else {
        return;
    };
    let nd = num_dim_type(layout);
    let mut str_dim = Element::new(parent.prefixed("strDim")).with_attr("type", "cat");
    if let Some(f) = cat_f {
        str_dim.push_element(Element::new(str_dim.prefixed("f")).with_text(f));
    }
    let mut cat_lvl =
        Element::new(str_dim.prefixed("lvl")).with_attr("ptCount", cats.len().to_string());
    fill_pts(&mut cat_lvl, cats);
    str_dim.push_element(cat_lvl);
    let mut num_dim = Element::new(parent.prefixed("numDim")).with_attr("type", nd);
    num_dim.push_element(Element::new(num_dim.prefixed("f")).with_text(val_f));
    let mut val_lvl = Element::new(num_dim.prefixed("lvl"))
        .with_attr("ptCount", vals.len().to_string())
        .with_attr("formatCode", "General");
    fill_pts(&mut val_lvl, vals);
    num_dim.push_element(val_lvl);
    let mut data = Element::new(parent.prefixed("data")).with_attr("id", id);
    data.push_element(str_dim);
    data.push_element(num_dim);
    parent.push_element(data);
}

fn push_series(
    space: &mut Element,
    layout: &str,
    index: usize,
    name: &str,
    name_f: Option<&str>,
    data_id: &str,
) {
    let Some(region) = region_mut(space) else {
        return;
    };
    let xml = series_xml(
        layout,
        index,
        name,
        name_f,
        data_id.parse().unwrap_or(index),
    );
    let wrapped = format!(r#"<cx:region xmlns:cx="{CHARTEX_NS}">{xml}</cx:region>"#);
    if let Ok(doc) = xml::parse(wrapped.as_bytes()) {
        if let Some(ser) = doc.root.element_children().next().cloned() {
            region.push_element(ser);
        }
    }
}

fn fill_pts(lvl: &mut Element, values: &[Value]) {
    let pt_name = lvl.prefixed("pt");
    for (i, v) in values.iter().enumerate() {
        lvl.push_element(
            Element::new(&pt_name)
                .with_attr("idx", i.to_string())
                .with_text(value_text(v)),
        );
    }
}

fn value_text(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(true) => "TRUE".into(),
        Value::Bool(false) => "FALSE".into(),
        other => other.to_string(),
    }
}
