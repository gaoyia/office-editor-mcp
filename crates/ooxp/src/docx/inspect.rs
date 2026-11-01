//! 从主文档与页眉页脚读出大纲和段落。

use serde_json::{json, Map, Value};

use crate::error::Error;
use crate::opc::{self, Package, Relationships};
use crate::xml::{self, Element};

use super::charts;
use super::comments;
use super::connector;
use super::east_asia;
use super::field;
use super::format;
use super::hf::{self, HfKind, HfPart};
use super::ink;
use super::link;
use super::math;
use super::notes;
use super::numbering;
use super::ole;
use super::para::{self, NoteKind};
use super::picture;
use super::request::InspectScope;
use super::revision;
use super::ruby;
use super::sdt;
use super::shape;
use super::smartart;
use super::styles;
use super::tbl;
use super::textbox;
use super::watermark;

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
    let expand_cxn = scope.as_ref().is_some_and(|s| s.connectors);
    let expand_shapes = scope.as_ref().is_some_and(|s| s.shapes);
    let expand_ink = scope.as_ref().is_some_and(|s| s.ink_canvases);
    let expand_model3ds = scope.as_ref().is_some_and(|s| s.model3ds);
    let expand_txbx_content = scope.as_ref().is_some_and(|s| s.text_box_content);
    let expand_charts = scope.as_ref().is_some_and(|s| s.charts);
    let expand_smartart = scope.as_ref().is_some_and(|s| s.smart_art);
    let expand_ole = scope.as_ref().is_some_and(|s| s.ole_objects);
    let expand_tables = scope.as_ref().is_some_and(|s| s.tables);
    let expand_revisions = scope.as_ref().is_some_and(|s| s.revisions);
    let expand_hyperlinks = scope.as_ref().is_some_and(|s| s.hyperlinks);
    let expand_bookmarks = scope.as_ref().is_some_and(|s| s.bookmarks);
    let expand_edit_ranges = scope.as_ref().is_some_and(|s| s.edit_ranges);
    let expand_fields = scope.as_ref().is_some_and(|s| s.fields);
    let expand_tocs = scope.as_ref().is_some_and(|s| s.tocs);
    let expand_captions = scope.as_ref().is_some_and(|s| s.captions);
    let expand_xrefs = scope.as_ref().is_some_and(|s| s.cross_refs);
    let expand_indexes = scope.as_ref().is_some_and(|s| s.indexes);
    let expand_xes = scope.as_ref().is_some_and(|s| s.index_entries);
    let expand_cites = scope.as_ref().is_some_and(|s| s.citations);
    let expand_biblios = scope.as_ref().is_some_and(|s| s.bibliographies);
    let block_flags = BlockFlags {
        tocs: expand_tocs,
        captions: expand_captions,
        cross_refs: expand_xrefs,
        indexes: expand_indexes,
        index_entries: expand_xes,
        citations: expand_cites,
        bibliographies: expand_biblios,
    };
    let expand_maths = scope.as_ref().is_some_and(|s| s.maths);
    let expand_rubies = scope.as_ref().is_some_and(|s| s.rubies);
    let expand_sdts = scope.as_ref().is_some_and(|s| s.content_controls);
    let expand_runs = scope.as_ref().is_some_and(|s| s.runs);
    let hf_expand = HfExpand {
        paragraphs: expand_hf,
        pictures: expand_pics,
        text_boxes: expand_txbx,
        connectors: expand_cxn,
        shapes: expand_shapes,
        ink_canvases: expand_ink,
        model3ds: expand_model3ds,
        text_box_content: expand_txbx_content,
        charts: expand_charts,
        smart_art: expand_smartart,
        ole_objects: expand_ole,
        tables: expand_tables,
        revisions: expand_revisions,
        hyperlinks: expand_hyperlinks,
        bookmarks: expand_bookmarks,
        edit_ranges: expand_edit_ranges,
        fields: expand_fields,
        tocs: expand_tocs,
        captions: expand_captions,
        cross_refs: expand_xrefs,
        indexes: expand_indexes,
        index_entries: expand_xes,
        citations: expand_cites,
        bibliographies: expand_biblios,
        maths: expand_maths,
        rubies: expand_rubies,
        content_controls: expand_sdts,
        runs: expand_runs,
    };
    let (headers, footers) = hf_arrays(&pkg, &catalog, &hf_expand)?;
    let sections = section_nodes(&pkg, &part, &doc.root)?;
    let (mirror_margins, gutter_at_top) = super::sect::document_flags(&pkg, &part)?;
    let picture_count = body_picture_count(&paras)?;
    let body_rels = Relationships::from_package(&pkg, &part)?;
    let body_boxes = collect_body_boxes(&paras)?;
    let text_box_count = body_boxes.len();
    let body_cxns = collect_body_cxns(&paras, Some(&body_rels))?;
    let connector_count = body_cxns.len();
    let body_shapes = collect_body_shapes(&paras, Some(&body_rels))?;
    let shape_count = body_shapes.len();
    let mut ink_canvas_count = 0usize;
    let mut body_model3d_count = 0usize;
    for p in &paras {
        ink_canvas_count += ink::count_in(&pkg, &part, p)?;
        body_model3d_count += super::model3d::count_in(&pkg, &part, p)?;
    }
    let footnote_count = notes::user_count(&pkg, &part, NoteKind::Footnote)?;
    let endnote_count = notes::user_count(&pkg, &part, NoteKind::Endnote)?;
    let comment_count = comments::count(&pkg, &part)?;
    let mut body_chart_count = 0usize;
    let mut body_smartart_count = 0usize;
    let mut body_ole_count = 0usize;
    for p in &paras {
        body_chart_count += charts::count_charts(p)?;
        body_smartart_count += smartart::count_smartarts(p)?;
        body_ole_count += ole::count_in(&pkg, &part, p)?;
    }
    let nested_ole = ole::collect_nested(&pkg, &part, &doc.root)?;
    body_ole_count += nested_ole.len();
    let body_table_count = tbl::count_tables(&doc.root);
    let source_count = super::biblio::count_sources(&pkg, &part)?;

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
        "oleObjectCount": body_ole_count,
        "watermarkCount": watermark::count_in_headers(&pkg, &catalog)?,
        "tableCount": body_table_count,
        "revisionCount": revision::count_revisions(&doc.root),
        "hyperlinkCount": link::count_hyperlinks(&doc.root),
        "bookmarkCount": link::count_bookmarks(&doc.root),
        "fieldCount": field::count_fields(&doc.root),
        "tocCount": field::count_tocs(&doc.root),
        "captionCount": field::count_captions(&doc.root),
        "crossRefCount": field::count_cross_refs(&doc.root),
        "indexCount": field::count_indexes(&doc.root),
        "indexEntryCount": field::count_index_entries(&doc.root),
        "sourceCount": source_count,
        "citationCount": field::count_citations(&doc.root),
        "bibliographyCount": field::count_bibliographies(&doc.root),
        "docPartCount": super::glossary::count_parts(&pkg, &part)?,
        "mathCount": math::count_maths(&doc.root),
        "rubyCount": ruby::count_rubies(&doc.root),
        "contentControlCount": sdt::count_sdts(&doc.root),
        "numberingCount": numbering::count_nums(&pkg, &part)?,
        "styleCount": styles::count_styles(&pkg, &part)?,
        "outline": outline,
        "headers": headers,
        "footers": footers,
        "sections": sections,
        "mirrorMargins": mirror_margins,
        "gutterAtTop": gutter_at_top,
        "props": crate::props::inspect(&pkg)?,
        "protection": super::protect::inspect(&pkg, &part)?,
        "theme": super::theme::inspect(&pkg, &part)?,
    });
    insert_json_after(
        tree.as_object_mut().unwrap(),
        "oleObjectCount",
        "model3dCount",
        json!(body_model3d_count),
    );
    insert_json_after(
        tree.as_object_mut().unwrap(),
        "textBoxCount",
        "connectorCount",
        json!(connector_count),
    );
    insert_json_after(
        tree.as_object_mut().unwrap(),
        "connectorCount",
        "shapeCount",
        json!(shape_count),
    );
    insert_json_after(
        tree.as_object_mut().unwrap(),
        "shapeCount",
        "inkCanvasCount",
        json!(ink_canvas_count),
    );
    insert_json_after(
        tree.as_object_mut().unwrap(),
        "styleCount",
        "embeddedFontCount",
        json!(super::font_embed::count(&pkg, &part)?),
    );
    insert_json_after(
        tree.as_object_mut().unwrap(),
        "theme",
        "themeOverride",
        super::theme::inspect_override(&pkg, &part)?,
    );
    insert_json_after(
        tree.as_object_mut().unwrap(),
        "themeOverride",
        "fontEmbed",
        super::font_embed::flags_json(&pkg, &part)?,
    );
    insert_json_after(
        tree.as_object_mut().unwrap(),
        "fontEmbed",
        "pageBackground",
        super::page_bg::background_json(&pkg, &part, &doc.root)?,
    );
    insert_json_after(
        tree.as_object_mut().unwrap(),
        "pageBackground",
        "eastAsianSettings",
        east_asia::settings_json(&pkg, &part)?,
    );
    insert_json_after(
        tree.as_object_mut().unwrap(),
        "eastAsianSettings",
        "bibliographyStyle",
        super::biblio::style_json(&pkg, &part)?,
    );
    insert_json_after(
        tree.as_object_mut().unwrap(),
        "bookmarkCount",
        "editRangeCount",
        json!(super::perm::count_edit_ranges(&doc.root)),
    );

    if let Some(scope) = scope {
        if scope.notes {
            let (footnotes, fn_seps) = notes_array(
                &pkg,
                &part,
                &doc.root,
                &catalog,
                NoteKind::Footnote,
                expand_pics,
                expand_txbx,
                expand_cxn,
                expand_txbx_content,
                expand_charts,
                expand_smartart,
                expand_runs,
                scope.revisions,
                &block_flags,
            )?;
            let (endnotes, en_seps) = notes_array(
                &pkg,
                &part,
                &doc.root,
                &catalog,
                NoteKind::Endnote,
                expand_pics,
                expand_txbx,
                expand_cxn,
                expand_txbx_content,
                expand_charts,
                expand_smartart,
                expand_runs,
                scope.revisions,
                &block_flags,
            )?;
            let mut seps = fn_seps;
            seps.extend(en_seps);
            tree["footnotes"] = footnotes;
            tree["endnotes"] = endnotes;
            tree["noteSeparators"] = Value::Array(seps);
        }
        let coverage = load_coverage(&pkg, &part, &doc.root, &catalog)?;
        if scope.comments {
            tree["comments"] = comments_array(
                &pkg,
                &part,
                &coverage,
                expand_pics,
                expand_txbx,
                expand_cxn,
                expand_txbx_content,
                expand_charts,
                expand_smartart,
                expand_runs,
                scope.revisions,
                &block_flags,
            )?;
        }
        if scope.charts {
            tree["charts"] = Value::Array(charts::collect_body(&pkg, &part, &paras)?);
        }
        if scope.smart_art {
            tree["smartArts"] = Value::Array(smartart::collect_body(&pkg, &part, &paras)?);
        }
        if scope.ole_objects {
            let mut items = ole::collect_body(&pkg, &part, &paras)?;
            items.extend(ole::collect_nested(&pkg, &part, &doc.root)?);
            tree["oleObjects"] = Value::Array(items);
        }
        if scope.watermarks {
            tree["watermarks"] = Value::Array(watermark::list_all(&pkg, &catalog)?);
        }
        if scope.tables {
            tree["tables"] = Value::Array(tbl::tables_json(
                &pkg,
                &part,
                &doc.root,
                None,
                expand_pics,
                expand_runs,
            )?);
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
        if scope.edit_ranges {
            tree["editRanges"] = Value::Array(super::perm::edit_ranges_json(&doc.root, None)?);
        }
        if scope.fields {
            tree["fields"] = Value::Array(field::fields_json(&doc.root, None)?);
        }
        if scope.tocs {
            tree["tocs"] = Value::Array(field::tocs_json(&doc.root, None)?);
        }
        if scope.captions {
            tree["captions"] = Value::Array(field::captions_json(&doc.root, None)?);
        }
        if scope.cross_refs {
            tree["crossRefs"] = Value::Array(field::cross_refs_json(&doc.root, None)?);
        }
        if scope.indexes {
            tree["indexes"] = Value::Array(field::indexes_json(&doc.root, None)?);
        }
        if scope.index_entries {
            tree["indexEntries"] = Value::Array(field::index_entries_json(&doc.root, None)?);
        }
        if scope.sources {
            tree["sources"] = Value::Array(super::biblio::sources_json(&pkg, &part)?);
        }
        if scope.citations {
            tree["citations"] = Value::Array(field::citations_json(&doc.root, None)?);
        }
        if scope.bibliographies {
            tree["bibliographies"] = Value::Array(field::bibliographies_json(&doc.root, None)?);
        }
        if scope.doc_parts {
            tree["docParts"] = Value::Array(super::glossary::parts_json(&pkg, &part)?);
        }
        if scope.embedded_fonts {
            tree["embeddedFonts"] = Value::Array(super::font_embed::fonts_json(&pkg, &part)?);
        }
        if scope.maths {
            tree["maths"] = Value::Array(math::maths_json(&doc.root, None)?);
        }
        if scope.rubies {
            tree["rubies"] = Value::Array(ruby::rubies_json(&doc.root, None)?);
        }
        if scope.content_controls {
            tree["contentControls"] = Value::Array(sdt::sdts_json(&doc.root, None)?);
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
                        &pkg,
                        &part,
                        p,
                        index,
                        expand_pics,
                        expand_txbx,
                        expand_cxn,
                        expand_shapes,
                        expand_ink,
                        expand_model3ds,
                        expand_runs,
                        textbox::TxbxExpand {
                            content: expand_txbx_content,
                            pictures: expand_pics,
                            runs: expand_runs,
                            charts: expand_charts,
                            smart_art: expand_smartart,
                            tables: expand_tables,
                            hyperlinks: expand_hyperlinks,
                            bookmarks: expand_bookmarks,
                            fields: expand_fields,
                            tocs: expand_tocs,
                            captions: expand_captions,
                            cross_refs: expand_xrefs,
                            indexes: expand_indexes,
                            index_entries: expand_xes,
                            citations: expand_cites,
                            bibliographies: expand_biblios,
                            maths: expand_maths,
                            rubies: expand_rubies,
                            content_controls: expand_sdts,
                        },
                        &body_boxes,
                        &body_cxns,
                        &body_shapes,
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
            "pageBorders": super::page_bg::borders_json(sect),
            "formProt": match hf::on_off_opt(sect, "formProt") {
                Some(b) => json!(b),
                None => Value::Null,
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

fn collect_body_cxns(
    paras: &[&crate::xml::Element],
    rels: Option<&Relationships>,
) -> Result<Vec<connector::CxnInfo>, Error> {
    let mut all = Vec::new();
    for (i, p) in paras.iter().enumerate() {
        let (id, _) = para::para_stable_id(p, i + 1);
        all.extend(connector::collect_connectors(p, &id, rels)?);
    }
    Ok(all)
}

fn cxns_for_para<'a>(all: &'a [connector::CxnInfo], para_id: &str) -> Vec<&'a connector::CxnInfo> {
    let prefix = format!("{para_id}/cxn:");
    all.iter().filter(|c| c.id.starts_with(&prefix)).collect()
}

fn collect_body_shapes(
    paras: &[&crate::xml::Element],
    rels: Option<&Relationships>,
) -> Result<Vec<shape::ShapeInfo>, Error> {
    let mut all = Vec::new();
    for (i, p) in paras.iter().enumerate() {
        let (id, _) = para::para_stable_id(p, i + 1);
        all.extend(shape::collect_shapes(p, &id, rels)?);
    }
    Ok(all)
}

fn shapes_for_para<'a>(all: &'a [shape::ShapeInfo], para_id: &str) -> Vec<&'a shape::ShapeInfo> {
    let prefix = format!("{para_id}/shape:");
    all.iter().filter(|c| c.id.starts_with(&prefix)).collect()
}

struct HfExpand {
    paragraphs: bool,
    pictures: bool,
    text_boxes: bool,
    connectors: bool,
    shapes: bool,
    ink_canvases: bool,
    model3ds: bool,
    text_box_content: bool,
    charts: bool,
    smart_art: bool,
    ole_objects: bool,
    tables: bool,
    revisions: bool,
    hyperlinks: bool,
    bookmarks: bool,
    edit_ranges: bool,
    fields: bool,
    tocs: bool,
    captions: bool,
    cross_refs: bool,
    indexes: bool,
    index_entries: bool,
    citations: bool,
    bibliographies: bool,
    maths: bool,
    rubies: bool,
    content_controls: bool,
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
    let rels = Relationships::from_package(pkg, &item.part)?;
    let paras = para::collect_paragraphs(&doc.root);
    let pics = picture::collect_pictures_with(&doc.root, &item.id, Some(&rels))?;
    let mut boxes = Vec::new();
    for (i, p) in paras.iter().enumerate() {
        let (id, _) = para::para_stable_id_in(&item.id, p, i + 1);
        boxes.extend(textbox::collect_textboxes(p, &id)?);
    }
    textbox::resolve_links(&mut boxes);
    let mut cxns = Vec::new();
    for (i, p) in paras.iter().enumerate() {
        let (id, _) = para::para_stable_id_in(&item.id, p, i + 1);
        cxns.extend(connector::collect_connectors(p, &id, Some(&rels))?);
    }
    let mut shapes = Vec::new();
    for (i, p) in paras.iter().enumerate() {
        let (id, _) = para::para_stable_id_in(&item.id, p, i + 1);
        shapes.extend(shape::collect_shapes(p, &id, Some(&rels))?);
    }
    let mut chart_count = 0usize;
    let mut smart_art_count = 0usize;
    let mut ole_count = 0usize;
    let mut model3d_n = 0usize;
    for p in &paras {
        chart_count += charts::count_charts(p)?;
        smart_art_count += smartart::count_smartarts(p)?;
        ole_count += ole::count_in(pkg, &item.part, p)?;
        model3d_n += super::model3d::count_in(pkg, &item.part, p)?;
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
        "oleObjectCount": ole_count,
        "watermarkCount": if item.kind == HfKind::Header {
            watermark::collect(&doc.root, &item.id).len()
        } else {
            0
        },
        "tableCount": tbl::count_tables(&doc.root),
        "revisionCount": revision::count_revisions(&doc.root),
        "hyperlinkCount": link::count_hyperlinks(&doc.root),
        "bookmarkCount": link::count_bookmarks(&doc.root),
        "fieldCount": field::count_fields(&doc.root),
        "mathCount": math::count_maths(&doc.root),
        "rubyCount": ruby::count_rubies(&doc.root),
        "contentControlCount": sdt::count_sdts(&doc.root),
    });
    insert_json_after(
        node.as_object_mut().unwrap(),
        "oleObjectCount",
        "model3dCount",
        json!(model3d_n),
    );
    insert_json_after(
        node.as_object_mut().unwrap(),
        "bookmarkCount",
        "editRangeCount",
        json!(super::perm::count_edit_ranges(&doc.root)),
    );
    insert_json_after(
        node.as_object_mut().unwrap(),
        "textBoxCount",
        "connectorCount",
        json!(cxns.len()),
    );
    insert_json_after(
        node.as_object_mut().unwrap(),
        "connectorCount",
        "shapeCount",
        json!(shapes.len()),
    );
    let mut ink_n = 0usize;
    for p in &paras {
        ink_n += ink::count_in(pkg, &item.part, p)?;
    }
    insert_json_after(
        node.as_object_mut().unwrap(),
        "shapeCount",
        "inkCanvasCount",
        json!(ink_n),
    );
    put_block_counts(&mut node, &doc.root, "fieldCount");
    if expand.pictures {
        node["pictures"] = Value::Array(picture::pictures_json(&pics));
    }
    if expand.text_boxes {
        let mut listed = Vec::new();
        for (i, p) in paras.iter().enumerate() {
            let (id, _) = para::para_stable_id_in(&item.id, p, i + 1);
            let mine: Vec<_> = boxes_for_para(&boxes, &id).into_iter().cloned().collect();
            listed.extend(textbox::textboxes_json(
                pkg,
                &item.part,
                p,
                &id,
                &mine,
                textbox::TxbxExpand {
                    content: expand.text_box_content,
                    pictures: expand.pictures,
                    runs: expand.runs,
                    charts: expand.charts,
                    smart_art: expand.smart_art,
                    tables: expand.tables,
                    hyperlinks: expand.hyperlinks,
                    bookmarks: expand.bookmarks,
                    fields: expand.fields,
                    tocs: expand.tocs,
                    captions: expand.captions,
                    cross_refs: expand.cross_refs,
                    indexes: expand.indexes,
                    index_entries: expand.index_entries,
                    citations: expand.citations,
                    bibliographies: expand.bibliographies,
                    maths: expand.maths,
                    rubies: expand.rubies,
                    content_controls: expand.content_controls,
                },
            )?);
        }
        node["textBoxes"] = Value::Array(listed);
    }
    if expand.connectors {
        node["connectors"] = Value::Array(connector::connectors_json(&cxns));
    }
    if expand.shapes {
        node["shapes"] = Value::Array(shape::shapes_json(&shapes));
    }
    if expand.ink_canvases {
        let mut listed = Vec::new();
        for (i, p) in paras.iter().enumerate() {
            let (id, _) = para::para_stable_id_in(&item.id, p, i + 1);
            listed.extend(ink::canvases_json(&ink::collect(pkg, &item.part, p, &id)?));
        }
        node["inkCanvases"] = Value::Array(listed);
    }
    if expand.model3ds {
        let mut listed = Vec::new();
        for (i, p) in paras.iter().enumerate() {
            let (id, _) = para::para_stable_id_in(&item.id, p, i + 1);
            listed.extend(super::model3d::list_json(&super::model3d::collect(
                pkg, &item.part, p, &id,
            )?));
        }
        node["model3ds"] = Value::Array(listed);
    }
    if expand.charts {
        node["charts"] = Value::Array(charts::collect_hf(pkg, item, &paras)?);
    }
    if expand.smart_art {
        node["smartArts"] = Value::Array(smartart::collect_hf(pkg, item, &paras)?);
    }
    if expand.ole_objects {
        node["oleObjects"] = Value::Array(ole::collect_hf(pkg, item, &paras)?);
    }
    if expand.tables {
        node["tables"] = Value::Array(tbl::tables_json(
            pkg,
            &item.part,
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
    if expand.edit_ranges {
        node["editRanges"] =
            Value::Array(super::perm::edit_ranges_json(&doc.root, Some(&item.id))?);
    }
    if expand.fields {
        node["fields"] = Value::Array(field::fields_json(&doc.root, Some(&item.id))?);
    }
    put_block_lists(
        &mut node,
        &doc.root,
        &item.id,
        &BlockFlags {
            tocs: expand.tocs,
            captions: expand.captions,
            cross_refs: expand.cross_refs,
            indexes: expand.indexes,
            index_entries: expand.index_entries,
            citations: expand.citations,
            bibliographies: expand.bibliographies,
        },
    )?;
    if expand.maths {
        node["maths"] = Value::Array(math::maths_json(&doc.root, Some(&item.id))?);
    }
    if expand.rubies {
        node["rubies"] = Value::Array(ruby::rubies_json(&doc.root, Some(&item.id))?);
    }
    if expand.content_controls {
        node["contentControls"] = Value::Array(sdt::sdts_json(&doc.root, Some(&item.id))?);
    }
    if expand.paragraphs {
        let mut paragraphs = Vec::new();
        for (i, p) in paras.iter().enumerate() {
            let index = i + 1;
            paragraphs.push(para_json(
                p,
                index,
                Some(&item.id),
                None,
                None,
                None,
                expand.runs,
            ));
        }
        node["paragraphs"] = Value::Array(paragraphs);
    }
    Ok(node)
}

fn body_para_json(
    pkg: &Package,
    host_part: &str,
    p: &crate::xml::Element,
    index: usize,
    expand_pics: bool,
    expand_txbx: bool,
    expand_cxn: bool,
    expand_shapes: bool,
    expand_ink: bool,
    expand_model3ds: bool,
    expand_runs: bool,
    txbx: textbox::TxbxExpand,
    all_boxes: &[textbox::TxbxInfo],
    all_cxns: &[connector::CxnInfo],
    all_shapes: &[shape::ShapeInfo],
) -> Result<Value, Error> {
    let (id, _) = para::para_stable_id(p, index);
    let rels = Relationships::from_package(pkg, host_part)?;
    let pics = picture::collect_pictures_with(p, &id, Some(&rels))?;
    let mine: Vec<_> = boxes_for_para(all_boxes, &id)
        .into_iter()
        .cloned()
        .collect();
    let mine_cxn: Vec<_> = cxns_for_para(all_cxns, &id).into_iter().cloned().collect();
    let mine_shape: Vec<_> = shapes_for_para(all_shapes, &id)
        .into_iter()
        .cloned()
        .collect();
    let inks = ink::collect(pkg, host_part, p, &id)?;
    let m3ds = super::model3d::collect(pkg, host_part, p, &id)?;
    let mut node = para_json(
        p,
        index,
        None,
        Some(pics.len()),
        Some(mine.len()),
        Some(mine_cxn.len()),
        false,
    );
    insert_json_after(
        node.as_object_mut().unwrap(),
        "connectorCount",
        "shapeCount",
        json!(mine_shape.len()),
    );
    insert_json_after(
        node.as_object_mut().unwrap(),
        "shapeCount",
        "inkCanvasCount",
        json!(inks.len()),
    );
    insert_json_after(
        node.as_object_mut().unwrap(),
        "inkCanvasCount",
        "model3dCount",
        json!(m3ds.len()),
    );
    if expand_pics {
        node["pictures"] = Value::Array(picture::pictures_json(&pics));
    }
    if expand_txbx {
        node["textBoxes"] = Value::Array(textbox::textboxes_json(
            pkg, host_part, p, &id, &mine, txbx,
        )?);
    }
    if expand_cxn {
        node["connectors"] = Value::Array(connector::connectors_json(&mine_cxn));
    }
    if expand_shapes {
        node["shapes"] = Value::Array(shape::shapes_json(&mine_shape));
    }
    if expand_ink {
        node["inkCanvases"] = Value::Array(ink::canvases_json(&inks));
    }
    if expand_model3ds {
        node["model3ds"] = Value::Array(super::model3d::list_json(&m3ds));
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
    connector_count: Option<usize>,
    expand_runs: bool,
) -> Value {
    let mut node = format::para_node(
        p,
        index,
        container,
        picture_count,
        text_box_count,
        connector_count,
    );
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
    expand_txbx: bool,
    expand_cxn: bool,
    expand_txbx_content: bool,
    expand_charts: bool,
    expand_smartart: bool,
    expand_runs: bool,
    expand_revisions: bool,
    blocks: &BlockFlags,
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
        let media = story_media(
            pkg,
            &part,
            note,
            &inspect_id,
            expand_pics,
            expand_txbx,
            expand_cxn,
            expand_charts,
            expand_smartart,
            expand_runs,
            textbox::TxbxExpand {
                content: expand_txbx_content,
                pictures: expand_pics,
                runs: expand_runs,
                charts: expand_charts,
                smart_art: expand_smartart,
                tables: false,
                ..Default::default()
            },
        )?;
        let mut node = json!({
            "id": inspect_id,
            "noteId": note_id,
            "type": "normal",
            "host": host,
            "paraCount": media.para_count,
            "pictureCount": media.pic_count,
            "textBoxCount": media.box_count,
            "connectorCount": media.cxn_count,
            "chartCount": media.chart_count,
            "smartArtCount": media.smart_art_count,
            "revisionCount": revision::count_revisions(note),
        });
        put_block_counts(&mut node, note, "revisionCount");
        if expand_revisions {
            node["revisions"] = Value::Array(revision::revisions_json(note, Some(&inspect_id))?);
        }
        put_block_lists(&mut node, note, &inspect_id, blocks)?;
        attach_story_lists(
            &mut node,
            &media,
            expand_pics,
            expand_txbx,
            expand_cxn,
            expand_charts,
            expand_smartart,
        );
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
    expand_txbx: bool,
    expand_cxn: bool,
    expand_txbx_content: bool,
    expand_charts: bool,
    expand_smartart: bool,
    expand_runs: bool,
    expand_revisions: bool,
    blocks: &BlockFlags,
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
        let media = story_media(
            pkg,
            &part,
            comment,
            &inspect_id,
            expand_pics,
            expand_txbx,
            expand_cxn,
            expand_charts,
            expand_smartart,
            expand_runs,
            textbox::TxbxExpand {
                content: expand_txbx_content,
                pictures: expand_pics,
                runs: expand_runs,
                charts: expand_charts,
                smart_art: expand_smartart,
                tables: false,
                ..Default::default()
            },
        )?;
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
            "paraCount": media.para_count,
            "pictureCount": media.pic_count,
            "textBoxCount": media.box_count,
            "connectorCount": media.cxn_count,
            "chartCount": media.chart_count,
            "smartArtCount": media.smart_art_count,
            "revisionCount": revision::count_revisions(comment),
        });
        put_block_counts(&mut node, comment, "revisionCount");
        if expand_revisions {
            node["revisions"] = Value::Array(revision::revisions_json(comment, Some(&inspect_id))?);
        }
        put_block_lists(&mut node, comment, &inspect_id, blocks)?;
        attach_story_lists(
            &mut node,
            &media,
            expand_pics,
            expand_txbx,
            expand_cxn,
            expand_charts,
            expand_smartart,
        );
        out.push(node);
    }
    Ok(Value::Array(out))
}

struct StoryMedia {
    para_count: usize,
    pic_count: usize,
    box_count: usize,
    cxn_count: usize,
    chart_count: usize,
    smart_art_count: usize,
    pictures: Vec<picture::PicInfo>,
    text_boxes: Vec<Value>,
    connectors: Vec<Value>,
    charts: Vec<Value>,
    smart_arts: Vec<Value>,
    paragraphs: Vec<Value>,
}

fn story_media(
    pkg: &Package,
    host_part: &str,
    story: &crate::xml::Element,
    container_id: &str,
    expand_pics: bool,
    expand_txbx: bool,
    expand_cxn: bool,
    expand_charts: bool,
    expand_smartart: bool,
    expand_runs: bool,
    txbx: textbox::TxbxExpand,
) -> Result<StoryMedia, Error> {
    let paras = para::collect_paragraphs(story);
    let rels = Relationships::from_package(pkg, host_part)?;
    let mut all_boxes = Vec::new();
    let mut all_cxns = Vec::new();
    for (i, p) in paras.iter().enumerate() {
        let (pid, _) = para::para_stable_id_in(container_id, p, i + 1);
        all_boxes.extend(textbox::collect_textboxes(p, &pid)?);
        all_cxns.extend(connector::collect_connectors(p, &pid, Some(&rels))?);
    }
    textbox::resolve_links(&mut all_boxes);
    let mut pic_count = 0usize;
    let mut chart_count = 0usize;
    let mut smart_art_count = 0usize;
    let mut all_pics = Vec::new();
    let mut charts_list = Vec::new();
    let mut smart_list = Vec::new();
    let mut listed_boxes = Vec::new();
    let mut listed_cxns = Vec::new();
    let mut para_nodes = Vec::new();
    for (i, p) in paras.iter().enumerate() {
        let index = i + 1;
        let (pid, _) = para::para_stable_id_in(container_id, p, index);
        let pics = picture::collect_pictures_with(p, &pid, Some(&rels))?;
        pic_count += pics.len();
        chart_count += charts::count_charts(p)?;
        smart_art_count += smartart::count_smartarts(p)?;
        let mine: Vec<_> = boxes_for_para(&all_boxes, &pid)
            .into_iter()
            .cloned()
            .collect();
        let mine_cxn: Vec<_> = cxns_for_para(&all_cxns, &pid)
            .into_iter()
            .cloned()
            .collect();
        let mut node = para_json(
            p,
            index,
            Some(container_id),
            Some(pics.len()),
            Some(mine.len()),
            Some(mine_cxn.len()),
            false,
        );
        if expand_pics {
            node["pictures"] = Value::Array(picture::pictures_json(&pics));
        }
        if expand_txbx {
            let json_boxes = textbox::textboxes_json(pkg, host_part, p, &pid, &mine, txbx)?;
            node["textBoxes"] = Value::Array(json_boxes.clone());
            listed_boxes.extend(json_boxes);
        }
        if expand_cxn {
            let json_cxns = connector::connectors_json(&mine_cxn);
            node["connectors"] = Value::Array(json_cxns.clone());
            listed_cxns.extend(json_cxns);
        }
        format::maybe_runs(&mut node, p, expand_runs);
        all_pics.extend(pics);
        if expand_charts {
            charts_list.extend(charts::collect_in_para(pkg, host_part, p, &pid)?);
        }
        if expand_smartart {
            let mut sa_index = 0usize;
            smart_list.extend(smartart::collect_in_para(
                pkg,
                host_part,
                p,
                &pid,
                &mut sa_index,
            )?);
        }
        para_nodes.push(node);
    }
    Ok(StoryMedia {
        para_count: paras.len(),
        pic_count,
        box_count: all_boxes.len(),
        cxn_count: all_cxns.len(),
        chart_count,
        smart_art_count,
        pictures: all_pics,
        text_boxes: listed_boxes,
        connectors: listed_cxns,
        charts: charts_list,
        smart_arts: smart_list,
        paragraphs: para_nodes,
    })
}

fn attach_story_lists(
    node: &mut Value,
    media: &StoryMedia,
    expand_pics: bool,
    expand_txbx: bool,
    expand_cxn: bool,
    expand_charts: bool,
    expand_smartart: bool,
) {
    if expand_pics {
        node["pictures"] = Value::Array(picture::pictures_json(&media.pictures));
    }
    if expand_txbx {
        node["textBoxes"] = Value::Array(media.text_boxes.clone());
    }
    if expand_cxn {
        node["connectors"] = Value::Array(media.connectors.clone());
    }
    if expand_charts {
        node["charts"] = Value::Array(media.charts.clone());
    }
    if expand_smartart {
        node["smartArts"] = Value::Array(media.smart_arts.clone());
    }
    node["paragraphs"] = Value::Array(media.paragraphs.clone());
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
                        | "connectors"
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

/// `json!` 键太多会撞递归上限，新增根字段按契约顺序插到已有键后面。
fn put_block_counts(node: &mut Value, root: &Element, after: &str) {
    let obj = node.as_object_mut().unwrap();
    let items = [
        ("tocCount", field::count_tocs(root)),
        ("captionCount", field::count_captions(root)),
        ("crossRefCount", field::count_cross_refs(root)),
        ("indexCount", field::count_indexes(root)),
        ("indexEntryCount", field::count_index_entries(root)),
        ("citationCount", field::count_citations(root)),
        ("bibliographyCount", field::count_bibliographies(root)),
    ];
    let mut anchor = after.to_string();
    for (key, n) in items {
        insert_json_after(obj, &anchor, key, json!(n));
        anchor = key.to_string();
    }
}

struct BlockFlags {
    tocs: bool,
    captions: bool,
    cross_refs: bool,
    indexes: bool,
    index_entries: bool,
    citations: bool,
    bibliographies: bool,
}

fn put_block_lists(
    node: &mut Value,
    root: &Element,
    container: &str,
    flags: &BlockFlags,
) -> Result<(), Error> {
    let id = Some(container);
    if flags.tocs {
        node["tocs"] = Value::Array(field::tocs_json(root, id)?);
    }
    if flags.captions {
        node["captions"] = Value::Array(field::captions_json(root, id)?);
    }
    if flags.cross_refs {
        node["crossRefs"] = Value::Array(field::cross_refs_json(root, id)?);
    }
    if flags.indexes {
        node["indexes"] = Value::Array(field::indexes_json(root, id)?);
    }
    if flags.index_entries {
        node["indexEntries"] = Value::Array(field::index_entries_json(root, id)?);
    }
    if flags.citations {
        node["citations"] = Value::Array(field::citations_json(root, id)?);
    }
    if flags.bibliographies {
        node["bibliographies"] = Value::Array(field::bibliographies_json(root, id)?);
    }
    Ok(())
}

fn insert_json_after(map: &mut Map<String, Value>, after: &str, key: &str, val: Value) {
    if !map.contains_key(after) {
        map.insert(key.into(), val);
        return;
    }
    let mut out = Map::new();
    let mut pending = Some(val);
    for (k, v) in std::mem::take(map) {
        let hit = k == after;
        out.insert(k, v);
        if hit {
            out.insert(key.into(), pending.take().expect("after 只出现一次"));
        }
    }
    *map = out;
}
