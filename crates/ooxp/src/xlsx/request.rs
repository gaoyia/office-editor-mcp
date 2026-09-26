//! xlsx 自己的 scope 与 ops 解析。

use serde_json::Value;

use crate::error::{Error, ErrorKind, IdReason, ScopeNeed, SheetNameReason, TypeName};
use crate::json;

use crate::chart::{ChartType, NewSeries};

use super::a1;
use super::chart::{self, SheetChartId};
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
pub enum XlsxOp {
    SetCell(SetCellOp),
    SetHeaderFooterText(SetHeaderFooterTextOp),
    ReplacePicture(ReplacePictureOp),
    InsertPicture(InsertPictureOp),
    DeletePicture(DeletePictureOp),
    SetPictureSize(SetPictureSizeOp),
    SetPictureAnchor(SetPictureAnchorOp),
    InsertChart(InsertChartOp),
    InsertChartSheet(InsertChartSheetOp),
    DeleteChartSheet(DeleteChartSheetOp),
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
            "setHeaderFooterText" => XlsxOp::SetHeaderFooterText(parse_set_hf_text(obj, i)?),
            "replacePicture" => XlsxOp::ReplacePicture(parse_replace_pic(obj, i)?),
            "insertPicture" => XlsxOp::InsertPicture(parse_insert_pic(obj, i)?),
            "deletePicture" => XlsxOp::DeletePicture(parse_delete_pic(obj, i)?),
            "setPictureSize" => XlsxOp::SetPictureSize(parse_set_pic_size(obj, i)?),
            "setPictureAnchor" => XlsxOp::SetPictureAnchor(parse_set_pic_anchor(obj, i)?),
            "insertChart" => XlsxOp::InsertChart(parse_insert_chart(obj, i)?),
            "insertChartSheet" => XlsxOp::InsertChartSheet(parse_insert_chart_sheet(obj, i)?),
            "deleteChartSheet" => XlsxOp::DeleteChartSheet(parse_delete_chart_sheet(obj, i)?),
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
            "setParaText"
            | "setShapeText"
            | "setHeaderFooterDistance"
            | "setHeaderFooterLink"
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
            | "setSmartArtLayout" => {
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
    deny_keys(obj, i, &["op", "id", "value", "formula"])?;
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
    Ok(SetCellOp { id, value, formula })
}

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
        sheet: parse_sheet_only_id(&id, i, IdReason::DeleteChartSheetNeedsChartsheet)?,
    })
}

fn parse_sheet_only_id(id: &str, op_index: usize, reason: IdReason) -> Result<String, Error> {
    if id.contains("/chart:")
        || id.contains("/pic:")
        || id.contains("/hf:")
        || id.starts_with("cell:")
    {
        return Err(Error::with_path(
            ErrorKind::InvalidId { reason },
            format!("ops[{op_index}].id"),
        ));
    }
    let Some(name) = id.strip_prefix("sheet:") else {
        return Err(Error::with_path(
            ErrorKind::InvalidId { reason },
            format!("ops[{op_index}].id"),
        ));
    };
    if name.is_empty() || name.contains('/') {
        return Err(Error::with_path(
            ErrorKind::InvalidId { reason },
            format!("ops[{op_index}].id"),
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
