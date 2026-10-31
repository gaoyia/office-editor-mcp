use serde_json::Value;

use crate::error::{Error, ErrorKind, TypeName};

use super::super::smartart::{self};
use super::super::textbox::{DEFAULT_CX, DEFAULT_CY};
use super::create_types::*;

pub(crate) fn parse_boxes(v: Option<&Value>, prefix: &str) -> Result<Vec<NewBox>, Error> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    let path = format!("{prefix}.textBoxes");
    let arr = v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "textBoxes".into(),
                expected: TypeName::Array,
            },
            path.clone(),
        )
    })?;
    let mut out = Vec::new();
    for (j, b) in arr.iter().enumerate() {
        let item_path = format!("{path}[{j}]");
        let obj = b.as_object().ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: item_path.clone(),
                    expected: TypeName::Object,
                },
                item_path.clone(),
            )
        })?;
        for k in obj.keys() {
            if !matches!(
                k.as_str(),
                "text"
                    | "cx"
                    | "cy"
                    | "hyperlink"
                    | "tables"
                    | "hyperlinks"
                    | "bookmarks"
                    | "fields"
                    | "maths"
                    | "contentControls"
                    | "rubies"
            ) {
                return Err(Error::with_path(
                    ErrorKind::UnknownField { field: k.clone() },
                    format!("{item_path}.{k}"),
                ));
            }
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
                    format!("{item_path}.text"),
                ));
            }
        };
        let cx =
            super::create_util::parse_emu(obj.get("cx"), &item_path, "cx")?.unwrap_or(DEFAULT_CX);
        let cy =
            super::create_util::parse_emu(obj.get("cy"), &item_path, "cy")?.unwrap_or(DEFAULT_CY);
        let hyperlink = super::doc_extra::parse_drawing_hlink(
            obj.get("hyperlink"),
            &format!("{item_path}.hyperlink"),
        )?;
        let tables = super::create_table::parse_tables(
            obj.get("tables"),
            &format!("{item_path}.tables"),
            1,
        )?;
        for (t_i, t) in tables.iter().enumerate() {
            if t.after == 0 {
                return Err(Error::with_path(
                    ErrorKind::MinValue {
                        field: "after".into(),
                        min: 1,
                    },
                    format!("{item_path}.tables[{t_i}].after"),
                ));
            }
        }
        let hyperlinks = super::create_link::parse_hyperlinks(obj.get("hyperlinks"), &item_path)?;
        let bookmarks = super::create_link::parse_bookmarks(obj.get("bookmarks"), &item_path)?;
        let fields = super::create_field::parse_fields(obj.get("fields"), &item_path)?;
        let maths = super::create_math::parse_maths(obj.get("maths"), &item_path)?;
        let content_controls =
            super::create_sdt::parse_sdts(obj.get("contentControls"), &item_path)?;
        let rubies = super::doc_extra::parse_rubies(obj.get("rubies"), &item_path)?;
        out.push(NewBox {
            text,
            cx,
            cy,
            doc_pr: 0,
            inner_para: 0,
            hyperlink,
            tables,
            hyperlinks,
            bookmarks,
            fields,
            maths,
            content_controls,
            rubies,
        });
    }
    Ok(out)
}

pub(crate) fn parse_smartarts(
    v: Option<&Value>,
    prefix: &str,
) -> Result<Vec<CreateSmartArt>, Error> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    let path = format!("{prefix}.smartArts");
    let arr = v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "smartArts".into(),
                expected: TypeName::Array,
            },
            path.clone(),
        )
    })?;
    let mut out = Vec::new();
    for (j, item) in arr.iter().enumerate() {
        let item_path = format!("{path}[{j}]");
        let obj = item.as_object().ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: item_path.clone(),
                    expected: TypeName::Object,
                },
                item_path.clone(),
            )
        })?;
        for k in obj.keys() {
            if !matches!(
                k.as_str(),
                "layout" | "points" | "kind" | "wrap" | "cx" | "cy" | "hyperlink"
            ) {
                return Err(Error::with_path(
                    ErrorKind::UnknownField { field: k.clone() },
                    format!("{item_path}.{k}"),
                ));
            }
        }
        out.push(CreateSmartArt {
            spec: smartart::parse_spec(obj, &item_path)?,
            hyperlink: super::doc_extra::parse_drawing_hlink(
                obj.get("hyperlink"),
                &format!("{item_path}.hyperlink"),
            )?,
        });
    }
    Ok(out)
}

pub(crate) fn parse_ole_objects(v: Option<&Value>, prefix: &str) -> Result<Vec<String>, Error> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    let path = format!("{prefix}.oleObjects");
    let arr = v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "oleObjects".into(),
                expected: TypeName::Array,
            },
            path.clone(),
        )
    })?;
    let mut out = Vec::new();
    for (j, item) in arr.iter().enumerate() {
        let item_path = format!("{path}[{j}]");
        let obj = item.as_object().ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: item_path.clone(),
                    expected: TypeName::Object,
                },
                item_path.clone(),
            )
        })?;
        for k in obj.keys() {
            if k.as_str() != "file" {
                return Err(Error::with_path(
                    ErrorKind::UnknownField { field: k.clone() },
                    format!("{item_path}.{k}"),
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
                    format!("{item_path}.file"),
                ));
            }
            Some(_) => {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: "file".into(),
                        expected: TypeName::NonEmptyString,
                    },
                    format!("{item_path}.file"),
                ));
            }
        };
        out.push(file);
    }
    Ok(out)
}

pub(crate) fn parse_ink_canvases(
    v: Option<&Value>,
    prefix: &str,
) -> Result<Vec<NewInkCanvas>, Error> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    let path = format!("{prefix}.inkCanvases");
    let arr = v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "inkCanvases".into(),
                expected: TypeName::Array,
            },
            path.clone(),
        )
    })?;
    let mut out = Vec::new();
    for (j, item) in arr.iter().enumerate() {
        let item_path = format!("{path}[{j}]");
        let obj = item.as_object().ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: item_path.clone(),
                    expected: TypeName::Object,
                },
                item_path.clone(),
            )
        })?;
        for k in obj.keys() {
            if !matches!(k.as_str(), "file" | "cx" | "cy" | "hyperlink") {
                return Err(Error::with_path(
                    ErrorKind::UnknownField { field: k.clone() },
                    format!("{item_path}.{k}"),
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
                    format!("{item_path}.file"),
                ));
            }
            Some(_) => {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: "file".into(),
                        expected: TypeName::NonEmptyString,
                    },
                    format!("{item_path}.file"),
                ));
            }
        };
        let cx = super::create_util::parse_emu(obj.get("cx"), &item_path, "cx")?;
        let cy = super::create_util::parse_emu(obj.get("cy"), &item_path, "cy")?;
        if cx.is_some() != cy.is_some() {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "cx and cy".into(),
                },
                item_path.clone(),
            ));
        }
        out.push(NewInkCanvas {
            file,
            cx,
            cy,
            hyperlink: super::doc_extra::parse_drawing_hlink(
                obj.get("hyperlink"),
                &format!("{item_path}.hyperlink"),
            )?,
        });
    }
    Ok(out)
}

pub(crate) fn parse_model3ds(v: Option<&Value>, prefix: &str) -> Result<Vec<NewModel3d>, Error> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    let path = format!("{prefix}.model3ds");
    let arr = v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "model3ds".into(),
                expected: TypeName::Array,
            },
            path.clone(),
        )
    })?;
    let mut out = Vec::new();
    for (j, item) in arr.iter().enumerate() {
        let item_path = format!("{path}[{j}]");
        let obj = item.as_object().ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: item_path.clone(),
                    expected: TypeName::Object,
                },
                item_path.clone(),
            )
        })?;
        for k in obj.keys() {
            if !matches!(k.as_str(), "file" | "cx" | "cy" | "hyperlink") {
                return Err(Error::with_path(
                    ErrorKind::UnknownField { field: k.clone() },
                    format!("{item_path}.{k}"),
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
                    format!("{item_path}.file"),
                ));
            }
            Some(_) => {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: "file".into(),
                        expected: TypeName::NonEmptyString,
                    },
                    format!("{item_path}.file"),
                ));
            }
        };
        let cx = super::create_util::parse_emu(obj.get("cx"), &item_path, "cx")?;
        let cy = super::create_util::parse_emu(obj.get("cy"), &item_path, "cy")?;
        if cx.is_some() != cy.is_some() {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "cx and cy".into(),
                },
                item_path.clone(),
            ));
        }
        out.push(NewModel3d {
            file,
            cx,
            cy,
            hyperlink: super::doc_extra::parse_drawing_hlink(
                obj.get("hyperlink"),
                &format!("{item_path}.hyperlink"),
            )?,
        });
    }
    Ok(out)
}
