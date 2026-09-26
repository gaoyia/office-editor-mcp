//! pptx 自己的 scope 与 ops 解析。

use serde_json::Value;

use crate::error::{Error, ErrorKind, ScopeNeed, TypeName};
use crate::json;

#[derive(Debug, Clone)]
pub struct InspectScope {
    pub slide: u32,
    pub max_shapes: u32,
}

#[derive(Debug, Clone)]
pub struct SetShapeTextOp {
    pub id: String,
    pub text: String,
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
    json::deny_unknown(obj, &["slide", "maxShapes"])?;
    let slide = obj.get("slide");
    let max = obj.get("maxShapes");
    match (slide, max) {
        (None, None) => Ok(None),
        (None, Some(_)) => Err(Error::with_path(
            ErrorKind::ScopeRequired {
                need: ScopeNeed::Slide,
            },
            "scope.slide",
        )),
        (Some(s), m) => {
            let slide = s.as_u64().ok_or_else(|| {
                Error::with_path(
                    ErrorKind::WrongType {
                        field: "slide".into(),
                        expected: TypeName::Integer,
                    },
                    "scope.slide",
                )
            })?;
            if slide < 1 {
                return Err(Error::with_path(
                    ErrorKind::MinValue {
                        field: "slide".into(),
                        min: 1,
                    },
                    "scope.slide",
                ));
            }
            let max = match m {
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
                slide: slide as u32,
                max_shapes: max as u32,
            }))
        }
    }
}

pub fn parse_ops(ops: &[Value]) -> Result<Vec<SetShapeTextOp>, Error> {
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
            || op_name == "setParaText"
            || op_name == "setHeaderFooterDistance"
            || op_name == "setHeaderFooterLink"
            || op_name == "setHeaderFooterText"
            || op_name == "replacePicture"
            || op_name == "insertPicture"
            || op_name == "deletePicture"
            || op_name == "setPictureSize"
            || op_name == "setPictureWrap"
            || op_name == "setPictureAnchor"
            || op_name == "insertTextBox"
            || op_name == "deleteTextBox"
            || op_name == "setTextBoxSize"
            || op_name == "setTextBoxWrap"
            || op_name == "setTextBoxFill"
            || op_name == "setTextBoxLine"
            || op_name == "setTextBoxDirection"
            || op_name == "insertPara"
            || op_name == "deletePara"
            || op_name == "setTextBoxLink"
            || op_name == "insertFootnote"
            || op_name == "insertEndnote"
            || op_name == "deleteFootnote"
            || op_name == "deleteEndnote"
            || op_name == "setNoteNumbering"
            || op_name == "insertComment"
            || op_name == "replyComment"
            || op_name == "deleteComment"
            || op_name == "setCommentText"
            || op_name == "setCommentMeta"
            || op_name == "setCommentDone"
            || op_name == "insertChart"
            || op_name == "deleteChart"
            || op_name == "setChartTitle"
            || op_name == "setChartSeries"
            || op_name == "addChartSeries"
            || op_name == "deleteChartSeries"
            || op_name == "setChartType"
            || op_name == "setChartAxisTitle"
            || op_name == "setChartLegend"
            || op_name == "setChartDataLabels"
            || op_name == "setChartSize"
            || op_name == "setChartWrap"
            || op_name == "setChartAnchor"
            || op_name == "insertSmartArt"
            || op_name == "deleteSmartArt"
            || op_name == "setSmartArtText"
            || op_name == "addSmartArtPoint"
            || op_name == "deleteSmartArtPoint"
            || op_name == "setSmartArtSize"
            || op_name == "setSmartArtWrap"
            || op_name == "setSmartArtColors"
            || op_name == "setSmartArtStyle"
            || op_name == "setSmartArtLayout"
        {
            return Err(Error::with_path(
                ErrorKind::OpNotForFormat {
                    format: "pptx".into(),
                    op: op_name.to_string(),
                },
                format!("ops[{i}].op"),
            ));
        }
        if op_name != "setShapeText" {
            return Err(Error::with_path(
                ErrorKind::UnsupportedOp {
                    op: op_name.to_string(),
                },
                format!("ops[{i}].op"),
            ));
        }
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
                        op: "setShapeText".into(),
                        field: "text".into(),
                    },
                    format!("ops[{i}].text"),
                ));
            }
        };
        out.push(SetShapeTextOp {
            id: id.to_string(),
            text,
        });
    }
    Ok(out)
}
