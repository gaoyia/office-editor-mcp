use serde_json::Value;

use crate::error::{Error, ErrorKind, TypeName};

use super::create_types::*;

pub(crate) fn parse_notes(v: Option<&Value>, slide_i: usize) -> Result<Option<NewNotes>, Error> {
    let Some(v) = v else {
        return Ok(None);
    };
    let prefix = format!("presentation.slides[{slide_i}].notes");
    match v {
        Value::String(s) => Ok(Some(NewNotes {
            text: s.clone(),
            anims: Vec::new(),
            maths: Vec::new(),
            model3ds: Vec::new(),
            inks: Vec::new(),
        })),
        Value::Object(obj) => {
            for k in obj.keys() {
                if k != "text" && k != "anims" && k != "maths" && k != "model3ds" && k != "inks" {
                    return Err(Error::with_path(
                        ErrorKind::UnknownField { field: k.clone() },
                        format!("{prefix}.{k}"),
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
                        format!("{prefix}.text"),
                    ));
                }
            };
            let anims =
                super::create_timing::parse_anims_at(obj.get("anims"), &format!("{prefix}.anims"))?;
            let maths = super::create_media::parse_maths(
                obj.get("maths"),
                slide_i,
                format!("{prefix}.maths"),
                true,
            )?;
            let model3ds = super::create_media::parse_model3ds(
                obj.get("model3ds"),
                slide_i,
                format!("{prefix}.model3ds"),
                true,
            )?;
            let inks = super::create_media::parse_inks(
                obj.get("inks"),
                slide_i,
                format!("{prefix}.inks"),
                true,
            )?;
            Ok(Some(NewNotes {
                text,
                anims,
                maths,
                model3ds,
                inks,
            }))
        }
        _ => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "notes".into(),
                expected: TypeName::String,
            },
            prefix,
        )),
    }
}
