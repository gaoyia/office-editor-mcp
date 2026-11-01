//! 从领域 JSON 生成 Transitional 文档。

use serde_json::Value;

use crate::error::{Error, ErrorKind, IdReason, TypeName};
use crate::opc::{
    ContentTypes, Package, Relationship, Relationships, CONTENT_TYPES_PART, ROOT_RELS_PART,
};
use crate::xml::{self, Element, Node};

use super::apply;
use super::charts::{self, InsertChartOp};
use super::connector;
use super::hf::{HfKind, HfType};
use super::ink;
use super::ink::InsertInkCanvasOp;
use super::model3d;
use super::numbering::NumberingKind;
use super::ole::InsertOleOp;
use super::para::{self, ParaTarget};
use super::picture::{self, WrapPos};
use super::request::create_types::*;
use super::request::doc_extra::{self, DocExtra, DrawingHlink, TableExtra};
use super::request::para_remain::{self, GroupSpec};
use super::request::{
    DocxOp, InsertBibliographyOp, InsertBibliographyPlace, InsertBookmarkOp, InsertCaptionOp,
    InsertCaptionPlace, InsertCitationOp, InsertCommentOp, InsertConnectorOp,
    InsertContentControlOp, InsertCrossRefOp, InsertEditRangeOp, InsertFieldOp,
    InsertHyperlinkHost, InsertHyperlinkOp, InsertIndexEntryOp, InsertIndexOp, InsertIndexPlace,
    InsertMathOp, InsertMathPlace, InsertNoteOp, InsertNumberingOp, InsertRubyOp, InsertRubyPlace,
    InsertSdtPlace, InsertShapeOp, InsertTableOp, InsertTocOp, InsertTocPlace, ReplyCommentOp,
    SetBibliographyStyleOp, SetCellBordersOp, SetCellFormatOp, SetParaNumberingOp, SetTableAlignOp,
    SetTableBordersOp, SetTableCellMarOp, SetTableFloatOp, SetTableRowHeightOp, SetTableStyleOp,
    SetTableWidthOp,
};
use super::shape;
use super::smartart::{self};
use super::tbl::{self, TableRef, TableStory};
use super::{
    CT_APP, CT_CORE, CT_DOCUMENT, CT_FOOTER, CT_HEADER, CT_SETTINGS, CT_STYLES, REL_APP, REL_CORE,
    REL_FOOTER, REL_HEADER, REL_OFFICE_DOCUMENT, REL_SETTINGS, REL_STYLES,
};

pub fn create(document: &Value) -> Result<Vec<u8>, Error> {
    let obj = document.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "document".into(),
                expected: TypeName::Object,
            },
            "document",
        )
    })?;
    for k in obj.keys() {
        if !matches!(
            k.as_str(),
            "paragraphs"
                | "headers"
                | "footers"
                | "tables"
                | "props"
                | "protection"
                | "watermarks"
                | "numberings"
                | "tocs"
                | "theme"
                | "pageBackground"
                | "pageBorders"
                | "groups"
        ) && !doc_extra::ROOT_KEYS.contains(&k.as_str())
        {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("document.{k}"),
            ));
        }
    }
    let arr = obj.get("paragraphs").ok_or_else(|| {
        Error::with_path(
            ErrorKind::MissingField {
                field: "paragraphs".into(),
            },
            "document.paragraphs",
        )
    })?;
    let arr = arr.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "paragraphs".into(),
                expected: TypeName::Array,
            },
            "document.paragraphs",
        )
    })?;

    let mut paras = Vec::new();
    for (i, p) in arr.iter().enumerate() {
        paras.push(super::request::create_story::parse_para(
            p,
            &format!("document.paragraphs[{i}]"),
            ParaKind::Body,
            i,
        )?);
    }
    if let Some(last) = paras.last() {
        if last.section_type.is_some() {
            let i = paras.len() - 1;
            return Err(Error::with_path(
                ErrorKind::SectionTypeOnLastPara,
                format!("document.paragraphs[{i}].sectionType"),
            ));
        }
    }
    if paras.is_empty() {
        paras.push(NewPara::empty());
    }
    assign_story_docpr(&mut paras);
    assign_box_inner_ids(&mut paras);
    let extra = doc_extra::parse(obj, paras.len())?;
    let tables = super::request::create_table::parse_tables(
        obj.get("tables"),
        "document.tables",
        paras.len(),
    )?;
    let groups = para_remain::parse_groups(obj.get("groups"), "document.groups")?;
    let numberings = super::request::create_field::parse_numberings(
        obj.get("numberings"),
        "document.numberings",
    )?;
    let tocs =
        super::request::create_field::parse_tocs(obj.get("tocs"), "document.tocs", paras.len())?;

    let mut header_items =
        super::request::create_story::parse_hf_side(obj.get("headers"), "headers")?;
    let mut footer_items =
        super::request::create_story::parse_hf_side(obj.get("footers"), "footers")?;
    for item in &mut header_items {
        assign_story_docpr(&mut item.paras);
        assign_box_inner_ids(&mut item.paras);
    }
    for item in &mut footer_items {
        assign_story_docpr(&mut item.paras);
        assign_box_inner_ids(&mut item.paras);
    }
    let need_even = header_items.iter().any(|h| h.hf_type == HfType::Even)
        || footer_items.iter().any(|h| h.hf_type == HfType::Even);
    let need_first = header_items.iter().any(|h| h.hf_type == HfType::First)
        || footer_items.iter().any(|h| h.hf_type == HfType::First);

    let mut pkg = Package::create();
    let mut overrides = vec![
        ("/word/document.xml".into(), CT_DOCUMENT.into()),
        ("/word/styles.xml".into(), CT_STYLES.into()),
        ("/word/settings.xml".into(), CT_SETTINGS.into()),
        ("/docProps/core.xml".into(), CT_CORE.into()),
        ("/docProps/app.xml".into(), CT_APP.into()),
    ];
    for i in 0..header_items.len() {
        overrides.push((format!("/word/header{}.xml", i + 1), CT_HEADER.into()));
    }
    for i in 0..footer_items.len() {
        overrides.push((format!("/word/footer{}.xml", i + 1), CT_FOOTER.into()));
    }
    let types = ContentTypes {
        defaults: vec![
            (
                "rels".into(),
                "application/vnd.openxmlformats-package.relationships+xml".into(),
            ),
            ("xml".into(), "application/xml".into()),
        ],
        overrides,
    };

    let mut root_rels = Relationships { rels: Vec::new() };
    root_rels.push(Relationship {
        id: "rId1".into(),
        rel_type: REL_OFFICE_DOCUMENT.into(),
        target: "word/document.xml".into(),
        target_mode: None,
        extra: Vec::new(),
    });
    root_rels.push(Relationship {
        id: "rId2".into(),
        rel_type: REL_CORE.into(),
        target: "docProps/core.xml".into(),
        target_mode: None,
        extra: Vec::new(),
    });
    root_rels.push(Relationship {
        id: "rId3".into(),
        rel_type: REL_APP.into(),
        target: "docProps/app.xml".into(),
        target_mode: None,
        extra: Vec::new(),
    });

    let mut doc_rels = Relationships { rels: Vec::new() };
    doc_rels.push(Relationship {
        id: "rId1".into(),
        rel_type: REL_STYLES.into(),
        target: "styles.xml".into(),
        target_mode: None,
        extra: Vec::new(),
    });
    doc_rels.push(Relationship {
        id: "rId2".into(),
        rel_type: REL_SETTINGS.into(),
        target: "settings.xml".into(),
        target_mode: None,
        extra: Vec::new(),
    });
    let mut next_rid = 3u32;
    let mut hf_refs: Vec<(HfKind, HfType, String)> = Vec::new();
    for (i, item) in header_items.iter().enumerate() {
        let rid = format!("rId{next_rid}");
        next_rid += 1;
        doc_rels.push(Relationship {
            id: rid.clone(),
            rel_type: REL_HEADER.into(),
            target: format!("header{}.xml", i + 1),
            target_mode: None,
            extra: Vec::new(),
        });
        hf_refs.push((HfKind::Header, item.hf_type, rid));
    }
    for (i, item) in footer_items.iter().enumerate() {
        let rid = format!("rId{next_rid}");
        next_rid += 1;
        doc_rels.push(Relationship {
            id: rid.clone(),
            rel_type: REL_FOOTER.into(),
            target: format!("footer{}.xml", i + 1),
            target_mode: None,
            extra: Vec::new(),
        });
        hf_refs.push((HfKind::Footer, item.hf_type, rid));
    }

    pkg.set(CONTENT_TYPES_PART, types.to_xml().into_bytes())?;
    pkg.set(ROOT_RELS_PART, root_rels.to_xml().into_bytes())?;
    pkg.set(
        "/word/document.xml",
        super::create_xml::document_xml(&paras, &tables, &hf_refs, need_first).into_bytes(),
    )?;
    pkg.set(
        "/word/_rels/document.xml.rels",
        doc_rels.to_xml().into_bytes(),
    )?;
    for (i, item) in header_items.iter().enumerate() {
        let part = format!("/word/header{}.xml", i + 1);
        pkg.set(
            &part,
            super::create_xml::hf_xml("hdr", &item.paras, &item.tables).into_bytes(),
        )?;
    }
    for (i, item) in footer_items.iter().enumerate() {
        let part = format!("/word/footer{}.xml", i + 1);
        pkg.set(
            &part,
            super::create_xml::hf_xml("ftr", &item.paras, &item.tables).into_bytes(),
        )?;
    }
    pkg.set(
        "/word/styles.xml",
        super::create_xml::styles_xml().into_bytes(),
    )?;
    pkg.set(
        "/word/settings.xml",
        super::create_xml::settings_xml(need_even).into_bytes(),
    )?;
    pkg.set(
        "/docProps/core.xml",
        super::create_xml::core_xml().into_bytes(),
    )?;
    pkg.set(
        "/docProps/app.xml",
        super::create_xml::app_xml().into_bytes(),
    )?;
    crate::props::apply_create(&mut pkg, obj.get("props"), "document.props")?;

    if let Some(v) = obj.get("theme") {
        super::theme::write_create(&mut pkg, &super::theme::parse_create(v)?)?;
    }

    attach_pictures(&mut pkg, "/word/document.xml", &paras)?;
    attach_smartarts(&mut pkg, "/word/document.xml", &paras)?;
    for (i, item) in header_items.iter().enumerate() {
        let part = format!("/word/header{}.xml", i + 1);
        attach_pictures(&mut pkg, &part, &item.paras)?;
        attach_smartarts(&mut pkg, &part, &item.paras)?;
    }
    for (i, item) in footer_items.iter().enumerate() {
        let part = format!("/word/footer{}.xml", i + 1);
        attach_pictures(&mut pkg, &part, &item.paras)?;
        attach_smartarts(&mut pkg, &part, &item.paras)?;
    }

    let bytes = pkg.save_to_vec()?;
    let bytes = apply_follow_ups(
        bytes,
        &paras,
        &header_items,
        &footer_items,
        &numberings,
        &tocs,
        &extra,
        &tables,
        &groups,
    )?;
    let bytes = if obj.get("pageBackground").is_some() || obj.get("pageBorders").is_some() {
        let mut pkg = crate::opc::Package::open_bytes(bytes)?;
        super::page_bg::apply_create(&mut pkg, obj.get("pageBackground"), obj.get("pageBorders"))?;
        pkg.save_to_vec()?
    } else {
        bytes
    };
    if obj.get("protection").is_some() || obj.get("watermarks").is_some() {
        let mut pkg = crate::opc::Package::open_bytes(bytes)?;
        super::protect::apply_create(&mut pkg, obj.get("protection"))?;
        super::watermark::apply_create(&mut pkg, obj.get("watermarks"))?;
        pkg.save_to_vec()
    } else {
        Ok(bytes)
    }
}

fn attach_smartarts(pkg: &mut Package, part: &str, paras: &[NewPara]) -> Result<(), Error> {
    if !paras.iter().any(NewPara::has_smartarts) {
        return Ok(());
    }
    let mut doc = xml::parse(pkg.get(part)?)?;
    for (i, para) in paras.iter().enumerate() {
        for item in &para.smart_arts {
            smartart::attach_to_para(pkg, part, &mut doc.root, i, &item.spec)?;
        }
    }
    pkg.set(part, doc.to_bytes()?)?;
    Ok(())
}

fn attach_pictures(pkg: &mut Package, part: &str, paras: &[NewPara]) -> Result<(), Error> {
    if !paras.iter().any(NewPara::has_pictures) {
        return Ok(());
    }
    let mut doc = xml::parse(pkg.get(part)?)?;
    picture::ensure_drawing_ns(&mut doc.root);
    for (i, para) in paras.iter().enumerate() {
        for pic in &para.pictures {
            let added = picture::add_media(pkg, part, &pic.file, pic.cx, pic.cy)?;
            let doc_pr = pic.doc_pr;
            let p = nth_story_para_mut(&mut doc.root, i).ok_or_else(|| {
                Error::xml_detail(
                    crate::error::XmlIssue::Parse,
                    format!("{part} missing paragraph {i}"),
                )
            })?;
            picture::insert_prepared_drawing(
                p,
                doc_pr,
                &added.blip_rid,
                added.svg_rid.as_deref(),
                added.cx,
                added.cy,
                pic.kind,
                pic.wrap,
                &pic.pos,
            )?;
        }
    }
    pkg.set(part, doc.to_bytes()?)?;
    Ok(())
}

fn apply_follow_ups(
    bytes: Vec<u8>,
    paras: &[NewPara],
    headers: &[HfCreateItem],
    footers: &[HfCreateItem],
    numberings: &[NumberingKind],
    tocs: &[NewToc],
    extra: &DocExtra,
    tables: &[NewTable],
    groups: &[GroupSpec],
) -> Result<Vec<u8>, Error> {
    let need = !numberings.is_empty()
        || !tocs.is_empty()
        || !doc_extra::is_empty(extra)
        || !groups.is_empty()
        || paras.iter().any(NewPara::has_follow_ups)
        || headers.iter().any(|h| {
            h.paras.iter().any(NewPara::has_follow_ups)
                || h.tables.iter().any(|t| t.extra.has_follow_ups())
                || !h.groups.is_empty()
        })
        || footers.iter().any(|h| {
            h.paras.iter().any(NewPara::has_follow_ups)
                || h.tables.iter().any(|t| t.extra.has_follow_ups())
                || !h.groups.is_empty()
        })
        || tables.iter().any(|t| t.extra.has_follow_ups());
    if !need {
        return Ok(bytes);
    }
    let mut ops = Vec::new();
    for s in &extra.styles {
        ops.push(DocxOp::InsertStyle(s.clone()));
    }
    for f in &extra.fonts {
        ops.push(DocxOp::InsertEmbeddedFont(f.clone()));
    }
    if let Some(fe) = extra.font_embed.clone() {
        ops.push(DocxOp::SetFontEmbed(fe));
    }
    if let Some(ea) = extra.east_asia.clone() {
        ops.push(DocxOp::SetEastAsianSettings(ea));
    }
    for s in &extra.sources {
        ops.push(DocxOp::InsertSource(s.clone()));
    }
    if let Some(name) = extra.bibliography_style.clone() {
        ops.push(DocxOp::SetBibliographyStyle(SetBibliographyStyleOp {
            style_name: name,
        }));
    }
    for kind in numberings {
        ops.push(DocxOp::InsertNumbering(InsertNumberingOp { kind: *kind }));
    }
    let mut comment_seq: i64 = 0;
    push_story_ops(&mut ops, paras, None, numberings.len(), &mut comment_seq)?;
    for item in headers {
        push_story_ops(
            &mut ops,
            &item.paras,
            Some((HfKind::Header, item.hf_type)),
            numberings.len(),
            &mut comment_seq,
        )?;
    }
    for item in footers {
        push_story_ops(
            &mut ops,
            &item.paras,
            Some((HfKind::Footer, item.hf_type)),
            numberings.len(),
            &mut comment_seq,
        )?;
    }
    for toc in tocs {
        let (id, place) = match toc.after {
            None => ("docx:document".to_string(), InsertTocPlace::EndOfDocument),
            Some(n) => {
                let id = story_para_id(None, (n as usize) - 1);
                let host = para::parse_para_id_at(&id, "document.tocs")?;
                let ParaTarget::Body(p) = host else {
                    return Err(Error::with_path(
                        ErrorKind::InvalidId {
                            reason: IdReason::InsertTocNeedsBodyPara,
                        },
                        "document.tocs",
                    ));
                };
                (id, InsertTocPlace::After(ParaTarget::Body(p)))
            }
        };
        ops.push(DocxOp::InsertToc(InsertTocOp {
            id,
            place,
            instr: toc.instr.clone(),
        }));
    }
    push_after_blocks(
        &mut ops,
        &extra.captions,
        |block, id, place| {
            DocxOp::InsertCaption(InsertCaptionOp {
                id,
                place,
                label: block.label.clone(),
                title: block.title.clone(),
                bookmark: block.bookmark.clone(),
            })
        },
        InsertCaptionPlace::EndOfDocument,
        InsertCaptionPlace::After,
        "document.captions",
    )?;
    push_instr_blocks(
        &mut ops,
        &extra.indexes,
        |block, id, place| {
            DocxOp::InsertIndex(InsertIndexOp {
                id,
                place,
                instr: block.instr.clone(),
            })
        },
        InsertIndexPlace::EndOfDocument,
        InsertIndexPlace::After,
        "document.indexes",
    )?;
    push_instr_blocks(
        &mut ops,
        &extra.bibliographies,
        |block, id, place| {
            DocxOp::InsertBibliography(InsertBibliographyOp {
                id,
                place,
                instr: block.instr.clone(),
            })
        },
        InsertBibliographyPlace::EndOfDocument,
        InsertBibliographyPlace::After,
        "document.bibliographies",
    )?;
    for p in &extra.doc_parts {
        ops.push(DocxOp::InsertDocPart(p.clone()));
    }
    push_table_ops(&mut ops, tables, None)?;
    for item in headers {
        push_table_ops(&mut ops, &item.tables, Some((HfKind::Header, item.hf_type)))?;
    }
    for item in footers {
        push_table_ops(&mut ops, &item.tables, Some((HfKind::Footer, item.hf_type)))?;
    }
    push_group_ops(&mut ops, groups, "docx:document", "document.groups")?;
    for item in headers {
        let hid = format!("header:{}:1", item.hf_type.as_str());
        push_group_ops(
            &mut ops,
            &item.groups,
            &hid,
            &format!("document.headers/{}/groups", item.hf_type.as_str()),
        )?;
    }
    for item in footers {
        let hid = format!("footer:{}:1", item.hf_type.as_str());
        push_group_ops(
            &mut ops,
            &item.groups,
            &hid,
            &format!("document.footers/{}/groups", item.hf_type.as_str()),
        )?;
    }
    apply::apply(&bytes, &ops)
}

fn push_after_blocks(
    ops: &mut Vec<DocxOp>,
    blocks: &[doc_extra::CaptionBlock],
    build: impl Fn(&doc_extra::CaptionBlock, String, InsertCaptionPlace) -> DocxOp,
    end: InsertCaptionPlace,
    after: fn(ParaTarget) -> InsertCaptionPlace,
    path: &str,
) -> Result<(), Error> {
    for block in blocks {
        let (id, place) = match block.after {
            None => ("docx:document".to_string(), end.clone()),
            Some(n) => {
                let id = story_para_id(None, (n as usize) - 1);
                let host = para::parse_para_id_at(&id, path)?;
                let ParaTarget::Body(p) = host else {
                    return Err(Error::with_path(
                        ErrorKind::InvalidId {
                            reason: IdReason::InsertTocNeedsBodyPara,
                        },
                        path,
                    ));
                };
                (id, after(ParaTarget::Body(p)))
            }
        };
        ops.push(build(block, id, place));
    }
    Ok(())
}

fn push_instr_blocks<P: Clone>(
    ops: &mut Vec<DocxOp>,
    blocks: &[doc_extra::AfterBlock],
    build: impl Fn(&doc_extra::AfterBlock, String, P) -> DocxOp,
    end: P,
    after: fn(ParaTarget) -> P,
    path: &str,
) -> Result<(), Error> {
    for block in blocks {
        let (id, place) = match block.after {
            None => ("docx:document".to_string(), end.clone()),
            Some(n) => {
                let id = story_para_id(None, (n as usize) - 1);
                let host = para::parse_para_id_at(&id, path)?;
                let ParaTarget::Body(p) = host else {
                    return Err(Error::with_path(
                        ErrorKind::InvalidId {
                            reason: IdReason::InsertTocNeedsBodyPara,
                        },
                        path,
                    ));
                };
                (id, after(ParaTarget::Body(p)))
            }
        };
        ops.push(build(block, id, place));
    }
    Ok(())
}

fn push_drawing_hlink(
    ops: &mut Vec<DocxOp>,
    id: String,
    host: InsertHyperlinkHost,
    h: &DrawingHlink,
) {
    ops.push(DocxOp::InsertHyperlink(InsertHyperlinkOp {
        id,
        host,
        url: h.url.clone(),
        anchor: h.anchor.clone(),
        tooltip: h.tooltip.clone(),
    }));
}

fn push_story_ops(
    ops: &mut Vec<DocxOp>,
    paras: &[NewPara],
    hf: Option<(HfKind, HfType)>,
    numbering_n: usize,
    comment_seq: &mut i64,
) -> Result<(), Error> {
    for (i, p) in paras.iter().enumerate() {
        let id = story_para_id(hf, i);
        let host = para::parse_para_id_at(&id, "document")?;
        for (n, ch) in p.charts.iter().enumerate() {
            ops.push(DocxOp::InsertChart(InsertChartOp {
                id: id.clone(),
                para: host.clone(),
                chart_type: ch.chart_type,
                of_pie_type: ch.of_pie_type.clone(),
                extra: ch.extra.clone(),
                title: ch.title.clone(),
                series: ch.series.clone(),
                kind: ch.kind,
                wrap: ch.wrap,
                pos: WrapPos::default(),
                cx: ch.cx,
                cy: ch.cy,
            }));
            if let Some(h) = &ch.hyperlink {
                let chart_id = format!("{id}/chart:index:{}", n + 1);
                let t = charts::parse_chart_id_at(&chart_id, "document")?;
                push_drawing_hlink(ops, chart_id, InsertHyperlinkHost::Chart(t), h);
            }
        }
        for file in &p.ole_objects {
            ops.push(DocxOp::InsertOle(InsertOleOp {
                id: id.clone(),
                para: host.clone(),
                file: file.clone(),
            }));
        }
        if !p.ink_canvases.is_empty() {
            let ink_host = para::parse_host_para_id_at(&id, "document")?;
            for (n, ink_item) in p.ink_canvases.iter().enumerate() {
                ops.push(DocxOp::InsertInkCanvas(InsertInkCanvasOp {
                    id: id.clone(),
                    host: ink_host.clone(),
                    file: ink_item.file.clone(),
                    cx: ink_item.cx,
                    cy: ink_item.cy,
                }));
                if let Some(h) = &ink_item.hyperlink {
                    let ink_id = format!("{id}/ink:index:{}", n + 1);
                    let t = ink::parse_ink_id_at(&ink_id, "document")?;
                    push_drawing_hlink(ops, ink_id, InsertHyperlinkHost::Ink(t), h);
                }
            }
        }
        if !p.model3ds.is_empty() {
            let host3d = para::parse_host_para_id_at(&id, "document")?;
            for (n, m) in p.model3ds.iter().enumerate() {
                ops.push(DocxOp::InsertModel3d(model3d::InsertModel3dOp {
                    id: id.clone(),
                    host: host3d.clone(),
                    file: m.file.clone(),
                    cx: m.cx,
                    cy: m.cy,
                    kind: m.kind,
                    wrap: m.wrap,
                    pos: m.pos.clone(),
                }));
                if let Some(h) = &m.hyperlink {
                    let m_id = format!("{id}/model3d:index:{}", n + 1);
                    let t = model3d::parse_id_at(&m_id, "document")?;
                    push_drawing_hlink(ops, m_id, InsertHyperlinkHost::Model3d(t), h);
                }
            }
        }
        if let Some(text) = &p.footnote {
            ops.push(DocxOp::InsertFootnote(InsertNoteOp {
                id: id.clone(),
                host: host.clone(),
                text: text.clone(),
            }));
        }
        if let Some(text) = &p.endnote {
            ops.push(DocxOp::InsertEndnote(InsertNoteOp {
                id: id.clone(),
                host: host.clone(),
                text: text.clone(),
            }));
        }
        for c in &p.comments {
            ops.push(DocxOp::InsertComment(InsertCommentOp {
                start: id.clone(),
                end: id.clone(),
                start_target: host.clone(),
                end_target: host.clone(),
                text: c.text.clone(),
                author: c.author.clone(),
                initials: c.initials.clone(),
                date: c.date.clone(),
            }));
            let parent_id = comment_seq.to_string();
            *comment_seq += 1;
            for r in &c.replies {
                ops.push(DocxOp::ReplyComment(ReplyCommentOp {
                    id: format!("comment:{parent_id}"),
                    comment_id: parent_id.clone(),
                    text: r.text.clone(),
                    author: r.author.clone(),
                    initials: r.initials.clone(),
                    date: r.date.clone(),
                }));
                *comment_seq += 1;
            }
        }
        if let Some(op) = para_remain::to_set_para_format(id.clone(), host.clone(), &p.remain) {
            ops.push(DocxOp::SetParaFormat(op));
        }
        if let Some(runs) = &p.runs {
            for (ri, r) in runs.iter().enumerate() {
                if let Some(fmt) = &r.format {
                    let run_id = format!("{id}/run:{}", ri + 1);
                    ops.push(DocxOp::SetRunFormat(para_remain::to_set_run_format(
                        run_id,
                        host.clone(),
                        ri + 1,
                        fmt,
                    )));
                }
            }
        }
        for b in &p.boxes {
            push_box_follow_ups(ops, &id, b, &story_field_path(hf, i, "textBoxes"))?;
        }
        for bm in &p.bookmarks {
            let end_id = match bm.end {
                None => id.clone(),
                Some(n) => {
                    let idx = n as usize;
                    if n < (i as u32 + 1) {
                        return Err(Error::with_path(
                            ErrorKind::BookmarkRangeReversed,
                            story_field_path(hf, i, "bookmarks"),
                        ));
                    }
                    if idx > paras.len() {
                        return Err(Error::with_path(
                            ErrorKind::CreateIndexOutOfRange {
                                field: "end".into(),
                                max: paras.len() as i64,
                            },
                            story_field_path(hf, i, "bookmarks"),
                        ));
                    }
                    story_para_id(hf, idx - 1)
                }
            };
            let end = para::parse_para_id_at(&end_id, "document")?;
            ops.push(DocxOp::InsertBookmark(InsertBookmarkOp {
                id: id.clone(),
                start: host.clone(),
                end,
                name: bm.name.clone(),
            }));
        }
        for h in &p.hyperlinks {
            ops.push(DocxOp::InsertHyperlink(InsertHyperlinkOp {
                id: id.clone(),
                host: InsertHyperlinkHost::Para {
                    para: host.clone(),
                    text: h.text.clone(),
                },
                url: h.url.clone(),
                anchor: h.anchor.clone(),
                tooltip: h.tooltip.clone(),
            }));
        }
        for f in &p.fields {
            ops.push(DocxOp::InsertField(InsertFieldOp {
                id: id.clone(),
                para: host.clone(),
                instr: f.instr.clone(),
                result: f.result.clone(),
                form: f.form.clone(),
            }));
        }
        for m in &p.maths {
            ops.push(DocxOp::InsertMath(InsertMathOp {
                id: id.clone(),
                place: InsertMathPlace::After(host.clone()),
                kind: m.kind,
                body: m.body.clone(),
                jc: m.jc.clone(),
            }));
        }
        for s in &p.content_controls {
            ops.push(DocxOp::InsertContentControl(InsertContentControlOp {
                id: id.clone(),
                place: InsertSdtPlace::After(host.clone()),
                binding: None,
                kind: s.kind,
                text: s.text.clone(),
                tag: s.tag.clone(),
                alias: s.alias.clone(),
                items: s.items.clone(),
                full_date: s.full_date.clone(),
                file: s.file.clone(),
                checked: s.checked,
            }));
        }
        if !p.shapes.is_empty() || !p.connectors.is_empty() {
            let shape_host = para::parse_host_para_id_at(&id, "document")?;
            for (n, s) in p.shapes.iter().enumerate() {
                ops.push(DocxOp::InsertShape(InsertShapeOp {
                    id: id.clone(),
                    host: shape_host.clone(),
                    cx: s.cx,
                    cy: s.cy,
                    prst: s.prst.clone(),
                }));
                if let Some(h) = &s.hyperlink {
                    let sid = format!("{id}/shape:index:{}", n + 1);
                    let t = shape::parse_shape_id_at(&sid, "document")?;
                    push_drawing_hlink(ops, sid, InsertHyperlinkHost::Shape(t), h);
                }
            }
            for (n, s) in p.connectors.iter().enumerate() {
                ops.push(DocxOp::InsertConnector(InsertConnectorOp {
                    id: id.clone(),
                    host: shape_host.clone(),
                    cx: s.cx,
                    cy: s.cy,
                    prst: s.prst.clone(),
                }));
                if let Some(h) = &s.hyperlink {
                    let cid = format!("{id}/cxn:index:{}", n + 1);
                    let t = connector::parse_cxn_id_at(&cid, "document")?;
                    push_drawing_hlink(ops, cid, InsertHyperlinkHost::Connector(t), h);
                }
            }
        }
        for r in &p.rubies {
            ops.push(DocxOp::InsertRuby(InsertRubyOp {
                id: id.clone(),
                place: InsertRubyPlace::After(host.clone()),
                base: r.base.clone(),
                rt: r.rt.clone(),
                align: r.align.clone(),
                hps: r.hps,
                hps_raise: r.hps_raise,
                hps_base_text: r.hps_base_text,
                lid: r.lid.clone(),
            }));
        }
        for e in &p.index_entries {
            ops.push(DocxOp::InsertIndexEntry(InsertIndexEntryOp {
                id: id.clone(),
                para: host.clone(),
                entry: e.entry.clone(),
            }));
        }
        for c in &p.citations {
            ops.push(DocxOp::InsertCitation(InsertCitationOp {
                id: id.clone(),
                para: host.clone(),
                tag: c.tag.clone(),
            }));
        }
        for c in &p.cross_refs {
            ops.push(DocxOp::InsertCrossRef(InsertCrossRefOp {
                id: id.clone(),
                para: host.clone(),
                bookmark: c.bookmark.clone(),
                kind: c.kind,
            }));
        }
        if let Some(er) = &p.edit_range {
            ops.push(DocxOp::InsertEditRange(InsertEditRangeOp {
                id: id.clone(),
                para: host.clone(),
                ed_grp: er.ed_grp.clone(),
                ed: er.ed.clone(),
            }));
        }
        if let Some(n) = p.num {
            if n as usize > numbering_n {
                return Err(Error::with_path(
                    ErrorKind::CreateIndexOutOfRange {
                        field: "num".into(),
                        max: numbering_n as i64,
                    },
                    story_field_path(hf, i, "num"),
                ));
            }
            ops.push(DocxOp::SetParaNumbering(SetParaNumberingOp {
                id: id.clone(),
                para: host.clone(),
                num: Some(n),
                ilvl: p.ilvl,
            }));
        }
        for pic in &p.pictures {
            if let Some(h) = &pic.hyperlink {
                let pic_id = format!("{id}/pic:{}", pic.doc_pr);
                let pic_t = picture::parse_pic_id_at(&pic_id, "document")?;
                push_drawing_hlink(ops, pic_id, InsertHyperlinkHost::Picture(pic_t), h);
            }
        }
        for b in &p.boxes {
            if let Some(h) = &b.hyperlink {
                let box_id = format!("{id}/txbx:{}", b.doc_pr);
                let box_t = para::parse_txbx_id_at(&box_id, "document")?;
                push_drawing_hlink(ops, box_id, InsertHyperlinkHost::TextBox(box_t), h);
            }
        }
        for (n, sa) in p.smart_arts.iter().enumerate() {
            if let Some(h) = &sa.hyperlink {
                let sa_id = format!("{id}/smartdiagram:index:{}", n + 1);
                let t = smartart::parse_diagram_id_at(&sa_id, "document")?;
                push_drawing_hlink(ops, sa_id, InsertHyperlinkHost::SmartArt(t), h);
            }
        }
    }
    Ok(())
}

fn push_group_ops(
    ops: &mut Vec<DocxOp>,
    groups: &[GroupSpec],
    host_id: &str,
    path: &str,
) -> Result<(), Error> {
    for (i, g) in groups.iter().enumerate() {
        let item_path = format!("{path}[{i}]");
        ops.push(DocxOp::GroupShapes(para_remain::to_group_shapes(
            host_id, g, &item_path,
        )?));
    }
    Ok(())
}

fn push_box_follow_ups(
    ops: &mut Vec<DocxOp>,
    host_para_id: &str,
    b: &NewBox,
    path: &str,
) -> Result<(), Error> {
    let box_id = format!("{host_para_id}/txbx:{}", b.doc_pr);
    let inner_id = format!("{box_id}/para:{:08X}", b.inner_para);
    let inner_host = para::parse_para_id_at(&inner_id, path)?;
    if !b.tables.is_empty() {
        let tx = para::parse_txbx_id_at(&box_id, path)?;
        for (ti, t) in b.tables.iter().enumerate() {
            ops.push(DocxOp::InsertTable(InsertTableOp {
                id: box_id.clone(),
                anchor: tbl::InsertAnchor::TextBoxEnd {
                    host: tx.host.clone(),
                    key: tx.key.clone(),
                },
                cells: t.cells.clone(),
            }));
            let tbl_id = format!("{box_id}/table:{}", ti + 1);
            let target = TableRef::Table {
                story: TableStory::TextBox {
                    host: tx.host.clone(),
                    key: tx.key.clone(),
                },
                n: ti + 1,
            };
            push_one_table_extra(ops, &t.extra, tbl_id, target);
        }
    }
    for bm in &b.bookmarks {
        let end_id = match bm.end {
            None => inner_id.clone(),
            Some(n) => {
                if n < 1 {
                    return Err(Error::with_path(
                        ErrorKind::BookmarkRangeReversed,
                        format!("{path}.bookmarks"),
                    ));
                }
                if n > 1 {
                    return Err(Error::with_path(
                        ErrorKind::CreateIndexOutOfRange {
                            field: "end".into(),
                            max: 1,
                        },
                        format!("{path}.bookmarks"),
                    ));
                }
                inner_id.clone()
            }
        };
        let end = para::parse_para_id_at(&end_id, path)?;
        ops.push(DocxOp::InsertBookmark(InsertBookmarkOp {
            id: inner_id.clone(),
            start: inner_host.clone(),
            end,
            name: bm.name.clone(),
        }));
    }
    for h in &b.hyperlinks {
        ops.push(DocxOp::InsertHyperlink(InsertHyperlinkOp {
            id: inner_id.clone(),
            host: InsertHyperlinkHost::Para {
                para: inner_host.clone(),
                text: h.text.clone(),
            },
            url: h.url.clone(),
            anchor: h.anchor.clone(),
            tooltip: h.tooltip.clone(),
        }));
    }
    for f in &b.fields {
        ops.push(DocxOp::InsertField(InsertFieldOp {
            id: inner_id.clone(),
            para: inner_host.clone(),
            instr: f.instr.clone(),
            result: f.result.clone(),
            form: f.form.clone(),
        }));
    }
    for m in &b.maths {
        ops.push(DocxOp::InsertMath(InsertMathOp {
            id: inner_id.clone(),
            place: InsertMathPlace::After(inner_host.clone()),
            kind: m.kind,
            body: m.body.clone(),
            jc: m.jc.clone(),
        }));
    }
    for s in &b.content_controls {
        ops.push(DocxOp::InsertContentControl(InsertContentControlOp {
            id: inner_id.clone(),
            place: InsertSdtPlace::After(inner_host.clone()),
            binding: None,
            kind: s.kind,
            text: s.text.clone(),
            tag: s.tag.clone(),
            alias: s.alias.clone(),
            items: s.items.clone(),
            full_date: s.full_date.clone(),
            file: s.file.clone(),
            checked: s.checked,
        }));
    }
    for r in &b.rubies {
        ops.push(DocxOp::InsertRuby(InsertRubyOp {
            id: inner_id.clone(),
            place: InsertRubyPlace::After(inner_host.clone()),
            base: r.base.clone(),
            rt: r.rt.clone(),
            align: r.align.clone(),
            hps: r.hps,
            hps_raise: r.hps_raise,
            hps_base_text: r.hps_base_text,
            lid: r.lid.clone(),
        }));
    }
    Ok(())
}

fn assign_story_docpr(paras: &mut [NewPara]) {
    let mut n = 1u32;
    for p in paras.iter_mut() {
        for b in &mut p.boxes {
            b.doc_pr = n;
            n += 1;
        }
    }
    for p in paras.iter_mut() {
        for pic in &mut p.pictures {
            pic.doc_pr = n;
            n += 1;
        }
    }
}

fn assign_box_inner_ids(paras: &mut [NewPara]) {
    let mut n = paras.len() as u32 + 1;
    for p in paras {
        for b in &mut p.boxes {
            b.inner_para = n;
            n = n.saturating_add(1);
        }
    }
}

fn story_table_id(hf: Option<(HfKind, HfType)>, index: usize) -> String {
    match hf {
        None => format!("table:{}", index + 1),
        Some((kind, hf_type)) => format!(
            "{}:{}:1/table:{}",
            kind.as_str(),
            hf_type.as_str(),
            index + 1
        ),
    }
}

fn story_table_ref(hf: Option<(HfKind, HfType)>, index: usize) -> TableRef {
    let n = index + 1;
    let story = match hf {
        None => TableStory::Body,
        Some((kind, hf_type)) => TableStory::HeaderFooter {
            kind,
            hf_type,
            n: 1,
        },
    };
    TableRef::Table { story, n }
}

fn row_ref_from_table(target: &TableRef, row: usize) -> TableRef {
    match target {
        TableRef::Table { story, n } => TableRef::Row {
            story: story.clone(),
            n: *n,
            row,
        },
        other => other.clone(),
    }
}

fn push_one_table_extra(ops: &mut Vec<DocxOp>, extra: &TableExtra, id: String, target: TableRef) {
    if extra.style.is_some() || extra.look.is_some() {
        ops.push(DocxOp::SetTableStyle(SetTableStyleOp {
            id: id.clone(),
            target: target.clone(),
            style: extra.style.clone().map(Some),
            look: extra.look.clone().map(Some),
        }));
    }
    if let Some(pos) = extra.pos.clone() {
        ops.push(DocxOp::SetTableFloat(SetTableFloatOp {
            id: id.clone(),
            target: target.clone(),
            pos: Some(pos),
        }));
    }
    if let Some(patch) = extra.width.clone() {
        ops.push(DocxOp::SetTableWidth(SetTableWidthOp {
            id: id.clone(),
            target: target.clone(),
            patch,
        }));
    }
    if extra.align.is_some() || extra.indent.is_some() {
        ops.push(DocxOp::SetTableAlign(SetTableAlignOp {
            id: id.clone(),
            target: target.clone(),
            align: extra.align.clone().map(Some),
            indent: extra.indent.map(Some),
        }));
    }
    if let Some(mar) = &extra.cell_mar {
        ops.push(DocxOp::SetTableCellMar(SetTableCellMarOp {
            id: id.clone(),
            target: target.clone(),
            top: mar.top,
            left: mar.left,
            bottom: mar.bottom,
            right: mar.right,
        }));
    }
    if let Some(borders) = &extra.borders {
        ops.push(DocxOp::SetTableBorders(SetTableBordersOp {
            id: id.clone(),
            target: target.clone(),
            borders: borders.clone(),
        }));
    }
    for (ri, rh) in extra.row_heights.iter().enumerate() {
        let row = ri + 1;
        ops.push(DocxOp::SetTableRowHeight(SetTableRowHeightOp {
            id: format!("{id}/row:{row}"),
            target: row_ref_from_table(&target, row),
            h: rh.h,
            rule: rh.rule.clone(),
        }));
    }
    for cf in &extra.cell_formats {
        let cell_id = format!("{id}/r{}c{}", cf.r, cf.c);
        let cell_target = match &target {
            TableRef::Table { story, n } => TableRef::Cell {
                story: story.clone(),
                n: *n,
                row: cf.r as usize,
                col: cf.c as usize,
            },
            other => other.clone(),
        };
        let has_fmt = cf.fill.is_some()
            || cf.top.is_some()
            || cf.left.is_some()
            || cf.bottom.is_some()
            || cf.right.is_some()
            || cf.tl2br.is_some()
            || cf.tr2bl.is_some()
            || cf.v_align.is_some()
            || cf.no_wrap.is_some()
            || cf.text_direction.is_some()
            || cf.fit_text.is_some()
            || cf.hide_mark.is_some()
            || cf.headers.is_some();
        if has_fmt {
            ops.push(DocxOp::SetCellFormat(SetCellFormatOp {
                id: cell_id.clone(),
                target: cell_target.clone(),
                fill: cf.fill.clone(),
                top: cf.top,
                left: cf.left,
                bottom: cf.bottom,
                right: cf.right,
                tl2br: cf.tl2br.clone(),
                tr2bl: cf.tr2bl.clone(),
                v_align: cf.v_align.clone(),
                no_wrap: cf.no_wrap,
                text_direction: cf.text_direction.clone(),
                fit_text: cf.fit_text,
                hide_mark: cf.hide_mark,
                headers: cf.headers.clone(),
            }));
        }
        if let Some(borders) = &cf.borders {
            ops.push(DocxOp::SetCellBorders(SetCellBordersOp {
                id: cell_id,
                target: cell_target,
                borders: borders.clone(),
            }));
        }
    }
}

fn push_table_ops(
    ops: &mut Vec<DocxOp>,
    tables: &[NewTable],
    hf: Option<(HfKind, HfType)>,
) -> Result<(), Error> {
    for (i, t) in tables.iter().enumerate() {
        let id = story_table_id(hf, i);
        let target = story_table_ref(hf, i);
        push_one_table_extra(ops, &t.extra, id, target);
    }
    Ok(())
}

fn story_para_id(hf: Option<(HfKind, HfType)>, index: usize) -> String {
    let hex = format!("{:08X}", index as u32 + 1);
    match hf {
        None => format!("para:{hex}"),
        Some((kind, hf_type)) => format!("{}:{}:1/para:{hex}", kind.as_str(), hf_type.as_str()),
    }
}

fn story_field_path(hf: Option<(HfKind, HfType)>, index: usize, field: &str) -> String {
    match hf {
        None => format!("document.paragraphs[{index}].{field}"),
        Some((kind, hf_type)) => {
            let side = match kind {
                HfKind::Header => "headers",
                HfKind::Footer => "footers",
            };
            format!(
                "document.{side}/{}/paragraphs[{index}].{field}",
                hf_type.as_str()
            )
        }
    }
}

/// 只数故事层的 `w:p`，不走进文本框。
fn nth_story_para_mut(root: &mut Element, index: usize) -> Option<&mut Element> {
    let parent = if root.local_name() == "document" {
        root.child_by_local_mut("body")?
    } else {
        root
    };
    let mut n = 0usize;
    for child in parent.children.iter_mut() {
        let Node::Element(e) = child else {
            continue;
        };
        if e.local_name() != "p" {
            continue;
        }
        if n == index {
            return Some(e);
        }
        n += 1;
    }
    None
}
