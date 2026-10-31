//! `/ppt/presProps.xml` 的 `p:showPr`。不算放映引擎，不写自定义放映范围。

use serde_json::{json, Map, Value};

use crate::error::{Error, ErrorKind};
use crate::opc::{self, Package, Relationships};
use crate::xml::{Element, Node};

#[derive(Debug, Clone)]
pub struct SetShowPrOp {
    pub type_name: Option<String>,
    pub loop_: Option<bool>,
    pub show_narration: Option<bool>,
    pub show_animation: Option<bool>,
    pub use_timings: Option<bool>,
}

struct ShowState {
    type_name: String,
    loop_: bool,
    show_narration: bool,
    show_animation: bool,
    use_timings: bool,
}

const BROWSE_MODE_URI: &str = "{F99C55AA-B7CB-42B0-86F8-08522FDF87E8}";
const P14_NS: &str = "http://schemas.microsoft.com/office/powerpoint/2010/main";
fn powerpoint_default() -> ShowState {
    ShowState {
        type_name: "present".into(),
        loop_: false,
        show_narration: true,
        show_animation: true,
        use_timings: true,
    }
}

fn is_powerpoint_default(s: &ShowState) -> bool {
    s.type_name == "present" && !s.loop_ && s.show_narration && s.show_animation && s.use_timings
}

pub fn inspect(pkg: &Package, pres_part: &str) -> Result<Value, Error> {
    let Some(part) = pres_props_part(pkg, pres_part)? else {
        return Ok(state_json(&powerpoint_default()));
    };
    let doc = super::list_pin::parse_part(pkg, &part)?;
    Ok(inspect_root(&doc.root))
}

fn inspect_root(root: &Element) -> Value {
    match root.child_by_local("showPr") {
        None => state_json(&powerpoint_default()),
        Some(el) => state_json(&read_existing(el)),
    }
}

fn state_json(s: &ShowState) -> Value {
    let mut obj = Map::new();
    obj.insert("type".into(), json!(s.type_name));
    obj.insert("loop".into(), json!(s.loop_));
    obj.insert("showNarration".into(), json!(s.show_narration));
    obj.insert("showAnimation".into(), json!(s.show_animation));
    obj.insert("useTimings".into(), json!(s.use_timings));
    Value::Object(obj)
}

fn read_existing(el: &Element) -> ShowState {
    ShowState {
        type_name: type_of(el),
        // schema 缺省：loop / showNarration 假，showAnimation / useTimings 真
        loop_: bool_attr(el, "loop", false),
        show_narration: bool_attr(el, "showNarration", false),
        show_animation: bool_attr(el, "showAnimation", true),
        use_timings: bool_attr(el, "useTimings", true),
    }
}

fn type_of(el: &Element) -> String {
    for c in el.element_children() {
        match c.local_name() {
            "present" => return "present".into(),
            "browse" => return "browse".into(),
            "kiosk" => return "kiosk".into(),
            _ => {}
        }
    }
    "present".into()
}

fn bool_attr(el: &Element, name: &str, default: bool) -> bool {
    match el.attr(name).or_else(|| el.attr_local(name)) {
        None => default,
        Some(s) => matches!(s, "1" | "true" | "TRUE" | "on"),
    }
}

fn pres_props_part(pkg: &Package, pres_part: &str) -> Result<Option<String>, Error> {
    let rels = Relationships::from_package(pkg, pres_part)?;
    let Some(rel) = rels
        .rels
        .iter()
        .find(|r| r.rel_type.ends_with("/presProps"))
    else {
        return Ok(None);
    };
    Ok(Some(opc::resolve_id(pkg, pres_part, &rel.id)?))
}

pub fn apply(pkg: &mut Package, pres_part: &str, op: &SetShowPrOp, i: usize) -> Result<(), Error> {
    let part = pres_props_part(pkg, pres_part)?.ok_or_else(|| {
        Error::with_path(
            ErrorKind::PartMissing {
                name: "/ppt/presProps.xml".into(),
            },
            format!("ops[{i}].id"),
        )
    })?;
    let mut doc = super::list_pin::parse_part(pkg, &part)?;
    patch_root(&mut doc.root, op);
    super::list_pin::write_part(pkg, &part, &doc)?;
    Ok(())
}

fn patch_root(root: &mut Element, op: &SetShowPrOp) {
    let existing = root.child_by_local("showPr").is_some();
    let extra = root.child_by_local("showPr").is_some_and(has_extra);
    let mut state = if existing {
        read_existing(root.child_by_local("showPr").expect("刚确认有 showPr"))
    } else {
        powerpoint_default()
    };
    if let Some(t) = &op.type_name {
        state.type_name = t.clone();
    }
    if let Some(v) = op.loop_ {
        state.loop_ = v;
    }
    if let Some(v) = op.show_narration {
        state.show_narration = v;
    }
    if let Some(v) = op.show_animation {
        state.show_animation = v;
    }
    if let Some(v) = op.use_timings {
        state.use_timings = v;
    }
    if is_powerpoint_default(&state) && !extra {
        remove_child(root, "showPr");
        return;
    }
    if !existing {
        let at = show_pr_insert_index(root);
        root.children
            .insert(at, Node::Element(Element::new(root.prefixed("showPr"))));
    }
    let el = root.child_by_local_mut("showPr").expect("刚确认有 showPr");
    write_state(el, &state, !existing);
    if state.type_name == "browse" {
        ensure_browse_mode(el);
    } else {
        strip_browse_mode(el);
    }
}

fn has_extra(el: &Element) -> bool {
    el.element_children().any(|c| match c.local_name() {
        "sldRg" | "custShow" | "penClr" => true,
        "extLst" => ext_lst_has_non_browse(c),
        _ => false,
    })
}

fn ext_lst_has_non_browse(lst: &Element) -> bool {
    lst.element_children()
        .any(|ext| ext.attr("uri").or_else(|| ext.attr_local("uri")) != Some(BROWSE_MODE_URI))
}

fn ensure_browse_mode(el: &mut Element) {
    if let Some(lst) = el.child_by_local("extLst") {
        if lst
            .element_children()
            .any(|ext| ext.attr("uri").or_else(|| ext.attr_local("uri")) == Some(BROWSE_MODE_URI))
        {
            return;
        }
    }
    let lst_name = el.prefixed("extLst");
    if el.child_by_local("extLst").is_none() {
        el.push_element(Element::new(lst_name));
    }
    let lst = el.child_by_local_mut("extLst").expect("刚插入 extLst");
    let mut ext = Element::new(lst.prefixed("ext")).with_attr("uri", BROWSE_MODE_URI);
    ext.push_element(
        Element::new("p14:browseMode")
            .with_attr("xmlns:p14", P14_NS)
            .with_attr("showStatus", "0"),
    );
    lst.push_element(ext);
}

fn strip_browse_mode(el: &mut Element) {
    if let Some(lst) = el.child_by_local_mut("extLst") {
        lst.children.retain(|n| match n {
            Node::Element(ext) => {
                ext.attr("uri").or_else(|| ext.attr_local("uri")) != Some(BROWSE_MODE_URI)
            }
            _ => true,
        });
    }
    let empty = el
        .child_by_local("extLst")
        .is_some_and(|lst| lst.element_children().next().is_none());
    if empty {
        remove_child(el, "extLst");
    }
}

fn write_state(el: &mut Element, state: &ShowState, created: bool) {
    set_type_child(el, &state.type_name);
    write_true_as_one(el, "loop", state.loop_);
    write_true_as_one(el, "showNarration", state.show_narration);
    write_false_as_zero(el, "showAnimation", state.show_animation);
    write_false_as_zero(el, "useTimings", state.use_timings);
    if created {
        ensure_sld_all(el);
    }
}

fn set_type_child(el: &mut Element, ty: &str) {
    el.children.retain(|n| match n {
        Node::Element(e) => !matches!(e.local_name(), "present" | "browse" | "kiosk"),
        _ => true,
    });
    let child = Element::new(el.prefixed(ty));
    let at = el
        .children
        .iter()
        .position(|n| matches!(n, Node::Element(_)))
        .unwrap_or(0);
    el.children.insert(at, Node::Element(child));
}

fn ensure_sld_all(el: &mut Element) {
    if el
        .element_children()
        .any(|c| matches!(c.local_name(), "sldAll" | "sldRg" | "custShow"))
    {
        return;
    }
    let at = el
        .children
        .iter()
        .position(|n| {
            matches!(
                n,
                Node::Element(e) if matches!(e.local_name(), "present" | "browse" | "kiosk")
            )
        })
        .map(|i| i + 1)
        .unwrap_or(el.children.len());
    el.children
        .insert(at, Node::Element(Element::new(el.prefixed("sldAll"))));
}

fn write_true_as_one(el: &mut Element, name: &str, value: bool) {
    if value {
        el.set_attr(name, "1");
    } else {
        el.remove_attr(name);
    }
}

fn write_false_as_zero(el: &mut Element, name: &str, value: bool) {
    if value {
        el.remove_attr(name);
    } else {
        el.set_attr(name, "0");
    }
}

fn show_pr_insert_index(root: &Element) -> usize {
    const AFTER: &[&str] = &["htmlPubPr", "webPr", "prnPr"];
    const BEFORE: &[&str] = &["clrMru", "extLst"];
    let mut last_after = None;
    let mut first_before = None;
    for (i, n) in root.children.iter().enumerate() {
        let Node::Element(e) = n else {
            continue;
        };
        let name = e.local_name();
        if AFTER.contains(&name) {
            last_after = Some(i);
        }
        if first_before.is_none() && BEFORE.contains(&name) {
            first_before = Some(i);
        }
    }
    if let Some(i) = last_after {
        return i + 1;
    }
    if let Some(i) = first_before {
        return i;
    }
    root.children.len()
}

fn remove_child(parent: &mut Element, local: &str) {
    parent.children.retain(|n| match n {
        Node::Element(e) => e.local_name() != local,
        _ => true,
    });
}
