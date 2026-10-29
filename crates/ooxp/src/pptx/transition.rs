//! 幻灯片、母版、版式上的 `<p:transition>`。不写 `<p:timing>`。

use serde_json::{json, Map, Value};

use crate::error::{Error, ErrorKind};
use crate::opc::{ContentTypes, Package, Relationship, Relationships};
use crate::xml::{Element, Node};

use super::media::REL_AUDIO;
use super::picture;
use super::shape::SlideMeta;
use super::REL_NS;

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
    pub master: Option<(usize, Option<usize>)>,
    pub type_name: Option<&'a str>,
    pub dur: Option<u32>,
    pub adv_click: bool,
    pub adv_tm: Option<u32>,
    pub dir: Option<&'a str>,
    pub inv_x: bool,
    pub inv_y: bool,
    pub sound: Option<&'a str>,
    pub orient: Option<&'a str>,
    pub spokes: Option<u32>,
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
            m.insert(
                "sound".into(),
                match sound_name(el) {
                    Some(s) => json!(s),
                    None => Value::Null,
                },
            );
            if let Some(d) = effect_dir(el) {
                m.insert("dir".into(), json!(d));
            }
            if let Some(o) = effect_orient(el) {
                m.insert("orient".into(), json!(o));
            }
            if let Some(n) = effect_spokes(el) {
                m.insert("spokes".into(), json!(n));
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
    pres_part: &str,
    pres_root: &Element,
    args: SetArgs<'_>,
    op_index: usize,
) -> Result<(), Error> {
    let json_path = format!("ops[{op_index}].id");
    let host = picture::TreeHost::from_parts(args.slide, false, args.master);
    let part = picture::resolve_tree_part(
        pkg,
        pres_part,
        pres_root,
        slides,
        &host,
        ErrorKind::MasterNotFound { n: 0 },
        &json_path,
    )?;
    let mut doc = crate::xml::parse(pkg.get(&part)?)?;
    let old_rids = {
        let mut rids = Vec::new();
        collect_snd_rids(&doc.root, &mut rids);
        rids
    };
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
            if let Some(path) = args.sound {
                attach_sound(pkg, &mut doc.root, &part, path, op_index)?;
            }
        }
    }
    drop_unused_audio(pkg, &doc.root, &part, &old_rids)?;
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
    if let Some(orient) = args.orient {
        if name != "split" || !ORIENT.contains(&orient) {
            return Err(Error::with_path(
                ErrorKind::InvalidTransitionOrient,
                format!("ops[{i}].orient"),
            ));
        }
    }
    if let Some(spokes) = args.spokes {
        if !matches!(name, "wheel" | "wheelReverse") || !(1..=8).contains(&spokes) {
            return Err(Error::with_path(
                ErrorKind::InvalidTransitionSpokes,
                format!("ops[{i}].spokes"),
            ));
        }
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
    let index = crate::xml::alternate_child(ac).unwrap_or(None)?;
    let Node::Element(choice) = &ac.children[index] else {
        return None;
    };
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

fn effect_orient(el: &Element) -> Option<String> {
    first_effect_el(el)
        .and_then(|e| e.attr_local("orient"))
        .map(str::to_string)
}

fn effect_spokes(el: &Element) -> Option<u64> {
    first_effect_el(el)
        .and_then(|e| e.attr_local("spokes"))
        .and_then(|s| s.parse().ok())
}

fn sound_name(el: &Element) -> Option<String> {
    for c in el.element_children() {
        if c.local_name() != "sndAc" {
            continue;
        }
        for st in c.element_children() {
            if st.local_name() != "stSnd" {
                continue;
            }
            for snd in st.element_children() {
                if snd.local_name() == "snd" {
                    if let Some(n) = snd.attr_local("name").filter(|s| !s.is_empty()) {
                        return Some(n.to_string());
                    }
                }
            }
        }
    }
    None
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
            "cSld" | "clrMapOvr" | "clrMap" | "sldLayoutIdLst" => after = i + 1,
            "timing" | "extLst" | "hf" | "txStyles" => return i,
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
    if name == "split" {
        if let Some(orient) = args.orient {
            e = e.with_attr("orient", orient);
        }
    }
    if matches!(name, "wheel" | "wheelReverse") {
        if let Some(spokes) = args.spokes {
            e = e.with_attr("spokes", spokes.to_string());
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

fn attach_sound(
    pkg: &mut Package,
    root: &mut Element,
    part: &str,
    file: &str,
    op_index: usize,
) -> Result<(), Error> {
    let p = std::path::PathBuf::from(file);
    let bytes = crate::host_files::read_user_file(file)
        .map_err(|e| e.at(format!("ops[{op_index}].sound")))?;
    if !is_wav(&bytes) {
        return Err(Error::with_path(
            ErrorKind::InvalidMediaFile,
            format!("ops[{op_index}].sound"),
        ));
    }
    let name = p
        .file_name()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("sound.wav")
        .to_string();
    let media_part = next_wav_part(pkg);
    pkg.set(&media_part, bytes)?;
    picture::ensure_xmlns(root, "r", REL_NS);
    let mut rels = Relationships::from_package(pkg, part)?;
    let rid = rels.next_rid();
    rels.push(Relationship {
        id: rid.clone(),
        rel_type: REL_AUDIO.into(),
        target: picture::rel_target_from(part, &media_part),
        target_mode: None,
        extra: Vec::new(),
    });
    rels.write_to(pkg, part)?;
    let mut cts = ContentTypes::from_package(pkg)?;
    cts.ensure_default("wav", "audio/x-wav");
    cts.write_to(pkg)?;
    push_snd_ac_on_transitions(root, &rid, &name);
    Ok(())
}

fn push_snd_ac_on_transitions(root: &mut Element, rid: &str, name: &str) {
    for n in &mut root.children {
        let Node::Element(e) = n else {
            continue;
        };
        if e.local_name() == "transition" {
            push_snd_ac(e, rid, name);
        } else if e.local_name() == "AlternateContent" {
            for child in e.element_children_mut() {
                if matches!(child.local_name(), "Choice" | "Fallback") {
                    for t in child.element_children_mut() {
                        if t.local_name() == "transition" {
                            push_snd_ac(t, rid, name);
                        }
                    }
                }
            }
        }
    }
}

fn push_snd_ac(tr: &mut Element, rid: &str, name: &str) {
    tr.children.retain(|n| match n {
        Node::Element(e) => e.local_name() != "sndAc",
        _ => true,
    });
    let snd = Element::new("p:snd")
        .with_attr("r:embed", rid)
        .with_attr("name", name);
    let mut st = Element::new("p:stSnd");
    st.push_element(snd);
    let mut ac = Element::new("p:sndAc");
    ac.push_element(st);
    tr.push_element(ac);
}

fn drop_unused_audio(
    pkg: &mut Package,
    root: &Element,
    part: &str,
    rids: &[String],
) -> Result<(), Error> {
    if rids.is_empty() {
        return Ok(());
    }
    let mut still = Vec::new();
    collect_snd_rids(root, &mut still);
    let mut rels = Relationships::from_package(pkg, part)?;
    let mut old_parts = Vec::new();
    for rid in rids {
        if still.iter().any(|s| s == rid) {
            continue;
        }
        if let Ok(p) = crate::opc::resolve_id(pkg, part, rid) {
            old_parts.push(p);
        }
        rels.remove_by_id(rid);
    }
    rels.write_to(pkg, part)?;
    for media in old_parts {
        if !picture::media_still_used(pkg, &media)? {
            pkg.remove(&media);
        }
    }
    Ok(())
}

fn collect_snd_rids(el: &Element, out: &mut Vec<String>) {
    if el.local_name() == "snd" {
        if let Some(id) = el.attr_local("embed").filter(|s| !s.is_empty()) {
            out.push(id.to_string());
        }
    }
    for c in el.element_children() {
        collect_snd_rids(c, out);
    }
}

fn next_wav_part(pkg: &Package) -> String {
    let mut n = 1u32;
    loop {
        let name = format!("/ppt/media/media{n}.wav");
        if !pkg.contains(&name) {
            return name;
        }
        n += 1;
    }
}

fn is_wav(bytes: &[u8]) -> bool {
    bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WAVE"
}
