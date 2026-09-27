//! docx 自己的 scope 与 ops 解析。

use std::collections::HashSet;

use serde_json::Value;

use crate::error::{Error, ErrorKind, IdReason, ScopeNeed, TypeName};
use crate::json;

use super::charts::{self, ChartType, NewSeries};
use super::hf::{HfKind, HfType};
use super::para::{
    self, BodyPara, CommentIdTarget, NoteIdTarget, NoteKind, ParaTarget, TxbxTarget,
};
use super::picture::{self, AlignH, AlignV, PicKind, PicTarget, WrapKind, WrapPos};
use super::tbl::{self, InsertAnchor, TableRef};

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
    /// 为 true 时列出脚注、尾注与分隔符。
    pub notes: bool,
    /// 为 true 时列出批注。
    pub comments: bool,
    /// 为 true 时列出嵌入图表。
    pub charts: bool,
    /// 为 true 时列出 SmartArt。
    pub smart_art: bool,
    /// 为 true 时列出表格。
    pub tables: bool,
    /// 为 true 时列出修订痕迹。
    pub revisions: bool,
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
pub struct InsertHeaderFooterOp {
    pub id: String,
    pub n: usize,
    pub kind: HfKind,
    pub hf_type: HfType,
}

#[derive(Debug, Clone)]
pub struct SetTitlePageOp {
    pub id: String,
    pub n: usize,
    pub title_page: bool,
}

#[derive(Debug, Clone)]
pub struct SetEvenAndOddHeadersOp {
    #[allow(dead_code)]
    pub id: String,
    pub even_and_odd: bool,
}

#[derive(Debug, Clone)]
pub struct SetPageMarginOp {
    pub id: String,
    pub n: usize,
    pub top: Option<i64>,
    pub bottom: Option<i64>,
    pub left: Option<u32>,
    pub right: Option<u32>,
    pub gutter: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct SetPageSizeOp {
    pub id: String,
    pub n: usize,
    pub page_width: Option<u32>,
    pub page_height: Option<u32>,
    pub orient: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ColWidth {
    pub width: Option<u32>,
    pub space: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct SetColumnsOp {
    pub id: String,
    pub n: usize,
    pub num: Option<u32>,
    pub space: Option<u32>,
    pub equal_width: Option<bool>,
    pub sep: Option<bool>,
    pub cols: Option<Vec<ColWidth>>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct SetVerticalAlignOp {
    pub id: String,
    pub n: usize,
    /// `None` 表示 JSON `null`，删掉 `w:vAlign`。
    pub v_align: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SetLineNumberingOp {
    pub id: String,
    pub n: usize,
    pub count_by: Option<u32>,
    pub start: Option<u32>,
    pub distance: Option<u32>,
    pub restart: Option<String>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct SetDocGridOp {
    pub id: String,
    pub n: usize,
    pub grid_type: Option<String>,
    pub line_pitch: Option<i64>,
    pub char_space: Option<i64>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct SetRtlGutterOp {
    pub id: String,
    pub n: usize,
    pub rtl_gutter: bool,
}

#[derive(Debug, Clone)]
pub struct SetMirrorMarginsOp {
    #[allow(dead_code)]
    pub id: String,
    pub mirror_margins: bool,
}

#[derive(Debug, Clone)]
pub struct SetGutterAtTopOp {
    #[allow(dead_code)]
    pub id: String,
    pub gutter_at_top: bool,
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
pub enum InsertParaPlace {
    TextBox {
        box_target: TxbxTarget,
        after: Option<ParaTarget>,
    },
    Note {
        kind: NoteKind,
        note_id: String,
        after: Option<BodyPara>,
    },
    Comment {
        comment_id: String,
        after: Option<BodyPara>,
    },
    Body {
        after: Option<BodyPara>,
    },
    HeaderFooter {
        kind: HfKind,
        hf_type: HfType,
        n: u32,
        after: Option<BodyPara>,
    },
}

#[derive(Debug, Clone)]
pub struct InsertParaOp {
    pub id: String,
    pub place: InsertParaPlace,
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
pub struct InsertNoteOp {
    pub id: String,
    pub host: ParaTarget,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct DeleteNoteOp {
    pub id: String,
    pub kind: NoteKind,
    pub note_id: String,
}

#[derive(Debug, Clone)]
pub enum NoteNumberTarget {
    Document,
    Section(usize),
}

#[derive(Debug, Clone)]
pub struct SetNoteNumberingOp {
    pub id: String,
    pub target: NoteNumberTarget,
    pub kind: NoteKind,
    pub num_fmt: Option<String>,
    pub num_start: Option<u32>,
    pub num_restart: Option<String>,
}

#[derive(Debug, Clone)]
pub struct InsertCommentOp {
    pub start: String,
    pub end: String,
    pub start_target: ParaTarget,
    pub end_target: ParaTarget,
    pub text: String,
    pub author: String,
    pub initials: String,
    pub date: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ReplyCommentOp {
    pub id: String,
    pub comment_id: String,
    pub text: String,
    pub author: String,
    pub initials: String,
    pub date: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DeleteCommentOp {
    pub id: String,
    pub comment_id: String,
}

#[derive(Debug, Clone)]
pub struct SetCommentTextOp {
    pub id: String,
    pub comment_id: String,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct SetCommentMetaOp {
    pub id: String,
    pub comment_id: String,
    pub author: Option<String>,
    pub initials: Option<String>,
    pub date: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SetCommentDoneOp {
    pub id: String,
    pub comment_id: String,
    pub done: bool,
}

#[derive(Debug, Clone)]
pub struct InsertTableOp {
    pub id: String,
    pub anchor: InsertAnchor,
    pub cells: Vec<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct DeleteTableOp {
    pub id: String,
    pub target: TableRef,
}

#[derive(Debug, Clone)]
pub struct InsertRowOp {
    pub id: String,
    pub target: TableRef,
}

#[derive(Debug, Clone)]
pub struct DeleteRowOp {
    pub id: String,
    pub target: TableRef,
}

#[derive(Debug, Clone)]
pub struct InsertColOp {
    pub id: String,
    pub target: TableRef,
    pub after: u32,
}

#[derive(Debug, Clone)]
pub struct DeleteColOp {
    pub id: String,
    pub target: TableRef,
    pub col: u32,
}

#[derive(Debug, Clone)]
pub struct MergeCellsOp {
    pub id: String,
    pub target: TableRef,
    pub start_row: usize,
    pub start_col: usize,
    pub end_row: usize,
    pub end_col: usize,
}

#[derive(Debug, Clone)]
pub struct SplitCellsOp {
    pub id: String,
    pub target: TableRef,
}

#[derive(Debug, Clone)]
pub struct RevisionOp {
    pub id: String,
    pub target: super::revision::RevRef,
}

#[derive(Debug, Clone)]
pub enum DocxOp {
    SetParaText(SetParaTextOp),
    SetHeaderFooterDistance(SetHfDistanceOp),
    SetHeaderFooterLink(SetHfLinkOp),
    InsertHeaderFooter(InsertHeaderFooterOp),
    SetTitlePage(SetTitlePageOp),
    SetEvenAndOddHeaders(SetEvenAndOddHeadersOp),
    SetPageMargin(SetPageMarginOp),
    SetPageSize(SetPageSizeOp),
    SetColumns(SetColumnsOp),
    SetVerticalAlign(SetVerticalAlignOp),
    SetLineNumbering(SetLineNumberingOp),
    SetDocGrid(SetDocGridOp),
    SetRtlGutter(SetRtlGutterOp),
    SetMirrorMargins(SetMirrorMarginsOp),
    SetGutterAtTop(SetGutterAtTopOp),
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
    InsertTable(InsertTableOp),
    DeleteTable(DeleteTableOp),
    InsertRow(InsertRowOp),
    DeleteRow(DeleteRowOp),
    InsertCol(InsertColOp),
    DeleteCol(DeleteColOp),
    MergeCells(MergeCellsOp),
    SplitCells(SplitCellsOp),
    AcceptRevision(RevisionOp),
    RejectRevision(RevisionOp),
    SetTextBoxLink(SetTextBoxLinkOp),
    InsertFootnote(InsertNoteOp),
    InsertEndnote(InsertNoteOp),
    DeleteFootnote(DeleteNoteOp),
    DeleteEndnote(DeleteNoteOp),
    SetNoteNumbering(SetNoteNumberingOp),
    InsertComment(InsertCommentOp),
    ReplyComment(ReplyCommentOp),
    DeleteComment(DeleteCommentOp),
    SetCommentText(SetCommentTextOp),
    SetCommentMeta(SetCommentMetaOp),
    SetCommentDone(SetCommentDoneOp),
    InsertChart(charts::InsertChartOp),
    DeleteChart(charts::DeleteChartOp),
    SetChartTitle(charts::SetChartTitleOp),
    SetChartSeries(charts::SetChartSeriesOp),
    AddChartSeries(charts::AddChartSeriesOp),
    DeleteChartSeries(charts::DeleteChartSeriesOp),
    SetChartType(charts::SetChartTypeOp),
    SetChartAxisTitle(charts::SetChartAxisTitleOp),
    SetChartLegend(charts::SetChartLegendOp),
    SetChartDataLabels(charts::SetChartDataLabelsOp),
    SetChartSize(charts::SetChartSizeOp),
    SetChartWrap(charts::SetChartWrapOp),
    InsertSmartArt(super::smartart::InsertSmartArtOp),
    DeleteSmartArt(super::smartart::DeleteSmartArtOp),
    SetSmartArtText(super::smartart::SetSmartArtTextOp),
    AddSmartArtPoint(super::smartart::AddSmartArtPointOp),
    DeleteSmartArtPoint(super::smartart::DeleteSmartArtPointOp),
    SetSmartArtSize(super::smartart::SetSmartArtSizeOp),
    SetSmartArtWrap(super::smartart::SetSmartArtWrapOp),
    SetSmartArtColors(super::smartart::SetSmartArtColorsOp),
    SetSmartArtStyle(super::smartart::SetSmartArtStyleOp),
    SetSmartArtLayout(super::smartart::SetSmartArtLayoutOp),
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
            "from",
            "maxParas",
            "headerFooter",
            "pictures",
            "textBoxes",
            "textBoxContent",
            "notes",
            "comments",
            "charts",
            "smartArt",
            "tables",
            "revisions",
        ],
    )?;

    let header_footer = parse_header_footer(obj.get("headerFooter"))?;
    let pictures = parse_bool_field(obj.get("pictures"), "pictures")?;
    let text_boxes = parse_bool_field(obj.get("textBoxes"), "textBoxes")?;
    let text_box_content = parse_bool_field(obj.get("textBoxContent"), "textBoxContent")?;
    let notes = parse_bool_field(obj.get("notes"), "notes")?;
    let comments = parse_bool_field(obj.get("comments"), "comments")?;
    let charts = parse_bool_field(obj.get("charts"), "charts")?;
    let smart_art = parse_bool_field(obj.get("smartArt"), "smartArt")?;
    let tables = parse_bool_field(obj.get("tables"), "tables")?;
    let revisions = parse_bool_field(obj.get("revisions"), "revisions")?;
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
                && !obj.contains_key("notes")
                && !obj.contains_key("comments")
                && !obj.contains_key("charts")
                && !obj.contains_key("smartArt")
                && !obj.contains_key("tables")
                && !obj.contains_key("revisions")
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
                notes,
                comments,
                charts,
                smart_art,
                tables,
                revisions,
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
                notes,
                comments,
                charts,
                smart_art,
                tables,
                revisions,
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
                ErrorKind::MissingField { field: "op".into() },
                format!("ops[{i}].op"),
            )
        })?;
        if op_name == "setCell"
            || op_name == "setShapeText"
            || op_name == "setNotesText"
            || op_name == "setPictureAnchor"
            || op_name == "setChartAnchor"
            || op_name == "setHeaderFooterText"
            || op_name == "insertChartSheet"
            || op_name == "deleteChartSheet"
            || op_name == "setPicturePos"
            || op_name == "setChartPos"
            || op_name == "setSmartArtPos"
        {
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
            "setHeaderFooterDistance" => out.push(DocxOp::SetHeaderFooterDistance(
                parse_set_hf_distance(obj, i)?,
            )),
            "setHeaderFooterLink" => {
                out.push(DocxOp::SetHeaderFooterLink(parse_set_hf_link(obj, i)?))
            }
            "insertHeaderFooter" => out.push(DocxOp::InsertHeaderFooter(parse_insert_hf(obj, i)?)),
            "setTitlePage" => out.push(DocxOp::SetTitlePage(parse_set_title_page(obj, i)?)),
            "setEvenAndOddHeaders" => out.push(DocxOp::SetEvenAndOddHeaders(
                parse_set_even_and_odd(obj, i)?,
            )),
            "setPageMargin" => out.push(DocxOp::SetPageMargin(parse_set_page_margin(obj, i)?)),
            "setPageSize" => out.push(DocxOp::SetPageSize(parse_set_page_size(obj, i)?)),
            "setColumns" => out.push(DocxOp::SetColumns(parse_set_columns(obj, i)?)),
            "setVerticalAlign" => {
                out.push(DocxOp::SetVerticalAlign(parse_set_vertical_align(obj, i)?))
            }
            "setLineNumbering" => {
                out.push(DocxOp::SetLineNumbering(parse_set_line_numbering(obj, i)?))
            }
            "setDocGrid" => out.push(DocxOp::SetDocGrid(parse_set_doc_grid(obj, i)?)),
            "setRtlGutter" => out.push(DocxOp::SetRtlGutter(parse_set_rtl_gutter(obj, i)?)),
            "setMirrorMargins" => {
                out.push(DocxOp::SetMirrorMargins(parse_set_mirror_margins(obj, i)?))
            }
            "setGutterAtTop" => out.push(DocxOp::SetGutterAtTop(parse_set_gutter_at_top(obj, i)?)),
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
            "insertTable" => out.push(DocxOp::InsertTable(parse_insert_table(obj, i)?)),
            "deleteTable" => out.push(DocxOp::DeleteTable(parse_delete_table(obj, i)?)),
            "insertRow" => out.push(DocxOp::InsertRow(parse_insert_row(obj, i)?)),
            "deleteRow" => out.push(DocxOp::DeleteRow(parse_delete_row(obj, i)?)),
            "insertCol" => out.push(DocxOp::InsertCol(parse_insert_col(obj, i)?)),
            "deleteCol" => out.push(DocxOp::DeleteCol(parse_delete_col(obj, i)?)),
            "mergeCells" => out.push(DocxOp::MergeCells(parse_merge_cells(obj, i)?)),
            "splitCells" => out.push(DocxOp::SplitCells(parse_split_cells(obj, i)?)),
            "acceptRevision" => out.push(DocxOp::AcceptRevision(parse_revision_op(obj, i)?)),
            "rejectRevision" => out.push(DocxOp::RejectRevision(parse_revision_op(obj, i)?)),
            "setTextBoxLink" => out.push(DocxOp::SetTextBoxLink(parse_set_txbx_link(obj, i)?)),
            "insertFootnote" => out.push(DocxOp::InsertFootnote(parse_insert_note(
                obj,
                i,
                NoteKind::Footnote,
            )?)),
            "insertEndnote" => out.push(DocxOp::InsertEndnote(parse_insert_note(
                obj,
                i,
                NoteKind::Endnote,
            )?)),
            "deleteFootnote" => out.push(DocxOp::DeleteFootnote(parse_delete_note(
                obj,
                i,
                NoteKind::Footnote,
            )?)),
            "deleteEndnote" => out.push(DocxOp::DeleteEndnote(parse_delete_note(
                obj,
                i,
                NoteKind::Endnote,
            )?)),
            "setNoteNumbering" => {
                out.push(DocxOp::SetNoteNumbering(parse_set_note_numbering(obj, i)?))
            }
            "insertComment" => out.push(DocxOp::InsertComment(parse_insert_comment(obj, i)?)),
            "replyComment" => out.push(DocxOp::ReplyComment(parse_reply_comment(obj, i)?)),
            "deleteComment" => out.push(DocxOp::DeleteComment(parse_delete_comment(obj, i)?)),
            "setCommentText" => out.push(DocxOp::SetCommentText(parse_set_comment_text(obj, i)?)),
            "setCommentMeta" => out.push(DocxOp::SetCommentMeta(parse_set_comment_meta(obj, i)?)),
            "setCommentDone" => out.push(DocxOp::SetCommentDone(parse_set_comment_done(obj, i)?)),
            "insertChart" => out.push(DocxOp::InsertChart(parse_insert_chart(obj, i)?)),
            "deleteChart" => out.push(DocxOp::DeleteChart(parse_delete_chart(obj, i)?)),
            "setChartTitle" => out.push(DocxOp::SetChartTitle(parse_set_chart_title(obj, i)?)),
            "setChartSeries" => out.push(DocxOp::SetChartSeries(parse_set_chart_series(obj, i)?)),
            "addChartSeries" => out.push(DocxOp::AddChartSeries(parse_add_chart_series(obj, i)?)),
            "deleteChartSeries" => out.push(DocxOp::DeleteChartSeries(parse_delete_chart_series(
                obj, i,
            )?)),
            "setChartType" => out.push(DocxOp::SetChartType(parse_set_chart_type(obj, i)?)),
            "setChartAxisTitle" => out.push(DocxOp::SetChartAxisTitle(parse_set_chart_axis_title(
                obj, i,
            )?)),
            "setChartLegend" => out.push(DocxOp::SetChartLegend(parse_set_chart_legend(obj, i)?)),
            "setChartDataLabels" => out.push(DocxOp::SetChartDataLabels(
                parse_set_chart_data_labels(obj, i)?,
            )),
            "setChartSize" => out.push(DocxOp::SetChartSize(parse_set_chart_size(obj, i)?)),
            "setChartWrap" => out.push(DocxOp::SetChartWrap(parse_set_chart_wrap(obj, i)?)),
            "insertSmartArt" => out.push(DocxOp::InsertSmartArt(parse_insert_smartart(obj, i)?)),
            "deleteSmartArt" => out.push(DocxOp::DeleteSmartArt(parse_delete_smartart(obj, i)?)),
            "setSmartArtText" => {
                out.push(DocxOp::SetSmartArtText(parse_set_smartart_text(obj, i)?))
            }
            "addSmartArtPoint" => {
                out.push(DocxOp::AddSmartArtPoint(parse_add_smartart_point(obj, i)?))
            }
            "deleteSmartArtPoint" => out.push(DocxOp::DeleteSmartArtPoint(
                parse_delete_smartart_point(obj, i)?,
            )),
            "setSmartArtSize" => {
                out.push(DocxOp::SetSmartArtSize(parse_set_smartart_size(obj, i)?))
            }
            "setSmartArtWrap" => {
                out.push(DocxOp::SetSmartArtWrap(parse_set_smartart_wrap(obj, i)?))
            }
            "setSmartArtColors" => out.push(DocxOp::SetSmartArtColors(parse_set_smartart_colors(
                obj, i,
            )?)),
            "setSmartArtStyle" => {
                out.push(DocxOp::SetSmartArtStyle(parse_set_smartart_style(obj, i)?))
            }
            "setSmartArtLayout" => out.push(DocxOp::SetSmartArtLayout(parse_set_smartart_layout(
                obj, i,
            )?)),
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
                ErrorKind::MissingField { field: "id".into() },
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
        if !matches!(
            k.as_str(),
            "op" | "id" | "headerDistance" | "footerDistance"
        ) {
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
                ErrorKind::MissingField { field: "id".into() },
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

fn parse_set_hf_link(obj: &serde_json::Map<String, Value>, i: usize) -> Result<SetHfLinkOp, Error> {
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
                ErrorKind::MissingField { field: "id".into() },
                format!("ops[{i}].id"),
            )
        })?;
    let n = parse_section_id(id, i)?;
    let kind = match obj.get("kind") {
        Some(Value::String(s)) => HfKind::parse(s)
            .ok_or_else(|| Error::with_path(ErrorKind::InvalidHfKind, format!("ops[{i}].kind")))?,
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
        Some(Value::String(s)) => HfType::parse(s)
            .ok_or_else(|| Error::with_path(ErrorKind::InvalidHfType, format!("ops[{i}].type")))?,
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

fn parse_insert_hf(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertHeaderFooterOp, Error> {
    for k in obj.keys() {
        if !matches!(k.as_str(), "op" | "id" | "kind" | "type") {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("ops[{i}].{k}"),
            ));
        }
    }
    let id = require_op_id(obj, i)?;
    let n = parse_section_id(&id, i)?;
    let kind = parse_hf_kind(obj, i, "insertHeaderFooter")?;
    let hf_type = parse_hf_type(obj, i, "insertHeaderFooter")?;
    Ok(InsertHeaderFooterOp {
        id,
        n,
        kind,
        hf_type,
    })
}

fn parse_set_title_page(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetTitlePageOp, Error> {
    for k in obj.keys() {
        if !matches!(k.as_str(), "op" | "id" | "titlePage") {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("ops[{i}].{k}"),
            ));
        }
    }
    let id = require_op_id(obj, i)?;
    let n = parse_section_id(&id, i)?;
    let title_page = parse_required_bool(obj.get("titlePage"), i, "setTitlePage", "titlePage")?;
    Ok(SetTitlePageOp { id, n, title_page })
}

fn parse_set_even_and_odd(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetEvenAndOddHeadersOp, Error> {
    for k in obj.keys() {
        if !matches!(k.as_str(), "op" | "id" | "evenAndOdd") {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("ops[{i}].{k}"),
            ));
        }
    }
    let id = require_op_id(obj, i)?;
    if id != "docx:document" {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::EvenOddNeedsDocument,
            },
            format!("ops[{i}].id"),
        ));
    }
    let even_and_odd = parse_required_bool(
        obj.get("evenAndOdd"),
        i,
        "setEvenAndOddHeaders",
        "evenAndOdd",
    )?;
    Ok(SetEvenAndOddHeadersOp { id, even_and_odd })
}

fn require_document_id(id: &str, i: usize, reason: IdReason) -> Result<(), Error> {
    if id == "docx:document" {
        return Ok(());
    }
    Err(Error::with_path(
        ErrorKind::InvalidId { reason },
        format!("ops[{i}].id"),
    ))
}

fn parse_enabled_false(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    other: bool,
) -> Result<Option<bool>, Error> {
    match obj.get("enabled") {
        None => Ok(None),
        Some(Value::Bool(false)) => {
            if other {
                return Err(Error::with_path(
                    ErrorKind::MutuallyExclusive {
                        a: "enabled".into(),
                        b: "other fields".into(),
                    },
                    format!("ops[{i}].enabled"),
                ));
            }
            Ok(Some(false))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::InvalidSectValue {
                field: "enabled".into(),
            },
            format!("ops[{i}].enabled"),
        )),
    }
}

fn parse_optional_bool_field(
    v: Option<&Value>,
    i: usize,
    field: &str,
) -> Result<Option<bool>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Bool(b)) => Ok(Some(*b)),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Boolean,
            },
            format!("ops[{i}].{field}"),
        )),
    }
}

fn parse_signed_twip_field(v: Option<&Value>, i: usize, field: &str) -> Result<Option<i64>, Error> {
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

fn parse_enum_field(
    v: Option<&Value>,
    i: usize,
    field: &str,
    allowed: &[&str],
) -> Result<Option<String>, Error> {
    match v {
        None => Ok(None),
        Some(Value::String(s)) if allowed.contains(&s.as_str()) => Ok(Some(s.clone())),
        Some(Value::String(_)) => Err(Error::with_path(
            ErrorKind::InvalidSectValue {
                field: field.into(),
            },
            format!("ops[{i}].{field}"),
        )),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].{field}"),
        )),
    }
}

fn parse_set_page_margin(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetPageMarginOp, Error> {
    deny_keys(
        obj,
        i,
        &["op", "id", "top", "bottom", "left", "right", "gutter"],
    )?;
    let id = require_op_id(obj, i)?;
    let n = parse_section_id(&id, i)?;
    let top = parse_signed_twip_field(obj.get("top"), i, "top")?;
    let bottom = parse_signed_twip_field(obj.get("bottom"), i, "bottom")?;
    let left = parse_twip_field(obj.get("left"), i, "left")?;
    let right = parse_twip_field(obj.get("right"), i, "right")?;
    let gutter = parse_twip_field(obj.get("gutter"), i, "gutter")?;
    if top.is_none() && bottom.is_none() && left.is_none() && right.is_none() && gutter.is_none() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setPageMargin".into(),
                field: "top, bottom, left, right, or gutter".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(SetPageMarginOp {
        id,
        n,
        top,
        bottom,
        left,
        right,
        gutter,
    })
}

fn parse_set_page_size(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetPageSizeOp, Error> {
    deny_keys(obj, i, &["op", "id", "pageWidth", "pageHeight", "orient"])?;
    let id = require_op_id(obj, i)?;
    let n = parse_section_id(&id, i)?;
    let page_width = parse_twip_field(obj.get("pageWidth"), i, "pageWidth")?;
    let page_height = parse_twip_field(obj.get("pageHeight"), i, "pageHeight")?;
    let orient = parse_enum_field(obj.get("orient"), i, "orient", &["portrait", "landscape"])?;
    if page_width.is_none() && page_height.is_none() && orient.is_none() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setPageSize".into(),
                field: "pageWidth, pageHeight, or orient".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(SetPageSizeOp {
        id,
        n,
        page_width,
        page_height,
        orient,
    })
}

fn parse_col_items(v: Option<&Value>, i: usize) -> Result<Option<Vec<ColWidth>>, Error> {
    let Some(v) = v else {
        return Ok(None);
    };
    let Value::Array(arr) = v else {
        return Err(Error::with_path(
            ErrorKind::WrongType {
                field: "cols".into(),
                expected: TypeName::Array,
            },
            format!("ops[{i}].cols"),
        ));
    };
    let mut out = Vec::new();
    for (j, item) in arr.iter().enumerate() {
        let Value::Object(m) = item else {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "cols".into(),
                    expected: TypeName::Object,
                },
                format!("ops[{i}].cols[{j}]"),
            ));
        };
        for k in m.keys() {
            if !matches!(k.as_str(), "width" | "space") {
                return Err(Error::with_path(
                    ErrorKind::UnknownField { field: k.clone() },
                    format!("ops[{i}].cols[{j}].{k}"),
                ));
            }
        }
        out.push(ColWidth {
            width: parse_twip_field(m.get("width"), i, "width")?,
            space: parse_twip_field(m.get("space"), i, "space")?,
        });
    }
    Ok(Some(out))
}

fn parse_set_columns(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetColumnsOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op",
            "id",
            "num",
            "space",
            "equalWidth",
            "sep",
            "cols",
            "enabled",
        ],
    )?;
    let id = require_op_id(obj, i)?;
    let n = parse_section_id(&id, i)?;
    let num = parse_twip_field(obj.get("num"), i, "num")?;
    if let Some(0) = num {
        return Err(Error::with_path(
            ErrorKind::MinValue {
                field: "num".into(),
                min: 1,
            },
            format!("ops[{i}].num"),
        ));
    }
    let space = parse_twip_field(obj.get("space"), i, "space")?;
    let equal_width = parse_optional_bool_field(obj.get("equalWidth"), i, "equalWidth")?;
    let sep = parse_optional_bool_field(obj.get("sep"), i, "sep")?;
    let cols = parse_col_items(obj.get("cols"), i)?;
    if equal_width == Some(true) && cols.is_some() {
        return Err(Error::with_path(
            ErrorKind::MutuallyExclusive {
                a: "equalWidth".into(),
                b: "cols".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    let other = num.is_some()
        || space.is_some()
        || equal_width.is_some()
        || sep.is_some()
        || cols.is_some();
    let enabled = parse_enabled_false(obj, i, other)?;
    if enabled.is_none() && !other {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setColumns".into(),
                field: "num, space, equalWidth, sep, cols, or enabled".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(SetColumnsOp {
        id,
        n,
        num,
        space,
        equal_width,
        sep,
        cols,
        enabled,
    })
}

fn parse_set_vertical_align(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetVerticalAlignOp, Error> {
    deny_keys(obj, i, &["op", "id", "vAlign"])?;
    let id = require_op_id(obj, i)?;
    let n = parse_section_id(&id, i)?;
    let v_align = match obj.get("vAlign") {
        None => {
            return Err(Error::with_path(
                ErrorKind::OpRequiresField {
                    op: "setVerticalAlign".into(),
                    field: "vAlign".into(),
                },
                format!("ops[{i}].vAlign"),
            ));
        }
        Some(Value::Null) => None,
        Some(Value::String(s)) if matches!(s.as_str(), "top" | "center" | "both" | "bottom") => {
            Some(s.clone())
        }
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidSectValue {
                    field: "vAlign".into(),
                },
                format!("ops[{i}].vAlign"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "vAlign".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].vAlign"),
            ));
        }
    };
    Ok(SetVerticalAlignOp { id, n, v_align })
}

fn parse_set_line_numbering(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetLineNumberingOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op", "id", "countBy", "start", "distance", "restart", "enabled",
        ],
    )?;
    let id = require_op_id(obj, i)?;
    let n = parse_section_id(&id, i)?;
    let count_by = parse_twip_field(obj.get("countBy"), i, "countBy")?;
    let start = parse_twip_field(obj.get("start"), i, "start")?;
    let distance = parse_twip_field(obj.get("distance"), i, "distance")?;
    let restart = parse_enum_field(
        obj.get("restart"),
        i,
        "restart",
        &["newPage", "newSection", "continuous"],
    )?;
    let other = count_by.is_some() || start.is_some() || distance.is_some() || restart.is_some();
    let enabled = parse_enabled_false(obj, i, other)?;
    if enabled.is_none() && !other {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setLineNumbering".into(),
                field: "countBy, start, distance, restart, or enabled".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(SetLineNumberingOp {
        id,
        n,
        count_by,
        start,
        distance,
        restart,
        enabled,
    })
}

fn parse_set_doc_grid(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetDocGridOp, Error> {
    deny_keys(
        obj,
        i,
        &["op", "id", "type", "linePitch", "charSpace", "enabled"],
    )?;
    let id = require_op_id(obj, i)?;
    let n = parse_section_id(&id, i)?;
    let grid_type = parse_enum_field(
        obj.get("type"),
        i,
        "type",
        &["default", "lines", "linesAndChars", "snapToChars"],
    )?;
    let line_pitch = parse_i64_field(obj.get("linePitch"), i, "linePitch")?;
    let char_space = parse_i64_field(obj.get("charSpace"), i, "charSpace")?;
    let other = grid_type.is_some() || line_pitch.is_some() || char_space.is_some();
    let enabled = parse_enabled_false(obj, i, other)?;
    if enabled.is_none() && !other {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setDocGrid".into(),
                field: "type, linePitch, charSpace, or enabled".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(SetDocGridOp {
        id,
        n,
        grid_type,
        line_pitch,
        char_space,
        enabled,
    })
}

fn parse_set_rtl_gutter(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetRtlGutterOp, Error> {
    deny_keys(obj, i, &["op", "id", "rtlGutter"])?;
    let id = require_op_id(obj, i)?;
    let n = parse_section_id(&id, i)?;
    let rtl_gutter = parse_required_bool(obj.get("rtlGutter"), i, "setRtlGutter", "rtlGutter")?;
    Ok(SetRtlGutterOp { id, n, rtl_gutter })
}

fn parse_set_mirror_margins(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetMirrorMarginsOp, Error> {
    deny_keys(obj, i, &["op", "id", "mirrorMargins"])?;
    let id = require_op_id(obj, i)?;
    require_document_id(&id, i, IdReason::SettingsNeedDocument)?;
    let mirror_margins = parse_required_bool(
        obj.get("mirrorMargins"),
        i,
        "setMirrorMargins",
        "mirrorMargins",
    )?;
    Ok(SetMirrorMarginsOp { id, mirror_margins })
}

fn parse_set_gutter_at_top(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetGutterAtTopOp, Error> {
    deny_keys(obj, i, &["op", "id", "gutterAtTop"])?;
    let id = require_op_id(obj, i)?;
    require_document_id(&id, i, IdReason::SettingsNeedDocument)?;
    let gutter_at_top =
        parse_required_bool(obj.get("gutterAtTop"), i, "setGutterAtTop", "gutterAtTop")?;
    Ok(SetGutterAtTopOp { id, gutter_at_top })
}

fn parse_insert_table(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertTableOp, Error> {
    deny_keys(obj, i, &["op", "id", "rows", "cols", "cells"])?;
    let id = require_op_id(obj, i)?;
    let anchor = tbl::parse_insert_anchor(&id, i)?;
    let rows = tbl::parse_dim(obj.get("rows"), i, "rows", tbl::max_rows())?;
    let cols = tbl::parse_dim(obj.get("cols"), i, "cols", tbl::max_cols())?;
    let cells = tbl::parse_cells_field(obj.get("cells"), i, rows, cols)?;
    Ok(InsertTableOp { id, anchor, cells })
}

fn parse_delete_table(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteTableOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_op_id(obj, i)?;
    let target = tbl::parse_table_only(&id, i, IdReason::DeleteTableNeedsTable)?;
    Ok(DeleteTableOp { id, target })
}

fn parse_insert_row(obj: &serde_json::Map<String, Value>, i: usize) -> Result<InsertRowOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_op_id(obj, i)?;
    let target = tbl::parse_table_or_row(&id, i)?;
    Ok(InsertRowOp { id, target })
}

fn parse_delete_row(obj: &serde_json::Map<String, Value>, i: usize) -> Result<DeleteRowOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_op_id(obj, i)?;
    let target = tbl::parse_row_ref(&id, i)?;
    Ok(DeleteRowOp { id, target })
}

fn parse_insert_col(obj: &serde_json::Map<String, Value>, i: usize) -> Result<InsertColOp, Error> {
    deny_keys(obj, i, &["op", "id", "after"])?;
    let id = require_op_id(obj, i)?;
    let target = tbl::parse_table_only(&id, i, IdReason::NeedTableId)?;
    let after = parse_col_after(obj.get("after"), i)?;
    Ok(InsertColOp { id, target, after })
}

fn parse_delete_col(obj: &serde_json::Map<String, Value>, i: usize) -> Result<DeleteColOp, Error> {
    deny_keys(obj, i, &["op", "id", "col"])?;
    let id = require_op_id(obj, i)?;
    let target = tbl::parse_table_only(&id, i, IdReason::NeedTableId)?;
    let col = tbl::parse_dim(obj.get("col"), i, "col", tbl::max_cols())? as u32;
    Ok(DeleteColOp { id, target, col })
}

fn parse_merge_cells(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<MergeCellsOp, Error> {
    deny_keys(
        obj,
        i,
        &["op", "id", "startRow", "startCol", "endRow", "endCol"],
    )?;
    let id = require_op_id(obj, i)?;
    let target = tbl::parse_table_only(&id, i, IdReason::NeedTableId)?;
    let start_row = tbl::parse_dim(obj.get("startRow"), i, "startRow", tbl::max_rows())?;
    let start_col = tbl::parse_dim(obj.get("startCol"), i, "startCol", tbl::max_cols())?;
    let end_row = tbl::parse_dim(obj.get("endRow"), i, "endRow", tbl::max_rows())?;
    let end_col = tbl::parse_dim(obj.get("endCol"), i, "endCol", tbl::max_cols())?;
    Ok(MergeCellsOp {
        id,
        target,
        start_row,
        start_col,
        end_row,
        end_col,
    })
}

fn parse_split_cells(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SplitCellsOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_op_id(obj, i)?;
    let target = tbl::parse_cell_ref(&id, i)?;
    Ok(SplitCellsOp { id, target })
}

fn parse_revision_op(obj: &serde_json::Map<String, Value>, i: usize) -> Result<RevisionOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_op_id(obj, i)?;
    let target = super::revision::parse_rev_ref(&id, &format!("ops[{i}].id"))?;
    Ok(RevisionOp { id, target })
}

fn parse_col_after(v: Option<&Value>, i: usize) -> Result<u32, Error> {
    let path = format!("ops[{i}].after");
    match v {
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
            Ok(u as u32)
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "after".into(),
                expected: TypeName::Integer,
            },
            path,
        )),
        None => Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "insertCol".into(),
                field: "after".into(),
            },
            path,
        )),
    }
}

fn require_op_id(obj: &serde_json::Map<String, Value>, i: usize) -> Result<String, Error> {
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

fn parse_hf_kind(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    op: &str,
) -> Result<HfKind, Error> {
    match obj.get("kind") {
        Some(Value::String(s)) => HfKind::parse(s)
            .ok_or_else(|| Error::with_path(ErrorKind::InvalidHfKind, format!("ops[{i}].kind"))),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "kind".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].kind"),
        )),
        None => Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: op.into(),
                field: "kind".into(),
            },
            format!("ops[{i}].kind"),
        )),
    }
}

fn parse_hf_type(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    op: &str,
) -> Result<HfType, Error> {
    match obj.get("type") {
        Some(Value::String(s)) => HfType::parse(s)
            .ok_or_else(|| Error::with_path(ErrorKind::InvalidHfType, format!("ops[{i}].type"))),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "type".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].type"),
        )),
        None => Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: op.into(),
                field: "type".into(),
            },
            format!("ops[{i}].type"),
        )),
    }
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

fn parse_twip_field(v: Option<&Value>, i: usize, field: &str) -> Result<Option<u32>, Error> {
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
                ErrorKind::MissingField { field: "id".into() },
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

fn deny_keys(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    allowed: &[&str],
) -> Result<(), Error> {
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
    Ok(SetPictureSizeOp { id, target, cx, cy })
}

fn parse_set_pic_wrap(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetPictureWrapOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op", "id", "kind", "wrap", "alignH", "alignV", "offsetX", "offsetY",
        ],
    )?;
    let id = require_id(obj, i)?;
    let target = picture::parse_pic_id(&id, i)?;
    let kind = match obj.get("kind") {
        Some(Value::String(s)) => picture::parse_pic_kind(s)
            .ok_or_else(|| Error::with_path(ErrorKind::InvalidPicKind, format!("ops[{i}].kind")))?,
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
        Some(Value::String(s)) => picture::parse_align_h(s).map(Some).ok_or_else(|| {
            Error::with_path(ErrorKind::InvalidPicAlign, format!("ops[{i}].alignH"))
        }),
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
        Some(Value::String(s)) => picture::parse_align_v(s).map(Some).ok_or_else(|| {
            Error::with_path(ErrorKind::InvalidPicAlign, format!("ops[{i}].alignV"))
        }),
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
    Ok(SetTextBoxSizeOp { id, target, cx, cy })
}

fn parse_set_txbx_wrap(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetTextBoxWrapOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op", "id", "kind", "wrap", "alignH", "alignV", "offsetX", "offsetY",
        ],
    )?;
    let id = require_id(obj, i)?;
    let target = para::parse_txbx_id(&id, i)?;
    let kind = match obj.get("kind") {
        Some(Value::String(s)) => picture::parse_pic_kind(s)
            .ok_or_else(|| Error::with_path(ErrorKind::InvalidPicKind, format!("ops[{i}].kind")))?,
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
    let text = parse_optional_text(obj, i)?;
    if let Some(note) = para::parse_note_id_at(&id, &path)? {
        let place = match note {
            NoteIdTarget::Sep => {
                return Err(Error::with_path(
                    ErrorKind::InvalidId {
                        reason: crate::error::IdReason::NoteSep,
                    },
                    path,
                ));
            }
            NoteIdTarget::Container { kind, note_id } => InsertParaPlace::Note {
                kind,
                note_id,
                after: None,
            },
            NoteIdTarget::Para {
                kind,
                note_id,
                para,
            } => InsertParaPlace::Note {
                kind,
                note_id,
                after: Some(para),
            },
        };
        return Ok(InsertParaOp { id, place, text });
    }
    if let Some(comment) = para::parse_comment_id_at(&id, &path)? {
        let place = match comment {
            CommentIdTarget::Container { comment_id } => InsertParaPlace::Comment {
                comment_id,
                after: None,
            },
            CommentIdTarget::Para { comment_id, para } => InsertParaPlace::Comment {
                comment_id,
                after: Some(para),
            },
        };
        return Ok(InsertParaOp { id, place, text });
    }
    if id == "docx:document" {
        return Ok(InsertParaOp {
            id,
            place: InsertParaPlace::Body { after: None },
            text,
        });
    }
    if let Some((kind, hf_type, n)) = para::parse_hf_container_id(&id) {
        return Ok(InsertParaOp {
            id,
            place: InsertParaPlace::HeaderFooter {
                kind,
                hf_type,
                n,
                after: None,
            },
            text,
        });
    }
    if let Ok(box_target) = para::parse_txbx_id_at(&id, &path) {
        return Ok(InsertParaOp {
            id,
            place: InsertParaPlace::TextBox {
                box_target,
                after: None,
            },
            text,
        });
    }
    match para::parse_para_id_at(&id, &path)? {
        ParaTarget::InTextBox { host, key, para } => Ok(InsertParaOp {
            id,
            place: InsertParaPlace::TextBox {
                box_target: TxbxTarget {
                    host: host.clone(),
                    key: key.clone(),
                },
                after: Some(ParaTarget::InTextBox { host, key, para }),
            },
            text,
        }),
        ParaTarget::Body(after) => Ok(InsertParaOp {
            id,
            place: InsertParaPlace::Body { after: Some(after) },
            text,
        }),
        ParaTarget::HeaderFooter {
            kind,
            hf_type,
            n,
            para,
        } => Ok(InsertParaOp {
            id,
            place: InsertParaPlace::HeaderFooter {
                kind,
                hf_type,
                n,
                after: Some(para),
            },
            text,
        }),
        ParaTarget::InNote { .. } | ParaTarget::InComment { .. } => Err(Error::with_path(
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
    if let Some(note) = para::parse_note_id_at(&id, &path)? {
        return match note {
            NoteIdTarget::Sep => Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: crate::error::IdReason::NoteSep,
                },
                path,
            )),
            NoteIdTarget::Container { .. } => Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: crate::error::IdReason::InsertParaNeedsBox,
                },
                path,
            )),
            NoteIdTarget::Para {
                kind,
                note_id,
                para,
            } => Ok(DeleteParaOp {
                id,
                inner: ParaTarget::InNote {
                    kind,
                    note_id,
                    para,
                },
            }),
        };
    }
    if let Some(comment) = para::parse_comment_id_at(&id, &path)? {
        return match comment {
            CommentIdTarget::Container { .. } => Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: crate::error::IdReason::InsertParaNeedsBox,
                },
                path,
            )),
            CommentIdTarget::Para { comment_id, para } => Ok(DeleteParaOp {
                id,
                inner: ParaTarget::InComment { comment_id, para },
            }),
        };
    }
    match para::parse_para_id_at(&id, &path) {
        Ok(inner @ ParaTarget::InTextBox { .. })
        | Ok(inner @ ParaTarget::Body(_))
        | Ok(inner @ ParaTarget::HeaderFooter { .. }) => Ok(DeleteParaOp { id, inner }),
        Ok(_) => Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: crate::error::IdReason::InsertParaNeedsBox,
            },
            path,
        )),
        Err(e) => Err(e),
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

fn parse_optional_text(obj: &serde_json::Map<String, Value>, i: usize) -> Result<String, Error> {
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

fn parse_insert_note(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    _kind: NoteKind,
) -> Result<InsertNoteOp, Error> {
    deny_keys(obj, i, &["op", "id", "text"])?;
    let id = require_id(obj, i)?;
    let path = format!("ops[{i}].id");
    if para::parse_note_id_at(&id, &path)?.is_some() {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: crate::error::IdReason::NotePrefix,
            },
            path,
        ));
    }
    let host = para::parse_para_id_at(&id, &path)?;
    if matches!(
        host,
        ParaTarget::InNote { .. } | ParaTarget::InComment { .. }
    ) {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: crate::error::IdReason::NotePrefix,
            },
            path,
        ));
    }
    Ok(InsertNoteOp {
        id,
        host,
        text: parse_optional_text(obj, i)?,
    })
}

fn parse_delete_note(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    kind: NoteKind,
) -> Result<DeleteNoteOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let path = format!("ops[{i}].id");
    let Some(note) = para::parse_note_id_at(&id, &path)? else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: crate::error::IdReason::NotePrefix,
            },
            path,
        ));
    };
    match note {
        NoteIdTarget::Sep => Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: crate::error::IdReason::NoteSep,
            },
            path,
        )),
        NoteIdTarget::Para { .. } => Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: crate::error::IdReason::NotePrefix,
            },
            path,
        )),
        NoteIdTarget::Container {
            kind: parsed,
            note_id,
        } => {
            if parsed != kind {
                return Err(Error::with_path(
                    ErrorKind::InvalidId {
                        reason: crate::error::IdReason::NotePrefix,
                    },
                    path,
                ));
            }
            Ok(DeleteNoteOp { id, kind, note_id })
        }
    }
}

fn parse_set_note_numbering(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetNoteNumberingOp, Error> {
    deny_keys(
        obj,
        i,
        &["op", "id", "kind", "numFmt", "numStart", "numRestart"],
    )?;
    let id = require_id(obj, i)?;
    let target = if id == "docx:document" {
        NoteNumberTarget::Document
    } else {
        NoteNumberTarget::Section(parse_section_id(&id, i)?)
    };
    let kind_s = obj.get("kind").and_then(Value::as_str).ok_or_else(|| {
        Error::with_path(
            ErrorKind::MissingField {
                field: "kind".into(),
            },
            format!("ops[{i}].kind"),
        )
    })?;
    let kind = NoteKind::parse(kind_s)
        .ok_or_else(|| Error::with_path(ErrorKind::InvalidNoteKind, format!("ops[{i}].kind")))?;
    let num_fmt = match obj.get("numFmt") {
        None => None,
        Some(Value::String(s)) => {
            if !matches!(
                s.as_str(),
                "decimal" | "upperRoman" | "lowerRoman" | "upperLetter" | "lowerLetter" | "chicago"
            ) {
                return Err(Error::with_path(
                    ErrorKind::InvalidNoteNumFmt,
                    format!("ops[{i}].numFmt"),
                ));
            }
            Some(s.clone())
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "numFmt".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].numFmt"),
            ));
        }
    };
    let num_start = match obj.get("numStart") {
        None => None,
        Some(Value::Number(n)) => {
            let u = n.as_u64().ok_or_else(|| {
                Error::with_path(
                    ErrorKind::WrongType {
                        field: "numStart".into(),
                        expected: TypeName::Integer,
                    },
                    format!("ops[{i}].numStart"),
                )
            })?;
            if u < 1 {
                return Err(Error::with_path(
                    ErrorKind::MinValue {
                        field: "numStart".into(),
                        min: 1,
                    },
                    format!("ops[{i}].numStart"),
                ));
            }
            if u > u32::MAX as u64 {
                return Err(Error::with_path(
                    ErrorKind::MinValue {
                        field: "numStart".into(),
                        min: 1,
                    },
                    format!("ops[{i}].numStart"),
                ));
            }
            Some(u as u32)
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "numStart".into(),
                    expected: TypeName::Integer,
                },
                format!("ops[{i}].numStart"),
            ));
        }
    };
    let num_restart = match obj.get("numRestart") {
        None => None,
        Some(Value::String(s)) => {
            if !matches!(s.as_str(), "continuous" | "eachSect" | "eachPage") {
                return Err(Error::with_path(
                    ErrorKind::InvalidNoteRestart,
                    format!("ops[{i}].numRestart"),
                ));
            }
            Some(s.clone())
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "numRestart".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].numRestart"),
            ));
        }
    };
    if num_fmt.is_none() && num_start.is_none() && num_restart.is_none() {
        return Err(Error::with_path(
            ErrorKind::NeedNoteNumberingField,
            format!("ops[{i}]"),
        ));
    }
    Ok(SetNoteNumberingOp {
        id,
        target,
        kind,
        num_fmt,
        num_start,
        num_restart,
    })
}

fn parse_comment_anchor(id: &str, path: &str) -> Result<ParaTarget, Error> {
    if para::parse_comment_id_at(id, path)?.is_some() {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: crate::error::IdReason::CommentPrefix,
            },
            path.to_string(),
        ));
    }
    para::parse_para_id_at(id, path)
}

fn parse_comment_container(id: &str, path: &str) -> Result<String, Error> {
    let Some(comment) = para::parse_comment_id_at(id, path)? else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: crate::error::IdReason::CommentPrefix,
            },
            path.to_string(),
        ));
    };
    match comment {
        CommentIdTarget::Container { comment_id } => Ok(comment_id),
        CommentIdTarget::Para { .. } => Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: crate::error::IdReason::CommentPrefix,
            },
            path.to_string(),
        )),
    }
}

fn parse_optional_attr(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    field: &str,
) -> Result<Option<String>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::String(s)) if !s.is_empty() => Ok(Some(s.clone())),
        Some(Value::String(_)) => Err(Error::with_path(
            ErrorKind::MissingField {
                field: field.into(),
            },
            format!("ops[{i}].{field}"),
        )),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].{field}"),
        )),
    }
}

fn parse_insert_comment(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertCommentOp, Error> {
    deny_keys(
        obj,
        i,
        &["op", "start", "end", "text", "author", "initials", "date"],
    )?;
    let start = match obj.get("start") {
        Some(Value::String(s)) if !s.is_empty() => s.clone(),
        Some(Value::String(_)) | None => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "start".into(),
                },
                format!("ops[{i}].start"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "start".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].start"),
            ));
        }
    };
    let end = match obj.get("end") {
        Some(Value::String(s)) if !s.is_empty() => s.clone(),
        Some(Value::String(_)) | None => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "end".into(),
                },
                format!("ops[{i}].end"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "end".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].end"),
            ));
        }
    };
    let start_target = parse_comment_anchor(&start, &format!("ops[{i}].start"))?;
    let end_target = parse_comment_anchor(&end, &format!("ops[{i}].end"))?;
    Ok(InsertCommentOp {
        start,
        end,
        start_target,
        end_target,
        text: parse_optional_text(obj, i)?,
        author: parse_optional_attr(obj, i, "author")?.unwrap_or_else(|| "ooxp".into()),
        initials: parse_optional_attr(obj, i, "initials")?.unwrap_or_else(|| "oo".into()),
        date: parse_optional_attr(obj, i, "date")?,
    })
}

fn parse_reply_comment(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<ReplyCommentOp, Error> {
    deny_keys(obj, i, &["op", "id", "text", "author", "initials", "date"])?;
    let id = require_id(obj, i)?;
    let comment_id = parse_comment_container(&id, &format!("ops[{i}].id"))?;
    Ok(ReplyCommentOp {
        id,
        comment_id,
        text: parse_optional_text(obj, i)?,
        author: parse_optional_attr(obj, i, "author")?.unwrap_or_else(|| "ooxp".into()),
        initials: parse_optional_attr(obj, i, "initials")?.unwrap_or_else(|| "oo".into()),
        date: parse_optional_attr(obj, i, "date")?,
    })
}

fn parse_delete_comment(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteCommentOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let comment_id = parse_comment_container(&id, &format!("ops[{i}].id"))?;
    Ok(DeleteCommentOp { id, comment_id })
}

fn parse_set_comment_text(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetCommentTextOp, Error> {
    deny_keys(obj, i, &["op", "id", "text"])?;
    let id = require_id(obj, i)?;
    let comment_id = parse_comment_container(&id, &format!("ops[{i}].id"))?;
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
    Ok(SetCommentTextOp {
        id,
        comment_id,
        text,
    })
}

fn parse_set_comment_meta(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetCommentMetaOp, Error> {
    deny_keys(obj, i, &["op", "id", "author", "initials", "date"])?;
    let id = require_id(obj, i)?;
    let comment_id = parse_comment_container(&id, &format!("ops[{i}].id"))?;
    let author = parse_optional_attr(obj, i, "author")?;
    let initials = parse_optional_attr(obj, i, "initials")?;
    let date = parse_optional_attr(obj, i, "date")?;
    if author.is_none() && initials.is_none() && date.is_none() {
        return Err(Error::with_path(
            ErrorKind::NeedCommentMetaField,
            format!("ops[{i}]"),
        ));
    }
    Ok(SetCommentMetaOp {
        id,
        comment_id,
        author,
        initials,
        date,
    })
}

fn parse_set_comment_done(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetCommentDoneOp, Error> {
    deny_keys(obj, i, &["op", "id", "done"])?;
    let id = require_id(obj, i)?;
    let comment_id = parse_comment_container(&id, &format!("ops[{i}].id"))?;
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
    Ok(SetCommentDoneOp {
        id,
        comment_id,
        done,
    })
}

fn parse_insert_chart(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<charts::InsertChartOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op", "id", "type", "title", "series", "kind", "wrap", "cx", "cy",
        ],
    )?;
    let id = require_id(obj, i)?;
    let para = charts::parse_insert_para_id(&id, i)?;
    let chart_type = parse_chart_type_field(obj.get("type"), i, true)?;
    let title = match obj.get("title") {
        None | Some(Value::Null) => None,
        Some(Value::String(s)) => Some(s.clone()),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "title".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].title"),
            ));
        }
    };
    let series = parse_insert_series(obj.get("series"), i, chart_type)?;
    if chart_type.needs_one_series() && series.len() != 1 {
        return Err(Error::with_path(
            ErrorKind::PieNeedsOneSeries,
            format!("ops[{i}].series"),
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
                format!("ops[{i}].series"),
            ));
        }
    } else if series.iter().any(|s| s.series_type.is_some()) {
        return Err(Error::with_path(
            ErrorKind::InvalidComboSeriesType,
            format!("ops[{i}].series"),
        ));
    }
    if chart_type != ChartType::Bubble && series.iter().any(|s| s.sizes.is_some()) {
        return Err(Error::with_path(
            ErrorKind::SizesOnNonBubble,
            format!("ops[{i}].series"),
        ));
    }
    let kind = match obj.get("kind") {
        None => PicKind::Anchor,
        Some(Value::String(s)) => picture::parse_pic_kind(s)
            .ok_or_else(|| Error::with_path(ErrorKind::InvalidPicKind, format!("ops[{i}].kind")))?,
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "kind".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].kind"),
            ));
        }
    };
    let wrap = match obj.get("wrap") {
        None | Some(Value::Null) => {
            if kind == PicKind::Anchor {
                Some(WrapKind::Square)
            } else {
                None
            }
        }
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
    Ok(charts::InsertChartOp {
        id,
        para,
        chart_type,
        title,
        series,
        kind,
        wrap,
        pos: WrapPos::default(),
        cx: cx.unwrap_or(charts::default_cx()),
        cy: cy.unwrap_or(charts::default_cy()),
    })
}

fn parse_insert_series(
    v: Option<&Value>,
    i: usize,
    _chart_type: ChartType,
) -> Result<Vec<NewSeries>, Error> {
    let arr = match v {
        Some(Value::Array(a)) if !a.is_empty() => a,
        Some(Value::Array(_)) | None => {
            return Err(Error::with_path(
                ErrorKind::OpRequiresField {
                    op: "insertChart".into(),
                    field: "series".into(),
                },
                format!("ops[{i}].series"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "series".into(),
                    expected: TypeName::Array,
                },
                format!("ops[{i}].series"),
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
                format!("ops[{i}].series[{j}]"),
            )
        })?;
        deny_keys(obj, i, &["name", "categories", "values", "sizes", "type"])?;
        let name = match obj.get("name") {
            None | Some(Value::Null) => None,
            Some(Value::String(s)) => Some(s.clone()),
            Some(_) => {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: format!("series[{j}].name"),
                        expected: TypeName::String,
                    },
                    format!("ops[{i}].series[{j}].name"),
                ));
            }
        };
        let categories =
            parse_datum_list(obj.get("categories"), i, &format!("series[{j}].categories"))?;
        let values = parse_datum_list(obj.get("values"), i, &format!("series[{j}].values"))?
            .filter(|v| !v.is_empty())
            .ok_or_else(|| {
                Error::with_path(
                    ErrorKind::OpRequiresField {
                        op: "insertChart".into(),
                        field: "series[].values".into(),
                    },
                    format!("ops[{i}].series[{j}].values"),
                )
            })?;
        let sizes = parse_datum_list(obj.get("sizes"), i, &format!("series[{j}].sizes"))?;
        let series_type =
            parse_optional_combo_series_type(obj.get("type"), i, &format!("series[{j}].type"))?;
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

fn parse_delete_chart(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<charts::DeleteChartOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let target = charts::parse_chart_id(&id, i)?;
    Ok(charts::DeleteChartOp { id, target })
}

fn parse_set_chart_title(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<charts::SetChartTitleOp, Error> {
    deny_keys(obj, i, &["op", "id", "title"])?;
    let id = require_id(obj, i)?;
    let target = charts::parse_chart_id(&id, i)?;
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
    Ok(charts::SetChartTitleOp { id, target, title })
}

fn parse_set_chart_series(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<charts::SetChartSeriesOp, Error> {
    deny_keys(
        obj,
        i,
        &["op", "id", "name", "categories", "values", "sizes"],
    )?;
    let id = require_id(obj, i)?;
    let target = charts::parse_series_id(&id, i)?;
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
    let categories = parse_datum_list(obj.get("categories"), i, "categories")?;
    let values = parse_datum_list(obj.get("values"), i, "values")?;
    let sizes = parse_datum_list(obj.get("sizes"), i, "sizes")?;
    if name.is_none() && categories.is_none() && values.is_none() && sizes.is_none() {
        return Err(Error::with_path(
            ErrorKind::NeedChartSeriesField,
            format!("ops[{i}]"),
        ));
    }
    Ok(charts::SetChartSeriesOp {
        id,
        target,
        name,
        categories,
        values,
        sizes,
    })
}

fn parse_add_chart_series(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<charts::AddChartSeriesOp, Error> {
    deny_keys(
        obj,
        i,
        &["op", "id", "name", "categories", "values", "sizes", "type"],
    )?;
    let id = require_id(obj, i)?;
    let target = charts::parse_chart_id(&id, i)?;
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
    let categories = parse_datum_list(obj.get("categories"), i, "categories")?;
    let values = parse_datum_list(obj.get("values"), i, "values")?
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
    let sizes = parse_datum_list(obj.get("sizes"), i, "sizes")?;
    let series_type = parse_optional_combo_series_type(obj.get("type"), i, "type")?;
    Ok(charts::AddChartSeriesOp {
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
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<charts::DeleteChartSeriesOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let target = charts::parse_series_id(&id, i)?;
    Ok(charts::DeleteChartSeriesOp { id, target })
}

fn parse_set_chart_type(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<charts::SetChartTypeOp, Error> {
    deny_keys(obj, i, &["op", "id", "type"])?;
    let id = require_id(obj, i)?;
    let target = charts::parse_chart_id(&id, i)?;
    let chart_type = parse_chart_type_field(obj.get("type"), i, true)?;
    if chart_type == ChartType::Combo {
        return Err(Error::with_path(
            ErrorKind::InvalidChartType,
            format!("ops[{i}].type"),
        ));
    }
    Ok(charts::SetChartTypeOp {
        id,
        target,
        chart_type,
    })
}

fn parse_set_chart_axis_title(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<charts::SetChartAxisTitleOp, Error> {
    deny_keys(obj, i, &["op", "id", "axis", "title"])?;
    let id = require_id(obj, i)?;
    let target = charts::parse_chart_id(&id, i)?;
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
    Ok(charts::SetChartAxisTitleOp {
        id,
        target,
        axis,
        title,
    })
}

fn parse_set_chart_legend(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<charts::SetChartLegendOp, Error> {
    deny_keys(obj, i, &["op", "id", "show", "pos"])?;
    let id = require_id(obj, i)?;
    let target = charts::parse_chart_id(&id, i)?;
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
    Ok(charts::SetChartLegendOp {
        id,
        target,
        show,
        pos,
    })
}

fn parse_set_chart_data_labels(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<charts::SetChartDataLabelsOp, Error> {
    deny_keys(obj, i, &["op", "id", "show"])?;
    let id = require_id(obj, i)?;
    let target = charts::parse_chart_id(&id, i)?;
    let show = parse_required_bool(obj.get("show"), i, "setChartDataLabels", "show")?;
    Ok(charts::SetChartDataLabelsOp { id, target, show })
}

fn parse_set_chart_size(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<charts::SetChartSizeOp, Error> {
    deny_keys(obj, i, &["op", "id", "cx", "cy"])?;
    let id = require_id(obj, i)?;
    let target = charts::parse_chart_id(&id, i)?;
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
    Ok(charts::SetChartSizeOp { id, target, cx, cy })
}

fn parse_set_chart_wrap(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<charts::SetChartWrapOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op", "id", "kind", "wrap", "alignH", "alignV", "offsetX", "offsetY",
        ],
    )?;
    let id = require_id(obj, i)?;
    let target = charts::parse_chart_id(&id, i)?;
    let kind = match obj.get("kind") {
        Some(Value::String(s)) => picture::parse_pic_kind(s)
            .ok_or_else(|| Error::with_path(ErrorKind::InvalidPicKind, format!("ops[{i}].kind")))?,
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
                    op: "setChartWrap".into(),
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
                op: "setChartWrap".into(),
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
    Ok(charts::SetChartWrapOp {
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

fn parse_optional_combo_series_type(
    v: Option<&Value>,
    i: usize,
    field: &str,
) -> Result<Option<ChartType>, Error> {
    match v {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => ChartType::parse_combo_series(s).map(Some).ok_or_else(|| {
            Error::with_path(
                ErrorKind::InvalidComboSeriesType,
                format!("ops[{i}].{field}"),
            )
        }),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].{field}"),
        )),
    }
}

fn parse_chart_type_field(v: Option<&Value>, i: usize, required: bool) -> Result<ChartType, Error> {
    match v {
        Some(Value::String(s)) => ChartType::parse(s)
            .ok_or_else(|| Error::with_path(ErrorKind::InvalidChartType, format!("ops[{i}].type"))),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "type".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].type"),
        )),
        None if required => Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "insertChart".into(),
                field: "type".into(),
            },
            format!("ops[{i}].type"),
        )),
        None => Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setChartType".into(),
                field: "type".into(),
            },
            format!("ops[{i}].type"),
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

fn parse_datum_list(v: Option<&Value>, i: usize, field: &str) -> Result<Option<Vec<Value>>, Error> {
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
                                field: format!("{field}[{j}]"),
                                expected: TypeName::String,
                            },
                            format!("ops[{i}].{field}[{j}]"),
                        ));
                    }
                }
            }
            Ok(Some(out))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Array,
            },
            format!("ops[{i}].{field}"),
        )),
    }
}

fn parse_insert_smartart(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<super::smartart::InsertSmartArtOp, Error> {
    deny_keys(
        obj,
        i,
        &["op", "id", "layout", "points", "kind", "wrap", "cx", "cy"],
    )?;
    let id = require_id(obj, i)?;
    let para = super::smartart::parse_insert_para_id(&id, i)?;
    let spec = super::smartart::parse_spec(obj, &format!("ops[{i}]"))?;
    Ok(super::smartart::InsertSmartArtOp { id, para, spec })
}

fn parse_delete_smartart(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<super::smartart::DeleteSmartArtOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let target = super::smartart::parse_diagram_id(&id, i)?;
    Ok(super::smartart::DeleteSmartArtOp { id, target })
}

fn parse_set_smartart_text(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<super::smartart::SetSmartArtTextOp, Error> {
    deny_keys(obj, i, &["op", "id", "text"])?;
    let id = require_id(obj, i)?;
    let target = super::smartart::parse_point_id(&id, i)?;
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
    Ok(super::smartart::SetSmartArtTextOp { id, target, text })
}

fn parse_add_smartart_point(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<super::smartart::AddSmartArtPointOp, Error> {
    deny_keys(obj, i, &["op", "id", "text", "role"])?;
    let id = require_id(obj, i)?;
    let parent = super::smartart::parse_add_parent(&id, i)?;
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
    let role = super::smartart::parse_role_field(obj.get("role"), &format!("ops[{i}].role"))?;
    Ok(super::smartart::AddSmartArtPointOp {
        id,
        parent,
        text,
        role,
    })
}

fn parse_delete_smartart_point(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<super::smartart::DeleteSmartArtPointOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let target = super::smartart::parse_point_id(&id, i)?;
    Ok(super::smartart::DeleteSmartArtPointOp { id, target })
}

fn parse_set_smartart_size(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<super::smartart::SetSmartArtSizeOp, Error> {
    deny_keys(obj, i, &["op", "id", "cx", "cy"])?;
    let id = require_id(obj, i)?;
    let target = super::smartart::parse_diagram_id(&id, i)?;
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
    Ok(super::smartart::SetSmartArtSizeOp { id, target, cx, cy })
}

fn parse_set_smartart_wrap(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<super::smartart::SetSmartArtWrapOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op", "id", "kind", "wrap", "alignH", "alignV", "offsetX", "offsetY",
        ],
    )?;
    let id = require_id(obj, i)?;
    let target = super::smartart::parse_diagram_id(&id, i)?;
    let kind = match obj.get("kind") {
        Some(Value::String(s)) => picture::parse_pic_kind(s)
            .ok_or_else(|| Error::with_path(ErrorKind::InvalidPicKind, format!("ops[{i}].kind")))?,
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
                    op: "setSmartArtWrap".into(),
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
                op: "setSmartArtWrap".into(),
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
    Ok(super::smartart::SetSmartArtWrapOp {
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

fn parse_set_smartart_colors(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<super::smartart::SetSmartArtColorsOp, Error> {
    deny_keys(obj, i, &["op", "id", "colors"])?;
    let id = require_id(obj, i)?;
    let target = super::smartart::parse_diagram_id(&id, i)?;
    let colors =
        super::smartart::parse_colors_field(obj.get("colors"), &format!("ops[{i}].colors"))?;
    Ok(super::smartart::SetSmartArtColorsOp { id, target, colors })
}

fn parse_set_smartart_style(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<super::smartart::SetSmartArtStyleOp, Error> {
    deny_keys(obj, i, &["op", "id", "style"])?;
    let id = require_id(obj, i)?;
    let target = super::smartart::parse_diagram_id(&id, i)?;
    let style = super::smartart::parse_style_field(obj.get("style"), &format!("ops[{i}].style"))?;
    Ok(super::smartart::SetSmartArtStyleOp { id, target, style })
}

fn parse_set_smartart_layout(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<super::smartart::SetSmartArtLayoutOp, Error> {
    deny_keys(obj, i, &["op", "id", "layout"])?;
    let id = require_id(obj, i)?;
    let target = super::smartart::parse_diagram_id(&id, i)?;
    let layout =
        super::smartart::parse_layout_field(obj.get("layout"), &format!("ops[{i}].layout"))?;
    Ok(super::smartart::SetSmartArtLayoutOp { id, target, layout })
}
