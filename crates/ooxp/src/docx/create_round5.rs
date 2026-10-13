//! create 第五轮：段边框底纹制表位、runs、组合。复用已有 apply。

use serde_json::Value;

use crate::error::{Error, ErrorKind, TypeName};

use super::format::{self, EastAsianLayout, ParaBordersPatch, TabStop};
use super::group;
use super::page_bg;
use super::para::ParaTarget;
use super::request::{GroupShapesOp, SetParaFormatOp, SetRunFormatOp};

pub struct ParaRemain {
    pub borders: Option<Option<ParaBordersPatch>>,
    pub shd: Option<Option<String>>,
    pub tabs: Option<Option<Vec<TabStop>>>,
}

impl ParaRemain {
    pub fn empty() -> Self {
        Self {
            borders: None,
            shd: None,
            tabs: None,
        }
    }

    pub fn has_follow_ups(&self) -> bool {
        self.borders.is_some() || self.shd.is_some() || self.tabs.is_some()
    }
}

pub struct RunFormatOwned {
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub underline: Option<Option<String>>,
    pub sz: Option<u32>,
    pub color: Option<String>,
    pub font: Option<String>,
    pub east_asia_font: Option<String>,
    pub em: Option<Option<String>>,
    pub east_asian_layout: Option<Option<EastAsianLayout>>,
    pub highlight: Option<Option<String>>,
    pub strike: Option<Option<bool>>,
    pub dstrike: Option<Option<bool>>,
    pub vert_align: Option<Option<String>>,
    pub spacing: Option<Option<i32>>,
    pub kern: Option<Option<u32>>,
}

pub struct RunSpec {
    pub text: String,
    pub format: Option<RunFormatOwned>,
}

pub struct GroupSpec {
    pub ids: Vec<String>,
}

pub fn parse_para_remain(
    obj: &serde_json::Map<String, Value>,
    path: &str,
) -> Result<ParaRemain, Error> {
    Ok(ParaRemain {
        borders: parse_borders(obj.get("borders"), path)?,
        shd: parse_shd(obj.get("shd"), path)?,
        tabs: parse_tabs(obj.get("tabs"), path)?,
    })
}

pub fn parse_runs(v: Option<&Value>, path: &str) -> Result<Option<Vec<RunSpec>>, Error> {
    let Some(v) = v else {
        return Ok(None);
    };
    let run_path = format!("{path}.runs");
    let arr = v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "runs".into(),
                expected: TypeName::Array,
            },
            run_path.clone(),
        )
    })?;
    if arr.is_empty() {
        return Err(Error::with_path(
            ErrorKind::EmptyArray {
                field: "runs".into(),
            },
            run_path,
        ));
    }
    let mut out = Vec::new();
    for (j, item) in arr.iter().enumerate() {
        let item_path = format!("{run_path}[{j}]");
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
            if !RUN_KEYS.contains(&k.as_str()) {
                return Err(Error::with_path(
                    ErrorKind::UnknownField { field: k.clone() },
                    format!("{item_path}.{k}"),
                ));
            }
        }
        let text = match obj.get("text") {
            Some(Value::String(s)) => s.clone(),
            None => {
                return Err(Error::with_path(
                    ErrorKind::MissingField {
                        field: "text".into(),
                    },
                    format!("{item_path}.text"),
                ));
            }
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
        let format = parse_run_format(obj, &item_path)?;
        out.push(RunSpec { text, format });
    }
    Ok(Some(out))
}

pub fn parse_groups(v: Option<&Value>, path: &str) -> Result<Vec<GroupSpec>, Error> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    let arr = v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "groups".into(),
                expected: TypeName::Array,
            },
            path.to_string(),
        )
    })?;
    let mut out = Vec::new();
    for (i, item) in arr.iter().enumerate() {
        let item_path = format!("{path}[{i}]");
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
            if k != "ids" {
                return Err(Error::with_path(
                    ErrorKind::UnknownField { field: k.clone() },
                    format!("{item_path}.{k}"),
                ));
            }
        }
        let ids_v = obj.get("ids").ok_or_else(|| {
            Error::with_path(
                ErrorKind::MissingField {
                    field: "ids".into(),
                },
                format!("{item_path}.ids"),
            )
        })?;
        let ids_arr = ids_v.as_array().ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: "ids".into(),
                    expected: TypeName::Array,
                },
                format!("{item_path}.ids"),
            )
        })?;
        if ids_arr.len() < 2 {
            return Err(Error::with_path(
                ErrorKind::GroupNeedsTwo,
                format!("{item_path}.ids"),
            ));
        }
        let mut ids = Vec::new();
        for (k, v) in ids_arr.iter().enumerate() {
            let s = v.as_str().ok_or_else(|| {
                Error::with_path(
                    ErrorKind::WrongType {
                        field: format!("ids[{k}]"),
                        expected: TypeName::String,
                    },
                    format!("{item_path}.ids[{k}]"),
                )
            })?;
            if s.is_empty() {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: format!("ids[{k}]"),
                        expected: TypeName::NonEmptyString,
                    },
                    format!("{item_path}.ids[{k}]"),
                ));
            }
            ids.push(s.to_string());
        }
        out.push(GroupSpec { ids });
    }
    Ok(out)
}

pub fn to_set_para_format(
    id: String,
    para: ParaTarget,
    remain: &ParaRemain,
) -> Option<SetParaFormatOp> {
    if !remain.has_follow_ups() {
        return None;
    }
    Some(SetParaFormatOp {
        id,
        para,
        align: None,
        left: None,
        right: None,
        first_line: None,
        hanging: None,
        before: None,
        after: None,
        line: None,
        line_rule: None,
        kinsoku: None,
        overflow_punct: None,
        top_line_punct: None,
        auto_space_de: None,
        auto_space_dn: None,
        borders: remain.borders.clone(),
        shd: remain.shd.clone(),
        tabs: remain.tabs.clone(),
    })
}

pub fn to_set_run_format(
    id: String,
    para: ParaTarget,
    run: usize,
    fmt: &RunFormatOwned,
) -> SetRunFormatOp {
    SetRunFormatOp {
        id,
        para,
        run,
        bold: fmt.bold,
        italic: fmt.italic,
        underline: fmt.underline.clone(),
        sz: fmt.sz,
        color: fmt.color.clone(),
        font: fmt.font.clone(),
        east_asia_font: fmt.east_asia_font.clone(),
        em: fmt.em.clone(),
        east_asian_layout: fmt.east_asian_layout.clone(),
        highlight: fmt.highlight.clone(),
        strike: fmt.strike,
        dstrike: fmt.dstrike,
        vert_align: fmt.vert_align.clone(),
        spacing: fmt.spacing,
        kern: fmt.kern,
    }
}

pub fn to_group_shapes(
    host_id: &str,
    spec: &GroupSpec,
    path: &str,
) -> Result<GroupShapesOp, Error> {
    let host = group::parse_group_host_at(host_id, path)?;
    let mut members = Vec::new();
    for (k, mid) in spec.ids.iter().enumerate() {
        members.push(group::parse_member_id_at(mid, &format!("{path}.ids[{k}]"))?);
    }
    Ok(GroupShapesOp {
        id: host_id.to_string(),
        host,
        members,
    })
}

const RUN_KEYS: &[&str] = &[
    "text",
    "bold",
    "italic",
    "underline",
    "sz",
    "color",
    "font",
    "eastAsiaFont",
    "em",
    "eastAsianLayout",
    "highlight",
    "strike",
    "dstrike",
    "vertAlign",
    "spacing",
    "kern",
];

fn parse_run_format(
    obj: &serde_json::Map<String, Value>,
    path: &str,
) -> Result<Option<RunFormatOwned>, Error> {
    let bold = parse_opt_bool(obj, path, "bold")?;
    let italic = parse_opt_bool(obj, path, "italic")?;
    let underline = parse_opt_null_enum(
        obj,
        path,
        "underline",
        format::UNDERLINE_SET,
        ErrorKind::InvalidUnderline,
    )?;
    let sz = parse_opt_u32_range(obj, path, "sz", 2, 3276)?;
    let color = match obj.get("color") {
        None => None,
        Some(Value::String(s)) => {
            if !is_rgb_hex(s) {
                return Err(Error::with_path(
                    ErrorKind::InvalidStyleColor,
                    format!("{path}.color"),
                ));
            }
            Some(s.clone())
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "color".into(),
                    expected: TypeName::String,
                },
                format!("{path}.color"),
            ));
        }
    };
    let font = parse_nonempty_opt_string(obj, path, "font")?;
    let east_asia_font = parse_nonempty_opt_string(obj, path, "eastAsiaFont")?;
    let em = parse_opt_null_enum(obj, path, "em", format::EM_SET, ErrorKind::InvalidEm)?;
    let east_asian_layout = parse_east_asian_layout(obj, path)?;
    let highlight = parse_opt_null_enum(
        obj,
        path,
        "highlight",
        format::HIGHLIGHT_SET,
        ErrorKind::InvalidHighlight,
    )?;
    let strike = parse_opt_null_bool(obj, path, "strike")?;
    let dstrike = parse_opt_null_bool(obj, path, "dstrike")?;
    let vert_align = parse_opt_null_enum(
        obj,
        path,
        "vertAlign",
        format::VERT_ALIGN_SET,
        ErrorKind::InvalidVertAlign,
    )?;
    let spacing = parse_opt_null_i32(obj, path, "spacing")?;
    let kern = parse_opt_null_u32_field(obj, path, "kern")?;
    if bold.is_none()
        && italic.is_none()
        && underline.is_none()
        && sz.is_none()
        && color.is_none()
        && font.is_none()
        && east_asia_font.is_none()
        && em.is_none()
        && east_asian_layout.is_none()
        && highlight.is_none()
        && strike.is_none()
        && dstrike.is_none()
        && vert_align.is_none()
        && spacing.is_none()
        && kern.is_none()
    {
        return Ok(None);
    }
    Ok(Some(RunFormatOwned {
        bold,
        italic,
        underline,
        sz,
        color,
        font,
        east_asia_font,
        em,
        east_asian_layout,
        highlight,
        strike,
        dstrike,
        vert_align,
        spacing,
        kern,
    }))
}

fn parse_borders(v: Option<&Value>, path: &str) -> Result<Option<Option<ParaBordersPatch>>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(None)),
        Some(Value::Object(map)) => {
            for k in map.keys() {
                if !["top", "left", "bottom", "right"].contains(&k.as_str()) {
                    return Err(Error::with_path(
                        ErrorKind::UnknownField { field: k.clone() },
                        format!("{path}.borders.{k}"),
                    ));
                }
            }
            let top = parse_border_side(map.get("top"), &format!("{path}.borders.top"))?;
            let left = parse_border_side(map.get("left"), &format!("{path}.borders.left"))?;
            let bottom = parse_border_side(map.get("bottom"), &format!("{path}.borders.bottom"))?;
            let right = parse_border_side(map.get("right"), &format!("{path}.borders.right"))?;
            if top.is_none() && left.is_none() && bottom.is_none() && right.is_none() {
                return Err(Error::with_path(
                    ErrorKind::NeedParaBordersField,
                    format!("{path}.borders"),
                ));
            }
            Ok(Some(Some(ParaBordersPatch {
                top,
                left,
                bottom,
                right,
            })))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "borders".into(),
                expected: TypeName::Object,
            },
            format!("{path}.borders"),
        )),
    }
}

fn parse_border_side(
    v: Option<&Value>,
    path: &str,
) -> Result<Option<Option<page_bg::BorderSide>>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(None)),
        Some(Value::Object(map)) => {
            for k in map.keys() {
                if !["val", "sz", "space", "color"].contains(&k.as_str()) {
                    return Err(Error::with_path(
                        ErrorKind::UnknownField { field: k.clone() },
                        format!("{path}.{k}"),
                    ));
                }
            }
            let val = match map.get("val") {
                Some(Value::String(s)) => page_bg::parse_border_val(s).ok_or_else(|| {
                    Error::with_path(ErrorKind::InvalidPageBorderVal, format!("{path}.val"))
                })?,
                Some(_) => {
                    return Err(Error::with_path(
                        ErrorKind::WrongType {
                            field: "val".into(),
                            expected: TypeName::String,
                        },
                        format!("{path}.val"),
                    ));
                }
                None => {
                    return Err(Error::with_path(
                        ErrorKind::OpRequiresField {
                            op: "setParaFormat".into(),
                            field: "val".into(),
                        },
                        format!("{path}.val"),
                    ));
                }
            };
            let sz = parse_opt_u32(map.get("sz"), &format!("{path}.sz"))?;
            let space = parse_opt_u32(map.get("space"), &format!("{path}.space"))?;
            let color = match map.get("color") {
                None => None,
                Some(Value::String(s)) => Some(page_bg::parse_side_color(s).ok_or_else(|| {
                    Error::with_path(ErrorKind::InvalidPageBorderColor, format!("{path}.color"))
                })?),
                Some(_) => {
                    return Err(Error::with_path(
                        ErrorKind::WrongType {
                            field: "color".into(),
                            expected: TypeName::String,
                        },
                        format!("{path}.color"),
                    ));
                }
            };
            Ok(Some(Some(page_bg::BorderSide {
                val: val.to_string(),
                sz,
                space,
                color,
            })))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "border".into(),
                expected: TypeName::Object,
            },
            path.to_string(),
        )),
    }
}

fn parse_shd(v: Option<&Value>, path: &str) -> Result<Option<Option<String>>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(None)),
        Some(Value::Object(map)) => {
            for k in map.keys() {
                if k != "fill" {
                    return Err(Error::with_path(
                        ErrorKind::UnknownField { field: k.clone() },
                        format!("{path}.shd.{k}"),
                    ));
                }
            }
            let fill = match map.get("fill") {
                Some(Value::String(s)) if is_rgb_hex(s) => s.to_ascii_uppercase(),
                Some(Value::String(_)) => {
                    return Err(Error::with_path(
                        ErrorKind::InvalidStyleColor,
                        format!("{path}.shd.fill"),
                    ));
                }
                Some(_) => {
                    return Err(Error::with_path(
                        ErrorKind::WrongType {
                            field: "fill".into(),
                            expected: TypeName::String,
                        },
                        format!("{path}.shd.fill"),
                    ));
                }
                None => {
                    return Err(Error::with_path(
                        ErrorKind::OpRequiresField {
                            op: "setParaFormat".into(),
                            field: "shd.fill".into(),
                        },
                        format!("{path}.shd"),
                    ));
                }
            };
            Ok(Some(Some(fill)))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "shd".into(),
                expected: TypeName::Object,
            },
            format!("{path}.shd"),
        )),
    }
}

fn parse_tabs(v: Option<&Value>, path: &str) -> Result<Option<Option<Vec<TabStop>>>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(None)),
        Some(Value::Array(arr)) => {
            if arr.is_empty() {
                return Ok(Some(None));
            }
            let mut out = Vec::new();
            for (j, item) in arr.iter().enumerate() {
                let item_path = format!("{path}.tabs[{j}]");
                let Value::Object(map) = item else {
                    return Err(Error::with_path(
                        ErrorKind::WrongType {
                            field: "tabs".into(),
                            expected: TypeName::Object,
                        },
                        item_path,
                    ));
                };
                for k in map.keys() {
                    if !["val", "pos"].contains(&k.as_str()) {
                        return Err(Error::with_path(
                            ErrorKind::UnknownField { field: k.clone() },
                            format!("{item_path}.{k}"),
                        ));
                    }
                }
                let val = match map.get("val") {
                    Some(Value::String(s)) if format::TAB_VAL_SET.contains(&s.as_str()) => {
                        s.clone()
                    }
                    Some(Value::String(_)) => {
                        return Err(Error::with_path(
                            ErrorKind::InvalidTabVal,
                            format!("{item_path}.val"),
                        ));
                    }
                    Some(_) => {
                        return Err(Error::with_path(
                            ErrorKind::WrongType {
                                field: "val".into(),
                                expected: TypeName::String,
                            },
                            format!("{item_path}.val"),
                        ));
                    }
                    None => {
                        return Err(Error::with_path(
                            ErrorKind::OpRequiresField {
                                op: "setParaFormat".into(),
                                field: "tabs.val".into(),
                            },
                            item_path,
                        ));
                    }
                };
                let pos = match map.get("pos") {
                    Some(Value::Number(n)) => {
                        let Some(v) = n.as_u64() else {
                            return Err(Error::with_path(
                                ErrorKind::WrongType {
                                    field: "pos".into(),
                                    expected: TypeName::Integer,
                                },
                                format!("{item_path}.pos"),
                            ));
                        };
                        if v > u32::MAX as u64 {
                            return Err(Error::with_path(
                                ErrorKind::WrongType {
                                    field: "pos".into(),
                                    expected: TypeName::Integer,
                                },
                                format!("{item_path}.pos"),
                            ));
                        }
                        v as u32
                    }
                    Some(_) => {
                        return Err(Error::with_path(
                            ErrorKind::WrongType {
                                field: "pos".into(),
                                expected: TypeName::Integer,
                            },
                            format!("{item_path}.pos"),
                        ));
                    }
                    None => {
                        return Err(Error::with_path(
                            ErrorKind::OpRequiresField {
                                op: "setParaFormat".into(),
                                field: "tabs.pos".into(),
                            },
                            item_path,
                        ));
                    }
                };
                out.push(TabStop { val, pos });
            }
            Ok(Some(Some(out)))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "tabs".into(),
                expected: TypeName::Array,
            },
            format!("{path}.tabs"),
        )),
    }
}

fn parse_east_asian_layout(
    obj: &serde_json::Map<String, Value>,
    path: &str,
) -> Result<Option<Option<EastAsianLayout>>, Error> {
    match obj.get("eastAsianLayout") {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(None)),
        Some(Value::Object(inner)) => {
            for k in inner.keys() {
                if !matches!(
                    k.as_str(),
                    "id" | "combine" | "combineBrackets" | "vert" | "vertCompress"
                ) {
                    return Err(Error::with_path(
                        ErrorKind::UnknownField { field: k.clone() },
                        format!("{path}.eastAsianLayout.{k}"),
                    ));
                }
            }
            let id = match inner.get("id") {
                None | Some(Value::Null) => None,
                Some(Value::Number(n)) => Some(
                    n.as_i64()
                        .or_else(|| n.as_u64().and_then(|u| i64::try_from(u).ok()))
                        .ok_or_else(|| {
                            Error::with_path(
                                ErrorKind::WrongType {
                                    field: "id".into(),
                                    expected: TypeName::Integer,
                                },
                                format!("{path}.eastAsianLayout.id"),
                            )
                        })?,
                ),
                Some(_) => {
                    return Err(Error::with_path(
                        ErrorKind::WrongType {
                            field: "id".into(),
                            expected: TypeName::Integer,
                        },
                        format!("{path}.eastAsianLayout.id"),
                    ));
                }
            };
            let combine = parse_inner_opt_bool(inner, path, "combine")?;
            let vert = parse_inner_opt_bool(inner, path, "vert")?;
            let vert_compress = parse_inner_opt_bool(inner, path, "vertCompress")?;
            let combine_brackets = match inner.get("combineBrackets") {
                None => None,
                Some(Value::Null) => None,
                Some(Value::String(s)) => {
                    if !format::COMBINE_BRACKETS_SET.contains(&s.as_str()) {
                        return Err(Error::with_path(
                            ErrorKind::InvalidCombineBrackets,
                            format!("{path}.eastAsianLayout.combineBrackets"),
                        ));
                    }
                    Some(s.clone())
                }
                Some(_) => {
                    return Err(Error::with_path(
                        ErrorKind::WrongType {
                            field: "combineBrackets".into(),
                            expected: TypeName::String,
                        },
                        format!("{path}.eastAsianLayout.combineBrackets"),
                    ));
                }
            };
            if combine.is_none() && vert.is_none() {
                return Err(Error::with_path(
                    ErrorKind::NeedEastAsianLayoutField,
                    format!("{path}.eastAsianLayout"),
                ));
            }
            if combine_brackets.is_some() && combine != Some(true) {
                return Err(Error::with_path(
                    ErrorKind::InvalidCombineBrackets,
                    format!("{path}.eastAsianLayout.combineBrackets"),
                ));
            }
            Ok(Some(Some(EastAsianLayout {
                id,
                combine,
                combine_brackets,
                vert,
                vert_compress,
            })))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "eastAsianLayout".into(),
                expected: TypeName::Object,
            },
            format!("{path}.eastAsianLayout"),
        )),
    }
}

fn parse_inner_opt_bool(
    obj: &serde_json::Map<String, Value>,
    path: &str,
    field: &str,
) -> Result<Option<bool>, Error> {
    match obj.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Bool(b)) => Ok(Some(*b)),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Boolean,
            },
            format!("{path}.eastAsianLayout.{field}"),
        )),
    }
}

fn parse_opt_bool(
    obj: &serde_json::Map<String, Value>,
    path: &str,
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
            format!("{path}.{field}"),
        )),
    }
}

fn parse_opt_null_bool(
    obj: &serde_json::Map<String, Value>,
    path: &str,
    field: &str,
) -> Result<Option<Option<bool>>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(None)),
        Some(Value::Bool(b)) => Ok(Some(Some(*b))),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Boolean,
            },
            format!("{path}.{field}"),
        )),
    }
}

fn parse_opt_null_enum(
    obj: &serde_json::Map<String, Value>,
    path: &str,
    field: &str,
    allowed: &[&str],
    bad: ErrorKind,
) -> Result<Option<Option<String>>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(None)),
        Some(Value::String(s)) => {
            if !allowed.contains(&s.as_str()) {
                return Err(Error::with_path(bad, format!("{path}.{field}")));
            }
            Ok(Some(Some(s.clone())))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::String,
            },
            format!("{path}.{field}"),
        )),
    }
}

fn parse_nonempty_opt_string(
    obj: &serde_json::Map<String, Value>,
    path: &str,
    field: &str,
) -> Result<Option<String>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::String(s)) => {
            if s.is_empty() {
                return Err(Error::with_path(
                    ErrorKind::EmptyStyleFont,
                    format!("{path}.{field}"),
                ));
            }
            Ok(Some(s.clone()))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::String,
            },
            format!("{path}.{field}"),
        )),
    }
}

fn parse_opt_u32_range(
    obj: &serde_json::Map<String, Value>,
    path: &str,
    field: &str,
    min: u64,
    max: u64,
) -> Result<Option<u32>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::Number(n)) => {
            let u = n.as_u64().ok_or_else(|| {
                Error::with_path(
                    ErrorKind::WrongType {
                        field: field.into(),
                        expected: TypeName::Integer,
                    },
                    format!("{path}.{field}"),
                )
            })?;
            if u < min {
                return Err(Error::with_path(
                    ErrorKind::MinValue {
                        field: field.into(),
                        min: min as i64,
                    },
                    format!("{path}.{field}"),
                ));
            }
            if u > max {
                return Err(Error::with_path(
                    ErrorKind::OutOfRange {
                        field: field.into(),
                        min: min as i64,
                        max: max as i64,
                    },
                    format!("{path}.{field}"),
                ));
            }
            Ok(Some(u as u32))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Integer,
            },
            format!("{path}.{field}"),
        )),
    }
}

fn parse_opt_u32(v: Option<&Value>, path: &str) -> Result<Option<u32>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Number(n)) => {
            if let Some(u) = n.as_u64() {
                if u > u32::MAX as u64 {
                    return Err(Error::with_path(
                        ErrorKind::WrongType {
                            field: "integer".into(),
                            expected: TypeName::Integer,
                        },
                        path.to_string(),
                    ));
                }
                return Ok(Some(u as u32));
            }
            Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "integer".into(),
                    expected: TypeName::Integer,
                },
                path.to_string(),
            ))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "integer".into(),
                expected: TypeName::Integer,
            },
            path.to_string(),
        )),
    }
}

fn parse_opt_null_i32(
    obj: &serde_json::Map<String, Value>,
    path: &str,
    field: &str,
) -> Result<Option<Option<i32>>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(None)),
        Some(Value::Number(n)) => {
            let v = n.as_i64().ok_or_else(|| {
                Error::with_path(
                    ErrorKind::WrongType {
                        field: field.into(),
                        expected: TypeName::Integer,
                    },
                    format!("{path}.{field}"),
                )
            })?;
            i32::try_from(v).map(|n| Some(Some(n))).map_err(|_| {
                Error::with_path(
                    ErrorKind::WrongType {
                        field: field.into(),
                        expected: TypeName::Integer,
                    },
                    format!("{path}.{field}"),
                )
            })
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Integer,
            },
            format!("{path}.{field}"),
        )),
    }
}

fn parse_opt_null_u32_field(
    obj: &serde_json::Map<String, Value>,
    path: &str,
    field: &str,
) -> Result<Option<Option<u32>>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(None)),
        Some(Value::Number(n)) => {
            let u = n.as_u64().ok_or_else(|| {
                Error::with_path(
                    ErrorKind::WrongType {
                        field: field.into(),
                        expected: TypeName::Integer,
                    },
                    format!("{path}.{field}"),
                )
            })?;
            u32::try_from(u).map(|v| Some(Some(v))).map_err(|_| {
                Error::with_path(
                    ErrorKind::WrongType {
                        field: field.into(),
                        expected: TypeName::Integer,
                    },
                    format!("{path}.{field}"),
                )
            })
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Integer,
            },
            format!("{path}.{field}"),
        )),
    }
}

fn is_rgb_hex(s: &str) -> bool {
    s.len() == 6 && s.bytes().all(|b| b.is_ascii_hexdigit())
}
