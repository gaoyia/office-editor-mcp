//! docx request：types。

use super::super::charts::{self};
use super::super::hf::{HfKind, HfType};
use super::super::math_expr::MathBody;
use super::super::para::{self, BodyPara, NoteKind, ParaTarget, TxbxTarget};
use super::super::picture::{PicKind, PicTarget, WrapKind, WrapPos};
use super::super::smartart;
use super::super::tbl::{InsertAnchor, TableRef};

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
pub enum ParaRunSource {
    Text(String),
    Range {
        start: usize,
        end: usize,
    },
    Match {
        text: Option<String>,
        after: Option<String>,
        before: Option<String>,
        nth: usize,
    },
}

#[derive(Debug, Clone)]
pub struct ParaRunPiece {
    pub source: ParaRunSource,
    pub format: Option<super::super::create_round5::RunFormatOwned>,
}

#[derive(Debug, Clone)]
pub struct SetParaRunsOp {
    pub id: String,
    pub runs: Vec<ParaRunPiece>,
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
pub struct SetTextBoxTextAnchorOp {
    pub id: String,
    pub target: TxbxTarget,
    /// `None` 表示 JSON `null`，去掉 `wps:bodyPr/@anchor`。
    pub value: Option<String>,
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
    pub host: super::super::group::GroupHost,
    pub members: Vec<super::super::group::GroupMember>,
}

#[derive(Debug, Clone)]
pub struct UngroupOp {
    pub id: String,
    pub member: super::super::group::GroupMember,
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
    pub target: super::super::revision::RevRef,
}

#[derive(Debug, Clone)]
pub enum InsertHyperlinkHost {
    Para { para: ParaTarget, text: String },
    Picture(PicTarget),
    TextBox(TxbxTarget),
    Shape(TxbxTarget),
    Connector(TxbxTarget),
    Ink(TxbxTarget),
    Model3d(TxbxTarget),
    Chart(charts::ChartTarget),
    SmartArt(smartart::DiagramTarget),
}

#[derive(Debug, Clone)]
pub enum HyperlinkTarget {
    Link(super::super::link::LinkRef),
    Picture(PicTarget),
    TextBox(TxbxTarget),
    Shape(TxbxTarget),
    Connector(TxbxTarget),
    Ink(TxbxTarget),
    Model3d(TxbxTarget),
    Chart(charts::ChartTarget),
    SmartArt(smartart::DiagramTarget),
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
    pub target: super::super::link::BookmarkRef,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct DeleteBookmarkOp {
    pub id: String,
    pub target: super::super::link::BookmarkRef,
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
    pub target: super::super::perm::PermRef,
}

#[derive(Debug, Clone)]
pub struct InsertFieldOp {
    pub id: String,
    pub para: ParaTarget,
    pub instr: String,
    pub result: Option<String>,
    pub form: Option<super::super::field::FormWrite>,
}

#[derive(Debug, Clone)]
pub struct SetFieldOp {
    pub id: String,
    pub target: super::super::field::FieldRef,
    pub instr: String,
}

#[derive(Debug, Clone)]
pub struct DeleteFieldOp {
    pub id: String,
    pub target: super::super::field::FieldRef,
    pub keep: super::super::field::FieldKeep,
}

#[derive(Debug, Clone)]
pub struct InsertNumberingOp {
    pub kind: super::super::numbering::NumberingKind,
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
    pub look: Option<Option<super::super::tbl::TableLookPatch>>,
}

#[derive(Debug, Clone)]
pub struct SetTableFloatOp {
    pub id: String,
    pub target: TableRef,
    pub pos: Option<super::super::tbl::TableFloat>,
}

#[derive(Debug, Clone)]
pub struct SetTableWidthOp {
    pub id: String,
    pub target: TableRef,
    pub patch: super::super::tbl::TableWidthPatch,
}

#[derive(Debug, Clone)]
pub struct SetTableAlignOp {
    pub id: String,
    pub target: TableRef,
    pub align: Option<Option<String>>,
    pub indent: Option<Option<u32>>,
}

#[derive(Debug, Clone)]
pub struct SetTableRowHeightOp {
    pub id: String,
    pub target: TableRef,
    pub h: Option<u32>,
    pub rule: Option<String>,
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
pub struct SetTableBordersOp {
    pub id: String,
    pub target: TableRef,
    pub borders: Option<super::super::tbl::TableBordersPatch>,
}

#[derive(Debug, Clone)]
pub struct SetCellBordersOp {
    pub id: String,
    pub target: TableRef,
    pub borders: Option<super::super::tbl::TableBordersPatch>,
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
    pub tl2br: Option<Option<super::super::tbl::DiagonalBorder>>,
    pub tr2bl: Option<Option<super::super::tbl::DiagonalBorder>>,
    pub v_align: Option<Option<String>>,
    pub no_wrap: Option<Option<bool>>,
    pub text_direction: Option<Option<String>>,
    pub fit_text: Option<Option<bool>>,
    pub hide_mark: Option<Option<bool>>,
    pub headers: Option<Option<Vec<String>>>,
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
    pub borders: Option<Option<super::super::format::ParaBordersPatch>>,
    pub shd: Option<Option<String>>,
    pub tabs: Option<Option<Vec<super::super::format::TabStop>>>,
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
    pub east_asian_layout: Option<Option<super::super::format::EastAsianLayout>>,
    pub highlight: Option<Option<String>>,
    pub strike: Option<Option<bool>>,
    pub dstrike: Option<Option<bool>>,
    pub vert_align: Option<Option<String>>,
    pub spacing: Option<Option<i32>>,
    pub kern: Option<Option<u32>>,
    pub scale: Option<Option<u32>>,
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
    pub break_type: super::super::sect::SectionBreakType,
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
    pub kind: super::super::field::CrossRefKind,
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
    pub style: super::super::font_embed::FontStyle,
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
    pub fill: Option<super::super::page_bg::PageFill>,
}

#[derive(Debug, Clone)]
pub struct SetPageBordersOp {
    pub id: String,
    pub n: usize,
    pub enabled: Option<bool>,
    pub offset_from: Option<String>,
    pub display: Option<String>,
    pub z_order: Option<String>,
    pub top: Option<Option<super::super::page_bg::BorderSide>>,
    pub left: Option<Option<super::super::page_bg::BorderSide>>,
    pub bottom: Option<Option<super::super::page_bg::BorderSide>>,
    pub right: Option<Option<super::super::page_bg::BorderSide>>,
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
    pub kind: super::super::math::MathKind,
    pub body: MathBody,
    pub jc: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SetMathOp {
    pub id: String,
    pub target: super::super::math::MathRef,
    pub body: Option<MathBody>,
    pub jc: Option<Option<String>>,
}

#[derive(Debug, Clone)]
pub struct DeleteMathOp {
    pub id: String,
    pub target: super::super::math::MathRef,
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
    pub target: super::super::ruby::RubyRef,
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
    pub target: super::super::ruby::RubyRef,
}

#[derive(Debug, Clone)]
pub struct SetEastAsianSettingsOp {
    pub character_spacing: Option<Option<String>>,
    pub strict: Option<Option<bool>>,
    pub no_before: Option<Option<super::super::east_asia::KinsokuChars>>,
    pub no_after: Option<Option<super::super::east_asia::KinsokuChars>>,
    pub theme_font_lang: Option<Option<super::super::east_asia::ThemeFontLang>>,
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
    pub kind: super::super::sdt::SdtKind,
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
    pub target: super::super::sdt::SdtRef,
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
    pub target: super::super::sdt::SdtRef,
    pub keep: super::super::sdt::SdtKeep,
}

#[derive(Debug, Clone)]
pub enum DocxOp {
    SetParaText(SetParaTextOp),
    SetParaRuns(SetParaRunsOp),
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
    SetTextBoxTextAnchor(SetTextBoxTextAnchorOp),
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
    SetTableWidth(SetTableWidthOp),
    SetTableAlign(SetTableAlignOp),
    SetTableRowHeight(SetTableRowHeightOp),
    SetTableCellMar(SetTableCellMarOp),
    SetTableBorders(SetTableBordersOp),
    SetCellBorders(SetCellBordersOp),
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
    InsertSmartArt(super::super::smartart::InsertSmartArtOp),
    DeleteSmartArt(super::super::smartart::DeleteSmartArtOp),
    SetSmartArtText(super::super::smartart::SetSmartArtTextOp),
    AddSmartArtPoint(super::super::smartart::AddSmartArtPointOp),
    DeleteSmartArtPoint(super::super::smartart::DeleteSmartArtPointOp),
    SetSmartArtSize(super::super::smartart::SetSmartArtSizeOp),
    SetSmartArtWrap(super::super::smartart::SetSmartArtWrapOp),
    SetSmartArtColors(super::super::smartart::SetSmartArtColorsOp),
    SetSmartArtStyle(super::super::smartart::SetSmartArtStyleOp),
    SetSmartArtLayout(super::super::smartart::SetSmartArtLayoutOp),
    InsertOle(super::super::ole::InsertOleOp),
    DeleteOle(super::super::ole::DeleteOleOp),
    SetOleSize(super::super::ole::SetOleSizeOp),
    InsertInkCanvas(super::super::ink::InsertInkCanvasOp),
    ReplaceInkCanvas(super::super::ink::ReplaceInkCanvasOp),
    DeleteInkCanvas(super::super::ink::DeleteInkCanvasOp),
    SetInkCanvasSize(super::super::ink::SetInkCanvasSizeOp),
    SetInkCanvasWrap(super::super::ink::SetInkCanvasWrapOp),
    InsertModel3d(super::super::model3d::InsertModel3dOp),
    ReplaceModel3d(super::super::model3d::ReplaceModel3dOp),
    DeleteModel3d(super::super::model3d::DeleteModel3dOp),
    SetModel3dSize(super::super::model3d::SetModel3dSizeOp),
    SetModel3dWrap(super::super::model3d::SetModel3dWrapOp),
    SetModel3dCamera(super::super::model3d::SetModel3dCameraOp),
    InsertWatermark(super::super::watermark::InsertWatermarkOp),
    DeleteWatermark(super::super::watermark::DeleteWatermarkOp),
    SetProtection(super::super::protect::SetProtectionOp),
    SetThemeColors(super::super::theme::SetThemeColorsOp),
    SetThemeFonts(super::super::theme::SetThemeFontsOp),
    SetThemeFmt(super::super::theme::SetThemeFmtOp),
    SetThemeExtraSchemes(super::super::theme::SetThemeExtraSchemesOp),
    SetThemeObjectDefaults(super::super::theme::SetThemeObjectDefaultsOp),
    SetThemeOverride(super::super::theme::SetThemeOverrideOp),
    SetProps(crate::props::SetPropsOp),
    SetCustomProp(crate::props::SetCustomPropOp),
    DeleteCustomProp(crate::props::DeleteCustomPropOp),
}
