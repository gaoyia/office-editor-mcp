//! 幻灯片 `<p:timing>`：效果摘要与预设效果增删。不接受整棵时间轴 JSON。

use std::collections::HashMap;

use serde_json::{json, Map, Value};

use crate::chart::{CHARTEX_NS, CHART_NS};
use crate::error::{Error, ErrorKind};
use crate::opc::Package;
use crate::smartart::DGM_NS;
use crate::xml::{Element, Node};

use super::master;
use super::notes;
use super::ole::{OLE_URI, OLE_URI_ISO};
use super::request::AnimTarget;
use super::shape::{self, SlideMeta};
use super::table::TABLE_URI;

const EFFECT_TYPES: &[&str] = &["clickEffect", "withEffect", "afterEffect"];
const PRESET_CLASS: &[&str] = &["entr", "exit", "emph", "path", "verb", "mediacall"];

pub fn list(root: &Element, host: &str, placeholders_only: bool) -> Vec<Value> {
    let objects = object_map(root, host, placeholders_only);
    let mut out = Vec::new();
    walk_timings(root, &mut |timing| {
        shape::walk_choice(timing, &mut |e| {
            if is_effect(e) {
                out.push(effect_json(e, &objects, host, out.len() + 1));
            }
        });
    });
    out
}

pub fn insert(
    pkg: &mut Package,
    slides: &[SlideMeta],
    pres_part: &str,
    pres_root: &Element,
    host_id: &str,
    target: &AnimTarget,
    trigger: &str,
    delay: u32,
    dur: u32,
    op_index: usize,
) -> Result<(), Error> {
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
    let node_type = node_type_of(trigger);
    match first_timing_mut(&mut doc.root) {
        None => {
            let timing = build_full_timing(&spid, node_type, 1, delay, dur, trigger != "click");
            let at = timing_insert_index(&doc.root);
            doc.root.children.insert(at, Node::Element(timing));
        }
        Some(timing) => {
            append_effect(timing, &spid, node_type, trigger, delay, dur);
        }
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
        let path = &paths[index - 1];
        let spid = spid_at(timing, path).map(str::to_string);
        remove_effect_at(timing, path);
        strip_idle_pars(timing);
        shape::prune_anim(timing);
        if let Some(spid) = spid {
            if !spid_still_used(timing, &spid) {
                strip_bld_p(timing, &spid);
            }
        }
        strip_idle_pars(timing);
        shape::prune_anim(timing);
        !has_effect(timing)
    };
    if drop_timing {
        doc.root.children.remove(timing_idx);
    }
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
    trigger: Option<&str>,
    delay: Option<u32>,
    dur: Option<u32>,
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
    let paths = effect_paths(timing);
    if index < 1 || index > paths.len() {
        return Err(Error::with_path(
            ErrorKind::AnimNotFound { id: id.to_string() },
            format!("ops[{op_index}].id"),
        ));
    }
    let path = paths[index - 1].clone();
    if let Some(delay) = delay {
        set_effect_delay(get_mut_at(timing, &path), delay);
    }
    if let Some(dur) = dur {
        set_effect_dur(get_mut_at(timing, &path), dur, op_index)?;
    }
    if let Some(trigger) = trigger {
        set_effect_trigger(timing, &path, index == 1, trigger);
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
    }
}

fn walk_timings<'a>(el: &'a Element, f: &mut impl FnMut(&'a Element)) {
    if el.local_name() == "AlternateContent" {
        if let Some(i) = shape::first_alt_child_index(el) {
            if let Some(Node::Element(child)) = el.children.get(i) {
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

fn effect_json(effect: &Element, objects: &HashMap<String, String>, host: &str, k: usize) -> Value {
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
    obj.insert("id".into(), json!(format!("{host}/anim:{k}")));
    obj.insert("target".into(), target);
    obj.insert("type".into(), json!(type_name));
    obj.insert("trigger".into(), json!(trigger));
    obj.insert("delay".into(), effect_delay_ms(effect));
    obj.insert("dur".into(), effect_dur_ms(effect));
    Value::Object(obj)
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
        .find(|e| e.local_name() == "cond")
        .and_then(|c| c.attr_local("delay"));
    tl_time_ms(delay, 0)
}

fn effect_dur_ms(effect: &Element) -> Value {
    let mut found = None;
    walk_effect_body(effect, &mut |e| {
        if found.is_some() || e.local_name() != "animEffect" {
            return;
        }
        found = Some(
            e.child_by_local("cBhvr")
                .and_then(|b| b.child_by_local("cTn"))
                .and_then(|c| c.attr_local("dur"))
                .map(str::to_string),
        );
    });
    match found {
        None => json!(500),
        Some(inner) => tl_time_ms(inner.as_deref(), 500),
    }
}

fn walk_effect_body<'a>(effect: &'a Element, f: &mut impl FnMut(&'a Element)) {
    fn rec<'a>(el: &'a Element, f: &mut impl FnMut(&'a Element), skip_nested: bool) {
        if el.local_name() == "AlternateContent" {
            if let Some(i) = shape::first_alt_child_index(el) {
                if let Some(Node::Element(child)) = el.children.get(i) {
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

fn object_map(root: &Element, host: &str, placeholders_only: bool) -> HashMap<String, String> {
    let mut map = HashMap::new();
    shape::walk_choice(root, &mut |e| {
        if placeholders_only {
            if is_placeholder_sp(e) {
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

fn is_placeholder_sp(el: &Element) -> bool {
    el.local_name() == "sp"
        && el
            .child_by_local("nvSpPr")
            .and_then(|nv| nv.child_by_local("nvPr"))
            .and_then(|nv| nv.child_by_local("ph"))
            .is_some()
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
    spid: &str,
    node_type: &str,
    trigger: &str,
    delay: u32,
    dur: u32,
) {
    let mut next = max_ctn_id(timing).saturating_add(1);
    if next < 1 {
        next = 1;
    }
    let click_like = trigger != "with" && trigger != "after";
    let has_any = has_effect(timing);
    ensure_main_seq_lst(timing);
    let seq_id = main_seq_id(timing).unwrap_or(2);
    if click_like || !has_any {
        let auto = !has_any && trigger != "click";
        let group = build_click_group(
            &take_ids(&mut next, 5),
            spid,
            node_type,
            delay,
            dur,
            auto,
            seq_id,
        );
        ensure_main_seq_lst(timing).push_element(group);
    } else if let Some(path) = effect_paths(timing).last().cloned() {
        // with / after：接到最后一条效果所在 par 的兄弟位置。
        let sibling = fade_effect_par(&take_ids(&mut next, 3), spid, node_type, delay, dur);
        append_sibling_par(timing, &path, sibling);
    } else {
        let group = build_click_group(
            &take_ids(&mut next, 5),
            spid,
            node_type,
            delay,
            dur,
            trigger != "click",
            seq_id,
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
    spid: &str,
    node_type: &str,
    start: u32,
    delay: u32,
    dur: u32,
    auto: bool,
) -> Element {
    let mut next = start;
    let root_id = take_ids(&mut next, 1)[0];
    let seq_id = take_ids(&mut next, 1)[0];
    let group = build_click_group(
        &take_ids(&mut next, 5),
        spid,
        node_type,
        delay,
        dur,
        auto,
        seq_id,
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

fn build_click_group(
    ids: &[u32],
    spid: &str,
    node_type: &str,
    delay: u32,
    dur: u32,
    auto: bool,
    main_seq_id: u32,
) -> Element {
    let effect = fade_effect_par(&ids[2..5], spid, node_type, delay, dur);

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

fn fade_effect_par(ids: &[u32], spid: &str, node_type: &str, delay: u32, dur: u32) -> Element {
    let mut name = Element::new("p:attrName");
    name.set_text("style.visibility");
    let mut names = Element::new("p:attrNameLst");
    names.push_element(name);

    let mut set_bhvr = Element::new("p:cBhvr");
    set_bhvr.push_element(
        Element::new("p:cTn")
            .with_attr("id", ids[1].to_string())
            .with_attr("dur", "1")
            .with_attr("fill", "hold"),
    );
    set_bhvr.push_element(tgt_el(spid));
    set_bhvr.push_element(names);
    let mut to = Element::new("p:to");
    to.push_element(Element::new("p:strVal").with_attr("val", "visible"));
    let mut set = Element::new("p:set");
    set.push_element(set_bhvr);
    set.push_element(to);

    let mut anim_bhvr = Element::new("p:cBhvr");
    anim_bhvr.push_element(
        Element::new("p:cTn")
            .with_attr("id", ids[2].to_string())
            .with_attr("dur", dur.to_string()),
    );
    anim_bhvr.push_element(tgt_el(spid));
    let mut anim = Element::new("p:animEffect")
        .with_attr("transition", "in")
        .with_attr("filter", "fade");
    anim.push_element(anim_bhvr);

    let mut kids = Element::new("p:childTnLst");
    kids.push_element(set);
    kids.push_element(anim);
    let mut ctn = Element::new("p:cTn")
        .with_attr("id", ids[0].to_string())
        .with_attr("presetID", "10")
        .with_attr("presetClass", "entr")
        .with_attr("presetSubtype", "0")
        .with_attr("fill", "hold")
        .with_attr("grpId", "0")
        .with_attr("nodeType", node_type);
    ctn.push_element(cond_lst(&delay.to_string()));
    ctn.push_element(kids);
    let mut par = Element::new("p:par");
    par.push_element(ctn);
    par
}

fn tgt_el(spid: &str) -> Element {
    let mut tgt = Element::new("p:tgtEl");
    tgt.push_element(Element::new("p:spTgt").with_attr("spid", spid));
    tgt
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

fn walk_all<'a>(el: &'a Element, f: &mut impl FnMut(&'a Element)) {
    if el.local_name() == "AlternateContent" {
        if let Some(i) = shape::first_alt_child_index(el) {
            if let Some(Node::Element(child)) = el.children.get(i) {
                walk_all(child, f);
            }
        }
        return;
    }
    f(el);
    for c in el.element_children() {
        walk_all(c, f);
    }
}

fn ensure_main_seq_lst(timing: &mut Element) -> &mut Element {
    if let Some(path) = find_ctn_path(timing, "mainSeq") {
        let ctn = get_mut_at(timing, &path);
        return ensure_child(ctn, "p:childTnLst");
    }
    // 有 timing 却没有 mainSeq：在 tnLst 下补一套。
    let mut next = max_ctn_id(timing).saturating_add(1);
    if next < 1 {
        next = 1;
    }
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
    fn rec(el: &Element, want: &str, path: &mut Vec<usize>) -> bool {
        if el.local_name() == "AlternateContent" {
            if let Some(i) = shape::first_alt_child_index(el) {
                if let Some(Node::Element(child)) = el.children.get(i) {
                    path.push(i);
                    if rec(child, want, path) {
                        return true;
                    }
                    path.pop();
                }
            }
            return false;
        }
        if el.local_name() == "cTn" && el.attr_local("nodeType") == Some(want) {
            return true;
        }
        for (i, n) in el.children.iter().enumerate() {
            if let Node::Element(c) = n {
                path.push(i);
                if rec(c, want, path) {
                    return true;
                }
                path.pop();
            }
        }
        false
    }
    let mut path = Vec::new();
    if rec(el, node_type, &mut path) {
        Some(path)
    } else {
        None
    }
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
    let mut path = Vec::new();
    walk_choice_paths(timing, &mut path, &mut |p, e| {
        if is_effect(e) {
            out.push(p.to_vec());
        }
    });
    out
}

fn walk_choice_paths<'a>(
    el: &'a Element,
    path: &mut Vec<usize>,
    f: &mut impl FnMut(&[usize], &'a Element),
) {
    if el.local_name() == "AlternateContent" {
        if let Some(i) = shape::first_alt_child_index(el) {
            if let Some(Node::Element(child)) = el.children.get(i) {
                path.push(i);
                walk_choice_paths(child, path, f);
                path.pop();
            }
        }
        return;
    }
    f(path, el);
    for (i, n) in el.children.iter().enumerate() {
        if let Node::Element(c) = n {
            path.push(i);
            walk_choice_paths(c, path, f);
            path.pop();
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
            "timing" | "hf" | "txStyles" | "matchingName" | "extLst" => return i,
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
    let mut path = None;
    find_anim_effect_dur_path(effect, &mut Vec::new(), &mut path);
    let Some(path) = path else {
        return Err(Error::with_path(
            ErrorKind::AnimNeedEffectDur,
            format!("ops[{op_index}].dur"),
        ));
    };
    get_mut_at(effect, &path).set_attr("dur", dur.to_string());
    Ok(())
}

fn find_anim_effect_dur_path(el: &Element, path: &mut Vec<usize>, found: &mut Option<Vec<usize>>) {
    if found.is_some() {
        return;
    }
    if el.local_name() == "AlternateContent" {
        if let Some(i) = shape::first_alt_child_index(el) {
            if let Some(Node::Element(child)) = el.children.get(i) {
                path.push(i);
                find_anim_effect_dur_path(child, path, found);
                path.pop();
            }
        }
        return;
    }
    if el.local_name() == "animEffect" {
        if let Some((i, ctn)) = el.children.iter().enumerate().find_map(|(i, n)| match n {
            Node::Element(e) if e.local_name() == "cBhvr" => Some((i, e)),
            _ => None,
        }) {
            if let Some((j, _)) = ctn.children.iter().enumerate().find_map(|(j, n)| match n {
                Node::Element(e) if e.local_name() == "cTn" => Some((j, e)),
                _ => None,
            }) {
                let mut p = path.clone();
                p.push(i);
                p.push(j);
                *found = Some(p);
            }
        }
        return;
    }
    if is_effect(el) && !path.is_empty() {
        return;
    }
    for (i, n) in el.children.iter().enumerate() {
        if let Node::Element(c) = n {
            path.push(i);
            find_anim_effect_dur_path(c, path, found);
            path.pop();
            if found.is_some() {
                return;
            }
        }
    }
}

fn set_effect_trigger(timing: &mut Element, path: &[usize], is_first: bool, trigger: &str) {
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
