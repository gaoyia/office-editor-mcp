//! pptx request：types。

use serde_json::Value;

use crate::chart::{ChartExtra, ChartType, NewSeries};
use crate::smartart::{ColorsKind, LayoutKind, NewPoint, Role, StyleKind};

use super::super::chart;
use super::super::comments;
use super::super::connector;
use super::super::field;
use super::super::group;
use super::super::media;
use super::super::picture;
use super::super::smartart;
use super::super::table;
use super::super::timing;

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
    pub geom: Option<Vec<super::super::geom::Path>>,
    pub format: Option<crate::drawing_text::TextFormat>,
}

#[derive(Debug, Clone)]
pub struct DeleteShapeOp {
    pub id: String,
    pub target: Option<super::super::shape::ShapeTarget>,
    pub placeholder: Option<super::super::master::PlaceholderId>,
}

#[derive(Debug, Clone)]
pub struct SetShapePosOp {
    pub id: String,
    pub target: Option<super::super::shape::ShapeTarget>,
    pub placeholder: Option<super::super::master::PlaceholderId>,
    pub cxn: Option<super::super::connector::CxnTarget>,
    pub x: Option<i64>,
    pub y: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct SetShapeSizeOp {
    pub id: String,
    pub target: Option<super::super::shape::ShapeTarget>,
    pub placeholder: Option<super::super::master::PlaceholderId>,
    pub cxn: Option<super::super::connector::CxnTarget>,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct SetShapeFillOp {
    pub id: String,
    pub target: Option<super::super::shape::ShapeTarget>,
    pub placeholder: Option<super::super::master::PlaceholderId>,
    pub color: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SetShapeLineOp {
    pub id: String,
    pub target: Option<super::super::shape::ShapeTarget>,
    pub placeholder: Option<super::super::master::PlaceholderId>,
    pub cxn: Option<super::super::connector::CxnTarget>,
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
    pub host: super::super::master::PlaceholderHost,
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
    pub placeholder: super::super::master::PlaceholderId,
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
    pub fonts: Vec<(super::super::theme::FontSlot, String)>,
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
    Shape(super::super::shape::ShapeTarget),
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

    pub(super) fn default_on(&self) -> LinkOn {
        match self {
            Self::Shape(_) => LinkOn::Shape,
            Self::Pic(_) => LinkOn::Pic,
            Self::Table(_) => LinkOn::Table,
            Self::Chart(_) => LinkOn::Chart,
            Self::SmartArt(_) => LinkOn::SmartArt,
            Self::Cell(_) => LinkOn::Cell,
        }
    }

    pub(super) fn on_ok(&self, on: LinkOn) -> bool {
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
pub enum PicGeomTarget {
    Picture(picture::PicTarget),
    Media(media::MediaTarget),
}

impl PicGeomTarget {
    pub fn slide(&self) -> usize {
        match self {
            Self::Picture(t) => t.slide,
            Self::Media(t) => t.slide,
        }
    }

    pub fn on_notes(&self) -> bool {
        match self {
            Self::Picture(t) => t.on_notes,
            Self::Media(t) => t.on_notes,
        }
    }

    pub fn master(&self) -> Option<(usize, Option<usize>)> {
        match self {
            Self::Picture(t) => t.master,
            Self::Media(t) => t.master,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SetPictureSizeOp {
    pub id: String,
    pub target: PicGeomTarget,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct SetPicturePosOp {
    pub id: String,
    pub target: PicGeomTarget,
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

#[derive(Debug, Clone)]
pub struct CreatePicture {
    pub op: InsertPictureOp,
    pub link: Option<CreateHyperlink>,
}

#[derive(Debug, Clone)]
pub struct CreateChart {
    pub op: InsertChartOp,
    pub link: Option<CreateHyperlink>,
}

#[derive(Debug, Clone)]
pub struct CreateSmartArt {
    pub op: InsertSmartArtOp,
    pub link: Option<CreateHyperlink>,
}

#[derive(Debug, Clone)]
pub struct CreateTableCellLink {
    pub row: usize,
    pub col: usize,
    pub link: CreateHyperlink,
}

#[derive(Debug, Clone)]
pub struct CreateTable {
    pub op: InsertTableOp,
    pub link: Option<CreateHyperlink>,
    pub cell_links: Vec<CreateTableCellLink>,
}

#[derive(Debug, Clone)]
pub struct CreateField {
    pub type_name: String,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct CreateShape {
    pub text: String,
    pub link: Option<CreateHyperlink>,
    pub format: Option<crate::drawing_text::TextFormat>,
    pub geom: Option<Vec<super::super::geom::Path>>,
    pub fields: Vec<CreateField>,
}

#[derive(Debug, Clone)]
pub struct CreateCxnBind {
    pub shape: usize,
    pub site: u32,
}

#[derive(Debug, Clone)]
pub struct CreateConnector {
    pub prst: String,
    pub x: Option<i64>,
    pub y: Option<i64>,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
    pub start: Option<CreateCxnBind>,
    pub end: Option<CreateCxnBind>,
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
    pub extra: ChartExtra,
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
    pub extra: ChartExtra,
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
    pub col_widths: Option<Vec<i64>>,
    pub row_heights: Option<Vec<i64>>,
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
pub struct SetTableColWidthsOp {
    pub id: String,
    pub target: table::TableTarget,
    pub cols: Vec<i64>,
}

#[derive(Debug, Clone)]
pub struct SetTableTrHeightOp {
    pub id: String,
    pub table: table::TableTarget,
    pub row: usize,
    pub h: i64,
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
    pub start: Option<super::super::connector::CxnBind>,
    pub end: Option<super::super::connector::CxnBind>,
}

#[derive(Debug, Clone)]
pub struct SetShapeGeomOp {
    pub id: String,
    pub target: Option<super::super::shape::ShapeTarget>,
    pub placeholder: Option<super::super::master::PlaceholderId>,
    pub paths: Vec<super::super::geom::Path>,
}

#[derive(Debug, Clone)]
pub struct SetConnectorOp {
    pub id: String,
    pub target: connector::CxnTarget,
    pub start: Option<connector::CxnBind>,
    pub end: Option<connector::CxnBind>,
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
    SetShapeGeom(SetShapeGeomOp),
    SetConnector(SetConnectorOp),
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
    InsertOle(super::super::ole::InsertOleOp),
    DeleteOle(super::super::ole::DeleteOleOp),
    InsertModel3d(super::super::model3d::InsertModel3dOp),
    ReplaceModel3d(super::super::model3d::ReplaceModel3dOp),
    DeleteModel3d(super::super::model3d::DeleteModel3dOp),
    SetModel3dSize(super::super::model3d::SetModel3dSizeOp),
    SetModel3dPos(super::super::model3d::SetModel3dPosOp),
    InsertMath(super::super::omml::InsertMathOp),
    SetMath(super::super::omml::SetMathOp),
    DeleteMath(super::super::omml::DeleteMathOp),
    SetMathSize(super::super::omml::SetMathSizeOp),
    SetMathPos(super::super::omml::SetMathPosOp),
    InsertZoom(super::super::zoom::InsertZoomOp),
    DeleteZoom(super::super::zoom::DeleteZoomOp),
    SetZoom(super::super::zoom::SetZoomOp),
    SetShowPr(super::super::show::SetShowPrOp),
    SetProtection(super::super::protect::SetProtectionOp),
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
    SetTableColWidths(SetTableColWidthsOp),
    SetTableTrHeight(SetTableTrHeightOp),
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

pub(super) const ANIM_HOST_KINDS: &[&str] = &[
    "shape",
    "cxn",
    "pic",
    "video",
    "audio",
    "table",
    "chart",
    "smartdiagram",
];

pub(super) type ShapePosHost = (
    Option<super::super::shape::ShapeTarget>,
    Option<super::super::master::PlaceholderId>,
    Option<connector::CxnTarget>,
);

pub(super) struct MediaClipFields {
    pub(super) st: Option<f64>,
    pub(super) end: Option<f64>,
    pub(super) bookmarks: Option<Vec<MediaBookmark>>,
    pub(super) tracks: Option<Vec<MediaTrackWrite>>,
    pub(super) track_display_loc: Option<String>,
}
