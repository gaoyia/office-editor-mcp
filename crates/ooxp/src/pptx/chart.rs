//! 幻灯片 `p:graphicFrame` 图表：名单、插删、改系列与画布坐标。embedding 走 `chart::embed`。

use serde_json::{json, Map, Value};

use crate::chart::{self, ChartRestrict, ChartType, CHARTEX_NS, CHART_NS};
use crate::error::{Error, ErrorKind, IdReason};
use crate::opc::{
    rels_part_name, resolve_rel_target, ContentTypes, Package, Relationship, Relationships,
};
use crate::xml::{self, Document, Element, Node};

use super::picture;
use super::request::{
    AddChartSeriesOp, DeleteChartOp, DeleteChartSeriesOp, DeleteErrorBarsOp, DeleteTrendlineOp,
    InsertChartOp, InsertErrorBarsOp, InsertTrendlineOp, SetChartAxisTitleOp, SetChartDataLabelsOp,
    SetChartLegendOp, SetChartPosOp, SetChartSeriesAxisOp, SetChartSeriesOp, SetChartSizeOp,
    SetChartTitleOp, SetChartTypeOp, SetErrorBarsOp, SetTrendlineOp,
};
use super::shape;
use super::{ExportedMedia, DML_NS, REL_NS};

#[derive(Debug, Clone)]
pub enum ChartKey {
    Id(String),
    Index(usize),
}

#[derive(Debug, Clone)]
pub struct ChartTarget {
    pub slide: usize,
    pub on_notes: bool,
    pub master: Option<(usize, Option<usize>)>,
    pub key: ChartKey,
}

pub fn parse_chart_id(id: &str, json_path: &str) -> Result<ChartTarget, Error> {
    let path = json_path.to_string();
    let (host, after) =
        picture::parse_object_id(id, json_path, "chart", IdReason::SlideChartPrefix)?;
    if after.is_empty() {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SlideChartContainer,
            },
            path,
        ));
    }
    let (slide, on_notes, master) = host.parts();
    if let Some(rest) = after.strip_prefix("index:") {
        let k: usize = rest.parse().map_err(|_| {
            Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::ChartIndexNotInt,
                },
                path.clone(),
            )
        })?;
        if k < 1 {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::ChartIndexFromOne,
                },
                path,
            ));
        }
        return Ok(ChartTarget {
            slide,
            on_notes,
            master,
            key: ChartKey::Index(k),
        });
    }
    if after.contains('/') {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SlideChartPrefix,
            },
            path,
        ));
    }
    Ok(ChartTarget {
        slide,
        on_notes,
        master,
        key: ChartKey::Id(after.to_string()),
    })
}

pub fn parse_series_id(id: &str, json_path: &str) -> Result<(ChartTarget, usize), Error> {
    let path = json_path.to_string();
    let (chart, series) = id.rsplit_once("/series:").ok_or_else(|| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SeriesPrefix,
            },
            path.clone(),
        )
    })?;
    let target = parse_chart_id(chart, json_path)?;
    let index: usize = series.parse().map_err(|_| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SeriesIndexNotInt,
            },
            path,
        )
    })?;
    Ok((target, index))
}

pub fn parse_trendline_id(id: &str, json_path: &str) -> Result<(ChartTarget, usize, usize), Error> {
    let (left, k) = crate::chart::remain::parse_member_id(
        id,
        "trendline",
        crate::chart::remain::IdFromOne::Trendline,
    )
    .map_err(|reason| Error::with_path(ErrorKind::InvalidId { reason }, json_path.to_string()))?;
    let (target, index) = parse_series_id(left, json_path)?;
    Ok((target, index, k))
}

pub fn parse_error_bars_id(
    id: &str,
    json_path: &str,
) -> Result<(ChartTarget, usize, usize), Error> {
    let (left, k) = crate::chart::remain::parse_member_id(
        id,
        "errorBars",
        crate::chart::remain::IdFromOne::ErrorBars,
    )
    .map_err(|reason| Error::with_path(ErrorKind::InvalidId { reason }, json_path.to_string()))?;
    let (target, index) = parse_series_id(left, json_path)?;
    Ok((target, index, k))
}

pub fn parse_insert_slide_id(id: &str, op_index: usize) -> Result<picture::TreeHost, Error> {
    picture::parse_insert_host(id, op_index, IdReason::InsertChartNeedsSlide)
}

pub fn count_charts(root: &Element) -> usize {
    let mut n = 0usize;
    count_in(root, &mut n);
    n
}

fn count_in(el: &Element, n: &mut usize) {
    if el.local_name() == "AlternateContent" {
        if let Some(i) = shape::first_alt_child_index(el) {
            if let Node::Element(child) = &el.children[i] {
                count_in(child, n);
            }
        }
        return;
    }
    if is_counted_chart(el) {
        *n += 1;
        return;
    }
    for child in &el.children {
        if let Node::Element(e) = child {
            count_in(e, n);
        }
    }
}

pub fn collect_charts(
    pkg: &Package,
    root: &Element,
    host: &str,
    slide_part: &str,
) -> Result<Vec<Value>, Error> {
    let mut out = Vec::new();
    let mut index = 0usize;
    collect_in(pkg, root, host, slide_part, &mut index, &mut out)?;
    Ok(out)
}

fn collect_in(
    pkg: &Package,
    el: &Element,
    host: &str,
    slide_part: &str,
    index: &mut usize,
    out: &mut Vec<Value>,
) -> Result<(), Error> {
    if el.local_name() == "AlternateContent" {
        if let Some(i) = shape::first_alt_child_index(el) {
            if let Node::Element(child) = &el.children[i] {
                return collect_in(pkg, child, host, slide_part, index, out);
            }
        }
        return Ok(());
    }
    if is_counted_chart(el) {
        *index += 1;
        out.push(chart_json(pkg, el, host, slide_part, *index)?);
        return Ok(());
    }
    for n in &el.children {
        if let Node::Element(e) = n {
            collect_in(pkg, e, host, slide_part, index, out)?;
        }
    }
    Ok(())
}

fn is_counted_chart(el: &Element) -> bool {
    el.local_name() == "graphicFrame" && graphic_uri(el).is_some()
}

fn graphic_uri(el: &Element) -> Option<String> {
    let mut found = None;
    shape::walk_choice(el, &mut |e| {
        if e.local_name() == "graphicData" && found.is_none() {
            found = e.attr_local("uri").map(str::to_string);
        }
    });
    found.filter(|u| {
        let n = u.trim_end_matches('/');
        n == CHART_NS || n == CHARTEX_NS
    })
}

fn is_chartex(el: &Element) -> bool {
    matches!(graphic_uri(el), Some(u) if u.trim_end_matches('/') == CHARTEX_NS)
}

fn chart_rid(frame: &Element) -> Option<String> {
    let mut found = None;
    shape::walk_choice(frame, &mut |e| {
        if found.is_none() && e.local_name() == "chart" && e.attr_local("id").is_some() {
            found = e.attr_local("id").map(str::to_string);
        }
    });
    found
}

fn cnv_of_frame(frame: &Element) -> (Option<String>, String) {
    let mut raw = None;
    let mut name = String::new();
    if let Some(nv) = frame.child_by_local("nvGraphicFramePr") {
        if let Some(pr) = nv.child_by_local("cNvPr") {
            raw = pr.attr_local("id").map(str::to_string);
            name = pr.attr_local("name").unwrap_or("").to_string();
        }
    }
    (raw, name)
}

fn parse_i64_attr(el: &Element, name: &str) -> Option<i64> {
    el.attr_local(name).and_then(|s| s.parse().ok())
}

fn frame_xfrm(frame: &Element) -> Option<&Element> {
    frame.child_by_local("xfrm")
}

fn chart_json(
    pkg: &Package,
    frame: &Element,
    host: &str,
    slide_part: &str,
    index: usize,
) -> Result<Value, Error> {
    let (raw_id, name) = cnv_of_frame(frame);
    let stable = raw_id.is_some();
    let id = match &raw_id {
        Some(v) => format!("{host}/chart:{v}"),
        None => format!("{host}/chart:index:{index}"),
    };
    let xfrm = frame_xfrm(frame);
    let (x, y) = match xfrm.and_then(|x| x.child_by_local("off")) {
        Some(off) => (parse_i64_attr(off, "x"), parse_i64_attr(off, "y")),
        None => (None, None),
    };
    let (cx, cy) = match xfrm.and_then(|x| x.child_by_local("ext")) {
        Some(ext) => (parse_i64_attr(ext, "cx"), parse_i64_attr(ext, "cy")),
        None => (None, None),
    };
    let chartex = is_chartex(frame);
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
    let mut space_for_keys: Option<Element> = None;
    if let Some(rid) = chart_rid(frame) {
        if let Ok(chart_part) = crate::opc::resolve_id(pkg, slide_part, &rid) {
            if let Ok(bytes) = pkg.get(&chart_part) {
                if let Ok(doc) = xml::parse(bytes) {
                    let embed = load_embed(pkg, &chart_part, &doc.root);
                    if !chartex {
                        ty = chart::inspect_type(&doc.root, false);
                    }
                    title = json!(chart::chart_title(&doc.root));
                    let (c, v) = chart::axis_titles(&doc.root, chart::uses_xy_axes(&ty));
                    cat_axis = json!(c);
                    val_axis = json!(v);
                    series = chart::series_json(&id, &doc.root, |f| {
                        embed
                            .as_deref()
                            .and_then(|b| chart::embed::read_formula(b, f))
                    })?;
                    legend = chart::legend_json(&doc.root);
                    data_labels = json!({ "show": chart::data_labels_on(&doc.root) });
                    flags = chart::flags_json(&doc.root, &ty);
                    space_for_keys = Some(doc.root);
                }
            }
        }
    }
    let mut m = Map::new();
    m.insert("id".into(), json!(id));
    m.insert("cNvPrId".into(), json!(raw_id));
    m.insert("name".into(), json!(name));
    m.insert("x".into(), json!(x));
    m.insert("y".into(), json!(y));
    m.insert("cx".into(), json!(cx));
    m.insert("cy".into(), json!(cy));
    if let Some(space) = &space_for_keys {
        chart::insert_type_keys(&mut m, &ty, space);
    } else {
        m.insert("type".into(), json!(ty));
    }
    m.insert("title".into(), title);
    m.insert("catAxisTitle".into(), cat_axis);
    m.insert("valAxisTitle".into(), val_axis);
    m.insert("series".into(), Value::Array(series));
    m.insert("legend".into(), legend);
    m.insert("dataLabels".into(), data_labels);
    m.insert("flags".into(), json!(flags));
    m.insert("stable".into(), json!(stable));
    Ok(Value::Object(m))
}

fn load_embed(pkg: &Package, chart_part: &str, space: &Element) -> Option<Vec<u8>> {
    let rid = chart::external_data_rid(space)?;
    let part = crate::opc::resolve_id(pkg, chart_part, &rid).ok()?;
    pkg.get(&part).ok().map(|b| b.to_vec())
}

fn key_matches(frame: &Element, index: usize, key: &ChartKey) -> bool {
    match key {
        ChartKey::Id(want) => cnv_of_frame(frame).0.as_deref() == Some(want.as_str()),
        ChartKey::Index(k) => index == *k,
    }
}

fn find_frame_mut<'a>(root: &'a mut Element, key: &ChartKey) -> Option<&'a mut Element> {
    let mut idx = 0usize;
    find_frame_in(root, &mut idx, key)
}

fn find_frame_in<'a>(
    el: &'a mut Element,
    idx: &mut usize,
    key: &ChartKey,
) -> Option<&'a mut Element> {
    if el.local_name() == "AlternateContent" {
        if let Some(i) = shape::first_alt_child_index(el) {
            if let Node::Element(child) = &mut el.children[i] {
                return find_frame_in(child, idx, key);
            }
        }
        return None;
    }
    if is_counted_chart(el) {
        *idx += 1;
        if key_matches(el, *idx, key) {
            return Some(el);
        }
        return None;
    }
    for child in el.children.as_mut_slice() {
        if let Node::Element(e) = child {
            if let Some(found) = find_frame_in(e, idx, key) {
                return Some(found);
            }
        }
    }
    None
}

fn find_frame_immut<'a>(root: &'a Element, key: &ChartKey) -> Option<&'a Element> {
    let mut idx = 0usize;
    find_frame_immut_in(root, &mut idx, key)
}

fn find_frame_immut_in<'a>(
    el: &'a Element,
    idx: &mut usize,
    key: &ChartKey,
) -> Option<&'a Element> {
    if el.local_name() == "AlternateContent" {
        if let Some(i) = shape::first_alt_child_index(el) {
            if let Node::Element(child) = &el.children[i] {
                return find_frame_immut_in(child, idx, key);
            }
        }
        return None;
    }
    if is_counted_chart(el) {
        *idx += 1;
        if key_matches(el, *idx, key) {
            return Some(el);
        }
        return None;
    }
    for n in &el.children {
        if let Node::Element(e) = n {
            if let Some(found) = find_frame_immut_in(e, idx, key) {
                return Some(found);
            }
        }
    }
    None
}

fn remove_frame(root: &mut Element, key: &ChartKey) -> bool {
    let mut idx = 0usize;
    remove_frame_in(root, &mut idx, key)
}

fn remove_frame_in(el: &mut Element, idx: &mut usize, key: &ChartKey) -> bool {
    if el.local_name() == "AlternateContent" {
        if let Some(i) = shape::first_alt_child_index(el) {
            if let Node::Element(child) = &mut el.children[i] {
                return remove_frame_in(child, idx, key);
            }
        }
        return false;
    }
    let mut i = 0;
    while i < el.children.len() {
        let is_hit = match &el.children[i] {
            Node::Element(e) if is_counted_chart(e) => {
                *idx += 1;
                key_matches(e, *idx, key)
            }
            _ => false,
        };
        if is_hit {
            el.children.remove(i);
            return true;
        }
        if let Node::Element(e) = &mut el.children[i] {
            if !is_counted_chart(e) && remove_frame_in(e, idx, key) {
                return true;
            }
        }
        i += 1;
    }
    false
}

fn rid_still_on_slide(root: &Element, rid: &str) -> bool {
    let mut used = false;
    shape::walk_choice(root, &mut |e| {
        if used {
            return;
        }
        if chart_rid(e).as_deref() == Some(rid) && e.local_name() == "graphicFrame" {
            used = true;
        }
    });
    used
}

fn load_chart_space(
    pkg: &Package,
    slide_root: &Element,
    slide_part: &str,
    target: &ChartTarget,
    id: &str,
    i: usize,
) -> Result<(Document, String), Error> {
    let frame = find_frame_immut(slide_root, &target.key).ok_or_else(|| {
        Error::with_path(
            ErrorKind::ChartNotFound { id: id.to_string() },
            format!("ops[{i}].id"),
        )
    })?;
    let rid = chart_rid(frame).ok_or_else(|| {
        Error::with_path(
            ErrorKind::ChartNotFound { id: id.to_string() },
            format!("ops[{i}].id"),
        )
    })?;
    let chart_part = crate::opc::resolve_id(pkg, slide_part, &rid)?;
    let doc = xml::parse(pkg.get(&chart_part)?)?;
    Ok((doc, chart_part))
}

pub fn export_chart_data(bytes: &[u8], id: &str) -> Result<ExportedMedia, Error> {
    let pkg = Package::open_bytes(bytes.to_vec())?;
    let target = parse_chart_id(id, "id")?;
    let pres_part = crate::opc::office_document_part(&pkg)?;
    let pres = xml::parse(pkg.get(&pres_part)?)?;
    let slides = shape::list_slides(&pkg, &pres_part, &pres.root)?;
    let host = super::picture::TreeHost::from_parts(target.slide, target.on_notes, target.master);
    let part = super::picture::resolve_tree_part(
        &pkg,
        &pres_part,
        &pres.root,
        &slides,
        &host,
        ErrorKind::ChartNotFound { id: id.to_string() },
        "id",
    )?;
    let doc = xml::parse(pkg.get(&part)?)?;
    let frame = find_frame_immut(&doc.root, &target.key)
        .ok_or_else(|| Error::with_path(ErrorKind::ChartNotFound { id: id.to_string() }, "id"))?;
    let rid = chart_rid(frame)
        .ok_or_else(|| Error::with_path(ErrorKind::ChartNotFound { id: id.to_string() }, "id"))?;
    let chart_part = crate::opc::resolve_id(&pkg, &part, &rid)?;
    let chart_doc = xml::parse(pkg.get(&chart_part)?)?;
    let embed_rid = chart::external_data_rid(&chart_doc.root).ok_or_else(|| {
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
    Ok(ExportedMedia {
        bytes: embed_bytes,
        content_type,
    })
}

pub fn insert_chart(
    pkg: &mut Package,
    slide_doc: &mut Document,
    slide_part: &str,
    pres_root: &Element,
    op: &InsertChartOp,
    i: usize,
) -> Result<(), Error> {
    let series = chart::normalize_insert_series(op.chart_type, &op.series);
    let cats = chart::shared_categories(&series);
    let embed = chart::embed::build(op.chart_type, &series, &cats)?;
    let embed_part =
        chart::embed::next_part(pkg, "/ppt/embeddings", "Microsoft_Excel_Worksheet", ".xlsx");
    let chart_part = chart::embed::next_part(pkg, "/ppt/charts", "chart", ".xml");
    pkg.set(&embed_part, embed)?;
    let xml = chart::build_chart_xml(
        op.chart_type,
        op.title.as_deref(),
        &series,
        &cats,
        chart::embed::SHEET,
        chart::embed::origin(),
        true,
        op.of_pie_type.as_deref(),
    );
    pkg.set(&chart_part, xml.into_bytes())?;

    let mut chart_rels = Relationships { rels: Vec::new() };
    chart_rels.push(Relationship {
        id: "rId1".into(),
        rel_type: chart::embed::REL_PACKAGE.into(),
        target: picture::rel_target_from(&chart_part, &embed_part),
        target_mode: None,
        extra: Vec::new(),
    });
    chart_rels.write_to(pkg, &chart_part)?;

    let mut slide_rels = Relationships::from_package(pkg, slide_part)?;
    let chart_rid = slide_rels.next_rid();
    slide_rels.push(Relationship {
        id: chart_rid.clone(),
        rel_type: chart::REL_CHART.into(),
        target: picture::rel_target_from(slide_part, &chart_part),
        target_mode: None,
        extra: Vec::new(),
    });
    slide_rels.write_to(pkg, slide_part)?;

    let mut cts = ContentTypes::from_package(pkg)?;
    cts.set_override(&chart_part, chart::CT_CHART)?;
    cts.set_override(&embed_part, chart::embed::CT_EMBED_XLSX)?;
    cts.write_to(pkg)?;

    let (cx, cy) = (
        op.cx.unwrap_or(chart::DEFAULT_CX),
        op.cy.unwrap_or(chart::DEFAULT_CY),
    );
    let (x, y) = match (op.x, op.y) {
        (Some(px), Some(py)) => (px, py),
        _ => {
            if op.on_notes {
                picture::vacant_pos_canvas(
                    super::notes::notes_size(pres_root),
                    &slide_doc.root,
                    cx,
                    cy,
                )
            } else {
                picture::vacant_pos(pres_root, &slide_doc.root, cx, cy)
            }
        }
    };
    picture::ensure_xmlns(&mut slide_doc.root, "r", REL_NS);
    picture::ensure_xmlns(&mut slide_doc.root, "a", DML_NS);
    picture::ensure_xmlns(&mut slide_doc.root, "c", CHART_NS);
    let cnv = picture::next_cnv_id(&slide_doc.root);
    let frame = build_graphic_frame(cnv, &format!("图表 {cnv}"), &chart_rid, x, y, cx, cy);
    picture::insert_into_sp_tree(&mut slide_doc.root, frame)?;
    let _ = i;
    Ok(())
}

fn build_graphic_frame(
    cnv: u32,
    name: &str,
    rid: &str,
    x: i64,
    y: i64,
    cx: i64,
    cy: i64,
) -> Element {
    let mut nv = Element::new("p:nvGraphicFramePr");
    nv.push_element(
        Element::new("p:cNvPr")
            .with_attr("id", cnv.to_string())
            .with_attr("name", name),
    );
    let mut locks = Element::new("p:cNvGraphicFramePr");
    locks.push_element(Element::new("a:graphicFrameLocks").with_attr("noGrp", "1"));
    nv.push_element(locks);
    nv.push_element(Element::new("p:nvPr"));
    let mut xfrm = Element::new("p:xfrm");
    xfrm.push_element(
        Element::new("a:off")
            .with_attr("x", x.to_string())
            .with_attr("y", y.to_string()),
    );
    xfrm.push_element(
        Element::new("a:ext")
            .with_attr("cx", cx.to_string())
            .with_attr("cy", cy.to_string()),
    );
    let mut data = Element::new("a:graphicData").with_attr("uri", CHART_NS);
    data.push_element(
        Element::new("c:chart")
            .with_attr("xmlns:c", CHART_NS)
            .with_attr("xmlns:r", REL_NS)
            .with_attr("r:id", rid),
    );
    let mut graphic = Element::new("a:graphic");
    graphic.push_element(data);
    let mut frame = Element::new("p:graphicFrame");
    frame.push_element(nv);
    frame.push_element(xfrm);
    frame.push_element(graphic);
    frame
}

pub fn delete_chart(
    pkg: &mut Package,
    slide_doc: &mut Document,
    slide_part: &str,
    op: &DeleteChartOp,
    i: usize,
) -> Result<(), Error> {
    let path = format!("ops[{i}].id");
    let rid = find_frame_immut(&slide_doc.root, &op.target.key).and_then(chart_rid);
    if !remove_frame(&mut slide_doc.root, &op.target.key) {
        return Err(Error::with_path(
            ErrorKind::ChartNotFound { id: op.id.clone() },
            path,
        ));
    }
    let Some(rid) = rid else {
        return Ok(());
    };
    let chart_part = match crate::opc::resolve_id(pkg, slide_part, &rid) {
        Ok(p) => p,
        Err(_) => return Ok(()),
    };
    let mut slide_rels = Relationships::from_package(pkg, slide_part)?;
    if !rid_still_on_slide(&slide_doc.root, &rid) {
        slide_rels.remove_by_id(&rid);
        slide_rels.write_to(pkg, slide_part)?;
    }
    drop_chart_tree(pkg, &chart_part)?;
    Ok(())
}

fn drop_chart_tree(pkg: &mut Package, chart_part: &str) -> Result<(), Error> {
    let satellites = {
        let rels = Relationships::from_package(pkg, chart_part)?;
        let mut out = Vec::new();
        for r in &rels.rels {
            if r.is_external() {
                continue;
            }
            if let Ok(p) = resolve_rel_target(chart_part, &r.target) {
                out.push(p);
            }
        }
        out
    };
    drop_unused(pkg, chart_part)?;
    for p in satellites {
        drop_unused(pkg, &p)?;
    }
    Ok(())
}

fn drop_unused(pkg: &mut Package, part: &str) -> Result<(), Error> {
    if picture::media_still_used(pkg, part)? {
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

pub fn apply_title(
    pkg: &mut Package,
    slide_root: &Element,
    slide_part: &str,
    op: &SetChartTitleOp,
    i: usize,
) -> Result<(), Error> {
    let (mut space, chart_part) =
        load_chart_space(pkg, slide_root, slide_part, &op.target, &op.id, i)?;
    chart::reject_restricted(&space.root, &op.id, i, ChartRestrict::Decorate)?;
    chart::set_chart_title(&mut space.root, &op.title);
    pkg.set(&chart_part, space.to_bytes()?)?;
    Ok(())
}

pub fn apply_series(
    pkg: &mut Package,
    slide_root: &Element,
    slide_part: &str,
    op: &SetChartSeriesOp,
    i: usize,
) -> Result<(), Error> {
    let (mut space, chart_part) =
        load_chart_space(pkg, slide_root, slide_part, &op.target, &op.id, i)?;
    chart::reject_restricted(&space.root, &op.id, i, ChartRestrict::Data)?;
    if op.index >= chart::series_count(&space.root) {
        return Err(Error::with_path(
            ErrorKind::ChartSeriesNotFound { id: op.id.clone() },
            format!("ops[{i}].id"),
        ));
    }
    if op.sizes.is_some() && !series_is_bubble(&space.root, op.index) {
        return Err(Error::with_path(
            ErrorKind::SizesOnNonBubble,
            format!("ops[{i}].sizes"),
        ));
    }
    write_series(
        pkg,
        &mut space.root,
        &chart_part,
        op.index,
        op.name.as_deref(),
        op.categories.as_deref(),
        op.values.as_deref(),
        op.sizes.as_deref(),
        i,
    )?;
    pkg.set(&chart_part, space.to_bytes()?)?;
    Ok(())
}

fn series_is_bubble(space: &Element, index: usize) -> bool {
    chart::collect_sers(space)
        .get(index)
        .is_some_and(|s| s.child_by_local("bubbleSize").is_some())
}

fn write_series(
    pkg: &mut Package,
    space: &mut Element,
    chart_part: &str,
    index: usize,
    name: Option<&str>,
    cats: Option<&[Value]>,
    vals: Option<&[Value]>,
    sizes: Option<&[Value]>,
    i: usize,
) -> Result<(), Error> {
    let snapshot = chart::with_ser_mut(space, index, |ser| {
        if let Some(n) = name {
            chart::set_series_name(ser, n);
        }
        if let Some(c) = cats {
            let local = if ser.child_by_local("xVal").is_some() {
                "xVal"
            } else {
                "cat"
            };
            let f = ser.child_by_local(local).and_then(chart::formula_of);
            let new_f = chart::embed::resized_formula(f.as_deref(), c.len());
            chart::set_pts(ser, local, c, new_f.as_deref());
        }
        if let Some(v) = vals {
            let local = if ser.child_by_local("yVal").is_some() {
                "yVal"
            } else {
                "val"
            };
            let f = ser.child_by_local(local).and_then(chart::formula_of);
            let new_f = chart::embed::resized_formula(f.as_deref(), v.len());
            chart::set_pts(ser, local, v, new_f.as_deref());
        }
        if let Some(s) = sizes {
            let f = ser.child_by_local("bubbleSize").and_then(chart::formula_of);
            let new_f = chart::embed::resized_formula(f.as_deref(), s.len());
            chart::set_pts(ser, "bubbleSize", s, new_f.as_deref());
        }
        ser.clone()
    });
    let Some(ser) = snapshot else {
        return Err(Error::with_path(
            ErrorKind::ChartSeriesNotFound {
                id: format!("series:{index}"),
            },
            format!("ops[{i}].id"),
        ));
    };
    if let Some(embed_rid) = chart::external_data_rid(space) {
        let embed_part = crate::opc::resolve_id(pkg, chart_part, &embed_rid)?;
        let bytes = pkg.get(&embed_part)?.to_vec();
        let updated = chart::embed::patch_series(&bytes, &ser, name, cats, vals, sizes)?;
        pkg.set(&embed_part, updated)?;
    }
    Ok(())
}

pub fn apply_add_series(
    pkg: &mut Package,
    slide_root: &Element,
    slide_part: &str,
    op: &AddChartSeriesOp,
    i: usize,
) -> Result<(), Error> {
    let (mut space, chart_part) =
        load_chart_space(pkg, slide_root, slide_part, &op.target, &op.id, i)?;
    chart::reject_restricted(&space.root, &op.id, i, ChartRestrict::Data)?;
    crate::chart::remain::reject_structure_edit(&space.root, &op.id, i)?;
    let ty = chart::inspect_type(&space.root, false);
    if ty == "pie" || ty == "doughnut" {
        return Err(Error::with_path(
            ErrorKind::PieNeedsOneSeries,
            format!("ops[{i}]"),
        ));
    }
    if op.series_type.is_some() && ty != "combo" {
        return Err(Error::with_path(
            ErrorKind::InvalidComboSeriesType,
            format!("ops[{i}].type"),
        ));
    }
    if op.sizes.is_some() && ty != "bubble" {
        return Err(Error::with_path(
            ErrorKind::SizesOnNonBubble,
            format!("ops[{i}].sizes"),
        ));
    }
    add_series(
        pkg,
        &mut space.root,
        &chart_part,
        op.name.as_deref(),
        op.categories.as_deref(),
        &op.values,
        op.sizes.as_deref(),
        op.series_type,
        i,
    )?;
    pkg.set(&chart_part, space.to_bytes()?)?;
    Ok(())
}

fn add_series(
    pkg: &mut Package,
    space: &mut Element,
    chart_part: &str,
    name: Option<&str>,
    cats: Option<&[Value]>,
    vals: &[Value],
    sizes: Option<&[Value]>,
    series_type: Option<ChartType>,
    i: usize,
) -> Result<(), Error> {
    let n = chart::series_count(space);
    let ty = chart::inspect_type(space, false);
    let cats_owned: Vec<Value> = if let Some(c) = cats {
        c.to_vec()
    } else if n > 0 {
        let first = chart::collect_sers(space)[0];
        let local = if first.child_by_local("xVal").is_some() {
            "xVal"
        } else {
            "cat"
        };
        chart::cache_values(first.child_by_local(local).unwrap_or(first)).unwrap_or_default()
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
    let idx = chart::unused_idx(space);
    let name = name
        .map(str::to_string)
        .unwrap_or_else(|| format!("系列{}", n + 1));
    let y_col_n = chart::next_val_col_from_formulas(space).unwrap_or(2);
    let col = chart::col_letters(y_col_n);
    let size_col_n = if ty == "bubble" {
        Some(y_col_n + 1)
    } else {
        None
    };
    let cat_ref = chart::collect_sers(space)
        .first()
        .and_then(|s| s.child_by_local("cat").or_else(|| s.child_by_local("xVal")))
        .and_then(chart::formula_of)
        .unwrap_or_else(|| format!("Sheet1!$A$2:$A${}", cats_owned.len() + 1));
    let val_ref = format!("Sheet1!${col}$2:${col}${}", vals.len() + 1);
    let name_ref = format!("Sheet1!${col}$1");
    let xy = chart::uses_xy_axes(&ty)
        || series_type.is_some_and(ChartType::is_xy)
        || chart::last_series_type(space).is_some_and(ChartType::is_xy);
    let sizes_owned = if ty == "bubble" {
        Some(
            sizes
                .map(|s| s.to_vec())
                .unwrap_or_else(|| chart::ones(vals.len())),
        )
    } else {
        None
    };
    let size_pair = match (sizes_owned.as_deref(), size_col_n.as_ref()) {
        (Some(sz), Some(cn)) => {
            let letters = chart::col_letters(*cn);
            let r = format!("Sheet1!${letters}$2:${letters}${}", sz.len() + 1);
            Some((sz, r))
        }
        _ => None,
    };
    let size_pair_ref = size_pair.as_ref().map(|(sz, r)| (*sz, r.as_str()));
    let xml = chart::ser_xml(
        idx,
        &name,
        &name_ref,
        &cat_ref,
        &cats_owned,
        &val_ref,
        vals,
        xy,
        size_pair_ref,
    );
    let ser_el = chart::parse_ser_xml(&xml)?;
    chart::insert_ser(space, ser_el, series_type);
    if let Some(embed_rid) = chart::external_data_rid(space) {
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
        if let (Some(sz), Some(cn)) = (sizes_owned.as_ref(), size_col_n) {
            let letters = chart::col_letters(cn);
            for (ri, v) in sz.iter().enumerate() {
                ops.push(json!({
                    "op": "setCell",
                    "id": format!("cell:Sheet1!{letters}{}", ri + 2),
                    "value": v
                }));
            }
        }
        let updated = chart::embed::apply_ops(&bytes, ops)?;
        pkg.set(&embed_part, updated)?;
    }
    Ok(())
}

pub fn apply_delete_series(
    pkg: &mut Package,
    slide_root: &Element,
    slide_part: &str,
    op: &DeleteChartSeriesOp,
    i: usize,
) -> Result<(), Error> {
    let (mut space, chart_part) =
        load_chart_space(pkg, slide_root, slide_part, &op.target, &op.id, i)?;
    chart::reject_restricted(&space.root, &op.id, i, ChartRestrict::Data)?;
    crate::chart::remain::reject_structure_edit(&space.root, &op.id, i)?;
    let n = chart::series_count(&space.root);
    if op.index >= n {
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
    chart::remove_series_at(&mut space.root, op.index);
    pkg.set(&chart_part, space.to_bytes()?)?;
    Ok(())
}

pub fn apply_type(
    pkg: &mut Package,
    slide_root: &Element,
    slide_part: &str,
    op: &SetChartTypeOp,
    i: usize,
) -> Result<(), Error> {
    let (mut space, chart_part) =
        load_chart_space(pkg, slide_root, slide_part, &op.target, &op.id, i)?;
    chart::reject_restricted(&space.root, &op.id, i, ChartRestrict::Data)?;
    if op.chart_type == ChartType::Combo {
        return Err(Error::with_path(
            ErrorKind::InvalidChartType,
            format!("ops[{i}].type"),
        ));
    }
    chart::convert_type(&mut space.root, op.chart_type, op.of_pie_type.as_deref(), i)?;
    pkg.set(&chart_part, space.to_bytes()?)?;
    Ok(())
}

pub fn apply_axis_title(
    pkg: &mut Package,
    slide_root: &Element,
    slide_part: &str,
    op: &SetChartAxisTitleOp,
    i: usize,
) -> Result<(), Error> {
    let (mut space, chart_part) =
        load_chart_space(pkg, slide_root, slide_part, &op.target, &op.id, i)?;
    chart::reject_restricted(&space.root, &op.id, i, ChartRestrict::Decorate)?;
    chart::set_axis_title(&mut space.root, &op.axis, &op.title, i)?;
    pkg.set(&chart_part, space.to_bytes()?)?;
    Ok(())
}

pub fn apply_legend(
    pkg: &mut Package,
    slide_root: &Element,
    slide_part: &str,
    op: &SetChartLegendOp,
    i: usize,
) -> Result<(), Error> {
    let (mut space, chart_part) =
        load_chart_space(pkg, slide_root, slide_part, &op.target, &op.id, i)?;
    chart::reject_restricted(&space.root, &op.id, i, ChartRestrict::Decorate)?;
    chart::set_legend(&mut space.root, op.show, op.pos.as_deref());
    pkg.set(&chart_part, space.to_bytes()?)?;
    Ok(())
}

pub fn apply_data_labels(
    pkg: &mut Package,
    slide_root: &Element,
    slide_part: &str,
    op: &SetChartDataLabelsOp,
    i: usize,
) -> Result<(), Error> {
    let (mut space, chart_part) =
        load_chart_space(pkg, slide_root, slide_part, &op.target, &op.id, i)?;
    chart::reject_restricted(&space.root, &op.id, i, ChartRestrict::Decorate)?;
    chart::set_data_labels(&mut space.root, op.show);
    pkg.set(&chart_part, space.to_bytes()?)?;
    Ok(())
}

pub fn apply_insert_trendline(
    pkg: &mut Package,
    slide_root: &Element,
    slide_part: &str,
    op: &InsertTrendlineOp,
    i: usize,
) -> Result<(), Error> {
    let (mut space, chart_part) =
        load_chart_space(pkg, slide_root, slide_part, &op.target, &op.id, i)?;
    chart::reject_restricted(&space.root, &op.id, i, ChartRestrict::Decorate)?;
    crate::chart::remain::insert_trendline(&mut space.root, &op.id, op.index, &op.spec, i)?;
    pkg.set(&chart_part, space.to_bytes()?)?;
    Ok(())
}

pub fn apply_set_trendline(
    pkg: &mut Package,
    slide_root: &Element,
    slide_part: &str,
    op: &SetTrendlineOp,
    i: usize,
) -> Result<(), Error> {
    let (mut space, chart_part) =
        load_chart_space(pkg, slide_root, slide_part, &op.target, &op.id, i)?;
    chart::reject_restricted(&space.root, &op.id, i, ChartRestrict::Decorate)?;
    crate::chart::remain::set_trendline(
        &mut space.root,
        &op.id,
        op.index,
        op.tl_index,
        &op.spec,
        i,
    )?;
    pkg.set(&chart_part, space.to_bytes()?)?;
    Ok(())
}

pub fn apply_delete_trendline(
    pkg: &mut Package,
    slide_root: &Element,
    slide_part: &str,
    op: &DeleteTrendlineOp,
    i: usize,
) -> Result<(), Error> {
    let (mut space, chart_part) =
        load_chart_space(pkg, slide_root, slide_part, &op.target, &op.id, i)?;
    chart::reject_restricted(&space.root, &op.id, i, ChartRestrict::Decorate)?;
    crate::chart::remain::delete_trendline(&mut space.root, &op.id, op.index, op.tl_index, i)?;
    pkg.set(&chart_part, space.to_bytes()?)?;
    Ok(())
}

pub fn apply_insert_error_bars(
    pkg: &mut Package,
    slide_root: &Element,
    slide_part: &str,
    op: &InsertErrorBarsOp,
    i: usize,
) -> Result<(), Error> {
    let (mut space, chart_part) =
        load_chart_space(pkg, slide_root, slide_part, &op.target, &op.id, i)?;
    chart::reject_restricted(&space.root, &op.id, i, ChartRestrict::Decorate)?;
    crate::chart::remain::insert_error_bars(&mut space.root, &op.id, op.index, &op.spec, i)?;
    pkg.set(&chart_part, space.to_bytes()?)?;
    Ok(())
}

pub fn apply_set_error_bars(
    pkg: &mut Package,
    slide_root: &Element,
    slide_part: &str,
    op: &SetErrorBarsOp,
    i: usize,
) -> Result<(), Error> {
    let (mut space, chart_part) =
        load_chart_space(pkg, slide_root, slide_part, &op.target, &op.id, i)?;
    chart::reject_restricted(&space.root, &op.id, i, ChartRestrict::Decorate)?;
    crate::chart::remain::set_error_bars(
        &mut space.root,
        &op.id,
        op.index,
        op.eb_index,
        &op.spec,
        i,
    )?;
    pkg.set(&chart_part, space.to_bytes()?)?;
    Ok(())
}

pub fn apply_delete_error_bars(
    pkg: &mut Package,
    slide_root: &Element,
    slide_part: &str,
    op: &DeleteErrorBarsOp,
    i: usize,
) -> Result<(), Error> {
    let (mut space, chart_part) =
        load_chart_space(pkg, slide_root, slide_part, &op.target, &op.id, i)?;
    chart::reject_restricted(&space.root, &op.id, i, ChartRestrict::Decorate)?;
    crate::chart::remain::delete_error_bars(&mut space.root, &op.id, op.index, op.eb_index, i)?;
    pkg.set(&chart_part, space.to_bytes()?)?;
    Ok(())
}

pub fn apply_series_axis(
    pkg: &mut Package,
    slide_root: &Element,
    slide_part: &str,
    op: &SetChartSeriesAxisOp,
    i: usize,
) -> Result<(), Error> {
    let (mut space, chart_part) =
        load_chart_space(pkg, slide_root, slide_part, &op.target, &op.id, i)?;
    chart::reject_restricted(&space.root, &op.id, i, ChartRestrict::Decorate)?;
    crate::chart::remain::set_series_axis(&mut space.root, &op.id, op.index, op.secondary, i)?;
    pkg.set(&chart_part, space.to_bytes()?)?;
    Ok(())
}

pub fn apply_size(slide_doc: &mut Document, op: &SetChartSizeOp, i: usize) -> Result<(), Error> {
    let path = format!("ops[{i}].id");
    let frame = find_frame_mut(&mut slide_doc.root, &op.target.key)
        .ok_or_else(|| Error::with_path(ErrorKind::ChartNotFound { id: op.id.clone() }, path))?;
    let xfrm = ensure_frame_xfrm(frame);
    let ext = ensure_a(xfrm, "ext");
    if let Some(cx) = op.cx {
        picture::set_attr_local(ext, "cx", &cx.to_string());
    }
    if let Some(cy) = op.cy {
        picture::set_attr_local(ext, "cy", &cy.to_string());
    }
    Ok(())
}

pub fn apply_pos(slide_doc: &mut Document, op: &SetChartPosOp, i: usize) -> Result<(), Error> {
    let path = format!("ops[{i}].id");
    let frame = find_frame_mut(&mut slide_doc.root, &op.target.key)
        .ok_or_else(|| Error::with_path(ErrorKind::ChartNotFound { id: op.id.clone() }, path))?;
    let xfrm = ensure_frame_xfrm(frame);
    let off = ensure_a(xfrm, "off");
    if let Some(x) = op.x {
        picture::set_attr_local(off, "x", &x.to_string());
    }
    if let Some(y) = op.y {
        picture::set_attr_local(off, "y", &y.to_string());
    }
    Ok(())
}

fn ensure_frame_xfrm(frame: &mut Element) -> &mut Element {
    if frame.child_by_local("xfrm").is_none() {
        frame.push_element(Element::new("p:xfrm"));
    }
    frame.child_by_local_mut("xfrm").expect("刚插入的 p:xfrm")
}

fn ensure_a<'a>(parent: &'a mut Element, local: &str) -> &'a mut Element {
    if parent.child_by_local(local).is_none() {
        parent.push_element(Element::new(format!("a:{local}")));
    }
    parent.child_by_local_mut(local).expect("刚插入的 a 子元素")
}
