//! pptx 自己的 scope 与 ops 解析。

use serde_json::{Map, Value};

use crate::chart::{ChartType, NewSeries};
use crate::error::{Error, ErrorKind, ScopeNeed, TypeName};
use crate::json;
use crate::smartart::{ColorsKind, LayoutKind, NewPoint, Role, StyleKind};

use super::chart;
use super::comments;
use super::picture;
use super::smartart;

#[derive(Debug, Clone)]
pub struct InspectScope {
    pub slide: Option<u32>,
    pub max_shapes: u32,
    pub pictures: bool,
    pub charts: bool,
    pub smart_art: bool,
    pub notes: bool,
    pub comments: bool,
}

#[derive(Debug, Clone)]
pub struct SetShapeTextOp {
    pub id: String,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct SetNotesTextOp {
    pub slide: usize,
    pub id: String,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct InsertCommentOp {
    pub slide: usize,
    pub text: String,
    pub author: String,
    pub initials: String,
    pub date: Option<String>,
    pub x: Option<i64>,
    pub y: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct ReplyCommentOp {
    pub id: String,
    pub text: String,
    pub author: String,
    pub initials: String,
    pub date: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DeleteCommentOp {
    pub id: String,
}

#[derive(Debug, Clone)]
pub struct SetCommentTextOp {
    pub id: String,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct SetCommentMetaOp {
    pub id: String,
    pub author: Option<String>,
    pub initials: Option<String>,
    pub date: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SetCommentDoneOp {
    pub id: String,
    pub done: bool,
}

#[derive(Debug, Clone)]
pub struct ReplacePictureOp {
    pub id: String,
    pub target: picture::PicTarget,
    pub file: String,
}

#[derive(Debug, Clone)]
pub struct InsertPictureOp {
    pub slide: usize,
    pub on_notes: bool,
    pub file: String,
    pub x: Option<i64>,
    pub y: Option<i64>,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct DeletePictureOp {
    pub id: String,
    pub target: picture::PicTarget,
}

#[derive(Debug, Clone)]
pub struct SetPictureSizeOp {
    pub id: String,
    pub target: picture::PicTarget,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct SetPicturePosOp {
    pub id: String,
    pub target: picture::PicTarget,
    pub x: Option<i64>,
    pub y: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct InsertChartOp {
    pub slide: usize,
    pub chart_type: ChartType,
    pub title: Option<String>,
    pub series: Vec<NewSeries>,
    pub x: Option<i64>,
    pub y: Option<i64>,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct DeleteChartOp {
    pub id: String,
    pub target: chart::ChartTarget,
}

#[derive(Debug, Clone)]
pub struct SetChartTitleOp {
    pub id: String,
    pub target: chart::ChartTarget,
    pub title: String,
}

#[derive(Debug, Clone)]
pub struct SetChartSeriesOp {
    pub id: String,
    pub target: chart::ChartTarget,
    pub index: usize,
    pub name: Option<String>,
    pub categories: Option<Vec<Value>>,
    pub values: Option<Vec<Value>>,
    pub sizes: Option<Vec<Value>>,
}

#[derive(Debug, Clone)]
pub struct AddChartSeriesOp {
    pub id: String,
    pub target: chart::ChartTarget,
    pub name: Option<String>,
    pub categories: Option<Vec<Value>>,
    pub values: Vec<Value>,
    pub sizes: Option<Vec<Value>>,
    pub series_type: Option<ChartType>,
}

#[derive(Debug, Clone)]
pub struct DeleteChartSeriesOp {
    pub id: String,
    pub target: chart::ChartTarget,
    pub index: usize,
}

#[derive(Debug, Clone)]
pub struct SetChartTypeOp {
    pub id: String,
    pub target: chart::ChartTarget,
    pub chart_type: ChartType,
}

#[derive(Debug, Clone)]
pub struct SetChartAxisTitleOp {
    pub id: String,
    pub target: chart::ChartTarget,
    pub axis: String,
    pub title: String,
}

#[derive(Debug, Clone)]
pub struct SetChartLegendOp {
    pub id: String,
    pub target: chart::ChartTarget,
    pub show: bool,
    pub pos: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SetChartDataLabelsOp {
    pub id: String,
    pub target: chart::ChartTarget,
    pub show: bool,
}

#[derive(Debug, Clone)]
pub struct SetChartSizeOp {
    pub id: String,
    pub target: chart::ChartTarget,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct SetChartPosOp {
    pub id: String,
    pub target: chart::ChartTarget,
    pub x: Option<i64>,
    pub y: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct InsertSmartArtOp {
    pub slide: usize,
    pub layout: LayoutKind,
    pub points: Option<Vec<NewPoint>>,
    pub x: Option<i64>,
    pub y: Option<i64>,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct DeleteSmartArtOp {
    pub id: String,
    pub target: smartart::SmartArtTarget,
}

#[derive(Debug, Clone)]
pub struct SetSmartArtTextOp {
    pub id: String,
    pub target: smartart::PointTarget,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct AddSmartArtPointOp {
    pub id: String,
    pub parent: smartart::AddParent,
    pub text: String,
    pub role: Role,
}

#[derive(Debug, Clone)]
pub struct DeleteSmartArtPointOp {
    pub id: String,
    pub target: smartart::PointTarget,
}

#[derive(Debug, Clone)]
pub struct SetSmartArtSizeOp {
    pub id: String,
    pub target: smartart::SmartArtTarget,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct SetSmartArtPosOp {
    pub id: String,
    pub target: smartart::SmartArtTarget,
    pub x: Option<i64>,
    pub y: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct SetSmartArtColorsOp {
    pub id: String,
    pub target: smartart::SmartArtTarget,
    pub colors: ColorsKind,
}

#[derive(Debug, Clone)]
pub struct SetSmartArtStyleOp {
    pub id: String,
    pub target: smartart::SmartArtTarget,
    pub style: StyleKind,
}

#[derive(Debug, Clone)]
pub struct SetSmartArtLayoutOp {
    pub id: String,
    pub target: smartart::SmartArtTarget,
    pub layout: LayoutKind,
}

#[derive(Debug, Clone)]
pub enum PptxOp {
    SetShapeText(SetShapeTextOp),
    SetNotesText(SetNotesTextOp),
    InsertComment(InsertCommentOp),
    ReplyComment(ReplyCommentOp),
    DeleteComment(DeleteCommentOp),
    SetCommentText(SetCommentTextOp),
    SetCommentMeta(SetCommentMetaOp),
    SetCommentDone(SetCommentDoneOp),
    ReplacePicture(ReplacePictureOp),
    InsertPicture(InsertPictureOp),
    DeletePicture(DeletePictureOp),
    SetPictureSize(SetPictureSizeOp),
    SetPicturePos(SetPicturePosOp),
    InsertChart(InsertChartOp),
    DeleteChart(DeleteChartOp),
    SetChartTitle(SetChartTitleOp),
    SetChartSeries(SetChartSeriesOp),
    AddChartSeries(AddChartSeriesOp),
    DeleteChartSeries(DeleteChartSeriesOp),
    SetChartType(SetChartTypeOp),
    SetChartAxisTitle(SetChartAxisTitleOp),
    SetChartLegend(SetChartLegendOp),
    SetChartDataLabels(SetChartDataLabelsOp),
    SetChartSize(SetChartSizeOp),
    SetChartPos(SetChartPosOp),
    InsertSmartArt(InsertSmartArtOp),
    DeleteSmartArt(DeleteSmartArtOp),
    SetSmartArtText(SetSmartArtTextOp),
    AddSmartArtPoint(AddSmartArtPointOp),
    DeleteSmartArtPoint(DeleteSmartArtPointOp),
    SetSmartArtSize(SetSmartArtSizeOp),
    SetSmartArtPos(SetSmartArtPosOp),
    SetSmartArtColors(SetSmartArtColorsOp),
    SetSmartArtStyle(SetSmartArtStyleOp),
    SetSmartArtLayout(SetSmartArtLayoutOp),
}

pub fn parse_scope(v: Option<&Value>) -> Result<Option<InspectScope>, Error> {
    let Some(v) = v else {
        return Ok(None);
    };
    let obj = v.as_object().ok_or_else(|| {
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
            "slide",
            "maxShapes",
            "pictures",
            "charts",
            "smartArt",
            "notes",
            "comments",
        ],
    )?;
    let pictures = parse_bool_field(obj.get("pictures"), "pictures")?;
    let charts = parse_bool_field(obj.get("charts"), "charts")?;
    let smart_art = parse_bool_field(obj.get("smartArt"), "smartArt")?;
    let notes = parse_bool_field(obj.get("notes"), "notes")?;
    let comments = parse_bool_field(obj.get("comments"), "comments")?;
    let slide = match obj.get("slide") {
        None => None,
        Some(s) => {
            let n = s.as_u64().ok_or_else(|| {
                Error::with_path(
                    ErrorKind::WrongType {
                        field: "slide".into(),
                        expected: TypeName::Integer,
                    },
                    "scope.slide",
                )
            })?;
            if n < 1 {
                return Err(Error::with_path(
                    ErrorKind::MinValue {
                        field: "slide".into(),
                        min: 1,
                    },
                    "scope.slide",
                ));
            }
            Some(n as u32)
        }
    };
    if obj.contains_key("maxShapes") && slide.is_none() {
        return Err(Error::with_path(
            ErrorKind::ScopeRequired {
                need: ScopeNeed::Slide,
            },
            "scope.slide",
        ));
    }
    let max_shapes = match obj.get("maxShapes") {
        None => 2000,
        Some(v) => {
            let n = v.as_u64().ok_or_else(|| {
                Error::with_path(
                    ErrorKind::WrongType {
                        field: "maxShapes".into(),
                        expected: TypeName::Integer,
                    },
                    "scope.maxShapes",
                )
            })?;
            if !(1..=5000).contains(&n) {
                return Err(Error::with_path(
                    ErrorKind::OutOfRange {
                        field: "maxShapes".into(),
                        min: 1,
                        max: 5000,
                    },
                    "scope.maxShapes",
                ));
            }
            n
        }
    };
    Ok(Some(InspectScope {
        slide,
        max_shapes: max_shapes as u32,
        pictures,
        charts,
        smart_art,
        notes,
        comments,
    }))
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

pub fn parse_ops(ops: &[Value]) -> Result<Vec<PptxOp>, Error> {
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
                ErrorKind::MissingField { field: "op".into() },
                format!("ops[{i}].op"),
            )
        })?;
        if is_foreign_op(op_name) {
            return Err(Error::with_path(
                ErrorKind::OpNotForFormat {
                    format: "pptx".into(),
                    op: op_name.to_string(),
                },
                format!("ops[{i}].op"),
            ));
        }
        let parsed = match op_name {
            "setShapeText" => PptxOp::SetShapeText(parse_set_shape_text(obj, i)?),
            "setNotesText" => PptxOp::SetNotesText(parse_set_notes_text(obj, i)?),
            "insertComment" => PptxOp::InsertComment(parse_insert_comment(obj, i)?),
            "replyComment" => PptxOp::ReplyComment(parse_reply_comment(obj, i)?),
            "deleteComment" => PptxOp::DeleteComment(parse_delete_comment(obj, i)?),
            "setCommentText" => PptxOp::SetCommentText(parse_set_comment_text(obj, i)?),
            "setCommentMeta" => PptxOp::SetCommentMeta(parse_set_comment_meta(obj, i)?),
            "setCommentDone" => PptxOp::SetCommentDone(parse_set_comment_done(obj, i)?),
            "replacePicture" => PptxOp::ReplacePicture(parse_replace_pic(obj, i)?),
            "insertPicture" => PptxOp::InsertPicture(parse_insert_pic(obj, i)?),
            "deletePicture" => PptxOp::DeletePicture(parse_delete_pic(obj, i)?),
            "setPictureSize" => PptxOp::SetPictureSize(parse_set_pic_size(obj, i)?),
            "setPicturePos" => PptxOp::SetPicturePos(parse_set_pic_pos(obj, i)?),
            "insertChart" => PptxOp::InsertChart(parse_insert_chart(obj, i)?),
            "deleteChart" => PptxOp::DeleteChart(parse_delete_chart(obj, i)?),
            "setChartTitle" => PptxOp::SetChartTitle(parse_set_chart_title(obj, i)?),
            "setChartSeries" => PptxOp::SetChartSeries(parse_set_chart_series(obj, i)?),
            "addChartSeries" => PptxOp::AddChartSeries(parse_add_chart_series(obj, i)?),
            "deleteChartSeries" => PptxOp::DeleteChartSeries(parse_delete_chart_series(obj, i)?),
            "setChartType" => PptxOp::SetChartType(parse_set_chart_type(obj, i)?),
            "setChartAxisTitle" => PptxOp::SetChartAxisTitle(parse_set_chart_axis_title(obj, i)?),
            "setChartLegend" => PptxOp::SetChartLegend(parse_set_chart_legend(obj, i)?),
            "setChartDataLabels" => {
                PptxOp::SetChartDataLabels(parse_set_chart_data_labels(obj, i)?)
            }
            "setChartSize" => PptxOp::SetChartSize(parse_set_chart_size(obj, i)?),
            "setChartPos" => PptxOp::SetChartPos(parse_set_chart_pos(obj, i)?),
            "insertSmartArt" => PptxOp::InsertSmartArt(parse_insert_smartart(obj, i)?),
            "deleteSmartArt" => PptxOp::DeleteSmartArt(parse_delete_smartart(obj, i)?),
            "setSmartArtText" => PptxOp::SetSmartArtText(parse_set_smartart_text(obj, i)?),
            "addSmartArtPoint" => PptxOp::AddSmartArtPoint(parse_add_smartart_point(obj, i)?),
            "deleteSmartArtPoint" => {
                PptxOp::DeleteSmartArtPoint(parse_delete_smartart_point(obj, i)?)
            }
            "setSmartArtSize" => PptxOp::SetSmartArtSize(parse_set_smartart_size(obj, i)?),
            "setSmartArtPos" => PptxOp::SetSmartArtPos(parse_set_smartart_pos(obj, i)?),
            "setSmartArtColors" => PptxOp::SetSmartArtColors(parse_set_smartart_colors(obj, i)?),
            "setSmartArtStyle" => PptxOp::SetSmartArtStyle(parse_set_smartart_style(obj, i)?),
            "setSmartArtLayout" => PptxOp::SetSmartArtLayout(parse_set_smartart_layout(obj, i)?),
            _ => {
                return Err(Error::with_path(
                    ErrorKind::UnsupportedOp {
                        op: op_name.to_string(),
                    },
                    format!("ops[{i}].op"),
                ));
            }
        };
        out.push(parsed);
    }
    Ok(out)
}

fn is_foreign_op(op_name: &str) -> bool {
    matches!(
        op_name,
        "setCell"
            | "setParaText"
            | "setHeaderFooterDistance"
            | "setPageMargin"
            | "setPageSize"
            | "setColumns"
            | "setVerticalAlign"
            | "setLineNumbering"
            | "setDocGrid"
            | "setRtlGutter"
            | "setMirrorMargins"
            | "setGutterAtTop"
            | "setHeaderFooterLink"
            | "insertHeaderFooter"
            | "setTitlePage"
            | "setEvenAndOddHeaders"
            | "setHeaderFooterText"
            | "setPictureWrap"
            | "setPictureAnchor"
            | "insertTextBox"
            | "deleteTextBox"
            | "setTextBoxSize"
            | "setTextBoxWrap"
            | "setTextBoxFill"
            | "setTextBoxLine"
            | "setTextBoxDirection"
            | "insertPara"
            | "deletePara"
            | "insertTable"
            | "deleteTable"
            | "insertRow"
            | "deleteRow"
            | "insertCol"
            | "deleteCol"
            | "mergeCells"
            | "splitCells"
            | "acceptRevision"
            | "rejectRevision"
            | "insertHyperlink"
            | "setHyperlink"
            | "deleteHyperlink"
            | "insertBookmark"
            | "setBookmark"
            | "deleteBookmark"
            | "insertField"
            | "setField"
            | "deleteField"
            | "insertNumbering"
            | "setNumbering"
            | "setParaNumbering"
            | "setStyle"
            | "setParaStyle"
            | "setTextBoxLink"
            | "insertFootnote"
            | "insertEndnote"
            | "deleteFootnote"
            | "deleteEndnote"
            | "setNoteNumbering"
            | "setChartWrap"
            | "setChartAnchor"
            | "insertChartSheet"
            | "deleteChartSheet"
            | "setSmartArtWrap"
    )
}

fn parse_set_shape_text(obj: &Map<String, Value>, i: usize) -> Result<SetShapeTextOp, Error> {
    deny_keys(obj, i, &["op", "id", "text"])?;
    let id = require_id(obj, i)?;
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
                    op: "setShapeText".into(),
                    field: "text".into(),
                },
                format!("ops[{i}].text"),
            ));
        }
    };
    Ok(SetShapeTextOp { id, text })
}

fn parse_set_notes_text(obj: &Map<String, Value>, i: usize) -> Result<SetNotesTextOp, Error> {
    deny_keys(obj, i, &["op", "id", "text"])?;
    let id = require_id(obj, i)?;
    let (slide, _) = picture::parse_insert_slide_or_notes(
        &id,
        i,
        crate::error::IdReason::SetNotesTextNeedsSlide,
    )?;
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
                    op: "setNotesText".into(),
                    field: "text".into(),
                },
                format!("ops[{i}].text"),
            ));
        }
    };
    Ok(SetNotesTextOp { slide, id, text })
}

fn parse_optional_text(obj: &Map<String, Value>, i: usize) -> Result<String, Error> {
    match obj.get("text") {
        None => Ok(String::new()),
        Some(Value::String(s)) => Ok(s.clone()),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "text".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].text"),
        )),
    }
}

fn parse_optional_str(
    obj: &Map<String, Value>,
    i: usize,
    field: &str,
    default: &str,
) -> Result<String, Error> {
    match obj.get(field) {
        None => Ok(default.to_string()),
        Some(Value::String(s)) => Ok(s.clone()),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].{field}"),
        )),
    }
}

fn parse_optional_date(obj: &Map<String, Value>, i: usize) -> Result<Option<String>, Error> {
    match obj.get("date") {
        None => Ok(None),
        Some(Value::String(s)) if !s.is_empty() => Ok(Some(s.clone())),
        Some(Value::String(_)) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "date".into(),
                expected: TypeName::NonEmptyString,
            },
            format!("ops[{i}].date"),
        )),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "date".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].date"),
        )),
    }
}

fn parse_insert_comment(obj: &Map<String, Value>, i: usize) -> Result<InsertCommentOp, Error> {
    deny_keys(
        obj,
        i,
        &["op", "id", "text", "author", "initials", "date", "x", "y"],
    )?;
    let id = require_id(obj, i)?;
    let slide = comments::parse_insert_slide(&id, i)?;
    let text = parse_optional_text(obj, i)?;
    let author = parse_optional_str(obj, i, "author", "ooxp")?;
    let initials = parse_optional_str(obj, i, "initials", "oo")?;
    let date = parse_optional_date(obj, i)?;
    let x = parse_off_field(obj.get("x"), i, "x")?;
    let y = parse_off_field(obj.get("y"), i, "y")?;
    if x.is_some() != y.is_some() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "insertComment".into(),
                field: "x and y together".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(InsertCommentOp {
        slide,
        text,
        author,
        initials,
        date,
        x,
        y,
    })
}

fn parse_reply_comment(obj: &Map<String, Value>, i: usize) -> Result<ReplyCommentOp, Error> {
    deny_keys(obj, i, &["op", "id", "text", "author", "initials", "date"])?;
    let id = require_id(obj, i)?;
    let _ = comments::parse_comment_id(&id, &format!("ops[{i}].id"))?;
    Ok(ReplyCommentOp {
        id,
        text: parse_optional_text(obj, i)?,
        author: parse_optional_str(obj, i, "author", "ooxp")?,
        initials: parse_optional_str(obj, i, "initials", "oo")?,
        date: parse_optional_date(obj, i)?,
    })
}

fn parse_delete_comment(obj: &Map<String, Value>, i: usize) -> Result<DeleteCommentOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let _ = comments::parse_comment_id(&id, &format!("ops[{i}].id"))?;
    Ok(DeleteCommentOp { id })
}

fn parse_set_comment_text(obj: &Map<String, Value>, i: usize) -> Result<SetCommentTextOp, Error> {
    deny_keys(obj, i, &["op", "id", "text"])?;
    let id = require_id(obj, i)?;
    let _ = comments::parse_comment_id(&id, &format!("ops[{i}].id"))?;
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
                    op: "setCommentText".into(),
                    field: "text".into(),
                },
                format!("ops[{i}].text"),
            ));
        }
    };
    Ok(SetCommentTextOp { id, text })
}

fn parse_set_comment_meta(obj: &Map<String, Value>, i: usize) -> Result<SetCommentMetaOp, Error> {
    deny_keys(obj, i, &["op", "id", "author", "initials", "date"])?;
    let id = require_id(obj, i)?;
    let _ = comments::parse_comment_id(&id, &format!("ops[{i}].id"))?;
    let author = match obj.get("author") {
        None => None,
        Some(Value::String(s)) => Some(s.clone()),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "author".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].author"),
            ));
        }
    };
    let initials = match obj.get("initials") {
        None => None,
        Some(Value::String(s)) => Some(s.clone()),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "initials".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].initials"),
            ));
        }
    };
    let date = parse_optional_date(obj, i)?;
    if author.is_none() && initials.is_none() && date.is_none() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setCommentMeta".into(),
                field: "author, initials, or date".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(SetCommentMetaOp {
        id,
        author,
        initials,
        date,
    })
}

fn parse_set_comment_done(obj: &Map<String, Value>, i: usize) -> Result<SetCommentDoneOp, Error> {
    deny_keys(obj, i, &["op", "id", "done"])?;
    let id = require_id(obj, i)?;
    let _ = comments::parse_comment_id(&id, &format!("ops[{i}].id"))?;
    let done = match obj.get("done") {
        Some(Value::Bool(b)) => *b,
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "done".into(),
                    expected: TypeName::Boolean,
                },
                format!("ops[{i}].done"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::OpRequiresField {
                    op: "setCommentDone".into(),
                    field: "done".into(),
                },
                format!("ops[{i}].done"),
            ));
        }
    };
    Ok(SetCommentDoneOp { id, done })
}

fn parse_replace_pic(obj: &Map<String, Value>, i: usize) -> Result<ReplacePictureOp, Error> {
    deny_keys(obj, i, &["op", "id", "file"])?;
    let id = require_id(obj, i)?;
    let target = picture::parse_pic_id(&id, &format!("ops[{i}].id"))?;
    let file = require_file(obj, i, "replacePicture")?;
    Ok(ReplacePictureOp { id, target, file })
}

fn parse_insert_pic(obj: &Map<String, Value>, i: usize) -> Result<InsertPictureOp, Error> {
    deny_keys(obj, i, &["op", "id", "file", "x", "y", "cx", "cy"])?;
    let id = require_id(obj, i)?;
    let (slide, on_notes) =
        picture::parse_insert_slide_or_notes(&id, i, crate::error::IdReason::InsertNeedsSlide)?;
    let file = require_file(obj, i, "insertPicture")?;
    let x = parse_off_field(obj.get("x"), i, "x")?;
    let y = parse_off_field(obj.get("y"), i, "y")?;
    if x.is_some() != y.is_some() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "insertPicture".into(),
                field: "x and y together".into(),
            },
            format!("ops[{i}]"),
        ));
    }
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
        slide,
        on_notes,
        file,
        x,
        y,
        cx,
        cy,
    })
}

fn parse_delete_pic(obj: &Map<String, Value>, i: usize) -> Result<DeletePictureOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let target = picture::parse_pic_id(&id, &format!("ops[{i}].id"))?;
    Ok(DeletePictureOp { id, target })
}

fn parse_set_pic_size(obj: &Map<String, Value>, i: usize) -> Result<SetPictureSizeOp, Error> {
    deny_keys(obj, i, &["op", "id", "cx", "cy"])?;
    let id = require_id(obj, i)?;
    let target = picture::parse_pic_id(&id, &format!("ops[{i}].id"))?;
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
    Ok(SetPictureSizeOp { id, target, cx, cy })
}

fn parse_set_pic_pos(obj: &Map<String, Value>, i: usize) -> Result<SetPicturePosOp, Error> {
    deny_keys(obj, i, &["op", "id", "x", "y"])?;
    let id = require_id(obj, i)?;
    let target = picture::parse_pic_id(&id, &format!("ops[{i}].id"))?;
    let x = parse_off_field(obj.get("x"), i, "x")?;
    let y = parse_off_field(obj.get("y"), i, "y")?;
    if x.is_none() && y.is_none() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setPicturePos".into(),
                field: "x or y".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(SetPicturePosOp { id, target, x, y })
}

fn parse_insert_chart(obj: &Map<String, Value>, i: usize) -> Result<InsertChartOp, Error> {
    deny_keys(
        obj,
        i,
        &["op", "id", "type", "title", "series", "x", "y", "cx", "cy"],
    )?;
    let id = require_id(obj, i)?;
    let slide = chart::parse_insert_slide_id(&id, i)?;
    let chart_type = parse_chart_type_field(obj.get("type"), &format!("ops[{i}].type"))?;
    let title = parse_chart_title_field(obj.get("title"), &format!("ops[{i}].title"))?;
    let series = parse_insert_series(obj.get("series"), &format!("ops[{i}]"), "insertChart")?;
    validate_insert_series(chart_type, &series, &format!("ops[{i}].series"))?;
    let x = parse_off_field(obj.get("x"), i, "x")?;
    let y = parse_off_field(obj.get("y"), i, "y")?;
    if x.is_some() != y.is_some() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "insertChart".into(),
                field: "x and y together".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    let cx = parse_emu_field(obj.get("cx"), i, "cx")?;
    let cy = parse_emu_field(obj.get("cy"), i, "cy")?;
    if cx.is_some() != cy.is_some() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "insertChart".into(),
                field: "cx and cy together".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(InsertChartOp {
        slide,
        chart_type,
        title,
        series,
        x,
        y,
        cx,
        cy,
    })
}

fn parse_delete_chart(obj: &Map<String, Value>, i: usize) -> Result<DeleteChartOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let target = chart::parse_chart_id(&id, &format!("ops[{i}].id"))?;
    Ok(DeleteChartOp { id, target })
}

fn parse_set_chart_title(obj: &Map<String, Value>, i: usize) -> Result<SetChartTitleOp, Error> {
    deny_keys(obj, i, &["op", "id", "title"])?;
    let id = require_id(obj, i)?;
    let target = chart::parse_chart_id(&id, &format!("ops[{i}].id"))?;
    let title = match obj.get("title") {
        Some(Value::String(s)) => s.clone(),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "title".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].title"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::OpRequiresField {
                    op: "setChartTitle".into(),
                    field: "title".into(),
                },
                format!("ops[{i}].title"),
            ));
        }
    };
    Ok(SetChartTitleOp { id, target, title })
}

fn parse_set_chart_series(obj: &Map<String, Value>, i: usize) -> Result<SetChartSeriesOp, Error> {
    deny_keys(
        obj,
        i,
        &["op", "id", "name", "categories", "values", "sizes"],
    )?;
    let id = require_id(obj, i)?;
    let (target, index) = chart::parse_series_id(&id, &format!("ops[{i}].id"))?;
    let name = match obj.get("name") {
        None | Some(Value::Null) => None,
        Some(Value::String(s)) => Some(s.clone()),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "name".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].name"),
            ));
        }
    };
    let categories = parse_datum_list(obj.get("categories"), &format!("ops[{i}].categories"))?;
    let values = parse_datum_list(obj.get("values"), &format!("ops[{i}].values"))?;
    let sizes = parse_datum_list(obj.get("sizes"), &format!("ops[{i}].sizes"))?;
    if name.is_none() && categories.is_none() && values.is_none() && sizes.is_none() {
        return Err(Error::with_path(
            ErrorKind::NeedChartSeriesField,
            format!("ops[{i}]"),
        ));
    }
    Ok(SetChartSeriesOp {
        id,
        target,
        index,
        name,
        categories,
        values,
        sizes,
    })
}

fn parse_add_chart_series(obj: &Map<String, Value>, i: usize) -> Result<AddChartSeriesOp, Error> {
    deny_keys(
        obj,
        i,
        &["op", "id", "name", "categories", "values", "sizes", "type"],
    )?;
    let id = require_id(obj, i)?;
    let target = chart::parse_chart_id(&id, &format!("ops[{i}].id"))?;
    let name = match obj.get("name") {
        None | Some(Value::Null) => None,
        Some(Value::String(s)) => Some(s.clone()),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "name".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].name"),
            ));
        }
    };
    let categories = parse_datum_list(obj.get("categories"), &format!("ops[{i}].categories"))?;
    let values = parse_datum_list(obj.get("values"), &format!("ops[{i}].values"))?
        .filter(|v| !v.is_empty())
        .ok_or_else(|| {
            Error::with_path(
                ErrorKind::OpRequiresField {
                    op: "addChartSeries".into(),
                    field: "values".into(),
                },
                format!("ops[{i}].values"),
            )
        })?;
    let sizes = parse_datum_list(obj.get("sizes"), &format!("ops[{i}].sizes"))?;
    let series_type = parse_optional_combo_series_type(obj.get("type"), &format!("ops[{i}].type"))?;
    Ok(AddChartSeriesOp {
        id,
        target,
        name,
        categories,
        values,
        sizes,
        series_type,
    })
}

fn parse_delete_chart_series(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<DeleteChartSeriesOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let (target, index) = chart::parse_series_id(&id, &format!("ops[{i}].id"))?;
    Ok(DeleteChartSeriesOp { id, target, index })
}

fn parse_set_chart_type(obj: &Map<String, Value>, i: usize) -> Result<SetChartTypeOp, Error> {
    deny_keys(obj, i, &["op", "id", "type"])?;
    let id = require_id(obj, i)?;
    let target = chart::parse_chart_id(&id, &format!("ops[{i}].id"))?;
    let chart_type = parse_chart_type_field(obj.get("type"), &format!("ops[{i}].type"))?;
    if chart_type == ChartType::Combo {
        return Err(Error::with_path(
            ErrorKind::InvalidChartType,
            format!("ops[{i}].type"),
        ));
    }
    Ok(SetChartTypeOp {
        id,
        target,
        chart_type,
    })
}

fn parse_set_chart_axis_title(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<SetChartAxisTitleOp, Error> {
    deny_keys(obj, i, &["op", "id", "axis", "title"])?;
    let id = require_id(obj, i)?;
    let target = chart::parse_chart_id(&id, &format!("ops[{i}].id"))?;
    let axis = match obj.get("axis") {
        Some(Value::String(s)) if s == "cat" || s == "val" => s.clone(),
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidChartAxis,
                format!("ops[{i}].axis"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "axis".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].axis"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::OpRequiresField {
                    op: "setChartAxisTitle".into(),
                    field: "axis".into(),
                },
                format!("ops[{i}].axis"),
            ));
        }
    };
    let title = match obj.get("title") {
        Some(Value::String(s)) => s.clone(),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "title".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].title"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::OpRequiresField {
                    op: "setChartAxisTitle".into(),
                    field: "title".into(),
                },
                format!("ops[{i}].title"),
            ));
        }
    };
    Ok(SetChartAxisTitleOp {
        id,
        target,
        axis,
        title,
    })
}

fn parse_set_chart_legend(obj: &Map<String, Value>, i: usize) -> Result<SetChartLegendOp, Error> {
    deny_keys(obj, i, &["op", "id", "show", "pos"])?;
    let id = require_id(obj, i)?;
    let target = chart::parse_chart_id(&id, &format!("ops[{i}].id"))?;
    let show = parse_required_bool(obj.get("show"), i, "setChartLegend", "show")?;
    let pos = match obj.get("pos") {
        None | Some(Value::Null) => None,
        Some(Value::String(s)) if matches!(s.as_str(), "b" | "t" | "l" | "r" | "tr") => {
            Some(s.clone())
        }
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidLegendPos,
                format!("ops[{i}].pos"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "pos".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].pos"),
            ));
        }
    };
    Ok(SetChartLegendOp {
        id,
        target,
        show,
        pos,
    })
}

fn parse_set_chart_data_labels(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<SetChartDataLabelsOp, Error> {
    deny_keys(obj, i, &["op", "id", "show"])?;
    let id = require_id(obj, i)?;
    let target = chart::parse_chart_id(&id, &format!("ops[{i}].id"))?;
    let show = parse_required_bool(obj.get("show"), i, "setChartDataLabels", "show")?;
    Ok(SetChartDataLabelsOp { id, target, show })
}

fn parse_set_chart_size(obj: &Map<String, Value>, i: usize) -> Result<SetChartSizeOp, Error> {
    deny_keys(obj, i, &["op", "id", "cx", "cy"])?;
    let id = require_id(obj, i)?;
    let target = chart::parse_chart_id(&id, &format!("ops[{i}].id"))?;
    let cx = parse_emu_field(obj.get("cx"), i, "cx")?;
    let cy = parse_emu_field(obj.get("cy"), i, "cy")?;
    if cx.is_none() && cy.is_none() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setChartSize".into(),
                field: "cx or cy".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(SetChartSizeOp { id, target, cx, cy })
}

fn parse_set_chart_pos(obj: &Map<String, Value>, i: usize) -> Result<SetChartPosOp, Error> {
    deny_keys(obj, i, &["op", "id", "x", "y"])?;
    let id = require_id(obj, i)?;
    let target = chart::parse_chart_id(&id, &format!("ops[{i}].id"))?;
    let x = parse_off_field(obj.get("x"), i, "x")?;
    let y = parse_off_field(obj.get("y"), i, "y")?;
    if x.is_none() && y.is_none() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setChartPos".into(),
                field: "x or y".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(SetChartPosOp { id, target, x, y })
}

fn parse_insert_smartart(obj: &Map<String, Value>, i: usize) -> Result<InsertSmartArtOp, Error> {
    deny_keys(
        obj,
        i,
        &["op", "id", "layout", "points", "x", "y", "cx", "cy"],
    )?;
    let id = require_id(obj, i)?;
    let slide = smartart::parse_insert_slide_id(&id, i)?;
    let (layout, points) = crate::smartart::parse_layout_and_points(obj, &format!("ops[{i}]"))?;
    let x = parse_off_field(obj.get("x"), i, "x")?;
    let y = parse_off_field(obj.get("y"), i, "y")?;
    if x.is_some() != y.is_some() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "insertSmartArt".into(),
                field: "x and y together".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    let cx = parse_emu_field(obj.get("cx"), i, "cx")?;
    let cy = parse_emu_field(obj.get("cy"), i, "cy")?;
    if cx.is_some() != cy.is_some() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "insertSmartArt".into(),
                field: "cx and cy together".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(InsertSmartArtOp {
        slide,
        layout,
        points,
        x,
        y,
        cx,
        cy,
    })
}

fn parse_delete_smartart(obj: &Map<String, Value>, i: usize) -> Result<DeleteSmartArtOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let target = smartart::parse_diagram_id(&id, &format!("ops[{i}].id"))?;
    Ok(DeleteSmartArtOp { id, target })
}

fn parse_set_smartart_text(obj: &Map<String, Value>, i: usize) -> Result<SetSmartArtTextOp, Error> {
    deny_keys(obj, i, &["op", "id", "text"])?;
    let id = require_id(obj, i)?;
    let target = smartart::parse_point_id(&id, &format!("ops[{i}].id"))?;
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
                    op: "setSmartArtText".into(),
                    field: "text".into(),
                },
                format!("ops[{i}].text"),
            ));
        }
    };
    Ok(SetSmartArtTextOp { id, target, text })
}

fn parse_add_smartart_point(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<AddSmartArtPointOp, Error> {
    deny_keys(obj, i, &["op", "id", "text", "role"])?;
    let id = require_id(obj, i)?;
    let parent = smartart::parse_add_parent(&id, &format!("ops[{i}].id"))?;
    let text = match obj.get("text") {
        None | Some(Value::Null) => String::new(),
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
    let role = crate::smartart::parse_role_field(obj.get("role"), &format!("ops[{i}].role"))?;
    Ok(AddSmartArtPointOp {
        id,
        parent,
        text,
        role,
    })
}

fn parse_delete_smartart_point(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<DeleteSmartArtPointOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let target = smartart::parse_point_id(&id, &format!("ops[{i}].id"))?;
    Ok(DeleteSmartArtPointOp { id, target })
}

fn parse_set_smartart_size(obj: &Map<String, Value>, i: usize) -> Result<SetSmartArtSizeOp, Error> {
    deny_keys(obj, i, &["op", "id", "cx", "cy"])?;
    let id = require_id(obj, i)?;
    let target = smartart::parse_diagram_id(&id, &format!("ops[{i}].id"))?;
    let cx = parse_emu_field(obj.get("cx"), i, "cx")?;
    let cy = parse_emu_field(obj.get("cy"), i, "cy")?;
    if cx.is_none() && cy.is_none() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setSmartArtSize".into(),
                field: "cx or cy".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(SetSmartArtSizeOp { id, target, cx, cy })
}

fn parse_set_smartart_pos(obj: &Map<String, Value>, i: usize) -> Result<SetSmartArtPosOp, Error> {
    deny_keys(obj, i, &["op", "id", "x", "y"])?;
    let id = require_id(obj, i)?;
    let target = smartart::parse_diagram_id(&id, &format!("ops[{i}].id"))?;
    let x = parse_off_field(obj.get("x"), i, "x")?;
    let y = parse_off_field(obj.get("y"), i, "y")?;
    if x.is_none() && y.is_none() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setSmartArtPos".into(),
                field: "x or y".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(SetSmartArtPosOp { id, target, x, y })
}

fn parse_set_smartart_colors(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<SetSmartArtColorsOp, Error> {
    deny_keys(obj, i, &["op", "id", "colors"])?;
    let id = require_id(obj, i)?;
    let target = smartart::parse_diagram_id(&id, &format!("ops[{i}].id"))?;
    let colors =
        crate::smartart::parse_colors_field(obj.get("colors"), &format!("ops[{i}].colors"))?;
    Ok(SetSmartArtColorsOp { id, target, colors })
}

fn parse_set_smartart_style(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<SetSmartArtStyleOp, Error> {
    deny_keys(obj, i, &["op", "id", "style"])?;
    let id = require_id(obj, i)?;
    let target = smartart::parse_diagram_id(&id, &format!("ops[{i}].id"))?;
    let style = crate::smartart::parse_style_field(obj.get("style"), &format!("ops[{i}].style"))?;
    Ok(SetSmartArtStyleOp { id, target, style })
}

fn parse_set_smartart_layout(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<SetSmartArtLayoutOp, Error> {
    deny_keys(obj, i, &["op", "id", "layout"])?;
    let id = require_id(obj, i)?;
    let target = smartart::parse_diagram_id(&id, &format!("ops[{i}].id"))?;
    let layout =
        crate::smartart::parse_layout_field(obj.get("layout"), &format!("ops[{i}].layout"))?;
    Ok(SetSmartArtLayoutOp { id, target, layout })
}

fn parse_chart_type_field(v: Option<&Value>, path: &str) -> Result<ChartType, Error> {
    match v {
        Some(Value::String(s)) => ChartType::parse(s)
            .ok_or_else(|| Error::with_path(ErrorKind::InvalidChartType, path.to_string())),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "type".into(),
                expected: TypeName::String,
            },
            path,
        )),
        None => Err(Error::with_path(
            ErrorKind::MissingField {
                field: "type".into(),
            },
            path,
        )),
    }
}

fn parse_chart_title_field(v: Option<&Value>, path: &str) -> Result<Option<String>, Error> {
    match v {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "title".into(),
                expected: TypeName::String,
            },
            path,
        )),
    }
}

fn validate_insert_series(
    chart_type: ChartType,
    series: &[NewSeries],
    path: &str,
) -> Result<(), Error> {
    if chart_type.needs_one_series() && series.len() != 1 {
        return Err(Error::with_path(ErrorKind::PieNeedsOneSeries, path));
    }
    if chart_type == ChartType::Combo {
        let kinds: std::collections::HashSet<&str> = series
            .iter()
            .filter_map(|s| s.series_type.map(ChartType::as_str))
            .collect();
        if series.iter().any(|s| s.series_type.is_none()) || kinds.len() < 2 {
            return Err(Error::with_path(ErrorKind::InvalidComboSeriesType, path));
        }
    } else if series.iter().any(|s| s.series_type.is_some()) {
        return Err(Error::with_path(ErrorKind::InvalidComboSeriesType, path));
    }
    if chart_type != ChartType::Bubble && series.iter().any(|s| s.sizes.is_some()) {
        return Err(Error::with_path(ErrorKind::SizesOnNonBubble, path));
    }
    Ok(())
}

fn parse_insert_series(v: Option<&Value>, base: &str, op: &str) -> Result<Vec<NewSeries>, Error> {
    let arr = match v {
        Some(Value::Array(a)) if !a.is_empty() => a,
        Some(Value::Array(_)) | None => {
            return Err(Error::with_path(
                ErrorKind::OpRequiresField {
                    op: op.into(),
                    field: "series".into(),
                },
                format!("{base}.series"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "series".into(),
                    expected: TypeName::Array,
                },
                format!("{base}.series"),
            ));
        }
    };
    let mut out = Vec::new();
    for (j, item) in arr.iter().enumerate() {
        let obj = item.as_object().ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: format!("series[{j}]"),
                    expected: TypeName::Object,
                },
                format!("{base}.series[{j}]"),
            )
        })?;
        deny_keys_at(
            obj,
            &format!("{base}.series[{j}]"),
            &["name", "categories", "values", "sizes", "type"],
        )?;
        let name = match obj.get("name") {
            None | Some(Value::Null) => None,
            Some(Value::String(s)) => Some(s.clone()),
            Some(_) => {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: format!("series[{j}].name"),
                        expected: TypeName::String,
                    },
                    format!("{base}.series[{j}].name"),
                ));
            }
        };
        let categories = parse_datum_list(
            obj.get("categories"),
            &format!("{base}.series[{j}].categories"),
        )?;
        let values = parse_datum_list(obj.get("values"), &format!("{base}.series[{j}].values"))?
            .filter(|v| !v.is_empty())
            .ok_or_else(|| {
                Error::with_path(
                    ErrorKind::OpRequiresField {
                        op: op.into(),
                        field: "series[].values".into(),
                    },
                    format!("{base}.series[{j}].values"),
                )
            })?;
        let sizes = parse_datum_list(obj.get("sizes"), &format!("{base}.series[{j}].sizes"))?;
        let series_type =
            parse_optional_combo_series_type(obj.get("type"), &format!("{base}.series[{j}].type"))?;
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

fn parse_optional_combo_series_type(
    v: Option<&Value>,
    path: &str,
) -> Result<Option<ChartType>, Error> {
    match v {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(ChartType::parse_combo_series(s).ok_or_else(|| {
            Error::with_path(ErrorKind::InvalidComboSeriesType, path.to_string())
        })?)),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "type".into(),
                expected: TypeName::String,
            },
            path,
        )),
    }
}

fn parse_datum_list(v: Option<&Value>, path: &str) -> Result<Option<Vec<Value>>, Error> {
    match v {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Array(a)) => {
            for (j, item) in a.iter().enumerate() {
                if !matches!(
                    item,
                    Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_)
                ) {
                    return Err(Error::with_path(
                        ErrorKind::InvalidCellValue,
                        format!("{path}[{j}]"),
                    ));
                }
            }
            Ok(Some(a.clone()))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "array".into(),
                expected: TypeName::Array,
            },
            path,
        )),
    }
}

fn parse_required_bool(v: Option<&Value>, i: usize, op: &str, field: &str) -> Result<bool, Error> {
    match v {
        Some(Value::Bool(b)) => Ok(*b),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Boolean,
            },
            format!("ops[{i}].{field}"),
        )),
        None => Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: op.into(),
                field: field.into(),
            },
            format!("ops[{i}].{field}"),
        )),
    }
}

fn deny_keys(obj: &Map<String, Value>, i: usize, allowed: &[&str]) -> Result<(), Error> {
    deny_keys_at(obj, &format!("ops[{i}]"), allowed)
}

fn deny_keys_at(obj: &Map<String, Value>, prefix: &str, allowed: &[&str]) -> Result<(), Error> {
    for k in obj.keys() {
        if !allowed.contains(&k.as_str()) {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("{prefix}.{k}"),
            ));
        }
    }
    Ok(())
}

fn require_id(obj: &Map<String, Value>, i: usize) -> Result<String, Error> {
    obj.get("id")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .ok_or_else(|| {
            Error::with_path(
                ErrorKind::MissingField { field: "id".into() },
                format!("ops[{i}].id"),
            )
        })
}

fn require_file(obj: &Map<String, Value>, i: usize, op: &str) -> Result<String, Error> {
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

pub(crate) fn parse_create_chart(
    v: &Value,
    prefix: &str,
    slide: usize,
) -> Result<InsertChartOp, Error> {
    let obj = v.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "charts".into(),
                expected: TypeName::Object,
            },
            prefix.to_string(),
        )
    })?;
    deny_keys_at(
        obj,
        prefix,
        &["type", "title", "series", "x", "y", "cx", "cy"],
    )?;
    let chart_type = parse_chart_type_field(obj.get("type"), &format!("{prefix}.type"))?;
    let title = parse_chart_title_field(obj.get("title"), &format!("{prefix}.title"))?;
    let series = parse_insert_series(obj.get("series"), prefix, "create")?;
    validate_insert_series(chart_type, &series, &format!("{prefix}.series"))?;
    let x = parse_off_at(obj.get("x"), &format!("{prefix}.x"), "x")?;
    let y = parse_off_at(obj.get("y"), &format!("{prefix}.y"), "y")?;
    if x.is_some() != y.is_some() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "create".into(),
                field: "x and y together".into(),
            },
            prefix.to_string(),
        ));
    }
    let cx = parse_emu_at(obj.get("cx"), &format!("{prefix}.cx"), "cx")?;
    let cy = parse_emu_at(obj.get("cy"), &format!("{prefix}.cy"), "cy")?;
    if cx.is_some() != cy.is_some() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "create".into(),
                field: "cx and cy together".into(),
            },
            prefix.to_string(),
        ));
    }
    Ok(InsertChartOp {
        slide,
        chart_type,
        title,
        series,
        x,
        y,
        cx,
        cy,
    })
}

pub(crate) fn parse_create_smartart(
    v: &Value,
    prefix: &str,
    slide: usize,
) -> Result<InsertSmartArtOp, Error> {
    let obj = v.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "smartArts".into(),
                expected: TypeName::Object,
            },
            prefix.to_string(),
        )
    })?;
    deny_keys_at(obj, prefix, &["layout", "points", "x", "y", "cx", "cy"])?;
    let (layout, points) = crate::smartart::parse_layout_and_points(obj, prefix)?;
    let x = parse_off_at(obj.get("x"), &format!("{prefix}.x"), "x")?;
    let y = parse_off_at(obj.get("y"), &format!("{prefix}.y"), "y")?;
    if x.is_some() != y.is_some() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "create".into(),
                field: "x and y together".into(),
            },
            prefix.to_string(),
        ));
    }
    let cx = parse_emu_at(obj.get("cx"), &format!("{prefix}.cx"), "cx")?;
    let cy = parse_emu_at(obj.get("cy"), &format!("{prefix}.cy"), "cy")?;
    if cx.is_some() != cy.is_some() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "create".into(),
                field: "cx and cy together".into(),
            },
            prefix.to_string(),
        ));
    }
    Ok(InsertSmartArtOp {
        slide,
        layout,
        points,
        x,
        y,
        cx,
        cy,
    })
}

pub(crate) fn parse_create_picture(
    v: &Value,
    prefix: &str,
    slide: usize,
) -> Result<InsertPictureOp, Error> {
    let obj = v.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "pictures".into(),
                expected: TypeName::Object,
            },
            prefix.to_string(),
        )
    })?;
    for k in obj.keys() {
        if !matches!(k.as_str(), "file" | "x" | "y" | "cx" | "cy") {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("{prefix}.{k}"),
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
                format!("{prefix}.file"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "file".into(),
                    expected: TypeName::NonEmptyString,
                },
                format!("{prefix}.file"),
            ));
        }
    };
    let x = parse_off_at(obj.get("x"), &format!("{prefix}.x"), "x")?;
    let y = parse_off_at(obj.get("y"), &format!("{prefix}.y"), "y")?;
    if x.is_some() != y.is_some() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "create".into(),
                field: "x and y together".into(),
            },
            prefix.to_string(),
        ));
    }
    let cx = parse_emu_at(obj.get("cx"), &format!("{prefix}.cx"), "cx")?;
    let cy = parse_emu_at(obj.get("cy"), &format!("{prefix}.cy"), "cy")?;
    if cx.is_some() != cy.is_some() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "create".into(),
                field: "cx and cy together".into(),
            },
            prefix.to_string(),
        ));
    }
    Ok(InsertPictureOp {
        slide,
        on_notes: false,
        file,
        x,
        y,
        cx,
        cy,
    })
}

fn parse_emu_field(v: Option<&Value>, i: usize, field: &str) -> Result<Option<i64>, Error> {
    parse_emu_at(v, &format!("ops[{i}].{field}"), field)
}

fn parse_emu_at(v: Option<&Value>, path: &str, field: &str) -> Result<Option<i64>, Error> {
    let Some(v) = v else {
        return Ok(None);
    };
    match v {
        Value::Number(n) => {
            if let Some(u) = n.as_u64() {
                if u == 0 || u > i64::MAX as u64 {
                    return Err(Error::with_path(
                        ErrorKind::MinValue {
                            field: field.into(),
                            min: 1,
                        },
                        path.to_string(),
                    ));
                }
                return Ok(Some(u as i64));
            }
            Err(Error::with_path(
                ErrorKind::WrongType {
                    field: field.into(),
                    expected: TypeName::Integer,
                },
                path.to_string(),
            ))
        }
        _ => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Integer,
            },
            path.to_string(),
        )),
    }
}

fn parse_off_field(v: Option<&Value>, i: usize, field: &str) -> Result<Option<i64>, Error> {
    parse_off_at(v, &format!("ops[{i}].{field}"), field)
}

fn parse_off_at(v: Option<&Value>, path: &str, field: &str) -> Result<Option<i64>, Error> {
    let Some(v) = v else {
        return Ok(None);
    };
    match v {
        Value::Number(n) => {
            if let Some(u) = n.as_u64() {
                if u > i64::MAX as u64 {
                    return Err(Error::with_path(
                        ErrorKind::WrongType {
                            field: field.into(),
                            expected: TypeName::Integer,
                        },
                        path,
                    ));
                }
                return Ok(Some(u as i64));
            }
            if let Some(i64v) = n.as_i64() {
                if i64v < 0 {
                    return Err(Error::with_path(
                        ErrorKind::MinValue {
                            field: field.into(),
                            min: 0,
                        },
                        path,
                    ));
                }
                return Ok(Some(i64v));
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
