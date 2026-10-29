//! 文本框：从 DrawingML / WordprocessingShape 收集 inspect 节点，并按 id 改框。

use serde_json::{json, Value};

use crate::error::{Error, ErrorKind, XmlIssue};
use crate::opc::{Package, Relationships};
use crate::xml::{Element, Node};

use super::charts;
use super::field;
use super::link;
use super::math;
use super::para::{self, BodyPara, TxbxKey, TxbxTarget};
use super::picture::{self, PicKind, WrapKind};
use super::ruby;
use super::sdt;
use super::smartart;
use super::tbl;
use super::{format, MC_NS, W14_NS, WP_NS};

/// 现代文本框命名空间（MS-DOCX WordprocessingShape）。
pub const WPS_NS: &str = "http://schemas.microsoft.com/office/word/2010/wordprocessingShape";
pub const WPG_NS: &str = "http://schemas.microsoft.com/office/word/2010/wordprocessingGroup";

/// 框内文字相对框的 `wps:bodyPr/@anchor`（ST_TextAnchoringType）。
pub const TEXT_BOX_TEXT_ANCHORS: &[&str] = &["t", "ctr", "b", "just", "dist"];

#[derive(Clone)]
pub struct TxbxInfo {
    pub id: String,
    pub doc_pr_id: Option<String>,
    pub name: Option<String>,
    pub kind: PicKind,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
    pub wrap: Option<WrapKind>,
    pub stable: bool,
    pub para_count: usize,
    pub picture_count: usize,
    pub chart_count: usize,
    pub smart_art_count: usize,
    pub table_count: usize,
    pub hyperlink_count: usize,
    pub bookmark_count: usize,
    pub field_count: usize,
    pub math_count: usize,
    pub ruby_count: usize,
    pub content_control_count: usize,
    pub story_id: Option<String>,
    pub seq: u32,
    pub prev: Option<String>,
    pub next: Option<String>,
}

pub fn collect_textboxes(root: &Element, host_para_id: &str) -> Result<Vec<TxbxInfo>, Error> {
    let mut out = Vec::new();
    let mut index = 0usize;
    walk_host(root, &mut |drawing| {
        if super::watermark::is_watermark_drawing(drawing) {
            return Ok(());
        }
        if is_group(drawing) {
            collect_group_boxes(drawing, host_para_id, &mut index, &mut out)?;
        } else if is_textbox(drawing) {
            index += 1;
            out.push(txbx_from_drawing(drawing, host_para_id, index)?);
        }
        Ok(())
    })?;
    Ok(out)
}

/// 在同一部件内按故事 id 与 seq 接上 prev / next。
pub fn resolve_links(boxes: &mut [TxbxInfo]) {
    let mut stories: Vec<(String, Vec<usize>)> = Vec::new();
    for (i, b) in boxes.iter().enumerate() {
        let Some(sid) = b.story_id.as_ref() else {
            continue;
        };
        if let Some((_, idxs)) = stories.iter_mut().find(|(s, _)| s == sid) {
            idxs.push(i);
        } else {
            stories.push((sid.clone(), vec![i]));
        }
    }
    for (_, mut idxs) in stories {
        if idxs.len() < 2 {
            continue;
        }
        idxs.sort_by_key(|i| boxes[*i].seq);
        for pos in 0..idxs.len() {
            let i = idxs[pos];
            if pos > 0 {
                let prev = idxs[pos - 1];
                boxes[i].prev = Some(boxes[prev].id.clone());
            }
            if pos + 1 < idxs.len() {
                let next = idxs[pos + 1];
                boxes[i].next = Some(boxes[next].id.clone());
            }
        }
    }
}

#[derive(Clone, Copy, Default)]
pub struct TxbxExpand {
    pub content: bool,
    pub pictures: bool,
    pub runs: bool,
    pub charts: bool,
    pub smart_art: bool,
    pub tables: bool,
    pub hyperlinks: bool,
    pub bookmarks: bool,
    pub fields: bool,
    pub maths: bool,
    pub rubies: bool,
    pub content_controls: bool,
}

pub fn textboxes_json(
    pkg: &Package,
    host_part: &str,
    host: &Element,
    host_para_id: &str,
    boxes: &[TxbxInfo],
    expand: TxbxExpand,
) -> Result<Vec<Value>, Error> {
    let mut out = Vec::new();
    for b in boxes {
        let text_anchor = text_anchor_json(host, b)?;
        let mut node = json!({
            "id": b.id,
            "docPrId": b.doc_pr_id,
            "name": b.name,
            "kind": b.kind.as_str(),
            "cx": b.cx,
            "cy": b.cy,
            "wrap": b.wrap.map(WrapKind::as_str),
            "textAnchor": text_anchor,
            "link": {
                "prev": b.prev,
                "next": b.next,
            },
            "paraCount": b.para_count,
            "pictureCount": b.picture_count,
            "chartCount": b.chart_count,
            "smartArtCount": b.smart_art_count,
            "tableCount": b.table_count,
            "hyperlinkCount": b.hyperlink_count,
            "bookmarkCount": b.bookmark_count,
            "fieldCount": b.field_count,
            "mathCount": b.math_count,
            "rubyCount": b.ruby_count,
            "contentControlCount": b.content_control_count,
            "stable": b.stable,
        });
        let rels = Relationships::from_package(pkg, host_part)?;
        node["hyperlink"] = match key_of_info(b).and_then(|k| find_textbox_drawing(host, &k)) {
            Some(d) => super::drawing_hlink::inspect_textbox(d, Some(&rels)),
            None => Value::Null,
        };
        if expand.content {
            fill_one_box(
                pkg,
                host_part,
                host,
                host_para_id,
                b,
                &mut node,
                expand.pictures,
                expand.runs,
            )?;
        }
        if expand.charts {
            node["charts"] =
                Value::Array(collect_inner_charts(pkg, host_part, host, host_para_id, b)?);
        }
        if expand.smart_art {
            node["smartArts"] = Value::Array(collect_inner_smartarts(
                pkg,
                host_part,
                host,
                host_para_id,
                b,
            )?);
        }
        if expand.tables && !is_note_or_comment_box(&b.id) {
            node["tables"] = match find_txbx_content_for(host, host_para_id, &b.id)? {
                Some(content) => Value::Array(tbl::tables_json(
                    pkg,
                    host_part,
                    content,
                    Some(&b.id),
                    expand.pictures,
                    expand.runs,
                )?),
                None => json!([]),
            };
        }
        if !is_note_or_comment_box(&b.id) {
            if let Some(content) = find_txbx_content_for(host, host_para_id, &b.id)? {
                if expand.hyperlinks {
                    node["hyperlinks"] =
                        Value::Array(link::hyperlinks_json(content, Some(&b.id), &rels)?);
                }
                if expand.bookmarks {
                    node["bookmarks"] = Value::Array(link::bookmarks_json(content, Some(&b.id))?);
                }
                if expand.fields {
                    node["fields"] = Value::Array(field::fields_json(content, Some(&b.id))?);
                }
                if expand.maths {
                    node["maths"] = Value::Array(math::maths_json(content, Some(&b.id))?);
                }
                if expand.rubies {
                    node["rubies"] = Value::Array(ruby::rubies_json(content, Some(&b.id))?);
                }
                if expand.content_controls {
                    node["contentControls"] = Value::Array(sdt::sdts_json(content, Some(&b.id))?);
                }
            } else {
                if expand.hyperlinks {
                    node["hyperlinks"] = json!([]);
                }
                if expand.bookmarks {
                    node["bookmarks"] = json!([]);
                }
                if expand.fields {
                    node["fields"] = json!([]);
                }
                if expand.maths {
                    node["maths"] = json!([]);
                }
                if expand.rubies {
                    node["rubies"] = json!([]);
                }
                if expand.content_controls {
                    node["contentControls"] = json!([]);
                }
            }
        }
        out.push(node);
    }
    Ok(out)
}

fn fill_one_box(
    pkg: &Package,
    host_part: &str,
    host: &Element,
    host_para_id: &str,
    info: &TxbxInfo,
    slot: &mut Value,
    pictures: bool,
    runs: bool,
) -> Result<(), Error> {
    let Some(content) = find_txbx_content_for(host, host_para_id, &info.id)? else {
        slot["paragraphs"] = json!([]);
        if pictures {
            slot["pictures"] = json!([]);
        }
        return Ok(());
    };
    let rels = Relationships::from_package(pkg, host_part)?;
    let paras = para::collect_paragraphs_in_txbx(content);
    let mut para_nodes = Vec::new();
    let mut pic_nodes = Vec::new();
    for (i, p) in paras.iter().enumerate() {
        let index = i + 1;
        let (pid, _) = para::para_stable_id_in(&info.id, p, index);
        let pics = picture::collect_pictures_with(p, &pid, Some(&rels))?;
        let mut node = format::para_node(p, index, Some(&info.id), Some(pics.len()), None, None);
        if pictures {
            node["pictures"] = Value::Array(picture::pictures_json(&pics));
            pic_nodes.extend(picture::pictures_json(&pics));
        }
        format::maybe_runs(&mut node, p, runs);
        para_nodes.push(node);
    }
    slot["paragraphs"] = Value::Array(para_nodes);
    if pictures {
        slot["pictures"] = Value::Array(pic_nodes);
    }
    Ok(())
}

fn collect_group_boxes(
    drawing: &Element,
    host_para_id: &str,
    index: &mut usize,
    out: &mut Vec<TxbxInfo>,
) -> Result<(), Error> {
    let kind = drawing_kind(drawing);
    let wrap = drawing_wrap(drawing, kind);
    let extent = drawing_extent(drawing)?;
    walk_shape(drawing, &mut |e| {
        if e.local_name() == "wsp" && has_txbx(e) {
            *index += 1;
            if let Ok(info) = txbx_from_wsp(e, host_para_id, *index, kind, wrap, extent) {
                out.push(info);
            }
        }
    });
    Ok(())
}

fn txbx_from_drawing(drawing: &Element, host: &str, index: usize) -> Result<TxbxInfo, Error> {
    let kind = drawing_kind(drawing);
    let wrap = drawing_wrap(drawing, kind);
    let (cx, cy) = drawing_extent(drawing)?;
    let (doc_pr_id, name) = doc_pr(drawing);
    let (id, stable) = box_id(host, &doc_pr_id, index);
    let content = find_txbx_content(drawing);
    let counts = content_counts(content, &id)?;
    let (story_id, seq) = story_of(drawing);
    Ok(TxbxInfo {
        id,
        doc_pr_id,
        name,
        kind,
        cx,
        cy,
        wrap,
        stable,
        para_count: counts.paras,
        picture_count: counts.pics,
        chart_count: counts.charts,
        smart_art_count: counts.smart,
        table_count: counts.tables,
        hyperlink_count: counts.hyperlinks,
        bookmark_count: counts.bookmarks,
        field_count: counts.fields,
        math_count: counts.maths,
        ruby_count: counts.rubies,
        content_control_count: counts.sdts,
        story_id,
        seq,
        prev: None,
        next: None,
    })
}

fn txbx_from_wsp(
    wsp: &Element,
    host: &str,
    index: usize,
    kind: PicKind,
    wrap: Option<WrapKind>,
    extent: (Option<i64>, Option<i64>),
) -> Result<TxbxInfo, Error> {
    let (doc_pr_id, name) = cnv_pr(wsp);
    let (id, stable) = box_id(host, &doc_pr_id, index);
    let content = find_txbx_content(wsp);
    let counts = content_counts(content, &id)?;
    let (story_id, seq) = story_of(wsp);
    Ok(TxbxInfo {
        id,
        doc_pr_id,
        name,
        kind,
        cx: extent.0,
        cy: extent.1,
        wrap,
        stable,
        para_count: counts.paras,
        picture_count: counts.pics,
        chart_count: counts.charts,
        smart_art_count: counts.smart,
        table_count: counts.tables,
        hyperlink_count: counts.hyperlinks,
        bookmark_count: counts.bookmarks,
        field_count: counts.fields,
        math_count: counts.maths,
        ruby_count: counts.rubies,
        content_control_count: counts.sdts,
        story_id,
        seq,
        prev: None,
        next: None,
    })
}

fn box_id(host: &str, doc_pr_id: &Option<String>, index: usize) -> (String, bool) {
    match doc_pr_id {
        Some(d) => (format!("{host}/txbx:{d}"), true),
        None => (format!("{host}/txbx:index:{index}"), false),
    }
}

struct ContentCounts {
    paras: usize,
    pics: usize,
    charts: usize,
    smart: usize,
    tables: usize,
    hyperlinks: usize,
    bookmarks: usize,
    fields: usize,
    maths: usize,
    rubies: usize,
    sdts: usize,
}

fn content_counts(content: Option<&Element>, box_id: &str) -> Result<ContentCounts, Error> {
    let empty = ContentCounts {
        paras: 0,
        pics: 0,
        charts: 0,
        smart: 0,
        tables: 0,
        hyperlinks: 0,
        bookmarks: 0,
        fields: 0,
        maths: 0,
        rubies: 0,
        sdts: 0,
    };
    let Some(content) = content else {
        return Ok(empty);
    };
    let paras = para::collect_paragraphs_in_txbx(content);
    let mut pics = 0usize;
    let mut chart_n = 0usize;
    let mut smart_n = 0usize;
    for (i, p) in paras.iter().enumerate() {
        let (pid, _) = para::para_stable_id_in(box_id, p, i + 1);
        pics += picture::collect_pictures(p, &pid)?.len();
        chart_n += charts::count_charts(p)?;
        smart_n += smartart::count_smartarts(p)?;
    }
    let skip = is_note_or_comment_box(box_id);
    Ok(ContentCounts {
        paras: paras.len(),
        pics,
        charts: chart_n,
        smart: smart_n,
        tables: if skip { 0 } else { tbl::count_tables(content) },
        hyperlinks: if skip {
            0
        } else {
            link::count_hyperlinks(content)
        },
        bookmarks: if skip {
            0
        } else {
            link::count_bookmarks(content)
        },
        fields: if skip {
            0
        } else {
            field::count_fields(content)
        },
        maths: if skip { 0 } else { math::count_maths(content) },
        rubies: if skip { 0 } else { ruby::count_rubies(content) },
        sdts: if skip { 0 } else { sdt::count_sdts(content) },
    })
}

fn is_note_or_comment_box(id: &str) -> bool {
    id.starts_with("footnote:") || id.starts_with("endnote:") || id.starts_with("comment:")
}

fn collect_inner_charts(
    pkg: &Package,
    host_part: &str,
    host: &Element,
    host_para_id: &str,
    info: &TxbxInfo,
) -> Result<Vec<Value>, Error> {
    let Some(content) = find_txbx_content_for(host, host_para_id, &info.id)? else {
        return Ok(Vec::new());
    };
    let paras = para::collect_paragraphs_in_txbx(content);
    let mut out = Vec::new();
    for (i, p) in paras.iter().enumerate() {
        let (pid, _) = para::para_stable_id_in(&info.id, p, i + 1);
        out.extend(charts::collect_in_para(pkg, host_part, p, &pid)?);
    }
    Ok(out)
}

fn collect_inner_smartarts(
    pkg: &Package,
    host_part: &str,
    host: &Element,
    host_para_id: &str,
    info: &TxbxInfo,
) -> Result<Vec<Value>, Error> {
    let Some(content) = find_txbx_content_for(host, host_para_id, &info.id)? else {
        return Ok(Vec::new());
    };
    let paras = para::collect_paragraphs_in_txbx(content);
    let mut out = Vec::new();
    for (i, p) in paras.iter().enumerate() {
        let (pid, _) = para::para_stable_id_in(&info.id, p, i + 1);
        let mut index = 0usize;
        out.extend(smartart::collect_in_para(
            pkg, host_part, p, &pid, &mut index,
        )?);
    }
    Ok(out)
}

fn find_txbx_content_for<'a>(
    host: &'a Element,
    host_para_id: &str,
    want_id: &str,
) -> Result<Option<&'a Element>, Error> {
    let mut index = 0usize;
    find_txbx_content_walk(host, host_para_id, want_id, &mut index)
}

fn find_txbx_content_walk<'a>(
    el: &'a Element,
    host_para_id: &str,
    want_id: &str,
    index: &mut usize,
) -> Result<Option<&'a Element>, Error> {
    Ok(crate::xml::search(el, |el| {
        if el.local_name() == "txbxContent" {
            return crate::xml::Search::Skip;
        }
        if el.local_name() == "AlternateContent" {
            if let Some(i) = first_alt_child_index(el) {
                if let Node::Element(child) = &el.children[i] {
                    return crate::xml::Search::Only(child);
                }
            }
            return crate::xml::Search::Skip;
        }
        if el.local_name() == "drawing" {
            if is_group(el) {
                if let Some(found) = find_group_content(el, host_para_id, want_id, index) {
                    return crate::xml::Search::Hit(found);
                }
                return crate::xml::Search::Skip;
            }
            if is_textbox(el) {
                *index += 1;
                let (doc_pr_id, _) = doc_pr(el);
                let (id, _) = box_id(host_para_id, &doc_pr_id, *index);
                if id == want_id {
                    if let Some(content) = find_txbx_content(el) {
                        return crate::xml::Search::Hit(content);
                    }
                }
            }
            return crate::xml::Search::Skip;
        }
        crate::xml::Search::down(el)
    }))
}

fn find_group_content<'a>(
    el: &'a Element,
    host_para_id: &str,
    want_id: &str,
    index: &mut usize,
) -> Option<&'a Element> {
    crate::xml::search(el, |el| {
        if el.local_name() == "txbxContent" {
            return crate::xml::Search::Skip;
        }
        if el.local_name() == "wsp" && has_txbx(el) {
            *index += 1;
            let (doc_pr_id, _) = cnv_pr(el);
            let (id, _) = box_id(host_para_id, &doc_pr_id, *index);
            if id == want_id {
                if let Some(content) = find_txbx_content(el) {
                    return crate::xml::Search::Hit(content);
                }
            }
            return crate::xml::Search::Skip;
        }
        crate::xml::Search::down(el)
    })
}

fn is_textbox(drawing: &Element) -> bool {
    has_txbx(drawing)
}

pub fn is_group(drawing: &Element) -> bool {
    let mut group = false;
    walk_shape(drawing, &mut |e| {
        if e.local_name() == "wgp" {
            group = true;
        }
    });
    group
}

fn has_txbx(el: &Element) -> bool {
    let mut found = false;
    walk_all(el, &mut |e| {
        if e.local_name() == "txbxContent"
            || e.local_name() == "txbx"
            || e.local_name() == "linkedTxbx"
        {
            found = true;
        }
    });
    found
}

fn find_txbx_content(el: &Element) -> Option<&Element> {
    crate::xml::find_by_local(el, "txbxContent")
}

fn story_of(el: &Element) -> (Option<String>, u32) {
    let mut story = None;
    let mut seq = 0u32;
    walk_all(el, &mut |e| match e.local_name() {
        "txbx" => {
            if story.is_none() {
                story = e.attr_local("id").map(str::to_string);
            }
        }
        "linkedTxbx" => {
            if story.is_none() {
                story = e.attr_local("id").map(str::to_string);
            }
            if let Some(s) = e.attr_local("seq") {
                if let Ok(n) = s.parse::<u32>() {
                    seq = n;
                }
            }
        }
        _ => {}
    });
    (story, seq)
}

fn doc_pr(drawing: &Element) -> (Option<String>, Option<String>) {
    let frame = drawing
        .element_children()
        .find(|e| e.local_name() == "inline" || e.local_name() == "anchor");
    let pr = frame.and_then(|f| f.child_by_local("docPr"));
    (
        pr.and_then(|p| p.attr_local("id")).map(str::to_string),
        pr.and_then(|p| p.attr_local("name")).map(str::to_string),
    )
}

fn cnv_pr(wsp: &Element) -> (Option<String>, Option<String>) {
    let mut id = None;
    let mut name = None;
    walk_all(wsp, &mut |e| {
        if e.local_name() == "cNvPr" && id.is_none() {
            id = e.attr_local("id").map(str::to_string);
            name = e.attr_local("name").map(str::to_string);
        }
    });
    (id, name)
}

fn drawing_kind(drawing: &Element) -> PicKind {
    if drawing
        .element_children()
        .any(|e| e.local_name() == "anchor")
    {
        PicKind::Anchor
    } else {
        PicKind::Inline
    }
}

fn drawing_wrap(drawing: &Element, kind: PicKind) -> Option<WrapKind> {
    if kind != PicKind::Anchor {
        return None;
    }
    let frame = drawing
        .element_children()
        .find(|e| e.local_name() == "anchor")?;
    frame
        .element_children()
        .find_map(|c| WrapKind::from_local(c.local_name()))
}

fn drawing_extent(drawing: &Element) -> Result<(Option<i64>, Option<i64>), Error> {
    let frame = drawing
        .element_children()
        .find(|e| e.local_name() == "inline" || e.local_name() == "anchor");
    let extent = frame.and_then(|f| f.child_by_local("extent"));
    let cx = parse_emu(extent, "cx")?;
    let cy = parse_emu(extent, "cy")?;
    Ok((cx, cy))
}

fn parse_emu(el: Option<&Element>, local: &str) -> Result<Option<i64>, Error> {
    let Some(el) = el else {
        return Ok(None);
    };
    let Some(raw) = el.attr_local(local) else {
        return Ok(None);
    };
    raw.parse::<i64>().map(Some).map_err(|_| {
        crate::error::Error::xml_detail(
            crate::error::XmlIssue::AttrRead,
            format!("wp:extent @{local}={raw}"),
        )
    })
}

fn walk_host(
    root: &Element,
    f: &mut impl FnMut(&Element) -> Result<(), Error>,
) -> Result<(), Error> {
    let mut stack = vec![root];
    while let Some(el) = stack.pop() {
        if el.local_name() == "txbxContent" {
            continue;
        }
        if el.local_name() == "AlternateContent" {
            if let Some(i) = first_alt_child_index(el) {
                if let Node::Element(child) = &el.children[i] {
                    stack.push(child);
                }
            }
            continue;
        }
        if el.local_name() == "drawing" {
            f(el)?;
            continue;
        }
        for node in el.children.iter().rev() {
            if let Node::Element(child) = node {
                stack.push(child);
            }
        }
    }
    Ok(())
}

fn walk_all(el: &Element, f: &mut impl FnMut(&Element)) {
    crate::xml::walk_elements(el, f);
}

/// 只走图形结构，不进框内段落，避免把框里嵌套的图误当成外层组合。
fn walk_shape(el: &Element, f: &mut impl FnMut(&Element)) {
    crate::xml::walk_elements_if(el, |node| {
        if node.local_name() == "txbxContent" {
            return false;
        }
        f(node);
        true
    });
}

fn first_alt_child_index(el: &Element) -> Option<usize> {
    crate::xml::alternate_child(el).unwrap_or(None)
}

pub const DEFAULT_CX: i64 = 1_828_800;
pub const DEFAULT_CY: i64 = 914_400;

/// 从宿主段落走到该框的 `w:txbxContent`。链接后继框常常没有内容节点。
pub fn content_path(host: &Element, key: &TxbxKey) -> Option<Vec<usize>> {
    let mut path = wsp_path(host, key)?;
    let wsp = para::element_at(host, &path)?;
    let extra = txbx_content_rel(wsp)?;
    path.extend(extra);
    Some(path)
}

fn txbx_content_rel(el: &Element) -> Option<Vec<usize>> {
    let mut path = Vec::new();
    if txbx_content_rel_walk(el, &mut path) {
        Some(path)
    } else {
        None
    }
}

fn txbx_content_rel_walk(el: &Element, path: &mut Vec<usize>) -> bool {
    for (i, n) in el.children.iter().enumerate() {
        if let Node::Element(e) = n {
            path.push(i);
            if e.local_name() == "txbxContent" {
                return true;
            }
            if txbx_content_rel_walk(e, path) {
                return true;
            }
            path.pop();
        }
    }
    false
}

pub fn find_txbx_content_by_key<'a>(host: &'a Element, key: &TxbxKey) -> Option<&'a Element> {
    let mut index = 0usize;
    find_content_by_key_walk(host, key, &mut index)
}

/// 框是否存在。Word 链接后继框常只有 `wps:linkedTxbx`、没有 `w:txbxContent`。
pub fn textbox_exists(host: &Element, key: &TxbxKey) -> bool {
    wsp_path(host, key).is_some()
}

pub fn find_txbx_content_by_key_mut<'a>(
    host: &'a mut Element,
    key: &TxbxKey,
) -> Option<&'a mut Element> {
    let mut index = 0usize;
    find_content_by_key_walk_mut(host, key, &mut index)
}

pub fn find_textbox_drawing_mut<'a>(
    host: &'a mut Element,
    key: &TxbxKey,
) -> Option<&'a mut Element> {
    let mut index = 0usize;
    find_drawing_by_key_mut(host, key, &mut index)
}

pub fn find_textbox_drawing<'a>(host: &'a Element, key: &TxbxKey) -> Option<&'a Element> {
    let mut index = 0usize;
    find_drawing_by_key(host, key, &mut index)
}

pub fn with_wsp_mut<T>(
    host: &mut Element,
    key: &TxbxKey,
    f: impl FnOnce(&mut Element) -> T,
) -> Option<T> {
    let path = wsp_path(host, key)?;
    let wsp = follow_path_mut(host, &path)?;
    Some(f(wsp))
}

pub fn ensure_textbox_ns(root: &mut Element) {
    picture::ensure_drawing_ns(root);
    ensure_xmlns(root, "mc", MC_NS);
    ensure_xmlns(root, "wps", WPS_NS);
    ensure_xmlns(root, "w14", W14_NS);
    ensure_xmlns(root, "wp", WP_NS);
    ensure_xmlns(root, "wpg", WPG_NS);
}

pub fn insert_textbox(p: &mut Element, doc_pr: u32, cx: i64, cy: i64, text: &str, para_id: &str) {
    insert_ac(p, build_textbox_ac(doc_pr, cx, cy, text, para_id));
}

pub fn insert_ac(p: &mut Element, ac: Element) {
    let mut run = Element::new(p.prefixed("r"));
    run.push_element(ac);
    p.children.push(Node::Element(run));
}

pub fn remove_textbox(host: &mut Element, key: &TxbxKey) -> bool {
    let mut index = 0usize;
    remove_textbox_walk(host, key, &mut index)
}

pub fn set_fill(wsp: &mut Element, color: Option<&str>) -> Result<(), Error> {
    let sp = ensure_sp_pr(wsp);
    remove_named(sp, &["solidFill", "noFill", "gradFill", "pattFill"]);
    let fill = match color {
        Some(c) => {
            check_hex(c)?;
            let mut fill = Element::new("a:solidFill");
            fill.push_element(Element::new("a:srgbClr").with_attr("val", c.to_ascii_uppercase()));
            fill
        }
        None => Element::new("a:noFill"),
    };
    // DrawingML 里填充必须在 `ln` 之前，否则 Word 对无框形状会忽略填色。
    let at = sp
        .children
        .iter()
        .position(|n| matches!(n, Node::Element(e) if e.local_name() == "ln"))
        .unwrap_or(sp.children.len());
    sp.children.insert(at, Node::Element(fill));
    Ok(())
}

pub fn set_line(wsp: &mut Element, color: Option<&str>, width: Option<i64>) -> Result<(), Error> {
    let sp = ensure_sp_pr(wsp);
    let existing = sp
        .children
        .iter()
        .position(|n| matches!(n, Node::Element(e) if e.local_name() == "ln"));
    if let Some(at) = existing {
        sp.children.remove(at);
    }
    let mut ln = Element::new("a:ln");
    if let Some(w) = width {
        ln.set_attr("w", w.to_string());
    }
    match color {
        Some(c) => {
            check_hex(c)?;
            let mut fill = Element::new("a:solidFill");
            fill.push_element(Element::new("a:srgbClr").with_attr("val", c.to_ascii_uppercase()));
            ln.push_element(fill);
        }
        None => ln.push_element(Element::new("a:noFill")),
    }
    sp.push_element(ln);
    Ok(())
}

pub fn set_direction(wsp: &mut Element, vertical: bool) {
    let body = ensure_body_pr(wsp);
    if vertical {
        set_attr_local(body, "vert", "eaVert");
    } else {
        remove_attr_local(body, "vert");
    }
}

pub fn set_text_anchor(wsp: &mut Element, value: Option<&str>) {
    let body = ensure_body_pr(wsp);
    match value {
        Some(v) => set_attr_local(body, "anchor", v),
        None => remove_attr_local(body, "anchor"),
    }
}

pub fn story_id_at(root: &Element, t: &TxbxTarget) -> Option<String> {
    story_id_of_target(root, t)
}

pub fn apply_link(
    root: &mut Element,
    from: &TxbxTarget,
    next: Option<&TxbxTarget>,
) -> Result<(), Error> {
    if let Some(b) = next {
        let story =
            story_id_of_target(root, from).unwrap_or_else(|| next_story_id(root).to_string());
        let seq_a = seq_of_target(root, from).unwrap_or(0);
        unlink_after(root, from)?;
        attach_chain(root, b, &story, seq_a + 1)?;
        let first = seq_a == 0 || is_first_of_story(root, from, &story);
        write_link_on(root, from, &story, seq_a, first)?;
    } else {
        unlink_after(root, from)?;
    }
    Ok(())
}

pub fn resequence_story(root: &mut Element, story_id: &str) {
    let mut hits = collect_story_hits(root, story_id);
    if hits.is_empty() {
        return;
    }
    hits.sort_by_key(|h| h.seq);
    for (i, hit) in hits.iter().enumerate() {
        let first = i == 0;
        let seq = if first { 0 } else { i as u32 };
        if let Some(p) = para::find_paragraph_mut(root, &hit.para) {
            let _ = with_wsp_mut(p, &hit.key, |wsp| {
                set_link_elem(wsp, story_id, seq, first);
            });
        }
    }
}

fn find_content_by_key_walk<'a>(
    el: &'a Element,
    key: &TxbxKey,
    index: &mut usize,
) -> Option<&'a Element> {
    crate::xml::search(el, |el| {
        if el.local_name() == "txbxContent" {
            return crate::xml::Search::Skip;
        }
        if el.local_name() == "AlternateContent" {
            if let Some(i) = first_alt_child_index(el) {
                if let Node::Element(child) = &el.children[i] {
                    return crate::xml::Search::Only(child);
                }
            }
            return crate::xml::Search::Skip;
        }
        if el.local_name() == "drawing" {
            if is_group(el) {
                if let Some(found) = find_group_content_by_key(el, key, index) {
                    return crate::xml::Search::Hit(found);
                }
                return crate::xml::Search::Skip;
            }
            if is_textbox(el) {
                *index += 1;
                let (doc_pr_id, _) = doc_pr(el);
                if para::matches_txbx_key(&doc_pr_id, *index, key) {
                    if let Some(content) = find_txbx_content(el) {
                        return crate::xml::Search::Hit(content);
                    }
                }
            }
            return crate::xml::Search::Skip;
        }
        crate::xml::Search::down(el)
    })
}

fn find_group_content_by_key<'a>(
    el: &'a Element,
    key: &TxbxKey,
    index: &mut usize,
) -> Option<&'a Element> {
    crate::xml::search(el, |el| {
        if el.local_name() == "txbxContent" {
            return crate::xml::Search::Skip;
        }
        if el.local_name() == "wsp" && has_txbx(el) {
            *index += 1;
            let (doc_pr_id, _) = cnv_pr(el);
            if para::matches_txbx_key(&doc_pr_id, *index, key) {
                if let Some(content) = find_txbx_content(el) {
                    return crate::xml::Search::Hit(content);
                }
            }
            return crate::xml::Search::Skip;
        }
        crate::xml::Search::down(el)
    })
}

fn find_content_by_key_walk_mut<'a>(
    el: &'a mut Element,
    key: &TxbxKey,
    index: &mut usize,
) -> Option<&'a mut Element> {
    let mut stack: Vec<Vec<usize>> = vec![Vec::new()];
    while let Some(path) = stack.pop() {
        let kind = {
            let node = crate::xml::element_at_ref(el, &path);
            node.local_name().to_string()
        };
        if kind == "txbxContent" {
            continue;
        }
        if kind == "AlternateContent" {
            let next = {
                let node = crate::xml::element_at_ref(el, &path);
                first_alt_child_index(node)
                    .filter(|i| matches!(&node.children[*i], Node::Element(_)))
            };
            if let Some(i) = next {
                let mut child = path;
                child.push(i);
                stack.push(child);
            }
            continue;
        }
        if kind == "drawing" {
            let (group, textbox, doc_pr_id) = {
                let node = crate::xml::element_at_ref(el, &path);
                (is_group(node), is_textbox(node), doc_pr(node).0)
            };
            if group {
                if let Some(rel) =
                    group_content_rel_path(crate::xml::element_at_ref(el, &path), key, index)
                {
                    let mut full = path;
                    full.extend(rel);
                    return Some(crate::xml::element_at_mut(el, &full));
                }
                continue;
            }
            if textbox {
                *index += 1;
                if para::matches_txbx_key(&doc_pr_id, *index, key) {
                    if let Some(rel) =
                        rel_path_to_local(crate::xml::element_at_ref(el, &path), "txbxContent")
                    {
                        let mut full = path;
                        full.extend(rel);
                        return Some(crate::xml::element_at_mut(el, &full));
                    }
                }
            }
            continue;
        }
        let indexes = crate::xml::element_indexes(crate::xml::element_at_ref(el, &path));
        for i in indexes.into_iter().rev() {
            let mut child = path.clone();
            child.push(i);
            stack.push(child);
        }
    }
    None
}

fn group_content_rel_path(el: &Element, key: &TxbxKey, index: &mut usize) -> Option<Vec<usize>> {
    let mut stack: Vec<Vec<usize>> = vec![Vec::new()];
    while let Some(path) = stack.pop() {
        let node = crate::xml::element_at_ref(el, &path);
        if node.local_name() == "txbxContent" {
            continue;
        }
        if node.local_name() == "wsp" && has_txbx(node) {
            *index += 1;
            let (doc_pr_id, _) = cnv_pr(node);
            if para::matches_txbx_key(&doc_pr_id, *index, key) {
                if let Some(rel) = rel_path_to_local(node, "txbxContent") {
                    let mut full = path;
                    full.extend(rel);
                    return Some(full);
                }
            }
            continue;
        }
        let indexes = crate::xml::element_indexes(node);
        for i in indexes.into_iter().rev() {
            let mut child = path.clone();
            child.push(i);
            stack.push(child);
        }
    }
    None
}

fn rel_path_to_local(root: &Element, local: &str) -> Option<Vec<usize>> {
    if root.local_name() == local {
        return Some(Vec::new());
    }
    let mut stack: Vec<Vec<usize>> = Vec::new();
    for i in crate::xml::element_indexes(root).into_iter().rev() {
        stack.push(vec![i]);
    }
    while let Some(path) = stack.pop() {
        let node = crate::xml::element_at_ref(root, &path);
        if node.local_name() == local {
            return Some(path);
        }
        let indexes = crate::xml::element_indexes(node);
        for i in indexes.into_iter().rev() {
            let mut child = path.clone();
            child.push(i);
            stack.push(child);
        }
    }
    None
}

fn find_drawing_by_key<'a>(
    el: &'a Element,
    key: &TxbxKey,
    index: &mut usize,
) -> Option<&'a Element> {
    crate::xml::search(el, |el| {
        if el.local_name() == "txbxContent" {
            return crate::xml::Search::Skip;
        }
        if el.local_name() == "AlternateContent" {
            if let Some(i) = first_alt_child_index(el) {
                if let Node::Element(child) = &el.children[i] {
                    return crate::xml::Search::Only(child);
                }
            }
            return crate::xml::Search::Skip;
        }
        if el.local_name() == "drawing" {
            if drawing_matches(el, key, index) {
                return crate::xml::Search::Hit(el);
            }
            return crate::xml::Search::Skip;
        }
        crate::xml::Search::down(el)
    })
}

fn find_drawing_by_key_mut<'a>(
    el: &'a mut Element,
    key: &TxbxKey,
    index: &mut usize,
) -> Option<&'a mut Element> {
    crate::xml::search_mut(el, |el| {
        if el.local_name() == "txbxContent" {
            return crate::xml::SearchMut::Skip;
        }
        if el.local_name() == "AlternateContent" {
            if let Some(i) = first_alt_child_index(el) {
                if matches!(&el.children[i], Node::Element(_)) {
                    return crate::xml::SearchMut::Only(i);
                }
            }
            return crate::xml::SearchMut::Skip;
        }
        if el.local_name() == "drawing" {
            if drawing_matches(el, key, index) {
                return crate::xml::SearchMut::Hit;
            }
            return crate::xml::SearchMut::Skip;
        }
        crate::xml::SearchMut::down(el)
    })
}

fn drawing_matches(drawing: &Element, key: &TxbxKey, index: &mut usize) -> bool {
    if is_group(drawing) {
        let mut hit = false;
        walk_shape(drawing, &mut |e| {
            if e.local_name() == "wsp" && has_txbx(e) {
                *index += 1;
                let (doc_pr_id, _) = cnv_pr(e);
                if para::matches_txbx_key(&doc_pr_id, *index, key) {
                    hit = true;
                }
            }
        });
        return hit;
    }
    if is_textbox(drawing) {
        *index += 1;
        let (doc_pr_id, _) = doc_pr(drawing);
        return para::matches_txbx_key(&doc_pr_id, *index, key);
    }
    false
}

fn wsp_path(host: &Element, key: &TxbxKey) -> Option<Vec<usize>> {
    let mut index = 0usize;
    let mut path = Vec::new();
    if wsp_path_walk(host, key, &mut index, &mut path) {
        Some(path)
    } else {
        None
    }
}

fn wsp_path_walk(el: &Element, key: &TxbxKey, index: &mut usize, path: &mut Vec<usize>) -> bool {
    if el.local_name() == "txbxContent" {
        return false;
    }
    if el.local_name() == "AlternateContent" {
        if let Some(i) = first_alt_child_index(el) {
            path.push(i);
            if let Node::Element(child) = &el.children[i] {
                if wsp_path_walk(child, key, index, path) {
                    return true;
                }
            }
            path.pop();
        }
        return false;
    }
    if el.local_name() == "drawing" {
        if is_group(el) {
            return wsp_path_in_group(el, key, index, path);
        }
        if is_textbox(el) {
            *index += 1;
            let (doc_pr_id, _) = doc_pr(el);
            if para::matches_txbx_key(&doc_pr_id, *index, key) {
                return wsp_path_first_wsp(el, path);
            }
        }
        return false;
    }
    for (i, n) in el.children.iter().enumerate() {
        if let Node::Element(e) = n {
            path.push(i);
            if wsp_path_walk(e, key, index, path) {
                return true;
            }
            path.pop();
        }
    }
    false
}

fn wsp_path_in_group(
    el: &Element,
    key: &TxbxKey,
    index: &mut usize,
    path: &mut Vec<usize>,
) -> bool {
    if el.local_name() == "txbxContent" {
        return false;
    }
    if el.local_name() == "wsp" && has_txbx(el) {
        *index += 1;
        let (doc_pr_id, _) = cnv_pr(el);
        return para::matches_txbx_key(&doc_pr_id, *index, key);
    }
    for (i, n) in el.children.iter().enumerate() {
        if let Node::Element(e) = n {
            path.push(i);
            if wsp_path_in_group(e, key, index, path) {
                return true;
            }
            path.pop();
        }
    }
    false
}

fn wsp_path_first_wsp(el: &Element, path: &mut Vec<usize>) -> bool {
    if el.local_name() == "txbxContent" {
        return false;
    }
    if el.local_name() == "wsp" {
        return true;
    }
    for (i, n) in el.children.iter().enumerate() {
        if let Node::Element(e) = n {
            path.push(i);
            if wsp_path_first_wsp(e, path) {
                return true;
            }
            path.pop();
        }
    }
    false
}

fn follow_path_mut<'a>(el: &'a mut Element, path: &[usize]) -> Option<&'a mut Element> {
    let mut cur = el;
    for &i in path {
        cur = match cur.children.get_mut(i) {
            Some(Node::Element(e)) => e,
            _ => return None,
        };
    }
    Some(cur)
}

fn follow_path<'a>(el: &'a Element, path: &[usize]) -> Option<&'a Element> {
    let mut cur = el;
    for &i in path {
        cur = match cur.children.get(i) {
            Some(Node::Element(e)) => e,
            _ => return None,
        };
    }
    Some(cur)
}

fn text_anchor_json(host: &Element, b: &TxbxInfo) -> Result<Value, Error> {
    let Some(key) = key_of_info(b) else {
        return Ok(Value::Null);
    };
    let Some(path) = wsp_path(host, &key) else {
        return Ok(Value::Null);
    };
    let Some(wsp) = follow_path(host, &path) else {
        return Ok(Value::Null);
    };
    let Some(body) = wsp.child_by_local("bodyPr") else {
        return Ok(Value::Null);
    };
    match body.attr_local("anchor") {
        None => Ok(Value::Null),
        Some(v) if TEXT_BOX_TEXT_ANCHORS.contains(&v) => Ok(Value::String(v.to_string())),
        Some(raw) => Err(Error::xml_detail(
            XmlIssue::Parse,
            format!("bodyPr/@anchor not recognized: {raw}"),
        )),
    }
}

fn remove_textbox_walk(el: &mut Element, key: &TxbxKey, index: &mut usize) -> bool {
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
            if ac_contains_group(&el.children[i]) {
                if let Node::Element(ac) = &mut el.children[i] {
                    if let Some(idx) = first_alt_child_index(ac) {
                        if let Node::Element(branch) = &mut ac.children[idx] {
                            if remove_textbox_walk(branch, key, index) {
                                let empty = !group_has_member(ac);
                                if empty {
                                    el.children.remove(i);
                                }
                                return true;
                            }
                        }
                    }
                }
                i += 1;
                continue;
            }
            if ac_has_box(&el.children[i], key, index) {
                el.children.remove(i);
                return true;
            }
            i += 1;
            continue;
        }
        if local == "drawing" {
            let hit = match &el.children[i] {
                Node::Element(d) => {
                    if is_group(d) {
                        false
                    } else if is_textbox(d) {
                        *index += 1;
                        let (doc_pr_id, _) = doc_pr(d);
                        para::matches_txbx_key(&doc_pr_id, *index, key)
                    } else {
                        false
                    }
                }
                Node::Text(_) => false,
            };
            if hit {
                el.children.remove(i);
                return true;
            }
            if let Node::Element(d) = &mut el.children[i] {
                if is_group(d) && remove_group_wsp(d, key, index) {
                    if !group_has_member(d) {
                        el.children.remove(i);
                    }
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
            if remove_textbox_walk(child, key, index) {
                if is_empty_run(child) {
                    el.children.remove(i);
                }
                return true;
            }
        }
        i += 1;
    }
    false
}

fn ac_contains_group(node: &Node) -> bool {
    let Node::Element(ac) = node else {
        return false;
    };
    let Some(i) = first_alt_child_index(ac) else {
        return false;
    };
    let Node::Element(branch) = &ac.children[i] else {
        return false;
    };
    let mut found = false;
    walk_shape(branch, &mut |e| {
        if e.local_name() == "wgp" {
            found = true;
        }
    });
    found
}

fn ac_has_box(node: &Node, key: &TxbxKey, index: &mut usize) -> bool {
    let Node::Element(ac) = node else {
        return false;
    };
    let Some(i) = first_alt_child_index(ac) else {
        return false;
    };
    let Node::Element(branch) = &ac.children[i] else {
        return false;
    };
    drawing_matches_in(branch, key, index)
}

fn drawing_matches_in(el: &Element, key: &TxbxKey, index: &mut usize) -> bool {
    if el.local_name() == "drawing" {
        return drawing_matches(el, key, index);
    }
    for n in &el.children {
        if let Node::Element(e) = n {
            if drawing_matches_in(e, key, index) {
                return true;
            }
        }
    }
    false
}

fn remove_group_wsp(el: &mut Element, key: &TxbxKey, index: &mut usize) -> bool {
    if el.local_name() == "txbxContent" {
        return false;
    }
    let mut i = 0;
    while i < el.children.len() {
        let is_wsp =
            matches!(&el.children[i], Node::Element(c) if c.local_name() == "wsp" && has_txbx(c));
        if is_wsp {
            *index += 1;
            let hit = match &el.children[i] {
                Node::Element(c) => {
                    let (doc_pr_id, _) = cnv_pr(c);
                    para::matches_txbx_key(&doc_pr_id, *index, key)
                }
                Node::Text(_) => false,
            };
            if hit {
                el.children.remove(i);
                return true;
            }
        }
        if let Node::Element(child) = &mut el.children[i] {
            if remove_group_wsp(child, key, index) {
                return true;
            }
        }
        i += 1;
    }
    false
}

pub fn group_has_member(el: &Element) -> bool {
    let mut found = false;
    walk_shape(el, &mut |e| {
        if e.local_name() == "pic" {
            found = true;
        }
        if e.local_name() == "blip" {
            found = true;
        }
        if e.local_name() == "wsp" && has_txbx(e) {
            found = true;
        }
        if e.local_name() == "wsp" && is_connector_wsp_local(e) {
            found = true;
        }
    });
    found
}

fn is_connector_wsp_local(wsp: &Element) -> bool {
    if wsp.local_name() != "wsp" || has_txbx(wsp) {
        return false;
    }
    let mut cn = false;
    let mut prst = None;
    walk_all(wsp, &mut |e| {
        if e.local_name() == "cNvCnPr" {
            cn = true;
        }
        if e.local_name() == "prstGeom" && prst.is_none() {
            prst = e.attr_local("prst").map(str::to_string);
        }
    });
    cn || prst.as_deref().is_some_and(|p| {
        matches!(
            p,
            "line"
                | "straightConnector1"
                | "bentConnector2"
                | "bentConnector3"
                | "bentConnector4"
                | "bentConnector5"
                | "curvedConnector2"
                | "curvedConnector3"
                | "curvedConnector4"
                | "curvedConnector5"
        )
    })
}

fn is_empty_run(el: &Element) -> bool {
    el.local_name() == "r" && el.element_children().all(|c| c.local_name() == "rPr")
}

pub fn textbox_ac_xml(doc_pr: u32, cx: i64, cy: i64, text: &str, para_id: &str) -> String {
    let run = if text.is_empty() {
        String::new()
    } else {
        let space = if needs_xml_space(text) {
            r#" xml:space="preserve""#
        } else {
            ""
        };
        format!(
            r#"<w:r><w:t{space}>{}</w:t></w:r>"#,
            crate::xml::escape_text(text)
        )
    };
    format!(
        r#"<mc:AlternateContent><mc:Choice Requires="wps"><w:drawing><wp:anchor distT="0" distB="0" distL="114300" distR="114300" simplePos="0" relativeHeight="251659264" behindDoc="0" locked="0" layoutInCell="1" allowOverlap="1"><wp:simplePos x="0" y="0"/><wp:positionH relativeFrom="margin"><wp:align>left</wp:align></wp:positionH><wp:positionV relativeFrom="margin"><wp:align>top</wp:align></wp:positionV><wp:extent cx="{cx}" cy="{cy}"/><wp:effectExtent l="0" t="0" r="0" b="0"/><wp:wrapSquare wrapText="bothSides"/><wp:docPr id="{doc_pr}" name="文本框 {doc_pr}"/><wp:cNvGraphicFramePr><a:graphicFrameLocks noChangeAspect="1"/></wp:cNvGraphicFramePr><a:graphic><a:graphicData uri="{WPS_NS}"><wps:wsp><wps:cNvSpPr txBox="1"/><wps:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="{cx}" cy="{cy}"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:solidFill><a:srgbClr val="FFFFFF"/></a:solidFill><a:ln w="6350"><a:solidFill><a:srgbClr val="000000"/></a:solidFill></a:ln></wps:spPr><wps:txbx><w:txbxContent><w:p w14:paraId="{para_id}" w14:textId="{para_id}">{run}</w:p></w:txbxContent></wps:txbx><wps:bodyPr wrap="square" lIns="91440" tIns="45720" rIns="91440" bIns="45720"><a:noAutofit/></wps:bodyPr></wps:wsp></a:graphicData></a:graphic></wp:anchor></w:drawing></mc:Choice><mc:Fallback><w:pict/></mc:Fallback></mc:AlternateContent>"#
    )
}

fn build_textbox_ac(doc_pr: u32, cx: i64, cy: i64, text: &str, para_id: &str) -> Element {
    let mut ac = Element::new("mc:AlternateContent");
    let mut choice = Element::new("mc:Choice").with_attr("Requires", "wps");
    let mut drawing = Element::new("w:drawing");
    let mut frame = Element::new("wp:anchor")
        .with_attr("distT", "0")
        .with_attr("distB", "0")
        .with_attr("distL", "114300")
        .with_attr("distR", "114300")
        .with_attr("simplePos", "0")
        .with_attr("relativeHeight", "251659264")
        .with_attr("behindDoc", "0")
        .with_attr("locked", "0")
        .with_attr("layoutInCell", "1")
        .with_attr("allowOverlap", "1");
    frame.push_element(
        Element::new("wp:simplePos")
            .with_attr("x", "0")
            .with_attr("y", "0"),
    );
    let mut pos_h = Element::new("wp:positionH").with_attr("relativeFrom", "margin");
    pos_h.push_element(Element::new("wp:align").with_text("left"));
    let mut pos_v = Element::new("wp:positionV").with_attr("relativeFrom", "margin");
    pos_v.push_element(Element::new("wp:align").with_text("top"));
    frame.push_element(pos_h);
    frame.push_element(pos_v);
    frame.push_element(
        Element::new("wp:extent")
            .with_attr("cx", cx.to_string())
            .with_attr("cy", cy.to_string()),
    );
    frame.push_element(
        Element::new("wp:effectExtent")
            .with_attr("l", "0")
            .with_attr("t", "0")
            .with_attr("r", "0")
            .with_attr("b", "0"),
    );
    frame.push_element(Element::new("wp:wrapSquare").with_attr("wrapText", "bothSides"));
    frame.push_element(
        Element::new("wp:docPr")
            .with_attr("id", doc_pr.to_string())
            .with_attr("name", format!("文本框 {doc_pr}")),
    );
    let mut frame_pr = Element::new("wp:cNvGraphicFramePr");
    frame_pr.push_element(Element::new("a:graphicFrameLocks").with_attr("noChangeAspect", "1"));
    frame.push_element(frame_pr);
    let mut graphic = Element::new("a:graphic");
    let mut data = Element::new("a:graphicData").with_attr("uri", WPS_NS);
    data.push_element(build_textbox_wsp(cx, cy, text, para_id));
    graphic.push_element(data);
    frame.push_element(graphic);
    drawing.push_element(frame);
    choice.push_element(drawing);
    ac.push_element(choice);
    let mut fallback = Element::new("mc:Fallback");
    fallback.push_element(Element::new("w:pict"));
    ac.push_element(fallback);
    ac
}

fn build_textbox_wsp(cx: i64, cy: i64, text: &str, para_id: &str) -> Element {
    let mut wsp = Element::new("wps:wsp");
    wsp.push_element(Element::new("wps:cNvSpPr").with_attr("txBox", "1"));
    let mut sp = Element::new("wps:spPr");
    let mut xfrm = Element::new("a:xfrm");
    xfrm.push_element(
        Element::new("a:off")
            .with_attr("x", "0")
            .with_attr("y", "0"),
    );
    xfrm.push_element(
        Element::new("a:ext")
            .with_attr("cx", cx.to_string())
            .with_attr("cy", cy.to_string()),
    );
    sp.push_element(xfrm);
    let mut geom = Element::new("a:prstGeom").with_attr("prst", "rect");
    geom.push_element(Element::new("a:avLst"));
    sp.push_element(geom);
    let mut fill = Element::new("a:solidFill");
    fill.push_element(Element::new("a:srgbClr").with_attr("val", "FFFFFF"));
    sp.push_element(fill);
    let mut ln = Element::new("a:ln").with_attr("w", "6350");
    let mut ln_fill = Element::new("a:solidFill");
    ln_fill.push_element(Element::new("a:srgbClr").with_attr("val", "000000"));
    ln.push_element(ln_fill);
    sp.push_element(ln);
    wsp.push_element(sp);
    let mut txbx = Element::new("wps:txbx");
    let mut content = Element::new("w:txbxContent");
    let mut inner = Element::new("w:p")
        .with_attr("w14:paraId", para_id)
        .with_attr("w14:textId", para_id);
    if !text.is_empty() {
        let mut t = Element::new("w:t");
        t.set_text(text);
        if needs_xml_space(text) {
            t.set_attr("xml:space", "preserve");
        }
        let mut r = Element::new("w:r");
        r.push_element(t);
        inner.push_element(r);
    }
    content.push_element(inner);
    txbx.push_element(content);
    wsp.push_element(txbx);
    let mut body_pr = Element::new("wps:bodyPr")
        .with_attr("wrap", "square")
        .with_attr("lIns", "91440")
        .with_attr("tIns", "45720")
        .with_attr("rIns", "91440")
        .with_attr("bIns", "45720");
    body_pr.push_element(Element::new("a:noAutofit"));
    wsp.push_element(body_pr);
    wsp
}

fn ensure_sp_pr(wsp: &mut Element) -> &mut Element {
    if wsp.child_by_local("spPr").is_none() {
        let mut at = 0;
        for (i, n) in wsp.children.iter().enumerate() {
            if let Node::Element(e) = n {
                if e.local_name() == "cNvSpPr" || e.local_name() == "cNvPr" {
                    at = i + 1;
                }
            }
        }
        wsp.children
            .insert(at, Node::Element(Element::new("wps:spPr")));
    }
    wsp.child_by_local_mut("spPr").unwrap()
}

fn ensure_body_pr(wsp: &mut Element) -> &mut Element {
    if wsp.child_by_local("bodyPr").is_none() {
        wsp.push_element(Element::new("wps:bodyPr"));
    }
    wsp.child_by_local_mut("bodyPr").unwrap()
}

fn remove_named(el: &mut Element, names: &[&str]) {
    el.children.retain(|n| match n {
        Node::Element(e) => !names.contains(&e.local_name()),
        Node::Text(_) => true,
    });
}

fn check_hex(s: &str) -> Result<(), Error> {
    if s.len() == 6 && s.chars().all(|c| c.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(Error::new(ErrorKind::InvalidHexColor))
    }
}

fn set_attr_local(el: &mut Element, local: &str, value: &str) {
    if let Some((k, _)) = el
        .attrs
        .iter()
        .find(|(k, _)| k.rsplit(':').next().unwrap_or(k) == local)
    {
        let k = k.clone();
        el.set_attr(&k, value);
        return;
    }
    el.set_attr(local, value);
}

fn remove_attr_local(el: &mut Element, local: &str) {
    el.attrs
        .retain(|(k, _)| k.rsplit(':').next().unwrap_or(k) != local);
}

fn ensure_xmlns(el: &mut Element, prefix: &str, uri: &str) {
    let key = format!("xmlns:{prefix}");
    if el.attr(&key).is_none() {
        el.set_attr(&key, uri);
    }
}

fn needs_xml_space(text: &str) -> bool {
    text.starts_with(' ')
        || text.starts_with('\t')
        || text.ends_with(' ')
        || text.ends_with('\t')
        || text.contains('\n')
}

fn set_link_elem(wsp: &mut Element, story_id: &str, seq: u32, first: bool) {
    let want = if first { "txbx" } else { "linkedTxbx" };
    if let Some(el) = find_link_elem_mut(wsp) {
        let prefix = el.name.split_once(':').map(|(p, _)| p).unwrap_or("wps");
        el.name = format!("{prefix}:{want}");
        el.remove_attr("id");
        el.remove_attr("wps:id");
        remove_attr_local(el, "id");
        remove_attr_local(el, "seq");
        el.set_attr("id", story_id);
        if !first {
            el.set_attr("seq", seq.to_string());
        }
    }
}

fn find_link_elem_mut(el: &mut Element) -> Option<&mut Element> {
    crate::xml::search_mut(el, |el| {
        if el.local_name() == "txbx" || el.local_name() == "linkedTxbx" {
            return crate::xml::SearchMut::Hit;
        }
        if el.local_name() == "txbxContent" {
            return crate::xml::SearchMut::Skip;
        }
        crate::xml::SearchMut::down(el)
    })
}

struct StoryHit {
    para: BodyPara,
    key: TxbxKey,
    seq: u32,
}

fn collect_story_hits(root: &Element, story_id: &str) -> Vec<StoryHit> {
    let paras = para::collect_paragraphs(root);
    let mut out = Vec::new();
    for (i, p) in paras.iter().enumerate() {
        let (host_id, _) = para::para_stable_id(p, i + 1);
        if let Ok(boxes) = collect_textboxes(p, &host_id) {
            for b in boxes {
                if b.story_id.as_deref() == Some(story_id) {
                    let key = match &b.doc_pr_id {
                        Some(d) => TxbxKey::DocPr(d.clone()),
                        None => {
                            if let Some(rest) = b.id.rsplit("/txbx:index:").next() {
                                if let Ok(n) = rest.parse() {
                                    TxbxKey::Index(n)
                                } else {
                                    continue;
                                }
                            } else {
                                continue;
                            }
                        }
                    };
                    out.push(StoryHit {
                        para: BodyPara::Index(i + 1),
                        key,
                        seq: b.seq,
                    });
                }
            }
        }
    }
    out
}

fn story_id_of_target(root: &Element, t: &TxbxTarget) -> Option<String> {
    info_of_target(root, t).and_then(|b| b.story_id)
}

fn seq_of_target(root: &Element, t: &TxbxTarget) -> Option<u32> {
    info_of_target(root, t).map(|b| b.seq)
}

fn info_of_target(root: &Element, t: &TxbxTarget) -> Option<TxbxInfo> {
    let p = para::find_paragraph(root, t.host.body())?;
    let boxes = collect_textboxes(p, "host").ok()?;
    boxes
        .into_iter()
        .find(|b| matches_info(&b.doc_pr_id, &b.id, &t.key))
}

fn key_of_info(b: &TxbxInfo) -> Option<TxbxKey> {
    match &b.doc_pr_id {
        Some(d) => Some(TxbxKey::DocPr(d.clone())),
        None => {
            b.id.rsplit("/txbx:index:")
                .next()
                .and_then(|s| s.parse().ok())
                .map(TxbxKey::Index)
        }
    }
}

fn matches_info(doc_pr_id: &Option<String>, id: &str, key: &TxbxKey) -> bool {
    match key {
        TxbxKey::DocPr(d) => doc_pr_id.as_deref() == Some(d.as_str()),
        TxbxKey::Index(n) => doc_pr_id.is_none() && id.ends_with(&format!("/txbx:index:{n}")),
    }
}

fn next_story_id(root: &Element) -> u32 {
    let mut max = 0u32;
    walk_all(root, &mut |e| {
        if e.local_name() == "txbx" || e.local_name() == "linkedTxbx" {
            if let Some(id) = e.attr_local("id") {
                if let Ok(n) = id.parse::<u32>() {
                    max = max.max(n);
                }
            }
        }
    });
    max + 1
}

fn is_first_of_story(root: &Element, t: &TxbxTarget, story: &str) -> bool {
    let mut hits = collect_story_hits(root, story);
    hits.sort_by_key(|h| h.seq);
    hits.first()
        .is_some_and(|h| matches_info_key(&h.key, &t.key) && same_body(&h.para, t.host.body()))
}

fn matches_info_key(a: &TxbxKey, b: &TxbxKey) -> bool {
    match (a, b) {
        (TxbxKey::DocPr(x), TxbxKey::DocPr(y)) => x == y,
        (TxbxKey::Index(x), TxbxKey::Index(y)) => x == y,
        _ => false,
    }
}

fn same_body(a: &BodyPara, b: &BodyPara) -> bool {
    match (a, b) {
        (BodyPara::Index(x), BodyPara::Index(y)) => x == y,
        (BodyPara::Hex(x), BodyPara::Hex(y)) => x.eq_ignore_ascii_case(y),
        _ => false,
    }
}

fn unlink_after(root: &mut Element, from: &TxbxTarget) -> Result<(), Error> {
    let Some(story) = story_id_of_target(root, from) else {
        return Ok(());
    };
    let seq = seq_of_target(root, from).unwrap_or(0);
    let mut hits = collect_story_hits(root, &story);
    hits.retain(|h| h.seq > seq);
    if hits.is_empty() {
        return Ok(());
    }
    let new_story = next_story_id(root).to_string();
    hits.sort_by_key(|h| h.seq);
    for (i, hit) in hits.iter().enumerate() {
        let first = i == 0;
        let new_seq = if first { 0 } else { i as u32 };
        if let Some(p) = para::find_paragraph_mut(root, &hit.para) {
            let _ = with_wsp_mut(p, &hit.key, |wsp| {
                set_link_elem(wsp, &new_story, new_seq, first);
            });
        }
    }
    Ok(())
}

fn attach_chain(
    root: &mut Element,
    start: &TxbxTarget,
    story: &str,
    start_seq: u32,
) -> Result<(), Error> {
    let old = story_id_of_target(root, start);
    let mut hits = if let Some(sid) = old {
        let mut h = collect_story_hits(root, &sid);
        h.sort_by_key(|x| x.seq);
        h
    } else {
        vec![StoryHit {
            para: start.host.body().clone(),
            key: start.key.clone(),
            seq: 0,
        }]
    };
    if hits.is_empty() {
        hits.push(StoryHit {
            para: start.host.body().clone(),
            key: start.key.clone(),
            seq: 0,
        });
    }
    for (i, hit) in hits.iter().enumerate() {
        let seq = start_seq + i as u32;
        let first = false;
        if let Some(p) = para::find_paragraph_mut(root, &hit.para) {
            let _ = with_wsp_mut(p, &hit.key, |wsp| {
                set_link_elem(wsp, story, seq, first);
            });
        }
    }
    Ok(())
}

fn write_link_on(
    root: &mut Element,
    t: &TxbxTarget,
    story: &str,
    seq: u32,
    first: bool,
) -> Result<(), Error> {
    let p = para::find_paragraph_mut(root, t.host.body())
        .ok_or_else(|| Error::new(ErrorKind::TextBoxNotFound { id: String::new() }))?;
    with_wsp_mut(p, &t.key, |wsp| set_link_elem(wsp, story, seq, first))
        .ok_or_else(|| Error::new(ErrorKind::TextBoxNotFound { id: String::new() }))?;
    Ok(())
}
