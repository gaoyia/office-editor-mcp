//! xlsx 自己的 scope 与 ops 解析。

use serde_json::Value;

use crate::error::{Error, ErrorKind, IdReason, ScopeNeed, SheetNameReason, TypeName};
use crate::json;

use crate::chart::{ChartExtra, ChartType, NewSeries};

use crate::smartart::{ColorsKind, LayoutKind, NewPoint, Role, StyleKind};

use super::a1;
use super::chart::{self, SheetChartId};
use super::comments;
use super::connection;
use super::custom_view;
use super::extlink;
use super::names;
use super::ole;
use super::picture::{self, SheetPicId};
use super::shape::{self, DrawObjKind, SheetDrawId};
use super::slicer;
use super::smartart as xlsx_smartart;

/// 工作表页眉页脚六个槽位之一。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HfSlot {
    kind: HfKind,
    side: HfSide,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HfKind {
    Odd,
    Even,
    First,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HfSide {
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
    pub model3ds: bool,
    pub maths: bool,
    pub slicers: bool,
    pub protected_ranges: bool,
    pub connections: bool,
    pub external_links: bool,
    pub custom_views: bool,
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
}

/// 省略 / `null` / 有值。`null` 表示把该分量重置为默认。
#[derive(Debug, Clone)]
pub enum Patch<T> {
    Absent,
    Clear,
    Set(T),
}

impl<T> Patch<T> {
    fn is_absent(&self) -> bool {
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
    fn has_patch(&self) -> bool {
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
    fn has_patch(&self) -> bool {
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
    /// xlsx 接受但不写入 XML，树上恒为 null。
    #[allow(dead_code)]
    pub initials: String,
    pub date: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ReplyCommentOp {
    pub id: String,
    pub text: String,
    pub author: String,
    /// xlsx 接受但不写入 XML，树上恒为 null。
    #[allow(dead_code)]
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
    /// xlsx 接受但不写入 XML。
    #[allow(dead_code)]
    pub initials: Option<String>,
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
    fn parse(s: &str) -> Option<Self> {
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
    fn parse(s: &str) -> Option<Self> {
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
    fn has_patch(&self) -> bool {
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
    #[allow(dead_code)]
    pub id: String,
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
    pub spec: crate::chart::remain::TrendlineWrite,
}

#[derive(Debug, Clone)]
pub struct SetTrendlineOp {
    pub id: String,
    pub target: SheetChartId,
    pub index: usize,
    pub tl_index: usize,
    pub spec: crate::chart::remain::TrendlineWrite,
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
    pub spec: crate::chart::remain::ErrorBarsWrite,
}

#[derive(Debug, Clone)]
pub struct SetErrorBarsOp {
    pub id: String,
    pub target: SheetChartId,
    pub index: usize,
    pub eb_index: usize,
    pub spec: crate::chart::remain::ErrorBarsWrite,
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
    InsertModel3d(super::model3d::InsertModel3dOp),
    ReplaceModel3d(super::model3d::ReplaceModel3dOp),
    DeleteModel3d(super::model3d::DeleteModel3dOp),
    InsertMath(super::omml::InsertMathOp),
    SetMath(super::omml::SetMathOp),
    DeleteMath(super::omml::DeleteMathOp),
    InsertSlicer(slicer::InsertSlicerOp),
    DeleteSlicer(slicer::DeleteSlicerOp),
    InsertConnection(connection::InsertConnectionOp),
    DeleteConnection(connection::DeleteConnectionOp),
    InsertExternalLink(extlink::InsertExternalLinkOp),
    DeleteExternalLink(extlink::DeleteExternalLinkOp),
    SetExternalLink(extlink::SetExternalLinkOp),
    SetTextBoxSize(SetTextBoxSizeOp),
    SetTextBoxAnchor(SetTextBoxAnchorOp),
    SetShapeText(SetShapeTextOp),
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
            "sheet",
            "range",
            "maxCells",
            "pictures",
            "charts",
            "headerFooter",
            "comments",
            "textBoxes",
            "shapes",
            "connectors",
            "smartArt",
            "hyperlinks",
            "sparklines",
            "oleObjects",
            "model3ds",
            "maths",
            "slicers",
            "protectedRanges",
            "connections",
            "externalLinks",
            "customViews",
        ],
    )?;
    let pictures = parse_bool_field(obj.get("pictures"), "pictures")?;
    let charts = parse_bool_field(obj.get("charts"), "charts")?;
    let header_footer = parse_bool_field(obj.get("headerFooter"), "headerFooter")?;
    let comments = parse_bool_field(obj.get("comments"), "comments")?;
    let text_boxes = parse_bool_field(obj.get("textBoxes"), "textBoxes")?;
    let shapes = parse_bool_field(obj.get("shapes"), "shapes")?;
    let connectors = parse_bool_field(obj.get("connectors"), "connectors")?;
    let smart_art = parse_bool_field(obj.get("smartArt"), "smartArt")?;
    let hyperlinks = parse_bool_field(obj.get("hyperlinks"), "hyperlinks")?;
    let sparklines = parse_bool_field(obj.get("sparklines"), "sparklines")?;
    let ole_objects = parse_bool_field(obj.get("oleObjects"), "oleObjects")?;
    let model3ds = parse_bool_field(obj.get("model3ds"), "model3ds")?;
    let maths = parse_bool_field(obj.get("maths"), "maths")?;
    let slicers = parse_bool_field(obj.get("slicers"), "slicers")?;
    let protected_ranges = parse_bool_field(obj.get("protectedRanges"), "protectedRanges")?;
    let connections = parse_bool_field(obj.get("connections"), "connections")?;
    let external_links = parse_bool_field(obj.get("externalLinks"), "externalLinks")?;
    let custom_views = parse_bool_field(obj.get("customViews"), "customViews")?;
    let sheet = obj.get("sheet").and_then(Value::as_str);
    let range = obj.get("range").and_then(Value::as_str);
    let max_cells = match obj.get("maxCells") {
        None => None,
        Some(Value::Number(n)) => n.as_u64(),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "maxCells".into(),
                    expected: TypeName::Integer,
                },
                "scope.maxCells",
            ));
        }
    };
    let cells = match (sheet, range, max_cells) {
        (None, None, None) => None,
        (Some(s), Some(r), m) => {
            let max = m.unwrap_or(5000);
            if !(1..=20000).contains(&max) {
                return Err(Error::with_path(
                    ErrorKind::OutOfRange {
                        field: "maxCells".into(),
                        min: 1,
                        max: 20000,
                    },
                    "scope.maxCells",
                ));
            }
            Some(CellScope {
                sheet: s.to_string(),
                range: r.to_string(),
                max_cells: max as u32,
            })
        }
        _ => {
            return Err(Error::with_path(
                ErrorKind::ScopeRequired {
                    need: ScopeNeed::SheetAndRange,
                },
                "scope",
            ));
        }
    };
    if cells.is_none()
        && !pictures
        && !charts
        && !header_footer
        && !comments
        && !text_boxes
        && !shapes
        && !connectors
        && !smart_art
        && !hyperlinks
        && !sparklines
        && !ole_objects
        && !model3ds
        && !maths
        && !slicers
        && !protected_ranges
        && !connections
        && !external_links
        && !custom_views
    {
        return Ok(None);
    }
    Ok(Some(InspectScope {
        cells,
        pictures,
        charts,
        header_footer,
        comments,
        text_boxes,
        shapes,
        connectors,
        smart_art,
        hyperlinks,
        sparklines,
        ole_objects,
        model3ds,
        maths,
        slicers,
        protected_ranges,
        connections,
        external_links,
        custom_views,
    }))
}

pub fn parse_ops(ops: &[Value]) -> Result<Vec<XlsxOp>, Error> {
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
        let parsed = match op_name {
            "setCell" => XlsxOp::SetCell(parse_set_cell(obj, i)?),
            "setCellStyle" => XlsxOp::SetCellStyle(Box::new(parse_set_cell_style(obj, i)?)),
            "setHeaderFooterText" => XlsxOp::SetHeaderFooterText(parse_set_hf_text(obj, i)?),
            "replacePicture" => XlsxOp::ReplacePicture(parse_replace_pic(obj, i)?),
            "insertPicture" => XlsxOp::InsertPicture(parse_insert_pic(obj, i)?),
            "deletePicture" => XlsxOp::DeletePicture(parse_delete_pic(obj, i)?),
            "setPictureSize" => XlsxOp::SetPictureSize(parse_set_pic_size(obj, i)?),
            "setPictureAnchor" => XlsxOp::SetPictureAnchor(parse_set_pic_anchor(obj, i)?),
            "insertChart" => XlsxOp::InsertChart(parse_insert_chart(obj, i)?),
            "insertChartSheet" => XlsxOp::InsertChartSheet(parse_insert_chart_sheet(obj, i)?),
            "deleteChartSheet" => XlsxOp::DeleteChartSheet(parse_delete_chart_sheet(obj, i)?),
            "insertSheet" => XlsxOp::InsertSheet(parse_insert_sheet(obj, i)?),
            "deleteSheet" => XlsxOp::DeleteSheet(parse_delete_sheet(obj, i)?),
            "renameSheet" => XlsxOp::RenameSheet(parse_rename_sheet(obj, i)?),
            "moveSheet" => XlsxOp::MoveSheet(parse_move_sheet(obj, i)?),
            "setSheetHidden" => XlsxOp::SetSheetHidden(parse_set_sheet_hidden(obj, i)?),
            "setColWidth" => XlsxOp::SetColWidth(parse_set_col_width(obj, i)?),
            "setRowHeight" => XlsxOp::SetRowHeight(parse_set_row_height(obj, i)?),
            "mergeRange" => XlsxOp::MergeRange(parse_merge_range(obj, i)?),
            "unmergeRange" => XlsxOp::UnmergeRange(parse_unmerge_range(obj, i)?),
            "insertDataValidation" => {
                XlsxOp::InsertDataValidation(parse_insert_data_validation(obj, i)?)
            }
            "setDataValidation" => XlsxOp::SetDataValidation(parse_set_data_validation(obj, i)?),
            "deleteDataValidation" => {
                XlsxOp::DeleteDataValidation(parse_delete_data_validation(obj, i)?)
            }
            "insertConditionalFormat" => {
                XlsxOp::InsertConditionalFormat(parse_insert_conditional_format(obj, i)?)
            }
            "setConditionalFormat" => {
                XlsxOp::SetConditionalFormat(parse_set_conditional_format(obj, i)?)
            }
            "deleteConditionalFormat" => {
                XlsxOp::DeleteConditionalFormat(parse_delete_conditional_format(obj, i)?)
            }
            "insertSheetTable" => XlsxOp::InsertSheetTable(parse_insert_sheet_table(obj, i)?),
            "setSheetTable" => XlsxOp::SetSheetTable(parse_set_sheet_table(obj, i)?),
            "deleteSheetTable" => XlsxOp::DeleteSheetTable(parse_delete_sheet_table(obj, i)?),
            "insertPivotTable" => XlsxOp::InsertPivotTable(parse_insert_pivot_table(obj, i)?),
            "setPivotTable" => XlsxOp::SetPivotTable(parse_set_pivot_table(obj, i)?),
            "deletePivotTable" => XlsxOp::DeletePivotTable(parse_delete_pivot_table(obj, i)?),
            "insertDefinedName" => XlsxOp::InsertDefinedName(parse_insert_defined_name(obj, i)?),
            "setDefinedName" => XlsxOp::SetDefinedName(parse_set_defined_name(obj, i)?),
            "deleteDefinedName" => XlsxOp::DeleteDefinedName(parse_delete_defined_name(obj, i)?),
            "setPrintSetup" => XlsxOp::SetPrintSetup(parse_set_print_setup(obj, i)?),
            "setSheetView" => XlsxOp::SetSheetView(parse_set_sheet_view(obj, i)?),
            "setAutoFilter" => XlsxOp::SetAutoFilter(parse_set_auto_filter(obj, i)?),
            "setSortState" => XlsxOp::SetSortState(parse_set_sort_state(obj, i)?),
            "insertCustomView" => XlsxOp::InsertCustomView(parse_insert_custom_view(obj, i)?),
            "deleteCustomView" => XlsxOp::DeleteCustomView(parse_delete_custom_view(obj, i)?),
            "showCustomView" => XlsxOp::ShowCustomView(parse_show_custom_view(obj, i)?),
            "groupRows" => XlsxOp::GroupRows(parse_group_rows(obj, i)?),
            "ungroupRows" => XlsxOp::UngroupRows(parse_ungroup_rows(obj, i)?),
            "groupCols" => XlsxOp::GroupCols(parse_group_cols(obj, i)?),
            "ungroupCols" => XlsxOp::UngroupCols(parse_ungroup_cols(obj, i)?),
            "setOutline" => XlsxOp::SetOutline(parse_set_outline(obj, i)?),
            "setRowHidden" => XlsxOp::SetRowHidden(parse_set_row_hidden(obj, i)?),
            "setColHidden" => XlsxOp::SetColHidden(parse_set_col_hidden(obj, i)?),
            "insertComment" => XlsxOp::InsertComment(parse_insert_comment(obj, i)?),
            "replyComment" => XlsxOp::ReplyComment(parse_reply_comment(obj, i)?),
            "deleteComment" => XlsxOp::DeleteComment(parse_delete_comment(obj, i)?),
            "setCommentText" => XlsxOp::SetCommentText(parse_set_comment_text(obj, i)?),
            "setCommentMeta" => XlsxOp::SetCommentMeta(parse_set_comment_meta(obj, i)?),
            "setCommentDone" => XlsxOp::SetCommentDone(parse_set_comment_done(obj, i)?),
            "insertHyperlink" => XlsxOp::InsertHyperlink(parse_insert_hyperlink(obj, i)?),
            "setHyperlink" => XlsxOp::SetHyperlink(parse_set_hyperlink(obj, i)?),
            "deleteHyperlink" => XlsxOp::DeleteHyperlink(parse_delete_hyperlink(obj, i)?),
            "insertSparkline" => XlsxOp::InsertSparkline(parse_insert_sparkline(obj, i)?),
            "setSparkline" => XlsxOp::SetSparkline(parse_set_sparkline(obj, i)?),
            "deleteSparkline" => XlsxOp::DeleteSparkline(parse_delete_sparkline(obj, i)?),
            "setProtection" => XlsxOp::SetProtection(parse_set_protection(obj, i)?),
            "insertProtectedRange" => {
                XlsxOp::InsertProtectedRange(parse_insert_protected_range(obj, i)?)
            }
            "setProtectedRange" => XlsxOp::SetProtectedRange(parse_set_protected_range(obj, i)?),
            "deleteProtectedRange" => {
                XlsxOp::DeleteProtectedRange(parse_delete_protected_range(obj, i)?)
            }
            "deleteChart" => XlsxOp::DeleteChart(parse_delete_chart(obj, i)?),
            "setChartTitle" => XlsxOp::SetChartTitle(parse_set_chart_title(obj, i)?),
            "setChartSeries" => XlsxOp::SetChartSeries(parse_set_chart_series(obj, i)?),
            "addChartSeries" => XlsxOp::AddChartSeries(parse_add_chart_series(obj, i)?),
            "deleteChartSeries" => XlsxOp::DeleteChartSeries(parse_delete_chart_series(obj, i)?),
            "setChartType" => XlsxOp::SetChartType(parse_set_chart_type(obj, i)?),
            "setChartAxisTitle" => XlsxOp::SetChartAxisTitle(parse_set_chart_axis_title(obj, i)?),
            "setChartLegend" => XlsxOp::SetChartLegend(parse_set_chart_legend(obj, i)?),
            "setChartDataLabels" => {
                XlsxOp::SetChartDataLabels(parse_set_chart_data_labels(obj, i)?)
            }
            "setChartSize" => XlsxOp::SetChartSize(parse_set_chart_size(obj, i)?),
            "setChartAnchor" => XlsxOp::SetChartAnchor(parse_set_chart_anchor(obj, i)?),
            "insertTrendline" => XlsxOp::InsertTrendline(parse_insert_trendline(obj, i)?),
            "setTrendline" => XlsxOp::SetTrendline(parse_set_trendline(obj, i)?),
            "deleteTrendline" => XlsxOp::DeleteTrendline(parse_delete_trendline(obj, i)?),
            "insertErrorBars" => XlsxOp::InsertErrorBars(parse_insert_error_bars(obj, i)?),
            "setErrorBars" => XlsxOp::SetErrorBars(parse_set_error_bars(obj, i)?),
            "deleteErrorBars" => XlsxOp::DeleteErrorBars(parse_delete_error_bars(obj, i)?),
            "setChartSeriesAxis" => XlsxOp::SetChartSeriesAxis(parse_set_series_axis(obj, i)?),
            "insertTextBox" => XlsxOp::InsertTextBox(parse_insert_textbox(obj, i)?),
            "insertShape" => XlsxOp::InsertShape(parse_insert_shape(obj, i)?),
            "insertConnector" => XlsxOp::InsertConnector(parse_insert_connector(obj, i)?),
            "deleteTextBox" => XlsxOp::DeleteTextBox(parse_delete_textbox(obj, i)?),
            "deleteShape" => XlsxOp::DeleteShape(parse_delete_shape(obj, i)?),
            "deleteConnector" => XlsxOp::DeleteConnector(parse_delete_connector(obj, i)?),
            "groupShapes" => XlsxOp::GroupShapes(parse_group_shapes(obj, i)?),
            "ungroup" => XlsxOp::Ungroup(parse_ungroup(obj, i)?),
            "insertOle" => XlsxOp::InsertOle(parse_insert_ole(obj, i)?),
            "deleteOle" => XlsxOp::DeleteOle(parse_delete_ole(obj, i)?),
            "insertModel3d" => XlsxOp::InsertModel3d(parse_insert_model3d(obj, i)?),
            "replaceModel3d" => XlsxOp::ReplaceModel3d(parse_replace_model3d(obj, i)?),
            "deleteModel3d" => XlsxOp::DeleteModel3d(parse_delete_model3d(obj, i)?),
            "insertMath" => XlsxOp::InsertMath(parse_insert_math(obj, i)?),
            "setMath" => XlsxOp::SetMath(parse_set_math(obj, i)?),
            "deleteMath" => XlsxOp::DeleteMath(parse_delete_math(obj, i)?),
            "insertSlicer" => XlsxOp::InsertSlicer(parse_insert_slicer(obj, i)?),
            "deleteSlicer" => XlsxOp::DeleteSlicer(parse_delete_slicer(obj, i)?),
            "insertConnection" => XlsxOp::InsertConnection(parse_insert_connection(obj, i)?),
            "deleteConnection" => XlsxOp::DeleteConnection(parse_delete_connection(obj, i)?),
            "insertExternalLink" => XlsxOp::InsertExternalLink(parse_insert_external_link(obj, i)?),
            "deleteExternalLink" => XlsxOp::DeleteExternalLink(parse_delete_external_link(obj, i)?),
            "setExternalLink" => XlsxOp::SetExternalLink(parse_set_external_link(obj, i)?),
            "setTextBoxSize" => XlsxOp::SetTextBoxSize(parse_set_textbox_size(obj, i)?),
            "setTextBoxAnchor" => XlsxOp::SetTextBoxAnchor(parse_set_textbox_anchor(obj, i)?),
            "setShapeText" => XlsxOp::SetShapeText(parse_set_shape_text(obj, i)?),
            "setShapeFormat" => XlsxOp::SetShapeFormat(parse_set_shape_format(obj, i)?),
            "insertSmartArt" => XlsxOp::InsertSmartArt(parse_insert_smartart(obj, i)?),
            "deleteSmartArt" => XlsxOp::DeleteSmartArt(parse_delete_smartart(obj, i)?),
            "setSmartArtText" => XlsxOp::SetSmartArtText(parse_set_smartart_text(obj, i)?),
            "addSmartArtPoint" => XlsxOp::AddSmartArtPoint(parse_add_smartart_point(obj, i)?),
            "deleteSmartArtPoint" => {
                XlsxOp::DeleteSmartArtPoint(parse_delete_smartart_point(obj, i)?)
            }
            "setSmartArtSize" => XlsxOp::SetSmartArtSize(parse_set_smartart_size(obj, i)?),
            "setSmartArtAnchor" => XlsxOp::SetSmartArtAnchor(parse_set_smartart_anchor(obj, i)?),
            "setSmartArtColors" => XlsxOp::SetSmartArtColors(parse_set_smartart_colors(obj, i)?),
            "setSmartArtStyle" => XlsxOp::SetSmartArtStyle(parse_set_smartart_style(obj, i)?),
            "setSmartArtLayout" => XlsxOp::SetSmartArtLayout(parse_set_smartart_layout(obj, i)?),
            "setProps" => XlsxOp::SetProps(crate::props::parse_set_props(obj, i, "xlsx:workbook")?),
            "setCustomProp" => XlsxOp::SetCustomProp(crate::props::parse_set_custom_prop(
                obj,
                i,
                "xlsx:workbook",
            )?),
            "deleteCustomProp" => {
                XlsxOp::DeleteCustomProp(crate::props::parse_delete_custom_prop(obj, i)?)
            }
            "setParaText"
            | "setNotesText"
            | "setPicturePos"
            | "setChartPos"
            | "setSmartArtPos"
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
            | "setPictureWrap"
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
            | "insertStyle"
            | "deleteStyle"
            | "setRunStyle"
            | "setTableStyle"
            | "setTableFloat"
            | "setTableCellMar"
            | "setCellFormat"
            | "setParaFormat"
            | "setRunFormat"
            | "insertRuby"
            | "setRuby"
            | "deleteRuby"
            | "setEastAsianSettings"
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
            | "setSmartArtWrap"
            | "insertWatermark"
            | "deleteWatermark"
            | "insertInkCanvas"
            | "replaceInkCanvas"
            | "deleteInkCanvas"
            | "insertSlide"
            | "deleteSlide"
            | "moveSlide"
            | "setSlideHidden"
            | "setTransition"
            | "insertAnim"
            | "setAnim"
            | "deleteAnim"
            | "moveAnim"
            | "setSlideLayout"
            | "insertLayout"
            | "deleteLayout"
            | "insertPlaceholder"
            | "deletePlaceholder"
            | "insertMaster"
            | "deleteMaster"
            | "setSlideSize"
            | "setThemeColors"
            | "setThemeFonts"
            | "setHandoutText"
            | "setShapePos"
            | "setShapeSize"
            | "setShapeFill"
            | "setShapeLine"
            | "setShapeGeom"
            | "setConnector"
            | "setTableCellText"
            | "setTablePos"
            | "setTableSize"
            | "insertVideo"
            | "insertAudio"
            | "deleteMedia"
            | "setMedia"
            | "insertSlideSection"
            | "setSlideSectionName"
            | "deleteSlideSection"
            | "setSlideSection"
            | "insertCustomShow"
            | "setCustomShow"
            | "deleteCustomShow"
            | "insertZoom"
            | "deleteZoom"
            | "setZoom"
            | "setShowPr" => {
                return Err(Error::with_path(
                    ErrorKind::OpNotForFormat {
                        format: "xlsx".into(),
                        op: op_name.to_string(),
                    },
                    format!("ops[{i}].op"),
                ));
            }
            other => {
                return Err(Error::with_path(
                    ErrorKind::UnsupportedOp {
                        op: other.to_string(),
                    },
                    format!("ops[{i}].op"),
                ));
            }
        };
        out.push(parsed);
    }
    Ok(out)
}

fn parse_set_cell(obj: &serde_json::Map<String, Value>, i: usize) -> Result<SetCellOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op",
            "id",
            "value",
            "formula",
            "shared",
            "sharedRef",
            "array",
            "arrayRef",
        ],
    )?;
    let id = require_id(obj, i)?;
    let has_value = obj.contains_key("value");
    let has_formula = obj.contains_key("formula");
    if has_value && has_formula {
        return Err(Error::with_path(
            ErrorKind::MutuallyExclusive {
                a: "value".into(),
                b: "formula".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    if !has_value && !has_formula {
        return Err(Error::with_path(
            ErrorKind::SetCellNeedsValueOrFormula,
            format!("ops[{i}]"),
        ));
    }
    let formula = if has_formula {
        let f = obj.get("formula").and_then(Value::as_str).ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: "formula".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].formula"),
            )
        })?;
        if !f.starts_with('=') {
            return Err(Error::with_path(
                ErrorKind::FormulaMustStartWithEquals,
                format!("ops[{i}].formula"),
            ));
        }
        Some(f.to_string())
    } else {
        None
    };
    let value = if has_value {
        Some(parse_cell_value(&obj["value"], &format!("ops[{i}].value"))?)
    } else {
        None
    };
    let shared = parse_opt_bool(obj, i, "shared")?;
    let array = parse_opt_bool(obj, i, "array")?;
    let shared_ref = parse_opt_a1_range(obj, i, "sharedRef")?;
    let array_ref = parse_opt_a1_range(obj, i, "arrayRef")?;
    if shared_ref.is_some() && array_ref.is_some() {
        return Err(Error::with_path(
            ErrorKind::MutuallyExclusive {
                a: "sharedRef".into(),
                b: "arrayRef".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    if shared == Some(true) && array == Some(true) {
        return Err(Error::with_path(
            ErrorKind::MutuallyExclusive {
                a: "shared".into(),
                b: "array".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    if shared == Some(false) && shared_ref.is_some() {
        return Err(Error::with_path(
            ErrorKind::MutuallyExclusive {
                a: "shared".into(),
                b: "sharedRef".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    if array == Some(false) && array_ref.is_some() {
        return Err(Error::with_path(
            ErrorKind::MutuallyExclusive {
                a: "array".into(),
                b: "arrayRef".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    if has_value
        && (shared == Some(true)
            || array == Some(true)
            || shared_ref.is_some()
            || array_ref.is_some())
    {
        let b = if shared == Some(true) {
            "shared"
        } else if array == Some(true) {
            "array"
        } else if shared_ref.is_some() {
            "sharedRef"
        } else {
            "arrayRef"
        };
        return Err(Error::with_path(
            ErrorKind::MutuallyExclusive {
                a: "value".into(),
                b: b.into(),
            },
            format!("ops[{i}]"),
        ));
    }
    if (shared_ref.is_some() || array_ref.is_some() || array == Some(true)) && formula.is_none() {
        return Err(Error::with_path(
            ErrorKind::MissingField {
                field: "formula".into(),
            },
            format!("ops[{i}].formula"),
        ));
    }
    Ok(SetCellOp {
        id,
        value,
        formula,
        shared,
        shared_ref,
        array,
        array_ref,
    })
}

fn parse_set_cell_style(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetCellStyleOp, Error> {
    deny_keys(
        obj,
        i,
        &["op", "id", "numFmt", "font", "fill", "border", "align"],
    )?;
    let id = require_id(obj, i)?;
    let num_fmt = parse_patch_string(obj, i, "numFmt")?;
    let font = parse_patch_object(obj, i, "font", parse_font_patch)?;
    let fill = parse_patch_object(obj, i, "fill", parse_fill_patch)?;
    let border = parse_patch_object(obj, i, "border", parse_border_patch)?;
    let align = parse_patch_object(obj, i, "align", parse_align_patch)?;
    if num_fmt.is_absent()
        && font.is_absent()
        && fill.is_absent()
        && border.is_absent()
        && align.is_absent()
    {
        return Err(Error::with_path(
            ErrorKind::NeedSetCellStyleField,
            format!("ops[{i}]"),
        ));
    }
    Ok(SetCellStyleOp {
        id,
        num_fmt,
        font,
        fill,
        border,
        align,
    })
}

fn parse_patch_string(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    field: &str,
) -> Result<Patch<String>, Error> {
    match obj.get(field) {
        None => Ok(Patch::Absent),
        Some(Value::Null) => Ok(Patch::Clear),
        Some(Value::String(s)) if !s.is_empty() => Ok(Patch::Set(s.clone())),
        Some(Value::String(_)) => Err(Error::with_path(
            ErrorKind::InvalidCellStyle {
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

fn parse_patch_object<T>(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    field: &str,
    parse: impl Fn(&serde_json::Map<String, Value>, usize, &str) -> Result<T, Error>,
) -> Result<Patch<T>, Error> {
    match obj.get(field) {
        None => Ok(Patch::Absent),
        Some(Value::Null) => Ok(Patch::Clear),
        Some(Value::Object(m)) => Ok(Patch::Set(parse(m, i, field)?)),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Object,
            },
            format!("ops[{i}].{field}"),
        )),
    }
}

fn deny_sub_keys(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    prefix: &str,
    allowed: &[&str],
) -> Result<(), Error> {
    for k in obj.keys() {
        if !allowed.contains(&k.as_str()) {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("ops[{i}].{prefix}.{k}"),
            ));
        }
    }
    Ok(())
}

fn parse_font_patch(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    prefix: &str,
) -> Result<FontPatch, Error> {
    deny_sub_keys(
        obj,
        i,
        prefix,
        &["name", "sz", "bold", "italic", "underline", "color"],
    )?;
    let name = match obj.get("name") {
        None => None,
        Some(Value::String(s)) if !s.is_empty() => Some(s.clone()),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::InvalidCellStyle {
                    field: "name".into(),
                },
                format!("ops[{i}].{prefix}.name"),
            ));
        }
    };
    let sz = match obj.get("sz") {
        None => None,
        Some(Value::Number(n)) => {
            let v = n.as_f64().unwrap_or(f64::NAN);
            if !v.is_finite() || v <= 0.0 || v > 409.0 {
                return Err(Error::with_path(
                    ErrorKind::InvalidCellStyle { field: "sz".into() },
                    format!("ops[{i}].{prefix}.sz"),
                ));
            }
            Some(v)
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "sz".into(),
                    expected: TypeName::Integer,
                },
                format!("ops[{i}].{prefix}.sz"),
            ));
        }
    };
    let bold = parse_sub_bool(obj, i, prefix, "bold")?;
    let italic = parse_sub_bool(obj, i, prefix, "italic")?;
    let underline = match obj.get("underline") {
        None => None,
        Some(Value::Null) | Some(Value::Bool(false)) => Some(None),
        Some(Value::Bool(true)) => Some(Some("single".into())),
        Some(Value::String(s)) => {
            if matches!(
                s.as_str(),
                "single" | "double" | "singleAccounting" | "doubleAccounting" | "none"
            ) {
                if s == "none" {
                    Some(None)
                } else {
                    Some(Some(s.clone()))
                }
            } else {
                return Err(Error::with_path(
                    ErrorKind::InvalidCellStyle {
                        field: "underline".into(),
                    },
                    format!("ops[{i}].{prefix}.underline"),
                ));
            }
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::InvalidCellStyle {
                    field: "underline".into(),
                },
                format!("ops[{i}].{prefix}.underline"),
            ));
        }
    };
    let color = parse_opt_color(obj, i, prefix, "color")?;
    Ok(FontPatch {
        name,
        sz,
        bold,
        italic,
        underline,
        color,
    })
}

fn parse_fill_patch(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    prefix: &str,
) -> Result<FillPatch, Error> {
    deny_sub_keys(obj, i, prefix, &["pattern", "fg", "bg"])?;
    let pattern = match obj.get("pattern") {
        None => None,
        Some(Value::String(s)) => {
            if s == "gradient" || !PATTERN_TYPES.contains(&s.as_str()) {
                return Err(Error::with_path(
                    ErrorKind::InvalidCellStyle {
                        field: "pattern".into(),
                    },
                    format!("ops[{i}].{prefix}.pattern"),
                ));
            }
            Some(s.clone())
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::InvalidCellStyle {
                    field: "pattern".into(),
                },
                format!("ops[{i}].{prefix}.pattern"),
            ));
        }
    };
    Ok(FillPatch {
        pattern,
        fg: parse_opt_color(obj, i, prefix, "fg")?,
        bg: parse_opt_color(obj, i, prefix, "bg")?,
    })
}

fn parse_border_patch(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    prefix: &str,
) -> Result<BorderPatch, Error> {
    deny_sub_keys(
        obj,
        i,
        prefix,
        &[
            "style",
            "color",
            "left",
            "right",
            "top",
            "bottom",
            "diagonal",
            "diagonalUp",
            "diagonalDown",
        ],
    )?;
    Ok(BorderPatch {
        style: parse_opt_border_style(obj, i, prefix, "style")?,
        color: parse_opt_color(obj, i, prefix, "color")?,
        left: parse_opt_border_side(obj, i, prefix, "left")?,
        right: parse_opt_border_side(obj, i, prefix, "right")?,
        top: parse_opt_border_side(obj, i, prefix, "top")?,
        bottom: parse_opt_border_side(obj, i, prefix, "bottom")?,
        diagonal: parse_opt_border_side(obj, i, prefix, "diagonal")?,
        diagonal_up: parse_sub_bool(obj, i, prefix, "diagonalUp")?,
        diagonal_down: parse_sub_bool(obj, i, prefix, "diagonalDown")?,
    })
}

fn parse_opt_border_side(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    prefix: &str,
    field: &str,
) -> Result<Option<BorderSidePatch>, Error> {
    let path = format!("{prefix}.{field}");
    match obj.get(field) {
        None => Ok(None),
        Some(Value::Object(m)) => {
            deny_sub_keys(m, i, &path, &["style", "color"])?;
            Ok(Some(BorderSidePatch {
                style: parse_opt_border_style(m, i, &path, "style")?,
                color: parse_opt_color(m, i, &path, "color")?,
            }))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Object,
            },
            format!("ops[{i}].{path}"),
        )),
    }
}

fn parse_opt_border_style(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    prefix: &str,
    field: &str,
) -> Result<Option<String>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::String(s)) if BORDER_STYLES.contains(&s.as_str()) => Ok(Some(s.clone())),
        Some(_) => Err(Error::with_path(
            ErrorKind::InvalidCellStyle {
                field: "style".into(),
            },
            format!("ops[{i}].{prefix}.{field}"),
        )),
    }
}

fn parse_align_patch(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    prefix: &str,
) -> Result<AlignPatch, Error> {
    deny_sub_keys(
        obj,
        i,
        prefix,
        &[
            "horizontal",
            "vertical",
            "wrap",
            "indent",
            "rotation",
            "shrink",
        ],
    )?;
    let horizontal = match obj.get("horizontal") {
        None => None,
        Some(Value::String(s)) if H_ALIGN.contains(&s.as_str()) => Some(s.clone()),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::InvalidCellStyle {
                    field: "horizontal".into(),
                },
                format!("ops[{i}].{prefix}.horizontal"),
            ));
        }
    };
    let vertical = match obj.get("vertical") {
        None => None,
        Some(Value::String(s)) if V_ALIGN.contains(&s.as_str()) => Some(s.clone()),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::InvalidCellStyle {
                    field: "vertical".into(),
                },
                format!("ops[{i}].{prefix}.vertical"),
            ));
        }
    };
    let indent = parse_opt_u32_field(obj, i, prefix, "indent")?;
    if let Some(n) = indent {
        if n > 250 {
            return Err(Error::with_path(
                ErrorKind::InvalidCellStyle {
                    field: "indent".into(),
                },
                format!("ops[{i}].{prefix}.indent"),
            ));
        }
    }
    let rotation = parse_opt_u32_field(obj, i, prefix, "rotation")?;
    if let Some(n) = rotation {
        if n > 180 {
            return Err(Error::with_path(
                ErrorKind::InvalidCellStyle {
                    field: "rotation".into(),
                },
                format!("ops[{i}].{prefix}.rotation"),
            ));
        }
    }
    Ok(AlignPatch {
        horizontal,
        vertical,
        wrap: parse_sub_bool(obj, i, prefix, "wrap")?,
        indent,
        rotation,
        shrink: parse_sub_bool(obj, i, prefix, "shrink")?,
    })
}

fn parse_sub_bool(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    prefix: &str,
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
            format!("ops[{i}].{prefix}.{field}"),
        )),
    }
}

fn parse_opt_color(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    prefix: &str,
    field: &str,
) -> Result<Option<ColorSpec>, Error> {
    let path = format!("{prefix}.{field}");
    match obj.get(field) {
        None => Ok(None),
        Some(Value::Object(m)) => Ok(Some(parse_color_spec(m, i, &path)?)),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Object,
            },
            format!("ops[{i}].{path}"),
        )),
    }
}

fn parse_color_spec(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    prefix: &str,
) -> Result<ColorSpec, Error> {
    deny_sub_keys(obj, i, prefix, &["rgb", "theme", "indexed", "tint"])?;
    let rgb = match obj.get("rgb") {
        None => None,
        Some(Value::String(s)) if is_cell_rgb(s) => Some(s.to_ascii_uppercase()),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::InvalidCellStyle {
                    field: "color".into(),
                },
                format!("ops[{i}].{prefix}.rgb"),
            ));
        }
    };
    let theme = parse_opt_u32_field(obj, i, prefix, "theme")?;
    let indexed = parse_opt_u32_field(obj, i, prefix, "indexed")?;
    let tint = match obj.get("tint") {
        None => None,
        Some(Value::Number(n)) => n.as_f64(),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::InvalidCellStyle {
                    field: "tint".into(),
                },
                format!("ops[{i}].{prefix}.tint"),
            ));
        }
    };
    let kinds = [rgb.is_some(), theme.is_some(), indexed.is_some()]
        .into_iter()
        .filter(|b| *b)
        .count();
    if kinds != 1 || (tint.is_some() && theme.is_none()) {
        return Err(Error::with_path(
            ErrorKind::InvalidCellStyle {
                field: "color".into(),
            },
            format!("ops[{i}].{prefix}"),
        ));
    }
    Ok(ColorSpec {
        rgb,
        theme,
        indexed,
        tint,
    })
}

fn parse_opt_u32_field(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    prefix: &str,
    field: &str,
) -> Result<Option<u32>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::Number(n)) => {
            if let Some(u) = n.as_u64().filter(|v| *v <= u32::MAX as u64) {
                Ok(Some(u as u32))
            } else {
                Err(Error::with_path(
                    ErrorKind::InvalidCellStyle {
                        field: field.into(),
                    },
                    format!("ops[{i}].{prefix}.{field}"),
                ))
            }
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::InvalidCellStyle {
                field: field.into(),
            },
            format!("ops[{i}].{prefix}.{field}"),
        )),
    }
}

fn is_cell_rgb(s: &str) -> bool {
    (s.len() == 6 || s.len() == 8) && s.chars().all(|c| c.is_ascii_hexdigit())
}

const PATTERN_TYPES: &[&str] = &[
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

const BORDER_STYLES: &[&str] = &[
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

const H_ALIGN: &[&str] = &[
    "general",
    "left",
    "center",
    "right",
    "fill",
    "justify",
    "centerContinuous",
    "distributed",
];

const V_ALIGN: &[&str] = &["top", "center", "bottom", "justify", "distributed"];

fn parse_set_hf_text(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetHeaderFooterTextOp, Error> {
    deny_keys(obj, i, &["op", "id", "text"])?;
    let id = require_id(obj, i)?;
    let (sheet, slot) = super::hf::parse_slot_id(&id, i)?;
    let text = match obj.get("text") {
        Some(Value::String(s)) => s.clone(),
        None => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "text".into(),
                },
                format!("ops[{i}].text"),
            ));
        }
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
    Ok(SetHeaderFooterTextOp {
        id,
        sheet,
        slot,
        text,
    })
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
    deny_keys(
        obj,
        i,
        &[
            "op", "id", "file", "cx", "cy", "from", "to", "anchor", "fromOffX", "fromOffY",
            "toOffX", "toOffY", "fromX", "fromY", "toX", "toY",
        ],
    )?;
    let id = require_id(obj, i)?;
    let (sheet, hf) = picture::parse_insert_sheet_id(&id, i)?;
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
    if hf.is_some() {
        for field in [
            "from", "to", "anchor", "fromOffX", "fromOffY", "toOffX", "toOffY", "fromX", "fromY",
            "toX", "toY",
        ] {
            if obj.contains_key(field) {
                return Err(Error::with_path(
                    ErrorKind::UnknownField {
                        field: field.into(),
                    },
                    format!("ops[{i}].{field}"),
                ));
            }
        }
        return Ok(InsertPictureOp {
            sheet,
            hf,
            file,
            cx,
            cy,
            from: a1::CellRef { col: 1, row: 1 },
            to: None,
            anchor: picture::AnchorKind::OneCell,
            from_off_x: 0,
            from_off_y: 0,
            to_off_x: 0,
            to_off_y: 0,
            rel: None,
        });
    }
    let rel = parse_rel_box(obj, i)?;
    if rel.is_some() {
        for field in [
            "from", "to", "anchor", "fromOffX", "fromOffY", "toOffX", "toOffY",
        ] {
            if obj.contains_key(field) {
                return Err(Error::with_path(
                    ErrorKind::UnknownField {
                        field: field.into(),
                    },
                    format!("ops[{i}].{field}"),
                ));
            }
        }
        return Ok(InsertPictureOp {
            sheet,
            hf: None,
            file,
            cx,
            cy,
            from: a1::CellRef { col: 1, row: 1 },
            to: None,
            anchor: picture::AnchorKind::RelSize,
            from_off_x: 0,
            from_off_y: 0,
            to_off_x: 0,
            to_off_y: 0,
            rel,
        });
    }
    let from = match obj.get("from") {
        Some(Value::String(s)) => a1::parse_cell(s).map_err(|e| e.at(format!("ops[{i}].from")))?,
        None => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "from".into(),
                },
                format!("ops[{i}].from"),
            ));
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
    let anchor = match obj.get("anchor") {
        None => picture::AnchorKind::OneCell,
        Some(Value::String(s)) => picture::AnchorKind::parse_insert(s).ok_or_else(|| {
            Error::with_path(ErrorKind::InvalidXlsxAnchor, format!("ops[{i}].anchor"))
        })?,
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "anchor".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].anchor"),
            ));
        }
    };
    let to = match obj.get("to") {
        None => None,
        Some(Value::String(s)) => {
            Some(a1::parse_cell(s).map_err(|e| e.at(format!("ops[{i}].to")))?)
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "to".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].to"),
            ));
        }
    };
    if anchor == picture::AnchorKind::TwoCell && to.is_none() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "insertPicture".into(),
                field: "to".into(),
            },
            format!("ops[{i}].to"),
        ));
    }
    if anchor == picture::AnchorKind::OneCell && to.is_some() {
        return Err(Error::with_path(
            ErrorKind::UnknownField { field: "to".into() },
            format!("ops[{i}].to"),
        ));
    }
    Ok(InsertPictureOp {
        sheet,
        hf: None,
        file,
        cx,
        cy,
        from,
        to,
        anchor,
        from_off_x: parse_off_field(obj.get("fromOffX"), i, "fromOffX")?.unwrap_or(0),
        from_off_y: parse_off_field(obj.get("fromOffY"), i, "fromOffY")?.unwrap_or(0),
        to_off_x: parse_off_field(obj.get("toOffX"), i, "toOffX")?.unwrap_or(0),
        to_off_y: parse_off_field(obj.get("toOffY"), i, "toOffY")?.unwrap_or(0),
        rel: None,
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
    deny_keys(obj, i, &["op", "id", "cx", "cy", "to", "toOffX", "toOffY"])?;
    let id = require_id(obj, i)?;
    let target = picture::parse_pic_id(&id, i)?;
    let s = parse_drawing_size(obj, i, "setPictureSize")?;
    Ok(SetPictureSizeOp {
        id,
        target,
        cx: s.cx,
        cy: s.cy,
        to: s.to,
        to_off_x: s.to_off_x,
        to_off_y: s.to_off_y,
    })
}

fn parse_set_pic_anchor(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetPictureAnchorOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op", "id", "anchor", "from", "to", "fromOffX", "fromOffY", "toOffX", "toOffY", "posX",
            "posY", "editAs", "cx", "cy", "fromX", "fromY", "toX", "toY",
        ],
    )?;
    let id = require_id(obj, i)?;
    let target = picture::parse_pic_id(&id, i)?;
    let p = parse_drawing_anchor_patch(obj, i, "setPictureAnchor")?;
    Ok(SetPictureAnchorOp {
        id,
        target,
        anchor: p.anchor,
        from: p.from,
        to: p.to,
        from_off_x: p.from_off_x,
        from_off_y: p.from_off_y,
        to_off_x: p.to_off_x,
        to_off_y: p.to_off_y,
        pos_x: p.pos_x,
        pos_y: p.pos_y,
        edit_as: p.edit_as,
        cx: p.cx,
        cy: p.cy,
        rel: p.rel,
    })
}

pub fn parse_cell_value(v: &Value, path: &str) -> Result<SetCellValue, Error> {
    match v {
        Value::Null => Ok(SetCellValue::Null),
        Value::Bool(b) => Ok(SetCellValue::Bool(*b)),
        Value::Number(n) => Ok(SetCellValue::Number(n.clone())),
        Value::String(s) => Ok(SetCellValue::Text(s.clone())),
        _ => Err(Error::with_path(ErrorKind::InvalidCellValue, path)),
    }
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

fn parse_opt_a1_range(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    field: &str,
) -> Result<Option<(a1::CellRef, a1::CellRef)>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::String(s)) => {
            let (start, end) = a1::parse_range(s).map_err(|e| e.at(format!("ops[{i}].{field}")))?;
            Ok(Some((start, end)))
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

fn parse_off_field(v: Option<&Value>, i: usize, field: &str) -> Result<Option<i64>, Error> {
    let Some(v) = v else {
        return Ok(None);
    };
    let path = format!("ops[{i}].{field}");
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

fn parse_opt_a1(v: Option<&Value>, i: usize, field: &str) -> Result<Option<a1::CellRef>, Error> {
    match v {
        None => Ok(None),
        Some(Value::String(s)) => Ok(Some(
            a1::parse_cell(s).map_err(|e| e.at(format!("ops[{i}].{field}")))?,
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

fn parse_required_a1(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    field: &str,
) -> Result<a1::CellRef, Error> {
    match obj.get(field) {
        Some(Value::String(s)) => a1::parse_cell(s).map_err(|e| e.at(format!("ops[{i}].{field}"))),
        None => Err(Error::with_path(
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

fn parse_insert_chart(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertChartOp, Error> {
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
            "cx",
            "cy",
            "from",
            "to",
            "anchor",
            "fromOffX",
            "fromOffY",
            "toOffX",
            "toOffY",
            "dataFrom",
        ],
    )?;
    if obj.contains_key("kind") {
        return Err(Error::with_path(
            ErrorKind::UnknownField {
                field: "kind".into(),
            },
            format!("ops[{i}].kind"),
        ));
    }
    if obj.contains_key("wrap") {
        return Err(Error::with_path(
            ErrorKind::UnknownField {
                field: "wrap".into(),
            },
            format!("ops[{i}].wrap"),
        ));
    }
    let id = require_id(obj, i)?;
    let sheet = chart::parse_insert_sheet_id(&id, i)?;
    let chart_type = parse_chart_type_field(obj.get("type"), i, true)?;
    let of_pie_type =
        crate::chart::parse_of_pie_type(obj.get("ofPieType"), &format!("ops[{i}].ofPieType"))?;
    crate::chart::reject_of_pie_on_other(
        chart_type,
        of_pie_type.as_deref(),
        &format!("ops[{i}].ofPieType"),
    )?;
    let title = parse_chart_title_field(obj.get("title"), i)?;
    let series = parse_insert_series(obj.get("series"), i, chart_type, "insertChart")?;
    let extra =
        crate::chart::parse_chart_extra(obj, chart_type, series.len(), &format!("ops[{i}]"), true)?;
    validate_insert_series(chart_type, &series, &extra, i)?;
    let (cx, cy) = parse_chart_size_pair(obj, i, "insertChart")?;
    let from = parse_required_a1(obj, i, "from")?;
    let data_from = parse_required_a1(obj, i, "dataFrom")?;
    let anchor = match obj.get("anchor") {
        None => picture::AnchorKind::OneCell,
        Some(Value::String(s)) => picture::AnchorKind::parse_insert(s).ok_or_else(|| {
            Error::with_path(ErrorKind::InvalidXlsxAnchor, format!("ops[{i}].anchor"))
        })?,
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "anchor".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].anchor"),
            ));
        }
    };
    let to = parse_opt_a1(obj.get("to"), i, "to")?;
    if anchor == picture::AnchorKind::TwoCell && to.is_none() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "insertChart".into(),
                field: "to".into(),
            },
            format!("ops[{i}].to"),
        ));
    }
    if anchor == picture::AnchorKind::OneCell && to.is_some() {
        return Err(Error::with_path(
            ErrorKind::UnknownField { field: "to".into() },
            format!("ops[{i}].to"),
        ));
    }
    Ok(InsertChartOp {
        sheet,
        chart_type,
        of_pie_type,
        extra,
        title,
        series,
        cx,
        cy,
        from,
        to,
        anchor,
        from_off_x: parse_off_field(obj.get("fromOffX"), i, "fromOffX")?.unwrap_or(0),
        from_off_y: parse_off_field(obj.get("fromOffY"), i, "fromOffY")?.unwrap_or(0),
        to_off_x: parse_off_field(obj.get("toOffX"), i, "toOffX")?.unwrap_or(0),
        to_off_y: parse_off_field(obj.get("toOffY"), i, "toOffY")?.unwrap_or(0),
        data_from,
    })
}

fn parse_insert_chart_sheet(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertChartSheetOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op",
            "id",
            "name",
            "dataSheet",
            "type",
            "ofPieType",
            "stockKind",
            "surfaceKind",
            "layoutId",
            "title",
            "series",
            "cx",
            "cy",
            "dataFrom",
        ],
    )?;
    let id = require_id(obj, i)?;
    if id != "xlsx:workbook" {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::InsertChartSheetNeedsWorkbook,
            },
            format!("ops[{i}].id"),
        ));
    }
    let name = match obj.get("name") {
        Some(Value::String(s)) => {
            validate_sheet_name(s, format!("ops[{i}].name"))?;
            s.clone()
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::InvalidSheetName {
                    reason: SheetNameReason::Missing,
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
    let data_sheet = match obj.get("dataSheet") {
        None | Some(Value::Null) => None,
        Some(Value::String(s)) if !s.is_empty() => Some(s.clone()),
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "dataSheet".into(),
                },
                format!("ops[{i}].dataSheet"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "dataSheet".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].dataSheet"),
            ));
        }
    };
    let chart_type = parse_chart_type_field(obj.get("type"), i, true)?;
    let of_pie_type =
        crate::chart::parse_of_pie_type(obj.get("ofPieType"), &format!("ops[{i}].ofPieType"))?;
    crate::chart::reject_of_pie_on_other(
        chart_type,
        of_pie_type.as_deref(),
        &format!("ops[{i}].ofPieType"),
    )?;
    let title = parse_chart_title_field(obj.get("title"), i)?;
    let series = parse_insert_series(obj.get("series"), i, chart_type, "insertChartSheet")?;
    let extra =
        crate::chart::parse_chart_extra(obj, chart_type, series.len(), &format!("ops[{i}]"), true)?;
    validate_insert_series(chart_type, &series, &extra, i)?;
    let (cx, cy) = parse_chart_size_pair(obj, i, "insertChartSheet")?;
    let data_from = parse_required_a1(obj, i, "dataFrom")?;
    Ok(InsertChartSheetOp {
        name,
        data_sheet,
        chart_type,
        of_pie_type,
        extra,
        title,
        series,
        cx,
        cy,
        data_from,
    })
}

fn parse_delete_chart_sheet(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteChartSheetOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    Ok(DeleteChartSheetOp {
        sheet: parse_sheet_only_id(&id, i, "id", IdReason::DeleteChartSheetNeedsChartsheet)?,
    })
}

fn parse_insert_sheet(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertSheetOp, Error> {
    deny_keys(obj, i, &["op", "id", "name", "after"])?;
    let id = require_id(obj, i)?;
    if id != "xlsx:workbook" {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::InsertSheetNeedsWorkbook,
            },
            format!("ops[{i}].id"),
        ));
    }
    Ok(InsertSheetOp {
        name: parse_new_sheet_name(obj, i)?,
        after: parse_optional_after(obj, i, false)?,
    })
}

fn parse_delete_sheet(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteSheetOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    Ok(DeleteSheetOp {
        sheet: parse_sheet_only_id(&id, i, "id", IdReason::DeleteSheetNeedsWorksheet)?,
    })
}

fn parse_rename_sheet(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<RenameSheetOp, Error> {
    deny_keys(obj, i, &["op", "id", "name"])?;
    let id = require_id(obj, i)?;
    Ok(RenameSheetOp {
        sheet: parse_sheet_only_id(&id, i, "id", IdReason::SheetOpNeedsSheet)?,
        name: parse_new_sheet_name(obj, i)?,
    })
}

fn parse_move_sheet(obj: &serde_json::Map<String, Value>, i: usize) -> Result<MoveSheetOp, Error> {
    deny_keys(obj, i, &["op", "id", "after"])?;
    let id = require_id(obj, i)?;
    Ok(MoveSheetOp {
        sheet: parse_sheet_only_id(&id, i, "id", IdReason::SheetOpNeedsSheet)?,
        after: parse_optional_after(obj, i, true)?,
    })
}

fn parse_set_sheet_hidden(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetSheetHiddenOp, Error> {
    deny_keys(obj, i, &["op", "id", "hidden"])?;
    let id = require_id(obj, i)?;
    Ok(SetSheetHiddenOp {
        sheet: parse_sheet_only_id(&id, i, "id", IdReason::SheetOpNeedsSheet)?,
        hidden: parse_required_bool(obj.get("hidden"), i, "setSheetHidden", "hidden")?,
    })
}

fn parse_set_col_width(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetColWidthOp, Error> {
    deny_keys(obj, i, &["op", "id", "col", "width", "bestFit"])?;
    let id = require_id(obj, i)?;
    let sheet = parse_sheet_only_id(&id, i, "id", IdReason::SheetOpNeedsSheet)?;
    let col = match obj.get("col") {
        Some(Value::String(s)) => s,
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "col".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].col"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "col".into(),
                },
                format!("ops[{i}].col"),
            ));
        }
    };
    let (min_col, max_col) = a1::parse_col_span(col).map_err(|e| e.at(format!("ops[{i}].col")))?;
    let width = parse_patch_dim(obj, i, "width", 255.0, ErrorKind::InvalidColWidth)?;
    let best_fit = match obj.get("bestFit") {
        None => None,
        Some(Value::Bool(b)) => Some(*b),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "bestFit".into(),
                    expected: TypeName::Boolean,
                },
                format!("ops[{i}].bestFit"),
            ));
        }
    };
    if width.is_absent() && best_fit.is_none() {
        return Err(Error::with_path(
            ErrorKind::NeedSetColWidthField,
            format!("ops[{i}]"),
        ));
    }
    Ok(SetColWidthOp {
        sheet,
        min_col,
        max_col,
        width,
        best_fit,
    })
}

fn parse_set_row_height(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetRowHeightOp, Error> {
    deny_keys(obj, i, &["op", "id", "row", "toRow", "height"])?;
    let id = require_id(obj, i)?;
    let sheet = parse_sheet_only_id(&id, i, "id", IdReason::SheetOpNeedsSheet)?;
    let row = parse_required_row(obj, i, "row")?;
    let to_row = match obj.get("toRow") {
        None => row,
        Some(_) => parse_required_row(obj, i, "toRow")?,
    };
    if to_row < row {
        return Err(Error::with_path(
            ErrorKind::RowRangeReversed,
            format!("ops[{i}].toRow"),
        ));
    }
    let height = match obj.get("height") {
        None => {
            return Err(Error::with_path(
                ErrorKind::NeedSetRowHeightField,
                format!("ops[{i}].height"),
            ));
        }
        Some(Value::Null) => Patch::Clear,
        Some(Value::Number(n)) => {
            let Some(v) = n.as_f64() else {
                return Err(Error::with_path(
                    ErrorKind::InvalidRowHeight,
                    format!("ops[{i}].height"),
                ));
            };
            if !(v > 0.0 && v <= 409.0) {
                return Err(Error::with_path(
                    ErrorKind::InvalidRowHeight,
                    format!("ops[{i}].height"),
                ));
            }
            Patch::Set(v)
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "height".into(),
                    expected: TypeName::Integer,
                },
                format!("ops[{i}].height"),
            ));
        }
    };
    Ok(SetRowHeightOp {
        sheet,
        row,
        to_row,
        height,
    })
}

fn parse_merge_range(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<MergeRangeOp, Error> {
    deny_keys(obj, i, &["op", "id", "range"])?;
    let id = require_id(obj, i)?;
    let sheet = parse_sheet_only_id(&id, i, "id", IdReason::SheetOpNeedsSheet)?;
    let (start, end) = parse_op_range(obj, i)?;
    if start == end {
        return Err(Error::with_path(
            ErrorKind::MergeNeedsMultipleCells,
            format!("ops[{i}].range"),
        ));
    }
    Ok(MergeRangeOp { sheet, start, end })
}

fn parse_unmerge_range(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<UnmergeRangeOp, Error> {
    deny_keys(obj, i, &["op", "id", "range"])?;
    let id = require_id(obj, i)?;
    let sheet = parse_sheet_only_id(&id, i, "id", IdReason::SheetOpNeedsSheet)?;
    let (start, end) = parse_op_range(obj, i)?;
    Ok(UnmergeRangeOp { sheet, start, end })
}

const DV_KEYS: &[&str] = &[
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

fn parse_insert_data_validation(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertDataValidationOp, Error> {
    deny_keys(obj, i, DV_KEYS)?;
    let id = require_id(obj, i)?;
    let sheet = parse_sheet_only_id(&id, i, "id", IdReason::SheetOpNeedsSheet)?;
    let sqref = parse_sqref_field(obj.get("sqref"), i, true)?;
    let type_ = parse_dv_type_field(obj.get("type"), i, true)?.ok_or_else(|| {
        Error::with_path(
            ErrorKind::MissingField {
                field: "type".into(),
            },
            format!("ops[{i}].type"),
        )
    })?;
    let operator = parse_dv_operator_field(obj.get("operator"), i, false)?;
    let formula1 = parse_dv_formula_opt(obj.get("formula1"), i, "formula1")?;
    let formula2 = parse_dv_formula_opt(obj.get("formula2"), i, "formula2")?;
    let dropdown = parse_opt_bool(obj, i, "dropdown")?;
    check_dv_fields(
        type_,
        operator,
        formula1.as_deref(),
        formula2.as_deref(),
        dropdown,
        i,
    )?;
    Ok(InsertDataValidationOp {
        sheet,
        sqref,
        type_,
        operator,
        formula1,
        formula2,
        allow_blank: parse_opt_bool(obj, i, "allowBlank")?.unwrap_or(true),
        dropdown,
        show_input_message: parse_opt_bool(obj, i, "showInputMessage")?.unwrap_or(true),
        show_error_message: parse_opt_bool(obj, i, "showErrorMessage")?.unwrap_or(true),
        error_style: parse_dv_error_style_opt(obj.get("errorStyle"), i)?,
        error_title: parse_dv_string_opt(obj.get("errorTitle"), i, "errorTitle")?,
        error: parse_dv_string_opt(obj.get("error"), i, "error")?,
        prompt_title: parse_dv_string_opt(obj.get("promptTitle"), i, "promptTitle")?,
        prompt: parse_dv_string_opt(obj.get("prompt"), i, "prompt")?,
    })
}

fn parse_set_data_validation(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetDataValidationOp, Error> {
    deny_keys(obj, i, DV_KEYS)?;
    let id = require_id(obj, i)?;
    let (sheet, index) = parse_dv_id(&id, i)?;
    let sqref = if obj.contains_key("sqref") {
        Some(parse_sqref_field(obj.get("sqref"), i, true)?)
    } else {
        None
    };
    let type_ = parse_dv_type_field(obj.get("type"), i, false)?;
    let operator = parse_dv_operator_patch(obj.get("operator"), i)?;
    let formula1 = parse_dv_formula_patch(obj.get("formula1"), i, "formula1")?;
    let formula2 = parse_dv_formula_patch(obj.get("formula2"), i, "formula2")?;
    let op = SetDataValidationOp {
        sheet,
        index,
        sqref,
        type_,
        operator,
        formula1,
        formula2,
        allow_blank: parse_opt_bool(obj, i, "allowBlank")?,
        dropdown: parse_opt_bool(obj, i, "dropdown")?,
        show_input_message: parse_opt_bool(obj, i, "showInputMessage")?,
        show_error_message: parse_opt_bool(obj, i, "showErrorMessage")?,
        error_style: parse_dv_error_style_patch(obj.get("errorStyle"), i)?,
        error_title: parse_dv_string_patch(obj.get("errorTitle"), i, "errorTitle")?,
        error: parse_dv_string_patch(obj.get("error"), i, "error")?,
        prompt_title: parse_dv_string_patch(obj.get("promptTitle"), i, "promptTitle")?,
        prompt: parse_dv_string_patch(obj.get("prompt"), i, "prompt")?,
    };
    if !op.has_patch() {
        return Err(Error::with_path(
            ErrorKind::NeedSetDataValidationField,
            format!("ops[{i}]"),
        ));
    }
    Ok(op)
}

fn parse_delete_data_validation(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteDataValidationOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let (sheet, index) = parse_dv_id(&id, i)?;
    Ok(DeleteDataValidationOp { sheet, index })
}

const CF_KEYS: &[&str] = &[
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

const ICON_SETS: &[&str] = &[
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

const CFVO_TYPES: &[&str] = &["num", "percent", "max", "min", "formula", "percentile"];

pub(crate) fn icon_set_count(set: &str) -> usize {
    if set.starts_with('5') {
        5
    } else if set.starts_with('4') {
        4
    } else {
        3
    }
}

fn parse_insert_conditional_format(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertConditionalFormatOp, Error> {
    deny_keys(obj, i, CF_KEYS)?;
    let id = require_id(obj, i)?;
    let sheet = parse_sheet_only_id(&id, i, "id", IdReason::SheetOpNeedsSheet)?;
    let sqref = parse_sqref_field(obj.get("sqref"), i, true)?;
    let type_ = parse_cf_type_field(obj.get("type"), i, true)?.ok_or_else(|| {
        Error::with_path(
            ErrorKind::MissingField {
                field: "type".into(),
            },
            format!("ops[{i}].type"),
        )
    })?;
    let operator = parse_cf_operator_field(obj.get("operator"), i, false)?;
    let formulas = parse_cf_formulas_opt(obj.get("formulas"), i)?;
    let dxf = parse_dxf_opt(obj.get("dxf"), i)?;
    let color_scale = parse_color_scale_opt(obj.get("colorScale"), i)?;
    let data_bar = parse_data_bar_opt(obj.get("dataBar"), i)?;
    let icon_set = parse_icon_set_opt(obj.get("iconSet"), i)?;
    let priority = parse_cf_priority_opt(obj.get("priority"), i)?;
    let stop_if_true = parse_opt_bool(obj, i, "stopIfTrue")?.unwrap_or(false);
    let text = parse_cf_text_opt(obj.get("text"), i)?;
    let rank = parse_opt_u32_top(obj, i, "rank", "rank", 1, u32::MAX)?;
    let percent = parse_opt_bool(obj, i, "percent")?;
    let bottom = parse_opt_bool(obj, i, "bottom")?;
    let above_average = parse_opt_bool(obj, i, "aboveAverage")?;
    let equal_average = parse_opt_bool(obj, i, "equalAverage")?;
    let std_dev = parse_cf_std_dev_opt(obj.get("stdDev"), i)?;
    check_cf_fields(
        type_,
        operator,
        formulas.as_deref(),
        dxf.as_ref(),
        color_scale.as_ref(),
        data_bar.as_ref(),
        icon_set.as_ref(),
        CfExtraCheck {
            text: text.as_deref(),
            rank,
            percent,
            bottom,
            above_average,
            equal_average,
            std_dev,
        },
        i,
        true,
    )?;
    Ok(InsertConditionalFormatOp {
        sheet,
        sqref,
        type_,
        operator,
        formulas,
        dxf,
        color_scale,
        data_bar,
        icon_set,
        priority,
        stop_if_true,
        text,
        rank,
        percent,
        bottom,
        above_average,
        equal_average,
        std_dev,
    })
}

fn parse_set_conditional_format(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetConditionalFormatOp, Error> {
    deny_keys(obj, i, CF_KEYS)?;
    let id = require_id(obj, i)?;
    let (sheet, index) = parse_cf_id(&id, i)?;
    let sqref = if obj.contains_key("sqref") {
        Some(parse_sqref_field(obj.get("sqref"), i, true)?)
    } else {
        None
    };
    let op = SetConditionalFormatOp {
        sheet,
        index,
        sqref,
        type_: parse_cf_type_field(obj.get("type"), i, false)?,
        operator: parse_cf_operator_patch(obj.get("operator"), i)?,
        formulas: parse_cf_formulas_patch(obj.get("formulas"), i)?,
        dxf: parse_dxf_patch_field(obj.get("dxf"), i)?,
        color_scale: parse_color_scale_patch(obj.get("colorScale"), i)?,
        data_bar: parse_data_bar_patch(obj.get("dataBar"), i)?,
        icon_set: parse_icon_set_patch(obj.get("iconSet"), i)?,
        priority: parse_cf_priority_opt(obj.get("priority"), i)?,
        stop_if_true: parse_opt_bool(obj, i, "stopIfTrue")?,
        text: parse_cf_text_patch(obj.get("text"), i)?,
        rank: parse_cf_rank_patch(obj.get("rank"), i)?,
        percent: parse_opt_bool(obj, i, "percent")?,
        bottom: parse_opt_bool(obj, i, "bottom")?,
        above_average: parse_cf_bool_patch(obj.get("aboveAverage"), i, "aboveAverage")?,
        equal_average: parse_opt_bool(obj, i, "equalAverage")?,
        std_dev: parse_cf_std_dev_patch(obj.get("stdDev"), i)?,
    };
    if !op.has_patch() {
        return Err(Error::with_path(
            ErrorKind::NeedSetConditionalFormatField,
            format!("ops[{i}]"),
        ));
    }
    Ok(op)
}

fn parse_delete_conditional_format(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteConditionalFormatOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let (sheet, index) = parse_cf_id(&id, i)?;
    Ok(DeleteConditionalFormatOp { sheet, index })
}

fn parse_insert_sheet_table(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertSheetTableOp, Error> {
    deny_keys(obj, i, &["op", "id", "ref", "name", "columns"])?;
    let id = require_id(obj, i)?;
    let sheet = parse_sheet_only_id(&id, i, "id", IdReason::SheetOpNeedsSheet)?;
    let range = parse_table_ref(obj.get("ref"), i, true)?.ok_or_else(|| {
        Error::with_path(
            ErrorKind::MissingField {
                field: "ref".into(),
            },
            format!("ops[{i}].ref"),
        )
    })?;
    Ok(InsertSheetTableOp {
        sheet,
        range,
        name: parse_table_name(obj.get("name"), i)?,
        columns: parse_table_columns(obj.get("columns"), i)?,
    })
}

fn parse_set_sheet_table(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetSheetTableOp, Error> {
    deny_keys(obj, i, &["op", "id", "ref", "name", "columns"])?;
    let id = require_id(obj, i)?;
    let (sheet, index) = parse_table_id(&id, i)?;
    let op = SetSheetTableOp {
        sheet,
        index,
        range: parse_table_ref(obj.get("ref"), i, false)?,
        name: parse_table_name(obj.get("name"), i)?,
        columns: parse_table_columns(obj.get("columns"), i)?,
    };
    if !op.has_patch() {
        return Err(Error::with_path(
            ErrorKind::NeedSetSheetTableField,
            format!("ops[{i}]"),
        ));
    }
    Ok(op)
}

fn parse_delete_sheet_table(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteSheetTableOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let (sheet, index) = parse_table_id(&id, i)?;
    Ok(DeleteSheetTableOp { sheet, index })
}

fn parse_insert_pivot_table(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertPivotTableOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op", "id", "source", "location", "name", "rows", "cols", "filters", "values",
        ],
    )?;
    let id = require_id(obj, i)?;
    let sheet = parse_sheet_only_id(&id, i, "id", IdReason::SheetOpNeedsSheet)?;
    let source = parse_pivot_source(obj.get("source"), i, true)?.ok_or_else(|| {
        Error::with_path(
            ErrorKind::MissingField {
                field: "source".into(),
            },
            format!("ops[{i}].source"),
        )
    })?;
    let location = parse_pivot_location(obj.get("location"), i, true)?.ok_or_else(|| {
        Error::with_path(
            ErrorKind::MissingField {
                field: "location".into(),
            },
            format!("ops[{i}].location"),
        )
    })?;
    let values = parse_pivot_fields(obj.get("values"), i, "values", true)?.ok_or_else(|| {
        Error::with_path(
            ErrorKind::MissingField {
                field: "values".into(),
            },
            format!("ops[{i}].values"),
        )
    })?;
    if values.is_empty() {
        return Err(Error::with_path(
            ErrorKind::InvalidPivotTable {
                field: "values".into(),
            },
            format!("ops[{i}].values"),
        ));
    }
    Ok(InsertPivotTableOp {
        sheet,
        source,
        location,
        name: parse_pivot_name(obj.get("name"), i)?,
        rows: parse_pivot_fields(obj.get("rows"), i, "rows", false)?.unwrap_or_default(),
        cols: parse_pivot_fields(obj.get("cols"), i, "cols", false)?.unwrap_or_default(),
        filters: parse_pivot_fields(obj.get("filters"), i, "filters", false)?.unwrap_or_default(),
        values,
    })
}

fn parse_set_pivot_table(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetPivotTableOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op", "id", "source", "location", "name", "rows", "cols", "filters", "values",
        ],
    )?;
    let id = require_id(obj, i)?;
    let (sheet, index) = parse_pivot_id(&id, i)?;
    let values = parse_pivot_fields(obj.get("values"), i, "values", false)?;
    if values.as_ref().is_some_and(|v| v.is_empty()) {
        return Err(Error::with_path(
            ErrorKind::InvalidPivotTable {
                field: "values".into(),
            },
            format!("ops[{i}].values"),
        ));
    }
    let op = SetPivotTableOp {
        sheet,
        index,
        source: parse_pivot_source(obj.get("source"), i, false)?,
        location: parse_pivot_location(obj.get("location"), i, false)?,
        name: parse_pivot_name(obj.get("name"), i)?,
        rows: parse_pivot_fields(obj.get("rows"), i, "rows", false)?,
        cols: parse_pivot_fields(obj.get("cols"), i, "cols", false)?,
        filters: parse_pivot_fields(obj.get("filters"), i, "filters", false)?,
        values,
    };
    if !op.has_patch() {
        return Err(Error::with_path(
            ErrorKind::NeedSetPivotTableField,
            format!("ops[{i}]"),
        ));
    }
    Ok(op)
}

fn parse_delete_pivot_table(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeletePivotTableOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let (sheet, index) = parse_pivot_id(&id, i)?;
    Ok(DeletePivotTableOp { sheet, index })
}

fn parse_insert_defined_name(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertDefinedNameOp, Error> {
    deny_keys(
        obj,
        i,
        &["op", "id", "name", "formula", "hidden", "localSheet"],
    )?;
    let id = require_id(obj, i)?;
    if id != "xlsx:workbook" {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::InsertDefinedNameNeedsWorkbook,
            },
            format!("ops[{i}].id"),
        ));
    }
    let name = parse_defined_name_name(obj.get("name"), i)?.ok_or_else(|| {
        Error::with_path(
            ErrorKind::MissingField {
                field: "name".into(),
            },
            format!("ops[{i}].name"),
        )
    })?;
    let formula = parse_defined_name_formula(obj.get("formula"), i)?.ok_or_else(|| {
        Error::with_path(
            ErrorKind::MissingField {
                field: "formula".into(),
            },
            format!("ops[{i}].formula"),
        )
    })?;
    let local_sheet = match obj.get("localSheet") {
        None => None,
        Some(Value::Null) => {
            return Err(Error::with_path(
                ErrorKind::InvalidDefinedName {
                    field: "localSheet".into(),
                },
                format!("ops[{i}].localSheet"),
            ));
        }
        Some(Value::String(s)) => {
            if s.is_empty() {
                return Err(Error::with_path(
                    ErrorKind::InvalidDefinedName {
                        field: "localSheet".into(),
                    },
                    format!("ops[{i}].localSheet"),
                ));
            }
            Some(s.clone())
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "localSheet".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].localSheet"),
            ));
        }
    };
    Ok(InsertDefinedNameOp {
        name,
        formula,
        hidden: parse_opt_bool(obj, i, "hidden")?,
        local_sheet,
    })
}

fn parse_set_defined_name(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetDefinedNameOp, Error> {
    deny_keys(
        obj,
        i,
        &["op", "id", "name", "formula", "hidden", "localSheet"],
    )?;
    let id = require_id(obj, i)?;
    let index = parse_defined_name_id(&id, i)?;
    if let Some(Value::Null) = obj.get("hidden") {
        return Err(Error::with_path(
            ErrorKind::InvalidDefinedName {
                field: "hidden".into(),
            },
            format!("ops[{i}].hidden"),
        ));
    }
    let op = SetDefinedNameOp {
        index,
        name: parse_defined_name_name(obj.get("name"), i)?,
        formula: parse_defined_name_formula(obj.get("formula"), i)?,
        hidden: parse_opt_bool(obj, i, "hidden")?,
        local_sheet: parse_set_local_sheet(obj, i)?,
    };
    if !op.has_patch() {
        return Err(Error::with_path(
            ErrorKind::NeedSetDefinedNameField,
            format!("ops[{i}]"),
        ));
    }
    Ok(op)
}

fn parse_delete_defined_name(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteDefinedNameOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    Ok(DeleteDefinedNameOp {
        index: parse_defined_name_id(&id, i)?,
    })
}

fn parse_set_print_setup(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetPrintSetupOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op",
            "id",
            "left",
            "right",
            "top",
            "bottom",
            "header",
            "footer",
            "paperSize",
            "orientation",
            "scale",
            "fitToPage",
            "fitToWidth",
            "fitToHeight",
            "gridLines",
            "headings",
            "hCenter",
            "vCenter",
            "printArea",
            "rowBreaks",
            "colBreaks",
        ],
    )?;
    let id = require_id(obj, i)?;
    let sheet = parse_sheet_only_id(&id, i, "id", IdReason::SheetOpNeedsSheet)?;
    let op = SetPrintSetupOp {
        sheet,
        left: parse_print_inch(obj, i, "left")?,
        right: parse_print_inch(obj, i, "right")?,
        top: parse_print_inch(obj, i, "top")?,
        bottom: parse_print_inch(obj, i, "bottom")?,
        header: parse_print_inch(obj, i, "header")?,
        footer: parse_print_inch(obj, i, "footer")?,
        paper_size: parse_print_u32(obj, i, "paperSize", 1, u32::MAX)?,
        orientation: parse_print_orientation(obj, i)?,
        scale: parse_print_u32(obj, i, "scale", 10, 400)?,
        fit_to_page: parse_opt_bool(obj, i, "fitToPage")?,
        fit_to_width: parse_print_u32(obj, i, "fitToWidth", 0, u32::MAX)?,
        fit_to_height: parse_print_u32(obj, i, "fitToHeight", 0, u32::MAX)?,
        grid_lines: parse_opt_bool(obj, i, "gridLines")?,
        headings: parse_opt_bool(obj, i, "headings")?,
        h_center: parse_opt_bool(obj, i, "hCenter")?,
        v_center: parse_opt_bool(obj, i, "vCenter")?,
        print_area: parse_print_area(obj, i)?,
        row_breaks: parse_print_breaks(obj, i, "rowBreaks", 1_048_576)?,
        col_breaks: parse_print_breaks(obj, i, "colBreaks", 16_384)?,
    };
    if !op.has_patch() {
        return Err(Error::with_path(
            ErrorKind::NeedSetPrintSetupField,
            format!("ops[{i}]"),
        ));
    }
    Ok(op)
}

fn parse_set_sheet_view(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetSheetViewOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op",
            "id",
            "zoom",
            "frozenRows",
            "frozenCols",
            "splitRows",
            "splitCols",
            "activeCell",
        ],
    )?;
    let id = require_id(obj, i)?;
    let sheet = parse_sheet_only_id(&id, i, "id", IdReason::SheetOpNeedsSheet)?;
    let op = SetSheetViewOp {
        sheet,
        zoom: parse_view_u32(obj, i, "zoom", 10, 400)?,
        frozen_rows: parse_view_u32(obj, i, "frozenRows", 0, 1_048_575)?,
        frozen_cols: parse_view_u32(obj, i, "frozenCols", 0, 16_383)?,
        split_rows: parse_view_u32(obj, i, "splitRows", 0, 1_048_575)?,
        split_cols: parse_view_u32(obj, i, "splitCols", 0, 16_383)?,
        active_cell: parse_view_cell(obj, i)?,
    };
    if op.has_freeze() && op.has_split() {
        let field = if op.split_rows.is_some() {
            "splitRows"
        } else {
            "splitCols"
        };
        return Err(Error::with_path(
            ErrorKind::InvalidSheetView {
                field: field.into(),
            },
            format!("ops[{i}].{field}"),
        ));
    }
    if !op.has_patch() {
        return Err(Error::with_path(
            ErrorKind::NeedSetSheetViewField,
            format!("ops[{i}]"),
        ));
    }
    Ok(op)
}

fn parse_set_auto_filter(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetAutoFilterOp, Error> {
    deny_keys(obj, i, &["op", "id", "ref"])?;
    let id = require_id(obj, i)?;
    let sheet = parse_sheet_only_id(&id, i, "id", IdReason::SheetOpNeedsSheet)?;
    let refer = match obj.get("ref") {
        None => {
            return Err(Error::with_path(
                ErrorKind::NeedSetAutoFilterField,
                format!("ops[{i}]"),
            ));
        }
        Some(Value::Null) => None,
        Some(Value::String(s)) => {
            if s.trim().is_empty() {
                return Err(Error::with_path(
                    ErrorKind::InvalidAutoFilter {
                        field: "ref".into(),
                    },
                    format!("ops[{i}].ref"),
                ));
            }
            Some(s.clone())
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "ref".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].ref"),
            ));
        }
    };
    Ok(SetAutoFilterOp { sheet, refer })
}

fn parse_set_sort_state(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetSortStateOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op",
            "id",
            "ref",
            "caseSensitive",
            "columnSort",
            "sortMethod",
            "columns",
        ],
    )?;
    let id = require_id(obj, i)?;
    let sheet = parse_sheet_only_id(&id, i, "id", IdReason::SheetOpNeedsSheet)?;
    let refer = match obj.get("ref") {
        None => {
            return Err(Error::with_path(
                ErrorKind::NeedSetSortStateField,
                format!("ops[{i}]"),
            ));
        }
        Some(Value::Null) => None,
        Some(Value::String(s)) => {
            if s.trim().is_empty() {
                return Err(Error::with_path(
                    ErrorKind::InvalidSortState {
                        field: "ref".into(),
                    },
                    format!("ops[{i}].ref"),
                ));
            }
            Some(s.clone())
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "ref".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].ref"),
            ));
        }
    };
    let Some(raw_ref) = refer.as_deref() else {
        for name in ["caseSensitive", "columnSort", "sortMethod", "columns"] {
            if obj.contains_key(name) {
                return Err(Error::with_path(
                    ErrorKind::InvalidSortState { field: name.into() },
                    format!("ops[{i}].{name}"),
                ));
            }
        }
        return Ok(SetSortStateOp {
            sheet,
            refer: None,
            case_sensitive: false,
            column_sort: false,
            sort_method: None,
            columns: Vec::new(),
        });
    };
    let (outer_start, outer_end) =
        a1::parse_range(raw_ref).map_err(|e| e.at(format!("ops[{i}].ref")))?;
    let formatted = a1::format_range(outer_start, outer_end);
    let case_sensitive = parse_opt_bool(obj, i, "caseSensitive")?.unwrap_or(false);
    let column_sort = parse_opt_bool(obj, i, "columnSort")?.unwrap_or(false);
    let sort_method = parse_sort_method(obj, i)?;
    let columns_v = obj
        .get("columns")
        .ok_or_else(|| Error::with_path(ErrorKind::NeedSetSortStateField, format!("ops[{i}]")))?;
    let arr = columns_v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "columns".into(),
                expected: TypeName::Array,
            },
            format!("ops[{i}].columns"),
        )
    })?;
    if arr.is_empty() {
        return Err(Error::with_path(
            ErrorKind::EmptyArray {
                field: "columns".into(),
            },
            format!("ops[{i}].columns"),
        ));
    }
    let mut columns = Vec::new();
    for (j, item) in arr.iter().enumerate() {
        let item_obj = item.as_object().ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: format!("columns[{j}]"),
                    expected: TypeName::Object,
                },
                format!("ops[{i}].columns[{j}]"),
            )
        })?;
        deny_keys_at(
            item_obj,
            i,
            &format!("columns[{j}]"),
            &[
                "ref",
                "descending",
                "sortBy",
                "customList",
                "dxfId",
                "color",
                "iconSet",
                "iconId",
            ],
        )?;
        let raw = item_obj
            .get("ref")
            .and_then(Value::as_str)
            .filter(|s| !s.trim().is_empty())
            .ok_or_else(|| {
                Error::with_path(
                    ErrorKind::InvalidSortState {
                        field: "ref".into(),
                    },
                    format!("ops[{i}].columns[{j}].ref"),
                )
            })?;
        let (start, end) =
            a1::parse_range(raw).map_err(|e| e.at(format!("ops[{i}].columns[{j}].ref")))?;
        if !a1::range_contains((outer_start, outer_end), (start, end)) {
            return Err(Error::with_path(
                ErrorKind::InvalidSortState {
                    field: "ref".into(),
                },
                format!("ops[{i}].columns[{j}].ref"),
            ));
        }
        let descending = match item_obj.get("descending") {
            None => false,
            Some(Value::Bool(b)) => *b,
            Some(_) => {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: "descending".into(),
                        expected: TypeName::Boolean,
                    },
                    format!("ops[{i}].columns[{j}].descending"),
                ));
            }
        };
        columns.push(parse_sort_condition(
            item_obj,
            i,
            j,
            a1::format_range(start, end),
            descending,
        )?);
    }
    Ok(SetSortStateOp {
        sheet,
        refer: Some(formatted),
        case_sensitive,
        column_sort,
        sort_method,
        columns,
    })
}

fn parse_sort_method(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<Option<String>, Error> {
    match obj.get("sortMethod") {
        None => Ok(None),
        Some(Value::String(s)) if s == "none" => Ok(None),
        Some(Value::String(s)) if s == "pinYin" || s == "stroke" => Ok(Some(s.clone())),
        Some(Value::String(_)) => Err(Error::with_path(
            ErrorKind::InvalidSortState {
                field: "sortMethod".into(),
            },
            format!("ops[{i}].sortMethod"),
        )),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "sortMethod".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].sortMethod"),
        )),
    }
}

fn parse_sort_condition(
    item_obj: &serde_json::Map<String, Value>,
    i: usize,
    j: usize,
    refer: String,
    descending: bool,
) -> Result<SortConditionOp, Error> {
    let loc = format!("columns[{j}]");
    let sort_by = match item_obj.get("sortBy") {
        None => None,
        Some(Value::String(s)) if s == "value" => None,
        Some(Value::String(s)) if s == "cellColor" || s == "fontColor" || s == "icon" => {
            Some(s.clone())
        }
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidSortState {
                    field: "sortBy".into(),
                },
                format!("ops[{i}].{loc}.sortBy"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "sortBy".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].{loc}.sortBy"),
            ));
        }
    };
    let custom_list = match item_obj.get("customList") {
        None => None,
        Some(Value::String(s)) if !s.is_empty() => Some(s.clone()),
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidSortState {
                    field: "customList".into(),
                },
                format!("ops[{i}].{loc}.customList"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "customList".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].{loc}.customList"),
            ));
        }
    };
    let color = match item_obj.get("color") {
        None => None,
        Some(Value::Object(m)) => Some(parse_color_spec(m, i, &format!("{loc}.color"))?),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "color".into(),
                    expected: TypeName::Object,
                },
                format!("ops[{i}].{loc}.color"),
            ));
        }
    };
    let dxf_id = parse_opt_sort_u32(item_obj, i, &loc, "dxfId")?;
    let icon_set = match item_obj.get("iconSet") {
        None => None,
        Some(Value::String(s)) if ICON_SETS.contains(&s.as_str()) => Some(s.clone()),
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidSortState {
                    field: "iconSet".into(),
                },
                format!("ops[{i}].{loc}.iconSet"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "iconSet".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].{loc}.iconSet"),
            ));
        }
    };
    let icon_id = parse_opt_sort_u32(item_obj, i, &loc, "iconId")?;
    match sort_by.as_deref() {
        None => {
            reject_sort_fields(
                i,
                &loc,
                &[
                    ("color", color.is_some()),
                    ("dxfId", dxf_id.is_some()),
                    ("iconSet", icon_set.is_some()),
                    ("iconId", icon_id.is_some()),
                ],
            )?;
        }
        Some("cellColor") | Some("fontColor") => {
            reject_sort_fields(
                i,
                &loc,
                &[
                    ("customList", custom_list.is_some()),
                    ("iconSet", icon_set.is_some()),
                    ("iconId", icon_id.is_some()),
                ],
            )?;
            if color.is_some() == dxf_id.is_some() {
                return Err(Error::with_path(
                    ErrorKind::InvalidSortState {
                        field: "color".into(),
                    },
                    format!("ops[{i}].{loc}.color"),
                ));
            }
        }
        Some("icon") => {
            reject_sort_fields(
                i,
                &loc,
                &[
                    ("customList", custom_list.is_some()),
                    ("color", color.is_some()),
                    ("dxfId", dxf_id.is_some()),
                ],
            )?;
        }
        Some(_) => unreachable!("sortBy 已在上面收窄"),
    }
    Ok(SortConditionOp {
        refer,
        descending,
        sort_by,
        custom_list,
        dxf_id,
        color,
        icon_set,
        icon_id,
    })
}

fn reject_sort_fields(i: usize, loc: &str, fields: &[(&str, bool)]) -> Result<(), Error> {
    for (name, hit) in fields {
        if *hit {
            return Err(Error::with_path(
                ErrorKind::InvalidSortState {
                    field: (*name).into(),
                },
                format!("ops[{i}].{loc}.{name}"),
            ));
        }
    }
    Ok(())
}

fn parse_opt_sort_u32(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    loc: &str,
    field: &str,
) -> Result<Option<u32>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::Number(n)) => {
            if let Some(u) = n.as_u64().filter(|v| *v <= u32::MAX as u64) {
                Ok(Some(u as u32))
            } else {
                Err(Error::with_path(
                    ErrorKind::InvalidSortState {
                        field: field.into(),
                    },
                    format!("ops[{i}].{loc}.{field}"),
                ))
            }
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Integer,
            },
            format!("ops[{i}].{loc}.{field}"),
        )),
    }
}

fn parse_group_rows(obj: &serde_json::Map<String, Value>, i: usize) -> Result<GroupRowsOp, Error> {
    deny_keys(obj, i, &["op", "id", "rows"])?;
    let id = require_id(obj, i)?;
    let sheet = parse_sheet_only_id(&id, i, "id", IdReason::SheetOpNeedsSheet)?;
    let (row, to_row) = parse_required_row_span(obj, i, "rows")?;
    Ok(GroupRowsOp { sheet, row, to_row })
}

fn parse_ungroup_rows(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<UngroupRowsOp, Error> {
    deny_keys(obj, i, &["op", "id", "rows"])?;
    let id = require_id(obj, i)?;
    let sheet = parse_sheet_only_id(&id, i, "id", IdReason::SheetOpNeedsSheet)?;
    let (row, to_row) = parse_required_row_span(obj, i, "rows")?;
    Ok(UngroupRowsOp { sheet, row, to_row })
}

fn parse_group_cols(obj: &serde_json::Map<String, Value>, i: usize) -> Result<GroupColsOp, Error> {
    deny_keys(obj, i, &["op", "id", "col"])?;
    let id = require_id(obj, i)?;
    let sheet = parse_sheet_only_id(&id, i, "id", IdReason::SheetOpNeedsSheet)?;
    let (min_col, max_col) = parse_required_col_span(obj, i, "col")?;
    Ok(GroupColsOp {
        sheet,
        min_col,
        max_col,
    })
}

fn parse_ungroup_cols(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<UngroupColsOp, Error> {
    deny_keys(obj, i, &["op", "id", "col"])?;
    let id = require_id(obj, i)?;
    let sheet = parse_sheet_only_id(&id, i, "id", IdReason::SheetOpNeedsSheet)?;
    let (min_col, max_col) = parse_required_col_span(obj, i, "col")?;
    Ok(UngroupColsOp {
        sheet,
        min_col,
        max_col,
    })
}

fn parse_set_outline(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetOutlineOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op",
            "id",
            "summaryBelow",
            "summaryRight",
            "collapseRows",
            "collapseCols",
            "collapsed",
        ],
    )?;
    let id = require_id(obj, i)?;
    let sheet = parse_sheet_only_id(&id, i, "id", IdReason::SheetOpNeedsSheet)?;
    let summary_below = parse_opt_bool(obj, i, "summaryBelow")?;
    let summary_right = parse_opt_bool(obj, i, "summaryRight")?;
    let collapse_rows = parse_opt_row_span(obj, i, "collapseRows")?;
    let collapse_cols = parse_opt_col_span(obj, i, "collapseCols")?;
    let collapsed = parse_opt_bool(obj, i, "collapsed")?;
    if summary_below.is_none()
        && summary_right.is_none()
        && collapse_rows.is_none()
        && collapse_cols.is_none()
    {
        return Err(Error::with_path(
            ErrorKind::NeedSetOutlineField,
            format!("ops[{i}]"),
        ));
    }
    if (collapse_rows.is_some() || collapse_cols.is_some()) && collapsed.is_none() {
        return Err(Error::with_path(
            ErrorKind::InvalidOutline {
                field: "collapsed".into(),
            },
            format!("ops[{i}].collapsed"),
        ));
    }
    if collapsed.is_some() && collapse_rows.is_none() && collapse_cols.is_none() {
        return Err(Error::with_path(
            ErrorKind::InvalidOutline {
                field: "collapsed".into(),
            },
            format!("ops[{i}].collapsed"),
        ));
    }
    Ok(SetOutlineOp {
        sheet,
        summary_below,
        summary_right,
        collapse_rows,
        collapse_cols,
        collapsed,
    })
}

fn parse_set_row_hidden(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetRowHiddenOp, Error> {
    deny_keys(obj, i, &["op", "id", "rows", "hidden"])?;
    let id = require_id(obj, i)?;
    let sheet = parse_sheet_only_id(&id, i, "id", IdReason::SheetOpNeedsSheet)?;
    let (row, to_row) = parse_required_row_span(obj, i, "rows")?;
    let hidden = parse_required_bool(obj.get("hidden"), i, "setRowHidden", "hidden")?;
    Ok(SetRowHiddenOp {
        sheet,
        row,
        to_row,
        hidden,
    })
}

fn parse_set_col_hidden(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetColHiddenOp, Error> {
    deny_keys(obj, i, &["op", "id", "col", "hidden"])?;
    let id = require_id(obj, i)?;
    let sheet = parse_sheet_only_id(&id, i, "id", IdReason::SheetOpNeedsSheet)?;
    let (min_col, max_col) = parse_required_col_span(obj, i, "col")?;
    let hidden = parse_required_bool(obj.get("hidden"), i, "setColHidden", "hidden")?;
    Ok(SetColHiddenOp {
        sheet,
        min_col,
        max_col,
        hidden,
    })
}

fn parse_required_row_span(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    field: &str,
) -> Result<(u32, u32), Error> {
    let raw = match obj.get(field) {
        Some(Value::String(s)) => s,
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: field.into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].{field}"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: field.into(),
                },
                format!("ops[{i}].{field}"),
            ));
        }
    };
    a1::parse_row_span(raw).map_err(|e| e.at(format!("ops[{i}].{field}")))
}

fn parse_required_col_span(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    field: &str,
) -> Result<(u32, u32), Error> {
    let raw = match obj.get(field) {
        Some(Value::String(s)) => s,
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: field.into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].{field}"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: field.into(),
                },
                format!("ops[{i}].{field}"),
            ));
        }
    };
    a1::parse_col_span(raw).map_err(|e| e.at(format!("ops[{i}].{field}")))
}

fn parse_opt_row_span(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    field: &str,
) -> Result<Option<(u32, u32)>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::String(s)) => a1::parse_row_span(s)
            .map(Some)
            .map_err(|e| e.at(format!("ops[{i}].{field}"))),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].{field}"),
        )),
    }
}

fn parse_opt_col_span(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    field: &str,
) -> Result<Option<(u32, u32)>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::String(s)) => a1::parse_col_span(s)
            .map(Some)
            .map_err(|e| e.at(format!("ops[{i}].{field}"))),
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
    deny_keys(obj, i, &["op", "id", "text", "author", "initials", "date"])?;
    let id = require_id(obj, i)?;
    let (sheet, a1) = comments::parse_insert_cell(&id, i)?;
    Ok(InsertCommentOp {
        sheet,
        a1,
        text: parse_optional_text(obj, i)?,
        author: parse_comment_str(obj, i, "author", "ooxp")?,
        initials: parse_comment_str(obj, i, "initials", "oo")?,
        date: parse_optional_date(obj, i)?,
    })
}

fn parse_reply_comment(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<ReplyCommentOp, Error> {
    deny_keys(obj, i, &["op", "id", "text", "author", "initials", "date"])?;
    let id = require_id(obj, i)?;
    let target = comments::parse_comment_id(&id, &format!("ops[{i}].id"))?;
    if matches!(target.key, comments::CommentKey::Reply { .. }) {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::XlsxCommentForm,
            },
            format!("ops[{i}].id"),
        ));
    }
    Ok(ReplyCommentOp {
        id,
        text: parse_optional_text(obj, i)?,
        author: parse_comment_str(obj, i, "author", "ooxp")?,
        initials: parse_comment_str(obj, i, "initials", "oo")?,
        date: parse_optional_date(obj, i)?,
    })
}

fn parse_delete_comment(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteCommentOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let _ = comments::parse_comment_id(&id, &format!("ops[{i}].id"))?;
    Ok(DeleteCommentOp { id })
}

fn parse_set_comment_text(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetCommentTextOp, Error> {
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

fn parse_set_comment_meta(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetCommentMetaOp, Error> {
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

fn parse_set_comment_done(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetCommentDoneOp, Error> {
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

fn parse_insert_hyperlink(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertHyperlinkOp, Error> {
    deny_keys(
        obj,
        i,
        &["op", "id", "ref", "url", "location", "tooltip", "display"],
    )?;
    let id = require_id(obj, i)?;
    let url = parse_opt_target(obj, i, "url", ErrorKind::EmptyHyperlinkUrl)?;
    let location = parse_opt_target(obj, i, "location", ErrorKind::EmptyHyperlinkLocation)?;
    let tooltip = parse_opt_plain(obj, i, "tooltip")?;
    let display = parse_opt_plain(obj, i, "display")?;
    if let Some(drawing) = parse_drawing_hlink_host(&id, i)? {
        if obj.contains_key("ref") {
            return Err(Error::with_path(
                ErrorKind::UnknownField {
                    field: "ref".into(),
                },
                format!("ops[{i}].ref"),
            ));
        }
        if display.is_some() {
            return Err(Error::with_path(
                ErrorKind::UnknownField {
                    field: "display".into(),
                },
                format!("ops[{i}].display"),
            ));
        }
        match (url.is_some(), location.is_some()) {
            (true, false) | (false, true) => {}
            (true, true) => {
                return Err(Error::with_path(
                    ErrorKind::MutuallyExclusive {
                        a: "url".into(),
                        b: "location".into(),
                    },
                    format!("ops[{i}]"),
                ));
            }
            (false, false) => {
                return Err(Error::with_path(
                    ErrorKind::XlsxHyperlinkNeedTarget,
                    format!("ops[{i}]"),
                ));
            }
        }
        let sheet = drawing.sheet().to_string();
        return Ok(InsertHyperlinkOp {
            sheet,
            refer: String::new(),
            start: a1::CellRef { col: 1, row: 1 },
            end: a1::CellRef { col: 1, row: 1 },
            url,
            location,
            tooltip,
            display: None,
            drawing: Some(drawing),
        });
    }
    let sheet = parse_sheet_only_id(&id, i, "id", IdReason::InsertHyperlinkNeedsSheet)?;
    let (refer, start, end) = parse_required_range(obj, i)?;
    if url.is_none() && location.is_none() {
        return Err(Error::with_path(
            ErrorKind::XlsxHyperlinkNeedTarget,
            format!("ops[{i}]"),
        ));
    }
    Ok(InsertHyperlinkOp {
        sheet,
        refer,
        start,
        end,
        url,
        location,
        tooltip,
        display,
        drawing: None,
    })
}

fn parse_drawing_hlink_host(id: &str, i: usize) -> Result<Option<DrawingHlinkHost>, Error> {
    if id.contains("/pic:") {
        return Ok(Some(DrawingHlinkHost::Picture(picture::parse_pic_id(
            id, i,
        )?)));
    }
    if id.contains("/smartdiagram:") {
        return Ok(Some(DrawingHlinkHost::SmartArt(
            xlsx_smartart::parse_diagram_id(id, i)?,
        )));
    }
    if id.contains("/shape:") {
        return Ok(Some(DrawingHlinkHost::Shape(shape::parse_shape_id(id, i)?)));
    }
    if id.contains("/sp:") {
        return Ok(Some(DrawingHlinkHost::TextBox(shape::parse_sp_id(id, i)?)));
    }
    Ok(None)
}

fn parse_set_hyperlink(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetHyperlinkOp, Error> {
    deny_keys(
        obj,
        i,
        &["op", "id", "url", "location", "tooltip", "display", "ref"],
    )?;
    let id = require_id(obj, i)?;
    let url = parse_hyperlink_patch(obj, i, "url", Some(ErrorKind::EmptyHyperlinkUrl))?;
    let location =
        parse_hyperlink_patch(obj, i, "location", Some(ErrorKind::EmptyHyperlinkLocation))?;
    let tooltip = parse_hyperlink_patch(obj, i, "tooltip", None)?;
    let display = parse_hyperlink_patch(obj, i, "display", None)?;
    if let Some(drawing) = parse_drawing_hlink_host(&id, i)? {
        if obj.contains_key("ref") {
            return Err(Error::with_path(
                ErrorKind::UnknownField {
                    field: "ref".into(),
                },
                format!("ops[{i}].ref"),
            ));
        }
        if !matches!(display, Patch::Absent) {
            return Err(Error::with_path(
                ErrorKind::UnknownField {
                    field: "display".into(),
                },
                format!("ops[{i}].display"),
            ));
        }
        if matches!(url, Patch::Set(_)) && matches!(location, Patch::Set(_)) {
            return Err(Error::with_path(
                ErrorKind::MutuallyExclusive {
                    a: "url".into(),
                    b: "location".into(),
                },
                format!("ops[{i}]"),
            ));
        }
        if url.is_absent() && location.is_absent() && tooltip.is_absent() {
            return Err(Error::with_path(
                ErrorKind::NeedXlsxSetHyperlinkField,
                format!("ops[{i}]"),
            ));
        }
        let sheet = drawing.sheet().to_string();
        return Ok(SetHyperlinkOp {
            id,
            sheet,
            index: 0,
            url,
            location,
            tooltip,
            display: Patch::Absent,
            refer: None,
            drawing: Some(drawing),
        });
    }
    let (sheet, index) = parse_link_id(&id, i)?;
    let refer = parse_opt_range(obj, i)?;
    if url.is_absent()
        && location.is_absent()
        && tooltip.is_absent()
        && display.is_absent()
        && refer.is_none()
    {
        return Err(Error::with_path(
            ErrorKind::NeedXlsxSetHyperlinkField,
            format!("ops[{i}]"),
        ));
    }
    Ok(SetHyperlinkOp {
        id,
        sheet,
        index,
        url,
        location,
        tooltip,
        display,
        refer,
        drawing: None,
    })
}

fn parse_delete_hyperlink(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteHyperlinkOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    if let Some(drawing) = parse_drawing_hlink_host(&id, i)? {
        let sheet = drawing.sheet().to_string();
        return Ok(DeleteHyperlinkOp {
            id,
            sheet,
            index: 0,
            drawing: Some(drawing),
        });
    }
    let (sheet, index) = parse_link_id(&id, i)?;
    Ok(DeleteHyperlinkOp {
        id,
        sheet,
        index,
        drawing: None,
    })
}

const SPARK_KEYS: &[&str] = &[
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

fn parse_insert_sparkline(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertSparklineOp, Error> {
    deny_keys(obj, i, SPARK_KEYS)?;
    let id = require_id(obj, i)?;
    let sheet = parse_sheet_only_id(&id, i, "id", IdReason::InsertSparklineNeedsSheet)?;
    let (refer, host) = parse_required_spark_host(obj, i)?;
    let formula = parse_required_spark_formula(obj, i)?;
    let min_axis = parse_spark_axis(obj, i, "minAxisType")?;
    let max_axis = parse_spark_axis(obj, i, "maxAxisType")?;
    let manual_min = parse_opt_f64_top(obj, i, "manualMin")?;
    let manual_max = parse_opt_f64_top(obj, i, "manualMax")?;
    check_spark_axis(min_axis, max_axis, manual_min, manual_max, i)?;
    Ok(InsertSparklineOp {
        sheet,
        refer,
        host,
        formula,
        kind: parse_spark_type(obj, i)?.unwrap_or(SparklineType::Line),
        display_empty: parse_spark_empty(obj, i)?.unwrap_or(SparkEmpty::Gap),
        markers: parse_opt_bool(obj, i, "markers")?.unwrap_or(false),
        high: parse_opt_bool(obj, i, "high")?.unwrap_or(false),
        low: parse_opt_bool(obj, i, "low")?.unwrap_or(false),
        first: parse_opt_bool(obj, i, "first")?.unwrap_or(false),
        last: parse_opt_bool(obj, i, "last")?.unwrap_or(false),
        negative: parse_opt_bool(obj, i, "negative")?.unwrap_or(false),
        display_x_axis: parse_opt_bool(obj, i, "displayXAxis")?.unwrap_or(false),
        line_weight: parse_spark_line_weight(obj, i)?,
        min_axis,
        max_axis,
        manual_min,
        manual_max,
        colors: parse_spark_colors(obj.get("colors"), i)?.unwrap_or_default(),
    })
}

fn parse_set_sparkline(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetSparklineOp, Error> {
    deny_keys(obj, i, SPARK_KEYS)?;
    let id = require_id(obj, i)?;
    let (sheet, index) = parse_spark_id(&id, i)?;
    let kind = parse_spark_type(obj, i)?;
    let refer = parse_opt_spark_host(obj, i)?;
    let formula = parse_opt_spark_formula(obj, i)?;
    let display_empty = parse_spark_empty(obj, i)?;
    let markers = parse_opt_bool(obj, i, "markers")?;
    let high = parse_opt_bool(obj, i, "high")?;
    let low = parse_opt_bool(obj, i, "low")?;
    let first = parse_opt_bool(obj, i, "first")?;
    let last = parse_opt_bool(obj, i, "last")?;
    let negative = parse_opt_bool(obj, i, "negative")?;
    let display_x_axis = parse_opt_bool(obj, i, "displayXAxis")?;
    let line_weight = parse_spark_line_weight_patch(obj, i)?;
    let min_axis = parse_spark_axis(obj, i, "minAxisType")?;
    let max_axis = parse_spark_axis(obj, i, "maxAxisType")?;
    let manual_min = parse_f64_patch(obj, i, "manualMin")?;
    let manual_max = parse_f64_patch(obj, i, "manualMax")?;
    let colors = parse_spark_colors(obj.get("colors"), i)?;
    let min_check = match &manual_min {
        Patch::Set(v) => Some(*v),
        _ => None,
    };
    let max_check = match &manual_max {
        Patch::Set(v) => Some(*v),
        _ => None,
    };
    if min_axis == Some(SparkAxisType::Custom) && min_check.is_none() {
        return Err(Error::with_path(
            ErrorKind::InvalidSparkline {
                field: "manualMin".into(),
            },
            format!("ops[{i}].manualMin"),
        ));
    }
    if max_axis == Some(SparkAxisType::Custom) && max_check.is_none() {
        return Err(Error::with_path(
            ErrorKind::InvalidSparkline {
                field: "manualMax".into(),
            },
            format!("ops[{i}].manualMax"),
        ));
    }
    if min_axis.is_some() && min_axis != Some(SparkAxisType::Custom) && min_check.is_some() {
        return Err(Error::with_path(
            ErrorKind::InvalidSparkline {
                field: "manualMin".into(),
            },
            format!("ops[{i}].manualMin"),
        ));
    }
    if max_axis.is_some() && max_axis != Some(SparkAxisType::Custom) && max_check.is_some() {
        return Err(Error::with_path(
            ErrorKind::InvalidSparkline {
                field: "manualMax".into(),
            },
            format!("ops[{i}].manualMax"),
        ));
    }
    if kind.is_none()
        && refer.is_none()
        && formula.is_absent()
        && display_empty.is_none()
        && markers.is_none()
        && high.is_none()
        && low.is_none()
        && first.is_none()
        && last.is_none()
        && negative.is_none()
        && display_x_axis.is_none()
        && line_weight.is_absent()
        && min_axis.is_none()
        && max_axis.is_none()
        && manual_min.is_absent()
        && manual_max.is_absent()
        && colors.is_none()
    {
        return Err(Error::with_path(
            ErrorKind::NeedSetSparklineField,
            format!("ops[{i}]"),
        ));
    }
    Ok(SetSparklineOp {
        id,
        sheet,
        index,
        kind,
        refer,
        formula,
        display_empty,
        markers,
        high,
        low,
        first,
        last,
        negative,
        display_x_axis,
        line_weight,
        min_axis,
        max_axis,
        manual_min,
        manual_max,
        colors,
    })
}

fn parse_delete_sparkline(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteSparklineOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let (sheet, index) = parse_spark_id(&id, i)?;
    Ok(DeleteSparklineOp { id, sheet, index })
}

fn parse_opt_password(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<Option<String>, Error> {
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
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetProtectionOp, Error> {
    let id = require_id(obj, i)?;
    if id == "xlsx:workbook" {
        deny_keys(
            obj,
            i,
            &[
                "op",
                "id",
                "lockStructure",
                "lockWindows",
                "lockRevision",
                "password",
            ],
        )?;
        let op = SetWorkbookProtectionOp {
            lock_structure: parse_opt_bool(obj, i, "lockStructure")?,
            lock_windows: parse_opt_bool(obj, i, "lockWindows")?,
            lock_revision: parse_opt_bool(obj, i, "lockRevision")?,
            password: parse_opt_password(obj, i)?,
        };
        if op.lock_structure.is_none() && op.lock_windows.is_none() && op.lock_revision.is_none() {
            return Err(Error::with_path(
                ErrorKind::NeedProtectionField,
                format!("ops[{i}]"),
            ));
        }
        return Ok(SetProtectionOp::Workbook(op));
    }
    deny_keys(
        obj,
        i,
        &[
            "op",
            "id",
            "enforced",
            "objects",
            "scenarios",
            "formatCells",
            "formatColumns",
            "formatRows",
            "insertColumns",
            "insertRows",
            "insertHyperlinks",
            "deleteColumns",
            "deleteRows",
            "selectLockedCells",
            "sort",
            "autoFilter",
            "pivotTables",
            "selectUnlockedCells",
            "password",
        ],
    )?;
    let sheet = parse_sheet_only_id(&id, i, "id", IdReason::SetProtectionNeedsSheetOrWorkbook)?;
    let op = SetSheetProtectionOp {
        sheet,
        enforced: parse_opt_bool(obj, i, "enforced")?,
        objects: parse_opt_bool(obj, i, "objects")?,
        scenarios: parse_opt_bool(obj, i, "scenarios")?,
        format_cells: parse_opt_bool(obj, i, "formatCells")?,
        format_columns: parse_opt_bool(obj, i, "formatColumns")?,
        format_rows: parse_opt_bool(obj, i, "formatRows")?,
        insert_columns: parse_opt_bool(obj, i, "insertColumns")?,
        insert_rows: parse_opt_bool(obj, i, "insertRows")?,
        insert_hyperlinks: parse_opt_bool(obj, i, "insertHyperlinks")?,
        delete_columns: parse_opt_bool(obj, i, "deleteColumns")?,
        delete_rows: parse_opt_bool(obj, i, "deleteRows")?,
        select_locked_cells: parse_opt_bool(obj, i, "selectLockedCells")?,
        sort: parse_opt_bool(obj, i, "sort")?,
        auto_filter: parse_opt_bool(obj, i, "autoFilter")?,
        pivot_tables: parse_opt_bool(obj, i, "pivotTables")?,
        select_unlocked_cells: parse_opt_bool(obj, i, "selectUnlockedCells")?,
        password: parse_opt_password(obj, i)?,
    };
    if op.enforced.is_none() && !op.has_flag() {
        return Err(Error::with_path(
            ErrorKind::NeedProtectionField,
            format!("ops[{i}]"),
        ));
    }
    Ok(SetProtectionOp::Sheet(op))
}

fn parse_insert_protected_range(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertProtectedRangeOp, Error> {
    deny_keys(obj, i, &["op", "id", "name", "sqref", "password"])?;
    let id = require_id(obj, i)?;
    let sheet = parse_sheet_only_id(&id, i, "id", IdReason::InsertProtectedRangeNeedsSheet)?;
    let name = parse_protected_range_name(obj.get("name"), i, true)?.ok_or_else(|| {
        Error::with_path(
            ErrorKind::MissingField {
                field: "name".into(),
            },
            format!("ops[{i}].name"),
        )
    })?;
    let sqref = parse_sqref_field(obj.get("sqref"), i, true)?;
    Ok(InsertProtectedRangeOp {
        sheet,
        name,
        sqref,
        password: parse_opt_password(obj, i)?,
    })
}

fn parse_set_protected_range(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetProtectedRangeOp, Error> {
    deny_keys(obj, i, &["op", "id", "name", "sqref", "password"])?;
    let id = require_id(obj, i)?;
    let (sheet, index) = parse_edit_range_id(&id, i)?;
    let sqref = if obj.contains_key("sqref") {
        Some(parse_sqref_field(obj.get("sqref"), i, true)?)
    } else {
        None
    };
    let op = SetProtectedRangeOp {
        sheet,
        index,
        name: parse_protected_range_name(obj.get("name"), i, false)?,
        sqref,
        password: parse_opt_password(obj, i)?,
    };
    if !op.has_patch() {
        return Err(Error::with_path(
            ErrorKind::NeedSetProtectedRangeField,
            format!("ops[{i}]"),
        ));
    }
    Ok(op)
}

fn parse_delete_protected_range(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteProtectedRangeOp, Error> {
    deny_keys(obj, i, &["op", "id", "password"])?;
    let id = require_id(obj, i)?;
    let (sheet, index) = parse_edit_range_id(&id, i)?;
    Ok(DeleteProtectedRangeOp {
        sheet,
        index,
        password: parse_opt_password(obj, i)?,
    })
}

fn parse_protected_range_name(
    v: Option<&Value>,
    i: usize,
    required: bool,
) -> Result<Option<String>, Error> {
    match v {
        None if required => Err(Error::with_path(
            ErrorKind::MissingField {
                field: "name".into(),
            },
            format!("ops[{i}].name"),
        )),
        None => Ok(None),
        Some(Value::Null) => Err(Error::with_path(
            ErrorKind::InvalidProtectedRangeName,
            format!("ops[{i}].name"),
        )),
        Some(Value::String(s)) => {
            let t = s.trim();
            if t.is_empty() || t.chars().count() > 255 {
                return Err(Error::with_path(
                    ErrorKind::InvalidProtectedRangeName,
                    format!("ops[{i}].name"),
                ));
            }
            Ok(Some(t.to_string()))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "name".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].name"),
        )),
    }
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

fn parse_comment_str(
    obj: &serde_json::Map<String, Value>,
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

fn parse_optional_date(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<Option<String>, Error> {
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

fn parse_view_u32(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    field: &str,
    min: u32,
    max: u32,
) -> Result<Option<u32>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::Number(n)) => {
            let v = n.as_u64().and_then(|x| u32::try_from(x).ok());
            let Some(v) = v.filter(|x| *x >= min && *x <= max) else {
                return Err(Error::with_path(
                    ErrorKind::InvalidSheetView {
                        field: field.into(),
                    },
                    format!("ops[{i}].{field}"),
                ));
            };
            Ok(Some(v))
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

fn parse_view_cell(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<Option<String>, Error> {
    match obj.get("activeCell") {
        None => Ok(None),
        Some(Value::String(s)) => {
            let cell = a1::parse_cell(s).map_err(|_| {
                Error::with_path(
                    ErrorKind::InvalidSheetView {
                        field: "activeCell".into(),
                    },
                    format!("ops[{i}].activeCell"),
                )
            })?;
            Ok(Some(a1::format_cell(cell.col, cell.row)))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "activeCell".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].activeCell"),
        )),
    }
}

fn parse_print_inch(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    field: &str,
) -> Result<Option<f64>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::Number(n)) => {
            let Some(v) = n.as_f64().filter(|x| x.is_finite() && *x >= 0.0) else {
                return Err(Error::with_path(
                    ErrorKind::InvalidPrintSetup {
                        field: field.into(),
                    },
                    format!("ops[{i}].{field}"),
                ));
            };
            Ok(Some(v))
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

fn parse_print_u32(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    field: &str,
    min: u32,
    max: u32,
) -> Result<Option<u32>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::Number(n)) => {
            let v = n.as_u64().and_then(|x| u32::try_from(x).ok());
            let Some(v) = v.filter(|x| *x >= min && *x <= max) else {
                return Err(Error::with_path(
                    ErrorKind::InvalidPrintSetup {
                        field: field.into(),
                    },
                    format!("ops[{i}].{field}"),
                ));
            };
            Ok(Some(v))
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

fn parse_print_orientation(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<Option<String>, Error> {
    match obj.get("orientation") {
        None => Ok(None),
        Some(Value::String(s)) if s == "portrait" || s == "landscape" => Ok(Some(s.clone())),
        Some(Value::String(_)) => Err(Error::with_path(
            ErrorKind::InvalidPrintSetup {
                field: "orientation".into(),
            },
            format!("ops[{i}].orientation"),
        )),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "orientation".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].orientation"),
        )),
    }
}

fn parse_print_area(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<Option<Option<String>>, Error> {
    match obj.get("printArea") {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(None)),
        Some(Value::String(s)) => {
            if s.trim().is_empty() {
                return Err(Error::with_path(
                    ErrorKind::InvalidPrintSetup {
                        field: "printArea".into(),
                    },
                    format!("ops[{i}].printArea"),
                ));
            }
            Ok(Some(Some(s.clone())))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "printArea".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].printArea"),
        )),
    }
}

fn parse_print_breaks(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    field: &str,
    max: u32,
) -> Result<Option<Vec<u32>>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::Array(arr)) => {
            let mut out = Vec::with_capacity(arr.len());
            for (j, v) in arr.iter().enumerate() {
                let Some(n) = v.as_u64().and_then(|x| u32::try_from(x).ok()) else {
                    return Err(Error::with_path(
                        ErrorKind::WrongType {
                            field: format!("{field}[{j}]"),
                            expected: TypeName::Integer,
                        },
                        format!("ops[{i}].{field}[{j}]"),
                    ));
                };
                if n < 1 || n > max {
                    return Err(Error::with_path(
                        ErrorKind::InvalidPrintSetup {
                            field: field.into(),
                        },
                        format!("ops[{i}].{field}[{j}]"),
                    ));
                }
                out.push(n);
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

fn parse_defined_name_name(v: Option<&Value>, i: usize) -> Result<Option<String>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Null) => Err(Error::with_path(
            ErrorKind::InvalidDefinedName {
                field: "name".into(),
            },
            format!("ops[{i}].name"),
        )),
        Some(Value::String(s)) => {
            names::validate_name(s, i)?;
            Ok(Some(s.clone()))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "name".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].name"),
        )),
    }
}

fn parse_defined_name_formula(v: Option<&Value>, i: usize) -> Result<Option<String>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Null) => Err(Error::with_path(
            ErrorKind::InvalidDefinedName {
                field: "formula".into(),
            },
            format!("ops[{i}].formula"),
        )),
        Some(Value::String(s)) => {
            names::validate_formula(s, i)?;
            Ok(Some(s.clone()))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "formula".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].formula"),
        )),
    }
}

fn parse_set_local_sheet(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<Option<Option<String>>, Error> {
    match obj.get("localSheet") {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(None)),
        Some(Value::String(s)) => {
            if s.is_empty() {
                return Err(Error::with_path(
                    ErrorKind::InvalidDefinedName {
                        field: "localSheet".into(),
                    },
                    format!("ops[{i}].localSheet"),
                ));
            }
            Ok(Some(Some(s.clone())))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "localSheet".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].localSheet"),
        )),
    }
}

fn parse_defined_name_id(id: &str, op_index: usize) -> Result<usize, Error> {
    let Some(nstr) = id.strip_prefix("name:") else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::DefinedNamePrefix,
            },
            format!("ops[{op_index}].id"),
        ));
    };
    if nstr.is_empty() || nstr.contains('/') || !nstr.chars().all(|c| c.is_ascii_digit()) {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::NameIndexNotInt,
            },
            format!("ops[{op_index}].id"),
        ));
    }
    let n: usize = nstr.parse().map_err(|_| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::NameIndexNotInt,
            },
            format!("ops[{op_index}].id"),
        )
    })?;
    if n == 0 {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::NameIndexFromOne,
            },
            format!("ops[{op_index}].id"),
        ));
    }
    Ok(n)
}

fn parse_pivot_name(v: Option<&Value>, i: usize) -> Result<Option<String>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Null) => Err(Error::with_path(
            ErrorKind::InvalidPivotTable {
                field: "name".into(),
            },
            format!("ops[{i}].name"),
        )),
        Some(Value::String(s)) => {
            if s.is_empty() || s.chars().any(char::is_whitespace) {
                return Err(Error::with_path(
                    ErrorKind::InvalidPivotTable {
                        field: "name".into(),
                    },
                    format!("ops[{i}].name"),
                ));
            }
            Ok(Some(s.clone()))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "name".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].name"),
        )),
    }
}

fn parse_pivot_id(id: &str, op_index: usize) -> Result<(String, usize), Error> {
    let Some(rest) = id.strip_prefix("sheet:") else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SheetPivotPrefix,
            },
            format!("ops[{op_index}].id"),
        ));
    };
    let Some((name, nstr)) = rest.split_once("/pivot:") else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SheetPivotPrefix,
            },
            format!("ops[{op_index}].id"),
        ));
    };
    if name.is_empty() || name.contains('/') {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SheetPivotPrefix,
            },
            format!("ops[{op_index}].id"),
        ));
    }
    if nstr.is_empty() || !nstr.chars().all(|c| c.is_ascii_digit()) {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::PivotIndexNotInt,
            },
            format!("ops[{op_index}].id"),
        ));
    }
    let n: usize = nstr.parse().map_err(|_| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::PivotIndexNotInt,
            },
            format!("ops[{op_index}].id"),
        )
    })?;
    if n == 0 {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::PivotIndexFromOne,
            },
            format!("ops[{op_index}].id"),
        ));
    }
    Ok((name.to_string(), n))
}

fn parse_pivot_source(
    v: Option<&Value>,
    i: usize,
    required: bool,
) -> Result<Option<PivotSource>, Error> {
    match v {
        None if required => Err(Error::with_path(
            ErrorKind::MissingField {
                field: "source".into(),
            },
            format!("ops[{i}].source"),
        )),
        None => Ok(None),
        Some(Value::Null) => Err(Error::with_path(
            ErrorKind::InvalidPivotTable {
                field: "source".into(),
            },
            format!("ops[{i}].source"),
        )),
        Some(Value::Object(obj)) => {
            deny_keys_at(obj, i, "source", &["sheet", "ref"])?;
            let sheet = match obj.get("sheet") {
                None => None,
                Some(Value::Null) => {
                    return Err(Error::with_path(
                        ErrorKind::InvalidPivotTable {
                            field: "source".into(),
                        },
                        format!("ops[{i}].source.sheet"),
                    ));
                }
                Some(Value::String(s)) if s.is_empty() => {
                    return Err(Error::with_path(
                        ErrorKind::InvalidPivotTable {
                            field: "source".into(),
                        },
                        format!("ops[{i}].source.sheet"),
                    ));
                }
                Some(Value::String(s)) => Some(s.clone()),
                Some(_) => {
                    return Err(Error::with_path(
                        ErrorKind::WrongType {
                            field: "sheet".into(),
                            expected: TypeName::String,
                        },
                        format!("ops[{i}].source.sheet"),
                    ));
                }
            };
            let refer = match obj.get("ref") {
                Some(Value::String(s)) => s.as_str(),
                None => {
                    return Err(Error::with_path(
                        ErrorKind::MissingField {
                            field: "ref".into(),
                        },
                        format!("ops[{i}].source.ref"),
                    ));
                }
                Some(_) => {
                    return Err(Error::with_path(
                        ErrorKind::WrongType {
                            field: "ref".into(),
                            expected: TypeName::String,
                        },
                        format!("ops[{i}].source.ref"),
                    ));
                }
            };
            let (start, end) =
                a1::parse_range(refer).map_err(|e| e.at(format!("ops[{i}].source.ref")))?;
            if start.row == end.row {
                return Err(Error::with_path(
                    ErrorKind::InvalidPivotTable {
                        field: "source".into(),
                    },
                    format!("ops[{i}].source.ref"),
                ));
            }
            Ok(Some(PivotSource {
                sheet,
                range: (start, end),
            }))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "source".into(),
                expected: TypeName::Object,
            },
            format!("ops[{i}].source"),
        )),
    }
}

fn parse_pivot_location(
    v: Option<&Value>,
    i: usize,
    required: bool,
) -> Result<Option<a1::CellRef>, Error> {
    match v {
        None if required => Err(Error::with_path(
            ErrorKind::MissingField {
                field: "location".into(),
            },
            format!("ops[{i}].location"),
        )),
        None => Ok(None),
        Some(Value::Null) => Err(Error::with_path(
            ErrorKind::InvalidPivotTable {
                field: "location".into(),
            },
            format!("ops[{i}].location"),
        )),
        Some(Value::String(s)) => {
            let (start, _) = a1::parse_range(s).map_err(|e| e.at(format!("ops[{i}].location")))?;
            Ok(Some(start))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "location".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].location"),
        )),
    }
}

fn parse_pivot_fields(
    v: Option<&Value>,
    i: usize,
    field: &str,
    required: bool,
) -> Result<Option<Vec<String>>, Error> {
    match v {
        None if required => Err(Error::with_path(
            ErrorKind::MissingField {
                field: field.into(),
            },
            format!("ops[{i}].{field}"),
        )),
        None => Ok(None),
        Some(Value::Null) => Err(Error::with_path(
            ErrorKind::InvalidPivotTable {
                field: field.into(),
            },
            format!("ops[{i}].{field}"),
        )),
        Some(Value::Array(arr)) => {
            let mut out = Vec::new();
            for (j, item) in arr.iter().enumerate() {
                let Some(s) = item.as_str() else {
                    return Err(Error::with_path(
                        ErrorKind::WrongType {
                            field: field.into(),
                            expected: TypeName::String,
                        },
                        format!("ops[{i}].{field}[{j}]"),
                    ));
                };
                if s.is_empty() {
                    return Err(Error::with_path(
                        ErrorKind::InvalidPivotTable {
                            field: field.into(),
                        },
                        format!("ops[{i}].{field}[{j}]"),
                    ));
                }
                out.push(s.to_string());
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

fn deny_keys_at(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    prefix: &str,
    allowed: &[&str],
) -> Result<(), Error> {
    for k in obj.keys() {
        if !allowed.contains(&k.as_str()) {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("ops[{i}].{prefix}.{k}"),
            ));
        }
    }
    Ok(())
}

fn parse_table_id(id: &str, op_index: usize) -> Result<(String, usize), Error> {
    let Some(rest) = id.strip_prefix("sheet:") else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SheetTablePrefix,
            },
            format!("ops[{op_index}].id"),
        ));
    };
    let Some((name, nstr)) = rest.split_once("/table:") else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SheetTablePrefix,
            },
            format!("ops[{op_index}].id"),
        ));
    };
    if name.is_empty() || name.contains('/') {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SheetTablePrefix,
            },
            format!("ops[{op_index}].id"),
        ));
    }
    if nstr.is_empty() || !nstr.chars().all(|c| c.is_ascii_digit()) {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::TableIndexNotInt,
            },
            format!("ops[{op_index}].id"),
        ));
    }
    let n: usize = nstr.parse().map_err(|_| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::TableIndexNotInt,
            },
            format!("ops[{op_index}].id"),
        )
    })?;
    if n == 0 {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::TableIndexFromOne,
            },
            format!("ops[{op_index}].id"),
        ));
    }
    Ok((name.to_string(), n))
}

fn parse_table_ref(
    v: Option<&Value>,
    i: usize,
    required: bool,
) -> Result<Option<(a1::CellRef, a1::CellRef)>, Error> {
    match v {
        None if required => Err(Error::with_path(
            ErrorKind::MissingField {
                field: "ref".into(),
            },
            format!("ops[{i}].ref"),
        )),
        None => Ok(None),
        Some(Value::Null) => Err(Error::with_path(
            ErrorKind::InvalidSheetTable {
                field: "ref".into(),
            },
            format!("ops[{i}].ref"),
        )),
        Some(Value::String(s)) => {
            let (start, end) = a1::parse_range(s).map_err(|e| e.at(format!("ops[{i}].ref")))?;
            if start.row == end.row {
                return Err(Error::with_path(
                    ErrorKind::InvalidSheetTable {
                        field: "ref".into(),
                    },
                    format!("ops[{i}].ref"),
                ));
            }
            Ok(Some((start, end)))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "ref".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].ref"),
        )),
    }
}

fn parse_table_name(v: Option<&Value>, i: usize) -> Result<Option<String>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Null) => Err(Error::with_path(
            ErrorKind::InvalidSheetTable {
                field: "name".into(),
            },
            format!("ops[{i}].name"),
        )),
        Some(Value::String(s)) => {
            if s.is_empty() || s.chars().any(char::is_whitespace) {
                return Err(Error::with_path(
                    ErrorKind::InvalidSheetTable {
                        field: "name".into(),
                    },
                    format!("ops[{i}].name"),
                ));
            }
            Ok(Some(s.clone()))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "name".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].name"),
        )),
    }
}

fn parse_table_columns(v: Option<&Value>, i: usize) -> Result<Option<Vec<String>>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Null) => Err(Error::with_path(
            ErrorKind::InvalidSheetTable {
                field: "columns".into(),
            },
            format!("ops[{i}].columns"),
        )),
        Some(Value::Array(arr)) if arr.is_empty() => Err(Error::with_path(
            ErrorKind::InvalidSheetTable {
                field: "columns".into(),
            },
            format!("ops[{i}].columns"),
        )),
        Some(Value::Array(arr)) => {
            let mut out = Vec::new();
            for (j, item) in arr.iter().enumerate() {
                let Some(s) = item.as_str() else {
                    return Err(Error::with_path(
                        ErrorKind::WrongType {
                            field: "columns".into(),
                            expected: TypeName::String,
                        },
                        format!("ops[{i}].columns[{j}]"),
                    ));
                };
                if s.is_empty() {
                    return Err(Error::with_path(
                        ErrorKind::InvalidSheetTable {
                            field: "columns".into(),
                        },
                        format!("ops[{i}].columns[{j}]"),
                    ));
                }
                out.push(s.to_string());
            }
            if has_table_col_dup(&out) {
                return Err(Error::with_path(
                    ErrorKind::InvalidSheetTable {
                        field: "columns".into(),
                    },
                    format!("ops[{i}].columns"),
                ));
            }
            Ok(Some(out))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "columns".into(),
                expected: TypeName::Array,
            },
            format!("ops[{i}].columns"),
        )),
    }
}

fn has_table_col_dup(cols: &[String]) -> bool {
    let mut seen = Vec::new();
    for c in cols {
        let k = c.to_ascii_lowercase();
        if seen.iter().any(|s: &String| s == &k) {
            return true;
        }
        seen.push(k);
    }
    false
}

fn parse_cf_id(id: &str, op_index: usize) -> Result<(String, usize), Error> {
    let Some(rest) = id.strip_prefix("sheet:") else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SheetCfPrefix,
            },
            format!("ops[{op_index}].id"),
        ));
    };
    let Some((name, nstr)) = rest.split_once("/cf:") else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SheetCfPrefix,
            },
            format!("ops[{op_index}].id"),
        ));
    };
    if name.is_empty() || name.contains('/') {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SheetCfPrefix,
            },
            format!("ops[{op_index}].id"),
        ));
    }
    if !nstr.chars().all(|c| c.is_ascii_digit()) {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::CfIndexNotInt,
            },
            format!("ops[{op_index}].id"),
        ));
    }
    let n: usize = nstr.parse().map_err(|_| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::CfIndexNotInt,
            },
            format!("ops[{op_index}].id"),
        )
    })?;
    if n == 0 {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::CfIndexFromOne,
            },
            format!("ops[{op_index}].id"),
        ));
    }
    Ok((name.to_string(), n))
}

fn parse_cf_type_field(
    v: Option<&Value>,
    i: usize,
    required: bool,
) -> Result<Option<CfType>, Error> {
    match v {
        None if required => Err(Error::with_path(
            ErrorKind::MissingField {
                field: "type".into(),
            },
            format!("ops[{i}].type"),
        )),
        None => Ok(None),
        Some(Value::String(s)) => CfType::parse(s).map(Some).ok_or_else(|| {
            Error::with_path(
                ErrorKind::InvalidConditionalFormat {
                    field: "type".into(),
                },
                format!("ops[{i}].type"),
            )
        }),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "type".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].type"),
        )),
    }
}

fn parse_cf_operator_field(
    v: Option<&Value>,
    i: usize,
    required: bool,
) -> Result<Option<CfOperator>, Error> {
    match v {
        None if required => Err(Error::with_path(
            ErrorKind::MissingField {
                field: "operator".into(),
            },
            format!("ops[{i}].operator"),
        )),
        None => Ok(None),
        Some(Value::String(s)) => CfOperator::parse(s).map(Some).ok_or_else(|| {
            Error::with_path(
                ErrorKind::InvalidConditionalFormat {
                    field: "operator".into(),
                },
                format!("ops[{i}].operator"),
            )
        }),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "operator".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].operator"),
        )),
    }
}

fn parse_cf_operator_patch(v: Option<&Value>, i: usize) -> Result<Patch<CfOperator>, Error> {
    match v {
        None => Ok(Patch::Absent),
        Some(Value::Null) => Ok(Patch::Clear),
        Some(Value::String(s)) => CfOperator::parse(s).map(Patch::Set).ok_or_else(|| {
            Error::with_path(
                ErrorKind::InvalidConditionalFormat {
                    field: "operator".into(),
                },
                format!("ops[{i}].operator"),
            )
        }),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "operator".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].operator"),
        )),
    }
}

fn parse_cf_formulas_opt(v: Option<&Value>, i: usize) -> Result<Option<Vec<String>>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Array(arr)) if arr.is_empty() => Err(Error::with_path(
            ErrorKind::InvalidConditionalFormat {
                field: "formulas".into(),
            },
            format!("ops[{i}].formulas"),
        )),
        Some(Value::Array(arr)) => {
            let mut out = Vec::new();
            for (j, item) in arr.iter().enumerate() {
                let Some(s) = item.as_str() else {
                    return Err(Error::with_path(
                        ErrorKind::WrongType {
                            field: "formulas".into(),
                            expected: TypeName::String,
                        },
                        format!("ops[{i}].formulas[{j}]"),
                    ));
                };
                out.push(strip_formula_eq(s));
            }
            if out.len() > 3 {
                return Err(Error::with_path(
                    ErrorKind::InvalidConditionalFormat {
                        field: "formulas".into(),
                    },
                    format!("ops[{i}].formulas"),
                ));
            }
            Ok(Some(out))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "formulas".into(),
                expected: TypeName::Array,
            },
            format!("ops[{i}].formulas"),
        )),
    }
}

fn parse_cf_formulas_patch(v: Option<&Value>, i: usize) -> Result<Patch<Vec<String>>, Error> {
    match v {
        None => Ok(Patch::Absent),
        Some(Value::Null) => Ok(Patch::Clear),
        Some(other) => Ok(Patch::Set(
            parse_cf_formulas_opt(Some(other), i)?.ok_or_else(|| {
                Error::with_path(
                    ErrorKind::InvalidConditionalFormat {
                        field: "formulas".into(),
                    },
                    format!("ops[{i}].formulas"),
                )
            })?,
        )),
    }
}

fn parse_cf_text_opt(v: Option<&Value>, i: usize) -> Result<Option<String>, Error> {
    match v {
        None => Ok(None),
        Some(Value::String(s)) if !s.is_empty() => Ok(Some(s.clone())),
        Some(Value::String(_)) => Err(Error::with_path(
            ErrorKind::InvalidConditionalFormat {
                field: "text".into(),
            },
            format!("ops[{i}].text"),
        )),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "text".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].text"),
        )),
    }
}

fn parse_cf_text_patch(v: Option<&Value>, i: usize) -> Result<Patch<String>, Error> {
    match v {
        None => Ok(Patch::Absent),
        Some(Value::Null) => Ok(Patch::Clear),
        Some(other) => Ok(Patch::Set(parse_cf_text_opt(Some(other), i)?.ok_or_else(
            || {
                Error::with_path(
                    ErrorKind::InvalidConditionalFormat {
                        field: "text".into(),
                    },
                    format!("ops[{i}].text"),
                )
            },
        )?)),
    }
}

fn parse_cf_rank_patch(v: Option<&Value>, i: usize) -> Result<Patch<u32>, Error> {
    match v {
        None => Ok(Patch::Absent),
        Some(Value::Null) => Err(Error::with_path(
            ErrorKind::InvalidConditionalFormat {
                field: "rank".into(),
            },
            format!("ops[{i}].rank"),
        )),
        Some(Value::Number(n)) => {
            let Some(v) = n.as_u64().and_then(|x| u32::try_from(x).ok()) else {
                return Err(Error::with_path(
                    ErrorKind::InvalidConditionalFormat {
                        field: "rank".into(),
                    },
                    format!("ops[{i}].rank"),
                ));
            };
            if v < 1 {
                return Err(Error::with_path(
                    ErrorKind::InvalidConditionalFormat {
                        field: "rank".into(),
                    },
                    format!("ops[{i}].rank"),
                ));
            }
            Ok(Patch::Set(v))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "rank".into(),
                expected: TypeName::Integer,
            },
            format!("ops[{i}].rank"),
        )),
    }
}

fn parse_cf_std_dev_opt(v: Option<&Value>, i: usize) -> Result<Option<i32>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Number(n)) => {
            let Some(v) = n.as_i64().and_then(|x| i32::try_from(x).ok()) else {
                return Err(Error::with_path(
                    ErrorKind::InvalidConditionalFormat {
                        field: "stdDev".into(),
                    },
                    format!("ops[{i}].stdDev"),
                ));
            };
            if v < 1 {
                return Err(Error::with_path(
                    ErrorKind::InvalidConditionalFormat {
                        field: "stdDev".into(),
                    },
                    format!("ops[{i}].stdDev"),
                ));
            }
            Ok(Some(v))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "stdDev".into(),
                expected: TypeName::Integer,
            },
            format!("ops[{i}].stdDev"),
        )),
    }
}

fn parse_cf_std_dev_patch(v: Option<&Value>, i: usize) -> Result<Patch<i32>, Error> {
    match v {
        None => Ok(Patch::Absent),
        Some(Value::Null) => Ok(Patch::Clear),
        Some(other) => Ok(Patch::Set(
            parse_cf_std_dev_opt(Some(other), i)?.ok_or_else(|| {
                Error::with_path(
                    ErrorKind::InvalidConditionalFormat {
                        field: "stdDev".into(),
                    },
                    format!("ops[{i}].stdDev"),
                )
            })?,
        )),
    }
}

fn parse_cf_bool_patch(v: Option<&Value>, i: usize, field: &str) -> Result<Patch<bool>, Error> {
    match v {
        None => Ok(Patch::Absent),
        Some(Value::Null) => Ok(Patch::Clear),
        Some(Value::Bool(b)) => Ok(Patch::Set(*b)),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Boolean,
            },
            format!("ops[{i}].{field}"),
        )),
    }
}

fn parse_dxf_opt(v: Option<&Value>, i: usize) -> Result<Option<DxfPatch>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Object(m)) => Ok(Some(parse_dxf_obj(m, i)?)),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "dxf".into(),
                expected: TypeName::Object,
            },
            format!("ops[{i}].dxf"),
        )),
    }
}

fn parse_dxf_patch_field(v: Option<&Value>, i: usize) -> Result<Patch<DxfPatch>, Error> {
    match v {
        None => Ok(Patch::Absent),
        Some(Value::Null) => Ok(Patch::Clear),
        Some(Value::Object(m)) => Ok(Patch::Set(parse_dxf_obj(m, i)?)),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "dxf".into(),
                expected: TypeName::Object,
            },
            format!("ops[{i}].dxf"),
        )),
    }
}

fn parse_dxf_obj(obj: &serde_json::Map<String, Value>, i: usize) -> Result<DxfPatch, Error> {
    deny_sub_keys(obj, i, "dxf", &["font", "fill", "border"])?;
    let font = match obj.get("font") {
        None => None,
        Some(Value::Object(m)) => Some(parse_font_patch(m, i, "dxf.font")?),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "font".into(),
                    expected: TypeName::Object,
                },
                format!("ops[{i}].dxf.font"),
            ));
        }
    };
    let fill = match obj.get("fill") {
        None => None,
        Some(Value::Object(m)) => Some(parse_fill_patch(m, i, "dxf.fill")?),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "fill".into(),
                    expected: TypeName::Object,
                },
                format!("ops[{i}].dxf.fill"),
            ));
        }
    };
    let border = match obj.get("border") {
        None => None,
        Some(Value::Object(m)) => Some(parse_border_patch(m, i, "dxf.border")?),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "border".into(),
                    expected: TypeName::Object,
                },
                format!("ops[{i}].dxf.border"),
            ));
        }
    };
    Ok(DxfPatch { font, fill, border })
}

fn parse_color_scale_opt(v: Option<&Value>, i: usize) -> Result<Option<ColorScaleSpec>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Object(m)) => Ok(Some(parse_color_scale_obj(m, i)?)),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "colorScale".into(),
                expected: TypeName::Object,
            },
            format!("ops[{i}].colorScale"),
        )),
    }
}

fn parse_color_scale_patch(v: Option<&Value>, i: usize) -> Result<Patch<ColorScaleSpec>, Error> {
    match v {
        None => Ok(Patch::Absent),
        Some(Value::Null) => Ok(Patch::Clear),
        Some(Value::Object(m)) => Ok(Patch::Set(parse_color_scale_obj(m, i)?)),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "colorScale".into(),
                expected: TypeName::Object,
            },
            format!("ops[{i}].colorScale"),
        )),
    }
}

fn parse_color_scale_obj(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<ColorScaleSpec, Error> {
    deny_sub_keys(obj, i, "colorScale", &["cfvo", "colors"])?;
    let colors = parse_color_array(obj.get("colors"), i, "colorScale.colors", 2, 3)?;
    let cfvo = parse_cfvo_array_opt(obj.get("cfvo"), i, "colorScale.cfvo", 2, 3)?;
    if let Some(v) = &cfvo {
        if v.len() != colors.len() {
            return Err(Error::with_path(
                ErrorKind::InvalidConditionalFormat {
                    field: "colorScale.cfvo".into(),
                },
                format!("ops[{i}].colorScale.cfvo"),
            ));
        }
    }
    Ok(ColorScaleSpec { cfvo, colors })
}

fn parse_data_bar_opt(v: Option<&Value>, i: usize) -> Result<Option<DataBarSpec>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Object(m)) => Ok(Some(parse_data_bar_obj(m, i)?)),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "dataBar".into(),
                expected: TypeName::Object,
            },
            format!("ops[{i}].dataBar"),
        )),
    }
}

fn parse_data_bar_patch(v: Option<&Value>, i: usize) -> Result<Patch<DataBarSpec>, Error> {
    match v {
        None => Ok(Patch::Absent),
        Some(Value::Null) => Ok(Patch::Clear),
        Some(Value::Object(m)) => Ok(Patch::Set(parse_data_bar_obj(m, i)?)),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "dataBar".into(),
                expected: TypeName::Object,
            },
            format!("ops[{i}].dataBar"),
        )),
    }
}

fn parse_data_bar_obj(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DataBarSpec, Error> {
    deny_sub_keys(
        obj,
        i,
        "dataBar",
        &["cfvo", "color", "minLength", "maxLength", "showValue"],
    )?;
    let cfvo = parse_cfvo_array_opt(obj.get("cfvo"), i, "dataBar.cfvo", 2, 2)?;
    let color = match obj.get("color") {
        None => None,
        Some(Value::Object(m)) => Some(parse_color_spec(m, i, "dataBar.color")?),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "color".into(),
                    expected: TypeName::Object,
                },
                format!("ops[{i}].dataBar.color"),
            ));
        }
    };
    Ok(DataBarSpec {
        cfvo,
        color,
        min_length: parse_opt_u32_top(obj, i, "minLength", "dataBar.minLength", 0, 100)?,
        max_length: parse_opt_u32_top(obj, i, "maxLength", "dataBar.maxLength", 0, 100)?,
        show_value: parse_sub_bool(obj, i, "dataBar", "showValue")?,
    })
}

fn parse_icon_set_opt(v: Option<&Value>, i: usize) -> Result<Option<IconSetSpec>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Object(m)) => Ok(Some(parse_icon_set_obj(m, i)?)),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "iconSet".into(),
                expected: TypeName::Object,
            },
            format!("ops[{i}].iconSet"),
        )),
    }
}

fn parse_icon_set_patch(v: Option<&Value>, i: usize) -> Result<Patch<IconSetSpec>, Error> {
    match v {
        None => Ok(Patch::Absent),
        Some(Value::Null) => Ok(Patch::Clear),
        Some(Value::Object(m)) => Ok(Patch::Set(parse_icon_set_obj(m, i)?)),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "iconSet".into(),
                expected: TypeName::Object,
            },
            format!("ops[{i}].iconSet"),
        )),
    }
}

fn parse_icon_set_obj(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<IconSetSpec, Error> {
    deny_sub_keys(
        obj,
        i,
        "iconSet",
        &["set", "cfvo", "showValue", "reverse", "percent"],
    )?;
    let set = match obj.get("set") {
        None => None,
        Some(Value::String(s)) if ICON_SETS.contains(&s.as_str()) => Some(s.clone()),
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidConditionalFormat {
                    field: "iconSet.set".into(),
                },
                format!("ops[{i}].iconSet.set"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "set".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].iconSet.set"),
            ));
        }
    };
    let n = icon_set_count(set.as_deref().unwrap_or("3TrafficLights1"));
    let cfvo = parse_cfvo_array_opt(obj.get("cfvo"), i, "iconSet.cfvo", n, n)?;
    Ok(IconSetSpec {
        set,
        cfvo,
        show_value: parse_sub_bool(obj, i, "iconSet", "showValue")?,
        reverse: parse_sub_bool(obj, i, "iconSet", "reverse")?,
        percent: parse_sub_bool(obj, i, "iconSet", "percent")?,
    })
}

fn parse_cfvo_array_opt(
    v: Option<&Value>,
    i: usize,
    path: &str,
    min: usize,
    max: usize,
) -> Result<Option<Vec<CfvoSpec>>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Array(arr)) => {
            if arr.len() < min || arr.len() > max {
                return Err(Error::with_path(
                    ErrorKind::InvalidConditionalFormat { field: path.into() },
                    format!("ops[{i}].{path}"),
                ));
            }
            let mut out = Vec::new();
            for (j, item) in arr.iter().enumerate() {
                let Some(obj) = item.as_object() else {
                    return Err(Error::with_path(
                        ErrorKind::WrongType {
                            field: "cfvo".into(),
                            expected: TypeName::Object,
                        },
                        format!("ops[{i}].{path}[{j}]"),
                    ));
                };
                out.push(parse_cfvo_obj(obj, i, &format!("{path}[{j}]"))?);
            }
            Ok(Some(out))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "cfvo".into(),
                expected: TypeName::Array,
            },
            format!("ops[{i}].{path}"),
        )),
    }
}

fn parse_cfvo_obj(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    prefix: &str,
) -> Result<CfvoSpec, Error> {
    deny_sub_keys(obj, i, prefix, &["type", "val", "gte"])?;
    let type_ = match obj.get("type") {
        Some(Value::String(s)) if CFVO_TYPES.contains(&s.as_str()) => s.clone(),
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidConditionalFormat {
                    field: "cfvo.type".into(),
                },
                format!("ops[{i}].{prefix}.type"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "type".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].{prefix}.type"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "type".into(),
                },
                format!("ops[{i}].{prefix}.type"),
            ));
        }
    };
    let val = match obj.get("val") {
        None => None,
        Some(Value::String(s)) => Some(s.clone()),
        Some(Value::Number(n)) => Some(n.to_string()),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "val".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].{prefix}.val"),
            ));
        }
    };
    Ok(CfvoSpec {
        type_,
        val,
        gte: parse_sub_bool(obj, i, prefix, "gte")?,
    })
}

fn parse_color_array(
    v: Option<&Value>,
    i: usize,
    path: &str,
    min: usize,
    max: usize,
) -> Result<Vec<ColorSpec>, Error> {
    let Some(Value::Array(arr)) = v else {
        return Err(Error::with_path(
            if v.is_none() {
                ErrorKind::MissingField {
                    field: "colors".into(),
                }
            } else {
                ErrorKind::WrongType {
                    field: "colors".into(),
                    expected: TypeName::Array,
                }
            },
            format!("ops[{i}].{path}"),
        ));
    };
    if arr.len() < min || arr.len() > max {
        return Err(Error::with_path(
            ErrorKind::InvalidConditionalFormat {
                field: "colors".into(),
            },
            format!("ops[{i}].{path}"),
        ));
    }
    let mut out = Vec::new();
    for (j, item) in arr.iter().enumerate() {
        let Some(obj) = item.as_object() else {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "colors".into(),
                    expected: TypeName::Object,
                },
                format!("ops[{i}].{path}[{j}]"),
            ));
        };
        out.push(parse_color_spec(obj, i, &format!("{path}[{j}]"))?);
    }
    Ok(out)
}

fn parse_cf_priority_opt(v: Option<&Value>, i: usize) -> Result<Option<i32>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Null) => Err(Error::with_path(
            ErrorKind::InvalidConditionalFormat {
                field: "priority".into(),
            },
            format!("ops[{i}].priority"),
        )),
        Some(Value::Number(n)) => {
            let Some(v) = n.as_i64().filter(|x| *x >= 1 && *x <= i32::MAX as i64) else {
                return Err(Error::with_path(
                    ErrorKind::InvalidConditionalFormat {
                        field: "priority".into(),
                    },
                    format!("ops[{i}].priority"),
                ));
            };
            Ok(Some(v as i32))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "priority".into(),
                expected: TypeName::Integer,
            },
            format!("ops[{i}].priority"),
        )),
    }
}

fn parse_opt_u32_top(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    field: &str,
    path: &str,
    min: u32,
    max: u32,
) -> Result<Option<u32>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::Number(n)) => {
            let Some(v) = n.as_u64().and_then(|x| u32::try_from(x).ok()) else {
                return Err(Error::with_path(
                    ErrorKind::InvalidConditionalFormat {
                        field: field.into(),
                    },
                    format!("ops[{i}].{path}"),
                ));
            };
            if v < min || v > max {
                return Err(Error::with_path(
                    ErrorKind::InvalidConditionalFormat {
                        field: field.into(),
                    },
                    format!("ops[{i}].{path}"),
                ));
            }
            Ok(Some(v))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Integer,
            },
            format!("ops[{i}].{path}"),
        )),
    }
}

pub(crate) fn check_cf_fields(
    type_: CfType,
    operator: Option<CfOperator>,
    formulas: Option<&[String]>,
    dxf: Option<&DxfPatch>,
    color_scale: Option<&ColorScaleSpec>,
    data_bar: Option<&DataBarSpec>,
    icon_set: Option<&IconSetSpec>,
    extra: CfExtraCheck<'_>,
    i: usize,
    insert: bool,
) -> Result<(), Error> {
    let bad = |field: &str| {
        Error::with_path(
            ErrorKind::InvalidConditionalFormat {
                field: field.into(),
            },
            format!("ops[{i}].{field}"),
        )
    };
    let reject_visual = || {
        if color_scale.is_some() {
            Err(bad("colorScale"))
        } else if data_bar.is_some() {
            Err(bad("dataBar"))
        } else if icon_set.is_some() {
            Err(bad("iconSet"))
        } else {
            Ok(())
        }
    };
    match type_ {
        CfType::CellIs => {
            reject_visual()?;
            reject_text_extra(&extra, i)?;
            reject_top10_extra(&extra, i)?;
            reject_avg_extra(&extra, i)?;
            let Some(op) = operator else {
                return Err(Error::with_path(
                    ErrorKind::MissingField {
                        field: "operator".into(),
                    },
                    format!("ops[{i}].operator"),
                ));
            };
            let need = op.formula_count();
            let got = formulas.map(|f| f.len()).unwrap_or(0);
            if got != need {
                return Err(Error::with_path(
                    ErrorKind::NeedConditionalFormatFormula {
                        field: "formulas".into(),
                    },
                    format!("ops[{i}].formulas"),
                ));
            }
        }
        CfType::Expression => {
            if operator.is_some() {
                return Err(bad("operator"));
            }
            reject_visual()?;
            reject_text_extra(&extra, i)?;
            reject_top10_extra(&extra, i)?;
            reject_avg_extra(&extra, i)?;
            if formulas.map(|f| f.len()).unwrap_or(0) != 1 {
                return Err(Error::with_path(
                    ErrorKind::NeedConditionalFormatFormula {
                        field: "formulas".into(),
                    },
                    format!("ops[{i}].formulas"),
                ));
            }
        }
        CfType::ColorScale => {
            if operator.is_some() {
                return Err(bad("operator"));
            }
            if formulas.is_some() {
                return Err(bad("formulas"));
            }
            if dxf.is_some() {
                return Err(bad("dxf"));
            }
            if data_bar.is_some() {
                return Err(bad("dataBar"));
            }
            if icon_set.is_some() {
                return Err(bad("iconSet"));
            }
            reject_text_extra(&extra, i)?;
            reject_top10_extra(&extra, i)?;
            reject_avg_extra(&extra, i)?;
            if insert && color_scale.is_none() {
                return Err(Error::with_path(
                    ErrorKind::MissingField {
                        field: "colorScale".into(),
                    },
                    format!("ops[{i}].colorScale"),
                ));
            }
        }
        CfType::DataBar => {
            if operator.is_some() {
                return Err(bad("operator"));
            }
            if formulas.is_some() {
                return Err(bad("formulas"));
            }
            if dxf.is_some() {
                return Err(bad("dxf"));
            }
            if color_scale.is_some() {
                return Err(bad("colorScale"));
            }
            if icon_set.is_some() {
                return Err(bad("iconSet"));
            }
            reject_text_extra(&extra, i)?;
            reject_top10_extra(&extra, i)?;
            reject_avg_extra(&extra, i)?;
        }
        CfType::IconSet => {
            if operator.is_some() {
                return Err(bad("operator"));
            }
            if formulas.is_some() {
                return Err(bad("formulas"));
            }
            if dxf.is_some() {
                return Err(bad("dxf"));
            }
            if color_scale.is_some() {
                return Err(bad("colorScale"));
            }
            if data_bar.is_some() {
                return Err(bad("dataBar"));
            }
            reject_text_extra(&extra, i)?;
            reject_top10_extra(&extra, i)?;
            reject_avg_extra(&extra, i)?;
        }
        CfType::Top10 => {
            if operator.is_some() {
                return Err(bad("operator"));
            }
            if formulas.is_some() {
                return Err(bad("formulas"));
            }
            reject_visual()?;
            reject_text_extra(&extra, i)?;
            reject_avg_extra(&extra, i)?;
        }
        CfType::AboveAverage => {
            if operator.is_some() {
                return Err(bad("operator"));
            }
            if formulas.is_some() {
                return Err(bad("formulas"));
            }
            reject_visual()?;
            reject_text_extra(&extra, i)?;
            reject_top10_extra(&extra, i)?;
        }
        CfType::ContainsText | CfType::NotContainsText | CfType::BeginsWith | CfType::EndsWith => {
            if operator.is_some() {
                return Err(bad("operator"));
            }
            reject_visual()?;
            reject_top10_extra(&extra, i)?;
            reject_avg_extra(&extra, i)?;
            if insert && extra.text.is_none() {
                return Err(Error::with_path(
                    ErrorKind::MissingField {
                        field: "text".into(),
                    },
                    format!("ops[{i}].text"),
                ));
            }
            if let Some(fs) = formulas {
                if fs.len() != 1 {
                    return Err(Error::with_path(
                        ErrorKind::NeedConditionalFormatFormula {
                            field: "formulas".into(),
                        },
                        format!("ops[{i}].formulas"),
                    ));
                }
            }
        }
    }
    Ok(())
}

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

fn reject_text_extra(extra: &CfExtraCheck<'_>, i: usize) -> Result<(), Error> {
    if extra.text.is_some() {
        return Err(Error::with_path(
            ErrorKind::InvalidConditionalFormat {
                field: "text".into(),
            },
            format!("ops[{i}].text"),
        ));
    }
    Ok(())
}

fn reject_top10_extra(extra: &CfExtraCheck<'_>, i: usize) -> Result<(), Error> {
    if extra.rank.is_some() {
        return Err(Error::with_path(
            ErrorKind::InvalidConditionalFormat {
                field: "rank".into(),
            },
            format!("ops[{i}].rank"),
        ));
    }
    if extra.percent.is_some() {
        return Err(Error::with_path(
            ErrorKind::InvalidConditionalFormat {
                field: "percent".into(),
            },
            format!("ops[{i}].percent"),
        ));
    }
    if extra.bottom.is_some() {
        return Err(Error::with_path(
            ErrorKind::InvalidConditionalFormat {
                field: "bottom".into(),
            },
            format!("ops[{i}].bottom"),
        ));
    }
    Ok(())
}

fn reject_avg_extra(extra: &CfExtraCheck<'_>, i: usize) -> Result<(), Error> {
    if extra.above_average.is_some() {
        return Err(Error::with_path(
            ErrorKind::InvalidConditionalFormat {
                field: "aboveAverage".into(),
            },
            format!("ops[{i}].aboveAverage"),
        ));
    }
    if extra.equal_average.is_some() {
        return Err(Error::with_path(
            ErrorKind::InvalidConditionalFormat {
                field: "equalAverage".into(),
            },
            format!("ops[{i}].equalAverage"),
        ));
    }
    if extra.std_dev.is_some() {
        return Err(Error::with_path(
            ErrorKind::InvalidConditionalFormat {
                field: "stdDev".into(),
            },
            format!("ops[{i}].stdDev"),
        ));
    }
    Ok(())
}

fn parse_dv_id(id: &str, op_index: usize) -> Result<(String, usize), Error> {
    let Some(rest) = id.strip_prefix("sheet:") else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SheetDvPrefix,
            },
            format!("ops[{op_index}].id"),
        ));
    };
    let Some((name, nstr)) = rest.split_once("/dv:") else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SheetDvPrefix,
            },
            format!("ops[{op_index}].id"),
        ));
    };
    if name.is_empty() || name.contains('/') {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SheetDvPrefix,
            },
            format!("ops[{op_index}].id"),
        ));
    }
    if nstr.is_empty() || !nstr.bytes().all(|b| b.is_ascii_digit()) {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::DvIndexNotInt,
            },
            format!("ops[{op_index}].id"),
        ));
    }
    let n: usize = nstr.parse().map_err(|_| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::DvIndexNotInt,
            },
            format!("ops[{op_index}].id"),
        )
    })?;
    if n == 0 {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::DvIndexFromOne,
            },
            format!("ops[{op_index}].id"),
        ));
    }
    Ok((name.to_string(), n))
}

fn parse_edit_range_id(id: &str, op_index: usize) -> Result<(String, usize), Error> {
    let Some(rest) = id.strip_prefix("sheet:") else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SheetEditRangePrefix,
            },
            format!("ops[{op_index}].id"),
        ));
    };
    let Some((name, nstr)) = rest.split_once("/editRange:") else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SheetEditRangePrefix,
            },
            format!("ops[{op_index}].id"),
        ));
    };
    if name.is_empty() || name.contains('/') {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SheetEditRangePrefix,
            },
            format!("ops[{op_index}].id"),
        ));
    }
    if nstr.is_empty() || !nstr.bytes().all(|b| b.is_ascii_digit()) {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::EditRangeIndexNotInt,
            },
            format!("ops[{op_index}].id"),
        ));
    }
    let n: usize = nstr.parse().map_err(|_| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::EditRangeIndexNotInt,
            },
            format!("ops[{op_index}].id"),
        )
    })?;
    if n == 0 {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::EditRangeIndexFromOne,
            },
            format!("ops[{op_index}].id"),
        ));
    }
    Ok((name.to_string(), n))
}

fn parse_link_id(id: &str, op_index: usize) -> Result<(String, usize), Error> {
    let Some(rest) = id.strip_prefix("sheet:") else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SheetLinkPrefix,
            },
            format!("ops[{op_index}].id"),
        ));
    };
    let Some((name, nstr)) = rest.split_once("/link:") else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SheetLinkPrefix,
            },
            format!("ops[{op_index}].id"),
        ));
    };
    if name.is_empty() || name.contains('/') {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SheetLinkPrefix,
            },
            format!("ops[{op_index}].id"),
        ));
    }
    if nstr.is_empty() || !nstr.bytes().all(|b| b.is_ascii_digit()) {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::LinkIndexNotInt,
            },
            format!("ops[{op_index}].id"),
        ));
    }
    let n: usize = nstr.parse().map_err(|_| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::LinkIndexNotInt,
            },
            format!("ops[{op_index}].id"),
        )
    })?;
    if n == 0 {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::LinkIndexFromOne,
            },
            format!("ops[{op_index}].id"),
        ));
    }
    Ok((name.to_string(), n))
}

fn parse_spark_id(id: &str, op_index: usize) -> Result<(String, usize), Error> {
    let Some(rest) = id.strip_prefix("sheet:") else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SheetSparkPrefix,
            },
            format!("ops[{op_index}].id"),
        ));
    };
    let Some((name, nstr)) = rest.split_once("/spark:") else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SheetSparkPrefix,
            },
            format!("ops[{op_index}].id"),
        ));
    };
    if name.is_empty() || name.contains('/') {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SheetSparkPrefix,
            },
            format!("ops[{op_index}].id"),
        ));
    }
    if nstr.is_empty() || !nstr.bytes().all(|b| b.is_ascii_digit()) {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SparkIndexNotInt,
            },
            format!("ops[{op_index}].id"),
        ));
    }
    let n: usize = nstr.parse().map_err(|_| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SparkIndexNotInt,
            },
            format!("ops[{op_index}].id"),
        )
    })?;
    if n == 0 {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SparkIndexFromOne,
            },
            format!("ops[{op_index}].id"),
        ));
    }
    Ok((name.to_string(), n))
}

fn parse_required_spark_host(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<(String, a1::CellRef), Error> {
    parse_opt_spark_host(obj, i)?.ok_or_else(|| {
        Error::with_path(
            ErrorKind::MissingField {
                field: "ref".into(),
            },
            format!("ops[{i}].ref"),
        )
    })
}

fn parse_opt_spark_host(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<Option<(String, a1::CellRef)>, Error> {
    match obj.get("ref") {
        None => Ok(None),
        Some(Value::String(s)) => {
            let (start, end) = a1::parse_range(s).map_err(|e| e.at(format!("ops[{i}].ref")))?;
            if start != end {
                return Err(Error::with_path(
                    ErrorKind::SparklineHostNotCell,
                    format!("ops[{i}].ref"),
                ));
            }
            Ok(Some((a1::format_range(start, end), start)))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "ref".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].ref"),
        )),
    }
}

fn parse_required_spark_formula(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<String, Error> {
    match obj.get("formula") {
        Some(Value::String(s)) => parse_spark_formula_text(s, i),
        None => Err(Error::with_path(
            ErrorKind::MissingField {
                field: "formula".into(),
            },
            format!("ops[{i}].formula"),
        )),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "formula".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].formula"),
        )),
    }
}

fn parse_opt_spark_formula(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<Patch<String>, Error> {
    match obj.get("formula") {
        None => Ok(Patch::Absent),
        Some(Value::String(s)) => Ok(Patch::Set(parse_spark_formula_text(s, i)?)),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "formula".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].formula"),
        )),
    }
}

fn parse_spark_formula_text(s: &str, i: usize) -> Result<String, Error> {
    if s.trim().is_empty() {
        return Err(Error::with_path(
            ErrorKind::EmptySparklineFormula,
            format!("ops[{i}].formula"),
        ));
    }
    if formula_is_block(s) {
        return Err(Error::with_path(
            ErrorKind::SparklineFormulaNotLine,
            format!("ops[{i}].formula"),
        ));
    }
    Ok(s.to_string())
}

fn formula_is_block(raw: &str) -> bool {
    let t = raw.trim();
    let a1 = match t.rfind('!') {
        Some(i) => &t[i + 1..],
        None => t,
    };
    match a1::parse_range(a1) {
        Ok((a, b)) => a.row != b.row && a.col != b.col,
        Err(_) => false,
    }
}

fn parse_spark_type(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<Option<SparklineType>, Error> {
    match obj.get("type") {
        None => Ok(None),
        Some(Value::String(s)) => match s.as_str() {
            "line" => Ok(Some(SparklineType::Line)),
            "column" => Ok(Some(SparklineType::Column)),
            "stacked" => Ok(Some(SparklineType::Stacked)),
            _ => Err(Error::with_path(
                ErrorKind::InvalidSparklineType,
                format!("ops[{i}].type"),
            )),
        },
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "type".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].type"),
        )),
    }
}

fn parse_spark_empty(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<Option<SparkEmpty>, Error> {
    match obj.get("displayEmptyCellsAs") {
        None => Ok(None),
        Some(Value::String(s)) => SparkEmpty::parse(s).map(Some).ok_or_else(|| {
            Error::with_path(
                ErrorKind::InvalidSparkline {
                    field: "displayEmptyCellsAs".into(),
                },
                format!("ops[{i}].displayEmptyCellsAs"),
            )
        }),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "displayEmptyCellsAs".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].displayEmptyCellsAs"),
        )),
    }
}

fn parse_spark_axis(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    field: &str,
) -> Result<Option<SparkAxisType>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::String(s)) => SparkAxisType::parse(s).map(Some).ok_or_else(|| {
            Error::with_path(
                ErrorKind::InvalidSparkline {
                    field: field.into(),
                },
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

fn parse_opt_f64_top(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    field: &str,
) -> Result<Option<f64>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::Number(n)) => {
            let Some(v) = n.as_f64().filter(|x| x.is_finite()) else {
                return Err(Error::with_path(
                    ErrorKind::InvalidSparkline {
                        field: field.into(),
                    },
                    format!("ops[{i}].{field}"),
                ));
            };
            Ok(Some(v))
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

fn parse_f64_patch(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    field: &str,
) -> Result<Patch<f64>, Error> {
    match obj.get(field) {
        None => Ok(Patch::Absent),
        Some(Value::Null) => Ok(Patch::Clear),
        Some(Value::Number(n)) => {
            let Some(v) = n.as_f64().filter(|x| x.is_finite()) else {
                return Err(Error::with_path(
                    ErrorKind::InvalidSparkline {
                        field: field.into(),
                    },
                    format!("ops[{i}].{field}"),
                ));
            };
            Ok(Patch::Set(v))
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

fn parse_spark_line_weight(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<Option<f64>, Error> {
    match obj.get("lineWeight") {
        None => Ok(None),
        Some(Value::Number(n)) => {
            let Some(v) = n.as_f64().filter(|x| x.is_finite()) else {
                return Err(Error::with_path(
                    ErrorKind::InvalidSparkline {
                        field: "lineWeight".into(),
                    },
                    format!("ops[{i}].lineWeight"),
                ));
            };
            if !(0.0..=1584.0).contains(&v) {
                return Err(Error::with_path(
                    ErrorKind::InvalidSparkline {
                        field: "lineWeight".into(),
                    },
                    format!("ops[{i}].lineWeight"),
                ));
            }
            Ok(Some(v))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "lineWeight".into(),
                expected: TypeName::Integer,
            },
            format!("ops[{i}].lineWeight"),
        )),
    }
}

fn parse_spark_line_weight_patch(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<Patch<f64>, Error> {
    match obj.get("lineWeight") {
        None => Ok(Patch::Absent),
        Some(Value::Null) => Ok(Patch::Clear),
        Some(Value::Number(n)) => {
            let Some(v) = n.as_f64().filter(|x| x.is_finite()) else {
                return Err(Error::with_path(
                    ErrorKind::InvalidSparkline {
                        field: "lineWeight".into(),
                    },
                    format!("ops[{i}].lineWeight"),
                ));
            };
            if !(0.0..=1584.0).contains(&v) {
                return Err(Error::with_path(
                    ErrorKind::InvalidSparkline {
                        field: "lineWeight".into(),
                    },
                    format!("ops[{i}].lineWeight"),
                ));
            }
            Ok(Patch::Set(v))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "lineWeight".into(),
                expected: TypeName::Integer,
            },
            format!("ops[{i}].lineWeight"),
        )),
    }
}

fn parse_spark_colors(v: Option<&Value>, i: usize) -> Result<Option<SparkColors>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Object(m)) => {
            deny_sub_keys(
                m,
                i,
                "colors",
                &[
                    "series", "negative", "axis", "markers", "first", "last", "high", "low",
                ],
            )?;
            Ok(Some(SparkColors {
                series: parse_opt_color(m, i, "colors", "series")?,
                negative: parse_opt_color(m, i, "colors", "negative")?,
                axis: parse_opt_color(m, i, "colors", "axis")?,
                markers: parse_opt_color(m, i, "colors", "markers")?,
                first: parse_opt_color(m, i, "colors", "first")?,
                last: parse_opt_color(m, i, "colors", "last")?,
                high: parse_opt_color(m, i, "colors", "high")?,
                low: parse_opt_color(m, i, "colors", "low")?,
            }))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "colors".into(),
                expected: TypeName::Object,
            },
            format!("ops[{i}].colors"),
        )),
    }
}

fn check_spark_axis(
    min_axis: Option<SparkAxisType>,
    max_axis: Option<SparkAxisType>,
    manual_min: Option<f64>,
    manual_max: Option<f64>,
    i: usize,
) -> Result<(), Error> {
    let bad = |field: &str| {
        Error::with_path(
            ErrorKind::InvalidSparkline {
                field: field.into(),
            },
            format!("ops[{i}].{field}"),
        )
    };
    match min_axis {
        Some(SparkAxisType::Custom) if manual_min.is_none() => return Err(bad("manualMin")),
        Some(kind) if kind != SparkAxisType::Custom && manual_min.is_some() => {
            return Err(bad("manualMin"));
        }
        None if manual_min.is_some() => return Err(bad("manualMin")),
        _ => {}
    }
    match max_axis {
        Some(SparkAxisType::Custom) if manual_max.is_none() => return Err(bad("manualMax")),
        Some(kind) if kind != SparkAxisType::Custom && manual_max.is_some() => {
            return Err(bad("manualMax"));
        }
        None if manual_max.is_some() => return Err(bad("manualMax")),
        _ => {}
    }
    Ok(())
}

fn parse_required_range(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<(String, a1::CellRef, a1::CellRef), Error> {
    match obj.get("ref") {
        Some(Value::String(s)) => {
            let (start, end) = a1::parse_range(s).map_err(|e| e.at(format!("ops[{i}].ref")))?;
            Ok((a1::format_range(start, end), start, end))
        }
        None => Err(Error::with_path(
            ErrorKind::MissingField {
                field: "ref".into(),
            },
            format!("ops[{i}].ref"),
        )),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "ref".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].ref"),
        )),
    }
}

fn parse_opt_range(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<Option<(String, a1::CellRef, a1::CellRef)>, Error> {
    match obj.get("ref") {
        None => Ok(None),
        Some(Value::String(s)) => {
            let (start, end) = a1::parse_range(s).map_err(|e| e.at(format!("ops[{i}].ref")))?;
            Ok(Some((a1::format_range(start, end), start, end)))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "ref".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].ref"),
        )),
    }
}

fn parse_opt_target(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    field: &str,
    empty: ErrorKind,
) -> Result<Option<String>, Error> {
    match obj.get(field) {
        None => Ok(None),
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

fn parse_opt_plain(
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

fn parse_hyperlink_patch(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    field: &str,
    empty: Option<ErrorKind>,
) -> Result<Patch<String>, Error> {
    match obj.get(field) {
        None => Ok(Patch::Absent),
        Some(Value::Null) => Ok(Patch::Clear),
        Some(Value::String(s)) if s.is_empty() => {
            if let Some(kind) = empty {
                Err(Error::with_path(kind, format!("ops[{i}].{field}")))
            } else {
                Ok(Patch::Set(String::new()))
            }
        }
        Some(Value::String(s)) => Ok(Patch::Set(s.clone())),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].{field}"),
        )),
    }
}

fn parse_sqref_field(
    v: Option<&Value>,
    i: usize,
    required: bool,
) -> Result<Vec<(a1::CellRef, a1::CellRef)>, Error> {
    let Some(v) = v else {
        if required {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "sqref".into(),
                },
                format!("ops[{i}].sqref"),
            ));
        }
        return Ok(Vec::new());
    };
    let parts: Vec<&str> = match v {
        Value::String(s) => s.split_whitespace().collect(),
        Value::Array(arr) => {
            let mut out = Vec::new();
            for (j, item) in arr.iter().enumerate() {
                let Some(s) = item.as_str() else {
                    return Err(Error::with_path(
                        ErrorKind::WrongType {
                            field: "sqref".into(),
                            expected: TypeName::String,
                        },
                        format!("ops[{i}].sqref[{j}]"),
                    ));
                };
                if s.trim().is_empty() {
                    return Err(Error::with_path(
                        ErrorKind::InvalidRange {
                            reason: crate::error::RangeReason::Empty,
                        },
                        format!("ops[{i}].sqref[{j}]"),
                    ));
                }
                out.push(s);
            }
            return parse_sqref_parts(&out, i);
        }
        _ => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "sqref".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].sqref"),
            ));
        }
    };
    if parts.is_empty() {
        return Err(Error::with_path(
            ErrorKind::InvalidRange {
                reason: crate::error::RangeReason::Empty,
            },
            format!("ops[{i}].sqref"),
        ));
    }
    parse_sqref_parts(&parts, i)
}

fn parse_sqref_parts(parts: &[&str], i: usize) -> Result<Vec<(a1::CellRef, a1::CellRef)>, Error> {
    let mut out = Vec::new();
    for p in parts {
        let range = a1::parse_range(p).map_err(|e| e.at(format!("ops[{i}].sqref")))?;
        out.push(range);
    }
    Ok(out)
}

fn parse_dv_type_field(
    v: Option<&Value>,
    i: usize,
    required: bool,
) -> Result<Option<DvType>, Error> {
    match v {
        None if required => Err(Error::with_path(
            ErrorKind::MissingField {
                field: "type".into(),
            },
            format!("ops[{i}].type"),
        )),
        None => Ok(None),
        Some(Value::String(s)) => DvType::parse(s).map(Some).ok_or_else(|| {
            Error::with_path(
                ErrorKind::InvalidDataValidation {
                    field: "type".into(),
                },
                format!("ops[{i}].type"),
            )
        }),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "type".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].type"),
        )),
    }
}

fn parse_dv_operator_field(
    v: Option<&Value>,
    i: usize,
    _required: bool,
) -> Result<Option<DvOperator>, Error> {
    match v {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => DvOperator::parse(s).map(Some).ok_or_else(|| {
            Error::with_path(
                ErrorKind::InvalidDataValidation {
                    field: "operator".into(),
                },
                format!("ops[{i}].operator"),
            )
        }),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "operator".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].operator"),
        )),
    }
}

fn parse_dv_operator_patch(v: Option<&Value>, i: usize) -> Result<Patch<DvOperator>, Error> {
    match v {
        None => Ok(Patch::Absent),
        Some(Value::Null) => Ok(Patch::Clear),
        Some(Value::String(s)) => {
            let op = DvOperator::parse(s).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::InvalidDataValidation {
                        field: "operator".into(),
                    },
                    format!("ops[{i}].operator"),
                )
            })?;
            Ok(Patch::Set(op))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "operator".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].operator"),
        )),
    }
}

fn strip_formula_eq(s: &str) -> String {
    let t = s.trim();
    t.strip_prefix('=')
        .map(|r| r.trim_start().to_string())
        .unwrap_or_else(|| t.to_string())
}

fn parse_dv_formula_opt(v: Option<&Value>, i: usize, field: &str) -> Result<Option<String>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => {
            let t = strip_formula_eq(s);
            if t.is_empty() {
                Ok(None)
            } else {
                Ok(Some(t))
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

fn parse_dv_formula_patch(
    v: Option<&Value>,
    i: usize,
    field: &str,
) -> Result<Patch<String>, Error> {
    match v {
        None => Ok(Patch::Absent),
        Some(Value::Null) => Ok(Patch::Clear),
        Some(Value::String(s)) => {
            let t = strip_formula_eq(s);
            if t.is_empty() {
                Ok(Patch::Clear)
            } else {
                Ok(Patch::Set(t))
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

fn parse_dv_string_opt(v: Option<&Value>, i: usize, field: &str) -> Result<Option<String>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Null) => Ok(None),
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

fn parse_dv_string_patch(v: Option<&Value>, i: usize, field: &str) -> Result<Patch<String>, Error> {
    match v {
        None => Ok(Patch::Absent),
        Some(Value::Null) => Ok(Patch::Clear),
        Some(Value::String(s)) => Ok(Patch::Set(s.clone())),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].{field}"),
        )),
    }
}

fn parse_dv_error_style_opt(v: Option<&Value>, i: usize) -> Result<Option<String>, Error> {
    match v {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) if matches!(s.as_str(), "stop" | "warning" | "information") => {
            Ok(Some(s.clone()))
        }
        Some(Value::String(_)) => Err(Error::with_path(
            ErrorKind::InvalidDataValidation {
                field: "errorStyle".into(),
            },
            format!("ops[{i}].errorStyle"),
        )),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "errorStyle".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].errorStyle"),
        )),
    }
}

fn parse_dv_error_style_patch(v: Option<&Value>, i: usize) -> Result<Patch<String>, Error> {
    match v {
        None => Ok(Patch::Absent),
        Some(Value::Null) => Ok(Patch::Clear),
        Some(Value::String(s)) if matches!(s.as_str(), "stop" | "warning" | "information") => {
            Ok(Patch::Set(s.clone()))
        }
        Some(Value::String(_)) => Err(Error::with_path(
            ErrorKind::InvalidDataValidation {
                field: "errorStyle".into(),
            },
            format!("ops[{i}].errorStyle"),
        )),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "errorStyle".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].errorStyle"),
        )),
    }
}

pub(crate) fn check_dv_fields(
    type_: DvType,
    operator: Option<DvOperator>,
    formula1: Option<&str>,
    formula2: Option<&str>,
    dropdown: Option<bool>,
    i: usize,
) -> Result<(), Error> {
    if dropdown.is_some() && type_ != DvType::List {
        return Err(Error::with_path(
            ErrorKind::InvalidDataValidation {
                field: "dropdown".into(),
            },
            format!("ops[{i}].dropdown"),
        ));
    }
    if operator.is_some() && !type_.allows_operator() {
        return Err(Error::with_path(
            ErrorKind::InvalidDataValidation {
                field: "operator".into(),
            },
            format!("ops[{i}].operator"),
        ));
    }
    let f1_ok = formula1.is_some_and(|s| !s.is_empty());
    if type_.needs_formula1() && !f1_ok {
        return Err(Error::with_path(
            ErrorKind::NeedDataValidationFormula {
                field: "formula1".into(),
            },
            format!("ops[{i}].formula1"),
        ));
    }
    let needs_f2 =
        type_.allows_operator() && operator.unwrap_or(DvOperator::Between).needs_formula2();
    let f2_ok = formula2.is_some_and(|s| !s.is_empty());
    if needs_f2 && !f2_ok {
        return Err(Error::with_path(
            ErrorKind::NeedDataValidationFormula {
                field: "formula2".into(),
            },
            format!("ops[{i}].formula2"),
        ));
    }
    if !needs_f2 && f2_ok {
        return Err(Error::with_path(
            ErrorKind::InvalidDataValidation {
                field: "formula2".into(),
            },
            format!("ops[{i}].formula2"),
        ));
    }
    Ok(())
}

fn parse_op_range(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<(a1::CellRef, a1::CellRef), Error> {
    let raw = match obj.get("range") {
        Some(Value::String(s)) => s,
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "range".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].range"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "range".into(),
                },
                format!("ops[{i}].range"),
            ));
        }
    };
    a1::parse_range(raw).map_err(|e| e.at(format!("ops[{i}].range")))
}

fn parse_required_row(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    field: &str,
) -> Result<u32, Error> {
    match obj.get(field) {
        Some(Value::Number(n)) => {
            let v = n
                .as_u64()
                .or_else(|| n.as_i64().filter(|x| *x >= 0).map(|x| x as u64));
            let Some(v) = v else {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: field.into(),
                        expected: TypeName::Integer,
                    },
                    format!("ops[{i}].{field}"),
                ));
            };
            if !(1..=1_048_576).contains(&v) {
                return Err(Error::with_path(
                    ErrorKind::InvalidRange {
                        reason: crate::error::RangeReason::RowOutOfRange { row: v as u32 },
                    },
                    format!("ops[{i}].{field}"),
                ));
            }
            Ok(v as u32)
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Integer,
            },
            format!("ops[{i}].{field}"),
        )),
        None => Err(Error::with_path(
            ErrorKind::MissingField {
                field: field.into(),
            },
            format!("ops[{i}].{field}"),
        )),
    }
}

fn parse_patch_dim(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    field: &str,
    max: f64,
    invalid: ErrorKind,
) -> Result<Patch<f64>, Error> {
    match obj.get(field) {
        None => Ok(Patch::Absent),
        Some(Value::Null) => Ok(Patch::Clear),
        Some(Value::Number(n)) => {
            let Some(v) = n.as_f64() else {
                return Err(Error::with_path(invalid, format!("ops[{i}].{field}")));
            };
            if !(v > 0.0 && v <= max) {
                return Err(Error::with_path(invalid, format!("ops[{i}].{field}")));
            }
            Ok(Patch::Set(v))
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

fn parse_new_sheet_name(obj: &serde_json::Map<String, Value>, i: usize) -> Result<String, Error> {
    match obj.get("name") {
        Some(Value::String(s)) => {
            validate_sheet_name(s, format!("ops[{i}].name"))?;
            Ok(s.clone())
        }
        None => Err(Error::with_path(
            ErrorKind::InvalidSheetName {
                reason: SheetNameReason::Missing,
            },
            format!("ops[{i}].name"),
        )),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "name".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].name"),
        )),
    }
}

fn parse_optional_after(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    allow_null: bool,
) -> Result<Option<String>, Error> {
    match obj.get("after") {
        None => Ok(None),
        Some(Value::Null) if allow_null => Ok(None),
        Some(Value::String(s)) => Ok(Some(parse_sheet_only_id(
            s,
            i,
            "after",
            IdReason::SheetOpNeedsSheet,
        )?)),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "after".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].after"),
        )),
    }
}

fn parse_sheet_only_id(
    id: &str,
    op_index: usize,
    field: &str,
    reason: IdReason,
) -> Result<String, Error> {
    if id.contains("/chart:")
        || id.contains("/pic:")
        || id.contains("/hf:")
        || id.contains("/dv:")
        || id.contains("/cf:")
        || id.contains("/table:")
        || id.contains("/pivot:")
        || id.contains("/link:")
        || id.contains("/spark:")
        || id.contains("/ole:")
        || id.contains("/model3d:")
        || id.contains("/math:")
        || id.contains("/slicer:")
        || id.contains("/editRange:")
        || id.starts_with("cell:")
    {
        return Err(Error::with_path(
            ErrorKind::InvalidId { reason },
            format!("ops[{op_index}].{field}"),
        ));
    }
    let Some(name) = id.strip_prefix("sheet:") else {
        return Err(Error::with_path(
            ErrorKind::InvalidId { reason },
            format!("ops[{op_index}].{field}"),
        ));
    };
    if name.is_empty() || name.contains('/') {
        return Err(Error::with_path(
            ErrorKind::InvalidId { reason },
            format!("ops[{op_index}].{field}"),
        ));
    }
    Ok(name.to_string())
}

pub(crate) fn validate_sheet_name(name: &str, path: impl Into<String>) -> Result<(), Error> {
    let path = path.into();
    if name.is_empty() || name.len() > 31 {
        return Err(Error::with_path(
            ErrorKind::InvalidSheetName {
                reason: SheetNameReason::Length,
            },
            path,
        ));
    }
    if name.eq_ignore_ascii_case("History") {
        return Err(Error::with_path(
            ErrorKind::InvalidSheetName {
                reason: SheetNameReason::History,
            },
            path,
        ));
    }
    if name.contains([':', '*', '?', '/', '[', ']', '\\']) {
        return Err(Error::with_path(
            ErrorKind::InvalidSheetName {
                reason: SheetNameReason::IllegalChars,
            },
            path,
        ));
    }
    Ok(())
}

fn parse_chart_title_field(v: Option<&Value>, i: usize) -> Result<Option<String>, Error> {
    match v {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "title".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].title"),
        )),
    }
}

fn validate_insert_series(
    chart_type: ChartType,
    series: &[NewSeries],
    extra: &ChartExtra,
    i: usize,
) -> Result<(), Error> {
    crate::chart::validate_insert_series_len(
        chart_type,
        series.len(),
        extra,
        &format!("ops[{i}].series"),
    )?;
    if chart_type == ChartType::Combo {
        let kinds: std::collections::HashSet<&str> = series
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
    Ok(())
}

fn parse_chart_size_pair(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    op: &str,
) -> Result<(Option<i64>, Option<i64>), Error> {
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
    Ok((cx, cy))
}

fn parse_insert_series(
    v: Option<&Value>,
    i: usize,
    _chart_type: ChartType,
    op: &str,
) -> Result<Vec<NewSeries>, Error> {
    let arr = match v {
        Some(Value::Array(a)) if !a.is_empty() => a,
        Some(Value::Array(_)) | None => {
            return Err(Error::with_path(
                ErrorKind::OpRequiresField {
                    op: op.into(),
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
                        op: op.into(),
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
) -> Result<DeleteChartOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let target = chart::parse_chart_id(&id, i)?;
    Ok(DeleteChartOp { id, target })
}

fn parse_set_chart_title(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetChartTitleOp, Error> {
    deny_keys(obj, i, &["op", "id", "title"])?;
    let id = require_id(obj, i)?;
    let target = chart::parse_chart_id(&id, i)?;
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

fn parse_set_chart_series(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetChartSeriesOp, Error> {
    deny_keys(
        obj,
        i,
        &["op", "id", "name", "categories", "values", "sizes"],
    )?;
    let id = require_id(obj, i)?;
    let (target, index) = chart::parse_series_id(&id, i)?;
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

fn parse_add_chart_series(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<AddChartSeriesOp, Error> {
    deny_keys(
        obj,
        i,
        &["op", "id", "name", "categories", "values", "sizes", "type"],
    )?;
    let id = require_id(obj, i)?;
    let target = chart::parse_chart_id(&id, i)?;
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
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteChartSeriesOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let (target, index) = chart::parse_series_id(&id, i)?;
    Ok(DeleteChartSeriesOp { id, target, index })
}

fn parse_set_chart_type(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetChartTypeOp, Error> {
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
    let target = chart::parse_chart_id(&id, i)?;
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
    Ok(SetChartTypeOp {
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
) -> Result<SetChartAxisTitleOp, Error> {
    deny_keys(obj, i, &["op", "id", "axis", "title"])?;
    let id = require_id(obj, i)?;
    let target = chart::parse_chart_id(&id, i)?;
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

fn parse_set_chart_legend(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetChartLegendOp, Error> {
    deny_keys(obj, i, &["op", "id", "show", "pos"])?;
    let id = require_id(obj, i)?;
    let target = chart::parse_chart_id(&id, i)?;
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
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetChartDataLabelsOp, Error> {
    deny_keys(obj, i, &["op", "id", "show"])?;
    let id = require_id(obj, i)?;
    let target = chart::parse_chart_id(&id, i)?;
    let show = parse_required_bool(obj.get("show"), i, "setChartDataLabels", "show")?;
    Ok(SetChartDataLabelsOp { id, target, show })
}

fn parse_set_chart_size(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetChartSizeOp, Error> {
    deny_keys(obj, i, &["op", "id", "cx", "cy", "to", "toOffX", "toOffY"])?;
    let id = require_id(obj, i)?;
    let target = chart::parse_chart_id(&id, i)?;
    let s = parse_drawing_size(obj, i, "setChartSize")?;
    Ok(SetChartSizeOp {
        id,
        target,
        cx: s.cx,
        cy: s.cy,
        to: s.to,
        to_off_x: s.to_off_x,
        to_off_y: s.to_off_y,
    })
}

fn parse_set_chart_anchor(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetChartAnchorOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op", "id", "anchor", "from", "to", "fromOffX", "fromOffY", "toOffX", "toOffY", "posX",
            "posY", "editAs", "cx", "cy",
        ],
    )?;
    let id = require_id(obj, i)?;
    let target = chart::parse_chart_id(&id, i)?;
    let p = parse_drawing_anchor_patch(obj, i, "setChartAnchor")?;
    Ok(SetChartAnchorOp {
        id,
        target,
        anchor: p.anchor,
        from: p.from,
        to: p.to,
        from_off_x: p.from_off_x,
        from_off_y: p.from_off_y,
        to_off_x: p.to_off_x,
        to_off_y: p.to_off_y,
        pos_x: p.pos_x,
        pos_y: p.pos_y,
        edit_as: p.edit_as,
        cx: p.cx,
        cy: p.cy,
    })
}

struct InsertAnchorSpec {
    from: a1::CellRef,
    to: Option<a1::CellRef>,
    anchor: picture::AnchorKind,
    from_off_x: i64,
    from_off_y: i64,
    to_off_x: i64,
    to_off_y: i64,
    cx: Option<i64>,
    cy: Option<i64>,
    rel: Option<RelBox>,
}

fn parse_unit_frac(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    field: &str,
) -> Result<Option<f64>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::Number(n)) => {
            let v = n.as_f64().ok_or_else(|| {
                Error::with_path(
                    ErrorKind::WrongType {
                        field: field.into(),
                        expected: TypeName::Integer,
                    },
                    format!("ops[{i}].{field}"),
                )
            })?;
            if !(0.0..=1.0).contains(&v) {
                return Err(Error::with_path(
                    ErrorKind::InvalidXlsxAnchor,
                    format!("ops[{i}].{field}"),
                ));
            }
            Ok(Some(v))
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

fn parse_rel_box(obj: &serde_json::Map<String, Value>, i: usize) -> Result<Option<RelBox>, Error> {
    let fx = parse_unit_frac(obj, i, "fromX")?;
    let fy = parse_unit_frac(obj, i, "fromY")?;
    let tx = parse_unit_frac(obj, i, "toX")?;
    let ty = parse_unit_frac(obj, i, "toY")?;
    match (fx, fy, tx, ty) {
        (None, None, None, None) => Ok(None),
        (Some(from_x), Some(from_y), Some(to_x), Some(to_y)) => Ok(Some(RelBox {
            from_x,
            from_y,
            to_x,
            to_y,
        })),
        _ => Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "insert".into(),
                field: "fromX/fromY/toX/toY together".into(),
            },
            format!("ops[{i}]"),
        )),
    }
}

fn parse_rel_box_patch(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<Option<RelBoxPatch>, Error> {
    let from_x = parse_unit_frac(obj, i, "fromX")?;
    let from_y = parse_unit_frac(obj, i, "fromY")?;
    let to_x = parse_unit_frac(obj, i, "toX")?;
    let to_y = parse_unit_frac(obj, i, "toY")?;
    if from_x.is_none() && from_y.is_none() && to_x.is_none() && to_y.is_none() {
        return Ok(None);
    }
    Ok(Some(RelBoxPatch {
        from_x,
        from_y,
        to_x,
        to_y,
    }))
}

fn parse_insert_anchor(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    op: &str,
) -> Result<InsertAnchorSpec, Error> {
    let rel = parse_rel_box(obj, i)?;
    if rel.is_some() {
        for field in [
            "from", "to", "anchor", "fromOffX", "fromOffY", "toOffX", "toOffY",
        ] {
            if obj.contains_key(field) {
                return Err(Error::with_path(
                    ErrorKind::UnknownField {
                        field: field.into(),
                    },
                    format!("ops[{i}].{field}"),
                ));
            }
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
        return Ok(InsertAnchorSpec {
            from: a1::CellRef { col: 1, row: 1 },
            to: None,
            anchor: picture::AnchorKind::RelSize,
            from_off_x: 0,
            from_off_y: 0,
            to_off_x: 0,
            to_off_y: 0,
            cx,
            cy,
            rel,
        });
    }
    let from = parse_required_a1(obj, i, "from")?;
    let anchor = match obj.get("anchor") {
        None => picture::AnchorKind::OneCell,
        Some(Value::String(s)) => picture::AnchorKind::parse_insert(s).ok_or_else(|| {
            Error::with_path(ErrorKind::InvalidXlsxAnchor, format!("ops[{i}].anchor"))
        })?,
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "anchor".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].anchor"),
            ));
        }
    };
    let to = parse_opt_a1(obj.get("to"), i, "to")?;
    if anchor == picture::AnchorKind::TwoCell && to.is_none() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: op.into(),
                field: "to".into(),
            },
            format!("ops[{i}].to"),
        ));
    }
    if anchor == picture::AnchorKind::OneCell && to.is_some() {
        return Err(Error::with_path(
            ErrorKind::UnknownField { field: "to".into() },
            format!("ops[{i}].to"),
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
    Ok(InsertAnchorSpec {
        from,
        to,
        anchor,
        from_off_x: parse_off_field(obj.get("fromOffX"), i, "fromOffX")?.unwrap_or(0),
        from_off_y: parse_off_field(obj.get("fromOffY"), i, "fromOffY")?.unwrap_or(0),
        to_off_x: parse_off_field(obj.get("toOffX"), i, "toOffX")?.unwrap_or(0),
        to_off_y: parse_off_field(obj.get("toOffY"), i, "toOffY")?.unwrap_or(0),
        cx,
        cy,
        rel: None,
    })
}

fn parse_opt_text(obj: &serde_json::Map<String, Value>, i: usize) -> Result<Option<String>, Error> {
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

fn parse_require_text(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    op: &str,
) -> Result<String, Error> {
    match obj.get("text") {
        Some(Value::String(s)) => Ok(s.clone()),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "text".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].text"),
        )),
        None => Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: op.into(),
                field: "text".into(),
            },
            format!("ops[{i}].text"),
        )),
    }
}

struct DrawingSizePatch {
    cx: Option<i64>,
    cy: Option<i64>,
    to: Option<a1::CellRef>,
    to_off_x: Option<i64>,
    to_off_y: Option<i64>,
}

fn parse_drawing_size(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    op: &str,
) -> Result<DrawingSizePatch, Error> {
    let cx = parse_emu_field(obj.get("cx"), i, "cx")?;
    let cy = parse_emu_field(obj.get("cy"), i, "cy")?;
    let to = parse_opt_a1(obj.get("to"), i, "to")?;
    let to_off_x = parse_off_field(obj.get("toOffX"), i, "toOffX")?;
    let to_off_y = parse_off_field(obj.get("toOffY"), i, "toOffY")?;
    if (cx.is_some() || cy.is_some()) && (to.is_some() || to_off_x.is_some() || to_off_y.is_some())
    {
        return Err(Error::with_path(
            ErrorKind::MutuallyExclusive {
                a: "cx/cy".into(),
                b: "to".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    if (to_off_x.is_some() || to_off_y.is_some()) && to.is_none() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: op.into(),
                field: "to".into(),
            },
            format!("ops[{i}].to"),
        ));
    }
    if cx.is_none() && cy.is_none() && to.is_none() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: op.into(),
                field: "cx or cy or to".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(DrawingSizePatch {
        cx,
        cy,
        to,
        to_off_x,
        to_off_y,
    })
}

struct DrawingAnchorPatch {
    anchor: Option<picture::AnchorKind>,
    from: Option<a1::CellRef>,
    to: Option<a1::CellRef>,
    from_off_x: Option<i64>,
    from_off_y: Option<i64>,
    to_off_x: Option<i64>,
    to_off_y: Option<i64>,
    pos_x: Option<i64>,
    pos_y: Option<i64>,
    edit_as: Option<picture::EditAs>,
    cx: Option<i64>,
    cy: Option<i64>,
    rel: Option<RelBoxPatch>,
}

fn parse_drawing_anchor_patch(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    op: &str,
) -> Result<DrawingAnchorPatch, Error> {
    let anchor = match obj.get("anchor") {
        None => None,
        Some(Value::String(s)) => Some(picture::AnchorKind::parse_set(s).ok_or_else(|| {
            Error::with_path(ErrorKind::InvalidXlsxAnchor, format!("ops[{i}].anchor"))
        })?),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "anchor".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].anchor"),
            ));
        }
    };
    let edit_as = match obj.get("editAs") {
        None => None,
        Some(Value::String(s)) => Some(picture::EditAs::parse(s).ok_or_else(|| {
            Error::with_path(ErrorKind::InvalidXlsxEditAs, format!("ops[{i}].editAs"))
        })?),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "editAs".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].editAs"),
            ));
        }
    };
    let from = parse_opt_a1(obj.get("from"), i, "from")?;
    let to = parse_opt_a1(obj.get("to"), i, "to")?;
    let from_off_x = parse_off_field(obj.get("fromOffX"), i, "fromOffX")?;
    let from_off_y = parse_off_field(obj.get("fromOffY"), i, "fromOffY")?;
    let to_off_x = parse_off_field(obj.get("toOffX"), i, "toOffX")?;
    let to_off_y = parse_off_field(obj.get("toOffY"), i, "toOffY")?;
    let pos_x = parse_off_field(obj.get("posX"), i, "posX")?;
    let pos_y = parse_off_field(obj.get("posY"), i, "posY")?;
    let cx = parse_emu_field(obj.get("cx"), i, "cx")?;
    let cy = parse_emu_field(obj.get("cy"), i, "cy")?;
    let rel = parse_rel_box_patch(obj, i)?;
    let has_any = anchor.is_some()
        || from.is_some()
        || to.is_some()
        || from_off_x.is_some()
        || from_off_y.is_some()
        || to_off_x.is_some()
        || to_off_y.is_some()
        || pos_x.is_some()
        || pos_y.is_some()
        || edit_as.is_some()
        || cx.is_some()
        || cy.is_some()
        || rel.is_some();
    if !has_any {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: op.into(),
                field: "anchor, from, to, offsets, pos, editAs, or size".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(DrawingAnchorPatch {
        anchor,
        from,
        to,
        from_off_x,
        from_off_y,
        to_off_x,
        to_off_y,
        pos_x,
        pos_y,
        edit_as,
        cx,
        cy,
        rel,
    })
}

fn parse_insert_textbox(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertTextBoxOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op",
            "id",
            "text",
            "cx",
            "cy",
            "from",
            "to",
            "anchor",
            "fromOffX",
            "fromOffY",
            "toOffX",
            "toOffY",
            "fromX",
            "fromY",
            "toX",
            "toY",
            "sz",
            "color",
            "textFill",
            "textOutline",
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
    let sheet = shape::parse_insert_sheet_id(&id, i, IdReason::InsertTextBoxNeedsSheet)?;
    let a = parse_insert_anchor(obj, i, "insertTextBox")?;
    Ok(InsertTextBoxOp {
        sheet,
        text: parse_opt_text(obj, i)?,
        cx: a.cx,
        cy: a.cy,
        from: a.from,
        to: a.to,
        anchor: a.anchor,
        from_off_x: a.from_off_x,
        from_off_y: a.from_off_y,
        to_off_x: a.to_off_x,
        to_off_y: a.to_off_y,
        format: crate::drawing_text::parse(obj, &format!("ops[{i}]"), false)?,
        rel: a.rel,
    })
}

fn parse_insert_shape(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertShapeOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op",
            "id",
            "text",
            "cx",
            "cy",
            "from",
            "to",
            "anchor",
            "fromOffX",
            "fromOffY",
            "toOffX",
            "toOffY",
            "fromX",
            "fromY",
            "toX",
            "toY",
            "prst",
            "sz",
            "color",
            "textFill",
            "textOutline",
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
    let sheet = shape::parse_insert_sheet_id(&id, i, IdReason::InsertShapeNeedsSheet)?;
    let a = parse_insert_anchor(obj, i, "insertShape")?;
    Ok(InsertShapeOp {
        sheet,
        text: parse_opt_text(obj, i)?,
        cx: a.cx,
        cy: a.cy,
        from: a.from,
        to: a.to,
        anchor: a.anchor,
        from_off_x: a.from_off_x,
        from_off_y: a.from_off_y,
        to_off_x: a.to_off_x,
        to_off_y: a.to_off_y,
        prst: parse_shape_prst(obj, i)?,
        format: crate::drawing_text::parse(obj, &format!("ops[{i}]"), false)?,
        rel: a.rel,
    })
}

fn parse_insert_connector(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertConnectorOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op", "id", "cx", "cy", "from", "to", "anchor", "fromOffX", "fromOffY", "toOffX",
            "toOffY", "fromX", "fromY", "toX", "toY", "prst",
        ],
    )?;
    let id = require_id(obj, i)?;
    let sheet = shape::parse_insert_sheet_id(&id, i, IdReason::InsertConnectorNeedsSheet)?;
    let a = parse_insert_anchor(obj, i, "insertConnector")?;
    Ok(InsertConnectorOp {
        sheet,
        cx: a.cx,
        cy: a.cy,
        from: a.from,
        to: a.to,
        anchor: a.anchor,
        from_off_x: a.from_off_x,
        from_off_y: a.from_off_y,
        to_off_x: a.to_off_x,
        to_off_y: a.to_off_y,
        prst: parse_connector_prst(obj, i)?,
        rel: a.rel,
    })
}

fn parse_shape_prst(obj: &serde_json::Map<String, Value>, i: usize) -> Result<String, Error> {
    match obj.get("prst") {
        None => Ok("rect".to_string()),
        Some(Value::String(s)) => {
            if crate::pptx::shape::is_writable_shape_prst(s) {
                Ok(s.clone())
            } else {
                Err(Error::with_path(
                    ErrorKind::InvalidShapePrst,
                    format!("ops[{i}].prst"),
                ))
            }
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "prst".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].prst"),
        )),
    }
}

fn parse_connector_prst(obj: &serde_json::Map<String, Value>, i: usize) -> Result<String, Error> {
    match obj.get("prst") {
        None => Ok("straightConnector1".to_string()),
        Some(Value::String(s)) => {
            if s == "line" || crate::pptx::connector::is_writable_prst(s) {
                Ok(s.clone())
            } else {
                Err(Error::with_path(
                    ErrorKind::InvalidConnectorPrst,
                    format!("ops[{i}].prst"),
                ))
            }
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "prst".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].prst"),
        )),
    }
}

fn parse_delete_textbox(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteTextBoxOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let target = shape::parse_sp_id(&id, i)?;
    Ok(DeleteTextBoxOp { id, target })
}

fn parse_delete_shape(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteShapeOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let target = shape::parse_shape_id(&id, i)?;
    Ok(DeleteShapeOp { id, target })
}

fn parse_delete_connector(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteConnectorOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let target = shape::parse_cxn_id(&id, i)?;
    Ok(DeleteConnectorOp { id, target })
}

fn parse_group_shapes(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<GroupShapesOp, Error> {
    deny_keys(obj, i, &["op", "id", "ids"])?;
    let id = require_id(obj, i)?;
    let sheet = shape::parse_insert_sheet_id(&id, i, IdReason::GroupNeedsSheet)?;
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
        let parsed =
            shape::parse_drawing_obj_id(mid, i).map_err(|e| e.at(format!("ops[{i}].ids[{k}]")))?;
        members.push(parsed);
    }
    Ok(GroupShapesOp {
        sheet,
        ids,
        members,
    })
}

fn parse_ungroup(obj: &serde_json::Map<String, Value>, i: usize) -> Result<UngroupOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let (target, kind) = shape::parse_drawing_obj_id(&id, i)?;
    Ok(UngroupOp { id, target, kind })
}

fn parse_insert_ole(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<ole::InsertOleOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op", "id", "file", "cx", "cy", "from", "to", "anchor", "fromOffX", "fromOffY",
            "toOffX", "toOffY",
        ],
    )?;
    let id = require_id(obj, i)?;
    let sheet = parse_sheet_only_id(&id, i, "id", IdReason::InsertOleNeedsSheet)?;
    let file = require_file(obj, i, "insertOle")?;
    let cx = parse_emu_field(obj.get("cx"), i, "cx")?;
    let cy = parse_emu_field(obj.get("cy"), i, "cy")?;
    if cx.is_some() != cy.is_some() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "insertOle".into(),
                field: "cx and cy together".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    let from = match obj.get("from") {
        Some(Value::String(s)) => a1::parse_cell(s).map_err(|e| e.at(format!("ops[{i}].from")))?,
        None => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "from".into(),
                },
                format!("ops[{i}].from"),
            ));
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
    let anchor = match obj.get("anchor") {
        None => picture::AnchorKind::OneCell,
        Some(Value::String(s)) => picture::AnchorKind::parse_insert(s).ok_or_else(|| {
            Error::with_path(ErrorKind::InvalidXlsxAnchor, format!("ops[{i}].anchor"))
        })?,
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "anchor".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].anchor"),
            ));
        }
    };
    let to = match obj.get("to") {
        None => None,
        Some(Value::String(s)) => {
            Some(a1::parse_cell(s).map_err(|e| e.at(format!("ops[{i}].to")))?)
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "to".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].to"),
            ));
        }
    };
    if anchor == picture::AnchorKind::TwoCell && to.is_none() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "insertOle".into(),
                field: "to".into(),
            },
            format!("ops[{i}].to"),
        ));
    }
    if anchor == picture::AnchorKind::OneCell && to.is_some() {
        return Err(Error::with_path(
            ErrorKind::UnknownField { field: "to".into() },
            format!("ops[{i}].to"),
        ));
    }
    Ok(ole::InsertOleOp {
        sheet,
        file,
        cx,
        cy,
        from,
        to,
        anchor,
        from_off_x: parse_off_field(obj.get("fromOffX"), i, "fromOffX")?.unwrap_or(0),
        from_off_y: parse_off_field(obj.get("fromOffY"), i, "fromOffY")?.unwrap_or(0),
        to_off_x: parse_off_field(obj.get("toOffX"), i, "toOffX")?.unwrap_or(0),
        to_off_y: parse_off_field(obj.get("toOffY"), i, "toOffY")?.unwrap_or(0),
    })
}

fn parse_delete_ole(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<ole::DeleteOleOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let target = ole::parse_ole_id(&id, &format!("ops[{i}].id"))?;
    Ok(ole::DeleteOleOp { id, target })
}

fn parse_insert_model3d(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<super::model3d::InsertModel3dOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op", "id", "file", "cx", "cy", "from", "to", "anchor", "fromOffX", "fromOffY",
            "toOffX", "toOffY",
        ],
    )?;
    let id = require_id(obj, i)?;
    let sheet = parse_sheet_only_id(&id, i, "id", IdReason::InsertModel3dNeedsSheet)?;
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
    let from = match obj.get("from") {
        Some(Value::String(s)) => a1::parse_cell(s).map_err(|e| e.at(format!("ops[{i}].from")))?,
        None => a1::CellRef { col: 1, row: 1 },
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
    let anchor = match obj.get("anchor") {
        None => picture::AnchorKind::OneCell,
        Some(Value::String(s)) => picture::AnchorKind::parse_insert(s).ok_or_else(|| {
            Error::with_path(ErrorKind::InvalidXlsxAnchor, format!("ops[{i}].anchor"))
        })?,
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "anchor".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].anchor"),
            ));
        }
    };
    let to = match obj.get("to") {
        None => None,
        Some(Value::String(s)) => {
            Some(a1::parse_cell(s).map_err(|e| e.at(format!("ops[{i}].to")))?)
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "to".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].to"),
            ));
        }
    };
    if anchor == picture::AnchorKind::TwoCell && to.is_none() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "insertModel3d".into(),
                field: "to".into(),
            },
            format!("ops[{i}].to"),
        ));
    }
    if anchor == picture::AnchorKind::OneCell && to.is_some() {
        return Err(Error::with_path(
            ErrorKind::UnknownField { field: "to".into() },
            format!("ops[{i}].to"),
        ));
    }
    Ok(super::model3d::InsertModel3dOp {
        sheet,
        file,
        cx,
        cy,
        from,
        to,
        anchor,
        from_off_x: parse_off_field(obj.get("fromOffX"), i, "fromOffX")?.unwrap_or(0),
        from_off_y: parse_off_field(obj.get("fromOffY"), i, "fromOffY")?.unwrap_or(0),
        to_off_x: parse_off_field(obj.get("toOffX"), i, "toOffX")?.unwrap_or(0),
        to_off_y: parse_off_field(obj.get("toOffY"), i, "toOffY")?.unwrap_or(0),
    })
}

fn parse_replace_model3d(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<super::model3d::ReplaceModel3dOp, Error> {
    deny_keys(obj, i, &["op", "id", "file"])?;
    let id = require_id(obj, i)?;
    let target = super::model3d::parse_id(&id, &format!("ops[{i}].id"))?;
    let file = require_file(obj, i, "replaceModel3d")?;
    Ok(super::model3d::ReplaceModel3dOp { id, target, file })
}

fn parse_delete_model3d(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<super::model3d::DeleteModel3dOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let target = super::model3d::parse_id(&id, &format!("ops[{i}].id"))?;
    Ok(super::model3d::DeleteModel3dOp { id, target })
}

fn parse_insert_math(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<super::omml::InsertMathOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op", "id", "text", "cx", "cy", "from", "to", "anchor", "fromOffX", "fromOffY",
            "toOffX", "toOffY", "expr", "kind",
        ],
    )?;
    crate::drawing_math::reject_expr_kind(obj, &format!("ops[{i}]"))?;
    let id = require_id(obj, i)?;
    let sheet = parse_sheet_only_id(&id, i, "id", IdReason::InsertMathNeedsSheet)?;
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
    let from = match obj.get("from") {
        Some(Value::String(s)) => a1::parse_cell(s).map_err(|e| e.at(format!("ops[{i}].from")))?,
        None => a1::CellRef { col: 1, row: 1 },
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
    let anchor = match obj.get("anchor") {
        None => picture::AnchorKind::OneCell,
        Some(Value::String(s)) => picture::AnchorKind::parse_insert(s).ok_or_else(|| {
            Error::with_path(ErrorKind::InvalidXlsxAnchor, format!("ops[{i}].anchor"))
        })?,
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "anchor".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].anchor"),
            ));
        }
    };
    let to = match obj.get("to") {
        None => None,
        Some(Value::String(s)) => {
            Some(a1::parse_cell(s).map_err(|e| e.at(format!("ops[{i}].to")))?)
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "to".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].to"),
            ));
        }
    };
    if anchor == picture::AnchorKind::TwoCell && to.is_none() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "insertMath".into(),
                field: "to".into(),
            },
            format!("ops[{i}].to"),
        ));
    }
    if anchor == picture::AnchorKind::OneCell && to.is_some() {
        return Err(Error::with_path(
            ErrorKind::UnknownField { field: "to".into() },
            format!("ops[{i}].to"),
        ));
    }
    Ok(super::omml::InsertMathOp {
        sheet,
        text,
        cx,
        cy,
        from,
        to,
        anchor,
        from_off_x: parse_off_field(obj.get("fromOffX"), i, "fromOffX")?.unwrap_or(0),
        from_off_y: parse_off_field(obj.get("fromOffY"), i, "fromOffY")?.unwrap_or(0),
        to_off_x: parse_off_field(obj.get("toOffX"), i, "toOffX")?.unwrap_or(0),
        to_off_y: parse_off_field(obj.get("toOffY"), i, "toOffY")?.unwrap_or(0),
    })
}

fn parse_set_math(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<super::omml::SetMathOp, Error> {
    deny_keys(obj, i, &["op", "id", "text", "expr", "kind"])?;
    crate::drawing_math::reject_expr_kind(obj, &format!("ops[{i}]"))?;
    let id = require_id(obj, i)?;
    let target = super::omml::parse_id(&id, &format!("ops[{i}].id"))?;
    let text = crate::drawing_math::require_text(obj, &format!("ops[{i}]"))?;
    Ok(super::omml::SetMathOp { id, target, text })
}

fn parse_delete_math(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<super::omml::DeleteMathOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let target = super::omml::parse_id(&id, &format!("ops[{i}].id"))?;
    Ok(super::omml::DeleteMathOp { id, target })
}

fn parse_insert_slicer(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<slicer::InsertSlicerOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op", "id", "source", "field", "kind", "level", "cx", "cy", "from", "to", "anchor",
            "fromOffX", "fromOffY", "toOffX", "toOffY",
        ],
    )?;
    let id = require_id(obj, i)?;
    let sheet = parse_sheet_only_id(&id, i, "id", IdReason::InsertSlicerNeedsSheet)?;
    let source_raw = match obj.get("source") {
        Some(Value::String(s)) if !s.is_empty() => s.clone(),
        None => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "source".into(),
                },
                format!("ops[{i}].source"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "source".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].source"),
            ));
        }
    };
    let source = parse_slicer_source(&source_raw, i)?;
    let field = match obj.get("field") {
        Some(Value::String(s)) if !s.is_empty() => s.clone(),
        None => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "field".into(),
                },
                format!("ops[{i}].field"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "field".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].field"),
            ));
        }
    };
    let cx = parse_emu_field(obj.get("cx"), i, "cx")?;
    let cy = parse_emu_field(obj.get("cy"), i, "cy")?;
    if cx.is_some() != cy.is_some() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "insertSlicer".into(),
                field: "cx and cy together".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    let from = match obj.get("from") {
        Some(Value::String(s)) => a1::parse_cell(s).map_err(|e| e.at(format!("ops[{i}].from")))?,
        None => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "from".into(),
                },
                format!("ops[{i}].from"),
            ));
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
    let anchor = match obj.get("anchor") {
        None => picture::AnchorKind::OneCell,
        Some(Value::String(s)) => picture::AnchorKind::parse_insert(s).ok_or_else(|| {
            Error::with_path(ErrorKind::InvalidXlsxAnchor, format!("ops[{i}].anchor"))
        })?,
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "anchor".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].anchor"),
            ));
        }
    };
    let to = match obj.get("to") {
        None => None,
        Some(Value::String(s)) => {
            Some(a1::parse_cell(s).map_err(|e| e.at(format!("ops[{i}].to")))?)
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "to".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].to"),
            ));
        }
    };
    if anchor == picture::AnchorKind::TwoCell && to.is_none() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "insertSlicer".into(),
                field: "to".into(),
            },
            format!("ops[{i}].to"),
        ));
    }
    if anchor == picture::AnchorKind::OneCell && to.is_some() {
        return Err(Error::with_path(
            ErrorKind::UnknownField { field: "to".into() },
            format!("ops[{i}].to"),
        ));
    }
    let kind = match obj.get("kind") {
        None => slicer::SlicerKind::Slicer,
        Some(Value::String(s)) if s == "slicer" => slicer::SlicerKind::Slicer,
        Some(Value::String(s)) if s == "timeline" => slicer::SlicerKind::Timeline,
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidSlicer {
                    field: "kind".into(),
                },
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
    let level = match obj.get("level") {
        None => 2,
        Some(_) if kind != slicer::SlicerKind::Timeline => {
            return Err(Error::with_path(
                ErrorKind::InvalidSlicer {
                    field: "level".into(),
                },
                format!("ops[{i}].level"),
            ));
        }
        Some(Value::Number(n)) => {
            let v = n.as_u64().and_then(|x| u32::try_from(x).ok());
            match v {
                Some(v) if v <= 3 => v,
                _ => {
                    return Err(Error::with_path(
                        ErrorKind::InvalidSlicer {
                            field: "level".into(),
                        },
                        format!("ops[{i}].level"),
                    ));
                }
            }
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "level".into(),
                    expected: TypeName::Integer,
                },
                format!("ops[{i}].level"),
            ));
        }
    };
    Ok(slicer::InsertSlicerOp {
        sheet,
        source,
        field,
        kind,
        level,
        cx,
        cy,
        from,
        to,
        anchor,
        from_off_x: parse_off_field(obj.get("fromOffX"), i, "fromOffX")?.unwrap_or(0),
        from_off_y: parse_off_field(obj.get("fromOffY"), i, "fromOffY")?.unwrap_or(0),
        to_off_x: parse_off_field(obj.get("toOffX"), i, "toOffX")?.unwrap_or(0),
        to_off_y: parse_off_field(obj.get("toOffY"), i, "toOffY")?.unwrap_or(0),
    })
}

fn parse_slicer_source(raw: &str, i: usize) -> Result<slicer::SlicerSource, Error> {
    if raw.contains("/table:") {
        let (sheet, index) =
            parse_table_id(raw, i).map_err(|e| e.at(format!("ops[{i}].source")))?;
        return Ok(slicer::SlicerSource::Table { sheet, index });
    }
    if raw.contains("/pivot:") {
        let (sheet, index) =
            parse_pivot_id(raw, i).map_err(|e| e.at(format!("ops[{i}].source")))?;
        return Ok(slicer::SlicerSource::Pivot { sheet, index });
    }
    Err(Error::with_path(
        ErrorKind::InvalidId {
            reason: IdReason::InsertSlicerNeedsTableOrPivot,
        },
        format!("ops[{i}].source"),
    ))
}

fn parse_delete_slicer(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<slicer::DeleteSlicerOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let target = slicer::parse_slicer_id(&id, &format!("ops[{i}].id"))?;
    Ok(slicer::DeleteSlicerOp { id, target })
}

fn parse_insert_custom_view(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<custom_view::InsertCustomViewOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op",
            "id",
            "name",
            "includePrintSettings",
            "includeHiddenRowCol",
        ],
    )?;
    let id = require_id(obj, i)?;
    if id != "xlsx:workbook" {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::InsertCustomViewNeedsWorkbook,
            },
            format!("ops[{i}].id"),
        ));
    }
    let name = match obj.get("name") {
        Some(Value::String(s)) if !s.is_empty() => s.clone(),
        None => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "name".into(),
                },
                format!("ops[{i}].name"),
            ));
        }
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidCustomView {
                    field: "name".into(),
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
    Ok(custom_view::InsertCustomViewOp {
        name,
        include_print_settings: parse_opt_bool(obj, i, "includePrintSettings")?.unwrap_or(true),
        include_hidden_row_col: parse_opt_bool(obj, i, "includeHiddenRowCol")?.unwrap_or(true),
    })
}

fn parse_delete_custom_view(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<custom_view::DeleteCustomViewOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let index = custom_view::parse_custom_view_id(&id, &format!("ops[{i}].id"))?;
    Ok(custom_view::DeleteCustomViewOp { id, index })
}

fn parse_show_custom_view(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<custom_view::ShowCustomViewOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let index = custom_view::parse_custom_view_id(&id, &format!("ops[{i}].id"))?;
    Ok(custom_view::ShowCustomViewOp { id, index })
}

fn parse_insert_connection(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<connection::InsertConnectionOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op",
            "id",
            "name",
            "type",
            "url",
            "sourceFile",
            "description",
        ],
    )?;
    let id = require_id(obj, i)?;
    if id != "xlsx:workbook" {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::InsertConnectionNeedsWorkbook,
            },
            format!("ops[{i}].id"),
        ));
    }
    let name = match obj.get("name") {
        Some(Value::String(s)) if !s.is_empty() => s.clone(),
        None => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "name".into(),
                },
                format!("ops[{i}].name"),
            ));
        }
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidConnection {
                    field: "name".into(),
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
    let kind = match obj.get("type") {
        Some(Value::String(s)) if s == "web" => connection::ConnectionKind::Web,
        Some(Value::String(s)) if s == "text" => connection::ConnectionKind::Text,
        None => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "type".into(),
                },
                format!("ops[{i}].type"),
            ));
        }
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidConnection {
                    field: "type".into(),
                },
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
    let url = parse_opt_nonempty_string(obj, i, "url")?;
    let source_file = parse_opt_nonempty_string(obj, i, "sourceFile")?;
    let description = match obj.get("description") {
        None => None,
        Some(Value::String(s)) => Some(s.clone()),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "description".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].description"),
            ));
        }
    };
    match kind {
        connection::ConnectionKind::Web => {
            if source_file.is_some() {
                return Err(Error::with_path(
                    ErrorKind::InvalidConnection {
                        field: "sourceFile".into(),
                    },
                    format!("ops[{i}].sourceFile"),
                ));
            }
            if url.is_none() {
                return Err(Error::with_path(
                    ErrorKind::MissingField {
                        field: "url".into(),
                    },
                    format!("ops[{i}].url"),
                ));
            }
        }
        connection::ConnectionKind::Text => {
            if url.is_some() {
                return Err(Error::with_path(
                    ErrorKind::InvalidConnection {
                        field: "url".into(),
                    },
                    format!("ops[{i}].url"),
                ));
            }
            if source_file.is_none() {
                return Err(Error::with_path(
                    ErrorKind::MissingField {
                        field: "sourceFile".into(),
                    },
                    format!("ops[{i}].sourceFile"),
                ));
            }
        }
    }
    Ok(connection::InsertConnectionOp {
        name,
        kind,
        url,
        source_file,
        description,
    })
}

fn parse_opt_nonempty_string(
    obj: &serde_json::Map<String, Value>,
    i: usize,
    field: &str,
) -> Result<Option<String>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::String(s)) if !s.is_empty() => Ok(Some(s.clone())),
        Some(Value::String(_)) => Err(Error::with_path(
            ErrorKind::InvalidConnection {
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

fn parse_delete_connection(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<connection::DeleteConnectionOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let key = connection::parse_connection_id(&id, &format!("ops[{i}].id"))?;
    Ok(connection::DeleteConnectionOp { id, key })
}

fn parse_insert_external_link(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<extlink::InsertExternalLinkOp, Error> {
    deny_keys(obj, i, &["op", "id", "path", "sheets"])?;
    let id = require_id(obj, i)?;
    if id != "xlsx:workbook" {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::InsertExternalLinkNeedsWorkbook,
            },
            format!("ops[{i}].id"),
        ));
    }
    let path = match obj.get("path") {
        Some(Value::String(s)) if !s.is_empty() => s.clone(),
        None => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "path".into(),
                },
                format!("ops[{i}].path"),
            ));
        }
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidExternalLink {
                    field: "path".into(),
                },
                format!("ops[{i}].path"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "path".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].path"),
            ));
        }
    };
    let sheets = match obj.get("sheets") {
        None => vec!["Sheet1".into()],
        Some(Value::Array(arr)) => {
            if arr.is_empty() {
                return Err(Error::with_path(
                    ErrorKind::InvalidExternalLink {
                        field: "sheets".into(),
                    },
                    format!("ops[{i}].sheets"),
                ));
            }
            let mut out = Vec::with_capacity(arr.len());
            for (j, v) in arr.iter().enumerate() {
                match v {
                    Value::String(s) if !s.is_empty() => out.push(s.clone()),
                    Value::String(_) => {
                        return Err(Error::with_path(
                            ErrorKind::InvalidExternalLink {
                                field: "sheets".into(),
                            },
                            format!("ops[{i}].sheets[{j}]"),
                        ));
                    }
                    _ => {
                        return Err(Error::with_path(
                            ErrorKind::WrongType {
                                field: "sheets".into(),
                                expected: TypeName::String,
                            },
                            format!("ops[{i}].sheets[{j}]"),
                        ));
                    }
                }
            }
            out
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "sheets".into(),
                    expected: TypeName::Array,
                },
                format!("ops[{i}].sheets"),
            ));
        }
    };
    Ok(extlink::InsertExternalLinkOp { path, sheets })
}

fn parse_delete_external_link(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<extlink::DeleteExternalLinkOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let index = extlink::parse_external_link_id(&id, &format!("ops[{i}].id"))?;
    Ok(extlink::DeleteExternalLinkOp { id, index })
}

fn parse_set_external_link(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<extlink::SetExternalLinkOp, Error> {
    deny_keys(obj, i, &["op", "id", "path"])?;
    let id = require_id(obj, i)?;
    let index = extlink::parse_external_link_id(&id, &format!("ops[{i}].id"))?;
    let path = match obj.get("path") {
        Some(Value::String(s)) if !s.is_empty() => s.clone(),
        None => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "path".into(),
                },
                format!("ops[{i}].path"),
            ));
        }
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidExternalLink {
                    field: "path".into(),
                },
                format!("ops[{i}].path"),
            ));
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "path".into(),
                    expected: TypeName::String,
                },
                format!("ops[{i}].path"),
            ));
        }
    };
    Ok(extlink::SetExternalLinkOp { id, index, path })
}

fn parse_set_textbox_size(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetTextBoxSizeOp, Error> {
    deny_keys(obj, i, &["op", "id", "cx", "cy", "to", "toOffX", "toOffY"])?;
    let id = require_id(obj, i)?;
    let (target, kind) = shape::parse_drawing_obj_id(&id, i)?;
    let s = parse_drawing_size(obj, i, "setTextBoxSize")?;
    Ok(SetTextBoxSizeOp {
        id,
        target,
        kind,
        cx: s.cx,
        cy: s.cy,
        to: s.to,
        to_off_x: s.to_off_x,
        to_off_y: s.to_off_y,
    })
}

fn parse_set_textbox_anchor(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetTextBoxAnchorOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op", "id", "anchor", "from", "to", "fromOffX", "fromOffY", "toOffX", "toOffY", "posX",
            "posY", "editAs", "cx", "cy", "fromX", "fromY", "toX", "toY",
        ],
    )?;
    let id = require_id(obj, i)?;
    let (target, kind) = shape::parse_drawing_obj_id(&id, i)?;
    let p = parse_drawing_anchor_patch(obj, i, "setTextBoxAnchor")?;
    Ok(SetTextBoxAnchorOp {
        id,
        target,
        kind,
        anchor: p.anchor,
        from: p.from,
        to: p.to,
        from_off_x: p.from_off_x,
        from_off_y: p.from_off_y,
        to_off_x: p.to_off_x,
        to_off_y: p.to_off_y,
        pos_x: p.pos_x,
        pos_y: p.pos_y,
        edit_as: p.edit_as,
        cx: p.cx,
        cy: p.cy,
        rel: p.rel,
    })
}

fn parse_set_shape_text(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetShapeTextOp, Error> {
    deny_keys(obj, i, &["op", "id", "text"])?;
    let id = require_id(obj, i)?;
    let (target, kind) = shape::parse_shape_text_id(&id, i)?;
    let text = parse_require_text(obj, i, "setShapeText")?;
    Ok(SetShapeTextOp {
        id,
        target,
        kind,
        text,
    })
}

fn parse_set_shape_format(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetShapeFormatOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op",
            "id",
            "sz",
            "color",
            "textFill",
            "textOutline",
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
    let (target, kind) = shape::parse_shape_text_id(&id, i)?;
    let format = crate::drawing_text::parse(obj, &format!("ops[{i}]"), true)?
        .expect("require_one 已保证有补丁");
    Ok(SetShapeFormatOp {
        id,
        target,
        kind,
        format,
    })
}

fn parse_insert_smartart(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertSmartArtOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op", "id", "layout", "points", "cx", "cy", "from", "to", "anchor", "fromOffX",
            "fromOffY", "toOffX", "toOffY",
        ],
    )?;
    let id = require_id(obj, i)?;
    let sheet = shape::parse_insert_sheet_id(&id, i, IdReason::InsertSmartArtNeedsSheet)?;
    let (layout, points) = crate::smartart::parse_layout_and_points(obj, &format!("ops[{i}]"))?;
    let a = parse_insert_anchor(obj, i, "insertSmartArt")?;
    Ok(InsertSmartArtOp {
        sheet,
        layout,
        points,
        cx: a.cx,
        cy: a.cy,
        from: a.from,
        to: a.to,
        anchor: a.anchor,
        from_off_x: a.from_off_x,
        from_off_y: a.from_off_y,
        to_off_x: a.to_off_x,
        to_off_y: a.to_off_y,
    })
}

fn parse_delete_smartart(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteSmartArtOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let target = xlsx_smartart::parse_diagram_id(&id, i)?;
    Ok(DeleteSmartArtOp { id, target })
}

fn parse_set_smartart_text(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetSmartArtTextOp, Error> {
    deny_keys(obj, i, &["op", "id", "text"])?;
    let id = require_id(obj, i)?;
    let target = xlsx_smartart::parse_point_id(&id, i)?;
    let text = parse_require_text(obj, i, "setSmartArtText")?;
    Ok(SetSmartArtTextOp { id, target, text })
}

fn parse_add_smartart_point(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<AddSmartArtPointOp, Error> {
    deny_keys(obj, i, &["op", "id", "text", "role"])?;
    let id = require_id(obj, i)?;
    let parent = xlsx_smartart::parse_add_parent(&id, i)?;
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
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteSmartArtPointOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let target = xlsx_smartart::parse_point_id(&id, i)?;
    Ok(DeleteSmartArtPointOp { id, target })
}

fn parse_set_smartart_size(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetSmartArtSizeOp, Error> {
    deny_keys(obj, i, &["op", "id", "cx", "cy", "to", "toOffX", "toOffY"])?;
    let id = require_id(obj, i)?;
    let target = xlsx_smartart::parse_diagram_id(&id, i)?;
    let s = parse_drawing_size(obj, i, "setSmartArtSize")?;
    Ok(SetSmartArtSizeOp {
        id,
        target,
        cx: s.cx,
        cy: s.cy,
        to: s.to,
        to_off_x: s.to_off_x,
        to_off_y: s.to_off_y,
    })
}

fn parse_set_smartart_anchor(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetSmartArtAnchorOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op", "id", "anchor", "from", "to", "fromOffX", "fromOffY", "toOffX", "toOffY", "posX",
            "posY", "editAs", "cx", "cy",
        ],
    )?;
    let id = require_id(obj, i)?;
    let target = xlsx_smartart::parse_diagram_id(&id, i)?;
    let p = parse_drawing_anchor_patch(obj, i, "setSmartArtAnchor")?;
    Ok(SetSmartArtAnchorOp {
        id,
        target,
        anchor: p.anchor,
        from: p.from,
        to: p.to,
        from_off_x: p.from_off_x,
        from_off_y: p.from_off_y,
        to_off_x: p.to_off_x,
        to_off_y: p.to_off_y,
        pos_x: p.pos_x,
        pos_y: p.pos_y,
        edit_as: p.edit_as,
        cx: p.cx,
        cy: p.cy,
    })
}

fn parse_set_smartart_colors(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetSmartArtColorsOp, Error> {
    deny_keys(obj, i, &["op", "id", "colors"])?;
    let id = require_id(obj, i)?;
    let target = xlsx_smartart::parse_diagram_id(&id, i)?;
    let colors =
        crate::smartart::parse_colors_field(obj.get("colors"), &format!("ops[{i}].colors"))?;
    Ok(SetSmartArtColorsOp { id, target, colors })
}

fn parse_set_smartart_style(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetSmartArtStyleOp, Error> {
    deny_keys(obj, i, &["op", "id", "style"])?;
    let id = require_id(obj, i)?;
    let target = xlsx_smartart::parse_diagram_id(&id, i)?;
    let style = crate::smartart::parse_style_field(obj.get("style"), &format!("ops[{i}].style"))?;
    Ok(SetSmartArtStyleOp { id, target, style })
}

fn parse_set_smartart_layout(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetSmartArtLayoutOp, Error> {
    deny_keys(obj, i, &["op", "id", "layout"])?;
    let id = require_id(obj, i)?;
    let target = xlsx_smartart::parse_diagram_id(&id, i)?;
    let layout =
        crate::smartart::parse_layout_field(obj.get("layout"), &format!("ops[{i}].layout"))?;
    Ok(SetSmartArtLayoutOp { id, target, layout })
}

fn parse_insert_trendline(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertTrendlineOp, Error> {
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
    let (target, index) = chart::parse_series_id(&id, i)?;
    let spec = crate::chart::remain::parse_trendline_write(obj, i, true)?;
    Ok(InsertTrendlineOp {
        id,
        target,
        index,
        spec,
    })
}

fn parse_set_trendline(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetTrendlineOp, Error> {
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
    let (target, index, tl_index) = chart::parse_trendline_id(&id, i)?;
    let spec = crate::chart::remain::parse_trendline_write(obj, i, false)?;
    Ok(SetTrendlineOp {
        id,
        target,
        index,
        tl_index,
        spec,
    })
}

fn parse_delete_trendline(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteTrendlineOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let (target, index, tl_index) = chart::parse_trendline_id(&id, i)?;
    Ok(DeleteTrendlineOp {
        id,
        target,
        index,
        tl_index,
    })
}

fn parse_insert_error_bars(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<InsertErrorBarsOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op", "id", "dir", "barType", "valType", "value", "plus", "minus",
        ],
    )?;
    let id = require_id(obj, i)?;
    let (target, index) = chart::parse_series_id(&id, i)?;
    let spec = crate::chart::remain::parse_error_bars_write(obj, i, true)?;
    Ok(InsertErrorBarsOp {
        id,
        target,
        index,
        spec,
    })
}

fn parse_set_error_bars(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetErrorBarsOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op", "id", "dir", "barType", "valType", "value", "plus", "minus",
        ],
    )?;
    let id = require_id(obj, i)?;
    let (target, index, eb_index) = chart::parse_error_bars_id(&id, i)?;
    let spec = crate::chart::remain::parse_error_bars_write(obj, i, false)?;
    Ok(SetErrorBarsOp {
        id,
        target,
        index,
        eb_index,
        spec,
    })
}

fn parse_delete_error_bars(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<DeleteErrorBarsOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let (target, index, eb_index) = chart::parse_error_bars_id(&id, i)?;
    Ok(DeleteErrorBarsOp {
        id,
        target,
        index,
        eb_index,
    })
}

fn parse_set_series_axis(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetChartSeriesAxisOp, Error> {
    deny_keys(obj, i, &["op", "id", "axis"])?;
    let id = require_id(obj, i)?;
    let (target, index) = chart::parse_series_id(&id, i)?;
    let secondary = crate::chart::remain::parse_series_axis(obj, i)?;
    Ok(SetChartSeriesAxisOp {
        id,
        target,
        index,
        secondary,
    })
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
                op: "insertChart".into(),
                field: "type".into(),
            },
            format!("ops[{i}].type"),
        )),
    }
}

fn parse_optional_combo_series_type(
    v: Option<&Value>,
    i: usize,
    field: &str,
) -> Result<Option<ChartType>, Error> {
    match v {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(ChartType::parse_combo_series(s).ok_or_else(|| {
            Error::with_path(
                ErrorKind::InvalidComboSeriesType,
                format!("ops[{i}].{field}"),
            )
        })?)),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].{field}"),
        )),
    }
}

fn parse_datum_list(v: Option<&Value>, i: usize, field: &str) -> Result<Option<Vec<Value>>, Error> {
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
                        format!("ops[{i}].{field}[{j}]"),
                    ));
                }
            }
            Ok(Some(a.clone()))
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
