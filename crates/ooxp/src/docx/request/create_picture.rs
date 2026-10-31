use serde_json::Value;

use crate::error::{Error, ErrorKind, TypeName};

use super::super::picture::{self, PicKind, WrapPos};
use super::create_types::*;

pub(crate) fn parse_pictures(v: Option<&Value>, prefix: &str) -> Result<Vec<NewPic>, Error> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    let path = format!("{prefix}.pictures");
    let arr = v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "pictures".into(),
                expected: TypeName::Array,
            },
            path.clone(),
        )
    })?;
    let mut out = Vec::new();
    for (j, p) in arr.iter().enumerate() {
        let item_path = format!("{path}[{j}]");
        let obj = p.as_object().ok_or_else(|| {
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
                "file"
                    | "cx"
                    | "cy"
                    | "kind"
                    | "wrap"
                    | "alignH"
                    | "alignV"
                    | "offsetX"
                    | "offsetY"
                    | "hyperlink"
            ) {
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
        let kind = match obj.get("kind") {
            None => PicKind::Inline,
            Some(Value::String(s)) => picture::parse_pic_kind(s).ok_or_else(|| {
                Error::with_path(ErrorKind::InvalidPicKind, format!("{item_path}.kind"))
            })?,
            Some(_) => {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: "kind".into(),
                        expected: TypeName::String,
                    },
                    format!("{item_path}.kind"),
                ));
            }
        };
        let wrap = match obj.get("wrap") {
            None | Some(Value::Null) => None,
            Some(Value::String(s)) => Some(picture::parse_wrap(s).ok_or_else(|| {
                Error::with_path(ErrorKind::InvalidPicWrap, format!("{item_path}.wrap"))
            })?),
            Some(_) => {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: "wrap".into(),
                        expected: TypeName::String,
                    },
                    format!("{item_path}.wrap"),
                ));
            }
        };
        if kind == PicKind::Inline && wrap.is_some() {
            return Err(Error::with_path(
                ErrorKind::InvalidPicWrap,
                format!("{item_path}.wrap"),
            ));
        }
        if kind == PicKind::Anchor && wrap.is_none() {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "wrap".into(),
                },
                format!("{item_path}.wrap"),
            ));
        }
        let align_h = super::create_util::parse_align_h(obj.get("alignH"), &item_path)?;
        let align_v = super::create_util::parse_align_v(obj.get("alignV"), &item_path)?;
        let offset_x = super::create_util::parse_i64(obj.get("offsetX"), &item_path, "offsetX")?;
        let offset_y = super::create_util::parse_i64(obj.get("offsetY"), &item_path, "offsetY")?;
        if align_h.is_some() && offset_x.is_some() {
            return Err(Error::with_path(
                ErrorKind::MutuallyExclusive {
                    a: "alignH".into(),
                    b: "offsetX".into(),
                },
                item_path.clone(),
            ));
        }
        if align_v.is_some() && offset_y.is_some() {
            return Err(Error::with_path(
                ErrorKind::MutuallyExclusive {
                    a: "alignV".into(),
                    b: "offsetY".into(),
                },
                item_path.clone(),
            ));
        }
        if kind == PicKind::Inline
            && (align_h.is_some() || align_v.is_some() || offset_x.is_some() || offset_y.is_some())
        {
            return Err(Error::with_path(
                ErrorKind::UnknownField {
                    field: "alignH".into(),
                },
                item_path,
            ));
        }
        let hyperlink = super::doc_extra::parse_drawing_hlink(
            obj.get("hyperlink"),
            &format!("{item_path}.hyperlink"),
        )?;
        out.push(NewPic {
            file,
            cx,
            cy,
            kind,
            wrap,
            pos: WrapPos {
                align_h,
                align_v,
                offset_x,
                offset_y,
            },
            doc_pr: 0,
            hyperlink,
        });
    }
    Ok(out)
}
