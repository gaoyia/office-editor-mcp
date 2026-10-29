//! DrawingML 公式框共用：判定 `a14:m` AlternateContent、拼接 `m:t`、写 OMML。
//! 不算公式排版引擎。不把 OMML 编成 JSON 树。

use serde_json::{Map, Value};

use crate::error::{Error, ErrorKind, TypeName};
use crate::xml::{Element, Node};

pub const A14_NS: &str = "http://schemas.microsoft.com/office/drawing/2010/main";
pub const MATH_NS: &str = "http://schemas.openxmlformats.org/officeDocument/2006/math";
pub const MC_NS: &str = "http://schemas.openxmlformats.org/markup-compatibility/2006";
/// 缺省 1 英寸见方。
pub const DEFAULT_CX: i64 = 914_400;
pub const DEFAULT_CY: i64 = 914_400;

pub fn choice_requires_a14(choice: &Element) -> bool {
    let Some(req) = choice.attr_local("Requires") else {
        return false;
    };
    req.split(|c: char| c.is_whitespace() || c == ',')
        .any(|t| t == "a14")
}

fn is_a14_m(el: &Element) -> bool {
    el.local_name() == "m"
        && el
            .element_children()
            .any(|c| matches!(c.local_name(), "oMathPara" | "oMath"))
}

fn has_omml(el: &Element) -> bool {
    if is_a14_m(el) || el.local_name() == "oMath" {
        return true;
    }
    el.element_children().any(has_omml)
}

/// Choice `Requires` 含 `a14`，且子孙有 `a14:m` 或 `m:oMath`。
pub fn is_math_ac(el: &Element) -> bool {
    if el.local_name() != "AlternateContent" {
        return false;
    }
    let Some(choice) = el.child_by_local("Choice") else {
        return false;
    };
    choice_requires_a14(choice) && has_omml(choice)
}

/// 该 `oMath` 内全部 `m:t` 的文本拼接。忽略 `a:fld` 外壳。
pub fn omml_text(el: &Element) -> String {
    let mut out = String::new();
    collect_mt(el, &mut out);
    out
}

fn collect_mt(el: &Element, out: &mut String) {
    if el.local_name() == "t" {
        out.push_str(&el.text_content());
        return;
    }
    for c in el.element_children() {
        collect_mt(c, out);
    }
}

/// 只从 Choice 里取公式字，避免把 Fallback 文本框拼进去。
pub fn choice_omml_text(ac: &Element) -> String {
    match ac.child_by_local("Choice") {
        Some(choice) => omml_text(choice),
        None => String::new(),
    }
}

pub fn first_sp(el: &Element) -> Option<&Element> {
    if el.local_name() == "sp" {
        return Some(el);
    }
    for c in el.element_children() {
        if let Some(sp) = first_sp(c) {
            return Some(sp);
        }
    }
    None
}

fn contains_sp(el: &Element) -> bool {
    el.local_name() == "sp" || el.element_children().any(contains_sp)
}

/// 找 Choice 里第一个 `sp`，给事后几何改 `xfrm`。
pub fn first_sp_mut(el: &mut Element) -> Option<&mut Element> {
    if el.local_name() == "sp" {
        return Some(el);
    }
    let mut i = 0usize;
    while i < el.children.len() {
        let hit = match &el.children[i] {
            Node::Element(e) => contains_sp(e),
            _ => false,
        };
        if hit {
            let Node::Element(child) = &mut el.children[i] else {
                return None;
            };
            return first_sp_mut(child);
        }
        i += 1;
    }
    None
}

pub fn cnv_of_sp(sp: &Element) -> (Option<String>, String) {
    let mut raw = None;
    let mut name = String::new();
    if let Some(nv) = sp.child_by_local("nvSpPr") {
        if let Some(pr) = nv.child_by_local("cNvPr") {
            raw = pr
                .attr_local("id")
                .filter(|s| !s.is_empty())
                .map(str::to_string);
            name = pr.attr_local("name").unwrap_or("").to_string();
        }
    }
    (raw, name)
}

pub fn cnv_of_ac(ac: &Element) -> (Option<String>, String) {
    ac.child_by_local("Choice")
        .and_then(first_sp)
        .map(cnv_of_sp)
        .unwrap_or((None, String::new()))
}

pub fn find_omath_mut(el: &mut Element) -> Option<&mut Element> {
    crate::xml::find_by_local_mut(el, "oMath")
}

fn math_t(text: &str) -> Element {
    let mut t = Element::new("m:t").with_text(text);
    if needs_preserve(text) {
        t.set_attr("xml:space", "preserve");
    }
    t
}

pub fn math_run(text: &str) -> Element {
    let mut r = Element::new("m:r");
    r.push_element(math_t(text));
    r
}

pub fn a14_m(text: &str) -> Element {
    let mut m = Element::new("a14:m");
    let mut para = Element::new("m:oMathPara");
    let mut omath = Element::new("m:oMath");
    omath.push_element(math_run(text));
    para.push_element(omath);
    m.push_element(para);
    m
}

pub fn fallback_p(text: &str) -> Element {
    let mut p = Element::new("a:p");
    let mut r = Element::new("a:r");
    r.push_element(Element::new("a:rPr").with_attr("sz", "1100"));
    let mut t = Element::new("a:t").with_text(text);
    if needs_preserve(text) {
        t.set_attr("xml:space", "preserve");
    }
    r.push_element(t);
    p.push_element(r);
    p
}

pub fn math_p(text: &str) -> Element {
    let mut p = Element::new("a:p");
    p.push_element(a14_m(text));
    p
}

fn needs_preserve(text: &str) -> bool {
    text.starts_with(char::is_whitespace)
        || text.ends_with(char::is_whitespace)
        || text.contains('\n')
}

/// 用一条 `m:r` / `m:t` 替换该 `oMath` 子节点；Fallback 里的 `a:t` 一并改成同一句。
pub fn set_ac_text(ac: &mut Element, text: &str) -> bool {
    {
        let Some(choice) = ac.child_by_local_mut("Choice") else {
            return false;
        };
        let Some(omath) = find_omath_mut(choice) else {
            return false;
        };
        omath.children.clear();
        omath.push_element(math_run(text));
    }
    if let Some(fb) = ac.child_by_local_mut("Fallback") {
        set_first_t(fb, text);
    }
    true
}

fn set_fallback_t(el: &mut Element, text: &str) -> bool {
    if el.local_name() == "t" {
        el.set_text(text);
        if needs_preserve(text) {
            el.set_attr("xml:space", "preserve");
        }
        return true;
    }
    for n in el.children.iter_mut() {
        if let crate::xml::Node::Element(c) = n {
            if set_fallback_t(c, text) {
                return true;
            }
        }
    }
    false
}

fn set_first_t(el: &mut Element, text: &str) {
    let _ = set_fallback_t(el, text);
}

pub fn collect_embed_rids(el: &Element) -> Vec<String> {
    crate::model3d::collect_embed_rids(el)
}

pub fn ensure_ignorable(root: &mut Element, token: &str) {
    let key = "mc:Ignorable";
    if let Some((_, v)) = root.attrs.iter_mut().find(|(k, _)| k == key) {
        if !v.split_whitespace().any(|t| t == token) {
            if !v.is_empty() {
                v.push(' ');
            }
            v.push_str(token);
        }
        return;
    }
    root.set_attr(key, token);
}

pub fn reject_expr_kind(obj: &Map<String, Value>, prefix: &str) -> Result<(), Error> {
    if obj.contains_key("expr") || obj.contains_key("kind") {
        return Err(Error::with_path(
            ErrorKind::MathDrawingNoExpr,
            prefix.to_string(),
        ));
    }
    Ok(())
}

pub fn require_text(obj: &Map<String, Value>, prefix: &str) -> Result<String, Error> {
    match obj.get("text") {
        Some(Value::String(s)) if !s.is_empty() => Ok(s.clone()),
        Some(Value::String(_)) => Err(Error::with_path(
            ErrorKind::EmptyMathText,
            format!("{prefix}.text"),
        )),
        None => Err(Error::with_path(
            ErrorKind::MissingField {
                field: "text".into(),
            },
            format!("{prefix}.text"),
        )),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "text".into(),
                expected: TypeName::String,
            },
            format!("{prefix}.text"),
        )),
    }
}
