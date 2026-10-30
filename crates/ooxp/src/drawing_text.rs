//! DrawingML 文本框共用：`a:rPr` / `a:pPr` / `a:bodyPr` 的 inspect 与打补丁。
//! 不算排版引擎。不写 theme 色，不改 `a:fld`。不写 `prstTxWarp` / `effectLst`。

use serde_json::{json, Map, Value};

use crate::error::{Error, ErrorKind, TypeName};
use crate::xml::{Element, Node};

pub const KEYS: &[&str] = &[
    "sz",
    "color",
    "textFill",
    "textOutline",
    "bold",
    "italic",
    "underline",
    "font",
    "eastAsiaFont",
    "align",
    "textAnchor",
    "spcBef",
    "spcAft",
    "lnSpc",
    "spc",
    "kern",
];

pub const SZ_MIN: i64 = 100;
pub const SZ_MAX: i64 = 400_000;
pub const ANG_DEFAULT: i64 = 5_400_000;
pub const ANG_MAX: i64 = 21_600_000;
pub const POS_MAX: i64 = 100_000;
pub const SPC_PTS_MAX: i64 = 158_400;
pub const SPC_CHAR_MIN: i64 = -400_000;
pub const SPC_CHAR_MAX: i64 = 400_000;
pub const KERN_MAX: i64 = 400_000;

const FILL_LOCALS: &[&str] = &[
    "noFill",
    "solidFill",
    "gradFill",
    "blipFill",
    "pattFill",
    "grpFill",
];

const ALIGN_OK: &[&str] = &["l", "ctr", "r", "just"];
const ANCHOR_OK: &[&str] = &["t", "ctr", "b", "just", "dist"];
const UNDERLINE_OK: &[&str] = &["sng", "none", "dbl"];

/// 写入补丁。外层 `Option` 表示未点名；内层 `None` 表示去掉显式值。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TextFormat {
    pub sz: Option<Option<i64>>,
    pub color: Option<Option<String>>,
    pub text_fill: Option<Option<TextFill>>,
    pub text_outline: Option<Option<TextOutline>>,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub underline: Option<Option<String>>,
    pub font: Option<String>,
    pub east_asia_font: Option<String>,
    pub align: Option<Option<String>>,
    pub text_anchor: Option<Option<String>>,
    pub spc_bef: Option<Option<TextSpacing>>,
    pub spc_aft: Option<Option<TextSpacing>>,
    pub ln_spc: Option<Option<TextSpacing>>,
    pub spc: Option<Option<i64>>,
    pub kern: Option<Option<i64>>,
}

impl TextFormat {
    pub fn has_run_patch(&self) -> bool {
        self.sz.is_some()
            || self.color.is_some()
            || self.text_fill.is_some()
            || self.text_outline.is_some()
            || self.bold.is_some()
            || self.italic.is_some()
            || self.underline.is_some()
            || self.font.is_some()
            || self.east_asia_font.is_some()
            || self.spc.is_some()
            || self.kern.is_some()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextFill {
    None,
    Gradient { ang: i64, stops: Vec<GradientStop> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GradientStop {
    pub pos: i64,
    pub color: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextOutline {
    pub color: String,
    pub width: i64,
}

/// `a:spcPts` / `a:spcPct`。`pts` 是百分之一磅，`pct` 是百分之一百分点（100000 为 100%）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpacingKind {
    Pts,
    Pct,
}

impl SpacingKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Pts => "pts",
            Self::Pct => "pct",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextSpacing {
    pub kind: SpacingKind,
    pub val: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct RunView {
    sz: Option<i64>,
    color: Option<String>,
    text_fill: Option<TextFill>,
    text_outline: Option<TextOutline>,
    bold: Option<bool>,
    italic: Option<bool>,
    underline: Option<String>,
    font: Option<String>,
    east_asia_font: Option<String>,
    spc: Option<i64>,
    kern: Option<i64>,
}

/// 从对象里读文字格式。`require_one` 为真时至少一个键，否则报 `NeedShapeFormatField`。
pub fn parse(
    obj: &Map<String, Value>,
    prefix: &str,
    require_one: bool,
) -> Result<Option<TextFormat>, Error> {
    let present = KEYS.iter().any(|k| obj.contains_key(*k));
    if !present {
        if require_one {
            return Err(Error::with_path(
                ErrorKind::NeedShapeFormatField,
                prefix.to_string(),
            ));
        }
        return Ok(None);
    }
    if obj.contains_key("color") && obj.contains_key("textFill") {
        return Err(Error::with_path(
            ErrorKind::MutuallyExclusive {
                a: "color".into(),
                b: "textFill".into(),
            },
            prefix.to_string(),
        ));
    }
    let mut fmt = TextFormat::default();
    fmt.sz = parse_opt_sz(obj.get("sz"), &format!("{prefix}.sz"))?;
    fmt.color = parse_opt_hex(obj.get("color"), &format!("{prefix}.color"), "color")?;
    fmt.text_fill = parse_opt_text_fill(obj.get("textFill"), &format!("{prefix}.textFill"))?;
    fmt.text_outline =
        parse_opt_text_outline(obj.get("textOutline"), &format!("{prefix}.textOutline"))?;
    fmt.bold = parse_opt_bool(obj.get("bold"), "bold", prefix)?;
    fmt.italic = parse_opt_bool(obj.get("italic"), "italic", prefix)?;
    fmt.underline = parse_opt_enum(
        obj.get("underline"),
        "underline",
        prefix,
        UNDERLINE_OK,
        ErrorKind::InvalidTextUnderline,
    )?;
    fmt.font = parse_opt_typeface(obj.get("font"), "font", prefix)?;
    fmt.east_asia_font = parse_opt_typeface(obj.get("eastAsiaFont"), "eastAsiaFont", prefix)?;
    fmt.align = parse_opt_enum(
        obj.get("align"),
        "align",
        prefix,
        ALIGN_OK,
        ErrorKind::InvalidTextAlign,
    )?;
    fmt.text_anchor = parse_opt_enum(
        obj.get("textAnchor"),
        "textAnchor",
        prefix,
        ANCHOR_OK,
        ErrorKind::InvalidTextAnchor,
    )?;
    fmt.spc_bef = parse_opt_spacing(obj.get("spcBef"), &format!("{prefix}.spcBef"), "spcBef")?;
    fmt.spc_aft = parse_opt_spacing(obj.get("spcAft"), &format!("{prefix}.spcAft"), "spcAft")?;
    fmt.ln_spc = parse_opt_spacing(obj.get("lnSpc"), &format!("{prefix}.lnSpc"), "lnSpc")?;
    fmt.spc = parse_opt_i64_range(
        obj.get("spc"),
        "spc",
        prefix,
        SPC_CHAR_MIN,
        SPC_CHAR_MAX,
        false,
    )?;
    fmt.kern = parse_opt_i64_range(obj.get("kern"), "kern", prefix, 0, KERN_MAX, true)?;
    Ok(Some(fmt))
}

/// 在 `text` 之后、`stable` 之前插入格式键与 `runs`。
pub fn insert_inspect(map: &mut Map<String, Value>, tx: Option<&Element>) {
    let view = summarize(tx);
    map.insert("sz".into(), opt_json_i64(view.sz));
    map.insert("color".into(), opt_json_str(view.color));
    map.insert("textFill".into(), json_fill(view.text_fill));
    map.insert("textOutline".into(), json_outline(view.text_outline));
    map.insert("bold".into(), opt_json_bool(view.bold));
    map.insert("italic".into(), opt_json_bool(view.italic));
    map.insert("underline".into(), opt_json_str(view.underline));
    map.insert("font".into(), opt_json_str(view.font));
    map.insert("eastAsiaFont".into(), opt_json_str(view.east_asia));
    map.insert("align".into(), opt_json_str(view.align));
    map.insert("textAnchor".into(), opt_json_str(view.anchor));
    map.insert("spcBef".into(), json_spacing(view.spc_bef));
    map.insert("spcAft".into(), json_spacing(view.spc_aft));
    map.insert("lnSpc".into(), json_spacing(view.ln_spc));
    map.insert("runs".into(), Value::Array(view.runs));
}

struct InspectView {
    sz: Option<i64>,
    color: Option<String>,
    text_fill: Option<TextFill>,
    text_outline: Option<TextOutline>,
    bold: Option<bool>,
    italic: Option<bool>,
    underline: Option<String>,
    font: Option<String>,
    east_asia: Option<String>,
    align: Option<String>,
    anchor: Option<String>,
    spc_bef: Option<TextSpacing>,
    spc_aft: Option<TextSpacing>,
    ln_spc: Option<TextSpacing>,
    runs: Vec<Value>,
}

fn summarize(tx: Option<&Element>) -> InspectView {
    let Some(tx) = tx else {
        return InspectView::empty();
    };
    let mut runs: Vec<RunView> = Vec::new();
    let mut run_jsons: Vec<Value> = Vec::new();
    let mut aligns: Vec<Option<String>> = Vec::new();
    let mut befs: Vec<Option<TextSpacing>> = Vec::new();
    let mut afts: Vec<Option<TextSpacing>> = Vec::new();
    let mut lns: Vec<Option<TextSpacing>> = Vec::new();
    for_each_p(tx, &mut |p| {
        aligns.push(p_align(p));
        befs.push(p_spacing(p, "spcBef"));
        afts.push(p_spacing(p, "spcAft"));
        lns.push(p_spacing(p, "lnSpc"));
        for_each_text_run(p, &mut |r| {
            let view = run_view(r.child_by_local("rPr"));
            run_jsons.push(run_json(r, &view));
            runs.push(view);
        });
    });
    let mixed_or = |pick: fn(&RunView) -> Option<String>| -> Option<String> {
        if runs.is_empty() {
            return None;
        }
        let first = pick(&runs[0]);
        if runs.iter().all(|r| pick(r) == first) {
            first
        } else {
            None
        }
    };
    let mixed_or_i64 = || -> Option<i64> {
        if runs.is_empty() {
            return None;
        }
        let first = runs[0].sz;
        if runs.iter().all(|r| r.sz == first) {
            first
        } else {
            None
        }
    };
    let mixed_or_bool = |pick: fn(&RunView) -> Option<bool>| -> Option<bool> {
        if runs.is_empty() {
            return None;
        }
        let first = pick(&runs[0]);
        if runs.iter().all(|r| pick(r) == first) {
            first
        } else {
            None
        }
    };
    let mixed_or_fill = || -> Option<TextFill> {
        if runs.is_empty() {
            return None;
        }
        let first = runs[0].text_fill.clone();
        if runs.iter().all(|r| r.text_fill == first) {
            first
        } else {
            None
        }
    };
    let mixed_or_outline = || -> Option<TextOutline> {
        if runs.is_empty() {
            return None;
        }
        let first = runs[0].text_outline.clone();
        if runs.iter().all(|r| r.text_outline == first) {
            first
        } else {
            None
        }
    };
    let align = if aligns.is_empty() {
        None
    } else {
        let first = aligns[0].clone();
        if aligns.iter().all(|a| *a == first) {
            first
        } else {
            None
        }
    };
    InspectView {
        sz: mixed_or_i64(),
        color: mixed_or(|r| r.color.clone()),
        text_fill: mixed_or_fill(),
        text_outline: mixed_or_outline(),
        bold: mixed_or_bool(|r| r.bold),
        italic: mixed_or_bool(|r| r.italic),
        underline: mixed_or(|r| r.underline.clone()),
        font: mixed_or(|r| r.font.clone()),
        east_asia: mixed_or(|r| r.east_asia_font.clone()),
        align,
        anchor: body_anchor(tx),
        spc_bef: mixed_spacing(&befs),
        spc_aft: mixed_spacing(&afts),
        ln_spc: mixed_spacing(&lns),
        runs: run_jsons,
    }
}

fn mixed_spacing(items: &[Option<TextSpacing>]) -> Option<TextSpacing> {
    if items.is_empty() {
        return None;
    }
    let first = items[0];
    if items.iter().all(|a| *a == first) {
        first
    } else {
        None
    }
}

impl InspectView {
    fn empty() -> Self {
        Self {
            sz: None,
            color: None,
            text_fill: None,
            text_outline: None,
            bold: None,
            italic: None,
            underline: None,
            font: None,
            east_asia: None,
            align: None,
            anchor: None,
            spc_bef: None,
            spc_aft: None,
            ln_spc: None,
            runs: Vec::new(),
        }
    }
}

/// 把补丁打到 `txBody`：每个文本 run 的 `rPr`、`endParaRPr`、各段 `pPr`、`bodyPr`。
pub fn apply(tx: &mut Element, fmt: &TextFormat) {
    if let Some(anchor) = &fmt.text_anchor {
        patch_body_anchor(tx, anchor.as_deref());
    }
    let run_needed = fmt.has_run_patch();
    for_each_p_mut(tx, &mut |p| {
        if let Some(algn) = &fmt.align {
            patch_p_align(p, algn.as_deref());
        }
        if let Some(spc) = &fmt.spc_bef {
            patch_p_spacing(p, "spcBef", spc.as_ref());
        }
        if let Some(spc) = &fmt.spc_aft {
            patch_p_spacing(p, "spcAft", spc.as_ref());
        }
        if let Some(spc) = &fmt.ln_spc {
            patch_p_spacing(p, "lnSpc", spc.as_ref());
        }
        if !run_needed {
            return;
        }
        let mut run_idx = Vec::new();
        let mut end_idx = None;
        for (i, n) in p.children.iter().enumerate() {
            if let Node::Element(e) = n {
                match e.local_name() {
                    "r" => run_idx.push(i),
                    "endParaRPr" => end_idx = Some(i),
                    _ => {}
                }
            }
        }
        let empty_runs = run_idx.is_empty();
        for i in run_idx {
            if let Node::Element(r) = &mut p.children[i] {
                let rpr = ensure_child_named(r, "rPr", 0);
                patch_rpr(rpr, fmt);
            }
        }
        match end_idx {
            Some(i) => {
                if let Node::Element(end) = &mut p.children[i] {
                    patch_rpr(end, fmt);
                }
            }
            None if empty_runs => {
                let mut end = Element::new(p.prefixed("endParaRPr"));
                patch_rpr(&mut end, fmt);
                p.push_element(end);
            }
            None => {}
        }
    });
}

fn parse_opt_sz(v: Option<&Value>, path: &str) -> Result<Option<Option<i64>>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(None)),
        Some(Value::Number(n)) => {
            let Some(n) = n
                .as_i64()
                .or_else(|| n.as_u64().and_then(|u| i64::try_from(u).ok()))
            else {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: "sz".into(),
                        expected: TypeName::Integer,
                    },
                    path.to_string(),
                ));
            };
            if n < SZ_MIN {
                return Err(Error::with_path(
                    ErrorKind::MinValue {
                        field: "sz".into(),
                        min: SZ_MIN,
                    },
                    path.to_string(),
                ));
            }
            if n > SZ_MAX {
                return Err(Error::with_path(
                    ErrorKind::OutOfRange {
                        field: "sz".into(),
                        min: SZ_MIN,
                        max: SZ_MAX,
                    },
                    path.to_string(),
                ));
            }
            Ok(Some(Some(n)))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "sz".into(),
                expected: TypeName::Integer,
            },
            path.to_string(),
        )),
    }
}

fn parse_opt_hex(
    v: Option<&Value>,
    path: &str,
    field: &str,
) -> Result<Option<Option<String>>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(None)),
        Some(Value::String(s)) => parse_hex_str(s, path).map(|h| Some(Some(h))),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::String,
            },
            path.to_string(),
        )),
    }
}

fn parse_hex_str(s: &str, path: &str) -> Result<String, Error> {
    if s.len() == 6 && s.chars().all(|c| c.is_ascii_hexdigit()) {
        Ok(s.to_ascii_uppercase())
    } else {
        Err(Error::with_path(
            ErrorKind::InvalidHexColor,
            path.to_string(),
        ))
    }
}

fn parse_opt_text_fill(v: Option<&Value>, path: &str) -> Result<Option<Option<TextFill>>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(None)),
        Some(Value::Object(obj)) => Ok(Some(Some(parse_text_fill_obj(obj, path)?))),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "textFill".into(),
                expected: TypeName::Object,
            },
            path.to_string(),
        )),
    }
}

fn parse_text_fill_obj(obj: &Map<String, Value>, path: &str) -> Result<TextFill, Error> {
    let type_name = match obj.get("type") {
        Some(Value::String(s)) => s.as_str(),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "type".into(),
                    expected: TypeName::String,
                },
                format!("{path}.type"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "type".into(),
                },
                format!("{path}.type"),
            ));
        }
    };
    match type_name {
        "none" => {
            deny_unknown(obj, path, &["type"])?;
            Ok(TextFill::None)
        }
        "gradient" => {
            deny_unknown(obj, path, &["type", "ang", "stops"])?;
            let ang = parse_opt_ang(obj.get("ang"), &format!("{path}.ang"))?.unwrap_or(ANG_DEFAULT);
            let stops = parse_stops(obj.get("stops"), &format!("{path}.stops"))?;
            Ok(TextFill::Gradient { ang, stops })
        }
        _ => Err(Error::with_path(
            ErrorKind::InvalidTextFillType,
            format!("{path}.type"),
        )),
    }
}

fn parse_opt_ang(v: Option<&Value>, path: &str) -> Result<Option<i64>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Number(n)) => {
            let Some(n) = n
                .as_i64()
                .or_else(|| n.as_u64().and_then(|u| i64::try_from(u).ok()))
            else {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: "ang".into(),
                        expected: TypeName::Integer,
                    },
                    path.to_string(),
                ));
            };
            if n < 0 {
                return Err(Error::with_path(
                    ErrorKind::MinValue {
                        field: "ang".into(),
                        min: 0,
                    },
                    path.to_string(),
                ));
            }
            if n > ANG_MAX {
                return Err(Error::with_path(
                    ErrorKind::OutOfRange {
                        field: "ang".into(),
                        min: 0,
                        max: ANG_MAX,
                    },
                    path.to_string(),
                ));
            }
            Ok(Some(n))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "ang".into(),
                expected: TypeName::Integer,
            },
            path.to_string(),
        )),
    }
}

fn parse_stops(v: Option<&Value>, path: &str) -> Result<Vec<GradientStop>, Error> {
    let Some(arr) = v else {
        return Err(Error::with_path(
            ErrorKind::NeedGradientStops,
            path.to_string(),
        ));
    };
    let Value::Array(items) = arr else {
        return Err(Error::with_path(
            ErrorKind::WrongType {
                field: "stops".into(),
                expected: TypeName::Array,
            },
            path.to_string(),
        ));
    };
    if items.len() < 2 {
        return Err(Error::with_path(
            ErrorKind::NeedGradientStops,
            path.to_string(),
        ));
    }
    let mut out = Vec::new();
    let mut seen = Vec::new();
    for (i, item) in items.iter().enumerate() {
        let prefix = format!("{path}[{i}]");
        let obj = item.as_object().ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: "stops".into(),
                    expected: TypeName::Object,
                },
                prefix.clone(),
            )
        })?;
        deny_unknown(obj, &prefix, &["pos", "color"])?;
        let pos = parse_stop_pos(obj.get("pos"), &format!("{prefix}.pos"))?;
        if seen.contains(&pos) {
            return Err(Error::with_path(
                ErrorKind::DuplicateGradientPos,
                format!("{prefix}.pos"),
            ));
        }
        seen.push(pos);
        let color = match obj.get("color") {
            Some(Value::String(s)) => parse_hex_str(s, &format!("{prefix}.color"))?,
            Some(_) => {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: "color".into(),
                        expected: TypeName::String,
                    },
                    format!("{prefix}.color"),
                ));
            }
            None => {
                return Err(Error::with_path(
                    ErrorKind::MissingField {
                        field: "color".into(),
                    },
                    format!("{prefix}.color"),
                ));
            }
        };
        out.push(GradientStop { pos, color });
    }
    Ok(out)
}

fn parse_stop_pos(v: Option<&Value>, path: &str) -> Result<i64, Error> {
    match v {
        Some(Value::Number(n)) => {
            let Some(n) = n
                .as_i64()
                .or_else(|| n.as_u64().and_then(|u| i64::try_from(u).ok()))
            else {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: "pos".into(),
                        expected: TypeName::Integer,
                    },
                    path.to_string(),
                ));
            };
            if n < 0 {
                return Err(Error::with_path(
                    ErrorKind::MinValue {
                        field: "pos".into(),
                        min: 0,
                    },
                    path.to_string(),
                ));
            }
            if n > POS_MAX {
                return Err(Error::with_path(
                    ErrorKind::OutOfRange {
                        field: "pos".into(),
                        min: 0,
                        max: POS_MAX,
                    },
                    path.to_string(),
                ));
            }
            Ok(n)
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "pos".into(),
                expected: TypeName::Integer,
            },
            path.to_string(),
        )),
        None => Err(Error::with_path(
            ErrorKind::MissingField {
                field: "pos".into(),
            },
            path.to_string(),
        )),
    }
}

fn parse_opt_text_outline(
    v: Option<&Value>,
    path: &str,
) -> Result<Option<Option<TextOutline>>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(None)),
        Some(Value::Object(obj)) => Ok(Some(Some(parse_text_outline_obj(obj, path)?))),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "textOutline".into(),
                expected: TypeName::Object,
            },
            path.to_string(),
        )),
    }
}

fn parse_text_outline_obj(obj: &Map<String, Value>, path: &str) -> Result<TextOutline, Error> {
    deny_unknown(obj, path, &["color", "width"])?;
    if !obj.contains_key("color") || !obj.contains_key("width") {
        return Err(Error::with_path(
            ErrorKind::NeedTextOutlineField,
            path.to_string(),
        ));
    }
    let color = match obj.get("color") {
        Some(Value::String(s)) => parse_hex_str(s, &format!("{path}.color"))?,
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "color".into(),
                    expected: TypeName::String,
                },
                format!("{path}.color"),
            ));
        }
        None => unreachable!(),
    };
    let width = match obj.get("width") {
        Some(Value::Number(n)) => {
            let Some(n) = n
                .as_i64()
                .or_else(|| n.as_u64().and_then(|u| i64::try_from(u).ok()))
            else {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: "width".into(),
                        expected: TypeName::Integer,
                    },
                    format!("{path}.width"),
                ));
            };
            if n < 1 {
                return Err(Error::with_path(
                    ErrorKind::MinValue {
                        field: "width".into(),
                        min: 1,
                    },
                    format!("{path}.width"),
                ));
            }
            n
        }
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "width".into(),
                    expected: TypeName::Integer,
                },
                format!("{path}.width"),
            ));
        }
        None => unreachable!(),
    };
    Ok(TextOutline { color, width })
}

fn deny_unknown(obj: &Map<String, Value>, prefix: &str, allowed: &[&str]) -> Result<(), Error> {
    for k in obj.keys() {
        if !allowed.iter().any(|a| *a == k) {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("{prefix}.{k}"),
            ));
        }
    }
    Ok(())
}

fn parse_opt_bool(v: Option<&Value>, field: &str, prefix: &str) -> Result<Option<bool>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Bool(b)) => Ok(Some(*b)),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Boolean,
            },
            format!("{prefix}.{field}"),
        )),
    }
}

fn parse_opt_typeface(
    v: Option<&Value>,
    field: &str,
    prefix: &str,
) -> Result<Option<String>, Error> {
    match v {
        None => Ok(None),
        Some(Value::String(s)) if s.is_empty() => Err(Error::with_path(
            ErrorKind::EmptyShapeTypeface,
            format!("{prefix}.{field}"),
        )),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::NonEmptyString,
            },
            format!("{prefix}.{field}"),
        )),
    }
}

fn parse_opt_enum(
    v: Option<&Value>,
    field: &str,
    prefix: &str,
    allowed: &[&str],
    bad: ErrorKind,
) -> Result<Option<Option<String>>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(None)),
        Some(Value::String(s)) => {
            if allowed.iter().any(|a| *a == s) {
                Ok(Some(Some(s.clone())))
            } else {
                Err(Error::with_path(bad, format!("{prefix}.{field}")))
            }
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::String,
            },
            format!("{prefix}.{field}"),
        )),
    }
}

fn parse_opt_spacing(
    v: Option<&Value>,
    path: &str,
    field: &str,
) -> Result<Option<Option<TextSpacing>>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(None)),
        Some(Value::Object(obj)) => Ok(Some(Some(parse_spacing_obj(obj, path)?))),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Object,
            },
            path.to_string(),
        )),
    }
}

fn parse_spacing_obj(obj: &Map<String, Value>, path: &str) -> Result<TextSpacing, Error> {
    deny_unknown(obj, path, &["type", "val"])?;
    let type_name = match obj.get("type") {
        Some(Value::String(s)) => s.as_str(),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "type".into(),
                    expected: TypeName::String,
                },
                format!("{path}.type"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "type".into(),
                },
                format!("{path}.type"),
            ));
        }
    };
    let kind = match type_name {
        "pts" => SpacingKind::Pts,
        "pct" => SpacingKind::Pct,
        _ => {
            return Err(Error::with_path(
                ErrorKind::InvalidTextSpacing,
                format!("{path}.type"),
            ));
        }
    };
    let val = match obj.get("val") {
        Some(Value::Number(n)) => n
            .as_i64()
            .or_else(|| n.as_u64().and_then(|u| i64::try_from(u).ok()))
            .ok_or_else(|| {
                Error::with_path(
                    ErrorKind::WrongType {
                        field: "val".into(),
                        expected: TypeName::Integer,
                    },
                    format!("{path}.val"),
                )
            })?,
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "val".into(),
                    expected: TypeName::Integer,
                },
                format!("{path}.val"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "val".into(),
                },
                format!("{path}.val"),
            ));
        }
    };
    match kind {
        SpacingKind::Pts => {
            if val < 0 {
                return Err(Error::with_path(
                    ErrorKind::MinValue {
                        field: "val".into(),
                        min: 0,
                    },
                    format!("{path}.val"),
                ));
            }
            if val > SPC_PTS_MAX {
                return Err(Error::with_path(
                    ErrorKind::OutOfRange {
                        field: "val".into(),
                        min: 0,
                        max: SPC_PTS_MAX,
                    },
                    format!("{path}.val"),
                ));
            }
        }
        SpacingKind::Pct => {
            if val < 0 {
                return Err(Error::with_path(
                    ErrorKind::MinValue {
                        field: "val".into(),
                        min: 0,
                    },
                    format!("{path}.val"),
                ));
            }
        }
    }
    Ok(TextSpacing { kind, val })
}

fn parse_opt_i64_range(
    v: Option<&Value>,
    field: &str,
    prefix: &str,
    min: i64,
    max: i64,
    min_is_floor: bool,
) -> Result<Option<Option<i64>>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(None)),
        Some(Value::Number(n)) => {
            let Some(n) = n
                .as_i64()
                .or_else(|| n.as_u64().and_then(|u| i64::try_from(u).ok()))
            else {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: field.into(),
                        expected: TypeName::Integer,
                    },
                    format!("{prefix}.{field}"),
                ));
            };
            if min_is_floor && n < min {
                return Err(Error::with_path(
                    ErrorKind::MinValue {
                        field: field.into(),
                        min,
                    },
                    format!("{prefix}.{field}"),
                ));
            }
            if n < min || n > max {
                return Err(Error::with_path(
                    ErrorKind::OutOfRange {
                        field: field.into(),
                        min,
                        max,
                    },
                    format!("{prefix}.{field}"),
                ));
            }
            Ok(Some(Some(n)))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Integer,
            },
            format!("{prefix}.{field}"),
        )),
    }
}

fn run_view(rpr: Option<&Element>) -> RunView {
    let Some(rpr) = rpr else {
        return RunView::default();
    };
    RunView {
        sz: attr_i64(rpr, "sz"),
        color: srgb_of(rpr),
        text_fill: fill_view(rpr),
        text_outline: outline_view(rpr),
        bold: on_off(rpr, "b"),
        italic: on_off(rpr, "i"),
        underline: rpr.attr_local("u").map(str::to_string),
        font: typeface_of(rpr, "latin"),
        east_asia_font: typeface_of(rpr, "ea"),
        spc: attr_i64(rpr, "spc"),
        kern: attr_i64(rpr, "kern"),
    }
}

fn run_json(r: &Element, view: &RunView) -> Value {
    let mut m = Map::new();
    m.insert("text".into(), json!(run_text(r)));
    m.insert("sz".into(), opt_json_i64(view.sz));
    m.insert("color".into(), opt_json_str(view.color.clone()));
    m.insert("textFill".into(), json_fill(view.text_fill.clone()));
    m.insert(
        "textOutline".into(),
        json_outline(view.text_outline.clone()),
    );
    m.insert("bold".into(), opt_json_bool(view.bold));
    m.insert("italic".into(), opt_json_bool(view.italic));
    m.insert("underline".into(), opt_json_str(view.underline.clone()));
    m.insert("font".into(), opt_json_str(view.font.clone()));
    m.insert(
        "eastAsiaFont".into(),
        opt_json_str(view.east_asia_font.clone()),
    );
    m.insert("spc".into(), opt_json_i64(view.spc));
    m.insert("kern".into(), opt_json_i64(view.kern));
    Value::Object(m)
}

fn run_text(r: &Element) -> String {
    r.child_by_local("t")
        .map(|t| t.text_content())
        .unwrap_or_default()
}

fn attr_i64(el: &Element, local: &str) -> Option<i64> {
    el.attr_local(local).and_then(|s| s.parse().ok())
}

fn srgb_of(rpr: &Element) -> Option<String> {
    rpr.child_by_local("solidFill")
        .and_then(|f| f.child_by_local("srgbClr"))
        .and_then(|c| c.attr_local("val"))
        .filter(|s| s.len() == 6 && s.chars().all(|c| c.is_ascii_hexdigit()))
        .map(|s| s.to_ascii_uppercase())
}

fn fill_view(rpr: &Element) -> Option<TextFill> {
    if rpr.child_by_local("noFill").is_some() {
        return Some(TextFill::None);
    }
    let grad = rpr.child_by_local("gradFill")?;
    if grad.child_by_local("path").is_some() {
        return None;
    }
    let lin = grad.child_by_local("lin")?;
    let ang = lin.attr_local("ang")?.parse().ok()?;
    if !(0..=ANG_MAX).contains(&ang) {
        return None;
    }
    let gs_lst = grad.child_by_local("gsLst")?;
    let mut stops = Vec::new();
    for gs in gs_lst.element_children() {
        if gs.local_name() != "gs" {
            continue;
        }
        let pos = parse_gs_pos(gs.attr_local("pos")?)?;
        let color = gs
            .child_by_local("srgbClr")
            .and_then(|c| {
                if c.element_children().next().is_some() {
                    return None;
                }
                c.attr_local("val")
            })
            .filter(|s| s.len() == 6 && s.chars().all(|c| c.is_ascii_hexdigit()))
            .map(|s| s.to_ascii_uppercase())?;
        stops.push(GradientStop { pos, color });
    }
    if stops.len() < 2 {
        return None;
    }
    Some(TextFill::Gradient { ang, stops })
}

fn parse_gs_pos(raw: &str) -> Option<i64> {
    let t = raw.trim();
    if let Some(p) = t.strip_suffix('%') {
        let n: i64 = p.parse().ok()?;
        let v = n.checked_mul(1000)?;
        (0..=POS_MAX).contains(&v).then_some(v)
    } else {
        let n: i64 = t.parse().ok()?;
        (0..=POS_MAX).contains(&n).then_some(n)
    }
}

fn outline_view(rpr: &Element) -> Option<TextOutline> {
    let ln = rpr.child_by_local("ln")?;
    let width: i64 = ln.attr_local("w")?.parse().ok()?;
    if width < 1 {
        return None;
    }
    let color = ln
        .child_by_local("solidFill")
        .and_then(|f| f.child_by_local("srgbClr"))
        .and_then(|c| {
            if c.element_children().next().is_some() {
                return None;
            }
            c.attr_local("val")
        })
        .filter(|s| s.len() == 6 && s.chars().all(|c| c.is_ascii_hexdigit()))
        .map(|s| s.to_ascii_uppercase())?;
    Some(TextOutline { color, width })
}

fn typeface_of(rpr: &Element, local: &str) -> Option<String> {
    rpr.child_by_local(local)
        .and_then(|e| e.attr_local("typeface"))
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn on_off(el: &Element, local: &str) -> Option<bool> {
    let v = el.attr_local(local)?;
    match v {
        "1" | "true" | "True" | "on" => Some(true),
        "0" | "false" | "False" | "off" => Some(false),
        _ => None,
    }
}

fn p_align(p: &Element) -> Option<String> {
    p.child_by_local("pPr")
        .and_then(|ppr| ppr.attr_local("algn"))
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn body_anchor(tx: &Element) -> Option<String> {
    tx.child_by_local("bodyPr")
        .and_then(|b| b.attr_local("anchor"))
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn patch_body_anchor(tx: &mut Element, val: Option<&str>) {
    let body = ensure_child_named(tx, "bodyPr", 0);
    match val {
        Some(v) => set_attr_local(body, "anchor", v),
        None => remove_attr_local(body, "anchor"),
    }
}

fn patch_p_align(p: &mut Element, val: Option<&str>) {
    let ppr = ensure_child_named(p, "pPr", 0);
    match val {
        Some(v) => set_attr_local(ppr, "algn", v),
        None => remove_attr_local(ppr, "algn"),
    }
}

fn p_spacing(p: &Element, local: &str) -> Option<TextSpacing> {
    let el = p.child_by_local("pPr")?.child_by_local(local)?;
    let pts = el.child_by_local("spcPts");
    let pct = el.child_by_local("spcPct");
    match (pts, pct) {
        (Some(pts), None) => {
            let val: i64 = pts.attr_local("val")?.parse().ok()?;
            Some(TextSpacing {
                kind: SpacingKind::Pts,
                val,
            })
        }
        (None, Some(pct)) => {
            let val: i64 = pct.attr_local("val")?.parse().ok()?;
            Some(TextSpacing {
                kind: SpacingKind::Pct,
                val,
            })
        }
        _ => None,
    }
}

fn patch_p_spacing(p: &mut Element, local: &str, val: Option<&TextSpacing>) {
    match val {
        None => {
            if let Some(ppr) = p.child_by_local_mut("pPr") {
                ppr.children.retain(|n| match n {
                    Node::Element(e) => e.local_name() != local,
                    _ => true,
                });
            }
        }
        Some(spc) => {
            let ppr = ensure_child_named(p, "pPr", 0);
            let el = insert_or_get_spacing(ppr, local);
            el.children.clear();
            let child = match spc.kind {
                SpacingKind::Pts => "spcPts",
                SpacingKind::Pct => "spcPct",
            };
            el.push_element(Element::new(el.prefixed(child)).with_attr("val", spc.val.to_string()));
        }
    }
}

fn insert_or_get_spacing<'a>(ppr: &'a mut Element, local: &str) -> &'a mut Element {
    if ppr.child_by_local(local).is_some() {
        return ppr.child_by_local_mut(local).expect("刚查过");
    }
    let at = spacing_insert_index(ppr, local);
    ppr.children.insert(
        at.min(ppr.children.len()),
        Node::Element(Element::new(ppr.prefixed(local))),
    );
    ppr.child_by_local_mut(local).expect("刚插入")
}

fn spacing_insert_index(ppr: &Element, local: &str) -> usize {
    let after = match local {
        "spcBef" => ppr
            .children
            .iter()
            .position(|n| matches!(n, Node::Element(e) if e.local_name() == "lnSpc"))
            .map(|i| i + 1),
        "spcAft" => ppr
            .children
            .iter()
            .rposition(|n| {
                matches!(n, Node::Element(e) if e.local_name() == "spcBef" || e.local_name() == "lnSpc")
            })
            .map(|i| i + 1),
        _ => None,
    };
    after.unwrap_or(0)
}

pub(crate) fn patch_rpr(rpr: &mut Element, fmt: &TextFormat) {
    if let Some(sz) = &fmt.sz {
        match sz {
            Some(n) => set_attr_local(rpr, "sz", &n.to_string()),
            None => remove_attr_local(rpr, "sz"),
        }
    }
    if let Some(bold) = fmt.bold {
        set_attr_local(rpr, "b", if bold { "1" } else { "0" });
    }
    if let Some(italic) = fmt.italic {
        set_attr_local(rpr, "i", if italic { "1" } else { "0" });
    }
    if let Some(u) = &fmt.underline {
        match u {
            Some(v) => set_attr_local(rpr, "u", v),
            None => remove_attr_local(rpr, "u"),
        }
    }
    if let Some(outline) = &fmt.text_outline {
        remove_ln(rpr);
        if let Some(o) = outline {
            rpr.children.insert(0, Node::Element(build_ln(rpr, o)));
        }
    }
    if let Some(color) = &fmt.color {
        remove_fills(rpr);
        if let Some(hex) = color {
            insert_fill(rpr, build_solid_fill(rpr, hex));
        }
    }
    if let Some(fill) = &fmt.text_fill {
        remove_fills(rpr);
        match fill {
            Some(TextFill::None) => insert_fill(rpr, Element::new(rpr.prefixed("noFill"))),
            Some(TextFill::Gradient { ang, stops }) => {
                insert_fill(rpr, build_grad_fill(rpr, *ang, stops));
            }
            None => {}
        }
    }
    if let Some(font) = &fmt.font {
        set_typeface(rpr, "latin", font);
    }
    if let Some(ea) = &fmt.east_asia_font {
        set_typeface(rpr, "ea", ea);
    }
    if let Some(spc) = &fmt.spc {
        match spc {
            Some(n) => set_attr_local(rpr, "spc", &n.to_string()),
            None => remove_attr_local(rpr, "spc"),
        }
    }
    if let Some(kern) = &fmt.kern {
        match kern {
            Some(n) => set_attr_local(rpr, "kern", &n.to_string()),
            None => remove_attr_local(rpr, "kern"),
        }
    }
}

fn remove_fills(rpr: &mut Element) {
    rpr.children.retain(|n| match n {
        Node::Element(e) => !FILL_LOCALS.contains(&e.local_name()),
        _ => true,
    });
}

fn remove_ln(rpr: &mut Element) {
    rpr.children.retain(|n| match n {
        Node::Element(e) => e.local_name() != "ln",
        _ => true,
    });
}

fn fill_insert_index(rpr: &Element) -> usize {
    rpr.children
        .iter()
        .position(|n| matches!(n, Node::Element(e) if e.local_name() == "ln"))
        .map(|i| i + 1)
        .unwrap_or(0)
}

fn insert_fill(rpr: &mut Element, fill: Element) {
    let at = fill_insert_index(rpr);
    rpr.children.insert(at, Node::Element(fill));
}

fn build_solid_fill(rpr: &Element, hex: &str) -> Element {
    let mut fill = Element::new(rpr.prefixed("solidFill"));
    fill.push_element(
        Element::new(rpr.prefixed("srgbClr")).with_attr("val", hex.to_ascii_uppercase()),
    );
    fill
}

fn build_grad_fill(rpr: &Element, ang: i64, stops: &[GradientStop]) -> Element {
    let mut fill = Element::new(rpr.prefixed("gradFill"))
        .with_attr("flip", "none")
        .with_attr("rotWithShape", "1");
    let mut gs_lst = Element::new(rpr.prefixed("gsLst"));
    for stop in stops {
        let mut gs = Element::new(rpr.prefixed("gs")).with_attr("pos", stop.pos.to_string());
        gs.push_element(
            Element::new(rpr.prefixed("srgbClr")).with_attr("val", stop.color.to_ascii_uppercase()),
        );
        gs_lst.push_element(gs);
    }
    fill.push_element(gs_lst);
    fill.push_element(
        Element::new(rpr.prefixed("lin"))
            .with_attr("ang", ang.to_string())
            .with_attr("scaled", "1"),
    );
    fill.push_element(Element::new(rpr.prefixed("tileRect")));
    fill
}

fn build_ln(rpr: &Element, outline: &TextOutline) -> Element {
    let mut ln = Element::new(rpr.prefixed("ln")).with_attr("w", outline.width.to_string());
    ln.push_element(build_solid_fill(rpr, &outline.color));
    ln
}

fn set_typeface(rpr: &mut Element, local: &str, face: &str) {
    if let Some(el) = rpr.child_by_local_mut(local) {
        set_attr_local(el, "typeface", face);
        return;
    }
    rpr.push_element(Element::new(rpr.prefixed(local)).with_attr("typeface", face));
}

fn ensure_child_named<'a>(parent: &'a mut Element, local: &str, at: usize) -> &'a mut Element {
    if parent.child_by_local(local).is_some() {
        return parent.child_by_local_mut(local).expect("刚查过");
    }
    parent.children.insert(
        at.min(parent.children.len()),
        Node::Element(Element::new(parent.prefixed(local))),
    );
    parent.child_by_local_mut(local).expect("刚插入")
}

fn set_attr_local(el: &mut Element, local: &str, val: &str) {
    if let Some((_, v)) = el
        .attrs
        .iter_mut()
        .find(|(k, _)| k.rsplit(':').next().unwrap_or(k) == local)
    {
        *v = val.to_string();
        return;
    }
    el.attrs.push((local.to_string(), val.to_string()));
}

fn remove_attr_local(el: &mut Element, local: &str) {
    el.attrs
        .retain(|(k, _)| k.rsplit(':').next().unwrap_or(k) != local);
}

fn first_alt_child_index(el: &Element) -> Option<usize> {
    crate::xml::alternate_child(el).unwrap_or(None)
}

fn for_each_p(el: &Element, f: &mut impl FnMut(&Element)) {
    for child in el.element_children() {
        match child.local_name() {
            "AlternateContent" => {
                if crate::drawing_math::is_math_ac(child) {
                    continue;
                }
                if let Some(i) = first_alt_child_index(child) {
                    if let Node::Element(inner) = &child.children[i] {
                        for_each_p(inner, f);
                    }
                }
            }
            "p" => f(child),
            "fld" | "bodyPr" | "lstStyle" => {}
            _ => for_each_p(child, f),
        }
    }
}

fn for_each_p_mut(el: &mut Element, f: &mut impl FnMut(&mut Element)) {
    let mut i = 0;
    while i < el.children.len() {
        let local = match &el.children[i] {
            Node::Element(e) => e.local_name().to_string(),
            _ => {
                i += 1;
                continue;
            }
        };
        match local.as_str() {
            "AlternateContent" => {
                let skip = matches!(&el.children[i], Node::Element(e) if crate::drawing_math::is_math_ac(e));
                if !skip {
                    if let Node::Element(ac) = &mut el.children[i] {
                        if let Some(j) = first_alt_child_index(ac) {
                            if let Node::Element(inner) = &mut ac.children[j] {
                                for_each_p_mut(inner, f);
                            }
                        }
                    }
                }
            }
            "p" => {
                if let Node::Element(p) = &mut el.children[i] {
                    f(p);
                }
            }
            "fld" | "bodyPr" | "lstStyle" => {}
            _ => {
                if let Node::Element(e) = &mut el.children[i] {
                    for_each_p_mut(e, f);
                }
            }
        }
        i += 1;
    }
}

fn for_each_text_run(p: &Element, f: &mut impl FnMut(&Element)) {
    for child in p.element_children() {
        match child.local_name() {
            "r" => f(child),
            "fld" => {}
            "AlternateContent" => {
                if crate::drawing_math::is_math_ac(child) {
                    continue;
                }
                if let Some(i) = first_alt_child_index(child) {
                    if let Node::Element(inner) = &child.children[i] {
                        for_each_text_run(inner, f);
                    }
                }
            }
            _ => {}
        }
    }
}

fn json_fill(v: Option<TextFill>) -> Value {
    match v {
        None => Value::Null,
        Some(TextFill::None) => json!({ "type": "none" }),
        Some(TextFill::Gradient { ang, stops }) => {
            let mut m = Map::new();
            m.insert("type".into(), json!("gradient"));
            m.insert("ang".into(), json!(ang));
            let stop_vals: Vec<Value> = stops
                .into_iter()
                .map(|s| {
                    let mut sm = Map::new();
                    sm.insert("pos".into(), json!(s.pos));
                    sm.insert("color".into(), json!(s.color));
                    Value::Object(sm)
                })
                .collect();
            m.insert("stops".into(), Value::Array(stop_vals));
            Value::Object(m)
        }
    }
}

fn json_outline(v: Option<TextOutline>) -> Value {
    match v {
        None => Value::Null,
        Some(o) => {
            let mut m = Map::new();
            m.insert("color".into(), json!(o.color));
            m.insert("width".into(), json!(o.width));
            Value::Object(m)
        }
    }
}

fn json_spacing(v: Option<TextSpacing>) -> Value {
    match v {
        None => Value::Null,
        Some(s) => json!({ "type": s.kind.as_str(), "val": s.val }),
    }
}

fn opt_json_i64(v: Option<i64>) -> Value {
    match v {
        Some(n) => json!(n),
        None => Value::Null,
    }
}

fn opt_json_str(v: Option<String>) -> Value {
    match v {
        Some(s) => json!(s),
        None => Value::Null,
    }
}

fn opt_json_bool(v: Option<bool>) -> Value {
    match v {
        Some(b) => json!(b),
        None => Value::Null,
    }
}

pub fn visible_text(sp: &Element) -> String {
    let Some(tx) = sp.child_by_local("txBody") else {
        return String::new();
    };
    txbody_visible_text(tx)
}

pub fn txbody_visible_text(tx: &Element) -> String {
    let mut out = String::new();
    collect_visible_text(tx, &mut out);
    out
}

/// 只收 `a:r` 里的 `a:t` 和段落级 `a:br`。
/// `a:fld` 是生成文本（§21.1.2.2.4），缓存的 `a:t` 不进 inspect 的 `text`。
fn collect_visible_text(el: &Element, out: &mut String) {
    if el.local_name() == "AlternateContent" {
        if crate::drawing_math::is_math_ac(el) {
            return;
        }
        if let Some(i) = first_alt_child_index(el) {
            if let Node::Element(child) = &el.children[i] {
                collect_visible_text(child, out);
            }
        }
        return;
    }
    if el.local_name() == "fld" {
        return;
    }
    match el.local_name() {
        "t" => out.push_str(&el.text_content()),
        "br" => out.push('\n'),
        _ => {}
    }
    for n in &el.children {
        if let Node::Element(e) = n {
            collect_visible_text(e, out);
        }
    }
}
/// 按 `docs/json-contract.md` 的替换规则原地改：第一个有文本 run 的段落收下全部新文本，
/// 其余段落清掉可见内容；清完后只剩 `a:pPr` / `a:endParaRPr` 的段删掉。
///
/// 换行必须在段落这一级处理。`a:br` 与 `a:r` 平级（`EG_TextRun`，ECMA-376 §A.4），
/// 而 `CT_RegularTextRun` 只允许一个可选 `a:rPr` 加恰好一个 `a:t`，
/// 所以既不能把 `a:br` 塞进 run 里，也不能让一个 run 带多个 `a:t`。
pub fn set_txbody_text(tx: &mut Element, text: &str) {
    let lines: Vec<&str> = text.split('\n').collect();
    let mut filled = false;
    rewrite_paras(tx, &mut filled, &lines);
    if !filled {
        append_text_para(tx, &lines);
    }
    prune_empty_paras(tx);
}

fn rewrite_paras(el: &mut Element, filled: &mut bool, lines: &[&str]) {
    if el.local_name() == "AlternateContent" {
        if let Some(i) = first_alt_child_index(el) {
            if let Node::Element(child) = &mut el.children[i] {
                rewrite_paras(child, filled, lines);
            }
        }
        return;
    }
    if el.local_name() == "p" {
        if !*filled && el.element_children().any(is_text_run) {
            fill_para(el, lines);
            *filled = true;
        } else {
            strip_para(el);
        }
        return;
    }
    for child in el.children.as_mut_slice() {
        if let Node::Element(e) = child {
            rewrite_paras(e, filled, lines);
        }
    }
}

fn is_text_run(e: &Element) -> bool {
    e.local_name() == "r" && e.element_children().any(|c| c.local_name() == "t")
}

/// 清掉本段的可见内容：`a:t` 和 `a:br` 都删，因此变空的 `a:r` 删。
/// `a:br` 一并删是因为 inspect 把它读成 `\n`，留一个下来读回值就和写入值差一个换行。
/// `a:pPr`、`a:endParaRPr`、`a:fld` 留着；随后由 `prune_empty_paras` 决定空段是否删除。
fn strip_para(p: &mut Element) {
    let mut i = p.children.len();
    while i > 0 {
        i -= 1;
        let Node::Element(e) = &mut p.children[i] else {
            continue;
        };
        match e.local_name() {
            "br" => {
                p.children.remove(i);
            }
            "r" => {
                remove_all_t(e);
                if is_empty_run(e) {
                    p.children.remove(i);
                }
            }
            _ => {}
        }
    }
}

/// 清完之后只剩 `a:pPr` / `a:endParaRPr` 的段删掉。
/// 含 `a:r`、`a:br`、`a:fld` 或其它不认识的子元素的段留下（未点名的域不清）。
/// `CT_TextBody` 至少留一段（§A.4，`minOccurs="1"`）；从后往前删，优先保住前面的写入段。
fn prune_empty_paras(el: &mut Element) {
    if el.local_name() == "AlternateContent" {
        if let Some(i) = first_alt_child_index(el) {
            if let Node::Element(child) = &mut el.children[i] {
                prune_empty_paras(child);
            }
        }
        return;
    }
    for child in el.children.as_mut_slice() {
        if let Node::Element(e) = child {
            if e.local_name() != "p" {
                prune_empty_paras(e);
            }
        }
    }
    let mut remaining = el
        .children
        .iter()
        .filter(|n| matches!(n, Node::Element(e) if e.local_name() == "p"))
        .count();
    if remaining <= 1 {
        return;
    }
    let mut i = el.children.len();
    while i > 0 {
        i -= 1;
        let Node::Element(e) = &el.children[i] else {
            continue;
        };
        if e.local_name() != "p" {
            continue;
        }
        if remaining <= 1 {
            break;
        }
        if is_prunable_para(e) {
            el.children.remove(i);
            remaining -= 1;
        }
    }
}

fn is_prunable_para(p: &Element) -> bool {
    !p.element_children()
        .any(|e| !matches!(e.local_name(), "pPr" | "endParaRPr"))
}

/// 第一行写进第一个文本 run 的 `a:t`，余下每行在段落里插一个 `a:br` 加一个新 `a:r`。
fn fill_para(p: &mut Element, lines: &[&str]) {
    // 先把除第一个文本 run 以外的可见内容清干净，免得回填时和旧内容混在一起。
    let mut seen = false;
    let mut i = 0;
    while i < p.children.len() {
        let Node::Element(e) = &mut p.children[i] else {
            i += 1;
            continue;
        };
        match e.local_name() {
            "r" if is_text_run(e) && !seen => {
                seen = true;
                i += 1;
            }
            "br" => {
                p.children.remove(i);
            }
            "r" => {
                remove_all_t(e);
                if is_empty_run(e) {
                    p.children.remove(i);
                } else {
                    i += 1;
                }
            }
            _ => i += 1,
        }
    }

    let Some(at) = p
        .children
        .iter()
        .position(|n| matches!(n, Node::Element(e) if is_text_run(e)))
    else {
        return;
    };
    let (rpr, t_name) = {
        let Node::Element(run) = &mut p.children[at] else {
            return;
        };
        let t_name = match run.element_children().find(|c| c.local_name() == "t") {
            Some(t) => t.name.clone(),
            None => return,
        };
        (fill_single_line(run, lines[0]), t_name)
    };

    let proto = Element::new(t_name.clone());
    let mut ins = at + 1;
    for line in &lines[1..] {
        p.children
            .insert(ins, Node::Element(Element::new(proto.prefixed("br"))));
        ins += 1;
        let mut run = Element::new(proto.prefixed("r"));
        if let Some(pr) = &rpr {
            run.push_element(pr.clone());
        }
        let mut t = Element::new(t_name.clone());
        apply_t_text(&mut t, line);
        run.push_element(t);
        p.children.insert(ins, Node::Element(run));
        ins += 1;
    }
}

/// 文本写进第一个 `a:t`，多余的 `a:t` 删掉。返回 `a:rPr` 的副本，给后续行的新 run 用。
fn fill_single_line(run: &mut Element, line: &str) -> Option<Element> {
    let mut seen = false;
    let mut drop = Vec::new();
    for (i, child) in run.children.iter_mut().enumerate() {
        if let Node::Element(e) = child {
            if e.local_name() == "t" {
                if seen {
                    drop.push(i);
                } else {
                    apply_t_text(e, line);
                    seen = true;
                }
            }
        }
    }
    for i in drop.into_iter().rev() {
        run.children.remove(i);
    }
    run.element_children()
        .find(|e| e.local_name() == "rPr")
        .cloned()
}

fn remove_all_t(r: &mut Element) {
    r.children.retain(|n| match n {
        Node::Element(e) => e.local_name() != "t",
        Node::Text(_) | Node::Raw(_) => true,
    });
}

fn is_empty_run(r: &Element) -> bool {
    !r.element_children().any(|e| e.local_name() != "rPr")
}

/// 一个文本 run 都没有时才走这里。已有段落就挂在第一段末尾，没有段落才新建一段，
/// 免得凭空多出一段。
fn append_text_para(tx: &mut Element, lines: &[&str]) {
    // txBody 自己是 `p:` 前缀，段落和 run 是 `a:` 前缀，前缀只能从里面的元素借。
    let sample = tx
        .element_children()
        .find(|e| e.local_name() == "p")
        .or_else(|| tx.element_children().next())
        .map(|e| e.name.clone())
        .unwrap_or_else(|| "a:bodyPr".to_string());
    let proto = Element::new(sample);
    let mut built: Vec<Element> = Vec::new();
    for (n, line) in lines.iter().enumerate() {
        if n > 0 {
            built.push(Element::new(proto.prefixed("br")));
        }
        let mut run = Element::new(proto.prefixed("r"));
        let mut t = Element::new(proto.prefixed("t"));
        apply_t_text(&mut t, line);
        run.push_element(t);
        built.push(run);
    }
    let existing = tx.children.iter_mut().find_map(|n| match n {
        Node::Element(e) if e.local_name() == "p" => Some(e),
        _ => None,
    });
    match existing {
        Some(p) => {
            // `a:endParaRPr` 必须在段末。空段常只有这一项，往后面塞 run 会被 PowerPoint 当成没字。
            let insert_at = p
                .children
                .iter()
                .position(|n| matches!(n, Node::Element(e) if e.local_name() == "endParaRPr"))
                .unwrap_or(p.children.len());
            for (k, el) in built.into_iter().enumerate() {
                p.children.insert(insert_at + k, Node::Element(el));
            }
        }
        None => {
            let mut p = Element::new(proto.prefixed("p"));
            for el in built {
                p.push_element(el);
            }
            tx.push_element(p);
        }
    }
}

fn apply_t_text(t: &mut Element, text: &str) {
    t.set_text(text.to_string());
    if needs_xml_space(text) {
        t.set_attr("xml:space", "preserve");
    } else {
        // `xml` 是 XML 规范的保留前缀，不能重新映射，所以精确名就够了。
        t.remove_attr("xml:space");
    }
}

fn needs_xml_space(text: &str) -> bool {
    text.starts_with(' ') || text.starts_with('\t') || text.ends_with(' ') || text.ends_with('\t')
}

/// 按可见文字把 `a:r` / `a:br` 切开。不改字，不删 `a:fld`，不合并段落。
pub fn set_txbody_runs(
    tx: &mut Element,
    pieces: &[crate::shape_runs::ShapeRunPiece],
    op_index: usize,
) -> Result<(), crate::error::Error> {
    let visible = txbody_visible_text(tx);
    let spans = crate::shape_runs::plan(&visible, pieces, op_index)?;
    let chars: Vec<char> = visible.chars().collect();
    let mut cursor = 0usize;
    rewrite_run_tree(tx, &mut cursor, &chars, &spans, pieces);
    Ok(())
}

fn rewrite_run_tree(
    el: &mut Element,
    cursor: &mut usize,
    chars: &[char],
    spans: &[crate::shape_runs::CharSpan],
    pieces: &[crate::shape_runs::ShapeRunPiece],
) {
    if el.local_name() == "AlternateContent" {
        if crate::drawing_math::is_math_ac(el) {
            return;
        }
        if let Some(i) = first_alt_child_index(el) {
            if let Node::Element(child) = &mut el.children[i] {
                rewrite_run_tree(child, cursor, chars, spans, pieces);
            }
        }
        return;
    }
    if el.local_name() == "fld" {
        return;
    }
    if el.local_name() == "p" {
        rewrite_para_runs(el, cursor, chars, spans, pieces);
        return;
    }
    for child in el.children.as_mut_slice() {
        if let Node::Element(e) = child {
            rewrite_run_tree(e, cursor, chars, spans, pieces);
        }
    }
}

fn rewrite_para_runs(
    p: &mut Element,
    cursor: &mut usize,
    chars: &[char],
    spans: &[crate::shape_runs::CharSpan],
    pieces: &[crate::shape_runs::ShapeRunPiece],
) {
    let mut i = 0usize;
    while i < p.children.len() {
        if !is_visible_text_node(&p.children[i]) {
            i += 1;
            continue;
        }
        let start = i;
        while i < p.children.len() && is_visible_text_node(&p.children[i]) {
            i += 1;
        }
        let end = i;
        let group_chars = p.children[start..end].iter().map(node_chars).sum::<usize>();
        let group_abs = *cursor;
        let segs = char_segs(&p.children[start..end]);
        let proto = match &p.children[start] {
            Node::Element(e) => Element::new(e.name.clone()),
            Node::Text(_) | Node::Raw(_) => Element::new("a:r"),
        };
        let nodes = emit_group(chars, group_abs, group_chars, &segs, spans, pieces, &proto);
        let new_len = nodes.len();
        for _ in start..end {
            p.children.remove(start);
        }
        for (k, node) in nodes.into_iter().enumerate() {
            p.children.insert(start + k, node);
        }
        *cursor += group_chars;
        i = start + new_len;
    }
}

fn is_visible_text_node(n: &Node) -> bool {
    let Node::Element(e) = n else {
        return false;
    };
    e.local_name() == "br" || is_text_run(e)
}

fn node_chars(n: &Node) -> usize {
    let Node::Element(e) = n else {
        return 0;
    };
    if e.local_name() == "br" {
        return 1;
    }
    e.element_children()
        .filter(|c| c.local_name() == "t")
        .map(|t| t.text_content().chars().count())
        .sum()
}

fn char_segs(nodes: &[Node]) -> Vec<(usize, Option<Element>)> {
    let mut acc = 0usize;
    let mut segs = Vec::new();
    for n in nodes {
        let Node::Element(e) = n else {
            continue;
        };
        acc += node_chars(n);
        segs.push((acc, e.child_by_local("rPr").cloned()));
    }
    segs
}

fn emit_group(
    chars: &[char],
    group_abs: usize,
    group_chars: usize,
    segs: &[(usize, Option<Element>)],
    spans: &[crate::shape_runs::CharSpan],
    pieces: &[crate::shape_runs::ShapeRunPiece],
    proto: &Element,
) -> Vec<Node> {
    let group_end = group_abs + group_chars;
    let mut out = Vec::new();
    for span in spans {
        let start = span.start.max(group_abs);
        let end = span.end.min(group_end);
        if start >= end {
            continue;
        }
        let format = span
            .piece
            .and_then(|idx| pieces.get(idx))
            .and_then(|piece| piece.format.as_ref());
        emit_chars(&mut out, chars, group_abs, start, end, segs, format, proto);
    }
    out
}

fn emit_chars(
    out: &mut Vec<Node>,
    chars: &[char],
    group_abs: usize,
    start: usize,
    end: usize,
    segs: &[(usize, Option<Element>)],
    format: Option<&crate::drawing_text::TextFormat>,
    proto: &Element,
) {
    let mut i = start;
    while i < end {
        let seg_i = seg_index(segs, i - group_abs);
        if chars[i] == '\n' {
            out.push(Node::Element(styled_br(
                proto,
                segs.get(seg_i).and_then(|seg| seg.1.clone()),
                format,
            )));
            i += 1;
            continue;
        }
        let mut j = i + 1;
        while j < end && chars[j] != '\n' && seg_index(segs, j - group_abs) == seg_i {
            j += 1;
        }
        let text: String = chars[i..j].iter().collect();
        let mut run = Element::new(proto.prefixed("r"));
        if let Some(rpr) = styled_rpr(proto, segs.get(seg_i).and_then(|s| s.1.clone()), format) {
            run.push_element(rpr);
        }
        let mut t = Element::new(proto.prefixed("t"));
        apply_t_text(&mut t, &text);
        run.push_element(t);
        out.push(Node::Element(run));
        i = j;
    }
}

fn seg_index(segs: &[(usize, Option<Element>)], local: usize) -> usize {
    segs.iter()
        .position(|(end, _)| local < *end)
        .unwrap_or(segs.len().saturating_sub(1))
}

fn styled_br(
    proto: &Element,
    base: Option<Element>,
    format: Option<&crate::drawing_text::TextFormat>,
) -> Element {
    let mut br = Element::new(proto.prefixed("br"));
    if let Some(rpr) = styled_rpr(proto, base, format) {
        br.children.insert(0, Node::Element(rpr));
    }
    br
}

fn styled_rpr(
    proto: &Element,
    base: Option<Element>,
    format: Option<&crate::drawing_text::TextFormat>,
) -> Option<Element> {
    let Some(fmt) = format.filter(|fmt| fmt.has_run_patch()) else {
        return base;
    };
    let mut rpr = base.unwrap_or_else(|| Element::new(proto.prefixed("rPr")));
    crate::drawing_text::patch_rpr(&mut rpr, fmt);
    Some(rpr)
}
