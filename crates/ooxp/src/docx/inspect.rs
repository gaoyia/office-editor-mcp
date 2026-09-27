//! 从主文档与页眉页脚读出大纲和段落。

use serde_json::{json, Value};

use crate::error::Error;
use crate::opc::{self, Package, Relationships};
use crate::xml;

use super::charts;
use super::comments;
use super::field;
use super::format;
use super::hf::{self, HfKind, HfPart};
use super::link;
use super::math;
use super::notes;
use super::numbering;
use super::para::{self, NoteKind};
use super::picture;
use super::request::InspectScope;
use super::revision;
use super::smartart;
use super::styles;
use super::tbl;
use super::textbox;

pub fn inspect(bytes: &[u8], scope: Option<InspectScope>) -> Result<Value, Error> {
    let pkg = Package::open_bytes(bytes.to_vec())?;
    let part = opc::office_document_part(&pkg)?;
    let doc = xml::parse(pkg.get(&part)?)?;
    let paras = para::collect_paragraphs(&doc.root);
    let para_count = paras.len();

    let mut outline = Vec::new();
    for (i, p) in paras.iter().enumerate() {
        let index = i + 1;
        if let Some(level) = para::heading_level(p) {
            let (id, _) = para::para_stable_id(p, index);
            outline.push(json!({
                "id": id,
                "index": index,
                "level": level,
                "style": para::p_style(p),
                "text": para::visible_text(p),
            }));
        }
    }

    let catalog = hf::catalog(&pkg, &part, &doc.root)?;
    let expand_hf = scope.as_ref().is_some_and(|s| s.header_footer);
    let expand_pics = scope.as_ref().is_some_and(|s| s.pictures);
    let expand_txbx = scope.as_ref().is_some_and(|s| s.text_boxes);
    let expand_txbx_content = scope.as_ref().is_some_and(|s| s.text_box_content);
    let expand_charts = scope.as_ref().is_some_and(|s| s.charts);
    let expand_smartart = scope.as_ref().is_some_and(|s| s.smart_art);
    let expand_tables = scope.as_ref().is_some_and(|s| s.tables);
    let expand_revisions = scope.as_ref().is_some_and(|s| s.revisions);
    let expand_hyperlinks = scope.as_ref().is_some_and(|s| s.hyperlinks);
    let expand_bookmarks = scope.as_ref().is_some_and(|s| s.bookmarks);
    let expand_fields = scope.as_ref().is_some_and(|s| s.fields);
    let expand_maths = scope.as_ref().is_some_and(|s| s.maths);
    let expand_runs = scope.as_ref().is_some_and(|s| s.runs);
    let hf_expand = HfExpand {
        paragraphs: expand_hf,
        pictures: expand_pics,
        text_boxes: expand_txbx,
        text_box_content: expand_txbx_content,
        charts: expand_charts,
        smart_art: expand_smartart,
        tables: expand_tables,
        revisions: expand_revisions,
        hyperlinks: expand_hyperlinks,
        bookmarks: expand_bookmarks,
        fields: expand_fields,
        maths: expand_maths,
        runs: expand_runs,
    };
    let (headers, footers) = hf_arrays(&pkg, &catalog, &hf_expand)?;
    let sections = section_nodes(&pkg, &part, &doc.root)?;
    let (mirror_margins, gutter_at_top) = super::sect::document_flags(&pkg, &part)?;
    let picture_count = body_picture_count(&paras)?;
    let body_boxes = collect_body_boxes(&paras)?;
    let text_box_count = body_boxes.len();
    let footnote_count = notes::user_count(&pkg, &part, NoteKind::Footnote)?;
    let endnote_count = notes::user_count(&pkg, &part, NoteKind::Endnote)?;
    let comment_count = comments::count(&pkg, &part)?;
    let mut body_chart_count = 0usize;
    let mut body_smartart_count = 0usize;
    for p in &paras {
        body_chart_count += charts::count_charts(p)?;
        body_smartart_count += smartart::count_smartarts(p)?;
    }
    let body_table_count = tbl::count_tables(&doc.root);

    let mut tree = json!({
        "schemaVersion": 1,
        "format": "docx",
        "id": "docx:document",
        "paraCount": para_count,
        "pictureCount": picture_count,
        "textBoxCount": text_box_count,
        "footnoteCount": footnote_count,
        "endnoteCount": endnote_count,
        "commentCount": comment_count,
        "chartCount": body_chart_count,
        "smartArtCount": body_smartart_count,
        "tableCount": body_table_count,
        "revisionCount": revision::count_revisions(&doc.root),
        "hyperlinkCount": link::count_hyperlinks(&doc.root),
        "bookmarkCount": link::count_bookmarks(&doc.root),
        "fieldCount": field::count_fields(&doc.root),
        "tocCount": field::count_tocs(&doc.root),
        "mathCount": math::count_maths(&doc.root),
        "numberingCount": numbering::count_nums(&pkg, &part)?,
        "styleCount": styles::count_styles(&pkg, &part)?,
        "outline": outline,
        "headers": headers,
        "footers": footers,
        "sections": sections,
        "mirrorMargins": mirror_margins,
        "gutterAtTop": gutter_at_top,
    });

    if let Some(scope) = scope {
        if scope.notes {
            let (footnotes, fn_seps) = notes_array(
                &pkg,
                &part,
                &doc.root,
                &catalog,
                NoteKind::Footnote,
                expand_pics,
                expand_runs,
            )?;
            let (endnotes, en_seps) = notes_array(
                &pkg,
                &part,
                &doc.root,
                &catalog,
                NoteKind::Endnote,
                expand_pics,
                expand_runs,
            )?;
            let mut seps = fn_seps;
            seps.extend(en_seps);
            tree["footnotes"] = footnotes;
            tree["endnotes"] = endnotes;
            tree["noteSeparators"] = Value::Array(seps);
        }
        let coverage = load_coverage(&pkg, &part, &doc.root, &catalog)?;
        if scope.comments {
            tree["comments"] = comments_array(&pkg, &part, &coverage, expand_pics, expand_runs)?;
        }
        if scope.charts {
            tree["charts"] = Value::Array(charts::collect_body(&pkg, &part, &paras)?);
        }
        if scope.smart_art {
            tree["smartArts"] = Value::Array(smartart::collect_body(&pkg, &part, &paras)?);
        }
        if scope.tables {
            tree["tables"] =
                Value::Array(tbl::tables_json(&doc.root, None, expand_pics, expand_runs)?);
        }
        if scope.revisions {
            tree["revisions"] = Value::Array(revision::revisions_json(&doc.root, None)?);
        }
        if scope.hyperlinks {
            let rels = Relationships::from_package(&pkg, &part)?;
            tree["hyperlinks"] = Value::Array(link::hyperlinks_json(&doc.root, None, &rels)?);
        }
        if scope.bookmarks {
            tree["bookmarks"] = Value::Array(link::bookmarks_json(&doc.root, None)?);
        }
        if scope.fields {
            tree["fields"] = Value::Array(field::fields_json(&doc.root, None)?);
        }
        if scope.tocs {
            tree["tocs"] = Value::Array(field::tocs_json(&doc.root)?);
        }
        if scope.maths {
            tree["maths"] = Value::Array(math::maths_json(&doc.root, None)?);
        }
        if scope.numbering {
            tree["numberings"] = Value::Array(numbering::numberings_json(&pkg, &part)?);
        }
        if scope.styles {
            tree["styles"] = Value::Array(styles::styles_json(&pkg, &part)?);
        }
        if let (Some(from), Some(max_paras)) = (scope.from, scope.max_paras) {
            let start = from as usize;
            let mut paragraphs = Vec::new();
            let mut truncated = false;
            if start <= para_count {
                for (i, p) in paras.iter().enumerate().skip(start.saturating_sub(1)) {
                    if paragraphs.len() as u32 >= max_paras {
                        truncated = true;
                        break;
                    }
                    let index = i + 1;
                    paragraphs.push(body_para_json(
                        p,
                        index,
                        expand_pics,
                        expand_txbx,
                        expand_txbx_content,
                        expand_runs,
                        &body_boxes,
                    )?);
                }
            }
            tree["from"] = json!(from);
            tree["truncated"] = json!(truncated);
            tree["paragraphs"] = Value::Array(paragraphs);
        }
        if let Some(cov) = coverage.as_ref() {
            stamp_comment_ids(&mut tree, cov);
        }
    }

    Ok(tree)
}

fn body_picture_count(paras: &[&crate::xml::Element]) -> Result<usize, Error> {
    let mut n = 0usize;
    for (i, p) in paras.iter().enumerate() {
        let (id, _) = para::para_stable_id(p, i + 1);
        n += picture::collect_pictures(p, &id)?.len();
    }
    Ok(n)
}

fn section_nodes(
    pkg: &Package,
    doc_part: &str,
    root: &crate::xml::Element,
) -> Result<Vec<Value>, Error> {
    let sects = hf::collect_sect_prs(root);
    let links = hf::section_link_states(pkg, doc_part, root)?;
    let mut out = Vec::new();
    for (i, sect) in sects.iter().enumerate() {
        let index = i + 1;
        let pg = sect.child_by_local("pgMar");
        let sz = sect.child_by_local("pgSz");
        let link = links.get(i).copied().unwrap_or(hf::SectionLinkState {
            header: hf::HfLinkTriple {
                default: Some(false),
                even: None,
                first: None,
            },
            footer: hf::HfLinkTriple {
                default: Some(false),
                even: None,
                first: None,
            },
        });
        out.push(json!({
            "id": format!("section:{index}"),
            "index": index,
            "headerDistance": hf::twip_attr(pg, "header")?,
            "footerDistance": hf::twip_attr(pg, "footer")?,
            "top": hf::signed_twip_attr(pg, "top")?,
            "bottom": hf::signed_twip_attr(pg, "bottom")?,
            "left": hf::twip_attr(pg, "left")?,
            "right": hf::twip_attr(pg, "right")?,
            "gutter": hf::twip_attr(pg, "gutter")?,
            "pageWidth": hf::measure_attr(sz, "w", "pgSz", false)?,
            "pageHeight": hf::measure_attr(sz, "h", "pgSz", false)?,
            "orient": super::sect::orient_value(sz)?,
            "columns": super::sect::columns_value(sect.child_by_local("cols"))?,
            "vAlign": super::sect::valign_value(sect.child_by_local("vAlign"))?,
            "lineNumbering": super::sect::line_numbering_value(sect.child_by_local("lnNumType"))?,
            "docGrid": super::sect::doc_grid_value(sect.child_by_local("docGrid"))?,
            "rtlGutter": super::sect::rtl_gutter_value(sect),
            "headerLink": {
                "default": link.header.default,
                "even": link.header.even,
                "first": link.header.first,
            },
            "footerLink": {
                "default": link.footer.default,
                "even": link.footer.even,
                "first": link.footer.first,
            },
        }));
    }
    Ok(out)
}

fn collect_body_boxes(paras: &[&crate::xml::Element]) -> Result<Vec<textbox::TxbxInfo>, Error> {
    let mut all = Vec::new();
    for (i, p) in paras.iter().enumerate() {
        let (id, _) = para::para_stable_id(p, i + 1);
        all.extend(textbox::collect_textboxes(p, &id)?);
    }
    textbox::resolve_links(&mut all);
    Ok(all)
}

fn boxes_for_para<'a>(all: &'a [textbox::TxbxInfo], para_id: &str) -> Vec<&'a textbox::TxbxInfo> {
    let prefix = format!("{para_id}/txbx:");
    all.iter().filter(|b| b.id.starts_with(&prefix)).collect()
}

struct HfExpand {
    paragraphs: bool,
    pictures: bool,
    text_boxes: bool,
    text_box_content: bool,
    charts: bool,
    smart_art: bool,
    tables: bool,
    revisions: bool,
    hyperlinks: bool,
    bookmarks: bool,
    fields: bool,
    maths: bool,
    runs: bool,
}

fn hf_arrays(
    pkg: &Package,
    catalog: &[HfPart],
    expand: &HfExpand,
) -> Result<(Vec<Value>, Vec<Value>), Error> {
    let mut headers = Vec::new();
    let mut footers = Vec::new();
    for item in catalog {
        let node = hf_node(pkg, item, expand)?;
        match item.kind {
            HfKind::Header => headers.push(node),
            HfKind::Footer => footers.push(node),
        }
    }
    Ok((headers, footers))
}

fn hf_node(pkg: &Package, item: &HfPart, expand: &HfExpand) -> Result<Value, Error> {
    let bytes = pkg.get(&item.part)?;
    let doc = xml::parse(bytes)?;
    let paras = para::collect_paragraphs(&doc.root);
    let pics = picture::collect_pictures(&doc.root, &item.id)?;
    let mut boxes = Vec::new();
    for (i, p) in paras.iter().enumerate() {
        let (id, _) = para::para_stable_id_in(&item.id, p, i + 1);
        boxes.extend(textbox::collect_textboxes(p, &id)?);
    }
    textbox::resolve_links(&mut boxes);
    let mut chart_count = 0usize;
    let mut smart_art_count = 0usize;
    for p in &paras {
        chart_count += charts::count_charts(p)?;
        smart_art_count += smartart::count_smartarts(p)?;
    }
    let mut node = json!({
        "id": item.id,
        "type": item.hf_type.as_str(),
        "sections": item.sections,
        "paraCount": paras.len(),
        "pictureCount": pics.len(),
        "textBoxCount": boxes.len(),
        "chartCount": chart_count,
        "smartArtCount": smart_art_count,
        "tableCount": tbl::count_tables(&doc.root),
        "revisionCount": revision::count_revisions(&doc.root),
        "hyperlinkCount": link::count_hyperlinks(&doc.root),
        "bookmarkCount": link::count_bookmarks(&doc.root),
        "fieldCount": field::count_fields(&doc.root),
        "mathCount": math::count_maths(&doc.root),
    });
    if expand.pictures {
        node["pictures"] = Value::Array(picture::pictures_json(&pics));
    }
    if expand.text_boxes {
        let mut listed = Vec::new();
        for (i, p) in paras.iter().enumerate() {
            let (id, _) = para::para_stable_id_in(&item.id, p, i + 1);
            let mine: Vec<_> = boxes_for_para(&boxes, &id).into_iter().cloned().collect();
            listed.extend(textbox::textboxes_json(
                p,
                &id,
                &mine,
                expand.text_box_content,
                expand.pictures,
                expand.runs,
            )?);
        }
        node["textBoxes"] = Value::Array(listed);
    }
    if expand.charts {
        node["charts"] = Value::Array(charts::collect_hf(pkg, item, &paras)?);
    }
    if expand.smart_art {
        node["smartArts"] = Value::Array(smartart::collect_hf(pkg, item, &paras)?);
    }
    if expand.tables {
        node["tables"] = Value::Array(tbl::tables_json(
            &doc.root,
            Some(&item.id),
            expand.pictures,
            expand.runs,
        )?);
    }
    if expand.revisions {
        node["revisions"] = Value::Array(revision::revisions_json(&doc.root, Some(&item.id))?);
    }
    if expand.hyperlinks {
        let rels = Relationships::from_package(pkg, &item.part)?;
        node["hyperlinks"] = Value::Array(link::hyperlinks_json(&doc.root, Some(&item.id), &rels)?);
    }
    if expand.bookmarks {
        node["bookmarks"] = Value::Array(link::bookmarks_json(&doc.root, Some(&item.id))?);
    }
    if expand.fields {
        node["fields"] = Value::Array(field::fields_json(&doc.root, Some(&item.id))?);
    }
    if expand.maths {
        node["maths"] = Value::Array(math::maths_json(&doc.root, Some(&item.id))?);
    }
    if expand.paragraphs {
        let mut paragraphs = Vec::new();
        for (i, p) in paras.iter().enumerate() {
            let index = i + 1;
            paragraphs.push(para_json(p, index, Some(&item.id), None, None, expand.runs));
        }
        node["paragraphs"] = Value::Array(paragraphs);
    }
    Ok(node)
}

fn body_para_json(
    p: &crate::xml::Element,
    index: usize,
    expand_pics: bool,
    expand_txbx: bool,
    expand_txbx_content: bool,
    expand_runs: bool,
    all_boxes: &[textbox::TxbxInfo],
) -> Result<Value, Error> {
    let (id, _) = para::para_stable_id(p, index);
    let pics = picture::collect_pictures(p, &id)?;
    let mine: Vec<_> = boxes_for_para(all_boxes, &id)
        .into_iter()
        .cloned()
        .collect();
    let mut node = para_json(p, index, None, Some(pics.len()), Some(mine.len()), false);
    if expand_pics {
        node["pictures"] = Value::Array(picture::pictures_json(&pics));
    }
    if expand_txbx {
        node["textBoxes"] = Value::Array(textbox::textboxes_json(
            p,
            &id,
            &mine,
            expand_txbx_content,
            expand_pics,
            expand_runs,
        )?);
    }
    let fn_refs = notes::refs_in_para(p, NoteKind::Footnote);
    if !fn_refs.is_empty() {
        node["footnoteRef"] = json!(fn_refs
            .into_iter()
            .map(|nid| NoteKind::Footnote.inspect_id(&nid))
            .collect::<Vec<_>>());
    }
    let en_refs = notes::refs_in_para(p, NoteKind::Endnote);
    if !en_refs.is_empty() {
        node["endnoteRef"] = json!(en_refs
            .into_iter()
            .map(|nid| NoteKind::Endnote.inspect_id(&nid))
            .collect::<Vec<_>>());
    }
    format::maybe_runs(&mut node, p, expand_runs);
    Ok(node)
}

fn para_json(
    p: &crate::xml::Element,
    index: usize,
    container: Option<&str>,
    picture_count: Option<usize>,
    text_box_count: Option<usize>,
    expand_runs: bool,
) -> Value {
    let mut node = format::para_node(p, index, container, picture_count, text_box_count);
    format::maybe_runs(&mut node, p, expand_runs);
    node
}

fn notes_array(
    pkg: &crate::opc::Package,
    doc_part: &str,
    doc_root: &crate::xml::Element,
    catalog: &[HfPart],
    kind: NoteKind,
    expand_pics: bool,
    expand_runs: bool,
) -> Result<(Value, Vec<Value>), Error> {
    let hosts = notes::collect_hosts(pkg, doc_part, doc_root, catalog, kind)?;
    let Some(part) = notes::part_name(pkg, doc_part, kind)? else {
        return Ok((Value::Array(Vec::new()), Vec::new()));
    };
    let doc = crate::xml::parse(pkg.get(&part)?)?;
    let mut users = Vec::new();
    for note in notes::user_notes(&doc.root, kind) {
        let note_id = note.attr_local("id").unwrap_or("").to_string();
        let inspect_id = kind.inspect_id(&note_id);
        let host: Vec<String> = hosts
            .iter()
            .filter(|(nid, _)| notes::ids_match(nid, &note_id))
            .map(|(_, pid)| pid.clone())
            .collect();
        let paras = para::collect_paragraphs(note);
        let mut pic_count = 0usize;
        let mut para_nodes = Vec::new();
        let mut all_pics = Vec::new();
        for (i, p) in paras.iter().enumerate() {
            let index = i + 1;
            let (pid, _) = para::para_stable_id_in(&inspect_id, p, index);
            let pics = picture::collect_pictures(p, &pid)?;
            pic_count += pics.len();
            let mut node = para_json(
                p,
                index,
                Some(&inspect_id),
                Some(pics.len()),
                Some(0),
                false,
            );
            if expand_pics {
                node["pictures"] = Value::Array(picture::pictures_json(&pics));
            }
            format::maybe_runs(&mut node, p, expand_runs);
            all_pics.extend(pics);
            para_nodes.push(node);
        }
        let mut node = json!({
            "id": inspect_id,
            "noteId": note_id,
            "type": "normal",
            "host": host,
            "paraCount": paras.len(),
            "pictureCount": pic_count,
        });
        if expand_pics {
            node["pictures"] = Value::Array(picture::pictures_json(&all_pics));
        }
        node["paragraphs"] = Value::Array(para_nodes);
        users.push(node);
    }
    let mut seps = Vec::new();
    for sep in notes::seps(&doc.root, kind) {
        let note_id = sep.attr_local("id").unwrap_or("").to_string();
        let typ = notes::sep_type(sep).unwrap_or("separator");
        seps.push(json!({
            "id": format!("{}:sep:{typ}", kind.as_str()),
            "kind": kind.as_str(),
            "type": typ,
            "noteId": note_id,
        }));
    }
    Ok((Value::Array(users), seps))
}

fn load_coverage(
    pkg: &crate::opc::Package,
    doc_part: &str,
    doc_root: &crate::xml::Element,
    catalog: &[HfPart],
) -> Result<Option<comments::CommentCoverage>, Error> {
    let Some(part) = comments::part_name(pkg, doc_part)? else {
        return Ok(None);
    };
    let Ok(bytes) = pkg.get(&part) else {
        return Ok(None);
    };
    let comments_doc = crate::xml::parse(bytes)?;
    let extended = if let Some(ex_part) = comments::extended_part_name(pkg, doc_part)? {
        pkg.get(&ex_part)
            .ok()
            .and_then(|b| crate::xml::parse(b).ok())
    } else {
        None
    };
    let cov = comments::collect_coverage(
        pkg,
        doc_part,
        doc_root,
        catalog,
        &comments_doc.root,
        extended.as_ref().map(|d| &d.root),
    )?;
    Ok(Some(cov))
}

fn comments_array(
    pkg: &crate::opc::Package,
    doc_part: &str,
    coverage: &Option<comments::CommentCoverage>,
    expand_pics: bool,
    expand_runs: bool,
) -> Result<Value, Error> {
    let Some(part) = comments::part_name(pkg, doc_part)? else {
        return Ok(Value::Array(Vec::new()));
    };
    let comments_doc = crate::xml::parse(pkg.get(&part)?)?;
    let extended = if let Some(ex_part) = comments::extended_part_name(pkg, doc_part)? {
        pkg.get(&ex_part)
            .ok()
            .and_then(|b| crate::xml::parse(b).ok())
    } else {
        None
    };
    let mut out = Vec::new();
    for comment in comments::comment_elements(&comments_doc.root) {
        let w_id = comment.attr_local("id").unwrap_or("").to_string();
        let inspect_id = comments::inspect_id(&w_id);
        let (done, parent) = comments::done_and_parent(
            comment,
            &comments_doc.root,
            extended.as_ref().map(|d| &d.root),
        );
        let kind = if comments::is_ink(comment) {
            "ink"
        } else {
            "normal"
        };
        let paras = para::collect_paragraphs(comment);
        let mut pic_count = 0usize;
        let mut para_nodes = Vec::new();
        let mut all_pics = Vec::new();
        for (i, p) in paras.iter().enumerate() {
            let index = i + 1;
            let (pid, _) = para::para_stable_id_in(&inspect_id, p, index);
            let pics = picture::collect_pictures(p, &pid)?;
            pic_count += pics.len();
            let mut node = para_json(
                p,
                index,
                Some(&inspect_id),
                Some(pics.len()),
                Some(0),
                false,
            );
            if expand_pics {
                node["pictures"] = Value::Array(picture::pictures_json(&pics));
            }
            format::maybe_runs(&mut node, p, expand_runs);
            all_pics.extend(pics);
            para_nodes.push(node);
        }
        let start = coverage
            .as_ref()
            .and_then(|c| c.start.get(&w_id))
            .cloned()
            .flatten();
        let end = coverage
            .as_ref()
            .and_then(|c| c.end.get(&w_id))
            .cloned()
            .flatten();
        let mut node = json!({
            "id": inspect_id,
            "commentId": w_id,
            "author": comment.attr_local("author"),
            "initials": comment.attr_local("initials"),
            "date": comment.attr_local("date"),
            "done": done,
            "parent": parent,
            "kind": kind,
            "start": start,
            "end": end,
            "paraCount": paras.len(),
            "pictureCount": pic_count,
        });
        if expand_pics {
            node["pictures"] = Value::Array(picture::pictures_json(&all_pics));
        }
        node["paragraphs"] = Value::Array(para_nodes);
        out.push(node);
    }
    Ok(Value::Array(out))
}

fn stamp_comment_ids(tree: &mut Value, coverage: &comments::CommentCoverage) {
    stamp_value(tree, coverage);
}

fn stamp_value(v: &mut Value, coverage: &comments::CommentCoverage) {
    match v {
        Value::Object(map) => {
            if let Some(Value::String(id)) = map.get("id") {
                if map.contains_key("text") && map.contains_key("stable") {
                    if let Some(ids) = coverage.on_para.get(id) {
                        if !ids.is_empty() {
                            let listed: Vec<String> =
                                ids.iter().map(|c| comments::inspect_id(c)).collect();
                            if map.contains_key("runs") {
                                let mut out = serde_json::Map::new();
                                for (k, v) in map.iter() {
                                    if k == "runs" {
                                        out.insert("commentIds".into(), json!(listed));
                                    }
                                    out.insert(k.clone(), v.clone());
                                }
                                *map = out;
                            } else {
                                map.insert("commentIds".into(), json!(listed));
                            }
                        }
                    }
                }
            }
            for (k, child) in map.iter_mut() {
                if matches!(
                    k.as_str(),
                    "paragraphs"
                        | "textBoxes"
                        | "headers"
                        | "footers"
                        | "footnotes"
                        | "endnotes"
                        | "comments"
                ) {
                    stamp_value(child, coverage);
                }
            }
        }
        Value::Array(arr) => {
            for x in arr {
                stamp_value(x, coverage);
            }
        }
        _ => {}
    }
}
