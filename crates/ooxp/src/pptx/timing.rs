//! 幻灯片 `<p:timing>`：效果摘要与预设效果增删。不接受整棵时间轴 JSON。

use std::collections::HashMap;

use serde_json::{json, Map, Value};

use crate::chart::{CHARTEX_NS, CHART_NS};
use crate::error::{Error, ErrorKind};
use crate::opc::{ContentTypes, Package, Relationship, Relationships};
use crate::smartart::DGM_NS;
use crate::xml::{Element, Node};

use super::anim_preset::{self, Preset};
use super::master;
use super::media::REL_AUDIO;
use super::notes;
use super::ole::{OLE_URI, OLE_URI_ISO};
use super::picture;
use super::request::AnimTarget;
use super::shape::{self, SlideMeta};
use super::table::TABLE_URI;
use super::REL_NS;

const EFFECT_TYPES: &[&str] = &["clickEffect", "withEffect", "afterEffect"];
const PRESET_CLASS: &[&str] = &["entr", "exit", "emph", "path", "verb", "mediacall"];

pub fn list(root: &Element, host: &str, placeholders_only: bool) -> Vec<Value> {
    let objects = object_map(root, host, placeholders_only);
    let mut out = Vec::new();
    walk_timings(root, &mut |timing| {
        collect_effects(timing, &objects, host, &mut out);
    });
    out
}

fn collect_effects(
    el: &Element,
    objects: &HashMap<String, String>,
    host: &str,
    out: &mut Vec<Value>,
) {
    struct Frame<'a> {
        el: &'a Element,
        in_ia: bool,
        click_on: Option<String>,
    }
    let mut stack = vec![Frame {
        el,
        in_ia: false,
        click_on: None,
    }];
    while let Some(Frame {
        el,
        in_ia,
        click_on,
    }) = stack.pop()
    {
        if el.local_name() == "AlternateContent" {
            if let Some(i) = shape::first_alt_child_index(el) {
                if let Some(Node::Element(child)) = el.children.get(i) {
                    stack.push(Frame {
                        el: child,
                        in_ia,
                        click_on,
                    });
                }
            }
            continue;
        }
        if el.local_name() == "cTn" && el.attr_local("nodeType") == Some("interactiveSeq") {
            let spid = click_on_spid(el);
            let kids: Vec<_> = el.element_children().collect();
            for child in kids.into_iter().rev() {
                stack.push(Frame {
                    el: child,
                    in_ia: true,
                    click_on: spid.clone(),
                });
            }
            continue;
        }
        if is_effect(el) {
            out.push(effect_json(
                el,
                objects,
                host,
                out.len() + 1,
                in_ia,
                click_on.as_deref(),
            ));
        }
        let kids: Vec<_> = el.element_children().collect();
        let click_on = click_on.clone();
        for child in kids.into_iter().rev() {
            stack.push(Frame {
                el: child,
                in_ia,
                click_on: click_on.clone(),
            });
        }
    }
}

fn click_on_spid(seq: &Element) -> Option<String> {
    let lst = seq.child_by_local("stCondLst")?;
    for cond in lst.element_children() {
        if cond.local_name() == "cond" && cond.attr_local("evt") == Some("onClick") {
            if let Some(tgt) = cond.child_by_local("tgtEl") {
                if let Some(sp) = tgt.child_by_local("spTgt") {
                    if let Some(v) = sp.attr_local("spid").filter(|s| !s.is_empty()) {
                        return Some(v.to_string());
                    }
                }
            }
        }
    }
    None
}

pub struct AnimWrite<'a> {
    pub type_name: &'a str,
    pub trigger: &'a str,
    pub delay: u32,
    pub dur: u32,
    pub repeat: RepeatWrite,
    pub sound: Option<&'a str>,
    pub para: Option<u32>,
    pub click_on: Option<&'a str>,
}

#[derive(Clone, Copy, Debug)]
pub enum RepeatWrite {
    Once,
    Times(u32),
    Indefinite,
}

pub fn anim_preset_ok(name: &str) -> bool {
    anim_preset::by_name(name).is_some()
}

pub fn insert(
    pkg: &mut Package,
    slides: &[SlideMeta],
    pres_part: &str,
    pres_root: &Element,
    host_id: &str,
    target: &AnimTarget,
    args: AnimWrite<'_>,
    op_index: usize,
) -> Result<(), Error> {
    let preset = anim_preset::by_name(args.type_name).ok_or_else(|| {
        Error::with_path(ErrorKind::InvalidAnimType, format!("ops[{op_index}].type"))
    })?;
    let (part, prefix) = resolve_part(pkg, slides, pres_part, pres_root, target, op_index)?;
    let mut doc = crate::xml::parse(pkg.get(&part)?)?;
    let spid = spid_for_host(&doc.root, &prefix, host_id, target.placeholders_only()).ok_or_else(
        || {
            Error::with_path(
                ErrorKind::AnimHostNotFound {
                    id: host_id.to_string(),
                },
                format!("ops[{op_index}].id"),
            )
        },
    )?;
    if let Some(para) = args.para {
        let n = paragraph_count(&doc.root, &spid);
        if n == 0 || para < 1 || para > n {
            return Err(Error::with_path(
                ErrorKind::InvalidAnimPara,
                format!("ops[{op_index}].para"),
            ));
        }
    }
    let click_spid = if args.trigger == "clickOn" {
        let id = args.click_on.ok_or_else(|| {
            Error::with_path(
                ErrorKind::MissingField {
                    field: "clickOn".into(),
                },
                format!("ops[{op_index}].clickOn"),
            )
        })?;
        Some(
            spid_for_host(&doc.root, &prefix, id, target.placeholders_only()).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::AnimHostNotFound { id: id.to_string() },
                    format!("ops[{op_index}].clickOn"),
                )
            })?,
        )
    } else {
        None
    };
    let node_type = node_type_of(args.trigger);
    match first_timing_mut(&mut doc.root) {
        None => {
            let timing = if args.trigger == "clickOn" {
                build_full_clickon(
                    preset,
                    &spid,
                    click_spid.as_deref().unwrap_or(&spid),
                    node_type,
                    1,
                    args.delay,
                    args.dur,
                    args.para,
                    args.repeat,
                )
            } else {
                build_full_timing(
                    preset,
                    &spid,
                    node_type,
                    1,
                    args.delay,
                    args.dur,
                    args.trigger != "click",
                    args.para,
                    args.repeat,
                )
            };
            let at = timing_insert_index(&doc.root);
            doc.root.children.insert(at, Node::Element(timing));
        }
        Some(timing) => {
            append_effect(
                timing,
                preset,
                &spid,
                node_type,
                args.trigger,
                args.delay,
                args.dur,
                args.para,
                args.repeat,
                click_spid.as_deref(),
            );
        }
    }
    if let Some(path) = args.sound {
        attach_sound(pkg, &mut doc.root, &part, path, op_index)?;
    }
    pkg.set(&part, doc.to_bytes()?)?;
    Ok(())
}

pub fn delete(
    pkg: &mut Package,
    slides: &[SlideMeta],
    pres_part: &str,
    pres_root: &Element,
    id: &str,
    target: &AnimTarget,
    index: usize,
    op_index: usize,
) -> Result<(), Error> {
    let (part, _) = resolve_part(pkg, slides, pres_part, pres_root, target, op_index)?;
    let mut doc = crate::xml::parse(pkg.get(&part)?)?;
    let Some(timing_idx) = first_timing_index(&doc.root) else {
        return Err(Error::with_path(
            ErrorKind::AnimNotFound { id: id.to_string() },
            format!("ops[{op_index}].id"),
        ));
    };
    let drop_timing = {
        let timing = match &mut doc.root.children[timing_idx] {
            Node::Element(e) => e,
            _ => unreachable!(),
        };
        let paths = effect_paths(timing);
        if index < 1 || index > paths.len() {
            return Err(Error::with_path(
                ErrorKind::AnimNotFound { id: id.to_string() },
                format!("ops[{op_index}].id"),
            ));
        }
        let path = paths[index - 1].clone();
        let mut rids = Vec::new();
        if let Some(effect) = get_at(timing, &path) {
            collect_snd_rids(effect, &mut rids);
        }
        let spid = spid_at(timing, &path).map(str::to_string);
        remove_effect_at(timing, &path);
        strip_idle_pars(timing);
        shape::prune_anim(timing);
        if let Some(spid) = spid {
            if !spid_still_used(timing, &spid) {
                strip_bld_p(timing, &spid);
            }
        }
        strip_idle_pars(timing);
        shape::prune_anim(timing);
        prune_empty_seqs(timing);
        (!has_effect(timing), rids)
    };
    if drop_timing.0 {
        doc.root.children.remove(timing_idx);
    }
    drop_unused_audio(pkg, &doc.root, &part, &drop_timing.1)?;
    pkg.set(&part, doc.to_bytes()?)?;
    Ok(())
}

pub fn set(
    pkg: &mut Package,
    slides: &[SlideMeta],
    pres_part: &str,
    pres_root: &Element,
    id: &str,
    target: &AnimTarget,
    index: usize,
    type_name: Option<&str>,
    trigger: Option<&str>,
    delay: Option<u32>,
    dur: Option<u32>,
    repeat: Option<RepeatWrite>,
    sound: Option<Option<&str>>,
    para: Option<Option<u32>>,
    click_on: Option<&str>,
    op_index: usize,
) -> Result<(), Error> {
    let (part, prefix) = resolve_part(pkg, slides, pres_part, pres_root, target, op_index)?;
    let mut doc = crate::xml::parse(pkg.get(&part)?)?;
    let path = {
        let Some(timing) = first_timing(&doc.root) else {
            return Err(Error::with_path(
                ErrorKind::AnimNotFound { id: id.to_string() },
                format!("ops[{op_index}].id"),
            ));
        };
        let paths = effect_paths(timing);
        if index < 1 || index > paths.len() {
            return Err(Error::with_path(
                ErrorKind::AnimNotFound { id: id.to_string() },
                format!("ops[{op_index}].id"),
            ));
        }
        paths[index - 1].clone()
    };
    if let Some(Some(n)) = para {
        let spid = first_timing(&doc.root)
            .and_then(|t| spid_at(t, &path).map(str::to_string))
            .ok_or_else(|| {
                Error::with_path(ErrorKind::InvalidAnimPara, format!("ops[{op_index}].para"))
            })?;
        let cnt = paragraph_count(&doc.root, &spid);
        if cnt == 0 || n < 1 || n > cnt {
            return Err(Error::with_path(
                ErrorKind::InvalidAnimPara,
                format!("ops[{op_index}].para"),
            ));
        }
    }
    let click_spid =
        if trigger == Some("clickOn") {
            let click_id = click_on.ok_or_else(|| {
                Error::with_path(
                    ErrorKind::MissingField {
                        field: "clickOn".into(),
                    },
                    format!("ops[{op_index}].clickOn"),
                )
            })?;
            Some(
                spid_for_host(&doc.root, &prefix, click_id, target.placeholders_only())
                    .ok_or_else(|| {
                        Error::with_path(
                            ErrorKind::AnimHostNotFound {
                                id: click_id.to_string(),
                            },
                            format!("ops[{op_index}].clickOn"),
                        )
                    })?,
            )
        } else if let Some(click_id) = click_on {
            Some(
                spid_for_host(&doc.root, &prefix, click_id, target.placeholders_only())
                    .ok_or_else(|| {
                        Error::with_path(
                            ErrorKind::AnimHostNotFound {
                                id: click_id.to_string(),
                            },
                            format!("ops[{op_index}].clickOn"),
                        )
                    })?,
            )
        } else {
            None
        };
    let ctn_id = first_timing(&doc.root)
        .and_then(|t| get_at(t, &path))
        .and_then(|e| e.attr_local("id"))
        .map(str::to_string);
    {
        let Some(timing) = first_timing_mut(&mut doc.root) else {
            return Err(Error::with_path(
                ErrorKind::AnimNotFound { id: id.to_string() },
                format!("ops[{op_index}].id"),
            ));
        };
        if let Some(delay) = delay {
            set_effect_delay(get_mut_at(timing, &path), delay);
        }
        if let Some(name) = type_name {
            let preset = anim_preset::by_name(name).ok_or_else(|| {
                Error::with_path(ErrorKind::InvalidAnimType, format!("ops[{op_index}].type"))
            })?;
            rewrite_type(timing, &path, preset, dur, para.flatten(), op_index)?;
        }
        if let Some(dur) = dur {
            set_effect_dur(get_mut_at(timing, &path), dur, op_index)?;
        }
        if let Some(repeat) = repeat {
            set_repeat(get_mut_at(timing, &path), repeat);
        }
        if let Some(para) = para {
            apply_para_to_tgt(get_mut_at(timing, &path), para);
        }
        if let Some(trigger) = trigger {
            set_effect_trigger(timing, &path, index == 1, trigger, click_spid.as_deref());
        } else if let Some(spid) = click_spid.as_deref() {
            set_click_on_spid(timing, &path, spid);
        }
    }
    if let Some(sound) = sound {
        let path = ctn_id
            .as_deref()
            .and_then(|id| first_timing(&doc.root).and_then(|t| effect_path_by_ctn_id(t, id)))
            .or_else(|| {
                first_timing(&doc.root).and_then(|t| {
                    let p = effect_paths(t);
                    p.get(index.saturating_sub(1)).cloned()
                })
            });
        if let Some(path) = path {
            match sound {
                None => strip_sound(pkg, &mut doc.root, &part, &path)?,
                Some(file) => {
                    strip_sound(pkg, &mut doc.root, &part, &path)?;
                    attach_sound_on(pkg, &mut doc.root, &part, &path, file, op_index)?;
                }
            }
        }
    }
    pkg.set(&part, doc.to_bytes()?)?;
    Ok(())
}

pub fn move_anim(
    pkg: &mut Package,
    slides: &[SlideMeta],
    pres_part: &str,
    pres_root: &Element,
    id: &str,
    target: &AnimTarget,
    index: usize,
    dest: usize,
    op_index: usize,
) -> Result<(), Error> {
    let (part, _) = resolve_part(pkg, slides, pres_part, pres_root, target, op_index)?;
    let mut doc = crate::xml::parse(pkg.get(&part)?)?;
    let Some(timing) = first_timing_mut(&mut doc.root) else {
        return Err(Error::with_path(
            ErrorKind::AnimNotFound { id: id.to_string() },
            format!("ops[{op_index}].id"),
        ));
    };
    let n = effect_paths(timing).len();
    if index < 1 || index > n {
        return Err(Error::with_path(
            ErrorKind::AnimNotFound { id: id.to_string() },
            format!("ops[{op_index}].id"),
        ));
    }
    if dest < 1 || dest > n {
        return Err(Error::with_path(
            ErrorKind::InvalidAnimMoveIndex,
            format!("ops[{op_index}].index"),
        ));
    }
    if index != dest {
        reorder_effect(timing, index - 1, dest - 1);
    }
    pkg.set(&part, doc.to_bytes()?)?;
    Ok(())
}

fn resolve_part(
    pkg: &Package,
    slides: &[SlideMeta],
    pres_part: &str,
    pres_root: &Element,
    target: &AnimTarget,
    op_index: usize,
) -> Result<(String, String), Error> {
    let path = format!("ops[{op_index}].id");
    match target {
        AnimTarget::Slide { slide } => {
            if *slide < 1 || *slide > slides.len() {
                return Err(Error::with_path(
                    ErrorKind::SlideNotFound { n: *slide as u32 },
                    path,
                ));
            }
            Ok((slides[slide - 1].part.clone(), target.prefix()))
        }
        AnimTarget::Notes { slide } => {
            if *slide < 1 || *slide > slides.len() {
                return Err(Error::with_path(
                    ErrorKind::SlideNotFound { n: *slide as u32 },
                    path,
                ));
            }
            let part = notes::notes_slide_part(pkg, &slides[slide - 1].part)?.ok_or_else(|| {
                Error::with_path(
                    ErrorKind::AnimHostNotFound {
                        id: target.prefix(),
                    },
                    path,
                )
            })?;
            Ok((part, target.prefix()))
        }
        AnimTarget::Master { master, layout } => {
            let catalog = master::catalog(pkg, pres_part, pres_root)?;
            let part = catalog.xml_part(*master, *layout, &path)?;
            Ok((part, target.prefix()))
        }
        AnimTarget::NotesMaster => {
            let part = notes::notes_master_part(pkg, pres_part)?.ok_or_else(|| {
                Error::with_path(
                    ErrorKind::AnimHostNotFound {
                        id: target.prefix(),
                    },
                    path,
                )
            })?;
            Ok((part, target.prefix()))
        }
        AnimTarget::Handout => {
            let part = super::handout::handout_part(pkg, pres_part)?.ok_or_else(|| {
                Error::with_path(
                    ErrorKind::AnimHostNotFound {
                        id: target.prefix(),
                    },
                    path,
                )
            })?;
            Ok((part, target.prefix()))
        }
    }
}

fn walk_timings<'a>(root: &'a Element, f: &mut impl FnMut(&'a Element)) {
    let mut stack = vec![root];
    while let Some(el) = stack.pop() {
        if el.local_name() == "AlternateContent" {
            if let Some(i) = shape::first_alt_child_index(el) {
                if let Some(Node::Element(child)) = el.children.get(i) {
                    stack.push(child);
                }
            }
            continue;
        }
        if el.local_name() == "timing" {
            f(el);
            continue;
        }
        let children: Vec<&Element> = el.element_children().collect();
        for child in children.into_iter().rev() {
            stack.push(child);
        }
    }
}

fn is_effect(el: &Element) -> bool {
    el.local_name() == "cTn"
        && el
            .attr_local("nodeType")
            .is_some_and(|t| EFFECT_TYPES.contains(&t))
}

fn effect_json(
    effect: &Element,
    objects: &HashMap<String, String>,
    host: &str,
    k: usize,
    in_ia: bool,
    click_on: Option<&str>,
) -> Value {
    let trigger = if in_ia {
        "clickOn"
    } else {
        match effect.attr_local("nodeType") {
            Some("withEffect") => "with",
            Some("afterEffect") => "after",
            _ => "click",
        }
    };
    let mut spid = None;
    let mut filter = None;
    let mut first_behavior = None;
    let mut para = None;
    let mut sound = None;
    walk_effect_body(effect, &mut |e| {
        if e.local_name() == "spTgt" && spid.is_none() {
            if let Some(v) = e.attr_local("spid").filter(|s| !s.is_empty()) {
                spid = Some(v.to_string());
            }
        }
        if e.local_name() == "pRg" && para.is_none() {
            if let Some(st) = e.attr_local("st").and_then(|s| s.parse::<u32>().ok()) {
                para = Some(st + 1);
            }
        }
        if e.local_name() == "snd" && sound.is_none() {
            sound = e
                .attr_local("name")
                .filter(|s| !s.is_empty())
                .map(str::to_string);
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
    let type_name = if let (Some(class), Some(id)) = (
        effect.attr_local("presetClass"),
        effect
            .attr_local("presetID")
            .and_then(|s| s_to_u32(s).ok())
            .or_else(|| effect.attr_local("presetID").and_then(|s| s.parse().ok())),
    ) {
        anim_preset::by_class_id(class, id)
            .map(|p| p.name.to_string())
            .unwrap_or_else(|| fallback_type(filter, first_behavior, effect))
    } else {
        fallback_type(filter, first_behavior, effect)
    };
    let target = spid
        .as_deref()
        .and_then(|s| lookup(objects, s))
        .map_or(Value::Null, |id| json!(id));
    let click = if in_ia {
        click_on
            .and_then(|s| lookup(objects, s))
            .map_or(Value::Null, |id| json!(id))
    } else {
        Value::Null
    };
    let mut obj = Map::new();
    obj.insert("id".into(), json!(format!("{host}/anim:{k}")));
    obj.insert("target".into(), target);
    obj.insert("type".into(), json!(type_name));
    obj.insert("trigger".into(), json!(trigger));
    obj.insert("delay".into(), effect_delay_ms(effect));
    obj.insert("dur".into(), effect_dur_ms(effect));
    obj.insert("repeat".into(), effect_repeat(effect));
    obj.insert("sound".into(), sound.map_or(Value::Null, |s| json!(s)));
    obj.insert("para".into(), para.map_or(Value::Null, |n| json!(n)));
    obj.insert("clickOn".into(), click);
    Value::Object(obj)
}

fn s_to_u32(s: &str) -> Result<u32, ()> {
    s.parse().map_err(|_| ())
}

fn fallback_type(
    filter: Option<String>,
    first_behavior: Option<String>,
    effect: &Element,
) -> String {
    if let Some(f) = filter {
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
    }
}

fn effect_repeat(effect: &Element) -> Value {
    match effect.attr_local("repeatCount") {
        None => json!(1),
        Some("indefinite") => Value::Null,
        Some(s) => s
            .parse::<u32>()
            .map(|n| json!((n / 1000).max(1)))
            .unwrap_or(Value::Null),
    }
}

fn tl_time_ms(raw: Option<&str>, missing: u32) -> Value {
    match raw {
        None => json!(missing),
        Some("indefinite") => Value::Null,
        Some(s) => s.parse::<u32>().map_or(Value::Null, |n| json!(n)),
    }
}

fn effect_delay_ms(effect: &Element) -> Value {
    let Some(lst) = effect.child_by_local("stCondLst") else {
        return json!(0);
    };
    let delay = lst
        .element_children()
        .find(|e| e.local_name() == "cond" && e.attr_local("evt").is_none())
        .and_then(|c| c.attr_local("delay"));
    tl_time_ms(delay, 0)
}

fn effect_dur_ms(effect: &Element) -> Value {
    let mut anim_effect = None;
    let mut other = None;
    let mut set = None;
    walk_effect_body(effect, &mut |e| {
        let local = e.local_name();
        if !matches!(
            local,
            "animEffect" | "anim" | "animRot" | "animScale" | "animMotion" | "animClr" | "set"
        ) {
            return;
        }
        let dur = e
            .child_by_local("cBhvr")
            .and_then(|b| b.child_by_local("cTn"))
            .and_then(|c| c.attr_local("dur"))
            .map(str::to_string);
        match local {
            "animEffect" if anim_effect.is_none() => anim_effect = Some(dur),
            "set" if set.is_none() => set = Some(dur),
            _ if other.is_none() => other = Some(dur),
            _ => {}
        }
    });
    let found = anim_effect.or(other).or(set);
    match found {
        None => json!(500),
        Some(inner) => tl_time_ms(inner.as_deref(), 500),
    }
}

fn walk_effect_body<'a>(effect: &'a Element, f: &mut impl FnMut(&'a Element)) {
    let mut stack = vec![(effect, false)];
    while let Some((el, skip_nested)) = stack.pop() {
        if el.local_name() == "AlternateContent" {
            if let Some(i) = shape::first_alt_child_index(el) {
                if let Some(Node::Element(child)) = el.children.get(i) {
                    stack.push((child, skip_nested));
                }
            }
            continue;
        }
        if skip_nested && is_effect(el) {
            continue;
        }
        f(el);
        let children: Vec<&Element> = el.element_children().collect();
        for child in children.into_iter().rev() {
            stack.push((child, true));
        }
    }
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

fn object_map(root: &Element, host: &str, placeholders_only: bool) -> HashMap<String, String> {
    let mut map = HashMap::new();
    shape::walk_choice(root, &mut |e| {
        if placeholders_only {
            if e.local_name() == "sp" {
                if let Some(id) = cnv_id(e, "nvSpPr") {
                    map.insert(id.clone(), format!("{host}/shape:{id}"));
                }
            }
            return;
        }
        match e.local_name() {
            "sp" => {
                if e.child_by_local("txBody").is_some() {
                    if let Some(id) = cnv_id(e, "nvSpPr") {
                        map.insert(id.clone(), format!("{host}/shape:{id}"));
                    }
                }
            }
            "cxnSp" => {
                if let Some(id) = cnv_id(e, "nvCxnSpPr") {
                    map.insert(id.clone(), format!("{host}/cxn:{id}"));
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
        }
    });
    map
}

fn spid_for_host(
    root: &Element,
    host: &str,
    host_id: &str,
    placeholders_only: bool,
) -> Option<String> {
    object_map(root, host, placeholders_only)
        .into_iter()
        .find(|(_, id)| id == host_id)
        .map(|(spid, _)| spid)
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
    } else if n == OLE_URI || n == OLE_URI_ISO {
        Some("ole")
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

fn node_type_of(trigger: &str) -> &'static str {
    match trigger {
        "with" => "withEffect",
        "after" => "afterEffect",
        _ => "clickEffect",
    }
}

fn append_effect(
    timing: &mut Element,
    preset: Preset,
    spid: &str,
    node_type: &str,
    trigger: &str,
    delay: u32,
    dur: u32,
    para: Option<u32>,
    repeat: RepeatWrite,
    click_spid: Option<&str>,
) {
    let mut next = max_ctn_id(timing).saturating_add(1);
    if next < 1 {
        next = 1;
    }
    if trigger == "clickOn" {
        let seq_id = take_ids(&mut next, 1)[0];
        let wrap = take_ids(&mut next, 2);
        let n = 1 + preset.behavior_ids();
        let mut effect = anim_preset::effect_par(
            preset,
            &take_ids(&mut next, n),
            spid,
            node_type,
            delay,
            dur,
            para,
        );
        set_repeat(effect_ctn_mut(&mut effect), repeat);
        let seq = wrap_interactive(seq_id, click_spid.unwrap_or(spid), &wrap, effect);
        append_tm_root_seq(timing, seq);
        ensure_bld_p(timing, spid);
        return;
    }
    let click_like = trigger != "with" && trigger != "after";
    let has_any = has_effect(timing);
    ensure_main_seq_lst(timing);
    let seq_id = main_seq_id(timing).unwrap_or(2);
    if click_like || !has_any {
        let auto = !has_any && trigger != "click";
        let group = build_click_group(
            preset,
            &take_ids(&mut next, 3 + preset.behavior_ids()),
            spid,
            node_type,
            delay,
            dur,
            auto,
            seq_id,
            para,
            repeat,
        );
        ensure_main_seq_lst(timing).push_element(group);
    } else if let Some(path) = effect_paths(timing).last().cloned() {
        let n = 1 + preset.behavior_ids();
        let mut sibling = anim_preset::effect_par(
            preset,
            &take_ids(&mut next, n),
            spid,
            node_type,
            delay,
            dur,
            para,
        );
        set_repeat(effect_ctn_mut(&mut sibling), repeat);
        append_sibling_par(timing, &path, sibling);
    } else {
        let auto = trigger != "click";
        let group = build_click_group(
            preset,
            &take_ids(&mut next, 3 + preset.behavior_ids()),
            spid,
            node_type,
            delay,
            dur,
            auto,
            seq_id,
            para,
            repeat,
        );
        ensure_main_seq_lst(timing).push_element(group);
    }
    ensure_bld_p(timing, spid);
}

fn take_ids(next: &mut u32, n: usize) -> Vec<u32> {
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        out.push(*next);
        *next = next.saturating_add(1);
    }
    out
}

fn build_full_timing(
    preset: Preset,
    spid: &str,
    node_type: &str,
    start: u32,
    delay: u32,
    dur: u32,
    auto: bool,
    para: Option<u32>,
    repeat: RepeatWrite,
) -> Element {
    let mut next = start;
    let root_id = take_ids(&mut next, 1)[0];
    let seq_id = take_ids(&mut next, 1)[0];
    let n = 3 + preset.behavior_ids();
    let group = build_click_group(
        preset,
        &take_ids(&mut next, n),
        spid,
        node_type,
        delay,
        dur,
        auto,
        seq_id,
        para,
        repeat,
    );

    let mut main = Element::new("p:cTn")
        .with_attr("id", seq_id.to_string())
        .with_attr("dur", "indefinite")
        .with_attr("nodeType", "mainSeq");
    let mut lst = Element::new("p:childTnLst");
    lst.push_element(group);
    main.push_element(lst);

    let mut seq = Element::new("p:seq")
        .with_attr("concurrent", "1")
        .with_attr("nextAc", "seek");
    seq.push_element(main);
    seq.push_element(nav_cond("prevCondLst", "onPrev"));
    seq.push_element(nav_cond("nextCondLst", "onNext"));

    let mut root_lst = Element::new("p:childTnLst");
    root_lst.push_element(seq);
    let mut root = Element::new("p:cTn")
        .with_attr("id", root_id.to_string())
        .with_attr("dur", "indefinite")
        .with_attr("restart", "never")
        .with_attr("nodeType", "tmRoot");
    root.push_element(root_lst);

    let mut par = Element::new("p:par");
    par.push_element(root);
    let mut tn = Element::new("p:tnLst");
    tn.push_element(par);

    let mut bld = Element::new("p:bldLst");
    bld.push_element(bld_p(spid));

    let mut timing = Element::new("p:timing");
    timing.push_element(tn);
    timing.push_element(bld);
    timing
}

fn build_full_clickon(
    preset: Preset,
    spid: &str,
    click_spid: &str,
    node_type: &str,
    start: u32,
    delay: u32,
    dur: u32,
    para: Option<u32>,
    repeat: RepeatWrite,
) -> Element {
    let mut next = start;
    let root_id = take_ids(&mut next, 1)[0];
    let seq_id = take_ids(&mut next, 1)[0];
    let wrap = take_ids(&mut next, 2);
    let n = 1 + preset.behavior_ids();
    let mut effect = anim_preset::effect_par(
        preset,
        &take_ids(&mut next, n),
        spid,
        node_type,
        delay,
        dur,
        para,
    );
    set_repeat(effect_ctn_mut(&mut effect), repeat);
    let seq = wrap_interactive(seq_id, click_spid, &wrap, effect);

    let mut root_lst = Element::new("p:childTnLst");
    root_lst.push_element(seq);
    let mut root = Element::new("p:cTn")
        .with_attr("id", root_id.to_string())
        .with_attr("dur", "indefinite")
        .with_attr("restart", "never")
        .with_attr("nodeType", "tmRoot");
    root.push_element(root_lst);
    let mut par = Element::new("p:par");
    par.push_element(root);
    let mut tn = Element::new("p:tnLst");
    tn.push_element(par);
    let mut bld = Element::new("p:bldLst");
    bld.push_element(bld_p(spid));
    let mut timing = Element::new("p:timing");
    timing.push_element(tn);
    timing.push_element(bld);
    timing
}

fn build_click_group(
    preset: Preset,
    ids: &[u32],
    spid: &str,
    node_type: &str,
    delay: u32,
    dur: u32,
    auto: bool,
    main_seq_id: u32,
    para: Option<u32>,
    repeat: RepeatWrite,
) -> Element {
    let mut effect = anim_preset::effect_par(preset, &ids[2..], spid, node_type, delay, dur, para);
    set_repeat(effect_ctn_mut(&mut effect), repeat);

    let mut inner_lst = Element::new("p:childTnLst");
    inner_lst.push_element(effect);
    let mut inner_ctn = Element::new("p:cTn")
        .with_attr("id", ids[1].to_string())
        .with_attr("fill", "hold");
    inner_ctn.push_element(cond_lst("0"));
    inner_ctn.push_element(inner_lst);
    let mut inner_par = Element::new("p:par");
    inner_par.push_element(inner_ctn);

    let mut outer_lst = Element::new("p:childTnLst");
    outer_lst.push_element(inner_par);
    let mut outer_ctn = Element::new("p:cTn")
        .with_attr("id", ids[0].to_string())
        .with_attr("fill", "hold");
    outer_ctn.push_element(click_group_conds(auto, main_seq_id));
    outer_ctn.push_element(outer_lst);
    let mut outer_par = Element::new("p:par");
    outer_par.push_element(outer_ctn);
    outer_par
}

fn click_group_conds(auto: bool, main_seq_id: u32) -> Element {
    let mut lst = Element::new("p:stCondLst");
    lst.push_element(Element::new("p:cond").with_attr("delay", "indefinite"));
    if auto {
        let mut cond = Element::new("p:cond")
            .with_attr("evt", "onBegin")
            .with_attr("delay", "0");
        cond.push_element(Element::new("p:tn").with_attr("val", main_seq_id.to_string()));
        lst.push_element(cond);
    }
    lst
}

fn cond_lst(delay: &str) -> Element {
    let mut lst = Element::new("p:stCondLst");
    lst.push_element(Element::new("p:cond").with_attr("delay", delay));
    lst
}

fn nav_cond(name: &str, evt: &str) -> Element {
    let mut tgt = Element::new("p:tgtEl");
    tgt.push_element(Element::new("p:sldTgt"));
    let mut cond = Element::new("p:cond")
        .with_attr("evt", evt)
        .with_attr("delay", "0");
    cond.push_element(tgt);
    let mut lst = Element::new(format!("p:{name}"));
    lst.push_element(cond);
    lst
}

fn bld_p(spid: &str) -> Element {
    Element::new("p:bldP")
        .with_attr("spid", spid)
        .with_attr("grpId", "0")
}

fn ensure_bld_p(timing: &mut Element, spid: &str) {
    let lst = ensure_child(timing, "p:bldLst");
    let exists = lst.element_children().any(|e| {
        e.local_name() == "bldP" && e.attr_local("spid").is_some_and(|v| ids_match(v, spid))
    });
    if !exists {
        lst.push_element(bld_p(spid));
    }
}

fn strip_bld_p(timing: &mut Element, spid: &str) {
    let Some(lst) = timing.child_by_local_mut("bldLst") else {
        return;
    };
    lst.children.retain(|n| match n {
        Node::Element(e) if e.local_name() == "bldP" => {
            !e.attr_local("spid").is_some_and(|v| ids_match(v, spid))
        }
        _ => true,
    });
}

fn spid_still_used(timing: &Element, spid: &str) -> bool {
    let mut used = false;
    shape::walk_choice(timing, &mut |e| {
        if is_effect(e) {
            walk_effect_body(e, &mut |n| {
                if n.local_name() == "spTgt"
                    && n.attr_local("spid").is_some_and(|v| ids_match(v, spid))
                {
                    used = true;
                }
            });
        }
    });
    used
}

fn has_effect(el: &Element) -> bool {
    let mut found = false;
    shape::walk_choice(el, &mut |e| {
        if is_effect(e) {
            found = true;
        }
    });
    found
}

fn max_ctn_id(el: &Element) -> u32 {
    let mut max = 0u32;
    walk_all(el, &mut |e| {
        if e.local_name() == "cTn" {
            if let Some(Ok(n)) = e.attr_local("id").map(str::parse::<u32>) {
                max = max.max(n);
            }
        }
    });
    max
}

fn walk_all<'a>(root: &'a Element, f: &mut impl FnMut(&'a Element)) {
    let mut stack = vec![root];
    while let Some(el) = stack.pop() {
        if el.local_name() == "AlternateContent" {
            if let Some(i) = shape::first_alt_child_index(el) {
                if let Some(Node::Element(child)) = el.children.get(i) {
                    stack.push(child);
                }
            }
            continue;
        }
        f(el);
        let children: Vec<&Element> = el.element_children().collect();
        for child in children.into_iter().rev() {
            stack.push(child);
        }
    }
}

fn ensure_main_seq_lst(timing: &mut Element) -> &mut Element {
    if let Some(path) = find_ctn_path(timing, "mainSeq") {
        let ctn = get_mut_at(timing, &path);
        return ensure_child(ctn, "p:childTnLst");
    }
    let mut next = max_ctn_id(timing).saturating_add(1);
    if next < 1 {
        next = 1;
    }
    if find_ctn_path(timing, "tmRoot").is_some() {
        let seq_id = next;
        let mut main = Element::new("p:cTn")
            .with_attr("id", seq_id.to_string())
            .with_attr("dur", "indefinite")
            .with_attr("nodeType", "mainSeq");
        main.push_element(Element::new("p:childTnLst"));
        let mut seq = Element::new("p:seq")
            .with_attr("concurrent", "1")
            .with_attr("nextAc", "seek");
        seq.push_element(main);
        seq.push_element(nav_cond("prevCondLst", "onPrev"));
        seq.push_element(nav_cond("nextCondLst", "onNext"));
        let root_path = find_ctn_path(timing, "tmRoot").expect("刚确认 tmRoot");
        let root = get_mut_at(timing, &root_path);
        ensure_child(root, "p:childTnLst").push_element(seq);
        let path = find_ctn_path(timing, "mainSeq").expect("刚写入 mainSeq");
        let ctn = get_mut_at(timing, &path);
        return ensure_child(ctn, "p:childTnLst");
    }
    // 有 timing 却没有 tmRoot：在 tnLst 下补一套。
    let tn = ensure_child(timing, "p:tnLst");
    let root_id = next;
    next += 1;
    let seq_id = next;
    let mut main = Element::new("p:cTn")
        .with_attr("id", seq_id.to_string())
        .with_attr("dur", "indefinite")
        .with_attr("nodeType", "mainSeq");
    main.push_element(Element::new("p:childTnLst"));
    let mut seq = Element::new("p:seq")
        .with_attr("concurrent", "1")
        .with_attr("nextAc", "seek");
    seq.push_element(main);
    seq.push_element(nav_cond("prevCondLst", "onPrev"));
    seq.push_element(nav_cond("nextCondLst", "onNext"));
    let mut root_lst = Element::new("p:childTnLst");
    root_lst.push_element(seq);
    let mut root = Element::new("p:cTn")
        .with_attr("id", root_id.to_string())
        .with_attr("dur", "indefinite")
        .with_attr("restart", "never")
        .with_attr("nodeType", "tmRoot");
    root.push_element(root_lst);
    let mut par = Element::new("p:par");
    par.push_element(root);
    tn.push_element(par);
    let path = find_ctn_path(timing, "mainSeq").expect("刚写入 mainSeq");
    let ctn = get_mut_at(timing, &path);
    ensure_child(ctn, "p:childTnLst")
}

fn find_ctn_path(el: &Element, node_type: &str) -> Option<Vec<usize>> {
    let mut stack: Vec<Vec<usize>> = vec![Vec::new()];
    while let Some(path) = stack.pop() {
        let node = path.iter().fold(el, |cur, i| match &cur.children[*i] {
            Node::Element(child) => child,
            Node::Text(_) => unreachable!("路径只指向元素"),
        });
        if node.local_name() == "AlternateContent" {
            if let Some(i) = shape::first_alt_child_index(node) {
                if matches!(&node.children[i], Node::Element(_)) {
                    let mut child = path.clone();
                    child.push(i);
                    stack.push(child);
                }
            }
            continue;
        }
        if node.local_name() == "cTn" && node.attr_local("nodeType") == Some(node_type) {
            return Some(path);
        }
        let indexes: Vec<usize> = node
            .children
            .iter()
            .enumerate()
            .filter_map(|(i, n)| matches!(n, Node::Element(_)).then_some(i))
            .collect();
        for i in indexes.into_iter().rev() {
            let mut child = path.clone();
            child.push(i);
            stack.push(child);
        }
    }
    None
}

fn get_mut_at<'a>(root: &'a mut Element, path: &[usize]) -> &'a mut Element {
    let mut cur = root;
    for &i in path {
        cur = match &mut cur.children[i] {
            Node::Element(e) => e,
            _ => panic!("路径必须指向元素"),
        };
    }
    cur
}

fn get_at<'a>(root: &'a Element, path: &[usize]) -> Option<&'a Element> {
    let mut cur = root;
    for &i in path {
        cur = match cur.children.get(i) {
            Some(Node::Element(e)) => e,
            _ => return None,
        };
    }
    Some(cur)
}

fn ensure_child<'a>(parent: &'a mut Element, name: &str) -> &'a mut Element {
    let local = name.rsplit(':').next().unwrap_or(name);
    if parent.child_by_local(local).is_none() {
        parent.push_element(Element::new(name));
    }
    parent.child_by_local_mut(local).expect("刚插入子元素")
}

fn effect_paths(timing: &Element) -> Vec<Vec<usize>> {
    let mut out = Vec::new();
    let path = Vec::new();
    walk_choice_paths(timing, &path, &mut |p, e| {
        if is_effect(e) {
            out.push(p.to_vec());
        }
    });
    out
}

fn walk_choice_paths<'a>(
    root: &'a Element,
    path: &[usize],
    f: &mut impl FnMut(&[usize], &'a Element),
) {
    let mut stack = vec![(root, path.to_vec())];
    while let Some((el, path)) = stack.pop() {
        if el.local_name() == "AlternateContent" {
            if let Some(i) = shape::first_alt_child_index(el) {
                if let Some(Node::Element(child)) = el.children.get(i) {
                    let mut next = path;
                    next.push(i);
                    stack.push((child, next));
                }
            }
            continue;
        }
        f(&path, el);
        let mut kids = Vec::new();
        for (i, n) in el.children.iter().enumerate() {
            if let Node::Element(child) = n {
                let mut next = path.clone();
                next.push(i);
                kids.push((child, next));
            }
        }
        for item in kids.into_iter().rev() {
            stack.push(item);
        }
    }
}

fn spid_at<'a>(root: &'a Element, path: &[usize]) -> Option<&'a str> {
    let mut cur = root;
    for &i in path {
        cur = match cur.children.get(i) {
            Some(Node::Element(e)) => e,
            _ => return None,
        };
    }
    let mut found = None;
    walk_effect_body(cur, &mut |e| {
        if found.is_none() && e.local_name() == "spTgt" {
            found = e.attr_local("spid");
        }
    });
    found
}

fn remove_effect_at(timing: &mut Element, path: &[usize]) {
    if path.len() >= 2 {
        let par_path = &path[..path.len() - 1];
        if is_local_at(timing, par_path, "par") {
            remove_at(timing, par_path);
            return;
        }
    }
    remove_at(timing, path);
}

fn is_local_at(root: &Element, path: &[usize], local: &str) -> bool {
    let mut cur = root;
    for &i in path {
        cur = match cur.children.get(i) {
            Some(Node::Element(e)) => e,
            _ => return false,
        };
    }
    cur.local_name() == local
}

fn strip_idle_pars(el: &mut Element) {
    for n in &mut el.children {
        if let Node::Element(e) = n {
            strip_idle_pars(e);
        }
    }
    el.children.retain(|n| match n {
        Node::Element(e) if e.local_name() == "par" => has_effect(e),
        _ => true,
    });
}

fn remove_at(root: &mut Element, path: &[usize]) {
    if path.is_empty() {
        return;
    }
    let (parent_path, last) = path.split_at(path.len() - 1);
    let parent = if parent_path.is_empty() {
        root
    } else {
        get_mut_at(root, parent_path)
    };
    parent.children.remove(last[0]);
}

fn append_sibling_par(timing: &mut Element, effect_path: &[usize], sibling: Element) {
    // 效果 cTn 在 par 里；把新 par 接到该 par 的父 childTnLst。
    if effect_path.len() >= 2 {
        let par_path = &effect_path[..effect_path.len() - 1];
        let parent_path = &par_path[..par_path.len() - 1];
        let parent = if parent_path.is_empty() {
            timing
        } else {
            get_mut_at(timing, parent_path)
        };
        parent.push_element(sibling);
        return;
    }
    ensure_main_seq_lst(timing).push_element(sibling);
}

fn first_timing_index(root: &Element) -> Option<usize> {
    root.children
        .iter()
        .position(|n| matches!(n, Node::Element(e) if e.local_name() == "timing"))
}

fn first_timing_mut(root: &mut Element) -> Option<&mut Element> {
    root.element_children_mut()
        .find(|e| e.local_name() == "timing")
}

fn timing_insert_index(root: &Element) -> usize {
    let mut after = 0usize;
    for (i, n) in root.children.iter().enumerate() {
        let Node::Element(e) = n else {
            continue;
        };
        match e.local_name() {
            // 幻灯片：cSld / clrMapOvr / transition。母版：clrMap / sldLayoutIdLst。版式同幻灯片再加 hf。
            "cSld" | "clrMap" | "clrMapOvr" | "sldLayoutIdLst" | "transition" => after = i + 1,
            "AlternateContent" if ac_has_transition(e) => after = i + 1,
            "timing" | "hf" | "notesStyle" | "txStyles" | "matchingName" | "extLst" => return i,
            _ => {}
        }
    }
    after
}

fn ac_has_transition(ac: &Element) -> bool {
    ac.element_children()
        .any(|c| c.element_children().any(|g| g.local_name() == "transition"))
}

fn main_seq_id(timing: &Element) -> Option<u32> {
    let path = find_ctn_path(timing, "mainSeq")?;
    get_at(timing, &path)?
        .attr_local("id")
        .and_then(|s| s.parse().ok())
}

fn set_effect_delay(effect: &mut Element, delay: u32) {
    let lst = ensure_child(effect, "p:stCondLst");
    let mut found = false;
    for n in &mut lst.children {
        let Node::Element(cond) = n else {
            continue;
        };
        if cond.local_name() != "cond" {
            continue;
        }
        if cond.attr_local("evt").is_some() {
            continue;
        }
        cond.set_attr("delay", delay.to_string());
        found = true;
        break;
    }
    if !found {
        lst.children.insert(
            0,
            Node::Element(Element::new("p:cond").with_attr("delay", delay.to_string())),
        );
    }
}

fn set_effect_dur(effect: &mut Element, dur: u32, op_index: usize) -> Result<(), Error> {
    let mut timed = Vec::new();
    let mut hidden = Vec::new();
    collect_dur_targets(effect, &mut Vec::new(), &mut timed, &mut hidden);
    if timed.is_empty() {
        return Err(Error::with_path(
            ErrorKind::AnimNeedEffectDur,
            format!("ops[{op_index}].dur"),
        ));
    }
    for path in timed {
        get_mut_at(effect, &path).set_attr("dur", dur.to_string());
    }
    let hidden_delay = dur.saturating_sub(1).to_string();
    for path in hidden {
        get_mut_at(effect, &path).set_attr("delay", hidden_delay.clone());
    }
    Ok(())
}

fn collect_dur_targets(
    el: &Element,
    path: &mut Vec<usize>,
    timed: &mut Vec<Vec<usize>>,
    hidden: &mut Vec<Vec<usize>>,
) {
    if el.local_name() == "AlternateContent" {
        if let Some(i) = shape::first_alt_child_index(el) {
            if let Some(Node::Element(child)) = el.children.get(i) {
                path.push(i);
                collect_dur_targets(child, path, timed, hidden);
                path.pop();
            }
        }
        return;
    }
    if is_effect(el) && !path.is_empty() {
        return;
    }
    let local = el.local_name();
    if matches!(
        local,
        "animEffect" | "anim" | "animRot" | "animScale" | "animMotion" | "animClr"
    ) {
        if let Some(ctn_path) = bhvr_ctn_path(el, path) {
            timed.push(ctn_path);
        }
    }
    if local == "set" && set_is_hidden(el) {
        if let Some(p) = hidden_delay_path(el, path) {
            hidden.push(p);
        }
    }
    for (i, n) in el.children.iter().enumerate() {
        if let Node::Element(c) = n {
            path.push(i);
            collect_dur_targets(c, path, timed, hidden);
            path.pop();
        }
    }
}

fn bhvr_ctn_path(el: &Element, prefix: &[usize]) -> Option<Vec<usize>> {
    let (i, ctn) = el.children.iter().enumerate().find_map(|(i, n)| match n {
        Node::Element(e) if e.local_name() == "cBhvr" => Some((i, e)),
        _ => None,
    })?;
    let (j, _) = ctn.children.iter().enumerate().find_map(|(j, n)| match n {
        Node::Element(e) if e.local_name() == "cTn" => Some((j, e)),
        _ => None,
    })?;
    let mut p = prefix.to_vec();
    p.push(i);
    p.push(j);
    Some(p)
}

fn set_is_hidden(set: &Element) -> bool {
    set.child_by_local("to")
        .and_then(|t| t.child_by_local("strVal"))
        .and_then(|s| s.attr_local("val"))
        == Some("hidden")
}

fn hidden_delay_path(set: &Element, prefix: &[usize]) -> Option<Vec<usize>> {
    let (i, bhvr) = set.children.iter().enumerate().find_map(|(i, n)| match n {
        Node::Element(e) if e.local_name() == "cBhvr" => Some((i, e)),
        _ => None,
    })?;
    let (j, ctn) = bhvr
        .children
        .iter()
        .enumerate()
        .find_map(|(j, n)| match n {
            Node::Element(e) if e.local_name() == "cTn" => Some((j, e)),
            _ => None,
        })?;
    let (k, lst) = ctn.children.iter().enumerate().find_map(|(k, n)| match n {
        Node::Element(e) if e.local_name() == "stCondLst" => Some((k, e)),
        _ => None,
    })?;
    let (m, _) = lst.children.iter().enumerate().find_map(|(m, n)| match n {
        Node::Element(e) if e.local_name() == "cond" && e.attr_local("evt").is_none() => {
            Some((m, e))
        }
        _ => None,
    })?;
    let mut p = prefix.to_vec();
    p.push(i);
    p.push(j);
    p.push(k);
    p.push(m);
    Some(p)
}

fn set_effect_trigger(
    timing: &mut Element,
    path: &[usize],
    is_first: bool,
    trigger: &str,
    click_spid: Option<&str>,
) {
    let in_ia = interactive_seq_path(timing, path).is_some();
    if trigger == "clickOn" {
        get_mut_at(timing, path).set_attr("nodeType", "clickEffect");
        if in_ia {
            if let Some(spid) = click_spid {
                set_click_on_spid(timing, path, spid);
            }
            return;
        }
        move_to_interactive(timing, path, click_spid.unwrap_or("2"));
        return;
    }
    if in_ia {
        move_from_interactive(timing, path, trigger);
        return;
    }
    let old = match get_at(timing, path).and_then(|e| e.attr_local("nodeType")) {
        Some("withEffect") => "with",
        Some("afterEffect") => "after",
        _ => "click",
    };
    get_mut_at(timing, path).set_attr("nodeType", node_type_of(trigger));
    let seq_id = main_seq_id(timing).unwrap_or(2);
    if is_first {
        if let Some(group) = click_group_ctn_path(timing, path) {
            set_group_auto(get_mut_at(timing, &group), trigger != "click", seq_id);
        }
        return;
    }
    let was_click = old == "click";
    let now_click = trigger == "click";
    if was_click == now_click {
        return;
    }
    if now_click {
        extract_to_click_group(timing, path, seq_id);
    } else {
        merge_into_previous(timing, path);
    }
}

fn move_to_interactive(timing: &mut Element, path: &[usize], click_spid: &str) {
    let Some(mut par) = take_effect_par(timing, path) else {
        return;
    };
    effect_ctn_mut(&mut par).set_attr("nodeType", "clickEffect");
    strip_idle_pars(timing);
    shape::prune_anim(timing);
    prune_empty_seqs(timing);
    let mut next = max_ctn_id(timing).saturating_add(1);
    if next < 1 {
        next = 1;
    }
    let seq_id = take_ids(&mut next, 1)[0];
    let wrap = take_ids(&mut next, 2);
    let seq = wrap_interactive(seq_id, click_spid, &wrap, par);
    append_tm_root_seq(timing, seq);
}

fn move_from_interactive(timing: &mut Element, path: &[usize], trigger: &str) {
    let Some(mut par) = take_effect_par(timing, path) else {
        return;
    };
    effect_ctn_mut(&mut par).set_attr("nodeType", node_type_of(trigger));
    strip_idle_pars(timing);
    shape::prune_anim(timing);
    prune_empty_seqs(timing);
    let remaining = main_effect_count(timing);
    let seq_id = main_seq_id(timing).unwrap_or(2);
    if trigger == "click" || remaining == 0 {
        let auto = remaining == 0 && trigger != "click";
        let mut next = max_ctn_id(timing).saturating_add(1);
        if next < 1 {
            next = 1;
        }
        let group = wrap_par_click_group(par, &take_ids(&mut next, 2), auto, seq_id);
        ensure_main_seq_lst(timing).push_element(group);
    } else if let Some(prev) = effect_paths(timing)
        .into_iter()
        .rev()
        .find(|p| interactive_seq_path(timing, p).is_none())
    {
        append_sibling_par(timing, &prev, par);
    } else {
        let mut next = max_ctn_id(timing).saturating_add(1);
        if next < 1 {
            next = 1;
        }
        let group = wrap_par_click_group(par, &take_ids(&mut next, 2), true, seq_id);
        ensure_main_seq_lst(timing).push_element(group);
    }
}

fn main_effect_count(timing: &Element) -> usize {
    effect_paths(timing)
        .iter()
        .filter(|p| interactive_seq_path(timing, p).is_none())
        .count()
}

fn prune_empty_seqs(timing: &mut Element) {
    let Some(root_path) = find_ctn_path(timing, "tmRoot") else {
        return;
    };
    let Some(lst_i) = get_at(timing, &root_path).and_then(|root| {
        root.children
            .iter()
            .position(|n| matches!(n, Node::Element(e) if e.local_name() == "childTnLst"))
    }) else {
        return;
    };
    let mut lst_path = root_path;
    lst_path.push(lst_i);
    let lst = get_mut_at(timing, &lst_path);
    lst.children.retain(|n| match n {
        Node::Element(e) if e.local_name() == "seq" => has_effect(e),
        _ => true,
    });
}

fn click_group_ctn_path(timing: &Element, effect_path: &[usize]) -> Option<Vec<usize>> {
    let main = find_ctn_path(timing, "mainSeq")?;
    let lst_i = get_at(timing, &main)?
        .children
        .iter()
        .position(|n| matches!(n, Node::Element(e) if e.local_name() == "childTnLst"))?;
    let mut lst_path = main;
    lst_path.push(lst_i);
    if effect_path.len() <= lst_path.len() || !effect_path.starts_with(&lst_path) {
        return None;
    }
    let mut group = lst_path;
    group.push(effect_path[group.len()]);
    let ctn_i = get_at(timing, &group)?
        .children
        .iter()
        .position(|n| matches!(n, Node::Element(e) if e.local_name() == "cTn"))?;
    group.push(ctn_i);
    Some(group)
}

fn set_group_auto(outer_ctn: &mut Element, auto: bool, seq_id: u32) {
    let lst = ensure_child(outer_ctn, "p:stCondLst");
    lst.children.retain(|n| match n {
        Node::Element(e) if e.local_name() == "cond" => e.attr_local("evt") != Some("onBegin"),
        _ => true,
    });
    let has_indef = lst
        .element_children()
        .any(|e| e.local_name() == "cond" && e.attr_local("evt").is_none());
    if !has_indef {
        lst.children.insert(
            0,
            Node::Element(Element::new("p:cond").with_attr("delay", "indefinite")),
        );
    }
    if auto {
        let mut cond = Element::new("p:cond")
            .with_attr("evt", "onBegin")
            .with_attr("delay", "0");
        cond.push_element(Element::new("p:tn").with_attr("val", seq_id.to_string()));
        lst.push_element(cond);
    }
}

fn take_effect_par(timing: &mut Element, effect_path: &[usize]) -> Option<Element> {
    if effect_path.len() < 2 {
        return None;
    }
    let par_path = &effect_path[..effect_path.len() - 1];
    if !is_local_at(timing, par_path, "par") {
        return None;
    }
    let par = get_at(timing, par_path)?.clone();
    remove_at(timing, par_path);
    Some(par)
}

fn wrap_par_click_group(effect_par: Element, ids: &[u32], auto: bool, seq_id: u32) -> Element {
    let mut inner_lst = Element::new("p:childTnLst");
    inner_lst.push_element(effect_par);
    let mut inner_ctn = Element::new("p:cTn")
        .with_attr("id", ids[1].to_string())
        .with_attr("fill", "hold");
    inner_ctn.push_element(cond_lst("0"));
    inner_ctn.push_element(inner_lst);
    let mut inner_par = Element::new("p:par");
    inner_par.push_element(inner_ctn);
    let mut outer_lst = Element::new("p:childTnLst");
    outer_lst.push_element(inner_par);
    let mut outer_ctn = Element::new("p:cTn")
        .with_attr("id", ids[0].to_string())
        .with_attr("fill", "hold");
    outer_ctn.push_element(click_group_conds(auto, seq_id));
    outer_ctn.push_element(outer_lst);
    let mut outer_par = Element::new("p:par");
    outer_par.push_element(outer_ctn);
    outer_par
}

fn extract_to_click_group(timing: &mut Element, path: &[usize], seq_id: u32) {
    let Some(par) = take_effect_par(timing, path) else {
        return;
    };
    strip_idle_pars(timing);
    shape::prune_anim(timing);
    let mut next = max_ctn_id(timing).saturating_add(1);
    if next < 1 {
        next = 1;
    }
    let group = wrap_par_click_group(par, &take_ids(&mut next, 2), false, seq_id);
    ensure_main_seq_lst(timing).push_element(group);
}

fn merge_into_previous(timing: &mut Element, path: &[usize]) {
    let Some(par) = take_effect_par(timing, path) else {
        return;
    };
    strip_idle_pars(timing);
    shape::prune_anim(timing);
    if let Some(prev) = effect_paths(timing).last().cloned() {
        append_sibling_par(timing, &prev, par);
    } else {
        let seq_id = main_seq_id(timing).unwrap_or(2);
        let mut next = max_ctn_id(timing).saturating_add(1);
        if next < 1 {
            next = 1;
        }
        let group = wrap_par_click_group(par, &take_ids(&mut next, 2), true, seq_id);
        ensure_main_seq_lst(timing).push_element(group);
    }
}

fn effect_ctn_mut(par: &mut Element) -> &mut Element {
    par.element_children_mut()
        .find(|e| e.local_name() == "cTn")
        .expect("效果 par 里有 cTn")
}

fn set_repeat(effect: &mut Element, repeat: RepeatWrite) {
    match repeat {
        RepeatWrite::Once => effect.remove_attr("repeatCount"),
        RepeatWrite::Times(n) if n <= 1 => effect.remove_attr("repeatCount"),
        RepeatWrite::Times(n) => {
            effect.set_attr("repeatCount", n.saturating_mul(1000).to_string());
        }
        RepeatWrite::Indefinite => effect.set_attr("repeatCount", "indefinite"),
    }
}

fn wrap_interactive(seq_id: u32, click_spid: &str, wrap_ids: &[u32], effect: Element) -> Element {
    let inner = wrap_par_delay0(effect, wrap_ids);
    let mut st = Element::new("p:stCondLst");
    st.push_element(cond_click(click_spid));
    let mut end = Element::new("p:endSync")
        .with_attr("evt", "end")
        .with_attr("delay", "0");
    end.push_element(Element::new("p:rtn").with_attr("val", "all"));
    let mut lst = Element::new("p:childTnLst");
    lst.push_element(inner);
    let mut ctn = Element::new("p:cTn")
        .with_attr("id", seq_id.to_string())
        .with_attr("restart", "whenNotActive")
        .with_attr("fill", "hold")
        .with_attr("evtFilter", "cancelBubble")
        .with_attr("nodeType", "interactiveSeq");
    ctn.push_element(st);
    ctn.push_element(end);
    ctn.push_element(lst);
    let mut next = Element::new("p:nextCondLst");
    next.push_element(cond_click(click_spid));
    let mut seq = Element::new("p:seq")
        .with_attr("concurrent", "1")
        .with_attr("nextAc", "seek");
    seq.push_element(ctn);
    seq.push_element(next);
    seq
}

fn cond_click(spid: &str) -> Element {
    let mut cond = Element::new("p:cond")
        .with_attr("evt", "onClick")
        .with_attr("delay", "0");
    cond.push_element(tgt_click(spid));
    cond
}

fn tgt_click(spid: &str) -> Element {
    let mut tgt = Element::new("p:tgtEl");
    tgt.push_element(Element::new("p:spTgt").with_attr("spid", spid));
    tgt
}

fn wrap_par_delay0(effect: Element, ids: &[u32]) -> Element {
    let mut inner_lst = Element::new("p:childTnLst");
    inner_lst.push_element(effect);
    let mut inner_ctn = Element::new("p:cTn")
        .with_attr("id", ids[1].to_string())
        .with_attr("fill", "hold");
    inner_ctn.push_element(cond_lst("0"));
    inner_ctn.push_element(inner_lst);
    let mut inner_par = Element::new("p:par");
    inner_par.push_element(inner_ctn);
    let mut outer_lst = Element::new("p:childTnLst");
    outer_lst.push_element(inner_par);
    let mut outer_ctn = Element::new("p:cTn")
        .with_attr("id", ids[0].to_string())
        .with_attr("fill", "hold");
    outer_ctn.push_element(cond_lst("0"));
    outer_ctn.push_element(outer_lst);
    let mut outer = Element::new("p:par");
    outer.push_element(outer_ctn);
    outer
}

fn append_tm_root_seq(timing: &mut Element, seq: Element) {
    if let Some(path) = find_ctn_path(timing, "tmRoot") {
        let root = get_mut_at(timing, &path);
        ensure_child(root, "p:childTnLst").push_element(seq);
        return;
    }
    ensure_main_seq_lst(timing);
    if let Some(path) = find_ctn_path(timing, "tmRoot") {
        let root = get_mut_at(timing, &path);
        ensure_child(root, "p:childTnLst").push_element(seq);
    }
}

fn paragraph_count(root: &Element, spid: &str) -> u32 {
    let mut n = 0u32;
    shape::walk_choice(root, &mut |e| {
        if n > 0 {
            return;
        }
        let id = match e.local_name() {
            "sp" => cnv_id(e, "nvSpPr"),
            "cxnSp" => cnv_id(e, "nvCxnSpPr"),
            "pic" => cnv_id(e, "nvPicPr"),
            "graphicFrame" => cnv_id(e, "nvGraphicFramePr"),
            _ => None,
        };
        let Some(id) = id else {
            return;
        };
        if !ids_match(&id, spid) {
            return;
        }
        if let Some(body) = e.child_by_local("txBody") {
            n = count_p(body);
        }
    });
    n
}

fn count_p(el: &Element) -> u32 {
    let mut n = 0u32;
    crate::xml::walk_elements(el, &mut |node| {
        if node.local_name() == "p" {
            n += 1;
        }
    });
    n
}

fn rewrite_type(
    timing: &mut Element,
    path: &[usize],
    preset: Preset,
    dur: Option<u32>,
    para: Option<u32>,
    _op_index: usize,
) -> Result<(), Error> {
    let effect = get_at(timing, path).expect("效果路径");
    let spid = spid_at(timing, path).unwrap_or("2").to_string();
    let node_type = effect
        .attr_local("nodeType")
        .unwrap_or("clickEffect")
        .to_string();
    let delay = match effect_delay_ms(effect) {
        Value::Number(n) => n.as_u64().unwrap_or(0) as u32,
        _ => 0,
    };
    let dur = dur.unwrap_or_else(|| match effect_dur_ms(effect) {
        Value::Number(n) => n.as_u64().unwrap_or(500) as u32,
        _ => 500,
    });
    let para = para.or_else(|| effect_para(effect));
    let mut next = max_ctn_id(timing).saturating_add(1);
    let ids = take_ids(&mut next, preset.behavior_ids());
    let effect = get_mut_at(timing, path);
    anim_preset::fill_effect(effect, preset, &ids, &spid, &node_type, delay, dur, para);
    Ok(())
}

fn effect_para(effect: &Element) -> Option<u32> {
    let mut para = None;
    walk_effect_body(effect, &mut |e| {
        if para.is_none() && e.local_name() == "pRg" {
            para = e
                .attr_local("st")
                .and_then(|s| s.parse::<u32>().ok())
                .map(|n| n + 1);
        }
    });
    para
}

fn apply_para_to_tgt(el: &mut Element, para: Option<u32>) {
    if el.local_name() == "spTgt" {
        el.children.retain(|n| match n {
            Node::Element(e) => e.local_name() != "txEl",
            _ => true,
        });
        if let Some(n) = para {
            let st = n.saturating_sub(1);
            let mut tx = Element::new("p:txEl");
            tx.push_element(
                Element::new("p:pRg")
                    .with_attr("st", st.to_string())
                    .with_attr("end", st.to_string()),
            );
            el.push_element(tx);
        }
    }
    for n in &mut el.children {
        if let Node::Element(c) = n {
            apply_para_to_tgt(c, para);
        }
    }
}

fn set_click_on_spid(timing: &mut Element, effect_path: &[usize], spid: &str) {
    let Some(seq_path) = interactive_seq_path(timing, effect_path) else {
        return;
    };
    let seq = get_mut_at(timing, &seq_path);
    patch_click_spid(seq, spid);
}

fn interactive_seq_path(timing: &Element, effect_path: &[usize]) -> Option<Vec<usize>> {
    let mut cur = Vec::new();
    for &i in effect_path {
        cur.push(i);
        if let Some(e) = get_at(timing, &cur) {
            if e.local_name() == "cTn" && e.attr_local("nodeType") == Some("interactiveSeq") {
                return Some(cur);
            }
        }
    }
    None
}

fn patch_click_spid(el: &mut Element, spid: &str) {
    if el.local_name() == "cond" && el.attr_local("evt") == Some("onClick") {
        if let Some(tgt) = el.child_by_local_mut("tgtEl") {
            if let Some(sp) = tgt.child_by_local_mut("spTgt") {
                sp.set_attr("spid", spid);
            }
        }
    }
    for n in &mut el.children {
        if let Node::Element(c) = n {
            if is_effect(c) {
                continue;
            }
            patch_click_spid(c, spid);
        }
    }
}

fn reorder_effect(timing: &mut Element, from: usize, to: usize) {
    let paths = effect_paths(timing);
    if from >= paths.len() || to >= paths.len() || from == to {
        return;
    }
    let from_path = paths[from].clone();
    let Some(unit) = take_unit(timing, &from_path) else {
        return;
    };
    strip_idle_pars(timing);
    shape::prune_anim(timing);
    prune_empty_seqs(timing);
    let paths = effect_paths(timing);
    if paths.is_empty() {
        let seq_id = main_seq_id(timing).unwrap_or(2);
        let mut next = max_ctn_id(timing).saturating_add(1);
        if next < 1 {
            next = 1;
        }
        let group = wrap_par_click_group(unit, &take_ids(&mut next, 2), false, seq_id);
        ensure_main_seq_lst(timing).push_element(group);
        return;
    }
    let dest_idx = if from < to { to - 1 } else { to };
    let dest_idx = dest_idx.min(paths.len() - 1);
    insert_unit(timing, &paths[dest_idx], unit, from < to);
}

fn unit_path(timing: &Element, effect_path: &[usize]) -> Vec<usize> {
    if let Some(ia) = interactive_seq_path(timing, effect_path) {
        if !ia.is_empty() {
            let parent = &ia[..ia.len() - 1];
            if is_local_at(timing, parent, "seq") {
                return parent.to_vec();
            }
        }
        return ia;
    }
    if let Some(g) = click_group_ctn_path(timing, effect_path) {
        if !g.is_empty() {
            let group_par = &g[..g.len() - 1];
            if get_at(timing, group_par).is_some_and(|el| effect_count_under(el) <= 1) {
                return group_par.to_vec();
            }
        }
    }
    if !effect_path.is_empty() && is_local_at(timing, &effect_path[..effect_path.len() - 1], "par")
    {
        return effect_path[..effect_path.len() - 1].to_vec();
    }
    effect_path.to_vec()
}

fn effect_count_under(el: &Element) -> usize {
    let mut n = 0usize;
    shape::walk_choice(el, &mut |e| {
        if is_effect(e) {
            n += 1;
        }
    });
    n
}

fn take_unit(timing: &mut Element, effect_path: &[usize]) -> Option<Element> {
    let u = unit_path(timing, effect_path);
    let el = get_at(timing, &u)?.clone();
    remove_at(timing, &u);
    Some(el)
}

fn insert_unit(timing: &mut Element, dest_effect_path: &[usize], unit: Element, after: bool) {
    let dest_unit = unit_path(timing, dest_effect_path);
    if dest_unit.is_empty() {
        ensure_main_seq_lst(timing).push_element(unit);
        return;
    }
    let (parent_path, last) = dest_unit.split_at(dest_unit.len() - 1);
    let parent = if parent_path.is_empty() {
        timing
    } else {
        get_mut_at(timing, parent_path)
    };
    let at = if after { last[0] + 1 } else { last[0] };
    let at = at.min(parent.children.len());
    parent.children.insert(at, Node::Element(unit));
}

fn effect_path_by_ctn_id(timing: &Element, id: &str) -> Option<Vec<usize>> {
    effect_paths(timing).into_iter().find(|p| {
        get_at(timing, p)
            .and_then(|e| e.attr_local("id"))
            .is_some_and(|v| v == id)
    })
}

fn attach_sound(
    pkg: &mut Package,
    root: &mut Element,
    part: &str,
    file: &str,
    op_index: usize,
) -> Result<(), Error> {
    let paths = first_timing(root).map(effect_paths).unwrap_or_default();
    let Some(path) = paths.last().cloned() else {
        return Ok(());
    };
    attach_sound_on(pkg, root, part, &path, file, op_index)
}

fn first_timing(root: &Element) -> Option<&Element> {
    first_timing_index(root).and_then(|i| match &root.children[i] {
        Node::Element(e) => Some(e),
        _ => None,
    })
}

fn attach_sound_on(
    pkg: &mut Package,
    root: &mut Element,
    part: &str,
    path: &[usize],
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
    let Some(timing) = first_timing_mut(root) else {
        return Ok(());
    };
    let effect = get_mut_at(timing, path);
    effect.children.retain(|n| match n {
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
    effect.push_element(ac);
    Ok(())
}

fn strip_sound(
    pkg: &mut Package,
    root: &mut Element,
    part: &str,
    path: &[usize],
) -> Result<(), Error> {
    let Some(timing) = first_timing(root) else {
        return Ok(());
    };
    let Some(effect) = get_at(timing, path) else {
        return Ok(());
    };
    let mut rids = Vec::new();
    collect_snd_rids(effect, &mut rids);
    let Some(timing) = first_timing_mut(root) else {
        return Ok(());
    };
    let effect = get_mut_at(timing, path);
    effect.children.retain(|n| match n {
        Node::Element(e) => e.local_name() != "sndAc",
        _ => true,
    });
    if rids.is_empty() {
        return Ok(());
    }
    drop_unused_audio(pkg, root, part, &rids)
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
    let still = {
        let mut left = Vec::new();
        if let Some(timing) = first_timing(root) {
            collect_snd_rids(timing, &mut left);
        }
        left
    };
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
