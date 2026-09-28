//! pptx 自己的 scope 与 ops 解析。

use serde_json::{Map, Value};

use crate::chart::{ChartType, NewSeries};
use crate::error::{Error, ErrorKind, IdReason, ScopeNeed, TypeName};
use crate::json;
use crate::smartart::{ColorsKind, LayoutKind, NewPoint, Role, StyleKind};

use super::chart;
use super::comments;
use super::picture;
use super::smartart;
use super::table;
use super::transition;

#[derive(Debug, Clone)]
pub struct InspectScope {
    pub slide: Option<u32>,
    pub max_shapes: u32,
    pub pictures: bool,
    pub charts: bool,
    pub smart_art: bool,
    pub notes: bool,
    pub comments: bool,
    pub tables: bool,
    pub timing: bool,
    pub masters: bool,
    pub handout: bool,
}

#[derive(Debug, Clone)]
pub struct SetShapeTextOp {
    pub id: String,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct InsertShapeOp {
    pub slide: usize,
    pub on_notes: bool,
    pub text: String,
    pub x: Option<i64>,
    pub y: Option<i64>,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct DeleteShapeOp {
    pub id: String,
    pub target: super::shape::ShapeTarget,
}

#[derive(Debug, Clone)]
pub struct SetShapePosOp {
    pub id: String,
    pub target: Option<super::shape::ShapeTarget>,
    pub placeholder: Option<super::master::PlaceholderId>,
    pub x: Option<i64>,
    pub y: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct SetShapeSizeOp {
    pub id: String,
    pub target: Option<super::shape::ShapeTarget>,
    pub placeholder: Option<super::master::PlaceholderId>,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct SetShapeFillOp {
    pub id: String,
    pub target: super::shape::ShapeTarget,
    pub color: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SetShapeLineOp {
    pub id: String,
    pub target: super::shape::ShapeTarget,
    pub color: Option<String>,
    pub width: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct SetNotesTextOp {
    pub slide: usize,
    pub id: String,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct InsertSlideOp {
    pub after: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct DeleteSlideOp {
    pub slide: usize,
}

#[derive(Debug, Clone)]
pub struct MoveSlideOp {
    pub slide: usize,
    pub after: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct SetSlideHiddenOp {
    pub slide: usize,
    pub hidden: bool,
}

#[derive(Debug, Clone)]
pub struct SetTransitionOp {
    pub slide: usize,
    pub type_name: Option<String>,
    pub dur: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct SetSlideLayoutOp {
    pub slide: usize,
    pub master: usize,
    pub layout: usize,
}

#[derive(Debug, Clone)]
pub struct SetHandoutTextOp {
    pub header: Option<String>,
    pub footer: Option<String>,
    pub date: Option<String>,
    pub sld_num: Option<String>,
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
pub struct InsertTrendlineOp {
    pub id: String,
    pub target: chart::ChartTarget,
    pub index: usize,
    pub spec: crate::chart::remain::TrendlineWrite,
}

#[derive(Debug, Clone)]
pub struct SetTrendlineOp {
    pub id: String,
    pub target: chart::ChartTarget,
    pub index: usize,
    pub tl_index: usize,
    pub spec: crate::chart::remain::TrendlineWrite,
}

#[derive(Debug, Clone)]
pub struct DeleteTrendlineOp {
    pub id: String,
    pub target: chart::ChartTarget,
    pub index: usize,
    pub tl_index: usize,
}

#[derive(Debug, Clone)]
pub struct InsertErrorBarsOp {
    pub id: String,
    pub target: chart::ChartTarget,
    pub index: usize,
    pub spec: crate::chart::remain::ErrorBarsWrite,
}

#[derive(Debug, Clone)]
pub struct SetErrorBarsOp {
    pub id: String,
    pub target: chart::ChartTarget,
    pub index: usize,
    pub eb_index: usize,
    pub spec: crate::chart::remain::ErrorBarsWrite,
}

#[derive(Debug, Clone)]
pub struct DeleteErrorBarsOp {
    pub id: String,
    pub target: chart::ChartTarget,
    pub index: usize,
    pub eb_index: usize,
}

#[derive(Debug, Clone)]
pub struct SetChartSeriesAxisOp {
    pub id: String,
    pub target: chart::ChartTarget,
    pub index: usize,
    pub secondary: bool,
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
pub struct InsertTableOp {
    pub slide: usize,
    pub rows: usize,
    pub cols: usize,
    pub cells: Vec<Vec<String>>,
    pub x: Option<i64>,
    pub y: Option<i64>,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct DeleteTableOp {
    pub id: String,
    pub target: table::TableTarget,
}

#[derive(Debug, Clone)]
pub struct InsertRowOp {
    pub id: String,
    pub target: table::TableOrRow,
}

#[derive(Debug, Clone)]
pub struct DeleteRowOp {
    pub id: String,
    pub table: table::TableTarget,
    pub row: usize,
}

#[derive(Debug, Clone)]
pub struct InsertColOp {
    pub id: String,
    pub target: table::TableTarget,
    pub after: u32,
}

#[derive(Debug, Clone)]
pub struct DeleteColOp {
    pub id: String,
    pub target: table::TableTarget,
    pub col: u32,
}

#[derive(Debug, Clone)]
pub struct SetTableCellTextOp {
    pub id: String,
    pub target: table::CellTarget,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct SetTablePosOp {
    pub id: String,
    pub target: table::TableTarget,
    pub x: Option<i64>,
    pub y: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct SetTableSizeOp {
    pub id: String,
    pub target: table::TableTarget,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
}

#[derive(Debug, Clone)]
pub enum PptxOp {
    SetShapeText(SetShapeTextOp),
    InsertShape(InsertShapeOp),
    DeleteShape(DeleteShapeOp),
    SetShapePos(SetShapePosOp),
    SetShapeSize(SetShapeSizeOp),
    SetShapeFill(SetShapeFillOp),
    SetShapeLine(SetShapeLineOp),
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
    InsertTrendline(InsertTrendlineOp),
    SetTrendline(SetTrendlineOp),
    DeleteTrendline(DeleteTrendlineOp),
    InsertErrorBars(InsertErrorBarsOp),
    SetErrorBars(SetErrorBarsOp),
    DeleteErrorBars(DeleteErrorBarsOp),
    SetChartSeriesAxis(SetChartSeriesAxisOp),
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
    InsertTable(InsertTableOp),
    DeleteTable(DeleteTableOp),
    InsertRow(InsertRowOp),
    DeleteRow(DeleteRowOp),
    InsertCol(InsertColOp),
    DeleteCol(DeleteColOp),
    SetTableCellText(SetTableCellTextOp),
    SetTablePos(SetTablePosOp),
    SetTableSize(SetTableSizeOp),
    InsertSlide(InsertSlideOp),
    DeleteSlide(DeleteSlideOp),
    MoveSlide(MoveSlideOp),
    SetSlideHidden(SetSlideHiddenOp),
    SetTransition(SetTransitionOp),
    SetSlideLayout(SetSlideLayoutOp),
    SetHandoutText(SetHandoutTextOp),
    SetProps(crate::props::SetPropsOp),
    SetCustomProp(crate::props::SetCustomPropOp),
    DeleteCustomProp(crate::props::DeleteCustomPropOp),
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
            "tables",
            "timing",
            "masters",
            "handout",
        ],
    )?;
    let pictures = parse_bool_field(obj.get("pictures"), "pictures")?;
    let charts = parse_bool_field(obj.get("charts"), "charts")?;
    let smart_art = parse_bool_field(obj.get("smartArt"), "smartArt")?;
    let notes = parse_bool_field(obj.get("notes"), "notes")?;
    let comments = parse_bool_field(obj.get("comments"), "comments")?;
    let tables = parse_bool_field(obj.get("tables"), "tables")?;
    let timing = parse_bool_field(obj.get("timing"), "timing")?;
    let masters = parse_bool_field(obj.get("masters"), "masters")?;
    let handout = parse_bool_field(obj.get("handout"), "handout")?;
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
        tables,
        timing,
        masters,
        handout,
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
            "insertShape" => PptxOp::InsertShape(parse_insert_shape(obj, i)?),
            "deleteShape" => PptxOp::DeleteShape(parse_delete_shape(obj, i)?),
            "setShapePos" => PptxOp::SetShapePos(parse_set_shape_pos(obj, i)?),
            "setShapeSize" => PptxOp::SetShapeSize(parse_set_shape_size(obj, i)?),
            "setShapeFill" => PptxOp::SetShapeFill(parse_set_shape_fill(obj, i)?),
            "setShapeLine" => PptxOp::SetShapeLine(parse_set_shape_line(obj, i)?),
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
            "insertTrendline" => PptxOp::InsertTrendline(parse_insert_trendline(obj, i)?),
            "setTrendline" => PptxOp::SetTrendline(parse_set_trendline(obj, i)?),
            "deleteTrendline" => PptxOp::DeleteTrendline(parse_delete_trendline(obj, i)?),
            "insertErrorBars" => PptxOp::InsertErrorBars(parse_insert_error_bars(obj, i)?),
            "setErrorBars" => PptxOp::SetErrorBars(parse_set_error_bars(obj, i)?),
            "deleteErrorBars" => PptxOp::DeleteErrorBars(parse_delete_error_bars(obj, i)?),
            "setChartSeriesAxis" => PptxOp::SetChartSeriesAxis(parse_set_series_axis(obj, i)?),
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
            "insertTable" => PptxOp::InsertTable(parse_insert_table(obj, i)?),
            "deleteTable" => PptxOp::DeleteTable(parse_delete_table(obj, i)?),
            "insertRow" => PptxOp::InsertRow(parse_insert_row(obj, i)?),
            "deleteRow" => PptxOp::DeleteRow(parse_delete_row(obj, i)?),
            "insertCol" => PptxOp::InsertCol(parse_insert_col(obj, i)?),
            "deleteCol" => PptxOp::DeleteCol(parse_delete_col(obj, i)?),
            "setTableCellText" => PptxOp::SetTableCellText(parse_set_table_cell_text(obj, i)?),
            "setTablePos" => PptxOp::SetTablePos(parse_set_table_pos(obj, i)?),
            "setTableSize" => PptxOp::SetTableSize(parse_set_table_size(obj, i)?),
            "insertSlide" => PptxOp::InsertSlide(parse_insert_slide(obj, i)?),
            "deleteSlide" => PptxOp::DeleteSlide(parse_delete_slide(obj, i)?),
            "moveSlide" => PptxOp::MoveSlide(parse_move_slide(obj, i)?),
            "setSlideHidden" => PptxOp::SetSlideHidden(parse_set_slide_hidden(obj, i)?),
            "setTransition" => PptxOp::SetTransition(parse_set_transition(obj, i)?),
            "setSlideLayout" => PptxOp::SetSlideLayout(parse_set_slide_layout(obj, i)?),
            "setHandoutText" => PptxOp::SetHandoutText(parse_set_handout_text(obj, i)?),
            "setProps" => {
                PptxOp::SetProps(crate::props::parse_set_props(obj, i, "pptx:presentation")?)
            }
            "setCustomProp" => PptxOp::SetCustomProp(crate::props::parse_set_custom_prop(
                obj,
                i,
                "pptx:presentation",
            )?),
            "deleteCustomProp" => {
                PptxOp::DeleteCustomProp(crate::props::parse_delete_custom_prop(obj, i)?)
            }
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
            | "mergeCells"
            | "splitCells"
            | "acceptRevision"
            | "rejectRevision"
            | "insertHyperlink"
            | "setHyperlink"
            | "deleteHyperlink"
            | "insertSparkline"
            | "setSparkline"
            | "deleteSparkline"
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
            | "setParaFormat"
            | "setRunFormat"
            | "insertSection"
            | "deleteSection"
            | "insertToc"
            | "deleteToc"
            | "insertMath"
            | "setMath"
            | "deleteMath"
            | "insertContentControl"
            | "setContentControl"
            | "deleteContentControl"
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
            | "insertSheet"
            | "deleteSheet"
            | "renameSheet"
            | "moveSheet"
            | "setSheetHidden"
            | "setCellStyle"
            | "setColWidth"
            | "setRowHeight"
            | "mergeRange"
            | "unmergeRange"
            | "insertDataValidation"
            | "setDataValidation"
            | "deleteDataValidation"
            | "insertConditionalFormat"
            | "setConditionalFormat"
            | "deleteConditionalFormat"
            | "insertSheetTable"
            | "setSheetTable"
            | "deleteSheetTable"
            | "insertPivotTable"
            | "setPivotTable"
            | "deletePivotTable"
            | "insertDefinedName"
            | "setDefinedName"
            | "deleteDefinedName"
            | "setPrintSetup"
            | "setSheetView"
            | "setAutoFilter"
            | "setSmartArtWrap"
            | "insertOle"
            | "deleteOle"
            | "insertWatermark"
            | "deleteWatermark"
            | "setProtection"
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

fn parse_insert_shape(obj: &Map<String, Value>, i: usize) -> Result<InsertShapeOp, Error> {
    deny_keys(obj, i, &["op", "id", "text", "x", "y", "cx", "cy"])?;
    let id = require_id(obj, i)?;
    let (slide, on_notes) =
        picture::parse_insert_slide_or_notes(&id, i, crate::error::IdReason::InsertNeedsSlide)?;
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
        None => String::new(),
    };
    let x = parse_off_field(obj.get("x"), i, "x")?;
    let y = parse_off_field(obj.get("y"), i, "y")?;
    if x.is_some() != y.is_some() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "insertShape".into(),
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
                op: "insertShape".into(),
                field: "cx and cy together".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(InsertShapeOp {
        slide,
        on_notes,
        text,
        x,
        y,
        cx,
        cy,
    })
}

fn parse_delete_shape(obj: &Map<String, Value>, i: usize) -> Result<DeleteShapeOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let target = super::shape::parse_shape_id(&id, i)?;
    Ok(DeleteShapeOp { id, target })
}

fn parse_set_shape_pos(obj: &Map<String, Value>, i: usize) -> Result<SetShapePosOp, Error> {
    deny_keys(obj, i, &["op", "id", "x", "y"])?;
    let id = require_id(obj, i)?;
    let (target, placeholder) = parse_shape_host(&id, i)?;
    let x = parse_off_field(obj.get("x"), i, "x")?;
    let y = parse_off_field(obj.get("y"), i, "y")?;
    if x.is_none() && y.is_none() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setShapePos".into(),
                field: "x or y".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(SetShapePosOp {
        id,
        target,
        placeholder,
        x,
        y,
    })
}

fn parse_set_shape_size(obj: &Map<String, Value>, i: usize) -> Result<SetShapeSizeOp, Error> {
    deny_keys(obj, i, &["op", "id", "cx", "cy"])?;
    let id = require_id(obj, i)?;
    let (target, placeholder) = parse_shape_host(&id, i)?;
    let cx = parse_emu_field(obj.get("cx"), i, "cx")?;
    let cy = parse_emu_field(obj.get("cy"), i, "cy")?;
    if cx.is_none() && cy.is_none() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setShapeSize".into(),
                field: "cx or cy".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(SetShapeSizeOp {
        id,
        target,
        placeholder,
        cx,
        cy,
    })
}

fn parse_set_shape_fill(obj: &Map<String, Value>, i: usize) -> Result<SetShapeFillOp, Error> {
    deny_keys(obj, i, &["op", "id", "color"])?;
    let id = require_id(obj, i)?;
    let target = super::shape::parse_shape_id(&id, i)?;
    let color = parse_color_field(obj, i, "setShapeFill")?;
    Ok(SetShapeFillOp { id, target, color })
}

fn parse_set_shape_line(obj: &Map<String, Value>, i: usize) -> Result<SetShapeLineOp, Error> {
    deny_keys(obj, i, &["op", "id", "color", "width"])?;
    let id = require_id(obj, i)?;
    let target = super::shape::parse_shape_id(&id, i)?;
    if !obj.contains_key("color") {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setShapeLine".into(),
                field: "color".into(),
            },
            format!("ops[{i}].color"),
        ));
    }
    let color = parse_color_field(obj, i, "setShapeLine")?;
    let width = parse_emu_field(obj.get("width"), i, "width")?;
    if color.is_none() && width.is_some() {
        return Err(Error::with_path(
            ErrorKind::UnknownField {
                field: "width".into(),
            },
            format!("ops[{i}].width"),
        ));
    }
    Ok(SetShapeLineOp {
        id,
        target,
        color,
        width,
    })
}

fn parse_color_field(
    obj: &Map<String, Value>,
    i: usize,
    op: &str,
) -> Result<Option<String>, Error> {
    match obj.get("color") {
        None => Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: op.into(),
                field: "color".into(),
            },
            format!("ops[{i}].color"),
        )),
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

fn parse_insert_slide(obj: &Map<String, Value>, i: usize) -> Result<InsertSlideOp, Error> {
    deny_keys(obj, i, &["op", "id", "after"])?;
    let id = require_id(obj, i)?;
    if id != "pptx:presentation" {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::InsertSlideNeedsPresentation,
            },
            format!("ops[{i}].id"),
        ));
    }
    Ok(InsertSlideOp {
        after: parse_optional_slide_after(obj, i, false)?,
    })
}

fn parse_delete_slide(obj: &Map<String, Value>, i: usize) -> Result<DeleteSlideOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    Ok(DeleteSlideOp {
        slide: parse_slide_only_id(&id, i, "id")?,
    })
}

fn parse_move_slide(obj: &Map<String, Value>, i: usize) -> Result<MoveSlideOp, Error> {
    deny_keys(obj, i, &["op", "id", "after"])?;
    let id = require_id(obj, i)?;
    Ok(MoveSlideOp {
        slide: parse_slide_only_id(&id, i, "id")?,
        after: parse_optional_slide_after(obj, i, true)?,
    })
}

fn parse_set_slide_hidden(obj: &Map<String, Value>, i: usize) -> Result<SetSlideHiddenOp, Error> {
    deny_keys(obj, i, &["op", "id", "hidden"])?;
    let id = require_id(obj, i)?;
    Ok(SetSlideHiddenOp {
        slide: parse_slide_only_id(&id, i, "id")?,
        hidden: parse_required_bool(obj.get("hidden"), i, "setSlideHidden", "hidden")?,
    })
}

fn parse_set_transition(obj: &Map<String, Value>, i: usize) -> Result<SetTransitionOp, Error> {
    deny_keys(obj, i, &["op", "id", "type", "dur"])?;
    let id = require_id(obj, i)?;
    let type_name = match obj.get("type") {
        None => {
            return Err(Error::with_path(
                ErrorKind::OpRequiresField {
                    op: "setTransition".into(),
                    field: "type".into(),
                },
                format!("ops[{i}].type"),
            ));
        }
        Some(Value::Null) => None,
        Some(Value::String(s)) => {
            if s.is_empty() || s == "other" || !transition::is_writable_type(s) {
                return Err(Error::with_path(
                    ErrorKind::InvalidTransitionType,
                    format!("ops[{i}].type"),
                ));
            }
            Some(s.clone())
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "type".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].type"),
            ));
        }
    };
    let dur = match obj.get("dur") {
        None => None,
        Some(_) if type_name.is_none() => None,
        Some(v) => Some(parse_transition_dur(v, i)?),
    };
    Ok(SetTransitionOp {
        slide: parse_slide_only_id(&id, i, "id")?,
        type_name,
        dur,
    })
}

fn parse_set_slide_layout(obj: &Map<String, Value>, i: usize) -> Result<SetSlideLayoutOp, Error> {
    deny_keys(obj, i, &["op", "id", "layout"])?;
    let id = require_id(obj, i)?;
    let layout = match obj.get("layout") {
        Some(Value::String(s)) if !s.is_empty() => s.clone(),
        Some(Value::String(_)) | None => {
            return Err(Error::with_path(
                ErrorKind::OpRequiresField {
                    op: "setSlideLayout".into(),
                    field: "layout".into(),
                },
                format!("ops[{i}].layout"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "layout".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].layout"),
            ));
        }
    };
    let (master, layout_n) = super::master::parse_layout_ref(&layout, i)?;
    Ok(SetSlideLayoutOp {
        slide: parse_slide_only_id(&id, i, "id")?,
        master,
        layout: layout_n,
    })
}

fn parse_set_handout_text(obj: &Map<String, Value>, i: usize) -> Result<SetHandoutTextOp, Error> {
    deny_keys(obj, i, &["op", "id", "header", "footer", "date", "sldNum"])?;
    let id = require_id(obj, i)?;
    if id != "handout" {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: crate::error::IdReason::SetHandoutTextNeedsHandout,
            },
            format!("ops[{i}].id"),
        ));
    }
    let header = parse_optional_slot(obj, i, "header")?;
    let footer = parse_optional_slot(obj, i, "footer")?;
    let date = parse_optional_slot(obj, i, "date")?;
    let sld_num = parse_optional_slot(obj, i, "sldNum")?;
    if header.is_none() && footer.is_none() && date.is_none() && sld_num.is_none() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setHandoutText".into(),
                field: "header, footer, date, or sldNum".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(SetHandoutTextOp {
        header,
        footer,
        date,
        sld_num,
    })
}

fn parse_optional_slot(
    obj: &Map<String, Value>,
    i: usize,
    field: &str,
) -> Result<Option<String>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].{field}"),
        )),
    }
}

fn parse_shape_host(
    id: &str,
    i: usize,
) -> Result<
    (
        Option<super::shape::ShapeTarget>,
        Option<super::master::PlaceholderId>,
    ),
    Error,
> {
    if id.starts_with("master:") {
        Ok((None, Some(super::master::parse_placeholder_id(id, i)?)))
    } else {
        Ok((Some(super::shape::parse_shape_id(id, i)?), None))
    }
}

fn parse_transition_dur(v: &Value, i: usize) -> Result<u32, Error> {
    let path = format!("ops[{i}].dur");
    let n = match v {
        Value::Number(num) => num.as_u64().ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: "dur".into(),
                    expected: TypeName::Integer,
                },
                path.clone(),
            )
        })?,
        _ => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "dur".into(),
                    expected: TypeName::Integer,
                },
                path,
            ));
        }
    };
    if !(1..=60000).contains(&n) {
        return Err(Error::with_path(ErrorKind::InvalidTransitionDur, path));
    }
    Ok(n as u32)
}

fn parse_optional_slide_after(
    obj: &Map<String, Value>,
    i: usize,
    allow_null: bool,
) -> Result<Option<usize>, Error> {
    match obj.get("after") {
        None => Ok(None),
        Some(Value::Null) if allow_null => Ok(None),
        Some(Value::String(s)) => Ok(Some(parse_slide_only_id(s, i, "after")?)),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "after".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].after"),
        )),
    }
}

fn parse_slide_only_id(id: &str, op_index: usize, field: &str) -> Result<usize, Error> {
    let path = format!("ops[{op_index}].{field}");
    let Some(rest) = id.strip_prefix("slide:") else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SlideOpNeedsSlide,
            },
            path,
        ));
    };
    if rest.is_empty() || rest.contains('/') {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SlideOpNeedsSlide,
            },
            path,
        ));
    }
    let slide: usize = rest.parse().map_err(|_| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SlideIndexNotInt,
            },
            path.clone(),
        )
    })?;
    if slide < 1 {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SlideIndexFromOne,
            },
            path,
        ));
    }
    Ok(slide)
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

fn parse_insert_trendline(obj: &Map<String, Value>, i: usize) -> Result<InsertTrendlineOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op",
            "id",
            "type",
            "order",
            "period",
            "displayRSquared",
            "displayEquation",
            "name",
        ],
    )?;
    let id = require_id(obj, i)?;
    let (target, index) = chart::parse_series_id(&id, &format!("ops[{i}].id"))?;
    let spec = crate::chart::remain::parse_trendline_write(obj, i, true)?;
    Ok(InsertTrendlineOp {
        id,
        target,
        index,
        spec,
    })
}

fn parse_set_trendline(obj: &Map<String, Value>, i: usize) -> Result<SetTrendlineOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op",
            "id",
            "type",
            "order",
            "period",
            "displayRSquared",
            "displayEquation",
            "name",
        ],
    )?;
    let id = require_id(obj, i)?;
    let (target, index, tl_index) = chart::parse_trendline_id(&id, &format!("ops[{i}].id"))?;
    let spec = crate::chart::remain::parse_trendline_write(obj, i, false)?;
    Ok(SetTrendlineOp {
        id,
        target,
        index,
        tl_index,
        spec,
    })
}

fn parse_delete_trendline(obj: &Map<String, Value>, i: usize) -> Result<DeleteTrendlineOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let (target, index, tl_index) = chart::parse_trendline_id(&id, &format!("ops[{i}].id"))?;
    Ok(DeleteTrendlineOp {
        id,
        target,
        index,
        tl_index,
    })
}

fn parse_insert_error_bars(obj: &Map<String, Value>, i: usize) -> Result<InsertErrorBarsOp, Error> {
    deny_keys(obj, i, &["op", "id", "dir", "barType", "valType", "value"])?;
    let id = require_id(obj, i)?;
    let (target, index) = chart::parse_series_id(&id, &format!("ops[{i}].id"))?;
    let spec = crate::chart::remain::parse_error_bars_write(obj, i, true)?;
    Ok(InsertErrorBarsOp {
        id,
        target,
        index,
        spec,
    })
}

fn parse_set_error_bars(obj: &Map<String, Value>, i: usize) -> Result<SetErrorBarsOp, Error> {
    deny_keys(obj, i, &["op", "id", "dir", "barType", "valType", "value"])?;
    let id = require_id(obj, i)?;
    let (target, index, eb_index) = chart::parse_error_bars_id(&id, &format!("ops[{i}].id"))?;
    let spec = crate::chart::remain::parse_error_bars_write(obj, i, false)?;
    Ok(SetErrorBarsOp {
        id,
        target,
        index,
        eb_index,
        spec,
    })
}

fn parse_delete_error_bars(obj: &Map<String, Value>, i: usize) -> Result<DeleteErrorBarsOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let (target, index, eb_index) = chart::parse_error_bars_id(&id, &format!("ops[{i}].id"))?;
    Ok(DeleteErrorBarsOp {
        id,
        target,
        index,
        eb_index,
    })
}

fn parse_set_series_axis(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<SetChartSeriesAxisOp, Error> {
    deny_keys(obj, i, &["op", "id", "axis"])?;
    let id = require_id(obj, i)?;
    let (target, index) = chart::parse_series_id(&id, &format!("ops[{i}].id"))?;
    let secondary = crate::chart::remain::parse_series_axis(obj, i)?;
    Ok(SetChartSeriesAxisOp {
        id,
        target,
        index,
        secondary,
    })
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

fn parse_insert_table(obj: &Map<String, Value>, i: usize) -> Result<InsertTableOp, Error> {
    deny_keys(
        obj,
        i,
        &["op", "id", "rows", "cols", "cells", "x", "y", "cx", "cy"],
    )?;
    let id = require_id(obj, i)?;
    let slide = table::parse_insert_slide_id(&id, i)?;
    let rows = table::parse_dim(
        obj.get("rows"),
        &format!("ops[{i}].rows"),
        "rows",
        1,
        table::max_rows(),
    )?;
    let cols = table::parse_dim(
        obj.get("cols"),
        &format!("ops[{i}].cols"),
        "cols",
        1,
        table::max_cols(),
    )?;
    let cells = table::parse_cells(obj.get("cells"), &format!("ops[{i}].cells"), rows, cols)?;
    let x = parse_off_field(obj.get("x"), i, "x")?;
    let y = parse_off_field(obj.get("y"), i, "y")?;
    if x.is_some() != y.is_some() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "insertTable".into(),
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
                op: "insertTable".into(),
                field: "cx and cy together".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(InsertTableOp {
        slide,
        rows,
        cols,
        cells,
        x,
        y,
        cx,
        cy,
    })
}

fn parse_delete_table(obj: &Map<String, Value>, i: usize) -> Result<DeleteTableOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let target = table::parse_table_id(
        &id,
        &format!("ops[{i}].id"),
        IdReason::DeleteTableNeedsTable,
    )?;
    Ok(DeleteTableOp { id, target })
}

fn parse_insert_row(obj: &Map<String, Value>, i: usize) -> Result<InsertRowOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let target = table::parse_table_or_row(&id, &format!("ops[{i}].id"))?;
    Ok(InsertRowOp { id, target })
}

fn parse_delete_row(obj: &Map<String, Value>, i: usize) -> Result<DeleteRowOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let (table, row) = table::parse_row_id(&id, &format!("ops[{i}].id"))?;
    Ok(DeleteRowOp { id, table, row })
}

fn parse_insert_col(obj: &Map<String, Value>, i: usize) -> Result<InsertColOp, Error> {
    deny_keys(obj, i, &["op", "id", "after"])?;
    let id = require_id(obj, i)?;
    let target = table::parse_table_id(&id, &format!("ops[{i}].id"), IdReason::NeedTableId)?;
    let after = table::parse_after(obj.get("after"), i)?;
    Ok(InsertColOp { id, target, after })
}

fn parse_delete_col(obj: &Map<String, Value>, i: usize) -> Result<DeleteColOp, Error> {
    deny_keys(obj, i, &["op", "id", "col"])?;
    let id = require_id(obj, i)?;
    let target = table::parse_table_id(&id, &format!("ops[{i}].id"), IdReason::NeedTableId)?;
    let col = table::parse_dim(
        obj.get("col"),
        &format!("ops[{i}].col"),
        "col",
        1,
        i64::MAX as u64,
    )? as u32;
    Ok(DeleteColOp { id, target, col })
}

fn parse_set_table_cell_text(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<SetTableCellTextOp, Error> {
    deny_keys(obj, i, &["op", "id", "text"])?;
    let id = require_id(obj, i)?;
    let target = table::parse_cell_id(&id, &format!("ops[{i}].id"))?;
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
                    op: "setTableCellText".into(),
                    field: "text".into(),
                },
                format!("ops[{i}].text"),
            ));
        }
    };
    Ok(SetTableCellTextOp { id, target, text })
}

fn parse_set_table_pos(obj: &Map<String, Value>, i: usize) -> Result<SetTablePosOp, Error> {
    deny_keys(obj, i, &["op", "id", "x", "y"])?;
    let id = require_id(obj, i)?;
    let target = table::parse_table_id(&id, &format!("ops[{i}].id"), IdReason::NeedTableId)?;
    let x = parse_off_field(obj.get("x"), i, "x")?;
    let y = parse_off_field(obj.get("y"), i, "y")?;
    if x.is_none() && y.is_none() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setTablePos".into(),
                field: "x or y".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(SetTablePosOp { id, target, x, y })
}

fn parse_set_table_size(obj: &Map<String, Value>, i: usize) -> Result<SetTableSizeOp, Error> {
    deny_keys(obj, i, &["op", "id", "cx", "cy"])?;
    let id = require_id(obj, i)?;
    let target = table::parse_table_id(&id, &format!("ops[{i}].id"), IdReason::NeedTableId)?;
    let cx = parse_emu_field(obj.get("cx"), i, "cx")?;
    let cy = parse_emu_field(obj.get("cy"), i, "cy")?;
    if cx.is_none() && cy.is_none() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setTableSize".into(),
                field: "cx or cy".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(SetTableSizeOp { id, target, cx, cy })
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

pub(crate) fn parse_create_table(
    v: &Value,
    prefix: &str,
    slide: usize,
) -> Result<InsertTableOp, Error> {
    let obj = v.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "tables".into(),
                expected: TypeName::Object,
            },
            prefix.to_string(),
        )
    })?;
    deny_keys_at(
        obj,
        prefix,
        &["rows", "cols", "cells", "x", "y", "cx", "cy"],
    )?;
    let rows = table::parse_dim(
        obj.get("rows"),
        &format!("{prefix}.rows"),
        "rows",
        1,
        table::max_rows(),
    )?;
    let cols = table::parse_dim(
        obj.get("cols"),
        &format!("{prefix}.cols"),
        "cols",
        1,
        table::max_cols(),
    )?;
    let cells = table::parse_cells(obj.get("cells"), &format!("{prefix}.cells"), rows, cols)?;
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
    Ok(InsertTableOp {
        slide,
        rows,
        cols,
        cells,
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
