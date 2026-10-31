//! 段落 `w:pPr` 与 run `w:rPr` 上的直接格式：inspect 与 apply。

use serde_json::{json, Map, Value};

use crate::error::{Error, ErrorKind, IdReason};
use crate::xml::{Element, Node};

use super::hf;
use super::numbering;
use super::para::{self, ParaTarget};

pub const ALIGN_SET: &[&str] = &[
    "left",
    "center",
    "right",
    "both",
    "distribute",
    "start",
    "end",
];

pub const LINE_RULE_SET: &[&str] = &["auto", "exact", "atLeast"];

pub const UNDERLINE_SET: &[&str] = &[
    "single",
    "none",
    "double",
    "thick",
    "dotted",
    "dash",
    "wave",
    "words",
    "dottedHeavy",
    "dashedHeavy",
    "dashLong",
    "dashLongHeavy",
    "dotDash",
    "dashDotHeavy",
    "dotDotDash",
    "dashDotDotHeavy",
    "wavyHeavy",
    "wavyDouble",
];

pub const EM_SET: &[&str] = &["none", "dot", "comma", "circle", "underDot"];

pub const COMBINE_BRACKETS_SET: &[&str] = &["none", "round", "square", "angle", "curly"];

pub const TAB_VAL_SET: &[&str] = &["left", "center", "right", "decimal", "bar"];

pub const HIGHLIGHT_SET: &[&str] = &[
    "yellow",
    "green",
    "cyan",
    "magenta",
    "red",
    "blue",
    "black",
    "white",
    "darkBlue",
    "darkCyan",
    "darkGreen",
    "darkMagenta",
    "darkRed",
    "darkYellow",
    "darkGray",
    "lightGray",
    "none",
];

pub const VERT_ALIGN_SET: &[&str] = &["superscript", "subscript", "baseline"];

/// `CT_PPrBase` 里禁则开关与 spacing / ind / jc 的相对顺序。
const PPR_ORDER: &[&str] = &[
    "pStyle",
    "keepNext",
    "keepLines",
    "pageBreakBefore",
    "framePr",
    "widowControl",
    "numPr",
    "suppressLineNumbers",
    "pBdr",
    "shd",
    "tabs",
    "suppressAutoHyphens",
    "kinsoku",
    "wordWrap",
    "overflowPunct",
    "topLinePunct",
    "autoSpaceDE",
    "autoSpaceDN",
    "bidi",
    "adjustRightInd",
    "snapToGrid",
    "spacing",
    "ind",
    "contextualSpacing",
    "jc",
];

const PBDR_ORDER: &[&str] = &["top", "left", "bottom", "right", "between", "bar"];

/// `EG_RPrBase` 里 `em` / `eastAsianLayout` 相对 `cs` / `lang` 的位置。
const RPR_ORDER: &[&str] = &[
    "rStyle",
    "rFonts",
    "b",
    "bCs",
    "i",
    "iCs",
    "caps",
    "smallCaps",
    "strike",
    "dstrike",
    "outline",
    "shadow",
    "emboss",
    "imprint",
    "noProof",
    "snapToGrid",
    "vanish",
    "webHidden",
    "color",
    "spacing",
    "w",
    "kern",
    "position",
    "sz",
    "szCs",
    "highlight",
    "u",
    "effect",
    "bdr",
    "shd",
    "fitText",
    "vertAlign",
    "rtl",
    "cs",
    "em",
    "lang",
    "eastAsianLayout",
];

pub struct ParaFormatPatch<'a> {
    pub align: Option<Option<&'a str>>,
    pub left: Option<Option<u32>>,
    pub right: Option<Option<u32>>,
    pub first_line: Option<Option<u32>>,
    pub hanging: Option<Option<u32>>,
    pub before: Option<Option<u32>>,
    pub after: Option<Option<u32>>,
    pub line: Option<Option<u32>>,
    pub line_rule: Option<Option<&'a str>>,
    pub kinsoku: Option<Option<bool>>,
    pub overflow_punct: Option<Option<bool>>,
    pub top_line_punct: Option<Option<bool>>,
    pub auto_space_de: Option<Option<bool>>,
    pub auto_space_dn: Option<Option<bool>>,
    pub borders: Option<Option<&'a ParaBordersPatch>>,
    pub shd: Option<Option<&'a str>>,
    pub tabs: Option<Option<&'a [TabStop]>>,
}

#[derive(Debug, Clone)]
pub struct EastAsianLayout {
    pub id: Option<i64>,
    pub combine: Option<bool>,
    pub combine_brackets: Option<String>,
    pub vert: Option<bool>,
    pub vert_compress: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct ParaBordersPatch {
    pub top: Option<Option<super::page_bg::BorderSide>>,
    pub left: Option<Option<super::page_bg::BorderSide>>,
    pub bottom: Option<Option<super::page_bg::BorderSide>>,
    pub right: Option<Option<super::page_bg::BorderSide>>,
}

#[derive(Debug, Clone)]
pub struct TabStop {
    pub val: String,
    pub pos: u32,
}

pub struct RunFormatPatch<'a> {
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub underline: Option<Option<&'a str>>,
    pub sz: Option<u32>,
    pub color: Option<&'a str>,
    pub font: Option<&'a str>,
    pub east_asia_font: Option<&'a str>,
    pub em: Option<Option<&'a str>>,
    pub east_asian_layout: Option<Option<EastAsianLayout>>,
    pub highlight: Option<Option<&'a str>>,
    pub strike: Option<Option<bool>>,
    pub dstrike: Option<Option<bool>>,
    pub vert_align: Option<Option<&'a str>>,
    pub spacing: Option<Option<i32>>,
    pub kern: Option<Option<u32>>,
    pub scale: Option<Option<u32>>,
}

pub fn para_node(
    p: &Element,
    index: usize,
    container: Option<&str>,
    picture_count: Option<usize>,
    text_box_count: Option<usize>,
    connector_count: Option<usize>,
) -> Value {
    let (id, stable) = match container {
        Some(c) => para::para_stable_id_in(c, p, index),
        None => para::para_stable_id(p, index),
    };
    let style = para::p_style(p);
    let text = para::visible_text(p);
    let mut node = if let Some(level) = para::heading_level(p) {
        json!({
            "id": id,
            "index": index,
            "style": style,
            "level": level,
            "text": text,
            "stable": stable,
        })
    } else {
        json!({
            "id": id,
            "index": index,
            "style": style,
            "text": text,
            "stable": stable,
        })
    };
    if let Some(obj) = node.as_object_mut() {
        numbering::stamp_para(obj, p);
        stamp_ppr(obj, p);
        if let Some(n) = picture_count {
            obj.insert("pictureCount".into(), json!(n));
        }
        if let Some(n) = text_box_count {
            obj.insert("textBoxCount".into(), json!(n));
        }
        if let Some(n) = connector_count {
            obj.insert("connectorCount".into(), json!(n));
        }
        stamp_run_count(obj, p);
    }
    node
}

pub fn maybe_runs(node: &mut Value, p: &Element, expand: bool) {
    if !expand {
        return;
    }
    let Some(obj) = node.as_object_mut() else {
        return;
    };
    let id = obj
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    obj.insert("runs".into(), Value::Array(runs_json(p, &id)));
}

pub fn parse_run_id(id: &str, path: &str) -> Result<(ParaTarget, usize), Error> {
    let Some((para_id, n_s)) = id.rsplit_once("/run:") else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SetRunFormatNeedsRun,
            },
            path.to_string(),
        ));
    };
    if n_s.is_empty() {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::RunPrefix,
            },
            path.to_string(),
        ));
    }
    let n: usize = n_s.parse().map_err(|_| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::RunIndexNotInt,
            },
            path.to_string(),
        )
    })?;
    if n < 1 {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::RunIndexFromOne,
            },
            path.to_string(),
        ));
    }
    let para = para::parse_para_id_at(para_id, path)?;
    Ok((para, n))
}

pub fn missing_run(id: &str, i: usize) -> Error {
    Error::with_path(
        ErrorKind::RunNotFound { id: id.to_string() },
        format!("ops[{i}].id"),
    )
}

pub fn apply_para(p: &mut Element, patch: ParaFormatPatch<'_>) {
    if patch.align.is_some()
        || patch.left.is_some()
        || patch.right.is_some()
        || patch.first_line.is_some()
        || patch.hanging.is_some()
        || patch.before.is_some()
        || patch.after.is_some()
        || patch.line.is_some()
        || patch.line_rule.is_some()
        || patch.kinsoku.is_some()
        || patch.overflow_punct.is_some()
        || patch.top_line_punct.is_some()
        || patch.auto_space_de.is_some()
        || patch.auto_space_dn.is_some()
        || patch.borders.is_some()
        || patch.shd.is_some()
        || patch.tabs.is_some()
    {
        ensure_ppr(p);
    }
    let Some(ppr) = p.child_by_local_mut("pPr") else {
        return;
    };
    if let Some(align) = patch.align {
        match align {
            Some(v) => {
                ensure_child(ppr, "jc");
                let jc = ppr.child_by_local_mut("jc").expect("刚写入 jc");
                let key = jc.prefixed("val");
                jc.set_attr(&key, v);
            }
            None => remove_child(ppr, "jc"),
        }
    }
    if patch.left.is_some()
        || patch.right.is_some()
        || patch.first_line.is_some()
        || patch.hanging.is_some()
    {
        apply_ind(ppr, &patch);
    }
    if patch.before.is_some()
        || patch.after.is_some()
        || patch.line.is_some()
        || patch.line_rule.is_some()
    {
        apply_spacing(ppr, &patch);
    }
    apply_on_off(ppr, "kinsoku", patch.kinsoku);
    apply_on_off(ppr, "overflowPunct", patch.overflow_punct);
    apply_on_off(ppr, "topLinePunct", patch.top_line_punct);
    apply_on_off(ppr, "autoSpaceDE", patch.auto_space_de);
    apply_on_off(ppr, "autoSpaceDN", patch.auto_space_dn);
    apply_p_borders(ppr, patch.borders);
    apply_shd(ppr, patch.shd);
    apply_tabs(ppr, patch.tabs);
}

pub fn bind_run_style(r: &mut Element, style_id: &str) {
    ensure_rpr(r);
    let rpr = r.child_by_local_mut("rPr").expect("刚写入 rPr");
    let el = hf::ensure_child_in_order(rpr, "rStyle", RPR_ORDER);
    let key = el.prefixed("val");
    el.set_attr(&key, style_id);
}

pub fn unbind_run_style(r: &mut Element) {
    let Some(rpr) = r.child_by_local_mut("rPr") else {
        return;
    };
    remove_child(rpr, "rStyle");
}

pub fn apply_run(r: &mut Element, patch: RunFormatPatch<'_>) {
    ensure_rpr(r);
    let rpr = r.child_by_local_mut("rPr").expect("刚写入 rPr");
    if let Some(b) = patch.bold {
        set_on_off(rpr, "b", b);
        set_on_off(rpr, "bCs", b);
    }
    if let Some(i) = patch.italic {
        set_on_off(rpr, "i", i);
        set_on_off(rpr, "iCs", i);
    }
    if let Some(u) = patch.underline {
        match u {
            Some(v) => {
                ensure_child(rpr, "u");
                let el = rpr.child_by_local_mut("u").expect("刚写入 u");
                let key = el.prefixed("val");
                el.set_attr(&key, v);
            }
            None => remove_child(rpr, "u"),
        }
    }
    if let Some(sz) = patch.sz {
        for local in ["sz", "szCs"] {
            ensure_child(rpr, local);
            let el = rpr.child_by_local_mut(local).expect("刚写入 sz");
            let key = el.prefixed("val");
            el.set_attr(&key, sz.to_string());
        }
    }
    if let Some(color) = patch.color {
        ensure_child(rpr, "color");
        let el = rpr.child_by_local_mut("color").expect("刚写入 color");
        el.attrs.retain(|(k, _)| {
            let local = k.rsplit(':').next().unwrap_or(k);
            !matches!(local, "themeColor" | "themeTint" | "themeShade" | "theme")
        });
        let key = el.prefixed("val");
        el.set_attr(&key, color.to_ascii_uppercase());
    }
    if patch.font.is_some() || patch.east_asia_font.is_some() {
        ensure_child(rpr, "rFonts");
        let fonts = rpr.child_by_local_mut("rFonts").expect("刚写入 rFonts");
        if let Some(name) = patch.font {
            let ascii = fonts.prefixed("ascii");
            let h_ansi = fonts.prefixed("hAnsi");
            let cs = fonts.prefixed("cs");
            fonts.set_attr(&ascii, name);
            fonts.set_attr(&h_ansi, name);
            fonts.set_attr(&cs, name);
        }
        if let Some(name) = patch.east_asia_font {
            let key = fonts.prefixed("eastAsia");
            fonts.set_attr(&key, name);
        }
    }
    if let Some(em) = patch.em {
        match em {
            Some(v) => {
                let el = hf::ensure_child_in_order(rpr, "em", RPR_ORDER);
                let key = el.prefixed("val");
                el.set_attr(&key, v);
            }
            None => remove_child(rpr, "em"),
        }
    }
    if let Some(layout) = patch.east_asian_layout {
        match layout {
            Some(spec) => write_east_asian_layout(rpr, &spec),
            None => remove_child(rpr, "eastAsianLayout"),
        }
    }
    apply_val_elem(rpr, "highlight", patch.highlight);
    apply_on_off_rpr(rpr, "strike", patch.strike);
    apply_on_off_rpr(rpr, "dstrike", patch.dstrike);
    apply_val_elem(rpr, "vertAlign", patch.vert_align);
    apply_int_val(rpr, "spacing", patch.spacing.map(|v| v.map(i64::from)));
    apply_int_val(rpr, "kern", patch.kern.map(|v| v.map(i64::from)));
    apply_int_val(rpr, "w", patch.scale.map(|v| v.map(i64::from)));
}

pub fn find_run_mut(p: &mut Element, n: usize) -> Option<&mut Element> {
    let mut idx = 0usize;
    find_run_in(p, &mut idx, n)
}

pub(crate) fn pin_runs_in_paragraph(p: &mut Element) {
    let mut n = 0usize;
    pin_runs_walk(p, &mut n);
}

fn pin_runs_walk(el: &mut Element, n: &mut usize) {
    if el.local_name() == "txbxContent" || el.local_name() == "del" {
        return;
    }
    if el.local_name() == "AlternateContent" {
        if let Some(i) = para::first_alt_child_index(el) {
            if let Node::Element(child) = &mut el.children[i] {
                pin_runs_walk(child, n);
            }
        }
        return;
    }
    if el.local_name() == "r" {
        *n += 1;
        let attr = crate::xml::pin_attr("run");
        if el.attr(&attr).is_none() {
            el.set_attr(&attr, n.to_string());
        }
        return;
    }
    let len = el.children.len();
    for i in 0..len {
        if let Node::Element(child) = &mut el.children[i] {
            pin_runs_walk(child, n);
        }
    }
}

fn stamp_ppr(node: &mut Map<String, Value>, p: &Element) {
    let ppr = p.child_by_local("pPr");
    let align = ppr
        .and_then(|e| e.child_by_local("jc"))
        .and_then(|e| e.attr_local("val"))
        .map(str::to_string);
    let indent = ppr.and_then(|e| e.child_by_local("ind")).map(indent_json);
    let spacing = ppr
        .and_then(|e| e.child_by_local("spacing"))
        .map(spacing_json);
    insert_before(
        node,
        "text",
        &[
            ("align", json!(align)),
            ("indent", json!(indent)),
            ("spacing", json!(spacing)),
            ("eastAsia", east_asia_json(ppr)),
            ("borders", ppr.map(borders_json).unwrap_or(Value::Null)),
            ("shd", ppr.map(shd_json).unwrap_or(Value::Null)),
            ("tabs", ppr.map(tabs_json).unwrap_or(Value::Null)),
        ],
    );
}

fn stamp_run_count(node: &mut Map<String, Value>, p: &Element) {
    let n = collect_runs(p).len();
    let after = if node.contains_key("connectorCount") {
        "connectorCount"
    } else if node.contains_key("textBoxCount") {
        "textBoxCount"
    } else if node.contains_key("pictureCount") {
        "pictureCount"
    } else {
        "stable"
    };
    insert_after(node, after, "runCount".into(), json!(n));
}

fn runs_json(p: &Element, para_id: &str) -> Vec<Value> {
    collect_runs(p)
        .into_iter()
        .enumerate()
        .map(|(i, r)| {
            let n = i + 1;
            let rpr = r.child_by_local("rPr");
            json!({
                "id": format!("{para_id}/run:{n}"),
                "index": n,
                "style": rpr
                    .and_then(|e| e.child_by_local("rStyle"))
                    .and_then(|e| e.attr_local("val"))
                    .map(str::to_string),
                "text": run_visible_text(r),
                "bold": rpr.and_then(|e| on_off_of(e, "b")),
                "italic": rpr.and_then(|e| on_off_of(e, "i")),
                "underline": rpr.and_then(underline_of),
                "sz": rpr.and_then(|e| int_val(e, "sz")),
                "color": rpr
                    .and_then(|e| e.child_by_local("color"))
                    .and_then(|e| e.attr_local("val"))
                    .map(str::to_string),
                "font": rpr.and_then(latin_font),
                "eastAsiaFont": rpr
                    .and_then(|e| e.child_by_local("rFonts"))
                    .and_then(|e| e.attr_local("eastAsia"))
                    .map(str::to_string),
                "em": rpr
                    .and_then(|e| e.child_by_local("em"))
                    .and_then(|e| e.attr_local("val"))
                    .map(str::to_string),
                "eastAsianLayout": rpr.map(east_asian_layout_json).unwrap_or(Value::Null),
                "highlight": rpr
                    .and_then(|e| e.child_by_local("highlight"))
                    .and_then(|e| e.attr_local("val"))
                    .map(str::to_string),
                "strike": rpr.and_then(|e| on_off_of(e, "strike")),
                "dstrike": rpr.and_then(|e| on_off_of(e, "dstrike")),
                "vertAlign": rpr
                    .and_then(|e| e.child_by_local("vertAlign"))
                    .and_then(|e| e.attr_local("val"))
                    .map(str::to_string),
                "spacing": rpr.and_then(|e| signed_int_val(e, "spacing")),
                "kern": rpr.and_then(|e| int_val(e, "kern")),
                "scale": rpr.and_then(scale_val),
            })
        })
        .collect()
}

fn collect_runs(p: &Element) -> Vec<&Element> {
    let mut out = Vec::new();
    walk_runs(p, false, &mut out);
    out
}

fn walk_runs<'a>(root: &'a Element, in_del: bool, out: &mut Vec<&'a Element>) {
    let mut stack = vec![(root, in_del)];
    while let Some((el, in_del)) = stack.pop() {
        if el.local_name() == "txbxContent" {
            continue;
        }
        if el.local_name() == "AlternateContent" {
            if let Some(i) = para::first_alt_child_index(el) {
                if let Node::Element(child) = &el.children[i] {
                    stack.push((child, in_del));
                }
            }
            continue;
        }
        let in_del = in_del || el.local_name() == "del";
        if el.local_name() == "r" {
            if !in_del {
                out.push(el);
            }
            continue;
        }
        for node in el.children.iter().rev() {
            if let Node::Element(child) = node {
                stack.push((child, in_del));
            }
        }
    }
}

fn find_run_in<'a>(el: &'a mut Element, idx: &mut usize, n: usize) -> Option<&'a mut Element> {
    crate::xml::search_mut(el, |el| {
        if el.local_name() == "txbxContent" || el.local_name() == "del" {
            return crate::xml::SearchMut::Skip;
        }
        if el.local_name() == "AlternateContent" {
            if let Some(i) = para::first_alt_child_index(el) {
                if matches!(&el.children[i], Node::Element(_)) {
                    return crate::xml::SearchMut::Only(i);
                }
            }
            return crate::xml::SearchMut::Skip;
        }
        if el.local_name() == "r" {
            *idx += 1;
            if crate::xml::pin_matches(el, &crate::xml::pin_attr("run"), n, *idx) {
                return crate::xml::SearchMut::Hit;
            }
            return crate::xml::SearchMut::Skip;
        }
        crate::xml::SearchMut::down(el)
    })
}

fn run_visible_text(r: &Element) -> String {
    let mut out = String::new();
    walk_run_text(r, false, &mut out);
    out
}

fn walk_run_text(root: &Element, in_del: bool, out: &mut String) {
    let mut stack = vec![(root, in_del)];
    while let Some((el, in_del)) = stack.pop() {
        if el.local_name() == "instrText" {
            continue;
        }
        let in_del = in_del || el.local_name() == "del";
        if !in_del {
            match el.local_name() {
                "t" => out.push_str(&el.text_content()),
                "tab" => out.push('\t'),
                "br" => out.push('\n'),
                _ => {}
            }
        }
        for node in el.children.iter().rev() {
            if let Node::Element(child) = node {
                stack.push((child, in_del));
            }
        }
    }
}

fn indent_json(ind: &Element) -> Value {
    json!({
        "left": int_attr(ind, "left").or_else(|| int_attr(ind, "start")),
        "right": int_attr(ind, "right").or_else(|| int_attr(ind, "end")),
        "firstLine": int_attr(ind, "firstLine"),
        "hanging": int_attr(ind, "hanging"),
    })
}

fn spacing_json(sp: &Element) -> Value {
    json!({
        "before": int_attr(sp, "before"),
        "after": int_attr(sp, "after"),
        "line": int_attr(sp, "line"),
        "lineRule": sp.attr_local("lineRule"),
    })
}

fn borders_json(ppr: &Element) -> Value {
    let Some(el) = ppr.child_by_local("pBdr") else {
        return Value::Null;
    };
    json!({
        "top": super::page_bg::side_json(el, "top"),
        "left": super::page_bg::side_json(el, "left"),
        "bottom": super::page_bg::side_json(el, "bottom"),
        "right": super::page_bg::side_json(el, "right"),
    })
}

fn shd_json(ppr: &Element) -> Value {
    let Some(el) = ppr.child_by_local("shd") else {
        return Value::Null;
    };
    json!({
        "fill": el.attr_local("fill"),
    })
}

fn tabs_json(ppr: &Element) -> Value {
    let Some(el) = ppr.child_by_local("tabs") else {
        return Value::Null;
    };
    let mut out = Vec::new();
    for n in &el.children {
        let Node::Element(tab) = n else {
            continue;
        };
        if tab.local_name() != "tab" {
            continue;
        }
        out.push(json!({
            "val": tab.attr_local("val"),
            "pos": int_attr(tab, "pos"),
        }));
    }
    if out.is_empty() {
        Value::Null
    } else {
        Value::Array(out)
    }
}

fn east_asia_json(ppr: Option<&Element>) -> Value {
    let Some(ppr) = ppr else {
        return Value::Null;
    };
    let names = [
        "kinsoku",
        "overflowPunct",
        "topLinePunct",
        "autoSpaceDE",
        "autoSpaceDN",
    ];
    if names.iter().all(|n| ppr.child_by_local(n).is_none()) {
        return Value::Null;
    }
    json!({
        "kinsoku": on_off_json(ppr, "kinsoku"),
        "overflowPunct": on_off_json(ppr, "overflowPunct"),
        "topLinePunct": on_off_json(ppr, "topLinePunct"),
        "autoSpaceDE": on_off_json(ppr, "autoSpaceDE"),
        "autoSpaceDN": on_off_json(ppr, "autoSpaceDN"),
    })
}

fn on_off_json(ppr: &Element, local: &str) -> Value {
    match ppr.child_by_local(local) {
        Some(e) => json!(on_off(e)),
        None => Value::Null,
    }
}

fn east_asian_layout_json(rpr: &Element) -> Value {
    let Some(el) = rpr.child_by_local("eastAsianLayout") else {
        return Value::Null;
    };
    json!({
        "id": el.attr_local("id").and_then(|s| s.parse::<i64>().ok()),
        "combine": attr_on_off(el, "combine"),
        "combineBrackets": el.attr_local("combineBrackets"),
        "vert": attr_on_off(el, "vert"),
        "vertCompress": attr_on_off(el, "vertCompress"),
    })
}

fn attr_on_off(el: &Element, local: &str) -> Value {
    match el.attr_local(local) {
        None => Value::Null,
        Some(v) => {
            let lower = v.to_ascii_lowercase();
            match lower.as_str() {
                "1" | "true" | "on" => json!(true),
                "0" | "false" | "off" => json!(false),
                _ => Value::Null,
            }
        }
    }
}

fn int_attr(el: &Element, local: &str) -> Option<u32> {
    el.attr_local(local).and_then(|s| s.parse().ok())
}

fn int_val(parent: &Element, local: &str) -> Option<u32> {
    parent
        .child_by_local(local)
        .and_then(|e| e.attr_local("val"))
        .and_then(|s| s.parse().ok())
}

fn scale_val(rpr: &Element) -> Option<u32> {
    let raw = rpr.child_by_local("w")?.attr_local("val")?;
    let s = raw.trim();
    let s = s.strip_suffix('%').unwrap_or(s);
    s.parse().ok()
}

fn signed_int_val(parent: &Element, local: &str) -> Option<i32> {
    parent
        .child_by_local(local)
        .and_then(|e| e.attr_local("val"))
        .and_then(|s| s.parse().ok())
}

fn on_off_of(rpr: &Element, local: &str) -> Option<bool> {
    rpr.child_by_local(local).map(on_off)
}

fn on_off(el: &Element) -> bool {
    !matches!(el.attr_local("val").unwrap_or(""), "0" | "false" | "off")
}

fn underline_of(rpr: &Element) -> Option<String> {
    let u = rpr.child_by_local("u")?;
    Some(
        u.attr_local("val")
            .filter(|s| !s.is_empty())
            .unwrap_or("single")
            .to_string(),
    )
}

fn latin_font(rpr: &Element) -> Option<String> {
    let fonts = rpr.child_by_local("rFonts")?;
    fonts
        .attr_local("ascii")
        .or_else(|| fonts.attr_local("hAnsi"))
        .map(str::to_string)
}

fn apply_ind(ppr: &mut Element, patch: &ParaFormatPatch<'_>) {
    ensure_child(ppr, "ind");
    {
        let ind = ppr.child_by_local_mut("ind").expect("刚写入 ind");
        if let Some(left) = patch.left {
            set_or_clear(ind, "left", left);
            clear_attr(ind, "start");
        }
        if let Some(right) = patch.right {
            set_or_clear(ind, "right", right);
            clear_attr(ind, "end");
        }
        if let Some(first) = patch.first_line {
            set_or_clear(ind, "firstLine", first);
            if first.is_some() {
                clear_attr(ind, "hanging");
            }
        }
        if let Some(hang) = patch.hanging {
            set_or_clear(ind, "hanging", hang);
            if hang.is_some() {
                clear_attr(ind, "firstLine");
            }
        }
    }
    let empty = ppr
        .child_by_local("ind")
        .is_some_and(|e| e.attrs.is_empty());
    if empty {
        remove_child(ppr, "ind");
    }
}

fn apply_spacing(ppr: &mut Element, patch: &ParaFormatPatch<'_>) {
    ensure_child(ppr, "spacing");
    {
        let sp = ppr.child_by_local_mut("spacing").expect("刚写入 spacing");
        if let Some(v) = patch.before {
            set_or_clear(sp, "before", v);
        }
        if let Some(v) = patch.after {
            set_or_clear(sp, "after", v);
        }
        if let Some(v) = patch.line {
            set_or_clear(sp, "line", v);
        }
        if let Some(rule) = patch.line_rule {
            match rule {
                Some(s) => {
                    let key = sp.prefixed("lineRule");
                    sp.set_attr(&key, s);
                }
                None => clear_attr(sp, "lineRule"),
            }
        }
    }
    let empty = ppr
        .child_by_local("spacing")
        .is_some_and(|e| e.attrs.is_empty());
    if empty {
        remove_child(ppr, "spacing");
    }
}

fn set_or_clear(el: &mut Element, local: &str, val: Option<u32>) {
    match val {
        Some(n) => {
            let key = el.prefixed(local);
            el.set_attr(&key, n.to_string());
        }
        None => clear_attr(el, local),
    }
}

fn clear_attr(el: &mut Element, local: &str) {
    let key = el.prefixed(local);
    el.remove_attr(&key);
    el.attrs
        .retain(|(k, _)| k.rsplit(':').next().unwrap_or(k) != local);
}

fn set_on_off(rpr: &mut Element, local: &str, on: bool) {
    ensure_child(rpr, local);
    let el = rpr.child_by_local_mut(local).expect("刚写入开关");
    let key = el.prefixed("val");
    if on {
        el.remove_attr(&key);
    } else {
        el.set_attr(&key, "0");
    }
}

fn apply_on_off(ppr: &mut Element, local: &str, val: Option<Option<bool>>) {
    match val {
        None => {}
        Some(None) => remove_child(ppr, local),
        Some(Some(on)) => {
            let el = hf::ensure_child_in_order(ppr, local, PPR_ORDER);
            let key = el.prefixed("val");
            if on {
                el.remove_attr(&key);
            } else {
                el.set_attr(&key, "0");
            }
        }
    }
}

fn apply_p_borders(ppr: &mut Element, patch: Option<Option<&ParaBordersPatch>>) {
    match patch {
        None => {}
        Some(None) => remove_child(ppr, "pBdr"),
        Some(Some(sides)) => {
            {
                let el = hf::ensure_child_in_order(ppr, "pBdr", PPR_ORDER);
                write_p_side(el, "top", sides.top.as_ref());
                write_p_side(el, "left", sides.left.as_ref());
                write_p_side(el, "bottom", sides.bottom.as_ref());
                write_p_side(el, "right", sides.right.as_ref());
            }
            let empty = ppr
                .child_by_local("pBdr")
                .is_some_and(|e| e.element_children().next().is_none());
            if empty {
                remove_child(ppr, "pBdr");
            }
        }
    }
}

fn write_p_side(
    parent: &mut Element,
    local: &str,
    slot: Option<&Option<super::page_bg::BorderSide>>,
) {
    let Some(slot) = slot else {
        return;
    };
    match slot {
        None => remove_child(parent, local),
        Some(b) => {
            remove_child(parent, local);
            let el = hf::ensure_child_in_order(parent, local, PBDR_ORDER);
            hf::set_local_attr(el, "val", &b.val);
            if let Some(sz) = b.sz {
                hf::set_local_attr(el, "sz", sz.to_string());
            }
            if let Some(space) = b.space {
                hf::set_local_attr(el, "space", space.to_string());
            }
            if let Some(color) = &b.color {
                hf::set_local_attr(el, "color", color);
            }
        }
    }
}

fn apply_shd(ppr: &mut Element, patch: Option<Option<&str>>) {
    match patch {
        None => {}
        Some(None) => remove_child(ppr, "shd"),
        Some(Some(fill)) => {
            let el = hf::ensure_child_in_order(ppr, "shd", PPR_ORDER);
            el.attrs.retain(|(k, _)| {
                let local = k.rsplit(':').next().unwrap_or(k);
                !matches!(
                    local,
                    "themeFill"
                        | "themeFillTint"
                        | "themeFillShade"
                        | "themeColor"
                        | "themeTint"
                        | "themeShade"
                )
            });
            hf::set_local_attr(el, "val", "clear");
            hf::set_local_attr(el, "color", "auto");
            hf::set_local_attr(el, "fill", fill.to_ascii_uppercase());
        }
    }
}

fn apply_tabs(ppr: &mut Element, patch: Option<Option<&[TabStop]>>) {
    match patch {
        None => {}
        Some(None) => remove_child(ppr, "tabs"),
        Some(Some([])) => remove_child(ppr, "tabs"),
        Some(Some(tabs)) => {
            remove_child(ppr, "tabs");
            let el = hf::ensure_child_in_order(ppr, "tabs", PPR_ORDER);
            for tab in tabs {
                let mut child = Element::new(el.prefixed("tab"));
                hf::set_local_attr(&mut child, "val", &tab.val);
                hf::set_local_attr(&mut child, "pos", tab.pos.to_string());
                el.push_element(child);
            }
        }
    }
}

fn apply_val_elem(rpr: &mut Element, local: &str, patch: Option<Option<&str>>) {
    match patch {
        None => {}
        Some(None) => remove_child(rpr, local),
        Some(Some(v)) => {
            let el = hf::ensure_child_in_order(rpr, local, RPR_ORDER);
            let key = el.prefixed("val");
            el.set_attr(&key, v);
        }
    }
}

fn apply_int_val(rpr: &mut Element, local: &str, patch: Option<Option<i64>>) {
    match patch {
        None => {}
        Some(None) => remove_child(rpr, local),
        Some(Some(v)) => {
            let el = hf::ensure_child_in_order(rpr, local, RPR_ORDER);
            let key = el.prefixed("val");
            el.set_attr(&key, v.to_string());
        }
    }
}

fn apply_on_off_rpr(rpr: &mut Element, local: &str, patch: Option<Option<bool>>) {
    match patch {
        None => {}
        Some(None) => remove_child(rpr, local),
        Some(Some(on)) => {
            let el = hf::ensure_child_in_order(rpr, local, RPR_ORDER);
            let key = el.prefixed("val");
            if on {
                el.remove_attr(&key);
            } else {
                el.set_attr(&key, "0");
            }
        }
    }
}

fn write_east_asian_layout(rpr: &mut Element, spec: &EastAsianLayout) {
    remove_child(rpr, "eastAsianLayout");
    let el = hf::ensure_child_in_order(rpr, "eastAsianLayout", RPR_ORDER);
    if let Some(id) = spec.id {
        let key = el.prefixed("id");
        el.set_attr(&key, id.to_string());
    }
    if let Some(on) = spec.combine {
        set_attr_on_off(el, "combine", on);
    }
    if let Some(ref b) = spec.combine_brackets {
        let key = el.prefixed("combineBrackets");
        el.set_attr(&key, b);
    }
    if let Some(on) = spec.vert {
        set_attr_on_off(el, "vert", on);
    }
    if let Some(on) = spec.vert_compress {
        set_attr_on_off(el, "vertCompress", on);
    }
}

fn set_attr_on_off(el: &mut Element, local: &str, on: bool) {
    let key = el.prefixed(local);
    el.set_attr(&key, if on { "1" } else { "0" });
}

fn ensure_ppr(p: &mut Element) {
    if p.child_by_local("pPr").is_some() {
        return;
    }
    let ppr = Element::new(p.prefixed("pPr"));
    p.children.insert(0, Node::Element(ppr));
}

fn ensure_rpr(r: &mut Element) {
    if r.child_by_local("rPr").is_some() {
        return;
    }
    let rpr = Element::new(r.prefixed("rPr"));
    r.children.insert(0, Node::Element(rpr));
}

fn ensure_child(parent: &mut Element, local: &str) {
    if parent.child_by_local(local).is_some() {
        return;
    }
    parent.push_element(Element::new(parent.prefixed(local)));
}

fn remove_child(parent: &mut Element, local: &str) {
    parent
        .children
        .retain(|n| !matches!(n, Node::Element(e) if e.local_name() == local));
}

fn insert_before(map: &mut Map<String, Value>, before: &str, pairs: &[(&str, Value)]) {
    let mut out = Map::new();
    let mut done = false;
    for (k, v) in map.iter() {
        if k == before {
            for (nk, nv) in pairs {
                out.insert((*nk).into(), nv.clone());
            }
            done = true;
        }
        out.insert(k.clone(), v.clone());
    }
    if !done {
        for (nk, nv) in pairs {
            out.insert((*nk).into(), nv.clone());
        }
    }
    *map = out;
}

fn insert_after(map: &mut Map<String, Value>, after: &str, key: String, val: Value) {
    if !map.contains_key(after) {
        map.insert(key, val);
        return;
    }
    let mut out = Map::new();
    for (k, v) in map.iter() {
        out.insert(k.clone(), v.clone());
        if k == after {
            out.insert(key.clone(), val.clone());
        }
    }
    *map = out;
}

use super::request::SetParaFormatOp;
use super::request::SetRunFormatOp;
use crate::opc::Package;
use crate::xml::Document;
use std::collections::HashMap;
// 从 apply 搬来的操作体。
pub(crate) fn apply_set_para_format(
    pkg: &Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &SetParaFormatOp,
    i: usize,
) -> Result<(), Error> {
    let part_name =
        super::apply::part_for_format_para(pkg, catalog, doc_part, &op.para, &op.id, i)?;
    let doc = super::apply::load_part(pkg, docs, &part_name)?;
    let el = super::apply::find_insert_para_mut(&mut doc.root, &op.para).ok_or_else(|| {
        Error::with_path(
            ErrorKind::ParaNotFound { id: op.id.clone() },
            format!("ops[{i}].id"),
        )
    })?;
    super::format::apply_para(
        el,
        super::format::ParaFormatPatch {
            align: op.align.as_ref().map(|v| v.as_deref()),
            left: op.left,
            right: op.right,
            first_line: op.first_line,
            hanging: op.hanging,
            before: op.before,
            after: op.after,
            line: op.line,
            line_rule: op.line_rule.as_ref().map(|v| v.as_deref()),
            kinsoku: op.kinsoku,
            overflow_punct: op.overflow_punct,
            top_line_punct: op.top_line_punct,
            auto_space_de: op.auto_space_de,
            auto_space_dn: op.auto_space_dn,
            borders: op.borders.as_ref().map(|v| v.as_ref()),
            shd: op.shd.as_ref().map(|v| v.as_deref()),
            tabs: op.tabs.as_ref().map(|v| v.as_deref()),
        },
    );
    dirty.insert(part_name, ());
    Ok(())
}
pub(crate) fn apply_set_run_format(
    pkg: &Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[hf::HfPart],
    op: &SetRunFormatOp,
    i: usize,
) -> Result<(), Error> {
    let part_name =
        super::apply::part_for_format_para(pkg, catalog, doc_part, &op.para, &op.id, i)?;
    let doc = super::apply::load_part(pkg, docs, &part_name)?;
    let el = super::apply::find_insert_para_mut(&mut doc.root, &op.para).ok_or_else(|| {
        Error::with_path(
            ErrorKind::ParaNotFound { id: op.id.clone() },
            format!("ops[{i}].id"),
        )
    })?;
    let run = super::format::find_run_mut(el, op.run)
        .ok_or_else(|| super::format::missing_run(&op.id, i))?;
    super::format::apply_run(
        run,
        super::format::RunFormatPatch {
            bold: op.bold,
            italic: op.italic,
            underline: op.underline.as_ref().map(|v| v.as_deref()),
            sz: op.sz,
            color: op.color.as_deref(),
            font: op.font.as_deref(),
            east_asia_font: op.east_asia_font.as_deref(),
            em: op.em.as_ref().map(|v| v.as_deref()),
            east_asian_layout: op.east_asian_layout.as_ref().map(|v| {
                v.as_ref().map(|s| super::format::EastAsianLayout {
                    id: s.id,
                    combine: s.combine,
                    combine_brackets: s.combine_brackets.clone(),
                    vert: s.vert,
                    vert_compress: s.vert_compress,
                })
            }),
            highlight: op.highlight.as_ref().map(|v| v.as_deref()),
            strike: op.strike,
            dstrike: op.dstrike,
            vert_align: op.vert_align.as_ref().map(|v| v.as_deref()),
            spacing: op.spacing,
            kern: op.kern,
            scale: op.scale,
        },
    );
    dirty.insert(part_name, ());
    Ok(())
}
pub(crate) fn run_patch(
    fmt: &super::request::para_remain::RunFormatOwned,
) -> super::format::RunFormatPatch<'_> {
    super::format::RunFormatPatch {
        bold: fmt.bold,
        italic: fmt.italic,
        underline: fmt.underline.as_ref().map(|v| v.as_deref()),
        sz: fmt.sz,
        color: fmt.color.as_deref(),
        font: fmt.font.as_deref(),
        east_asia_font: fmt.east_asia_font.as_deref(),
        em: fmt.em.as_ref().map(|v| v.as_deref()),
        east_asian_layout: fmt.east_asian_layout.clone(),
        highlight: fmt.highlight.as_ref().map(|v| v.as_deref()),
        strike: fmt.strike,
        dstrike: fmt.dstrike,
        vert_align: fmt.vert_align.as_ref().map(|v| v.as_deref()),
        spacing: fmt.spacing,
        kern: fmt.kern,
        scale: fmt.scale,
    }
}
