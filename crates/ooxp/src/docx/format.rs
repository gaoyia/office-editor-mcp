//! 段落 `w:pPr` 与 run `w:rPr` 上的直接格式：inspect 与 apply。

use serde_json::{json, Map, Value};

use crate::error::{Error, ErrorKind, IdReason};
use crate::xml::{Element, Node};

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
}

pub struct RunFormatPatch<'a> {
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub underline: Option<Option<&'a str>>,
    pub sz: Option<u32>,
    pub color: Option<&'a str>,
    pub font: Option<&'a str>,
    pub east_asia_font: Option<&'a str>,
}

pub fn para_node(
    p: &Element,
    index: usize,
    container: Option<&str>,
    picture_count: Option<usize>,
    text_box_count: Option<usize>,
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
    {
        ensure_ppr(p);
    }
    if patch.before.is_some()
        || patch.after.is_some()
        || patch.line.is_some()
        || patch.line_rule.is_some()
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
}

pub fn find_run_mut(p: &mut Element, n: usize) -> Option<&mut Element> {
    let mut idx = 0usize;
    find_run_in(p, &mut idx, n)
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
        [
            ("align", json!(align)),
            ("indent", json!(indent)),
            ("spacing", json!(spacing)),
        ],
    );
}

fn stamp_run_count(node: &mut Map<String, Value>, p: &Element) {
    let n = collect_runs(p).len();
    let after = if node.contains_key("textBoxCount") {
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
            })
        })
        .collect()
}

fn collect_runs(p: &Element) -> Vec<&Element> {
    let mut out = Vec::new();
    walk_runs(p, false, &mut out);
    out
}

fn walk_runs<'a>(el: &'a Element, in_del: bool, out: &mut Vec<&'a Element>) {
    if el.local_name() == "txbxContent" {
        return;
    }
    if el.local_name() == "AlternateContent" {
        if let Some(i) = para::first_alt_child_index(el) {
            if let Node::Element(child) = &el.children[i] {
                walk_runs(child, in_del, out);
            }
        }
        return;
    }
    let in_del = in_del || el.local_name() == "del";
    if el.local_name() == "r" {
        if !in_del {
            out.push(el);
        }
        return;
    }
    for n in &el.children {
        if let Node::Element(e) = n {
            walk_runs(e, in_del, out);
        }
    }
}

fn find_run_in<'a>(el: &'a mut Element, idx: &mut usize, n: usize) -> Option<&'a mut Element> {
    if el.local_name() == "txbxContent" {
        return None;
    }
    if el.local_name() == "del" {
        return None;
    }
    if el.local_name() == "AlternateContent" {
        if let Some(i) = para::first_alt_child_index(el) {
            if let Node::Element(e) = &mut el.children[i] {
                return find_run_in(e, idx, n);
            }
        }
        return None;
    }
    if el.local_name() == "r" {
        *idx += 1;
        if *idx == n {
            return Some(el);
        }
        return None;
    }
    let children = el.children.as_mut_slice();
    for child in children {
        if let Node::Element(e) = child {
            if let Some(found) = find_run_in(e, idx, n) {
                return Some(found);
            }
        }
    }
    None
}

fn run_visible_text(r: &Element) -> String {
    let mut out = String::new();
    walk_run_text(r, false, &mut out);
    out
}

fn walk_run_text(el: &Element, in_del: bool, out: &mut String) {
    if el.local_name() == "instrText" {
        return;
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
    for n in &el.children {
        if let Node::Element(e) = n {
            walk_run_text(e, in_del, out);
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

fn int_attr(el: &Element, local: &str) -> Option<u32> {
    el.attr_local(local).and_then(|s| s.parse().ok())
}

fn int_val(parent: &Element, local: &str) -> Option<u32> {
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

fn insert_before(map: &mut Map<String, Value>, before: &str, pairs: [(&str, Value); 3]) {
    let mut out = Map::new();
    let mut done = false;
    for (k, v) in map.iter() {
        if k == before {
            for (nk, nv) in &pairs {
                out.insert((*nk).into(), nv.clone());
            }
            done = true;
        }
        out.insert(k.clone(), v.clone());
    }
    if !done {
        for (nk, nv) in pairs {
            out.insert(nk.into(), nv);
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
