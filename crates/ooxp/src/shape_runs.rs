//! 形状 / 文本框可见文字的切分计划。写入在 `drawing_text::set_txbody_runs`。

use serde_json::{Map, Value};

use crate::drawing_text::{self, TextFormat};
use crate::error::{Error, ErrorKind, TypeName};
use crate::text_span;

#[derive(Debug, Clone)]
pub enum ShapeRunSource {
    Text(String),
    Range {
        start: usize,
        end: usize,
    },
    Match {
        text: Option<String>,
        after: Option<String>,
        before: Option<String>,
        nth: usize,
    },
}

#[derive(Debug, Clone)]
pub struct ShapeRunPiece {
    pub source: ShapeRunSource,
    pub format: Option<TextFormat>,
}

pub(crate) struct CharSpan {
    pub start: usize,
    pub end: usize,
    pub piece: Option<usize>,
}

const FORMAT_KEYS: &[&str] = &[
    "sz",
    "color",
    "bold",
    "italic",
    "underline",
    "font",
    "eastAsiaFont",
];

pub fn parse_shape_runs(obj: &Map<String, Value>, i: usize) -> Result<Vec<ShapeRunPiece>, Error> {
    let runs_v = obj.get("runs").ok_or_else(|| {
        Error::with_path(
            ErrorKind::MissingField {
                field: "runs".into(),
            },
            format!("ops[{i}].runs"),
        )
    })?;
    let arr = runs_v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "runs".into(),
                expected: TypeName::Array,
            },
            format!("ops[{i}].runs"),
        )
    })?;
    if arr.is_empty() {
        return Err(Error::with_path(
            ErrorKind::EmptyArray {
                field: "runs".into(),
            },
            format!("ops[{i}].runs"),
        ));
    }
    let mut runs = Vec::new();
    let mut mode: Option<u8> = None;
    for (j, item) in arr.iter().enumerate() {
        let item_path = format!("ops[{i}].runs[{j}]");
        let item = item.as_object().ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: item_path.clone(),
                    expected: TypeName::Object,
                },
                item_path.clone(),
            )
        })?;
        for k in item.keys() {
            if k != "text"
                && k != "match"
                && k != "after"
                && k != "before"
                && k != "nth"
                && k != "start"
                && k != "end"
                && !FORMAT_KEYS.contains(&k.as_str())
            {
                return Err(Error::with_path(
                    ErrorKind::UnknownField { field: k.clone() },
                    format!("{item_path}.{k}"),
                ));
            }
        }
        let has_text = item.contains_key("text");
        let has_match = item.contains_key("match");
        let has_after = item.contains_key("after");
        let has_before = item.contains_key("before");
        let has_nth = item.contains_key("nth");
        let has_range = item.contains_key("start") || item.contains_key("end");
        let has_anchor = has_match || has_after || has_before || has_nth;
        let chosen = [has_text, has_anchor, has_range]
            .into_iter()
            .filter(|on| *on)
            .count();
        if chosen != 1 {
            return Err(Error::with_path(
                ErrorKind::MutuallyExclusive {
                    a: "text".into(),
                    b: "match".into(),
                },
                item_path,
            ));
        }
        let this_mode = if has_text {
            0
        } else if has_anchor {
            1
        } else {
            2
        };
        if let Some(prev) = mode {
            if prev != this_mode {
                return Err(Error::with_path(
                    ErrorKind::MutuallyExclusive {
                        a: "text".into(),
                        b: "match".into(),
                    },
                    item_path,
                ));
            }
        }
        mode = Some(this_mode);
        let source = if has_text {
            ShapeRunSource::Text(req_string(item, &item_path, "text")?)
        } else if has_anchor {
            let matched = if has_match {
                Some(req_nonempty(item, &item_path, "match")?)
            } else {
                None
            };
            let after = opt_nonempty(item, &item_path, "after")?;
            let before = opt_nonempty(item, &item_path, "before")?;
            if matched.is_none() && (after.is_none() || before.is_none()) {
                let field = if after.is_none() { "after" } else { "before" };
                return Err(Error::with_path(
                    ErrorKind::MissingField {
                        field: field.into(),
                    },
                    format!("{item_path}.{field}"),
                ));
            }
            let nth = parse_nth(item, &item_path)?;
            ShapeRunSource::Match {
                text: matched,
                after,
                before,
                nth,
            }
        } else {
            let start = req_usize(item, &item_path, "start")?;
            let end = req_usize(item, &item_path, "end")?;
            if end < start {
                return Err(Error::with_path(
                    ErrorKind::OutOfRange {
                        field: "end".into(),
                        min: start as i64,
                        max: i64::MAX,
                    },
                    format!("{item_path}.end"),
                ));
            }
            ShapeRunSource::Range { start, end }
        };
        let format = drawing_text::parse(item, &item_path, false)?;
        runs.push(ShapeRunPiece { source, format });
    }
    Ok(runs)
}

pub(crate) fn plan(
    visible: &str,
    pieces: &[ShapeRunPiece],
    i: usize,
) -> Result<Vec<CharSpan>, Error> {
    let chars: Vec<char> = visible.chars().collect();
    let Some(first) = pieces.first() else {
        return Ok(Vec::new());
    };
    match &first.source {
        ShapeRunSource::Match { .. } => plan_match(visible, pieces, i),
        ShapeRunSource::Text(_) => plan_text(visible, pieces, i),
        ShapeRunSource::Range { .. } => plan_range(&chars, pieces, i),
    }
}

fn plan_text(visible: &str, pieces: &[ShapeRunPiece], i: usize) -> Result<Vec<CharSpan>, Error> {
    let joined: String = pieces
        .iter()
        .map(|piece| match &piece.source {
            ShapeRunSource::Text(text) => text.as_str(),
            _ => "",
        })
        .collect();
    if joined != visible {
        return Err(Error::with_path(
            ErrorKind::ShapeRunMismatch { text: joined },
            format!("ops[{i}].runs"),
        ));
    }
    let mut cursor = 0usize;
    let mut spans = Vec::new();
    for (j, piece) in pieces.iter().enumerate() {
        let ShapeRunSource::Text(text) = &piece.source else {
            continue;
        };
        let end = cursor + text.chars().count();
        spans.push(CharSpan {
            start: cursor,
            end,
            piece: Some(j),
        });
        cursor = end;
    }
    Ok(spans)
}

fn plan_range(chars: &[char], pieces: &[ShapeRunPiece], i: usize) -> Result<Vec<CharSpan>, Error> {
    let mut cursor = 0usize;
    let mut spans = Vec::new();
    for (j, piece) in pieces.iter().enumerate() {
        let ShapeRunSource::Range { start, end } = piece.source else {
            continue;
        };
        if start != cursor || end > chars.len() {
            return Err(Error::with_path(
                ErrorKind::OutOfRange {
                    field: "start".into(),
                    min: cursor as i64,
                    max: chars.len() as i64,
                },
                format!("ops[{i}].runs[{j}].start"),
            ));
        }
        spans.push(CharSpan {
            start,
            end,
            piece: Some(j),
        });
        cursor = end;
    }
    if cursor != chars.len() {
        return Err(Error::with_path(
            ErrorKind::OutOfRange {
                field: "end".into(),
                min: 0,
                max: chars.len() as i64,
            },
            format!("ops[{i}].runs"),
        ));
    }
    Ok(spans)
}

fn plan_match(visible: &str, pieces: &[ShapeRunPiece], i: usize) -> Result<Vec<CharSpan>, Error> {
    let chars: Vec<char> = visible.chars().collect();
    let mut cursor_chars = 0usize;
    let mut cursor_bytes = 0usize;
    let mut spans = Vec::new();
    for (j, piece) in pieces.iter().enumerate() {
        let ShapeRunSource::Match {
            text: needle,
            after,
            before,
            nth,
        } = &piece.source
        else {
            continue;
        };
        let Some((abs_bytes, end_bytes)) = text_span::locate_span(
            visible,
            cursor_bytes,
            needle.as_deref(),
            after.as_deref(),
            before.as_deref(),
            *nth,
        ) else {
            let (field, text) = match needle {
                Some(text) => ("match", text.clone()),
                None => ("before", before.clone().unwrap_or_default()),
            };
            return Err(Error::with_path(
                ErrorKind::ShapeRunMismatch { text },
                format!("ops[{i}].runs[{j}].{field}"),
            ));
        };
        let gap = visible[cursor_bytes..abs_bytes].chars().count();
        let start = cursor_chars + gap;
        let end = start + visible[abs_bytes..end_bytes].chars().count();
        if gap > 0 {
            spans.push(CharSpan {
                start: cursor_chars,
                end: start,
                piece: None,
            });
        }
        spans.push(CharSpan {
            start,
            end,
            piece: Some(j),
        });
        cursor_chars = end;
        cursor_bytes = visible
            .chars()
            .take(cursor_chars)
            .map(|ch| ch.len_utf8())
            .sum();
    }
    if cursor_chars < chars.len() {
        spans.push(CharSpan {
            start: cursor_chars,
            end: chars.len(),
            piece: None,
        });
    }
    Ok(spans)
}

fn req_string(obj: &Map<String, Value>, path: &str, key: &str) -> Result<String, Error> {
    match obj.get(key) {
        Some(Value::String(s)) => Ok(s.clone()),
        _ => Err(Error::with_path(
            ErrorKind::WrongType {
                field: key.into(),
                expected: TypeName::String,
            },
            format!("{path}.{key}"),
        )),
    }
}

fn req_nonempty(obj: &Map<String, Value>, path: &str, key: &str) -> Result<String, Error> {
    match obj.get(key) {
        Some(Value::String(s)) if !s.is_empty() => Ok(s.clone()),
        Some(Value::String(_)) => Err(Error::with_path(
            ErrorKind::MissingField { field: key.into() },
            format!("{path}.{key}"),
        )),
        _ => Err(Error::with_path(
            ErrorKind::WrongType {
                field: key.into(),
                expected: TypeName::NonEmptyString,
            },
            format!("{path}.{key}"),
        )),
    }
}

fn opt_nonempty(obj: &Map<String, Value>, path: &str, key: &str) -> Result<Option<String>, Error> {
    match obj.get(key) {
        None => Ok(None),
        Some(_) => req_nonempty(obj, path, key).map(Some),
    }
}

fn parse_nth(obj: &Map<String, Value>, path: &str) -> Result<usize, Error> {
    match obj.get("nth") {
        None => Ok(1),
        Some(Value::Number(n)) => match n.as_u64() {
            Some(n) if n >= 1 => Ok(n as usize),
            _ => Err(Error::with_path(
                ErrorKind::OutOfRange {
                    field: "nth".into(),
                    min: 1,
                    max: i64::MAX,
                },
                format!("{path}.nth"),
            )),
        },
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "nth".into(),
                expected: TypeName::Integer,
            },
            format!("{path}.nth"),
        )),
    }
}

fn req_usize(obj: &Map<String, Value>, path: &str, key: &str) -> Result<usize, Error> {
    match obj.get(key) {
        Some(Value::Number(n)) => n.as_u64().map(|n| n as usize).ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: key.into(),
                    expected: TypeName::Integer,
                },
                format!("{path}.{key}"),
            )
        }),
        _ => Err(Error::with_path(
            ErrorKind::MissingField { field: key.into() },
            format!("{path}.{key}"),
        )),
    }
}
