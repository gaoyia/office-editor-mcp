//! 工作表窗体：按钮、复选框、下拉。
//!
//! 一个控件要四处对上才进树：工作表 `control/@shapeId`、Choice 里 `cNvPr/@id`
//! 与 `compatExt/@spid`、VML `v:shape/@id`。标题在 Choice 的 `a:t` 和 VML 文本框。
//! 链接、列表、勾选、选中项在 `formControlPr` 和 VML 里各写一处。
//! 插入和删除把这四处一起写入或摘掉。修改已有控件时不改锚点。

use std::collections::{HashMap, HashSet};

use serde_json::{json, Map, Value};

use crate::error::{Error, ErrorCode, ErrorKind, IdReason, TypeName};
use crate::op_fields::{deny_keys, parse_opt_bool};
use crate::opc::{self, ContentTypes, Package, Relationship, Relationships};
use crate::xml::{self, Document, Element, Node, DEFAULT_DECL};

use super::a1;
use super::comments;
use super::layout;
use super::list_pin;
use super::picture;

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

const MC_NS: &str = "http://schemas.openxmlformats.org/markup-compatibility/2006";
const X14_NS: &str = "http://schemas.microsoft.com/office/spreadsheetml/2009/9/main";
const A14_NS: &str = "http://schemas.microsoft.com/office/drawing/2010/main";
const V_NS: &str = "urn:schemas-microsoft-com:vml";
const O_NS: &str = "urn:schemas-microsoft-com:office:office";
const X_NS: &str = "urn:schemas-microsoft-com:office:excel";
const COMPAT_URI: &str = "{63B3BB69-23CF-44E3-9099-C40C66FF867C}";
const REL_CTRL: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/ctrlProp";
const CT_CTRL: &str = "application/vnd.ms-excel.controlproperties+xml";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ControlKind {
    Button,
    CheckBox,
    Drop,
}

impl ControlKind {
    fn object_type(self) -> &'static str {
        match self {
            Self::Button => "Button",
            Self::CheckBox => "CheckBox",
            Self::Drop => "Drop",
        }
    }

    fn vml_type(self) -> &'static str {
        match self {
            Self::Button => "Button",
            Self::CheckBox => "Checkbox",
            Self::Drop => "Drop",
        }
    }

    fn name_label(self) -> &'static str {
        match self {
            Self::Button => "Button",
            Self::CheckBox => "Check Box",
            Self::Drop => "Drop Down",
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct InsertFormControlOp {
    pub(crate) sheet: String,
    kind: ControlKind,
    caption: Option<String>,
    fmla_link: Option<String>,
    fmla_range: Option<String>,
    checked: Option<bool>,
    sel: Option<u64>,
    from: a1::CellRef,
    to: a1::CellRef,
    from_off_x: i64,
    from_off_y: i64,
    to_off_x: i64,
    to_off_y: i64,
}

#[derive(Debug, Clone)]
pub(crate) struct DeleteFormControlOp {
    pub(crate) sheet: String,
    shape_id: String,
}

pub(crate) fn parse_insert(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<InsertFormControlOp, Error> {
    deny_keys(
        obj,
        i,
        &[
            "op",
            "id",
            "objectType",
            "caption",
            "fmlaLink",
            "fmlaRange",
            "checked",
            "sel",
            "from",
            "to",
            "fromOffX",
            "fromOffY",
            "toOffX",
            "toOffY",
        ],
    )?;
    let id = require_str(obj, i, "id")?;
    let sheet = parse_sheet_id(id, &format!("ops[{i}].id"))?;
    let kind = match require_str(obj, i, "objectType")? {
        "Button" => ControlKind::Button,
        "CheckBox" => ControlKind::CheckBox,
        "Drop" => ControlKind::Drop,
        _ => return Err(field_err("objectType", i)),
    };
    let caption = parse_caption(obj, i)?;
    let fmla_link = parse_optional_text(obj, i, "fmlaLink")?;
    let fmla_range = parse_optional_text(obj, i, "fmlaRange")?;
    let checked = parse_opt_bool(obj, i, "checked")?;
    let sel = parse_sel(obj, i)?;
    reject_insert(kind, &caption, &fmla_link, &fmla_range, checked, sel, i)?;
    Ok(InsertFormControlOp {
        sheet,
        kind,
        caption,
        fmla_link,
        fmla_range,
        checked,
        sel,
        from: parse_cell_field(obj, i, "from")?,
        to: parse_cell_field(obj, i, "to")?,
        from_off_x: parse_off(obj, i, "fromOffX")?,
        from_off_y: parse_off(obj, i, "fromOffY")?,
        to_off_x: parse_off(obj, i, "toOffX")?,
        to_off_y: parse_off(obj, i, "toOffY")?,
    })
}

pub(crate) fn parse_delete(
    obj: &Map<String, Value>,
    i: usize,
) -> Result<DeleteFormControlOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_str(obj, i, "id")?;
    let (sheet, shape_id) = parse_id(id, &format!("ops[{i}].id"))?;
    Ok(DeleteFormControlOp { sheet, shape_id })
}

pub(crate) fn insert(
    pkg: &mut Package,
    sheet_doc: &mut Document,
    sheet_part: &str,
    op: &InsertFormControlOp,
    op_index: usize,
) -> Result<(), Error> {
    picture::ensure_xmlns(&mut sheet_doc.root, "r", super::REL_NS);
    picture::ensure_xmlns(&mut sheet_doc.root, "xdr", picture::XDR_NS);
    picture::ensure_xmlns(&mut sheet_doc.root, "mc", MC_NS);
    picture::ensure_xmlns(&mut sheet_doc.root, "x14", X14_NS);
    let drawing_part = picture::ensure_drawing_part(pkg, sheet_doc, sheet_part)?;
    let vml_part = ensure_vml_part(pkg, sheet_part, &mut sheet_doc.root)?;
    let mut drawing = list_pin::parse_part(pkg, &drawing_part)?;
    let mut vml = list_pin::parse_part(pkg, &vml_part)?;
    let sid = next_shape_id(&sheet_doc.root, &drawing.root, &vml.root);
    let number = next_name_number(&sheet_doc.root);
    let name = format!("{} {number}", op.kind.name_label());
    let sel = op.sel.unwrap_or(0);
    let ctrl_part = pkg.first_free(|n| format!("/xl/ctrlProps/ctrlProp{n}.xml"));
    list_pin::write_part(pkg, &ctrl_part, &ctrl_doc(op, sel))?;
    let rid = add_rel(pkg, sheet_part, REL_CTRL, &ctrl_part)?;
    let mut cts = ContentTypes::from_package(pkg)?;
    cts.set_override(&ctrl_part, CT_CTRL)?;
    cts.write_to(pkg)?;

    let control = control_block(op, sid, &rid, &name);
    push_control(&mut sheet_doc.root, control).map_err(|err| err.at(format!("ops[{op_index}]")))?;
    picture::ensure_xmlns(&mut drawing.root, "a14", A14_NS);
    picture::ensure_xmlns(&mut drawing.root, "mc", MC_NS);
    drawing.root.push_element(drawing_block(op, sid, &name));
    prepare_vml(&mut vml.root);
    vml.root.push_element(vml_shape(op, sid, number, sel));
    list_pin::write_part(pkg, &drawing_part, &drawing)?;
    list_pin::write_part(pkg, &vml_part, &vml)?;
    if let Some(on) = op.checked {
        if let Some(cell) = op
            .fmla_link
            .as_deref()
            .and_then(|text| linked_cell(text, &op.sheet))
        {
            write_check_cell(&mut sheet_doc.root, cell, on);
        }
    }
    Ok(())
}

pub(crate) fn delete(
    pkg: &mut Package,
    sheet: &mut Element,
    sheet_part: &str,
    op: &DeleteFormControlOp,
    op_index: usize,
) -> Result<(), Error> {
    let loaded = load(pkg, sheet_part, sheet)?;
    let Some(item) = loaded
        .items
        .iter()
        .find(|item| item.shape_id == op.shape_id)
    else {
        return Err(Error::with_path(
            ErrorKind::FormControlNotFound {
                id: format!("sheet:{}/control:{}", op.sheet, op.shape_id),
            },
            format!("ops[{op_index}].id"),
        ));
    };
    let drawing_part = loaded.drawings[item.drawing_idx].0.clone();
    let vml_part = loaded.vmls[item.vml_idx].0.clone();
    let ctrl_part = item.ctrl_part.clone();
    let sid = op.shape_id.clone();
    let mut drawing = list_pin::parse_part(pkg, &drawing_part)?;
    let mut vml = list_pin::parse_part(pkg, &vml_part)?;
    if !strip_top_child(&mut drawing.root, &|el| contains_sp(el, &sid))
        || !remove_shape(&mut vml.root, &sid)
        || !sheet_has_control(sheet, &sid)
    {
        return Err(Error::with_path(
            ErrorKind::FormControlNotFound {
                id: format!("sheet:{}/control:{}", op.sheet, sid),
            },
            format!("ops[{op_index}].id"),
        ));
    }
    remove_control(sheet, &sid);
    list_pin::write_part(pkg, &drawing_part, &drawing)?;
    list_pin::write_part(pkg, &vml_part, &vml)?;
    drop_rel_to(pkg, sheet_part, &ctrl_part)?;
    pkg.remove(&ctrl_part);
    pkg.remove(&opc::rels_part_name(&ctrl_part));
    let mut cts = ContentTypes::from_package(pkg)?;
    cts.remove_override(&ctrl_part);
    cts.write_to(pkg)?;
    Ok(())
}

fn reject_insert(
    kind: ControlKind,
    caption: &Option<String>,
    fmla_link: &Option<String>,
    fmla_range: &Option<String>,
    checked: Option<bool>,
    sel: Option<u64>,
    op_index: usize,
) -> Result<(), Error> {
    let bad = |field: &str| field_err(field, op_index);
    match kind {
        ControlKind::Button => {
            if caption.is_none() {
                return Err(bad("caption"));
            }
            if fmla_link.is_some() {
                return Err(bad("fmlaLink"));
            }
            if fmla_range.is_some() {
                return Err(bad("fmlaRange"));
            }
            if checked.is_some() {
                return Err(bad("checked"));
            }
            if sel.is_some() {
                return Err(bad("sel"));
            }
        }
        ControlKind::CheckBox => {
            if caption.is_none() {
                return Err(bad("caption"));
            }
            if fmla_range.is_some() {
                return Err(bad("fmlaRange"));
            }
            if sel.is_some() {
                return Err(bad("sel"));
            }
        }
        ControlKind::Drop => {
            if caption.is_some() {
                return Err(bad("caption"));
            }
            if fmla_link.is_some() {
                return Err(bad("fmlaLink"));
            }
            if checked.is_some() {
                return Err(bad("checked"));
            }
        }
    }
    Ok(())
}

fn ctrl_doc(op: &InsertFormControlOp, sel: u64) -> Document {
    let mut root = Element::new("formControlPr")
        .with_attr("xmlns", X14_NS)
        .with_attr("objectType", op.kind.object_type());
    match op.kind {
        ControlKind::Button => {
            root.set_attr("lockText", "1");
        }
        ControlKind::CheckBox => {
            root.set_attr("lockText", "1");
            root.set_attr("noThreeD", "1");
            if let Some(link) = &op.fmla_link {
                root.set_attr("fmlaLink", link);
            }
            if op.checked == Some(true) {
                root.set_attr("checked", "Checked");
            }
        }
        ControlKind::Drop => {
            root.set_attr("dropStyle", "combo");
            root.set_attr("dx", "22");
            root.set_attr("noThreeD", "1");
            root.set_attr("sel", sel.to_string());
            root.set_attr("val", "0");
            if let Some(range) = &op.fmla_range {
                root.set_attr("fmlaRange", range);
            }
        }
    }
    Document::new(Some(DEFAULT_DECL.to_string()), root)
}

fn control_block(op: &InsertFormControlOp, sid: u32, rid: &str, name: &str) -> Element {
    let mut control = Element::new("control")
        .with_attr("shapeId", sid.to_string())
        .with_attr("r:id", rid)
        .with_attr("name", name);
    let mut pr = Element::new("controlPr").with_attr("defaultSize", "0");
    match op.kind {
        ControlKind::Button => {
            pr.set_attr("print", "0");
            pr.set_attr("autoFill", "0");
            pr.set_attr("autoPict", "0");
        }
        ControlKind::CheckBox => {
            pr.set_attr("autoFill", "0");
            pr.set_attr("autoLine", "0");
            pr.set_attr("autoPict", "0");
        }
        ControlKind::Drop => {
            pr.set_attr("autoFill", "0");
            pr.set_attr("autoPict", "0");
        }
    }
    let mut anchor = Element::new("anchor").with_attr("moveWithCells", "1");
    if op.kind == ControlKind::Button {
        anchor.set_attr("sizeWithCells", "1");
    }
    anchor.push_element(picture::marker_el(
        "from",
        op.from,
        op.from_off_x,
        op.from_off_y,
    ));
    anchor.push_element(picture::marker_el("to", op.to, op.to_off_x, op.to_off_y));
    pr.push_element(anchor);
    control.push_element(pr);
    let mut choice = Element::new("mc:Choice").with_attr("Requires", "x14");
    choice.push_element(control);
    let mut host = Element::new("mc:AlternateContent").with_attr("xmlns:mc", MC_NS);
    host.push_element(choice);
    host
}

fn push_control(sheet: &mut Element, control: Element) -> Result<(), Error> {
    if find_path(sheet, &|el| el.local_name() == "controls").is_none() {
        let controls = Element::new("controls");
        let mut choice = Element::new("mc:Choice").with_attr("Requires", "x14");
        choice.push_element(controls);
        let mut host = Element::new("mc:AlternateContent").with_attr("xmlns:mc", MC_NS);
        host.push_element(choice);
        layout::insert_child_ordered(sheet, host, "controls");
    }
    let path =
        find_path(sheet, &|el| el.local_name() == "controls").ok_or_else(|| field_err("id", 0))?;
    let host = try_follow_mut(sheet, &path).ok_or_else(|| field_err("id", 0))?;
    host.push_element(control);
    Ok(())
}

fn drawing_block(op: &InsertFormControlOp, sid: u32, name: &str) -> Element {
    let mut cnv = Element::new("xdr:cNvPr")
        .with_attr("id", sid.to_string())
        .with_attr("name", name)
        .with_attr("hidden", "1");
    let mut ext = Element::new("a:ext").with_attr("uri", COMPAT_URI);
    ext.push_element(Element::new("a14:compatExt").with_attr("spid", format!("_x0000_s{sid}")));
    let mut list = Element::new("a:extLst");
    list.push_element(ext);
    cnv.push_element(list);
    let mut nv = Element::new("xdr:nvSpPr");
    nv.push_element(cnv);
    nv.push_element(Element::new("xdr:cNvSpPr"));
    let mut xfrm = Element::new("a:xfrm");
    xfrm.push_element(
        Element::new("a:off")
            .with_attr("x", "0")
            .with_attr("y", "0"),
    );
    xfrm.push_element(
        Element::new("a:ext")
            .with_attr("cx", "0")
            .with_attr("cy", "0"),
    );
    let mut geom = Element::new("a:prstGeom").with_attr("prst", "rect");
    geom.push_element(Element::new("a:avLst"));
    let mut line = Element::new("a:ln").with_attr("w", "9525");
    line.push_element(Element::new("a:miter"));
    let mut sp_pr = Element::new("xdr:spPr").with_attr("bwMode", "auto");
    sp_pr.push_element(xfrm);
    sp_pr.push_element(geom);
    sp_pr.push_element(Element::new("a:noFill"));
    sp_pr.push_element(line);
    let mut sp = Element::new("xdr:sp")
        .with_attr("macro", "")
        .with_attr("textlink", "");
    sp.push_element(nv);
    sp.push_element(sp_pr);
    if let Some(caption) = &op.caption {
        sp.push_element(tx_body(caption));
    }
    sp.push_element(if op.kind == ControlKind::Button {
        Element::new("xdr:clientData").with_attr("fPrintsWithSheet", "0")
    } else {
        Element::new("xdr:clientData")
    });
    let mut anchor = Element::new("xdr:twoCellAnchor");
    if op.kind != ControlKind::Button {
        anchor.set_attr("editAs", "oneCell");
    }
    anchor.push_element(picture::marker_el(
        "xdr:from",
        op.from,
        op.from_off_x,
        op.from_off_y,
    ));
    anchor.push_element(picture::marker_el(
        "xdr:to",
        op.to,
        op.to_off_x,
        op.to_off_y,
    ));
    anchor.push_element(sp);
    anchor.push_element(Element::new("xdr:clientData"));
    let mut choice = Element::new("mc:Choice")
        .with_attr("xmlns:a14", A14_NS)
        .with_attr("Requires", "a14");
    choice.push_element(anchor);
    let mut host = Element::new("mc:AlternateContent").with_attr("xmlns:mc", MC_NS);
    host.push_element(choice);
    host.push_element(Element::new("mc:Fallback"));
    host
}

fn tx_body(caption: &str) -> Element {
    let body = Element::new("a:bodyPr")
        .with_attr("vertOverflow", "clip")
        .with_attr("wrap", "square")
        .with_attr("lIns", "27432")
        .with_attr("tIns", "22860")
        .with_attr("rIns", "27432")
        .with_attr("bIns", "22860")
        .with_attr("anchor", "ctr")
        .with_attr("upright", "1");
    let mut def = Element::new("a:pPr").with_attr("algn", "ctr");
    def.push_element(Element::new("a:defRPr").with_attr("sz", "1100"));
    let mut run = Element::new("a:r");
    run.push_element(
        Element::new("a:rPr")
            .with_attr("lang", "zh-CN")
            .with_attr("sz", "1100"),
    );
    run.push_element(Element::new("a:t").with_text(caption));
    let mut para = Element::new("a:p");
    para.push_element(def);
    para.push_element(run);
    let mut body_el = Element::new("xdr:txBody");
    body_el.push_element(body);
    body_el.push_element(Element::new("a:lstStyle"));
    body_el.push_element(para);
    body_el
}

fn vml_shape(op: &InsertFormControlOp, sid: u32, z: u32, sel: u64) -> Element {
    let mut shape = Element::new("v:shape")
        .with_attr("id", format!("_x0000_s{sid}"))
        .with_attr("type", "#_x0000_t201")
        .with_attr(
            "style",
            format!(
                "position:absolute;margin-left:0;margin-top:0;width:90pt;height:24pt;z-index:{z}"
            ),
        )
        .with_attr("o:insetmode", "auto");
    match op.kind {
        ControlKind::Button => {
            shape.set_attr("fillcolor", "buttonFace [67]");
        }
        ControlKind::CheckBox | ControlKind::Drop => {
            shape.set_attr("filled", "f");
            shape.set_attr("strokecolor", "windowText [64]");
        }
    }
    let mut fill = Element::new("v:fill");
    match op.kind {
        ControlKind::Button => {
            fill.set_attr("color2", "buttonFace [67]");
            fill.set_attr("o:detectmouseclick", "t");
        }
        ControlKind::CheckBox | ControlKind::Drop => {
            fill.set_attr("color2", "window [65]");
        }
    }
    shape.push_element(fill);
    shape.push_element(
        Element::new("o:lock")
            .with_attr("v:ext", "edit")
            .with_attr("rotation", "t"),
    );
    if let Some(caption) = &op.caption {
        shape.push_element(vml_textbox(op.kind, caption));
    }
    shape.push_element(vml_client(op, sel));
    shape
}

fn vml_textbox(kind: ControlKind, caption: &str) -> Element {
    let align = if kind == ControlKind::Button {
        "center"
    } else {
        "left"
    };
    let font = Element::new("font")
        .with_attr("face", "Calibri")
        .with_attr("size", "220")
        .with_attr("color", "#000000")
        .with_text(caption);
    let mut div = Element::new("div").with_attr("style", format!("text-align:{align}"));
    div.push_element(font);
    let mut textbox = Element::new("v:textbox")
        .with_attr("style", "mso-direction-alt:auto")
        .with_attr("o:singleclick", "f");
    textbox.push_element(div);
    textbox
}

fn vml_client(op: &InsertFormControlOp, sel: u64) -> Element {
    let mut client = Element::new("x:ClientData").with_attr("ObjectType", op.kind.vml_type());
    let push_text = |parent: &mut Element, local: &str, text: &str| {
        parent.push_element(Element::new(parent.prefixed(local)).with_text(text));
    };
    let push_empty = |parent: &mut Element, local: &str| {
        parent.push_element(Element::new(parent.prefixed(local)));
    };
    if op.kind != ControlKind::Button {
        push_empty(&mut client, "SizeWithCells");
    }
    push_text(&mut client, "Anchor", &vml_anchor(op.from, op.to));
    match op.kind {
        ControlKind::Button => {
            push_text(&mut client, "PrintObject", "False");
            push_text(&mut client, "AutoFill", "False");
            push_text(&mut client, "TextHAlign", "Center");
            push_text(&mut client, "TextVAlign", "Center");
        }
        ControlKind::CheckBox => {
            push_text(&mut client, "AutoFill", "False");
            push_text(&mut client, "AutoLine", "False");
            push_text(&mut client, "TextVAlign", "Center");
            if op.checked == Some(true) {
                push_text(&mut client, "Checked", "1");
            }
            if let Some(link) = &op.fmla_link {
                push_text(&mut client, "FmlaLink", link);
            }
            push_empty(&mut client, "NoThreeD");
        }
        ControlKind::Drop => {
            push_text(&mut client, "AutoFill", "False");
            push_text(&mut client, "Val", "0");
            push_text(&mut client, "Min", "0");
            push_text(&mut client, "Max", "0");
            push_text(&mut client, "Inc", "1");
            push_text(&mut client, "Page", "10");
            push_text(&mut client, "Dx", "22");
            if let Some(range) = &op.fmla_range {
                push_text(&mut client, "FmlaRange", range);
            }
            push_text(&mut client, "Sel", &sel.to_string());
            push_empty(&mut client, "NoThreeD2");
            push_text(&mut client, "SelType", "Single");
            push_text(&mut client, "LCT", "Normal");
            push_text(&mut client, "DropStyle", "Combo");
            push_text(&mut client, "DropLines", "8");
        }
    }
    client
}

fn vml_anchor(from: a1::CellRef, to: a1::CellRef) -> String {
    format!(
        "{}, 0, {}, 0, {}, 0, {}, 0",
        from.col.saturating_sub(1),
        from.row.saturating_sub(1),
        to.col.saturating_sub(1),
        to.row.saturating_sub(1)
    )
}

fn prepare_vml(root: &mut Element) {
    picture::ensure_xmlns(root, "v", V_NS);
    picture::ensure_xmlns(root, "o", O_NS);
    picture::ensure_xmlns(root, "x", X_NS);
    if root
        .element_children()
        .all(|el| el.local_name() != "shapelayout")
    {
        let mut layout = Element::new("o:shapelayout").with_attr("v:ext", "edit");
        layout.push_element(
            Element::new("o:idmap")
                .with_attr("v:ext", "edit")
                .with_attr("data", "1"),
        );
        root.children.insert(0, Node::Element(layout));
    }
    let mut has_type = false;
    walk_all(root, &mut |el| {
        if el.local_name() == "shapetype" && el.attr("id") == Some("_x0000_t201") {
            has_type = true;
        }
    });
    if !has_type {
        root.push_element(shapetype_201());
    }
}

fn shapetype_201() -> Element {
    let mut el = Element::new("v:shapetype")
        .with_attr("id", "_x0000_t201")
        .with_attr("coordsize", "21600,21600")
        .with_attr("o:spt", "201")
        .with_attr("path", "m,l,21600r21600,l21600,xe");
    el.push_element(Element::new("v:stroke").with_attr("joinstyle", "miter"));
    el.push_element(
        Element::new("v:path")
            .with_attr("shadowok", "f")
            .with_attr("o:extrusionok", "f")
            .with_attr("strokeok", "f")
            .with_attr("fillok", "f")
            .with_attr("o:connecttype", "rect"),
    );
    el.push_element(
        Element::new("o:lock")
            .with_attr("v:ext", "edit")
            .with_attr("shapetype", "t"),
    );
    el
}

fn ensure_vml_part(
    pkg: &mut Package,
    sheet_part: &str,
    sheet: &mut Element,
) -> Result<String, Error> {
    if let Some(el) = sheet
        .element_children()
        .find(|child| child.local_name() == "legacyDrawing")
    {
        if let Some(rid) = rel_id(el) {
            if let Ok(part) = opc::resolve_id(pkg, sheet_part, rid) {
                if !pkg.contains(&part) {
                    list_pin::write_part(pkg, &part, &empty_vml_doc())?;
                }
                return Ok(part);
            }
        }
    }
    let rels = Relationships::from_package(pkg, sheet_part)?;
    if let Some(rel) = rels
        .rels
        .iter()
        .find(|rel| rel.rel_type == comments::REL_VML)
    {
        let part = opc::resolve_rel_target(sheet_part, &rel.target)?;
        if !pkg.contains(&part) {
            list_pin::write_part(pkg, &part, &empty_vml_doc())?;
        }
        if sheet
            .element_children()
            .all(|child| child.local_name() != "legacyDrawing")
        {
            layout::insert_child_ordered(
                sheet,
                Element::new("legacyDrawing").with_attr("r:id", &rel.id),
                "legacyDrawing",
            );
        }
        return Ok(part);
    }
    let part = pkg.first_free(|n| format!("/xl/drawings/vmlDrawing{n}.vml"));
    list_pin::write_part(pkg, &part, &empty_vml_doc())?;
    let rid = add_rel(pkg, sheet_part, comments::REL_VML, &part)?;
    layout::insert_child_ordered(
        sheet,
        Element::new("legacyDrawing").with_attr("r:id", rid),
        "legacyDrawing",
    );
    let mut cts = ContentTypes::from_package(pkg)?;
    cts.ensure_default("vml", comments::CT_VML);
    cts.write_to(pkg)?;
    Ok(part)
}

fn empty_vml_doc() -> Document {
    let root = Element::new("xml")
        .with_attr("xmlns:v", V_NS)
        .with_attr("xmlns:o", O_NS)
        .with_attr("xmlns:x", X_NS);
    Document::new(Some(DEFAULT_DECL.to_string()), root)
}

fn add_rel(
    pkg: &mut Package,
    source: &str,
    rel_type: &str,
    target_part: &str,
) -> Result<String, Error> {
    let mut rels = Relationships::from_package(pkg, source)?;
    let id = rels.next_rid();
    rels.push(Relationship {
        id: id.clone(),
        rel_type: rel_type.to_string(),
        target: picture::rel_target_from(source, target_part),
        target_mode: None,
        extra: Vec::new(),
    });
    rels.write_to(pkg, source)?;
    Ok(id)
}

fn drop_rel_to(pkg: &mut Package, sheet_part: &str, target_part: &str) -> Result<(), Error> {
    let mut rels = Relationships::from_package(pkg, sheet_part)?;
    rels.rels.retain(|rel| {
        if rel.is_external() {
            return true;
        }
        match opc::resolve_rel_target(sheet_part, &rel.target) {
            Ok(part) => part != target_part,
            Err(_) => true,
        }
    });
    rels.write_to(pkg, sheet_part)?;
    Ok(())
}

fn next_shape_id(sheet: &Element, drawing: &Element, vml: &Element) -> u32 {
    let mut max = 1024u32;
    walk_ids(sheet, &mut max);
    walk_ids(drawing, &mut max);
    walk_ids(vml, &mut max);
    max.saturating_add(1)
}

fn walk_ids(el: &Element, max: &mut u32) {
    if matches!(el.local_name(), "control" | "oleObject") {
        note_id(max, el.attr("shapeId"));
    }
    if el.local_name() == "cNvPr" {
        note_id(max, el.attr("id"));
    }
    if el.local_name() == "shape" {
        note_id(max, el.attr("id"));
    }
    for child in el.element_children() {
        walk_ids(child, max);
    }
}

fn note_id(max: &mut u32, text: Option<&str>) {
    let Some(text) = text else {
        return;
    };
    let digits = text.strip_prefix("_x0000_s").unwrap_or(text);
    if let Ok(value) = digits.parse::<u32>() {
        *max = (*max).max(value);
    }
}

fn next_name_number(sheet: &Element) -> u32 {
    let mut max = 0u32;
    walk_all(sheet, &mut |el| {
        if el.local_name() != "control" {
            return;
        }
        let Some(name) = el.attr("name") else {
            return;
        };
        let Some(tail) = name
            .rsplit(|ch: char| !ch.is_ascii_digit())
            .next()
            .filter(|text| !text.is_empty())
        else {
            return;
        };
        if let Ok(value) = tail.parse::<u32>() {
            max = max.max(value);
        }
    });
    max.saturating_add(1)
}

fn sheet_has_control(sheet: &Element, sid: &str) -> bool {
    find_path(sheet, &|el| {
        el.local_name() == "control" && el.attr("shapeId") == Some(sid)
    })
    .is_some()
}

fn remove_control(sheet: &mut Element, sid: &str) {
    let Some(path) = find_path(sheet, &|el| {
        el.local_name() == "control" && el.attr("shapeId") == Some(sid)
    }) else {
        return;
    };
    if path.is_empty() {
        return;
    }
    let mut cut = path.len() - 1;
    for index in (0..path.len()).rev() {
        let Some(node) = try_follow(sheet, &path[..=index]) else {
            continue;
        };
        if node.local_name() == "AlternateContent" && only_this_control(node, sid) {
            cut = index;
            break;
        }
    }
    remove_at(sheet, &path[..=cut]);
    prune_controls(sheet);
}

fn only_this_control(el: &Element, sid: &str) -> bool {
    let mut ids = Vec::new();
    collect_control_ids(el, &mut ids);
    !ids.is_empty() && ids.iter().all(|id| *id == sid)
}

fn collect_control_ids<'a>(el: &'a Element, out: &mut Vec<&'a str>) {
    if el.local_name() == "control" {
        if let Some(id) = el.attr("shapeId") {
            out.push(id);
        }
    }
    for child in el.element_children() {
        collect_control_ids(child, out);
    }
}

fn prune_controls(sheet: &mut Element) {
    let Some(path) = find_path(sheet, &|el| el.local_name() == "controls") else {
        return;
    };
    let Some(node) = try_follow(sheet, &path) else {
        return;
    };
    if has_control(node) || path.is_empty() {
        return;
    }
    let Some(top) = try_follow(sheet, &path[..1]) else {
        return;
    };
    if !has_control(top) {
        sheet.children.remove(path[0]);
    }
}

fn has_control(el: &Element) -> bool {
    if el.local_name() == "control" {
        return true;
    }
    el.element_children().any(has_control)
}

fn remove_at(root: &mut Element, path: &[usize]) {
    if path.is_empty() {
        return;
    }
    let idx = path[path.len() - 1];
    let Some(parent) = (if path.len() == 1 {
        Some(root)
    } else {
        try_follow_mut(root, &path[..path.len() - 1])
    }) else {
        return;
    };
    if idx < parent.children.len() {
        parent.children.remove(idx);
    }
}

fn strip_top_child(root: &mut Element, pred: &impl Fn(&Element) -> bool) -> bool {
    let Some(index) = root.children.iter().position(|node| match node {
        Node::Element(el) => pred(el),
        Node::Text(_) | Node::Raw(_) => false,
    }) else {
        return false;
    };
    root.children.remove(index);
    true
}

fn contains_sp(el: &Element, sid: &str) -> bool {
    if sp_matches(el, sid) {
        return true;
    }
    el.element_children().any(|child| contains_sp(child, sid))
}

fn remove_shape(root: &mut Element, sid: &str) -> bool {
    let want = format!("_x0000_s{sid}");
    let mut index = 0;
    while index < root.children.len() {
        let matched = match &root.children[index] {
            Node::Element(child) => {
                child.local_name() == "shape" && child.attr("id") == Some(want.as_str())
            }
            Node::Text(_) | Node::Raw(_) => false,
        };
        if matched {
            root.children.remove(index);
            return true;
        }
        if let Node::Element(child) = &mut root.children[index] {
            if remove_shape(child, sid) {
                return true;
            }
        }
        index += 1;
    }
    false
}

fn find_path(el: &Element, pred: &impl Fn(&Element) -> bool) -> Option<Vec<usize>> {
    let mut out = Vec::new();
    collect_paths(el, pred, &mut Vec::new(), &mut out);
    out.into_iter().next()
}

fn parse_sheet_id(id: &str, json_path: &str) -> Result<String, Error> {
    let bad = || {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::InsertFormControlNeedsSheet,
            },
            json_path.to_string(),
        )
    };
    let Some(name) = id.strip_prefix("sheet:") else {
        return Err(bad());
    };
    if name.is_empty() || name.contains('/') {
        return Err(bad());
    }
    Ok(name.to_string())
}

fn require_str<'a>(obj: &'a Map<String, Value>, i: usize, field: &str) -> Result<&'a str, Error> {
    match obj.get(field) {
        Some(Value::String(text)) if !text.is_empty() => Ok(text),
        None => Err(Error::with_path(
            ErrorKind::MissingField {
                field: field.into(),
            },
            format!("ops[{i}].{field}"),
        )),
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

fn parse_optional_text(
    obj: &Map<String, Value>,
    i: usize,
    field: &str,
) -> Result<Option<String>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::String(text)) if !text.is_empty() => Ok(Some(text.clone())),
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

fn parse_cell_field(obj: &Map<String, Value>, i: usize, field: &str) -> Result<a1::CellRef, Error> {
    let text = require_str(obj, i, field)?;
    a1::parse_cell(text).map_err(|err| err.at(format!("ops[{i}].{field}")))
}

fn parse_off(obj: &Map<String, Value>, i: usize, field: &str) -> Result<i64, Error> {
    let Some(value) = obj.get(field) else {
        return Ok(0);
    };
    let path = format!("ops[{i}].{field}");
    let Some(number) = value.as_u64() else {
        if value.as_i64().is_some_and(|item| item < 0) {
            return Err(Error::with_path(
                ErrorKind::MinValue {
                    field: field.into(),
                    min: 0,
                },
                path,
            ));
        }
        return Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Integer,
            },
            path,
        ));
    };
    if number > i64::MAX as u64 {
        return Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Integer,
            },
            path,
        ));
    }
    Ok(number as i64)
}
