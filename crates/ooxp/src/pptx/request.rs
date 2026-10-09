//! pptx 自己的 scope 与 ops 解析。

use serde_json::{Map, Value};

use crate::chart::{ChartType, NewSeries};
use crate::error::{Error, ErrorKind, IdReason, ScopeNeed, TypeName};
use crate::json;
use crate::smartart::{ColorsKind, LayoutKind, NewPoint, Role, StyleKind};

use super::chart;
use super::comments;
use super::connector;
use super::custom_show;
use super::field;
use super::group;
use super::hyperlink;
use super::media;
use super::ole;
use super::picture;
use super::section;
use super::shape;
use super::smartart;
use super::table;
use super::timing;
use super::transition;
use super::zoom;

#[derive(Debug, Clone)]
pub struct InspectScope {
    pub slide: Option<u32>,
    pub max_shapes: u32,
    pub pictures: bool,
    pub media: bool,
    pub charts: bool,
    pub smart_art: bool,
    pub notes: bool,
    pub comments: bool,
    pub hyperlinks: bool,
    pub fields: bool,
    pub tables: bool,
    pub connectors: bool,
    pub timing: bool,
    pub masters: bool,
    pub notes_master: bool,
    pub handout: bool,
    pub sections: bool,
    pub custom_shows: bool,
    pub ole_objects: bool,
    pub model3ds: bool,
    pub maths: bool,
    pub zooms: bool,
}

#[derive(Debug, Clone)]
pub struct SetShapeTextOp {
    pub id: String,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct SetShapeFormatOp {
    pub id: String,
    pub format: crate::drawing_text::TextFormat,
}

#[derive(Debug, Clone)]
pub enum InsertShapeHost {
    Slide {
        slide: usize,
        on_notes: bool,
    },
    Master {
        master: usize,
        layout: Option<usize>,
    },
}

#[derive(Debug, Clone)]
pub struct InsertShapeOp {
    pub host: InsertShapeHost,
    pub text: String,
    pub x: Option<i64>,
    pub y: Option<i64>,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
    pub prst: String,
    pub format: Option<crate::drawing_text::TextFormat>,
}

#[derive(Debug, Clone)]
pub struct DeleteShapeOp {
    pub id: String,
    pub target: Option<super::shape::ShapeTarget>,
    pub placeholder: Option<super::master::PlaceholderId>,
}

#[derive(Debug, Clone)]
pub struct SetShapePosOp {
    pub id: String,
    pub target: Option<super::shape::ShapeTarget>,
    pub placeholder: Option<super::master::PlaceholderId>,
    pub cxn: Option<super::connector::CxnTarget>,
    pub x: Option<i64>,
    pub y: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct SetShapeSizeOp {
    pub id: String,
    pub target: Option<super::shape::ShapeTarget>,
    pub placeholder: Option<super::master::PlaceholderId>,
    pub cxn: Option<super::connector::CxnTarget>,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct SetShapeFillOp {
    pub id: String,
    pub target: Option<super::shape::ShapeTarget>,
    pub placeholder: Option<super::master::PlaceholderId>,
    pub color: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SetShapeLineOp {
    pub id: String,
    pub target: Option<super::shape::ShapeTarget>,
    pub placeholder: Option<super::master::PlaceholderId>,
    pub cxn: Option<super::connector::CxnTarget>,
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
    pub master: Option<(usize, Option<usize>)>,
    pub type_name: Option<String>,
    pub dur: Option<u32>,
    pub adv_click: bool,
    pub adv_tm: Option<u32>,
    pub dir: Option<String>,
    pub inv_x: bool,
    pub inv_y: bool,
    pub sound: Option<String>,
    pub orient: Option<String>,
    pub spokes: Option<u32>,
}

#[derive(Debug, Clone)]
pub enum AnimTarget {
    Slide {
        slide: usize,
    },
    Notes {
        slide: usize,
    },
    Master {
        master: usize,
        layout: Option<usize>,
    },
    NotesMaster,
    Handout,
}

impl AnimTarget {
    pub fn prefix(&self) -> String {
        match self {
            Self::Slide { slide } => format!("slide:{slide}"),
            Self::Notes { slide } => format!("slide:{slide}/notes"),
            Self::Master {
                master,
                layout: None,
            } => format!("master:{master}"),
            Self::Master {
                master,
                layout: Some(k),
            } => format!("master:{master}/layout:{k}"),
            Self::NotesMaster => "notesMaster".into(),
            Self::Handout => "handout".into(),
        }
    }

    pub fn placeholders_only(&self) -> bool {
        matches!(
            self,
            Self::Master { .. } | Self::NotesMaster | Self::Handout
        )
    }
}

#[derive(Debug, Clone)]
pub struct InsertAnimOp {
    pub id: String,
    pub target: AnimTarget,
    pub type_name: String,
    pub trigger: String,
    pub delay: u32,
    pub dur: u32,
    pub repeat: timing::RepeatWrite,
    pub sound: Option<String>,
    pub para: Option<u32>,
    pub click_on: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DeleteAnimOp {
    pub id: String,
    pub target: AnimTarget,
    pub index: usize,
}

#[derive(Debug, Clone)]
pub struct SetAnimOp {
    pub id: String,
    pub target: AnimTarget,
    pub index: usize,
    pub type_name: Option<String>,
    pub trigger: Option<String>,
    pub delay: Option<u32>,
    pub dur: Option<u32>,
    pub repeat: Option<timing::RepeatWrite>,
    pub sound: Option<Option<String>>,
    pub para: Option<Option<u32>>,
    pub click_on: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MoveAnimOp {
    pub id: String,
    pub target: AnimTarget,
    pub index: usize,
    pub dest: usize,
}

#[derive(Debug, Clone)]
pub struct SetSlideLayoutOp {
    pub slide: usize,
    pub master: usize,
    pub layout: usize,
}

#[derive(Debug, Clone)]
pub struct InsertLayoutOp {
    pub master: usize,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct DeleteLayoutOp {
    pub master: usize,
    pub layout: usize,
}

#[derive(Debug, Clone)]
pub struct InsertPlaceholderOp {
    pub host: super::master::PlaceholderHost,
    pub type_name: String,
    pub text: String,
    pub x: Option<i64>,
    pub y: Option<i64>,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct DeletePlaceholderOp {
    pub id: String,
    pub placeholder: super::master::PlaceholderId,
}

#[derive(Debug, Clone)]
pub struct InsertMasterOp {
    pub from: usize,
}

#[derive(Debug, Clone)]
pub struct DeleteMasterOp {
    pub master: usize,
}

#[derive(Debug, Clone)]
pub struct SetSlideSizeOp {
    pub cx: i64,
    pub cy: i64,
    pub type_name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SetThemeColorsOp {
    pub master: usize,
    pub slots: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub struct SetThemeFontsOp {
    pub master: usize,
    pub fonts: Vec<(super::theme::FontSlot, String)>,
}

#[derive(Debug, Clone)]
pub struct SetHandoutTextOp {
    pub header: Option<String>,
    pub footer: Option<String>,
    pub date: Option<String>,
    pub sld_num: Option<String>,
}

#[derive(Debug, Clone)]
pub enum PptxLinkKind {
    Url(String),
    File(String),
    Slide(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkEvent {
    Click,
    Hover,
}

impl LinkEvent {
    pub fn cnv_tag(self) -> &'static str {
        match self {
            Self::Click => "hlinkClick",
            Self::Hover => "hlinkHover",
        }
    }

    pub fn run_tag(self) -> &'static str {
        match self {
            Self::Click => "hlinkClick",
            Self::Hover => "hlinkMouseOver",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkOn {
    Shape,
    Run,
    Pic,
    Table,
    Chart,
    SmartArt,
    Cell,
}

impl LinkOn {
    pub fn is_run_like(self) -> bool {
        matches!(self, Self::Run | Self::Cell)
    }
}

#[derive(Debug, Clone)]
pub enum LinkInsertHost {
    Shape(super::shape::ShapeTarget),
    Pic(picture::PicTarget),
    Table(table::TableTarget),
    Chart(chart::ChartTarget),
    SmartArt(smartart::SmartArtTarget),
    Cell(table::CellTarget),
}

impl LinkInsertHost {
    pub fn parts(&self) -> (usize, bool, Option<(usize, Option<usize>)>) {
        match self {
            Self::Shape(t) => (t.slide, t.on_notes, t.master),
            Self::Pic(t) => (t.slide, t.on_notes, t.master),
            Self::Table(t) => (t.slide, t.on_notes, t.master),
            Self::Chart(t) => (t.slide, t.on_notes, t.master),
            Self::SmartArt(t) => (t.slide, t.on_notes, t.master),
            Self::Cell(c) => (c.table.slide, c.table.on_notes, c.table.master),
        }
    }

    fn default_on(&self) -> LinkOn {
        match self {
            Self::Shape(_) => LinkOn::Shape,
            Self::Pic(_) => LinkOn::Pic,
            Self::Table(_) => LinkOn::Table,
            Self::Chart(_) => LinkOn::Chart,
            Self::SmartArt(_) => LinkOn::SmartArt,
            Self::Cell(_) => LinkOn::Cell,
        }
    }

    fn on_ok(&self, on: LinkOn) -> bool {
        matches!(
            (self, on),
            (Self::Shape(_), LinkOn::Shape | LinkOn::Run)
                | (Self::Pic(_), LinkOn::Pic)
                | (Self::Table(_), LinkOn::Table)
                | (Self::Chart(_), LinkOn::Chart)
                | (Self::SmartArt(_), LinkOn::SmartArt)
                | (Self::Cell(_), LinkOn::Cell)
        )
    }
}

#[derive(Debug, Clone)]
pub struct InsertHyperlinkOp {
    pub id: String,
    pub host: LinkInsertHost,
    pub target: PptxLinkKind,
    pub tooltip: Option<String>,
    pub on: LinkOn,
    pub event: LinkEvent,
    pub text: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SetHyperlinkOp {
    pub id: String,
    pub slide: usize,
    pub on_notes: bool,
    pub master: Option<(usize, Option<usize>)>,
    pub index: usize,
    pub target: Option<PptxLinkKind>,
    pub tooltip: Option<Option<String>>,
}

#[derive(Debug, Clone)]
pub struct DeleteHyperlinkOp {
    pub id: String,
    pub slide: usize,
    pub on_notes: bool,
    pub master: Option<(usize, Option<usize>)>,
    pub index: usize,
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
    pub master: Option<(usize, Option<usize>)>,
    pub comment: Option<comments::CommentTarget>,
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
pub struct MediaBookmark {
    pub name: String,
    pub time: f64,
}

#[derive(Debug, Clone)]
pub struct MediaTrackWrite {
    pub id: Option<String>,
    pub label: String,
    pub lang: Option<String>,
    pub file: String,
}

#[derive(Debug, Clone)]
pub struct InsertMediaOp {
    pub slide: usize,
    pub on_notes: bool,
    pub master: Option<(usize, Option<usize>)>,
    pub file: String,
    pub x: Option<i64>,
    pub y: Option<i64>,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
    pub st: Option<f64>,
    pub end: Option<f64>,
    pub bookmarks: Option<Vec<MediaBookmark>>,
    pub tracks: Option<Vec<MediaTrackWrite>>,
    pub track_display_loc: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SetMediaOp {
    pub id: String,
    pub target: media::MediaTarget,
    pub st: Option<f64>,
    pub end: Option<f64>,
    pub bookmarks: Option<Vec<MediaBookmark>>,
    pub tracks: Option<Vec<MediaTrackWrite>>,
    pub track_display_loc: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CreateHyperlink {
    pub target: PptxLinkKind,
    pub tooltip: Option<String>,
    pub on_run: bool,
    pub event: LinkEvent,
}

#[derive(Debug, Clone, Copy)]
pub enum CreateAnimHost {
    Shape(usize),
    Picture(usize),
    Video(usize),
    Audio(usize),
    Table(usize),
    Chart(usize),
    SmartArt(usize),
}

#[derive(Debug, Clone)]
pub struct CreateAnim {
    pub type_name: String,
    pub trigger: String,
    pub delay: u32,
    pub dur: u32,
    pub repeat: timing::RepeatWrite,
    pub sound: Option<String>,
    pub para: Option<u32>,
    pub host: CreateAnimHost,
    pub click_on: Option<CreateAnimHost>,
}

#[derive(Debug, Clone)]
pub struct CreateSection {
    pub name: String,
    pub start: usize,
}

#[derive(Debug, Clone)]
pub struct DeleteMediaOp {
    pub id: String,
    pub target: media::MediaTarget,
}

#[derive(Debug, Clone)]
pub struct InsertSlideSectionOp {
    pub name: String,
    pub start: Option<usize>,
    pub after: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct SetSlideSectionNameOp {
    pub section: usize,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct DeleteSlideSectionOp {
    pub section: usize,
}

#[derive(Debug, Clone)]
pub struct SetSlideSectionOp {
    pub slide: usize,
    pub section: usize,
}

#[derive(Debug, Clone)]
pub struct InsertCustomShowOp {
    pub name: String,
    pub slides: Vec<usize>,
    pub after: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct SetCustomShowOp {
    pub show: usize,
    pub name: Option<String>,
    pub slides: Option<Vec<usize>>,
}

#[derive(Debug, Clone)]
pub struct DeleteCustomShowOp {
    pub show: usize,
}

#[derive(Debug, Clone)]
pub struct InsertChartOp {
    pub slide: usize,
    pub on_notes: bool,
    pub master: Option<(usize, Option<usize>)>,
    pub chart_type: ChartType,
    pub of_pie_type: Option<String>,
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
    pub of_pie_type: Option<String>,
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
    pub on_notes: bool,
    pub master: Option<(usize, Option<usize>)>,
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
    pub on_notes: bool,
    pub master: Option<(usize, Option<usize>)>,
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
pub struct InsertConnectorOp {
    pub slide: usize,
    pub on_notes: bool,
    pub master: Option<(usize, Option<usize>)>,
    pub prst: String,
    pub x: Option<i64>,
    pub y: Option<i64>,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct DeleteConnectorOp {
    pub id: String,
    pub target: connector::CxnTarget,
}

#[derive(Debug, Clone)]
pub struct GroupShapesOp {
    pub slide: usize,
    pub ids: Vec<String>,
    pub members: Vec<group::MemberId>,
}

#[derive(Debug, Clone)]
pub struct UngroupOp {
    pub id: String,
    pub member: group::MemberId,
}

#[derive(Debug, Clone)]
pub enum PptxOp {
    SetShapeText(SetShapeTextOp),
    InsertShape(InsertShapeOp),
    InsertConnector(InsertConnectorOp),
    DeleteConnector(DeleteConnectorOp),
    GroupShapes(GroupShapesOp),
    Ungroup(UngroupOp),
    DeleteShape(DeleteShapeOp),
    SetShapePos(SetShapePosOp),
    SetShapeSize(SetShapeSizeOp),
    SetShapeFill(SetShapeFillOp),
    SetShapeFormat(SetShapeFormatOp),
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
    InsertVideo(InsertMediaOp),
    InsertAudio(InsertMediaOp),
    DeleteMedia(DeleteMediaOp),
    SetMedia(SetMediaOp),
    InsertSlideSection(InsertSlideSectionOp),
    SetSlideSectionName(SetSlideSectionNameOp),
    DeleteSlideSection(DeleteSlideSectionOp),
    SetSlideSection(SetSlideSectionOp),
    InsertCustomShow(InsertCustomShowOp),
    SetCustomShow(SetCustomShowOp),
    DeleteCustomShow(DeleteCustomShowOp),
    InsertOle(super::ole::InsertOleOp),
    DeleteOle(super::ole::DeleteOleOp),
    InsertModel3d(super::model3d::InsertModel3dOp),
    ReplaceModel3d(super::model3d::ReplaceModel3dOp),
    DeleteModel3d(super::model3d::DeleteModel3dOp),
    InsertMath(super::omml::InsertMathOp),
    SetMath(super::omml::SetMathOp),
    DeleteMath(super::omml::DeleteMathOp),
    InsertZoom(super::zoom::InsertZoomOp),
    DeleteZoom(super::zoom::DeleteZoomOp),
    SetZoom(super::zoom::SetZoomOp),
    SetShowPr(super::show::SetShowPrOp),
    SetProtection(super::protect::SetProtectionOp),
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
    InsertAnim(InsertAnimOp),
    SetAnim(SetAnimOp),
    DeleteAnim(DeleteAnimOp),
    MoveAnim(MoveAnimOp),
    SetSlideLayout(SetSlideLayoutOp),
    InsertLayout(InsertLayoutOp),
    DeleteLayout(DeleteLayoutOp),
    InsertPlaceholder(InsertPlaceholderOp),
    DeletePlaceholder(DeletePlaceholderOp),
    InsertMaster(InsertMasterOp),
    DeleteMaster(DeleteMasterOp),
    SetSlideSize(SetSlideSizeOp),
    SetThemeColors(SetThemeColorsOp),
    SetThemeFonts(SetThemeFontsOp),
    SetHandoutText(SetHandoutTextOp),
    InsertHyperlink(InsertHyperlinkOp),
    SetHyperlink(SetHyperlinkOp),
    DeleteHyperlink(DeleteHyperlinkOp),
    InsertField(field::InsertFieldOp),
    SetField(field::SetFieldOp),
    DeleteField(field::DeleteFieldOp),
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
            "media",
            "charts",
            "smartArt",
            "notes",
            "comments",
            "hyperlinks",
            "fields",
            "tables",
            "connectors",
            "timing",
            "masters",
            "notesMaster",
            "handout",
            "sections",
            "customShows",
            "oleObjects",
            "model3ds",
            "maths",
            "zooms",
        ],
    )?;
    let pictures = parse_bool_field(obj.get("pictures"), "pictures")?;
    let media = parse_bool_field(obj.get("media"), "media")?;
    let charts = parse_bool_field(obj.get("charts"), "charts")?;
    let smart_art = parse_bool_field(obj.get("smartArt"), "smartArt")?;
    let notes = parse_bool_field(obj.get("notes"), "notes")?;
    let comments = parse_bool_field(obj.get("comments"), "comments")?;
    let hyperlinks = parse_bool_field(obj.get("hyperlinks"), "hyperlinks")?;
    let fields = parse_bool_field(obj.get("fields"), "fields")?;
    let tables = parse_bool_field(obj.get("tables"), "tables")?;
    let connectors = parse_bool_field(obj.get("connectors"), "connectors")?;
    let timing = parse_bool_field(obj.get("timing"), "timing")?;
    let masters = parse_bool_field(obj.get("masters"), "masters")?;
    let notes_master = parse_bool_field(obj.get("notesMaster"), "notesMaster")?;
    let handout = parse_bool_field(obj.get("handout"), "handout")?;
    let sections = parse_bool_field(obj.get("sections"), "sections")?;
    let custom_shows = parse_bool_field(obj.get("customShows"), "customShows")?;
    let ole_objects = parse_bool_field(obj.get("oleObjects"), "oleObjects")?;
    let model3ds = parse_bool_field(obj.get("model3ds"), "model3ds")?;
    let maths = parse_bool_field(obj.get("maths"), "maths")?;
    let zooms = parse_bool_field(obj.get("zooms"), "zooms")?;
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
        media,
        charts,
        smart_art,
        notes,
        comments,
        hyperlinks,
        fields,
        tables,
        connectors,
        timing,
        masters,
        notes_master,
        handout,
        sections,
        custom_shows,
        ole_objects,
        model3ds,
        maths,
        zooms,
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

fn parse_opt_bool(obj: &Map<String, Value>, i: usize, field: &str) -> Result<Option<bool>, Error> {
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
            "insertConnector" => PptxOp::InsertConnector(parse_insert_connector(obj, i)?),
            "deleteConnector" => PptxOp::DeleteConnector(parse_delete_connector(obj, i)?),
            "groupShapes" => PptxOp::GroupShapes(parse_group_shapes(obj, i)?),
            "ungroup" => PptxOp::Ungroup(parse_ungroup(obj, i)?),
            "deleteShape" => PptxOp::DeleteShape(parse_delete_shape(obj, i)?),
            "setShapePos" => PptxOp::SetShapePos(parse_set_shape_pos(obj, i)?),
            "setShapeSize" => PptxOp::SetShapeSize(parse_set_shape_size(obj, i)?),
            "setShapeFill" => PptxOp::SetShapeFill(parse_set_shape_fill(obj, i)?),
            "setShapeFormat" => PptxOp::SetShapeFormat(parse_set_shape_format(obj, i)?),
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
            "insertVideo" => PptxOp::InsertVideo(parse_insert_media(obj, i, "insertVideo")?),
            "insertAudio" => PptxOp::InsertAudio(parse_insert_media(obj, i, "insertAudio")?),
            "deleteMedia" => PptxOp::DeleteMedia(parse_delete_media(obj, i)?),
            "setMedia" => PptxOp::SetMedia(parse_set_media(obj, i)?),
            "insertSlideSection" => PptxOp::InsertSlideSection(parse_insert_slide_section(obj, i)?),
            "setSlideSectionName" => {
                PptxOp::SetSlideSectionName(parse_set_slide_section_name(obj, i)?)
            }
            "deleteSlideSection" => PptxOp::DeleteSlideSection(parse_delete_slide_section(obj, i)?),
            "setSlideSection" => PptxOp::SetSlideSection(parse_set_slide_section(obj, i)?),
            "insertCustomShow" => PptxOp::InsertCustomShow(parse_insert_custom_show(obj, i)?),
            "setCustomShow" => PptxOp::SetCustomShow(parse_set_custom_show(obj, i)?),
            "deleteCustomShow" => PptxOp::DeleteCustomShow(parse_delete_custom_show(obj, i)?),
            "insertOle" => PptxOp::InsertOle(parse_insert_ole(obj, i)?),
            "deleteOle" => PptxOp::DeleteOle(parse_delete_ole(obj, i)?),
            "insertModel3d" => PptxOp::InsertModel3d(parse_insert_model3d(obj, i)?),
            "replaceModel3d" => PptxOp::ReplaceModel3d(parse_replace_model3d(obj, i)?),
            "deleteModel3d" => PptxOp::DeleteModel3d(parse_delete_model3d(obj, i)?),
            "insertMath" => PptxOp::InsertMath(parse_insert_math(obj, i)?),
            "setMath" => PptxOp::SetMath(parse_set_math(obj, i)?),
            "deleteMath" => PptxOp::DeleteMath(parse_delete_math(obj, i)?),
            "insertZoom" => PptxOp::InsertZoom(parse_insert_zoom(obj, i)?),
            "deleteZoom" => PptxOp::DeleteZoom(parse_delete_zoom(obj, i)?),
            "setZoom" => PptxOp::SetZoom(parse_set_zoom(obj, i)?),
            "setShowPr" => PptxOp::SetShowPr(parse_set_show_pr(obj, i)?),
            "setProtection" => PptxOp::SetProtection(parse_set_protection(obj, i)?),
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
            "insertAnim" => PptxOp::InsertAnim(parse_insert_anim(obj, i)?),
            "setAnim" => PptxOp::SetAnim(parse_set_anim(obj, i)?),
            "deleteAnim" => PptxOp::DeleteAnim(parse_delete_anim(obj, i)?),
            "moveAnim" => PptxOp::MoveAnim(parse_move_anim(obj, i)?),
            "setSlideLayout" => PptxOp::SetSlideLayout(parse_set_slide_layout(obj, i)?),
            "insertLayout" => PptxOp::InsertLayout(parse_insert_layout(obj, i)?),
            "deleteLayout" => PptxOp::DeleteLayout(parse_delete_layout(obj, i)?),
            "insertPlaceholder" => PptxOp::InsertPlaceholder(parse_insert_placeholder(obj, i)?),
            "deletePlaceholder" => PptxOp::DeletePlaceholder(parse_delete_placeholder(obj, i)?),
            "insertMaster" => PptxOp::InsertMaster(parse_insert_master(obj, i)?),
            "deleteMaster" => PptxOp::DeleteMaster(parse_delete_master(obj, i)?),
            "setSlideSize" => PptxOp::SetSlideSize(parse_set_slide_size(obj, i)?),
            "setThemeColors" => PptxOp::SetThemeColors(parse_set_theme_colors(obj, i)?),
            "setThemeFonts" => PptxOp::SetThemeFonts(parse_set_theme_fonts(obj, i)?),
            "setHandoutText" => PptxOp::SetHandoutText(parse_set_handout_text(obj, i)?),
            "insertHyperlink" => PptxOp::InsertHyperlink(parse_insert_hyperlink(obj, i)?),
            "setHyperlink" => PptxOp::SetHyperlink(parse_set_hyperlink(obj, i)?),
            "deleteHyperlink" => PptxOp::DeleteHyperlink(parse_delete_hyperlink(obj, i)?),
            "insertField" => PptxOp::InsertField(parse_insert_field(obj, i)?),
            "setField" => PptxOp::SetField(parse_set_field(obj, i)?),
            "deleteField" => PptxOp::DeleteField(parse_delete_field(obj, i)?),
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
            | "insertSparkline"
            | "setSparkline"
            | "deleteSparkline"
            | "insertSlicer"
            | "deleteSlicer"
            | "insertConnection"
            | "deleteConnection"
            | "insertExternalLink"
            | "deleteExternalLink"
            | "insertCustomView"
            | "deleteCustomView"
            | "showCustomView"
            | "insertBookmark"
            | "setBookmark"
            | "deleteBookmark"
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
            | "insertCaption"
            | "deleteCaption"
            | "insertCrossRef"
            | "deleteCrossRef"
            | "insertIndex"
            | "deleteIndex"
            | "insertIndexEntry"
            | "deleteIndexEntry"
            | "insertSource"
            | "deleteSource"
            | "insertCitation"
            | "deleteCitation"
            | "insertBibliography"
            | "deleteBibliography"
            | "insertDocPart"
            | "deleteDocPart"
            | "insertDocPartContent"
            | "insertEmbeddedFont"
            | "deleteEmbeddedFont"
            | "setFontEmbed"
            | "setPageBackground"
            | "setPageBorders"
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
            | "setSortState"
            | "groupRows"
            | "ungroupRows"
            | "groupCols"
            | "ungroupCols"
            | "setOutline"
            | "setSmartArtWrap"
            | "insertWatermark"
            | "deleteWatermark"
            | "insertInkCanvas"
            | "replaceInkCanvas"
            | "deleteInkCanvas"
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
    deny_keys(
        obj,
        i,
        &[
            "op",
            "id",
            "text",
            "x",
            "y",
            "cx",
            "cy",
            "prst",
            "sz",
            "color",
            "bold",
            "italic",
            "underline",
            "font",
            "eastAsiaFont",
            "align",
            "textAnchor",
        ],
    )?;
    let id = require_id(obj, i)?;
    let host = parse_insert_shape_host(&id, i)?;
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
    let prst = match obj.get("prst") {
        None => "rect".to_string(),
        Some(Value::String(s)) => {
            if super::shape::is_writable_shape_prst(s) {
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
        host,
        text,
        x,
        y,
        cx,
        cy,
        prst,
        format: crate::drawing_text::parse(obj, &format!("ops[{i}]"), false)?,
    })
}

fn parse_set_shape_format(obj: &Map<String, Value>, i: usize) -> Result<SetShapeFormatOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op",
            "id",
            "sz",
            "color",
            "bold",
            "italic",
            "underline",
            "font",
            "eastAsiaFont",
            "align",
            "textAnchor",
        ],
    )?;
    let id = require_id(obj, i)?;
    let format = crate::drawing_text::parse(obj, &format!("ops[{i}]"), true)?
        .expect("require_one 已保证有补丁");
    Ok(SetShapeFormatOp { id, format })
}

fn parse_insert_connector(obj: &Map<String, Value>, i: usize) -> Result<InsertConnectorOp, Error> {
    deny_keys(obj, i, &["op", "id", "prst", "x", "y", "cx", "cy"])?;
    let id = require_id(obj, i)?;
    let (slide, on_notes, master) = connector::parse_insert_slide_id(&id, i)?.parts();
    let prst = match obj.get("prst") {
        None => "straightConnector1".to_string(),
        Some(Value::String(s)) => {
            if connector::is_writable_prst(s) {
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
    let x = parse_off_field(obj.get("x"), i, "x")?;
    let y = parse_off_field(obj.get("y"), i, "y")?;
    if x.is_some() != y.is_some() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "insertConnector".into(),
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
                op: "insertConnector".into(),
                field: "cx and cy together".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(InsertConnectorOp {
        slide,
        on_notes,
        master,
        prst,
        x,
        y,
        cx,
        cy,
    })
}

fn parse_delete_connector(obj: &Map<String, Value>, i: usize) -> Result<DeleteConnectorOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let target = connector::parse_cxn_id(&id, i)?;
    Ok(DeleteConnectorOp { id, target })
}

fn parse_group_shapes(obj: &Map<String, Value>, i: usize) -> Result<GroupShapesOp, Error> {
    deny_keys(obj, i, &["op", "id", "ids"])?;
    let id = require_id(obj, i)?;
    let slide = connector::parse_slide_only(&id, i, crate::error::IdReason::SlideOpNeedsSlide)?;
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
        let m = group::parse_member_id(mid, i).map_err(|e| e.at(format!("ops[{i}].ids[{k}]")))?;
        members.push(m);
    }
    Ok(GroupShapesOp {
        slide,
        ids,
        members,
    })
}

fn parse_ungroup(obj: &Map<String, Value>, i: usize) -> Result<UngroupOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let member = group::parse_member_id(&id, i)?;
    Ok(UngroupOp { id, member })
}

fn parse_delete_shape(obj: &Map<String, Value>, i: usize) -> Result<DeleteShapeOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let (target, placeholder) = parse_slide_or_master_shape(&id, i)?;
    Ok(DeleteShapeOp {
        id,
        target,
        placeholder,
    })
}

fn parse_set_shape_pos(obj: &Map<String, Value>, i: usize) -> Result<SetShapePosOp, Error> {
    deny_keys(obj, i, &["op", "id", "x", "y"])?;
    let id = require_id(obj, i)?;
    let (target, placeholder, cxn) = parse_shape_pos_host(&id, i)?;
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
        cxn,
        x,
        y,
    })
}

fn parse_set_shape_size(obj: &Map<String, Value>, i: usize) -> Result<SetShapeSizeOp, Error> {
    deny_keys(obj, i, &["op", "id", "cx", "cy"])?;
    let id = require_id(obj, i)?;
    let (target, placeholder, cxn) = parse_shape_pos_host(&id, i)?;
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
        cxn,
        cx,
        cy,
    })
}

fn parse_set_shape_fill(obj: &Map<String, Value>, i: usize) -> Result<SetShapeFillOp, Error> {
    deny_keys(obj, i, &["op", "id", "color"])?;
    let id = require_id(obj, i)?;
    let (target, placeholder) = parse_slide_or_master_shape(&id, i)?;
    let color = parse_color_field(obj, i, "setShapeFill")?;
    Ok(SetShapeFillOp {
        id,
        target,
        placeholder,
        color,
    })
}

fn parse_set_shape_line(obj: &Map<String, Value>, i: usize) -> Result<SetShapeLineOp, Error> {
    deny_keys(obj, i, &["op", "id", "color", "width"])?;
    let id = require_id(obj, i)?;
    let (target, placeholder, cxn) = if id.starts_with("master:")
        || id.starts_with("notesMaster/")
        || id.starts_with("handout/")
    {
        (
            None,
            Some(super::master::parse_placeholder_id(&id, i)?),
            None,
        )
    } else if connector::looks_like_cxn_id(&id) {
        (None, None, Some(connector::parse_cxn_id(&id, i)?))
    } else {
        (Some(super::shape::parse_shape_id(&id, i)?), None, None)
    };
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
        placeholder,
        cxn,
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
    deny_keys(
        obj,
        i,
        &[
            "op", "id", "type", "dur", "advClick", "advTm", "dir", "invX", "invY", "sound",
            "orient", "spokes",
        ],
    )?;
    let id = require_id(obj, i)?;
    let host = picture::parse_insert_host(&id, i, IdReason::SetTransitionNeedsHost)?;
    if host.on_notes() {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SetTransitionNeedsHost,
            },
            format!("ops[{i}].id"),
        ));
    }
    let (slide, _, master) = host.parts();
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
    let adv_click = match obj.get("advClick") {
        None => true,
        Some(Value::Bool(b)) => *b,
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "advClick".into(),
                    expected: TypeName::Boolean,
                },
                format!("ops[{i}].advClick"),
            ));
        }
    };
    let adv_tm = match obj.get("advTm") {
        None | Some(Value::Null) => None,
        Some(_) if type_name.is_none() => None,
        Some(v) => Some(parse_transition_adv_tm(v, i)?),
    };
    let dir = match obj.get("dir") {
        None => None,
        Some(_) if type_name.is_none() => None,
        Some(Value::String(s)) if !s.is_empty() => Some(s.clone()),
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidTransitionDir,
                format!("ops[{i}].dir"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "dir".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].dir"),
            ));
        }
    };
    let inv_x = parse_optional_flag(obj.get("invX"), i, "invX")?;
    let inv_y = parse_optional_flag(obj.get("invY"), i, "invY")?;
    let sound = match obj.get("sound") {
        None => None,
        Some(_) if type_name.is_none() => None,
        Some(_) => parse_anim_sound(obj.get("sound"), i)?,
    };
    let orient = match obj.get("orient") {
        None => None,
        Some(_) if type_name.is_none() => None,
        Some(Value::String(s)) if !s.is_empty() => Some(s.clone()),
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidTransitionOrient,
                format!("ops[{i}].orient"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "orient".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].orient"),
            ));
        }
    };
    let spokes = match obj.get("spokes") {
        None => None,
        Some(_) if type_name.is_none() => None,
        Some(v) => Some(parse_transition_spokes(v, &format!("ops[{i}].spokes"))?),
    };
    Ok(SetTransitionOp {
        slide,
        master,
        type_name,
        dur,
        adv_click,
        adv_tm,
        dir,
        inv_x,
        inv_y,
        sound,
        orient,
        spokes,
    })
}

const ANIM_HOST_KINDS: &[&str] = &[
    "shape",
    "cxn",
    "pic",
    "video",
    "audio",
    "table",
    "chart",
    "smartdiagram",
];

fn parse_insert_anim(obj: &Map<String, Value>, i: usize) -> Result<InsertAnimOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op", "id", "type", "trigger", "delay", "dur", "repeat", "sound", "para", "clickOn",
        ],
    )?;
    let id = require_id(obj, i)?;
    let target = parse_anim_host(&id, i)?;
    let type_name = parse_anim_type(obj.get("type"), i, true)?.ok_or_else(|| {
        Error::with_path(
            ErrorKind::OpRequiresField {
                op: "insertAnim".into(),
                field: "type".into(),
            },
            format!("ops[{i}].type"),
        )
    })?;
    let trigger = parse_anim_trigger(obj.get("trigger"), i, true)?.ok_or_else(|| {
        Error::with_path(ErrorKind::InvalidAnimTrigger, format!("ops[{i}].trigger"))
    })?;
    let delay = match obj.get("delay") {
        None => 0,
        Some(v) => parse_transition_ms(v, i, "delay", ErrorKind::InvalidAnimDelay, 0, 60000)?,
    };
    let dur = match obj.get("dur") {
        None => 500,
        Some(v) => parse_transition_ms(v, i, "dur", ErrorKind::InvalidAnimDur, 1, 60000)?,
    };
    let repeat = parse_anim_repeat(obj.get("repeat"), i)?;
    let sound = parse_anim_sound(obj.get("sound"), i)?;
    let para = parse_anim_para(obj.get("para"), i)?;
    let click_on = match (trigger.as_str(), obj.get("clickOn")) {
        ("clickOn", None) => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "clickOn".into(),
                },
                format!("ops[{i}].clickOn"),
            ));
        }
        ("clickOn", Some(Value::String(s))) if !s.is_empty() => Some(s.clone()),
        ("clickOn", Some(_)) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "clickOn".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].clickOn"),
            ));
        }
        (_, Some(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidAnimTrigger,
                format!("ops[{i}].clickOn"),
            ));
        }
        _ => None,
    };
    Ok(InsertAnimOp {
        id,
        target,
        type_name,
        trigger,
        delay,
        dur,
        repeat,
        sound,
        para,
        click_on,
    })
}

fn parse_set_anim(obj: &Map<String, Value>, i: usize) -> Result<SetAnimOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op", "id", "type", "trigger", "delay", "dur", "repeat", "sound", "para", "clickOn",
        ],
    )?;
    let id = require_id(obj, i)?;
    let (target, index) = parse_anim_id(&id, i)?;
    let type_name = parse_anim_type(obj.get("type"), i, false)?;
    let trigger = parse_anim_trigger(obj.get("trigger"), i, false)?;
    let delay = match obj.get("delay") {
        None => None,
        Some(v) => Some(parse_transition_ms(
            v,
            i,
            "delay",
            ErrorKind::InvalidAnimDelay,
            0,
            60000,
        )?),
    };
    let dur = match obj.get("dur") {
        None => None,
        Some(v) => Some(parse_transition_ms(
            v,
            i,
            "dur",
            ErrorKind::InvalidAnimDur,
            1,
            60000,
        )?),
    };
    let repeat = if obj.contains_key("repeat") {
        Some(parse_anim_repeat(obj.get("repeat"), i)?)
    } else {
        None
    };
    let sound = match obj.get("sound") {
        None => None,
        Some(Value::Null) => Some(None),
        Some(v) => Some(Some(parse_anim_sound(Some(v), i)?.ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: "sound".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].sound"),
            )
        })?)),
    };
    let para = match obj.get("para") {
        None => None,
        Some(Value::Null) => Some(None),
        Some(v) => Some(Some(parse_anim_para(Some(v), i)?.ok_or_else(|| {
            Error::with_path(ErrorKind::InvalidAnimPara, format!("ops[{i}].para"))
        })?)),
    };
    let click_on = match obj.get("clickOn") {
        None => None,
        Some(Value::String(s)) if !s.is_empty() => Some(s.clone()),
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidAnimTrigger,
                format!("ops[{i}].clickOn"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "clickOn".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].clickOn"),
            ));
        }
    };
    if type_name.is_none()
        && trigger.is_none()
        && delay.is_none()
        && dur.is_none()
        && repeat.is_none()
        && sound.is_none()
        && para.is_none()
        && click_on.is_none()
    {
        return Err(Error::with_path(
            ErrorKind::NeedAnimSetField,
            format!("ops[{i}]"),
        ));
    }
    if trigger.as_deref() == Some("clickOn") && click_on.is_none() {
        return Err(Error::with_path(
            ErrorKind::MissingField {
                field: "clickOn".into(),
            },
            format!("ops[{i}].clickOn"),
        ));
    }
    Ok(SetAnimOp {
        id,
        target,
        index,
        type_name,
        trigger,
        delay,
        dur,
        repeat,
        sound,
        para,
        click_on,
    })
}

fn parse_move_anim(obj: &Map<String, Value>, i: usize) -> Result<MoveAnimOp, Error> {
    deny_keys(obj, i, &["op", "id", "index"])?;
    let id = require_id(obj, i)?;
    let (target, index) = parse_anim_id(&id, i)?;
    let dest = match obj.get("index") {
        None => {
            return Err(Error::with_path(
                ErrorKind::OpRequiresField {
                    op: "moveAnim".into(),
                    field: "index".into(),
                },
                format!("ops[{i}].index"),
            ));
        }
        Some(Value::Number(n)) => {
            let Some(v) = n.as_u64() else {
                return Err(Error::with_path(
                    ErrorKind::InvalidAnimMoveIndex,
                    format!("ops[{i}].index"),
                ));
            };
            if v < 1 {
                return Err(Error::with_path(
                    ErrorKind::InvalidAnimMoveIndex,
                    format!("ops[{i}].index"),
                ));
            }
            v as usize
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "index".into(),
                    expected: TypeName::Integer,
                },
                format!("ops[{i}].index"),
            ));
        }
    };
    Ok(MoveAnimOp {
        id,
        target,
        index,
        dest,
    })
}

fn parse_anim_type(v: Option<&Value>, i: usize, required: bool) -> Result<Option<String>, Error> {
    match v {
        None if required => Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "insertAnim".into(),
                field: "type".into(),
            },
            format!("ops[{i}].type"),
        )),
        None => Ok(None),
        Some(Value::String(s)) if timing::anim_preset_ok(s) => Ok(Some(s.clone())),
        Some(Value::String(_)) => Err(Error::with_path(
            ErrorKind::InvalidAnimType,
            format!("ops[{i}].type"),
        )),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "type".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].type"),
        )),
    }
}

fn parse_anim_repeat(v: Option<&Value>, i: usize) -> Result<timing::RepeatWrite, Error> {
    match v {
        None => Ok(timing::RepeatWrite::Once),
        Some(Value::Null) => Ok(timing::RepeatWrite::Indefinite),
        Some(Value::Number(n)) => {
            let Some(v) = n.as_u64() else {
                return Err(Error::with_path(
                    ErrorKind::InvalidAnimRepeat,
                    format!("ops[{i}].repeat"),
                ));
            };
            if !(1..=99).contains(&v) {
                return Err(Error::with_path(
                    ErrorKind::InvalidAnimRepeat,
                    format!("ops[{i}].repeat"),
                ));
            }
            if v == 1 {
                Ok(timing::RepeatWrite::Once)
            } else {
                Ok(timing::RepeatWrite::Times(v as u32))
            }
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::InvalidAnimRepeat,
            format!("ops[{i}].repeat"),
        )),
    }
}

fn parse_anim_sound(v: Option<&Value>, i: usize) -> Result<Option<String>, Error> {
    match v {
        None => Ok(None),
        Some(Value::String(s)) if !s.is_empty() => Ok(Some(s.clone())),
        Some(Value::String(_)) | Some(Value::Null) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "sound".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].sound"),
        )),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "sound".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].sound"),
        )),
    }
}

fn parse_anim_para(v: Option<&Value>, i: usize) -> Result<Option<u32>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Number(n)) => {
            let Some(v) = n.as_u64() else {
                return Err(Error::with_path(
                    ErrorKind::InvalidAnimPara,
                    format!("ops[{i}].para"),
                ));
            };
            if v < 1 {
                return Err(Error::with_path(
                    ErrorKind::InvalidAnimPara,
                    format!("ops[{i}].para"),
                ));
            }
            Ok(Some(v as u32))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::InvalidAnimPara,
            format!("ops[{i}].para"),
        )),
    }
}

fn parse_anim_trigger(
    v: Option<&Value>,
    i: usize,
    required_default_click: bool,
) -> Result<Option<String>, Error> {
    match v {
        None if required_default_click => Ok(Some("click".to_string())),
        None => Ok(None),
        Some(Value::String(s)) if matches!(s.as_str(), "click" | "with" | "after" | "clickOn") => {
            Ok(Some(s.clone()))
        }
        Some(Value::String(_)) => Err(Error::with_path(
            ErrorKind::InvalidAnimTrigger,
            format!("ops[{i}].trigger"),
        )),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "trigger".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].trigger"),
        )),
    }
}

fn parse_delete_anim(obj: &Map<String, Value>, i: usize) -> Result<DeleteAnimOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let (target, index) = parse_anim_id(&id, i)?;
    Ok(DeleteAnimOp { id, target, index })
}

fn parse_positive_index(
    raw: &str,
    path: &str,
    not_int: IdReason,
    from_one: IdReason,
) -> Result<usize, Error> {
    let n: usize = raw.parse().map_err(|_| {
        Error::with_path(ErrorKind::InvalidId { reason: not_int }, path.to_string())
    })?;
    if n < 1 {
        return Err(Error::with_path(
            ErrorKind::InvalidId { reason: from_one },
            path.to_string(),
        ));
    }
    Ok(n)
}

fn parse_kind_key(after: &str, path: &str) -> Result<(), Error> {
    let Some((kind, key)) = after.split_once(':') else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::AnimForm,
            },
            path.to_string(),
        ));
    };
    if !ANIM_HOST_KINDS.contains(&kind) || key.is_empty() || key.contains('/') {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::AnimForm,
            },
            path.to_string(),
        ));
    }
    Ok(())
}

fn parse_anim_host(id: &str, op_index: usize) -> Result<AnimTarget, Error> {
    let path = format!("ops[{op_index}].id");
    if let Some(rest) = id.strip_prefix("notesMaster/") {
        parse_kind_key(rest, &path)?;
        return Ok(AnimTarget::NotesMaster);
    }
    if let Some(rest) = id.strip_prefix("handout/") {
        parse_kind_key(rest, &path)?;
        return Ok(AnimTarget::Handout);
    }
    if let Some(rest) = id.strip_prefix("master:") {
        let Some((n, after)) = rest.split_once('/') else {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::InsertAnimNeedsHost,
                },
                path,
            ));
        };
        let master = parse_positive_index(
            n,
            &path,
            IdReason::SlideIndexNotInt,
            IdReason::SlideIndexFromOne,
        )?;
        if let Some(layout_rest) = after.strip_prefix("layout:") {
            let Some((k, shape)) = layout_rest.split_once('/') else {
                return Err(Error::with_path(
                    ErrorKind::InvalidId {
                        reason: IdReason::InsertAnimNeedsHost,
                    },
                    path,
                ));
            };
            let layout = parse_positive_index(
                k,
                &path,
                IdReason::SlideIndexNotInt,
                IdReason::SlideIndexFromOne,
            )?;
            let Some(key) = shape.strip_prefix("shape:") else {
                return Err(Error::with_path(
                    ErrorKind::InvalidId {
                        reason: IdReason::InsertAnimNeedsHost,
                    },
                    path,
                ));
            };
            if key.is_empty() || key.contains('/') {
                return Err(Error::with_path(
                    ErrorKind::InvalidId {
                        reason: IdReason::InsertAnimNeedsHost,
                    },
                    path,
                ));
            }
            return Ok(AnimTarget::Master {
                master,
                layout: Some(layout),
            });
        }
        if let Some(key) = after.strip_prefix("shape:") {
            if key.is_empty() || key.contains('/') {
                return Err(Error::with_path(
                    ErrorKind::InvalidId {
                        reason: IdReason::InsertAnimNeedsHost,
                    },
                    path,
                ));
            }
            return Ok(AnimTarget::Master {
                master,
                layout: None,
            });
        }
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::InsertAnimNeedsHost,
            },
            path,
        ));
    }
    let Some(rest) = id.strip_prefix("slide:") else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::AnimForm,
            },
            path,
        ));
    };
    let Some((n, after)) = rest.split_once('/') else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::InsertAnimNeedsHost,
            },
            path,
        ));
    };
    let slide = parse_positive_index(
        n,
        &path,
        IdReason::SlideIndexNotInt,
        IdReason::SlideIndexFromOne,
    )?;
    if let Some(notes_after) = after.strip_prefix("notes/") {
        parse_kind_key(notes_after, &path)?;
        return Ok(AnimTarget::Notes { slide });
    }
    parse_kind_key(after, &path)?;
    Ok(AnimTarget::Slide { slide })
}

fn parse_anim_id(id: &str, op_index: usize) -> Result<(AnimTarget, usize), Error> {
    let path = format!("ops[{op_index}].id");
    if let Some(rest) = id.strip_prefix("notesMaster/anim:") {
        if rest.is_empty() || rest.contains('/') {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::AnimForm,
                },
                path,
            ));
        }
        let index = parse_positive_index(
            rest,
            &path,
            IdReason::AnimIndexNotInt,
            IdReason::AnimIndexFromOne,
        )?;
        return Ok((AnimTarget::NotesMaster, index));
    }
    if let Some(rest) = id.strip_prefix("handout/anim:") {
        if rest.is_empty() || rest.contains('/') {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::AnimForm,
                },
                path,
            ));
        }
        let index = parse_positive_index(
            rest,
            &path,
            IdReason::AnimIndexNotInt,
            IdReason::AnimIndexFromOne,
        )?;
        return Ok((AnimTarget::Handout, index));
    }
    if let Some(rest) = id.strip_prefix("master:") {
        if let Some((n, after)) = rest.split_once("/layout:") {
            let Some((k, anim)) = after.split_once("/anim:") else {
                return Err(Error::with_path(
                    ErrorKind::InvalidId {
                        reason: IdReason::AnimForm,
                    },
                    path,
                ));
            };
            if anim.is_empty() || anim.contains('/') {
                return Err(Error::with_path(
                    ErrorKind::InvalidId {
                        reason: IdReason::AnimForm,
                    },
                    path,
                ));
            }
            let master = parse_positive_index(
                n,
                &path,
                IdReason::SlideIndexNotInt,
                IdReason::SlideIndexFromOne,
            )?;
            let layout = parse_positive_index(
                k,
                &path,
                IdReason::SlideIndexNotInt,
                IdReason::SlideIndexFromOne,
            )?;
            let index = parse_positive_index(
                anim,
                &path,
                IdReason::AnimIndexNotInt,
                IdReason::AnimIndexFromOne,
            )?;
            return Ok((
                AnimTarget::Master {
                    master,
                    layout: Some(layout),
                },
                index,
            ));
        }
        let Some((n, after)) = rest.split_once("/anim:") else {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::AnimForm,
                },
                path,
            ));
        };
        if after.is_empty() || after.contains('/') {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::AnimForm,
                },
                path,
            ));
        }
        let master = parse_positive_index(
            n,
            &path,
            IdReason::SlideIndexNotInt,
            IdReason::SlideIndexFromOne,
        )?;
        let index = parse_positive_index(
            after,
            &path,
            IdReason::AnimIndexNotInt,
            IdReason::AnimIndexFromOne,
        )?;
        return Ok((
            AnimTarget::Master {
                master,
                layout: None,
            },
            index,
        ));
    }
    let Some(rest) = id.strip_prefix("slide:") else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::AnimForm,
            },
            path,
        ));
    };
    if let Some((n, after)) = rest.split_once("/notes/anim:") {
        if after.is_empty() || after.contains('/') {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::AnimForm,
                },
                path,
            ));
        }
        let slide = parse_positive_index(
            n,
            &path,
            IdReason::SlideIndexNotInt,
            IdReason::SlideIndexFromOne,
        )?;
        let index = parse_positive_index(
            after,
            &path,
            IdReason::AnimIndexNotInt,
            IdReason::AnimIndexFromOne,
        )?;
        return Ok((AnimTarget::Notes { slide }, index));
    }
    let Some((n, after)) = rest.split_once("/anim:") else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::AnimForm,
            },
            path,
        ));
    };
    if after.is_empty() || after.contains('/') {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::AnimForm,
            },
            path,
        ));
    }
    let slide = parse_positive_index(
        n,
        &path,
        IdReason::SlideIndexNotInt,
        IdReason::SlideIndexFromOne,
    )?;
    let index = parse_positive_index(
        after,
        &path,
        IdReason::AnimIndexNotInt,
        IdReason::AnimIndexFromOne,
    )?;
    Ok((AnimTarget::Slide { slide }, index))
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

fn parse_insert_layout(obj: &Map<String, Value>, i: usize) -> Result<InsertLayoutOp, Error> {
    deny_keys(obj, i, &["op", "id", "name"])?;
    let id = require_id(obj, i)?;
    let master = super::master::parse_master_only_id(&id, i, IdReason::InsertLayoutNeedsMaster)?;
    let name = match obj.get("name") {
        None => "空白".to_string(),
        Some(Value::String(s)) if !s.is_empty() => s.clone(),
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "name".into(),
                    expected: TypeName::NonEmptyString,
                },
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
    Ok(InsertLayoutOp { master, name })
}

fn parse_delete_layout(obj: &Map<String, Value>, i: usize) -> Result<DeleteLayoutOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let (master, layout) = super::master::parse_layout_ref_at(&id, &format!("ops[{i}].id"))?;
    Ok(DeleteLayoutOp { master, layout })
}

fn parse_insert_placeholder(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<InsertPlaceholderOp, Error> {
    deny_keys(obj, i, &["op", "id", "type", "text", "x", "y", "cx", "cy"])?;
    let id = require_id(obj, i)?;
    let host = super::master::parse_insert_placeholder_host(&id, i)?;
    let type_name = match obj.get("type") {
        None => {
            return Err(Error::with_path(
                ErrorKind::OpRequiresField {
                    op: "insertPlaceholder".into(),
                    field: "type".into(),
                },
                format!("ops[{i}].type"),
            ));
        }
        Some(Value::String(s)) if super::master::is_writable_placeholder_type(s) => s.clone(),
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidPlaceholderType,
                format!("ops[{i}].type"),
            ));
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
                op: "insertPlaceholder".into(),
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
                op: "insertPlaceholder".into(),
                field: "cx and cy together".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(InsertPlaceholderOp {
        host,
        type_name,
        text,
        x,
        y,
        cx,
        cy,
    })
}

fn parse_delete_placeholder(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<DeletePlaceholderOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let placeholder = super::master::parse_placeholder_id(&id, i)?;
    Ok(DeletePlaceholderOp { id, placeholder })
}

fn parse_insert_master(obj: &Map<String, Value>, i: usize) -> Result<InsertMasterOp, Error> {
    deny_keys(obj, i, &["op", "id", "from"])?;
    let id = require_id(obj, i)?;
    if id != "pptx:presentation" {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::InsertMasterNeedsPresentation,
            },
            format!("ops[{i}].id"),
        ));
    }
    let from = match obj.get("from") {
        None => 1usize,
        Some(Value::String(s)) => {
            super::master::parse_master_only_at(s, &format!("ops[{i}].from"), IdReason::MasterForm)?
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "from".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].from"),
            ));
        }
    };
    Ok(InsertMasterOp { from })
}

fn parse_delete_master(obj: &Map<String, Value>, i: usize) -> Result<DeleteMasterOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    Ok(DeleteMasterOp {
        master: super::master::parse_master_only_id(&id, i, IdReason::DeleteMasterNeedsMaster)?,
    })
}

fn parse_set_slide_size(obj: &Map<String, Value>, i: usize) -> Result<SetSlideSizeOp, Error> {
    deny_keys(obj, i, &["op", "id", "cx", "cy", "type"])?;
    let id = require_id(obj, i)?;
    if id != "pptx:presentation" {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SetSlideSizeNeedsPresentation,
            },
            format!("ops[{i}].id"),
        ));
    }
    let cx = parse_emu_field(obj.get("cx"), i, "cx")?.ok_or_else(|| {
        Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setSlideSize".into(),
                field: "cx".into(),
            },
            format!("ops[{i}].cx"),
        )
    })?;
    let cy = parse_emu_field(obj.get("cy"), i, "cy")?.ok_or_else(|| {
        Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setSlideSize".into(),
                field: "cy".into(),
            },
            format!("ops[{i}].cy"),
        )
    })?;
    let type_name = match obj.get("type") {
        None => None,
        Some(Value::String(s)) if super::theme::is_slide_size_type(s) => Some(s.clone()),
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidSlideSizeType,
                format!("ops[{i}].type"),
            ));
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
    Ok(SetSlideSizeOp { cx, cy, type_name })
}

fn parse_hex_slot(v: &Value, i: usize, field: &str) -> Result<String, Error> {
    match v {
        Value::String(s) if s.len() == 6 && s.chars().all(|c| c.is_ascii_hexdigit()) => {
            Ok(s.clone())
        }
        Value::String(_) => Err(Error::with_path(
            ErrorKind::InvalidHexColor,
            format!("ops[{i}].{field}"),
        )),
        _ => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].{field}"),
        )),
    }
}

fn parse_set_theme_colors(obj: &Map<String, Value>, i: usize) -> Result<SetThemeColorsOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op", "id", "dk1", "lt1", "dk2", "lt2", "accent1", "accent2", "accent3", "accent4",
            "accent5", "accent6", "hlink", "folHlink",
        ],
    )?;
    let id = require_id(obj, i)?;
    let master = super::master::parse_master_only_id(&id, i, IdReason::SetThemeNeedsMaster)?;
    let mut slots = Vec::new();
    for slot in super::theme::COLOR_SLOTS {
        if let Some(v) = obj.get(*slot) {
            slots.push(((*slot).to_string(), parse_hex_slot(v, i, slot)?));
        }
    }
    if slots.is_empty() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setThemeColors".into(),
                field: "a color slot".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(SetThemeColorsOp { master, slots })
}

fn parse_theme_face(
    obj: &Map<String, Value>,
    i: usize,
    field: &str,
    allow_empty: bool,
) -> Result<Option<String>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::String(s)) if !s.is_empty() || allow_empty => Ok(Some(s.clone())),
        Some(Value::String(_)) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::NonEmptyString,
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

fn parse_set_theme_fonts(obj: &Map<String, Value>, i: usize) -> Result<SetThemeFontsOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op",
            "id",
            "majorLatin",
            "minorLatin",
            "majorEa",
            "minorEa",
            "majorCs",
            "minorCs",
        ],
    )?;
    let id = require_id(obj, i)?;
    let master = super::master::parse_master_only_id(&id, i, IdReason::SetThemeNeedsMaster)?;
    let mut fonts = Vec::new();
    if let Some(s) = parse_theme_face(obj, i, "majorLatin", false)? {
        fonts.push((super::theme::FontSlot::MajorLatin, s));
    }
    if let Some(s) = parse_theme_face(obj, i, "minorLatin", false)? {
        fonts.push((super::theme::FontSlot::MinorLatin, s));
    }
    if let Some(s) = parse_theme_face(obj, i, "majorEa", true)? {
        fonts.push((super::theme::FontSlot::MajorEa, s));
    }
    if let Some(s) = parse_theme_face(obj, i, "minorEa", true)? {
        fonts.push((super::theme::FontSlot::MinorEa, s));
    }
    if let Some(s) = parse_theme_face(obj, i, "majorCs", true)? {
        fonts.push((super::theme::FontSlot::MajorCs, s));
    }
    if let Some(s) = parse_theme_face(obj, i, "minorCs", true)? {
        fonts.push((super::theme::FontSlot::MinorCs, s));
    }
    if fonts.is_empty() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setThemeFonts".into(),
                field: "a font slot".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(SetThemeFontsOp { master, fonts })
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

fn parse_insert_hyperlink(obj: &Map<String, Value>, i: usize) -> Result<InsertHyperlinkOp, Error> {
    let on = parse_pptx_link_on(obj, i)?;
    let event = parse_pptx_link_event(obj, i)?;
    let run_like = on.is_some_and(LinkOn::is_run_like);
    let mut allowed = vec!["op", "id", "url", "file", "slide", "tooltip", "on", "event"];
    if run_like || on.is_none() {
        allowed.push("text");
    }
    deny_keys(obj, i, &allowed)?;
    let id = require_id(obj, i)?;
    let host = parse_insert_hyperlink_host(&id, i)?;
    let on = match on {
        Some(v) => v,
        None => host.default_on(),
    };
    if !host.on_ok(on) {
        return Err(Error::with_path(
            ErrorKind::InvalidHyperlinkOn,
            format!("ops[{i}].on"),
        ));
    }
    if obj.get("text").is_some() && !on.is_run_like() {
        return Err(Error::with_path(
            ErrorKind::UnknownField {
                field: "text".into(),
            },
            format!("ops[{i}].text"),
        ));
    }
    let target = parse_pptx_link_target(obj, i)?
        .ok_or_else(|| Error::with_path(ErrorKind::PptxHyperlinkNeedTarget, format!("ops[{i}]")))?;
    Ok(InsertHyperlinkOp {
        id,
        host,
        target,
        tooltip: parse_optional_tooltip(obj, i)?,
        on,
        event,
        text: if on.is_run_like() {
            parse_optional_run_text(obj, i)?
        } else {
            None
        },
    })
}

fn parse_pptx_link_on(obj: &Map<String, Value>, i: usize) -> Result<Option<LinkOn>, Error> {
    match obj.get("on") {
        None => Ok(None),
        Some(Value::String(s)) if s == "shape" => Ok(Some(LinkOn::Shape)),
        Some(Value::String(s)) if s == "run" => Ok(Some(LinkOn::Run)),
        Some(Value::String(s)) if s == "pic" => Ok(Some(LinkOn::Pic)),
        Some(Value::String(s)) if s == "table" => Ok(Some(LinkOn::Table)),
        Some(Value::String(s)) if s == "chart" => Ok(Some(LinkOn::Chart)),
        Some(Value::String(s)) if s == "smartArt" => Ok(Some(LinkOn::SmartArt)),
        Some(Value::String(s)) if s == "cell" => Ok(Some(LinkOn::Cell)),
        Some(Value::String(_)) => Err(Error::with_path(
            ErrorKind::InvalidHyperlinkOn,
            format!("ops[{i}].on"),
        )),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "on".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].on"),
        )),
    }
}

fn parse_pptx_link_event(obj: &Map<String, Value>, i: usize) -> Result<LinkEvent, Error> {
    match obj.get("event") {
        None => Ok(LinkEvent::Click),
        Some(Value::String(s)) if s == "click" => Ok(LinkEvent::Click),
        Some(Value::String(s)) if s == "hover" => Ok(LinkEvent::Hover),
        Some(Value::String(_)) => Err(Error::with_path(
            ErrorKind::InvalidHyperlinkEvent,
            format!("ops[{i}].event"),
        )),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "event".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].event"),
        )),
    }
}

fn parse_optional_run_text(obj: &Map<String, Value>, i: usize) -> Result<Option<String>, Error> {
    match obj.get("text") {
        None => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "text".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].text"),
        )),
    }
}

fn parse_set_hyperlink(obj: &Map<String, Value>, i: usize) -> Result<SetHyperlinkOp, Error> {
    deny_keys(obj, i, &["op", "id", "url", "file", "slide", "tooltip"])?;
    let id = require_id(obj, i)?;
    let (host, index) = hyperlink::parse_link_id(&id, i)?;
    let (slide, on_notes, master) = host.parts();
    let target = parse_pptx_link_target(obj, i)?;
    let tooltip = parse_set_tooltip(obj, i)?;
    if target.is_none() && tooltip.is_none() {
        return Err(Error::with_path(
            ErrorKind::PptxNeedSetHyperlinkField,
            format!("ops[{i}]"),
        ));
    }
    Ok(SetHyperlinkOp {
        id,
        slide,
        on_notes,
        master,
        index,
        target,
        tooltip,
    })
}

fn parse_delete_hyperlink(obj: &Map<String, Value>, i: usize) -> Result<DeleteHyperlinkOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let (host, index) = hyperlink::parse_link_id(&id, i)?;
    let (slide, on_notes, master) = host.parts();
    Ok(DeleteHyperlinkOp {
        id,
        slide,
        on_notes,
        master,
        index,
    })
}

fn parse_insert_field(obj: &Map<String, Value>, i: usize) -> Result<field::InsertFieldOp, Error> {
    deny_keys(obj, i, &["op", "id", "type", "text"])?;
    let id = require_id(obj, i)?;
    let shape = parse_insert_field_shape(&id, i)?;
    let type_name = require_fld_type(obj, i)?;
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
    Ok(field::InsertFieldOp {
        id,
        shape,
        type_name,
        text,
    })
}

fn parse_set_field(obj: &Map<String, Value>, i: usize) -> Result<field::SetFieldOp, Error> {
    deny_keys(obj, i, &["op", "id", "type", "text"])?;
    let id = require_id(obj, i)?;
    let (host, key) = field::parse_field_id(&id, i)?;
    let (slide, on_notes, master) = host.parts();
    let type_name = match obj.get("type") {
        None => None,
        Some(Value::String(s)) => {
            if !field::is_reserved_type(s) {
                return Err(Error::with_path(
                    ErrorKind::InvalidFldType,
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
    let text = match obj.get("text") {
        None => None,
        Some(Value::String(s)) => Some(s.clone()),
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
    if type_name.is_none() && text.is_none() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setField".into(),
                field: "type".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(field::SetFieldOp {
        id,
        slide,
        on_notes,
        master,
        key,
        type_name,
        text,
    })
}

fn parse_delete_field(obj: &Map<String, Value>, i: usize) -> Result<field::DeleteFieldOp, Error> {
    deny_keys(obj, i, &["op", "id", "keep"])?;
    let id = require_id(obj, i)?;
    let (host, key) = field::parse_field_id(&id, i)?;
    let (slide, on_notes, master) = host.parts();
    let keep = match obj.get("keep") {
        None => field::FieldKeep::Result,
        Some(Value::String(s)) if s == "result" => field::FieldKeep::Result,
        Some(Value::String(s)) if s == "instr" => field::FieldKeep::Instr,
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
    Ok(field::DeleteFieldOp {
        id,
        slide,
        on_notes,
        master,
        key,
        keep,
    })
}

fn parse_insert_hyperlink_host(id: &str, i: usize) -> Result<LinkInsertHost, Error> {
    let path = format!("ops[{i}].id");
    if id.starts_with("notesMaster") || id.starts_with("handout") {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::InsertHyperlinkNeedsShape,
            },
            path,
        ));
    }
    if id.contains("/r:") {
        let cell = table::parse_cell_id(id, &path).map_err(|_| {
            Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::InsertHyperlinkNeedsShape,
                },
                path.clone(),
            )
        })?;
        return Ok(LinkInsertHost::Cell(cell));
    }
    if id.contains("/pic:") {
        let pic = picture::parse_pic_id(id, &path).map_err(|_| {
            Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::InsertHyperlinkNeedsShape,
                },
                path.clone(),
            )
        })?;
        if pic.comment.is_some() {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::InsertHyperlinkNeedsShape,
                },
                path,
            ));
        }
        return Ok(LinkInsertHost::Pic(pic));
    }
    if id.contains("/table:") {
        let table = table::parse_table_id(id, &path, IdReason::InsertHyperlinkNeedsShape)?;
        return Ok(LinkInsertHost::Table(table));
    }
    if id.contains("/chart:") {
        let chart = chart::parse_chart_id(id, &path).map_err(|_| {
            Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::InsertHyperlinkNeedsShape,
                },
                path,
            )
        })?;
        return Ok(LinkInsertHost::Chart(chart));
    }
    if id.contains("/smartdiagram:") {
        let art = smartart::parse_diagram_id(id, &path).map_err(|_| {
            Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::InsertHyperlinkNeedsShape,
                },
                path,
            )
        })?;
        return Ok(LinkInsertHost::SmartArt(art));
    }
    Ok(LinkInsertHost::Shape(parse_insert_hyperlink_shape(id, i)?))
}

fn parse_insert_field_shape(id: &str, i: usize) -> Result<shape::ShapeTarget, Error> {
    parse_link_or_field_shape(id, i, IdReason::InsertFieldNeedsShape)
}

fn parse_insert_hyperlink_shape(id: &str, i: usize) -> Result<shape::ShapeTarget, Error> {
    parse_link_or_field_shape(id, i, IdReason::InsertHyperlinkNeedsShape)
}

fn parse_link_or_field_shape(
    id: &str,
    i: usize,
    bad: IdReason,
) -> Result<shape::ShapeTarget, Error> {
    if id.starts_with("notesMaster") || id.starts_with("handout") {
        return Err(Error::with_path(
            ErrorKind::InvalidId { reason: bad },
            format!("ops[{i}].id"),
        ));
    }
    if id.starts_with("master:") {
        let ph = super::master::parse_placeholder_id(id, i).map_err(|_| {
            Error::with_path(
                ErrorKind::InvalidId {
                    reason: bad.clone(),
                },
                format!("ops[{i}].id"),
            )
        })?;
        let super::master::PlaceholderHost::Master { master, layout } = ph.host else {
            return Err(Error::with_path(
                ErrorKind::InvalidId { reason: bad },
                format!("ops[{i}].id"),
            ));
        };
        return Ok(shape::ShapeTarget {
            slide: 0,
            on_notes: false,
            master: Some((master, layout)),
            key: ph.key,
        });
    }
    shape::parse_shape_id(id, i)
        .map_err(|_| Error::with_path(ErrorKind::InvalidId { reason: bad }, format!("ops[{i}].id")))
}

fn require_fld_type(obj: &Map<String, Value>, i: usize) -> Result<String, Error> {
    match obj.get("type") {
        Some(Value::String(s)) if field::is_reserved_type(s) => Ok(s.clone()),
        Some(Value::String(_)) => Err(Error::with_path(
            ErrorKind::InvalidFldType,
            format!("ops[{i}].type"),
        )),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "type".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].type"),
        )),
        None => Err(Error::with_path(
            ErrorKind::MissingField {
                field: "type".into(),
            },
            format!("ops[{i}].type"),
        )),
    }
}

fn parse_pptx_link_target(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<Option<PptxLinkKind>, Error> {
    let url = parse_pptx_target_string(obj, i, "url", ErrorKind::EmptyHyperlinkUrl)?;
    let file = parse_pptx_target_string(obj, i, "file", ErrorKind::EmptyHyperlinkFile)?;
    let slide = match obj.get("slide") {
        None => None,
        Some(Value::Null) => {
            return Err(Error::with_path(
                ErrorKind::PptxHyperlinkNeedTarget,
                format!("ops[{i}].slide"),
            ));
        }
        Some(Value::String(s)) if s.is_empty() => {
            return Err(Error::with_path(
                ErrorKind::PptxHyperlinkNeedTarget,
                format!("ops[{i}].slide"),
            ));
        }
        Some(Value::String(s)) => Some(hyperlink::parse_jump_slide(s, i, "slide")?),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "slide".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].slide"),
            ));
        }
    };
    match (url, file, slide) {
        (Some(u), None, None) => Ok(Some(PptxLinkKind::Url(u))),
        (None, Some(f), None) => Ok(Some(PptxLinkKind::File(f))),
        (None, None, Some(n)) => Ok(Some(PptxLinkKind::Slide(n))),
        (None, None, None) => Ok(None),
        _ => Err(Error::with_path(
            ErrorKind::PptxHyperlinkNeedTarget,
            format!("ops[{i}]"),
        )),
    }
}

fn parse_pptx_target_string(
    obj: &Map<String, Value>,
    i: usize,
    field: &str,
    empty: ErrorKind,
) -> Result<Option<String>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::Null) => Err(Error::with_path(empty, format!("ops[{i}].{field}"))),
        Some(Value::String(s)) if s.is_empty() => {
            Err(Error::with_path(empty, format!("ops[{i}].{field}")))
        }
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

fn parse_optional_tooltip(obj: &Map<String, Value>, i: usize) -> Result<Option<String>, Error> {
    match obj.get("tooltip") {
        None => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "tooltip".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].tooltip"),
        )),
    }
}

fn parse_set_tooltip(obj: &Map<String, Value>, i: usize) -> Result<Option<Option<String>>, Error> {
    match obj.get("tooltip") {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(None)),
        Some(Value::String(s)) => Ok(Some(Some(s.clone()))),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "tooltip".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].tooltip"),
        )),
    }
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

type ShapePosHost = (
    Option<super::shape::ShapeTarget>,
    Option<super::master::PlaceholderId>,
    Option<connector::CxnTarget>,
);

fn parse_shape_pos_host(id: &str, i: usize) -> Result<ShapePosHost, Error> {
    if id.starts_with("master:") || id.starts_with("notesMaster/") || id.starts_with("handout/") {
        Ok((
            None,
            Some(super::master::parse_placeholder_id(id, i)?),
            None,
        ))
    } else if connector::looks_like_cxn_id(id) {
        Ok((None, None, Some(connector::parse_cxn_id(id, i)?)))
    } else {
        Ok((Some(super::shape::parse_shape_id(id, i)?), None, None))
    }
}

fn parse_slide_or_master_shape(
    id: &str,
    i: usize,
) -> Result<
    (
        Option<super::shape::ShapeTarget>,
        Option<super::master::PlaceholderId>,
    ),
    Error,
> {
    if id.starts_with("master:") || id.starts_with("notesMaster/") || id.starts_with("handout/") {
        Ok((None, Some(super::master::parse_placeholder_id(id, i)?)))
    } else {
        Ok((Some(super::shape::parse_shape_id(id, i)?), None))
    }
}

fn parse_insert_shape_host(id: &str, i: usize) -> Result<InsertShapeHost, Error> {
    if id.starts_with("master:") {
        if id.contains("/layout:") {
            let (master, layout) = super::master::parse_layout_ref(id, i)?;
            return Ok(InsertShapeHost::Master {
                master,
                layout: Some(layout),
            });
        }
        let master =
            super::master::parse_master_only_id(id, i, crate::error::IdReason::InsertShapeHost)?;
        return Ok(InsertShapeHost::Master {
            master,
            layout: None,
        });
    }
    let (slide, on_notes) =
        picture::parse_insert_slide_or_notes(id, i, crate::error::IdReason::InsertShapeHost)?;
    Ok(InsertShapeHost::Slide { slide, on_notes })
}

fn parse_transition_dur(v: &Value, i: usize) -> Result<u32, Error> {
    parse_transition_ms(v, i, "dur", ErrorKind::InvalidTransitionDur, 1, 60000)
}

fn parse_transition_adv_tm(v: &Value, i: usize) -> Result<u32, Error> {
    parse_transition_ms(
        v,
        i,
        "advTm",
        ErrorKind::InvalidTransitionAdvTm,
        1,
        86400000,
    )
}

fn parse_transition_spokes(v: &Value, path: &str) -> Result<u32, Error> {
    let n = match v {
        Value::Number(num) => num.as_u64().ok_or_else(|| {
            Error::with_path(ErrorKind::InvalidTransitionSpokes, path.to_string())
        })?,
        _ => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "spokes".into(),
                    expected: TypeName::Integer,
                },
                path.to_string(),
            ));
        }
    };
    if !(1..=8).contains(&n) {
        return Err(Error::with_path(
            ErrorKind::InvalidTransitionSpokes,
            path.to_string(),
        ));
    }
    Ok(n as u32)
}

fn parse_transition_ms(
    v: &Value,
    i: usize,
    field: &str,
    invalid: ErrorKind,
    min: u64,
    max: u64,
) -> Result<u32, Error> {
    let path = format!("ops[{i}].{field}");
    let n = match v {
        Value::Number(num) => num.as_u64().ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: field.into(),
                    expected: TypeName::Integer,
                },
                path.clone(),
            )
        })?,
        _ => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: field.into(),
                    expected: TypeName::Integer,
                },
                path,
            ));
        }
    };
    if !(min..=max).contains(&n) {
        return Err(Error::with_path(invalid, path));
    }
    Ok(n as u32)
}

fn parse_optional_flag(v: Option<&Value>, i: usize, field: &str) -> Result<bool, Error> {
    match v {
        None => Ok(false),
        Some(Value::Bool(b)) => Ok(*b),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Boolean,
            },
            format!("ops[{i}].{field}"),
        )),
    }
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
    let id = require_id(obj, i)?;
    if id.contains("/comment:") {
        deny_keys(obj, i, &["op", "id", "file"])?;
        let target = comments::parse_comment_id(&id, &format!("ops[{i}].id"))?;
        let file = require_file(obj, i, "insertPicture")?;
        return Ok(InsertPictureOp {
            slide: target.slide,
            on_notes: false,
            master: None,
            comment: Some(target),
            file,
            x: None,
            y: None,
            cx: None,
            cy: None,
        });
    }
    deny_keys(obj, i, &["op", "id", "file", "x", "y", "cx", "cy"])?;
    let host = picture::parse_insert_host(&id, i, crate::error::IdReason::InsertNeedsSlide)?;
    let (slide, on_notes, master) = host.parts();
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
        master,
        comment: None,
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
    picture::reject_comment_pic(&target, &format!("ops[{i}].id"))?;
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
    picture::reject_comment_pic(&target, &format!("ops[{i}].id"))?;
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

fn parse_insert_media(
    obj: &Map<String, Value>,
    i: usize,
    op: &str,
) -> Result<InsertMediaOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op",
            "id",
            "file",
            "x",
            "y",
            "cx",
            "cy",
            "st",
            "end",
            "bookmarks",
            "tracks",
            "trackDisplayLoc",
        ],
    )?;
    let id = require_id(obj, i)?;
    let (slide, on_notes, master) = media::parse_insert_slide(&id, i)?.parts();
    let file = require_file(obj, i, op)?;
    let x = parse_off_field(obj.get("x"), i, "x")?;
    let y = parse_off_field(obj.get("y"), i, "y")?;
    if x.is_some() != y.is_some() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: op.into(),
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
                op: op.into(),
                field: "cx and cy together".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    let prefix = format!("ops[{i}]");
    let extras = parse_media_clip_fields(obj, &prefix, false)?;
    Ok(InsertMediaOp {
        slide,
        on_notes,
        master,
        file,
        x,
        y,
        cx,
        cy,
        st: extras.st,
        end: extras.end,
        bookmarks: extras.bookmarks,
        tracks: extras.tracks,
        track_display_loc: extras.track_display_loc,
    })
}

struct MediaClipFields {
    st: Option<f64>,
    end: Option<f64>,
    bookmarks: Option<Vec<MediaBookmark>>,
    tracks: Option<Vec<MediaTrackWrite>>,
    track_display_loc: Option<String>,
}

fn parse_media_clip_fields(
    obj: &Map<String, Value>,
    prefix: &str,
    allow_null_lists: bool,
) -> Result<MediaClipFields, Error> {
    let st = parse_optional_media_time(obj.get("st"), &format!("{prefix}.st"))?;
    let end = parse_optional_media_time(obj.get("end"), &format!("{prefix}.end"))?;
    let bookmarks = parse_media_bookmarks(
        obj.get("bookmarks"),
        &format!("{prefix}.bookmarks"),
        allow_null_lists,
    )?;
    let tracks = parse_media_tracks(
        obj.get("tracks"),
        &format!("{prefix}.tracks"),
        allow_null_lists,
    )?;
    let track_display_loc = parse_track_display_loc(
        obj.get("trackDisplayLoc"),
        &format!("{prefix}.trackDisplayLoc"),
        tracks.as_ref(),
    )?;
    Ok(MediaClipFields {
        st,
        end,
        bookmarks,
        tracks,
        track_display_loc,
    })
}

fn parse_optional_media_time(v: Option<&Value>, path: &str) -> Result<Option<f64>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Number(n)) => {
            let x = n
                .as_f64()
                .ok_or_else(|| Error::with_path(ErrorKind::InvalidMediaTime, path.to_string()))?;
            if !x.is_finite() || x < 0.0 {
                return Err(Error::with_path(
                    ErrorKind::InvalidMediaTime,
                    path.to_string(),
                ));
            }
            Ok(Some(x))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::InvalidMediaTime,
            path.to_string(),
        )),
    }
}

fn parse_media_bookmarks(
    v: Option<&Value>,
    path: &str,
    allow_null: bool,
) -> Result<Option<Vec<MediaBookmark>>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Null) if allow_null => Ok(Some(Vec::new())),
        Some(Value::Array(arr)) => {
            let mut out = Vec::new();
            let mut names = Vec::new();
            for (j, item) in arr.iter().enumerate() {
                let obj = item.as_object().ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::WrongType {
                            field: "bookmarks".into(),
                            expected: TypeName::Object,
                        },
                        format!("{path}[{j}]"),
                    )
                })?;
                deny_keys_at(obj, &format!("{path}[{j}]"), &["name", "time"])?;
                let name = match obj.get("name") {
                    Some(Value::String(s)) if !s.is_empty() => s.clone(),
                    _ => {
                        return Err(Error::with_path(
                            ErrorKind::InvalidTrack,
                            format!("{path}[{j}].name"),
                        ));
                    }
                };
                if names.iter().any(|n| n == &name) {
                    return Err(Error::with_path(
                        ErrorKind::DuplicateBookmarkName,
                        format!("{path}[{j}].name"),
                    ));
                }
                names.push(name.clone());
                let time =
                    parse_optional_media_time(obj.get("time"), &format!("{path}[{j}].time"))?
                        .ok_or_else(|| {
                            Error::with_path(
                                ErrorKind::MissingField {
                                    field: "time".into(),
                                },
                                format!("{path}[{j}].time"),
                            )
                        })?;
                out.push(MediaBookmark { name, time });
            }
            Ok(Some(out))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "bookmarks".into(),
                expected: TypeName::Array,
            },
            path.to_string(),
        )),
    }
}

fn parse_media_tracks(
    v: Option<&Value>,
    path: &str,
    allow_null: bool,
) -> Result<Option<Vec<MediaTrackWrite>>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Null) if allow_null => Ok(Some(Vec::new())),
        Some(Value::Array(arr)) => {
            let mut out = Vec::new();
            for (j, item) in arr.iter().enumerate() {
                let obj = item.as_object().ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::WrongType {
                            field: "tracks".into(),
                            expected: TypeName::Object,
                        },
                        format!("{path}[{j}]"),
                    )
                })?;
                deny_keys_at(
                    obj,
                    &format!("{path}[{j}]"),
                    &["id", "label", "lang", "file"],
                )?;
                let id = match obj.get("id") {
                    None => None,
                    Some(Value::String(s)) if media::is_track_guid(s) => Some(s.clone()),
                    Some(_) => {
                        return Err(Error::with_path(
                            ErrorKind::InvalidTrack,
                            format!("{path}[{j}].id"),
                        ));
                    }
                };
                let label = match obj.get("label") {
                    Some(Value::String(s)) if !s.is_empty() => s.clone(),
                    _ => {
                        return Err(Error::with_path(
                            ErrorKind::InvalidTrack,
                            format!("{path}[{j}].label"),
                        ));
                    }
                };
                let lang = match obj.get("lang") {
                    None => None,
                    Some(Value::String(s)) if !s.is_empty() => Some(s.clone()),
                    Some(_) => {
                        return Err(Error::with_path(
                            ErrorKind::InvalidTrack,
                            format!("{path}[{j}].lang"),
                        ));
                    }
                };
                let file = match obj.get("file") {
                    Some(Value::String(s)) if !s.is_empty() => s.clone(),
                    Some(Value::String(_)) | None => {
                        return Err(Error::with_path(
                            ErrorKind::MissingField {
                                field: "file".into(),
                            },
                            format!("{path}[{j}].file"),
                        ));
                    }
                    Some(_) => {
                        return Err(Error::with_path(
                            ErrorKind::WrongType {
                                field: "file".into(),
                                expected: TypeName::NonEmptyString,
                            },
                            format!("{path}[{j}].file"),
                        ));
                    }
                };
                out.push(MediaTrackWrite {
                    id,
                    label,
                    lang,
                    file,
                });
            }
            Ok(Some(out))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "tracks".into(),
                expected: TypeName::Array,
            },
            path.to_string(),
        )),
    }
}

fn parse_track_display_loc(
    v: Option<&Value>,
    path: &str,
    tracks: Option<&Vec<MediaTrackWrite>>,
) -> Result<Option<String>, Error> {
    match v {
        None => Ok(None),
        Some(_) if tracks.map(|t| t.is_empty()).unwrap_or(true) => {
            Err(Error::with_path(ErrorKind::InvalidTrack, path.to_string()))
        }
        Some(Value::String(s)) if s == "media" || s == "slide" => Ok(Some(s.clone())),
        Some(_) => Err(Error::with_path(ErrorKind::InvalidTrack, path.to_string())),
    }
}

fn parse_set_media(obj: &Map<String, Value>, i: usize) -> Result<SetMediaOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op",
            "id",
            "st",
            "end",
            "bookmarks",
            "tracks",
            "trackDisplayLoc",
        ],
    )?;
    let id = require_id(obj, i)?;
    let target = media::parse_media_id(&id, &format!("ops[{i}].id"))?;
    let prefix = format!("ops[{i}]");
    let extras = parse_media_clip_fields(obj, &prefix, true)?;
    if extras.st.is_none()
        && extras.end.is_none()
        && extras.bookmarks.is_none()
        && extras.tracks.is_none()
    {
        return Err(Error::with_path(
            ErrorKind::NeedSetMediaField,
            format!("ops[{i}]"),
        ));
    }
    Ok(SetMediaOp {
        id,
        target,
        st: extras.st,
        end: extras.end,
        bookmarks: extras.bookmarks,
        tracks: extras.tracks,
        track_display_loc: extras.track_display_loc,
    })
}

fn parse_delete_media(obj: &Map<String, Value>, i: usize) -> Result<DeleteMediaOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let target = media::parse_media_id(&id, &format!("ops[{i}].id"))?;
    Ok(DeleteMediaOp { id, target })
}

fn parse_insert_slide_section(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<InsertSlideSectionOp, Error> {
    deny_keys(obj, i, &["op", "id", "name", "start", "after"])?;
    let id = require_id(obj, i)?;
    if id != "pptx:presentation" {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::InsertSlideSectionNeedsPresentation,
            },
            format!("ops[{i}].id"),
        ));
    }
    if obj.contains_key("start") && obj.contains_key("after") {
        return Err(Error::with_path(
            ErrorKind::MutuallyExclusive {
                a: "start".into(),
                b: "after".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(InsertSlideSectionOp {
        name: require_name(obj, i)?,
        start: parse_optional_slide_id_field(obj, i, "start")?,
        after: parse_optional_section_id_field(obj, i, "after")?,
    })
}

fn parse_set_slide_section_name(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<SetSlideSectionNameOp, Error> {
    deny_keys(obj, i, &["op", "id", "name"])?;
    let id = require_id(obj, i)?;
    Ok(SetSlideSectionNameOp {
        section: section::parse_section_id(&id, &format!("ops[{i}].id"))?,
        name: require_name(obj, i)?,
    })
}

fn parse_delete_slide_section(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<DeleteSlideSectionOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    Ok(DeleteSlideSectionOp {
        section: section::parse_section_id(&id, &format!("ops[{i}].id"))?,
    })
}

fn parse_set_slide_section(obj: &Map<String, Value>, i: usize) -> Result<SetSlideSectionOp, Error> {
    deny_keys(obj, i, &["op", "id", "section"])?;
    let id = require_id(obj, i)?;
    let slide = if id
        .strip_prefix("slide:")
        .is_some_and(|rest| !rest.is_empty() && !rest.contains('/'))
    {
        parse_slide_only_id(&id, i, "id")?
    } else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SetSlideSectionNeedsSlide,
            },
            format!("ops[{i}].id"),
        ));
    };
    let section = match obj.get("section") {
        Some(Value::String(s)) => section::parse_section_id(s, &format!("ops[{i}].section"))?,
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "section".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].section"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "section".into(),
                },
                format!("ops[{i}].section"),
            ));
        }
    };
    Ok(SetSlideSectionOp { slide, section })
}

fn parse_insert_custom_show(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<InsertCustomShowOp, Error> {
    deny_keys(obj, i, &["op", "id", "name", "slides", "after"])?;
    let id = require_id(obj, i)?;
    if id != "pptx:presentation" {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::InsertCustomShowNeedsPresentation,
            },
            format!("ops[{i}].id"),
        ));
    }
    let name = require_name(obj, i)?;
    custom_show::check_name(&name, &format!("ops[{i}].name"))?;
    Ok(InsertCustomShowOp {
        name,
        slides: parse_slides_array(obj, i, false)?.unwrap_or_default(),
        after: parse_optional_show_id_field(obj, i, "after")?,
    })
}

fn parse_set_custom_show(obj: &Map<String, Value>, i: usize) -> Result<SetCustomShowOp, Error> {
    deny_keys(obj, i, &["op", "id", "name", "slides"])?;
    let id = require_id(obj, i)?;
    let show = custom_show::parse_show_id(&id, &format!("ops[{i}].id"))?;
    let name = match obj.get("name") {
        None => None,
        Some(Value::String(s)) => {
            custom_show::check_name(s, &format!("ops[{i}].name"))?;
            Some(s.clone())
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
    let slides = parse_slides_array(obj, i, true)?;
    if name.is_none() && slides.is_none() {
        return Err(Error::with_path(
            ErrorKind::NeedCustomShowField,
            format!("ops[{i}]"),
        ));
    }
    Ok(SetCustomShowOp { show, name, slides })
}

fn parse_delete_custom_show(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<DeleteCustomShowOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    Ok(DeleteCustomShowOp {
        show: custom_show::parse_show_id(&id, &format!("ops[{i}].id"))?,
    })
}

fn parse_insert_ole(obj: &Map<String, Value>, i: usize) -> Result<ole::InsertOleOp, Error> {
    deny_keys(obj, i, &["op", "id", "file"])?;
    let id = require_id(obj, i)?;
    let (slide, on_notes, master) = ole::parse_insert_slide_id(&id, i)?.parts();
    let file = require_file(obj, i, "insertOle")?;
    Ok(ole::InsertOleOp {
        slide,
        on_notes,
        master,
        file,
    })
}

fn parse_delete_ole(obj: &Map<String, Value>, i: usize) -> Result<ole::DeleteOleOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let target = ole::parse_ole_id(&id, &format!("ops[{i}].id"))?;
    Ok(ole::DeleteOleOp { id, target })
}

fn parse_insert_model3d(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<super::model3d::InsertModel3dOp, Error> {
    deny_keys(obj, i, &["op", "id", "file", "x", "y", "cx", "cy"])?;
    let id = require_id(obj, i)?;
    let (slide, on_notes, master) =
        picture::parse_insert_host(&id, i, IdReason::InsertModel3dNeedsSlide)?.parts();
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
        slide,
        on_notes,
        master,
        file,
        x: parse_emu_field(obj.get("x"), i, "x")?,
        y: parse_emu_field(obj.get("y"), i, "y")?,
        cx,
        cy,
    })
}

fn parse_replace_model3d(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<super::model3d::ReplaceModel3dOp, Error> {
    deny_keys(obj, i, &["op", "id", "file"])?;
    let id = require_id(obj, i)?;
    let target = super::model3d::parse_id(&id, &format!("ops[{i}].id"))?;
    let file = require_file(obj, i, "replaceModel3d")?;
    Ok(super::model3d::ReplaceModel3dOp { id, target, file })
}

fn parse_delete_model3d(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<super::model3d::DeleteModel3dOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let target = super::model3d::parse_id(&id, &format!("ops[{i}].id"))?;
    Ok(super::model3d::DeleteModel3dOp { id, target })
}

fn parse_insert_math(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<super::omml::InsertMathOp, Error> {
    deny_keys(
        obj,
        i,
        &["op", "id", "text", "x", "y", "cx", "cy", "expr", "kind"],
    )?;
    crate::drawing_math::reject_expr_kind(obj, &format!("ops[{i}]"))?;
    let id = require_id(obj, i)?;
    let (slide, on_notes, master) =
        picture::parse_insert_host(&id, i, IdReason::InsertMathNeedsSlide)?.parts();
    let text = crate::drawing_math::require_text(obj, &format!("ops[{i}]"))?;
    let cx = parse_emu_field(obj.get("cx"), i, "cx")?;
    let cy = parse_emu_field(obj.get("cy"), i, "cy")?;
    if cx.is_some() != cy.is_some() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "insertMath".into(),
                field: "cx and cy together".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(super::omml::InsertMathOp {
        slide,
        on_notes,
        master,
        text,
        x: parse_emu_field(obj.get("x"), i, "x")?,
        y: parse_emu_field(obj.get("y"), i, "y")?,
        cx,
        cy,
    })
}

fn parse_set_math(obj: &Map<String, Value>, i: usize) -> Result<super::omml::SetMathOp, Error> {
    deny_keys(obj, i, &["op", "id", "text", "expr", "kind"])?;
    crate::drawing_math::reject_expr_kind(obj, &format!("ops[{i}]"))?;
    let id = require_id(obj, i)?;
    let target = super::omml::parse_id(&id, &format!("ops[{i}].id"))?;
    let text = crate::drawing_math::require_text(obj, &format!("ops[{i}]"))?;
    Ok(super::omml::SetMathOp { id, target, text })
}

fn parse_delete_math(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<super::omml::DeleteMathOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let target = super::omml::parse_id(&id, &format!("ops[{i}].id"))?;
    Ok(super::omml::DeleteMathOp { id, target })
}

fn parse_insert_zoom(obj: &Map<String, Value>, i: usize) -> Result<zoom::InsertZoomOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op", "id", "type", "target", "targets", "x", "y", "cx", "cy",
        ],
    )?;
    let id = require_id(obj, i)?;
    let (slide, on_notes, master) = zoom::parse_insert_slide_id(&id, i)?.parts();
    let kind_s = match obj.get("type") {
        None => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "type".into(),
                },
                format!("ops[{i}].type"),
            ));
        }
        Some(Value::String(s)) if s == "slide" || s == "section" || s == "summary" => s.clone(),
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidZoomType,
                format!("ops[{i}].type"),
            ));
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
    if obj.contains_key("target") && obj.contains_key("targets") {
        return Err(Error::with_path(
            ErrorKind::MutuallyExclusive {
                a: "target".into(),
                b: "targets".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    let kind = if kind_s == "summary" {
        if obj.contains_key("target") {
            return Err(Error::with_path(
                ErrorKind::InvalidZoomTargets,
                format!("ops[{i}].target"),
            ));
        }
        let targets = parse_zoom_targets(obj, i)?;
        zoom::InsertKind::Summary { targets }
    } else if obj.contains_key("targets") {
        return Err(Error::with_path(
            ErrorKind::InvalidZoomTargets,
            format!("ops[{i}].targets"),
        ));
    } else {
        let target_s = match obj.get("target") {
            None => {
                return Err(Error::with_path(
                    ErrorKind::MissingField {
                        field: "target".into(),
                    },
                    format!("ops[{i}].target"),
                ));
            }
            Some(Value::String(s)) if !s.is_empty() => s.as_str(),
            Some(Value::String(_)) => {
                return Err(Error::with_path(
                    ErrorKind::MissingField {
                        field: "target".into(),
                    },
                    format!("ops[{i}].target"),
                ));
            }
            Some(_) => {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: "target".into(),
                        expected: TypeName::String,
                    },
                    format!("ops[{i}].target"),
                ));
            }
        };
        if kind_s == "slide" {
            let target = zoom::parse_slide_target(target_s, &format!("ops[{i}].target"))?;
            if target == slide {
                return Err(Error::with_path(
                    ErrorKind::ZoomTargetSelf,
                    format!("ops[{i}].target"),
                ));
            }
            zoom::InsertKind::Slide { target }
        } else {
            let target = section::parse_section_id(target_s, &format!("ops[{i}].target"))?;
            zoom::InsertKind::Section { target }
        }
    };
    let x = parse_off_field(obj.get("x"), i, "x")?;
    let y = parse_off_field(obj.get("y"), i, "y")?;
    if x.is_some() != y.is_some() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "insertZoom".into(),
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
                op: "insertZoom".into(),
                field: "cx and cy together".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(zoom::InsertZoomOp {
        slide,
        on_notes,
        master,
        kind,
        x,
        y,
        cx,
        cy,
    })
}

fn parse_zoom_targets(obj: &Map<String, Value>, i: usize) -> Result<Vec<usize>, Error> {
    match obj.get("targets") {
        None => Err(Error::with_path(
            ErrorKind::MissingField {
                field: "targets".into(),
            },
            format!("ops[{i}].targets"),
        )),
        Some(Value::Array(arr)) if arr.is_empty() => Err(Error::with_path(
            ErrorKind::InvalidZoomTargets,
            format!("ops[{i}].targets"),
        )),
        Some(Value::Array(arr)) => {
            let mut out = Vec::new();
            for (j, v) in arr.iter().enumerate() {
                let s = v.as_str().ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::WrongType {
                            field: format!("targets[{j}]"),
                            expected: TypeName::String,
                        },
                        format!("ops[{i}].targets[{j}]"),
                    )
                })?;
                let n = section::parse_section_id(s, &format!("ops[{i}].targets[{j}]"))?;
                if out.contains(&n) {
                    return Err(Error::with_path(
                        ErrorKind::InvalidZoomTargets,
                        format!("ops[{i}].targets[{j}]"),
                    ));
                }
                out.push(n);
            }
            Ok(out)
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "targets".into(),
                expected: TypeName::Array,
            },
            format!("ops[{i}].targets"),
        )),
    }
}

fn parse_delete_zoom(obj: &Map<String, Value>, i: usize) -> Result<zoom::DeleteZoomOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let target = zoom::parse_zoom_id(&id, &format!("ops[{i}].id"))?;
    Ok(zoom::DeleteZoomOp { id, target })
}

fn parse_set_zoom(obj: &Map<String, Value>, i: usize) -> Result<zoom::SetZoomOp, Error> {
    deny_keys(obj, i, &["op", "id", "target", "targets", "file"])?;
    let id = require_id(obj, i)?;
    let target = zoom::parse_zoom_id(&id, &format!("ops[{i}].id"))?;
    if obj.contains_key("target") && obj.contains_key("targets") {
        return Err(Error::with_path(
            ErrorKind::MutuallyExclusive {
                a: "target".into(),
                b: "targets".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    let dest = if obj.contains_key("targets") {
        Some(zoom::SetZoomDest::Summary {
            targets: parse_zoom_targets(obj, i)?,
        })
    } else {
        match obj.get("target") {
            None => None,
            Some(Value::String(s)) if s.starts_with("slide:") => {
                let n = zoom::parse_slide_target(s, &format!("ops[{i}].target"))?;
                Some(zoom::SetZoomDest::Slide { target: n })
            }
            Some(Value::String(s)) if s.starts_with("section:") => {
                let n = section::parse_section_id(s, &format!("ops[{i}].target"))?;
                Some(zoom::SetZoomDest::Section { target: n })
            }
            Some(Value::String(_)) => {
                return Err(Error::with_path(
                    ErrorKind::InvalidId {
                        reason: IdReason::ZoomTargetSlide,
                    },
                    format!("ops[{i}].target"),
                ));
            }
            Some(_) => {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: "target".into(),
                        expected: TypeName::String,
                    },
                    format!("ops[{i}].target"),
                ));
            }
        }
    };
    let file = match obj.get("file") {
        None => None,
        Some(Value::String(s)) if !s.is_empty() => Some(s.clone()),
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "file".into(),
                },
                format!("ops[{i}].file"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "file".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].file"),
            ));
        }
    };
    if dest.is_none() && file.is_none() {
        return Err(Error::with_path(
            ErrorKind::NeedZoomSetField,
            format!("ops[{i}]"),
        ));
    }
    Ok(zoom::SetZoomOp {
        id,
        target,
        dest,
        file,
    })
}

fn parse_set_show_pr(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<super::show::SetShowPrOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op",
            "id",
            "type",
            "loop",
            "showNarration",
            "showAnimation",
            "useTimings",
        ],
    )?;
    let id = require_id(obj, i)?;
    if id != "pptx:presentation" {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SetShowPrNeedsPresentation,
            },
            format!("ops[{i}].id"),
        ));
    }
    let type_name = match obj.get("type") {
        None => None,
        Some(Value::String(s)) if matches!(s.as_str(), "present" | "browse" | "kiosk") => {
            Some(s.clone())
        }
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidShowPrType,
                format!("ops[{i}].type"),
            ));
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
    let loop_ = parse_opt_bool(obj, i, "loop")?;
    let show_narration = parse_opt_bool(obj, i, "showNarration")?;
    let show_animation = parse_opt_bool(obj, i, "showAnimation")?;
    let use_timings = parse_opt_bool(obj, i, "useTimings")?;
    if type_name.is_none()
        && loop_.is_none()
        && show_narration.is_none()
        && show_animation.is_none()
        && use_timings.is_none()
    {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setShowPr".into(),
                field: "type, loop, showNarration, showAnimation, or useTimings".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(super::show::SetShowPrOp {
        type_name,
        loop_,
        show_narration,
        show_animation,
        use_timings,
    })
}

fn parse_opt_password(obj: &Map<String, Value>, i: usize) -> Result<Option<String>, Error> {
    match obj.get("password") {
        None => Ok(None),
        Some(Value::String(s)) if !s.is_empty() => Ok(Some(s.clone())),
        Some(Value::String(_)) => Err(Error::with_path(
            ErrorKind::ProtectionPasswordBlank,
            format!("ops[{i}].password"),
        )),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "password".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].password"),
        )),
    }
}

fn parse_set_protection(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<super::protect::SetProtectionOp, Error> {
    deny_keys(obj, i, &["op", "id", "enforced", "password"])?;
    let id = require_id(obj, i)?;
    if id != "pptx:presentation" {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SetProtectionNeedsPresentation,
            },
            format!("ops[{i}].id"),
        ));
    }
    let enforced = match obj.get("enforced") {
        Some(Value::Bool(b)) => *b,
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "enforced".into(),
                    expected: TypeName::Boolean,
                },
                format!("ops[{i}].enforced"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::NeedProtectionField,
                format!("ops[{i}]"),
            ));
        }
    };
    Ok(super::protect::SetProtectionOp {
        enforced,
        password: parse_opt_password(obj, i)?,
    })
}

fn parse_slides_array(
    obj: &Map<String, Value>,
    i: usize,
    optional: bool,
) -> Result<Option<Vec<usize>>, Error> {
    match obj.get("slides") {
        None => {
            if optional {
                Ok(None)
            } else {
                Ok(Some(Vec::new()))
            }
        }
        Some(Value::Array(arr)) => {
            let mut out = Vec::new();
            for (j, v) in arr.iter().enumerate() {
                let s = v.as_str().ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::WrongType {
                            field: format!("slides[{j}]"),
                            expected: TypeName::String,
                        },
                        format!("ops[{i}].slides[{j}]"),
                    )
                })?;
                out.push(parse_slide_only_id(s, i, &format!("slides[{j}]"))?);
            }
            Ok(Some(out))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "slides".into(),
                expected: TypeName::Array,
            },
            format!("ops[{i}].slides"),
        )),
    }
}

fn parse_optional_show_id_field(
    obj: &Map<String, Value>,
    i: usize,
    field: &str,
) -> Result<Option<usize>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::String(s)) => Ok(Some(custom_show::parse_show_id(
            s,
            &format!("ops[{i}].{field}"),
        )?)),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].{field}"),
        )),
    }
}

fn require_name(obj: &Map<String, Value>, i: usize) -> Result<String, Error> {
    match obj.get("name") {
        Some(Value::String(s)) => Ok(s.clone()),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "name".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].name"),
        )),
        None => Err(Error::with_path(
            ErrorKind::MissingField {
                field: "name".into(),
            },
            format!("ops[{i}].name"),
        )),
    }
}

fn parse_optional_slide_id_field(
    obj: &Map<String, Value>,
    i: usize,
    field: &str,
) -> Result<Option<usize>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::String(s)) => Ok(Some(parse_slide_only_id(s, i, field)?)),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].{field}"),
        )),
    }
}

fn parse_optional_section_id_field(
    obj: &Map<String, Value>,
    i: usize,
    field: &str,
) -> Result<Option<usize>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::String(s)) => Ok(Some(section::parse_section_id(
            s,
            &format!("ops[{i}].{field}"),
        )?)),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].{field}"),
        )),
    }
}

fn parse_insert_chart(obj: &Map<String, Value>, i: usize) -> Result<InsertChartOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op",
            "id",
            "type",
            "ofPieType",
            "title",
            "series",
            "x",
            "y",
            "cx",
            "cy",
        ],
    )?;
    let id = require_id(obj, i)?;
    let (slide, on_notes, master) = chart::parse_insert_slide_id(&id, i)?.parts();
    let chart_type = parse_chart_type_field(obj.get("type"), &format!("ops[{i}].type"))?;
    let of_pie_type =
        crate::chart::parse_of_pie_type(obj.get("ofPieType"), &format!("ops[{i}].ofPieType"))?;
    crate::chart::reject_of_pie_on_other(
        chart_type,
        of_pie_type.as_deref(),
        &format!("ops[{i}].ofPieType"),
    )?;
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
        on_notes,
        master,
        chart_type,
        of_pie_type,
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
    deny_keys(obj, i, &["op", "id", "type", "ofPieType"])?;
    let id = require_id(obj, i)?;
    let target = chart::parse_chart_id(&id, &format!("ops[{i}].id"))?;
    let chart_type = parse_chart_type_field(obj.get("type"), &format!("ops[{i}].type"))?;
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
    Ok(SetChartTypeOp {
        id,
        target,
        chart_type,
        of_pie_type,
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
    let (slide, on_notes, master) = smartart::parse_insert_slide_id(&id, i)?.parts();
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
        on_notes,
        master,
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
    let (slide, on_notes, master) = table::parse_insert_slide_id(&id, i)?.parts();
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
        on_notes,
        master,
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
    crate::chart::validate_insert_series_len(chart_type, series.len(), path)?;
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
        &["type", "ofPieType", "title", "series", "x", "y", "cx", "cy"],
    )?;
    let chart_type = parse_chart_type_field(obj.get("type"), &format!("{prefix}.type"))?;
    let of_pie_type =
        crate::chart::parse_of_pie_type(obj.get("ofPieType"), &format!("{prefix}.ofPieType"))?;
    crate::chart::reject_of_pie_on_other(
        chart_type,
        of_pie_type.as_deref(),
        &format!("{prefix}.ofPieType"),
    )?;
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
        on_notes: false,
        master: None,
        chart_type,
        of_pie_type,
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
        on_notes: false,
        master: None,
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
        on_notes: false,
        master: None,
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
        master: None,
        comment: None,
        file,
        x,
        y,
        cx,
        cy,
    })
}

pub(crate) fn parse_create_comment(
    v: &Value,
    prefix: &str,
    slide: usize,
) -> Result<InsertCommentOp, Error> {
    let obj = v.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "comments".into(),
                expected: TypeName::Object,
            },
            prefix.to_string(),
        )
    })?;
    deny_keys_at(
        obj,
        prefix,
        &["text", "author", "initials", "date", "x", "y"],
    )?;
    let text = match obj.get("text") {
        None => String::new(),
        Some(Value::String(s)) => s.clone(),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "text".into(),
                    expected: TypeName::String,
                },
                format!("{prefix}.text"),
            ));
        }
    };
    let author = match obj.get("author") {
        None => "ooxp".into(),
        Some(Value::String(s)) => s.clone(),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "author".into(),
                    expected: TypeName::String,
                },
                format!("{prefix}.author"),
            ));
        }
    };
    let initials = match obj.get("initials") {
        None => "oo".into(),
        Some(Value::String(s)) => s.clone(),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "initials".into(),
                    expected: TypeName::String,
                },
                format!("{prefix}.initials"),
            ));
        }
    };
    let date = match obj.get("date") {
        None => None,
        Some(Value::String(s)) if !s.is_empty() => Some(s.clone()),
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "date".into(),
                    expected: TypeName::NonEmptyString,
                },
                format!("{prefix}.date"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "date".into(),
                    expected: TypeName::String,
                },
                format!("{prefix}.date"),
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

pub(crate) fn parse_create_media(
    v: &Value,
    prefix: &str,
    slide: usize,
) -> Result<InsertMediaOp, Error> {
    let obj = v.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "file".into(),
                expected: TypeName::Object,
            },
            prefix.to_string(),
        )
    })?;
    deny_keys_at(
        obj,
        prefix,
        &[
            "file",
            "x",
            "y",
            "cx",
            "cy",
            "st",
            "end",
            "bookmarks",
            "tracks",
            "trackDisplayLoc",
        ],
    )?;
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
    let extras = parse_media_clip_fields(obj, prefix, false)?;
    Ok(InsertMediaOp {
        slide,
        on_notes: false,
        master: None,
        file,
        x,
        y,
        cx,
        cy,
        st: extras.st,
        end: extras.end,
        bookmarks: extras.bookmarks,
        tracks: extras.tracks,
        track_display_loc: extras.track_display_loc,
    })
}

pub(crate) fn parse_create_shape(
    v: &Value,
    prefix: &str,
) -> Result<
    (
        String,
        Option<CreateHyperlink>,
        Option<crate::drawing_text::TextFormat>,
    ),
    Error,
> {
    let obj = v.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "shapes".into(),
                expected: TypeName::Object,
            },
            prefix.to_string(),
        )
    })?;
    deny_keys_at(
        obj,
        prefix,
        &[
            "text",
            "url",
            "file",
            "slide",
            "tooltip",
            "on",
            "event",
            "sz",
            "color",
            "bold",
            "italic",
            "underline",
            "font",
            "eastAsiaFont",
            "align",
            "textAnchor",
        ],
    )?;
    let text = match obj.get("text") {
        None => String::new(),
        Some(Value::String(s)) => s.clone(),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "text".into(),
                    expected: TypeName::String,
                },
                format!("{prefix}.text"),
            ));
        }
    };
    let url = parse_create_link_string(obj, prefix, "url", ErrorKind::EmptyHyperlinkUrl)?;
    let file = parse_create_link_string(obj, prefix, "file", ErrorKind::EmptyHyperlinkFile)?;
    let slide = match obj.get("slide") {
        None => None,
        Some(Value::Null) => {
            return Err(Error::with_path(
                ErrorKind::PptxHyperlinkNeedTarget,
                format!("{prefix}.slide"),
            ));
        }
        Some(Value::Number(n)) => {
            let Some(u) = n.as_u64() else {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: "slide".into(),
                        expected: TypeName::Integer,
                    },
                    format!("{prefix}.slide"),
                ));
            };
            if u < 1 || u > usize::MAX as u64 {
                return Err(Error::with_path(
                    ErrorKind::MinValue {
                        field: "slide".into(),
                        min: 1,
                    },
                    format!("{prefix}.slide"),
                ));
            }
            Some(u as usize)
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "slide".into(),
                    expected: TypeName::Integer,
                },
                format!("{prefix}.slide"),
            ));
        }
    };
    let tooltip = match obj.get("tooltip") {
        None => None,
        Some(Value::String(s)) => Some(s.clone()),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "tooltip".into(),
                    expected: TypeName::String,
                },
                format!("{prefix}.tooltip"),
            ));
        }
    };
    let target = match (url, file, slide) {
        (Some(u), None, None) => Some(PptxLinkKind::Url(u)),
        (None, Some(f), None) => Some(PptxLinkKind::File(f)),
        (None, None, Some(n)) => Some(PptxLinkKind::Slide(n)),
        (None, None, None) => None,
        _ => {
            return Err(Error::with_path(
                ErrorKind::PptxHyperlinkNeedTarget,
                prefix.to_string(),
            ));
        }
    };
    if target.is_none() && tooltip.is_some() {
        return Err(Error::with_path(
            ErrorKind::PptxHyperlinkNeedTarget,
            format!("{prefix}.tooltip"),
        ));
    }
    let on_run = match obj.get("on") {
        None => false,
        Some(Value::String(s)) if s == "shape" => false,
        Some(Value::String(s)) if s == "run" => true,
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidHyperlinkOn,
                format!("{prefix}.on"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "on".into(),
                    expected: TypeName::String,
                },
                format!("{prefix}.on"),
            ));
        }
    };
    let event = match obj.get("event") {
        None => LinkEvent::Click,
        Some(Value::String(s)) if s == "click" => LinkEvent::Click,
        Some(Value::String(s)) if s == "hover" => LinkEvent::Hover,
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidHyperlinkEvent,
                format!("{prefix}.event"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "event".into(),
                    expected: TypeName::String,
                },
                format!("{prefix}.event"),
            ));
        }
    };
    if target.is_none() && (on_run || obj.get("on").is_some() || obj.get("event").is_some()) {
        return Err(Error::with_path(
            ErrorKind::PptxHyperlinkNeedTarget,
            prefix.to_string(),
        ));
    }
    let link = target.map(|target| CreateHyperlink {
        target,
        tooltip,
        on_run,
        event,
    });
    let format = crate::drawing_text::parse(obj, prefix, false)?;
    Ok((text, link, format))
}

pub(crate) fn parse_create_transition(
    v: &Value,
    prefix: &str,
    slide: usize,
) -> Result<SetTransitionOp, Error> {
    let obj = v.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "transition".into(),
                expected: TypeName::Object,
            },
            prefix.to_string(),
        )
    })?;
    deny_keys_at(
        obj,
        prefix,
        &[
            "type", "dur", "advClick", "advTm", "dir", "invX", "invY", "sound", "orient", "spokes",
        ],
    )?;
    let type_name = match obj.get("type") {
        None => {
            return Err(Error::with_path(
                ErrorKind::OpRequiresField {
                    op: "create".into(),
                    field: "type".into(),
                },
                format!("{prefix}.type"),
            ));
        }
        Some(Value::Null) => {
            return Err(Error::with_path(
                ErrorKind::InvalidTransitionType,
                format!("{prefix}.type"),
            ));
        }
        Some(Value::String(s)) => {
            if s.is_empty() || s == "other" || !transition::is_writable_type(s) {
                return Err(Error::with_path(
                    ErrorKind::InvalidTransitionType,
                    format!("{prefix}.type"),
                ));
            }
            s.clone()
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "type".into(),
                    expected: TypeName::String,
                },
                format!("{prefix}.type"),
            ));
        }
    };
    let dur = match obj.get("dur") {
        None => None,
        Some(v) => Some(parse_transition_ms_at(
            v,
            &format!("{prefix}.dur"),
            "dur",
            ErrorKind::InvalidTransitionDur,
            1,
            60000,
        )?),
    };
    let adv_click = match obj.get("advClick") {
        None => true,
        Some(Value::Bool(b)) => *b,
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "advClick".into(),
                    expected: TypeName::Boolean,
                },
                format!("{prefix}.advClick"),
            ));
        }
    };
    let adv_tm = match obj.get("advTm") {
        None | Some(Value::Null) => None,
        Some(v) => Some(parse_transition_ms_at(
            v,
            &format!("{prefix}.advTm"),
            "advTm",
            ErrorKind::InvalidTransitionAdvTm,
            1,
            86400000,
        )?),
    };
    let dir = match obj.get("dir") {
        None => None,
        Some(Value::String(s)) if !s.is_empty() => Some(s.clone()),
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidTransitionDir,
                format!("{prefix}.dir"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "dir".into(),
                    expected: TypeName::String,
                },
                format!("{prefix}.dir"),
            ));
        }
    };
    let inv_x = parse_optional_flag_at(obj.get("invX"), &format!("{prefix}.invX"), "invX")?;
    let inv_y = parse_optional_flag_at(obj.get("invY"), &format!("{prefix}.invY"), "invY")?;
    let sound = match obj.get("sound") {
        None => None,
        Some(Value::String(s)) if !s.is_empty() => Some(s.clone()),
        Some(Value::String(_)) | Some(Value::Null) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "sound".into(),
                    expected: TypeName::String,
                },
                format!("{prefix}.sound"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "sound".into(),
                    expected: TypeName::String,
                },
                format!("{prefix}.sound"),
            ));
        }
    };
    let orient = match obj.get("orient") {
        None => None,
        Some(Value::String(s)) if !s.is_empty() => Some(s.clone()),
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidTransitionOrient,
                format!("{prefix}.orient"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "orient".into(),
                    expected: TypeName::String,
                },
                format!("{prefix}.orient"),
            ));
        }
    };
    let spokes = match obj.get("spokes") {
        None => None,
        Some(v) => Some(parse_transition_spokes(v, &format!("{prefix}.spokes"))?),
    };
    Ok(SetTransitionOp {
        slide,
        master: None,
        type_name: Some(type_name),
        dur,
        adv_click,
        adv_tm,
        dir,
        inv_x,
        inv_y,
        sound,
        orient,
        spokes,
    })
}

pub(crate) fn parse_create_anim(v: &Value, prefix: &str) -> Result<CreateAnim, Error> {
    let obj = v.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "anims".into(),
                expected: TypeName::Object,
            },
            prefix.to_string(),
        )
    })?;
    deny_keys_at(
        obj,
        prefix,
        &[
            "type", "trigger", "delay", "dur", "repeat", "sound", "para", "clickOn", "shape",
            "picture", "video", "audio", "table", "chart", "smartArt",
        ],
    )?;
    let type_name = match obj.get("type") {
        None => {
            return Err(Error::with_path(
                ErrorKind::OpRequiresField {
                    op: "create".into(),
                    field: "type".into(),
                },
                format!("{prefix}.type"),
            ));
        }
        Some(Value::String(s)) if timing::anim_preset_ok(s) => s.clone(),
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidAnimType,
                format!("{prefix}.type"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "type".into(),
                    expected: TypeName::String,
                },
                format!("{prefix}.type"),
            ));
        }
    };
    let trigger = match obj.get("trigger") {
        None => "click".to_string(),
        Some(Value::String(s)) if matches!(s.as_str(), "click" | "with" | "after" | "clickOn") => {
            s.clone()
        }
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidAnimTrigger,
                format!("{prefix}.trigger"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "trigger".into(),
                    expected: TypeName::String,
                },
                format!("{prefix}.trigger"),
            ));
        }
    };
    let delay = match obj.get("delay") {
        None => 0,
        Some(v) => parse_transition_ms_at(
            v,
            &format!("{prefix}.delay"),
            "delay",
            ErrorKind::InvalidAnimDelay,
            0,
            60000,
        )?,
    };
    let dur = match obj.get("dur") {
        None => 500,
        Some(v) => parse_transition_ms_at(
            v,
            &format!("{prefix}.dur"),
            "dur",
            ErrorKind::InvalidAnimDur,
            1,
            60000,
        )?,
    };
    let repeat = parse_create_anim_repeat(obj.get("repeat"), &format!("{prefix}.repeat"))?;
    let sound = match obj.get("sound") {
        None => None,
        Some(Value::String(s)) if !s.is_empty() => Some(s.clone()),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "sound".into(),
                    expected: TypeName::String,
                },
                format!("{prefix}.sound"),
            ));
        }
    };
    let para = match obj.get("para") {
        None => None,
        Some(Value::Number(n)) => {
            let Some(v) = n.as_u64() else {
                return Err(Error::with_path(
                    ErrorKind::InvalidAnimPara,
                    format!("{prefix}.para"),
                ));
            };
            if v < 1 {
                return Err(Error::with_path(
                    ErrorKind::InvalidAnimPara,
                    format!("{prefix}.para"),
                ));
            }
            Some(v as u32)
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::InvalidAnimPara,
                format!("{prefix}.para"),
            ));
        }
    };
    let host = parse_create_anim_host(obj, prefix)?;
    let click_on = match (trigger.as_str(), obj.get("clickOn")) {
        ("clickOn", None) => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "clickOn".into(),
                },
                format!("{prefix}.clickOn"),
            ));
        }
        ("clickOn", Some(v)) => Some(parse_create_click_on(v, &format!("{prefix}.clickOn"))?),
        (_, None) => None,
        (_, Some(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidAnimTrigger,
                format!("{prefix}.clickOn"),
            ));
        }
    };
    Ok(CreateAnim {
        type_name,
        trigger,
        delay,
        dur,
        repeat,
        sound,
        para,
        host,
        click_on,
    })
}

fn parse_create_anim_repeat(v: Option<&Value>, path: &str) -> Result<timing::RepeatWrite, Error> {
    match v {
        None => Ok(timing::RepeatWrite::Once),
        Some(Value::Null) => Ok(timing::RepeatWrite::Indefinite),
        Some(Value::Number(n)) => {
            let Some(v) = n.as_u64() else {
                return Err(Error::with_path(
                    ErrorKind::InvalidAnimRepeat,
                    path.to_string(),
                ));
            };
            if !(1..=99).contains(&v) {
                return Err(Error::with_path(
                    ErrorKind::InvalidAnimRepeat,
                    path.to_string(),
                ));
            }
            if v == 1 {
                Ok(timing::RepeatWrite::Once)
            } else {
                Ok(timing::RepeatWrite::Times(v as u32))
            }
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::InvalidAnimRepeat,
            path.to_string(),
        )),
    }
}

fn parse_create_click_on(v: &Value, prefix: &str) -> Result<CreateAnimHost, Error> {
    let obj = v.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "clickOn".into(),
                expected: TypeName::Object,
            },
            prefix.to_string(),
        )
    })?;
    deny_keys_at(
        obj,
        prefix,
        &[
            "shape", "picture", "video", "audio", "table", "chart", "smartArt",
        ],
    )?;
    parse_create_anim_host(obj, prefix)
}

fn parse_create_anim_host(obj: &Map<String, Value>, prefix: &str) -> Result<CreateAnimHost, Error> {
    let mut found: Vec<(&str, CreateAnimHost)> = Vec::new();
    for (field, make) in [
        (
            "shape",
            CreateAnimHost::Shape as fn(usize) -> CreateAnimHost,
        ),
        (
            "picture",
            CreateAnimHost::Picture as fn(usize) -> CreateAnimHost,
        ),
        (
            "video",
            CreateAnimHost::Video as fn(usize) -> CreateAnimHost,
        ),
        (
            "audio",
            CreateAnimHost::Audio as fn(usize) -> CreateAnimHost,
        ),
        (
            "table",
            CreateAnimHost::Table as fn(usize) -> CreateAnimHost,
        ),
        (
            "chart",
            CreateAnimHost::Chart as fn(usize) -> CreateAnimHost,
        ),
        (
            "smartArt",
            CreateAnimHost::SmartArt as fn(usize) -> CreateAnimHost,
        ),
    ] {
        if obj.contains_key(field) {
            let n = parse_create_index(obj.get(field), &format!("{prefix}.{field}"), field)?;
            found.push((field, make(n)));
        }
    }
    match found.len() {
        0 => Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "create".into(),
                field: "shape".into(),
            },
            prefix.to_string(),
        )),
        1 => Ok(found.remove(0).1),
        _ => Err(Error::with_path(
            ErrorKind::MutuallyExclusive {
                a: found[0].0.into(),
                b: found[1].0.into(),
            },
            prefix.to_string(),
        )),
    }
}

pub(crate) fn parse_create_section(v: &Value, prefix: &str) -> Result<CreateSection, Error> {
    let obj = v.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "sections".into(),
                expected: TypeName::Object,
            },
            prefix.to_string(),
        )
    })?;
    deny_keys_at(obj, prefix, &["name", "start"])?;
    let name = match obj.get("name") {
        Some(Value::String(s)) => s.clone(),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "name".into(),
                    expected: TypeName::String,
                },
                format!("{prefix}.name"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "name".into(),
                },
                format!("{prefix}.name"),
            ));
        }
    };
    let start = match obj.get("start") {
        None => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "start".into(),
                },
                format!("{prefix}.start"),
            ));
        }
        Some(v) => parse_create_index(Some(v), &format!("{prefix}.start"), "start")?,
    };
    Ok(CreateSection { name, start })
}

fn parse_create_index(v: Option<&Value>, path: &str, field: &str) -> Result<usize, Error> {
    match v {
        Some(Value::Number(n)) => {
            let Some(u) = n.as_u64() else {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: field.into(),
                        expected: TypeName::Integer,
                    },
                    path.to_string(),
                ));
            };
            if u < 1 || u > usize::MAX as u64 {
                return Err(Error::with_path(
                    ErrorKind::MinValue {
                        field: field.into(),
                        min: 1,
                    },
                    path.to_string(),
                ));
            }
            Ok(u as usize)
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Integer,
            },
            path.to_string(),
        )),
        None => Err(Error::with_path(
            ErrorKind::MissingField {
                field: field.into(),
            },
            path.to_string(),
        )),
    }
}

fn parse_create_link_string(
    obj: &Map<String, Value>,
    prefix: &str,
    field: &str,
    empty: ErrorKind,
) -> Result<Option<String>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::Null) => Err(Error::with_path(empty, format!("{prefix}.{field}"))),
        Some(Value::String(s)) if s.is_empty() => {
            Err(Error::with_path(empty, format!("{prefix}.{field}")))
        }
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::String,
            },
            format!("{prefix}.{field}"),
        )),
    }
}

fn parse_transition_ms_at(
    v: &Value,
    path: &str,
    field: &str,
    invalid: ErrorKind,
    min: u64,
    max: u64,
) -> Result<u32, Error> {
    let n = match v {
        Value::Number(num) => num.as_u64().ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: field.into(),
                    expected: TypeName::Integer,
                },
                path.to_string(),
            )
        })?,
        _ => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: field.into(),
                    expected: TypeName::Integer,
                },
                path.to_string(),
            ));
        }
    };
    if !(min..=max).contains(&n) {
        return Err(Error::with_path(invalid, path.to_string()));
    }
    Ok(n as u32)
}

fn parse_optional_flag_at(v: Option<&Value>, path: &str, field: &str) -> Result<bool, Error> {
    match v {
        None => Ok(false),
        Some(Value::Bool(b)) => Ok(*b),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Boolean,
            },
            path.to_string(),
        )),
    }
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
