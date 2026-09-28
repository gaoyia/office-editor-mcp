//! 幻灯片 `<p:timing>` 只读摘要。不写时间轴。

use std::collections::HashMap;

use serde_json::{json, Map, Value};

use crate::chart::{CHARTEX_NS, CHART_NS};
use crate::smartart::DGM_NS;
use crate::xml::{Element, Node};

use super::shape;
use super::table::TABLE_URI;

const EFFECT_TYPES: &[&str] = &["clickEffect", "withEffect", "afterEffect"];
const PRESET_CLASS: &[&str] = &["entr", "exit", "emph", "path", "verb", "mediacall"];

pub fn list(root: &Element, slide: usize) -> Vec<Value> {
    let objects = object_map(root, slide);
    let mut out = Vec::new();
    walk_timings(root, &mut |timing| {
        shape::walk_choice(timing, &mut |e| {
            if is_effect(e) {
                out.push(effect_json(e, &objects));
            }
        });
    });
    out
}

fn walk_timings<'a>(el: &'a Element, f: &mut impl FnMut(&'a Element)) {
    if el.local_name() == "AlternateContent" {
        if let Some(i) = shape::first_alt_child_index(el) {
            if let Node::Element(child) = &el.children[i] {
                walk_timings(child, f);
            }
        }
        return;
    }
    if el.local_name() == "timing" {
        f(el);
        return;
    }
    for c in el.element_children() {
        walk_timings(c, f);
    }
}

fn is_effect(el: &Element) -> bool {
    el.local_name() == "cTn"
        && el
            .attr_local("nodeType")
            .is_some_and(|t| EFFECT_TYPES.contains(&t))
}

fn effect_json(effect: &Element, objects: &HashMap<String, String>) -> Value {
    let trigger = match effect.attr_local("nodeType") {
        Some("withEffect") => "with",
        Some("afterEffect") => "after",
        _ => "click",
    };
    let mut spid = None;
    let mut filter = None;
    let mut first_behavior = None;
    walk_effect_body(effect, &mut |e| {
        if e.local_name() == "spTgt" && spid.is_none() {
            if let Some(v) = e.attr_local("spid").filter(|s| !s.is_empty()) {
                spid = Some(v.to_string());
            }
        }
        if e.local_name() == "animEffect" && filter.is_none() {
            if let Some(v) = e.attr_local("filter").filter(|s| !s.is_empty()) {
                filter = Some(short_filter(v));
            }
        }
        if first_behavior.is_none() {
            match e.local_name() {
                "animMotion" | "animRot" | "animScale" | "animClr" | "anim" | "set" => {
                    first_behavior = Some(e.local_name().to_string());
                }
                _ => {}
            }
        }
    });
    let type_name = if let Some(f) = filter {
        f
    } else if let Some(b) = first_behavior {
        match b.as_str() {
            "animMotion" => "motion",
            "animRot" => "spin",
            "animScale" => "scale",
            "animClr" => "color",
            "anim" => "anim",
            "set" => "set",
            _ => "other",
        }
        .to_string()
    } else if let Some(pc) = effect
        .attr_local("presetClass")
        .filter(|s| PRESET_CLASS.contains(s))
    {
        pc.to_string()
    } else {
        "other".into()
    };
    let target = spid
        .as_deref()
        .and_then(|s| lookup(objects, s))
        .map_or(Value::Null, |id| json!(id));
    let mut obj = Map::new();
    obj.insert("target".into(), target);
    obj.insert("type".into(), json!(type_name));
    obj.insert("trigger".into(), json!(trigger));
    Value::Object(obj)
}

fn walk_effect_body<'a>(effect: &'a Element, f: &mut impl FnMut(&'a Element)) {
    fn rec<'a>(el: &'a Element, f: &mut impl FnMut(&'a Element), skip_nested: bool) {
        if el.local_name() == "AlternateContent" {
            if let Some(i) = shape::first_alt_child_index(el) {
                if let Node::Element(child) = &el.children[i] {
                    rec(child, f, skip_nested);
                }
            }
            return;
        }
        if skip_nested && is_effect(el) {
            return;
        }
        f(el);
        for c in el.element_children() {
            rec(c, f, true);
        }
    }
    rec(effect, f, false);
}

fn short_filter(raw: &str) -> String {
    match raw.find('(') {
        Some(i) => raw[..i].to_string(),
        None => raw.to_string(),
    }
}

fn lookup<'a>(objects: &'a HashMap<String, String>, spid: &str) -> Option<&'a str> {
    if let Some(id) = objects.get(spid) {
        return Some(id.as_str());
    }
    objects
        .iter()
        .find(|(k, _)| ids_match(k, spid))
        .map(|(_, v)| v.as_str())
}

fn ids_match(file: &str, req: &str) -> bool {
    match (file.parse::<u64>(), req.parse::<u64>()) {
        (Ok(a), Ok(b)) => a == b,
        _ => file == req,
    }
}

fn object_map(root: &Element, slide: usize) -> HashMap<String, String> {
    let host = format!("slide:{slide}");
    let mut map = HashMap::new();
    shape::walk_choice(root, &mut |e| match e.local_name() {
        "sp" => {
            if let Some(id) = cnv_id(e, "nvSpPr") {
                map.insert(id.clone(), format!("{host}/shape:{id}"));
            }
        }
        "pic" => {
            if let Some(id) = cnv_id(e, "nvPicPr") {
                let kind = match super::media::media_kind(e) {
                    Some(super::media::MediaKind::Video) => "video",
                    Some(super::media::MediaKind::Audio) => "audio",
                    None => "pic",
                };
                map.insert(id.clone(), format!("{host}/{kind}:{id}"));
            }
        }
        "graphicFrame" => {
            if let Some(id) = cnv_id(e, "nvGraphicFramePr") {
                if let Some(kind) = frame_kind(e) {
                    map.insert(id.clone(), format!("{host}/{kind}:{id}"));
                }
            }
        }
        _ => {}
    });
    map
}

fn cnv_id(el: &Element, nv_local: &str) -> Option<String> {
    el.child_by_local(nv_local)
        .and_then(|nv| nv.child_by_local("cNvPr"))
        .and_then(|pr| pr.attr_local("id"))
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn frame_kind(frame: &Element) -> Option<&'static str> {
    let uri = graphic_uri(frame)?;
    let n = uri.trim_end_matches('/');
    if n == TABLE_URI {
        Some("table")
    } else if n == CHART_NS || n == CHARTEX_NS {
        Some("chart")
    } else if n == DGM_NS {
        Some("smartdiagram")
    } else {
        None
    }
}

fn graphic_uri(el: &Element) -> Option<String> {
    let mut found = None;
    shape::walk_choice(el, &mut |e| {
        if e.local_name() == "graphicData" && found.is_none() {
            found = e.attr_local("uri").map(str::to_string);
        }
    });
    found
}
