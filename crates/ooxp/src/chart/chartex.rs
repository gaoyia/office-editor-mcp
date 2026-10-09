//! Office 2016+ `cx:chartSpace`（chartex）。按 16.0 另存骨架写入，不渲染。

use serde_json::{json, Map, Value};

use super::embed;
use crate::chart::{format_a1_cell, format_a1_formula, NewSeries, CHARTEX_NS, REL_NS};
use crate::opc::{ContentTypes, Package, Relationship, Relationships};
use crate::xlsx::a1::CellRef;
use crate::xml::{self, Element};

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

/// 本期可写的 `cx:series/@layoutId`。`regionMap` 需要 geoCache，不写。
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

fn pts(vals: &[Value], numeric: bool) -> String {
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
        if numeric {
            body.push_str(&format!(r#"<cx:pt idx="{i}">{t}</cx:pt>"#));
        } else {
            body.push_str(&format!(r#"<cx:pt idx="{i}">{}</cx:pt>"#, escape(&t)));
        }
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
            pts(cats, false),
            vals.len(),
            pts(vals, true)
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
