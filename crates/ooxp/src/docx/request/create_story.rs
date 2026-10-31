use std::collections::HashSet;

use serde_json::Value;

use crate::error::{Error, ErrorKind, TypeName};

use super::super::hf::HfType;
use super::super::sect::SectionBreakType;
use super::create_types::*;

pub(crate) fn parse_hf_side(v: Option<&Value>, side: &str) -> Result<Vec<HfCreateItem>, Error> {
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
        let tables = super::create_table::parse_tables(
            obj.get("tables"),
            &format!("{item_path}.tables"),
            paras.len(),
        )?;
        let groups =
            super::para_remain::parse_groups(obj.get("groups"), &format!("{item_path}.groups"))?;
        out.push(HfCreateItem {
            hf_type,
            paras,
            tables,
            groups,
        });
    }
    Ok(out)
}

pub(crate) fn parse_hf_paras(v: Option<&Value>, path: &str) -> Result<Vec<NewPara>, Error> {
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

pub(crate) fn parse_para(
    v: &Value,
    path: &str,
    kind: ParaKind,
    i: usize,
) -> Result<NewPara, Error> {
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
            Some(Value::String(s)) => super::create_util::canonicalize_style(s, i)?,
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
        ParaKind::Body => super::create_drawing::parse_boxes(obj.get("textBoxes"), path)?,
        ParaKind::HeaderFooter => Vec::new(),
    };
    let pictures = super::create_picture::parse_pictures(obj.get("pictures"), path)?;
    let smart_arts = super::create_drawing::parse_smartarts(obj.get("smartArts"), path)?;
    let charts = super::create_chart::parse_charts(obj.get("charts"), path)?;
    let ole_objects = super::create_drawing::parse_ole_objects(obj.get("oleObjects"), path)?;
    let ink_canvases = super::create_drawing::parse_ink_canvases(obj.get("inkCanvases"), path)?;
    let model3ds = super::create_drawing::parse_model3ds(obj.get("model3ds"), path)?;
    let footnote = super::create_util::parse_optional_str(obj.get("footnote"), path, "footnote")?;
    let endnote = super::create_util::parse_optional_str(obj.get("endnote"), path, "endnote")?;
    let comments = super::create_comment::parse_comments(
        obj.get("comments"),
        &format!("{path}.comments"),
        false,
    )?;
    let hyperlinks = super::create_link::parse_hyperlinks(obj.get("hyperlinks"), path)?;
    let bookmarks = super::create_link::parse_bookmarks(obj.get("bookmarks"), path)?;
    let fields = super::create_field::parse_fields(obj.get("fields"), path)?;
    let maths = super::create_math::parse_maths(obj.get("maths"), path)?;
    let content_controls = super::create_sdt::parse_sdts(obj.get("contentControls"), path)?;
    let shapes = super::doc_extra::parse_shapes(obj.get("shapes"), path)?;
    let connectors = super::doc_extra::parse_connectors(obj.get("connectors"), path)?;
    let rubies = super::doc_extra::parse_rubies(obj.get("rubies"), path)?;
    let citations = super::doc_extra::parse_citations(obj.get("citations"), path)?;
    let index_entries = super::doc_extra::parse_index_entries(obj.get("indexEntries"), path)?;
    let cross_refs = super::doc_extra::parse_cross_refs(obj.get("crossRefs"), path)?;
    let edit_range = super::doc_extra::parse_edit_range(obj.get("editRange"), path)?;
    let (num, ilvl) = parse_para_num(obj, path)?;
    let section_type = match kind {
        ParaKind::Body => parse_create_section_type(obj.get("sectionType"), path)?,
        ParaKind::HeaderFooter => None,
    };
    let remain = super::para_remain::parse_para_remain(obj, path)?;
    let runs = super::para_remain::parse_runs(obj.get("runs"), path)?;
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

pub(crate) fn parse_create_section_type(
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

pub(crate) fn parse_para_num(
    obj: &serde_json::Map<String, Value>,
    path: &str,
) -> Result<(Option<u32>, u32), Error> {
    let num = super::create_util::parse_index1(obj.get("num"), &format!("{path}.num"))?;
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
