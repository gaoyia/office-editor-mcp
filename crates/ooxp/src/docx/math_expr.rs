//! 结构化 OMML：`expr` 读写。不排版，不解析 UnicodeMath。

use serde_json::{json, Map, Value};

use crate::error::{Error, ErrorKind, TypeName};
use crate::xml::Element;

use super::math;

/// insertMath / setMath / create 的正文：线性字或节点树，恰好一个。
#[derive(Debug, Clone)]
pub enum MathBody {
    Text(String),
    Expr(Vec<MathNode>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FracType {
    Bar,
    Skw,
    Lin,
    NoBar,
}

impl FracType {
    fn as_json(self) -> Option<&'static str> {
        match self {
            Self::Bar => None,
            Self::Skw => Some("skw"),
            Self::Lin => Some("lin"),
            Self::NoBar => Some("noBar"),
        }
    }

    fn xml_val(self) -> Option<&'static str> {
        match self {
            Self::Bar => None,
            Self::Skw => Some("skw"),
            Self::Lin => Some("lin"),
            Self::NoBar => Some("noBar"),
        }
    }

    fn parse(s: &str) -> Option<Self> {
        match s {
            "bar" => Some(Self::Bar),
            "skw" => Some(Self::Skw),
            "lin" => Some(Self::Lin),
            "noBar" => Some(Self::NoBar),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum MathNode {
    Text {
        text: String,
        brk: bool,
    },
    Frac {
        frac_type: FracType,
        num: Vec<MathNode>,
        den: Vec<MathNode>,
    },
    Rad {
        deg_hide: bool,
        deg: Vec<MathNode>,
        e: Vec<MathNode>,
    },
    SSub {
        e: Vec<MathNode>,
        sub: Vec<MathNode>,
    },
    SSup {
        e: Vec<MathNode>,
        sup: Vec<MathNode>,
    },
    SSubSup {
        e: Vec<MathNode>,
        sub: Vec<MathNode>,
        sup: Vec<MathNode>,
    },
    SPre {
        e: Vec<MathNode>,
        sub: Vec<MathNode>,
        sup: Vec<MathNode>,
    },
    Nary {
        chr: Option<String>,
        sub: Vec<MathNode>,
        sup: Vec<MathNode>,
        e: Vec<MathNode>,
    },
    Delim {
        args: Vec<Vec<MathNode>>,
    },
    Func {
        f_name: Vec<MathNode>,
        e: Vec<MathNode>,
    },
    Mat {
        rows: Vec<Vec<Vec<MathNode>>>,
    },
    Acc {
        chr: Option<String>,
        e: Vec<MathNode>,
    },
    Bar {
        e: Vec<MathNode>,
    },
    LimLow {
        e: Vec<MathNode>,
        lim: Vec<MathNode>,
    },
    LimUpp {
        e: Vec<MathNode>,
        lim: Vec<MathNode>,
    },
    EqArr {
        rows: Vec<Vec<MathNode>>,
    },
    Phant {
        e: Vec<MathNode>,
        show: Option<bool>,
        zero_wid: Option<bool>,
        zero_asc: Option<bool>,
        zero_desc: Option<bool>,
        transp: Option<bool>,
    },
    BorderBox {
        e: Vec<MathNode>,
        hide_top: Option<bool>,
        hide_bot: Option<bool>,
        hide_left: Option<bool>,
        hide_right: Option<bool>,
    },
    Other {
        name: String,
        text: String,
    },
}

pub fn parse_math_body(obj: &Map<String, Value>, path: &str) -> Result<MathBody, Error> {
    let has_text = obj.contains_key("text");
    let has_expr = obj.contains_key("expr");
    match (has_text, has_expr) {
        (true, true) => Err(Error::with_path(
            ErrorKind::MutuallyExclusive {
                a: "text".into(),
                b: "expr".into(),
            },
            path.to_string(),
        )),
        (false, false) => Err(Error::with_path(
            ErrorKind::MathNeedTextOrExpr,
            path.to_string(),
        )),
        (true, false) => {
            let text = parse_nonempty_text(obj.get("text"), &format!("{path}.text"))?;
            Ok(MathBody::Text(text))
        }
        (false, true) => {
            let nodes = parse_node_array(obj.get("expr"), &format!("{path}.expr"), true)?;
            Ok(MathBody::Expr(nodes))
        }
    }
}

pub fn nodes_to_json(nodes: &[MathNode]) -> Vec<Value> {
    nodes.iter().map(node_to_json).collect()
}

pub fn expr_of_omath(om: &Element) -> Vec<Value> {
    nodes_to_json(&read_seq(om))
}

impl MathBody {
    pub fn to_omath(&self) -> Element {
        match self {
            Self::Text(t) => math::new_omath(t),
            Self::Expr(nodes) => {
                let mut om = Element::new("m:oMath");
                for n in nodes {
                    om.push_element(n.to_element());
                }
                om
            }
        }
    }

    pub fn to_omath_para_jc(&self, jc: Option<&str>) -> Element {
        let mut wrap = Element::new("m:oMathPara");
        if let Some(val) = jc {
            let mut jc_el = Element::new("m:jc");
            jc_el.set_attr("m:val", val);
            let mut pr = Element::new("m:oMathParaPr");
            pr.push_element(jc_el);
            wrap.push_element(pr);
        }
        wrap.push_element(self.to_omath());
        wrap
    }

    pub fn apply_to_omath(&self, om: &mut Element) {
        om.children.clear();
        match self {
            Self::Text(t) => om.push_element(math::new_math_run(t)),
            Self::Expr(nodes) => {
                for n in nodes {
                    om.push_element(n.to_element());
                }
            }
        }
    }
}

impl MathNode {
    fn to_element(&self) -> Element {
        match self {
            Self::Text { text, brk } => math::new_math_run_brk(text, *brk),
            Self::Frac {
                frac_type,
                num,
                den,
            } => {
                let mut f = Element::new("m:f");
                if let Some(val) = frac_type.xml_val() {
                    let mut ty = Element::new("m:type");
                    ty.set_attr("m:val", val);
                    let mut pr = Element::new("m:fPr");
                    pr.push_element(ty);
                    f.push_element(pr);
                }
                f.push_element(wrap_slot("m:num", num));
                f.push_element(wrap_slot("m:den", den));
                f
            }
            Self::Rad { deg_hide, deg, e } => {
                let mut rad = Element::new("m:rad");
                if *deg_hide {
                    let mut hide = Element::new("m:degHide");
                    hide.set_attr("m:val", "1");
                    let mut pr = Element::new("m:radPr");
                    pr.push_element(hide);
                    rad.push_element(pr);
                    rad.push_element(Element::new("m:deg"));
                } else if !deg.is_empty() {
                    rad.push_element(wrap_slot("m:deg", deg));
                }
                rad.push_element(wrap_slot("m:e", e));
                rad
            }
            Self::SSub { e, sub } => script("m:sSub", e, Some(sub), None),
            Self::SSup { e, sup } => script("m:sSup", e, None, Some(sup)),
            Self::SSubSup { e, sub, sup } => script("m:sSubSup", e, Some(sub), Some(sup)),
            Self::SPre { e, sub, sup } => script("m:sPre", e, Some(sub), Some(sup)),
            Self::Nary { chr, sub, sup, e } => {
                let mut nary = Element::new("m:nary");
                if let Some(c) = chr {
                    let mut ch = Element::new("m:chr");
                    ch.set_attr("m:val", c.as_str());
                    let mut pr = Element::new("m:naryPr");
                    pr.push_element(ch);
                    nary.push_element(pr);
                }
                if !sub.is_empty() {
                    nary.push_element(wrap_slot("m:sub", sub));
                }
                if !sup.is_empty() {
                    nary.push_element(wrap_slot("m:sup", sup));
                }
                nary.push_element(wrap_slot("m:e", e));
                nary
            }
            Self::Delim { args } => {
                let mut d = Element::new("m:d");
                for a in args {
                    d.push_element(wrap_slot("m:e", a));
                }
                d
            }
            Self::Func { f_name, e } => {
                let mut func = Element::new("m:func");
                func.push_element(wrap_slot("m:fName", f_name));
                func.push_element(wrap_slot("m:e", e));
                func
            }
            Self::Mat { rows } => {
                let mut m = Element::new("m:m");
                for row in rows {
                    let mut mr = Element::new("m:mr");
                    for cell in row {
                        mr.push_element(wrap_slot("m:e", cell));
                    }
                    m.push_element(mr);
                }
                m
            }
            Self::Acc { chr, e } => {
                let mut acc = Element::new("m:acc");
                if let Some(c) = chr {
                    let mut ch = Element::new("m:chr");
                    ch.set_attr("m:val", c.as_str());
                    let mut pr = Element::new("m:accPr");
                    pr.push_element(ch);
                    acc.push_element(pr);
                }
                acc.push_element(wrap_slot("m:e", e));
                acc
            }
            Self::Bar { e } => {
                let mut bar = Element::new("m:bar");
                bar.push_element(wrap_slot("m:e", e));
                bar
            }
            Self::LimLow { e, lim } => lim_el("m:limLow", e, lim),
            Self::LimUpp { e, lim } => lim_el("m:limUpp", e, lim),
            Self::EqArr { rows } => {
                let mut arr = Element::new("m:eqArr");
                for row in rows {
                    arr.push_element(wrap_slot("m:e", row));
                }
                arr
            }
            Self::Phant {
                e,
                show,
                zero_wid,
                zero_asc,
                zero_desc,
                transp,
            } => {
                let mut phant = Element::new("m:phant");
                if let Some(pr) = pr_with_flags(
                    "m:phantPr",
                    &[
                        ("show", *show),
                        ("zeroWid", *zero_wid),
                        ("zeroAsc", *zero_asc),
                        ("zeroDesc", *zero_desc),
                        ("transp", *transp),
                    ],
                ) {
                    phant.push_element(pr);
                }
                phant.push_element(wrap_slot("m:e", e));
                phant
            }
            Self::BorderBox {
                e,
                hide_top,
                hide_bot,
                hide_left,
                hide_right,
            } => {
                let mut box_el = Element::new("m:borderBox");
                if let Some(pr) = pr_with_flags(
                    "m:borderBoxPr",
                    &[
                        ("hideTop", *hide_top),
                        ("hideBot", *hide_bot),
                        ("hideLeft", *hide_left),
                        ("hideRight", *hide_right),
                    ],
                ) {
                    box_el.push_element(pr);
                }
                box_el.push_element(wrap_slot("m:e", e));
                box_el
            }
            Self::Other { .. } => math::new_math_run(""),
        }
    }
}

fn script(
    name: &str,
    e: &[MathNode],
    sub: Option<&[MathNode]>,
    sup: Option<&[MathNode]>,
) -> Element {
    let mut el = Element::new(name);
    el.push_element(wrap_slot("m:e", e));
    if let Some(s) = sub {
        el.push_element(wrap_slot("m:sub", s));
    }
    if let Some(s) = sup {
        el.push_element(wrap_slot("m:sup", s));
    }
    el
}

fn lim_el(name: &str, e: &[MathNode], lim: &[MathNode]) -> Element {
    let mut el = Element::new(name);
    el.push_element(wrap_slot("m:e", e));
    el.push_element(wrap_slot("m:lim", lim));
    el
}

fn wrap_slot(name: &str, nodes: &[MathNode]) -> Element {
    let mut slot = Element::new(name);
    for n in nodes {
        slot.push_element(n.to_element());
    }
    slot
}

fn skip_local(name: &str) -> bool {
    matches!(
        name,
        "fPr"
            | "radPr"
            | "naryPr"
            | "dPr"
            | "accPr"
            | "sSubPr"
            | "sSupPr"
            | "sSubSupPr"
            | "sPrePr"
            | "funcPr"
            | "mPr"
            | "mrPr"
            | "barPr"
            | "limLowPr"
            | "limUppPr"
            | "eqArrPr"
            | "boxPr"
            | "borderBoxPr"
            | "groupChrPr"
            | "phantPr"
            | "ctrlPr"
            | "rPr"
            | "argPr"
    )
}

fn read_seq(el: &Element) -> Vec<MathNode> {
    el.element_children()
        .filter(|c| !skip_local(c.local_name()))
        .map(read_node)
        .collect()
}

fn read_node(el: &Element) -> MathNode {
    match el.local_name() {
        "r" => MathNode::Text {
            text: math::math_run_text(el),
            brk: el
                .child_by_local("rPr")
                .and_then(|pr| pr.child_by_local("brk"))
                .is_some(),
        },
        "f" => MathNode::Frac {
            frac_type: read_frac_type(el),
            num: slot_nodes(el, "num"),
            den: slot_nodes(el, "den"),
        },
        "rad" => {
            let deg_hide = pr_flag(el, "radPr", "degHide");
            MathNode::Rad {
                deg_hide,
                deg: if deg_hide {
                    Vec::new()
                } else {
                    slot_nodes(el, "deg")
                },
                e: slot_nodes(el, "e"),
            }
        }
        "sSub" => MathNode::SSub {
            e: slot_nodes(el, "e"),
            sub: slot_nodes(el, "sub"),
        },
        "sSup" => MathNode::SSup {
            e: slot_nodes(el, "e"),
            sup: slot_nodes(el, "sup"),
        },
        "sSubSup" => MathNode::SSubSup {
            e: slot_nodes(el, "e"),
            sub: slot_nodes(el, "sub"),
            sup: slot_nodes(el, "sup"),
        },
        "sPre" => MathNode::SPre {
            e: slot_nodes(el, "e"),
            sub: slot_nodes(el, "sub"),
            sup: slot_nodes(el, "sup"),
        },
        "nary" => MathNode::Nary {
            chr: pr_chr(el, "naryPr"),
            sub: slot_nodes(el, "sub"),
            sup: slot_nodes(el, "sup"),
            e: slot_nodes(el, "e"),
        },
        "d" => MathNode::Delim {
            args: el
                .element_children()
                .filter(|c| c.local_name() == "e")
                .map(read_seq)
                .collect(),
        },
        "func" => MathNode::Func {
            f_name: slot_nodes(el, "fName"),
            e: slot_nodes(el, "e"),
        },
        "m" => MathNode::Mat {
            rows: el
                .element_children()
                .filter(|c| c.local_name() == "mr")
                .map(|mr| {
                    mr.element_children()
                        .filter(|c| c.local_name() == "e")
                        .map(read_seq)
                        .collect()
                })
                .collect(),
        },
        "acc" => MathNode::Acc {
            chr: pr_chr(el, "accPr"),
            e: slot_nodes(el, "e"),
        },
        "bar" => MathNode::Bar {
            e: slot_nodes(el, "e"),
        },
        "limLow" => MathNode::LimLow {
            e: slot_nodes(el, "e"),
            lim: slot_nodes(el, "lim"),
        },
        "limUpp" => MathNode::LimUpp {
            e: slot_nodes(el, "e"),
            lim: slot_nodes(el, "lim"),
        },
        "eqArr" => MathNode::EqArr {
            rows: el
                .element_children()
                .filter(|c| c.local_name() == "e")
                .map(read_seq)
                .collect(),
        },
        "phant" => MathNode::Phant {
            e: slot_nodes(el, "e"),
            show: pr_flag_opt(el, "phantPr", "show"),
            zero_wid: pr_flag_opt(el, "phantPr", "zeroWid"),
            zero_asc: pr_flag_opt(el, "phantPr", "zeroAsc"),
            zero_desc: pr_flag_opt(el, "phantPr", "zeroDesc"),
            transp: pr_flag_opt(el, "phantPr", "transp"),
        },
        "borderBox" => MathNode::BorderBox {
            e: slot_nodes(el, "e"),
            hide_top: pr_flag_opt(el, "borderBoxPr", "hideTop"),
            hide_bot: pr_flag_opt(el, "borderBoxPr", "hideBot"),
            hide_left: pr_flag_opt(el, "borderBoxPr", "hideLeft"),
            hide_right: pr_flag_opt(el, "borderBoxPr", "hideRight"),
        },
        name => MathNode::Other {
            name: name.to_string(),
            text: math::math_text(el),
        },
    }
}

fn slot_nodes(el: &Element, local: &str) -> Vec<MathNode> {
    el.child_by_local(local).map(read_seq).unwrap_or_default()
}

fn read_frac_type(el: &Element) -> FracType {
    el.child_by_local("fPr")
        .and_then(|pr| pr.child_by_local("type"))
        .and_then(|t| t.attr_local("val"))
        .and_then(FracType::parse)
        .unwrap_or(FracType::Bar)
}

fn pr_chr(el: &Element, pr: &str) -> Option<String> {
    el.child_by_local(pr)
        .and_then(|pr| pr.child_by_local("chr"))
        .and_then(|c| c.attr_local("val"))
        .map(|s| s.to_string())
}

fn pr_flag(el: &Element, pr: &str, flag: &str) -> bool {
    pr_flag_opt(el, pr, flag).unwrap_or(false)
}

fn pr_flag_opt(el: &Element, pr: &str, flag: &str) -> Option<bool> {
    let child = el.child_by_local(pr)?.child_by_local(flag)?;
    Some(match child.attr_local("val") {
        None => true,
        Some(v) => {
            let lower = v.to_ascii_lowercase();
            matches!(lower.as_str(), "1" | "true" | "on")
        }
    })
}

fn pr_with_flags(name: &str, flags: &[(&str, Option<bool>)]) -> Option<Element> {
    if flags.iter().all(|(_, v)| v.is_none()) {
        return None;
    }
    let mut pr = Element::new(name);
    for (local, val) in flags {
        let Some(b) = val else {
            continue;
        };
        let mut f = Element::new(format!("m:{local}"));
        f.set_attr("m:val", if *b { "1" } else { "0" });
        pr.push_element(f);
    }
    Some(pr)
}

fn node_to_json(n: &MathNode) -> Value {
    match n {
        MathNode::Text { text, brk } => {
            let mut o = Map::new();
            o.insert("type".into(), json!("text"));
            o.insert("text".into(), json!(text));
            if *brk {
                o.insert("brk".into(), json!(true));
            }
            Value::Object(o)
        }
        MathNode::Frac {
            frac_type,
            num,
            den,
        } => {
            let mut o = Map::new();
            o.insert("type".into(), json!("frac"));
            if let Some(k) = frac_type.as_json() {
                o.insert("fracType".into(), json!(k));
            }
            o.insert("num".into(), json!(nodes_to_json(num)));
            o.insert("den".into(), json!(nodes_to_json(den)));
            Value::Object(o)
        }
        MathNode::Rad { deg_hide, deg, e } => {
            let mut o = Map::new();
            o.insert("type".into(), json!("rad"));
            if *deg_hide {
                o.insert("degHide".into(), json!(true));
            } else if !deg.is_empty() {
                o.insert("deg".into(), json!(nodes_to_json(deg)));
            }
            o.insert("e".into(), json!(nodes_to_json(e)));
            Value::Object(o)
        }
        MathNode::SSub { e, sub } => json!({
            "type": "sSub",
            "e": nodes_to_json(e),
            "sub": nodes_to_json(sub),
        }),
        MathNode::SSup { e, sup } => json!({
            "type": "sSup",
            "e": nodes_to_json(e),
            "sup": nodes_to_json(sup),
        }),
        MathNode::SSubSup { e, sub, sup } => json!({
            "type": "sSubSup",
            "e": nodes_to_json(e),
            "sub": nodes_to_json(sub),
            "sup": nodes_to_json(sup),
        }),
        MathNode::SPre { e, sub, sup } => json!({
            "type": "sPre",
            "e": nodes_to_json(e),
            "sub": nodes_to_json(sub),
            "sup": nodes_to_json(sup),
        }),
        MathNode::Nary { chr, sub, sup, e } => {
            let mut o = Map::new();
            o.insert("type".into(), json!("nary"));
            if let Some(c) = chr {
                o.insert("chr".into(), json!(c));
            }
            if !sub.is_empty() {
                o.insert("sub".into(), json!(nodes_to_json(sub)));
            }
            if !sup.is_empty() {
                o.insert("sup".into(), json!(nodes_to_json(sup)));
            }
            o.insert("e".into(), json!(nodes_to_json(e)));
            Value::Object(o)
        }
        MathNode::Delim { args } => json!({
            "type": "delim",
            "args": args.iter().map(|a| nodes_to_json(a)).collect::<Vec<_>>(),
        }),
        MathNode::Func { f_name, e } => json!({
            "type": "func",
            "fName": nodes_to_json(f_name),
            "e": nodes_to_json(e),
        }),
        MathNode::Mat { rows } => {
            let rows_json: Vec<Value> = rows
                .iter()
                .map(|row| {
                    json!({
                        "cells": row.iter().map(|c| nodes_to_json(c)).collect::<Vec<_>>()
                    })
                })
                .collect();
            json!({ "type": "mat", "rows": rows_json })
        }
        MathNode::Acc { chr, e } => {
            let mut o = Map::new();
            o.insert("type".into(), json!("acc"));
            if let Some(c) = chr {
                o.insert("chr".into(), json!(c));
            }
            o.insert("e".into(), json!(nodes_to_json(e)));
            Value::Object(o)
        }
        MathNode::Bar { e } => json!({ "type": "bar", "e": nodes_to_json(e) }),
        MathNode::LimLow { e, lim } => json!({
            "type": "limLow",
            "e": nodes_to_json(e),
            "lim": nodes_to_json(lim),
        }),
        MathNode::LimUpp { e, lim } => json!({
            "type": "limUpp",
            "e": nodes_to_json(e),
            "lim": nodes_to_json(lim),
        }),
        MathNode::EqArr { rows } => json!({
            "type": "eqArr",
            "rows": rows.iter().map(|r| nodes_to_json(r)).collect::<Vec<_>>(),
        }),
        MathNode::Phant {
            e,
            show,
            zero_wid,
            zero_asc,
            zero_desc,
            transp,
        } => {
            let mut o = Map::new();
            o.insert("type".into(), json!("phant"));
            insert_opt_bool(&mut o, "show", *show);
            insert_opt_bool(&mut o, "zeroWid", *zero_wid);
            insert_opt_bool(&mut o, "zeroAsc", *zero_asc);
            insert_opt_bool(&mut o, "zeroDesc", *zero_desc);
            insert_opt_bool(&mut o, "transp", *transp);
            o.insert("e".into(), json!(nodes_to_json(e)));
            Value::Object(o)
        }
        MathNode::BorderBox {
            e,
            hide_top,
            hide_bot,
            hide_left,
            hide_right,
        } => {
            let mut o = Map::new();
            o.insert("type".into(), json!("borderBox"));
            insert_opt_bool(&mut o, "hideTop", *hide_top);
            insert_opt_bool(&mut o, "hideBot", *hide_bot);
            insert_opt_bool(&mut o, "hideLeft", *hide_left);
            insert_opt_bool(&mut o, "hideRight", *hide_right);
            o.insert("e".into(), json!(nodes_to_json(e)));
            Value::Object(o)
        }
        MathNode::Other { name, text } => json!({
            "type": "other",
            "name": name,
            "text": text,
        }),
    }
}

fn parse_nonempty_text(v: Option<&Value>, path: &str) -> Result<String, Error> {
    match v {
        Some(Value::String(s)) if !s.is_empty() => Ok(s.clone()),
        Some(Value::String(_)) => Err(Error::with_path(ErrorKind::EmptyMathText, path.to_string())),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "text".into(),
                expected: TypeName::String,
            },
            path.to_string(),
        )),
        None => Err(Error::with_path(ErrorKind::EmptyMathText, path.to_string())),
    }
}

fn parse_node_array(
    v: Option<&Value>,
    path: &str,
    require_nonempty: bool,
) -> Result<Vec<MathNode>, Error> {
    let arr = match v {
        Some(Value::Array(a)) => a,
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: path.rsplit('.').next().unwrap_or("expr").into(),
                    expected: TypeName::Array,
                },
                path.to_string(),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: path.rsplit('.').next().unwrap_or("expr").into(),
                    expected: TypeName::Array,
                },
                path.to_string(),
            ));
        }
    };
    if require_nonempty && arr.is_empty() {
        return Err(Error::with_path(ErrorKind::EmptyMathExpr, path.to_string()));
    }
    let mut out = Vec::new();
    for (i, item) in arr.iter().enumerate() {
        out.push(parse_node(item, &format!("{path}[{i}]"))?);
    }
    Ok(out)
}

fn parse_node(v: &Value, path: &str) -> Result<MathNode, Error> {
    let obj = v.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: path.to_string(),
                expected: TypeName::Object,
            },
            path.to_string(),
        )
    })?;
    let ty = match obj.get("type") {
        Some(Value::String(s)) => s.as_str(),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "type".into(),
                    expected: TypeName::String,
                },
                format!("{path}.type"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "type".into(),
                },
                format!("{path}.type"),
            ));
        }
    };
    match ty {
        "text" => {
            deny_node_keys(obj, path, &["type", "text", "brk"])?;
            let brk = parse_opt_node_bool(obj, path, "brk")?.unwrap_or(false);
            Ok(MathNode::Text {
                text: parse_nonempty_text(obj.get("text"), &format!("{path}.text"))?,
                brk,
            })
        }
        "frac" => {
            deny_node_keys(obj, path, &["type", "fracType", "num", "den"])?;
            let frac_type = match obj.get("fracType") {
                None => FracType::Bar,
                Some(Value::String(s)) => FracType::parse(s).ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::InvalidMathExprType { got: s.clone() },
                        format!("{path}.fracType"),
                    )
                })?,
                Some(_) => {
                    return Err(Error::with_path(
                        ErrorKind::WrongType {
                            field: "fracType".into(),
                            expected: TypeName::String,
                        },
                        format!("{path}.fracType"),
                    ));
                }
            };
            Ok(MathNode::Frac {
                frac_type,
                num: require_slot(obj, path, "num")?,
                den: require_slot(obj, path, "den")?,
            })
        }
        "rad" => {
            deny_node_keys(obj, path, &["type", "degHide", "deg", "e"])?;
            let deg_hide = match obj.get("degHide") {
                None => false,
                Some(Value::Bool(b)) => *b,
                Some(_) => {
                    return Err(Error::with_path(
                        ErrorKind::WrongType {
                            field: "degHide".into(),
                            expected: TypeName::Boolean,
                        },
                        format!("{path}.degHide"),
                    ));
                }
            };
            let deg = if obj.contains_key("deg") {
                parse_node_array(obj.get("deg"), &format!("{path}.deg"), false)?
            } else {
                Vec::new()
            };
            if deg_hide && !deg.is_empty() {
                return Err(Error::with_path(
                    ErrorKind::MutuallyExclusive {
                        a: "degHide".into(),
                        b: "deg".into(),
                    },
                    path.to_string(),
                ));
            }
            Ok(MathNode::Rad {
                deg_hide,
                deg,
                e: require_slot(obj, path, "e")?,
            })
        }
        "sSub" => {
            deny_node_keys(obj, path, &["type", "e", "sub"])?;
            Ok(MathNode::SSub {
                e: require_slot(obj, path, "e")?,
                sub: require_slot(obj, path, "sub")?,
            })
        }
        "sSup" => {
            deny_node_keys(obj, path, &["type", "e", "sup"])?;
            Ok(MathNode::SSup {
                e: require_slot(obj, path, "e")?,
                sup: require_slot(obj, path, "sup")?,
            })
        }
        "sSubSup" => {
            deny_node_keys(obj, path, &["type", "e", "sub", "sup"])?;
            Ok(MathNode::SSubSup {
                e: require_slot(obj, path, "e")?,
                sub: require_slot(obj, path, "sub")?,
                sup: require_slot(obj, path, "sup")?,
            })
        }
        "sPre" => {
            deny_node_keys(obj, path, &["type", "e", "sub", "sup"])?;
            Ok(MathNode::SPre {
                e: require_slot(obj, path, "e")?,
                sub: require_slot(obj, path, "sub")?,
                sup: require_slot(obj, path, "sup")?,
            })
        }
        "nary" => {
            deny_node_keys(obj, path, &["type", "chr", "sub", "sup", "e"])?;
            let chr = optional_str(obj.get("chr"), &format!("{path}.chr"))?;
            let sub = optional_slot(obj, path, "sub")?;
            let sup = optional_slot(obj, path, "sup")?;
            Ok(MathNode::Nary {
                chr,
                sub,
                sup,
                e: require_slot(obj, path, "e")?,
            })
        }
        "delim" => {
            deny_node_keys(obj, path, &["type", "args"])?;
            let args_v = obj.get("args").ok_or_else(|| {
                Error::with_path(
                    ErrorKind::MissingField {
                        field: "args".into(),
                    },
                    format!("{path}.args"),
                )
            })?;
            let arr = args_v.as_array().ok_or_else(|| {
                Error::with_path(
                    ErrorKind::WrongType {
                        field: "args".into(),
                        expected: TypeName::Array,
                    },
                    format!("{path}.args"),
                )
            })?;
            if arr.is_empty() {
                return Err(Error::with_path(
                    ErrorKind::EmptyMathSlot {
                        field: "args".into(),
                    },
                    format!("{path}.args"),
                ));
            }
            let mut args = Vec::new();
            for (i, item) in arr.iter().enumerate() {
                args.push(parse_node_array(
                    Some(item),
                    &format!("{path}.args[{i}]"),
                    true,
                )?);
            }
            Ok(MathNode::Delim { args })
        }
        "func" => {
            deny_node_keys(obj, path, &["type", "fName", "e"])?;
            Ok(MathNode::Func {
                f_name: require_slot(obj, path, "fName")?,
                e: require_slot(obj, path, "e")?,
            })
        }
        "mat" => {
            deny_node_keys(obj, path, &["type", "rows"])?;
            Ok(MathNode::Mat {
                rows: parse_mat_rows(obj.get("rows"), &format!("{path}.rows"))?,
            })
        }
        "acc" => {
            deny_node_keys(obj, path, &["type", "chr", "e"])?;
            Ok(MathNode::Acc {
                chr: optional_str(obj.get("chr"), &format!("{path}.chr"))?,
                e: require_slot(obj, path, "e")?,
            })
        }
        "bar" => {
            deny_node_keys(obj, path, &["type", "e"])?;
            Ok(MathNode::Bar {
                e: require_slot(obj, path, "e")?,
            })
        }
        "limLow" => {
            deny_node_keys(obj, path, &["type", "e", "lim"])?;
            Ok(MathNode::LimLow {
                e: require_slot(obj, path, "e")?,
                lim: require_slot(obj, path, "lim")?,
            })
        }
        "limUpp" => {
            deny_node_keys(obj, path, &["type", "e", "lim"])?;
            Ok(MathNode::LimUpp {
                e: require_slot(obj, path, "e")?,
                lim: require_slot(obj, path, "lim")?,
            })
        }
        "eqArr" => {
            deny_node_keys(obj, path, &["type", "rows"])?;
            Ok(MathNode::EqArr {
                rows: parse_eq_rows(obj.get("rows"), &format!("{path}.rows"))?,
            })
        }
        "phant" => {
            deny_node_keys(
                obj,
                path,
                &[
                    "type", "e", "show", "zeroWid", "zeroAsc", "zeroDesc", "transp",
                ],
            )?;
            Ok(MathNode::Phant {
                e: require_slot(obj, path, "e")?,
                show: parse_opt_node_bool(obj, path, "show")?,
                zero_wid: parse_opt_node_bool(obj, path, "zeroWid")?,
                zero_asc: parse_opt_node_bool(obj, path, "zeroAsc")?,
                zero_desc: parse_opt_node_bool(obj, path, "zeroDesc")?,
                transp: parse_opt_node_bool(obj, path, "transp")?,
            })
        }
        "borderBox" => {
            deny_node_keys(
                obj,
                path,
                &["type", "e", "hideTop", "hideBot", "hideLeft", "hideRight"],
            )?;
            Ok(MathNode::BorderBox {
                e: require_slot(obj, path, "e")?,
                hide_top: parse_opt_node_bool(obj, path, "hideTop")?,
                hide_bot: parse_opt_node_bool(obj, path, "hideBot")?,
                hide_left: parse_opt_node_bool(obj, path, "hideLeft")?,
                hide_right: parse_opt_node_bool(obj, path, "hideRight")?,
            })
        }
        other => Err(Error::with_path(
            ErrorKind::InvalidMathExprType {
                got: other.to_string(),
            },
            format!("{path}.type"),
        )),
    }
}

fn insert_opt_bool(o: &mut Map<String, Value>, key: &str, val: Option<bool>) {
    if let Some(b) = val {
        o.insert(key.into(), json!(b));
    }
}

fn parse_opt_node_bool(
    obj: &Map<String, Value>,
    path: &str,
    field: &str,
) -> Result<Option<bool>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::Bool(b)) => Ok(Some(*b)),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Boolean,
            },
            format!("{path}.{field}"),
        )),
    }
}

fn deny_node_keys(obj: &Map<String, Value>, path: &str, allowed: &[&str]) -> Result<(), Error> {
    for k in obj.keys() {
        if !allowed.contains(&k.as_str()) {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("{path}.{k}"),
            ));
        }
    }
    Ok(())
}

fn require_slot(obj: &Map<String, Value>, path: &str, field: &str) -> Result<Vec<MathNode>, Error> {
    let nodes = parse_node_array(obj.get(field), &format!("{path}.{field}"), true)?;
    if nodes.is_empty() {
        return Err(Error::with_path(
            ErrorKind::EmptyMathSlot {
                field: field.into(),
            },
            format!("{path}.{field}"),
        ));
    }
    Ok(nodes)
}

fn optional_slot(
    obj: &Map<String, Value>,
    path: &str,
    field: &str,
) -> Result<Vec<MathNode>, Error> {
    if !obj.contains_key(field) {
        return Ok(Vec::new());
    }
    parse_node_array(obj.get(field), &format!("{path}.{field}"), false)
}

fn optional_str(v: Option<&Value>, path: &str) -> Result<Option<String>, Error> {
    match v {
        None => Ok(None),
        Some(Value::String(s)) if !s.is_empty() => Ok(Some(s.clone())),
        Some(Value::String(_)) => Err(Error::with_path(ErrorKind::EmptyMathText, path.to_string())),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "chr".into(),
                expected: TypeName::String,
            },
            path.to_string(),
        )),
    }
}

fn parse_mat_rows(v: Option<&Value>, path: &str) -> Result<Vec<Vec<Vec<MathNode>>>, Error> {
    let arr = v.and_then(Value::as_array).ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "rows".into(),
                expected: TypeName::Array,
            },
            path.to_string(),
        )
    })?;
    if arr.is_empty() {
        return Err(Error::with_path(
            ErrorKind::EmptyMathSlot {
                field: "rows".into(),
            },
            path.to_string(),
        ));
    }
    let mut rows = Vec::new();
    let mut width: Option<usize> = None;
    for (i, item) in arr.iter().enumerate() {
        let item_path = format!("{path}[{i}]");
        let obj = item.as_object().ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: item_path.clone(),
                    expected: TypeName::Object,
                },
                item_path.clone(),
            )
        })?;
        deny_node_keys(obj, &item_path, &["cells"])?;
        let cells_v = obj.get("cells").ok_or_else(|| {
            Error::with_path(
                ErrorKind::MissingField {
                    field: "cells".into(),
                },
                format!("{item_path}.cells"),
            )
        })?;
        let cells_arr = cells_v.as_array().ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: "cells".into(),
                    expected: TypeName::Array,
                },
                format!("{item_path}.cells"),
            )
        })?;
        if cells_arr.is_empty() {
            return Err(Error::with_path(
                ErrorKind::EmptyMathSlot {
                    field: "cells".into(),
                },
                format!("{item_path}.cells"),
            ));
        }
        if let Some(w) = width {
            if cells_arr.len() != w {
                return Err(Error::with_path(
                    ErrorKind::InvalidMathExprType {
                        got: "mat-ragged".into(),
                    },
                    format!("{item_path}.cells"),
                ));
            }
        } else {
            width = Some(cells_arr.len());
        }
        let mut cells = Vec::new();
        for (j, cell) in cells_arr.iter().enumerate() {
            cells.push(parse_node_array(
                Some(cell),
                &format!("{item_path}.cells[{j}]"),
                true,
            )?);
        }
        rows.push(cells);
    }
    Ok(rows)
}

fn parse_eq_rows(v: Option<&Value>, path: &str) -> Result<Vec<Vec<MathNode>>, Error> {
    let arr = v.and_then(Value::as_array).ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "rows".into(),
                expected: TypeName::Array,
            },
            path.to_string(),
        )
    })?;
    if arr.is_empty() {
        return Err(Error::with_path(
            ErrorKind::EmptyMathSlot {
                field: "rows".into(),
            },
            path.to_string(),
        ));
    }
    let mut rows = Vec::new();
    for (i, item) in arr.iter().enumerate() {
        rows.push(parse_node_array(Some(item), &format!("{path}[{i}]"), true)?);
    }
    Ok(rows)
}
