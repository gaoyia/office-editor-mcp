//! pptx request：media。

use serde_json::{Map, Value};

use crate::error::{Error, ErrorKind, TypeName};

use super::super::media;

use super::common::*;
use super::types::*;

pub(super) fn parse_insert_media(
    obj: &Map<String, Value>,
    i: usize,
    op: &str,
) -> Result<InsertMediaOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op",
            "id",
            "file",
            "x",
            "y",
            "cx",
            "cy",
            "st",
            "end",
            "bookmarks",
            "tracks",
            "trackDisplayLoc",
        ],
    )?;
    let id = require_id(obj, i)?;
    let (slide, on_notes, master) = media::parse_insert_slide(&id, i)?.parts();
    let file = require_file(obj, i, op)?;
    let x = parse_off_field(obj.get("x"), i, "x")?;
    let y = parse_off_field(obj.get("y"), i, "y")?;
    if x.is_some() != y.is_some() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: op.into(),
                field: "x and y together".into(),
            },
            format!("ops[{i}]"),
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
    let prefix = format!("ops[{i}]");
    let extras = parse_media_clip_fields(obj, &prefix, false)?;
    Ok(InsertMediaOp {
        slide,
        on_notes,
        master,
        file,
        x,
        y,
        cx,
        cy,
        st: extras.st,
        end: extras.end,
        bookmarks: extras.bookmarks,
        tracks: extras.tracks,
        track_display_loc: extras.track_display_loc,
    })
}

pub(super) fn parse_media_clip_fields(
    obj: &Map<String, Value>,
    prefix: &str,
    allow_null_lists: bool,
) -> Result<MediaClipFields, Error> {
    let st = parse_optional_media_time(obj.get("st"), &format!("{prefix}.st"))?;
    let end = parse_optional_media_time(obj.get("end"), &format!("{prefix}.end"))?;
    let bookmarks = parse_media_bookmarks(
        obj.get("bookmarks"),
        &format!("{prefix}.bookmarks"),
        allow_null_lists,
    )?;
    let tracks = parse_media_tracks(
        obj.get("tracks"),
        &format!("{prefix}.tracks"),
        allow_null_lists,
    )?;
    let track_display_loc = parse_track_display_loc(
        obj.get("trackDisplayLoc"),
        &format!("{prefix}.trackDisplayLoc"),
        tracks.as_ref(),
    )?;
    Ok(MediaClipFields {
        st,
        end,
        bookmarks,
        tracks,
        track_display_loc,
    })
}

pub(super) fn parse_optional_media_time(
    v: Option<&Value>,
    path: &str,
) -> Result<Option<f64>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Number(n)) => {
            let x = n
                .as_f64()
                .ok_or_else(|| Error::with_path(ErrorKind::InvalidMediaTime, path.to_string()))?;
            if !x.is_finite() || x < 0.0 {
                return Err(Error::with_path(
                    ErrorKind::InvalidMediaTime,
                    path.to_string(),
                ));
            }
            Ok(Some(x))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::InvalidMediaTime,
            path.to_string(),
        )),
    }
}

pub(super) fn parse_media_bookmarks(
    v: Option<&Value>,
    path: &str,
    allow_null: bool,
) -> Result<Option<Vec<MediaBookmark>>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Null) if allow_null => Ok(Some(Vec::new())),
        Some(Value::Array(arr)) => {
            let mut out = Vec::new();
            let mut names = Vec::new();
            for (j, item) in arr.iter().enumerate() {
                let obj = item.as_object().ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::WrongType {
                            field: "bookmarks".into(),
                            expected: TypeName::Object,
                        },
                        format!("{path}[{j}]"),
                    )
                })?;
                deny_keys_at(obj, &format!("{path}[{j}]"), &["name", "time"])?;
                let name = match obj.get("name") {
                    Some(Value::String(s)) if !s.is_empty() => s.clone(),
                    _ => {
                        return Err(Error::with_path(
                            ErrorKind::InvalidTrack,
                            format!("{path}[{j}].name"),
                        ));
                    }
                };
                if names.iter().any(|n| n == &name) {
                    return Err(Error::with_path(
                        ErrorKind::DuplicateBookmarkName,
                        format!("{path}[{j}].name"),
                    ));
                }
                names.push(name.clone());
                let time =
                    parse_optional_media_time(obj.get("time"), &format!("{path}[{j}].time"))?
                        .ok_or_else(|| {
                            Error::with_path(
                                ErrorKind::MissingField {
                                    field: "time".into(),
                                },
                                format!("{path}[{j}].time"),
                            )
                        })?;
                out.push(MediaBookmark { name, time });
            }
            Ok(Some(out))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "bookmarks".into(),
                expected: TypeName::Array,
            },
            path.to_string(),
        )),
    }
}

pub(super) fn parse_media_tracks(
    v: Option<&Value>,
    path: &str,
    allow_null: bool,
) -> Result<Option<Vec<MediaTrackWrite>>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Null) if allow_null => Ok(Some(Vec::new())),
        Some(Value::Array(arr)) => {
            let mut out = Vec::new();
            for (j, item) in arr.iter().enumerate() {
                let obj = item.as_object().ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::WrongType {
                            field: "tracks".into(),
                            expected: TypeName::Object,
                        },
                        format!("{path}[{j}]"),
                    )
                })?;
                deny_keys_at(
                    obj,
                    &format!("{path}[{j}]"),
                    &["id", "label", "lang", "file"],
                )?;
                let id = match obj.get("id") {
                    None => None,
                    Some(Value::String(s)) if media::is_track_guid(s) => Some(s.clone()),
                    Some(_) => {
                        return Err(Error::with_path(
                            ErrorKind::InvalidTrack,
                            format!("{path}[{j}].id"),
                        ));
                    }
                };
                let label = match obj.get("label") {
                    Some(Value::String(s)) if !s.is_empty() => s.clone(),
                    _ => {
                        return Err(Error::with_path(
                            ErrorKind::InvalidTrack,
                            format!("{path}[{j}].label"),
                        ));
                    }
                };
                let lang = match obj.get("lang") {
                    None => None,
                    Some(Value::String(s)) if !s.is_empty() => Some(s.clone()),
                    Some(_) => {
                        return Err(Error::with_path(
                            ErrorKind::InvalidTrack,
                            format!("{path}[{j}].lang"),
                        ));
                    }
                };
                let file = match obj.get("file") {
                    Some(Value::String(s)) if !s.is_empty() => s.clone(),
                    Some(Value::String(_)) | None => {
                        return Err(Error::with_path(
                            ErrorKind::MissingField {
                                field: "file".into(),
                            },
                            format!("{path}[{j}].file"),
                        ));
                    }
                    Some(_) => {
                        return Err(Error::with_path(
                            ErrorKind::WrongType {
                                field: "file".into(),
                                expected: TypeName::NonEmptyString,
                            },
                            format!("{path}[{j}].file"),
                        ));
                    }
                };
                out.push(MediaTrackWrite {
                    id,
                    label,
                    lang,
                    file,
                });
            }
            Ok(Some(out))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "tracks".into(),
                expected: TypeName::Array,
            },
            path.to_string(),
        )),
    }
}

pub(super) fn parse_track_display_loc(
    v: Option<&Value>,
    path: &str,
    tracks: Option<&Vec<MediaTrackWrite>>,
) -> Result<Option<String>, Error> {
    match v {
        None => Ok(None),
        Some(_) if tracks.map(|t| t.is_empty()).unwrap_or(true) => {
            Err(Error::with_path(ErrorKind::InvalidTrack, path.to_string()))
        }
        Some(Value::String(s)) if s == "media" || s == "slide" => Ok(Some(s.clone())),
        Some(_) => Err(Error::with_path(ErrorKind::InvalidTrack, path.to_string())),
    }
}

pub(super) fn parse_set_media(obj: &Map<String, Value>, i: usize) -> Result<SetMediaOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op",
            "id",
            "st",
            "end",
            "bookmarks",
            "tracks",
            "trackDisplayLoc",
        ],
    )?;
    let id = require_id(obj, i)?;
    let target = media::parse_media_id(&id, &format!("ops[{i}].id"))?;
    let prefix = format!("ops[{i}]");
    let extras = parse_media_clip_fields(obj, &prefix, true)?;
    if extras.st.is_none()
        && extras.end.is_none()
        && extras.bookmarks.is_none()
        && extras.tracks.is_none()
    {
        return Err(Error::with_path(
            ErrorKind::NeedSetMediaField,
            format!("ops[{i}]"),
        ));
    }
    Ok(SetMediaOp {
        id,
        target,
        st: extras.st,
        end: extras.end,
        bookmarks: extras.bookmarks,
        tracks: extras.tracks,
        track_display_loc: extras.track_display_loc,
    })
}

pub(super) fn parse_delete_media(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<DeleteMediaOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_id(obj, i)?;
    let target = media::parse_media_id(&id, &format!("ops[{i}].id"))?;
    Ok(DeleteMediaOp { id, target })
}
