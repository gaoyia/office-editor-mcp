//! xlsx 自己的 scope 与 ops 解析。

use serde_json::Value;

use crate::error::{Error, ErrorKind, ScopeNeed, TypeName};
use crate::json;

use super::a1;
use super::picture::{self, SheetPicId};

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
pub struct ReplacePictureOp {
    pub id: String,
    pub target: SheetPicId,
    pub file: String,
}

#[derive(Debug, Clone)]
pub struct InsertPictureOp {
    pub sheet: String,
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
pub enum XlsxOp {
    SetCell(SetCellOp),
    ReplacePicture(ReplacePictureOp),
    InsertPicture(InsertPictureOp),
    DeletePicture(DeletePictureOp),
    SetPictureSize(SetPictureSizeOp),
    SetPictureAnchor(SetPictureAnchorOp),
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
    json::deny_unknown(obj, &["sheet", "range", "maxCells", "pictures"])?;
    let pictures = parse_bool_field(obj.get("pictures"), "pictures")?;
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
    if cells.is_none() && !pictures {
        return Ok(None);
    }
    Ok(Some(InspectScope { cells, pictures }))
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
            "replacePicture" => XlsxOp::ReplacePicture(parse_replace_pic(obj, i)?),
            "insertPicture" => XlsxOp::InsertPicture(parse_insert_pic(obj, i)?),
            "deletePicture" => XlsxOp::DeletePicture(parse_delete_pic(obj, i)?),
            "setPictureSize" => XlsxOp::SetPictureSize(parse_set_pic_size(obj, i)?),
            "setPictureAnchor" => XlsxOp::SetPictureAnchor(parse_set_pic_anchor(obj, i)?),
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
            | "insertChart"
            | "deleteChart"
            | "setChartTitle"
            | "setChartSeries"
            | "addChartSeries"
            | "deleteChartSeries"
            | "setChartType"
            | "setChartAxisTitle"
            | "setChartLegend"
            | "setChartDataLabels"
            | "setChartSize"
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
            "op",
            "id",
            "file",
            "cx",
            "cy",
            "from",
            "to",
            "anchor",
            "fromOffX",
            "fromOffY",
            "toOffX",
            "toOffY",
        ],
    )?;
    let id = require_id(obj, i)?;
    let sheet = picture::parse_insert_sheet_id(&id, i)?;
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
            ErrorKind::UnknownField {
                field: "to".into(),
            },
            format!("ops[{i}].to"),
        ));
    }
    Ok(InsertPictureOp {
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
            "op",
            "id",
            "anchor",
            "from",
            "to",
            "fromOffX",
            "fromOffY",
            "toOffX",
            "toOffY",
            "posX",
            "posY",
            "editAs",
            "cx",
            "cy",
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
