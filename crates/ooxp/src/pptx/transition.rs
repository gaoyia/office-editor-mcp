//! 幻灯片 `<p:transition>`：名单、时长与增删。不写 `<p:timing>`。

use serde_json::{json, Value};

use crate::error::{Error, ErrorKind};
use crate::opc::Package;
use crate::xml::{Element, Node};

use super::picture;
use super::shape::SlideMeta;

const MC_NS: &str = "http://schemas.openxmlformats.org/markup-compatibility/2006";
const P14_NS: &str = "http://schemas.microsoft.com/office/powerpoint/2010/main";
const P15_NS: &str = "http://schemas.microsoft.com/office/powerpoint/2012/main";

const ECMA: &[&str] = &[
    "blinds",
    "checker",
    "circle",
    "dissolve",
    "comb",
    "cover",
    "cut",
    "diamond",
    "fade",
    "newsflash",
    "plus",
    "pull",
    "push",
    "random",
    "randomBar",
    "split",
    "strips",
    "wedge",
    "wheel",
    "wipe",
    "zoom",
];

const P14: &[&str] = &[
    "vortex",
    "switch",
    "flip",
    "ripple",
    "honeycomb",
    "prism",
    "doors",
    "window",
    "ferris",
    "gallery",
    "conveyor",
    "pan",
    "glitter",
    "warp",
    "flythrough",
    "flash",
    "shred",
    "reveal",
    "wheelReverse",
];

pub fn is_writable_type(name: &str) -> bool {
    name == "none" || ECMA.contains(&name) || P14.contains(&name) || name == "morph"
}

pub fn inspect(root: &Element) -> Value {
    match find_transition(root) {
        None => Value::Null,
        Some(el) => json!({
            "type": effect_type(el),
            "dur": duration_ms(el),
        }),
    }
}

pub fn apply(
    pkg: &mut Package,
    slides: &[SlideMeta],
    slide: usize,
    type_name: Option<&str>,
    dur: Option<u32>,
    op_index: usize,
) -> Result<(), Error> {
    if slide < 1 || slide > slides.len() {
        return Err(Error::with_path(
            ErrorKind::SlideNotFound { n: slide as u32 },
            format!("ops[{op_index}].id"),
        ));
    }
    let part = slides[slide - 1].part.clone();
    let mut doc = crate::xml::parse(pkg.get(&part)?)?;
    match type_name {
        None => {
            if !remove_all_transitions(&mut doc.root) {
                return Ok(());
            }
        }
        Some(name) => {
            remove_all_transitions(&mut doc.root);
            picture::ensure_xmlns(&mut doc.root, "p14", P14_NS);
            if name == "morph" {
                picture::ensure_xmlns(&mut doc.root, "p15", P15_NS);
            }
            let dur = dur.unwrap_or(500);
            let node = build_transition_node(name, dur);
            let at = insert_index(&doc.root);
            doc.root.children.insert(at, node);
        }
    }
    pkg.set(&part, doc.to_bytes()?)?;
    Ok(())
}

fn find_transition(root: &Element) -> Option<&Element> {
    for n in &root.children {
        let Node::Element(e) = n else {
            continue;
        };
        if e.local_name() == "transition" {
            return Some(e);
        }
        if e.local_name() == "AlternateContent" {
            if let Some(t) = transition_in_ac(e) {
                return Some(t);
            }
        }
    }
    None
}

fn transition_in_ac(ac: &Element) -> Option<&Element> {
    let choice = ac
        .element_children()
        .find(|c| c.local_name() == "Choice")
        .or_else(|| ac.element_children().find(|c| c.local_name() == "Fallback"))?;
    choice
        .element_children()
        .find(|c| c.local_name() == "transition")
}

fn effect_type(el: &Element) -> String {
    match first_effect(el) {
        None => "none".into(),
        Some(name) if is_writable_type(name) => name.to_string(),
        Some(_) => "other".into(),
    }
}

fn first_effect(el: &Element) -> Option<&str> {
    el.element_children()
        .map(|c| c.local_name())
        .find(|n| *n != "sndAc" && *n != "extLst")
}

fn duration_ms(el: &Element) -> i64 {
    if let Some(s) = el.attr_local("dur") {
        if let Ok(n) = s.parse::<i64>() {
            if n >= 1 {
                return n;
            }
        }
    }
    match el.attr_local("spd") {
        Some("slow") => 1000,
        Some("med") => 750,
        _ => 500,
    }
}

fn remove_all_transitions(root: &mut Element) -> bool {
    let mut any = false;
    while remove_transition(root) {
        any = true;
    }
    any
}

fn remove_transition(root: &mut Element) -> bool {
    let mut i = 0;
    while i < root.children.len() {
        let hit = match &root.children[i] {
            Node::Element(e) if e.local_name() == "transition" => true,
            Node::Element(e) if e.local_name() == "AlternateContent" => ac_holds_transition(e),
            _ => false,
        };
        if hit {
            root.children.remove(i);
            return true;
        }
        i += 1;
    }
    false
}

fn ac_holds_transition(ac: &Element) -> bool {
    transition_in_ac(ac).is_some()
}

fn insert_index(root: &Element) -> usize {
    let mut after = 0usize;
    for (i, n) in root.children.iter().enumerate() {
        let Node::Element(e) = n else {
            continue;
        };
        match e.local_name() {
            "cSld" | "clrMapOvr" => after = i + 1,
            "timing" | "extLst" => return i,
            _ => {}
        }
    }
    after
}

fn spd_for(dur: u32) -> &'static str {
    if dur <= 500 {
        "fast"
    } else if dur <= 750 {
        "med"
    } else {
        "slow"
    }
}

fn build_transition_el(effect: Option<(&str, &str)>, dur: u32) -> Element {
    let mut tr = Element::new("p:transition")
        .with_attr("spd", spd_for(dur))
        .with_attr("p14:dur", dur.to_string());
    if let Some((prefix, name)) = effect {
        tr.push_element(Element::new(format!("{prefix}:{name}")));
    }
    tr
}

fn build_transition_node(name: &str, dur: u32) -> Node {
    if name == "none" {
        return Node::Element(build_transition_el(None, dur));
    }
    if ECMA.contains(&name) {
        return Node::Element(build_transition_el(Some(("p", name)), dur));
    }
    let (prefix, ns, req) = if name == "morph" {
        ("p15", P15_NS, "p15")
    } else {
        ("p14", P14_NS, "p14")
    };
    let mut choice = Element::new("mc:Choice")
        .with_attr("Requires", req)
        .with_attr(format!("xmlns:{prefix}"), ns);
    choice.push_element(build_transition_el(Some((prefix, name)), dur));
    let mut fallback = Element::new("mc:Fallback");
    fallback.push_element(build_transition_el(Some(("p", "fade")), dur));
    let mut ac = Element::new("mc:AlternateContent").with_attr("xmlns:mc", MC_NS);
    ac.push_element(choice);
    ac.push_element(fallback);
    Node::Element(ac)
}
