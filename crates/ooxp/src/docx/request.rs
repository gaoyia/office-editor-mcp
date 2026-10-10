//! docx 自己的 scope 与 ops 解析。

use std::collections::HashSet;

use serde_json::Value;

use crate::error::{Error, ErrorKind, IdReason, ScopeNeed, TypeName};
use crate::json;

use super::charts::{self, ChartType, NewSeries};
use super::hf::{HfKind, HfType};
use super::math_expr::MathBody;
use super::para::{
    self, BodyPara, CommentIdTarget, HostPara, NoteIdTarget, NoteKind, ParaTarget, TxbxTarget,
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
    /// 为 true 时列出连接线。
    pub connectors: bool,
    /// 为 true 时列出无框形状。
    pub shapes: bool,
    /// 为 true 时列出墨迹画布。
    pub ink_canvases: bool,
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
    /// 为 true 时列出嵌入对象。
    pub ole_objects: bool,
    /// 为 true 时列出 3D 模型。
    pub model3ds: bool,
    /// 为 true 时列出页眉 DrawingML 水印。
    pub watermarks: bool,
    /// 为 true 时列出表格。
    pub tables: bool,
    /// 为 true 时列出修订痕迹。
    pub revisions: bool,
    /// 为 true 时列出超链接。
    pub hyperlinks: bool,
    /// 为 true 时列出书签。
    pub bookmarks: bool,
    /// 为 true 时列出例外编辑区。
    pub edit_ranges: bool,
    /// 为 true 时列出域。
    pub fields: bool,
    /// 为 true 时列出目录块。
    pub tocs: bool,
    /// 为 true 时列出题注。
    pub captions: bool,
    /// 为 true 时列出交叉引用。
    pub cross_refs: bool,
    /// 为 true 时列出 INDEX 块。
    pub indexes: bool,
    /// 为 true 时列出 XE 索引项。
    pub index_entries: bool,
    /// 为 true 时列出书目源。
    pub sources: bool,
    /// 为 true 时列出 CITATION。
    pub citations: bool,
    /// 为 true 时列出 BIBLIOGRAPHY 块。
    pub bibliographies: bool,
    /// 为 true 时列出 glossary 构建基块。
    pub doc_parts: bool,
    /// 为 true 时列出嵌入字体。
    pub embedded_fonts: bool,
    /// 为 true 时列出公式。
    pub maths: bool,
    /// 为 true 时列出拼音。
    pub rubies: bool,
    /// 为 true 时列出内容控件。
    pub content_controls: bool,
    /// 为 true 时列出编号定义。
    pub numbering: bool,
    /// 为 true 时列出样式定义。
    pub styles: bool,
    /// 为 true 时列出已切出段落上的 run。
    pub runs: bool,
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
pub struct InsertShapeOp {
    pub id: String,
    pub host: para::HostPara,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
    pub prst: String,
}

#[derive(Debug, Clone)]
pub struct DeleteShapeOp {
    pub id: String,
    pub target: TxbxTarget,
}

#[derive(Debug, Clone)]
pub struct SetShapeSizeOp {
    pub id: String,
    pub target: TxbxTarget,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct SetShapePosOp {
    pub id: String,
    pub target: TxbxTarget,
    pub x: Option<i64>,
    pub y: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct SetShapeFillOp {
    pub id: String,
    pub target: TxbxTarget,
    pub color: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SetShapeLineOp {
    pub id: String,
    pub target: TxbxTarget,
    pub color: Option<String>,
    pub width: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct InsertConnectorOp {
    pub id: String,
    pub host: para::HostPara,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
    pub prst: String,
}

#[derive(Debug, Clone)]
pub struct DeleteConnectorOp {
    pub id: String,
    pub target: TxbxTarget,
}

#[derive(Debug, Clone)]
pub struct GroupShapesOp {
    pub id: String,
    pub host: super::group::GroupHost,
    pub members: Vec<super::group::GroupMember>,
}

#[derive(Debug, Clone)]
pub struct UngroupOp {
    pub id: String,
    pub member: super::group::GroupMember,
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
pub enum InsertHyperlinkHost {
    Para { para: ParaTarget, text: String },
    Picture(PicTarget),
    TextBox(TxbxTarget),
}

#[derive(Debug, Clone)]
pub enum HyperlinkTarget {
    Link(super::link::LinkRef),
    Picture(PicTarget),
    TextBox(TxbxTarget),
}

#[derive(Debug, Clone)]
pub struct InsertHyperlinkOp {
    pub id: String,
    pub host: InsertHyperlinkHost,
    pub url: Option<String>,
    pub anchor: Option<String>,
    pub tooltip: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SetHyperlinkOp {
    pub id: String,
    pub target: HyperlinkTarget,
    pub url: Option<String>,
    pub anchor: Option<String>,
    pub text: Option<String>,
    pub tooltip: Option<Option<String>>,
}

#[derive(Debug, Clone)]
pub struct DeleteHyperlinkOp {
    pub id: String,
    pub target: HyperlinkTarget,
}

#[derive(Debug, Clone)]
pub struct InsertBookmarkOp {
    pub id: String,
    pub start: ParaTarget,
    pub end: ParaTarget,
    pub name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SetBookmarkOp {
    pub id: String,
    pub target: super::link::BookmarkRef,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct DeleteBookmarkOp {
    pub id: String,
    pub target: super::link::BookmarkRef,
}

#[derive(Debug, Clone)]
pub struct InsertEditRangeOp {
    pub id: String,
    pub para: ParaTarget,
    pub ed_grp: Option<String>,
    pub ed: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DeleteEditRangeOp {
    pub id: String,
    pub target: super::perm::PermRef,
}

#[derive(Debug, Clone)]
pub struct InsertFieldOp {
    pub id: String,
    pub para: ParaTarget,
    pub instr: String,
    pub result: Option<String>,
    pub form: Option<super::field::FormWrite>,
}

#[derive(Debug, Clone)]
pub struct SetFieldOp {
    pub id: String,
    pub target: super::field::FieldRef,
    pub instr: String,
}

#[derive(Debug, Clone)]
pub struct DeleteFieldOp {
    pub id: String,
    pub target: super::field::FieldRef,
    pub keep: super::field::FieldKeep,
}

#[derive(Debug, Clone)]
pub struct InsertNumberingOp {
    pub kind: super::numbering::NumberingKind,
}

#[derive(Debug, Clone)]
pub struct SetNumberingOp {
    pub id: String,
    pub abstract_id: u32,
    pub ilvl: u32,
    pub num_fmt: Option<String>,
    pub start: Option<u32>,
    pub lvl_text: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SetParaNumberingOp {
    pub id: String,
    pub para: ParaTarget,
    pub num: Option<u32>,
    pub ilvl: u32,
}

#[derive(Debug, Clone)]
pub struct SetStyleOp {
    pub id: String,
    pub style_id: String,
    pub font: Option<String>,
    pub east_asia_font: Option<String>,
    pub sz: Option<u32>,
    pub bold: Option<bool>,
    pub color: Option<String>,
    pub before: Option<u32>,
    pub after: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct SetParaStyleOp {
    pub id: String,
    pub para: ParaTarget,
    pub style: Option<String>,
}

#[derive(Debug, Clone)]
pub struct InsertStyleOp {
    pub style_id: String,
    pub ty: String,
    pub name: String,
    pub based_on: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DeleteStyleOp {
    pub id: String,
    pub style_id: String,
}

#[derive(Debug, Clone)]
pub struct SetRunStyleOp {
    pub id: String,
    pub para: ParaTarget,
    pub run: usize,
    pub style: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SetTableStyleOp {
    pub id: String,
    pub target: TableRef,
    pub style: Option<Option<String>>,
    pub look: Option<Option<super::tbl::TableLookPatch>>,
}

#[derive(Debug, Clone)]
pub struct SetTableFloatOp {
    pub id: String,
    pub target: TableRef,
    pub pos: Option<super::tbl::TableFloat>,
}

#[derive(Debug, Clone)]
pub struct SetTableCellMarOp {
    pub id: String,
    pub target: TableRef,
    pub top: Option<Option<u32>>,
    pub left: Option<Option<u32>>,
    pub bottom: Option<Option<u32>>,
    pub right: Option<Option<u32>>,
}

#[derive(Debug, Clone)]
pub struct SetCellFormatOp {
    pub id: String,
    pub target: TableRef,
    pub fill: Option<Option<String>>,
    pub top: Option<Option<u32>>,
    pub left: Option<Option<u32>>,
    pub bottom: Option<Option<u32>>,
    pub right: Option<Option<u32>>,
    pub tl2br: Option<Option<super::tbl::DiagonalBorder>>,
    pub tr2bl: Option<Option<super::tbl::DiagonalBorder>>,
}

#[derive(Debug, Clone)]
pub struct SetParaFormatOp {
    pub id: String,
    pub para: ParaTarget,
    pub align: Option<Option<String>>,
    pub left: Option<Option<u32>>,
    pub right: Option<Option<u32>>,
    pub first_line: Option<Option<u32>>,
    pub hanging: Option<Option<u32>>,
    pub before: Option<Option<u32>>,
    pub after: Option<Option<u32>>,
    pub line: Option<Option<u32>>,
    pub line_rule: Option<Option<String>>,
    pub kinsoku: Option<Option<bool>>,
    pub overflow_punct: Option<Option<bool>>,
    pub top_line_punct: Option<Option<bool>>,
    pub auto_space_de: Option<Option<bool>>,
    pub auto_space_dn: Option<Option<bool>>,
}

#[derive(Debug, Clone)]
pub struct SetRunFormatOp {
    pub id: String,
    pub para: ParaTarget,
    pub run: usize,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub underline: Option<Option<String>>,
    pub sz: Option<u32>,
    pub color: Option<String>,
    pub font: Option<String>,
    pub east_asia_font: Option<String>,
    pub em: Option<Option<String>>,
    pub east_asian_layout: Option<Option<super::format::EastAsianLayout>>,
}

#[derive(Debug, Clone)]
pub enum InsertSectionPlace {
    After(para::BodyPara),
    EndOfDocument,
}

#[derive(Debug, Clone)]
pub struct InsertSectionOp {
    pub id: String,
    pub place: InsertSectionPlace,
    pub break_type: super::sect::SectionBreakType,
}

#[derive(Debug, Clone)]
pub struct DeleteSectionOp {
    pub id: String,
    pub n: usize,
}

#[derive(Debug, Clone)]
pub enum InsertTocPlace {
    After(para::BodyPara),
    EndOfDocument,
}

#[derive(Debug, Clone)]
pub struct InsertTocOp {
    pub id: String,
    pub place: InsertTocPlace,
    pub instr: String,
}

#[derive(Debug, Clone)]
pub struct DeleteTocOp {
    pub id: String,
    pub n: usize,
}

#[derive(Debug, Clone)]
pub enum InsertCaptionPlace {
    After(para::BodyPara),
    EndOfDocument,
}

#[derive(Debug, Clone)]
pub struct InsertCaptionOp {
    pub id: String,
    pub place: InsertCaptionPlace,
    pub label: String,
    pub title: Option<String>,
    pub bookmark: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DeleteCaptionOp {
    pub id: String,
    pub n: usize,
}

#[derive(Debug, Clone)]
pub struct InsertCrossRefOp {
    pub id: String,
    pub para: ParaTarget,
    pub bookmark: String,
    pub kind: super::field::CrossRefKind,
}

#[derive(Debug, Clone)]
pub struct DeleteCrossRefOp {
    pub id: String,
    pub n: usize,
}

#[derive(Debug, Clone)]
pub enum InsertIndexPlace {
    After(para::BodyPara),
    EndOfDocument,
}

#[derive(Debug, Clone)]
pub struct InsertIndexOp {
    pub id: String,
    pub place: InsertIndexPlace,
    pub instr: String,
}

#[derive(Debug, Clone)]
pub struct DeleteIndexOp {
    pub id: String,
    pub n: usize,
}

#[derive(Debug, Clone)]
pub struct InsertIndexEntryOp {
    pub id: String,
    pub para: ParaTarget,
    pub entry: String,
}

#[derive(Debug, Clone)]
pub struct DeleteIndexEntryOp {
    pub id: String,
    pub n: usize,
}

#[derive(Debug, Clone)]
pub struct InsertSourceOp {
    pub tag: String,
    pub source_type: String,
    pub title: String,
    pub year: Option<String>,
    pub last: Option<String>,
    pub first: Option<String>,
    pub city: Option<String>,
    pub publisher: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DeleteSourceOp {
    pub id: String,
    pub n: usize,
}

#[derive(Debug, Clone)]
pub struct InsertCitationOp {
    pub id: String,
    pub para: ParaTarget,
    pub tag: String,
}

#[derive(Debug, Clone)]
pub struct DeleteCitationOp {
    pub id: String,
    pub n: usize,
}

#[derive(Debug, Clone)]
pub enum InsertBibliographyPlace {
    After(para::BodyPara),
    EndOfDocument,
}

#[derive(Debug, Clone)]
pub struct InsertBibliographyOp {
    pub id: String,
    pub place: InsertBibliographyPlace,
    pub instr: String,
}

#[derive(Debug, Clone)]
pub struct DeleteBibliographyOp {
    pub id: String,
    pub n: usize,
}

#[derive(Debug, Clone)]
pub struct InsertDocPartOp {
    pub name: String,
    pub gallery: String,
    pub category: String,
    pub behavior: String,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct DeleteDocPartOp {
    pub id: String,
    pub n: usize,
}

#[derive(Debug, Clone)]
pub enum InsertDocPartContentPlace {
    After(para::BodyPara),
    EndOfDocument,
}

#[derive(Debug, Clone)]
pub struct InsertDocPartContentOp {
    pub id: String,
    pub place: InsertDocPartContentPlace,
    pub n: usize,
    pub part_id: String,
}

#[derive(Debug, Clone)]
pub struct InsertEmbeddedFontOp {
    pub name: String,
    pub style: super::font_embed::FontStyle,
    pub file: String,
}

#[derive(Debug, Clone)]
pub struct DeleteEmbeddedFontOp {
    pub id: String,
    pub n: usize,
}

#[derive(Debug, Clone)]
pub struct SetFontEmbedOp {
    pub true_type: Option<bool>,
    pub system_fonts: Option<bool>,
    pub subset: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct SetPageBackgroundOp {
    pub color: Option<String>,
    pub fill: Option<super::page_bg::PageFill>,
}

#[derive(Debug, Clone)]
pub struct SetPageBordersOp {
    pub id: String,
    pub n: usize,
    pub enabled: Option<bool>,
    pub offset_from: Option<String>,
    pub display: Option<String>,
    pub z_order: Option<String>,
    pub top: Option<Option<super::page_bg::BorderSide>>,
    pub left: Option<Option<super::page_bg::BorderSide>>,
    pub bottom: Option<Option<super::page_bg::BorderSide>>,
    pub right: Option<Option<super::page_bg::BorderSide>>,
}

#[derive(Debug, Clone)]
pub struct SetFormProtOp {
    pub id: String,
    pub n: usize,
    pub form_prot: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct SetBibliographyStyleOp {
    pub style_name: String,
}

#[derive(Debug, Clone)]
pub enum InsertMathPlace {
    After(ParaTarget),
    EndOfDocument,
}

#[derive(Debug, Clone)]
pub struct InsertMathOp {
    pub id: String,
    pub place: InsertMathPlace,
    pub kind: super::math::MathKind,
    pub body: MathBody,
    pub jc: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SetMathOp {
    pub id: String,
    pub target: super::math::MathRef,
    pub body: Option<MathBody>,
    pub jc: Option<Option<String>>,
}

#[derive(Debug, Clone)]
pub struct DeleteMathOp {
    pub id: String,
    pub target: super::math::MathRef,
}

#[derive(Debug, Clone)]
pub enum InsertRubyPlace {
    After(ParaTarget),
    EndOfDocument,
}

#[derive(Debug, Clone)]
pub struct InsertRubyOp {
    pub id: String,
    pub place: InsertRubyPlace,
    pub base: String,
    pub rt: String,
    pub align: String,
    pub hps: u32,
    pub hps_raise: u32,
    pub hps_base_text: u32,
    pub lid: String,
}

#[derive(Debug, Clone)]
pub struct SetRubyOp {
    pub id: String,
    pub target: super::ruby::RubyRef,
    pub base: Option<String>,
    pub rt: Option<String>,
    pub align: Option<String>,
    pub hps: Option<u32>,
    pub hps_raise: Option<u32>,
    pub hps_base_text: Option<u32>,
    pub lid: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DeleteRubyOp {
    pub id: String,
    pub target: super::ruby::RubyRef,
}

#[derive(Debug, Clone)]
pub struct SetEastAsianSettingsOp {
    pub character_spacing: Option<Option<String>>,
    pub strict: Option<Option<bool>>,
    pub no_before: Option<Option<super::east_asia::KinsokuChars>>,
    pub no_after: Option<Option<super::east_asia::KinsokuChars>>,
}

#[derive(Debug, Clone)]
pub enum InsertSdtPlace {
    After(ParaTarget),
    EndOfDocument,
}

#[derive(Debug, Clone)]
pub struct InsertContentControlOp {
    pub id: String,
    pub place: InsertSdtPlace,
    pub kind: super::sdt::SdtKind,
    pub text: String,
    pub tag: Option<String>,
    pub alias: Option<String>,
    pub items: Vec<String>,
    pub full_date: Option<String>,
    pub file: Option<String>,
    pub checked: bool,
}

#[derive(Debug, Clone)]
pub struct SetContentControlOp {
    pub id: String,
    pub target: super::sdt::SdtRef,
    pub text: Option<String>,
    pub tag: Option<String>,
    pub alias: Option<String>,
    pub items: Option<Vec<String>>,
    pub full_date: Option<String>,
    pub file: Option<String>,
    pub checked: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct DeleteContentControlOp {
    pub id: String,
    pub target: super::sdt::SdtRef,
    pub keep: super::sdt::SdtKeep,
}

#[derive(Debug, Clone)]
pub enum DocxOp {
    SetParaText(SetParaTextOp),
    SetHeaderFooterDistance(SetHfDistanceOp),
    SetHeaderFooterLink(SetHfLinkOp),
    InsertHeaderFooter(InsertHeaderFooterOp),
    InsertSection(InsertSectionOp),
    DeleteSection(DeleteSectionOp),
    InsertToc(InsertTocOp),
    DeleteToc(DeleteTocOp),
    InsertCaption(InsertCaptionOp),
    DeleteCaption(DeleteCaptionOp),
    InsertCrossRef(InsertCrossRefOp),
    DeleteCrossRef(DeleteCrossRefOp),
    InsertIndex(InsertIndexOp),
    DeleteIndex(DeleteIndexOp),
    InsertIndexEntry(InsertIndexEntryOp),
    DeleteIndexEntry(DeleteIndexEntryOp),
    InsertSource(InsertSourceOp),
    DeleteSource(DeleteSourceOp),
    InsertCitation(InsertCitationOp),
    DeleteCitation(DeleteCitationOp),
    InsertBibliography(InsertBibliographyOp),
    DeleteBibliography(DeleteBibliographyOp),
    SetBibliographyStyle(SetBibliographyStyleOp),
    InsertDocPart(InsertDocPartOp),
    DeleteDocPart(DeleteDocPartOp),
    InsertDocPartContent(InsertDocPartContentOp),
    InsertEmbeddedFont(InsertEmbeddedFontOp),
    DeleteEmbeddedFont(DeleteEmbeddedFontOp),
    SetFontEmbed(SetFontEmbedOp),
    SetPageBackground(SetPageBackgroundOp),
    SetPageBorders(SetPageBordersOp),
    SetFormProt(SetFormProtOp),
    InsertMath(InsertMathOp),
    SetMath(SetMathOp),
    DeleteMath(DeleteMathOp),
    InsertRuby(InsertRubyOp),
    SetRuby(SetRubyOp),
    DeleteRuby(DeleteRubyOp),
    SetEastAsianSettings(SetEastAsianSettingsOp),
    InsertContentControl(InsertContentControlOp),
    SetContentControl(SetContentControlOp),
    DeleteContentControl(DeleteContentControlOp),
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
    InsertConnector(InsertConnectorOp),
    DeleteConnector(DeleteConnectorOp),
    InsertShape(InsertShapeOp),
    DeleteShape(DeleteShapeOp),
    SetShapeSize(SetShapeSizeOp),
    SetShapePos(SetShapePosOp),
    SetShapeFill(SetShapeFillOp),
    SetShapeLine(SetShapeLineOp),
    GroupShapes(GroupShapesOp),
    Ungroup(UngroupOp),
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
    InsertHyperlink(InsertHyperlinkOp),
    SetHyperlink(SetHyperlinkOp),
    DeleteHyperlink(DeleteHyperlinkOp),
    InsertBookmark(InsertBookmarkOp),
    SetBookmark(SetBookmarkOp),
    DeleteBookmark(DeleteBookmarkOp),
    InsertEditRange(InsertEditRangeOp),
    DeleteEditRange(DeleteEditRangeOp),
    InsertField(InsertFieldOp),
    SetField(SetFieldOp),
    DeleteField(DeleteFieldOp),
    InsertNumbering(InsertNumberingOp),
    SetNumbering(SetNumberingOp),
    SetParaNumbering(SetParaNumberingOp),
    SetStyle(SetStyleOp),
    SetParaStyle(SetParaStyleOp),
    InsertStyle(InsertStyleOp),
    DeleteStyle(DeleteStyleOp),
    SetRunStyle(SetRunStyleOp),
    SetTableStyle(SetTableStyleOp),
    SetTableFloat(SetTableFloatOp),
    SetTableCellMar(SetTableCellMarOp),
    SetCellFormat(SetCellFormatOp),
    SetParaFormat(SetParaFormatOp),
    SetRunFormat(SetRunFormatOp),
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
    InsertTrendline(charts::InsertTrendlineOp),
    SetTrendline(charts::SetTrendlineOp),
    DeleteTrendline(charts::DeleteTrendlineOp),
    InsertErrorBars(charts::InsertErrorBarsOp),
    SetErrorBars(charts::SetErrorBarsOp),
    DeleteErrorBars(charts::DeleteErrorBarsOp),
    SetChartSeriesAxis(charts::SetChartSeriesAxisOp),
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
    InsertOle(super::ole::InsertOleOp),
    DeleteOle(super::ole::DeleteOleOp),
    InsertInkCanvas(super::ink::InsertInkCanvasOp),
    ReplaceInkCanvas(super::ink::ReplaceInkCanvasOp),
    DeleteInkCanvas(super::ink::DeleteInkCanvasOp),
    InsertModel3d(super::model3d::InsertModel3dOp),
    ReplaceModel3d(super::model3d::ReplaceModel3dOp),
    DeleteModel3d(super::model3d::DeleteModel3dOp),
    InsertWatermark(super::watermark::InsertWatermarkOp),
    DeleteWatermark(super::watermark::DeleteWatermarkOp),
    SetProtection(super::protect::SetProtectionOp),
    SetThemeColors(super::theme::SetThemeColorsOp),
    SetThemeFonts(super::theme::SetThemeFontsOp),
    SetThemeFmt(super::theme::SetThemeFmtOp),
    SetThemeOverride(super::theme::SetThemeOverrideOp),
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
            "from",
            "maxParas",
            "headerFooter",
            "pictures",
            "textBoxes",
            "connectors",
            "shapes",
            "inkCanvases",
            "textBoxContent",
            "notes",
            "comments",
            "charts",
            "smartArt",
            "oleObjects",
            "model3ds",
            "watermarks",
            "tables",
            "revisions",
            "hyperlinks",
            "bookmarks",
            "editRanges",
            "fields",
            "tocs",
            "captions",
            "crossRefs",
            "indexes",
            "indexEntries",
            "sources",
            "citations",
            "bibliographies",
            "docParts",
            "embeddedFonts",
            "maths",
            "rubies",
            "contentControls",
            "numbering",
            "styles",
            "runs",
        ],
    )?;

    let header_footer = parse_header_footer(obj.get("headerFooter"))?;
    let pictures = parse_bool_field(obj.get("pictures"), "pictures")?;
    let text_boxes = parse_bool_field(obj.get("textBoxes"), "textBoxes")?;
    let connectors = parse_bool_field(obj.get("connectors"), "connectors")?;
    let shapes = parse_bool_field(obj.get("shapes"), "shapes")?;
    let ink_canvases = parse_bool_field(obj.get("inkCanvases"), "inkCanvases")?;
    let text_box_content = parse_bool_field(obj.get("textBoxContent"), "textBoxContent")?;
    let notes = parse_bool_field(obj.get("notes"), "notes")?;
    let comments = parse_bool_field(obj.get("comments"), "comments")?;
    let charts = parse_bool_field(obj.get("charts"), "charts")?;
    let smart_art = parse_bool_field(obj.get("smartArt"), "smartArt")?;
    let ole_objects = parse_bool_field(obj.get("oleObjects"), "oleObjects")?;
    let model3ds = parse_bool_field(obj.get("model3ds"), "model3ds")?;
    let watermarks = parse_bool_field(obj.get("watermarks"), "watermarks")?;
    let tables = parse_bool_field(obj.get("tables"), "tables")?;
    let revisions = parse_bool_field(obj.get("revisions"), "revisions")?;
    let hyperlinks = parse_bool_field(obj.get("hyperlinks"), "hyperlinks")?;
    let bookmarks = parse_bool_field(obj.get("bookmarks"), "bookmarks")?;
    let edit_ranges = parse_bool_field(obj.get("editRanges"), "editRanges")?;
    let fields = parse_bool_field(obj.get("fields"), "fields")?;
    let tocs = parse_bool_field(obj.get("tocs"), "tocs")?;
    let captions = parse_bool_field(obj.get("captions"), "captions")?;
    let cross_refs = parse_bool_field(obj.get("crossRefs"), "crossRefs")?;
    let indexes = parse_bool_field(obj.get("indexes"), "indexes")?;
    let index_entries = parse_bool_field(obj.get("indexEntries"), "indexEntries")?;
    let sources = parse_bool_field(obj.get("sources"), "sources")?;
    let citations = parse_bool_field(obj.get("citations"), "citations")?;
    let bibliographies = parse_bool_field(obj.get("bibliographies"), "bibliographies")?;
    let doc_parts = parse_bool_field(obj.get("docParts"), "docParts")?;
    let embedded_fonts = parse_bool_field(obj.get("embeddedFonts"), "embeddedFonts")?;
    let maths = parse_bool_field(obj.get("maths"), "maths")?;
    let rubies = parse_bool_field(obj.get("rubies"), "rubies")?;
    let content_controls = parse_bool_field(obj.get("contentControls"), "contentControls")?;
    let numbering = parse_bool_field(obj.get("numbering"), "numbering")?;
    let styles = parse_bool_field(obj.get("styles"), "styles")?;
    let runs = parse_bool_field(obj.get("runs"), "runs")?;
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
                && !obj.contains_key("connectors")
                && !obj.contains_key("shapes")
                && !obj.contains_key("inkCanvases")
                && !obj.contains_key("textBoxContent")
                && !obj.contains_key("notes")
                && !obj.contains_key("comments")
                && !obj.contains_key("charts")
                && !obj.contains_key("smartArt")
                && !obj.contains_key("oleObjects")
                && !obj.contains_key("model3ds")
                && !obj.contains_key("watermarks")
                && !obj.contains_key("tables")
                && !obj.contains_key("revisions")
                && !obj.contains_key("hyperlinks")
                && !obj.contains_key("bookmarks")
                && !obj.contains_key("editRanges")
                && !obj.contains_key("fields")
                && !obj.contains_key("tocs")
                && !obj.contains_key("captions")
                && !obj.contains_key("crossRefs")
                && !obj.contains_key("indexes")
                && !obj.contains_key("indexEntries")
                && !obj.contains_key("sources")
                && !obj.contains_key("citations")
                && !obj.contains_key("bibliographies")
                && !obj.contains_key("docParts")
                && !obj.contains_key("embeddedFonts")
                && !obj.contains_key("maths")
                && !obj.contains_key("rubies")
                && !obj.contains_key("contentControls")
                && !obj.contains_key("numbering")
                && !obj.contains_key("styles")
                && !obj.contains_key("runs")
            {
                return Ok(None);
            }
            Ok(Some(InspectScope {
                from: None,
                max_paras: None,
                header_footer,
                pictures,
                text_boxes,
                connectors,
                shapes,
                ink_canvases,
                text_box_content,
                notes,
                comments,
                charts,
                smart_art,
                ole_objects,
                model3ds,
                watermarks,
                tables,
                revisions,
                hyperlinks,
                bookmarks,
                edit_ranges,
                fields,
                tocs,
                captions,
                cross_refs,
                indexes,
                index_entries,
                sources,
                citations,
                bibliographies,
                doc_parts,
                embedded_fonts,
                maths,
                rubies,
                content_controls,
                numbering,
                styles,
                runs,
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
                connectors,
                shapes,
                ink_canvases,
                text_box_content,
                notes,
                comments,
                charts,
                smart_art,
                ole_objects,
                model3ds,
                watermarks,
                tables,
                revisions,
                hyperlinks,
                bookmarks,
                edit_ranges,
                fields,
                tocs,
                captions,
                cross_refs,
                indexes,
                index_entries,
                sources,
                citations,
                bibliographies,
                doc_parts,
                embedded_fonts,
                maths,
                rubies,
                content_controls,
                numbering,
                styles,
                runs,
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
            || op_name == "insertSheet"
            || op_name == "deleteSheet"
            || op_name == "renameSheet"
            || op_name == "moveSheet"
            || op_name == "setSheetHidden"
            || op_name == "setCellStyle"
            || op_name == "insertCellStyle"
            || op_name == "deleteCellStyle"
            || op_name == "setCellNamedStyle"
            || op_name == "setColWidth"
            || op_name == "setRowHeight"
            || op_name == "mergeRange"
            || op_name == "unmergeRange"
            || op_name == "insertDataValidation"
            || op_name == "setDataValidation"
            || op_name == "deleteDataValidation"
            || op_name == "insertConditionalFormat"
            || op_name == "setConditionalFormat"
            || op_name == "deleteConditionalFormat"
            || op_name == "insertSheetTable"
            || op_name == "setSheetTable"
            || op_name == "deleteSheetTable"
            || op_name == "insertPivotTable"
            || op_name == "setPivotTable"
            || op_name == "deletePivotTable"
            || op_name == "insertDefinedName"
            || op_name == "setDefinedName"
            || op_name == "deleteDefinedName"
            || op_name == "insertProtectedRange"
            || op_name == "setProtectedRange"
            || op_name == "deleteProtectedRange"
            || op_name == "setPrintSetup"
            || op_name == "setSheetView"
            || op_name == "setAutoFilter"
            || op_name == "setSortState"
            || op_name == "insertCustomView"
            || op_name == "deleteCustomView"
            || op_name == "showCustomView"
            || op_name == "groupRows"
            || op_name == "ungroupRows"
            || op_name == "groupCols"
            || op_name == "ungroupCols"
            || op_name == "setOutline"
            || op_name == "setRowHidden"
            || op_name == "setColHidden"
            || op_name == "setPicturePos"
            || op_name == "setChartPos"
            || op_name == "setSmartArtPos"
            || op_name == "insertSparkline"
            || op_name == "setSparkline"
            || op_name == "deleteSparkline"
            || op_name == "insertSlicer"
            || op_name == "deleteSlicer"
            || op_name == "insertConnection"
            || op_name == "deleteConnection"
            || op_name == "insertExternalLink"
            || op_name == "setExternalLink"
            || op_name == "deleteExternalLink"
            || op_name == "insertSlide"
            || op_name == "deleteSlide"
            || op_name == "moveSlide"
            || op_name == "setSlideHidden"
            || op_name == "setTransition"
            || op_name == "insertAnim"
            || op_name == "setAnim"
            || op_name == "deleteAnim"
            || op_name == "moveAnim"
            || op_name == "setSlideLayout"
            || op_name == "insertLayout"
            || op_name == "deleteLayout"
            || op_name == "insertPlaceholder"
            || op_name == "deletePlaceholder"
            || op_name == "insertMaster"
            || op_name == "deleteMaster"
            || op_name == "setSlideSize"
            || op_name == "setHandoutText"
            || op_name == "setShapeFormat"
            || op_name == "setShapeGeom"
            || op_name == "setConnector"
            || op_name == "setTableCellText"
            || op_name == "setTablePos"
            || op_name == "setTableSize"
            || op_name == "insertVideo"
            || op_name == "insertAudio"
            || op_name == "deleteMedia"
            || op_name == "setMedia"
            || op_name == "insertSlideSection"
            || op_name == "setSlideSectionName"
            || op_name == "deleteSlideSection"
            || op_name == "setSlideSection"
            || op_name == "insertCustomShow"
            || op_name == "setCustomShow"
            || op_name == "deleteCustomShow"
            || op_name == "insertZoom"
            || op_name == "deleteZoom"
            || op_name == "setZoom"
            || op_name == "setShowPr"
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
            "insertSection" => out.push(DocxOp::InsertSection(parse_insert_section(obj, i)?)),
            "deleteSection" => out.push(DocxOp::DeleteSection(parse_delete_section(obj, i)?)),
            "insertToc" => out.push(DocxOp::InsertToc(parse_insert_toc(obj, i)?)),
            "deleteToc" => out.push(DocxOp::DeleteToc(parse_delete_toc(obj, i)?)),
            "insertCaption" => out.push(DocxOp::InsertCaption(parse_insert_caption(obj, i)?)),
            "deleteCaption" => out.push(DocxOp::DeleteCaption(parse_delete_caption(obj, i)?)),
            "insertCrossRef" => out.push(DocxOp::InsertCrossRef(parse_insert_cross_ref(obj, i)?)),
            "deleteCrossRef" => out.push(DocxOp::DeleteCrossRef(parse_delete_cross_ref(obj, i)?)),
            "insertIndex" => out.push(DocxOp::InsertIndex(parse_insert_index(obj, i)?)),
            "deleteIndex" => out.push(DocxOp::DeleteIndex(parse_delete_index(obj, i)?)),
            "insertIndexEntry" => {
                out.push(DocxOp::InsertIndexEntry(parse_insert_index_entry(obj, i)?))
            }
            "deleteIndexEntry" => {
                out.push(DocxOp::DeleteIndexEntry(parse_delete_index_entry(obj, i)?))
            }
            "insertSource" => out.push(DocxOp::InsertSource(parse_insert_source(obj, i)?)),
            "deleteSource" => out.push(DocxOp::DeleteSource(parse_delete_source(obj, i)?)),
            "insertCitation" => out.push(DocxOp::InsertCitation(parse_insert_citation(obj, i)?)),
            "deleteCitation" => out.push(DocxOp::DeleteCitation(parse_delete_citation(obj, i)?)),
            "insertBibliography" => out.push(DocxOp::InsertBibliography(
                parse_insert_bibliography(obj, i)?,
            )),
            "deleteBibliography" => out.push(DocxOp::DeleteBibliography(
                parse_delete_bibliography(obj, i)?,
            )),
            "setBibliographyStyle" => out.push(DocxOp::SetBibliographyStyle(
                parse_set_bibliography_style(obj, i)?,
            )),
            "insertDocPart" => out.push(DocxOp::InsertDocPart(parse_insert_doc_part(obj, i)?)),
            "deleteDocPart" => out.push(DocxOp::DeleteDocPart(parse_delete_doc_part(obj, i)?)),
            "insertDocPartContent" => out.push(DocxOp::InsertDocPartContent(
                parse_insert_doc_part_content(obj, i)?,
            )),
            "insertEmbeddedFont" => out.push(DocxOp::InsertEmbeddedFont(
                parse_insert_embedded_font(obj, i)?,
            )),
            "deleteEmbeddedFont" => out.push(DocxOp::DeleteEmbeddedFont(
                parse_delete_embedded_font(obj, i)?,
            )),
            "setFontEmbed" => out.push(DocxOp::SetFontEmbed(parse_set_font_embed(obj, i)?)),
            "setPageBackground" => out.push(DocxOp::SetPageBackground(parse_set_page_background(
                obj, i,
            )?)),
            "setPageBorders" => out.push(DocxOp::SetPageBorders(parse_set_page_borders(obj, i)?)),
            "setFormProt" => out.push(DocxOp::SetFormProt(parse_set_form_prot(obj, i)?)),
            "insertMath" => out.push(DocxOp::InsertMath(parse_insert_math(obj, i)?)),
            "setMath" => out.push(DocxOp::SetMath(parse_set_math(obj, i)?)),
            "deleteMath" => out.push(DocxOp::DeleteMath(parse_delete_math(obj, i)?)),
            "insertRuby" => out.push(DocxOp::InsertRuby(parse_insert_ruby(obj, i)?)),
            "setRuby" => out.push(DocxOp::SetRuby(parse_set_ruby(obj, i)?)),
            "deleteRuby" => out.push(DocxOp::DeleteRuby(parse_delete_ruby(obj, i)?)),
            "setEastAsianSettings" => out.push(DocxOp::SetEastAsianSettings(
                parse_set_east_asian_settings(obj, i)?,
            )),
            "insertContentControl" => {
                out.push(DocxOp::InsertContentControl(parse_insert_sdt(obj, i)?))
            }
            "setContentControl" => out.push(DocxOp::SetContentControl(parse_set_sdt(obj, i)?)),
            "deleteContentControl" => {
                out.push(DocxOp::DeleteContentControl(parse_delete_sdt(obj, i)?))
            }
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
            "insertConnector" => out.push(DocxOp::InsertConnector(parse_insert_connector(obj, i)?)),
            "deleteConnector" => out.push(DocxOp::DeleteConnector(parse_delete_connector(obj, i)?)),
            "insertShape" => out.push(DocxOp::InsertShape(parse_insert_shape(obj, i)?)),
            "deleteShape" => out.push(DocxOp::DeleteShape(parse_delete_shape(obj, i)?)),
            "setShapeSize" => out.push(DocxOp::SetShapeSize(parse_set_shape_size(obj, i)?)),
            "setShapePos" => out.push(DocxOp::SetShapePos(parse_set_shape_pos(obj, i)?)),
            "setShapeFill" => out.push(DocxOp::SetShapeFill(parse_set_shape_fill(obj, i)?)),
            "setShapeLine" => out.push(DocxOp::SetShapeLine(parse_set_shape_line(obj, i)?)),
            "groupShapes" => out.push(DocxOp::GroupShapes(parse_group_shapes(obj, i)?)),
            "ungroup" => out.push(DocxOp::Ungroup(parse_ungroup(obj, i)?)),
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
            "insertHyperlink" => out.push(DocxOp::InsertHyperlink(parse_insert_hyperlink(obj, i)?)),
            "setHyperlink" => out.push(DocxOp::SetHyperlink(parse_set_hyperlink(obj, i)?)),
            "deleteHyperlink" => out.push(DocxOp::DeleteHyperlink(parse_delete_hyperlink(obj, i)?)),
            "insertBookmark" => out.push(DocxOp::InsertBookmark(parse_insert_bookmark(obj, i)?)),
            "setBookmark" => out.push(DocxOp::SetBookmark(parse_set_bookmark(obj, i)?)),
            "deleteBookmark" => out.push(DocxOp::DeleteBookmark(parse_delete_bookmark(obj, i)?)),
            "insertEditRange" => {
                out.push(DocxOp::InsertEditRange(parse_insert_edit_range(obj, i)?))
            }
            "deleteEditRange" => {
                out.push(DocxOp::DeleteEditRange(parse_delete_edit_range(obj, i)?))
            }
            "insertField" => out.push(DocxOp::InsertField(parse_insert_field(obj, i)?)),
            "setField" => out.push(DocxOp::SetField(parse_set_field(obj, i)?)),
            "deleteField" => out.push(DocxOp::DeleteField(parse_delete_field(obj, i)?)),
            "insertNumbering" => out.push(DocxOp::InsertNumbering(parse_insert_numbering(obj, i)?)),
            "setNumbering" => out.push(DocxOp::SetNumbering(parse_set_numbering(obj, i)?)),
            "setParaNumbering" => {
                out.push(DocxOp::SetParaNumbering(parse_set_para_numbering(obj, i)?))
            }
            "setStyle" => out.push(DocxOp::SetStyle(parse_set_style(obj, i)?)),
            "setParaStyle" => out.push(DocxOp::SetParaStyle(parse_set_para_style(obj, i)?)),
            "insertStyle" => out.push(DocxOp::InsertStyle(parse_insert_style(obj, i)?)),
            "deleteStyle" => out.push(DocxOp::DeleteStyle(parse_delete_style(obj, i)?)),
            "setRunStyle" => out.push(DocxOp::SetRunStyle(parse_set_run_style(obj, i)?)),
            "setTableStyle" => out.push(DocxOp::SetTableStyle(parse_set_table_style(obj, i)?)),
            "setTableFloat" => out.push(DocxOp::SetTableFloat(parse_set_table_float(obj, i)?)),
            "setTableCellMar" => {
                out.push(DocxOp::SetTableCellMar(parse_set_table_cell_mar(obj, i)?))
            }
            "setCellFormat" => out.push(DocxOp::SetCellFormat(parse_set_cell_format(obj, i)?)),
            "setParaFormat" => out.push(DocxOp::SetParaFormat(parse_set_para_format(obj, i)?)),
            "setRunFormat" => out.push(DocxOp::SetRunFormat(parse_set_run_format(obj, i)?)),
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
            "insertTrendline" => out.push(DocxOp::InsertTrendline(parse_insert_trendline(obj, i)?)),
            "setTrendline" => out.push(DocxOp::SetTrendline(parse_set_trendline(obj, i)?)),
            "deleteTrendline" => out.push(DocxOp::DeleteTrendline(parse_delete_trendline(obj, i)?)),
            "insertErrorBars" => {
                out.push(DocxOp::InsertErrorBars(parse_insert_error_bars(obj, i)?))
            }
            "setErrorBars" => out.push(DocxOp::SetErrorBars(parse_set_error_bars(obj, i)?)),
            "deleteErrorBars" => {
                out.push(DocxOp::DeleteErrorBars(parse_delete_error_bars(obj, i)?))
            }
            "setChartSeriesAxis" => {
                out.push(DocxOp::SetChartSeriesAxis(parse_set_series_axis(obj, i)?))
            }
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
            "insertOle" => out.push(DocxOp::InsertOle(parse_insert_ole(obj, i)?)),
            "deleteOle" => out.push(DocxOp::DeleteOle(parse_delete_ole(obj, i)?)),
            "insertInkCanvas" => {
                out.push(DocxOp::InsertInkCanvas(parse_insert_ink_canvas(obj, i)?))
            }
            "replaceInkCanvas" => {
                out.push(DocxOp::ReplaceInkCanvas(parse_replace_ink_canvas(obj, i)?))
            }
            "deleteInkCanvas" => {
                out.push(DocxOp::DeleteInkCanvas(parse_delete_ink_canvas(obj, i)?))
            }
            "insertModel3d" => out.push(DocxOp::InsertModel3d(parse_insert_model3d(obj, i)?)),
            "replaceModel3d" => out.push(DocxOp::ReplaceModel3d(parse_replace_model3d(obj, i)?)),
            "deleteModel3d" => out.push(DocxOp::DeleteModel3d(parse_delete_model3d(obj, i)?)),
            "insertWatermark" => out.push(DocxOp::InsertWatermark(super::watermark::parse_insert(
                obj, i,
            )?)),
            "deleteWatermark" => out.push(DocxOp::DeleteWatermark(super::watermark::parse_delete(
                obj, i,
            )?)),
            "setProtection" => out.push(DocxOp::SetProtection(super::protect::parse_op(obj, i)?)),
            "setThemeColors" => out.push(DocxOp::SetThemeColors(super::theme::parse_set_colors(
                obj, i,
            )?)),
            "setThemeFonts" => out.push(DocxOp::SetThemeFonts(super::theme::parse_set_fonts(
                obj, i,
            )?)),
            "setThemeFmt" => out.push(DocxOp::SetThemeFmt(super::theme::parse_set_fmt(obj, i)?)),
            "setThemeOverride" => out.push(DocxOp::SetThemeOverride(
                super::theme::parse_set_override(obj, i)?,
            )),
            "setProps" => out.push(DocxOp::SetProps(crate::props::parse_set_props(
                obj,
                i,
                "docx:document",
            )?)),
            "setCustomProp" => out.push(DocxOp::SetCustomProp(
                crate::props::parse_set_custom_prop(obj, i, "docx:document")?,
            )),
            "deleteCustomProp" => out.push(DocxOp::DeleteCustomProp(
                crate::props::parse_delete_custom_prop(obj, i)?,
            )),
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

fn parse_insert_section(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertSectionOp, Error> {
    deny_keys(obj, i, &["op", "id", "type"])?;
    let id = require_op_id(obj, i)?;
    let break_type = match obj.get("type") {
        None => super::sect::SectionBreakType::NextPage,
        Some(Value::String(s)) => super::sect::parse_section_break_type(s)
            .map_err(|kind| Error::with_path(kind, format!("ops[{i}].type")))?,
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
    let path = format!("ops[{i}].id");
    let place = if id == "docx:document" {
        InsertSectionPlace::EndOfDocument
    } else {
        match para::parse_para_id_at(&id, &path)? {
            ParaTarget::Body(p) => InsertSectionPlace::After(p),
            _ => {
                return Err(Error::with_path(
                    ErrorKind::InvalidId {
                        reason: IdReason::InsertSectionNeedsBodyPara,
                    },
                    path,
                ));
            }
        }
    };
    Ok(InsertSectionOp {
        id,
        place,
        break_type,
    })
}

fn parse_delete_section(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteSectionOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_op_id(obj, i)?;
    let n = parse_section_id(&id, i)?;
    Ok(DeleteSectionOp { id, n })
}

fn parse_insert_toc(obj: &serde_json::Map<String, Value>, i: usize) -> Result<InsertTocOp, Error> {
    deny_keys(obj, i, &["op", "id", "kind", "instr"])?;
    let id = require_op_id(obj, i)?;
    let kind = match obj.get("kind") {
        None => None,
        Some(Value::String(s)) => Some(
            super::field::parse_toc_kind(s)
                .map_err(|kind| Error::with_path(kind, format!("ops[{i}].kind")))?,
        ),
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
    let instr = match obj.get("instr") {
        None => None,
        Some(Value::String(s)) => {
            if s.trim().is_empty() {
                return Err(Error::with_path(
                    ErrorKind::EmptyFieldInstr,
                    format!("ops[{i}].instr"),
                ));
            }
            if !super::field::instr_is_toc_or_toa(s) {
                return Err(Error::with_path(
                    ErrorKind::InvalidTocKind,
                    format!("ops[{i}].instr"),
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
                format!("ops[{i}].instr"),
            ));
        }
    };
    let instr = match instr {
        Some(s) => s,
        None => kind
            .unwrap_or(super::field::TocKind::Toc)
            .default_instr()
            .to_string(),
    };
    let path = format!("ops[{i}].id");
    let place = if id == "docx:document" {
        InsertTocPlace::EndOfDocument
    } else {
        match para::parse_para_id_at(&id, &path)? {
            ParaTarget::Body(p) => InsertTocPlace::After(p),
            _ => {
                return Err(Error::with_path(
                    ErrorKind::InvalidId {
                        reason: IdReason::InsertTocNeedsBodyPara,
                    },
                    path,
                ));
            }
        }
    };
    Ok(InsertTocOp { id, place, instr })
}

fn parse_delete_toc(obj: &serde_json::Map<String, Value>, i: usize) -> Result<DeleteTocOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_op_id(obj, i)?;
    let n = parse_toc_id(&id, i)?;
    Ok(DeleteTocOp { id, n })
}

fn parse_insert_caption(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertCaptionOp, Error> {
    deny_keys(obj, i, &["op", "id", "label", "title", "bookmark"])?;
    let id = require_op_id(obj, i)?;
    let label = match obj.get("label") {
        None => "Figure".to_string(),
        Some(Value::String(s)) => {
            super::field::parse_seq_identifier(s)
                .map_err(|kind| Error::with_path(kind, format!("ops[{i}].label")))?;
            s.clone()
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "label".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].label"),
            ));
        }
    };
    let title = optional_op_string(obj, i, "title")?;
    let bookmark = match obj.get("bookmark") {
        None => None,
        Some(Value::String(s)) => {
            if s.is_empty() || s.chars().any(char::is_whitespace) {
                return Err(Error::with_path(
                    ErrorKind::InvalidBookmarkName,
                    format!("ops[{i}].bookmark"),
                ));
            }
            Some(s.clone())
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "bookmark".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].bookmark"),
            ));
        }
    };
    let path = format!("ops[{i}].id");
    let place = if id == "docx:document" {
        InsertCaptionPlace::EndOfDocument
    } else {
        match para::parse_para_id_at(&id, &path)? {
            ParaTarget::Body(p) => InsertCaptionPlace::After(p),
            _ => {
                return Err(Error::with_path(
                    ErrorKind::InvalidId {
                        reason: IdReason::InsertCaptionNeedsBodyPara,
                    },
                    path,
                ));
            }
        }
    };
    Ok(InsertCaptionOp {
        id,
        place,
        label,
        title,
        bookmark,
    })
}

fn parse_delete_caption(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteCaptionOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_op_id(obj, i)?;
    let n = parse_caption_id(&id, i)?;
    Ok(DeleteCaptionOp { id, n })
}

fn parse_insert_cross_ref(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertCrossRefOp, Error> {
    deny_keys(obj, i, &["op", "id", "bookmark", "kind"])?;
    let id = require_op_id(obj, i)?;
    let para = parse_story_para_at(
        &id,
        &format!("ops[{i}].id"),
        IdReason::InsertCrossRefNeedsPara,
    )?;
    let bookmark = match obj.get("bookmark") {
        Some(Value::String(s)) => {
            if s.is_empty() || s.chars().any(char::is_whitespace) {
                return Err(Error::with_path(
                    ErrorKind::InvalidBookmarkName,
                    format!("ops[{i}].bookmark"),
                ));
            }
            s.clone()
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "bookmark".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].bookmark"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::OpRequiresField {
                    op: "insertCrossRef".into(),
                    field: "bookmark".into(),
                },
                format!("ops[{i}].bookmark"),
            ));
        }
    };
    let kind = match obj.get("kind") {
        None => super::field::CrossRefKind::Ref,
        Some(Value::String(s)) => super::field::parse_cross_ref_kind(s)
            .map_err(|kind| Error::with_path(kind, format!("ops[{i}].kind")))?,
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
    Ok(InsertCrossRefOp {
        id,
        para,
        bookmark,
        kind,
    })
}

fn parse_delete_cross_ref(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteCrossRefOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_op_id(obj, i)?;
    let n = parse_xref_id(&id, i)?;
    Ok(DeleteCrossRefOp { id, n })
}

fn parse_insert_index(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertIndexOp, Error> {
    deny_keys(obj, i, &["op", "id", "instr"])?;
    let id = require_op_id(obj, i)?;
    let instr = match obj.get("instr") {
        None => super::field::default_index_instr().to_string(),
        Some(Value::String(s)) => {
            if s.trim().is_empty() {
                return Err(Error::with_path(
                    ErrorKind::EmptyFieldInstr,
                    format!("ops[{i}].instr"),
                ));
            }
            if !super::field::instr_is_index(s) {
                return Err(Error::with_path(
                    ErrorKind::InvalidIndexInstr,
                    format!("ops[{i}].instr"),
                ));
            }
            s.clone()
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "instr".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].instr"),
            ));
        }
    };
    let path = format!("ops[{i}].id");
    let place = if id == "docx:document" {
        InsertIndexPlace::EndOfDocument
    } else {
        match para::parse_para_id_at(&id, &path)? {
            ParaTarget::Body(p) => InsertIndexPlace::After(p),
            _ => {
                return Err(Error::with_path(
                    ErrorKind::InvalidId {
                        reason: IdReason::InsertIndexNeedsBodyPara,
                    },
                    path,
                ));
            }
        }
    };
    Ok(InsertIndexOp { id, place, instr })
}

fn parse_delete_index(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteIndexOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_op_id(obj, i)?;
    let n = parse_index_id(&id, i)?;
    Ok(DeleteIndexOp { id, n })
}

fn parse_insert_index_entry(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertIndexEntryOp, Error> {
    deny_keys(obj, i, &["op", "id", "entry"])?;
    let id = require_op_id(obj, i)?;
    let para = parse_story_para_at(
        &id,
        &format!("ops[{i}].id"),
        IdReason::InsertIndexEntryNeedsPara,
    )?;
    let entry = match obj.get("entry") {
        Some(Value::String(s)) => {
            super::field::parse_index_entry(s)
                .map_err(|kind| Error::with_path(kind, format!("ops[{i}].entry")))?;
            s.clone()
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "entry".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].entry"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::OpRequiresField {
                    op: "insertIndexEntry".into(),
                    field: "entry".into(),
                },
                format!("ops[{i}].entry"),
            ));
        }
    };
    Ok(InsertIndexEntryOp { id, para, entry })
}

fn parse_delete_index_entry(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteIndexEntryOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_op_id(obj, i)?;
    let n = parse_xe_id(&id, i)?;
    Ok(DeleteIndexEntryOp { id, n })
}

fn parse_insert_source(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertSourceOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op",
            "id",
            "tag",
            "sourceType",
            "title",
            "year",
            "last",
            "first",
            "city",
            "publisher",
        ],
    )?;
    let id = require_op_id(obj, i)?;
    if id != "docx:document" {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::InsertSourceNeedsDocument,
            },
            format!("ops[{i}].id"),
        ));
    }
    let tag = require_string_field(obj, i, "insertSource", "tag")?;
    if super::biblio::tag_invalid(&tag) {
        return Err(Error::with_path(
            ErrorKind::InvalidSourceTag,
            format!("ops[{i}].tag"),
        ));
    }
    let source_type = match obj.get("sourceType") {
        None => super::biblio::DEFAULT_SOURCE_TYPE.to_string(),
        Some(Value::String(s)) => {
            if !super::biblio::source_type_ok(s) {
                return Err(Error::with_path(
                    ErrorKind::InvalidSourceType,
                    format!("ops[{i}].sourceType"),
                ));
            }
            s.clone()
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "sourceType".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].sourceType"),
            ));
        }
    };
    let title = require_string_field(obj, i, "insertSource", "title")?;
    if title.is_empty() || title.chars().all(char::is_whitespace) {
        return Err(Error::with_path(
            ErrorKind::EmptySourceTitle,
            format!("ops[{i}].title"),
        ));
    }
    Ok(InsertSourceOp {
        tag,
        source_type,
        title,
        year: optional_string(obj, i, "year")?,
        last: optional_string(obj, i, "last")?,
        first: optional_string(obj, i, "first")?,
        city: optional_string(obj, i, "city")?,
        publisher: optional_string(obj, i, "publisher")?,
    })
}

fn parse_delete_source(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteSourceOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_op_id(obj, i)?;
    let n = parse_source_id(&id, i)?;
    Ok(DeleteSourceOp { id, n })
}

fn parse_insert_citation(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertCitationOp, Error> {
    deny_keys(obj, i, &["op", "id", "tag"])?;
    let id = require_op_id(obj, i)?;
    let para = parse_story_para_at(
        &id,
        &format!("ops[{i}].id"),
        IdReason::InsertCitationNeedsPara,
    )?;
    let tag = require_string_field(obj, i, "insertCitation", "tag")?;
    if super::biblio::tag_invalid(&tag) {
        return Err(Error::with_path(
            ErrorKind::InvalidCitationTag,
            format!("ops[{i}].tag"),
        ));
    }
    Ok(InsertCitationOp { id, para, tag })
}

fn parse_delete_citation(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteCitationOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_op_id(obj, i)?;
    let n = parse_cite_id(&id, i)?;
    Ok(DeleteCitationOp { id, n })
}

fn parse_insert_bibliography(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertBibliographyOp, Error> {
    deny_keys(obj, i, &["op", "id", "instr"])?;
    let id = require_op_id(obj, i)?;
    let instr = match obj.get("instr") {
        None => super::field::default_bibliography_instr().to_string(),
        Some(Value::String(s)) => {
            if s.trim().is_empty() {
                return Err(Error::with_path(
                    ErrorKind::EmptyFieldInstr,
                    format!("ops[{i}].instr"),
                ));
            }
            if !super::field::instr_is_bibliography(s) {
                return Err(Error::with_path(
                    ErrorKind::InvalidBibliographyInstr,
                    format!("ops[{i}].instr"),
                ));
            }
            s.clone()
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "instr".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].instr"),
            ));
        }
    };
    let path = format!("ops[{i}].id");
    let place = if id == "docx:document" {
        InsertBibliographyPlace::EndOfDocument
    } else {
        match para::parse_para_id_at(&id, &path)? {
            ParaTarget::Body(p) => InsertBibliographyPlace::After(p),
            _ => {
                return Err(Error::with_path(
                    ErrorKind::InvalidId {
                        reason: IdReason::InsertBibliographyNeedsBodyPara,
                    },
                    path,
                ));
            }
        }
    };
    Ok(InsertBibliographyOp { id, place, instr })
}

fn parse_delete_bibliography(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteBibliographyOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_op_id(obj, i)?;
    let n = parse_biblio_id(&id, i)?;
    Ok(DeleteBibliographyOp { id, n })
}

fn parse_set_bibliography_style(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetBibliographyStyleOp, Error> {
    deny_keys(obj, i, &["op", "id", "styleName"])?;
    let id = require_op_id(obj, i)?;
    require_document_id(&id, i, IdReason::SetBibliographyStyleNeedsDocument)?;
    let style_name = require_string_field(obj, i, "setBibliographyStyle", "styleName")?;
    if !super::biblio::style_ok(&style_name) {
        return Err(Error::with_path(
            ErrorKind::InvalidBibliographyStyle,
            format!("ops[{i}].styleName"),
        ));
    }
    Ok(SetBibliographyStyleOp { style_name })
}

fn parse_insert_doc_part(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertDocPartOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op", "id", "name", "text", "gallery", "category", "behavior",
        ],
    )?;
    let id = require_op_id(obj, i)?;
    if id != "docx:document" {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::InsertDocPartNeedsDocument,
            },
            format!("ops[{i}].id"),
        ));
    }
    let name = require_string_field(obj, i, "insertDocPart", "name")?;
    if name.is_empty() || name.chars().all(char::is_whitespace) {
        return Err(Error::with_path(
            ErrorKind::EmptyDocPartName,
            format!("ops[{i}].name"),
        ));
    }
    let text = require_string_field(obj, i, "insertDocPart", "text")?;
    if text.is_empty() || text.chars().all(char::is_whitespace) {
        return Err(Error::with_path(
            ErrorKind::EmptyDocPartText,
            format!("ops[{i}].text"),
        ));
    }
    let gallery = match obj.get("gallery") {
        None => super::glossary::DEFAULT_GALLERY.to_string(),
        Some(Value::String(s)) => {
            if !super::glossary::gallery_ok(s) {
                return Err(Error::with_path(
                    ErrorKind::InvalidDocPartGallery,
                    format!("ops[{i}].gallery"),
                ));
            }
            s.clone()
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "gallery".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].gallery"),
            ));
        }
    };
    let category = match obj.get("category") {
        None => super::glossary::DEFAULT_CATEGORY.to_string(),
        Some(Value::String(s)) => {
            if s.is_empty() || s.chars().all(char::is_whitespace) {
                return Err(Error::with_path(
                    ErrorKind::EmptyDocPartName,
                    format!("ops[{i}].category"),
                ));
            }
            s.clone()
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "category".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].category"),
            ));
        }
    };
    let behavior = match obj.get("behavior") {
        None => super::glossary::DEFAULT_BEHAVIOR.to_string(),
        Some(Value::String(s)) => {
            if !super::glossary::behavior_ok(s) {
                return Err(Error::with_path(
                    ErrorKind::InvalidDocPartBehavior,
                    format!("ops[{i}].behavior"),
                ));
            }
            s.clone()
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "behavior".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].behavior"),
            ));
        }
    };
    Ok(InsertDocPartOp {
        name,
        gallery,
        category,
        behavior,
        text,
    })
}

fn parse_delete_doc_part(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteDocPartOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_op_id(obj, i)?;
    let n = parse_doc_part_id(&id, i)?;
    Ok(DeleteDocPartOp { id, n })
}

fn parse_insert_doc_part_content(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertDocPartContentOp, Error> {
    deny_keys(obj, i, &["op", "id", "docPart"])?;
    let id = require_op_id(obj, i)?;
    let part_id = match obj.get("docPart") {
        Some(Value::String(s)) => s.clone(),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "docPart".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].docPart"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::OpRequiresField {
                    op: "insertDocPartContent".into(),
                    field: "docPart".into(),
                },
                format!("ops[{i}].docPart"),
            ));
        }
    };
    let n = parse_doc_part_id_at(&part_id, &format!("ops[{i}].docPart"))?;
    let path = format!("ops[{i}].id");
    let place = if id == "docx:document" {
        InsertDocPartContentPlace::EndOfDocument
    } else {
        match para::parse_para_id_at(&id, &path)? {
            ParaTarget::Body(p) => InsertDocPartContentPlace::After(p),
            _ => {
                return Err(Error::with_path(
                    ErrorKind::InvalidId {
                        reason: IdReason::InsertDocPartContentNeedsBodyPara,
                    },
                    path,
                ));
            }
        }
    };
    Ok(InsertDocPartContentOp {
        id,
        place,
        n,
        part_id,
    })
}

fn parse_insert_embedded_font(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertEmbeddedFontOp, Error> {
    deny_keys(obj, i, &["op", "id", "name", "file", "style"])?;
    let id = require_op_id(obj, i)?;
    if id != "docx:document" {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::InsertEmbeddedFontNeedsDocument,
            },
            format!("ops[{i}].id"),
        ));
    }
    let name = require_string_field(obj, i, "insertEmbeddedFont", "name")?;
    if name.is_empty() || name.chars().all(char::is_whitespace) {
        return Err(Error::with_path(
            ErrorKind::EmptyFontName,
            format!("ops[{i}].name"),
        ));
    }
    let file = require_string_field(obj, i, "insertEmbeddedFont", "file")?;
    if file.is_empty() || file.chars().all(char::is_whitespace) {
        return Err(Error::with_path(
            ErrorKind::InvalidFontFile,
            format!("ops[{i}].file"),
        ));
    }
    let style = match obj.get("style") {
        None => super::font_embed::FontStyle::Regular,
        Some(Value::String(s)) => super::font_embed::FontStyle::parse(s).ok_or_else(|| {
            Error::with_path(ErrorKind::InvalidFontStyle, format!("ops[{i}].style"))
        })?,
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "style".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].style"),
            ));
        }
    };
    Ok(InsertEmbeddedFontOp { name, style, file })
}

fn parse_delete_embedded_font(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteEmbeddedFontOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_op_id(obj, i)?;
    let n = super::font_embed::parse_id(&id, &format!("ops[{i}].id"))?;
    Ok(DeleteEmbeddedFontOp { id, n })
}

fn parse_set_font_embed(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetFontEmbedOp, Error> {
    deny_keys(obj, i, &["op", "id", "trueType", "systemFonts", "subset"])?;
    let id = require_op_id(obj, i)?;
    if id != "docx:document" {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SetFontEmbedNeedsDocument,
            },
            format!("ops[{i}].id"),
        ));
    }
    let true_type = parse_optional_bool_field(obj.get("trueType"), i, "trueType")?;
    let system_fonts = parse_optional_bool_field(obj.get("systemFonts"), i, "systemFonts")?;
    let subset = parse_optional_bool_field(obj.get("subset"), i, "subset")?;
    if true_type.is_none() && system_fonts.is_none() && subset.is_none() {
        return Err(Error::with_path(
            ErrorKind::NeedFontEmbedField,
            format!("ops[{i}]"),
        ));
    }
    Ok(SetFontEmbedOp {
        true_type,
        system_fonts,
        subset,
    })
}

fn parse_set_page_background(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetPageBackgroundOp, Error> {
    deny_keys(obj, i, &["op", "id", "color", "fill"])?;
    let id = require_op_id(obj, i)?;
    require_document_id(&id, i, IdReason::SetPageBackgroundNeedsDocument)?;
    let color = match obj.get("color") {
        None => {
            return Err(Error::with_path(
                ErrorKind::OpRequiresField {
                    op: "setPageBackground".into(),
                    field: "color".into(),
                },
                format!("ops[{i}].color"),
            ));
        }
        Some(Value::Null) => None,
        Some(Value::String(s)) => {
            let parsed = super::page_bg::parse_color(s).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::InvalidPageBackgroundColor,
                    format!("ops[{i}].color"),
                )
            })?;
            Some(parsed)
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "color".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].color"),
            ));
        }
    };
    let fill = match obj.get("fill") {
        None => None,
        Some(v) => Some(super::page_bg::parse_fill(v, &format!("ops[{i}].fill"))?),
    };
    if color.is_none() && fill.is_some() {
        return Err(Error::with_path(
            ErrorKind::InvalidPageBackgroundFill,
            format!("ops[{i}].fill"),
        ));
    }
    Ok(SetPageBackgroundOp { color, fill })
}

fn parse_set_page_borders(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetPageBordersOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op",
            "id",
            "enabled",
            "offsetFrom",
            "display",
            "zOrder",
            "top",
            "left",
            "bottom",
            "right",
        ],
    )?;
    let id = require_op_id(obj, i)?;
    let n = parse_section_id(&id, i)?;
    let offset_from = parse_page_border_enum(
        obj.get("offsetFrom"),
        i,
        "offsetFrom",
        ErrorKind::InvalidPageBorderOffset,
        &["page", "text"],
    )?;
    let display = parse_page_border_enum(
        obj.get("display"),
        i,
        "display",
        ErrorKind::InvalidPageBorderDisplay,
        &["allPages", "firstPage", "notFirstPage"],
    )?;
    let z_order = parse_page_border_enum(
        obj.get("zOrder"),
        i,
        "zOrder",
        ErrorKind::InvalidPageBorderZOrder,
        &["front", "back"],
    )?;
    let top = parse_border_side(obj.get("top"), i, "top")?;
    let left = parse_border_side(obj.get("left"), i, "left")?;
    let bottom = parse_border_side(obj.get("bottom"), i, "bottom")?;
    let right = parse_border_side(obj.get("right"), i, "right")?;
    let other = offset_from.is_some()
        || display.is_some()
        || z_order.is_some()
        || top.is_some()
        || left.is_some()
        || bottom.is_some()
        || right.is_some();
    let enabled = parse_enabled_false(obj, i, other)?;
    if enabled.is_none() && !other {
        return Err(Error::with_path(
            ErrorKind::NeedPageBordersField,
            format!("ops[{i}]"),
        ));
    }
    Ok(SetPageBordersOp {
        id,
        n,
        enabled,
        offset_from,
        display,
        z_order,
        top,
        left,
        bottom,
        right,
    })
}

fn parse_set_form_prot(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetFormProtOp, Error> {
    deny_keys(obj, i, &["op", "id", "formProt"])?;
    let id = require_op_id(obj, i)?;
    let n = parse_section_id(&id, i)?;
    let form_prot = match parse_opt_null_bool(obj, i, "formProt")? {
        Some(v) => v,
        None => {
            return Err(Error::with_path(
                ErrorKind::OpRequiresField {
                    op: "setFormProt".into(),
                    field: "formProt".into(),
                },
                format!("ops[{i}].formProt"),
            ));
        }
    };
    Ok(SetFormProtOp { id, n, form_prot })
}

fn parse_page_border_enum(
    v: Option<&Value>,
    i: usize,
    field: &str,
    kind: ErrorKind,
    allowed: &[&str],
) -> Result<Option<String>, Error> {
    match v {
        None => Ok(None),
        Some(Value::String(s)) if allowed.contains(&s.as_str()) => Ok(Some(s.clone())),
        Some(Value::String(_)) => Err(Error::with_path(kind, format!("ops[{i}].{field}"))),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].{field}"),
        )),
    }
}

fn parse_border_side(
    v: Option<&Value>,
    i: usize,
    field: &str,
) -> Result<Option<Option<super::page_bg::BorderSide>>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(None)),
        Some(Value::Object(map)) => {
            for k in map.keys() {
                if !["val", "sz", "space", "color"].contains(&k.as_str()) {
                    return Err(Error::with_path(
                        ErrorKind::UnknownField { field: k.clone() },
                        format!("ops[{i}].{field}.{k}"),
                    ));
                }
            }
            let val = match map.get("val") {
                Some(Value::String(s)) => super::page_bg::parse_border_val(s).ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::InvalidPageBorderVal,
                        format!("ops[{i}].{field}.val"),
                    )
                })?,
                Some(_) => {
                    return Err(Error::with_path(
                        ErrorKind::WrongType {
                            field: format!("{field}.val"),
                            expected: TypeName::String,
                        },
                        format!("ops[{i}].{field}.val"),
                    ));
                }
                None => {
                    return Err(Error::with_path(
                        ErrorKind::OpRequiresField {
                            op: "setPageBorders".into(),
                            field: "val".into(),
                        },
                        format!("ops[{i}].{field}.val"),
                    ));
                }
            };
            let sz = parse_twip_field(map.get("sz"), i, &format!("{field}.sz"))?;
            let space = parse_twip_field(map.get("space"), i, &format!("{field}.space"))?;
            let color = match map.get("color") {
                None => None,
                Some(Value::String(s)) => {
                    Some(super::page_bg::parse_side_color(s).ok_or_else(|| {
                        Error::with_path(
                            ErrorKind::InvalidPageBorderColor,
                            format!("ops[{i}].{field}.color"),
                        )
                    })?)
                }
                Some(_) => {
                    return Err(Error::with_path(
                        ErrorKind::WrongType {
                            field: format!("{field}.color"),
                            expected: TypeName::String,
                        },
                        format!("ops[{i}].{field}.color"),
                    ));
                }
            };
            Ok(Some(Some(super::page_bg::BorderSide {
                val: val.to_string(),
                sz,
                space,
                color,
            })))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Object,
            },
            format!("ops[{i}].{field}"),
        )),
    }
}

fn parse_doc_part_id(id: &str, op_index: usize) -> Result<usize, Error> {
    parse_doc_part_id_at(id, &format!("ops[{op_index}].id"))
}

fn parse_doc_part_id_at(id: &str, path: &str) -> Result<usize, Error> {
    let rest = id.strip_prefix("docPart:").ok_or_else(|| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::DocPartPrefix,
            },
            path.to_string(),
        )
    })?;
    let n: usize = rest.parse().map_err(|_| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::DocPartIndexNotInt,
            },
            path.to_string(),
        )
    })?;
    if n < 1 {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::DocPartIndexFromOne,
            },
            path.to_string(),
        ));
    }
    Ok(n)
}

fn optional_string(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    field: &str,
) -> Result<Option<String>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::String(s)) => {
            if s.is_empty() {
                Ok(None)
            } else {
                Ok(Some(s.clone()))
            }
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].{field}"),
        )),
    }
}

fn require_string_field(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    op: &str,
    field: &str,
) -> Result<String, Error> {
    match obj.get(field) {
        Some(Value::String(s)) => Ok(s.clone()),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::String,
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

fn parse_insert_math(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertMathOp, Error> {
    deny_keys(obj, i, &["op", "id", "kind", "text", "expr", "jc"])?;
    let id = require_op_id(obj, i)?;
    let kind = match obj.get("kind") {
        None => super::math::MathKind::Inline,
        Some(Value::String(s)) if s == "inline" => super::math::MathKind::Inline,
        Some(Value::String(s)) if s == "display" => super::math::MathKind::Display,
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidMathKind,
                format!("ops[{i}].kind"),
            ));
        }
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
    let jc = match obj.get("jc") {
        None => None,
        Some(Value::String(s)) if super::math::MATH_JC.contains(&s.as_str()) => Some(s.clone()),
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidMathJc,
                format!("ops[{i}].jc"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "jc".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].jc"),
            ));
        }
    };
    if jc.is_some() && kind == super::math::MathKind::Inline {
        return Err(Error::with_path(
            ErrorKind::MathJcOnInline,
            format!("ops[{i}].jc"),
        ));
    }
    let body = super::math_expr::parse_math_body(obj, &format!("ops[{i}]"))?;
    let path = format!("ops[{i}].id");
    let place = if id == "docx:document" {
        InsertMathPlace::EndOfDocument
    } else {
        InsertMathPlace::After(parse_story_or_box_para_at(
            &id,
            &path,
            IdReason::InsertMathNeedsPara,
        )?)
    };
    Ok(InsertMathOp {
        id,
        place,
        kind,
        body,
        jc,
    })
}

fn parse_set_math(obj: &serde_json::Map<String, Value>, i: usize) -> Result<SetMathOp, Error> {
    deny_keys(obj, i, &["op", "id", "text", "expr", "jc"])?;
    let id = require_op_id(obj, i)?;
    let target = super::math::parse_math_ref(&id, &format!("ops[{i}].id"))?;
    let jc = parse_opt_null_enum(obj, i, "jc", super::math::MATH_JC, ErrorKind::InvalidMathJc)?;
    let has_text = obj.contains_key("text");
    let has_expr = obj.contains_key("expr");
    let body = if has_text || has_expr {
        Some(super::math_expr::parse_math_body(
            obj,
            &format!("ops[{i}]"),
        )?)
    } else if jc.is_some() {
        None
    } else {
        return Err(Error::with_path(
            ErrorKind::MathNeedTextOrExpr,
            format!("ops[{i}]"),
        ));
    };
    Ok(SetMathOp {
        id,
        target,
        body,
        jc,
    })
}

fn parse_delete_math(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteMathOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_op_id(obj, i)?;
    let target = super::math::parse_math_ref(&id, &format!("ops[{i}].id"))?;
    Ok(DeleteMathOp { id, target })
}

fn parse_insert_ruby(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertRubyOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op",
            "id",
            "base",
            "rt",
            "align",
            "hps",
            "hpsRaise",
            "hpsBaseText",
            "lid",
        ],
    )?;
    let id = require_op_id(obj, i)?;
    let base = require_nonempty(obj, i, "insertRuby", "base", ErrorKind::EmptyRubyBase)?;
    let rt = require_nonempty(obj, i, "insertRuby", "rt", ErrorKind::EmptyRubyRt)?;
    let align = match obj.get("align") {
        None => "distributeSpace".to_string(),
        Some(Value::String(s)) => {
            if !super::ruby::ALIGN_SET.contains(&s.as_str()) {
                return Err(Error::with_path(
                    ErrorKind::InvalidRubyAlign,
                    format!("ops[{i}].align"),
                ));
            }
            s.clone()
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "align".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].align"),
            ));
        }
    };
    let hps = parse_opt_u32_range(obj, i, "hps", 2, 3276)?.unwrap_or(12);
    let hps_raise = parse_opt_u32_range(obj, i, "hpsRaise", 2, 3276)?.unwrap_or(hps);
    let hps_base_text = parse_opt_u32_range(obj, i, "hpsBaseText", 2, 3276)?.unwrap_or(24);
    let lid = match obj.get("lid") {
        None => "zh-CN".to_string(),
        Some(Value::String(s)) => {
            if s.is_empty() {
                return Err(Error::with_path(
                    ErrorKind::EmptyRubyLid,
                    format!("ops[{i}].lid"),
                ));
            }
            s.clone()
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "lid".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].lid"),
            ));
        }
    };
    let path = format!("ops[{i}].id");
    let place = if id == "docx:document" {
        InsertRubyPlace::EndOfDocument
    } else {
        InsertRubyPlace::After(parse_story_or_box_para_at(
            &id,
            &path,
            IdReason::InsertRubyNeedsPara,
        )?)
    };
    Ok(InsertRubyOp {
        id,
        place,
        base,
        rt,
        align,
        hps,
        hps_raise,
        hps_base_text,
        lid,
    })
}

fn parse_set_ruby(obj: &serde_json::Map<String, Value>, i: usize) -> Result<SetRubyOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op",
            "id",
            "base",
            "rt",
            "align",
            "hps",
            "hpsRaise",
            "hpsBaseText",
            "lid",
        ],
    )?;
    let id = require_op_id(obj, i)?;
    let target = super::ruby::parse_ruby_ref(&id, &format!("ops[{i}].id"))?;
    let base = parse_opt_nonempty(obj, i, "base", ErrorKind::EmptyRubyBase)?;
    let rt = parse_opt_nonempty(obj, i, "rt", ErrorKind::EmptyRubyRt)?;
    let align = parse_opt_null_enum(
        obj,
        i,
        "align",
        super::ruby::ALIGN_SET,
        ErrorKind::InvalidRubyAlign,
    )?;
    let align = match align {
        Some(Some(s)) => Some(s),
        Some(None) => {
            return Err(Error::with_path(
                ErrorKind::InvalidRubyAlign,
                format!("ops[{i}].align"),
            ));
        }
        None => None,
    };
    let hps = parse_opt_u32_range(obj, i, "hps", 2, 3276)?;
    let hps_raise = parse_opt_u32_range(obj, i, "hpsRaise", 2, 3276)?;
    let hps_base_text = parse_opt_u32_range(obj, i, "hpsBaseText", 2, 3276)?;
    let lid = parse_opt_nonempty(obj, i, "lid", ErrorKind::EmptyRubyLid)?;
    if base.is_none()
        && rt.is_none()
        && align.is_none()
        && hps.is_none()
        && hps_raise.is_none()
        && hps_base_text.is_none()
        && lid.is_none()
    {
        return Err(Error::with_path(
            ErrorKind::NeedSetRubyField,
            format!("ops[{i}]"),
        ));
    }
    Ok(SetRubyOp {
        id,
        target,
        base,
        rt,
        align,
        hps,
        hps_raise,
        hps_base_text,
        lid,
    })
}

fn parse_delete_ruby(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteRubyOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_op_id(obj, i)?;
    let target = super::ruby::parse_ruby_ref(&id, &format!("ops[{i}].id"))?;
    Ok(DeleteRubyOp { id, target })
}

fn parse_set_east_asian_settings(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetEastAsianSettingsOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op",
            "id",
            "characterSpacingControl",
            "strictFirstAndLastChars",
            "noLineBreaksBefore",
            "noLineBreaksAfter",
        ],
    )?;
    let id = require_op_id(obj, i)?;
    require_document_id(&id, i, IdReason::SetEastAsianSettingsNeedsDocument)?;
    let character_spacing = parse_opt_null_enum(
        obj,
        i,
        "characterSpacingControl",
        super::east_asia::CHARACTER_SPACING_SET,
        ErrorKind::InvalidCharacterSpacing,
    )?;
    let strict = parse_opt_null_bool(obj, i, "strictFirstAndLastChars")?;
    let no_before = parse_opt_kinsoku(obj, i, "noLineBreaksBefore")?;
    let no_after = parse_opt_kinsoku(obj, i, "noLineBreaksAfter")?;
    if character_spacing.is_none() && strict.is_none() && no_before.is_none() && no_after.is_none()
    {
        return Err(Error::with_path(
            ErrorKind::NeedSetEastAsianSettingsField,
            format!("ops[{i}]"),
        ));
    }
    Ok(SetEastAsianSettingsOp {
        character_spacing,
        strict,
        no_before,
        no_after,
    })
}

fn parse_opt_kinsoku(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    field: &str,
) -> Result<Option<Option<super::east_asia::KinsokuChars>>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(None)),
        Some(Value::Object(inner)) => {
            for k in inner.keys() {
                if !matches!(k.as_str(), "lang" | "chars") {
                    return Err(Error::with_path(
                        ErrorKind::UnknownField { field: k.clone() },
                        format!("ops[{i}].{field}.{k}"),
                    ));
                }
            }
            let lang = match inner.get("lang") {
                Some(Value::String(s)) if !s.is_empty() => s.clone(),
                Some(Value::String(_)) => {
                    return Err(Error::with_path(
                        ErrorKind::EmptyKinsokuLang,
                        format!("ops[{i}].{field}.lang"),
                    ));
                }
                Some(_) => {
                    return Err(Error::with_path(
                        ErrorKind::WrongType {
                            field: "lang".into(),
                            expected: TypeName::String,
                        },
                        format!("ops[{i}].{field}.lang"),
                    ));
                }
                None => {
                    return Err(Error::with_path(
                        ErrorKind::EmptyKinsokuLang,
                        format!("ops[{i}].{field}.lang"),
                    ));
                }
            };
            let chars = match inner.get("chars") {
                Some(Value::String(s)) if !s.is_empty() => s.clone(),
                Some(Value::String(_)) => {
                    return Err(Error::with_path(
                        ErrorKind::EmptyKinsokuChars,
                        format!("ops[{i}].{field}.chars"),
                    ));
                }
                Some(_) => {
                    return Err(Error::with_path(
                        ErrorKind::WrongType {
                            field: "chars".into(),
                            expected: TypeName::String,
                        },
                        format!("ops[{i}].{field}.chars"),
                    ));
                }
                None => {
                    return Err(Error::with_path(
                        ErrorKind::EmptyKinsokuChars,
                        format!("ops[{i}].{field}.chars"),
                    ));
                }
            };
            Ok(Some(Some(super::east_asia::KinsokuChars { lang, chars })))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Object,
            },
            format!("ops[{i}].{field}"),
        )),
    }
}

fn require_nonempty(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    op: &str,
    field: &str,
    empty: ErrorKind,
) -> Result<String, Error> {
    match obj.get(field) {
        Some(Value::String(s)) if !s.is_empty() => Ok(s.clone()),
        Some(Value::String(_)) => Err(Error::with_path(empty, format!("ops[{i}].{field}"))),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::String,
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

fn parse_opt_nonempty(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    field: &str,
    empty: ErrorKind,
) -> Result<Option<String>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::String(s)) => {
            if s.is_empty() {
                return Err(Error::with_path(empty, format!("ops[{i}].{field}")));
            }
            Ok(Some(s.clone()))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].{field}"),
        )),
    }
}

fn parse_insert_sdt(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertContentControlOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op", "id", "kind", "text", "tag", "alias", "items", "fullDate", "file", "checked",
        ],
    )?;
    let id = require_op_id(obj, i)?;
    let kind = match obj.get("kind") {
        None => super::sdt::SdtKind::PlainText,
        Some(Value::String(s)) => super::sdt::SdtKind::parse(s).ok_or_else(|| {
            Error::with_path(
                ErrorKind::InvalidContentControlKind,
                format!("ops[{i}].kind"),
            )
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
    };
    let text_opt = parse_sdt_text_opt(obj, i)?;
    let tag = optional_op_string(obj, i, "tag")?;
    let alias = optional_op_string(obj, i, "alias")?;
    let items = parse_sdt_items(obj, i)?;
    if !items.is_empty()
        && !matches!(
            kind,
            super::sdt::SdtKind::DropDown | super::sdt::SdtKind::ComboBox
        )
    {
        return Err(Error::with_path(
            ErrorKind::ContentControlFieldMismatch,
            format!("ops[{i}].items"),
        ));
    }
    let full_date = optional_op_string(obj, i, "fullDate")?;
    if full_date.is_some() && kind != super::sdt::SdtKind::Date {
        return Err(Error::with_path(
            ErrorKind::ContentControlFieldMismatch,
            format!("ops[{i}].fullDate"),
        ));
    }
    if let Some(fd) = &full_date {
        if fd.is_empty() {
            return Err(Error::with_path(
                ErrorKind::EmptyContentControlText,
                format!("ops[{i}].fullDate"),
            ));
        }
    }
    let file = match obj.get("file") {
        None => None,
        Some(_) => Some(require_file(obj, i, "insertContentControl")?),
    };
    if file.is_some() && kind != super::sdt::SdtKind::Picture {
        return Err(Error::with_path(
            ErrorKind::ContentControlFieldMismatch,
            format!("ops[{i}].file"),
        ));
    }
    let checked = parse_optional_bool_field(obj.get("checked"), i, "checked")?;
    if checked.is_some() && kind != super::sdt::SdtKind::Checkbox {
        return Err(Error::with_path(
            ErrorKind::ContentControlFieldMismatch,
            format!("ops[{i}].checked"),
        ));
    }
    if kind.needs_text() {
        parse_sdt_text(obj, i, "insertContentControl")?;
    } else if text_opt.is_some() {
        return Err(Error::with_path(
            ErrorKind::ContentControlFieldMismatch,
            format!("ops[{i}].text"),
        ));
    }
    if kind == super::sdt::SdtKind::Picture && file.is_none() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "insertContentControl".into(),
                field: "file".into(),
            },
            format!("ops[{i}].file"),
        ));
    }
    let text = text_opt.unwrap_or_default();
    let path = format!("ops[{i}].id");
    let place = if id == "docx:document" {
        InsertSdtPlace::EndOfDocument
    } else {
        InsertSdtPlace::After(parse_story_or_box_para_at(
            &id,
            &path,
            IdReason::InsertSdtNeedsPara,
        )?)
    };
    Ok(InsertContentControlOp {
        id,
        place,
        kind,
        text,
        tag,
        alias,
        items,
        full_date,
        file,
        checked: checked.unwrap_or(false),
    })
}

fn parse_set_sdt(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetContentControlOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op", "id", "text", "tag", "alias", "items", "fullDate", "file", "checked",
        ],
    )?;
    let id = require_op_id(obj, i)?;
    let target = super::sdt::parse_sdt_ref(&id, &format!("ops[{i}].id"))?;
    let text = parse_sdt_text_opt(obj, i)?;
    let tag = optional_op_string(obj, i, "tag")?;
    let alias = optional_op_string(obj, i, "alias")?;
    let items = if obj.contains_key("items") {
        Some(parse_sdt_items(obj, i)?)
    } else {
        None
    };
    let full_date = optional_op_string(obj, i, "fullDate")?;
    let file = match obj.get("file") {
        None => None,
        Some(_) => Some(require_file(obj, i, "setContentControl")?),
    };
    let checked = parse_optional_bool_field(obj.get("checked"), i, "checked")?;
    if file.is_some() && text.is_some() {
        return Err(Error::with_path(
            ErrorKind::ContentControlFieldMismatch,
            format!("ops[{i}].file"),
        ));
    }
    if checked.is_some() && (text.is_some() || file.is_some()) {
        return Err(Error::with_path(
            ErrorKind::ContentControlFieldMismatch,
            format!("ops[{i}].checked"),
        ));
    }
    Ok(SetContentControlOp {
        id,
        target,
        text,
        tag,
        alias,
        items,
        full_date,
        file,
        checked,
    })
}

fn parse_delete_sdt(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteContentControlOp, Error> {
    deny_keys(obj, i, &["op", "id", "keep"])?;
    let id = require_op_id(obj, i)?;
    let target = super::sdt::parse_sdt_ref(&id, &format!("ops[{i}].id"))?;
    let keep = match obj.get("keep") {
        None => super::sdt::SdtKeep::Content,
        Some(Value::String(s)) => super::sdt::SdtKeep::parse(s).ok_or_else(|| {
            Error::with_path(
                ErrorKind::InvalidContentControlKeep,
                format!("ops[{i}].keep"),
            )
        })?,
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "keep".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].keep"),
            ));
        }
    };
    Ok(DeleteContentControlOp { id, target, keep })
}

fn parse_sdt_text(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    op: &str,
) -> Result<String, Error> {
    match parse_sdt_text_opt(obj, i)? {
        Some(s) => Ok(s),
        None => Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: op.into(),
                field: "text".into(),
            },
            format!("ops[{i}].text"),
        )),
    }
}

fn parse_sdt_text_opt(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<Option<String>, Error> {
    match obj.get("text") {
        Some(Value::String(s)) => {
            if s.is_empty() {
                return Err(Error::with_path(
                    ErrorKind::EmptyContentControlText,
                    format!("ops[{i}].text"),
                ));
            }
            Ok(Some(s.clone()))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "text".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].text"),
        )),
        None => Ok(None),
    }
}

fn parse_sdt_items(obj: &serde_json::Map<String, Value>, i: usize) -> Result<Vec<String>, Error> {
    match obj.get("items") {
        None => Ok(Vec::new()),
        Some(Value::Array(arr)) => {
            if arr.is_empty() {
                return Err(Error::with_path(
                    ErrorKind::InvalidContentControlItems,
                    format!("ops[{i}].items"),
                ));
            }
            let mut out = Vec::new();
            for (j, v) in arr.iter().enumerate() {
                let s = v.as_str().ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::WrongType {
                            field: "items".into(),
                            expected: TypeName::String,
                        },
                        format!("ops[{i}].items[{j}]"),
                    )
                })?;
                if s.is_empty() {
                    return Err(Error::with_path(
                        ErrorKind::InvalidContentControlItems,
                        format!("ops[{i}].items[{j}]"),
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
            format!("ops[{i}].items"),
        )),
    }
}

fn parse_toc_id(id: &str, op_index: usize) -> Result<usize, Error> {
    let path = format!("ops[{op_index}].id");
    let rest = id.strip_prefix("toc:").ok_or_else(|| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::TocPrefix,
            },
            path.clone(),
        )
    })?;
    let n: usize = rest.parse().map_err(|_| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::TocIndexNotInt,
            },
            path.clone(),
        )
    })?;
    if n < 1 {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::TocIndexFromOne,
            },
            path,
        ));
    }
    Ok(n)
}

fn parse_caption_id(id: &str, op_index: usize) -> Result<usize, Error> {
    let path = format!("ops[{op_index}].id");
    let rest = id.strip_prefix("caption:").ok_or_else(|| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::CaptionPrefix,
            },
            path.clone(),
        )
    })?;
    let n: usize = rest.parse().map_err(|_| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::CaptionIndexNotInt,
            },
            path.clone(),
        )
    })?;
    if n < 1 {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::CaptionIndexFromOne,
            },
            path,
        ));
    }
    Ok(n)
}

fn parse_xref_id(id: &str, op_index: usize) -> Result<usize, Error> {
    let path = format!("ops[{op_index}].id");
    let rest = id.strip_prefix("xref:").ok_or_else(|| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::XrefPrefix,
            },
            path.clone(),
        )
    })?;
    let n: usize = rest.parse().map_err(|_| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::XrefIndexNotInt,
            },
            path.clone(),
        )
    })?;
    if n < 1 {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::XrefIndexFromOne,
            },
            path,
        ));
    }
    Ok(n)
}

fn parse_index_id(id: &str, op_index: usize) -> Result<usize, Error> {
    let path = format!("ops[{op_index}].id");
    let rest = id.strip_prefix("index:").ok_or_else(|| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::IndexPrefix,
            },
            path.clone(),
        )
    })?;
    let n: usize = rest.parse().map_err(|_| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::IndexIndexNotInt,
            },
            path.clone(),
        )
    })?;
    if n < 1 {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::IndexIndexFromOne,
            },
            path,
        ));
    }
    Ok(n)
}

fn parse_xe_id(id: &str, op_index: usize) -> Result<usize, Error> {
    let path = format!("ops[{op_index}].id");
    let rest = id.strip_prefix("xe:").ok_or_else(|| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::XePrefix,
            },
            path.clone(),
        )
    })?;
    let n: usize = rest.parse().map_err(|_| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::XeIndexNotInt,
            },
            path.clone(),
        )
    })?;
    if n < 1 {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::XeIndexFromOne,
            },
            path,
        ));
    }
    Ok(n)
}

fn parse_source_id(id: &str, op_index: usize) -> Result<usize, Error> {
    let path = format!("ops[{op_index}].id");
    let rest = id.strip_prefix("source:").ok_or_else(|| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SourcePrefix,
            },
            path.clone(),
        )
    })?;
    let n: usize = rest.parse().map_err(|_| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SourceIndexNotInt,
            },
            path.clone(),
        )
    })?;
    if n < 1 {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SourceIndexFromOne,
            },
            path,
        ));
    }
    Ok(n)
}

fn parse_cite_id(id: &str, op_index: usize) -> Result<usize, Error> {
    let path = format!("ops[{op_index}].id");
    let rest = id.strip_prefix("cite:").ok_or_else(|| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::CitePrefix,
            },
            path.clone(),
        )
    })?;
    let n: usize = rest.parse().map_err(|_| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::CiteIndexNotInt,
            },
            path.clone(),
        )
    })?;
    if n < 1 {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::CiteIndexFromOne,
            },
            path,
        ));
    }
    Ok(n)
}

fn parse_biblio_id(id: &str, op_index: usize) -> Result<usize, Error> {
    let path = format!("ops[{op_index}].id");
    let rest = id.strip_prefix("biblio:").ok_or_else(|| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::BiblioPrefix,
            },
            path.clone(),
        )
    })?;
    let n: usize = rest.parse().map_err(|_| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::BiblioIndexNotInt,
            },
            path.clone(),
        )
    })?;
    if n < 1 {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::BiblioIndexFromOne,
            },
            path,
        ));
    }
    Ok(n)
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

fn parse_story_para_at(id: &str, path: &str, reason: IdReason) -> Result<ParaTarget, Error> {
    let target = para::parse_para_id_at(id, path)?;
    match target {
        ParaTarget::Body(_) | ParaTarget::HeaderFooter { .. } => Ok(target),
        _ => Err(Error::with_path(
            ErrorKind::InvalidId { reason },
            path.to_string(),
        )),
    }
}

fn parse_story_or_box_para_at(id: &str, path: &str, reason: IdReason) -> Result<ParaTarget, Error> {
    let target = para::parse_para_id_at(id, path)?;
    match &target {
        ParaTarget::Body(_) | ParaTarget::HeaderFooter { .. } => Ok(target),
        ParaTarget::InTextBox { host, .. } => match host {
            HostPara::Body(_) | HostPara::HeaderFooter { .. } => Ok(target),
            HostPara::InNote { .. } | HostPara::InComment { .. } => Err(Error::with_path(
                ErrorKind::InvalidId { reason },
                path.to_string(),
            )),
        },
        _ => Err(Error::with_path(
            ErrorKind::InvalidId { reason },
            path.to_string(),
        )),
    }
}

fn same_story(a: &ParaTarget, b: &ParaTarget) -> bool {
    match (a, b) {
        (ParaTarget::Body(_), ParaTarget::Body(_)) => true,
        (
            ParaTarget::HeaderFooter {
                kind: k1,
                hf_type: t1,
                n: n1,
                ..
            },
            ParaTarget::HeaderFooter {
                kind: k2,
                hf_type: t2,
                n: n2,
                ..
            },
        ) => k1 == k2 && t1 == t2 && n1 == n2,
        (
            ParaTarget::InTextBox {
                host: h1, key: k1, ..
            },
            ParaTarget::InTextBox {
                host: h2, key: k2, ..
            },
        ) => h1 == h2 && k1 == k2,
        _ => false,
    }
}

fn optional_op_string(
    obj: &serde_json::Map<String, Value>,
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

fn parse_insert_hyperlink(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertHyperlinkOp, Error> {
    let id = require_op_id(obj, i)?;
    let path = format!("ops[{i}].id");
    let host = if looks_like_pic_hlink_id(&id) {
        deny_keys(obj, i, &["op", "id", "url", "anchor", "tooltip"])?;
        let pic = picture::parse_pic_id_at(&id, &path)?;
        reject_pic_for_hlink(&pic, &path)?;
        InsertHyperlinkHost::Picture(pic)
    } else if looks_like_txbx_hlink_id(&id) {
        deny_keys(obj, i, &["op", "id", "url", "anchor", "tooltip"])?;
        let box_t = para::parse_txbx_id_at(&id, &path)?;
        reject_txbx_for_hlink(&box_t, &path)?;
        InsertHyperlinkHost::TextBox(box_t)
    } else {
        deny_keys(obj, i, &["op", "id", "text", "url", "anchor", "tooltip"])?;
        let para = parse_story_or_box_para_at(&id, &path, IdReason::InsertLinkNeedsPara)?;
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
                        op: "insertHyperlink".into(),
                        field: "text".into(),
                    },
                    format!("ops[{i}].text"),
                ));
            }
        };
        InsertHyperlinkHost::Para { para, text }
    };
    let url = optional_op_string(obj, i, "url")?;
    let anchor = optional_op_string(obj, i, "anchor")?;
    match (&url, &anchor) {
        (Some(_), Some(_)) => {
            return Err(Error::with_path(
                ErrorKind::MutuallyExclusive {
                    a: "url".into(),
                    b: "anchor".into(),
                },
                format!("ops[{i}]"),
            ));
        }
        (None, None) => {
            return Err(Error::with_path(
                ErrorKind::HyperlinkNeedTarget,
                format!("ops[{i}]"),
            ));
        }
        _ => {}
    }
    if let Some(u) = url.as_deref() {
        if u.is_empty() {
            return Err(Error::with_path(
                ErrorKind::EmptyHyperlinkUrl,
                format!("ops[{i}].url"),
            ));
        }
    }
    if let Some(a) = anchor.as_deref() {
        super::link::check_bookmark_name(a, i, "anchor")?;
    }
    let tooltip = optional_op_string(obj, i, "tooltip")?;
    Ok(InsertHyperlinkOp {
        id,
        host,
        url,
        anchor,
        tooltip,
    })
}

fn parse_set_hyperlink(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetHyperlinkOp, Error> {
    let id = require_op_id(obj, i)?;
    let path = format!("ops[{i}].id");
    let drawing = looks_like_pic_hlink_id(&id) || looks_like_txbx_hlink_id(&id);
    if drawing {
        deny_keys(obj, i, &["op", "id", "url", "anchor", "tooltip"])?;
    } else {
        deny_keys(obj, i, &["op", "id", "url", "anchor", "text", "tooltip"])?;
    }
    let target = parse_hyperlink_target(&id, &path)?;
    let url = optional_op_string(obj, i, "url")?;
    let anchor = optional_op_string(obj, i, "anchor")?;
    let text = if drawing {
        None
    } else {
        optional_op_string(obj, i, "text")?
    };
    let tooltip = match obj.get("tooltip") {
        None => None,
        Some(Value::Null) => Some(None),
        Some(Value::String(s)) => Some(Some(s.clone())),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "tooltip".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].tooltip"),
            ));
        }
    };
    if url.is_some() && anchor.is_some() {
        return Err(Error::with_path(
            ErrorKind::MutuallyExclusive {
                a: "url".into(),
                b: "anchor".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    if url.is_none() && anchor.is_none() && text.is_none() && tooltip.is_none() {
        return Err(Error::with_path(
            ErrorKind::NeedSetHyperlinkField,
            format!("ops[{i}]"),
        ));
    }
    if let Some(u) = url.as_deref() {
        if u.is_empty() {
            return Err(Error::with_path(
                ErrorKind::EmptyHyperlinkUrl,
                format!("ops[{i}].url"),
            ));
        }
    }
    if let Some(a) = anchor.as_deref() {
        super::link::check_bookmark_name(a, i, "anchor")?;
    }
    Ok(SetHyperlinkOp {
        id,
        target,
        url,
        anchor,
        text,
        tooltip,
    })
}

fn parse_delete_hyperlink(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteHyperlinkOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_op_id(obj, i)?;
    let target = parse_hyperlink_target(&id, &format!("ops[{i}].id"))?;
    Ok(DeleteHyperlinkOp { id, target })
}

fn looks_like_pic_hlink_id(id: &str) -> bool {
    id.contains("/pic:")
}

fn looks_like_txbx_hlink_id(id: &str) -> bool {
    let Some(rest) = id.split_once("/txbx:").map(|(_, r)| r) else {
        return false;
    };
    !rest.contains('/') && !id.contains("/pic:")
}

fn parse_hyperlink_target(id: &str, path: &str) -> Result<HyperlinkTarget, Error> {
    if looks_like_pic_hlink_id(id) {
        let pic = picture::parse_pic_id_at(id, path)?;
        reject_pic_for_hlink(&pic, path)?;
        return Ok(HyperlinkTarget::Picture(pic));
    }
    if looks_like_txbx_hlink_id(id) {
        let box_t = para::parse_txbx_id_at(id, path)?;
        reject_txbx_for_hlink(&box_t, path)?;
        return Ok(HyperlinkTarget::TextBox(box_t));
    }
    let link = super::link::parse_link_ref(id, path)?;
    Ok(HyperlinkTarget::Link(link))
}

fn reject_pic_for_hlink(target: &PicTarget, path: &str) -> Result<(), Error> {
    match target {
        PicTarget::Body { .. } | PicTarget::HeaderFooter { .. } => Ok(()),
        _ => Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::InsertLinkNeedsPara,
            },
            path.to_string(),
        )),
    }
}

fn reject_txbx_for_hlink(target: &TxbxTarget, path: &str) -> Result<(), Error> {
    match &target.host {
        HostPara::Body(_) | HostPara::HeaderFooter { .. } => Ok(()),
        _ => Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::InsertLinkNeedsPara,
            },
            path.to_string(),
        )),
    }
}

fn parse_insert_bookmark(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertBookmarkOp, Error> {
    deny_keys(obj, i, &["op", "id", "name", "end"])?;
    let id = require_op_id(obj, i)?;
    let start = parse_story_or_box_para_at(
        &id,
        &format!("ops[{i}].id"),
        IdReason::InsertBookmarkNeedsPara,
    )?;
    let end = match obj.get("end") {
        None => start.clone(),
        Some(Value::String(s)) if !s.is_empty() => parse_story_or_box_para_at(
            s,
            &format!("ops[{i}].end"),
            IdReason::InsertBookmarkNeedsPara,
        )?,
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::InsertBookmarkNeedsPara,
                },
                format!("ops[{i}].end"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "end".into(),
                    expected: TypeName::NonEmptyString,
                },
                format!("ops[{i}].end"),
            ));
        }
    };
    if !same_story(&start, &end) {
        return Err(Error::with_path(
            ErrorKind::BookmarkRangeCrossStory,
            format!("ops[{i}].end"),
        ));
    }
    let name = optional_op_string(obj, i, "name")?;
    if let Some(n) = name.as_deref() {
        super::link::check_bookmark_name(n, i, "name")?;
    }
    Ok(InsertBookmarkOp {
        id,
        start,
        end,
        name,
    })
}

fn parse_set_bookmark(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetBookmarkOp, Error> {
    deny_keys(obj, i, &["op", "id", "name"])?;
    let id = require_op_id(obj, i)?;
    let target = super::link::parse_bookmark_ref(&id, &format!("ops[{i}].id"))?;
    let name = match obj.get("name") {
        Some(Value::String(s)) => {
            super::link::check_bookmark_name(s, i, "name")?;
            s.clone()
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "name".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].name"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::OpRequiresField {
                    op: "setBookmark".into(),
                    field: "name".into(),
                },
                format!("ops[{i}].name"),
            ));
        }
    };
    Ok(SetBookmarkOp { id, target, name })
}

fn parse_delete_bookmark(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteBookmarkOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_op_id(obj, i)?;
    let target = super::link::parse_bookmark_ref(&id, &format!("ops[{i}].id"))?;
    Ok(DeleteBookmarkOp { id, target })
}

fn parse_insert_edit_range(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertEditRangeOp, Error> {
    deny_keys(obj, i, &["op", "id", "edGrp", "ed"])?;
    let id = require_op_id(obj, i)?;
    let para = parse_story_para_at(
        &id,
        &format!("ops[{i}].id"),
        IdReason::InsertEditRangeNeedsPara,
    )?;
    let ed_grp = match obj.get("edGrp") {
        None => None,
        Some(Value::String(s)) if super::perm::ed_grp_ok(s) => Some(s.clone()),
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidEdGrp,
                format!("ops[{i}].edGrp"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "edGrp".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].edGrp"),
            ));
        }
    };
    let ed = match obj.get("ed") {
        None => None,
        Some(Value::String(s)) if !s.is_empty() => Some(s.clone()),
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::EmptyEditRangeEd,
                format!("ops[{i}].ed"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "ed".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].ed"),
            ));
        }
    };
    if ed_grp.is_some() && ed.is_some() {
        return Err(Error::with_path(
            ErrorKind::MutuallyExclusive {
                a: "edGrp".into(),
                b: "ed".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    let ed_grp = if ed_grp.is_none() && ed.is_none() {
        Some("everyone".into())
    } else {
        ed_grp
    };
    Ok(InsertEditRangeOp {
        id,
        para,
        ed_grp,
        ed,
    })
}

fn parse_delete_edit_range(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteEditRangeOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_op_id(obj, i)?;
    let target = super::perm::parse_perm_ref(&id, &format!("ops[{i}].id"))?;
    Ok(DeleteEditRangeOp { id, target })
}

fn require_instr(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    missing: ErrorKind,
) -> Result<String, Error> {
    match obj.get("instr") {
        Some(Value::String(s)) => {
            if s.trim().is_empty() {
                return Err(Error::with_path(
                    ErrorKind::EmptyFieldInstr,
                    format!("ops[{i}].instr"),
                ));
            }
            Ok(s.clone())
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "instr".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].instr"),
        )),
        None => Err(Error::with_path(missing, format!("ops[{i}].instr"))),
    }
}

fn parse_insert_field(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertFieldOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op", "id", "instr", "result", "code", "arg", "name", "items", "checked",
        ],
    )?;
    let id = require_op_id(obj, i)?;
    let para =
        parse_story_or_box_para_at(&id, &format!("ops[{i}].id"), IdReason::InsertFieldNeedsPara)?;
    let body = super::field::parse_field_body(obj, &format!("ops[{i}]"))?;
    Ok(InsertFieldOp {
        id,
        para,
        instr: body.instr,
        result: body.result,
        form: body.form,
    })
}

fn parse_set_field(obj: &serde_json::Map<String, Value>, i: usize) -> Result<SetFieldOp, Error> {
    deny_keys(obj, i, &["op", "id", "instr"])?;
    let id = require_op_id(obj, i)?;
    let target = super::field::parse_field_ref(&id, &format!("ops[{i}].id"))?;
    let instr = require_instr(obj, i, ErrorKind::NeedSetFieldInstr)?;
    Ok(SetFieldOp { id, target, instr })
}

fn parse_delete_field(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteFieldOp, Error> {
    deny_keys(obj, i, &["op", "id", "keep"])?;
    let id = require_op_id(obj, i)?;
    let target = super::field::parse_field_ref(&id, &format!("ops[{i}].id"))?;
    let keep = match obj.get("keep") {
        None => super::field::FieldKeep::Result,
        Some(Value::String(s)) if s == "result" => super::field::FieldKeep::Result,
        Some(Value::String(s)) if s == "instr" => super::field::FieldKeep::Instr,
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidFieldKeep,
                format!("ops[{i}].keep"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "keep".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].keep"),
            ));
        }
    };
    Ok(DeleteFieldOp { id, target, keep })
}

fn parse_insert_numbering(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertNumberingOp, Error> {
    deny_keys(obj, i, &["op", "id", "kind"])?;
    let id = require_op_id(obj, i)?;
    if id != "docx:document" {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::InsertNumberingNeedsDocument,
            },
            format!("ops[{i}].id"),
        ));
    }
    let kind = match obj.get("kind").and_then(Value::as_str) {
        Some(s) => super::numbering::parse_kind(s).ok_or_else(|| {
            Error::with_path(ErrorKind::InvalidNumberingKind, format!("ops[{i}].kind"))
        })?,
        None => {
            return Err(Error::with_path(
                ErrorKind::OpRequiresField {
                    op: "insertNumbering".into(),
                    field: "kind".into(),
                },
                format!("ops[{i}].kind"),
            ));
        }
    };
    Ok(InsertNumberingOp { kind })
}

fn parse_set_numbering(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetNumberingOp, Error> {
    deny_keys(obj, i, &["op", "id", "ilvl", "numFmt", "start", "lvlText"])?;
    let id = require_op_id(obj, i)?;
    let abstract_id = super::numbering::parse_abstract_ref(&id, &format!("ops[{i}].id"))?;
    let ilvl = match obj.get("ilvl") {
        Some(Value::Number(n)) => n.as_u64().ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: "ilvl".into(),
                    expected: TypeName::Integer,
                },
                format!("ops[{i}].ilvl"),
            )
        })? as u32,
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "ilvl".into(),
                    expected: TypeName::Integer,
                },
                format!("ops[{i}].ilvl"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::OpRequiresField {
                    op: "setNumbering".into(),
                    field: "ilvl".into(),
                },
                format!("ops[{i}].ilvl"),
            ));
        }
    };
    if ilvl > 8 {
        return Err(Error::with_path(
            ErrorKind::InvalidIlvl,
            format!("ops[{i}].ilvl"),
        ));
    }
    let num_fmt = match obj.get("numFmt") {
        None => None,
        Some(Value::String(s)) => {
            if !super::numbering::NUM_FMT_SET.contains(&s.as_str()) {
                return Err(Error::with_path(
                    ErrorKind::InvalidListNumFmt,
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
    let start = match obj.get("start") {
        None => None,
        Some(Value::Number(n)) => {
            let u = n.as_u64().ok_or_else(|| {
                Error::with_path(
                    ErrorKind::WrongType {
                        field: "start".into(),
                        expected: TypeName::Integer,
                    },
                    format!("ops[{i}].start"),
                )
            })?;
            if u < 1 {
                return Err(Error::with_path(
                    ErrorKind::MinValue {
                        field: "start".into(),
                        min: 1,
                    },
                    format!("ops[{i}].start"),
                ));
            }
            Some(u as u32)
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "start".into(),
                    expected: TypeName::Integer,
                },
                format!("ops[{i}].start"),
            ));
        }
    };
    let lvl_text = optional_op_string(obj, i, "lvlText")?;
    if num_fmt.is_none() && start.is_none() && lvl_text.is_none() {
        return Err(Error::with_path(
            ErrorKind::NeedSetNumberingField,
            format!("ops[{i}]"),
        ));
    }
    Ok(SetNumberingOp {
        id,
        abstract_id,
        ilvl,
        num_fmt,
        start,
        lvl_text,
    })
}

fn parse_set_para_numbering(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetParaNumberingOp, Error> {
    deny_keys(obj, i, &["op", "id", "num", "ilvl"])?;
    let id = require_op_id(obj, i)?;
    let para = parse_story_para_at(
        &id,
        &format!("ops[{i}].id"),
        IdReason::SetParaNumberingNeedsPara,
    )?;
    let num = match obj.get("num") {
        Some(Value::Null) => None,
        Some(Value::String(s)) => Some(super::numbering::parse_num_ref(
            s,
            &format!("ops[{i}].num"),
        )?),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "num".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].num"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::NeedSetParaNumberingNum,
                format!("ops[{i}].num"),
            ));
        }
    };
    let ilvl = match obj.get("ilvl") {
        None => 0,
        Some(Value::Number(n)) => n.as_u64().ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: "ilvl".into(),
                    expected: TypeName::Integer,
                },
                format!("ops[{i}].ilvl"),
            )
        })? as u32,
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "ilvl".into(),
                    expected: TypeName::Integer,
                },
                format!("ops[{i}].ilvl"),
            ));
        }
    };
    if ilvl > 8 {
        return Err(Error::with_path(
            ErrorKind::InvalidIlvl,
            format!("ops[{i}].ilvl"),
        ));
    }
    Ok(SetParaNumberingOp {
        id,
        para,
        num,
        ilvl,
    })
}

fn parse_set_style(obj: &serde_json::Map<String, Value>, i: usize) -> Result<SetStyleOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op",
            "id",
            "font",
            "eastAsiaFont",
            "sz",
            "bold",
            "color",
            "before",
            "after",
        ],
    )?;
    let id = require_op_id(obj, i)?;
    let style_id = super::styles::parse_style_ref(&id, &format!("ops[{i}].id"))?;
    let font = parse_nonempty_opt_string(obj, i, "font")?;
    let east_asia_font = parse_nonempty_opt_string(obj, i, "eastAsiaFont")?;
    let sz = parse_opt_u32_range(obj, i, "sz", 2, 3276)?;
    let bold = match obj.get("bold") {
        None => None,
        Some(Value::Bool(b)) => Some(*b),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "bold".into(),
                    expected: TypeName::Boolean,
                },
                format!("ops[{i}].bold"),
            ));
        }
    };
    let color = match obj.get("color") {
        None => None,
        Some(Value::String(s)) => {
            if !is_rgb_hex(s) {
                return Err(Error::with_path(
                    ErrorKind::InvalidStyleColor,
                    format!("ops[{i}].color"),
                ));
            }
            Some(s.clone())
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "color".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].color"),
            ));
        }
    };
    let before = parse_opt_u32_min(obj, i, "before", 0)?;
    let after = parse_opt_u32_min(obj, i, "after", 0)?;
    if font.is_none()
        && east_asia_font.is_none()
        && sz.is_none()
        && bold.is_none()
        && color.is_none()
        && before.is_none()
        && after.is_none()
    {
        return Err(Error::with_path(
            ErrorKind::NeedSetStyleField,
            format!("ops[{i}]"),
        ));
    }
    Ok(SetStyleOp {
        id,
        style_id,
        font,
        east_asia_font,
        sz,
        bold,
        color,
        before,
        after,
    })
}

fn parse_set_para_style(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetParaStyleOp, Error> {
    deny_keys(obj, i, &["op", "id", "style"])?;
    let id = require_op_id(obj, i)?;
    let para = parse_story_para_at(
        &id,
        &format!("ops[{i}].id"),
        IdReason::SetParaStyleNeedsPara,
    )?;
    let style = match obj.get("style") {
        Some(Value::Null) => None,
        Some(Value::String(s)) => Some(super::styles::parse_style_ref(
            s,
            &format!("ops[{i}].style"),
        )?),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "style".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].style"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::NeedSetParaStyleStyle,
                format!("ops[{i}].style"),
            ));
        }
    };
    Ok(SetParaStyleOp { id, para, style })
}

fn parse_insert_style(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertStyleOp, Error> {
    deny_keys(obj, i, &["op", "id", "styleId", "type", "name", "basedOn"])?;
    let id = require_op_id(obj, i)?;
    if id != "docx:document" {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::InsertStyleNeedsDocument,
            },
            format!("ops[{i}].id"),
        ));
    }
    let style_id = match obj.get("styleId") {
        Some(Value::String(s)) => {
            super::styles::validate_style_id_token(s, &format!("ops[{i}].styleId"))?;
            s.clone()
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "styleId".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].styleId"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::NeedInsertStyleField,
                format!("ops[{i}].styleId"),
            ));
        }
    };
    let ty = match obj.get("type").and_then(Value::as_str) {
        Some(t @ ("paragraph" | "character" | "table")) => t.to_string(),
        Some("numbering") | Some(_) => {
            return Err(Error::with_path(
                ErrorKind::InvalidStyleType,
                format!("ops[{i}].type"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::NeedInsertStyleField,
                format!("ops[{i}].type"),
            ));
        }
    };
    let name = match obj.get("name") {
        None => style_id.clone(),
        Some(Value::String(s)) if !s.is_empty() => s.clone(),
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::NeedInsertStyleField,
                format!("ops[{i}].name"),
            ));
        }
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
    let based_on = match obj.get("basedOn") {
        None | Some(Value::Null) => None,
        Some(Value::String(s)) => Some(super::styles::parse_style_ref(
            s,
            &format!("ops[{i}].basedOn"),
        )?),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "basedOn".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].basedOn"),
            ));
        }
    };
    Ok(InsertStyleOp {
        style_id,
        ty,
        name,
        based_on,
    })
}

fn parse_delete_style(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteStyleOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_op_id(obj, i)?;
    let style_id = super::styles::parse_style_ref(&id, &format!("ops[{i}].id"))?;
    Ok(DeleteStyleOp { id, style_id })
}

fn parse_set_run_style(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetRunStyleOp, Error> {
    deny_keys(obj, i, &["op", "id", "style"])?;
    let id = require_op_id(obj, i)?;
    let (para, run) = super::format::parse_run_id(&id, &format!("ops[{i}].id"))?;
    match &para {
        ParaTarget::Body(_) | ParaTarget::HeaderFooter { .. } => {}
        _ => {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::SetRunStyleNeedsStoryRun,
                },
                format!("ops[{i}].id"),
            ));
        }
    }
    let style = match obj.get("style") {
        Some(Value::Null) => None,
        Some(Value::String(s)) => Some(super::styles::parse_style_ref(
            s,
            &format!("ops[{i}].style"),
        )?),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "style".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].style"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::NeedSetParaStyleStyle,
                format!("ops[{i}].style"),
            ));
        }
    };
    Ok(SetRunStyleOp {
        id,
        para,
        run,
        style,
    })
}

fn parse_set_table_style(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetTableStyleOp, Error> {
    deny_keys(obj, i, &["op", "id", "style", "look"])?;
    let id = require_op_id(obj, i)?;
    let target = tbl::parse_table_only(&id, i, IdReason::NeedTableId)?;
    let style = match obj.get("style") {
        None => None,
        Some(Value::Null) => Some(None),
        Some(Value::String(s)) => Some(Some(super::styles::parse_style_ref(
            s,
            &format!("ops[{i}].style"),
        )?)),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "style".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].style"),
            ));
        }
    };
    let look = match obj.get("look") {
        None => None,
        Some(Value::Null) => Some(None),
        Some(Value::Object(m)) => {
            deny_keys(
                m,
                i,
                &[
                    "firstRow",
                    "lastRow",
                    "firstColumn",
                    "lastColumn",
                    "bandedRows",
                    "bandedColumns",
                ],
            )?;
            Some(Some(super::tbl::TableLookPatch {
                first_row: parse_opt_bool(m, i, "firstRow")?,
                last_row: parse_opt_bool(m, i, "lastRow")?,
                first_column: parse_opt_bool(m, i, "firstColumn")?,
                last_column: parse_opt_bool(m, i, "lastColumn")?,
                banded_rows: parse_opt_bool(m, i, "bandedRows")?,
                banded_columns: parse_opt_bool(m, i, "bandedColumns")?,
            }))
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "look".into(),
                    expected: TypeName::Object,
                },
                format!("ops[{i}].look"),
            ));
        }
    };
    if style.is_none() && look.is_none() {
        return Err(Error::with_path(
            ErrorKind::NeedSetTableStyleField,
            format!("ops[{i}]"),
        ));
    }
    Ok(SetTableStyleOp {
        id,
        target,
        style,
        look,
    })
}

fn parse_set_table_float(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetTableFloatOp, Error> {
    deny_keys(obj, i, &["op", "id", "pos"])?;
    let id = require_op_id(obj, i)?;
    let target = tbl::parse_table_only(&id, i, IdReason::NeedTableId)?;
    let pos = match obj.get("pos") {
        Some(Value::Null) => None,
        Some(Value::Object(m)) => {
            deny_keys(
                m,
                i,
                &[
                    "horzAnchor",
                    "vertAnchor",
                    "x",
                    "y",
                    "leftFromText",
                    "rightFromText",
                    "topFromText",
                    "bottomFromText",
                ],
            )?;
            let horz = parse_opt_tbl_anchor(m, i, "horzAnchor")?;
            let vert = parse_opt_tbl_anchor(m, i, "vertAnchor")?;
            Some(super::tbl::TableFloat {
                horz_anchor: horz,
                vert_anchor: vert,
                x: parse_opt_i32(m, i, "x")?,
                y: parse_opt_i32(m, i, "y")?,
                left_from_text: parse_opt_u32_min(m, i, "leftFromText", 0)?,
                right_from_text: parse_opt_u32_min(m, i, "rightFromText", 0)?,
                top_from_text: parse_opt_u32_min(m, i, "topFromText", 0)?,
                bottom_from_text: parse_opt_u32_min(m, i, "bottomFromText", 0)?,
            })
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "pos".into(),
                    expected: TypeName::Object,
                },
                format!("ops[{i}].pos"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::NeedSetTableFloatField,
                format!("ops[{i}].pos"),
            ));
        }
    };
    Ok(SetTableFloatOp { id, target, pos })
}

fn parse_set_table_cell_mar(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetTableCellMarOp, Error> {
    deny_keys(obj, i, &["op", "id", "top", "left", "bottom", "right"])?;
    let id = require_op_id(obj, i)?;
    let target = tbl::parse_table_only(&id, i, IdReason::NeedTableId)?;
    let top = parse_opt_null_u32(obj, i, "top")?;
    let left = parse_opt_null_u32(obj, i, "left")?;
    let bottom = parse_opt_null_u32(obj, i, "bottom")?;
    let right = parse_opt_null_u32(obj, i, "right")?;
    if top.is_none() && left.is_none() && bottom.is_none() && right.is_none() {
        return Err(Error::with_path(
            ErrorKind::NeedSetTableCellMarField,
            format!("ops[{i}]"),
        ));
    }
    Ok(SetTableCellMarOp {
        id,
        target,
        top,
        left,
        bottom,
        right,
    })
}

fn parse_set_cell_format(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetCellFormatOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op", "id", "fill", "top", "left", "bottom", "right", "tl2br", "tr2bl",
        ],
    )?;
    let id = require_op_id(obj, i)?;
    let target = tbl::parse_cell_ref(&id, i)?;
    let fill = parse_opt_null_rgb(obj, i, "fill")?;
    let top = parse_opt_null_u32(obj, i, "top")?;
    let left = parse_opt_null_u32(obj, i, "left")?;
    let bottom = parse_opt_null_u32(obj, i, "bottom")?;
    let right = parse_opt_null_u32(obj, i, "right")?;
    let tl2br = parse_opt_null_diagonal(obj, i, "tl2br")?;
    let tr2bl = parse_opt_null_diagonal(obj, i, "tr2bl")?;
    if fill.is_none()
        && top.is_none()
        && left.is_none()
        && bottom.is_none()
        && right.is_none()
        && tl2br.is_none()
        && tr2bl.is_none()
    {
        return Err(Error::with_path(
            ErrorKind::NeedSetCellFormatField,
            format!("ops[{i}]"),
        ));
    }
    Ok(SetCellFormatOp {
        id,
        target,
        fill,
        top,
        left,
        bottom,
        right,
        tl2br,
        tr2bl,
    })
}

fn parse_set_para_format(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetParaFormatOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op",
            "id",
            "align",
            "left",
            "right",
            "firstLine",
            "hanging",
            "before",
            "after",
            "line",
            "lineRule",
            "kinsoku",
            "overflowPunct",
            "topLinePunct",
            "autoSpaceDE",
            "autoSpaceDN",
        ],
    )?;
    let id = require_op_id(obj, i)?;
    let para = para::parse_para_id(&id, i)?;
    let align = parse_opt_null_enum(
        obj,
        i,
        "align",
        super::format::ALIGN_SET,
        ErrorKind::InvalidAlign,
    )?;
    let left = parse_opt_null_u32(obj, i, "left")?;
    let right = parse_opt_null_u32(obj, i, "right")?;
    let first_line = parse_opt_null_u32(obj, i, "firstLine")?;
    let hanging = parse_opt_null_u32(obj, i, "hanging")?;
    if matches!((&first_line, &hanging), (Some(Some(_)), Some(Some(_)))) {
        return Err(Error::with_path(
            ErrorKind::MutuallyExclusive {
                a: "firstLine".into(),
                b: "hanging".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    let before = parse_opt_null_u32(obj, i, "before")?;
    let after = parse_opt_null_u32(obj, i, "after")?;
    let line = parse_opt_null_u32(obj, i, "line")?;
    let line_rule = parse_opt_null_enum(
        obj,
        i,
        "lineRule",
        super::format::LINE_RULE_SET,
        ErrorKind::InvalidLineRule,
    )?;
    let kinsoku = parse_opt_null_bool(obj, i, "kinsoku")?;
    let overflow_punct = parse_opt_null_bool(obj, i, "overflowPunct")?;
    let top_line_punct = parse_opt_null_bool(obj, i, "topLinePunct")?;
    let auto_space_de = parse_opt_null_bool(obj, i, "autoSpaceDE")?;
    let auto_space_dn = parse_opt_null_bool(obj, i, "autoSpaceDN")?;
    if align.is_none()
        && left.is_none()
        && right.is_none()
        && first_line.is_none()
        && hanging.is_none()
        && before.is_none()
        && after.is_none()
        && line.is_none()
        && line_rule.is_none()
        && kinsoku.is_none()
        && overflow_punct.is_none()
        && top_line_punct.is_none()
        && auto_space_de.is_none()
        && auto_space_dn.is_none()
    {
        return Err(Error::with_path(
            ErrorKind::NeedSetParaFormatField,
            format!("ops[{i}]"),
        ));
    }
    Ok(SetParaFormatOp {
        id,
        para,
        align,
        left,
        right,
        first_line,
        hanging,
        before,
        after,
        line,
        line_rule,
        kinsoku,
        overflow_punct,
        top_line_punct,
        auto_space_de,
        auto_space_dn,
    })
}

fn parse_set_run_format(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetRunFormatOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op",
            "id",
            "bold",
            "italic",
            "underline",
            "sz",
            "color",
            "font",
            "eastAsiaFont",
            "em",
            "eastAsianLayout",
        ],
    )?;
    let id = require_op_id(obj, i)?;
    let (para, run) = super::format::parse_run_id(&id, &format!("ops[{i}].id"))?;
    let bold = parse_opt_bool(obj, i, "bold")?;
    let italic = parse_opt_bool(obj, i, "italic")?;
    let underline = parse_opt_null_enum(
        obj,
        i,
        "underline",
        super::format::UNDERLINE_SET,
        ErrorKind::InvalidUnderline,
    )?;
    let sz = parse_opt_u32_range(obj, i, "sz", 2, 3276)?;
    let color = match obj.get("color") {
        None => None,
        Some(Value::String(s)) => {
            if !is_rgb_hex(s) {
                return Err(Error::with_path(
                    ErrorKind::InvalidStyleColor,
                    format!("ops[{i}].color"),
                ));
            }
            Some(s.clone())
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "color".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].color"),
            ));
        }
    };
    let font = parse_nonempty_opt_string(obj, i, "font")?;
    let east_asia_font = parse_nonempty_opt_string(obj, i, "eastAsiaFont")?;
    let em = parse_opt_null_enum(obj, i, "em", super::format::EM_SET, ErrorKind::InvalidEm)?;
    let east_asian_layout = parse_opt_east_asian_layout(obj, i)?;
    if bold.is_none()
        && italic.is_none()
        && underline.is_none()
        && sz.is_none()
        && color.is_none()
        && font.is_none()
        && east_asia_font.is_none()
        && em.is_none()
        && east_asian_layout.is_none()
    {
        return Err(Error::with_path(
            ErrorKind::NeedSetRunFormatField,
            format!("ops[{i}]"),
        ));
    }
    Ok(SetRunFormatOp {
        id,
        para,
        run,
        bold,
        italic,
        underline,
        sz,
        color,
        font,
        east_asia_font,
        em,
        east_asian_layout,
    })
}

fn parse_opt_null_bool(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    field: &str,
) -> Result<Option<Option<bool>>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(None)),
        Some(Value::Bool(b)) => Ok(Some(Some(*b))),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Boolean,
            },
            format!("ops[{i}].{field}"),
        )),
    }
}

fn parse_opt_east_asian_layout(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<Option<Option<super::format::EastAsianLayout>>, Error> {
    match obj.get("eastAsianLayout") {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(None)),
        Some(Value::Object(inner)) => {
            for k in inner.keys() {
                if !matches!(
                    k.as_str(),
                    "id" | "combine" | "combineBrackets" | "vert" | "vertCompress"
                ) {
                    return Err(Error::with_path(
                        ErrorKind::UnknownField { field: k.clone() },
                        format!("ops[{i}].eastAsianLayout.{k}"),
                    ));
                }
            }
            let id = parse_layout_id(inner, i)?;
            let combine = parse_inner_opt_bool(inner, i, "combine")?;
            let vert = parse_inner_opt_bool(inner, i, "vert")?;
            let vert_compress = parse_inner_opt_bool(inner, i, "vertCompress")?;
            let combine_brackets = match inner.get("combineBrackets") {
                None => None,
                Some(Value::Null) => None,
                Some(Value::String(s)) => {
                    if !super::format::COMBINE_BRACKETS_SET.contains(&s.as_str()) {
                        return Err(Error::with_path(
                            ErrorKind::InvalidCombineBrackets,
                            format!("ops[{i}].eastAsianLayout.combineBrackets"),
                        ));
                    }
                    Some(s.clone())
                }
                Some(_) => {
                    return Err(Error::with_path(
                        ErrorKind::WrongType {
                            field: "combineBrackets".into(),
                            expected: TypeName::String,
                        },
                        format!("ops[{i}].eastAsianLayout.combineBrackets"),
                    ));
                }
            };
            if combine.is_none() && vert.is_none() {
                return Err(Error::with_path(
                    ErrorKind::NeedEastAsianLayoutField,
                    format!("ops[{i}].eastAsianLayout"),
                ));
            }
            if combine_brackets.is_some() && combine != Some(true) {
                return Err(Error::with_path(
                    ErrorKind::InvalidCombineBrackets,
                    format!("ops[{i}].eastAsianLayout.combineBrackets"),
                ));
            }
            Ok(Some(Some(super::format::EastAsianLayout {
                id,
                combine,
                combine_brackets,
                vert,
                vert_compress,
            })))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "eastAsianLayout".into(),
                expected: TypeName::Object,
            },
            format!("ops[{i}].eastAsianLayout"),
        )),
    }
}

fn parse_layout_id(obj: &serde_json::Map<String, Value>, i: usize) -> Result<Option<i64>, Error> {
    match obj.get("id") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(n)) => {
            let v = n
                .as_i64()
                .or_else(|| n.as_u64().and_then(|u| i64::try_from(u).ok()))
                .ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::WrongType {
                            field: "id".into(),
                            expected: TypeName::Integer,
                        },
                        format!("ops[{i}].eastAsianLayout.id"),
                    )
                })?;
            Ok(Some(v))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "id".into(),
                expected: TypeName::Integer,
            },
            format!("ops[{i}].eastAsianLayout.id"),
        )),
    }
}

fn parse_inner_opt_bool(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    field: &str,
) -> Result<Option<bool>, Error> {
    match obj.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Bool(b)) => Ok(Some(*b)),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Boolean,
            },
            format!("ops[{i}].eastAsianLayout.{field}"),
        )),
    }
}

fn parse_opt_bool(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    field: &str,
) -> Result<Option<bool>, Error> {
    match obj.get(field) {
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

fn parse_opt_null_u32(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    field: &str,
) -> Result<Option<Option<u32>>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(None)),
        Some(Value::Number(n)) => {
            let u = n.as_u64().ok_or_else(|| {
                Error::with_path(
                    ErrorKind::WrongType {
                        field: field.into(),
                        expected: TypeName::Integer,
                    },
                    format!("ops[{i}].{field}"),
                )
            })?;
            Ok(Some(Some(u as u32)))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Integer,
            },
            format!("ops[{i}].{field}"),
        )),
    }
}

fn parse_opt_null_enum(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    field: &str,
    allowed: &[&str],
    bad: ErrorKind,
) -> Result<Option<Option<String>>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(None)),
        Some(Value::String(s)) => {
            if !allowed.contains(&s.as_str()) {
                return Err(Error::with_path(bad, format!("ops[{i}].{field}")));
            }
            Ok(Some(Some(s.clone())))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].{field}"),
        )),
    }
}

fn parse_nonempty_opt_string(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    field: &str,
) -> Result<Option<String>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::String(s)) => {
            if s.is_empty() {
                return Err(Error::with_path(
                    ErrorKind::EmptyStyleFont,
                    format!("ops[{i}].{field}"),
                ));
            }
            Ok(Some(s.clone()))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].{field}"),
        )),
    }
}

fn parse_opt_u32_range(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    field: &str,
    min: u64,
    max: u64,
) -> Result<Option<u32>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::Number(n)) => {
            let u = n.as_u64().ok_or_else(|| {
                Error::with_path(
                    ErrorKind::WrongType {
                        field: field.into(),
                        expected: TypeName::Integer,
                    },
                    format!("ops[{i}].{field}"),
                )
            })?;
            if u < min {
                return Err(Error::with_path(
                    ErrorKind::MinValue {
                        field: field.into(),
                        min: min as i64,
                    },
                    format!("ops[{i}].{field}"),
                ));
            }
            if u > max {
                return Err(Error::with_path(
                    ErrorKind::OutOfRange {
                        field: field.into(),
                        min: min as i64,
                        max: max as i64,
                    },
                    format!("ops[{i}].{field}"),
                ));
            }
            Ok(Some(u as u32))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Integer,
            },
            format!("ops[{i}].{field}"),
        )),
    }
}

fn parse_opt_tbl_anchor(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    field: &str,
) -> Result<Option<String>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::String(s)) if super::tbl::TBL_ANCHORS.contains(&s.as_str()) => {
            Ok(Some(s.clone()))
        }
        Some(Value::String(_)) => Err(Error::with_path(
            ErrorKind::InvalidTblAnchor,
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

fn parse_opt_i32(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    field: &str,
) -> Result<Option<i32>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::Number(n)) => {
            let v = n.as_i64().ok_or_else(|| {
                Error::with_path(
                    ErrorKind::WrongType {
                        field: field.into(),
                        expected: TypeName::Integer,
                    },
                    format!("ops[{i}].{field}"),
                )
            })?;
            i32::try_from(v).map(Some).map_err(|_| {
                Error::with_path(
                    ErrorKind::WrongType {
                        field: field.into(),
                        expected: TypeName::Integer,
                    },
                    format!("ops[{i}].{field}"),
                )
            })
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Integer,
            },
            format!("ops[{i}].{field}"),
        )),
    }
}

fn parse_opt_null_rgb(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    field: &str,
) -> Result<Option<Option<String>>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(None)),
        Some(Value::String(s)) if is_rgb_hex(s) => Ok(Some(Some(s.to_ascii_uppercase()))),
        Some(Value::String(_)) => Err(Error::with_path(
            ErrorKind::InvalidStyleColor,
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

fn parse_opt_null_diagonal(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    field: &str,
) -> Result<Option<Option<super::tbl::DiagonalBorder>>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(None)),
        Some(Value::Object(m)) => {
            deny_keys(m, i, &["val", "sz", "color"])?;
            let val = match m.get("val") {
                None => "single".to_string(),
                Some(Value::String(s)) if super::tbl::is_diagonal_val(s) => s.clone(),
                Some(Value::String(_)) => {
                    return Err(Error::with_path(
                        ErrorKind::InvalidPageBorderVal,
                        format!("ops[{i}].{field}.val"),
                    ));
                }
                Some(_) => {
                    return Err(Error::with_path(
                        ErrorKind::WrongType {
                            field: "val".into(),
                            expected: TypeName::String,
                        },
                        format!("ops[{i}].{field}.val"),
                    ));
                }
            };
            let sz = parse_opt_u32_min(m, i, "sz", 0)?;
            let color = match m.get("color") {
                None => None,
                Some(Value::String(s)) if s.eq_ignore_ascii_case("auto") => Some("auto".into()),
                Some(Value::String(s)) if is_rgb_hex(s) => Some(s.to_ascii_uppercase()),
                Some(Value::String(_)) => {
                    return Err(Error::with_path(
                        ErrorKind::InvalidPageBorderColor,
                        format!("ops[{i}].{field}.color"),
                    ));
                }
                Some(_) => {
                    return Err(Error::with_path(
                        ErrorKind::WrongType {
                            field: "color".into(),
                            expected: TypeName::String,
                        },
                        format!("ops[{i}].{field}.color"),
                    ));
                }
            };
            Ok(Some(Some(super::tbl::DiagonalBorder { val, sz, color })))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Object,
            },
            format!("ops[{i}].{field}"),
        )),
    }
}

fn parse_opt_u32_min(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    field: &str,
    min: u64,
) -> Result<Option<u32>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::Number(n)) => {
            let u = n.as_u64().ok_or_else(|| {
                Error::with_path(
                    ErrorKind::WrongType {
                        field: field.into(),
                        expected: TypeName::Integer,
                    },
                    format!("ops[{i}].{field}"),
                )
            })?;
            if u < min {
                return Err(Error::with_path(
                    ErrorKind::MinValue {
                        field: field.into(),
                        min: min as i64,
                    },
                    format!("ops[{i}].{field}"),
                ));
            }
            Ok(Some(u as u32))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Integer,
            },
            format!("ops[{i}].{field}"),
        )),
    }
}

fn is_rgb_hex(s: &str) -> bool {
    s.len() == 6 && s.bytes().all(|b| b.is_ascii_hexdigit())
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
    op: &str,
) -> Result<Option<String>, Error> {
    match obj.get("color") {
        None if required => Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: op.into(),
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
    let color = parse_color_field(obj, i, true, "setTextBoxFill")?;
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
    let color = parse_color_field(obj, i, false, "setTextBoxLine")?;
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

fn parse_insert_connector(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertConnectorOp, Error> {
    deny_keys(obj, i, &["op", "id", "prst", "cx", "cy"])?;
    let id = require_id(obj, i)?;
    let host = para::parse_host_para_id_at(&id, &format!("ops[{i}].id"))?;
    let cx = parse_emu_field(obj.get("cx"), i, "cx")?;
    let cy = parse_emu_field(obj.get("cy"), i, "cy")?;
    if cx.is_some() != cy.is_some() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "insertConnector".into(),
                field: "cx and cy together".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    let prst = match obj.get("prst") {
        None => "straightConnector1".to_string(),
        Some(Value::String(s)) => {
            if super::connector::is_writable_prst(s) {
                s.clone()
            } else {
                return Err(Error::with_path(
                    ErrorKind::InvalidConnectorPrst,
                    format!("ops[{i}].prst"),
                ));
            }
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "prst".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].prst"),
            ));
        }
    };
    Ok(InsertConnectorOp {
        id,
        host,
        cx,
        cy,
        prst,
    })
}

fn parse_delete_connector(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteConnectorOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let target = super::connector::parse_cxn_id(&id, i)?;
    Ok(DeleteConnectorOp { id, target })
}

fn parse_insert_shape(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertShapeOp, Error> {
    deny_keys(obj, i, &["op", "id", "prst", "cx", "cy"])?;
    let id = require_id(obj, i)?;
    let host = para::parse_host_para_id_at(&id, &format!("ops[{i}].id"))?;
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
    let prst = match obj.get("prst") {
        None => "rect".to_string(),
        Some(Value::String(s)) => {
            if super::shape::is_writable_prst(s) {
                s.clone()
            } else {
                return Err(Error::with_path(
                    ErrorKind::InvalidShapePrst,
                    format!("ops[{i}].prst"),
                ));
            }
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "prst".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].prst"),
            ));
        }
    };
    Ok(InsertShapeOp {
        id,
        host,
        cx,
        cy,
        prst,
    })
}

fn parse_delete_shape(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteShapeOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let target = super::shape::parse_shape_id(&id, i)?;
    Ok(DeleteShapeOp { id, target })
}

fn parse_set_shape_size(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetShapeSizeOp, Error> {
    deny_keys(obj, i, &["op", "id", "cx", "cy"])?;
    let id = require_id(obj, i)?;
    let target = super::shape::parse_shape_id(&id, i)?;
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
    Ok(SetShapeSizeOp { id, target, cx, cy })
}

fn parse_set_shape_pos(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetShapePosOp, Error> {
    deny_keys(obj, i, &["op", "id", "x", "y"])?;
    let id = require_id(obj, i)?;
    let target = super::shape::parse_shape_id(&id, i)?;
    let x = parse_i64_field(obj.get("x"), i, "x")?;
    let y = parse_i64_field(obj.get("y"), i, "y")?;
    if x.is_none() && y.is_none() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setShapePos".into(),
                field: "x or y".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(SetShapePosOp { id, target, x, y })
}

fn parse_set_shape_fill(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetShapeFillOp, Error> {
    deny_keys(obj, i, &["op", "id", "color"])?;
    let id = require_id(obj, i)?;
    let target = super::shape::parse_shape_id(&id, i)?;
    let color = parse_color_field(obj, i, true, "setShapeFill")?;
    Ok(SetShapeFillOp { id, target, color })
}

fn parse_set_shape_line(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetShapeLineOp, Error> {
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
    let color = parse_color_field(obj, i, false, "setShapeLine")?;
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

fn parse_group_shapes(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<GroupShapesOp, Error> {
    deny_keys(obj, i, &["op", "id", "ids"])?;
    let id = require_id(obj, i)?;
    let host = super::group::parse_group_host(&id, i)?;
    let ids = match obj.get("ids") {
        Some(Value::Array(arr)) => {
            if arr.len() < 2 {
                return Err(Error::with_path(
                    ErrorKind::GroupNeedsTwo,
                    format!("ops[{i}].ids"),
                ));
            }
            let mut out = Vec::new();
            for (k, v) in arr.iter().enumerate() {
                let s = v.as_str().ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::WrongType {
                            field: format!("ids[{k}]"),
                            expected: TypeName::String,
                        },
                        format!("ops[{i}].ids[{k}]"),
                    )
                })?;
                if s.is_empty() {
                    return Err(Error::with_path(
                        ErrorKind::WrongType {
                            field: format!("ids[{k}]"),
                            expected: TypeName::NonEmptyString,
                        },
                        format!("ops[{i}].ids[{k}]"),
                    ));
                }
                out.push(s.to_string());
            }
            out
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "ids".into(),
                    expected: TypeName::Array,
                },
                format!("ops[{i}].ids"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::OpRequiresField {
                    op: "groupShapes".into(),
                    field: "ids".into(),
                },
                format!("ops[{i}].ids"),
            ));
        }
    };
    let mut members = Vec::new();
    for (k, mid) in ids.iter().enumerate() {
        let parsed = super::group::parse_member_id(mid, i)
            .map_err(|e| e.at(format!("ops[{i}].ids[{k}]")))?;
        members.push(parsed);
    }
    Ok(GroupShapesOp { id, host, members })
}

fn parse_ungroup(obj: &serde_json::Map<String, Value>, i: usize) -> Result<UngroupOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let member = super::group::parse_member_id(&id, i)?;
    Ok(UngroupOp { id, member })
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
            "op",
            "id",
            "type",
            "ofPieType",
            "stockKind",
            "surfaceKind",
            "layoutId",
            "title",
            "series",
            "kind",
            "wrap",
            "cx",
            "cy",
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
    let of_pie_type =
        crate::chart::parse_of_pie_type(obj.get("ofPieType"), &format!("ops[{i}].ofPieType"))?;
    crate::chart::reject_of_pie_on_other(
        chart_type,
        of_pie_type.as_deref(),
        &format!("ops[{i}].ofPieType"),
    )?;
    let series = parse_insert_series(obj.get("series"), i, chart_type)?;
    let extra =
        crate::chart::parse_chart_extra(obj, chart_type, series.len(), &format!("ops[{i}]"), true)?;
    crate::chart::validate_insert_series_len(
        chart_type,
        series.len(),
        &extra,
        &format!("ops[{i}].series"),
    )?;
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
        of_pie_type,
        extra,
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

fn parse_insert_ole(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<super::ole::InsertOleOp, Error> {
    deny_keys(obj, i, &["op", "id", "file"])?;
    let id = require_id(obj, i)?;
    let para = super::ole::parse_insert_para_id(&id, i)?;
    let file = require_file(obj, i, "insertOle")?;
    Ok(super::ole::InsertOleOp { id, para, file })
}

fn parse_delete_ole(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<super::ole::DeleteOleOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let target = super::ole::parse_ole_id(&id, i)?;
    Ok(super::ole::DeleteOleOp { id, target })
}

fn parse_insert_ink_canvas(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<super::ink::InsertInkCanvasOp, Error> {
    deny_keys(obj, i, &["op", "id", "file", "cx", "cy"])?;
    let id = require_id(obj, i)?;
    let host = para::parse_host_para_id_at(&id, &format!("ops[{i}].id"))?;
    let file = require_file(obj, i, "insertInkCanvas")?;
    let cx = parse_emu_field(obj.get("cx"), i, "cx")?;
    let cy = parse_emu_field(obj.get("cy"), i, "cy")?;
    if cx.is_some() != cy.is_some() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "insertInkCanvas".into(),
                field: "cx and cy together".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(super::ink::InsertInkCanvasOp {
        id,
        host,
        file,
        cx,
        cy,
    })
}

fn parse_replace_ink_canvas(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<super::ink::ReplaceInkCanvasOp, Error> {
    deny_keys(obj, i, &["op", "id", "file"])?;
    let id = require_id(obj, i)?;
    let target = super::ink::parse_ink_id(&id, i)?;
    let file = require_file(obj, i, "replaceInkCanvas")?;
    Ok(super::ink::ReplaceInkCanvasOp { id, target, file })
}

fn parse_delete_ink_canvas(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<super::ink::DeleteInkCanvasOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let target = super::ink::parse_ink_id(&id, i)?;
    Ok(super::ink::DeleteInkCanvasOp { id, target })
}

fn parse_insert_model3d(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<super::model3d::InsertModel3dOp, Error> {
    deny_keys(obj, i, &["op", "id", "file", "cx", "cy"])?;
    let id = require_id(obj, i)?;
    let host = para::parse_host_para_id_at(&id, &format!("ops[{i}].id"))?;
    let file = require_file(obj, i, "insertModel3d")?;
    let cx = parse_emu_field(obj.get("cx"), i, "cx")?;
    let cy = parse_emu_field(obj.get("cy"), i, "cy")?;
    if cx.is_some() != cy.is_some() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "insertModel3d".into(),
                field: "cx and cy together".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(super::model3d::InsertModel3dOp {
        id,
        host,
        file,
        cx,
        cy,
    })
}

fn parse_replace_model3d(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<super::model3d::ReplaceModel3dOp, Error> {
    deny_keys(obj, i, &["op", "id", "file"])?;
    let id = require_id(obj, i)?;
    let target = super::model3d::parse_id(&id, i)?;
    let file = require_file(obj, i, "replaceModel3d")?;
    Ok(super::model3d::ReplaceModel3dOp { id, target, file })
}

fn parse_delete_model3d(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<super::model3d::DeleteModel3dOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let target = super::model3d::parse_id(&id, i)?;
    Ok(super::model3d::DeleteModel3dOp { id, target })
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
    deny_keys(
        obj,
        i,
        &[
            "op",
            "id",
            "type",
            "ofPieType",
            "stockKind",
            "surfaceKind",
            "layoutId",
        ],
    )?;
    let id = require_id(obj, i)?;
    let target = charts::parse_chart_id(&id, i)?;
    let chart_type = parse_chart_type_field(obj.get("type"), i, true)?;
    if chart_type == ChartType::Combo {
        return Err(Error::with_path(
            ErrorKind::InvalidChartType,
            format!("ops[{i}].type"),
        ));
    }
    let of_pie_type =
        crate::chart::parse_of_pie_type(obj.get("ofPieType"), &format!("ops[{i}].ofPieType"))?;
    crate::chart::reject_of_pie_on_other(
        chart_type,
        of_pie_type.as_deref(),
        &format!("ops[{i}].ofPieType"),
    )?;
    let extra = crate::chart::parse_chart_extra(obj, chart_type, 0, &format!("ops[{i}]"), false)?;
    Ok(charts::SetChartTypeOp {
        id,
        target,
        chart_type,
        of_pie_type,
        extra,
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

fn parse_insert_trendline(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<charts::InsertTrendlineOp, Error> {
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
    let target = charts::parse_series_id(&id, i)?;
    let spec = crate::chart::remain::parse_trendline_write(obj, i, true)?;
    Ok(charts::InsertTrendlineOp { id, target, spec })
}

fn parse_set_trendline(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<charts::SetTrendlineOp, Error> {
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
    let (target, tl_index) = charts::parse_trendline_id(&id, i)?;
    let spec = crate::chart::remain::parse_trendline_write(obj, i, false)?;
    Ok(charts::SetTrendlineOp {
        id,
        target,
        tl_index,
        spec,
    })
}

fn parse_delete_trendline(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<charts::DeleteTrendlineOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let (target, tl_index) = charts::parse_trendline_id(&id, i)?;
    Ok(charts::DeleteTrendlineOp {
        id,
        target,
        tl_index,
    })
}

fn parse_insert_error_bars(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<charts::InsertErrorBarsOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op", "id", "dir", "barType", "valType", "value", "plus", "minus",
        ],
    )?;
    let id = require_id(obj, i)?;
    let target = charts::parse_series_id(&id, i)?;
    let spec = crate::chart::remain::parse_error_bars_write(obj, i, true)?;
    Ok(charts::InsertErrorBarsOp { id, target, spec })
}

fn parse_set_error_bars(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<charts::SetErrorBarsOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op", "id", "dir", "barType", "valType", "value", "plus", "minus",
        ],
    )?;
    let id = require_id(obj, i)?;
    let (target, eb_index) = charts::parse_error_bars_id(&id, i)?;
    let spec = crate::chart::remain::parse_error_bars_write(obj, i, false)?;
    Ok(charts::SetErrorBarsOp {
        id,
        target,
        eb_index,
        spec,
    })
}

fn parse_delete_error_bars(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<charts::DeleteErrorBarsOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let (target, eb_index) = charts::parse_error_bars_id(&id, i)?;
    Ok(charts::DeleteErrorBarsOp {
        id,
        target,
        eb_index,
    })
}

fn parse_set_series_axis(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<charts::SetChartSeriesAxisOp, Error> {
    deny_keys(obj, i, &["op", "id", "axis"])?;
    let id = require_id(obj, i)?;
    let target = charts::parse_series_id(&id, i)?;
    let secondary = crate::chart::remain::parse_series_axis(obj, i)?;
    Ok(charts::SetChartSeriesAxisOp {
        id,
        target,
        secondary,
    })
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
