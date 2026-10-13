//! 从领域 JSON 生成 Transitional 文档。

use std::collections::HashSet;

use serde_json::Value;

use crate::error::{Error, ErrorKind, IdReason, TypeName};
use crate::opc::{
    ContentTypes, Package, Relationship, Relationships, CONTENT_TYPES_PART, ROOT_RELS_PART,
};
use crate::xml::{self, Element, Node};

use super::apply;
use super::charts::{self, ChartType, InsertChartOp, NewSeries};
use super::connector;
use super::create_round4::{
    self, CitationSpec, CrossRefSpec, DocExtra, DrawingHlink, EditRangeSpec, IndexEntrySpec,
    RubySpec, ShapeSpec, TableExtra,
};
use super::create_round5::{self, GroupSpec, ParaRemain, RunSpec};
use super::field;
use super::hf::{HfKind, HfType};
use super::ink;
use super::ink::InsertInkCanvasOp;
use super::math::MathKind;
use super::math_expr::{self, MathBody};
use super::model3d;
use super::numbering::NumberingKind;
use super::ole::InsertOleOp;
use super::para::{self, ParaTarget};
use super::picture::{self, PicKind, WrapKind, WrapPos};
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
use super::sdt::SdtKind;
use super::sect::SectionBreakType;
use super::shape;
use super::smartart::{self, InsertSpec};
use super::tbl::{self, TableRef, TableStory};
use super::textbox::{self, DEFAULT_CX, DEFAULT_CY, WPS_NS};
use super::{
    A_NS, CT_APP, CT_CORE, CT_DOCUMENT, CT_FOOTER, CT_HEADER, CT_SETTINGS, CT_STYLES, MC_NS,
    PIC_NS, REL_APP, REL_CORE, REL_FOOTER, REL_HEADER, REL_NS, REL_OFFICE_DOCUMENT, REL_SETTINGS,
    REL_STYLES, W14_NS, WML_NS, WP_NS,
};

struct NewBox {
    text: String,
    cx: i64,
    cy: i64,
    doc_pr: u32,
    inner_para: u32,
    hyperlink: Option<DrawingHlink>,
    tables: Vec<NewTable>,
    hyperlinks: Vec<NewHyperlink>,
    bookmarks: Vec<NewBookmark>,
    fields: Vec<NewField>,
    maths: Vec<NewMath>,
    content_controls: Vec<NewSdt>,
    rubies: Vec<RubySpec>,
}

impl NewBox {
    fn has_follow_ups(&self) -> bool {
        self.hyperlink.is_some()
            || !self.tables.is_empty()
            || !self.hyperlinks.is_empty()
            || !self.bookmarks.is_empty()
            || !self.fields.is_empty()
            || !self.maths.is_empty()
            || !self.content_controls.is_empty()
            || !self.rubies.is_empty()
    }
}

struct NewPic {
    file: String,
    cx: Option<i64>,
    cy: Option<i64>,
    kind: PicKind,
    wrap: Option<WrapKind>,
    pos: WrapPos,
    doc_pr: u32,
    hyperlink: Option<DrawingHlink>,
}

struct CreateSmartArt {
    spec: InsertSpec,
    hyperlink: Option<DrawingHlink>,
}

struct CreateChart {
    chart_type: ChartType,
    of_pie_type: Option<String>,
    extra: crate::chart::ChartExtra,
    title: Option<String>,
    series: Vec<NewSeries>,
    kind: PicKind,
    wrap: Option<WrapKind>,
    cx: i64,
    cy: i64,
    hyperlink: Option<DrawingHlink>,
}

struct NewComment {
    text: String,
    author: String,
    initials: String,
    date: Option<String>,
    replies: Vec<NewComment>,
}

struct NewHyperlink {
    text: String,
    url: Option<String>,
    anchor: Option<String>,
    tooltip: Option<String>,
}

struct NewBookmark {
    name: Option<String>,
    end: Option<u32>,
}

struct NewField {
    instr: String,
    result: Option<String>,
    form: Option<super::field::FormWrite>,
}

struct NewMath {
    body: MathBody,
    kind: MathKind,
    jc: Option<String>,
}

struct NewSdt {
    text: String,
    kind: SdtKind,
    tag: Option<String>,
    alias: Option<String>,
    items: Vec<String>,
    full_date: Option<String>,
    file: Option<String>,
    checked: bool,
}

struct NewToc {
    after: Option<u32>,
    instr: String,
}

struct NewInkCanvas {
    file: String,
    cx: Option<i64>,
    cy: Option<i64>,
    hyperlink: Option<DrawingHlink>,
}

struct NewModel3d {
    file: String,
    cx: Option<i64>,
    cy: Option<i64>,
    hyperlink: Option<DrawingHlink>,
}

struct NewPara {
    text: String,
    style: Option<&'static str>,
    boxes: Vec<NewBox>,
    pictures: Vec<NewPic>,
    smart_arts: Vec<CreateSmartArt>,
    charts: Vec<CreateChart>,
    ole_objects: Vec<String>,
    ink_canvases: Vec<NewInkCanvas>,
    model3ds: Vec<NewModel3d>,
    footnote: Option<String>,
    endnote: Option<String>,
    comments: Vec<NewComment>,
    hyperlinks: Vec<NewHyperlink>,
    bookmarks: Vec<NewBookmark>,
    fields: Vec<NewField>,
    maths: Vec<NewMath>,
    content_controls: Vec<NewSdt>,
    shapes: Vec<ShapeSpec>,
    connectors: Vec<ShapeSpec>,
    rubies: Vec<RubySpec>,
    citations: Vec<CitationSpec>,
    index_entries: Vec<IndexEntrySpec>,
    cross_refs: Vec<CrossRefSpec>,
    edit_range: Option<EditRangeSpec>,
    num: Option<u32>,
    ilvl: u32,
    section_type: Option<SectionBreakType>,
    remain: ParaRemain,
    runs: Option<Vec<RunSpec>>,
}

impl NewPara {
    fn empty() -> Self {
        Self {
            text: String::new(),
            style: None,
            boxes: Vec::new(),
            pictures: Vec::new(),
            smart_arts: Vec::new(),
            charts: Vec::new(),
            ole_objects: Vec::new(),
            ink_canvases: Vec::new(),
            model3ds: Vec::new(),
            footnote: None,
            endnote: None,
            comments: Vec::new(),
            hyperlinks: Vec::new(),
            bookmarks: Vec::new(),
            fields: Vec::new(),
            maths: Vec::new(),
            content_controls: Vec::new(),
            shapes: Vec::new(),
            connectors: Vec::new(),
            rubies: Vec::new(),
            citations: Vec::new(),
            index_entries: Vec::new(),
            cross_refs: Vec::new(),
            edit_range: None,
            num: None,
            ilvl: 0,
            section_type: None,
            remain: ParaRemain::empty(),
            runs: None,
        }
    }

    fn has_pictures(&self) -> bool {
        !self.pictures.is_empty()
    }

    fn has_smartarts(&self) -> bool {
        !self.smart_arts.is_empty()
    }

    fn has_follow_ups(&self) -> bool {
        !self.charts.is_empty()
            || !self.ole_objects.is_empty()
            || !self.ink_canvases.is_empty()
            || !self.model3ds.is_empty()
            || self.footnote.is_some()
            || self.endnote.is_some()
            || !self.comments.is_empty()
            || !self.hyperlinks.is_empty()
            || !self.bookmarks.is_empty()
            || !self.fields.is_empty()
            || !self.maths.is_empty()
            || !self.content_controls.is_empty()
            || !self.shapes.is_empty()
            || !self.connectors.is_empty()
            || !self.rubies.is_empty()
            || !self.citations.is_empty()
            || !self.index_entries.is_empty()
            || !self.cross_refs.is_empty()
            || self.edit_range.is_some()
            || self.num.is_some()
            || self.remain.has_follow_ups()
            || self
                .runs
                .as_ref()
                .is_some_and(|rs| rs.iter().any(|r| r.format.is_some()))
            || self.boxes.iter().any(|b| b.has_follow_ups())
            || self.pictures.iter().any(|p| p.hyperlink.is_some())
            || self.smart_arts.iter().any(|s| s.hyperlink.is_some())
            || self.charts.iter().any(|c| c.hyperlink.is_some())
            || self.ink_canvases.iter().any(|c| c.hyperlink.is_some())
            || self.model3ds.iter().any(|m| m.hyperlink.is_some())
            || self.shapes.iter().any(|s| s.hyperlink.is_some())
            || self.connectors.iter().any(|s| s.hyperlink.is_some())
    }
}

enum ParaKind {
    Body,
    HeaderFooter,
}

struct HfCreateItem {
    hf_type: HfType,
    paras: Vec<NewPara>,
    tables: Vec<NewTable>,
    groups: Vec<GroupSpec>,
}

struct NewTable {
    after: u32,
    cells: Vec<Vec<String>>,
    extra: TableExtra,
}

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
        ) && !create_round4::ROOT_KEYS.contains(&k.as_str())
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
        paras.push(parse_para(
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
    let extra = create_round4::parse(obj, paras.len())?;
    let tables = parse_tables(obj.get("tables"), "document.tables", paras.len())?;
    let groups = create_round5::parse_groups(obj.get("groups"), "document.groups")?;
    let numberings = parse_numberings(obj.get("numberings"), "document.numberings")?;
    let tocs = parse_tocs(obj.get("tocs"), "document.tocs", paras.len())?;

    let mut header_items = parse_hf_side(obj.get("headers"), "headers")?;
    let mut footer_items = parse_hf_side(obj.get("footers"), "footers")?;
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
        document_xml(&paras, &tables, &hf_refs, need_first).into_bytes(),
    )?;
    pkg.set(
        "/word/_rels/document.xml.rels",
        doc_rels.to_xml().into_bytes(),
    )?;
    for (i, item) in header_items.iter().enumerate() {
        let part = format!("/word/header{}.xml", i + 1);
        pkg.set(&part, hf_xml("hdr", &item.paras, &item.tables).into_bytes())?;
    }
    for (i, item) in footer_items.iter().enumerate() {
        let part = format!("/word/footer{}.xml", i + 1);
        pkg.set(&part, hf_xml("ftr", &item.paras, &item.tables).into_bytes())?;
    }
    pkg.set("/word/styles.xml", styles_xml().into_bytes())?;
    pkg.set("/word/settings.xml", settings_xml(need_even).into_bytes())?;
    pkg.set("/docProps/core.xml", core_xml().into_bytes())?;
    pkg.set("/docProps/app.xml", app_xml().into_bytes())?;
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

fn parse_hf_side(v: Option<&Value>, side: &str) -> Result<Vec<HfCreateItem>, Error> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    let path = format!("document.{side}");
    let arr = v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: side.into(),
                expected: TypeName::Array,
            },
            path.clone(),
        )
    })?;
    if arr.is_empty() {
        return Ok(Vec::new());
    }
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for (i, item) in arr.iter().enumerate() {
        let item_path = format!("{path}[{i}]");
        let obj = item.as_object().ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: format!("{side}[{i}]"),
                    expected: TypeName::Object,
                },
                item_path.clone(),
            )
        })?;
        for k in obj.keys() {
            if !matches!(k.as_str(), "type" | "paragraphs" | "tables" | "groups") {
                return Err(Error::with_path(
                    ErrorKind::UnknownField { field: k.clone() },
                    format!("{item_path}.{k}"),
                ));
            }
        }
        let hf_type = match obj.get("type") {
            None => HfType::Default,
            Some(Value::String(s)) => HfType::parse(s).ok_or_else(|| {
                Error::with_path(ErrorKind::InvalidHfType, format!("{item_path}.type"))
            })?,
            Some(_) => {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: "type".into(),
                        expected: TypeName::String,
                    },
                    format!("{item_path}.type"),
                ));
            }
        };
        if !seen.insert(hf_type) {
            return Err(Error::with_path(ErrorKind::CreateHfDuplicate, path));
        }
        let paras = parse_hf_paras(obj.get("paragraphs"), &format!("{item_path}.paragraphs"))?;
        let tables = parse_tables(
            obj.get("tables"),
            &format!("{item_path}.tables"),
            paras.len(),
        )?;
        let groups =
            create_round5::parse_groups(obj.get("groups"), &format!("{item_path}.groups"))?;
        out.push(HfCreateItem {
            hf_type,
            paras,
            tables,
            groups,
        });
    }
    Ok(out)
}

fn parse_hf_paras(v: Option<&Value>, path: &str) -> Result<Vec<NewPara>, Error> {
    let Some(v) = v else {
        return Ok(vec![NewPara::empty()]);
    };
    let arr = v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "paragraphs".into(),
                expected: TypeName::Array,
            },
            path,
        )
    })?;
    if arr.is_empty() {
        return Ok(vec![NewPara::empty()]);
    }
    let mut out = Vec::new();
    for (i, p) in arr.iter().enumerate() {
        out.push(parse_para(
            p,
            &format!("{path}[{i}]"),
            ParaKind::HeaderFooter,
            i,
        )?);
    }
    Ok(out)
}

fn parse_para(v: &Value, path: &str, kind: ParaKind, i: usize) -> Result<NewPara, Error> {
    let obj = v.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: path.into(),
                expected: TypeName::Object,
            },
            path,
        )
    })?;
    let allowed = match kind {
        ParaKind::Body => [
            "text",
            "style",
            "textBoxes",
            "pictures",
            "smartArts",
            "charts",
            "oleObjects",
            "inkCanvases",
            "model3ds",
            "footnote",
            "endnote",
            "comments",
            "hyperlinks",
            "bookmarks",
            "fields",
            "maths",
            "contentControls",
            "shapes",
            "connectors",
            "rubies",
            "citations",
            "indexEntries",
            "crossRefs",
            "editRange",
            "num",
            "ilvl",
            "sectionType",
            "borders",
            "shd",
            "tabs",
            "runs",
        ]
        .as_slice(),
        ParaKind::HeaderFooter => [
            "text",
            "pictures",
            "smartArts",
            "charts",
            "oleObjects",
            "inkCanvases",
            "model3ds",
            "footnote",
            "endnote",
            "comments",
            "hyperlinks",
            "bookmarks",
            "fields",
            "maths",
            "contentControls",
            "shapes",
            "connectors",
            "rubies",
            "citations",
            "indexEntries",
            "crossRefs",
            "editRange",
            "num",
            "ilvl",
            "borders",
            "shd",
            "tabs",
            "runs",
        ]
        .as_slice(),
    };
    for k in obj.keys() {
        if !allowed.contains(&k.as_str()) {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("{path}.{k}"),
            ));
        }
    }
    let text = match obj.get("text") {
        None => String::new(),
        Some(Value::String(s)) => s.clone(),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "text".into(),
                    expected: TypeName::String,
                },
                format!("{path}.text"),
            ));
        }
    };
    let style = match kind {
        ParaKind::Body => match obj.get("style") {
            None => None,
            Some(Value::String(s)) => canonicalize_style(s, i)?,
            Some(_) => {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: "style".into(),
                        expected: TypeName::String,
                    },
                    format!("{path}.style"),
                ));
            }
        },
        ParaKind::HeaderFooter => None,
    };
    let boxes = match kind {
        ParaKind::Body => parse_boxes(obj.get("textBoxes"), path)?,
        ParaKind::HeaderFooter => Vec::new(),
    };
    let pictures = parse_pictures(obj.get("pictures"), path)?;
    let smart_arts = parse_smartarts(obj.get("smartArts"), path)?;
    let charts = parse_charts(obj.get("charts"), path)?;
    let ole_objects = parse_ole_objects(obj.get("oleObjects"), path)?;
    let ink_canvases = parse_ink_canvases(obj.get("inkCanvases"), path)?;
    let model3ds = parse_model3ds(obj.get("model3ds"), path)?;
    let footnote = parse_optional_str(obj.get("footnote"), path, "footnote")?;
    let endnote = parse_optional_str(obj.get("endnote"), path, "endnote")?;
    let comments = parse_comments(obj.get("comments"), &format!("{path}.comments"), false)?;
    let hyperlinks = parse_hyperlinks(obj.get("hyperlinks"), path)?;
    let bookmarks = parse_bookmarks(obj.get("bookmarks"), path)?;
    let fields = parse_fields(obj.get("fields"), path)?;
    let maths = parse_maths(obj.get("maths"), path)?;
    let content_controls = parse_sdts(obj.get("contentControls"), path)?;
    let shapes = create_round4::parse_shapes(obj.get("shapes"), path)?;
    let connectors = create_round4::parse_connectors(obj.get("connectors"), path)?;
    let rubies = create_round4::parse_rubies(obj.get("rubies"), path)?;
    let citations = create_round4::parse_citations(obj.get("citations"), path)?;
    let index_entries = create_round4::parse_index_entries(obj.get("indexEntries"), path)?;
    let cross_refs = create_round4::parse_cross_refs(obj.get("crossRefs"), path)?;
    let edit_range = create_round4::parse_edit_range(obj.get("editRange"), path)?;
    let (num, ilvl) = parse_para_num(obj, path)?;
    let section_type = match kind {
        ParaKind::Body => parse_create_section_type(obj.get("sectionType"), path)?,
        ParaKind::HeaderFooter => None,
    };
    let remain = create_round5::parse_para_remain(obj, path)?;
    let runs = create_round5::parse_runs(obj.get("runs"), path)?;
    Ok(NewPara {
        text,
        style,
        boxes,
        pictures,
        smart_arts,
        charts,
        ole_objects,
        ink_canvases,
        model3ds,
        footnote,
        endnote,
        comments,
        hyperlinks,
        bookmarks,
        fields,
        maths,
        content_controls,
        shapes,
        connectors,
        rubies,
        citations,
        index_entries,
        cross_refs,
        edit_range,
        num,
        ilvl,
        section_type,
        remain,
        runs,
    })
}

fn parse_create_section_type(
    v: Option<&Value>,
    path: &str,
) -> Result<Option<SectionBreakType>, Error> {
    match v {
        None => Ok(None),
        Some(Value::String(s)) => Ok(Some(SectionBreakType::parse(s).ok_or_else(|| {
            Error::with_path(ErrorKind::InvalidSectionType, format!("{path}.sectionType"))
        })?)),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "sectionType".into(),
                expected: TypeName::String,
            },
            format!("{path}.sectionType"),
        )),
    }
}

fn parse_boxes(v: Option<&Value>, prefix: &str) -> Result<Vec<NewBox>, Error> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    let path = format!("{prefix}.textBoxes");
    let arr = v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "textBoxes".into(),
                expected: TypeName::Array,
            },
            path.clone(),
        )
    })?;
    let mut out = Vec::new();
    for (j, b) in arr.iter().enumerate() {
        let item_path = format!("{path}[{j}]");
        let obj = b.as_object().ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: item_path.clone(),
                    expected: TypeName::Object,
                },
                item_path.clone(),
            )
        })?;
        for k in obj.keys() {
            if !matches!(
                k.as_str(),
                "text"
                    | "cx"
                    | "cy"
                    | "hyperlink"
                    | "tables"
                    | "hyperlinks"
                    | "bookmarks"
                    | "fields"
                    | "maths"
                    | "contentControls"
                    | "rubies"
            ) {
                return Err(Error::with_path(
                    ErrorKind::UnknownField { field: k.clone() },
                    format!("{item_path}.{k}"),
                ));
            }
        }
        let text = match obj.get("text") {
            None => String::new(),
            Some(Value::String(s)) => s.clone(),
            Some(_) => {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: "text".into(),
                        expected: TypeName::String,
                    },
                    format!("{item_path}.text"),
                ));
            }
        };
        let cx = parse_emu(obj.get("cx"), &item_path, "cx")?.unwrap_or(DEFAULT_CX);
        let cy = parse_emu(obj.get("cy"), &item_path, "cy")?.unwrap_or(DEFAULT_CY);
        let hyperlink = create_round4::parse_drawing_hlink(
            obj.get("hyperlink"),
            &format!("{item_path}.hyperlink"),
        )?;
        let tables = parse_tables(obj.get("tables"), &format!("{item_path}.tables"), 1)?;
        for (t_i, t) in tables.iter().enumerate() {
            if t.after == 0 {
                return Err(Error::with_path(
                    ErrorKind::MinValue {
                        field: "after".into(),
                        min: 1,
                    },
                    format!("{item_path}.tables[{t_i}].after"),
                ));
            }
        }
        let hyperlinks = parse_hyperlinks(obj.get("hyperlinks"), &item_path)?;
        let bookmarks = parse_bookmarks(obj.get("bookmarks"), &item_path)?;
        let fields = parse_fields(obj.get("fields"), &item_path)?;
        let maths = parse_maths(obj.get("maths"), &item_path)?;
        let content_controls = parse_sdts(obj.get("contentControls"), &item_path)?;
        let rubies = create_round4::parse_rubies(obj.get("rubies"), &item_path)?;
        out.push(NewBox {
            text,
            cx,
            cy,
            doc_pr: 0,
            inner_para: 0,
            hyperlink,
            tables,
            hyperlinks,
            bookmarks,
            fields,
            maths,
            content_controls,
            rubies,
        });
    }
    Ok(out)
}

fn parse_pictures(v: Option<&Value>, prefix: &str) -> Result<Vec<NewPic>, Error> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    let path = format!("{prefix}.pictures");
    let arr = v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "pictures".into(),
                expected: TypeName::Array,
            },
            path.clone(),
        )
    })?;
    let mut out = Vec::new();
    for (j, p) in arr.iter().enumerate() {
        let item_path = format!("{path}[{j}]");
        let obj = p.as_object().ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: item_path.clone(),
                    expected: TypeName::Object,
                },
                item_path.clone(),
            )
        })?;
        for k in obj.keys() {
            if !matches!(
                k.as_str(),
                "file"
                    | "cx"
                    | "cy"
                    | "kind"
                    | "wrap"
                    | "alignH"
                    | "alignV"
                    | "offsetX"
                    | "offsetY"
                    | "hyperlink"
            ) {
                return Err(Error::with_path(
                    ErrorKind::UnknownField { field: k.clone() },
                    format!("{item_path}.{k}"),
                ));
            }
        }
        let file = match obj.get("file") {
            Some(Value::String(s)) if !s.is_empty() => s.clone(),
            Some(Value::String(_)) | None => {
                return Err(Error::with_path(
                    ErrorKind::MissingField {
                        field: "file".into(),
                    },
                    format!("{item_path}.file"),
                ));
            }
            Some(_) => {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: "file".into(),
                        expected: TypeName::NonEmptyString,
                    },
                    format!("{item_path}.file"),
                ));
            }
        };
        let cx = parse_emu(obj.get("cx"), &item_path, "cx")?;
        let cy = parse_emu(obj.get("cy"), &item_path, "cy")?;
        if cx.is_some() != cy.is_some() {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "cx and cy".into(),
                },
                item_path.clone(),
            ));
        }
        let kind = match obj.get("kind") {
            None => PicKind::Inline,
            Some(Value::String(s)) => picture::parse_pic_kind(s).ok_or_else(|| {
                Error::with_path(ErrorKind::InvalidPicKind, format!("{item_path}.kind"))
            })?,
            Some(_) => {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: "kind".into(),
                        expected: TypeName::String,
                    },
                    format!("{item_path}.kind"),
                ));
            }
        };
        let wrap = match obj.get("wrap") {
            None | Some(Value::Null) => None,
            Some(Value::String(s)) => Some(picture::parse_wrap(s).ok_or_else(|| {
                Error::with_path(ErrorKind::InvalidPicWrap, format!("{item_path}.wrap"))
            })?),
            Some(_) => {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: "wrap".into(),
                        expected: TypeName::String,
                    },
                    format!("{item_path}.wrap"),
                ));
            }
        };
        if kind == PicKind::Inline && wrap.is_some() {
            return Err(Error::with_path(
                ErrorKind::InvalidPicWrap,
                format!("{item_path}.wrap"),
            ));
        }
        if kind == PicKind::Anchor && wrap.is_none() {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "wrap".into(),
                },
                format!("{item_path}.wrap"),
            ));
        }
        let align_h = parse_align_h(obj.get("alignH"), &item_path)?;
        let align_v = parse_align_v(obj.get("alignV"), &item_path)?;
        let offset_x = parse_i64(obj.get("offsetX"), &item_path, "offsetX")?;
        let offset_y = parse_i64(obj.get("offsetY"), &item_path, "offsetY")?;
        if align_h.is_some() && offset_x.is_some() {
            return Err(Error::with_path(
                ErrorKind::MutuallyExclusive {
                    a: "alignH".into(),
                    b: "offsetX".into(),
                },
                item_path.clone(),
            ));
        }
        if align_v.is_some() && offset_y.is_some() {
            return Err(Error::with_path(
                ErrorKind::MutuallyExclusive {
                    a: "alignV".into(),
                    b: "offsetY".into(),
                },
                item_path.clone(),
            ));
        }
        if kind == PicKind::Inline
            && (align_h.is_some() || align_v.is_some() || offset_x.is_some() || offset_y.is_some())
        {
            return Err(Error::with_path(
                ErrorKind::UnknownField {
                    field: "alignH".into(),
                },
                item_path,
            ));
        }
        let hyperlink = create_round4::parse_drawing_hlink(
            obj.get("hyperlink"),
            &format!("{item_path}.hyperlink"),
        )?;
        out.push(NewPic {
            file,
            cx,
            cy,
            kind,
            wrap,
            pos: WrapPos {
                align_h,
                align_v,
                offset_x,
                offset_y,
            },
            doc_pr: 0,
            hyperlink,
        });
    }
    Ok(out)
}

fn parse_align_h(v: Option<&Value>, item_path: &str) -> Result<Option<picture::AlignH>, Error> {
    match v {
        None => Ok(None),
        Some(Value::String(s)) => picture::parse_align_h(s).map(Some).ok_or_else(|| {
            Error::with_path(ErrorKind::InvalidPicAlign, format!("{item_path}.alignH"))
        }),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "alignH".into(),
                expected: TypeName::String,
            },
            format!("{item_path}.alignH"),
        )),
    }
}

fn parse_align_v(v: Option<&Value>, item_path: &str) -> Result<Option<picture::AlignV>, Error> {
    match v {
        None => Ok(None),
        Some(Value::String(s)) => picture::parse_align_v(s).map(Some).ok_or_else(|| {
            Error::with_path(ErrorKind::InvalidPicAlign, format!("{item_path}.alignV"))
        }),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "alignV".into(),
                expected: TypeName::String,
            },
            format!("{item_path}.alignV"),
        )),
    }
}

fn parse_emu(v: Option<&Value>, prefix: &str, field: &str) -> Result<Option<i64>, Error> {
    let Some(v) = v else {
        return Ok(None);
    };
    let path = format!("{prefix}.{field}");
    match v {
        Value::Number(n) => {
            if let Some(u) = n.as_u64() {
                if u == 0 || u > i64::MAX as u64 {
                    return Err(Error::with_path(
                        ErrorKind::MinValue {
                            field: field.into(),
                            min: 1,
                        },
                        path,
                    ));
                }
                return Ok(Some(u as i64));
            }
            Err(Error::with_path(
                ErrorKind::WrongType {
                    field: field.into(),
                    expected: TypeName::Integer,
                },
                path,
            ))
        }
        _ => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Integer,
            },
            path,
        )),
    }
}

fn parse_i64(v: Option<&Value>, prefix: &str, field: &str) -> Result<Option<i64>, Error> {
    let Some(v) = v else {
        return Ok(None);
    };
    let path = format!("{prefix}.{field}");
    match v {
        Value::Number(n) => n.as_i64().map(Some).ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: field.into(),
                    expected: TypeName::Integer,
                },
                path,
            )
        }),
        _ => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Integer,
            },
            path,
        )),
    }
}

fn canonicalize_style(s: &str, i: usize) -> Result<Option<&'static str>, Error> {
    if s.eq_ignore_ascii_case("Normal") {
        return Ok(None);
    }
    for c in ["Heading1", "Heading2", "Heading3"] {
        if s.eq_ignore_ascii_case(c) {
            return Ok(Some(c));
        }
    }
    Err(Error::with_path(
        ErrorKind::InvalidParaStyle,
        format!("document.paragraphs[{i}].style"),
    ))
}

fn para_xml(p: &NewPara, id: &str, first_break: bool, refs_xml: &str, title_pg: bool) -> String {
    let mut ppr_inner = String::new();
    if let Some(s) = p.style {
        ppr_inner.push_str(&format!(r#"<w:pStyle w:val="{s}"/>"#));
    }
    if let Some(ty) = p.section_type {
        let refs = if first_break { refs_xml } else { "" };
        let title = first_break && title_pg;
        ppr_inner.push_str(&sect_pr_inner_xml(Some(ty.as_str()), refs, title));
    }
    let ppr = if ppr_inner.is_empty() {
        String::new()
    } else {
        format!("<w:pPr>{ppr_inner}</w:pPr>")
    };
    let t = if let Some(runs) = &p.runs {
        let mut s = String::new();
        for r in runs {
            s.push_str(&run_t_xml(&r.text));
        }
        s
    } else if p.text.is_empty() {
        String::new()
    } else {
        let space = if needs_xml_space(&p.text) {
            r#" xml:space="preserve""#
        } else {
            ""
        };
        format!("<w:r><w:t{space}>{}</w:t></w:r>", xml::escape_text(&p.text))
    };
    let mut boxes = String::new();
    for b in &p.boxes {
        let inner_id = format!("{:08X}", b.inner_para);
        boxes.push_str(&format!(
            "<w:r>{}</w:r>",
            textbox::textbox_ac_xml(b.doc_pr, b.cx, b.cy, &b.text, &inner_id)
        ));
    }
    format!(r#"<w:p w14:paraId="{id}" w14:textId="{id}">{ppr}{t}{boxes}</w:p>"#)
}

fn run_t_xml(text: &str) -> String {
    if text.is_empty() {
        return "<w:r><w:t/></w:r>".into();
    }
    let space = if needs_xml_space(text) {
        r#" xml:space="preserve""#
    } else {
        ""
    };
    format!("<w:r><w:t{space}>{}</w:t></w:r>", xml::escape_text(text))
}

fn sect_pr_inner_xml(ty: Option<&str>, refs_xml: &str, title_pg: bool) -> String {
    let type_xml = ty
        .map(|t| format!(r#"<w:type w:val="{t}"/>"#))
        .unwrap_or_default();
    let title = if title_pg { "<w:titlePg/>" } else { "" };
    format!(
        r#"<w:sectPr>{refs_xml}{type_xml}<w:pgSz w:w="11906" w:h="16838"/><w:pgMar w:top="1440" w:right="1440" w:bottom="1440" w:left="1440" w:header="708" w:footer="708" w:gutter="0"/>{title}</w:sectPr>"#
    )
}

fn story_xmlns() -> String {
    format!(
        r#"xmlns:w="{WML_NS}" xmlns:r="{REL_NS}" xmlns:w14="{W14_NS}" xmlns:mc="{MC_NS}" xmlns:wp="{WP_NS}" xmlns:a="{A_NS}" xmlns:pic="{PIC_NS}" xmlns:wps="{WPS_NS}" mc:Ignorable="w14""#
    )
}

fn document_xml(
    paras: &[NewPara],
    tables: &[NewTable],
    refs: &[(HfKind, HfType, String)],
    title_pg: bool,
) -> String {
    let mut refs_xml = String::new();
    for (kind, hf_type, rid) in refs {
        let tag = match kind {
            HfKind::Header => "headerReference",
            HfKind::Footer => "footerReference",
        };
        refs_xml.push_str(&format!(
            r#"<w:{tag} w:type="{}" r:id="{rid}"/>"#,
            hf_type.as_str()
        ));
    }
    let first_break = paras.iter().position(|p| p.section_type.is_some());
    let body = emit_story_blocks(paras, tables, first_break, &refs_xml, title_pg);
    let last_refs = if first_break.is_some() { "" } else { &refs_xml };
    let last_title = first_break.is_none() && title_pg;
    let last_sect = sect_pr_inner_xml(None, last_refs, last_title);
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document {}><w:body>{body}{last_sect}</w:body></w:document>"#,
        story_xmlns()
    )
}

fn hf_xml(tag: &str, paras: &[NewPara], tables: &[NewTable]) -> String {
    let body = emit_story_blocks(paras, tables, None, "", false);
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:{tag} {}>{body}</w:{tag}>"#,
        story_xmlns()
    )
}

fn emit_story_blocks(
    paras: &[NewPara],
    tables: &[NewTable],
    first_break: Option<usize>,
    refs_xml: &str,
    title_pg: bool,
) -> String {
    let mut para_xmls = Vec::new();
    let mut next_para = paras.len() as u32 + 1;
    for p in paras {
        next_para = next_para.saturating_add(p.boxes.len() as u32);
    }
    for (i, p) in paras.iter().enumerate() {
        let id = format!("{:08X}", i as u32 + 1);
        para_xmls.push(para_xml(p, &id, first_break == Some(i), refs_xml, title_pg));
    }
    let mut table_xmls = Vec::new();
    for t in tables {
        let n = t.cells.iter().map(|r| r.len()).sum::<usize>();
        let mut ids = Vec::with_capacity(n);
        for _ in 0..n {
            ids.push(format!("{:08X}", next_para));
            next_para = next_para.saturating_add(1);
        }
        table_xmls.push(tbl::table_xml(&t.cells, &ids));
    }
    let mut body = String::new();
    append_tables_after(&mut body, tables, &table_xmls, 0);
    for (i, xml) in para_xmls.iter().enumerate() {
        body.push_str(xml);
        append_tables_after(&mut body, tables, &table_xmls, i as u32 + 1);
    }
    body
}

fn append_tables_after(body: &mut String, tables: &[NewTable], xmls: &[String], after: u32) {
    for (i, t) in tables.iter().enumerate() {
        if t.after == after {
            body.push_str(&xmls[i]);
        }
    }
}

fn parse_tables(v: Option<&Value>, path: &str, para_n: usize) -> Result<Vec<NewTable>, Error> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    let arr = v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "tables".into(),
                expected: TypeName::Array,
            },
            path,
        )
    })?;
    let mut out = Vec::new();
    for (i, item) in arr.iter().enumerate() {
        let item_path = format!("{path}[{i}]");
        let obj = item.as_object().ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: item_path.clone(),
                    expected: TypeName::Object,
                },
                item_path.clone(),
            )
        })?;
        for k in obj.keys() {
            if !matches!(
                k.as_str(),
                "cells"
                    | "after"
                    | "style"
                    | "look"
                    | "pos"
                    | "align"
                    | "indent"
                    | "width"
                    | "cols"
                    | "rowHeights"
                    | "cellMar"
                    | "borders"
                    | "cellFormats"
            ) {
                return Err(Error::with_path(
                    ErrorKind::UnknownField { field: k.clone() },
                    format!("{item_path}.{k}"),
                ));
            }
        }
        let cells_v = obj.get("cells").ok_or_else(|| {
            Error::with_path(
                ErrorKind::MissingField {
                    field: "cells".into(),
                },
                format!("{item_path}.cells"),
            )
        })?;
        let cells = tbl::parse_create_cells(cells_v, &format!("{item_path}.cells"))?;
        let after = parse_table_after(obj.get("after"), &format!("{item_path}.after"), para_n)?;
        let rows = cells.len();
        let cols = cells.first().map(|r| r.len()).unwrap_or(0);
        let extra = create_round4::parse_table_extra(obj, &item_path, rows, cols)?;
        out.push(NewTable {
            after,
            cells,
            extra,
        });
    }
    Ok(out)
}

fn parse_table_after(v: Option<&Value>, path: &str, para_n: usize) -> Result<u32, Error> {
    match v {
        None => Ok(para_n as u32),
        Some(Value::Number(n)) => {
            let Some(u) = n.as_u64() else {
                if n.as_i64().is_some_and(|x| x < 0) {
                    return Err(Error::with_path(
                        ErrorKind::MinValue {
                            field: "after".into(),
                            min: 0,
                        },
                        path,
                    ));
                }
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: "after".into(),
                        expected: TypeName::Integer,
                    },
                    path,
                ));
            };
            if u > para_n as u64 {
                return Err(Error::with_path(
                    ErrorKind::OutOfRange {
                        field: "after".into(),
                        min: 0,
                        max: para_n as i64,
                    },
                    path,
                ));
            }
            Ok(u as u32)
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "after".into(),
                expected: TypeName::Integer,
            },
            path,
        )),
    }
}

fn parse_smartarts(v: Option<&Value>, prefix: &str) -> Result<Vec<CreateSmartArt>, Error> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    let path = format!("{prefix}.smartArts");
    let arr = v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "smartArts".into(),
                expected: TypeName::Array,
            },
            path.clone(),
        )
    })?;
    let mut out = Vec::new();
    for (j, item) in arr.iter().enumerate() {
        let item_path = format!("{path}[{j}]");
        let obj = item.as_object().ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: item_path.clone(),
                    expected: TypeName::Object,
                },
                item_path.clone(),
            )
        })?;
        for k in obj.keys() {
            if !matches!(
                k.as_str(),
                "layout" | "points" | "kind" | "wrap" | "cx" | "cy" | "hyperlink"
            ) {
                return Err(Error::with_path(
                    ErrorKind::UnknownField { field: k.clone() },
                    format!("{item_path}.{k}"),
                ));
            }
        }
        out.push(CreateSmartArt {
            spec: smartart::parse_spec(obj, &item_path)?,
            hyperlink: create_round4::parse_drawing_hlink(
                obj.get("hyperlink"),
                &format!("{item_path}.hyperlink"),
            )?,
        });
    }
    Ok(out)
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

fn parse_optional_str(v: Option<&Value>, path: &str, field: &str) -> Result<Option<String>, Error> {
    match v {
        None => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::String,
            },
            format!("{path}.{field}"),
        )),
    }
}

fn parse_optional_bool(v: Option<&Value>, path: &str, field: &str) -> Result<Option<bool>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Bool(b)) => Ok(Some(*b)),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Boolean,
            },
            format!("{path}.{field}"),
        )),
    }
}

fn parse_comments(v: Option<&Value>, path: &str, reply: bool) -> Result<Vec<NewComment>, Error> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    let field = if reply { "replies" } else { "comments" };
    let arr = v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Array,
            },
            path,
        )
    })?;
    let mut out = Vec::new();
    for (j, item) in arr.iter().enumerate() {
        let item_path = format!("{path}[{j}]");
        let obj = item.as_object().ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: item_path.clone(),
                    expected: TypeName::Object,
                },
                item_path.clone(),
            )
        })?;
        for k in obj.keys() {
            if reply {
                if !matches!(k.as_str(), "text" | "author" | "initials" | "date") {
                    return Err(Error::with_path(
                        ErrorKind::UnknownField { field: k.clone() },
                        format!("{item_path}.{k}"),
                    ));
                }
            } else if !matches!(
                k.as_str(),
                "text" | "author" | "initials" | "date" | "replies"
            ) {
                return Err(Error::with_path(
                    ErrorKind::UnknownField { field: k.clone() },
                    format!("{item_path}.{k}"),
                ));
            }
        }
        let text = match obj.get("text") {
            None => String::new(),
            Some(Value::String(s)) => s.clone(),
            Some(_) => {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: "text".into(),
                        expected: TypeName::String,
                    },
                    format!("{item_path}.text"),
                ));
            }
        };
        let author = parse_comment_attr(obj.get("author"), &item_path, "author")?
            .unwrap_or_else(|| "ooxp".into());
        let initials = parse_comment_attr(obj.get("initials"), &item_path, "initials")?
            .unwrap_or_else(|| "oo".into());
        let date = parse_comment_attr(obj.get("date"), &item_path, "date")?;
        let replies = if reply {
            Vec::new()
        } else {
            parse_comments(obj.get("replies"), &format!("{item_path}.replies"), true)?
        };
        out.push(NewComment {
            text,
            author,
            initials,
            date,
            replies,
        });
    }
    Ok(out)
}

fn parse_comment_attr(v: Option<&Value>, path: &str, field: &str) -> Result<Option<String>, Error> {
    match v {
        None => Ok(None),
        Some(Value::String(s)) if !s.is_empty() => Ok(Some(s.clone())),
        Some(Value::String(_)) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::NonEmptyString,
            },
            format!("{path}.{field}"),
        )),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::String,
            },
            format!("{path}.{field}"),
        )),
    }
}

fn parse_ole_objects(v: Option<&Value>, prefix: &str) -> Result<Vec<String>, Error> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    let path = format!("{prefix}.oleObjects");
    let arr = v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "oleObjects".into(),
                expected: TypeName::Array,
            },
            path.clone(),
        )
    })?;
    let mut out = Vec::new();
    for (j, item) in arr.iter().enumerate() {
        let item_path = format!("{path}[{j}]");
        let obj = item.as_object().ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: item_path.clone(),
                    expected: TypeName::Object,
                },
                item_path.clone(),
            )
        })?;
        for k in obj.keys() {
            if k.as_str() != "file" {
                return Err(Error::with_path(
                    ErrorKind::UnknownField { field: k.clone() },
                    format!("{item_path}.{k}"),
                ));
            }
        }
        let file = match obj.get("file") {
            Some(Value::String(s)) if !s.is_empty() => s.clone(),
            Some(Value::String(_)) | None => {
                return Err(Error::with_path(
                    ErrorKind::MissingField {
                        field: "file".into(),
                    },
                    format!("{item_path}.file"),
                ));
            }
            Some(_) => {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: "file".into(),
                        expected: TypeName::NonEmptyString,
                    },
                    format!("{item_path}.file"),
                ));
            }
        };
        out.push(file);
    }
    Ok(out)
}

fn parse_ink_canvases(v: Option<&Value>, prefix: &str) -> Result<Vec<NewInkCanvas>, Error> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    let path = format!("{prefix}.inkCanvases");
    let arr = v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "inkCanvases".into(),
                expected: TypeName::Array,
            },
            path.clone(),
        )
    })?;
    let mut out = Vec::new();
    for (j, item) in arr.iter().enumerate() {
        let item_path = format!("{path}[{j}]");
        let obj = item.as_object().ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: item_path.clone(),
                    expected: TypeName::Object,
                },
                item_path.clone(),
            )
        })?;
        for k in obj.keys() {
            if !matches!(k.as_str(), "file" | "cx" | "cy" | "hyperlink") {
                return Err(Error::with_path(
                    ErrorKind::UnknownField { field: k.clone() },
                    format!("{item_path}.{k}"),
                ));
            }
        }
        let file = match obj.get("file") {
            Some(Value::String(s)) if !s.is_empty() => s.clone(),
            Some(Value::String(_)) | None => {
                return Err(Error::with_path(
                    ErrorKind::MissingField {
                        field: "file".into(),
                    },
                    format!("{item_path}.file"),
                ));
            }
            Some(_) => {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: "file".into(),
                        expected: TypeName::NonEmptyString,
                    },
                    format!("{item_path}.file"),
                ));
            }
        };
        let cx = parse_emu(obj.get("cx"), &item_path, "cx")?;
        let cy = parse_emu(obj.get("cy"), &item_path, "cy")?;
        if cx.is_some() != cy.is_some() {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "cx and cy".into(),
                },
                item_path.clone(),
            ));
        }
        out.push(NewInkCanvas {
            file,
            cx,
            cy,
            hyperlink: create_round4::parse_drawing_hlink(
                obj.get("hyperlink"),
                &format!("{item_path}.hyperlink"),
            )?,
        });
    }
    Ok(out)
}

fn parse_model3ds(v: Option<&Value>, prefix: &str) -> Result<Vec<NewModel3d>, Error> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    let path = format!("{prefix}.model3ds");
    let arr = v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "model3ds".into(),
                expected: TypeName::Array,
            },
            path.clone(),
        )
    })?;
    let mut out = Vec::new();
    for (j, item) in arr.iter().enumerate() {
        let item_path = format!("{path}[{j}]");
        let obj = item.as_object().ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: item_path.clone(),
                    expected: TypeName::Object,
                },
                item_path.clone(),
            )
        })?;
        for k in obj.keys() {
            if !matches!(k.as_str(), "file" | "cx" | "cy" | "hyperlink") {
                return Err(Error::with_path(
                    ErrorKind::UnknownField { field: k.clone() },
                    format!("{item_path}.{k}"),
                ));
            }
        }
        let file = match obj.get("file") {
            Some(Value::String(s)) if !s.is_empty() => s.clone(),
            Some(Value::String(_)) | None => {
                return Err(Error::with_path(
                    ErrorKind::MissingField {
                        field: "file".into(),
                    },
                    format!("{item_path}.file"),
                ));
            }
            Some(_) => {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: "file".into(),
                        expected: TypeName::NonEmptyString,
                    },
                    format!("{item_path}.file"),
                ));
            }
        };
        let cx = parse_emu(obj.get("cx"), &item_path, "cx")?;
        let cy = parse_emu(obj.get("cy"), &item_path, "cy")?;
        if cx.is_some() != cy.is_some() {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "cx and cy".into(),
                },
                item_path.clone(),
            ));
        }
        out.push(NewModel3d {
            file,
            cx,
            cy,
            hyperlink: create_round4::parse_drawing_hlink(
                obj.get("hyperlink"),
                &format!("{item_path}.hyperlink"),
            )?,
        });
    }
    Ok(out)
}

fn parse_charts(v: Option<&Value>, prefix: &str) -> Result<Vec<CreateChart>, Error> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    let path = format!("{prefix}.charts");
    let arr = v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "charts".into(),
                expected: TypeName::Array,
            },
            path.clone(),
        )
    })?;
    let mut out = Vec::new();
    for (j, item) in arr.iter().enumerate() {
        let item_path = format!("{path}[{j}]");
        out.push(parse_one_chart(item, &item_path)?);
    }
    Ok(out)
}

fn parse_one_chart(v: &Value, path: &str) -> Result<CreateChart, Error> {
    let obj = v.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: path.into(),
                expected: TypeName::Object,
            },
            path,
        )
    })?;
    for k in obj.keys() {
        if !matches!(
            k.as_str(),
            "type"
                | "ofPieType"
                | "stockKind"
                | "surfaceKind"
                | "layoutId"
                | "title"
                | "series"
                | "kind"
                | "wrap"
                | "cx"
                | "cy"
                | "hyperlink"
        ) {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("{path}.{k}"),
            ));
        }
    }
    let chart_type = match obj.get("type") {
        Some(Value::String(s)) => ChartType::parse(s)
            .ok_or_else(|| Error::with_path(ErrorKind::InvalidChartType, format!("{path}.type")))?,
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "type".into(),
                    expected: TypeName::String,
                },
                format!("{path}.type"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "type".into(),
                },
                format!("{path}.type"),
            ));
        }
    };
    let title = match obj.get("title") {
        None | Some(Value::Null) => None,
        Some(Value::String(s)) => Some(s.clone()),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "title".into(),
                    expected: TypeName::String,
                },
                format!("{path}.title"),
            ));
        }
    };
    let series = parse_create_series(obj.get("series"), path)?;
    let extra = crate::chart::parse_chart_extra(obj, chart_type, series.len(), path, true)?;
    crate::chart::validate_insert_series_len(
        chart_type,
        series.len(),
        &extra,
        &format!("{path}.series"),
    )?;
    let of_pie_type =
        crate::chart::parse_of_pie_type(obj.get("ofPieType"), &format!("{path}.ofPieType"))?;
    crate::chart::reject_of_pie_on_other(
        chart_type,
        of_pie_type.as_deref(),
        &format!("{path}.ofPieType"),
    )?;
    if chart_type == ChartType::Combo {
        let kinds: HashSet<&str> = series
            .iter()
            .filter_map(|s| s.series_type.map(ChartType::as_str))
            .collect();
        if series.iter().any(|s| s.series_type.is_none()) || kinds.len() < 2 {
            return Err(Error::with_path(
                ErrorKind::InvalidComboSeriesType,
                format!("{path}.series"),
            ));
        }
    } else if series.iter().any(|s| s.series_type.is_some()) {
        return Err(Error::with_path(
            ErrorKind::InvalidComboSeriesType,
            format!("{path}.series"),
        ));
    }
    if chart_type != ChartType::Bubble && series.iter().any(|s| s.sizes.is_some()) {
        return Err(Error::with_path(
            ErrorKind::SizesOnNonBubble,
            format!("{path}.series"),
        ));
    }
    let kind = match obj.get("kind") {
        None => PicKind::Anchor,
        Some(Value::String(s)) => picture::parse_pic_kind(s)
            .ok_or_else(|| Error::with_path(ErrorKind::InvalidPicKind, format!("{path}.kind")))?,
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "kind".into(),
                    expected: TypeName::String,
                },
                format!("{path}.kind"),
            ));
        }
    };
    let wrap =
        match obj.get("wrap") {
            None | Some(Value::Null) => {
                if kind == PicKind::Anchor {
                    Some(WrapKind::Square)
                } else {
                    None
                }
            }
            Some(Value::String(s)) => Some(picture::parse_wrap(s).ok_or_else(|| {
                Error::with_path(ErrorKind::InvalidPicWrap, format!("{path}.wrap"))
            })?),
            Some(_) => {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: "wrap".into(),
                        expected: TypeName::String,
                    },
                    format!("{path}.wrap"),
                ));
            }
        };
    if kind == PicKind::Inline && wrap.is_some() {
        return Err(Error::with_path(
            ErrorKind::InvalidPicWrap,
            format!("{path}.wrap"),
        ));
    }
    let cx = parse_emu(obj.get("cx"), path, "cx")?;
    let cy = parse_emu(obj.get("cy"), path, "cy")?;
    if cx.is_some() != cy.is_some() {
        return Err(Error::with_path(
            ErrorKind::MissingField {
                field: "cx and cy".into(),
            },
            path.to_string(),
        ));
    }
    Ok(CreateChart {
        chart_type,
        of_pie_type,
        extra,
        title,
        series,
        kind,
        wrap,
        cx: cx.unwrap_or(charts::default_cx()),
        cy: cy.unwrap_or(charts::default_cy()),
        hyperlink: create_round4::parse_drawing_hlink(
            obj.get("hyperlink"),
            &format!("{path}.hyperlink"),
        )?,
    })
}

fn parse_create_series(v: Option<&Value>, path: &str) -> Result<Vec<NewSeries>, Error> {
    let arr = match v {
        Some(Value::Array(a)) if !a.is_empty() => a,
        Some(Value::Array(_)) | None => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "series".into(),
                },
                format!("{path}.series"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "series".into(),
                    expected: TypeName::Array,
                },
                format!("{path}.series"),
            ));
        }
    };
    let mut out = Vec::new();
    for (j, item) in arr.iter().enumerate() {
        let item_path = format!("{path}.series[{j}]");
        let obj = item.as_object().ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: format!("series[{j}]"),
                    expected: TypeName::Object,
                },
                item_path.clone(),
            )
        })?;
        for k in obj.keys() {
            if !matches!(
                k.as_str(),
                "name" | "categories" | "values" | "sizes" | "type"
            ) {
                return Err(Error::with_path(
                    ErrorKind::UnknownField { field: k.clone() },
                    format!("{item_path}.{k}"),
                ));
            }
        }
        let name = match obj.get("name") {
            None | Some(Value::Null) => None,
            Some(Value::String(s)) => Some(s.clone()),
            Some(_) => {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: "name".into(),
                        expected: TypeName::String,
                    },
                    format!("{item_path}.name"),
                ));
            }
        };
        let categories =
            parse_datum_list(obj.get("categories"), &format!("{item_path}.categories"))?;
        let values = parse_datum_list(obj.get("values"), &format!("{item_path}.values"))?
            .filter(|v| !v.is_empty())
            .ok_or_else(|| {
                Error::with_path(
                    ErrorKind::MissingField {
                        field: "values".into(),
                    },
                    format!("{item_path}.values"),
                )
            })?;
        let sizes = parse_datum_list(obj.get("sizes"), &format!("{item_path}.sizes"))?;
        let series_type = match obj.get("type") {
            None | Some(Value::Null) => None,
            Some(Value::String(s)) => Some(ChartType::parse_combo_series(s).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::InvalidComboSeriesType,
                    format!("{item_path}.type"),
                )
            })?),
            Some(_) => {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: "type".into(),
                        expected: TypeName::String,
                    },
                    format!("{item_path}.type"),
                ));
            }
        };
        out.push(NewSeries {
            name,
            categories,
            values,
            sizes,
            series_type,
        });
    }
    Ok(out)
}

fn parse_datum_list(v: Option<&Value>, path: &str) -> Result<Option<Vec<Value>>, Error> {
    match v {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Array(a)) => {
            let mut out = Vec::new();
            for (j, item) in a.iter().enumerate() {
                match item {
                    Value::String(_) | Value::Number(_) => out.push(item.clone()),
                    _ => {
                        return Err(Error::with_path(
                            ErrorKind::WrongType {
                                field: format!("[{j}]"),
                                expected: TypeName::String,
                            },
                            format!("{path}[{j}]"),
                        ));
                    }
                }
            }
            Ok(Some(out))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "array".into(),
                expected: TypeName::Array,
            },
            path.to_string(),
        )),
    }
}

fn parse_hyperlinks(v: Option<&Value>, prefix: &str) -> Result<Vec<NewHyperlink>, Error> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    let path = format!("{prefix}.hyperlinks");
    let arr = v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "hyperlinks".into(),
                expected: TypeName::Array,
            },
            path.clone(),
        )
    })?;
    let mut out = Vec::new();
    for (j, item) in arr.iter().enumerate() {
        let item_path = format!("{path}[{j}]");
        let obj = item.as_object().ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: item_path.clone(),
                    expected: TypeName::Object,
                },
                item_path.clone(),
            )
        })?;
        for k in obj.keys() {
            if !matches!(k.as_str(), "text" | "url" | "anchor" | "tooltip") {
                return Err(Error::with_path(
                    ErrorKind::UnknownField { field: k.clone() },
                    format!("{item_path}.{k}"),
                ));
            }
        }
        let text = match obj.get("text") {
            Some(Value::String(s)) => s.clone(),
            Some(_) => {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: "text".into(),
                        expected: TypeName::String,
                    },
                    format!("{item_path}.text"),
                ));
            }
            None => {
                return Err(Error::with_path(
                    ErrorKind::MissingField {
                        field: "text".into(),
                    },
                    format!("{item_path}.text"),
                ));
            }
        };
        let url = parse_optional_str(obj.get("url"), &item_path, "url")?;
        let anchor = parse_optional_str(obj.get("anchor"), &item_path, "anchor")?;
        match (&url, &anchor) {
            (Some(_), Some(_)) => {
                return Err(Error::with_path(
                    ErrorKind::MutuallyExclusive {
                        a: "url".into(),
                        b: "anchor".into(),
                    },
                    item_path.clone(),
                ));
            }
            (None, None) => {
                return Err(Error::with_path(ErrorKind::HyperlinkNeedTarget, item_path));
            }
            _ => {}
        }
        if let Some(u) = url.as_deref() {
            if u.is_empty() {
                return Err(Error::with_path(
                    ErrorKind::EmptyHyperlinkUrl,
                    format!("{item_path}.url"),
                ));
            }
        }
        if let Some(a) = anchor.as_deref() {
            if a.is_empty() || a.starts_with('_') || a.chars().any(char::is_whitespace) {
                return Err(Error::with_path(
                    ErrorKind::InvalidBookmarkName,
                    format!("{item_path}.anchor"),
                ));
            }
        }
        let tooltip = parse_optional_str(obj.get("tooltip"), &item_path, "tooltip")?;
        out.push(NewHyperlink {
            text,
            url,
            anchor,
            tooltip,
        });
    }
    Ok(out)
}

fn parse_bookmarks(v: Option<&Value>, prefix: &str) -> Result<Vec<NewBookmark>, Error> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    let path = format!("{prefix}.bookmarks");
    let arr = v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "bookmarks".into(),
                expected: TypeName::Array,
            },
            path.clone(),
        )
    })?;
    let mut out = Vec::new();
    for (j, item) in arr.iter().enumerate() {
        let item_path = format!("{path}[{j}]");
        let obj = item.as_object().ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: item_path.clone(),
                    expected: TypeName::Object,
                },
                item_path.clone(),
            )
        })?;
        for k in obj.keys() {
            if !matches!(k.as_str(), "name" | "end") {
                return Err(Error::with_path(
                    ErrorKind::UnknownField { field: k.clone() },
                    format!("{item_path}.{k}"),
                ));
            }
        }
        let name = match obj.get("name") {
            None => None,
            Some(Value::String(s)) => {
                if s.is_empty() || s.starts_with('_') || s.chars().any(char::is_whitespace) {
                    return Err(Error::with_path(
                        ErrorKind::InvalidBookmarkName,
                        format!("{item_path}.name"),
                    ));
                }
                Some(s.clone())
            }
            Some(_) => {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: "name".into(),
                        expected: TypeName::String,
                    },
                    format!("{item_path}.name"),
                ));
            }
        };
        let end = parse_index1(obj.get("end"), &format!("{item_path}.end"))?;
        out.push(NewBookmark { name, end });
    }
    Ok(out)
}

fn parse_fields(v: Option<&Value>, prefix: &str) -> Result<Vec<NewField>, Error> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    let path = format!("{prefix}.fields");
    let arr = v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "fields".into(),
                expected: TypeName::Array,
            },
            path.clone(),
        )
    })?;
    let mut out = Vec::new();
    for (j, item) in arr.iter().enumerate() {
        let item_path = format!("{path}[{j}]");
        let obj = item.as_object().ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: item_path.clone(),
                    expected: TypeName::Object,
                },
                item_path.clone(),
            )
        })?;
        for k in obj.keys() {
            if !matches!(
                k.as_str(),
                "instr" | "result" | "code" | "arg" | "name" | "items" | "checked"
            ) {
                return Err(Error::with_path(
                    ErrorKind::UnknownField { field: k.clone() },
                    format!("{item_path}.{k}"),
                ));
            }
        }
        let body = super::field::parse_field_body(obj, &item_path)?;
        out.push(NewField {
            instr: body.instr,
            result: body.result,
            form: body.form,
        });
    }
    Ok(out)
}

fn parse_maths(v: Option<&Value>, prefix: &str) -> Result<Vec<NewMath>, Error> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    let path = format!("{prefix}.maths");
    let arr = v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "maths".into(),
                expected: TypeName::Array,
            },
            path.clone(),
        )
    })?;
    let mut out = Vec::new();
    for (j, item) in arr.iter().enumerate() {
        let item_path = format!("{path}[{j}]");
        let obj = item.as_object().ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: item_path.clone(),
                    expected: TypeName::Object,
                },
                item_path.clone(),
            )
        })?;
        for k in obj.keys() {
            if !matches!(k.as_str(), "text" | "kind" | "expr" | "jc") {
                return Err(Error::with_path(
                    ErrorKind::UnknownField { field: k.clone() },
                    format!("{item_path}.{k}"),
                ));
            }
        }
        let body = math_expr::parse_math_body(obj, &item_path)?;
        let kind = match obj.get("kind") {
            None => MathKind::Inline,
            Some(Value::String(s)) if s == "inline" => MathKind::Inline,
            Some(Value::String(s)) if s == "display" => MathKind::Display,
            Some(Value::String(_)) => {
                return Err(Error::with_path(
                    ErrorKind::InvalidMathKind,
                    format!("{item_path}.kind"),
                ));
            }
            Some(_) => {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: "kind".into(),
                        expected: TypeName::String,
                    },
                    format!("{item_path}.kind"),
                ));
            }
        };
        let jc = match obj.get("jc") {
            None => None,
            Some(Value::String(s)) if super::math::MATH_JC.contains(&s.as_str()) => Some(s.clone()),
            Some(Value::String(_)) => {
                return Err(Error::with_path(
                    ErrorKind::InvalidMathJc,
                    format!("{item_path}.jc"),
                ));
            }
            Some(_) => {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: "jc".into(),
                        expected: TypeName::String,
                    },
                    format!("{item_path}.jc"),
                ));
            }
        };
        if jc.is_some() && kind == MathKind::Inline {
            return Err(Error::with_path(
                ErrorKind::MathJcOnInline,
                format!("{item_path}.jc"),
            ));
        }
        out.push(NewMath { body, kind, jc });
    }
    Ok(out)
}

fn parse_sdts(v: Option<&Value>, prefix: &str) -> Result<Vec<NewSdt>, Error> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    let path = format!("{prefix}.contentControls");
    let arr = v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "contentControls".into(),
                expected: TypeName::Array,
            },
            path.clone(),
        )
    })?;
    let mut out = Vec::new();
    for (j, item) in arr.iter().enumerate() {
        let item_path = format!("{path}[{j}]");
        let obj = item.as_object().ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: item_path.clone(),
                    expected: TypeName::Object,
                },
                item_path.clone(),
            )
        })?;
        for k in obj.keys() {
            if !matches!(
                k.as_str(),
                "text" | "kind" | "tag" | "alias" | "items" | "fullDate" | "file" | "checked"
            ) {
                return Err(Error::with_path(
                    ErrorKind::UnknownField { field: k.clone() },
                    format!("{item_path}.{k}"),
                ));
            }
        }
        let kind = match obj.get("kind") {
            None => SdtKind::PlainText,
            Some(Value::String(s)) => SdtKind::parse(s).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::InvalidContentControlKind,
                    format!("{item_path}.kind"),
                )
            })?,
            Some(_) => {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: "kind".into(),
                        expected: TypeName::String,
                    },
                    format!("{item_path}.kind"),
                ));
            }
        };
        let text_opt = match obj.get("text") {
            Some(Value::String(s)) if !s.is_empty() => Some(s.clone()),
            Some(Value::String(_)) => {
                return Err(Error::with_path(
                    ErrorKind::EmptyContentControlText,
                    format!("{item_path}.text"),
                ));
            }
            Some(_) => {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: "text".into(),
                        expected: TypeName::String,
                    },
                    format!("{item_path}.text"),
                ));
            }
            None => None,
        };
        let tag = parse_optional_str(obj.get("tag"), &item_path, "tag")?;
        let alias = parse_optional_str(obj.get("alias"), &item_path, "alias")?;
        let items = parse_sdt_items(obj.get("items"), &item_path)?;
        if !items.is_empty() && !matches!(kind, SdtKind::DropDown | SdtKind::ComboBox) {
            return Err(Error::with_path(
                ErrorKind::ContentControlFieldMismatch,
                format!("{item_path}.items"),
            ));
        }
        let full_date = parse_optional_str(obj.get("fullDate"), &item_path, "fullDate")?;
        if full_date.is_some() && kind != SdtKind::Date {
            return Err(Error::with_path(
                ErrorKind::ContentControlFieldMismatch,
                format!("{item_path}.fullDate"),
            ));
        }
        if let Some(fd) = &full_date {
            if fd.is_empty() {
                return Err(Error::with_path(
                    ErrorKind::EmptyContentControlText,
                    format!("{item_path}.fullDate"),
                ));
            }
        }
        let file = match obj.get("file") {
            None => None,
            Some(Value::String(s)) if !s.is_empty() => Some(s.clone()),
            Some(Value::String(_)) => {
                return Err(Error::with_path(
                    ErrorKind::MissingField {
                        field: "file".into(),
                    },
                    format!("{item_path}.file"),
                ));
            }
            Some(_) => {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: "file".into(),
                        expected: TypeName::NonEmptyString,
                    },
                    format!("{item_path}.file"),
                ));
            }
        };
        if file.is_some() && kind != SdtKind::Picture {
            return Err(Error::with_path(
                ErrorKind::ContentControlFieldMismatch,
                format!("{item_path}.file"),
            ));
        }
        let checked = parse_optional_bool(obj.get("checked"), &item_path, "checked")?;
        if checked.is_some() && kind != SdtKind::Checkbox {
            return Err(Error::with_path(
                ErrorKind::ContentControlFieldMismatch,
                format!("{item_path}.checked"),
            ));
        }
        match kind {
            SdtKind::Picture => {
                if file.is_none() {
                    return Err(Error::with_path(
                        ErrorKind::MissingField {
                            field: "file".into(),
                        },
                        format!("{item_path}.file"),
                    ));
                }
                if text_opt.is_some() {
                    return Err(Error::with_path(
                        ErrorKind::ContentControlFieldMismatch,
                        format!("{item_path}.text"),
                    ));
                }
            }
            SdtKind::Checkbox => {
                if text_opt.is_some() {
                    return Err(Error::with_path(
                        ErrorKind::ContentControlFieldMismatch,
                        format!("{item_path}.text"),
                    ));
                }
            }
            _ => {
                if text_opt.is_none() {
                    return Err(Error::with_path(
                        ErrorKind::MissingField {
                            field: "text".into(),
                        },
                        format!("{item_path}.text"),
                    ));
                }
            }
        }
        out.push(NewSdt {
            text: text_opt.unwrap_or_default(),
            kind,
            tag,
            alias,
            items,
            full_date,
            file,
            checked: checked.unwrap_or(false),
        });
    }
    Ok(out)
}

fn parse_sdt_items(v: Option<&Value>, item_path: &str) -> Result<Vec<String>, Error> {
    match v {
        None => Ok(Vec::new()),
        Some(Value::Array(arr)) => {
            if arr.is_empty() {
                return Err(Error::with_path(
                    ErrorKind::InvalidContentControlItems,
                    format!("{item_path}.items"),
                ));
            }
            let mut out = Vec::new();
            for (j, item) in arr.iter().enumerate() {
                let s = item.as_str().ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::WrongType {
                            field: "items".into(),
                            expected: TypeName::String,
                        },
                        format!("{item_path}.items[{j}]"),
                    )
                })?;
                if s.is_empty() {
                    return Err(Error::with_path(
                        ErrorKind::InvalidContentControlItems,
                        format!("{item_path}.items[{j}]"),
                    ));
                }
                out.push(s.to_string());
            }
            Ok(out)
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "items".into(),
                expected: TypeName::Array,
            },
            format!("{item_path}.items"),
        )),
    }
}

fn parse_para_num(
    obj: &serde_json::Map<String, Value>,
    path: &str,
) -> Result<(Option<u32>, u32), Error> {
    let num = parse_index1(obj.get("num"), &format!("{path}.num"))?;
    let ilvl = match obj.get("ilvl") {
        None => 0,
        Some(Value::Number(n)) => {
            let Some(u) = n.as_u64() else {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: "ilvl".into(),
                        expected: TypeName::Integer,
                    },
                    format!("{path}.ilvl"),
                ));
            };
            if u > 8 {
                return Err(Error::with_path(
                    ErrorKind::InvalidIlvl,
                    format!("{path}.ilvl"),
                ));
            }
            u as u32
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "ilvl".into(),
                    expected: TypeName::Integer,
                },
                format!("{path}.ilvl"),
            ));
        }
    };
    if obj.get("ilvl").is_some() && num.is_none() {
        return Err(Error::with_path(
            ErrorKind::MissingField {
                field: "num".into(),
            },
            format!("{path}.num"),
        ));
    }
    Ok((num, ilvl))
}

fn parse_numberings(v: Option<&Value>, path: &str) -> Result<Vec<NumberingKind>, Error> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    let arr = v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "numberings".into(),
                expected: TypeName::Array,
            },
            path,
        )
    })?;
    let mut out = Vec::new();
    for (i, item) in arr.iter().enumerate() {
        let item_path = format!("{path}[{i}]");
        let obj = item.as_object().ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: item_path.clone(),
                    expected: TypeName::Object,
                },
                item_path.clone(),
            )
        })?;
        for k in obj.keys() {
            if k.as_str() != "kind" {
                return Err(Error::with_path(
                    ErrorKind::UnknownField { field: k.clone() },
                    format!("{item_path}.{k}"),
                ));
            }
        }
        let kind = match obj.get("kind") {
            Some(Value::String(s)) => super::numbering::parse_kind(s).ok_or_else(|| {
                Error::with_path(ErrorKind::InvalidNumberingKind, format!("{item_path}.kind"))
            })?,
            Some(_) => {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: "kind".into(),
                        expected: TypeName::String,
                    },
                    format!("{item_path}.kind"),
                ));
            }
            None => {
                return Err(Error::with_path(
                    ErrorKind::MissingField {
                        field: "kind".into(),
                    },
                    format!("{item_path}.kind"),
                ));
            }
        };
        out.push(kind);
    }
    Ok(out)
}

fn parse_tocs(v: Option<&Value>, path: &str, para_n: usize) -> Result<Vec<NewToc>, Error> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    let arr = v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "tocs".into(),
                expected: TypeName::Array,
            },
            path,
        )
    })?;
    let mut out = Vec::new();
    for (i, item) in arr.iter().enumerate() {
        let item_path = format!("{path}[{i}]");
        let obj = item.as_object().ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: item_path.clone(),
                    expected: TypeName::Object,
                },
                item_path.clone(),
            )
        })?;
        for k in obj.keys() {
            if !matches!(k.as_str(), "kind" | "instr" | "after") {
                return Err(Error::with_path(
                    ErrorKind::UnknownField { field: k.clone() },
                    format!("{item_path}.{k}"),
                ));
            }
        }
        let kind = match obj.get("kind") {
            None => None,
            Some(Value::String(s)) => Some(
                field::parse_toc_kind(s)
                    .map_err(|kind| Error::with_path(kind, format!("{item_path}.kind")))?,
            ),
            Some(_) => {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: "kind".into(),
                        expected: TypeName::String,
                    },
                    format!("{item_path}.kind"),
                ));
            }
        };
        let instr = match obj.get("instr") {
            None => None,
            Some(Value::String(s)) => {
                if s.trim().is_empty() {
                    return Err(Error::with_path(
                        ErrorKind::EmptyFieldInstr,
                        format!("{item_path}.instr"),
                    ));
                }
                if !field::instr_is_toc_or_toa(s) {
                    return Err(Error::with_path(
                        ErrorKind::InvalidTocKind,
                        format!("{item_path}.instr"),
                    ));
                }
                Some(s.clone())
            }
            Some(_) => {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: "instr".into(),
                        expected: TypeName::String,
                    },
                    format!("{item_path}.instr"),
                ));
            }
        };
        let instr = match instr {
            Some(s) => s,
            None => kind
                .unwrap_or(field::TocKind::Toc)
                .default_instr()
                .to_string(),
        };
        let after = parse_index1(obj.get("after"), &format!("{item_path}.after"))?;
        if let Some(n) = after {
            if n as usize > para_n {
                return Err(Error::with_path(
                    ErrorKind::CreateIndexOutOfRange {
                        field: "after".into(),
                        max: para_n as i64,
                    },
                    format!("{item_path}.after"),
                ));
            }
        }
        out.push(NewToc { after, instr });
    }
    Ok(out)
}

fn parse_index1(v: Option<&Value>, path: &str) -> Result<Option<u32>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Number(n)) => {
            let Some(u) = n.as_u64() else {
                if n.as_i64().is_some_and(|x| x < 1) {
                    return Err(Error::with_path(
                        ErrorKind::MinValue {
                            field: path.rsplit('.').next().unwrap_or("index").into(),
                            min: 1,
                        },
                        path,
                    ));
                }
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: path.rsplit('.').next().unwrap_or("index").into(),
                        expected: TypeName::Integer,
                    },
                    path,
                ));
            };
            if u == 0 {
                return Err(Error::with_path(
                    ErrorKind::MinValue {
                        field: path.rsplit('.').next().unwrap_or("index").into(),
                        min: 1,
                    },
                    path,
                ));
            }
            if u > u32::MAX as u64 {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: path.rsplit('.').next().unwrap_or("index").into(),
                        expected: TypeName::Integer,
                    },
                    path,
                ));
            }
            Ok(Some(u as u32))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: path.rsplit('.').next().unwrap_or("index").into(),
                expected: TypeName::Integer,
            },
            path,
        )),
    }
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
        || !create_round4::is_empty(extra)
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
                (id, InsertTocPlace::After(p))
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
    blocks: &[create_round4::CaptionBlock],
    build: impl Fn(&create_round4::CaptionBlock, String, InsertCaptionPlace) -> DocxOp,
    end: InsertCaptionPlace,
    after: fn(para::BodyPara) -> InsertCaptionPlace,
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
                (id, after(p))
            }
        };
        ops.push(build(block, id, place));
    }
    Ok(())
}

fn push_instr_blocks<P: Clone>(
    ops: &mut Vec<DocxOp>,
    blocks: &[create_round4::AfterBlock],
    build: impl Fn(&create_round4::AfterBlock, String, P) -> DocxOp,
    end: P,
    after: fn(para::BodyPara) -> P,
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
                (id, after(p))
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
        if let Some(op) = create_round5::to_set_para_format(id.clone(), host.clone(), &p.remain) {
            ops.push(DocxOp::SetParaFormat(op));
        }
        if let Some(runs) = &p.runs {
            for (ri, r) in runs.iter().enumerate() {
                if let Some(fmt) = &r.format {
                    let run_id = format!("{id}/run:{}", ri + 1);
                    ops.push(DocxOp::SetRunFormat(create_round5::to_set_run_format(
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
        ops.push(DocxOp::GroupShapes(create_round5::to_group_shapes(
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
            || cf.v_align.is_some();
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

fn needs_xml_space(text: &str) -> bool {
    text.starts_with(' ')
        || text.starts_with('\t')
        || text.ends_with(' ')
        || text.ends_with('\t')
        || text.contains('\n')
}

fn styles_xml() -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:styles xmlns:w="{WML_NS}"><w:docDefaults><w:rPrDefault><w:rPr><w:rFonts w:ascii="Calibri" w:hAnsi="Calibri" w:eastAsia="宋体"/><w:sz w:val="22"/><w:color w:val="000000"/></w:rPr></w:rPrDefault><w:pPrDefault/></w:docDefaults><w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/><w:qFormat/></w:style>{h1}{h2}{h3}</w:styles>"#,
        h1 = heading_style("Heading1", "heading 1", "0", "32"),
        h2 = heading_style("Heading2", "heading 2", "1", "26"),
        h3 = heading_style("Heading3", "heading 3", "2", "24"),
    )
}

fn settings_xml(even_and_odd: bool) -> String {
    let even = if even_and_odd {
        "<w:evenAndOddHeaders/>"
    } else {
        ""
    };
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:settings xmlns:w="{WML_NS}">{even}<w:compat><w:compatSetting w:name="compatibilityMode" w:uri="http://schemas.microsoft.com/office/word" w:val="15"/></w:compat></w:settings>"#
    )
}

fn heading_style(id: &str, name: &str, outline: &str, sz: &str) -> String {
    format!(
        r#"<w:style w:type="paragraph" w:styleId="{id}"><w:name w:val="{name}"/><w:basedOn w:val="Normal"/><w:uiPriority w:val="9"/><w:qFormat/><w:pPr><w:outlineLvl w:val="{outline}"/><w:spacing w:before="240" w:after="0"/></w:pPr><w:rPr><w:b/><w:sz w:val="{sz}"/><w:color w:val="000000"/></w:rPr></w:style>"#
    )
}

fn core_xml() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:dcterms="http://purl.org/dc/terms/" xmlns:dcmitype="http://purl.org/dc/dcmitype/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"><dc:title/><dc:creator>ooxp</dc:creator></cp:coreProperties>"#.to_string()
}

fn app_xml() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties" xmlns:vt="http://schemas.openxmlformats.org/officeDocument/2006/docPropsVTypes"><Application>ooxp</Application></Properties>"#.to_string()
}
