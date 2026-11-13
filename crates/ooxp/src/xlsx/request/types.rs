//! xlsx request：types。

use serde_json::Value;

use crate::chart::{ChartExtra, ChartType, NewSeries};

use crate::smartart::{ColorsKind, LayoutKind, NewPoint, Role, StyleKind};

use super::super::a1;
use super::super::chart::SheetChartId;
use super::super::connection;
use super::super::custom_view;
use super::super::extlink;
use super::super::form_control;
use super::super::ole;
use super::super::picture::{self, SheetPicId};
use super::super::shape::{DrawObjKind, SheetDrawId};
use super::super::slicer;
use super::super::smartart as xlsx_smartart;
use super::super::style;

/// 工作表页眉页脚六个槽位之一。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HfSlot {
    kind: HfKind,
    side: HfSide,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum HfKind {
    Odd,
    Even,
    First,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum HfSide {
    Header,
    Footer,
}

impl HfSlot {
    pub const ODD_HEADER: Self = Self {
        kind: HfKind::Odd,
        side: HfSide::Header,
    };
    pub const ODD_FOOTER: Self = Self {
        kind: HfKind::Odd,
        side: HfSide::Footer,
    };
    pub const EVEN_HEADER: Self = Self {
        kind: HfKind::Even,
        side: HfSide::Header,
    };
    pub const EVEN_FOOTER: Self = Self {
        kind: HfKind::Even,
        side: HfSide::Footer,
    };
    pub const FIRST_HEADER: Self = Self {
        kind: HfKind::First,
        side: HfSide::Header,
    };
    pub const FIRST_FOOTER: Self = Self {
        kind: HfKind::First,
        side: HfSide::Footer,
    };
    pub const ALL: [Self; 6] = [
        Self::ODD_HEADER,
        Self::ODD_FOOTER,
        Self::EVEN_HEADER,
        Self::EVEN_FOOTER,
        Self::FIRST_HEADER,
        Self::FIRST_FOOTER,
    ];

    pub fn parse(kind: &str, side: &str) -> Option<Self> {
        let kind = match kind {
            "odd" => HfKind::Odd,
            "even" => HfKind::Even,
            "first" => HfKind::First,
            _ => return None,
        };
        let side = match side {
            "header" => HfSide::Header,
            "footer" => HfSide::Footer,
            _ => return None,
        };
        Some(Self { kind, side })
    }

    pub fn id(self, sheet: &str) -> String {
        format!("sheet:{sheet}/hf:{}:{}", self.kind_str(), self.side_str())
    }

    pub fn kind_str(self) -> &'static str {
        match self.kind {
            HfKind::Odd => "odd",
            HfKind::Even => "even",
            HfKind::First => "first",
        }
    }

    pub fn side_str(self) -> &'static str {
        match self.side {
            HfSide::Header => "header",
            HfSide::Footer => "footer",
        }
    }

    pub fn xml_local(self) -> &'static str {
        match (self.kind, self.side) {
            (HfKind::Odd, HfSide::Header) => "oddHeader",
            (HfKind::Odd, HfSide::Footer) => "oddFooter",
            (HfKind::Even, HfSide::Header) => "evenHeader",
            (HfKind::Even, HfSide::Footer) => "evenFooter",
            (HfKind::First, HfSide::Header) => "firstHeader",
            (HfKind::First, HfSide::Footer) => "firstFooter",
        }
    }

    pub fn index(self) -> usize {
        Self::ALL.iter().position(|s| *s == self).unwrap()
    }

    /// 中、左、右区属性，插入时按这个顺序占空位。
    pub fn section_attrs(self) -> [&'static str; 3] {
        match (self.kind, self.side) {
            (HfKind::Odd, HfSide::Header) => ["cho", "lho", "rho"],
            (HfKind::Even, HfSide::Header) => ["che", "lhe", "rhe"],
            (HfKind::First, HfSide::Header) => ["chf", "lhf", "rhf"],
            (HfKind::Odd, HfSide::Footer) => ["cfo", "lfo", "rfo"],
            (HfKind::Even, HfSide::Footer) => ["cfe", "lfe", "rfe"],
            (HfKind::First, HfSide::Footer) => ["cff", "lff", "rff"],
        }
    }
}

#[derive(Debug, Clone)]
pub struct CellScope {
    pub sheet: String,
    pub range: String,
    pub max_cells: u32,
}

#[derive(Debug, Clone)]
pub struct InspectScope {
    pub cells: Option<CellScope>,
    pub pictures: bool,
    pub charts: bool,
    pub header_footer: bool,
    pub comments: bool,
    pub text_boxes: bool,
    pub shapes: bool,
    pub connectors: bool,
    pub smart_art: bool,
    pub hyperlinks: bool,
    pub sparklines: bool,
    pub ole_objects: bool,
    pub form_controls: bool,
    pub model3ds: bool,
    pub inks: bool,
    pub maths: bool,
    pub slicers: bool,
    pub protected_ranges: bool,
    pub connections: bool,
    pub external_links: bool,
    pub custom_views: bool,
    pub cell_styles: bool,
}

#[derive(Debug, Clone)]
pub enum SetCellValue {
    Number(serde_json::Number),
    Text(String),
    Bool(bool),
    Null,
}

#[derive(Debug, Clone)]
pub struct SetCellOp {
    pub id: String,
    pub value: Option<SetCellValue>,
    pub formula: Option<String>,
    pub shared: Option<bool>,
    pub shared_ref: Option<(a1::CellRef, a1::CellRef)>,
    pub array: Option<bool>,
    pub array_ref: Option<(a1::CellRef, a1::CellRef)>,
    pub data_table_ref: Option<(a1::CellRef, a1::CellRef)>,
}

/// 省略 / `null` / 有值。`null` 表示把该分量重置为默认。
#[derive(Debug, Clone)]
pub enum Patch<T> {
    Absent,
    Clear,
    Set(T),
}

impl<T> Patch<T> {
    pub(super) fn is_absent(&self) -> bool {
        matches!(self, Self::Absent)
    }

    pub(crate) fn is_present(&self) -> bool {
        !matches!(self, Self::Absent)
    }
}

#[derive(Debug, Clone)]
pub struct ColorSpec {
    pub rgb: Option<String>,
    pub theme: Option<u32>,
    pub indexed: Option<u32>,
    pub tint: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct FontPatch {
    pub name: Option<String>,
    pub sz: Option<f64>,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    /// `None` 省略；`Some(None)` 去掉下划线；`Some(Some(v))` 写入。
    pub underline: Option<Option<String>>,
    pub color: Option<ColorSpec>,
}

#[derive(Debug, Clone)]
pub struct FillPatch {
    pub pattern: Option<String>,
    pub fg: Option<ColorSpec>,
    pub bg: Option<ColorSpec>,
}

#[derive(Debug, Clone)]
pub struct BorderSidePatch {
    pub style: Option<String>,
    pub color: Option<ColorSpec>,
}

#[derive(Debug, Clone)]
pub struct BorderPatch {
    pub style: Option<String>,
    pub color: Option<ColorSpec>,
    pub left: Option<BorderSidePatch>,
    pub right: Option<BorderSidePatch>,
    pub top: Option<BorderSidePatch>,
    pub bottom: Option<BorderSidePatch>,
    pub diagonal: Option<BorderSidePatch>,
    pub diagonal_up: Option<bool>,
    pub diagonal_down: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct AlignPatch {
    pub horizontal: Option<String>,
    pub vertical: Option<String>,
    pub wrap: Option<bool>,
    pub indent: Option<u32>,
    pub rotation: Option<u32>,
    pub shrink: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct SetCellStyleOp {
    pub id: String,
    pub num_fmt: Patch<String>,
    pub font: Patch<FontPatch>,
    pub fill: Patch<FillPatch>,
    pub border: Patch<BorderPatch>,
    pub align: Patch<AlignPatch>,
}

#[derive(Debug, Clone)]
pub struct SetColWidthOp {
    pub sheet: String,
    pub min_col: u32,
    pub max_col: u32,
    pub width: Patch<f64>,
    pub best_fit: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct SetRowHeightOp {
    pub sheet: String,
    pub row: u32,
    pub to_row: u32,
    pub height: Patch<f64>,
}

#[derive(Debug, Clone)]
pub struct MergeRangeOp {
    pub sheet: String,
    pub start: a1::CellRef,
    pub end: a1::CellRef,
}

#[derive(Debug, Clone)]
pub struct UnmergeRangeOp {
    pub sheet: String,
    pub start: a1::CellRef,
    pub end: a1::CellRef,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DvType {
    None,
    Whole,
    Decimal,
    List,
    Date,
    Time,
    TextLength,
    Custom,
}

impl DvType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Whole => "whole",
            Self::Decimal => "decimal",
            Self::List => "list",
            Self::Date => "date",
            Self::Time => "time",
            Self::TextLength => "textLength",
            Self::Custom => "custom",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "none" => Self::None,
            "whole" => Self::Whole,
            "decimal" => Self::Decimal,
            "list" => Self::List,
            "date" => Self::Date,
            "time" => Self::Time,
            "textLength" => Self::TextLength,
            "custom" => Self::Custom,
            _ => return None,
        })
    }

    pub fn needs_formula1(self) -> bool {
        !matches!(self, Self::None)
    }

    pub fn allows_operator(self) -> bool {
        matches!(
            self,
            Self::Whole | Self::Decimal | Self::Date | Self::Time | Self::TextLength
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DvOperator {
    Between,
    NotBetween,
    Equal,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
}

impl DvOperator {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Between => "between",
            Self::NotBetween => "notBetween",
            Self::Equal => "equal",
            Self::NotEqual => "notEqual",
            Self::LessThan => "lessThan",
            Self::LessThanOrEqual => "lessThanOrEqual",
            Self::GreaterThan => "greaterThan",
            Self::GreaterThanOrEqual => "greaterThanOrEqual",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "between" => Self::Between,
            "notBetween" => Self::NotBetween,
            "equal" => Self::Equal,
            "notEqual" => Self::NotEqual,
            "lessThan" => Self::LessThan,
            "lessThanOrEqual" => Self::LessThanOrEqual,
            "greaterThan" => Self::GreaterThan,
            "greaterThanOrEqual" => Self::GreaterThanOrEqual,
            _ => return None,
        })
    }

    pub fn needs_formula2(self) -> bool {
        matches!(self, Self::Between | Self::NotBetween)
    }
}

#[derive(Debug, Clone)]
pub struct InsertDataValidationOp {
    pub sheet: String,
    pub sqref: Vec<(a1::CellRef, a1::CellRef)>,
    pub type_: DvType,
    pub operator: Option<DvOperator>,
    pub formula1: Option<String>,
    pub formula2: Option<String>,
    pub allow_blank: bool,
    pub dropdown: Option<bool>,
    pub show_input_message: bool,
    pub show_error_message: bool,
    pub error_style: Option<String>,
    pub error_title: Option<String>,
    pub error: Option<String>,
    pub prompt_title: Option<String>,
    pub prompt: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SetDataValidationOp {
    pub sheet: String,
    pub index: usize,
    pub sqref: Option<Vec<(a1::CellRef, a1::CellRef)>>,
    pub type_: Option<DvType>,
    pub operator: Patch<DvOperator>,
    pub formula1: Patch<String>,
    pub formula2: Patch<String>,
    pub allow_blank: Option<bool>,
    pub dropdown: Option<bool>,
    pub show_input_message: Option<bool>,
    pub show_error_message: Option<bool>,
    pub error_style: Patch<String>,
    pub error_title: Patch<String>,
    pub error: Patch<String>,
    pub prompt_title: Patch<String>,
    pub prompt: Patch<String>,
}

impl SetDataValidationOp {
    pub fn has_patch(&self) -> bool {
        self.sqref.is_some()
            || self.type_.is_some()
            || self.operator.is_present()
            || self.formula1.is_present()
            || self.formula2.is_present()
            || self.allow_blank.is_some()
            || self.dropdown.is_some()
            || self.show_input_message.is_some()
            || self.show_error_message.is_some()
            || self.error_style.is_present()
            || self.error_title.is_present()
            || self.error.is_present()
            || self.prompt_title.is_present()
            || self.prompt.is_present()
    }
}

#[derive(Debug, Clone)]
pub struct DeleteDataValidationOp {
    pub sheet: String,
    pub index: usize,
}

/// 条件格式差分格式，形状与单元格样式的 font/fill/border 相同。
#[derive(Debug, Clone)]
pub struct DxfPatch {
    pub font: Option<FontPatch>,
    pub fill: Option<FillPatch>,
    pub border: Option<BorderPatch>,
}

impl DxfPatch {
    pub fn is_empty(&self) -> bool {
        self.font.is_none() && self.fill.is_none() && self.border.is_none()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CfType {
    CellIs,
    Expression,
    ColorScale,
    DataBar,
    IconSet,
    Top10,
    AboveAverage,
    ContainsText,
    NotContainsText,
    BeginsWith,
    EndsWith,
}

impl CfType {
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "cellIs" => Self::CellIs,
            "expression" => Self::Expression,
            "colorScale" => Self::ColorScale,
            "dataBar" => Self::DataBar,
            "iconSet" => Self::IconSet,
            "top10" => Self::Top10,
            "aboveAverage" => Self::AboveAverage,
            "containsText" => Self::ContainsText,
            "notContainsText" => Self::NotContainsText,
            "beginsWith" => Self::BeginsWith,
            "endsWith" => Self::EndsWith,
            _ => return None,
        })
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::CellIs => "cellIs",
            Self::Expression => "expression",
            Self::ColorScale => "colorScale",
            Self::DataBar => "dataBar",
            Self::IconSet => "iconSet",
            Self::Top10 => "top10",
            Self::AboveAverage => "aboveAverage",
            Self::ContainsText => "containsText",
            Self::NotContainsText => "notContainsText",
            Self::BeginsWith => "beginsWith",
            Self::EndsWith => "endsWith",
        }
    }

    pub fn is_writable(s: &str) -> bool {
        Self::parse(s).is_some()
    }

    pub(crate) fn is_text(self) -> bool {
        matches!(
            self,
            Self::ContainsText | Self::NotContainsText | Self::BeginsWith | Self::EndsWith
        )
    }

    pub(crate) fn text_operator(self) -> Option<&'static str> {
        match self {
            Self::ContainsText => Some("containsText"),
            Self::NotContainsText => Some("notContains"),
            Self::BeginsWith => Some("beginsWith"),
            Self::EndsWith => Some("endsWith"),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CfOperator {
    LessThan,
    LessThanOrEqual,
    Equal,
    NotEqual,
    GreaterThanOrEqual,
    GreaterThan,
    Between,
    NotBetween,
}

impl CfOperator {
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "lessThan" => Self::LessThan,
            "lessThanOrEqual" => Self::LessThanOrEqual,
            "equal" => Self::Equal,
            "notEqual" => Self::NotEqual,
            "greaterThanOrEqual" => Self::GreaterThanOrEqual,
            "greaterThan" => Self::GreaterThan,
            "between" => Self::Between,
            "notBetween" => Self::NotBetween,
            _ => return None,
        })
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::LessThan => "lessThan",
            Self::LessThanOrEqual => "lessThanOrEqual",
            Self::Equal => "equal",
            Self::NotEqual => "notEqual",
            Self::GreaterThanOrEqual => "greaterThanOrEqual",
            Self::GreaterThan => "greaterThan",
            Self::Between => "between",
            Self::NotBetween => "notBetween",
        }
    }

    pub fn formula_count(self) -> usize {
        match self {
            Self::Between | Self::NotBetween => 2,
            _ => 1,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CfvoSpec {
    pub type_: String,
    pub val: Option<String>,
    pub gte: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct ColorScaleSpec {
    pub cfvo: Option<Vec<CfvoSpec>>,
    pub colors: Vec<ColorSpec>,
}

#[derive(Debug, Clone)]
pub struct DataBarSpec {
    pub cfvo: Option<Vec<CfvoSpec>>,
    pub color: Option<ColorSpec>,
    pub min_length: Option<u32>,
    pub max_length: Option<u32>,
    pub show_value: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct IconSetSpec {
    pub set: Option<String>,
    pub cfvo: Option<Vec<CfvoSpec>>,
    pub show_value: Option<bool>,
    pub reverse: Option<bool>,
    pub percent: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct InsertConditionalFormatOp {
    pub sheet: String,
    pub sqref: Vec<(a1::CellRef, a1::CellRef)>,
    pub type_: CfType,
    pub operator: Option<CfOperator>,
    pub formulas: Option<Vec<String>>,
    pub dxf: Option<DxfPatch>,
    pub color_scale: Option<ColorScaleSpec>,
    pub data_bar: Option<DataBarSpec>,
    pub icon_set: Option<IconSetSpec>,
    pub priority: Option<i32>,
    pub stop_if_true: bool,
    pub text: Option<String>,
    pub rank: Option<u32>,
    pub percent: Option<bool>,
    pub bottom: Option<bool>,
    pub above_average: Option<bool>,
    pub equal_average: Option<bool>,
    pub std_dev: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct SetConditionalFormatOp {
    pub sheet: String,
    pub index: usize,
    pub sqref: Option<Vec<(a1::CellRef, a1::CellRef)>>,
    pub type_: Option<CfType>,
    pub operator: Patch<CfOperator>,
    pub formulas: Patch<Vec<String>>,
    pub dxf: Patch<DxfPatch>,
    pub color_scale: Patch<ColorScaleSpec>,
    pub data_bar: Patch<DataBarSpec>,
    pub icon_set: Patch<IconSetSpec>,
    pub priority: Option<i32>,
    pub stop_if_true: Option<bool>,
    pub text: Patch<String>,
    pub rank: Patch<u32>,
    pub percent: Option<bool>,
    pub bottom: Option<bool>,
    pub above_average: Patch<bool>,
    pub equal_average: Option<bool>,
    pub std_dev: Patch<i32>,
}

impl SetConditionalFormatOp {
    pub fn has_patch(&self) -> bool {
        self.sqref.is_some()
            || self.type_.is_some()
            || self.operator.is_present()
            || self.formulas.is_present()
            || self.dxf.is_present()
            || self.color_scale.is_present()
            || self.data_bar.is_present()
            || self.icon_set.is_present()
            || self.priority.is_some()
            || self.stop_if_true.is_some()
            || self.text.is_present()
            || self.rank.is_present()
            || self.percent.is_some()
            || self.bottom.is_some()
            || self.above_average.is_present()
            || self.equal_average.is_some()
            || self.std_dev.is_present()
    }
}

#[derive(Debug, Clone)]
pub struct DeleteConditionalFormatOp {
    pub sheet: String,
    pub index: usize,
}

#[derive(Debug, Clone)]
pub struct InsertSheetTableOp {
    pub sheet: String,
    pub range: (a1::CellRef, a1::CellRef),
    pub name: Option<String>,
    pub columns: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct SetSheetTableOp {
    pub sheet: String,
    pub index: usize,
    pub range: Option<(a1::CellRef, a1::CellRef)>,
    pub name: Option<String>,
    pub columns: Option<Vec<String>>,
}

impl SetSheetTableOp {
    pub fn has_patch(&self) -> bool {
        self.range.is_some() || self.name.is_some() || self.columns.is_some()
    }
}

#[derive(Debug, Clone)]
pub struct DeleteSheetTableOp {
    pub sheet: String,
    pub index: usize,
}

#[derive(Debug, Clone)]
pub struct PivotSource {
    pub sheet: Option<String>,
    pub range: (a1::CellRef, a1::CellRef),
}

#[derive(Debug, Clone)]
pub struct InsertPivotTableOp {
    pub sheet: String,
    pub source: PivotSource,
    pub location: a1::CellRef,
    pub name: Option<String>,
    pub rows: Vec<String>,
    pub cols: Vec<String>,
    pub filters: Vec<String>,
    pub values: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SetPivotTableOp {
    pub sheet: String,
    pub index: usize,
    pub source: Option<PivotSource>,
    pub location: Option<a1::CellRef>,
    pub name: Option<String>,
    pub rows: Option<Vec<String>>,
    pub cols: Option<Vec<String>>,
    pub filters: Option<Vec<String>>,
    pub values: Option<Vec<String>>,
}

impl SetPivotTableOp {
    pub fn has_patch(&self) -> bool {
        self.source.is_some()
            || self.location.is_some()
            || self.name.is_some()
            || self.rows.is_some()
            || self.cols.is_some()
            || self.filters.is_some()
            || self.values.is_some()
    }
}

#[derive(Debug, Clone)]
pub struct DeletePivotTableOp {
    pub sheet: String,
    pub index: usize,
}

#[derive(Debug, Clone)]
pub struct InsertDefinedNameOp {
    pub name: String,
    pub formula: String,
    pub hidden: Option<bool>,
    pub local_sheet: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SetDefinedNameOp {
    pub index: usize,
    pub name: Option<String>,
    pub formula: Option<String>,
    pub hidden: Option<bool>,
    /// `None` 省略；`Some(None)` 改成簿级；`Some(Some(名))` 改成该表级。
    pub local_sheet: Option<Option<String>>,
}

impl SetDefinedNameOp {
    pub fn has_patch(&self) -> bool {
        self.name.is_some()
            || self.formula.is_some()
            || self.hidden.is_some()
            || self.local_sheet.is_some()
    }
}

#[derive(Debug, Clone)]
pub struct DeleteDefinedNameOp {
    pub index: usize,
}

#[derive(Debug, Clone)]
pub struct SetPrintSetupOp {
    pub sheet: String,
    pub left: Option<f64>,
    pub right: Option<f64>,
    pub top: Option<f64>,
    pub bottom: Option<f64>,
    pub header: Option<f64>,
    pub footer: Option<f64>,
    pub paper_size: Option<u32>,
    pub orientation: Option<String>,
    pub scale: Option<u32>,
    pub fit_to_page: Option<bool>,
    pub fit_to_width: Option<u32>,
    pub fit_to_height: Option<u32>,
    pub grid_lines: Option<bool>,
    pub headings: Option<bool>,
    pub h_center: Option<bool>,
    pub v_center: Option<bool>,
    /// `None` 省略；`Some(None)` 删打印区域；`Some(Some(A1))` 写入。
    pub print_area: Option<Option<String>>,
    pub row_breaks: Option<Vec<u32>>,
    pub col_breaks: Option<Vec<u32>>,
}

impl SetPrintSetupOp {
    pub(super) fn has_patch(&self) -> bool {
        self.left.is_some()
            || self.right.is_some()
            || self.top.is_some()
            || self.bottom.is_some()
            || self.header.is_some()
            || self.footer.is_some()
            || self.paper_size.is_some()
            || self.orientation.is_some()
            || self.scale.is_some()
            || self.fit_to_page.is_some()
            || self.fit_to_width.is_some()
            || self.fit_to_height.is_some()
            || self.grid_lines.is_some()
            || self.headings.is_some()
            || self.h_center.is_some()
            || self.v_center.is_some()
            || self.print_area.is_some()
            || self.row_breaks.is_some()
            || self.col_breaks.is_some()
    }
}

#[derive(Debug, Clone)]
pub struct SetSheetViewOp {
    pub sheet: String,
    pub zoom: Option<u32>,
    pub frozen_rows: Option<u32>,
    pub frozen_cols: Option<u32>,
    pub split_rows: Option<u32>,
    pub split_cols: Option<u32>,
    pub active_cell: Option<String>,
}

impl SetSheetViewOp {
    pub(super) fn has_patch(&self) -> bool {
        self.zoom.is_some()
            || self.frozen_rows.is_some()
            || self.frozen_cols.is_some()
            || self.split_rows.is_some()
            || self.split_cols.is_some()
            || self.active_cell.is_some()
    }

    pub(crate) fn has_freeze(&self) -> bool {
        self.frozen_rows.is_some() || self.frozen_cols.is_some()
    }

    pub(crate) fn has_split(&self) -> bool {
        self.split_rows.is_some() || self.split_cols.is_some()
    }
}

#[derive(Debug, Clone)]
pub struct SetAutoFilterOp {
    pub sheet: String,
    /// `None` 删除工作表级筛选；`Some` 写入规范化前的原文，apply 再解析。
    pub refer: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SortConditionOp {
    pub refer: String,
    pub descending: bool,
    /// `None` 表示按值（不写 `@sortBy`）。
    pub sort_by: Option<String>,
    pub custom_list: Option<String>,
    pub dxf_id: Option<u32>,
    pub color: Option<ColorSpec>,
    pub icon_set: Option<String>,
    pub icon_id: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct SetSortStateOp {
    pub sheet: String,
    /// `None` 删除工作表级排序；`Some` 为已规范化的 A1。
    pub refer: Option<String>,
    pub case_sensitive: bool,
    pub column_sort: bool,
    /// `None` 不写属性。只存 `pinYin` / `stroke`。
    pub sort_method: Option<String>,
    pub columns: Vec<SortConditionOp>,
}

#[derive(Debug, Clone)]
pub struct GroupRowsOp {
    pub sheet: String,
    pub row: u32,
    pub to_row: u32,
}

#[derive(Debug, Clone)]
pub struct UngroupRowsOp {
    pub sheet: String,
    pub row: u32,
    pub to_row: u32,
}

#[derive(Debug, Clone)]
pub struct GroupColsOp {
    pub sheet: String,
    pub min_col: u32,
    pub max_col: u32,
}

#[derive(Debug, Clone)]
pub struct UngroupColsOp {
    pub sheet: String,
    pub min_col: u32,
    pub max_col: u32,
}

#[derive(Debug, Clone)]
pub struct SetOutlineOp {
    pub sheet: String,
    pub summary_below: Option<bool>,
    pub summary_right: Option<bool>,
    pub collapse_rows: Option<(u32, u32)>,
    pub collapse_cols: Option<(u32, u32)>,
    pub collapsed: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct SetRowHiddenOp {
    pub sheet: String,
    pub row: u32,
    pub to_row: u32,
    pub hidden: bool,
}

#[derive(Debug, Clone)]
pub struct SetColHiddenOp {
    pub sheet: String,
    pub min_col: u32,
    pub max_col: u32,
    pub hidden: bool,
}

#[derive(Debug, Clone)]
pub struct InsertCommentOp {
    pub sheet: String,
    pub a1: String,
    pub text: String,
    pub author: String,
    pub date: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ReplyCommentOp {
    pub id: String,
    pub text: String,
    pub author: String,
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
    pub date: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SetCommentDoneOp {
    pub id: String,
    pub done: bool,
}

#[derive(Debug, Clone)]
pub struct InsertHyperlinkOp {
    pub sheet: String,
    pub refer: String,
    pub start: a1::CellRef,
    pub end: a1::CellRef,
    pub url: Option<String>,
    pub location: Option<String>,
    pub tooltip: Option<String>,
    pub display: Option<String>,
    pub drawing: Option<DrawingHlinkHost>,
}

#[derive(Debug, Clone)]
pub enum DrawingHlinkHost {
    Picture(picture::SheetPicId),
    TextBox(SheetDrawId),
    Shape(SheetDrawId),
    SmartArt(SheetDrawId),
}

impl DrawingHlinkHost {
    pub fn sheet(&self) -> &str {
        match self {
            Self::Picture(p) => &p.sheet,
            Self::TextBox(d) | Self::Shape(d) | Self::SmartArt(d) => &d.sheet,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SetHyperlinkOp {
    pub id: String,
    pub sheet: String,
    pub index: usize,
    pub url: Patch<String>,
    pub location: Patch<String>,
    pub tooltip: Patch<String>,
    pub display: Patch<String>,
    pub refer: Option<(String, a1::CellRef, a1::CellRef)>,
    pub drawing: Option<DrawingHlinkHost>,
}

#[derive(Debug, Clone)]
pub struct DeleteHyperlinkOp {
    pub id: String,
    pub sheet: String,
    pub index: usize,
    pub drawing: Option<DrawingHlinkHost>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SparklineType {
    Line,
    Column,
    Stacked,
}

impl SparklineType {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Line => "line",
            Self::Column => "column",
            Self::Stacked => "stacked",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SparkAxisType {
    Individual,
    Group,
    Custom,
}

impl SparkAxisType {
    pub(super) fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "individual" => Self::Individual,
            "group" => Self::Group,
            "custom" => Self::Custom,
            _ => return None,
        })
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Individual => "individual",
            Self::Group => "group",
            Self::Custom => "custom",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SparkEmpty {
    Gap,
    Span,
    Zero,
}

impl SparkEmpty {
    pub(super) fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "gap" => Self::Gap,
            "span" => Self::Span,
            "zero" => Self::Zero,
            _ => return None,
        })
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Gap => "gap",
            Self::Span => "span",
            Self::Zero => "zero",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct SparkColors {
    pub series: Option<ColorSpec>,
    pub negative: Option<ColorSpec>,
    pub axis: Option<ColorSpec>,
    pub markers: Option<ColorSpec>,
    pub first: Option<ColorSpec>,
    pub last: Option<ColorSpec>,
    pub high: Option<ColorSpec>,
    pub low: Option<ColorSpec>,
}

#[derive(Debug, Clone)]
pub struct InsertSparklineOp {
    pub sheet: String,
    pub refer: String,
    pub host: a1::CellRef,
    pub formula: String,
    pub kind: SparklineType,
    pub display_empty: SparkEmpty,
    pub markers: bool,
    pub high: bool,
    pub low: bool,
    pub first: bool,
    pub last: bool,
    pub negative: bool,
    pub display_x_axis: bool,
    pub line_weight: Option<f64>,
    pub min_axis: Option<SparkAxisType>,
    pub max_axis: Option<SparkAxisType>,
    pub manual_min: Option<f64>,
    pub manual_max: Option<f64>,
    pub colors: SparkColors,
}

#[derive(Debug, Clone)]
pub struct SetSparklineOp {
    pub id: String,
    pub sheet: String,
    pub index: usize,
    pub kind: Option<SparklineType>,
    pub refer: Option<(String, a1::CellRef)>,
    pub formula: Patch<String>,
    pub display_empty: Option<SparkEmpty>,
    pub markers: Option<bool>,
    pub high: Option<bool>,
    pub low: Option<bool>,
    pub first: Option<bool>,
    pub last: Option<bool>,
    pub negative: Option<bool>,
    pub display_x_axis: Option<bool>,
    pub line_weight: Patch<f64>,
    pub min_axis: Option<SparkAxisType>,
    pub max_axis: Option<SparkAxisType>,
    pub manual_min: Patch<f64>,
    pub manual_max: Patch<f64>,
    pub colors: Option<SparkColors>,
}

#[derive(Debug, Clone)]
pub struct DeleteSparklineOp {
    pub id: String,
    pub sheet: String,
    pub index: usize,
}

#[derive(Debug, Clone)]
pub struct SetSheetProtectionOp {
    pub sheet: String,
    pub enforced: Option<bool>,
    pub objects: Option<bool>,
    pub scenarios: Option<bool>,
    pub format_cells: Option<bool>,
    pub format_columns: Option<bool>,
    pub format_rows: Option<bool>,
    pub insert_columns: Option<bool>,
    pub insert_rows: Option<bool>,
    pub insert_hyperlinks: Option<bool>,
    pub delete_columns: Option<bool>,
    pub delete_rows: Option<bool>,
    pub select_locked_cells: Option<bool>,
    pub sort: Option<bool>,
    pub auto_filter: Option<bool>,
    pub pivot_tables: Option<bool>,
    pub select_unlocked_cells: Option<bool>,
    pub password: Option<String>,
}

impl SetSheetProtectionOp {
    pub(crate) fn has_flag(&self) -> bool {
        self.objects.is_some()
            || self.scenarios.is_some()
            || self.format_cells.is_some()
            || self.format_columns.is_some()
            || self.format_rows.is_some()
            || self.insert_columns.is_some()
            || self.insert_rows.is_some()
            || self.insert_hyperlinks.is_some()
            || self.delete_columns.is_some()
            || self.delete_rows.is_some()
            || self.select_locked_cells.is_some()
            || self.sort.is_some()
            || self.auto_filter.is_some()
            || self.pivot_tables.is_some()
            || self.select_unlocked_cells.is_some()
    }
}

#[derive(Debug, Clone)]
pub struct SetWorkbookProtectionOp {
    pub lock_structure: Option<bool>,
    pub lock_windows: Option<bool>,
    pub lock_revision: Option<bool>,
    pub password: Option<String>,
}

#[derive(Debug, Clone)]
pub enum SetProtectionOp {
    Sheet(SetSheetProtectionOp),
    Workbook(SetWorkbookProtectionOp),
}

#[derive(Debug, Clone)]
pub struct InsertProtectedRangeOp {
    pub sheet: String,
    pub name: String,
    pub sqref: Vec<(a1::CellRef, a1::CellRef)>,
    pub password: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SetProtectedRangeOp {
    pub sheet: String,
    pub index: usize,
    pub name: Option<String>,
    pub sqref: Option<Vec<(a1::CellRef, a1::CellRef)>>,
    pub password: Option<String>,
}

impl SetProtectedRangeOp {
    pub(super) fn has_patch(&self) -> bool {
        self.name.is_some() || self.sqref.is_some() || self.password.is_some()
    }
}

#[derive(Debug, Clone)]
pub struct DeleteProtectedRangeOp {
    pub sheet: String,
    pub index: usize,
    pub password: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SetHeaderFooterTextOp {
    pub sheet: String,
    pub slot: HfSlot,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct ReplacePictureOp {
    pub id: String,
    pub target: SheetPicId,
    pub file: String,
}

#[derive(Debug, Clone, Copy)]
pub struct RelBox {
    pub from_x: f64,
    pub from_y: f64,
    pub to_x: f64,
    pub to_y: f64,
}

#[derive(Debug, Clone)]
pub struct InsertPictureOp {
    pub sheet: String,
    pub hf: Option<HfSlot>,
    pub file: String,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
    pub from: a1::CellRef,
    pub to: Option<a1::CellRef>,
    pub anchor: picture::AnchorKind,
    pub from_off_x: i64,
    pub from_off_y: i64,
    pub to_off_x: i64,
    pub to_off_y: i64,
    pub rel: Option<RelBox>,
}

#[derive(Debug, Clone)]
pub struct DeletePictureOp {
    pub id: String,
    pub target: SheetPicId,
}

#[derive(Debug, Clone)]
pub struct SetPictureSizeOp {
    pub id: String,
    pub target: SheetPicId,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
    pub to: Option<a1::CellRef>,
    pub to_off_x: Option<i64>,
    pub to_off_y: Option<i64>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct RelBoxPatch {
    pub from_x: Option<f64>,
    pub from_y: Option<f64>,
    pub to_x: Option<f64>,
    pub to_y: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct SetPictureAnchorOp {
    pub id: String,
    pub target: SheetPicId,
    pub anchor: Option<picture::AnchorKind>,
    pub from: Option<a1::CellRef>,
    pub to: Option<a1::CellRef>,
    pub from_off_x: Option<i64>,
    pub from_off_y: Option<i64>,
    pub to_off_x: Option<i64>,
    pub to_off_y: Option<i64>,
    pub pos_x: Option<i64>,
    pub pos_y: Option<i64>,
    pub edit_as: Option<picture::EditAs>,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
    pub rel: Option<RelBoxPatch>,
}

#[derive(Debug, Clone)]
pub struct InsertChartOp {
    pub sheet: String,
    pub chart_type: ChartType,
    pub of_pie_type: Option<String>,
    pub extra: ChartExtra,
    pub title: Option<String>,
    pub series: Vec<NewSeries>,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
    pub from: a1::CellRef,
    pub to: Option<a1::CellRef>,
    pub anchor: picture::AnchorKind,
    pub from_off_x: i64,
    pub from_off_y: i64,
    pub to_off_x: i64,
    pub to_off_y: i64,
    pub data_from: a1::CellRef,
}

#[derive(Debug, Clone)]
pub struct InsertChartSheetOp {
    pub name: String,
    pub data_sheet: Option<String>,
    pub chart_type: ChartType,
    pub of_pie_type: Option<String>,
    pub extra: ChartExtra,
    pub title: Option<String>,
    pub series: Vec<NewSeries>,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
    pub data_from: a1::CellRef,
}

#[derive(Debug, Clone)]
pub struct DeleteChartSheetOp {
    pub sheet: String,
}

#[derive(Debug, Clone)]
pub struct InsertSheetOp {
    pub name: String,
    pub after: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DeleteSheetOp {
    pub sheet: String,
}

#[derive(Debug, Clone)]
pub struct RenameSheetOp {
    pub sheet: String,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct MoveSheetOp {
    pub sheet: String,
    pub after: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SetSheetHiddenOp {
    pub sheet: String,
    pub hidden: bool,
}

#[derive(Debug, Clone)]
pub struct SetSheetBackgroundOp {
    pub sheet: String,
    pub file: String,
}

#[derive(Debug, Clone)]
pub struct DeleteSheetBackgroundOp {
    pub sheet: String,
}

#[derive(Debug, Clone)]
pub struct DeleteChartOp {
    pub id: String,
    pub target: SheetChartId,
}

#[derive(Debug, Clone)]
pub struct SetChartTitleOp {
    pub id: String,
    pub target: SheetChartId,
    pub title: String,
}

#[derive(Debug, Clone)]
pub struct SetChartSeriesOp {
    pub id: String,
    pub target: SheetChartId,
    pub index: usize,
    pub name: Option<String>,
    pub categories: Option<Vec<Value>>,
    pub values: Option<Vec<Value>>,
    pub sizes: Option<Vec<Value>>,
}

#[derive(Debug, Clone)]
pub struct AddChartSeriesOp {
    pub id: String,
    pub target: SheetChartId,
    pub name: Option<String>,
    pub categories: Option<Vec<Value>>,
    pub values: Vec<Value>,
    pub sizes: Option<Vec<Value>>,
    pub series_type: Option<ChartType>,
}

#[derive(Debug, Clone)]
pub struct DeleteChartSeriesOp {
    pub id: String,
    pub target: SheetChartId,
    pub index: usize,
}

#[derive(Debug, Clone)]
pub struct SetChartTypeOp {
    pub id: String,
    pub target: SheetChartId,
    pub chart_type: ChartType,
    pub of_pie_type: Option<String>,
    pub extra: ChartExtra,
}

#[derive(Debug, Clone)]
pub struct SetChartAxisTitleOp {
    pub id: String,
    pub target: SheetChartId,
    pub axis: String,
    pub title: String,
}

#[derive(Debug, Clone)]
pub struct SetChartLegendOp {
    pub id: String,
    pub target: SheetChartId,
    pub show: bool,
    pub pos: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SetChartDataLabelsOp {
    pub id: String,
    pub target: SheetChartId,
    pub show: bool,
}

#[derive(Debug, Clone)]
pub struct SetChartSizeOp {
    pub id: String,
    pub target: SheetChartId,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
    pub to: Option<a1::CellRef>,
    pub to_off_x: Option<i64>,
    pub to_off_y: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct SetChartAnchorOp {
    pub id: String,
    pub target: SheetChartId,
    pub anchor: Option<picture::AnchorKind>,
    pub from: Option<a1::CellRef>,
    pub to: Option<a1::CellRef>,
    pub from_off_x: Option<i64>,
    pub from_off_y: Option<i64>,
    pub to_off_x: Option<i64>,
    pub to_off_y: Option<i64>,
    pub pos_x: Option<i64>,
    pub pos_y: Option<i64>,
    pub edit_as: Option<picture::EditAs>,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct InsertTrendlineOp {
    pub id: String,
    pub target: SheetChartId,
    pub index: usize,
    pub spec: crate::chart::series_decor::TrendlineWrite,
}

#[derive(Debug, Clone)]
pub struct SetTrendlineOp {
    pub id: String,
    pub target: SheetChartId,
    pub index: usize,
    pub tl_index: usize,
    pub spec: crate::chart::series_decor::TrendlineWrite,
}

#[derive(Debug, Clone)]
pub struct DeleteTrendlineOp {
    pub id: String,
    pub target: SheetChartId,
    pub index: usize,
    pub tl_index: usize,
}

#[derive(Debug, Clone)]
pub struct InsertErrorBarsOp {
    pub id: String,
    pub target: SheetChartId,
    pub index: usize,
    pub spec: crate::chart::series_decor::ErrorBarsWrite,
}

#[derive(Debug, Clone)]
pub struct SetErrorBarsOp {
    pub id: String,
    pub target: SheetChartId,
    pub index: usize,
    pub eb_index: usize,
    pub spec: crate::chart::series_decor::ErrorBarsWrite,
}

#[derive(Debug, Clone)]
pub struct DeleteErrorBarsOp {
    pub id: String,
    pub target: SheetChartId,
    pub index: usize,
    pub eb_index: usize,
}

#[derive(Debug, Clone)]
pub struct SetChartSeriesAxisOp {
    pub id: String,
    pub target: SheetChartId,
    pub index: usize,
    pub secondary: bool,
}

#[derive(Debug, Clone)]
pub struct InsertTextBoxOp {
    pub sheet: String,
    pub text: Option<String>,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
    pub from: a1::CellRef,
    pub to: Option<a1::CellRef>,
    pub anchor: picture::AnchorKind,
    pub from_off_x: i64,
    pub from_off_y: i64,
    pub to_off_x: i64,
    pub to_off_y: i64,
    pub format: Option<crate::drawing_text::TextFormat>,
    pub rel: Option<RelBox>,
}

#[derive(Debug, Clone)]
pub struct InsertShapeOp {
    pub sheet: String,
    pub text: Option<String>,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
    pub from: a1::CellRef,
    pub to: Option<a1::CellRef>,
    pub anchor: picture::AnchorKind,
    pub from_off_x: i64,
    pub from_off_y: i64,
    pub to_off_x: i64,
    pub to_off_y: i64,
    pub prst: String,
    pub format: Option<crate::drawing_text::TextFormat>,
    pub rel: Option<RelBox>,
}

#[derive(Debug, Clone)]
pub struct InsertConnectorOp {
    pub sheet: String,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
    pub from: a1::CellRef,
    pub to: Option<a1::CellRef>,
    pub anchor: picture::AnchorKind,
    pub from_off_x: i64,
    pub from_off_y: i64,
    pub to_off_x: i64,
    pub to_off_y: i64,
    pub prst: String,
    pub rel: Option<RelBox>,
}

#[derive(Debug, Clone)]
pub struct DeleteTextBoxOp {
    pub id: String,
    pub target: SheetDrawId,
}

#[derive(Debug, Clone)]
pub struct DeleteShapeOp {
    pub id: String,
    pub target: SheetDrawId,
}

#[derive(Debug, Clone)]
pub struct DeleteConnectorOp {
    pub id: String,
    pub target: SheetDrawId,
}

#[derive(Debug, Clone)]
pub struct GroupShapesOp {
    pub sheet: String,
    pub ids: Vec<String>,
    pub members: Vec<(SheetDrawId, DrawObjKind)>,
}

#[derive(Debug, Clone)]
pub struct UngroupOp {
    pub id: String,
    pub target: SheetDrawId,
    pub kind: DrawObjKind,
}

#[derive(Debug, Clone)]
pub struct SetTextBoxSizeOp {
    pub id: String,
    pub target: SheetDrawId,
    pub kind: DrawObjKind,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
    pub to: Option<a1::CellRef>,
    pub to_off_x: Option<i64>,
    pub to_off_y: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct SetTextBoxAnchorOp {
    pub id: String,
    pub target: SheetDrawId,
    pub kind: DrawObjKind,
    pub anchor: Option<picture::AnchorKind>,
    pub from: Option<a1::CellRef>,
    pub to: Option<a1::CellRef>,
    pub from_off_x: Option<i64>,
    pub from_off_y: Option<i64>,
    pub to_off_x: Option<i64>,
    pub to_off_y: Option<i64>,
    pub pos_x: Option<i64>,
    pub pos_y: Option<i64>,
    pub edit_as: Option<picture::EditAs>,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
    pub rel: Option<RelBoxPatch>,
}

#[derive(Debug, Clone)]
pub struct SetShapeTextOp {
    pub id: String,
    pub target: SheetDrawId,
    pub kind: DrawObjKind,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct SetShapeRunsOp {
    pub id: String,
    pub target: SheetDrawId,
    pub kind: DrawObjKind,
    pub runs: Vec<crate::shape_runs::ShapeRunPiece>,
}

#[derive(Debug, Clone)]
pub struct SetShapeFormatOp {
    pub id: String,
    pub target: SheetDrawId,
    pub kind: DrawObjKind,
    pub format: crate::drawing_text::TextFormat,
}

#[derive(Debug, Clone)]
pub struct InsertSmartArtOp {
    pub sheet: String,
    pub layout: LayoutKind,
    pub points: Option<Vec<NewPoint>>,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
    pub from: a1::CellRef,
    pub to: Option<a1::CellRef>,
    pub anchor: picture::AnchorKind,
    pub from_off_x: i64,
    pub from_off_y: i64,
    pub to_off_x: i64,
    pub to_off_y: i64,
}

#[derive(Debug, Clone)]
pub struct DeleteSmartArtOp {
    pub id: String,
    pub target: SheetDrawId,
}

#[derive(Debug, Clone)]
pub struct SetSmartArtTextOp {
    pub id: String,
    pub target: xlsx_smartart::PointTarget,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct AddSmartArtPointOp {
    pub id: String,
    pub parent: xlsx_smartart::AddParent,
    pub text: String,
    pub role: Role,
}

#[derive(Debug, Clone)]
pub struct DeleteSmartArtPointOp {
    pub id: String,
    pub target: xlsx_smartart::PointTarget,
}

#[derive(Debug, Clone)]
pub struct SetSmartArtSizeOp {
    pub id: String,
    pub target: SheetDrawId,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
    pub to: Option<a1::CellRef>,
    pub to_off_x: Option<i64>,
    pub to_off_y: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct SetSmartArtAnchorOp {
    pub id: String,
    pub target: SheetDrawId,
    pub anchor: Option<picture::AnchorKind>,
    pub from: Option<a1::CellRef>,
    pub to: Option<a1::CellRef>,
    pub from_off_x: Option<i64>,
    pub from_off_y: Option<i64>,
    pub to_off_x: Option<i64>,
    pub to_off_y: Option<i64>,
    pub pos_x: Option<i64>,
    pub pos_y: Option<i64>,
    pub edit_as: Option<picture::EditAs>,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct SetSmartArtColorsOp {
    pub id: String,
    pub target: SheetDrawId,
    pub colors: ColorsKind,
}

#[derive(Debug, Clone)]
pub struct SetSmartArtStyleOp {
    pub id: String,
    pub target: SheetDrawId,
    pub style: StyleKind,
}

#[derive(Debug, Clone)]
pub struct SetSmartArtLayoutOp {
    pub id: String,
    pub target: SheetDrawId,
    pub layout: LayoutKind,
}

#[derive(Debug, Clone)]
pub enum XlsxOp {
    SetCell(SetCellOp),
    SetCellStyle(Box<SetCellStyleOp>),
    SetHeaderFooterText(SetHeaderFooterTextOp),
    ReplacePicture(ReplacePictureOp),
    InsertPicture(InsertPictureOp),
    DeletePicture(DeletePictureOp),
    SetPictureSize(SetPictureSizeOp),
    SetPictureAnchor(SetPictureAnchorOp),
    InsertChart(InsertChartOp),
    InsertChartSheet(InsertChartSheetOp),
    DeleteChartSheet(DeleteChartSheetOp),
    InsertSheet(InsertSheetOp),
    DeleteSheet(DeleteSheetOp),
    RenameSheet(RenameSheetOp),
    MoveSheet(MoveSheetOp),
    SetSheetHidden(SetSheetHiddenOp),
    SetSheetBackground(SetSheetBackgroundOp),
    DeleteSheetBackground(DeleteSheetBackgroundOp),
    SetColWidth(SetColWidthOp),
    SetRowHeight(SetRowHeightOp),
    MergeRange(MergeRangeOp),
    UnmergeRange(UnmergeRangeOp),
    InsertDataValidation(InsertDataValidationOp),
    SetDataValidation(SetDataValidationOp),
    DeleteDataValidation(DeleteDataValidationOp),
    InsertConditionalFormat(InsertConditionalFormatOp),
    SetConditionalFormat(SetConditionalFormatOp),
    DeleteConditionalFormat(DeleteConditionalFormatOp),
    InsertSheetTable(InsertSheetTableOp),
    SetSheetTable(SetSheetTableOp),
    DeleteSheetTable(DeleteSheetTableOp),
    InsertPivotTable(InsertPivotTableOp),
    SetPivotTable(SetPivotTableOp),
    DeletePivotTable(DeletePivotTableOp),
    InsertDefinedName(InsertDefinedNameOp),
    SetDefinedName(SetDefinedNameOp),
    DeleteDefinedName(DeleteDefinedNameOp),
    SetPrintSetup(SetPrintSetupOp),
    SetSheetView(SetSheetViewOp),
    SetAutoFilter(SetAutoFilterOp),
    SetSortState(SetSortStateOp),
    InsertCustomView(custom_view::InsertCustomViewOp),
    DeleteCustomView(custom_view::DeleteCustomViewOp),
    ShowCustomView(custom_view::ShowCustomViewOp),
    InsertCellStyle(style::InsertCellStyleOp),
    DeleteCellStyle(style::DeleteCellStyleOp),
    SetCellNamedStyle(style::SetCellNamedStyleOp),
    GroupRows(GroupRowsOp),
    UngroupRows(UngroupRowsOp),
    GroupCols(GroupColsOp),
    UngroupCols(UngroupColsOp),
    SetOutline(SetOutlineOp),
    SetRowHidden(SetRowHiddenOp),
    SetColHidden(SetColHiddenOp),
    InsertComment(InsertCommentOp),
    ReplyComment(ReplyCommentOp),
    DeleteComment(DeleteCommentOp),
    SetCommentText(SetCommentTextOp),
    SetCommentMeta(SetCommentMetaOp),
    SetCommentDone(SetCommentDoneOp),
    InsertHyperlink(InsertHyperlinkOp),
    SetHyperlink(SetHyperlinkOp),
    DeleteHyperlink(DeleteHyperlinkOp),
    InsertSparkline(InsertSparklineOp),
    SetSparkline(SetSparklineOp),
    DeleteSparkline(DeleteSparklineOp),
    SetProtection(SetProtectionOp),
    InsertProtectedRange(InsertProtectedRangeOp),
    SetProtectedRange(SetProtectedRangeOp),
    DeleteProtectedRange(DeleteProtectedRangeOp),
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
    SetChartAnchor(SetChartAnchorOp),
    InsertTrendline(InsertTrendlineOp),
    SetTrendline(SetTrendlineOp),
    DeleteTrendline(DeleteTrendlineOp),
    InsertErrorBars(InsertErrorBarsOp),
    SetErrorBars(SetErrorBarsOp),
    DeleteErrorBars(DeleteErrorBarsOp),
    SetChartSeriesAxis(SetChartSeriesAxisOp),
    InsertTextBox(InsertTextBoxOp),
    InsertShape(InsertShapeOp),
    InsertConnector(InsertConnectorOp),
    DeleteTextBox(DeleteTextBoxOp),
    DeleteShape(DeleteShapeOp),
    DeleteConnector(DeleteConnectorOp),
    GroupShapes(GroupShapesOp),
    Ungroup(UngroupOp),
    InsertOle(ole::InsertOleOp),
    DeleteOle(ole::DeleteOleOp),
    SetOleSize(ole::SetOleSizeOp),
    InsertModel3d(super::super::model3d::InsertModel3dOp),
    ReplaceModel3d(super::super::model3d::ReplaceModel3dOp),
    DeleteModel3d(super::super::model3d::DeleteModel3dOp),
    SetModel3dSize(super::super::model3d::SetModel3dSizeOp),
    SetModel3dAnchor(super::super::model3d::SetModel3dAnchorOp),
    SetModel3dCamera(super::super::model3d::SetModel3dCameraOp),
    InsertInk(super::super::ink::InsertInkOp),
    ReplaceInk(super::super::ink::ReplaceInkOp),
    DeleteInk(super::super::ink::DeleteInkOp),
    SetInkSize(super::super::ink::SetInkSizeOp),
    SetInkAnchor(super::super::ink::SetInkAnchorOp),
    InsertMath(super::super::omml::InsertMathOp),
    SetMath(super::super::omml::SetMathOp),
    DeleteMath(super::super::omml::DeleteMathOp),
    SetMathSize(super::super::omml::SetMathSizeOp),
    SetMathAnchor(super::super::omml::SetMathAnchorOp),
    InsertSlicer(slicer::InsertSlicerOp),
    DeleteSlicer(slicer::DeleteSlicerOp),
    SetSlicerSize(slicer::SetSlicerSizeOp),
    SetSlicerAnchor(slicer::SetSlicerAnchorOp),
    SetSlicerView(slicer::SetSlicerViewOp),
    SetSlicerFilter(slicer::SetSlicerFilterOp),
    SetThemeColors(super::super::theme::SetThemeColorsOp),
    SetThemeFonts(super::super::theme::SetThemeFontsOp),
    SetThemeFmt(super::super::theme::SetThemeFmtOp),
    SetThemeExtraSchemes(super::super::theme::SetThemeExtraSchemesOp),
    SetThemeObjectDefaults(super::super::theme::SetThemeObjectDefaultsOp),
    SetThemeOverride(super::super::theme::SetThemeOverrideOp),
    InsertConnection(connection::InsertConnectionOp),
    DeleteConnection(connection::DeleteConnectionOp),
    InsertExternalLink(extlink::InsertExternalLinkOp),
    DeleteExternalLink(extlink::DeleteExternalLinkOp),
    SetExternalLink(extlink::SetExternalLinkOp),
    SetTextBoxSize(SetTextBoxSizeOp),
    SetTextBoxAnchor(SetTextBoxAnchorOp),
    SetShapeText(SetShapeTextOp),
    SetShapeRuns(SetShapeRunsOp),
    SetShapeFormat(SetShapeFormatOp),
    InsertSmartArt(InsertSmartArtOp),
    DeleteSmartArt(DeleteSmartArtOp),
    SetSmartArtText(SetSmartArtTextOp),
    AddSmartArtPoint(AddSmartArtPointOp),
    DeleteSmartArtPoint(DeleteSmartArtPointOp),
    SetSmartArtSize(SetSmartArtSizeOp),
    SetSmartArtAnchor(SetSmartArtAnchorOp),
    SetSmartArtColors(SetSmartArtColorsOp),
    SetSmartArtStyle(SetSmartArtStyleOp),
    SetSmartArtLayout(SetSmartArtLayoutOp),
    SetFormControl(form_control::SetFormControlOp),
    SetProps(crate::props::SetPropsOp),
    SetCustomProp(crate::props::SetCustomPropOp),
    DeleteCustomProp(crate::props::DeleteCustomPropOp),
}

pub(super) const PATTERN_TYPES: &[&str] = &[
    "none",
    "solid",
    "mediumGray",
    "darkGray",
    "lightGray",
    "darkHorizontal",
    "darkVertical",
    "darkDown",
    "darkUp",
    "darkGrid",
    "darkTrellis",
    "lightHorizontal",
    "lightVertical",
    "lightDown",
    "lightUp",
    "lightGrid",
    "lightTrellis",
    "gray125",
    "gray0625",
];

pub(super) const BORDER_STYLES: &[&str] = &[
    "none",
    "thin",
    "medium",
    "dashed",
    "dotted",
    "thick",
    "double",
    "hair",
    "mediumDashed",
    "dashDot",
    "mediumDashDot",
    "dashDotDot",
    "mediumDashDotDot",
    "slantDashDot",
];

pub(super) const H_ALIGN: &[&str] = &[
    "general",
    "left",
    "center",
    "right",
    "fill",
    "justify",
    "centerContinuous",
    "distributed",
];

pub(super) const V_ALIGN: &[&str] = &["top", "center", "bottom", "justify", "distributed"];

pub(super) const DV_KEYS: &[&str] = &[
    "op",
    "id",
    "sqref",
    "type",
    "operator",
    "formula1",
    "formula2",
    "allowBlank",
    "dropdown",
    "showInputMessage",
    "showErrorMessage",
    "errorStyle",
    "errorTitle",
    "error",
    "promptTitle",
    "prompt",
];

pub(super) const CF_KEYS: &[&str] = &[
    "op",
    "id",
    "sqref",
    "type",
    "operator",
    "formulas",
    "dxf",
    "colorScale",
    "dataBar",
    "iconSet",
    "priority",
    "stopIfTrue",
    "text",
    "rank",
    "percent",
    "bottom",
    "aboveAverage",
    "equalAverage",
    "stdDev",
];

pub(super) const ICON_SETS: &[&str] = &[
    "3Arrows",
    "3ArrowsGray",
    "3Flags",
    "3Signs",
    "3Symbols",
    "3Symbols2",
    "3TrafficLights1",
    "3TrafficLights2",
    "4Arrows",
    "4ArrowsGray",
    "4Rating",
    "4RedToBlack",
    "4TrafficLights",
    "5Arrows",
    "5ArrowsGray",
    "5Quarters",
    "5Rating",
];

pub(super) const CFVO_TYPES: &[&str] = &["num", "percent", "max", "min", "formula", "percentile"];

pub(super) const SPARK_KEYS: &[&str] = &[
    "op",
    "id",
    "ref",
    "formula",
    "type",
    "displayEmptyCellsAs",
    "markers",
    "high",
    "low",
    "first",
    "last",
    "negative",
    "displayXAxis",
    "lineWeight",
    "minAxisType",
    "maxAxisType",
    "manualMin",
    "manualMax",
    "colors",
];

#[derive(Clone, Copy, Default)]
pub(crate) struct CfExtraCheck<'a> {
    pub text: Option<&'a str>,
    pub rank: Option<u32>,
    pub percent: Option<bool>,
    pub bottom: Option<bool>,
    pub above_average: Option<bool>,
    pub equal_average: Option<bool>,
    pub std_dev: Option<i32>,
}

pub(super) struct InsertAnchorSpec {
    pub(super) from: a1::CellRef,
    pub(super) to: Option<a1::CellRef>,
    pub(super) anchor: picture::AnchorKind,
    pub(super) from_off_x: i64,
    pub(super) from_off_y: i64,
    pub(super) to_off_x: i64,
    pub(super) to_off_y: i64,
    pub(super) cx: Option<i64>,
    pub(super) cy: Option<i64>,
    pub(super) rel: Option<RelBox>,
}

pub(super) struct DrawingSizePatch {
    pub(super) cx: Option<i64>,
    pub(super) cy: Option<i64>,
    pub(super) to: Option<a1::CellRef>,
    pub(super) to_off_x: Option<i64>,
    pub(super) to_off_y: Option<i64>,
}

pub(super) struct DrawingAnchorPatch {
    pub(super) anchor: Option<picture::AnchorKind>,
    pub(super) from: Option<a1::CellRef>,
    pub(super) to: Option<a1::CellRef>,
    pub(super) from_off_x: Option<i64>,
    pub(super) from_off_y: Option<i64>,
    pub(super) to_off_x: Option<i64>,
    pub(super) to_off_y: Option<i64>,
    pub(super) pos_x: Option<i64>,
    pub(super) pos_y: Option<i64>,
    pub(super) edit_as: Option<picture::EditAs>,
    pub(super) cx: Option<i64>,
    pub(super) cy: Option<i64>,
    pub(super) rel: Option<RelBoxPatch>,
}
