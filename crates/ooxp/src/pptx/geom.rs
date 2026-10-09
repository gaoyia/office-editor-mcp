//! 自定义几何 `a:custGeom`：inspect 路径要点，apply 整份替换。

use serde_json::{json, Map, Value};

use crate::xml::{Element, Node};

use super::picture;

#[derive(Debug, Clone)]
pub struct Path {
    pub w: i64,
    pub h: i64,
    pub ops: Vec<Op>,
}

#[derive(Debug, Clone)]
pub enum Op {
    MoveTo { x: i64, y: i64 },
    LnTo { x: i64, y: i64 },
    Close,
}

pub fn inspect_json(sp: &Element) -> Option<Value> {
    let cust = sp
        .child_by_local("spPr")
        .and_then(|p| p.child_by_local("custGeom"))?;
    let path_lst = cust.child_by_local("pathLst")?;
    let mut paths = Vec::new();
    for path in path_lst.children_by_local("path") {
        paths.push(inspect_path(path));
    }
    let mut obj = Map::new();
    obj.insert("paths".into(), Value::Array(paths));
    Some(Value::Object(obj))
}

fn inspect_path(path: &Element) -> Value {
    let mut obj = Map::new();
    obj.insert("w".into(), json_i64_attr(Some(path), "w"));
    obj.insert("h".into(), json_i64_attr(Some(path), "h"));
    let mut ops = Vec::new();
    for n in &path.children {
        let Node::Element(el) = n else {
            continue;
        };
        if let Some(op) = inspect_op(el) {
            ops.push(op);
        }
    }
    obj.insert("ops".into(), Value::Array(ops));
    Value::Object(obj)
}

fn inspect_op(el: &Element) -> Option<Value> {
    let mut obj = Map::new();
    let ty = el.local_name();
    obj.insert("type".into(), json!(ty));
    match ty {
        "moveTo" | "lnTo" => {
            let pt = el.child_by_local("pt");
            obj.insert("x".into(), json_i64_attr(pt, "x"));
            obj.insert("y".into(), json_i64_attr(pt, "y"));
        }
        "close" => {}
        "arcTo" => {
            obj.insert("wR".into(), json_num_or_str_attr(el, "wR"));
            obj.insert("hR".into(), json_num_or_str_attr(el, "hR"));
            obj.insert("stAng".into(), json_num_or_str_attr(el, "stAng"));
            obj.insert("swAng".into(), json_num_or_str_attr(el, "swAng"));
        }
        _ => {
            let pts: Vec<Value> = el
                .children_by_local("pt")
                .into_iter()
                .map(|pt| {
                    json!({
                        "x": json_i64_attr(Some(pt), "x"),
                        "y": json_i64_attr(Some(pt), "y"),
                    })
                })
                .collect();
            if !pts.is_empty() {
                obj.insert("pts".into(), Value::Array(pts));
            }
        }
    }
    Some(Value::Object(obj))
}

fn json_i64_attr(el: Option<&Element>, name: &str) -> Value {
    match el
        .and_then(|e| e.attr(name))
        .and_then(|s| s.parse::<i64>().ok())
    {
        Some(n) => json!(n),
        None => Value::Null,
    }
}

fn json_num_or_str_attr(el: &Element, name: &str) -> Value {
    match el.attr(name) {
        Some(s) => match s.parse::<i64>() {
            Ok(n) => json!(n),
            Err(_) => json!(s),
        },
        None => Value::Null,
    }
}

pub fn apply_to_sp(sp: &mut Element, paths: &[Path]) {
    let sp_pr = picture::ensure_child(sp, "spPr");
    let geom = build_cust_geom(paths);
    let idx = sp_pr.children.iter().position(|n| {
        matches!(
            n,
            Node::Element(e) if e.local_name() == "prstGeom" || e.local_name() == "custGeom"
        )
    });
    if let Some(i) = idx {
        sp_pr.children[i] = Node::Element(geom);
        return;
    }
    let after = sp_pr
        .children
        .iter()
        .position(|n| matches!(n, Node::Element(e) if e.local_name() == "xfrm"));
    if let Some(i) = after {
        sp_pr.children.insert(i + 1, Node::Element(geom));
    } else {
        sp_pr.children.insert(0, Node::Element(geom));
    }
}

pub fn build_cust_geom(paths: &[Path]) -> Element {
    let mut cust = Element::new("a:custGeom");
    cust.push_element(Element::new("a:avLst"));
    cust.push_element(Element::new("a:gdLst"));
    cust.push_element(Element::new("a:ahLst"));
    cust.push_element(Element::new("a:cxnLst"));
    cust.push_element(
        Element::new("a:rect")
            .with_attr("l", "0")
            .with_attr("t", "0")
            .with_attr("r", "0")
            .with_attr("b", "0"),
    );
    let mut lst = Element::new("a:pathLst");
    for path in paths {
        lst.push_element(build_path(path));
    }
    cust.push_element(lst);
    cust
}

fn build_path(path: &Path) -> Element {
    let mut el = Element::new("a:path")
        .with_attr("w", path.w.to_string())
        .with_attr("h", path.h.to_string());
    for op in &path.ops {
        el.push_element(build_op(op));
    }
    el
}

fn build_op(op: &Op) -> Element {
    match op {
        Op::MoveTo { x, y } => {
            let mut e = Element::new("a:moveTo");
            e.push_element(pt(*x, *y));
            e
        }
        Op::LnTo { x, y } => {
            let mut e = Element::new("a:lnTo");
            e.push_element(pt(*x, *y));
            e
        }
        Op::Close => Element::new("a:close"),
    }
}

fn pt(x: i64, y: i64) -> Element {
    Element::new("a:pt")
        .with_attr("x", x.to_string())
        .with_attr("y", y.to_string())
}
