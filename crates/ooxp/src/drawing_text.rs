//! DrawingML 文本框共用：`a:rPr` / `a:pPr` / `a:bodyPr` 的 inspect 与打补丁。
//! 不算排版引擎。不写 theme 色，不改 `a:fld`。

use serde_json::{json, Map, Value};

use crate::error::{Error, ErrorKind, TypeName};
use crate::xml::{Element, Node};

pub const KEYS: &[&str] = &[
    "sz",
    "color",
    "bold",
    "italic",
    "underline",
    "font",
    "eastAsiaFont",
    "align",
    "textAnchor",
];

pub const SZ_MIN: i64 = 100;
pub const SZ_MAX: i64 = 400_000;

const FILL_LOCALS: &[&str] = &[
    "noFill",
    "solidFill",
    "gradFill",
    "blipFill",
    "pattFill",
    "grpFill",
];

const ALIGN_OK: &[&str] = &["l", "ctr", "r", "just"];
const ANCHOR_OK: &[&str] = &["t", "ctr", "b"];
const UNDERLINE_OK: &[&str] = &["sng", "none", "dbl"];

/// 写入补丁。外层 `Option` 表示未点名；内层 `None` 表示去掉显式值。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TextFormat {
    pub sz: Option<Option<i64>>,
    pub color: Option<Option<String>>,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub underline: Option<Option<String>>,
    pub font: Option<String>,
    pub east_asia_font: Option<String>,
    pub align: Option<Option<String>>,
    pub text_anchor: Option<Option<String>>,
}

impl TextFormat {
    pub fn has_run_patch(&self) -> bool {
        self.sz.is_some()
            || self.color.is_some()
            || self.bold.is_some()
            || self.italic.is_some()
            || self.underline.is_some()
            || self.font.is_some()
            || self.east_asia_font.is_some()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct RunView {
    sz: Option<i64>,
    color: Option<String>,
    bold: Option<bool>,
    italic: Option<bool>,
    underline: Option<String>,
    font: Option<String>,
    east_asia_font: Option<String>,
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
    let mut fmt = TextFormat::default();
    fmt.sz = parse_opt_sz(obj.get("sz"), &format!("{prefix}.sz"))?;
    fmt.color = parse_opt_hex(obj.get("color"), &format!("{prefix}.color"))?;
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
    Ok(Some(fmt))
}

/// 在 `text` 之后、`stable` 之前插入九个格式键。
pub fn insert_inspect(map: &mut Map<String, Value>, tx: Option<&Element>) {
    let view = summarize(tx);
    map.insert("sz".into(), opt_json_i64(view.sz));
    map.insert("color".into(), opt_json_str(view.color));
    map.insert("bold".into(), opt_json_bool(view.bold));
    map.insert("italic".into(), opt_json_bool(view.italic));
    map.insert("underline".into(), opt_json_str(view.underline));
    map.insert("font".into(), opt_json_str(view.font));
    map.insert("eastAsiaFont".into(), opt_json_str(view.east_asia));
    map.insert("align".into(), opt_json_str(view.align));
    map.insert("textAnchor".into(), opt_json_str(view.anchor));
}

struct InspectView {
    sz: Option<i64>,
    color: Option<String>,
    bold: Option<bool>,
    italic: Option<bool>,
    underline: Option<String>,
    font: Option<String>,
    east_asia: Option<String>,
    align: Option<String>,
    anchor: Option<String>,
}

fn summarize(tx: Option<&Element>) -> InspectView {
    let Some(tx) = tx else {
        return InspectView::empty();
    };
    let mut runs: Vec<RunView> = Vec::new();
    let mut aligns: Vec<Option<String>> = Vec::new();
    for_each_p(tx, &mut |p| {
        aligns.push(p_align(p));
        for_each_text_run(p, &mut |r| {
            runs.push(run_view(r.child_by_local("rPr")));
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
        bold: mixed_or_bool(|r| r.bold),
        italic: mixed_or_bool(|r| r.italic),
        underline: mixed_or(|r| r.underline.clone()),
        font: mixed_or(|r| r.font.clone()),
        east_asia: mixed_or(|r| r.east_asia_font.clone()),
        align,
        anchor: body_anchor(tx),
    }
}

impl InspectView {
    fn empty() -> Self {
        Self {
            sz: None,
            color: None,
            bold: None,
            italic: None,
            underline: None,
            font: None,
            east_asia: None,
            align: None,
            anchor: None,
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

fn parse_opt_hex(v: Option<&Value>, path: &str) -> Result<Option<Option<String>>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(None)),
        Some(Value::String(s)) => {
            if s.len() == 6 && s.chars().all(|c| c.is_ascii_hexdigit()) {
                Ok(Some(Some(s.clone())))
            } else {
                Err(Error::with_path(
                    ErrorKind::InvalidHexColor,
                    path.to_string(),
                ))
            }
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "color".into(),
                expected: TypeName::String,
            },
            path.to_string(),
        )),
    }
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

fn run_view(rpr: Option<&Element>) -> RunView {
    let Some(rpr) = rpr else {
        return RunView::default();
    };
    RunView {
        sz: attr_i64(rpr, "sz"),
        color: srgb_of(rpr),
        bold: on_off(rpr, "b"),
        italic: on_off(rpr, "i"),
        underline: rpr.attr_local("u").map(str::to_string),
        font: typeface_of(rpr, "latin"),
        east_asia_font: typeface_of(rpr, "ea"),
    }
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

fn patch_rpr(rpr: &mut Element, fmt: &TextFormat) {
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
    if let Some(color) = &fmt.color {
        remove_fills(rpr);
        if let Some(hex) = color {
            let mut fill = Element::new(rpr.prefixed("solidFill"));
            fill.push_element(
                Element::new(rpr.prefixed("srgbClr")).with_attr("val", hex.to_ascii_uppercase()),
            );
            rpr.children.insert(0, Node::Element(fill));
        }
    }
    if let Some(font) = &fmt.font {
        set_typeface(rpr, "latin", font);
    }
    if let Some(ea) = &fmt.east_asia_font {
        set_typeface(rpr, "ea", ea);
    }
}

fn remove_fills(rpr: &mut Element) {
    rpr.children.retain(|n| match n {
        Node::Element(e) => !FILL_LOCALS.contains(&e.local_name()),
        _ => true,
    });
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
    let mut fallback = None;
    for (i, n) in el.children.iter().enumerate() {
        if let Node::Element(e) = n {
            if e.local_name() == "Choice" {
                return Some(i);
            }
            if e.local_name() == "Fallback" && fallback.is_none() {
                fallback = Some(i);
            }
        }
    }
    fallback
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
