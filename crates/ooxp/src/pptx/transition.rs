//! 幻灯片 `<p:transition>`：名单、时长、自动进片、方向与 prstTrans。不写 `<p:timing>`。

use serde_json::{json, Map, Value};

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

const PRST: &[&str] = &[
    "fallOver",
    "drape",
    "curtains",
    "wind",
    "prestige",
    "fracture",
    "crush",
    "peelOff",
    "pageCurlSingle",
    "pageCurlDouble",
    "airplane",
    "origami",
];

const SIDE: &[&str] = &["l", "r", "u", "d"];
const EIGHT: &[&str] = &["l", "r", "u", "d", "ld", "lu", "rd", "ru"];
const CORNER: &[&str] = &["lu", "ru", "ld", "rd"];
const ORIENT: &[&str] = &["horz", "vert"];
const IN_OUT: &[&str] = &["in", "out"];
const LEFT_RIGHT: &[&str] = &["l", "r"];

pub struct SetArgs<'a> {
    pub slide: usize,
    pub type_name: Option<&'a str>,
    pub dur: Option<u32>,
    pub adv_click: bool,
    pub adv_tm: Option<u32>,
    pub dir: Option<&'a str>,
    pub inv_x: bool,
    pub inv_y: bool,
}

pub fn is_writable_type(name: &str) -> bool {
    name == "none"
        || ECMA.contains(&name)
        || P14.contains(&name)
        || name == "morph"
        || PRST.contains(&name)
}

pub fn inspect(root: &Element) -> Value {
    match find_transition(root) {
        None => Value::Null,
        Some(el) => {
            let mut m = Map::new();
            m.insert("type".into(), json!(effect_type(el)));
            m.insert("dur".into(), json!(duration_ms(el)));
            m.insert("advClick".into(), json!(adv_click(el)));
            m.insert(
                "advTm".into(),
                match adv_tm(el) {
                    Some(n) => json!(n),
                    None => Value::Null,
                },
            );
            if let Some(d) = effect_dir(el) {
                m.insert("dir".into(), json!(d));
            }
            if xml_flag(first_effect_el(el).and_then(|e| e.attr_local("invX"))) {
                m.insert("invX".into(), json!(true));
            }
            if xml_flag(first_effect_el(el).and_then(|e| e.attr_local("invY"))) {
                m.insert("invY".into(), json!(true));
            }
            Value::Object(m)
        }
    }
}

pub fn apply(
    pkg: &mut Package,
    slides: &[SlideMeta],
    args: SetArgs<'_>,
    op_index: usize,
) -> Result<(), Error> {
    if args.slide < 1 || args.slide > slides.len() {
        return Err(Error::with_path(
            ErrorKind::SlideNotFound {
                n: args.slide as u32,
            },
            format!("ops[{op_index}].id"),
        ));
    }
    let part = slides[args.slide - 1].part.clone();
    let mut doc = crate::xml::parse(pkg.get(&part)?)?;
    match args.type_name {
        None => {
            if !remove_all_transitions(&mut doc.root) {
                return Ok(());
            }
        }
        Some(name) => {
            validate_extras(name, &args, op_index)?;
            remove_all_transitions(&mut doc.root);
            picture::ensure_xmlns(&mut doc.root, "p14", P14_NS);
            if name == "morph" || PRST.contains(&name) {
                picture::ensure_xmlns(&mut doc.root, "p15", P15_NS);
            }
            let dur = args.dur.unwrap_or(500);
            let node = build_transition_node(name, dur, &args);
            let at = insert_index(&doc.root);
            doc.root.children.insert(at, node);
        }
    }
    pkg.set(&part, doc.to_bytes()?)?;
    Ok(())
}

fn dir_values(name: &str) -> Option<&'static [&'static str]> {
    match name {
        "wipe" | "push" | "pan" | "vortex" => Some(SIDE),
        "cover" | "pull" => Some(EIGHT),
        "strips" => Some(CORNER),
        "blinds" | "checker" | "comb" | "randomBar" | "doors" | "window" => Some(ORIENT),
        "zoom" | "warp" | "split" => Some(IN_OUT),
        "conveyor" | "gallery" | "ferris" | "flip" | "switch" | "reveal" => Some(LEFT_RIGHT),
        _ => None,
    }
}

fn validate_extras(name: &str, args: &SetArgs<'_>, i: usize) -> Result<(), Error> {
    if let Some(dir) = args.dir {
        match dir_values(name) {
            None => {
                return Err(Error::with_path(
                    ErrorKind::InvalidTransitionDir,
                    format!("ops[{i}].dir"),
                ));
            }
            Some(ok) if !ok.contains(&dir) => {
                return Err(Error::with_path(
                    ErrorKind::InvalidTransitionDir,
                    format!("ops[{i}].dir"),
                ));
            }
            Some(_) => {}
        }
    }
    if (args.inv_x || args.inv_y) && !PRST.contains(&name) {
        let field = if args.inv_x { "invX" } else { "invY" };
        return Err(Error::with_path(
            ErrorKind::InvalidTransitionDir,
            format!("ops[{i}].{field}"),
        ));
    }
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

fn first_effect_el(el: &Element) -> Option<&Element> {
    el.element_children()
        .find(|c| c.local_name() != "sndAc" && c.local_name() != "extLst")
}

fn effect_type(el: &Element) -> String {
    match first_effect_el(el) {
        None => "none".into(),
        Some(child) if child.local_name() == "prstTrans" => match child.attr_local("prst") {
            Some(prst) if PRST.contains(&prst) => prst.to_string(),
            _ => "other".into(),
        },
        Some(child) if is_writable_type(child.local_name()) => child.local_name().to_string(),
        Some(_) => "other".into(),
    }
}

fn effect_dir(el: &Element) -> Option<String> {
    first_effect_el(el)
        .and_then(|e| e.attr_local("dir"))
        .map(str::to_string)
}

fn xml_flag(raw: Option<&str>) -> bool {
    matches!(raw.map(str::trim), Some("1" | "true" | "TRUE" | "on"))
}

fn adv_click(el: &Element) -> bool {
    !matches!(
        el.attr_local("advClick").map(str::trim),
        Some("0" | "false" | "FALSE" | "off")
    )
}

fn adv_tm(el: &Element) -> Option<i64> {
    let s = el.attr_local("advTm")?;
    let n = s.parse::<i64>().ok()?;
    (n >= 1).then_some(n)
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

fn build_effect_el(name: &str, args: &SetArgs<'_>) -> Option<Element> {
    if name == "none" {
        return None;
    }
    if PRST.contains(&name) {
        let mut e = Element::new("p15:prstTrans").with_attr("prst", name);
        if args.inv_x {
            e = e.with_attr("invX", "1");
        }
        if args.inv_y {
            e = e.with_attr("invY", "1");
        }
        return Some(e);
    }
    let prefix = if ECMA.contains(&name) {
        "p"
    } else if name == "morph" {
        "p15"
    } else {
        "p14"
    };
    let mut e = Element::new(format!("{prefix}:{name}"));
    if let Some(dir) = args.dir {
        if dir_values(name).is_some() {
            e = e.with_attr("dir", dir);
        }
    }
    Some(e)
}

fn build_transition_el(name: &str, dur: u32, args: &SetArgs<'_>) -> Element {
    let mut tr = Element::new("p:transition")
        .with_attr("spd", spd_for(dur))
        .with_attr("p14:dur", dur.to_string());
    if !args.adv_click {
        tr = tr.with_attr("advClick", "0");
    }
    if let Some(ms) = args.adv_tm {
        tr = tr.with_attr("advTm", ms.to_string());
    }
    if let Some(effect) = build_effect_el(name, args) {
        tr.push_element(effect);
    }
    tr
}

fn build_transition_node(name: &str, dur: u32, args: &SetArgs<'_>) -> Node {
    if name == "none" || ECMA.contains(&name) {
        return Node::Element(build_transition_el(name, dur, args));
    }
    let (prefix, ns, req) = if name == "morph" || PRST.contains(&name) {
        ("p15", P15_NS, "p15")
    } else {
        ("p14", P14_NS, "p14")
    };
    let mut choice = Element::new("mc:Choice")
        .with_attr("Requires", req)
        .with_attr(format!("xmlns:{prefix}"), ns);
    choice.push_element(build_transition_el(name, dur, args));
    let mut fallback = Element::new("mc:Fallback");
    fallback.push_element(build_transition_el("fade", dur, args));
    let mut ac = Element::new("mc:AlternateContent").with_attr("xmlns:mc", MC_NS);
    ac.push_element(choice);
    ac.push_element(fallback);
    Node::Element(ac)
}
