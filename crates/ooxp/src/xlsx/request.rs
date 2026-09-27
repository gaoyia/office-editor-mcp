//! xlsx 自己的 scope 与 ops 解析。

use serde_json::Value;

use crate::error::{Error, ErrorKind, IdReason, ScopeNeed, SheetNameReason, TypeName};
use crate::json;

use crate::chart::{ChartType, NewSeries};

use super::a1;
use super::chart::{self, SheetChartId};
use super::names;
use super::picture::{self, SheetPicId};

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
}

#[derive(Debug, Clone)]
pub struct AlignPatch {
    pub horizontal: Option<String>,
    pub vertical: Option<String>,
    pub wrap: Option<bool>,
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
}

impl CfType {
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "cellIs" => Self::CellIs,
            "expression" => Self::Expression,
            "colorScale" => Self::ColorScale,
            "dataBar" => Self::DataBar,
            "iconSet" => Self::IconSet,
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
        }
    }

    pub fn is_writable(s: &str) -> bool {
        Self::parse(s).is_some()
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
}

#[derive(Debug, Clone)]
pub struct InsertChartOp {
    pub sheet: String,
    pub chart_type: ChartType,
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
        ],
    )?;
    let pictures = parse_bool_field(obj.get("pictures"), "pictures")?;
    let charts = parse_bool_field(obj.get("charts"), "charts")?;
    let header_footer = parse_bool_field(obj.get("headerFooter"), "headerFooter")?;
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
    if cells.is_none() && !pictures && !charts && !header_footer {
        return Ok(None);
    }
    Ok(Some(InspectScope {
        cells,
        pictures,
        charts,
        header_footer,
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
            | "setShapeText"
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
            | "insertComment"
            | "replyComment"
            | "deleteComment"
            | "setCommentText"
            | "setCommentMeta"
            | "setCommentDone"
            | "setChartWrap"
            | "insertSmartArt"
            | "deleteSmartArt"
            | "setSmartArtText"
            | "addSmartArtPoint"
            | "deleteSmartArtPoint"
            | "setSmartArtSize"
            | "setSmartArtWrap"
            | "setSmartArtColors"
            | "setSmartArtStyle"
            | "setSmartArtLayout"
            | "insertOle"
            | "deleteOle"
            | "insertWatermark"
            | "deleteWatermark"
            | "setProtection" => {
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
        &["style", "color", "left", "right", "top", "bottom"],
    )?;
    Ok(BorderPatch {
        style: parse_opt_border_style(obj, i, prefix, "style")?,
        color: parse_opt_color(obj, i, prefix, "color")?,
        left: parse_opt_border_side(obj, i, prefix, "left")?,
        right: parse_opt_border_side(obj, i, prefix, "right")?,
        top: parse_opt_border_side(obj, i, prefix, "top")?,
        bottom: parse_opt_border_side(obj, i, prefix, "bottom")?,
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
    deny_sub_keys(obj, i, prefix, &["horizontal", "vertical", "wrap"])?;
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
    Ok(AlignPatch {
        horizontal,
        vertical,
        wrap: parse_sub_bool(obj, i, prefix, "wrap")?,
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
            "toOffX", "toOffY",
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

fn parse_set_pic_anchor(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<SetPictureAnchorOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op", "id", "anchor", "from", "to", "fromOffX", "fromOffY", "toOffX", "toOffY", "posX",
            "posY", "editAs", "cx", "cy",
        ],
    )?;
    let id = require_id(obj, i)?;
    let target = picture::parse_pic_id(&id, i)?;
    let anchor = match obj.get("anchor") {
        None => None,
        Some(Value::String(s)) if s == "absolute" => {
            return Err(Error::with_path(
                ErrorKind::PictureAnchorAbsolute,
                format!("ops[{i}].anchor"),
            ));
        }
        Some(Value::String(s)) => Some(picture::AnchorKind::parse_insert(s).ok_or_else(|| {
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
    let from = parse_opt_a1(obj.get("from"), i, "from")?;
    let to = parse_opt_a1(obj.get("to"), i, "to")?;
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
    let op = SetPictureAnchorOp {
        id,
        target,
        anchor,
        from,
        to,
        from_off_x: parse_off_field(obj.get("fromOffX"), i, "fromOffX")?,
        from_off_y: parse_off_field(obj.get("fromOffY"), i, "fromOffY")?,
        to_off_x: parse_off_field(obj.get("toOffX"), i, "toOffX")?,
        to_off_y: parse_off_field(obj.get("toOffY"), i, "toOffY")?,
        pos_x: parse_i64_field(obj.get("posX"), i, "posX")?,
        pos_y: parse_i64_field(obj.get("posY"), i, "posY")?,
        edit_as,
        cx: parse_emu_field(obj.get("cx"), i, "cx")?,
        cy: parse_emu_field(obj.get("cy"), i, "cy")?,
    };
    let has_any = op.anchor.is_some()
        || op.from.is_some()
        || op.to.is_some()
        || op.from_off_x.is_some()
        || op.from_off_y.is_some()
        || op.to_off_x.is_some()
        || op.to_off_y.is_some()
        || op.pos_x.is_some()
        || op.pos_y.is_some()
        || op.edit_as.is_some()
        || op.cx.is_some()
        || op.cy.is_some();
    if !has_any {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setPictureAnchor".into(),
                field: "anchor, from, to, offsets, pos, editAs, or size".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(op)
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

fn parse_i64_field(v: Option<&Value>, i: usize, field: &str) -> Result<Option<i64>, Error> {
    let Some(v) = v else {
        return Ok(None);
    };
    let path = format!("ops[{i}].{field}");
    match v {
        Value::Number(n) => {
            if let Some(i64v) = n.as_i64() {
                return Ok(Some(i64v));
            }
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
            "op", "id", "type", "title", "series", "cx", "cy", "from", "to", "anchor", "fromOffX",
            "fromOffY", "toOffX", "toOffY", "dataFrom",
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
    let title = parse_chart_title_field(obj.get("title"), i)?;
    let series = parse_insert_series(obj.get("series"), i, chart_type, "insertChart")?;
    validate_insert_series(chart_type, &series, i)?;
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
    let title = parse_chart_title_field(obj.get("title"), i)?;
    let series = parse_insert_series(obj.get("series"), i, chart_type, "insertChartSheet")?;
    validate_insert_series(chart_type, &series, i)?;
    let (cx, cy) = parse_chart_size_pair(obj, i, "insertChartSheet")?;
    let data_from = parse_required_a1(obj, i, "dataFrom")?;
    Ok(InsertChartSheetOp {
        name,
        data_sheet,
        chart_type,
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
    check_cf_fields(
        type_,
        operator,
        formulas.as_deref(),
        dxf.as_ref(),
        color_scale.as_ref(),
        data_bar.as_ref(),
        icon_set.as_ref(),
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
    match type_ {
        CfType::CellIs => {
            if color_scale.is_some() {
                return Err(bad("colorScale"));
            }
            if data_bar.is_some() {
                return Err(bad("dataBar"));
            }
            if icon_set.is_some() {
                return Err(bad("iconSet"));
            }
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
            if color_scale.is_some() {
                return Err(bad("colorScale"));
            }
            if data_bar.is_some() {
                return Err(bad("dataBar"));
            }
            if icon_set.is_some() {
                return Err(bad("iconSet"));
            }
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
        }
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
    i: usize,
) -> Result<(), Error> {
    if chart_type.needs_one_series() && series.len() != 1 {
        return Err(Error::with_path(
            ErrorKind::PieNeedsOneSeries,
            format!("ops[{i}].series"),
        ));
    }
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
    deny_keys(obj, i, &["op", "id", "type"])?;
    let id = require_id(obj, i)?;
    let target = chart::parse_chart_id(&id, i)?;
    let chart_type = parse_chart_type_field(obj.get("type"), i, true)?;
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
    deny_keys(obj, i, &["op", "id", "cx", "cy"])?;
    let id = require_id(obj, i)?;
    let target = chart::parse_chart_id(&id, i)?;
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
    let anchor = match obj.get("anchor") {
        None => None,
        Some(Value::String(s)) if s == "absolute" => {
            return Err(Error::with_path(
                ErrorKind::PictureAnchorAbsolute,
                format!("ops[{i}].anchor"),
            ));
        }
        Some(Value::String(s)) => Some(picture::AnchorKind::parse_insert(s).ok_or_else(|| {
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
    let op = SetChartAnchorOp {
        id,
        target,
        anchor,
        from: parse_opt_a1(obj.get("from"), i, "from")?,
        to: parse_opt_a1(obj.get("to"), i, "to")?,
        from_off_x: parse_off_field(obj.get("fromOffX"), i, "fromOffX")?,
        from_off_y: parse_off_field(obj.get("fromOffY"), i, "fromOffY")?,
        to_off_x: parse_off_field(obj.get("toOffX"), i, "toOffX")?,
        to_off_y: parse_off_field(obj.get("toOffY"), i, "toOffY")?,
        pos_x: parse_i64_field(obj.get("posX"), i, "posX")?,
        pos_y: parse_i64_field(obj.get("posY"), i, "posY")?,
        edit_as: match obj.get("editAs") {
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
        },
        cx: parse_emu_field(obj.get("cx"), i, "cx")?,
        cy: parse_emu_field(obj.get("cy"), i, "cy")?,
    };
    let has_any = op.anchor.is_some()
        || op.from.is_some()
        || op.to.is_some()
        || op.from_off_x.is_some()
        || op.from_off_y.is_some()
        || op.to_off_x.is_some()
        || op.to_off_y.is_some()
        || op.pos_x.is_some()
        || op.pos_y.is_some()
        || op.edit_as.is_some()
        || op.cx.is_some()
        || op.cy.is_some();
    if !has_any {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "setChartAnchor".into(),
                field: "anchor, from, to, offsets, pos, editAs, or size".into(),
            },
            format!("ops[{i}]"),
        ));
    }
    Ok(op)
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
    deny_keys(obj, i, &["op", "id", "dir", "barType", "valType", "value"])?;
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
    deny_keys(obj, i, &["op", "id", "dir", "barType", "valType", "value"])?;
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
