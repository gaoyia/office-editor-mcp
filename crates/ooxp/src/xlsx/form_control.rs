//! 工作表窗体：按钮、复选框、下拉。
//!
//! 一个控件要四处对上才进树：工作表 `control/@shapeId`、Choice 里 `cNvPr/@id`
//! 与 `compatExt/@spid`、VML `v:shape/@id`。标题在 Choice 的 `a:t` 和 VML 文本框。
//! 链接、列表、勾选、选中项在 `formControlPr` 和 VML 里各写一处。

use std::collections::{HashMap, HashSet};

use serde_json::{json, Map, Value};

use crate::error::{Error, ErrorCode, ErrorKind, IdReason, TypeName};
use crate::op_fields::{deny_keys, parse_opt_bool};
use crate::opc::{self, Package, Relationships};
use crate::xml::{self, Document, Element, Node};

use super::list_pin;

/// 省略、清空，或写成非空字符串。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TextPatch {
    Absent,
    Clear,
    Set(String),
}

impl TextPatch {
    fn is_absent(&self) -> bool {
        matches!(self, Self::Absent)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct SetFormControlOp {
    pub sheet: String,
    pub shape_id: String,
    pub caption: Option<String>,
    pub fmla_link: TextPatch,
    pub fmla_range: TextPatch,
    pub checked: Option<bool>,
    pub sel: Option<u64>,
}

struct Item {
    shape_id: String,
    name: String,
    object_type: String,
    drawing_idx: usize,
    sp_path: Vec<usize>,
    vml_idx: usize,
    shape_path: Vec<usize>,
    ctrl_part: String,
}

struct Loaded {
    drawings: Vec<(String, Document)>,
    vmls: Vec<(String, Document)>,
    ctrls: HashMap<String, Document>,
    items: Vec<Item>,
}

pub(crate) fn parse_set(obj: &Map<String, Value>, i: usize) -> Result<SetFormControlOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op",
            "id",
            "caption",
            "fmlaLink",
            "fmlaRange",
            "checked",
            "sel",
        ],
    )?;
    let id = obj.get("id").and_then(Value::as_str).ok_or_else(|| {
        Error::with_path(
            ErrorKind::MissingField { field: "id".into() },
            format!("ops[{i}].id"),
        )
    })?;
    let (sheet, shape_id) = parse_id(id, &format!("ops[{i}].id"))?;
    let caption = parse_caption(obj, i)?;
    let fmla_link = parse_formula(obj, i, "fmlaLink")?;
    let fmla_range = parse_formula(obj, i, "fmlaRange")?;
    let checked = parse_opt_bool(obj, i, "checked")?;
    let sel = parse_sel(obj, i)?;
    if caption.is_none()
        && fmla_link.is_absent()
        && fmla_range.is_absent()
        && checked.is_none()
        && sel.is_none()
    {
        return Err(Error::with_path(
            ErrorKind::NeedSetFormControlField,
            format!("ops[{i}]"),
        ));
    }
    Ok(SetFormControlOp {
        sheet,
        shape_id,
        caption,
        fmla_link,
        fmla_range,
        checked,
        sel,
    })
}

pub(crate) fn parse_id(id: &str, json_path: &str) -> Result<(String, String), Error> {
    let rest = id.strip_prefix("sheet:").ok_or_else(|| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SheetControlPrefix,
            },
            json_path.to_string(),
        )
    })?;
    let Some((name, shape_id)) = rest.split_once("/control:") else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SheetControlContainer,
            },
            json_path.to_string(),
        ));
    };
    if name.is_empty() || shape_id.is_empty() {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SheetControlPrefix,
            },
            json_path.to_string(),
        ));
    }
    Ok((name.to_string(), shape_id.to_string()))
}

pub(crate) fn collect(
    pkg: &Package,
    sheet_part: &str,
    sheet: &Element,
    sheet_name: &str,
) -> Result<Vec<Value>, Error> {
    let loaded = load(pkg, sheet_part, sheet)?;
    Ok(loaded
        .items
        .iter()
        .map(|item| loaded.to_json(item, sheet_name))
        .collect())
}

pub(crate) fn set(
    pkg: &mut Package,
    sheet: &mut Element,
    sheet_part: &str,
    op: &SetFormControlOp,
    op_index: usize,
) -> Result<(), Error> {
    let mut loaded = load(pkg, sheet_part, sheet)?;
    let Some(pos) = loaded
        .items
        .iter()
        .position(|item| item.shape_id == op.shape_id)
    else {
        return Err(Error::with_path(
            ErrorKind::FormControlNotFound {
                id: format!("sheet:{}/control:{}", op.sheet, op.shape_id),
            },
            format!("ops[{op_index}].id"),
        ));
    };
    let object_type = loaded.items[pos].object_type.clone();
    reject_for_type(&object_type, op, op_index)?;
    if op.caption.is_some() {
        require_caption_nodes(&loaded, &loaded.items[pos], op_index)?;
    }

    let item = &loaded.items[pos];
    let drawing_idx = item.drawing_idx;
    let sp_path = item.sp_path.clone();
    let vml_idx = item.vml_idx;
    let shape_path = item.shape_path.clone();
    let ctrl_part = item.ctrl_part.clone();
    let existing_link = loaded
        .ctrls
        .get(&ctrl_part)
        .and_then(|doc| nonempty(&doc.root, "fmlaLink"));
    let drawing_part = loaded.drawings[drawing_idx].0.clone();
    let vml_part = loaded.vmls[vml_idx].0.clone();

    let mut drawing_dirty = false;
    let mut vml_dirty = false;
    let mut ctrl_dirty = false;

    if let Some(caption) = &op.caption {
        let drawing = &mut loaded.drawings[drawing_idx].1;
        let sp = try_follow_mut(&mut drawing.root, &sp_path)
            .ok_or_else(|| field_err("caption", op_index))?;
        let text_paths = paths_under(sp, "txBody", "t");
        let vml = &mut loaded.vmls[vml_idx].1;
        let shape = try_follow_mut(&mut vml.root, &shape_path)
            .ok_or_else(|| field_err("caption", op_index))?;
        let font_paths = paths_under(shape, "textbox", "font");
        write_joined(drawing, &sp_path, &text_paths, caption);
        write_joined(vml, &shape_path, &font_paths, caption);
        drawing_dirty = true;
        vml_dirty = true;
    }

    let vml = &mut loaded.vmls[vml_idx].1;
    let shape = try_follow_mut(&mut vml.root, &shape_path).ok_or_else(|| {
        Error::with_path(
            ErrorKind::FormControlNotFound {
                id: format!("sheet:{}/control:{}", op.sheet, op.shape_id),
            },
            format!("ops[{op_index}].id"),
        )
    })?;
    let client_path = client_data_path(shape).ok_or_else(|| {
        Error::with_path(
            ErrorKind::FormControlNotFound {
                id: format!("sheet:{}/control:{}", op.sheet, op.shape_id),
            },
            format!("ops[{op_index}].id"),
        )
    })?;
    let client = try_follow_mut(shape, &client_path).expect("刚找到的 ClientData");
    if apply_text_pair(client, "FmlaLink", &op.fmla_link) {
        vml_dirty = true;
        ctrl_dirty = true;
    }
    if apply_text_pair(client, "FmlaRange", &op.fmla_range) {
        vml_dirty = true;
        ctrl_dirty = true;
    }
    if let Some(checked) = op.checked {
        if checked {
            set_child_text(client, "Checked", "1");
        } else {
            remove_local(client, "Checked");
        }
        vml_dirty = true;
        ctrl_dirty = true;
    }
    if let Some(sel) = op.sel {
        set_child_text(client, "Sel", &sel.to_string());
        vml_dirty = true;
        ctrl_dirty = true;
    }

    if ctrl_dirty {
        let pr = loaded.ctrls.get_mut(&ctrl_part).expect("控件属性已装入");
        match &op.fmla_link {
            TextPatch::Absent => {}
            TextPatch::Clear => remove_attr_local(&mut pr.root, "fmlaLink"),
            TextPatch::Set(value) => set_attr_local(&mut pr.root, "fmlaLink", value),
        }
        match &op.fmla_range {
            TextPatch::Absent => {}
            TextPatch::Clear => remove_attr_local(&mut pr.root, "fmlaRange"),
            TextPatch::Set(value) => set_attr_local(&mut pr.root, "fmlaRange", value),
        }
        if let Some(checked) = op.checked {
            if checked {
                set_attr_local(&mut pr.root, "checked", "Checked");
            } else {
                remove_attr_local(&mut pr.root, "checked");
            }
        }
        if let Some(sel) = op.sel {
            set_attr_local(&mut pr.root, "sel", &sel.to_string());
        }
    }

    if drawing_dirty {
        list_pin::write_part(pkg, &drawing_part, &loaded.drawings[drawing_idx].1)?;
    }
    if vml_dirty {
        list_pin::write_part(pkg, &vml_part, &loaded.vmls[vml_idx].1)?;
    }
    if ctrl_dirty {
        let doc = loaded.ctrls.get(&ctrl_part).expect("控件属性已装入");
        list_pin::write_part(pkg, &ctrl_part, doc)?;
    }
    // Excel 打开时以链接格为准。格子是空的，勾选标记会被清掉。
    if let Some(on) = op.checked {
        let link = match &op.fmla_link {
            TextPatch::Set(value) => Some(value.clone()),
            TextPatch::Clear | TextPatch::Absent => existing_link,
        };
        if let Some(cell) = link
            .as_deref()
            .and_then(|text| linked_cell(text, &op.sheet))
        {
            write_check_cell(sheet, cell, on);
        }
    }
    Ok(())
}

fn linked_cell(link: &str, sheet_name: &str) -> Option<super::a1::CellRef> {
    let body = if let Some((name, rest)) = link.split_once('!') {
        let name = name.trim().trim_matches('\'');
        if !name.eq_ignore_ascii_case(sheet_name) {
            return None;
        }
        rest.trim()
    } else {
        link.trim()
    };
    super::a1::parse_cell(body).ok()
}

fn write_check_cell(sheet: &mut Element, cell: super::a1::CellRef, on: bool) {
    {
        let data = super::cell::ensure_sheet_data(sheet);
        let node = super::cell::ensure_cell(data, cell);
        node.set_attr("t", "b");
        node.children.retain(|child| match child {
            Node::Element(el) => !matches!(el.local_name(), "f" | "v" | "is"),
            Node::Text(_) | Node::Raw(_) => true,
        });
        node.push_element(Element::new("v").with_text(if on { "1" } else { "0" }));
    }
    super::cell::expand_dimension(sheet, cell);
}

fn reject_for_type(object_type: &str, op: &SetFormControlOp, op_index: usize) -> Result<(), Error> {
    let bad = |field: &str| field_err(field, op_index);
    match object_type {
        "Button" => {
            if !op.fmla_link.is_absent() {
                return Err(bad("fmlaLink"));
            }
            if !op.fmla_range.is_absent() {
                return Err(bad("fmlaRange"));
            }
            if op.checked.is_some() {
                return Err(bad("checked"));
            }
            if op.sel.is_some() {
                return Err(bad("sel"));
            }
        }
        "CheckBox" => {
            if !op.fmla_range.is_absent() {
                return Err(bad("fmlaRange"));
            }
            if op.sel.is_some() {
                return Err(bad("sel"));
            }
        }
        "Drop" => {
            if !op.fmla_link.is_absent() {
                return Err(bad("fmlaLink"));
            }
            if op.checked.is_some() {
                return Err(bad("checked"));
            }
        }
        _ => {
            return Err(bad("objectType"));
        }
    }
    Ok(())
}

fn require_caption_nodes(loaded: &Loaded, item: &Item, op_index: usize) -> Result<(), Error> {
    let drawing = &loaded.drawings[item.drawing_idx].1;
    let sp =
        try_follow(&drawing.root, &item.sp_path).ok_or_else(|| field_err("caption", op_index))?;
    let vml = &loaded.vmls[item.vml_idx].1;
    let shape =
        try_follow(&vml.root, &item.shape_path).ok_or_else(|| field_err("caption", op_index))?;
    if paths_under(sp, "txBody", "t").is_empty() || paths_under(shape, "textbox", "font").is_empty()
    {
        return Err(field_err("caption", op_index));
    }
    Ok(())
}

fn field_err(field: &str, op_index: usize) -> Error {
    Error::with_path(
        ErrorKind::InvalidFormControl {
            field: field.to_string(),
        },
        format!("ops[{op_index}].{field}"),
    )
}

fn load(pkg: &Package, sheet_part: &str, sheet: &Element) -> Result<Loaded, Error> {
    let controls = control_elements(sheet);
    if controls.is_empty() {
        return Ok(Loaded {
            drawings: Vec::new(),
            vmls: Vec::new(),
            ctrls: HashMap::new(),
            items: Vec::new(),
        });
    }
    let rels = Relationships::from_package(pkg, sheet_part)?;
    let drawing_parts = parts_by_suffix(sheet_part, &rels, "/drawing")?;
    let vml_parts = parts_by_suffix(sheet_part, &rels, "/vmlDrawing")?;
    let mut drawings = Vec::new();
    for part in &drawing_parts {
        drawings.push((part.clone(), list_pin::parse_part(pkg, part)?));
    }
    let mut vmls = Vec::new();
    for part in &vml_parts {
        vmls.push((part.clone(), list_pin::parse_part(pkg, part)?));
    }
    let mut ctrls = HashMap::new();
    let mut items = Vec::new();
    let mut seen = HashSet::new();
    for control in controls {
        if attr_local(control, "link").is_some() {
            continue;
        }
        let Some(shape_id) = control.attr("shapeId").filter(|s| !s.is_empty()) else {
            continue;
        };
        if seen.contains(shape_id) {
            continue;
        }
        let Some(rid) = rel_id(control) else {
            continue;
        };
        let Some(rel) = rels.find_by_id(rid) else {
            continue;
        };
        if rel.is_external() || !rel.rel_type.ends_with("/ctrlProp") {
            continue;
        }
        let Ok(ctrl_part) = opc::resolve_rel_target(sheet_part, &rel.target) else {
            continue;
        };
        if !ctrls.contains_key(&ctrl_part) {
            let bytes = match pkg.get(&ctrl_part) {
                Ok(bytes) => bytes,
                Err(err) if err.code() == ErrorCode::OpcPartMissing => continue,
                Err(err) => return Err(err),
            };
            ctrls.insert(ctrl_part.clone(), xml::parse(bytes)?);
        }
        let Some(pr) = ctrls.get(&ctrl_part) else {
            continue;
        };
        if pr.root.local_name() != "formControlPr" {
            continue;
        }
        let Some(object_type) = pr.root.attr("objectType") else {
            continue;
        };
        if !matches!(object_type, "Button" | "CheckBox" | "Drop") {
            continue;
        }
        let Some((drawing_idx, sp_path)) = find_choice_sp(&drawings, shape_id) else {
            continue;
        };
        let Some((vml_idx, shape_path)) = find_vml_shape(&vmls, shape_id, object_type) else {
            continue;
        };
        items.push(Item {
            shape_id: shape_id.to_string(),
            name: control.attr("name").unwrap_or("").to_string(),
            object_type: object_type.to_string(),
            drawing_idx,
            sp_path,
            vml_idx,
            shape_path,
            ctrl_part,
        });
        seen.insert(shape_id.to_string());
    }
    Ok(Loaded {
        drawings,
        vmls,
        ctrls,
        items,
    })
}

impl Loaded {
    fn to_json(&self, item: &Item, sheet_name: &str) -> Value {
        let sp = try_follow(&self.drawings[item.drawing_idx].1.root, &item.sp_path);
        let shape = try_follow(&self.vmls[item.vml_idx].1.root, &item.shape_path);
        let caption = sp.and_then(|el| joined_text(el, "txBody", "t"));
        let pr = self.ctrls.get(&item.ctrl_part).map(|doc| &doc.root);
        let fmla_link = pr.and_then(|el| nonempty(el, "fmlaLink"));
        let fmla_range = pr.and_then(|el| nonempty(el, "fmlaRange"));
        let checked = if item.object_type == "CheckBox" {
            let attr_on = pr.is_some_and(|el| attr_local(el, "checked") == Some("Checked"));
            let vml_on = shape.is_some_and(vml_checked);
            json!(attr_on && vml_on)
        } else {
            Value::Null
        };
        let sel = if item.object_type == "Drop" {
            match pr
                .and_then(|el| attr_local(el, "sel"))
                .and_then(|s| s.parse::<u64>().ok())
            {
                Some(n) => json!(n),
                None => Value::Null,
            }
        } else {
            Value::Null
        };
        json!({
            "id": format!("sheet:{sheet_name}/control:{}", item.shape_id),
            "shapeId": item.shape_id,
            "name": item.name,
            "objectType": item.object_type,
            "caption": caption,
            "fmlaLink": fmla_link,
            "fmlaRange": fmla_range,
            "checked": checked,
            "sel": sel,
        })
    }
}

fn control_elements(sheet: &Element) -> Vec<&Element> {
    let mut out = Vec::new();
    walk_controls(sheet, false, &mut out);
    out
}

fn walk_controls<'a>(el: &'a Element, in_controls: bool, out: &mut Vec<&'a Element>) {
    let here = in_controls || el.local_name() == "controls";
    if here && el.local_name() == "control" {
        out.push(el);
    }
    for child in el.element_children() {
        walk_controls(child, here, out);
    }
}

fn parts_by_suffix(
    sheet_part: &str,
    rels: &Relationships,
    suffix: &str,
) -> Result<Vec<String>, Error> {
    let mut out = Vec::new();
    for rel in &rels.rels {
        if rel.is_external() || !rel.rel_type.ends_with(suffix) {
            continue;
        }
        out.push(opc::resolve_rel_target(sheet_part, &rel.target)?);
    }
    Ok(out)
}

fn find_choice_sp(drawings: &[(String, Document)], shape_id: &str) -> Option<(usize, Vec<usize>)> {
    for (idx, (_, doc)) in drawings.iter().enumerate() {
        let mut paths = Vec::new();
        collect_paths(
            &doc.root,
            &|el| sp_matches(el, shape_id),
            &mut Vec::new(),
            &mut paths,
        );
        if let Some(path) = paths
            .into_iter()
            .find(|path| has_ancestor(&doc.root, path, "Choice"))
        {
            return Some((idx, path));
        }
    }
    None
}

fn find_vml_shape(
    vmls: &[(String, Document)],
    shape_id: &str,
    object_type: &str,
) -> Option<(usize, Vec<usize>)> {
    for (idx, (_, doc)) in vmls.iter().enumerate() {
        let mut paths = Vec::new();
        collect_paths(
            &doc.root,
            &|el| shape_matches(el, shape_id, object_type),
            &mut Vec::new(),
            &mut paths,
        );
        if let Some(path) = paths.into_iter().next() {
            return Some((idx, path));
        }
    }
    None
}

fn sp_matches(el: &Element, shape_id: &str) -> bool {
    if el.local_name() != "sp" {
        return false;
    }
    let want = format!("_x0000_s{shape_id}");
    let mut id_ok = false;
    let mut spid_ok = false;
    walk_all(el, &mut |node| {
        if node.local_name() == "cNvPr" && node.attr("id") == Some(shape_id) {
            id_ok = true;
        }
        if node.local_name() == "compatExt" && node.attr("spid") == Some(want.as_str()) {
            spid_ok = true;
        }
    });
    id_ok && spid_ok
}

fn shape_matches(el: &Element, shape_id: &str, object_type: &str) -> bool {
    if el.local_name() != "shape" {
        return false;
    }
    let want = format!("_x0000_s{shape_id}");
    if el.attr("id") != Some(want.as_str()) {
        return false;
    }
    let mut ok = false;
    walk_all(el, &mut |node| {
        if node.local_name() == "ClientData" {
            if let Some(kind) = node.attr("ObjectType") {
                if vml_type_matches(kind, object_type) {
                    ok = true;
                }
            }
        }
    });
    ok
}

fn vml_type_matches(vml_type: &str, object_type: &str) -> bool {
    matches!(
        (vml_type, object_type),
        ("Button", "Button") | ("Checkbox", "CheckBox") | ("Drop", "Drop")
    )
}

fn vml_checked(shape: &Element) -> bool {
    let mut on = false;
    walk_all(shape, &mut |node| {
        if node.local_name() == "Checked" && node.text_content().trim() == "1" {
            on = true;
        }
    });
    on
}

fn joined_text(el: &Element, ancestor: &str, local: &str) -> Option<String> {
    let mut parts = Vec::new();
    collect_text_under(el, ancestor, local, false, &mut parts);
    if parts.is_empty() {
        None
    } else {
        Some(parts.concat())
    }
}

fn collect_text_under(
    el: &Element,
    ancestor: &str,
    local: &str,
    inside: bool,
    out: &mut Vec<String>,
) {
    let here = inside || el.local_name() == ancestor;
    if here && el.local_name() == local && el.local_name() != ancestor {
        out.push(el.text_content());
    }
    for child in el.element_children() {
        collect_text_under(child, ancestor, local, here, out);
    }
}

fn nonempty(el: &Element, local: &str) -> Option<String> {
    attr_local(el, local)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn rel_id(el: &Element) -> Option<&str> {
    el.attrs.iter().find_map(|(key, value)| {
        let (prefix, local) = key.split_once(':')?;
        (!prefix.is_empty() && local == "id").then_some(value.as_str())
    })
}

fn attr_local<'a>(el: &'a Element, local: &str) -> Option<&'a str> {
    el.attr_local(local)
}

fn collect_paths(
    el: &Element,
    pred: &impl Fn(&Element) -> bool,
    prefix: &mut Vec<usize>,
    out: &mut Vec<Vec<usize>>,
) {
    if pred(el) {
        out.push(prefix.clone());
    }
    for (i, node) in el.children.iter().enumerate() {
        if let Node::Element(child) = node {
            prefix.push(i);
            collect_paths(child, pred, prefix, out);
            prefix.pop();
        }
    }
}

fn paths_under(el: &Element, ancestor: &str, local: &str) -> Vec<Vec<usize>> {
    let mut out = Vec::new();
    collect_under(el, ancestor, local, false, &mut Vec::new(), &mut out);
    out
}

fn collect_under(
    el: &Element,
    ancestor: &str,
    local: &str,
    inside: bool,
    prefix: &mut Vec<usize>,
    out: &mut Vec<Vec<usize>>,
) {
    let here = inside || el.local_name() == ancestor;
    if here && el.local_name() == local && el.local_name() != ancestor {
        out.push(prefix.clone());
    }
    for (i, node) in el.children.iter().enumerate() {
        if let Node::Element(child) = node {
            prefix.push(i);
            collect_under(child, ancestor, local, here, prefix, out);
            prefix.pop();
        }
    }
}

fn has_ancestor(root: &Element, path: &[usize], local: &str) -> bool {
    if path.is_empty() {
        return false;
    }
    if root.local_name() == local {
        return true;
    }
    let mut cur = root;
    for &idx in &path[..path.len() - 1] {
        cur = match cur.children.get(idx) {
            Some(Node::Element(el)) => el,
            _ => return false,
        };
        if cur.local_name() == local {
            return true;
        }
    }
    false
}

fn try_follow<'a>(el: &'a Element, path: &[usize]) -> Option<&'a Element> {
    let mut cur = el;
    for &idx in path {
        cur = match cur.children.get(idx)? {
            Node::Element(child) => child,
            _ => return None,
        };
    }
    Some(cur)
}

fn try_follow_mut<'a>(el: &'a mut Element, path: &[usize]) -> Option<&'a mut Element> {
    let mut cur = el;
    for &idx in path {
        cur = match cur.children.get_mut(idx)? {
            Node::Element(child) => child,
            _ => return None,
        };
    }
    Some(cur)
}

fn client_data_path(shape: &Element) -> Option<Vec<usize>> {
    let mut paths = Vec::new();
    collect_paths(
        shape,
        &|el| el.local_name() == "ClientData",
        &mut Vec::new(),
        &mut paths,
    );
    paths.into_iter().next()
}

fn write_joined(doc: &mut Document, base: &[usize], rel_paths: &[Vec<usize>], text: &str) {
    for (n, rel) in rel_paths.iter().enumerate() {
        let mut path = base.to_vec();
        path.extend(rel);
        let Some(node) = try_follow_mut(&mut doc.root, &path) else {
            continue;
        };
        if n == 0 {
            node.set_text(text);
        } else {
            node.set_text("");
        }
    }
}

fn apply_text_pair(client: &mut Element, local: &str, patch: &TextPatch) -> bool {
    match patch {
        TextPatch::Absent => false,
        TextPatch::Clear => {
            remove_local(client, local);
            true
        }
        TextPatch::Set(value) => {
            set_child_text(client, local, value);
            true
        }
    }
}

fn set_child_text(parent: &mut Element, local: &str, text: &str) {
    if let Some(child) = parent.child_by_local_mut(local) {
        child.set_text(text);
        return;
    }
    parent.push_element(Element::new(parent.prefixed(local)).with_text(text));
}

fn remove_local(el: &mut Element, local: &str) {
    el.children.retain(|node| match node {
        Node::Element(child) => child.local_name() != local,
        _ => true,
    });
}

fn set_attr_local(el: &mut Element, local: &str, value: &str) {
    if let Some((_, existing)) = el
        .attrs
        .iter_mut()
        .find(|(key, _)| key.rsplit(':').next().unwrap_or(key) == local)
    {
        *existing = value.to_string();
        return;
    }
    el.set_attr(local, value);
}

fn remove_attr_local(el: &mut Element, local: &str) {
    el.attrs
        .retain(|(key, _)| key.rsplit(':').next().unwrap_or(key) != local);
}

fn walk_all(el: &Element, visit: &mut impl FnMut(&Element)) {
    visit(el);
    for child in el.element_children() {
        walk_all(child, visit);
    }
}

fn parse_caption(obj: &Map<String, Value>, i: usize) -> Result<Option<String>, Error> {
    match obj.get("caption") {
        None => Ok(None),
        Some(Value::String(text)) if !text.is_empty() => Ok(Some(text.clone())),
        Some(Value::String(_)) => Err(field_err("caption", i)),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "caption".into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].caption"),
        )),
    }
}

fn parse_formula(obj: &Map<String, Value>, i: usize, field: &str) -> Result<TextPatch, Error> {
    match obj.get(field) {
        None => Ok(TextPatch::Absent),
        Some(Value::Null) => Ok(TextPatch::Clear),
        Some(Value::String(text)) if !text.is_empty() => Ok(TextPatch::Set(text.clone())),
        Some(Value::String(_)) => Err(field_err(field, i)),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::String,
            },
            format!("ops[{i}].{field}"),
        )),
    }
}

fn parse_sel(obj: &Map<String, Value>, i: usize) -> Result<Option<u64>, Error> {
    match obj.get("sel") {
        None => Ok(None),
        Some(Value::Number(number)) => match number.as_u64() {
            Some(value) => Ok(Some(value)),
            None => Err(field_err("sel", i)),
        },
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "sel".into(),
                expected: TypeName::Integer,
            },
            format!("ops[{i}].sel"),
        )),
    }
}
