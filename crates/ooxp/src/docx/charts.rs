//! 正文与页眉页脚里的嵌入图表：名单、改数据、插删、改类型。不渲染。

use std::collections::{HashMap, HashSet};

use serde_json::{json, Number, Value};

use crate::error::{Error, ErrorKind, IdReason};
use crate::opc::{
    rels_part_name, resolve_rel_target, split_dir_file, ContentTypes, Package, Relationship,
    Relationships,
};
use crate::xml::{self, Document, Element, Node};
use crate::xlsx;

use super::hf::{HfKind, HfPart, HfType};
use super::para::{self, BodyPara, ParaTarget};
use super::picture::{self, PicKind, WrapKind, WrapPos};
use super::{A_NS, CT_CHART, REL_CHART, REL_NS};

pub const CHART_NS: &str = "http://schemas.openxmlformats.org/drawingml/2006/chart";
pub const CHARTEX_NS: &str = "http://schemas.microsoft.com/office/drawing/2014/chartex";
pub const REL_PACKAGE: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/package";
pub const CT_EMBED_XLSX: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet";

const DEFAULT_CX: i64 = 5_486_400;
const DEFAULT_CY: i64 = 3_200_400;

const WRAPPERS: &[&str] = &[
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChartType {
    Bar,
    Col,
    Line,
    Pie,
    Area,
    Scatter,
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
        }
    }

    fn wrapper(self) -> &'static str {
        match self {
            Self::Bar | Self::Col => "barChart",
            Self::Line => "lineChart",
            Self::Pie => "pieChart",
            Self::Area => "areaChart",
            Self::Scatter => "scatterChart",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChartKey {
    DocPr(String),
    Index(usize),
}

#[derive(Debug, Clone)]
pub enum ChartTarget {
    Body {
        para: BodyPara,
        key: ChartKey,
    },
    HeaderFooter {
        kind: HfKind,
        hf_type: HfType,
        n: u32,
        para: BodyPara,
        key: ChartKey,
    },
}

impl ChartTarget {
    fn para(&self) -> &BodyPara {
        match self {
            Self::Body { para, .. } | Self::HeaderFooter { para, .. } => para,
        }
    }

    fn key(&self) -> &ChartKey {
        match self {
            Self::Body { key, .. } | Self::HeaderFooter { key, .. } => key,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SeriesTarget {
    pub chart: ChartTarget,
    pub index: usize,
}

#[derive(Debug, Clone)]
pub struct NewSeries {
    pub name: Option<String>,
    pub categories: Option<Vec<Value>>,
    pub values: Vec<Value>,
}

#[derive(Debug, Clone)]
pub struct InsertChartOp {
    pub id: String,
    pub para: ParaTarget,
    pub chart_type: ChartType,
    pub title: Option<String>,
    pub series: Vec<NewSeries>,
    pub kind: PicKind,
    pub wrap: Option<WrapKind>,
    pub pos: WrapPos,
    pub cx: i64,
    pub cy: i64,
}

#[derive(Debug, Clone)]
pub struct DeleteChartOp {
    pub id: String,
    pub target: ChartTarget,
}

#[derive(Debug, Clone)]
pub struct SetChartTitleOp {
    pub id: String,
    pub target: ChartTarget,
    pub title: String,
}

#[derive(Debug, Clone)]
pub struct SetChartSeriesOp {
    pub id: String,
    pub target: SeriesTarget,
    pub name: Option<String>,
    pub categories: Option<Vec<Value>>,
    pub values: Option<Vec<Value>>,
}

#[derive(Debug, Clone)]
pub struct AddChartSeriesOp {
    pub id: String,
    pub target: ChartTarget,
    pub name: Option<String>,
    pub categories: Option<Vec<Value>>,
    pub values: Vec<Value>,
}

#[derive(Debug, Clone)]
pub struct DeleteChartSeriesOp {
    pub id: String,
    pub target: SeriesTarget,
}

#[derive(Debug, Clone)]
pub struct SetChartTypeOp {
    pub id: String,
    pub target: ChartTarget,
    pub chart_type: ChartType,
}

#[derive(Debug, Clone)]
pub struct SetChartAxisTitleOp {
    pub id: String,
    pub target: ChartTarget,
    pub axis: String,
    pub title: String,
}

#[derive(Debug, Clone)]
pub struct SetChartLegendOp {
    pub id: String,
    pub target: ChartTarget,
    pub show: bool,
    pub pos: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SetChartDataLabelsOp {
    pub id: String,
    pub target: ChartTarget,
    pub show: bool,
}

#[derive(Debug, Clone)]
pub struct SetChartSizeOp {
    pub id: String,
    pub target: ChartTarget,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct SetChartWrapOp {
    pub id: String,
    pub target: ChartTarget,
    pub kind: PicKind,
    pub wrap: Option<WrapKind>,
    pub pos: WrapPos,
}

pub fn default_cx() -> i64 {
    DEFAULT_CX
}

pub fn default_cy() -> i64 {
    DEFAULT_CY
}

/// 只计该树上被收入的图表（遇文本框内容停住）。
pub fn count_charts(root: &Element) -> Result<usize, Error> {
    let mut n = 0usize;
    picture::walk_drawings(root, &mut |d| {
        if is_chart(d) {
            n += 1;
        }
        Ok(())
    })?;
    Ok(n)
}

pub fn collect_in_para(
    pkg: &Package,
    host_part: &str,
    para: &Element,
    para_id: &str,
) -> Result<Vec<Value>, Error> {
    let mut out = Vec::new();
    let mut index = 0usize;
    picture::walk_drawings(para, &mut |drawing| {
        if !is_chart(drawing) {
            return Ok(());
        }
        index += 1;
        out.push(chart_json(pkg, host_part, drawing, para_id, index)?);
        Ok(())
    })?;
    Ok(out)
}

pub fn collect_body(
    pkg: &Package,
    doc_part: &str,
    paras: &[&Element],
) -> Result<Vec<Value>, Error> {
    let mut out = Vec::new();
    for (i, p) in paras.iter().enumerate() {
        let (id, _) = para::para_stable_id(p, i + 1);
        out.extend(collect_in_para(pkg, doc_part, p, &id)?);
    }
    Ok(out)
}

pub fn collect_hf(
    pkg: &Package,
    item: &HfPart,
    paras: &[&Element],
) -> Result<Vec<Value>, Error> {
    let mut out = Vec::new();
    for (i, p) in paras.iter().enumerate() {
        let (id, _) = para::para_stable_id_in(&item.id, p, i + 1);
        out.extend(collect_in_para(pkg, &item.part, p, &id)?);
    }
    Ok(out)
}

pub fn parse_chart_id(id: &str, op_index: usize) -> Result<ChartTarget, Error> {
    parse_chart_id_at(id, &format!("ops[{op_index}].id"))
}

pub fn parse_chart_id_at(id: &str, path: &str) -> Result<ChartTarget, Error> {
    if id.contains("/series:") {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::ChartPrefix,
            },
            path.to_string(),
        ));
    }
    let Some((left, key_s)) = id.rsplit_once("/chart:") else {
        if looks_like_para_or_container(id) {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::ChartContainer,
                },
                path.to_string(),
            ));
        }
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::ChartPrefix,
            },
            path.to_string(),
        ));
    };
    if left.contains("/txbx:")
        || left.starts_with("footnote:")
        || left.starts_with("endnote:")
        || left.starts_with("comment:")
    {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::ChartPrefix,
            },
            path.to_string(),
        ));
    }
    let key = parse_chart_key(key_s, path)?;
    match para::parse_para_id_at(left, path)? {
        ParaTarget::Body(para) => Ok(ChartTarget::Body { para, key }),
        ParaTarget::HeaderFooter {
            kind,
            hf_type,
            n,
            para,
        } => Ok(ChartTarget::HeaderFooter {
            kind,
            hf_type,
            n,
            para,
            key,
        }),
        _ => Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::ChartPrefix,
            },
            path.to_string(),
        )),
    }
}

pub fn parse_series_id(id: &str, op_index: usize) -> Result<SeriesTarget, Error> {
    let path = format!("ops[{op_index}].id");
    let Some((left, idx_s)) = id.rsplit_once("/series:") else {
        if id.contains("/chart:") {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::SeriesPrefix,
                },
                path,
            ));
        }
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SeriesPrefix,
            },
            path,
        ));
    };
    let index: usize = idx_s.parse().map_err(|_| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SeriesIndexNotInt,
            },
            path.clone(),
        )
    })?;
    let chart = parse_chart_id_at(left, &path)?;
    Ok(SeriesTarget { chart, index })
}

pub fn parse_insert_para_id(id: &str, op_index: usize) -> Result<ParaTarget, Error> {
    let path = format!("ops[{op_index}].id");
    if id.contains("/chart:")
        || id.contains("/series:")
        || id.contains("/pic:")
        || id.contains("/smartdiagram:")
        || id.contains("/pt:")
    {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::InsertChartNeedsPara,
            },
            path,
        ));
    }
    if let Some((_, after)) = id.split_once("/txbx:") {
        if !after.contains("/para:") {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::InsertChartNeedsPara,
                },
                path,
            ));
        }
    }
    let target = para::parse_para_id_at(id, &path)?;
    match target {
        ParaTarget::Body(_) | ParaTarget::HeaderFooter { .. } => Ok(target),
        _ => Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::InsertChartNeedsPara,
            },
            path,
        )),
    }
}

pub fn export_chart_data(
    bytes: &[u8],
    id: &str,
) -> Result<super::export::ExportedMedia, Error> {
    let pkg = Package::open_bytes(bytes.to_vec())?;
    let target = parse_chart_id_at(id, "id")?;
    let doc_part = crate::opc::office_document_part(&pkg)?;
    let doc_xml = xml::parse(pkg.get(&doc_part)?)?;
    let catalog = super::hf::catalog(&pkg, &doc_part, &doc_xml.root)?;
    let host = part_for_target(&pkg, &catalog, &doc_part, &target, id, "id")?;
    let xml_doc = if host == doc_part {
        doc_xml
    } else {
        xml::parse(pkg.get(&host)?)?
    };
    let drawing = find_drawing(&xml_doc.root, &target).ok_or_else(|| {
        Error::with_path(ErrorKind::ChartNotFound { id: id.to_string() }, "id")
    })?;
    let rid = chart_rid(drawing).ok_or_else(|| {
        Error::with_path(ErrorKind::ChartNotFound { id: id.to_string() }, "id")
    })?;
    let chart_part = crate::opc::resolve_id(&pkg, &host, &rid)?;
    let chart_doc = xml::parse(pkg.get(&chart_part)?)?;
    let embed_rid = external_data_rid(&chart_doc.root).ok_or_else(|| {
        Error::with_path(
            ErrorKind::PartMissing {
                name: format!("{id} embedding"),
            },
            "id",
        )
    })?;
    let embed = crate::opc::resolve_id(&pkg, &chart_part, &embed_rid)?;
    let embed_bytes = pkg.get(&embed)?.to_vec();
    let cts = ContentTypes::from_package(&pkg)?;
    let content_type = cts.content_type_of(&embed).map(str::to_string);
    Ok(super::export::ExportedMedia {
        bytes: embed_bytes,
        content_type,
    })
}

pub fn apply_insert(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[HfPart],
    op: &InsertChartOp,
    i: usize,
) -> Result<(), Error> {
    let host = part_for_para(catalog, doc_part, &op.para, &op.id, i)?;
    {
        let doc = load_part(pkg, docs, &host)?;
        if find_para_mut(&mut doc.root, &op.para).is_none() {
            return Err(Error::with_path(
                ErrorKind::ParaNotFound { id: op.id.clone() },
                format!("ops[{i}].id"),
            ));
        }
    }
    let cats = shared_categories(&op.series);
    let embed = build_embedding(&op.series, &cats)?;
    let embed_part = next_numbered(pkg, "/word/embeddings", "Microsoft_Excel_Worksheet", ".xlsx");
    let chart_part = next_numbered(pkg, "/word/charts", "chart", ".xml");
    pkg.set(&embed_part, embed)?;
    let chart_xml = build_chart_xml(op.chart_type, op.title.as_deref(), &op.series, &cats);
    pkg.set(&chart_part, chart_xml.into_bytes())?;

    let mut chart_rels = Relationships { rels: Vec::new() };
    chart_rels.push(Relationship {
        id: "rId1".into(),
        rel_type: REL_PACKAGE.into(),
        target: picture::rel_target_from(&chart_part, &embed_part),
        target_mode: None,
        extra: Vec::new(),
    });
    chart_rels.write_to(pkg, &chart_part)?;

    let mut host_rels = Relationships::from_package(pkg, &host)?;
    let chart_rid = host_rels.next_rid();
    host_rels.push(Relationship {
        id: chart_rid.clone(),
        rel_type: REL_CHART.into(),
        target: picture::rel_target_from(&host, &chart_part),
        target_mode: None,
        extra: Vec::new(),
    });
    host_rels.write_to(pkg, &host)?;

    let mut cts = ContentTypes::from_package(pkg)?;
    cts.set_override(&chart_part, CT_CHART)?;
    cts.set_override(&embed_part, CT_EMBED_XLSX)?;
    cts.write_to(pkg)?;

    let doc = load_part(pkg, docs, &host)?;
    ensure_chart_ns(&mut doc.root);
    let doc_pr = picture::next_docpr_id(&doc.root);
    let p = find_para_mut(&mut doc.root, &op.para).ok_or_else(|| {
        Error::with_path(
            ErrorKind::ParaNotFound { id: op.id.clone() },
            format!("ops[{i}].id"),
        )
    })?;
    let mut drawing = build_chart_drawing(doc_pr, &chart_rid, op.cx, op.cy);
    picture::apply_wrap(&mut drawing, op.kind, op.wrap, &op.pos)
        .map_err(|e| e.at(format!("ops[{i}]")))?;
    picture::insert_drawing_into_para(p, drawing);
    dirty.insert(host, ());
    Ok(())
}

pub fn apply_delete(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[HfPart],
    op: &DeleteChartOp,
    i: usize,
) -> Result<(), Error> {
    let host = part_for_target(pkg, catalog, doc_part, &op.target, &op.id, &format!("ops[{i}].id"))?;
    let (chart_rid, chart_part, satellites) = {
        let doc = load_part(pkg, docs, &host)?;
        let drawing = find_drawing(&doc.root, &op.target).ok_or_else(|| {
            Error::with_path(
                ErrorKind::ChartNotFound { id: op.id.clone() },
                format!("ops[{i}].id"),
            )
        })?;
        let rid = chart_rid(drawing).ok_or_else(|| {
            Error::with_path(
                ErrorKind::ChartNotFound { id: op.id.clone() },
                format!("ops[{i}].id"),
            )
        })?;
        let part = crate::opc::resolve_id(pkg, &host, &rid)?;
        let sats = chart_satellites(pkg, &part)?;
        (rid, part, sats)
    };
    {
        let doc = load_part(pkg, docs, &host)?;
        if !remove_chart(&mut doc.root, op.target.key()) {
            return Err(Error::with_path(
                ErrorKind::ChartNotFound { id: op.id.clone() },
                format!("ops[{i}].id"),
            ));
        }
    }
    dirty.insert(host.clone(), ());
    remove_rel_if_unused(pkg, &host, &chart_rid, &docs.get(&host).unwrap().root)?;
    drop_unused(pkg, &chart_part)?;
    for s in satellites {
        drop_unused(pkg, &s)?;
    }
    Ok(())
}

pub fn apply_title(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    doc_part: &str,
    catalog: &[HfPart],
    op: &SetChartTitleOp,
    i: usize,
) -> Result<(), Error> {
    let mut space = load_chart_space(pkg, docs, doc_part, catalog, &op.target, &op.id, i)?;
    reject_restricted(&space.0.root, &op.id, i)?;
    set_chart_title(&mut space.0.root, &op.title);
    pkg.set(&space.1, space.0.to_bytes()?)?;
    Ok(())
}

pub fn apply_series(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    doc_part: &str,
    catalog: &[HfPart],
    op: &SetChartSeriesOp,
    i: usize,
) -> Result<(), Error> {
    let (mut space, chart_part, host) =
        load_chart_space_host(pkg, docs, doc_part, catalog, &op.target.chart, &op.id, i)?;
    reject_restricted(&space.root, &op.id, i)?;
    let n = series_count(&space.root);
    if op.target.index >= n {
        return Err(Error::with_path(
            ErrorKind::ChartSeriesNotFound { id: op.id.clone() },
            format!("ops[{i}].id"),
        ));
    }
    write_series(
        pkg,
        &mut space.root,
        &chart_part,
        op.target.index,
        op.name.as_deref(),
        op.categories.as_deref(),
        op.values.as_deref(),
        i,
    )?;
    pkg.set(&chart_part, space.to_bytes()?)?;
    let _ = host;
    Ok(())
}

pub fn apply_add_series(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    doc_part: &str,
    catalog: &[HfPart],
    op: &AddChartSeriesOp,
    i: usize,
) -> Result<(), Error> {
    let (mut space, chart_part, _) =
        load_chart_space_host(pkg, docs, doc_part, catalog, &op.target, &op.id, i)?;
    reject_restricted(&space.root, &op.id, i)?;
    if inspect_type(&space.root, false) == "pie" {
        return Err(Error::with_path(
            ErrorKind::PieNeedsOneSeries,
            format!("ops[{i}]"),
        ));
    }
    add_series(
        pkg,
        &mut space.root,
        &chart_part,
        op.name.as_deref(),
        op.categories.as_deref(),
        &op.values,
        i,
    )?;
    pkg.set(&chart_part, space.to_bytes()?)?;
    Ok(())
}

pub fn apply_delete_series(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    doc_part: &str,
    catalog: &[HfPart],
    op: &DeleteChartSeriesOp,
    i: usize,
) -> Result<(), Error> {
    let (mut space, chart_part, _) =
        load_chart_space_host(pkg, docs, doc_part, catalog, &op.target.chart, &op.id, i)?;
    reject_restricted(&space.root, &op.id, i)?;
    let n = series_count(&space.root);
    if op.target.index >= n {
        return Err(Error::with_path(
            ErrorKind::ChartSeriesNotFound { id: op.id.clone() },
            format!("ops[{i}].id"),
        ));
    }
    if n <= 1 {
        return Err(Error::with_path(
            ErrorKind::LastChartSeries,
            format!("ops[{i}].id"),
        ));
    }
    remove_series_at(&mut space.root, op.target.index);
    pkg.set(&chart_part, space.to_bytes()?)?;
    Ok(())
}

pub fn apply_type(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    doc_part: &str,
    catalog: &[HfPart],
    op: &SetChartTypeOp,
    i: usize,
) -> Result<(), Error> {
    let (mut space, chart_part, _) =
        load_chart_space_host(pkg, docs, doc_part, catalog, &op.target, &op.id, i)?;
    reject_restricted(&space.root, &op.id, i)?;
    convert_type(&mut space.root, op.chart_type, i)?;
    pkg.set(&chart_part, space.to_bytes()?)?;
    Ok(())
}

pub fn apply_axis_title(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    doc_part: &str,
    catalog: &[HfPart],
    op: &SetChartAxisTitleOp,
    i: usize,
) -> Result<(), Error> {
    let (mut space, chart_part, _) =
        load_chart_space_host(pkg, docs, doc_part, catalog, &op.target, &op.id, i)?;
    reject_restricted(&space.root, &op.id, i)?;
    set_axis_title(&mut space.root, &op.axis, &op.title, i)?;
    pkg.set(&chart_part, space.to_bytes()?)?;
    Ok(())
}

pub fn apply_legend(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    doc_part: &str,
    catalog: &[HfPart],
    op: &SetChartLegendOp,
    i: usize,
) -> Result<(), Error> {
    let (mut space, chart_part, _) =
        load_chart_space_host(pkg, docs, doc_part, catalog, &op.target, &op.id, i)?;
    reject_restricted(&space.root, &op.id, i)?;
    set_legend(&mut space.root, op.show, op.pos.as_deref());
    pkg.set(&chart_part, space.to_bytes()?)?;
    Ok(())
}

pub fn apply_data_labels(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    doc_part: &str,
    catalog: &[HfPart],
    op: &SetChartDataLabelsOp,
    i: usize,
) -> Result<(), Error> {
    let (mut space, chart_part, _) =
        load_chart_space_host(pkg, docs, doc_part, catalog, &op.target, &op.id, i)?;
    reject_restricted(&space.root, &op.id, i)?;
    set_data_labels(&mut space.root, op.show);
    pkg.set(&chart_part, space.to_bytes()?)?;
    Ok(())
}

pub fn apply_size(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[HfPart],
    op: &SetChartSizeOp,
    i: usize,
) -> Result<(), Error> {
    let host = part_for_target(pkg, catalog, doc_part, &op.target, &op.id, &format!("ops[{i}].id"))?;
    let doc = load_part(pkg, docs, &host)?;
    let drawing = find_drawing_mut(&mut doc.root, &op.target).ok_or_else(|| {
        Error::with_path(
            ErrorKind::ChartNotFound { id: op.id.clone() },
            format!("ops[{i}].id"),
        )
    })?;
    if !picture::set_extent(drawing, op.cx, op.cy) {
        return Err(Error::with_path(
            ErrorKind::ChartNotFound { id: op.id.clone() },
            format!("ops[{i}].id"),
        ));
    }
    dirty.insert(host, ());
    Ok(())
}

pub fn apply_wrap(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[HfPart],
    op: &SetChartWrapOp,
    i: usize,
) -> Result<(), Error> {
    let host = part_for_target(pkg, catalog, doc_part, &op.target, &op.id, &format!("ops[{i}].id"))?;
    let doc = load_part(pkg, docs, &host)?;
    ensure_chart_ns(&mut doc.root);
    let drawing = find_drawing_mut(&mut doc.root, &op.target).ok_or_else(|| {
        Error::with_path(
            ErrorKind::ChartNotFound { id: op.id.clone() },
            format!("ops[{i}].id"),
        )
    })?;
    picture::apply_wrap(drawing, op.kind, op.wrap, &op.pos)
        .map_err(|e| e.at(format!("ops[{i}]")))?;
    dirty.insert(host, ());
    Ok(())
}

fn chart_json(
    pkg: &Package,
    host_part: &str,
    drawing: &Element,
    para_id: &str,
    index: usize,
) -> Result<Value, Error> {
    let frame = frame(drawing);
    let kind = match frame.map(Element::local_name) {
        Some("anchor") => PicKind::Anchor,
        _ => PicKind::Inline,
    };
    let doc_pr = frame.and_then(|f| f.child_by_local("docPr"));
    let raw_id = doc_pr
        .and_then(|d| d.attr_local("id"))
        .filter(|s| !s.is_empty());
    let name = doc_pr
        .and_then(|d| d.attr_local("name"))
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    let (id, stable, doc_pr_id) = match raw_id {
        Some(v) => (format!("{para_id}/chart:{v}"), true, Some(v.to_string())),
        None => (
            format!("{para_id}/chart:index:{index}"),
            false,
            None,
        ),
    };
    let extent = frame.and_then(|f| f.child_by_local("extent"));
    let cx = parse_emu(extent, "cx")?;
    let cy = parse_emu(extent, "cy")?;
    let wrap = if kind == PicKind::Anchor {
        frame.and_then(|f| {
            f.element_children()
                .find_map(|c| WrapKind::from_local(c.local_name()))
        })
    } else {
        None
    };
    let chartex = is_chartex(drawing);
    let mut ty = if chartex {
        "chartex".to_string()
    } else {
        "other".to_string()
    };
    let mut title = Value::Null;
    let mut cat_axis = Value::Null;
    let mut val_axis = Value::Null;
    let mut series = Vec::new();
    let mut legend = json!({ "show": false, "pos": null });
    let mut data_labels = json!({ "show": false });
    let mut flags: Vec<Value> = Vec::new();
    if let Some(rid) = chart_rid(drawing) {
        if let Ok(chart_part) = crate::opc::resolve_id(pkg, host_part, &rid) {
            if let Ok(bytes) = pkg.get(&chart_part) {
                if let Ok(doc) = xml::parse(bytes) {
                    let embed = load_embed(pkg, &chart_part, &doc.root);
                    if !chartex {
                        ty = inspect_type(&doc.root, false);
                    }
                    title = json!(chart_title(&doc.root));
                    let (c, v) = axis_titles(&doc.root, ty == "scatter");
                    cat_axis = json!(c);
                    val_axis = json!(v);
                    series = series_json(&id, &doc.root, embed.as_deref())?;
                    legend = legend_json(&doc.root);
                    data_labels = json!({ "show": data_labels_on(&doc.root) });
                    flags = flags_json(&doc.root, &ty);
                }
            }
        }
    }
    Ok(json!({
        "id": id,
        "docPrId": doc_pr_id,
        "name": name,
        "kind": kind.as_str(),
        "cx": cx,
        "cy": cy,
        "wrap": wrap.map(WrapKind::as_str),
        "type": ty,
        "title": title,
        "catAxisTitle": cat_axis,
        "valAxisTitle": val_axis,
        "series": series,
        "legend": legend,
        "dataLabels": data_labels,
        "flags": flags,
        "stable": stable,
    }))
}

fn series_json(chart_id: &str, space: &Element, embed: Option<&[u8]>) -> Result<Vec<Value>, Error> {
    let mut out = Vec::new();
    for (index, ser) in collect_sers(space).into_iter().enumerate() {
        let scatter = ancestor_is(ser, "scatterChart");
        let name = series_name(ser);
        let (cat_local, val_local) = if scatter {
            ("xVal", "yVal")
        } else {
            ("cat", "val")
        };
        let (cats, cat_stale) = read_pts(ser, cat_local, embed);
        let (vals, val_stale) = read_pts(ser, val_local, embed);
        let mut node = json!({
            "id": format!("{chart_id}/series:{index}"),
            "index": index,
            "name": name,
            "categories": cats,
            "values": vals,
        });
        if cat_stale || val_stale {
            node["cacheStale"] = json!(true);
        }
        out.push(node);
    }
    Ok(out)
}

fn read_pts(ser: &Element, local: &str, embed: Option<&[u8]>) -> (Vec<Value>, bool) {
    let Some(el) = find_child(ser, local) else {
        return (Vec::new(), false);
    };
    let cache = cache_values(el);
    let formula = formula_of(el);
    let embed_vals = match (embed, formula.as_deref()) {
        (Some(bytes), Some(f)) => read_embed_formula(bytes, f),
        _ => None,
    };
    match (cache, embed_vals) {
        (Some(c), Some(e)) => {
            let stale = !values_equal(&c, &e);
            (e, stale)
        }
        (None, Some(e)) => (e, false),
        (Some(c), None) => (c, false),
        (None, None) => (Vec::new(), false),
    }
}

fn cache_values(el: &Element) -> Option<Vec<Value>> {
    let cache = find_desc(el, "numCache")
        .or_else(|| find_desc(el, "strCache"))
        .or_else(|| find_desc(el, "numLit"))
        .or_else(|| find_desc(el, "strLit"));
    let cache = cache?;
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

fn formula_of(el: &Element) -> Option<String> {
    find_desc(el, "f").map(|f| f.text_content())
}

fn read_embed_formula(bytes: &[u8], formula: &str) -> Option<Vec<Value>> {
    let (sheet, range) = parse_chart_formula(formula)?;
    let scope = xlsx::parse_scope(Some(&json!({
        "sheet": sheet,
        "range": range,
        "maxCells": 20000
    })))
    .ok()??;
    let tree = xlsx::inspect(bytes, Some(scope)).ok()?;
    let (start, end) = xlsx::a1::parse_range(&range).ok()?;
    let mut by_ref = HashMap::new();
    if let Some(cells) = tree["sheets"][0]["cells"].as_array() {
        for c in cells {
            if let Some(r) = c["ref"].as_str() {
                by_ref.insert(r.to_string(), c["value"].clone());
            }
        }
    }
    let mut out = Vec::new();
    let mut texts = Vec::new();
    let mut all_num = true;
    for row in start.row..=end.row {
        for col in start.col..=end.col {
            let a1 = xlsx::a1::format_cell(col, row);
            match by_ref.get(&a1) {
                Some(Value::Number(n)) => {
                    out.push(Value::Number(n.clone()));
                    texts.push(n.to_string());
                }
                Some(Value::String(s)) => {
                    all_num = false;
                    out.push(Value::String(s.clone()));
                    texts.push(s.clone());
                }
                Some(Value::Bool(b)) => {
                    all_num = false;
                    out.push(json!(b));
                    texts.push(b.to_string());
                }
                Some(Value::Null) | None => {
                    all_num = false;
                    out.push(json!(""));
                    texts.push(String::new());
                }
                Some(other) => {
                    all_num = false;
                    out.push(other.clone());
                    texts.push(other.to_string());
                }
            }
        }
    }
    if all_num {
        Some(out)
    } else {
        Some(texts.into_iter().map(Value::String).collect())
    }
}

fn parse_chart_formula(f: &str) -> Option<(String, String)> {
    let mut s = f.trim().trim_start_matches('=').trim();
    if let Some(rest) = s.strip_prefix('[') {
        s = rest.split_once(']')?.1.trim_start_matches('!').trim();
    }
    let (sheet, range) = if let Some(rest) = s.strip_prefix('\'') {
        let end = rest.find('\'')?;
        let sheet = rest[..end].replace("''", "'");
        let after = rest[end + 1..].trim_start_matches('!').trim();
        (sheet, after)
    } else {
        let (a, b) = s.split_once('!')?;
        (a.to_string(), b.trim())
    };
    Some((sheet, range.replace('$', "")))
}

fn to_json_array(raw: Vec<String>) -> Vec<Value> {
    let nums: Option<Vec<Number>> = raw
        .iter()
        .map(|s| parse_num(s))
        .collect::<Option<Vec<_>>>();
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
    t.parse::<f64>()
        .ok()
        .and_then(Number::from_f64)
}

fn values_equal(a: &[Value], b: &[Value]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).all(|(x, y)| match (x, y) {
        (Value::Number(m), Value::Number(n)) => m.as_f64() == n.as_f64(),
        _ => x.to_string() == y.to_string(),
    })
}

fn inspect_type(space: &Element, _chartex: bool) -> String {
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

fn wrapper_kind(el: &Element) -> Option<&'static str> {
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
        "ofPieChart" | "doughnutChart" | "bubbleChart" | "radarChart" | "stockChart"
        | "surfaceChart" | "surface3DChart" => Some("other"),
        _ => None,
    }
}

fn chart_title(space: &Element) -> Option<String> {
    let chart = inner_chart(space)?;
    if bool_val(chart.child_by_local("autoTitleDeleted")) {
        return None;
    }
    title_text(chart.child_by_local("title"))
}

fn axis_titles(space: &Element, scatter: bool) -> (Option<String>, Option<String>) {
    let Some(plot) = plot_area(space) else {
        return (None, None);
    };
    if scatter {
        let vals: Vec<_> = plot
            .element_children()
            .filter(|c| c.local_name() == "valAx")
            .collect();
        let cat = vals.first().and_then(|a| title_text(a.child_by_local("title")));
        let val = vals.get(1).and_then(|a| title_text(a.child_by_local("title")));
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

fn title_text(title: Option<&Element>) -> Option<String> {
    let title = title?;
    let tx = title.child_by_local("tx")?;
    if let Some(v) = tx.child_by_local("v") {
        let s = v.text_content();
        if s.is_empty() {
            return None;
        }
        return Some(s);
    }
    if let Some(rich) = find_desc(tx, "rich") {
        let s = join_t(rich);
        if s.is_empty() {
            return None;
        }
        return Some(s);
    }
    if let Some(cache) = find_desc(tx, "strCache").or_else(|| find_desc(tx, "numCache")) {
        let mut s = String::new();
        for pt in descendants_local(cache, "pt") {
            if let Some(v) = pt.child_by_local("v") {
                s.push_str(&v.text_content());
            }
        }
        if s.is_empty() {
            return None;
        }
        return Some(s);
    }
    None
}

fn legend_json(space: &Element) -> Value {
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

fn data_labels_on(space: &Element) -> bool {
    let Some(plot) = plot_area(space) else {
        return false;
    };
    dlbls_on(plot)
}

fn dlbls_on(el: &Element) -> bool {
    if el.local_name() == "dLbls" {
        return ["showVal", "showCatName", "showSerName", "showPercent"]
            .iter()
            .any(|n| bool_val(el.child_by_local(n)));
    }
    el.element_children().any(dlbls_on)
}

fn flags_json(space: &Element, ty: &str) -> Vec<Value> {
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
    if ty == "pie" || ty == "other" || ty == "chartex" {
        return false;
    }
    let val_n = plot
        .element_children()
        .filter(|c| matches!(c.local_name(), "valAx" | "dateAx"))
        .count();
    if ty == "scatter" {
        val_n > 2
    } else {
        val_n > 1
    }
}

fn series_name(ser: &Element) -> Option<String> {
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

fn is_chart(drawing: &Element) -> bool {
    graphic_uri(drawing).is_some()
}

fn is_chartex(drawing: &Element) -> bool {
    matches!(graphic_uri(drawing), Some(u) if norm_uri(&u) == CHARTEX_NS)
}

fn graphic_uri(drawing: &Element) -> Option<String> {
    let mut found = None;
    walk_outside_txbx(drawing, &mut |e| {
        if e.local_name() == "graphicData" && found.is_none() {
            found = e.attr_local("uri").map(str::to_string);
        }
    });
    found.filter(|u| {
        let n = norm_uri(u);
        n == CHART_NS || n == CHARTEX_NS
    })
}

fn norm_uri(u: &str) -> &str {
    u.trim_end_matches('/')
}

fn chart_rid(drawing: &Element) -> Option<String> {
    let mut found = None;
    walk_outside_txbx(drawing, &mut |e| {
        if found.is_none()
            && e.local_name() == "chart"
            && e.attr_local("id").is_some()
        {
            found = e.attr_local("id").map(str::to_string);
        }
    });
    found
}

fn external_data_rid(space: &Element) -> Option<String> {
    space
        .child_by_local("externalData")
        .and_then(|e| e.attr_local("id"))
        .map(str::to_string)
}

fn load_embed(pkg: &Package, chart_part: &str, space: &Element) -> Option<Vec<u8>> {
    let rid = external_data_rid(space)?;
    let part = crate::opc::resolve_id(pkg, chart_part, &rid).ok()?;
    pkg.get(&part).ok().map(|b| b.to_vec())
}

fn inner_chart(space: &Element) -> Option<&Element> {
    space.child_by_local("chart")
}

fn inner_chart_mut(space: &mut Element) -> Option<&mut Element> {
    space.child_by_local_mut("chart")
}

fn plot_area(space: &Element) -> Option<&Element> {
    inner_chart(space)?.child_by_local("plotArea")
}

fn plot_area_mut(space: &mut Element) -> Option<&mut Element> {
    inner_chart_mut(space)?.child_by_local_mut("plotArea")
}

fn collect_sers(space: &Element) -> Vec<&Element> {
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

fn series_count(space: &Element) -> usize {
    collect_sers(space).len()
}

fn looks_like_para_or_container(id: &str) -> bool {
    id.starts_with("para:")
        || id.starts_with("header:")
        || id.starts_with("footer:")
        || id.starts_with("footnote:")
        || id.starts_with("endnote:")
        || id.starts_with("comment:")
}

fn parse_chart_key(rest: &str, path: &str) -> Result<ChartKey, Error> {
    if let Some(n) = rest.strip_prefix("index:") {
        let idx: usize = n.parse().map_err(|_| {
            Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::ChartIndexNotInt,
                },
                path.to_string(),
            )
        })?;
        if idx < 1 {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::ChartIndexFromOne,
                },
                path.to_string(),
            ));
        }
        return Ok(ChartKey::Index(idx));
    }
    if rest.is_empty() {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::ChartPrefix,
            },
            path.to_string(),
        ));
    }
    Ok(ChartKey::DocPr(rest.to_string()))
}

fn matches_key(drawing: &Element, key: &ChartKey, index: usize) -> bool {
    match key {
        ChartKey::Index(n) => index == *n,
        ChartKey::DocPr(want) => {
            let Some(got) = frame(drawing)
                .and_then(|f| f.child_by_local("docPr"))
                .and_then(|d| d.attr_local("id"))
            else {
                return false;
            };
            if let (Ok(a), Ok(b)) = (want.parse::<u32>(), got.parse::<u32>()) {
                a == b
            } else {
                want == got
            }
        }
    }
}

fn find_drawing<'a>(root: &'a Element, target: &ChartTarget) -> Option<&'a Element> {
    let p = para::find_paragraph(root, target.para())?;
    find_chart_walk(p, &mut 0, target.key())
}

fn find_chart_walk<'a>(el: &'a Element, idx: &mut usize, key: &ChartKey) -> Option<&'a Element> {
    if el.local_name() == "txbxContent" {
        return None;
    }
    if el.local_name() == "AlternateContent" {
        if let Some(i) = first_alt_child_index(el) {
            if let Node::Element(e) = &el.children[i] {
                return find_chart_walk(e, idx, key);
            }
        }
        return None;
    }
    if el.local_name() == "drawing" && is_chart(el) {
        *idx += 1;
        if matches_key(el, key, *idx) {
            return Some(el);
        }
        return None;
    }
    for n in &el.children {
        if let Node::Element(e) = n {
            if let Some(found) = find_chart_walk(e, idx, key) {
                return Some(found);
            }
        }
    }
    None
}

fn find_drawing_mut<'a>(root: &'a mut Element, target: &ChartTarget) -> Option<&'a mut Element> {
    let p = para::find_paragraph_mut(root, target.para())?;
    find_chart_mut_walk(p, &mut 0, target.key())
}

fn find_chart_mut_walk<'a>(
    el: &'a mut Element,
    idx: &mut usize,
    key: &ChartKey,
) -> Option<&'a mut Element> {
    if el.local_name() == "txbxContent" {
        return None;
    }
    if el.local_name() == "AlternateContent" {
        if let Some(i) = first_alt_child_index(el) {
            if let Node::Element(e) = &mut el.children[i] {
                return find_chart_mut_walk(e, idx, key);
            }
        }
        return None;
    }
    if el.local_name() == "drawing" && is_chart(el) {
        *idx += 1;
        if matches_key(el, key, *idx) {
            return Some(el);
        }
        return None;
    }
    let children = el.children.as_mut_slice();
    for child in children {
        if let Node::Element(e) = child {
            if let Some(found) = find_chart_mut_walk(e, idx, key) {
                return Some(found);
            }
        }
    }
    None
}

fn remove_chart(root: &mut Element, key: &ChartKey) -> bool {
    remove_chart_walk(root, &mut 0, key)
}

fn remove_chart_walk(el: &mut Element, idx: &mut usize, key: &ChartKey) -> bool {
    if el.local_name() == "txbxContent" {
        return false;
    }
    let mut i = 0;
    while i < el.children.len() {
        let local = match &el.children[i] {
            Node::Element(c) => c.local_name().to_string(),
            Node::Text(_) => {
                i += 1;
                continue;
            }
        };
        if local == "AlternateContent" {
            if ac_has_chart(&el.children[i], idx, key) {
                el.children.remove(i);
                return true;
            }
            i += 1;
            continue;
        }
        if local == "drawing" {
            let is_c = matches!(&el.children[i], Node::Element(c) if is_chart(c));
            if is_c {
                *idx += 1;
                let hit = match &el.children[i] {
                    Node::Element(c) => matches_key(c, key, *idx),
                    Node::Text(_) => false,
                };
                if hit {
                    el.children.remove(i);
                    return true;
                }
            }
            i += 1;
            continue;
        }
        if local == "txbxContent" {
            i += 1;
            continue;
        }
        if let Node::Element(child) = &mut el.children[i] {
            if remove_chart_walk(child, idx, key) {
                if child.local_name() == "r"
                    && child.element_children().all(|c| c.local_name() == "rPr")
                {
                    el.children.remove(i);
                }
                return true;
            }
        }
        i += 1;
    }
    false
}

fn ac_has_chart(node: &Node, idx: &mut usize, key: &ChartKey) -> bool {
    let Node::Element(ac) = node else {
        return false;
    };
    let Some(i) = first_alt_child_index(ac) else {
        return false;
    };
    let Node::Element(branch) = &ac.children[i] else {
        return false;
    };
    let mut hit = false;
    let _ = picture::walk_drawings(branch, &mut |d| {
        if is_chart(d) {
            *idx += 1;
            if matches_key(d, key, *idx) {
                hit = true;
            }
        }
        Ok(())
    });
    hit
}

fn first_alt_child_index(el: &Element) -> Option<usize> {
    let mut fallback = None;
    for (i, n) in el.children.iter().enumerate() {
        if let Node::Element(e) = n {
            if e.local_name() == "Choice" {
                return Some(i);
            }
            if e.local_name() == "Fallback" && fallback.is_none() {
                fallback = Some(i);
            }
        }
    }
    fallback
}

fn frame(drawing: &Element) -> Option<&Element> {
    drawing
        .element_children()
        .find(|e| e.local_name() == "inline" || e.local_name() == "anchor")
}

fn parse_emu(el: Option<&Element>, local: &str) -> Result<Option<i64>, Error> {
    let Some(el) = el else {
        return Ok(None);
    };
    let Some(raw) = el.attr_local(local) else {
        return Ok(None);
    };
    raw.parse::<i64>().map(Some).map_err(|_| {
        Error::xml_detail(
            crate::error::XmlIssue::AttrRead,
            format!("wp:extent @{local}={raw}"),
        )
    })
}

fn bool_val(el: Option<&Element>) -> bool {
    matches!(
        el.and_then(|e| e.attr_local("val")).unwrap_or("0"),
        "1" | "true" | "on" | "True"
    )
}

fn walk_outside_txbx(el: &Element, f: &mut impl FnMut(&Element)) {
    if el.local_name() == "txbxContent" {
        return;
    }
    f(el);
    for n in &el.children {
        if let Node::Element(e) = n {
            walk_outside_txbx(e, f);
        }
    }
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

fn ancestor_is(el: &Element, _local: &str) -> bool {
    // ser 的父就是包裹；用父名在 collect 时已知。这里按子元素判断。
    el.child_by_local("xVal").is_some() || el.child_by_local("yVal").is_some()
}

fn part_for_para(
    catalog: &[HfPart],
    doc_part: &str,
    para: &ParaTarget,
    id: &str,
    i: usize,
) -> Result<String, Error> {
    match para {
        ParaTarget::Body(_) => Ok(doc_part.to_string()),
        ParaTarget::HeaderFooter {
            kind, hf_type, n, ..
        } => {
            let container_id = format!("{}:{}:{n}", kind.as_str(), hf_type.as_str());
            let item = super::hf::find_by_id(catalog, &container_id).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::ParaNotFound { id: id.to_string() },
                    format!("ops[{i}].id"),
                )
            })?;
            Ok(item.part.clone())
        }
        _ => Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::InsertChartNeedsPara,
            },
            format!("ops[{i}].id"),
        )),
    }
}

fn part_for_target(
    _pkg: &Package,
    catalog: &[HfPart],
    doc_part: &str,
    target: &ChartTarget,
    id: &str,
    path: &str,
) -> Result<String, Error> {
    match target {
        ChartTarget::Body { .. } => Ok(doc_part.to_string()),
        ChartTarget::HeaderFooter {
            kind, hf_type, n, ..
        } => {
            let container_id = format!("{}:{}:{n}", kind.as_str(), hf_type.as_str());
            let item = super::hf::find_by_id(catalog, &container_id).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::ChartNotFound { id: id.to_string() },
                    path.to_string(),
                )
            })?;
            Ok(item.part.clone())
        }
    }
}

fn find_para_mut<'a>(root: &'a mut Element, target: &ParaTarget) -> Option<&'a mut Element> {
    match target {
        ParaTarget::Body(p) | ParaTarget::HeaderFooter { para: p, .. } => {
            para::find_paragraph_mut(root, p)
        }
        _ => None,
    }
}

fn load_part<'a>(
    pkg: &Package,
    docs: &'a mut HashMap<String, Document>,
    part: &str,
) -> Result<&'a mut Document, Error> {
    if !docs.contains_key(part) {
        docs.insert(part.to_string(), xml::parse(pkg.get(part)?)?);
    }
    Ok(docs.get_mut(part).unwrap())
}

fn load_chart_space(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    doc_part: &str,
    catalog: &[HfPart],
    target: &ChartTarget,
    id: &str,
    i: usize,
) -> Result<(Document, String), Error> {
    let (space, part, _) = load_chart_space_host(pkg, docs, doc_part, catalog, target, id, i)?;
    Ok((space, part))
}

fn load_chart_space_host(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    doc_part: &str,
    catalog: &[HfPart],
    target: &ChartTarget,
    id: &str,
    i: usize,
) -> Result<(Document, String, String), Error> {
    let path = format!("ops[{i}].id");
    let host = part_for_target(pkg, catalog, doc_part, target, id, &path)?;
    let doc = load_part(pkg, docs, &host)?;
    let drawing = find_drawing(&doc.root, target).ok_or_else(|| {
        Error::with_path(ErrorKind::ChartNotFound { id: id.to_string() }, path.clone())
    })?;
    let rid = chart_rid(drawing).ok_or_else(|| {
        Error::with_path(ErrorKind::ChartNotFound { id: id.to_string() }, path.clone())
    })?;
    let chart_part = crate::opc::resolve_id(pkg, &host, &rid)?;
    let space = xml::parse(pkg.get(&chart_part)?)?;
    Ok((space, chart_part, host))
}

fn reject_restricted(space: &Element, id: &str, i: usize) -> Result<(), Error> {
    let ty = inspect_type(space, false);
    let flags = flags_json(space, &ty);
    let blocked = matches!(ty.as_str(), "combo" | "other" | "chartex")
        || flags.iter().any(|f| {
            matches!(
                f.as_str(),
                Some("secondaryAxis" | "trendline" | "errorBars" | "pivot")
            )
        });
    if blocked {
        return Err(Error::with_path(
            ErrorKind::RestrictedChart { id: id.to_string() },
            format!("ops[{i}].id"),
        ));
    }
    Ok(())
}

fn ensure_chart_ns(root: &mut Element) {
    picture::ensure_drawing_ns(root);
    ensure_xmlns(root, "c", CHART_NS);
}

fn ensure_xmlns(el: &mut Element, prefix: &str, uri: &str) {
    let key = format!("xmlns:{prefix}");
    if el.attr(&key).is_none() {
        el.set_attr(&key, uri);
    }
}

fn build_chart_drawing(doc_pr: u32, rid: &str, cx: i64, cy: i64) -> Element {
    let mut drawing = Element::new("w:drawing");
    let mut inline = Element::new("wp:inline")
        .with_attr("distT", "0")
        .with_attr("distB", "0")
        .with_attr("distL", "0")
        .with_attr("distR", "0");
    inline.push_element(
        Element::new("wp:extent")
            .with_attr("cx", cx.to_string())
            .with_attr("cy", cy.to_string()),
    );
    inline.push_element(
        Element::new("wp:effectExtent")
            .with_attr("l", "0")
            .with_attr("t", "0")
            .with_attr("r", "0")
            .with_attr("b", "0"),
    );
    inline.push_element(
        Element::new("wp:docPr")
            .with_attr("id", doc_pr.to_string())
            .with_attr("name", format!("图表 {doc_pr}")),
    );
    let mut frame_pr = Element::new("wp:cNvGraphicFramePr");
    frame_pr.push_element(Element::new("a:graphicFrameLocks").with_attr("noGrp", "1"));
    inline.push_element(frame_pr);
    let mut graphic = Element::new("a:graphic");
    let mut data = Element::new("a:graphicData").with_attr("uri", CHART_NS);
    data.push_element(
        Element::new("c:chart")
            .with_attr("xmlns:c", CHART_NS)
            .with_attr("xmlns:r", REL_NS)
            .with_attr("r:id", rid),
    );
    graphic.push_element(data);
    inline.push_element(graphic);
    drawing.push_element(inline);
    drawing
}

fn shared_categories(series: &[NewSeries]) -> Vec<Value> {
    if let Some(s) = series.iter().find_map(|s| s.categories.as_ref()) {
        return s.clone();
    }
    let n = series.first().map(|s| s.values.len()).unwrap_or(0);
    (1..=n).map(|i| json!(i.to_string())).collect()
}

fn build_embedding(series: &[NewSeries], cats: &[Value]) -> Result<Vec<u8>, Error> {
    let mut cells = Vec::new();
    for (i, cat) in cats.iter().enumerate() {
        cells.push(json!({ "ref": format!("A{}", i + 2), "value": cat }));
    }
    for (si, s) in series.iter().enumerate() {
        let col = xlsx::a1::col_to_letters((si as u32) + 2);
        let name = s
            .name
            .clone()
            .unwrap_or_else(|| format!("系列{}", si + 1));
        cells.push(json!({ "ref": format!("{col}1"), "value": name }));
        for (ri, v) in s.values.iter().enumerate() {
            cells.push(json!({ "ref": format!("{}{}", col, ri + 2), "value": v }));
        }
    }
    xlsx::create(&json!({ "sheets": [{ "name": "Sheet1", "cells": cells }] }))
}

fn build_chart_xml(
    ty: ChartType,
    title: Option<&str>,
    series: &[NewSeries],
    cats: &[Value],
) -> String {
    let scatter = ty == ChartType::Scatter;
    let pie = ty == ChartType::Pie;
    let mut sers = String::new();
    for (i, ser) in series.iter().enumerate() {
        let col = xlsx::a1::col_to_letters((i as u32) + 2);
        let name = ser
            .name
            .clone()
            .unwrap_or_else(|| format!("系列{}", i + 1));
        let cat_ref = format!("Sheet1!$A$2:$A${}", cats.len() + 1);
        let val_ref = format!("Sheet1!${col}$2:${col}${}", ser.values.len() + 1);
        let name_ref = format!("Sheet1!${col}$1");
        sers.push_str(&ser_xml(
            i as u32,
            &name,
            &name_ref,
            &cat_ref,
            cats,
            &val_ref,
            &ser.values,
            scatter,
        ));
    }
    let wrapper = ty.wrapper();
    let extra = match ty {
        ChartType::Bar => r#"<c:barDir val="bar"/><c:grouping val="clustered"/><c:varyColors val="0"/>"#,
        ChartType::Col => r#"<c:barDir val="col"/><c:grouping val="clustered"/><c:varyColors val="0"/>"#,
        ChartType::Line => r#"<c:grouping val="standard"/><c:varyColors val="0"/>"#,
        ChartType::Area => r#"<c:grouping val="standard"/><c:varyColors val="0"/>"#,
        ChartType::Pie => r#"<c:varyColors val="1"/>"#,
        ChartType::Scatter => r#"<c:scatterStyle val="lineMarker"/><c:varyColors val="0"/>"#,
    };
    let axes = if pie {
        String::new()
    } else if scatter {
        ax_pair_scatter()
    } else {
        ax_pair()
    };
    let ax_ids = if pie {
        String::new()
    } else {
        r#"<c:axId val="1"/><c:axId val="2"/>"#.into()
    };
    let title_xml = match title {
        Some(t) if !t.is_empty() => format!(
            r#"<c:autoTitleDeleted val="0"/>{}"#,
            title_el(t)
        ),
        _ => r#"<c:autoTitleDeleted val="1"/>"#.into(),
    };
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><c:chartSpace xmlns:c="{CHART_NS}" xmlns:a="{A_NS}" xmlns:r="{REL_NS}"><c:chart>{title_xml}<c:plotArea><c:layout/><c:{wrapper}>{extra}{sers}<c:dLbls><c:showLegendKey val="0"/><c:showVal val="0"/><c:showCatName val="0"/><c:showSerName val="0"/><c:showPercent val="0"/></c:dLbls>{ax_ids}</c:{wrapper}>{axes}</c:plotArea><c:legend><c:legendPos val="r"/><c:overlay val="0"/></c:legend><c:plotVisOnly val="1"/></c:chart><c:externalData r:id="rId1"><c:autoUpdate val="0"/></c:externalData></c:chartSpace>"#
    )
}

fn ser_xml(
    idx: u32,
    name: &str,
    name_ref: &str,
    cat_ref: &str,
    cats: &[Value],
    val_ref: &str,
    vals: &[Value],
    scatter: bool,
) -> String {
    let (cat_tag, val_tag) = if scatter {
        ("c:xVal", "c:yVal")
    } else {
        ("c:cat", "c:val")
    };
    format!(
        r#"<c:ser><c:idx val="{idx}"/><c:order val="{idx}"/><c:tx>{}</c:tx><{cat_tag}>{}</{cat_tag}><{val_tag}>{}</{val_tag}></c:ser>"#,
        str_ref(name_ref, &[json!(name)]),
        str_or_num_ref(cat_ref, cats),
        str_or_num_ref(val_ref, vals),
    )
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
    if pts.iter().all(|v| v.is_number()) {
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

fn next_numbered(pkg: &Package, dir: &str, stem: &str, ext: &str) -> String {
    let mut n = 1u32;
    loop {
        let name = format!("{dir}/{stem}{n}{ext}");
        if !pkg.contains(&name) {
            return name;
        }
        n += 1;
    }
}

fn chart_satellites(pkg: &Package, chart_part: &str) -> Result<Vec<String>, Error> {
    let mut out = Vec::new();
    let rels = Relationships::from_package(pkg, chart_part)?;
    for r in &rels.rels {
        if r.is_external() {
            continue;
        }
        if let Ok(p) = resolve_rel_target(chart_part, &r.target) {
            out.push(p);
        }
    }
    Ok(out)
}

fn drop_unused(pkg: &mut Package, part: &str) -> Result<(), Error> {
    if part_still_used(pkg, part)? {
        return Ok(());
    }
    let rels = rels_part_name(part);
    pkg.remove(part);
    pkg.remove(&rels);
    let mut cts = ContentTypes::from_package(pkg)?;
    cts.remove_override(part);
    cts.write_to(pkg)?;
    Ok(())
}

fn part_still_used(pkg: &Package, target: &str) -> Result<bool, Error> {
    for name in pkg.part_names() {
        if !name.ends_with(".rels") {
            continue;
        }
        let Some(source) = source_from_rels(&name) else {
            continue;
        };
        let rels = Relationships::parse(pkg.get(&name)?)?;
        for r in &rels.rels {
            if r.is_external() {
                continue;
            }
            if resolve_rel_target(&source, &r.target).ok().as_deref() == Some(target) {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn source_from_rels(rels: &str) -> Option<String> {
    if rels == "/_rels/.rels" {
        return Some("/".into());
    }
    let (dir, file) = split_dir_file(rels);
    let parent = dir.strip_suffix("/_rels")?;
    let stem = file.strip_suffix(".rels")?;
    if parent.is_empty() || parent == "/" {
        Some(format!("/{stem}"))
    } else {
        Some(format!("{parent}/{stem}"))
    }
}

fn remove_rel_if_unused(
    pkg: &mut Package,
    part: &str,
    rid: &str,
    root: &Element,
) -> Result<(), Error> {
    let mut found = false;
    walk_outside_txbx(root, &mut |e| {
        if e.local_name() == "chart" && e.attr_local("id") == Some(rid) {
            found = true;
        }
    });
    if found {
        return Ok(());
    }
    let mut rels = Relationships::from_package(pkg, part)?;
    rels.remove_by_id(rid);
    if rels.rels.is_empty() {
        pkg.remove(&rels_part_name(part));
    } else {
        rels.write_to(pkg, part)?;
    }
    Ok(())
}

fn set_chart_title(space: &mut Element, title: &str) {
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
        let mut nodes = Vec::new();
        for n in doc.root.children {
            nodes.push(n);
        }
        for n in nodes.into_iter().rev() {
            chart.children.insert(0, n);
        }
    }
}

fn set_axis_title(space: &mut Element, axis: &str, title: &str, i: usize) -> Result<(), Error> {
    let scatter = inspect_type(space, false) == "scatter";
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
                ax.children.insert(1.min(ax.children.len()), Node::Element(t));
            }
        }
    }
    Ok(())
}

fn set_legend(space: &mut Element, show: bool, pos: Option<&str>) {
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

fn set_data_labels(space: &mut Element, show: bool) {
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

fn strip_dlbls(el: &mut Element) {
    el.children.retain(|n| match n {
        Node::Element(e) => e.local_name() != "dLbls",
        _ => true,
    });
    for c in el.element_children_mut() {
        strip_dlbls(c);
    }
}

fn write_series(
    pkg: &mut Package,
    space: &mut Element,
    chart_part: &str,
    index: usize,
    name: Option<&str>,
    cats: Option<&[Value]>,
    vals: Option<&[Value]>,
    i: usize,
) -> Result<(), Error> {
    if index >= series_count(space) {
        return Err(Error::with_path(
            ErrorKind::ChartSeriesNotFound {
                id: format!("series:{index}"),
            },
            format!("ops[{i}].id"),
        ));
    }
    let embed_snapshot = with_ser_mut(space, index, |ser| {
        if let Some(n) = name {
            set_series_name(ser, n);
        }
        if let Some(c) = cats {
            let local = if ser.child_by_local("xVal").is_some() {
                "xVal"
            } else {
                "cat"
            };
            set_pts(ser, local, c);
        }
        if let Some(v) = vals {
            let local = if ser.child_by_local("yVal").is_some() {
                "yVal"
            } else {
                "val"
            };
            set_pts(ser, local, v);
        }
        ser.clone()
    });
    let Some(ser) = embed_snapshot else {
        return Err(Error::with_path(
            ErrorKind::ChartSeriesNotFound {
                id: format!("series:{index}"),
            },
            format!("ops[{i}].id"),
        ));
    };
    if let Some(embed_rid) = external_data_rid(space) {
        let embed_part = crate::opc::resolve_id(pkg, chart_part, &embed_rid)?;
        let bytes = pkg.get(&embed_part)?.to_vec();
        let updated = patch_embed(&bytes, &ser, name, cats, vals)?;
        pkg.set(&embed_part, updated)?;
    }
    Ok(())
}

fn with_ser_mut<R>(
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

fn set_series_name(ser: &mut Element, name: &str) {
    let mut tx = ser.child_by_local_mut("tx");
    if tx.is_none() {
        ser.children.insert(
            2.min(ser.children.len()),
            Node::Element(Element::new("c:tx")),
        );
        tx = ser.child_by_local_mut("tx");
    }
    let Some(tx) = tx else {
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

fn set_pts(ser: &mut Element, local: &str, values: &[Value]) {
    let tag = format!("c:{local}");
    let existing_f = ser
        .child_by_local(local)
        .and_then(formula_of)
        .unwrap_or_else(|| default_formula(local, values.len()));
    let f = resize_formula(&existing_f, values.len());
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

fn default_formula(local: &str, n: usize) -> String {
    let end = n.max(1) + 1;
    match local {
        "cat" | "xVal" => format!("Sheet1!$A$2:$A${end}"),
        _ => format!("Sheet1!$B$2:$B${end}"),
    }
}

fn resize_formula(f: &str, n: usize) -> String {
    let Some((sheet, range)) = parse_chart_formula(f) else {
        return f.to_string();
    };
    let Ok((start, mut end)) = xlsx::a1::parse_range(&range) else {
        return f.to_string();
    };
    let len = n.max(1) as u32;
    if start.col == end.col {
        end.row = start.row + len - 1;
    } else if start.row == end.row {
        end.col = start.col + len - 1;
    }
    let a = xlsx::a1::format_cell(start.col, start.row);
    let b = xlsx::a1::format_cell(end.col, end.row);
    let (ac, ar) = split_a1(&a);
    let (bc, br) = split_a1(&b);
    format!("{sheet}!${ac}${ar}:${bc}${br}")
}

fn split_a1(a1: &str) -> (String, String) {
    let i = a1.find(|c: char| c.is_ascii_digit()).unwrap_or(a1.len());
    (a1[..i].to_string(), a1[i..].to_string())
}

fn patch_embed(
    bytes: &[u8],
    ser: &Element,
    name: Option<&str>,
    cats: Option<&[Value]>,
    vals: Option<&[Value]>,
) -> Result<Vec<u8>, Error> {
    let mut ops = Vec::new();
    if let Some(n) = name {
        if let Some(f) = ser.child_by_local("tx").and_then(formula_of) {
            if let Some((sheet, range)) = parse_chart_formula(&f) {
                let a1 = range.split(':').next().unwrap_or(&range);
                ops.push(json!({
                    "op": "setCell",
                    "id": format!("cell:{sheet}!{a1}"),
                    "value": n
                }));
            }
        }
    }
    if let Some(c) = cats {
        push_range_ops(&mut ops, ser, "cat", "xVal", c);
    }
    if let Some(v) = vals {
        push_range_ops(&mut ops, ser, "val", "yVal", v);
    }
    if ops.is_empty() {
        return Ok(bytes.to_vec());
    }
    let parsed = xlsx::parse_ops(&ops)?;
    xlsx::apply(bytes, &parsed)
}

fn push_range_ops(ops: &mut Vec<Value>, ser: &Element, a: &str, b: &str, values: &[Value]) {
    let el = ser.child_by_local(a).or_else(|| ser.child_by_local(b));
    let Some(f) = el.and_then(formula_of) else {
        return;
    };
    let Some((sheet, range)) = parse_chart_formula(&f) else {
        return;
    };
    let Ok((start, _)) = xlsx::a1::parse_range(&range) else {
        return;
    };
    for (i, v) in values.iter().enumerate() {
        let col = start.col;
        let row = start.row + i as u32;
        let a1 = xlsx::a1::format_cell(col, row);
        ops.push(json!({
            "op": "setCell",
            "id": format!("cell:{sheet}!{a1}"),
            "value": v
        }));
    }
}

fn add_series(
    pkg: &mut Package,
    space: &mut Element,
    chart_part: &str,
    name: Option<&str>,
    cats: Option<&[Value]>,
    vals: &[Value],
    i: usize,
) -> Result<(), Error> {
    let n = series_count(space);
    let cats_owned: Vec<Value> = if let Some(c) = cats {
        c.to_vec()
    } else if n > 0 {
        let first = collect_sers(space)[0];
        let local = if first.child_by_local("xVal").is_some() {
            "xVal"
        } else {
            "cat"
        };
        cache_values(first.child_by_local(local).unwrap_or(first)).unwrap_or_default()
    } else {
        return Err(Error::with_path(
            ErrorKind::NeedChartCategories,
            format!("ops[{i}].categories"),
        ));
    };
    if cats_owned.is_empty() {
        return Err(Error::with_path(
            ErrorKind::NeedChartCategories,
            format!("ops[{i}].categories"),
        ));
    }
    let idx = unused_idx(space);
    let name = name
        .map(str::to_string)
        .unwrap_or_else(|| format!("系列{}", n + 1));
    let col = xlsx::a1::col_to_letters((n as u32) + 2);
    let cat_ref = formula_of(
        collect_sers(space)
            .first()
            .and_then(|s| s.child_by_local("cat").or_else(|| s.child_by_local("xVal")))
            .unwrap_or_else(|| collect_sers(space)[0]),
    )
    .unwrap_or_else(|| format!("Sheet1!$A$2:$A${}", cats_owned.len() + 1));
    let val_ref = format!("Sheet1!${col}$2:${col}${}", vals.len() + 1);
    let name_ref = format!("Sheet1!${col}$1");
    let scatter = inspect_type(space, false) == "scatter";
    let xml = ser_xml(
        idx,
        &name,
        &name_ref,
        &cat_ref,
        &cats_owned,
        &val_ref,
        vals,
        scatter,
    );
    let wrapped = format!(r#"<c:root xmlns:c="{CHART_NS}">{xml}</c:root>"#);
    let doc = xml::parse(wrapped.as_bytes())?;
    let ser_el = doc
        .root
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
        })?;
    if let Some(plot) = plot_area_mut(space) {
        if let Some(w) = plot
            .element_children_mut()
            .find(|c| WRAPPERS.contains(&c.local_name()))
        {
            let at = w
                .children
                .iter()
                .position(|n| {
                    matches!(n, Node::Element(e) if e.local_name() == "dLbls" || e.local_name() == "axId")
                })
                .unwrap_or(w.children.len());
            w.children.insert(at, Node::Element(ser_el));
        }
    }
    if let Some(embed_rid) = external_data_rid(space) {
        let embed_part = crate::opc::resolve_id(pkg, chart_part, &embed_rid)?;
        let bytes = pkg.get(&embed_part)?.to_vec();
        let mut ops = vec![json!({
            "op": "setCell",
            "id": format!("cell:Sheet1!{col}1"),
            "value": name
        })];
        for (ri, v) in vals.iter().enumerate() {
            ops.push(json!({
                "op": "setCell",
                "id": format!("cell:Sheet1!{col}{}", ri + 2),
                "value": v
            }));
        }
        let parsed = xlsx::parse_ops(&ops)?;
        let updated = xlsx::apply(&bytes, &parsed)?;
        pkg.set(&embed_part, updated)?;
    }
    Ok(())
}

fn unused_idx(space: &Element) -> u32 {
    let mut used: HashSet<u32> = HashSet::new();
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

fn remove_series_at(space: &mut Element, index: usize) {
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

fn convert_type(space: &mut Element, target: ChartType, i: usize) -> Result<(), Error> {
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
    if target == ChartType::Pie && series_count(space) != 1 {
        return Err(Error::with_path(
            ErrorKind::ChartTypeNotConvertible,
            format!("ops[{i}].type"),
        ));
    }
    let from_scatter = current == "scatter";
    let to_scatter = target == ChartType::Scatter;
    if (from_scatter || to_scatter) && !scatter_convertible(space, from_scatter, to_scatter) {
        return Err(Error::with_path(
            ErrorKind::ChartTypeNotConvertible,
            format!("ops[{i}].type"),
        ));
    }
    let Some(plot) = plot_area_mut(space) else {
        return Err(Error::with_path(
            ErrorKind::ChartTypeNotConvertible,
            format!("ops[{i}].type"),
        ));
    };
    let wrap_i = plot.children.iter().position(|n| {
        matches!(n, Node::Element(e) if WRAPPERS.contains(&e.local_name()))
    });
    let Some(wrap_i) = wrap_i else {
        return Err(Error::with_path(
            ErrorKind::ChartTypeNotConvertible,
            format!("ops[{i}].type"),
        ));
    };
    let Node::Element(wrap) = &mut plot.children[wrap_i] else {
        return Err(Error::with_path(
            ErrorKind::ChartTypeNotConvertible,
            format!("ops[{i}].type"),
        ));
    };
    wrap.name = format!("c:{}", target.wrapper());
    wrap.children.retain(|n| match n {
        Node::Element(e) => {
            !matches!(e.local_name(), "barDir" | "grouping" | "scatterStyle" | "varyColors")
        }
        _ => true,
    });
    let mut prefix = Vec::new();
    match target {
        ChartType::Bar => {
            prefix.push(Element::new("c:barDir").with_attr("val", "bar"));
            prefix.push(Element::new("c:grouping").with_attr("val", "clustered"));
            prefix.push(Element::new("c:varyColors").with_attr("val", "0"));
        }
        ChartType::Col => {
            prefix.push(Element::new("c:barDir").with_attr("val", "col"));
            prefix.push(Element::new("c:grouping").with_attr("val", "clustered"));
            prefix.push(Element::new("c:varyColors").with_attr("val", "0"));
        }
        ChartType::Line | ChartType::Area => {
            prefix.push(Element::new("c:grouping").with_attr("val", "standard"));
            prefix.push(Element::new("c:varyColors").with_attr("val", "0"));
        }
        ChartType::Pie => {
            prefix.push(Element::new("c:varyColors").with_attr("val", "1"));
        }
        ChartType::Scatter => {
            prefix.push(Element::new("c:scatterStyle").with_attr("val", "lineMarker"));
            prefix.push(Element::new("c:varyColors").with_attr("val", "0"));
        }
    }
    for el in prefix.into_iter().rev() {
        wrap.children.insert(0, Node::Element(el));
    }
    for c in wrap.element_children_mut() {
        if c.local_name() == "ser" {
            rename_ser_xy(c, to_scatter, from_scatter);
        }
    }
    if target == ChartType::Pie {
        wrap.children.retain(|n| match n {
            Node::Element(e) => e.local_name() != "axId",
            _ => true,
        });
    } else if wrap.element_children().all(|c| c.local_name() != "axId") {
        wrap.push_element(Element::new("c:axId").with_attr("val", "1"));
        wrap.push_element(Element::new("c:axId").with_attr("val", "2"));
    }
    // 轴
    let plot = plot_area_mut(space).unwrap();
    if target == ChartType::Pie {
        plot.children.retain(|n| match n {
            Node::Element(e) => {
                !matches!(e.local_name(), "catAx" | "valAx" | "dateAx" | "serAx")
            }
            _ => true,
        });
    } else {
        let has_cat = plot
            .element_children()
            .any(|c| c.local_name() == "catAx");
        let val_n = plot
            .element_children()
            .filter(|c| c.local_name() == "valAx")
            .count();
        if to_scatter {
            for c in plot.element_children_mut() {
                if c.local_name() == "catAx" {
                    c.name = "c:valAx".into();
                }
            }
        } else if from_scatter {
            let mut first = true;
            for c in plot.element_children_mut() {
                if c.local_name() == "valAx" && first {
                    c.name = "c:catAx".into();
                    first = false;
                }
            }
        } else if !has_cat && val_n == 0 {
            let xml = format!(
                r#"<c:plotArea xmlns:c="{CHART_NS}">{}</c:plotArea>"#,
                if target == ChartType::Scatter {
                    ax_pair_scatter()
                } else {
                    ax_pair()
                }
            );
            if let Ok(doc) = xml::parse(xml.as_bytes()) {
                for n in doc.root.children {
                    plot.children.push(n);
                }
            }
        }
    }
    Ok(())
}

fn scatter_convertible(space: &Element, from_scatter: bool, to_scatter: bool) -> bool {
    for ser in collect_sers(space) {
        if to_scatter && !from_scatter {
            if ser.child_by_local("cat").is_none() || ser.child_by_local("val").is_none() {
                return false;
            }
        }
        if from_scatter && !to_scatter {
            if ser.child_by_local("xVal").is_none() || ser.child_by_local("yVal").is_none() {
                return false;
            }
        }
    }
    true
}

fn rename_ser_xy(ser: &mut Element, to_scatter: bool, from_scatter: bool) {
    for c in ser.element_children_mut() {
        if to_scatter && !from_scatter {
            if c.local_name() == "cat" {
                c.name = "c:xVal".into();
            }
            if c.local_name() == "val" {
                c.name = "c:yVal".into();
            }
        }
        if from_scatter && !to_scatter {
            if c.local_name() == "xVal" {
                c.name = "c:cat".into();
            }
            if c.local_name() == "yVal" {
                c.name = "c:val".into();
            }
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
