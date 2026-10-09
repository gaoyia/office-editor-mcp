//! 动画预设模板：对照 PowerPoint 16.0 另存的最小树。不接受整棵时间轴 JSON。

use crate::xml::{Element, Node};

#[derive(Clone, Copy)]
pub struct Preset {
    pub name: &'static str,
    pub class: &'static str,
    pub preset_id: u32,
    pub subtype: u32,
    pub kind: Kind,
}

#[derive(Clone, Copy)]
pub enum Kind {
    Appear,
    EffectIn { filter: &'static str },
    EffectOut { filter: &'static str },
    Fly { out: bool },
    Zoom,
    Spin,
    GrowShrink,
    PathRight,
}

const PRESETS: &[Preset] = &[
    Preset {
        name: "appear",
        class: "entr",
        preset_id: 1,
        subtype: 0,
        kind: Kind::Appear,
    },
    Preset {
        name: "fade",
        class: "entr",
        preset_id: 10,
        subtype: 0,
        kind: Kind::EffectIn { filter: "fade" },
    },
    Preset {
        name: "flyIn",
        class: "entr",
        preset_id: 2,
        subtype: 4,
        kind: Kind::Fly { out: false },
    },
    Preset {
        name: "wipe",
        class: "entr",
        preset_id: 22,
        subtype: 4,
        kind: Kind::EffectIn {
            filter: "wipe(down)",
        },
    },
    Preset {
        name: "split",
        class: "entr",
        preset_id: 16,
        subtype: 21,
        kind: Kind::EffectIn {
            filter: "barn(inVertical)",
        },
    },
    Preset {
        name: "zoom",
        class: "entr",
        preset_id: 23,
        subtype: 16,
        kind: Kind::Zoom,
    },
    Preset {
        name: "spin",
        class: "emph",
        preset_id: 8,
        subtype: 0,
        kind: Kind::Spin,
    },
    Preset {
        name: "growShrink",
        class: "emph",
        preset_id: 6,
        subtype: 0,
        kind: Kind::GrowShrink,
    },
    Preset {
        name: "fadeOut",
        class: "exit",
        preset_id: 10,
        subtype: 0,
        kind: Kind::EffectOut { filter: "fade" },
    },
    Preset {
        name: "flyOut",
        class: "exit",
        preset_id: 2,
        subtype: 4,
        kind: Kind::Fly { out: true },
    },
    Preset {
        name: "wipeOut",
        class: "exit",
        preset_id: 22,
        subtype: 4,
        kind: Kind::EffectOut {
            filter: "wipe(down)",
        },
    },
    Preset {
        name: "pathRight",
        class: "path",
        preset_id: 44,
        subtype: 0,
        kind: Kind::PathRight,
    },
];

const PATH_RIGHT: &str = "M 0 0 L 0.067 -0.04 C 0.081 -0.049 0.102 -0.054 0.124 -0.054 C 0.149 -0.054 0.169 -0.049 0.183 -0.04 L 0.25 0 E";

pub fn by_name(name: &str) -> Option<Preset> {
    PRESETS.iter().copied().find(|p| p.name == name)
}

pub fn by_class_id(class: &str, preset_id: u32) -> Option<Preset> {
    PRESETS
        .iter()
        .copied()
        .find(|p| p.class == class && p.preset_id == preset_id)
}

impl Preset {
    pub fn behavior_ids(self) -> usize {
        match self.kind {
            Kind::Appear | Kind::Spin | Kind::GrowShrink | Kind::PathRight => 1,
            Kind::EffectIn { .. } | Kind::EffectOut { .. } => 2,
            Kind::Fly { .. } | Kind::Zoom => 3,
        }
    }
}

pub fn effect_par(
    preset: Preset,
    ids: &[u32],
    spid: &str,
    node_type: &str,
    delay: u32,
    dur: u32,
    para: Option<u32>,
) -> Element {
    let mut ctn = effect_ctn(preset, ids[0], node_type, delay);
    let mut kids = Element::new("p:childTnLst");
    for b in behaviors(preset, &ids[1..], spid, dur, para) {
        kids.push_element(b);
    }
    ctn.push_element(kids);
    let mut par = Element::new("p:par");
    par.push_element(ctn);
    par
}

pub fn fill_effect(
    effect: &mut Element,
    preset: Preset,
    ids: &[u32],
    spid: &str,
    node_type: &str,
    delay: u32,
    dur: u32,
    para: Option<u32>,
) {
    effect.set_attr("presetID", preset.preset_id.to_string());
    effect.set_attr("presetClass", preset.class);
    effect.set_attr("presetSubtype", preset.subtype.to_string());
    effect.set_attr("fill", "hold");
    effect.set_attr("grpId", "0");
    effect.set_attr("nodeType", node_type);
    if matches!(preset.kind, Kind::PathRight) {
        effect.set_attr("accel", "50000");
        effect.set_attr("decel", "50000");
    } else {
        effect.remove_attr("accel");
        effect.remove_attr("decel");
    }
    // 保留 stCondLst / sndAc，只换子树。
    let delay_lst = effect
        .children
        .iter()
        .position(|n| matches!(n, Node::Element(e) if e.local_name() == "stCondLst"));
    if delay_lst.is_none() {
        effect
            .children
            .insert(0, Node::Element(cond_lst(&delay.to_string())));
    }
    effect.children.retain(|n| match n {
        Node::Element(e) => e.local_name() != "childTnLst",
        _ => true,
    });
    let mut kids = Element::new("p:childTnLst");
    for b in behaviors(preset, ids, spid, dur, para) {
        kids.push_element(b);
    }
    let at = effect
        .children
        .iter()
        .position(|n| matches!(n, Node::Element(e) if e.local_name() == "sndAc"))
        .unwrap_or(effect.children.len());
    effect.children.insert(at, Node::Element(kids));
}

fn effect_ctn(preset: Preset, id: u32, node_type: &str, delay: u32) -> Element {
    let mut ctn = Element::new("p:cTn")
        .with_attr("id", id.to_string())
        .with_attr("presetID", preset.preset_id.to_string())
        .with_attr("presetClass", preset.class)
        .with_attr("presetSubtype", preset.subtype.to_string())
        .with_attr("fill", "hold")
        .with_attr("grpId", "0")
        .with_attr("nodeType", node_type);
    if matches!(preset.kind, Kind::PathRight) {
        ctn.set_attr("accel", "50000");
        ctn.set_attr("decel", "50000");
    }
    ctn.push_element(cond_lst(&delay.to_string()));
    ctn
}

fn behaviors(preset: Preset, ids: &[u32], spid: &str, dur: u32, para: Option<u32>) -> Vec<Element> {
    match preset.kind {
        Kind::Appear => vec![vis_set(ids[0], spid, para, "visible", None)],
        Kind::EffectIn { filter } => vec![
            vis_set(ids[0], spid, para, "visible", None),
            anim_effect(ids[1], spid, para, dur, "in", filter),
        ],
        Kind::EffectOut { filter } => vec![
            anim_effect(ids[0], spid, para, dur, "out", filter),
            vis_set(ids[1], spid, para, "hidden", Some(dur.saturating_sub(1))),
        ],
        Kind::Fly { out: false } => vec![
            vis_set(ids[0], spid, para, "visible", None),
            ppt_anim(ids[1], spid, para, dur, "ppt_x", "#ppt_x", "#ppt_x", true),
            ppt_anim(
                ids[2],
                spid,
                para,
                dur,
                "ppt_y",
                "1+#ppt_h/2",
                "#ppt_y",
                true,
            ),
        ],
        Kind::Fly { out: true } => vec![
            ppt_anim(ids[0], spid, para, dur, "ppt_x", "#ppt_x", "#ppt_x", true),
            ppt_anim(
                ids[1],
                spid,
                para,
                dur,
                "ppt_y",
                "#ppt_y",
                "1+#ppt_h/2",
                true,
            ),
            vis_set(ids[2], spid, para, "hidden", Some(dur.saturating_sub(1))),
        ],
        Kind::Zoom => vec![
            vis_set(ids[0], spid, para, "visible", None),
            size_anim(ids[1], spid, para, dur, "ppt_w"),
            size_anim(ids[2], spid, para, dur, "ppt_h"),
        ],
        Kind::Spin => vec![anim_rot(ids[0], spid, para, dur)],
        Kind::GrowShrink => vec![anim_scale(ids[0], spid, para, dur)],
        Kind::PathRight => vec![anim_motion(ids[0], spid, para, dur)],
    }
}

pub fn tgt_el(spid: &str, para: Option<u32>) -> Element {
    let mut sp = Element::new("p:spTgt").with_attr("spid", spid);
    if let Some(n) = para {
        let st = n.saturating_sub(1);
        let mut tx = Element::new("p:txEl");
        tx.push_element(
            Element::new("p:pRg")
                .with_attr("st", st.to_string())
                .with_attr("end", st.to_string()),
        );
        sp.push_element(tx);
    }
    let mut tgt = Element::new("p:tgtEl");
    tgt.push_element(sp);
    tgt
}

fn vis_set(id: u32, spid: &str, para: Option<u32>, to: &str, delay: Option<u32>) -> Element {
    let mut name = Element::new("p:attrName");
    name.set_text("style.visibility");
    let mut names = Element::new("p:attrNameLst");
    names.push_element(name);
    let mut ctn = Element::new("p:cTn")
        .with_attr("id", id.to_string())
        .with_attr("dur", "1")
        .with_attr("fill", "hold");
    ctn.push_element(cond_lst(&delay.unwrap_or(0).to_string()));
    let mut bhvr = Element::new("p:cBhvr");
    bhvr.push_element(ctn);
    bhvr.push_element(tgt_el(spid, para));
    bhvr.push_element(names);
    let mut to_el = Element::new("p:to");
    to_el.push_element(Element::new("p:strVal").with_attr("val", to));
    let mut set = Element::new("p:set");
    set.push_element(bhvr);
    set.push_element(to_el);
    set
}

fn anim_effect(
    id: u32,
    spid: &str,
    para: Option<u32>,
    dur: u32,
    transition: &str,
    filter: &str,
) -> Element {
    let mut bhvr = Element::new("p:cBhvr");
    bhvr.push_element(
        Element::new("p:cTn")
            .with_attr("id", id.to_string())
            .with_attr("dur", dur.to_string()),
    );
    bhvr.push_element(tgt_el(spid, para));
    let mut anim = Element::new("p:animEffect")
        .with_attr("transition", transition)
        .with_attr("filter", filter);
    anim.push_element(bhvr);
    anim
}

fn ppt_anim(
    id: u32,
    spid: &str,
    para: Option<u32>,
    dur: u32,
    attr: &str,
    from: &str,
    to: &str,
    additive: bool,
) -> Element {
    let ctn = Element::new("p:cTn")
        .with_attr("id", id.to_string())
        .with_attr("dur", dur.to_string())
        .with_attr("fill", "hold");
    let mut bhvr = Element::new("p:cBhvr");
    if additive {
        bhvr.set_attr("additive", "base");
    }
    bhvr.push_element(ctn);
    bhvr.push_element(tgt_el(spid, para));
    let mut names = Element::new("p:attrNameLst");
    let mut name = Element::new("p:attrName");
    name.set_text(attr);
    names.push_element(name);
    bhvr.push_element(names);
    let mut tavs = Element::new("p:tavLst");
    tavs.push_element(tav("0", from));
    tavs.push_element(tav("100000", to));
    let mut anim = Element::new("p:anim")
        .with_attr("calcmode", "lin")
        .with_attr("valueType", "num");
    anim.push_element(bhvr);
    anim.push_element(tavs);
    anim
}

fn size_anim(id: u32, spid: &str, para: Option<u32>, dur: u32, attr: &str) -> Element {
    let ctn = Element::new("p:cTn")
        .with_attr("id", id.to_string())
        .with_attr("dur", dur.to_string())
        .with_attr("fill", "hold");
    let mut bhvr = Element::new("p:cBhvr");
    bhvr.push_element(ctn);
    bhvr.push_element(tgt_el(spid, para));
    let mut names = Element::new("p:attrNameLst");
    let mut name = Element::new("p:attrName");
    name.set_text(attr);
    names.push_element(name);
    bhvr.push_element(names);
    let mut tavs = Element::new("p:tavLst");
    let mut t0 = Element::new("p:tav").with_attr("tm", "0");
    let mut v0 = Element::new("p:val");
    v0.push_element(Element::new("p:fltVal").with_attr("val", "0"));
    t0.push_element(v0);
    tavs.push_element(t0);
    tavs.push_element(tav("100000", &format!("#{attr}")));
    let mut anim = Element::new("p:anim")
        .with_attr("calcmode", "lin")
        .with_attr("valueType", "num");
    anim.push_element(bhvr);
    anim.push_element(tavs);
    anim
}

fn tav(tm: &str, val: &str) -> Element {
    let mut tav = Element::new("p:tav").with_attr("tm", tm);
    let mut v = Element::new("p:val");
    v.push_element(Element::new("p:strVal").with_attr("val", val));
    tav.push_element(v);
    tav
}

fn anim_rot(id: u32, spid: &str, para: Option<u32>, dur: u32) -> Element {
    let ctn = Element::new("p:cTn")
        .with_attr("id", id.to_string())
        .with_attr("dur", dur.to_string())
        .with_attr("fill", "hold");
    let mut bhvr = Element::new("p:cBhvr");
    bhvr.push_element(ctn);
    bhvr.push_element(tgt_el(spid, para));
    let mut names = Element::new("p:attrNameLst");
    let mut name = Element::new("p:attrName");
    name.set_text("r");
    names.push_element(name);
    bhvr.push_element(names);
    let mut rot = Element::new("p:animRot").with_attr("by", "21600000");
    rot.push_element(bhvr);
    rot
}

fn anim_scale(id: u32, spid: &str, para: Option<u32>, dur: u32) -> Element {
    let ctn = Element::new("p:cTn")
        .with_attr("id", id.to_string())
        .with_attr("dur", dur.to_string())
        .with_attr("fill", "hold");
    let mut bhvr = Element::new("p:cBhvr");
    bhvr.push_element(ctn);
    bhvr.push_element(tgt_el(spid, para));
    let by = Element::new("p:by")
        .with_attr("x", "150000")
        .with_attr("y", "150000");
    let mut scale = Element::new("p:animScale");
    scale.push_element(bhvr);
    scale.push_element(by);
    scale
}

fn anim_motion(id: u32, spid: &str, para: Option<u32>, dur: u32) -> Element {
    let ctn = Element::new("p:cTn")
        .with_attr("id", id.to_string())
        .with_attr("dur", dur.to_string())
        .with_attr("fill", "hold");
    let mut bhvr = Element::new("p:cBhvr");
    bhvr.push_element(ctn);
    bhvr.push_element(tgt_el(spid, para));
    let mut names = Element::new("p:attrNameLst");
    for attr in ["ppt_x", "ppt_y"] {
        let mut name = Element::new("p:attrName");
        name.set_text(attr);
        names.push_element(name);
    }
    bhvr.push_element(names);
    let mut motion = Element::new("p:animMotion")
        .with_attr("origin", "layout")
        .with_attr("path", PATH_RIGHT)
        .with_attr("pathEditMode", "relative")
        .with_attr("ptsTypes", "");
    motion.push_element(bhvr);
    motion
}

fn cond_lst(delay: &str) -> Element {
    let mut lst = Element::new("p:stCondLst");
    lst.push_element(Element::new("p:cond").with_attr("delay", delay));
    lst
}
