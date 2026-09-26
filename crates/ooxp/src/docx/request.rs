//! docx 自己的 scope 与 ops 解析。

use serde_json::Value;

use crate::error::{Error, ErrorKind, IdReason, ScopeNeed, TypeName};
use crate::json;

use super::hf::{HfKind, HfType};
use super::picture::{self, AlignH, AlignV, PicKind, PicTarget, WrapKind, WrapPos};
use super::para::{self, ParaTarget, TxbxTarget};

#[derive(Debug, Clone)]
pub struct InspectScope {
    /// 正文切片起点；与 `max_paras` 成对出现。
    pub from: Option<u32>,
    pub max_paras: Option<u32>,
    /// 为 true 时展开页眉页脚段落。
    pub header_footer: bool,
    /// 为 true 时列出页眉页脚里的图。
    pub pictures: bool,
    /// 为 true 时列出文本框。
    pub text_boxes: bool,
    /// 为 true 时展开框内段落。
    pub text_box_content: bool,
}

#[derive(Debug, Clone)]
pub struct SetParaTextOp {
    pub id: String,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct SetHfDistanceOp {
    pub id: String,
    pub n: usize,
    pub header_distance: Option<u32>,
    pub footer_distance: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct SetHfLinkOp {
    pub id: String,
    pub n: usize,
    pub kind: HfKind,
    pub hf_type: HfType,
    pub linked: bool,
}

#[derive(Debug, Clone)]
pub struct ReplacePictureOp {
    pub id: String,
    pub target: PicTarget,
    pub file: String,
}

#[derive(Debug, Clone)]
pub struct InsertPictureOp {
    pub id: String,
    pub para: ParaTarget,
    pub file: String,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct DeletePictureOp {
    pub id: String,
    pub target: PicTarget,
}

#[derive(Debug, Clone)]
pub struct SetPictureSizeOp {
    pub id: String,
    pub target: PicTarget,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct SetPictureWrapOp {
    pub id: String,
    pub target: PicTarget,
    pub kind: PicKind,
    pub wrap: Option<WrapKind>,
    pub pos: WrapPos,
}

#[derive(Debug, Clone)]
pub struct InsertTextBoxOp {
    pub id: String,
    pub host: para::HostPara,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct DeleteTextBoxOp {
    pub id: String,
    pub target: TxbxTarget,
}

#[derive(Debug, Clone)]
pub struct SetTextBoxSizeOp {
    pub id: String,
    pub target: TxbxTarget,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct SetTextBoxWrapOp {
    pub id: String,
    pub target: TxbxTarget,
    pub kind: PicKind,
    pub wrap: Option<WrapKind>,
    pub pos: WrapPos,
}

#[derive(Debug, Clone)]
pub struct SetTextBoxFillOp {
    pub id: String,
    pub target: TxbxTarget,
    pub color: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SetTextBoxLineOp {
    pub id: String,
    pub target: TxbxTarget,
    pub color: Option<String>,
    pub width: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct SetTextBoxDirectionOp {
    pub id: String,
    pub target: TxbxTarget,
    pub vertical: bool,
}

#[derive(Debug, Clone)]
pub struct InsertParaOp {
    pub id: String,
    pub after: Option<ParaTarget>,
    pub box_target: TxbxTarget,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct DeleteParaOp {
    pub id: String,
    pub inner: ParaTarget,
}

#[derive(Debug, Clone)]
pub struct SetTextBoxLinkOp {
    pub id: String,
    pub target: TxbxTarget,
    pub next: Option<TxbxTarget>,
    pub next_id: Option<String>,
}

#[derive(Debug, Clone)]
pub enum DocxOp {
    SetParaText(SetParaTextOp),
    SetHeaderFooterDistance(SetHfDistanceOp),
    SetHeaderFooterLink(SetHfLinkOp),
    ReplacePicture(ReplacePictureOp),
    InsertPicture(InsertPictureOp),
    DeletePicture(DeletePictureOp),
    SetPictureSize(SetPictureSizeOp),
    SetPictureWrap(SetPictureWrapOp),
    InsertTextBox(InsertTextBoxOp),
    DeleteTextBox(DeleteTextBoxOp),
    SetTextBoxSize(SetTextBoxSizeOp),
    SetTextBoxWrap(SetTextBoxWrapOp),
    SetTextBoxFill(SetTextBoxFillOp),
    SetTextBoxLine(SetTextBoxLineOp),
    SetTextBoxDirection(SetTextBoxDirectionOp),
    InsertPara(InsertParaOp),
    DeletePara(DeleteParaOp),
    SetTextBoxLink(SetTextBoxLinkOp),
}

pub fn parse_scope(v: Option<&Value>) -> Result<Option<InspectScope>, Error> {
    let Some(v) = v else {
        return Ok(None);
    };
    let obj = v
        .as_object()
        .ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: "scope".into(),
                    expected: TypeName::Object,
                },
                "scope",
            )
        })?;
    if obj.is_empty() {
        return Ok(None);
    }
    json::deny_unknown(
        obj,
        &[
            "from",
            "maxParas",
            "headerFooter",
            "pictures",
            "textBoxes",
            "textBoxContent",
        ],
    )?;

    let header_footer = parse_header_footer(obj.get("headerFooter"))?;
    let pictures = parse_bool_field(obj.get("pictures"), "pictures")?;
    let text_boxes = parse_bool_field(obj.get("textBoxes"), "textBoxes")?;
    let text_box_content = parse_bool_field(obj.get("textBoxContent"), "textBoxContent")?;
    if text_box_content && !text_boxes {
        return Err(Error::with_path(
            ErrorKind::ScopeFieldRequires {
                field: "textBoxContent".into(),
                required: "textBoxes".into(),
            },
            "scope.textBoxContent",
        ));
    }
    let from = obj.get("from");
    let max = obj.get("maxParas");
    match (from, max) {
        (None, None) => {
            if !obj.contains_key("headerFooter")
                && !obj.contains_key("pictures")
                && !obj.contains_key("textBoxes")
                && !obj.contains_key("textBoxContent")
            {
                return Ok(None);
            }
            Ok(Some(InspectScope {
                from: None,
                max_paras: None,
                header_footer,
                pictures,
                text_boxes,
                text_box_content,
            }))
        }
        (Some(f), Some(m)) => {
            let from = f.as_u64().ok_or_else(|| {
                Error::with_path(
                    ErrorKind::WrongType {
                        field: "from".into(),
                        expected: TypeName::Integer,
                    },
                    "scope.from",
                )
            })?;
            let max = m.as_u64().ok_or_else(|| {
                Error::with_path(
                    ErrorKind::WrongType {
                        field: "maxParas".into(),
                        expected: TypeName::Integer,
                    },
                    "scope.maxParas",
                )
            })?;
            if from < 1 {
                return Err(Error::with_path(
                    ErrorKind::MinValue {
                        field: "from".into(),
                        min: 1,
                    },
                    "scope.from",
                ));
            }
            if !(1..=5000).contains(&max) {
                return Err(Error::with_path(
                    ErrorKind::OutOfRange {
                        field: "maxParas".into(),
                        min: 1,
                        max: 5000,
                    },
                    "scope.maxParas",
                ));
            }
            Ok(Some(InspectScope {
                from: Some(from as u32),
                max_paras: Some(max as u32),
                header_footer,
                pictures,
                text_boxes,
                text_box_content,
            }))
        }
        _ => Err(Error::with_path(
            ErrorKind::ScopeRequired {
                need: ScopeNeed::FromAndMaxParas,
            },
            "scope",
        )),
    }
}

fn parse_header_footer(v: Option<&Value>) -> Result<bool, Error> {
    parse_bool_field(v, "headerFooter")
}

fn parse_bool_field(v: Option<&Value>, field: &str) -> Result<bool, Error> {
    match v {
        None => Ok(false),
        Some(Value::Bool(b)) => Ok(*b),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Boolean,
            },
            format!("scope.{field}"),
        )),
    }
}

pub fn parse_ops(ops: &[Value]) -> Result<Vec<DocxOp>, Error> {
    let mut out = Vec::new();
    for (i, op) in ops.iter().enumerate() {
        let obj = op.as_object().ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: format!("ops[{i}]"),
                    expected: TypeName::Object,
                },
                format!("ops[{i}]"),
            )
        })?;
        let op_name = obj.get("op").and_then(Value::as_str).ok_or_else(|| {
            Error::with_path(
                ErrorKind::MissingField {
                    field: "op".into(),
                },
                format!("ops[{i}].op"),
            )
        })?;
        if op_name == "setCell" || op_name == "setShapeText" {
            return Err(Error::with_path(
                ErrorKind::OpNotForFormat {
                    format: "docx".into(),
                    op: op_name.to_string(),
                },
                format!("ops[{i}].op"),
            ));
        }
        match op_name {
            "setParaText" => out.push(DocxOp::SetParaText(parse_set_para_text(obj, i)?)),
            "setHeaderFooterDistance" => {
                out.push(DocxOp::SetHeaderFooterDistance(parse_set_hf_distance(obj, i)?))
            }
            "setHeaderFooterLink" => {
                out.push(DocxOp::SetHeaderFooterLink(parse_set_hf_link(obj, i)?))
            }
            "replacePicture" => out.push(DocxOp::ReplacePicture(parse_replace_pic(obj, i)?)),
            "insertPicture" => out.push(DocxOp::InsertPicture(parse_insert_pic(obj, i)?)),
            "deletePicture" => out.push(DocxOp::DeletePicture(parse_delete_pic(obj, i)?)),
            "setPictureSize" => out.push(DocxOp::SetPictureSize(parse_set_pic_size(obj, i)?)),
            "setPictureWrap" => out.push(DocxOp::SetPictureWrap(parse_set_pic_wrap(obj, i)?)),
            "insertTextBox" => out.push(DocxOp::InsertTextBox(parse_insert_txbx(obj, i)?)),
            "deleteTextBox" => out.push(DocxOp::DeleteTextBox(parse_delete_txbx(obj, i)?)),
            "setTextBoxSize" => out.push(DocxOp::SetTextBoxSize(parse_set_txbx_size(obj, i)?)),
            "setTextBoxWrap" => out.push(DocxOp::SetTextBoxWrap(parse_set_txbx_wrap(obj, i)?)),
            "setTextBoxFill" => out.push(DocxOp::SetTextBoxFill(parse_set_txbx_fill(obj, i)?)),
            "setTextBoxLine" => out.push(DocxOp::SetTextBoxLine(parse_set_txbx_line(obj, i)?)),
            "setTextBoxDirection" => {
                out.push(DocxOp::SetTextBoxDirection(parse_set_txbx_dir(obj, i)?))
            }
            "insertPara" => out.push(DocxOp::InsertPara(parse_insert_para(obj, i)?)),
            "deletePara" => out.push(DocxOp::DeletePara(parse_delete_para(obj, i)?)),
            "setTextBoxLink" => out.push(DocxOp::SetTextBoxLink(parse_set_txbx_link(obj, i)?)),
            _ => {
                return Err(Error::with_path(
                    ErrorKind::UnsupportedOp {
                        op: op_name.to_string(),
                    },
                    format!("ops[{i}].op"),
                ));
            }
        }
    }
    Ok(out)
}

fn parse_set_para_text(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetParaTextOp, Error> {
    for k in obj.keys() {
        if !matches!(k.as_str(), "op" | "id" | "text") {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("ops[{i}].{k}"),
            ));
        }
    }
    let id = obj
        .get("id")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            Error::with_path(
                ErrorKind::MissingField {
                    field: "id".into(),
                },
                format!("ops[{i}].id"),
            )
        })?;
    let text = match obj.get("text") {
        Some(Value::String(s)) => s.clone(),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "text".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].text"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::OpRequiresField {
                    op: "setParaText".into(),
                    field: "text".into(),
                },
                format!("ops[{i}].text"),
            ));
        }
    };
    Ok(SetParaTextOp {
        id: id.to_string(),
        text,
    })
}

fn parse_set_hf_distance(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetHfDistanceOp, Error> {
    for k in obj.keys() {
        if !matches!(k.as_str(), "op" | "id" | "headerDistance" | "footerDistance") {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("ops[{i}].{k}"),
            ));
        }
    }
    let id = obj
        .get("id")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            Error::with_path(
                ErrorKind::MissingField {
                    field: "id".into(),
                },
                format!("ops[{i}].id"),
            )
        })?;
    let n = parse_section_id(id, i)?;
    let header_distance = parse_twip_field(obj.get("headerDistance"), i, "headerDistance")?;
    let footer_distance = parse_twip_field(obj.get("footerDistance"), i, "footerDistance")?;
    if header_distance.is_none() && footer_distance.is_none() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setHeaderFooterDistance".into(),
                field: "headerDistance or footerDistance".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(SetHfDistanceOp {
        id: id.to_string(),
        n,
        header_distance,
        footer_distance,
    })
}

fn parse_set_hf_link(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetHfLinkOp, Error> {
    for k in obj.keys() {
        if !matches!(k.as_str(), "op" | "id" | "kind" | "type" | "linked") {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("ops[{i}].{k}"),
            ));
        }
    }
    let id = obj
        .get("id")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            Error::with_path(
                ErrorKind::MissingField {
                    field: "id".into(),
                },
                format!("ops[{i}].id"),
            )
        })?;
    let n = parse_section_id(id, i)?;
    let kind = match obj.get("kind") {
        Some(Value::String(s)) => HfKind::parse(s).ok_or_else(|| {
            Error::with_path(ErrorKind::InvalidHfKind, format!("ops[{i}].kind"))
        })?,
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "kind".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].kind"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::OpRequiresField {
                    op: "setHeaderFooterLink".into(),
                    field: "kind".into(),
                },
                format!("ops[{i}].kind"),
            ));
        }
    };
    let hf_type = match obj.get("type") {
        Some(Value::String(s)) => HfType::parse(s).ok_or_else(|| {
            Error::with_path(ErrorKind::InvalidHfType, format!("ops[{i}].type"))
        })?,
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "type".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].type"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::OpRequiresField {
                    op: "setHeaderFooterLink".into(),
                    field: "type".into(),
                },
                format!("ops[{i}].type"),
            ));
        }
    };
    let linked = match obj.get("linked") {
        Some(Value::Bool(b)) => *b,
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "linked".into(),
                    expected: TypeName::Boolean,
                },
                format!("ops[{i}].linked"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::OpRequiresField {
                    op: "setHeaderFooterLink".into(),
                    field: "linked".into(),
                },
                format!("ops[{i}].linked"),
            ));
        }
    };
    Ok(SetHfLinkOp {
        id: id.to_string(),
        n,
        kind,
        hf_type,
        linked,
    })
}

fn parse_section_id(id: &str, op_index: usize) -> Result<usize, Error> {
    let path = format!("ops[{op_index}].id");
    let rest = id.strip_prefix("section:").ok_or_else(|| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SectionPrefix,
            },
            path.clone(),
        )
    })?;
    let n: usize = rest.parse().map_err(|_| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SectionIndexNotInt,
            },
            path.clone(),
        )
    })?;
    if n < 1 {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SectionIndexFromOne,
            },
            path,
        ));
    }
    Ok(n)
}

fn parse_twip_field(
    v: Option<&Value>,
    i: usize,
    field: &str,
) -> Result<Option<u32>, Error> {
    let Some(v) = v else {
        return Ok(None);
    };
    let path = format!("ops[{i}].{field}");
    match v {
        Value::Number(n) => {
            if let Some(u) = n.as_u64() {
                if u > u32::MAX as u64 {
                    return Err(Error::with_path(
                        ErrorKind::MinValue {
                            field: field.into(),
                            min: 0,
                        },
                        path,
                    ));
                }
                return Ok(Some(u as u32));
            }
            if n.as_i64().is_some_and(|x| x < 0) {
                return Err(Error::with_path(
                    ErrorKind::MinValue {
                        field: field.into(),
                        min: 0,
                    },
                    path,
                ));
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

fn require_id(obj: &serde_json::Map<String, Value>, i: usize) -> Result<String, Error> {
    obj.get("id")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .ok_or_else(|| {
            Error::with_path(
                ErrorKind::MissingField {
                    field: "id".into(),
                },
                format!("ops[{i}].id"),
            )
        })
}

fn require_file(obj: &serde_json::Map<String, Value>, i: usize, op: &str) -> Result<String, Error> {
    match obj.get("file") {
        Some(Value::String(s)) if !s.is_empty() => Ok(s.clone()),
        Some(Value::String(_)) | None => Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: op.into(),
                field: "file".into(),
            },
            format!("ops[{i}].file"),
        )),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "file".into(),
                expected: TypeName::NonEmptyString,
            },
            format!("ops[{i}].file"),
        )),
    }
}

fn deny_keys(obj: &serde_json::Map<String, Value>, i: usize, allowed: &[&str]) -> Result<(), Error> {
    for k in obj.keys() {
        if !allowed.contains(&k.as_str()) {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("ops[{i}].{k}"),
            ));
        }
    }
    Ok(())
}

fn parse_replace_pic(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<ReplacePictureOp, Error> {
    deny_keys(obj, i, &["op", "id", "file"])?;
    let id = require_id(obj, i)?;
    let target = picture::parse_pic_id(&id, i)?;
    let file = require_file(obj, i, "replacePicture")?;
    Ok(ReplacePictureOp { id, target, file })
}

fn parse_insert_pic(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertPictureOp, Error> {
    deny_keys(obj, i, &["op", "id", "file", "cx", "cy"])?;
    let id = require_id(obj, i)?;
    let para = picture::parse_insert_para_id(&id, i)?;
    let file = require_file(obj, i, "insertPicture")?;
    let cx = parse_emu_field(obj.get("cx"), i, "cx")?;
    let cy = parse_emu_field(obj.get("cy"), i, "cy")?;
    if cx.is_some() != cy.is_some() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "insertPicture".into(),
                field: "cx and cy together".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(InsertPictureOp {
        id,
        para,
        file,
        cx,
        cy,
    })
}

fn parse_delete_pic(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeletePictureOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let target = picture::parse_pic_id(&id, i)?;
    Ok(DeletePictureOp { id, target })
}

fn parse_set_pic_size(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetPictureSizeOp, Error> {
    deny_keys(obj, i, &["op", "id", "cx", "cy"])?;
    let id = require_id(obj, i)?;
    let target = picture::parse_pic_id(&id, i)?;
    let cx = parse_emu_field(obj.get("cx"), i, "cx")?;
    let cy = parse_emu_field(obj.get("cy"), i, "cy")?;
    if cx.is_none() && cy.is_none() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setPictureSize".into(),
                field: "cx or cy".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(SetPictureSizeOp {
        id,
        target,
        cx,
        cy,
    })
}

fn parse_set_pic_wrap(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetPictureWrapOp, Error> {
    deny_keys(
        obj,
        i,
        &["op", "id", "kind", "wrap", "alignH", "alignV", "offsetX", "offsetY"],
    )?;
    let id = require_id(obj, i)?;
    let target = picture::parse_pic_id(&id, i)?;
    let kind = match obj.get("kind") {
        Some(Value::String(s)) => picture::parse_pic_kind(s).ok_or_else(|| {
            Error::with_path(ErrorKind::InvalidPicKind, format!("ops[{i}].kind"))
        })?,
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "kind".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].kind"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::OpRequiresField {
                    op: "setPictureWrap".into(),
                    field: "kind".into(),
                },
                format!("ops[{i}].kind"),
            ));
        }
    };
    let wrap = match obj.get("wrap") {
        None | Some(Value::Null) => None,
        Some(Value::String(s)) => Some(picture::parse_wrap(s).ok_or_else(|| {
            Error::with_path(ErrorKind::InvalidPicWrap, format!("ops[{i}].wrap"))
        })?),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "wrap".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].wrap"),
            ));
        }
    };
    if kind == PicKind::Inline && wrap.is_some() {
        return Err(Error::with_path(
            ErrorKind::InvalidPicWrap,
            format!("ops[{i}].wrap"),
        ));
    }
    if kind == PicKind::Anchor && wrap.is_none() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setPictureWrap".into(),
                field: "wrap".into(),
            },
            format!("ops[{i}].wrap"),
        ));
    }
    let align_h = parse_align_h_field(obj.get("alignH"), i)?;
    let align_v = parse_align_v_field(obj.get("alignV"), i)?;
    let offset_x = parse_i64_field(obj.get("offsetX"), i, "offsetX")?;
    let offset_y = parse_i64_field(obj.get("offsetY"), i, "offsetY")?;
    if align_h.is_some() && offset_x.is_some() {
        return Err(Error::with_path(
            ErrorKind::MutuallyExclusive {
                a: "alignH".into(),
                b: "offsetX".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    if align_v.is_some() && offset_y.is_some() {
        return Err(Error::with_path(
            ErrorKind::MutuallyExclusive {
                a: "alignV".into(),
                b: "offsetY".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    if kind == PicKind::Inline
        && (align_h.is_some() || align_v.is_some() || offset_x.is_some() || offset_y.is_some())
    {
        return Err(Error::with_path(
            ErrorKind::UnknownField {
                field: "alignH".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(SetPictureWrapOp {
        id,
        target,
        kind,
        wrap,
        pos: WrapPos {
            align_h,
            align_v,
            offset_x,
            offset_y,
        },
    })
}

fn parse_align_h_field(v: Option<&Value>, i: usize) -> Result<Option<AlignH>, Error> {
    match v {
        None => Ok(None),
        Some(Value::String(s)) => picture::parse_align_h(s)
            .map(Some)
            .ok_or_else(|| Error::with_path(ErrorKind::InvalidPicAlign, format!("ops[{i}].alignH"))),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "alignH".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].alignH"),
        )),
    }
}

fn parse_align_v_field(v: Option<&Value>, i: usize) -> Result<Option<AlignV>, Error> {
    match v {
        None => Ok(None),
        Some(Value::String(s)) => picture::parse_align_v(s)
            .map(Some)
            .ok_or_else(|| Error::with_path(ErrorKind::InvalidPicAlign, format!("ops[{i}].alignV"))),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "alignV".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].alignV"),
        )),
    }
}

fn parse_emu_field(v: Option<&Value>, i: usize, field: &str) -> Result<Option<i64>, Error> {
    let Some(v) = v else {
        return Ok(None);
    };
    let path = format!("ops[{i}].{field}");
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

fn parse_i64_field(v: Option<&Value>, i: usize, field: &str) -> Result<Option<i64>, Error> {
    let Some(v) = v else {
        return Ok(None);
    };
    let path = format!("ops[{i}].{field}");
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

fn parse_insert_txbx(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertTextBoxOp, Error> {
    deny_keys(obj, i, &["op", "id", "cx", "cy", "text"])?;
    let id = require_id(obj, i)?;
    let host = para::parse_host_para_id_at(&id, &format!("ops[{i}].id"))?;
    let cx = parse_emu_field(obj.get("cx"), i, "cx")?;
    let cy = parse_emu_field(obj.get("cy"), i, "cy")?;
    if cx.is_some() != cy.is_some() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "insertTextBox".into(),
                field: "cx and cy together".into(),
            },
            format!("ops[{i}]"),
        ));
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
                format!("ops[{i}].text"),
            ));
        }
    };
    Ok(InsertTextBoxOp {
        id,
        host,
        cx,
        cy,
        text,
    })
}

fn parse_delete_txbx(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteTextBoxOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let target = para::parse_txbx_id(&id, i)?;
    Ok(DeleteTextBoxOp { id, target })
}

fn parse_set_txbx_size(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetTextBoxSizeOp, Error> {
    deny_keys(obj, i, &["op", "id", "cx", "cy"])?;
    let id = require_id(obj, i)?;
    let target = para::parse_txbx_id(&id, i)?;
    let cx = parse_emu_field(obj.get("cx"), i, "cx")?;
    let cy = parse_emu_field(obj.get("cy"), i, "cy")?;
    if cx.is_none() && cy.is_none() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setTextBoxSize".into(),
                field: "cx or cy".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(SetTextBoxSizeOp {
        id,
        target,
        cx,
        cy,
    })
}

fn parse_set_txbx_wrap(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetTextBoxWrapOp, Error> {
    deny_keys(
        obj,
        i,
        &["op", "id", "kind", "wrap", "alignH", "alignV", "offsetX", "offsetY"],
    )?;
    let id = require_id(obj, i)?;
    let target = para::parse_txbx_id(&id, i)?;
    let kind = match obj.get("kind") {
        Some(Value::String(s)) => picture::parse_pic_kind(s).ok_or_else(|| {
            Error::with_path(ErrorKind::InvalidPicKind, format!("ops[{i}].kind"))
        })?,
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "kind".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].kind"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::OpRequiresField {
                    op: "setTextBoxWrap".into(),
                    field: "kind".into(),
                },
                format!("ops[{i}].kind"),
            ));
        }
    };
    let wrap = match obj.get("wrap") {
        None | Some(Value::Null) => None,
        Some(Value::String(s)) => Some(picture::parse_wrap(s).ok_or_else(|| {
            Error::with_path(ErrorKind::InvalidPicWrap, format!("ops[{i}].wrap"))
        })?),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "wrap".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].wrap"),
            ));
        }
    };
    if kind == PicKind::Inline && wrap.is_some() {
        return Err(Error::with_path(
            ErrorKind::InvalidPicWrap,
            format!("ops[{i}].wrap"),
        ));
    }
    if kind == PicKind::Anchor && wrap.is_none() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setTextBoxWrap".into(),
                field: "wrap".into(),
            },
            format!("ops[{i}].wrap"),
        ));
    }
    let align_h = parse_align_h_field(obj.get("alignH"), i)?;
    let align_v = parse_align_v_field(obj.get("alignV"), i)?;
    let offset_x = parse_i64_field(obj.get("offsetX"), i, "offsetX")?;
    let offset_y = parse_i64_field(obj.get("offsetY"), i, "offsetY")?;
    if align_h.is_some() && offset_x.is_some() {
        return Err(Error::with_path(
            ErrorKind::MutuallyExclusive {
                a: "alignH".into(),
                b: "offsetX".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    if align_v.is_some() && offset_y.is_some() {
        return Err(Error::with_path(
            ErrorKind::MutuallyExclusive {
                a: "alignV".into(),
                b: "offsetY".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(SetTextBoxWrapOp {
        id,
        target,
        kind,
        wrap,
        pos: WrapPos {
            align_h,
            align_v,
            offset_x,
            offset_y,
        },
    })
}

fn parse_color_field(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    required: bool,
) -> Result<Option<String>, Error> {
    match obj.get("color") {
        None if required => Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setTextBoxFill".into(),
                field: "color".into(),
            },
            format!("ops[{i}].color"),
        )),
        None => Ok(None),
        Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => {
            if s.len() == 6 && s.chars().all(|c| c.is_ascii_hexdigit()) {
                Ok(Some(s.clone()))
            } else {
                Err(Error::with_path(
                    ErrorKind::InvalidHexColor,
                    format!("ops[{i}].color"),
                ))
            }
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "color".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].color"),
        )),
    }
}

fn parse_set_txbx_fill(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetTextBoxFillOp, Error> {
    deny_keys(obj, i, &["op", "id", "color"])?;
    let id = require_id(obj, i)?;
    let target = para::parse_txbx_id(&id, i)?;
    let color = parse_color_field(obj, i, true)?;
    Ok(SetTextBoxFillOp { id, target, color })
}

fn parse_set_txbx_line(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetTextBoxLineOp, Error> {
    deny_keys(obj, i, &["op", "id", "color", "width"])?;
    let id = require_id(obj, i)?;
    let target = para::parse_txbx_id(&id, i)?;
    if !obj.contains_key("color") {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setTextBoxLine".into(),
                field: "color".into(),
            },
            format!("ops[{i}].color"),
        ));
    }
    let color = parse_color_field(obj, i, false)?;
    let width = parse_emu_field(obj.get("width"), i, "width")?;
    if color.is_none() && width.is_some() {
        return Err(Error::with_path(
            ErrorKind::UnknownField {
                field: "width".into(),
            },
            format!("ops[{i}].width"),
        ));
    }
    Ok(SetTextBoxLineOp {
        id,
        target,
        color,
        width,
    })
}

fn parse_set_txbx_dir(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetTextBoxDirectionOp, Error> {
    deny_keys(obj, i, &["op", "id", "direction"])?;
    let id = require_id(obj, i)?;
    let target = para::parse_txbx_id(&id, i)?;
    let vertical = match obj.get("direction") {
        Some(Value::String(s)) if s == "horizontal" => false,
        Some(Value::String(s)) if s == "vertical" => true,
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidTextBoxDirection,
                format!("ops[{i}].direction"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "direction".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].direction"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::OpRequiresField {
                    op: "setTextBoxDirection".into(),
                    field: "direction".into(),
                },
                format!("ops[{i}].direction"),
            ));
        }
    };
    Ok(SetTextBoxDirectionOp {
        id,
        target,
        vertical,
    })
}

fn parse_insert_para(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertParaOp, Error> {
    deny_keys(obj, i, &["op", "id", "text"])?;
    let id = require_id(obj, i)?;
    let path = format!("ops[{i}].id");
    if !id.contains("/txbx:") {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: crate::error::IdReason::InsertParaBody,
            },
            path,
        ));
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
                format!("ops[{i}].text"),
            ));
        }
    };
    if let Ok(box_target) = para::parse_txbx_id_at(&id, &path) {
        return Ok(InsertParaOp {
            id,
            after: None,
            box_target,
            text,
        });
    }
    match para::parse_para_id_at(&id, &path)? {
        ParaTarget::InTextBox { host, key, para } => Ok(InsertParaOp {
            id,
            after: Some(ParaTarget::InTextBox {
                host: host.clone(),
                key: key.clone(),
                para,
            }),
            box_target: TxbxTarget { host, key },
            text,
        }),
        _ => Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: crate::error::IdReason::InsertParaNeedsBox,
            },
            path,
        )),
    }
}

fn parse_delete_para(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteParaOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let path = format!("ops[{i}].id");
    if !id.contains("/txbx:") {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: crate::error::IdReason::InsertParaBody,
            },
            path,
        ));
    }
    match para::parse_para_id_at(&id, &path) {
        Ok(inner @ ParaTarget::InTextBox { .. }) => Ok(DeleteParaOp { id, inner }),
        Ok(_) | Err(_) => Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: crate::error::IdReason::InsertParaNeedsBox,
            },
            path,
        )),
    }
}

fn parse_set_txbx_link(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetTextBoxLinkOp, Error> {
    deny_keys(obj, i, &["op", "id", "next"])?;
    let id = require_id(obj, i)?;
    let target = para::parse_txbx_id(&id, i)?;
    let (next, next_id) = match obj.get("next") {
        None => {
            return Err(Error::with_path(
                ErrorKind::OpRequiresField {
                    op: "setTextBoxLink".into(),
                    field: "next".into(),
                },
                format!("ops[{i}].next"),
            ));
        }
        Some(Value::Null) => (None, None),
        Some(Value::String(s)) if !s.is_empty() => {
            let n = para::parse_txbx_id_at(s, &format!("ops[{i}].next"))?;
            (Some(n), Some(s.clone()))
        }
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "next".into(),
                },
                format!("ops[{i}].next"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "next".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].next"),
            ));
        }
    };
    Ok(SetTextBoxLinkOp {
        id,
        target,
        next,
        next_id,
    })
}
