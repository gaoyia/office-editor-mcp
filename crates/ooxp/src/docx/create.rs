//! 从领域 JSON 生成 Transitional 文档。

use std::collections::HashSet;

use serde_json::Value;

use crate::error::{Error, ErrorKind, TypeName};
use crate::opc::{
    ContentTypes, Package, Relationship, Relationships, CONTENT_TYPES_PART, ROOT_RELS_PART,
};
use crate::xml::{self, Element, Node};

use super::apply;
use super::charts::{self, ChartType, InsertChartOp, NewSeries};
use super::hf::{HfKind, HfType};
use super::ole::InsertOleOp;
use super::para;
use super::picture::{self, PicKind, WrapKind, WrapPos};
use super::request::{DocxOp, InsertCommentOp, InsertNoteOp};
use super::sect::SectionBreakType;
use super::smartart::{self, InsertSpec};
use super::tbl;
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
}

struct NewPic {
    file: String,
    cx: Option<i64>,
    cy: Option<i64>,
    kind: PicKind,
    wrap: Option<WrapKind>,
    pos: WrapPos,
}

struct CreateChart {
    chart_type: ChartType,
    title: Option<String>,
    series: Vec<NewSeries>,
    kind: PicKind,
    wrap: Option<WrapKind>,
    cx: i64,
    cy: i64,
}

struct NewComment {
    text: String,
    author: String,
    initials: String,
    date: Option<String>,
}

struct NewPara {
    text: String,
    style: Option<&'static str>,
    boxes: Vec<NewBox>,
    pictures: Vec<NewPic>,
    smart_arts: Vec<InsertSpec>,
    charts: Vec<CreateChart>,
    ole_objects: Vec<String>,
    footnote: Option<String>,
    endnote: Option<String>,
    comments: Vec<NewComment>,
    section_type: Option<SectionBreakType>,
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
            footnote: None,
            endnote: None,
            comments: Vec::new(),
            section_type: None,
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
            || self.footnote.is_some()
            || self.endnote.is_some()
            || !self.comments.is_empty()
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
}

struct NewTable {
    after: u32,
    cells: Vec<Vec<String>>,
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
            "paragraphs" | "headers" | "footers" | "tables" | "props" | "protection"
        ) {
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
    let tables = parse_tables(obj.get("tables"), "document.tables", paras.len())?;

    let header_items = parse_hf_side(obj.get("headers"), "headers")?;
    let footer_items = parse_hf_side(obj.get("footers"), "footers")?;
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
    let bytes = apply_follow_ups(bytes, &paras, &header_items, &footer_items)?;
    if obj.get("protection").is_some() {
        let mut pkg = crate::opc::Package::open_bytes(bytes)?;
        super::protect::apply_create(&mut pkg, obj.get("protection"))?;
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
            if !matches!(k.as_str(), "type" | "paragraphs" | "tables") {
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
        out.push(HfCreateItem {
            hf_type,
            paras,
            tables,
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
            "footnote",
            "endnote",
            "comments",
            "sectionType",
        ]
        .as_slice(),
        ParaKind::HeaderFooter => [
            "text",
            "pictures",
            "smartArts",
            "charts",
            "oleObjects",
            "footnote",
            "endnote",
            "comments",
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
        ParaKind::Body => parse_boxes(obj.get("textBoxes"), i)?,
        ParaKind::HeaderFooter => Vec::new(),
    };
    let pictures = parse_pictures(obj.get("pictures"), path)?;
    let smart_arts = parse_smartarts(obj.get("smartArts"), path)?;
    let charts = parse_charts(obj.get("charts"), path)?;
    let ole_objects = parse_ole_objects(obj.get("oleObjects"), path)?;
    let footnote = parse_optional_str(obj.get("footnote"), path, "footnote")?;
    let endnote = parse_optional_str(obj.get("endnote"), path, "endnote")?;
    let comments = parse_comments(obj.get("comments"), path)?;
    let section_type = match kind {
        ParaKind::Body => parse_create_section_type(obj.get("sectionType"), path)?,
        ParaKind::HeaderFooter => None,
    };
    Ok(NewPara {
        text,
        style,
        boxes,
        pictures,
        smart_arts,
        charts,
        ole_objects,
        footnote,
        endnote,
        comments,
        section_type,
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

fn parse_boxes(v: Option<&Value>, i: usize) -> Result<Vec<NewBox>, Error> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    let arr = v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: format!("document.paragraphs[{i}].textBoxes"),
                expected: TypeName::Array,
            },
            format!("document.paragraphs[{i}].textBoxes"),
        )
    })?;
    let mut out = Vec::new();
    for (j, b) in arr.iter().enumerate() {
        let obj = b.as_object().ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: format!("document.paragraphs[{i}].textBoxes[{j}]"),
                    expected: TypeName::Object,
                },
                format!("document.paragraphs[{i}].textBoxes[{j}]"),
            )
        })?;
        for k in obj.keys() {
            if !matches!(k.as_str(), "text" | "cx" | "cy") {
                return Err(Error::with_path(
                    ErrorKind::UnknownField { field: k.clone() },
                    format!("document.paragraphs[{i}].textBoxes[{j}].{k}"),
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
                    format!("document.paragraphs[{i}].textBoxes[{j}].text"),
                ));
            }
        };
        let prefix = format!("document.paragraphs[{i}].textBoxes[{j}]");
        let cx = parse_emu(obj.get("cx"), &prefix, "cx")?.unwrap_or(DEFAULT_CX);
        let cy = parse_emu(obj.get("cy"), &prefix, "cy")?.unwrap_or(DEFAULT_CY);
        out.push(NewBox { text, cx, cy });
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

fn para_xml(
    p: &NewPara,
    id: &str,
    next_para: &mut u32,
    doc_pr: &mut u32,
    first_break: bool,
    refs_xml: &str,
    title_pg: bool,
) -> String {
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
    let space = if needs_xml_space(&p.text) {
        r#" xml:space="preserve""#
    } else {
        ""
    };
    let t = if p.text.is_empty() {
        String::new()
    } else {
        format!("<w:r><w:t{space}>{}</w:t></w:r>", xml::escape_text(&p.text))
    };
    let mut boxes = String::new();
    for b in &p.boxes {
        let inner_id = format!("{:08X}", *next_para);
        *next_para += 1;
        boxes.push_str(&format!(
            "<w:r>{}</w:r>",
            textbox::textbox_ac_xml(*doc_pr, b.cx, b.cy, &b.text, &inner_id)
        ));
        *doc_pr += 1;
    }
    format!(r#"<w:p w14:paraId="{id}" w14:textId="{id}">{ppr}{t}{boxes}</w:p>"#)
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
    let mut doc_pr = 1u32;
    for (i, p) in paras.iter().enumerate() {
        let id = format!("{:08X}", i as u32 + 1);
        para_xmls.push(para_xml(
            p,
            &id,
            &mut next_para,
            &mut doc_pr,
            first_break == Some(i),
            refs_xml,
            title_pg,
        ));
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
            if !matches!(k.as_str(), "cells" | "after") {
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
        out.push(NewTable { after, cells });
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

fn parse_smartarts(v: Option<&Value>, prefix: &str) -> Result<Vec<InsertSpec>, Error> {
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
                "layout" | "points" | "kind" | "wrap" | "cx" | "cy"
            ) {
                return Err(Error::with_path(
                    ErrorKind::UnknownField { field: k.clone() },
                    format!("{item_path}.{k}"),
                ));
            }
        }
        out.push(smartart::parse_spec(obj, &item_path)?);
    }
    Ok(out)
}

fn attach_smartarts(pkg: &mut Package, part: &str, paras: &[NewPara]) -> Result<(), Error> {
    if !paras.iter().any(NewPara::has_smartarts) {
        return Ok(());
    }
    let mut doc = xml::parse(pkg.get(part)?)?;
    for (i, para) in paras.iter().enumerate() {
        for spec in &para.smart_arts {
            smartart::attach_to_para(pkg, part, &mut doc.root, i, spec)?;
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
            let (rid, cx, cy) = picture::add_media(pkg, part, &pic.file, pic.cx, pic.cy)?;
            let doc_pr = picture::next_docpr_id(&doc.root);
            let p = nth_story_para_mut(&mut doc.root, i).ok_or_else(|| {
                Error::xml_detail(
                    crate::error::XmlIssue::Parse,
                    format!("{part} missing paragraph {i}"),
                )
            })?;
            picture::insert_prepared_drawing(
                p, doc_pr, &rid, cx, cy, pic.kind, pic.wrap, &pic.pos,
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

fn parse_comments(v: Option<&Value>, prefix: &str) -> Result<Vec<NewComment>, Error> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    let path = format!("{prefix}.comments");
    let arr = v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "comments".into(),
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
            if !matches!(k.as_str(), "text" | "author" | "initials" | "date") {
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
        out.push(NewComment {
            text,
            author,
            initials,
            date,
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
            "type" | "title" | "series" | "kind" | "wrap" | "cx" | "cy"
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
    if chart_type.needs_one_series() && series.len() != 1 {
        return Err(Error::with_path(
            ErrorKind::PieNeedsOneSeries,
            format!("{path}.series"),
        ));
    }
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
        title,
        series,
        kind,
        wrap,
        cx: cx.unwrap_or(charts::default_cx()),
        cy: cy.unwrap_or(charts::default_cy()),
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

fn apply_follow_ups(
    bytes: Vec<u8>,
    paras: &[NewPara],
    headers: &[HfCreateItem],
    footers: &[HfCreateItem],
) -> Result<Vec<u8>, Error> {
    let need = paras.iter().any(NewPara::has_follow_ups)
        || headers
            .iter()
            .any(|h| h.paras.iter().any(NewPara::has_follow_ups))
        || footers
            .iter()
            .any(|h| h.paras.iter().any(NewPara::has_follow_ups));
    if !need {
        return Ok(bytes);
    }
    let mut ops = Vec::new();
    push_story_ops(&mut ops, paras, None)?;
    for item in headers {
        push_story_ops(&mut ops, &item.paras, Some((HfKind::Header, item.hf_type)))?;
    }
    for item in footers {
        push_story_ops(&mut ops, &item.paras, Some((HfKind::Footer, item.hf_type)))?;
    }
    apply::apply(&bytes, &ops)
}

fn push_story_ops(
    ops: &mut Vec<DocxOp>,
    paras: &[NewPara],
    hf: Option<(HfKind, HfType)>,
) -> Result<(), Error> {
    for (i, p) in paras.iter().enumerate() {
        let id = story_para_id(hf, i);
        let host = para::parse_para_id_at(&id, "document")?;
        for ch in &p.charts {
            ops.push(DocxOp::InsertChart(InsertChartOp {
                id: id.clone(),
                para: host.clone(),
                chart_type: ch.chart_type,
                title: ch.title.clone(),
                series: ch.series.clone(),
                kind: ch.kind,
                wrap: ch.wrap,
                pos: WrapPos::default(),
                cx: ch.cx,
                cy: ch.cy,
            }));
        }
        for file in &p.ole_objects {
            ops.push(DocxOp::InsertOle(InsertOleOp {
                id: id.clone(),
                para: host.clone(),
                file: file.clone(),
            }));
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
        }
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
